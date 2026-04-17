use std::path::PathBuf;

use uuid::Uuid;

use crate::{BabataResult, error::BabataError};

pub fn babata_dir() -> BabataResult<PathBuf> {
    Ok(user_home_dir()?.join(".babata"))
}

pub fn providers_dir() -> BabataResult<PathBuf> {
    Ok(babata_dir()?.join("providers"))
}

pub fn provider_dir(provider_name: &str) -> BabataResult<PathBuf> {
    Ok(providers_dir()?.join(provider_name))
}

pub fn channels_dir() -> BabataResult<PathBuf> {
    Ok(babata_dir()?.join("channels"))
}

pub fn user_home_dir() -> BabataResult<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .map_err(|_| {
            BabataError::internal("Failed to resolve home directory from HOME or USERPROFILE")
        })
}

pub fn task_dir(task_id: Uuid) -> BabataResult<PathBuf> {
    Ok(babata_dir()?.join("tasks").join(task_id.to_string()))
}

pub fn channel_dir(channel_name: &str) -> BabataResult<PathBuf> {
    Ok(channels_dir()?.join(channel_name.to_ascii_lowercase()))
}

pub fn agent_dir(agent_name: &str) -> BabataResult<PathBuf> {
    Ok(babata_dir()?.join("agents").join(agent_name))
}

pub const fn build_commit() -> Option<&'static str> {
    match option_env!("BABATA_GIT_COMMIT") {
        Some(commit_id) if !commit_id.is_empty() => Some(commit_id),
        _ => None,
    }
}
