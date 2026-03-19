use reqwest::Client;
use serde_json::{Value, json};

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolContext, ToolSpec},
    error::BabataError,
    http::DEFAULT_HTTP_BASE_URL,
    message::Content,
    task::CreateTaskRequest,
};

#[derive(Debug)]
pub struct CreateTaskTool {
    spec: ToolSpec,
    http_client: Client,
}

impl CreateTaskTool {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            spec: ToolSpec {
                name: "create_task".to_string(),
                description:
                    "Create a task through the local HTTP API. By default this creates a subtask of the current task. Use task_type='root' to create a root task instead. Supports an optional agent override."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "prompt": {
                            "type": "string",
                            "description": "The prompt for the task to create"
                        },
                        "agent": {
                            "type": "string",
                            "description": "Optional agent name for the task"
                        },
                        "task_type": {
                            "type": "string",
                            "description": "The type of task to create: 'subtask' or 'root'. Defaults to 'subtask'."
                        }
                    },
                    "required": ["prompt"]
                }),
            },
            http_client: Client::new(),
        })
    }
}

#[async_trait::async_trait]
impl Tool for CreateTaskTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, context: &ToolContext<'_>) -> BabataResult<String> {
        let args: Value = serde_json::from_str(args)?;
        let prompt = args["prompt"]
            .as_str()
            .ok_or_else(|| BabataError::tool("Missing required parameter: prompt"))?;

        if prompt.trim().is_empty() {
            return Err(BabataError::tool("prompt cannot be empty"));
        }

        let request_body = CreateTaskRequest {
            prompt: vec![Content::Text {
                text: prompt.to_string(),
            }],
            agent: args["agent"].as_str().map(ToOwned::to_owned),
            parent_task_id: parse_parent_task_id(&args, context)?,
        };

        let response = self
            .http_client
            .post(format!("{DEFAULT_HTTP_BASE_URL}/tasks"))
            .json(&request_body)
            .send()
            .await
            .map_err(|err| {
                BabataError::tool(format!("Failed to call create_task HTTP API: {}", err))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::tool(format!(
                "create_task HTTP API returned status {}: {}",
                status, body
            )));
        }

        response.text().await.map_err(|err| {
            BabataError::tool(format!(
                "Failed to read create_task HTTP API response body: {}",
                err
            ))
        })
    }
}

fn parse_parent_task_id(
    args: &Value,
    context: &ToolContext<'_>,
) -> BabataResult<Option<uuid::Uuid>> {
    let task_type = args["task_type"].as_str().unwrap_or("subtask");
    match task_type {
        "root" => Ok(None),
        "subtask" => Ok(Some(*context.task_id)),
        _ => Err(BabataError::tool(format!(
            "Invalid task_type '{}'; expected 'subtask' or 'root'",
            task_type
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_parent_task_id;
    use crate::agent::babata::ToolContext;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn parse_parent_task_id_defaults_to_current_task_for_subtask() {
        let task_id = Uuid::new_v4();
        let context = ToolContext {
            task_id: &task_id,
            parent_task_id: None,
            root_task_id: &task_id,
        };

        let parent_task_id = parse_parent_task_id(&json!({}), &context).expect("parent task id");
        assert_eq!(parent_task_id, Some(task_id));
    }

    #[test]
    fn parse_parent_task_id_returns_none_for_root_task() {
        let task_id = Uuid::new_v4();
        let context = ToolContext {
            task_id: &task_id,
            parent_task_id: None,
            root_task_id: &task_id,
        };

        let parent_task_id =
            parse_parent_task_id(&json!({ "task_type": "root" }), &context).expect("root task");
        assert_eq!(parent_task_id, None);
    }
}
