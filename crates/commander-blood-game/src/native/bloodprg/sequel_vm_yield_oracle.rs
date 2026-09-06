//! Compare the original COD traversal between pre-frame preparation and post-scans.

use super::*;
use commander_blood_formats::bas::decode_script_bas;
use commander_blood_formats::code::{ScriptDialect, decode_script_code_for_dialect};
use commander_blood_formats::instruction::decode_complete_script_instruction;
use commander_blood_formats::script::{
    decode_script_dictionary, decode_script_directory, decode_script_state_for_dialect,
};
use serde::Deserialize;
use std::convert::Infallible;

#[derive(Deserialize)]
struct Vector {
    name: String,
    mode: String,
    gate: String,
    locked_before: u8,
    cod: String,
    var: String,
    deb: String,
    dic: String,
    cod_after: String,
    var_after: String,
    vm: u8,
    start_locked: u8,
    c2_gate: u8,
    yield_signals: Vec<u8>,
    entries: Vec<usize>,
    end_marker: bool,
    cursor: usize,
    resume: u8,
    saved_cursor: usize,
    request: u8,
}

fn hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

struct ScanBoundary {
    prepared: bool,
    reached: bool,
    presentation: ScriptPresentationScanState,
    state_at_boundary: Vec<u8>,
}

impl ScriptDispatchHost for ScanBoundary {
    type Error = Infallible;

    fn prepare_script_state(&mut self, _: ScriptPreFrameContext<'_>) -> Result<(), Self::Error> {
        assert!(!self.prepared);
        self.prepared = true;
        Ok(())
    }

    fn scan_presentation(&mut self, context: ScriptPostScanContext<'_>) -> Result<(), Self::Error> {
        assert!(self.prepared && !self.reached);
        self.reached = true;
        self.state_at_boundary = context.state.encode();
        context
            .dispatch
            .export_presentation_scan_state(&mut self.presentation);
        Ok(())
    }

    fn selector_root(&self) -> Option<ScriptCodeOffset> {
        None
    }
    fn environment_activity(&self) -> ScriptEnvironmentActivity {
        panic!("no environment opcode at this boundary")
    }
    fn clock(&self) -> ScriptClock {
        panic!("no clock opcode at this boundary")
    }
    fn sequence_context(&self) -> SequenceRequestContext {
        panic!("no sequence opcode at this boundary")
    }
    fn navigation_context(&self) -> Option<ScriptRecordStateNavigationContext> {
        panic!("no navigation opcode at this boundary")
    }
    fn aboard_context(
        &mut self,
        _: commander_blood_formats::script::ScriptObjectId,
    ) -> Result<ScriptAboardRecordContext, Self::Error> {
        panic!("no aboard opcode at this boundary")
    }
    fn transfer_context(
        &mut self,
        _: commander_blood_formats::script::ScriptObjectId,
    ) -> Result<ScriptTransferContext, Self::Error> {
        panic!("no transfer opcode at this boundary")
    }
}

#[test]
fn sequel_a6_outer_loop_matches_original_vm_pause_and_handoff_lock() {
    let vectors: Vec<Vector> =
        include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_vm_yield.jsonl")
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
    assert_eq!(vectors.len(), 76);
    for vector in vectors {
        let code_bytes = hex(&vector.cod);
        let directory = decode_script_directory(&hex(&vector.deb)).unwrap();
        let dictionary = decode_script_dictionary(&hex(&vector.dic)).unwrap();
        let mut state = decode_script_state_for_dialect(
            &hex(&vector.var),
            &directory,
            ScriptDialect::BigBugBang,
        )
        .unwrap();
        let code = decode_script_code_for_dialect(&code_bytes, ScriptDialect::BigBugBang).unwrap();
        let dialogue = decode_script_bas(&[0xFF], &dictionary).unwrap();
        let instructions = code
            .tokens()
            .iter()
            .map(|token| {
                decode_complete_script_instruction(token, &state, &directory, &dictionary).unwrap()
            })
            .collect::<Vec<_>>();
        let builtins = ScriptProfileBuiltins {
            player: directory.find_active_object(b"blood"),
            ..Default::default()
        };
        let mut records =
            ScriptProfileRecordState::recover(&[], &state, &dictionary, builtins).unwrap();
        let mut slots = [None; 16];
        if vector.gate != "empty" {
            slots[0] = directory.find_active_object(b"item");
        }
        *records.record_runtime.aboard_objects_mut() =
            super::super::AboardObjectRoster::from_test_slots(slots);
        let mut host = ScanBoundary {
            prepared: false,
            reached: false,
            state_at_boundary: Vec::new(),
            presentation: ScriptPresentationScanState {
                start_locked: vector.locked_before != 0,
                c2_gate_active: true,
                hold_ready: true,
                ..Default::default()
            },
        };
        let mut dispatch = ScriptDispatchState::default();
        dispatch.begin_frame();
        dispatch.import_presentation_scan_state(&host.presentation);
        dispatch.text_presentation.yield_signal = 9;
        dispatch.text_presentation.subtitle_word_list_mode = vector.mode == "subtitle";
        dispatch.text_presentation.subtitle_display_active = vector.gate == "subtitle";
        dispatch.text_presentation.menu_deferred = vector.gate == "menu";
        dispatch.text_presentation.request_flags =
            super::super::PresentationRequestFlags::decode(0x40);
        let mut runtime = ScriptRuntime::default();
        let mut selector = ScriptSelectorState::default();
        let mut procedures = super::super::ScriptProcedureStates::default();
        let mut sequence_slots = super::super::ScriptSequenceSlots::default();
        let mut dispatcher = Dispatcher {
            code: &code,
            instructions: &instructions,
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
        let outcome =
            execute_decoded_script_frame(&code, &instructions, true, &mut runtime, &mut dispatcher)
                .unwrap();
        assert!(host.reached, "{}", vector.name);
        assert_eq!(
            outcome.executed_instructions,
            vector.entries.len(),
            "{}",
            vector.name
        );
        assert_eq!(outcome.skipped_instructions, 0);
        assert_eq!(
            outcome.presentation_yields,
            vector
                .yield_signals
                .iter()
                .filter(|&&value| value != 0)
                .count(),
            "{}",
            vector.name
        );
        assert_eq!(
            outcome.next_instruction,
            Some(ScriptCodeOffset::new(vector.cursor)),
            "{}",
            vector.name
        );
        assert_eq!(
            outcome.end,
            if vector.end_marker {
                ScriptFrameEnd::EndMarker
            } else {
                ScriptFrameEnd::ResumeBoundary
            },
            "{}",
            vector.name
        );
        assert_eq!(
            host.state_at_boundary,
            hex(&vector.var_after),
            "{}",
            vector.name
        );
        assert_eq!(
            dispatch.pending_vm_execution_write.unwrap_or(true),
            vector.vm != 0,
            "{}",
            vector.name
        );
        assert_eq!(
            host.presentation.start_locked,
            vector.start_locked != 0,
            "{}",
            vector.name
        );
        assert_eq!(
            host.presentation.c2_gate_active,
            vector.c2_gate != 0,
            "{}",
            vector.name
        );
        assert_eq!(
            dispatch.text_presentation.yield_signal,
            *vector.yield_signals.last().unwrap(),
            "{}",
            vector.name
        );
        assert_eq!(
            dispatch.text_presentation.request_flags.bits(),
            vector.request,
            "{}",
            vector.name
        );
        assert_eq!(
            runtime.selector_resume_active(),
            vector.resume & 2 != 0,
            "{}",
            vector.name
        );
        if vector.resume & 2 != 0 {
            assert_eq!(
                runtime.saved_resume_cursor(),
                Some(ScriptCodeOffset::new(vector.saved_cursor)),
                "{}",
                vector.name
            );
        }
        let mut encoded = code_bytes;
        for (token, &entry) in code.tokens().iter().zip(&vector.entries) {
            assert_eq!(token.source_offset(), ScriptCodeOffset::new(entry));
            let high_flags = &mut encoded[entry + 5];
            *high_flags = (*high_flags & 0x7F)
                | (u8::from(dispatch.text_instructions[&token.source_offset()].is_active()) << 7);
        }
        assert_eq!(encoded, hex(&vector.cod_after), "{}", vector.name);
        // A later post-scan owner can release the lock; the old yield is consumed.
        host.presentation.start_locked = false;
        dispatch.export_presentation_scan_state(&mut host.presentation);
        assert!(!host.presentation.start_locked);
    }
}
