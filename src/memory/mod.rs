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

pub fn build_memory(config: &Config) -> BabataResult<Box<dyn Memory>> {
    match config.memory {
        MemoryConfig::Simple => {
            let memory = SimpleMemory::new()?;
            Ok(Box::new(memory))
        }
        MemoryConfig::Hybrid { .. } => {
            todo!()
        }
    }
}
