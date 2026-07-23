fn main() {
    let d = std::fs::read("output/_tmp_iso/FRIGO.FD").unwrap();
    let img = commander_blood_tools::lbm::decode_lbm(&d).expect("lbm");
    println!("FRIGO.FD: {}x{}", img.width, img.height);
}
