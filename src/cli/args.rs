use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version)]
pub struct Args {
    #[arg(long, default_value = "main")]
    pub agent: String,
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
    Serve,
    Start,
    Restart,
}

#[derive(Subcommand, Debug)]
pub enum ProviderAction {
    Add {
        #[arg(value_name = "PROVIDER_CONFIG_JSON")]
        provider_config_json: String,
    },
    Delete {
        #[arg(value_name = "PROVIDER_NAME")]
        name: String,
    },
    List,
}
