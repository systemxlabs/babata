mod bash;
mod read_file;
mod write_file;

pub use bash::*;
pub use read_file::*;
pub use write_file::*;

use crate::BabataResult;
use serde_json::Value;
use std::fmt::Debug;

#[async_trait::async_trait]
pub trait Tool: Debug + Send + Sync {
    fn spec(&self) -> &ToolSpec;
    async fn execute(&self, args: Value) -> BabataResult<String>;
}

#[derive(Debug, Clone)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}
