//! Find which asset produces the oracle's hub view: decode the first frames of every
//! HNM under the asset dirs and pixel-diff against the oracle background.

use commander_blood_tools::hnm::HnmFile;

fn oracle_bg() -> Vec<u8> {
    let mut frames: Vec<Vec<u8>> = Vec::new();
    for sy in (20..=190).step_by(34) {
        for sx in (40..=280).step_by(40) {
            if let Ok(d) = std::fs::read(format!("boot_frames/hg_{sx}_{sy}.ppm")) {
                let hdr =
                    d.iter().enumerate().filter(|&(_, &b)| b == b'\n').nth(2).unwrap().0 + 1;
                frames.push(d[hdr..].to_vec());
            }
        }
    }
    let n = frames.len();
    let mut bg = vec![0u8; 320 * 200 * 3];
    let mut buf = vec![0u8; n];
    for i in 0..bg.len() {
        for (j, f) in frames.iter().enumerate() {
            buf[j] = f[i];
        }
        buf.sort_unstable();
        bg[i] = buf[n / 2];
    }
    bg
}

fn main() {
    let bg = oracle_bg();
    let mut results: Vec<(f64, String, usize)> = Vec::new();
    for sub in ["sq", "ob", "pl", "pe", "fd"] {
        let dir = format!("output/_tmp_dat/{sub}");
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e != "hnm").unwrap_or(true) {
                continue;
            }
            let Ok(h) = HnmFile::open(&path) else { continue };
            let mut fb = vec![0u8; 320 * 200];
            let mut pal = [[0u8; 3]; 256];
            let nf = h.frame_count();
            for probe in 0..nf {
                h.decode_frame(probe, &mut fb, &mut pal);
                // Compare the LEFT 150 columns only (the oracle's right side holds the
                // live menu overlay; left = the purple loops + orb basin background).
                let (mut acc, mut cnt) = (0f64, 1f64);
                for y in 0..200usize {
                    for x in 0..150usize {
                        // skip the CANCEL text + orb sprite area
                        if (60..=110).contains(&y) && x >= 70 {
                            continue;
                        }
                        if (100..=140).contains(&y) && x >= 100 {
                            continue;
                        }
                        let i = y * 320 + x;
                        let c = pal[fb[i] as usize];
                        for ch in 0..3 {
                            acc += (c[ch] as f64 - bg[i * 3 + ch] as f64).abs();
                            cnt += 1.0;
                        }
                    }
                }
                results.push((acc / cnt, format!("{sub}/{}", entry.file_name().to_string_lossy()), probe));
            }
        }
    }
    results.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    println!("best matches vs the oracle hub view:");
    for (m, name, fr) in results.iter().take(12) {
        println!("  {m:8.2}  {name} frame {fr}");
    }
}
