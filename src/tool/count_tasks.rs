use serde_json::{Value, json};

use crate::{
    BabataResult,
    error::BabataError,
    task::{TaskStatus, TaskStore},
    tool::{Tool, ToolContext, ToolSpec},
};

#[derive(Debug)]
pub struct CountTasksTool {
    spec: ToolSpec,
    task_store: TaskStore,
}

impl CountTasksTool {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            spec: ToolSpec {
                name: "count_tasks".to_string(),
                description: "Count tasks. Supports optional status filter.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "status": {
                            "type": "string",
                            "description": "Optional task status filter"
                        }
                    }
                }),
            },
            task_store: TaskStore::new()?,
        })
    }
}

#[async_trait::async_trait]
impl Tool for CountTasksTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, _context: &ToolContext<'_>) -> BabataResult<String> {
        let status = parse_args(args)?;

        let count = self.task_store.count_tasks(status)?;
        Ok(count.to_string())
    }
}

fn parse_args(args: &str) -> BabataResult<Option<TaskStatus>> {
    let args: Value = serde_json::from_str(args)?;
    let status =
        match args["status"].as_str() {
            Some(status) => Some(status.parse::<TaskStatus>().map_err(|err| {
                BabataError::tool(format!("Invalid status '{}': {}", status, err))
            })?),
            None => None,
        };
    Ok(status)
}
