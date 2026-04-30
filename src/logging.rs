use std::{env, num::NonZeroUsize};

use logforth::{
    append::file::FileBuilder, filter::env_filter::EnvFilterBuilder, layout::JsonLayout,
};

use crate::{BabataResult, error::BabataError, utils::babata_dir};

/// Log an info message prefixed with `[task_id]`.
#[macro_export]
macro_rules! task_info {
    ($task_id:expr, $($arg:tt)*) => {
        log::info!("[{}] {}", $task_id, std::format_args!($($arg)*))
    };
}

/// Log a warn message prefixed with `[task_id]`.
#[macro_export]
macro_rules! task_warn {
    ($task_id:expr, $($arg:tt)*) => {
        log::warn!("[{}] {}", $task_id, std::format_args!($($arg)*))
    };
}

/// Log an error message prefixed with `[task_id]`.
#[macro_export]
macro_rules! task_error {
    ($task_id:expr, $($arg:tt)*) => {
        log::error!("[{}] {}", $task_id, std::format_args!($($arg)*))
    };
}

pub fn init() -> BabataResult<()> {
    match LogOutput::from_env()? {
        LogOutput::File => init_file_logger(),
        LogOutput::Stdio => init_stdio_logger(),
    }
}

fn init_file_logger() -> BabataResult<()> {
    let log_dir = babata_dir()?.join("logs");
    let max_log_files = NonZeroUsize::new(7).expect("non-zero");

    let file = FileBuilder::new(log_dir, "babata")
        .layout(JsonLayout::default())
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

fn init_stdio_logger() -> BabataResult<()> {
    let filter = EnvFilterBuilder::from_default_env_or("info").build();

    logforth::starter_log::builder()
        .dispatch(|d| {
            d.filter(filter)
                .append(logforth::append::Stdout::default().with_layout(JsonLayout::default()))
        })
        .try_apply()
        .map_err(|err| BabataError::internal(format!("Failed to initialize logger: {err}")))?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogOutput {
    File,
    Stdio,
}

impl LogOutput {
    fn from_env() -> BabataResult<Self> {
        match env::var("LOG_OUTPUT") {
            Ok(value) => Self::parse(&value),
            Err(env::VarError::NotPresent) => Ok(Self::File),
            Err(err) => Err(BabataError::internal(format!(
                "Failed to read LOG_OUTPUT from environment: {err}"
            ))),
        }
    }

    fn parse(raw: &str) -> BabataResult<Self> {
        let value = raw.trim().to_ascii_lowercase();

        match value.as_str() {
            "" | "file" => Ok(Self::File),
            "stdio" => Ok(Self::Stdio),
            _ => Err(BabataError::internal(format!(
                "Invalid LOG_OUTPUT '{}'. Supported values: file, stdio",
                raw
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LogOutput;

    #[test]
    fn parse_log_output_supports_file() {
        assert_eq!(
            LogOutput::parse("file").expect("parse file"),
            LogOutput::File
        );
    }

    #[test]
    fn parse_log_output_supports_stdio() {
        assert_eq!(
            LogOutput::parse("stdio").expect("parse stdio"),
            LogOutput::Stdio
        );
    }

    #[test]
    fn parse_log_output_is_case_insensitive() {
        assert_eq!(
            LogOutput::parse("StDiO").expect("parse mixed case"),
            LogOutput::Stdio
        );
    }

    #[test]
    fn parse_log_output_rejects_invalid_value() {
        let err = LogOutput::parse("unknown").expect_err("invalid value should fail");
        assert!(err.to_string().contains("Invalid LOG_OUTPUT"));
    }
}
