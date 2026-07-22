//! BAS conversation menu-stack — the clean-port model of the game's concept-menu
//! navigation, reverse-engineered from the script VM (segment 0x067c, dispatch
//! `gs:0x6EB0`) that executes `SCRIPTn.BAS`.
//!
//! The running game keeps the ACTIVE concept menu in `gs:0x6772` and the parent
//! menu it was entered from in `gs:0x6774` — a stack. Entering a sub-topic that
//! opens another menu PUSHES it (VM opcode `0xA3`, handler seg 0x067c +0x446:
//! saves `gs:0x6772→gs:0x6774`, `gs:0x6782→gs:0x6784`, then sets the new menu);
//! backing out (`bye_bye`) POPS. Verified live against `milestone_script2.state`:
//! `gs:0x6772` = 0x42d (`talk/fear/weakness/…`, the active sub-menu) with
//! `gs:0x6774` = 0x2f (the top-level menu beneath it) — exactly reproduced here.
//!
//! This models that stack over the menus decoded by [`crate::concept_menu`]. It is
//! the navigation layer the simplified clean port (`engine.rs`) needs to show the
//! right topic list per conversation beat; the per-topic branch targets (which
//! sub-menu each topic opens) come from the BAS control flow and are wired as the
//! VM's dialogue execution reaches each `0xA3`.

use crate::concept_menu::{decode_menus, ConceptMenu};

/// A back-out topic that pops the menu stack (the game's universal "leave" verb).
const BACK_TOPICS: [&str; 2] = ["bye_bye", "talk"];

/// The concept-menu stack for one script's conversation, mirroring the game's
/// `gs:0x6772`/`gs:0x6774` menu stack.
#[derive(Debug, Clone)]
pub struct BasMenuStack {
    /// Every concept menu decoded from the script's `.BAS`, keyed by BAS offset.
    menus: Vec<ConceptMenu>,
    /// Active menu BAS offsets, innermost last (the top = current = `gs:0x6772`).
    stack: Vec<usize>,
}

impl BasMenuStack {
    /// Decode a script's menus and seed the stack with the ENTRY menu — the first
    /// `0xA3` the BAS VM reaches from offset 0 (verified: SCRIPT2's entry is 0x2f,
    /// the top-level menu, which is the base of the live `gs:0x6774` stack).
    pub fn new(bas: &[u8], dic: &[u8]) -> Option<Self> {
        let menus = decode_menus(bas, dic, 3);
        let entry = menus.first()?.bas_offset;
        Some(Self { menus, stack: vec![entry] })
    }

    /// The menu currently displayed (the top of the stack = `gs:0x6772`).
    pub fn current(&self) -> Option<&ConceptMenu> {
        let off = *self.stack.last()?;
        self.menus.iter().find(|m| m.bas_offset == off)
    }

    /// All decoded menus (for wiring topic → sub-menu targets from the BAS flow).
    pub fn menus(&self) -> &[ConceptMenu] {
        &self.menus
    }

    /// Enter the menu at `bas_offset` (push, as VM opcode `0xA3` does). Returns
    /// false if no menu is defined there.
    pub fn push(&mut self, bas_offset: usize) -> bool {
        if self.menus.iter().any(|m| m.bas_offset == bas_offset) {
            self.stack.push(bas_offset);
            true
        } else {
            false
        }
    }

    /// Back out of the current menu (pop). Never empties the stack — the entry
    /// menu always remains, matching the game (the top-level menu is never popped).
    pub fn pop(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.pop();
            true
        } else {
            false
        }
    }

    /// Whether a topic label backs out of the menu (pops) rather than descending.
    pub fn is_back_topic(label: &str) -> bool {
        BACK_TOPICS.iter().any(|b| b.eq_ignore_ascii_case(label))
    }

    /// Current stack depth (1 = at the entry menu).
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn read(n: u32, ext: &str) -> Option<Vec<u8>> {
        for base in ["accuracy/cdrive/cblood", "../accuracy/cdrive/cblood"] {
            if let Ok(b) = std::fs::read(Path::new(base).join(format!("SCRIPT{n}.{ext}"))) {
                return Some(b);
            }
        }
        None
    }

    /// The stack seeds at SCRIPT2's entry menu (top-level) and reproduces the live
    /// `gs:0x6772`/`gs:0x6774` state: pushing the fear/anger sub-menu (0x42d) makes
    /// it current with the top-level (0x2f) beneath — exactly the captured state.
    #[test]
    fn menu_stack_reproduces_live_gs6772_state() {
        let (Some(bas), Some(dic)) = (read(2, "BAS"), read(2, "DIC")) else {
            return;
        };
        let mut st = BasMenuStack::new(&bas, &dic).expect("script2 menus decode");
        // Entry = the top-level menu at BAS 0x2f (base of the live stack, gs:0x6774).
        let entry = st.current().unwrap();
        assert_eq!(entry.bas_offset, 0x2f);
        assert!(entry.labels.iter().any(|l| l == "optimization"));
        // Push the live current menu (gs:0x6772 = 0x42d = fear/anger sub-conversation).
        assert!(st.push(0x42d));
        let cur = st.current().unwrap();
        assert_eq!(cur.bas_offset, 0x42d);
        assert!(cur.labels.iter().any(|l| l == "fear"));
        assert_eq!(st.depth(), 2);
        // Back out (bye_bye) → top-level again.
        assert!(st.pop());
        assert_eq!(st.current().unwrap().bas_offset, 0x2f);
        assert!(BasMenuStack::is_back_topic("bye_bye"));
    }
}
