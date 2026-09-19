use std::process::ExitCode;

use commander_blood_game::{app, game::GameVariant};

fn main() -> ExitCode {
    if let Err(error) = app::run_for_game(GameVariant::BigBugBang) {
        eprintln!("error: {error:#}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
