//! Display-only English COD subtitles and choices, bound to original resources.

use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use commander_blood_formats::code::{
    ScriptCodeOffset, ScriptDialect, decode_script_code_for_dialect,
};
use commander_blood_formats::instruction::{
    ScriptTextStateNumber, ScriptTextWord, decode_script_text,
};
use commander_blood_formats::script::{ScriptWordId, decode_script_dictionary};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::native::bloodprg::{InlineMenuDisplayWord, LoadedScriptProfile};

const OPENING_ENGLISH: &str = include_str!("../../../../localization/big-bug-bang/en/script1.json");
const SCRIPT2_ENGLISH: &str = include_str!("../../../../localization/big-bug-bang/en/script2.json");
const SCRIPT3_ENGLISH: &str = include_str!("../../../../localization/big-bug-bang/en/script3.json");
const SCRIPT4_ENGLISH: &str = include_str!("../../../../localization/big-bug-bang/en/script4.json");
const SCRIPT5_ENGLISH: &str = include_str!("../../../../localization/big-bug-bang/en/script5.json");
const SCRIPT7_ENGLISH: &str = include_str!("../../../../localization/big-bug-bang/en/script7.json");
const SCRIPT8_ENGLISH: &str = include_str!("../../../../localization/big-bug-bang/en/script8.json");
const SCRIPT9_ENGLISH: &str = include_str!("../../../../localization/big-bug-bang/en/script9.json");
const SCRIPT10_ENGLISH: &str =
    include_str!("../../../../localization/big-bug-bang/en/script10.json");
const SCRIPT12_ENGLISH: &str =
    include_str!("../../../../localization/big-bug-bang/en/script12.json");
const SCRIPT14_ENGLISH: &str =
    include_str!("../../../../localization/big-bug-bang/en/script14.json");
const SCRIPT15_ENGLISH: &str =
    include_str!("../../../../localization/big-bug-bang/en/script15.json");
const SCRIPT16_ENGLISH: &str =
    include_str!("../../../../localization/big-bug-bang/en/script16.json");
const SCRIPT17_ENGLISH: &str =
    include_str!("../../../../localization/big-bug-bang/en/script17.json");
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
    choices: BTreeMap<ScriptCodeOffset, (Vec<ScriptWordId>, Vec<Box<[u8]>>)>,
    menus: BTreeMap<ScriptCodeOffset, InlineMenuTranslation>,
}

struct InlineMenuTranslation {
    source: Box<[ScriptTextWord]>,
    display: Box<[InlineMenuDisplayWord]>,
}

impl SequelEnglishSubtitles {
    pub(super) fn for_profile(profile: &LoadedScriptProfile) -> Result<Option<Self>> {
        if profile.code().dialect() != ScriptDialect::BigBugBang {
            return Ok(None);
        }
        let (name, catalog) = match profile.id().value() {
            0 => ("SCRIPT1", OPENING_ENGLISH),
            1 => ("SCRIPT2", SCRIPT2_ENGLISH),
            2 => ("SCRIPT3", SCRIPT3_ENGLISH),
            3 => ("SCRIPT4", SCRIPT4_ENGLISH),
            4 => ("SCRIPT5", SCRIPT5_ENGLISH),
            6 => ("SCRIPT7", SCRIPT7_ENGLISH),
            7 => ("SCRIPT8", SCRIPT8_ENGLISH),
            8 => ("SCRIPT9", SCRIPT9_ENGLISH),
            9 => ("SCRIPT10", SCRIPT10_ENGLISH),
            11 => ("SCRIPT12", SCRIPT12_ENGLISH),
            13 => ("SCRIPT14", SCRIPT14_ENGLISH),
            14 => ("SCRIPT15", SCRIPT15_ENGLISH),
            15 => ("SCRIPT16", SCRIPT16_ENGLISH),
            16 => ("SCRIPT17", SCRIPT17_ENGLISH),
            _ => return Ok(None),
        };
        Self::from_catalog(
            &profile.code().encode(),
            &profile.dictionary().encode(),
            name,
            catalog,
        )
    }

    #[cfg(test)]
    fn from_sources(cod: &[u8], dic: &[u8]) -> Result<Option<Self>> {
        Self::from_catalog(cod, dic, "SCRIPT1", OPENING_ENGLISH)
    }

    fn from_catalog(cod: &[u8], dic: &[u8], name: &str, catalog: &str) -> Result<Option<Self>> {
        let translation: Translation = serde_json::from_str(catalog)?;
        ensure!(
            translation.format == "bbb-cod-display-translation-v1"
                && translation.language == "en"
                && translation.profile == name,
            "unsupported bundled English subtitle catalog"
        );
        // Modified/other-edition resources must never receive address-matched text.
        if translation.cod_sha256 != format!("{:x}", Sha256::digest(cod))
            || translation.dic_sha256 != format!("{:x}", Sha256::digest(dic))
        {
            return Ok(None);
        }
        let code = decode_script_code_for_dialect(cod, ScriptDialect::BigBugBang)?;
        let dictionary = decode_script_dictionary(dic)?;
        let mut choices = BTreeMap::new();
        let mut menus = BTreeMap::new();
        let mut subtitles = BTreeMap::new();
        let prefix = format!("bbb.{}.cod.", name.to_ascii_lowercase());
        for (id, sections) in translation.messages {
            let address = id
                .strip_prefix(&prefix)
                .context("invalid English subtitle site ID")?;
            let offset = usize::from_str_radix(address, 16)?;
            ensure!(
                cod.get(offset) == Some(&166),
                "English subtitle site is not A6"
            );
            let prose = sections
                .first()
                .context("English subtitle has no prose section")?;
            let token = code
                .tokens()
                .iter()
                .find(|token| token.source_offset().index() == offset)
                .context("English text site is not an instruction boundary")?;
            let text = decode_script_text(token, &dictionary)?;
            if sections.len() > 1 {
                let source = text
                    .words
                    .split(|word| *word == ScriptTextWord::SectionSeparator)
                    .skip(1)
                    .collect::<Vec<_>>();
                ensure!(
                    source.len() == sections.len() - 1,
                    "English choice section mismatch"
                );
                let mut words = Vec::new();
                let mut labels = Vec::new();
                for (source, translated) in source.into_iter().zip(sections.iter().skip(1)) {
                    if source == [ScriptTextWord::InventoryChoices] {
                        ensure!(
                            sections.len() == 2 && translated == "<inventory_choices>",
                            "English inventory choices must preserve the sole generator section"
                        );
                        continue;
                    }
                    let translated = translated.split_whitespace().collect::<Vec<_>>();
                    ensure!(
                        source.len() == translated.len(),
                        "English choice count mismatch"
                    );
                    for (word, label) in source.iter().zip(translated) {
                        let ScriptTextWord::Dictionary(word) = word else {
                            anyhow::bail!("English choice requires a static dictionary word");
                        };
                        ensure!(
                            label.is_ascii() && !label.bytes().any(|byte| byte.is_ascii_control()),
                            "English choice requires printable ASCII"
                        );
                        words.push(*word);
                        labels.push(label.as_bytes().into());
                    }
                }
                if !words.is_empty() {
                    choices.insert(ScriptCodeOffset::new(offset), (words, labels));
                }
            }
            let display = parse_display_words(prose, &text.words)?;
            if display
                .iter()
                .any(|word| matches!(word, InlineMenuDisplayWord::StateNumber(_)))
            {
                ensure!(
                    !text.control.emits_spoken_text(),
                    "numeric English prose requires a menu-only source"
                );
            } else {
                let subtitle = if prose.is_empty() {
                    Box::from([])
                } else {
                    wrap_subtitle(prose)?
                };
                subtitles.insert(ScriptCodeOffset::new(offset), subtitle);
            }
            menus.insert(
                ScriptCodeOffset::new(offset),
                InlineMenuTranslation {
                    source: text.words,
                    display,
                },
            );
        }
        Ok(Some(Self {
            subtitles,
            choices,
            menus,
        }))
    }

    pub(super) fn subtitle(&self, instruction: ScriptCodeOffset) -> Option<Box<[u8]>> {
        self.subtitles.get(&instruction).cloned()
    }

    pub(super) fn choice_labels(
        &self,
        instruction: ScriptCodeOffset,
        words: &[ScriptWordId],
    ) -> Option<&[Box<[u8]>]> {
        let (expected, labels) = self.choices.get(&instruction)?;
        (expected.as_slice() == words).then_some(labels.as_slice())
    }

    pub(super) fn menu_words(
        &self,
        instruction: ScriptCodeOffset,
        source: &[ScriptTextWord],
    ) -> Option<&[InlineMenuDisplayWord]> {
        let menu = self.menus.get(&instruction)?;
        (menu.source.as_ref() == source).then_some(menu.display.as_ref())
    }
}

fn parse_display_words(
    prose: &str,
    source: &[ScriptTextWord],
) -> Result<Box<[InlineMenuDisplayWord]>> {
    if prose.is_empty() {
        ensure!(
            source.is_empty(),
            "English empty text must match an empty source"
        );
        return Ok(Box::from([]));
    }
    ensure!(
        !prose.is_empty() && prose.is_ascii() && !prose.bytes().any(|byte| byte.is_ascii_control()),
        "English menu requires printable ASCII"
    );
    let mut expected = Vec::new();
    for word in source
        .iter()
        .take_while(|word| **word != ScriptTextWord::SectionSeparator)
    {
        match word {
            ScriptTextWord::StateNumber(number) => expected.push(*number),
            ScriptTextWord::InventoryChoices => {
                anyhow::bail!("English inventory generator translation is not supported")
            }
            _ => {}
        }
    }
    let mut actual = Vec::new();
    let words = prose
        .split_whitespace()
        .map(|word| {
            if let Some(offset) = word
                .strip_prefix("<state:")
                .and_then(|word| word.strip_suffix('>'))
            {
                let number = ScriptTextStateNumber::decode(
                    offset
                        .parse::<u16>()
                        .context("invalid English state-number marker")?,
                );
                actual.push(number);
                Ok(InlineMenuDisplayWord::StateNumber(number))
            } else {
                ensure!(!word.contains(['<', '>']), "invalid English menu marker");
                Ok(InlineMenuDisplayWord::Literal(Box::from(word.as_bytes())))
            }
        })
        .collect::<Result<Box<[_]>>>()?;
    ensure!(!words.is_empty(), "English menu has no display words");
    ensure!(
        actual == expected,
        "English menu must preserve the ordered live-number sources"
    );
    Ok(words)
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
    #[test]
    fn translated_menu_preserves_live_number_sources_and_order() {
        use super::*;
        let credits = ScriptTextStateNumber::decode(7922);
        let bionium = ScriptTextStateNumber::decode(8362);
        let source = [
            ScriptTextWord::StateNumber(credits),
            ScriptTextWord::StateNumber(bionium),
        ];
        let words =
            parse_display_words("Credits: <state:7922> Bionium: <state:8362>", &source).unwrap();
        assert_eq!(words[1], InlineMenuDisplayWord::StateNumber(credits));
        assert_eq!(words[3], InlineMenuDisplayWord::StateNumber(bionium));
        for invalid in [
            "Credits: 100 Bionium: 200",
            "<state:8362> <state:7922>",
            "<state:7922> <state:7922>",
            "<state:7922>",
            "<state:7922> <state:8362> <state:8362>",
            "<state:no> <state:8362>",
            "<state:65536> <state:8362>",
            "<state:7922>, <state:8362>",
        ] {
            assert!(parse_display_words(invalid, &source).is_err(), "{invalid}");
        }
        assert!(parse_display_words("<inventory>", &[ScriptTextWord::InventoryChoices]).is_err());
        assert!(parse_display_words("   ", &[]).is_err());
    }

    use super::*;
    use crate::assets::OriginalResourceStore;
    use crate::native::bloodprg::{OriginalResourceCatalog, ScriptProfileId};

    #[test]
    fn empty_translation_requires_an_empty_authored_stream() {
        assert!(parse_display_words("", &[]).unwrap().is_empty());
        assert!(parse_display_words("", &[ScriptTextWord::SectionSeparator]).is_err());
        assert!(parse_display_words("", &[ScriptTextWord::InventoryChoices]).is_err());
        assert!(parse_display_words(" ", &[]).is_err());
    }

    #[test]
    #[ignore = "requires the user's imported Big Bug Bang resources"]
    fn authentic_script5_binds_choices_dynamic_counts_and_rgb_text() {
        check_profile_rgb_text(
            4,
            "SCRIPT5",
            SCRIPT5_ENGLISH,
            216,
            7,
            &[0x1050, 0x16b5],
            &[0x1537, 0x1ea6],
        );
    }

    #[test]
    #[ignore = "requires the user's imported Big Bug Bang resources"]
    fn authentic_script17_binds_choices_dynamic_counts_and_rgb_text() {
        check_profile_rgb_text(
            16,
            "SCRIPT17",
            SCRIPT17_ENGLISH,
            132,
            5,
            &[0x0e2d, 0x1310],
            &[0x127a, 0x175d],
        );
    }

    #[test]
    #[ignore = "requires the user's imported Big Bug Bang resources"]
    fn authentic_script15_binds_dynamic_counts_inventory_and_rgb_text() {
        check_profile_rgb_text(
            14,
            "SCRIPT15",
            SCRIPT15_ENGLISH,
            102,
            0,
            &[0x079b, 0x0c9e, 0x1177],
            &[0x0c4d, 0x1126, 0x15f4],
        );
    }

    #[test]
    #[ignore = "requires the user's imported Big Bug Bang resources"]
    fn authentic_script7_binds_choices_dynamic_counts_and_rgb_text() {
        check_profile_rgb_text(
            6,
            "SCRIPT7",
            SCRIPT7_ENGLISH,
            140,
            3,
            &[0x07d2, 0x0d74, 0x1351],
            &[0x0ccb, 0x1288, 0x180e],
        );
    }

    #[test]
    #[ignore = "requires the user's imported Big Bug Bang resources"]
    fn authentic_script9_binds_choices_dynamic_counts_and_rgb_text() {
        check_profile_rgb_text(
            8,
            "SCRIPT9",
            SCRIPT9_ENGLISH,
            176,
            3,
            &[0x110a, 0x1544, 0x197b],
            &[0x14ed, 0x1924, 0x1d59],
        );
    }

    #[test]
    #[ignore = "requires the user's imported Big Bug Bang resources"]
    fn authentic_script16_binds_choices_dynamic_counts_and_rgb_text() {
        check_profile_rgb_text(
            15,
            "SCRIPT16",
            SCRIPT16_ENGLISH,
            209,
            7,
            &[0x1088, 0x15f7, 0x1b6a],
            &[0x15a4, 0x1b17, 0x2088],
        );
    }

    #[test]
    #[ignore = "requires the user's imported Big Bug Bang resources"]
    fn authentic_script8_binds_choices_dynamic_counts_and_rgb_text() {
        check_profile_rgb_text(
            7,
            "SCRIPT8",
            SCRIPT8_ENGLISH,
            288,
            3,
            &[
                0x06f1, 0x0b84, 0x0f49, 0x130e, 0x179d, 0x1b62, 0x1f27, 0x23b4, 0x2779,
            ],
            &[
                0x0b33, 0x0ef8, 0x12bd, 0x174c, 0x1b11, 0x1ed6, 0x2363, 0x2728, 0x2aed,
            ],
        );
        let translation: Translation = serde_json::from_str(SCRIPT8_ENGLISH).unwrap();
        assert_eq!(
            translation.messages["bbb.script8.cod.00001fe5"],
            ["TELEPORTING DECODER INTO CRYOBOX"],
            "retain the authored decoder caption even though Morning_Oil offers gifts"
        );
    }

    #[test]
    #[ignore = "requires the user's imported Big Bug Bang resources"]
    fn authentic_script12_binds_choices_dynamic_counts_and_rgb_text() {
        check_profile_rgb_text(
            11,
            "SCRIPT12",
            SCRIPT12_ENGLISH,
            302,
            8,
            &[0x0a18, 0x20c8],
            &[0x1e75, 0x2a6b],
        );
    }

    #[test]
    #[ignore = "requires the user's imported Big Bug Bang resources"]
    fn authentic_script10_binds_choices_inventory_and_rgb_text() {
        check_profile_rgb_text(9, "SCRIPT10", SCRIPT10_ENGLISH, 303, 8, &[], &[0x1bef]);
    }

    #[test]
    #[ignore = "requires the user's imported Big Bug Bang resources"]
    fn authentic_script14_binds_letter_choices_dynamic_counts_and_rgb_text() {
        check_profile_rgb_text(
            13,
            "SCRIPT14",
            SCRIPT14_ENGLISH,
            377,
            10,
            &[0x0a7b, 0x1dcb, 0x2cfa],
            &[0x1a44, 0x2a11, 0x34f7],
        );
        let translation: Translation = serde_json::from_str(SCRIPT14_ENGLISH).unwrap();
        for (site, letters) in [
            ("bbb.script14.cod.00000dff", "s u p e r t r o m p"),
            ("bbb.script14.cod.00000e6d", "e m a s c u l a t o r"),
            ("bbb.script14.cod.00000ed3", "p o u r q u o i m o i"),
        ] {
            assert_eq!(translation.messages[site][1], letters);
        }
    }

    fn check_profile_rgb_text(
        profile_id: u8,
        name: &str,
        english: &str,
        sites: usize,
        choices: usize,
        dynamic_sites: &[usize],
        inventory_sites: &[usize],
    ) {
        use crate::native::bloodprg::*;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../output/big-bug-bang/imported-assets/resources");
        let cod = std::fs::read(root.join(format!("{name}.COD"))).unwrap();
        let dic = std::fs::read(root.join(format!("{name}.DIC"))).unwrap();
        let executable = std::fs::read(root.join("../../disc/BLOOD2PG.EXE")).unwrap();
        let resources = OriginalResourceCatalog::decode_blood2pg(&executable).unwrap();
        let store = OriginalResourceStore::new(root, None, [], true);
        let mut manager = ScriptProfileManager::new(
            OriginalScriptProfileCatalog::decode_blood2pg(&executable).unwrap(),
        );
        let mut cache = OriginalResourceCache::new();
        manager
            .select(ScriptProfileId::INITIAL, &mut cache, &store, &resources)
            .unwrap();
        manager
            .select(
                ScriptProfileId::new_for_dialect(profile_id, ScriptDialect::BigBugBang).unwrap(),
                &mut cache,
                &store,
                &resources,
            )
            .unwrap();
        let profile = manager.current().unwrap();
        let catalog = SequelEnglishSubtitles::for_profile(profile)
            .unwrap()
            .unwrap();
        assert_eq!(profile.code().encode(), cod);
        assert_eq!(profile.dictionary().encode(), dic);
        assert_eq!(catalog.menus.len(), sites);
        assert_eq!(catalog.subtitles.len(), sites - dynamic_sites.len());
        assert_eq!(catalog.choices.len(), choices);
        for &address in dynamic_sites {
            let site = ScriptCodeOffset::new(address);
            let menu = &catalog.menus[&site];
            assert!(catalog.subtitle(site).is_none());
            assert_eq!(
                menu.display
                    .iter()
                    .filter(|word| matches!(word, InlineMenuDisplayWord::StateNumber(_)))
                    .count(),
                1
            );
            assert_eq!(
                catalog.menu_words(site, &menu.source),
                Some(menu.display.as_ref())
            );
        }
        for &address in inventory_sites {
            let site = ScriptCodeOffset::new(address);
            let menu = &catalog.menus[&site];
            assert_eq!(menu.source.last(), Some(&ScriptTextWord::InventoryChoices));
            assert_eq!(
                catalog.menu_words(site, &menu.source),
                Some(menu.display.as_ref())
            );
            assert!(!catalog.choices.contains_key(&site));
        }
        for (site, (words, labels)) in &catalog.choices {
            assert_eq!(catalog.choice_labels(*site, words), Some(labels.as_slice()));
            let reversed = words.iter().rev().copied().collect::<Vec<_>>();
            if reversed != *words {
                assert!(catalog.choice_labels(*site, &reversed).is_none());
            } else {
                assert_eq!(
                    catalog.choice_labels(*site, &reversed),
                    Some(labels.as_slice())
                );
            }
            assert!(
                words
                    .iter()
                    .all(|word| profile.dictionary().word(*word).is_some())
            );
        }
        let fonts = crate::game::GameVariant::BigBugBang
            .decode_fonts(&executable)
            .unwrap();
        let colors = crate::game::GameVariant::BigBugBang
            .decode_default_vga_palette(&executable)
            .unwrap();
        let ui = crate::ui::DialogueUiAssets::import(&fonts, &colors).unwrap();
        let mut pixels = crate::ui::RgbaUiOverlay::new(320, 200);
        for (site, text) in &catalog.subtitles {
            pixels.clear();
            for (index, line) in text
                .split(|byte| *byte == b'\r')
                .filter(|line| !line.is_empty())
                .enumerate()
            {
                assert!(line.len() <= LINE_COLUMNS, "{site:?}");
                assert!(8 + index * 8 + 8 <= 200, "{site:?}");
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
            let has_pixels = pixels.pixels().chunks_exact(4).any(|pixel| pixel[3] != 0);
            if catalog.menus[site].source.is_empty() {
                assert!(text.is_empty(), "unexpected text at empty source {site:?}");
                assert!(!has_pixels, "unexpected pixels at empty source {site:?}");
            } else {
                assert!(has_pixels, "blank {site:?}");
            }
            assert!(
                pixels
                    .pixels()
                    .chunks_exact(320 * 4)
                    .all(|row| row[..10 * 4].iter().all(|value| *value == 0)
                        && row[282 * 4..].iter().all(|value| *value == 0)),
                "overflow {site:?}"
            );
        }
        assert!(
            SequelEnglishSubtitles::from_catalog(b"changed", &dic, name, english)
                .unwrap()
                .is_none()
        );
        assert!(
            SequelEnglishSubtitles::from_catalog(&cod, b"changed", name, english)
                .unwrap()
                .is_none()
        );
        assert!(
            SequelEnglishSubtitles::from_catalog(&cod, &dic, "WRONG_PROFILE", english).is_err()
        );
    }

    #[test]
    #[ignore = "requires the user's imported Big Bug Bang resources"]
    fn authentic_opening_renders_all_rgb_subtitles() {
        check_profile_rgb_text(0, "SCRIPT1", OPENING_ENGLISH, 89, 3, &[], &[]);
    }

    #[test]
    #[ignore = "requires the user's imported Big Bug Bang resources"]
    fn authentic_script4_preserves_live_numbers_and_inventory() {
        check_profile_rgb_text(
            3,
            "SCRIPT4",
            SCRIPT4_ENGLISH,
            660,
            11,
            &[0x1096, 0x2b01, 0x4138, 0x4191],
            &[0x1d8d, 0x3e11, 0x561d],
        );
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../output/big-bug-bang/imported-assets/resources");
        let cod = std::fs::read(root.join("SCRIPT4.COD")).unwrap();
        let dic = std::fs::read(root.join("SCRIPT4.DIC")).unwrap();
        let catalog = SequelEnglishSubtitles::from_catalog(&cod, &dic, "SCRIPT4", SCRIPT4_ENGLISH)
            .unwrap()
            .unwrap();
        assert_eq!(catalog.menus.len(), 660);
        assert_eq!(catalog.subtitles.len(), 656);
        for address in [0x1096, 0x2b01, 0x4138, 0x4191] {
            let site = ScriptCodeOffset::new(address);
            let menu = &catalog.menus[&site];
            assert!(catalog.subtitle(site).is_none());
            assert!(
                menu.display
                    .iter()
                    .any(|word| matches!(word, InlineMenuDisplayWord::StateNumber(_)))
            );
            assert_eq!(
                catalog.menu_words(site, &menu.source),
                Some(menu.display.as_ref())
            );
        }
        for address in [0x1d8d, 0x3e11, 0x561d] {
            let site = ScriptCodeOffset::new(address);
            let menu = &catalog.menus[&site];
            assert_eq!(menu.source.last(), Some(&ScriptTextWord::InventoryChoices));
            assert_eq!(
                catalog.menu_words(site, &menu.source),
                Some(menu.display.as_ref())
            );
            assert!(!catalog.choices.contains_key(&site));
        }
        for text in catalog.subtitles.values() {
            assert!(
                text.split(|b| *b == b'\r')
                    .all(|line| line.len() <= LINE_COLUMNS)
            );
        }
        assert!(
            SequelEnglishSubtitles::from_catalog(&cod, &dic, "SCRIPT3", SCRIPT4_ENGLISH).is_err()
        );
        assert!(
            SequelEnglishSubtitles::from_catalog(b"changed", &dic, "SCRIPT4", SCRIPT4_ENGLISH)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    #[ignore = "requires the user's imported Big Bug Bang resources"]
    fn authentic_script3_preserves_inventory_generators_and_empty_text() {
        check_profile_rgb_text(
            2,
            "SCRIPT3",
            SCRIPT3_ENGLISH,
            779,
            30,
            &[0x1eb6, 0x3e93, 0x5b54],
            &[0x17c9, 0x3379, 0x4c4d, 0x65ad],
        );
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../output/big-bug-bang/imported-assets/resources");
        let cod = std::fs::read(root.join("SCRIPT3.COD")).unwrap();
        let dic = std::fs::read(root.join("SCRIPT3.DIC")).unwrap();
        let catalog = SequelEnglishSubtitles::from_catalog(&cod, &dic, "SCRIPT3", SCRIPT3_ENGLISH)
            .unwrap()
            .unwrap();
        assert_eq!(catalog.menus.len(), 779);
        assert_eq!(catalog.subtitles.len(), 776);
        for address in [0x17c9, 0x3379, 0x4c4d, 0x65ad] {
            let site = ScriptCodeOffset::new(address);
            let menu = &catalog.menus[&site];
            assert_eq!(menu.source.last(), Some(&ScriptTextWord::InventoryChoices));
            assert_eq!(
                catalog.menu_words(site, &menu.source),
                Some(menu.display.as_ref())
            );
            assert_eq!(
                menu.display.as_ref(),
                &[InlineMenuDisplayWord::Literal(Box::from(
                    b"GIVE:".as_slice()
                ))]
            );
            assert!(!catalog.choices.contains_key(&site));
            assert!(catalog.choice_labels(site, &[]).is_none());
        }
        let empty = ScriptCodeOffset::new(0x65bd);
        assert!(catalog.menus[&empty].source.is_empty());
        assert!(catalog.menus[&empty].display.is_empty());
        assert!(catalog.subtitle(empty).unwrap().is_empty());
        for replacement in ["GIFT", "<inventory_choices> EXTRA", ""] {
            let mut invalid: serde_json::Value = serde_json::from_str(SCRIPT3_ENGLISH).unwrap();
            invalid["messages"]["bbb.script3.cod.000017c9"][1] = replacement.into();
            assert!(
                SequelEnglishSubtitles::from_catalog(&cod, &dic, "SCRIPT3", &invalid.to_string())
                    .is_err()
            );
        }
        for text in catalog.subtitles.values() {
            assert!(
                text.split(|b| *b == b'\r')
                    .all(|line| line.len() <= LINE_COLUMNS)
            );
        }
    }

    #[test]
    #[ignore = "requires the user's imported Big Bug Bang resources"]
    fn authentic_script2_catalog_preserves_all_sites_and_live_numbers() {
        check_profile_rgb_text(
            1,
            "SCRIPT2",
            SCRIPT2_ENGLISH,
            1197,
            38,
            &[
                0x3bee, 0x3c4c, 0x3c7d, 0x3cb4, 0x3ceb, 0x3d24, 0x3d57, 0x3d92, 0x3dc9, 0x3e76,
                0x4ae0,
            ],
            &[],
        );
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../output/big-bug-bang/imported-assets/resources");
        let mut cod = std::fs::read(root.join("SCRIPT2.COD")).unwrap();
        let mut dic = std::fs::read(root.join("SCRIPT2.DIC")).unwrap();
        let catalog = SequelEnglishSubtitles::from_catalog(&cod, &dic, "SCRIPT2", SCRIPT2_ENGLISH)
            .unwrap()
            .unwrap();
        assert_eq!(catalog.menus.len(), 1197);
        assert_eq!(catalog.subtitles.len(), 1186);
        let mut numeric = 0;
        for (site, menu) in &catalog.menus {
            assert_eq!(
                catalog.menu_words(*site, &menu.source),
                Some(menu.display.as_ref())
            );
            if menu
                .display
                .iter()
                .any(|word| matches!(word, InlineMenuDisplayWord::StateNumber(_)))
            {
                numeric += 1;
                assert!(catalog.subtitle(*site).is_none());
            } else {
                let text = catalog.subtitle(*site).unwrap();
                assert!(
                    text.split(|b| *b == b'\r')
                        .all(|line| line.len() <= LINE_COLUMNS)
                );
            }
        }
        assert_eq!(numeric, 11);
        assert!(
            SequelEnglishSubtitles::from_sources(&cod, &dic)
                .unwrap()
                .is_none()
        );
        assert!(
            SequelEnglishSubtitles::from_catalog(&cod, &dic, "SCRIPT1", SCRIPT2_ENGLISH).is_err()
        );
        cod[0] ^= 1;
        assert!(
            SequelEnglishSubtitles::from_catalog(&cod, &dic, "SCRIPT2", SCRIPT2_ENGLISH)
                .unwrap()
                .is_none()
        );
        cod[0] ^= 1;
        dic[0] ^= 1;
        assert!(
            SequelEnglishSubtitles::from_catalog(&cod, &dic, "SCRIPT2", SCRIPT2_ENGLISH)
                .unwrap()
                .is_none()
        );
    }

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
        assert_eq!(catalog.choices.len(), 3);
        assert_eq!(catalog.menus.len(), 89);
        for (site, menu) in &catalog.menus {
            assert_eq!(
                catalog.menu_words(*site, &menu.source),
                Some(menu.display.as_ref())
            );
            assert!(
                catalog
                    .menu_words(ScriptCodeOffset::new(0), &menu.source)
                    .is_none()
            );
            let mut changed = menu.source.to_vec();
            changed.push(ScriptTextWord::SectionSeparator);
            assert!(catalog.menu_words(*site, &changed).is_none());
        }
        let dictionary = decode_script_dictionary(&dic).unwrap();
        for (site, (words, labels)) in &catalog.choices {
            assert_eq!(catalog.choice_labels(*site, words), Some(labels.as_slice()));
            assert!(catalog.choice_labels(*site, &[]).is_none());
            assert!(
                catalog
                    .choice_labels(ScriptCodeOffset::new(0), words)
                    .is_none()
            );
            for word in words {
                assert!(dictionary.word(*word).is_some());
            }
            if words.len() > 1 {
                let reversed = words.iter().rev().copied().collect::<Vec<_>>();
                assert!(catalog.choice_labels(*site, &reversed).is_none());
            }
        }
        let (words, labels) = &catalog.choices[&ScriptCodeOffset::new(0x727)];
        assert_eq!(
            words
                .iter()
                .map(|word| dictionary.word(*word).unwrap())
                .collect::<Vec<_>>(),
            [b"JOUER".as_slice(), b"EXPLICATIONS".as_slice()]
        );
        assert_eq!(
            labels
                .iter()
                .map(|label| label.as_ref())
                .collect::<Vec<_>>(),
            [b"PLAY".as_slice(), b"INSTRUCTIONS".as_slice()]
        );
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
        struct Metrics<'a>(&'a commander_blood_formats::bloodprg::BloodprgFontResources);
        impl crate::native::bloodprg::InlineMenuTextMetrics for Metrics<'_> {
            fn rendered_width(&mut self, text: &[u8]) -> u16 {
                use crate::native::bloodprg::*;
                draw_planar_dialogue_text(
                    &mut vec![0; 320 * 200],
                    self.0,
                    text,
                    FontPoint { x: 0, y: 0 },
                    FontVerticalBand {
                        top: 0,
                        bottom: 199,
                    },
                    239,
                )
                .unwrap()
                .draw_width
            }
            fn lookahead_width(&mut self, text: Option<&[u8]>) -> u16 {
                use crate::native::bloodprg::*;
                measure_game_text_width(text.unwrap_or_default(), GameFontFace::Main, self.0)
                    .unwrap()
            }
        }
        for (site, menu) in &catalog.menus {
            use crate::native::bloodprg::*;
            let mut presentation = TextPresentationState {
                menu_deferred: true,
                menu_words: menu.source.clone(),
                menu_word_count: menu.source.len(),
                menu_reveal_count: menu.display.len() + 1,
                ..Default::default()
            };
            let InlineMenuRevealOutcome::Frame(frame) = reveal_inline_menu_display_step(
                &mut presentation,
                &dictionary,
                None,
                false,
                2,
                &mut Metrics(&fonts),
                Some(&menu.display),
            )
            .unwrap() else {
                panic!("translated menu is gated")
            };
            assert_eq!(frame.placements.len(), menu.display.len(), "{site:?}");
            assert_eq!(presentation.menu_words, menu.source);
            let mut pixels = vec![0; 320 * 200];
            for placement in frame.placements {
                assert!(
                    placement.position[0] >= 10 && placement.position[0] + placement.width <= 320,
                    "horizontal overflow at {site:?}: {placement:?}"
                );
                assert!(
                    placement.position[1] <= 192,
                    "vertical overflow at {site:?}"
                );
                draw_planar_dialogue_text(
                    &mut pixels,
                    &fonts,
                    &placement.text,
                    FontPoint {
                        x: i32::from(placement.position[0]),
                        y: i32::from(placement.position[1]),
                    },
                    FontVerticalBand {
                        top: 0,
                        bottom: 199,
                    },
                    placement.color,
                )
                .unwrap();
            }
            assert!(
                pixels.iter().any(|pixel| *pixel != 0),
                "blank translated menu at {site:?}"
            );
        }
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
