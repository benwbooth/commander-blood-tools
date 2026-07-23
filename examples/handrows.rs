//! Numeric ground-truth check: for each skeleton record in the live manu3 dump, compare
//! the STORED composed matrix rows (+0x12..+0x32) against build_matrix(stored angles
//! +0x4E/+0x50/+0x52). If they match, segments are independent (no hierarchy) and the
//! port's matrix path is verifiable cell-by-cell; if not, the delta shows the composition.

use commander_blood_tools::manu3_hand::build_matrix;

fn main() {
    let ds = std::fs::read("accuracy/manu3/manu3_ds.bin").unwrap();
    let rdi16 = |at: usize| i16::from_le_bytes([ds[at], ds[at + 1]]) as i32;
    let rdi32 = |at: usize| i32::from_le_bytes([ds[at], ds[at + 1], ds[at + 2], ds[at + 3]]);
    let base = 0x2394usize;
    for si in 0..16 {
        let rec = base + si * 0x5e;
        let a = [rdi16(rec + 0x4e), rdi16(rec + 0x50), rdi16(rec + 0x52)];
        let m = build_matrix(a[0], a[1], a[2]);
        let stored: Vec<i32> = (0..9).map(|i| rdi32(rec + 0x12 + i * 4)).collect();
        // stored layout: X row (0..2), Y row (3..5), Z row (6..8)
        let cnt = rdi16(rec + 2);
        println!("seg {si:2} cnt={cnt:3} angles={a:?}");
        println!("  stored X={:?} Y={:?} Z={:?}", &stored[0..3], &stored[3..6], &stored[6..9]);
        println!(
            "  built  r0={:?} r1={:?} r2={:?}",
            &m[0..3],
            &m[3..6],
            &m[6..9]
        );
    }
}
