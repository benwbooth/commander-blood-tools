//! Diff the port's SCRIPT1 boot sequence against the LIVE oracle's OCR'd line order
//! (boot_frames/tut4_replay.log): the guidance narration must emit the same lines in
//! the same order, then reach the CRYOBOX instruction.

use commander_blood_tools::descript::DescriptDb;
use commander_blood_tools::vm::{VmEvent, VmMachine};
use std::path::Path;

fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase()
        .replace('1', "I")
}

fn main() {
    let iso = Path::new("output/_tmp_iso");
    let descript = DescriptDb::parse_file(&iso.join("DESCRIPT.DES")).unwrap();
    let hnm_music = descript.hnm_music_map();
    let bundles =
        commander_blood_tools::script::parse_script_dir(iso, &descript, &hnm_music).unwrap();
    let bundle = bundles.iter().find(|b| b.script == "SCRIPT1").unwrap();
    let mut map = std::collections::HashMap::new();
    for e in bundle.speech_events.iter().filter(|e| !e.text.trim().is_empty()) {
        map.insert(e.offset, e.text.clone());
    }
    let cod = std::fs::read("output/_tmp_iso/SCRIPT1.COD").unwrap();
    let var = std::fs::read("output/_tmp_iso/SCRIPT1.VAR").unwrap_or_default();
    let mut m = VmMachine::new();
    m.load_cod(&cod);
    m.load_var(&var);
    m.presentation_busy = true;
    m.presentation_active = true;
    m.flag_252a = true;
    m.flag_274f = true;
    // The boot presenter per the bytecode + oracle: HONK (actor 2148, related 40).
    m.start_actor_presentation(2148, 40);
    m.satisfy_opening_location_guards();
    let mut port_lines: Vec<String> = Vec::new();
    for _ in 0..400 {
        for ev in m.run_frame() {
            if let VmEvent::Text { offset } = ev {
                if let Some(text) = map.get(&offset) {
                    let n = norm(text);
                    if !n.is_empty() && port_lines.last() != Some(&n) {
                        port_lines.push(n);
                    }
                }
            }
        }
        if m.halted() {
            break;
        }
    }

    // Oracle: the OCR'd stable lines in observed order (from tut4_replay.log).
    let oracle = [
        "WELCOME ABOARD THE ARK",
        "IF THE PHONE RINGS JUST HIT THE",
        "CAPN BOB OUR REVERED LEADER IS",
        "OF COURSE YOU CAN WAKE CAPN BOB",
        "CAPN BOB KNOWS EVERYTHING",
        "OUR SHIP IS CURRENTLY",
        "REMEMBER DEEP SPACE IS NO PLACE",
        "IF YOU HAVE QUESTIONS I HAVE ALL",
        "CLICK QUICK ON CRYOBOX CAPN BOB",
    ];
    println!("port emitted {} lines:", port_lines.len());
    for (i, l) in port_lines.iter().enumerate().take(16) {
        println!("  {i}: {}", &l[..l.len().min(48)]);
    }
    println!("\noracle prefix check:");
    let mut cursor = 0usize;
    let mut hits = 0;
    for o in &oracle {
        let on = norm(o);
        let found = port_lines[cursor..]
            .iter()
            .position(|l| l.starts_with(&on[..on.len().min(20)]));
        match found {
            Some(rel) => {
                hits += 1;
                cursor += rel + 1;
                println!("  OK    {o}");
            }
            None => println!("  MISS  {o}"),
        }
    }
    println!("\n{hits}/{} oracle lines matched IN ORDER", oracle.len());
}
