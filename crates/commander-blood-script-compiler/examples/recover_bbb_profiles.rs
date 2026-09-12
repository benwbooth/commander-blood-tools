//! Recover all 17 unified BBB profiles and verify all active companions.

use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use commander_blood_script_compiler::{
    ProfileDialect, ProfileImages, decompile_profile, require_same_profile,
};

fn main() -> Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 2 {
        bail!("usage: recover_bbb_profiles RESOURCE_DIRECTORY OUTPUT_DIRECTORY");
    }
    let input = PathBuf::from(&args[0]);
    let output = PathBuf::from(&args[1]);
    fs::create_dir_all(&output)?;

    let mut verified_bytes = 0usize;
    for profile in 1..=17 {
        let name = format!("SCRIPT{profile}");
        let read = |extension: &str| {
            let path = input.join(format!("{name}.{extension}"));
            fs::read(&path).with_context(|| format!("reading {}", path.display()))
        };
        let shipped = ProfileImages {
            dialect: ProfileDialect::BigBugBang,
            name: name.clone(),
            cod: read("COD")?,
            bas: None,
            deb: read("DEB")?,
            dic: read("DIC")?,
            var: read("VAR")?,
        };
        let source = decompile_profile(&shipped).with_context(|| name.clone())?;
        let rebuilt = commander_blood_script_compiler::compile_profile(&source)?;
        require_same_profile(&rebuilt, &shipped)?;
        verified_bytes += shipped
            .extensions()
            .iter()
            .map(|extension| shipped.image(extension).expect("owned extension").len())
            .sum::<usize>();
        let path = output.join(format!("script{profile}.blood"));
        fs::write(&path, source).with_context(|| format!("writing {}", path.display()))?;
        println!("{name}: four active companions verified");
    }
    println!("verified {verified_bytes} bytes across 68 active VM resources");
    Ok(())
}
