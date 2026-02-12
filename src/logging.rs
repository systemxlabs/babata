use std::num::NonZeroUsize;

use logforth::{append::file::FileBuilder, filter::env_filter::EnvFilterBuilder};

use crate::{BabataResult, error::BabataError, utils::babata_dir};

pub fn init() -> BabataResult<()> {
    let log_dir = babata_dir()?.join("logs");
    let max_log_files = NonZeroUsize::new(1).expect("1 is non-zero");

    let file = FileBuilder::new(log_dir, "babata")
        .filename_suffix("log")
        .rollover_daily()
        .max_log_files(max_log_files)
        .build()
        .map_err(|err| {
            BabataError::internal(format!("Failed to build file appender for logger: {err}"))
        })?;

    let filter = EnvFilterBuilder::from_default_env_or("info").build();

    logforth::starter_log::builder()
        .dispatch(|d| d.filter(filter).append(file))
        .try_apply()
        .map_err(|err| BabataError::internal(format!("Failed to initialize logger: {err}")))?;

    Ok(())
}
