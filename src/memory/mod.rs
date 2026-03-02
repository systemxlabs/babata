mod store;

pub use store::*;

use crate::{BabataResult, message::Message};

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

    pub fn scan_messages(&self, limit: Option<usize>) -> BabataResult<Vec<Message>> {
        self.message_store.scan_messages(limit)
    }

    pub fn build_context(&self, user_messages: Vec<Message>) -> BabataResult<Vec<Message>> {
        let mut context = self.scan_messages(Some(Self::CONTEXT_LIMIT))?;
        context.extend(user_messages);
        Ok(context)
    }
}
