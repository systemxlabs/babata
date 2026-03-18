use std::{collections::HashMap, sync::Arc, time::Duration};

use backon::{ExponentialBuilder, Retryable};
use log::{info, warn};

use crate::{
    BabataResult,
    agent::{
        Agent, AgentTask,
        babata::{
            GenerationRequest, Provider, Tool, ToolContext, ToolSpec, build_system_prompt,
            build_tools, create_provider, load_skills,
        },
    },
    channel::Channel,
    config::{AgentConfig, Config},
    error::BabataError,
    memory::{Memory, build_memory},
    message::Message,
};

const PROVIDER_RETRY_MAX_TIMES: usize = 3;
const PROVIDER_RETRY_MIN_DELAY_MS: u64 = 200;
const PROVIDER_RETRY_MAX_DELAY_SECS: u64 = 2;

#[derive(Debug)]
pub struct BabataAgent {
    pub memory: Box<dyn Memory>,
    pub tools: HashMap<String, Arc<dyn Tool>>,
}

impl BabataAgent {
    pub fn new(config: &Config, channels: HashMap<String, Arc<dyn Channel>>) -> BabataResult<Self> {
        let agent_config = config.get_agent(BabataAgent::name())?;
        #[allow(irrefutable_let_patterns)]
        let AgentConfig::Babata(babata_config) = agent_config else {
            return Err(BabataError::config(
                "Agent config for 'babata' must be of type 'BabataAgentConfig'",
            ));
        };

        let memory = build_memory(config, &babata_config.memory)?;
        let tools = build_tools(channels)?;

        Ok(Self { memory, tools })
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

#[async_trait::async_trait]
impl Agent for BabataAgent {
    fn name() -> &'static str {
        "babata"
    }

    fn description() -> &'static str {
        "Use for general tasks, task orchestration, task management, and simple scripting"
    }

    async fn execute(&self, task: AgentTask) -> BabataResult<()> {
        let config = Config::load()?;
        let agent_config = config.get_agent(BabataAgent::name())?;
        #[allow(irrefutable_let_patterns)]
        let AgentConfig::Babata(babata_config) = agent_config else {
            return Err(BabataError::config(
                "Agent config for 'babata' must be of type 'BabataAgentConfig'",
            ));
        };

        let provider_config = config.get_provider(&babata_config.provider)?;
        let provider = create_provider(provider_config)?;
        let model = babata_config.model.clone();

        let skills = load_skills()?;

        let tool_specs = self.collect_tool_specs();

        let system_prompt = build_system_prompt(&skills)?;

        let crate::agent::AgentTask {
            task_id,
            parent_task_id,
            root_task_id,
            prompt,
        } = task;
        let tool_context: ToolContext<'_> = ToolContext {
            task_id: &task_id,
            parent_task_id: parent_task_id.as_ref(),
            root_task_id: &root_task_id,
        };
        let context = self.memory.build_context(&prompt).await?;
        let mut conversation = vec![Message::UserPrompt { content: prompt }];

        let mut success = false;
        let max_steps = 100;
        for _ in 0..max_steps {
            let message = generate_with_retry(
                provider.as_ref(),
                &model,
                &system_prompt,
                &conversation,
                &context,
                &tool_specs,
            )
            .await?;
            info!("Provider returned message: {:?}", message);
            conversation.push(message.clone());

            match message {
                Message::AssistantResponse { .. } => {
                    success = true;
                    break;
                }
                Message::AssistantToolCalls { calls, .. } => {
                    if calls.is_empty() {
                        return Err(BabataError::provider("Provider returned empty tool calls"));
                    }

                    for call in calls {
                        if let Some(tool) = self.tools.get(&call.tool_name) {
                            let result = match tool.execute(&call.args, &tool_context).await {
                                Ok(result) => result,
                                Err(e) => format!("Tool execution failed with message: {e}"),
                            };
                            conversation.push(Message::ToolResult { call, result });
                        } else {
                            conversation.push(Message::ToolResult {
                                call: call.clone(),
                                result: format!("Unknown tool: {}", call.tool_name),
                            });
                        }
                    }
                }
                Message::UserPrompt { .. } | Message::ToolResult { .. } => {
                    return Err(BabataError::provider(
                        "Provider returned unsupported message type",
                    ));
                }
            }
        }

        if success {
            self.memory.append_messages(conversation).await?;
            Ok(())
        } else {
            Err(BabataError::provider(format!(
                "Max steps ({}) reached before final answer",
                max_steps
            )))
        }
    }
}

async fn generate_with_retry(
    provider: &dyn Provider,
    model: &str,
    system_prompt: &str,
    prompts: &[Message],
    context: &str,
    tool_specs: &[ToolSpec],
) -> BabataResult<Message> {
    let backoff = ExponentialBuilder::default()
        .with_min_delay(Duration::from_millis(PROVIDER_RETRY_MIN_DELAY_MS))
        .with_max_delay(Duration::from_secs(PROVIDER_RETRY_MAX_DELAY_SECS))
        .with_max_times(PROVIDER_RETRY_MAX_TIMES);

    (|| async {
        let response = provider
            .generate(GenerationRequest {
                system_prompt,
                model,
                prompts,
                context,
                tools: tool_specs,
            })
            .await?;
        Ok(response.message)
    })
    .retry(backoff)
    .when(|err| matches!(err, BabataError::Provider(_, _)))
    .notify(|err, wait| {
        warn!(
            "Provider generate failed: {:?}. Retrying in {:?}",
            err, wait
        )
    })
    .await
}
