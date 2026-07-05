use super::*;
use commander_blood_tools::ship3d::{
    Ship3dDirtyRectList, Ship3dObjectSpriteDescriptor, Ship3dPointCloudRender,
    Ship3dProjectionMatrix, Ship3dProjectionOrigin, Ship3dProjectionPoint,
    Ship3dProjectionViewport, Ship3dSpriteSlotRenderCommand,
    collect_ship_3d_dirty_sprite_slot_render_commands, project_ship_3d_object_sprite,
};

// ===========================================================================
// Palette and framebuffer helpers
// ===========================================================================

pub(super) fn parse_palette_block(
    data: &[u8],
    mut pos: usize,
    palette: &mut [[u8; 3]; 256],
) -> usize {
    while pos + 1 < data.len() {
        let start = data[pos];
        let count = data[pos + 1];
        pos += 2;

        if start == 0xFF && count == 0xFF {
            break;
        }

        let n = if count == 0 { 256 } else { count as usize };
        for i in 0..n {
            if pos + 2 >= data.len() {
                return pos;
            }
            let idx = start as usize + i;
            if idx < 256 {
                palette[idx] = [
                    (data[pos] << 2) | (data[pos] >> 4),
                    (data[pos + 1] << 2) | (data[pos + 1] >> 4),
                    (data[pos + 2] << 2) | (data[pos + 2] >> 4),
                ];
            }
            pos += 3;
        }
    }
    pos
}

pub(super) fn fb_to_rgb(fb: &[u8], palette: &[[u8; 3]; 256], rgb: &mut [u8]) {
    for (i, &px) in fb.iter().enumerate() {
        let c = palette[px as usize];
        rgb[i * 3] = c[0];
        rgb[i * 3 + 1] = c[1];
        rgb[i * 3 + 2] = c[2];
    }
}

pub(super) fn fill_rect_indexed_clipped(
    fb: &mut [u8],
    color: u8,
    x: isize,
    y: isize,
    width: isize,
    height: isize,
    clip: (usize, usize, usize, usize),
) {
    if width <= 0 || height <= 0 || fb.len() < VIEWPORT_W * VIEWPORT_H {
        return;
    }

    let (clip_left, clip_right, clip_top, clip_bottom) = clip;
    let x0 = x.max(clip_left as isize).max(0) as usize;
    let y0 = y.max(clip_top as isize).max(0) as usize;
    let x1 = (x + width)
        .min(clip_right as isize)
        .min(VIEWPORT_W as isize)
        .max(x0 as isize) as usize;
    let y1 = (y + height)
        .min(clip_bottom as isize)
        .min(VIEWPORT_H as isize)
        .max(y0 as isize) as usize;

    for row in y0..y1 {
        fb[row * VIEWPORT_W + x0..row * VIEWPORT_W + x1].fill(color);
    }
}

pub(super) fn fill_band_indexed(fb: &mut [u8], color: u8, clip_top: usize, clip_bottom: usize) {
    if clip_bottom <= clip_top {
        return;
    }
    fill_rect_indexed_clipped(
        fb,
        color,
        0,
        clip_top as isize,
        VIEWPORT_W as isize,
        (clip_bottom - clip_top) as isize,
        (0, VIEWPORT_W, clip_top, clip_bottom),
    );
}

pub(super) fn fill_scene_band_indexed(fb: &mut [u8], color: u8) {
    fill_band_indexed(fb, color, SCENE_TOP, SCENE_BOTTOM);
}

pub(super) fn copy_framebuffer_full_indexed(dst: &mut [u8], src: &[u8]) {
    let len = VIEWPORT_W * VIEWPORT_H;
    if dst.len() < len || src.len() < len {
        return;
    }
    dst[..len].copy_from_slice(&src[..len]);
}

pub(super) fn copy_dirty_rects_secondary_to_primary_indexed(
    primary: &mut [u8],
    secondary: &[u8],
    dirty_rects: &[Ship3dProjectionViewport],
    copy_enabled: bool,
) -> usize {
    let len = VIEWPORT_W * VIEWPORT_H;
    if !copy_enabled || primary.len() < len || secondary.len() < len {
        return 0;
    }

    let mut copied = 0usize;
    for rect in dirty_rects {
        if signed_word_to_isize(rect.left) < 0 {
            break;
        }

        let left = clamp_signed_word_to_viewport(rect.left, VIEWPORT_W);
        let right = clamp_signed_word_to_viewport(rect.right, VIEWPORT_W);
        let top = clamp_signed_word_to_viewport(rect.top, VIEWPORT_H);
        let bottom = clamp_signed_word_to_viewport(rect.bottom, VIEWPORT_H);
        if right <= left || bottom <= top {
            continue;
        }

        for y in top..bottom {
            let start = y * VIEWPORT_W + left;
            let end = y * VIEWPORT_W + right;
            primary[start..end].copy_from_slice(&secondary[start..end]);
            copied += end - start;
        }
    }

    copied
}

pub(super) fn remap_rect_indexed_clipped(
    fb: &mut [u8],
    table: &[u8; 256],
    x: isize,
    y: isize,
    width: isize,
    height: isize,
    clip: (usize, usize, usize, usize),
) {
    if width <= 0 || height <= 0 || fb.len() < VIEWPORT_W * VIEWPORT_H {
        return;
    }

    let (clip_left, clip_right, clip_top, clip_bottom) = clip;
    let x0 = x.max(clip_left as isize).max(0) as usize;
    let y0 = y.max(clip_top as isize).max(0) as usize;
    let x1 = (x + width)
        .min(clip_right as isize)
        .min(VIEWPORT_W as isize)
        .max(x0 as isize) as usize;
    let y1 = (y + height)
        .min(clip_bottom as isize)
        .min(VIEWPORT_H as isize)
        .max(y0 as isize) as usize;

    for row in y0..y1 {
        for px in &mut fb[row * VIEWPORT_W + x0..row * VIEWPORT_W + x1] {
            *px = table[*px as usize];
        }
    }
}

pub(super) fn copy_vga_planar_to_linear_indexed(dst: &mut [u8], planes: &[u8]) {
    let len = VIEWPORT_W * VIEWPORT_H;
    let plane_len = len / 4;
    if dst.len() < len || planes.len() < len {
        return;
    }

    for plane in 0..4 {
        let src = &planes[plane * plane_len..(plane + 1) * plane_len];
        for (idx, value) in src.iter().copied().enumerate() {
            dst[idx * 4 + plane] = value;
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct RawSpriteFrame<'a> {
    pub(super) stride: usize,
    pub(super) x_offset: isize,
    pub(super) y_offset: isize,
    pub(super) pixels: &'a [u8],
}

impl<'a> RawSpriteFrame<'a> {
    pub(super) fn parse(data: &'a [u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        let stride = u16::from_le_bytes([data[0], data[1]]) as usize;
        let x_offset = i16::from_le_bytes([data[4], data[5]]) as isize;
        let y_offset = i16::from_le_bytes([data[6], data[7]]) as isize;
        Some(Self {
            stride,
            x_offset,
            y_offset,
            pixels: &data[8..],
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct RleSpriteFrame<'a> {
    pub(super) stride: usize,
    pub(super) x_offset: isize,
    pub(super) y_offset: isize,
    pub(super) encoded: &'a [u8],
}

impl<'a> RleSpriteFrame<'a> {
    pub(super) fn parse(data: &'a [u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        let stride = u16::from_le_bytes([data[0], data[1]]) as usize;
        let x_offset = i16::from_le_bytes([data[4], data[5]]) as isize;
        let y_offset = i16::from_le_bytes([data[6], data[7]]) as isize;
        Some(Self {
            stride,
            x_offset,
            y_offset,
            encoded: &data[8..],
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ScaledSpriteFrame<'a> {
    pub(super) source_width: usize,
    pub(super) source_height: usize,
    pub(super) pixels: &'a [u8],
}

impl<'a> ScaledSpriteFrame<'a> {
    pub(super) fn parse(data: &'a [u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        let source_width = u16::from_le_bytes([data[0], data[1]]) as usize;
        let source_height = u16::from_le_bytes([data[2], data[3]]) as usize;
        Some(Self {
            source_width,
            source_height,
            pixels: &data[8..],
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SpriteBlitRequest {
    pub(super) x: isize,
    pub(super) y: isize,
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) flip_x: bool,
    pub(super) flip_y: bool,
    pub(super) clip: (usize, usize, usize, usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Ship3dSpriteSlotFrame<'a> {
    Raw(&'a [u8]),
    Rle(&'a [u8]),
    Scaled(&'a [u8]),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SpriteSlotFrameTable<'a> {
    pub(super) flags: u16,
    pub(super) frame_offsets: Vec<usize>,
    pub(super) frames: Vec<&'a [u8]>,
}

impl<'a> SpriteSlotFrameTable<'a> {
    pub(super) fn parse(data: &'a [u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }

        let flags = u16::from_le_bytes([data[0], data[1]]);
        let frame_count = u16::from_le_bytes([data[2], data[3]]) as usize;
        let table_end = 4usize.checked_add(frame_count.checked_mul(4)?)?;
        if table_end > data.len() {
            return None;
        }

        let mut frame_starts = Vec::with_capacity(frame_count);
        for idx in 0..frame_count {
            let pos = 4 + idx * 4;
            let packed_offset =
                u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                    as usize;
            let frame_start = 4usize.checked_add(packed_offset)?;
            if frame_start < table_end || frame_start + 8 > data.len() {
                return None;
            }
            frame_starts.push(frame_start);
        }

        let mut frames = Vec::with_capacity(frame_count);
        for (idx, frame_start) in frame_starts.iter().copied().enumerate() {
            let frame_end = frame_starts.get(idx + 1).copied().unwrap_or(data.len());
            if frame_end < frame_start || frame_end > data.len() {
                return None;
            }
            frames.push(&data[frame_start..frame_end]);
        }

        Some(Self {
            flags,
            frame_offsets: frame_starts,
            frames,
        })
    }

    pub(super) fn slot_state_flags(&self) -> u16 {
        (self.flags & 0x0004) | 0x0083
    }

    pub(super) fn dispatch_index(&self) -> u8 {
        ((self.slot_state_flags() >> 1) & 0x07) as u8
    }

    pub(super) fn frame(&self, index: usize) -> Option<Ship3dSpriteSlotFrame<'a>> {
        let data = *self.frames.get(index)?;
        ship_3d_sprite_slot_frame_for_dispatch(data, self.dispatch_index())
    }
}

pub(super) fn ship_3d_sprite_slot_frame_for_dispatch(
    data: &[u8],
    dispatch_index: u8,
) -> Option<Ship3dSpriteSlotFrame<'_>> {
    match dispatch_index {
        0 | 2 => Some(Ship3dSpriteSlotFrame::Raw(data)),
        1 | 3 => Some(Ship3dSpriteSlotFrame::Rle(data)),
        4 => Some(Ship3dSpriteSlotFrame::Scaled(data)),
        _ => None,
    }
}

/// One decoded `.spr` frame as a width x height grid of palette indices.
pub(super) struct SpriteFrameImage {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) indices: Vec<u8>,
}

/// Decode every frame of a `.spr` sprite bank to palette-index grids. The frame
/// header is `[0]=width, [2]=height, [4]=x offset, [6]=y offset`; Raw frames are
/// `width*height` bytes after the 8-byte header, RLE frames decode via
/// [`decode_rle_sprite_pixels`]. Returns `None` if the bank header is unparseable;
/// individual frames that fail to decode are skipped. This exposes the verified
/// decode for inspection tooling (`--spr`).
pub(super) fn decode_sprite_bank_indices(data: &[u8]) -> Option<Vec<SpriteFrameImage>> {
    let table = SpriteSlotFrameTable::parse(data)?;
    let dispatch = table.dispatch_index();
    let mut out = Vec::new();
    for frame in &table.frames {
        if frame.len() < 8 {
            continue;
        }
        let width = u16::from_le_bytes([frame[0], frame[1]]) as usize;
        let height = u16::from_le_bytes([frame[2], frame[3]]) as usize;
        if width == 0 || height == 0 {
            continue;
        }
        let indices = match ship_3d_sprite_slot_frame_for_dispatch(frame, dispatch) {
            Some(Ship3dSpriteSlotFrame::Raw(d)) => {
                let body = &d[8..];
                if body.len() < width * height {
                    continue;
                }
                body[..width * height].to_vec()
            }
            Some(Ship3dSpriteSlotFrame::Rle(d)) => {
                match RleSpriteFrame::parse(d).and_then(|f| decode_rle_sprite_pixels(f, height)) {
                    Some(px) if px.len() == width * height => px,
                    _ => continue,
                }
            }
            _ => continue,
        };
        out.push(SpriteFrameImage {
            width,
            height,
            indices,
        });
    }
    Some(out)
}

pub(super) fn blit_raw_transparent_sprite_indexed(
    fb: &mut [u8],
    frame: RawSpriteFrame<'_>,
    request: SpriteBlitRequest,
    remap_table: Option<&[u8; 256]>,
) {
    blit_raw_sprite_indexed(fb, frame, request, true, remap_table);
}

pub(super) fn blit_raw_opaque_sprite_indexed(
    fb: &mut [u8],
    frame: RawSpriteFrame<'_>,
    request: SpriteBlitRequest,
) {
    blit_raw_sprite_indexed(fb, frame, request, false, None);
}

pub(super) fn blit_rle_transparent_sprite_indexed(
    fb: &mut [u8],
    frame: RleSpriteFrame<'_>,
    request: SpriteBlitRequest,
    remap_table: Option<&[u8; 256]>,
) {
    let Some(pixels) = decode_rle_sprite_pixels(frame, request.height) else {
        return;
    };
    let raw = RawSpriteFrame {
        stride: frame.stride,
        x_offset: frame.x_offset,
        y_offset: frame.y_offset,
        pixels: &pixels,
    };
    blit_raw_transparent_sprite_indexed(fb, raw, request, remap_table);
}

pub(super) fn blit_rle_opaque_sprite_indexed(
    fb: &mut [u8],
    frame: RleSpriteFrame<'_>,
    request: SpriteBlitRequest,
) {
    let Some(pixels) = decode_rle_sprite_pixels(frame, request.height) else {
        return;
    };
    let raw = RawSpriteFrame {
        stride: frame.stride,
        x_offset: frame.x_offset,
        y_offset: frame.y_offset,
        pixels: &pixels,
    };
    blit_raw_opaque_sprite_indexed(fb, raw, request);
}

pub(super) fn blit_scaled_transparent_sprite_indexed(
    fb: &mut [u8],
    frame: ScaledSpriteFrame<'_>,
    request: SpriteBlitRequest,
) {
    if fb.len() < VIEWPORT_W * VIEWPORT_H
        || frame.source_width == 0
        || frame.source_height == 0
        || request.width == 0
        || request.height == 0
    {
        return;
    }

    let rect_left = request.x;
    let rect_top = request.y;
    let rect_right = rect_left + request.width as isize;
    let rect_bottom = rect_top + request.height as isize;

    let (clip_left, clip_right, clip_top, clip_bottom) = request.clip;
    let x0 = rect_left
        .max(clip_left as isize)
        .max(0)
        .min(VIEWPORT_W as isize) as usize;
    let y0 = rect_top
        .max(clip_top as isize)
        .max(0)
        .min(VIEWPORT_H as isize) as usize;
    let x1 = rect_right
        .min(clip_right as isize)
        .min(VIEWPORT_W as isize)
        .max(x0 as isize) as usize;
    let y1 = rect_bottom
        .min(clip_bottom as isize)
        .min(VIEWPORT_H as isize)
        .max(y0 as isize) as usize;

    if x1 <= x0 || y1 <= y0 {
        return;
    }

    let x_step = ((frame.source_width as u64) << 16) / request.width as u64;
    let y_step = ((frame.source_height as u64) << 16) / request.height as u64;
    let left_skip = x0 as isize - rect_left;
    let top_skip = y0 as isize - rect_top;

    for dst_y in y0..y1 {
        let scaled_y = (top_skip as u64 + (dst_y - y0) as u64) * y_step;
        let source_y = (scaled_y >> 16) as usize;
        if source_y >= frame.source_height {
            continue;
        }
        let source_row = source_y * frame.source_width;
        for dst_x in x0..x1 {
            let scaled_x = (left_skip as u64 + (dst_x - x0) as u64) * x_step;
            let source_x = (scaled_x >> 16) as usize;
            if source_x >= frame.source_width {
                continue;
            }
            let Some(source_pixel) = frame.pixels.get(source_row + source_x).copied() else {
                continue;
            };
            if source_pixel != 0 {
                fb[dst_y * VIEWPORT_W + dst_x] = source_pixel;
            }
        }
    }
}

pub(super) fn blit_ship_3d_sprite_slot_command_indexed(
    fb: &mut [u8],
    command: Ship3dSpriteSlotRenderCommand,
    frame: Ship3dSpriteSlotFrame<'_>,
    remap_table_5f11: Option<&[u8; 256]>,
    remap_table_6011: Option<&[u8; 256]>,
) -> bool {
    let request = SpriteBlitRequest {
        x: signed_word_to_isize(command.slot_rect.left),
        y: signed_word_to_isize(command.slot_rect.top),
        width: command.slot_rect.right.wrapping_sub(command.slot_rect.left) as usize,
        height: command.slot_rect.bottom.wrapping_sub(command.slot_rect.top) as usize,
        flip_x: command.flip_x,
        flip_y: command.flip_y,
        clip: ship_3d_viewport_clip(command.dirty_rect),
    };
    let remap_table = ship_3d_destination_remap_table(
        command.destination_remap_mode,
        remap_table_5f11,
        remap_table_6011,
    );

    match (command.dispatch_index, frame) {
        (0, Ship3dSpriteSlotFrame::Raw(data)) => {
            let Some(frame) = RawSpriteFrame::parse(data) else {
                return false;
            };
            blit_raw_transparent_sprite_indexed(fb, frame, request, remap_table);
            true
        }
        (1, Ship3dSpriteSlotFrame::Rle(data)) => {
            let Some(frame) = RleSpriteFrame::parse(data) else {
                return false;
            };
            blit_rle_transparent_sprite_indexed(fb, frame, request, remap_table);
            true
        }
        (2, Ship3dSpriteSlotFrame::Raw(data)) => {
            let Some(frame) = RawSpriteFrame::parse(data) else {
                return false;
            };
            blit_raw_opaque_sprite_indexed(fb, frame, request);
            true
        }
        (3, Ship3dSpriteSlotFrame::Rle(data)) => {
            let Some(frame) = RleSpriteFrame::parse(data) else {
                return false;
            };
            blit_rle_opaque_sprite_indexed(fb, frame, request);
            true
        }
        (4, Ship3dSpriteSlotFrame::Scaled(data)) => {
            let Some(frame) = ScaledSpriteFrame::parse(data) else {
                return false;
            };
            blit_scaled_transparent_sprite_indexed(fb, frame, request);
            true
        }
        (5..=7, _) => true,
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct Ship3dDirtySpriteRenderResult {
    pub(super) rendered_commands: usize,
    pub(super) missing_frames: usize,
    pub(super) rejected_commands: usize,
    pub(super) copied_pixels: usize,
}

pub(super) fn render_ship_3d_dirty_sprite_commands_indexed<'a, F>(
    primary: &mut [u8],
    secondary: &mut [u8],
    commands: &[Ship3dSpriteSlotRenderCommand],
    dirty_rects: &[Ship3dProjectionViewport],
    copyback_enabled: bool,
    mut frame_for_command: F,
    remap_table_5f11: Option<&[u8; 256]>,
    remap_table_6011: Option<&[u8; 256]>,
) -> Ship3dDirtySpriteRenderResult
where
    F: FnMut(&Ship3dSpriteSlotRenderCommand) -> Option<Ship3dSpriteSlotFrame<'a>>,
{
    let mut result = Ship3dDirtySpriteRenderResult::default();

    for command in commands {
        let Some(frame) = frame_for_command(command) else {
            result.missing_frames += 1;
            continue;
        };

        if blit_ship_3d_sprite_slot_command_indexed(
            secondary,
            *command,
            frame,
            remap_table_5f11,
            remap_table_6011,
        ) {
            result.rendered_commands += 1;
        } else {
            result.rejected_commands += 1;
        }
    }

    result.copied_pixels = copy_dirty_rects_secondary_to_primary_indexed(
        primary,
        secondary,
        dirty_rects,
        copyback_enabled,
    );
    result
}

/// Compose a complete ship-3D view frame from the individually-tested pipeline
/// stages: start from the `render_ship_3d_starfield` background, project each
/// object's 3D anchor into its slot descriptor (`project_ship_3d_object_sprite`),
/// then composite the sprite slots over the background with the double-buffered
/// dirty-rect compositor. Returns the 320x200 indexed frame. This is the
/// top-level integration that wires the ship-3D render chain into one scene
/// render — previously the stages (projection, command collection, blit,
/// starfield) were only exercised separately.
pub(super) fn compose_ship_3d_scene_indexed<'a, F>(
    background: &Ship3dPointCloudRender,
    slots: &mut [Ship3dObjectSpriteDescriptor],
    anchors: &[Ship3dProjectionPoint],
    origin: Ship3dProjectionOrigin,
    matrix: Ship3dProjectionMatrix,
    frame_for_command: F,
    remap_5f11: Option<&[u8; 256]>,
    remap_6011: Option<&[u8; 256]>,
) -> Vec<u8>
where
    F: FnMut(&Ship3dSpriteSlotRenderCommand) -> Option<Ship3dSpriteSlotFrame<'a>>,
{
    for (slot, anchor) in slots.iter_mut().zip(anchors.iter()) {
        let _ = project_ship_3d_object_sprite(*anchor, origin, matrix, slot);
    }
    let mut primary = background.buffer.clone();
    let mut secondary = background.buffer.clone();
    if slots.is_empty() {
        return primary;
    }
    let dirty = Ship3dDirtyRectList {
        rects: vec![Ship3dProjectionViewport {
            left: 0,
            top: 0,
            right: VIEWPORT_W as u16,
            bottom: VIEWPORT_H as u16,
        }],
        sentinel: 0,
    };
    let commands =
        collect_ship_3d_dirty_sprite_slot_render_commands(slots, &dirty, 0, slots.len() - 1);
    render_ship_3d_dirty_sprite_commands_indexed(
        &mut primary,
        &mut secondary,
        &commands,
        &dirty.rects,
        true,
        frame_for_command,
        remap_5f11,
        remap_6011,
    );
    primary
}

fn decode_rle_sprite_pixels(frame: RleSpriteFrame<'_>, height: usize) -> Option<Vec<u8>> {
    if frame.stride == 0 {
        return None;
    }

    let len = frame.stride.checked_mul(height)?;
    let mut pixels = Vec::with_capacity(len);
    let mut pos = 0usize;
    for _ in 0..height {
        let row_start = pixels.len();
        while pixels.len() - row_start < frame.stride {
            let control = *frame.encoded.get(pos)?;
            pos += 1;
            if control & 0x80 != 0 {
                let run_len = (0u8.wrapping_sub(control) as usize) + 1;
                if pixels.len() - row_start + run_len > frame.stride {
                    return None;
                }
                let value = *frame.encoded.get(pos)?;
                pos += 1;
                pixels.extend(std::iter::repeat(value).take(run_len));
            } else {
                let run_len = control as usize + 1;
                if pixels.len() - row_start + run_len > frame.stride {
                    return None;
                }
                let end = pos.checked_add(run_len)?;
                pixels.extend_from_slice(frame.encoded.get(pos..end)?);
                pos = end;
            }
        }
    }

    Some(pixels)
}

fn blit_raw_sprite_indexed(
    fb: &mut [u8],
    frame: RawSpriteFrame<'_>,
    request: SpriteBlitRequest,
    transparent_zero: bool,
    remap_table: Option<&[u8; 256]>,
) {
    if fb.len() < VIEWPORT_W * VIEWPORT_H
        || frame.stride == 0
        || request.width == 0
        || request.height == 0
    {
        return;
    }

    let rect_left = request.x + frame.x_offset;
    let rect_top = request.y + frame.y_offset;
    let rect_right = rect_left + request.width as isize;
    let rect_bottom = rect_top + request.height as isize;

    let (clip_left, clip_right, clip_top, clip_bottom) = request.clip;
    let x0 = rect_left
        .max(clip_left as isize)
        .max(0)
        .min(VIEWPORT_W as isize) as usize;
    let y0 = rect_top
        .max(clip_top as isize)
        .max(0)
        .min(VIEWPORT_H as isize) as usize;
    let x1 = rect_right
        .min(clip_right as isize)
        .min(VIEWPORT_W as isize)
        .max(x0 as isize) as usize;
    let y1 = rect_bottom
        .min(clip_bottom as isize)
        .min(VIEWPORT_H as isize)
        .max(y0 as isize) as usize;

    if x1 <= x0 || y1 <= y0 {
        return;
    }

    for dst_y in y0..y1 {
        let source_y = if request.flip_y {
            (rect_bottom - 1 - dst_y as isize) as usize
        } else {
            (dst_y as isize - rect_top) as usize
        };
        let source_row = source_y.saturating_mul(frame.stride);
        for dst_x in x0..x1 {
            let source_x = if request.flip_x {
                (rect_right - 1 - dst_x as isize) as usize
            } else {
                (dst_x as isize - rect_left) as usize
            };
            let Some(source_pixel) = frame.pixels.get(source_row + source_x).copied() else {
                continue;
            };
            let dst_idx = dst_y * VIEWPORT_W + dst_x;
            if transparent_zero && source_pixel == 0 {
                continue;
            }
            fb[dst_idx] = if let Some(table) = remap_table {
                table[fb[dst_idx] as usize]
            } else {
                source_pixel
            };
        }
    }
}

fn ship_3d_destination_remap_table<'a>(
    mode: u8,
    remap_table_5f11: Option<&'a [u8; 256]>,
    remap_table_6011: Option<&'a [u8; 256]>,
) -> Option<&'a [u8; 256]> {
    match mode & 0x03 {
        0 => None,
        1 => remap_table_5f11,
        _ => remap_table_6011,
    }
}

fn ship_3d_viewport_clip(viewport: Ship3dProjectionViewport) -> (usize, usize, usize, usize) {
    (
        clamp_signed_word_to_viewport(viewport.left, VIEWPORT_W),
        clamp_signed_word_to_viewport(viewport.right, VIEWPORT_W),
        clamp_signed_word_to_viewport(viewport.top, VIEWPORT_H),
        clamp_signed_word_to_viewport(viewport.bottom, VIEWPORT_H),
    )
}

fn clamp_signed_word_to_viewport(value: u16, limit: usize) -> usize {
    signed_word_to_isize(value).clamp(0, limit as isize) as usize
}

fn signed_word_to_isize(value: u16) -> isize {
    value as i16 as isize
}

pub(super) fn render_subtitles_indexed(fb: &mut [u8], cues: &[SubtitleCue], time: f64) {
    let Some((cue, visible_lines)) = active_subtitle_lines(cues, time) else {
        return;
    };

    let (clip_top, clip_bottom) = subtitle_clip_bounds(cue.active_line_id);
    for (line_idx, line) in visible_lines.iter().enumerate() {
        let y = SUBTITLE_Y + line_idx * GAME_FONT_LINE_HEIGHT;
        draw_game_text_indexed_clipped(fb, line, SUBTITLE_X, y, clip_top, clip_bottom);
    }
}

pub(super) fn render_subtitles_rgb(
    rgb: &mut [u8],
    palette: &[[u8; 3]; 256],
    cues: &[SubtitleCue],
    time: f64,
) {
    let Some((cue, visible_lines)) = active_subtitle_lines(cues, time) else {
        return;
    };

    let (clip_top, clip_bottom) = subtitle_clip_bounds(cue.active_line_id);
    for (line_idx, line) in visible_lines.iter().enumerate() {
        let y = SUBTITLE_Y + line_idx * GAME_FONT_LINE_HEIGHT;
        draw_game_text_rgb_clipped(rgb, palette, line, SUBTITLE_X, y, clip_top, clip_bottom);
    }
}

fn active_subtitle_lines(cues: &[SubtitleCue], time: f64) -> Option<(&SubtitleCue, Vec<String>)> {
    let Some((_, cue)) = cues.iter().enumerate().find(|(idx, cue)| {
        let start = cue.tick as f64 / 10.0;
        let end = cue_end_time(cues, *idx);
        time >= start && time < end
    }) else {
        return None;
    };

    let full_text = cue.text.trim();
    if full_text.is_empty() {
        return None;
    }

    let start = cue.tick as f64 / 10.0;
    let visible_chars =
        ((time - start).max(0.0) * default_subtitle_reveal_chars_per_second()).ceil() as usize;
    if visible_chars == 0 {
        return None;
    }

    // The line breaks are already game-exact: assemble_dialogue inserts them at
    // the 35-char boundary like the game's 0xA6 handler (see re/REVERSE.md). Use
    // those breaks directly rather than re-wrapping by pixel width.
    let lines: Vec<String> = full_text
        .replace('\r', "\n")
        .lines()
        .map(|l| l.to_string())
        .collect();
    let visible_lines = visible_subtitle_lines(&lines, visible_chars);
    Some((cue, visible_lines))
}

pub(super) fn subtitle_clip_bounds(active_line_id: Option<u16>) -> (usize, usize) {
    match active_line_id {
        // BLOODPRG.EXE's per-frame dialogue updater sets render_string clipping
        // to gs:0x5239..0x523B for these active line ids, then restores it.
        Some(5 | 0x27) => (SCENE_TOP, SCENE_BOTTOM),
        _ => (0, VIEWPORT_H),
    }
}

pub(super) fn cue_end_time(cues: &[SubtitleCue], idx: usize) -> f64 {
    let start = cues[idx].tick as f64 / 10.0;
    cues.get(idx + 1)
        .map(|next| next.tick as f64 / 10.0)
        .filter(|end| *end > start + 0.25)
        .unwrap_or(start + 4.0)
}

pub(super) fn visible_subtitle_lines(lines: &[String], visible_chars: usize) -> Vec<String> {
    let mut remaining = visible_chars;
    let mut out = Vec::new();
    for line in lines {
        if remaining == 0 {
            break;
        }
        let line_len = line.chars().count();
        let take = remaining.min(line_len);
        out.push(line.chars().take(take).collect());
        remaining = remaining.saturating_sub(line_len);
    }
    out
}

fn draw_game_text_indexed_clipped(
    fb: &mut [u8],
    text: &str,
    x: usize,
    y: usize,
    clip_top: usize,
    clip_bottom: usize,
) {
    let visible_chars = text.chars().count();
    let mut cx = x;
    for (char_index, ch) in text.chars().enumerate() {
        if let Some(glyph) = subtitle_draw_glyph(ch) {
            draw_game_glyph_indexed_clipped(
                fb,
                glyph.rows,
                cx,
                y,
                subtitle_glyph_color_index(char_index, visible_chars),
                clip_top,
                clip_bottom,
            );
        }
        cx += game_font_advance(ch);
        if cx >= VIEWPORT_W {
            break;
        }
    }
}

fn draw_game_text_rgb_clipped(
    rgb: &mut [u8],
    palette: &[[u8; 3]; 256],
    text: &str,
    x: usize,
    y: usize,
    clip_top: usize,
    clip_bottom: usize,
) {
    let visible_chars = text.chars().count();
    let mut cx = x;
    for (char_index, ch) in text.chars().enumerate() {
        if let Some(glyph) = subtitle_draw_glyph(ch) {
            draw_game_glyph_rgb_clipped(
                rgb,
                glyph.rows,
                cx,
                y,
                palette[subtitle_glyph_color_index(char_index, visible_chars) as usize],
                clip_top,
                clip_bottom,
            );
        }
        cx += game_font_advance(ch);
        if cx >= VIEWPORT_W {
            break;
        }
    }
}

fn subtitle_glyph_color_index(char_index: usize, visible_chars: usize) -> u8 {
    if char_index + 1 == visible_chars {
        SUBTITLE_COLOR_REVEAL_EDGE
    } else {
        SUBTITLE_COLOR_REVEALED
    }
}

fn draw_game_glyph_indexed_clipped(
    fb: &mut [u8],
    rows: [u8; GAME_FONT_HEIGHT],
    x: usize,
    y: usize,
    color_index: u8,
    clip_top: usize,
    clip_bottom: usize,
) {
    for (gy, row) in rows.iter().copied().enumerate() {
        for gx in 0..GAME_FONT_WIDTH {
            if (row & (0x80 >> gx)) == 0 {
                continue;
            }
            let px = x + gx;
            let py = y + gy;
            if px < VIEWPORT_W && py >= clip_top && py < clip_bottom && py < VIEWPORT_H {
                fb[py * VIEWPORT_W + px] = color_index;
            }
        }
    }
}

fn draw_game_glyph_rgb_clipped(
    rgb: &mut [u8],
    rows: [u8; GAME_FONT_HEIGHT],
    x: usize,
    y: usize,
    color: [u8; 3],
    clip_top: usize,
    clip_bottom: usize,
) {
    for (gy, row) in rows.iter().copied().enumerate() {
        for gx in 0..GAME_FONT_WIDTH {
            if (row & (0x80 >> gx)) == 0 {
                continue;
            }
            let px = x + gx;
            let py = y + gy;
            if px < VIEWPORT_W && py >= clip_top && py < clip_bottom && py < VIEWPORT_H {
                let idx = (py * VIEWPORT_W + px) * 3;
                rgb[idx] = color[0];
                rgb[idx + 1] = color[1];
                rgb[idx + 2] = color[2];
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct GameFontGlyph {
    pub(super) rows: [u8; GAME_FONT_HEIGHT],
    pub(super) advance: usize,
}

/// The glyph to blit for a subtitle character. Unknown glyphs fall back to '?'
/// so garbage is visible, BUT a space (' ') draws NOTHING (returns None) — the
/// game's `render_string` (BLOODPRG.EXE @0x31D7) skips the space glyph and only
/// advances DI by 6. Without this, `game_font_glyph(' ')` (None, no space glyph)
/// fell through to the '?' fallback and rendered every space as '?'. Verified
/// against a real playthrough (spaces were showing as '?' in generated subtitles).
pub(super) fn subtitle_draw_glyph(ch: char) -> Option<GameFontGlyph> {
    if ch == ' ' {
        return None;
    }
    game_font_glyph(ch).or_else(|| game_font_glyph('?'))
}

pub(super) fn game_font_glyph(ch: char) -> Option<GameFontGlyph> {
    let code = ch as usize;
    let idx = *GAME_FONT_CHAR_MAP.get(code)?;
    if idx == 0xff {
        return None;
    }
    let idx = idx as usize;
    Some(GameFontGlyph {
        rows: GAME_FONT_GLYPHS[idx],
        advance: GAME_FONT_WIDTHS[idx] as usize,
    })
}

pub(super) fn game_font_advance(ch: char) -> usize {
    if ch == ' ' {
        return GAME_FONT_SPACE_ADVANCE;
    }
    game_font_glyph(ch)
        .or_else(|| game_font_glyph('?'))
        .map(|glyph| glyph.advance)
        .unwrap_or(GAME_FONT_SPACE_ADVANCE)
}

// Dialogue scene layout, RE'd from BLOODPRG.EXE. The scene is composited in the
// framebuffer band `gs:0x5239..0x523B` with black bars / HUD outside. The
// letterbox mode (gs:0x2793 & 8) uses rows 0x23..0xA5 (35..165), a 130px band
// that the 320x130 talk-HNM frames fill exactly. See re/REVERSE.md.
pub(super) const SCENE_TOP: usize = 0x23; // 35
pub(super) const SCENE_BOTTOM: usize = 0xA5; // 165

// BLOODPRG.EXE's reveal renderer is called from 0x94EE with BX=[0x5E5C] and
// DX=[0x5E5E]. The initialized words at those DS offsets are 10 and 8; each
// CR-delimited subtitle line advances DX by 8.
pub(super) const SUBTITLE_X: usize = 10;
pub(super) const SUBTITLE_Y: usize = 8;
pub(super) const GAME_FONT_WIDTH: usize = 8;
pub(super) const GAME_FONT_HEIGHT: usize = 8;
pub(super) const GAME_FONT_LINE_HEIGHT: usize = 8;
pub(super) const SUBTITLE_COLOR_REVEALED: u8 = 0xFD;
pub(super) const SUBTITLE_COLOR_REVEAL_EDGE: u8 = 0xFE;
// The subtitle reveal draws with palette indices 0xFD (revealed) / 0xFE (edge)
// — see REVERSE.md "0xFD for already-revealed glyphs and 0xFE" for the edge.
// These are RESERVED high-palette entries (0xC0..0xFF) that the game fills at
// runtime; a scene's LBM/HNM palette leaves them [0,0,0], so drawing the subtitle
// through the raw scene palette renders it BLACK (invisible) — verified against a
// real playthrough where some scenes (e.g. usine/moskit10) showed no subtitle.
// The near-white [245,245,245] is the game's subtitle colour (matches the fixed
// RGB used before subtitles moved to palette indices in commit 881b184).
const SUBTITLE_RGB: [u8; 3] = [245, 245, 245];

/// Set the reserved subtitle-reveal palette entries (0xFD/0xFE) to the game's
/// subtitle colour. Call on any scene palette before rendering subtitles through
/// it, so the reveal is visible regardless of what the scene LBM/HNM left at those
/// reserved indices. Safe: 0xFD/0xFE are reserved and unused by scene backgrounds.
pub(super) fn apply_reserved_subtitle_palette(pal: &mut [[u8; 3]; 256]) {
    pal[SUBTITLE_COLOR_REVEALED as usize] = SUBTITLE_RGB;
    pal[SUBTITLE_COLOR_REVEAL_EDGE as usize] = SUBTITLE_RGB;
}
// Space advance: the game's glyph blitter (BLOODPRG.EXE render_string @0x31D7)
// advances a 0x20 space by 6 pixels (`add di, 6`), not a full glyph cell.
pub(super) const GAME_FONT_SPACE_ADVANCE: usize = 6;

// Extracted from BLOODPRG.EXE:
// - ASCII to glyph index map: file offset 0x14c22
// - glyph advances: file offsets 0x14cd2..0x14d27
// - 8-byte glyph rows: file offset 0x14d28
#[rustfmt::skip]
pub(super) const GAME_FONT_CHAR_MAP: [u8; 128] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0x1c, 0x24, 0xff, 0xff, 0xff, 0xff, 0x26, 0xff, 0xff, 0xff, 0x23, 0x25, 0x22, 0x1e, 0xff,
    0x4c, 0x4d, 0x4e, 0x4f, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x1f, 0x20, 0xff, 0xff, 0xff, 0x1a,
    0xff, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
    0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0xff, 0xff, 0xff, 0xff, 0x21,
    0xff, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35,
    0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f, 0x40, 0xff, 0xff, 0xff, 0xff, 0xff,
];

#[rustfmt::skip]
pub(super) const GAME_FONT_WIDTHS: [u8; 86] = [
    0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x03, 0x09, 0x09, 0x09, 0x0a, 0x09, 0x09, 0x09,
    0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x0a, 0x09, 0x09, 0x09, 0x03, 0x03, 0x03, 0x03,
    0x03, 0x05, 0x07, 0x07, 0x07, 0x03, 0x03, 0x08, 0x08, 0x08, 0x08, 0x08, 0x06, 0x08, 0x08, 0x03,
    0x06, 0x08, 0x03, 0x09, 0x08, 0x08, 0x08, 0x08, 0x06, 0x08, 0x06, 0x08, 0x09, 0x08, 0x08, 0x08,
    0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x05, 0x05, 0x08, 0x08, 0x08, 0x08, 0x09, 0x04, 0x09, 0x09,
    0x09, 0x09, 0x09, 0x09, 0x09, 0x09,
];

#[rustfmt::skip]
pub(super) const GAME_FONT_GLYPHS: [[u8; GAME_FONT_HEIGHT]; 86] = [
    [0x00, 0x7e, 0x82, 0x82, 0x82, 0xfe, 0x82, 0x00],
    [0x00, 0xfc, 0x84, 0xfe, 0x82, 0x82, 0xfe, 0x00],
    [0x00, 0xfc, 0x80, 0x80, 0x80, 0x80, 0xfe, 0x00],
    [0x00, 0xfc, 0x86, 0x82, 0x82, 0x82, 0xfe, 0x00],
    [0x00, 0xfe, 0x80, 0xfe, 0x80, 0x80, 0xfe, 0x00],
    [0x00, 0xfe, 0x80, 0xfe, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0xfc, 0x80, 0x80, 0x86, 0x82, 0xfe, 0x00],
    [0x00, 0x82, 0x82, 0x82, 0xfe, 0x82, 0x82, 0x00],
    [0x00, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0x02, 0x02, 0x02, 0x02, 0x82, 0xfe, 0x00],
    [0x00, 0x84, 0x84, 0x84, 0xfc, 0x82, 0x82, 0x00],
    [0x00, 0x80, 0x80, 0x80, 0x80, 0x80, 0xfe, 0x00],
    [0x00, 0xe7, 0x99, 0x81, 0x81, 0x81, 0x81, 0x00],
    [0x00, 0xc2, 0xa2, 0x92, 0x8a, 0x86, 0x82, 0x00],
    [0x00, 0xfe, 0x82, 0x82, 0x82, 0x82, 0xfe, 0x00],
    [0x00, 0xfe, 0x82, 0x82, 0xfe, 0x80, 0x80, 0x00],
    [0x00, 0xfe, 0x82, 0x82, 0x82, 0xfe, 0x02, 0x00],
    [0x00, 0xfe, 0x82, 0x82, 0xfc, 0x82, 0x82, 0x00],
    [0x00, 0xfe, 0x80, 0xfe, 0x02, 0x02, 0xfe, 0x00],
    [0x00, 0xfe, 0x20, 0x20, 0x20, 0x20, 0x20, 0x00],
    [0x00, 0x82, 0x82, 0x82, 0x82, 0x82, 0x7c, 0x00],
    [0x00, 0x82, 0x82, 0x82, 0x44, 0x28, 0x10, 0x00],
    [0x00, 0x81, 0x81, 0x81, 0x81, 0x99, 0x66, 0x00],
    [0x00, 0x82, 0x44, 0x38, 0x44, 0x82, 0x82, 0x00],
    [0x00, 0x82, 0x82, 0x82, 0x7e, 0x04, 0x78, 0x00],
    [0x00, 0xfe, 0x02, 0x7c, 0x80, 0x80, 0xfe, 0x00],
    [0x00, 0xfe, 0x82, 0x1e, 0x10, 0x00, 0x10, 0x00],
    [0x00, 0x10, 0x00, 0x10, 0xf0, 0x82, 0xfe, 0x00],
    [0x00, 0x80, 0x80, 0x80, 0x80, 0x00, 0x80, 0x00],
    [0x00, 0x80, 0x00, 0x80, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00],
    [0x00, 0x00, 0x80, 0x00, 0x00, 0x80, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x80, 0x80],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xe0, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0xf8, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x20, 0x20, 0xf8, 0x20, 0x20, 0x00],
    [0x00, 0xa0, 0xa0, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x80],
    [0x00, 0x80, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x7c, 0x04, 0xfc, 0x84, 0xfc, 0x00],
    [0x00, 0x80, 0xfc, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x00, 0x00, 0xf8, 0x80, 0x80, 0x80, 0xfc, 0x00],
    [0x00, 0x04, 0xfc, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x00, 0x00, 0xfc, 0x84, 0xfc, 0x80, 0xfc, 0x00],
    [0x00, 0xf0, 0x80, 0xf0, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0x00, 0xfc, 0x84, 0x84, 0xfc, 0x04, 0x7c],
    [0x00, 0x80, 0xfc, 0x84, 0x84, 0x84, 0x84, 0x00],
    [0x80, 0x00, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00],
    [0x10, 0x00, 0x10, 0x10, 0x10, 0x10, 0x90, 0xf0],
    [0x00, 0x80, 0x88, 0x88, 0xf8, 0x84, 0x84, 0x00],
    [0x00, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0x00, 0xec, 0x92, 0x92, 0x92, 0x92, 0x00],
    [0x00, 0x00, 0xf8, 0x84, 0x84, 0x84, 0x84, 0x00],
    [0x00, 0x00, 0xfc, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x00, 0x00, 0xfc, 0x84, 0x84, 0xfc, 0x80, 0x80],
    [0x00, 0x00, 0xfc, 0x84, 0x84, 0x84, 0xfc, 0x04],
    [0x00, 0x00, 0xf0, 0x90, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0x00, 0xfc, 0x80, 0xfc, 0x04, 0xfc, 0x00],
    [0x00, 0x80, 0xf0, 0x80, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0x00, 0x84, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x00, 0x00, 0x84, 0x84, 0x84, 0x48, 0x30, 0x00],
    [0x00, 0x00, 0x82, 0x82, 0x82, 0x92, 0x6c, 0x00],
    [0x00, 0x00, 0x84, 0x48, 0x30, 0x48, 0x84, 0x00],
    [0x00, 0x00, 0x84, 0x84, 0x84, 0xfc, 0x10, 0x10],
    [0x00, 0x00, 0xfc, 0x04, 0x78, 0x80, 0xfc, 0x00],
    [0x48, 0x00, 0x7c, 0x04, 0xfc, 0x84, 0xfc, 0x00],
    [0x78, 0x00, 0x7c, 0x04, 0xfc, 0x84, 0xfc, 0x00],
    [0x48, 0x00, 0xfc, 0x84, 0xfc, 0x80, 0xfc, 0x00],
    [0x78, 0x00, 0xfc, 0x84, 0xfc, 0x80, 0xfc, 0x00],
    [0xa0, 0x00, 0x40, 0x40, 0x40, 0x40, 0x40, 0x00],
    [0xe0, 0x00, 0x40, 0x40, 0x40, 0x40, 0x40, 0x00],
    [0x48, 0x00, 0xfc, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x78, 0x00, 0xfc, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x48, 0x00, 0x84, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x78, 0x00, 0x84, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x00, 0x00, 0xf8, 0x80, 0x80, 0x80, 0xfc, 0x20],
    [0x00, 0x7c, 0x82, 0x82, 0x82, 0x82, 0x7c, 0x00],
    [0x00, 0x40, 0xc0, 0x40, 0x40, 0x40, 0x40, 0x00],
    [0x00, 0x7c, 0x82, 0x02, 0x7c, 0x80, 0xfe, 0x00],
    [0x00, 0xfc, 0x02, 0x3c, 0x02, 0x02, 0xfc, 0x00],
    [0x00, 0x80, 0x80, 0x80, 0x88, 0xfe, 0x08, 0x00],
    [0x00, 0xfe, 0x80, 0xfc, 0x02, 0x02, 0xfc, 0x00],
    [0x00, 0x7c, 0x80, 0xfc, 0x82, 0x82, 0x7c, 0x00],
    [0x00, 0xfe, 0x82, 0x04, 0x04, 0x04, 0x04, 0x00],
    [0x00, 0x7c, 0x82, 0x7c, 0x82, 0x82, 0x7c, 0x00],
    [0x00, 0x7c, 0x82, 0x82, 0x7e, 0x02, 0x7c, 0x00],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub(super) fn recovered_game_font_matches_executable_rows() {
        let m = game_font_glyph('M').expect("M glyph");
        assert_eq!(m.advance, 10);
        assert_eq!(m.rows, [0x00, 0xe7, 0x99, 0x81, 0x81, 0x81, 0x81, 0x00]);

        let e = game_font_glyph('e').expect("e glyph");
        assert_eq!(e.advance, 8);
        assert_eq!(e.rows, [0x00, 0x00, 0xfc, 0x84, 0xfc, 0x80, 0xfc, 0x00]);
    }

    #[test]
    fn reserved_subtitle_palette_makes_reveal_visible_on_black_scene_palette() {
        // Regression: a scene palette with [0,0,0] at the reserved subtitle
        // indices (as real LBM/HNM palettes have) rendered subtitles invisible.
        let mut pal = [[0u8; 3]; 256];
        assert_eq!(pal[SUBTITLE_COLOR_REVEALED as usize], [0, 0, 0]);
        assert_eq!(pal[SUBTITLE_COLOR_REVEAL_EDGE as usize], [0, 0, 0]);
        apply_reserved_subtitle_palette(&mut pal);
        assert_eq!(pal[SUBTITLE_COLOR_REVEALED as usize], SUBTITLE_RGB);
        assert_eq!(pal[SUBTITLE_COLOR_REVEAL_EDGE as usize], SUBTITLE_RGB);
        assert_ne!(SUBTITLE_RGB, [0, 0, 0]);

        // And a full render through such a palette now produces visible pixels.
        let cues = [SubtitleCue {
            tick: 0,
            text: "ME".to_string(),
            active_line_id: None,
        }];
        let mut rgb = vec![0u8; VIEWPORT_W * VIEWPORT_H * 3];
        render_subtitles_rgb(
            &mut rgb,
            &pal,
            &cues,
            2.0 / default_subtitle_reveal_chars_per_second(),
        );
        assert!(
            rgb.chunks_exact(3).any(|px| px == SUBTITLE_RGB),
            "subtitle should be visible after applying reserved palette"
        );
    }

    #[test]
    fn subtitle_space_draws_nothing_not_a_question_mark() {
        // Regression: a space must draw NO glyph (blank), while unknown chars
        // still fall back to '?'. Previously space fell through to '?'.
        assert!(subtitle_draw_glyph(' ').is_none());
        // A known glyph is drawn as itself.
        assert_eq!(subtitle_draw_glyph('M'), game_font_glyph('M'));
        // A genuinely unknown, non-space char falls back to the '?' glyph.
        assert_eq!(subtitle_draw_glyph('\u{2603}'), game_font_glyph('?'));
        assert!(game_font_glyph('?').is_some());
    }

    #[test]
    fn real_spr_bank_parses_with_recovered_frame_table_layout() {
        // The ship-3D nav orb sprite bank. If the ISO has been extracted, the
        // loose .spr files land under _tmp_iso; the exporter now copies them into
        // _tmp_dat/spr/. Confirm the recovered SpriteSlotFrameTable layout parses
        // a real bank: BORXX.SPR is 16 rotation frames with flags 0x0004.
        let candidates = [
            "output/_tmp_iso/BORXX.SPR",
            "output/_tmp_dat/spr/BORXX.SPR",
            "../output/_tmp_iso/BORXX.SPR",
        ];
        let Some(data) = candidates.iter().find_map(|p| std::fs::read(p).ok()) else {
            eprintln!("skipping: BORXX.SPR not available (run the exporter first)");
            return;
        };
        let table = SpriteSlotFrameTable::parse(&data).expect("BORXX.SPR parses");
        assert_eq!(table.flags, 0x0004);
        assert_eq!(table.frames.len(), 16);
        assert_eq!(table.slot_state_flags(), 0x0087); // (0x0004 & 0x0004) | 0x0083
        // Each frame carries a parseable raw/rle sprite header.
        assert!(table.frame(0).is_some());
    }

    #[test]
    fn decode_sprite_bank_indices_decodes_all_orb_frames() {
        let candidates = [
            "output/_tmp_iso/BORXX.SPR",
            "output/_tmp_dat/spr/BORXX.SPR",
            "../output/_tmp_iso/BORXX.SPR",
        ];
        let Some(data) = candidates.iter().find_map(|p| std::fs::read(p).ok()) else {
            eprintln!("skipping: BORXX.SPR not available (run the exporter first)");
            return;
        };
        let frames = decode_sprite_bank_indices(&data).expect("bank decodes");
        assert_eq!(frames.len(), 16);
        assert_eq!((frames[0].width, frames[0].height), (40, 33));
        assert_eq!(frames[0].indices.len(), 40 * 33);
        assert!(frames.iter().all(|f| f.indices.len() == f.width * f.height));
    }

    #[test]
    fn real_spr_rle_frame_decodes_to_width_by_height_pixels() {
        // Decode a real ship-sprite frame end to end. The .spr frame header is
        // [0]=width(stride), [2]=height, [4]=x offset, [6]=y offset, RLE bytes
        // from +8. BORXX.SPR (nav orb) dispatches RLE; frame 0 is 40x33.
        let candidates = [
            "output/_tmp_iso/BORXX.SPR",
            "output/_tmp_dat/spr/BORXX.SPR",
            "../output/_tmp_iso/BORXX.SPR",
        ];
        let Some(data) = candidates.iter().find_map(|p| std::fs::read(p).ok()) else {
            eprintln!("skipping: BORXX.SPR not available (run the exporter first)");
            return;
        };
        let table = SpriteSlotFrameTable::parse(&data).expect("BORXX.SPR parses");
        assert_eq!(table.dispatch_index(), 3); // (0x87 >> 1) & 7 = RLE
        let frame0 = table.frames[0];
        let width = u16::from_le_bytes([frame0[0], frame0[1]]) as usize;
        let height = u16::from_le_bytes([frame0[2], frame0[3]]) as usize;
        assert_eq!((width, height), (40, 33));
        let rle = RleSpriteFrame::parse(frame0).expect("rle frame header");
        assert_eq!(rle.stride, width);
        let pixels = decode_rle_sprite_pixels(rle, height).expect("rle decode");
        assert_eq!(pixels.len(), width * height);
        // A real orb frame, not a flat fill: many distinct palette indices.
        let distinct = pixels
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        assert!(distinct > 16, "orb frame looks flat: {distinct} indices");
    }

    #[test]
    fn sprite_blitter_dispatch_table_matches_binary() {
        // The ship-3D dirty-sprite renderer (0x0299:0x14E1) dispatches through an
        // 8-entry near-pointer table at cs:0x1592 (file 0x4522), indexed by
        // (slot_state >> 1) & 0x0E. Verify our frame-dispatch classification
        // matches the binary: entries 0..=4 are five distinct real blitters
        // (raw/rle transparent+opaque, scaled), entries 5..=7 are shared `ret`
        // (0xC3) stubs -- exactly the Some(0..=4) / None(5..=7) boundary of
        // `ship_3d_sprite_slot_frame_for_dispatch`.
        let candidates = ["re/bin/BLOODPRG.EXE", "../re/bin/BLOODPRG.EXE"];
        let Some(data) = candidates.iter().find_map(|p| std::fs::read(p).ok()) else {
            eprintln!("skipping: BLOODPRG.EXE not available");
            return;
        };
        let seg_base = 0x600 + 0x299 * 16; // segment 0x0299 load address
        let table = seg_base + 0x1592;
        let mut targets = [0u16; 8];
        for (i, t) in targets.iter_mut().enumerate() {
            *t = u16::from_le_bytes([data[table + i * 2], data[table + i * 2 + 1]]);
        }

        // Entries 5,6,7 point at consecutive `ret` bytes -> no-op stubs -> None.
        for idx in 5u8..=7 {
            let off = seg_base + targets[idx as usize] as usize;
            assert_eq!(data[off], 0xC3, "dispatch[{idx}] is not a ret stub");
            assert!(ship_3d_sprite_slot_frame_for_dispatch(&[0u8; 16], idx).is_none());
        }

        // Entries 0..=4 are five distinct real blitters -> classifier yields a
        // frame variant for each.
        let reals = &targets[0..5];
        for (i, &t) in reals.iter().enumerate() {
            assert!(
                data[seg_base + t as usize] != 0xC3,
                "dispatch[{i}] is a stub"
            );
            assert!(ship_3d_sprite_slot_frame_for_dispatch(&[0u8; 16], i as u8).is_some());
        }
        let distinct: std::collections::HashSet<u16> = reals.iter().copied().collect();
        assert_eq!(distinct.len(), 5, "the five real blitters must be distinct");
    }

    #[test]
    fn active_line_clip_bounds_match_dialogue_updater_special_cases() {
        assert_eq!(subtitle_clip_bounds(Some(5)), (SCENE_TOP, SCENE_BOTTOM));
        assert_eq!(subtitle_clip_bounds(Some(0x27)), (SCENE_TOP, SCENE_BOTTOM));
        assert_eq!(subtitle_clip_bounds(Some(0x2b)), (0, VIEWPORT_H));
        assert_eq!(subtitle_clip_bounds(None), (0, VIEWPORT_H));
    }

    #[test]
    fn clipped_text_does_not_draw_outside_active_line_window() {
        let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];
        draw_game_text_indexed_clipped(&mut fb, "M", SUBTITLE_X, 0, SCENE_TOP, SCENE_BOTTOM);
        assert!(fb.iter().all(|sample| *sample == 0));

        draw_game_text_indexed_clipped(
            &mut fb,
            "M",
            SUBTITLE_X,
            SCENE_TOP,
            SCENE_TOP,
            SCENE_BOTTOM,
        );
        assert!(fb.iter().any(|sample| *sample != 0));
    }

    #[test]
    fn recovered_rect_fill_clips_to_viewport_and_active_band() {
        let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];

        fill_rect_indexed_clipped(
            &mut fb,
            7,
            318,
            (SCENE_TOP as isize) - 5,
            8,
            8,
            (0, VIEWPORT_W, SCENE_TOP, SCENE_TOP + 3),
        );

        let filled = fb.iter().filter(|sample| **sample == 7).count();
        assert_eq!(filled, 2 * 3);
        assert_eq!(fb[SCENE_TOP * VIEWPORT_W + 317], 0);
        assert_eq!(fb[SCENE_TOP * VIEWPORT_W + 318], 7);
        assert_eq!(fb[SCENE_TOP * VIEWPORT_W + 319], 7);
        assert_eq!(fb[(SCENE_TOP + 3) * VIEWPORT_W + 318], 0);
    }

    #[test]
    fn recovered_scene_band_fill_only_touches_scene_rows() {
        let mut fb = vec![1u8; VIEWPORT_W * VIEWPORT_H];

        fill_scene_band_indexed(&mut fb, 9);

        assert!(
            fb[..SCENE_TOP * VIEWPORT_W]
                .iter()
                .all(|sample| *sample == 1)
        );
        assert!(
            fb[SCENE_TOP * VIEWPORT_W..SCENE_BOTTOM * VIEWPORT_W]
                .iter()
                .all(|sample| *sample == 9)
        );
        assert!(
            fb[SCENE_BOTTOM * VIEWPORT_W..]
                .iter()
                .all(|sample| *sample == 1)
        );
    }

    #[test]
    fn recovered_framebuffer_copy_uses_one_full_viewport() {
        let len = VIEWPORT_W * VIEWPORT_H;
        let mut src = vec![0u8; len + 1];
        let mut dst = vec![3u8; len + 1];
        src[0] = 11;
        src[len - 1] = 22;
        src[len] = 33;

        copy_framebuffer_full_indexed(&mut dst, &src);

        assert_eq!(dst[0], 11);
        assert_eq!(dst[len - 1], 22);
        assert_eq!(dst[len], 3);
    }

    #[test]
    fn recovered_dirty_rect_copyback_copies_secondary_rows_to_primary() {
        let mut primary = vec![1u8; VIEWPORT_W * VIEWPORT_H];
        let mut secondary = vec![2u8; VIEWPORT_W * VIEWPORT_H];
        secondary[2 * VIEWPORT_W + 3] = 33;
        secondary[3 * VIEWPORT_W + 4] = 44;

        let copied = copy_dirty_rects_secondary_to_primary_indexed(
            &mut primary,
            &secondary,
            &[Ship3dProjectionViewport {
                left: 3,
                right: 6,
                top: 2,
                bottom: 4,
            }],
            true,
        );

        assert_eq!(copied, 6);
        assert_eq!(primary[2 * VIEWPORT_W + 2], 1);
        assert_eq!(primary[2 * VIEWPORT_W + 3], 33);
        assert_eq!(primary[2 * VIEWPORT_W + 4], 2);
        assert_eq!(primary[3 * VIEWPORT_W + 4], 44);
        assert_eq!(primary[4 * VIEWPORT_W + 3], 1);
    }

    #[test]
    fn recovered_dirty_rect_copyback_honors_gate_and_negative_left_sentinel() {
        let mut primary = vec![1u8; VIEWPORT_W * VIEWPORT_H];
        let mut secondary = vec![9u8; VIEWPORT_W * VIEWPORT_H];
        secondary[VIEWPORT_W + 1] = 11;
        secondary[VIEWPORT_W + 4] = 44;
        let rects = [
            Ship3dProjectionViewport {
                left: 1,
                right: 2,
                top: 1,
                bottom: 2,
            },
            Ship3dProjectionViewport {
                left: 0xffff,
                right: 0,
                top: 0,
                bottom: 0,
            },
            Ship3dProjectionViewport {
                left: 4,
                right: 5,
                top: 1,
                bottom: 2,
            },
        ];

        assert_eq!(
            copy_dirty_rects_secondary_to_primary_indexed(&mut primary, &secondary, &rects, false),
            0
        );
        assert!(primary.iter().all(|sample| *sample == 1));

        assert_eq!(
            copy_dirty_rects_secondary_to_primary_indexed(&mut primary, &secondary, &rects, true),
            1
        );
        assert_eq!(primary[VIEWPORT_W + 1], 11);
        assert_eq!(primary[VIEWPORT_W + 4], 1);
    }

    #[test]
    fn recovered_dirty_rect_copyback_clamps_to_viewport() {
        let mut primary = vec![0u8; VIEWPORT_W * VIEWPORT_H];
        let secondary = vec![5u8; VIEWPORT_W * VIEWPORT_H];

        let copied = copy_dirty_rects_secondary_to_primary_indexed(
            &mut primary,
            &secondary,
            &[Ship3dProjectionViewport {
                left: (VIEWPORT_W - 1) as u16,
                right: (VIEWPORT_W + 4) as u16,
                top: (VIEWPORT_H - 1) as u16,
                bottom: (VIEWPORT_H + 4) as u16,
            }],
            true,
        );

        assert_eq!(copied, 1);
        assert_eq!(primary[VIEWPORT_W * VIEWPORT_H - 1], 5);
        assert_eq!(primary[VIEWPORT_W * (VIEWPORT_H - 1) - 1], 0);
    }

    #[test]
    fn recovered_rect_palette_remap_clips_and_uses_source_pixel_as_table_index() {
        let mut table = [0u8; 256];
        for (idx, value) in table.iter_mut().enumerate() {
            *value = 255 - idx as u8;
        }
        let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];
        fb[SCENE_TOP * VIEWPORT_W + 318] = 10;
        fb[SCENE_TOP * VIEWPORT_W + 319] = 11;
        fb[(SCENE_TOP + 1) * VIEWPORT_W + 318] = 12;
        fb[(SCENE_TOP + 1) * VIEWPORT_W + 319] = 13;
        fb[(SCENE_TOP + 2) * VIEWPORT_W + 318] = 14;

        remap_rect_indexed_clipped(
            &mut fb,
            &table,
            318,
            SCENE_TOP as isize,
            8,
            8,
            (0, VIEWPORT_W, SCENE_TOP, SCENE_TOP + 2),
        );

        assert_eq!(fb[SCENE_TOP * VIEWPORT_W + 318], 245);
        assert_eq!(fb[SCENE_TOP * VIEWPORT_W + 319], 244);
        assert_eq!(fb[(SCENE_TOP + 1) * VIEWPORT_W + 318], 243);
        assert_eq!(fb[(SCENE_TOP + 1) * VIEWPORT_W + 319], 242);
        assert_eq!(fb[(SCENE_TOP + 2) * VIEWPORT_W + 318], 14);
    }

    #[test]
    fn recovered_vga_planar_capture_interleaves_four_read_map_planes() {
        let len = VIEWPORT_W * VIEWPORT_H;
        let plane_len = len / 4;
        let mut planes = vec![0u8; len + 1];
        let mut dst = vec![0u8; len + 1];

        planes[0] = 10;
        planes[plane_len] = 20;
        planes[plane_len * 2] = 30;
        planes[plane_len * 3] = 40;
        planes[1] = 11;
        planes[plane_len + 1] = 21;
        planes[plane_len * 2 + 1] = 31;
        planes[plane_len * 3 + 1] = 41;
        planes[len] = 99;
        dst[len] = 7;

        copy_vga_planar_to_linear_indexed(&mut dst, &planes);

        assert_eq!(&dst[..8], &[10, 20, 30, 40, 11, 21, 31, 41]);
        assert_eq!(dst[len], 7);
    }

    #[test]
    fn sprite_slot_frame_table_uses_binary_offset_base_and_dispatch_flags() {
        let first = rle_sprite_frame(2, 1, &[1, 5, 6]);
        let second = rle_sprite_frame(3, 1, &[2, 7, 8, 9]);
        let data = sprite_slot_frame_table(0x0004, &[&first, &second]);

        assert_eq!(u32::from_le_bytes(data[4..8].try_into().unwrap()), 8);
        let table = SpriteSlotFrameTable::parse(&data).expect("sprite slot frame table");

        assert_eq!(table.flags, 0x0004);
        assert_eq!(table.slot_state_flags(), 0x0087);
        assert_eq!(table.dispatch_index(), 3);
        assert_eq!(table.frame_offsets, vec![12, 23]);
        assert_eq!(table.frames.len(), 2);
        assert_eq!(
            u16::from_le_bytes(table.frames[0][0..2].try_into().unwrap()),
            2
        );
        assert!(matches!(
            table.frame(0),
            Some(Ship3dSpriteSlotFrame::Rle(_))
        ));
    }

    #[test]
    fn sprite_slot_frame_table_rejects_offsets_inside_header_table() {
        let mut data = Vec::new();
        data.extend_from_slice(&0x0004u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&rle_sprite_frame(1, 1, &[0, 9]));

        assert_eq!(SpriteSlotFrameTable::parse(&data), None);
    }

    #[test]
    fn compose_ship_3d_scene_overlays_sprite_on_starfield_background() {
        // Uniform "starfield" background distinct from the sprite pixels.
        let background = Ship3dPointCloudRender {
            buffer: vec![0x11u8; VIEWPORT_W * VIEWPORT_H],
            plotted: 0,
        };
        // A real parsed sprite frame (reuses the tested sprite-table path).
        let frame = rle_sprite_frame(2, 1, &[1, 0x5a, 0x5b]);
        let data = sprite_slot_frame_table(0x0004, &[&frame]);
        let table = SpriteSlotFrameTable::parse(&data).expect("sprite slot frame table");
        let dispatch = table.dispatch_index() as u16;

        // One visible+active slot whose flags encode that dispatch index. Pre-set
        // to the projected position/extent (draw 159,100 extent 2x1) so the
        // in-compose projection is a no-op and never raises the DIRTY flag (which
        // shares a bit with the dispatch index).
        let mut slots = [Ship3dObjectSpriteDescriptor {
            flags: 0x0080 | 0x0001 | (dispatch << 1),
            source_width: 2,
            source_height: 1,
            draw_x: 159,
            draw_y: 100,
            extent_width: 2,
            extent_height: 1,
            committed_draw_x: 159,
            committed_draw_y: 100,
            committed_extent_width: 2,
            committed_extent_height: 1,
        }];
        // Project to screen centre: X'/Y' matrix rows zero -> (160,100); depth from
        // z with terms[8]=0x8000 and z=1024 gives depth=1024 -> extent 2x1 (matches
        // the pre-set values, so nothing changes and no DIRTY bit is raised).
        let matrix = Ship3dProjectionMatrix {
            terms: [0, 0, 0, 0, 0, 0, 0, 0, 0x8000],
        };
        let anchors = [Ship3dProjectionPoint {
            x: 0,
            y: 0,
            z: 1024,
        }];
        let origin = Ship3dProjectionOrigin { x: 0, y: 0, z: 0 };

        let composed = compose_ship_3d_scene_indexed(
            &background,
            &mut slots,
            &anchors,
            origin,
            matrix,
            |_| table.frame(0),
            None,
            None,
        );

        assert_eq!(composed.len(), VIEWPORT_W * VIEWPORT_H);
        assert!(
            composed.iter().any(|&p| p == 0x11),
            "starfield background must be preserved outside the sprite"
        );
        assert!(
            composed.iter().any(|&p| p == 0x5a || p == 0x5b),
            "the projected sprite must be composited over the background"
        );
    }

    #[test]
    fn ship_3d_dirty_sprite_pipeline_can_use_parsed_sprite_frame_table() {
        let frame = rle_sprite_frame(2, 1, &[1, 0x5a, 0x5b]);
        let data = sprite_slot_frame_table(0x0004, &[&frame]);
        let table = SpriteSlotFrameTable::parse(&data).expect("sprite slot frame table");
        let y = SCENE_TOP;
        let commands = [ship_3d_sprite_slot_command(
            table.dispatch_index(),
            0,
            10,
            y as u16,
            2,
            1,
        )];
        let dirty_rects = [Ship3dProjectionViewport {
            left: 10,
            right: 12,
            top: y as u16,
            bottom: (y + 1) as u16,
        }];
        let mut primary = vec![0u8; VIEWPORT_W * VIEWPORT_H];
        let mut secondary = vec![0u8; VIEWPORT_W * VIEWPORT_H];

        let result = render_ship_3d_dirty_sprite_commands_indexed(
            &mut primary,
            &mut secondary,
            &commands,
            &dirty_rects,
            true,
            |_| table.frame(0),
            None,
            None,
        );

        assert_eq!(
            result,
            Ship3dDirtySpriteRenderResult {
                rendered_commands: 1,
                copied_pixels: 2,
                ..Ship3dDirtySpriteRenderResult::default()
            }
        );
        assert_eq!(
            &primary[y * VIEWPORT_W + 10..y * VIEWPORT_W + 12],
            &[0x5a, 0x5b]
        );
    }

    #[test]
    fn recovered_raw_transparent_sprite_blit_clips_and_skips_zero_pixels() {
        let mut frame_bytes = vec![0u8; 8];
        frame_bytes[0..2].copy_from_slice(&4u16.to_le_bytes());
        frame_bytes.extend_from_slice(&[0, 1, 2, 99, 3, 0, 4, 99]);
        let frame = RawSpriteFrame::parse(&frame_bytes).expect("raw sprite frame");
        let mut fb = vec![9u8; VIEWPORT_W * VIEWPORT_H];

        blit_raw_transparent_sprite_indexed(
            &mut fb,
            frame,
            SpriteBlitRequest {
                x: 318,
                y: SCENE_TOP as isize - 1,
                width: 3,
                height: 2,
                flip_x: false,
                flip_y: false,
                clip: (0, VIEWPORT_W, SCENE_TOP, SCENE_TOP + 1),
            },
            None,
        );

        assert_eq!(fb[SCENE_TOP * VIEWPORT_W + 317], 9);
        assert_eq!(fb[SCENE_TOP * VIEWPORT_W + 318], 3);
        assert_eq!(fb[SCENE_TOP * VIEWPORT_W + 319], 9);
        assert_eq!(fb[(SCENE_TOP + 1) * VIEWPORT_W + 318], 9);
    }

    #[test]
    fn recovered_raw_transparent_sprite_blit_uses_source_as_remap_mask() {
        let mut frame_bytes = vec![0u8; 8];
        frame_bytes[0..2].copy_from_slice(&3u16.to_le_bytes());
        frame_bytes.extend_from_slice(&[7, 0, 8]);
        let frame = RawSpriteFrame::parse(&frame_bytes).expect("raw sprite frame");
        let mut remap = [0u8; 256];
        for (idx, value) in remap.iter_mut().enumerate() {
            *value = 255 - idx as u8;
        }
        let y = SCENE_TOP;
        let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];
        fb[y * VIEWPORT_W + 10] = 10;
        fb[y * VIEWPORT_W + 11] = 11;
        fb[y * VIEWPORT_W + 12] = 12;

        blit_raw_transparent_sprite_indexed(
            &mut fb,
            frame,
            SpriteBlitRequest {
                x: 10,
                y: y as isize,
                width: 3,
                height: 1,
                flip_x: false,
                flip_y: false,
                clip: (0, VIEWPORT_W, SCENE_TOP, SCENE_BOTTOM),
            },
            Some(&remap),
        );

        assert_eq!(fb[y * VIEWPORT_W + 10], 245);
        assert_eq!(fb[y * VIEWPORT_W + 11], 11);
        assert_eq!(fb[y * VIEWPORT_W + 12], 243);
    }

    #[test]
    fn recovered_raw_sprite_frame_origin_offsets_adjust_destination_rect() {
        let mut frame_bytes = vec![0u8; 8];
        frame_bytes[0..2].copy_from_slice(&1u16.to_le_bytes());
        frame_bytes[4..6].copy_from_slice(&(-2i16).to_le_bytes());
        frame_bytes[6..8].copy_from_slice(&1i16.to_le_bytes());
        frame_bytes.push(5);
        let frame = RawSpriteFrame::parse(&frame_bytes).expect("raw sprite frame");
        let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];

        blit_raw_opaque_sprite_indexed(
            &mut fb,
            frame,
            SpriteBlitRequest {
                x: 12,
                y: SCENE_TOP as isize,
                width: 1,
                height: 1,
                flip_x: false,
                flip_y: false,
                clip: (0, VIEWPORT_W, SCENE_TOP, SCENE_BOTTOM),
            },
        );

        assert_eq!(fb[SCENE_TOP * VIEWPORT_W + 12], 0);
        assert_eq!(fb[(SCENE_TOP + 1) * VIEWPORT_W + 10], 5);
    }

    #[test]
    fn recovered_raw_opaque_sprite_blit_writes_zero_and_honors_flip_flags() {
        let mut frame_bytes = vec![0u8; 8];
        frame_bytes[0..2].copy_from_slice(&3u16.to_le_bytes());
        frame_bytes.extend_from_slice(&[1, 2, 3, 4, 0, 6]);
        let frame = RawSpriteFrame::parse(&frame_bytes).expect("raw sprite frame");
        let y = SCENE_TOP;
        let mut fb = vec![9u8; VIEWPORT_W * VIEWPORT_H];

        blit_raw_opaque_sprite_indexed(
            &mut fb,
            frame,
            SpriteBlitRequest {
                x: 10,
                y: y as isize,
                width: 3,
                height: 2,
                flip_x: true,
                flip_y: true,
                clip: (0, VIEWPORT_W, SCENE_TOP, SCENE_BOTTOM),
            },
        );

        assert_eq!(&fb[y * VIEWPORT_W + 10..y * VIEWPORT_W + 13], &[6, 0, 4]);
        assert_eq!(
            &fb[(y + 1) * VIEWPORT_W + 10..(y + 1) * VIEWPORT_W + 13],
            &[3, 2, 1]
        );
    }

    #[test]
    fn recovered_raw_sprite_horizontal_flip_clipping_matches_binary_edge_case() {
        let mut frame_bytes = vec![0u8; 8];
        frame_bytes[0..2].copy_from_slice(&4u16.to_le_bytes());
        frame_bytes.extend_from_slice(&[1, 2, 3, 4]);
        let frame = RawSpriteFrame::parse(&frame_bytes).expect("raw sprite frame");
        let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];

        blit_raw_opaque_sprite_indexed(
            &mut fb,
            frame,
            SpriteBlitRequest {
                x: 318,
                y: SCENE_TOP as isize,
                width: 4,
                height: 1,
                flip_x: true,
                flip_y: false,
                clip: (0, VIEWPORT_W, SCENE_TOP, SCENE_BOTTOM),
            },
        );

        assert_eq!(fb[SCENE_TOP * VIEWPORT_W + 318], 4);
        assert_eq!(fb[SCENE_TOP * VIEWPORT_W + 319], 3);
    }

    #[test]
    fn recovered_rle_transparent_sprite_blit_decodes_literal_and_fill_runs() {
        let mut frame_bytes = vec![0u8; 8];
        frame_bytes[0..2].copy_from_slice(&5u16.to_le_bytes());
        frame_bytes.extend_from_slice(&[1, 1, 0, 0xfe, 4]);
        let frame = RleSpriteFrame::parse(&frame_bytes).expect("RLE sprite frame");
        let y = SCENE_TOP;
        let mut fb = vec![9u8; VIEWPORT_W * VIEWPORT_H];

        blit_rle_transparent_sprite_indexed(
            &mut fb,
            frame,
            SpriteBlitRequest {
                x: 10,
                y: y as isize,
                width: 5,
                height: 1,
                flip_x: false,
                flip_y: false,
                clip: (0, VIEWPORT_W, SCENE_TOP, SCENE_BOTTOM),
            },
            None,
        );

        assert_eq!(
            &fb[y * VIEWPORT_W + 10..y * VIEWPORT_W + 15],
            &[1, 9, 4, 4, 4]
        );
    }

    #[test]
    fn recovered_rle_transparent_sprite_blit_uses_decoded_nonzero_remap_mask() {
        let mut frame_bytes = vec![0u8; 8];
        frame_bytes[0..2].copy_from_slice(&5u16.to_le_bytes());
        frame_bytes.extend_from_slice(&[1, 1, 0, 0xfe, 4]);
        let frame = RleSpriteFrame::parse(&frame_bytes).expect("RLE sprite frame");
        let mut remap = [0u8; 256];
        for (idx, value) in remap.iter_mut().enumerate() {
            *value = 255 - idx as u8;
        }
        let y = SCENE_TOP;
        let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];
        for idx in 0..5 {
            fb[y * VIEWPORT_W + 10 + idx] = 10 + idx as u8;
        }

        blit_rle_transparent_sprite_indexed(
            &mut fb,
            frame,
            SpriteBlitRequest {
                x: 10,
                y: y as isize,
                width: 5,
                height: 1,
                flip_x: false,
                flip_y: false,
                clip: (0, VIEWPORT_W, SCENE_TOP, SCENE_BOTTOM),
            },
            Some(&remap),
        );

        assert_eq!(
            &fb[y * VIEWPORT_W + 10..y * VIEWPORT_W + 15],
            &[245, 11, 243, 242, 241]
        );
    }

    #[test]
    fn recovered_rle_opaque_sprite_blit_writes_zero_fill_and_copy_runs() {
        let mut frame_bytes = vec![0u8; 8];
        frame_bytes[0..2].copy_from_slice(&5u16.to_le_bytes());
        frame_bytes.extend_from_slice(&[0xfe, 0, 1, 5, 6]);
        let frame = RleSpriteFrame::parse(&frame_bytes).expect("RLE sprite frame");
        let y = SCENE_TOP;
        let mut fb = vec![9u8; VIEWPORT_W * VIEWPORT_H];

        blit_rle_opaque_sprite_indexed(
            &mut fb,
            frame,
            SpriteBlitRequest {
                x: 10,
                y: y as isize,
                width: 5,
                height: 1,
                flip_x: false,
                flip_y: false,
                clip: (0, VIEWPORT_W, SCENE_TOP, SCENE_BOTTOM),
            },
        );

        assert_eq!(
            &fb[y * VIEWPORT_W + 10..y * VIEWPORT_W + 15],
            &[0, 0, 0, 5, 6]
        );
    }

    #[test]
    fn recovered_rle_sprite_blit_reuses_raw_flip_mapping_after_decode() {
        let mut frame_bytes = vec![0u8; 8];
        frame_bytes[0..2].copy_from_slice(&3u16.to_le_bytes());
        frame_bytes.extend_from_slice(&[2, 1, 2, 3, 2, 4, 5, 6]);
        let frame = RleSpriteFrame::parse(&frame_bytes).expect("RLE sprite frame");
        let y = SCENE_TOP;
        let mut fb = vec![9u8; VIEWPORT_W * VIEWPORT_H];

        blit_rle_opaque_sprite_indexed(
            &mut fb,
            frame,
            SpriteBlitRequest {
                x: 10,
                y: y as isize,
                width: 3,
                height: 2,
                flip_x: true,
                flip_y: true,
                clip: (0, VIEWPORT_W, SCENE_TOP, SCENE_BOTTOM),
            },
        );

        assert_eq!(&fb[y * VIEWPORT_W + 10..y * VIEWPORT_W + 13], &[6, 5, 4]);
        assert_eq!(
            &fb[(y + 1) * VIEWPORT_W + 10..(y + 1) * VIEWPORT_W + 13],
            &[3, 2, 1]
        );
    }

    #[test]
    fn recovered_rle_sprite_blit_ignores_incomplete_encoded_rows() {
        let mut frame_bytes = vec![0u8; 8];
        frame_bytes[0..2].copy_from_slice(&3u16.to_le_bytes());
        frame_bytes.extend_from_slice(&[2, 1]);
        let frame = RleSpriteFrame::parse(&frame_bytes).expect("RLE sprite frame");
        let y = SCENE_TOP;
        let mut fb = vec![9u8; VIEWPORT_W * VIEWPORT_H];

        blit_rle_opaque_sprite_indexed(
            &mut fb,
            frame,
            SpriteBlitRequest {
                x: 10,
                y: y as isize,
                width: 3,
                height: 1,
                flip_x: false,
                flip_y: false,
                clip: (0, VIEWPORT_W, SCENE_TOP, SCENE_BOTTOM),
            },
        );

        assert_eq!(&fb[y * VIEWPORT_W + 10..y * VIEWPORT_W + 13], &[9, 9, 9]);
    }

    #[test]
    fn recovered_scaled_transparent_sprite_blit_uses_16_16_nearest_sampling() {
        let mut frame_bytes = vec![0u8; 8];
        frame_bytes[0..2].copy_from_slice(&2u16.to_le_bytes());
        frame_bytes[2..4].copy_from_slice(&2u16.to_le_bytes());
        frame_bytes.extend_from_slice(&[1, 0, 2, 3]);
        let frame = ScaledSpriteFrame::parse(&frame_bytes).expect("scaled sprite frame");
        let y = SCENE_TOP;
        let mut fb = vec![9u8; VIEWPORT_W * VIEWPORT_H];

        blit_scaled_transparent_sprite_indexed(
            &mut fb,
            frame,
            SpriteBlitRequest {
                x: 10,
                y: y as isize,
                width: 4,
                height: 4,
                flip_x: true,
                flip_y: true,
                clip: (0, VIEWPORT_W, SCENE_TOP, SCENE_BOTTOM),
            },
        );

        assert_eq!(&fb[y * VIEWPORT_W + 10..y * VIEWPORT_W + 14], &[1, 1, 9, 9]);
        assert_eq!(
            &fb[(y + 1) * VIEWPORT_W + 10..(y + 1) * VIEWPORT_W + 14],
            &[1, 1, 9, 9]
        );
        assert_eq!(
            &fb[(y + 2) * VIEWPORT_W + 10..(y + 2) * VIEWPORT_W + 14],
            &[2, 2, 3, 3]
        );
        assert_eq!(
            &fb[(y + 3) * VIEWPORT_W + 10..(y + 3) * VIEWPORT_W + 14],
            &[2, 2, 3, 3]
        );
    }

    #[test]
    fn recovered_scaled_transparent_sprite_blit_downsamples_with_integer_source_steps() {
        let mut frame_bytes = vec![0u8; 8];
        frame_bytes[0..2].copy_from_slice(&4u16.to_le_bytes());
        frame_bytes[2..4].copy_from_slice(&1u16.to_le_bytes());
        frame_bytes.extend_from_slice(&[1, 2, 3, 4]);
        let frame = ScaledSpriteFrame::parse(&frame_bytes).expect("scaled sprite frame");
        let y = SCENE_TOP;
        let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];

        blit_scaled_transparent_sprite_indexed(
            &mut fb,
            frame,
            SpriteBlitRequest {
                x: 10,
                y: y as isize,
                width: 2,
                height: 1,
                flip_x: false,
                flip_y: false,
                clip: (0, VIEWPORT_W, SCENE_TOP, SCENE_BOTTOM),
            },
        );

        assert_eq!(&fb[y * VIEWPORT_W + 10..y * VIEWPORT_W + 12], &[1, 3]);
    }

    #[test]
    fn recovered_scaled_transparent_sprite_blit_clipping_advances_accumulators() {
        let mut frame_bytes = vec![0u8; 8];
        frame_bytes[0..2].copy_from_slice(&4u16.to_le_bytes());
        frame_bytes[2..4].copy_from_slice(&4u16.to_le_bytes());
        frame_bytes.extend_from_slice(&[
            1, 2, 3, 4, //
            5, 6, 7, 8, //
            9, 10, 11, 12, //
            13, 14, 15, 16,
        ]);
        let frame = ScaledSpriteFrame::parse(&frame_bytes).expect("scaled sprite frame");
        let y = SCENE_TOP;
        let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];

        blit_scaled_transparent_sprite_indexed(
            &mut fb,
            frame,
            SpriteBlitRequest {
                x: 8,
                y: y as isize - 2,
                width: 8,
                height: 8,
                flip_x: false,
                flip_y: false,
                clip: (10, 14, SCENE_TOP, SCENE_TOP + 2),
            },
        );

        assert_eq!(&fb[y * VIEWPORT_W + 10..y * VIEWPORT_W + 14], &[6, 6, 7, 7]);
        assert_eq!(
            &fb[(y + 1) * VIEWPORT_W + 10..(y + 1) * VIEWPORT_W + 14],
            &[6, 6, 7, 7]
        );
        assert_eq!(fb[(y + 2) * VIEWPORT_W + 10], 0);
    }

    #[test]
    fn recovered_scaled_transparent_sprite_blit_rejects_zero_destination_extent() {
        let mut frame_bytes = vec![0u8; 8];
        frame_bytes[0..2].copy_from_slice(&1u16.to_le_bytes());
        frame_bytes[2..4].copy_from_slice(&1u16.to_le_bytes());
        frame_bytes.push(7);
        let frame = ScaledSpriteFrame::parse(&frame_bytes).expect("scaled sprite frame");
        let y = SCENE_TOP;
        let mut fb = vec![9u8; VIEWPORT_W * VIEWPORT_H];

        blit_scaled_transparent_sprite_indexed(
            &mut fb,
            frame,
            SpriteBlitRequest {
                x: 10,
                y: y as isize,
                width: 0,
                height: 1,
                flip_x: false,
                flip_y: false,
                clip: (0, VIEWPORT_W, SCENE_TOP, SCENE_BOTTOM),
            },
        );

        assert_eq!(fb[y * VIEWPORT_W + 10], 9);
    }

    #[test]
    fn ship_3d_sprite_slot_command_uses_secondary_destination_remap_table() {
        let mut frame_bytes = vec![0u8; 8];
        frame_bytes[0..2].copy_from_slice(&3u16.to_le_bytes());
        frame_bytes.extend_from_slice(&[7, 0, 8]);
        let mut remap_5f11 = [0u8; 256];
        let mut remap_6011 = [0u8; 256];
        for (idx, value) in remap_5f11.iter_mut().enumerate() {
            *value = idx as u8;
        }
        for (idx, value) in remap_6011.iter_mut().enumerate() {
            *value = 255 - idx as u8;
        }
        let y = SCENE_TOP;
        let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];
        fb[y * VIEWPORT_W + 10] = 10;
        fb[y * VIEWPORT_W + 11] = 11;
        fb[y * VIEWPORT_W + 12] = 12;

        assert!(blit_ship_3d_sprite_slot_command_indexed(
            &mut fb,
            ship_3d_sprite_slot_command(0, 2, 10, y as u16, 3, 1),
            Ship3dSpriteSlotFrame::Raw(&frame_bytes),
            Some(&remap_5f11),
            Some(&remap_6011),
        ));

        assert_eq!(
            &fb[y * VIEWPORT_W + 10..y * VIEWPORT_W + 13],
            &[245, 11, 243]
        );
    }

    #[test]
    fn ship_3d_sprite_slot_command_dispatches_rle_opaque_and_scaled_modes() {
        let mut rle_frame = vec![0u8; 8];
        rle_frame[0..2].copy_from_slice(&3u16.to_le_bytes());
        rle_frame.extend_from_slice(&[0xfe, 4]);
        let y = SCENE_TOP;
        let mut fb = vec![9u8; VIEWPORT_W * VIEWPORT_H];

        assert!(blit_ship_3d_sprite_slot_command_indexed(
            &mut fb,
            ship_3d_sprite_slot_command(3, 0, 10, y as u16, 3, 1),
            Ship3dSpriteSlotFrame::Rle(&rle_frame),
            None,
            None,
        ));
        assert_eq!(&fb[y * VIEWPORT_W + 10..y * VIEWPORT_W + 13], &[4, 4, 4]);

        let mut scaled_frame = vec![0u8; 8];
        scaled_frame[0..2].copy_from_slice(&2u16.to_le_bytes());
        scaled_frame[2..4].copy_from_slice(&1u16.to_le_bytes());
        scaled_frame.extend_from_slice(&[5, 6]);

        assert!(blit_ship_3d_sprite_slot_command_indexed(
            &mut fb,
            ship_3d_sprite_slot_command(4, 0, 20, y as u16, 4, 1),
            Ship3dSpriteSlotFrame::Scaled(&scaled_frame),
            None,
            None,
        ));
        assert_eq!(&fb[y * VIEWPORT_W + 20..y * VIEWPORT_W + 24], &[5, 5, 6, 6]);
    }

    #[test]
    fn ship_3d_sprite_slot_command_keeps_noop_modes_and_rejects_frame_mismatch() {
        let mut fb = vec![7u8; VIEWPORT_W * VIEWPORT_H];
        let before = fb.clone();

        assert!(blit_ship_3d_sprite_slot_command_indexed(
            &mut fb,
            ship_3d_sprite_slot_command(5, 0, 10, SCENE_TOP as u16, 1, 1),
            Ship3dSpriteSlotFrame::Raw(&[]),
            None,
            None,
        ));
        assert_eq!(fb, before);

        assert!(!blit_ship_3d_sprite_slot_command_indexed(
            &mut fb,
            ship_3d_sprite_slot_command(2, 0, 10, SCENE_TOP as u16, 1, 1),
            Ship3dSpriteSlotFrame::Rle(&[]),
            None,
            None,
        ));
        assert_eq!(fb, before);
    }

    #[test]
    fn ship_3d_dirty_sprite_pipeline_renders_commands_in_order_and_copybacks() {
        let mut frame_a = vec![0u8; 8];
        frame_a[0..2].copy_from_slice(&2u16.to_le_bytes());
        frame_a.extend_from_slice(&[1, 2]);
        let mut frame_b = vec![0u8; 8];
        frame_b[0..2].copy_from_slice(&2u16.to_le_bytes());
        frame_b.extend_from_slice(&[3, 4]);
        let y = SCENE_TOP;
        let commands = [
            ship_3d_sprite_slot_command_for_slot(0, 2, 0, 10, y as u16, 2, 1),
            ship_3d_sprite_slot_command_for_slot(1, 2, 0, 11, y as u16, 2, 1),
        ];
        let dirty_rects = [Ship3dProjectionViewport {
            left: 10,
            right: 13,
            top: y as u16,
            bottom: (y + 1) as u16,
        }];
        let mut primary = vec![9u8; VIEWPORT_W * VIEWPORT_H];
        let mut secondary = vec![0u8; VIEWPORT_W * VIEWPORT_H];

        let result = render_ship_3d_dirty_sprite_commands_indexed(
            &mut primary,
            &mut secondary,
            &commands,
            &dirty_rects,
            true,
            |command| match command.slot_index {
                0 => Some(Ship3dSpriteSlotFrame::Raw(&frame_a)),
                1 => Some(Ship3dSpriteSlotFrame::Raw(&frame_b)),
                _ => None,
            },
            None,
            None,
        );

        assert_eq!(
            result,
            Ship3dDirtySpriteRenderResult {
                rendered_commands: 2,
                copied_pixels: 3,
                ..Ship3dDirtySpriteRenderResult::default()
            }
        );
        assert_eq!(
            &secondary[y * VIEWPORT_W + 10..y * VIEWPORT_W + 13],
            &[1, 3, 4]
        );
        assert_eq!(
            &primary[y * VIEWPORT_W + 10..y * VIEWPORT_W + 13],
            &[1, 3, 4]
        );
        assert_eq!(primary[y * VIEWPORT_W + 9], 9);
    }

    #[test]
    fn ship_3d_dirty_sprite_pipeline_reports_missing_and_rejected_frames() {
        let y = SCENE_TOP;
        let commands = [
            ship_3d_sprite_slot_command_for_slot(0, 2, 0, 10, y as u16, 1, 1),
            ship_3d_sprite_slot_command_for_slot(1, 2, 0, 11, y as u16, 1, 1),
        ];
        let mut primary = vec![7u8; VIEWPORT_W * VIEWPORT_H];
        let mut secondary = vec![3u8; VIEWPORT_W * VIEWPORT_H];
        let before_primary = primary.clone();
        let before_secondary = secondary.clone();

        let result = render_ship_3d_dirty_sprite_commands_indexed(
            &mut primary,
            &mut secondary,
            &commands,
            &[],
            false,
            |command| (command.slot_index == 1).then_some(Ship3dSpriteSlotFrame::Rle(&[])),
            None,
            None,
        );

        assert_eq!(
            result,
            Ship3dDirtySpriteRenderResult {
                missing_frames: 1,
                rejected_commands: 1,
                ..Ship3dDirtySpriteRenderResult::default()
            }
        );
        assert_eq!(primary, before_primary);
        assert_eq!(secondary, before_secondary);
    }

    #[test]
    fn ship_3d_dirty_sprite_pipeline_can_render_without_copyback() {
        let mut frame = vec![0u8; 8];
        frame[0..2].copy_from_slice(&1u16.to_le_bytes());
        frame.push(0x4d);
        let y = SCENE_TOP;
        let commands = [ship_3d_sprite_slot_command(2, 0, 10, y as u16, 1, 1)];
        let dirty_rects = [Ship3dProjectionViewport {
            left: 10,
            right: 11,
            top: y as u16,
            bottom: (y + 1) as u16,
        }];
        let mut primary = vec![0u8; VIEWPORT_W * VIEWPORT_H];
        let mut secondary = vec![0u8; VIEWPORT_W * VIEWPORT_H];

        let result = render_ship_3d_dirty_sprite_commands_indexed(
            &mut primary,
            &mut secondary,
            &commands,
            &dirty_rects,
            false,
            |_| Some(Ship3dSpriteSlotFrame::Raw(&frame)),
            None,
            None,
        );

        assert_eq!(
            result,
            Ship3dDirtySpriteRenderResult {
                rendered_commands: 1,
                ..Ship3dDirtySpriteRenderResult::default()
            }
        );
        assert_eq!(secondary[y * VIEWPORT_W + 10], 0x4d);
        assert_eq!(primary[y * VIEWPORT_W + 10], 0);
    }

    #[test]
    fn subtitles_use_binary_reveal_palette_indices() {
        let cues = [SubtitleCue {
            tick: 0,
            text: "ME".to_string(),
            active_line_id: None,
        }];
        let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];

        render_subtitles_indexed(
            &mut fb,
            &cues,
            2.0 / default_subtitle_reveal_chars_per_second(),
        );

        assert!(fb.iter().any(|sample| *sample == SUBTITLE_COLOR_REVEALED));
        assert!(
            fb.iter()
                .any(|sample| *sample == SUBTITLE_COLOR_REVEAL_EDGE)
        );
    }

    #[test]
    fn rgb_subtitles_map_binary_indices_through_palette() {
        let cues = [SubtitleCue {
            tick: 0,
            text: "ME".to_string(),
            active_line_id: None,
        }];
        let mut palette = [[0u8; 3]; 256];
        palette[SUBTITLE_COLOR_REVEALED as usize] = [1, 2, 3];
        palette[SUBTITLE_COLOR_REVEAL_EDGE as usize] = [4, 5, 6];
        let mut rgb = vec![0u8; VIEWPORT_W * VIEWPORT_H * 3];

        render_subtitles_rgb(
            &mut rgb,
            &palette,
            &cues,
            2.0 / default_subtitle_reveal_chars_per_second(),
        );

        assert!(rgb.chunks_exact(3).any(|pixel| pixel == [1, 2, 3]));
        assert!(rgb.chunks_exact(3).any(|pixel| pixel == [4, 5, 6]));
    }

    fn ship_3d_sprite_slot_command(
        dispatch_index: u8,
        destination_remap_mode: u8,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    ) -> Ship3dSpriteSlotRenderCommand {
        ship_3d_sprite_slot_command_for_slot(
            0,
            dispatch_index,
            destination_remap_mode,
            x,
            y,
            width,
            height,
        )
    }

    fn ship_3d_sprite_slot_command_for_slot(
        slot_index: usize,
        dispatch_index: u8,
        destination_remap_mode: u8,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    ) -> Ship3dSpriteSlotRenderCommand {
        Ship3dSpriteSlotRenderCommand {
            slot_index,
            dispatch_index,
            destination_remap_mode,
            flip_x: false,
            flip_y: false,
            slot_rect: Ship3dProjectionViewport {
                left: x,
                right: x.wrapping_add(width),
                top: y,
                bottom: y.wrapping_add(height),
            },
            dirty_rect: Ship3dProjectionViewport {
                left: 0,
                right: VIEWPORT_W as u16,
                top: 0,
                bottom: VIEWPORT_H as u16,
            },
        }
    }

    fn rle_sprite_frame(stride: u16, height: u16, encoded: &[u8]) -> Vec<u8> {
        let mut frame = Vec::new();
        frame.extend_from_slice(&stride.to_le_bytes());
        frame.extend_from_slice(&height.to_le_bytes());
        frame.extend_from_slice(&0i16.to_le_bytes());
        frame.extend_from_slice(&0i16.to_le_bytes());
        frame.extend_from_slice(encoded);
        frame
    }

    fn sprite_slot_frame_table(flags: u16, frames: &[&[u8]]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&flags.to_le_bytes());
        data.extend_from_slice(&(frames.len() as u16).to_le_bytes());
        data.resize(4 + frames.len() * 4, 0);
        for (idx, frame) in frames.iter().enumerate() {
            let offset = data.len() - 4;
            data[4 + idx * 4..8 + idx * 4].copy_from_slice(&(offset as u32).to_le_bytes());
            data.extend_from_slice(frame);
        }
        data
    }
}
