mod store;

pub use store::*;

use crate::{
    BabataResult,
    message::Message,
};

pub struct Memory {
    message_store: MessageStore,
}

impl Memory {
    const CONTEXT_LIMIT: usize = 50;

    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            message_store: MessageStore::new()?,
        })
    }

    pub fn insert_messages(&self, messages: &[Message]) -> BabataResult<()> {
        self.message_store.insert_messages(messages)
    }

    pub fn scan_messages(&self) -> BabataResult<Vec<Message>> {
        self.message_store.scan_messages()
    }

    pub fn build_context(&self) -> BabataResult<Vec<Message>> {
        let messages = self.scan_messages()?;
        if messages.len() <= Self::CONTEXT_LIMIT {
            return Ok(messages);
        }

        Ok(messages[messages.len() - Self::CONTEXT_LIMIT..].to_vec())
    }
}
