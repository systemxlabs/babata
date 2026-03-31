use serde_json::{Value, json};

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolContext, ToolSpec},
    error::BabataError,
    http::ListTasksResponse,
    task::{TaskStatus, TaskStore},
};

#[derive(Debug)]
pub struct ListTasksTool {
    spec: ToolSpec,
    task_store: TaskStore,
}

impl ListTasksTool {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            spec: ToolSpec {
                name: "list_tasks".to_string(),
                description:
                    "List tasks. Supports optional status filter and offset. The limit parameter is required."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "status": {
                            "type": "string",
                            "description": "Optional task status filter"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Required max number of tasks to return"
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Optional number of tasks to skip before returning results"
                        }
                    },
                    "required": ["limit"]
                }),
            },
            task_store: TaskStore::new()?,
        })
    }
}

#[async_trait::async_trait]
impl Tool for ListTasksTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, _context: &ToolContext<'_>) -> BabataResult<String> {
        let args: Value = serde_json::from_str(args)?;
        let status = match args["status"].as_str() {
            Some(status) => Some(status.parse::<TaskStatus>().map_err(|err| {
                BabataError::tool(format!("Invalid status '{}': {}", status, err))
            })?),
            None => None,
        };
        let limit = match args["limit"].as_u64() {
            Some(limit) => usize::try_from(limit).map_err(|_| {
                BabataError::tool(format!("limit '{}' is too large for this platform", limit))
            })?,
            None => return Err(BabataError::tool("limit is required")),
        };
        let offset = match args["offset"].as_u64() {
            Some(offset) => Some(usize::try_from(offset).map_err(|_| {
                BabataError::tool(format!(
                    "offset '{}' is too large for this platform",
                    offset
                ))
            })?),
            None => None,
        };

        let tasks = self.task_store.list_tasks(status, limit, offset)?;
        let response = ListTasksResponse::from_records(tasks);
        serde_json::to_string(&response).map_err(Into::into)
    }
}
