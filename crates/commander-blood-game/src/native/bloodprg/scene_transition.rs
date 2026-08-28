//! Scene-transition coordination over typed presentation and palette state.

use std::error::Error;
use std::fmt;

use commander_blood_formats::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};

use super::{
    BridgeSpriteEntity, BridgeSpriteEntityError, IndexedGamePalette, Manu3AnimationSelector,
    ScriptPresentationScanState, TextPresentationState, advance_bridge_sprite_state,
};

const DIALOGUE_OVERLAY_ENTITY_INDEX: usize = 4;
const WORLD_ART_ENTITY_INDEX: usize = 31;
const SCENE_IMAGE_FIRST_ROW: u16 = 35;
const SCENE_IMAGE_LAST_ROW: u16 = 165;
const SCENE_IMAGE_CLEAR_COLOR: u8 = 0;
const SCENE_PALETTE_FIRST_COLOR: usize = 128;
const SCENE_PALETTE_COLOR_COUNT: usize = 64;
const SCENE_PALETTE_LAST_COLOR: usize = SCENE_PALETTE_FIRST_COLOR + SCENE_PALETTE_COLOR_COUNT - 1;
const SCENE_PALETTE_DARKEN_AMOUNT: u8 = 40;
const SCENE_PALETTE_TRANSITION_INCREMENT: u8 = 5;
const ALIEN_OVERLAY_ENTRY_LINE: u16 = 7;
const PALETTE_FADE_IN_LINE: u16 = 39;
const PALETTE_FADE_OUT_LINE: u16 = 40;
const DESCRIPTION_LOOKUP_LINE: u16 = 41;
const COMPLETE_LINE: u16 = 42;
const NON_PRESENTATION_LOADED_LINE: u16 = 43;

/// Priority phase of the scene transition after replacing native flag aliases.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SceneTransitionPhase {
    /// No transition owns the bridge scene.
    #[default]
    Inactive,
    /// Initialize entity, record, and description state.
    Initialize,
    /// Load and present the transition image.
    LoadImage,
    /// Wait to arm the deferred actor record.
    ArmDeferredRecord,
    /// Coordinate bridge steering and an optional alien overlay.
    Bridge,
    /// Wait for the restored palette presentation to finish.
    Finish,
    /// Release presentation state and return control to the bridge.
    Cleanup,
}

/// Semantic source of the record used by the current scene transition.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SceneTransitionRecordSource {
    /// The record already selected by the surrounding action coordinator.
    #[default]
    Current,
    /// The deferred navigation record adopted during initialization.
    Deferred,
}

/// Record family that selects the two image-load branches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneTransitionRecordKind {
    /// Native kind two, which performs palette transitions and alien coordination.
    Presentation,
    /// Any other record kind, which clears the center band and exits directly.
    Other,
}

/// Authored presentation line selected by the scene state machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneTransitionLine {
    /// Line seven requests a scene-image reload after bridge steering.
    AlienOverlayEntry,
    /// Line 39 advances the dark-to-live upper palette transition.
    PaletteFadeIn,
    /// Line 40 restores the pre-overlay upper palette.
    PaletteFadeOut,
    /// Line 41 resolves the deferred scene description.
    DescriptionLookup,
    /// Line 42 completes the scene transition.
    Complete,
    /// Line 43 follows a non-presentation image load.
    NonPresentationLoaded,
    /// A line owned by another coordinator and preserved without reinterpretation.
    Other(u16),
}

impl SceneTransitionLine {
    /// Decode one authored presentation line without discarding unknown values.
    pub const fn from_number(number: u16) -> Self {
        match number {
            ALIEN_OVERLAY_ENTRY_LINE => Self::AlienOverlayEntry,
            PALETTE_FADE_IN_LINE => Self::PaletteFadeIn,
            PALETTE_FADE_OUT_LINE => Self::PaletteFadeOut,
            DESCRIPTION_LOOKUP_LINE => Self::DescriptionLookup,
            COMPLETE_LINE => Self::Complete,
            NON_PRESENTATION_LOADED_LINE => Self::NonPresentationLoaded,
            other => Self::Other(other),
        }
    }

    /// Return the original authored presentation-line number.
    pub const fn number(self) -> u16 {
        match self {
            Self::AlienOverlayEntry => ALIEN_OVERLAY_ENTRY_LINE,
            Self::PaletteFadeIn => PALETTE_FADE_IN_LINE,
            Self::PaletteFadeOut => PALETTE_FADE_OUT_LINE,
            Self::DescriptionLookup => DESCRIPTION_LOOKUP_LINE,
            Self::Complete => COMPLETE_LINE,
            Self::NonPresentationLoaded => NON_PRESENTATION_LOADED_LINE,
            Self::Other(number) => number,
        }
    }
}

/// Inclusive logical rows cleared on the non-presentation image path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneImageBand {
    /// First logical row.
    pub first_row: u16,
    /// Last logical row.
    pub last_row: u16,
    /// Indexed fill color.
    pub color: u8,
}

/// Decoder options applied to one transition-image load.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneImageLoadOptions {
    /// Publish the decoded PBM palette.
    pub refresh_palette: bool,
    /// Preserve existing pixels where the source index is zero.
    pub transparent_zero: bool,
}

/// Typed upper-palette transition parameters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScenePaletteTransition {
    /// First indexed color included in the transition.
    pub first_color: usize,
    /// Last indexed color included in the transition.
    pub last_color: usize,
    /// Percentage added by each palette step.
    pub increment: u8,
    /// Current transition percentage.
    pub percent: u8,
}

/// Live, source, and target palettes owned by the transition pipeline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneTransitionPalettes {
    /// Palette currently used by indexed artwork.
    pub live: IndexedGamePalette,
    /// Source palette consumed by the interpolation step.
    pub source: IndexedGamePalette,
    /// Target palette consumed by the interpolation step.
    pub target: IndexedGamePalette,
    /// Current interpolation range and progress.
    pub transition: ScenePaletteTransition,
}

impl Default for SceneTransitionPalettes {
    fn default() -> Self {
        Self {
            live: [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT],
            source: [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT],
            target: [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT],
            transition: ScenePaletteTransition::default(),
        }
    }
}

/// Mutable state retained between scene-transition frames.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SceneTransitionState {
    /// Current semantic phase.
    pub phase: SceneTransitionPhase,
    /// Record selected by the phase machine.
    pub record_source: SceneTransitionRecordSource,
    /// Sprite clipping must be snapshotted before the next bridge frame.
    pub clip_snapshot_ready: bool,
    /// Callback-written bridge block observed after steering.
    pub bridge_blocked: bool,
    /// Callback- or line-written scene-image reload request.
    pub bridge_reload_requested: bool,
    /// Current authored scene presentation line.
    pub active_line: Option<SceneTransitionLine>,
    /// Whether scene-image presentation currently owns the content region.
    pub scene_gate_active: bool,
    /// Whether ordinary bridge UI input is enabled.
    pub ui_enabled: bool,
    /// Vertical origin used by the current scene resource.
    pub image_vertical_offset: u16,
    /// PBM palette publication option retained between image loads.
    pub pbm_palette_refresh: bool,
    /// PBM transparent-zero option retained between image loads.
    pub pbm_transparent_zero: bool,
    /// Exact MANU3 request published by image, deferred, and cleanup phases.
    pub manu3_animation: Option<Manu3AnimationSelector>,
    /// Native C4 deferred-record kind has been armed.
    pub deferred_actor_record_armed: bool,
    /// Bridge artwork must redraw after cleanup.
    pub redraw_pending: bool,
}

impl SceneTransitionState {
    /// Arm a fresh scene transition.
    pub fn begin(&mut self) {
        self.phase = SceneTransitionPhase::Initialize;
        self.record_source = SceneTransitionRecordSource::Current;
        self.bridge_blocked = false;
        self.bridge_reload_requested = false;
    }
}

/// Observable path taken by one transition step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneTransitionOutcome {
    /// No scene transition was active.
    Inactive,
    /// Deferred record and description state were initialized.
    Initialized,
    /// A presentation or C2 gate retained the current phase.
    Waiting,
    /// The scene image and its palette branch were prepared.
    ImageLoaded,
    /// A deferred actor record was armed.
    DeferredRecordArmed,
    /// The bridge path requested or completed an image reload.
    ImageReloaded,
    /// Bridge or alien work ran without advancing to cleanup.
    BridgeActive,
    /// Palette restoration advanced to the finish phase.
    PaletteRestoreStarted,
    /// Finish advanced to cleanup.
    Finishing,
    /// Presentation ownership was released to the bridge.
    CleanedUp,
}

/// Typed scene, resource, bridge, and HUD boundaries called by the coordinator.
pub trait SceneTransitionHost {
    /// Typed scene-link value consumed by the presentation dispatcher.
    type SceneLink;
    /// Host failure propagated without fallback behavior.
    type Error;

    /// Return the current record family after any preceding callback mutation.
    fn scene_record_kind(&self, source: SceneTransitionRecordSource) -> SceneTransitionRecordKind;

    /// Resolve assets for the deferred scene record.
    fn lookup_scene_description(
        &mut self,
        source: SceneTransitionRecordSource,
        presentation_interface_active: bool,
        text: &mut TextPresentationState,
    ) -> Result<(), Self::Error>;

    /// Dispatch the current authored scene line before phase-specific work.
    fn dispatch_scene_line(
        &mut self,
        link: &Self::SceneLink,
        state: &mut SceneTransitionState,
        presentation: &mut ScriptPresentationScanState,
    ) -> Result<(), Self::Error>;

    /// Decode `frigo.fd` and update its live palette when requested.
    fn load_scene_image(
        &mut self,
        options: SceneImageLoadOptions,
        live_palette: &mut IndexedGamePalette,
    ) -> Result<(), Self::Error>;

    /// Present the newly decoded scene image.
    fn present_scene_image(&mut self) -> Result<(), Self::Error>;

    /// Clear the authored center band in the retained back buffer.
    fn clear_scene_image_band(&mut self, band: SceneImageBand) -> Result<(), Self::Error>;

    /// Advance bridge steering, with typed access to callback-written phase state.
    fn update_bridge(
        &mut self,
        state: &mut SceneTransitionState,
        presentation: &mut ScriptPresentationScanState,
    ) -> Result<(), Self::Error>;

    /// Advance the currently selected alien overlay.
    fn run_alien_overlay(
        &mut self,
        presentation: &mut ScriptPresentationScanState,
    ) -> Result<(), Self::Error>;

    /// Draw and snapshot the 3D HUD palette, then reset the camera.
    fn initialize_ship_hud(
        &mut self,
        live_palette: &mut IndexedGamePalette,
    ) -> Result<(), Self::Error>;
}

/// Invalid flat entity state or failed host work during scene coordination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneTransitionError<HostError> {
    /// One of the two fixed bridge entities was absent.
    Entity(BridgeSpriteEntityError),
    /// A resource, renderer, bridge, alien, or HUD callback failed.
    Host(HostError),
}

impl<HostError: fmt::Debug> fmt::Display for SceneTransitionError<HostError> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "scene transition failed: {self:?}")
    }
}

impl<HostError: fmt::Debug> Error for SceneTransitionError<HostError> {}

/// Advance `scene_transition_step` at BLOODPRG file offset `0x001855`.
///
/// Native phase-bit priority, callback rereads, image options, palette copies,
/// and cleanup ordering are retained. Typed entities, records, palettes, and
/// subsystem callbacks replace fixed offsets, segmented pointers, and VGA
/// framebuffer ownership.
pub fn update_scene_transition<Host: SceneTransitionHost>(
    state: &mut SceneTransitionState,
    presentation: &mut ScriptPresentationScanState,
    text: &mut TextPresentationState,
    palettes: &mut SceneTransitionPalettes,
    entities: &mut [BridgeSpriteEntity],
    scene_link: &Host::SceneLink,
    host: &mut Host,
) -> Result<SceneTransitionOutcome, SceneTransitionError<Host::Error>> {
    let phase = state.phase;
    if phase == SceneTransitionPhase::Inactive {
        return Ok(SceneTransitionOutcome::Inactive);
    }

    state.clip_snapshot_ready = true;
    if phase == SceneTransitionPhase::Initialize {
        validate_transition_entities(entities)?;
        advance_bridge_sprite_state(entities, DIALOGUE_OVERLAY_ENTITY_INDEX)
            .map_err(SceneTransitionError::Entity)?;
        advance_bridge_sprite_state(entities, WORLD_ART_ENTITY_INDEX)
            .map_err(SceneTransitionError::Entity)?;
        state.ui_enabled = false;
        state.phase = SceneTransitionPhase::LoadImage;
        state.record_source = SceneTransitionRecordSource::Deferred;
        state.active_line = Some(SceneTransitionLine::DescriptionLookup);
        host.lookup_scene_description(state.record_source, state.ui_enabled, text)
            .map_err(SceneTransitionError::Host)?;
        return Ok(SceneTransitionOutcome::Initialized);
    }

    host.dispatch_scene_line(scene_link, state, presentation)
        .map_err(SceneTransitionError::Host)?;

    match phase {
        SceneTransitionPhase::LoadImage => {
            if presentation.c2_gate_active {
                return Ok(SceneTransitionOutcome::Waiting);
            }
            load_transition_image(state, palettes, host)?;
            if host.scene_record_kind(state.record_source) == SceneTransitionRecordKind::Other {
                state.manu3_animation = Some(Manu3AnimationSelector::Disabled);
                host.clear_scene_image_band(SceneImageBand {
                    first_row: SCENE_IMAGE_FIRST_ROW,
                    last_row: SCENE_IMAGE_LAST_ROW,
                    color: SCENE_IMAGE_CLEAR_COLOR,
                })
                .map_err(SceneTransitionError::Host)?;
                state.phase = SceneTransitionPhase::Bridge;
                state.active_line = Some(SceneTransitionLine::NonPresentationLoaded);
            } else {
                prepare_dark_palette_transition(palettes);
                state.phase = SceneTransitionPhase::ArmDeferredRecord;
                state.active_line = Some(SceneTransitionLine::PaletteFadeIn);
            }
            Ok(SceneTransitionOutcome::ImageLoaded)
        }
        SceneTransitionPhase::ArmDeferredRecord => {
            if presentation.c2_gate_active {
                return Ok(SceneTransitionOutcome::Waiting);
            }
            state.deferred_actor_record_armed = true;
            state.phase = SceneTransitionPhase::Bridge;
            state.bridge_blocked = true;
            state.bridge_reload_requested = false;
            state.manu3_animation = Some(Manu3AnimationSelector::Neutral);
            Ok(SceneTransitionOutcome::DeferredRecordArmed)
        }
        SceneTransitionPhase::Bridge => {
            host.update_bridge(state, presentation)
                .map_err(SceneTransitionError::Host)?;
            if host.scene_record_kind(state.record_source) == SceneTransitionRecordKind::Other {
                if presentation.c2_gate_active {
                    return Ok(SceneTransitionOutcome::Waiting);
                }
                begin_cleanup(state);
                return Ok(SceneTransitionOutcome::Finishing);
            }
            if state.bridge_blocked {
                return Ok(SceneTransitionOutcome::BridgeActive);
            }
            if state.active_line == Some(SceneTransitionLine::AlienOverlayEntry) {
                state.bridge_reload_requested = true;
                return Ok(SceneTransitionOutcome::BridgeActive);
            }
            if state.bridge_reload_requested {
                state.bridge_reload_requested = false;
                state.pbm_palette_refresh = false;
                let options = scene_image_options(state);
                host.load_scene_image(options, &mut palettes.live)
                    .map_err(SceneTransitionError::Host)?;
                return Ok(SceneTransitionOutcome::ImageReloaded);
            }

            host.run_alien_overlay(presentation)
                .map_err(SceneTransitionError::Host)?;
            if presentation.active || presentation.c2_gate_active {
                return Ok(SceneTransitionOutcome::BridgeActive);
            }
            prepare_live_palette_restore(palettes);
            state.phase = SceneTransitionPhase::Finish;
            state.bridge_blocked = false;
            state.bridge_reload_requested = false;
            state.active_line = Some(SceneTransitionLine::PaletteFadeOut);
            Ok(SceneTransitionOutcome::PaletteRestoreStarted)
        }
        SceneTransitionPhase::Finish => {
            if presentation.c2_gate_active {
                return Ok(SceneTransitionOutcome::Waiting);
            }
            begin_cleanup(state);
            Ok(SceneTransitionOutcome::Finishing)
        }
        SceneTransitionPhase::Cleanup => {
            if presentation.c2_gate_active {
                return Ok(SceneTransitionOutcome::Waiting);
            }
            state.manu3_animation = Some(Manu3AnimationSelector::Neutral);
            state.phase = SceneTransitionPhase::Inactive;
            state.bridge_blocked = false;
            state.bridge_reload_requested = false;
            state.ui_enabled = true;
            state.active_line = None;
            presentation.c2_gate_active = false;
            presentation.ui_busy = false;
            presentation.start_locked = false;
            presentation.hold_ready = false;
            presentation.text_wait_active = false;
            text.selected_line = None;
            text.subtitle_display_active = false;
            text.request_flags.clear_primary_requests();
            state.redraw_pending = true;
            host.initialize_ship_hud(&mut palettes.live)
                .map_err(SceneTransitionError::Host)?;
            Ok(SceneTransitionOutcome::CleanedUp)
        }
        SceneTransitionPhase::Inactive | SceneTransitionPhase::Initialize => {
            unreachable!("inactive and initialization phases returned before dispatch")
        }
    }
}

fn validate_transition_entities<HostError>(
    entities: &[BridgeSpriteEntity],
) -> Result<(), SceneTransitionError<HostError>> {
    if entities.get(WORLD_ART_ENTITY_INDEX).is_none() {
        return Err(SceneTransitionError::Entity(BridgeSpriteEntityError {
            entity_index: WORLD_ART_ENTITY_INDEX,
            entity_count: entities.len(),
        }));
    }
    Ok(())
}

fn load_transition_image<Host: SceneTransitionHost>(
    state: &mut SceneTransitionState,
    palettes: &mut SceneTransitionPalettes,
    host: &mut Host,
) -> Result<(), SceneTransitionError<Host::Error>> {
    state.phase = SceneTransitionPhase::ArmDeferredRecord;
    state.image_vertical_offset = SCENE_IMAGE_FIRST_ROW;
    state.scene_gate_active = true;
    state.pbm_palette_refresh = true;
    state.pbm_transparent_zero = false;
    let options = scene_image_options(state);
    host.load_scene_image(options, &mut palettes.live)
        .map_err(SceneTransitionError::Host)?;
    host.present_scene_image()
        .map_err(SceneTransitionError::Host)
}

fn scene_image_options(state: &SceneTransitionState) -> SceneImageLoadOptions {
    SceneImageLoadOptions {
        refresh_palette: state.pbm_palette_refresh,
        transparent_zero: state.pbm_transparent_zero,
    }
}

fn prepare_dark_palette_transition(palettes: &mut SceneTransitionPalettes) {
    let end = SCENE_PALETTE_FIRST_COLOR + SCENE_PALETTE_COLOR_COUNT;
    palettes.target[SCENE_PALETTE_FIRST_COLOR..end]
        .copy_from_slice(&palettes.live[SCENE_PALETTE_FIRST_COLOR..end]);
    for color_index in SCENE_PALETTE_FIRST_COLOR..end {
        palettes.source[color_index] = palettes.live[color_index]
            .map(|component| component.saturating_sub(SCENE_PALETTE_DARKEN_AMOUNT));
    }
    palettes.transition.first_color = SCENE_PALETTE_FIRST_COLOR;
    palettes.transition.last_color = SCENE_PALETTE_LAST_COLOR;
    palettes.transition.increment = SCENE_PALETTE_TRANSITION_INCREMENT;
}

fn prepare_live_palette_restore(palettes: &mut SceneTransitionPalettes) {
    let end = SCENE_PALETTE_FIRST_COLOR + SCENE_PALETTE_COLOR_COUNT;
    palettes.source[SCENE_PALETTE_FIRST_COLOR..end]
        .copy_from_slice(&palettes.target[SCENE_PALETTE_FIRST_COLOR..end]);
    palettes.target[SCENE_PALETTE_FIRST_COLOR..end]
        .copy_from_slice(&palettes.live[SCENE_PALETTE_FIRST_COLOR..end]);
    palettes.transition.percent = u8::MIN;
}

fn begin_cleanup(state: &mut SceneTransitionState) {
    state.image_vertical_offset = u16::MIN;
    state.phase = SceneTransitionPhase::Cleanup;
    state.bridge_blocked = false;
    state.bridge_reload_requested = false;
    state.active_line = Some(SceneTransitionLine::Complete);
    state.scene_gate_active = false;
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use serde::Deserialize;
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::native::bloodprg::{BRIDGE_SPRITE_ENTITY_COUNT, BridgeSpriteFlags};

    const ORACLE_VECTOR_COUNT: usize = 21;
    const ORIGINAL_ACTIVE_FLAG: u16 = 128;
    const ORIGINAL_STATE_ZERO_FLAG: u16 = 1;
    const ORIGINAL_DIRTY_FLAG: u16 = 2;
    const ORIGINAL_NO_LINE: u16 = u16::MAX;
    const ORIGINAL_LINE_ALIEN_OVERLAY_ENTRY: u16 = 7;
    const ORIGINAL_LINE_PALETTE_FADE_IN: u16 = 39;
    const ORIGINAL_LINE_PALETTE_FADE_OUT: u16 = 40;
    const ORIGINAL_LINE_DESCRIPTION_LOOKUP: u16 = 41;
    const ORIGINAL_LINE_COMPLETE: u16 = 42;
    const ORIGINAL_LINE_NON_PRESENTATION: u16 = 43;
    const ORIGINAL_ACTIVE_PHASE: u8 = 0x01;
    const ORIGINAL_LOAD_PHASE: u8 = 0x02;
    const ORIGINAL_DEFERRED_PHASE: u8 = 0x04;
    const ORIGINAL_BRIDGE_PHASE: u8 = 0x08;
    const ORIGINAL_FINISH_PHASE: u8 = 0x10;
    const ORIGINAL_CLEANUP_PHASE: u8 = 0x20;
    const ORIGINAL_RELOAD_PHASE: u8 = 0x40;
    const ORIGINAL_BLOCKED_PHASE: u8 = 0x80;
    const ORIGINAL_PRESENTATION_RECORD_KIND: u16 = 2;
    const ORIGINAL_GAME_BYTE_SEED: usize = 0x41;
    const ORIGINAL_GAME_BYTE_STEP: usize = 37;
    const ORIGINAL_GAME_PAGE_STEP: usize = 13;
    const ORIGINAL_CASE_STEP: usize = 29;
    const ORIGINAL_SCENE_GATE_OFFSET: usize = 0x274F;
    const ORIGINAL_PALETTE_REFRESH_OFFSET: usize = 0x5B53;
    const ORIGINAL_PRESENTATION_ACTIVE_OFFSET: usize = 0x67AC;
    const ORIGINAL_TRANSPARENT_ZERO: u8 = 0xA6;
    const INITIAL_OTHER_LINE: u16 = 51;
    const TEXT_REQUEST_TEST_BITS: u8 = 0xA7;

    #[derive(Deserialize)]
    struct OracleCall {
        callee: String,
        palette_refresh: Option<u8>,
        transparent_zero: Option<u8>,
        top: Option<u16>,
        bottom: Option<u16>,
    }

    #[derive(Deserialize)]
    struct SceneTransitionOracle {
        name: String,
        phase_before: u8,
        record_kind: u16,
        c2_gate_before: u8,
        active_line_before: u16,
        calls: Vec<OracleCall>,
        phase_after: u8,
        clip_snapshot_flags: u16,
        scene_record_offset: u16,
        active_line_after: u16,
        scene_gate_after: u8,
        c2_gate_after: u8,
        palette_refresh_after: u8,
        transparent_zero_after: u8,
        target_high_sha256: String,
        source_high_sha256: String,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum RecordedCall {
        Lookup,
        Dispatch,
        Load(SceneImageLoadOptions),
        Present,
        Clear(SceneImageBand),
        Bridge,
        Alien,
        Hud,
    }

    struct OracleHost {
        record_kind: SceneTransitionRecordKind,
        calls: Vec<RecordedCall>,
        write_blocked: bool,
        write_reload: bool,
        alien_active: bool,
        alien_c2_gate: bool,
    }

    impl SceneTransitionHost for OracleHost {
        type SceneLink = u16;
        type Error = Infallible;

        fn scene_record_kind(
            &self,
            _source: SceneTransitionRecordSource,
        ) -> SceneTransitionRecordKind {
            self.record_kind
        }

        fn lookup_scene_description(
            &mut self,
            _source: SceneTransitionRecordSource,
            _presentation_interface_active: bool,
            _text: &mut TextPresentationState,
        ) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::Lookup);
            Ok(())
        }

        fn dispatch_scene_line(
            &mut self,
            _link: &Self::SceneLink,
            _state: &mut SceneTransitionState,
            _presentation: &mut ScriptPresentationScanState,
        ) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::Dispatch);
            Ok(())
        }

        fn load_scene_image(
            &mut self,
            options: SceneImageLoadOptions,
            _live_palette: &mut IndexedGamePalette,
        ) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::Load(options));
            Ok(())
        }

        fn present_scene_image(&mut self) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::Present);
            Ok(())
        }

        fn clear_scene_image_band(&mut self, band: SceneImageBand) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::Clear(band));
            Ok(())
        }

        fn update_bridge(
            &mut self,
            state: &mut SceneTransitionState,
            _presentation: &mut ScriptPresentationScanState,
        ) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::Bridge);
            if self.write_blocked {
                state.bridge_blocked = true;
                state.bridge_reload_requested = false;
            }
            if self.write_reload {
                state.bridge_blocked = false;
                state.bridge_reload_requested = true;
            }
            Ok(())
        }

        fn run_alien_overlay(
            &mut self,
            presentation: &mut ScriptPresentationScanState,
        ) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::Alien);
            if self.alien_active {
                presentation.active = true;
            }
            if self.alien_c2_gate {
                presentation.c2_gate_active = true;
            }
            Ok(())
        }

        fn initialize_ship_hud(
            &mut self,
            _live_palette: &mut IndexedGamePalette,
        ) -> Result<(), Self::Error> {
            self.calls.push(RecordedCall::Hud);
            Ok(())
        }
    }

    #[test]
    fn coordinator_matches_every_original_scene_transition_vector() {
        let vectors: Vec<SceneTransitionOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_1855_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for (case_index, vector) in vectors.into_iter().enumerate() {
            let mut state = initial_state(&vector, case_index);
            let mut presentation = initial_presentation(&vector, case_index);
            let mut text = TextPresentationState {
                selected_line: Some(17),
                subtitle_display_active: true,
                request_flags: super::super::PresentationRequestFlags::decode(
                    TEXT_REQUEST_TEST_BITS,
                ),
                ..TextPresentationState::default()
            };
            let mut palettes = initial_palettes(case_index);
            let mut entities = [BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT];
            let initial_entity_flags =
                BridgeSpriteFlags::from_bits(ORIGINAL_ACTIVE_FLAG | ORIGINAL_STATE_ZERO_FLAG);
            entities[DIALOGUE_OVERLAY_ENTITY_INDEX].flags = initial_entity_flags;
            entities[WORLD_ART_ENTITY_INDEX].flags = initial_entity_flags;
            let mut host = OracleHost {
                record_kind: if vector.record_kind == ORIGINAL_PRESENTATION_RECORD_KIND {
                    SceneTransitionRecordKind::Presentation
                } else {
                    SceneTransitionRecordKind::Other
                },
                calls: Vec::new(),
                write_blocked: vector.name == "bridge_callback_sets_blocked",
                write_reload: vector.name == "bridge_callback_sets_reload",
                alien_active: vector.name == "bridge_alien_remains_active",
                alien_c2_gate: vector.name == "bridge_alien_sets_c2_gate",
            };

            update_scene_transition(
                &mut state,
                &mut presentation,
                &mut text,
                &mut palettes,
                &mut entities,
                &0x7E00,
                &mut host,
            )
            .unwrap();

            assert_eq!(host.calls, expected_calls(&vector.calls), "{}", vector.name);
            assert_eq!(
                state.phase,
                phase_from_original(vector.phase_after),
                "{}",
                vector.name
            );
            assert_eq!(
                state.bridge_blocked,
                vector.phase_after & ORIGINAL_BLOCKED_PHASE != 0,
                "{}",
                vector.name
            );
            assert_eq!(
                state.bridge_reload_requested,
                vector.phase_after & ORIGINAL_RELOAD_PHASE != 0,
                "{}",
                vector.name
            );
            assert_eq!(
                state.clip_snapshot_ready,
                vector.clip_snapshot_flags == 1,
                "{}",
                vector.name
            );
            assert_eq!(
                state.record_source,
                if vector.scene_record_offset == 0x0380 {
                    SceneTransitionRecordSource::Deferred
                } else {
                    SceneTransitionRecordSource::Current
                },
                "{}",
                vector.name
            );
            assert_eq!(
                line_number(state.active_line),
                vector.active_line_after,
                "{}",
                vector.name
            );
            assert_eq!(
                state.scene_gate_active,
                vector.scene_gate_after & 1 != 0,
                "{}",
                vector.name
            );
            assert_eq!(
                presentation.c2_gate_active,
                vector.c2_gate_after & 1 != 0,
                "{}",
                vector.name
            );
            assert_eq!(
                state.pbm_palette_refresh,
                vector.palette_refresh_after & 1 != 0,
                "{}",
                vector.name
            );
            assert_eq!(
                state.pbm_transparent_zero,
                vector.transparent_zero_after & 1 != 0,
                "{}",
                vector.name
            );
            assert_eq!(
                palette_hash(&palettes.target),
                vector.target_high_sha256,
                "{}",
                vector.name
            );
            assert_eq!(
                palette_hash(&palettes.source),
                vector.source_high_sha256,
                "{}",
                vector.name
            );
            if vector.name == "load_nonpresentation" {
                assert_eq!(
                    state.manu3_animation,
                    Some(Manu3AnimationSelector::Disabled)
                );
            }

            let expected_entity_flags = if vector.name == "active_initializes_record" {
                ORIGINAL_ACTIVE_FLAG | ORIGINAL_DIRTY_FLAG
            } else {
                ORIGINAL_ACTIVE_FLAG | ORIGINAL_STATE_ZERO_FLAG
            };
            assert_eq!(
                entities[DIALOGUE_OVERLAY_ENTITY_INDEX].flags.bits(),
                expected_entity_flags,
                "{}",
                vector.name
            );
            assert_eq!(
                entities[WORLD_ART_ENTITY_INDEX].flags.bits(),
                expected_entity_flags,
                "{}",
                vector.name
            );
            if vector.name == "cleanup_resets_presentation" {
                assert!(state.ui_enabled);
                assert!(state.redraw_pending);
                assert_eq!(state.manu3_animation, Some(Manu3AnimationSelector::Neutral));
                assert!(!presentation.ui_busy);
                assert!(!presentation.start_locked);
                assert!(!presentation.hold_ready);
                assert!(!presentation.text_wait_active);
                assert_eq!(text.selected_line, None);
                assert!(!text.subtitle_display_active);
                assert_eq!(text.request_flags.bits(), TEXT_REQUEST_TEST_BITS & !3);
            }
        }
    }

    #[test]
    fn short_entity_tables_fail_before_mutating_entity_four() {
        let mut state = SceneTransitionState::default();
        state.begin();
        let mut presentation = ScriptPresentationScanState::default();
        let mut text = TextPresentationState::default();
        let mut palettes = SceneTransitionPalettes::default();
        let mut entities = [BridgeSpriteEntity::default(); WORLD_ART_ENTITY_INDEX];
        entities[DIALOGUE_OVERLAY_ENTITY_INDEX].flags =
            BridgeSpriteFlags::from_bits(ORIGINAL_ACTIVE_FLAG | ORIGINAL_STATE_ZERO_FLAG);
        let before = entities;
        let mut host = OracleHost {
            record_kind: SceneTransitionRecordKind::Presentation,
            calls: Vec::new(),
            write_blocked: false,
            write_reload: false,
            alien_active: false,
            alien_c2_gate: false,
        };

        let error = update_scene_transition(
            &mut state,
            &mut presentation,
            &mut text,
            &mut palettes,
            &mut entities,
            &u16::MIN,
            &mut host,
        )
        .unwrap_err();

        assert!(matches!(error, SceneTransitionError::Entity(_)));
        assert_eq!(entities, before);
        assert!(host.calls.is_empty());
    }

    fn initial_state(vector: &SceneTransitionOracle, case_index: usize) -> SceneTransitionState {
        SceneTransitionState {
            phase: phase_from_original(vector.phase_before),
            record_source: SceneTransitionRecordSource::Current,
            clip_snapshot_ready: false,
            bridge_blocked: vector.phase_before & ORIGINAL_BLOCKED_PHASE != 0,
            bridge_reload_requested: vector.phase_before & ORIGINAL_RELOAD_PHASE != 0,
            active_line: line_from_number(vector.active_line_before),
            scene_gate_active: seeded_game_byte(ORIGINAL_SCENE_GATE_OFFSET, case_index) & 1 != 0,
            ui_enabled: true,
            image_vertical_offset: 0x7A7A,
            pbm_palette_refresh: seeded_game_byte(ORIGINAL_PALETTE_REFRESH_OFFSET, case_index) & 1
                != 0,
            pbm_transparent_zero: ORIGINAL_TRANSPARENT_ZERO & 1 != 0,
            manu3_animation: Some(Manu3AnimationSelector::Neutral),
            deferred_actor_record_armed: false,
            redraw_pending: false,
        }
    }

    fn initial_presentation(
        vector: &SceneTransitionOracle,
        case_index: usize,
    ) -> ScriptPresentationScanState {
        ScriptPresentationScanState {
            active: seeded_game_byte(ORIGINAL_PRESENTATION_ACTIVE_OFFSET, case_index) & 1 != 0,
            c2_gate_active: vector.c2_gate_before & 1 != 0,
            ui_busy: true,
            start_locked: true,
            hold_ready: true,
            text_wait_active: true,
            ..ScriptPresentationScanState::default()
        }
    }

    fn initial_palettes(case_index: usize) -> SceneTransitionPalettes {
        let mut palettes = SceneTransitionPalettes::default();
        for flat_index in 0..SCENE_PALETTE_COLOR_COUNT * 3 {
            let color = SCENE_PALETTE_FIRST_COLOR + flat_index / 3;
            let component = flat_index % 3;
            palettes.live[color][component] = (flat_index * 17 + case_index * 11 + 3) as u8 & 0x3F;
            palettes.target[color][component] = (flat_index * 23 + case_index * 7 + 0x91) as u8;
            palettes.source[color][component] = (flat_index * 31 + case_index * 5 + 0x53) as u8;
        }
        palettes
    }

    fn expected_calls(calls: &[OracleCall]) -> Vec<RecordedCall> {
        calls
            .iter()
            .filter_map(|call| match call.callee.as_str() {
                "entity_flag_state_transition" => None,
                "vm_c2_descript_lookup" => Some(RecordedCall::Lookup),
                "dlg_line_id_scene_dispatch" => Some(RecordedCall::Dispatch),
                "pbm_image_load_and_decode" => Some(RecordedCall::Load(SceneImageLoadOptions {
                    refresh_palette: call.palette_refresh.unwrap() & 1 != 0,
                    transparent_zero: call.transparent_zero.unwrap() & 1 != 0,
                })),
                "full_screen_blit" => Some(RecordedCall::Present),
                "back_buffer_fill" => Some(RecordedCall::Clear(SceneImageBand {
                    first_row: call.top.unwrap(),
                    last_row: call.bottom.unwrap(),
                    color: SCENE_IMAGE_CLEAR_COLOR,
                })),
                "bridge_steer_update" => Some(RecordedCall::Bridge),
                "alien_overlay_cycle" => Some(RecordedCall::Alien),
                "ship_3d_hud_palette_snapshot_and_camera_reset" => Some(RecordedCall::Hud),
                other => panic!("unknown scene transition oracle call {other}"),
            })
            .collect()
    }

    fn phase_from_original(phase: u8) -> SceneTransitionPhase {
        if phase & ORIGINAL_ACTIVE_PHASE == 0 {
            SceneTransitionPhase::Inactive
        } else if phase & ORIGINAL_LOAD_PHASE != 0 {
            SceneTransitionPhase::LoadImage
        } else if phase & ORIGINAL_DEFERRED_PHASE != 0 {
            SceneTransitionPhase::ArmDeferredRecord
        } else if phase & ORIGINAL_BRIDGE_PHASE != 0 {
            SceneTransitionPhase::Bridge
        } else if phase & ORIGINAL_FINISH_PHASE != 0 {
            SceneTransitionPhase::Finish
        } else if phase & ORIGINAL_CLEANUP_PHASE != 0 {
            SceneTransitionPhase::Cleanup
        } else {
            SceneTransitionPhase::Initialize
        }
    }

    fn line_from_number(line: u16) -> Option<SceneTransitionLine> {
        match line {
            ORIGINAL_NO_LINE => None,
            ORIGINAL_LINE_ALIEN_OVERLAY_ENTRY => Some(SceneTransitionLine::AlienOverlayEntry),
            ORIGINAL_LINE_PALETTE_FADE_IN => Some(SceneTransitionLine::PaletteFadeIn),
            ORIGINAL_LINE_PALETTE_FADE_OUT => Some(SceneTransitionLine::PaletteFadeOut),
            ORIGINAL_LINE_DESCRIPTION_LOOKUP => Some(SceneTransitionLine::DescriptionLookup),
            ORIGINAL_LINE_COMPLETE => Some(SceneTransitionLine::Complete),
            ORIGINAL_LINE_NON_PRESENTATION => Some(SceneTransitionLine::NonPresentationLoaded),
            other => Some(SceneTransitionLine::Other(other)),
        }
    }

    fn line_number(line: Option<SceneTransitionLine>) -> u16 {
        match line {
            None => ORIGINAL_NO_LINE,
            Some(SceneTransitionLine::AlienOverlayEntry) => ORIGINAL_LINE_ALIEN_OVERLAY_ENTRY,
            Some(SceneTransitionLine::PaletteFadeIn) => ORIGINAL_LINE_PALETTE_FADE_IN,
            Some(SceneTransitionLine::PaletteFadeOut) => ORIGINAL_LINE_PALETTE_FADE_OUT,
            Some(SceneTransitionLine::DescriptionLookup) => ORIGINAL_LINE_DESCRIPTION_LOOKUP,
            Some(SceneTransitionLine::Complete) => ORIGINAL_LINE_COMPLETE,
            Some(SceneTransitionLine::NonPresentationLoaded) => ORIGINAL_LINE_NON_PRESENTATION,
            Some(SceneTransitionLine::Other(other)) => other,
        }
    }

    fn seeded_game_byte(offset: usize, case_index: usize) -> u8 {
        (offset * ORIGINAL_GAME_BYTE_STEP
            + (offset >> 8) * ORIGINAL_GAME_PAGE_STEP
            + case_index * ORIGINAL_CASE_STEP
            + ORIGINAL_GAME_BYTE_SEED) as u8
    }

    fn palette_hash(palette: &IndexedGamePalette) -> String {
        let end = SCENE_PALETTE_FIRST_COLOR + SCENE_PALETTE_COLOR_COUNT;
        let mut hasher = Sha256::new();
        for color in &palette[SCENE_PALETTE_FIRST_COLOR..end] {
            hasher.update(color);
        }
        format!("{:x}", hasher.finalize())
    }

    #[test]
    fn oracle_fixture_keeps_the_unmapped_initial_line_explicit() {
        assert_eq!(
            line_from_number(INITIAL_OTHER_LINE),
            Some(SceneTransitionLine::Other(INITIAL_OTHER_LINE))
        );
    }
}
