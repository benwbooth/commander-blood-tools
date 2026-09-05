//! Complete original object-choice sequences, including native transition helpers.

use commander_blood_formats::script::decode_script_directory;
use serde::Deserialize;

use super::*;

#[derive(Deserialize)]
struct Oracle {
    name: String,
    inventory: bool,
    labels: Vec<Vec<u8>>,
    selection: Option<usize>,
    frames: Vec<Frame>,
}

#[derive(Deserialize)]
struct Frame {
    phase: u8,
    active: u8,
    selected: u16,
    rect: [u16; 4],
    target: [u16; 4],
    step: u8,
    request: u8,
    deferred: u8,
    subtitle: u8,
    hold: u8,
    ui: u8,
    vm_enabled: bool,
    draws: Vec<Draw>,
    backgrounds: Vec<[u16; 4]>,
    pointer: [u16; 2],
    pressed: bool,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Draw {
    text: Vec<u8>,
    x: u16,
    y: u16,
    color: u8,
}

struct Backend {
    fonts: commander_blood_formats::bloodprg::BloodprgFontResources,
    transition: FramebufferTransitionState,
    pointer: ChoiceListPointer,
    backgrounds: Vec<[u16; 4]>,
}

fn rectangle(rect: ChoiceListRect) -> [u16; 4] {
    [
        rect.origin[0] as u16,
        rect.origin[1] as u16,
        rect.size[0],
        rect.size[1],
    ]
}

fn transition_rectangle(rect: ChoiceListRect) -> TransitionRect {
    TransitionRect::new(
        rect.origin[0],
        rect.origin[1],
        rect.size[0] as i16,
        rect.size[1] as i16,
    )
}

impl ChoiceListBackend for Backend {
    fn measure_label(&mut self, label: &[u8]) -> u16 {
        measure_game_text_width(label, GameFontFace::SquareCaps, &self.fonts).unwrap()
    }

    fn prepare_background(&mut self, rect: ChoiceListRect) {
        self.backgrounds.push(rectangle(rect));
    }

    fn pointer(&mut self) -> ChoiceListPointer {
        self.pointer
    }
}

impl PresentationWordChoiceBackend for Backend {
    fn advance_word_choice_transition(
        &mut self,
        source: ChoiceListRect,
        target: ChoiceListRect,
    ) -> bool {
        match advance_framebuffer_rect_transition(
            &mut self.transition,
            transition_rectangle(source),
            transition_rectangle(target),
        )
        .unwrap()
        {
            Some(region) => {
                self.backgrounds
                    .push([region.x, region.y, region.width, region.height]);
                false
            }
            None => true,
        }
    }
}

#[test]
#[ignore = "requires original sequel executable fonts and cancel label"]
fn sequel_inventory_chooser_matches_complete_native_open_select_cancel_and_close() {
    let executable = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../output/big-bug-bang/disc/BLOOD2PG.EXE"),
    )
    .unwrap();
    let game = crate::game::GameVariant::BigBugBang;
    let fonts = game.decode_fonts(&executable).unwrap();
    let cancel = game.decode_inventory_cancel_label(&executable).unwrap();
    assert_eq!(cancel.as_ref(), b"ANNULER");
    let vectors: Vec<Oracle> =
        include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_inventory_choice.jsonl")
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
    assert_eq!(vectors.len(), 11);
    for vector in vectors {
        let mut directory_bytes = Vec::new();
        for index in 0..vector.labels.len() {
            let mut entry = [0; 20];
            let name = format!("item{index}");
            entry[..name.len()].copy_from_slice(name.as_bytes());
            entry[16..18].copy_from_slice(&((0x100 + index * 32) as u16).to_le_bytes());
            entry[18..20].copy_from_slice(&1u16.to_le_bytes());
            directory_bytes.extend(entry);
        }
        directory_bytes.extend([0; 20]);
        let directory = decode_script_directory(&directory_bytes).unwrap();
        let ids = directory
            .active_objects()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        let mut choices: Vec<_> = ids
            .iter()
            .zip(&vector.labels)
            .map(|(id, label)| PresentationWordChoice::inventory(*id, label.clone()))
            .collect();
        if !vector.inventory {
            let bytes = vector
                .labels
                .iter()
                .flat_map(|label| label.iter().copied().chain([0]))
                .collect::<Vec<_>>();
            let dictionary =
                commander_blood_formats::script::decode_script_dictionary(&bytes).unwrap();
            choices = dictionary
                .words()
                .zip(&vector.labels)
                .map(|((word, _), label)| PresentationWordChoice::new(word, label.clone()))
                .collect();
        }
        let identities = choices
            .iter()
            .map(|choice| choice.identity)
            .collect::<Vec<_>>();
        let mut state = PresentationWordChoiceState {
            dialect: commander_blood_formats::code::ScriptDialect::BigBugBang,
            vm_execution_enabled: true,
            active: true,
            choices,
            inventory_cancel_label: vector.inventory.then(|| cancel.clone()),
            presentation_deferred: true,
            text_display_active: true,
            dialogue_hold_complete: true,
            request_pending: true,
            ..Default::default()
        };
        let mut backend = Backend {
            fonts: fonts.clone(),
            pointer: ChoiceListPointer::default(),
            backgrounds: Vec::new(),
            transition: FramebufferTransitionState {
                total_steps: 4,
                current_step: 0,
            },
        };
        for (index, native) in vector.frames.iter().enumerate() {
            backend.backgrounds.clear();
            backend.pointer = ChoiceListPointer {
                position: native.pointer.map(|value| value as i16),
                primary_pressed: native.pressed,
            };
            let outcome = update_presentation_word_choice(
                PresentationWordChoiceContext {
                    presentation_active: true,
                    request_busy: false,
                    animation_target: ChoiceListRect {
                        origin: [0, 100],
                        size: [0, 0],
                    },
                },
                &mut state,
                &mut backend,
            );
            let frame = match &outcome {
                PresentationWordChoiceOutcome::AwaitingSelection(frame) => Some(frame),
                PresentationWordChoiceOutcome::Selected { frame, .. }
                | PresentationWordChoiceOutcome::CancelSelected(frame) => {
                    backend.transition.current_step = 0;
                    Some(frame)
                }
                _ => None,
            };
            let draws = frame
                .into_iter()
                .flat_map(|frame| frame.rows.iter())
                .map(|row| Draw {
                    text: match row.kind {
                        ChoiceListRowKind::Item(index) => vector.labels[index].clone(),
                        ChoiceListRowKind::Cancel => cancel.to_vec(),
                    },
                    x: row.position[0],
                    y: row.position[1],
                    color: row.color,
                })
                .collect::<Vec<_>>();
            assert_eq!(draws, native.draws, "{} frame {index}", vector.name);
            if !draws.is_empty() {
                assert_rgb_rows(&backend.fonts, &draws, state.current_rect);
            }
            assert_eq!(
                backend.backgrounds, native.backgrounds,
                "{} frame {index}",
                vector.name
            );
            assert_eq!(rectangle(state.current_rect), native.rect);
            assert_eq!(rectangle(state.animation_target), native.target);
            assert_eq!(backend.transition.current_step, native.step);
            let phase = match state.phase {
                PresentationWordChoicePhase::Closed => 0,
                PresentationWordChoicePhase::Opening => 1,
                PresentationWordChoicePhase::Selecting => 2,
                PresentationWordChoicePhase::Closing => 3,
            };
            assert_eq!(phase, native.phase);
            assert_eq!(state.active, native.active != 0);
            assert_eq!(state.interface_active, native.ui & 4 != 0);
            assert_eq!(state.vm_execution_enabled, native.vm_enabled);
            assert_eq!(state.request_pending, native.request & 1 != 0);
            assert_eq!(state.presentation_deferred, native.deferred != 0);
            assert_eq!(state.text_display_active, native.subtitle != 0);
            assert_eq!(state.dialogue_hold_complete, native.hold != 0);
            if native.active == 0 {
                let selected = vector.selection.unwrap();
                if selected < ids.len() {
                    assert_eq!(
                        outcome,
                        PresentationWordChoiceOutcome::Completed(identities[selected])
                    );
                    assert_eq!(native.selected, (0x104 + selected * 32) as u16);
                } else {
                    assert_eq!(outcome, PresentationWordChoiceOutcome::Cancelled);
                    assert_eq!(native.selected, 0);
                }
            }
        }
    }
}

fn assert_rgb_rows(
    fonts: &commander_blood_formats::bloodprg::BloodprgFontResources,
    draws: &[Draw],
    rect: ChoiceListRect,
) {
    use crate::ui::{ChoiceUiAssets, RgbaUiOverlay};
    let colors = [[17, 31, 47]; 256];
    let assets = ChoiceUiAssets::import(fonts, &colors).unwrap();
    let mut overlay = RgbaUiOverlay::new(320, 200);
    let mut reference = vec![0; 320 * 200];
    for draw in draws {
        assets
            .draw_text(
                &mut overlay,
                &draw.text,
                [i32::from(draw.x), i32::from(draw.y)],
                draw.color.try_into().unwrap(),
            )
            .unwrap();
        draw_square_caps_text(
            &mut reference,
            fonts,
            &draw.text,
            FontPoint {
                x: i32::from(draw.x),
                y: i32::from(draw.y),
            },
            FontVerticalBand {
                top: 0,
                bottom: 199,
            },
            1,
        )
        .unwrap();
    }
    assert!(reference.iter().any(|pixel| *pixel != 0));
    for (index, (coverage, rgba)) in reference
        .iter()
        .zip(overlay.pixels().chunks_exact(4))
        .enumerate()
    {
        assert_eq!(*coverage != 0, rgba[3] != 0);
        if *coverage != 0 {
            assert_eq!(rgba, [69, 125, 190, 255]);
            let x = (index % 320) as i16;
            let y = (index / 320) as i16;
            assert!(x >= rect.origin[0] && x < rect.origin[0] + rect.size[0] as i16);
            assert!(y >= rect.origin[1] && y < rect.origin[1] + rect.size[1] as i16);
        }
    }
}
