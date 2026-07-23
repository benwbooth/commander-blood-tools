//! Probe the ppit01 frame-0 RLE decode directly: does it error, and what comes out?

use commander_blood_tools::decompress::decompress_rle_173;

fn main() {
    for name in ["output/_tmp_dat/sq/ppit01.hnm", "output/_tmp_dat/sq/cliptoot.hnm"] {
        let h = commander_blood_tools::hnm::HnmFile::open(std::path::Path::new(name)).unwrap();
        println!("{name}: {} frames, first offsets {:?}", h.frame_count(),
            (0..5).map(|i| h.frame_dims(i)).collect::<Vec<_>>());
    }
    let d = std::fs::read("output/_tmp_dat/sq/ppit01.hnm").unwrap();
    let hs = u16::from_le_bytes([d[0], d[1]]) as usize;
    for (label, off) in [("ppit01 f0", 0usize), ("ppit01 f1", 0x4fbe)] {
        let abs = hs + off;
        let sc = u16::from_le_bytes([d[abs], d[abs + 1]]) as usize;
        let _ = sc;
        // frame block: skip the sc word? decode path: chunks then vhdr at abs+2 when
        // no chunks parse. vhdr at abs+2, block at abs+6.
        let fds = abs + 6;
        match decompress_rle_173(&d, fds) {
            Err(e) => println!("{label}: ERR {e}"),
            Ok(px) => {
                let nz = px.iter().filter(|&&p| p != 0).count();
                use std::collections::HashSet;
                let distinct: HashSet<u8> = px.iter().copied().collect();
                println!(
                    "{label}: ok {} bytes, nonzero {nz}, distinct {}",
                    px.len(),
                    distinct.len()
                );
            }
        }
    }
    let c = std::fs::read("output/_tmp_dat/sq/cliptoot.hnm").unwrap();
    let chs = u16::from_le_bytes([c[0], c[1]]) as usize;
    match decompress_rle_173(&c, chs + 6) {
        Err(e) => println!("cliptoot f0: ERR {e}"),
        Ok(px) => println!(
            "cliptoot f0: ok {} bytes, nonzero {}",
            px.len(),
            px.iter().filter(|&&p| p != 0).count()
        ),
    }
}
