mod definition;

pub use definition::*;

use std::{collections::HashMap, fmt::Debug, sync::Arc, time::Duration};

use backon::{ExponentialBuilder, Retryable};
use futures::future::join_all;
use log::{info, warn};
use uuid::Uuid;

use crate::{
    BabataResult,
    channel::Channel,
    config::Config,
    error::BabataError,
    memory::Memory,
    message::{Content, Message},
    provider::{GenerationRequest, Provider, create_provider},
    skill::load_skills,
    system_prompt::build_system_prompts,
    tool::{Tool, ToolContext, ToolSpec, build_tools},
};

const PROVIDER_RETRY_MAX_TIMES: usize = 3;
const PROVIDER_RETRY_MIN_DELAY_MS: u64 = 200;
const PROVIDER_RETRY_MAX_DELAY_SECS: u64 = 2;

#[derive(Debug, Clone)]
pub struct AgentTask {
    pub task_id: Uuid,
    pub parent_task_id: Option<Uuid>,
    pub root_task_id: Uuid,
    pub prompt: Vec<Content>,
}

pub fn build_agents(
    definitions: &[AgentDefinition],
    channels: HashMap<String, Arc<dyn Channel>>,
) -> BabataResult<HashMap<String, Arc<Agent>>> {
    let mut agents = HashMap::with_capacity(definitions.len());
    for def in definitions {
        let agent = Agent::new(def.clone(), channels.clone())?;
        agents.insert(def.frontmatter.name.clone(), Arc::new(agent));
    }
    Ok(agents)
}

#[derive(Debug)]
pub struct Agent {
    pub definition: AgentDefinition,
    pub memory: Memory,
    pub tools: HashMap<String, Arc<dyn Tool>>,
}

impl Agent {
    pub fn new(
        definition: AgentDefinition,
        channels: HashMap<String, Arc<dyn Channel>>,
    ) -> BabataResult<Self> {
        let memory = Memory::new(definition.agent_home()?)?;
        let tools = build_tools(channels)?;

        Ok(Self {
            definition,
            memory,
            tools,
        })
    }

    fn collect_tool_specs(&self) -> Vec<ToolSpec> {
        let allowed = &self.definition.frontmatter.allowed_tools;

        // If allowed_tools contains exactly one element "*", allow all tools
        let allow_all = allowed.len() == 1 && allowed[0] == "*";

        let mut specs: Vec<ToolSpec> = if allow_all {
            self.tools.values().map(|tool| tool.spec().clone()).collect()
        } else {
            self.tools
                .values()
                .filter(|tool| allowed.contains(&tool.spec().name))
                .map(|tool| tool.spec().clone())
                .collect()
        };

        specs.sort_by(|a, b| a.name.cmp(&b.name));
        specs
    }

    pub async fn execute(&self, task: AgentTask) -> BabataResult<()> {
        let config = Config::load()?;
        let definations = load_agent_definitions()?;

        let provider_config = config.get_provider(&self.definition.frontmatter.provider)?;
        let provider = create_provider(provider_config)?;
        let model = self.definition.frontmatter.model.clone();

        let skills = load_skills()?;

        let tool_specs = self.collect_tool_specs();

        let system_prompts = build_system_prompts(&config, &definations, &skills)?;

        let AgentTask {
            task_id,
            parent_task_id,
            root_task_id,
            prompt,
        } = task;
        let context = self.memory.build_context(&prompt).await?;
        let mut conversation = vec![Message::UserPrompt { content: prompt }];

        let mut success = false;
        let max_steps = 100;
        for _ in 0..max_steps {
            let message = generate_with_retry(
                provider.as_ref(),
                &model,
                &system_prompts,
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

                    // Execute tool calls in parallel
                    let tool_futures = calls.into_iter().map(|call| {
                        let tools = &self.tools;
                        let task_id = &task_id;
                        let parent_task_id = parent_task_id.as_ref();
                        let root_task_id = &root_task_id;

                        async move {
                            let tool_context: ToolContext<'_> = ToolContext {
                                task_id,
                                parent_task_id,
                                root_task_id,
                                call_id: &call.call_id,
                            };

                            let result = if let Some(tool) = tools.get(&call.tool_name) {
                                match tool.execute(&call.args, &tool_context).await {
                                    Ok(result) => result,
                                    Err(e) => format!("Tool execution failed with message: {e}"),
                                }
                            } else {
                                format!("Unknown tool: {}", call.tool_name)
                            };

                            Message::ToolResult { call, result }
                        }
                    });

                    let results = join_all(tool_futures).await;
                    conversation.extend(results);
                }
                Message::UserPrompt { .. } | Message::ToolResult { .. } => {
                    return Err(BabataError::provider(
                        "Provider returned unsupported message type",
                    ));
                }
            }
        }

        if success {
            self.memory.append_messages(&conversation)?;
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
    system_prompts: &[String],
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
                system_prompts,
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
