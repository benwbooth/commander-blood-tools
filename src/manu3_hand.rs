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

const MESH: &[u8] = include_bytes!("../accuracy/manu3/hand_skeletal.bin");
/// The hand's 256-wide texture (palette-index rows) lifted from the live data segment.
const TEX: &[u8] = include_bytes!("../accuracy/manu3/hand_tex.bin");
const TEX_W: usize = 256;
/// The game's own sin/cos tables (ds:0x26, 1024 entries x {cos:i16, sin:i16}, Q14).
const TRIG: &[u8] = include_bytes!("../accuracy/manu3/trig_tables.bin");

#[inline]
fn tcos(off: i32) -> i32 {
    let o = (off & 0xFFC) as usize;
    i16::from_le_bytes([TRIG[o], TRIG[o + 1]]) as i32
}
#[inline]
fn tsin(off: i32) -> i32 {
    let o = (off & 0xFFC) as usize;
    i16::from_le_bytes([TRIG[o + 2], TRIG[o + 3]]) as i32
}

/// EXACT transcription of the manu3 matrix build (xdb 0x270..0x3DE): three Euler
/// angles as raw byte-offsets (4/step) -> the 9 Q15-ish cells, composed via the
/// angle-sum identity reads the original performs. Cell order matches the
/// transform: out.x = (m00 x + m01 y + m02 z)>>15 etc.
fn build_matrix(a1: i32, a2: i32, a3: i32) -> [i32; 9] {
    let d = a2 + a3;
    let m21 = -(2 * tsin(a1));
    let mut eax = tcos(a1 - d) - tcos(a1 + d);
    let mut ebp = tsin(a1 - d) + tsin(a1 + d);
    eax >>= 1;
    ebp >>= 1;
    eax += tsin(d);
    ebp += tcos(d);
    let mut m10 = eax;
    let mut m02 = -eax;
    let mut m00 = ebp;
    let mut m12 = ebp;
    // second block: d2 = a2 - a3
    let d2 = a2 - a3;
    let mut eax2 = tcos(a1 - d2) - tcos(a1 + d2);
    let mut ebp2 = tsin(a1 - d2) + tsin(a1 + d2);
    eax2 >>= 1;
    ebp2 >>= 1;
    let ecx = tsin(d2) - eax2;
    let edx = tcos(d2) - ebp2;
    m10 -= ecx;
    m02 -= ecx;
    m00 += edx;
    m12 -= edx;
    // m11/m01 from a3±a1
    let m11 = tcos(a3 + a1) + tcos(a3 - a1);
    let m01 = -(tsin(a3 + a1) + tsin(a3 - a1));
    // m22/m20 from a2±a1
    let m22 = tcos(a2 + a1) + tcos(a2 - a1);
    let m20 = tsin(a2 + a1) + tsin(a2 - a1);
    [m00, m01, m02, m10, m11, m12, m20, m21, m22]
}

pub struct HandMesh {
    /// 16 skeleton segments: {vert_count, T (Q8 dwords), angles (raw table offsets)}.
    segs: Vec<(u16, [i32; 3], [i16; 3])>,
    verts: Vec<[i16; 3]>,
    uvs: Vec<[i16; 2]>,
    /// index >= verts.len(): alias — resolves to `alias_src`.
    alias_src: Vec<u16>,
    faces: Vec<[u16; 3]>,
}

impl HandMesh {
    pub fn load() -> HandMesh {
        let rd16 = |at: usize| u16::from_le_bytes([MESH[at], MESH[at + 1]]);
        let rdi16 = |at: usize| i16::from_le_bytes([MESH[at], MESH[at + 1]]);
        let rdi32 = |at: usize| {
            i32::from_le_bytes([MESH[at], MESH[at + 1], MESH[at + 2], MESH[at + 3]])
        };
        let ns = rd16(0) as usize;
        let nv = rd16(2) as usize;
        let na = rd16(4) as usize;
        let nf = rd16(6) as usize;
        let mut at = 8;
        let mut segs = Vec::with_capacity(ns);
        for _ in 0..ns {
            let cnt = rd16(at);
            let t = [rdi32(at + 2), rdi32(at + 6), rdi32(at + 10)];
            let a = [rdi16(at + 14), rdi16(at + 16), rdi16(at + 18)];
            segs.push((cnt, t, a));
            at += 20;
        }
        let mut verts = Vec::with_capacity(nv);
        let mut uvs = Vec::with_capacity(nv + na);
        for _ in 0..nv {
            verts.push([rdi16(at), rdi16(at + 2), rdi16(at + 4)]);
            uvs.push([rdi16(at + 6), rdi16(at + 8)]);
            at += 10;
        }
        let mut alias_src = Vec::with_capacity(na);
        for _ in 0..na {
            alias_src.push(rd16(at));
            uvs.push([rdi16(at + 2), rdi16(at + 4)]);
            at += 6;
        }
        let mut faces = Vec::with_capacity(nf);
        for _ in 0..nf {
            faces.push([rd16(at), rd16(at + 2), rd16(at + 4)]);
            at += 6;
        }
        HandMesh { segs, verts, uvs, alias_src, faces }
    }

    /// Render the hand into an indexed framebuffer with the cursor at (cx, cy).
    /// Implements the decoded cursor law + Q15 transform + painter-sorted flat fill.
    pub fn draw(&self, fb: &mut [u8], w: usize, h: usize, cx: i32, cy: i32) {
        // EXACT PROJECTION (0x549 path, transcribed): per vertex —
        //   depth = (z_row·v + Tz) >> 8
        //   sx = (x_row·v + Tx) / depth + centre_x        (centres live: 252, 110)
        //   sy = -((y_row·v + Ty) / depth) + centre_y     (y negated)
        // Rows/T mapping per the code: +0x12=y-row/+0x36=Ty, +0x1E=x-row/+0x3A=Tx,
        // +0x2A=z-row/+0x3E=Tz. Our matrix cells follow the transform layout
        // (m00..m22 as x/y/z rows), so y-row = (m10,m11,m12) etc. The asset's T is
        // stored (T36,T3A,T3E) = (Ty,Tx,Tz) in raw Q; used directly — units cancel
        // in the divide, no invented scales.
        let mut pts: Vec<(f32, f32, f32)> = Vec::with_capacity(self.uvs.len());
        let mut vi = 0usize;
        for (si, &(cnt, t, a)) in self.segs.iter().enumerate() {
            let (mut a1, mut a2, a3) = (a[0] as i32, a[1] as i32, a[2] as i32);
            if si == 0 {
                a1 += (cy - 100) * 2;
                a2 += (cx - 160) * 2;
            }
            let m = build_matrix(a1, a2, a3);
            let (ty, tx, tz) = (t[0] as i64, t[1] as i64, t[2] as i64);
            for _ in 0..cnt {
                if vi >= self.verts.len() {
                    break;
                }
                let v = self.verts[vi];
                let (x, y, z) = (v[0] as i64, v[1] as i64, v[2] as i64);
                let (m0, m1, m2) = (m[0] as i64, m[1] as i64, m[2] as i64);
                let (m3, m4, m5) = (m[3] as i64, m[4] as i64, m[5] as i64);
                let (m6, m7, m8) = (m[6] as i64, m[7] as i64, m[8] as i64);
                let xr = m0 * x + m1 * y + m2 * z + tx;
                let yr = m3 * x + m4 * y + m5 * z + ty;
                let zr = m6 * x + m7 * y + m8 * z + tz;
                let depth = (zr >> 8).max(1);
                let sx = (xr / depth) as f32 + 252.0;
                let sy = -((yr / depth) as f32) + 110.0;
                pts.push((sx, sy, depth as f32));
                vi += 1;
            }
        }
        // Aliases resolve to their source's projected point.
        for &src in &self.alias_src {
            let p = pts.get(src as usize).copied().unwrap_or((0.0, 0.0, 1.0));
            pts.push(p);
        }
        // Anchor: the FINGERTIP (topmost projected point) lands at the cursor.
        let (mut tipx, mut tipy) = (0.0f32, f32::MAX);
        for &(x, y, _) in &pts {
            if y < tipy {
                tipy = y;
                tipx = x;
            }
        }
        for p in &mut pts {
            p.0 += cx as f32 - tipx;
            p.1 += cy as f32 - tipy;
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
        assert_eq!(m.verts.len(), 110);
        assert_eq!(m.alias_src.len(), 32);
        assert_eq!(m.faces.len(), 216);
        assert_eq!(m.segs.len(), 16);
        let mut fb = vec![0u8; 320 * 200];
        m.draw(&mut fb, 320, 200, 160, 100);
        let lit = fb.iter().filter(|&&p| p != 0).count();
        assert!(lit > 300, "the hand renders textured pixels ({lit} px)");
    }
}
