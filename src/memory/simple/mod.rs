mod store;

use std::path::PathBuf;

pub use store::*;

use crate::{BabataResult, memory::Memory, message::Message, utils::babata_dir};

#[derive(Debug)]
pub struct SimpleMemory {
    message_store: MessageStore,
}

impl SimpleMemory {
    const CONTEXT_LIMIT: usize = 50;

    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            message_store: MessageStore::new()?,
        })
    }

    pub fn default_db_path() -> BabataResult<PathBuf> {
        let dir = babata_dir()?;
        Ok(dir.join("memory").join("message.db"))
    }
}

#[async_trait::async_trait]
impl Memory for SimpleMemory {
    async fn insert_messages(&self, messages: Vec<Message>) -> BabataResult<()> {
        self.message_store.insert_messages(&messages)
    }

    async fn build_context(&self, _prompts: &[Message]) -> BabataResult<Vec<Message>> {
        self.message_store.scan_messages(Some(Self::CONTEXT_LIMIT))
    }
}
