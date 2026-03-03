use log::info;

use crate::{
    BabataResult,
    config::Config,
    error::BabataError,
    message::{Content, Message},
    provider::create_provider,
    skill::load_skills,
    system_prompt::load_system_prompt_files,
    task::AgentTask,
    tool::build_tools,
};

use super::Args;

pub fn run(args: &Args) {
    if let Err(err) = run_prompt(args) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run_prompt(args: &Args) -> BabataResult<()> {
    let config = Config::load()?;

    let agent_config = config.get_agent(&args.agent).ok_or_else(|| {
        BabataError::config(format!(
            "Agent '{}' not found in config; run \"babata onboard\" first",
            args.agent
        ))
    })?;

    let Some(provider_config) = config
        .providers
        .iter()
        .find(|provider| provider.matches_name(&agent_config.provider))
    else {
        return Err(BabataError::config(format!(
            "Provider '{}' not found in config",
            agent_config.provider
        )));
    };

    let provider = create_provider(provider_config)?;

    let prompt = args
        .prompt
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| BabataError::config("Prompt is required"))?
        .to_string();

    let user_message = Message::UserPrompt {
        content: vec![Content::Text { text: prompt }],
    };
    info!("User message before task.run: {:?}", user_message);

    let task = AgentTask::new(
        vec![user_message],
        provider,
        agent_config.model.clone(),
        build_tools(),
        load_system_prompt_files()?,
        load_skills()?,
    );

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| {
            BabataError::internal(format!("Failed to initialize async runtime: {err}"))
        })?;

    let message = runtime.block_on(task.run())?;
    info!("Task run result message: {:?}", message);
    print_final_message(&message)?;

    Ok(())
}

fn print_final_message(message: &Message) -> BabataResult<()> {
    match message {
        Message::AssistantResponse { content, .. } => {
            for part in content {
                match part {
                    Content::Text { text } => println!("{text}"),
                    Content::ImageUrl { url } => println!("[image_url] {url}"),
                    Content::ImageData { media_type, .. } => {
                        println!("[image_data] {media_type}")
                    }
                    Content::AudioData { media_type, .. } => {
                        println!("[audio_data] {media_type}")
                    }
                }
            }
            Ok(())
        }
        _ => Err(BabataError::internal(
            "AgentTask returned non-final message type",
        )),
    }
}
