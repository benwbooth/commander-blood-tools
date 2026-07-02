//! Execution-order walker for compiled-BASIC `SCRIPT*.COD` bytecode.
//!
//! This replaces the old "scan a function for `0xA6`/`0xC4` and guess token
//! sizes" approach with a faithful token walk derived by reverse-engineering the
//! VM token decoder in `BLOODPRG.EXE` (`token_advance` @ file 0x62B6, dispatch
//! @ 0x5627). See `re/REVERSE.md` for the full analysis.
//!
//! ## Opcode model (recovered)
//! Valid opcodes are `0xA0..=0xD3` (the VM biases every opcode by `0xA0`). A
//! per-opcode descriptor table at `BLOODPRG.EXE` file 0x14338 (`DS:0x6F18`) gives
//! two bytes per opcode, `[len_mode0, len_mode1_or_sentinel]`:
//! * If the second byte has bit7 set it is a **mode-control sentinel**, and the
//!   token length is `len_mode0`. `0xFF` switches the decoder into mode 1,
//!   `0xFE` back to mode 0, and `0xFD`/`0xFB` additionally consume a following
//!   `0xA1` byte if present.
//! * Otherwise the token length is `len_mode0` in mode 0 or `len_mode1` in mode 1.
//!
//! Length-0 entries are special: `0xA6` is the TEXT token (`A6 b1 b2 b3 b4 b5`
//! then, if `b4 & 0x10`, a u16 loop target, then a `0x0000`-terminated list of
//! dictionary-word offsets). `0xA8/0xAC/0xCC/0xD3` are bare 1-byte opcodes.
//!
//! Status: token decoding is verified byte-exact against the binary (see tests).
//! The pieces here are the foundation for the VM-event renderer that will
//! replace the heuristic in `character.rs`; they are not wired into the live
//! export path yet (hence `#[allow(dead_code)]`). A faithful whole-script walk
//! additionally needs control-flow interpretation — see `walks_real_scripts`.
#![allow(dead_code)]

use serde::Serialize;

/// Per-opcode descriptor bytes for opcodes `0xA0..=0xD3`, transcribed from
/// `BLOODPRG.EXE` file offset 0x14338 (`DS:0x6F18`). `(len_mode0, byte1)` where
/// `byte1` is either `len_mode1` or a mode-control sentinel (bit7 set).
/// Verified against the binary by `tests::table_matches_binary` when
/// `re/bin/BLOODPRG.EXE` is available.
pub const OPCODE_DESC: [(u8, u8); 0x34] = [
    /* A0 */ (0x03, 0xff),
    /* A1 */ (0x01, 0xfe),
    /* A2 */ (0x03, 0x03),
    /* A3 */ (0x03, 0xfb),
    /* A4 */ (0x03, 0x03),
    /* A5 */ (0x04, 0x02),
    /* A6 */ (0x00, 0x00),
    /* A7 */ (0x03, 0x03),
    /* A8 */ (0x00, 0x00),
    /* A9 */ (0x04, 0xff),
    /* AA */ (0x01, 0x01),
    /* AB */ (0x04, 0x04),
    /* AC */ (0x00, 0x00),
    /* AD */ (0x05, 0x05),
    /* AE */ (0x05, 0xfd),
    /* AF */ (0x05, 0xfd),
    /* B0 */ (0x05, 0xfd),
    /* B1 */ (0x07, 0x07),
    /* B2 */ (0x05, 0xfd),
    /* B3 */ (0x05, 0xfd),
    /* B4 */ (0x07, 0x07),
    /* B5 */ (0x07, 0x07),
    /* B6 */ (0x07, 0x07),
    /* B7 */ (0x04, 0xfd),
    /* B8 */ (0x07, 0x07),
    /* B9 */ (0x07, 0x07),
    /* BA */ (0x05, 0xfd),
    /* BB */ (0x05, 0xfd),
    /* BC */ (0x05, 0xfd),
    /* BD */ (0x07, 0x07),
    /* BE */ (0x07, 0x07),
    /* BF */ (0x07, 0x07),
    /* C0 */ (0x07, 0x07),
    /* C1 */ (0x05, 0xfd),
    /* C2 */ (0x05, 0xfd),
    /* C3 */ (0x05, 0xfd),
    /* C4 */ (0x05, 0xfd),
    /* C5 */ (0x05, 0xfd),
    /* C6 */ (0x05, 0xfd),
    /* C7 */ (0x05, 0xfd),
    /* C8 */ (0x05, 0xfd),
    /* C9 */ (0x03, 0xfd),
    /* CA */ (0x05, 0x05),
    /* CB */ (0x06, 0x06),
    /* CC */ (0x00, 0x00),
    /* CD */ (0x07, 0xfd),
    /* CE */ (0x01, 0x01),
    /* CF */ (0x01, 0x01),
    /* D0 */ (0x01, 0x01),
    /* D1 */ (0x01, 0x01),
    /* D2 */ (0x02, 0x02),
    /* D3 */ (0x00, 0x00),
];

pub const OP_MIN: u8 = 0xA0;
pub const OP_MAX: u8 = 0xD3;
pub const OP_TEXT: u8 = 0xA6;
pub const OP_ACTOR: u8 = 0xC4;

/// Opcodes whose descriptor length is 0 (other than `0xA6`): the VM advances
/// past them with helper `0x6293`, which scans byte-by-byte for a `0x0000` word
/// terminator and skips it (plus one more byte if a third zero follows). So
/// these are variable-length: `opcode <bytes...> 00 00`.
const VAR_TERMINATED: [u8; 4] = [0xA8, 0xAC, 0xCC, 0xD3];

/// Replicates helper `0x6293`: from `start`, scan byte-by-byte until a `0x0000`
/// word, skip it, then skip one extra byte if it is also zero. Returns the
/// offset just past the terminator.
fn scan_zero_word(cod: &[u8], start: usize, end: usize) -> usize {
    let mut p = start;
    while p + 1 < end && !(cod[p] == 0 && cod[p + 1] == 0) {
        p += 1;
    }
    p += 2;
    if p < end && cod.get(p) == Some(&0) {
        p += 1;
    }
    p.min(end)
}

/// A single decoded token from a COD stream, in execution order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum VmToken {
    /// `0xA6` TEXT token.
    Text {
        offset: usize,
        /// `b1:b2` — index into the per-line record table (`gs:0x6724`).
        line_index: u16,
        /// `b3` — voice/speaker selector (`0xFF` = none).
        voice_selector: u8,
        /// `b4` — control flags (bit3 `0x08`=skip, bit4 `0x10`=loop, …).
        flags_b4: u8,
        /// `b5` — bit7 `0x80` = active/display flag.
        flags_b5: u8,
        /// Loop target word present when `b4 & 0x10`.
        loop_target: Option<u16>,
        /// `0x0000`-terminated list of `SCRIPT*.DIC` word offsets.
        word_offsets: Vec<u16>,
    },
    /// `0xC4` actor/object reference (operand is the first u16 after the opcode).
    Actor {
        offset: usize,
        operand: u16,
        len: usize,
    },
    /// Any other opcode; raw length recorded.
    Op {
        offset: usize,
        opcode: u8,
        len: usize,
    },
    /// Decoder fell off the rails (byte outside `0xA0..=0xD3` where a token was
    /// expected). Walking stops; the offset is where it happened.
    Invalid { offset: usize, byte: u8 },
}

/// Walk `cod[start..end]` in execution order, yielding tokens. Stops at `end`,
/// at the `0xFF` end marker, or at the first byte that cannot be a token.
pub fn walk(cod: &[u8], start: usize, end: usize) -> Vec<VmToken> {
    let end = end.min(cod.len());
    let mut pos = start;
    let mut mode1 = false; // decoder mode (gs:0x67AD); false = mode 0
    let mut out = Vec::new();

    while pos < end {
        let op = cod[pos];
        if op == 0xFF {
            break; // end-of-program marker (executor: `cmp al,0xFF; je end`)
        }
        if !(OP_MIN..=OP_MAX).contains(&op) {
            out.push(VmToken::Invalid {
                offset: pos,
                byte: op,
            });
            break;
        }
        let (b0, b1) = OPCODE_DESC[(op - OP_MIN) as usize];

        if op == OP_TEXT {
            match decode_text(cod, pos, end) {
                Some((tok, next)) => {
                    out.push(tok);
                    pos = next;
                }
                None => {
                    out.push(VmToken::Invalid {
                        offset: pos,
                        byte: op,
                    });
                    break;
                }
            }
            continue;
        }

        if VAR_TERMINATED.contains(&op) {
            let next = scan_zero_word(cod, pos + 1, end);
            out.push(VmToken::Op {
                offset: pos,
                opcode: op,
                len: next - pos,
            });
            pos = next;
            continue;
        }

        // Determine token length + any mode change.
        let len;
        if b1 & 0x80 != 0 {
            // mode-control sentinel: length is b0, plus a possible 0xA1 skip.
            let mut l = b0 as usize;
            match b1 {
                0xFF => mode1 = true,
                0xFE => mode1 = false,
                0xFD | 0xFB => {
                    if cod.get(pos + 1) == Some(&0xA1) {
                        l += 1;
                    }
                }
                _ => {}
            }
            len = l.max(1);
        } else {
            len = (if mode1 { b1 } else { b0 } as usize).max(1);
        }

        if op == OP_ACTOR {
            let operand = read_u16(cod, pos + 1).unwrap_or(0);
            out.push(VmToken::Actor {
                offset: pos,
                operand,
                len,
            });
        } else {
            out.push(VmToken::Op {
                offset: pos,
                opcode: op,
                len,
            });
        }
        pos += len;
    }
    out
}

/// Decode an `0xA6` TEXT token starting at `pos`. Returns the token and the
/// offset just past it, or `None` if malformed.
fn decode_text(cod: &[u8], pos: usize, end: usize) -> Option<(VmToken, usize)> {
    // A6 b1 b2 b3 b4 b5  [loop_target?]  w0 w1 ... 0x0000
    if pos + 6 > end {
        return None;
    }
    let line_index = read_u16(cod, pos + 1)?;
    let b3 = cod[pos + 3];
    let b4 = cod[pos + 4];
    let b5 = cod[pos + 5];
    // The active/display flag (bit7 of b5) is set in real data; a token without
    // it is still structurally valid, so we don't reject on it here.
    let mut p = pos + 6;
    let loop_target = if b4 & 0x10 != 0 {
        let lt = read_u16(cod, p)?;
        p += 2;
        Some(lt)
    } else {
        None
    };
    let mut word_offsets = Vec::new();
    loop {
        let w = read_u16(cod, p)?;
        p += 2;
        if w == 0 {
            break;
        }
        word_offsets.push(w);
        if word_offsets.len() > 512 || p > end {
            return None;
        }
    }
    Some((
        VmToken::Text {
            offset: pos,
            line_index,
            voice_selector: b3,
            flags_b4: b4,
            flags_b5: b5,
            loop_target,
            word_offsets,
        },
        p,
    ))
}

#[inline]
fn read_u16(cod: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*cod.get(at)?, *cod.get(at + 1)?]))
}

// ---------------------------------------------------------------------------
// Bounded state interpreter (runtime location/speaker recovery)
//
// Background/speaker are runtime state: a character's current location lives in
// field `obj+24` of the VM state area (loaded from SCRIPT*.VAR; see REVERSE.md).
// The script mutates it via the assignment opcodes. This interpreter executes
// those assignments while walking, so we can read `state[actor+24]` at each
// 0xA6 line instead of the static initial value.
//
// Opcodes executed (decoded from BLOODPRG.EXE):
//   * 0x6863 family (B1/B4/B5/B6/BE/BF/C0), 7 bytes:
//       op [op1:u16] [operator:u8] [op2mode:u8] [op2:u16]
//       operator 0xF5=set, 0xF6=add, 0xF7=sub; op2mode 0xC0/0xC2 => op2 indirect
//       (`state[op2]`). Writes `state[op1]`.
//   * 0xC4: actor reference; operand = object_offset + 0x3A (talk field).
// NOTE: this is a LINEAR pass — it does not yet evaluate the 0xAF-family
// conditionals/branches, so a value the game would skip can still be applied.
// Adequate for deterministic cutscene runs; see REVERSE.md for the caveat.

const ASSIGN_7: [u8; 7] = [0xB1, 0xB4, 0xB5, 0xB6, 0xBE, 0xBF, 0xC0];
const TALK_FIELD: u16 = 0x3A;
const LOCATION_FIELD: u16 = 24;

/// A `0xA6` line's resolved runtime scene state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct LineState {
    pub offset: usize,
    /// Object offset of the current speaker (from the last `0xC4`), if any.
    pub actor_offset: Option<u16>,
    /// The speaker's current location object offset (`state[actor+24]`), if a
    /// speaker is known.
    pub location_offset: Option<u16>,
}

/// Read a u16 from `state` (the mutable VAR image) at byte address `addr`.
fn state_u16(state: &[u8], addr: u16) -> u16 {
    let a = addr as usize;
    if a + 1 < state.len() {
        u16::from_le_bytes([state[a], state[a + 1]])
    } else {
        0
    }
}

fn state_set_u16(state: &mut [u8], addr: u16, val: u16) {
    let a = addr as usize;
    if a + 1 < state.len() {
        state[a] = (val & 0xFF) as u8;
        state[a + 1] = (val >> 8) as u8;
    }
}

/// Walk `cod`, executing assignment opcodes against a copy of `var` (the initial
/// state image), and return the resolved scene state at every `0xA6` line.
pub fn interpret_line_states(cod: &[u8], var: &[u8]) -> Vec<LineState> {
    let mut state = var.to_vec();
    let mut actor: Option<u16> = None;
    let mut out = Vec::new();
    let mut pos = 0usize;
    let mut mode1 = false;
    let end = cod.len();

    while pos < end {
        let op = cod[pos];
        if op == 0xFF || !(OP_MIN..=OP_MAX).contains(&op) {
            break;
        }
        let (b0, b1) = OPCODE_DESC[(op - OP_MIN) as usize];

        if op == OP_ACTOR {
            if let Some(operand) = read_u16(cod, pos + 1) {
                actor = Some(operand.wrapping_sub(TALK_FIELD));
            }
        }
        if ASSIGN_7.contains(&op) && pos + 7 <= end {
            let op1 = read_u16(cod, pos + 1).unwrap_or(0);
            let operator = cod[pos + 3];
            let op2mode = cod[pos + 4];
            let op2 = read_u16(cod, pos + 5).unwrap_or(0);
            let value = if op2mode == 0xC0 || op2mode == 0xC2 {
                state_u16(&state, op2)
            } else {
                op2
            };
            let cur = state_u16(&state, op1);
            let next = match operator {
                0xF5 => Some(value),
                0xF6 => Some(cur.wrapping_add(value)),
                0xF7 => Some(cur.wrapping_sub(value)),
                _ => None, // comparison operators: no state write here
            };
            if let Some(v) = next {
                state_set_u16(&mut state, op1, v);
            }
        }

        if op == OP_TEXT {
            let location_offset = actor.map(|a| state_u16(&state, a.wrapping_add(LOCATION_FIELD)));
            out.push(LineState {
                offset: pos,
                actor_offset: actor,
                location_offset,
            });
            // advance past the text token
            match decode_text(cod, pos, end) {
                Some((_, next)) => pos = next,
                None => break,
            }
            continue;
        }
        if VAR_TERMINATED.contains(&op) {
            pos = scan_zero_word(cod, pos + 1, end);
            continue;
        }
        let len = if b1 & 0x80 != 0 {
            let mut l = b0 as usize;
            match b1 {
                0xFF => mode1 = true,
                0xFE => mode1 = false,
                0xFD | 0xFB => {
                    if cod.get(pos + 1) == Some(&0xA1) {
                        l += 1;
                    }
                }
                _ => {}
            }
            l.max(1)
        } else {
            (if mode1 { b1 } else { b0 } as usize).max(1)
        };
        pos += len;
    }
    out
}

// ---------------------------------------------------------------------------
// VM-event schema + emitter (renderer foundation)
//
// The goal is to drive cutscene rendering from an ordered event stream instead
// of the `(script,function)+actor` grouping heuristic in `character.rs`. These
// are the events the game's presentation layer effectively produces while
// walking a dialogue run; the emitter below turns the decoded per-line fields
// (now correct after the `decode_text_call_at` fix) into that stream, emitting
// state-change events (background/music/speaker) only on transitions.
//
// Not yet wired into the mp4 pipeline — that integration is where the DOSBox-X
// oracle is needed to validate timing/voice/animation. Unit-tested here so the
// schema and ordering are pinned down first.
// ---------------------------------------------------------------------------

/// One presentation event in execution order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum SceneEvent {
    SetBackground {
        hnm: Option<String>,
        record: Option<String>,
    },
    PlayMusic {
        music: Option<String>,
    },
    ShowSpeaker {
        actor: String,
    },
    PlayVoice {
        clip_index: usize,
    },
    PlayTalkHnm {
        clip_index: usize,
    },
    DrawSubtitle {
        text: String,
        voice_selector: u8,
        flags: u8,
    },
    /// Per-character UI "chatter" bleeps during the animated text reveal (tb.snd).
    PlayChatter,
    Clear,
}

/// Minimal per-line input for the emitter — the fields a decoded `0xA6` line
/// plus its resolved scene context provide. Decoupled from `ScriptSpeechLine`
/// so the emitter stays unit-testable.
#[derive(Clone, Debug, Default, Serialize)]
pub struct LineInput {
    pub actor: Option<String>,
    pub background_hnm: Option<String>,
    pub background_record: Option<String>,
    pub background_music: Option<String>,
    pub voice_selector: u8,
    pub flags_b4: u8,
    pub clip_index: Option<usize>,
    pub text: String,
}

/// Turn an ordered sequence of decoded dialogue lines into a presentation event
/// stream, emitting background/music/speaker changes only on transition and a
/// trailing `Clear`.
pub fn emit_scene_events(lines: &[LineInput]) -> Vec<SceneEvent> {
    let mut events = Vec::new();
    let mut cur_bg: Option<(Option<String>, Option<String>)> = None;
    let mut cur_music: Option<Option<String>> = None;
    let mut cur_actor: Option<String> = None;

    for line in lines {
        let bg = (line.background_hnm.clone(), line.background_record.clone());
        if cur_bg.as_ref() != Some(&bg) {
            events.push(SceneEvent::SetBackground {
                hnm: bg.0.clone(),
                record: bg.1.clone(),
            });
            cur_bg = Some(bg);
        }
        if cur_music.as_ref() != Some(&line.background_music) {
            events.push(SceneEvent::PlayMusic {
                music: line.background_music.clone(),
            });
            cur_music = Some(line.background_music.clone());
        }
        if let Some(actor) = &line.actor {
            if cur_actor.as_ref() != Some(actor) {
                events.push(SceneEvent::ShowSpeaker {
                    actor: actor.clone(),
                });
                cur_actor = Some(actor.clone());
            }
        }
        if let Some(clip) = line.clip_index {
            events.push(SceneEvent::PlayTalkHnm { clip_index: clip });
            events.push(SceneEvent::PlayVoice { clip_index: clip });
        }
        events.push(SceneEvent::DrawSubtitle {
            text: line.text.clone(),
            voice_selector: line.voice_selector,
            flags: line.flags_b4,
        });
        events.push(SceneEvent::PlayChatter);
    }
    events.push(SceneEvent::Clear);
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a tiny synthetic COD: a 1-byte op, an A6 text token (no loop), an
    /// A6 text token (with loop bit), then the 0xFF end marker.
    #[test]
    fn walks_synthetic_cod() {
        let mut cod = Vec::new();
        cod.push(0xCE); // 1-byte op (CE descriptor len 1)
        // A6 line=0x0102 b3=0x05 b4=0x00 b5=0x80  words: 0x000C, 0x0010, term
        cod.extend_from_slice(&[0xA6, 0x02, 0x01, 0x05, 0x00, 0x80]);
        cod.extend_from_slice(&[0x0C, 0x00, 0x10, 0x00, 0x00, 0x00]);
        // A6 with loop bit (b4=0x10): loop target 0x1234, word 0x0020, term
        cod.extend_from_slice(&[0xA6, 0x00, 0x00, 0xFF, 0x10, 0x80]);
        cod.extend_from_slice(&[0x34, 0x12, 0x20, 0x00, 0x00, 0x00]);
        cod.push(0xFF); // end

        let toks = walk(&cod, 0, cod.len());
        assert_eq!(toks.len(), 3);
        assert_eq!(
            toks[0],
            VmToken::Op {
                offset: 0,
                opcode: 0xCE,
                len: 1
            }
        );
        match &toks[1] {
            VmToken::Text {
                line_index,
                voice_selector,
                flags_b4,
                flags_b5,
                loop_target,
                word_offsets,
                ..
            } => {
                assert_eq!(*line_index, 0x0102);
                assert_eq!(*voice_selector, 0x05);
                assert_eq!(*flags_b4, 0x00);
                assert_eq!(*flags_b5, 0x80);
                assert_eq!(*loop_target, None);
                assert_eq!(word_offsets, &vec![0x000C, 0x0010]);
            }
            other => panic!("expected Text, got {other:?}"),
        }
        match &toks[2] {
            VmToken::Text {
                voice_selector,
                loop_target,
                word_offsets,
                ..
            } => {
                assert_eq!(*voice_selector, 0xFF); // no voice
                assert_eq!(*loop_target, Some(0x1234));
                assert_eq!(word_offsets, &vec![0x0020]);
            }
            other => panic!("expected looped Text, got {other:?}"),
        }
    }

    #[test]
    fn emits_state_changes_on_transition_only() {
        let lines = vec![
            LineInput {
                actor: Some("Bob_Morlock".into()),
                background_hnm: Some("petrol10".into()),
                background_music: Some("mus1".into()),
                clip_index: Some(0),
                voice_selector: 0x01,
                flags_b4: 0x00,
                text: "hi".into(),
                ..Default::default()
            },
            // same bg/music/actor → no repeated Set/Play* state events
            LineInput {
                actor: Some("Bob_Morlock".into()),
                background_hnm: Some("petrol10".into()),
                background_music: Some("mus1".into()),
                clip_index: Some(1),
                voice_selector: 0xFF,
                flags_b4: 0x10,
                text: "there".into(),
                ..Default::default()
            },
        ];
        let ev = emit_scene_events(&lines);
        // exactly one SetBackground / PlayMusic / ShowSpeaker across both lines
        assert_eq!(
            ev.iter()
                .filter(|e| matches!(e, SceneEvent::SetBackground { .. }))
                .count(),
            1
        );
        assert_eq!(
            ev.iter()
                .filter(|e| matches!(e, SceneEvent::PlayMusic { .. }))
                .count(),
            1
        );
        assert_eq!(
            ev.iter()
                .filter(|e| matches!(e, SceneEvent::ShowSpeaker { .. }))
                .count(),
            1
        );
        // two subtitles + two voices, trailing Clear
        assert_eq!(
            ev.iter()
                .filter(|e| matches!(e, SceneEvent::DrawSubtitle { .. }))
                .count(),
            2
        );
        assert_eq!(
            ev.iter()
                .filter(|e| matches!(e, SceneEvent::PlayVoice { .. }))
                .count(),
            2
        );
        assert_eq!(ev.last(), Some(&SceneEvent::Clear));
    }

    /// Interpreter probe: when extracted scripts are present, run the state
    /// interpreter and report how many 0xA6 lines resolve a runtime location
    /// (non-zero `state[actor+24]`). Should match `vm::walk`'s text count and a
    /// meaningful fraction should carry a location (prototype: ~63% resolve to a
    /// real DESCRIPT location; here we just count non-zero, a looser bound).
    #[test]
    fn interpreter_resolves_runtime_locations_if_present() {
        for idx in 1..=5 {
            for prefix in ["output/scripts", "../output/scripts"] {
                let cp = format!("{prefix}/SCRIPT{idx}.COD");
                let vp = format!("{prefix}/SCRIPT{idx}.VAR");
                let (Ok(cod), Ok(var)) = (std::fs::read(&cp), std::fs::read(&vp)) else {
                    continue;
                };
                let states = interpret_line_states(&cod, &var);
                let texts = walk(&cod, 0, cod.len())
                    .iter()
                    .filter(|t| matches!(t, VmToken::Text { .. }))
                    .count();
                assert_eq!(states.len(), texts, "one LineState per 0xA6 line");
                let with_loc = states
                    .iter()
                    .filter(|s| s.location_offset.is_some_and(|l| l != 0))
                    .count();
                eprintln!(
                    "SCRIPT{idx}: {} lines, {with_loc} with a runtime location",
                    states.len()
                );
            }
        }
    }

    /// If the real binary is present, confirm the embedded descriptor table
    /// matches `BLOODPRG.EXE` file offset 0x14338, so the constant can't drift.
    #[test]
    fn table_matches_binary() {
        const TABLE_OFF: usize = 0x14338;
        let candidates = ["re/bin/BLOODPRG.EXE", "../re/bin/BLOODPRG.EXE"];
        let Some(data) = candidates.iter().find_map(|p| std::fs::read(p).ok()) else {
            eprintln!("skipping: BLOODPRG.EXE not available");
            return;
        };
        for (i, &(b0, b1)) in OPCODE_DESC.iter().enumerate() {
            let off = TABLE_OFF + i * 2;
            assert_eq!(data[off], b0, "byte0 mismatch at opcode {:#04x}", 0xA0 + i);
            assert_eq!(
                data[off + 1],
                b1,
                "byte1 mismatch at opcode {:#04x}",
                0xA0 + i
            );
        }
    }

    /// A *linear* walk from offset 0 decodes every real script cleanly to the
    /// `0xFF` end marker with zero `Invalid` tokens — the COD is fully linearly
    /// walkable (no control-flow interpreter needed for a full pass). Asserts no
    /// Invalid token for any present script.
    #[test]
    fn walks_real_scripts_if_present() {
        for idx in 1..=5 {
            for prefix in ["output/scripts", "../output/scripts"] {
                let path = format!("{prefix}/SCRIPT{idx}.COD");
                let Ok(cod) = std::fs::read(&path) else {
                    continue;
                };
                let toks = walk(&cod, 0, cod.len());
                let invalid = toks
                    .iter()
                    .filter(|t| matches!(t, VmToken::Invalid { .. }))
                    .count();
                let texts = toks
                    .iter()
                    .filter(|t| matches!(t, VmToken::Text { .. }))
                    .count();
                eprintln!(
                    "{path}: {} tokens, {texts} text, {invalid} invalid",
                    toks.len()
                );
                assert_eq!(invalid, 0, "{path} should walk cleanly");
            }
        }
    }
}
