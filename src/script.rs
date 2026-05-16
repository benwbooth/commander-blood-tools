use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::descript::{DescriptDb, RecordKind};
use crate::util::media_stem;

pub const OBJECT_LOCATION_FIELD: usize = 24;
pub const OBJECT_TALK_FIELD: u16 = 0x3a;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DebSymbol {
    pub name: String,
    pub offset: u16,
    pub kind: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ScriptFunction {
    pub script: String,
    pub name: String,
    pub offset: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CharacterContext {
    pub script: String,
    pub actor_record: String,
    pub actor_object_offset: u16,
    pub actor_talk_ref: u16,
    pub talk_count: usize,
    pub location_record: Option<String>,
    pub background_hnm: Option<String>,
    pub background_music: Option<String>,
    pub source: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SpeechEvent {
    pub script: String,
    pub function_name: String,
    pub offset: usize,
    pub actor_record: Option<String>,
    pub param0: Option<u8>,
    pub param1: Option<u8>,
    pub clip_index: Option<usize>,
    pub background_record: Option<String>,
    pub background_hnm: Option<String>,
    pub background_music: Option<String>,
    pub source: String,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ScriptBundle {
    pub script: String,
    pub symbols: Vec<DebSymbol>,
    pub functions: Vec<ScriptFunction>,
    pub character_contexts: Vec<CharacterContext>,
    pub speech_events: Vec<SpeechEvent>,
}

pub fn parse_deb(data: &[u8]) -> Vec<DebSymbol> {
    data.chunks_exact(20)
        .filter_map(|record| {
            let name_len = record[..16].iter().position(|&b| b == 0).unwrap_or(16);
            if name_len == 0 {
                return None;
            }
            Some(DebSymbol {
                name: String::from_utf8_lossy(&record[..name_len]).to_string(),
                offset: u16::from_le_bytes([record[16], record[17]]),
                kind: u16::from_le_bytes([record[18], record[19]]),
            })
        })
        .collect()
}

pub fn parse_dictionary(data: &[u8]) -> HashMap<u16, String> {
    let mut words = HashMap::new();
    let mut pos = 0usize;
    while pos < data.len() {
        let start = pos;
        while pos < data.len() && data[pos] != 0 {
            pos += 1;
        }
        if pos > start {
            words.insert(
                start as u16,
                String::from_utf8_lossy(&data[start..pos]).to_string(),
            );
        }
        pos += 1;
    }
    words
}

pub fn functions_from_symbols(
    script: &str,
    symbols: &[DebSymbol],
    cod_len: usize,
) -> Vec<ScriptFunction> {
    let mut functions: Vec<_> = symbols
        .iter()
        .filter(|symbol| symbol.kind == 2 && symbol.offset != 0xffff)
        .map(|symbol| ScriptFunction {
            script: script.to_string(),
            name: symbol.name.clone(),
            offset: symbol.offset as usize,
        })
        .filter(|function| function.offset < cod_len)
        .collect();
    if functions.is_empty() {
        functions.push(ScriptFunction {
            script: script.to_string(),
            name: script.to_string(),
            offset: 0,
        });
    }
    functions.sort_by_key(|function| function.offset);
    functions.dedup_by(|a, b| a.offset == b.offset && a.name.eq_ignore_ascii_case(&b.name));
    functions
}

pub fn build_character_contexts(
    script: &str,
    symbols: &[DebSymbol],
    var: &[u8],
    descript_db: &DescriptDb,
    hnm_music: &HashMap<String, String>,
) -> Vec<CharacterContext> {
    let object_names = object_name_map(symbols);
    let character_names = descript_db.character_names();
    let mut contexts = Vec::new();

    for symbol in symbols.iter().filter(|symbol| symbol.kind == 1) {
        if !character_names
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(&symbol.name))
        {
            continue;
        }

        let Some(actor_record) = descript_db.record(&symbol.name) else {
            continue;
        };

        let var_offset = symbol.offset as usize;
        let location_offset = var
            .get(var_offset + OBJECT_LOCATION_FIELD..var_offset + OBJECT_LOCATION_FIELD + 2)
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]));
        let location_record = location_offset.and_then(|loc| object_names.get(&loc).cloned());
        let background = location_record
            .as_ref()
            .and_then(|loc_name| descript_db.record(loc_name))
            .filter(|record| record.kind == RecordKind::Location);
        let background_hnm = background.and_then(|record| record.full_hnms.first().cloned());
        let background_music = background_hnm
            .as_ref()
            .and_then(|hnm| hnm_music.get(&media_stem(hnm)).cloned())
            .or_else(|| {
                background
                    .and_then(|record| record.music.first())
                    .map(|music| media_stem(music))
            });

        contexts.push(CharacterContext {
            script: script.to_string(),
            actor_record: actor_record.name.clone(),
            actor_object_offset: symbol.offset,
            actor_talk_ref: symbol.offset.saturating_add(OBJECT_TALK_FIELD),
            talk_count: actor_record.talk_hnms.len(),
            location_record,
            background_hnm,
            background_music,
            source: format!("{script}.DEB object + {script}.VAR object location field"),
        });
    }

    contexts.sort_by(|a, b| {
        a.actor_record
            .to_ascii_lowercase()
            .cmp(&b.actor_record.to_ascii_lowercase())
            .then(a.actor_object_offset.cmp(&b.actor_object_offset))
    });
    contexts
}

pub fn parse_speech_events(
    script: &str,
    cod: &[u8],
    dictionary: &HashMap<u16, String>,
    functions: &[ScriptFunction],
    contexts: &[CharacterContext],
) -> Vec<SpeechEvent> {
    let actor_refs: HashMap<u16, &CharacterContext> = contexts
        .iter()
        .map(|context| (context.actor_talk_ref, context))
        .collect();
    let mut functions = functions.to_vec();
    if functions.is_empty() {
        functions.push(ScriptFunction {
            script: script.to_string(),
            name: script.to_string(),
            offset: 0,
        });
    }
    functions.sort_by_key(|function| function.offset);

    let mut events = Vec::new();
    for idx in 0..functions.len() {
        let function_start = functions[idx].offset;
        let function_end = functions
            .get(idx + 1)
            .map(|next| next.offset)
            .unwrap_or(cod.len())
            .min(cod.len());
        if function_start >= function_end {
            continue;
        }

        let mut current_actor: Option<&CharacterContext> = None;
        let mut rel = 0usize;
        while function_start + rel < function_end {
            let pos = function_start + rel;
            if pos + 2 < function_end && cod[pos] == 0xc4 {
                let addr = u16::from_le_bytes([cod[pos + 1], cod[pos + 2]]);
                if let Some(actor) = actor_refs.get(&addr) {
                    current_actor = Some(*actor);
                }
            }

            if !cod[pos..function_end].starts_with(b"\xa6\x0a\x07") {
                rel += 1;
                continue;
            }

            let Some(marker_rel) = cod[pos + 3..function_end.min(pos + 12)]
                .iter()
                .position(|&b| b == 0x80)
            else {
                rel += 1;
                continue;
            };
            let marker = pos + 3 + marker_rel;
            if marker < pos + 5 {
                rel += 1;
                continue;
            }

            let mut text_pos = marker + 1;
            let mut words = Vec::new();
            while text_pos + 1 < function_end {
                let word_off = u16::from_le_bytes([cod[text_pos], cod[text_pos + 1]]);
                text_pos += 2;
                if word_off == 0 {
                    break;
                }
                let Some(word) = dictionary.get(&word_off) else {
                    words.clear();
                    break;
                };
                words.push(word.as_str());
            }

            if words.is_empty() {
                rel += 1;
                continue;
            }

            let params = &cod[pos + 3..marker];
            let param0 = params.first().copied();
            let param1 = params.get(1).copied();
            let actor_speaks = current_actor.is_some() && param1.is_some_and(|style| style < 0x10);
            let clip_index = current_actor.and_then(|actor| {
                if !actor_speaks {
                    return None;
                }
                match (param0, param1) {
                    (Some(0xff), Some(idx)) if (idx as usize) < actor.talk_count => {
                        Some(idx as usize)
                    }
                    (Some(idx), _) if idx > 0 && (idx as usize) <= actor.talk_count => {
                        Some(idx as usize - 1)
                    }
                    _ => None,
                }
            });
            let source = match (current_actor, actor_speaks, clip_index) {
                (Some(_), true, Some(_)) => {
                    "SCRIPT bytecode actor ref + DESCRIPT talk clip".to_string()
                }
                (Some(_), true, None) => {
                    "SCRIPT bytecode actor ref; subtitle has no mapped talk clip".to_string()
                }
                (Some(_), false, _) => {
                    "SCRIPT bytecode actor ref; non-character subtitle channel".to_string()
                }
                (None, _, _) => "SCRIPT subtitle text only".to_string(),
            };

            events.push(SpeechEvent {
                script: script.to_string(),
                function_name: functions[idx].name.clone(),
                offset: pos,
                actor_record: current_actor.map(|actor| actor.actor_record.clone()),
                param0,
                param1,
                clip_index,
                background_record: current_actor.and_then(|actor| actor.location_record.clone()),
                background_hnm: current_actor.and_then(|actor| actor.background_hnm.clone()),
                background_music: current_actor.and_then(|actor| actor.background_music.clone()),
                source,
                text: words.join(" "),
            });

            rel += 1;
        }
    }

    events
}

pub fn parse_script_bundle(
    script: &str,
    cod: &[u8],
    deb: &[u8],
    dic: &[u8],
    var: &[u8],
    descript_db: &DescriptDb,
    hnm_music: &HashMap<String, String>,
) -> ScriptBundle {
    let symbols = parse_deb(deb);
    let dictionary = parse_dictionary(dic);
    let functions = functions_from_symbols(script, &symbols, cod.len());
    let character_contexts =
        build_character_contexts(script, &symbols, var, descript_db, hnm_music);
    let speech_events =
        parse_speech_events(script, cod, &dictionary, &functions, &character_contexts);

    ScriptBundle {
        script: script.to_string(),
        symbols,
        functions,
        character_contexts,
        speech_events,
    }
}

pub fn parse_script_dir(
    iso_dir: impl AsRef<Path>,
    descript_db: &DescriptDb,
    hnm_music: &HashMap<String, String>,
) -> Result<Vec<ScriptBundle>> {
    let iso_dir = iso_dir.as_ref();
    let mut bundles = Vec::new();
    for script_idx in 1..=5 {
        let script = format!("SCRIPT{script_idx}");
        let Some(cod_path) = find_file_recursive(iso_dir, &format!("{script}.COD")) else {
            continue;
        };
        let Some(deb_path) = find_file_recursive(iso_dir, &format!("{script}.DEB")) else {
            continue;
        };
        let Some(dic_path) = find_file_recursive(iso_dir, &format!("{script}.DIC")) else {
            continue;
        };
        let Some(var_path) = find_file_recursive(iso_dir, &format!("{script}.VAR")) else {
            continue;
        };

        let cod = fs::read(&cod_path).with_context(|| format!("reading {}", cod_path.display()))?;
        let deb = fs::read(&deb_path).with_context(|| format!("reading {}", deb_path.display()))?;
        let dic = fs::read(&dic_path).with_context(|| format!("reading {}", dic_path.display()))?;
        let var = fs::read(&var_path).with_context(|| format!("reading {}", var_path.display()))?;
        bundles.push(parse_script_bundle(
            &script,
            &cod,
            &deb,
            &dic,
            &var,
            descript_db,
            hnm_music,
        ));
    }
    Ok(bundles)
}

pub fn object_name_map(symbols: &[DebSymbol]) -> HashMap<u16, String> {
    symbols
        .iter()
        .filter(|symbol| symbol.kind == 1)
        .map(|symbol| (symbol.offset, symbol.name.clone()))
        .collect()
}

fn find_file_recursive(dir: &Path, name: &str) -> Option<PathBuf> {
    let target = name.to_ascii_lowercase();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        let Ok(read_dir) = fs::read_dir(&path) else {
            continue;
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path
                .file_name()
                .map(|file_name| file_name.to_string_lossy().to_ascii_lowercase() == target)
                .unwrap_or(false)
            {
                return Some(path);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_deb_twenty_byte_records() {
        let mut data = Vec::new();
        let mut record = [0u8; 20];
        record[..11].copy_from_slice(b"Bob_Morlock");
        record[16..18].copy_from_slice(&0x1234u16.to_le_bytes());
        record[18..20].copy_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&record);

        let symbols = parse_deb(&data);
        assert_eq!(
            symbols,
            vec![DebSymbol {
                name: "Bob_Morlock".to_string(),
                offset: 0x1234,
                kind: 1,
            }]
        );
    }

    #[test]
    fn parses_dictionary_offsets() {
        let dict = parse_dictionary(b"HELLO\0WORLD\0");
        assert_eq!(dict.get(&0).map(String::as_str), Some("HELLO"));
        assert_eq!(dict.get(&6).map(String::as_str), Some("WORLD"));
    }

    #[test]
    fn decodes_speech_events_with_actor_clip_mapping() {
        let dictionary = parse_dictionary(b"\0HELLO\0WORLD\0");
        let functions = vec![ScriptFunction {
            script: "SCRIPTX".to_string(),
            name: "Func".to_string(),
            offset: 0,
        }];
        let contexts = vec![CharacterContext {
            script: "SCRIPTX".to_string(),
            actor_record: "Bob_Morlock".to_string(),
            actor_object_offset: 0x0100,
            actor_talk_ref: 0x013a,
            talk_count: 3,
            location_record: Some("gobar".to_string()),
            background_hnm: Some("gobar1.hnm".to_string()),
            background_music: Some("carnhal2".to_string()),
            source: "test".to_string(),
        }];
        let cod = [
            0xc4, 0x3a, 0x01, // current actor = Bob_Morlock + talk field
            0xa6, 0x0a, 0x07, 0x02, 0x01, 0x80, 0x01, 0x00, 0x07, 0x00, 0x00, 0x00,
        ];

        let events = parse_speech_events("SCRIPTX", &cod, &dictionary, &functions, &contexts);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].actor_record.as_deref(), Some("Bob_Morlock"));
        assert_eq!(events[0].clip_index, Some(1));
        assert_eq!(events[0].background_hnm.as_deref(), Some("gobar1.hnm"));
        assert_eq!(events[0].text, "HELLO WORLD");
    }
}
