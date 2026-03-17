use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Babata agent CLI",
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
    #[command(about = "Agent config management (add/delete/list)")]
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },
    #[command(about = "Provider config management (add/delete/list)")]
    Provider {
        #[command(subcommand)]
        action: ProviderAction,
    },
    #[command(about = "Task management commands")]
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },
    #[command(about = "Interactive setup (provider/agent/channel/service)")]
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
    #[command(
        name = "windows-service-host",
        hide = true,
        about = "Run Windows service host"
    )]
    WindowsServiceHost {
        #[arg(long, hide = true, value_name = "HOME_DIR")]
        home_dir: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ProviderAction {
    #[command(about = "Add or update provider config (JSON)")]
    Add {
        #[arg(
            value_name = "PROVIDER_CONFIG_JSON",
            help = "Provider config JSON, e.g. {\"name\":\"openai\",\"api_key\":\"sk-...\"}"
        )]
        provider_config_json: String,
    },
    #[command(about = "Delete a provider by name")]
    Delete {
        #[arg(
            value_name = "PROVIDER_NAME",
            help = "Provider name, e.g. openai, kimi, or moonshot"
        )]
        name: String,
    },
    #[command(about = "List all provider configs (one JSON per line)")]
    List,
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

#[derive(Subcommand, Debug)]
pub enum AgentAction {
    #[command(about = "Add or update agent config (JSON)")]
    Add {
        #[arg(
            value_name = "AGENT_CONFIG_JSON",
            help = "Agent config JSON, e.g. {\"name\":\"main\",\"provider\":\"openai\",\"model\":\"gpt-4.1\"}"
        )]
        agent_config_json: String,
    },
    #[command(about = "Delete an agent by name (main agent cannot be deleted)")]
    Delete {
        #[arg(value_name = "AGENT_NAME", help = "Agent name (must not be main)")]
        name: String,
    },
    #[command(about = "List all agent configs (one JSON per line)")]
    List,
}

#[derive(Subcommand, Debug)]
pub enum TaskAction {
    #[command(about = "Pause a task by id")]
    Pause {
        #[arg(value_name = "TASK_ID", help = "Task UUID")]
        task_id: String,
    },
    #[command(about = "Resume a task by id")]
    Resume {
        #[arg(value_name = "TASK_ID", help = "Task UUID")]
        task_id: String,
    },
    #[command(about = "Cancel a task by id")]
    Cancel {
        #[arg(value_name = "TASK_ID", help = "Task UUID")]
        task_id: String,
    },
    #[command(about = "Create a task")]
    Create {
        #[arg(long, value_name = "PROMPT", help = "Task prompt text")]
        prompt: String,
        #[arg(long, value_name = "AGENT", help = "Optional agent name")]
        agent: Option<String>,
        #[arg(
            long = "parent-task-id",
            value_name = "PARENT_TASK_ID",
            help = "Optional parent task UUID"
        )]
        parent_task_id: Option<String>,
    },
    #[command(about = "List tasks")]
    List {
        #[arg(long, value_name = "STATUS", help = "Optional task status filter")]
        status: Option<String>,
        #[arg(long, value_name = "LIMIT", help = "Optional max number of tasks")]
        limit: Option<usize>,
        #[arg(
            long,
            help = "Render task list as a pretty table instead of JSON lines"
        )]
        pretty_format: bool,
    },
    #[command(about = "Get a task by id")]
    Get {
        #[arg(value_name = "TASK_ID", help = "Task UUID")]
        task_id: String,
    },
}
