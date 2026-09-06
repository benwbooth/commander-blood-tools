//! Display-only English COD subtitles, bound to the active original resources.

use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use commander_blood_formats::code::{ScriptCodeOffset, ScriptDialect};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::native::bloodprg::{LoadedScriptProfile, ScriptProfileId};

const OPENING_ENGLISH: &str = include_str!("../../../../localization/big-bug-bang/en/script1.json");
const LINE_COLUMNS: usize = 34;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Translation {
    format: String,
    language: String,
    profile: String,
    cod_sha256: String,
    dic_sha256: String,
    messages: BTreeMap<String, Vec<String>>,
}

pub(super) struct SequelEnglishSubtitles {
    subtitles: BTreeMap<ScriptCodeOffset, Box<[u8]>>,
}

impl SequelEnglishSubtitles {
    pub(super) fn for_profile(profile: &LoadedScriptProfile) -> Result<Option<Self>> {
        if profile.code().dialect() != ScriptDialect::BigBugBang
            || profile.id() != ScriptProfileId::INITIAL
        {
            return Ok(None);
        }
        Self::from_sources(&profile.code().encode(), &profile.dictionary().encode())
    }

    fn from_sources(cod: &[u8], dic: &[u8]) -> Result<Option<Self>> {
        let translation: Translation = serde_json::from_str(OPENING_ENGLISH)?;
        ensure!(
            translation.format == "bbb-cod-display-translation-v1"
                && translation.language == "en"
                && translation.profile == "SCRIPT1",
            "unsupported bundled English subtitle catalog"
        );
        // Modified/other-edition resources must never receive address-matched text.
        if translation.cod_sha256 != format!("{:x}", Sha256::digest(cod))
            || translation.dic_sha256 != format!("{:x}", Sha256::digest(dic))
        {
            return Ok(None);
        }
        let subtitles = translation
            .messages
            .into_iter()
            .map(|(id, sections)| {
                let address = id
                    .strip_prefix("bbb.script1.cod.")
                    .context("invalid English subtitle site ID")?;
                let offset = usize::from_str_radix(address, 16)?;
                ensure!(
                    cod.get(offset) == Some(&166),
                    "English subtitle site is not A6"
                );
                let prose = sections
                    .first()
                    .context("English subtitle has no prose section")?;
                Ok((ScriptCodeOffset::new(offset), wrap_subtitle(prose)?))
            })
            .collect::<Result<_>>()?;
        Ok(Some(Self { subtitles }))
    }

    pub(super) fn subtitle(&self, instruction: ScriptCodeOffset) -> Option<Box<[u8]>> {
        self.subtitles.get(&instruction).cloned()
    }
}

fn wrap_subtitle(prose: &str) -> Result<Box<[u8]>> {
    ensure!(
        prose.is_ascii() && !prose.bytes().any(|b| b.is_ascii_control()) && !prose.contains('<'),
        "English subtitle requires printable, static ASCII prose"
    );
    let mut output = Vec::new();
    let mut columns = 0;
    for word in prose.split_whitespace() {
        ensure!(
            word.len() <= LINE_COLUMNS,
            "English subtitle word exceeds line width"
        );
        if columns != 0 {
            if columns + 1 + word.len() > LINE_COLUMNS {
                output.push(b'\r');
                columns = 0;
            } else {
                output.push(b' ');
                columns += 1;
            }
        }
        output.extend_from_slice(word.as_bytes());
        columns += word.len();
    }
    output.push(b'\r');
    Ok(output.into_boxed_slice())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::OriginalResourceStore;
    use crate::native::bloodprg::OriginalResourceCatalog;

    #[test]
    fn bundled_prose_wraps_without_losing_words_or_overflowing() {
        let translation: Translation = serde_json::from_str(OPENING_ENGLISH).unwrap();
        assert_eq!(translation.messages.len(), 89);
        for (id, sections) in translation.messages {
            let prose = &sections[0];
            let bytes = wrap_subtitle(prose).unwrap();
            assert_eq!(bytes.last(), Some(&b'\r'));
            assert!(
                bytes
                    .split(|b| *b == b'\r')
                    .all(|line| line.len() <= LINE_COLUMNS),
                "{id}"
            );
            assert_eq!(
                String::from_utf8(bytes.into())
                    .unwrap()
                    .split_whitespace()
                    .collect::<Vec<_>>(),
                prose.split_whitespace().collect::<Vec<_>>(),
                "{id}"
            );
        }
        assert!(wrap_subtitle("<state:8368>").is_err());
        assert!(wrap_subtitle(&"a".repeat(LINE_COLUMNS + 1)).is_err());
        assert!(wrap_subtitle("line\nbreak").is_err());
    }

    #[test]
    fn unrelated_resources_keep_the_original_language() {
        assert!(
            SequelEnglishSubtitles::from_sources(b"different COD", b"different DIC")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn english_reveal_finishes_at_the_translated_length() {
        use crate::native::bloodprg::{
            SubtitleFrameDraw, SubtitleRevealLine, SubtitleRevealOutcome, SubtitleRevealPhase,
            SubtitleRevealRenderer, SubtitleRevealState, TextPresentationState,
            update_subtitle_reveal,
        };
        #[derive(Default)]
        struct Lines {
            count: usize,
        }
        impl SubtitleRevealRenderer for Lines {
            fn draw_frame_primitive(&mut self, _: SubtitleFrameDraw) {
                panic!("no frame primitives supplied");
            }
            fn draw_subtitle_line(&mut self, line: SubtitleRevealLine<'_>) {
                assert!(line.text.len() <= LINE_COLUMNS);
                self.count += 1;
            }
        }
        let translation: Translation = serde_json::from_str(OPENING_ENGLISH).unwrap();
        for sections in translation.messages.values() {
            let subtitle = wrap_subtitle(&sections[0]).unwrap();
            let length = subtitle.len();
            let lines = subtitle.iter().filter(|b| **b == b'\r').count();
            let mut presentation = TextPresentationState {
                subtitle_text: subtitle,
                subtitle_reveal_cursor: Some(0),
                subtitle_display_active: true,
                ..Default::default()
            };
            let mut reveal = SubtitleRevealState {
                phase: SubtitleRevealPhase::Text,
                display_mode: true,
                text_speed_step: 2,
                ..Default::default()
            };
            for cursor in 0..=length {
                reveal.reveal_delay = 0;
                let mut renderer = Lines::default();
                let outcome =
                    update_subtitle_reveal(&mut presentation, &mut reveal, &[], &[], &mut renderer)
                        .unwrap();
                assert!(matches!(outcome, SubtitleRevealOutcome::TextFrame { .. }));
                assert_eq!(renderer.count, lines);
                assert_eq!(
                    presentation.subtitle_reveal_cursor,
                    Some((cursor + 1).min(length))
                );
                assert_eq!(presentation.dialogue_hold_complete, cursor == length);
            }
        }
    }

    #[test]
    #[ignore = "requires the user's imported Big Bug Bang resources"]
    fn authentic_opening_resources_select_english_and_reject_mutations() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../output/big-bug-bang/imported-assets/resources");
        let cod = std::fs::read(root.join("SCRIPT1.COD")).unwrap();
        let mut dic = std::fs::read(root.join("SCRIPT1.DIC")).unwrap();
        let catalog = SequelEnglishSubtitles::from_sources(&cod, &dic)
            .unwrap()
            .unwrap();
        assert_eq!(catalog.subtitles.len(), 89);
        assert_eq!(
            catalog
                .subtitle(ScriptCodeOffset::new(0x6ed))
                .unwrap()
                .as_ref(),
            b"Hello Commander, this is HONK,\ryour faithful onboard computer.\r"
        );
        assert!(catalog.subtitle(ScriptCodeOffset::new(0)).is_none());
        let executable = std::fs::read(root.join("../../disc/BLOOD2PG.EXE")).unwrap();
        let resources = OriginalResourceCatalog::decode_blood2pg(&executable).unwrap();
        let store = OriginalResourceStore::new(root.clone(), None, [], true);
        let mut manager = crate::native::bloodprg::ScriptProfileManager::new(
            crate::native::bloodprg::OriginalScriptProfileCatalog::decode_blood2pg(&executable)
                .unwrap(),
        );
        let mut cache = crate::native::bloodprg::OriginalResourceCache::new();
        manager
            .select(ScriptProfileId::INITIAL, &mut cache, &store, &resources)
            .unwrap();
        assert_eq!(manager.current().unwrap().code().encode(), cod);
        assert_eq!(manager.current().unwrap().dictionary().encode(), dic);
        assert_eq!(
            SequelEnglishSubtitles::for_profile(manager.current().unwrap())
                .unwrap()
                .unwrap()
                .subtitles,
            catalog.subtitles
        );
        let fonts = crate::game::GameVariant::BigBugBang
            .decode_fonts(&executable)
            .unwrap();
        for (site, subtitle) in &catalog.subtitles {
            let mut pixels = vec![0; 320 * 200];
            for (line_index, line) in subtitle.split_inclusive(|b| *b == b'\r').enumerate() {
                let drawn = crate::native::bloodprg::draw_subtitle_reveal_line(
                    &mut pixels,
                    &fonts,
                    line,
                    crate::native::bloodprg::FontPoint {
                        x: 10,
                        y: 8 + line_index as i32 * 8,
                    },
                    line.len() as i32,
                )
                .unwrap();
                assert_eq!(drawn.processed_characters, line.len() - 1, "{site:?}");
            }
            assert!(
                pixels.iter().any(|pixel| *pixel != 0),
                "blank English subtitle {site:?}"
            );
            assert!(
                pixels
                    .chunks_exact(320)
                    .all(|row| row[..10].iter().all(|p| *p == 0)
                        && row[282..].iter().all(|p| *p == 0)),
                "English subtitle escaped its horizontal bounds {site:?}"
            );
        }
        dic[0] ^= 1;
        assert!(
            SequelEnglishSubtitles::from_sources(&cod, &dic)
                .unwrap()
                .is_none()
        );
    }
}
