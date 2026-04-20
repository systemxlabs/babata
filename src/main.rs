use std::sync::Arc;

use babata::{
    BabataResult,
    agent::load_default_agent,
    channel::{ChannelConfig, build_channels, start_channel_loops},
    http::HttpApp,
    message::Content,
    task::{CreateTaskRequest, TaskLauncher, TaskManager, TaskStore},
    utils::{babata_dir, build_commit},
};
use log::info;

#[tokio::main]
async fn main() -> BabataResult<()> {
    babata::logging::init()?;

    info!("Server run at babata dir: {}", babata_dir()?.display());

    let channel_configs = ChannelConfig::load_all()?;
    let channels = build_channels(&channel_configs)?;
    let task_store = TaskStore::new()?;
    let task_launcher = TaskLauncher::new(channels.clone())?;
    let task_manager = Arc::new(TaskManager::new(task_store, task_launcher)?);

    let http_app = HttpApp::new(task_manager.clone());

    task_manager.start()?;
    start_channel_loops(channels, task_manager.clone());

    if !channel_configs.is_empty() {
        broadcast_service_started(&task_manager).await?;
    }

    http_app.serve().await?;

    Ok(())
}

async fn broadcast_service_started(task_manager: &Arc<TaskManager>) -> BabataResult<()> {
    let notification = format!(
        "Babata server started.\nVersion: {}\nBuild commit: {}\nBabata home: {}",
        env!("CARGO_PKG_VERSION"),
        build_commit().unwrap_or("unknown"),
        babata_dir()?.display(),
    );

    let prompt = Content::Text {
        text: format!("Send below notification to each channel: \n{notification}"),
    };

    if let Ok(default_agent) = load_default_agent() {
        let task = CreateTaskRequest {
            description: "broadcast service started notification".to_string(),
            prompt: vec![prompt],
            parent_task_id: None,
            agent: default_agent.frontmatter.name.clone(),
            never_ends: false,
        };
        task_manager.create_task(task)?;
    }
    Ok(())
}
