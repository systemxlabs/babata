use std::path::PathBuf;

use crate::{BabataResult, error::BabataError};

pub fn babata_dir() -> BabataResult<PathBuf> {
    let home_dir = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| {
            BabataError::internal("Failed to resolve home directory from HOME or USERPROFILE")
        })?;
    Ok(PathBuf::from(home_dir).join(".babata"))
}
