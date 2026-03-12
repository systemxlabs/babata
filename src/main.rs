use clap::Parser;

fn main() {
    if let Err(err) = babata::logging::init() {
        eprintln!("{err}");
        std::process::exit(1);
    }

    match babata::cli::Command::parse() {
        babata::cli::Command::Server { action } => match action {
            babata::cli::ServerAction::Serve => babata::cli::server::serve(),
            babata::cli::ServerAction::Start => babata::cli::server::start(),
            babata::cli::ServerAction::Stop => babata::cli::server::stop(),
            babata::cli::ServerAction::Restart => babata::cli::server::restart(),
            babata::cli::ServerAction::WindowsServiceHost { home_dir } => {
                babata::cli::server::windows_service_host(&home_dir)
            }
        },
        babata::cli::Command::Channel { action } => match action {
            babata::cli::ChannelAction::Add {
                channel_config_json,
            } => babata::cli::channel::add(&channel_config_json),
            babata::cli::ChannelAction::Delete { name } => babata::cli::channel::delete(&name),
            babata::cli::ChannelAction::List => babata::cli::channel::list(),
        },
        babata::cli::Command::Agent { action } => match action {
            babata::cli::AgentAction::Add { agent_config_json } => {
                babata::cli::agent::add(&agent_config_json)
            }
            babata::cli::AgentAction::Delete { name } => babata::cli::agent::delete(&name),
            babata::cli::AgentAction::List => babata::cli::agent::list(),
        },
        babata::cli::Command::Provider { action } => match action {
            babata::cli::ProviderAction::Add {
                provider_config_json,
            } => babata::cli::provider::add(&provider_config_json),
            babata::cli::ProviderAction::Delete { name } => {
                babata::cli::provider::delete(&name)
            }
            babata::cli::ProviderAction::List => babata::cli::provider::list(),
        },
        babata::cli::Command::Onboard => babata::cli::onboard::run(),
    }
}
