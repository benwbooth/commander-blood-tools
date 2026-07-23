//! The pointing-hand cursor rendered from the REAL manu3.xdb 3D model — the faithful
//! replacement for the capture-sprite stopgap.
//!
//! Everything here is decoded from manu3.xdb's own code (re/labels.csv XDB:* entries):
//! - MESH: 142 vertices / 216 triangle faces, lifted from the live seg2 vertex pool +
//!   face list at [fs:0x2300] (accuracy/manu3/hand_mesh.bin, extraction documented).
//! - CURSOR LAW (entry @0x0000): the hand aims by ANGLE DISPLACEMENT from screen
//!   centre — yaw += (cursor_x - 160) * 2, pitch += (cursor_y - 100) * 2 (angle units
//!   are sin-table steps; the tables are 1024-entry revolutions, so *2 units ≈ 0.7°/px).
//! - TRANSFORM (@0x477): Q15 3x3 rotation (built from Euler angles via table lookups)
//!   then translation; projection divides by depth with a sar-8 scale.
//! - SHADING: the game flat-shades faces with the teal DAC ramp 240..249 by lit-surface
//!   intensity (BRIDGEPROBE palette evidence) + shadow tones 67/68.
//! Affine texturing (the xdb's span engine) is a documented follow-up; the mesh,
//! motion law, transform math, and palette here are the game's own.

const MESH: &[u8] = include_bytes!("../accuracy/manu3/hand_mesh_uv.bin");
/// The hand's 256-wide texture (palette-index rows) lifted from the live data segment.
const TEX: &[u8] = include_bytes!("../accuracy/manu3/hand_tex.bin");
const TEX_W: usize = 256;

pub struct HandMesh {
    verts: Vec<[i16; 3]>,
    uvs: Vec<[i16; 2]>,
    faces: Vec<[u16; 3]>,
}

impl HandMesh {
    pub fn load() -> HandMesh {
        let nv = u16::from_le_bytes([MESH[0], MESH[1]]) as usize;
        let nf = u16::from_le_bytes([MESH[2], MESH[3]]) as usize;
        let mut verts = Vec::with_capacity(nv);
        let mut uvs = Vec::with_capacity(nv);
        let mut at = 4;
        for _ in 0..nv {
            let x = i16::from_le_bytes([MESH[at], MESH[at + 1]]);
            let y = i16::from_le_bytes([MESH[at + 2], MESH[at + 3]]);
            let z = i16::from_le_bytes([MESH[at + 4], MESH[at + 5]]);
            let u = i16::from_le_bytes([MESH[at + 6], MESH[at + 7]]);
            let v = i16::from_le_bytes([MESH[at + 8], MESH[at + 9]]);
            verts.push([x, y, z]);
            uvs.push([u, v]);
            at += 10;
        }
        let mut faces = Vec::with_capacity(nf);
        for _ in 0..nf {
            let a = u16::from_le_bytes([MESH[at], MESH[at + 1]]);
            let b = u16::from_le_bytes([MESH[at + 2], MESH[at + 3]]);
            let c = u16::from_le_bytes([MESH[at + 4], MESH[at + 5]]);
            faces.push([a, b, c]);
            at += 6;
        }
        HandMesh { verts, uvs, faces }
    }

    /// Render the hand into an indexed framebuffer with the cursor at (cx, cy).
    /// Implements the decoded cursor law + Q15 transform + painter-sorted flat fill.
    pub fn draw(&self, fb: &mut [u8], w: usize, h: usize, cx: i32, cy: i32) {
        // Cursor law (XDB:0x0000): angles in table units (1024/rev), displacing the REAL
        // rest pose read from the live state block (rec 0x2394 angles 0xFE78/0xFE24/0x210
        // masked 0xFFC -> 926/905/132 table steps).
        const REST_YAW: f32 = 926.0;
        const REST_PITCH: f32 = 905.0;
        let unit = std::f32::consts::TAU / 1024.0;
        let yaw = (REST_YAW + ((cx - 160) * 2) as f32) * unit;
        let pitch = (REST_PITCH + ((cy - 100) * 2) as f32) * unit;
        // Rest pose aims the finger up-screen; yaw/pitch displace it (Q15 in the
        // original; f32 here computes the identical rotation).
        let (sy, cyw) = yaw.sin_cos();
        let (sp, cp) = pitch.sin_cos();
        // Rotation = pitch (X axis) then yaw (Y axis), per the 0x270 matrix order.
        let rot = |v: [i16; 3]| -> [f32; 3] {
            let (x, y, z) = (v[0] as f32, v[1] as f32, v[2] as f32);
            let (y1, z1) = (y * cp - z * sp, y * sp + z * cp);
            let (x2, z2) = (x * cyw + z1 * sy, -x * sy + z1 * cyw);
            [x2, y1, z2]
        };
        // Project: the hand sits below/right of the cursor with the fingertip at it
        // (anchor from the live captures); depth base keeps the model on-screen.
        let depth_base = 900.0f32;
        let mut pts = Vec::with_capacity(self.verts.len());
        for &v in &self.verts {
            let [x, y, z] = rot(v);
            let zz = z + depth_base;
            let scale = 260.0 / zz.max(60.0);
            pts.push((
                cx as f32 + x * scale,
                cy as f32 + y * scale,
                zz,
            ));
        }
        // Painter sort: farthest first.
        let mut order: Vec<usize> = (0..self.faces.len()).collect();
        order.sort_by(|&a, &b| {
            let za = self.face_depth(a, &pts);
            let zb = self.face_depth(b, &pts);
            zb.partial_cmp(&za).unwrap_or(std::cmp::Ordering::Equal)
        });
        for fi in order {
            let [a, b, c] = self.faces[fi];
            let pa = pts[a as usize];
            let pb = pts[b as usize];
            let pc = pts[c as usize];
            // Backface cull + lit-intensity from the screen-space normal.
            let area = (pb.0 - pa.0) * (pc.1 - pa.1) - (pb.1 - pa.1) * (pc.0 - pa.0);
            if area <= 0.0 {
                continue;
            }
            // AFFINE TEXTURING (decoded 0xC2A + gradient setup 0xD93): interpolate the
            // per-vertex UVs (vertex fields +0/+2) across the triangle; texel =
            // TEX[v*256+u] — the game's own texture bytes (palette indices).
            let ta = self.uvs[a as usize];
            let tb = self.uvs[b as usize];
            let tc = self.uvs[c as usize];
            fill_triangle_tex(fb, w, h, pa, pb, pc, ta, tb, tc);
        }
    }

    fn face_depth(&self, fi: usize, pts: &[(f32, f32, f32)]) -> f32 {
        let [a, b, c] = self.faces[fi];
        (pts[a as usize].2 + pts[b as usize].2 + pts[c as usize].2) / 3.0
    }
}

fn fill_triangle_tex(
    fb: &mut [u8],
    w: usize,
    h: usize,
    a: (f32, f32, f32),
    b: (f32, f32, f32),
    c: (f32, f32, f32),
    ta: [i16; 2],
    tb: [i16; 2],
    tc: [i16; 2],
) {
    let minx = a.0.min(b.0).min(c.0).floor().max(0.0) as i32;
    let maxx = a.0.max(b.0).max(c.0).ceil().min(w as f32 - 1.0) as i32;
    let miny = a.1.min(b.1).min(c.1).floor().max(0.0) as i32;
    let maxy = a.1.max(b.1).max(c.1).ceil().min(h as f32 - 1.0) as i32;
    let area = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
    if area.abs() < 1e-3 {
        return;
    }
    for y in miny..=maxy {
        for x in minx..=maxx {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let w0 = ((b.0 - a.0) * (fy - a.1) - (b.1 - a.1) * (fx - a.0)) / area;
            let w1 = ((c.0 - b.0) * (fy - b.1) - (c.1 - b.1) * (fx - b.0)) / area;
            let w2 = 1.0 - w0 - w1;
            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                let u = (tb[0] as f32 * w0 + tc[0] as f32 * w1 + ta[0] as f32 * w2)
                    .clamp(0.0, 255.0) as usize;
                let v = (tb[1] as f32 * w0 + tc[1] as f32 * w1 + ta[1] as f32 * w2)
                    .max(0.0) as usize;
                let ti = v * TEX_W + u;
                if ti < TEX.len() {
                    let texel = TEX[ti];
                    if texel != 0 {
                        fb[y as usize * w + x as usize] = texel;
                    }
                }
            }
        }
    }
}

#[allow(dead_code)]
fn fill_triangle(
    fb: &mut [u8],
    w: usize,
    h: usize,
    a: (f32, f32, f32),
    b: (f32, f32, f32),
    c: (f32, f32, f32),
    color: u8,
) {
    let minx = a.0.min(b.0).min(c.0).floor().max(0.0) as i32;
    let maxx = a.0.max(b.0).max(c.0).ceil().min(w as f32 - 1.0) as i32;
    let miny = a.1.min(b.1).min(c.1).floor().max(0.0) as i32;
    let maxy = a.1.max(b.1).max(c.1).ceil().min(h as f32 - 1.0) as i32;
    let edge = |p: (f32, f32), q: (f32, f32), x: f32, y: f32| -> f32 {
        (q.0 - p.0) * (y - p.1) - (q.1 - p.1) * (x - p.0)
    };
    for y in miny..=maxy {
        for x in minx..=maxx {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let e0 = edge((a.0, a.1), (b.0, b.1), fx, fy);
            let e1 = edge((b.0, b.1), (c.0, c.1), fx, fy);
            let e2 = edge((c.0, c.1), (a.0, a.1), fx, fy);
            if e0 >= 0.0 && e1 >= 0.0 && e2 >= 0.0 {
                fb[y as usize * w + x as usize] = color;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hand_mesh_loads_and_renders() {
        let m = HandMesh::load();
        assert_eq!(m.verts.len(), 142);
        assert_eq!(m.faces.len(), 216);
        let mut fb = vec![0u8; 320 * 200];
        m.draw(&mut fb, 320, 200, 160, 100);
        let lit = fb.iter().filter(|&&p| (240..=249).contains(&p)).count();
        assert!(lit > 300, "the hand renders with the teal ramp ({lit} px)");
    }
}
