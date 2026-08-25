//! Fixed-point camera matrix and position update shared by all alien scenes.

use commander_blood_formats::alien::{AXIS_COUNT, AlienTrigonometryPair, TRIGONOMETRY_ENTRY_COUNT};

const X_AXIS: usize = 0;
const Y_AXIS: usize = 1;
const Z_AXIS: usize = 2;
const ANGLE_MASK: u16 = 0x0ffc;
const ANGLE_TABLE_SHIFT: u32 = 2;
const MATRIX_EASING_SHIFT: u32 = 3;
const MATRIX_ROUNDING_SHIFT: u32 = 2;
const MATRIX_ROUNDING_MASK: u32 = 0x0000_0001;
const POSITION_STEP_SHIFT: u32 = 3;
const POSITION_INTEGER_SHIFT: u32 = 16;
const DOUBLE_ANGLE_COMPONENT: i32 = 2;
const HALF_COMPONENT_SHIFT: u32 = 1;
const ZERO_MATRIX_COMPONENT: i32 = 0;

/// Wrapping Euler-angle inputs used by the alien camera matrix routine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienCameraAngles {
    /// Vertical camera angle.
    pub pitch: i16,
    /// Primary horizontal camera angle.
    pub pan: i16,
    /// Smoothed secondary horizontal angle.
    pub secondary_pan: i16,
}

/// Persistent fixed-point transform state for an alien scene camera.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienCameraTransform {
    /// Normalized pitch, pan, and secondary-pan angles used this frame.
    pub normalized_angles: [u16; AXIS_COUNT],
    /// Matrix produced directly from the normalized Euler angles.
    pub target_matrix: [[i32; AXIS_COUNT]; AXIS_COUNT],
    /// Eased camera orientation matrix used for rendering.
    pub matrix: [[i32; AXIS_COUNT]; AXIS_COUNT],
    /// Wrapping fixed-point camera position.
    pub position: [i32; AXIS_COUNT],
    /// Signed integer view components taken from the position high words.
    pub view: [i16; AXIS_COUNT],
    /// Matrix-transformed view vector published to the scene hierarchy.
    pub transformed_view: [i32; AXIS_COUNT],
}

impl AlienCameraTransform {
    /// Rebuild and ease the camera transform using the recovered XDB arithmetic.
    pub fn update(
        &mut self,
        angles: AlienCameraAngles,
        depth_velocity: i16,
        trigonometry: &[AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
    ) {
        let pitch = normalize_angle(angles.pitch);
        let pan = normalize_angle(angles.pan);
        let secondary = normalize_angle(angles.secondary_pan);
        self.normalized_angles = [pitch, pan, secondary];
        self.target_matrix = target_matrix(pitch, pan, secondary, trigonometry);

        for row in 0..AXIS_COUNT {
            for column in 0..AXIS_COUNT {
                let current = self.matrix[row][column] as u32;
                let target = self.target_matrix[row][column] as u32;
                let delta = target.wrapping_sub(current);
                let step = (delta as i32) >> MATRIX_EASING_SHIFT;
                let rounding = (delta >> MATRIX_ROUNDING_SHIFT) & MATRIX_ROUNDING_MASK;
                self.matrix[row][column] =
                    current.wrapping_add(step as u32).wrapping_add(rounding) as i32;
            }
        }

        let depth_factor = (depth_velocity as i32).wrapping_neg() as u32;
        for axis in 0..AXIS_COUNT {
            let product = (self.matrix[Z_AXIS][axis] as u32).wrapping_mul(depth_factor);
            self.position[axis] =
                self.position[axis].wrapping_add((product as i32) >> POSITION_STEP_SHIFT);
            self.view[axis] =
                ((self.position[axis] as u32 >> POSITION_INTEGER_SHIFT) as u16) as i16;
        }

        for row in 0..AXIS_COUNT {
            let mut accumulator = u32::MIN;
            for column in 0..AXIS_COUNT {
                accumulator = accumulator.wrapping_add(
                    (self.matrix[row][column] as u32).wrapping_mul(self.view[column] as i32 as u32),
                );
            }
            self.transformed_view[row] = accumulator as i32;
        }
    }
}

fn normalize_angle(angle: i16) -> u16 {
    angle as u16 & ANGLE_MASK
}

fn angle_sample(
    trigonometry: &[AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
    angle: u16,
) -> AlienTrigonometryPair {
    trigonometry[usize::from((angle & ANGLE_MASK) >> ANGLE_TABLE_SHIFT)]
}

fn target_matrix(
    pitch: u16,
    pan: u16,
    secondary: u16,
    trigonometry: &[AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
) -> [[i32; AXIS_COUNT]; AXIS_COUNT] {
    let mut target = [[ZERO_MATRIX_COMPONENT; AXIS_COUNT]; AXIS_COUNT];
    target[Z_AXIS][Y_AXIS] = i32::from(angle_sample(trigonometry, pitch).sine)
        .wrapping_mul(DOUBLE_ANGLE_COMPONENT)
        .wrapping_neg();

    let combined = pan.wrapping_add(secondary) & ANGLE_MASK;
    let first = angle_sample(trigonometry, pitch.wrapping_sub(combined));
    let second = angle_sample(trigonometry, pitch.wrapping_add(combined));
    let axis = angle_sample(trigonometry, combined);
    let cosine_half_difference =
        i32::from(first.cosine).wrapping_sub(i32::from(second.cosine)) >> HALF_COMPONENT_SHIFT;
    let sine_half_sum =
        i32::from(first.sine).wrapping_add(i32::from(second.sine)) >> HALF_COMPONENT_SHIFT;
    let correction = cosine_half_difference.wrapping_add(i32::from(axis.sine));
    target[Y_AXIS][X_AXIS] = correction;
    target[X_AXIS][Z_AXIS] = correction.wrapping_neg();
    let correction = sine_half_sum.wrapping_add(i32::from(axis.cosine));
    target[X_AXIS][X_AXIS] = correction;
    target[Y_AXIS][Z_AXIS] = correction;

    let combined = pan.wrapping_sub(secondary) & ANGLE_MASK;
    let first = angle_sample(trigonometry, pitch.wrapping_sub(combined));
    let second = angle_sample(trigonometry, pitch.wrapping_add(combined));
    let axis = angle_sample(trigonometry, combined);
    let cosine_half_difference =
        i32::from(first.cosine).wrapping_sub(i32::from(second.cosine)) >> HALF_COMPONENT_SHIFT;
    let sine_half_sum =
        i32::from(first.sine).wrapping_add(i32::from(second.sine)) >> HALF_COMPONENT_SHIFT;
    let correction = i32::from(axis.sine).wrapping_sub(cosine_half_difference);
    target[Y_AXIS][X_AXIS] = target[Y_AXIS][X_AXIS].wrapping_sub(correction);
    target[X_AXIS][Z_AXIS] = target[X_AXIS][Z_AXIS].wrapping_sub(correction);
    let correction = i32::from(axis.cosine).wrapping_sub(sine_half_sum);
    target[X_AXIS][X_AXIS] = target[X_AXIS][X_AXIS].wrapping_add(correction);
    target[Y_AXIS][Z_AXIS] = target[Y_AXIS][Z_AXIS].wrapping_sub(correction);

    let first = angle_sample(trigonometry, secondary.wrapping_add(pitch));
    let second = angle_sample(trigonometry, secondary.wrapping_sub(pitch));
    target[Y_AXIS][Y_AXIS] = i32::from(first.cosine).wrapping_add(i32::from(second.cosine));
    target[X_AXIS][Y_AXIS] = i32::from(first.sine)
        .wrapping_add(i32::from(second.sine))
        .wrapping_neg();

    let first = angle_sample(trigonometry, pan.wrapping_add(pitch));
    let second = angle_sample(trigonometry, pan.wrapping_sub(pitch));
    target[Z_AXIS][Z_AXIS] = i32::from(first.cosine).wrapping_add(i32::from(second.cosine));
    target[Z_AXIS][X_AXIS] = i32::from(first.sine).wrapping_add(i32::from(second.sine));
    target
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct TrigonometryPattern {
        cosine_multiplier: u16,
        cosine_offset: u16,
        sine_multiplier: u16,
        sine_offset: u16,
    }

    #[derive(Deserialize)]
    struct CameraMatrixVector {
        name: String,
        trigonometry_pattern: TrigonometryPattern,
        angles_before: [u16; AXIS_COUNT],
        normalized_angles: [u16; AXIS_COUNT],
        depth_step: i16,
        camera_matrix_before: [i32; AXIS_COUNT * AXIS_COUNT],
        target_matrix: [i32; AXIS_COUNT * AXIS_COUNT],
        camera_matrix: [i32; AXIS_COUNT * AXIS_COUNT],
        camera_position_before: [i32; AXIS_COUNT],
        camera_position: [i32; AXIS_COUNT],
        view: [i16; AXIS_COUNT],
        result: [i32; AXIS_COUNT],
    }

    fn matrix(flat: [i32; AXIS_COUNT * AXIS_COUNT]) -> [[i32; AXIS_COUNT]; AXIS_COUNT] {
        std::array::from_fn(|row| std::array::from_fn(|column| flat[row * AXIS_COUNT + column]))
    }

    fn trigonometry(
        pattern: TrigonometryPattern,
    ) -> [AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT] {
        std::array::from_fn(|index| AlienTrigonometryPair {
            cosine: (index as u16)
                .wrapping_mul(pattern.cosine_multiplier)
                .wrapping_add(pattern.cosine_offset) as i16,
            sine: (index as u16)
                .wrapping_mul(pattern.sine_multiplier)
                .wrapping_add(pattern.sine_offset) as i16,
        })
    }

    #[test]
    fn camera_transform_matches_every_original_alien_overlay_vector() {
        let suites = [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_1dd8_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_1e1d_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_1edd_natural.json"),
        ];
        for json in suites {
            let vectors: Vec<CameraMatrixVector> = serde_json::from_str(json).unwrap();
            for vector in vectors {
                let table = trigonometry(vector.trigonometry_pattern);
                let mut state = AlienCameraTransform {
                    matrix: matrix(vector.camera_matrix_before),
                    position: vector.camera_position_before,
                    ..AlienCameraTransform::default()
                };
                state.update(
                    AlienCameraAngles {
                        pitch: vector.angles_before[X_AXIS] as i16,
                        pan: vector.angles_before[Y_AXIS] as i16,
                        secondary_pan: vector.angles_before[Z_AXIS] as i16,
                    },
                    vector.depth_step,
                    &table,
                );

                assert_eq!(
                    state.normalized_angles, vector.normalized_angles,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    state.target_matrix,
                    matrix(vector.target_matrix),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    state.matrix,
                    matrix(vector.camera_matrix),
                    "{}",
                    vector.name
                );
                assert_eq!(state.position, vector.camera_position, "{}", vector.name);
                assert_eq!(state.view, vector.view, "{}", vector.name);
                assert_eq!(state.transformed_view, vector.result, "{}", vector.name);
            }
        }
    }
}
