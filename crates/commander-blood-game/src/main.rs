use std::process::ExitCode;

fn main() -> ExitCode {
    if let Err(error) = commander_blood_game::app::run() {
        eprintln!("error: {error:#}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
