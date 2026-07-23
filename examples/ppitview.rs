//! Decode the ppit* (pupitre = console) HNMs and report fb/palette liveness.

use commander_blood_tools::hnm::HnmFile;
use std::path::Path;

fn main() {
    for name in [
        "output/_tmp_dat/sq/ppit01.hnm",
        "output/_tmp_dat/sq/ppit02.hnm",
        "output/_tmp_dat/sq/ppitbb01.hnm",
        "output/_tmp_dat/sq/ppitbb02.hnm",
        "output/_tmp_dat/sq/ppitbb03.hnm",
        "output/_tmp_dat/sq/ejectorx.hnm",
    ] {
        let Ok(h) = HnmFile::open(Path::new(name)) else {
            println!("{name}: missing");
            continue;
        };
        let nf = h.frame_count();
        let dims = h.frame_dims(0);
        let mut fb = vec![0u8; 320 * 200];
        // Header palette: these clips carry no per-frame `pl` chunks.
        let mut pal = h.palette;
        for f in 0..nf.min(40) {
            h.decode_frame(f, &mut fb, &mut pal);
        }
        let fbnz = fb.iter().filter(|&&p| p != 0).count();
        let palnz = pal.iter().filter(|c| c.iter().any(|&v| v != 0)).count();
        println!(
            "{name}: {nf} frames, dims {dims:?}, band_y {} | fb nonzero {fbnz}, pal nonzero {palnz}",
            h.band_y_origin()
        );
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
