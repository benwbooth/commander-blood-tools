//! Flat-framebuffer renderer for the recovered bridge choice-list planner.

use anyhow::{Context, Result};

use crate::native::bloodprg::{
    BridgeSpriteRect, ChoiceListBackend, ChoiceListConfig, ChoiceListFrame, ChoiceListPointer,
    ChoiceListRect, ChoiceListRowKind, ChoiceListState, FontPoint, FontVerticalBand, GameFontFace,
    PaletteRemapTable, RasterPoint, build_banked_tint_table, draw_square_caps_text,
    measure_game_text_width, remap_framebuffer_rect, update_choice_list,
};

use super::{LOGICAL_FRAMEBUFFER_HEIGHT, LOGICAL_FRAMEBUFFER_WIDTH, OriginalGameRuntime};

const BRIDGE_CONSOLE_TINT_FIRST: u8 = 224;
const LOGICAL_DISPLAY_CLIP: BridgeSpriteRect = BridgeSpriteRect {
    left: 0,
    right: LOGICAL_FRAMEBUFFER_WIDTH as i32,
    top: 0,
    bottom: LOGICAL_FRAMEBUFFER_HEIGHT as i32,
};
const FULL_LOGICAL_FONT_BAND: FontVerticalBand = FontVerticalBand {
    top: 0,
    bottom: LOGICAL_FRAMEBUFFER_HEIGHT as i32 - 1,
};

/// Canonical values of the three mutable globals controlling list layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct RuntimeChoiceListStyle {
    /// Shared horizontal list anchor.
    pub center_x: i16,
    /// Whether each measured row retains its own width.
    pub preserve_individual_widths: bool,
    /// Whether the synthetic cancel row is appended.
    pub extra_cancel_entry: bool,
}

impl RuntimeChoiceListStyle {
    /// Values written by `ship_3d_hud_init` before target selection.
    pub const SHIP_TARGET: Self = Self {
        center_x: 80,
        preserve_individual_widths: true,
        extra_cancel_entry: true,
    };

    /// Values written by `presentation_ready_gate` when a word choice opens.
    pub const PRESENTATION_WORD_CHOICE: Self = Self {
        center_x: 225,
        preserve_individual_widths: false,
        extra_cancel_entry: false,
    };
}

pub(super) fn prepare_choice_list_frame(
    runtime: &mut OriginalGameRuntime,
    labels: &[&[u8]],
    config: ChoiceListConfig<'_>,
    state: &mut ChoiceListState,
    pointer: ChoiceListPointer,
) -> Result<ChoiceListFrame> {
    let fonts = runtime.data().font_resources().clone();
    let mut tint = [u8::MIN; 256];
    build_banked_tint_table(runtime.live_palette(), &mut tint, BRIDGE_CONSOLE_TINT_FIRST)
        .context("building the bridge choice-list tint table")?;

    let mut backend = RuntimeChoiceListBackend::new(runtime, &fonts, &tint, pointer);
    let frame = update_choice_list(labels, config, state, &mut backend);
    backend.finish()?;
    Ok(frame)
}

pub(super) struct RuntimeChoiceListBackend<'runtime> {
    runtime: &'runtime mut OriginalGameRuntime,
    fonts: &'runtime commander_blood_formats::bloodprg::BloodprgFontResources,
    tint: &'runtime PaletteRemapTable,
    pointer: ChoiceListPointer,
    deferred_error: Option<anyhow::Error>,
}

impl RuntimeChoiceListBackend<'_> {
    pub(super) fn new<'runtime>(
        runtime: &'runtime mut OriginalGameRuntime,
        fonts: &'runtime commander_blood_formats::bloodprg::BloodprgFontResources,
        tint: &'runtime PaletteRemapTable,
        pointer: ChoiceListPointer,
    ) -> RuntimeChoiceListBackend<'runtime> {
        RuntimeChoiceListBackend {
            runtime,
            fonts,
            tint,
            pointer,
            deferred_error: None,
        }
    }

    pub(super) fn record_error(&mut self, result: Result<()>) {
        if self.deferred_error.is_none()
            && let Err(error) = result
        {
            self.deferred_error = Some(error);
        }
    }

    pub(super) fn finish(&mut self) -> Result<()> {
        match self.deferred_error.take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    pub(super) fn runtime_mut(&mut self) -> &mut OriginalGameRuntime {
        self.runtime
    }

    pub(super) fn remap_region(&mut self, origin: RasterPoint, width: u16, height: u16) {
        let result = remap_framebuffer_rect(
            self.runtime.front_buffer_mut().pixels_mut(),
            LOGICAL_DISPLAY_CLIP,
            origin,
            width,
            height,
            self.tint,
        )
        .context("remapping a bridge choice-list transition region")
        .map(|_| ());
        self.record_error(result);
    }
}

impl ChoiceListBackend for RuntimeChoiceListBackend<'_> {
    fn measure_label(&mut self, label: &[u8]) -> u16 {
        match measure_game_text_width(label, GameFontFace::SquareCaps, self.fonts)
            .context("measuring a bridge choice-list label")
        {
            Ok(width) => width,
            Err(error) => {
                if self.deferred_error.is_none() {
                    self.deferred_error = Some(error);
                }
                u16::MIN
            }
        }
    }

    fn prepare_background(&mut self, rect: ChoiceListRect) {
        let result = remap_framebuffer_rect(
            self.runtime.front_buffer_mut().pixels_mut(),
            LOGICAL_DISPLAY_CLIP,
            RasterPoint {
                x: i32::from(rect.origin[0]),
                y: i32::from(rect.origin[1]),
            },
            rect.size[0],
            rect.size[1],
            self.tint,
        )
        .context("remapping the bridge choice-list background")
        .map(|_| ());
        self.record_error(result);
    }

    fn pointer(&mut self) -> ChoiceListPointer {
        self.pointer
    }
}

pub(super) fn draw_choice_list_rows(
    runtime: &mut OriginalGameRuntime,
    fonts: &commander_blood_formats::bloodprg::BloodprgFontResources,
    labels: &[&[u8]],
    cancel_label: Option<&[u8]>,
    frame: &ChoiceListFrame,
) -> Result<()> {
    for row in &frame.rows {
        let label = match row.kind {
            ChoiceListRowKind::Item(index) => labels
                .get(index)
                .copied()
                .with_context(|| format!("choice-list row {index} has no corresponding label"))?,
            ChoiceListRowKind::Cancel => {
                cancel_label.context("choice-list emitted a cancel row without a label")?
            }
        };
        draw_square_caps_text(
            runtime.front_buffer_mut().pixels_mut(),
            fonts,
            label,
            FontPoint {
                x: i32::from(row.position[0]),
                y: i32::from(row.position[1]),
            },
            FULL_LOGICAL_FONT_BAND,
            row.color,
        )
        .context("drawing a bridge choice-list row")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::runtime::{OriginalGameData, OriginalGameDataPaths};

    const TEST_CENTER_X: i16 = 160;
    const TEST_BACKGROUND_INDEX: u8 = 225;
    const FIRST_ROW_Y_INSET: i16 = 4;
    static TEMPORARY_ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(u64::MIN);

    struct TemporaryRoot(std::path::PathBuf);

    impl TemporaryRoot {
        fn create() -> Self {
            let sequence = TEMPORARY_ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "commander-blood-choice-list-{}-{sequence}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TemporaryRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn original_fonts_and_palette_render_an_interactive_choice_frame() {
        let Ok(paths) = OriginalGameDataPaths::discover(None) else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut runtime = OriginalGameRuntime::new(data);
        let labels: [&[u8]; 2] = [b"ARK", b"PTERRA"];
        let mut state = ChoiceListState::default();
        let layout = prepare_choice_list_frame(
            &mut runtime,
            &labels,
            ChoiceListConfig {
                center_x: TEST_CENTER_X,
                preserve_individual_widths: false,
                cancel_label: None,
                layout_only: true,
            },
            &mut state,
            ChoiceListPointer::default(),
        )
        .unwrap();

        runtime
            .front_buffer_mut()
            .pixels_mut()
            .fill(TEST_BACKGROUND_INDEX);
        let config = ChoiceListConfig {
            center_x: TEST_CENTER_X,
            preserve_individual_widths: false,
            cancel_label: None,
            layout_only: false,
        };
        let frame = prepare_choice_list_frame(
            &mut runtime,
            &labels,
            config,
            &mut state,
            ChoiceListPointer {
                position: [
                    layout.rect.origin[0],
                    layout.rect.origin[1].wrapping_add(FIRST_ROW_Y_INSET),
                ],
                primary_pressed: true,
            },
        )
        .unwrap();
        assert_eq!(frame.selected_item, Some(usize::MIN));
        assert!(!frame.cancelled);
        assert_eq!(frame.rows.len(), labels.len());

        let fonts = runtime.data().font_resources().clone();
        draw_choice_list_rows(&mut runtime, &fonts, &labels, None, &frame).unwrap();
        assert!(
            runtime
                .front_buffer()
                .pixels()
                .iter()
                .any(|pixel| *pixel != TEST_BACKGROUND_INDEX)
        );
    }
}
