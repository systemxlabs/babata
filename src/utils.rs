use std::path::PathBuf;

use crate::{BabataResult, error::BabataError};

pub fn babata_dir() -> BabataResult<PathBuf> {
    Ok(resolve_home_dir()?.join(".babata"))
}

pub fn resolve_home_dir() -> BabataResult<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .map_err(|_| {
            BabataError::internal("Failed to resolve home directory from HOME or USERPROFILE")
        })
}
