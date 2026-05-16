use super::*;

#[derive(Clone, Debug)]
pub(super) struct ScriptTextCall {
    pub(super) offset: usize,
    pub(super) text_end: usize,
    pub(super) call_target: u16,
    pub(super) params: Vec<u8>,
    pub(super) words: Vec<String>,
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
            rows.extend(parse_function_text_calls(
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

pub(super) fn parse_function_text_calls(
    script: &str,
    function_name: &str,
    cod: &[u8],
    function_start: usize,
    function_end: usize,
    words: &HashMap<u16, String>,
    actor_refs: &HashMap<u16, ScriptActorRef>,
) -> Vec<ScriptSpeechLine> {
    let mut rows = Vec::new();
    if function_start >= function_end || function_start >= cod.len() {
        return rows;
    }

    let mut current_actor: Option<ScriptActorRef> = None;
    let mut pos = function_start;
    while pos < function_end {
        if pos + 2 < function_end && cod[pos] == 0xc4 {
            let addr = u16::from_le_bytes([cod[pos + 1], cod[pos + 2]]);
            current_actor = actor_refs.get(&addr).cloned();
            pos += 3;
            continue;
        }

        let Some(call) = decode_text_call_at(cod, function_end, words, pos) else {
            pos += 1;
            continue;
        };

        let param0 = call.params.first().copied();
        let param1 = call.params.get(1).copied();
        let actor = current_actor.clone();
        let actor_speaks = actor.is_some() && param1.is_some_and(|style| style < 0x10);
        let clip_index = actor.as_ref().and_then(|actor| {
            if !actor_speaks {
                return None;
            }
            match (param0, param1) {
                (Some(0xff), Some(idx)) if (idx as usize) < actor.talk_count => Some(idx as usize),
                (Some(idx), _) if idx > 0 && (idx as usize) <= actor.talk_count => {
                    Some(idx as usize - 1)
                }
                _ => None,
            }
        });
        let source = match (&actor, actor_speaks, clip_index) {
            (Some(_), true, Some(_)) => {
                "SCRIPT text call + tracked actor ref + DESCRIPT talk clip".to_string()
            }
            (Some(_), true, None) => {
                "SCRIPT text call + tracked actor ref; no mapped talk clip".to_string()
            }
            (Some(_), false, _) => {
                "SCRIPT text call + tracked actor ref; non-character subtitle channel".to_string()
            }
            (None, _, _) => "SCRIPT text call; no tracked actor ref".to_string(),
        };

        rows.push(ScriptSpeechLine {
            script: script.to_string(),
            function_name: function_name.to_string(),
            offset: call.offset,
            actor_record: actor.as_ref().map(|actor| actor.record_name.clone()),
            param0,
            param1,
            clip_index,
            background_record: actor
                .as_ref()
                .and_then(|actor| actor.background_record.clone()),
            background_hnm: actor
                .as_ref()
                .and_then(|actor| actor.background_hnm.clone()),
            background_music: actor
                .as_ref()
                .and_then(|actor| actor.background_music.clone()),
            source,
            text: call.words.join(" "),
            call_target: call.call_target,
            params_hex: hex_bytes(&call.params),
            text_end: call.text_end,
            actor_ref: actor.as_ref().map(|actor| actor.talk_ref),
            actor_proof: actor
                .as_ref()
                .map(|actor| format!("tracked 0xc4 actor ref 0x{:04x}", actor.talk_ref))
                .unwrap_or_default(),
            word_count: call.words.len(),
        });

        pos = call.text_end;
    }

    rows
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
        if pos + 2 < function_end && cod[pos] == 0xc4 {
            push_raw_disassembly(script, function_name, cod, &mut rows, raw_start.take(), pos);
            let addr = u16::from_le_bytes([cod[pos + 1], cod[pos + 2]]);
            current_actor = actor_refs.get(&addr).cloned();
            rows.push(ScriptDisassemblyLine {
                script: script.to_string(),
                function_name: function_name.to_string(),
                offset: pos,
                len: 3,
                opcode: "c4".to_string(),
                mnemonic: "actor_ref".to_string(),
                operands: format!("ref=0x{addr:04x}"),
                actor_record: current_actor
                    .as_ref()
                    .map(|actor| actor.record_name.clone()),
                text: None,
            });
            pos += 3;
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
                text: Some(call.words.join(" ")),
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

pub(super) fn decode_text_call_at(
    cod: &[u8],
    function_end: usize,
    words: &HashMap<u16, String>,
    pos: usize,
) -> Option<ScriptTextCall> {
    if pos + 4 >= function_end || cod.get(pos).copied()? != 0xa6 {
        return None;
    }

    let call_target = u16::from_le_bytes([cod[pos + 1], cod[pos + 2]]);
    let marker_search_end = (pos + 16).min(function_end);
    let marker_rel = cod[pos + 3..marker_search_end]
        .iter()
        .position(|&byte| byte == 0x80)?;
    let marker = pos + 3 + marker_rel;
    if marker != pos + 5 {
        return None;
    }
    if cod[pos + 3..marker].contains(&0xa6) {
        return None;
    }
    let mut text_pos = marker + 1;
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
        offset: pos,
        text_end: text_pos,
        call_target,
        params: cod[pos + 3..marker].to_vec(),
        words: decoded_words,
    })
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

pub(super) fn write_script_dialogue_manifest(
    rows: &[ScriptSpeechLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut groups: BTreeMap<(String, String, String), Vec<&ScriptSpeechLine>> = BTreeMap::new();
    for row in rows {
        let Some(actor) = row.actor_record.as_ref() else {
            continue;
        };
        if row.clip_index.is_none() {
            continue;
        }
        groups
            .entry((row.script.clone(), row.function_name.clone(), actor.clone()))
            .or_default()
            .push(row);
    }

    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "mp4\tscript\tfunction\tactor\tbackground_record\tbackground_hnm\tbackground_music\tline_count\tclip_indices"
    )?;
    for ((script, function_name, actor), mut lines) in groups {
        lines.sort_by_key(|line| line.offset);
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
        let cod = [
            0xc4, 0x3a, 0x00, 0xa6, 0x34, 0x12, 0xff, 0x02, 0x80, 0x01, 0x00, 0x00, 0x00,
        ];
        let mut actors = HashMap::new();
        actors.insert(
            0x003a,
            ScriptActorRef {
                talk_ref: 0x003a,
                record_name: "Test_Actor".to_string(),
                background_record: Some("Test_Room".to_string()),
                background_hnm: Some("room.hnm".to_string()),
                background_music: Some("music".to_string()),
                talk_count: 4,
            },
        );

        let rows =
            parse_function_text_calls("SCRIPTX", "func", &cod, 0, cod.len(), &words, &actors);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].actor_record.as_deref(), Some("Test_Actor"));
        assert_eq!(rows[0].clip_index, Some(2));
        assert_eq!(rows[0].background_record.as_deref(), Some("Test_Room"));
        assert_eq!(rows[0].actor_ref, Some(0x003a));
    }

    #[test]
    fn disassembly_uses_function_bounds_and_decodes_known_ops() {
        let mut words = HashMap::new();
        words.insert(0x0001, "hello".to_string());
        let cod = [
            0x01, 0x02, 0xc4, 0x3a, 0x00, 0xa6, 0x34, 0x12, 0x01, 0x00, 0x80, 0x01, 0x00, 0x00,
            0x00, 0x03,
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
        assert!(rows.iter().any(|row| row.mnemonic == "actor_ref"));
        assert!(
            rows.iter()
                .any(|row| row.mnemonic == "text_call" && row.text.as_deref() == Some("hello"))
        );
        assert_eq!(rows[0].function_name, "func");
    }
}
