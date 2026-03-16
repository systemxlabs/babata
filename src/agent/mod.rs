pub mod babata;

use std::{collections::HashMap, fmt::Debug, sync::Arc};

use crate::{
    BabataResult,
    agent::babata::{BabataAgent, ToolContext},
    channel::Channel,
    config::{AgentConfig, Config},
    message::Content,
};

#[async_trait::async_trait]
pub trait Agent: Debug + Send + Sync {
    fn name() -> &'static str
    where
        Self: Sized;
    async fn execute(&self, prompt: Vec<Content>, tool_context: ToolContext) -> BabataResult<()>;
}

pub fn build_agents(
    config: &Config,
    channels: HashMap<String, Arc<dyn Channel>>,
) -> BabataResult<HashMap<String, Arc<dyn Agent>>> {
    let mut agents: HashMap<String, Arc<dyn Agent>> = HashMap::new();

    for agent_config in &config.agents {
        match agent_config {
            AgentConfig::Babata(_) => {
                let agent_name = BabataAgent::name().to_string();
                let agent: Arc<dyn Agent> = Arc::new(BabataAgent::new(config, channels.clone())?);
                agents.insert(agent_name, agent);
            }
        }
    }

    Ok(agents)
}
