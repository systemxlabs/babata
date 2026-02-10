use uuid::Uuid;

use crate::{BabataResult, message::Message};

pub trait Memory {
    fn init(&self) -> BabataResult<()>;
    fn create_session(&self) -> BabataResult<Uuid>;
    fn add_messages(&self, session_id: Uuid, messages: &[Message]) -> BabataResult<()>;
    fn load_messages(&self, session_id: Uuid) -> BabataResult<Vec<Message>>;
    fn search_messages(
        &self,
        session_id: Uuid,
        query: &str,
        limit: usize,
    ) -> BabataResult<Vec<Message>>;
}
