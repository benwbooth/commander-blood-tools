mod extract;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn usage() {
    eprintln!(
        "Commander Blood reverse-engineering tools\n\
         \n\
         Usage:\n\
           commander-blood-tools inspect-bloodprg [BLOODPRG.EXE]\n\
           commander-blood-tools inspect-vm SCRIPT.COD [SCRIPT.VAR]\n\
           commander-blood-tools inspect-descript DESCRIPT.DES\n\
           commander-blood-tools inspect-scripts GAME_DIR\n\
           commander-blood-tools inspect-character-combinations GAME_DIR\n\
           commander-blood-tools <extractor options>\n\
         \n\
         Run `cbvm --help` for BloodScript compiler commands.\n\
         Run `commander-blood` to start the new Rust game port."
    );
}

fn run() -> anyhow::Result<()> {
    let mut arguments = std::env::args().skip(1);
    match arguments.next().as_deref() {
        None => {
            usage();
            Ok(())
        }
        Some("inspect-bloodprg") => {
            let path = arguments
                .next()
                .unwrap_or_else(|| "re/bin/BLOODPRG.EXE".to_string());
            let binary = commander_blood_tools::bloodprg::BloodPrg::parse_file(&path)?;
            println!("{}", serde_json::to_string_pretty(&binary.inspect()?)?);
            Ok(())
        }
        Some("inspect-vm") => {
            #[derive(serde::Serialize)]
            struct VmInspection {
                tokens: Vec<commander_blood_tools::vm::VmToken>,
                line_states: Option<Vec<commander_blood_tools::vm::LineState>>,
                execution_trace: Option<commander_blood_tools::vm::ExecutionTrace>,
            }

            let cod_path = arguments
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: inspect-vm <SCRIPT.COD> [SCRIPT.VAR]"))?;
            let cod = std::fs::read(&cod_path)?;
            let tokens = commander_blood_tools::vm::walk(&cod, 0, cod.len());
            let var = arguments.next().map(std::fs::read).transpose()?;
            let line_states = var
                .as_ref()
                .map(|var| commander_blood_tools::vm::interpret_line_states(&cod, var));
            let execution_trace = var
                .as_ref()
                .map(|var| commander_blood_tools::vm::execute_trace(&cod, var));
            println!(
                "{}",
                serde_json::to_string_pretty(&VmInspection {
                    tokens,
                    line_states,
                    execution_trace,
                })?
            );
            Ok(())
        }
        Some("inspect-descript") => {
            let path = arguments
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: inspect-descript <DESCRIPT.DES>"))?;
            let database = commander_blood_tools::descript::DescriptDb::parse_file(&path)?;
            println!("{}", serde_json::to_string_pretty(&database)?);
            Ok(())
        }
        Some("inspect-scripts") => {
            let game_directory = arguments
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: inspect-scripts <game-dir>"))?;
            let descript_path = std::path::Path::new(&game_directory).join("DESCRIPT.DES");
            let database = commander_blood_tools::descript::DescriptDb::parse_file(&descript_path)?;
            let bundles = commander_blood_tools::script::parse_script_dir(
                &game_directory,
                &database,
                &database.hnm_music_map(),
            )?;
            println!("{}", serde_json::to_string_pretty(&bundles)?);
            Ok(())
        }
        Some("inspect-character-combinations") => {
            let game_directory = arguments.next().ok_or_else(|| {
                anyhow::anyhow!("usage: inspect-character-combinations <game-dir>")
            })?;
            let descript_path = std::path::Path::new(&game_directory).join("DESCRIPT.DES");
            let database = commander_blood_tools::descript::DescriptDb::parse_file(&descript_path)?;
            let bundles = commander_blood_tools::script::parse_script_dir(
                &game_directory,
                &database,
                &database.hnm_music_map(),
            )?;

            println!(
                "script\tactor\tactor_object_offset\tactor_talk_ref\tlocation_record\tbackground_hnm\tbackground_music\tsource"
            );
            for bundle in bundles {
                for context in bundle.character_contexts {
                    println!(
                        "{}\t{}\t0x{:04x}\t0x{:04x}\t{}\t{}\t{}\t{}",
                        context.script,
                        context.actor_record,
                        context.actor_object_offset,
                        context.actor_talk_ref,
                        context.location_record.as_deref().unwrap_or(""),
                        context.background_hnm.as_deref().unwrap_or(""),
                        context.background_music.as_deref().unwrap_or(""),
                        context.source
                    );
                }
            }
            Ok(())
        }
        Some(_) => extract::run().map_err(|error| anyhow::anyhow!("{error}")),
    }
}
