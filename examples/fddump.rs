use commander_blood_tools::lbm::decode_lbm;
fn main() {
    for p in std::env::args().skip(1) {
        let data = std::fs::read(&p).unwrap();
        match decode_lbm(&data) {
            Some(img) => {
                let (w, h) = (img.width, img.height);
                let mut buf = Vec::from(format!("P6\n{w} {h}\n255\n").as_bytes());
                for &idx in &img.pixels {
                    let c = img.palette[idx as usize];
                    buf.extend_from_slice(&c);
                }
                let base = p.rsplit('/').next().unwrap().replace('.', "_");
                let out = format!("/tmp/ben/nix-shell.K54TJW/claude-1000/-home-ben-src-commander-blood-tools/47e60fe4-6817-4d2f-a5e9-d628c3ddc80a/scratchpad/fd_{base}.ppm");
                std::fs::write(&out, buf).unwrap();
                println!("{p}: {w}x{h} -> {out}");
            }
            None => println!("{p}: decode failed"),
        }
    }
}
