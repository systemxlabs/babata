use clap::Parser;

#[derive(Parser, Debug)]
#[command(version)]
pub struct Args {
    #[arg(long, default_value = "main")]
    agent: String,

    #[arg(long, default_value = None )]
    session_id: Option<String>,
}
