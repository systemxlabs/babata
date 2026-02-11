use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version)]
pub struct Args {
    #[arg(long, default_value = "main")]
    pub agent: String,
    #[arg(long)]
    pub provider: Option<String>,
    #[arg(long)]
    pub model: Option<String>,
    #[arg()]
    pub prompt: Option<String>,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Server {
        #[command(subcommand)]
        action: ServerAction,
    },
    Provider {
        #[command(subcommand)]
        action: ProviderAction,
    },
    Onboard,
}

#[derive(Subcommand, Debug)]
pub enum ServerAction {
    Start,
    Restart,
}

#[derive(Subcommand, Debug)]
pub enum ProviderAction {
    Add {
        #[arg(long)]
        name: String,
        #[arg(long)]
        api_key: String,
    },
    Delete {
        #[arg(long)]
        name: String,
    },
    List,
}
