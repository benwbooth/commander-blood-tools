// quick: dump palette entries 0xFD..0xFF of several HNMs
use commander_blood_tools::hnm::HnmFile;
fn main() {
    for p in std::env::args().skip(1) {
        match HnmFile::open(std::path::Path::new(&p)) {
            Ok(h) => {
                let pal = h.palette;
                println!("{p}");
                for i in [0xFD, 0xFE, 0xFF] {
                    let c = pal[i];
                    println!("  0x{i:02X}: {:3} {:3} {:3}", c[0], c[1], c[2]);
                }
            }
            Err(e) => println!("{p}: {e}"),
        }
    }
}
