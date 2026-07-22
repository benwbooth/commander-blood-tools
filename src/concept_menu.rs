//! Concept-menu (topic-list) decoder for the location/conversation scripts.
//!
//! The game's conversation system presents CONCEPT MENUS — vertical lists of
//! topic words the player clicks to steer a dialogue (psychotherapy topics,
//! location destinations, shop items, per-character conversation subjects). Each
//! menu is emitted by the script VM's **opcode `0xA3`** in `SCRIPTn.BAS`,
//! immediately followed by a run of little-endian `u16` offsets into
//! `SCRIPTn.DIC`; each offset points at a NUL-terminated concept word. The run
//! ends at the first `u16` that is not a valid single-token dictionary offset.
//!
//! This is the label source that was previously RE-pending: the port had to fall
//! back to hard-coded `ONE..NINE` guesses for SCRIPT2 and linear playback for the
//! location scripts. Decoding `0xA3` recovers the REAL labels for every menu in
//! every script directly from the data files (no runtime state needed). Verified:
//! SCRIPT2's psychotherapy menu decodes to exactly the words captured from the
//! live game (`accuracy/captures/bridge/concept_menu.ppm`).

use std::collections::HashMap;

/// One decoded concept menu: the byte offset of its `0xA3` opcode in the `.BAS`
/// image, and the ordered topic labels (as stored — lowercase; the square-caps
/// menu font renders them uppercase).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConceptMenu {
    /// Byte offset of the `0xA3` opcode in the `.BAS` image.
    pub bas_offset: usize,
    /// Ordered concept labels (dictionary words), first is usually `talk`/`bye_bye`.
    pub labels: Vec<String>,
}

/// Parse a `SCRIPTn.DIC` into {byte offset -> word}. Words are NUL-terminated.
fn parse_dic(dic: &[u8]) -> HashMap<u16, String> {
    let mut words = HashMap::new();
    let mut pos = 0usize;
    while pos < dic.len() {
        let start = pos;
        while pos < dic.len() && dic[pos] != 0 {
            pos += 1;
        }
        if pos > start {
            words.insert(start as u16, String::from_utf8_lossy(&dic[start..pos]).into_owned());
        }
        pos += 1;
    }
    words
}

/// A dictionary word is a valid CONCEPT label if it is a single token (no spaces
/// — sentence fragments in the script are multi-word) of a menu-plausible length.
fn is_concept_label(w: &str) -> bool {
    (2..=16).contains(&w.len()) && !w.contains(' ')
}

/// Decode every `0xA3` concept menu in a script's `.BAS`, resolving each entry's
/// `u16` offset through the `.DIC`. Menus with fewer than `min_labels` entries are
/// dropped (a bare `0xA3` byte in unrelated data won't be followed by a long run
/// of valid single-token dictionary offsets, so this reliably rejects noise).
pub fn decode_menus(bas: &[u8], dic: &[u8], min_labels: usize) -> Vec<ConceptMenu> {
    let words = parse_dic(dic);
    let mut menus = Vec::new();
    let mut b = 0usize;
    while b + 1 < bas.len() {
        if bas[b] != 0xA3 {
            b += 1;
            continue;
        }
        let mut labels = Vec::new();
        let mut j = b + 1;
        while j + 1 < bas.len() {
            let off = u16::from_le_bytes([bas[j], bas[j + 1]]);
            match words.get(&off) {
                Some(w) if is_concept_label(w) => {
                    labels.push(w.clone());
                    j += 2;
                }
                _ => break,
            }
        }
        if labels.len() >= min_labels {
            menus.push(ConceptMenu { bas_offset: b, labels });
            b = j;
        } else {
            b += 1;
        }
    }
    menus
}

/// Find the first decoded menu whose (case-insensitive) label set contains all of
/// `required` — used to locate a specific menu (e.g. the destination list) by its
/// known members without depending on its `.BAS` offset.
pub fn find_menu_containing<'a>(menus: &'a [ConceptMenu], required: &[&str]) -> Option<&'a ConceptMenu> {
    menus.iter().find(|m| {
        required.iter().all(|r| {
            m.labels.iter().any(|l| l.eq_ignore_ascii_case(r))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn read_script(n: u32, ext: &str) -> Option<Vec<u8>> {
        for base in ["accuracy/cdrive/cblood", "../accuracy/cdrive/cblood"] {
            let p = Path::new(base).join(format!("SCRIPT{n}.{ext}"));
            if let Ok(b) = std::fs::read(&p) {
                return Some(b);
            }
        }
        None
    }

    /// The psychotherapy concept menu decodes to EXACTLY the words captured from
    /// the live game (concept_menu.ppm), proving the `0xA3` decode is correct.
    #[test]
    fn script2_psychotherapy_menu_matches_live_capture() {
        let (Some(bas), Some(dic)) = (read_script(2, "BAS"), read_script(2, "DIC")) else {
            return;
        };
        let menus = decode_menus(&bas, &dic, 4);
        let psy = find_menu_containing(&menus, &["ego", "super_ego", "libido", "what"])
            .expect("psychotherapy menu present");
        let got: Vec<String> = psy.labels.iter().map(|s| s.to_uppercase()).collect();
        assert_eq!(
            got,
            [
                "TALK", "EGO", "SUPER_EGO", "UNDER_EGO", "END_OF_MONTH", "LIBIDO", "WHO", "WHERE",
                "WHEN", "WHAT", "HOW", "WHY"
            ]
        );
    }

    /// The location scripts expose the destination/planet concepts as `0xA3`
    /// menus — the real nav-topic labels (blocker #3), decoded from disk.
    #[test]
    fn location_scripts_expose_destination_menus() {
        let (Some(bas), Some(dic)) = (read_script(3, "BAS"), read_script(3, "DIC")) else {
            return;
        };
        let menus = decode_menus(&bas, &dic, 4);
        // SCRIPT3 carries the planet-destination menu (corpo/magnus/vista/…).
        let dest = find_menu_containing(&menus, &["corpo", "magnus", "vista", "tumul"])
            .expect("destination menu present");
        assert!(dest.labels.iter().any(|l| l.eq_ignore_ascii_case("pterra")));
        // Plenty of per-character conversation menus are recovered too.
        assert!(menus.len() > 10, "many concept menus decoded: {}", menus.len());
    }
}
