//! Bridge presentation panel phase machine and subsystem coordination.

use super::{PresentationChoiceNumber, SequenceSubtitlePlayback};

const PANEL_FILL_COLOR: u8 = 224;
const PANEL_FRAME_COLOR: u8 = 239;
const PANEL_NOISE_MODE: u8 = 3;
const CONTENT_TOP: u16 = 10;
const CONTENT_HEIGHT: u16 = 140;
const CONTENT_FRAME_HEIGHT: u16 = 130;
const SCREEN_WIDTH: u16 = 320;
const SCREEN_HEIGHT: u16 = 200;

const FULL_SCREEN: PresentationRenderRegion =
    PresentationRenderRegion::new([0, 0], [SCREEN_WIDTH, SCREEN_HEIGHT]);
const CONTENT_CLEAR: PresentationRenderRegion =
    PresentationRenderRegion::new([0, 0], [SCREEN_WIDTH, CONTENT_HEIGHT]);
const CONTENT_FRAME: PresentationRenderRegion =
    PresentationRenderRegion::new([0, CONTENT_TOP], [SCREEN_WIDTH, CONTENT_FRAME_HEIGHT]);
const CONTENT_NOISE: PresentationRenderRegion =
    PresentationRenderRegion::new([1, CONTENT_TOP], [SCREEN_WIDTH - 1, CONTENT_FRAME_HEIGHT]);
const PANEL_RECTS: [PresentationRenderRegion; PresentationPanelStep::COUNT] = [
    PresentationRenderRegion::new([155, 67], [10, 15]),
    PresentationRenderRegion::new([143, 57], [34, 35]),
    PresentationRenderRegion::new([120, 51], [80, 47]),
    PresentationRenderRegion::new([76, 43], [168, 63]),
    PresentationRenderRegion::new([26, 30], [268, 89]),
    CONTENT_FRAME,
];

/// Logical indexed-framebuffer rectangle used by panel rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationRenderRegion {
    /// Upper-left logical pixel.
    pub origin: [u16; 2],
    /// Logical width and height.
    pub size: [u16; 2],
}

impl PresentationRenderRegion {
    /// Build a logical render rectangle.
    pub const fn new(origin: [u16; 2], size: [u16; 2]) -> Self {
        Self { origin, size }
    }
}

/// Semantic framebuffer selected for one panel render operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationRenderTarget {
    /// Buffer currently presented to the player.
    Front,
    /// Retained buffer prepared for a later presentation.
    Back,
}

/// One of the six authored panel-expansion rectangles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationPanelStep {
    /// Small center rectangle.
    One,
    /// Second expansion rectangle.
    Two,
    /// Third expansion rectangle.
    Three,
    /// Fourth expansion rectangle.
    Four,
    /// Fifth expansion rectangle.
    Five,
    /// Full content rectangle.
    Six,
}

impl PresentationPanelStep {
    /// Number of authored panel rectangles.
    pub const COUNT: usize = 6;

    const fn index(self) -> usize {
        match self {
            Self::One => 0,
            Self::Two => 1,
            Self::Three => 2,
            Self::Four => 3,
            Self::Five => 4,
            Self::Six => 5,
        }
    }

    const fn next(self) -> Option<Self> {
        match self {
            Self::One => Some(Self::Two),
            Self::Two => Some(Self::Three),
            Self::Three => Some(Self::Four),
            Self::Four => Some(Self::Five),
            Self::Five => Some(Self::Six),
            Self::Six => None,
        }
    }

    const fn previous(self) -> Option<Self> {
        match self {
            Self::One => None,
            Self::Two => Some(Self::One),
            Self::Three => Some(Self::Two),
            Self::Four => Some(Self::Three),
            Self::Five => Some(Self::Four),
            Self::Six => Some(Self::Five),
        }
    }
}

/// One of the three palette/noise transition frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationTransitionFrame {
    /// First transition frame.
    One,
    /// Second transition frame.
    Two,
    /// Final transition frame.
    Three,
}

impl PresentationTransitionFrame {
    const fn next(self) -> Option<Self> {
        match self {
            Self::One => Some(Self::Two),
            Self::Two => Some(Self::Three),
            Self::Three => None,
        }
    }
}

/// Typed phase of the bridge presentation panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationPanelPhase {
    /// First active call initializes presentation ownership.
    Begin,
    /// Draw one expanding panel rectangle.
    Opening(PresentationPanelStep),
    /// Apply one palette/noise transition frame.
    Transition(PresentationTransitionFrame),
    /// Present the selected record and service its scene queue.
    Active,
    /// Draw one contracting panel rectangle.
    Closing(PresentationPanelStep),
    /// Release presentation ownership after the last closing rectangle.
    Finalizing,
}

/// Music action requested by a newly applied DESCRIPT record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationMusicChange {
    /// Continue using the current background stream.
    Retained,
    /// Reload the music selected by the DESCRIPT record.
    Reload,
}

/// Vertical text layout selected by the panel lifecycle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PresentationTextOrigin {
    /// Normal bridge text layout used outside panel opening.
    #[default]
    Normal,
    /// Temporary opening layout used while the panel owns the bridge.
    Opening,
}

/// Placement of the currently loaded presentation resource.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PresentationResourcePlacement {
    /// No selected record occupies the presentation content area.
    #[default]
    Hidden,
    /// The resource begins at the panel's content row.
    ContentPanel,
}

/// Owned scene work produced by applying one selected DESCRIPT record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationDescriptPlan {
    music: PresentationMusicChange,
    scene_lines: Vec<Box<[u8]>>,
}

impl PresentationDescriptPlan {
    /// Build a plan from its music decision and authored scene lines.
    pub fn new(music: PresentationMusicChange, scene_lines: Vec<Box<[u8]>>) -> Self {
        Self { music, scene_lines }
    }
}

/// Scene context passed to the dialogue/presentation dispatcher.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationSceneContext<'a, SceneLink> {
    /// Scene context inherited from the bridge frame that queued the work.
    Queued(&'a SceneLink),
    /// Fixed presentation content area used for a newly selected record.
    ContentPanel,
}

/// Queue state returned after one scene-dispatch call.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresentationSceneStatus {
    /// Whether the dispatched line still owns the presentation queue.
    pub queued: bool,
    /// Whether a video/resource frame is ready for subtitle and mask drawing.
    pub frame_presented: bool,
}

/// Mutable semantic state owned by the presentation panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationScreenState {
    active: bool,
    phase: PresentationPanelPhase,
    selected_choice: PresentationChoiceNumber,
    reverse: bool,
    primary_pressed: bool,
    scene_status: PresentationSceneStatus,
    scene_lines: Vec<Box<[u8]>>,
    next_scene_line: usize,
    current_scene_line: Option<Box<[u8]>>,
    subtitle_playback: SequenceSubtitlePlayback,
    text_origin: PresentationTextOrigin,
    resource_placement: PresentationResourcePlacement,
    redraw_requested: bool,
    panel_hover_restore_requested: bool,
    screen_rebuild_pending: bool,
    completion_audio_pending: bool,
    choice_change_animation_requested: bool,
    reverse_resource_variant_restored: bool,
}

impl Default for PresentationScreenState {
    fn default() -> Self {
        Self {
            active: false,
            phase: PresentationPanelPhase::Begin,
            selected_choice: PresentationChoiceNumber::One,
            reverse: false,
            primary_pressed: false,
            scene_status: PresentationSceneStatus::default(),
            scene_lines: Vec::new(),
            next_scene_line: usize::MIN,
            current_scene_line: None,
            subtitle_playback: SequenceSubtitlePlayback::default(),
            text_origin: PresentationTextOrigin::default(),
            resource_placement: PresentationResourcePlacement::default(),
            redraw_requested: false,
            panel_hover_restore_requested: false,
            screen_rebuild_pending: false,
            completion_audio_pending: false,
            choice_change_animation_requested: false,
            reverse_resource_variant_restored: false,
        }
    }
}

impl PresentationScreenState {
    /// Set whether the bridge presentation panel owns this frame.
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Return whether the bridge presentation panel owns this frame.
    pub const fn active(&self) -> bool {
        self.active
    }

    /// Replace the typed panel phase.
    pub fn set_phase(&mut self, phase: PresentationPanelPhase) {
        self.phase = phase;
    }

    /// Return the current typed panel phase.
    pub const fn phase(&self) -> PresentationPanelPhase {
        self.phase
    }

    /// Select one of the six authored presentation records.
    pub fn set_selected_choice(&mut self, choice: PresentationChoiceNumber) {
        self.selected_choice = choice;
    }

    /// Return the selected presentation record.
    pub const fn selected_choice(&self) -> PresentationChoiceNumber {
        self.selected_choice
    }

    /// Set whether accepted input closes the panel instead of cycling records.
    pub fn set_reverse(&mut self, reverse: bool) {
        self.reverse = reverse;
    }

    /// Set the primary-button edge for this frame.
    pub fn set_primary_pressed(&mut self, pressed: bool) {
        self.primary_pressed = pressed;
    }

    /// Replace scene queue state, normally from an outer bridge dispatcher.
    pub fn set_scene_status(&mut self, status: PresentationSceneStatus) {
        self.scene_status = status;
    }

    /// Return current scene queue state.
    pub const fn scene_status(&self) -> PresentationSceneStatus {
        self.scene_status
    }

    /// Number of loaded scene lines not yet submitted.
    pub fn remaining_scene_lines(&self) -> usize {
        self.scene_lines.len().saturating_sub(self.next_scene_line)
    }

    /// Return mutable sequence-subtitle playback for renderer integration.
    pub fn subtitle_playback_mut(&mut self) -> &mut SequenceSubtitlePlayback {
        &mut self.subtitle_playback
    }

    /// Return the panel-owned text layout.
    pub const fn text_origin(&self) -> PresentationTextOrigin {
        self.text_origin
    }

    /// Return where the selected resource is placed.
    pub const fn resource_placement(&self) -> PresentationResourcePlacement {
        self.resource_placement
    }

    /// Set whether the outer bridge requested a presentation redraw.
    pub fn set_redraw_requested(&mut self, requested: bool) {
        self.redraw_requested = requested;
    }

    /// Return whether the outer bridge requested a presentation redraw.
    pub const fn redraw_requested(&self) -> bool {
        self.redraw_requested
    }

    /// Return whether hover exit should restore the active panel actor state.
    pub const fn panel_hover_restore_requested(&self) -> bool {
        self.panel_hover_restore_requested
    }

    /// Return whether the bridge screen must be rebuilt.
    pub const fn screen_rebuild_pending(&self) -> bool {
        self.screen_rebuild_pending
    }

    /// Return whether completion audio must run after finalization.
    pub const fn completion_audio_pending(&self) -> bool {
        self.completion_audio_pending
    }

    /// Return whether MANU3 must play the choice-change animation.
    pub const fn choice_change_animation_requested(&self) -> bool {
        self.choice_change_animation_requested
    }

    /// Return whether reverse close restored its resource variant.
    pub const fn reverse_resource_variant_restored(&self) -> bool {
        self.reverse_resource_variant_restored
    }
}

/// Rendering, resource, audio, and scene services used by the phase machine.
pub trait PresentationScreenBackend {
    /// Typed name used to select a DESCRIPT record.
    type RecordName;
    /// Typed bridge scene-link value.
    type SceneLink;
    /// Backend failure from resource or scene work.
    type Error;

    /// Fill one logical rectangle in one semantic framebuffer.
    fn fill_region(
        &mut self,
        target: PresentationRenderTarget,
        color: u8,
        region: PresentationRenderRegion,
    );
    /// Draw the panel border around one logical rectangle.
    fn frame_region(
        &mut self,
        target: PresentationRenderTarget,
        color: u8,
        region: PresentationRenderRegion,
    );
    /// Apply the recovered presentation palette remap.
    fn remap_palette(&mut self, target: PresentationRenderTarget, region: PresentationRenderRegion);
    /// Draw presentation noise in one logical rectangle.
    fn draw_noise(
        &mut self,
        target: PresentationRenderTarget,
        mode: u8,
        region: PresentationRenderRegion,
    );
    /// Transition the presentation entity into its opening state.
    fn transition_presentation_entity(&mut self);
    /// Play the recovered presentation open/selection clip.
    fn play_presentation_clip(&mut self);
    /// Apply a selected DESCRIPT record and return its owned scene plan.
    fn load_descript(
        &mut self,
        record: &Self::RecordName,
    ) -> Result<PresentationDescriptPlan, Self::Error>;
    /// Reload music selected by the last DESCRIPT application.
    fn reload_descript_music(&mut self) -> Result<(), Self::Error>;
    /// Start or resume the current presentation music stream.
    fn start_music_stream(&mut self);
    /// Dispatch one scene line and return its resulting queue state.
    fn dispatch_scene(
        &mut self,
        context: &PresentationSceneContext<'_, Self::SceneLink>,
        line: Option<&[u8]>,
    ) -> Result<PresentationSceneStatus, Self::Error>;
    /// Draw the current frame-timed subtitle.
    fn draw_sequence_subtitle(&mut self, playback: &mut SequenceSubtitlePlayback);
    /// Draw the number mask for the selected presentation record.
    fn draw_choice_number(&mut self, choice: PresentationChoiceNumber);
    /// Release queued scene presentation state after accepted input.
    fn cancel_scene_presentation(&mut self);
    /// Prepare audio state for a normal choice change.
    fn prepare_choice_audio(&mut self);
    /// Restore the ship HUD palette and reset its camera after a normal close.
    fn reset_ship_camera(&mut self);
}

/// Terminal work performed by one phase-machine call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationScreenOutcome {
    /// Presentation ownership was inactive.
    Inactive,
    /// Presentation ownership and opening state were initialized.
    Initialized,
    /// One opening, transition, or closing animation frame was rendered.
    Animated,
    /// The active panel is waiting for input on an empty record slot.
    WaitingForSelection,
    /// A dispatched scene remains queued.
    WaitingForScene,
    /// All scene lines completed and a transition back to the panel began.
    SceneLinesCompleted,
    /// Primary input changed the selection or started closing.
    InputAccepted,
    /// Closing completed and presentation ownership was released.
    Finalized,
}

/// Advance one frame of the bridge presentation panel.
///
/// This translates `screen_mode_update` at BLOODPRG routine offset `0x0079E5`.
/// Typed phases, choices, owned scene lines, render targets, and callback results
/// replace numeric phases, fixed record tables, buffer swaps, and shared flags.
pub fn update_presentation_screen<Backend: PresentationScreenBackend>(
    state: &mut PresentationScreenState,
    records: &[Option<Backend::RecordName>; PresentationChoiceNumber::COUNT],
    queued_scene_link: &Backend::SceneLink,
    backend: &mut Backend,
) -> Result<PresentationScreenOutcome, Backend::Error> {
    if !state.active {
        return Ok(PresentationScreenOutcome::Inactive);
    }
    if state.scene_status.queued {
        if state.primary_pressed {
            return Ok(accept_input(state, backend));
        }
        return dispatch_and_pump(
            state,
            PresentationSceneContext::Queued(queued_scene_link),
            backend,
        );
    }

    match state.phase {
        PresentationPanelPhase::Begin => {
            state.selected_choice = PresentationChoiceNumber::One;
            state.phase = PresentationPanelPhase::Opening(PresentationPanelStep::One);
            state.text_origin = PresentationTextOrigin::Opening;
            state.panel_hover_restore_requested = true;
            backend.transition_presentation_entity();
            backend.play_presentation_clip();
            Ok(PresentationScreenOutcome::Initialized)
        }
        PresentationPanelPhase::Opening(step) => {
            draw_panel_rect(step, backend);
            state.phase = match step.next() {
                Some(next) => PresentationPanelPhase::Opening(next),
                None => PresentationPanelPhase::Transition(PresentationTransitionFrame::One),
            };
            Ok(PresentationScreenOutcome::Animated)
        }
        PresentationPanelPhase::Transition(frame) => {
            backend.remap_palette(PresentationRenderTarget::Front, FULL_SCREEN);
            backend.draw_noise(
                PresentationRenderTarget::Front,
                PANEL_NOISE_MODE,
                CONTENT_NOISE,
            );
            state.phase = match frame.next() {
                Some(next) => PresentationPanelPhase::Transition(next),
                None => PresentationPanelPhase::Active,
            };
            Ok(PresentationScreenOutcome::Animated)
        }
        PresentationPanelPhase::Active => {
            render_active_panel_background(backend);
            let Some(record) = records[state.selected_choice.index()].as_ref() else {
                backend.draw_noise(
                    PresentationRenderTarget::Front,
                    PANEL_NOISE_MODE,
                    CONTENT_NOISE,
                );
                backend.draw_choice_number(state.selected_choice);
                return if state.primary_pressed {
                    Ok(accept_input(state, backend))
                } else {
                    Ok(PresentationScreenOutcome::WaitingForSelection)
                };
            };

            let plan = backend.load_descript(record)?;
            if plan.music == PresentationMusicChange::Reload {
                backend.reload_descript_music()?;
            }
            backend.start_music_stream();
            state.scene_lines = plan.scene_lines;
            state.next_scene_line = usize::MIN;
            state.current_scene_line = None;
            state.subtitle_playback.restart();
            state.resource_placement = PresentationResourcePlacement::ContentPanel;
            dispatch_and_pump(state, PresentationSceneContext::ContentPanel, backend)
        }
        PresentationPanelPhase::Closing(step) => {
            draw_panel_rect(step, backend);
            state.phase = match step.previous() {
                Some(previous) => PresentationPanelPhase::Closing(previous),
                None => PresentationPanelPhase::Finalizing,
            };
            Ok(PresentationScreenOutcome::Animated)
        }
        PresentationPanelPhase::Finalizing => {
            state.active = false;
            state.phase = PresentationPanelPhase::Begin;
            state.resource_placement = PresentationResourcePlacement::Hidden;
            state.redraw_requested = false;
            state.completion_audio_pending = true;
            state.text_origin = PresentationTextOrigin::Normal;
            state.screen_rebuild_pending = true;
            if state.reverse {
                state.reverse = false;
                state.reverse_resource_variant_restored = true;
            } else {
                backend.reset_ship_camera();
            }
            Ok(PresentationScreenOutcome::Finalized)
        }
    }
}

fn draw_panel_rect<Backend: PresentationScreenBackend>(
    step: PresentationPanelStep,
    backend: &mut Backend,
) {
    let region = PANEL_RECTS[step.index()];
    backend.fill_region(PresentationRenderTarget::Front, PANEL_FILL_COLOR, region);
    backend.frame_region(PresentationRenderTarget::Front, PANEL_FRAME_COLOR, region);
}

fn render_active_panel_background<Backend: PresentationScreenBackend>(backend: &mut Backend) {
    backend.remap_palette(PresentationRenderTarget::Front, FULL_SCREEN);
    backend.remap_palette(PresentationRenderTarget::Back, FULL_SCREEN);
    backend.fill_region(PresentationRenderTarget::Back, u8::MIN, CONTENT_CLEAR);
}

fn dispatch_and_pump<Backend: PresentationScreenBackend>(
    state: &mut PresentationScreenState,
    context: PresentationSceneContext<'_, Backend::SceneLink>,
    backend: &mut Backend,
) -> Result<PresentationScreenOutcome, Backend::Error> {
    loop {
        if state.current_scene_line.is_some() || state.scene_status.queued {
            state.scene_status =
                backend.dispatch_scene(&context, state.current_scene_line.as_deref())?;
            if state.scene_status.queued {
                if state.scene_status.frame_presented {
                    backend.draw_sequence_subtitle(&mut state.subtitle_playback);
                    backend.draw_choice_number(state.selected_choice);
                }
                return Ok(PresentationScreenOutcome::WaitingForScene);
            }
        }

        let Some(line) = state.scene_lines.get(state.next_scene_line).cloned() else {
            state.current_scene_line = None;
            state.screen_rebuild_pending = true;
            state.phase = PresentationPanelPhase::Transition(PresentationTransitionFrame::One);
            return Ok(PresentationScreenOutcome::SceneLinesCompleted);
        };
        state.next_scene_line += 1;
        state.current_scene_line = Some(line);
    }
}

fn accept_input<Backend: PresentationScreenBackend>(
    state: &mut PresentationScreenState,
    backend: &mut Backend,
) -> PresentationScreenOutcome {
    if state.reverse {
        state.phase = PresentationPanelPhase::Closing(PresentationPanelStep::Six);
        if state.scene_status.queued {
            backend.cancel_scene_presentation();
            state.scene_status.queued = false;
        }
    } else {
        state.choice_change_animation_requested = true;
        backend.prepare_choice_audio();
        backend.play_presentation_clip();
        backend.cancel_scene_presentation();
        state.scene_status.queued = false;
        state.selected_choice = next_choice(state.selected_choice);
        state.phase = PresentationPanelPhase::Transition(PresentationTransitionFrame::One);
    }
    backend.fill_region(PresentationRenderTarget::Back, u8::MIN, CONTENT_FRAME);
    PresentationScreenOutcome::InputAccepted
}

fn next_choice(choice: PresentationChoiceNumber) -> PresentationChoiceNumber {
    let next = (choice.index() + 1) % PresentationChoiceNumber::COUNT;
    PresentationChoiceNumber::from_index(next as u8).unwrap()
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 34;
    const QUEUED_SCENE_LINK: u16 = 17_185;

    #[derive(Deserialize)]
    struct ScreenOracle {
        name: String,
        active: u8,
        phase_before: u16,
        phase_after: u16,
        queue_after: u8,
        selected_before: u8,
        selected_after: u8,
        remaining_records: usize,
        action_taken: bool,
        calls: Vec<OracleCall>,
    }

    #[derive(Deserialize)]
    struct OracleCall {
        call: String,
        color: Option<u16>,
        x: Option<u16>,
        y: Option<u16>,
        width: Option<u16>,
        height: Option<u16>,
    }

    #[derive(Debug)]
    struct RecordedCall {
        name: &'static str,
        target: Option<PresentationRenderTarget>,
        color: Option<u8>,
        region: Option<PresentationRenderRegion>,
    }

    struct OracleBackend {
        calls: Vec<RecordedCall>,
        music: PresentationMusicChange,
        scene_lines: Vec<Box<[u8]>>,
        dispatch_results: VecDeque<PresentationSceneStatus>,
    }

    impl OracleBackend {
        fn record(&mut self, name: &'static str) {
            self.calls.push(RecordedCall {
                name,
                target: None,
                color: None,
                region: None,
            });
        }
    }

    impl PresentationScreenBackend for OracleBackend {
        type RecordName = Box<[u8]>;
        type SceneLink = u16;
        type Error = std::convert::Infallible;

        fn fill_region(
            &mut self,
            target: PresentationRenderTarget,
            color: u8,
            region: PresentationRenderRegion,
        ) {
            self.calls.push(RecordedCall {
                name: "framebuffer_rect_fill",
                target: Some(target),
                color: Some(color),
                region: Some(region),
            });
        }
        fn frame_region(
            &mut self,
            target: PresentationRenderTarget,
            color: u8,
            region: PresentationRenderRegion,
        ) {
            self.calls.push(RecordedCall {
                name: "composite_draw_a",
                target: Some(target),
                color: Some(color),
                region: Some(region),
            });
        }
        fn remap_palette(
            &mut self,
            target: PresentationRenderTarget,
            region: PresentationRenderRegion,
        ) {
            self.calls.push(RecordedCall {
                name: "framebuffer_rect_palette_remap",
                target: Some(target),
                color: None,
                region: Some(region),
            });
        }
        fn draw_noise(
            &mut self,
            target: PresentationRenderTarget,
            mode: u8,
            region: PresentationRenderRegion,
        ) {
            self.calls.push(RecordedCall {
                name: "framebuffer_noise_rect",
                target: Some(target),
                color: Some(mode),
                region: Some(region),
            });
        }
        fn transition_presentation_entity(&mut self) {
            self.record("entity_flag_state_transition");
        }
        fn play_presentation_clip(&mut self) {
            self.record("snd_play_clip");
        }
        fn load_descript(
            &mut self,
            _: &Self::RecordName,
        ) -> Result<PresentationDescriptPlan, Self::Error> {
            self.record("vm_c2_descript_lookup");
            Ok(PresentationDescriptPlan::new(
                self.music,
                self.scene_lines.clone(),
            ))
        }
        fn reload_descript_music(&mut self) -> Result<(), Self::Error> {
            self.record("snd_driver_call");
            self.record("snd_stream_source_load");
            Ok(())
        }
        fn start_music_stream(&mut self) {
            self.record("snd_stream_start");
        }
        fn dispatch_scene(
            &mut self,
            _: &PresentationSceneContext<'_, Self::SceneLink>,
            _: Option<&[u8]>,
        ) -> Result<PresentationSceneStatus, Self::Error> {
            self.record("dlg_line_id_scene_dispatch");
            Ok(self.dispatch_results.pop_front().unwrap_or_default())
        }
        fn draw_sequence_subtitle(&mut self, _: &mut SequenceSubtitlePlayback) {
            self.record("list_walk_f18");
        }
        fn draw_choice_number(&mut self, _: PresentationChoiceNumber) {
            self.record("selected_mask_overlay");
        }
        fn cancel_scene_presentation(&mut self) {
            self.record("presentation_update_1fb2");
        }
        fn prepare_choice_audio(&mut self) {
            self.record("snd_driver_call");
        }
        fn reset_ship_camera(&mut self) {
            self.record("ship_3d_hud_palette_snapshot_and_camera_reset");
        }
    }

    #[test]
    fn phase_machine_matches_every_original_semantic_vector() {
        let vectors: Vec<ScreenOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_79e5_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut state = oracle_state(&vector);
            let records = oracle_records(&vector.name, vector.selected_before);
            let mut backend = oracle_backend(&vector.name);
            let outcome =
                update_presentation_screen(&mut state, &records, &QUEUED_SCENE_LINK, &mut backend)
                    .unwrap();

            assert_eq!(
                state.phase(),
                decode_oracle_phase(vector.phase_after),
                "{}",
                vector.name
            );
            assert_eq!(
                state.selected_choice().index(),
                usize::from(vector.selected_after),
                "{}",
                vector.name
            );
            assert_eq!(
                u8::from(state.scene_status().queued),
                vector.queue_after & 1,
                "{}",
                vector.name
            );
            assert_eq!(
                state.remaining_scene_lines(),
                vector.remaining_records,
                "{}",
                vector.name
            );
            assert_eq!(
                outcome == PresentationScreenOutcome::InputAccepted,
                vector.action_taken,
                "{}",
                vector.name
            );
            assert_calls(&backend.calls, &vector.calls, &vector.name);
        }
    }

    #[test]
    fn active_panel_names_front_and_back_targets_without_buffer_swaps() {
        let mut state = PresentationScreenState::default();
        state.set_active(true);
        state.set_phase(PresentationPanelPhase::Active);
        let records = std::array::from_fn(|_| None);
        let mut backend = oracle_backend("steady_empty_record");

        update_presentation_screen(&mut state, &records, &QUEUED_SCENE_LINK, &mut backend).unwrap();

        let targets: Vec<_> = backend
            .calls
            .iter()
            .filter_map(|call| call.target)
            .collect();
        assert_eq!(
            targets,
            [
                PresentationRenderTarget::Front,
                PresentationRenderTarget::Back,
                PresentationRenderTarget::Back,
                PresentationRenderTarget::Front,
            ],
        );
    }

    #[test]
    fn lifecycle_state_replaces_unexplained_native_state_numbers() {
        let mut state = PresentationScreenState::default();
        state.set_active(true);
        state.set_redraw_requested(true);
        let records = std::array::from_fn(|_| None);
        let mut backend = oracle_backend("phase_zero_initializes");

        update_presentation_screen(&mut state, &records, &QUEUED_SCENE_LINK, &mut backend).unwrap();
        assert_eq!(state.text_origin(), PresentationTextOrigin::Opening);
        assert!(state.panel_hover_restore_requested());

        state.set_phase(PresentationPanelPhase::Finalizing);
        update_presentation_screen(&mut state, &records, &QUEUED_SCENE_LINK, &mut backend).unwrap();
        assert_eq!(state.text_origin(), PresentationTextOrigin::Normal);
        assert_eq!(
            state.resource_placement(),
            PresentationResourcePlacement::Hidden,
        );
        assert!(!state.redraw_requested());
        assert!(state.screen_rebuild_pending());
        assert!(state.completion_audio_pending());
    }

    fn oracle_state(vector: &ScreenOracle) -> PresentationScreenState {
        let mut state = PresentationScreenState::default();
        state.set_active(vector.active & 1 != u8::MIN);
        state.set_phase(decode_oracle_phase(vector.phase_before));
        state.set_selected_choice(
            PresentationChoiceNumber::from_index(vector.selected_before).unwrap(),
        );
        state.set_reverse(vector.name.contains("reverse"));
        state.set_primary_pressed(matches!(
            vector.name.as_str(),
            "steady_empty_cycles_selection"
                | "steady_empty_wraps_selection"
                | "steady_empty_reverse_mode_starts_close"
                | "queued_mouse_cycles_without_dispatch"
        ));
        state.set_scene_status(PresentationSceneStatus {
            queued: vector.name.starts_with("queued_"),
            frame_presented: false,
        });
        state
    }

    fn oracle_records(
        name: &str,
        selected: u8,
    ) -> [Option<Box<[u8]>>; PresentationChoiceNumber::COUNT] {
        let mut records = std::array::from_fn(|_| None);
        if name.starts_with("lookup_") {
            records[usize::from(selected)] = Some(Box::from(b"record".as_slice()));
        }
        records
    }

    fn oracle_backend(name: &str) -> OracleBackend {
        let music = if name == "lookup_music_reload_without_lines" {
            PresentationMusicChange::Reload
        } else {
            PresentationMusicChange::Retained
        };
        let scene_lines = if matches!(
            name,
            "lookup_pumps_two_records" | "lookup_pauses_after_first_record"
        ) {
            vec![
                Box::from(b"FIRST-LINE".as_slice()),
                Box::from(b"SECOND".as_slice()),
            ]
        } else {
            Vec::new()
        };
        let dispatch_results = match name {
            "queued_scene_waits_without_frame" => VecDeque::from([PresentationSceneStatus {
                queued: true,
                frame_presented: false,
            }]),
            "queued_scene_draws_text_and_mask" | "lookup_pauses_after_first_record" => {
                VecDeque::from([PresentationSceneStatus {
                    queued: true,
                    frame_presented: true,
                }])
            }
            _ => VecDeque::new(),
        };
        OracleBackend {
            calls: Vec::new(),
            music,
            scene_lines,
            dispatch_results,
        }
    }

    fn assert_calls(actual: &[RecordedCall], expected: &[OracleCall], name: &str) {
        assert_eq!(actual.len(), expected.len(), "{name}");
        for (actual, expected) in actual.iter().zip(expected) {
            assert_eq!(actual.name, expected.call, "{name}");
            if let Some(color) = expected.color {
                assert_eq!(actual.color.map(u16::from), Some(color), "{name}");
            }
            if let Some(x) = expected.x {
                assert_eq!(
                    actual.region,
                    Some(PresentationRenderRegion::new(
                        [x, expected.y.unwrap()],
                        [expected.width.unwrap(), expected.height.unwrap()],
                    )),
                    "{name}",
                );
            }
        }
    }

    fn decode_oracle_phase(phase: u16) -> PresentationPanelPhase {
        match phase {
            0 => PresentationPanelPhase::Begin,
            1 => PresentationPanelPhase::Opening(PresentationPanelStep::One),
            2 => PresentationPanelPhase::Opening(PresentationPanelStep::Two),
            3 => PresentationPanelPhase::Opening(PresentationPanelStep::Three),
            4 => PresentationPanelPhase::Opening(PresentationPanelStep::Four),
            5 => PresentationPanelPhase::Opening(PresentationPanelStep::Five),
            6 => PresentationPanelPhase::Opening(PresentationPanelStep::Six),
            7 => PresentationPanelPhase::Transition(PresentationTransitionFrame::One),
            8 => PresentationPanelPhase::Transition(PresentationTransitionFrame::Two),
            9 => PresentationPanelPhase::Transition(PresentationTransitionFrame::Three),
            10 | 17_185 => PresentationPanelPhase::Active,
            100 => PresentationPanelPhase::Finalizing,
            101 => PresentationPanelPhase::Closing(PresentationPanelStep::One),
            102 => PresentationPanelPhase::Closing(PresentationPanelStep::Two),
            103 => PresentationPanelPhase::Closing(PresentationPanelStep::Three),
            104 => PresentationPanelPhase::Closing(PresentationPanelStep::Four),
            105 => PresentationPanelPhase::Closing(PresentationPanelStep::Five),
            106 => PresentationPanelPhase::Closing(PresentationPanelStep::Six),
            _ => panic!("unknown oracle phase {phase}"),
        }
    }
}
