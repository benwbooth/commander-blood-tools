//! Concrete flat-memory renderer and input host for the ship target selector.

use anyhow::{Context, Result};
use commander_blood_formats::script::{
    ScriptDirectory, ScriptObjectId, ScriptObjectKind, ScriptState, ScriptStateObjectReference,
};

use crate::native::bloodprg::{
    ChoiceListConfig, ChoiceListFrame, ChoiceListHandRequest, ChoiceListPointer, ChoiceListRect,
    ChoiceListState, FramebufferTransitionState, RasterPoint, ScriptFieldSelector,
    ShipTargetListPass, ShipTargetListSelection, ShipTargetListSource, ShipTargetSelectionHost,
    ShipTargetSelectionOutcome, ShipTargetSelectionState, TransitionRect,
    advance_framebuffer_rect_transition, script_field_offset, select_ship_target,
};

use super::OriginalGameRuntime;
use super::choice_list::{RuntimeChoiceListBackend, draw_choice_list_rows};

const TARGET_LIST_CENTER_X: i16 = 80;
const TARGET_TRANSITION_CENTER_Y: i16 = 100;
const COLLAPSED_TARGET_EXTENT: i16 = 0;
const FALLBACK_TARGET_LABEL: &[u8] = b"GO";
const CANCEL_LABEL: &[u8] = b"CANCEL";

/// Resolve Arche's linked navigation object and its native direct-target gate.
///
/// This translates the record-link and kind-mask probe in `ship_3d_hud_init`.
/// The original mask's two bits are the decoded auxiliary and black-hole kinds;
/// stable object identities replace serialized VAR offsets.
pub(super) fn ship_hud_arche_link(
    state: &ScriptState,
    arche: ScriptObjectId,
) -> Result<(ScriptObjectId, bool)> {
    let arche_record = state
        .object(arche)
        .with_context(|| format!("Arche object {arche:?} is absent from profile state"))?;
    let field_offset =
        script_field_offset(arche_record.kind, ScriptFieldSelector::HOLDER_OR_LOCATION)
            .with_context(|| format!("Arche object {arche:?} has no navigation-link field"))?;
    let field = state
        .object_word(arche, field_offset / size_of::<u16>())
        .with_context(|| format!("Arche object {arche:?} has a truncated navigation-link field"))?;
    let ScriptStateObjectReference::Object(link) = state
        .object_reference(field)
        .with_context(|| format!("Arche object {arche:?} has an invalid navigation link"))?
    else {
        anyhow::bail!("Arche object {arche:?} uses the navigation-link sentinel");
    };
    let linked_record = state
        .object(link)
        .with_context(|| format!("Arche navigation link {link:?} is absent from profile state"))?;
    let directly_selectable = !matches!(
        linked_record.kind,
        ScriptObjectKind::Auxiliary | ScriptObjectKind::BlackHole
    );
    Ok((link, directly_selectable))
}

/// Result of one concrete target-list frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeShipTargetSelection {
    /// Semantic outcome returned by the recovered selector.
    pub outcome: ShipTargetSelectionOutcome<ScriptObjectId>,
    /// Whether the original list interaction requested its selection sound.
    pub selection_sound_requested: bool,
}

/// Persistent list geometry and interaction state for the ship HUD.
#[derive(Default)]
pub struct RuntimeShipTargetSelector {
    choice_list: ChoiceListState,
    current_rect: ChoiceListRect,
    last_frame: Option<ChoiceListFrame>,
    hand_requests: Vec<ChoiceListHandRequest>,
}

impl RuntimeShipTargetSelector {
    /// Borrow the most recently measured or rendered list rectangle.
    pub const fn current_rect(&self) -> ChoiceListRect {
        self.current_rect
    }

    /// Borrow the most recent list frame, when layout has run at least once.
    pub fn last_frame(&self) -> Option<&ChoiceListFrame> {
        self.last_frame.as_ref()
    }

    /// Drain ordered MANU3 selector writes emitted by the shared list widget.
    pub(super) fn take_hand_requests(&mut self) -> Vec<ChoiceListHandRequest> {
        std::mem::take(&mut self.hand_requests)
    }

    /// Advance `ship_3d_target_record_select` using real profile names and pixels.
    pub fn update(
        &mut self,
        runtime: &mut OriginalGameRuntime,
        pointer: ChoiceListPointer,
        current_hand_animation: u16,
        state: &mut ShipTargetSelectionState<ScriptObjectId>,
        presentable_targets: &[ScriptObjectId],
    ) -> Result<RuntimeShipTargetSelection> {
        let profile = runtime
            .current_profile()
            .context("ship target selection requires a loaded BloodScript profile")?;
        let labels = target_labels(profile.directory(), presentable_targets)?;
        let fonts = runtime.data().font_resources().clone();
        let mut backend = RuntimeShipTargetBackend {
            list: RuntimeChoiceListBackend::new(runtime, &fonts, pointer, current_hand_animation),
            labels: &labels,
            choice_list: &mut self.choice_list,
            current_rect: &mut self.current_rect,
            last_frame: &mut self.last_frame,
            selection_sound_requested: false,
        };
        let outcome = select_ship_target(
            state,
            presentable_targets,
            usize::from(!FALLBACK_TARGET_LABEL.is_empty()),
            &mut backend,
        )
        .map_err(|error| anyhow::anyhow!("invalid ship target selection: {error:?}"))?;
        backend.list.finish()?;
        self.hand_requests.extend(backend.list.take_hand_requests());
        Ok(RuntimeShipTargetSelection {
            outcome,
            selection_sound_requested: backend.selection_sound_requested,
        })
    }
}

fn target_labels(
    directory: &ScriptDirectory,
    targets: &[ScriptObjectId],
) -> Result<Vec<Box<[u8]>>> {
    targets
        .iter()
        .enumerate()
        .map(|(index, target)| {
            directory
                .object(*target)
                .map(|entry| Box::from(entry.name()))
                .with_context(|| {
                    format!("ship target {index} references missing profile object {target:?}")
                })
        })
        .collect()
}

struct RuntimeShipTargetBackend<'runtime, 'state> {
    list: RuntimeChoiceListBackend<'runtime>,
    labels: &'state [Box<[u8]>],
    choice_list: &'state mut ChoiceListState,
    current_rect: &'state mut ChoiceListRect,
    last_frame: &'state mut Option<ChoiceListFrame>,
    selection_sound_requested: bool,
}

impl RuntimeShipTargetBackend<'_, '_> {
    fn owned_labels(&self, source: ShipTargetListSource) -> Vec<Box<[u8]>> {
        match source {
            ShipTargetListSource::PresentableRecords => self.labels.to_vec(),
            ShipTargetListSource::FallbackLabels => vec![Box::from(FALLBACK_TARGET_LABEL)],
        }
    }

    fn record_error<T>(&mut self, result: Result<T>, fallback: T) -> T {
        match result {
            Ok(value) => value,
            Err(error) => {
                self.list.record_error(Err(error));
                fallback
            }
        }
    }
}

impl ShipTargetSelectionHost for RuntimeShipTargetBackend<'_, '_> {
    fn update_target_list(
        &mut self,
        source: ShipTargetListSource,
        pass: ShipTargetListPass,
        _phase: u8,
        _transition_step: u16,
    ) -> ShipTargetListSelection {
        let owned_labels = self.owned_labels(source);
        let labels = owned_labels.iter().map(Box::as_ref).collect::<Vec<_>>();
        let config = ChoiceListConfig {
            center_x: TARGET_LIST_CENTER_X,
            preserve_individual_widths: true,
            cancel_label: Some(CANCEL_LABEL),
            layout_only: pass == ShipTargetListPass::MeasureOnly,
        };
        let frame = crate::native::bloodprg::update_choice_list(
            &labels,
            config,
            self.choice_list,
            &mut self.list,
        );
        *self.current_rect = frame.rect;
        if pass == ShipTargetListPass::Interactive {
            let draw =
                draw_choice_list_rows(self.list.runtime_mut(), &labels, Some(CANCEL_LABEL), &frame);
            self.list.record_error(draw);
        }

        let selection = if frame.cancelled {
            ShipTargetListSelection::Cancel
        } else if let Some(index) = frame.selected_item {
            ShipTargetListSelection::Item(index)
        } else {
            ShipTargetListSelection::None
        };
        self.selection_sound_requested |= selection != ShipTargetListSelection::None;
        *self.last_frame = Some(frame);
        selection
    }

    fn advance_target_transition(&mut self, current_step: &mut u16, total_steps: u16) {
        let Ok(current) = u8::try_from(*current_step) else {
            self.list.record_error(Err(anyhow::anyhow!(
                "ship target transition step {current_step} exceeds byte-sized native state"
            )));
            return;
        };
        let Ok(total) = u8::try_from(total_steps) else {
            self.list.record_error(Err(anyhow::anyhow!(
                "ship target transition duration {total_steps} exceeds byte-sized native state"
            )));
            return;
        };
        let mut transition = FramebufferTransitionState {
            total_steps: total,
            current_step: current,
        };
        let source = transition_rect(*self.current_rect);
        let target = TransitionRect::new(
            TARGET_LIST_CENTER_X,
            TARGET_TRANSITION_CENTER_Y,
            COLLAPSED_TARGET_EXTENT,
            COLLAPSED_TARGET_EXTENT,
        );
        let region = self.record_error(
            advance_framebuffer_rect_transition(&mut transition, source, target)
                .context("advancing the ship target-list transition"),
            None,
        );
        *current_step = u16::from(transition.current_step);
        if let Some(region) = region {
            self.list.darken_region(
                RasterPoint {
                    x: i32::from(region.x),
                    y: i32::from(region.y),
                },
                region.width,
                region.height,
            );
        }
    }
}

fn transition_rect(rect: ChoiceListRect) -> TransitionRect {
    TransitionRect::new(
        rect.origin[0],
        rect.origin[1],
        rect.size[0] as i16,
        rect.size[1] as i16,
    )
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::native::bloodprg::{ChoiceListRowKind, ScriptProfileId, ShipTargetSelectionOutcome};
    use crate::runtime::{OriginalGameData, OriginalGameDataPaths};

    const TEST_BACKGROUND_INDEX: u8 = 225;
    const TARGET_TRANSITION_STEPS: u16 = 10;

    #[test]
    fn original_profile_target_list_opens_selects_and_cancels() {
        let Some(data) = original_game_data() else {
            return;
        };
        let mut runtime = OriginalGameRuntime::new(data);
        runtime
            .load_profile(ScriptProfileId::INITIAL)
            .expect("loading the shipped initial profile");
        let arche = runtime
            .current_profile()
            .and_then(|profile| profile.builtins().archetype)
            .expect("initial profile binds Arche");
        runtime.front_buffer_mut().clear(TEST_BACKGROUND_INDEX);

        let mut selector = RuntimeShipTargetSelector::default();
        let mut state = ShipTargetSelectionState {
            phase: 1,
            transition_step: u16::MIN,
            transition_total_steps: TARGET_TRANSITION_STEPS,
            fallback_active: false,
            current_target: arche,
            depth_opening_flags: u8::MIN,
            depth_step: u8::MIN,
        };
        let opening = selector
            .update(
                &mut runtime,
                ChoiceListPointer::default(),
                u16::MIN,
                &mut state,
                &[arche],
            )
            .unwrap();
        assert_eq!(opening.outcome, ShipTargetSelectionOutcome::Transitioning);
        assert_eq!(state.phase, 2);
        assert_eq!(state.transition_step, 1);
        assert!(!opening.selection_sound_requested);

        state.phase = u8::MIN;
        let interactive = selector
            .update(
                &mut runtime,
                ChoiceListPointer::default(),
                u16::MIN,
                &mut state,
                &[arche],
            )
            .unwrap();
        assert_eq!(interactive.outcome, ShipTargetSelectionOutcome::NoSelection);
        let item_position = selector
            .last_frame()
            .unwrap()
            .rows
            .iter()
            .find_map(|row| matches!(row.kind, ChoiceListRowKind::Item(0)).then_some(row.position))
            .unwrap();
        let selected = selector
            .update(
                &mut runtime,
                pressed_pointer(item_position),
                u16::MIN,
                &mut state,
                &[arche],
            )
            .unwrap();
        assert_eq!(
            selected.outcome,
            ShipTargetSelectionOutcome::Selected(arche)
        );
        assert!(selected.selection_sound_requested);

        let cancel_position = selector
            .last_frame()
            .unwrap()
            .rows
            .iter()
            .find_map(|row| matches!(row.kind, ChoiceListRowKind::Cancel).then_some(row.position))
            .unwrap();
        let cancelled = selector
            .update(
                &mut runtime,
                pressed_pointer(cancel_position),
                u16::MIN,
                &mut state,
                &[arche],
            )
            .unwrap();
        assert_eq!(
            cancelled.outcome,
            ShipTargetSelectionOutcome::CloseRequested
        );
        assert_eq!(state.depth_opening_flags, 1);
        assert_eq!(state.depth_step, 6);
    }

    #[test]
    fn empty_presentable_list_uses_the_executable_go_fallback() {
        let Some(data) = original_game_data() else {
            return;
        };
        let mut runtime = OriginalGameRuntime::new(data);
        runtime.load_profile(ScriptProfileId::INITIAL).unwrap();
        let arche = runtime
            .current_profile()
            .and_then(|profile| profile.builtins().archetype)
            .unwrap();
        let mut selector = RuntimeShipTargetSelector::default();
        let mut state = ShipTargetSelectionState {
            phase: u8::MIN,
            transition_step: u16::MIN,
            transition_total_steps: TARGET_TRANSITION_STEPS,
            fallback_active: false,
            current_target: arche,
            depth_opening_flags: u8::MIN,
            depth_step: u8::MIN,
        };
        let idle = selector
            .update(
                &mut runtime,
                ChoiceListPointer::default(),
                u16::MIN,
                &mut state,
                &[],
            )
            .unwrap();
        assert_eq!(idle.outcome, ShipTargetSelectionOutcome::NoSelection);
        assert!(state.fallback_active);
        let item_position = selector
            .last_frame()
            .unwrap()
            .rows
            .iter()
            .find_map(|row| matches!(row.kind, ChoiceListRowKind::Item(0)).then_some(row.position))
            .unwrap();
        let selected = selector
            .update(
                &mut runtime,
                pressed_pointer(item_position),
                u16::MIN,
                &mut state,
                &[],
            )
            .unwrap();
        assert_eq!(
            selected.outcome,
            ShipTargetSelectionOutcome::Selected(arche)
        );
    }

    #[test]
    fn every_shipped_profile_has_a_typed_arche_navigation_link() {
        let Some(data) = original_game_data() else {
            return;
        };
        let mut runtime = OriginalGameRuntime::new(data);
        for profile_id in ScriptProfileId::all() {
            runtime.load_profile(profile_id).unwrap();
            let profile = runtime.current_profile().unwrap();
            let arche = profile
                .builtins()
                .archetype
                .unwrap_or_else(|| panic!("profile {} has no Arche", profile_id.value()));
            let (link, directly_selectable) = ship_hud_arche_link(profile.state(), arche).unwrap();
            let linked_kind = profile.state().object(link).unwrap().kind;
            assert_eq!(
                directly_selectable,
                !matches!(
                    linked_kind,
                    ScriptObjectKind::Auxiliary | ScriptObjectKind::BlackHole
                ),
                "profile {}",
                profile_id.value()
            );
        }
    }

    fn pressed_pointer(position: [u16; 2]) -> ChoiceListPointer {
        ChoiceListPointer {
            position: [position[0] as i16, position[1] as i16],
            primary_pressed: true,
        }
    }

    fn original_game_data() -> Option<OriginalGameData> {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        [
            workspace_root.join("output/_tmp_iso"),
            workspace_root.join("commander-blood-audio/_tmp_iso"),
            workspace_root.join("accuracy/cblood_install/cblood"),
        ]
        .into_iter()
        .find_map(|root: PathBuf| OriginalGameDataPaths::from_root(root).ok())
        .and_then(|paths| {
            OriginalGameData::load_with_writable_root(paths, std::env::temp_dir()).ok()
        })
    }
}
