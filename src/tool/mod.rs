mod collaborate;
mod control_task;
mod create_task;
mod delete_tasks;
mod edit_file;
mod glob;
mod grep;
mod query_messages;
mod query_tasks;
mod query_truncation;
mod read_file;
mod shell;
mod sleep;
mod steer_task;
mod update_task;
mod user_feedback;
mod wait_task;
mod write_file;

pub use collaborate::*;
pub use control_task::*;
pub use create_task::*;
pub use delete_tasks::*;
pub use edit_file::*;
pub use glob::*;
pub use grep::*;
pub use query_messages::*;
pub use query_tasks::*;
pub use read_file::*;
pub use shell::*;
pub use sleep::*;
pub use steer_task::*;
pub use update_task::*;
pub use user_feedback::*;
pub use wait_task::*;
pub use write_file::*;

use crate::{BabataResult, channel::Channel, error::BabataError};
use schemars::Schema;
use serde::de::DeserializeOwned;
use std::{collections::HashMap, fmt::Debug, sync::Arc};
use uuid::Uuid;

#[async_trait::async_trait]
pub trait Tool: Debug + Send + Sync {
    fn spec(&self) -> &ToolSpec;
    async fn execute(&self, args: &str, context: &ToolContext<'_>) -> BabataResult<String>;
}

#[derive(Debug, Clone)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: Schema,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolContext<'a> {
    pub task_id: &'a Uuid,
    pub parent_task_id: Option<&'a Uuid>,
    pub root_task_id: &'a Uuid,
    pub call_id: &'a str,
}

impl ToolContext<'_> {
    #[cfg(test)]
    pub fn test() -> Self {
        use std::sync::OnceLock;

        static TASK_ID: OnceLock<Uuid> = OnceLock::new();
        let task_id = TASK_ID.get_or_init(Uuid::nil);
        Self {
            task_id,
            parent_task_id: None,
            root_task_id: task_id,
            call_id: "test_call_id",
        }
    }
}

pub fn parse_tool_args<T: DeserializeOwned>(args: &str) -> BabataResult<T> {
    serde_json::from_str(args)
        .map_err(|err| BabataError::tool(format!("Invalid tool arguments: {err}")))
}

/// Resolve a tool path argument: expand tilde and fall back to the current
/// working directory (or "." if that fails).
pub fn resolve_tool_path(path: Option<String>) -> String {
    path.map(|p| shellexpand::tilde(&p).to_string())
        .unwrap_or_else(|| {
            std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string())
        })
}

pub fn build_tools(
    channels: HashMap<String, Arc<dyn Channel>>,
) -> BabataResult<HashMap<String, Arc<dyn Tool>>> {
    let tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(ShellTool::new()),
        Arc::new(ReadFileTool::new()),
        Arc::new(WriteFileTool::new()),
        Arc::new(EditFileTool::new()),
        Arc::new(GlobTool::new()),
        Arc::new(GrepTool::new()),
        Arc::new(ControlTaskTool::new()?),
        Arc::new(CreateTaskTool::new()?),
        Arc::new(DeleteTasksTool::new()?),
        Arc::new(QueryMessagesTool::new()),
        Arc::new(QueryTasksTool::new()?),
        Arc::new(WaitTaskTool::new()?),
        Arc::new(SleepTool::new()),
        Arc::new(CollaborateTool::new()),
        Arc::new(SteerTaskTool::new()),
        Arc::new(UpdateTaskTool::new()?),
        Arc::new(UserFeedbackTool::new(channels)),
    ];

    let mut tool_map = HashMap::with_capacity(tools.len());
    for tool in tools {
        tool_map.insert(tool.spec().name.clone(), tool);
    }

    Ok(tool_map)
}
