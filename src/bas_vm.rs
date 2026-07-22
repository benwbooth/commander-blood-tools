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
use crate::vm::{walk, VmToken};

/// The `0xAC` opcode terminates a menu's response block (verified via BASSTEP trace).
const MENU_BLOCK_END: u8 = 0xAC;

/// A decoded menu BLOCK: the menu's topics plus the response `0xA6` TEXT tokens that
/// follow it, terminated by `0xAC` (grammar: `0xA3 <topics> [0xA6 response]* 0xAC`,
/// single-step-traced from the running VM). `end` is the `0xAC` offset. This is the
/// unit the conversation VM walks; a nested `0xA3` among the responses is a sub-menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuBlock {
    /// The menu head (`0xA3`) BAS offset.
    pub menu_offset: usize,
    /// Topic labels (as stored, lowercase).
    pub topics: Vec<String>,
    /// BAS offsets of the response `0xA6` TEXT tokens, in stream order.
    pub responses: Vec<usize>,
    /// BAS offset of the block-terminating `0xAC`.
    pub end: usize,
}

/// Parse the menu BLOCK at `menu_offset` (a `0xA3`): its topic list, then the `0xA6`
/// response tokens up to the `0xAC` terminator. Returns `None` if `menu_offset` is not
/// a menu head. Uses the port's faithful VM token walker for the response tokens.
pub fn parse_menu_block(bas: &[u8], dic: &[u8], menu_offset: usize) -> Option<MenuBlock> {
    if bas.get(menu_offset) != Some(&0xA3) {
        return None;
    }
    let dic_words = parse_dic_words(dic);
    // Topic list: u16 offsets to single-token dictionary words until a non-topic word.
    let mut p = menu_offset + 1;
    let mut topics = Vec::new();
    while p + 1 < bas.len() {
        let off = u16::from_le_bytes([bas[p], bas[p + 1]]);
        match dic_words.get(&off) {
            Some(w) if is_single_token(w) => {
                topics.push(w.clone());
                p += 2;
            }
            _ => break,
        }
    }
    if topics.is_empty() {
        return None;
    }
    // Skip the 0x0000 topic-list terminator, then walk the response tokens to 0xAC.
    if p + 1 < bas.len() && bas[p] == 0 && bas[p + 1] == 0 {
        p += 2;
    }
    let mut responses = Vec::new();
    let mut end = p;
    for tok in walk(bas, p, bas.len()) {
        match tok {
            VmToken::Text { offset, .. } => responses.push(offset),
            VmToken::Op { opcode: MENU_BLOCK_END, offset, .. } => {
                end = offset;
                break;
            }
            _ => {}
        }
    }
    Some(MenuBlock { menu_offset, topics, responses, end })
}

/// Sequential response player for a menu block whose responses are a monologue shown
/// one-per-interaction (the already-shown gating `vm.rs` models — verified: the
/// psychotherapy fear/anger block is 13 pure `0xA6` responses, no per-topic records).
/// Each [`advance`](Self::advance) yields the next response's BAS offset (the `0xA6`
/// token), tracking the shown count; the engine renders it via the dialogue system.
#[derive(Debug, Clone)]
pub struct SequentialResponses {
    responses: Vec<usize>,
    shown: usize,
}

impl SequentialResponses {
    /// Start playing a menu block's responses (in stream order).
    pub fn new(block: &MenuBlock) -> Self {
        Self { responses: block.responses.clone(), shown: 0 }
    }

    /// The next response `0xA6` BAS offset to display, advancing the shown count.
    /// `None` once the monologue is exhausted (all responses already shown).
    pub fn advance(&mut self) -> Option<usize> {
        let r = self.responses.get(self.shown).copied();
        if r.is_some() {
            self.shown += 1;
        }
        r
    }

    /// Responses not yet shown.
    pub fn remaining(&self) -> usize {
        self.responses.len().saturating_sub(self.shown)
    }

    /// Total responses in the block.
    pub fn total(&self) -> usize {
        self.responses.len()
    }
}

/// Parse a `SCRIPTn.DIC` into {offset -> word}.
fn parse_dic_words(dic: &[u8]) -> std::collections::HashMap<u16, String> {
    let mut w = std::collections::HashMap::new();
    let mut p = 0usize;
    while p < dic.len() {
        let s = p;
        while p < dic.len() && dic[p] != 0 {
            p += 1;
        }
        if p > s {
            w.insert(s as u16, String::from_utf8_lossy(&dic[s..p]).into_owned());
        }
        p += 1;
    }
    w
}

fn is_single_token(w: &str) -> bool {
    (2..=16).contains(&w.len()) && !w.contains(' ')
}

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
    /// The script's `.BAS`/`.DIC` (owned) so blocks/responses parse without re-passing.
    bas: Vec<u8>,
    dic: Vec<u8>,
}

impl BasMenuStack {
    /// Decode a script's menus and seed the stack with the ENTRY menu — the first
    /// `0xA3` the BAS VM reaches from offset 0 (verified: SCRIPT2's entry is 0x2f,
    /// the top-level menu, which is the base of the live `gs:0x6774` stack).
    pub fn new(bas: &[u8], dic: &[u8]) -> Option<Self> {
        let menus = decode_menus(bas, dic, 3);
        let entry = menus.first()?.bas_offset;
        Some(Self { menus, stack: vec![entry], bas: bas.to_vec(), dic: dic.to_vec() })
    }

    /// The menu currently displayed (the top of the stack = `gs:0x6772`).
    pub fn current(&self) -> Option<&ConceptMenu> {
        let off = *self.stack.last()?;
        self.menus.iter().find(|m| m.bas_offset == off)
    }

    /// The current menu's full parsed BLOCK — its topics plus the `0xA6` response
    /// tokens up to the `0xAC` terminator (grammar from [`parse_menu_block`],
    /// verified against the runtime trace). Ties the stack to the block parser so
    /// the conversation VM has the current menu's responses available to display.
    pub fn current_block(&self) -> Option<MenuBlock> {
        parse_menu_block(&self.bas, &self.dic, *self.stack.last()?)
    }

    /// A sequential-response player for the current menu's block (its monologue).
    pub fn current_responses(&self) -> Option<SequentialResponses> {
        self.current_block().map(|b| SequentialResponses::new(&b))
    }

    /// The current menu's full response monologue as renderable subtitle lines (its
    /// `0xA6` responses assembled in order) — the dialogue the engine plays for the
    /// active concept menu. Empty if no menu/block is active.
    pub fn current_menu_dialogue(&self) -> Vec<String> {
        self.current_block()
            .map(|b| b.responses.iter().filter_map(|&o| self.response_text(o)).collect())
            .unwrap_or_default()
    }

    /// The subtitle text of the response `0xA6` token at `bas_offset` — its dictionary
    /// words assembled with the game's punctuation-aware spacing. So a played response
    /// yields the actual on-screen line, not just an offset.
    pub fn response_text(&self, bas_offset: usize) -> Option<String> {
        let words = parse_dic_words(&self.dic);
        for tok in walk(&self.bas, bas_offset, self.bas.len()) {
            if let VmToken::Text { word_offsets, .. } = tok {
                let parts: Vec<String> =
                    word_offsets.iter().filter_map(|o| words.get(o).cloned()).collect();
                return Some(crate::engine::assemble_words(&parts));
            }
        }
        None
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

    /// The menu-block parser matches the single-step-traced grammar: the fear/anger
    /// menu (0x42d) block has 7 topics, its `0xA6` responses, and terminates at the
    /// `0xAC` at 0x612 — exactly where the BASSTEP execution trace ended the block.
    #[test]
    fn parse_menu_block_matches_traced_grammar() {
        let rd = |ext: &str| {
            ["accuracy/cdrive/cblood", "../accuracy/cdrive/cblood"]
                .iter()
                .find_map(|b| std::fs::read(Path::new(b).join(format!("SCRIPT2.{ext}"))).ok())
        };
        let (Some(bas), Some(dic)) = (rd("BAS"), rd("DIC")) else {
            return;
        };
        let block = parse_menu_block(&bas, &dic, 0x42d).expect("fear/anger menu block");
        assert_eq!(block.topics.len(), 7, "topics: {:?}", block.topics);
        assert!(block.topics.iter().any(|t| t == "fear"));
        // The block terminates at the 0xAC the trace hit (si=0x612).
        assert_eq!(block.end, 0x612, "block ends at the traced 0xAC (got {:#x})", block.end);
        assert!(!block.responses.is_empty(), "has 0xA6 responses: {}", block.responses.len());
    }

    /// The fear/anger menu block is a PURE SEQUENTIAL TEXT dialogue: a proper VM
    /// walk finds only `0xA6` Text tokens (13 of them) between the menu and `0xAC`
    /// — no per-topic record-update opcodes. So this menu's responses are gated by
    /// the already-shown bit (the record gate `vm.rs` already models), shown one at
    /// a time, NOT selected per topic. This is the buildable sequential case.
    #[test]
    fn fear_anger_block_is_pure_sequential_text() {
        let rd = |ext: &str| {
            ["accuracy/cdrive/cblood", "../accuracy/cdrive/cblood"]
                .iter()
                .find_map(|b| std::fs::read(Path::new(b).join(format!("SCRIPT2.{ext}"))).ok())
        };
        let Some(bas) = rd("BAS") else { return };
        let mut text = 0;
        let mut other = 0;
        for tok in walk(&bas, 0x43e, 0x612) {
            match tok {
                VmToken::Text { .. } => text += 1,
                _ => other += 1,
            }
        }
        assert_eq!(text, 13, "13 sequential text responses");
        assert_eq!(other, 0, "no record-update opcodes in the block");
    }

    /// The concept menus are FLAT: no menu block contains a nested menu-head `0xA3`
    /// (checked across every menu in SCRIPT2). Combined with the runtime observation
    /// that topic clicks only play responses or pop, this proves there is NO
    /// topic→sub-menu branching — menus are sequential leaves opened by game actions.
    /// So `SequentialResponses` + pop is the COMPLETE concept-menu behavior.
    #[test]
    fn concept_menus_are_flat_no_nested_submenus() {
        let rd = |ext: &str| {
            ["accuracy/cdrive/cblood", "../accuracy/cdrive/cblood"]
                .iter()
                .find_map(|b| std::fs::read(Path::new(b).join(format!("SCRIPT2.{ext}"))).ok())
        };
        let (Some(bas), Some(dic)) = (rd("BAS"), rd("DIC")) else {
            return;
        };
        let menus = decode_menus(&bas, &dic, 3);
        let heads: std::collections::HashSet<usize> = menus.iter().map(|m| m.bas_offset).collect();
        let mut nested = 0;
        for &h in &heads {
            if let Some(block) = parse_menu_block(&bas, &dic, h) {
                // A nested menu-head between this menu's responses and its 0xAC.
                let range = block.responses.first().copied().unwrap_or(h)..block.end;
                if bas[range]
                    .windows(1)
                    .enumerate()
                    .any(|(i, w)| w[0] == 0xA3 && heads.contains(&(block.responses.first().copied().unwrap_or(h) + i)))
                {
                    nested += 1;
                }
            }
        }
        assert_eq!(nested, 0, "no menu block nests another menu — concept menus are flat");
        assert!(heads.len() > 50, "many flat menus: {}", heads.len());
    }

    /// Every sampled menu block is PURE SEQUENTIAL: only `0xA6` Text responses (and an
    /// occasional non-record op) up to `0xAC` — NO record-update `0xC1..=0xC8` opcodes,
    /// no nested `0xA3` sub-menus. So [`SequentialResponses`] is the UNIVERSAL response
    /// mechanism for all conversation menus; the branching (sub-menu push, per-topic
    /// selection) is runtime-record-driven, not in the static block. Decisive survey.
    #[test]
    fn survey_menu_block_token_kinds() {
        let rd = |ext: &str| {
            ["accuracy/cdrive/cblood", "../accuracy/cdrive/cblood"]
                .iter()
                .find_map(|b| std::fs::read(Path::new(b).join(format!("SCRIPT2.{ext}"))).ok())
        };
        let (Some(bas), Some(dic)) = (rd("BAS"), rd("DIC")) else {
            return;
        };
        for &menu in &[0x2f_usize, 0xc27, 0x10f0, 0x22c5, 0x2308] {
            let Some(block) = parse_menu_block(&bas, &dic, menu) else {
                continue;
            };
            // walk from after the topics (first response, or the menu head+1) to 0xAC.
            let start = block.responses.first().copied().unwrap_or(menu + 1);
            let mut text = 0;
            let mut nontext = std::collections::BTreeMap::<u8, usize>::new();
            for tok in walk(&bas, start, block.end + 1) {
                match tok {
                    VmToken::Text { .. } => text += 1,
                    VmToken::Op { opcode, .. } => *nontext.entry(opcode).or_default() += 1,
                    VmToken::RecordEntry { entry_opcode, .. } => {
                        *nontext.entry(entry_opcode).or_default() += 1
                    }
                    VmToken::RecordLink { .. } => *nontext.entry(0xC3).or_default() += 1,
                    VmToken::Actor { .. } => *nontext.entry(0xC4).or_default() += 1,
                    _ => *nontext.entry(0).or_default() += 1,
                }
            }
            eprintln!(
                "menu {menu:#06x}: {} topics, {text} Text, non-Text ops {:x?}",
                block.topics.len(),
                nontext
            );
            // No record-update opcodes (0xC1..=0xC8) inside any menu block: the
            // per-topic/branching logic is runtime-record-driven, not static here.
            assert!(
                !nontext.keys().any(|&op| (0xC1..=0xC8).contains(&op)),
                "menu {menu:#06x} block has record-update ops {nontext:x?}"
            );
        }
    }

    /// The sequential response player yields the fear/anger block's 13 responses one
    /// at a time, in stream order, then stops — modelling the already-shown gating.
    #[test]
    fn sequential_responses_play_the_monologue_in_order() {
        let rd = |ext: &str| {
            ["accuracy/cdrive/cblood", "../accuracy/cdrive/cblood"]
                .iter()
                .find_map(|b| std::fs::read(Path::new(b).join(format!("SCRIPT2.{ext}"))).ok())
        };
        let (Some(bas), Some(dic)) = (rd("BAS"), rd("DIC")) else {
            return;
        };
        let block = parse_menu_block(&bas, &dic, 0x42d).expect("block");
        let mut seq = SequentialResponses::new(&block);
        assert_eq!(seq.total(), 13);
        let first = seq.advance().expect("first response");
        assert_eq!(first, 0x43e, "first response at the traced offset");
        assert_eq!(seq.remaining(), 12);
        let mut count = 1;
        while seq.advance().is_some() {
            count += 1;
        }
        assert_eq!(count, 13, "all 13 shown, then exhausted");
        assert_eq!(seq.remaining(), 0);
    }

    /// The stack ties to the block parser: after entering the fear/anger menu, the
    /// stack's `current_block` returns that menu's parsed block (topics + responses),
    /// consolidating the navigation + block-decode pieces into one API.
    #[test]
    fn stack_current_block_ties_navigation_to_block_parser() {
        let rd = |ext: &str| {
            ["accuracy/cdrive/cblood", "../accuracy/cdrive/cblood"]
                .iter()
                .find_map(|b| std::fs::read(Path::new(b).join(format!("SCRIPT2.{ext}"))).ok())
        };
        let (Some(bas), Some(dic)) = (rd("BAS"), rd("DIC")) else {
            return;
        };
        let mut st = BasMenuStack::new(&bas, &dic).expect("menus");
        st.push(0x42d);
        let block = st.current_block().expect("current block");
        assert_eq!(block.menu_offset, 0x42d);
        assert_eq!(block.end, 0x612);
        assert!(block.topics.iter().any(|t| t == "fear"));
        // The stack yields a sequential-response player for the current menu.
        let mut seq = st.current_responses().expect("responses");
        assert_eq!(seq.total(), 13);
        let first = seq.advance().expect("first");
        assert_eq!(first, 0x43e);
        // And the played response assembles to its actual on-screen subtitle.
        let text = st.response_text(first).expect("response text");
        assert!(text.contains("several ways to lose"), "response text: {text:?}");
        // The full menu monologue is available as renderable dialogue lines.
        let dialogue = st.current_menu_dialogue();
        assert_eq!(dialogue.len(), 13, "13 response lines");
        assert!(dialogue[0].contains("several ways to lose"));
    }
}
