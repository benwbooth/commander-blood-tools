//! The shared VM interaction layer — the frontend's dispatch policy over
//! [`crate::vm::VmMachine`], extracted so the windowed app and the headless
//! verify harness drive the SAME machinery from the same inputs (the
//! matched-drive dual-run lane). Every method mirrors a decoded engine path:
//! row engagement starts an actor presentation (the C4 write, 0x6C7E),
//! concept clicks re-enter at the menu's dispatch region (0x677C), idle
//! frames beat the state countdowns (0x8AA) and promote queued presentations
//! (the C3->C4 scan).

use crate::vm::{VmEvent, VmMachine, VmToken};
use std::collections::HashMap;

pub struct VmDrive {
    pub m: VmMachine,
    /// Text-offset -> decoded display line (through the DIC, menu words
    /// stripped at the 0xFFFF separator).
    pub texts: HashMap<usize, String>,
    /// DIC word -> offset, for concept dispatch by label.
    pub words: HashMap<String, u16>,
    /// The DEB symbol table: name -> object offset (talk = +58).
    pub symbols: HashMap<String, u16>,
}

impl VmDrive {
    pub fn new(cod: &[u8], var: &[u8], dic_raw: &[u8], deb: &[u8]) -> Self {
        let mut m = VmMachine::new();
        m.load_cod(cod);
        m.load_var(var);
        let dic = crate::script::parse_dictionary(dic_raw);
        let mut texts = HashMap::new();
        for t in crate::vm::walk(cod, 0, cod.len()) {
            if let VmToken::Text { offset, word_offsets, .. } = t {
                let text: String = word_offsets
                    .iter()
                    .take_while(|&&w| w != 0xFFFF)
                    .filter_map(|w| dic.get(w).cloned())
                    .collect::<Vec<_>>()
                    .join(" ");
                texts.insert(offset, text);
            }
        }
        let words: HashMap<String, u16> =
            dic.iter().map(|(&o, w)| (w.to_lowercase(), o)).collect();
        let symbols: HashMap<String, u16> = crate::engine::deb_actor_name_map(deb)
            .into_iter()
            .map(|(off, name)| (name.to_lowercase(), off))
            .collect();
        VmDrive { m, texts, words, symbols }
    }

    /// One passive frame: VM frame + the idle beat. No promotion — queued
    /// calls RING until an interaction takes them (the scenario's clicks
    /// decide, as at the oracle's hub).
    pub fn frame(&mut self) -> Vec<String> {
        let mut lines = Vec::new();
        for ev in self.m.run_frame() {
            if let VmEvent::Text { offset } = ev {
                if let Some(t) = self.texts.get(&offset) {
                    lines.push(t.clone());
                }
            }
        }
        if !self.m.presentation_busy {
            self.m.tick_state_countdowns();
        }
        lines
    }

    /// The active idle frame: passive + take whatever is queued (the
    /// playthrough drives' policy).
    pub fn frame_idle(&mut self) -> Vec<String> {
        let lines = self.frame();
        if !self.m.presentation_busy {
            let _ = self.m.promote_queued_presentation();
        }
        lines
    }

    /// Engage a named actor (a console row / cryobox / contact click): starts
    /// their talk presentation (object offset + 58).
    pub fn engage(&mut self, name: &str) -> bool {
        if let Some(&obj) = self.symbols.get(&name.to_lowercase()) {
            self.m.start_actor_presentation(obj.wrapping_add(58), 40);
            true
        } else {
            false
        }
    }

    /// A concept-menu click by label (the box row's word).
    pub fn concept(&mut self, label: &str) -> bool {
        if let Some(&off) = self.words.get(&label.to_lowercase()) {
            self.m.dispatch_concept(off);
            true
        } else {
            false
        }
    }

    /// The player's advance/cancel click on a lingering presentation.
    pub fn advance(&mut self) {
        if self.m.presentation_busy {
            if let Some(actor) = self.m.active_actor {
                self.m.rec_write_pub(actor, 0);
            }
            self.m.active_actor = None;
            self.m.presentation_busy = false;
        }
    }
}
