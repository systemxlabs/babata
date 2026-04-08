use reqwest::Client;
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    BabataResult,
    error::BabataError,
    http::DEFAULT_HTTP_BASE_URL,
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args},
};

#[derive(Debug)]
pub struct GetTaskTool {
    spec: ToolSpec,
    http_client: Client,
}

impl GetTaskTool {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            spec: ToolSpec {
                name: "get_task".to_string(),
                description: "Get task metadata by id.".to_string(),
                parameters: schemars::schema_for!(GetTaskArgs),
            },
            http_client: Client::new(),
        })
    }
}

#[async_trait::async_trait]
impl Tool for GetTaskTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, _context: &ToolContext<'_>) -> BabataResult<String> {
        let args: GetTaskArgs = parse_tool_args(args)?;
        let task_id = args.task_id;

        let response = self
            .http_client
            .get(format!("{DEFAULT_HTTP_BASE_URL}/api/tasks/{task_id}"))
            .send()
            .await
            .map_err(|err| {
                BabataError::tool(format!("Failed to call get_task HTTP API: {}", err))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::tool(format!(
                "get_task HTTP API returned status {}: {}",
                status, body
            )));
        }

        response.text().await.map_err(|err| {
            BabataError::tool(format!(
                "Failed to read get_task HTTP API response body: {}",
                err
            ))
        })
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct GetTaskArgs {
    #[schemars(description = "The UUID of the task to fetch")]
    task_id: Uuid,
}
