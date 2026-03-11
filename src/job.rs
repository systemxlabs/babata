use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use chrono::Local;
use log::{error, info};

use crate::{
    BabataResult,
    error::BabataError,
    message::{Content, Message},
    runtime::TaskRuntime,
    task::NewTask,
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

#[derive(Debug)]
pub struct JobManager {
    runtime: Arc<TaskRuntime>,
    agent_name: String,
}

impl JobManager {
    pub fn new(runtime: Arc<TaskRuntime>, agent_name: impl Into<String>) -> Self {
        Self {
            runtime,
            agent_name: agent_name.into(),
        }
    }

    pub fn start(&self) {
        let runtime = Arc::clone(&self.runtime);
        let agent_name = self.agent_name.clone();

        tokio::spawn(async move {
            info!("Start running job checker loop");
            let mut last_run_minute = Local::now().timestamp() / 60;

            loop {
                let current_minute = Local::now().timestamp() / 60;
                if current_minute != last_run_minute {
                    last_run_minute = current_minute;
                    if let Err(err) = run_job(runtime.clone(), &agent_name).await {
                        error!("Job run failed: {}", err);
                    }
                }

                tokio::time::sleep(JOB_CHECK_INTERVAL).await;
            }
        });
    }
}

async fn run_job(runtime: Arc<TaskRuntime>, agent_name: &str) -> BabataResult<()> {
    info!("Starting to run job");
    let jobs = load_jobs()?;
    if jobs.is_empty() {
        info!(
            "No jobs found under '{}/jobs', skipping job run",
            babata_dir()?.display()
        );
        return Ok(());
    }

    let config = crate::config::Config::load()?;
    let agent_config = config.get_agent(agent_name)?;
    let user_message = Message::UserPrompt {
        content: vec![Content::Text {
            text: build_job_prompt(&jobs),
        }],
    };

    runtime
        .submit_task(NewTask {
            agent_name: agent_config.name.clone(),
            provider_name: agent_config.provider.clone(),
            model: agent_config.model.clone(),
            task_markdown: build_job_task_markdown(&jobs),
            initial_progress: build_job_progress_markdown(),
            initial_history: vec![user_message],
            parent_task_id: None,
            root_task_id: None,
        })
        .await?;

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Job {
    job_name: String,
    job_dir: PathBuf,
    job_definition: String,
    job_definition_path: PathBuf,
}

fn build_job_task_markdown(jobs: &[Job]) -> String {
    format!(
        r#"# Task

## Goal
Evaluate the loaded jobs and execute the ones that should run now.

## Input
{}

## Completion Criteria
- Check every loaded job against the current minute.
- Execute only the jobs that should run now.
- Record outputs in job history files when work is done.
- Keep `progress.md` current so the scheduler can recover after restart.
"#,
        build_job_prompt(jobs)
    )
}

fn build_job_progress_markdown() -> String {
    r#"# Progress

## Current Goal
- Inspect the loaded jobs and decide which ones should run now.

## Completed
- Scheduler task created.

## Outstanding
- Evaluate schedules.
- Execute eligible jobs.
- Record job history changes.
"#
    .to_string()
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
