use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use commander_blood_tools::bas_cfg::{self, BasControlFlow};
use commander_blood_tools::bloodscript;
use commander_blood_tools::vm_bundle;
use commander_blood_tools::vm_cfg::{self, CodControlFlow};
use commander_blood_tools::vm_data::{self, DataKind};
use commander_blood_tools::vm_source::{self, ImageKind};

fn usage() -> ! {
    eprintln!(
        "usage:\n  cbvm disassemble <cod|bas> <image> <dictionary> <output>\n  cbvm assemble <source> <output>\n  cbvm decompile-bundle <game-dir> <output-dir>\n  cbvm decompile-bloodscript <game-dir> <output-dir>\n  cbvm decompile-structured <game-dir> <output-dir>\n  cbvm decompile-data-bundle <game-dir> <output-dir>\n  cbvm compile-bloodscript <source> <output> [dictionary]\n  cbvm compile-data <deb|dic|var> <source> <output>\n  cbvm compile-bundle <source-dir> <game-dir> <output-dir>\n  cbvm build-runtime-tree <source-dir> <game-dir> <output-dir>\n  cbvm analyze-control-flow <game-dir> <output-dir>\n  cbvm analyze-bas-control-flow <game-dir> <output-dir>"
    );
    std::process::exit(2);
}

fn read_dictionary(path: &Path) -> Result<std::collections::HashMap<u16, String>> {
    let bytes =
        std::fs::read(path).with_context(|| format!("reading dictionary {}", path.display()))?;
    Ok(commander_blood_tools::script::parse_dictionary(&bytes))
}

fn guard_rejection_summary(counts: &std::collections::BTreeMap<String, usize>) -> String {
    if counts.is_empty() {
        return "-".to_string();
    }
    counts
        .iter()
        .map(|(reason, count)| format!("{reason}={count}"))
        .collect::<Vec<_>>()
        .join(",")
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
    var_path: Option<&Path>,
    output_path: &Path,
    structured: bool,
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
    let source = if structured {
        let var_path = var_path
            .ok_or_else(|| anyhow::anyhow!("structured decompilation requires a VAR image"))?;
        let var = std::fs::read(var_path)
            .with_context(|| format!("reading VM object data {}", var_path.display()))?;
        match kind {
            ImageKind::Cod => bloodscript::decompile_structured_cod_with_symbols(
                &image,
                &var,
                &dictionary,
                &symbols,
            )?,
            ImageKind::Bas => bloodscript::decompile_structured_bas_with_symbols(
                &image,
                &var,
                &dictionary,
                &symbols,
            )?,
        }
    } else {
        bloodscript::decompile_with_symbols(kind, &image, &dictionary, &symbols)?
    };
    let rebuilt = bloodscript::compile_with_dictionary(&source.source, &dictionary)?;
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

fn write_vm_data(
    kind: DataKind,
    input_path: &Path,
    output_path: &Path,
) -> Result<vm_data::Decompilation> {
    let input = std::fs::read(input_path)
        .with_context(|| format!("reading VM data {}", input_path.display()))?;
    let source = vm_data::decompile(kind, &input)
        .with_context(|| format!("decompiling {}", input_path.display()))?;
    let rebuilt = vm_data::compile(kind, &source.source)?;
    if rebuilt != input {
        bail!(
            "internal BloodData round-trip failure for {}",
            input_path.display()
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

fn write_control_flow(
    script: &str,
    image_path: &Path,
    symbol_path: &Path,
    output_path: &Path,
) -> Result<CodControlFlow> {
    let image = std::fs::read(image_path)
        .with_context(|| format!("reading VM image {}", image_path.display()))?;
    let symbols = std::fs::read(symbol_path)
        .with_context(|| format!("reading symbol table {}", symbol_path.display()))?;
    let graph = vm_cfg::analyze_cod(
        script,
        &image,
        &commander_blood_tools::script::parse_deb(&symbols),
    )?;
    if let Some(parent) = output_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut json = serde_json::to_vec_pretty(&graph)?;
    json.push(b'\n');
    std::fs::write(output_path, json)
        .with_context(|| format!("writing {}", output_path.display()))?;
    Ok(graph)
}

fn write_bas_control_flow(
    script: &str,
    image_path: &Path,
    var_path: &Path,
    dictionary_path: &Path,
    symbol_path: &Path,
    output_path: &Path,
) -> Result<BasControlFlow> {
    let image = std::fs::read(image_path)
        .with_context(|| format!("reading VM image {}", image_path.display()))?;
    let var = std::fs::read(var_path)
        .with_context(|| format!("reading VM object data {}", var_path.display()))?;
    let dictionary = read_dictionary(dictionary_path)?;
    let symbols = std::fs::read(symbol_path)
        .with_context(|| format!("reading symbol table {}", symbol_path.display()))?;
    let graph = bas_cfg::analyze_bas(
        script,
        &image,
        &var,
        &dictionary,
        &commander_blood_tools::script::parse_deb(&symbols),
    )?;
    if let Some(parent) = output_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut json = serde_json::to_vec_pretty(&graph)?;
    json.push(b'\n');
    std::fs::write(output_path, json)
        .with_context(|| format!("writing {}", output_path.display()))?;
    Ok(graph)
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
            let dictionary = args
                .next()
                .map(PathBuf::from)
                .map(|path| read_dictionary(&path))
                .transpose()?
                .unwrap_or_default();
            if args.next().is_some() {
                usage();
            }
            let text = std::fs::read_to_string(&source)
                .with_context(|| format!("reading {}", source.display()))?;
            let image = bloodscript::compile_with_dictionary(&text, &dictionary)?;
            std::fs::write(&output, &image)
                .with_context(|| format!("writing {}", output.display()))?;
            println!("wrote {}: {} byte(s)", output.display(), image.len());
        }
        Some("compile-data") => {
            let kind = DataKind::parse(&args.next().unwrap_or_else(|| usage()))?;
            let source = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let output = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            if args.next().is_some() {
                usage();
            }
            let text = std::fs::read_to_string(&source)
                .with_context(|| format!("reading {}", source.display()))?;
            let image = vm_data::compile(kind, &text)?;
            std::fs::write(&output, &image)
                .with_context(|| format!("writing {}", output.display()))?;
            println!("wrote {}: {} byte(s)", output.display(), image.len());
        }
        Some("compile-bundle") => {
            let source_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let game_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let output_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            if args.next().is_some() {
                usage();
            }
            let entries = vm_bundle::compile_bundle(&source_dir, &game_dir, &output_dir)?;
            std::fs::write(
                output_dir.join("cbvm-bundle-manifest.tsv"),
                vm_bundle::manifest(&entries),
            )?;
            println!(
                "wrote {}: {} byte-exact VM resource(s)",
                output_dir.display(),
                entries.len()
            );
        }
        Some("build-runtime-tree") => {
            let source_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let game_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let output_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            if args.next().is_some() {
                usage();
            }
            let (entries, stats) =
                vm_bundle::build_runtime_tree(&source_dir, &game_dir, &output_dir)?;
            std::fs::write(
                output_dir.join("cbvm-bundle-manifest.tsv"),
                vm_bundle::manifest(&entries),
            )?;
            println!(
                "wrote {}: {} byte-exact VM resource(s), {} hardlinked asset(s), {} copied asset(s)",
                output_dir.display(),
                entries.len(),
                stats.hardlinked_files,
                stats.copied_files
            );
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
                        None,
                        &output,
                        false,
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
        Some("decompile-structured") => {
            let game_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let output_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            if args.next().is_some() {
                usage();
            }
            std::fs::create_dir_all(&output_dir)?;
            let mut manifest = String::from(
                "script\timage\tinput_bytes\ttyped_statements\ttyped_bytes\tgeneric_op_statements\tgeneric_op_bytes\traw_bytes\tsymbolic_labels\tprocedures\tobject_aliases\tobject_alias_uses\tfield_aliases\tfield_alias_uses\tdictionary_offsets\tdictionary_uses\tstructured_guards\tunstructured_guards\tguard_rejections\tselector_lists\tcases\troundtrip\n",
            );
            for script in 1..=5 {
                let dictionary = game_dir.join(format!("SCRIPT{script}.DIC"));
                let symbols = game_dir.join(format!("SCRIPT{script}.DEB"));
                let var = game_dir.join(format!("SCRIPT{script}.VAR"));
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
                        Some(symbols.as_path()),
                        Some(var.as_path()),
                        &output,
                        true,
                    )?;
                    let input_bytes = std::fs::metadata(&image)?.len();
                    manifest.push_str(&format!(
                        "SCRIPT{script}\t{extension}\t{input_bytes}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\tbyte_exact\n",
                        source.typed_statements,
                        source.typed_bytes,
                        source.generic_op_statements,
                        source.generic_op_bytes,
                        source.raw_bytes,
                        source.symbolic_labels,
                        source.procedures,
                        source.object_aliases,
                        source.object_alias_uses,
                        source.field_aliases,
                        source.field_alias_uses,
                        source.dictionary_offsets,
                        source.dictionary_uses,
                        source.structured_guards,
                        source.unstructured_guards,
                        guard_rejection_summary(&source.guard_rejection_counts),
                        source.structured_selector_lists,
                        source.structured_cases
                    ));
                    println!("verified {} -> {}", image.display(), output.display());
                }
            }
            std::fs::write(output_dir.join("manifest.tsv"), manifest)?;
        }
        Some("decompile-data-bundle") => {
            let game_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let output_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            if args.next().is_some() {
                usage();
            }
            std::fs::create_dir_all(&output_dir)?;
            let mut manifest = String::from("script\timage\tinput_bytes\tstatements\troundtrip\n");
            for script in 1..=5 {
                for kind in [DataKind::Deb, DataKind::Dic, DataKind::Var] {
                    let extension = kind.extension();
                    let input = game_dir.join(format!("SCRIPT{script}.{extension}"));
                    let output = output_dir.join(format!(
                        "script{script}.{}.blooddata",
                        extension.to_ascii_lowercase()
                    ));
                    let source = write_vm_data(kind, &input, &output)?;
                    manifest.push_str(&format!(
                        "SCRIPT{script}\t{extension}\t{}\t{}\tbyte_exact\n",
                        source.bytes, source.statements
                    ));
                    println!("verified {} -> {}", input.display(), output.display());
                }
            }
            std::fs::write(output_dir.join("data-manifest.tsv"), manifest)?;
        }
        Some("analyze-control-flow") => {
            let game_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let output_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            if args.next().is_some() {
                usage();
            }
            std::fs::create_dir_all(&output_dir)?;
            let mut manifest = String::from(
                "script\tinput_bytes\tinstructions\tprocedures\tblocks\treachable_blocks\tedges\tpoke_instructions\tpatched_block_flags\tmutable_block_flags\tunresolved_guard_branches\n",
            );
            for script in 1..=5 {
                let name = format!("SCRIPT{script}");
                let image = game_dir.join(format!("{name}.COD"));
                let symbols = game_dir.join(format!("{name}.DEB"));
                let output = output_dir.join(format!("script{script}.cod.cfg.json"));
                let graph = write_control_flow(&name, &image, &symbols, &output)?;
                manifest.push_str(&format!(
                    "{name}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                    graph.image_bytes,
                    graph.instruction_count,
                    graph.procedure_count,
                    graph.block_count,
                    graph.reachable_block_count,
                    graph.edge_count,
                    graph.poke_instruction_count,
                    graph.patched_block_flag_count,
                    graph.mutable_block_flag_count,
                    graph.unresolved_guard_branches.len()
                ));
                println!("analyzed {} -> {}", image.display(), output.display());
            }
            std::fs::write(output_dir.join("manifest.tsv"), manifest)?;
        }
        Some("analyze-bas-control-flow") => {
            let game_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let output_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            if args.next().is_some() {
                usage();
            }
            std::fs::create_dir_all(&output_dir)?;
            let mut manifest = String::from(
                "script\tinput_bytes\ttokens\tselector_nodes\tlists\tentrypoints\tdirect_next\tedges\n",
            );
            for script in 1..=5 {
                let name = format!("SCRIPT{script}");
                let image = game_dir.join(format!("{name}.BAS"));
                let var = game_dir.join(format!("{name}.VAR"));
                let dictionary = game_dir.join(format!("{name}.DIC"));
                let symbols = game_dir.join(format!("{name}.DEB"));
                let output = output_dir.join(format!("script{script}.bas.cfg.json"));
                let graph =
                    write_bas_control_flow(&name, &image, &var, &dictionary, &symbols, &output)?;
                manifest.push_str(&format!(
                    "{name}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                    graph.image_bytes,
                    graph.token_count,
                    graph.selector_node_count,
                    graph.list_count,
                    graph.entrypoint_count,
                    graph.direct_next_count,
                    graph.edge_count,
                ));
                println!("analyzed {} -> {}", image.display(), output.display());
            }
            std::fs::write(output_dir.join("manifest.tsv"), manifest)?;
        }
        _ => usage(),
    }
    Ok(())
}
