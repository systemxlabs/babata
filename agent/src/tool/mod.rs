mod bash;

use crate::BabataResult;
use serde_json::Value;
use std::fmt::Debug;

#[async_trait::async_trait]
pub trait Tool: Debug + Send + Sync {
    fn spec(&self) -> &ToolSpec;
    async fn execute(&self, parameters: &str) -> BabataResult<String>;
}

pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}
