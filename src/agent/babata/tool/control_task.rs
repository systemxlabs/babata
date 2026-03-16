use reqwest::Client;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolContext, ToolSpec},
    error::BabataError,
    http::DEFAULT_HTTP_BASE_URL,
};

#[derive(Debug)]
pub struct ControlTaskTool {
    spec: ToolSpec,
    http_client: Client,
}

impl ControlTaskTool {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            spec: ToolSpec {
                name: "control_task".to_string(),
                description:
                    "Control a task through a high-level action. Supported actions: pause, resume, cancel."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "task_id": {
                            "type": "string",
                            "description": "The UUID of the task to control"
                        },
                        "action": {
                            "type": "string",
                            "description": "The control action: pause, resume, or cancel"
                        }
                    },
                    "required": ["task_id", "action"]
                }),
            },
            http_client: Client::new(),
        })
    }
}

#[async_trait::async_trait]
impl Tool for ControlTaskTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, _context: &ToolContext) -> BabataResult<String> {
        let args: Value = serde_json::from_str(args)?;
        let task_id = args["task_id"]
            .as_str()
            .ok_or_else(|| BabataError::tool("Missing required parameter: task_id"))?;
        let action = args["action"]
            .as_str()
            .ok_or_else(|| BabataError::tool("Missing required parameter: action"))?;

        let task_id = Uuid::parse_str(task_id)
            .map_err(|err| BabataError::tool(format!("Invalid task_id '{}': {}", task_id, err)))?;
        validate_action(action)?;
        let url = format!("{DEFAULT_HTTP_BASE_URL}/tasks/{task_id}/{action}");

        let response = self.http_client.post(url).send().await.map_err(|err| {
            BabataError::tool(format!("Failed to call control_task HTTP API: {}", err))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::tool(format!(
                "control_task HTTP API returned status {}: {}",
                status, body
            )));
        }

        Ok(format!("Applied action '{}' to task '{}'", action, task_id))
    }
}

fn validate_action(action: &str) -> BabataResult<()> {
    match action {
        "pause" | "resume" | "cancel" => Ok(()),
        _ => Err(BabataError::tool(format!(
            "Invalid action '{}'; expected one of: pause, resume, cancel",
            action
        ))),
    }
}
