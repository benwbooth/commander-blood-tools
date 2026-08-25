//! Deterministic fixed-point starfield shared by all three alien scenes.

use std::fmt;

use commander_blood_formats::alien::AXIS_COUNT;

const X_AXIS: usize = 0;
const Y_AXIS: usize = 1;
const Z_AXIS: usize = 2;
/// Number of authored stars generated on every alien-scene frame.
pub const STAR_COUNT: usize = 1_200;
const RANDOM_ROTATION: u32 = 7;
const CAMERA_CELL_SHIFT: u32 = 13;
const DEPTH_SHIFT: u32 = 8;
const SHADE_SHIFT: u32 = 15;
const SCREEN_CENTER_X: i16 = 160;
const SCREEN_CENTER_Y: i16 = 100;
const SCREEN_WIDTH: i16 = 320;
const SCREEN_HEIGHT: i16 = 200;
const SHADE_TABLE_ENTRY_COUNT: usize = 256;
const ZERO_COMPONENT: i32 = 0;
const ZERO_SCREEN_COORDINATE: i16 = 0;

type Matrix = [[i32; AXIS_COUNT]; AXIS_COUNT];

/// One visible star ready for submission to the modern renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienStar {
    /// Original 320-by-200 screen coordinate.
    pub screen: [i16; 2],
    /// Fixed-point distance-derived shade-table index.
    pub shade: u16,
    /// Palette index selected by the current star shade table.
    pub palette_index: u8,
}

/// Counts for every recovered star rejection branch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienStarRejections {
    /// Camera-space depth accumulator was negative.
    pub negative_depth: usize,
    /// Nonnegative depth became zero after fixed-point scaling.
    pub zero_shifted_depth: usize,
    /// Projected coordinate was left of the viewport.
    pub left: usize,
    /// Projected coordinate was right of the viewport.
    pub right: usize,
    /// Projected coordinate was above the viewport.
    pub top: usize,
    /// Projected coordinate was below the viewport.
    pub bottom: usize,
}

/// Typed output of one complete 1,200-star generation pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienStarfieldFrame {
    /// Logical camera cells derived from the fixed-point camera position.
    pub camera_cells: [u16; AXIS_COUNT],
    /// Internal random value after all generated coordinates.
    pub random_after: u32,
    /// Visible stars in generation order.
    pub stars: Vec<AlienStar>,
    /// Branch-specific rejection totals.
    pub rejections: AlienStarRejections,
}

/// Arithmetic or table failure while generating a starfield.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienStarfieldError {
    /// Signed screen division overflowed for a generated star.
    ProjectionDivisionOverflow {
        /// Star whose projection failed.
        star_index: usize,
    },
    /// A generated shade fell outside the recovered 256-entry table.
    InvalidShade {
        /// Star with the invalid shade.
        star_index: usize,
        /// Out-of-range shade value.
        shade: u16,
    },
}

impl fmt::Display for AlienStarfieldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid alien starfield state: {self:?}")
    }
}

impl std::error::Error for AlienStarfieldError {}

/// Generate the exact native star stream as flat screen-space point data.
///
/// VGA plane lists and port writes are presentation details and are not
/// represented. Generation order is retained, including overwrite order for
/// stars that land on the same pixel.
pub fn generate_starfield(
    seed: u32,
    camera_position: [i32; AXIS_COUNT],
    camera_matrix: Matrix,
    shade_table: &[u8; SHADE_TABLE_ENTRY_COUNT],
) -> Result<AlienStarfieldFrame, AlienStarfieldError> {
    let camera_cells =
        camera_position.map(|position| (position as u32 >> CAMERA_CELL_SHIFT) as u16);
    let mut random = seed;
    let mut stars = Vec::with_capacity(STAR_COUNT);
    let mut rejections = AlienStarRejections::default();

    for star_index in usize::MIN..STAR_COUNT {
        let coordinates = std::array::from_fn(|axis| {
            random = random_step(random);
            i32::from(camera_cells[axis].wrapping_sub(random as u16) as i16)
        });
        let depth_accumulator = wrapping_dot(camera_matrix[Z_AXIS], coordinates);
        if depth_accumulator < ZERO_COMPONENT {
            rejections.negative_depth += 1;
            continue;
        }
        let depth = depth_accumulator >> DEPTH_SHIFT;
        if depth == ZERO_COMPONENT {
            rejections.zero_shifted_depth += 1;
            continue;
        }

        let screen_x = wrapping_dot(camera_matrix[X_AXIS], coordinates)
            .checked_div(depth)
            .ok_or(AlienStarfieldError::ProjectionDivisionOverflow { star_index })?
            as i16;
        let screen_x = screen_x.wrapping_add(SCREEN_CENTER_X);
        if screen_x < ZERO_SCREEN_COORDINATE {
            rejections.left += 1;
            continue;
        }
        if screen_x >= SCREEN_WIDTH {
            rejections.right += 1;
            continue;
        }

        let screen_y = wrapping_dot(camera_matrix[Y_AXIS], coordinates)
            .checked_div(depth)
            .ok_or(AlienStarfieldError::ProjectionDivisionOverflow { star_index })?
            as i16;
        let screen_y = screen_y.wrapping_neg().wrapping_add(SCREEN_CENTER_Y);
        if screen_y < ZERO_SCREEN_COORDINATE {
            rejections.top += 1;
            continue;
        }
        if screen_y >= SCREEN_HEIGHT {
            rejections.bottom += 1;
            continue;
        }

        let shade = (depth as u32 >> SHADE_SHIFT) as u16;
        let palette_index = *shade_table
            .get(usize::from(shade))
            .ok_or(AlienStarfieldError::InvalidShade { star_index, shade })?;
        stars.push(AlienStar {
            screen: [screen_x, screen_y],
            shade,
            palette_index,
        });
    }

    Ok(AlienStarfieldFrame {
        camera_cells,
        random_after: random,
        stars,
        rejections,
    })
}

fn random_step(value: u32) -> u32 {
    let rotated = value.rotate_right(RANDOM_ROTATION);
    rotated.wrapping_sub(rotated >> (u32::BITS - 1))
}

fn wrapping_dot(left: [i32; AXIS_COUNT], right: [i32; AXIS_COUNT]) -> i32 {
    left.into_iter()
        .zip(right)
        .fold(u32::MIN, |accumulator, (left, right)| {
            accumulator.wrapping_add((left as u32).wrapping_mul(right as u32))
        }) as i32
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct StarVector {
        screen: [i16; 2],
        shade: u16,
        palette_index: u8,
    }

    #[derive(Deserialize)]
    struct RejectionVector {
        negative_depth: usize,
        zero_shifted_depth: usize,
        left: usize,
        right: usize,
        top: usize,
        bottom: usize,
    }

    #[derive(Deserialize)]
    struct StarfieldVector {
        name: String,
        seed: u32,
        camera_matrix: [i32; AXIS_COUNT * AXIS_COUNT],
        camera_position: [i32; AXIS_COUNT],
        shade_table: Vec<u8>,
        camera_cells: [u16; AXIS_COUNT],
        random_after: u32,
        stars: Vec<StarVector>,
        rejections: RejectionVector,
    }

    fn matrix(flat: [i32; AXIS_COUNT * AXIS_COUNT]) -> Matrix {
        std::array::from_fn(|row| std::array::from_fn(|column| flat[row * AXIS_COUNT + column]))
    }

    fn run_vector(vector: StarfieldVector) {
        let shade_table: [u8; SHADE_TABLE_ENTRY_COUNT] = vector.shade_table.try_into().unwrap();
        let frame = generate_starfield(
            vector.seed,
            vector.camera_position,
            matrix(vector.camera_matrix),
            &shade_table,
        )
        .unwrap();
        assert_eq!(frame.camera_cells, vector.camera_cells, "{}", vector.name);
        assert_eq!(frame.random_after, vector.random_after, "{}", vector.name);
        assert_eq!(frame.stars.len(), vector.stars.len(), "{}", vector.name);
        for (actual, expected) in frame.stars.iter().zip(&vector.stars) {
            assert_eq!(actual.screen, expected.screen, "{}", vector.name);
            assert_eq!(actual.shade, expected.shade, "{}", vector.name);
            assert_eq!(
                actual.palette_index, expected.palette_index,
                "{}",
                vector.name
            );
        }
        assert_eq!(
            frame.rejections,
            AlienStarRejections {
                negative_depth: vector.rejections.negative_depth,
                zero_shifted_depth: vector.rejections.zero_shifted_depth,
                left: vector.rejections.left,
                right: vector.rejections.right,
                top: vector.rejections.top,
                bottom: vector.rejections.bottom,
            },
            "{}",
            vector.name
        );
    }

    #[test]
    fn starfield_matches_every_original_alien_overlay_vector() {
        let suites = [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_0734_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_0775_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_0775_natural.json"),
        ];
        for json in suites {
            let vectors: Vec<StarfieldVector> = serde_json::from_str(json).unwrap();
            for vector in vectors {
                run_vector(vector);
            }
        }
    }
}
