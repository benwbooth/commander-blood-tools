//! Ship-view target-list opening, fallback, and selection flow.

const PHASE_LAYOUT_PENDING: u8 = 1;
const PHASE_TRANSITION_ACTIVE: u8 = 2;
const DEPTH_OPENING_ACTIVE: u8 = 1;
const TARGET_PANEL_OPEN_STEP: u8 = 6;

/// Which decoded target list supplies labels for the ship-view panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipTargetListSource {
    /// Presentable records built from the current Arche relation.
    PresentableRecords,
    /// Static fallback labels used when no presentable records exist.
    FallbackLabels,
}

/// Purpose of one target-list layout call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipTargetListPass {
    /// Measure the panel before its opening interpolation begins.
    MeasureOnly,
    /// Render the open panel and process pointer input.
    Interactive,
}

/// Bounded semantic result returned by the modern choice-list host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipTargetListSelection {
    /// No row was selected this frame.
    None,
    /// One ordinary row was selected by its zero-based list index.
    Item(usize),
    /// The synthetic final cancel row was selected.
    Cancel,
}

/// Mutable state owned by the ship target selector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipTargetSelectionState<RecordId> {
    /// Native phase byte; low bits retain the recovered wrapping transitions.
    pub phase: u8,
    /// Current rectangle-interpolation step.
    pub transition_step: u16,
    /// Final rectangle-interpolation step.
    pub transition_total_steps: u16,
    /// Whether the current update selected the fallback label source.
    pub fallback_active: bool,
    /// Current typed target returned for any ordinary fallback selection.
    pub current_target: RecordId,
    /// Native opening flags; bit zero owns the depth-door transition.
    pub depth_opening_flags: u8,
    /// Low-byte depth movement requested when cancel is selected.
    pub depth_step: u8,
}

/// Renderer and input work whose call order is visible in the native routine.
pub trait ShipTargetSelectionHost {
    /// Measure or interact with the selected list source.
    fn update_target_list(
        &mut self,
        source: ShipTargetListSource,
        pass: ShipTargetListPass,
        phase: u8,
        transition_step: u16,
    ) -> ShipTargetListSelection;

    /// Advance the opening rectangle while mutating the shared step counter.
    fn advance_target_transition(&mut self, current_step: &mut u16, total_steps: u16);
}

/// Terminal result of one target-selector update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShipTargetSelectionOutcome<RecordId> {
    /// Opening interpolation has not reached its final step.
    Transitioning,
    /// The panel remains open without a selected row.
    NoSelection,
    /// A presentable record or the current fallback target was selected.
    Selected(RecordId),
    /// Cancel requested the depth-door opening transition.
    CloseRequested,
}

/// Invalid typed list result returned by a host implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipTargetSelectionError {
    /// List source that reported the invalid index.
    pub source: ShipTargetListSource,
    /// Out-of-range index reported by the host.
    pub selected_index: usize,
    /// Number of ordinary rows available in that source.
    pub available_items: usize,
}

/// Update native BLOODPRG target selector `0x00B2BB` over flat typed lists.
///
/// Empty slices replace the original `0xFFFF`-first fallback test, and
/// [`ShipTargetListSelection::Cancel`] replaces the terminal sentinel row.
/// Record identities remain owned values rather than offsets into a segmented
/// record heap. The phase byte intentionally retains wrapping addition because
/// the original accepts combined and high-bit states as observable input.
pub fn select_ship_target<RecordId: Clone, Host: ShipTargetSelectionHost>(
    state: &mut ShipTargetSelectionState<RecordId>,
    presentable_targets: &[RecordId],
    fallback_label_count: usize,
    host: &mut Host,
) -> Result<ShipTargetSelectionOutcome<RecordId>, ShipTargetSelectionError> {
    let (source, available_items) = if presentable_targets.is_empty() {
        state.fallback_active = true;
        (ShipTargetListSource::FallbackLabels, fallback_label_count)
    } else {
        state.fallback_active = false;
        (
            ShipTargetListSource::PresentableRecords,
            presentable_targets.len(),
        )
    };

    if state.phase & PHASE_LAYOUT_PENDING != u8::MIN {
        let _ = host.update_target_list(
            source,
            ShipTargetListPass::MeasureOnly,
            state.phase,
            state.transition_step,
        );
        state.transition_step = u16::MIN;
        state.phase = state.phase.wrapping_add(1);
    }

    if state.phase & PHASE_TRANSITION_ACTIVE != u8::MIN {
        let transition_complete = state.transition_step == state.transition_total_steps;
        host.advance_target_transition(&mut state.transition_step, state.transition_total_steps);
        if !transition_complete {
            return Ok(ShipTargetSelectionOutcome::Transitioning);
        }
        state.phase = u8::MIN;
    }

    match host.update_target_list(
        source,
        ShipTargetListPass::Interactive,
        state.phase,
        state.transition_step,
    ) {
        ShipTargetListSelection::None => Ok(ShipTargetSelectionOutcome::NoSelection),
        ShipTargetListSelection::Cancel => {
            state.depth_opening_flags = DEPTH_OPENING_ACTIVE;
            state.depth_step = TARGET_PANEL_OPEN_STEP;
            Ok(ShipTargetSelectionOutcome::CloseRequested)
        }
        ShipTargetListSelection::Item(selected_index) => {
            if selected_index >= available_items {
                return Err(ShipTargetSelectionError {
                    source,
                    selected_index,
                    available_items,
                });
            }
            let target = match source {
                ShipTargetListSource::PresentableRecords => {
                    presentable_targets[selected_index].clone()
                }
                ShipTargetListSource::FallbackLabels => state.current_target.clone(),
            };
            Ok(ShipTargetSelectionOutcome::Selected(target))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use serde::Deserialize;

    use super::*;

    const NATIVE_NONE: u16 = u16::MAX;
    const NATIVE_MEASURE_ONLY: u8 = 1;
    const NATIVE_RECORD_NAME_BYTES: u16 = 4;
    const NATIVE_LIST_ENTRY_BYTES: u16 = 2;
    const INITIAL_OPENING_FLAGS: u8 = 211;
    const INITIAL_DEPTH_STEP: u8 = 229;
    const ARBITRARY_CURRENT_TARGET: u16 = 17_767;

    #[derive(Deserialize)]
    struct TargetVector {
        name: String,
        primary: Vec<u16>,
        fallback: Vec<u16>,
        phase_before: u8,
        phase_after: u8,
        tick_before: u16,
        tick_after: u16,
        used_fallback: bool,
        calls: Vec<CallVector>,
        selected_record: u16,
        opening_after: u8,
        depth_step_after: u8,
    }

    #[derive(Clone, Deserialize)]
    struct CallVector {
        call: String,
        string_segment: Option<u16>,
        items_segment: Option<u16>,
        query_mode: Option<u8>,
        phase: u8,
        tick: u16,
        total: Option<u16>,
        result: Option<u16>,
        complete: Option<bool>,
    }

    struct OracleHost {
        calls: VecDeque<CallVector>,
        ordinary_item_count: usize,
    }

    impl ShipTargetSelectionHost for OracleHost {
        fn update_target_list(
            &mut self,
            source: ShipTargetListSource,
            pass: ShipTargetListPass,
            phase: u8,
            transition_step: u16,
        ) -> ShipTargetListSelection {
            let call = self.calls.pop_front().expect("missing oracle list call");
            assert_eq!(call.call, "list_widget_layout_unified");
            assert_eq!(call.phase, phase);
            assert_eq!(call.tick, transition_step);
            assert_eq!(
                call.query_mode == Some(NATIVE_MEASURE_ONLY),
                pass == ShipTargetListPass::MeasureOnly
            );
            assert_eq!(
                call.string_segment == call.items_segment,
                source == ShipTargetListSource::FallbackLabels
            );

            native_selection(call.result.unwrap(), self.ordinary_item_count)
        }

        fn advance_target_transition(&mut self, current_step: &mut u16, total_steps: u16) {
            let call = self
                .calls
                .pop_front()
                .expect("missing oracle transition call");
            assert_eq!(call.call, "framebuffer_rect_interpolate_and_remap_step");
            assert_eq!(call.phase, PHASE_TRANSITION_ACTIVE);
            assert_eq!(call.tick, *current_step);
            assert_eq!(call.total, Some(total_steps));
            assert_eq!(call.complete, Some(*current_step == total_steps));
            if *current_step != total_steps {
                *current_step = current_step.wrapping_add(1);
            }
        }
    }

    #[test]
    fn selector_matches_every_original_target_vector() {
        let vectors: Vec<TargetVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_b2bb_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), 17);

        for vector in vectors {
            let presentable_targets = native_records(&vector.primary);
            let fallback_label_count = native_item_count(&vector.fallback);
            let initial_target = if vector.used_fallback
                && vector.selected_record != u16::MIN
                && vector.selected_record != NATIVE_NONE
            {
                vector.selected_record
            } else {
                ARBITRARY_CURRENT_TARGET
            };
            let transition_total_steps =
                vector.calls.iter().find_map(|call| call.total).unwrap_or(6);
            let mut state = ShipTargetSelectionState {
                phase: vector.phase_before,
                transition_step: vector.tick_before,
                transition_total_steps,
                fallback_active: false,
                current_target: initial_target,
                depth_opening_flags: INITIAL_OPENING_FLAGS,
                depth_step: INITIAL_DEPTH_STEP,
            };
            let ordinary_item_count = if presentable_targets.is_empty() {
                fallback_label_count
            } else {
                presentable_targets.len()
            };
            let mut host = OracleHost {
                calls: vector.calls.clone().into(),
                ordinary_item_count,
            };

            let outcome = select_ship_target(
                &mut state,
                &presentable_targets,
                fallback_label_count,
                &mut host,
            )
            .unwrap();

            assert!(host.calls.is_empty(), "{}", vector.name);
            assert_eq!(state.phase, vector.phase_after, "{}", vector.name);
            assert_eq!(state.transition_step, vector.tick_after, "{}", vector.name);
            assert_eq!(
                state.fallback_active, vector.used_fallback,
                "{}",
                vector.name
            );
            assert_eq!(
                state.depth_opening_flags, vector.opening_after,
                "{}",
                vector.name
            );
            assert_eq!(state.depth_step, vector.depth_step_after, "{}", vector.name);
            assert_eq!(
                outcome,
                expected_outcome(&vector, initial_target),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn out_of_range_host_selection_is_rejected_without_indexing() {
        struct InvalidHost;

        impl ShipTargetSelectionHost for InvalidHost {
            fn update_target_list(
                &mut self,
                _source: ShipTargetListSource,
                _pass: ShipTargetListPass,
                _phase: u8,
                _transition_step: u16,
            ) -> ShipTargetListSelection {
                ShipTargetListSelection::Item(3)
            }

            fn advance_target_transition(&mut self, _current_step: &mut u16, _total_steps: u16) {
                panic!("inactive transition must not advance");
            }
        }

        let mut state = ShipTargetSelectionState {
            phase: u8::MIN,
            transition_step: u16::MIN,
            transition_total_steps: 6,
            fallback_active: false,
            current_target: 9_u16,
            depth_opening_flags: u8::MIN,
            depth_step: u8::MIN,
        };
        let error = select_ship_target(&mut state, &[5_u16], 0, &mut InvalidHost).unwrap_err();
        assert_eq!(
            error,
            ShipTargetSelectionError {
                source: ShipTargetListSource::PresentableRecords,
                selected_index: 3,
                available_items: 1,
            }
        );
    }

    fn native_records(items: &[u16]) -> Vec<u16> {
        items
            .iter()
            .copied()
            .take_while(|item| *item != NATIVE_NONE)
            .map(|name_offset| name_offset.wrapping_sub(NATIVE_RECORD_NAME_BYTES))
            .collect()
    }

    fn native_item_count(items: &[u16]) -> usize {
        items
            .iter()
            .take_while(|item| **item != NATIVE_NONE)
            .count()
    }

    fn native_selection(result: u16, item_count: usize) -> ShipTargetListSelection {
        if result == NATIVE_NONE {
            return ShipTargetListSelection::None;
        }
        let byte_offset = result.wrapping_mul(NATIVE_LIST_ENTRY_BYTES);
        let index = usize::from(byte_offset / NATIVE_LIST_ENTRY_BYTES);
        if index == item_count {
            ShipTargetListSelection::Cancel
        } else {
            ShipTargetListSelection::Item(index)
        }
    }

    fn expected_outcome(
        vector: &TargetVector,
        initial_target: u16,
    ) -> ShipTargetSelectionOutcome<u16> {
        if vector.calls.last().is_some_and(|call| {
            call.call == "framebuffer_rect_interpolate_and_remap_step"
                && call.complete == Some(false)
        }) {
            return ShipTargetSelectionOutcome::Transitioning;
        }
        match vector.selected_record {
            u16::MIN => ShipTargetSelectionOutcome::NoSelection,
            NATIVE_NONE => ShipTargetSelectionOutcome::CloseRequested,
            selected if vector.used_fallback => {
                assert_eq!(selected, initial_target, "{}", vector.name);
                ShipTargetSelectionOutcome::Selected(initial_target)
            }
            selected => ShipTargetSelectionOutcome::Selected(selected),
        }
    }
}
