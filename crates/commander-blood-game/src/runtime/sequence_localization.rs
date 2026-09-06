//! Display-only sequence captions; source cues and playback clocks stay intact.

use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use commander_blood_formats::descript::DescriptSequenceSubtitle;
use commander_blood_formats::descript_database::{DescriptCommand, DescriptDatabase};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::game::GameVariant;

const ENGLISH: &str = include_str!("../../../../localization/big-bug-bang/en/sequences.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    format: String,
    language: String,
    descript_sha256: String,
    sequences: BTreeMap<String, Vec<Cue>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Cue {
    frame: u16,
    text: String,
}

struct Sequence {
    source: Vec<DescriptSequenceSubtitle>,
    display: Vec<DescriptSequenceSubtitle>,
}

#[derive(Default)]
pub(super) struct EnglishSequenceCaptions {
    sequences: Vec<Sequence>,
}

impl EnglishSequenceCaptions {
    pub(super) fn load(
        game: GameVariant,
        bytes: &[u8],
        database: &DescriptDatabase,
    ) -> Result<Self> {
        Self::from_catalog(game, bytes, database, ENGLISH)
    }

    fn from_catalog(
        game: GameVariant,
        bytes: &[u8],
        database: &DescriptDatabase,
        json: &str,
    ) -> Result<Self> {
        if game != GameVariant::BigBugBang {
            return Ok(Self::default());
        }
        let catalog: Catalog = serde_json::from_str(json)?;
        ensure!(
            catalog.format == "bbb-sequence-display-translation-v1" && catalog.language == "en",
            "unsupported sequence caption translation"
        );
        if format!("{:x}", Sha256::digest(bytes)) != catalog.descript_sha256 {
            return Ok(Self::default());
        }
        let mut result = Self::default();
        for (name, cues) in catalog.sequences {
            let record = database
                .lookup(name.as_bytes())
                .context("translated sequence is missing")?;
            let source: Vec<_> = record
                .commands()
                .iter()
                .filter_map(|command| match command {
                    DescriptCommand::SequenceSubtitle(cue) => Some(cue.clone()),
                    _ => None,
                })
                .collect();
            ensure!(
                !source.is_empty() && source.len() == cues.len(),
                "sequence caption count mismatch: {name}"
            );
            let mut display = Vec::with_capacity(cues.len());
            for (source, cue) in source.iter().zip(cues) {
                ensure!(
                    source.first_visible_frame() == cue.frame,
                    "sequence caption frame mismatch: {name}"
                );
                ensure!(
                    source.text().is_empty() == cue.text.is_empty(),
                    "sequence caption blank mismatch: {name}"
                );
                ensure!(
                    cue.text.bytes().all(|byte| (b' '..=b'~').contains(&byte)),
                    "sequence caption requires printable ASCII: {name}"
                );
                display.push(DescriptSequenceSubtitle::new(
                    cue.frame,
                    cue.text.into_bytes().into_boxed_slice(),
                ));
            }
            // Assets retain the complete cue stream, not its directory name.
            // Identical streams must never select conflicting translations.
            if let Some(existing) = result.sequences.iter().find(|entry| entry.source == source) {
                ensure!(
                    existing.display == display,
                    "ambiguous sequence caption translation: {name}"
                );
            } else {
                result.sequences.push(Sequence { source, display });
            }
        }
        Ok(result)
    }

    pub(super) fn display<'a>(
        &'a self,
        source: &'a [DescriptSequenceSubtitle],
    ) -> &'a [DescriptSequenceSubtitle] {
        self.sequences
            .iter()
            .find(|entry| entry.source == source)
            .map_or(source, |entry| entry.display.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::bloodprg::{
        CenteredSequenceSubtitleLine, SequenceSubtitlePlayback, SequenceSubtitleRenderer,
        present_sequence_subtitle,
    };

    struct BoundsRenderer;
    impl SequenceSubtitleRenderer for BoundsRenderer {
        type Error = anyhow::Error;
        fn visible_frame(&self) -> u16 {
            i16::MAX as u16
        }
        fn draw_centered_line(&mut self, line: CenteredSequenceSubtitleLine<'_>) -> Result<()> {
            ensure!(
                usize::from(line.position[0]) + line.text.len() * 8 <= 320,
                "caption exceeds horizontal bounds"
            );
            ensure!(
                line.position[1] + 8 <= 200,
                "caption exceeds vertical bounds"
            );
            Ok(())
        }
    }

    #[test]
    fn bundled_captions_fit_the_original_line_planner() {
        let catalog: Catalog = serde_json::from_str(ENGLISH).unwrap();
        for (name, cues) in catalog.sequences {
            for cue in cues {
                let caption = DescriptSequenceSubtitle::new(
                    cue.frame,
                    cue.text.into_bytes().into_boxed_slice(),
                );
                present_sequence_subtitle(
                    &[caption],
                    &mut SequenceSubtitlePlayback::default(),
                    &mut BoundsRenderer,
                )
                .unwrap_or_else(|error| panic!("{name} frame {}: {error}", cue.frame));
            }
        }
    }

    #[test]
    #[ignore = "requires the user's imported Big Bug Bang resources"]
    fn authentic_sequences_preserve_frames_blanks_and_source_identity() {
        let bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../output/big-bug-bang/imported-assets/resources/DESCRIPT.DES"),
        )
        .unwrap();
        let database = DescriptDatabase::parse(&bytes).unwrap();
        assert_eq!(
            database
                .records()
                .iter()
                .flat_map(|record| record.commands())
                .filter(|command| matches!(command, DescriptCommand::SequenceSubtitle(_)))
                .count(),
            706
        );
        let captions =
            EnglishSequenceCaptions::load(GameVariant::BigBugBang, &bytes, &database).unwrap();
        assert_eq!(captions.sequences.len(), 23);
        assert_eq!(
            captions
                .sequences
                .iter()
                .map(|entry| entry.display.len())
                .sum::<usize>(),
            215
        );
        for entry in &captions.sequences {
            assert_eq!(captions.display(&entry.source), entry.display);
            for (source, display) in entry.source.iter().zip(&entry.display) {
                assert_eq!(source.first_visible_frame(), display.first_visible_frame());
                assert_eq!(source.text().is_empty(), display.text().is_empty());
            }
            let mut changed = entry.source.clone();
            changed[0] = DescriptSequenceSubtitle::new(
                changed[0].first_visible_frame(),
                Box::from(b"changed".as_slice()),
            );
            assert_eq!(captions.display(&changed), changed);
        }
        let unbound: Vec<_> = database
            .lookup(b"2ppit")
            .unwrap()
            .commands()
            .iter()
            .filter_map(|command| match command {
                DescriptCommand::SequenceSubtitle(cue) => Some(cue.clone()),
                _ => None,
            })
            .collect();
        assert!(!unbound.is_empty());
        assert_eq!(captions.display(&unbound), unbound);
        for game in [GameVariant::CommanderBlood, GameVariant::BigBugBang] {
            let other_bytes = if game == GameVariant::BigBugBang {
                b"changed".as_slice()
            } else {
                &bytes
            };
            assert!(
                EnglishSequenceCaptions::load(game, other_bytes, &database)
                    .unwrap()
                    .sequences
                    .is_empty()
            );
        }
        for field in ["frame", "text"] {
            let mut invalid: serde_json::Value = serde_json::from_str(ENGLISH).unwrap();
            invalid["sequences"]["1ppit"][0][field] = if field == "frame" {
                11.into()
            } else {
                "".into()
            };
            assert!(
                EnglishSequenceCaptions::from_catalog(
                    GameVariant::BigBugBang,
                    &bytes,
                    &database,
                    &invalid.to_string()
                )
                .is_err()
            );
        }
    }
}
