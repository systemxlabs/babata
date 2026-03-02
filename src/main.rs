use clap::Parser;

fn main() {
    if let Err(err) = babata::logging::init() {
        eprintln!("{err}");
        std::process::exit(1);
    }

    let args = babata::cli::Args::parse();
    match &args.command {
        Some(babata::cli::Command::Server { action }) => match action {
            babata::cli::ServerAction::Serve => babata::cli::server::serve(&args),
            babata::cli::ServerAction::Start => babata::cli::server::start(&args),
            babata::cli::ServerAction::Stop => babata::cli::server::stop(&args),
            babata::cli::ServerAction::Restart => babata::cli::server::restart(&args),
            babata::cli::ServerAction::WindowsServiceHost { home_dir } => {
                babata::cli::server::windows_service_host(&args, home_dir)
            }
        },
        Some(babata::cli::Command::Channel { action }) => match action {
            babata::cli::ChannelAction::Add {
                channel_config_json,
            } => babata::cli::channel::add(&args, channel_config_json),
            babata::cli::ChannelAction::Delete { name } => {
                babata::cli::channel::delete(&args, name)
            }
            babata::cli::ChannelAction::List => babata::cli::channel::list(&args),
        },
        Some(babata::cli::Command::Agent { action }) => match action {
            babata::cli::AgentAction::Add { agent_config_json } => {
                babata::cli::agent::add(&args, agent_config_json)
            }
            babata::cli::AgentAction::Delete { name } => babata::cli::agent::delete(&args, name),
            babata::cli::AgentAction::List => babata::cli::agent::list(&args),
        },
        Some(babata::cli::Command::Provider { action }) => match action {
            babata::cli::ProviderAction::Add {
                provider_config_json,
            } => babata::cli::provider::add(&args, provider_config_json),
            babata::cli::ProviderAction::Delete { name } => {
                babata::cli::provider::delete(&args, name)
            }
            babata::cli::ProviderAction::List => babata::cli::provider::list(&args),
        },
        Some(babata::cli::Command::Job { action }) => match action {
            babata::cli::JobAction::Add { job_config_json } => {
                babata::cli::job::add(&args, job_config_json)
            }
            babata::cli::JobAction::Delete { name } => babata::cli::job::delete(&args, name),
            babata::cli::JobAction::List => babata::cli::job::list(&args),
            babata::cli::JobAction::History { name, limit } => {
                babata::cli::job::history(&args, name.as_deref(), *limit)
            }
        },
        Some(babata::cli::Command::Onboard) => babata::cli::onboard::run(&args),
        None => babata::cli::prompt::run(&args),
    }
}
