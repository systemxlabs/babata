use std::{collections::HashMap, sync::Arc, time::Duration};

use backon::{ExponentialBuilder, Retryable};
use log::{info, warn};

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolSpec, build_tools},
    agent::{
        Agent,
        babata::{
            GenerationRequest, Provider, Skill, SystemPromptFile, build_system_prompt,
            create_provider, load_skills, load_system_prompt_files,
        },
    },
    config::{AgentConfig, Config},
    error::BabataError,
    memory::{Memory, build_memory},
    message::{Content, Message},
};

const PROVIDER_RETRY_MAX_TIMES: usize = 3;
const PROVIDER_RETRY_MIN_DELAY_MS: u64 = 200;
const PROVIDER_RETRY_MAX_DELAY_SECS: u64 = 2;

#[derive(Debug)]
pub struct BabataAgent {
    pub provider: Arc<dyn Provider>,
    pub model: String,
    pub memory: Box<dyn Memory>,
    pub tools: HashMap<String, Arc<dyn Tool>>,
    pub system_prompt_files: Vec<SystemPromptFile>,
    pub skills: Vec<Skill>,
    pub max_steps: usize,
}

impl BabataAgent {
    pub fn new(config: &Config) -> BabataResult<Self> {
        let agent_config = config.get_agent(BabataAgent::name())?;
        let AgentConfig::Babata(babata_config) = agent_config else {
            return Err(BabataError::config(format!(
                "Agent config for 'babata' must be of type 'BabataAgentConfig'"
            )));
        };

        let provider_config = config.get_provider(&babata_config.provider)?;
        let provider = create_provider(provider_config)?;
        let model = babata_config.model.clone();
        let memory = build_memory(config, &babata_config.memory)?;
        let tools = build_tools();
        let system_prompt_files = load_system_prompt_files()?;
        let skills = load_skills()?;

        Ok(Self {
            provider,
            model,
            memory,
            tools,
            system_prompt_files,
            skills,
            max_steps: 100,
        })
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

    async fn generate_with_retry(
        &self,
        system_prompt: &str,
        prompts: &[Message],
        context: &[Message],
        tool_specs: &[ToolSpec],
    ) -> BabataResult<Message> {
        let backoff = ExponentialBuilder::default()
            .with_min_delay(Duration::from_millis(PROVIDER_RETRY_MIN_DELAY_MS))
            .with_max_delay(Duration::from_secs(PROVIDER_RETRY_MAX_DELAY_SECS))
            .with_max_times(PROVIDER_RETRY_MAX_TIMES);

        (|| async {
            let response = self
                .provider
                .generate(GenerationRequest {
                    system_prompt,
                    model: &self.model,
                    prompts,
                    context,
                    tools: tool_specs,
                })
                .await?;
            Ok(response.message)
        })
        .retry(backoff)
        .when(|err| matches!(err, BabataError::Provider(_, _)))
        .notify(|err, wait| warn!("Provider generate failed: {}. Retrying in {:?}", err, wait))
        .await
    }
}

#[async_trait::async_trait]
impl Agent for BabataAgent {
    fn name() -> &'static str {
        "babata"
    }

    async fn execute(&self, prompt: Vec<Content>) -> BabataResult<()> {
        let tool_specs = self.collect_tool_specs();

        let system_prompt = build_system_prompt(&self.system_prompt_files, &self.skills)?;
        let mut prompts = vec![Message::UserPrompt { content: prompt }];

        let context = self.memory.build_context(&prompts).await?;

        for _ in 0..self.max_steps {
            let message = self
                .generate_with_retry(&system_prompt, &prompts, &context, &tool_specs)
                .await?;
            info!("Provider returned message: {:?}", message);
            prompts.push(message.clone());

            match message {
                Message::AssistantResponse { .. } => return Ok(()),
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
                        prompts.push(Message::ToolResult { call, result });
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
}
