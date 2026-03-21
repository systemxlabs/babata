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
            babata::cli::ProviderAction::Delete { name } => babata::cli::provider::delete(&name),
            babata::cli::ProviderAction::List => babata::cli::provider::list(),
        },
        babata::cli::Command::Task { action } => match action {
            babata::cli::TaskAction::Pause { task_id } => babata::cli::task::pause(&task_id),
            babata::cli::TaskAction::Resume { task_id } => babata::cli::task::resume(&task_id),
            babata::cli::TaskAction::Cancel { task_id } => babata::cli::task::cancel(&task_id),
            babata::cli::TaskAction::Relaunch { task_id, reason } => {
                babata::cli::task::relaunch(&task_id, &reason)
            }
            babata::cli::TaskAction::Create {
                prompt,
                agent,
                parent_task_id,
                never_ends,
            } => babata::cli::task::create(
                &prompt,
                agent.as_deref(),
                parent_task_id.as_deref(),
                never_ends,
            ),
            babata::cli::TaskAction::List {
                status,
                limit,
                pretty_format,
            } => babata::cli::task::list(status.as_deref(), limit, pretty_format),
            babata::cli::TaskAction::Get { task_id } => babata::cli::task::get(&task_id),
        },
        babata::cli::Command::Onboard => babata::cli::onboard::run(),
    }
}
