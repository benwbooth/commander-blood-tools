//! Scan readable profiles into static dialogue graphs without running the game.

use anyhow::{Context, Result, bail};
use std::{env, fs, io, path::PathBuf};

fn main() -> Result<()> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() != 1 {
        bail!("usage: dialogue_catalog PROFILE.blood");
    }
    let path = PathBuf::from(&args[0]);
    let source =
        fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let result = commander_blood_script_compiler::analyze_dialogue(&source)
        .with_context(|| format!("analyzing {}", path.display()))?;
    serde_json::to_writer_pretty(io::stdout().lock(), &result)?;
    Ok(())
}
