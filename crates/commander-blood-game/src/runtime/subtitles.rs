//! Flat-framebuffer host for the recovered progressive subtitle renderer.

use anyhow::{Context, Result};

use crate::native::bloodprg::{
    BridgeSpriteRect, FontPoint, PaletteRemapTable, RasterPoint, RasterSpanPaint,
    SubtitleFrameDraw, SubtitleFramePrimitive, SubtitleFramePrimitiveKind, SubtitleRevealLine,
    SubtitleRevealOutcome, SubtitleRevealRenderer, SubtitleRevealState, TextPresentationState,
    build_palette_blend_remap_table, draw_planar_horizontal_span, draw_planar_vertical_span,
    draw_subtitle_reveal_line, update_subtitle_reveal,
};

use super::{LOGICAL_FRAMEBUFFER_HEIGHT, LOGICAL_FRAMEBUFFER_WIDTH, OriginalGameRuntime};

const SUBTITLE_TEXT_ORIGIN: [u16; 2] = [10, 8];
const DARK_FRAME_REMAP_PERCENT: u8 = 50;
const BLACK_BLEND_TARGET: [u8; 3] = [u8::MIN; 3];
const GAME_TIMER_TICKS_PER_FRAME: u16 = 8;
const SECONDARY_FRAME_FIRST_PRIMITIVE: usize = 8;
const CARRIAGE_RETURN: u8 = b'\r';
const LOGICAL_DISPLAY_CLIP: BridgeSpriteRect = BridgeSpriteRect {
    left: 0,
    right: LOGICAL_FRAMEBUFFER_WIDTH as i32,
    top: 0,
    bottom: LOGICAL_FRAMEBUFFER_HEIGHT as i32,
};

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

/// Persistent timers and palette remap used by the lifecycle subtitle callback.
pub struct RuntimeSubtitleReveal {
    state: SubtitleRevealState,
    remap_table: PaletteRemapTable,
    remap_palette: Option<crate::native::bloodprg::IndexedGamePalette>,
}

impl RuntimeSubtitleReveal {
    /// Construct subtitle timing from the step decoded out of `BLOODPRG.EXE`.
    pub fn new(initial_text_speed_step: u16) -> Self {
        Self {
            state: SubtitleRevealState {
                text_speed_step: initial_text_speed_step,
                text_origin: SUBTITLE_TEXT_ORIGIN,
                ..SubtitleRevealState::default()
            },
            remap_table: [u8::MIN; 256],
            remap_palette: None,
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

    /// Advance one game-frame worth of timer interrupts, then draw the subtitle.
    pub fn update(
        &mut self,
        runtime: &mut OriginalGameRuntime,
        presentation: &mut TextPresentationState,
    ) -> Result<SubtitleRevealOutcome> {
        self.advance_frame_timers();
        self.refresh_remap_table(runtime)?;
        let fonts = runtime.data().font_resources().clone();
        let mut renderer = RuntimeSubtitleRenderer {
            runtime,
            fonts: &fonts,
            remap_table: &self.remap_table,
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

    fn advance_frame_timers(&mut self) {
        self.state.opening_frame_pulse = false;
        self.state.reveal_delay = self
            .state
            .reveal_delay
            .saturating_sub(GAME_TIMER_TICKS_PER_FRAME);
    }

    fn refresh_remap_table(&mut self, runtime: &OriginalGameRuntime) -> Result<()> {
        if self.remap_palette.as_ref() == Some(runtime.live_palette()) {
            return Ok(());
        }
        build_palette_blend_remap_table(
            runtime.live_palette(),
            &mut self.remap_table,
            DARK_FRAME_REMAP_PERCENT,
            BLACK_BLEND_TARGET,
        )
        .context("building the subtitle-frame darkening table")?;
        self.remap_palette = Some(*runtime.live_palette());
        Ok(())
    }
}

struct RuntimeSubtitleRenderer<'runtime, 'resources> {
    runtime: &'runtime mut OriginalGameRuntime,
    fonts: &'resources commander_blood_formats::bloodprg::BloodprgFontResources,
    remap_table: &'resources PaletteRemapTable,
    deferred_error: Option<anyhow::Error>,
}

impl RuntimeSubtitleRenderer<'_, '_> {
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

impl SubtitleRevealRenderer for RuntimeSubtitleRenderer<'_, '_> {
    fn draw_frame_primitive(&mut self, draw: SubtitleFrameDraw) {
        let paint = if draw.remap {
            RasterSpanPaint::Remap(self.remap_table)
        } else {
            RasterSpanPaint::Solid(draw.color)
        };
        let start = RasterPoint {
            x: i32::from(draw.primitive.origin[0]),
            y: i32::from(draw.primitive.origin[1]),
        };
        let result = match draw.primitive.kind {
            SubtitleFramePrimitiveKind::Horizontal => draw_planar_horizontal_span(
                self.runtime.front_buffer_mut().pixels_mut(),
                LOGICAL_DISPLAY_CLIP,
                start,
                draw.primitive.extent,
                paint,
            ),
            SubtitleFramePrimitiveKind::Vertical => draw_planar_vertical_span(
                self.runtime.front_buffer_mut().pixels_mut(),
                LOGICAL_DISPLAY_CLIP,
                start,
                draw.primitive.extent,
                paint,
            ),
        }
        .context("drawing a recovered subtitle-frame primitive");
        self.record(result);
    }

    fn draw_subtitle_line(&mut self, line: SubtitleRevealLine<'_>) {
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
        let result = draw_subtitle_reveal_line(
            self.runtime.front_buffer_mut().pixels_mut(),
            self.fonts,
            &terminated,
            FontPoint {
                x: i32::from(line.position[0]),
                y: i32::from(line.position[1]),
            },
            reveal_cursor,
        )
        .context("drawing a progressively revealed subtitle line");
        self.record(result);
    }
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

    static TEMPORARY_ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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
        assert!(runtime.front_buffer().pixels().contains(&u8::MAX));

        subtitle.state.phase = crate::native::bloodprg::SubtitleRevealPhase::Text;
        presentation.subtitle_reveal_cursor = Some(presentation.subtitle_text.len());
        subtitle.update(&mut runtime, &mut presentation).unwrap();
        assert!(
            runtime
                .front_buffer()
                .pixels()
                .contains(&REVEALED_SUBTITLE_COLOR)
        );
    }

    fn read_executable_word(bytes: &[u8], offset: usize) -> u16 {
        u16::from_le_bytes(
            bytes[offset..offset + ENCODED_WORD_SIZE]
                .try_into()
                .unwrap(),
        )
    }
}
