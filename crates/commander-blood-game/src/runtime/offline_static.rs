//! Source-bound, explicitly prepared text sites, without story traversal.

use anyhow::{Context, Result, bail, ensure};
use commander_blood_formats::bas::ScriptBasInstruction;
use commander_blood_formats::code::ScriptCodeOffset;
use commander_blood_formats::instruction::{DecodedScriptInstruction, ScriptText, ScriptTextWord};
use commander_blood_formats::script::{ScriptObjectId, ScriptObjectKind, ScriptWordId};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::native::bloodprg::{
    LoadedScriptProfile, ScriptActionRecord, ScriptFieldSelector, script_field_offset,
};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum StaticTextSource {
    Cod,
    Bas,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OfflineStaticTextPlan {
    pub schema: u8,
    pub game: crate::game::GameVariant,
    pub title: String,
    pub initial_profile: u8,
    pub cod_sha256: String,
    pub dic_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bas_sha256: Option<String>,
    pub source: StaticTextSource,
    pub text_site: usize,
    pub context: StaticTextContext,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum StaticTextContext {
    SourceDefault {
        actor_offset: usize,
        descript_records: Vec<String>,
        /// DIC offsets pushed into the concept history, oldest first. Only the
        /// authored history candidates (A6 section 1) are accepted.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        history_concepts: Vec<u16>,
    },
    /// Built-in bridge records (Honk, menu) present over the bridge itself: the
    /// console's immediate choices queue them without a contact scene transition
    /// (nav_choice_handler_0 0x8713, nav_choice_handler_3 0x8848).
    BridgeConsole {
        actor_offset: usize,
        choice: StaticConsoleChoice,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        history_concepts: Vec<u16>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum StaticConsoleChoice {
    Horn,
    Radio,
}

impl StaticTextContext {
    pub fn actor_offset(&self) -> usize {
        match self {
            Self::SourceDefault { actor_offset, .. } | Self::BridgeConsole { actor_offset, .. } => {
                *actor_offset
            }
        }
    }

    pub fn descriptions(&self) -> &[String] {
        match self {
            Self::SourceDefault {
                descript_records, ..
            } => descript_records,
            Self::BridgeConsole { .. } => &[],
        }
    }

    pub fn history_concepts(&self) -> &[u16] {
        match self {
            Self::SourceDefault {
                history_concepts, ..
            }
            | Self::BridgeConsole {
                history_concepts, ..
            } => history_concepts,
        }
    }

    pub fn console_choice(&self) -> Option<StaticConsoleChoice> {
        match self {
            Self::SourceDefault { .. } => None,
            Self::BridgeConsole { choice, .. } => Some(*choice),
        }
    }
}

pub(super) struct BoundStaticText {
    pub text: ScriptText,
    pub actor: ScriptObjectId,
    pub history: Vec<ScriptWordId>,
    pub authored: Value,
}

pub(super) fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Return the authored history candidates (section 1) of a history-gated line.
pub(super) fn history_candidates(text: &ScriptText) -> Vec<ScriptWordId> {
    if !text.control.uses_history_condition() {
        return Vec::new();
    }
    text.words
        .iter()
        .skip_while(|word| !matches!(word, ScriptTextWord::SectionSeparator))
        .skip(1)
        .map_while(|word| match word {
            ScriptTextWord::Dictionary(word) => Some(*word),
            _ => None,
        })
        .collect()
}

pub(super) fn validate_plain_text(text: &ScriptText) -> Result<()> {
    ensure!(
        !text.words.is_empty(),
        "unsupported_static_text: control_only"
    );
    ensure!(
        !text.control.uses_random_gate() && !text.control.uses_record_condition(),
        "unsupported_static_text: conditional_gate"
    );
    // A6 b4&0x40 (vm_op_a6_text 0x660C) compares section 1 against the concept
    // history; display stops at the first separator. Only the two-section shape
    // without a resume menu is accepted, so the prepared history is exactly section 1.
    let sections = 1 + text
        .words
        .iter()
        .filter(|word| matches!(word, ScriptTextWord::SectionSeparator))
        .count();
    if text.control.uses_history_condition() {
        ensure!(
            sections == 2
                && !matches!(text.words.first(), Some(ScriptTextWord::SectionSeparator))
                && !history_candidates(text).is_empty(),
            "unsupported_static_text: history_section_shape"
        );
    }
    // b4&0x08 only arms vm_skip_count (DS:0x67AB) in vm_op_a6_text (0x660C); vm_run_wrapper
    // (0x55A4) consumes it only when the line was rejected, so a published line ignores it.
    ensure!(
        !text.control.arms_resume(),
        "unsupported_static_text: resume_control"
    );
    for word in &text.words {
        match word {
            ScriptTextWord::Dictionary(_) => {}
            ScriptTextWord::StateNumber(_) => bail!("unsupported_static_text: state_number"),
            ScriptTextWord::InventoryChoices => bail!("unsupported_static_text: inventory_choices"),
            ScriptTextWord::SectionSeparator if text.control.uses_history_condition() => {}
            ScriptTextWord::SectionSeparator => bail!("unsupported_static_text: word_sections"),
        }
    }
    ensure!(
        (-1..=31).contains(&text.presentation_selector),
        "unsupported_static_text: non_character_presentation_selector"
    );
    Ok(())
}

pub(super) fn bind_static_text(
    plan: &OfflineStaticTextPlan,
    profile: &LoadedScriptProfile,
) -> Result<BoundStaticText> {
    ensure!(plan.schema == 1, "unsupported static text plan schema");
    ensure!(
        plan.initial_profile == profile.id().value(),
        "static text profile mismatch"
    );
    ensure!(
        plan.game.script_dialect() == profile.code().dialect(),
        "static text game mismatch"
    );
    ensure!(
        sha256(&profile.code().encode()) == plan.cod_sha256,
        "static text COD hash mismatch"
    );
    ensure!(
        sha256(&profile.dictionary().encode()) == plan.dic_sha256,
        "static text DIC hash mismatch"
    );
    if let Some(expected) = &plan.bas_sha256 {
        ensure!(
            sha256(profile.dialogue().encoded_bytes()) == *expected,
            "static text BAS hash mismatch"
        );
    }
    let site = ScriptCodeOffset::new(plan.text_site);
    let (text, end, bytes) = match plan.source {
        StaticTextSource::Cod => {
            let token = profile
                .code()
                .tokens()
                .iter()
                .find(|token| token.source_offset() == site)
                .context("static text site is not a COD instruction boundary")?;
            let Some(DecodedScriptInstruction::Text(text)) = profile.instruction_at(site) else {
                bail!("static text site is not a COD A6 instruction");
            };
            (text, token.end_offset().index(), profile.code().encode())
        }
        StaticTextSource::Bas => {
            ensure!(
                plan.bas_sha256.is_some(),
                "static BAS text requires bas_sha256"
            );
            let token = profile
                .dialogue()
                .decoded()?
                .tokens()
                .iter()
                .find(|token| token.source_offset() == site)
                .context("static text site is not a BAS instruction boundary")?;
            let ScriptBasInstruction::Text(text) = token.instruction() else {
                bail!("static text site is not a BAS A6 instruction");
            };
            (
                text,
                token.end_offset().index(),
                profile.dialogue().encoded_bytes().to_vec(),
            )
        }
    };
    validate_plain_text(text)?;
    let history = history_candidates(text);
    let history_offsets = history
        .iter()
        .map(|word| profile.dictionary().source_offset(*word))
        .collect::<Option<Vec<_>>>()
        .context("static history concept has no DIC offset")?;
    ensure!(
        plan.context.history_concepts() == history_offsets.as_slice(),
        "static history concepts must equal the authored A6 history candidates"
    );
    let actor = profile
        .state()
        .objects()
        .iter()
        .find(|object| object.source_offset() == plan.context.actor_offset())
        .context("static presentation actor is not a VAR object boundary")?;
    ensure!(
        text.line_record.byte_offset() == actor.source_offset(),
        "unsupported_static_text: line_owner_differs_from_presentation_actor"
    );
    if let Some(choice) = plan.context.console_choice() {
        let builtins = profile.builtins();
        let expected = match choice {
            StaticConsoleChoice::Horn => builtins.horn,
            StaticConsoleChoice::Radio => builtins.menu,
        };
        ensure!(
            expected == Some(actor.id),
            "static console choice does not queue the line owner"
        );
    } else {
        ensure!(
            actor.kind == ScriptObjectKind::Actor,
            "unsupported_static_text: non_actor_context"
        );
        ensure!(
            !plan.context.descriptions().is_empty(),
            "static context requires DESCRIPT records"
        );
    }
    ensure!(
        plan.context
            .descriptions()
            .iter()
            .all(|name| !name.is_empty() && name.is_ascii()),
        "static DESCRIPT names must be nonempty ASCII"
    );
    let actor_name = profile
        .directory()
        .object(actor.id)
        .context("static actor has no active directory entry")?
        .name();
    ensure!(
        plan.context
            .descriptions()
            .last()
            .is_none_or(|name| name.as_bytes() == actor_name),
        "last static DESCRIPT record must name the presentation actor"
    );
    let encoded = &bytes[plan.text_site..end];
    Ok(BoundStaticText {
        text: text.clone(),
        actor: actor.id,
        history,
        authored: json!({
            "source": plan.source, "text_site": plan.text_site, "end_offset": end,
            "instruction_bytes": encoded, "instruction_sha256": sha256(encoded),
            "line_record_offset": text.line_record.byte_offset(),
            "presentation_selector": text.presentation_selector, "control_bits": text.control.bits(),
            "word_offsets": text.words.iter().map(|word| match word {
                ScriptTextWord::Dictionary(word) => profile.dictionary().source_offset(*word),
                _ => None,
            }).collect::<Vec<_>>(),
            "actor_name": String::from_utf8_lossy(actor_name),
            "history_concepts": history_offsets,
        }),
    })
}

/// Install non-actionable reciprocal C4 records; never execute an actor's code.
pub(super) fn prepare_static_actor(
    profile: &mut LoadedScriptProfile,
    actor: ScriptObjectId,
) -> Result<Value> {
    let before = profile.synchronized_state()?.encode();
    let parts = profile.execution_parts();
    let player = parts
        .builtins
        .player
        .context("static context has no player")?;
    for (owner, related) in [(player, actor), (actor, player)] {
        let kind = parts
            .state
            .object(owner)
            .context("static C4 owner disappeared")?
            .kind;
        let offset = script_field_offset(kind, ScriptFieldSelector::ACTION)
            .context("static C4 owner lacks an action field")?;
        let slot = parts
            .state
            .object_word_triple(owner, offset / 2)
            .context("static C4 action field is truncated")?;
        parts
            .record_state
            .action_records
            .set_record(slot, ScriptActionRecord::ActorPresentation(related));
        parts
            .record_state
            .action_records
            .set_actionable(slot, false);
    }
    parts
        .record_state
        .commit_to_var(parts.state, parts.directory, parts.dictionary)?;
    let flags = parts
        .state
        .object_word(actor, 1)
        .context("static actor flags are missing")?;
    let old = parts
        .state
        .word(flags)
        .context("static actor flags are unreadable")?;
    ensure!(
        parts.state.set_word(flags, old & !0x8000),
        "static actor flags are unwritable"
    );
    parts.selector_state.clear_presentation_branches();
    parts.selector_state.clear_concept_history();
    *parts.runtime = crate::native::bloodprg::ScriptRuntime::new();
    let after = profile.synchronized_state()?.encode();
    state_delta(&before, &after)
}

pub(super) fn state_delta(before: &[u8], after: &[u8]) -> Result<Value> {
    ensure!(
        before.len() == after.len(),
        "static setup changed the VAR layout"
    );
    Ok(json!({
        "before_sha256": sha256(before), "after_sha256": sha256(after),
        "byte_changes": before.iter().zip(after).enumerate()
            .filter(|(_, (old, new))| old != new)
            .map(|(offset, (old, new))| json!({"offset": offset, "before": old, "after": new}))
            .collect::<Vec<_>>(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use commander_blood_formats::instruction::{
        ScriptLineRecordOffset, ScriptTextControl, ScriptTextStateNumber,
    };
    use commander_blood_formats::script::decode_script_dictionary;

    fn plain(control: u16) -> ScriptText {
        let dictionary = decode_script_dictionary(b"hello\0").unwrap();
        ScriptText {
            line_record: ScriptLineRecordOffset::decode(0),
            presentation_selector: -1,
            control: ScriptTextControl::decode(control),
            resume_target: None,
            record_condition_operand: None,
            words: vec![ScriptTextWord::Dictionary(
                dictionary.resolve_source_offset(0).unwrap(),
            )]
            .into_boxed_slice(),
        }
    }

    #[test]
    fn static_plain_validation_does_not_rewrite_authored_control() {
        for bits in [0, 1, 0x20, 0x8000, 0x8021, 0x8008, 0xF008] {
            let text = plain(bits);
            let before = text.clone();
            validate_plain_text(&text).unwrap();
            assert_eq!(text, before);
        }
    }

    #[test]
    fn static_unsupported_controls_fail_closed() {
        for bits in [2, 4, 0x10, 0x40] {
            assert!(validate_plain_text(&plain(bits)).is_err(), "{bits:#x}");
        }
        for word in [
            ScriptTextWord::SectionSeparator,
            ScriptTextWord::InventoryChoices,
            ScriptTextWord::StateNumber(ScriptTextStateNumber::decode(100)),
        ] {
            let mut text = plain(0);
            text.words = vec![word].into_boxed_slice();
            assert!(validate_plain_text(&text).is_err());
        }
        let mut text = plain(0);
        text.words = Box::new([]);
        assert!(
            validate_plain_text(&text)
                .unwrap_err()
                .to_string()
                .contains("control_only")
        );
    }

    #[test]
    fn static_history_line_prepares_exactly_its_authored_candidates() {
        let dictionary = decode_script_dictionary(b"Hello\0hello\0hi\0").unwrap();
        let word = |offset| ScriptTextWord::Dictionary(dictionary.resolve_source_offset(offset).unwrap());
        // CB SCRIPT2 BAS 0x16EC shape: b4=0x41, "Hello friend..." | hello.
        let mut text = plain(0x8041);
        text.words = vec![word(0), ScriptTextWord::SectionSeparator, word(6), word(12)].into_boxed_slice();
        validate_plain_text(&text).unwrap();
        assert_eq!(
            history_candidates(&text),
            vec![
                dictionary.resolve_source_offset(6).unwrap(),
                dictionary.resolve_source_offset(12).unwrap()
            ]
        );
        assert!(history_candidates(&plain(0x8000)).is_empty());
        for words in [
            vec![word(0), ScriptTextWord::SectionSeparator],
            vec![ScriptTextWord::SectionSeparator, word(6)],
            vec![word(0), ScriptTextWord::SectionSeparator, word(6), ScriptTextWord::SectionSeparator, word(12)],
        ] {
            text.words = words.into_boxed_slice();
            assert!(validate_plain_text(&text).is_err());
        }
        // A resume-armed history line opens a reply menu; it stays unsupported.
        let mut resume = plain(0x8050);
        resume.words = vec![word(0), ScriptTextWord::SectionSeparator, word(6)].into_boxed_slice();
        assert!(validate_plain_text(&resume).is_err());
        // Without the history bit, separators remain unsupported.
        let mut sections = plain(0x8000);
        sections.words = vec![word(0), ScriptTextWord::SectionSeparator, word(6)].into_boxed_slice();
        assert!(validate_plain_text(&sections).is_err());
    }

    #[test]
    fn static_bridge_console_context_has_no_descriptions() {
        let plan = json!({"schema":1,"game":"commander_blood","title":"test","initial_profile":1,
            "cod_sha256":"cod","dic_sha256":"dic","source":"cod","text_site":17,
            "context":{"kind":"bridge_console","actor_offset":456,"choice":"horn"}});
        let parsed: OfflineStaticTextPlan = serde_json::from_value(plan.clone()).unwrap();
        assert_eq!(parsed.context.console_choice(), Some(StaticConsoleChoice::Horn));
        assert!(parsed.context.descriptions().is_empty());
        let mut bad = plan;
        bad["context"]["descript_records"] = json!(["Honk"]);
        assert!(serde_json::from_value::<OfflineStaticTextPlan>(bad).is_err());
    }

    #[test]
    fn static_plan_rejects_unknown_fields_and_snapshot_context() {
        let plan = json!({"schema":1,"game":"commander_blood","title":"test","initial_profile":0,
            "cod_sha256":"cod","dic_sha256":"dic","source":"bas","text_site":17,
            "context":{"kind":"source_default","actor_offset":456,"descript_records":["bob"]}});
        let parsed: OfflineStaticTextPlan = serde_json::from_value(plan.clone()).unwrap();
        assert_eq!(parsed.source, StaticTextSource::Bas);
        assert_eq!(parsed.text_site, 17);
        let mut bad = plan.clone();
        bad["force_conditions"] = json!(true);
        assert!(serde_json::from_value::<OfflineStaticTextPlan>(bad).is_err());
        let mut bad = plan;
        bad["context"]["kind"] = json!("snapshot");
        assert!(serde_json::from_value::<OfflineStaticTextPlan>(bad).is_err());
    }

    #[test]
    fn static_state_delta_retains_every_changed_byte() {
        let delta = state_delta(&[0, 1, 2], &[0, 3, 4]).unwrap();
        assert_eq!(
            delta["byte_changes"],
            json!([
            {"offset":1,"before":1,"after":3}, {"offset":2,"before":2,"after":4}])
        );
        assert!(state_delta(&[0], &[0, 1]).is_err());
    }
}
