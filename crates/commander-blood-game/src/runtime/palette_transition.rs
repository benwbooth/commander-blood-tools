//! Flat palette-transition ownership for the recovered game lifecycle.

use std::ops::RangeInclusive;

use commander_blood_formats::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};

use crate::native::bloodprg::{
    GameLifecycleState, IndexedGamePalette, PaletteInterpolationRequest, PalettePipelineError,
    PaletteTransitionState, PaletteUploadState, advance_palette_transition,
    interpolate_palette_range, take_palette_upload_request,
};

const INITIAL_TRANSITION_PERCENT: u16 = 100;
const INITIAL_TRANSITION_INCREMENT: u16 = u16::MIN;
const INITIAL_TRANSITION_COLOR: u8 = u8::MIN;
const INITIAL_DIRTY_FLAGS: u8 = u8::MIN;
const EMPTY_PALETTE: IndexedGamePalette = [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];

/// Complete typed inputs for one recovered zero-to-100 palette transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePaletteTransitionConfig {
    /// Palette approached as the transition percentage increases.
    pub source: IndexedGamePalette,
    /// Palette shown at transition percentage zero.
    pub target: IndexedGamePalette,
    /// Initial wrapping transition percentage.
    pub initial_percent: u16,
    /// Wrapping amount added on each game frame.
    pub increment: u16,
    /// Inclusive palette-index interval modified by the transition.
    pub colors: RangeInclusive<u8>,
}

/// Result of one lifecycle palette update and its upload gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimePaletteTransitionOutcome {
    /// Exact interpolation request emitted by the recovered transition step.
    pub interpolation: Option<PaletteInterpolationRequest>,
    /// Whether the original dirty gate requested a palette upload this frame.
    pub upload_requested: bool,
}

/// Persistent source, target, progress, and dirty state for palette fades.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePaletteTransition {
    source: IndexedGamePalette,
    target: IndexedGamePalette,
    state: PaletteTransitionState,
}

impl Default for RuntimePaletteTransition {
    fn default() -> Self {
        Self {
            source: EMPTY_PALETTE,
            target: EMPTY_PALETTE,
            state: PaletteTransitionState {
                percent: INITIAL_TRANSITION_PERCENT,
                increment: INITIAL_TRANSITION_INCREMENT,
                first: INITIAL_TRANSITION_COLOR,
                last: INITIAL_TRANSITION_COLOR,
                dirty_flags: INITIAL_DIRTY_FLAGS,
            },
        }
    }
}

impl RuntimePaletteTransition {
    /// Borrow the current recovered progress, range, and dirty state.
    pub const fn state(&self) -> &PaletteTransitionState {
        &self.state
    }

    /// Synchronize progress written by the recovered ship-depth compositor.
    pub fn set_progress_percent(&mut self, percent: u16) {
        self.state.percent = percent;
    }

    /// Replace all transition inputs after validating the typed color interval.
    pub fn configure(
        &mut self,
        config: RuntimePaletteTransitionConfig,
    ) -> Result<(), PalettePipelineError> {
        if config.colors.is_empty() {
            return Err(PalettePipelineError::InvalidPaletteRange {
                first: usize::from(*config.colors.start()),
                last: usize::from(*config.colors.end()),
            });
        }
        self.source = config.source;
        self.target = config.target;
        self.state.percent = config.initial_percent;
        self.state.increment = config.increment;
        self.state.first = *config.colors.start();
        self.state.last = *config.colors.end();
        Ok(())
    }

    /// Advance interpolation and apply the original post-upload latch clears.
    pub fn update(
        &mut self,
        live_palette: &mut IndexedGamePalette,
        lifecycle: &mut GameLifecycleState,
    ) -> Result<RuntimePaletteTransitionOutcome, PalettePipelineError> {
        let interpolation = advance_palette_transition(&mut self.state);
        if let Some(request) = interpolation {
            interpolate_palette_range(
                &self.source,
                &self.target,
                live_palette,
                request.percent,
                usize::from(request.first)..=usize::from(request.last),
            )?;
        }

        let mut upload = PaletteUploadState {
            dirty_flags: self.state.dirty_flags,
            primary_pressed: u8::from(lifecycle.primary_pointer_pressed),
            secondary_pressed: u8::from(lifecycle.secondary_pointer_pressed),
            press_pending: lifecycle.pointer_press_pending,
        };
        let upload_requested = take_palette_upload_request(&mut upload);
        self.state.dirty_flags = upload.dirty_flags;
        lifecycle.primary_pointer_pressed = upload.primary_pressed != u8::MIN;
        lifecycle.secondary_pointer_pressed = upload.secondary_pressed != u8::MIN;
        lifecycle.pointer_press_pending = upload.press_pending;

        Ok(RuntimePaletteTransitionOutcome {
            interpolation,
            upload_requested,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::OriginalGameDataPaths;

    const BLOODPRG_DATA_FILE_OFFSET: usize = 54_304;
    const TRANSITION_INCREMENT_DATA_OFFSET: usize = 21_069;
    const TRANSITION_PERCENT_DATA_OFFSET: usize = 21_071;
    const TRANSITION_TARGET_DATA_OFFSET: usize = 21_841;
    const TRANSITION_SOURCE_DATA_OFFSET: usize = 22_609;
    const TRANSITION_FIRST_DATA_OFFSET: usize = 23_377;
    const TRANSITION_LAST_DATA_OFFSET: usize = 23_378;
    const PALETTE_DIRTY_DATA_OFFSET: usize = 23_381;
    const ENCODED_WORD_SIZE: usize = 2;
    const ENCODED_PALETTE_SIZE: usize = PALETTE_ENTRY_COUNT * RGB_COMPONENT_COUNT;
    const TEST_COLOR_INDEX: u8 = 12;
    const TEST_INITIAL_PERCENT: u16 = u16::MIN;
    const TEST_INCREMENT: u16 = 25;
    const TEST_SOURCE_COLOR: [u8; RGB_COMPONENT_COUNT] = [60, 20, 4];
    const TEST_TARGET_COLOR: [u8; RGB_COMPONENT_COUNT] = [20, 4, 0];
    const TEST_INTERPOLATED_COLOR: [u8; RGB_COMPONENT_COUNT] = [30, 8, 1];
    const TEST_PENDING_PRESS_COUNT: u8 = 3;

    #[test]
    fn default_state_matches_the_executable_globals() {
        let Ok(paths) = OriginalGameDataPaths::discover(None) else {
            return;
        };
        let executable = std::fs::read(paths.executable()).unwrap();
        let transition = RuntimePaletteTransition::default();

        assert_eq!(
            read_executable_word(
                &executable,
                BLOODPRG_DATA_FILE_OFFSET + TRANSITION_INCREMENT_DATA_OFFSET,
            ),
            transition.state.increment
        );
        assert_eq!(
            read_executable_word(
                &executable,
                BLOODPRG_DATA_FILE_OFFSET + TRANSITION_PERCENT_DATA_OFFSET,
            ),
            transition.state.percent
        );
        assert_eq!(
            executable[BLOODPRG_DATA_FILE_OFFSET + TRANSITION_FIRST_DATA_OFFSET],
            transition.state.first
        );
        assert_eq!(
            executable[BLOODPRG_DATA_FILE_OFFSET + TRANSITION_LAST_DATA_OFFSET],
            transition.state.last
        );
        assert_eq!(
            executable[BLOODPRG_DATA_FILE_OFFSET + PALETTE_DIRTY_DATA_OFFSET],
            transition.state.dirty_flags
        );
        assert_palette_bytes(
            &executable,
            BLOODPRG_DATA_FILE_OFFSET + TRANSITION_SOURCE_DATA_OFFSET,
            &transition.source,
        );
        assert_palette_bytes(
            &executable,
            BLOODPRG_DATA_FILE_OFFSET + TRANSITION_TARGET_DATA_OFFSET,
            &transition.target,
        );
    }

    #[test]
    fn configured_transition_updates_live_colors_and_upload_latches() {
        let mut source = EMPTY_PALETTE;
        let mut target = EMPTY_PALETTE;
        source[usize::from(TEST_COLOR_INDEX)] = TEST_SOURCE_COLOR;
        target[usize::from(TEST_COLOR_INDEX)] = TEST_TARGET_COLOR;
        let mut transition = RuntimePaletteTransition::default();
        transition
            .configure(RuntimePaletteTransitionConfig {
                source,
                target,
                initial_percent: TEST_INITIAL_PERCENT,
                increment: TEST_INCREMENT,
                colors: TEST_COLOR_INDEX..=TEST_COLOR_INDEX,
            })
            .unwrap();
        let mut live = EMPTY_PALETTE;
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.primary_pointer_pressed = true;
        lifecycle.secondary_pointer_pressed = true;
        lifecycle.pointer_press_pending = TEST_PENDING_PRESS_COUNT;

        let outcome = transition.update(&mut live, &mut lifecycle).unwrap();

        assert_eq!(
            outcome.interpolation,
            Some(PaletteInterpolationRequest {
                percent: TEST_INCREMENT as i8,
                first: TEST_COLOR_INDEX,
                last: TEST_COLOR_INDEX,
            })
        );
        assert!(outcome.upload_requested);
        assert_eq!(live[usize::from(TEST_COLOR_INDEX)], TEST_INTERPOLATED_COLOR);
        assert_eq!(transition.state.percent, TEST_INCREMENT);
        assert_eq!(transition.state.dirty_flags, u8::MIN);
        assert!(!lifecycle.primary_pointer_pressed);
        assert!(lifecycle.secondary_pointer_pressed);
        assert_eq!(lifecycle.pointer_press_pending, u8::MIN);
    }

    #[test]
    fn completed_transition_preserves_clean_upload_latches() {
        let mut transition = RuntimePaletteTransition::default();
        let mut live = EMPTY_PALETTE;
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.primary_pointer_pressed = true;
        lifecycle.secondary_pointer_pressed = true;
        lifecycle.pointer_press_pending = TEST_PENDING_PRESS_COUNT;

        let outcome = transition.update(&mut live, &mut lifecycle).unwrap();

        assert_eq!(outcome.interpolation, None);
        assert!(!outcome.upload_requested);
        assert!(lifecycle.primary_pointer_pressed);
        assert!(lifecycle.secondary_pointer_pressed);
        assert_eq!(lifecycle.pointer_press_pending, TEST_PENDING_PRESS_COUNT);
    }

    fn read_executable_word(bytes: &[u8], offset: usize) -> u16 {
        u16::from_le_bytes(
            bytes[offset..offset + ENCODED_WORD_SIZE]
                .try_into()
                .unwrap(),
        )
    }

    fn assert_palette_bytes(executable: &[u8], offset: usize, palette: &IndexedGamePalette) {
        let encoded = &executable[offset..offset + ENCODED_PALETTE_SIZE];
        assert!(
            encoded
                .iter()
                .copied()
                .eq(palette.iter().flatten().copied())
        );
    }
}
