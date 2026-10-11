//! `spatial-dpp` binary entry point.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(sdpp_cli::run(&args));
}
