//! Device-free execution of the production main lifecycle for authored sequences.

use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};
use commander_blood_formats::code::ScriptCodeOffset;
use commander_blood_formats::descript::DescriptRecordKind;
use commander_blood_formats::descript_database::DescriptDatabase;
use commander_blood_formats::instruction::{
    DecodedScriptInstruction, ScriptSequenceSlot, ScriptSequenceSlotName, ScriptTextWord,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::input::INITIAL_LOGICAL_POINTER;
use super::platform::GamePitClock;
use super::{
    GAME_FRAME_DURATION, ModernGameServices, OfflinePresentationInterval, OfflinePresentationSink,
    PRESENTATION_FRAME_DURATION, RuntimeAlienOverlayFrameInput, RuntimeAudioHost,
    RuntimeGameLifecycleHost, RuntimePlatformDriver,
};
use crate::native::bloodprg::{
    GameLifecycleState, GameSession, InputAction, LoadedScriptProfile, PointerButtons,
    PointerSample, PresentationWordChoicePhase, SCRIPT_SEQUENCE_SAVE_BLOCK_BYTE_COUNT, ScriptClock,
    ScriptSequenceSlots, initialize_game_runtime, run_game_runtime_frame, shutdown_game,
};

pub(super) trait OfflineGameSink: OfflinePresentationSink {
    fn write_native_state(&mut self, time_ns: u64, state: &Value) -> Result<()>;
}

/// Exact source bindings and semantic choices for one native dialogue chapter.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OfflineDialogueChapter {
    pub schema: u8,
    pub game: crate::game::GameVariant,
    pub title: String,
    pub initial_profile: u8,
    pub cod_sha256: String,
    pub dic_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bas_sha256: Option<String>,
    pub target: String,
    #[serde(default, skip_serializing_if = "OfflineDialogueEntry::is_radio")]
    pub entry: OfflineDialogueEntry,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_procedure: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_encounter_guard: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub travel_setup: Option<OfflineTravelSetup>,
    pub choices: Vec<OfflineDialogueChoice>,
    #[serde(default, skip_serializing_if = "no_exit_retries")]
    pub max_exit_retries: u8,
    pub required_cod_sites: Vec<usize>,
    pub required_frame_boundary_cod_sites: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expected_unpublished_cod_sites: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_bas_sites: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_frame_boundary_bas_sites: Vec<usize>,
    pub end: OfflineDialogueEnd,
}

fn no_exit_retries(value: &u8) -> bool {
    *value == 0
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum OfflineDialogueEntry {
    #[default]
    Radio,
    Contact,
    Travel,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OfflineTravelSetup {
    pub planet: String,
    pub destination: String,
    pub procedure_offset: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supporting_procedures: Vec<usize>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub stage_actor_at_destination: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stage_aboard_inventory: Vec<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_evolution_guard: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_encounter_guard: Option<usize>,
}

fn is_false(value: &bool) -> bool {
    !value
}

impl OfflineDialogueEntry {
    fn is_radio(&self) -> bool {
        matches!(self, Self::Radio)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OfflineDialogueChoice {
    #[serde(default, skip_serializing_if = "OfflineDialogueChoiceSource::is_cod")]
    pub source: OfflineDialogueChoiceSource,
    pub text_site: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub word_offset: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory_item: Option<u16>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum OfflineDialogueChoiceSource {
    #[default]
    Cod,
    Bas,
    BasMenu,
    Inventory,
    InventoryCancel,
}

impl OfflineDialogueChoiceSource {
    fn is_cod(&self) -> bool {
        matches!(self, Self::Cod)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum OfflineDialogueEnd {
    PresentationFinished,
    ProfileLoaded { profile: u8 },
}

pub(super) fn validate_dialogue_chapter(
    chapter: &OfflineDialogueChapter,
    profile: &LoadedScriptProfile,
) -> Result<()> {
    ensure!(chapter.schema == 1, "unsupported dialogue chapter schema");
    ensure!(
        chapter.initial_profile == profile.id().value(),
        "wrong initial dialogue profile"
    );
    ensure!(
        format!("{:x}", Sha256::digest(profile.code().encode())) == chapter.cod_sha256,
        "dialogue chapter COD hash does not match the loaded profile"
    );
    ensure!(
        format!("{:x}", Sha256::digest(profile.dictionary().encode())) == chapter.dic_sha256,
        "dialogue chapter DIC hash does not match the loaded profile"
    );
    if let Some(procedure) = chapter.contact_procedure {
        ensure!(
            matches!(chapter.entry, OfflineDialogueEntry::Contact),
            "contact preparation requires contact entry"
        );
        super::contact_scenario::validate_contact_chapter(profile, procedure, &chapter.target)?;
    }
    if let Some(offset) = chapter.contact_encounter_guard {
        let procedure = chapter
            .contact_procedure
            .context("contact encounter guard requires prepared contact entry")?;
        super::contact_scenario::contact_encounter_guard(
            profile,
            procedure,
            &chapter.target,
            offset,
        )?;
    }
    ensure!(
        matches!(chapter.entry, OfflineDialogueEntry::Travel) == chapter.travel_setup.is_some(),
        "travel entry requires travel setup, and other entries must not contain it"
    );
    if let Some(setup) = &chapter.travel_setup {
        super::contact_scenario::validate_travel_chapter(
            profile,
            setup.procedure_offset,
            &chapter.target,
        )?;
        super::contact_scenario::travel_supporting_procedures(
            profile,
            setup.procedure_offset,
            &chapter.target,
            &setup.supporting_procedures,
        )?;
        if let Some(offset) = setup.actor_evolution_guard {
            ensure!(
                chapter.game == crate::game::GameVariant::BigBugBang,
                "evolution preparation currently covers BBB only"
            );
            super::contact_scenario::travel_actor_evolution_guard(
                profile,
                setup.procedure_offset,
                &chapter.target,
                &setup.supporting_procedures,
                offset,
            )?;
        }
        if let Some(offset) = setup.actor_encounter_guard {
            ensure!(
                chapter.game == crate::game::GameVariant::BigBugBang,
                "travel encounter preparation currently covers BBB only"
            );
            super::contact_scenario::travel_actor_encounter_guard(
                profile,
                setup.procedure_offset,
                &chapter.target,
                offset,
            )?;
        }
        if setup.stage_actor_at_destination {
            ensure!(
                chapter.game == crate::game::GameVariant::BigBugBang,
                "explicit travel actor placement currently covers BBB only"
            );
            super::contact_scenario::travel_actor_destination(
                profile,
                &chapter.target,
                &setup.planet,
                &setup.destination,
            )?;
        }
        if !setup.stage_aboard_inventory.is_empty() {
            ensure!(
                chapter.game == crate::game::GameVariant::BigBugBang,
                "explicit inventory staging currently covers BBB only"
            );
            super::contact_scenario::validate_aboard_inventory(
                profile,
                &setup.stage_aboard_inventory,
            )?;
        }
        for name in [&setup.planet, &setup.destination] {
            ensure!(
                profile
                    .directory()
                    .find_active_object(name.as_bytes())
                    .is_some(),
                "travel setup references an unknown destination"
            );
        }
    }
    let uses_bas = !chapter.required_bas_sites.is_empty()
        || chapter.choices.iter().any(|choice| {
            matches!(
                choice.source,
                OfflineDialogueChoiceSource::Bas | OfflineDialogueChoiceSource::BasMenu
            )
        });
    ensure!(
        !uses_bas || chapter.bas_sha256.is_some(),
        "BAS chapter requires a source hash"
    );
    if let Some(hash) = &chapter.bas_sha256 {
        ensure!(
            format!("{:x}", Sha256::digest(profile.dialogue().encoded_bytes())) == *hash,
            "dialogue chapter BAS hash does not match the loaded profile"
        );
    }
    ensure!(
        !chapter.required_cod_sites.is_empty() || !chapter.required_bas_sites.is_empty(),
        "dialogue chapter has no required text sites"
    );
    ensure!(
        (!chapter.required_frame_boundary_cod_sites.is_empty()
            || !chapter.required_frame_boundary_bas_sites.is_empty())
            && chapter
                .required_frame_boundary_cod_sites
                .iter()
                .all(|site| chapter.required_cod_sites.contains(site)),
        "frame-boundary dialogue sites must be a nonempty subset of the required publications"
    );
    ensure!(
        chapter
            .required_frame_boundary_bas_sites
            .iter()
            .all(|site| chapter.required_bas_sites.contains(site)),
        "frame-boundary BAS sites must be required publications"
    );
    for &offset in &chapter.required_bas_sites {
        ensure!(
            matches!(
                bas_instruction(profile, offset)?,
                commander_blood_formats::bas::ScriptBasInstruction::Text(_)
            ),
            "required BAS site {offset:#x} is not a text instruction"
        );
    }
    for &offset in &chapter.required_cod_sites {
        ensure!(
            matches!(
                profile.instruction_at(ScriptCodeOffset::new(offset)),
                Some(DecodedScriptInstruction::Text(_))
            ),
            "required COD site {offset:#x} is not a text instruction"
        );
    }
    for &offset in &chapter.expected_unpublished_cod_sites {
        ensure!(
            !chapter.required_cod_sites.contains(&offset),
            "dialogue site cannot be both required and unpublished"
        );
        ensure!(
            matches!(
                profile.instruction_at(ScriptCodeOffset::new(offset)),
                Some(DecodedScriptInstruction::Text(_))
            ),
            "unpublished COD site {offset:#x} is not a text instruction"
        );
    }
    for choice in &chapter.choices {
        if matches!(
            choice.source,
            OfflineDialogueChoiceSource::Inventory | OfflineDialogueChoiceSource::InventoryCancel
        ) {
            ensure!(
                chapter.game == crate::game::GameVariant::BigBugBang
                    && choice.word_offset.is_none(),
                "inventory selection requires BBB and cannot name a dictionary word"
            );
            if matches!(choice.source, OfflineDialogueChoiceSource::Inventory) {
                let item = choice
                    .inventory_item
                    .as_ref()
                    .context("inventory choice needs an item")?;
                super::contact_scenario::validate_aboard_inventory(
                    profile,
                    std::slice::from_ref(item),
                )?;
            } else {
                ensure!(
                    choice.inventory_item.is_none(),
                    "inventory cancellation cannot name an item"
                );
            }
            let Some(DecodedScriptInstruction::Text(text)) =
                profile.instruction_at(ScriptCodeOffset::new(choice.text_site))
            else {
                bail!("inventory choice site is not a COD text instruction");
            };
            ensure!(
                text.words.contains(&ScriptTextWord::InventoryChoices),
                "choice site does not offer native inventory objects"
            );
            let recipient = profile
                .directory()
                .find_active_object(chapter.target.as_bytes())
                .context("inventory recipient is not a chapter actor")?;
            ensure!(
                profile.state().object(recipient).unwrap().source_offset()
                    == usize::from(text.line_record.byte_offset()),
                "inventory choice belongs to another recipient"
            );
            continue;
        }
        ensure!(
            choice.inventory_item.is_none(),
            "dictionary choice cannot name an inventory item"
        );
        let word = profile
            .dictionary()
            .resolve_source_offset(
                choice
                    .word_offset
                    .context("dictionary choice needs a word offset")?,
            )
            .context("choice word offset is not a dictionary boundary")?;
        let text = match choice.source {
            OfflineDialogueChoiceSource::Cod => {
                let Some(DecodedScriptInstruction::Text(text)) =
                    profile.instruction_at(ScriptCodeOffset::new(choice.text_site))
                else {
                    bail!(
                        "choice site {:#x} is not a COD text instruction",
                        choice.text_site
                    );
                };
                text
            }
            OfflineDialogueChoiceSource::Bas => {
                let commander_blood_formats::bas::ScriptBasInstruction::Text(text) =
                    bas_instruction(profile, choice.text_site)?
                else {
                    bail!(
                        "choice site {:#x} is not a BAS text instruction",
                        choice.text_site
                    );
                };
                text
            }
            OfflineDialogueChoiceSource::BasMenu => {
                let commander_blood_formats::bas::ScriptBasInstruction::Menu(words) =
                    bas_instruction(profile, choice.text_site)?
                else {
                    bail!("choice site is not a BAS menu header");
                };
                ensure!(
                    words.contains(&word),
                    "choice word is not in the authored BAS menu"
                );
                continue;
            }
            OfflineDialogueChoiceSource::Inventory
            | OfflineDialogueChoiceSource::InventoryCancel => {
                unreachable!("inventory handled above")
            }
        };
        ensure!(
            text.words
                .split(|word| *word == ScriptTextWord::SectionSeparator)
                .skip(1)
                .flatten()
                .any(|value| *value == ScriptTextWord::Dictionary(word)),
            "requested word is not an authored choice at {:#x}",
            choice.text_site
        );
    }
    ensure!(
        chapter.max_exit_retries <= 8,
        "too many authored exit retries"
    );
    if chapter.max_exit_retries > 0 {
        let exit = chapter
            .choices
            .last()
            .context("exit retries need a final choice")?;
        let word = profile
            .dictionary()
            .resolve_source_offset(
                exit.word_offset
                    .context("exit retry needs a dictionary word")?,
            )
            .context("exit retry word disappeared")?;
        ensure!(
            matches!(exit.source, OfflineDialogueChoiceSource::BasMenu)
                && profile
                    .dictionary()
                    .word(word)
                    .unwrap()
                    .eq_ignore_ascii_case(b"bye_bye"),
            "only the authored BAS bye_bye menu choice may be retried"
        );
    }
    Ok(())
}

fn bas_instruction(
    profile: &LoadedScriptProfile,
    offset: usize,
) -> Result<&commander_blood_formats::bas::ScriptBasInstruction> {
    profile
        .dialogue()
        .decoded()?
        .tokens()
        .iter()
        .find(|token| token.source_offset().index() == offset)
        .map(|token| token.instruction())
        .with_context(|| format!("no BAS instruction at {offset:#x}"))
}

pub(super) fn capture_dialogue_chapter(
    services: ModernGameServices<'_>,
    chapter: &OfflineDialogueChapter,
    max_frames: u64,
    sink: &mut dyn OfflineGameSink,
) -> Result<(Value, Vec<u8>)> {
    ensure!(
        services.runtime().data().game() == chapter.game,
        "dialogue chapter names another game"
    );
    let requested_profile = crate::native::bloodprg::ScriptProfileId::new_for_dialect(
        chapter.initial_profile,
        chapter.game.script_dialect(),
    )
    .context("invalid dialogue profile for this game")?;
    let mut host = RuntimeGameLifecycleHost::with_platform(
        services,
        OfflineGamePlatform::new(max_frames, sink),
        None,
        39,
        script_clock,
        None,
    );
    let mut lifecycle = GameLifecycleState::default();
    let mut session = GameSession::default();
    let capture = (|| {
        ensure!(
            initialize_game_runtime(&mut lifecycle, &mut host, &mut session)?.is_none(),
            "native initialization exited before dialogue"
        );
        ensure!(
            run_game_runtime_frame(&mut lifecycle, &mut host, &mut session)?.is_none(),
            "native first frame exited before dialogue selection"
        );
        let profile = host
            .services()
            .runtime()
            .current_profile()
            .context("initial profile was not loaded")?;
        let removed_sequences = profile.sequence_slots().encode_save_block();
        ensure!(
            !host.services().presentation_screen_state()?.active(),
            "startup panel opened before chapter selection"
        );
        host.services_mut()
            .runtime_mut()
            .current_profile_mut()
            .unwrap()
            .sequence_slots_mut()
            .restore_save_block(&[0; SCRIPT_SEQUENCE_SAVE_BLOCK_BYTE_COUNT])?;
        let mut closing_startup = false;
        let mut startup_closed = false;
        for _ in 0..max_frames {
            ensure!(
                run_game_runtime_frame(&mut lifecycle, &mut host, &mut session)?.is_none(),
                "native lifecycle exited while closing the startup panel"
            );
            if host.services().presentation_screen_state()?.active() && !closing_startup {
                closing_startup = host
                    .services_mut()
                    .begin_presentation_panel_close_if_open()?;
            }
            if closing_startup
                && !host.services().presentation_screen_state()?.active()
                && !lifecycle.presentation_mode
                && !lifecycle.navigation_rebuild_pending
            {
                startup_closed = true;
                break;
            }
        }
        ensure!(
            startup_closed,
            "native startup panel did not close before dialogue selection"
        );
        let mut profile_selection = None;
        if chapter.initial_profile != 0 {
            host.services_mut()
                .request_script_profile(requested_profile);
            for _ in 0..max_frames {
                ensure!(
                    run_game_runtime_frame(&mut lifecycle, &mut host, &mut session)?.is_none(),
                    "native lifecycle exited during chapter profile selection"
                );
                if host
                    .services()
                    .runtime()
                    .current_profile()
                    .map(LoadedScriptProfile::id)
                    == Some(requested_profile)
                    && lifecycle.pending_profile.is_none()
                    && !lifecycle.navigation_rebuild_pending
                {
                    profile_selection = Some(host.platform().elapsed_ns);
                    break;
                }
            }
            ensure!(
                profile_selection.is_some(),
                "chapter profile selection did not finish"
            );
        }
        let profile = host
            .services()
            .runtime()
            .current_profile()
            .context("chapter profile disappeared")?;
        validate_dialogue_chapter(chapter, profile)?;
        let target = profile
            .directory()
            .find_active_object(chapter.target.as_bytes())
            .context("dialogue target is not a named native object")?;
        let contact_preparation = if let Some(procedure) = chapter.contact_procedure {
            let before = crate::native::bloodprg::OriginalSaveGame::capture(profile)?.encode();
            super::contact_scenario::prepare_contact_for_chapter(
                host.services_mut().runtime_mut(),
                procedure,
                &chapter.target,
            )?;
            let encounter_preparation = if let Some(offset) = chapter.contact_encounter_guard {
                let profile = host
                    .services_mut()
                    .runtime_mut()
                    .current_profile_mut()
                    .unwrap();
                let (counter, value) = super::contact_scenario::contact_encounter_guard(
                    profile,
                    procedure,
                    &chapter.target,
                    offset,
                )?;
                let mut state = profile.synchronized_state()?;
                // Native C4 entry increments this counter before evaluating the body guard.
                ensure!(
                    state.set_word(counter, value - 1),
                    "contact encounter counter is unbound"
                );
                profile.replace_state(state)?;
                Some(serde_json::json!({
                    "offset": offset,
                    "at_presentation": value,
                    "before_entry": value - 1,
                }))
            } else {
                None
            };
            let after = crate::native::bloodprg::OriginalSaveGame::capture(
                host.services().runtime().current_profile().unwrap(),
            )?
            .encode();
            ensure!(
                before.len() == after.len(),
                "contact setup changed the save layout"
            );
            let changes = before.iter().zip(&after).enumerate()
                .filter(|(_, (old, new))| old != new)
                .map(|(offset, (old, new))| serde_json::json!({"offset": offset, "before": old, "after": new}))
                .collect::<Vec<_>>();
            let mut preparation = serde_json::json!({
                "procedure_offset": procedure,
                "before_save_sha256": format!("{:x}", Sha256::digest(&before)),
                "after_save_sha256": format!("{:x}", Sha256::digest(&after)),
                "save_byte_changes": changes,
                "scope": "selected contact procedure and authored entry predicates; prepared chapter state, not a gameplay route",
            });
            if let Some(encounter) = encounter_preparation {
                preparation["encounter_guard"] = encounter;
            }
            if chapter.game == crate::game::GameVariant::CommanderBlood {
                preparation["manifest_sha256"] = serde_json::json!(format!(
                    "{:x}",
                    Sha256::digest(include_bytes!(
                        "../../../../re/vm/contact-manifest/contact-manifest.json"
                    ))
                ));
            } else {
                preparation["cod_sha256"] = serde_json::json!(chapter.cod_sha256);
                preparation["guard_source"] = serde_json::json!("typed_cod_outer_guard");
            }
            Some(preparation)
        } else {
            None
        };
        let travel_preparation = if let Some(setup) = &chapter.travel_setup {
            let before = crate::native::bloodprg::OriginalSaveGame::capture(
                host.services().runtime().current_profile().unwrap(),
            )?
            .encode();
            host.services_mut()
                .teleport_arche_to_navigation_target(setup.planet.as_bytes())?;
            let profile = host
                .services_mut()
                .runtime_mut()
                .current_profile_mut()
                .unwrap();
            // The teleport writes VAR directly; refresh typed relations before guard preparation.
            profile.replace_state(profile.state().clone())?;
            super::contact_scenario::prepare_travel_for_chapter(
                profile,
                setup.procedure_offset,
                &chapter.target,
            )?;
            for procedure in super::contact_scenario::travel_supporting_procedures(
                profile,
                setup.procedure_offset,
                &chapter.target,
                &setup.supporting_procedures,
            )? {
                profile.procedures_mut().set_enabled(procedure, true)?;
            }
            if setup.stage_actor_at_destination {
                super::contact_scenario::stage_travel_actor(
                    profile,
                    &chapter.target,
                    &setup.planet,
                    &setup.destination,
                )?;
            }
            if !setup.stage_aboard_inventory.is_empty() {
                super::contact_scenario::stage_aboard_inventory(
                    profile,
                    &setup.stage_aboard_inventory,
                )?;
            }
            let encounter = if let Some(offset) = setup.actor_encounter_guard {
                let (field, value) = super::contact_scenario::travel_actor_encounter_guard(
                    profile,
                    setup.procedure_offset,
                    &chapter.target,
                    offset,
                )?;
                let before = profile.state().word(field).unwrap();
                let mut state = profile.synchronized_state()?;
                // Native C4 entry increments the counter before the story guard runs.
                ensure!(
                    state.set_word(field, value - 1),
                    "travel encounter counter disappeared"
                );
                profile.replace_state(state)?;
                Some(serde_json::json!({
                    "offset": offset, "before": before,
                    "before_entry": value - 1, "at_presentation": value,
                }))
            } else {
                None
            };
            let evolution = if let Some(offset) = setup.actor_evolution_guard {
                let (field, value) = super::contact_scenario::travel_actor_evolution_guard(
                    profile,
                    setup.procedure_offset,
                    &chapter.target,
                    &setup.supporting_procedures,
                    offset,
                )?;
                let before = profile.state().word(field).unwrap();
                let mut state = profile.synchronized_state()?;
                ensure!(
                    state.set_word(field, value),
                    "actor evolution field disappeared"
                );
                profile.replace_state(state)?;
                Some(serde_json::json!({"offset": offset, "before": before, "value": value}))
            } else {
                None
            };
            let after = crate::native::bloodprg::OriginalSaveGame::capture(profile)?.encode();
            ensure!(
                before.len() == after.len(),
                "travel setup changed the save layout"
            );
            let changes = before.iter().zip(&after).enumerate()
                .filter(|(_, (old, new))| old != new)
                .map(|(offset, (old, new))| serde_json::json!({"offset": offset, "before": old, "after": new}))
                .collect::<Vec<_>>();
            let mut preparation = serde_json::json!({
                "setup": setup,
                "before_save_sha256": format!("{:x}", Sha256::digest(&before)),
                "after_save_sha256": format!("{:x}", Sha256::digest(&after)),
                "save_byte_changes": changes,
                "scope": "authored outer travel guard and planet position; post-HUD chapter entry, not a gameplay route",
            });
            if let Some(evolution) = evolution {
                preparation["actor_evolution_guard"] = evolution;
            }
            if let Some(encounter) = encounter {
                preparation["actor_encounter_guard"] = encounter;
            }
            Some(preparation)
        } else {
            None
        };
        if host.services().pending_ship_presentation_owner() == Some(target) {
            host.services_mut().clear_pending_ship_presentation_owner();
        }
        let selection_method = match chapter.entry {
            OfflineDialogueEntry::Radio => {
                host.services_mut().load_radio_sound_bank()?;
                host.services_mut().defer_ship_actor_presentation(target);
                "typed C4 radio entry with original radio sound bank"
            }
            OfflineDialogueEntry::Contact => {
                host.services_mut().request_scene_transition(target)?;
                "typed CONTACTS scene transition"
            }
            OfflineDialogueEntry::Travel => {
                let setup = chapter.travel_setup.as_ref().unwrap();
                let directory = host
                    .services()
                    .runtime()
                    .current_profile()
                    .unwrap()
                    .directory();
                let planet = directory
                    .find_active_object(setup.planet.as_bytes())
                    .context("travel planet is not a named native object")?;
                let destination = directory
                    .find_active_object(setup.destination.as_bytes())
                    .unwrap();
                host.services_mut()
                    .begin_chapter_travel(planet, destination, &mut lifecycle)?;
                "native post-HUD travel presentation and automatic actor selection"
            }
        };
        let travel_music = if matches!(chapter.entry, OfflineDialogueEntry::Travel) {
            host.services()
                .script_backend()
                .assets()
                .music()
                .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
        } else {
            None
        };
        host.services_mut()
            .script_backend_mut()
            .observe_text_publications();
        host.platform_mut().begin_capture();
        let mut saw_target = false;
        let mut observed_sites = std::collections::BTreeSet::new();
        let mut frame_sites = std::collections::BTreeSet::new();
        let mut observed_bas_sites = std::collections::BTreeSet::new();
        let mut frame_bas_sites = std::collections::BTreeSet::new();
        let mut publications = Vec::new();
        let mut choices = Vec::new();
        for main_frame in 1..=max_frames {
            ensure!(
                run_game_runtime_frame(&mut lifecycle, &mut host, &mut session)?.is_none(),
                "native lifecycle exited before dialogue completion"
            );
            for event in host
                .services_mut()
                .script_backend_mut()
                .take_text_publications()
            {
                if event.profile == chapter.initial_profile {
                    if event.bas {
                        observed_bas_sites.insert(event.offset);
                    } else {
                        observed_sites.insert(event.offset);
                    }
                }
                publications.push(serde_json::json!({
                    "publication": event, "frame_end_ns": host.platform().elapsed_ns - host.platform().capture_origin_ns,
                }));
            }
            let profile = host
                .services()
                .runtime()
                .current_profile()
                .context("dialogue profile disappeared")?;
            let current_profile = profile.id().value();
            if current_profile == chapter.initial_profile {
                saw_target |= profile.active_actor_presentation_related() == Some(target)
                    && lifecycle.presentation.active;
                if let Some(site) = host.services().published_text_site() {
                    frame_sites.insert(site.index());
                }
                if let Some(site) = host.services().published_bas_text_site() {
                    frame_bas_sites.insert(site.index());
                }
                if host.services().presentation_word_choice_phase()?
                    == PresentationWordChoicePhase::Selecting
                {
                    let expected = chapter
                        .choices
                        .get(choices.len())
                        .or_else(|| {
                            (choices.len()
                                < chapter.choices.len() + usize::from(chapter.max_exit_retries))
                            .then(|| chapter.choices.last())
                            .flatten()
                        })
                        .context("unplanned native dialogue choice")?;
                    let source_site = match expected.source {
                        OfflineDialogueChoiceSource::Cod => host.services().published_text_site(),
                        OfflineDialogueChoiceSource::Bas => {
                            host.services().published_bas_text_site()
                        }
                        OfflineDialogueChoiceSource::BasMenu => profile
                            .selector_state()
                            .current_branch()
                            .map(|branch| branch.body),
                        OfflineDialogueChoiceSource::Inventory
                        | OfflineDialogueChoiceSource::InventoryCancel => profile
                            .selector_state()
                            .inventory()
                            .saved_line()
                            .map(|line| line.instruction),
                    };
                    ensure!(
                        source_site == Some(ScriptCodeOffset::new(expected.text_site)),
                        "native dialogue reached a different choice site: expected {:?} {:#x}, observed {:?}",
                        expected.source,
                        expected.text_site,
                        source_site
                    );
                    if matches!(
                        expected.source,
                        OfflineDialogueChoiceSource::InventoryCancel
                    ) {
                        host.services_mut().request_inventory_cancel()?;
                    } else {
                        let identity =
                            if matches!(expected.source, OfflineDialogueChoiceSource::Inventory) {
                                let item = super::contact_scenario::validate_aboard_inventory(
                                    profile,
                                    std::slice::from_ref(expected.inventory_item.as_ref().unwrap()),
                                )?[0]
                                    .object()
                                    .context("validated inventory item disappeared")?;
                                crate::native::bloodprg::PresentationChoiceId::Inventory(item)
                            } else {
                                let word = profile
                                    .dictionary()
                                    .resolve_source_offset(expected.word_offset.unwrap())
                                    .context("validated choice word disappeared")?;
                                crate::native::bloodprg::PresentationChoiceId::Dictionary(word)
                            };
                        host.services_mut().request_dialogue_choice(identity)?;
                    }
                    let mut selected = serde_json::to_value(expected)?;
                    selected["requested_at_ns"] = serde_json::json!(
                        host.platform().elapsed_ns - host.platform().capture_origin_ns
                    );
                    choices.push(selected);
                }
            }
            let contact_closed = !matches!(chapter.entry, OfflineDialogueEntry::Contact)
                || (host.services().runtime_scene_transition()?.state().phase
                    == crate::native::bloodprg::SceneTransitionPhase::Inactive
                    && !lifecycle.navigation_rebuild_pending);
            let travel_closed = !matches!(chapter.entry, OfflineDialogueEntry::Travel)
                || (host.services().ship_presentation_state().flags == 0
                    && !lifecycle.presentation.sequence_active
                    && !lifecycle.navigation_rebuild_pending);
            let completed = saw_target
                && contact_closed
                && travel_closed
                && match chapter.end {
                    OfflineDialogueEnd::PresentationFinished => !lifecycle.presentation.active,
                    OfflineDialogueEnd::ProfileLoaded { profile } => current_profile == profile,
                };
            if completed {
                ensure!(
                    choices.len() >= chapter.choices.len(),
                    "dialogue ended before all planned choices"
                );
                ensure!(
                    chapter
                        .required_cod_sites
                        .iter()
                        .all(|site| observed_sites.contains(site)),
                    "dialogue ended without publishing every required text site: observed {observed_sites:?}"
                );
                ensure!(
                    chapter
                        .required_bas_sites
                        .iter()
                        .all(|site| observed_bas_sites.contains(site)),
                    "dialogue ended without publishing every required BAS site: observed {observed_bas_sites:?}"
                );
                ensure!(
                    chapter
                        .required_frame_boundary_bas_sites
                        .iter()
                        .all(|site| frame_bas_sites.contains(site)),
                    "dialogue ended without every required frame-boundary BAS site"
                );
                ensure!(
                    chapter
                        .expected_unpublished_cod_sites
                        .iter()
                        .all(|site| !observed_sites.contains(site)),
                    "dialogue published a site declared absent on this branch"
                );
                ensure!(
                    chapter
                        .required_frame_boundary_cod_sites
                        .iter()
                        .all(|site| frame_sites.contains(site)),
                    "dialogue ended without every required frame-boundary text site: observed {frame_sites:?}"
                );
                let driver = host.platform();
                return Ok((
                    serde_json::json!({
                        "presented_frames": driver.captured_waits,
                        "duration_ns": driver.elapsed_ns - driver.capture_origin_ns,
                        "audio_samples": driver.sample_cursor - driver.capture_origin_sample,
                        "main_loop_frames": main_frame, "bootstrap_duration_ns": driver.capture_origin_ns,
                        "total_timer_ticks": driver.timer_ticks, "chapter": chapter,
                        "selection_method": selection_method,
                        "setup_scope": "native startup panel close; startup sequence slots cleared; optional native profile handoff; no pointer input; not a gameplay route",
                        "profile_selected_at_ns": profile_selection,
                        "contact_transition_closed": contact_closed,
                        "contact_preparation": contact_preparation,
                        "travel_preparation": travel_preparation,
                        "travel_music": travel_music,
                        "travel_transition_closed": travel_closed,
                        "removed_sequence_slots": removed_sequences.as_slice(),
                        "published_cod_sites": observed_sites, "frame_boundary_cod_sites": frame_sites,
                        "published_bas_sites": observed_bas_sites, "frame_boundary_bas_sites": frame_bas_sites,
                        "publications": publications, "choices": choices,
                        "final_profile": current_profile,
                    }),
                    host.services().read_offline_rgba()?,
                ));
            }
        }
        bail!("native dialogue exceeded its main-loop cap before completion")
    })();
    host.platform_mut().capturing = false;
    let cleanup = shutdown_game(&mut lifecycle, &mut host, session, false);
    match (capture, cleanup) {
        (Ok(capture), Ok(())) => Ok(capture),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(error), Err(cleanup)) => {
            Err(error.context(format!("cleanup also failed: {cleanup:#}")))
        }
    }
}

#[derive(Serialize)]
pub(super) struct OfflineStartupReport {
    pub presented_frames: u64,
    pub duration_ns: u64,
    pub audio_samples: u64,
    pub main_loop_frames: u64,
    pub completed_sequence_lists: u64,
    pub bootstrap_duration_ns: u64,
    pub total_timer_ticks: u64,
    pub sequence_record: String,
    pub authored_videos: Vec<String>,
    pub authored_music: Option<String>,
    pub caption_cues: Vec<Value>,
    pub selection_method: &'static str,
    pub selected_record_at_ns: Option<u64>,
}

fn script_clock() -> Result<ScriptClock> {
    Ok(ScriptClock {
        hour: 12,
        day: 2,
        month: 1,
    })
}

pub(super) fn capture_startup_cinematic(
    services: ModernGameServices<'_>,
    record: Option<&str>,
    max_frames: u64,
    sink: &mut dyn OfflineGameSink,
) -> Result<(OfflineStartupReport, Vec<u8>)> {
    ensure!(max_frames > 0, "offline frame cap must be nonzero");
    let selected_record = record
        .map(|name| {
            validate_sequence_record(
                services.runtime().data().descript_database(),
                name.as_bytes(),
            )
        })
        .transpose()?;
    let mut host = RuntimeGameLifecycleHost::with_platform(
        services,
        OfflineGamePlatform::new(max_frames, sink),
        None,
        39,
        script_clock,
        None,
    );
    let mut lifecycle = GameLifecycleState::default();
    let mut session = GameSession::default();
    let capture = (|| {
        ensure!(
            initialize_game_runtime(&mut lifecycle, &mut host, &mut session)?.is_none(),
            "native initialization exited before the cinematic"
        );
        let initial_completions = host.services().completed_presentation_sequence_lists()?;
        host.platform_mut().begin_capture();
        let mut selected_record_at_ns = None;
        for main_frame in 1..=max_frames {
            ensure!(
                run_game_runtime_frame(&mut lifecycle, &mut host, &mut session)?.is_none(),
                "native lifecycle exited before completing its cinematic sequence list"
            );
            if let Some(name) = &selected_record
                && selected_record_at_ns.is_none()
                && host.services().runtime().current_profile().is_some()
            {
                // The first frame loads the profile; the panel actor has not opened yet.
                // Set only its record slot, leaving native actor, palette, and audio startup intact.
                let panel = host.services().presentation_screen_state()?;
                ensure!(
                    !panel.active() && !panel.scene_status().queued,
                    "sequence panel started before the export record could be selected"
                );
                select_first_sequence(
                    host.services_mut()
                        .runtime_mut()
                        .current_profile_mut()
                        .context("initial script profile disappeared")?
                        .sequence_slots_mut(),
                    name,
                )?;
                selected_record_at_ns =
                    Some(host.platform().elapsed_ns - host.platform().capture_origin_ns);
            }
            let completed =
                host.services().completed_presentation_sequence_lists()? - initial_completions;
            if completed != 0 {
                let driver = host.platform();
                let services = host.services();
                let selected = services
                    .presentation_screen_state()?
                    .selected_choice()
                    .index();
                let records = services.presentation_sequence_records()?;
                let record = records[selected]
                    .as_ref()
                    .context("completed sequence has no record")?;
                let assets = services.script_backend().assets();
                let source_cues = assets.sequence_subtitles();
                let display_cues = services
                    .runtime()
                    .data()
                    .english_sequence_captions
                    .display(source_cues);
                let report = OfflineStartupReport {
                    presented_frames: driver.captured_waits,
                    duration_ns: driver.elapsed_ns - driver.capture_origin_ns,
                    audio_samples: driver.sample_cursor - driver.capture_origin_sample,
                    main_loop_frames: main_frame,
                    completed_sequence_lists: completed,
                    bootstrap_duration_ns: driver.capture_origin_ns,
                    total_timer_ticks: driver.timer_ticks,
                    sequence_record: String::from_utf8_lossy(record).into_owned(),
                    authored_videos: assets.sequence_videos().iter()
                        .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned()).collect(),
                    authored_music: assets.music().map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned()),
                    caption_cues: source_cues.iter().zip(display_cues).map(|(source, display)| serde_json::json!({
                        "authored_frame": source.first_visible_frame(), "source_bytes": source.text(),
                        "display_text": String::from_utf8_lossy(display.text()),
                    })).collect(),
                    selection_method: if selected_record.is_some() { "explicit DESCRIPT record in startup slot one; not a gameplay route" } else { "unmodified native startup" },
                    selected_record_at_ns,
                };
                if let Some(name) = &selected_record {
                    ensure!(
                        selected_record_at_ns.is_some() && record.as_ref() == name.as_bytes(),
                        "native sequence did not complete the requested record"
                    );
                }
                ensure!(report.presented_frames > 0, "cinematic emitted no frames");
                return Ok((report, host.services().read_offline_rgba()?));
            }
        }
        bail!("native cinematic exceeded its main-loop cap before completion")
    })();
    host.platform_mut().capturing = false;
    // Release native resources without appending an unrelated credits sequence.
    let cleanup = shutdown_game(&mut lifecycle, &mut host, session, false);
    match (capture, cleanup) {
        (Ok(capture), Ok(())) => Ok(capture),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(error), Err(cleanup)) => {
            Err(error.context(format!("cleanup also failed: {cleanup:#}")))
        }
    }
}

fn validate_sequence_record(
    database: &DescriptDatabase,
    name: &[u8],
) -> Result<ScriptSequenceSlotName> {
    let record = database.lookup(name).context("unknown DESCRIPT record")?;
    ensure!(
        record.kind() == DescriptRecordKind::Sequence,
        "DESCRIPT record is not a sequence"
    );
    ScriptSequenceSlotName::new(record.name()).context("sequence name does not fit its native slot")
}

fn select_first_sequence(
    slots: &mut ScriptSequenceSlots,
    name: &ScriptSequenceSlotName,
) -> Result<()> {
    ensure!(!name.as_bytes().is_empty(), "empty sequence name");
    let mut saved = slots.encode_save_block();
    let first = &mut saved[..SCRIPT_SEQUENCE_SAVE_BLOCK_BYTE_COUNT / ScriptSequenceSlot::COUNT];
    first.fill(0);
    first[..name.as_bytes().len()].copy_from_slice(name.as_bytes());
    slots
        .restore_save_block(&saved)
        .context("selecting the export sequence")
}

struct OfflineGamePlatform<'sink> {
    epoch: Instant,
    pit: GamePitClock,
    elapsed_ns: u64,
    sample_cursor: u64,
    timer_ticks: u64,
    total_waits: u64,
    captured_waits: u64,
    max_frames: u64,
    capturing: bool,
    capture_origin_ns: u64,
    capture_origin_sample: u64,
    pointer: [i16; 2],
    alien_pointer: Option<[f32; 2]>,
    sink: &'sink mut dyn OfflineGameSink,
}

impl<'sink> OfflineGamePlatform<'sink> {
    fn new(max_frames: u64, sink: &'sink mut dyn OfflineGameSink) -> Self {
        Self {
            epoch: Instant::now(),
            pit: GamePitClock::default(),
            elapsed_ns: 0,
            sample_cursor: 0,
            timer_ticks: 0,
            total_waits: 0,
            captured_waits: 0,
            max_frames,
            capturing: false,
            capture_origin_ns: 0,
            capture_origin_sample: 0,
            pointer: INITIAL_LOGICAL_POINTER,
            alien_pointer: None,
            sink,
        }
    }

    fn begin_capture(&mut self) {
        self.capturing = true;
        self.capture_origin_ns = self.elapsed_ns;
        self.capture_origin_sample = self.sample_cursor;
    }

    fn wait(&mut self, services: &mut ModernGameServices<'_>, duration: Duration) -> Result<()> {
        ensure!(
            self.total_waits < self.max_frames,
            "offline lifecycle exceeded its frame cap"
        );
        let duration_ns = u64::try_from(duration.as_nanos())?;
        let end = self
            .elapsed_ns
            .checked_add(duration_ns)
            .context("offline timeline overflow")?;
        let sample_end = (u128::from(end) * u128::from(RuntimeAudioHost::output_sample_rate_hz())
            / 1_000_000_000) as u64;
        let mut audio = vec![0.0; usize::try_from(sample_end - self.sample_cursor)?];
        services.render_offline_audio(&mut audio)?;
        if self.capturing {
            let rgba = services.read_offline_rgba()?;
            self.sink.write_interval(OfflinePresentationInterval {
                start_ns: self.elapsed_ns - self.capture_origin_ns,
                duration_ns,
                rgba: &rgba,
                audio: &audio,
            })?;
            self.captured_waits += 1;
        }
        self.elapsed_ns = end;
        self.sample_cursor = sample_end;
        self.total_waits += 1;
        Ok(())
    }
}

impl<'window> RuntimePlatformDriver<'window> for OfflineGamePlatform<'_> {
    fn dispatch_events(
        &mut self,
        services: &mut ModernGameServices<'window>,
        state: &mut GameLifecycleState,
    ) -> Result<Option<InputAction>> {
        services.dispatch_lifecycle_input(state)
    }
    fn dispatch_game_events(
        &mut self,
        services: &mut ModernGameServices<'window>,
        state: &mut GameLifecycleState,
    ) -> Result<Option<InputAction>> {
        self.dispatch_events(services, state)
    }
    fn record_scenario_frame_boundary(
        &mut self,
        services: &mut ModernGameServices<'window>,
        state: &mut GameLifecycleState,
    ) -> Result<()> {
        if self.capturing {
            self.sink.write_native_state(
                self.elapsed_ns - self.capture_origin_ns,
                &services.semantic_trace_snapshot(state)?,
            )?;
        }
        Ok(())
    }
    fn poll_alien_overlay_frame(
        &mut self,
        services: &mut ModernGameServices<'window>,
    ) -> Result<RuntimeAlienOverlayFrameInput> {
        let pointer = self
            .alien_pointer
            .context("alien pointer is not acquired")?;
        Ok(RuntimeAlienOverlayFrameInput::from_driver_pointer(
            pointer,
            PointerButtons::NONE,
            services.input_mut().drain_alien_key_events(false),
        ))
    }
    fn begin_alien_overlay_input(&mut self) -> Result<()> {
        ensure!(
            self.alien_pointer.is_none(),
            "alien pointer already acquired"
        );
        self.alien_pointer = Some([320.0, 512.0]);
        Ok(())
    }
    fn finish_alien_overlay_input(&mut self) -> bool {
        self.alien_pointer.take().is_some()
    }
    fn poll_pointer(&mut self, services: &mut ModernGameServices<'window>) -> PointerSample {
        services.publish_lifecycle_logical_pointer(self.pointer, PointerButtons::NONE)
    }
    fn logical_pointer(&self) -> [i16; 2] {
        self.pointer
    }
    fn synchronize_bridge_pointer(&mut self, position: [i16; 2]) {
        self.pointer = position;
    }
    fn start_game_timer(&mut self) {
        self.pit
            .start(self.epoch + Duration::from_nanos(self.elapsed_ns));
    }
    fn stop_game_timer(&mut self) {
        self.pit.stop();
    }
    fn take_game_timer_ticks(&mut self) -> u64 {
        let ticks = self
            .pit
            .take_elapsed_ticks(self.epoch + Duration::from_nanos(self.elapsed_ns));
        self.timer_ticks += ticks;
        ticks
    }
    fn take_bridge_horizontal_delta(&mut self) -> i32 {
        0
    }
    fn pace_frame(&mut self, services: &mut ModernGameServices<'window>) -> Result<()> {
        self.wait(services, GAME_FRAME_DURATION)
    }
    fn pace_presentation_frame(
        &mut self,
        services: &mut ModernGameServices<'window>,
    ) -> Result<()> {
        self.wait(services, PRESENTATION_FRAME_DURATION)
    }
    fn wait_for_visual_refresh(
        &mut self,
        services: &mut ModernGameServices<'window>,
    ) -> Result<Option<f32>> {
        let duration = if services.presentation_stream_active() {
            PRESENTATION_FRAME_DURATION
        } else {
            GAME_FRAME_DURATION
        };
        self.wait(services, duration)?;
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapter_selection_changes_only_first_slot() {
        let mut slots = ScriptSequenceSlots::default();
        let mut saved = slots.encode_save_block();
        saved[16..22].copy_from_slice(b"other\0");
        slots.restore_save_block(&saved).unwrap();
        for name in ["present", "48finbob", "22pubscandoig"] {
            let name = ScriptSequenceSlotName::new(name.as_bytes()).unwrap();
            select_first_sequence(&mut slots, &name).unwrap();
            assert_eq!(slots.ordered_names()[0], Some(name.as_bytes()));
            assert_eq!(&slots.encode_save_block()[16..], &saved[16..]);
        }
        let before = slots.clone();
        assert!(
            select_first_sequence(&mut slots, &ScriptSequenceSlotName::new(&b""[..]).unwrap())
                .is_err()
        );
        assert_eq!(slots, before);
    }

    #[test]
    #[ignore = "requires both imported game asset stores"]
    fn every_authored_sequence_can_be_selected_but_non_sequences_are_rejected() {
        let cb = std::path::PathBuf::from(std::env::var_os("CBLOOD_ASSET_CACHE").unwrap());
        let bbb = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../output/big-bug-bang/imported-assets");
        for (root, expected_count) in [(cb, 11), (bbb, 54)] {
            let bytes = std::fs::read(root.join("resources/DESCRIPT.DES")).unwrap();
            let database = DescriptDatabase::parse(&bytes).unwrap();
            let mut count = 0;
            for record in database.records() {
                let name = record.name();
                let selected = validate_sequence_record(&database, name);
                if record.kind() == DescriptRecordKind::Sequence {
                    let mut slots = ScriptSequenceSlots::default();
                    select_first_sequence(&mut slots, &selected.unwrap()).unwrap();
                    assert_eq!(slots.ordered_names()[0], Some(record.name()));
                    count += 1;
                } else {
                    assert!(selected.is_err(), "{}", String::from_utf8_lossy(name));
                }
            }
            assert_eq!(count, expected_count);
            assert!(validate_sequence_record(&database, b"not-a-record").is_err());
            assert!(validate_sequence_record(&database, b"").is_err());
        }
    }
}
