use reqwest::Client;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    BabataResult,
    error::BabataError,
    http::DEFAULT_HTTP_BASE_URL,
    message::Content,
    task::CreateTaskRequest,
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args},
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
                    "Create a task. By default this creates a subtask of the current task. Use task_type='roottask' to create a root task instead. Supports an optional agent override."
                        .to_string(),
                parameters: schemars::schema_for!(CreateTaskArgs),
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
        let args: CreateTaskArgs = parse_tool_args(args)?;

        let request_body = CreateTaskRequest {
            prompt: vec![Content::Text {
                text: args.prompt.clone(),
            }],
            agent: args.agent.clone(),
            parent_task_id: parse_parent_task_id(&args, context)?,
            never_ends: args.never_ends,
        };

        let response = self
            .http_client
            .post(format!("{DEFAULT_HTTP_BASE_URL}/api/tasks"))
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

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CreateTaskArgs {
    #[schemars(description = "The prompt for the task to create")]
    prompt: String,
    #[schemars(description = "Optional agent name for the task")]
    agent: Option<String>,
    #[schemars(description = "Boolean flag stored on the task record.")]
    never_ends: bool,
    #[schemars(
        description = "The type of task to create: 'subtask' or 'roottask'. Defaults to 'subtask'."
    )]
    task_type: Option<String>,
}

fn parse_parent_task_id(
    args: &CreateTaskArgs,
    context: &ToolContext<'_>,
) -> BabataResult<Option<uuid::Uuid>> {
    let task_type = args.task_type.as_deref().unwrap_or("subtask");
    match task_type {
        "roottask" => Ok(None),
        "subtask" => Ok(Some(*context.task_id)),
        _ => Err(BabataError::tool(format!(
            "Invalid task_type '{}'; expected 'subtask' or 'roottask'",
            task_type
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{CreateTaskArgs, parse_parent_task_id};
    use crate::tool::{ToolContext, parse_tool_args};
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn parse_parent_task_id_defaults_to_current_task_for_subtask() {
        let task_id = Uuid::new_v4();
        let context = ToolContext {
            task_id: &task_id,
            parent_task_id: None,
            root_task_id: &task_id,
            call_id: "test_call_id",
        };

        let args = parse_tool_args::<CreateTaskArgs>(
            &json!({ "prompt": "x", "never_ends": false }).to_string(),
        )
        .expect("parse args");
        let parent_task_id = parse_parent_task_id(&args, &context).expect("parent task id");
        assert_eq!(parent_task_id, Some(task_id));
    }

    #[test]
    fn parse_parent_task_id_returns_none_for_root_task() {
        let task_id = Uuid::new_v4();
        let context = ToolContext {
            task_id: &task_id,
            parent_task_id: None,
            root_task_id: &task_id,
            call_id: "test_call_id",
        };

        let args = parse_tool_args::<CreateTaskArgs>(
            &json!({ "prompt": "x", "never_ends": false, "task_type": "roottask" }).to_string(),
        )
        .expect("parse args");
        let parent_task_id = parse_parent_task_id(&args, &context).expect("root task");
        assert_eq!(parent_task_id, None);
    }

    #[test]
    fn parse_never_ends_requires_parameter() {
        let error = parse_tool_args::<CreateTaskArgs>(&json!({ "prompt": "x" }).to_string())
            .expect_err("missing never_ends should fail");
        assert!(
            error
                .to_string()
                .contains("Missing required parameter: never_ends")
        );
    }

    #[test]
    fn parse_never_ends_requires_boolean_value() {
        let error = parse_tool_args::<CreateTaskArgs>(
            &json!({ "prompt": "x", "never_ends": "yes" }).to_string(),
        )
        .expect_err("string should fail");
        assert!(error.to_string().contains("Invalid tool arguments"));
    }

    #[test]
    fn parse_never_ends_accepts_boolean_value() {
        let args = parse_tool_args::<CreateTaskArgs>(
            &json!({ "prompt": "x", "never_ends": true }).to_string(),
        )
        .expect("parse");
        assert!(args.never_ends);
    }

    #[test]
    fn parse_args_extracts_prompt_and_agent() {
        let args = parse_tool_args::<CreateTaskArgs>(
            &json!({
                "prompt": "Test prompt",
                "agent": "test_agent",
                "never_ends": false
            })
            .to_string(),
        )
        .expect("parse args");
        assert_eq!(args.prompt, "Test prompt");
        assert_eq!(args.agent, Some("test_agent".to_string()));
    }

    #[test]
    fn parse_args_agent_is_optional() {
        let args = parse_tool_args::<CreateTaskArgs>(
            &json!({ "prompt": "Test prompt", "never_ends": false }).to_string(),
        )
        .expect("parse args");
        assert_eq!(args.prompt, "Test prompt");
        assert_eq!(args.agent, None);
    }

    #[test]
    fn parse_args_allows_empty_prompt_string() {
        let args = parse_tool_args::<CreateTaskArgs>(
            &json!({ "prompt": "   ", "never_ends": false }).to_string(),
        )
        .expect("empty prompt still parses");
        assert_eq!(args.prompt, "   ");
    }

    #[test]
    fn parse_args_rejects_missing_prompt() {
        let error = parse_tool_args::<CreateTaskArgs>(&json!({ "never_ends": false }).to_string())
            .expect_err("missing prompt");
        assert!(
            error
                .to_string()
                .contains("Missing required parameter: prompt")
        );
    }
}
