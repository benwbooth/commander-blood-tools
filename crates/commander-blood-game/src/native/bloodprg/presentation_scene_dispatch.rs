//! Scene-image and streamed-presentation coordination for one authored line.

use std::error::Error;
use std::fmt;
use std::ops::Range;

use commander_blood_formats::lbm::RGB_COMPONENT_COUNT;

use super::{
    IndexedGamePalette, PresentationPresentPolicy, PresentationResourceId, PresentationUpdateState,
    ShipHudPaletteSnapshot,
};

const PRESENTATION_ACTIVE_FLAG: u8 = 1;
const PRESENTATION_REQUEST_FLAG: u8 = 2;
const SHIP_ACTIVE_PRESENTATION_FLAG: u16 = 8;
const MAX_SIGNED_PRESENTATION_LINE: u16 = i16::MAX as u16;
const SCRUTER_JO_PRESENTATION_LINE: u16 = 29;
const SHARED_CACHE_PRESENTATION_LINE: u16 = 8;
const PALETTE_TRANSITION_PRESENTATION_LINE: u16 = 39;
const DISPLAY_CLEAR_PRESENTATION_LINE: u16 = 5;
const DRAW_VIA_BACK_BUFFER_LINES: [u16; 2] = [2, 7];
const SKIP_BACK_BUFFER_PRESENT_LINES: [u16; 10] = [0, 1, 3, 4, 5, 6, 41, 42, 43, 44];
const UNCLAMPED_LINE_ID_COUNT: usize = 8;
const SCENE_IMAGE_ROW_COUNT: usize = 130;
const DISPLAY_CLEAR_FIRST_ROW: usize = 35;
const DISPLAY_CLEAR_LAST_ROW: usize = 165;
const BLACK_REMAP_PERCENT: u8 = 50;
const BLACK_REMAP_TARGET: [u8; RGB_COMPONENT_COUNT] = [u8::MIN; RGB_COMPONENT_COUNT];
const PALETTE_TRANSITION_ENTRY_DELTA: u16 = 20;
const SHIP_DEPTH_ENTRY_DELTA: u16 = 8;
const SHIP_DEPTH_OPENING_STEP: u8 = 6;

/// Scene artwork selected alongside one presentation resource.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationSceneDescriptor<ImageId> {
    /// Absent artwork selects the original cleared-band path.
    pub image: Option<ImageId>,
}

/// Backing source selected for the streamed presentation line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PresentationSceneSource {
    /// Independently opened resource bytes.
    #[default]
    Owned,
    /// Shared cached resource bytes used by presentation line eight.
    SharedCache,
}

/// Scene-image work completed before a presentation sequence starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationSceneImageOutcome {
    /// Scene ownership suppressed image lookup for this update.
    SkippedBySceneGate,
    /// The already loaded scene image remained current.
    Reused,
    /// A different scene image and its upper palette window were loaded.
    Loaded,
    /// No image was authored, so the vertical presentation band was cleared.
    Cleared,
}

/// Active-queue transition selected after one service pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PresentationSceneActiveTransition {
    /// Queue service did not reach a transition threshold.
    #[default]
    None,
    /// Presentation line 39 reset the palette transition percentage.
    PaletteReset,
    /// Ship presentation work opened the six-step depth transition.
    ShipDepthOpening,
}

/// Mutable semantic state retained by the scene dispatcher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationSceneDispatchState<ImageId> {
    /// Shared C2 gate, active line, request flags, and bridge redraw state.
    pub presentation: PresentationUpdateState,
    /// Most recently displayed line.
    pub displayed_line: Option<u16>,
    /// Whether a VM sequence keeps the newly loaded line visible.
    pub sequence_active: bool,
    /// Whether another scene coordinator owns image loading.
    pub scene_gate: bool,
    /// Whether ship/HUD state blocks active queue service.
    pub dispatch_blocked: bool,
    /// Whether line 29 selected Scruter Jo's alien overlay.
    pub alien_overlay_armed: bool,
    /// Deferred temporary sound action published by overlay completion.
    pub temporary_sound_trigger: bool,
    /// Finale-derived navigation-choice sound gate.
    pub navigation_choice_sound_gate: bool,
    /// Current frame destination and row policy.
    pub present_policy: PresentationPresentPolicy,
    /// Resource source selected for the current line.
    pub source: PresentationSceneSource,
    /// Last scene image decoded into the back buffer.
    pub loaded_scene_image: Option<ImageId>,
    /// Whether the latest load or service pass presented a frame.
    pub frame_presented: bool,
    /// Complete low-word ship state; bit eight owns presentation redraws.
    pub ship_active_flags: u16,
    /// Finale request sampled when an active presentation finishes.
    pub finale_requested: bool,
    /// Queue metric used by line-specific transition thresholds.
    pub entry_metric: u16,
    /// Current queue read index subtracted with 16-bit wrapping.
    pub read_wrap_index: u16,
    /// Current palette transition percentage.
    pub palette_transition_percent: u16,
    /// Whether the ship depth door is opening.
    pub depth_opening: bool,
    /// Current ship depth-opening step.
    pub depth_step: u8,
}

impl<ImageId> Default for PresentationSceneDispatchState<ImageId> {
    fn default() -> Self {
        Self {
            presentation: PresentationUpdateState::default(),
            displayed_line: None,
            sequence_active: false,
            scene_gate: false,
            dispatch_blocked: false,
            alien_overlay_armed: false,
            temporary_sound_trigger: false,
            navigation_choice_sound_gate: false,
            present_policy: PresentationPresentPolicy::default(),
            source: PresentationSceneSource::Owned,
            loaded_scene_image: None,
            frame_presented: false,
            ship_active_flags: u16::MIN,
            finale_requested: false,
            entry_metric: u16::MIN,
            read_wrap_index: u16::MIN,
            palette_transition_percent: u16::MIN,
            depth_opening: false,
            depth_step: u8::MIN,
        }
    }
}

/// Typed authored data and palette storage consumed by one dispatch.
pub struct PresentationSceneDispatchContext<'a, RecordId, ImageId> {
    /// Scene artwork indexed by presentation resource identity.
    pub scenes: &'a [PresentationSceneDescriptor<ImageId>],
    /// Record related from the active C4 triple.
    pub active_record_related: Option<&'a RecordId>,
    /// Typed identity of Scruter Jo's object record.
    pub scruter_jo_record: &'a RecordId,
    /// Exactly the first eight native line IDs that enable unclamped rows.
    pub unclamped_line_ids: &'a [u8; UNCLAMPED_LINE_ID_COUNT],
    /// Whether line eight can use a shared cached resource source.
    pub shared_cache_available: bool,
    /// Scene palette updated by image decoding.
    pub scene_palette: &'a mut IndexedGamePalette,
    /// Captured scene colors 128 through 191.
    pub presentation_palette: &'a mut ShipHudPaletteSnapshot,
}

/// Already translated resource and renderer operations called by the coordinator.
pub trait PresentationSceneDispatchHost<ImageId> {
    /// Host failure propagated without fallback behavior.
    type Error;

    /// Decode one scene image with scene-palette refresh and transparent zero.
    fn load_scene_image(
        &mut self,
        image: &ImageId,
        scene_palette: &mut IndexedGamePalette,
    ) -> Result<(), Self::Error>;

    /// Clear one half-open row band in the back buffer.
    fn clear_back_buffer_band(&mut self, rows: Range<usize>, color: u8) -> Result<(), Self::Error>;

    /// Load, activate, present, and prefill one resource sequence.
    fn load_presentation_sequence(
        &mut self,
        resource: PresentationResourceId,
        source: PresentationSceneSource,
        policy: PresentationPresentPolicy,
    ) -> Result<bool, Self::Error>;

    /// Build the black palette remap used when a new line becomes displayed.
    fn build_black_remap(
        &mut self,
        blend_percent: u8,
        target: [u8; RGB_COMPONENT_COUNT],
    ) -> Result<(), Self::Error>;

    /// Service one active streamed-presentation queue frame.
    fn service_presentation_queue(
        &mut self,
        policy: PresentationPresentPolicy,
    ) -> Result<bool, Self::Error>;

    /// Return whether the presentation source remains open or draining.
    fn presentation_source_open_or_draining(&mut self) -> bool;

    /// Clear one half-open row band in the display buffer.
    fn clear_display_band(&mut self, rows: Range<usize>, color: u8) -> Result<(), Self::Error>;
}

/// Terminal path selected by one scene dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationSceneDispatchOutcome {
    /// No active presentation line exists.
    NoActiveLine,
    /// The armed alien overlay converted the next line into a sound trigger.
    AlienOverlayTriggered,
    /// A new sequence started, optionally followed by a black remap.
    SequenceStarted {
        /// Image operation preceding the sequence load.
        image: PresentationSceneImageOutcome,
        /// Whether a changed displayed line built the black remap.
        black_remap_built: bool,
    },
    /// Ship/HUD state suppressed active queue service.
    ActiveDispatchBlocked,
    /// The stream finished and released presentation ownership.
    PresentationFinished,
    /// Queue service retained the active line and may have advanced a transition.
    Active {
        /// Line-specific transition selected after queue service.
        transition: PresentationSceneActiveTransition,
    },
}

/// Invalid authored data or host operation encountered by scene dispatch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresentationSceneDispatchError<HostError> {
    /// The active line had no authored scene descriptor.
    MissingSceneDescriptor(PresentationResourceId),
    /// Line 29 had no typed related record to compare with Scruter Jo.
    MissingActiveRecordRelation,
    /// Adding the 130-row image band overflowed the host row domain.
    SceneBandOverflow {
        /// Authored vertical row offset.
        first_row: usize,
    },
    /// A translated resource or renderer operation failed.
    Host(HostError),
}

impl<HostError: fmt::Debug> fmt::Display for PresentationSceneDispatchError<HostError> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid presentation scene dispatch: {self:?}")
    }
}

impl<HostError> Error for PresentationSceneDispatchError<HostError>
where
    HostError: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Host(source) => Some(source),
            Self::MissingSceneDescriptor(_)
            | Self::MissingActiveRecordRelation
            | Self::SceneBandOverflow { .. } => None,
        }
    }
}

fn line_requests_skipped_back_buffer_present(line: u16) -> bool {
    SKIP_BACK_BUFFER_PRESENT_LINES.contains(&line)
}

fn configure_present_policy(
    line: u16,
    unclamped_line_ids: &[u8; UNCLAMPED_LINE_ID_COUNT],
    policy: &mut PresentationPresentPolicy,
    request_flags: &mut u8,
) {
    policy.draw_via_back_buffer = DRAW_VIA_BACK_BUFFER_LINES.contains(&line);
    policy.skip_back_buffer_present = false;
    policy.unclamped_rows = unclamped_line_ids.contains(&(line as u8));
    if !policy.draw_via_back_buffer && line_requests_skipped_back_buffer_present(line) {
        *request_flags |= PRESENTATION_REQUEST_FLAG;
        policy.skip_back_buffer_present = true;
    }
}

fn prepare_scene_image<RecordId, ImageId, Host>(
    line: PresentationResourceId,
    state: &mut PresentationSceneDispatchState<ImageId>,
    context: &mut PresentationSceneDispatchContext<'_, RecordId, ImageId>,
    host: &mut Host,
) -> Result<PresentationSceneImageOutcome, PresentationSceneDispatchError<Host::Error>>
where
    ImageId: Clone + Eq,
    Host: PresentationSceneDispatchHost<ImageId>,
{
    if state.scene_gate {
        return Ok(PresentationSceneImageOutcome::SkippedBySceneGate);
    }

    let descriptor = context
        .scenes
        .get(usize::from(line.get()))
        .ok_or(PresentationSceneDispatchError::MissingSceneDescriptor(line))?;
    let mut outcome = PresentationSceneImageOutcome::Reused;
    match &descriptor.image {
        Some(image) if state.loaded_scene_image.as_ref() != Some(image) => {
            state.loaded_scene_image = Some(image.clone());
            host.load_scene_image(image, context.scene_palette)
                .map_err(PresentationSceneDispatchError::Host)?;
            context.presentation_palette.copy_from_slice(
                &context.scene_palette[super::SHIP_HUD_PALETTE_FIRST
                    ..super::SHIP_HUD_PALETTE_FIRST + super::SHIP_HUD_PALETTE_COLOR_COUNT],
            );
            outcome = PresentationSceneImageOutcome::Loaded;
        }
        Some(_) => {}
        None => {
            state.loaded_scene_image = None;
        }
    }

    if state.loaded_scene_image.is_none() {
        let first_row = state.present_policy.vertical_offset;
        let last_row = first_row
            .checked_add(SCENE_IMAGE_ROW_COUNT)
            .ok_or(PresentationSceneDispatchError::SceneBandOverflow { first_row })?;
        host.clear_back_buffer_band(first_row..last_row, u8::MIN)
            .map_err(PresentationSceneDispatchError::Host)?;
        outcome = PresentationSceneImageOutcome::Cleared;
    }
    Ok(outcome)
}

/// Dispatch one scene-image or active streamed-presentation update.
///
/// This translates `dlg_line_id_scene_dispatch` at BLOODPRG offset `0x009D10`.
/// Optional resource and record identities replace near and far pointers;
/// explicit booleans replace low-bit gates; typed palette and policy state
/// retain image caching, the first-eight-only row-mode table, source selection,
/// queue teardown, and line-specific transition thresholds.
pub fn dispatch_presentation_scene<RecordId, ImageId, Host>(
    state: &mut PresentationSceneDispatchState<ImageId>,
    context: &mut PresentationSceneDispatchContext<'_, RecordId, ImageId>,
    host: &mut Host,
) -> Result<PresentationSceneDispatchOutcome, PresentationSceneDispatchError<Host::Error>>
where
    RecordId: Eq,
    ImageId: Clone + Eq,
    Host: PresentationSceneDispatchHost<ImageId>,
{
    state.frame_presented = false;
    let Some(line) = state.presentation.active_line else {
        return Ok(PresentationSceneDispatchOutcome::NoActiveLine);
    };
    if line > MAX_SIGNED_PRESENTATION_LINE {
        return Ok(PresentationSceneDispatchOutcome::NoActiveLine);
    }
    let resource = PresentationResourceId::new(line);

    if state.presentation.gate_flags & PRESENTATION_ACTIVE_FLAG == u8::MIN {
        if line == SCRUTER_JO_PRESENTATION_LINE {
            let related = context
                .active_record_related
                .ok_or(PresentationSceneDispatchError::MissingActiveRecordRelation)?;
            state.alien_overlay_armed = related == context.scruter_jo_record;
        } else if state.alien_overlay_armed {
            state.temporary_sound_trigger = true;
            return Ok(PresentationSceneDispatchOutcome::AlienOverlayTriggered);
        }

        let image = prepare_scene_image(resource, state, context, host)?;
        state.presentation.gate_flags = PRESENTATION_ACTIVE_FLAG;
        configure_present_policy(
            line,
            context.unclamped_line_ids,
            &mut state.present_policy,
            &mut state.presentation.request_flags,
        );
        state.source = if line == SHARED_CACHE_PRESENTATION_LINE && context.shared_cache_available {
            PresentationSceneSource::SharedCache
        } else {
            PresentationSceneSource::Owned
        };
        state.frame_presented = host
            .load_presentation_sequence(resource, state.source, state.present_policy)
            .map_err(PresentationSceneDispatchError::Host)?;

        if !state.sequence_active && !state.scene_gate {
            return Ok(PresentationSceneDispatchOutcome::SequenceStarted {
                image,
                black_remap_built: false,
            });
        }
        if state.displayed_line == state.presentation.active_line {
            return Ok(PresentationSceneDispatchOutcome::SequenceStarted {
                image,
                black_remap_built: false,
            });
        }
        state.displayed_line = state.presentation.active_line;
        host.build_black_remap(BLACK_REMAP_PERCENT, BLACK_REMAP_TARGET)
            .map_err(PresentationSceneDispatchError::Host)?;
        return Ok(PresentationSceneDispatchOutcome::SequenceStarted {
            image,
            black_remap_built: true,
        });
    }

    if state.dispatch_blocked {
        return Ok(PresentationSceneDispatchOutcome::ActiveDispatchBlocked);
    }
    state.frame_presented = host
        .service_presentation_queue(state.present_policy)
        .map_err(PresentationSceneDispatchError::Host)?;
    if !host.presentation_source_open_or_draining() {
        if state.ship_active_flags & SHIP_ACTIVE_PRESENTATION_FLAG != u16::MIN {
            state.presentation.bridge_redraw_pending = PRESENTATION_ACTIVE_FLAG;
        }
        if line == DISPLAY_CLEAR_PRESENTATION_LINE {
            host.clear_display_band(DISPLAY_CLEAR_FIRST_ROW..DISPLAY_CLEAR_LAST_ROW, u8::MIN)
                .map_err(PresentationSceneDispatchError::Host)?;
        }
        state.temporary_sound_trigger = state.alien_overlay_armed;
        state.navigation_choice_sound_gate = state.finale_requested;
        state.presentation.gate_flags = u8::MIN;
        state.displayed_line = state.presentation.active_line;
        state.presentation.active_line = None;
        state.presentation.request_flags &= !PRESENTATION_REQUEST_FLAG;
        return Ok(PresentationSceneDispatchOutcome::PresentationFinished);
    }

    let entry_delta = state.entry_metric.wrapping_sub(state.read_wrap_index);
    let transition = if line == PALETTE_TRANSITION_PRESENTATION_LINE {
        if entry_delta == PALETTE_TRANSITION_ENTRY_DELTA {
            state.palette_transition_percent = u16::MIN;
            PresentationSceneActiveTransition::PaletteReset
        } else {
            PresentationSceneActiveTransition::None
        }
    } else if state.ship_active_flags & SHIP_ACTIVE_PRESENTATION_FLAG != u16::MIN
        && entry_delta == SHIP_DEPTH_ENTRY_DELTA
    {
        state.depth_opening = true;
        state.depth_step = SHIP_DEPTH_OPENING_STEP;
        PresentationSceneActiveTransition::ShipDepthOpening
    } else {
        PresentationSceneActiveTransition::None
    };
    Ok(PresentationSceneDispatchOutcome::Active { transition })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 11;
    const TEST_IMAGE_ID: u16 = 0x1234;
    const DIFFERENT_IMAGE_ID: u16 = 0x2222;
    const SCRUTER_RECORD_ID: u16 = 0x4567;
    const OTHER_RECORD_ID: u16 = 0x6666;
    const INITIAL_REQUEST_FLAGS: u8 = 0xA3;
    const INITIAL_BRIDGE_REDRAW: u8 = 0x44;
    const INITIAL_PALETTE_TRANSITION: u16 = 0x7777;
    const INITIAL_DEPTH_STEP: u8 = 0x33;
    const INITIAL_ENTRY_METRIC: u16 = 0x3333;
    const INITIAL_READ_INDEX: u16 = 0x1111;
    const TRANSITION_ENTRY_METRIC: u16 = 0x24;
    const DEPTH_ENTRY_METRIC: u16 = 0x18;
    const THRESHOLD_READ_INDEX: u16 = 0x10;
    const DISPLAYED_BEFORE_REMAP: u16 = 7;
    const SCENE_VERTICAL_OFFSET: usize = 53;
    const INITIAL_PALETTE_BYTE: u8 = 0xA5;
    const MODE_TABLE_BASE: u8 = 0xE0;

    #[derive(Deserialize)]
    struct DispatchOracle {
        name: String,
        line: u16,
        presentation_gate: u8,
        scene_gate: u8,
        calls: Vec<OracleCall>,
        result: DispatchResult,
        caller_es_palette_copy: String,
    }

    #[derive(Deserialize)]
    struct OracleCall {
        call: String,
    }

    #[derive(Deserialize)]
    struct DispatchResult {
        active_line: u16,
        displayed_line: u16,
        presentation_gate: u8,
        alien_overlay_armed: u8,
        temp_snd_trigger: u8,
        draw_via_back_buffer: u8,
        skip_back_buffer_present: u8,
        source_is_banked: u8,
        unclamped_row_count: u8,
        request_flags: u8,
        depth_opening: u8,
        depth_step: u8,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Event {
        LoadImage,
        ClearBackBuffer(Range<usize>),
        LoadSequence(PresentationResourceId),
        BuildBlackRemap,
        ServiceQueue,
        QueryQueueState,
        ClearDisplay(Range<usize>),
    }

    struct RecordingHost {
        events: Vec<Event>,
        source_open_or_draining: bool,
    }

    impl PresentationSceneDispatchHost<u16> for RecordingHost {
        type Error = std::convert::Infallible;

        fn load_scene_image(
            &mut self,
            _image: &u16,
            scene_palette: &mut IndexedGamePalette,
        ) -> Result<(), Self::Error> {
            self.events.push(Event::LoadImage);
            for (byte_index, component) in scene_palette[super::super::SHIP_HUD_PALETTE_FIRST
                ..super::super::SHIP_HUD_PALETTE_FIRST + super::super::SHIP_HUD_PALETTE_COLOR_COUNT]
                .iter_mut()
                .flatten()
                .enumerate()
            {
                *component = (byte_index * 29 + 7) as u8;
            }
            Ok(())
        }

        fn clear_back_buffer_band(
            &mut self,
            rows: Range<usize>,
            color: u8,
        ) -> Result<(), Self::Error> {
            assert_eq!(color, u8::MIN);
            self.events.push(Event::ClearBackBuffer(rows));
            Ok(())
        }

        fn load_presentation_sequence(
            &mut self,
            resource: PresentationResourceId,
            _source: PresentationSceneSource,
            _policy: PresentationPresentPolicy,
        ) -> Result<bool, Self::Error> {
            self.events.push(Event::LoadSequence(resource));
            Ok(true)
        }

        fn build_black_remap(
            &mut self,
            blend_percent: u8,
            target: [u8; RGB_COMPONENT_COUNT],
        ) -> Result<(), Self::Error> {
            assert_eq!(blend_percent, BLACK_REMAP_PERCENT);
            assert_eq!(target, BLACK_REMAP_TARGET);
            self.events.push(Event::BuildBlackRemap);
            Ok(())
        }

        fn service_presentation_queue(
            &mut self,
            _policy: PresentationPresentPolicy,
        ) -> Result<bool, Self::Error> {
            self.events.push(Event::ServiceQueue);
            Ok(false)
        }

        fn presentation_source_open_or_draining(&mut self) -> bool {
            self.events.push(Event::QueryQueueState);
            self.source_open_or_draining
        }

        fn clear_display_band(&mut self, rows: Range<usize>, color: u8) -> Result<(), Self::Error> {
            assert_eq!(color, u8::MIN);
            self.events.push(Event::ClearDisplay(rows));
            Ok(())
        }
    }

    fn optional_line(value: u16) -> Option<u16> {
        (value != u16::MAX).then_some(value)
    }

    fn decode_hex(encoded: &str) -> Vec<u8> {
        encoded
            .as_bytes()
            .chunks_exact(2)
            .map(|digits| {
                let digits = std::str::from_utf8(digits).unwrap();
                u8::from_str_radix(digits, 16).unwrap()
            })
            .collect()
    }

    fn expected_events(vector: &DispatchOracle) -> Vec<Event> {
        vector
            .calls
            .iter()
            .map(|call| match call.call.as_str() {
                "pbm_image_load_and_decode" => Event::LoadImage,
                "back_buffer_fill" => Event::ClearBackBuffer(
                    SCENE_VERTICAL_OFFSET..SCENE_VERTICAL_OFFSET + SCENE_IMAGE_ROW_COUNT,
                ),
                "resource_load_sequence" => {
                    Event::LoadSequence(PresentationResourceId::new(vector.line))
                }
                "palette_blend_remap_table_build" => Event::BuildBlackRemap,
                "ems_resource_flush" => Event::ServiceQueue,
                "list_d8c_state_le_one" => Event::QueryQueueState,
                "blit_fill_row_5221" => {
                    Event::ClearDisplay(DISPLAY_CLEAR_FIRST_ROW..DISPLAY_CLEAR_LAST_ROW)
                }
                unknown => panic!("unknown scene-dispatch oracle call {unknown}"),
            })
            .collect()
    }

    #[test]
    fn scene_dispatch_accounts_for_every_original_coordinator_vector() {
        let vectors: Vec<DispatchOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_9d10_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut scenes = vec![PresentationSceneDescriptor { image: None }; 64];
            let image = match vector.name.as_str() {
                "missing_image_clears_back_buffer_band" => None,
                _ => Some(TEST_IMAGE_ID),
            };
            if let Some(scene) = scenes.get_mut(usize::from(vector.line)) {
                scene.image = image;
            }
            let mut mode_ids = std::array::from_fn(|index| MODE_TABLE_BASE + index as u8);
            match vector.name.as_str() {
                "new_image_loads_and_copies_palette" => mode_ids[usize::MIN] = vector.line as u8,
                "unbanked_line_honors_first_eight_mode_slots" => {
                    mode_ids[UNCLAMPED_LINE_ID_COUNT - 1] = vector.line as u8;
                }
                _ => {}
            }
            let relation = if vector.name == "scruter_jo_record_arms_overlay" {
                SCRUTER_RECORD_ID
            } else {
                OTHER_RECORD_ID
            };
            let loaded_scene_image = match vector.name.as_str() {
                "new_image_loads_and_copies_palette" | "missing_image_clears_back_buffer_band" => {
                    Some(DIFFERENT_IMAGE_ID)
                }
                _ => image,
            };
            let mut state = PresentationSceneDispatchState {
                presentation: PresentationUpdateState {
                    gate_flags: vector.presentation_gate,
                    bridge_redraw_pending: INITIAL_BRIDGE_REDRAW,
                    active_line: optional_line(vector.line),
                    request_flags: INITIAL_REQUEST_FLAGS,
                },
                displayed_line: match vector.name.as_str() {
                    "banked_line_builds_black_remap" => Some(DISPLAYED_BEFORE_REMAP),
                    _ => optional_line(vector.line),
                },
                sequence_active: vector.name == "banked_line_builds_black_remap",
                scene_gate: vector.scene_gate & PRESENTATION_ACTIVE_FLAG != u8::MIN,
                dispatch_blocked: vector.name == "active_dispatch_blocked",
                alien_overlay_armed: matches!(
                    vector.name.as_str(),
                    "armed_overlay_triggers_on_next_line" | "active_line_five_teardown"
                ),
                temporary_sound_trigger: true,
                navigation_choice_sound_gate: true,
                present_policy: PresentationPresentPolicy {
                    draw_via_back_buffer: true,
                    skip_back_buffer_present: true,
                    unclamped_rows: true,
                    vertical_offset: SCENE_VERTICAL_OFFSET,
                },
                source: PresentationSceneSource::Owned,
                loaded_scene_image,
                frame_presented: true,
                ship_active_flags: if matches!(
                    vector.name.as_str(),
                    "active_line_five_teardown" | "ready_ship_line_arms_depth_opening"
                ) {
                    SHIP_ACTIVE_PRESENTATION_FLAG
                } else {
                    u16::MIN
                },
                finale_requested: vector.name == "active_line_five_teardown",
                entry_metric: match vector.name.as_str() {
                    "ready_line_27_resets_transition" => TRANSITION_ENTRY_METRIC,
                    "ready_ship_line_arms_depth_opening" => DEPTH_ENTRY_METRIC,
                    _ => INITIAL_ENTRY_METRIC,
                },
                read_wrap_index: if matches!(
                    vector.name.as_str(),
                    "ready_line_27_resets_transition" | "ready_ship_line_arms_depth_opening"
                ) {
                    THRESHOLD_READ_INDEX
                } else {
                    INITIAL_READ_INDEX
                },
                palette_transition_percent: INITIAL_PALETTE_TRANSITION,
                depth_opening: false,
                depth_step: INITIAL_DEPTH_STEP,
            };
            let mut scene_palette = [[u8::MIN; RGB_COMPONENT_COUNT]; 256];
            let mut presentation_palette = [[INITIAL_PALETTE_BYTE; RGB_COMPONENT_COUNT];
                super::super::SHIP_HUD_PALETTE_COLOR_COUNT];
            let mut context = PresentationSceneDispatchContext {
                scenes: &scenes,
                active_record_related: Some(&relation),
                scruter_jo_record: &SCRUTER_RECORD_ID,
                unclamped_line_ids: &mode_ids,
                shared_cache_available: vector.name == "banked_line_builds_black_remap",
                scene_palette: &mut scene_palette,
                presentation_palette: &mut presentation_palette,
            };
            let mut host = RecordingHost {
                events: Vec::new(),
                source_open_or_draining: vector.name != "active_line_five_teardown",
            };

            dispatch_presentation_scene(&mut state, &mut context, &mut host).unwrap();

            assert_eq!(host.events, expected_events(&vector), "{}", vector.name);
            assert_eq!(
                state.presentation.active_line,
                optional_line(vector.result.active_line),
                "{}",
                vector.name
            );
            assert_eq!(
                state.displayed_line,
                optional_line(vector.result.displayed_line),
                "{}",
                vector.name
            );
            assert_eq!(
                state.presentation.gate_flags, vector.result.presentation_gate,
                "{}",
                vector.name
            );
            assert_eq!(
                state.alien_overlay_armed,
                vector.result.alien_overlay_armed & PRESENTATION_ACTIVE_FLAG != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                state.temporary_sound_trigger,
                vector.result.temp_snd_trigger & PRESENTATION_ACTIVE_FLAG != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                state.present_policy.draw_via_back_buffer,
                vector.result.draw_via_back_buffer & PRESENTATION_ACTIVE_FLAG != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                state.present_policy.skip_back_buffer_present,
                vector.result.skip_back_buffer_present & PRESENTATION_ACTIVE_FLAG != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                state.source == PresentationSceneSource::SharedCache,
                vector.result.source_is_banked & PRESENTATION_ACTIVE_FLAG != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                state.present_policy.unclamped_rows,
                vector.result.unclamped_row_count & PRESENTATION_ACTIVE_FLAG != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                state.presentation.request_flags, vector.result.request_flags,
                "{}",
                vector.name
            );
            assert_eq!(
                state.presentation.bridge_redraw_pending,
                if vector.name == "active_line_five_teardown" {
                    PRESENTATION_ACTIVE_FLAG
                } else {
                    INITIAL_BRIDGE_REDRAW
                },
                "{}",
                vector.name
            );
            assert_eq!(
                state.palette_transition_percent,
                if vector.name == "ready_line_27_resets_transition" {
                    u16::MIN
                } else {
                    INITIAL_PALETTE_TRANSITION
                },
                "{}",
                vector.name
            );
            assert_eq!(
                state.depth_opening,
                vector.result.depth_opening & PRESENTATION_ACTIVE_FLAG != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                state.depth_step, vector.result.depth_step,
                "{}",
                vector.name
            );
            let actual_palette: Vec<_> = presentation_palette.iter().flatten().copied().collect();
            assert_eq!(
                actual_palette,
                decode_hex(&vector.caller_es_palette_copy),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn every_signed_negative_line_returns_before_descriptor_lookup() {
        let relation = OTHER_RECORD_ID;
        let mut scene_palette = [[u8::MIN; RGB_COMPONENT_COUNT]; 256];
        let mut presentation_palette =
            [[u8::MIN; RGB_COMPONENT_COUNT]; super::super::SHIP_HUD_PALETTE_COLOR_COUNT];
        let mut context = PresentationSceneDispatchContext {
            scenes: &[],
            active_record_related: Some(&relation),
            scruter_jo_record: &SCRUTER_RECORD_ID,
            unclamped_line_ids: &[u8::MIN; UNCLAMPED_LINE_ID_COUNT],
            shared_cache_available: false,
            scene_palette: &mut scene_palette,
            presentation_palette: &mut presentation_palette,
        };
        let mut host = RecordingHost {
            events: Vec::new(),
            source_open_or_draining: true,
        };

        for line in [i16::MIN as u16, u16::MAX] {
            let mut state = PresentationSceneDispatchState {
                presentation: PresentationUpdateState {
                    active_line: Some(line),
                    ..PresentationUpdateState::default()
                },
                ..PresentationSceneDispatchState::default()
            };
            assert_eq!(
                dispatch_presentation_scene(&mut state, &mut context, &mut host).unwrap(),
                PresentationSceneDispatchOutcome::NoActiveLine
            );
        }
        assert!(host.events.is_empty());
    }
}
