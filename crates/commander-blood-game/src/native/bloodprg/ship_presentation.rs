//! Top-level ship presentation phase coordinator.

use super::ShipViewEntityId;

const PRESENTATION_ACTIVE: u16 = 1;
const DIALOGUE_PHASE: u16 = 2;
const HUD_PHASE: u16 = 4;
const TRAVEL_PHASE: u16 = 8;
const NAVIGATION_PHASE: u16 = 16;
const PHASE_MASK: u16 = DIALOGUE_PHASE | HUD_PHASE | TRAVEL_PHASE | NAVIGATION_PHASE;
const PHASE_READY: u8 = 1;
const PRESENTATION_GATE_ACTIVE: u16 = 1;
const HUD_INITIALIZATION_PENDING: u8 = 1;
const TRAVEL_REDRAW_PENDING: u8 = 1;
const DIALOGUE_FIRST_LINE: u16 = 4;
const DIALOGUE_LINE_END: u16 = 6;
const TRAVEL_REDRAW_STATE: u16 = PRESENTATION_ACTIVE | NAVIGATION_PHASE;
const HUD_ACTIVE_STATE: u16 = PRESENTATION_ACTIVE | HUD_PHASE;
const TRAVEL_STATUS_LINE: u16 = 3;
const TRANSITION_COMPLETE_PERCENT: u16 = 100;
const DIALOGUE_ENTITY: ShipViewEntityId = ShipViewEntityId::new(4);
const SHIP_VIEW_TRANSITION_ENTITY: ShipViewEntityId = ShipViewEntityId::new(31);

/// Mutable state read and written by the ship presentation FSM.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipPresentationState {
    /// Active and phase bits. Unknown high bits remain intact during setup.
    pub flags: u16,
    /// Whether bridge entity geometry has been snapshotted for clipping.
    pub clip_snapshot_ready: bool,
    /// Shared UI state cleared when ship presentation initializes.
    pub ui_state: u16,
    /// Current line in the automatic dialogue cycle.
    pub dialogue_cycle_line: u16,
    /// Whether a scene dispatcher currently blocks bridge presentation.
    pub scene_dispatch_blocked: bool,
    /// Current ship-view depth offset.
    pub depth_offset: u16,
    /// Bit flags controlling the opening depth transition.
    pub depth_opening_flags: u8,
    /// Dialogue phase-completion flags.
    pub dialogue_phase_ready: u8,
    /// Active C2 presentation gate.
    pub presentation_gate: u16,
    /// Whether HUD initialization is pending a complete transition.
    pub hud_initialization_pending: u8,
    /// Current palette-transition completion percentage.
    pub transition_percent: u16,
    /// Travel redraw request flags.
    pub bridge_redraw_pending: u8,
    /// Script line requested by the current presentation phase.
    pub active_line: u16,
}

impl ShipPresentationState {
    /// Return whether the ship HUD phase suppresses the ordinary subtitle hold.
    pub const fn hud_active(&self) -> bool {
        self.flags & HUD_PHASE != u16::MIN
    }
}

/// Subsystems called by the ship presentation coordinator.
pub trait ShipPresentationHost {
    /// Typed scene link forwarded to the scene dispatcher.
    type SceneLink;

    /// Advance one fixed bridge entity's flag state.
    fn transition_entity(&mut self, entity: ShipViewEntityId);
    /// Advance the separately recovered depth transition.
    fn advance_depth(&mut self);
    /// Submit the separately recovered two-band presentation effect.
    fn compose_depth_band(&mut self);
    /// Dispatch the current linked scene before phase-specific work.
    fn dispatch_scene(&mut self, scene_link: &Self::SceneLink);
    /// Initialize or advance the ship HUD.
    fn update_hud(&mut self);
    /// Clear the travelling display band.
    fn clear_travel_band(&mut self);
    /// Advance navigation presentation state.
    fn update_navigation(&mut self);
}

/// Terminal path selected by one ship presentation update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipPresentationOutcome {
    /// Presentation bit zero was clear.
    Inactive,
    /// Presentation state was initialized and no phase body ran.
    Initialized,
    /// Dialogue is waiting for the C2 presentation gate.
    DialogueBlocked,
    /// One automatic dialogue line was published.
    DialogueLinePublished,
    /// Dialogue closed to the HUD state without another phase this frame.
    DialogueClosed,
    /// HUD handling had precedence for this frame.
    Hud,
    /// Travel handling had precedence for this frame.
    Travel,
    /// Navigation handling ran.
    Navigation,
    /// Common depth, band, and scene work ran with no recognized phase bit.
    CommonOnly,
}

/// Run native BLOODPRG routine `0x00AFA0` over typed state.
///
/// Phase precedence is evaluated from the flag word captured on entry. This is
/// significant when dialogue completion writes the live HUD state during a
/// frame that also carried another phase bit.
pub fn update_ship_presentation<H: ShipPresentationHost>(
    state: &mut ShipPresentationState,
    scene_link: &H::SceneLink,
    host: &mut H,
) -> ShipPresentationOutcome {
    let entry_flags = state.flags;
    if entry_flags & PRESENTATION_ACTIVE == u16::MIN {
        return ShipPresentationOutcome::Inactive;
    }

    state.clip_snapshot_ready = true;
    if entry_flags & PHASE_MASK == u16::MIN {
        host.transition_entity(DIALOGUE_ENTITY);
        host.transition_entity(SHIP_VIEW_TRANSITION_ENTITY);
        state.ui_state = u16::MIN;
        state.flags |= DIALOGUE_PHASE;
        state.dialogue_cycle_line = DIALOGUE_FIRST_LINE;
        state.scene_dispatch_blocked = false;
        state.depth_offset = u16::MIN;
        state.depth_opening_flags = u8::MIN;
        return ShipPresentationOutcome::Initialized;
    }

    host.advance_depth();
    host.compose_depth_band();
    host.dispatch_scene(scene_link);

    if entry_flags & DIALOGUE_PHASE != u16::MIN {
        if state.dialogue_phase_ready & PHASE_READY == u8::MIN {
            if state.presentation_gate & PRESENTATION_GATE_ACTIVE != u16::MIN {
                return ShipPresentationOutcome::DialogueBlocked;
            }

            let line = state.dialogue_cycle_line;
            if line != u16::MIN {
                state.active_line = line;
                let next_line = line.wrapping_add(1);
                state.dialogue_cycle_line = if next_line == DIALOGUE_LINE_END {
                    u16::MIN
                } else {
                    next_line
                };
                return ShipPresentationOutcome::DialogueLinePublished;
            }

            state.dialogue_phase_ready = u8::MIN;
            state.flags = HUD_ACTIVE_STATE;
            return ShipPresentationOutcome::DialogueClosed;
        }

        state.dialogue_phase_ready = u8::MIN;
        state.flags = HUD_ACTIVE_STATE;
    }

    if entry_flags & HUD_PHASE != u16::MIN {
        if state.hud_initialization_pending & HUD_INITIALIZATION_PENDING == u8::MIN
            || state.transition_percent == TRANSITION_COMPLETE_PERCENT
        {
            host.update_hud();
        }
        return ShipPresentationOutcome::Hud;
    }

    if entry_flags & TRAVEL_PHASE != u16::MIN {
        if state.bridge_redraw_pending & TRAVEL_REDRAW_PENDING != u8::MIN {
            state.flags = TRAVEL_REDRAW_STATE;
            host.clear_travel_band();
        } else if state.presentation_gate & PRESENTATION_GATE_ACTIVE == u16::MIN {
            state.active_line = TRAVEL_STATUS_LINE;
            state.bridge_redraw_pending = u8::MIN;
        }
        return ShipPresentationOutcome::Travel;
    }

    if entry_flags & NAVIGATION_PHASE != u16::MIN {
        host.update_navigation();
        return ShipPresentationOutcome::Navigation;
    }

    ShipPresentationOutcome::CommonOnly
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const CLIP_SNAPSHOT_ADDRESS: usize = 21_065;
    const UI_STATE_ADDRESS: usize = 10_131;
    const FLAGS_ADDRESS: usize = 9_459;
    const DIALOGUE_CYCLE_ADDRESS: usize = 9_461;
    const SCENE_BLOCKED_ADDRESS: usize = 9_517;
    const DEPTH_OFFSET_ADDRESS: usize = 9_511;
    const DEPTH_OPENING_ADDRESS: usize = 9_519;
    const DIALOGUE_READY_ADDRESS: usize = 9_524;
    const REDRAW_PENDING_ADDRESS: usize = 10_200;
    const ACTIVE_LINE_ADDRESS: usize = 26_504;

    #[derive(Deserialize)]
    struct PresentationVector {
        name: String,
        state_before: u16,
        dialogue_cycle_before: u16,
        ready: u8,
        presentation_gate: u16,
        hud_pending: u8,
        transition_percent: u16,
        redraw: u8,
        calls: Vec<CallVector>,
        writes: Vec<[u32; 3]>,
    }

    #[derive(Deserialize)]
    struct CallVector {
        name: String,
    }

    #[derive(Default)]
    struct RecordingHost {
        calls: Vec<String>,
    }

    impl ShipPresentationHost for RecordingHost {
        type SceneLink = u16;

        fn transition_entity(&mut self, entity: ShipViewEntityId) {
            self.calls.push(format!("entity:{}", entity.value()));
        }

        fn advance_depth(&mut self) {
            self.calls.push("depth".to_owned());
        }

        fn compose_depth_band(&mut self) {
            self.calls.push("band".to_owned());
        }

        fn dispatch_scene(&mut self, _scene_link: &Self::SceneLink) {
            self.calls.push("dispatch".to_owned());
        }

        fn update_hud(&mut self) {
            self.calls.push("hud".to_owned());
        }

        fn clear_travel_band(&mut self) {
            self.calls.push("fill".to_owned());
        }

        fn update_navigation(&mut self) {
            self.calls.push("nav".to_owned());
        }
    }

    #[test]
    fn fsm_matches_every_original_dispatch_vector() {
        let vectors: Vec<PresentationVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_afa0_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), 20);
        for vector in vectors {
            let mut state = ShipPresentationState {
                flags: vector.state_before,
                clip_snapshot_ready: false,
                ui_state: 43_690,
                dialogue_cycle_line: vector.dialogue_cycle_before,
                scene_dispatch_blocked: true,
                depth_offset: 48_059,
                depth_opening_flags: 204,
                dialogue_phase_ready: vector.ready,
                presentation_gate: vector.presentation_gate,
                hud_initialization_pending: vector.hud_pending,
                transition_percent: vector.transition_percent,
                bridge_redraw_pending: vector.redraw,
                active_line: 52_428,
            };
            let mut expected = state;
            apply_expected_writes(&mut expected, &vector.writes);
            let mut host = RecordingHost::default();
            update_ship_presentation(&mut state, &17, &mut host);

            assert_eq!(state, expected, "{}", vector.name);
            assert_eq!(
                host.calls,
                vector
                    .calls
                    .iter()
                    .map(|call| call.name.clone())
                    .collect::<Vec<_>>(),
                "{}",
                vector.name
            );
        }
    }

    fn apply_expected_writes(state: &mut ShipPresentationState, writes: &[[u32; 3]]) {
        for [address, _size, value] in writes {
            match *address as usize {
                CLIP_SNAPSHOT_ADDRESS => state.clip_snapshot_ready = *value != 0,
                UI_STATE_ADDRESS => state.ui_state = *value as u16,
                FLAGS_ADDRESS => state.flags = *value as u16,
                DIALOGUE_CYCLE_ADDRESS => state.dialogue_cycle_line = *value as u16,
                SCENE_BLOCKED_ADDRESS => state.scene_dispatch_blocked = *value != 0,
                DEPTH_OFFSET_ADDRESS => state.depth_offset = *value as u16,
                DEPTH_OPENING_ADDRESS => state.depth_opening_flags = *value as u8,
                DIALOGUE_READY_ADDRESS => state.dialogue_phase_ready = *value as u8,
                REDRAW_PENDING_ADDRESS => state.bridge_redraw_pending = *value as u8,
                ACTIVE_LINE_ADDRESS => state.active_line = *value as u16,
                _ => panic!("unknown original write address {address}"),
            }
        }
    }
}
