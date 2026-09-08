//! Location captions only; DESCRIPT lookup names and presentation state stay native.

use anyhow::{Context, Result, ensure};
use commander_blood_formats::descript_database::{DescriptCommand, DescriptDatabase};
use sha2::{Digest, Sha256};

use crate::game::GameVariant;
use crate::native::bloodprg::TextPresentationState;

const DESCRIPT_SHA256: &str = "3ffb0122c13ea951fa9a24df3ddbbeb383a5da91f645d8833a2de1ee760e913d";
const CAPTIONS: &[(&[u8], &[u8], &[u8])] = &[
    (b"Arche", b"Arche:\r", b"Ark:\r"),
    (
        b"Ekatomb",
        b"Ekatomb: un jour on y tombe...\r",
        b"Ekatomb: we'll all end up here...\r",
    ),
];

#[derive(Clone, Default)]
pub(super) struct EnglishDescriptCaptions {
    enabled: bool,
}

impl EnglishDescriptCaptions {
    pub(super) fn load(
        game: GameVariant,
        bytes: &[u8],
        database: &DescriptDatabase,
    ) -> Result<Self> {
        if game != GameVariant::BigBugBang
            || format!("{:x}", Sha256::digest(bytes)) != DESCRIPT_SHA256
        {
            return Ok(Self::default());
        }
        for &(name, source, _) in CAPTIONS {
            let record = database
                .lookup(name)
                .context("missing translated location")?;
            let captions: Vec<_> = record
                .commands()
                .iter()
                .filter_map(|command| {
                    if let DescriptCommand::Caption(caption) = command {
                        Some(caption.text())
                    } else {
                        None
                    }
                })
                .collect();
            ensure!(
                captions == [source],
                "translated location caption source changed"
            );
        }
        Ok(Self { enabled: true })
    }

    pub(super) fn apply(&self, name: &[u8], text: &mut TextPresentationState) {
        if !self.enabled {
            return;
        }
        if let Some((_, _, english)) = CAPTIONS
            .iter()
            .find(|(record, source, _)| *record == name && *source == text.subtitle_text.as_ref())
        {
            text.subtitle_text = (*english).into();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::bloodprg::{SubtitleRevealLine, stage_descript_caption};

    #[test]
    #[ignore = "requires the user's imported Big Bug Bang resources"]
    fn authentic_captions_render_without_changing_presentation_state() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../output/big-bug-bang/imported-assets");
        let bytes = std::fs::read(root.join("resources/DESCRIPT.DES")).unwrap();
        let database = DescriptDatabase::parse(&bytes).unwrap();
        let english =
            EnglishDescriptCaptions::load(GameVariant::BigBugBang, &bytes, &database).unwrap();
        let exe = std::fs::read(root.join("../disc/BLOOD2PG.EXE")).unwrap();
        let fonts = GameVariant::BigBugBang.decode_fonts(&exe).unwrap();
        let colors = GameVariant::BigBugBang
            .decode_default_vga_palette(&exe)
            .unwrap();
        let ui = crate::ui::DialogueUiAssets::import(&fonts, &colors).unwrap();
        let mut count = 0;
        let mut changed = 0;
        for record in database.records() {
            for command in record.commands() {
                let DescriptCommand::Caption(caption) = command else {
                    continue;
                };
                let mut text = TextPresentationState::default();
                stage_descript_caption(caption, &mut text);
                let before = text.clone();
                english.apply(record.name(), &mut text);
                if text.subtitle_text != before.subtitle_text {
                    changed += 1;
                }
                let display = text.subtitle_text.clone();
                text.subtitle_text = before.subtitle_text.clone();
                assert_eq!(text, before);
                let mut pixels = crate::ui::RgbaUiOverlay::new(320, 200);
                for (index, line) in display.split(|byte| *byte == b'\r').enumerate() {
                    assert!(line.len() <= 34);
                    ui.draw_line(
                        &mut pixels,
                        SubtitleRevealLine {
                            text: line,
                            byte_offset: 0,
                            reveal_cursor: line.len() + 2,
                            position: [10, 8 + index as u16 * 8],
                        },
                    )
                    .unwrap();
                }
                let visible = display.iter().any(|byte| !byte.is_ascii_whitespace());
                assert_eq!(
                    pixels.pixels().chunks_exact(4).any(|pixel| pixel[3] != 0),
                    visible
                );
                count += 1;
            }
        }
        assert_eq!(count, 75);
        assert_eq!(changed, 2);
        for (game, source) in [
            (GameVariant::CommanderBlood, bytes.as_slice()),
            (GameVariant::BigBugBang, b"changed".as_slice()),
        ] {
            let disabled = EnglishDescriptCaptions::load(game, source, &database).unwrap();
            assert!(!disabled.enabled);
        }
        for &(name, source, display) in CAPTIONS {
            let mut text = TextPresentationState::default();
            text.subtitle_text = source.into();
            english.apply(b"unrelated", &mut text);
            assert_eq!(text.subtitle_text.as_ref(), source);
            text.subtitle_text = b"changed".as_slice().into();
            english.apply(name, &mut text);
            assert_eq!(text.subtitle_text.as_ref(), b"changed");
            text.subtitle_text = source.into();
            english.apply(name, &mut text);
            assert_eq!(text.subtitle_text.as_ref(), display);
            english.apply(name, &mut text);
            assert_eq!(text.subtitle_text.as_ref(), display);
        }
    }
}
