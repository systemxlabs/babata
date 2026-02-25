use std::{collections::HashMap, time::Duration};

use chrono::Utc;
use log::{error, info, warn};

use crate::{
    BabataResult,
    config::{Config, JobConfig, Schedule},
};

use super::{JobHistoryStore, JobRunner};

const CONFIG_RELOAD_INTERVAL: Duration = Duration::from_secs(10);

pub struct RunningJob {
    pub config: Config,
    pub job_config: JobConfig,
    pub handle: tokio::task::JoinHandle<()>,
}

pub fn start_job_scheduler() {
    tokio::spawn(async {
        let mut running_jobs: HashMap<String, RunningJob> = HashMap::new();

        loop {
            match Config::load() {
                Ok(config) => sync_running_jobs(&mut running_jobs, config),
                Err(err) => {
                    warn!(
                        "Failed to load config for dynamic job scheduler refresh; retrying: {}",
                        err
                    );
                }
            }

            tokio::time::sleep(CONFIG_RELOAD_INTERVAL).await;
        }
    });
}

fn sync_running_jobs(running_jobs: &mut HashMap<String, RunningJob>, config: Config) {
    let enabled_jobs: HashMap<String, JobConfig> = config
        .jobs
        .iter()
        .filter(|job| job.enabled)
        .map(|job| (job.name.clone(), job.clone()))
        .collect();

    let mut jobs_to_restart = Vec::new();

    for (job_name, running_job) in running_jobs.iter() {
        let should_restart = match enabled_jobs.get(job_name) {
            Some(new_job) => is_job_changed(&running_job.job_config, new_job),
            None => true,
        };

        if should_restart {
            jobs_to_restart.push(job_name.clone());
        }
    }

    for job_name in jobs_to_restart {
        if let Some(running_job) = running_jobs.remove(&job_name) {
            running_job.handle.abort();
            info!("Removed running job '{}'", job_name);
        }
    }

    for (job_name, job_config) in enabled_jobs {
        if running_jobs.contains_key(&job_name) {
            continue;
        }

        match create_running_job(config.clone(), job_config) {
            Ok(running_job) => {
                running_jobs.insert(job_name.clone(), running_job);
                info!("Added running job '{}'", job_name);
            }
            Err(err) => {
                warn!(
                    "Failed to add running job '{}' from current config snapshot: {}",
                    job_name, err
                );
            }
        }
    }
}

fn create_running_job(config: Config, job_config: JobConfig) -> BabataResult<RunningJob> {
    let schedule = job_config.schedule.clone();
    let handle = spawn_running_job_task(config.clone(), job_config.name.clone(), schedule);
    Ok(RunningJob {
        config,
        job_config,
        handle,
    })
}

fn spawn_running_job_task(
    config: Config,
    job_name: String,
    schedule: Schedule,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(err) = run_running_job_task(config, &job_name, schedule).await {
            error!("Running job task '{}' exited with error: {}", job_name, err);
        }
    })
}

async fn run_running_job_task(
    config: Config,
    job_name: &str,
    schedule: Schedule,
) -> BabataResult<()> {
    let history_store = JobHistoryStore::new()?;

    loop {
        let Some(next_run) = schedule.next_run_from_now()? else {
            info!("Job '{}' is done", job_name,);
            return Ok(());
        };

        let wait_duration = next_run
            .signed_duration_since(Utc::now())
            .to_std()
            .unwrap_or_else(|_| Duration::from_secs(0));
        if wait_duration > Duration::from_secs(0) {
            tokio::time::sleep(wait_duration).await;
        }

        let runner = JobRunner::new(config.clone(), job_name.to_string(), history_store.clone());
        if let Err(err) = runner.run().await {
            error!("Scheduled job '{}' failed: {}", job_name, err);
        }
    }
}

pub fn is_job_changed(old_job: &JobConfig, new_job: &JobConfig) -> bool {
    old_job.agent_name != new_job.agent_name
        || old_job.schedule != new_job.schedule
        || old_job.prompt != new_job.prompt
}
