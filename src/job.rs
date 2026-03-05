use std::{collections::HashMap, sync::Arc, time::Duration};

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
Read all jobs from `.babata/jobs/` under the user's home directory.
Determine whether each job should run at the current time.
You must NOT create, modify, or delete any job definitions.
You may only read and execute jobs, and write/update/clean job execution history records.
If a job should run, execute it according to its description and record the execution result.
If a job has invalid configuration, missing files, or any other issue, skip that job and continue with others.
"#;
const JOB_CHECK_INTERVAL: Duration = Duration::from_secs(45);
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
        let mut interval = tokio::time::interval(JOB_CHECK_INTERVAL);

        loop {
            interval.tick().await;

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

    task.run().await?;

    Ok(())
}
