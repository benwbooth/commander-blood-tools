//! RGB overlay host for the recovered progressive subtitle renderer.

use anyhow::{Context, Result};

use crate::native::bloodprg::{
    FontPoint, GameTimerState, PresentationTextOrigin, SubtitleFrameDraw, SubtitleFramePrimitive,
    SubtitleFramePrimitiveKind, SubtitleRevealLine, SubtitleRevealOutcome, SubtitleRevealPhase,
    SubtitleRevealRenderer, SubtitleRevealState, TextPresentationState, draw_subtitle_reveal_line,
    update_subtitle_reveal,
};

use super::{LOGICAL_FRAMEBUFFER_HEIGHT, LOGICAL_FRAMEBUFFER_WIDTH, OriginalGameRuntime};

const SUBTITLE_TEXT_ORIGIN: [u16; 2] = [10, 8];
const SECONDARY_FRAME_FIRST_PRIMITIVE: usize = 8;
const CARRIAGE_RETURN: u8 = b'\r';

// SS:0x5E6F through the 0xFFFF terminator in BLOODPRG.EXE. The secondary
// frame starts at SS:0x5EAF, so it is the final eight entries of this table.
const SUBTITLE_FRAME_PRIMITIVES: [SubtitleFramePrimitive; 16] = [
    horizontal([4, 4], 12),
    vertical([4, 5], 11),
    horizontal([304, 4], 12),
    vertical([316, 4], 12),
    horizontal([4, 196], 12),
    vertical([4, 184], 12),
    horizontal([304, 196], 12),
    vertical([316, 184], 13),
    horizontal([5, 5], 10),
    vertical([5, 6], 9),
    horizontal([305, 5], 10),
    vertical([315, 5], 10),
    horizontal([5, 195], 10),
    vertical([5, 185], 10),
    horizontal([305, 195], 10),
    vertical([315, 185], 11),
];

const fn horizontal(origin: [u16; 2], extent: u16) -> SubtitleFramePrimitive {
    SubtitleFramePrimitive {
        kind: SubtitleFramePrimitiveKind::Horizontal,
        origin,
        extent,
    }
}

const fn vertical(origin: [u16; 2], extent: u16) -> SubtitleFramePrimitive {
    SubtitleFramePrimitive {
        kind: SubtitleFramePrimitiveKind::Vertical,
        origin,
        extent,
    }
}

/// Persistent timers used by the lifecycle subtitle callback.
pub struct RuntimeSubtitleReveal {
    state: SubtitleRevealState,
    text_drawn: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct SubtitleRasterAudit {
    pub expected_pixel_count: usize,
    pub matching_pixel_count: usize,
}

impl RuntimeSubtitleReveal {
    /// Construct subtitle timing from the step decoded out of `BLOODPRG.EXE`.
    pub fn new(initial_text_speed_step: u16) -> Self {
        Self {
            text_drawn: false,
            state: SubtitleRevealState {
                text_speed_step: initial_text_speed_step,
                text_origin: SUBTITLE_TEXT_ORIGIN,
                ..SubtitleRevealState::default()
            },
        }
    }
    /// Borrow the exact reveal phase, timer, origin, and host gates.
    pub const fn state(&self) -> &SubtitleRevealState {
        &self.state
    }

    /// Apply a player-selected text-speed step without changing reveal progress.
    pub fn set_text_speed_step(&mut self, step: u16) {
        self.state.text_speed_step = step;
    }

    /// Synchronize the panel-owned native subtitle Y origin.
    pub(super) fn set_presentation_text_origin(&mut self, origin: PresentationTextOrigin) {
        self.state.text_origin[1] = origin.logical_y();
    }

    /// Draw the synchronous loading caption emitted by the native VOC source loader.
    pub fn draw_stream_wait_prompt(
        &mut self,
        runtime: &mut OriginalGameRuntime,
        presentation: &mut TextPresentationState,
        prompt: &[u8],
    ) -> Result<SubtitleRevealOutcome> {
        let reveal_cursor = prompt
            .iter()
            .position(|byte| *byte == CARRIAGE_RETURN)
            .context("stream wait prompt has no carriage-return terminator")?;
        presentation.subtitle_text = Box::from(prompt);
        presentation.subtitle_reveal_cursor = Some(reveal_cursor);
        presentation.hold_ready = false;
        presentation.menu_deferred = false;
        self.state.phase = crate::native::bloodprg::SubtitleRevealPhase::Text;
        self.state.display_mode = true;
        let outcome = self.update(runtime, presentation);
        self.state.display_mode = false;
        outcome
    }

    /// Draw one subtitle frame using countdowns advanced by the canonical game timer.
    pub fn update(
        &mut self,
        runtime: &mut OriginalGameRuntime,
        presentation: &mut TextPresentationState,
    ) -> Result<SubtitleRevealOutcome> {
        let mut renderer = RuntimeSubtitleRenderer {
            runtime,
            deferred_error: None,
        };
        let outcome = update_subtitle_reveal(
            presentation,
            &mut self.state,
            &SUBTITLE_FRAME_PRIMITIVES,
            &SUBTITLE_FRAME_PRIMITIVES[SECONDARY_FRAME_FIRST_PRIMITIVE..],
            &mut renderer,
        )
        .context("advancing the recovered subtitle reveal")?;
        renderer.finish()?;
        self.text_drawn = matches!(outcome, SubtitleRevealOutcome::TextFrame { .. });
        Ok(outcome)
    }

    pub(super) fn import_lifecycle_state(
        &mut self,
        presentation: &crate::native::bloodprg::GamePresentationScheduler,
        ship_hud_active: bool,
    ) {
        self.state.display_mode = presentation.subtitle_word_list_mode;
        self.state.hold_owned_by_subtitle =
            presentation.owner == Some(crate::native::bloodprg::GamePresentationOwner::Subtitle);
        self.state.ship_hud_active = ship_hud_active;
    }

    /// Publish subtitle-owned countdown values before the canonical timer advances.
    pub(super) fn export_timer_state(&self, timer: &mut GameTimerState) {
        timer.subtitle_reveal_delay = self.state.reveal_delay;
        timer.subtitle_opening_frame_pulse = u16::from(self.state.opening_frame_pulse);
    }

    /// Import countdown values after the canonical timer has advanced one game frame.
    pub(super) fn import_timer_state(&mut self, timer: &GameTimerState) {
        self.state.reveal_delay = timer.subtitle_reveal_delay;
        self.state.opening_frame_pulse = timer.subtitle_opening_frame_pulse != u16::MIN;
    }

    /// Compare the native glyph raster against the RGB layer actually composited.
    pub(super) fn raster_audit(
        &self,
        runtime: &OriginalGameRuntime,
        presentation: &TextPresentationState,
    ) -> Result<Option<SubtitleRasterAudit>> {
        if !self.text_drawn
            || self.state.phase != SubtitleRevealPhase::Text
            || presentation.subtitle_reveal_cursor.is_none()
        {
            return Ok(None);
        }

        let mut state = self.state;
        state.reveal_delay = 1;
        let mut presentation = presentation.clone();
        let mut expected = vec![u8::MIN; LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT];
        let fonts = runtime.data().font_resources();
        let mut renderer = SubtitleRasterAuditRenderer {
            pixels: &mut expected,
            fonts,
            deferred_error: None,
        };
        let outcome =
            update_subtitle_reveal(&mut presentation, &mut state, &[], &[], &mut renderer)
                .context("reconstructing the current subtitle glyph raster")?;
        renderer.finish()?;
        if !matches!(outcome, SubtitleRevealOutcome::TextFrame { .. }) {
            return Ok(None);
        }

        let actual = runtime.ui_overlay_rgba();
        let mut expected_pixel_count = usize::MIN;
        let mut matching_pixel_count = usize::MIN;
        for (expected, actual) in expected.iter().copied().zip(actual.chunks_exact(4)) {
            if expected == u8::MIN {
                continue;
            }
            expected_pixel_count += 1;
            matching_pixel_count +=
                usize::from(runtime.data().dialogue_ui_assets.color(expected)? == actual);
        }
        Ok(Some(SubtitleRasterAudit {
            expected_pixel_count,
            matching_pixel_count,
        }))
    }
}

struct RuntimeSubtitleRenderer<'runtime> {
    runtime: &'runtime mut OriginalGameRuntime,
    deferred_error: Option<anyhow::Error>,
}

impl RuntimeSubtitleRenderer<'_> {
    fn record<T>(&mut self, result: Result<T>) {
        if self.deferred_error.is_none()
            && let Err(error) = result
        {
            self.deferred_error = Some(error);
        }
    }

    fn finish(self) -> Result<()> {
        match self.deferred_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl SubtitleRevealRenderer for RuntimeSubtitleRenderer<'_> {
    fn draw_frame_primitive(&mut self, draw: SubtitleFrameDraw) {
        let result = self.runtime.draw_dialogue_frame(draw);
        self.record(result);
    }

    fn draw_subtitle_line(&mut self, line: SubtitleRevealLine<'_>) {
        let result = self.runtime.draw_dialogue_line(line);
        self.record(result);
    }
}

struct SubtitleRasterAuditRenderer<'pixels, 'resources> {
    pixels: &'pixels mut [u8],
    fonts: &'resources commander_blood_formats::bloodprg::BloodprgFontResources,
    deferred_error: Option<anyhow::Error>,
}

impl SubtitleRasterAuditRenderer<'_, '_> {
    fn finish(self) -> Result<()> {
        match self.deferred_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl SubtitleRevealRenderer for SubtitleRasterAuditRenderer<'_, '_> {
    fn draw_frame_primitive(&mut self, _draw: SubtitleFrameDraw) {}

    fn draw_subtitle_line(&mut self, line: SubtitleRevealLine<'_>) {
        if self.deferred_error.is_some() {
            return;
        }
        self.deferred_error = draw_runtime_subtitle_line(self.pixels, self.fonts, line)
            .context("reconstructing a progressively revealed subtitle line")
            .err();
    }
}

fn draw_runtime_subtitle_line(
    pixels: &mut [u8],
    fonts: &commander_blood_formats::bloodprg::BloodprgFontResources,
    line: SubtitleRevealLine<'_>,
) -> Result<()> {
    let mut terminated = Vec::with_capacity(line.text.len().saturating_add(1));
    terminated.extend_from_slice(line.text);
    terminated.push(CARRIAGE_RETURN);
    let reveal_cursor = i64::try_from(line.reveal_cursor)
        .unwrap_or(i64::MAX)
        .saturating_sub(i64::try_from(line.byte_offset).unwrap_or(i64::MAX));
    let reveal_cursor = i32::try_from(reveal_cursor).unwrap_or_else(|_| {
        if reveal_cursor.is_negative() {
            i32::MIN
        } else {
            i32::MAX
        }
    });
    draw_subtitle_reveal_line(
        pixels,
        fonts,
        &terminated,
        FontPoint {
            x: i32::from(line.position[0]),
            y: i32::from(line.position[1]),
        },
        reveal_cursor,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use commander_blood_formats::bloodprg::decode_bloodprg_bridge_menu_text;

    use super::*;
    use crate::runtime::{OriginalGameData, OriginalGameDataPaths};

    const BLOODPRG_DATA_FILE_OFFSET: usize = 54_304;
    const INITIAL_TEXT_SPEED_DATA_OFFSET: usize = 2_762;
    const SUBTITLE_TEXT_ORIGIN_DATA_OFFSET: usize = 24_156;
    const SUBTITLE_FRAME_TABLE_DATA_OFFSET: usize = 24_175;
    const SUBTITLE_FRAME_RECORD_SIZE: usize = 8;
    const ENCODED_WORD_SIZE: usize = 2;
    const SUBTITLE_FRAME_X_OFFSET: usize = ENCODED_WORD_SIZE;
    const SUBTITLE_FRAME_Y_OFFSET: usize = ENCODED_WORD_SIZE * 2;
    const SUBTITLE_FRAME_EXTENT_OFFSET: usize = ENCODED_WORD_SIZE * 3;
    const SUBTITLE_FRAME_KIND_HORIZONTAL: u16 = 0;
    const SUBTITLE_FRAME_KIND_VERTICAL: u16 = 1;
    const SUBTITLE_FRAME_KIND_TERMINATOR: u16 = u16::MAX;
    const TEST_BACKGROUND_INDEX: u8 = 16;
    const REVEALED_SUBTITLE_COLOR: u8 = u8::MAX - 2;
    const FIRST_FRAME_TIMER_TICKS: usize = 8;
    const REMAINING_OPENING_PULSE_TICKS: usize = 24;

    static TEMPORARY_ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn presentation_panel_origin_updates_only_the_native_subtitle_y_coordinate() {
        let mut subtitle = RuntimeSubtitleReveal::new(u16::MIN);
        let original_x = subtitle.state().text_origin[0];

        subtitle.set_presentation_text_origin(PresentationTextOrigin::Opening);
        assert_eq!(subtitle.state().text_origin[0], original_x);
        assert_eq!(
            subtitle.state().text_origin[1],
            PresentationTextOrigin::Opening.logical_y()
        );

        subtitle.set_presentation_text_origin(PresentationTextOrigin::Normal);
        assert_eq!(subtitle.state().text_origin, SUBTITLE_TEXT_ORIGIN);
    }

    struct TemporaryRoot(std::path::PathBuf);

    impl TemporaryRoot {
        fn create() -> Self {
            let sequence = TEMPORARY_ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            Self(std::env::temp_dir().join(format!(
                "commander-blood-subtitle-test-{}-{sequence}",
                std::process::id(),
            )))
        }
    }

    impl Drop for TemporaryRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn recovered_frame_geometry_matches_the_executable_table() {
        let Ok(paths) = OriginalGameDataPaths::discover(None) else {
            return;
        };
        let executable = std::fs::read(paths.executable()).unwrap();
        let menu_text = decode_bloodprg_bridge_menu_text(&executable).unwrap();
        assert_eq!(
            read_executable_word(
                &executable,
                BLOODPRG_DATA_FILE_OFFSET + INITIAL_TEXT_SPEED_DATA_OFFSET
            ),
            menu_text.initial_text_speed_step()
        );
        assert_eq!(
            [
                read_executable_word(
                    &executable,
                    BLOODPRG_DATA_FILE_OFFSET + SUBTITLE_TEXT_ORIGIN_DATA_OFFSET
                ),
                read_executable_word(
                    &executable,
                    BLOODPRG_DATA_FILE_OFFSET
                        + SUBTITLE_TEXT_ORIGIN_DATA_OFFSET
                        + ENCODED_WORD_SIZE
                ),
            ],
            SUBTITLE_TEXT_ORIGIN
        );
        let table_start = BLOODPRG_DATA_FILE_OFFSET + SUBTITLE_FRAME_TABLE_DATA_OFFSET;
        let table_size = SUBTITLE_FRAME_PRIMITIVES.len() * SUBTITLE_FRAME_RECORD_SIZE;
        let table = &executable[table_start..table_start + table_size];

        for (record, primitive) in table
            .chunks_exact(SUBTITLE_FRAME_RECORD_SIZE)
            .zip(SUBTITLE_FRAME_PRIMITIVES)
        {
            let kind = read_executable_word(record, usize::MIN);
            let x = read_executable_word(record, SUBTITLE_FRAME_X_OFFSET);
            let y = read_executable_word(record, SUBTITLE_FRAME_Y_OFFSET);
            let extent = read_executable_word(record, SUBTITLE_FRAME_EXTENT_OFFSET);
            assert_eq!(
                kind,
                match primitive.kind {
                    SubtitleFramePrimitiveKind::Horizontal => SUBTITLE_FRAME_KIND_HORIZONTAL,
                    SubtitleFramePrimitiveKind::Vertical => SUBTITLE_FRAME_KIND_VERTICAL,
                }
            );
            assert_eq!(primitive.origin, [x, y]);
            assert_eq!(primitive.extent, extent);
        }
        let terminator = table_start + table_size;
        assert_eq!(
            read_executable_word(&executable, terminator),
            SUBTITLE_FRAME_KIND_TERMINATOR
        );
    }

    #[test]
    fn subtitle_countdowns_round_trip_through_the_canonical_timer_cadence() {
        let mut subtitle = RuntimeSubtitleReveal::new(8);
        subtitle.state.reveal_delay = 2;
        subtitle.state.opening_frame_pulse = true;
        let mut timer = GameTimerState::default();
        timer.start();
        let mut script = crate::native::bloodprg::ScriptRuntime::default();

        subtitle.export_timer_state(&mut timer);
        for _ in usize::MIN..FIRST_FRAME_TIMER_TICKS {
            crate::native::bloodprg::advance_game_timer_tick(
                &mut timer,
                &mut script,
                crate::native::bloodprg::GameTimerContext::default(),
            );
        }
        subtitle.import_timer_state(&timer);

        assert_eq!(subtitle.state.reveal_delay, u16::MIN);
        assert!(subtitle.state.opening_frame_pulse);

        for _ in usize::MIN..REMAINING_OPENING_PULSE_TICKS {
            crate::native::bloodprg::advance_game_timer_tick(
                &mut timer,
                &mut script,
                crate::native::bloodprg::GameTimerContext::default(),
            );
        }
        subtitle.import_timer_state(&timer);

        assert!(!subtitle.state.opening_frame_pulse);
    }

    #[test]
    fn original_palette_and_fonts_draw_the_opening_and_text_frames() {
        let Ok(paths) = OriginalGameDataPaths::discover(None) else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let initial_text_speed_step = data.bridge_menu_text().initial_text_speed_step();
        let mut runtime = OriginalGameRuntime::new(data);
        runtime
            .front_buffer_mut()
            .pixels_mut()
            .fill(TEST_BACKGROUND_INDEX);
        let mut presentation = TextPresentationState {
            subtitle_display_active: true,
            subtitle_text: Box::from(b"ARK\r".as_slice()),
            ..TextPresentationState::default()
        };
        let mut subtitle = RuntimeSubtitleReveal::new(initial_text_speed_step);

        assert!(matches!(
            subtitle.update(&mut runtime, &mut presentation).unwrap(),
            SubtitleRevealOutcome::OpeningFrame { .. }
        ));
        let bright = runtime.data().dialogue_ui_assets.color(u8::MAX).unwrap();
        assert!(
            runtime
                .ui_overlay_rgba()
                .chunks_exact(4)
                .any(|pixel| pixel == bright)
        );

        subtitle.state.phase = crate::native::bloodprg::SubtitleRevealPhase::Text;
        presentation.subtitle_reveal_cursor = Some(presentation.subtitle_text.len());
        subtitle.update(&mut runtime, &mut presentation).unwrap();
        assert!(runtime.ui_overlay_rgba().chunks_exact(4).any(|pixel| {
            pixel
                == runtime
                    .data()
                    .dialogue_ui_assets
                    .color(REVEALED_SUBTITLE_COLOR)
                    .unwrap()
        }));
        assert!(
            runtime
                .front_buffer()
                .pixels()
                .iter()
                .all(|&pixel| pixel == TEST_BACKGROUND_INDEX)
        );
        // A video EOF retains a pre-text RGB page. UI must remain independent
        // of that page, and a new video palette must not recolor the text.
        let overlay = runtime.ui_overlay_rgba().to_vec();
        runtime.front_buffer_mut().pixels_mut().fill(0);
        runtime.live_palette_mut().fill([63, 0, 0]);
        assert_eq!(runtime.ui_overlay_rgba(), overlay);
        let audit = subtitle
            .raster_audit(&runtime, &presentation)
            .unwrap()
            .unwrap();
        assert!(audit.expected_pixel_count > 0);
        assert_eq!(audit.expected_pixel_count, audit.matching_pixel_count);
        runtime.clear_ui_overlay();
        presentation.subtitle_display_active = false;
        presentation.hold_ready = false;
        subtitle.update(&mut runtime, &mut presentation).unwrap();
        assert!(runtime.ui_overlay_rgba().iter().all(|&byte| byte == 0));
    }

    #[test]
    fn stream_wait_prompt_uses_the_shared_text_buffer_and_reveal_cursor() {
        let Ok(paths) = OriginalGameDataPaths::discover(None) else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let initial_text_speed_step = data.bridge_menu_text().initial_text_speed_step();
        let mut runtime = OriginalGameRuntime::new(data);
        runtime
            .front_buffer_mut()
            .pixels_mut()
            .fill(TEST_BACKGROUND_INDEX);
        let mut presentation = TextPresentationState {
            menu_deferred: true,
            hold_ready: true,
            subtitle_text: Box::from(b"prior\r".as_slice()),
            ..TextPresentationState::default()
        };
        let mut subtitle = RuntimeSubtitleReveal::new(initial_text_speed_step);

        let outcome = subtitle
            .draw_stream_wait_prompt(
                &mut runtime,
                &mut presentation,
                crate::native::bloodprg::AUDIO_STREAM_WAIT_PROMPT,
            )
            .unwrap();

        assert!(matches!(outcome, SubtitleRevealOutcome::TextFrame { .. }));
        assert_eq!(
            presentation.subtitle_text.as_ref(),
            crate::native::bloodprg::AUDIO_STREAM_WAIT_PROMPT
        );
        assert_eq!(
            presentation.subtitle_reveal_cursor,
            Some(crate::native::bloodprg::AUDIO_STREAM_WAIT_PROMPT.len())
        );
        assert!(!presentation.subtitle_display_active);
        assert!(!presentation.menu_deferred);
        assert!(!presentation.hold_ready);
        assert!(!subtitle.state.display_mode);
        assert!(runtime.ui_overlay_rgba().chunks_exact(4).any(|pixel| {
            pixel
                == runtime
                    .data()
                    .dialogue_ui_assets
                    .color(REVEALED_SUBTITLE_COLOR)
                    .unwrap()
        }));
    }

    fn read_executable_word(bytes: &[u8], offset: usize) -> u16 {
        u16::from_le_bytes(
            bytes[offset..offset + ENCODED_WORD_SIZE]
                .try_into()
                .unwrap(),
        )
    }
}
