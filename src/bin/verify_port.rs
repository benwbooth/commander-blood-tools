//! DUAL-RUN DIFFERENTIAL, port side: execute the same scenario file as the oracle's
//! VERIFYSCRIPT mode against the port's EngineState at the matching start state (the
//! bridge hub), writing one settled RGB frame per step into boot_frames/vp_*.ppm.
//! tools/verify_compare.py then scores oracle vs port per step.

use commander_blood_tools::engine::{EngineState, MouseInput};
use std::path::Path;

fn main() {
    let scenario = std::env::args().nth(1).expect("usage: verify_port <scenario.tsv>");
    let iso = Path::new("output/_tmp_iso");
    let out = Path::new("boot_frames");
    std::fs::create_dir_all(out).unwrap();

    let mut e = EngineState::new();
    e.load_bridge(iso);
    e.load_console_font(iso);
    // The VM rides along: the same SCRIPT2 machine the app drives, so the
    // scenario produces a LINE TRANSCRIPT (vp_transcript.txt) for line-level
    // comparison against the oracle's SAYDUMP — the matched-drive lane's
    // first plank. The decoded BAS menu stack replaces the old hardcoded box
    // literals (a no-transcription retirement).
    let rd = |ext: &str| std::fs::read(format!("output/_tmp_iso/SCRIPT2.{ext}"));
    let mut vm = commander_blood_tools::vm::VmMachine::new();
    let mut transcript: Vec<String> = Vec::new();
    let mut vm_texts: std::collections::HashMap<usize, String> = Default::default();
    if let (Ok(cod), Ok(var), Ok(dic_raw)) = (rd("COD"), rd("VAR"), rd("DIC")) {
        vm.load_cod(&cod);
        vm.load_var(&var);
        let dic = commander_blood_tools::script::parse_dictionary(&dic_raw);
        for t in commander_blood_tools::vm::walk(&cod, 0, cod.len()) {
            if let commander_blood_tools::vm::VmToken::Text { offset, word_offsets, .. } = t {
                let text: String = word_offsets
                    .iter()
                    .take_while(|&&w| w != 0xFFFF)
                    .filter_map(|w| dic.get(w).cloned())
                    .collect::<Vec<_>>()
                    .join(" ");
                vm_texts.insert(offset, text);
            }
        }
        if let (Ok(bas), Ok(d)) = (rd("BAS"), rd("DIC")) {
            e.load_bas_menus(&bas, &d);
        }
    }
    e.on_ship = true;
    e.bridge_active = true;
    // The hub state: ring frame 45 (the oracle's script2.state view), menu baked.
    e.bridge.frame = 45;
    // The oracle hub is the PRESENTATION state (CANCEL live): model it exactly.
    e.set_speech_dialogue(vec![(String::new(), None)]);
    e.hub_presentation = true;
    let (mut mx, mut my) = (160u16, 100u16);
    // The oracle hub is in PRESENTATION state: steering is script-locked ([0x2793]
    // bit2). Model the same lock: pin the ring mouse to the view centre each step.
    let step_engine = |e: &mut EngineState, mx: u16, my: u16, buttons: u16| {
        e.bridge.ring_mouse_x = (45 * 8) as i32; // centred: no ring chase
        e.step(MouseInput { x: mx, y: my, buttons, ..Default::default() });
        e.bridge.frame = 45;
    };
    // settle after each action: a few game ticks
    let settle = |e: &mut EngineState, mx: u16, my: u16| {
        for _ in 0..8 {
            step_engine(e, mx, my, 0);
        }
    };

    let text = std::fs::read_to_string(&scenario).unwrap();
    let mut step = 0usize;
    for line in text.lines() {
        let toks: Vec<&str> = line.split_whitespace().collect();
        if toks.is_empty() || toks[0].starts_with('#') {
            continue;
        }
        match toks[0] {
            "move" => {
                mx = toks[1].parse().unwrap();
                my = toks[2].parse().unwrap();
            }
            "click" => {
                mx = toks[1].parse().unwrap();
                my = toks[2].parse().unwrap();
                // Mirror the windowed dispatch: CANCEL label, then the decoded
                // console-row rects, then the engine's own click paths.
                if !e.hub_cancel_click(mx, my) {
                    // The console rows only dispatch once the presentation is done
                    // (the windowed game's dialogue_finished gate; oracle-confirmed:
                    // clicks during the live presentation are ignored).
                    match if e.hub_presentation { None } else { e.bridge_press(mx, my) } {
                        Some(0) => {
                            e.bridge.engaged_row = Some(0);
                            // The decoded BAS entry menu (the game's own concept
                            // stack), not a transcription.
                            let labels = e.current_bas_menu_labels();
                            e.console_box = if labels.is_empty() {
                                Vec::new()
                            } else {
                                labels
                            };
                            e.console_box_kind = 3;
                        }
                        Some(1) => e.bridge.engaged_row = Some(1),
                        Some(2) => {
                            e.bridge.engaged_row = Some(2);
                            // The cryobox candidate box comes from the VM's
                            // crew state in the app; headless, leave it to the
                            // engine's own population.
                            e.console_box_kind = 2;
                        }
                        Some(3) => e.bridge.engaged_row = Some(3),
                        Some(4) => {
                            e.bridge.engaged_row = Some(4);
                            e.console_box = vec![
                                "TEXT".into(),
                                "MUSIC_OFF".into(),
                                "SAVE".into(),
                                "LOAD".into(),
                                "QUIT".into(),
                                "CANCEL".into(),
                            ];
                            e.console_box_kind = 4;
                        }
                        _ => {}
                    }
                }
                step_engine(&mut e, mx, my, 1);
            }
            "key" => {}
            "wait" => {
                let frames: usize = toks[1].parse().unwrap();
                for _ in 0..frames {
                    step_engine(&mut e, mx, my, 0);
                }
                // The VM rides the waits: frames + beats, transcripting lines.
                for ev in vm.run_frame() {
                    if let commander_blood_tools::vm::VmEvent::Text { offset } = ev {
                        if let Some(t) = vm_texts.get(&offset) {
                            transcript.push(t.clone());
                        }
                    }
                }
                vm.tick_state_countdowns();
            }
            _ => {}
        }
        settle(&mut e, mx, my);
        let mut ppm = b"P6\n320 200\n255\n".to_vec();
        for &px in e.framebuffer.iter() {
            ppm.extend_from_slice(&e.scene_palette[px as usize]);
        }
        std::fs::write(out.join(format!("vp_{step:03}.ppm")), ppm).unwrap();
        step += 1;
    }
    std::fs::write(
        out.join("vp_transcript.txt"),
        transcript.join("\n"),
    )
    .unwrap();
    println!("verify_port done: {step} steps, {} transcript lines", transcript.len());
}
