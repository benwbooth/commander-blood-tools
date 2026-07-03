use super::*;

/// A decoded `0xA6` TEXT token from a `SCRIPT*.COD` stream.
///
/// Token layout recovered by reverse-engineering the VM TEXT handler at
/// `BLOODPRG.EXE` file 0x660C (see `re/REVERSE.md` "0xA6 TEXT handler"):
///
/// ```text
///   A6  b1 b2  b3  b4  b5   w0 w1 ... wN  0x0000
/// ```
///
/// * `b1:b2` (u16, little-endian) — index into the per-line record table
///   (`gs:0x6724`); kept here as [`call_target`].
/// * `b3` — per-line *selector* the handler stores to `gs:0x1FAB`
///   (→ `gs:0x6788 = b3 + 9`, the active-dialogue-line id). `0xFF` = none.
///   Strongest candidate for the voice/speaker clip selector. Held as
///   `params[0]`.
/// * `b4` — *control-flag word* (NOT a clip index): bit3 `0x08` = conditional
///   skip-count follows, bit4 `0x10` = loop with target word, bits 0/2 tweak
///   parsing. Held as `params[1]`.
/// * `b5` — flags; bit7 `0x80` = the "active / display" flag (always set in real
///   data); this is the marker the decoder anchors on.
/// * `w*` — u16 dictionary-word offsets into `SCRIPT*.DIC`, `0x0000`-terminated.
#[derive(Clone, Debug)]
pub(super) struct ScriptTextCall {
    pub(super) text_end: usize,
    /// `b1:b2` — per-line record index (`gs:0x6724` table).
    pub(super) call_target: u16,
    /// `[b3, b4]` — voice selector and control-flag word (see type docs).
    pub(super) params: Vec<u8>,
    pub(super) words: Vec<String>,
}

/// A dialogue line's background resolved from the *runtime* scene state computed
/// by the bounded interpreter (`vm::interpret_line_states`), keyed by the line's
/// COD offset. Only lines whose runtime location resolves to a real DESCRIPT
/// Location record are included — no fabricated/fallback values.
#[derive(Clone, Default)]
pub(super) struct RuntimeBackground {
    pub(super) record: Option<String>,
    pub(super) hnm: Option<String>,
    pub(super) music: Option<String>,
}

/// Execute the script (bounded interpreter) and resolve each `0xA6` line's
/// runtime location (`state[actor+24]`) to a DESCRIPT background. Returns a map
/// from line COD offset to the resolved background for lines that resolve.
fn resolve_runtime_backgrounds(
    cod: &[u8],
    var: &[u8],
    deb: &[u8],
    descript_db: &DescriptDb,
    hnm_music: &HashMap<String, String>,
) -> HashMap<usize, RuntimeBackground> {
    // object_names: offset -> name for DEB objects (kind 1).
    let mut object_names: HashMap<u16, String> = HashMap::new();
    for record in deb.chunks_exact(20) {
        let name_len = record[..16].iter().position(|&b| b == 0).unwrap_or(16);
        let name = String::from_utf8_lossy(&record[..name_len]).to_string();
        let offset = u16::from_le_bytes([record[16], record[17]]);
        let kind = u16::from_le_bytes([record[18], record[19]]);
        if kind == 1 {
            object_names.insert(offset, name);
        }
    }

    let mut out = HashMap::new();
    for line in vm::interpret_line_states(cod, var) {
        let Some(loc_off) = line.location_offset.filter(|&l| l != 0) else {
            continue;
        };
        let Some(name) = object_names.get(&loc_off) else {
            continue;
        };
        let Some(record) = descript_db.record(name).filter(|r| r.kind == 1) else {
            continue; // not a DESCRIPT Location — don't invent a background
        };
        let hnm = record.full_hnms.first().cloned();
        let music = hnm
            .as_ref()
            .and_then(|h| hnm_music.get(&media_stem(h)).cloned())
            .or_else(|| record.music.first().map(|m| media_stem(m)));
        out.insert(
            line.offset,
            RuntimeBackground {
                record: Some(name.clone()),
                hnm,
                music,
            },
        );
    }
    out
}

pub(super) fn parse_script_speech(
    iso_dir: &Path,
    descript_db: Option<&DescriptDb>,
    hnm_music: &HashMap<String, String>,
) -> Result<Vec<ScriptSpeechLine>, Box<dyn Error>> {
    let mut rows = Vec::new();
    let character_names = descript_db
        .map(|db| db.character_names())
        .unwrap_or_default();

    for script_idx in 1..=5 {
        let cod_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.COD"));
        let dic_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.DIC"));
        let deb_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.DEB"));
        let var_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.VAR"));
        let (Some(cod_path), Some(dic_path)) = (cod_path, dic_path) else {
            continue;
        };

        let cod = fs::read(&cod_path)?;
        let words = parse_script_dictionary(&dic_path)?;
        let script = format!("SCRIPT{script_idx}");
        // Execute the script's state logic to resolve each line's *runtime*
        // location → background, keyed by COD offset (no fallback values).
        let runtime_bg = match (&deb_path, &var_path, descript_db) {
            (Some(d), Some(v), Some(db)) => match (fs::read(d), fs::read(v)) {
                (Ok(deb), Ok(var)) => resolve_runtime_backgrounds(&cod, &var, &deb, db, hnm_music),
                _ => HashMap::new(),
            },
            _ => HashMap::new(),
        };
        let (mut functions, actor_refs, _) =
            if let (Some(deb_path), Some(var_path), Some(db)) = (deb_path, var_path, descript_db) {
                parse_script_symbols(
                    &script,
                    &deb_path,
                    &var_path,
                    db,
                    hnm_music,
                    &character_names,
                )?
            } else {
                (Vec::new(), HashMap::new(), Vec::new())
            };
        if functions.is_empty() {
            functions.push((0, script.as_str().to_string()));
        }
        functions.sort_by_key(|(offset, _)| *offset);
        functions.push((cod.len(), "END".to_string()));

        rows.extend(parse_script_text_calls(
            &script,
            &cod,
            &words,
            &functions,
            &actor_refs,
            &runtime_bg,
        ));
    }

    Ok(rows)
}

pub(super) fn parse_script_disassembly(
    iso_dir: &Path,
    descript_db: Option<&DescriptDb>,
    hnm_music: &HashMap<String, String>,
) -> Result<Vec<ScriptDisassemblyLine>, Box<dyn Error>> {
    let mut rows = Vec::new();
    let character_names = descript_db
        .map(|db| db.character_names())
        .unwrap_or_default();

    for script_idx in 1..=5 {
        let cod_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.COD"));
        let dic_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.DIC"));
        let deb_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.DEB"));
        let var_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.VAR"));
        let (Some(cod_path), Some(dic_path)) = (cod_path, dic_path) else {
            continue;
        };

        let cod = fs::read(cod_path)?;
        let words = parse_script_dictionary(&dic_path)?;
        let script = format!("SCRIPT{script_idx}");
        let (mut functions, actor_refs, _) =
            if let (Some(deb_path), Some(var_path), Some(db)) = (deb_path, var_path, descript_db) {
                parse_script_symbols(
                    &script,
                    &deb_path,
                    &var_path,
                    db,
                    hnm_music,
                    &character_names,
                )?
            } else {
                (Vec::new(), HashMap::new(), Vec::new())
            };
        if functions.is_empty() {
            functions.push((0, script.as_str().to_string()));
        }
        functions.sort_by_key(|(offset, _)| *offset);
        functions.push((cod.len(), "END".to_string()));

        for pair in functions.windows(2) {
            let function_start = pair[0].0;
            let function_name = &pair[0].1;
            let function_end = pair[1].0.min(cod.len());
            rows.extend(disassemble_function(
                &script,
                function_name,
                &cod,
                function_start,
                function_end,
                &words,
                &actor_refs,
            ));
        }
    }

    Ok(rows)
}

pub(super) fn parse_script_branch_trace(
    iso_dir: &Path,
) -> Result<Vec<ScriptBranchTraceLine>, Box<dyn Error>> {
    let mut rows = Vec::new();
    for script_idx in 1..=5 {
        let cod_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.COD"));
        let var_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.VAR"));
        let (Some(cod_path), Some(var_path)) = (cod_path, var_path) else {
            continue;
        };

        let cod = fs::read(cod_path)?;
        let var = fs::read(var_path)?;
        let script = format!("SCRIPT{script_idx}");
        let trace = vm::execute_trace(&cod, &var);
        rows.extend(
            trace
                .branch_events
                .into_iter()
                .enumerate()
                .map(|(event_index, event)| ScriptBranchTraceLine {
                    script: script.clone(),
                    event_index,
                    offset: event.offset,
                    opcode: event.opcode,
                    target: event.target,
                    branch_taken: event.branch_taken,
                    condition_passed: event.condition_passed,
                    stack_depth: event.stack_depth,
                    detail: event.detail.to_string(),
                }),
        );
    }
    Ok(rows)
}

#[derive(Clone, Debug)]
struct TextCallInfo {
    offset: usize,
    line_index: u16,
    voice_selector: u8,
    flags_b4: u8,
    text: String,
    text_end: usize,
}

pub(super) fn parse_script_executed_speech(
    iso_dir: &Path,
    descript_db: Option<&DescriptDb>,
    hnm_music: &HashMap<String, String>,
) -> Result<Vec<ScriptExecutedSpeechLine>, Box<dyn Error>> {
    let mut rows = Vec::new();
    let character_names = descript_db
        .map(|db| db.character_names())
        .unwrap_or_default();

    for script_idx in 1..=5 {
        let cod_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.COD"));
        let dic_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.DIC"));
        let deb_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.DEB"));
        let var_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.VAR"));
        let (Some(cod_path), Some(dic_path), Some(var_path)) = (cod_path, dic_path, var_path)
        else {
            continue;
        };

        let cod = fs::read(&cod_path)?;
        let var = fs::read(&var_path)?;
        let words = parse_script_dictionary(&dic_path)?;
        let script = format!("SCRIPT{script_idx}");
        let (mut functions, actor_refs, object_names) =
            if let (Some(deb_path), Some(db)) = (&deb_path, descript_db) {
                let (functions, actor_refs, _) = parse_script_symbols(
                    &script,
                    deb_path,
                    &var_path,
                    db,
                    hnm_music,
                    &character_names,
                )?;
                let deb = fs::read(deb_path)?;
                (functions, actor_refs, parse_deb_object_names(&deb))
            } else {
                (Vec::new(), HashMap::new(), HashMap::new())
            };
        if functions.is_empty() {
            functions.push((0, script.as_str().to_string()));
        }
        functions.sort_by_key(|(offset, _)| *offset);
        functions.push((cod.len(), "END".to_string()));

        let actor_by_offset: HashMap<u16, ScriptActorRef> = actor_refs
            .values()
            .cloned()
            .map(|actor| {
                (
                    actor.talk_ref.saturating_sub(SCRIPT_OBJECT_TALK_FIELD),
                    actor,
                )
            })
            .collect();
        let text_calls = text_calls_by_offset(&cod, &words);
        let trace = vm::execute_trace(&cod, &var);

        for (sequence_index, state) in trace.line_states.iter().enumerate() {
            let Some(call) = text_calls.get(&state.offset) else {
                continue;
            };
            let actor = state
                .actor_offset
                .and_then(|offset| actor_by_offset.get(&offset).cloned());
            let background = state.location_offset.and_then(|loc| {
                resolve_background_from_location(loc, &object_names, descript_db, hnm_music)
            });
            let actor_speaks = actor.is_some() && call.flags_b4 < 0x10;
            let clip_index = actor.as_ref().and_then(|actor| {
                if !actor_speaks {
                    return None;
                }
                match call.voice_selector {
                    idx if idx > 0 && idx != 0xff && (idx as usize) <= actor.talk_count => {
                        Some(idx as usize - 1)
                    }
                    _ => None,
                }
            });
            let source = match (&actor, actor_speaks, clip_index) {
                (Some(_), true, Some(_)) => {
                    "SCRIPT VM execute_trace + actor state + DESCRIPT talk clip"
                }
                (Some(_), true, None) => {
                    "SCRIPT VM execute_trace + actor state; no mapped talk clip"
                }
                (Some(_), false, _) => {
                    "SCRIPT VM execute_trace + actor state; non-character subtitle channel"
                }
                (None, _, _) => "SCRIPT VM execute_trace; no tracked actor state",
            };

            rows.push(ScriptExecutedSpeechLine {
                script: script.clone(),
                sequence_index,
                function_name: function_name_for_offset(&functions, call.offset).to_string(),
                offset: call.offset,
                actor_record: actor.as_ref().map(|actor| actor.record_name.clone()),
                actor_ref: actor.as_ref().map(|actor| actor.talk_ref),
                location_offset: state.location_offset.filter(|offset| *offset != 0),
                background_record: background
                    .as_ref()
                    .and_then(|background| background.record.clone()),
                background_hnm: background
                    .as_ref()
                    .and_then(|background| background.hnm.clone()),
                background_music: background
                    .as_ref()
                    .and_then(|background| background.music.clone()),
                param0: call.voice_selector,
                param1: call.flags_b4,
                clip_index,
                text: call.text.clone(),
                call_target: call.line_index,
                text_end: call.text_end,
                source: source.to_string(),
            });
        }
    }

    Ok(rows)
}

fn text_calls_by_offset(cod: &[u8], words: &HashMap<u16, String>) -> HashMap<usize, TextCallInfo> {
    let mut calls = HashMap::new();
    for token in vm::walk(cod, 0, cod.len()) {
        let vm::VmToken::Text {
            offset,
            line_index,
            voice_selector,
            flags_b4,
            loop_target,
            word_offsets,
            ..
        } = token
        else {
            continue;
        };
        let Some(decoded_words) = decode_vm_words(words, &word_offsets) else {
            continue;
        };
        let text_end = text_token_end(offset, flags_b4, loop_target, word_offsets.len());
        calls.insert(
            offset,
            TextCallInfo {
                offset,
                line_index,
                voice_selector,
                flags_b4,
                text: assemble_dialogue(&decoded_words),
                text_end,
            },
        );
    }
    calls
}

fn parse_deb_object_names(deb: &[u8]) -> HashMap<u16, String> {
    let mut object_names = HashMap::new();
    for record in deb.chunks_exact(20) {
        let name_len = record[..16].iter().position(|&b| b == 0).unwrap_or(16);
        let name = String::from_utf8_lossy(&record[..name_len]).to_string();
        let offset = u16::from_le_bytes([record[16], record[17]]);
        let kind = u16::from_le_bytes([record[18], record[19]]);
        if kind == 1 {
            object_names.insert(offset, name);
        }
    }
    object_names
}

fn resolve_background_from_location(
    location_offset: u16,
    object_names: &HashMap<u16, String>,
    descript_db: Option<&DescriptDb>,
    hnm_music: &HashMap<String, String>,
) -> Option<RuntimeBackground> {
    let db = descript_db?;
    let name = object_names.get(&location_offset)?;
    let record = db.record(name).filter(|record| record.kind == 1)?;
    let hnm = record.full_hnms.first().cloned();
    let music = hnm
        .as_ref()
        .and_then(|hnm| hnm_music.get(&media_stem(hnm)).cloned())
        .or_else(|| record.music.first().map(|music| media_stem(music)));
    Some(RuntimeBackground {
        record: Some(name.clone()),
        hnm,
        music,
    })
}

pub(super) fn parse_script_text_calls(
    script: &str,
    cod: &[u8],
    words: &HashMap<u16, String>,
    functions: &[(usize, String)],
    actor_refs: &HashMap<u16, ScriptActorRef>,
    runtime_bg: &HashMap<usize, RuntimeBackground>,
) -> Vec<ScriptSpeechLine> {
    let mut rows = Vec::new();
    let mut current_actor: Option<ScriptActorRef> = None;

    for token in vm::walk(cod, 0, cod.len()) {
        match token {
            vm::VmToken::Actor { operand, .. } => {
                current_actor = actor_refs.get(&operand).cloned();
            }
            vm::VmToken::Text {
                offset,
                line_index,
                voice_selector,
                flags_b4,
                loop_target,
                word_offsets,
                ..
            } => {
                let Some(decoded_words) = decode_vm_words(words, &word_offsets) else {
                    continue;
                };
                let function_name = function_name_for_offset(functions, offset);

                let param0 = Some(voice_selector); // = 0xA6 token b3 (voice selector)
                let param1 = Some(flags_b4); // = 0xA6 token b4 (control flags)
                let rt = runtime_bg.get(&offset); // runtime scene for this line, if resolved
                let actor = current_actor.clone();
                // Voice clip-index (RE, re/REVERSE.md "voice clip-index", confirmed by
                // tracing gs:0x6788 = b3 + 9 into the son.snd player + the export-data
                // distribution): `param0` (b3) is the per-line voice selector —
                //   * b3 == 0xFF or 0x00 => NO voice (narrator/menu/tutorial subtitle;
                //     b3+9 = 0x108 is the out-of-range "none" line id), and
                //   * b3 in 1..=N => 1-based index into the actor's son.snd talk clips,
                //     so clip = b3 - 1.
                // `param1` (b4) is the control-flag word (bit3=skip, bit4=loop) — NOT a
                // clip index. The earlier `(0xFF, b4) => clip = b4` branch misread the
                // flag word as an index, spuriously voicing ~26% of lines (every
                // b3==0xFF narrator line); removed. `param1 < 0x10` (no loop/skip bits)
                // still gates whether the line is shown/spoken.
                let actor_speaks = actor.is_some() && flags_b4 < 0x10;
                let clip_index = actor.as_ref().and_then(|actor| {
                    if !actor_speaks {
                        return None;
                    }
                    match voice_selector {
                        idx if idx > 0 && idx != 0xff && (idx as usize) <= actor.talk_count => {
                            Some(idx as usize - 1)
                        }
                        _ => None,
                    }
                });
                let source = match (&actor, actor_speaks, clip_index) {
                    (Some(_), true, Some(_)) => {
                        "SCRIPT VM token + tracked actor ref + DESCRIPT talk clip".to_string()
                    }
                    (Some(_), true, None) => {
                        "SCRIPT VM token + tracked actor ref; no mapped talk clip".to_string()
                    }
                    (Some(_), false, _) => {
                        "SCRIPT VM token + tracked actor ref; non-character subtitle channel"
                            .to_string()
                    }
                    (None, _, _) => "SCRIPT VM token; no tracked actor ref".to_string(),
                };
                let params = [voice_selector, flags_b4];

                rows.push(ScriptSpeechLine {
                    script: script.to_string(),
                    function_name: function_name.to_string(),
                    offset,
                    actor_record: actor.as_ref().map(|actor| actor.record_name.clone()),
                    param0,
                    param1,
                    clip_index,
                    // Prefer the runtime location computed by executing the script; fall
                    // back to the actor's initial location only when the interpreter did
                    // not resolve a real DESCRIPT location for this line. Both are
                    // computed from data (no hardcoded character table).
                    background_record: rt
                        .and_then(|b| b.record.clone())
                        .or_else(|| actor.as_ref().and_then(|a| a.background_record.clone())),
                    background_hnm: rt
                        .and_then(|b| b.hnm.clone())
                        .or_else(|| actor.as_ref().and_then(|a| a.background_hnm.clone())),
                    background_music: rt
                        .and_then(|b| b.music.clone())
                        .or_else(|| actor.as_ref().and_then(|a| a.background_music.clone())),
                    source,
                    text: assemble_dialogue(&decoded_words),
                    call_target: line_index,
                    params_hex: hex_bytes(&params),
                    text_end: text_token_end(offset, flags_b4, loop_target, word_offsets.len()),
                    actor_ref: actor.as_ref().map(|actor| actor.talk_ref),
                    actor_proof: actor
                        .as_ref()
                        .map(|actor| format!("tracked 0xc4 actor ref 0x{:04x}", actor.talk_ref))
                        .unwrap_or_default(),
                    word_count: decoded_words.len(),
                });
            }
            vm::VmToken::Invalid { .. } => break,
            _ => {}
        }
    }

    rows
}

fn function_name_for_offset(functions: &[(usize, String)], offset: usize) -> &str {
    functions
        .iter()
        .rev()
        .find(|(function_offset, _)| *function_offset <= offset)
        .map(|(_, name)| name.as_str())
        .unwrap_or("")
}

fn decode_vm_words(words: &HashMap<u16, String>, word_offsets: &[u16]) -> Option<Vec<String>> {
    let decoded: Vec<String> = word_offsets
        .iter()
        .map(|offset| words.get(offset).cloned())
        .collect::<Option<_>>()?;
    if decoded.is_empty() {
        None
    } else {
        Some(decoded)
    }
}

fn text_token_end(
    offset: usize,
    flags_b4: u8,
    loop_target: Option<u16>,
    word_count: usize,
) -> usize {
    let loop_len = if flags_b4 & 0x10 != 0 || loop_target.is_some() {
        2
    } else {
        0
    };
    offset + 6 + loop_len + word_count * 2 + 2
}

pub(super) fn disassemble_function(
    script: &str,
    function_name: &str,
    cod: &[u8],
    function_start: usize,
    function_end: usize,
    words: &HashMap<u16, String>,
    actor_refs: &HashMap<u16, ScriptActorRef>,
) -> Vec<ScriptDisassemblyLine> {
    let mut rows = Vec::new();
    if function_start >= function_end || function_start >= cod.len() {
        return rows;
    }

    let mut current_actor: Option<ScriptActorRef> = None;
    let mut raw_start: Option<usize> = None;
    let mut pos = function_start;
    while pos < function_end {
        if pos + 4 < function_end && cod[pos] == 0xc4 {
            push_raw_disassembly(script, function_name, cod, &mut rows, raw_start.take(), pos);
            let addr = u16::from_le_bytes([cod[pos + 1], cod[pos + 2]]);
            let extra = u16::from_le_bytes([cod[pos + 3], cod[pos + 4]]);
            current_actor = actor_refs.get(&addr).cloned();
            rows.push(ScriptDisassemblyLine {
                script: script.to_string(),
                function_name: function_name.to_string(),
                offset: pos,
                len: 5,
                opcode: "c4".to_string(),
                mnemonic: "actor_ref".to_string(),
                operands: format!("ref=0x{addr:04x} extra=0x{extra:04x}"),
                actor_record: current_actor
                    .as_ref()
                    .map(|actor| actor.record_name.clone()),
                text: None,
            });
            pos += 5;
            continue;
        }

        if let Some(call) = decode_text_call_at(cod, function_end, words, pos) {
            push_raw_disassembly(script, function_name, cod, &mut rows, raw_start.take(), pos);
            rows.push(ScriptDisassemblyLine {
                script: script.to_string(),
                function_name: function_name.to_string(),
                offset: pos,
                len: call.text_end - pos,
                opcode: "a6".to_string(),
                mnemonic: "text_call".to_string(),
                operands: format!(
                    "target=0x{:04x} params={} words={}",
                    call.call_target,
                    hex_bytes(&call.params),
                    call.words.len()
                ),
                actor_record: current_actor
                    .as_ref()
                    .map(|actor| actor.record_name.clone()),
                text: Some(assemble_dialogue(&call.words)),
            });
            pos = call.text_end;
            continue;
        }

        if raw_start.is_none() {
            raw_start = Some(pos);
        }
        pos += 1;
        if raw_start.is_some_and(|start| pos - start >= 32) {
            push_raw_disassembly(script, function_name, cod, &mut rows, raw_start.take(), pos);
        }
    }
    push_raw_disassembly(
        script,
        function_name,
        cod,
        &mut rows,
        raw_start.take(),
        function_end,
    );

    rows
}

pub(super) fn push_raw_disassembly(
    script: &str,
    function_name: &str,
    cod: &[u8],
    rows: &mut Vec<ScriptDisassemblyLine>,
    start: Option<usize>,
    end: usize,
) {
    let Some(start) = start else {
        return;
    };
    if start >= end || start >= cod.len() {
        return;
    }
    let end = end.min(cod.len());
    rows.push(ScriptDisassemblyLine {
        script: script.to_string(),
        function_name: function_name.to_string(),
        offset: start,
        len: end - start,
        opcode: "raw".to_string(),
        mnemonic: "raw".to_string(),
        operands: hex_bytes(&cod[start..end]),
        actor_record: None,
        text: None,
    });
}

/// Assemble a dialogue line's words into the on-screen string exactly as the
/// game's 0xA6 handler does (BLOODPRG.EXE 0x66CD–0x6739, see re/REVERSE.md):
/// a space between words, except no space before a word that starts with
/// `, . ? ! :`; and a line break once the current line reaches 0x23 (35) chars
/// (wrap only happens on the space path; long single words are not split).
pub(super) fn assemble_dialogue(words: &[String]) -> String {
    let parts: Vec<&String> = words.iter().filter(|w| !w.is_empty()).collect();
    let mut out = String::new();
    let mut line_len: usize = 0;
    for (i, w) in parts.iter().enumerate() {
        out.push_str(w);
        line_len += w.chars().count();
        if i + 1 < parts.len() {
            let attaches = matches!(
                parts[i + 1].chars().next(),
                Some(',' | '.' | '?' | '!' | ':')
            );
            if !attaches {
                out.push(' ');
                line_len += 1;
                if line_len >= 0x23 {
                    out.push('\n');
                    line_len = 0;
                }
            }
        }
    }
    out
}

pub(super) fn decode_text_call_at(
    cod: &[u8],
    function_end: usize,
    words: &HashMap<u16, String>,
    pos: usize,
) -> Option<ScriptTextCall> {
    if pos + 4 >= function_end || cod.get(pos).copied()? != 0xa6 {
        return None;
    }

    // Fixed token layout recovered from the VM TEXT handler (BLOODPRG.EXE
    // 0x660C): `A6 b1 b2 b3 b4 b5 [loop:u16?] w0 w1 ... 0x0000`.
    // * b1:b2 = line-record index (call_target)
    // * b3 = params[0] (voice selector), b4 = params[1] (control flags)
    // * b5 (pos+5) bit7 = active/display flag (may be 0x80/0x90/0xA0/...)
    // * if b4 & 0x10 (loop), a u16 loop target precedes the word list.
    if pos + 6 > function_end {
        return None;
    }
    let call_target = u16::from_le_bytes([cod[pos + 1], cod[pos + 2]]);
    let b4 = cod[pos + 4];
    let b5 = cod[pos + 5];
    // Require the active/display flag (bit7). Previously this matched only the
    // exact byte 0x80, which dropped lines whose b5 carried extra flag bits.
    if b5 & 0x80 == 0 {
        return None;
    }
    let marker = pos + 5;
    // Skip the loop-target word when the loop bit is set, so it is not mistaken
    // for a dictionary-word offset (which dropped looped lines entirely before).
    let mut text_pos = marker + 1;
    if b4 & 0x10 != 0 {
        text_pos += 2;
    }
    let mut decoded_words = Vec::new();
    let mut found_end = false;

    while text_pos + 1 < function_end {
        let word_off = u16::from_le_bytes([cod[text_pos], cod[text_pos + 1]]);
        text_pos += 2;
        if word_off == 0 {
            found_end = true;
            break;
        }
        let word = words.get(&word_off)?;
        decoded_words.push(word.clone());
        if decoded_words.len() > 256 {
            return None;
        }
    }

    if !found_end || decoded_words.is_empty() || text_pos > function_end {
        return None;
    }

    Some(ScriptTextCall {
        text_end: text_pos,
        call_target,
        params: cod[pos + 3..marker].to_vec(),
        words: decoded_words,
    })
}

#[cfg(test)]
mod assemble_tests {
    use super::*;

    fn w(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn no_space_before_punctuation() {
        // game rule: no space before , . ? ! :
        assert_eq!(assemble_dialogue(&w(&["Oh", "no", "!"])), "Oh no!");
        assert_eq!(
            assemble_dialogue(&w(&["Commander", ",", "I"])),
            "Commander, I"
        );
        assert_eq!(assemble_dialogue(&w(&["you", ":"])), "you:");
        // ';' is NOT in the game's set -> keeps a space
        assert_eq!(assemble_dialogue(&w(&["a", ";", "b"])), "a ; b");
    }

    #[test]
    fn wraps_at_35_chars() {
        // 8x "wordword" (8 chars) + spaces: line breaks once length reaches 0x23.
        let out = assemble_dialogue(&w(&["abcdefgh"; 8]));
        assert!(out.contains('\n'), "should wrap long lines: {out:?}");
        for line in out.split('\n') {
            assert!(line.chars().count() <= 40, "line not over-long: {line:?}");
        }
    }
}

#[cfg(test)]
mod decode_text_tests {
    use super::*;

    fn words_fixture() -> HashMap<u16, String> {
        let mut w = HashMap::new();
        w.insert(0x000C, "hello".to_string());
        w.insert(0x0010, "world".to_string());
        w.insert(0x0020, "loop".to_string());
        w
    }

    /// Plain TEXT token with b5 == 0x80.
    #[test]
    fn decodes_plain_token() {
        let words = words_fixture();
        // A6 b1 b2 b3 b4 b5  w0   w1   term
        let cod = [
            0xA6, 0x02, 0x01, 0x05, 0x00, 0x80, 0x0C, 0x00, 0x10, 0x00, 0x00, 0x00,
        ];
        let call = decode_text_call_at(&cod, cod.len(), &words, 0).expect("should decode");
        assert_eq!(call.call_target, 0x0102);
        assert_eq!(call.params, vec![0x05, 0x00]); // b3, b4
        assert_eq!(call.words, vec!["hello", "world"]);
    }

    /// b5 carries extra flag bits (0xA0): bit7 still set → must decode. The old
    /// `== 0x80` check dropped this line.
    #[test]
    fn decodes_token_with_extra_b5_flags() {
        let words = words_fixture();
        let cod = [0xA6, 0x00, 0x00, 0xFF, 0x08, 0xA0, 0x0C, 0x00, 0x00, 0x00];
        let call = decode_text_call_at(&cod, cod.len(), &words, 0).expect("0xA0 b5 should decode");
        assert_eq!(call.words, vec!["hello"]);
    }

    /// Loop token (b4 & 0x10): a u16 loop target precedes the word list and must
    /// be skipped. The old decoder read it as a (bogus) dict offset and dropped
    /// the whole line.
    #[test]
    fn decodes_loop_token_skipping_loop_target() {
        let words = words_fixture();
        // loop target 0x1234 is NOT a valid dict offset; old code returned None.
        let cod = [
            0xA6, 0x00, 0x00, 0xFF, 0x10, 0x80, 0x34, 0x12, 0x20, 0x00, 0x00, 0x00,
        ];
        let call =
            decode_text_call_at(&cod, cod.len(), &words, 0).expect("loop token should decode");
        assert_eq!(call.params, vec![0xFF, 0x10]); // b3=0xFF (no voice), b4=0x10 (loop)
        assert_eq!(call.words, vec!["loop"]);
    }
}

pub(super) fn parse_script_character_contexts(
    iso_dir: &Path,
    descript_db: &DescriptDb,
    hnm_music: &HashMap<String, String>,
) -> Result<Vec<ScriptCharacterContextLine>, Box<dyn Error>> {
    let character_names = descript_db.character_names();
    let mut rows = Vec::new();

    for script_idx in 1..=5 {
        let script = format!("SCRIPT{script_idx}");
        let deb_path = find_file_recursive(iso_dir, &format!("{script}.DEB"));
        let var_path = find_file_recursive(iso_dir, &format!("{script}.VAR"));
        let (Some(deb_path), Some(var_path)) = (deb_path, var_path) else {
            continue;
        };
        let (_, _, contexts) = parse_script_symbols(
            &script,
            &deb_path,
            &var_path,
            descript_db,
            hnm_music,
            &character_names,
        )?;
        rows.extend(contexts);
    }

    rows.sort_by(|a, b| {
        a.actor_record
            .to_ascii_lowercase()
            .cmp(&b.actor_record.to_ascii_lowercase())
            .then(a.script.cmp(&b.script))
            .then(a.actor_object_offset.cmp(&b.actor_object_offset))
    });
    Ok(rows)
}

pub(super) fn parse_script_symbols(
    script: &str,
    deb_path: &Path,
    var_path: &Path,
    descript_db: &DescriptDb,
    hnm_music: &HashMap<String, String>,
    character_names: &[String],
) -> Result<
    (
        Vec<(usize, String)>,
        HashMap<u16, ScriptActorRef>,
        Vec<ScriptCharacterContextLine>,
    ),
    Box<dyn Error>,
> {
    let deb = fs::read(deb_path)?;
    let var = fs::read(var_path)?;
    let mut object_names: HashMap<u16, String> = HashMap::new();
    let mut functions = Vec::new();

    for record in deb.chunks_exact(20) {
        let name_len = record[..16].iter().position(|&b| b == 0).unwrap_or(16);
        let name = String::from_utf8_lossy(&record[..name_len]).to_string();
        let offset = u16::from_le_bytes([record[16], record[17]]);
        let kind = u16::from_le_bytes([record[18], record[19]]);
        match kind {
            1 => {
                object_names.insert(offset, name);
            }
            2 if offset != 0xffff => functions.push((offset as usize, name)),
            _ => {}
        }
    }

    let mut actor_refs = HashMap::new();
    let mut contexts = Vec::new();
    for (&offset, name) in &object_names {
        if !character_names
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(name))
        {
            continue;
        }

        let Some(record) = descript_db.record(name) else {
            continue;
        };
        let var_offset = offset as usize;
        let location_offset = var
            .get(
                var_offset + SCRIPT_OBJECT_LOCATION_FIELD
                    ..var_offset + SCRIPT_OBJECT_LOCATION_FIELD + 2,
            )
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]));
        let background_record = location_offset.and_then(|loc| object_names.get(&loc).cloned());
        let background = background_record
            .as_ref()
            .and_then(|loc_name| descript_db.record(loc_name))
            .filter(|record| record.kind == 1);
        let background_hnm = background.and_then(|record| record.full_hnms.first().cloned());
        let background_music = background_hnm
            .as_ref()
            .and_then(|hnm| hnm_music.get(&media_stem(hnm)).cloned())
            .or_else(|| {
                background
                    .and_then(|record| record.music.first())
                    .map(|music| media_stem(music))
            });

        let actor_talk_ref = offset.saturating_add(SCRIPT_OBJECT_TALK_FIELD);
        contexts.push(ScriptCharacterContextLine {
            script: script.to_string(),
            actor_record: record.name.clone(),
            actor_object_offset: offset,
            actor_talk_ref,
            background_record: background_record.clone(),
            background_hnm: background_hnm.clone(),
            background_music: background_music.clone(),
            source: format!("{script}.DEB object + {script}.VAR object location field"),
        });
        actor_refs.insert(
            actor_talk_ref,
            ScriptActorRef {
                talk_ref: actor_talk_ref,
                record_name: record.name.clone(),
                background_record,
                background_hnm,
                background_music,
                talk_count: record.talk_hnms.len(),
            },
        );
    }

    contexts.sort_by(|a, b| {
        a.actor_record
            .to_ascii_lowercase()
            .cmp(&b.actor_record.to_ascii_lowercase())
            .then(a.actor_object_offset.cmp(&b.actor_object_offset))
    });

    Ok((functions, actor_refs, contexts))
}

pub(super) fn write_script_speech_manifest(
    rows: &[ScriptSpeechLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "script\tfunction\toffset\tactor\tparam0\tparam1\tclip_index\tbackground_record\tbackground_hnm\tbackground_music\tsource\ttext\tcall_target\tparams_hex\ttext_end\tactor_ref\tactor_proof\tword_count"
    )?;
    for row in rows {
        writeln!(
            file,
            "{}\t{}\t0x{:05x}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t0x{:04x}\t{}\t0x{:05x}\t{}\t{}\t{}",
            row.script,
            row.function_name,
            row.offset,
            row.actor_record.as_deref().unwrap_or(""),
            row.param0
                .map(|param| format!("{param:02x}"))
                .unwrap_or_default(),
            row.param1
                .map(|param| format!("{param:02x}"))
                .unwrap_or_default(),
            row.clip_index
                .map(|idx| idx.to_string())
                .unwrap_or_default(),
            row.background_record.as_deref().unwrap_or(""),
            row.background_hnm.as_deref().unwrap_or(""),
            row.background_music.as_deref().unwrap_or(""),
            row.source,
            clean_tsv(&row.text),
            row.call_target,
            row.params_hex,
            row.text_end,
            row.actor_ref
                .map(|actor_ref| format!("0x{actor_ref:04x}"))
                .unwrap_or_default(),
            row.actor_proof,
            row.word_count
        )?;
    }
    Ok(())
}

pub(super) fn write_script_executed_speech_manifest(
    rows: &[ScriptExecutedSpeechLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "script\tsequence_index\tfunction\toffset\tactor\tactor_ref\tlocation_offset\tbackground_record\tbackground_hnm\tbackground_music\tparam0\tparam1\tclip_index\tcall_target\ttext_end\tsource\ttext"
    )?;
    for row in rows {
        writeln!(
            file,
            "{}\t{}\t{}\t0x{:05x}\t{}\t{}\t{}\t{}\t{}\t{}\t{:02x}\t{:02x}\t{}\t0x{:04x}\t0x{:05x}\t{}\t{}",
            row.script,
            row.sequence_index,
            row.function_name,
            row.offset,
            row.actor_record.as_deref().unwrap_or(""),
            row.actor_ref
                .map(|actor_ref| format!("0x{actor_ref:04x}"))
                .unwrap_or_default(),
            row.location_offset
                .map(|location_offset| format!("0x{location_offset:04x}"))
                .unwrap_or_default(),
            row.background_record.as_deref().unwrap_or(""),
            row.background_hnm.as_deref().unwrap_or(""),
            row.background_music.as_deref().unwrap_or(""),
            row.param0,
            row.param1,
            row.clip_index
                .map(|idx| idx.to_string())
                .unwrap_or_default(),
            row.call_target,
            row.text_end,
            clean_tsv(&row.source),
            clean_tsv(&row.text),
        )?;
    }
    Ok(())
}

pub(super) fn write_script_disassembly_manifest(
    rows: &[ScriptDisassemblyLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "script\tfunction\toffset\tlen\topcode\tmnemonic\toperands\tactor\ttext"
    )?;
    for row in rows {
        writeln!(
            file,
            "{}\t{}\t0x{:05x}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.script,
            row.function_name,
            row.offset,
            row.len,
            row.opcode,
            row.mnemonic,
            clean_tsv(&row.operands),
            row.actor_record.as_deref().unwrap_or(""),
            row.text.as_deref().map(clean_tsv).unwrap_or_default()
        )?;
    }
    Ok(())
}

pub(super) fn write_script_branch_trace_manifest(
    rows: &[ScriptBranchTraceLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "script\tevent_index\toffset\topcode\ttarget\tbranch_taken\tcondition_passed\tstack_depth\tdetail"
    )?;
    for row in rows {
        writeln!(
            file,
            "{}\t{}\t0x{:05x}\t{:02x}\t{}\t{}\t{}\t{}\t{}",
            row.script,
            row.event_index,
            row.offset,
            row.opcode,
            row.target
                .map(|target| format!("0x{target:04x}"))
                .unwrap_or_default(),
            row.branch_taken,
            row.condition_passed
                .map(|passed| passed.to_string())
                .unwrap_or_default(),
            row.stack_depth,
            clean_tsv(&row.detail),
        )?;
    }
    Ok(())
}

#[derive(Debug)]
struct ScriptDialogueRun<'a> {
    script: String,
    run_index: usize,
    first_offset: usize,
    last_offset: usize,
    background_record: Option<String>,
    background_hnm: Option<String>,
    background_music: Option<String>,
    lines: Vec<&'a ScriptSpeechLine>,
}

#[derive(Debug)]
pub(super) struct ScriptExecutedDialogueRun<'a> {
    pub(super) script: String,
    pub(super) run_index: usize,
    pub(super) first_sequence: usize,
    pub(super) last_sequence: usize,
    pub(super) first_offset: usize,
    pub(super) last_offset: usize,
    pub(super) background_record: Option<String>,
    pub(super) background_hnm: Option<String>,
    pub(super) background_music: Option<String>,
    pub(super) lines: Vec<&'a ScriptExecutedSpeechLine>,
}

fn script_dialogue_runs(rows: &[ScriptSpeechLine]) -> Vec<ScriptDialogueRun<'_>> {
    let mut ordered: Vec<&ScriptSpeechLine> = rows
        .iter()
        .filter(|row| row.clip_index.is_some() || !row.text.trim().is_empty())
        .collect();
    ordered.sort_by(|a, b| (a.script.as_str(), a.offset).cmp(&(b.script.as_str(), b.offset)));

    let mut runs: Vec<ScriptDialogueRun<'_>> = Vec::new();
    for row in ordered {
        let same_run = runs.last().is_some_and(|run| {
            run.script == row.script
                && run.background_record == row.background_record
                && run.background_hnm == row.background_hnm
                && run.background_music == row.background_music
        });
        if same_run {
            let run = runs.last_mut().expect("run exists");
            run.last_offset = row.offset;
            run.lines.push(row);
            continue;
        }

        let run_index = runs.iter().filter(|run| run.script == row.script).count() + 1;
        runs.push(ScriptDialogueRun {
            script: row.script.clone(),
            run_index,
            first_offset: row.offset,
            last_offset: row.offset,
            background_record: row.background_record.clone(),
            background_hnm: row.background_hnm.clone(),
            background_music: row.background_music.clone(),
            lines: vec![row],
        });
    }
    runs
}

pub(super) fn script_executed_dialogue_runs(
    rows: &[ScriptExecutedSpeechLine],
) -> Vec<ScriptExecutedDialogueRun<'_>> {
    let mut ordered: Vec<&ScriptExecutedSpeechLine> = rows
        .iter()
        .filter(|row| row.clip_index.is_some() || !row.text.trim().is_empty())
        .collect();
    ordered.sort_by(|a, b| {
        (a.script.as_str(), a.sequence_index).cmp(&(b.script.as_str(), b.sequence_index))
    });

    let mut runs: Vec<ScriptExecutedDialogueRun<'_>> = Vec::new();
    for row in ordered {
        let same_run = runs.last().is_some_and(|run| {
            run.script == row.script
                && run.background_record == row.background_record
                && run.background_hnm == row.background_hnm
                && run.background_music == row.background_music
        });
        if same_run {
            let run = runs.last_mut().expect("run exists");
            run.last_sequence = row.sequence_index;
            run.last_offset = row.offset;
            run.lines.push(row);
            continue;
        }

        let run_index = runs.iter().filter(|run| run.script == row.script).count() + 1;
        runs.push(ScriptExecutedDialogueRun {
            script: row.script.clone(),
            run_index,
            first_sequence: row.sequence_index,
            last_sequence: row.sequence_index,
            first_offset: row.offset,
            last_offset: row.offset,
            background_record: row.background_record.clone(),
            background_hnm: row.background_hnm.clone(),
            background_music: row.background_music.clone(),
            lines: vec![row],
        });
    }
    runs
}

pub(super) fn write_script_executed_dialogue_runs_manifest(
    rows: &[ScriptExecutedSpeechLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let runs = script_executed_dialogue_runs(rows);
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "run_id\tmp4\tscript\tfirst_sequence\tlast_sequence\tfirst_offset\tlast_offset\tbackground_record\tbackground_hnm\tbackground_music\tline_count\tvoiced_count\tactors\tclip_refs\tfirst_text"
    )?;
    for run in runs {
        let run_id = format!("{}-{:04}", run.script, run.run_index);
        let location = run
            .background_record
            .as_deref()
            .or(run.background_hnm.as_deref())
            .unwrap_or("nolocation");
        let output_stem = format!(
            "executed-dialogue-run - {} - {:04} - {}",
            safe_file_stem(&run.script),
            run.run_index,
            safe_file_stem(location)
        );
        let actors = unique_join(
            run.lines
                .iter()
                .filter_map(|line| line.actor_record.as_deref()),
        );
        let clip_refs = run
            .lines
            .iter()
            .filter_map(|line| {
                line.clip_index.map(|clip| {
                    format!(
                        "{}:{clip}",
                        line.actor_record.as_deref().unwrap_or("noactor")
                    )
                })
            })
            .collect::<Vec<_>>()
            .join(",");
        let voiced_count = run
            .lines
            .iter()
            .filter(|line| line.clip_index.is_some())
            .count();
        let first_text = run
            .lines
            .first()
            .map(|line| clean_tsv(&line.text))
            .unwrap_or_default();
        writeln!(
            file,
            "{}\t{}.mp4\t{}\t{}\t{}\t0x{:05x}\t0x{:05x}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            run_id,
            output_stem,
            run.script,
            run.first_sequence,
            run.last_sequence,
            run.first_offset,
            run.last_offset,
            run.background_record.as_deref().unwrap_or(""),
            run.background_hnm.as_deref().unwrap_or(""),
            run.background_music.as_deref().unwrap_or(""),
            run.lines.len(),
            voiced_count,
            actors,
            clip_refs,
            first_text
        )?;
    }
    Ok(())
}

pub(super) fn write_script_dialogue_runs_manifest(
    rows: &[ScriptSpeechLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let runs = script_dialogue_runs(rows);
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "run_id\tmp4\tscript\tfirst_offset\tlast_offset\tbackground_record\tbackground_hnm\tbackground_music\tline_count\tvoiced_count\tactors\tclip_refs\tfirst_text"
    )?;
    for run in runs {
        let run_id = format!("{}-{:04}", run.script, run.run_index);
        let location = run
            .background_record
            .as_deref()
            .or(run.background_hnm.as_deref())
            .unwrap_or("nolocation");
        let output_stem = format!(
            "dialogue-run - {} - {:04} - {}",
            safe_file_stem(&run.script),
            run.run_index,
            safe_file_stem(location)
        );
        let actors = unique_join(
            run.lines
                .iter()
                .filter_map(|line| line.actor_record.as_deref()),
        );
        let clip_refs = run
            .lines
            .iter()
            .filter_map(|line| {
                line.clip_index.map(|clip| {
                    format!(
                        "{}:{clip}",
                        line.actor_record.as_deref().unwrap_or("noactor")
                    )
                })
            })
            .collect::<Vec<_>>()
            .join(",");
        let voiced_count = run
            .lines
            .iter()
            .filter(|line| line.clip_index.is_some())
            .count();
        let first_text = run
            .lines
            .first()
            .map(|line| clean_tsv(&line.text))
            .unwrap_or_default();
        writeln!(
            file,
            "{}\t{}.mp4\t{}\t0x{:05x}\t0x{:05x}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            run_id,
            output_stem,
            run.script,
            run.first_offset,
            run.last_offset,
            run.background_record.as_deref().unwrap_or(""),
            run.background_hnm.as_deref().unwrap_or(""),
            run.background_music.as_deref().unwrap_or(""),
            run.lines.len(),
            voiced_count,
            actors,
            clip_refs,
            first_text
        )?;
    }
    Ok(())
}

fn unique_join<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let mut out: Vec<&'a str> = Vec::new();
    for value in values {
        if !out.iter().any(|seen| seen.eq_ignore_ascii_case(value)) {
            out.push(value);
        }
    }
    out.join(",")
}

pub(super) fn write_script_dialogue_manifest(
    rows: &[ScriptExecutedSpeechLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut groups: BTreeMap<(String, String, String), Vec<&ScriptExecutedSpeechLine>> =
        BTreeMap::new();
    for row in rows {
        let Some(actor) = row.actor_record.as_ref() else {
            continue;
        };
        if row.clip_index.is_none() && row.text.trim().is_empty() {
            continue;
        }
        // Group by (script, location, actor) to match the combined per-location
        // videos produced by create_character_dialogue_videos_from_scene.
        let location = row
            .background_record
            .clone()
            .unwrap_or_else(|| "nolocation".to_string());
        groups
            .entry((row.script.clone(), location, actor.clone()))
            .or_default()
            .push(row);
    }

    // Order the dialogue composites by their position in the dialog tree, i.e.
    // the branch-aware script execution order, rather than alphabetically by
    // location/function name or by raw COD offset.
    let mut ordered: Vec<((String, String, String), Vec<&ScriptExecutedSpeechLine>)> =
        groups.into_iter().collect();
    for (_, lines) in ordered.iter_mut() {
        lines.sort_by_key(|line| line.sequence_index);
    }
    // Dialog trees are per-character, so keep each character's nodes together
    // (script, then actor), ordered within by executed sequence position.
    ordered.sort_by(|a, b| {
        let oa = a.1.first().map(|l| l.sequence_index).unwrap_or(usize::MAX);
        let ob = b.1.first().map(|l| l.sequence_index).unwrap_or(usize::MAX);
        (a.0 .0.as_str(), a.0 .2.as_str(), oa).cmp(&(b.0 .0.as_str(), b.0 .2.as_str(), ob))
    });

    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "mp4\tscript\tfunction\tactor\tbackground_record\tbackground_hnm\tbackground_music\tline_count\tclip_indices"
    )?;
    for ((script, function_name, actor), lines) in ordered {
        let output_stem = format!(
            "dialogue - {} - {} - {}",
            safe_file_stem(&script),
            safe_file_stem(&function_name),
            safe_file_stem(&actor)
        );
        let clip_indices = lines
            .iter()
            .filter_map(|line| line.clip_index.map(|idx| idx.to_string()))
            .collect::<Vec<_>>()
            .join(",");
        let first = lines[0];
        writeln!(
            file,
            "{}.mp4\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            output_stem,
            script,
            function_name,
            actor,
            first.background_record.as_deref().unwrap_or(""),
            first.background_hnm.as_deref().unwrap_or(""),
            first.background_music.as_deref().unwrap_or(""),
            lines.len(),
            clip_indices
        )?;
    }

    Ok(())
}

pub(super) fn parse_script_dictionary(path: &Path) -> Result<HashMap<u16, String>, Box<dyn Error>> {
    let data = fs::read(path)?;
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
    Ok(words)
}

pub(super) fn hex_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn clean_tsv(text: &str) -> String {
    text.replace(['\t', '\r', '\n'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn speech_line(
        script: &str,
        offset: usize,
        actor: Option<&str>,
        location: Option<&str>,
        text: &str,
    ) -> ScriptSpeechLine {
        ScriptSpeechLine {
            script: script.to_string(),
            function_name: "func".to_string(),
            offset,
            actor_record: actor.map(str::to_string),
            param0: Some(1),
            param1: Some(0),
            clip_index: actor.map(|_| 0),
            background_record: location.map(str::to_string),
            background_hnm: location.map(|loc| format!("{loc}.hnm")),
            background_music: location.map(|loc| format!("{loc}_music")),
            source: "test".to_string(),
            text: text.to_string(),
            call_target: 0x1234,
            params_hex: "01 00".to_string(),
            text_end: offset + 12,
            actor_ref: Some(0x003a),
            actor_proof: "test".to_string(),
            word_count: 1,
        }
    }

    fn executed_speech_line(
        script: &str,
        sequence_index: usize,
        offset: usize,
        actor: Option<&str>,
        location: Option<&str>,
        text: &str,
    ) -> ScriptExecutedSpeechLine {
        ScriptExecutedSpeechLine {
            script: script.to_string(),
            sequence_index,
            function_name: "func".to_string(),
            offset,
            actor_record: actor.map(str::to_string),
            actor_ref: actor.map(|_| 0x003a),
            location_offset: location.map(|_| 0x1000),
            background_record: location.map(str::to_string),
            background_hnm: location.map(|loc| format!("{loc}.hnm")),
            background_music: location.map(|loc| format!("{loc}_music")),
            param0: 1,
            param1: 0,
            clip_index: actor.map(|_| 0),
            text: text.to_string(),
            call_target: 0x1234,
            text_end: offset + 12,
            source: "test".to_string(),
        }
    }

    #[test]
    fn decodes_general_a6_text_call_shape() {
        let mut words = HashMap::new();
        words.insert(0x0001, "hello".to_string());
        words.insert(0x0008, "commander".to_string());
        let cod = [
            0xa6, 0x34, 0x12, 0xff, 0x02, 0x80, 0x01, 0x00, 0x08, 0x00, 0x00, 0x00,
        ];

        let call = decode_text_call_at(&cod, cod.len(), &words, 0).expect("text call");
        assert_eq!(call.call_target, 0x1234);
        assert_eq!(call.params, vec![0xff, 0x02]);
        assert_eq!(call.words, vec!["hello", "commander"]);
        assert_eq!(call.text_end, cod.len());
    }

    #[test]
    fn tracks_actor_ref_into_text_call_clip_mapping() {
        let mut words = HashMap::new();
        words.insert(0x0001, "hello".to_string());
        let actor = ScriptActorRef {
            talk_ref: 0x003a,
            record_name: "Test_Actor".to_string(),
            background_record: Some("Test_Room".to_string()),
            background_hnm: Some("room.hnm".to_string()),
            background_music: Some("music".to_string()),
            talk_count: 4,
        };
        let mut actors = HashMap::new();
        actors.insert(0x003a, actor);

        // b3 = 0x03 (1-based voice selector) => clip = b3 - 1 = 2; b4 = 0x02 is
        // the control-flag word, NOT the clip index.
        let voiced = [
            0xc4, 0x3a, 0x00, 0x00, 0x00, 0xa6, 0x34, 0x12, 0x03, 0x02, 0x80, 0x01, 0x00, 0x00,
            0x00,
        ];
        let functions = vec![(0, "func".to_string()), (voiced.len(), "END".to_string())];
        let rows = parse_script_text_calls(
            "SCRIPTX",
            &voiced,
            &words,
            &functions,
            &actors,
            &HashMap::new(),
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].actor_record.as_deref(), Some("Test_Actor"));
        assert_eq!(rows[0].clip_index, Some(2));
        assert_eq!(rows[0].background_record.as_deref(), Some("Test_Room"));
        assert_eq!(rows[0].actor_ref, Some(0x003a));

        // b3 = 0xFF => narrator/menu subtitle, NO voice clip (b4 must not be
        // misread as an index). Regression guard for the removed `(0xFF,b4)` branch.
        let narrator = [
            0xc4, 0x3a, 0x00, 0x00, 0x00, 0xa6, 0x34, 0x12, 0xff, 0x02, 0x80, 0x01, 0x00, 0x00,
            0x00,
        ];
        let functions = vec![(0, "func".to_string()), (narrator.len(), "END".to_string())];
        let rows = parse_script_text_calls(
            "SCRIPTX",
            &narrator,
            &words,
            &functions,
            &actors,
            &HashMap::new(),
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].actor_record.as_deref(), Some("Test_Actor"));
        assert_eq!(rows[0].clip_index, None);
    }

    #[test]
    fn parses_real_script_speech_with_vm_tokens_if_present() {
        for prefix in ["output", "../output"] {
            let root = Path::new(prefix);
            let descript_path = root.join("DESCRIPT.DES");
            if !descript_path.exists() {
                continue;
            }

            let db = crate::extract::descript::parse_descript(&descript_path)
                .expect("parse DESCRIPT.DES");
            let hnm_music = db.hnm_music_map();
            let rows =
                parse_script_speech(root, Some(&db), &hnm_music).expect("parse script speech");
            assert!(
                rows.len() > 3000,
                "expected full VM-token speech coverage, got {} rows",
                rows.len()
            );
            assert!(
                rows.iter()
                    .any(|row| row.script == "SCRIPT2" && row.clip_index.is_some()),
                "SCRIPT2 should include voiced dialogue"
            );
            assert!(
                rows.iter()
                    .any(|row| row.source.starts_with("SCRIPT VM token")),
                "speech rows should come from the VM-token parser"
            );
            return;
        }

        eprintln!("skipping: extracted output scripts not available");
    }

    #[test]
    fn parses_real_script_branch_trace_if_present() {
        for prefix in ["output", "../output"] {
            let root = Path::new(prefix);
            if !root.join("scripts").exists() {
                continue;
            }

            let rows = parse_script_branch_trace(root).expect("parse branch trace");
            assert!(
                rows.len() > 1000,
                "expected real branch/control events, got {} rows",
                rows.len()
            );
            assert!(
                rows.iter()
                    .any(|row| row.script == "SCRIPT2" && row.branch_taken),
                "SCRIPT2 should include taken branch events"
            );
            let path = std::env::temp_dir().join(format!(
                "commander-blood-branch-trace-{}.tsv",
                std::process::id()
            ));
            write_script_branch_trace_manifest(&rows, &path).expect("write branch trace");
            let manifest = fs::read_to_string(&path).expect("read branch trace");
            let _ = fs::remove_file(&path);
            assert!(
                manifest.starts_with("script\tevent_index\toffset\topcode\ttarget\tbranch_taken")
            );
            assert!(manifest.contains("condition"));
            return;
        }

        eprintln!("skipping: extracted output scripts not available");
    }

    #[test]
    fn parses_real_script_executed_speech_if_present() {
        for prefix in ["output", "../output"] {
            let root = Path::new(prefix);
            let descript_path = root.join("DESCRIPT.DES");
            if !descript_path.exists() {
                continue;
            }

            let db = crate::extract::descript::parse_descript(&descript_path)
                .expect("parse DESCRIPT.DES");
            let hnm_music = db.hnm_music_map();
            let rows = parse_script_executed_speech(root, Some(&db), &hnm_music)
                .expect("parse executed speech");
            assert!(
                rows.len() > 900,
                "expected branch-aware executed dialogue, got {} rows",
                rows.len()
            );
            assert!(
                rows.iter()
                    .any(|row| row.script == "SCRIPT2" && row.clip_index.is_some()),
                "SCRIPT2 should include executed voiced dialogue"
            );
            assert!(
                rows.windows(2).all(|pair| {
                    pair[0].script != pair[1].script
                        || pair[0].sequence_index <= pair[1].sequence_index
                }),
                "executed dialogue rows should preserve per-script sequence order"
            );
            let path = std::env::temp_dir().join(format!(
                "commander-blood-executed-speech-{}.tsv",
                std::process::id()
            ));
            write_script_executed_speech_manifest(&rows, &path).expect("write executed speech");
            let manifest = fs::read_to_string(&path).expect("read executed speech");
            let _ = fs::remove_file(&path);
            assert!(manifest.starts_with("script\tsequence_index\tfunction\toffset"));
            assert!(manifest.contains("SCRIPT VM execute_trace"));
            return;
        }

        eprintln!("skipping: extracted output scripts not available");
    }

    #[test]
    fn dialogue_runs_keep_multi_actor_execution_order_and_split_locations() {
        let rows = vec![
            speech_line("SCRIPT2", 0x10, Some("Actor_A"), Some("Room1"), "a"),
            speech_line("SCRIPT2", 0x20, Some("Actor_B"), Some("Room1"), "b"),
            speech_line("SCRIPT2", 0x30, Some("Actor_A"), Some("Room2"), "c"),
            speech_line("SCRIPT2", 0x40, None, Some("Room2"), "narrator"),
            speech_line("SCRIPT3", 0x10, Some("Actor_A"), Some("Room1"), "d"),
        ];

        let runs = script_dialogue_runs(&rows);
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].script, "SCRIPT2");
        assert_eq!(runs[0].run_index, 1);
        assert_eq!(runs[0].first_offset, 0x10);
        assert_eq!(runs[0].last_offset, 0x20);
        assert_eq!(runs[0].lines.len(), 2);
        assert_eq!(runs[0].lines[0].actor_record.as_deref(), Some("Actor_A"));
        assert_eq!(runs[0].lines[1].actor_record.as_deref(), Some("Actor_B"));

        assert_eq!(runs[1].run_index, 2);
        assert_eq!(runs[1].background_record.as_deref(), Some("Room2"));
        assert_eq!(runs[1].lines.len(), 2);
        assert_eq!(runs[1].lines[1].actor_record, None);

        assert_eq!(runs[2].script, "SCRIPT3");
        assert_eq!(runs[2].run_index, 1);

        let path = std::env::temp_dir().join(format!(
            "commander-blood-dialogue-runs-{}.tsv",
            std::process::id()
        ));
        write_script_dialogue_runs_manifest(&rows, &path).expect("write dialogue runs");
        let manifest = fs::read_to_string(&path).expect("read dialogue runs");
        let _ = fs::remove_file(&path);
        assert!(manifest.contains("SCRIPT2-0001"));
        assert!(manifest.contains("Actor_A,Actor_B"));
    }

    #[test]
    fn executed_dialogue_runs_follow_sequence_order_and_split_locations() {
        let rows = vec![
            executed_speech_line("SCRIPT2", 0, 0x50, Some("Actor_A"), Some("Room1"), "a"),
            executed_speech_line("SCRIPT2", 1, 0x10, Some("Actor_B"), Some("Room1"), "b"),
            executed_speech_line("SCRIPT2", 2, 0x30, Some("Actor_A"), Some("Room2"), "c"),
            executed_speech_line("SCRIPT3", 0, 0x10, Some("Actor_A"), Some("Room1"), "d"),
        ];

        let runs = script_executed_dialogue_runs(&rows);
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].script, "SCRIPT2");
        assert_eq!(runs[0].run_index, 1);
        assert_eq!(runs[0].first_sequence, 0);
        assert_eq!(runs[0].last_sequence, 1);
        // Sequence order is authoritative even when COD offsets are not sorted.
        assert_eq!(runs[0].first_offset, 0x50);
        assert_eq!(runs[0].last_offset, 0x10);
        assert_eq!(runs[0].lines[0].actor_record.as_deref(), Some("Actor_A"));
        assert_eq!(runs[0].lines[1].actor_record.as_deref(), Some("Actor_B"));

        assert_eq!(runs[1].run_index, 2);
        assert_eq!(runs[1].background_record.as_deref(), Some("Room2"));
        assert_eq!(runs[2].script, "SCRIPT3");
        assert_eq!(runs[2].run_index, 1);

        let path = std::env::temp_dir().join(format!(
            "commander-blood-executed-dialogue-runs-{}.tsv",
            std::process::id()
        ));
        write_script_executed_dialogue_runs_manifest(&rows, &path)
            .expect("write executed dialogue runs");
        let manifest = fs::read_to_string(&path).expect("read executed dialogue runs");
        let _ = fs::remove_file(&path);
        assert!(manifest.contains("SCRIPT2-0001"));
        assert!(manifest.contains("Actor_A,Actor_B"));
        assert!(manifest.contains("0x00050\t0x00010"));
    }

    #[test]
    fn dialogue_video_manifest_uses_executed_sequence_order() {
        let mut early =
            executed_speech_line("SCRIPT2", 0, 0x50, Some("Actor_A"), Some("Room1"), "first");
        early.clip_index = Some(1);
        let mut late =
            executed_speech_line("SCRIPT2", 1, 0x10, Some("Actor_A"), Some("Room1"), "second");
        late.clip_index = Some(2);
        let mut silent =
            executed_speech_line("SCRIPT2", 2, 0x20, Some("Actor_A"), Some("Room1"), "silent");
        silent.clip_index = None;
        let rows = vec![late, silent, early];

        let path = std::env::temp_dir().join(format!(
            "commander-blood-dialogue-videos-{}.tsv",
            std::process::id()
        ));
        write_script_dialogue_manifest(&rows, &path).expect("write dialogue videos");
        let manifest = fs::read_to_string(&path).expect("read dialogue videos");
        let _ = fs::remove_file(&path);
        assert!(manifest.contains("dialogue - script2 - room1 - actor_a.mp4"));
        assert!(manifest.contains("\t3\t1,2"));
        assert!(!manifest.contains("\t3\t2,1"));
    }

    #[test]
    fn disassembly_uses_function_bounds_and_decodes_known_ops() {
        let mut words = HashMap::new();
        words.insert(0x0001, "hello".to_string());
        let cod = [
            0x01, 0x02, 0xc4, 0x3a, 0x00, 0x00, 0x00, 0xa6, 0x34, 0x12, 0x01, 0x00, 0x80, 0x01,
            0x00, 0x00, 0x00, 0x03,
        ];
        let mut actors = HashMap::new();
        actors.insert(
            0x003a,
            ScriptActorRef {
                talk_ref: 0x003a,
                record_name: "Test_Actor".to_string(),
                background_record: None,
                background_hnm: None,
                background_music: None,
                talk_count: 4,
            },
        );

        let rows = disassemble_function("SCRIPTX", "func", &cod, 0, cod.len(), &words, &actors);
        assert!(rows
            .iter()
            .any(|row| row.mnemonic == "actor_ref" && row.len == 5));
        assert!(rows
            .iter()
            .any(|row| row.mnemonic == "text_call" && row.text.as_deref() == Some("hello")));
        assert_eq!(rows[0].function_name, "func");
    }
}
