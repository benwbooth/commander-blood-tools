//! Dump a world room image's decoded palette — the cyan-cast investigation.
fn main() {
    for name in ["output/_tmp_dat/fd/pterra1f.lbm", "output/_tmp_dat/fd/kortex1b.lbm"] {
        let Ok(d) = std::fs::read(name) else { println!("{name}: missing"); continue };
        let Some(img) = commander_blood_tools::lbm::decode_pbm(&d) else {
            println!("{name}: no decode");
            continue;
        };
        let used: std::collections::BTreeSet<u8> = img.pixels.iter().copied().collect();
        println!("{name}: {}x{} distinct {}", img.width, img.height, used.len());
        for &i in used.iter().take(12) {
            let c = img.palette.get(i as usize).copied().unwrap_or([0, 0, 0]);
            println!("  idx {i:3} -> {c:?}");
        }
    }
}
