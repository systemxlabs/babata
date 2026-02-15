use crate::{
    BabataResult,
    config::{Config, JobConfig},
    error::BabataError,
    job::JobHistoryStore,
};

use super::Args;

pub fn add(_args: &Args, job_config_json: &str) {
    if let Err(err) = run_add(job_config_json) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn delete(_args: &Args, name: &str) {
    if let Err(err) = run_delete(name) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn list(_args: &Args) {
    if let Err(err) = run_list() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn history(_args: &Args, name: Option<&str>, limit: usize) {
    if let Err(err) = run_history(name, limit) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run_add(job_config_json: &str) -> BabataResult<()> {
    let job_config: JobConfig = serde_json::from_str(job_config_json).map_err(|err| {
        BabataError::config(format!(
            "Invalid job config JSON '{}': {}",
            job_config_json, err
        ))
    })?;
    job_config.validate()?;

    let mut config = Config::load()?;
    let job_name = job_config.name.clone();
    config.upsert_job(job_config);
    config.validate()?;
    config.save()?;

    println!("Added/updated job '{}' in config", job_name);
    Ok(())
}

fn run_delete(name: &str) -> BabataResult<()> {
    let mut config = Config::load()?;

    let index = config
        .jobs
        .iter()
        .position(|job| job.name == name)
        .ok_or_else(|| BabataError::config(format!("Job '{}' not found in config", name)))?;

    let deleted = config.jobs.remove(index);
    config.validate()?;
    config.save()?;

    println!("Deleted job '{}'", deleted.name);
    Ok(())
}

fn run_list() -> BabataResult<()> {
    let config = Config::load()?;

    for job_config in &config.jobs {
        let payload = serde_json::to_string(job_config).map_err(|err| {
            BabataError::internal(format!(
                "Failed to serialize job '{}' config to JSON: {}",
                job_config.name, err
            ))
        })?;
        println!("{payload}");
    }

    Ok(())
}

fn run_history(name: Option<&str>, limit: usize) -> BabataResult<()> {
    let store = JobHistoryStore::new()?;
    let rows = store.query(name, limit)?;

    for row in rows {
        let payload = serde_json::to_string(&row).map_err(|err| {
            BabataError::internal(format!(
                "Failed to serialize job history row into JSON: {}",
                err
            ))
        })?;
        println!("{payload}");
    }

    Ok(())
}
