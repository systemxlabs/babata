mod telegram;

pub use telegram::*;

use std::fmt::Debug;

use crate::{BabataResult, message::Message};

#[async_trait::async_trait]
pub trait Channel: Debug + Send + Sync {
    // Send messages to the channel
    async fn send(&self, messages: &[Message]) -> BabataResult<()>;
    // Receive messages, blocking until messages are available
    async fn receive(&self) -> BabataResult<Vec<Message>>;
    // Try to receive messages, returning None if no messages are available
    async fn try_receive(&self) -> BabataResult<Option<Vec<Message>>>;
}
