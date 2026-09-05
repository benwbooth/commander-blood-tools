//! RGB overlay renderer for the recovered bridge choice-list planner.

use anyhow::{Context, Result};

use crate::native::bloodprg::{
    ChoiceListBackend, ChoiceListConfig, ChoiceListFrame, ChoiceListHandRequest, ChoiceListPointer,
    ChoiceListRect, ChoiceListRowKind, ChoiceListState, GameFontFace, RasterPoint,
    measure_game_text_width, update_choice_list,
};

use super::OriginalGameRuntime;

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
    /// Values written by `nav_choice_dispatch` when a bridge command is selected.
    pub const BRIDGE_CONSOLE: Self = Self {
        center_x: 100,
        preserve_individual_widths: true,
        extra_cancel_entry: true,
    };

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
    current_hand_animation: u16,
) -> Result<(ChoiceListFrame, Vec<ChoiceListHandRequest>)> {
    let fonts = runtime.data().font_resources().clone();
    let mut backend =
        RuntimeChoiceListBackend::new(runtime, &fonts, pointer, current_hand_animation);
    let frame = update_choice_list(labels, config, state, &mut backend);
    backend.finish()?;
    let hand_requests = backend.take_hand_requests();
    Ok((frame, hand_requests))
}

pub(super) struct RuntimeChoiceListBackend<'runtime> {
    runtime: &'runtime mut OriginalGameRuntime,
    fonts: &'runtime commander_blood_formats::bloodprg::BloodprgFontResources,
    pointer: ChoiceListPointer,
    current_hand_animation: u16,
    hand_requests: Vec<ChoiceListHandRequest>,
    deferred_error: Option<anyhow::Error>,
}

impl RuntimeChoiceListBackend<'_> {
    pub(super) fn new<'runtime>(
        runtime: &'runtime mut OriginalGameRuntime,
        fonts: &'runtime commander_blood_formats::bloodprg::BloodprgFontResources,
        pointer: ChoiceListPointer,
        current_hand_animation: u16,
    ) -> RuntimeChoiceListBackend<'runtime> {
        RuntimeChoiceListBackend {
            runtime,
            fonts,
            pointer,
            current_hand_animation,
            hand_requests: Vec::new(),
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

    pub(super) fn take_hand_requests(&mut self) -> Vec<ChoiceListHandRequest> {
        std::mem::take(&mut self.hand_requests)
    }

    pub(super) fn darken_region(&mut self, origin: RasterPoint, width: u16, height: u16) {
        self.runtime
            .darken_ui_rect([origin.x, origin.y], [width, height]);
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
        self.runtime
            .darken_ui_rect(rect.origin.map(i32::from), rect.size);
    }

    fn pointer(&mut self) -> ChoiceListPointer {
        self.pointer
    }

    fn current_hand_animation(&self) -> u16 {
        self.current_hand_animation
    }

    fn request_hand_animation(&mut self, request: ChoiceListHandRequest) {
        if request.restart_current {
            self.current_hand_animation = u16::MIN;
        }
        self.hand_requests.push(request);
    }
}

pub(super) fn draw_choice_list_rows(
    runtime: &mut OriginalGameRuntime,
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
        runtime
            .draw_choice_text(label, row.position.map(i32::from), row.color.try_into()?)
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
    fn imported_rgb_choices_preserve_c_layout_without_recoloring_the_scene() {
        let Ok(paths) = OriginalGameDataPaths::discover(None) else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut runtime = OriginalGameRuntime::new(data);
        let labels: [&[u8]; 2] = [b"ARK", b"PTERRA"];
        let mut state = ChoiceListState::default();
        let (layout, _) = prepare_choice_list_frame(
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
            u16::MIN,
        )
        .unwrap();
        assert_eq!(layout.rect.origin, [100, 85]);
        assert_eq!(layout.rect.size, [120, 30]);
        assert!(runtime.ui_overlay_rgba().iter().all(|&byte| byte == 0));

        runtime
            .front_buffer_mut()
            .pixels_mut()
            .fill(TEST_BACKGROUND_INDEX);
        // A later video changing its legacy colors must not recolor imported UI.
        runtime.live_palette_mut().fill([63, 0, 0]);
        let colors_before = *runtime.live_palette();
        let config = ChoiceListConfig {
            center_x: TEST_CENTER_X,
            preserve_individual_widths: false,
            cancel_label: None,
            layout_only: false,
        };
        let (frame, _) = prepare_choice_list_frame(
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
            u16::MIN,
        )
        .unwrap();
        assert_eq!(frame.selected_item, Some(usize::MIN));
        assert!(!frame.cancelled);
        assert_eq!(frame.rows.len(), labels.len());

        let fonts = runtime.data().font_resources().clone();
        draw_choice_list_rows(&mut runtime, &labels, None, &frame).unwrap();
        assert!(
            runtime
                .front_buffer()
                .pixels()
                .iter()
                .all(|pixel| *pixel == TEST_BACKGROUND_INDEX)
        );
        assert_eq!(runtime.live_palette(), &colors_before);
        let overlay = runtime.ui_overlay_rgba().to_vec();
        assert!(overlay.chunks_exact(4).any(|pixel| pixel == [0, 0, 0, 128]));
        assert!(overlay.chunks_exact(4).any(|pixel| pixel[3] == 255));

        // Compare every glyph pixel with the independently oracle-tested C font raster.
        let mut reference = vec![0; super::super::LOGICAL_FRAMEBUFFER_PIXEL_COUNT];
        for (row, label) in frame.rows.iter().zip(labels) {
            crate::native::bloodprg::draw_square_caps_text(
                &mut reference,
                &fonts,
                label,
                crate::native::bloodprg::FontPoint {
                    x: i32::from(row.position[0]),
                    y: i32::from(row.position[1]),
                },
                crate::native::bloodprg::FontVerticalBand {
                    top: 0,
                    bottom: 199,
                },
                row.color,
            )
            .unwrap();
        }
        for (&style, pixel) in reference.iter().zip(overlay.chunks_exact(4)) {
            if style == 0 {
                assert_ne!(pixel[3], 255);
                continue;
            }
            let color = runtime.data().default_vga_palette()[usize::from(style)];
            assert_eq!(
                pixel,
                [
                    (color[0] << 2) | (color[0] >> 4),
                    (color[1] << 2) | (color[1] >> 4),
                    (color[2] << 2) | (color[2] >> 4),
                    255
                ]
            );
        }

        runtime.clear_ui_overlay();
        runtime.live_palette_mut().fill([0, 63, 0]);
        let mut backend =
            RuntimeChoiceListBackend::new(&mut runtime, &fonts, ChoiceListPointer::default(), 0);
        backend.prepare_background(frame.rect);
        drop(backend);
        draw_choice_list_rows(&mut runtime, &labels, None, &frame).unwrap();
        assert_eq!(runtime.ui_overlay_rgba(), overlay);
    }
}
