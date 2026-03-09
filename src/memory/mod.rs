#[cfg(feature = "mem-hybrid")]
mod hybrid;
mod simple;

use crate::{
    BabataResult,
    config::{Config, MemoryConfig},
    memory::simple::SimpleMemory,
    message::Message,
};
use std::fmt::Debug;

#[async_trait::async_trait]
pub trait Memory: Debug + Sync + Send {
    async fn insert_messages(&self, messages: Vec<Message>) -> BabataResult<()>;
    async fn build_context(&self, prompts: &[Message]) -> BabataResult<Vec<Message>>;
}

pub fn build_memory(config: &Config, memory_name: &str) -> BabataResult<Box<dyn Memory>> {
    let memory_config = config
        .get_memory(memory_name)
        .unwrap_or(&MemoryConfig::Simple);
    match memory_config {
        MemoryConfig::Simple => {
            let memory = SimpleMemory::new()?;
            Ok(Box::new(memory))
        }
        MemoryConfig::Hybrid(hybrid_config) => {
            #[cfg(feature = "mem-hybrid")]
            {
                let memory = hybrid::build_memory(hybrid_config)?;
                Ok(Box::new(memory))
            }
            #[cfg(not(feature = "mem-hybrid"))]
            {
                let _ = hybrid_config;
                Err(crate::error::BabataError::config(
                    "Hybrid memory requires the 'mem-hybrid' feature",
                ))
            }
        }
    }
}
