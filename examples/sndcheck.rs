use commander_blood_tools::snd::SndBank;
fn main() {
    for p in std::env::args().skip(1) {
        match SndBank::read(std::path::Path::new(&p)) {
            Ok(b) => {
                println!("{p}: {} clips", b.clip_count());
                for i in 5..b.clip_count().min(18) {
                    if let Some(c) = b.clip(i) {
                        println!("  clip {i}: {} bytes @ {} Hz (~{:.2}s)", c.pcm.len(), c.sample_rate, c.pcm.len() as f32 / c.sample_rate.max(1) as f32);
                    }
                }
            }
            Err(e) => println!("{p}: {e}"),
        }
    }
}
