//! Inventory display labels only; selection continues to carry original object IDs.

use anyhow::{Result, ensure};
use serde::Deserialize;

use crate::game::GameVariant;
use crate::native::bloodprg::{PresentationChoiceId, PresentationWordChoice};

const ENGLISH: &str = include_str!("../../../../localization/big-bug-bang/en/inventory.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    format: String,
    language: String,
    entries: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    record: usize,
    source: Vec<u8>,
    english: String,
}

pub(super) fn localize(
    game: GameVariant,
    executable: &[u8],
    choices: &mut [PresentationWordChoice],
    cancel: &mut Option<Box<[u8]>>,
) -> Result<()> {
    if game != GameVariant::BigBugBang {
        return Ok(());
    }
    game.validate_native_build(executable)?;
    let catalog: Catalog = serde_json::from_str(ENGLISH)?;
    ensure!(catalog.format == "bbb-inventory-display-translation-v1" && catalog.language == "en");
    for choice in choices {
        let PresentationChoiceId::Inventory(id) = choice.identity else {
            continue;
        };
        if let Some(entry) = catalog.entries.iter().find(|entry| {
            entry.record == id.index() && entry.source.as_slice() == choice.label.as_ref()
        }) {
            choice.label = entry.english.as_bytes().into();
        }
    }
    if cancel.as_deref() == Some(b"ANNULER") {
        *cancel = Some(Box::from(b"CANCEL".as_slice()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::bloodprg::{
        ChoiceListBackend, ChoiceListConfig, ChoiceListPointer, ChoiceListRect, ChoiceListRowKind,
        ChoiceListState, GameFontFace, measure_game_text_width, update_choice_list_for_dialect,
    };
    use commander_blood_formats::code::ScriptDialect;
    use commander_blood_formats::script::{
        ScriptObjectKind, decode_script_directory, decode_script_state_for_dialect,
    };

    #[test]
    fn catalog_is_unique_printable_and_bounded() {
        let catalog: Catalog = serde_json::from_str(ENGLISH).unwrap();
        let mut records = std::collections::BTreeSet::new();
        assert_eq!(catalog.entries.len(), 25);
        for entry in catalog.entries {
            assert!(records.insert(entry.record));
            assert!(!entry.source.is_empty());
            assert!(!entry.english.is_empty() && entry.english.len() <= 16);
            assert!(
                entry
                    .english
                    .bytes()
                    .all(|byte| (b' '..=b'~').contains(&byte))
            );
        }
    }

    #[test]
    #[ignore = "requires all original BBB VAR/DEB resources and executable fonts"]
    fn authentic_inventory_labels_preserve_identity_and_render_rgb() {
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../output/big-bug-bang");
        let executable = std::fs::read(root.join("disc/BLOOD2PG.EXE")).unwrap();
        let game = GameVariant::BigBugBang;
        let fonts = game.decode_fonts(&executable).unwrap();
        let palette = game.decode_default_vga_palette(&executable).unwrap();
        let ui = crate::ui::ChoiceUiAssets::import(&fonts, &palette).unwrap();
        let catalog: Catalog = serde_json::from_str(ENGLISH).unwrap();
        let mut total = 0;
        for profile in 1..=17 {
            let directory = decode_script_directory(
                &std::fs::read(root.join(format!("imported-assets/resources/SCRIPT{profile}.DEB")))
                    .unwrap(),
            )
            .unwrap();
            let state = decode_script_state_for_dialect(
                &std::fs::read(root.join(format!("imported-assets/resources/SCRIPT{profile}.VAR")))
                    .unwrap(),
                &directory,
                ScriptDialect::BigBugBang,
            )
            .unwrap();
            let original: Vec<_> = state
                .objects()
                .iter()
                .filter(|object| object.kind == ScriptObjectKind::InventoryItem)
                .map(|object| {
                    let name = &object.bytes()[4..20];
                    let length = name.iter().position(|byte| *byte == 0).unwrap();
                    PresentationWordChoice::inventory(object.id, &name[..length])
                })
                .collect();
            assert_eq!(original.len(), 25);
            let mut choices = original.clone();
            let mut cancel = Some(game.decode_inventory_cancel_label(&executable).unwrap());
            localize(game, &executable, &mut choices, &mut cancel).unwrap();
            assert_eq!(cancel.as_deref(), Some(b"CANCEL".as_slice()));
            if profile == 1 {
                check_translated_lists(&fonts, &ui, &choices, cancel.as_deref().unwrap());
            }
            for (before, after) in original.iter().zip(&choices) {
                assert_eq!(before.identity, after.identity);
                let PresentationChoiceId::Inventory(id) = after.identity else {
                    panic!()
                };
                let entry = catalog
                    .entries
                    .iter()
                    .find(|entry| entry.record == id.index())
                    .unwrap();
                assert_eq!(
                    before.label.as_ref(),
                    entry.source,
                    "SCRIPT{profile}, {id:?}"
                );
                assert_eq!(after.label.as_ref(), entry.english.as_bytes());
                let mut overlay = crate::ui::RgbaUiOverlay::new(320, 200);
                ui.draw_text(
                    &mut overlay,
                    &after.label,
                    [175, 89],
                    crate::ui::ChoiceTextStyle::Normal,
                )
                .unwrap();
                assert!(overlay.pixels().chunks_exact(4).any(|pixel| pixel[3] != 0));
                total += 1;
            }
            let mut altered = original.clone();
            altered[0].label = Box::from(b"custom crown".as_slice());
            altered[1].identity = original[0].identity;
            let first = altered[..2].to_vec();
            cancel = Some(Box::from(b"CUSTOM".as_slice()));
            localize(game, &executable, &mut altered, &mut cancel).unwrap();
            assert_eq!(&altered[..2], first);
            assert_eq!(cancel.as_deref(), Some(b"CUSTOM".as_slice()));
            let mut unchanged = original.clone();
            assert!(localize(game, b"changed", &mut unchanged, &mut cancel).is_err());
            assert_eq!(unchanged, original);
            localize(
                GameVariant::CommanderBlood,
                &executable,
                &mut unchanged,
                &mut cancel,
            )
            .unwrap();
            assert_eq!(unchanged, original);
        }
        assert_eq!(total, 425);
    }

    struct ListBackend<'a> {
        fonts: &'a commander_blood_formats::bloodprg::BloodprgFontResources,
        pointer: ChoiceListPointer,
        prepared: Option<ChoiceListRect>,
    }

    impl ChoiceListBackend for ListBackend<'_> {
        fn measure_label(&mut self, label: &[u8]) -> u16 {
            measure_game_text_width(label, GameFontFace::SquareCaps, self.fonts).unwrap()
        }
        fn prepare_background(&mut self, rect: ChoiceListRect) {
            self.prepared = Some(rect);
        }
        fn pointer(&mut self) -> ChoiceListPointer {
            assert!(self.prepared.is_some());
            self.pointer
        }
    }

    fn check_translated_lists(
        fonts: &commander_blood_formats::bloodprg::BloodprgFontResources,
        ui: &crate::ui::ChoiceUiAssets,
        choices: &[PresentationWordChoice],
        cancel: &[u8],
    ) {
        let mut cases = 0;
        for start in 0..choices.len() {
            for count in 1..=16 {
                let selected: Vec<_> = choices.iter().cycle().skip(start).take(count).collect();
                let labels: Vec<_> = selected.iter().map(|item| item.label.as_ref()).collect();
                let config = ChoiceListConfig {
                    center_x: 225,
                    preserve_individual_widths: false,
                    cancel_label: Some(cancel),
                    layout_only: false,
                };
                let mut backend = ListBackend {
                    fonts,
                    pointer: ChoiceListPointer::default(),
                    prepared: None,
                };
                let mut state = ChoiceListState::default();
                let frame = update_choice_list_for_dialect(
                    &labels,
                    config,
                    &mut state,
                    &mut backend,
                    ScriptDialect::BigBugBang,
                );
                let [x, y] = frame.rect.origin.map(i32::from);
                let [w, h] = frame.rect.size.map(i32::from);
                assert!(x >= 0 && y >= 0 && x + w <= 320 && y + h <= 200);
                assert_eq!(frame.rows.len(), count + 1);
                for row in &frame.rows {
                    let text = match row.kind {
                        ChoiceListRowKind::Item(index) => labels[index],
                        ChoiceListRowKind::Cancel => cancel,
                    };
                    let mut overlay = crate::ui::RgbaUiOverlay::new(320, 200);
                    ui.draw_text(
                        &mut overlay,
                        text,
                        row.position.map(i32::from),
                        crate::ui::ChoiceTextStyle::Normal,
                    )
                    .unwrap();
                    let mut pixels = 0;
                    for (index, pixel) in overlay.pixels().chunks_exact(4).enumerate() {
                        if pixel[3] != 0 {
                            let px = (index % 320) as i32;
                            let py = (index / 320) as i32;
                            assert!(px >= x && px < x + w && py >= y && py < y + h);
                            pixels += 1;
                        }
                    }
                    assert!(pixels > 0);
                    backend.pointer = ChoiceListPointer {
                        position: [row.position[0] as i16 + 1, row.position[1] as i16 + 1],
                        primary_pressed: true,
                    };
                    let clicked = update_choice_list_for_dialect(
                        &labels,
                        config,
                        &mut state,
                        &mut backend,
                        ScriptDialect::BigBugBang,
                    );
                    match row.kind {
                        ChoiceListRowKind::Item(index) => {
                            assert_eq!(clicked.selected_item, Some(index));
                            assert!(!clicked.cancelled);
                            assert_eq!(
                                selected[index].identity,
                                choices[(start + index) % choices.len()].identity
                            );
                        }
                        ChoiceListRowKind::Cancel => {
                            assert!(clicked.cancelled);
                            assert_eq!(clicked.selected_item, None);
                        }
                    }
                }
                cases += 1;
            }
        }
        assert_eq!(cases, 400);
    }
}
