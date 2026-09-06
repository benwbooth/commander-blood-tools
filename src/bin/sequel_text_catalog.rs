//! Authored COD translation source, not a reachability or localization-completion claim.

use std::path::Path;

use anyhow::{Context, Result, ensure};
use commander_blood_formats::code::{ScriptDialect, decode_script_code_for_dialect};
use commander_blood_formats::instruction::{ScriptText, ScriptTextWord, decode_script_text};
use commander_blood_formats::script::{ScriptDictionary, decode_script_dictionary};
use commander_blood_tools::font::cp437_string;
use serde::Serialize;
use sha2::{Digest, Sha256};

const PROFILE_COUNT: usize = 17;
const TEXT_OPCODE: u8 = 166;
const CP437_DECODER_LIMIT: u8 = 176;

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Part {
    Word {
        dictionary_byte: u16,
        source: String,
        source_bytes: Vec<u8>,
        unmapped_bytes: Vec<u8>,
    },
    StateNumber {
        state_byte: u16,
    },
    InventoryChoices,
}

#[derive(Debug, Serialize)]
struct Section {
    source: String,
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Message {
    id: String,
    source_byte: usize,
    line_record_byte: usize,
    presentation_selector: i8,
    control: u16,
    emits_spoken_text: bool,
    arms_resume: bool,
    uses_history_condition: bool,
    resume_byte: Option<usize>,
    record_condition_operand: Option<u16>,
    sections: Vec<Section>,
}

#[derive(Serialize)]
struct Profile {
    name: String,
    cod_sha256: String,
    dic_sha256: String,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Catalog {
    format: &'static str,
    game: &'static str,
    source_encoding: &'static str,
    scope: &'static str,
    excluded: [&'static str; 3],
    profiles: Vec<Profile>,
}

fn message(
    name: &str,
    source_byte: usize,
    text: &ScriptText,
    dictionary: &ScriptDictionary,
) -> Result<Message> {
    let mut sections = vec![Section {
        source: String::new(),
        parts: Vec::new(),
    }];
    for word in &text.words {
        if *word == ScriptTextWord::SectionSeparator {
            sections.push(Section {
                source: String::new(),
                parts: Vec::new(),
            });
            continue;
        }
        let (part, preview) = match word {
            ScriptTextWord::Dictionary(word) => {
                let bytes = dictionary.word(*word).context("missing interned word")?;
                let unmapped_bytes = bytes
                    .iter()
                    .copied()
                    .filter(|byte| *byte >= CP437_DECODER_LIMIT)
                    .collect();
                let source = bytes
                    .iter()
                    .map(|byte| {
                        if *byte >= CP437_DECODER_LIMIT {
                            format!("\\x{byte:02x}")
                        } else {
                            cp437_string(&[*byte])
                        }
                    })
                    .collect::<String>();
                let dictionary_byte = dictionary
                    .source_offset(*word)
                    .context("missing dictionary source identity")?;
                (
                    Part::Word {
                        dictionary_byte,
                        source: source.clone(),
                        source_bytes: bytes.to_vec(),
                        unmapped_bytes,
                    },
                    source,
                )
            }
            ScriptTextWord::StateNumber(number) => {
                let state_byte = number.source_offset();
                (
                    Part::StateNumber { state_byte },
                    format!("<state:{state_byte}>"),
                )
            }
            ScriptTextWord::InventoryChoices => {
                (Part::InventoryChoices, "<inventory_choices>".to_owned())
            }
            ScriptTextWord::SectionSeparator => unreachable!("handled before section access"),
        };
        let section = sections
            .last_mut()
            .expect("an initial section always exists");
        if !section.parts.is_empty() {
            section.source.push(' ');
        }
        section.source.push_str(&preview);
        section.parts.push(part);
    }
    Ok(Message {
        id: format!("bbb.{}.cod.{source_byte:08x}", name.to_ascii_lowercase()),
        source_byte,
        line_record_byte: text.line_record.byte_offset(),
        presentation_selector: text.presentation_selector,
        control: text.control.bits(),
        emits_spoken_text: text.control.emits_spoken_text(),
        arms_resume: text.control.arms_resume(),
        uses_history_condition: text.control.uses_history_condition(),
        resume_byte: text.resume_target.map(|target| target.index()),
        record_condition_operand: text.record_condition_operand,
        sections,
    })
}

fn export(root: &Path) -> Result<Catalog> {
    let mut profiles = Vec::new();
    for profile in 1..=PROFILE_COUNT {
        let name = format!("SCRIPT{profile}");
        let cod = std::fs::read(root.join(format!("{name}.COD")))
            .with_context(|| format!("reading {name}.COD"))?;
        let dic = std::fs::read(root.join(format!("{name}.DIC")))
            .with_context(|| format!("reading {name}.DIC"))?;
        let dictionary = decode_script_dictionary(&dic)?;
        let code = decode_script_code_for_dialect(&cod, ScriptDialect::BigBugBang)?;
        let mut messages = Vec::new();
        for token in code
            .tokens()
            .iter()
            .filter(|token| token.opcode().byte() == TEXT_OPCODE)
        {
            let text = decode_script_text(token, &dictionary)?;
            messages.push(
                message(&name, token.source_offset().index(), &text, &dictionary)
                    .with_context(|| format!("{name}.COD at {}", token.source_offset().index()))?,
            );
        }
        profiles.push(Profile {
            name,
            cod_sha256: format!("{:x}", Sha256::digest(&cod)),
            dic_sha256: format!("{:x}", Sha256::digest(&dic)),
            messages,
        });
    }
    Ok(Catalog {
        format: "bbb-authored-cod-text-v1",
        game: "big-bug-bang",
        source_encoding: "repository CP437 decoder; bytes outside its range use explicit \\xNN escapes, with source_bytes retained",
        scope: "All typed A6 instructions in 17 authored COD resources; no reachability claim. Section source is a reading aid; typed parts and original identities are authoritative.",
        excluded: [
            "BAS dialogue",
            "native UI and object display names",
            "media-embedded text",
        ],
        profiles,
    })
}

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let root = args
        .next()
        .context("usage: sequel_text_catalog <loose-resource-root>")?;
    ensure!(
        args.next().is_none(),
        "expected only the loose resource root"
    );
    let catalog = export(Path::new(&root))?;
    serde_json::to_writer_pretty(std::io::stdout().lock(), &catalog)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use commander_blood_formats::instruction::{
        ScriptLineRecordOffset, ScriptTextControl, ScriptTextStateNumber,
    };

    #[test]
    #[ignore = "requires the user's imported Big Bug Bang resources"]
    fn original_corpus_preserves_every_word_and_dynamic_marker() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("output/big-bug-bang/imported-assets/resources");
        let catalog = export(&root).unwrap();
        let expected_counts = [
            89, 1197, 779, 660, 216, 543, 140, 288, 176, 303, 633, 302, 775, 377, 102, 209, 132,
        ];
        assert_eq!(catalog.profiles.len(), expected_counts.len());
        let mut identities = std::collections::HashSet::new();
        let (mut word_count, mut number_count, mut inventory_count, mut unmapped_count) =
            (0, 0, 0, 0);
        for (profile, expected) in catalog.profiles.iter().zip(expected_counts) {
            assert_eq!(profile.messages.len(), expected, "{}", profile.name);
            let dictionary = std::fs::read(root.join(format!("{}.DIC", profile.name))).unwrap();
            for message in &profile.messages {
                assert!(identities.insert(&message.id));
                for part in message.sections.iter().flat_map(|section| &section.parts) {
                    match part {
                        Part::Word {
                            dictionary_byte,
                            source_bytes,
                            unmapped_bytes,
                            ..
                        } => {
                            word_count += 1;
                            let start = usize::from(*dictionary_byte);
                            let end = start
                                + dictionary[start..]
                                    .iter()
                                    .position(|byte| *byte == 0)
                                    .unwrap();
                            assert_eq!(source_bytes.as_slice(), &dictionary[start..end]);
                            unmapped_count += usize::from(!unmapped_bytes.is_empty());
                        }
                        Part::StateNumber { .. } => number_count += 1,
                        Part::InventoryChoices => inventory_count += 1,
                    }
                }
            }
        }
        assert_eq!(identities.len(), 6921);
        assert_eq!(
            (word_count, number_count, inventory_count, unmapped_count),
            (44599, 58, 46, 5)
        );
    }

    #[test]
    fn sections_keep_dictionary_identity_and_dynamic_operands() {
        let dictionary = decode_script_dictionary(b"\0\0caf\x82\0JOUER\0A\xefe\0").unwrap();
        let text = ScriptText {
            line_record: ScriptLineRecordOffset::decode(10),
            presentation_selector: -1,
            control: ScriptTextControl::decode(32816),
            resume_target: Some(commander_blood_formats::code::ScriptCodeOffset::new(80)),
            record_condition_operand: None,
            words: vec![
                ScriptTextWord::Dictionary(dictionary.resolve_source_offset(2).unwrap()),
                ScriptTextWord::StateNumber(ScriptTextStateNumber::decode(8368)),
                ScriptTextWord::SectionSeparator,
                ScriptTextWord::Dictionary(dictionary.resolve_source_offset(7).unwrap()),
                ScriptTextWord::InventoryChoices,
                ScriptTextWord::SectionSeparator,
                ScriptTextWord::Dictionary(dictionary.resolve_source_offset(13).unwrap()),
                ScriptTextWord::SectionSeparator,
            ]
            .into_boxed_slice(),
        };
        let actual = message("SCRIPT1", 100, &text, &dictionary).unwrap();
        assert_eq!(actual.id, "bbb.script1.cod.00000064");
        assert_eq!(actual.sections.len(), 4);
        assert_eq!(actual.sections[0].source, "caf\u{e9} <state:8368>");
        assert_eq!(actual.sections[1].source, "JOUER <inventory_choices>");
        assert_eq!(actual.sections[2].source, "A\\xefe");
        assert!(
            matches!(&actual.sections[2].parts[0], Part::Word { source_bytes, unmapped_bytes, .. }
            if source_bytes == b"A\xefe" && unmapped_bytes == &[239])
        );
        assert!(actual.sections[3].parts.is_empty());
        assert!(matches!(
            actual.sections[1].parts[0],
            Part::Word {
                dictionary_byte: 7,
                ..
            }
        ));
        assert!(matches!(
            actual.sections[0].parts[1],
            Part::StateNumber { state_byte: 8368 }
        ));
        assert_eq!(actual.resume_byte, Some(80));
        assert!(actual.emits_spoken_text && actual.arms_resume);
    }
}
