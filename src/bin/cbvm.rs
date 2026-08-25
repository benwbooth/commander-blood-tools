use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use commander_blood_tools::bas_cfg::{self, BasControlFlow};
use commander_blood_tools::contact_manifest;
use commander_blood_tools::vm_bundle;
use commander_blood_tools::vm_cfg::{self, CodControlFlow};
use commander_blood_tools::vm_profile::{self, ProfileImages};
use commander_blood_tools::vm_source::{self, ImageKind};

const PROFILE_EXTENSIONS: [&str; 5] = ["COD", "BAS", "DEB", "DIC", "VAR"];

fn usage() -> ! {
    eprintln!(
        "usage:\n  cbvm disassemble <cod|bas> <image> <dictionary> <output>\n  cbvm assemble <source> <output>\n  cbvm decompile-bundle <game-dir> <output-dir>\n  cbvm decompile-unified <game-dir> <output-dir>\n  cbvm compile-profile <source> <output-dir>\n  cbvm compile-bundle <source-dir> <game-dir> <output-dir>\n  cbvm build-runtime-tree <source-dir> <game-dir> <output-dir>\n  cbvm analyze-control-flow <game-dir> <output-dir>\n  cbvm analyze-bas-control-flow <game-dir> <output-dir>\n  cbvm analyze-contact-manifest <game-dir> <output-dir>"
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

fn read_profile(game_dir: &Path, script: usize) -> Result<ProfileImages> {
    let name = format!("SCRIPT{script}");
    let read = |extension: &str| {
        let path = game_dir.join(format!("{name}.{extension}"));
        std::fs::read(&path).with_context(|| format!("reading VM image {}", path.display()))
    };
    Ok(ProfileImages {
        name: name.clone(),
        cod: read("COD")?,
        bas: read("BAS")?,
        deb: read("DEB")?,
        dic: read("DIC")?,
        var: read("VAR")?,
    })
}

fn write_profile(profile: &ProfileImages, output_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(output_dir)?;
    for extension in PROFILE_EXTENSIONS {
        let output = output_dir.join(format!("{}.{extension}", profile.name));
        std::fs::write(
            &output,
            profile.image(extension).expect("known profile extension"),
        )
        .with_context(|| format!("writing {}", output.display()))?;
    }
    Ok(())
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
        Some("compile-profile") => {
            let source = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let output_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            if args.next().is_some() {
                usage();
            }
            let text = std::fs::read_to_string(&source)
                .with_context(|| format!("reading {}", source.display()))?;
            let profile = vm_profile::compile(&text)?;
            write_profile(&profile, &output_dir)?;
            println!(
                "wrote {}: five VM resources from {}",
                output_dir.display(),
                source.display()
            );
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
        Some("decompile-unified") => {
            let game_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let output_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            if args.next().is_some() {
                usage();
            }
            std::fs::create_dir_all(&output_dir)?;
            let mut manifest = String::from("script\timage\tinput_bytes\tsource\troundtrip\n");
            for script in 1..=5 {
                let profile = read_profile(&game_dir, script)?;
                let source = vm_profile::decompile(&profile)?;
                let output = output_dir.join(format!("script{script}.blood"));
                std::fs::write(&output, source)
                    .with_context(|| format!("writing {}", output.display()))?;
                for extension in PROFILE_EXTENSIONS {
                    manifest.push_str(&format!(
                        "SCRIPT{script}\t{extension}\t{}\tscript{script}.blood\tbyte_exact\n",
                        profile
                            .image(extension)
                            .expect("known profile extension")
                            .len()
                    ));
                }
                println!("verified SCRIPT{script} -> {}", output.display());
            }
            std::fs::write(output_dir.join("manifest.tsv"), manifest)?;
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
                "script\tinput_bytes\ttokens\tselector_nodes\tlists\tentrypoints\tdirect_next\tedges\tmenu_choices\tdialogue_events\n",
            );
            let mut scenarios = String::from(
                "scenario\tscript\tobject\tlist_index\tnode_offset\tselector_offset\tselector\tmenu_row\tchoice_offset\tchoice\tdialogue_events\n",
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
                    "{name}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                    graph.image_bytes,
                    graph.token_count,
                    graph.selector_node_count,
                    graph.list_count,
                    graph.entrypoint_count,
                    graph.direct_next_count,
                    graph.edge_count,
                    graph.menu_choice_count,
                    graph.dialogue_event_count,
                ));
                for node in &graph.nodes {
                    let object = &graph.lists[node.list_index].entrypoint.object_name;
                    for (row, choice) in node.menu_choices.iter().enumerate() {
                        let selector = node.selector_name.replace(['\t', '\n', '\r'], " ");
                        let choice_text = choice
                            .text
                            .as_deref()
                            .unwrap_or("<unknown>")
                            .replace(['\t', '\n', '\r'], " ");
                        let scenario = format!(
                            "{}:{}:{:04x}:{}",
                            name.to_ascii_lowercase(),
                            object,
                            node.offset,
                            row,
                        );
                        scenarios.push_str(&format!(
                            "{scenario}\t{name}\t{object}\t{}\t0x{:04x}\t0x{:04x}\t{selector}\t{}\t0x{:04x}\t{choice_text}\t{}\n",
                            node.list_index,
                            node.offset,
                            node.selector,
                            row,
                            choice.offset,
                            node.dialogue_events.len(),
                        ));
                    }
                }
                println!("analyzed {} -> {}", image.display(), output.display());
            }
            std::fs::write(output_dir.join("manifest.tsv"), manifest)?;
            std::fs::write(output_dir.join("scenarios.tsv"), scenarios)?;
        }
        Some("analyze-contact-manifest") => {
            let game_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let output_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            if args.next().is_some() {
                usage();
            }
            std::fs::create_dir_all(&output_dir)?;
            let manifest = contact_manifest::analyze_game_dir(&game_dir)?;
            let mut json = serde_json::to_vec_pretty(&manifest)?;
            json.push(b'\n');
            std::fs::write(output_dir.join("contact-manifest.json"), json)?;
            std::fs::write(
                output_dir.join("contact-manifest.tsv"),
                contact_manifest::tsv(&manifest),
            )?;
            println!(
                "wrote {}: {} contact procedure(s), {} direct, {} conditioned, {} text token(s)",
                output_dir.display(),
                manifest.procedure_count,
                manifest.direct_entry_count,
                manifest.conditioned_entry_count,
                manifest.text_count,
            );
        }
        _ => usage(),
    }
    Ok(())
}
