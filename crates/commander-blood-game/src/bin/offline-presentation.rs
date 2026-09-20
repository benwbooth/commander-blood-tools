//! Export complete opening or credits presentations without a window.

use anyhow::{Result, bail};
use commander_blood_game::native::bloodprg::PresentationResourceId;
use commander_blood_game::runtime::offline_export::export_presentation;

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if !(3..=4).contains(&args.len()) {
        bail!(
            "usage: offline-presentation IMPORTED_ASSETS opening|credits OUTPUT_DIR [MAX_FRAMES]"
        );
    }
    let line = match args[1].to_str() {
        Some("opening") => 0,
        Some("credits") => 1,
        _ => bail!("expected opening or credits"),
    };
    let max_frames = args
        .get(3)
        .map(|s| s.to_string_lossy().parse())
        .transpose()?
        .unwrap_or(100_000);
    export_presentation(
        args[0].as_ref(),
        PresentationResourceId::new(line),
        args[2].as_ref(),
        max_frames,
    )
}
