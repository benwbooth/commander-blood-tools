//! Find Honk's talk record offset in SCRIPT2 (the C4 actor guards' operand).

fn main() {
    let cod = std::fs::read("output/_tmp_iso/SCRIPT2.COD").unwrap();
    // C4 guard encoding: 0xC4 <u16 record> <u16 related>; scan for related==40
    // occurrences and histogram the record operands.
    use std::collections::HashMap;
    let mut hist: HashMap<u16, u32> = HashMap::new();
    for i in 0..cod.len().saturating_sub(5) {
        if cod[i] == 0xC4 {
            let rec = u16::from_le_bytes([cod[i + 1], cod[i + 2]]);
            let rel = u16::from_le_bytes([cod[i + 3], cod[i + 4]]);
            if rel == 40 {
                *hist.entry(rec).or_default() += 1;
            }
        }
    }
    let mut v: Vec<_> = hist.into_iter().collect();
    v.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    println!("C4 rel-40 records: {v:?}");
}
