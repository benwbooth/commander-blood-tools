use super::*;

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
            let (function_start, function_name) = (&pair[0].0, &pair[0].1);
            let function_end = pair[1].0.min(cod.len());
            if *function_start >= function_end {
                continue;
            }
            let mut current_actor: Option<ScriptActorRef> = None;
            let mut rel = 0usize;
            while *function_start + rel < function_end {
                let pos = *function_start + rel;
                if pos + 2 < function_end && cod[pos] == 0xc4 {
                    let addr = u16::from_le_bytes([cod[pos + 1], cod[pos + 2]]);
                    if let Some(actor) = actor_refs.get(&addr) {
                        current_actor = Some(actor.clone());
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
                let mut words_out = Vec::new();
                while text_pos + 1 < function_end {
                    let word_off = u16::from_le_bytes([cod[text_pos], cod[text_pos + 1]]);
                    text_pos += 2;
                    if word_off == 0 {
                        break;
                    }
                    let Some(word) = words.get(&word_off) else {
                        words_out.clear();
                        break;
                    };
                    words_out.push(word.as_str());
                }

                if words_out.is_empty() {
                    rel += 1;
                    continue;
                }

                let params = &cod[pos + 3..marker];
                let param0 = params.first().copied();
                let param1 = params.get(1).copied();
                let actor = current_actor.clone();
                let actor_speaks = actor.is_some() && param1.is_some_and(|style| style < 0x10);
                let clip_index = actor.as_ref().and_then(|actor| {
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
                let source = match (&actor, actor_speaks, clip_index) {
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

                rows.push(ScriptSpeechLine {
                    script: script.clone(),
                    function_name: function_name.clone(),
                    offset: pos,
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
                    text: words_out.join(" "),
                });

                rel += 1;
            }
        }
    }

    Ok(rows)
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
        "script\tfunction\toffset\tactor\tparam0\tparam1\tclip_index\tbackground_record\tbackground_hnm\tbackground_music\tsource\ttext"
    )?;
    for row in rows {
        writeln!(
            file,
            "{}\t{}\t0x{:05x}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
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
            row.text.replace('\t', " ")
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
