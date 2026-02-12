use clap::Parser;

fn main() {
    if let Err(err) = babata::logging::init() {
        eprintln!("{err}");
        std::process::exit(1);
    }

    let args = babata::cli::Args::parse();
    babata::cli::handle(&args);
}
