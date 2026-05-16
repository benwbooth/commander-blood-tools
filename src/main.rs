mod legacy;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("inspect-descript") => {
            let path = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: inspect-descript <DESCRIPT.DES>"))?;
            let db = commander_blood_tools::descript::DescriptDb::parse_file(&path)?;
            println!("{}", serde_json::to_string_pretty(&db)?);
            Ok(())
        }
        Some("inspect-scripts") => {
            let iso_dir = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: inspect-scripts <iso-dir>"))?;
            let descript_path = std::path::Path::new(&iso_dir).join("DESCRIPT.DES");
            let db = commander_blood_tools::descript::DescriptDb::parse_file(&descript_path)?;
            let hnm_music = db.hnm_music_map();
            let bundles =
                commander_blood_tools::script::parse_script_dir(&iso_dir, &db, &hnm_music)?;
            println!("{}", serde_json::to_string_pretty(&bundles)?);
            Ok(())
        }
        Some("inspect-character-combinations") => {
            let iso_dir = args.next().ok_or_else(|| {
                anyhow::anyhow!("usage: inspect-character-combinations <iso-dir>")
            })?;
            let descript_path = std::path::Path::new(&iso_dir).join("DESCRIPT.DES");
            let db = commander_blood_tools::descript::DescriptDb::parse_file(&descript_path)?;
            let hnm_music = db.hnm_music_map();
            let bundles =
                commander_blood_tools::script::parse_script_dir(&iso_dir, &db, &hnm_music)?;

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
        _ => legacy::run().map_err(|err| anyhow::anyhow!("{err}")),
    }
}
