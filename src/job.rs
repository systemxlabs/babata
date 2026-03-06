use std::{collections::HashMap, sync::Arc, time::{Duration, Instant}};

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
};

const JOB_PROMPT: &str = r#"
Read all `job.md` files from `{BABATA_HOME}/jobs/<job_name>/job.md`.
Determine whether each job should run at the current time.
If a job should run, execute it according to `job.md` and record the execution result in history files.
If a job has invalid configuration, missing files, or any other issue, skip that job and continue with others, DO NOT TRY to fix job.
You MUST NOT create, modify, or delete any `job.md` file.
YOU MUST NOT create folder under `{BABATA_HOME}/jobs/`.
You are ONLY allowed to create/write/edit/delete job history files.
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
            text: JOB_PROMPT.to_string(),
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
    info!("Job run completed in {} seconds", now.elapsed().as_secs_f32());

    Ok(())
}
