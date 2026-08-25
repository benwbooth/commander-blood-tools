//! Typed topic and sequence requests from BloodScript A7 and A8 handlers.

use commander_blood_formats::instruction::{ScriptSequenceRequest, ScriptTopicOffer};
use commander_blood_formats::script::ScriptWordId;

use super::PresentationRequestFlags;

const FINALE_SEQUENCE_PREFIX: &[u8] = b"fin.";

/// Resource-table line selected by a sequence request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationResourceLine {
    /// The `sq/` HNM sequence descriptor used by A8.
    Sequence,
}

/// Flat state shared by topic collection and sequence presentation requests.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SequencePresentationState {
    /// Whether the surrounding dialogue presentation currently accepts topics.
    pub presentation_active: bool,
    /// One pending concept appended to the next dialogue choice list.
    pub offered_topic: Option<ScriptWordId>,
    /// Basename appended to the game's `sq/` sequence directory.
    pub sequence_basename: Box<[u8]>,
    /// Whether the selected sequence begins the game-ending flow.
    pub finale_requested: bool,
    /// Resource-table line staged for the presentation dispatcher.
    pub active_resource_line: Option<PresentationResourceLine>,
    /// Existing presentation work currently owns the sequence path.
    pub presentation_gate_active: bool,
    /// Whether the cached scene image still matches the next request.
    pub loaded_scene_image_valid: bool,
    /// Dialogue dispatch is waiting on its current gate.
    pub dialogue_gate_active: bool,
}

/// Activity gates consulted after A8 has loaded its owned basename.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SequenceRequestContext {
    /// The ship interface is active.
    pub ship_active: bool,
    /// A scene transition currently permits a presentation request.
    pub scene_gate_active: bool,
}

/// Apply `vm_op_a7_set_if_presentation` to one interned optional topic.
pub fn offer_topic_if_presentation_active(
    offer: ScriptTopicOffer,
    state: &mut SequencePresentationState,
) -> bool {
    if state.presentation_active {
        state.offered_topic = offer.topic;
        true
    } else {
        false
    }
}

/// Apply `vm_op_a8_load_string` without a mutable fixed-address string buffer.
pub fn load_sequence_request(
    request: &ScriptSequenceRequest,
    context: SequenceRequestContext,
    request_flags: &mut PresentationRequestFlags,
    state: &mut SequencePresentationState,
) -> bool {
    state.sequence_basename = Box::from(request.basename());
    if request.basename().starts_with(FINALE_SEQUENCE_PREFIX) {
        state.finale_requested = true;
    }

    if request_flags.sequence_request_pending()
        || !(context.ship_active || context.scene_gate_active)
    {
        return false;
    }

    state.active_resource_line = Some(PresentationResourceLine::Sequence);
    request_flags.request_sequence();
    state.presentation_gate_active = false;
    state.loaded_scene_image_valid = false;
    state.dialogue_gate_active = false;
    true
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::instruction::{ScriptSequenceRequest, ScriptTopicOffer};
    use commander_blood_formats::script::decode_script_dictionary;
    use serde::Deserialize;

    use super::*;

    const PRESENTATION_ACTIVE_BIT: u8 = 1;
    const SEQUENCE_REQUEST_BIT: u8 = 2;
    const DEFAULT_PRESENTATION_FLAGS: u8 = 0xA0;
    const SHIP_ACTIVE_PRESENTATION_FLAGS: u8 = 0xA1;
    const PENDING_SEQUENCE_PRESENTATION_FLAGS: u8 = 0xA2;
    const SCENE_ACTIVITY_PRESENTATION_FLAGS: u8 = 0x10;

    #[derive(Deserialize)]
    struct TopicOracle {
        operand: u16,
        presentation_active: u8,
        store_performed: bool,
        register_before: u16,
        register_after: u16,
    }

    #[derive(Deserialize)]
    struct SequenceOracle {
        name: String,
        text_hex: String,
        finale_set: bool,
        request_raised: bool,
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
    fn topic_offers_match_every_original_a7_vector() {
        let dictionary_data = vec![u8::MIN; usize::from(u16::MAX) + 1];
        let dictionary = decode_script_dictionary(&dictionary_data).unwrap();
        let vectors: Vec<TopicOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_67ba_natural.json"
        ))
        .unwrap();

        for vector in vectors {
            let resolve = |offset| {
                (offset != u16::MIN).then(|| dictionary.resolve_source_offset(offset).unwrap())
            };
            let mut state = SequencePresentationState {
                presentation_active: vector.presentation_active & PRESENTATION_ACTIVE_BIT
                    != u8::MIN,
                offered_topic: resolve(vector.register_before),
                ..SequencePresentationState::default()
            };
            let stored = offer_topic_if_presentation_active(
                ScriptTopicOffer {
                    topic: resolve(vector.operand),
                },
                &mut state,
            );

            assert_eq!(stored, vector.store_performed);
            assert_eq!(state.offered_topic, resolve(vector.register_after));
        }
    }

    #[test]
    fn sequence_requests_match_every_original_a8_vector() {
        let vectors: Vec<SequenceOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_67c8_natural.json"
        ))
        .unwrap();

        for vector in vectors {
            let basename = bytes_from_hex(&vector.text_hex);
            let request = ScriptSequenceRequest::new(basename.clone()).unwrap();
            let (initial_flags, context) = match vector.name.as_str() {
                "request_blocked_by_pending_bit" => (
                    PENDING_SEQUENCE_PRESENTATION_FLAGS,
                    SequenceRequestContext {
                        ship_active: true,
                        scene_gate_active: true,
                    },
                ),
                "request_from_ship_flag" => (
                    SHIP_ACTIVE_PRESENTATION_FLAGS,
                    SequenceRequestContext {
                        ship_active: true,
                        scene_gate_active: false,
                    },
                ),
                "request_from_scene_gate" => (
                    SCENE_ACTIVITY_PRESENTATION_FLAGS,
                    SequenceRequestContext {
                        ship_active: false,
                        scene_gate_active: true,
                    },
                ),
                "unrelated_activity_bits_do_not_request" => (
                    SCENE_ACTIVITY_PRESENTATION_FLAGS,
                    SequenceRequestContext {
                        ship_active: false,
                        scene_gate_active: false,
                    },
                ),
                _ => (
                    DEFAULT_PRESENTATION_FLAGS,
                    SequenceRequestContext::default(),
                ),
            };
            let mut request_flags = PresentationRequestFlags::decode(initial_flags);
            let mut state = SequencePresentationState {
                presentation_gate_active: true,
                loaded_scene_image_valid: true,
                dialogue_gate_active: true,
                ..SequencePresentationState::default()
            };

            let raised = load_sequence_request(&request, context, &mut request_flags, &mut state);

            assert_eq!(state.sequence_basename, basename, "{}", vector.name);
            assert_eq!(state.finale_requested, vector.finale_set, "{}", vector.name);
            assert_eq!(raised, vector.request_raised, "{}", vector.name);
            assert_eq!(
                request_flags.bits(),
                initial_flags
                    | if raised {
                        SEQUENCE_REQUEST_BIT
                    } else {
                        u8::MIN
                    },
                "{}",
                vector.name
            );
            assert_eq!(
                state.active_resource_line,
                raised.then_some(PresentationResourceLine::Sequence),
                "{}",
                vector.name
            );
            assert_eq!(state.presentation_gate_active, !raised, "{}", vector.name);
            assert_eq!(state.loaded_scene_image_valid, !raised, "{}", vector.name);
            assert_eq!(state.dialogue_gate_active, !raised, "{}", vector.name);
        }
    }

    #[test]
    fn ordinary_sequences_do_not_clear_an_existing_finale_latch() {
        let request = ScriptSequenceRequest::new(b"ordinary.hnm".as_slice()).unwrap();
        let mut state = SequencePresentationState {
            finale_requested: true,
            ..SequencePresentationState::default()
        };
        load_sequence_request(
            &request,
            SequenceRequestContext::default(),
            &mut PresentationRequestFlags::default(),
            &mut state,
        );
        assert!(state.finale_requested);
    }
}
