//! Export typed DESCRIPT scene references for the anthology catalog.

use anyhow::{Context, Result};
use commander_blood_formats::descript_database::{DescriptCommand as C, DescriptDatabase};
use serde_json::json;

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let path = args.next().context("usage: video-catalog DESCRIPT.DES")?;
    anyhow::ensure!(args.next().is_none(), "usage: video-catalog DESCRIPT.DES");
    let bytes = std::fs::read(path)?;
    let db = DescriptDatabase::parse(&bytes).map_err(|error| anyhow::anyhow!("{error:?}"))?;
    let records: Vec<_> = db.records().iter().map(|record| {
        let mut references = Vec::new();
        let mut captions = Vec::new();
        for command in record.commands() {
            let reference = match command {
                C::Background(value) => Some(("background", "FD", value.source_name())),
                C::LocationVideo(value) => Some(("arrival", "PL", value.as_bytes())),
                C::TalkClip(value) => Some(("talk", "PE", value.video().as_bytes())),
                C::IdleClip(value) => Some(("idle", "PE", value.video().as_bytes())),
                C::CharacterRightVideo(value) => Some(("right", "PE", value.as_bytes())),
                C::CharacterLeftVideo(value) => Some(("left", "PE", value.as_bytes())),
                C::SequenceVideo(value) => Some(("sequence", "SQ", value.as_bytes())),
                C::ObjectVideo(value) => Some(("object", "OB", value.as_bytes())),
                C::SoundBank(value) => Some(("chatter", "SN", value.as_bytes())),
                C::Music(value) => Some(("music", "MU", value.as_bytes())),
                C::CharacterSprite(value) => Some(("portrait", "", value.as_bytes())),
                C::Caption(value) => { captions.push(String::from_utf8_lossy(value.text()).into_owned()); None },
                _ => None,
            };
            if let Some((role, directory, name)) = reference {
                let source = String::from_utf8_lossy(name).replace('\\', "/");
                let resource = if source.contains('/') || directory.is_empty() { source } else { format!("{directory}/{source}") };
                references.push(json!({"role":role, "resource":resource.to_uppercase()}));
            }
        }
        json!({"name":String::from_utf8_lossy(record.name()), "kind":format!("{:?}", record.kind()), "references":references, "captions":captions})
    }).collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"schema":1, "records":records}))?
    );
    Ok(())
}
