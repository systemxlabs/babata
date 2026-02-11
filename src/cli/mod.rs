mod args;
pub mod onboard;
pub mod prompt;
pub mod provider;
pub mod server;

pub use args::*;

pub fn handle(args: &Args) {
    match &args.command {
        Some(Command::Server { action }) => match action {
            ServerAction::Start => server::start(args),
            ServerAction::Restart => server::restart(args),
        },
        Some(Command::Provider { action }) => match action {
            ProviderAction::Add { name, api_key } => provider::add(args, name, api_key),
            ProviderAction::Delete { name } => provider::delete(args, name),
            ProviderAction::List => provider::list(args),
        },
        Some(Command::Onboard) => onboard::run(args),
        None => prompt::run(args),
    }
}
