use clap::Parser;

fn main() {
    let args = babata::cli::Args::parse();
    babata::cli::handle(&args);
}
