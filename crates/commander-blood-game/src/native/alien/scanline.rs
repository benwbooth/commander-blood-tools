//! Flat-memory form of the recovered alien fixed-point scanline state.
//!
//! The DOS overlays stored these values in linked 90-byte records. The modern
//! port owns them as typed values while preserving the original wrapping
//! arithmetic, edge phases, texture addressing, and half-open scanline spans.

use commander_blood_formats::alien::{RASTER_RECIPROCAL_COUNT, TEXTURE_HEIGHT, TEXTURE_WIDTH};

use super::AlienRenderVertex;

const FIXED_FRACTION_BITS: u32 = 16;
const TEXTURE_FRACTION_BITS: u32 = 8;
const TEXTURE_BANK_SHIFT: u32 = 8;
const TEXTURE_BANK_MASK: u16 = 15;
const TEXTURE_BANK_SIZE: usize = 1 << 16;
#[cfg(test)]
const FOUR_PLANE_COLUMN_PERIOD: usize = 4;
const TRIANGLE_VERTEX_COUNT: usize = 3;
pub(crate) const ALIEN_RASTER_WIDTH: usize = 320;
pub(crate) const ALIEN_RASTER_HEIGHT: usize = 200;
const ALIEN_RASTER_RECORD_CAPACITY: usize = 600;

/// Column phase selected when a triangle crosses its middle X coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[cfg_attr(test, serde(rename_all = "snake_case"))]
pub(crate) enum AlienRasterAdvance {
    /// Replace the left edge and its texture/depth origin.
    SecondaryLeft,
    /// Replace the right edge while continuing the left edge.
    SecondaryRight,
    /// Retire the record after its current column run.
    Remove,
}

/// One active triangle in the recovered fixed-point column rasterizer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[cfg_attr(test, serde(default))]
pub(crate) struct AlienRasterRecord {
    pub(crate) edge_0_position: i32,
    pub(crate) edge_0_step: i32,
    pub(crate) edge_1_position: i32,
    pub(crate) edge_1_step: i32,
    pub(crate) depth_position: i32,
    pub(crate) depth_step: i32,
    pub(crate) depth_gradient: i32,
    pub(crate) advance: AlienRasterAdvance,
    pub(crate) remaining: i16,
    pub(crate) secondary_remaining: i16,
    pub(crate) secondary_edge_position: i32,
    pub(crate) secondary_edge_step: i32,
    pub(crate) secondary_depth_position: i32,
    pub(crate) secondary_depth_step: i32,
    pub(crate) texture_u: i16,
    pub(crate) texture_v: i16,
    pub(crate) secondary_texture_u: i16,
    pub(crate) secondary_texture_v: i16,
    pub(crate) texture_u_step: i16,
    pub(crate) texture_v_step: i16,
    pub(crate) secondary_texture_u_step: i16,
    pub(crate) secondary_texture_v_step: i16,
    pub(crate) texture_du: i16,
    pub(crate) texture_dv: i16,
    /// Zero-based 64 KiB bank in the decoded 256-by-512 atlas.
    pub(crate) texture_bank: u8,
}

/// One face activation submitted to a fresh native raster pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AlienRasterActivation {
    pub(crate) first_column: usize,
    pub(crate) record: AlienRasterRecord,
}

impl Default for AlienRasterRecord {
    fn default() -> Self {
        Self {
            edge_0_position: 0,
            edge_0_step: 0,
            edge_1_position: 0,
            edge_1_step: 0,
            depth_position: 0,
            depth_step: 0,
            depth_gradient: 0,
            advance: AlienRasterAdvance::Remove,
            remaining: 0,
            secondary_remaining: 0,
            secondary_edge_position: 0,
            secondary_edge_step: 0,
            secondary_depth_position: 0,
            secondary_depth_step: 0,
            texture_u: 0,
            texture_v: 0,
            secondary_texture_u: 0,
            secondary_texture_v: 0,
            texture_u_step: 0,
            texture_v_step: 0,
            secondary_texture_u_step: 0,
            secondary_texture_v_step: 0,
            texture_du: 0,
            texture_dv: 0,
            texture_bank: 0,
        }
    }
}

/// Modern semantic output modes retained by the native renderer oracle.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AlienRasterOutputMode {
    /// Draw every column into a flat surface.
    Linear,
    /// Draw every column; the DOS plane select is only a storage detail.
    ModeX,
    /// Draw every fourth column, matching the native all-plane pass.
    FourPlanes,
}

/// One indexed texel emitted by fixed-point scan conversion.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub(crate) struct AlienRasterPixel {
    pub(crate) x: usize,
    pub(crate) y: usize,
    pub(crate) value: u8,
}

/// Convert one cyclically X-ordered triangle to the native column record.
pub(crate) fn build_raster_record(
    vertices: [AlienRenderVertex; TRIANGLE_VERTEX_COUNT],
    reciprocals: &[i32; RASTER_RECIPROCAL_COUNT],
    raster_capacity_available: bool,
) -> Option<AlienRasterRecord> {
    if !raster_capacity_available {
        return None;
    }
    let [vertex_0, vertex_1, vertex_2] = vertices;
    let x_0 = vertex_0.screen[0] as u16;
    let x_1 = vertex_1.screen[0] as u16;
    let x_2 = vertex_2.screen[0] as u16;
    let width_1 = x_1.wrapping_sub(x_0);
    let width_2 = x_2.wrapping_sub(x_0);
    let mut record = AlienRasterRecord {
        texture_bank: ((vertex_0.texture[1] >> TEXTURE_BANK_SHIFT) & TEXTURE_BANK_MASK) as u8,
        ..Default::default()
    };
    let mut clipping_mode = ClippingMode::None;

    if width_1 == 0 {
        let vertical_span = (vertex_1.screen[1] as u16).wrapping_sub(vertex_0.screen[1] as u16);
        if width_2 == 0
            || usize::from(width_2) >= RASTER_RECIPROCAL_COUNT
            || (vertical_span as i16) <= 0
            || usize::from(vertical_span) >= RASTER_RECIPROCAL_COUNT
        {
            return None;
        }
        let reciprocal_1 = reciprocals[usize::from(vertical_span)];
        let reciprocal_2 = reciprocals[usize::from(width_2)];
        record.remaining = width_2.wrapping_sub(1) as i16;

        record.edge_0_step = multiply_low(
            vertex_2.screen[1].wrapping_sub(vertex_0.screen[1]) as i32,
            reciprocal_2,
        );
        record.edge_0_position = fixed_y(vertex_0.screen[1]).wrapping_add(record.edge_0_step >> 1);
        record.edge_1_step = multiply_low(
            vertex_2.screen[1].wrapping_sub(vertex_1.screen[1]) as i32,
            reciprocal_2,
        );
        record.edge_1_position = fixed_y(vertex_1.screen[1]).wrapping_add(record.edge_1_step >> 1);

        let delta_1 = multiply_low(
            word_difference(vertex_1.texture[0], vertex_0.texture[0]) as i32,
            reciprocal_1,
        );
        let delta_2 = multiply_low(
            word_difference(vertex_2.texture[0], vertex_0.texture[0]) as i32,
            reciprocal_2,
        );
        record.texture_du = (delta_1 >> TEXTURE_FRACTION_BITS) as i16;
        record.texture_u_step = (delta_2 >> TEXTURE_FRACTION_BITS) as i16;
        record.texture_u = texture_origin(vertex_0.texture[0], delta_2 >> 9);

        let delta_1 = multiply_low(
            i32::from(vertex_1.texture[1]) - i32::from(vertex_0.texture[1]),
            reciprocal_1,
        );
        let delta_2 = multiply_low(
            i32::from(vertex_2.texture[1]) - i32::from(vertex_0.texture[1]),
            reciprocal_2,
        );
        record.texture_dv = (delta_1 >> TEXTURE_FRACTION_BITS) as i16;
        record.texture_v_step = (delta_2 >> TEXTURE_FRACTION_BITS) as i16;
        record.texture_v = texture_origin(vertex_0.texture[1], delta_2 >> 9);

        record.depth_step = multiply_q16(vertex_2.depth.wrapping_sub(vertex_0.depth), reciprocal_2);
        record.depth_position = vertex_0.depth.wrapping_add(record.depth_step >> 1);
        record.depth_gradient =
            multiply_q16(vertex_1.depth.wrapping_sub(vertex_0.depth), reciprocal_1);
    } else {
        if width_2 == 0
            || usize::from(width_1) >= RASTER_RECIPROCAL_COUNT
            || usize::from(width_2) >= RASTER_RECIPROCAL_COUNT
        {
            return None;
        }
        let reciprocal_1 = reciprocals[usize::from(width_1)];
        let reciprocal_2 = reciprocals[usize::from(width_2)];
        record.remaining = width_2.wrapping_sub(1) as i16;
        record.edge_1_step = multiply_low(
            vertex_1.screen[1].wrapping_sub(vertex_0.screen[1]) as i32,
            reciprocal_1,
        );
        record.edge_0_step = multiply_low(
            vertex_2.screen[1].wrapping_sub(vertex_0.screen[1]) as i32,
            reciprocal_2,
        );
        let area = record.edge_0_step.wrapping_sub(record.edge_1_step);
        if area >= 0 {
            return None;
        }
        let denominator = -(area >> TEXTURE_FRACTION_BITS);
        record.edge_0_position = fixed_y(vertex_0.screen[1]).wrapping_add(record.edge_0_step >> 1);
        record.edge_1_position = fixed_y(vertex_0.screen[1]).wrapping_add(record.edge_1_step >> 1);

        let delta_1 = multiply_low(
            word_difference(vertex_1.texture[0], vertex_0.texture[0]) as i32,
            reciprocal_1,
        );
        let delta_2 = multiply_low(
            word_difference(vertex_2.texture[0], vertex_0.texture[0]) as i32,
            reciprocal_2,
        );
        record.texture_du = delta_1.wrapping_sub(delta_2).wrapping_div(denominator) as i16;
        record.texture_u_step = (delta_2 >> TEXTURE_FRACTION_BITS) as i16;
        record.texture_u =
            texture_origin(vertex_0.texture[0], i32::from(record.texture_u_step) >> 1);

        let delta_1 = multiply_low(
            i32::from(vertex_1.texture[1]) - i32::from(vertex_0.texture[1]),
            reciprocal_1,
        );
        let delta_2 = multiply_low(
            i32::from(vertex_2.texture[1]) - i32::from(vertex_0.texture[1]),
            reciprocal_2,
        );
        record.texture_dv = delta_1.wrapping_sub(delta_2).wrapping_div(denominator) as i16;
        record.texture_v_step = (delta_2 >> TEXTURE_FRACTION_BITS) as i16;
        record.texture_v =
            texture_origin(vertex_0.texture[1], i32::from(record.texture_v_step) >> 1);

        record.depth_step = multiply_q16(vertex_2.depth.wrapping_sub(vertex_0.depth), reciprocal_2);
        record.depth_position = vertex_0.depth.wrapping_add(record.depth_step >> 1);
        let delta_1 = multiply_low(vertex_1.depth.wrapping_sub(vertex_0.depth), reciprocal_1);
        let delta_2 = multiply_low(vertex_2.depth.wrapping_sub(vertex_0.depth), reciprocal_2);
        record.depth_gradient =
            delta_1.wrapping_sub(delta_2).wrapping_div(denominator) >> TEXTURE_FRACTION_BITS;

        let x_difference = x_1.wrapping_sub(x_2) as i16;
        if x_difference > 0 {
            let secondary_width = x_1.wrapping_sub(x_2);
            let reciprocal = *reciprocals.get(usize::from(secondary_width))?;
            if (x_2 as i16) < 0 {
                let clipped_columns = 0u16.wrapping_sub(x_2);
                record.remaining = x_1.wrapping_sub(1) as i16;
                record.texture_u_step = (multiply_low(
                    word_difference(vertex_1.texture[0], vertex_2.texture[0]) as i32,
                    reciprocal,
                ) >> TEXTURE_FRACTION_BITS) as i16;
                record.texture_v_step = (multiply_low(
                    word_difference(vertex_1.texture[1], vertex_2.texture[1]) as i32,
                    reciprocal,
                ) >> TEXTURE_FRACTION_BITS) as i16;
                record.texture_u = texture_origin(
                    vertex_2.texture[0],
                    i32::from((record.texture_u_step as u16).wrapping_mul(clipped_columns)),
                );
                record.texture_v = texture_origin(
                    vertex_2.texture[1],
                    i32::from((record.texture_v_step as u16).wrapping_mul(clipped_columns)),
                );
                record.edge_0_step = multiply_low(
                    vertex_1.screen[1].wrapping_sub(vertex_2.screen[1]) as i32,
                    reciprocal,
                );
                record.edge_0_position = fixed_y(vertex_2.screen[1])
                    .wrapping_add(multiply_low(record.edge_0_step, i32::from(clipped_columns)));
                record.depth_step =
                    multiply_q16(vertex_1.depth.wrapping_sub(vertex_2.depth), reciprocal);
                record.depth_position = vertex_2
                    .depth
                    .wrapping_add(multiply_low(record.depth_step, i32::from(clipped_columns)));
                let left_clip = 0u16.wrapping_sub(x_0);
                record.edge_1_position = record
                    .edge_1_position
                    .wrapping_add(multiply_low(record.edge_1_step, i32::from(left_clip)));
                clipping_mode = ClippingMode::SecondaryLeft;
            } else {
                record.secondary_remaining = secondary_width.wrapping_sub(1) as i16;
                record.secondary_texture_u_step = (multiply_low(
                    word_difference(vertex_1.texture[0], vertex_2.texture[0]) as i32,
                    reciprocal,
                ) >> TEXTURE_FRACTION_BITS)
                    as i16;
                record.secondary_texture_v_step = (multiply_low(
                    word_difference(vertex_1.texture[1], vertex_2.texture[1]) as i32,
                    reciprocal,
                ) >> TEXTURE_FRACTION_BITS)
                    as i16;
                record.secondary_texture_u = texture_origin(
                    vertex_2.texture[0],
                    i32::from(record.secondary_texture_u_step) >> 1,
                );
                record.secondary_texture_v = texture_origin(
                    vertex_2.texture[1],
                    i32::from(record.secondary_texture_v_step) >> 1,
                );
                record.secondary_edge_step = multiply_low(
                    vertex_1.screen[1].wrapping_sub(vertex_2.screen[1]) as i32,
                    reciprocal,
                );
                record.secondary_edge_position =
                    fixed_y(vertex_2.screen[1]).wrapping_add(record.secondary_edge_step >> 1);
                record.secondary_depth_step =
                    multiply_q16(vertex_1.depth.wrapping_sub(vertex_2.depth), reciprocal);
                record.secondary_depth_position = vertex_2
                    .depth
                    .wrapping_add(record.secondary_depth_step >> 1);
                record.advance = AlienRasterAdvance::SecondaryLeft;
            }
        } else if x_difference < 0 {
            let secondary_width = x_2.wrapping_sub(x_1);
            let reciprocal = *reciprocals.get(usize::from(secondary_width))?;
            if (x_1 as i16) < 0 {
                let clipped_columns = 0u16.wrapping_sub(x_1);
                record.edge_1_step = multiply_low(
                    vertex_2.screen[1].wrapping_sub(vertex_1.screen[1]) as i32,
                    reciprocal,
                );
                record.edge_1_position = fixed_y(vertex_1.screen[1])
                    .wrapping_add(multiply_low(record.edge_1_step, i32::from(clipped_columns)));
                clipping_mode = ClippingMode::SecondaryRight;
            } else {
                record.remaining = (record.remaining as u16).wrapping_sub(secondary_width) as i16;
                record.secondary_remaining = secondary_width.wrapping_sub(1) as i16;
                record.secondary_edge_step = multiply_low(
                    vertex_2.screen[1].wrapping_sub(vertex_1.screen[1]) as i32,
                    reciprocal,
                );
                record.secondary_edge_position =
                    fixed_y(vertex_1.screen[1]).wrapping_add(record.secondary_edge_step >> 1);
                record.advance = AlienRasterAdvance::SecondaryRight;
            }
        }
    }

    if (x_0 as i16) < 0 && clipping_mode != ClippingMode::SecondaryLeft {
        let clipped_columns = 0u16.wrapping_sub(x_0);
        record.remaining = (record.remaining as u16).wrapping_sub(clipped_columns) as i16;
        record.edge_0_position = record
            .edge_0_position
            .wrapping_add(multiply_low(record.edge_0_step, i32::from(clipped_columns)));
        if clipping_mode != ClippingMode::SecondaryRight {
            record.edge_1_position = record
                .edge_1_position
                .wrapping_add(multiply_low(record.edge_1_step, i32::from(clipped_columns)));
        }
        record.depth_position = record
            .depth_position
            .wrapping_add(multiply_low(record.depth_step, i32::from(clipped_columns)));
        record.texture_u = word_add(
            record.texture_u,
            (record.texture_u_step as u16).wrapping_mul(clipped_columns),
        );
        record.texture_v = word_add(
            record.texture_v,
            (record.texture_v_step as u16).wrapping_mul(clipped_columns),
        );
    }
    Some(record)
}

/// Scan one isolated native raster record into flat indexed pixels.
///
/// This is also the directly executable contract for record clipping and phase
/// transitions. Multi-record visible-span ordering is layered above it.
#[cfg(test)]
pub(crate) fn rasterize_single_record(
    mut record: AlienRasterRecord,
    first_column: usize,
    mode: AlienRasterOutputMode,
    texture: &[u8],
) -> Option<Vec<AlienRasterPixel>> {
    let mut pixels = Vec::new();
    let mut column = first_column;
    loop {
        let draw_column =
            mode != AlienRasterOutputMode::FourPlanes || column % FOUR_PLANE_COLUMN_PERIOD == 0;
        if draw_column {
            let native_start = fixed_integer(record.edge_0_position);
            let start = native_start.max(0);
            let end = fixed_integer(record.edge_1_position);
            let relative = start.wrapping_sub(native_start);
            let mut texture_u = word_add(
                record.texture_u,
                (record.texture_du as u16).wrapping_mul(relative as u16),
            );
            let mut texture_v = word_add(
                record.texture_v,
                (record.texture_dv as u16).wrapping_mul(relative as u16),
            );
            for y in start..end {
                let bank_offset = usize::from(record.texture_bank) * TEXTURE_BANK_SIZE;
                let texture_offset = usize::from(
                    ((texture_u as u16) >> TEXTURE_FRACTION_BITS) | ((texture_v as u16) & 0xFF00),
                );
                let value = *texture.get(bank_offset + texture_offset)?;
                let output_columns = if mode == AlienRasterOutputMode::FourPlanes {
                    column..column + FOUR_PLANE_COLUMN_PERIOD
                } else {
                    column..column + 1
                };
                let y = usize::try_from(y).ok()?;
                pixels.extend(output_columns.map(|x| AlienRasterPixel { x, y, value }));
                texture_u = word_add(texture_u, record.texture_du as u16);
                texture_v = word_add(texture_v, record.texture_dv as u16);
            }
        }
        column = column.wrapping_add(1);
        if !record.advance_column() {
            break;
        }
    }
    Some(pixels)
}

/// Run the recovered active-span renderer and emit source texel offsets.
///
/// Activations must retain the original bucket and face order. The output
/// callback receives logical 320-by-200 coordinates and an offset into the
/// decoded two-bank texture atlas.
pub(crate) fn rasterize_activations(
    activations: &[AlienRasterActivation],
    texture_length: usize,
    mut emit: impl FnMut(usize, usize, usize),
) -> bool {
    let mut active = Vec::new();
    let mut next_activation = 0;
    for column in 0..ALIEN_RASTER_WIDTH {
        while let Some(activation) = activations.get(next_activation) {
            if activation.first_column != column {
                break;
            }
            if active.len() < ALIEN_RASTER_RECORD_CAPACITY {
                insert_active_record(&mut active, activation.record);
            }
            next_activation += 1;
        }
        if !rasterize_active_column(&active, column, texture_length, &mut emit) {
            return false;
        }
        active.retain_mut(AlienRasterRecord::advance_column);
        active.sort_by_key(|record| record.edge_0_position);
    }
    true
}

fn insert_active_record(active: &mut Vec<AlienRasterRecord>, record: AlienRasterRecord) {
    let Some(first) = active.first() else {
        active.push(record);
        return;
    };
    let mut insertion = 0;
    if record.edge_0_position > first.edge_0_position
        || (record.edge_0_position == first.edge_0_position
            && record.edge_0_step > first.edge_0_step)
    {
        insertion = 1;
        while let Some(next) = active.get(insertion) {
            if record.edge_0_position > next.edge_0_position
                || (record.edge_0_position == next.edge_0_position
                    && record.edge_0_step > next.edge_0_position)
            {
                insertion += 1;
            } else {
                break;
            }
        }
    }
    active.insert(insertion, record);
}

fn rasterize_active_column(
    active: &[AlienRasterRecord],
    column: usize,
    texture_length: usize,
    emit: &mut impl FnMut(usize, usize, usize),
) -> bool {
    let mut depth_order = Vec::new();
    let mut next_start = 0;
    while let Some(record) = active.get(next_start) {
        let top = fixed_integer(record.edge_0_position);
        if top >= 0 {
            break;
        }
        let bottom = fixed_integer(record.edge_1_position);
        if bottom > 0 {
            let projected_depth = record.depth_position.wrapping_add(
                i32::from(top)
                    .wrapping_neg()
                    .wrapping_mul(record.depth_gradient),
            );
            let insertion = depth_order
                .iter()
                .position(|&existing_index| {
                    let existing: &AlienRasterRecord = &active[existing_index];
                    let existing_top = fixed_integer(existing.edge_0_position);
                    let existing_depth = existing.depth_position.wrapping_add(
                        i32::from(existing_top)
                            .wrapping_neg()
                            .wrapping_mul(existing.depth_gradient),
                    );
                    projected_depth <= existing_depth
                })
                .unwrap_or(depth_order.len());
            depth_order.insert(insertion, next_start);
        }
        next_start += 1;
    }

    for y in 0..ALIEN_RASTER_HEIGHT {
        let coordinate = y as i16;
        depth_order.retain(|&record_index| {
            fixed_integer(active[record_index].edge_1_position) > coordinate
        });
        while let Some(record) = active.get(next_start) {
            if fixed_integer(record.edge_0_position) != coordinate {
                break;
            }
            let record_index = next_start;
            next_start += 1;
            if fixed_integer(record.edge_1_position) <= coordinate {
                continue;
            }
            let record = &active[record_index];
            let insertion = depth_order
                .iter()
                .position(|&existing_index| {
                    let existing = active[existing_index];
                    if record.edge_0_position >= existing.edge_1_position {
                        return false;
                    }
                    let distance = record
                        .edge_0_position
                        .wrapping_sub(existing.edge_0_position);
                    let projected_depth = existing
                        .depth_position
                        .wrapping_add(multiply_q16(distance, existing.depth_gradient));
                    projected_depth >= record.depth_position
                })
                .unwrap_or(depth_order.len());
            depth_order.insert(insertion, record_index);
        }

        let Some(&visible_index) = depth_order.first() else {
            continue;
        };
        let record = active[visible_index];
        let relative = coordinate.wrapping_sub(fixed_integer(record.edge_0_position));
        let texture_u = word_add(
            record.texture_u,
            (record.texture_du as u16).wrapping_mul(relative as u16),
        );
        let texture_v = word_add(
            record.texture_v,
            (record.texture_dv as u16).wrapping_mul(relative as u16),
        );
        let texture_offset = usize::from(record.texture_bank) * TEXTURE_BANK_SIZE
            + usize::from(
                ((texture_u as u16) >> TEXTURE_FRACTION_BITS) | ((texture_v as u16) & 0xFF00),
            );
        if texture_offset >= texture_length {
            return false;
        }
        emit(column, y, texture_offset);
    }
    true
}

impl AlienRasterRecord {
    fn advance_column(&mut self) -> bool {
        self.remaining = (self.remaining as u16).wrapping_sub(1) as i16;
        if self.remaining >= 0 {
            self.texture_u = word_add(self.texture_u, self.texture_u_step as u16);
            self.texture_v = word_add(self.texture_v, self.texture_v_step as u16);
            self.edge_0_position = self.edge_0_position.wrapping_add(self.edge_0_step);
            self.edge_1_position = self.edge_1_position.wrapping_add(self.edge_1_step);
            self.depth_position = self.depth_position.wrapping_add(self.depth_step);
            return true;
        }
        match self.advance {
            AlienRasterAdvance::SecondaryLeft => {
                self.edge_0_position = self.secondary_edge_position;
                self.edge_0_step = self.secondary_edge_step;
                self.depth_position = self.secondary_depth_position;
                self.depth_step = self.secondary_depth_step;
                self.texture_u = self.secondary_texture_u;
                self.texture_v = self.secondary_texture_v;
                self.texture_u_step = self.secondary_texture_u_step;
                self.texture_v_step = self.secondary_texture_v_step;
                self.remaining = self.secondary_remaining;
                self.advance = AlienRasterAdvance::Remove;
                self.edge_1_position = self.edge_1_position.wrapping_add(self.edge_1_step);
                true
            }
            AlienRasterAdvance::SecondaryRight => {
                self.edge_0_position = self.edge_0_position.wrapping_add(self.edge_0_step);
                self.depth_position = self.depth_position.wrapping_add(self.depth_step);
                self.texture_u = word_add(self.texture_u, self.texture_u_step as u16);
                self.texture_v = word_add(self.texture_v, self.texture_v_step as u16);
                self.edge_1_position = self.secondary_edge_position;
                self.edge_1_step = self.secondary_edge_step;
                self.remaining = self.secondary_remaining;
                self.advance = AlienRasterAdvance::Remove;
                true
            }
            AlienRasterAdvance::Remove => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClippingMode {
    None,
    SecondaryLeft,
    SecondaryRight,
}

const fn fixed_y(value: i16) -> i32 {
    (value as i32) << FIXED_FRACTION_BITS
}

const fn fixed_integer(value: i32) -> i16 {
    (value >> FIXED_FRACTION_BITS) as i16
}

const fn word_difference(left: u16, right: u16) -> i16 {
    left.wrapping_sub(right) as i16
}

const fn word_add(left: i16, right: u16) -> i16 {
    (left as u16).wrapping_add(right) as i16
}

const fn texture_origin(coordinate: u16, delta: i32) -> i16 {
    coordinate
        .wrapping_shl(TEXTURE_FRACTION_BITS)
        .wrapping_add(delta as u16) as i16
}

const fn multiply_low(left: i32, right: i32) -> i32 {
    (left as u32).wrapping_mul(right as u32) as i32
}

const fn multiply_q16(left: i32, right: i32) -> i32 {
    ((left as i64).wrapping_mul(right as i64) >> FIXED_FRACTION_BITS) as i32
}

const _: () = assert!(TEXTURE_WIDTH * TEXTURE_HEIGHT == TEXTURE_BANK_SIZE * 2);

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use sha2::{Digest, Sha256};

    use super::*;

    #[derive(Deserialize)]
    struct ReciprocalInput {
        width: usize,
        value: i32,
    }

    #[derive(Deserialize)]
    struct ActivationVector {
        name: String,
        screen: [[i16; 2]; TRIANGLE_VERTEX_COUNT],
        texture: [[u16; 2]; TRIANGLE_VERTEX_COUNT],
        depth: [i32; TRIANGLE_VERTEX_COUNT],
        reciprocals: Vec<ReciprocalInput>,
        accepted: bool,
        record: Option<ExpectedRecord>,
    }

    #[derive(Deserialize)]
    struct ExpectedRecord {
        edge_0_position: Option<i32>,
        edge_0_step: Option<i32>,
        edge_1_position: Option<i32>,
        edge_1_step: Option<i32>,
        depth_position: Option<i32>,
        depth_step: Option<i32>,
        depth_gradient: Option<i32>,
        advance: Option<AlienRasterAdvance>,
        remaining: Option<i16>,
        secondary_remaining: Option<i16>,
        secondary_edge_position: Option<i32>,
        secondary_edge_step: Option<i32>,
        secondary_depth_position: Option<i32>,
        secondary_depth_step: Option<i32>,
        texture_u: Option<i16>,
        texture_v: Option<i16>,
        secondary_texture_u: Option<i16>,
        secondary_texture_v: Option<i16>,
        texture_u_step: Option<i16>,
        texture_v_step: Option<i16>,
        secondary_texture_u_step: Option<i16>,
        secondary_texture_v_step: Option<i16>,
        texture_du: Option<i16>,
        texture_dv: Option<i16>,
        texture_bank: Option<u8>,
    }

    #[derive(Deserialize)]
    struct RendererVector {
        name: String,
        bucket_column: Option<usize>,
        output_mode: Option<AlienRasterOutputMode>,
        initial_record: Option<AlienRasterRecord>,
        initial_records: Option<Vec<AlienRasterRecord>>,
        texture: Option<OracleTexture>,
        framebuffer: Option<FramebufferContract>,
        logical_pixels: Option<Vec<AlienRasterPixel>>,
    }

    #[derive(Deserialize)]
    struct OracleTexture {
        unit: Vec<u8>,
        repetitions: usize,
        sha256: String,
    }

    #[derive(Deserialize)]
    struct FramebufferContract {
        width: usize,
        height: usize,
        initial_value: u8,
        sha256: String,
    }

    #[test]
    fn record_builder_matches_every_direct_alien_overlay_vector() {
        let fixtures = [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_2b6d_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_2bdd_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_2c9d_natural.json"),
        ];
        for fixture in fixtures {
            let vectors: Vec<ActivationVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                let mut reciprocals = [0; RASTER_RECIPROCAL_COUNT];
                for input in vector.reciprocals {
                    reciprocals[input.width] = input.value;
                }
                let vertices = std::array::from_fn(|index| AlienRenderVertex {
                    screen: vector.screen[index],
                    texture: vector.texture[index],
                    depth: vector.depth[index],
                });
                let actual = build_raster_record(vertices, &reciprocals, vector.name != "inactive");
                assert_eq!(actual.is_some(), vector.accepted, "{}", vector.name);
                if let (Some(actual), Some(expected)) = (actual, vector.record) {
                    expected.assert_matches(actual, &vector.name);
                }
            }
        }
    }

    #[test]
    fn isolated_scan_conversion_matches_every_direct_alien_overlay_vector() {
        let fixtures = [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_2572_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_25d6_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_2696_natural.json"),
        ];
        for fixture in fixtures {
            let vectors: Vec<RendererVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                let (
                    Some(column),
                    Some(mode),
                    Some(record),
                    Some(texture_contract),
                    Some(framebuffer_contract),
                    Some(expected),
                ) = (
                    vector.bucket_column,
                    vector.output_mode,
                    vector.initial_record,
                    vector.texture,
                    vector.framebuffer,
                    vector.logical_pixels,
                )
                else {
                    continue;
                };
                let texture = texture_contract.unit.repeat(texture_contract.repetitions);
                assert_eq!(
                    hex_sha256(&texture),
                    texture_contract.sha256,
                    "{}",
                    vector.name
                );
                let actual = rasterize_single_record(record, column, mode, &texture).unwrap();
                assert_eq!(actual, expected, "{}", vector.name);
                assert_eq!(framebuffer_contract.width, ALIEN_RASTER_WIDTH);
                assert_eq!(framebuffer_contract.height, ALIEN_RASTER_HEIGHT);
                let mut framebuffer = vec![
                    framebuffer_contract.initial_value;
                    framebuffer_contract.width * framebuffer_contract.height
                ];
                for pixel in actual {
                    framebuffer[pixel.y * framebuffer_contract.width + pixel.x] = pixel.value;
                }
                assert_eq!(
                    hex_sha256(&framebuffer),
                    framebuffer_contract.sha256,
                    "{} framebuffer",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn active_span_ordering_matches_every_direct_alien_overlay_vector() {
        let fixtures = [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_2572_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_25d6_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_2696_natural.json"),
        ];
        for fixture in fixtures {
            let vectors: Vec<RendererVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                let (
                    Some(column),
                    Some(records),
                    Some(texture_contract),
                    Some(framebuffer_contract),
                    Some(expected_pixels),
                ) = (
                    vector.bucket_column,
                    vector.initial_records,
                    vector.texture,
                    vector.framebuffer,
                    vector.logical_pixels,
                )
                else {
                    continue;
                };
                let texture = texture_contract.unit.repeat(texture_contract.repetitions);
                let activations = records
                    .into_iter()
                    .map(|record| AlienRasterActivation {
                        first_column: column,
                        record,
                    })
                    .collect::<Vec<_>>();
                let mut framebuffer = vec![
                    framebuffer_contract.initial_value;
                    framebuffer_contract.width * framebuffer_contract.height
                ];
                assert!(rasterize_activations(
                    &activations,
                    texture.len(),
                    |x, y, texture_offset| {
                        framebuffer[y * framebuffer_contract.width + x] = texture[texture_offset];
                    },
                ));
                let actual_pixels = framebuffer
                    .iter()
                    .copied()
                    .enumerate()
                    .filter(|(_offset, value)| *value != framebuffer_contract.initial_value)
                    .map(|(offset, value)| AlienRasterPixel {
                        x: offset % framebuffer_contract.width,
                        y: offset / framebuffer_contract.width,
                        value,
                    })
                    .collect::<Vec<_>>();
                assert_eq!(actual_pixels, expected_pixels, "{}", vector.name);
                assert_eq!(
                    hex_sha256(&framebuffer),
                    framebuffer_contract.sha256,
                    "{} framebuffer",
                    vector.name
                );
            }
        }
    }

    fn hex_sha256(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    impl ExpectedRecord {
        fn assert_matches(&self, actual: AlienRasterRecord, name: &str) {
            macro_rules! field {
                ($field:ident) => {
                    if let Some(expected) = self.$field {
                        assert_eq!(actual.$field, expected, "{} {}", name, stringify!($field));
                    }
                };
            }
            field!(edge_0_position);
            field!(edge_0_step);
            field!(edge_1_position);
            field!(edge_1_step);
            field!(depth_position);
            field!(depth_step);
            field!(depth_gradient);
            field!(advance);
            field!(remaining);
            field!(secondary_remaining);
            field!(secondary_edge_position);
            field!(secondary_edge_step);
            field!(secondary_depth_position);
            field!(secondary_depth_step);
            field!(texture_u);
            field!(texture_v);
            field!(secondary_texture_u);
            field!(secondary_texture_v);
            field!(texture_u_step);
            field!(texture_v_step);
            field!(secondary_texture_u_step);
            field!(secondary_texture_v_step);
            field!(texture_du);
            field!(texture_dv);
            field!(texture_bank);
        }
    }
}
