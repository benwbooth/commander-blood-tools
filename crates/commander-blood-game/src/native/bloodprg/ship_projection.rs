//! Fixed-point bridge ship projection over owned points and flat framebuffers.

use std::error::Error;
use std::fmt;

use commander_blood_formats::bloodprg::{
    BLOODPRG_BRIDGE_PROJECTION_ANCHOR_COUNT, BLOODPRG_BRIDGE_TRIGONOMETRY_SAMPLE_COUNT,
    BloodprgBridgeResources,
};

use crate::native::random::BloodPrng;

use super::{
    BRIDGE_SPRITE_ENTITY_COUNT, BridgeSpriteEntity, BridgeSpriteEntityError, BridgeSpriteExtent,
    BridgeSpritePosition, LOGICAL_FRAMEBUFFER_HEIGHT, LOGICAL_FRAMEBUFFER_WIDTH,
    update_bridge_sprite_extent, update_bridge_sprite_position,
};

/// Number of authored angle samples in one complete rotation.
pub const SHIP_TRIGONOMETRY_SAMPLE_COUNT: usize = BLOODPRG_BRIDGE_TRIGONOMETRY_SAMPLE_COUNT;
/// Number of points in the bridge starfield.
pub const SHIP_POINT_CLOUD_COUNT: usize = 1_000;
/// Number of navigation-object anchors projected over the bridge starfield.
pub const SHIP_OBJECT_ANCHOR_COUNT: usize = BLOODPRG_BRIDGE_PROJECTION_ANCHOR_COUNT;

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
const POINT_RANDOM_MODULUS: u16 = u16::MAX;
const FIRST_NAVIGATION_ENTITY_INDEX: usize = 21;
const ENTITY_INDEX_STEP: usize = 1;
const OBJECT_DEPTH_WRAP: i32 = 65_536;
const OBJECT_SCALE_NUMERATOR: u32 = 1_048_576;
const OBJECT_DIMENSION_SHIFT: u32 = 10;
const HALF_EXTENT_SHIFT: u32 = 1;

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

/// One world-space anchor for a bridge navigation object sprite.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipObjectAnchor {
    /// World-space coordinates stored as wrapping words by the original game.
    pub position: [u16; MATRIX_DIMENSION],
}

/// Owned authored resources consumed by the bridge projection pipeline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipProjectionResources {
    /// Exact two-degree trigonometry table decoded from the executable image.
    pub trigonometry: [ShipTrigonometrySample; SHIP_TRIGONOMETRY_SAMPLE_COUNT],
    /// Exact eleven-anchor projector input decoded from the executable image.
    pub object_anchors: [ShipObjectAnchor; SHIP_OBJECT_ANCHOR_COUNT],
}

impl From<BloodprgBridgeResources> for ShipProjectionResources {
    fn from(resources: BloodprgBridgeResources) -> Self {
        Self {
            trigonometry: resources.trigonometry.map(|sample| ShipTrigonometrySample {
                cosine: sample.cosine,
                sine: sample.sine,
            }),
            object_anchors: resources.projection_anchors.map(|anchor| ShipObjectAnchor {
                position: anchor.position,
            }),
        }
    }
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

/// One visible, nonzero-depth navigation-object projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipObjectSpriteProjection {
    /// Source anchor index in forward authored order.
    pub anchor_index: usize,
    /// Destination bridge entity index, assigned in reverse order.
    pub entity_index: usize,
    /// Camera-relative wrapping coordinates used by the dot products.
    pub camera_relative_position: [u16; MATRIX_DIMENSION],
    /// Logical screen center calculated before sprite centering.
    pub screen: [u16; SCREEN_DIMENSION],
    /// Wrapped positive depth used by perspective division.
    pub depth: u16,
    /// Reciprocal depth scale used for source dimensions.
    pub depth_scale: u16,
    /// Perspective-scaled destination extent requested from the entity helper.
    pub scaled_extent: BridgeSpriteExtent,
    /// Centered logical position requested from the entity helper.
    pub draw_position: BridgeSpritePosition,
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
    /// The navigation-object list did not contain its fixed authored anchors.
    InvalidObjectAnchorCount {
        /// Supplied anchor count.
        actual: usize,
    },
    /// The entity table could not contain navigation entities 21 through 31.
    InvalidSpriteEntityCount {
        /// Supplied entity count.
        actual: usize,
        /// Minimum required entity count.
        required: usize,
    },
    /// An entity index became invalid after the complete table was validated.
    SpriteEntity(BridgeSpriteEntityError),
    /// Native negative-depth wrapping produced a non-positive divisor.
    InvalidWrappedObjectDepth {
        /// Anchor being projected.
        anchor_index: usize,
        /// Wrapped depth that could not be divided by.
        depth: i32,
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

impl From<BridgeSpriteEntityError> for ShipProjectionError {
    fn from(error: BridgeSpriteEntityError) -> Self {
        Self::SpriteEntity(error)
    }
}

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

/// Fill the persistent bridge point cloud from Commander Blood's PRNG.
///
/// This translates `ship_3d_point_cloud_randomize` at BLOODPRG routine offset
/// `0x009B67`. Exactly three random values populate each owned point record and
/// the persistent scratch word is intentionally left unchanged.
pub fn randomize_ship_point_cloud(
    points: &mut [ShipPointRecord],
    random: &mut BloodPrng,
) -> Result<(), ShipProjectionError> {
    if points.len() != SHIP_POINT_CLOUD_COUNT {
        return Err(ShipProjectionError::InvalidPointCount {
            actual: points.len(),
        });
    }
    fill_ship_point_cloud(points, || random.next(POINT_RANDOM_MODULUS));
    Ok(())
}

/// Project bridge navigation anchors into their sprite entities.
///
/// This translates `ship_3d_object_sprite_project` at BLOODPRG routine offset
/// `0x009B98`. Eleven owned anchors map forward to entity indices 31 through 21.
/// The extent helper's historical ambient input is supplied as an explicit typed
/// value, while decoded source dimensions and mutable entity geometry stay owned.
pub fn project_ship_object_sprites(
    anchors: &[ShipObjectAnchor],
    camera: ShipCameraPosition,
    matrix: ShipProjectionMatrix,
    extent_comparison: BridgeSpriteExtent,
    entities: &mut [BridgeSpriteEntity],
) -> Result<Box<[ShipObjectSpriteProjection]>, ShipProjectionError> {
    project_ship_object_sprites_with_extent_comparison(anchors, camera, matrix, entities, |_, _| {
        extent_comparison
    })
}

/// Project bridge navigation anchors using each entity's decoded source extent.
///
/// The original call leaves `BP` pointing at its projection matrix while the
/// extent helper interprets matrix bytes as a far comparison pointer. That
/// segment alias is preserved by [`project_ship_object_sprites`] for oracle
/// work, but it has no valid owner in a flat address space. Runtime rendering
/// uses the source-frame dimensions that the helper is logically meant to
/// compare against, clearing the scaled flag at the sprite's natural size.
pub fn project_ship_object_sprites_against_source_extent(
    anchors: &[ShipObjectAnchor],
    camera: ShipCameraPosition,
    matrix: ShipProjectionMatrix,
    entities: &mut [BridgeSpriteEntity],
) -> Result<Box<[ShipObjectSpriteProjection]>, ShipProjectionError> {
    project_ship_object_sprites_with_extent_comparison(
        anchors,
        camera,
        matrix,
        entities,
        |_, entity| entity.source_extent,
    )
}

fn project_ship_object_sprites_with_extent_comparison(
    anchors: &[ShipObjectAnchor],
    camera: ShipCameraPosition,
    matrix: ShipProjectionMatrix,
    entities: &mut [BridgeSpriteEntity],
    mut extent_comparison_for: impl FnMut(usize, &BridgeSpriteEntity) -> BridgeSpriteExtent,
) -> Result<Box<[ShipObjectSpriteProjection]>, ShipProjectionError> {
    if anchors.len() != SHIP_OBJECT_ANCHOR_COUNT {
        return Err(ShipProjectionError::InvalidObjectAnchorCount {
            actual: anchors.len(),
        });
    }
    if entities.len() < BRIDGE_SPRITE_ENTITY_COUNT {
        return Err(ShipProjectionError::InvalidSpriteEntityCount {
            actual: entities.len(),
            required: BRIDGE_SPRITE_ENTITY_COUNT,
        });
    }

    let mut staged_entities = entities.to_vec();
    let mut projections = Vec::with_capacity(SHIP_OBJECT_ANCHOR_COUNT);
    for (anchor_index, anchor) in anchors.iter().copied().enumerate() {
        let entity_index = FIRST_NAVIGATION_ENTITY_INDEX
            + (SHIP_OBJECT_ANCHOR_COUNT - ENTITY_INDEX_STEP - anchor_index);
        if !staged_entities[entity_index].flags.is_visible() {
            continue;
        }

        let camera_relative_position =
            std::array::from_fn(|axis| anchor.position[axis].wrapping_sub(camera.position[axis]));
        let signed_position = camera_relative_position.map(|component| i32::from(component as i16));
        let raw_depth = wrapping_dot(signed_position, matrix.rows[Z_AXIS]) >> MATRIX_FIXED_SHIFT;
        if raw_depth == NON_VISIBLE_DEPTH_CEILING {
            continue;
        }
        let depth = if raw_depth < NON_VISIBLE_DEPTH_CEILING {
            raw_depth.wrapping_add(OBJECT_DEPTH_WRAP)
        } else {
            raw_depth
        };
        let depth_divisor =
            u32::try_from(depth).map_err(|_| ShipProjectionError::InvalidWrappedObjectDepth {
                anchor_index,
                depth,
            })?;
        if depth_divisor == u32::MIN {
            return Err(ShipProjectionError::InvalidWrappedObjectDepth {
                anchor_index,
                depth,
            });
        }

        let depth_scale = (OBJECT_SCALE_NUMERATOR / depth_divisor) as u16;
        let x_axis = wrapping_dot(signed_position, matrix.rows[X_AXIS]) >> PROJECTION_AXIS_SHIFT;
        let y_axis = wrapping_dot(signed_position, matrix.rows[Y_AXIS]) >> PROJECTION_AXIS_SHIFT;
        let screen = [
            ((x_axis / depth) as u16).wrapping_add(PROJECTION_CENTER_X),
            ((y_axis / depth) as u16).wrapping_add(PROJECTION_CENTER_Y),
        ];
        let source_extent = staged_entities[entity_index].source_extent;
        let scaled_extent = BridgeSpriteExtent {
            width: scale_object_dimension(source_extent.width, depth_scale),
            height: scale_object_dimension(source_extent.height, depth_scale),
        };
        let extent_comparison = extent_comparison_for(entity_index, &staged_entities[entity_index]);
        update_bridge_sprite_extent(
            &mut staged_entities,
            entity_index,
            scaled_extent,
            extent_comparison,
        )?;

        let extent = staged_entities[entity_index].extent;
        let draw_position = BridgeSpritePosition {
            x: screen[X_AXIS].wrapping_sub(extent.width >> HALF_EXTENT_SHIFT),
            y: screen[Y_AXIS].wrapping_sub(extent.height >> HALF_EXTENT_SHIFT),
        };
        update_bridge_sprite_position(&mut staged_entities, entity_index, draw_position)?;
        projections.push(ShipObjectSpriteProjection {
            anchor_index,
            entity_index,
            camera_relative_position,
            screen,
            depth: depth as u16,
            depth_scale,
            scaled_extent,
            draw_position,
        });
    }

    entities.copy_from_slice(&staged_entities);
    Ok(projections.into_boxed_slice())
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

fn scale_object_dimension(dimension: u16, depth_scale: u16) -> u16 {
    (u32::from(dimension).wrapping_mul(u32::from(depth_scale)) >> OBJECT_DIMENSION_SHIFT) as u16
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

fn fill_ship_point_cloud(points: &mut [ShipPointRecord], mut next_random: impl FnMut() -> u16) {
    for point in points {
        point.position = std::array::from_fn(|_| next_random());
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use sha2::{Digest, Sha256};

    use super::super::BridgeSpriteFlags;
    use super::*;

    const MATRIX_ORACLE_COUNT: usize = 12;
    const POINT_CLOUD_ORACLE_COUNT: usize = 6;
    const PLOT_ORACLE_COUNT: usize = 14;
    const RANDOMIZE_ORACLE_COUNT: usize = 4;
    const OBJECT_ORACLE_COUNT: usize = 5;
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
    const POINT_RECORD_SIZE: usize = 8;
    const POINT_POSITION_BYTE_COUNT: usize = 6;
    const RANDOM_OUTPUT_COUNT: usize = SHIP_POINT_CLOUD_COUNT * MATRIX_DIMENSION;
    const RANDOM_INITIAL_BYTE_MULTIPLIER: usize = 17;
    const RANDOM_CASE_SEEDS: [usize; RANDOMIZE_ORACLE_COUNT] = [17, 55, 93, 131];
    const RAMP_START: usize = 4_660;
    const RAMP_STEP: usize = 37;
    const LCG_INITIAL_STATE: u16 = 44_257;
    const LCG_MULTIPLIER: u16 = 25_173;
    const LCG_INCREMENT: u16 = 13_849;
    const EXTREMA_CYCLE: [u16; 6] = [0, 1, 32_767, 32_768, 65_533, 65_534];
    const OBJECT_CASE_SEEDS: [usize; OBJECT_ORACLE_COUNT] = [17, 43, 69, 95, 121];
    const OBJECT_EVENT_WORD_COUNT: usize = 6;
    const OBJECT_EVENT_BYTE_COUNT: usize = 1 + OBJECT_EVENT_WORD_COUNT * 2;
    const EXTENT_EVENT_TAG: u8 = 0;
    const POSITION_EVENT_TAG: u8 = 1;
    const MIXED_VISIBILITY_CASE: usize = 1;
    const NEGATIVE_DEPTH_CASE: usize = 2;
    const EQUAL_SOURCE_EXTENT_CASE: usize = 3;
    const OVERFLOW_MATRIX_CASE: usize = 4;
    const COMPARISON_WIDTH_BASE: usize = 32;
    const COMPARISON_HEIGHT_BASE: usize = 24;
    const COMPARISON_WIDTH_STEP: usize = 3;
    const COMPARISON_HEIGHT_STEP: usize = 5;
    const ORACLE_ZERO: i32 = 0;
    const ORACLE_POSITIVE_UNIT: i32 = 32_768;
    const ORACLE_NEGATIVE_UNIT: i32 = -32_768;
    const ORACLE_MATRIX_COMPARISON_VALUE: i32 = 1_744_830_976;
    const OVERFLOW_MATRIX_X_X: i32 = 1_879_048_193;
    const OVERFLOW_MATRIX_X_Z: i32 = -54_880_137;
    const OVERFLOW_MATRIX_Y_X: i32 = 1_790_762_751;
    const OVERFLOW_MATRIX_Y_Y: i32 = -19_088_743;
    const OBJECT_CAMERA_X_BASE: usize = 256;
    const OBJECT_CAMERA_Y: u16 = 33_280;
    const OBJECT_CAMERA_Z: u16 = 65_280;
    const OBJECT_ANCHOR_X_BASE: usize = 4_608;
    const OBJECT_ANCHOR_Y_BASE: usize = 17_152;
    const OBJECT_ANCHOR_X_SEED_SCALE: usize = 17;
    const OBJECT_ANCHOR_Y_SEED_SCALE: usize = 11;
    const OBJECT_ANCHOR_X_STEP: usize = 977;
    const OBJECT_ANCHOR_Y_STEP: usize = 613;
    const OBJECT_POSITIVE_DEPTH_BASE: usize = 900;
    const OBJECT_DEPTH_STEP: usize = 37;
    const OBJECT_NEGATIVE_DEPTH_BASE: usize = 1_000;
    const OBJECT_NEGATIVE_DEPTH_STEP: usize = 17;
    const FINAL_OBJECT_DEPTH: u16 = 1_024;
    const ENTITY_FLAG_HIGH_BITS: u16 = 43_776;
    const ENTITY_STATE_ZERO_BIT: u16 = 1;
    const ENTITY_VISIBLE_BIT: u16 = 128;
    const ENTITY_EXTENT_CHANGED_BIT: u16 = 16;
    const ENTITY_DRAW_X_BASE: usize = 28_672;
    const ENTITY_DRAW_Y_BASE: usize = 32_768;
    const ENTITY_DRAW_X_STEP: usize = 13;
    const ENTITY_DRAW_Y_STEP: usize = 17;
    const ENTITY_EXTENT_WIDTH_BASE: usize = 9;
    const ENTITY_EXTENT_HEIGHT_BASE: usize = 11;
    const ENTITY_SOURCE_WIDTH_BASE: usize = 17;
    const ENTITY_SOURCE_HEIGHT_BASE: usize = 13;
    const ENTITY_SOURCE_WIDTH_STEP: usize = 2;
    const ENTITY_SOURCE_HEIGHT_STEP: usize = 3;
    const VISIBILITY_PATTERN_DIVISOR: usize = 3;
    const HIDDEN_VISIBILITY_REMAINDER: usize = 1;
    const FIRST_ZERO_DEPTH_ANCHOR: usize = 2;
    const SECOND_ZERO_DEPTH_ANCHOR: usize = 7;
    const FINAL_OVERREAD_BASE: usize = 42_240;
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

    #[derive(Deserialize)]
    struct RandomizeOracle {
        name: String,
        prng_call_count: usize,
        prng_outputs_sha256: String,
        first_record: [u16; POINT_RECORD_COMPONENT_COUNT],
        last_record: [u16; POINT_RECORD_COMPONENT_COUNT],
        point_cloud_sha256: String,
        scratch_sha256: String,
    }

    #[derive(Deserialize)]
    struct ObjectProjectionOracle {
        name: String,
        anchors: usize,
        entity_ids_in_order: Vec<usize>,
        helper_events: usize,
        extent_comparison_loads: usize,
        helper_sequence_sha256: String,
        first_event: serde_json::Value,
        last_event: serde_json::Value,
        final_work: [u16; POINT_RECORD_COMPONENT_COUNT],
    }

    struct ObjectProjectionFixture {
        seed: usize,
        anchors: Vec<ShipObjectAnchor>,
        camera: ShipCameraPosition,
        matrix: ShipProjectionMatrix,
        comparison: BridgeSpriteExtent,
        entities: Vec<BridgeSpriteEntity>,
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
    fn point_cloud_randomizer_matches_every_original_call_order_vector() {
        let vectors: Vec<RandomizeOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_9b67_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), RANDOMIZE_ORACLE_COUNT);

        for (case_index, vector) in vectors.into_iter().enumerate() {
            let outputs = random_outputs(case_index);
            assert_eq!(outputs.len(), vector.prng_call_count, "{}", vector.name);
            assert_eq!(
                hash_words(&outputs),
                vector.prng_outputs_sha256,
                "{}",
                vector.name
            );

            let seed = RANDOM_CASE_SEEDS[case_index];
            let raw_before: Vec<u8> = (usize::MIN..SHIP_POINT_CLOUD_COUNT * POINT_RECORD_SIZE)
                .map(|index| (seed + index * RANDOM_INITIAL_BYTE_MULTIPLIER) as u8)
                .collect();
            let mut points: Vec<ShipPointRecord> = raw_before
                .chunks_exact(POINT_RECORD_SIZE)
                .map(decode_point_record)
                .collect();
            let mut output_index = usize::MIN;
            fill_ship_point_cloud(&mut points, || {
                let value = outputs[output_index];
                output_index += 1;
                value
            });
            assert_eq!(output_index, RANDOM_OUTPUT_COUNT, "{}", vector.name);

            let raw_after = encode_point_cloud(&points);
            let scratch: Vec<u8> = raw_after
                .chunks_exact(POINT_RECORD_SIZE)
                .flat_map(|record| record[POINT_POSITION_BYTE_COUNT..].iter().copied())
                .collect();
            assert_eq!(
                record_words(points[usize::MIN]),
                vector.first_record,
                "{}",
                vector.name
            );
            assert_eq!(
                record_words(points[SHIP_POINT_CLOUD_COUNT - 1]),
                vector.last_record,
                "{}",
                vector.name
            );
            assert_eq!(
                format!("{:x}", Sha256::digest(raw_after)),
                vector.point_cloud_sha256,
                "{}",
                vector.name
            );
            assert_eq!(
                format!("{:x}", Sha256::digest(scratch)),
                vector.scratch_sha256,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn object_sprite_projection_matches_every_typed_original_vector() {
        let vectors: Vec<ObjectProjectionOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_9b98_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), OBJECT_ORACLE_COUNT);

        for (case_index, vector) in vectors.into_iter().enumerate() {
            let fixture = object_projection_fixture(case_index);
            assert_eq!(vector.anchors, SHIP_OBJECT_ANCHOR_COUNT, "{}", vector.name);
            assert_eq!(
                vector.entity_ids_in_order,
                (usize::MIN..SHIP_OBJECT_ANCHOR_COUNT)
                    .map(|anchor_index| {
                        FIRST_NAVIGATION_ENTITY_INDEX
                            + (SHIP_OBJECT_ANCHOR_COUNT - ENTITY_INDEX_STEP - anchor_index)
                    })
                    .collect::<Vec<_>>(),
                "{}",
                vector.name
            );

            let final_entity_visible = fixture.entities[FIRST_NAVIGATION_ENTITY_INDEX]
                .flags
                .is_visible();
            let mut entities = fixture.entities;
            let projections = project_ship_object_sprites(
                &fixture.anchors,
                fixture.camera,
                fixture.matrix,
                fixture.comparison,
                &mut entities,
            )
            .unwrap();
            assert_eq!(
                vector.helper_events,
                projections.len() * 2,
                "{}",
                vector.name
            );
            assert_eq!(
                vector.extent_comparison_loads,
                projections.len(),
                "{}",
                vector.name
            );
            assert_eq!(
                object_projection_hash(&projections),
                vector.helper_sequence_sha256,
                "{}",
                vector.name
            );
            assert_eq!(
                object_extent_event(projections.first().unwrap()),
                vector.first_event,
                "{}",
                vector.name
            );
            assert_eq!(
                object_position_event(projections.last().unwrap()),
                vector.last_event,
                "{}",
                vector.name
            );

            let final_anchor = fixture.anchors[SHIP_OBJECT_ANCHOR_COUNT - ENTITY_INDEX_STEP];
            let expected_final_position = if final_entity_visible {
                std::array::from_fn(|axis| {
                    final_anchor.position[axis].wrapping_sub(fixture.camera.position[axis])
                })
            } else {
                final_anchor.position
            };
            assert_eq!(
                vector.final_work[..MATRIX_DIMENSION],
                expected_final_position,
                "{}",
                vector.name
            );
            // The fourth oracle word is an unused source-window lookahead. It is
            // checked as evidence but intentionally has no runtime representation.
            assert_eq!(
                vector.final_work[POINT_RECORD_COMPONENT_COUNT - ENTITY_INDEX_STEP],
                (FINAL_OVERREAD_BASE + fixture.seed) as u16,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn flat_runtime_compares_projected_extents_with_each_source_frame() {
        const NATURAL_DEPTH: u16 = 1_024;
        const SOURCE_WIDTH: u16 = 40;
        const SOURCE_HEIGHT: u16 = 24;

        let mut anchors = [ShipObjectAnchor::default(); SHIP_OBJECT_ANCHOR_COUNT];
        anchors[SHIP_OBJECT_ANCHOR_COUNT - ENTITY_INDEX_STEP].position[Z_AXIS] = NATURAL_DEPTH;
        let matrix = ShipProjectionMatrix {
            rows: [
                [ORACLE_POSITIVE_UNIT, ORACLE_ZERO, ORACLE_ZERO],
                [ORACLE_ZERO, ORACLE_POSITIVE_UNIT, ORACLE_ZERO],
                [ORACLE_ZERO, ORACLE_ZERO, ORACLE_POSITIVE_UNIT],
            ],
        };
        let source_extent = BridgeSpriteExtent {
            width: SOURCE_WIDTH,
            height: SOURCE_HEIGHT,
        };
        let mut entities = [BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT];
        entities[FIRST_NAVIGATION_ENTITY_INDEX] = BridgeSpriteEntity {
            flags: BridgeSpriteFlags::from_bits(
                ENTITY_STATE_ZERO_BIT | ENTITY_VISIBLE_BIT | ENTITY_EXTENT_CHANGED_BIT,
            ),
            source_extent,
            extent: source_extent,
            ..BridgeSpriteEntity::default()
        };

        let projections = project_ship_object_sprites_against_source_extent(
            &anchors,
            ShipCameraPosition::default(),
            matrix,
            &mut entities,
        )
        .unwrap();

        assert_eq!(projections.len(), 1);
        assert_eq!(projections[0].scaled_extent, source_extent);
        assert!(
            !entities[FIRST_NAVIGATION_ENTITY_INDEX]
                .flags
                .has_scaled_extent()
        );
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

        let mut random = BloodPrng {
            seed: 4_660,
            mix_low: 37,
            mix_high: 41,
            counter: 43,
        };
        let random_before = random;
        let mut short_points = vec![ShipPointRecord::default(); SHIP_POINT_CLOUD_COUNT - 1];
        assert_eq!(
            randomize_ship_point_cloud(&mut short_points, &mut random),
            Err(ShipProjectionError::InvalidPointCount {
                actual: SHIP_POINT_CLOUD_COUNT - 1,
            })
        );
        assert_eq!(random, random_before);

        let anchors = vec![ShipObjectAnchor::default(); SHIP_OBJECT_ANCHOR_COUNT];
        let mut entities = vec![BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT];
        let entity_before = entities.clone();
        assert_eq!(
            project_ship_object_sprites(
                &anchors[..SHIP_OBJECT_ANCHOR_COUNT - ENTITY_INDEX_STEP],
                ShipCameraPosition::default(),
                matrix,
                BridgeSpriteExtent::default(),
                &mut entities,
            ),
            Err(ShipProjectionError::InvalidObjectAnchorCount {
                actual: SHIP_OBJECT_ANCHOR_COUNT - ENTITY_INDEX_STEP,
            })
        );
        assert_eq!(entities, entity_before);

        let mut short_entities =
            vec![BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT - ENTITY_INDEX_STEP];
        let short_entity_before = short_entities.clone();
        assert_eq!(
            project_ship_object_sprites(
                &anchors,
                ShipCameraPosition::default(),
                matrix,
                BridgeSpriteExtent::default(),
                &mut short_entities,
            ),
            Err(ShipProjectionError::InvalidSpriteEntityCount {
                actual: BRIDGE_SPRITE_ENTITY_COUNT - ENTITY_INDEX_STEP,
                required: BRIDGE_SPRITE_ENTITY_COUNT,
            })
        );
        assert_eq!(short_entities, short_entity_before);
    }

    fn object_projection_fixture(case_index: usize) -> ObjectProjectionFixture {
        let seed = OBJECT_CASE_SEEDS[case_index];
        let camera = ShipCameraPosition {
            position: [
                (OBJECT_CAMERA_X_BASE + seed) as u16,
                OBJECT_CAMERA_Y,
                OBJECT_CAMERA_Z,
            ],
        };
        let mut anchors: Vec<ShipObjectAnchor> = (usize::MIN..SHIP_OBJECT_ANCHOR_COUNT)
            .map(|anchor_index| {
                let z = if case_index == NEGATIVE_DEPTH_CASE {
                    camera.position[Z_AXIS].wrapping_sub(
                        (OBJECT_NEGATIVE_DEPTH_BASE + anchor_index * OBJECT_NEGATIVE_DEPTH_STEP)
                            as u16,
                    )
                } else if case_index == MIXED_VISIBILITY_CASE
                    && matches!(
                        anchor_index,
                        FIRST_ZERO_DEPTH_ANCHOR | SECOND_ZERO_DEPTH_ANCHOR
                    )
                {
                    camera.position[Z_AXIS]
                } else {
                    camera.position[Z_AXIS].wrapping_add(
                        (OBJECT_POSITIVE_DEPTH_BASE + anchor_index * OBJECT_DEPTH_STEP) as u16,
                    )
                };
                ShipObjectAnchor {
                    position: [
                        (OBJECT_ANCHOR_X_BASE
                            + seed * OBJECT_ANCHOR_X_SEED_SCALE
                            + anchor_index * OBJECT_ANCHOR_X_STEP) as u16,
                        (OBJECT_ANCHOR_Y_BASE
                            + seed * OBJECT_ANCHOR_Y_SEED_SCALE
                            + anchor_index * OBJECT_ANCHOR_Y_STEP) as u16,
                        z,
                    ],
                }
            })
            .collect();
        anchors[SHIP_OBJECT_ANCHOR_COUNT - ENTITY_INDEX_STEP].position = [
            camera.position[X_AXIS],
            camera.position[Y_AXIS],
            camera.position[Z_AXIS].wrapping_add(FINAL_OBJECT_DEPTH),
        ];

        let mut matrix = ShipProjectionMatrix {
            rows: [
                [
                    ORACLE_POSITIVE_UNIT,
                    ORACLE_MATRIX_COMPARISON_VALUE,
                    ORACLE_ZERO,
                ],
                [ORACLE_ZERO, ORACLE_NEGATIVE_UNIT, ORACLE_ZERO],
                [ORACLE_ZERO, ORACLE_ZERO, ORACLE_POSITIVE_UNIT],
            ],
        };
        if case_index == OVERFLOW_MATRIX_CASE {
            matrix.rows[X_AXIS][X_AXIS] = OVERFLOW_MATRIX_X_X;
            matrix.rows[X_AXIS][Z_AXIS] = OVERFLOW_MATRIX_X_Z;
            matrix.rows[Y_AXIS][X_AXIS] = OVERFLOW_MATRIX_Y_X;
            matrix.rows[Y_AXIS][Y_AXIS] = OVERFLOW_MATRIX_Y_Y;
        }

        let comparison = BridgeSpriteExtent {
            width: (COMPARISON_WIDTH_BASE + case_index * COMPARISON_WIDTH_STEP) as u16,
            height: (COMPARISON_HEIGHT_BASE + case_index * COMPARISON_HEIGHT_STEP) as u16,
        };
        let mut entities = vec![BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT];
        for (entity_index, entity) in entities
            .iter_mut()
            .enumerate()
            .skip(FIRST_NAVIGATION_ENTITY_INDEX)
        {
            let anchor_index = BRIDGE_SPRITE_ENTITY_COUNT - ENTITY_INDEX_STEP - entity_index;
            let mixed_visibility =
                matches!(case_index, MIXED_VISIBILITY_CASE | OVERFLOW_MATRIX_CASE);
            let visible = !mixed_visibility
                || anchor_index % VISIBILITY_PATTERN_DIVISOR != HIDDEN_VISIBILITY_REMAINDER;
            let mut flags = ENTITY_FLAG_HIGH_BITS
                | ENTITY_STATE_ZERO_BIT
                | if visible {
                    ENTITY_VISIBLE_BIT
                } else {
                    u16::MIN
                };
            if case_index == EQUAL_SOURCE_EXTENT_CASE {
                flags |= ENTITY_EXTENT_CHANGED_BIT;
            }
            let source_extent = if case_index == EQUAL_SOURCE_EXTENT_CASE {
                comparison
            } else {
                BridgeSpriteExtent {
                    width: (ENTITY_SOURCE_WIDTH_BASE + entity_index * ENTITY_SOURCE_WIDTH_STEP)
                        as u16,
                    height: (ENTITY_SOURCE_HEIGHT_BASE + entity_index * ENTITY_SOURCE_HEIGHT_STEP)
                        as u16,
                }
            };
            *entity = BridgeSpriteEntity {
                flags: BridgeSpriteFlags::from_bits(flags),
                source_extent,
                draw_position: BridgeSpritePosition {
                    x: (ENTITY_DRAW_X_BASE + entity_index * ENTITY_DRAW_X_STEP) as u16,
                    y: (ENTITY_DRAW_Y_BASE + entity_index * ENTITY_DRAW_Y_STEP) as u16,
                },
                extent: BridgeSpriteExtent {
                    width: (ENTITY_EXTENT_WIDTH_BASE + entity_index) as u16,
                    height: (ENTITY_EXTENT_HEIGHT_BASE + entity_index) as u16,
                },
                ..BridgeSpriteEntity::default()
            };
        }

        ObjectProjectionFixture {
            seed,
            anchors,
            camera,
            matrix,
            comparison,
            entities,
        }
    }

    fn object_projection_hash(projections: &[ShipObjectSpriteProjection]) -> String {
        let mut bytes = Vec::with_capacity(projections.len() * OBJECT_EVENT_BYTE_COUNT * 2);
        for projection in projections {
            append_object_event(
                &mut bytes,
                EXTENT_EVENT_TAG,
                [
                    projection.entity_index as u16,
                    projection.scaled_extent.width,
                    projection.scaled_extent.height,
                    projection.screen[X_AXIS],
                    projection.screen[Y_AXIS],
                    projection.depth_scale,
                ],
            );
            append_object_event(
                &mut bytes,
                POSITION_EVENT_TAG,
                [
                    projection.entity_index as u16,
                    projection.draw_position.x,
                    projection.draw_position.y,
                    projection.screen[X_AXIS],
                    projection.screen[Y_AXIS],
                    projection.depth_scale,
                ],
            );
        }
        format!("{:x}", Sha256::digest(bytes))
    }

    fn append_object_event(bytes: &mut Vec<u8>, tag: u8, words: [u16; OBJECT_EVENT_WORD_COUNT]) {
        bytes.push(tag);
        for word in words {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
    }

    fn object_extent_event(projection: &ShipObjectSpriteProjection) -> serde_json::Value {
        serde_json::json!([
            "extent",
            projection.entity_index,
            projection.scaled_extent.width,
            projection.scaled_extent.height,
            projection.screen[X_AXIS],
            projection.screen[Y_AXIS],
            projection.depth_scale,
        ])
    }

    fn object_position_event(projection: &ShipObjectSpriteProjection) -> serde_json::Value {
        serde_json::json!([
            "position",
            projection.entity_index,
            projection.draw_position.x,
            projection.draw_position.y,
            projection.screen[X_AXIS],
            projection.screen[Y_AXIS],
            projection.depth_scale,
        ])
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

    fn random_outputs(case_index: usize) -> Vec<u16> {
        match case_index {
            0 => vec![u16::MIN; RANDOM_OUTPUT_COUNT],
            1 => (usize::MIN..RANDOM_OUTPUT_COUNT)
                .map(|index| ((RAMP_START + index * RAMP_STEP) % usize::from(u16::MAX)) as u16)
                .collect(),
            2 => (usize::MIN..RANDOM_OUTPUT_COUNT)
                .map(|index| EXTREMA_CYCLE[index % EXTREMA_CYCLE.len()])
                .collect(),
            3 => {
                let mut state = LCG_INITIAL_STATE;
                (usize::MIN..RANDOM_OUTPUT_COUNT)
                    .map(|_| {
                        state = state
                            .wrapping_mul(LCG_MULTIPLIER)
                            .wrapping_add(LCG_INCREMENT);
                        state % u16::MAX
                    })
                    .collect()
            }
            _ => panic!("unexpected randomizer fixture"),
        }
    }

    fn hash_words(words: &[u16]) -> String {
        let bytes: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
        format!("{:x}", Sha256::digest(bytes))
    }

    fn decode_point_record(bytes: &[u8]) -> ShipPointRecord {
        ShipPointRecord {
            position: [
                u16::from_le_bytes([bytes[0], bytes[1]]),
                u16::from_le_bytes([bytes[2], bytes[3]]),
                u16::from_le_bytes([bytes[4], bytes[5]]),
            ],
            scratch: u16::from_le_bytes([bytes[6], bytes[7]]),
        }
    }

    fn encode_point_cloud(points: &[ShipPointRecord]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(points.len() * POINT_RECORD_SIZE);
        for point in points {
            for component in point.position {
                bytes.extend_from_slice(&component.to_le_bytes());
            }
            bytes.extend_from_slice(&point.scratch.to_le_bytes());
        }
        bytes
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
