pub mod hybrid;
pub mod simple;

use crate::{BabataResult, config::Config, memory::simple::SimpleMemory, message::Message};
use std::fmt::Debug;

#[async_trait::async_trait]
pub trait Memory: Debug + Sync + Send {
    async fn insert_messages(&self, messages: Vec<Message>) -> BabataResult<()>;
    async fn build_context(&self, prompts: &[Message]) -> BabataResult<Vec<Message>>;
}

pub fn build_memory(_config: &Config) -> BabataResult<Box<dyn Memory>> {
    let memory = SimpleMemory::new()?;
    Ok(Box::new(memory))
}
