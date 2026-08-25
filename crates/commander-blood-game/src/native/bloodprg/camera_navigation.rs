//! Camera navigation activation over typed world and presentation state.

use commander_blood_formats::script::ScriptObjectKind;

const RGB_PALETTE_BYTES: usize = 768;
const PALETTE_TRANSITION_INCREMENT: u8 = 20;

/// Current location data consulted by camera navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CameraNavigationLocation {
    /// Decoded VAR object kind.
    pub kind: ScriptObjectKind,
    /// Number of available destination/access records.
    pub access_count: u16,
}

/// Semantic state of the camera-navigation actor slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CameraNavigationSlot {
    /// A locked slot cannot be armed by an unavailable destination.
    pub locked: bool,
    /// The actor slot is ready to present the unavailable-destination state.
    pub ready: bool,
}

/// Presentation state published after the navigation region accepts input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraNavigationPresentation {
    /// Camera navigation did not publish a new actor state.
    #[default]
    Unchanged,
    /// The destination region was selected.
    DestinationSelected,
}

/// Ship mode entered when the selected location has an available destination.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraNavigationShipMode {
    /// No camera-navigation transition was requested.
    #[default]
    Unchanged,
    /// Prepare the ship view for destination entry.
    EnteringDestination,
}

/// Flat owned palette state used by the camera transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraNavigationPaletteTransition {
    /// Black transition source palette.
    pub source: [u8; RGB_PALETTE_BYTES],
    /// Snapshot of the live target palette.
    pub target: [u8; RGB_PALETTE_BYTES],
    /// Initial interpolation percentage.
    pub percent: u8,
    /// Percentage increment applied by each transition update.
    pub increment: u8,
    /// First palette entry included in the transition.
    pub first_color: u8,
    /// Last palette entry included in the transition.
    pub last_color: u8,
}

impl CameraNavigationPaletteTransition {
    fn from_live_palette(live_palette: &[u8; RGB_PALETTE_BYTES]) -> Self {
        Self {
            source: [u8::MIN; RGB_PALETTE_BYTES],
            target: *live_palette,
            percent: u8::MIN,
            increment: PALETTE_TRANSITION_INCREMENT,
            first_color: u8::MIN,
            last_color: u8::MAX,
        }
    }
}

/// Mutable semantic state affected by a successful navigation-region poll.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CameraNavigationState {
    camera_view_active: bool,
    approach_active: bool,
    presentation: CameraNavigationPresentation,
    redraw_requested: bool,
    palette_transition: Option<CameraNavigationPaletteTransition>,
    ui_active: bool,
    ship_mode: CameraNavigationShipMode,
    hud_initialization_pending: bool,
    dialogue_hold_complete: bool,
    scene_dispatch_blocked: bool,
    ship_depth_offset: u16,
    depth_opening: u16,
    hud_initialized: bool,
}

impl CameraNavigationState {
    /// Set whether the camera view already owns input.
    pub fn set_camera_view_active(&mut self, active: bool) {
        self.camera_view_active = active;
    }

    /// Set whether a camera approach is already in progress.
    pub fn set_approach_active(&mut self, active: bool) {
        self.approach_active = active;
    }

    /// Return the actor-presentation state published by this update.
    pub const fn presentation(&self) -> CameraNavigationPresentation {
        self.presentation
    }

    /// Return whether the bridge needs a redraw.
    pub const fn redraw_requested(&self) -> bool {
        self.redraw_requested
    }

    /// Return the initialized flat palette transition, when present.
    pub const fn palette_transition(&self) -> Option<&CameraNavigationPaletteTransition> {
        self.palette_transition.as_ref()
    }

    /// Return the ship mode selected by camera navigation.
    pub const fn ship_mode(&self) -> CameraNavigationShipMode {
        self.ship_mode
    }

    /// Return whether the 3D HUD must initialize for the transition.
    pub const fn hud_initialization_pending(&self) -> bool {
        self.hud_initialization_pending
    }

    /// Return whether normal bridge UI remains active.
    pub const fn ui_active(&self) -> bool {
        self.ui_active
    }

    /// Return whether dialogue completion remains latched.
    pub const fn dialogue_hold_complete(&self) -> bool {
        self.dialogue_hold_complete
    }

    /// Return whether 3D scene dispatch is blocked.
    pub const fn scene_dispatch_blocked(&self) -> bool {
        self.scene_dispatch_blocked
    }

    /// Return the reset ship-depth offset.
    pub const fn ship_depth_offset(&self) -> u16 {
        self.ship_depth_offset
    }

    /// Return the reset depth-opening state.
    pub const fn depth_opening(&self) -> u16 {
        self.depth_opening
    }

    /// Return whether the 3D HUD is already initialized.
    pub const fn hud_initialized(&self) -> bool {
        self.hud_initialized
    }
}

/// UI callback used to poll the camera-navigation hit region.
pub trait CameraNavigationRegionPoll {
    /// Poll the destination region.
    ///
    /// The callback may update location availability and slot state, matching
    /// changes made by the input/event pump before the routine rereads them.
    fn poll_destination_region(
        &mut self,
        location: &mut CameraNavigationLocation,
        slot: &mut CameraNavigationSlot,
    ) -> bool;
}

/// Terminal path taken by one camera-navigation update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraNavigationOutcome {
    /// An active camera view owns input.
    CameraViewActive,
    /// A camera approach is already in progress.
    ApproachActive,
    /// The current object is not a supported destination kind.
    UnsupportedLocation,
    /// The destination region did not accept input.
    RegionNotSelected,
    /// The location has no available destination; its slot may have been armed.
    DestinationUnavailable,
    /// Palette and ship state were initialized for destination entry.
    TransitionStarted,
}

/// Poll camera navigation and initialize an available destination transition.
///
/// This translates `camera_nav_update` at BLOODPRG routine offset `0x00792D`.
/// Decoded objects, owned palettes, and semantic slot/ship state replace record
/// arena offsets, memory copies, packed slot flags, and numeric state words.
pub fn update_camera_navigation<Poll: CameraNavigationRegionPoll>(
    state: &mut CameraNavigationState,
    location: &mut CameraNavigationLocation,
    slot: &mut CameraNavigationSlot,
    live_palette: &[u8; RGB_PALETTE_BYTES],
    poll: &mut Poll,
) -> CameraNavigationOutcome {
    if state.camera_view_active {
        return CameraNavigationOutcome::CameraViewActive;
    }
    if state.approach_active {
        return CameraNavigationOutcome::ApproachActive;
    }
    if !matches!(
        location.kind,
        ScriptObjectKind::CelestialBody | ScriptObjectKind::NavigationEntity
    ) {
        return CameraNavigationOutcome::UnsupportedLocation;
    }
    if !poll.poll_destination_region(location, slot) {
        return CameraNavigationOutcome::RegionNotSelected;
    }

    state.presentation = CameraNavigationPresentation::DestinationSelected;
    if location.access_count == u16::MIN {
        state.redraw_requested = true;
        if !slot.locked {
            slot.ready = true;
        }
        return CameraNavigationOutcome::DestinationUnavailable;
    }

    state.palette_transition = Some(CameraNavigationPaletteTransition::from_live_palette(
        live_palette,
    ));
    state.ui_active = false;
    state.ship_mode = CameraNavigationShipMode::EnteringDestination;
    state.hud_initialization_pending = true;
    state.dialogue_hold_complete = false;
    state.scene_dispatch_blocked = false;
    state.ship_depth_offset = u16::MIN;
    state.depth_opening = u16::MIN;
    state.hud_initialized = false;
    CameraNavigationOutcome::TransitionStarted
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 16;

    #[derive(Deserialize)]
    struct NavigationOracle {
        name: String,
        camera_active: u8,
        approach_phase: u8,
        kind: u16,
        ui_result: Option<u16>,
        access_count_before: u16,
        access_count_after: u16,
        slot_flags_before: u8,
        slot_flags_after: u8,
        helper_called: bool,
        full_transition: bool,
        no_destination: bool,
    }

    struct OraclePoll {
        called: bool,
        selected: bool,
        access_count_after: u16,
        slot_after: CameraNavigationSlot,
    }

    impl CameraNavigationRegionPoll for OraclePoll {
        fn poll_destination_region(
            &mut self,
            location: &mut CameraNavigationLocation,
            slot: &mut CameraNavigationSlot,
        ) -> bool {
            self.called = true;
            location.access_count = self.access_count_after;
            *slot = self.slot_after;
            self.selected
        }
    }

    #[test]
    fn update_matches_every_original_semantic_vector() {
        let vectors: Vec<NavigationOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_792d_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);
        let live_palette = std::array::from_fn(|index| index as u8);

        for vector in vectors {
            let mut state = CameraNavigationState::default();
            state.set_camera_view_active(vector.camera_active & 1 != u8::MIN);
            state.set_approach_active(vector.approach_phase != u8::MIN);
            let mut location = CameraNavigationLocation {
                kind: oracle_kind(vector.kind),
                access_count: vector.access_count_before,
            };
            let mut slot = oracle_slot(vector.slot_flags_before);
            let mut poll = OraclePoll {
                called: false,
                selected: vector.ui_result == Some(31),
                access_count_after: vector.access_count_after,
                slot_after: oracle_slot(vector.slot_flags_after),
            };

            let outcome = update_camera_navigation(
                &mut state,
                &mut location,
                &mut slot,
                &live_palette,
                &mut poll,
            );

            assert_eq!(poll.called, vector.helper_called, "{}", vector.name);
            assert_eq!(
                location.access_count, vector.access_count_after,
                "{}",
                vector.name
            );
            assert_eq!(
                slot,
                oracle_slot(vector.slot_flags_after),
                "{}",
                vector.name
            );
            assert_eq!(
                outcome == CameraNavigationOutcome::TransitionStarted,
                vector.full_transition,
                "{}",
                vector.name
            );
            assert_eq!(
                outcome == CameraNavigationOutcome::DestinationUnavailable,
                vector.no_destination,
                "{}",
                vector.name
            );
            if vector.full_transition {
                let transition = state.palette_transition().unwrap();
                assert_eq!(
                    transition.source,
                    [u8::MIN; RGB_PALETTE_BYTES],
                    "{}",
                    vector.name
                );
                assert_eq!(transition.target, live_palette, "{}", vector.name);
                assert_eq!(
                    transition.increment, PALETTE_TRANSITION_INCREMENT,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    state.ship_mode(),
                    CameraNavigationShipMode::EnteringDestination
                );
                assert!(state.hud_initialization_pending());
                assert!(!state.ui_active());
                assert!(!state.dialogue_hold_complete());
                assert!(!state.scene_dispatch_blocked());
                assert_eq!(state.ship_depth_offset(), u16::MIN);
                assert_eq!(state.depth_opening(), u16::MIN);
                assert!(!state.hud_initialized());
            }
        }
    }

    fn oracle_kind(kind: u16) -> ScriptObjectKind {
        match kind {
            0 | 256 => ScriptObjectKind::BlackHole,
            16 => ScriptObjectKind::NavigationEntity,
            8 | 24 => ScriptObjectKind::CelestialBody,
            _ => panic!("unknown camera-navigation oracle kind {kind}"),
        }
    }

    fn oracle_slot(flags: u8) -> CameraNavigationSlot {
        CameraNavigationSlot {
            locked: flags & 2 != u8::MIN,
            ready: flags & 8 != u8::MIN,
        }
    }
}
