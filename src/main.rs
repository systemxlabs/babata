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
            babata::cli::ServerAction::Restart => babata::cli::server::restart(&args),
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
        Some(babata::cli::Command::Onboard) => babata::cli::onboard::run(&args),
        None => babata::cli::prompt::run(&args),
    }
}
