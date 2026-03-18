mod control_task;
mod create_subtask;
mod edit_file;
mod get_task;
mod list_subtasks;
mod list_tasks;
mod read_file;
mod shell;
mod sleep;
mod user_feedback;
mod wait_task;
mod write_file;

pub use control_task::*;
pub use create_subtask::*;
pub use edit_file::*;
pub use get_task::*;
pub use list_subtasks::*;
pub use list_tasks::*;
pub use read_file::*;
pub use shell::*;
pub use sleep::*;
pub use user_feedback::*;
pub use wait_task::*;
pub use write_file::*;

use crate::{BabataResult, channel::Channel};
use serde_json::Value;
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
    pub parameters: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolContext<'a> {
    pub task_id: &'a Uuid,
    pub parent_task_id: Option<&'a Uuid>,
    pub root_task_id: &'a Uuid,
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
        }
    }
}

pub fn build_tools(
    channels: HashMap<String, Arc<dyn Channel>>,
) -> BabataResult<HashMap<String, Arc<dyn Tool>>> {
    let tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(ShellTool::new()),
        Arc::new(ReadFileTool::new()),
        Arc::new(WriteFileTool::new()),
        Arc::new(EditFileTool::new()),
        Arc::new(ControlTaskTool::new()?),
        Arc::new(CreateSubtaskTool::new()?),
        Arc::new(GetTaskTool::new()?),
        Arc::new(ListTasksTool::new()?),
        Arc::new(ListSubtasksTool::new()?),
        Arc::new(WaitTaskTool::new()?),
        Arc::new(SleepTool::new()),
        Arc::new(UserFeedbackTool::new(channels)),
    ];

    let mut tool_map = HashMap::with_capacity(tools.len());
    for tool in tools {
        tool_map.insert(tool.spec().name.clone(), tool);
    }

    Ok(tool_map)
}
