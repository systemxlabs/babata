use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::Local;
use log::{error, info};
use tokio::{sync::Mutex, task::JoinHandle};

use crate::{
    BabataResult,
    config::Config,
    error::BabataError,
    message::{Content, Message},
    provider::create_provider,
    skill::load_skills,
    system_prompt::load_system_prompt_files,
    task::AgentTask,
    tool::{Tool, build_tools},
    utils::babata_dir,
};

const JOB_PROMPT: &str = r#"
The job definitions are already loaded below from `{BABATA_HOME}/jobs/<job_name>/job.md`.

Workflow:
- For each loaded job, determine whether it should run at the current time.
  When checking schedule matching, only compare the minute of the current local time with the minute required by the job schedule.
  Example: if the current local time is `2026-03-06T17:15:22.838327+08:00`, a job with cron `0 * * * *` should not run, a job with cron `*/5 * * * *` should run.
- If a job should run, execute it according to the loaded job definition and record the execution result in history files.
- If a job has invalid configuration, missing files, or any other issue, skip that job and continue with others, DO NOT TRY to fix job.
- If a job will never run again in the future, delete the entire job directory.

Constraints:
- If a job will run again in the future, you MUST NOT modify or delete the `job.md` file.
- You MUST NOT create folder under `{BABATA_HOME}/jobs/`.
- You are allowed to create/write/edit/delete job history files.
"#;
const JOB_CHECK_INTERVAL: Duration = Duration::from_secs(30);
const JOB_MANAGER_CHECK_INTERVAL: Duration = Duration::from_secs(10 * 60);

pub struct JobManager {
    tools: HashMap<String, Arc<dyn Tool>>,
    job_loop: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl JobManager {
    pub fn new() -> Self {
        Self {
            tools: build_tools(),
            job_loop: Arc::new(Mutex::new(None)),
        }
    }

    pub fn start(&self) {
        let job_loop = self.job_loop.clone();
        let tools = self.tools.clone();
        tokio::spawn(async move {
            loop {
                {
                    let mut guard = job_loop.lock().await;
                    let need_spawn = match guard.as_ref() {
                        Some(handle) => handle.is_finished(),
                        None => true,
                    };

                    if need_spawn {
                        info!("Spawning new job loop");
                        let new_handle = start_job_loop(tools.clone()).await;
                        *guard = Some(new_handle);
                    }
                }

                tokio::time::sleep(JOB_MANAGER_CHECK_INTERVAL).await;
            }
        });
    }
}

impl Default for JobManager {
    fn default() -> Self {
        Self::new()
    }
}

async fn start_job_loop(tools: HashMap<String, Arc<dyn Tool>>) -> JoinHandle<()> {
    tokio::spawn(async move {
        info!("Start running job checker loop");

        let mut last_run_minute = Local::now().timestamp() / 60;

        loop {
            let current_minute = Local::now().timestamp() / 60;
            if current_minute == last_run_minute {
                tokio::time::sleep(JOB_CHECK_INTERVAL).await;
                continue;
            }
            last_run_minute = current_minute;

            let tools = tools.clone();
            tokio::spawn(async move {
                if let Err(err) = run_job(tools).await {
                    error!("Job run failed: {}", err);
                };
            });
        }
    })
}

async fn run_job(tools: HashMap<String, Arc<dyn Tool>>) -> BabataResult<()> {
    info!("Starting to run job");
    let jobs = load_jobs()?;
    if jobs.is_empty() {
        info!(
            "No jobs found under '{}/jobs', skipping job run",
            babata_dir()?.display()
        );
        return Ok(());
    }

    let config = Config::load()?;
    let agent_config = config
        .get_agent("main")
        .ok_or_else(|| BabataError::internal("Missing 'main' agent config"))?;
    let provider_config = config
        .get_provider(&agent_config.provider)
        .ok_or_else(|| BabataError::internal("Missing provider config"))?;

    let provider = create_provider(provider_config)?;

    let user_message = Message::UserPrompt {
        content: vec![Content::Text {
            text: build_job_prompt(&jobs),
        }],
    };

    let task = AgentTask::new(
        vec![user_message.clone()],
        provider,
        agent_config.model.clone(),
        tools.clone(),
        load_system_prompt_files()?,
        load_skills()?,
    );

    let now = Instant::now();
    task.run().await?;
    info!(
        "Job run completed in {} seconds",
        now.elapsed().as_secs_f32()
    );

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Job {
    job_name: String,
    job_dir: PathBuf,
    job_definition: String,
    job_definition_path: PathBuf,
}

fn build_job_prompt(jobs: &[Job]) -> String {
    let jobs_context = jobs
        .iter()
        .map(|job| {
            format!(
                r#"## Job: {}
Job dir: `{}`
`job.md` path: `{}`
`job.md` content:
{}"#,
                job.job_name,
                job.job_dir.display(),
                job.job_definition_path.display(),
                job.job_definition
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    format!("{JOB_PROMPT}\nLoaded jobs:\n\n{jobs_context}")
}

fn load_jobs() -> BabataResult<Vec<Job>> {
    load_jobs_from_dir(&babata_dir()?.join("jobs"))
}

fn load_jobs_from_dir(dir: &Path) -> BabataResult<Vec<Job>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    if !dir.is_dir() {
        return Err(BabataError::config(format!(
            "Jobs path '{}' is not a directory",
            dir.display()
        )));
    }

    let entries = std::fs::read_dir(dir).map_err(|err| {
        BabataError::config(format!(
            "Failed to read jobs directory '{}': {}",
            dir.display(),
            err
        ))
    })?;

    let mut jobs = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|err| {
            BabataError::config(format!(
                "Failed to read jobs directory entry in '{}': {}",
                dir.display(),
                err
            ))
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let job_path = path.join("job.md");
        if !job_path.is_file() {
            continue;
        }

        let job_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                BabataError::config(format!(
                    "Failed to resolve job name from path '{}'",
                    path.display()
                ))
            })?
            .to_string();
        let job_definition = std::fs::read_to_string(&job_path).map_err(|err| {
            BabataError::config(format!(
                "Failed to read job file '{}': {}",
                job_path.display(),
                err
            ))
        })?;
        jobs.push(Job {
            job_name,
            job_dir: path,
            job_definition,
            job_definition_path: job_path,
        });
    }

    Ok(jobs)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::{build_job_prompt, load_jobs_from_dir};

    fn temp_jobs_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("babata-job-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn returns_empty_when_no_job_md_exists() {
        let dir = temp_jobs_dir();
        fs::create_dir_all(dir.join("job-a")).unwrap();

        let jobs = load_jobs_from_dir(&dir).unwrap();

        fs::remove_dir_all(&dir).unwrap();
        assert!(jobs.is_empty());
    }

    #[test]
    fn loads_job_name_path_and_content() {
        let dir = temp_jobs_dir();
        let job_dir = dir.join("job-a");
        fs::create_dir_all(&job_dir).unwrap();
        let job_path = job_dir.join("job.md");
        fs::write(&job_path, "# job").unwrap();

        let jobs = load_jobs_from_dir(&dir).unwrap();

        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].job_name, "job-a");
        assert_eq!(jobs[0].job_dir, job_dir);
        assert_eq!(jobs[0].job_definition_path, job_path);
        assert_eq!(jobs[0].job_definition, "# job");
    }

    #[test]
    fn job_prompt_includes_delete_job_dir_instruction() {
        let dir = temp_jobs_dir();
        let job_dir = dir.join("job-a");
        fs::create_dir_all(&job_dir).unwrap();
        let job_path = job_dir.join("job.md");
        fs::write(&job_path, "# job").unwrap();

        let jobs = load_jobs_from_dir(&dir).unwrap();
        let prompt = build_job_prompt(&jobs);

        fs::remove_dir_all(&dir).unwrap();
        assert!(prompt.contains(
            "If a job will never run again in the future, delete the entire job directory."
        ));
    }
}
