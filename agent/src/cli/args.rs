use clap::Parser;

#[derive(Parser, Debug)]
#[command(version)]
pub struct Args {
    /// Name of the person to greet
    #[arg(long, default_value = None )]
    session_id: Option<String>,
}
