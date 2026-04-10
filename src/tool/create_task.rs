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

#[derive(Debug, Deserialize)]
struct CreateTaskResponse {
    task_id: String,
    status: String,
}

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
                description: "Create a root task or subtask to run asynchronously.".to_string(),
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

        let parent_task_id = match args.task_type.unwrap_or_default() {
            TaskType::RootTask => None,
            TaskType::Subtask => Some(*context.task_id),
        };
        let request_body = CreateTaskRequest {
            description: args.description,
            prompt: vec![Content::Text { text: args.prompt }],
            agent: args.agent,
            parent_task_id,
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

        let response_body = response.json::<CreateTaskResponse>().await.map_err(|err| {
            BabataError::tool(format!(
                "Failed to deserialize create_task HTTP API response: {}",
                err
            ))
        })?;

        Ok(format!(
            "Task created successfully. Task ID: {}, Status: {}",
            response_body.task_id, response_body.status
        ))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CreateTaskArgs {
    #[schemars(description = "Short task description stored on the task record")]
    description: String,
    #[schemars(description = "The prompt for the task to create")]
    prompt: String,
    #[schemars(description = "The agent name for the task")]
    agent: String,
    #[schemars(description = "Boolean flag stored on the task record.")]
    never_ends: bool,
    #[schemars(
        description = "The type of task to create: 'subtask' or 'roottask'. Defaults to 'subtask'."
    )]
    task_type: Option<TaskType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum TaskType {
    #[default]
    Subtask,
    RootTask,
}

#[cfg(test)]
mod tests {
    use super::{CreateTaskArgs, TaskType};
    use crate::tool::{ToolContext, parse_tool_args};
    use serde_json::json;
    use uuid::Uuid;

    fn parent_task_id(args: &CreateTaskArgs, context: &ToolContext<'_>) -> Option<Uuid> {
        match args.task_type.unwrap_or_default() {
            TaskType::RootTask => None,
            TaskType::Subtask => Some(*context.task_id),
        }
    }

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
            &json!({ "description": "x", "prompt": "x", "agent": "test_agent", "never_ends": false }).to_string(),
        )
        .expect("parse args");
        assert_eq!(parent_task_id(&args, &context), Some(task_id));
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
            &json!({ "description": "x", "prompt": "x", "agent": "test_agent", "never_ends": false, "task_type": "roottask" }).to_string(),
        )
        .expect("parse args");
        assert_eq!(parent_task_id(&args, &context), None);
        assert_eq!(args.task_type, Some(TaskType::RootTask));
    }

    #[test]
    fn parse_task_type_rejects_unknown_value() {
        let error = parse_tool_args::<CreateTaskArgs>(
            &json!({ "description": "x", "prompt": "x", "agent": "test_agent", "never_ends": false, "task_type": "other" }).to_string(),
        )
        .expect_err("invalid task_type should fail");

        assert!(error.to_string().contains("Invalid tool arguments"));
    }

    #[test]
    fn parse_never_ends_requires_parameter() {
        let error = parse_tool_args::<CreateTaskArgs>(
            &json!({ "description": "x", "prompt": "x", "agent": "test_agent" }).to_string(),
        )
        .expect_err("missing never_ends should fail");
        assert!(
            error.to_string().contains("Invalid tool arguments")
                && error.to_string().contains("missing field `never_ends`")
        );
    }

    #[test]
    fn parse_never_ends_requires_boolean_value() {
        let error = parse_tool_args::<CreateTaskArgs>(
            &json!({ "description": "x", "prompt": "x", "agent": "test_agent", "never_ends": "yes" }).to_string(),
        )
        .expect_err("string should fail");
        assert!(error.to_string().contains("Invalid tool arguments"));
    }

    #[test]
    fn parse_never_ends_accepts_boolean_value() {
        let args = parse_tool_args::<CreateTaskArgs>(
            &json!({ "description": "x", "prompt": "x", "agent": "test_agent", "never_ends": true }).to_string(),
        )
        .expect("parse");
        assert!(args.never_ends);
    }

    #[test]
    fn parse_args_extracts_prompt_and_agent() {
        let args = parse_tool_args::<CreateTaskArgs>(
            &json!({
                "description": "Task summary",
                "prompt": "Test prompt",
                "agent": "test_agent",
                "never_ends": false
            })
            .to_string(),
        )
        .expect("parse args");
        assert_eq!(args.description, "Task summary");
        assert_eq!(args.prompt, "Test prompt");
        assert_eq!(args.agent, "test_agent".to_string());
    }

    #[test]
    fn parse_args_allows_empty_prompt_string() {
        let args = parse_tool_args::<CreateTaskArgs>(
            &json!({ "description": "Task summary", "prompt": "   ", "agent": "test_agent", "never_ends": false })
                .to_string(),
        )
        .expect("empty prompt still parses");
        assert_eq!(args.prompt, "   ");
    }

    #[test]
    fn parse_args_rejects_missing_description() {
        let error = parse_tool_args::<CreateTaskArgs>(
            &json!({ "prompt": "Test prompt", "agent": "test_agent", "never_ends": false })
                .to_string(),
        )
        .expect_err("missing description");
        assert!(
            error.to_string().contains("Invalid tool arguments")
                && error.to_string().contains("missing field `description`")
        );
    }

    #[test]
    fn parse_args_rejects_missing_prompt() {
        let error = parse_tool_args::<CreateTaskArgs>(
            &json!({ "description": "Task summary", "agent": "test_agent", "never_ends": false })
                .to_string(),
        )
        .expect_err("missing prompt");
        assert!(
            error.to_string().contains("Invalid tool arguments")
                && error.to_string().contains("missing field `prompt`")
        );
    }

    #[test]
    fn parse_args_rejects_missing_agent() {
        let error = parse_tool_args::<CreateTaskArgs>(
            &json!({
                "description": "Task summary",
                "prompt": "Test prompt",
                "never_ends": false
            })
            .to_string(),
        )
        .expect_err("missing agent");
        assert!(
            error.to_string().contains("Invalid tool arguments")
                && error.to_string().contains("missing field `agent`")
        );
    }
}
