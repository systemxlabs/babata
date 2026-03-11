pub mod babata;

use std::fmt::Debug;

use crate::{BabataResult, message::Content};

#[async_trait::async_trait]
pub trait Agent: Debug + Send + Sync {
    fn name() -> &'static str;
    async fn execute(&self, prompt: Vec<Content>) -> BabataResult<()>;
}
