use std::{collections::HashMap, sync::Arc};

use log::info;

use crate::{
    BabataResult,
    error::BabataError,
    message::Message,
    provider::{GenerationReqest, Provider},
    skill::Skill,
    system_prompt::{SystemPromptFile, build_system_prompt},
    tool::{Tool, ToolSpec},
};

pub struct AgentTask {
    pub messages: Vec<Message>,
    pub provider: Arc<dyn Provider>,
    pub model: String,
    pub tools: HashMap<String, Arc<dyn Tool>>,
    pub system_prompt_files: Vec<SystemPromptFile>,
    pub skills: Vec<Skill>,
    pub max_steps: usize,
}

impl AgentTask {
    pub fn new(
        messages: Vec<Message>,
        provider: Arc<dyn Provider>,
        model: String,
        tools: HashMap<String, Arc<dyn Tool>>,
        system_prompt_files: Vec<SystemPromptFile>,
        skills: Vec<Skill>,
    ) -> Self {
        AgentTask {
            messages,
            provider,
            model,
            tools,
            system_prompt_files,
            skills,
            max_steps: 100,
        }
    }

    pub async fn run(&self) -> BabataResult<Message> {
        if self.max_steps == 0 {
            return Err(BabataError::internal("max_steps must be greater than 0"));
        }

        let mut messages = self.messages.clone();
        let tool_specs = self.collect_tool_specs();
        let system_prompt = build_system_prompt(&self.system_prompt_files, &self.skills);

        for _ in 0..self.max_steps {
            let response = self
                .provider
                .generate(GenerationReqest {
                    system_prompt: &system_prompt,
                    model: &self.model,
                    messages: &messages,
                    tools: &tool_specs,
                })
                .await?;

            let message = response.message;
            info!("Provider returned message: {:?}", message);
            messages.push(message.clone());

            match message {
                Message::AssistantResponse { .. } => return Ok(message),
                Message::AssistantToolCalls { calls, .. } => {
                    if calls.is_empty() {
                        return Err(BabataError::provider("Provider returned empty tool calls"));
                    }

                    for call in calls {
                        let tool = self.tools.get(&call.tool_name).ok_or_else(|| {
                            BabataError::tool(format!(
                                "Unknown tool requested by provider: {}",
                                call.tool_name
                            ))
                        })?;

                        let result = match tool.execute(&call.args).await {
                            Ok(result) => result,
                            Err(e) => format!("Tool execution failed with message: {e}"),
                        };
                        messages.push(Message::ToolResult { call, result });
                    }
                }
                Message::UserPrompt { .. } | Message::ToolResult { .. } => {
                    return Err(BabataError::provider(
                        "Provider returned unsupported message type",
                    ));
                }
            }
        }

        Err(BabataError::provider(format!(
            "Max steps ({}) reached before final answer",
            self.max_steps
        )))
    }

    fn collect_tool_specs(&self) -> Vec<ToolSpec> {
        let mut specs: Vec<ToolSpec> = self
            .tools
            .values()
            .map(|tool| tool.spec().clone())
            .collect();
        specs.sort_by(|a, b| a.name.cmp(&b.name));
        specs
    }
}
