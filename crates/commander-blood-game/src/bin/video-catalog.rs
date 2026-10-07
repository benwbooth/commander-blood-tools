//! Export typed DESCRIPT scene references for the anthology catalog.

use anyhow::{Context, Result};
use commander_blood_formats::descript::DescriptCharacterBackground;
use commander_blood_formats::descript_database::{DescriptCommand as C, DescriptDatabase};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let path = args.next().context("usage: video-catalog DESCRIPT.DES")?;
    anyhow::ensure!(args.next().is_none(), "usage: video-catalog DESCRIPT.DES");
    let bytes = std::fs::read(path)?;
    println!("{}", serde_json::to_string_pretty(&catalog(&bytes)?)?);
    Ok(())
}

fn catalog(bytes: &[u8]) -> Result<Value> {
    let db = DescriptDatabase::parse(bytes).map_err(|error| anyhow::anyhow!("{error:?}"))?;
    let mut records = Vec::with_capacity(db.records().len());
    for (index, record) in db.records().iter().enumerate() {
        // The typed parser validates the 18-byte directory entries and record framing.
        let offset_field = 2 + index * 18 + 16;
        let record_offset = usize::from(u16::from_le_bytes([
            bytes[offset_field],
            bytes[offset_field + 1],
        ]));
        let mut source_offset = record_offset + 2;
        let mut references = Vec::new();
        let mut captions = Vec::new();
        let mut commands = Vec::with_capacity(record.commands().len());
        for command in record.commands() {
            let (entry, byte_count) = command_json(command, bytes, source_offset)?;
            commands.push(entry);
            source_offset += byte_count;
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
                C::Caption(value) => {
                    captions.push(String::from_utf8_lossy(value.text()).into_owned());
                    None
                }
                C::LocationLayout(_) | C::SequenceSubtitle(_) => None,
            };
            if let Some((role, directory, name)) = reference {
                let source = String::from_utf8_lossy(name).replace('\\', "/");
                let resource = if source.contains('/') || directory.is_empty() {
                    source
                } else {
                    format!("{directory}/{source}")
                };
                references.push(json!({"role":role, "resource":resource.to_uppercase()}));
            }
        }
        records.push(json!({
            "name": String::from_utf8_lossy(record.name()),
            "name_bytes": record.name(),
            "kind": format!("{:?}", record.kind()),
            "references": references,
            "captions": captions,
            "commands": commands,
        }));
    }
    Ok(json!({
        "schema": 1,
        "descript_sha256": format!("{:x}", Sha256::digest(bytes)),
        "descript_byte_count": bytes.len(),
        "records": records,
    }))
}

fn named_command(kind: &str, name: &[u8]) -> Value {
    json!({"kind": kind, "name": String::from_utf8_lossy(name), "name_bytes": name})
}

fn background_json(background: DescriptCharacterBackground) -> Value {
    match background {
        DescriptCharacterBackground::None => {
            json!({"kind": "none", "slot": null, "encoded": 255})
        }
        DescriptCharacterBackground::Cached(slot) => {
            json!({"kind": "cached", "slot": slot.encode(), "encoded": slot.encode()})
        }
    }
}

fn command_json(command: &C, bytes: &[u8], source_offset: usize) -> Result<(Value, usize)> {
    // Payload sizes follow the typed codecs; printable names leave the next opcode unconsumed.
    let (mut entry, payload_byte_count) = match command {
        C::Background(value) => {
            let mut entry = named_command("background", value.source_name());
            entry["slot"] = json!(value.slot().encode());
            (entry, 1 + value.source_name().len())
        }
        C::Caption(value) => (
            json!({
                "kind": "caption",
                "text": String::from_utf8_lossy(value.text()),
                "text_bytes": value.text(),
            }),
            value.text().len() + 1,
        ),
        C::LocationVideo(value) => (
            named_command("location_video", value.as_bytes()),
            value.as_bytes().len(),
        ),
        C::TalkClip(value) => {
            let mut entry = named_command("talk_clip", value.video().as_bytes());
            entry["background"] = background_json(value.background());
            (entry, 1 + value.video().as_bytes().len())
        }
        C::LocationLayout(value) => (
            json!({"kind": "location_layout", "top_row": value.top_row()}),
            2,
        ),
        C::CharacterRightVideo(value) => (
            named_command("character_right_video", value.as_bytes()),
            value.as_bytes().len(),
        ),
        C::CharacterLeftVideo(value) => (
            named_command("character_left_video", value.as_bytes()),
            value.as_bytes().len(),
        ),
        C::IdleClip(value) => {
            let mut entry = named_command("idle_clip", value.video().as_bytes());
            entry["background"] = background_json(value.background());
            (entry, 1 + value.video().as_bytes().len())
        }
        C::SequenceVideo(value) => (
            named_command("sequence_video", value.as_bytes()),
            value.as_bytes().len(),
        ),
        C::SequenceSubtitle(value) => (
            json!({
                "kind": "sequence_subtitle",
                "first_visible_frame": value.first_visible_frame(),
                "text": String::from_utf8_lossy(value.text()),
                "text_bytes": value.text(),
            }),
            2 + value.text().len() + 1,
        ),
        C::CharacterSprite(value) => (
            named_command("character_sprite", value.as_bytes()),
            value.as_bytes().len(),
        ),
        C::ObjectVideo(value) => (
            named_command("object_video", value.as_bytes()),
            value.as_bytes().len(),
        ),
        C::SoundBank(value) => (
            named_command("sound_bank", value.as_bytes()),
            value.as_bytes().len(),
        ),
        C::Music(value) => {
            // Music decoding normalizes bytes. Preserve the authored name separately.
            let name = bytes
                .get(source_offset + 1..source_offset + 1 + value.as_bytes().len())
                .context("music command is outside DESCRIPT source bytes")?;
            let mut entry = named_command("music", name);
            entry["normalized_name"] = json!(String::from_utf8_lossy(value.as_bytes()));
            entry["normalized_name_bytes"] = json!(value.as_bytes());
            (entry, value.as_bytes().len())
        }
    };
    let byte_count = 1 + payload_byte_count;
    let source_bytes = bytes
        .get(source_offset..source_offset + byte_count)
        .context("command is outside DESCRIPT source bytes")?;
    entry["source_offset"] = json!(source_offset);
    entry["source_byte_count"] = json!(byte_count);
    entry["source_bytes"] = json!(source_bytes);
    Ok((entry, byte_count))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database(name: &[u8], commands: &[&[u8]]) -> Vec<u8> {
        assert!(name.len() < 16);
        let mut bytes = 1_u16.to_le_bytes().to_vec();
        bytes.extend_from_slice(name);
        bytes.resize(18, 0);
        bytes.extend_from_slice(&21_u16.to_le_bytes());
        bytes.push(4);
        let length =
            u16::try_from(3 + commands.iter().map(|value| value.len()).sum::<usize>()).unwrap();
        bytes.extend_from_slice(&length.to_le_bytes());
        for command in commands {
            bytes.extend_from_slice(command);
        }
        bytes.push(255);
        bytes
    }

    #[test]
    fn exports_every_command_in_order_with_parameters_and_legacy_fields() {
        let source: &[&[u8]] = &[
            b"\x03\x04fd\\room.lbm",
            b"\x05caption\0",
            b"\x06arrival.hnm",
            b"\x07\x02talk.hnm",
            b"\x08\x34\x12",
            b"\x09right.hnm",
            b"\x0aleft.hnm",
            b"\x0b\xffidle.hnm",
            b"\x0csequence.hnm",
            b"\x0d\x23\x01subtitle\0",
            b"\x0eportrait.spr",
            b"\x10object.hnm",
            b"\x11chatter.snd",
            b"\x12mixed.voc",
            b"\x07\xfftalk.hnm",
        ];
        let bytes = database(b"record", source);
        let output = catalog(&bytes).unwrap();
        let record = &output["records"][0];
        let commands = record["commands"].as_array().unwrap();
        let kinds: Vec<_> = commands
            .iter()
            .map(|value| value["kind"].as_str().unwrap())
            .collect();
        assert_eq!(
            kinds,
            [
                "background",
                "caption",
                "location_video",
                "talk_clip",
                "location_layout",
                "character_right_video",
                "character_left_video",
                "idle_clip",
                "sequence_video",
                "sequence_subtitle",
                "character_sprite",
                "object_video",
                "sound_bank",
                "music",
                "talk_clip",
            ]
        );
        assert_eq!(record["name"], "record");
        assert_eq!(record["name_bytes"], json!(b"record".as_slice()));
        assert_eq!(record["kind"], "Sequence");
        assert_eq!(record["captions"], json!(["caption"]));
        assert_eq!(
            record["references"],
            json!([
                {"role": "background", "resource": "FD/ROOM.LBM"},
                {"role": "arrival", "resource": "PL/ARRIVAL.HNM"},
                {"role": "talk", "resource": "PE/TALK.HNM"},
                {"role": "right", "resource": "PE/RIGHT.HNM"},
                {"role": "left", "resource": "PE/LEFT.HNM"},
                {"role": "idle", "resource": "PE/IDLE.HNM"},
                {"role": "sequence", "resource": "SQ/SEQUENCE.HNM"},
                {"role": "portrait", "resource": "PORTRAIT.SPR"},
                {"role": "object", "resource": "OB/OBJECT.HNM"},
                {"role": "chatter", "resource": "SN/CHATTER.SND"},
                {"role": "music", "resource": "MU/MIXED.VOC"},
                {"role": "talk", "resource": "PE/TALK.HNM"},
            ])
        );
        assert_eq!(commands[0]["slot"], 4);
        assert_eq!(
            commands[3]["background"],
            json!({"kind": "cached", "slot": 2, "encoded": 2})
        );
        assert_eq!(
            commands[7]["background"],
            json!({"kind": "none", "slot": null, "encoded": 255})
        );
        assert_eq!(commands[14]["background"], commands[7]["background"]);
        assert_eq!(commands[4]["top_row"], 0x1234);
        assert_eq!(commands[9]["first_visible_frame"], 0x0123);
        assert_eq!(commands[13]["name_bytes"], json!(b"mixed.voc".as_slice()));
        assert_eq!(
            commands[13]["normalized_name_bytes"],
            json!(b"MIXED.VOC".as_slice())
        );
        let mut offset = 23;
        for (command, encoded) in commands.iter().zip(source) {
            assert_eq!(command["source_offset"], offset);
            assert_eq!(command["source_byte_count"], encoded.len());
            assert_eq!(command["source_bytes"], json!(encoded));
            offset += encoded.len();
        }
        assert_eq!(offset, bytes.len() - 1);
    }

    #[test]
    fn preserves_non_utf8_name_and_text_bytes_alongside_readable_strings() {
        let bytes = database(
            b"R\xff",
            &[b"\x05A\x80\xff\0", b"\x0d\xff\xffB\xfe\0", b"\x05\0"],
        );
        let output = catalog(&bytes).unwrap();
        let record = &output["records"][0];
        assert_eq!(record["name_bytes"], json!([b'R', 255]));
        assert_eq!(record["name"], String::from_utf8_lossy(b"R\xff").as_ref());
        assert_eq!(record["commands"][0]["text_bytes"], json!([b'A', 128, 255]));
        assert_eq!(
            record["commands"][0]["text"],
            String::from_utf8_lossy(b"A\x80\xff").as_ref()
        );
        assert_eq!(record["commands"][1]["text_bytes"], json!([b'B', 254]));
        assert_eq!(record["commands"][1]["first_visible_frame"], 65535);
        assert_eq!(record["commands"][2]["text_bytes"], json!([]));
        assert_eq!(
            record["captions"],
            json!([String::from_utf8_lossy(b"A\x80\xff"), ""])
        );
    }

    #[test]
    fn root_provenance_hashes_the_complete_source_and_keeps_schema_one() {
        let bytes = database(b"record", &[]);
        let output = catalog(&bytes).unwrap();
        assert_eq!(output["schema"], 1);
        assert_eq!(output["descript_byte_count"], bytes.len());
        assert_eq!(
            output["descript_sha256"],
            format!("{:x}", Sha256::digest(&bytes))
        );
        assert_eq!(output["records"][0]["commands"], json!([]));

        let mut changed = bytes.clone();
        changed[17] = 42; // Unused directory padding still belongs to source provenance.
        let changed_output = catalog(&changed).unwrap();
        assert_ne!(changed_output["descript_sha256"], output["descript_sha256"]);
        assert_eq!(changed_output["records"], output["records"]);
        assert!(catalog(&bytes[..bytes.len() - 1]).is_err());
    }
}
