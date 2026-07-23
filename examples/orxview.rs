//! View ORX.FD (the console close-up background) next to the oracle hub view.

fn main() {
    let d = std::fs::read("output/_tmp_iso/ORX.FD").unwrap();
    let img = commander_blood_tools::lbm::decode_lbm(&d).expect("lbm");
    println!("ORX.FD: {}x{} palette {}", img.width, img.height, img.palette.len());
    let (w, h) = (img.width as usize, img.height as usize);
    let mut rgb = vec![0u8; w * h * 3];
    for (i, &p) in img.pixels.iter().enumerate() {
        let c = img.palette.get(p as usize).copied().unwrap_or([255, 0, 255]);
        rgb[i * 3..i * 3 + 3].copy_from_slice(&c);
    }
    let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
    out.extend_from_slice(&rgb);
    std::fs::write("accuracy/comparisons/hand/orx.ppm", out).unwrap();
    println!("wrote accuracy/comparisons/hand/orx.ppm");
}
