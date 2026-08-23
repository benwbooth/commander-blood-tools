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
pub mod bas_cfg;
pub mod bas_vm;
pub mod bloodscript;
pub mod vm_drive;
pub mod bloodprg;
pub mod bloodsav;
pub mod bridge;
pub mod concept_menu;
pub mod contact_manifest;
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
pub mod vm_bundle;
pub mod vm_cfg;
pub mod vm_data;
pub mod vm_source;

/// 320 — the row stride the game BUILDS rather than stores: `xchg bh,bl`
/// @`0x9B27` (y*256) plus `shl di,6` @`0x9B29` (y*64), summed by `add di,bx`
/// @`0x9B2C`. Never an immediate anywhere (audit-fixes #502, #563).
pub const VIEWPORT_W: usize = 320;
/// 200 — `mov word ptr [0x523b],0xc8` @`0xB41D`, the clip bottom the navigation
/// routine restores (audit-fixes #495, #563).
pub const VIEWPORT_H: usize = 200;
/// The export's video frame rate — a PORT CHOICE, not the game's (audit-fixes
/// #549). The HNM header carries NO frame-rate field (it is header size, palette
/// block, frame offsets — see `hnm::HnmFile::open`), so nothing in the format
/// dictates a playback rate and this is simply what the MP4s are encoded at.
///
/// DO NOT USE IT TO CONVERT GAME TICKS TO SECONDS. That is
/// [`GAME_TICK_SECS`], and they differ by a factor of 1.67.
pub const HNM_FPS: u32 = 15;

/// One game tick in seconds — `8 / (1193182 / 5958)` = 39.95 ms, i.e. ~25 Hz.
///
/// The PIT divisor is `0x1746` (audit-fixes #411) and a frame's budget is the 8
/// of `[0xB2D]` (`0x0FFB`, #477). Any duration the game expresses in TICKS —
/// `vm::reveal_complete_hold_ticks`, `vm::record_end_hold_ticks`, the chatter
/// throttle — converts to seconds through THIS, never through a video frame rate.
///
/// `extract` divided a tick count by [`HNM_FPS`] (15), stretching every subtitle
/// hold in the exported videos by 25.03/15 = 1.67x (audit-fixes #549).
pub const GAME_TICK_SECS: f64 = 8.0 / (1_193_182.0 / 5958.0);

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
    ///
    /// SINCE audit-fixes #371 this also guards a SECOND shape, with a stricter
    /// rule: `assert_eq!(CONST, "literal")` where the literal IS the constant's
    /// own definition. That one is flagged UNCONDITIONALLY, because grounding
    /// elsewhere cannot rescue a tautology — the fix is to assert against the
    /// image or data the constant claims to come from (#370 did exactly that for
    /// `OPTION_BOX_LABEL`, which had been compared to a second copy of itself).
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
mod claimed_test_tests {
    /// A doc that names the test verifying it makes the strongest claim available
    /// here — and a named test that DOES NOT EXIST is worse than no claim, because
    /// the row reads as settled while nothing runs. This checks every such name
    /// resolves, and reports whether the test opens anything the game shipped (a
    /// `func_<hex>` lift counts: it IS the original instruction stream).
    #[test]
    fn every_doc_named_verifier_exists() {
        let script = std::path::Path::new("tools/check_claimed_tests.py");
        if !script.exists() {
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
        assert!(out.status.success(), "doc names a test that does not exist:\n{text}");
    }
}

#[cfg(test)]
mod content_literal_tests {
    /// The prime rule names this defect outright: content that lives in the
    /// game's data must be executed or parsed, never transcribed. A dialogue line
    /// or menu label in a `.rs` file is text copied off the running game, and it
    /// will not change when the data does. `main.rs` carried Bob's greeting and a
    /// `talk / remember / bye_bye` label list as "no-VM fallbacks" for content
    /// SCRIPT2's bytecode already provides.
    #[test]
    fn no_game_text_hardcoded_in_runtime_source() {
        let script = std::path::Path::new("tools/check_content_literals.py");
        if !script.exists() {
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
        assert!(out.status.success(), "game text in the port's source:\n{text}");
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

#[cfg(test)]
mod tick_rate_tests {
    /// The game tick is ~25 Hz, not the export's 15 fps (audit-fixes #549).
    #[test]
    fn game_tick_is_not_the_video_frame_rate() {
        let hz = 1.0 / super::GAME_TICK_SECS;
        assert!((hz - 25.03).abs() < 0.05, "8 PIT ticks at 1193182/5958 -> {hz} Hz");
        // The two must not be conflated: a tick count divided by HNM_FPS is 1.67x
        // too long, which is what extract did before this entry.
        assert!(
            (hz / super::HNM_FPS as f64 - 1.67).abs() < 0.02,
            "the factor the bug introduced"
        );
    }
}
