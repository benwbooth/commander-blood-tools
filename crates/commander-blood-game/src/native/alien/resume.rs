//! Typed state and callbacks for alien resume behavior.

use std::fmt;

use commander_blood_formats::alien::{AlienTrigonometryPair, TRIGONOMETRY_ENTRY_COUNT};

use super::{AlienNodePose, AlienSpecies};

const X_AXIS: usize = 0;
const Y_AXIS: usize = 1;
const Z_AXIS: usize = 2;
const PITCH_AXIS: usize = 0;
const PAN_AXIS: usize = 1;
const AMER_DEPTH_BOUND: i32 = 100;
const OTHER_DEPTH_BOUND: i32 = 200;
const LATERAL_BOUND: i32 = 200;
const VERTICAL_MINIMUM: i16 = -200;
const VERTICAL_MAXIMUM_EXCLUSIVE: i16 = 200;
const VERTICAL_EASING_SHIFT: u32 = 3;
const PITCH_EASING_SHIFT: u32 = 1;
const ANGLE_MASK: u16 = 0x0ffc;
const ANGLE_INDEX_SHIFT: u32 = 2;
const NEGATIVE_DIRECTION_PAN_STEP: i16 = -32;
const NONNEGATIVE_DIRECTION_PAN_STEP: i16 = 16;
const TEXTURE_COMPONENT_COUNT: usize = 2;
const TEXTURE_U_COMPONENT: usize = 0;
const TEXTURE_V_COMPONENT: usize = 1;
const PHASE_HIGH_BYTE_SHIFT: u32 = 8;
const PHASE_HIGH_CLAMP: i8 = 22;
const PHASE_ZERO_STEP: u8 = 2;
const PHASE_REVERSE_STEP: u8 = 254;
const AMER_RESUME_TEXTURE_VERTEX_COUNT: usize = 54;
const CROOLIS_RESUME_TEXTURE_VERTEX_COUNT: usize = 26;
const SCRUT_RESUME_TEXTURE_VERTEX_COUNT: usize = 44;
const AMER_RESUME_TEXTURE_TARGETS: [(usize, TextureDirection); 4] = [
    (0, TextureDirection::Add),
    (53, TextureDirection::Subtract),
    (35, TextureDirection::Add),
    (25, TextureDirection::Subtract),
];
const CROOLIS_RESUME_TEXTURE_TARGETS: [(usize, TextureDirection); 2] =
    [(0, TextureDirection::Add), (25, TextureDirection::Subtract)];
const SCRUT_RESUME_TEXTURE_TARGETS: [(usize, TextureDirection); 4] = [
    (0, TextureDirection::Add),
    (43, TextureDirection::Subtract),
    (42, TextureDirection::Add),
    (25, TextureDirection::Subtract),
];

#[derive(Clone, Copy)]
enum TextureDirection {
    Add,
    Subtract,
}

/// Resume callback selected by the recovered slot-13 coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienResumeCallback {
    /// Begin the species-specific resume state machine.
    Begin,
    /// Move the current node toward its queued partner.
    Pair,
    /// Continue texture motion while the resume delay expires.
    Timeout,
    /// Move the current node back toward the active queue anchor.
    Final,
}

/// Typed continuation state owned by one resumable behavior method.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienResumeMethodState {
    /// Callback selected for the next coordinator invocation.
    pub callback: Option<AlienResumeCallback>,
    /// Current phase within the resume state machine.
    pub phase: u16,
    /// Optional node paired with the currently resumed node.
    pub paired_node: Option<usize>,
    /// Optional node whose state is being resumed.
    pub resumed_node: Option<usize>,
}

/// Callback boundary retained by the recovered resume coordinator.
pub trait AlienResumeCallbacks {
    /// Error returned by the concrete callback implementation.
    type Error;

    /// Invoke the selected resume callback.
    fn invoke(
        &mut self,
        species: AlienSpecies,
        callback: AlienResumeCallback,
        state: &mut AlienResumeMethodState,
    ) -> Result<(), Self::Error>;
}

/// Stage completed by one invocation of the resume coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienResumeUpdate {
    /// The initial callback and empty pairing state were installed.
    Initialized,
    /// The previously selected callback was invoked.
    CallbackInvoked,
}

/// Spatial relationship found while steering a resumed model pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienResumePairUpdate {
    /// Both nodes are inside the species-specific pairing bounds.
    Inside,
    /// The current node was steered toward an outlying paired node.
    Outside,
}

/// Texture animation result produced by one resume-state update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienResumeTextureUpdate {
    /// Signed low-byte displacement applied to the selected coordinates.
    pub delta: i16,
    /// Packed animation phase after its wrapping high-byte advance.
    pub phase: u16,
}

/// State produced by one resume timeout continuation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienResumeTimeoutUpdate {
    /// Texture-coordinate motion completed before the countdown update.
    pub texture: AlienResumeTextureUpdate,
    /// Wrapping post-decrement countdown value.
    pub countdown: u16,
    /// Whether the post-decrement sign selected the final continuation.
    pub final_stage_selected: bool,
}

/// Invalid model data supplied to the recovered resume texture animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienResumeTextureError {
    /// Number of texture coordinates required by the species animation.
    pub required: usize,
    /// Number of texture coordinates present in the decoded model.
    pub available: usize,
}

impl fmt::Display for AlienResumeTextureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "resume texture animation requires {} vertices, but the model contains {}",
            self.required, self.available
        )
    }
}

impl std::error::Error for AlienResumeTextureError {}

/// Initialize or dispatch the recovered slot-13 resume method.
pub fn initialize_or_dispatch_resume<C: AlienResumeCallbacks>(
    species: AlienSpecies,
    state: &mut AlienResumeMethodState,
    callbacks: &mut C,
) -> Result<AlienResumeUpdate, C::Error> {
    if let Some(callback) = state.callback {
        callbacks.invoke(species, callback, state)?;
        return Ok(AlienResumeUpdate::CallbackInvoked);
    }

    state.callback = Some(AlienResumeCallback::Begin);
    state.phase = u16::MIN;
    state.paired_node = None;
    Ok(AlienResumeUpdate::Initialized)
}

/// Animate the species-specific texture vertices used by the resume sequence.
pub fn update_resume_texture_motion(
    species: AlienSpecies,
    state: &mut AlienResumeMethodState,
    texture_coordinates: &mut [[i16; TEXTURE_COMPONENT_COUNT]],
) -> Result<AlienResumeTextureUpdate, AlienResumeTextureError> {
    let (component, required, targets): (_, _, &[(usize, TextureDirection)]) = match species {
        AlienSpecies::Amer => (
            TEXTURE_V_COMPONENT,
            AMER_RESUME_TEXTURE_VERTEX_COUNT,
            &AMER_RESUME_TEXTURE_TARGETS,
        ),
        AlienSpecies::Croolis => (
            TEXTURE_U_COMPONENT,
            CROOLIS_RESUME_TEXTURE_VERTEX_COUNT,
            &CROOLIS_RESUME_TEXTURE_TARGETS,
        ),
        AlienSpecies::Scrut => (
            TEXTURE_V_COMPONENT,
            SCRUT_RESUME_TEXTURE_VERTEX_COUNT,
            &SCRUT_RESUME_TEXTURE_TARGETS,
        ),
    };
    if texture_coordinates.len() < required {
        return Err(AlienResumeTextureError {
            required,
            available: texture_coordinates.len(),
        });
    }

    let low = state.phase as u8;
    let delta = i16::from(low as i8);
    for &(vertex, direction) in targets {
        let coordinate = &mut texture_coordinates[vertex][component];
        *coordinate = match direction {
            TextureDirection::Add => coordinate.wrapping_add(delta),
            TextureDirection::Subtract => coordinate.wrapping_sub(delta),
        };
    }

    let high = ((state.phase >> PHASE_HIGH_BYTE_SHIFT) as u8).wrapping_add(low);
    let next_low = if (high as i8) >= PHASE_HIGH_CLAMP {
        PHASE_REVERSE_STEP
    } else if high == u8::default() {
        PHASE_ZERO_STEP
    } else {
        low
    };
    state.phase = (u16::from(high) << PHASE_HIGH_BYTE_SHIFT) | u16::from(next_low);
    Ok(AlienResumeTextureUpdate {
        delta,
        phase: state.phase,
    })
}

/// Advance resume texture motion and select the final stage after timeout.
pub fn update_resume_timeout(
    species: AlienSpecies,
    state: &mut AlienResumeMethodState,
    texture_coordinates: &mut [[i16; TEXTURE_COMPONENT_COUNT]],
    countdown: &mut u16,
) -> Result<AlienResumeTimeoutUpdate, AlienResumeTextureError> {
    let texture = update_resume_texture_motion(species, state, texture_coordinates)?;
    *countdown = countdown.wrapping_sub(1);
    let final_stage_selected = (*countdown as i16).is_negative();
    if final_stage_selected {
        state.callback = Some(AlienResumeCallback::Final);
    }
    Ok(AlienResumeTimeoutUpdate {
        texture,
        countdown: *countdown,
        final_stage_selected,
    })
}

/// Test a resumed node pair and steer the current node when they remain apart.
pub fn update_resume_pair_steering(
    species: AlienSpecies,
    current: &mut AlienNodePose,
    other: &AlienNodePose,
    trigonometry: &[AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
) -> AlienResumePairUpdate {
    let depth_delta = i32::from(position_word(other, Z_AXIS))
        .wrapping_sub(i32::from(position_word(current, Z_AXIS)));
    let lateral_delta = i32::from(position_word(other, X_AXIS))
        .wrapping_sub(i32::from(position_word(current, X_AXIS)));
    let vertical_delta = position_word(other, Y_AXIS).wrapping_sub(position_word(current, Y_AXIS));
    let depth_bound = match species {
        AlienSpecies::Amer => AMER_DEPTH_BOUND,
        AlienSpecies::Croolis | AlienSpecies::Scrut => OTHER_DEPTH_BOUND,
    };
    if (-depth_bound..=depth_bound).contains(&depth_delta)
        && (-LATERAL_BOUND..=LATERAL_BOUND).contains(&lateral_delta)
        && (VERTICAL_MINIMUM..VERTICAL_MAXIMUM_EXCLUSIVE).contains(&vertical_delta)
    {
        return AlienResumePairUpdate::Inside;
    }

    let vertical_step = vertical_delta >> VERTICAL_EASING_SHIFT;
    let pitch = (current.angles[PITCH_AXIS] as i16).wrapping_sub(vertical_step);
    current.angles[PITCH_AXIS] = (pitch >> PITCH_EASING_SHIFT) as u16;

    let sample_offset = current.angles[PAN_AXIS] & ANGLE_MASK;
    let sample = trigonometry[usize::from(sample_offset >> ANGLE_INDEX_SHIFT)];
    let direction = i32::from(sample.cosine)
        .wrapping_mul(lateral_delta)
        .wrapping_sub(i32::from(sample.sine).wrapping_mul(depth_delta));
    let pan_step = if direction < i32::default() {
        NEGATIVE_DIRECTION_PAN_STEP
    } else {
        NONNEGATIVE_DIRECTION_PAN_STEP
    };
    current.angles[PAN_AXIS] = sample_offset.wrapping_add(pan_step as u16);
    AlienResumePairUpdate::Outside
}

fn position_word(node: &AlienNodePose, axis: usize) -> i16 {
    node.local_position[axis] as i16
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use commander_blood_formats::alien::{AXIS_COUNT, AlienNodeParent, AlienTransformData};
    use serde::Deserialize;

    use super::*;

    const PRESERVED_RESUMED_NODE: usize = 37;

    #[derive(Deserialize)]
    struct ResumeVector {
        name: String,
        module: String,
        resume_before: u16,
        resume_after: u16,
        resume_step_before: u16,
        resume_step_after: u16,
        resume_value_before: u16,
        resume_value_after: u16,
        tail_dispatched: bool,
    }

    #[derive(Deserialize)]
    struct ResumePairVector {
        name: String,
        module: String,
        outside: bool,
        current_position: [u32; AXIS_COUNT],
        other_position: [u32; AXIS_COUNT],
        pitch_before: u16,
        pitch_after: u16,
        pan_before: u16,
        pan_after: u16,
        cosine: u16,
        sine: u16,
        current_position_after: [u32; AXIS_COUNT],
        other_position_after: [u32; AXIS_COUNT],
    }

    #[derive(Deserialize)]
    struct ResumeTextureVector {
        name: String,
        module: String,
        component: String,
        required_vertex_count: usize,
        phase_before: u16,
        phase_after: u16,
        signed_delta: i16,
        targets: Vec<ResumeTextureTargetVector>,
    }

    #[derive(Deserialize)]
    struct ResumeTextureTargetVector {
        vertex: usize,
        direction: i16,
        before: u16,
        after: u16,
    }

    #[derive(Deserialize)]
    struct ResumeTimeoutVector {
        name: String,
        module: String,
        component: String,
        required_vertex_count: usize,
        phase_before: u16,
        phase_after: u16,
        signed_delta: i16,
        targets: Vec<ResumeTextureTargetVector>,
        countdown_before: u16,
        countdown_after: u16,
        final_selected: bool,
    }

    #[derive(Default)]
    struct CallbackRecorder {
        calls: Vec<(AlienSpecies, AlienResumeCallback)>,
    }

    impl AlienResumeCallbacks for CallbackRecorder {
        type Error = Infallible;

        fn invoke(
            &mut self,
            species: AlienSpecies,
            callback: AlienResumeCallback,
            _state: &mut AlienResumeMethodState,
        ) -> Result<(), Self::Error> {
            self.calls.push((species, callback));
            Ok(())
        }
    }

    fn species(module: &str) -> AlienSpecies {
        match module {
            "amer" => AlienSpecies::Amer,
            "croolis" => AlienSpecies::Croolis,
            "scrut" => AlienSpecies::Scrut,
            _ => panic!("unknown alien module {module}"),
        }
    }

    fn fixtures() -> [&'static str; 3] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_1bea_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_1b46_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_1bfb_natural.json"),
        ]
    }

    fn pair_fixtures() -> [&'static str; 3] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_1cfa_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_1c46_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_1d06_natural.json"),
        ]
    }

    fn texture_fixtures() -> [&'static str; 3] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_1c03_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_1b5f_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_1c14_natural.json"),
        ]
    }

    fn timeout_fixtures() -> [&'static str; 3] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_1cbf_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_1c0b_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_1ccb_natural.json"),
        ]
    }

    fn node(position: [u32; AXIS_COUNT], pitch: u16, pan: u16) -> AlienNodePose {
        AlienNodePose {
            parent: AlienNodeParent::Root,
            scene_parent: None,
            first_vertex: usize::default(),
            vertex_count: 1,
            transform: AlienTransformData::default(),
            local_position: position.map(|value| value as i32),
            angles: [pitch, pan, u16::default()],
            radial_offset: i16::default(),
        }
    }

    #[test]
    fn initialization_matches_every_original_overlay_vector() {
        for fixture in fixtures() {
            let vectors: Vec<ResumeVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors.into_iter().take(3) {
                assert_eq!(vector.resume_before, u16::MIN);
                assert!(!vector.tail_dispatched);
                let mut state = AlienResumeMethodState {
                    callback: None,
                    phase: vector.resume_step_before,
                    paired_node: Some(usize::from(vector.resume_value_before)),
                    resumed_node: Some(PRESERVED_RESUMED_NODE),
                };
                let mut callbacks = CallbackRecorder::default();

                assert_eq!(
                    initialize_or_dispatch_resume(
                        species(&vector.module),
                        &mut state,
                        &mut callbacks,
                    )
                    .unwrap(),
                    AlienResumeUpdate::Initialized,
                    "{}",
                    vector.name
                );
                assert_eq!(state.callback, Some(AlienResumeCallback::Begin));
                assert_ne!(vector.resume_after, u16::MIN);
                assert_eq!(state.phase, vector.resume_step_after);
                assert_eq!(state.paired_node, None);
                assert_eq!(vector.resume_value_after, u16::MIN);
                assert_eq!(state.resumed_node, Some(PRESERVED_RESUMED_NODE));
                assert!(callbacks.calls.is_empty());
            }
        }
    }

    #[test]
    fn dispatch_matches_every_original_overlay_vector() {
        for fixture in fixtures() {
            let vectors: Vec<ResumeVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors.into_iter().skip(3) {
                assert_ne!(vector.resume_before, u16::MIN);
                assert!(vector.tail_dispatched);
                let paired_node = Some(usize::from(vector.resume_value_before));
                let mut state = AlienResumeMethodState {
                    callback: Some(AlienResumeCallback::Begin),
                    phase: vector.resume_step_before,
                    paired_node,
                    resumed_node: Some(PRESERVED_RESUMED_NODE),
                };
                let mut callbacks = CallbackRecorder::default();

                assert_eq!(
                    initialize_or_dispatch_resume(
                        species(&vector.module),
                        &mut state,
                        &mut callbacks,
                    )
                    .unwrap(),
                    AlienResumeUpdate::CallbackInvoked,
                    "{}",
                    vector.name
                );
                assert_eq!(state.callback, Some(AlienResumeCallback::Begin));
                assert_eq!(vector.resume_after, vector.resume_before);
                assert_eq!(state.phase, vector.resume_step_after);
                assert_eq!(vector.resume_step_after, vector.resume_step_before);
                assert_eq!(state.paired_node, paired_node);
                assert_eq!(vector.resume_value_after, vector.resume_value_before);
                assert_eq!(state.resumed_node, Some(PRESERVED_RESUMED_NODE));
                assert_eq!(
                    callbacks.calls,
                    vec![(species(&vector.module), AlienResumeCallback::Begin)]
                );
            }
        }
    }

    #[test]
    fn pair_steering_matches_every_original_overlay_vector() {
        for fixture in pair_fixtures() {
            let vectors: Vec<ResumePairVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                let species = species(&vector.module);
                let mut current = node(
                    vector.current_position,
                    vector.pitch_before,
                    vector.pan_before,
                );
                let other = node(vector.other_position, u16::default(), u16::default());
                let mut trigonometry = [AlienTrigonometryPair::default(); TRIGONOMETRY_ENTRY_COUNT];
                let sample_index =
                    usize::from((vector.pan_before & ANGLE_MASK) >> ANGLE_INDEX_SHIFT);
                trigonometry[sample_index] = AlienTrigonometryPair {
                    cosine: vector.cosine as i16,
                    sine: vector.sine as i16,
                };

                assert_eq!(
                    update_resume_pair_steering(species, &mut current, &other, &trigonometry,),
                    if vector.outside {
                        AlienResumePairUpdate::Outside
                    } else {
                        AlienResumePairUpdate::Inside
                    },
                    "{}",
                    vector.name
                );
                assert_eq!(
                    current.angles[PITCH_AXIS], vector.pitch_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    current.angles[PAN_AXIS], vector.pan_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    current.local_position.map(|value| value as u32),
                    vector.current_position_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    other.local_position.map(|value| value as u32),
                    vector.other_position_after,
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn texture_motion_matches_every_original_overlay_vector() {
        for fixture in texture_fixtures() {
            let vectors: Vec<ResumeTextureVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                let component = match vector.component.as_str() {
                    "u" => TEXTURE_U_COMPONENT,
                    "v" => TEXTURE_V_COMPONENT,
                    value => panic!("unknown texture component {value}"),
                };
                let mut texture_coordinates =
                    vec![[12_345_i16, -23_456_i16]; vector.required_vertex_count];
                for target in &vector.targets {
                    texture_coordinates[target.vertex][component] = target.before as i16;
                }
                let mut expected = texture_coordinates.clone();
                for target in &vector.targets {
                    expected[target.vertex][component] = target.after as i16;
                    assert!(matches!(target.direction, -1 | 1));
                }
                let preserved_callback = Some(AlienResumeCallback::Begin);
                let preserved_paired_node = Some(17);
                let preserved_resumed_node = Some(29);
                let mut state = AlienResumeMethodState {
                    callback: preserved_callback,
                    phase: vector.phase_before,
                    paired_node: preserved_paired_node,
                    resumed_node: preserved_resumed_node,
                };

                assert_eq!(
                    update_resume_texture_motion(
                        species(&vector.module),
                        &mut state,
                        &mut texture_coordinates,
                    ),
                    Ok(AlienResumeTextureUpdate {
                        delta: vector.signed_delta,
                        phase: vector.phase_after,
                    }),
                    "{}",
                    vector.name
                );
                assert_eq!(texture_coordinates, expected, "{}", vector.name);
                assert_eq!(state.phase, vector.phase_after, "{}", vector.name);
                assert_eq!(state.callback, preserved_callback, "{}", vector.name);
                assert_eq!(state.paired_node, preserved_paired_node, "{}", vector.name);
                assert_eq!(
                    state.resumed_node, preserved_resumed_node,
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn texture_motion_rejects_truncated_model_data_without_mutation() {
        for species in [
            AlienSpecies::Amer,
            AlienSpecies::Croolis,
            AlienSpecies::Scrut,
        ] {
            let required = match species {
                AlienSpecies::Amer => AMER_RESUME_TEXTURE_VERTEX_COUNT,
                AlienSpecies::Croolis => CROOLIS_RESUME_TEXTURE_VERTEX_COUNT,
                AlienSpecies::Scrut => SCRUT_RESUME_TEXTURE_VERTEX_COUNT,
            };
            let mut state = AlienResumeMethodState {
                phase: 0x0102,
                ..AlienResumeMethodState::default()
            };
            let original_state = state;
            let mut coordinates = vec![[123, 456]; required - 1];
            let original_coordinates = coordinates.clone();

            assert_eq!(
                update_resume_texture_motion(species, &mut state, &mut coordinates),
                Err(AlienResumeTextureError {
                    required,
                    available: required - 1,
                })
            );
            assert_eq!(state, original_state);
            assert_eq!(coordinates, original_coordinates);
        }
    }

    #[test]
    fn timeout_continuation_matches_every_original_overlay_vector() {
        for fixture in timeout_fixtures() {
            let vectors: Vec<ResumeTimeoutVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                let component = match vector.component.as_str() {
                    "u" => TEXTURE_U_COMPONENT,
                    "v" => TEXTURE_V_COMPONENT,
                    value => panic!("unknown texture component {value}"),
                };
                let mut texture_coordinates =
                    vec![[12_345_i16, -23_456_i16]; vector.required_vertex_count];
                for target in &vector.targets {
                    texture_coordinates[target.vertex][component] = target.before as i16;
                }
                let mut expected = texture_coordinates.clone();
                for target in &vector.targets {
                    expected[target.vertex][component] = target.after as i16;
                }
                let mut state = AlienResumeMethodState {
                    callback: Some(AlienResumeCallback::Timeout),
                    phase: vector.phase_before,
                    paired_node: Some(17),
                    resumed_node: Some(29),
                };
                let mut countdown = vector.countdown_before;

                assert_eq!(
                    update_resume_timeout(
                        species(&vector.module),
                        &mut state,
                        &mut texture_coordinates,
                        &mut countdown,
                    ),
                    Ok(AlienResumeTimeoutUpdate {
                        texture: AlienResumeTextureUpdate {
                            delta: vector.signed_delta,
                            phase: vector.phase_after,
                        },
                        countdown: vector.countdown_after,
                        final_stage_selected: vector.final_selected,
                    }),
                    "{}",
                    vector.name
                );
                assert_eq!(texture_coordinates, expected, "{}", vector.name);
                assert_eq!(state.phase, vector.phase_after, "{}", vector.name);
                assert_eq!(countdown, vector.countdown_after, "{}", vector.name);
                assert_eq!(
                    state.callback,
                    Some(if vector.final_selected {
                        AlienResumeCallback::Final
                    } else {
                        AlienResumeCallback::Timeout
                    }),
                    "{}",
                    vector.name
                );
                assert_eq!(state.paired_node, Some(17), "{}", vector.name);
                assert_eq!(state.resumed_node, Some(29), "{}", vector.name);
            }
        }
    }
}
