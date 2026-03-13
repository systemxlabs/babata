mod control_task;
mod create_task;
mod edit_file;
mod get_task;
mod list_task;
mod read_file;
mod shell;
mod sleep;
mod user_feedback;
mod write_file;

pub use control_task::*;
pub use create_task::*;
pub use edit_file::*;
pub use get_task::*;
pub use list_task::*;
pub use read_file::*;
pub use shell::*;
pub use sleep::*;
pub use user_feedback::*;
pub use write_file::*;

use crate::{BabataResult, channel::Channel};
use serde_json::Value;
use std::{collections::HashMap, fmt::Debug, sync::Arc};

#[async_trait::async_trait]
pub trait Tool: Debug + Send + Sync {
    fn spec(&self) -> &ToolSpec;
    async fn execute(&self, args: &str) -> BabataResult<String>;
}

#[derive(Debug, Clone)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: Value,
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
        Arc::new(CreateTaskTool::new()?),
        Arc::new(GetTaskTool::new()?),
        Arc::new(ListTaskTool::new()?),
        Arc::new(SleepTool::new()),
        Arc::new(UserFeedbackTool::new(channels)),
    ];

    let mut tool_map = HashMap::with_capacity(tools.len());
    for tool in tools {
        tool_map.insert(tool.spec().name.clone(), tool);
    }

    Ok(tool_map)
}
