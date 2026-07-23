//! Render the hand DIRECTLY from the live manu3 data-segment dump: per-segment COMPOSED
//! matrix rows (+0x12/+0x1E/+0x2A dwords) + T (+0x36..) exactly as the game left them,
//! through the re-verified 0x549 projection. No matrix building, no cursor law, no
//! anchoring — this is the game's own composed state, so the output MUST look like the
//! real hand. Establishes the numeric ground truth the port's own matrix path must match.

use std::path::Path;

fn main() {
    let ds = std::fs::read("accuracy/manu3/manu3_ds.bin").unwrap();
    let seg2 = std::fs::read("accuracy/manu3/manu3_seg2_1b76.bin").unwrap();
    let tex = std::fs::read("accuracy/manu3/hand_tex.bin").unwrap();
    let rd16 = |d: &[u8], at: usize| u16::from_le_bytes([d[at], d[at + 1]]) as usize;
    let rdi16 = |d: &[u8], at: usize| i16::from_le_bytes([d[at], d[at + 1]]) as i64;
    let rdi32 = |d: &[u8], at: usize| {
        i32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]]) as i64
    };

    // Vertex pool: 110 x 20B at seg2:0 (+0/+2 UV, +4..+8 model xyz), then 32 aliases.
    let nvert = 110usize;
    let nalias = 32usize;

    // Skeleton records: ds:0x2394, stride 0x5E. +2 = vert count; rows at +0x12(X),
    // +0x1E(Y), +0x2A(Z=depth) as 3 dwords each; T at +0x36(x)/+0x3A(y)/+0x3E(z).
    let base = 0x2394usize;
    let mut pts: Vec<(f32, f32, f32)> = Vec::new();
    let mut rec = base;
    let mut vi = 0usize;
    while vi < nvert {
        let cnt = rd16(&ds, rec + 2);
        let xr_row = [rdi32(&ds, rec + 0x12), rdi32(&ds, rec + 0x16), rdi32(&ds, rec + 0x1a)];
        let yr_row = [rdi32(&ds, rec + 0x1e), rdi32(&ds, rec + 0x22), rdi32(&ds, rec + 0x26)];
        let zr_row = [rdi32(&ds, rec + 0x2a), rdi32(&ds, rec + 0x2e), rdi32(&ds, rec + 0x32)];
        let t = [rdi32(&ds, rec + 0x36), rdi32(&ds, rec + 0x3a), rdi32(&ds, rec + 0x3e)];
        for _ in 0..cnt {
            if vi >= nvert {
                break;
            }
            let at = vi * 20;
            let v = [rdi16(&seg2, at + 4), rdi16(&seg2, at + 6), rdi16(&seg2, at + 8)];
            let dot = |r: &[i64; 3]| r[0] * v[0] + r[1] * v[1] + r[2] * v[2];
            let depth = (dot(&zr_row) + t[2]) >> 8;
            if depth <= 0 {
                pts.push((-1000.0, -1000.0, 1.0));
            } else {
                let sx = (dot(&xr_row) + t[0]) / depth + 252;
                let sy = -((dot(&yr_row) + t[1]) / depth) + 110;
                pts.push((sx as f32, sy as f32, depth as f32));
            }
            vi += 1;
        }
        rec += 0x5e;
    }
    println!("verts projected: {} (records used: {})", pts.len(), (rec - base) / 0x5e);
    // Aliases: 20B records after the pool, +4 = source vertex byte-offset in the pool.
    for a in 0..nalias {
        let at = (nvert + a) * 20;
        let src_off = rd16(&seg2, at + 4);
        let src = src_off / 20;
        let p = pts.get(src).copied().unwrap_or((-1000.0, -1000.0, 1.0));
        pts.push(p);
    }

    // Faces: 216 x 8B {link, v0, v1, v2} at seg2:0xB18 — vertex fields are byte-offsets.
    let mut fb = vec![0u8; 320 * 200];
    let mut zbuf = vec![f32::MAX; 320 * 200];
    let mut drawn = 0;
    for f in 0..216 {
        let at = 0xb18 + f * 8;
        let (a, b, c) = (
            rd16(&seg2, at + 2) / 20,
            rd16(&seg2, at + 4) / 20,
            rd16(&seg2, at + 6) / 20,
        );
        let (pa, pb, pc) = (pts[a], pts[b], pts[c]);
        let area = (pb.0 - pa.0) * (pc.1 - pa.1) - (pb.1 - pa.1) * (pc.0 - pa.0);
        if area <= 0.0 {
            continue;
        }
        drawn += 1;
        let uv = |i: usize| {
            let at = i.min(nvert + nalias - 1) * 20;
            (rd16(&seg2, at) as f32, rd16(&seg2, at + 2) as f32)
        };
        let (ua, ub, uc) = (uv(a), uv(b), uv(c));
        let (x0, x1) = (
            pa.0.min(pb.0).min(pc.0).max(0.0) as i32,
            pa.0.max(pb.0).max(pc.0).min(319.0) as i32,
        );
        let (y0, y1) = (
            pa.1.min(pb.1).min(pc.1).max(0.0) as i32,
            pa.1.max(pb.1).max(pc.1).min(199.0) as i32,
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let w0 = (pb.0 - pa.0) * (py - pa.1) - (pb.1 - pa.1) * (px - pa.0);
                let w1 = (pc.0 - pb.0) * (py - pb.1) - (pc.1 - pb.1) * (px - pb.0);
                let w2 = (pa.0 - pc.0) * (py - pc.1) - (pa.1 - pc.1) * (px - pc.0);
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let (l0, l1, l2) = (w1 / area, w2 / area, w0 / area);
                let z = pa.2 * l0 + pb.2 * l1 + pc.2 * l2;
                let idx = (y * 320 + x) as usize;
                if z >= zbuf[idx] {
                    continue;
                }
                let u = (ua.0 * l0 + ub.0 * l1 + uc.0 * l2) as usize;
                let v = (ua.1 * l0 + ub.1 * l1 + uc.1 * l2) as usize;
                let hi = |q: usize| (q >> 8) & 0xff;
                let texel = tex.get(hi(v) * 256 + hi(u)).copied().unwrap_or(0);
                if texel != 0 {
                    zbuf[idx] = z;
                    fb[idx] = texel;
                }
            }
        }
    }
    println!("faces drawn: {drawn}/216");
    let on: usize = fb.iter().filter(|&&p| p != 0).count();
    println!("lit pixels: {on}");
    // bbox
    let (mut x0, mut y0, mut x1, mut y1) = (320, 200, 0, 0);
    for y in 0..200 {
        for x in 0..320 {
            if fb[y * 320 + x] != 0 {
                x0 = x0.min(x);
                x1 = x1.max(x);
                y0 = y0.min(y);
                y1 = y1.max(y);
            }
        }
    }
    println!("bbox: ({x0},{y0})..({x1},{y1})  {}x{}", x1 - x0 + 1, y1 - y0 + 1);

    // Save as RGB using the game palette range the hand indices live in: reuse the
    // gp[] palette install the engine does — approximate with a grayscale if absent.
    let mut rgb = vec![0u8; 320 * 200 * 3];
    // Simple: show indices as intensity ramp (shape is what matters here).
    for (i, &p) in fb.iter().enumerate() {
        let v = if p == 0 { 0 } else { 40 + (p as u16 * 3 / 4) as u8 };
        rgb[i * 3] = v;
        rgb[i * 3 + 1] = v.saturating_add(20);
        rgb[i * 3 + 2] = v.saturating_add(40);
    }
    let mut out = b"P6\n320 200\n255\n".to_vec();
    out.extend_from_slice(&rgb);
    std::fs::create_dir_all(Path::new("accuracy/comparisons/hand")).unwrap();
    std::fs::write("accuracy/comparisons/hand/handraw.ppm", out).unwrap();
    println!("wrote accuracy/comparisons/hand/handraw.ppm");
}
