use crate::{BabataResult, message::Message};

pub trait Memory {
    fn init(&self) -> BabataResult<()>;
    fn add(&self, message: &[Message]) -> BabataResult<()>;
    fn search(&self, query: &str, limit: usize) -> BabataResult<Vec<Message>>;
}
