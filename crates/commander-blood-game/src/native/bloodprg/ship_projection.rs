//! Fixed-point bridge ship projection over owned points and flat framebuffers.

use std::error::Error;
use std::fmt;

use super::{LOGICAL_FRAMEBUFFER_HEIGHT, LOGICAL_FRAMEBUFFER_WIDTH};

/// Number of authored angle samples in one complete rotation.
pub const SHIP_TRIGONOMETRY_SAMPLE_COUNT: usize = 180;
/// Number of points in the bridge starfield.
pub const SHIP_POINT_CLOUD_COUNT: usize = 1_000;

const X_AXIS: usize = 0;
const Y_AXIS: usize = 1;
const Z_AXIS: usize = 2;
const SCREEN_DIMENSION: usize = 2;
const MATRIX_DIMENSION: usize = 3;
const MATRIX_FIXED_SHIFT: u32 = 15;
const PROJECTION_AXIS_SHIFT: u32 = 7;
const PROJECTION_CENTER_X: u16 = 160;
const PROJECTION_CENTER_Y: u16 = 100;
const POINT_SHADE_SHIFT: u32 = 12;
const POINT_SHADE_BASE: u8 = 239;
const Q14_TO_Q15_SCALE: i32 = 2;
const NON_VISIBLE_DEPTH_CEILING: i32 = 0;
const LOGICAL_SCREEN_ORIGIN: i16 = 0;

/// Signed Q14 cosine and sine sample from the recovered bridge angle table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipTrigonometrySample {
    /// Cosine at this two-degree step.
    pub cosine: i16,
    /// Sine at this two-degree step.
    pub sine: i16,
}

/// Observed owners of the three bridge projection angles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipProjectionAngles {
    /// Camera yaw animated by the bridge camera state machine.
    pub camera_yaw: u16,
    /// Navigation heading written by bridge steering.
    pub navigation_heading: u16,
    /// Third camera rotation; the shipped bridge initializes this to zero.
    pub camera_roll: u16,
}

/// Row-major signed Q15 bridge projection matrix.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipProjectionMatrix {
    /// Screen-x, screen-y, and depth rows in that order.
    pub rows: [[i32; MATRIX_DIMENSION]; MATRIX_DIMENSION],
}

/// One source point from the persistent bridge starfield.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipPointRecord {
    /// World-space coordinates stored as wrapping words by the original game.
    pub position: [u16; MATRIX_DIMENSION],
    /// Unrelated persistent word carried by the original eight-byte record.
    pub scratch: u16,
}

/// Current bridge camera position in the source coordinate domain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipCameraPosition {
    /// World-space camera coordinates.
    pub position: [u16; MATRIX_DIMENSION],
}

/// Half-open signed clipping rectangle used by bridge point rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipProjectionClip {
    /// Inclusive left edge.
    pub left: i16,
    /// Exclusive right edge.
    pub right: i16,
    /// Inclusive top edge.
    pub top: i16,
    /// Exclusive bottom edge.
    pub bottom: i16,
}

/// Complete logical-screen clip used by ordinary bridge rendering.
pub const FULL_SHIP_PROJECTION_CLIP: ShipProjectionClip = ShipProjectionClip {
    left: LOGICAL_SCREEN_ORIGIN,
    right: LOGICAL_FRAMEBUFFER_WIDTH as i16,
    top: LOGICAL_SCREEN_ORIGIN,
    bottom: LOGICAL_FRAMEBUFFER_HEIGHT as i16,
};

/// One positive-depth result emitted by the point-cloud projector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipProjectedPoint {
    /// Source point index in the persistent cloud.
    pub source_index: usize,
    /// Camera-relative wrapping coordinates used by the fixed-point dot products.
    pub camera_relative_point: ShipPointRecord,
    /// Logical 320-by-200 screen coordinate, possibly outside the current clip.
    pub screen: [u16; SCREEN_DIMENSION],
    /// Positive projected depth, narrowed exactly like the original output word.
    pub depth: u16,
}

/// One projected point that survived clipping and the first-write-wins test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipPlottedPoint {
    /// Complete projection result suitable for a scaled wgpu point pass.
    pub projection: ShipProjectedPoint,
    /// Index written in the original-resolution compatibility framebuffer.
    pub framebuffer_index: usize,
    /// Authored depth-derived palette index.
    pub palette_index: u8,
}

/// Output of one complete point-cloud projection pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipPointCloudProjection {
    /// Every positive-depth point passed to the native point plotter.
    pub projected: Box<[ShipProjectedPoint]>,
    /// Points that also survived clipping and framebuffer occupancy.
    pub plotted: Box<[ShipPlottedPoint]>,
    /// Final translated work record retained by the original routine.
    pub last_camera_relative_point: ShipPointRecord,
}

/// Invalid typed input to the bridge projection pipeline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShipProjectionError {
    /// An angle was outside the recovered 180-sample table.
    AngleOutOfRange {
        /// Runtime role of the invalid angle.
        role: ShipProjectionAngleRole,
        /// Invalid table index.
        angle: u16,
        /// Available table entries.
        sample_count: usize,
    },
    /// The starfield did not contain the fixed number of recovered records.
    InvalidPointCount {
        /// Supplied point count.
        actual: usize,
    },
    /// The compatibility framebuffer cannot hold one logical game frame.
    FramebufferTooShort {
        /// Supplied byte count.
        actual: usize,
    },
}

impl fmt::Display for ShipProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid bridge ship projection: {self:?}")
    }
}

impl Error for ShipProjectionError {}

/// Runtime role identifying an invalid projection angle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipProjectionAngleRole {
    /// Camera yaw animated by the bridge camera state machine.
    CameraYaw,
    /// Navigation heading written by bridge steering.
    NavigationHeading,
    /// Third camera rotation initialized to zero by the shipped bridge.
    CameraRoll,
}

/// Build the bridge's signed Q15 projection matrix.
///
/// This translates `ship_3d_projection_matrix_build` at BLOODPRG routine offset
/// `0x0098B9`. Checked table lookup replaces the original unchecked data access;
/// all multiply, add, negate, and shift operations preserve native 32-bit wrap.
pub fn build_ship_projection_matrix(
    angle_table: &[ShipTrigonometrySample],
    angles: ShipProjectionAngles,
) -> Result<ShipProjectionMatrix, ShipProjectionError> {
    let [a_cosine, a_sine] = doubled_angle_pair(
        angle_table,
        angles.camera_yaw,
        ShipProjectionAngleRole::CameraYaw,
    )?;
    let [b_cosine, b_sine] = doubled_angle_pair(
        angle_table,
        angles.navigation_heading,
        ShipProjectionAngleRole::NavigationHeading,
    )?;
    let [c_cosine, c_sine] = doubled_angle_pair(
        angle_table,
        angles.camera_roll,
        ShipProjectionAngleRole::CameraRoll,
    )?;

    let b_sine_c_sine = fixed_multiply(b_sine, c_sine);
    let c_sine_b_cosine = fixed_multiply(c_sine, b_cosine);

    Ok(ShipProjectionMatrix {
        rows: [
            [
                a_cosine
                    .wrapping_mul(b_cosine)
                    .wrapping_add(b_sine_c_sine.wrapping_mul(a_sine))
                    >> MATRIX_FIXED_SHIFT,
                c_cosine.wrapping_mul(a_sine).wrapping_neg() >> MATRIX_FIXED_SHIFT,
                c_sine_b_cosine
                    .wrapping_mul(a_sine)
                    .wrapping_sub(a_cosine.wrapping_mul(b_sine))
                    >> MATRIX_FIXED_SHIFT,
            ],
            [
                b_sine_c_sine
                    .wrapping_mul(a_cosine)
                    .wrapping_sub(a_sine.wrapping_mul(b_cosine))
                    >> MATRIX_FIXED_SHIFT,
                fixed_multiply(c_cosine, a_cosine).wrapping_neg(),
                b_sine
                    .wrapping_mul(a_sine)
                    .wrapping_add(c_sine_b_cosine.wrapping_mul(a_cosine))
                    >> MATRIX_FIXED_SHIFT,
            ],
            [
                fixed_multiply(b_sine, c_cosine),
                c_sine,
                fixed_multiply(c_cosine, b_cosine),
            ],
        ],
    })
}

/// Project all 1,000 bridge points and apply the native point plotter.
///
/// This translates `ship_3d_point_cloud_project` at BLOODPRG routine offset
/// `0x009A10`. The fixed record count is validated before any framebuffer write.
/// Positive-depth projections are retained separately from plotted points so a
/// high-resolution renderer can preserve the game geometry and ordering.
pub fn project_ship_point_cloud(
    points: &[ShipPointRecord],
    camera: ShipCameraPosition,
    matrix: ShipProjectionMatrix,
    clip: ShipProjectionClip,
    framebuffer: &mut [u8],
) -> Result<ShipPointCloudProjection, ShipProjectionError> {
    validate_framebuffer(framebuffer)?;
    if points.len() != SHIP_POINT_CLOUD_COUNT {
        return Err(ShipProjectionError::InvalidPointCount {
            actual: points.len(),
        });
    }

    let mut projected = Vec::with_capacity(SHIP_POINT_CLOUD_COUNT);
    let mut plotted = Vec::with_capacity(SHIP_POINT_CLOUD_COUNT);
    let mut last_camera_relative_point = ShipPointRecord::default();

    for (source_index, point) in points.iter().copied().enumerate() {
        last_camera_relative_point = camera_relative_point(point, camera);
        let Some(projection) = project_point(source_index, last_camera_relative_point, matrix)
        else {
            continue;
        };
        if let Some((framebuffer_index, palette_index)) =
            plot_ship_point(framebuffer, clip, projection.screen, projection.depth)?
        {
            plotted.push(ShipPlottedPoint {
                projection,
                framebuffer_index,
                palette_index,
            });
        }
        projected.push(projection);
    }

    Ok(ShipPointCloudProjection {
        projected: projected.into_boxed_slice(),
        plotted: plotted.into_boxed_slice(),
        last_camera_relative_point,
    })
}

/// Plot one projected bridge point into the original-resolution indexed frame.
///
/// This translates `ship_3d_plot_point` at BLOODPRG routine offset `0x009B04`.
/// Signed half-open clipping and the first-write-wins rule are preserved. Values
/// outside the logical screen are rejected instead of wrapping to another byte.
pub fn plot_ship_point(
    framebuffer: &mut [u8],
    clip: ShipProjectionClip,
    screen: [u16; SCREEN_DIMENSION],
    depth: u16,
) -> Result<Option<(usize, u8)>, ShipProjectionError> {
    validate_framebuffer(framebuffer)?;

    let x = screen[X_AXIS] as i16;
    let y = screen[Y_AXIS] as i16;
    if x < clip.left || x >= clip.right || y < clip.top || y >= clip.bottom {
        return Ok(None);
    }
    let Ok(x) = usize::try_from(x) else {
        return Ok(None);
    };
    let Ok(y) = usize::try_from(y) else {
        return Ok(None);
    };
    if x >= LOGICAL_FRAMEBUFFER_WIDTH || y >= LOGICAL_FRAMEBUFFER_HEIGHT {
        return Ok(None);
    }

    let framebuffer_index = y * LOGICAL_FRAMEBUFFER_WIDTH + x;
    if framebuffer[framebuffer_index] != u8::MIN {
        return Ok(None);
    }

    let palette_index = POINT_SHADE_BASE.wrapping_sub((depth >> POINT_SHADE_SHIFT) as u8);
    framebuffer[framebuffer_index] = palette_index;
    Ok(Some((framebuffer_index, palette_index)))
}

fn doubled_angle_pair(
    angle_table: &[ShipTrigonometrySample],
    angle: u16,
    role: ShipProjectionAngleRole,
) -> Result<[i32; SCREEN_DIMENSION], ShipProjectionError> {
    let angle_index = usize::from(angle);
    let sample = angle_table
        .get(angle_index)
        .filter(|_| angle_index < SHIP_TRIGONOMETRY_SAMPLE_COUNT)
        .ok_or(ShipProjectionError::AngleOutOfRange {
            role,
            angle,
            sample_count: angle_table.len(),
        })?;
    Ok([
        i32::from(sample.cosine).wrapping_mul(Q14_TO_Q15_SCALE),
        i32::from(sample.sine).wrapping_mul(Q14_TO_Q15_SCALE),
    ])
}

fn fixed_multiply(left: i32, right: i32) -> i32 {
    left.wrapping_mul(right) >> MATRIX_FIXED_SHIFT
}

fn camera_relative_point(point: ShipPointRecord, camera: ShipCameraPosition) -> ShipPointRecord {
    ShipPointRecord {
        position: std::array::from_fn(|axis| {
            point.position[axis].wrapping_sub(camera.position[axis])
        }),
        scratch: point.scratch,
    }
}

fn project_point(
    source_index: usize,
    point: ShipPointRecord,
    matrix: ShipProjectionMatrix,
) -> Option<ShipProjectedPoint> {
    let signed_position = point.position.map(|component| i32::from(component as i16));
    let depth = wrapping_dot(signed_position, matrix.rows[Z_AXIS]) >> MATRIX_FIXED_SHIFT;
    if depth <= NON_VISIBLE_DEPTH_CEILING {
        return None;
    }

    let x_axis = wrapping_dot(signed_position, matrix.rows[X_AXIS]) >> PROJECTION_AXIS_SHIFT;
    let y_axis = wrapping_dot(signed_position, matrix.rows[Y_AXIS]) >> PROJECTION_AXIS_SHIFT;
    Some(ShipProjectedPoint {
        source_index,
        camera_relative_point: point,
        screen: [
            ((x_axis / depth) as u16).wrapping_add(PROJECTION_CENTER_X),
            ((y_axis / depth) as u16).wrapping_add(PROJECTION_CENTER_Y),
        ],
        depth: depth as u16,
    })
}

fn wrapping_dot(left: [i32; MATRIX_DIMENSION], right: [i32; MATRIX_DIMENSION]) -> i32 {
    left[X_AXIS]
        .wrapping_mul(right[X_AXIS])
        .wrapping_add(left[Y_AXIS].wrapping_mul(right[Y_AXIS]))
        .wrapping_add(left[Z_AXIS].wrapping_mul(right[Z_AXIS]))
}

fn validate_framebuffer(framebuffer: &[u8]) -> Result<(), ShipProjectionError> {
    let required = LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT;
    if framebuffer.len() < required {
        return Err(ShipProjectionError::FramebufferTooShort {
            actual: framebuffer.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use sha2::{Digest, Sha256};

    use super::*;

    const MATRIX_ORACLE_COUNT: usize = 12;
    const POINT_CLOUD_ORACLE_COUNT: usize = 6;
    const PLOT_ORACLE_COUNT: usize = 14;
    const POINT_RECORD_COMPONENT_COUNT: usize = 4;
    const VALID_MATRIX_ORACLE_COUNT: usize = MATRIX_ORACLE_COUNT - 1;
    const VALID_POINT_CLOUD_ORACLE_COUNT: usize = POINT_CLOUD_ORACLE_COUNT - 1;
    const INVALID_LOGICAL_PLOT_COUNT: usize = 3;
    const POINT_X_MULTIPLIER: usize = 977;
    const POINT_Y_MULTIPLIER: usize = 613;
    const POINT_Z_MULTIPLIER: usize = 283;
    const REJECTED_POINT_Z_MULTIPLIER: usize = 37;
    const POINT_SCRATCH_MULTIPLIER: usize = 4_369;
    const POINT_X_BIAS: usize = 4_660;
    const POINT_Y_BIAS: usize = 17_185;
    const XY_SEED_SCALE: usize = 257;
    const Y_SEED_SCALE: usize = 131;
    const Z_SEED_SCALE: usize = 17;
    const NONZERO_DEPTH_BIAS: usize = 1;
    const FINAL_TEST_DEPTH: u16 = 1_000;
    const FRAMEBUFFER_TEST_VALUE: u8 = 37;
    const SHORT_FRAMEBUFFER_TEST_VALUE: u8 = 41;
    const PROJECTION_HASH_RECORD_SIZE: usize = 10;
    const CASE_SEEDS: [usize; POINT_CLOUD_ORACLE_COUNT] = [17, 43, 69, 95, 121, 147];
    const FIRST_CASE_DEPTHS: [u16; 6] = [0, 1, 1_000, 32_767, 65_535, 32_768];

    #[derive(Deserialize)]
    struct MatrixOracle {
        name: String,
        angles_a_b_c: [u16; MATRIX_DIMENSION],
        table_pairs_a_b_c: [[i16; 2]; MATRIX_DIMENSION],
        matrix: [i32; MATRIX_DIMENSION * MATRIX_DIMENSION],
    }

    #[derive(Deserialize)]
    struct PointCloudOracle {
        name: String,
        runtime_ds_equals_gs: bool,
        iterations: usize,
        matrix: [i32; MATRIX_DIMENSION * MATRIX_DIMENSION],
        camera: [u16; MATRIX_DIMENSION],
        plot_calls: usize,
        plot_sequence_sha256: String,
        first_plot: Option<ProjectionCallOracle>,
        last_plot: Option<ProjectionCallOracle>,
        final_work: [u16; POINT_RECORD_COMPONENT_COUNT],
    }

    #[derive(Deserialize)]
    struct ProjectionCallOracle {
        point_index: usize,
        projected: [u16; MATRIX_DIMENSION],
        work: [u16; POINT_RECORD_COMPONENT_COUNT],
    }

    #[derive(Deserialize)]
    struct PlotOracle {
        name: String,
        x: i16,
        y: i16,
        depth: u16,
        clip: [i16; POINT_RECORD_COMPONENT_COUNT],
        outcome: String,
        natural_offset: Option<usize>,
        pixel_before: u8,
        pixel_after: u8,
    }

    #[test]
    fn projection_matrix_matches_every_typed_original_vector() {
        let vectors: Vec<MatrixOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_98b9_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), MATRIX_ORACLE_COUNT);

        let mut matched = usize::MIN;
        for vector in vectors {
            let mut table = vec![ShipTrigonometrySample::default(); SHIP_TRIGONOMETRY_SAMPLE_COUNT];
            for (angle, pair) in vector
                .angles_a_b_c
                .into_iter()
                .zip(vector.table_pairs_a_b_c)
            {
                if let Some(sample) = table.get_mut(usize::from(angle)) {
                    *sample = ShipTrigonometrySample {
                        cosine: pair[X_AXIS],
                        sine: pair[Y_AXIS],
                    };
                }
            }
            let angles = ShipProjectionAngles {
                camera_yaw: vector.angles_a_b_c[X_AXIS],
                navigation_heading: vector.angles_a_b_c[Y_AXIS],
                camera_roll: vector.angles_a_b_c[Z_AXIS],
            };

            match build_ship_projection_matrix(&table, angles) {
                Ok(matrix) => {
                    assert_eq!(flatten(matrix), vector.matrix, "{}", vector.name);
                    matched += 1;
                }
                Err(ShipProjectionError::AngleOutOfRange { angle, .. }) => {
                    assert_eq!(angle, SHIP_TRIGONOMETRY_SAMPLE_COUNT as u16);
                    assert_eq!(vector.name, "angle_boundaries");
                }
                Err(error) => panic!("{}: {error}", vector.name),
            }
        }
        assert_eq!(matched, VALID_MATRIX_ORACLE_COUNT);
    }

    #[test]
    fn point_cloud_projection_matches_every_flat_original_vector() {
        let vectors: Vec<PointCloudOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_9a10_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), POINT_CLOUD_ORACLE_COUNT);

        for (case_index, vector) in vectors.iter().enumerate() {
            let points = point_cloud_fixture(case_index, vector);
            let matrix = matrix_from_flat(vector.matrix);
            let mut framebuffer =
                vec![u8::MIN; LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT];
            let result = project_ship_point_cloud(
                &points,
                ShipCameraPosition {
                    position: vector.camera,
                },
                matrix,
                FULL_SHIP_PROJECTION_CLIP,
                &mut framebuffer,
            )
            .unwrap();

            if case_index < VALID_POINT_CLOUD_ORACLE_COUNT {
                assert!(vector.runtime_ds_equals_gs, "{}", vector.name);
                assert_eq!(vector.iterations, SHIP_POINT_CLOUD_COUNT, "{}", vector.name);
                assert_eq!(result.projected.len(), vector.plot_calls, "{}", vector.name);
                assert_eq!(
                    projection_hash(&result.projected),
                    vector.plot_sequence_sha256
                );
                assert_projection(vector.first_plot.as_ref(), result.projected.first());
                assert_projection(vector.last_plot.as_ref(), result.projected.last());
                assert_eq!(
                    record_words(result.last_camera_relative_point),
                    vector.final_work,
                    "{}",
                    vector.name
                );
                continue;
            }

            assert!(!vector.runtime_ds_equals_gs);
            assert_eq!(vector.iterations, MATRIX_DIMENSION);
            assert!(result.projected.len() > vector.plot_calls);
            assert_eq!(
                projection_hash(&result.projected[..vector.plot_calls]),
                vector.plot_sequence_sha256
            );
        }
    }

    #[test]
    fn point_plot_matches_valid_vectors_and_rejects_wrapping_coordinates() {
        let vectors: Vec<PlotOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_9b04_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), PLOT_ORACLE_COUNT);

        let mut invalid_logical_coordinates = usize::MIN;
        for vector in vectors {
            let mut framebuffer =
                vec![u8::MIN; LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT];
            let logical_x = usize::try_from(vector.x);
            let logical_y = usize::try_from(vector.y);
            let logical_position = logical_x
                .ok()
                .zip(logical_y.ok())
                .filter(|(x, y)| *x < LOGICAL_FRAMEBUFFER_WIDTH && *y < LOGICAL_FRAMEBUFFER_HEIGHT);
            if let Some((x, y)) = logical_position {
                framebuffer[y * LOGICAL_FRAMEBUFFER_WIDTH + x] = vector.pixel_before;
            }
            let before = framebuffer.clone();
            let result = plot_ship_point(
                &mut framebuffer,
                ShipProjectionClip {
                    left: vector.clip[X_AXIS],
                    right: vector.clip[Y_AXIS],
                    top: vector.clip[Z_AXIS],
                    bottom: vector.clip[POINT_RECORD_COMPONENT_COUNT - 1],
                },
                [vector.x as u16, vector.y as u16],
                vector.depth,
            )
            .unwrap();

            let Some((x, y)) = logical_position else {
                if vector.outcome == "draw" {
                    invalid_logical_coordinates += 1;
                }
                assert_eq!(result, None, "{}", vector.name);
                assert_eq!(framebuffer, before, "{}", vector.name);
                continue;
            };
            let offset = y * LOGICAL_FRAMEBUFFER_WIDTH + x;
            if vector.outcome == "draw" {
                assert_eq!(
                    result,
                    Some((offset, vector.pixel_after)),
                    "{}",
                    vector.name
                );
                assert_eq!(vector.natural_offset, Some(offset), "{}", vector.name);
            } else {
                assert_eq!(result, None, "{}", vector.name);
            }
            assert_eq!(framebuffer[offset], vector.pixel_after, "{}", vector.name);
        }
        assert_eq!(invalid_logical_coordinates, INVALID_LOGICAL_PLOT_COUNT);
    }

    #[test]
    fn malformed_flat_inputs_fail_before_framebuffer_mutation() {
        let matrix = ShipProjectionMatrix::default();
        let points = vec![ShipPointRecord::default(); SHIP_POINT_CLOUD_COUNT - 1];
        let mut framebuffer =
            vec![FRAMEBUFFER_TEST_VALUE; LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT];
        let before = framebuffer.clone();
        assert_eq!(
            project_ship_point_cloud(
                &points,
                ShipCameraPosition::default(),
                matrix,
                FULL_SHIP_PROJECTION_CLIP,
                &mut framebuffer,
            ),
            Err(ShipProjectionError::InvalidPointCount {
                actual: SHIP_POINT_CLOUD_COUNT - 1,
            })
        );
        assert_eq!(framebuffer, before);

        let mut short_framebuffer = vec![SHORT_FRAMEBUFFER_TEST_VALUE; LOGICAL_FRAMEBUFFER_WIDTH];
        let short_before = short_framebuffer.clone();
        assert_eq!(
            project_ship_point_cloud(
                &vec![ShipPointRecord::default(); SHIP_POINT_CLOUD_COUNT],
                ShipCameraPosition::default(),
                matrix,
                FULL_SHIP_PROJECTION_CLIP,
                &mut short_framebuffer,
            ),
            Err(ShipProjectionError::FramebufferTooShort {
                actual: LOGICAL_FRAMEBUFFER_WIDTH,
            })
        );
        assert_eq!(short_framebuffer, short_before);
    }

    fn point_cloud_fixture(case_index: usize, vector: &PointCloudOracle) -> Vec<ShipPointRecord> {
        let seed = CASE_SEEDS[case_index];
        let mut points: Vec<ShipPointRecord> = (usize::MIN..SHIP_POINT_CLOUD_COUNT)
            .map(|point_index| {
                let x =
                    (point_index * POINT_X_MULTIPLIER + seed * XY_SEED_SCALE + POINT_X_BIAS) as u16;
                let y =
                    (point_index * POINT_Y_MULTIPLIER + seed * Y_SEED_SCALE + POINT_Y_BIAS) as u16;
                let z = if case_index == usize::MIN {
                    FIRST_CASE_DEPTHS[point_index % FIRST_CASE_DEPTHS.len()]
                } else if case_index == MATRIX_DIMENSION {
                    ((point_index * REJECTED_POINT_Z_MULTIPLIER + NONZERO_DEPTH_BIAS)
                        & i16::MAX as usize) as u16
                } else {
                    (point_index * POINT_Z_MULTIPLIER + seed * Z_SEED_SCALE + NONZERO_DEPTH_BIAS)
                        as u16
                };
                ShipPointRecord {
                    position: [x, y, z],
                    scratch: (point_index * POINT_SCRATCH_MULTIPLIER + seed * XY_SEED_SCALE) as u16,
                }
            })
            .collect();

        let final_index = vector.iterations - 1;
        points[final_index].position = [
            vector.camera[X_AXIS],
            vector.camera[Y_AXIS],
            vector.camera[Z_AXIS].wrapping_add(FINAL_TEST_DEPTH),
        ];
        points
    }

    fn assert_projection(
        expected: Option<&ProjectionCallOracle>,
        actual: Option<&ShipProjectedPoint>,
    ) {
        match (expected, actual) {
            (None, None) => {}
            (Some(expected), Some(actual)) => {
                assert_eq!(actual.source_index, expected.point_index);
                assert_eq!(
                    [actual.screen[X_AXIS], actual.screen[Y_AXIS], actual.depth],
                    expected.projected
                );
                assert_eq!(
                    [
                        actual.camera_relative_point.position[X_AXIS],
                        actual.camera_relative_point.position[Y_AXIS],
                        actual.camera_relative_point.position[Z_AXIS],
                        actual.camera_relative_point.scratch,
                    ],
                    expected.work
                );
            }
            _ => panic!("projection presence differs"),
        }
    }

    fn projection_hash(projected: &[ShipProjectedPoint]) -> String {
        let mut bytes = Vec::with_capacity(projected.len() * PROJECTION_HASH_RECORD_SIZE);
        for point in projected {
            bytes.extend_from_slice(&(point.source_index as u32).to_le_bytes());
            bytes.extend_from_slice(&point.screen[X_AXIS].to_le_bytes());
            bytes.extend_from_slice(&point.screen[Y_AXIS].to_le_bytes());
            bytes.extend_from_slice(&point.depth.to_le_bytes());
        }
        format!("{:x}", Sha256::digest(bytes))
    }

    fn matrix_from_flat(
        values: [i32; MATRIX_DIMENSION * MATRIX_DIMENSION],
    ) -> ShipProjectionMatrix {
        ShipProjectionMatrix {
            rows: [
                [values[0], values[1], values[2]],
                [values[3], values[4], values[5]],
                [values[6], values[7], values[8]],
            ],
        }
    }

    fn flatten(matrix: ShipProjectionMatrix) -> [i32; MATRIX_DIMENSION * MATRIX_DIMENSION] {
        [
            matrix.rows[0][0],
            matrix.rows[0][1],
            matrix.rows[0][2],
            matrix.rows[1][0],
            matrix.rows[1][1],
            matrix.rows[1][2],
            matrix.rows[2][0],
            matrix.rows[2][1],
            matrix.rows[2][2],
        ]
    }

    fn record_words(record: ShipPointRecord) -> [u16; POINT_RECORD_COMPONENT_COUNT] {
        [
            record.position[X_AXIS],
            record.position[Y_AXIS],
            record.position[Z_AXIS],
            record.scratch,
        ]
    }
}
