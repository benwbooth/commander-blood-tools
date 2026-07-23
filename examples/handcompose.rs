//! Determine the composition rule numerically from the live dump:
//! (1) Does stored_seg4 == stored_seg0 · local(seg4 angles) or local · stored_seg0?
//! (2) Does stored_seg0 == M[ds:0x2250] · local(seg0 angles) (either order)?
//! Q15 products, sar 15. Rotation inverse = transpose lets us extract the local delta.

use commander_blood_tools::manu3_hand::build_matrix;

fn mul(a: &[i64; 9], b: &[i64; 9]) -> [i64; 9] {
    let mut r = [0i64; 9];
    for i in 0..3 {
        for j in 0..3 {
            let mut s = 0i64;
            for k in 0..3 {
                s += a[i * 3 + k] * b[k * 3 + j];
            }
            r[i * 3 + j] = s >> 15;
        }
    }
    r
}
fn tr(a: &[i64; 9]) -> [i64; 9] {
    let mut r = [0i64; 9];
    for i in 0..3 {
        for j in 0..3 {
            r[i * 3 + j] = a[j * 3 + i];
        }
    }
    r
}
fn err(a: &[i64; 9], b: &[i64; 9]) -> i64 {
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).max().unwrap()
}

fn main() {
    let ds = std::fs::read("accuracy/manu3/manu3_ds.bin").unwrap();
    let rdi16 = |at: usize| i16::from_le_bytes([ds[at], ds[at + 1]]) as i32;
    let rdi32 =
        |at: usize| i32::from_le_bytes([ds[at], ds[at + 1], ds[at + 2], ds[at + 3]]) as i64;
    let stored = |si: usize| {
        let rec = 0x2394 + si * 0x5e;
        let mut m = [0i64; 9];
        for i in 0..9 {
            m[i] = rdi32(rec + 0x12 + i * 4);
        }
        m
    };
    let angles = |si: usize| {
        let rec = 0x2394 + si * 0x5e;
        [rdi16(rec + 0x4e), rdi16(rec + 0x50), rdi16(rec + 0x52)]
    };
    let b64 = |a: [i32; 3]| {
        let m = build_matrix(a[0], a[1], a[2]);
        let mut r = [0i64; 9];
        for i in 0..9 {
            r[i] = m[i] as i64;
        }
        r
    };

    let s0 = stored(0);
    for si in [4usize, 5, 7] {
        let s = stored(si);
        let local = b64(angles(si));
        println!(
            "seg {si}: err(parent*local)={}  err(local*parent)={}  err(parent*localT)={}  err(localT*parent)={}",
            err(&mul(&s0, &local), &s),
            err(&mul(&local, &s0), &s),
            err(&mul(&s0, &tr(&local)), &s),
            err(&mul(&tr(&local), &s0), &s),
        );
        // Also: what IS the true local delta? parent^T * stored
        let d = mul(&tr(&s0), &s);
        println!("  true local (s0^T*s4) = {:?}", d);
        println!("  built local          = {:?}", local);
    }

    // Root vs the global matrix at ds:0x2250 (9 dwords assumed).
    let mut g = [0i64; 9];
    for i in 0..9 {
        g[i] = rdi32(0x2250 + i * 4);
    }
    println!("\nglobal M[0x2250] = {g:?}");
    let l0 = b64(angles(0));
    println!(
        "root: err(G*local)={}  err(local*G)={}  err(G*localT)={}  err(localT*G)={}",
        err(&mul(&g, &l0), &s0),
        err(&mul(&l0, &g), &s0),
        err(&mul(&g, &tr(&l0)), &s0),
        err(&mul(&tr(&l0), &g), &s0),
    );
    let d0 = mul(&tr(&g), &s0);
    println!("  true root local (G^T*s0) = {d0:?}");
    println!("  built root local         = {l0:?}");
}
