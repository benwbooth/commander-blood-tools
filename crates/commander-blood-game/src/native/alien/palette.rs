//! Palette-index cycling and model motion used by the alien texture animator.

use std::fmt;

use commander_blood_formats::alien::{AXIS_COUNT, AlienNodeParent, PALETTE_REMAP_ENTRY_COUNT};

use super::{AlienModelPose, AlienSpecies};

const X_AXIS: usize = 0;
const Y_AXIS: usize = 1;
const Z_AXIS: usize = 2;
const FIRST_NODE: usize = 0;
const PITCH_AXIS: usize = 0;
const PRIMARY_PAN_AXIS: usize = 1;
const SECONDARY_PAN_AXIS: usize = 2;
const ROOT_DIAGONAL: i32 = 32_768;
const DEPTH_SCALE_SHIFT: u32 = 8;
const HORIZONTAL_SCALE_SHIFT: u32 = 2;
const POSITION_DELTA_SHIFT: u32 = 16;
const MOUSE_ANGLE_SHIFT: u32 = 2;
const VERTICAL_MOTION_FACTOR: i32 = -60;
const MAXIMUM_CYCLE_LEVEL: u16 = 128;
const CYCLE_REVERSAL_COUNTDOWN: u8 = 3;
const PULSE_SHIFT_MASK: u16 = 3;
const PULSE_BASE_LEVELS: [u16; AXIS_COUNT] = [10, 13, 11];
const TEXTURE_PAGE_BYTE_COUNT: usize = 256;
const LOW_REGION_PAGE_LIMIT: u16 = 63;
const LOW_REGION_BYTE_COUNT: usize = 30;
const HIGH_REGION_FIRST_BYTE: usize = LOW_REGION_BYTE_COUNT;
const ZERO_COMPONENT: i32 = 0;

/// Signed mouse sample consumed by the texture animator's model motion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienPaletteInput {
    /// Horizontal mouse position used for both pan angles and lateral motion.
    pub x: i16,
    /// Vertical mouse position negated into the model pitch.
    pub y: i16,
}

/// Persistent cycle and pulse state shared by the palette-animation method.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienPaletteAnimationState {
    /// Phase used to determine which texture pages changed this frame.
    pub previous_level: u16,
    /// Signed phase increment.
    pub step: i8,
    /// Frames remaining before reversing the phase increment.
    pub countdown: u8,
    /// Current CROOLIS/SCRUT pulse levels.
    pub pulse_levels: [u16; AXIS_COUNT],
}

/// Observable result of one palette-animation update.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienPaletteUpdate {
    /// Texture bytes whose palette index changed after applying the remap table.
    pub changed_texture_bytes: usize,
}

/// Invalid typed state supplied to the recovered palette-animation method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienPaletteError {
    /// The method always operates on the first model node.
    MissingPrimaryNode,
    /// A malformed cycle produced a reversed unsigned texture-page interval.
    InvalidTexturePageRange {
        /// First page requested by the cycle.
        first: u16,
        /// Exclusive final page requested by the cycle.
        last: u16,
    },
    /// A selected page extends beyond the owned texture atlas.
    TextureRegionOutOfBounds {
        /// First byte requested by the remap operation.
        start: usize,
        /// Exclusive final byte requested by the remap operation.
        end: usize,
        /// Number of available texture bytes.
        available: usize,
    },
}

impl fmt::Display for AlienPaletteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid alien palette-animation state: {self:?}")
    }
}

impl std::error::Error for AlienPaletteError {}

/// Update palette-model motion, cycle state, pulses, and animated texture pages.
pub fn update_palette_animation(
    species: AlienSpecies,
    pose: &mut AlienModelPose,
    input: AlienPaletteInput,
    scene_flags: &mut u16,
    method_delta: &mut i16,
    state: &mut AlienPaletteAnimationState,
    texture: &mut [u8],
    remap: &[u8; PALETTE_REMAP_ENTRY_COUNT],
) -> Result<AlienPaletteUpdate, AlienPaletteError> {
    let node = pose
        .nodes
        .get_mut(FIRST_NODE)
        .ok_or(AlienPaletteError::MissingPrimaryNode)?;

    for axis in usize::MIN..AXIS_COUNT {
        pose.root.matrix[axis][axis] = ROOT_DIAGONAL;
    }
    pose.root.translation[X_AXIS] = ZERO_COMPONENT;
    pose.root.translation[Y_AXIS] = ZERO_COMPONENT;
    node.parent = AlienNodeParent::Root;

    let scaled_depth = node.transform.translation[Z_AXIS] >> DEPTH_SCALE_SHIFT;
    let horizontal = scaled_depth
        .wrapping_mul(i32::from(input.x))
        .wrapping_shr(HORIZONTAL_SCALE_SHIFT)
        .wrapping_sub(node.transform.translation[X_AXIS])
        >> POSITION_DELTA_SHIFT;
    let vertical = VERTICAL_MOTION_FACTOR
        .wrapping_mul(scaled_depth)
        .wrapping_sub(node.transform.translation[Y_AXIS])
        >> POSITION_DELTA_SHIFT;
    let mouse_pan = (input.x as u16).wrapping_shl(MOUSE_ANGLE_SHIFT);
    node.angles[PRIMARY_PAN_AXIS] = mouse_pan;
    node.angles[SECONDARY_PAN_AXIS] = mouse_pan;
    node.angles[PITCH_AXIS] = u16::MIN.wrapping_sub(input.y as u16);
    node.local_position[X_AXIS] = node.local_position[X_AXIS].wrapping_add(horizontal);
    node.local_position[Y_AXIS] = node.local_position[Y_AXIS].wrapping_add(vertical);

    if species != AlienSpecies::Amer && *scene_flags != u16::MIN {
        *scene_flags = scene_flags.wrapping_sub(1);
        let shift = u32::from(*scene_flags & PULSE_SHIFT_MASK);
        state.pulse_levels = PULSE_BASE_LEVELS.map(|level| level << shift);
    }

    let current_level = *method_delta as u16;
    if current_level > MAXIMUM_CYCLE_LEVEL {
        return Ok(AlienPaletteUpdate::default());
    }
    let mut lower = MAXIMUM_CYCLE_LEVEL.wrapping_sub(current_level);
    let mut upper = MAXIMUM_CYCLE_LEVEL.wrapping_sub(state.previous_level);
    state.previous_level = current_level;

    let next_level = (current_level as u8).wrapping_add(state.step as u8);
    if (next_level as i8).is_negative() {
        return Ok(AlienPaletteUpdate::default());
    }
    let next_countdown = state.countdown.wrapping_sub(1);
    if (next_countdown as i8).is_negative() {
        state.countdown = CYCLE_REVERSAL_COUNTDOWN;
        state.step = state.step.wrapping_neg();
    } else {
        state.countdown = next_countdown;
    }
    *method_delta = i16::from(next_level);

    if lower == upper {
        return Ok(AlienPaletteUpdate::default());
    }
    if (lower as i16) > (upper as i16) {
        std::mem::swap(&mut lower, &mut upper);
    }

    let mut changed_texture_bytes = remap_texture_pages(
        texture,
        remap,
        lower.saturating_sub(LOW_REGION_PAGE_LIMIT),
        upper.saturating_sub(LOW_REGION_PAGE_LIMIT),
        HIGH_REGION_FIRST_BYTE,
        TEXTURE_PAGE_BYTE_COUNT,
    )?;
    changed_texture_bytes += remap_texture_pages(
        texture,
        remap,
        lower.min(LOW_REGION_PAGE_LIMIT),
        upper.min(LOW_REGION_PAGE_LIMIT),
        usize::MIN,
        LOW_REGION_BYTE_COUNT,
    )?;
    Ok(AlienPaletteUpdate {
        changed_texture_bytes,
    })
}

fn remap_texture_pages(
    texture: &mut [u8],
    remap: &[u8; PALETTE_REMAP_ENTRY_COUNT],
    first_page: u16,
    last_page: u16,
    first_byte: usize,
    last_byte: usize,
) -> Result<usize, AlienPaletteError> {
    if first_page > last_page {
        return Err(AlienPaletteError::InvalidTexturePageRange {
            first: first_page,
            last: last_page,
        });
    }
    let mut changed = usize::MIN;
    for page in first_page..last_page {
        let page_start = usize::from(page) * TEXTURE_PAGE_BYTE_COUNT;
        let start = page_start + first_byte;
        let end = page_start + last_byte;
        let available = texture.len();
        let region =
            texture
                .get_mut(start..end)
                .ok_or(AlienPaletteError::TextureRegionOutOfBounds {
                    start,
                    end,
                    available,
                })?;
        for palette_index in region {
            let replacement = remap[usize::from(*palette_index)];
            changed += usize::from(replacement != *palette_index);
            *palette_index = replacement;
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::alien::{AlienFaceData, AlienTransformData};
    use serde::Deserialize;
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::native::alien::{AlienModelPose, AlienNodePose, AlienProjectedVertex};

    const HEX_BYTE_DIGITS: usize = 2;
    const TEST_TEXTURE_BYTE_COUNT: usize = 65_536;
    const TEXTURE_PATTERN_MULTIPLIER: usize = 37;
    const TEXTURE_CASE_STEP: usize = 43;
    const TEXTURE_PATTERN_BIAS: usize = 17;
    const TEXTURE_PATTERN_MASK: usize = 0xff;
    const NODE_TRANSLATION_X_BASE: u32 = 0x7000_0004;
    const NODE_TRANSLATION_X_STEP: u32 = 0x1111_1111;
    const NODE_TRANSLATION_Y_BASE: u32 = 0x9000_0004;
    const NODE_TRANSLATION_Y_STEP: u32 = 0x0102_0304;
    const NODE_TRANSLATION_Z_BASE: u32 = 0x8000_0100;
    const NODE_TRANSLATION_Z_STEP: u32 = 0x0123_4567;
    const NODE_POSITION_X_BASE: u32 = 0x7fff_fffc;
    const NODE_POSITION_X_STEP: u32 = 0x0001_0203;
    const NODE_POSITION_Y_BASE: u32 = 0x8000_0004;
    const NODE_POSITION_Y_STEP: u32 = 0x0002_0304;
    const ROOT_PRESERVED_COMPONENT: i32 = 777;
    const NODE_PRESERVED_COMPONENT: i32 = 888;
    const INITIAL_PULSE_LEVELS: [u16; AXIS_COUNT] = [101, 102, 103];
    const PALETTE_REMAP_HEX: &str = concat!(
        "00302f151b181a111d0f100e0d0c0b09",
        "0a07e6fd5403e85005e7060456081e1f",
        "202122232425262728292a2b2c2d2e02",
        "01e9ee4d4cebef4bec4a4947483d3e3f",
        "404142434445463b3c3a39373433c8aa",
        "1751525314551c5758595a5b5c5d5e5f",
        "606162636465666768696a6b6c6d6e6f",
        "707172737475767778797a7b7c7d7e7f",
        "80818283848586ea88898a8b8c8d8e8f",
        "909192939495969798999a9b9c9d9e9f",
        "a0a1a2a3a4a5a6a7a8a94ff0acadaeaf",
        "b0b1b2b3b4b5b6b7b8b9babbbcbdbebf",
        "c0c1c2c3c4c5c6c74ec9caedcccdcecf",
        "d0d1d2c9d4d5d6d7d8d9dadbdcdddedf",
        "e0e1e2e3e4e512191631873538cb3236",
        "abf1f2f3f4f5f6f7f8f9fafbfc13feff",
    );

    #[derive(Clone, Copy)]
    struct InitialCase {
        previous_level: u16,
        step: i8,
        countdown: u8,
        scene_flags: u16,
    }

    const INITIAL_CASES: [InitialCase; 8] = [
        InitialCase {
            previous_level: 32,
            step: 1,
            countdown: 2,
            scene_flags: 0,
        },
        InitialCase {
            previous_level: 112,
            step: 1,
            countdown: 2,
            scene_flags: 1,
        },
        InitialCase {
            previous_level: 64,
            step: 0,
            countdown: 2,
            scene_flags: 2,
        },
        InitialCase {
            previous_level: 56,
            step: -4,
            countdown: 2,
            scene_flags: 3,
        },
        InitialCase {
            previous_level: 96,
            step: -4,
            countdown: 1,
            scene_flags: 4,
        },
        InitialCase {
            previous_level: 60,
            step: -2,
            countdown: 3,
            scene_flags: 5,
        },
        InitialCase {
            previous_level: 80,
            step: 2,
            countdown: 4,
            scene_flags: 256,
        },
        InitialCase {
            previous_level: 100,
            step: -3,
            countdown: 0,
            scene_flags: u16::MAX,
        },
    ];

    #[derive(Deserialize)]
    struct PaletteVector {
        name: String,
        module: String,
        mouse: [i16; 2],
        position_after: [i32; 2],
        level_before: u16,
        level_after: u16,
        previous_after: u16,
        control_after: u16,
        pulse_after: u16,
        changed_palette_bytes: usize,
        palette_sha256: String,
    }

    fn species(module: &str) -> AlienSpecies {
        match module {
            "amer" => AlienSpecies::Amer,
            "croolis" => AlienSpecies::Croolis,
            "scrut" => AlienSpecies::Scrut,
            _ => panic!("unknown alien module {module}"),
        }
    }

    fn palette_remap() -> [u8; PALETTE_REMAP_ENTRY_COUNT] {
        assert_eq!(
            PALETTE_REMAP_HEX.len(),
            PALETTE_REMAP_ENTRY_COUNT * HEX_BYTE_DIGITS
        );
        std::array::from_fn(|index| {
            let start = index * HEX_BYTE_DIGITS;
            u8::from_str_radix(&PALETTE_REMAP_HEX[start..start + HEX_BYTE_DIGITS], 16).unwrap()
        })
    }

    fn model_pose(case_index: usize) -> AlienModelPose {
        let index = case_index as u32;
        let node = AlienNodePose {
            parent: AlienNodeParent::SceneCamera,
            scene_parent: None,
            first_vertex: usize::MIN,
            vertex_count: 1,
            transform: AlienTransformData {
                matrix: Default::default(),
                translation: [
                    NODE_TRANSLATION_X_BASE.wrapping_add(index * NODE_TRANSLATION_X_STEP) as i32,
                    NODE_TRANSLATION_Y_BASE.wrapping_sub(index * NODE_TRANSLATION_Y_STEP) as i32,
                    NODE_TRANSLATION_Z_BASE.wrapping_add(index * NODE_TRANSLATION_Z_STEP) as i32,
                ],
            },
            local_position: [
                NODE_POSITION_X_BASE.wrapping_sub(index * NODE_POSITION_X_STEP) as i32,
                NODE_POSITION_Y_BASE.wrapping_add(index * NODE_POSITION_Y_STEP) as i32,
                NODE_PRESERVED_COMPONENT,
            ],
            angles: [u16::MAX; AXIS_COUNT],
            radial_offset: i16::MIN,
        };
        AlienModelPose {
            root: AlienTransformData {
                matrix: [[ROOT_PRESERVED_COMPONENT; AXIS_COUNT]; AXIS_COUNT],
                translation: [
                    ROOT_PRESERVED_COMPONENT,
                    ROOT_PRESERVED_COMPONENT,
                    ROOT_PRESERVED_COMPONENT,
                ],
            },
            nodes: vec![node],
            projected_vertices: vec![AlienProjectedVertex::default()],
            texture_coordinates: vec![[i16::MIN; 2]],
            object_positions: vec![[i16::MIN; AXIS_COUNT]],
            authored_vertex_count: 1,
            faces: Vec::<AlienFaceData>::new(),
            last_rotation_matrix: Default::default(),
            last_common_clip: u16::MIN,
        }
    }

    fn packed_cycle_control(state: AlienPaletteAnimationState) -> u16 {
        u16::from(state.step as u8) | (u16::from(state.countdown) << u8::BITS)
    }

    fn sha256(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    #[test]
    fn palette_animation_matches_every_original_overlay_vector() {
        let fixtures = [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_0355_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_036a_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_036a_natural.json"),
        ];
        let remap = palette_remap();
        for fixture in fixtures {
            let vectors: Vec<PaletteVector> = serde_json::from_str(fixture).unwrap();
            for (case_index, vector) in vectors.into_iter().enumerate() {
                let initial = INITIAL_CASES[case_index];
                let mut pose = model_pose(case_index);
                let mut method_delta = vector.level_before as i16;
                let mut state = AlienPaletteAnimationState {
                    previous_level: initial.previous_level,
                    step: initial.step,
                    countdown: initial.countdown,
                    pulse_levels: INITIAL_PULSE_LEVELS,
                };
                let mut scene_flags = initial.scene_flags;
                let mut texture = (usize::MIN..TEST_TEXTURE_BYTE_COUNT)
                    .map(|position| {
                        ((position * TEXTURE_PATTERN_MULTIPLIER
                            + case_index * TEXTURE_CASE_STEP
                            + TEXTURE_PATTERN_BIAS)
                            & TEXTURE_PATTERN_MASK) as u8
                    })
                    .collect::<Vec<_>>();
                let update = update_palette_animation(
                    species(&vector.module),
                    &mut pose,
                    AlienPaletteInput {
                        x: vector.mouse[X_AXIS],
                        y: vector.mouse[Y_AXIS],
                    },
                    &mut scene_flags,
                    &mut method_delta,
                    &mut state,
                    &mut texture,
                    &remap,
                )
                .unwrap();

                assert_eq!(
                    pose.nodes[FIRST_NODE].local_position[..2],
                    vector.position_after,
                    "{}",
                    vector.name
                );
                assert_eq!(method_delta as u16, vector.level_after, "{}", vector.name);
                assert_eq!(
                    state.previous_level, vector.previous_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    packed_cycle_control(state),
                    vector.control_after,
                    "{}",
                    vector.name
                );
                assert_eq!(scene_flags, vector.pulse_after, "{}", vector.name);
                assert_eq!(
                    update.changed_texture_bytes, vector.changed_palette_bytes,
                    "{}",
                    vector.name
                );
                assert_eq!(sha256(&texture), vector.palette_sha256, "{}", vector.name);
                assert_eq!(pose.nodes[FIRST_NODE].parent, AlienNodeParent::Root);
                assert_eq!(
                    pose.nodes[FIRST_NODE].angles,
                    [
                        u16::MIN.wrapping_sub(vector.mouse[Y_AXIS] as u16),
                        (vector.mouse[X_AXIS] as u16).wrapping_shl(MOUSE_ANGLE_SHIFT),
                        (vector.mouse[X_AXIS] as u16).wrapping_shl(MOUSE_ANGLE_SHIFT),
                    ]
                );
                assert_eq!(
                    pose.nodes[FIRST_NODE].local_position[Z_AXIS],
                    NODE_PRESERVED_COMPONENT
                );
                assert_eq!(
                    pose.root.translation,
                    [ZERO_COMPONENT, ZERO_COMPONENT, ROOT_PRESERVED_COMPONENT]
                );
                for axis in usize::MIN..AXIS_COUNT {
                    assert_eq!(pose.root.matrix[axis][axis], ROOT_DIAGONAL);
                }
                if species(&vector.module) == AlienSpecies::Amer || initial.scene_flags == u16::MIN
                {
                    assert_eq!(state.pulse_levels, INITIAL_PULSE_LEVELS);
                } else {
                    let shift = u32::from(scene_flags & PULSE_SHIFT_MASK);
                    assert_eq!(
                        state.pulse_levels,
                        PULSE_BASE_LEVELS.map(|level| level << shift)
                    );
                }
            }
        }
    }

    #[test]
    fn palette_animation_rejects_missing_nodes_and_short_textures() {
        let remap = palette_remap();
        let mut pose = model_pose(usize::MIN);
        pose.nodes.clear();
        let mut method_delta = i16::MIN;
        let mut scene_flags = u16::MIN;
        assert_eq!(
            update_palette_animation(
                AlienSpecies::Amer,
                &mut pose,
                AlienPaletteInput::default(),
                &mut scene_flags,
                &mut method_delta,
                &mut AlienPaletteAnimationState::default(),
                &mut [],
                &remap,
            ),
            Err(AlienPaletteError::MissingPrimaryNode)
        );

        let mut pose = model_pose(usize::MIN);
        let mut method_delta = 100;
        let mut scene_flags = u16::MIN;
        let mut state = AlienPaletteAnimationState {
            previous_level: 96,
            step: 1,
            countdown: 1,
            ..AlienPaletteAnimationState::default()
        };
        assert!(matches!(
            update_palette_animation(
                AlienSpecies::Amer,
                &mut pose,
                AlienPaletteInput::default(),
                &mut scene_flags,
                &mut method_delta,
                &mut state,
                &mut [],
                &remap,
            ),
            Err(AlienPaletteError::TextureRegionOutOfBounds { .. })
        ));
    }
}
