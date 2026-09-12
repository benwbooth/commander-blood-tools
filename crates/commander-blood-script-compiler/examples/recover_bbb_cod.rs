//! Recover all 17 COD sources; verify byte identity before writing each result.

use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use commander_blood_script_compiler::{
    compile_program, decompile_structured_big_bug_bang_cod_with_symbols, parse_source_dictionary,
    parse_source_directory,
};

fn main() -> Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 2 {
        bail!("usage: recover_bbb_cod RESOURCE_DIRECTORY OUTPUT_DIRECTORY");
    }
    let input = PathBuf::from(&args[0]);
    let output = PathBuf::from(&args[1]);
    fs::create_dir_all(&output)?;
    for profile in 1..=17 {
        let name = format!("SCRIPT{profile}");
        let cod = fs::read(input.join(format!("{name}.COD"))).with_context(|| name.clone())?;
        let dictionary = parse_source_dictionary(&fs::read(input.join(format!("{name}.DIC")))?);
        let symbols = parse_source_directory(&fs::read(input.join(format!("{name}.DEB")))?);
        let var = fs::read(input.join(format!("{name}.VAR")))?;
        let recovered =
            decompile_structured_big_bug_bang_cod_with_symbols(&cod, &var, &dictionary, &symbols)?;
        if recovered.raw_bytes != 0 || recovered.generic_op_statements != 0 {
            bail!("{name} contains unresolved bytes or operations");
        }
        if compile_program(&recovered.source, &dictionary)? != cod {
            bail!("{name} source does not reproduce its original COD");
        }
        fs::write(output.join(format!("{name}.blood")), recovered.source)?;
        println!(
            "{name}: {} statements, {} bytes verified",
            recovered.typed_statements,
            cod.len()
        );
    }
    Ok(())
}
