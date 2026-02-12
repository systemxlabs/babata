use crate::{BabataResult, agent::AgentLoop, config::Config, error::BabataError};

use super::Args;

pub fn serve(args: &Args) {
    if let Err(err) = run_serve(args) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn start(_args: &Args) {
    // TODO: implement server start flow.
}

pub fn restart(_args: &Args) {
    // TODO: implement server restart flow.
}

fn run_serve(_args: &Args) -> BabataResult<()> {
    let config = Config::load()?;
    let agent_loop = AgentLoop::new(config)?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| {
            BabataError::internal(format!("Failed to initialize async runtime: {err}"))
        })?;

    runtime.block_on(agent_loop.run())?;
    Ok(())
}
