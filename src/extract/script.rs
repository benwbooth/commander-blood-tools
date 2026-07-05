use super::*;

/// A decoded `0xA6` TEXT token from a `SCRIPT*.COD` stream.
///
/// Token layout recovered by reverse-engineering the VM TEXT handler at
/// `BLOODPRG.EXE` file 0x660C (see `re/REVERSE.md` "0xA6 TEXT handler"):
///
/// ```text
///   A6  b1 b2  b3  b4  b5   [loop:u16?] [control:u16?] w0 ... wN  0x0000
/// ```
///
/// * `b1:b2` (u16, little-endian) — index into the per-line record table
///   (`gs:0x6724`); kept here as [`call_target`].
/// * `b3` — per-line *selector* the handler stores to `gs:0x1FAB`
///   (→ `gs:0x6788 = sign_extend(b3) + 9`, the active-dialogue-line id).
///   `0xFF` = no voice; `1..=N` is a one-based talk-clip selector. Held as
///   `params[0]`.
/// * `b4` — *control-flag word* (NOT a clip index): bit3 `0x08` = conditional
///   skip-count follows, bit4 `0x10` = loop with target word, bit2 `0x04` =
///   extra control word before the dictionary words, bit0 `0x01` preserves the
///   active/display flag after accepting the line. Held as `params[1]`.
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

    let context = vm_execution_context_from_deb(deb, Some(descript_db));
    let mut out = HashMap::new();
    for line in vm::interpret_line_states_with_context(cod, var, &context) {
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

pub(super) fn parse_script_text_flags(
    iso_dir: &Path,
    descript_db: Option<&DescriptDb>,
    hnm_music: &HashMap<String, String>,
) -> Result<Vec<ScriptTextFlagLine>, Box<dyn Error>> {
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
        let (mut functions, _, _) =
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

        let mut text_calls: Vec<TextCallInfo> =
            text_calls_by_offset(&cod, &words).into_values().collect();
        text_calls.sort_by_key(|call| call.offset);
        for call in text_calls {
            rows.push(ScriptTextFlagLine {
                script: script.clone(),
                function_name: function_name_for_offset(&functions, call.offset).to_string(),
                offset: call.offset,
                line_index: call.line_index,
                voice_selector: call.voice_selector,
                active_line_id: vm::text_selector_active_line_id(call.voice_selector),
                flags_b4: call.flags_b4,
                flags_b5: call.flags_b5,
                loop_target: call.loop_target,
                active: call.flags_b5 & 0x80 != 0,
                skip_count: text_skip_count(call.flags_b4, call.flags_b5),
                summary: text_control_summary(call.flags_b4, call.flags_b5, call.loop_target),
                text: call.text,
            });
        }
    }

    Ok(rows)
}

fn text_skip_count(flags_b4: u8, flags_b5: u8) -> Option<u8> {
    vm::text_conditional_skip_count(flags_b4, flags_b5)
}

fn text_control_summary(flags_b4: u8, flags_b5: u8, loop_target: Option<u16>) -> String {
    let mut parts = Vec::new();
    if flags_b5 & 0x80 != 0 {
        parts.push("active".to_string());
    } else {
        parts.push("inactive".to_string());
    }
    if let Some(skip_count) = text_skip_count(flags_b4, flags_b5) {
        parts.push(format!("conditional-skip:{skip_count}"));
    }
    if flags_b4 & 0x10 != 0 {
        parts.push(match loop_target {
            Some(target) => format!("loop:0x{target:04x}"),
            None => "loop".to_string(),
        });
    }
    if flags_b4 & vm::TEXT_PRESERVE_ACTIVE_FLAG != 0 {
        parts.push("preserve-active".to_string());
    }
    if flags_b4 & 0x04 != 0 {
        parts.push("skip-extra-word".to_string());
    }
    let unknown_b4 = flags_b4 & !(0x01 | 0x04 | 0x08 | 0x10);
    if unknown_b4 != 0 {
        parts.push(format!("b4-unknown:0x{unknown_b4:02x}"));
    }
    let b5_payload = flags_b5 & 0x7f;
    if b5_payload != 0 {
        parts.push(format!("b5-payload:0x{b5_payload:02x}"));
    }
    parts.join(",")
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
    descript_db: Option<&DescriptDb>,
) -> Result<Vec<ScriptBranchTraceLine>, Box<dyn Error>> {
    let mut rows = Vec::new();
    for script_idx in 1..=5 {
        let cod_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.COD"));
        let var_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.VAR"));
        let deb_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.DEB"));
        let (Some(cod_path), Some(var_path)) = (cod_path, var_path) else {
            continue;
        };

        let cod = fs::read(cod_path)?;
        let var = fs::read(var_path)?;
        let context = match deb_path {
            Some(path) => vm_execution_context_from_deb(&fs::read(path)?, descript_db),
            None => vm::ExecutionContext::default(),
        };
        let script = format!("SCRIPT{script_idx}");
        let trace = vm::execute_trace_with_context(&cod, &var, &context);
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

pub(super) fn parse_script_post_update(
    iso_dir: &Path,
    descript_db: Option<&DescriptDb>,
) -> Result<Vec<ScriptPostUpdateLine>, Box<dyn Error>> {
    let mut rows = Vec::new();
    for script_idx in 1..=5 {
        let cod_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.COD"));
        let var_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.VAR"));
        let deb_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.DEB"));
        let (Some(cod_path), Some(var_path)) = (cod_path, var_path) else {
            continue;
        };

        let cod = fs::read(cod_path)?;
        let var = fs::read(var_path)?;
        let context = match deb_path {
            Some(path) => vm_execution_context_from_deb(&fs::read(path)?, descript_db),
            None => vm::ExecutionContext::default(),
        };
        let script = format!("SCRIPT{script_idx}");
        let trace = vm::execute_trace_with_context(&cod, &var, &context);
        let mut event_index = 0usize;

        for event in &trace.post_update.actor_record_pairs {
            rows.push(ScriptPostUpdateLine {
                script: script.clone(),
                event_index,
                event_kind: "c4_pair".to_string(),
                record_offset: Some(event.record_offset),
                related_record_offset: Some(event.related_record_offset),
                owner_offset: None,
                target: None,
                ready: None,
            });
            event_index += 1;
        }

        for event in &trace.post_update.presentation_handoffs {
            rows.push(ScriptPostUpdateLine {
                script: script.clone(),
                event_index,
                event_kind: "presentation_handoff".to_string(),
                record_offset: Some(event.record_offset),
                related_record_offset: None,
                owner_offset: Some(event.owner_offset),
                target: Some(event.target),
                ready: None,
            });
            event_index += 1;
        }

        if trace.pending_script_profile().is_some() {
            rows.push(ScriptPostUpdateLine {
                script,
                event_index,
                event_kind: "pending_profile_dispatch".to_string(),
                record_offset: None,
                related_record_offset: None,
                owner_offset: None,
                target: None,
                ready: Some(trace.post_update.pending_script_profile_dispatch_ready),
            });
        }
    }
    Ok(rows)
}

pub(super) fn parse_script_branch_scenarios(
    iso_dir: &Path,
    branch_rows: &[ScriptBranchTraceLine],
    descript_db: Option<&DescriptDb>,
) -> Result<Vec<ScriptBranchScenarioLine>, Box<dyn Error>> {
    let mut rows = Vec::new();
    for script_idx in 1..=5 {
        let cod_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.COD"));
        let dic_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.DIC"));
        let var_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.VAR"));
        let deb_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.DEB"));
        let (Some(cod_path), Some(dic_path), Some(var_path)) = (cod_path, dic_path, var_path)
        else {
            continue;
        };

        let cod = fs::read(&cod_path)?;
        let var = fs::read(&var_path)?;
        let context = match deb_path {
            Some(path) => vm_execution_context_from_deb(&fs::read(path)?, descript_db),
            None => vm::ExecutionContext::default(),
        };
        let words = parse_script_dictionary(&dic_path)?;
        let text_calls = text_calls_by_offset(&cod, &words);
        let script = format!("SCRIPT{script_idx}");
        let default_trace = vm::execute_trace_with_context(&cod, &var, &context);
        let default_offsets = executed_text_offsets(&default_trace, &text_calls);

        let default_set: BTreeSet<usize> = default_offsets.iter().copied().collect();
        let mut single_flip_covered = default_set.clone();
        let decisions: Vec<(usize, u8, bool)> = branch_rows
            .iter()
            .filter(|row| row.script == script && row.condition_passed.is_some())
            .map(|row| (row.offset, row.opcode, row.condition_passed.unwrap()))
            .collect();
        let mut decision_index = 0usize;
        for &(dec_offset, dec_opcode, default_condition_passed) in &decisions {
            decision_index += 1;
            let forced_condition_passed = !default_condition_passed;
            let scenario_trace = vm::execute_trace_with_overrides_and_context(
                &cod,
                &var,
                &[vm::BranchOverride {
                    offset: dec_offset,
                    condition_passed: forced_condition_passed,
                }],
                &context,
            );
            let scenario_offsets = executed_text_offsets(&scenario_trace, &text_calls);
            let scenario_set: BTreeSet<usize> = scenario_offsets.iter().copied().collect();
            let new_offsets: Vec<usize> = scenario_set.difference(&default_set).copied().collect();
            let lost_offsets: Vec<usize> = default_set.difference(&scenario_set).copied().collect();
            single_flip_covered.extend(scenario_set.iter().copied());
            rows.push(ScriptBranchScenarioLine {
                scenario_kind: "branch-override".to_string(),
                script: script.clone(),
                scenario_id: format!("{}-branch-{:04}", script, decision_index),
                decision_index,
                forced_offset: dec_offset,
                opcode: dec_opcode,
                default_condition_passed,
                forced_condition_passed,
                extra_overrides: Vec::new(),
                rtc_hour: None,
                rtc_month: None,
                rtc_day: None,
                default_text_calls: default_offsets.len(),
                scenario_text_calls: scenario_offsets.len(),
                new_text_calls: new_offsets.len(),
                lost_text_calls: lost_offsets.len(),
                first_new_offsets: new_offsets
                    .iter()
                    .take(12)
                    .map(|offset| format!("0x{offset:05x}"))
                    .collect::<Vec<_>>()
                    .join(","),
                halted: format!("{:?}", scenario_trace.halted),
                steps: scenario_trace.steps,
            });
        }

        // Depth-2 reachability-validated exploration: for each single-flip
        // decision, flip a SECOND decision that is actually reached in the
        // depth-1 trace, and emit the scenario only if it reaches dialogue lines
        // no single-flip did. Validated to add ~11% coverage (see the ignored
        // measure_depth2_branch_coverage_gain test). Budget-capped so the extra
        // video count stays bounded.
        let mut depth2_index = 0usize;
        let mut depth2_budget = 2000usize;
        let mut depth2_emitted = 0usize;
        'outer: for &(dec_offset, _, default_condition_passed) in &decisions {
            let ovr1 = vm::BranchOverride {
                offset: dec_offset,
                condition_passed: !default_condition_passed,
            };
            let t1 = vm::execute_trace_with_overrides_and_context(&cod, &var, &[ovr1], &context);
            for event in &t1.branch_events {
                if depth2_budget == 0 || depth2_emitted >= 60 {
                    break 'outer;
                }
                let Some(cp2) = event.condition_passed else {
                    continue;
                };
                if event.offset == dec_offset {
                    continue;
                }
                depth2_budget -= 1;
                let ovr2 = vm::BranchOverride {
                    offset: event.offset,
                    condition_passed: !cp2,
                };
                let t2 = vm::execute_trace_with_overrides_and_context(
                    &cod,
                    &var,
                    &[ovr1, ovr2],
                    &context,
                );
                let t2_set: BTreeSet<usize> = executed_text_offsets(&t2, &text_calls)
                    .into_iter()
                    .collect();
                let new_offsets: Vec<usize> =
                    t2_set.difference(&single_flip_covered).copied().collect();
                if new_offsets.is_empty() {
                    continue;
                }
                single_flip_covered.extend(new_offsets.iter().copied());
                depth2_index += 1;
                depth2_emitted += 1;
                rows.push(ScriptBranchScenarioLine {
                    scenario_kind: "branch-override".to_string(),
                    script: script.clone(),
                    scenario_id: format!("{}-branch2-{:04}", script, depth2_index),
                    decision_index: depth2_index,
                    forced_offset: dec_offset,
                    opcode: 0,
                    default_condition_passed,
                    forced_condition_passed: !default_condition_passed,
                    extra_overrides: vec![(event.offset, !cp2)],
                    rtc_hour: None,
                    rtc_month: None,
                    rtc_day: None,
                    default_text_calls: default_offsets.len(),
                    scenario_text_calls: t2_set.len(),
                    new_text_calls: new_offsets.len(),
                    lost_text_calls: 0,
                    first_new_offsets: new_offsets
                        .iter()
                        .take(12)
                        .map(|offset| format!("0x{offset:05x}"))
                        .collect::<Vec<_>>()
                        .join(","),
                    halted: format!("{:?}", t2.halted),
                    steps: t2.steps,
                });
            }
        }

        for rtc in rtc_replay_scenarios_for_cod(&cod) {
            let scenario_context = context.clone().with_bios_rtc(rtc.hour, rtc.month, rtc.day);
            let scenario_trace = vm::execute_trace_with_context(&cod, &var, &scenario_context);
            let scenario_offsets = executed_text_offsets(&scenario_trace, &text_calls);
            let default_set: BTreeSet<usize> = default_offsets.iter().copied().collect();
            let scenario_set: BTreeSet<usize> = scenario_offsets.iter().copied().collect();
            let new_offsets: Vec<usize> = scenario_set.difference(&default_set).copied().collect();
            let lost_offsets: Vec<usize> = default_set.difference(&scenario_set).copied().collect();
            rows.push(ScriptBranchScenarioLine {
                scenario_kind: "rtc".to_string(),
                script: script.clone(),
                scenario_id: format!(
                    "{}-rtc-{:02}h-{:02}{:02}",
                    script, rtc.hour, rtc.month, rtc.day
                ),
                decision_index: 0,
                forced_offset: 0,
                opcode: 0,
                default_condition_passed: false,
                forced_condition_passed: false,
                extra_overrides: Vec::new(),
                rtc_hour: Some(rtc.hour),
                rtc_month: Some(rtc.month),
                rtc_day: Some(rtc.day),
                default_text_calls: default_offsets.len(),
                scenario_text_calls: scenario_offsets.len(),
                new_text_calls: new_offsets.len(),
                lost_text_calls: lost_offsets.len(),
                first_new_offsets: new_offsets
                    .iter()
                    .take(12)
                    .map(|offset| format!("0x{offset:05x}"))
                    .collect::<Vec<_>>()
                    .join(","),
                halted: format!("{:?}", scenario_trace.halted),
                steps: scenario_trace.steps,
            });
        }
    }
    Ok(rows)
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct RtcReplayScenario {
    hour: u8,
    month: u8,
    day: u8,
}

fn rtc_replay_scenarios_for_cod(cod: &[u8]) -> Vec<RtcReplayScenario> {
    let mut thresholds = BTreeSet::new();
    let mut dates = BTreeSet::new();
    let mut has_rtc_token = false;

    for token in vm::walk(cod, 0, cod.len()) {
        match token {
            vm::VmToken::GlobalWordCompare { value, .. } if value <= 23 => {
                has_rtc_token = true;
                thresholds.insert(value as u8);
            }
            vm::VmToken::GlobalPairCompare { packed_value, .. } => {
                let month = (packed_value >> 8) as u8;
                let day = packed_value as u8;
                if (1..=12).contains(&month) && (1..=31).contains(&day) {
                    has_rtc_token = true;
                    dates.insert((month, day));
                }
            }
            _ => {}
        }
    }

    if !has_rtc_token {
        return Vec::new();
    }

    let mut hours = BTreeSet::new();
    if thresholds.is_empty() {
        hours.insert(12);
    } else {
        hours.insert(0);
        for threshold in thresholds {
            hours.insert(threshold);
            if threshold < 23 {
                hours.insert(threshold + 1);
            }
        }
    }

    // Jan 2 is an ordinary non-holiday baseline distinct from observed 1/1 and
    // 12/25 seasonal checks.
    dates.insert((1, 2));

    let mut scenarios = BTreeSet::new();
    for hour in hours {
        for (month, day) in &dates {
            scenarios.insert(RtcReplayScenario {
                hour,
                month: *month,
                day: *day,
            });
        }
    }
    scenarios.into_iter().collect()
}

fn executed_text_offsets(
    trace: &vm::ExecutionTrace,
    text_calls: &HashMap<usize, TextCallInfo>,
) -> Vec<usize> {
    trace
        .line_states
        .iter()
        .filter_map(|state| {
            text_calls
                .contains_key(&state.offset)
                .then_some(state.offset)
        })
        .collect()
}

#[derive(Clone, Debug)]
struct TextCallInfo {
    offset: usize,
    line_index: u16,
    voice_selector: u8,
    flags_b4: u8,
    flags_b5: u8,
    skip_count: Option<u8>,
    loop_target: Option<u16>,
    text: String,
    text_end: usize,
}

struct LoadedScriptProfile {
    profile_index: u16,
    d2_operand: u8,
    script: String,
    cod: Vec<u8>,
    var: Vec<u8>,
    functions: Vec<(usize, String)>,
    actor_by_offset: HashMap<u16, ScriptActorRef>,
    object_names: HashMap<u16, String>,
    context: vm::ExecutionContext,
    text_calls: HashMap<usize, TextCallInfo>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct ScriptProfileSequenceExport {
    pub(super) runs: Vec<ScriptProfileRunLine>,
    pub(super) dialogue: Vec<ScriptProfileExecutedSpeechLine>,
}

#[derive(Clone, Debug)]
pub(super) struct ScriptProfileRunLine {
    pub(super) sequence_id: String,
    pub(super) run_index: usize,
    pub(super) profile_index: u16,
    pub(super) d2_operand: u8,
    pub(super) script: String,
    pub(super) steps: usize,
    pub(super) text_calls: usize,
    pub(super) pending_profile_index: Option<u16>,
    pub(super) pending_script: Option<String>,
    pub(super) pending_dispatch_ready: bool,
    pub(super) post_update_pairs: String,
    pub(super) presentation_handoffs: String,
    pub(super) request_summary: String,
    pub(super) halted_after_run: String,
}

#[derive(Clone, Debug)]
pub(super) struct ScriptProfileExecutedSpeechLine {
    pub(super) sequence_id: String,
    pub(super) global_sequence_index: usize,
    pub(super) run_index: usize,
    pub(super) profile_index: u16,
    pub(super) d2_operand: u8,
    pub(super) script_sequence_index: usize,
    pub(super) row: ScriptExecutedSpeechLine,
}

fn required_profile_slot_name(
    profile: &ScriptResourceProfile,
    slot_index: usize,
) -> Result<&str, Box<dyn Error>> {
    profile
        .slots
        .iter()
        .find(|slot| slot.slot == slot_index)
        .map(|slot| slot.name.as_str())
        .ok_or_else(|| {
            format!(
                "script resource profile {} is missing slot {slot_index}",
                profile.profile_index
            )
            .into()
        })
}

fn required_profile_slot_path(
    iso_dir: &Path,
    profile: &ScriptResourceProfile,
    slot_index: usize,
) -> Result<PathBuf, Box<dyn Error>> {
    let name = required_profile_slot_name(profile, slot_index)?;
    find_file_recursive(iso_dir, name).ok_or_else(|| {
        format!(
            "script resource profile {} references {name}, but it was not found",
            profile.profile_index
        )
        .into()
    })
}

fn load_script_profile(
    iso_dir: &Path,
    profile: &ScriptResourceProfile,
    descript_db: Option<&DescriptDb>,
    hnm_music: &HashMap<String, String>,
    character_names: &[String],
) -> Result<LoadedScriptProfile, Box<dyn Error>> {
    let cod_path = required_profile_slot_path(iso_dir, profile, 0)?;
    let var_path = required_profile_slot_path(iso_dir, profile, 2)?;
    let dic_path = required_profile_slot_path(iso_dir, profile, 3)?;
    let deb_path = required_profile_slot_path(iso_dir, profile, 4)?;

    let cod = fs::read(&cod_path)?;
    let var = fs::read(&var_path)?;
    let deb = fs::read(&deb_path)?;
    let words = parse_script_dictionary(&dic_path)?;
    let script = format!("SCRIPT{}", profile.script_number);
    let object_names = parse_deb_object_names(&deb);
    let context = vm_execution_context_from_deb(&deb, descript_db);
    let (mut functions, actor_refs) = if let Some(db) = descript_db {
        let (functions, actor_refs, _) = parse_script_symbols(
            &script,
            &deb_path,
            &var_path,
            db,
            hnm_music,
            character_names,
        )?;
        (functions, actor_refs)
    } else {
        (Vec::new(), HashMap::new())
    };
    if functions.is_empty() {
        functions.push((0, script.as_str().to_string()));
    }
    functions.sort_by_key(|(offset, _)| *offset);
    functions.push((cod.len(), "END".to_string()));

    let actor_by_offset = actor_refs
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

    Ok(LoadedScriptProfile {
        profile_index: profile.profile_index as u16,
        d2_operand: profile.d2_operand,
        script,
        cod,
        var,
        functions,
        actor_by_offset,
        object_names,
        context,
        text_calls,
    })
}

fn loaded_profile_by_index(
    profiles: &[LoadedScriptProfile],
    profile_index: u16,
) -> Option<&LoadedScriptProfile> {
    profiles
        .iter()
        .find(|profile| profile.profile_index == profile_index)
}

fn script_profile_request_summary(trace: &vm::ExecutionTrace) -> String {
    trace
        .script_profile_requests
        .iter()
        .map(|event| {
            format!(
                "0x{:05x}:{}->{}",
                event.offset, event.operand, event.profile_index
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn post_update_pair_summary(trace: &vm::ExecutionTrace) -> String {
    trace
        .post_update
        .actor_record_pairs
        .iter()
        .map(|event| {
            format!(
                "0x{:04x}->0x{:04x}",
                event.record_offset, event.related_record_offset
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn presentation_handoff_summary(trace: &vm::ExecutionTrace) -> String {
    trace
        .post_update
        .presentation_handoffs
        .iter()
        .map(|event| {
            format!(
                "owner=0x{:04x}:record=0x{:04x}->0x{:04x}",
                event.owner_offset, event.record_offset, event.target
            )
        })
        .collect::<Vec<_>>()
        .join(",")
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
        let (mut functions, actor_refs, object_names, context) =
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
                let object_names = parse_deb_object_names(&deb);
                let context = vm_execution_context_from_deb(&deb, descript_db);
                (functions, actor_refs, object_names, context)
            } else {
                (
                    Vec::new(),
                    HashMap::new(),
                    HashMap::new(),
                    vm::ExecutionContext::default(),
                )
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
        let trace = vm::execute_trace_with_context(&cod, &var, &context);
        rows.extend(executed_speech_rows_from_trace(
            &script,
            None,
            "SCRIPT VM execute_trace",
            &trace,
            &text_calls,
            &functions,
            &actor_by_offset,
            &object_names,
            descript_db,
            hnm_music,
        ));
    }

    Ok(rows)
}

/// Dialogue in named COD functions the main execution trace never enters
/// (event-triggered scenes — the source of ~40% of otherwise-uncovered dialogue).
/// These lines are resolved by the STATIC analysis (`parse_script_text_calls`:
/// per-offset actor + runtime background), not execution, because the function's
/// speaker is set by its caller, not within it. Emits only renderable lines (a
/// resolved actor AND background) in never-executed functions, skipping any line
/// the trace already reached (dedup). Grouped per function via a `fn:<script>:<fn>`
/// scenario id so each becomes its own scene run. See the sess-004 breakthrough in
/// the YouTube-oracle memory.
pub(super) fn parse_script_uncovered_speech(
    iso_dir: &Path,
    descript_db: Option<&DescriptDb>,
    hnm_music: &HashMap<String, String>,
) -> Result<Vec<ScriptExecutedSpeechLine>, Box<dyn Error>> {
    let mut rows = Vec::new();
    let character_names = descript_db
        .map(|db| db.character_names())
        .unwrap_or_default();

    for script_idx in 1..=5 {
        let (Some(cod_path), Some(dic_path), Some(deb_path), Some(var_path)) = (
            find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.COD")),
            find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.DIC")),
            find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.DEB")),
            find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.VAR")),
        ) else {
            continue;
        };
        let Some(db) = descript_db else {
            continue;
        };

        let cod = fs::read(&cod_path)?;
        let var = fs::read(&var_path)?;
        let deb = fs::read(&deb_path)?;
        let words = parse_script_dictionary(&dic_path)?;
        let script = format!("SCRIPT{script_idx}");
        let runtime_bg = resolve_runtime_backgrounds(&cod, &var, &deb, db, hnm_music);
        let (mut functions, actor_refs, _) = parse_script_symbols(
            &script,
            &deb_path,
            &var_path,
            db,
            hnm_music,
            &character_names,
        )?;
        if functions.is_empty() {
            functions.push((0, script.clone()));
        }
        functions.sort_by_key(|(offset, _)| *offset);
        functions.push((cod.len(), "END".to_string()));

        // All static text-call lines, fully resolved (actor + runtime background).
        let static_lines =
            parse_script_text_calls(&script, &cod, &words, &functions, &actor_refs, &runtime_bg);

        // Offsets the main execution trace actually reached.
        let context = vm_execution_context_from_deb(&deb, descript_db);
        let text_calls = text_calls_by_offset(&cod, &words);
        let trace = vm::execute_trace_with_context(&cod, &var, &context);
        let executed_offsets: std::collections::BTreeSet<usize> =
            executed_text_offsets(&trace, &text_calls)
                .into_iter()
                .collect();
        // A function is "covered" if any of its text calls executed; skip those
        // entirely so we only add genuinely-missing event-triggered scenes.
        let covered_fns: std::collections::BTreeSet<&str> = static_lines
            .iter()
            .filter(|line| executed_offsets.contains(&line.offset))
            .map(|line| line.function_name.as_str())
            .collect();

        let mut seq = 0usize;
        for line in &static_lines {
            if covered_fns.contains(line.function_name.as_str())
                || executed_offsets.contains(&line.offset)
                || line.actor_record.is_none()
                || (line.background_hnm.is_none() && line.background_record.is_none())
                || line.text.trim().is_empty()
            {
                continue;
            }
            rows.push(executed_line_from_static(&script, line, seq));
            seq += 1;
        }
    }

    Ok(rows)
}

/// Convert a statically-resolved [`ScriptSpeechLine`] into the executed-line form
/// the dialogue-run video renderer consumes, tagging it with a `fn:<script>:<fn>`
/// scenario id so it groups into a per-function scene run.
fn executed_line_from_static(
    script: &str,
    line: &ScriptSpeechLine,
    sequence_index: usize,
) -> ScriptExecutedSpeechLine {
    ScriptExecutedSpeechLine {
        scenario_id: Some(format!("fn:{}:{}", script, line.function_name)),
        script: script.to_string(),
        sequence_index,
        function_name: line.function_name.clone(),
        offset: line.offset,
        actor_record: line.actor_record.clone(),
        actor_ref: line.actor_ref,
        location_offset: None,
        background_record: line.background_record.clone(),
        background_hnm: line.background_hnm.clone(),
        background_music: line.background_music.clone(),
        param0: line.param0.unwrap_or(0),
        param1: line.param1.unwrap_or(0),
        skip_count: line.skip_count,
        loop_target: line.loop_target,
        active_line_id: line
            .active_line_id
            .unwrap_or_else(|| vm::text_selector_active_line_id(1)),
        clip_index: line.clip_index,
        text: line.text.clone(),
        call_target: line.call_target,
        text_end: line.text_end,
        source: "static uncovered function scene".to_string(),
    }
}

pub(super) fn parse_script_profile_sequence(
    iso_dir: &Path,
    profiles: &[ScriptResourceProfile],
    descript_db: Option<&DescriptDb>,
    hnm_music: &HashMap<String, String>,
) -> Result<ScriptProfileSequenceExport, Box<dyn Error>> {
    if profiles.is_empty() {
        return Ok(ScriptProfileSequenceExport::default());
    }

    let character_names = descript_db
        .map(|db| db.character_names())
        .unwrap_or_default();
    let mut loaded_profiles = Vec::with_capacity(profiles.len());
    for profile in profiles {
        loaded_profiles.push(load_script_profile(
            iso_dir,
            profile,
            descript_db,
            hnm_music,
            &character_names,
        )?);
    }

    let programs: Vec<_> = loaded_profiles
        .iter()
        .map(|profile| vm::ScriptProfileProgram {
            profile_index: profile.profile_index,
            cod: &profile.cod,
            var: &profile.var,
            context: profile.context.clone(),
        })
        .collect();
    let execution = vm::execute_script_profile_sequence(&programs, 0, 32);
    let mut run_rows = Vec::new();
    let mut dialogue_rows = Vec::new();

    for (run_ordinal, run) in execution.runs.iter().enumerate() {
        let Some(profile) = loaded_profile_by_index(&loaded_profiles, run.profile_index) else {
            continue;
        };
        let mut rows = executed_speech_rows_from_trace(
            &profile.script,
            None,
            "SCRIPT VM execute_script_profile_sequence",
            &run.trace,
            &profile.text_calls,
            &profile.functions,
            &profile.actor_by_offset,
            &profile.object_names,
            descript_db,
            hnm_music,
        );
        let text_calls = rows.len();
        for row in rows.drain(..) {
            dialogue_rows.push(ScriptProfileExecutedSpeechLine {
                sequence_id: "default".to_string(),
                global_sequence_index: dialogue_rows.len(),
                run_index: run.run_index,
                profile_index: run.profile_index,
                d2_operand: profile.d2_operand,
                script_sequence_index: row.sequence_index,
                row,
            });
        }

        let pending_profile_index = run.trace.pending_script_profile();
        let pending_script = pending_profile_index
            .and_then(|idx| loaded_profile_by_index(&loaded_profiles, idx))
            .map(|profile| profile.script.clone());
        let halted_after_run = if run_ordinal + 1 == execution.runs.len() {
            format!("{:?}", execution.halted)
        } else {
            "handoff".to_string()
        };
        run_rows.push(ScriptProfileRunLine {
            sequence_id: "default".to_string(),
            run_index: run.run_index,
            profile_index: run.profile_index,
            d2_operand: profile.d2_operand,
            script: profile.script.clone(),
            steps: run.trace.steps,
            text_calls,
            pending_profile_index,
            pending_script,
            pending_dispatch_ready: run.trace.post_update.pending_script_profile_dispatch_ready,
            post_update_pairs: post_update_pair_summary(&run.trace),
            presentation_handoffs: presentation_handoff_summary(&run.trace),
            request_summary: script_profile_request_summary(&run.trace),
            halted_after_run,
        });
    }

    Ok(ScriptProfileSequenceExport {
        runs: run_rows,
        dialogue: dialogue_rows,
    })
}

pub(super) fn parse_script_branch_scenario_speech(
    iso_dir: &Path,
    descript_db: Option<&DescriptDb>,
    hnm_music: &HashMap<String, String>,
    scenarios: &[ScriptBranchScenarioLine],
) -> Result<Vec<ScriptExecutedSpeechLine>, Box<dyn Error>> {
    let mut rows = Vec::new();
    let character_names = descript_db
        .map(|db| db.character_names())
        .unwrap_or_default();

    for script_idx in 1..=5 {
        let script = format!("SCRIPT{script_idx}");
        let script_scenarios: Vec<&ScriptBranchScenarioLine> = scenarios
            .iter()
            .filter(|scenario| scenario.script == script)
            .collect();
        if script_scenarios.is_empty() {
            continue;
        }

        let cod_path = find_file_recursive(iso_dir, &format!("{script}.COD"));
        let dic_path = find_file_recursive(iso_dir, &format!("{script}.DIC"));
        let deb_path = find_file_recursive(iso_dir, &format!("{script}.DEB"));
        let var_path = find_file_recursive(iso_dir, &format!("{script}.VAR"));
        let (Some(cod_path), Some(dic_path), Some(var_path)) = (cod_path, dic_path, var_path)
        else {
            continue;
        };

        let cod = fs::read(&cod_path)?;
        let var = fs::read(&var_path)?;
        let words = parse_script_dictionary(&dic_path)?;
        let (mut functions, actor_refs, object_names, context) =
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
                let object_names = parse_deb_object_names(&deb);
                let context = vm_execution_context_from_deb(&deb, descript_db);
                (functions, actor_refs, object_names, context)
            } else {
                (
                    Vec::new(),
                    HashMap::new(),
                    HashMap::new(),
                    vm::ExecutionContext::default(),
                )
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

        for scenario in script_scenarios {
            let mut scenario_context = context.clone();
            if let (Some(hour), Some(month), Some(day)) =
                (scenario.rtc_hour, scenario.rtc_month, scenario.rtc_day)
            {
                scenario_context = scenario_context.with_bios_rtc(hour, month, day);
            }
            let trace = if scenario.scenario_kind == "branch-override" {
                let mut overrides = vec![vm::BranchOverride {
                    offset: scenario.forced_offset,
                    condition_passed: scenario.forced_condition_passed,
                }];
                overrides.extend(scenario.extra_overrides.iter().map(|&(offset, cp)| {
                    vm::BranchOverride {
                        offset,
                        condition_passed: cp,
                    }
                }));
                vm::execute_trace_with_overrides_and_context(
                    &cod,
                    &var,
                    &overrides,
                    &scenario_context,
                )
            } else {
                vm::execute_trace_with_context(&cod, &var, &scenario_context)
            };
            let source_base = if scenario.scenario_kind == "rtc" {
                "SCRIPT VM execute_trace + BIOS RTC scenario"
            } else {
                "SCRIPT VM execute_trace_with_overrides"
            };
            rows.extend(executed_speech_rows_from_trace(
                &script,
                Some(scenario.scenario_id.as_str()),
                source_base,
                &trace,
                &text_calls,
                &functions,
                &actor_by_offset,
                &object_names,
                descript_db,
                hnm_music,
            ));
        }
    }

    Ok(rows)
}

fn executed_speech_rows_from_trace(
    script: &str,
    scenario_id: Option<&str>,
    source_base: &str,
    trace: &vm::ExecutionTrace,
    text_calls: &HashMap<usize, TextCallInfo>,
    functions: &[(usize, String)],
    actor_by_offset: &HashMap<u16, ScriptActorRef>,
    object_names: &HashMap<u16, String>,
    descript_db: Option<&DescriptDb>,
    hnm_music: &HashMap<String, String>,
) -> Vec<ScriptExecutedSpeechLine> {
    let mut rows = Vec::new();
    for (sequence_index, state) in trace.line_states.iter().enumerate() {
        let Some(call) = text_calls.get(&state.offset) else {
            continue;
        };
        let actor = state
            .actor_offset
            .and_then(|offset| actor_by_offset.get(&offset).cloned());
        let background = state.location_offset.and_then(|loc| {
            resolve_background_from_location(loc, object_names, descript_db, hnm_music)
        });
        let actor_speaks = actor.is_some() && call.flags_b4 < 0x10;
        let clip_index = actor.as_ref().and_then(|actor| {
            if !actor_speaks {
                return None;
            }
            vm::text_selector_voice_clip_index(call.voice_selector, actor.talk_count)
        });
        let source = match (&actor, actor_speaks, clip_index) {
            (Some(_), true, Some(_)) => {
                format!("{source_base} + actor state + DESCRIPT talk clip")
            }
            (Some(_), true, None) => {
                format!("{source_base} + actor state; no mapped talk clip")
            }
            (Some(_), false, _) => {
                format!("{source_base} + actor state; non-character subtitle channel")
            }
            (None, _, _) => format!("{source_base}; no tracked actor state"),
        };

        rows.push(ScriptExecutedSpeechLine {
            scenario_id: scenario_id.map(str::to_string),
            script: script.to_string(),
            sequence_index,
            function_name: function_name_for_offset(functions, call.offset).to_string(),
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
            skip_count: call.skip_count,
            loop_target: call.loop_target,
            active_line_id: vm::text_selector_active_line_id(call.voice_selector),
            clip_index,
            text: call.text.clone(),
            call_target: call.line_index,
            text_end: call.text_end,
            source,
        });
    }
    rows
}

fn text_calls_by_offset(cod: &[u8], words: &HashMap<u16, String>) -> HashMap<usize, TextCallInfo> {
    let mut calls = HashMap::new();
    for token in vm::walk(cod, 0, cod.len()) {
        let vm::VmToken::Text {
            offset,
            line_index,
            voice_selector,
            flags_b4,
            flags_b5,
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
                flags_b5,
                skip_count: vm::text_conditional_skip_count(flags_b4, flags_b5),
                loop_target,
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

fn vm_execution_context_from_deb(
    deb: &[u8],
    descript_db: Option<&DescriptDb>,
) -> vm::ExecutionContext {
    let object_names = parse_deb_object_names(deb);
    let mut context = vm_execution_context_from_object_names(&object_names, descript_db);
    for record in deb.chunks_exact(20) {
        let kind = u16::from_le_bytes([record[18], record[19]]);
        if kind != 1 && kind != 5 {
            continue;
        }
        let name_len = record[..16].iter().position(|&b| b == 0).unwrap_or(16);
        if name_len == 0 {
            continue;
        }
        let name = String::from_utf8_lossy(&record[..name_len]);
        let offset = u16::from_le_bytes([record[16], record[17]]);
        context = context.with_vm_named_object(name.as_ref(), offset);
    }
    context
}

fn vm_execution_context_from_object_names(
    object_names: &HashMap<u16, String>,
    descript_db: Option<&DescriptDb>,
) -> vm::ExecutionContext {
    let mut context = vm::ExecutionContext::from_object_offsets(object_names.keys().copied());
    for (&offset, name) in object_names {
        context = context.with_vm_named_object(name, offset);
    }
    if let Some(db) = descript_db {
        for name in db.record_names() {
            context = context.with_descript_entry_name(name);
        }
    }
    context
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
            vm::VmToken::Actor { record_offset, .. } => {
                current_actor = actor_refs.get(&record_offset).cloned();
            }
            vm::VmToken::RecordClear { record_offset, .. } => {
                if matches!(current_actor.as_ref(), Some(actor) if actor.talk_ref == record_offset)
                {
                    current_actor = None;
                }
            }
            vm::VmToken::Text {
                offset,
                line_index,
                voice_selector,
                flags_b4,
                flags_b5,
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
                // tracing gs:0x6788 = sign_extend(b3) + 9 into the son.snd player + the export-data
                // distribution): `param0` (b3) is the per-line voice selector —
                //   * b3 == 0xFF or 0x00 => NO voice (narrator/menu/tutorial subtitle;
                //     sign-extended b3 + 9 wraps to active line id 8), and
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
                    vm::text_selector_voice_clip_index(voice_selector, actor.talk_count)
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
                let active_line_id = vm::text_selector_active_line_id(voice_selector);

                rows.push(ScriptSpeechLine {
                    script: script.to_string(),
                    function_name: function_name.to_string(),
                    offset,
                    actor_record: actor.as_ref().map(|actor| actor.record_name.clone()),
                    param0,
                    param1,
                    skip_count: vm::text_conditional_skip_count(flags_b4, flags_b5),
                    loop_target,
                    active_line_id: Some(active_line_id),
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
    let control_len = if flags_b4 & 0x04 != 0 { 2 } else { 0 };
    offset + 6 + loop_len + control_len + word_count * 2 + 2
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
        let tokens = vm::walk(cod, pos, function_end);
        if tokens.is_empty() {
            raw_start.get_or_insert(pos);
            pos += 1;
            continue;
        }

        for token in tokens {
            match token {
                vm::VmToken::Invalid { offset, .. } => {
                    raw_start.get_or_insert(offset);
                    pos = (offset + 1).min(function_end);
                    break;
                }
                token => {
                    let offset = vm_token_offset(&token);
                    let len = vm_token_len(&token);
                    if vm_token_has_disassembly(&token, words) {
                        push_raw_disassembly(
                            script,
                            function_name,
                            cod,
                            &mut rows,
                            raw_start.take(),
                            offset,
                        );
                    }
                    let emitted = push_vm_token_disassembly(
                        script,
                        function_name,
                        words,
                        actor_refs,
                        &mut current_actor,
                        &mut rows,
                        &token,
                    );
                    if !emitted {
                        raw_start.get_or_insert(offset);
                    }
                    pos = (offset + len).min(function_end);
                }
            }

            if raw_start.is_some_and(|start| pos - start >= 32) {
                push_raw_disassembly(script, function_name, cod, &mut rows, raw_start.take(), pos);
            }
            if pos >= function_end {
                break;
            }
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

fn vm_token_has_disassembly(token: &vm::VmToken, words: &HashMap<u16, String>) -> bool {
    match token {
        vm::VmToken::Text { word_offsets, .. } => decode_vm_words(words, word_offsets).is_some(),
        vm::VmToken::ScriptProfileRequest { .. } => true,
        vm::VmToken::Op { .. } | vm::VmToken::Invalid { .. } => false,
        _ => true,
    }
}

fn current_actor_record(current_actor: &Option<ScriptActorRef>) -> Option<String> {
    current_actor
        .as_ref()
        .map(|actor| actor.record_name.clone())
}

fn push_vm_token_disassembly(
    script: &str,
    function_name: &str,
    words: &HashMap<u16, String>,
    actor_refs: &HashMap<u16, ScriptActorRef>,
    current_actor: &mut Option<ScriptActorRef>,
    rows: &mut Vec<ScriptDisassemblyLine>,
    token: &vm::VmToken,
) -> bool {
    match token {
        vm::VmToken::Actor {
            offset,
            record_offset,
            related_record_offset,
            len,
            ..
        } => {
            *current_actor = actor_refs.get(record_offset).cloned();
            rows.push(ScriptDisassemblyLine {
                script: script.to_string(),
                function_name: function_name.to_string(),
                offset: *offset,
                len: *len,
                opcode: "c4".to_string(),
                mnemonic: "actor_ref".to_string(),
                operands: format!("ref=0x{record_offset:04x} extra=0x{related_record_offset:04x}"),
                actor_record: current_actor_record(current_actor),
                text: None,
            });
            true
        }
        vm::VmToken::RecordLink {
            offset,
            record_offset,
            related_record_offset,
            len,
            ..
        } => {
            rows.push(ScriptDisassemblyLine {
                script: script.to_string(),
                function_name: function_name.to_string(),
                offset: *offset,
                len: *len,
                opcode: "c3".to_string(),
                mnemonic: "record_link".to_string(),
                operands: format!(
                    "ref=0x{record_offset:04x} related=0x{related_record_offset:04x} aux=0x0001"
                ),
                actor_record: current_actor_record(current_actor),
                text: None,
            });
            true
        }
        vm::VmToken::RecordEntry {
            offset,
            entry_opcode,
            record_offset,
            operand,
            stored_related_offset,
            aux_word,
            len,
            ..
        } => {
            rows.push(ScriptDisassemblyLine {
                script: script.to_string(),
                function_name: function_name.to_string(),
                offset: *offset,
                len: *len,
                opcode: format!("{entry_opcode:02x}"),
                mnemonic: "record_entry".to_string(),
                operands: format!(
                    "kind=0x{entry_opcode:02x} ref=0x{record_offset:04x} operand=0x{operand:04x} stored_related=0x{stored_related_offset:04x} aux=0x{aux_word:04x}"
                ),
                actor_record: current_actor_record(current_actor),
                text: None,
            });
            true
        }
        vm::VmToken::BitFlag {
            offset,
            flag_offset,
            bit_index,
            byte_offset,
            mask,
            clear,
            len,
        } => {
            rows.push(ScriptDisassemblyLine {
                script: script.to_string(),
                function_name: function_name.to_string(),
                offset: *offset,
                len: *len,
                opcode: "b7".to_string(),
                mnemonic: "bit_flag".to_string(),
                operands: format!(
                    "ref=0x{flag_offset:04x} bit={bit_index} byte=0x{byte_offset:04x} mask=0x{mask:02x} action={}",
                    if *clear { "clear_or_invert_test" } else { "set_or_test" }
                ),
                actor_record: current_actor_record(current_actor),
                text: None,
            });
            true
        }
        vm::VmToken::RecordState {
            offset,
            opcode,
            record_offset,
            operand,
            inverted,
            len,
        } => {
            rows.push(ScriptDisassemblyLine {
                script: script.to_string(),
                function_name: function_name.to_string(),
                offset: *offset,
                len: *len,
                opcode: format!("{opcode:02x}"),
                mnemonic: "record_state".to_string(),
                operands: format!(
                    "kind=0x{opcode:02x} ref=0x{record_offset:04x} operand=0x{operand:04x} inverted={inverted}"
                ),
                actor_record: current_actor_record(current_actor),
                text: None,
            });
            true
        }
        vm::VmToken::GlobalWordCompare {
            offset,
            operator,
            tag,
            value,
            len,
        } => {
            rows.push(ScriptDisassemblyLine {
                script: script.to_string(),
                function_name: function_name.to_string(),
                offset: *offset,
                len: *len,
                opcode: "ca".to_string(),
                mnemonic: "global_word_compare".to_string(),
                operands: format!(
                    "global=gs:0x0aa6 op=0x{operator:02x} tag=0x{tag:02x} value=0x{value:04x}"
                ),
                actor_record: current_actor_record(current_actor),
                text: None,
            });
            true
        }
        vm::VmToken::GlobalPairCompare {
            offset,
            operator,
            packed_value,
            reserved,
            len,
        } => {
            rows.push(ScriptDisassemblyLine {
                script: script.to_string(),
                function_name: function_name.to_string(),
                offset: *offset,
                len: *len,
                opcode: "cb".to_string(),
                mnemonic: "global_pair_compare".to_string(),
                operands: format!(
                    "global=gs:0x0aaa:0x0aa8 op=0x{operator:02x} packed=0x{packed_value:04x} reserved=0x{reserved:04x}"
                ),
                actor_record: current_actor_record(current_actor),
                text: None,
            });
            true
        }
        vm::VmToken::PairRecord {
            offset,
            opcode,
            record_offset,
            first_word,
            second_word,
            len,
        } => {
            rows.push(ScriptDisassemblyLine {
                script: script.to_string(),
                function_name: function_name.to_string(),
                offset: *offset,
                len: *len,
                opcode: format!("{opcode:02x}"),
                mnemonic: "pair_record".to_string(),
                operands: format!(
                    "ref=0x{record_offset:04x} first=0x{first_word:04x} second=0x{second_word:04x}"
                ),
                actor_record: current_actor_record(current_actor),
                text: None,
            });
            true
        }
        vm::VmToken::RecordTriple {
            offset,
            record_offset,
            first_word,
            second_word,
            inverted,
            len,
        } => {
            rows.push(ScriptDisassemblyLine {
                script: script.to_string(),
                function_name: function_name.to_string(),
                offset: *offset,
                len: *len,
                opcode: "cd".to_string(),
                mnemonic: "record_triple".to_string(),
                operands: format!(
                    "ref=0x{record_offset:04x} first=0x{first_word:04x} second=0x{second_word:04x} inverted={inverted}"
                ),
                actor_record: current_actor_record(current_actor),
                text: None,
            });
            true
        }
        vm::VmToken::RecordClear {
            offset,
            record_offset,
            len,
        } => {
            if matches!(current_actor.as_ref(), Some(actor) if actor.talk_ref == *record_offset) {
                *current_actor = None;
            }
            rows.push(ScriptDisassemblyLine {
                script: script.to_string(),
                function_name: function_name.to_string(),
                offset: *offset,
                len: *len,
                opcode: "c9".to_string(),
                mnemonic: "record_clear".to_string(),
                operands: format!("ref=0x{record_offset:04x}"),
                actor_record: None,
                text: None,
            });
            true
        }
        vm::VmToken::ScriptProfileRequest {
            offset,
            operand,
            profile_index,
            len,
        } => {
            rows.push(ScriptDisassemblyLine {
                script: script.to_string(),
                function_name: function_name.to_string(),
                offset: *offset,
                len: *len,
                opcode: "d2".to_string(),
                mnemonic: "script_profile_request".to_string(),
                operands: format!("operand={operand} profile_index=0x{profile_index:04x}"),
                actor_record: current_actor_record(current_actor),
                text: None,
            });
            true
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
            let Some(decoded_words) = decode_vm_words(words, word_offsets) else {
                return false;
            };
            let params = [*voice_selector, *flags_b4];
            rows.push(ScriptDisassemblyLine {
                script: script.to_string(),
                function_name: function_name.to_string(),
                offset: *offset,
                len: text_token_end(*offset, *flags_b4, *loop_target, word_offsets.len()) - offset,
                opcode: "a6".to_string(),
                mnemonic: "text_call".to_string(),
                operands: format!(
                    "target=0x{line_index:04x} params={} words={}",
                    hex_bytes(&params),
                    decoded_words.len()
                ),
                actor_record: current_actor_record(current_actor),
                text: Some(assemble_dialogue(&decoded_words)),
            });
            true
        }
        vm::VmToken::Op { .. } | vm::VmToken::Invalid { .. } => false,
    }
}

fn vm_token_offset(token: &vm::VmToken) -> usize {
    match token {
        vm::VmToken::Text { offset, .. }
        | vm::VmToken::Actor { offset, .. }
        | vm::VmToken::RecordLink { offset, .. }
        | vm::VmToken::RecordEntry { offset, .. }
        | vm::VmToken::RecordClear { offset, .. }
        | vm::VmToken::BitFlag { offset, .. }
        | vm::VmToken::RecordState { offset, .. }
        | vm::VmToken::GlobalWordCompare { offset, .. }
        | vm::VmToken::GlobalPairCompare { offset, .. }
        | vm::VmToken::PairRecord { offset, .. }
        | vm::VmToken::RecordTriple { offset, .. }
        | vm::VmToken::ScriptProfileRequest { offset, .. }
        | vm::VmToken::Op { offset, .. }
        | vm::VmToken::Invalid { offset, .. } => *offset,
    }
}

fn vm_token_len(token: &vm::VmToken) -> usize {
    match token {
        vm::VmToken::Text {
            offset,
            flags_b4,
            loop_target,
            word_offsets,
            ..
        } => text_token_end(*offset, *flags_b4, *loop_target, word_offsets.len()) - offset,
        vm::VmToken::Actor { len, .. }
        | vm::VmToken::RecordLink { len, .. }
        | vm::VmToken::RecordEntry { len, .. }
        | vm::VmToken::RecordClear { len, .. }
        | vm::VmToken::BitFlag { len, .. }
        | vm::VmToken::RecordState { len, .. }
        | vm::VmToken::GlobalWordCompare { len, .. }
        | vm::VmToken::GlobalPairCompare { len, .. }
        | vm::VmToken::PairRecord { len, .. }
        | vm::VmToken::RecordTriple { len, .. }
        | vm::VmToken::ScriptProfileRequest { len, .. }
        | vm::VmToken::Op { len, .. } => *len,
        vm::VmToken::Invalid { .. } => 1,
    }
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
    // 0x660C): `A6 b1 b2 b3 b4 b5 [loop:u16?] [control:u16?] w0 ... 0x0000`.
    // * b1:b2 = line-record index (call_target)
    // * b3 = params[0] (voice selector), b4 = params[1] (control flags)
    // * b5 (pos+5) bit7 = active/display flag (may be 0x80/0x90/0xA0/...)
    // * if b4 & 0x10 (loop), a u16 loop target precedes the word list.
    // * if b4 & 0x04, one extra u16 control word precedes the word list.
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
    if b4 & 0x04 != 0 {
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
        w.insert(0x0030, "extra".to_string());
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

    /// Control-word token (b4 & 0x04): a u16 control word precedes the word list
    /// and must not be read as a dictionary offset.
    #[test]
    fn decodes_control_word_token_skipping_extra_word() {
        let words = words_fixture();
        // control word 0x7777 is NOT a valid dict offset; old code returned None.
        let cod = [
            0xA6, 0x00, 0x00, 0xFF, 0x04, 0x80, 0x77, 0x77, 0x30, 0x00, 0x00, 0x00,
        ];
        let call = decode_text_call_at(&cod, cod.len(), &words, 0)
            .expect("control-word token should decode");
        assert_eq!(call.params, vec![0xFF, 0x04]); // b3=0xFF (no voice), b4=0x04
        assert_eq!(call.words, vec!["extra"]);
        assert_eq!(call.text_end, cod.len());
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
        "script\tfunction\toffset\tactor\tparam0\tparam1\tskip_count\tloop_target\tactive_line_id\tclip_index\tbackground_record\tbackground_hnm\tbackground_music\tsource\ttext\tcall_target\tparams_hex\ttext_end\tactor_ref\tactor_proof\tword_count"
    )?;
    for row in rows {
        writeln!(
            file,
            "{}\t{}\t0x{:05x}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t0x{:04x}\t{}\t0x{:05x}\t{}\t{}\t{}",
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
            row.skip_count
                .map(|count| count.to_string())
                .unwrap_or_default(),
            row.loop_target
                .map(|target| format!("0x{target:04x}"))
                .unwrap_or_default(),
            row.active_line_id
                .map(|active_line_id| format!("0x{active_line_id:04x}"))
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

pub(super) fn write_script_text_flags_manifest(
    rows: &[ScriptTextFlagLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "script\tfunction\toffset\tline_index\tvoice_selector\tactive_line_id\tflags_b4\tflags_b5\tactive\tskip_count\tloop_target\tsummary\ttext"
    )?;
    for row in rows {
        writeln!(
            file,
            "{}\t{}\t0x{:05x}\t0x{:04x}\t{:02x}\t0x{:04x}\t{:02x}\t{:02x}\t{}\t{}\t{}\t{}\t{}",
            row.script,
            row.function_name,
            row.offset,
            row.line_index,
            row.voice_selector,
            row.active_line_id,
            row.flags_b4,
            row.flags_b5,
            row.active,
            row.skip_count
                .map(|count| count.to_string())
                .unwrap_or_default(),
            row.loop_target
                .map(|target| format!("0x{target:04x}"))
                .unwrap_or_default(),
            clean_tsv(&row.summary),
            clean_tsv(&row.text),
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
        "script\tsequence_index\tfunction\toffset\tactor\tactor_ref\tlocation_offset\tbackground_record\tbackground_hnm\tbackground_music\tparam0\tparam1\tskip_count\tloop_target\tactive_line_id\tclip_index\tcall_target\ttext_end\tsource\ttext"
    )?;
    for row in rows {
        writeln!(
            file,
            "{}\t{}\t{}\t0x{:05x}\t{}\t{}\t{}\t{}\t{}\t{}\t{:02x}\t{:02x}\t{}\t{}\t0x{:04x}\t{}\t0x{:04x}\t0x{:05x}\t{}\t{}",
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
            row.skip_count
                .map(|count| count.to_string())
                .unwrap_or_default(),
            row.loop_target
                .map(|target| format!("0x{target:04x}"))
                .unwrap_or_default(),
            row.active_line_id,
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

pub(super) fn write_script_branch_scenario_speech_manifest(
    rows: &[ScriptExecutedSpeechLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "scenario_id\tscript\tsequence_index\tfunction\toffset\tactor\tactor_ref\tlocation_offset\tbackground_record\tbackground_hnm\tbackground_music\tparam0\tparam1\tskip_count\tloop_target\tactive_line_id\tclip_index\tcall_target\ttext_end\tsource\ttext"
    )?;
    for row in rows {
        writeln!(
            file,
            "{}\t{}\t{}\t{}\t0x{:05x}\t{}\t{}\t{}\t{}\t{}\t{}\t{:02x}\t{:02x}\t{}\t{}\t0x{:04x}\t{}\t0x{:04x}\t0x{:05x}\t{}\t{}",
            row.scenario_id.as_deref().unwrap_or(""),
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
            row.skip_count
                .map(|count| count.to_string())
                .unwrap_or_default(),
            row.loop_target
                .map(|target| format!("0x{target:04x}"))
                .unwrap_or_default(),
            row.active_line_id,
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

pub(super) fn write_script_profile_runs_manifest(
    rows: &[ScriptProfileRunLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "sequence_id\trun_index\tprofile_index\td2_operand\tscript\tsteps\ttext_calls\tpending_profile_index\tpending_script\tpending_dispatch_ready\tpost_update_pairs\tpresentation_handoffs\trequests\thalted_after_run"
    )?;
    for row in rows {
        writeln!(
            file,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.sequence_id,
            row.run_index,
            row.profile_index,
            row.d2_operand,
            row.script,
            row.steps,
            row.text_calls,
            row.pending_profile_index
                .map(|idx| idx.to_string())
                .unwrap_or_default(),
            row.pending_script.as_deref().unwrap_or(""),
            row.pending_dispatch_ready,
            clean_tsv(&row.post_update_pairs),
            clean_tsv(&row.presentation_handoffs),
            clean_tsv(&row.request_summary),
            clean_tsv(&row.halted_after_run),
        )?;
    }
    Ok(())
}

pub(super) fn write_script_profile_executed_speech_manifest(
    rows: &[ScriptProfileExecutedSpeechLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "sequence_id\tglobal_sequence_index\trun_index\tprofile_index\td2_operand\tscript\tscript_sequence_index\tfunction\toffset\tactor\tactor_ref\tlocation_offset\tbackground_record\tbackground_hnm\tbackground_music\tparam0\tparam1\tskip_count\tloop_target\tactive_line_id\tclip_index\tcall_target\ttext_end\tsource\ttext"
    )?;
    for row in rows {
        writeln!(
            file,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t0x{:05x}\t{}\t{}\t{}\t{}\t{}\t{}\t{:02x}\t{:02x}\t{}\t{}\t0x{:04x}\t{}\t0x{:04x}\t0x{:05x}\t{}\t{}",
            row.sequence_id,
            row.global_sequence_index,
            row.run_index,
            row.profile_index,
            row.d2_operand,
            row.row.script,
            row.script_sequence_index,
            row.row.function_name,
            row.row.offset,
            row.row.actor_record.as_deref().unwrap_or(""),
            row.row
                .actor_ref
                .map(|actor_ref| format!("0x{actor_ref:04x}"))
                .unwrap_or_default(),
            row.row
                .location_offset
                .map(|location_offset| format!("0x{location_offset:04x}"))
                .unwrap_or_default(),
            row.row.background_record.as_deref().unwrap_or(""),
            row.row.background_hnm.as_deref().unwrap_or(""),
            row.row.background_music.as_deref().unwrap_or(""),
            row.row.param0,
            row.row.param1,
            row.row
                .skip_count
                .map(|count| count.to_string())
                .unwrap_or_default(),
            row.row
                .loop_target
                .map(|target| format!("0x{target:04x}"))
                .unwrap_or_default(),
            row.row.active_line_id,
            row.row
                .clip_index
                .map(|idx| idx.to_string())
                .unwrap_or_default(),
            row.row.call_target,
            row.row.text_end,
            clean_tsv(&row.row.source),
            clean_tsv(&row.row.text),
        )?;
    }
    Ok(())
}

pub(super) fn write_script_profile_dialogue_runs_manifest(
    rows: &[ScriptProfileExecutedSpeechLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let runs = script_profile_dialogue_runs(rows);
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "sequence_id\trun_id\tmp4\tfirst_global_sequence\tlast_global_sequence\tfirst_profile_index\tlast_profile_index\tfirst_script\tlast_script\tbackground_record\tbackground_hnm\tbackground_music\tline_count\tvoiced_count\tactors\tclip_refs\tfirst_text\tunresolved_actor_count\tunresolved_background_count\tunresolved_voice_count"
    )?;
    for run in runs {
        let run_id = profile_dialogue_run_id(&run);
        let output_stem = profile_dialogue_run_output_stem(&run);
        let coverage = executed_run_coverage(run.lines.iter().map(|line| &line.row));
        let actors = unique_join(
            run.lines
                .iter()
                .filter_map(|line| line.row.actor_record.as_deref()),
        );
        let clip_refs = run
            .lines
            .iter()
            .filter_map(|line| {
                line.row.clip_index.map(|clip| {
                    format!(
                        "{}:{clip}",
                        line.row.actor_record.as_deref().unwrap_or("noactor")
                    )
                })
            })
            .collect::<Vec<_>>()
            .join(",");
        let voiced_count = run
            .lines
            .iter()
            .filter(|line| line.row.clip_index.is_some())
            .count();
        let first_text = run
            .lines
            .first()
            .map(|line| clean_tsv(&line.row.text))
            .unwrap_or_default();
        writeln!(
            file,
            "{}\t{}\t{}.mp4\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            run.sequence_id,
            run_id,
            output_stem,
            run.first_global_sequence,
            run.last_global_sequence,
            run.first_profile_index,
            run.last_profile_index,
            run.first_script,
            run.last_script,
            run.background_record.as_deref().unwrap_or(""),
            run.background_hnm.as_deref().unwrap_or(""),
            run.background_music.as_deref().unwrap_or(""),
            run.lines.len(),
            voiced_count,
            actors,
            clip_refs,
            first_text,
            coverage.unresolved_actor_count,
            coverage.unresolved_background_count,
            coverage.unresolved_voice_count
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

pub(super) fn write_script_post_update_manifest(
    rows: &[ScriptPostUpdateLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "script\tevent_index\tevent_kind\trecord_offset\trelated_record_offset\towner_offset\ttarget\tready"
    )?;
    for row in rows {
        writeln!(
            file,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.script,
            row.event_index,
            clean_tsv(&row.event_kind),
            row.record_offset
                .map(|offset| format!("0x{offset:04x}"))
                .unwrap_or_default(),
            row.related_record_offset
                .map(|offset| format!("0x{offset:04x}"))
                .unwrap_or_default(),
            row.owner_offset
                .map(|offset| format!("0x{offset:04x}"))
                .unwrap_or_default(),
            row.target
                .map(|target| format!("0x{target:04x}"))
                .unwrap_or_default(),
            row.ready.map(|ready| ready.to_string()).unwrap_or_default(),
        )?;
    }
    Ok(())
}

pub(super) fn write_script_branch_decisions_manifest(
    rows: &[ScriptBranchTraceLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "script\tdecision_index\toffset\topcode\tcondition_passed\tobserved_path\tobserved_target\talternate_path\talternate_target\tstack_depth\tdetail"
    )?;

    let mut decision_index_by_script: BTreeMap<&str, usize> = BTreeMap::new();
    for row in rows.iter().filter(|row| row.condition_passed.is_some()) {
        let decision_index = decision_index_by_script
            .entry(row.script.as_str())
            .and_modify(|idx| *idx += 1)
            .or_insert(1);
        let (observed_path, observed_target, alternate_path, alternate_target) = if row.branch_taken
        {
            (
                "jump",
                row.target
                    .map(|target| format!("0x{target:04x}"))
                    .unwrap_or_default(),
                "fallthrough",
                String::new(),
            )
        } else {
            (
                "fallthrough",
                String::new(),
                "jump",
                row.target
                    .map(|target| format!("0x{target:04x}"))
                    .unwrap_or_default(),
            )
        };
        writeln!(
            file,
            "{}\t{}\t0x{:05x}\t{:02x}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.script,
            decision_index,
            row.offset,
            row.opcode,
            row.condition_passed.unwrap(),
            observed_path,
            observed_target,
            alternate_path,
            alternate_target,
            row.stack_depth,
            clean_tsv(&row.detail),
        )?;
    }
    Ok(())
}

pub(super) fn write_script_branch_coverage_manifest(
    speech_rows: &[ScriptSpeechLine],
    executed_rows: &[ScriptExecutedSpeechLine],
    branch_rows: &[ScriptBranchTraceLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut scripts: BTreeSet<&str> = BTreeSet::new();
    scripts.extend(speech_rows.iter().map(|row| row.script.as_str()));
    scripts.extend(executed_rows.iter().map(|row| row.script.as_str()));
    scripts.extend(branch_rows.iter().map(|row| row.script.as_str()));

    let runs = script_executed_dialogue_runs(executed_rows);
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "script\tstatic_text_calls\texecuted_text_calls\tunexecuted_text_calls\texecuted_percent\tbranch_events\tdecisions\tpassed_decisions\tfailed_decisions\tbranches_taken\texecuted_dialogue_runs"
    )?;

    for script in scripts {
        let static_text_calls = speech_rows
            .iter()
            .filter(|row| row.script == script)
            .count();
        let executed_text_calls = executed_rows
            .iter()
            .filter(|row| row.script == script)
            .count();
        let unexecuted_text_calls = static_text_calls.saturating_sub(executed_text_calls);
        let executed_percent = if static_text_calls == 0 {
            0.0
        } else {
            executed_text_calls as f64 * 100.0 / static_text_calls as f64
        };
        let script_branches: Vec<&ScriptBranchTraceLine> = branch_rows
            .iter()
            .filter(|row| row.script == script)
            .collect();
        let decisions = script_branches
            .iter()
            .filter(|row| row.condition_passed.is_some())
            .count();
        let passed_decisions = script_branches
            .iter()
            .filter(|row| row.condition_passed == Some(true))
            .count();
        let failed_decisions = script_branches
            .iter()
            .filter(|row| row.condition_passed == Some(false))
            .count();
        let branches_taken = script_branches
            .iter()
            .filter(|row| row.branch_taken)
            .count();
        let executed_dialogue_runs = runs.iter().filter(|run| run.script == script).count();
        writeln!(
            file,
            "{}\t{}\t{}\t{}\t{:.2}\t{}\t{}\t{}\t{}\t{}\t{}",
            script,
            static_text_calls,
            executed_text_calls,
            unexecuted_text_calls,
            executed_percent,
            script_branches.len(),
            decisions,
            passed_decisions,
            failed_decisions,
            branches_taken,
            executed_dialogue_runs
        )?;
    }
    Ok(())
}

pub(super) fn write_script_branch_scenarios_manifest(
    rows: &[ScriptBranchScenarioLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "script\tscenario_id\tscenario_kind\tdecision_index\tforced_offset\topcode\tdefault_condition_passed\tforced_condition_passed\trtc_hour\trtc_month\trtc_day\tdefault_text_calls\tscenario_text_calls\tnew_text_calls\tlost_text_calls\tfirst_new_offsets\thalted\tsteps"
    )?;
    for row in rows {
        writeln!(
            file,
            "{}\t{}\t{}\t{}\t0x{:05x}\t{:02x}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.script,
            row.scenario_id,
            row.scenario_kind,
            row.decision_index,
            row.forced_offset,
            row.opcode,
            row.default_condition_passed,
            row.forced_condition_passed,
            row.rtc_hour
                .map(|hour| hour.to_string())
                .unwrap_or_default(),
            row.rtc_month
                .map(|month| month.to_string())
                .unwrap_or_default(),
            row.rtc_day.map(|day| day.to_string()).unwrap_or_default(),
            row.default_text_calls,
            row.scenario_text_calls,
            row.new_text_calls,
            row.lost_text_calls,
            row.first_new_offsets,
            clean_tsv(&row.halted),
            row.steps,
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
    pub(super) scenario_id: Option<String>,
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

#[derive(Debug)]
pub(super) struct ScriptProfileDialogueRun<'a> {
    pub(super) sequence_id: String,
    pub(super) run_index: usize,
    pub(super) first_global_sequence: usize,
    pub(super) last_global_sequence: usize,
    pub(super) first_profile_index: u16,
    pub(super) last_profile_index: u16,
    pub(super) first_script: String,
    pub(super) last_script: String,
    pub(super) background_record: Option<String>,
    pub(super) background_hnm: Option<String>,
    pub(super) background_music: Option<String>,
    pub(super) lines: Vec<&'a ScriptProfileExecutedSpeechLine>,
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
        (
            a.scenario_id.as_deref().unwrap_or(""),
            a.script.as_str(),
            a.sequence_index,
        )
            .cmp(&(
                b.scenario_id.as_deref().unwrap_or(""),
                b.script.as_str(),
                b.sequence_index,
            ))
    });

    let mut runs: Vec<ScriptExecutedDialogueRun<'_>> = Vec::new();
    for row in ordered {
        let same_run = runs.last().is_some_and(|run| {
            run.scenario_id == row.scenario_id
                && run.script == row.script
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

        let run_index = runs
            .iter()
            .filter(|run| run.scenario_id == row.scenario_id && run.script == row.script)
            .count()
            + 1;
        runs.push(ScriptExecutedDialogueRun {
            scenario_id: row.scenario_id.clone(),
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

pub(super) fn script_profile_dialogue_runs(
    rows: &[ScriptProfileExecutedSpeechLine],
) -> Vec<ScriptProfileDialogueRun<'_>> {
    let mut ordered: Vec<&ScriptProfileExecutedSpeechLine> = rows
        .iter()
        .filter(|row| row.row.clip_index.is_some() || !row.row.text.trim().is_empty())
        .collect();
    ordered.sort_by(|a, b| {
        (a.sequence_id.as_str(), a.global_sequence_index)
            .cmp(&(b.sequence_id.as_str(), b.global_sequence_index))
    });

    let mut runs: Vec<ScriptProfileDialogueRun<'_>> = Vec::new();
    for row in ordered {
        let same_run = runs.last().is_some_and(|run| {
            run.sequence_id == row.sequence_id
                && run.background_record == row.row.background_record
                && run.background_hnm == row.row.background_hnm
                && run.background_music == row.row.background_music
        });
        if same_run {
            let run = runs.last_mut().expect("run exists");
            run.last_global_sequence = row.global_sequence_index;
            run.last_profile_index = row.profile_index;
            run.last_script = row.row.script.clone();
            run.lines.push(row);
            continue;
        }

        let run_index = runs
            .iter()
            .filter(|run| run.sequence_id == row.sequence_id)
            .count()
            + 1;
        runs.push(ScriptProfileDialogueRun {
            sequence_id: row.sequence_id.clone(),
            run_index,
            first_global_sequence: row.global_sequence_index,
            last_global_sequence: row.global_sequence_index,
            first_profile_index: row.profile_index,
            last_profile_index: row.profile_index,
            first_script: row.row.script.clone(),
            last_script: row.row.script.clone(),
            background_record: row.row.background_record.clone(),
            background_hnm: row.row.background_hnm.clone(),
            background_music: row.row.background_music.clone(),
            lines: vec![row],
        });
    }
    runs
}

fn executed_dialogue_run_id(run: &ScriptExecutedDialogueRun<'_>) -> String {
    if let Some(scenario_id) = &run.scenario_id {
        format!("{scenario_id}-run-{:04}", run.run_index)
    } else {
        format!("{}-{:04}", run.script, run.run_index)
    }
}

pub(super) fn profile_dialogue_run_id(run: &ScriptProfileDialogueRun<'_>) -> String {
    format!("{}-profile-run-{:04}", run.sequence_id, run.run_index)
}

#[derive(Clone, Copy, Debug, Default)]
struct RunCoverage {
    unresolved_actor_count: usize,
    unresolved_background_count: usize,
    unresolved_voice_count: usize,
}

fn speech_run_coverage<'a>(lines: impl IntoIterator<Item = &'a ScriptSpeechLine>) -> RunCoverage {
    let mut coverage = RunCoverage::default();
    for line in lines {
        if line.actor_record.is_none() {
            coverage.unresolved_actor_count += 1;
        }
        if line.background_record.is_none() && line.background_hnm.is_none() {
            coverage.unresolved_background_count += 1;
        }
        if line.actor_record.is_some()
            && line.clip_index.is_none()
            && line.param0.is_some_and(vm::text_selector_requests_voice)
            && line.param1.is_some_and(|flags| flags < 0x10)
        {
            coverage.unresolved_voice_count += 1;
        }
    }
    coverage
}

fn executed_run_coverage<'a>(
    lines: impl IntoIterator<Item = &'a ScriptExecutedSpeechLine>,
) -> RunCoverage {
    let mut coverage = RunCoverage::default();
    for line in lines {
        if line.actor_record.is_none() {
            coverage.unresolved_actor_count += 1;
        }
        if line.background_record.is_none() && line.background_hnm.is_none() {
            coverage.unresolved_background_count += 1;
        }
        if line.actor_record.is_some()
            && line.clip_index.is_none()
            && vm::text_selector_requests_voice(line.param0)
            && line.param1 < 0x10
        {
            coverage.unresolved_voice_count += 1;
        }
    }
    coverage
}

pub(super) fn executed_dialogue_run_output_stem(run: &ScriptExecutedDialogueRun<'_>) -> String {
    let location = run
        .background_record
        .as_deref()
        .or(run.background_hnm.as_deref())
        .unwrap_or("nolocation");
    if let Some(scenario_id) = &run.scenario_id {
        // Static uncovered-function scenes are tagged `fn:<script>:<function>`.
        if let Some(rest) = scenario_id.strip_prefix("fn:") {
            return format!(
                "function-dialogue-run - {} - {:04} - {}",
                safe_file_stem(&rest.replace(':', "-")),
                run.run_index,
                safe_file_stem(location)
            );
        }
        format!(
            "branch-scenario-dialogue-run - {} - {:04} - {}",
            safe_file_stem(scenario_id),
            run.run_index,
            safe_file_stem(location)
        )
    } else {
        format!(
            "executed-dialogue-run - {} - {:04} - {}",
            safe_file_stem(&run.script),
            run.run_index,
            safe_file_stem(location)
        )
    }
}

pub(super) fn profile_dialogue_run_output_stem(run: &ScriptProfileDialogueRun<'_>) -> String {
    let location = run
        .background_record
        .as_deref()
        .or(run.background_hnm.as_deref())
        .unwrap_or("nolocation");
    format!(
        "profile-dialogue-run - {} - {:04} - {}",
        safe_file_stem(&run.sequence_id),
        run.run_index,
        safe_file_stem(location)
    )
}

fn executed_line_input(row: &ScriptExecutedSpeechLine) -> vm::LineInput {
    vm::LineInput {
        actor: row.actor_record.clone(),
        background_hnm: row.background_hnm.clone(),
        background_record: row.background_record.clone(),
        background_music: row.background_music.clone(),
        voice_selector: row.param0,
        active_line_id: row.active_line_id,
        flags_b4: row.param1,
        skip_count: row.skip_count,
        loop_target: row.loop_target,
        clip_index: row.clip_index,
        text: row.text.clone(),
    }
}

fn scene_event_kind(event: &vm::SceneEvent) -> &'static str {
    match event {
        vm::SceneEvent::SetBackground { .. } => "set_background",
        vm::SceneEvent::PlayMusic { .. } => "play_music",
        vm::SceneEvent::ShowSpeaker { .. } => "show_speaker",
        vm::SceneEvent::PlayTalkHnm { .. } => "play_talk_hnm",
        vm::SceneEvent::PlayVoice { .. } => "play_voice",
        vm::SceneEvent::DrawSubtitle { .. } => "draw_subtitle",
        vm::SceneEvent::PlayChatter { .. } => "play_chatter",
        vm::SceneEvent::UnresolvedBackground { .. } => "unresolved_background",
        vm::SceneEvent::UnresolvedActor { .. } => "unresolved_actor",
        vm::SceneEvent::UnresolvedVoice { .. } => "unresolved_voice",
        vm::SceneEvent::Clear => "clear",
    }
}

fn format_scene_event_fields(
    event: &vm::SceneEvent,
    source: Option<&ScriptExecutedSpeechLine>,
) -> (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
) {
    let mut actor = source
        .and_then(|line| line.actor_record.clone())
        .unwrap_or_default();
    let mut background_record = source
        .and_then(|line| line.background_record.clone())
        .unwrap_or_default();
    let mut background_hnm = source
        .and_then(|line| line.background_hnm.clone())
        .unwrap_or_default();
    let mut background_music = source
        .and_then(|line| line.background_music.clone())
        .unwrap_or_default();
    let mut clip_index = String::new();
    let mut voice_selector = String::new();
    let mut active_line_id = String::new();
    let mut flags_b4 = String::new();
    let mut skip_count = String::new();
    let mut loop_target = String::new();
    let mut text = String::new();
    let source_detail = source
        .map(|line| clean_tsv(&line.source))
        .unwrap_or_default();

    match event {
        vm::SceneEvent::SetBackground { hnm, record } => {
            background_record = record.clone().unwrap_or_default();
            background_hnm = hnm.clone().unwrap_or_default();
        }
        vm::SceneEvent::PlayMusic { music } => {
            background_music = music.clone().unwrap_or_default();
        }
        vm::SceneEvent::ShowSpeaker { actor: event_actor } => {
            actor = event_actor.clone();
        }
        vm::SceneEvent::PlayTalkHnm {
            clip_index: event_clip,
        }
        | vm::SceneEvent::PlayVoice {
            clip_index: event_clip,
        } => {
            clip_index = event_clip.to_string();
        }
        vm::SceneEvent::DrawSubtitle {
            text: event_text,
            voice_selector: event_voice_selector,
            active_line_id: event_active_line_id,
            flags,
            skip_count: event_skip_count,
            loop_target: event_loop_target,
        } => {
            voice_selector = format!("{event_voice_selector:02x}");
            active_line_id = format!("0x{event_active_line_id:04x}");
            flags_b4 = format!("{flags:02x}");
            skip_count = event_skip_count
                .map(|count| count.to_string())
                .unwrap_or_default();
            loop_target = event_loop_target
                .map(|target| format!("0x{target:04x}"))
                .unwrap_or_default();
            text = clean_tsv(event_text);
        }
        vm::SceneEvent::PlayChatter {
            active_line_id: event_active_line_id,
        } => {
            active_line_id = format!("0x{event_active_line_id:04x}");
        }
        vm::SceneEvent::UnresolvedBackground {
            active_line_id: event_active_line_id,
        }
        | vm::SceneEvent::UnresolvedActor {
            active_line_id: event_active_line_id,
        } => {
            active_line_id = format!("0x{event_active_line_id:04x}");
        }
        vm::SceneEvent::UnresolvedVoice {
            voice_selector: event_voice_selector,
            active_line_id: event_active_line_id,
        } => {
            voice_selector = format!("{event_voice_selector:02x}");
            active_line_id = format!("0x{event_active_line_id:04x}");
        }
        vm::SceneEvent::Clear => {}
    }

    (
        actor,
        background_record,
        background_hnm,
        background_music,
        clip_index,
        voice_selector,
        active_line_id,
        flags_b4,
        skip_count,
        loop_target,
        text,
        source_detail,
    )
}

pub(super) fn write_script_scene_events_manifest(
    rows: &[ScriptExecutedSpeechLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let runs = script_executed_dialogue_runs(rows);
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "scenario_id\trun_id\tmp4\tscript\trun_index\tevent_index\tevent_kind\tsequence_index\toffset\tactor\tbackground_record\tbackground_hnm\tbackground_music\tclip_index\tvoice_selector\tactive_line_id\tflags_b4\tskip_count\tloop_target\ttext\tsource"
    )?;
    for run in runs {
        let run_id = executed_dialogue_run_id(&run);
        let output_stem = executed_dialogue_run_output_stem(&run);
        let inputs = run
            .lines
            .iter()
            .map(|line| executed_line_input(line))
            .collect::<Vec<_>>();
        let events = vm::emit_scene_events(&inputs);
        let mut line_index = 0usize;

        for (event_index, event) in events.iter().enumerate() {
            let source = if matches!(event, vm::SceneEvent::Clear) {
                None
            } else {
                run.lines.get(line_index).copied()
            };
            let (
                actor,
                background_record,
                background_hnm,
                background_music,
                clip_index,
                voice_selector,
                active_line_id,
                flags_b4,
                skip_count,
                loop_target,
                text,
                source_detail,
            ) = format_scene_event_fields(event, source);
            writeln!(
                file,
                "{}\t{}\t{}.mp4\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                run.scenario_id.as_deref().unwrap_or(""),
                run_id,
                output_stem,
                run.script,
                run.run_index,
                event_index,
                scene_event_kind(event),
                source
                    .map(|line| line.sequence_index.to_string())
                    .unwrap_or_default(),
                source
                    .map(|line| format!("0x{:05x}", line.offset))
                    .unwrap_or_default(),
                actor,
                background_record,
                background_hnm,
                background_music,
                clip_index,
                voice_selector,
                active_line_id,
                flags_b4,
                skip_count,
                loop_target,
                text,
                source_detail,
            )?;
            if matches!(event, vm::SceneEvent::PlayChatter { .. }) {
                line_index += 1;
            }
        }
    }
    Ok(())
}

pub(super) fn write_script_profile_scene_events_manifest(
    rows: &[ScriptProfileExecutedSpeechLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let runs = script_profile_dialogue_runs(rows);
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "sequence_id\trun_id\tmp4\trun_index\tfirst_profile_index\tlast_profile_index\tevent_index\tevent_kind\tglobal_sequence_index\tprofile_index\td2_operand\tscript\tscript_sequence_index\toffset\tactor\tbackground_record\tbackground_hnm\tbackground_music\tclip_index\tvoice_selector\tactive_line_id\tflags_b4\tskip_count\tloop_target\ttext\tsource"
    )?;
    for run in runs {
        let run_id = profile_dialogue_run_id(&run);
        let output_stem = profile_dialogue_run_output_stem(&run);
        let inputs = run
            .lines
            .iter()
            .map(|line| executed_line_input(&line.row))
            .collect::<Vec<_>>();
        let events = vm::emit_scene_events(&inputs);
        let mut line_index = 0usize;

        for (event_index, event) in events.iter().enumerate() {
            let source = if matches!(event, vm::SceneEvent::Clear) {
                None
            } else {
                run.lines.get(line_index).copied()
            };
            let (
                actor,
                background_record,
                background_hnm,
                background_music,
                clip_index,
                voice_selector,
                active_line_id,
                flags_b4,
                skip_count,
                loop_target,
                text,
                source_detail,
            ) = format_scene_event_fields(event, source.map(|line| &line.row));
            writeln!(
                file,
                "{}\t{}\t{}.mp4\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                run.sequence_id,
                run_id,
                output_stem,
                run.run_index,
                run.first_profile_index,
                run.last_profile_index,
                event_index,
                scene_event_kind(event),
                source
                    .map(|line| line.global_sequence_index.to_string())
                    .unwrap_or_default(),
                source
                    .map(|line| line.profile_index.to_string())
                    .unwrap_or_default(),
                source
                    .map(|line| line.d2_operand.to_string())
                    .unwrap_or_default(),
                source.map(|line| line.row.script.as_str()).unwrap_or(""),
                source
                    .map(|line| line.script_sequence_index.to_string())
                    .unwrap_or_default(),
                source
                    .map(|line| format!("0x{:05x}", line.row.offset))
                    .unwrap_or_default(),
                actor,
                background_record,
                background_hnm,
                background_music,
                clip_index,
                voice_selector,
                active_line_id,
                flags_b4,
                skip_count,
                loop_target,
                text,
                source_detail,
            )?;
            if matches!(event, vm::SceneEvent::PlayChatter { .. }) {
                line_index += 1;
            }
        }
    }
    Ok(())
}

pub(super) fn write_script_executed_dialogue_runs_manifest(
    rows: &[ScriptExecutedSpeechLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let runs = script_executed_dialogue_runs(rows);
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "run_id\tmp4\tscript\tfirst_sequence\tlast_sequence\tfirst_offset\tlast_offset\tbackground_record\tbackground_hnm\tbackground_music\tline_count\tvoiced_count\tactors\tclip_refs\tfirst_text\tunresolved_actor_count\tunresolved_background_count\tunresolved_voice_count"
    )?;
    for run in runs {
        let run_id = executed_dialogue_run_id(&run);
        let output_stem = executed_dialogue_run_output_stem(&run);
        let coverage = executed_run_coverage(run.lines.iter().copied());
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
            "{}\t{}.mp4\t{}\t{}\t{}\t0x{:05x}\t0x{:05x}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
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
            first_text,
            coverage.unresolved_actor_count,
            coverage.unresolved_background_count,
            coverage.unresolved_voice_count
        )?;
    }
    Ok(())
}

pub(super) fn write_script_branch_scenario_dialogue_runs_manifest(
    rows: &[ScriptExecutedSpeechLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let runs = script_executed_dialogue_runs(rows);
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "scenario_id\trun_id\tmp4\tscript\tfirst_sequence\tlast_sequence\tfirst_offset\tlast_offset\tbackground_record\tbackground_hnm\tbackground_music\tline_count\tvoiced_count\tactors\tclip_refs\tfirst_text\tunresolved_actor_count\tunresolved_background_count\tunresolved_voice_count"
    )?;
    for run in runs {
        let run_id = executed_dialogue_run_id(&run);
        let output_stem = executed_dialogue_run_output_stem(&run);
        let coverage = executed_run_coverage(run.lines.iter().copied());
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
            "{}\t{}\t{}.mp4\t{}\t{}\t{}\t0x{:05x}\t0x{:05x}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            run.scenario_id.as_deref().unwrap_or(""),
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
            first_text,
            coverage.unresolved_actor_count,
            coverage.unresolved_background_count,
            coverage.unresolved_voice_count
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
        "run_id\tmp4\tscript\tfirst_offset\tlast_offset\tbackground_record\tbackground_hnm\tbackground_music\tline_count\tvoiced_count\tactors\tclip_refs\tfirst_text\tunresolved_actor_count\tunresolved_background_count\tunresolved_voice_count"
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
        let coverage = speech_run_coverage(run.lines.iter().copied());
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
            "{}\t{}.mp4\t{}\t0x{:05x}\t0x{:05x}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
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
            first_text,
            coverage.unresolved_actor_count,
            coverage.unresolved_background_count,
            coverage.unresolved_voice_count
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

#[cfg(test)]
fn write_legacy_script_dialogue_manifest(
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
        (a.0.0.as_str(), a.0.2.as_str(), oa).cmp(&(b.0.0.as_str(), b.0.2.as_str(), ob))
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
            skip_count: None,
            loop_target: None,
            active_line_id: Some(vm::text_selector_active_line_id(1)),
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
            scenario_id: None,
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
            skip_count: None,
            loop_target: None,
            active_line_id: vm::text_selector_active_line_id(1),
            clip_index: actor.map(|_| 0),
            text: text.to_string(),
            call_target: 0x1234,
            text_end: offset + 12,
            source: "test".to_string(),
        }
    }

    fn script_resource_profile(profile_index: u8, script_number: u8) -> ScriptResourceProfile {
        let extensions = ["cod", "bas", "var", "dic", "deb"];
        ScriptResourceProfile {
            profile_index,
            d2_operand: script_number,
            script_number,
            slots: extensions
                .iter()
                .enumerate()
                .map(|(slot, extension)| ScriptResourceProfileSlot {
                    slot,
                    resource_id: slot as u16,
                    name: format!("script{script_number}.{extension}"),
                })
                .collect(),
        }
    }

    fn profile_speech_line(
        global_sequence_index: usize,
        profile_index: u16,
        script: &str,
        script_sequence_index: usize,
        offset: usize,
        actor: Option<&str>,
        location: Option<&str>,
        text: &str,
    ) -> ScriptProfileExecutedSpeechLine {
        ScriptProfileExecutedSpeechLine {
            sequence_id: "default".to_string(),
            global_sequence_index,
            run_index: profile_index as usize,
            profile_index,
            d2_operand: profile_index as u8 + 1,
            script_sequence_index,
            row: executed_speech_line(script, script_sequence_index, offset, actor, location, text),
        }
    }

    fn manifest_row<'a>(manifest: &'a str, prefix: &str) -> Vec<&'a str> {
        manifest
            .lines()
            .find(|line| line.starts_with(prefix))
            .unwrap_or_else(|| panic!("manifest row with prefix {prefix:?}"))
            .split('\t')
            .collect()
    }

    fn branch_trace_line(
        script: &str,
        event_index: usize,
        offset: usize,
        opcode: u8,
        target: Option<u16>,
        branch_taken: bool,
        condition_passed: Option<bool>,
        detail: &str,
    ) -> ScriptBranchTraceLine {
        ScriptBranchTraceLine {
            script: script.to_string(),
            event_index,
            offset,
            opcode,
            target,
            branch_taken,
            condition_passed,
            stack_depth: 1,
            detail: detail.to_string(),
        }
    }

    fn synthetic_branch_script_dir() -> (PathBuf, usize) {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "commander-blood-branch-scenarios-{}-{nonce}",
            std::process::id(),
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create scenario script dir");

        let mut cod = Vec::new();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        cod.push(0xC0);
        cod.extend_from_slice(&0x0010u16.to_le_bytes());
        cod.push(0xF5);
        cod.push(0xC1);
        cod.extend_from_slice(&0x2222u16.to_le_bytes());
        cod.extend_from_slice(&[0xA6, 0x01, 0x00, 0xff, 0x00, 0x80]);
        cod.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[1..3].copy_from_slice(&target.to_le_bytes());
        cod.extend_from_slice(&[0xA6, 0x02, 0x00, 0xff, 0x00, 0x80]);
        cod.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
        cod.push(0xff);

        fs::write(root.join("SCRIPT1.COD"), cod).expect("write cod");
        let mut var = vec![0; 0x20];
        var[0x10] = 0x11;
        var[0x11] = 0x11;
        fs::write(root.join("SCRIPT1.VAR"), var).expect("write var");
        fs::write(root.join("SCRIPT1.DIC"), b"\0hello\0").expect("write dic");
        (root, condition_offset)
    }

    fn synthetic_rtc_script_dir() -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "commander-blood-rtc-scenarios-{}-{nonce}",
            std::process::id(),
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create rtc scenario script dir");

        let mut cod = Vec::new();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        cod.push(vm::OP_GLOBAL_WORD_COMPARE);
        cod.push(0xF1);
        cod.push(0xC1);
        cod.extend_from_slice(&8u16.to_le_bytes());
        cod.extend_from_slice(&[0xA6, 0x01, 0x00, 0xff, 0x00, 0x80]);
        cod.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[1..3].copy_from_slice(&target.to_le_bytes());
        cod.extend_from_slice(&[0xA6, 0x02, 0x00, 0xff, 0x00, 0x80]);
        cod.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
        cod.push(0xff);

        fs::write(root.join("SCRIPT1.COD"), cod).expect("write cod");
        fs::write(root.join("SCRIPT1.VAR"), vec![0; 0x20]).expect("write var");
        fs::write(root.join("SCRIPT1.DIC"), b"\0hello\0").expect("write dic");
        root
    }

    #[test]
    fn deb_context_marks_blood_as_special_object() {
        fn push_deb_record(deb: &mut Vec<u8>, name: &[u8], offset: u16, kind: u16) {
            let mut record = [0u8; 20];
            record[..name.len()].copy_from_slice(name);
            record[16..18].copy_from_slice(&offset.to_le_bytes());
            record[18..20].copy_from_slice(&kind.to_le_bytes());
            deb.extend_from_slice(&record);
        }

        let field = 0x0010u16;
        let blood = 0x0100u16;
        let arche = 0x0200u16;
        let scruter_jo = 0x0300u16;
        let vbio = 0x0400u16;
        let mut deb = Vec::new();
        push_deb_record(&mut deb, b"blood", blood, 1);
        push_deb_record(&mut deb, b"arche", arche, 1);
        push_deb_record(&mut deb, b"Scruter_Jo", scruter_jo, 1);
        push_deb_record(&mut deb, b"vbio", vbio, 5);

        let context = vm_execution_context_from_deb(&deb, None);
        assert_eq!(context.vm_named_object_offsets().blood, Some(blood));
        assert_eq!(context.vm_named_object_offsets().arche, Some(arche));
        assert_eq!(
            context.vm_named_object_offsets().scruter_jo,
            Some(scruter_jo)
        );
        assert_eq!(context.vm_named_object_offsets().vbio, Some(vbio));
        let mut var = vec![0; 0x0200];
        var[field as usize..field as usize + 2].copy_from_slice(&0xffffu16.to_le_bytes());

        let mut cod = Vec::new();
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        cod.push(0xAF);
        cod.extend_from_slice(&field.to_le_bytes());
        cod.extend_from_slice(&blood.to_le_bytes());
        let first_text = cod.len();
        cod.extend_from_slice(&[0xA6, 0x01, 0x00, 0xff, 0x00, 0x80]);
        cod.extend_from_slice(&0u16.to_le_bytes());
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        cod.extend_from_slice(&[0xA6, 0x02, 0x00, 0xff, 0x00, 0x80]);
        cod.extend_from_slice(&0u16.to_le_bytes());
        cod.push(0xff);

        let trace = vm::execute_trace_with_context(&cod, &var, &context);
        assert_eq!(trace.halted, vm::ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == 0xAF
                && !event.branch_taken
                && event.condition_passed == Some(true)
        }));
    }

    #[test]
    fn descript_context_enables_c2_kind400_descriptor_branch() {
        fn push_word_equals(cod: &mut Vec<u8>, addr: u16, value: u16) {
            cod.push(0xB1);
            cod.extend_from_slice(&addr.to_le_bytes());
            cod.push(0xF5);
            cod.push(0x00);
            cod.extend_from_slice(&value.to_le_bytes());
        }

        let db = DescriptDb {
            records: vec![DescriptRecord {
                name: "PRESENTE".to_string(),
                kind: 1,
                music: Vec::new(),
                full_hnms: Vec::new(),
                backgrounds: Vec::new(),
                sequence_hnms: Vec::new(),
                idle_hnms: Vec::new(),
                talk_hnms: Vec::new(),
                snd: None,
                sprite: None,
                labels: Vec::new(),
                subtitles: Vec::new(),
            }],
        };
        let owner = 0x0100u16;
        let record = owner + SCRIPT_OBJECT_TALK_FIELD;
        let target_record = 0x0200u16;
        let mut object_names = HashMap::new();
        object_names.insert(owner, "actor".to_string());
        let context = vm_execution_context_from_object_names(&object_names, Some(&db));

        let mut var = vec![0; 0x7000];
        var[owner as usize + 2] = 1;
        var[target_record as usize..target_record as usize + 2]
            .copy_from_slice(&0x0400u16.to_le_bytes());
        var[target_record as usize + 2] = 0x20;
        let name = b"PRESENTE";
        let name_start = target_record as usize + 4;
        var[name_start..name_start + name.len()].copy_from_slice(name);

        let mut cod = Vec::new();
        cod.push(vm::OP_RECORD_STATE_MAX);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&target_record.to_le_bytes());
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        push_word_equals(&mut cod, 0x6788, 0x002B);
        let first_text = cod.len();
        cod.extend_from_slice(&[0xA6, 0x01, 0x00, 0xff, 0x00, 0x80]);
        cod.extend_from_slice(&0u16.to_le_bytes());
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        cod.extend_from_slice(&[0xA6, 0x02, 0x00, 0xff, 0x00, 0x80]);
        cod.extend_from_slice(&0u16.to_le_bytes());
        cod.push(0xff);

        let trace = vm::execute_trace_with_context(&cod, &var, &context);
        assert_eq!(trace.halted, vm::ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == 0xB1
                && !event.branch_taken
                && event.condition_passed == Some(true)
        }));
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

        let cleared = [
            0xc4, 0x3a, 0x00, 0x28, 0x00, 0xc9, 0x3a, 0x00, 0xc3, 0x3a, 0x00, 0x28, 0x00, 0xa6,
            0x34, 0x12, 0xff, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00,
        ];
        let functions = vec![(0, "func".to_string()), (cleared.len(), "END".to_string())];
        let rows = parse_script_text_calls(
            "SCRIPTX",
            &cleared,
            &words,
            &functions,
            &actors,
            &HashMap::new(),
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].actor_record, None);
        assert_eq!(rows[0].background_record, None);
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

            let rows = parse_script_branch_trace(root, None).expect("parse branch trace");
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

    // Measurement (not an assertion): does reachability-validated depth-2 branch
    // exploration reach more dialogue text-calls than the current single-flip? Run
    // with `cargo test measure_depth2_branch_coverage_gain -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn measure_depth2_branch_coverage_gain() {
        use std::collections::BTreeSet;
        let Some(root) = ["output", "../output"]
            .iter()
            .map(Path::new)
            .find(|r| find_file_recursive(r, "SCRIPT1.COD").is_some())
        else {
            eprintln!("skipping: extracted output scripts not available");
            return;
        };
        let (mut tot_d, mut tot_s, mut tot_2, mut tot_3) = (0usize, 0usize, 0usize, 0usize);
        for idx in 1..=5 {
            let (Some(cod_p), Some(dic_p), Some(var_p)) = (
                find_file_recursive(root, &format!("SCRIPT{idx}.COD")),
                find_file_recursive(root, &format!("SCRIPT{idx}.DIC")),
                find_file_recursive(root, &format!("SCRIPT{idx}.VAR")),
            ) else {
                continue;
            };
            let cod = fs::read(&cod_p).unwrap();
            let var = fs::read(&var_p).unwrap();
            let context = match find_file_recursive(root, &format!("SCRIPT{idx}.DEB")) {
                Some(p) => vm_execution_context_from_deb(&fs::read(p).unwrap(), None),
                None => vm::ExecutionContext::default(),
            };
            let words = parse_script_dictionary(&dic_p).unwrap();
            let text_calls = text_calls_by_offset(&cod, &words);
            let offs = |ovr: &[vm::BranchOverride]| -> Vec<usize> {
                let t = vm::execute_trace_with_overrides_and_context(&cod, &var, ovr, &context);
                executed_text_offsets(&t, &text_calls)
            };
            let default_trace =
                vm::execute_trace_with_overrides_and_context(&cod, &var, &[], &context);
            let covered: BTreeSet<usize> = executed_text_offsets(&default_trace, &text_calls)
                .into_iter()
                .collect();
            let n_default = covered.len();
            let decisions: Vec<(usize, bool)> = default_trace
                .branch_events
                .iter()
                .filter_map(|e| e.condition_passed.map(|c| (e.offset, c)))
                .collect();
            let mut single = covered.clone();
            let mut d1: Vec<Vec<vm::BranchOverride>> = Vec::new();
            for (off, cp) in &decisions {
                let ovr = vec![vm::BranchOverride {
                    offset: *off,
                    condition_passed: !cp,
                }];
                single.extend(offs(&ovr));
                d1.push(ovr);
            }
            let mut depth2 = single.clone();
            let mut d2_new: Vec<Vec<vm::BranchOverride>> = Vec::new();
            let mut budget = 3000usize;
            for ovr1 in &d1 {
                if budget == 0 {
                    break;
                }
                let t1 = vm::execute_trace_with_overrides_and_context(&cod, &var, ovr1, &context);
                for e in &t1.branch_events {
                    if budget == 0 {
                        break;
                    }
                    let Some(cp) = e.condition_passed else {
                        continue;
                    };
                    if e.offset == ovr1[0].offset {
                        continue;
                    }
                    let mut ovr2 = ovr1.clone();
                    ovr2.push(vm::BranchOverride {
                        offset: e.offset,
                        condition_passed: !cp,
                    });
                    let before = depth2.len();
                    depth2.extend(offs(&ovr2));
                    if depth2.len() > before {
                        d2_new.push(ovr2);
                    }
                    budget -= 1;
                }
            }
            // depth-3: flip a 3rd reachable decision on the new-covering depth-2
            // scenarios (reachability-validated, budget-capped).
            let mut depth3 = depth2.clone();
            let mut budget3 = 3000usize;
            for ovr2 in &d2_new {
                if budget3 == 0 {
                    break;
                }
                let t2 = vm::execute_trace_with_overrides_and_context(&cod, &var, ovr2, &context);
                for e in &t2.branch_events {
                    if budget3 == 0 {
                        break;
                    }
                    let Some(cp) = e.condition_passed else {
                        continue;
                    };
                    if ovr2.iter().any(|o| o.offset == e.offset) {
                        continue;
                    }
                    let mut ovr3 = ovr2.clone();
                    ovr3.push(vm::BranchOverride {
                        offset: e.offset,
                        condition_passed: !cp,
                    });
                    depth3.extend(offs(&ovr3));
                    budget3 -= 1;
                }
            }
            eprintln!(
                "SCRIPT{idx}: default={n_default} single-flip={} depth2={} depth3={} (decisions={})",
                single.len(),
                depth2.len(),
                depth3.len(),
                decisions.len()
            );
            tot_d += n_default;
            tot_s += single.len();
            tot_2 += depth2.len();
            tot_3 += depth3.len();
        }
        eprintln!(
            "TOTAL text-calls reached: default={tot_d} single-flip={tot_s} depth2={tot_2} depth3={tot_3}"
        );
    }

    // Renders clay3's uncovered scene(s) against cached _tmp_dat to a scratch dir:
    //   cargo test render_sample_uncovered_scene -- --ignored --nocapture
    #[test]
    #[ignore]
    fn render_sample_uncovered_scene() {
        let Some(out) = ["output", "../output"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("_tmp_dat").exists() && p.join("_tmp_iso").exists())
        else {
            eprintln!("skipping: no cached extraction");
            return;
        };
        let tmp_iso = out.join("_tmp_iso");
        let tmp_dat = out.join("_tmp_dat");
        let db =
            super::descript::parse_descript(&find_file_recursive(&tmp_iso, "DESCRIPT.DES").unwrap())
                .unwrap();
        let hnm_music = db.hnm_music_map();
        let clay3: Vec<_> = parse_script_uncovered_speech(&tmp_iso, Some(&db), &hnm_music)
            .unwrap()
            .into_iter()
            .filter(|r| r.scenario_id.as_deref() == Some("fn:SCRIPT4:clay3"))
            .collect();
        assert!(!clay3.is_empty(), "clay3 lines exist");
        eprintln!("clay3 lines: {}", clay3.len());
        let mp4_dir = Path::new("/tmp/ben_uncov_render");
        let _ = fs::create_dir_all(mp4_dir);
        let subtitle_sfx = tmp_dat.join("sn").join("tb.snd");
        let n = crate::extract::character::create_executed_dialogue_run_videos(
            &tmp_dat,
            mp4_dir,
            &db,
            &clay3,
            subtitle_sfx.exists().then_some(subtitle_sfx.as_path()),
        )
        .unwrap();
        eprintln!("rendered {n} clay3 scene video(s) -> {}", mp4_dir.display());
        assert!(n >= 1, "at least one clay3 scene must render");
    }

    // Validates parse_script_uncovered_speech against cached extraction:
    //   cargo test measure_uncovered_speech_rows -- --ignored --nocapture
    #[test]
    #[ignore]
    fn measure_uncovered_speech_rows() {
        let Some(iso) = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .map(Path::new)
            .find(|p| find_file_recursive(p, "SCRIPT4.COD").is_some())
        else {
            eprintln!("skipping: no cached _tmp_iso");
            return;
        };
        let descript_path = find_file_recursive(iso, "DESCRIPT.DES").unwrap();
        let db = super::descript::parse_descript(&descript_path).unwrap();
        let hnm_music = db.hnm_music_map();
        let rows = parse_script_uncovered_speech(iso, Some(&db), &hnm_music).unwrap();
        eprintln!("uncovered speech rows: {}", rows.len());
        for idx in 1..=5 {
            let s = format!("SCRIPT{idx}");
            let c = rows.iter().filter(|r| r.script == s).count();
            let runs = rows
                .iter()
                .filter(|r| r.script == s)
                .map(|r| r.scenario_id.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .len();
            eprintln!("  {s}: {c} lines across {runs} function-scenes");
        }
        let honk = rows.iter().find(|r| r.text.contains("Honk filled me in"));
        eprintln!(
            "Honk line present: {} actor={:?} bg={:?}",
            honk.is_some(),
            honk.map(|r| r.actor_record.as_deref()),
            honk.map(|r| r.background_hnm.as_deref()),
        );
        assert!(rows.len() > 500, "expected substantial uncovered coverage");
        assert!(honk.is_some(), "clay3 'Honk filled me in' must be covered");
    }

    // Verifies the PRODUCTION path (parse_script_branch_scenarios) emits depth-2
    // "branch2" scenarios that cover dialogue lines no single-flip did.
    #[test]
    #[ignore]
    fn depth2_scenarios_add_coverage_in_production() {
        let Some(root) = ["output", "../output"]
            .iter()
            .map(Path::new)
            .find(|r| find_file_recursive(r, "SCRIPT1.COD").is_some())
        else {
            eprintln!("skipping: extracted output scripts not available");
            return;
        };
        let branch_rows = parse_script_branch_trace(root, None).expect("branch trace");
        let scenarios = parse_script_branch_scenarios(root, &branch_rows, None).expect("scenarios");
        let branch2: Vec<_> = scenarios
            .iter()
            .filter(|s| s.scenario_id.contains("-branch2-"))
            .collect();
        let new_lines: usize = branch2.iter().map(|s| s.new_text_calls).sum();
        eprintln!(
            "production depth-2 scenarios: {} covering {} new text-calls beyond single-flip",
            branch2.len(),
            new_lines
        );
        assert!(
            !branch2.is_empty(),
            "expected depth-2 scenarios to be emitted"
        );
        assert!(new_lines > 0, "depth-2 should cover new dialogue lines");
    }

    #[test]
    fn script_post_update_manifest_records_pending_profile_gate() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "commander-blood-post-update-{}-{nonce}",
            std::process::id(),
        ));
        fs::create_dir_all(&root).expect("create temp script root");
        fs::write(
            root.join("script1.cod"),
            [vm::OP_SCRIPT_PROFILE_REQUEST, 0x02, 0xff],
        )
        .expect("write script1.cod");
        fs::write(root.join("script1.var"), vec![0; 0x8000]).expect("write script1.var");

        let rows = parse_script_post_update(&root, None).expect("parse post update");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].script, "SCRIPT1");
        assert_eq!(rows[0].event_index, 0);
        assert_eq!(rows[0].event_kind, "pending_profile_dispatch");
        assert_eq!(rows[0].ready, Some(true));

        let path = root.join("script-post-update.tsv");
        write_script_post_update_manifest(&rows, &path).expect("write post update");
        let manifest = fs::read_to_string(&path).expect("read post update");
        assert!(
            manifest.starts_with(
                "script\tevent_index\tevent_kind\trecord_offset\trelated_record_offset"
            )
        );
        assert!(manifest.contains("SCRIPT1\t0\tpending_profile_dispatch\t\t\t\t\ttrue"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn parses_real_script_disassembly_global_conditions_if_present() {
        for prefix in ["output", "../output"] {
            let root = Path::new(prefix);
            if !root.join("scripts").exists() {
                continue;
            }

            let rows = parse_script_disassembly(root, None, &HashMap::new())
                .expect("parse script disassembly");
            assert!(rows.iter().any(|row| {
                row.script == "SCRIPT2"
                    && row.offset == 0x033a0
                    && row.mnemonic == "global_pair_compare"
                    && row.operands.contains("packed=0x0c19")
            }));
            assert!(rows.iter().any(|row| {
                row.script == "SCRIPT2"
                    && row.offset == 0x034eb
                    && row.mnemonic == "global_word_compare"
                    && row.operands.contains("value=0x0008")
            }));
            assert!(
                rows.iter()
                    .filter(|row| row.mnemonic == "global_word_compare")
                    .count()
                    > 40,
                "expected mode-aware CA rows from real scripts"
            );
            return;
        }

        eprintln!("skipping: extracted output scripts not available");
    }

    #[test]
    fn parses_real_script_rtc_scenarios_if_present() {
        for prefix in ["output", "../output"] {
            let root = Path::new(prefix);
            if !root.join("scripts").exists() {
                continue;
            }

            let branch_rows = parse_script_branch_trace(root, None).expect("parse branch trace");
            let scenarios = parse_script_branch_scenarios(root, &branch_rows, None)
                .expect("parse branch scenarios");
            let rtc_rows: Vec<&ScriptBranchScenarioLine> = scenarios
                .iter()
                .filter(|row| row.scenario_kind == "rtc")
                .collect();
            assert!(
                rtc_rows.len() > 20,
                "expected real RTC replay scenarios, got {} rows",
                rtc_rows.len()
            );
            assert!(rtc_rows.iter().any(|row| {
                row.script == "SCRIPT2"
                    && row.rtc_hour == Some(0)
                    && row.rtc_month == Some(12)
                    && row.rtc_day == Some(25)
            }));
            assert!(rtc_rows.iter().any(|row| {
                row.script == "SCRIPT3"
                    && row.rtc_hour == Some(22)
                    && row.rtc_month == Some(1)
                    && row.rtc_day == Some(2)
            }));
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
                rows.len() > 750,
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
    fn script_profile_sequence_follows_binary_profile_handoff() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "commander-blood-profile-sequence-{}-{nonce}",
            std::process::id(),
        ));
        fs::create_dir_all(&root).expect("create temp script root");

        fs::write(
            root.join("script1.cod"),
            [vm::OP_SCRIPT_PROFILE_REQUEST, 0x02, 0xff],
        )
        .expect("write script1.cod");
        fs::write(root.join("script1.var"), vec![0; 0x8000]).expect("write script1.var");
        fs::write(root.join("script1.dic"), []).expect("write script1.dic");
        fs::write(root.join("script1.deb"), []).expect("write script1.deb");
        fs::write(root.join("script1.bas"), []).expect("write script1.bas");

        let mut script2_cod = vec![vm::OP_TEXT];
        script2_cod.extend_from_slice(&0x1234u16.to_le_bytes());
        script2_cod.extend_from_slice(&[0xff, 0x00, vm::TEXT_ACTIVE_DISPLAY_FLAG]);
        script2_cod.extend_from_slice(&1u16.to_le_bytes());
        script2_cod.extend_from_slice(&0u16.to_le_bytes());
        script2_cod.push(0xff);
        fs::write(root.join("script2.cod"), script2_cod).expect("write script2.cod");
        fs::write(root.join("script2.var"), vec![0; 0x8000]).expect("write script2.var");
        fs::write(root.join("script2.dic"), b"\0hello\0").expect("write script2.dic");
        fs::write(root.join("script2.deb"), []).expect("write script2.deb");
        fs::write(root.join("script2.bas"), []).expect("write script2.bas");

        let profiles = vec![script_resource_profile(0, 1), script_resource_profile(1, 2)];
        let export = parse_script_profile_sequence(&root, &profiles, None, &HashMap::new())
            .expect("parse profile sequence");

        assert_eq!(export.runs.len(), 2);
        assert_eq!(export.runs[0].script, "SCRIPT1");
        assert_eq!(export.runs[0].pending_profile_index, Some(1));
        assert_eq!(export.runs[0].pending_script.as_deref(), Some("SCRIPT2"));
        assert!(export.runs[0].pending_dispatch_ready);
        assert_eq!(export.runs[0].post_update_pairs, "");
        assert_eq!(export.runs[0].presentation_handoffs, "");
        assert_eq!(export.runs[0].request_summary, "0x00000:2->1");
        assert_eq!(export.runs[1].script, "SCRIPT2");
        assert_eq!(export.runs[1].halted_after_run, "NoPendingProfile");

        assert_eq!(export.dialogue.len(), 1);
        let line = &export.dialogue[0];
        assert_eq!(line.global_sequence_index, 0);
        assert_eq!(line.run_index, 1);
        assert_eq!(line.profile_index, 1);
        assert_eq!(line.row.script, "SCRIPT2");
        assert_eq!(line.row.offset, 0);
        assert!(
            line.row
                .source
                .starts_with("SCRIPT VM execute_script_profile_sequence")
        );

        let runs_path = root.join("script-profile-runs.tsv");
        write_script_profile_runs_manifest(&export.runs, &runs_path).expect("write profile runs");
        let runs_manifest = fs::read_to_string(&runs_path).expect("read profile runs");
        assert!(runs_manifest.starts_with(
            "sequence_id\trun_index\tprofile_index\td2_operand\tscript\tsteps\ttext_calls\tpending_profile_index\tpending_script\tpending_dispatch_ready"
        ));
        assert!(runs_manifest.contains(
            "default\t0\t0\t1\tSCRIPT1\t2\t0\t1\tSCRIPT2\ttrue\t\t\t0x00000:2->1\thandoff"
        ));

        let dialogue_path = root.join("script-profile-executed-dialogue.tsv");
        write_script_profile_executed_speech_manifest(&export.dialogue, &dialogue_path)
            .expect("write profile dialogue");
        let dialogue_manifest = fs::read_to_string(&dialogue_path).expect("read profile dialogue");
        assert!(
            dialogue_manifest
                .starts_with("sequence_id\tglobal_sequence_index\trun_index\tprofile_index")
        );
        assert!(dialogue_manifest.contains("default\t0\t1\t1\t2\tSCRIPT2"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn profile_dialogue_runs_follow_global_sequence_order() {
        let rows = vec![
            profile_speech_line(1, 1, "SCRIPT2", 0, 0x50, None, Some("Room1"), "b"),
            profile_speech_line(
                0,
                0,
                "SCRIPT1",
                0,
                0x10,
                Some("Actor_A"),
                Some("Room1"),
                "a",
            ),
            profile_speech_line(
                2,
                2,
                "SCRIPT3",
                0,
                0x30,
                Some("Actor_A"),
                Some("Room2"),
                "c",
            ),
        ];

        let runs = script_profile_dialogue_runs(&rows);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].sequence_id, "default");
        assert_eq!(runs[0].run_index, 1);
        assert_eq!(runs[0].first_global_sequence, 0);
        assert_eq!(runs[0].last_global_sequence, 1);
        assert_eq!(runs[0].first_script, "SCRIPT1");
        assert_eq!(runs[0].last_script, "SCRIPT2");
        assert_eq!(runs[0].lines[0].row.script, "SCRIPT1");
        assert_eq!(runs[0].lines[1].row.script, "SCRIPT2");
        assert_eq!(runs[1].run_index, 2);
        assert_eq!(runs[1].background_record.as_deref(), Some("Room2"));

        let path = std::env::temp_dir().join(format!(
            "commander-blood-profile-dialogue-runs-{}.tsv",
            std::process::id()
        ));
        write_script_profile_dialogue_runs_manifest(&rows, &path)
            .expect("write profile dialogue runs");
        let manifest = fs::read_to_string(&path).expect("read profile dialogue runs");
        let _ = fs::remove_file(&path);
        assert!(manifest.starts_with("sequence_id\trun_id\tmp4"));
        assert!(
            manifest
                .lines()
                .next()
                .is_some_and(|header| header.ends_with(
                    "\tunresolved_actor_count\tunresolved_background_count\tunresolved_voice_count"
                ))
        );
        assert!(manifest.contains("default-profile-run-0001"));
        assert!(manifest.contains("SCRIPT1\tSCRIPT2"));
        let run_row = manifest_row(&manifest, "default\tdefault-profile-run-0001\t");
        assert_eq!(&run_row[17..20], &["1", "0", "0"]);
    }

    #[test]
    fn dialogue_runs_keep_multi_actor_execution_order_and_split_locations() {
        let rows = vec![
            speech_line("SCRIPT2", 0x10, Some("Actor_A"), Some("Room1"), "a"),
            speech_line("SCRIPT2", 0x20, Some("Actor_B"), Some("Room1"), "b"),
            speech_line("SCRIPT2", 0x30, Some("Actor_A"), Some("Room2"), "c"),
            speech_line("SCRIPT2", 0x40, None, Some("Room2"), "narrator"),
            speech_line("SCRIPT2", 0x50, None, None, "unknown"),
            speech_line("SCRIPT3", 0x10, Some("Actor_A"), Some("Room1"), "d"),
        ];

        let runs = script_dialogue_runs(&rows);
        assert_eq!(runs.len(), 4);
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

        assert_eq!(runs[2].script, "SCRIPT2");
        assert_eq!(runs[2].run_index, 3);
        assert_eq!(runs[2].background_record, None);
        assert_eq!(runs[3].script, "SCRIPT3");
        assert_eq!(runs[3].run_index, 1);

        let path = std::env::temp_dir().join(format!(
            "commander-blood-dialogue-runs-{}.tsv",
            std::process::id()
        ));
        write_script_dialogue_runs_manifest(&rows, &path).expect("write dialogue runs");
        let manifest = fs::read_to_string(&path).expect("read dialogue runs");
        let _ = fs::remove_file(&path);
        assert!(manifest.contains("SCRIPT2-0001"));
        assert!(
            manifest
                .lines()
                .next()
                .is_some_and(|header| header.ends_with(
                    "\tunresolved_actor_count\tunresolved_background_count\tunresolved_voice_count"
                ))
        );
        assert!(manifest.contains("Actor_A,Actor_B"));
        let room2_row = manifest_row(&manifest, "SCRIPT2-0002\t");
        assert_eq!(&room2_row[13..16], &["1", "0", "0"]);
        let nolocation_row = manifest_row(&manifest, "SCRIPT2-0003\t");
        assert_eq!(&nolocation_row[13..16], &["1", "1", "0"]);
    }

    #[test]
    fn executed_dialogue_runs_follow_sequence_order_and_split_locations() {
        let mut missing_voice =
            executed_speech_line("SCRIPT2", 3, 0x40, Some("Actor_A"), Some("Room2"), "silent");
        missing_voice.clip_index = None;
        let mut deliberate_silent = executed_speech_line(
            "SCRIPT2",
            4,
            0x45,
            Some("Actor_A"),
            Some("Room2"),
            "selector none",
        );
        deliberate_silent.param0 = vm::TEXT_SELECTOR_NONE;
        deliberate_silent.active_line_id = vm::text_selector_active_line_id(vm::TEXT_SELECTOR_NONE);
        deliberate_silent.clip_index = None;
        let rows = vec![
            executed_speech_line("SCRIPT2", 0, 0x50, Some("Actor_A"), Some("Room1"), "a"),
            executed_speech_line("SCRIPT2", 1, 0x10, Some("Actor_B"), Some("Room1"), "b"),
            executed_speech_line("SCRIPT2", 2, 0x30, Some("Actor_A"), Some("Room2"), "c"),
            missing_voice,
            deliberate_silent,
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
        assert!(
            manifest
                .lines()
                .next()
                .is_some_and(|header| header.ends_with(
                    "\tunresolved_actor_count\tunresolved_background_count\tunresolved_voice_count"
                ))
        );
        assert!(manifest.contains("Actor_A,Actor_B"));
        assert!(manifest.contains("0x00050\t0x00010"));
        let room2_row = manifest_row(&manifest, "SCRIPT2-0002\t");
        assert_eq!(&room2_row[15..18], &["0", "0", "1"]);
    }

    #[test]
    fn scene_events_manifest_exports_renderer_event_stream() {
        let mut looped =
            executed_speech_line("SCRIPT2", 1, 0x60, Some("Actor_A"), Some("Room1"), "second");
        looped.param1 = 0x18;
        looped.skip_count = Some(3);
        looped.loop_target = Some(0x1234);
        let rows = vec![
            executed_speech_line("SCRIPT2", 0, 0x50, Some("Actor_A"), Some("Room1"), "first"),
            looped,
        ];

        let path = std::env::temp_dir().join(format!(
            "commander-blood-scene-events-{}.tsv",
            std::process::id()
        ));
        write_script_scene_events_manifest(&rows, &path).expect("write scene events");
        let manifest = fs::read_to_string(&path).expect("read scene events");
        let _ = fs::remove_file(&path);

        assert!(manifest.starts_with("scenario_id\trun_id\tmp4\tscript"));
        assert!(
            manifest
                .lines()
                .next()
                .is_some_and(|header| header.ends_with("\tsource"))
        );
        assert!(manifest.contains("executed-dialogue-run - script2 - 0001 - room1.mp4"));
        assert!(manifest.contains("\tset_background\t0\t0x00050\tActor_A\tRoom1"));
        assert!(manifest.contains("\tshow_speaker\t0\t0x00050\tActor_A"));
        assert!(
            manifest
                .contains("\tplay_talk_hnm\t0\t0x00050\tActor_A\tRoom1\tRoom1.hnm\tRoom1_music\t0")
        );
        assert!(
            manifest
                .contains("\tplay_voice\t0\t0x00050\tActor_A\tRoom1\tRoom1.hnm\tRoom1_music\t0")
        );
        assert!(manifest.contains("\tdraw_subtitle\t0\t0x00050\tActor_A\tRoom1\tRoom1.hnm\tRoom1_music\t\t01\t0x000a\t00\t\t\tfirst"));
        assert!(manifest.contains("\tdraw_subtitle\t1\t0x00060\tActor_A\tRoom1\tRoom1.hnm\tRoom1_music\t\t01\t0x000a\t18\t3\t0x1234\tsecond"));
        assert!(manifest.contains(
            "\tplay_chatter\t0\t0x00050\tActor_A\tRoom1\tRoom1.hnm\tRoom1_music\t\t\t0x000a"
        ));
        let draw = manifest
            .lines()
            .find(|line| line.contains("\tdraw_subtitle\t"))
            .expect("draw subtitle row");
        assert!(draw.ends_with("\tfirst\ttest"));
        let last = manifest.lines().last().expect("last scene-event row");
        let columns = last.split('\t').collect::<Vec<_>>();
        assert_eq!(columns.len(), 21);
        assert_eq!(columns[6], "clear");
        assert_eq!(columns[7], "");
        assert_eq!(columns[8], "");
        assert_eq!(columns[20], "");
    }

    #[test]
    fn scene_events_manifest_reports_unresolved_presentation_inputs() {
        let mut missing_voice = executed_speech_line(
            "SCRIPT2",
            1,
            0x60,
            Some("Actor_A"),
            Some("Room1"),
            "missing voice",
        );
        missing_voice.param0 = 0x05;
        missing_voice.active_line_id = vm::text_selector_active_line_id(0x05);
        missing_voice.clip_index = None;

        let mut deliberate_silent = executed_speech_line(
            "SCRIPT2",
            2,
            0x70,
            Some("Actor_A"),
            Some("Room1"),
            "silent by selector",
        );
        deliberate_silent.param0 = vm::TEXT_SELECTOR_NONE;
        deliberate_silent.active_line_id = vm::text_selector_active_line_id(vm::TEXT_SELECTOR_NONE);
        deliberate_silent.clip_index = None;

        let rows = vec![
            executed_speech_line("SCRIPT2", 0, 0x50, None, None, "missing context"),
            missing_voice,
            deliberate_silent,
        ];

        let path = std::env::temp_dir().join(format!(
            "commander-blood-unresolved-scene-events-{}.tsv",
            std::process::id()
        ));
        write_script_scene_events_manifest(&rows, &path).expect("write scene events");
        let manifest = fs::read_to_string(&path).expect("read scene events");
        let _ = fs::remove_file(&path);

        let unresolved_background = manifest
            .lines()
            .find(|line| line.contains("\tunresolved_background\t"))
            .expect("unresolved background event");
        let columns = unresolved_background.split('\t').collect::<Vec<_>>();
        assert_eq!(columns[7], "0");
        assert_eq!(columns[8], "0x00050");
        assert_eq!(columns[15], "0x000a");
        assert_eq!(columns[20], "test");

        let unresolved_actor = manifest
            .lines()
            .find(|line| line.contains("\tunresolved_actor\t"))
            .expect("unresolved actor event");
        let columns = unresolved_actor.split('\t').collect::<Vec<_>>();
        assert_eq!(columns[7], "0");
        assert_eq!(columns[8], "0x00050");
        assert_eq!(columns[15], "0x000a");
        assert_eq!(columns[20], "test");

        let unresolved_voice = manifest
            .lines()
            .find(|line| line.contains("\tunresolved_voice\t"))
            .expect("unresolved voice event");
        let columns = unresolved_voice.split('\t').collect::<Vec<_>>();
        assert_eq!(columns[7], "1");
        assert_eq!(columns[8], "0x00060");
        assert_eq!(columns[9], "Actor_A");
        assert_eq!(columns[10], "Room1");
        assert_eq!(columns[11], "Room1.hnm");
        assert_eq!(columns[12], "Room1_music");
        assert_eq!(columns[14], "05");
        assert_eq!(columns[15], "0x000e");
        assert_eq!(columns[20], "test");
        assert_eq!(manifest.matches("\tunresolved_voice\t").count(), 1);
    }

    #[test]
    fn profile_scene_events_manifest_exports_global_context() {
        let rows = vec![
            profile_speech_line(
                0,
                0,
                "SCRIPT1",
                0,
                0x10,
                Some("Actor_A"),
                Some("Room1"),
                "a",
            ),
            profile_speech_line(
                1,
                1,
                "SCRIPT2",
                0,
                0x20,
                Some("Actor_A"),
                Some("Room1"),
                "b",
            ),
        ];

        let path = std::env::temp_dir().join(format!(
            "commander-blood-profile-scene-events-{}.tsv",
            std::process::id()
        ));
        write_script_profile_scene_events_manifest(&rows, &path)
            .expect("write profile scene events");
        let manifest = fs::read_to_string(&path).expect("read profile scene events");
        let _ = fs::remove_file(&path);

        assert!(manifest.starts_with("sequence_id\trun_id\tmp4"));
        assert!(
            manifest
                .lines()
                .next()
                .is_some_and(|header| header.ends_with("\tsource"))
        );
        assert!(manifest.contains("default\tdefault-profile-run-0001"));
        assert!(manifest.contains("profile-dialogue-run - default - 0001 - room1.mp4"));
        assert!(manifest.contains("\tdraw_subtitle\t1\t1\t2\tSCRIPT2\t0\t0x00020"));
        let draw = manifest
            .lines()
            .find(|line| line.contains("\tdraw_subtitle\t") && line.contains("\tSCRIPT2\t"))
            .expect("profile draw subtitle row");
        assert!(draw.ends_with("\tb\ttest"));
    }

    #[test]
    fn legacy_dialogue_video_manifest_uses_executed_sequence_order() {
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
        write_legacy_script_dialogue_manifest(&rows, &path).expect("write dialogue videos");
        let manifest = fs::read_to_string(&path).expect("read dialogue videos");
        let _ = fs::remove_file(&path);
        assert!(manifest.contains("dialogue - script2 - room1 - actor_a.mp4"));
        assert!(manifest.contains("\t3\t1,2"));
        assert!(!manifest.contains("\t3\t2,1"));
    }

    #[test]
    fn branch_decisions_manifest_records_alternate_path() {
        let rows = vec![
            branch_trace_line(
                "SCRIPT2",
                0,
                0x10,
                0xaf,
                Some(0x40),
                false,
                Some(true),
                "condition passed",
            ),
            branch_trace_line(
                "SCRIPT2",
                1,
                0x20,
                0xaf,
                Some(0x80),
                true,
                Some(false),
                "condition failed",
            ),
            branch_trace_line(
                "SCRIPT2",
                2,
                0x30,
                0xa1,
                Some(0x90),
                false,
                None,
                "condition block end",
            ),
        ];

        let path = std::env::temp_dir().join(format!(
            "commander-blood-branch-decisions-{}.tsv",
            std::process::id()
        ));
        write_script_branch_decisions_manifest(&rows, &path).expect("write branch decisions");
        let manifest = fs::read_to_string(&path).expect("read branch decisions");
        let _ = fs::remove_file(&path);
        assert!(manifest.contains("SCRIPT2\t1\t0x00010\taf\ttrue\tfallthrough\t\tjump\t0x0040"));
        assert!(manifest.contains("SCRIPT2\t2\t0x00020\taf\tfalse\tjump\t0x0080\tfallthrough\t"));
        assert!(!manifest.contains("condition block end"));
    }

    #[test]
    fn branch_coverage_manifest_reports_default_execution_gap() {
        let speech_rows = vec![
            speech_line("SCRIPT2", 0x10, Some("Actor_A"), Some("Room1"), "a"),
            speech_line("SCRIPT2", 0x20, Some("Actor_A"), Some("Room1"), "b"),
            speech_line("SCRIPT2", 0x30, Some("Actor_A"), Some("Room1"), "c"),
        ];
        let executed_rows = vec![executed_speech_line(
            "SCRIPT2",
            0,
            0x10,
            Some("Actor_A"),
            Some("Room1"),
            "a",
        )];
        let branch_rows = vec![
            branch_trace_line(
                "SCRIPT2",
                0,
                0x10,
                0xaf,
                Some(0x40),
                false,
                Some(true),
                "condition passed",
            ),
            branch_trace_line(
                "SCRIPT2",
                1,
                0x20,
                0xaf,
                Some(0x80),
                true,
                Some(false),
                "condition failed",
            ),
        ];

        let path = std::env::temp_dir().join(format!(
            "commander-blood-branch-coverage-{}.tsv",
            std::process::id()
        ));
        write_script_branch_coverage_manifest(&speech_rows, &executed_rows, &branch_rows, &path)
            .expect("write branch coverage");
        let manifest = fs::read_to_string(&path).expect("read branch coverage");
        let _ = fs::remove_file(&path);
        assert!(manifest.contains("SCRIPT2\t3\t1\t2\t33.33\t2\t2\t1\t1\t1\t1"));
    }

    #[test]
    fn text_flags_manifest_decodes_b4_b5_controls() {
        assert_eq!(text_skip_count(0x08, 0xa0), Some(3));
        assert_eq!(
            text_control_summary(0x39, 0xa0, Some(0x1234)),
            "active,conditional-skip:3,loop:0x1234,preserve-active,b4-unknown:0x20,b5-payload:0x20"
        );

        let rows = vec![ScriptTextFlagLine {
            script: "SCRIPT2".to_string(),
            function_name: "func".to_string(),
            offset: 0x20,
            line_index: 0x0102,
            voice_selector: 0x03,
            active_line_id: vm::text_selector_active_line_id(0x03),
            flags_b4: 0x39,
            flags_b5: 0xa0,
            loop_target: Some(0x1234),
            active: true,
            skip_count: Some(3),
            summary: text_control_summary(0x39, 0xa0, Some(0x1234)),
            text: "hello".to_string(),
        }];

        let path = std::env::temp_dir().join(format!(
            "commander-blood-text-flags-{}.tsv",
            std::process::id()
        ));
        write_script_text_flags_manifest(&rows, &path).expect("write text flags");
        let manifest = fs::read_to_string(&path).expect("read text flags");
        let _ = fs::remove_file(&path);
        assert!(manifest.starts_with("script\tfunction\toffset\tline_index"));
        assert!(
            manifest
                .contains("SCRIPT2\tfunc\t0x00020\t0x0102\t03\t0x000c\t39\ta0\ttrue\t3\t0x1234")
        );
        assert!(manifest.contains("conditional-skip:3"));
    }

    #[test]
    fn branch_scenarios_force_alternate_condition_once() {
        let (root, condition_offset) = synthetic_branch_script_dir();
        let branch_rows = vec![branch_trace_line(
            "SCRIPT1",
            0,
            condition_offset,
            0xc0,
            Some(0x0010),
            true,
            Some(false),
            "condition failed",
        )];

        let rows = parse_script_branch_scenarios(&root, &branch_rows, None)
            .expect("parse branch scenarios");
        let _ = fs::remove_dir_all(&root);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].scenario_id, "SCRIPT1-branch-0001");
        assert_eq!(rows[0].default_condition_passed, false);
        assert_eq!(rows[0].forced_condition_passed, true);
        assert_eq!(rows[0].default_text_calls, 1);
        assert_eq!(rows[0].scenario_text_calls, 2);
        assert_eq!(rows[0].new_text_calls, 1);
        assert_eq!(rows[0].lost_text_calls, 0);

        let path = std::env::temp_dir().join(format!(
            "commander-blood-branch-scenarios-{}.tsv",
            std::process::id()
        ));
        write_script_branch_scenarios_manifest(&rows, &path).expect("write branch scenarios");
        let manifest = fs::read_to_string(&path).expect("read branch scenarios");
        let _ = fs::remove_file(&path);
        assert!(manifest.contains("SCRIPT1\tSCRIPT1-branch-0001\tbranch-override\t1"));
        assert!(manifest.contains("\tfalse\ttrue\t\t\t\t1\t2\t1\t0\t"));
    }

    #[test]
    fn rtc_scenarios_replay_global_clock_conditions() {
        let root = synthetic_rtc_script_dir();

        let scenarios =
            parse_script_branch_scenarios(&root, &[], None).expect("parse rtc branch scenarios");
        assert_eq!(scenarios.len(), 3);
        assert!(scenarios.iter().all(|row| row.scenario_kind == "rtc"));
        assert!(scenarios.iter().any(|row| {
            row.scenario_id == "SCRIPT1-rtc-00h-0102"
                && row.rtc_hour == Some(0)
                && row.rtc_month == Some(1)
                && row.rtc_day == Some(2)
        }));
        assert!(scenarios.iter().any(|row| {
            row.scenario_id == "SCRIPT1-rtc-09h-0102"
                && row.scenario_text_calls == 1
                && row.lost_text_calls == 1
        }));

        let rows = parse_script_branch_scenario_speech(&root, None, &HashMap::new(), &scenarios)
            .expect("parse rtc scenario speech");
        let _ = fs::remove_dir_all(&root);

        assert!(!rows.is_empty());
        assert!(rows.iter().all(|row| {
            row.scenario_id
                .as_deref()
                .is_some_and(|id| id.starts_with("SCRIPT1-rtc-"))
                && row.source.contains("BIOS RTC scenario")
        }));

        let path = std::env::temp_dir().join(format!(
            "commander-blood-rtc-scenarios-{}.tsv",
            std::process::id()
        ));
        write_script_branch_scenarios_manifest(&scenarios, &path).expect("write rtc scenarios");
        let manifest = fs::read_to_string(&path).expect("read rtc scenarios");
        let _ = fs::remove_file(&path);
        assert!(manifest.starts_with("script\tscenario_id\tscenario_kind"));
        assert!(
            manifest.contains(
                "SCRIPT1\tSCRIPT1-rtc-09h-0102\trtc\t0\t0x00000\t00\tfalse\tfalse\t9\t1\t2"
            )
        );
    }

    #[test]
    fn branch_scenario_speech_uses_forced_trace() {
        let (root, condition_offset) = synthetic_branch_script_dir();
        let branch_rows = vec![branch_trace_line(
            "SCRIPT1",
            0,
            condition_offset,
            0xc0,
            Some(0x0010),
            true,
            Some(false),
            "condition failed",
        )];

        let scenarios = parse_script_branch_scenarios(&root, &branch_rows, None)
            .expect("parse branch scenarios");
        let rows = parse_script_branch_scenario_speech(&root, None, &HashMap::new(), &scenarios)
            .expect("parse branch scenario speech");
        let _ = fs::remove_dir_all(&root);

        assert_eq!(rows.len(), 2);
        assert!(
            rows.iter()
                .all(|row| row.scenario_id.as_deref() == Some("SCRIPT1-branch-0001"))
        );
        assert_eq!(rows[0].sequence_index, 0);
        assert_eq!(rows[1].sequence_index, 1);
        assert!(
            rows.iter()
                .all(|row| row.source.contains("execute_trace_with_overrides"))
        );

        let path = std::env::temp_dir().join(format!(
            "commander-blood-branch-scenario-dialogue-{}.tsv",
            std::process::id()
        ));
        write_script_branch_scenario_speech_manifest(&rows, &path)
            .expect("write branch scenario speech");
        let manifest = fs::read_to_string(&path).expect("read branch scenario speech");
        let _ = fs::remove_file(&path);
        assert!(manifest.starts_with("scenario_id\tscript\tsequence_index"));
        assert!(manifest.contains("SCRIPT1-branch-0001\tSCRIPT1\t0"));
    }

    #[test]
    fn scenario_dialogue_runs_do_not_merge_with_default_execution() {
        let default = executed_speech_line("SCRIPT2", 0, 0x10, Some("Actor_A"), Some("Room1"), "a");
        let mut scenario =
            executed_speech_line("SCRIPT2", 0, 0x20, Some("Actor_A"), Some("Room1"), "b");
        scenario.scenario_id = Some("SCRIPT2-branch-0001".to_string());

        let rows = vec![scenario, default];
        let runs = script_executed_dialogue_runs(&rows);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].scenario_id, None);
        assert_eq!(runs[0].run_index, 1);
        assert_eq!(runs[1].scenario_id.as_deref(), Some("SCRIPT2-branch-0001"));
        assert_eq!(runs[1].run_index, 1);
        assert_eq!(
            executed_dialogue_run_output_stem(&runs[1]),
            "branch-scenario-dialogue-run - script2-branch-0001 - 0001 - room1"
        );

        let path = std::env::temp_dir().join(format!(
            "commander-blood-branch-scenario-dialogue-runs-{}.tsv",
            std::process::id()
        ));
        write_script_branch_scenario_dialogue_runs_manifest(&rows, &path)
            .expect("write branch scenario runs");
        let manifest = fs::read_to_string(&path).expect("read branch scenario runs");
        let _ = fs::remove_file(&path);
        assert!(
            manifest
                .lines()
                .next()
                .is_some_and(|header| header.ends_with(
                    "\tunresolved_actor_count\tunresolved_background_count\tunresolved_voice_count"
                ))
        );
        assert!(manifest.contains("SCRIPT2-branch-0001\tSCRIPT2-branch-0001-run-0001"));
        assert!(
            manifest
                .contains("branch-scenario-dialogue-run - script2-branch-0001 - 0001 - room1.mp4")
        );
        let scenario_row = manifest_row(
            &manifest,
            "SCRIPT2-branch-0001\tSCRIPT2-branch-0001-run-0001\t",
        );
        assert_eq!(&scenario_row[16..19], &["0", "0", "0"]);
    }

    #[test]
    fn disassembly_uses_function_bounds_and_decodes_known_ops() {
        let mut words = HashMap::new();
        words.insert(0x0001, "hello".to_string());
        let cod = [
            0x01, 0x02, 0xc4, 0x3a, 0x00, 0x00, 0x00, 0xa6, 0x34, 0x12, 0x01, 0x00, 0x80, 0x01,
            0x00, 0x00, 0x00, 0xc3, 0x3a, 0x00, 0x28, 0x00, 0xc6, 0x8e, 0x10, 0x52, 0x10, 0xc9,
            0x3a, 0x00, 0xb7, 0x10, 0x00, 0x09, 0xc1, 0x4e, 0x12, 0x52, 0x0d, 0xca, 0xf1, 0xc1,
            0x08, 0x00, 0xcb, 0xf5, 0x19, 0x0c, 0xca, 0x07, 0xb8, 0x20, 0x00, 0x34, 0x12, 0x78,
            0x56, 0xcd, 0x94, 0x05, 0x04, 0x10, 0x28, 0x00, 0x03,
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
        assert!(
            rows.iter()
                .any(|row| row.mnemonic == "actor_ref" && row.len == 5)
        );
        assert!(
            rows.iter()
                .any(|row| row.mnemonic == "text_call" && row.text.as_deref() == Some("hello"))
        );
        assert!(
            rows.iter()
                .any(|row| row.mnemonic == "record_link" && row.len == 5)
        );
        assert!(
            rows.iter()
                .any(|row| row.mnemonic == "record_entry" && row.opcode == "c6" && row.len == 5)
        );
        assert!(
            rows.iter()
                .any(|row| row.mnemonic == "record_clear" && row.len == 3)
        );
        assert!(rows.iter().any(|row| {
            row.mnemonic == "bit_flag"
                && row.len == 4
                && row.operands.contains("byte=0x0011")
                && row.operands.contains("mask=0x40")
        }));
        assert!(rows.iter().any(|row| {
            row.mnemonic == "record_state"
                && row.opcode == "c1"
                && row.operands.contains("ref=0x124e")
                && row.operands.contains("operand=0x0d52")
        }));
        assert!(rows.iter().any(|row| {
            row.mnemonic == "global_word_compare"
                && row.operands.contains("global=gs:0x0aa6")
                && row.operands.contains("value=0x0008")
        }));
        assert!(rows.iter().any(|row| {
            row.mnemonic == "global_pair_compare"
                && row.operands.contains("global=gs:0x0aaa:0x0aa8")
                && row.operands.contains("packed=0x0c19")
        }));
        assert!(rows.iter().any(|row| {
            row.mnemonic == "pair_record"
                && row.opcode == "b8"
                && row.operands.contains("ref=0x0020")
                && row.operands.contains("first=0x1234")
                && row.operands.contains("second=0x5678")
        }));
        assert!(rows.iter().any(|row| {
            row.mnemonic == "record_triple"
                && row.opcode == "cd"
                && row.operands.contains("ref=0x0594")
                && row.operands.contains("first=0x1004")
                && row.operands.contains("second=0x0028")
                && row.operands.contains("inverted=false")
        }));
        assert_eq!(rows[0].function_name, "func");
    }
}
