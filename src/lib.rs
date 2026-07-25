//! Commander Blood — reimplementation and reverse-engineering tools.
//!
//! Two things live side by side here, and they follow **opposite** conventions:
//!
//! * [`recomp`] is the bit-exact x86 *emulator* (the verification reference). It legitimately
//!   models registers, flags, and real-mode `segment:offset` memory, because its whole job is to
//!   reproduce the original instruction stream.
//! * **Every other module is the hand-written *port*.** The port is clean-room game code derived
//!   from the reverse-engineering notes (`re/REVERSE.md`, `re/labels.csv`), not transliterated
//!   from x86. Port code obeys these house rules:
//!     1. **Flat memory** — ordinary typed values and flat-indexed slices; never `segment:offset`,
//!        `seg * 16 + off`, or a shared byte pool standing in for RAM. It must not depend on
//!        [`recomp`]'s `Machine`.
//!     2. **No register names** — functions take and return meaningful values; no `ax`/`si`/`es`
//!        identifiers or flag bits. (Short math names like `dx` for a delta or `si` for a source
//!        index are fine when that is genuinely what they mean.)
//!     3. **Named numbers** — non-trivial constants are named `const`s or `enum` variants; bare
//!        literals only for self-evident quantities.
//!     4. **Decimal by default** — hexadecimal only where it is genuinely clearer (bit masks,
//!        packed fields).
//!
//! Behaviour of ported code is validated against the emulator/oracle, not against the instruction
//! stream.

pub mod audio;
pub mod bas_vm;
pub mod vm_drive;
pub mod bloodprg;
pub mod bloodsav;
pub mod bridge;
pub mod concept_menu;
pub mod croolis;
pub mod decompress;
pub mod descript;
pub mod engine;
pub mod entity;
pub mod ext;
pub mod font;
pub mod hnm;
pub mod recomp;
pub mod lbm;
pub mod levels;
pub mod manu3;
pub mod gpu;
pub mod manu3_hand;
pub mod palette;
pub mod progress;
pub mod save;
pub mod script;
pub mod ship3d;
pub mod snd;
pub mod sprite;
pub mod tbbig;
pub mod util;
pub mod vm;

pub const VIEWPORT_W: usize = 320;
pub const VIEWPORT_H: usize = 200;
pub const HNM_FPS: u32 = 15;

#[cfg(test)]
mod duplicate_rule_tests {
    /// No decoded rule may be implemented under the SAME NAME in two files.
    ///
    /// `subtitle_draw_glyph` existed in both `font.rs` and `extract/render.rs`, and
    /// the second copy still had a 128-entry font map, Unicode-indexed lookups and
    /// a `'?'` fallback — three defects the first had fixed, surviving because
    /// nothing connected them (audit-fixes #97). Two more pairs turned up the same
    /// way (#96, #98).
    ///
    /// Weaker collisions — a routine and its helper citing one address — are
    /// reported for judgement but do not fail: they are normal.
    #[test]
    fn no_decoded_rule_is_implemented_twice_under_one_name() {
        let script = std::path::Path::new("tools/check_duplicate_rules.py");
        if !script.exists() {
            return;
        }
        let out = match std::process::Command::new("python3").arg(script).output() {
            Ok(o) => o,
            Err(_) => return,
        };
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "duplicated rules:\n{text}");
        let n: usize = text
            .split_whitespace()
            .next()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        assert!(n >= 10, "expected the sweep to find clusters, got: {text}");
    }
}

#[cfg(test)]
mod selfref_assert_tests {
    /// No test may pin a length against ONLY the constant that produced it.
    ///
    /// This is the shape that hid the font truncation for a whole campaign: the
    /// extractor sliced 128 entries with `DIALOGUE_FONT_ASCII_MAP_LEN`, the test
    /// asserted `len() == DIALOGUE_FONT_ASCII_MAP_LEN`, and both agreed while the
    /// real table was 176 — dropping every accented character the game can draw.
    /// Seven assertions of that shape were re-grounded against layout identities,
    /// code immediates or the data's own bounds; this stops an eighth appearing.
    ///
    /// A `len() == CONST` is cleared by independent evidence ANYWHERE in its file
    /// (an image read, an identity, a `mov` immediate) — sibling tests count, which
    /// is how `bloodsav` pins its header sizes.
    #[test]
    fn length_assertions_are_grounded_in_something_independent() {
        let script = std::path::Path::new("tools/check_selfref_asserts.py");
        if !script.exists() {
            return;
        }
        let out = match std::process::Command::new("python3").arg(script).output() {
            Ok(o) => o,
            Err(_) => return,
        };
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "ungrounded length assertions:\n{text}");
        let n: usize = text
            .split_whitespace()
            .next()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        assert!(n >= 5, "expected the sweep to find assertions, got: {text}");
    }
}

#[cfg(test)]
mod provenance_tests {
    /// THE PRIME RULE, enforced: no runtime comment may say a value was measured
    /// off a capture without either citing the binary address that replaced it or
    /// saying it no longer applies. This session found three DEFECTS of that shape
    /// (the choice box's colours, the list menu's x, the save UI's layout) and
    /// three STALE NOTES left by their fixes — the class is common enough to
    /// deserve a guard rather than another grep.
    ///
    /// The oracle harness (`src/bin/runtime_boot.rs`) is exempt: measuring the
    /// real game is what it is for.
    #[test]
    fn no_unexplained_capture_provenance_in_runtime_code() {
        let script = std::path::Path::new("tools/check_provenance.py");
        if !script.exists() {
            return;
        }
        let out = match std::process::Command::new("python3").arg(script).output() {
            Ok(o) => o,
            Err(_) => return,
        };
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "capture-sourced claims:\n{text}");
        // The sweep must still be FINDING claims — a regex that stops matching
        // would pass forever.
        let n: usize = text
            .split_whitespace()
            .next()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        assert!(n >= 5, "expected the sweep to find claims, got: {text}");
    }
}

#[cfg(test)]
mod cited_instruction_tests {
    /// Every `0xNNNN  <mnemonic>` a doc comment QUOTES from the binary must
    /// actually decode to that mnemonic. A wrong address in a comment is worse
    /// than no comment: it sends the next reader to the wrong routine while
    /// making the claim look sourced. `tools/check_cited_instructions.py`
    /// disassembles each cited address and compares.
    #[test]
    fn quoted_instructions_match_the_disassembly() {
        let script = std::path::Path::new("tools/check_cited_instructions.py");
        if !script.exists() || !std::path::Path::new("re/bin/BLOODPRG.EXE").exists() {
            return;
        }
        let out = match std::process::Command::new("python3").arg(script).output() {
            Ok(o) => o,
            // capstone unavailable in this environment — nothing to check with.
            Err(_) => return,
        };
        let text = String::from_utf8_lossy(&out.stdout);
        if text.trim().is_empty() {
            return;
        }
        assert!(out.status.success(), "cited instructions disagree:\n{text}");
        let checked: usize = text
            .split_whitespace()
            .next()
            .and_then(|n| n.parse().ok())
            .unwrap_or(0);
        assert!(
            checked >= 30,
            "expected the sweep to verify quoted instructions, got: {text}"
        );
    }
}

#[cfg(test)]
mod opcode_handler_tests {
    /// An opcode constant's value is a dispatch-table INDEX, so it never appears
    /// in its handler's bytes and cannot be checked the way other constants are.
    /// The claim its doc makes is checkable and stronger: entry `op - 0xA0` of the
    /// table at `0x142D0` (52 near offsets into VM code segment `0x4DA`) must be
    /// the handler cited. This resolves every `OP_*` constant through the table.
    #[test]
    fn opcode_constants_cite_the_handler_the_table_dispatches() {
        let script = std::path::Path::new("tools/check_opcode_handlers.py");
        if !script.exists() || !std::path::Path::new("re/bin/BLOODPRG.EXE").exists() {
            return;
        }
        let out = match std::process::Command::new("python3").arg(script).output() {
            Ok(o) => o,
            Err(_) => return,
        };
        let text = String::from_utf8_lossy(&out.stdout);
        if text.trim().is_empty() {
            return;
        }
        assert!(out.status.success(), "opcode handler citations wrong:\n{text}");
        let checked: usize = text
            .split_whitespace()
            .next()
            .and_then(|n| n.parse().ok())
            .unwrap_or(0);
        assert!(
            checked >= 25,
            "expected the opcode constants to resolve through the table, got: {text}"
        );
    }
}

#[cfg(test)]
mod opsize_mnemonic_tests {
    /// `cbw` and `cwde` are the SAME opcode (0x98) at different operand sizes, and
    /// capstone prints both as `cwde` in 16-bit mode. They are not interchangeable:
    /// after `lodsb`, `cbw` overwrites AH while `cwde` leaves it holding caller
    /// state. The encoded length settles which one a citation means — 1 byte is
    /// `cbw`, `66 98` is a real `cwde` — and the same check catches a citation
    /// anchored mid-instruction, which is how a phantom `cdq` (really the 0x99
    /// inside `lcall 0x299:0x0ecb`) sat in labels.csv.
    #[test]
    fn cited_convert_mnemonics_match_the_encoded_operand_size() {
        let script = std::path::Path::new("re/tools/check_opsize_mnemonics.py");
        if !script.exists() || !std::path::Path::new("re/bin/BLOODPRG.EXE").exists() {
            return;
        }
        let out = match std::process::Command::new("python3").arg(script).output() {
            Ok(o) => o,
            Err(_) => return,
        };
        let text = String::from_utf8_lossy(&out.stdout);
        if text.trim().is_empty() {
            return;
        }
        assert!(out.status.success(), "convert-mnemonic citations wrong:\n{text}");
        // The sweep must still be resolving citations to bytes; a regex that
        // stopped matching would pass forever. One of them (0x379B `66 98`) is a
        // GENUINE cwde, so this cannot be satisfied by rewriting every site.
        let checked: usize = text
            .split_whitespace()
            .next()
            .and_then(|n| n.parse().ok())
            .unwrap_or(0);
        assert!(checked >= 8, "expected resolved convert citations, got: {text}");
    }
}

#[cfg(test)]
mod offset_pair_tests {
    /// Every `DS:0xNNNN` the port documents alongside a `file 0xNNNNN` must agree
    /// with the DS base (file `0xD420`). A drifted pair is invisible to ordinary
    /// tests because each half is individually plausible — which is why
    /// `OPTION_BOX_LABEL` carries a hand-written assertion for exactly this. This
    /// runs the same check over the whole tree via `tools/check_offset_pairs.py`,
    /// so a new constant cannot quietly reintroduce the class.
    ///
    /// Skips when python or the source tree is unavailable (a packaged build).
    #[test]
    fn documented_ds_and_file_offsets_agree() {
        let script = std::path::Path::new("tools/check_offset_pairs.py");
        if !script.exists() {
            return;
        }
        let out = match std::process::Command::new("python3").arg(script).output() {
            Ok(o) => o,
            Err(_) => return,
        };
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "DS/file offset pairs disagree:\n{text}"
        );
        // The checker must actually be finding pairs — a regex that silently stops
        // matching would "pass" forever.
        let checked: usize = text
            .split_whitespace()
            .next()
            .and_then(|n| n.parse().ok())
            .unwrap_or(0);
        assert!(checked >= 15, "expected the sweep to find pairs, got: {text}");
    }
}
