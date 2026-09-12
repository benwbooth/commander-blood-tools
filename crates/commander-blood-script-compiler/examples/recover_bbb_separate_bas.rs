//! Recover BBB's preserved standalone BAS artifact without inventing ownership.

use std::{collections::HashMap, env, fs, path::PathBuf};

use anyhow::{Context, Result, bail, ensure};
use commander_blood_script_compiler::{compile_program, decompile_unbound_bas};

fn main() -> Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 2 {
        bail!("usage: recover_bbb_separate_bas SCRIPT2.BAS OUTPUT_SOURCE");
    }
    let input = PathBuf::from(&args[0]);
    let output = PathBuf::from(&args[1]);
    let image = fs::read(&input).with_context(|| format!("reading {}", input.display()))?;
    let recovered = decompile_unbound_bas(&image)?;
    ensure!(recovered.raw_bytes == 0, "unrecovered BAS bytes remain");
    ensure!(
        recovered.generic_op_statements == 0,
        "generic BAS operations remain"
    );
    let rebuilt = compile_program(&recovered.source, &HashMap::new())?;
    ensure!(
        rebuilt == image,
        "recovered BAS source did not rebuild exactly"
    );
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, recovered.source)
        .with_context(|| format!("writing {}", output.display()))?;
    println!(
        "verified {} bytes as {} typed statements with no dictionary binding",
        image.len(),
        recovered.typed_statements
    );
    Ok(())
}
