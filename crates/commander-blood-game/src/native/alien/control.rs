//! Mouse-driven camera control shared by the three alien XDB overlays.

const DRIVER_CENTER_X: u16 = 320;
const DRIVER_CENTER_Y: u16 = 512;
const HORIZONTAL_INPUT_DIVISOR_SHIFT: u32 = 1;
const HORIZONTAL_FILTER_SHIFT: u32 = 1;
const SECONDARY_PAN_SCALE_SHIFT: u32 = 3;
const SECONDARY_PAN_FILTER_SHIFT: u32 = 1;
const VERTICAL_INPUT_SCALE_SHIFT: u32 = 1;
const PITCH_FILTER_SHIFT: u32 = 4;
const DEPTH_DAMPING_SHIFT: u32 = 3;
const DEAD_ZONE_HALF_WIDTH: i16 = 5;
const DEAD_ZONE_WIDTH: i16 = DEAD_ZONE_HALF_WIDTH * 2;
const PRIMARY_MOUSE_BUTTON: u16 = 0x0001;
const SECONDARY_MOUSE_BUTTON: u16 = 0x0002;
const AMER_INTERACTION_SIGNAL: u16 = 0x0001;
const CURSOR_UP_KEY: u16 = 0x4800;
const CURSOR_DOWN_KEY: u16 = 0x5000;
const ASCII_SPACE: u8 = 0x20;
const INTERACTION_REQUEST_FLAG: u16 = 0x0010;
const BUTTON_DEPTH_ACCELERATION: i16 = 10;
const SINGLE_DEPTH_UNIT: i16 = 1;
const KEY_DEPTH_STEP: i16 = 8;
const CONTROL_DEPTH_IMPULSE: i16 = -100;
const CONTROL_DEPTH_ADJUSTMENT: i16 = 64;
const DEPTH_DAMPING_THRESHOLD: i16 = -8;

/// Alien scene whose native controller rules are active.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienSpecies {
    /// AMER overlay behavior.
    Amer,
    /// CROOLIS overlay behavior.
    Croolis,
    /// SCRUT overlay behavior.
    Scrut,
}

impl AlienSpecies {
    fn uses_amer_input_policy(self) -> bool {
        self == Self::Amer
    }
}

/// Raw mouse sample in the alien scenes' authored 640-by-1024 control range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienMouseSample {
    /// Horizontal driver coordinate centered at 320.
    pub x: u16,
    /// Vertical driver coordinate centered at 512.
    pub y: u16,
    /// Original low-bit button mask; bit zero is primary and bit one secondary.
    pub buttons: u16,
}

/// Discrete keyboard or interaction action recognized during a camera step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienInputAction {
    /// No recognized discrete action.
    None,
    /// Cursor-up increased depth velocity.
    IncreaseDepth,
    /// Cursor-down decreased depth velocity.
    DecreaseDepth,
    /// Space requested the alien interaction associated with the current scene.
    Interact,
}

/// Observable result of one recovered mouse-camera update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienCameraStep {
    /// Signed cursor displacement from the driver center.
    pub centered_cursor: [i16; 2],
    /// Discrete key action handled by this update.
    pub action: AlienInputAction,
}

/// Persistent fixed-point camera and input state shared by alien scene frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienCameraControl {
    /// Smoothed horizontal mouse delta.
    pub horizontal_filter: i16,
    /// Camera pitch accumulator.
    pub pitch: i16,
    /// Primary camera pan accumulator.
    pub pan: i16,
    /// Smoothed secondary pan accumulator.
    pub secondary_pan: i16,
    /// Forward/backward camera velocity.
    pub depth_velocity: i16,
    /// Interaction signal published by alien behavior callbacks.
    pub interaction_signal: u16,
    /// Pending original keyboard event, including its scan code.
    pub key_event: u16,
    /// Scene action flags updated by recognized input.
    pub scene_flags: u16,
}

impl AlienCameraControl {
    /// Queue one semantic keyboard action for the next recovered input step.
    pub fn queue_action(&mut self, action: AlienInputAction) {
        self.key_event = match action {
            AlienInputAction::None => u16::MIN,
            AlienInputAction::IncreaseDepth => CURSOR_UP_KEY,
            AlienInputAction::DecreaseDepth => CURSOR_DOWN_KEY,
            AlienInputAction::Interact => u16::from(ASCII_SPACE),
        };
    }

    /// Apply the recovered mouse-camera routine for one alien scene frame.
    ///
    /// All arithmetic intentionally wraps at 16 bits because those accumulators
    /// are part of the game rules. The input coordinates are ordinary values;
    /// no DOS address or machine-register state is represented here.
    pub fn step(&mut self, species: AlienSpecies, mouse: AlienMouseSample) -> AlienCameraStep {
        let centered_x = mouse.x.wrapping_sub(DRIVER_CENTER_X) as i16;
        let centered_y = mouse.y.wrapping_sub(DRIVER_CENTER_Y) as i16;

        let horizontal = dead_zone(centered_x >> HORIZONTAL_INPUT_DIVISOR_SHIFT);
        let horizontal = horizontal.wrapping_sub(self.horizontal_filter) >> HORIZONTAL_FILTER_SHIFT;
        self.horizontal_filter = horizontal;
        self.pan = self.pan.wrapping_add(horizontal);

        let secondary_delta = horizontal
            .wrapping_shl(SECONDARY_PAN_SCALE_SHIFT)
            .wrapping_sub(self.secondary_pan)
            >> SECONDARY_PAN_FILTER_SHIFT;
        self.secondary_pan = self.secondary_pan.wrapping_add(secondary_delta);

        let vertical = dead_zone(centered_y.wrapping_neg());
        let vertical = vertical
            .wrapping_shl(VERTICAL_INPUT_SCALE_SHIFT)
            .wrapping_sub(self.pitch)
            >> PITCH_FILTER_SHIFT;
        self.pitch = self.pitch.wrapping_add(vertical);

        if mouse.buttons & PRIMARY_MOUSE_BUTTON != u16::MIN {
            self.depth_velocity = self.depth_velocity.wrapping_add(BUTTON_DEPTH_ACCELERATION);
        }
        if mouse.buttons & SECONDARY_MOUSE_BUTTON != u16::MIN {
            self.depth_velocity = self
                .depth_velocity
                .wrapping_sub(self.depth_velocity >> DEPTH_DAMPING_SHIFT)
                .wrapping_sub(SINGLE_DEPTH_UNIT);
        }

        let interaction_active = if species.uses_amer_input_policy() {
            self.interaction_signal & AMER_INTERACTION_SIGNAL != u16::MIN
        } else {
            self.interaction_signal != u16::MIN
        };
        if self.depth_velocity <= DEPTH_DAMPING_THRESHOLD {
            self.depth_velocity = self.depth_velocity.wrapping_add(KEY_DEPTH_STEP);
            if interaction_active {
                self.depth_velocity = self.depth_velocity.wrapping_sub(CONTROL_DEPTH_ADJUSTMENT);
            }
        } else if interaction_active {
            self.depth_velocity = CONTROL_DEPTH_IMPULSE;
        }

        if species.uses_amer_input_policy() {
            self.interaction_signal = u16::MIN;
        }
        let key = self.key_event;
        if !species.uses_amer_input_policy() {
            self.key_event = u16::MIN;
        }
        let action = match key {
            CURSOR_UP_KEY => {
                self.key_event = u16::MIN;
                self.depth_velocity = self.depth_velocity.wrapping_add(KEY_DEPTH_STEP);
                AlienInputAction::IncreaseDepth
            }
            CURSOR_DOWN_KEY => {
                self.key_event = u16::MIN;
                self.depth_velocity = self.depth_velocity.wrapping_sub(KEY_DEPTH_STEP);
                AlienInputAction::DecreaseDepth
            }
            _ if !species.uses_amer_input_policy() && key as u8 == ASCII_SPACE => {
                self.scene_flags |= INTERACTION_REQUEST_FLAG;
                AlienInputAction::Interact
            }
            _ => AlienInputAction::None,
        };

        AlienCameraStep {
            centered_cursor: [centered_x, centered_y],
            action,
        }
    }
}

fn dead_zone(value: i16) -> i16 {
    let shifted = value.wrapping_sub(DEAD_ZONE_HALF_WIDTH);
    if !shifted.is_negative() {
        return shifted;
    }
    let opposite_edge = shifted.wrapping_add(DEAD_ZONE_WIDTH);
    if !opposite_edge.is_negative() {
        i16::default()
    } else {
        opposite_edge
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct MouseVector {
        x: u16,
        y: u16,
        buttons: u16,
    }

    #[derive(Deserialize)]
    struct CameraVector {
        pitch: i16,
        pan: i16,
        pan_target: i16,
        depth_step: i16,
    }

    #[derive(Deserialize)]
    struct ControlVector {
        name: String,
        mouse: MouseVector,
        filter_x_before: i16,
        camera_before: CameraVector,
        control_before: u16,
        key_before: u16,
        code_flags_before: u16,
        centered: CenteredVector,
        filter_x_after: i16,
        camera_after: CameraVector,
        control_after: u16,
        key_after: u16,
        code_flags_after: u16,
        final_path: String,
    }

    #[derive(Deserialize)]
    struct CenteredVector {
        x: i16,
        y: i16,
    }

    fn expected_action(path: &str) -> AlienInputAction {
        match path {
            "cursor_up" => AlienInputAction::IncreaseDepth,
            "cursor_down" => AlienInputAction::DecreaseDepth,
            "space_action" => AlienInputAction::Interact,
            "unhandled_cleared" | "unhandled_retained" => AlienInputAction::None,
            _ => panic!("unrecognized original control path {path}"),
        }
    }

    #[test]
    fn controller_matches_every_original_alien_overlay_vector() {
        let suites = [
            (
                AlienSpecies::Amer,
                include_str!(
                    "../../../../../re/tools/oracle_vectors/xdb_amer_func_0223_natural.json"
                ),
            ),
            (
                AlienSpecies::Croolis,
                include_str!(
                    "../../../../../re/tools/oracle_vectors/xdb_croolis_func_022a_natural.json"
                ),
            ),
            (
                AlienSpecies::Scrut,
                include_str!(
                    "../../../../../re/tools/oracle_vectors/xdb_scrut_func_022a_natural.json"
                ),
            ),
        ];

        for (species, json) in suites {
            let vectors: Vec<ControlVector> = serde_json::from_str(json).unwrap();
            for vector in vectors {
                let mut state = AlienCameraControl {
                    horizontal_filter: vector.filter_x_before,
                    pitch: vector.camera_before.pitch,
                    pan: vector.camera_before.pan,
                    secondary_pan: vector.camera_before.pan_target,
                    depth_velocity: vector.camera_before.depth_step,
                    interaction_signal: vector.control_before,
                    key_event: vector.key_before,
                    scene_flags: vector.code_flags_before,
                };
                let result = state.step(
                    species,
                    AlienMouseSample {
                        x: vector.mouse.x,
                        y: vector.mouse.y,
                        buttons: vector.mouse.buttons,
                    },
                );

                assert_eq!(
                    result.centered_cursor,
                    [vector.centered.x, vector.centered.y],
                    "{} {species:?}",
                    vector.name
                );
                assert_eq!(
                    result.action,
                    expected_action(&vector.final_path),
                    "{} {species:?}",
                    vector.name
                );
                assert_eq!(
                    state.horizontal_filter, vector.filter_x_after,
                    "{} {species:?}",
                    vector.name
                );
                assert_eq!(
                    state.pitch, vector.camera_after.pitch,
                    "{} {species:?}",
                    vector.name
                );
                assert_eq!(
                    state.pan, vector.camera_after.pan,
                    "{} {species:?}",
                    vector.name
                );
                assert_eq!(
                    state.secondary_pan, vector.camera_after.pan_target,
                    "{} {species:?}",
                    vector.name
                );
                assert_eq!(
                    state.depth_velocity, vector.camera_after.depth_step,
                    "{} {species:?}",
                    vector.name
                );
                assert_eq!(
                    state.interaction_signal, vector.control_after,
                    "{} {species:?}",
                    vector.name
                );
                assert_eq!(
                    state.key_event, vector.key_after,
                    "{} {species:?}",
                    vector.name
                );
                assert_eq!(
                    state.scene_flags, vector.code_flags_after,
                    "{} {species:?}",
                    vector.name
                );
            }
        }
    }
}
