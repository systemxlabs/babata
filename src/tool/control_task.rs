use reqwest::Client;
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    BabataResult,
    error::BabataError,
    http::{ControlTaskRequest, DEFAULT_HTTP_BASE_URL, TaskAction},
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args},
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
                parameters: schemars::schema_for!(ControlTaskArgs),
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

    async fn execute(&self, args: &str, _context: &ToolContext<'_>) -> BabataResult<String> {
        let args: ControlTaskArgs = parse_tool_args(args)?;

        let url = format!("{DEFAULT_HTTP_BASE_URL}/api/tasks/{}/control", args.task_id);

        let response = self
            .http_client
            .post(url)
            .json(&ControlTaskRequest {
                action: args.action,
            })
            .send()
            .await
            .map_err(|err| {
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

        Ok(format!(
            "Applied action '{}' to task '{}'",
            args.action, args.task_id
        ))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ControlTaskArgs {
    #[schemars(description = "The UUID of the task to control")]
    task_id: Uuid,
    #[schemars(description = "The control action: pause, resume, or cancel")]
    action: TaskAction,
}
