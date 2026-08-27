//! Bridge-frame orchestration over typed state and renderer services.

const BRIDGE_LEFT_SCREEN_EDGE: u16 = 160;

/// Semantic scene state published while steering or rebuilding the bridge view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BridgeActorPresentationState {
    /// No state was published by this frame coordinator.
    #[default]
    Unchanged,
    /// The bridge screen is being rebuilt.
    Rebuilding,
    /// Steering changed while the pointer was on the right side of the screen.
    SteeringRight,
    /// Steering changed while the pointer was on the left side of the screen.
    SteeringLeft,
    /// Pointer hover owns one authored presentation orb.
    PresentationHover,
}

/// Typed sprite groups committed or redrawn by the bridge coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeSpriteRange {
    /// Every bridge sprite.
    All,
    /// Sprites used by an active camera transition.
    Transition,
    /// Character and presentation sprites drawn after a completed frame.
    Actors,
}

/// Flat scene data threaded through one bridge frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeSceneContext<SceneLink, ComparisonExtent> {
    scene_link: SceneLink,
    comparison_extent: ComparisonExtent,
}

impl<SceneLink, ComparisonExtent> BridgeSceneContext<SceneLink, ComparisonExtent> {
    /// Build a context from independent scene-link and comparison-extent values.
    pub const fn new(scene_link: SceneLink, comparison_extent: ComparisonExtent) -> Self {
        Self {
            scene_link,
            comparison_extent,
        }
    }

    /// Return the scene link consumed by presentation dispatchers.
    pub const fn scene_link(&self) -> &SceneLink {
        &self.scene_link
    }

    /// Return the source extent consumed by camera-state checks.
    pub const fn comparison_extent(&self) -> &ComparisonExtent {
        &self.comparison_extent
    }
}

/// Mutable semantic state observed throughout one bridge frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BridgeFrameState {
    active: bool,
    scene_dispatch_pending: bool,
    screen_rebuild_pending: bool,
    transition_pending: bool,
    presentation_queued: bool,
    primary_camera_view: bool,
    frame_ready: bool,
    actor_completion: bool,
    clip_snapshot_ready: bool,
    mouse_x: u16,
    actor_presentation: BridgeActorPresentationState,
    previous_presentation: BridgeActorPresentationState,
}

impl BridgeFrameState {
    /// Set whether bridge rendering is active.
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Set whether this frame must dispatch its scene immediately.
    pub fn set_scene_dispatch_pending(&mut self, pending: bool) {
        self.scene_dispatch_pending = pending;
    }

    /// Set whether bridge screen state must be rebuilt before steering.
    pub fn set_screen_rebuild_pending(&mut self, pending: bool) {
        self.screen_rebuild_pending = pending;
    }

    /// Set whether the camera transition state machine must advance.
    pub fn set_transition_pending(&mut self, pending: bool) {
        self.transition_pending = pending;
    }

    /// Return whether a camera transition is pending.
    pub const fn transition_pending(&self) -> bool {
        self.transition_pending
    }

    /// Set whether a presentation is queued and suppresses dirty-region work.
    pub fn set_presentation_queued(&mut self, queued: bool) {
        self.presentation_queued = queued;
    }

    /// Return whether a presentation is queued.
    pub const fn presentation_queued(&self) -> bool {
        self.presentation_queued
    }

    /// Set whether the bridge camera is in its primary view.
    pub fn set_primary_camera_view(&mut self, primary: bool) {
        self.primary_camera_view = primary;
    }

    /// Set whether the current resource frame is ready for final presentation.
    pub fn set_frame_ready(&mut self, ready: bool) {
        self.frame_ready = ready;
    }

    /// Return whether the current resource frame is ready.
    pub const fn frame_ready(&self) -> bool {
        self.frame_ready
    }

    /// Set whether actor presentation completed during this frame.
    pub fn set_actor_completion(&mut self, complete: bool) {
        self.actor_completion = complete;
    }

    /// Return whether actor presentation completed during this frame.
    pub const fn actor_completion(&self) -> bool {
        self.actor_completion
    }

    /// Set the horizontal pointer coordinate used after a steering change.
    pub fn set_mouse_x(&mut self, mouse_x: u16) {
        self.mouse_x = mouse_x;
    }

    /// Return the actor-presentation state published by this frame.
    pub const fn actor_presentation(&self) -> BridgeActorPresentationState {
        self.actor_presentation
    }

    /// Return the previous presentation state published during a rebuild.
    pub const fn previous_presentation(&self) -> BridgeActorPresentationState {
        self.previous_presentation
    }

    /// Return whether sprite geometry was snapshotted before presentation dispatch.
    pub const fn clip_snapshot_ready(&self) -> bool {
        self.clip_snapshot_ready
    }
}

/// Host and subsystem operations sequenced by the bridge frame coordinator.
pub trait BridgeFrameBackend {
    /// Typed scene-link value used by presentation dispatchers.
    type SceneLink;
    /// Typed source extent used by camera-state checks.
    type ComparisonExtent;
    /// Backend failure from resource, rendering, world, or presentation work.
    type Error;

    /// Dispatch an already pending scene and stop this bridge frame.
    fn dispatch_scene(
        &mut self,
        scene_link: &Self::SceneLink,
        state: &mut BridgeFrameState,
    ) -> Result<(), Self::Error>;
    /// Rebuild screen flags before the normal frame sequence.
    fn initialize_screen_flags(&mut self, state: &mut BridgeFrameState) -> Result<(), Self::Error>;
    /// Update bridge steering, including replacement of the current scene context.
    fn update_steering(
        &mut self,
        context: &mut BridgeSceneContext<Self::SceneLink, Self::ComparisonExtent>,
        state: &mut BridgeFrameState,
    ) -> Result<bool, Self::Error>;
    /// Present the steering update immediately.
    fn flip_page(&mut self, state: &mut BridgeFrameState) -> Result<(), Self::Error>;
    /// Advance the camera transition state machine.
    fn advance_camera_transition(
        &mut self,
        scene_link: &Self::SceneLink,
        state: &mut BridgeFrameState,
    ) -> Result<(), Self::Error>;
    /// Update presentation-mode bits before committing sprites.
    fn update_presentation_mode_bits(
        &mut self,
        state: &mut BridgeFrameState,
    ) -> Result<(), Self::Error>;
    /// Commit current geometry for one semantic sprite range.
    fn commit_sprite_geometry(
        &mut self,
        range: BridgeSpriteRange,
        state: &mut BridgeFrameState,
    ) -> Result<(), Self::Error>;
    /// Dispatch presentation-mode rendering.
    fn dispatch_presentation_mode(
        &mut self,
        state: &mut BridgeFrameState,
    ) -> Result<(), Self::Error>;
    /// Update all bridge actor slots.
    fn update_actor_slots(&mut self, state: &mut BridgeFrameState) -> Result<(), Self::Error>;
    /// Render dirty sprites from one semantic range.
    fn render_dirty_sprites(
        &mut self,
        range: BridgeSpriteRange,
        state: &mut BridgeFrameState,
    ) -> Result<(), Self::Error>;
    /// Copy dirty work-surface pixels into the display surface.
    fn copy_dirty_regions(&mut self, state: &mut BridgeFrameState) -> Result<(), Self::Error>;
    /// Reconcile camera state against the scene's typed comparison extent.
    fn check_camera_state(
        &mut self,
        comparison_extent: &Self::ComparisonExtent,
        state: &mut BridgeFrameState,
    ) -> Result<(), Self::Error>;
    /// Update camera navigation after state reconciliation.
    fn update_camera_navigation(&mut self, state: &mut BridgeFrameState)
    -> Result<(), Self::Error>;
    /// Update the bridge presentation screen for the current scene link.
    fn update_screen_mode(
        &mut self,
        scene_link: &Self::SceneLink,
        state: &mut BridgeFrameState,
    ) -> Result<(), Self::Error>;
    /// Apply the character-name-area palette effect.
    fn update_name_area_palette(&mut self, state: &mut BridgeFrameState)
    -> Result<(), Self::Error>;
    /// Update navigation state after the frame becomes ready.
    fn update_navigation_state(&mut self, state: &mut BridgeFrameState) -> Result<(), Self::Error>;
    /// Dispatch a pending navigation choice.
    fn dispatch_navigation_choice(
        &mut self,
        state: &mut BridgeFrameState,
    ) -> Result<(), Self::Error>;
    /// Apply the fixed completion-region palette remap.
    fn remap_completion_region(&mut self, state: &mut BridgeFrameState) -> Result<(), Self::Error>;
}

/// Terminal path taken by one bridge-frame update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeFrameOutcome {
    /// Rendering was inactive and no backend operation ran.
    Inactive,
    /// A pending scene was dispatched through the early path.
    SceneDispatched,
    /// Core frame work ran but no resource frame was ready.
    WaitingForFrame,
    /// The complete bridge frame and late navigation stage ran.
    Presented,
}

/// Run one bridge frame through typed state and subsystem boundaries.
///
/// This translates `bridge_render_frame` at BLOODPRG file offset `0x0077E0`.
/// Separate scene-link and comparison-extent values replace the native context
/// alias, while semantic booleans and ranges replace packed flag and slot values.
pub fn render_bridge_frame<Backend: BridgeFrameBackend>(
    state: &mut BridgeFrameState,
    context: &mut BridgeSceneContext<Backend::SceneLink, Backend::ComparisonExtent>,
    backend: &mut Backend,
) -> Result<BridgeFrameOutcome, Backend::Error> {
    if !state.active {
        return Ok(BridgeFrameOutcome::Inactive);
    }

    if state.scene_dispatch_pending {
        backend.dispatch_scene(context.scene_link(), state)?;
        return Ok(BridgeFrameOutcome::SceneDispatched);
    }

    if state.screen_rebuild_pending {
        state.actor_presentation = BridgeActorPresentationState::Rebuilding;
        state.previous_presentation = BridgeActorPresentationState::Rebuilding;
        backend.initialize_screen_flags(state)?;
    }

    if backend.update_steering(context, state)? {
        state.actor_presentation = if state.mouse_x <= BRIDGE_LEFT_SCREEN_EDGE {
            BridgeActorPresentationState::SteeringLeft
        } else {
            BridgeActorPresentationState::SteeringRight
        };
        backend.flip_page(state)?;
    }

    if state.transition_pending {
        backend.advance_camera_transition(context.scene_link(), state)?;
    }
    backend.update_presentation_mode_bits(state)?;
    backend.commit_sprite_geometry(BridgeSpriteRange::All, state)?;
    state.clip_snapshot_ready = true;
    backend.dispatch_presentation_mode(state)?;
    backend.update_actor_slots(state)?;

    if !state.presentation_queued {
        if state.transition_pending {
            backend.render_dirty_sprites(BridgeSpriteRange::Transition, state)?;
        } else if state.primary_camera_view {
            backend.copy_dirty_regions(state)?;
        }
    }

    backend.check_camera_state(context.comparison_extent(), state)?;
    backend.update_camera_navigation(state)?;
    backend.update_screen_mode(context.scene_link(), state)?;
    if !state.frame_ready {
        return Ok(BridgeFrameOutcome::WaitingForFrame);
    }

    backend.render_dirty_sprites(BridgeSpriteRange::Actors, state)?;
    backend.update_name_area_palette(state)?;
    backend.update_navigation_state(state)?;
    backend.dispatch_navigation_choice(state)?;
    if state.actor_completion {
        backend.remap_completion_region(state)?;
    }
    Ok(BridgeFrameOutcome::Presented)
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 21;
    const ACTIVE_MASK: u16 = 1;
    const SCENE_DISPATCH_MASK: u8 = 2;
    const ORACLE_ALL_FIRST: u16 = 0;
    const ORACLE_ALL_LAST: u16 = 31;
    const ORACLE_TRANSITION_FIRST: u16 = 20;
    const ORACLE_TRANSITION_LAST: u16 = 31;
    const ORACLE_ACTORS_FIRST: u16 = 1;
    const ORACLE_ACTORS_LAST: u16 = 19;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum RecordedCall {
        DispatchScene,
        InitializeScreen,
        UpdateSteering,
        FlipPage,
        AdvanceTransition,
        UpdateModeBits,
        CommitSprites(BridgeSpriteRange),
        DispatchMode,
        UpdateActorSlots,
        RenderSprites(BridgeSpriteRange),
        CopyDirtyRegions,
        CheckCamera,
        UpdateCamera,
        UpdateScreen,
        UpdateNameArea,
        UpdateNavigation,
        DispatchChoice,
        CompletionRemap,
    }

    #[derive(Deserialize)]
    struct CallOracle {
        call: String,
        first_object: Option<u16>,
        last_object: Option<u16>,
    }

    #[derive(Deserialize)]
    struct BridgeOracle {
        name: String,
        ui: u16,
        transition_phase: u8,
        bridge_changed: bool,
        entry_context: u16,
        bridge_context: u16,
        presentation_state_after: u16,
        queue_after: u8,
        frame_ready_after: u8,
        completion_after: u8,
        final_remap: bool,
        calls: Vec<CallOracle>,
    }

    struct RecordingBackend {
        calls: Vec<RecordedCall>,
        replacement_link: u16,
        replacement_extent: u16,
        bridge_changed: bool,
        transition_during_initialize: bool,
        queue_during_actor_update: bool,
        frame_ready_during_screen_update: bool,
        completion_during_choice: bool,
        dispatched_links: Vec<u16>,
        screen_links: Vec<u16>,
        checked_extents: Vec<u16>,
    }

    impl BridgeFrameBackend for RecordingBackend {
        type SceneLink = u16;
        type ComparisonExtent = u16;
        type Error = Infallible;

        fn dispatch_scene(
            &mut self,
            scene_link: &u16,
            _state: &mut BridgeFrameState,
        ) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::DispatchScene);
            self.dispatched_links.push(*scene_link);
            Ok(())
        }

        fn initialize_screen_flags(
            &mut self,
            state: &mut BridgeFrameState,
        ) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::InitializeScreen);
            if self.transition_during_initialize {
                state.set_transition_pending(true);
            }
            Ok(())
        }

        fn update_steering(
            &mut self,
            context: &mut BridgeSceneContext<u16, u16>,
            _state: &mut BridgeFrameState,
        ) -> Result<bool, Self::Error> {
            self.calls.push(RecordedCall::UpdateSteering);
            *context = BridgeSceneContext::new(self.replacement_link, self.replacement_extent);
            Ok(self.bridge_changed)
        }

        fn flip_page(&mut self, _state: &mut BridgeFrameState) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::FlipPage);
            Ok(())
        }

        fn advance_camera_transition(
            &mut self,
            _scene_link: &u16,
            _state: &mut BridgeFrameState,
        ) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::AdvanceTransition);
            Ok(())
        }

        fn update_presentation_mode_bits(
            &mut self,
            _state: &mut BridgeFrameState,
        ) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::UpdateModeBits);
            Ok(())
        }

        fn commit_sprite_geometry(
            &mut self,
            range: BridgeSpriteRange,
            _state: &mut BridgeFrameState,
        ) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::CommitSprites(range));
            Ok(())
        }

        fn dispatch_presentation_mode(
            &mut self,
            _state: &mut BridgeFrameState,
        ) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::DispatchMode);
            Ok(())
        }

        fn update_actor_slots(&mut self, state: &mut BridgeFrameState) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::UpdateActorSlots);
            if self.queue_during_actor_update {
                state.set_presentation_queued(true);
            }
            Ok(())
        }

        fn render_dirty_sprites(
            &mut self,
            range: BridgeSpriteRange,
            _state: &mut BridgeFrameState,
        ) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::RenderSprites(range));
            Ok(())
        }

        fn copy_dirty_regions(&mut self, _state: &mut BridgeFrameState) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::CopyDirtyRegions);
            Ok(())
        }

        fn check_camera_state(
            &mut self,
            comparison_extent: &u16,
            _state: &mut BridgeFrameState,
        ) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::CheckCamera);
            self.checked_extents.push(*comparison_extent);
            Ok(())
        }

        fn update_camera_navigation(
            &mut self,
            _state: &mut BridgeFrameState,
        ) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::UpdateCamera);
            Ok(())
        }

        fn update_screen_mode(
            &mut self,
            scene_link: &u16,
            state: &mut BridgeFrameState,
        ) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::UpdateScreen);
            self.screen_links.push(*scene_link);
            if self.frame_ready_during_screen_update {
                state.set_frame_ready(true);
            }
            Ok(())
        }

        fn update_name_area_palette(
            &mut self,
            _state: &mut BridgeFrameState,
        ) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::UpdateNameArea);
            Ok(())
        }

        fn update_navigation_state(
            &mut self,
            _state: &mut BridgeFrameState,
        ) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::UpdateNavigation);
            Ok(())
        }

        fn dispatch_navigation_choice(
            &mut self,
            state: &mut BridgeFrameState,
        ) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::DispatchChoice);
            if self.completion_during_choice {
                state.set_actor_completion(true);
            }
            Ok(())
        }

        fn remap_completion_region(
            &mut self,
            _state: &mut BridgeFrameState,
        ) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::CompletionRemap);
            Ok(())
        }
    }

    fn oracle_sprite_range(call: &CallOracle) -> BridgeSpriteRange {
        match (call.first_object, call.last_object) {
            (Some(ORACLE_ALL_FIRST), Some(ORACLE_ALL_LAST)) => BridgeSpriteRange::All,
            (Some(ORACLE_TRANSITION_FIRST), Some(ORACLE_TRANSITION_LAST)) => {
                BridgeSpriteRange::Transition
            }
            (Some(ORACLE_ACTORS_FIRST), Some(ORACLE_ACTORS_LAST)) => BridgeSpriteRange::Actors,
            range => panic!("unknown oracle sprite range {range:?}"),
        }
    }

    fn expected_call(call: &CallOracle) -> RecordedCall {
        match call.call.as_str() {
            "dlg_line_id_scene_dispatch" => RecordedCall::DispatchScene,
            "screen_flags_init" => RecordedCall::InitializeScreen,
            "bridge_steer_update" => RecordedCall::UpdateSteering,
            "page_flip" => RecordedCall::FlipPage,
            "camera_fsm_state_gate" => RecordedCall::AdvanceTransition,
            "presentation_mode_bits_update" => RecordedCall::UpdateModeBits,
            "sprite_slot_commit_dirty_range" => {
                RecordedCall::CommitSprites(oracle_sprite_range(call))
            }
            "presentation_mode_dispatch" => RecordedCall::DispatchMode,
            "nav_actor_slot_update_loop" => RecordedCall::UpdateActorSlots,
            "sprite_slot_dirty_range_render" => {
                RecordedCall::RenderSprites(oracle_sprite_range(call))
            }
            "dirty_rects_copy_secondary_to_primary" => RecordedCall::CopyDirtyRegions,
            "nav_camera_state_check" => RecordedCall::CheckCamera,
            "camera_nav_update" => RecordedCall::UpdateCamera,
            "screen_mode_update" => RecordedCall::UpdateScreen,
            "mode_gate_27e8" => RecordedCall::UpdateNameArea,
            "nav_state_gate" => RecordedCall::UpdateNavigation,
            "nav_choice_dispatch" => RecordedCall::DispatchChoice,
            "framebuffer_rect_palette_remap" => RecordedCall::CompletionRemap,
            name => panic!("unknown bridge oracle call {name}"),
        }
    }

    #[test]
    fn bridge_frame_matches_every_original_coordinator_vector() {
        let vectors: Vec<BridgeOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_77e0_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let expected_calls: Vec<_> = vector.calls.iter().map(expected_call).collect();
            let transition_during_initialize =
                vector.name == "screen_rebuild_callback_sets_transition";
            let queue_during_actor_update = vector.name == "actor_callback_sets_queue";
            let frame_ready_during_screen_update =
                vector.name == "screen_callback_sets_frame_ready";
            let completion_during_choice = vector.name == "choice_callback_completes";

            let mut state = BridgeFrameState::default();
            state.set_active(vector.ui & ACTIVE_MASK != u16::MIN);
            state.set_scene_dispatch_pending(
                vector.transition_phase & SCENE_DISPATCH_MASK != u8::MIN,
            );
            state.set_screen_rebuild_pending(
                expected_calls.contains(&RecordedCall::InitializeScreen),
            );
            state.set_transition_pending(
                expected_calls.contains(&RecordedCall::AdvanceTransition)
                    && !transition_during_initialize,
            );
            state.set_presentation_queued(
                vector.queue_after & 1 != u8::MIN && !queue_during_actor_update,
            );
            state.set_primary_camera_view(vector.name != "camera_state_suppresses_dirty_copy");
            state.set_frame_ready(
                vector.frame_ready_after & 1 != u8::MIN && !frame_ready_during_screen_update,
            );
            state.set_actor_completion(
                vector.completion_after & 1 != u8::MIN && !completion_during_choice,
            );
            state.set_mouse_x(if vector.presentation_state_after == 3 {
                BRIDGE_LEFT_SCREEN_EDGE
            } else {
                BRIDGE_LEFT_SCREEN_EDGE + 1
            });

            let mut context = BridgeSceneContext::new(vector.entry_context, vector.entry_context);
            let mut backend = RecordingBackend {
                calls: Vec::new(),
                replacement_link: vector.bridge_context,
                replacement_extent: vector.bridge_context,
                bridge_changed: vector.bridge_changed,
                transition_during_initialize,
                queue_during_actor_update,
                frame_ready_during_screen_update,
                completion_during_choice,
                dispatched_links: Vec::new(),
                screen_links: Vec::new(),
                checked_extents: Vec::new(),
            };

            let outcome = render_bridge_frame(&mut state, &mut context, &mut backend).unwrap();

            assert_eq!(backend.calls, expected_calls, "{}", vector.name);
            assert_eq!(state.presentation_queued(), vector.queue_after & 1 != 0);
            assert_eq!(state.frame_ready(), vector.frame_ready_after & 1 != 0);
            assert_eq!(state.actor_completion(), vector.completion_after & 1 != 0);
            assert_eq!(
                backend.calls.contains(&RecordedCall::CompletionRemap),
                vector.final_remap,
                "{}",
                vector.name
            );
            let expected_presentation = match vector.presentation_state_after {
                1 => BridgeActorPresentationState::Rebuilding,
                2 => BridgeActorPresentationState::SteeringRight,
                3 => BridgeActorPresentationState::SteeringLeft,
                _ => BridgeActorPresentationState::Unchanged,
            };
            assert_eq!(
                state.actor_presentation(),
                expected_presentation,
                "{}",
                vector.name
            );
            let expected_outcome = if vector.ui & ACTIVE_MASK == u16::MIN {
                BridgeFrameOutcome::Inactive
            } else if vector.transition_phase & SCENE_DISPATCH_MASK != u8::MIN {
                BridgeFrameOutcome::SceneDispatched
            } else if vector.frame_ready_after & 1 == u8::MIN {
                BridgeFrameOutcome::WaitingForFrame
            } else {
                BridgeFrameOutcome::Presented
            };
            assert_eq!(outcome, expected_outcome, "{}", vector.name);
        }
    }
}
