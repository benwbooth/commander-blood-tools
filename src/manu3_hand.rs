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

/// The live manu3 data segment (64KB dump): the skeleton NODE TREE lives at
/// 0x2274 (root) + 16 records at 0x2394 + i*0x5E. Record layout (decoded 0x270/0x3DE):
///   +0x00 parent record ptr   +0x02 vertex count
///   +0x12..+0x32 composed rows (X/Y/Z, Q15 dwords)   +0x36/3A/3E composed T
///   +0x42/46/4A local position L (dwords)            +0x4E/50/52 local Euler angles
///   +0x54 forward speed (position integrator: L += local-Z * speed >> 16)
/// Composition (verified EXACT vs every node): rows = parent_rows*build(angles) >> 15,
/// T = parent_rows @ L + T_parent. The wrist (0x2394) is the runtime root: its T is the
/// hand's view-space placement and its a1/a2 receive the cursor law non-destructively
/// (entry 0x0000 pushes [0x23E2]/[0x23E4], adds (y-100)*2 / (x-160)*2, composes, pops).
const DS: &[u8] = include_bytes!("../accuracy/manu3/manu3_ds.bin");
/// The seg2 pool: 110 x 20B vertices {+0/+2 UV, +4..+8 model xyz} + 32 alias records
/// (+4 = source byte offset), then the 216 x 8B face list at 0xB18 {link, v0, v1, v2}.
const SEG2: &[u8] = include_bytes!("../accuracy/manu3/manu3_seg2_1b76.bin");
/// The node-tree state block ds[0x2274..0x2974]: root + 16 records, mutable at runtime
/// (pose tweens write cells into it exactly as the game does).
const STATE_BASE: usize = 0x2274;
const STATE_LEN: usize = 0x700;
const WRIST: usize = 0x2394;
/// The hand's 256-wide texture (palette-index rows) lifted from the live data segment.
// THE REAL TEXTURE SEGMENT: the fill's fs parameter block (captured live at the
// span setup 0x120B: fs=17A3, fs:[2]=1B76 vertex seg, fs:[4]=1C94 TEXTURE seg,
// fs:[6]=2094) names manu3_seg4_1c94.bin as the texture base — its material spans
// rows 0..62, exactly the mesh's v range, so the seam faces (v 43..62) sample real
// smooth skin. (The old hand_tex.bin bank came from ds:0x6400 — a different buffer;
// rows past 41 there are unrelated scratch, which forced a row clamp + palm banding.)
const TEX: &[u8] = include_bytes!("../accuracy/manu3/manu3_seg4_1c94.bin");
const TEX_W: usize = 256;
/// The game's own sin/cos tables (ds:0x26, 1024 entries x {cos:i16, sin:i16}, Q14).
const TRIG: &[u8] = include_bytes!("../accuracy/manu3/trig_tables.bin");

/// The hand's texture bytes + row width (for GPU upload).
pub fn hand_texture() -> (&'static [u8], usize) {
    (TEX, TEX_W)
}

#[inline]
fn rd_parent(state: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([state[at - STATE_BASE], state[at - STATE_BASE + 1]])
}

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
pub fn build_matrix(a1: i32, a2: i32, a3: i32) -> [i32; 9] {
    // EXACT 0x270 build (verified vs every dumped node, err <= 3): the game computes
    // the products via angle-sum identities on the Q14 table; the closed forms are
    //   m00=c2c3-s1s2s3  m01=c2s3+s1s2c3  m02=c1s2
    //   m10=-c1s3        m11=c1c3         m12=-s1
    //   m20=-(s2c3+s1c2s3) m21=s1c2c3-s2s3 m22=c1c2     (all x2: Q14 -> Q15)
    let (c1, s1) = (tcos(a1) as i64, tsin(a1) as i64);
    let (c2, s2) = (tcos(a2) as i64, tsin(a2) as i64);
    let (c3, s3) = (tcos(a3) as i64, tsin(a3) as i64);
    let q = |x: i64, y: i64| (x * y) >> 14;
    [
        (2 * (q(c2, c3) - q(q(s1, s2), s3))) as i32,
        (2 * (q(c2, s3) + q(q(s1, s2), c3))) as i32,
        (2 * q(c1, s2)) as i32,
        (-2 * q(c1, s3)) as i32,
        (2 * q(c1, c3)) as i32,
        (-2 * s1) as i32,
        (-2 * (q(s2, c3) + q(q(s1, c2), s3))) as i32,
        (2 * (q(q(s1, c2), c3) - q(s2, s3))) as i32,
        (2 * q(c1, c2)) as i32,
    ]
}

pub struct HandMesh {
    /// Current pose selector + its running player (selector -> sequence index,
    /// per the 0x181 dispatch; decoded contexts: 1=rest, 2=steer-right, 3=steer-left,
    /// 0xB=UI-close, 0xFFFF=hidden).
    pose_sel: u16,
    pose: Option<PosePlayer>,
    /// The live node-tree block ds[0x2274..0x2974] — root + 16 skeleton records,
    /// addressed by original ds offsets (pose tween cells write straight into it).
    state: Vec<u8>,
    verts: Vec<[i16; 3]>,
    uvs: Vec<[i16; 2]>,
    /// index >= verts.len(): alias — resolves to `alias_src`.
    alias_src: Vec<u16>,
    faces: Vec<[u16; 3]>,
}

impl HandMesh {
    pub fn load() -> HandMesh {
        let rd16 = |d: &[u8], at: usize| u16::from_le_bytes([d[at], d[at + 1]]);
        let rdi16 = |d: &[u8], at: usize| i16::from_le_bytes([d[at], d[at + 1]]);
        let state = DS[STATE_BASE..STATE_BASE + STATE_LEN].to_vec();
        let nvert = 110usize;
        let nalias = 32usize;
        let mut verts = Vec::with_capacity(nvert);
        let mut uvs = Vec::with_capacity(nvert + nalias);
        for i in 0..nvert {
            let at = i * 20;
            verts.push([rdi16(SEG2, at + 4), rdi16(SEG2, at + 6), rdi16(SEG2, at + 8)]);
            uvs.push([rdi16(SEG2, at), rdi16(SEG2, at + 2)]);
        }
        let mut alias_src = Vec::with_capacity(nalias);
        for a in 0..nalias {
            let at = (nvert + a) * 20;
            alias_src.push(rd16(SEG2, at + 4) / 20);
            // An alias shares its SOURCE's projected position but carries its OWN
            // texture coordinates (+0/+2) — aliases ARE the mesh's UV-seam vertices
            // (verified: every alias's own UV differs from its source's).
            uvs.push([rdi16(SEG2, at), rdi16(SEG2, at + 2)]);
        }
        let mut faces = Vec::with_capacity(216);
        for f in 0..216 {
            let at = 0xB18 + f * 8;
            faces.push([
                rd16(SEG2, at + 2) / 20,
                rd16(SEG2, at + 4) / 20,
                rd16(SEG2, at + 6) / 20,
            ]);
        }
        HandMesh { pose_sel: 1, pose: None, state, verts, uvs, alias_src, faces }
    }

    #[inline]
    fn st16(&self, off: usize) -> i32 {
        i16::from_le_bytes([self.state[off - STATE_BASE], self.state[off - STATE_BASE + 1]])
            as i32
    }
    #[inline]
    fn st32(&self, off: usize) -> i64 {
        let o = off - STATE_BASE;
        i32::from_le_bytes([
            self.state[o],
            self.state[o + 1],
            self.state[o + 2],
            self.state[o + 3],
        ]) as i64
    }

    /// Compose the skeleton for cursor (cx, cy): per node, local = build(angles) —
    /// the wrist gets the cursor law added non-destructively — then
    /// rows = parent_rows * local >> 15 and T = parent_rows @ L + T_parent
    /// (the wrist keeps its stored view-space T). Returns per-node (rows, T).
    fn compose(&self, cx: i32, cy: i32) -> Vec<([i64; 9], [i64; 3])> {
        let mut out: std::collections::HashMap<usize, ([i64; 9], [i64; 3])> =
            std::collections::HashMap::new();
        // Root 0x2274: stored rows (identity in the dump) + stored T.
        let read_rows = |at: usize| {
            let mut r = [0i64; 9];
            for i in 0..9 {
                r[i] = self.st32(at + 0x12 + i * 4);
            }
            r
        };
        let read_t = |at: usize| {
            [self.st32(at + 0x36), self.st32(at + 0x3A), self.st32(at + 0x3E)]
        };
        out.insert(0x2274, (read_rows(0x2274), read_t(0x2274)));
        let mut result = Vec::with_capacity(16);
        for i in 0..16 {
            let at = WRIST + i * 0x5E;
            let parent = rd_parent(&self.state, at);
            let (prow, pt) = out[&(parent as usize)];
            let (mut a1, mut a2, a3) =
                (self.st16(at + 0x4E), self.st16(at + 0x50), self.st16(at + 0x52));
            if at == WRIST {
                // The cursor law (entry 0x0000): pitch += (y-100)*2, yaw += (x-160)*2.
                a1 += (cy - 100) * 2;
                a2 += (cx - 160) * 2;
            }
            let local = build_matrix(a1, a2, a3);
            let mut rows = [0i64; 9];
            for r in 0..3 {
                for c in 0..3 {
                    let mut acc = 0i64;
                    for k in 0..3 {
                        acc += prow[r * 3 + k] * local[k * 3 + c] as i64;
                    }
                    rows[r * 3 + c] = acc >> 15;
                }
            }
            let t = if at == WRIST {
                read_t(at)
            } else {
                let l = [self.st32(at + 0x42), self.st32(at + 0x46), self.st32(at + 0x4A)];
                let mut t = [0i64; 3];
                for r in 0..3 {
                    t[r] = prow[r * 3] * l[0] + prow[r * 3 + 1] * l[1] + prow[r * 3 + 2] * l[2]
                        + pt[r];
                }
                t
            };
            out.insert(at, (rows, t));
            result.push((rows, t));
        }
        result
    }

    /// Render the hand into an indexed framebuffer with the cursor at (cx, cy):
    /// the decoded cursor law + hierarchical composition + the re-verified 0x549
    /// projection. No screen-space fixups — the fingertip lands at the cursor
    /// because the game's own math puts it there (HANDGRID oracle: tip = cursor+(2,-3)).
    pub fn draw(&self, fb: &mut [u8], w: usize, h: usize, cx: i32, cy: i32) {
        let nodes = self.compose(cx, cy);
        // CURSOR-CENTRED PROJECTION (entry 0x0060..0x0119, exact): the game projects
        // the FINGERTIP (vertex 34 through node 0x24AE, the last index-finger joint)
        // and derives the projection centres from the cursor —
        //   centre_x [0x223E] = cursor_x - (Xrow.tip + Tx)/tip_depth
        //   centre_y [0x2242] = cursor_y + (Yrow.tip + Ty)/tip_depth
        // so the fingertip lands EXACTLY at the cursor by construction; the wrist law
        // rotation supplies the aiming parallax. Then 0x549 projects every vertex:
        //   depth = (Zrow.v + Tz) >> 8 (skip <= 0)
        //   sx = (Xrow.v + Tx)/depth + centre_x;  sy = -((Yrow.v + Ty)/depth) + centre_y
        let (ctr_x, ctr_y) = {
            let (rows, t) = nodes[3];
            let v = self.verts[34];
            let (x, y, z) = (v[0] as i64, v[1] as i64, v[2] as i64);
            let depth = ((rows[6] * x + rows[7] * y + rows[8] * z + t[2]) >> 8).max(1);
            let xo = (rows[0] * x + rows[1] * y + rows[2] * z + t[0]) / depth;
            let yo = (rows[3] * x + rows[4] * y + rows[5] * z + t[1]) / depth;
            (cx as i64 - xo, cy as i64 + yo)
        };
        let mut pts: Vec<(f32, f32, f32)> = Vec::with_capacity(self.uvs.len());
        let mut vi = 0usize;
        for i in 0..16 {
            let cnt = self.st16(WRIST + i * 0x5E + 2) as usize;
            let (rows, t) = nodes[i];
            for _ in 0..cnt {
                if vi >= self.verts.len() {
                    break;
                }
                let v = self.verts[vi];
                let (x, y, z) = (v[0] as i64, v[1] as i64, v[2] as i64);
                let zr = rows[6] * x + rows[7] * y + rows[8] * z + t[2];
                let depth = zr >> 8;
                if depth <= 0 {
                    pts.push((-4096.0, -4096.0, 1.0));
                } else {
                    let xr = rows[0] * x + rows[1] * y + rows[2] * z + t[0];
                    let yr = rows[3] * x + rows[4] * y + rows[5] * z + t[1];
                    let sx = (xr / depth + ctr_x) as f32;
                    let sy = (-(yr / depth) + ctr_y) as f32;
                    pts.push((sx, sy, depth as f32));
                }
                vi += 1;
            }
        }
        // Aliases resolve to their source's projected point.
        for &src in &self.alias_src {
            let p = pts.get(src as usize).copied().unwrap_or((-4096.0, -4096.0, 1.0));
            pts.push(p);
        }
        // Hidden surfaces resolve PER PIXEL by depth (a z-buffer computes the same
        // visibility the game's depth-sorted span engine does; only sub-pixel edge
        // stepping can differ). Painter order kept for deterministic iteration.
        let mut zbuf = vec![f32::MAX; w * h];
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
            let area = (pb.0 - pa.0) * (pc.1 - pa.1) - (pb.1 - pa.1) * (pc.0 - pa.0);
            if area <= 0.0 {
                continue;
            }
            let ta = self.uvs[a as usize];
            let tb = self.uvs[b as usize];
            let tc = self.uvs[c as usize];
            fill_triangle_tex(fb, &mut zbuf, w, h, pa, pb, pc, ta, tb, tc);
        }
    }

    /// Snapshot the mutable skeleton state (for between-tick interpolation).
    pub fn snapshot_state(&self) -> Vec<u8> {
        self.state.clone()
    }

    /// Like [`Self::triangles`], but with the animated skeleton cells (angles +
    /// local positions of every node) LERPed between a previous snapshot and the
    /// current state by `alpha` — smooth pose motion between game ticks. The
    /// interpolation happens on the INPUT cells (i16 angles / i32 positions),
    /// then the exact compose/projection runs unchanged.
    pub fn triangles_lerp(
        &mut self,
        cx: i32,
        cy: i32,
        prev: &[u8],
        alpha: f32,
    ) -> Vec<[[f32; 5]; 3]> {
        if prev.len() != self.state.len() || alpha >= 1.0 {
            return self.triangles(cx, cy);
        }
        let cur = self.state.clone();
        let a = alpha.clamp(0.0, 1.0);
        for i in 0..16 {
            let rec = (WRIST - STATE_BASE) + i * 0x5E;
            for field in [0x4Eusize, 0x50, 0x52] {
                let o = rec + field;
                let pv = i16::from_le_bytes([prev[o], prev[o + 1]]) as f32;
                let cv = i16::from_le_bytes([cur[o], cur[o + 1]]) as f32;
                let v = (pv + (cv - pv) * a) as i16;
                self.state[o..o + 2].copy_from_slice(&v.to_le_bytes());
            }
            for field in [0x42usize, 0x46, 0x4A] {
                let o = rec + field;
                let pv = i32::from_le_bytes([prev[o], prev[o + 1], prev[o + 2], prev[o + 3]])
                    as f32;
                let cv = i32::from_le_bytes([cur[o], cur[o + 1], cur[o + 2], cur[o + 3]])
                    as f32;
                let v = (pv + (cv - pv) * a) as i32;
                self.state[o..o + 4].copy_from_slice(&v.to_le_bytes());
            }
        }
        let tris = self.triangles(cx, cy);
        self.state = cur;
        tris
    }

    /// The hand's textured triangles for GPU rendering: same compose + cursor-centred
    /// projection as draw(), emitted as screen-space (x, y, depth, u, v) vertices
    /// (u/v in texel units — the game's Q8 coords / 256). Backface-culled like the
    /// software path; depth in game units for the z-buffer.
    pub fn triangles(&self, cx: i32, cy: i32) -> Vec<[[f32; 5]; 3]> {
        let pts = self.debug_project(cx, cy);
        let mut out = Vec::with_capacity(self.faces.len());
        for fi in 0..self.faces.len() {
            let [a, b, c] = self.faces[fi];
            let pa = pts[a as usize];
            let pb = pts[b as usize];
            let pc = pts[c as usize];
            let area = (pb.0 - pa.0) * (pc.1 - pa.1) - (pb.1 - pa.1) * (pc.0 - pa.0);
            if area <= 0.0 {
                continue;
            }
            let uv = |i: usize| {
                // self.uvs holds TEXEL coordinates (0..255 / 0..rows) directly —
                // the same raw values the software fill interpolates and clamps.
                let t = self.uvs[i];
                [t[0] as f32, t[1] as f32]
            };
            let (ua, ub, uc) = (uv(a as usize), uv(b as usize), uv(c as usize));
            out.push([
                [pa.0, pa.1, pa.2, ua[0], ua[1]],
                [pb.0, pb.1, pb.2, ub[0], ub[1]],
                [pc.0, pc.1, pc.2, uc[0], uc[1]],
            ]);
        }
        out
    }

    /// Debug helper: the projected point cloud (screen x, y, depth) for a cursor.
    pub fn debug_project(&self, cx: i32, cy: i32) -> Vec<(f32, f32, f32)> {
        let nodes = self.compose(cx, cy);
        let (ctr_x, ctr_y) = {
            let (rows, t) = nodes[3];
            let v = self.verts[34];
            let (x, y, z) = (v[0] as i64, v[1] as i64, v[2] as i64);
            let depth = ((rows[6] * x + rows[7] * y + rows[8] * z + t[2]) >> 8).max(1);
            let xo = (rows[0] * x + rows[1] * y + rows[2] * z + t[0]) / depth;
            let yo = (rows[3] * x + rows[4] * y + rows[5] * z + t[1]) / depth;
            (cx as i64 - xo, cy as i64 + yo)
        };
        let mut pts = Vec::new();
        let mut vi = 0usize;
        for i in 0..16 {
            let cnt = self.st16(WRIST + i * 0x5E + 2) as usize;
            let (rows, t) = nodes[i];
            for _ in 0..cnt {
                if vi >= self.verts.len() {
                    break;
                }
                let v = self.verts[vi];
                let (x, y, z) = (v[0] as i64, v[1] as i64, v[2] as i64);
                let zr = rows[6] * x + rows[7] * y + rows[8] * z + t[2];
                let depth = zr >> 8;
                if depth <= 0 {
                    pts.push((-4096.0, -4096.0, 1.0));
                } else {
                    let xr = rows[0] * x + rows[1] * y + rows[2] * z + t[0];
                    let yr = rows[3] * x + rows[4] * y + rows[5] * z + t[1];
                    pts.push(((xr / depth + ctr_x) as f32, (-(yr / depth) + ctr_y) as f32, depth as f32));
                }
                vi += 1;
            }
        }
        // Aliases resolve to their source's projected point (as in draw()).
        for &src in &self.alias_src {
            let p = pts.get(src as usize).copied().unwrap_or((-4096.0, -4096.0, 1.0));
            pts.push(p);
        }
        pts
    }

    fn face_depth(&self, fi: usize, pts: &[(f32, f32, f32)]) -> f32 {
        let [a, b, c] = self.faces[fi];
        (pts[a as usize].2 + pts[b as usize].2 + pts[c as usize].2) / 3.0
    }
}

fn fill_triangle_tex(
    fb: &mut [u8],
    zbuf: &mut [f32],
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
    // Top-left fill convention (matches a span rasterizer's pixel ownership: a pixel
    // belongs to exactly one triangle along shared edges, eliminating the ±1px seam
    // that a naive >=0 test leaves — the sub-pixel edge residual).
    let tl = |ex: f32, ey: f32| -> bool { ey < 0.0 || (ey == 0.0 && ex < 0.0) };
    let e_ab = (b.0 - a.0, b.1 - a.1);
    let e_bc = (c.0 - b.0, c.1 - b.1);
    let e_ca = (a.0 - c.0, a.1 - c.1);
    let bias0 = if tl(e_ab.0, e_ab.1) { 0.0 } else { -1e-4 };
    let bias1 = if tl(e_bc.0, e_bc.1) { 0.0 } else { -1e-4 };
    let bias2 = if tl(e_ca.0, e_ca.1) { 0.0 } else { -1e-4 };
    for y in miny..=maxy {
        for x in minx..=maxx {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let w0 = ((b.0 - a.0) * (fy - a.1) - (b.1 - a.1) * (fx - a.0)) / area;
            let w1 = ((c.0 - b.0) * (fy - b.1) - (c.1 - b.1) * (fx - b.0)) / area;
            let w2 = 1.0 - w0 - w1;
            if w0 >= bias0 && w1 >= bias1 && w2 >= bias2 {
                let u = (tb[0] as f32 * w0 + tc[0] as f32 * w1 + ta[0] as f32 * w2)
                    .clamp(0.0, 255.0) as usize;
                let v = (tb[1] as f32 * w0 + tc[1] as f32 * w1 + ta[1] as f32 * w2)
                    .max(0.0) as usize;
                let z = a.2 * w2 + b.2 * w0 + c.2 * w1;
                let pi = y as usize * w + x as usize;
                if z >= zbuf[pi] {
                    continue;
                }
                // Unconditional write — the game's fill has NO transparency.
                // The seg4 texture's material spans rows 0..62 = the mesh's whole v
                // range; clamp only as a safety bound (interpolation overshoot).
                let vc = v.min(62);
                let ti = vc * TEX_W + u;
                if ti < TEX.len() {
                    zbuf[pi] = z;
                    fb[pi] = TEX[ti];
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

/// The phased tween pose player (exact transcription of 0x181/0x1DF/0x19B):
/// applies sequence `seq`'s groups to the segment cells, animating the hand.
/// Cells address the live record block (0x2394 + seg*0x5E + field).
pub struct PosePlayer {
    seq: Vec<[u16; 4]>,
    cursor: usize,
    phase: u16,
    active: Vec<(u16, u16, i32, i32)>, // (counter, cell, accum(Q16), step(Q16))
}

impl PosePlayer {
    /// Look up the sequence for a SELECTOR through the game's own dispatch table
    /// (0x181 decoded: table base = ds:[0x2306] = 0x2974; sequence = base +
    /// table[(selector & 0x1F) * 2]; groups run until a count==0 terminator).
    /// 17 distinct sequences, one per selector 0..0x10; higher selectors alias
    /// selector 0's no-op.
    pub fn new(selector: usize) -> Option<PosePlayer> {
        let rd16 = |at: usize| u16::from_le_bytes([DS[at], DS[at + 1]]);
        let base = rd16(0x2306) as usize;
        let off = rd16(base + (selector & 0x1F) * 2) as usize;
        let mut at = base + off;
        let mut seq = Vec::new();
        loop {
            let g = [rd16(at), rd16(at + 2), rd16(at + 4), rd16(at + 6)];
            seq.push(g);
            if g[0] & 0xFF == 0 || seq.len() > 512 {
                break;
            }
            at += 8;
        }
        Some(PosePlayer { seq, cursor: 0, phase: 0, active: Vec::new() })
    }

    /// One frame: construct due groups (phase match), step active tweens; writes go
    /// through `cells` (cell address -> current value), exactly as 0x1DF/0x19B do.
    pub fn step(&mut self, cells: &mut dyn FnMut(u16, Option<i16>) -> i16) {
        // Construct groups whose phase == current phase.
        while self.cursor < self.seq.len() {
            let g = self.seq[self.cursor];
            let count = g[0] & 0xFF;
            let phase = g[0] >> 8;
            if count == 0 {
                // count==0 = end of sequence (the 0x23E path).
                self.cursor = self.seq.len();
                break;
            }
            if phase != self.phase {
                // Phase boundary: advance once per frame (the 0x239 path).
                self.phase += 1;
                break;
            }
            let target = g[2];
            let end = g[3] as i16 as i32;
            let cur = cells(target, None) as i32;
            let step = ((end - cur) << 16) / count as i32;
            self.active.push((count - 1, target, (cur << 16) + step, step));
            self.cursor += 1;
        }
        // Step active tweens (0x19B): write value, decrement, accumulate.
        let mut i = 0;
        while i < self.active.len() {
            let cell = self.active[i].1;
            let val = (self.active[i].2 >> 16) as i16;
            cells(cell, Some(val));
            if self.active[i].0 == 0 {
                self.active.swap_remove(i);
            } else {
                self.active[i].0 -= 1;
                let step = self.active[i].3;
                self.active[i].2 += step;
                i += 1;
            }
        }
    }

    pub fn done(&self) -> bool {
        self.cursor >= self.seq.len() && self.active.is_empty()
    }
}

impl HandMesh {
    /// Select the pose (decoded selector semantics); a change starts that sequence.
    pub fn set_pose(&mut self, sel: u16) {
        if sel != self.pose_sel {
            self.pose_sel = sel;
            self.pose = PosePlayer::new(sel as usize);
        }
    }

    /// Advance the current pose animation one frame (call per rendered frame).
    pub fn tick_pose(&mut self) {
        if let Some(mut p) = self.pose.take() {
            if !p.done() {
                self.animate(&mut p);
                self.pose = Some(p);
            }
        }
    }

    /// Apply one pose-player frame to the live skeleton: tween cell writes land in
    /// the node-tree state block BY ORIGINAL DS OFFSET (cells address 0x2274..0x2974:
    /// node angles +0x4E/50/52, local position words, wrist T), exactly as the game's
    /// tween engine pokes ds cells.
    pub fn animate(&mut self, player: &mut PosePlayer) {
        let state = &mut self.state;
        player.step(&mut |cell, write| {
            let off = cell as usize;
            if !(STATE_BASE..STATE_BASE + STATE_LEN - 1).contains(&off) {
                return 0;
            }
            let o = off - STATE_BASE;
            match write {
                Some(v) => {
                    state[o..o + 2].copy_from_slice(&v.to_le_bytes());
                    v
                }
                None => i16::from_le_bytes([state[o], state[o + 1]]),
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pose player runs sequence 0 to completion, writing plausible values
    /// into the segment cells (the exact 0x1DF construction + 0x19B stepping).
    #[test]
    fn pose_player_animates_cells() {
        // Sequence 0 is the null pose (empty stream); later selectors carry the
        // real animations — at least one must terminate AND animate cells.
        let mut animated = false;
        for si in 0..8 {
            let Some(mut p) = PosePlayer::new(si) else { continue };
            let mut store = std::collections::HashMap::new();
            let mut frames = 0;
            while !p.done() && frames < 4000 {
                p.step(&mut |cell, write| {
                    if let Some(v) = write {
                        store.insert(cell, v);
                        v
                    } else {
                        store.get(&cell).copied().unwrap_or(0)
                    }
                });
                frames += 1;
            }
            if frames < 4000 && store.len() > 4 {
                animated = true;
                break;
            }
        }
        assert!(animated, "at least one pose sequence animates cells");
    }

    /// Animating the mesh with a pose sequence changes the skeleton state and the
    /// hand still renders — the pose pipeline is live end to end.
    #[test]
    fn pose_animates_the_skeleton() {
        let mut m = HandMesh::load();
        let before = m.state.clone();
        for si in 1..8 {
            if let Some(mut p) = PosePlayer::new(si) {
                for _ in 0..600 {
                    if p.done() {
                        break;
                    }
                    m.animate(&mut p);
                }
            }
        }
        assert_ne!(before, m.state, "pose sequences move the joints");
        let mut fb = vec![0u8; 320 * 200];
        m.draw(&mut fb, 320, 200, 160, 100);
        assert!(fb.iter().any(|&p| p != 0), "the animated hand still renders");
    }

    #[test]
    fn hand_mesh_loads_and_renders() {
        let m = HandMesh::load();
        assert_eq!(m.verts.len(), 110);
        assert_eq!(m.alias_src.len(), 32);
        assert_eq!(m.faces.len(), 216);
        let mut fb = vec![0u8; 320 * 200];
        m.draw(&mut fb, 320, 200, 160, 100);
        let lit = fb.iter().filter(|&&p| p != 0).count();
        assert!(lit > 300, "the hand renders textured pixels ({lit} px)");
    }
}
