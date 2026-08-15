use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use commander_blood_tools::bloodscript;
use commander_blood_tools::vm_source::{self, ImageKind};

fn usage() -> ! {
    eprintln!(
        "usage:\n  cbvm disassemble <cod|bas> <image> <dictionary> <output>\n  cbvm assemble <source> <output>\n  cbvm decompile-bundle <game-dir> <output-dir>\n  cbvm decompile-bloodscript <game-dir> <output-dir>\n  cbvm compile-bloodscript <source> <output>"
    );
    std::process::exit(2);
}

fn read_dictionary(path: &Path) -> Result<std::collections::HashMap<u16, String>> {
    let bytes =
        std::fs::read(path).with_context(|| format!("reading dictionary {}", path.display()))?;
    Ok(commander_blood_tools::script::parse_dictionary(&bytes))
}

fn write_disassembly(
    kind: ImageKind,
    image_path: &Path,
    dictionary_path: &Path,
    output_path: &Path,
) -> Result<vm_source::Disassembly> {
    let image = std::fs::read(image_path)
        .with_context(|| format!("reading VM image {}", image_path.display()))?;
    let dictionary = read_dictionary(dictionary_path)?;
    let listing = vm_source::disassemble(kind, &image, &dictionary)?;
    let rebuilt = vm_source::assemble(&listing.source)?;
    if rebuilt != image {
        bail!("internal round-trip failure for {}", image_path.display());
    }
    if let Some(parent) = output_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output_path, &listing.source)
        .with_context(|| format!("writing {}", output_path.display()))?;
    Ok(listing)
}

fn write_bloodscript(
    kind: ImageKind,
    image_path: &Path,
    dictionary_path: &Path,
    symbol_path: Option<&Path>,
    output_path: &Path,
) -> Result<bloodscript::Decompilation> {
    let image = std::fs::read(image_path)
        .with_context(|| format!("reading VM image {}", image_path.display()))?;
    let dictionary = read_dictionary(dictionary_path)?;
    let symbols = symbol_path
        .map(|path| {
            std::fs::read(path)
                .with_context(|| format!("reading symbol table {}", path.display()))
                .map(|bytes| commander_blood_tools::script::parse_deb(&bytes))
        })
        .transpose()?
        .unwrap_or_default();
    let source = bloodscript::decompile_with_symbols(kind, &image, &dictionary, &symbols)?;
    let rebuilt = bloodscript::compile(&source.source)?;
    if rebuilt != image {
        bail!(
            "internal BloodScript round-trip failure for {}",
            image_path.display()
        );
    }
    if let Some(parent) = output_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output_path, &source.source)
        .with_context(|| format!("writing {}", output_path.display()))?;
    Ok(source)
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("disassemble") => {
            let kind = ImageKind::parse(&args.next().unwrap_or_else(|| usage()))?;
            let image = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let dictionary = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let output = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            if args.next().is_some() {
                usage();
            }
            let listing = write_disassembly(kind, &image, &dictionary, &output)?;
            println!(
                "wrote {}: {} semantic span(s), {} semantic byte(s), {} raw byte(s), byte-exact round trip",
                output.display(),
                listing.semantic_spans,
                listing.semantic_bytes,
                listing.raw_bytes
            );
        }
        Some("assemble") => {
            let source = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let output = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            if args.next().is_some() {
                usage();
            }
            let text = std::fs::read_to_string(&source)
                .with_context(|| format!("reading {}", source.display()))?;
            let image = vm_source::assemble(&text)?;
            std::fs::write(&output, &image)
                .with_context(|| format!("writing {}", output.display()))?;
            println!("wrote {}: {} byte(s)", output.display(), image.len());
        }
        Some("compile-bloodscript") => {
            let source = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let output = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            if args.next().is_some() {
                usage();
            }
            let text = std::fs::read_to_string(&source)
                .with_context(|| format!("reading {}", source.display()))?;
            let image = bloodscript::compile(&text)?;
            std::fs::write(&output, &image)
                .with_context(|| format!("writing {}", output.display()))?;
            println!("wrote {}: {} byte(s)", output.display(), image.len());
        }
        Some("decompile-bundle") => {
            let game_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let output_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            if args.next().is_some() {
                usage();
            }
            std::fs::create_dir_all(&output_dir)?;
            let mut manifest = String::from(
                "script\timage\tinput_bytes\tsemantic_spans\tsemantic_bytes\traw_bytes\troundtrip\n",
            );
            for script in 1..=5 {
                let dictionary = game_dir.join(format!("SCRIPT{script}.DIC"));
                for (extension, kind) in [("COD", ImageKind::Cod), ("BAS", ImageKind::Bas)] {
                    let image = game_dir.join(format!("SCRIPT{script}.{extension}"));
                    let output = output_dir.join(format!(
                        "script{script}.{}.cbvm",
                        extension.to_ascii_lowercase()
                    ));
                    let listing = write_disassembly(kind, &image, &dictionary, &output)?;
                    let input_bytes = std::fs::metadata(&image)?.len();
                    manifest.push_str(&format!(
                        "SCRIPT{script}\t{extension}\t{input_bytes}\t{}\t{}\t{}\tbyte_exact\n",
                        listing.semantic_spans, listing.semantic_bytes, listing.raw_bytes
                    ));
                    println!("verified {} -> {}", image.display(), output.display());
                }
            }
            std::fs::write(output_dir.join("manifest.tsv"), manifest)?;
        }
        Some("decompile-bloodscript") => {
            let game_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let output_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            if args.next().is_some() {
                usage();
            }
            std::fs::create_dir_all(&output_dir)?;
            let mut manifest = String::from(
                "script\timage\tinput_bytes\ttyped_statements\ttyped_bytes\tgeneric_op_statements\tgeneric_op_bytes\traw_bytes\tsymbolic_labels\tprocedures\troundtrip\n",
            );
            for script in 1..=5 {
                let dictionary = game_dir.join(format!("SCRIPT{script}.DIC"));
                let symbols = game_dir.join(format!("SCRIPT{script}.DEB"));
                for (extension, kind) in [("COD", ImageKind::Cod), ("BAS", ImageKind::Bas)] {
                    let image = game_dir.join(format!("SCRIPT{script}.{extension}"));
                    let output = output_dir.join(format!(
                        "script{script}.{}.blood",
                        extension.to_ascii_lowercase()
                    ));
                    let source = write_bloodscript(
                        kind,
                        &image,
                        &dictionary,
                        (kind == ImageKind::Cod).then_some(symbols.as_path()),
                        &output,
                    )?;
                    let input_bytes = std::fs::metadata(&image)?.len();
                    manifest.push_str(&format!(
                        "SCRIPT{script}\t{extension}\t{input_bytes}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\tbyte_exact\n",
                        source.typed_statements,
                        source.typed_bytes,
                        source.generic_op_statements,
                        source.generic_op_bytes,
                        source.raw_bytes,
                        source.symbolic_labels,
                        source.procedures
                    ));
                    println!("verified {} -> {}", image.display(), output.display());
                }
            }
            std::fs::write(output_dir.join("manifest.tsv"), manifest)?;
        }
        _ => usage(),
    }
    Ok(())
}
