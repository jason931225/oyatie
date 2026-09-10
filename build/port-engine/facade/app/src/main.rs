use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        args.push("help".to_owned());
    }
    port_engine_app::cli::run(&args)
}
