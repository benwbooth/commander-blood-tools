//! Decode the ppit* (pupitre = console) HNMs with their true frame dims/origins.

use commander_blood_tools::hnm::HnmFile;
use std::path::Path;

fn main() {
    for name in [
        "output/_tmp_dat/sq/ppit01.hnm",
        "output/_tmp_dat/sq/ppit02.hnm",
        "output/_tmp_dat/ob/ppitbb01.hnm",
        "output/_tmp_dat/ob/ppitbb02.hnm",
        "output/_tmp_dat/ob/ppitbb03.hnm",
        "output/_tmp_dat/ob/ejectorx.hnm",
    ] {
        let Ok(h) = HnmFile::open(Path::new(name)) else {
            println!("{name}: missing");
            continue;
        };
        let nf = h.frame_count();
        let dims = h.frame_dims(0);
        println!("{name}: {nf} frames, dims {:?}, band_y {}", dims, h.band_y_origin());
        let mut fb = vec![0u8; 320 * 200];
        let mut pal = [[0u8; 3]; 256];
        for f in 0..nf.min(40) {
            h.decode_frame(f, &mut fb, &mut pal);
        }
        let mut rgb = vec![0u8; 320 * 200 * 3];
        for (i, &p) in fb.iter().enumerate() {
            rgb[i * 3..i * 3 + 3].copy_from_slice(&pal[p as usize]);
        }
        let stem = Path::new(name).file_stem().unwrap().to_string_lossy();
        let mut out = b"P6\n320 200\n255\n".to_vec();
        out.extend_from_slice(&rgb);
        std::fs::write(format!("accuracy/comparisons/hand/{stem}.ppm"), out).unwrap();
    }
}
