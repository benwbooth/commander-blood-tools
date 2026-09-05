//! Sequel-only panel controls recovered from CC, D7 and their native consumers.
//!
//! D7 owns GS:0x6B73, not the inherited A8 `fin.*` flag at GS:0x6B93.

use commander_blood_formats::code::ScriptDialect;
use commander_blood_formats::instruction::{ScriptSequenceSlot, ScriptSequenceSlotAssignment};

use super::ScriptSequenceSlots;

/// Session-owned sequel controls, retained across script-profile changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SequelPresentationControl {
    /// D7's one-way ending latch; it does not itself start a video or quit.
    pub ending_active: bool,
    /// Last CC assignment requests its slot, including in query mode.
    pub requested_choice: Option<ScriptSequenceSlot>,
}

/// Destination after the panel's complete scene list, not after one video frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SequelPanelCompletion {
    /// An ordinary, non-ending scene list closes its panel.
    Close,
    /// Startup reverse mode returns to transition, even with the ending latch.
    Transition,
    /// Publish the main loop's next-frame shutdown request.
    Shutdown,
}

/// Control of the ready panel actor before it advances its presentation line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SequelPanelActorAction {
    /// A script selection bypasses both the ending gate and animation selection.
    ScriptSelection,
    /// Select the panel hand animation and continue the ordinary actor path.
    Ordinary,
    /// Select the panel hand animation but return before advancing the line.
    EndingBlocked,
}

impl SequelPresentationControl {
    /// D7 writes one unconditionally, including in query mode (file 0x6E67).
    pub fn begin_ending(&mut self) {
        self.ending_active = true;
    }

    /// Queued scene decision at 0x8C14: explicit selection precedes the ending
    /// cancellation gate. The caller consumes that selection only on this path.
    pub const fn accepts_queued_input(self, primary_pressed: bool) -> bool {
        self.requested_choice.is_some() || (!self.ending_active && primary_pressed)
    }

    /// Scene-list completion at 0x8C75-0x8C8A.
    pub const fn completion(self, reverse: bool) -> SequelPanelCompletion {
        if reverse {
            SequelPanelCompletion::Transition
        } else if self.ending_active {
            SequelPanelCompletion::Shutdown
        } else {
            SequelPanelCompletion::Close
        }
    }

    /// Ready actor decision at 0x92E4-0x92F8, after marking its line present.
    pub const fn actor_action(self) -> SequelPanelActorAction {
        if self.requested_choice.is_some() {
            SequelPanelActorAction::ScriptSelection
        } else if self.ending_active {
            SequelPanelActorAction::EndingBlocked
        } else {
            SequelPanelActorAction::Ordinary
        }
    }
}

/// Apply the shared CC slot copy and the sequel's additional request write.
/// Native 0x69ED writes the zero-based choice before copying its bounded name.
pub fn assign_presentation_sequence(
    assignment: ScriptSequenceSlotAssignment,
    dialect: ScriptDialect,
    slots: &mut ScriptSequenceSlots,
    sequel: &mut SequelPresentationControl,
) {
    if dialect == ScriptDialect::BigBugBang {
        sequel.requested_choice = Some(assignment.slot());
    }
    slots.assign(assignment);
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use commander_blood_formats::code::{
        ScriptCodeOffset, ScriptTokenDecoder, decode_script_token,
    };
    use commander_blood_formats::instruction::decode_script_sequence_slot_assignment;
    use serde::Deserialize;

    use super::*;

    const ENDING: u16 = 0x6B73;
    const PENDING_CHOICE: u16 = 0x2A84;
    const CHOICE_TABLE: u16 = 0x7086;
    const PRIMARY: u16 = 0x0C36;
    const REVERSE: u16 = 0x2A80;
    const SHUTDOWN: u16 = 0x0D1D;

    #[derive(Deserialize)]
    struct Oracle {
        name: String,
        input: BTreeMap<u16, u8>,
        token: Vec<u8>,
        destination: u16,
        output: BTreeMap<u16, u8>,
        next_script_offset: usize,
    }

    fn choice(value: u8) -> Option<ScriptSequenceSlot> {
        if value == u8::MAX {
            None
        } else {
            ScriptSequenceSlot::decode(value + 1)
        }
    }

    #[test]
    fn sequel_presentation_controls_match_native_instruction_and_decision_vectors() {
        let mut count = 0;
        for line in
            include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_presentation.jsonl")
                .lines()
        {
            let vector: Oracle = serde_json::from_str(line).unwrap();
            let mut control = SequelPresentationControl {
                ending_active: vector.input.get(&ENDING).is_some_and(|v| v & 1 != 0),
                requested_choice: choice(*vector.input.get(&PENDING_CHOICE).unwrap_or(&u8::MAX)),
            };
            if vector.name.starts_with("d7_") {
                control.begin_ending();
                assert_eq!(
                    u8::from(control.ending_active),
                    vector.output[&ENDING],
                    "{}",
                    vector.name
                );
                assert_eq!(vector.next_script_offset, vector.token.len());
            } else if vector.name.starts_with("cc_") {
                let token = decode_script_token(
                    &vector.token,
                    ScriptCodeOffset::new(0),
                    &mut ScriptTokenDecoder::new(ScriptDialect::BigBugBang),
                )
                .unwrap();
                let assignment = decode_script_sequence_slot_assignment(&token).unwrap();
                let mut before = [0xA4; 96];
                for field in before.chunks_exact_mut(16) {
                    field[0] = 0;
                }
                let mut slots = ScriptSequenceSlots::default();
                slots.restore_save_block(&before).unwrap();
                assign_presentation_sequence(
                    assignment.clone(),
                    ScriptDialect::BigBugBang,
                    &mut slots,
                    &mut control,
                );
                assert_eq!(
                    control.requested_choice,
                    choice(vector.output[&PENDING_CHOICE]),
                    "{}",
                    vector.name
                );
                for (index, value) in slots.encode_save_block().into_iter().enumerate() {
                    assert_eq!(
                        value,
                        vector.output[&(CHOICE_TABLE + index as u16)],
                        "{} at {index}",
                        vector.name
                    );
                }
                assert_eq!(vector.next_script_offset, vector.token.len());
                if assignment.name().as_bytes().is_empty() {
                    assert!(slots.name(assignment.slot()).is_none());
                    assert!(slots.ordered_names()[assignment.slot().index()].is_none());
                }
                control.requested_choice = None;
                assign_presentation_sequence(
                    assignment,
                    ScriptDialect::CommanderBlood,
                    &mut slots,
                    &mut control,
                );
                assert_eq!(
                    control.requested_choice, None,
                    "Commander CC must not select a channel"
                );
            } else if vector.name.starts_with("queued_") {
                assert_eq!(
                    control.accepts_queued_input(vector.input[&PRIMARY] != 0),
                    vector.destination == 0x8C9D,
                    "{}",
                    vector.name
                );
            } else if vector.name.starts_with("completed_") {
                let expected = if vector.output[&SHUTDOWN] != 0 {
                    SequelPanelCompletion::Shutdown
                } else if vector.destination == 0x8CA4 {
                    SequelPanelCompletion::Close
                } else {
                    SequelPanelCompletion::Transition
                };
                assert_eq!(
                    control.completion(vector.input[&REVERSE] != 0),
                    expected,
                    "{}",
                    vector.name
                );
            } else if vector.name.starts_with("actor_") {
                let expected = if vector.output[&0x0C2A] == 23 {
                    SequelPanelActorAction::ScriptSelection
                } else if vector.destination == 0x935C {
                    SequelPanelActorAction::EndingBlocked
                } else {
                    SequelPanelActorAction::Ordinary
                };
                assert_eq!(control.actor_action(), expected, "{}", vector.name);
            } else {
                panic!("unhandled oracle case {}", vector.name);
            }
            count += 1;
        }
        assert_eq!(count, 66);
    }
}
