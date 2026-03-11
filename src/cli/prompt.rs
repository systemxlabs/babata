use std::sync::Arc;

use log::info;

use crate::{
    BabataResult,
    config::Config,
    error::BabataError,
    message::{Content, Message},
    runtime::TaskRuntime,
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
    let prompt = args
        .prompt
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| BabataError::config("Prompt is required"))?
        .to_string();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|err| {
            BabataError::internal(format!("Failed to initialize async runtime: {err}"))
        })?;

    runtime.block_on(async {
        let runtime = Arc::new(TaskRuntime::new(config)?);
        runtime.resume_running_tasks().await?;

        let user_message = Message::UserPrompt {
            content: vec![Content::Text { text: prompt }],
        };
        let task_id = runtime
            .submit_prompt_task(&args.agent, user_message)
            .await?;
        let message = runtime.wait_for_task(&task_id).await?;
        info!("Task '{}' finished with message: {:?}", task_id, message);
        print_final_message(&message)
    })
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
            "TaskRuntime returned non-final message type",
        )),
    }
}
