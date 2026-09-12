//! BBB executable comparison for the inherited A8 sequence-request handler.

use commander_blood_formats::instruction::ScriptSequenceRequest;
use serde::Deserialize;

use super::*;

#[derive(Debug, Deserialize)]
struct Vector {
    name: String,
    basename_hex: String,
    script_start: usize,
    cursor: usize,
    finale_before: u8,
    finale_after: u8,
    request_before: u8,
    request_after: u8,
    ship_flags: u16,
    scene_gate: u8,
    raised: bool,
    active_line_after: u16,
    presentation_gate_after: u8,
    loaded_image_after: u16,
    mouse_idle_after: u8,
}

fn bytes_from_hex(encoded: &str) -> Box<[u8]> {
    encoded
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

#[test]
fn sequel_a8_uses_shared_sequence_request_state() {
    const VECTOR_COUNT: usize = 64;

    let vectors: Vec<Vector> =
        include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_sequence_request.jsonl")
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
    assert_eq!(vectors.len(), VECTOR_COUNT);

    for vector in vectors {
        let basename = bytes_from_hex(&vector.basename_hex);
        let request = ScriptSequenceRequest::new(basename.clone()).unwrap();
        assert_eq!(request.basename(), basename.as_ref(), "{}", vector.name);

        let mut request_flags = PresentationRequestFlags::decode(vector.request_before);
        let mut state = SequencePresentationState {
            finale_requested: vector.finale_before != u8::MIN,
            presentation_gate_active: true,
            loaded_scene_image_valid: true,
            dialogue_gate_active: true,
            ..SequencePresentationState::default()
        };
        let raised = load_sequence_request(
            &request,
            SequenceRequestContext {
                ship_active: vector.ship_flags & 1 != u16::MIN,
                scene_gate_active: vector.scene_gate & 1 != u8::MIN,
            },
            &mut request_flags,
            &mut state,
        );

        assert_eq!(raised, vector.raised, "{}", vector.name);
        assert_eq!(state.sequence_basename, basename, "{}", vector.name);
        assert_eq!(
            state.finale_requested,
            vector.finale_after != u8::MIN,
            "{}",
            vector.name
        );
        assert_eq!(
            request_flags.bits(),
            vector.request_after,
            "{}",
            vector.name
        );
        assert_eq!(
            state.active_resource_line,
            (vector.active_line_after == PresentationResourceLine::Sequence.number())
                .then_some(PresentationResourceLine::Sequence),
            "{}",
            vector.name
        );
        assert_eq!(
            state.presentation_gate_active,
            vector.presentation_gate_after != u8::MIN,
            "{}",
            vector.name
        );
        assert_eq!(
            state.loaded_scene_image_valid,
            vector.loaded_image_after != u16::MAX,
            "{}",
            vector.name
        );
        assert_eq!(
            state.dialogue_gate_active,
            vector.mouse_idle_after != u8::MIN,
            "{}",
            vector.name
        );
        assert_eq!(
            state.mouse_idle_low_byte_clear_pending,
            vector.mouse_idle_after == u8::MIN,
            "{}",
            vector.name
        );
        assert_eq!(
            (vector.script_start + basename.len() + 2) & 0xFFFF,
            vector.cursor,
            "{}",
            vector.name
        );
    }
}
