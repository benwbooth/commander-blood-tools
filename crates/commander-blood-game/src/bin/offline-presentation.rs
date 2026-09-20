//! Export native logo, credits, or startup-cinematic segments without a window.

use anyhow::{Result, bail};
use commander_blood_game::native::bloodprg::PresentationResourceId;
use commander_blood_game::runtime::offline_export::{
    export_presentation, export_startup_cinematic,
};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if !(3..=4).contains(&args.len()) {
        bail!(
            "usage: offline-presentation IMPORTED_ASSETS opening|credits|cinematic OUTPUT_DIR [MAX_FRAMES]"
        );
    }
    let max_frames = args
        .get(3)
        .map(|s| s.to_string_lossy().parse())
        .transpose()?
        .unwrap_or(100_000);
    match args[1].to_str() {
        Some("opening") | Some("credits") => export_presentation(
            args[0].as_ref(),
            PresentationResourceId::new(u16::from(args[1] == "credits")),
            args[2].as_ref(),
            max_frames,
        ),
        Some("cinematic") => {
            export_startup_cinematic(args[0].as_ref(), args[2].as_ref(), max_frames)
        }
        _ => bail!("expected opening, credits, or cinematic"),
    }
}
