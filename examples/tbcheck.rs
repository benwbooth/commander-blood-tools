//! Direct decoder check: unpack TB.BIG frame 45 via the port's BridgePanorama and
//! compare byte-for-byte against the interpreter's hub screen indices.

fn main() {
    let data = std::fs::read("output/_tmp_iso/TB.BIG").unwrap();
    let pan = commander_blood_tools::tbbig::BridgePanorama::parse(data).unwrap();
    let hub = std::fs::read("accuracy/captures/hub_indices.bin").unwrap();
    let mut fb = vec![0u8; 320 * 200];
    pan.unpack_frame_over(45, &mut fb, false).unwrap();
    let matches = fb.iter().zip(&hub).filter(|(a, b)| a == b).count();
    println!("port frame 45 vs hub indices: {}/64000 = {:.2}%", matches, matches as f64 / 640.0);
    // left half only (right side has the live CANCEL/overlay differences)
    let lm = (0..200)
        .flat_map(|y| (0..150).map(move |x| y * 320 + x))
        .filter(|&i| fb[i] == hub[i])
        .count();
    println!("left half: {}/30000 = {:.2}%", lm, lm as f64 / 300.0);
}
