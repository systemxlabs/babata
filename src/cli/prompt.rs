use crate::{
    BabataResult,
    error::BabataError,
    message::{Content, Message},
};

use super::Args;

pub fn run(args: &Args) {
    if let Err(err) = run_prompt(args) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run_prompt(args: &Args) -> BabataResult<()> {
    unimplemented!()
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
