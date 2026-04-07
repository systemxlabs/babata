pub mod agent;
pub mod channel;
pub mod cli;
pub mod config;
pub mod error;
pub mod http;
pub mod logging;
pub mod memory;
pub mod message;
pub mod provider;
pub mod skill;
pub mod system_prompt;
pub mod task;
pub mod tool;
pub mod utils;

pub type BabataResult<T> = std::result::Result<T, crate::error::BabataError>;
