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
}
