//! Histogram the length-only (Op) tokens across all scripts — the operand-modeling
//! priority list for the round-trip compiler.

use commander_blood_tools::vm::{walk, VmToken};
use std::collections::HashMap;

fn main() {
    let mut hist: HashMap<u8, (u32, usize)> = HashMap::new();
    for n in 1..=5u32 {
        let cod = std::fs::read(format!("output/_tmp_iso/SCRIPT{n}.COD")).unwrap();
        for t in walk(&cod, 0, cod.len()) {
            if let VmToken::Op { opcode, len, .. } = t {
                let e = hist.entry(opcode).or_default();
                e.0 += 1;
                e.1 += len;
            }
        }
    }
    let mut v: Vec<_> = hist.into_iter().collect();
    v.sort_by_key(|&(_, (n, _))| std::cmp::Reverse(n));
    for (op, (n, bytes)) in v {
        println!("{op:#04x}: {n} tokens, {bytes} bytes");
    }
}
