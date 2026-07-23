use commander_blood_tools::vm::{self, VmMachine, VmEvent, VmToken};
fn main() {
    let n: u32 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let concept: u16 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    let cod = std::fs::read(format!("output/_tmp_iso/SCRIPT{n}.COD")).unwrap();
    let var = std::fs::read(format!("output/_tmp_iso/SCRIPT{n}.VAR")).unwrap();
    // Map token offset -> text for display
    let toks = vm::walk(&cod, 0, cod.len());
    let dic_raw = std::fs::read(format!("output/_tmp_iso/SCRIPT{n}.DIC")).unwrap();
    let dic = commander_blood_tools::script::parse_dictionary(&dic_raw);
    let mut m = VmMachine::new();
    m.load_cod(&cod);
    m.load_var(&var);
    m.concept = concept;
    if std::env::var("VMPRES").is_ok() {
        m.presentation_busy = true;
        m.presentation_active = true;
    }
    // Start a specific actor's presentation: VMACTOR=<record_offset>,<related>
    if let Ok(spec) = std::env::var("VMACTOR") {
        let parts: Vec<u16> = spec.split(',').filter_map(|p| p.parse().ok()).collect();
        if parts.len() == 2 {
            m.start_actor_presentation(parts[0], parts[1]);
        }
    }
    // trace mode: step-by-step with pc log
    if std::env::var("VMTRACE").is_ok() {
        for i in 0..60 {
            let pc = m.pc;
            let op = m.cod.get(pc).copied().unwrap_or(0xFF);
            let alive = m.step();
            println!("step {i}: pc={pc} op={op:#04x} -> pc={} stack={:?} q={} alive={alive}", m.pc, m.stack, m.query);
            if !alive { break; }
        }
        return;
    }
    let mut total = 0;
    for frame in 0..500 {
        let evs = m.run_frame();
        for e in &evs {
            match e {
                VmEvent::Text { offset } => {
                    let text: String = toks.iter().find_map(|t| match t {
                        VmToken::Text { offset: o, word_offsets, .. } if o == offset => {
                            Some(
                                word_offsets
                                    .iter()
                                    .filter_map(|w| dic.get(w).cloned())
                                    .collect::<Vec<_>>()
                                    .join(" "),
                            )
                        }
                        _ => None,
                    }).unwrap_or_default();
                    println!("TEXT @{offset}: {}", text.replace('\n', " ").chars().take(60).collect::<String>());
                }
                VmEvent::Actor { offset } => println!("ACTOR @{offset}"),
                VmEvent::ProfileRequest(p) => println!("PROFILE {p}"),
                VmEvent::LoadString(s) => println!("STR {s}"),
            }
            total += 1;
            if total > 40 { println!("... (truncated)"); return; }
        }
        if evs.is_empty() { println!("[no events, frame {frame}]"); if frame > 3 { break; } }
    }
}
