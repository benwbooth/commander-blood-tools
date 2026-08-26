//! Concrete flat renderer and logical-pointer host for the confirmation modal.

use anyhow::{Context, Result};
use commander_blood_formats::bloodprg::{BloodprgConfirmDialogRegions, BloodprgHitRectangle};

use crate::native::bloodprg::{
    BridgeSpriteRect, ConfirmDialogFrame, ConfirmDialogHits, ConfirmDialogOutcome,
    ConfirmDialogState, FontPoint, FontVerticalBand, GameLifecycleState, PresentationHitRectangle,
    PrimaryPointerSample, RasterPoint, RasterSpanPaint, draw_rect_outline, draw_square_caps_text,
    fill_framebuffer_rect, primary_pointer_hits_region, update_confirm_dialog,
};

use super::OriginalGameRuntime;

const LOGICAL_DISPLAY_CLIP: BridgeSpriteRect = BridgeSpriteRect {
    left: 0,
    right: 320,
    top: 0,
    bottom: 200,
};
const LOGICAL_FONT_BAND: FontVerticalBand = FontVerticalBand {
    top: 0,
    bottom: 199,
};
const MODAL_UI_FLAG: u16 = 1 << 2;

/// Persistent confirmation state and executable-authored response regions.
pub struct RuntimeConfirmDialog {
    state: ConfirmDialogState,
    regions: BloodprgConfirmDialogRegions,
}

impl RuntimeConfirmDialog {
    /// Construct an inactive dialog around decoded executable geometry.
    pub const fn new(regions: BloodprgConfirmDialogRegions) -> Self {
        Self {
            state: ConfirmDialogState {
                navigation_choice_gate: u8::MIN,
                navigation_state: u16::MIN,
                ui_flags: u16::MIN,
                primary_pointer_pressed: false,
                pointer_press_pending: false,
            },
            regions,
        }
    }

    /// Borrow state shared with the translated navigation-choice coordinator.
    pub const fn state(&self) -> &ConfirmDialogState {
        &self.state
    }

    /// Mutably borrow state shared with the translated navigation-choice coordinator.
    pub fn state_mut(&mut self) -> &mut ConfirmDialogState {
        &mut self.state
    }

    /// Advance, draw, and synchronize one exact confirmation-dialog frame.
    pub fn update(
        &mut self,
        runtime: &mut OriginalGameRuntime,
        lifecycle: &mut GameLifecycleState,
        pointer_position: [i16; 2],
    ) -> Result<ConfirmDialogOutcome> {
        self.state.primary_pointer_pressed = lifecycle.primary_pointer_pressed;
        self.state.pointer_press_pending = lifecycle.pointer_press_pending != u8::MIN;
        if lifecycle.modal_ui_busy() {
            self.state.ui_flags |= MODAL_UI_FLAG;
        } else {
            self.state.ui_flags &= !MODAL_UI_FLAG;
        }

        let pointer = PrimaryPointerSample {
            primary_pressed: lifecycle.primary_pointer_pressed,
            position: pointer_position,
        };
        let outcome = update_confirm_dialog(
            &mut self.state,
            ConfirmDialogHits {
                yes: hit(pointer, self.regions.yes),
                no: hit(pointer, self.regions.no),
            },
        );

        let frame = match outcome {
            ConfirmDialogOutcome::Inactive => None,
            ConfirmDialogOutcome::AwaitingChoice(frame)
            | ConfirmDialogOutcome::Confirmed(frame)
            | ConfirmDialogOutcome::Cancelled(frame) => Some(frame),
        };
        if let Some(frame) = frame {
            draw_frame(runtime, frame)?;
        }

        lifecycle.primary_pointer_pressed = self.state.primary_pointer_pressed;
        if !self.state.pointer_press_pending {
            lifecycle.pointer_press_pending = u8::MIN;
        }
        if !matches!(outcome, ConfirmDialogOutcome::Inactive) {
            lifecycle.set_modal_ui_busy(self.state.ui_flags & MODAL_UI_FLAG != u16::MIN);
        }
        Ok(outcome)
    }
}

fn hit(pointer: PrimaryPointerSample, region: BloodprgHitRectangle) -> bool {
    primary_pointer_hits_region(
        pointer,
        PresentationHitRectangle::new(region.origin, region.size),
    )
}

fn draw_frame(runtime: &mut OriginalGameRuntime, frame: ConfirmDialogFrame) -> Result<()> {
    let fonts = runtime.data().font_resources().clone();
    let pixels = runtime.front_buffer_mut().pixels_mut();
    let origin = RasterPoint {
        x: i32::from(frame.panel.x),
        y: i32::from(frame.panel.y),
    };
    fill_framebuffer_rect(
        pixels,
        LOGICAL_DISPLAY_CLIP,
        origin,
        frame.panel.width,
        frame.panel.height,
        frame.background_palette_index,
    )
    .context("filling the confirmation dialog")?;
    draw_rect_outline(
        pixels,
        LOGICAL_DISPLAY_CLIP,
        origin,
        frame.panel.width,
        frame.panel.height,
        RasterSpanPaint::Solid(frame.foreground_palette_index),
    )
    .context("outlining the confirmation dialog")?;
    for label in frame.labels {
        draw_square_caps_text(
            pixels,
            &fonts,
            label.text,
            FontPoint {
                x: i32::from(label.position[0]),
                y: i32::from(label.position[1]),
            },
            LOGICAL_FONT_BAND,
            frame.foreground_palette_index,
        )
        .context("drawing a confirmation dialog label")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{OriginalGameData, OriginalGameDataPaths};

    const TEST_REGIONS: BloodprgConfirmDialogRegions = BloodprgConfirmDialogRegions {
        yes: BloodprgHitRectangle {
            origin: [120, 105],
            size: [30, 10],
        },
        no: BloodprgHitRectangle {
            origin: [180, 105],
            size: [20, 10],
        },
    };

    #[test]
    fn inactive_dialog_starts_without_navigation_or_pointer_state() {
        let dialog = RuntimeConfirmDialog::new(TEST_REGIONS);
        assert_eq!(dialog.state().navigation_choice_gate, u8::MIN);
        assert_eq!(dialog.state().navigation_state, u16::MIN);
        assert!(!dialog.state().primary_pointer_pressed);
        assert!(!dialog.state().pointer_press_pending);
    }

    #[test]
    fn decoded_regions_use_the_recovered_inclusive_hit_test() {
        let pressed = PrimaryPointerSample {
            primary_pressed: true,
            position: [150, 115],
        };
        assert!(hit(pressed, TEST_REGIONS.yes));
        assert!(!hit(pressed, TEST_REGIONS.no));
    }

    #[test]
    fn affirmative_selection_draws_and_updates_flat_lifecycle_state() {
        let Ok(paths) = OriginalGameDataPaths::discover(None) else {
            return;
        };
        let data = OriginalGameData::load(paths).unwrap();
        let mut runtime = OriginalGameRuntime::new(data);
        let mut dialog = RuntimeConfirmDialog::new(TEST_REGIONS);
        dialog.state_mut().navigation_choice_gate = 2;
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.primary_pointer_pressed = true;
        lifecycle.pointer_press_pending = 1;

        let outcome = dialog
            .update(&mut runtime, &mut lifecycle, TEST_REGIONS.yes.origin)
            .unwrap();

        assert!(matches!(outcome, ConfirmDialogOutcome::Confirmed(_)));
        assert_eq!(dialog.state().navigation_choice_gate, 1);
        assert!(lifecycle.modal_ui_busy());
        assert!(
            runtime
                .front_buffer()
                .pixels()
                .iter()
                .any(|pixel| *pixel != u8::MIN)
        );
    }
}
