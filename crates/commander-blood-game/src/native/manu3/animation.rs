//! Animation selection and fixed-point tweening for the MANU3 minigame.
//!
//! This module translates the four recovered routines at MANU3 file offsets
//! `0x017c`, `0x0181`, `0x019b`, and `0x01df`. The original routines addressed
//! scripts, targets, and records through 16-bit pointers. The modern port uses
//! ordinary owned collections and checked indices while preserving the game
//! state transitions and wrapping arithmetic.

use std::error::Error;
use std::fmt::{Display, Formatter};

/// Number of animation selectors recognized by the recovered native code.
pub const ANIMATION_SEQUENCE_COUNT: usize = 32;

const ANIMATION_SELECTOR_MASK: usize = ANIMATION_SEQUENCE_COUNT - 1;
const ACTIVE_PHASE_HIGH_BYTE_MASK: u16 = 0xff00;
const COMPLETED_PHASE: u16 = 0x0100;
const FIXED_POINT_FRACTIONAL_BITS: u32 = 16;
const FIXED_POINT_ONE: i32 = 1 << FIXED_POINT_FRACTIONAL_BITS;
const CURSOR_HORIZONTAL_CENTER: i16 = 160;
const CURSOR_YAW_SCALE: u16 = 2;
const PHASE_INCREMENT: u16 = 1;
const FRAME_DECREMENT: i16 = 1;

/// Cursor position used to steer the MANU3 camera.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CursorPosition {
    /// Horizontal position in the original 320-pixel coordinate system.
    pub x: i16,
    /// Vertical position in the original 200-pixel coordinate system.
    pub y: i16,
}

/// Camera angles captured when an animation sequence finishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CameraOrientation {
    /// Vertical camera angle.
    pub pitch: u16,
    /// Horizontal camera angle after applying cursor steering.
    pub yaw: u16,
}

/// One command in a phased MANU3 animation sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TweenSpecification {
    /// Number of frames over which the target moves to its end value.
    pub frame_count: u8,
    /// Sequence phase in which this command becomes active.
    pub phase: u8,
    /// Index of the signed value controlled by this command.
    pub target: usize,
    /// Value published at the end of the tween.
    pub end_value: i16,
}

impl TweenSpecification {
    /// Construct one animation command.
    pub const fn new(frame_count: u8, phase: u8, target: usize, end_value: i16) -> Self {
        Self {
            frame_count,
            phase,
            target,
            end_value,
        }
    }

    /// Construct the zero-count command that terminates an animation script.
    pub const fn end() -> Self {
        Self::new(u8::MIN, u8::MIN, usize::MIN, i16::MIN)
    }
}

/// Typed commands belonging to one selectable MANU3 animation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TweenScript {
    specifications: Vec<TweenSpecification>,
}

impl TweenScript {
    /// Build a script from its ordered commands.
    ///
    /// A missing terminal command is accepted and treated like an implicit
    /// zero-count command after the final element.
    pub fn new(specifications: Vec<TweenSpecification>) -> Self {
        Self { specifications }
    }
}

/// Complete selector table for the MANU3 animation scripts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnimationLibrary {
    sequences: [TweenScript; ANIMATION_SEQUENCE_COUNT],
}

impl AnimationLibrary {
    /// Build the fixed-size script table used by native selector values.
    pub const fn new(sequences: [TweenScript; ANIMATION_SEQUENCE_COUNT]) -> Self {
        Self { sequences }
    }

    fn sequence(&self, selector: u16) -> (usize, &TweenScript) {
        let index = usize::from(selector) & ANIMATION_SELECTOR_MASK;
        (index, &self.sequences[index])
    }
}

/// A currently active fixed-point interpolation record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TweenRecord {
    /// Number of updates remaining after the next value is published.
    pub frames_remaining: i16,
    /// Index of the target value controlled by this record.
    pub target: usize,
    /// Current signed 16.16 value, stored as raw wrapping bits.
    pub accumulator: u32,
    /// Signed 16.16 amount added after a live frame.
    pub step: i32,
}

/// Invalid typed state encountered while advancing a MANU3 animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationError {
    /// A script attempted to activate more tweens than the record pool holds.
    RecordCapacityExceeded {
        /// Number of records available to the animation system.
        capacity: usize,
    },
    /// A script or active record referred to a nonexistent target.
    TargetOutOfRange {
        /// Invalid target index.
        target: usize,
        /// Number of available targets.
        target_count: usize,
    },
}

impl Display for AnimationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RecordCapacityExceeded { capacity } => {
                write!(
                    formatter,
                    "MANU3 tween record capacity exceeded ({capacity})"
                )
            }
            Self::TargetOutOfRange {
                target,
                target_count,
            } => write!(
                formatter,
                "MANU3 tween target {target} is outside {target_count} values"
            ),
        }
    }
}

impl Error for AnimationError {}

/// Flat-memory state for MANU3 animation selection and tween playback.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manu3Animation {
    phase: u16,
    current_script: TweenScript,
    next_specification: usize,
    active_order: Vec<usize>,
    active_count: usize,
    records: Vec<TweenRecord>,
    targets: Vec<i16>,
    cursor: CursorPosition,
    view_pitch: u16,
    view_yaw: u16,
    finished_orientation: CameraOrientation,
}

impl Manu3Animation {
    /// Allocate animation state with a fixed tween-record pool and initial targets.
    pub fn new(record_capacity: usize, targets: Vec<i16>) -> Self {
        Self {
            phase: u16::MIN,
            current_script: TweenScript::default(),
            next_specification: usize::MIN,
            active_order: (usize::MIN..record_capacity).collect(),
            active_count: usize::MIN,
            records: vec![TweenRecord::default(); record_capacity],
            targets,
            cursor: CursorPosition::default(),
            view_pitch: u16::MIN,
            view_yaw: u16::MIN,
            finished_orientation: CameraOrientation {
                pitch: u16::MIN,
                yaw: u16::MIN,
            },
        }
    }

    /// Current 16-bit sequence phase.
    pub const fn phase(&self) -> u16 {
        self.phase
    }

    /// Number of interpolation records active this frame.
    pub const fn active_tween_count(&self) -> usize {
        self.active_count
    }

    /// Current values of all script-addressable animation fields.
    pub fn targets(&self) -> &[i16] {
        &self.targets
    }

    /// Update the cursor and camera values used by the completion path.
    pub const fn set_camera_input(
        &mut self,
        cursor: CursorPosition,
        view_pitch: u16,
        view_yaw: u16,
    ) {
        self.cursor = cursor;
        self.view_pitch = view_pitch;
        self.view_yaw = view_yaw;
    }

    /// Return the final camera orientation after the script has completed.
    pub fn completed_orientation(&self) -> Option<CameraOrientation> {
        (self.phase == COMPLETED_PHASE).then_some(self.finished_orientation)
    }

    /// Far-entry replacement for recovered routine `xdb_manu3_anim_select_entry`.
    ///
    /// The original function only adapted its calling convention. In flat
    /// memory it remains a documented forwarding boundary for traceability.
    pub fn select_animation_entry(
        &mut self,
        selector: u16,
        library: &AnimationLibrary,
    ) -> Result<usize, AnimationError> {
        self.select_animation(selector, library)
    }

    /// Select and start one of the 32 scripts handled by recovered routine
    /// `xdb_manu3_anim_select` at MANU3 file offset `0x0181`.
    pub fn select_animation(
        &mut self,
        selector: u16,
        library: &AnimationLibrary,
    ) -> Result<usize, AnimationError> {
        let (selected_index, script) = library.sequence(selector);
        self.phase = u16::MIN;
        self.current_script = script.clone();
        self.next_specification = usize::MIN;
        // xdb_manu3_anim_select passes the beginning of the active-slot table
        // to the constructor. A new selector replaces the prior active set;
        // retaining its end cursor would leak records across animations.
        self.active_count = usize::MIN;
        self.construct_tweens()?;
        Ok(selected_index)
    }

    /// Publish and advance all active records as recovered routine
    /// `xdb_manu3_tween_step` does at MANU3 file offset `0x019b`.
    pub fn step_tweens(&mut self) -> Result<(), AnimationError> {
        if self.phase & ACTIVE_PHASE_HIGH_BYTE_MASK != u16::MIN {
            return Ok(());
        }

        let mut cursor = usize::MIN;
        while cursor != self.active_count {
            let record_index = self.active_order[cursor];
            let record = self.records[record_index];
            let target_count = self.targets.len();
            let target =
                self.targets
                    .get_mut(record.target)
                    .ok_or(AnimationError::TargetOutOfRange {
                        target: record.target,
                        target_count,
                    })?;
            *target = (record.accumulator >> FIXED_POINT_FRACTIONAL_BITS) as u16 as i16;

            let updated_record = &mut self.records[record_index];
            updated_record.frames_remaining = updated_record
                .frames_remaining
                .wrapping_sub(FRAME_DECREMENT);
            if updated_record.frames_remaining.is_negative() {
                self.active_count -= 1;
                self.active_order.swap(cursor, self.active_count);
            } else {
                updated_record.accumulator = updated_record
                    .accumulator
                    .wrapping_add(updated_record.step as u32);
                cursor += 1;
            }
        }

        self.construct_tweens()
    }

    /// Preview the next authored target values without changing this animation.
    pub(crate) fn preview_next_targets(&self) -> Result<Vec<i16>, AnimationError> {
        let mut predicted = self.clone();
        predicted.step_tweens()?;
        Ok(predicted.targets)
    }

    /// Activate every script command assigned to the current phase as recovered
    /// routine `xdb_manu3_tween_constructor` does at file offset `0x01df`.
    pub fn construct_tweens(&mut self) -> Result<(), AnimationError> {
        let mut active_cursor = self.active_count;
        let next_frame_count = loop {
            let Some(specification) = self
                .current_script
                .specifications
                .get(self.next_specification)
                .copied()
            else {
                break u8::MIN;
            };

            if specification.frame_count == u8::MIN || specification.phase != self.phase as u8 {
                break specification.frame_count;
            }
            if active_cursor == self.records.len() {
                return Err(AnimationError::RecordCapacityExceeded {
                    capacity: self.records.len(),
                });
            }

            let target_count = self.targets.len();
            let current = *self.targets.get(specification.target).ok_or(
                AnimationError::TargetOutOfRange {
                    target: specification.target,
                    target_count,
                },
            )?;
            let delta = specification.end_value.wrapping_sub(current);
            let step = (i32::from(delta) * FIXED_POINT_ONE) / i32::from(specification.frame_count);
            let accumulator = (i32::from(current) * FIXED_POINT_ONE).wrapping_add(step) as u32;
            let record_index = self.active_order[active_cursor];
            self.records[record_index] = TweenRecord {
                frames_remaining: i16::from(specification.frame_count) - FRAME_DECREMENT,
                target: specification.target,
                accumulator,
                step,
            };

            active_cursor += 1;
            self.next_specification += 1;
        };

        self.active_count = active_cursor;
        if next_frame_count != u8::MIN {
            self.phase = self.phase.wrapping_add(PHASE_INCREMENT);
        } else if self.active_count == usize::MIN {
            let cursor_delta = (self.cursor.x as u16)
                .wrapping_sub(CURSOR_HORIZONTAL_CENTER as u16)
                .wrapping_mul(CURSOR_YAW_SCALE);
            self.finished_orientation = CameraOrientation {
                pitch: self.view_pitch,
                yaw: self.view_yaw.wrapping_sub(cursor_delta),
            };
            self.phase = COMPLETED_PHASE;
        } else {
            self.phase = self.phase.wrapping_add(PHASE_INCREMENT);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ANIMATION_ENTRY_VECTOR_COUNT: usize = 4;
    const ANIMATION_SELECT_VECTOR_COUNT: usize = 4;
    const TWEEN_STEP_VECTOR_COUNT: usize = 8;
    const TWEEN_CONSTRUCTOR_VECTOR_COUNT: usize = 8;
    const TEST_RECORD_CAPACITY: usize = 8;
    const TEST_CURSOR_X: i16 = 183;
    const TEST_CURSOR_Y: i16 = 91;
    const TEST_VIEW_PITCH: u16 = 4_660;
    const TEST_VIEW_YAW: u16 = 9_320;
    const MISMATCH_FRAME_COUNT: u8 = 1;
    const MISMATCH_PHASE_INCREMENT: u8 = 1;

    #[derive(Deserialize)]
    struct AnimationEntryVector {
        selector: u16,
    }

    #[derive(Deserialize)]
    struct AnimationSelectVector {
        selector: u16,
        masked_selector: usize,
    }

    #[derive(Deserialize)]
    struct TweenStepVector {
        name: String,
        phase: u16,
        active_slots_after: Vec<u16>,
        record_steps: Vec<RecordStep>,
    }

    #[derive(Deserialize)]
    struct RecordStep {
        record_offset: u16,
        published_value: i16,
        counter_before: u16,
        counter_after: u16,
        accumulator_before: u32,
        accumulator_after: u32,
        expired: bool,
    }

    #[derive(Deserialize)]
    struct TweenConstructorVector {
        name: String,
        phase_before: u16,
        phase_after: u16,
        processed_records: Vec<ConstructedRecord>,
        final_path: String,
    }

    #[derive(Deserialize)]
    struct ConstructedRecord {
        counter: u16,
        step: i32,
        accumulator: i64,
        remainder: i32,
    }

    fn selector_test_library() -> AnimationLibrary {
        let sequences = std::array::from_fn(|_| {
            TweenScript::new(vec![TweenSpecification::new(
                MISMATCH_FRAME_COUNT,
                MISMATCH_PHASE_INCREMENT,
                usize::MIN,
                i16::MIN,
            )])
        });
        AnimationLibrary::new(sequences)
    }

    #[test]
    fn entry_forwards_every_original_binary_selector_case() {
        let vectors: Vec<AnimationEntryVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/xdb_manu3_func_017c_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ANIMATION_ENTRY_VECTOR_COUNT);

        let library = selector_test_library();
        for vector in vectors {
            let mut animation = Manu3Animation::new(TEST_RECORD_CAPACITY, vec![i16::MIN]);
            let selected = animation
                .select_animation_entry(vector.selector, &library)
                .unwrap();
            assert_eq!(
                selected,
                usize::from(vector.selector) & ANIMATION_SELECTOR_MASK
            );
        }
    }

    #[test]
    fn selection_matches_every_original_binary_masking_case() {
        let vectors: Vec<AnimationSelectVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/xdb_manu3_func_0181_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ANIMATION_SELECT_VECTOR_COUNT);

        let library = selector_test_library();
        for vector in vectors {
            let mut animation = Manu3Animation::new(TEST_RECORD_CAPACITY, vec![i16::MIN]);
            let selected = animation
                .select_animation(vector.selector, &library)
                .unwrap();
            assert_eq!(selected, vector.masked_selector);
        }
    }

    #[test]
    fn selecting_an_animation_replaces_the_prior_active_tween_set() {
        let sequences = std::array::from_fn(|index| {
            let specifications = if index == 0 {
                vec![
                    TweenSpecification::new(10, 0, 0, 100),
                    TweenSpecification::new(10, 0, 1, 200),
                    TweenSpecification::end(),
                ]
            } else {
                vec![
                    TweenSpecification::new(10, 0, 0, -100),
                    TweenSpecification::end(),
                ]
            };
            TweenScript::new(specifications)
        });
        let library = AnimationLibrary::new(sequences);
        let mut animation = Manu3Animation::new(2, vec![0, 0]);

        animation.select_animation(0, &library).unwrap();
        assert_eq!(animation.active_tween_count(), 2);
        for _ in 0..200 {
            animation.select_animation(1, &library).unwrap();
            assert_eq!(animation.active_tween_count(), 1);
        }
    }

    #[test]
    fn tween_step_matches_all_original_binary_state_transitions() {
        let vectors: Vec<TweenStepVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/xdb_manu3_func_019b_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), TWEEN_STEP_VECTOR_COUNT);

        for vector in vectors {
            let record_ids: Vec<u16> = if vector.record_steps.is_empty() {
                vector.active_slots_after.clone()
            } else {
                vector
                    .record_steps
                    .iter()
                    .map(|step| step.record_offset)
                    .collect()
            };
            let mut animation =
                Manu3Animation::new(record_ids.len(), vec![i16::MIN; record_ids.len()]);
            animation.phase = vector.phase;
            animation.active_count = record_ids.len();
            animation.current_script = TweenScript::new(vec![TweenSpecification::end()]);

            for (record_index, step) in vector.record_steps.iter().enumerate() {
                let derived_step = if step.expired {
                    i32::MIN
                } else {
                    step.accumulator_after.wrapping_sub(step.accumulator_before) as i32
                };
                animation.records[record_index] = TweenRecord {
                    frames_remaining: step.counter_before as i16,
                    target: record_index,
                    accumulator: step.accumulator_before,
                    step: derived_step,
                };
            }

            animation.step_tweens().unwrap();

            let reordered_ids: Vec<u16> = animation
                .active_order
                .iter()
                .map(|index| record_ids[*index])
                .collect();
            assert_eq!(reordered_ids, vector.active_slots_after, "{}", vector.name);
            for (record_index, expected) in vector.record_steps.iter().enumerate() {
                let record = animation.records[record_index];
                assert_eq!(
                    animation.targets[record_index], expected.published_value,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    record.frames_remaining as u16, expected.counter_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    record.accumulator, expected.accumulator_after,
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn tween_constructor_matches_all_original_binary_arithmetic_cases() {
        let vectors: Vec<TweenConstructorVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/xdb_manu3_func_01df_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), TWEEN_CONSTRUCTOR_VECTOR_COUNT);

        for vector in vectors {
            let has_preexisting_record = vector.name == "empty_after_active_phase_wrap";
            let record_capacity =
                vector.processed_records.len() + usize::from(has_preexisting_record);
            let mut targets = Vec::with_capacity(vector.processed_records.len().max(1));
            let mut specifications = Vec::with_capacity(vector.processed_records.len() + 1);

            for expected in &vector.processed_records {
                let frame_count = expected.counter.wrapping_add(1) as u8;
                let initial_accumulator =
                    (expected.accumulator as u32).wrapping_sub(expected.step as u32);
                let current = (initial_accumulator >> FIXED_POINT_FRACTIONAL_BITS) as u16 as i16;
                let numerator = i64::from(expected.step) * i64::from(frame_count)
                    + i64::from(expected.remainder);
                let delta = (numerator / i64::from(FIXED_POINT_ONE)) as i16;
                let target = targets.len();
                targets.push(current);
                specifications.push(TweenSpecification::new(
                    frame_count,
                    vector.phase_before as u8,
                    target,
                    current.wrapping_add(delta),
                ));
            }

            if vector.final_path == "phase_advance" {
                specifications.push(TweenSpecification::new(
                    MISMATCH_FRAME_COUNT,
                    (vector.phase_before as u8).wrapping_add(MISMATCH_PHASE_INCREMENT),
                    usize::MIN,
                    i16::MIN,
                ));
                if targets.is_empty() {
                    targets.push(i16::MIN);
                }
            } else {
                specifications.push(TweenSpecification::end());
            }

            let mut animation = Manu3Animation::new(record_capacity, targets);
            animation.phase = vector.phase_before;
            animation.active_count = usize::from(has_preexisting_record);
            animation.current_script = TweenScript::new(specifications);
            animation.set_camera_input(
                CursorPosition {
                    x: TEST_CURSOR_X,
                    y: TEST_CURSOR_Y,
                },
                TEST_VIEW_PITCH,
                TEST_VIEW_YAW,
            );
            animation.construct_tweens().unwrap();

            assert_eq!(animation.phase(), vector.phase_after, "{}", vector.name);
            for (record_index, expected) in vector.processed_records.iter().enumerate() {
                let pool_index = animation.active_order[record_index];
                let record = animation.records[pool_index];
                assert_eq!(
                    record.frames_remaining as u16, expected.counter,
                    "{}",
                    vector.name
                );
                assert_eq!(record.step, expected.step, "{}", vector.name);
                assert_eq!(
                    record.accumulator, expected.accumulator as u32,
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn completion_uses_cursor_adjusted_camera_orientation() {
        let mut animation = Manu3Animation::new(usize::MIN, Vec::new());
        animation.current_script = TweenScript::new(vec![TweenSpecification::end()]);
        animation.set_camera_input(
            CursorPosition {
                x: TEST_CURSOR_X,
                y: TEST_CURSOR_Y,
            },
            TEST_VIEW_PITCH,
            TEST_VIEW_YAW,
        );

        animation.construct_tweens().unwrap();

        let expected_cursor_delta = (TEST_CURSOR_X as u16)
            .wrapping_sub(CURSOR_HORIZONTAL_CENTER as u16)
            .wrapping_mul(CURSOR_YAW_SCALE);
        assert_eq!(
            animation.completed_orientation(),
            Some(CameraOrientation {
                pitch: TEST_VIEW_PITCH,
                yaw: TEST_VIEW_YAW.wrapping_sub(expected_cursor_delta),
            })
        );
    }

    #[test]
    fn previewing_next_targets_does_not_advance_authoritative_animation() {
        const TARGET: i16 = 10;
        const FRAME_COUNT: u8 = 2;
        let sequences = std::array::from_fn(|_| {
            TweenScript::new(vec![
                TweenSpecification::new(FRAME_COUNT, u8::MIN, usize::MIN, TARGET),
                TweenSpecification::end(),
            ])
        });
        let library = AnimationLibrary::new(sequences);
        let mut animation = Manu3Animation::new(1, vec![0]);
        animation.select_animation(0, &library).unwrap();
        let before = animation.clone();

        assert_eq!(animation.preview_next_targets().unwrap(), vec![5]);
        assert_eq!(animation, before);
    }
}
