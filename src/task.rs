use std::{collections::HashMap, sync::Arc, time::Duration};

use backon::{ExponentialBuilder, Retryable};
use log::{info, warn};

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
    pub prompts: Vec<Message>,
    pub context: Vec<Message>,
    pub provider: Arc<dyn Provider>,
    pub model: String,
    pub tools: HashMap<String, Arc<dyn Tool>>,
    pub system_prompt_files: Vec<SystemPromptFile>,
    pub skills: Vec<Skill>,
    pub max_steps: usize,
}

const PROVIDER_RETRY_MAX_TIMES: usize = 3;
const PROVIDER_RETRY_MIN_DELAY_MS: u64 = 200;
const PROVIDER_RETRY_MAX_DELAY_SECS: u64 = 2;

impl AgentTask {
    pub fn new(
        prompts: Vec<Message>,
        context: Vec<Message>,
        provider: Arc<dyn Provider>,
        model: String,
        tools: HashMap<String, Arc<dyn Tool>>,
        system_prompt_files: Vec<SystemPromptFile>,
        skills: Vec<Skill>,
    ) -> Self {
        AgentTask {
            prompts,
            context,
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

        let mut prompts = self.prompts.clone();
        let tool_specs = self.collect_tool_specs();

        let system_prompt = build_system_prompt(&self.system_prompt_files, &self.skills)?;

        for _ in 0..self.max_steps {
            let message = self
                .generate_with_retry(&system_prompt, &prompts, &self.context, &tool_specs)
                .await?;
            info!("Provider returned message: {:?}", message);
            prompts.push(message.clone());

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
                .generate(GenerationReqest {
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

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use super::AgentTask;
    use crate::{
        error::BabataError,
        message::{Content, Message},
        provider::{
            GenerationReqest, GenerationResponse, InteractionRequest, InteractionResponse, Model,
            Provider,
        },
    };

    const TEST_MODELS: &[Model] = &[Model {
        provider: "test",
        name: "test-model",
        context_length: 8_192,
    }];

    #[derive(Debug, Clone, Copy)]
    enum FailureMode {
        Provider { times: usize },
        Internal { times: usize },
    }

    #[derive(Debug)]
    struct RetryTestProvider {
        calls: Arc<AtomicUsize>,
        failure_mode: FailureMode,
    }

    impl RetryTestProvider {
        fn new(calls: Arc<AtomicUsize>, failure_mode: FailureMode) -> Self {
            Self {
                calls,
                failure_mode,
            }
        }
    }

    #[async_trait::async_trait]
    impl Provider for RetryTestProvider {
        fn name() -> &'static str {
            "test"
        }

        fn supported_models() -> &'static [Model] {
            TEST_MODELS
        }

        async fn generate<'a>(
            &self,
            _request: GenerationReqest<'a>,
        ) -> crate::BabataResult<GenerationResponse> {
            let attempt = self.calls.fetch_add(1, Ordering::SeqCst);

            match self.failure_mode {
                FailureMode::Provider { times } if attempt < times => {
                    return Err(BabataError::provider("transient provider failure"));
                }
                FailureMode::Internal { times } if attempt < times => {
                    return Err(BabataError::internal("non-provider failure"));
                }
                _ => {}
            }

            Ok(GenerationResponse {
                message: Message::AssistantResponse {
                    content: vec![Content::Text {
                        text: "ok".to_string(),
                    }],
                    reasoning_content: None,
                },
            })
        }

        async fn interact(
            &self,
            _request: InteractionRequest,
        ) -> crate::BabataResult<InteractionResponse> {
            Ok(InteractionResponse {})
        }
    }

    fn build_task(provider: Arc<dyn Provider>) -> AgentTask {
        AgentTask::new(
            vec![Message::UserPrompt {
                content: vec![Content::Text {
                    text: "hello".to_string(),
                }],
            }],
            Vec::new(),
            provider,
            "test-model".to_string(),
            HashMap::new(),
            Vec::new(),
            Vec::new(),
        )
    }

    #[tokio::test]
    async fn run_retries_provider_errors() {
        let calls = Arc::new(AtomicUsize::new(0));
        let provider = Arc::new(RetryTestProvider::new(
            calls.clone(),
            FailureMode::Provider { times: 2 },
        ));

        let task = build_task(provider);
        let result = task.run().await;

        assert!(result.is_ok());
        assert!(calls.load(Ordering::SeqCst) >= 3);
    }

    #[tokio::test]
    async fn run_does_not_retry_non_provider_errors() {
        let calls = Arc::new(AtomicUsize::new(0));
        let provider = Arc::new(RetryTestProvider::new(
            calls.clone(),
            FailureMode::Internal { times: 1 },
        ));

        let task = build_task(provider);
        let result = task.run().await;

        assert!(matches!(result, Err(BabataError::Internal(_, _))));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
