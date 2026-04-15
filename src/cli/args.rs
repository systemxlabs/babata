use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Babata CLI",
    long_about = None,
    arg_required_else_help = true
)]
pub enum Command {
    #[command(about = "Server management commands (serve/start/stop/restart)")]
    Server {
        #[command(subcommand)]
        action: ServerAction,
    },
    #[command(about = "Channel config management (add/delete/list)")]
    Channel {
        #[command(subcommand)]
        action: ChannelAction,
    },
    #[command(about = "Interactive setup (agent/channel/service)")]
    Onboard,
}

#[derive(Subcommand, Debug)]
pub enum ServerAction {
    #[command(about = "Run the server loop in foreground")]
    Serve,
    #[command(about = "Start background service on current platform")]
    Start,
    #[command(about = "Stop background service on current platform")]
    Stop,
    #[command(about = "Restart background service on current platform")]
    Restart,
}

#[derive(Subcommand, Debug)]
pub enum ChannelAction {
    #[command(about = "Add or update channel config (JSON)")]
    Add {
        #[arg(
            value_name = "CHANNEL_CONFIG_JSON",
            help = "Channel config JSON, e.g. {\"name\":\"telegram\",\"bot_token\":\"123:abc\",\"user_id\":123456789}"
        )]
        channel_config_json: String,
    },
    #[command(about = "Delete a channel by name")]
    Delete {
        #[arg(value_name = "CHANNEL_NAME", help = "Channel name, e.g. telegram")]
        name: String,
    },
    #[command(about = "List all channel configs (one JSON per line)")]
    List,
}
