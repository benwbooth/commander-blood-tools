//! Complete native transfer/descriptor vectors through dispatch and the real DES parser.

use commander_blood_formats::bas::decode_script_bas;
use commander_blood_formats::code::{ScriptDialect, decode_script_code_for_dialect};
use commander_blood_formats::descript_database::DescriptDatabase;
use commander_blood_formats::instruction::{ScriptLineRecordOffset, ScriptText, ScriptTextControl};
use commander_blood_formats::script::{
    ScriptObjectId, ScriptObjectKind, decode_script_dictionary, decode_script_directory,
    decode_script_state_for_dialect,
};
use serde::Deserialize;

use super::super::{
    AboardObjectRoster, DescriptApplicationContext, DescriptBackgroundCache,
    DescriptBackgroundSource, DescriptIdleClipSource, DescriptPresentationAssets,
    DescriptSoundBankLoader, ScriptActionDescription, ScriptActionRuntimeState,
    ScriptExecutionBackend, ScriptExecutionService, ScriptPresentationEntity, SequelInventoryLine,
    lookup_and_apply_descript_record,
};
use super::*;

#[derive(Deserialize)]
struct Vector {
    name: Vec<u8>,
    #[serde(default)]
    database: Vec<u8>,
    database_missing: bool,
    #[serde(default)]
    authored: bool,
    database_sha256: Option<String>,
    gate: u8,
    calls: Vec<u16>,
    vm_enabled: u8,
    start_locked: u8,
    request: u8,
    active_line: u16,
    selected: u16,
    resume: u8,
    alternate: u16,
    saved_line: u16,
    choices_head: u16,
    slots: Vec<u16>,
    holder: u16,
    object_flags: u16,
    line_flags: u8,
    video: Vec<u8>,
    caption: Vec<u8>,
    subtitle_active: u8,
    subtitle_cursor: u16,
}

// The captured object records must never request external resources at lookup.
// Fail on unexpected work instead of satisfying it with synthetic media.
struct NoResourceLoads;
impl DescriptBackgroundSource for NoResourceLoads {
    type Error = &'static str;
    fn load_background(&mut self, _: &[u8]) -> Result<Box<[u8]>, Self::Error> {
        Err("unexpected background load")
    }
}
impl DescriptSoundBankLoader for NoResourceLoads {
    type Error = &'static str;
    fn load_sound_bank(&mut self, _: &[u8]) -> Result<(), Self::Error> {
        Err("unexpected sound-bank load")
    }
}
impl DescriptIdleClipSource for NoResourceLoads {
    type Error = &'static str;
    fn load_idle_clip(&mut self, _: &[u8]) -> Result<Box<[u8]>, Self::Error> {
        Err("unexpected idle load")
    }
}

struct Backend {
    database: Option<DescriptDatabase>,
    assets: DescriptPresentationAssets,
    gate: u8,
    lookups: usize,
    fail_once: bool,
}

impl ScriptExecutionBackend for Backend {
    type Error = &'static str;
    fn environment_activity(&self) -> ScriptEnvironmentActivity {
        ScriptEnvironmentActivity {
            bridge_active: self.gate == 1,
            ..Default::default()
        }
    }
    fn apply_action_description(
        &mut self,
        _: ScriptObjectId,
        name: &[u8],
        text: &mut TextPresentationState,
    ) -> Result<ScriptActionDescription, Self::Error> {
        if std::mem::take(&mut self.fail_once) {
            return Err("descriptor backend failure");
        }
        self.lookups += 1;
        let Some(database) = &self.database else {
            // Explicit file-unavailable backend result, not an invented record.
            return Ok(ScriptActionDescription::default());
        };
        let mut cache = DescriptBackgroundCache::default();
        let mut background = NoResourceLoads;
        let mut sound = NoResourceLoads;
        let mut idle = NoResourceLoads;
        let mut context = DescriptApplicationContext::new(
            false,
            &mut self.assets,
            text,
            &mut cache,
            &mut background,
            &mut sound,
            &mut idle,
        );
        let application = lookup_and_apply_descript_record(database, name, &mut context)
            .expect("object descriptor requires no external loads");
        Ok(ScriptActionDescription {
            available: application.is_some(),
            ..Default::default()
        })
    }
    fn clock(&self) -> ScriptClock {
        panic!("unexpected clock access")
    }
    fn sequence_context(&self) -> SequenceRequestContext {
        panic!("unexpected sequence access")
    }
    fn navigation_context(&self) -> Option<ScriptRecordStateNavigationContext> {
        panic!("unexpected navigation")
    }
    fn action_runtime_state(&self) -> ScriptActionRuntimeState {
        panic!("unexpected action scan")
    }
    fn aboard_context(
        &mut self,
        _: ScriptObjectId,
    ) -> Result<ScriptAboardRecordContext, Self::Error> {
        panic!("unexpected aboard operation")
    }
    fn transfer_context(
        &mut self,
        _: ScriptObjectId,
    ) -> Result<ScriptTransferContext, Self::Error> {
        panic!("unexpected CD transfer")
    }
    fn lookup_presentation_description(
        &mut self,
        _: ScriptObjectId,
        _: &[u8],
        _: &mut TextPresentationState,
    ) -> Result<bool, Self::Error> {
        panic!("unexpected presentation scan")
    }
    fn restart_name_area_effect(&mut self) -> Result<(), Self::Error> {
        panic!("unexpected name effect")
    }
    fn transition_presentation_entity(
        &mut self,
        _: ScriptPresentationEntity,
    ) -> Result<(), Self::Error> {
        panic!("unexpected entity transition")
    }
    fn restart_navigation_music(&mut self) -> Result<(), Self::Error> {
        panic!("unexpected music restart")
    }
    fn play_radio_clip(&mut self, _: u16) -> Result<(), Self::Error> {
        panic!("unexpected radio clip")
    }
    fn start_camera_transition(&mut self, _: u8) -> Result<(), Self::Error> {
        panic!("unexpected camera transition")
    }
    fn reset_ship_hud(&mut self) -> Result<(), Self::Error> {
        panic!("unexpected HUD reset")
    }
}

fn run_vector(vector: Vector, database: &[u8]) {
    let mut directory_bytes = Vec::new();
    let mut bytes = Vec::new();
    for (name, kind) in [
        (b"blood".as_slice(), ScriptObjectKind::Player),
        (vector.name.as_slice(), ScriptObjectKind::InventoryItem),
        (b"recipient", ScriptObjectKind::Actor),
    ] {
        let mut entry = [0; 20];
        entry[..name.len()].copy_from_slice(name);
        entry[16..18].copy_from_slice(&(bytes.len() as u16).to_le_bytes());
        entry[18..20].copy_from_slice(&1u16.to_le_bytes());
        directory_bytes.extend(entry);
        let mut record = vec![0; kind.record_size_for_dialect(ScriptDialect::BigBugBang)];
        record[..2].copy_from_slice(&kind.mask().to_le_bytes());
        record[4..4 + name.len()].copy_from_slice(name);
        if kind == ScriptObjectKind::InventoryItem {
            record[2..4].copy_from_slice(&0x12u16.to_le_bytes());
            record[20..22].copy_from_slice(&u16::MAX.to_le_bytes());
        }
        bytes.extend(record);
    }
    directory_bytes.extend([0; 20]);
    let directory = decode_script_directory(&directory_bytes).unwrap();
    let mut state =
        decode_script_state_for_dialect(&bytes, &directory, ScriptDialect::BigBugBang).unwrap();
    let item = state.objects()[1].id;
    let recipient = state.objects()[2].id;
    let dictionary = decode_script_dictionary(b"OLD\0").unwrap();
    let previous = dictionary.words().next().unwrap().0;
    let dialogue = decode_script_bas(&[0xFF], &dictionary).unwrap();
    let code = decode_script_code_for_dialect(&[0xFF], ScriptDialect::BigBugBang).unwrap();
    let builtins = ScriptProfileBuiltins {
        player: Some(state.objects()[0].id),
        ..Default::default()
    };
    let mut records =
        ScriptProfileRecordState::recover(&[], &state, &dictionary, builtins).unwrap();
    let mut slots = [None; 16];
    slots[0] = Some(item);
    slots[2] = Some(item);
    *records.record_runtime.aboard_objects_mut() = AboardObjectRoster::from_test_slots(slots);
    let mut runtime = ScriptRuntime::default();
    runtime.arm_resume(ScriptCodeOffset::new(100), 0);
    assert!(runtime.activate_selector_resume());
    let mut selector = ScriptSelectorState::default();
    selector.history_mut().push(previous);
    let history = selector.history().clone();
    let mut dispatch = ScriptDispatchState::default();
    let line = SequelInventoryLine {
        instruction: ScriptCodeOffset::new(0),
        recipient,
    };
    selector
        .inventory_mut()
        .offer(
            line,
            records.record_runtime.aboard_objects(),
            &state,
            &mut runtime,
            &mut dispatch.text_presentation,
        )
        .unwrap();
    selector
        .inventory_mut()
        .complete_choice(Some(item), &mut runtime)
        .unwrap();
    dispatch.text_instructions.insert(
        line.instruction,
        TextInstructionState::new(&ScriptText {
            line_record: ScriptLineRecordOffset::decode(0),
            presentation_selector: 0,
            control: ScriptTextControl::decode(0x2130),
            resume_target: Some(ScriptCodeOffset::new(100)),
            record_condition_operand: None,
            words: Box::new([]),
        }),
    );
    dispatch.text_presentation.request_flags =
        super::super::PresentationRequestFlags::decode(if vector.gate == 2 { 2 } else { 0 });
    dispatch.text_presentation.subtitle_text = Box::from(b"BEFORE".as_slice());
    dispatch.text_presentation.subtitle_display_active = false;
    dispatch.text_presentation.subtitle_reveal_cursor = Some(0x1234);
    let mut host = ScriptExecutionService::new(Backend {
        database: (!vector.database_missing).then(|| DescriptDatabase::parse(database).unwrap()),
        assets: DescriptPresentationAssets::default(),
        gate: vector.gate,
        lookups: 0,
        fail_once: vector.gate == 0,
    });
    host.presentation_state_mut().start_locked = true;
    let mut procedures = super::super::ScriptProcedureStates::default();
    let mut sequence_slots = super::super::ScriptSequenceSlots::default();
    let mut dispatcher = Dispatcher {
        code: &code,
        instructions: &[],
        dialogue: &dialogue,
        state: &mut state,
        dictionary: &dictionary,
        directory: &directory,
        builtins,
        procedures: &mut procedures,
        selector: &mut selector,
        sequence_slots: &mut sequence_slots,
        records: &mut records,
        dispatch: &mut dispatch,
        host: &mut host,
    };
    if vector.gate == 0 {
        let failure = dispatcher.commit_selected_concept(&mut runtime);
        assert!(
            matches!(failure, Err(ScriptDispatchError::Host(_))),
            "{failure:?}"
        );
        assert_eq!(
            dispatcher.selector.inventory().descriptor_lookup(),
            Some(item)
        );
        assert_eq!(dispatcher.selector.inventory().selected(), Some(item));
        assert_eq!(
            dispatcher.records.record_runtime.aboard_objects().slots()[0],
            None
        );
        assert_eq!(
            dispatcher.records.record_runtime.aboard_objects().slots()[2],
            Some(item)
        );
        assert_eq!(dispatcher.dispatch.pending_vm_execution_write, None);
        assert_eq!(dispatcher.dispatch.pending_active_line_write(), None);
    }
    dispatcher.commit_selected_concept(&mut runtime).unwrap();
    let after_transfer = dispatcher.state.clone();
    dispatcher.commit_selected_concept(&mut runtime).unwrap();
    assert_eq!(dispatcher.state, &after_transfer);
    assert_eq!(selector.inventory().descriptor_lookup(), None);
    let completed = selector.inventory().clone();
    assert_eq!(
        selector.inventory_mut().complete_descriptor_lookup(),
        Err(super::super::SequelInventoryError::NoPendingDescriptor)
    );
    assert_eq!(selector.inventory(), &completed);
    assert_eq!(
        selector.inventory().selected().is_none(),
        vector.selected == 0
    );
    assert_eq!(
        selector.inventory().saved_line().is_none(),
        vector.saved_line == 0
    );
    assert_eq!(
        selector.inventory().choices().is_empty(),
        vector.choices_head == 0
    );
    assert_eq!(selector.history(), &history);
    assert_eq!(runtime.resume_state().is_none(), vector.resume == 0);
    assert_eq!(runtime.alternate_concept().is_none(), vector.alternate == 0);
    let flags = state.object_word(item, 1).unwrap();
    let holder = state.object_word(item, 10).unwrap();
    assert_eq!(state.word(flags), Some(vector.object_flags));
    assert_eq!(vector.holder, 0x200);
    assert_eq!(
        state.word(holder),
        Some(state.object(recipient).unwrap().source_offset() as u16)
    );
    let mut expected_bytes = bytes;
    let offset = state.object(item).unwrap().source_offset();
    expected_bytes[offset + 2..offset + 4].copy_from_slice(&vector.object_flags.to_le_bytes());
    expected_bytes[offset + 20..offset + 22]
        .copy_from_slice(&state.word(holder).unwrap().to_le_bytes());
    assert_eq!(
        state,
        decode_script_state_for_dialect(&expected_bytes, &directory, ScriptDialect::BigBugBang)
            .unwrap()
    );
    assert_eq!(
        records.record_runtime.aboard_objects().slots().as_slice(),
        vector
            .slots
            .iter()
            .map(|&slot| (slot != 0).then_some(item))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        dispatch.text_instructions[&line.instruction].is_active(),
        vector.line_flags & 0x80 != 0
    );
    assert_eq!(
        dispatch.pending_vm_execution_write.unwrap_or(true),
        vector.vm_enabled != 0
    );
    assert_eq!(
        dispatch.pending_active_line_write().unwrap_or(0x1234),
        vector.active_line
    );
    assert_eq!(
        host.presentation_state().start_locked,
        vector.start_locked != 0
    );
    assert_eq!(
        dispatch.text_presentation.request_flags.bits(),
        vector.request
    );
    assert_eq!(
        host.backend()
            .assets
            .object_scene_video()
            .unwrap_or(b"previous.hnm"),
        vector.video
    );
    assert_eq!(
        dispatch.text_presentation.subtitle_text.as_ref(),
        vector.caption
    );
    assert_eq!(
        dispatch.text_presentation.subtitle_display_active,
        vector.subtitle_active != 0
    );
    assert_eq!(
        dispatch
            .text_presentation
            .subtitle_reveal_cursor
            .unwrap_or(0),
        usize::from(vector.subtitle_cursor)
    );
    assert_eq!(
        host.backend().lookups,
        usize::from(vector.calls.contains(&0x8450))
    );
}

fn vectors() -> impl Iterator<Item = Vector> {
    include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_inventory_descriptor.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
}

#[test]
fn sequel_inventory_descriptor_dispatch_matches_all_synthetic_native_transfers() {
    let mut count = 0;
    for vector in vectors().filter(|vector| !vector.authored) {
        let database = vector.database.clone();
        run_vector(vector, &database);
        count += 1;
    }
    assert_eq!(count, 20);
}

#[test]
#[ignore = "requires the original sequel DESCRIPT.DES database"]
fn sequel_inventory_descriptor_dispatch_matches_all_authored_object_descriptions() {
    use sha2::{Digest, Sha256};
    let database = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../output/big-bug-bang/imported-assets/resources/DESCRIPT.DES"),
    )
    .unwrap();
    let hash = format!("{:x}", Sha256::digest(&database));
    let mut count = 0;
    for vector in vectors().filter(|vector| vector.authored) {
        assert_eq!(vector.database_sha256.as_deref(), Some(hash.as_str()));
        run_vector(vector, &database);
        count += 1;
    }
    assert_eq!(count, 25);
}
