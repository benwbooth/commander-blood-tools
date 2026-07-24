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
//! then optional control words, then a `0x0000`-terminated list of
//! dictionary-word offsets). `0xA8/0xAC/0xCC/0xD3` are bare 1-byte opcodes.
//!
//! Status: token decoding is verified byte-exact against the binary (see tests).
//! The pieces here are the foundation for the VM-event renderer that will
//! replace the heuristic in `character.rs`. `walk()` preserves the linear
//! all-lines view used by comprehensive manifests; `execute_trace()` follows the
//! recovered A0/A1 branch stack for a concrete initial `SCRIPT*.VAR` state.
#![allow(dead_code)]

use std::collections::BTreeMap;

use serde::Serialize;

use crate::ship3d;

/// Per-opcode descriptor bytes for opcodes `0xA0..=0xD3`, transcribed from
/// `BLOODPRG.EXE` file offset 0x14338 (`DS:0x6F18`). `(len_mode0, byte1)` where
/// `byte1` is either `len_mode1` or a mode-control sentinel (bit7 set).
/// Verified against the binary by `tests::table_matches_binary` when
/// `re/bin/BLOODPRG.EXE` is available.
// NOTE: the engine's table at DS:0x6F18 has only 0x34 real entries (A0..D3);
// the bytes that follow are a debug string ("...memoire libre..."). But
// vm_token_advance (0x62B6) indexes the table with ANY opcode byte >= 0xA0 —
// scripts DO use opcodes beyond 0xD3 (SCRIPT2 has 0xE4 at 0x2F60) and the
// engine then reads the string bytes as (len_mode0, len_mode1). That
// out-of-bounds read is load-bearing 1994 behavior, so the port's table
// reproduces all 0x60 entries byte-exactly from the binary image.
pub const OPCODE_DESC: [(u8, u8); 0x60] = [
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
    /* D4 */ (0x6d, 0x65),
    /* D5 */ (0x6d, 0x6f),
    /* D6 */ (0x69, 0x72),
    /* D7 */ (0x65, 0x20),
    /* D8 */ (0x6c, 0x69),
    /* D9 */ (0x62, 0x72),
    /* DA */ (0x65, 0x00),
    /* DB */ (0x00, 0x00),
    /* DC */ (0x46, 0x0a),
    /* DD */ (0x09, 0x00),
    /* DE */ (0x66, 0x69),
    /* DF */ (0x6e, 0x00),
    /* E0 */ (0x00, 0x00),
    /* E1 */ (0x00, 0x00),
    /* E2 */ (0x00, 0x00),
    /* E3 */ (0x00, 0x00),
    /* E4 */ (0x00, 0x00),
    /* E5 */ (0x00, 0x00),
    /* E6 */ (0x00, 0x00),
    /* E7 */ (0x00, 0x00),
    /* E8 */ (0xff, 0xff),
    /* E9 */ (0xff, 0xff),
    /* EA */ (0xff, 0xff),
    /* EB */ (0xff, 0xff),
    /* EC */ (0xff, 0xff),
    /* ED */ (0xff, 0xff),
    /* EE */ (0xff, 0xff),
    /* EF */ (0xff, 0xff),
    /* F0 */ (0xff, 0xff),
    /* F1 */ (0xff, 0xff),
    /* F2 */ (0xff, 0xff),
    /* F3 */ (0xff, 0xff),
    /* F4 */ (0xff, 0xff),
    /* F5 */ (0xff, 0xff),
    /* F6 */ (0xff, 0xff),
    /* F7 */ (0xff, 0xff),
    /* F8 */ (0xff, 0x27),
    /* F9 */ (0xff, 0xff),
    /* FA */ (0xff, 0xff),
    /* FB */ (0xff, 0x28),
    /* FC */ (0xff, 0xff),
    /* FD */ (0xff, 0xff),
    /* FE */ (0xff, 0x29),
    /* FF */ (0x25, 0xff),
];

pub const OP_MIN: u8 = 0xA0;
pub const OP_MAX: u8 = 0xFE;
pub const OP_TEXT: u8 = 0xA6;
pub const OP_BIT_FLAG: u8 = 0xB7;
pub const OP_PAIR_RECORD_A: u8 = 0xB8;
pub const OP_PAIR_RECORD_B: u8 = 0xB9;
pub const OP_PAIR_RECORD_C: u8 = 0xBD;
pub const OP_RECORD_STATE_MIN: u8 = 0xC1;
pub const OP_RECORD_STATE_MAX: u8 = 0xC2;
pub const OP_RECORD_LINK: u8 = 0xC3;
pub const OP_ACTOR: u8 = 0xC4;
pub const OP_RECORD_ENTRY_MIN: u8 = 0xC5;
pub const OP_RECORD_ENTRY_MAX: u8 = 0xC8;
pub const OP_RECORD_CLEAR: u8 = 0xC9;
pub const OP_GLOBAL_WORD_COMPARE: u8 = 0xCA;
pub const OP_GLOBAL_PAIR_COMPARE: u8 = 0xCB;
pub const OP_RECORD_TRIPLE: u8 = 0xCD;
pub const OP_SCRIPT_PROFILE_REQUEST: u8 = 0xD2;
// Control-flow opcodes decoded from the handler table (file 0x142d0) this session; the
// handler behaviors (labels.csv) confirm the record/compare constants above.
/// `0xA0` PUSH operand → VM operand stack (`gs:0x6820`, ptr `gs:0x6884`). Handler 0x6559.
pub const OP_PUSH: u8 = 0xA0;
/// `0xA1` POP the VM operand stack. Handler 0x6572.
pub const OP_POP: u8 = 0xA1;
/// `0xA4` unconditional JUMP (PC = operand). Handler 0x65db.
pub const OP_JUMP: u8 = 0xA4;
/// `0xA5` conditional branch on the `gs:0x6ade` state-array flag. Handler 0x65eb.
pub const OP_COND_STATE_ARRAY: u8 = 0xA5;
/// `0xA8` load a null-terminated string operand into buffer `0x2120`. Handler 0x67c8.
pub const OP_LOAD_STRING: u8 = 0xA8;
/// `0xA9` conditional jump on operand bit0. Handler 0x6830.
pub const OP_COND_JUMP: u8 = 0xA9;
/// `0xAA`/`0xAC` YIELD — set `gs:0x67b4`; the exec loop breaks the frame. Handlers 0x6855/0x685c.
pub const OP_YIELD_A: u8 = 0xAA;
pub const OP_YIELD_B: u8 = 0xAC;
/// `0xAB` poke a byte to `[address operand]` (set-variable). Handler 0x684c.
pub const OP_POKE_BYTE: u8 = 0xAB;
/// `0xCE`/`0xD0` conditional branch on game flags `[0x2793]`/`[0x252a]` via `vm_branch`.
pub const OP_COND_BRANCH_PRESENTATION: u8 = 0xCE;
pub const OP_COND_BRANCH_GAMEFLAG: u8 = 0xD0;
/// `0xCC` set a byte in the 16-byte-record table `gs:0x6cde`. Handler 0x64ce.
pub const OP_SET_RECORD_BYTE: u8 = 0xCC;

/// The decoded VM query/set model (`gs:0x67ad`): record opcodes COMPARE-and-branch while
/// query mode is on (inside an `A0 … A1` block), or WRITE (set) while it is off — the
/// behaviour verified across `0xB8`/`0x6946`/the `C5..C8` family. This is the tested
/// model of that dual mode: [`enter_query`] (opcode `0xA0`) / [`exit_query`] (`0xA1`) toggle
/// it, and [`record_op`] dispatches a 2-word record access accordingly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuerySetMode {
    /// `gs:0x67ad` — true while inside an `A0 … A1` query block.
    pub query: bool,
}

/// The result of a record opcode under [`QuerySetMode`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordOpResult {
    /// Query mode: the record's two words matched the operands (fall through).
    QueryMatched,
    /// Query mode: mismatch — the VM branches (`vm_branch` 0x6462).
    QueryBranch,
    /// Set mode: the two words `(a, b)` were written into the record.
    Wrote(u16, u16),
}

impl QuerySetMode {
    /// Opcode `0xA0` PUSH — enter query mode (`gs:0x67ad = 1`).
    pub fn enter_query(&mut self) {
        self.query = true;
    }
    /// Opcode `0xA1` POP — exit query mode (`gs:0x67ad = 0`).
    pub fn exit_query(&mut self) {
        self.query = false;
    }

    /// Apply a compound state operator (the decoded `0x6863`-family operator byte, in
    /// `ah`) to `state[op1]` with `op2`. In query mode the operator is a **comparison**
    /// (`0xF0`ne/`0xF1`lt/`0xF2`gt/`0xF3`le/`0xF4`ge/`0xF5`eq) whose result decides
    /// branch-or-continue; in set mode it is an **assignment** (`0xF5`set/`0xF6`add/
    /// `0xF7`sub) that returns the new `state[op1]`. Returns `Ok(new_value)` for a set,
    /// `Err(matched)` for a query (`matched == true` → continue, false → `vm_branch`).
    pub fn apply_operator(&self, operator: u8, cur: u16, op2: u16) -> Result<u16, bool> {
        if self.query {
            let matched = match operator {
                0xF0 => cur != op2,
                0xF1 => cur < op2,
                0xF2 => cur > op2,
                0xF3 => cur <= op2,
                0xF4 => cur >= op2,
                0xF5 => cur == op2,
                _ => false,
            };
            Err(matched)
        } else {
            let new = match operator {
                0xF5 => op2,                    // SET
                0xF6 => cur.wrapping_add(op2),  // ADD
                0xF7 => cur.wrapping_sub(op2),  // SUB
                _ => cur,
            };
            Ok(new)
        }
    }

    /// A 2-word record opcode (`0xB8` family): in query mode compare the operands
    /// `(a, b)` against the record's current `(cur_a, cur_b)` — match falls through, else
    /// branch; in set mode the operands are written. `wildcard` (the `gs:0x674e` sentinel
    /// substitution used by the shared `0x6946` handler) makes an operand match anything.
    pub fn record_op(
        &self,
        operands: (u16, u16),
        current: (u16, u16),
        wildcard: Option<u16>,
    ) -> RecordOpResult {
        if !self.query {
            return RecordOpResult::Wrote(operands.0, operands.1);
        }
        let matches = |op: u16, cur: u16| wildcard == Some(op) || op == cur;
        if matches(operands.0, current.0) && matches(operands.1, current.1) {
            RecordOpResult::QueryMatched
        } else {
            RecordOpResult::QueryBranch
        }
    }
}
pub const TEXT_SELECTOR_NONE: u8 = 0xFF;
pub const TEXT_SELECTOR_SILENT: u8 = 0x00;
pub const ACTIVE_LINE_ID_BIAS: u16 = 9;
pub const CHATTER_HOLD_EXTRA_TICKS: u16 = 6;
pub const TEXT_PRESERVE_ACTIVE_FLAG: u8 = 0x01;
pub const TEXT_EXTRA_CONTROL_WORD_FLAG: u8 = 0x04;
pub const TEXT_CONDITIONAL_SKIP_FLAG: u8 = 0x08;
pub const TEXT_LOOP_TARGET_FLAG: u8 = 0x10;
pub const TEXT_ACTIVE_DISPLAY_FLAG: u8 = 0x80;
pub const TEXT_LINE_ALREADY_SHOWN_FLAG: u16 = 0x8000;

/// Port the TEXT handler's `b3` selector bridge:
/// `cbw; mov gs:[0x1FAB],ax`, then `mov ax,[0x1FAB]; add ax,9; mov [0x6788],ax`.
pub fn text_selector_active_line_id(selector: u8) -> u16 {
    (selector as i8 as i16 as u16).wrapping_add(ACTIVE_LINE_ID_BIAS)
}

/// Resolve a TEXT `b3` selector to the actor's zero-based `son.snd` talk clip.
///
/// Current evidence: `0x00` and `0xFF` are subtitle/no-voice channels, while
/// `1..=talk_clip_count` are one-based talk clip selectors. This replaces the
/// removed heuristic that treated `b4` control flags as a fallback clip index.
pub fn text_selector_voice_clip_index(selector: u8, talk_clip_count: usize) -> Option<usize> {
    let one_based = selector as usize;
    if text_selector_requests_voice(selector) && one_based <= talk_clip_count {
        Some(one_based - 1)
    } else {
        None
    }
}

pub fn text_selector_requests_voice(selector: u8) -> bool {
    selector != TEXT_SELECTOR_NONE && selector != TEXT_SELECTOR_SILENT
}

pub fn text_flags_are_active(flags_b5: u8) -> bool {
    flags_b5 & TEXT_ACTIVE_DISPLAY_FLAG != 0
}

/// Port the A6 handler's conditional-skip count at file `0x661E..0x662C`:
/// `b4 & 0x08` stores `((b5 >> 4) & 7) + 1` in `gs:0x67AB`.
pub fn text_conditional_skip_count(flags_b4: u8, flags_b5: u8) -> Option<u8> {
    (flags_b4 & TEXT_CONDITIONAL_SKIP_FLAG != 0).then_some(((flags_b5 >> 4) & 0x07) + 1)
}

/// Port the accepted-line self-modifying write in the A6 handler at file
/// `0x668D..0x669B`: `b4 & 1` preserves the token's active bit, otherwise the
/// handler clears bit7 of `b5` in the COD stream after accepting the line.
pub fn text_flags_after_accept(flags_b4: u8, flags_b5: u8) -> u8 {
    if flags_b4 & TEXT_PRESERVE_ACTIVE_FLAG != 0 {
        flags_b5
    } else {
        flags_b5 & !TEXT_ACTIVE_DISPLAY_FLAG
    }
}

pub fn text_line_flags_offset(line_index: u16) -> u16 {
    line_index.wrapping_add(2)
}

pub fn text_presentation_record_offset(line_index: u16) -> u16 {
    line_index.wrapping_add(TALK_FIELD)
}

pub fn text_line_already_shown(flag_word: u16) -> bool {
    flag_word & TEXT_LINE_ALREADY_SHOWN_FLAG != 0
}

pub fn is_record_entry_opcode(opcode: u8) -> bool {
    (OP_RECORD_ENTRY_MIN..=OP_RECORD_ENTRY_MAX).contains(&opcode)
}

pub fn is_record_state_opcode(opcode: u8) -> bool {
    (OP_RECORD_STATE_MIN..=OP_RECORD_STATE_MAX).contains(&opcode)
}

pub fn is_global_compare_opcode(opcode: u8) -> bool {
    opcode == OP_GLOBAL_WORD_COMPARE || opcode == OP_GLOBAL_PAIR_COMPARE
}

pub fn is_pair_record_opcode(opcode: u8) -> bool {
    matches!(
        opcode,
        OP_PAIR_RECORD_A | OP_PAIR_RECORD_B | OP_PAIR_RECORD_C
    )
}

pub fn record_entry_stored_related_offset(opcode: u8, operand: u16) -> u16 {
    if opcode == 0xC8 { 0 } else { operand }
}

/// Port the `0xD2` handler at `BLOODPRG.EXE` file `0x64B8`:
/// `lodsb; cbw; dec ax; mov gs:[0x6780], ax`.
pub fn script_profile_index_from_request_operand(operand: u8) -> u16 {
    ((operand as i8 as i16) - 1) as u16
}

/// `0xB7` addresses bits high-bit-first inside each byte: bit 0 is mask 0x80,
/// bit 7 is mask 0x01, then bit 8 starts the next byte at mask 0x80.
pub fn bit_flag_byte_offset(base_offset: u16, bit_index: u8) -> u16 {
    base_offset.wrapping_add((bit_index >> 3) as u16)
}

pub fn bit_flag_mask(bit_index: u8) -> u8 {
    0x80u8 >> (bit_index & 7)
}

pub fn vm_field_offset(selector: u8, kind: u16) -> Option<u16> {
    if kind == 0 {
        return None;
    }
    let bit = kind.trailing_zeros() as usize;
    let index = selector as usize * 16 + bit;
    VM_FIELD_OFFSET_TABLE.get(index).copied().map(u16::from)
}

/// Port the reveal-complete hold timer at `BLOODPRG.EXE` `0x94D4..0x94DD`:
/// `b35 = gs:[0x0ACA] << 2; gs:[0x67BB] = 1`.
pub fn reveal_complete_hold_ticks(text_speed_step: u16) -> u16 {
    text_speed_step.wrapping_shl(2)
}

/// Port the text-speed init at `BLOODPRG.EXE` `0x1B29..0x1B3D`: the config text-speed
/// setting index is doubled (`add ax,ax`), setting 4 is special-cased (`cmp ax,8;
/// add ax,4`), then `gs:[0x0ACA] = (ax >> 1) + 1`. Settings 0..4 map to steps
/// {1,2,3,4,7}; the step drives the reveal rate (`gs:[0xB31] = step >> 2` frames per
/// character, @0x94BA region) and the hold timers around this one.
pub fn text_speed_step_from_setting(setting: u16) -> u16 {
    let mut doubled = setting.wrapping_add(setting);
    if doubled == 8 {
        doubled = doubled.wrapping_add(4);
    }
    (doubled >> 1).wrapping_add(1)
}

/// Frames per revealed character for a text-speed step: the reveal loop resets the
/// per-character countdown `gs:[0xB31] = step >> 2` (see `REVERSE.md` @0x94BA); a
/// zero countdown reveals a character every frame, so the effective cost is at least
/// one frame per character.
pub fn reveal_frames_per_char(text_speed_step: u16) -> u16 {
    (text_speed_step >> 2).max(1)
}

/// Port the record-end hold timer at `BLOODPRG.EXE` `0x7378..0x738C`:
/// `b35 = gs:[0x27CF] * (gs:[0x0ACA] >> 1) + 6; gs:[0x67BB] = 1`.
pub fn record_end_hold_ticks(record_units: u16, text_speed_step: u16) -> u16 {
    record_units
        .wrapping_mul(text_speed_step >> 1)
        .wrapping_add(CHATTER_HOLD_EXTRA_TICKS)
}

/// Opcodes whose descriptor length is 0 (other than `0xA6`): the VM advances
/// past them with helper `0x6293`, which scans byte-by-byte for a `0x0000` word
/// terminator and skips it (plus one more byte if a third zero follows). So
/// these are variable-length: `opcode <bytes...> 00 00`.

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
        /// Extra control word present when `b4 & 0x04`; not a DIC word offset.
        control_word: Option<u16>,
        /// `0x0000`-terminated list of `SCRIPT*.DIC` word offsets.
        word_offsets: Vec<u16>,
    },
    /// `0xC4` actor/object record operation.
    ///
    /// The DOS handler consumes two u16 operands. The first one is the record
    /// offset the extractor uses as `object_offset + 0x3A` to track the current
    /// speaker; the second is the related record offset stored by the handler.
    Actor {
        offset: usize,
        record_offset: u16,
        related_record_offset: u16,
        inverted: bool,
        len: usize,
    },
    /// `0xC3` record link.
    ///
    /// The DOS handler consumes two u16 operands and writes a 6-byte record
    /// entry `{0x00C3, related_record_offset, 1}` on the mode-0 success path.
    /// This is a line-record relation, not a speaker marker.
    RecordLink {
        offset: usize,
        record_offset: u16,
        related_record_offset: u16,
        inverted: bool,
        len: usize,
    },
    /// `0xC5..=0xC8` record entry.
    ///
    /// These handlers consume two u16 operands and write a 6-byte line-record
    /// entry on their mode-0 success path. For `0xC5..=0xC7`, the second token
    /// word is the stored related record; for `0xC8`, the handler stores zero
    /// there after confirming the destination record is empty.
    RecordEntry {
        offset: usize,
        entry_opcode: u8,
        record_offset: u16,
        operand: u16,
        stored_related_offset: u16,
        aux_word: u16,
        inverted: bool,
        len: usize,
    },
    /// `0xC9` record clear.
    ///
    /// The DOS handler zeros the 6-byte record at this offset. If the cleared
    /// record currently holds a `0xC4` actor entry, it also clears that related
    /// actor subrecord and resets presentation gate bytes.
    RecordClear {
        offset: usize,
        record_offset: u16,
        len: usize,
    },
    /// `0xB7` bit flag set/clear/test over the line-record/state area.
    ///
    /// Optional `0xA1` after the opcode inverts mode-1 tests and turns mode-0
    /// writes into clears. Bits are numbered high-bit-first inside a byte.
    BitFlag {
        offset: usize,
        flag_offset: u16,
        bit_index: u8,
        byte_offset: u16,
        mask: u8,
        clear: bool,
        len: usize,
    },
    /// `0xC1..=0xC2` line-record state operations.
    ///
    /// Both consume the same raw token shape, `<opcode> <record:u16>
    /// <operand:u16>`. Their handlers resolve additional table state before
    /// mutating or branching, so the Rust token deliberately preserves the raw
    /// operands instead of reducing them to a guessed presentation action.
    RecordState {
        offset: usize,
        opcode: u8,
        record_offset: u16,
        operand: u16,
        inverted: bool,
        len: usize,
    },
    /// `0xCA` compares a u16 token value against global `gs:0x0AA6`.
    GlobalWordCompare {
        offset: usize,
        operator: u8,
        tag: u8,
        value: u16,
        len: usize,
    },
    /// `0xCB` compares a packed two-byte token value against globals
    /// `gs:0x0AAA:0x0AA8`, preserving the final consumed word as `reserved`.
    GlobalPairCompare {
        offset: usize,
        operator: u8,
        packed_value: u16,
        reserved: u16,
        len: usize,
    },
    /// `0xB8`/`0xB9`/`0xBD` pair-record assignment/compare.
    PairRecord {
        offset: usize,
        opcode: u8,
        record_offset: u16,
        first_word: u16,
        second_word: u16,
        len: usize,
    },
    /// `0xCD` record-triple operation. Optional `0xA1` after the opcode inverts
    /// the mode-1 comparison path; mode-0 side effects require the resolved
    /// line-record table model and are not executed yet.
    RecordTriple {
        offset: usize,
        record_offset: u16,
        first_word: u16,
        second_word: u16,
        inverted: bool,
        len: usize,
    },
    /// `0xD2 <operand>` requests a script/resource profile switch after the
    /// current VM pass. The handler stores `sign_extend(operand) - 1` in
    /// `gs:0x6780`; the main loop later calls the profile selector at
    /// `0x53A0` when presentation state is idle.
    ScriptProfileRequest {
        offset: usize,
        operand: u8,
        profile_index: u16,
        len: usize,
    },
    /// Any other opcode: raw length from the descriptor table, with the operand
    /// bytes captured LOSSLESSLY (the token IR round-trips byte-exact; the ASM
    /// semantics of these ops live in VmMachine's handlers).
    Op {
        offset: usize,
        opcode: u8,
        len: usize,
        operands: Vec<u8>,
    },
    /// Decoder fell off the rails (byte outside `0xA0..=0xD3` where a token was
    /// expected). Walking stops; the offset is where it happened.
    Invalid { offset: usize, byte: u8 },
}

/// RE-ENCODE a decoded token back to its byte form — the inverse of [`walk`]'s
/// decoding, from the STRUCTURED FIELDS ONLY (no source peeking). Returns `None`
/// for content-opaque tokens (`Op`, `Invalid`), whose bytes the model knows only
/// by length. The round-trip test compares the encoding against the original
/// slice for every token of every script — the byte-exactness proof that the
/// token model matches the bitcode.
pub fn encode_token(t: &VmToken) -> Option<Vec<u8>> {
    let mut b = Vec::new();
    let w = |b: &mut Vec<u8>, v: u16| b.extend_from_slice(&v.to_le_bytes());
    match t {
        VmToken::Text {
            line_index,
            voice_selector,
            flags_b4,
            flags_b5,
            loop_target,
            control_word,
            word_offsets,
            ..
        } => {
            b.push(OP_TEXT);
            w(&mut b, *line_index);
            b.push(*voice_selector);
            b.push(*flags_b4);
            b.push(*flags_b5);
            if let Some(lt) = loop_target {
                w(&mut b, *lt);
            }
            if let Some(cw) = control_word {
                w(&mut b, *cw);
            }
            for wo in word_offsets {
                w(&mut b, *wo);
            }
            w(&mut b, 0);
        }
        VmToken::Actor { record_offset, related_record_offset, inverted, .. } => {
            b.push(0xC4);
            if *inverted {
                b.push(0xA1);
            }
            w(&mut b, *record_offset);
            w(&mut b, *related_record_offset);
        }
        VmToken::RecordLink { record_offset, related_record_offset, inverted, .. } => {
            b.push(0xC3);
            if *inverted {
                b.push(0xA1);
            }
            w(&mut b, *record_offset);
            w(&mut b, *related_record_offset);
        }
        VmToken::RecordEntry { entry_opcode, record_offset, operand, inverted, .. } => {
            b.push(*entry_opcode);
            if *inverted {
                b.push(0xA1);
            }
            w(&mut b, *record_offset);
            w(&mut b, *operand);
        }
        VmToken::RecordClear { record_offset, .. } => {
            b.push(0xC9);
            w(&mut b, *record_offset);
        }
        VmToken::RecordState { opcode, record_offset, operand, inverted, .. } => {
            b.push(*opcode);
            if *inverted {
                b.push(0xA1);
            }
            w(&mut b, *record_offset);
            w(&mut b, *operand);
        }
        VmToken::BitFlag { flag_offset, bit_index, clear, .. } => {
            b.push(OP_BIT_FLAG);
            if *clear {
                b.push(0xA1);
            }
            w(&mut b, *flag_offset);
            b.push(*bit_index);
        }
        VmToken::GlobalWordCompare { operator, tag, value, .. } => {
            b.push(OP_GLOBAL_WORD_COMPARE);
            b.push(*operator);
            b.push(*tag);
            w(&mut b, *value);
        }
        VmToken::GlobalPairCompare { operator, packed_value, reserved, .. } => {
            b.push(OP_GLOBAL_PAIR_COMPARE);
            b.push(*operator);
            w(&mut b, *packed_value);
            w(&mut b, *reserved);
        }
        VmToken::PairRecord { opcode, record_offset, first_word, second_word, .. } => {
            b.push(*opcode);
            w(&mut b, *record_offset);
            w(&mut b, *first_word);
            w(&mut b, *second_word);
        }
        VmToken::RecordTriple { record_offset, first_word, second_word, inverted, .. } => {
            b.push(OP_RECORD_TRIPLE);
            if *inverted {
                b.push(0xA1);
            }
            w(&mut b, *record_offset);
            w(&mut b, *first_word);
            w(&mut b, *second_word);
        }
        VmToken::ScriptProfileRequest { operand, .. } => {
            b.push(OP_SCRIPT_PROFILE_REQUEST);
            b.push(*operand);
        }
        VmToken::Op { opcode, operands, .. } => {
            b.push(*opcode);
            b.extend_from_slice(operands);
        }
        VmToken::Invalid { .. } => return None,
    }
    Some(b)
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

        // Determine token length + any mode change — vm_token_advance 0x62B6
        // exactly: sentinels (b1 bit7) keep len=b0 (FF/FE switch the mode,
        // FD/FB take an optional 0xA1 skip); otherwise len = table[mode]. A
        // resolved length of ZERO means zero-word-terminated (vm_token_special
        // 0x6293) — this is PER MODE (0xDA/0xDD/0xDF are fixed-length in mode 0
        // but var-terminated in mode 1), which the old hardcoded VAR_TERMINATED
        // set missed: it desynced the walk at SCRIPT2 0x2F7F and hid the COD's
        // entire tail (69% of the stream) from the decompile.
        let len;
        if b1 & 0x80 != 0 {
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
            let l = if mode1 { b1 } else { b0 } as usize;
            if l == 0 {
                let next = scan_zero_word(cod, pos + 1, end);
                out.push(VmToken::Op {
                    offset: pos,
                    opcode: op,
                    len: next - pos,
                    operands: cod[pos + 1..next].to_vec(),
                });
                pos = next;
                continue;
            }
            len = l;
        }

        if op == OP_BIT_FLAG {
            let clear = cod.get(pos + 1) == Some(&0xA1);
            let operand_pos = pos + 1 + usize::from(clear);
            let flag_offset = read_u16(cod, operand_pos).unwrap_or(0);
            let bit_index = cod.get(operand_pos + 2).copied().unwrap_or(0);
            out.push(VmToken::BitFlag {
                offset: pos,
                flag_offset,
                bit_index,
                byte_offset: bit_flag_byte_offset(flag_offset, bit_index),
                mask: bit_flag_mask(bit_index),
                clear,
                len,
            });
        } else if is_record_state_opcode(op) {
            let inverted = mode1 && cod.get(pos + 1) == Some(&0xA1);
            let operand_pos = pos + 1 + usize::from(inverted);
            let record_offset = read_u16(cod, operand_pos).unwrap_or(0);
            let operand = read_u16(cod, operand_pos + 2).unwrap_or(0);
            out.push(VmToken::RecordState {
                offset: pos,
                opcode: op,
                record_offset,
                operand,
                inverted,
                len,
            });
        } else if op == OP_GLOBAL_WORD_COMPARE {
            out.push(VmToken::GlobalWordCompare {
                offset: pos,
                operator: cod.get(pos + 1).copied().unwrap_or(0),
                tag: cod.get(pos + 2).copied().unwrap_or(0),
                value: read_u16(cod, pos + 3).unwrap_or(0),
                len,
            });
        } else if op == OP_GLOBAL_PAIR_COMPARE {
            out.push(VmToken::GlobalPairCompare {
                offset: pos,
                operator: cod.get(pos + 1).copied().unwrap_or(0),
                packed_value: read_u16(cod, pos + 2).unwrap_or(0),
                reserved: read_u16(cod, pos + 4).unwrap_or(0),
                len,
            });
        } else if is_pair_record_opcode(op) {
            out.push(VmToken::PairRecord {
                offset: pos,
                opcode: op,
                record_offset: read_u16(cod, pos + 1).unwrap_or(0),
                first_word: read_u16(cod, pos + 3).unwrap_or(0),
                second_word: read_u16(cod, pos + 5).unwrap_or(0),
                len,
            });
        } else if op == OP_RECORD_TRIPLE {
            let inverted = cod.get(pos + 1) == Some(&0xA1);
            let operand_pos = pos + 1 + usize::from(inverted);
            out.push(VmToken::RecordTriple {
                offset: pos,
                record_offset: read_u16(cod, operand_pos).unwrap_or(0),
                first_word: read_u16(cod, operand_pos + 2).unwrap_or(0),
                second_word: read_u16(cod, operand_pos + 4).unwrap_or(0),
                inverted,
                len,
            });
        } else if op == OP_RECORD_LINK {
            let inverted = mode1 && cod.get(pos + 1) == Some(&0xA1);
            let operand_pos = pos + 1 + usize::from(inverted);
            let record_offset = read_u16(cod, operand_pos).unwrap_or(0);
            let related_record_offset = read_u16(cod, operand_pos + 2).unwrap_or(0);
            out.push(VmToken::RecordLink {
                offset: pos,
                record_offset,
                related_record_offset,
                inverted,
                len,
            });
        } else if is_record_entry_opcode(op) {
            let inverted = mode1 && cod.get(pos + 1) == Some(&0xA1);
            let operand_pos = pos + 1 + usize::from(inverted);
            let record_offset = read_u16(cod, operand_pos).unwrap_or(0);
            let operand = read_u16(cod, operand_pos + 2).unwrap_or(0);
            out.push(VmToken::RecordEntry {
                offset: pos,
                entry_opcode: op,
                record_offset,
                operand,
                stored_related_offset: record_entry_stored_related_offset(op, operand),
                aux_word: 0,
                inverted,
                len,
            });
        } else if op == OP_ACTOR {
            let inverted = mode1 && cod.get(pos + 1) == Some(&0xA1);
            let operand_pos = pos + 1 + usize::from(inverted);
            let record_offset = read_u16(cod, operand_pos).unwrap_or(0);
            let related_record_offset = read_u16(cod, operand_pos + 2).unwrap_or(0);
            out.push(VmToken::Actor {
                offset: pos,
                record_offset,
                related_record_offset,
                inverted,
                len,
            });
        } else if op == OP_RECORD_CLEAR {
            let record_offset = read_u16(cod, pos + 1).unwrap_or(0);
            out.push(VmToken::RecordClear {
                offset: pos,
                record_offset,
                len,
            });
        } else if op == OP_SCRIPT_PROFILE_REQUEST {
            let operand = cod.get(pos + 1).copied().unwrap_or(0);
            out.push(VmToken::ScriptProfileRequest {
                offset: pos,
                operand,
                profile_index: script_profile_index_from_request_operand(operand),
                len,
            });
        } else {
            out.push(VmToken::Op {
                offset: pos,
                opcode: op,
                len,
                operands: cod[pos + 1..(pos + len).min(end)].to_vec(),
            });
        }
        pos += len;
    }
    out
}

/// Decode an `0xA6` TEXT token starting at `pos`. Returns the token and the
/// offset just past it, or `None` if malformed.
fn decode_text(cod: &[u8], pos: usize, end: usize) -> Option<(VmToken, usize)> {
    // A6 b1 b2 b3 b4 b5  [loop_target?] [control_word?]  w0 w1 ... 0x0000
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
    let control_word = if b4 & 0x04 != 0 {
        let word = read_u16(cod, p)?;
        p += 2;
        Some(word)
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
            control_word,
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
//       (`state[op2]`). Writes `state[op1]` in mode 0 only.
//   * 0x6902 family (AE/B0), 5 bytes plus optional A1 prefix:
//       set/clear a bit mask in `state[op1]` in mode 0.
//   * 0x6946 family (AD/AF/B2/B3/BA/BB/BC), 5 bytes:
//       direct `state[op1] = op2` in mode 0, including the 16-entry sentinel
//       list used when op2 is the `blood` object or `0xffff`.
//   * 0xB7, 4 bytes plus optional A1 prefix:
//       set/clear/test one high-bit-first byte flag in the state area.
//   * 0xB8/0xB9/0xBD, 7 bytes:
//       store/compare a two-word pair at a direct record offset.
//   * 0xC1, 5 bytes plus optional A1 prefix:
//       writes {0x00C1, operand, 2} to an active owner's empty direct record in
//       mode 0; mode-1 direct compares and the raw-operand 1/2 resolved
//       selector-0x11/selector-0x13 compares are evaluated when host state has
//       the concrete record entries. Known mode-0 write failures call the
//       branch-fail helper in branch-aware traces. The kind-0x10 ship-3D
//       source-list path is available when `ExecutionContext` supplies the live
//       DS:0x6886 scratch bytes and navigation/object tables.
//   * 0xC2, 5 bytes plus optional A1 prefix:
//       in mode 0, active owners can mark the operand record's kind-specific
//       field as 0xffff via helper table 0x6D60 and kind-2 records set active
//       dialogue line 0x27. Kind-0x0400 records can set active line 0x2B when
//       helper 0x7409 finds a matching `descript.des` entry. Mode-1 direct
//       compares are evaluated with context.
//   * 0xCD, 7 bytes plus optional A1 prefix:
//       compare a direct three-word record in mode 1; mode-0 resolved-table
//       side effects are still pending the line-record table model.
//   * 0xC4: actor/record reference. The first operand is the destination record
//       offset and doubles as object_offset + 0x3A (talk field) for speaker
//       tracking; the second operand is the related record offset stored by the
//       DOS handler. Mode 0 writes the direct record entry and updates speaker
//       tracking; mode 1 compares the record entry and may branch.
//   * 0xC3: record link. The handler writes {0x00C3, related, 1}; this is
//       presentation record state, not a speaker change. Known guarded mode-0
//       failures branch when owner context is available.
//   * 0xC5..=0xC8: record entries. Successful mode-0 writes are guarded per
//       handler (C6 is unconditional; C8 stores zero despite consuming an
//       operand), and mode-1 direct compares are evaluated when host state has a
//       concrete record entry. Known guarded mode-0 failures branch.
//   * 0xC9: record clear. The handler zeroes a 6-byte record in both modes and,
//       when the previous entry was 0xC4, clears the related actor subrecord too.
//   * 0xCA/0xCB: global conditions. They compare token operands against
//       runtime globals `gs:0x0AA6` and `gs:0x0AAA:0x0AA8`; branch evaluation
//       is available when `ExecutionContext` supplies those globals. The DOS VM
//       refreshes them from BIOS RTC calls immediately before entering the
//       interpreter: hour -> 0x0AA6, day -> 0x0AA8, month -> 0x0AAA.
//   * 0xD2: request a script/resource profile switch by storing
//       sign_extend(operand)-1 in `gs:0x6780`. The main loop handles the actual
//       cross-profile handoff after the current VM pass, so traces decode the
//       token but do not recursively execute the next script yet.
// The post-VM object scan at 0x5816 is only partially represented: the recovered
// C4 pair update marks a direct C4 record consumed and writes the reciprocal
// selector-0x13 C4 record on the related object. The kind-1 presentation
// start/stop flag updates and kind-2 control-flow handoff are represented, but
// the direct render/audio calls remain pending.
// NOTE: `interpret_line_states` is a LINEAR pass: it applies mode-0 state
// mutations and uses guarded mode-1 actor records as context, but does not take
// branches. `execute_trace` models the recovered branch helper for conditionals
// whose runtime state inputs are available; see REVERSE.md for unresolved table
// inputs that still require deeper runtime modeling.

const ASSIGN_7: [u8; 7] = [0xB1, 0xB4, 0xB5, 0xB6, 0xBE, 0xBF, 0xC0];
const BITMASK_5: [u8; 2] = [0xAE, 0xB0];
const ASSIGN_5: [u8; 7] = [0xAD, 0xAF, 0xB2, 0xB3, 0xBA, 0xBB, 0xBC];
const TALK_FIELD: u16 = 0x3A;
const LOCATION_FIELD: u16 = 24;
const SPECIAL_OBJECT_SLOT_COUNT: usize = 16;
const VM_FIELD_OFFSET_SELECTOR_PRESENTATION_HANDOFF: u8 = 0x02;
const VM_FIELD_OFFSET_SELECTOR_C2: u8 = 0x11;
const VM_FIELD_OFFSET_SELECTOR_C9_RELATED: u8 = 0x13;
const C2_ACTIVE_LINE_KIND2: u16 = 0x27;
const C2_ACTIVE_LINE_KIND400: u16 = 0x2B;
const VM_UI_FLAGS: u16 = 0x2793;
const C2_PRESENTATION_GATE: u16 = 0x1FB2;
const C2_PRESENTATION_FLAGS: u16 = 0x67AA;
const C2_PRESENTATION_BUSY_FLAG: u8 = 0x02;
const VM_ACTIVE_LINE: u16 = 0x6788;
const C9_PRESENTATION_GATE_A: u16 = 0x252A;
const C9_PRESENTATION_GATE_B: u16 = 0x2531;
const C4_POST_UPDATE_SENTINEL: u16 = 0xFFFF;
const VM_PENDING_RESOURCE_PROFILE: u16 = 0x6780;
const VM_PRESENTATION_PRIMARY_C4_RECORD: u16 = 0x675E;
const VM_PRESENTATION_ACTIVE: u16 = 0x67AC;
const VM_PRESENTATION_RELATED_FLAG20: u16 = 0x67AF;
const VM_PRESENTATION_DEFER_A: u16 = 0x67B0;
const VM_PRESENTATION_LOOP_FLAG: u16 = 0x67B1;
const VM_PRESENTATION_PAIR_WRITE_DISABLED: u16 = 0x67B6;
const VM_PRESENTATION_START_LOCK: u16 = 0x67B7;
const VM_PRESENTATION_TEXT_WAIT: u16 = 0x67BA;
const VM_PRESENTATION_HOLD_COMPLETE: u16 = 0x67BB;
const VM_PRESENTATION_HOLD_READY: u16 = 0x67BC;
const VM_PRESENTATION_WORD_BUFFER: u16 = 0x67F8;
const VM_PRESENTATION_STATUS_WORD: u16 = 0x0A32;
const VM_PRESENTATION_ACTIVE_RECORD: u16 = 0x6762;
const VM_PRESENTATION_DEFERRED_RECORD_TYPE: u16 = 0x6768;
const VM_PRESENTATION_DEFERRED_RECORD_RELATED: u16 = 0x676A;
const VM_PRESENTATION_DEFERRED_RECORD_AUX: u16 = 0x676C;
const VM_PRESENTATION_SIGNAL_SLOT: u16 = 26522;
const VM_PRESENTATION_SCENE_DIRTY: u16 = 0x5B55;
const VM_PRESENTATION_INPUT_GATE_A: u16 = 0x24F3;
const VM_PRESENTATION_INPUT_GATE_B: u16 = 0x2751;
const VM_PRESENTATION_INPUT_GATE_C: u16 = 0x5E64;
const VM_PRESENTATION_INPUT_GATE_D: u16 = 0x2565;
const VM_PRESENTATION_INPUT_GATE_E: u16 = 0x2736;
const VM_PRESENTATION_INPUT_GATE_F: u16 = 0x2737;
const VM_PRESENTATION_HANDOFF_GATE: u16 = 0x27D7;
const VM_PRESENTATION_INPUT_GATE_G: u16 = 0x27DA;
const VM_PRESENTATION_INPUT_GATE_H: u16 = 0x2792;
const VM_PRESENTATION_INPUT_GATE_I: u16 = 0x2A19;
const VM_PRESENTATION_DESCRIPTOR_PENDING: u16 = 0x27E8;
const VM_BRANCH_A: u16 = 0x6782;
const VM_BRANCH_B: u16 = 0x6784;
const VM_PC_SAVED: u16 = 0x6776;

const MAIN_PENDING_PROFILE_IDLE_GATES: [u16; 10] = [
    VM_PRESENTATION_ACTIVE,
    VM_PRESENTATION_INPUT_GATE_A,
    VM_PRESENTATION_INPUT_GATE_B,
    VM_PRESENTATION_DEFER_A,
    VM_PRESENTATION_INPUT_GATE_C,
    VM_PRESENTATION_INPUT_GATE_D,
    VM_PRESENTATION_INPUT_GATE_E,
    VM_PRESENTATION_INPUT_GATE_F,
    VM_PRESENTATION_INPUT_GATE_G,
    VM_PRESENTATION_INPUT_GATE_H,
];

/// Field-offset lookup table used by helper `0x6023`:
/// `gs:[0x6D60 + selector * 16 + bsf(kind)]`.
/// Transcribed from `BLOODPRG.EXE` file `0x14180..0x142CF`.
const VM_FIELD_OFFSET_TABLE: [u8; 0x150] = [
    0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x04, 0x16, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x1a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x32, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x34, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x1e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x38, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x36, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x20, 0x44, 0x1c, 0x1c, 0x22, 0x00, 0x00, 0x16, 0x00, 0x10, 0x16, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x46, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x14, 0x14, 0x14, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x06, 0x18, 0x16, 0x16, 0x16, 0x00, 0x00, 0x14, 0x00, 0x04, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x1a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x08, 0x3a, 0x00, 0x00, 0x1c, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BranchEvent {
    pub offset: usize,
    pub opcode: u8,
    pub target: Option<u16>,
    pub branch_taken: bool,
    pub condition_passed: Option<bool>,
    pub stack_depth: usize,
    pub detail: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ScriptProfileRequestEvent {
    pub offset: usize,
    pub operand: u8,
    pub profile_index: u16,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct PostUpdateTrace {
    pub actor_record_pairs: Vec<PostUpdateActorRecordPair>,
    pub presentation_handoffs: Vec<PresentationHandoffEvent>,
    pub pending_script_profile_dispatch_ready: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct PostUpdateActorRecordPair {
    pub record_offset: u16,
    pub related_record_offset: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct PresentationHandoffEvent {
    pub owner_offset: u16,
    pub record_offset: u16,
    pub target: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum ExecutionHalt {
    EndMarker,
    InvalidOpcode { offset: usize, byte: u8 },
    InvalidTarget { offset: usize, target: u16 },
    StepLimit { limit: usize },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExecutionTrace {
    pub line_states: Vec<LineState>,
    pub branch_events: Vec<BranchEvent>,
    pub script_profile_requests: Vec<ScriptProfileRequestEvent>,
    pub post_update: PostUpdateTrace,
    pub steps: usize,
    pub halted: ExecutionHalt,
}

impl ExecutionTrace {
    pub fn pending_script_profile(&self) -> Option<u16> {
        self.script_profile_requests
            .last()
            .map(|event| event.profile_index)
            .filter(|profile_index| *profile_index != 0xffff)
    }
}

pub struct ScriptProfileProgram<'a> {
    pub profile_index: u16,
    pub cod: &'a [u8],
    pub var: &'a [u8],
    pub context: ExecutionContext,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ScriptProfileRun {
    pub run_index: usize,
    pub profile_index: u16,
    pub trace: ExecutionTrace,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum ScriptProfileExecutionHalt {
    NoPendingProfile,
    PendingProfileNotReady {
        profile_index: u16,
    },
    MissingProfile {
        profile_index: u16,
    },
    RunLimit {
        limit: usize,
        next_profile_index: u16,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ScriptProfileExecution {
    pub runs: Vec<ScriptProfileRun>,
    pub halted: ScriptProfileExecutionHalt,
}

struct ExecutedTrace {
    trace: ExecutionTrace,
    final_state: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Ship3dC1RuntimeContext {
    navigation_records: Vec<ship3d::Ship3dNavigationRuntimeRecord>,
    object_table_records: Vec<u16>,
    source_list_bytes: Vec<u8>,
    position_runtime: Option<Ship3dC1PositionRuntime>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Ship3dC1PositionRuntime {
    records: Vec<ship3d::Ship3dPositionRecord>,
    fields: Vec<ship3d::Ship3dPositionField>,
    arche_object: u16,
    inherited_kind100_compare_word: u16,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VmNamedObjectOffsets {
    pub blood: Option<u16>,
    pub orxx: Option<u16>,
    pub honk: Option<u16>,
    pub menu: Option<u16>,
    pub arche: Option<u16>,
    pub ark: Option<u16>,
    pub scruter_jo: Option<u16>,
    pub vbio: Option<u16>,
}

impl VmNamedObjectOffsets {
    fn set(&mut self, name: &str, offset: u16) -> bool {
        if name.eq_ignore_ascii_case("blood") {
            self.blood = Some(offset);
        } else if name.eq_ignore_ascii_case("orxx") {
            self.orxx = Some(offset);
        } else if name.eq_ignore_ascii_case("Honk") {
            self.honk = Some(offset);
        } else if name.eq_ignore_ascii_case("menu") {
            self.menu = Some(offset);
        } else if name.eq_ignore_ascii_case("arche") {
            self.arche = Some(offset);
        } else if name.eq_ignore_ascii_case("Ark") {
            self.ark = Some(offset);
        } else if name.eq_ignore_ascii_case("Scruter_Jo") {
            self.scruter_jo = Some(offset);
        } else if name.eq_ignore_ascii_case("vbio") {
            self.vbio = Some(offset);
        } else {
            return false;
        }
        true
    }
}

/// Runtime tables the DOS VM receives through globals outside `SCRIPT*.VAR`.
///
/// `object_offsets` mirrors the 20-byte object table scanned by helper `0x6034`:
/// it maps a record/field offset to the owning object by taking the previous
/// object offset from the sorted kind-1 object records.
///
/// `special_object_offset` is DOS `gs:0x674e`, initialized from the DEB object
/// named `blood`. Handler `0x6946` maps that RHS value to `0xffff` before
/// mode-1 equality/inversion tests.
///
/// `descript_entry_names` mirrors the `descript.des` directory scanned by
/// helper `0x7409`. The C2 kind-0x0400 path passes `operand + 4` as a
/// NUL-terminated name and treats a matching directory entry as helper success.
///
/// `text_presentation_record_gating` models the A6 handler's `object+0x3A`
/// `0x00C4` check. It stays opt-in until the C4 presentation setup path is
/// complete enough for real-script exports to satisfy that gate.
///
/// `strict_actor_record_branching` models the mode-1 C4 handler's branch-fail
/// path for empty records. It stays opt-in because the mode-0 presentation setup
/// path that should populate those records is still incomplete.
///
/// `named_object_offsets` mirrors the startup scan at `0x5486`, which compares
/// DEB object names against built-in strings and stores matching offsets in VM
/// globals (`blood` -> `0x674E`, `orxx` -> `0x6750`, `arche` -> `0x6752`, ...).
///
/// `ship3d_c1_runtime` is the recovered scratch/runtime state for the C1
/// kind-`0x10` branch. It is explicit because helper `0x006210` reads from the
/// live `DS:0x6886` bytes using the current `SI` cursor, not just from parsed
/// object records. The optional position runtime models the earlier
/// distance/selector-`0x11` redirect at file `0x006BEA..0x006C04`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExecutionContext {
    object_offsets: Vec<u16>,
    special_object_offset: Option<u16>,
    named_object_offsets: VmNamedObjectOffsets,
    global_word_0aa6: Option<u16>,
    global_pair_0aaa_0aa8: Option<(u8, u8)>,
    descript_entry_names: Vec<Vec<u8>>,
    text_presentation_record_gating: bool,
    text_line_display_gating: bool,
    strict_actor_record_branching: bool,
    ship3d_c1_runtime: Option<Ship3dC1RuntimeContext>,
}

impl ExecutionContext {
    pub fn from_object_offsets<I>(offsets: I) -> Self
    where
        I: IntoIterator<Item = u16>,
    {
        let mut object_offsets: Vec<u16> = offsets.into_iter().collect();
        object_offsets.sort_unstable();
        object_offsets.dedup();
        Self {
            object_offsets,
            ..Self::default()
        }
    }

    pub fn with_global_word_0aa6(mut self, value: u16) -> Self {
        self.global_word_0aa6 = Some(value);
        self
    }

    pub fn with_global_pair_0aaa_0aa8(mut self, high: u8, low: u8) -> Self {
        self.global_pair_0aaa_0aa8 = Some((high, low));
        self
    }

    pub fn with_special_object_offset(mut self, value: u16) -> Self {
        self.special_object_offset = Some(value);
        self.named_object_offsets.blood = Some(value);
        self
    }

    pub fn with_vm_named_object(mut self, name: impl AsRef<str>, offset: u16) -> Self {
        let name = name.as_ref();
        if self.named_object_offsets.set(name, offset) && name.eq_ignore_ascii_case("blood") {
            self.special_object_offset = Some(offset);
        }
        self
    }

    pub fn with_descript_entry_name(mut self, name: impl AsRef<str>) -> Self {
        let bytes = name.as_ref().as_bytes();
        if !bytes.is_empty()
            && !bytes.contains(&0)
            && !self
                .descript_entry_names
                .iter()
                .any(|known| known.as_slice() == bytes)
        {
            self.descript_entry_names.push(bytes.to_vec());
        }
        self
    }

    pub fn with_bios_rtc(mut self, hour_24: u8, month: u8, day: u8) -> Self {
        self.global_word_0aa6 = Some(hour_24 as u16);
        self.global_pair_0aaa_0aa8 = Some((month, day));
        self
    }

    pub fn with_text_line_display_gating(mut self) -> Self {
        self.text_line_display_gating = true;
        self
    }

    pub fn with_text_presentation_record_gating(mut self) -> Self {
        self.text_presentation_record_gating = true;
        self
    }

    pub fn with_strict_actor_record_branching(mut self) -> Self {
        self.strict_actor_record_branching = true;
        self
    }

    pub fn with_ship_3d_c1_runtime<I, J>(
        mut self,
        navigation_records: I,
        object_table_records: J,
        source_list_bytes: impl Into<Vec<u8>>,
    ) -> Self
    where
        I: IntoIterator<Item = ship3d::Ship3dNavigationRuntimeRecord>,
        J: IntoIterator<Item = u16>,
    {
        self.ship3d_c1_runtime = Some(Ship3dC1RuntimeContext {
            navigation_records: navigation_records.into_iter().collect(),
            object_table_records: object_table_records.into_iter().collect(),
            source_list_bytes: source_list_bytes.into(),
            position_runtime: None,
        });
        self
    }

    pub fn with_ship_3d_c1_positions<I, J>(
        mut self,
        records: I,
        fields: J,
        arche_object: u16,
        inherited_kind100_compare_word: u16,
    ) -> Self
    where
        I: IntoIterator<Item = ship3d::Ship3dPositionRecord>,
        J: IntoIterator<Item = ship3d::Ship3dPositionField>,
    {
        let runtime = self.ship3d_c1_runtime.get_or_insert_with(Default::default);
        runtime.position_runtime = Some(Ship3dC1PositionRuntime {
            records: records.into_iter().collect(),
            fields: fields.into_iter().collect(),
            arche_object,
            inherited_kind100_compare_word,
        });
        self
    }

    pub fn vm_named_object_offsets(&self) -> &VmNamedObjectOffsets {
        &self.named_object_offsets
    }

    fn owner_object_offset(&self, record_offset: u16) -> Option<u16> {
        self.object_offsets
            .iter()
            .rev()
            .copied()
            .find(|offset| *offset < record_offset)
    }

    fn remap_special_rhs(&self, value: u16) -> u16 {
        if self.special_object_offset == Some(value) {
            0xffff
        } else {
            value
        }
    }

    fn is_special_rhs(&self, value: u16) -> bool {
        self.special_object_offset == Some(value)
    }

    fn c2_descript_lookup_succeeds(&self, state: &[u8], record_offset: u16) -> bool {
        let name_offset = record_offset.wrapping_add(4);
        self.descript_entry_names
            .iter()
            .any(|name| state_c_string_equals(state, name_offset, name))
    }
}

/// Force one condition result while executing a concrete scenario. This is a
/// branch-enumeration aid: the offset is the conditional opcode offset reported
/// in `BranchEvent`, and `condition_passed` is the result to use instead of the
/// current VAR-state comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BranchOverride {
    pub offset: usize,
    pub condition_passed: bool,
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

fn state_has_u16(state: &[u8], addr: u16) -> bool {
    (addr as usize)
        .checked_add(1)
        .is_some_and(|end| end < state.len())
}

fn state_set_u16(state: &mut [u8], addr: u16, val: u16) {
    let a = addr as usize;
    if a + 1 < state.len() {
        state[a] = (val & 0xFF) as u8;
        state[a + 1] = (val >> 8) as u8;
    }
}

fn state_u8(state: &[u8], addr: u16) -> u8 {
    state.get(addr as usize).copied().unwrap_or(0)
}

fn state_set_u8(state: &mut [u8], addr: u16, val: u8) {
    if let Some(slot) = state.get_mut(addr as usize) {
        *slot = val;
    }
}

fn state_or_u8(state: &mut [u8], addr: u16, mask: u8) {
    let value = state_u8(state, addr) | mask;
    state_set_u8(state, addr, value);
}

fn state_and_u8(state: &mut [u8], addr: u16, mask: u8) {
    let value = state_u8(state, addr) & mask;
    state_set_u8(state, addr, value);
}

fn state_and_u16(state: &mut [u8], addr: u16, mask: u16) {
    let value = state_u16(state, addr) & mask;
    state_set_u16(state, addr, value);
}

fn pending_script_profile_dispatch_ready(state: &[u8]) -> bool {
    state_has_u16(state, VM_PENDING_RESOURCE_PROFILE)
        && state_u16(state, VM_PENDING_RESOURCE_PROFILE) != 0xffff
        && state_u8(state, VM_UI_FLAGS) & 0x0e == 0
        && MAIN_PENDING_PROFILE_IDLE_GATES
            .iter()
            .all(|addr| state_u8(state, *addr) == 0)
}

fn state_c_string_equals(state: &[u8], addr: u16, expected: &[u8]) -> bool {
    let start = addr as usize;
    let end = match start.checked_add(expected.len()) {
        Some(end) => end,
        None => return false,
    };
    if end >= state.len() {
        return false;
    }
    &state[start..end] == expected && state[end] == 0
}

fn text_line_should_display(state: &[u8], line_index: u16, flags_b5: u8) -> bool {
    text_flags_are_active(flags_b5)
        && !text_line_already_shown(state_u16(state, text_line_flags_offset(line_index)))
}

fn text_presentation_record_is_active(state: &[u8], line_index: u16) -> bool {
    state_u16(state, text_presentation_record_offset(line_index)) == OP_ACTOR as u16
}

fn text_runtime_gates_allow(
    state: &[u8],
    context: &ExecutionContext,
    line_index: u16,
    flags_b5: u8,
) -> bool {
    (!context.text_presentation_record_gating
        || text_presentation_record_is_active(state, line_index))
        && (!context.text_line_display_gating
            || text_line_should_display(state, line_index, flags_b5))
}

#[derive(Default)]
struct TextTokenRuntimeFlags {
    flags_b5_by_offset: BTreeMap<usize, u8>,
}

impl TextTokenRuntimeFlags {
    fn flags_b5(&self, offset: usize, original_flags_b5: u8) -> u8 {
        self.flags_b5_by_offset
            .get(&offset)
            .copied()
            .unwrap_or(original_flags_b5)
    }

    fn accept_line(&mut self, offset: usize, flags_b4: u8, effective_flags_b5: u8) {
        let next = text_flags_after_accept(flags_b4, effective_flags_b5);
        if next != effective_flags_b5 {
            self.flags_b5_by_offset.insert(offset, next);
        }
    }
}

fn mark_text_line_shown(state: &mut [u8], line_index: u16) {
    let flags_offset = text_line_flags_offset(line_index);
    state_set_u16(
        state,
        flags_offset,
        state_u16(state, flags_offset) | TEXT_LINE_ALREADY_SHOWN_FLAG,
    );
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct SpecialObjectSlots {
    slots: [u16; SPECIAL_OBJECT_SLOT_COUNT],
}

impl SpecialObjectSlots {
    fn remove(&mut self, value: u16) -> bool {
        if let Some(slot) = self.slots.iter_mut().find(|slot| **slot == value) {
            *slot = 0;
            true
        } else {
            false
        }
    }

    fn insert(&mut self, value: u16) -> bool {
        if self.slots.contains(&value) {
            return true;
        }
        if let Some(slot) = self.slots.iter_mut().find(|slot| **slot == 0) {
            *slot = value;
            true
        } else {
            false
        }
    }
}

fn actor_object_offset_from_record(record_offset: u16) -> Option<u16> {
    record_offset.checked_sub(TALK_FIELD)
}

fn record_owner_object_offset(context: &ExecutionContext, record_offset: u16) -> Option<u16> {
    context.owner_object_offset(record_offset)
}

fn apply_assign5_mode0(
    state: &mut [u8],
    context: &ExecutionContext,
    special_slots: &mut SpecialObjectSlots,
    field_offset: u16,
    value: u16,
) {
    let owner = record_owner_object_offset(context, field_offset);
    if state_u16(state, field_offset) == 0xffff {
        if let Some(owner) = owner {
            special_slots.remove(owner);
        }
    }

    let mut stored = value;
    if value == 0xffff || context.is_special_rhs(value) {
        if let Some(owner) = owner {
            if !special_slots.insert(owner) {
                return;
            }
            stored = 0xffff;
        }
    }

    state_set_u16(state, field_offset, stored);
}

fn record_owner_is_active(
    state: &[u8],
    context: &ExecutionContext,
    record_offset: u16,
) -> Option<bool> {
    record_owner_object_offset(context, record_offset)
        .map(|owner| state_u8(state, owner.wrapping_add(2)) & 1 != 0)
}

fn actor_record_is_active(state: &[u8], record_offset: u16) -> bool {
    actor_object_offset_from_record(record_offset)
        .map(|actor| state_u8(state, actor.wrapping_add(2)) & 1 != 0)
        .unwrap_or(false)
}

fn actor_record_condition(
    state: &[u8],
    record_offset: u16,
    related_record_offset: u16,
    inverted: bool,
    strict: bool,
) -> Option<bool> {
    let record_type = state_u16(state, record_offset);
    let stored_related = state_u16(state, record_offset.wrapping_add(2));
    if !strict && record_type == 0 && stored_related == 0 {
        return None;
    }
    let matched = actor_record_is_active(state, record_offset)
        && record_type == OP_ACTOR as u16
        && stored_related == related_record_offset;
    Some(if inverted { !matched } else { matched })
}

fn write_actor_record(state: &mut [u8], record_offset: u16, related_record_offset: u16) {
    state_set_u16(state, record_offset, OP_ACTOR as u16);
    state_set_u16(state, record_offset.wrapping_add(2), related_record_offset);
    state_set_u16(state, record_offset.wrapping_add(4), 0);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PresentationKind1Update {
    Unchanged,
    Started,
    AlreadyActive,
    Stopped,
}

fn post_update_kind1_presentation_state(
    state: &mut [u8],
    record_offset: u16,
) -> PresentationKind1Update {
    if state_u16(state, record_offset) == OP_ACTOR as u16 {
        let related_offset = state_u16(state, record_offset.wrapping_add(2));
        state_set_u8(
            state,
            VM_PRESENTATION_RELATED_FLAG20,
            u8::from(state_u8(state, related_offset.wrapping_add(2)) & 0x20 != 0),
        );
        if state_u8(state, VM_PRESENTATION_ACTIVE) & 1 != 0 {
            return PresentationKind1Update::AlreadyActive;
        }

        state_set_u8(state, VM_PRESENTATION_SCENE_DIRTY, 1);
        state_set_u16(state, VM_PRESENTATION_STATUS_WORD, 1);
        state_set_u8(state, VM_PRESENTATION_ACTIVE, 1);
        state_set_u16(state, VM_BRANCH_A, 0);
        state_set_u16(state, VM_BRANCH_B, 0);
        state_set_u16(state, VM_PC_SAVED, 0);
        state_set_u16(state, VM_PRESENTATION_WORD_BUFFER, 0);
        state_set_u16(state, VM_PRESENTATION_INPUT_GATE_I, 0);
        state_set_u8(state, VM_PRESENTATION_TEXT_WAIT, 0);
        state_set_u8(state, VM_PRESENTATION_HANDOFF_GATE, 0);
        state_set_u8(state, VM_PRESENTATION_HOLD_READY, 0);
        state_set_u8(state, VM_PRESENTATION_HOLD_COMPLETE, 0);
        state_set_u16(state, VM_PRESENTATION_SIGNAL_SLOT, 0);
        state_set_u8(state, VM_PRESENTATION_START_LOCK, 1);
        state_or_u8(state, VM_UI_FLAGS, 0x04);
        state_or_u8(state, related_offset.wrapping_add(3), 0x80);
        state_and_u8(state, VM_PRESENTATION_INPUT_GATE_B, 0x7f);
        return PresentationKind1Update::Started;
    }

    if state_u8(state, VM_PRESENTATION_ACTIVE) & 1 == 0 {
        return PresentationKind1Update::Unchanged;
    }

    state_set_u16(state, VM_PRESENTATION_STATUS_WORD, 1);
    state_set_u16(state, VM_BRANCH_A, 0);
    state_set_u16(state, VM_BRANCH_B, 0);
    state_set_u8(state, VM_PRESENTATION_LOOP_FLAG, 0);
    state_set_u8(state, VM_PRESENTATION_ACTIVE, 0);
    state_set_u16(state, VM_PRESENTATION_ACTIVE_RECORD, 0);
    state_and_u16(state, VM_UI_FLAGS, 0xfffb);
    state_and_u8(state, C2_PRESENTATION_FLAGS, 0xfc);
    state_set_u16(state, VM_PRESENTATION_WORD_BUFFER, 0);
    state_set_u8(state, VM_PRESENTATION_START_LOCK, 0);
    state_set_u8(state, VM_PRESENTATION_DESCRIPTOR_PENDING, 0);
    PresentationKind1Update::Stopped
}

fn post_update_kind2_presentation_handoff_target(
    state: &[u8],
    context: &ExecutionContext,
    owner_offset: u16,
    record_offset: u16,
) -> Option<u16> {
    if state_u8(state, VM_PRESENTATION_ACTIVE) & 1 == 0
        || state_u8(state, C2_PRESENTATION_GATE) & 1 != 0
        || state_u8(state, VM_PRESENTATION_HANDOFF_GATE) & 1 != 0
        || state_u8(state, VM_PRESENTATION_START_LOCK) & 1 != 0
    {
        return None;
    }

    let primary_record = state_u16(state, VM_PRESENTATION_PRIMARY_C4_RECORD);
    if state_u16(state, primary_record) != OP_ACTOR as u16 {
        return None;
    }
    if state_u16(state, record_offset) != OP_ACTOR as u16 {
        return None;
    }
    if Some(state_u16(state, record_offset.wrapping_add(2))) != context.special_object_offset {
        return None;
    }
    if state_u16(state, owner_offset.wrapping_add(2)) & TEXT_LINE_ALREADY_SHOWN_FLAG != 0 {
        return None;
    }

    let owner_kind = state_u16(state, owner_offset);
    let target_offset = owner_offset.wrapping_add(vm_field_offset(
        VM_FIELD_OFFSET_SELECTOR_PRESENTATION_HANDOFF,
        owner_kind,
    )?);
    let target = state_u16(state, target_offset);
    (target != 0).then_some(target)
}

fn post_update_deferred_record_write(
    state: &mut [u8],
    context: &ExecutionContext,
    record_offset: u16,
) -> Option<u16> {
    let related = state_u16(state, VM_PRESENTATION_DEFERRED_RECORD_RELATED);
    if related == 0 {
        return None;
    }
    let record_type = state_u16(state, VM_PRESENTATION_DEFERRED_RECORD_TYPE);
    if record_type == 0 {
        return None;
    }

    let write_offset = if record_type == OP_RECORD_STATE_MIN as u16
        || record_type == OP_RECORD_ENTRY_MIN as u16 + 1
    {
        let arche_offset = context.named_object_offsets.arche?;
        let field_offset = vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, 0x10)?;
        let write_offset = arche_offset.wrapping_add(field_offset);
        state_set_u16(state, write_offset, record_type);
        state_set_u16(state, write_offset.wrapping_add(2), related);
        state_set_u16(state, write_offset.wrapping_add(4), 0);
        write_offset
    } else {
        state_set_u16(state, record_offset, record_type);
        state_set_u16(state, record_offset.wrapping_add(2), related);
        state_set_u16(
            state,
            record_offset.wrapping_add(4),
            state_u16(state, VM_PRESENTATION_DEFERRED_RECORD_AUX),
        );
        record_offset
    };

    state_set_u16(state, VM_PRESENTATION_DEFERRED_RECORD_TYPE, 0);
    state_set_u16(state, VM_PRESENTATION_DEFERRED_RECORD_RELATED, 0);
    state_set_u16(state, VM_PRESENTATION_DEFERRED_RECORD_AUX, 0);
    Some(write_offset)
}

fn post_update_actor_record_pair(
    state: &mut [u8],
    owner_offset: u16,
    record_offset: u16,
) -> Option<u16> {
    if state_u16(state, record_offset) != OP_ACTOR as u16
        || state_u16(state, record_offset.wrapping_add(4)) != 0
        || state_u8(state, VM_PRESENTATION_PAIR_WRITE_DISABLED) & 1 != 0
    {
        return None;
    }

    state_set_u16(
        state,
        record_offset.wrapping_add(4),
        C4_POST_UPDATE_SENTINEL,
    );

    let related_offset = state_u16(state, record_offset.wrapping_add(2));
    let related_kind = state_u16(state, related_offset);
    let related_field = related_offset.wrapping_add(vm_field_offset(
        VM_FIELD_OFFSET_SELECTOR_C9_RELATED,
        related_kind,
    )?);
    state_set_u16(state, related_field, OP_ACTOR as u16);
    state_set_u16(state, related_field.wrapping_add(2), owner_offset);
    state_set_u16(
        state,
        related_field.wrapping_add(4),
        C4_POST_UPDATE_SENTINEL,
    );
    Some(related_field)
}

fn post_update_actor_records_for_active_objects(
    state: &mut [u8],
    context: &ExecutionContext,
) -> Vec<(u16, u16)> {
    post_update_execution_state(state, context)
        .actor_record_pairs
        .into_iter()
        .map(|event| (event.record_offset, event.related_record_offset))
        .collect()
}

fn post_update_execution_state(state: &mut [u8], context: &ExecutionContext) -> PostUpdateTrace {
    let mut post_update = PostUpdateTrace::default();
    state_set_u8(state, VM_PRESENTATION_PAIR_WRITE_DISABLED, 0);
    for owner_offset in context.object_offsets.iter().copied() {
        if state_u8(state, owner_offset.wrapping_add(2)) & 1 == 0 {
            continue;
        }
        let owner_kind = state_u16(state, owner_offset);
        let Some(field_offset) = vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, owner_kind)
        else {
            continue;
        };
        let record_offset = owner_offset.wrapping_add(field_offset);
        if owner_kind == 2 {
            if let Some(target) = post_update_kind2_presentation_handoff_target(
                state,
                context,
                owner_offset,
                record_offset,
            ) {
                post_update
                    .presentation_handoffs
                    .push(PresentationHandoffEvent {
                        owner_offset,
                        record_offset,
                        target,
                    });
            }
        }
        if owner_kind == 1 {
            post_update_kind1_presentation_state(state, record_offset);
            post_update_deferred_record_write(state, context, record_offset);
        }
        if let Some(related_record_offset) =
            post_update_actor_record_pair(state, owner_offset, record_offset)
        {
            post_update
                .actor_record_pairs
                .push(PostUpdateActorRecordPair {
                    record_offset,
                    related_record_offset,
                });
        }
    }
    post_update.pending_script_profile_dispatch_ready =
        pending_script_profile_dispatch_ready(state);
    post_update
}

fn append_post_update_trace(
    post_update: &mut PostUpdateTrace,
    mut pass_update: PostUpdateTrace,
) -> Option<u16> {
    let handoff_target = pass_update
        .presentation_handoffs
        .last()
        .map(|event| event.target);
    post_update
        .actor_record_pairs
        .append(&mut pass_update.actor_record_pairs);
    post_update
        .presentation_handoffs
        .append(&mut pass_update.presentation_handoffs);
    post_update.pending_script_profile_dispatch_ready =
        pass_update.pending_script_profile_dispatch_ready;
    handoff_target
}

fn record_link_condition(
    state: &[u8],
    context: &ExecutionContext,
    record_offset: u16,
    related_record_offset: u16,
    inverted: bool,
) -> Option<bool> {
    let record_type = state_u16(state, record_offset);
    let stored_related = state_u16(state, record_offset.wrapping_add(2));
    if record_type == 0 && stored_related == 0 {
        return None;
    }
    let owner_active = record_owner_is_active(state, context, record_offset)?;
    let matched = owner_active
        && record_type == OP_RECORD_LINK as u16
        && stored_related == related_record_offset;
    Some(if inverted { !matched } else { matched })
}

fn write_record_link(state: &mut [u8], record_offset: u16, related_record_offset: u16) {
    state_set_u16(state, record_offset, OP_RECORD_LINK as u16);
    state_set_u16(state, record_offset.wrapping_add(2), related_record_offset);
    state_set_u16(state, record_offset.wrapping_add(4), 1);
}

fn write_record_link_mode0(
    state: &mut [u8],
    context: &ExecutionContext,
    record_offset: u16,
    related_record_offset: u16,
) -> Option<bool> {
    let owner_active = record_owner_is_active(state, context, record_offset)?;
    if !owner_active
        || state_u8(state, related_record_offset.wrapping_add(2)) & 1 == 0
        || state_u16(state, record_offset) == OP_ACTOR as u16
    {
        return Some(false);
    }

    write_record_link(state, record_offset, related_record_offset);
    Some(true)
}

fn record_state_condition(
    state: &[u8],
    context: &ExecutionContext,
    opcode: u8,
    record_offset: u16,
    operand: u16,
    inverted: bool,
) -> Option<bool> {
    let record_type = state_u16(state, record_offset);
    let stored_operand = state_u16(state, record_offset.wrapping_add(2));
    if opcode == OP_RECORD_STATE_MIN {
        if let Some(passed) = c1_record_state_resolved_mode1_condition(
            state,
            context,
            record_offset,
            operand,
            record_type,
            inverted,
        ) {
            return Some(passed);
        }
    }
    if record_type == 0 && stored_operand == 0 {
        return None;
    }
    let owner_active = if opcode == 0xC2 {
        record_owner_is_active(state, context, record_offset)?
    } else {
        true
    };
    let matched = owner_active && record_type == opcode as u16 && stored_operand == operand;
    Some(if inverted { !matched } else { matched })
}

fn c1_record_state_resolved_mode1_condition(
    state: &[u8],
    context: &ExecutionContext,
    record_offset: u16,
    operand: u16,
    direct_record_type: u16,
    inverted: bool,
) -> Option<bool> {
    if direct_record_type == OP_RECORD_STATE_MIN as u16 || (operand != 1 && operand != 2) {
        return None;
    }

    let owner_offset = record_owner_object_offset(context, record_offset)?;
    let parent_field = vm_field_offset(ship3d::SHIP_3D_FIELD_SELECTOR_PARENT_LINK, operand)?;
    let target_offset = state_u16(state, owner_offset.wrapping_add(parent_field));
    let target_kind = state_u16(state, target_offset);
    let Some(destination_field) =
        vm_field_offset(ship3d::SHIP_3D_C1_DESTINATION_SELECTOR, target_kind)
    else {
        return Some(inverted);
    };
    if destination_field == 0 {
        return Some(inverted);
    }

    let slot_offset = target_offset.wrapping_add(destination_field);
    let matched = state_u16(state, slot_offset) == OP_RECORD_STATE_MIN as u16
        && state_u16(state, slot_offset.wrapping_add(2)) == operand;
    Some(if inverted { !matched } else { matched })
}

fn ship3d_c1_source_records_from_bytes(source_list_bytes: &[u8]) -> Option<Vec<u16>> {
    let mut source_records = Vec::new();
    for chunk in source_list_bytes.chunks_exact(2) {
        let record = u16::from_le_bytes([chunk[0], chunk[1]]);
        source_records.push(record);
        if record == ship3d::SHIP_3D_TARGET_EXIT_SENTINEL {
            return Some(source_records);
        }
    }
    None
}

fn ship3d_record_state_slot(state: &[u8], record_offset: u16) -> ship3d::Ship3dRecordStateSlot {
    ship3d::Ship3dRecordStateSlot {
        opcode: state_u16(state, record_offset),
        operand: state_u16(state, record_offset.wrapping_add(2)),
        aux_word: state_u16(state, record_offset.wrapping_add(4)),
    }
}

fn write_ship3d_record_state_slot(
    state: &mut [u8],
    record_offset: u16,
    slot: ship3d::Ship3dRecordStateSlot,
) {
    state_set_u16(state, record_offset, slot.opcode);
    state_set_u16(state, record_offset.wrapping_add(2), slot.operand);
    state_set_u16(state, record_offset.wrapping_add(4), slot.aux_word);
}

fn resolve_c1_record_state_ship3d_target(
    state: &[u8],
    runtime: &Ship3dC1RuntimeContext,
    owner_offset: u16,
    operand: u16,
) -> Option<Option<u16>> {
    let owner_kind = state_u16(state, owner_offset);
    let mut target_offset = owner_offset;

    if operand == 1 || operand == 2 {
        let Some(position_runtime) = runtime.position_runtime.as_ref() else {
            return Some(None);
        };
        let Some(distance) = ship3d::ship_3d_position_distance(
            &position_runtime.records,
            &position_runtime.fields,
            operand,
            owner_offset,
            position_runtime.arche_object,
            position_runtime.inherited_kind100_compare_word,
        ) else {
            return Some(None);
        };

        if distance != 0 {
            let Some(parent_field) =
                vm_field_offset(ship3d::SHIP_3D_FIELD_SELECTOR_PARENT_LINK, owner_kind)
            else {
                return Some(None);
            };
            if parent_field == 0 {
                return Some(None);
            }
            target_offset = state_u16(state, owner_offset.wrapping_add(parent_field));
            if state_u16(state, target_offset) != ship3d::SHIP_3D_C1_KIND10_RECORD_KIND {
                return Some(None);
            }
        }
    }

    if state_u16(state, target_offset) == ship3d::SHIP_3D_C1_KIND10_RECORD_KIND {
        Some(Some(target_offset))
    } else {
        None
    }
}

fn write_c1_record_state_ship3d(
    state: &mut [u8],
    context: &ExecutionContext,
    owner_offset: u16,
    operand: u16,
) -> Option<bool> {
    let Some(runtime) = context.ship3d_c1_runtime.as_ref() else {
        return None;
    };
    let Some(target_offset) =
        resolve_c1_record_state_ship3d_target(state, runtime, owner_offset, operand)
    else {
        return None;
    };
    let Some(target_offset) = target_offset else {
        return Some(false);
    };

    let Some(source_records) = ship3d_c1_source_records_from_bytes(&runtime.source_list_bytes)
    else {
        return Some(false);
    };
    let Some(selected_source) = ship3d::select_ship_3d_c1_source_record(
        &source_records,
        &runtime.navigation_records,
        &runtime.object_table_records,
        &runtime.source_list_bytes,
        operand,
        state_u8(state, operand.wrapping_add(2)),
    ) else {
        return Some(false);
    };
    if selected_source.is_none() {
        return Some(false);
    }

    let Some(destination_record_offset) = ship3d::resolve_ship_3d_c1_kind10_destination_record(
        target_offset,
        ship3d::SHIP_3D_C1_KIND10_RECORD_KIND,
    ) else {
        return Some(false);
    };
    let mut slot = ship3d_record_state_slot(state, destination_record_offset);
    match ship3d::write_ship_3d_c1_kind10_destination_slot(
        target_offset,
        ship3d::SHIP_3D_C1_KIND10_RECORD_KIND,
        &mut slot,
        operand,
    ) {
        Some(Some(write)) => {
            write_ship3d_record_state_slot(state, write.destination_record_offset, write.slot);
            Some(true)
        }
        None | Some(None) => Some(false),
    }
}

fn write_c1_record_state_mode0(
    state: &mut [u8],
    context: &ExecutionContext,
    record_offset: u16,
    operand: u16,
) -> Option<bool> {
    let Some(owner_offset) = record_owner_object_offset(context, record_offset) else {
        return None;
    };
    if state_u8(state, owner_offset.wrapping_add(2)) & 1 == 0 {
        return Some(false);
    }
    if let Some(wrote) = write_c1_record_state_ship3d(state, context, owner_offset, operand) {
        return Some(wrote);
    }
    if state_u16(state, record_offset) != 0 {
        return Some(false);
    }
    state_set_u16(state, record_offset, OP_RECORD_STATE_MIN as u16);
    state_set_u16(state, record_offset.wrapping_add(2), operand);
    state_set_u16(state, record_offset.wrapping_add(4), 2);
    Some(true)
}

fn write_c2_record_state_direct(
    state: &mut [u8],
    context: &ExecutionContext,
    special_slots: &mut SpecialObjectSlots,
    record_offset: u16,
    target_record_offset: u16,
) -> bool {
    if record_owner_is_active(state, context, record_offset) != Some(true) {
        return false;
    }
    if state_u8(state, target_record_offset.wrapping_add(2)) & 0x20 == 0 {
        return false;
    }
    if !special_slots.insert(target_record_offset) {
        return false;
    }

    let kind = state_u16(state, target_record_offset);
    if let Some(field_offset) = vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C2, kind) {
        state_set_u16(
            state,
            target_record_offset.wrapping_add(field_offset),
            0xffff,
        );
    }

    if state_u8(state, 0x2793) & 1 == 0
        && state_u8(state, C2_PRESENTATION_FLAGS) & C2_PRESENTATION_BUSY_FLAG == 0
    {
        if kind == 2 {
            state_set_u8(state, C2_PRESENTATION_GATE, 0);
            state_set_u16(state, VM_ACTIVE_LINE, C2_ACTIVE_LINE_KIND2);
        } else if kind == 0x0400 && context.c2_descript_lookup_succeeds(state, target_record_offset)
        {
            state_set_u8(state, C2_PRESENTATION_GATE, 0);
            state_set_u8(
                state,
                C2_PRESENTATION_FLAGS,
                state_u8(state, C2_PRESENTATION_FLAGS) | C2_PRESENTATION_BUSY_FLAG,
            );
            state_set_u16(state, VM_ACTIVE_LINE, C2_ACTIVE_LINE_KIND400);
        }
    }

    true
}

fn clear_record_words(state: &mut [u8], record_offset: u16) {
    state_set_u16(state, record_offset, 0);
    state_set_u16(state, record_offset.wrapping_add(2), 0);
    state_set_u16(state, record_offset.wrapping_add(4), 0);
}

fn clear_record(state: &mut [u8], record_offset: u16) -> Option<u16> {
    let old_type = state_u16(state, record_offset);
    let old_related = state_u16(state, record_offset.wrapping_add(2));
    clear_record_words(state, record_offset);
    if old_type != OP_ACTOR as u16 {
        return None;
    }

    let related_kind = state_u16(state, old_related);
    if let Some(field_offset) = vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, related_kind) {
        clear_record_words(state, old_related.wrapping_add(field_offset));
    }
    state_set_u8(state, C9_PRESENTATION_GATE_A, 0);
    state_set_u8(state, C9_PRESENTATION_GATE_B, 6);
    Some(old_related)
}

fn write_record_entry(state: &mut [u8], opcode: u8, record_offset: u16, stored_related: u16) {
    state_set_u16(state, record_offset, opcode as u16);
    state_set_u16(state, record_offset.wrapping_add(2), stored_related);
    state_set_u16(state, record_offset.wrapping_add(4), 0);
}

fn write_record_entry_mode0(
    state: &mut [u8],
    opcode: u8,
    record_offset: u16,
    operand: u16,
) -> bool {
    match opcode {
        0xC5 => {
            if state_u8(state, operand.wrapping_add(2)) & 1 == 0
                || state_u16(state, operand) != 0x0200
                || state_u16(state, record_offset) != 0
            {
                return false;
            }
            write_record_entry(state, opcode, record_offset, operand);
            true
        }
        0xC6 => {
            write_record_entry(state, opcode, record_offset, operand);
            true
        }
        0xC7 => {
            let record_type = state_u16(state, record_offset);
            if state_u8(state, operand.wrapping_add(2)) & 1 == 0
                || (record_type != 0 && record_type != OP_ACTOR as u16)
            {
                return false;
            }
            write_record_entry(state, opcode, record_offset, operand);
            true
        }
        0xC8 => {
            if state_u16(state, record_offset) != 0 {
                return false;
            }
            write_record_entry(state, opcode, record_offset, 0);
            true
        }
        _ => false,
    }
}

fn record_entry_condition(
    state: &[u8],
    opcode: u8,
    record_offset: u16,
    operand: u16,
    inverted: bool,
) -> Option<bool> {
    let record_type = state_u16(state, record_offset);
    let stored_related = state_u16(state, record_offset.wrapping_add(2));
    if record_type == 0 && stored_related == 0 {
        return None;
    }
    let matched = record_type == opcode as u16 && stored_related == operand;
    Some(if inverted { !matched } else { matched })
}

fn branch_fail(branch_stack: &mut Vec<u16>) -> Option<u16> {
    branch_stack.pop()
}

fn push_mode0_branch_fail(
    branch_stack: &mut Vec<u16>,
    branch_events: &mut Vec<BranchEvent>,
    offset: usize,
    opcode: u8,
    detail: &'static str,
) -> Option<u16> {
    let target = branch_fail(branch_stack)?;
    branch_events.push(BranchEvent {
        offset,
        opcode,
        target: Some(target),
        branch_taken: true,
        condition_passed: Some(false),
        stack_depth: branch_stack.len(),
        detail,
    });
    Some(target)
}

fn compare_vm_words(operator: u8, left: u16, right: u16) -> Option<bool> {
    let signed_left = left as i16;
    let signed_right = right as i16;
    match operator {
        0xF0 => Some(left != right),
        0xF1 => Some(signed_left < signed_right),
        0xF2 => Some(signed_left > signed_right),
        0xF3 => Some(signed_left <= signed_right),
        0xF4 => Some(signed_left >= signed_right),
        0xF5 => Some(left == right),
        _ => None,
    }
}

fn global_word_condition(context: &ExecutionContext, operator: u8, value: u16) -> Option<bool> {
    let global = context.global_word_0aa6?;
    let passed = match operator {
        0xF1 => (value as i16) > (global as i16),
        0xF2 => (value as i16) < (global as i16),
        _ => value == global,
    };
    Some(passed)
}

fn global_pair_condition(
    context: &ExecutionContext,
    operator: u8,
    packed_value: u16,
) -> Option<bool> {
    let (global_high, global_low) = context.global_pair_0aaa_0aa8?;
    let token_high = (packed_value >> 8) as u8;
    let token_low = packed_value as u8;
    let token_pair = (token_high as i8, token_low as i8);
    let global_pair = (global_high as i8, global_low as i8);
    let passed = match operator {
        0xF1 => token_pair > global_pair,
        0xF2 => token_pair < global_pair,
        _ => token_high == global_high && token_low == global_low,
    };
    Some(passed)
}

/// Walk `cod`, executing assignment opcodes against a copy of `var` (the initial
/// state image), and return the resolved scene state at every `0xA6` line.
pub fn interpret_line_states(cod: &[u8], var: &[u8]) -> Vec<LineState> {
    interpret_line_states_with_context(cod, var, &ExecutionContext::default())
}

pub fn interpret_line_states_with_context(
    cod: &[u8],
    var: &[u8],
    context: &ExecutionContext,
) -> Vec<LineState> {
    let mut state = var.to_vec();
    let mut actor: Option<u16> = None;
    let mut out = Vec::new();
    let mut special_slots = SpecialObjectSlots::default();
    let mut text_token_flags = TextTokenRuntimeFlags::default();
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
            let inverted = mode1 && cod.get(pos + 1) == Some(&0xA1);
            let operand_pos = pos + 1 + usize::from(inverted);
            if let Some(record_offset) = read_u16(cod, operand_pos) {
                if let Some(actor_offset) = actor_object_offset_from_record(record_offset) {
                    actor = Some(actor_offset);
                }
                if !mode1 {
                    let related_record_offset = read_u16(cod, operand_pos + 2).unwrap_or(0);
                    write_actor_record(&mut state, record_offset, related_record_offset);
                }
            }
        }
        if op == OP_RECORD_CLEAR {
            if let Some(record_offset) = read_u16(cod, pos + 1) {
                clear_record(&mut state, record_offset);
                if actor.map(|a| a.wrapping_add(TALK_FIELD)) == Some(record_offset) {
                    actor = None;
                }
            }
        }
        if !mode1 && is_record_entry_opcode(op) {
            let record_offset = read_u16(cod, pos + 1).unwrap_or(0);
            let operand = read_u16(cod, pos + 3).unwrap_or(0);
            write_record_entry_mode0(&mut state, op, record_offset, operand);
        }
        if !mode1 && ASSIGN_7.contains(&op) && pos + 7 <= end {
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
        if !mode1 && BITMASK_5.contains(&op) {
            let mut p = pos + 1;
            let clear = cod.get(p) == Some(&0xA1);
            if clear {
                p += 1;
            }
            if p + 4 <= end {
                let op1 = read_u16(cod, p).unwrap_or(0);
                let mask = read_u16(cod, p + 2).unwrap_or(0);
                let cur = state_u16(&state, op1);
                let next = if clear { cur & !mask } else { cur | mask };
                state_set_u16(&mut state, op1, next);
            }
        }
        if !mode1 && ASSIGN_5.contains(&op) && pos + 5 <= end {
            let op1 = read_u16(cod, pos + 1).unwrap_or(0);
            let value = read_u16(cod, pos + 3).unwrap_or(0);
            apply_assign5_mode0(&mut state, context, &mut special_slots, op1, value);
        }
        if !mode1 && op == OP_BIT_FLAG {
            let clear = cod.get(pos + 1) == Some(&0xA1);
            let p = pos + 1 + usize::from(clear);
            if p + 3 <= end {
                let flag_offset = read_u16(cod, p).unwrap_or(0);
                let bit_index = cod[p + 2];
                let byte_offset = bit_flag_byte_offset(flag_offset, bit_index);
                let mask = bit_flag_mask(bit_index);
                let cur = state_u8(&state, byte_offset);
                let next = if clear { cur & !mask } else { cur | mask };
                state_set_u8(&mut state, byte_offset, next);
            }
        }
        if !mode1 && is_pair_record_opcode(op) && pos + 7 <= end {
            let record_offset = read_u16(cod, pos + 1).unwrap_or(0);
            let first_word = read_u16(cod, pos + 3).unwrap_or(0);
            let second_word = read_u16(cod, pos + 5).unwrap_or(0);
            state_set_u16(&mut state, record_offset, first_word);
            state_set_u16(&mut state, record_offset.wrapping_add(2), second_word);
        }
        if !mode1 && op == OP_RECORD_STATE_MIN && pos + 5 <= end {
            let record_offset = read_u16(cod, pos + 1).unwrap_or(0);
            let operand = read_u16(cod, pos + 3).unwrap_or(0);
            let _ = write_c1_record_state_mode0(&mut state, context, record_offset, operand);
        }
        if !mode1 && op == OP_RECORD_STATE_MAX && pos + 5 <= end {
            let record_offset = read_u16(cod, pos + 1).unwrap_or(0);
            let operand = read_u16(cod, pos + 3).unwrap_or(0);
            write_c2_record_state_direct(
                &mut state,
                context,
                &mut special_slots,
                record_offset,
                operand,
            );
        }

        if op == OP_TEXT {
            match decode_text(cod, pos, end) {
                Some((
                    VmToken::Text {
                        line_index,
                        flags_b4,
                        flags_b5,
                        ..
                    },
                    next,
                )) => {
                    let effective_flags_b5 = text_token_flags.flags_b5(pos, flags_b5);
                    if text_runtime_gates_allow(&state, context, line_index, effective_flags_b5) {
                        if context.text_line_display_gating {
                            mark_text_line_shown(&mut state, line_index);
                        }
                        text_token_flags.accept_line(pos, flags_b4, effective_flags_b5);
                        let location_offset =
                            actor.map(|a| state_u16(&state, a.wrapping_add(LOCATION_FIELD)));
                        out.push(LineState {
                            offset: pos,
                            actor_offset: actor,
                            location_offset,
                        });
                    }
                    pos = next;
                }
                None => break,
                _ => unreachable!("decode_text only returns TEXT tokens"),
            }
            continue;
        }
        // Same per-mode zero-length rule as `walk` (vm_token_advance 0x62B6):
        // a resolved length of 0 means zero-word-terminated in THAT mode.
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
            let l = if mode1 { b1 } else { b0 } as usize;
            if l == 0 {
                pos = scan_zero_word(cod, pos + 1, end);
                continue;
            }
            l
        };
        pos += len;
    }
    out
}

/// Execute the subset of VM control flow that has been tied to concrete handler
/// code. This follows A0/A1 condition blocks and direct A4/A9 jumps, while still
/// using the same bounded state model as `interpret_line_states`.
pub fn execute_trace(cod: &[u8], var: &[u8]) -> ExecutionTrace {
    execute_trace_with_overrides(cod, var, &[])
}

pub fn execute_trace_with_context(
    cod: &[u8],
    var: &[u8],
    context: &ExecutionContext,
) -> ExecutionTrace {
    execute_trace_with_overrides_and_context(cod, var, &[], context)
}

/// Execute a concrete VM path, optionally forcing selected condition outcomes.
/// Overrides are keyed by conditional opcode offset and are applied only after a
/// real condition has been decoded at that offset.
pub fn execute_trace_with_overrides(
    cod: &[u8],
    var: &[u8],
    overrides: &[BranchOverride],
) -> ExecutionTrace {
    execute_trace_with_overrides_and_context(cod, var, overrides, &ExecutionContext::default())
}

pub fn execute_trace_with_overrides_and_context(
    cod: &[u8],
    var: &[u8],
    overrides: &[BranchOverride],
    context: &ExecutionContext,
) -> ExecutionTrace {
    execute_trace_state_with_overrides_and_context(cod, var, overrides, context, 0).trace
}

/// Execute a concrete VM path starting at an arbitrary COD `start` offset instead
/// of the script entry (0). Used to reach dialogue in named functions that the
/// main control flow never calls (event-triggered scenes) — the biggest source of
/// uncovered dialogue. The function is expected to establish its own actor and
/// background context via its opening tokens, which the static symbol analysis
/// confirms it does (e.g. clay3 sets Anna_Haf / Magnus).
pub fn execute_trace_from_offset(cod: &[u8], var: &[u8], start: usize) -> ExecutionTrace {
    execute_trace_state_with_overrides_and_context(
        cod,
        var,
        &[],
        &ExecutionContext::default(),
        start,
    )
    .trace
}

fn execute_trace_state_with_overrides_and_context(
    cod: &[u8],
    var: &[u8],
    overrides: &[BranchOverride],
    context: &ExecutionContext,
    start: usize,
) -> ExecutedTrace {
    const STEP_LIMIT_MULTIPLIER: usize = 64;

    let mut state = var.to_vec();
    let mut actor: Option<u16> = None;
    let mut line_states = Vec::new();
    let mut branch_events = Vec::new();
    let mut script_profile_requests = Vec::new();
    let mut branch_stack: Vec<u16> = Vec::new();
    let mut post_update = PostUpdateTrace::default();
    let mut special_slots = SpecialObjectSlots::default();
    let mut text_token_flags = TextTokenRuntimeFlags::default();
    let mut pos = start;
    let mut mode1 = false;
    let end = cod.len();
    let step_limit = end.saturating_mul(STEP_LIMIT_MULTIPLIER).max(1024);
    let mut steps = 0usize;
    let mut halted = ExecutionHalt::EndMarker;

    'execution: loop {
        if pos >= end {
            if matches!(halted, ExecutionHalt::EndMarker) {
                let handoff_target = append_post_update_trace(
                    &mut post_update,
                    post_update_execution_state(&mut state, context),
                );
                if let Some(target) = handoff_target {
                    if target as usize >= end {
                        halted = ExecutionHalt::InvalidTarget {
                            offset: end,
                            target,
                        };
                        break 'execution;
                    }
                    pos = target as usize;
                    mode1 = false;
                    branch_stack.clear();
                    actor = None;
                    continue 'execution;
                }
            }
            break 'execution;
        }

        if steps >= step_limit {
            halted = ExecutionHalt::StepLimit { limit: step_limit };
            break 'execution;
        }
        steps += 1;

        let token_start = pos;
        let op = cod[token_start];
        if op == 0xFF {
            halted = ExecutionHalt::EndMarker;
            let handoff_target = append_post_update_trace(
                &mut post_update,
                post_update_execution_state(&mut state, context),
            );
            if let Some(target) = handoff_target {
                if target as usize >= end {
                    halted = ExecutionHalt::InvalidTarget {
                        offset: token_start,
                        target,
                    };
                    break 'execution;
                }
                pos = target as usize;
                mode1 = false;
                branch_stack.clear();
                actor = None;
                continue 'execution;
            }
            break 'execution;
        }
        if !(OP_MIN..=OP_MAX).contains(&op) {
            halted = ExecutionHalt::InvalidOpcode {
                offset: token_start,
                byte: op,
            };
            break 'execution;
        }
        let (b0, b1) = OPCODE_DESC[(op - OP_MIN) as usize];

        if op == 0xA0 {
            if let Some(target) = read_u16(cod, token_start + 1) {
                branch_stack.push(target);
                branch_events.push(BranchEvent {
                    offset: token_start,
                    opcode: op,
                    target: Some(target),
                    branch_taken: false,
                    condition_passed: None,
                    stack_depth: branch_stack.len(),
                    detail: "condition block start",
                });
            }
        } else if op == 0xA1 {
            if branch_stack.len() > 1 {
                branch_stack.pop();
            }
            branch_events.push(BranchEvent {
                offset: token_start,
                opcode: op,
                target: branch_stack.last().copied(),
                branch_taken: false,
                condition_passed: None,
                stack_depth: branch_stack.len(),
                detail: "condition block end",
            });
        } else if op == 0xA4 {
            let target = read_u16(cod, token_start + 1).unwrap_or(0);
            branch_events.push(BranchEvent {
                offset: token_start,
                opcode: op,
                target: Some(target),
                branch_taken: true,
                condition_passed: None,
                stack_depth: branch_stack.len(),
                detail: "direct jump",
            });
            if target as usize >= end {
                halted = ExecutionHalt::InvalidTarget {
                    offset: token_start,
                    target,
                };
                break 'execution;
            }
            pos = target as usize;
            continue;
        } else if op == 0xA9 {
            let flag = cod.get(token_start + 1).copied().unwrap_or(0);
            let target = read_u16(cod, token_start + 2).unwrap_or(0);
            if flag & 1 == 0 {
                branch_events.push(BranchEvent {
                    offset: token_start,
                    opcode: op,
                    target: Some(target),
                    branch_taken: true,
                    condition_passed: None,
                    stack_depth: branch_stack.len(),
                    detail: "indexed direct jump",
                });
                if target as usize >= end {
                    halted = ExecutionHalt::InvalidTarget {
                        offset: token_start,
                        target,
                    };
                    break 'execution;
                }
                pos = target as usize;
                continue;
            }
            mode1 = true;
            branch_stack.clear();
            branch_stack.push(target);
            branch_events.push(BranchEvent {
                offset: token_start,
                opcode: op,
                target: Some(target),
                branch_taken: false,
                condition_passed: None,
                stack_depth: branch_stack.len(),
                detail: "condition block reset",
            });
            pos = (token_start + 4).min(end);
            continue;
        }

        if op == OP_SCRIPT_PROFILE_REQUEST {
            let operand = cod.get(token_start + 1).copied().unwrap_or(0);
            let profile_index = script_profile_index_from_request_operand(operand);
            state_set_u16(&mut state, VM_PENDING_RESOURCE_PROFILE, profile_index);
            script_profile_requests.push(ScriptProfileRequestEvent {
                offset: token_start,
                operand,
                profile_index,
            });
        }

        let mut branch_target: Option<u16> = None;
        let mut condition_passed: Option<bool> = None;

        if mode1 && ASSIGN_7.contains(&op) && token_start + 7 <= end {
            let op1 = read_u16(cod, token_start + 1).unwrap_or(0);
            let operator = cod[token_start + 3];
            let op2mode = cod[token_start + 4];
            let op2 = read_u16(cod, token_start + 5).unwrap_or(0);
            let right = if op2mode == 0xC0 || op2mode == 0xC2 {
                state_u16(&state, op2)
            } else {
                op2
            };
            condition_passed = compare_vm_words(operator, state_u16(&state, op1), right);
        } else if mode1 && BITMASK_5.contains(&op) {
            let mut p = token_start + 1;
            let inverted = cod.get(p) == Some(&0xA1);
            if inverted {
                p += 1;
            }
            if p + 4 <= end {
                let op1 = read_u16(cod, p).unwrap_or(0);
                let mask = read_u16(cod, p + 2).unwrap_or(0);
                let bit_set = state_u16(&state, op1) & mask != 0;
                let passed = if inverted { !bit_set } else { bit_set };
                condition_passed = Some(passed);
            }
        } else if mode1 && ASSIGN_5.contains(&op) {
            let mut p = token_start + 1;
            let inverted = cod.get(p) == Some(&0xA1);
            if inverted {
                p += 1;
            }
            if p + 4 <= end {
                let op1 = read_u16(cod, p).unwrap_or(0);
                let value = context.remap_special_rhs(read_u16(cod, p + 2).unwrap_or(0));
                let equal = state_u16(&state, op1) == value;
                let passed = if inverted { !equal } else { equal };
                condition_passed = Some(passed);
            }
        } else if mode1 && op == OP_BIT_FLAG {
            let inverted = cod.get(token_start + 1) == Some(&0xA1);
            let p = token_start + 1 + usize::from(inverted);
            if p + 3 <= end {
                let flag_offset = read_u16(cod, p).unwrap_or(0);
                let bit_index = cod[p + 2];
                let byte_offset = bit_flag_byte_offset(flag_offset, bit_index);
                let bit_set = state_u8(&state, byte_offset) & bit_flag_mask(bit_index) != 0;
                condition_passed = Some(if inverted { !bit_set } else { bit_set });
            }
        } else if mode1 && is_pair_record_opcode(op) && token_start + 7 <= end {
            let record_offset = read_u16(cod, token_start + 1).unwrap_or(0);
            let first_word = read_u16(cod, token_start + 3).unwrap_or(0);
            let second_word = read_u16(cod, token_start + 5).unwrap_or(0);
            condition_passed = Some(
                state_u16(&state, record_offset) == first_word
                    && state_u16(&state, record_offset.wrapping_add(2)) == second_word,
            );
        } else if mode1 && is_record_state_opcode(op) {
            let inverted = cod.get(token_start + 1) == Some(&0xA1);
            let p = token_start + 1 + usize::from(inverted);
            if p + 4 <= end {
                let record_offset = read_u16(cod, p).unwrap_or(0);
                let operand = read_u16(cod, p + 2).unwrap_or(0);
                condition_passed =
                    record_state_condition(&state, context, op, record_offset, operand, inverted);
            }
        } else if mode1 && is_record_entry_opcode(op) {
            let inverted = cod.get(token_start + 1) == Some(&0xA1);
            let p = token_start + 1 + usize::from(inverted);
            if p + 4 <= end {
                let record_offset = read_u16(cod, p).unwrap_or(0);
                let operand = read_u16(cod, p + 2).unwrap_or(0);
                condition_passed =
                    record_entry_condition(&state, op, record_offset, operand, inverted);
            }
        } else if mode1 && op == OP_RECORD_LINK {
            let inverted = cod.get(token_start + 1) == Some(&0xA1);
            let p = token_start + 1 + usize::from(inverted);
            if p + 4 <= end {
                let record_offset = read_u16(cod, p).unwrap_or(0);
                let related_record_offset = read_u16(cod, p + 2).unwrap_or(0);
                condition_passed = record_link_condition(
                    &state,
                    context,
                    record_offset,
                    related_record_offset,
                    inverted,
                );
            }
        } else if mode1 && op == OP_ACTOR {
            let inverted = cod.get(token_start + 1) == Some(&0xA1);
            let p = token_start + 1 + usize::from(inverted);
            if p + 4 <= end {
                let record_offset = read_u16(cod, p).unwrap_or(0);
                let related_record_offset = read_u16(cod, p + 2).unwrap_or(0);
                condition_passed = actor_record_condition(
                    &state,
                    record_offset,
                    related_record_offset,
                    inverted,
                    context.strict_actor_record_branching,
                );
            }
        } else if mode1 && op == OP_RECORD_TRIPLE {
            let inverted = cod.get(token_start + 1) == Some(&0xA1);
            let p = token_start + 1 + usize::from(inverted);
            if p + 6 <= end {
                let record_offset = read_u16(cod, p).unwrap_or(0);
                let first_word = read_u16(cod, p + 2).unwrap_or(0);
                let second_word = read_u16(cod, p + 4).unwrap_or(0);
                let matched = state_u16(&state, record_offset) == OP_RECORD_TRIPLE as u16
                    && state_u16(&state, record_offset.wrapping_add(2)) == first_word
                    && state_u16(&state, record_offset.wrapping_add(4)) == second_word;
                condition_passed = Some(if inverted { !matched } else { matched });
            }
        } else if mode1 && op == OP_GLOBAL_WORD_COMPARE && token_start + 5 <= end {
            let operator = cod[token_start + 1];
            let value = read_u16(cod, token_start + 3).unwrap_or(0);
            condition_passed = global_word_condition(context, operator, value);
        } else if mode1 && op == OP_GLOBAL_PAIR_COMPARE && token_start + 6 <= end {
            let operator = cod[token_start + 1];
            let packed_value = read_u16(cod, token_start + 2).unwrap_or(0);
            condition_passed = global_pair_condition(context, operator, packed_value);
        }

        let forced = overrides
            .iter()
            .find(|override_| override_.offset == token_start)
            .copied();
        if condition_passed.is_some() {
            if let Some(override_) = forced {
                condition_passed = Some(override_.condition_passed);
            }
            if condition_passed == Some(false) {
                branch_target = branch_fail(&mut branch_stack);
            }
        }
        let branch_detail = match (forced, condition_passed) {
            (Some(_), Some(true)) => "condition forced passed",
            (Some(_), Some(false)) => "condition forced failed",
            (None, Some(true)) => "condition passed",
            _ => "condition failed",
        };

        if let Some(target) = branch_target {
            mode1 = false;
            branch_events.push(BranchEvent {
                offset: token_start,
                opcode: op,
                target: Some(target),
                branch_taken: true,
                condition_passed,
                stack_depth: branch_stack.len(),
                detail: branch_detail,
            });
            if target as usize >= end {
                halted = ExecutionHalt::InvalidTarget {
                    offset: token_start,
                    target,
                };
                break 'execution;
            }
            pos = target as usize;
            continue;
        } else if condition_passed.is_some() {
            branch_events.push(BranchEvent {
                offset: token_start,
                opcode: op,
                target: branch_stack.last().copied(),
                branch_taken: false,
                condition_passed,
                stack_depth: branch_stack.len(),
                detail: branch_detail,
            });
        }

        if !mode1 && op == OP_ACTOR {
            if let Some(record_offset) = read_u16(cod, token_start + 1) {
                if let Some(actor_offset) = actor_object_offset_from_record(record_offset) {
                    actor = Some(actor_offset);
                }
                let related_record_offset = read_u16(cod, token_start + 3).unwrap_or(0);
                write_actor_record(&mut state, record_offset, related_record_offset);
            }
        }
        if mode1 && op == OP_ACTOR {
            let inverted = cod.get(token_start + 1) == Some(&0xA1);
            let p = token_start + 1 + usize::from(inverted);
            if let Some(record_offset) = read_u16(cod, p) {
                if let Some(actor_offset) = actor_object_offset_from_record(record_offset) {
                    actor = Some(actor_offset);
                }
            }
        }
        if op == OP_RECORD_CLEAR {
            if let Some(record_offset) = read_u16(cod, token_start + 1) {
                clear_record(&mut state, record_offset);
                if actor.map(|a| a.wrapping_add(TALK_FIELD)) == Some(record_offset) {
                    actor = None;
                }
            }
        }
        if !mode1 && op == OP_RECORD_LINK {
            let record_offset = read_u16(cod, token_start + 1).unwrap_or(0);
            let related_record_offset = read_u16(cod, token_start + 3).unwrap_or(0);
            if let Some(false) =
                write_record_link_mode0(&mut state, context, record_offset, related_record_offset)
            {
                if let Some(target) = push_mode0_branch_fail(
                    &mut branch_stack,
                    &mut branch_events,
                    token_start,
                    op,
                    "mode0 C3 write failed",
                ) {
                    mode1 = false;
                    if target as usize >= end {
                        halted = ExecutionHalt::InvalidTarget {
                            offset: token_start,
                            target,
                        };
                        break 'execution;
                    }
                    pos = target as usize;
                    continue;
                }
            }
        }
        if !mode1 && is_record_entry_opcode(op) {
            let record_offset = read_u16(cod, token_start + 1).unwrap_or(0);
            let operand = read_u16(cod, token_start + 3).unwrap_or(0);
            if !write_record_entry_mode0(&mut state, op, record_offset, operand) {
                if let Some(target) = push_mode0_branch_fail(
                    &mut branch_stack,
                    &mut branch_events,
                    token_start,
                    op,
                    "mode0 record entry write failed",
                ) {
                    mode1 = false;
                    if target as usize >= end {
                        halted = ExecutionHalt::InvalidTarget {
                            offset: token_start,
                            target,
                        };
                        break 'execution;
                    }
                    pos = target as usize;
                    continue;
                }
            }
        }
        if !mode1 && ASSIGN_7.contains(&op) && token_start + 7 <= end {
            let op1 = read_u16(cod, token_start + 1).unwrap_or(0);
            let operator = cod[token_start + 3];
            let op2mode = cod[token_start + 4];
            let op2 = read_u16(cod, token_start + 5).unwrap_or(0);
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
                _ => None,
            };
            if let Some(v) = next {
                state_set_u16(&mut state, op1, v);
            }
        }
        if !mode1 && BITMASK_5.contains(&op) {
            let mut p = token_start + 1;
            let clear = cod.get(p) == Some(&0xA1);
            if clear {
                p += 1;
            }
            if p + 4 <= end {
                let op1 = read_u16(cod, p).unwrap_or(0);
                let mask = read_u16(cod, p + 2).unwrap_or(0);
                let cur = state_u16(&state, op1);
                let next = if clear { cur & !mask } else { cur | mask };
                state_set_u16(&mut state, op1, next);
            }
        }
        if !mode1 && ASSIGN_5.contains(&op) && token_start + 5 <= end {
            let op1 = read_u16(cod, token_start + 1).unwrap_or(0);
            let value = read_u16(cod, token_start + 3).unwrap_or(0);
            apply_assign5_mode0(&mut state, context, &mut special_slots, op1, value);
        }
        if !mode1 && op == OP_BIT_FLAG {
            let clear = cod.get(token_start + 1) == Some(&0xA1);
            let p = token_start + 1 + usize::from(clear);
            if p + 3 <= end {
                let flag_offset = read_u16(cod, p).unwrap_or(0);
                let bit_index = cod[p + 2];
                let byte_offset = bit_flag_byte_offset(flag_offset, bit_index);
                let mask = bit_flag_mask(bit_index);
                let cur = state_u8(&state, byte_offset);
                let next = if clear { cur & !mask } else { cur | mask };
                state_set_u8(&mut state, byte_offset, next);
            }
        }
        if !mode1 && is_pair_record_opcode(op) && token_start + 7 <= end {
            let record_offset = read_u16(cod, token_start + 1).unwrap_or(0);
            let first_word = read_u16(cod, token_start + 3).unwrap_or(0);
            let second_word = read_u16(cod, token_start + 5).unwrap_or(0);
            state_set_u16(&mut state, record_offset, first_word);
            state_set_u16(&mut state, record_offset.wrapping_add(2), second_word);
        }
        if !mode1 && op == OP_RECORD_STATE_MIN && token_start + 5 <= end {
            let record_offset = read_u16(cod, token_start + 1).unwrap_or(0);
            let operand = read_u16(cod, token_start + 3).unwrap_or(0);
            if let Some(false) =
                write_c1_record_state_mode0(&mut state, context, record_offset, operand)
            {
                if let Some(target) = push_mode0_branch_fail(
                    &mut branch_stack,
                    &mut branch_events,
                    token_start,
                    op,
                    "mode0 C1 write failed",
                ) {
                    mode1 = false;
                    if target as usize >= end {
                        halted = ExecutionHalt::InvalidTarget {
                            offset: token_start,
                            target,
                        };
                        break 'execution;
                    }
                    pos = target as usize;
                    continue;
                }
            }
        }
        if !mode1 && op == OP_RECORD_STATE_MAX && token_start + 5 <= end {
            let record_offset = read_u16(cod, token_start + 1).unwrap_or(0);
            let operand = read_u16(cod, token_start + 3).unwrap_or(0);
            write_c2_record_state_direct(
                &mut state,
                context,
                &mut special_slots,
                record_offset,
                operand,
            );
        }

        if op == OP_TEXT {
            match decode_text(cod, token_start, end) {
                Some((
                    VmToken::Text {
                        line_index,
                        flags_b4,
                        flags_b5,
                        ..
                    },
                    next,
                )) => {
                    let effective_flags_b5 = text_token_flags.flags_b5(token_start, flags_b5);
                    if text_runtime_gates_allow(&state, context, line_index, effective_flags_b5) {
                        if context.text_line_display_gating {
                            mark_text_line_shown(&mut state, line_index);
                        }
                        text_token_flags.accept_line(token_start, flags_b4, effective_flags_b5);
                        let location_offset =
                            actor.map(|a| state_u16(&state, a.wrapping_add(LOCATION_FIELD)));
                        line_states.push(LineState {
                            offset: token_start,
                            actor_offset: actor,
                            location_offset,
                        });
                    }
                    pos = next;
                    continue;
                }
                None => {
                    halted = ExecutionHalt::InvalidOpcode {
                        offset: token_start,
                        byte: op,
                    };
                    break 'execution;
                }
                _ => unreachable!("decode_text only returns TEXT tokens"),
            }
        }
        let len = if b1 & 0x80 != 0 {
            let mut l = b0 as usize;
            match b1 {
                0xFF => mode1 = true,
                0xFE => mode1 = false,
                0xFD | 0xFB => {
                    if cod.get(token_start + 1) == Some(&0xA1) {
                        l += 1;
                    }
                }
                _ => {}
            }
            l.max(1)
        } else {
            let l = if mode1 { b1 } else { b0 } as usize;
            if l == 0 {
                // Per-mode zero length = zero-word-terminated (0x6293).
                pos = scan_zero_word(cod, token_start + 1, end);
                continue;
            }
            l
        };
        pos = (token_start + len).min(end);
    }

    let trace = ExecutionTrace {
        line_states,
        branch_events,
        script_profile_requests,
        post_update,
        steps,
        halted,
    };

    ExecutedTrace {
        trace,
        final_state: state,
    }
}

pub fn execute_script_profile_sequence(
    programs: &[ScriptProfileProgram<'_>],
    initial_profile_index: u16,
    run_limit: usize,
) -> ScriptProfileExecution {
    let mut runs = Vec::new();
    let mut next_profile_index = initial_profile_index;
    let mut runtime_states: BTreeMap<u16, Vec<u8>> = programs
        .iter()
        .map(|program| (program.profile_index, program.var.to_vec()))
        .collect();

    for run_index in 0..run_limit {
        let Some(program) = programs
            .iter()
            .find(|program| program.profile_index == next_profile_index)
        else {
            return ScriptProfileExecution {
                runs,
                halted: ScriptProfileExecutionHalt::MissingProfile {
                    profile_index: next_profile_index,
                },
            };
        };

        let initial_state = runtime_states
            .get(&program.profile_index)
            .map(Vec::as_slice)
            .unwrap_or(program.var);
        let executed = execute_trace_state_with_overrides_and_context(
            program.cod,
            initial_state,
            &[],
            &program.context,
            0,
        );
        runtime_states.insert(program.profile_index, executed.final_state);
        let trace = executed.trace;
        let pending = trace.pending_script_profile();
        let pending_dispatch_ready = trace.post_update.pending_script_profile_dispatch_ready;
        runs.push(ScriptProfileRun {
            run_index,
            profile_index: program.profile_index,
            trace,
        });

        match pending {
            Some(profile_index) if pending_dispatch_ready => next_profile_index = profile_index,
            Some(profile_index) => {
                return ScriptProfileExecution {
                    runs,
                    halted: ScriptProfileExecutionHalt::PendingProfileNotReady { profile_index },
                };
            }
            None => {
                return ScriptProfileExecution {
                    runs,
                    halted: ScriptProfileExecutionHalt::NoPendingProfile,
                };
            }
        }
    }

    ScriptProfileExecution {
        runs,
        halted: ScriptProfileExecutionHalt::RunLimit {
            limit: run_limit,
            next_profile_index,
        },
    }
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
// The current mp4 pipeline consumes these events from branch-aware executed
// dialogue rows. The remaining accuracy work is to enumerate or select
// non-initial branches and move from per-character composites to whole dialogue
// runs.
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
        active_line_id: u16,
        flags: u8,
        skip_count: Option<u8>,
        loop_target: Option<u16>,
    },
    /// Subtitle chatter event from the dialogue display state machine (tb.snd).
    PlayChatter {
        active_line_id: u16,
    },
    UnresolvedBackground {
        active_line_id: u16,
    },
    UnresolvedActor {
        active_line_id: u16,
    },
    UnresolvedVoice {
        voice_selector: u8,
        active_line_id: u16,
    },
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
    pub active_line_id: u16,
    pub flags_b4: u8,
    pub skip_count: Option<u8>,
    pub loop_target: Option<u16>,
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
        if line.background_record.is_none() && line.background_hnm.is_none() {
            events.push(SceneEvent::UnresolvedBackground {
                active_line_id: line.active_line_id,
            });
        }
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
        } else if !line.text.trim().is_empty() {
            events.push(SceneEvent::UnresolvedActor {
                active_line_id: line.active_line_id,
            });
        }
        if let Some(clip) = line.clip_index {
            events.push(SceneEvent::PlayTalkHnm { clip_index: clip });
            events.push(SceneEvent::PlayVoice { clip_index: clip });
        } else if line.actor.is_some()
            && line.flags_b4 < 0x10
            && text_selector_requests_voice(line.voice_selector)
        {
            events.push(SceneEvent::UnresolvedVoice {
                voice_selector: line.voice_selector,
                active_line_id: line.active_line_id,
            });
        }
        events.push(SceneEvent::DrawSubtitle {
            text: line.text.clone(),
            voice_selector: line.voice_selector,
            active_line_id: line.active_line_id,
            flags: line.flags_b4,
            skip_count: line.skip_count,
            loop_target: line.loop_target,
        });
        events.push(SceneEvent::PlayChatter {
            active_line_id: line.active_line_id,
        });
    }
    events.push(SceneEvent::Clear);
    events
}


// ============================================================================
// FAITHFUL VM EXECUTOR — ported opcode-by-opcode from the BLOODPRG disassembly
// (dispatch 0x5627 via the handler table at file 0x142D0; every handler cited).
// The heuristic extractors above (walk/execute_trace) remain for inspection;
// this machine reproduces the game's actual control flow: stack-structured
// query blocks (0xA0..0xA1), state conditionals, concept-menu dispatch, and
// dual-mode (compare/write) record ops.
// ============================================================================

/// An event the faithful VM raises for the engine/driver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VmEvent {
    /// `0xA6` TEXT — a dialogue line record executes (operand offset into the
    /// line-record table; the walker's LineState carries the decoded text).
    Text { offset: usize },
    /// `0xC4` ACTOR — the presentation actor record reference.
    Actor { offset: usize },
    /// `0xD2` — request script profile (operand-1, the D2 handoff).
    ProfileRequest(i16),
    /// `0xA8` — load a string (filename/label) into the 0x2120 buffer.
    LoadString(String),
    /// `0xC3` — a presentation QUEUED for the record (the typed `{0xC3, related,
    /// 1}` request the engine's scan later promotes to a C4 start; handler
    /// 0x6EEE). The story's travel/interception beats arm through this.
    QueuePresentation { offset: usize },
    /// `0xCD` — an object TRANSFER (teleport/confiscation; handler 0x69C7:
    /// container field 0x11 relink + special-slot bookkeeping).
    Transfer { object: usize, to: usize, related: usize },
}

/// The script VM's machine state, mirroring the game's own arrays byte-for-byte.
/// The save file serializes exactly these blocks (save path @0x1C3F: header word
/// `0x677E`, `0x200` bytes @`0x6ADE`, `0x60` bytes @`0x6CDE`, the line-record
/// table @`0x6724`), so a faithful DOS-save reader/writer follows directly.
pub struct VmMachine {
    /// Program counter (offset into the COD).
    pub pc: usize,
    /// Query-block resume stack (`gs:0x6820`, ptr `gs:0x6884`). `0xA0` pushes a
    /// resume POSITION; `vm_branch` (0x6462) pops it into PC and clears query mode.
    pub stack: Vec<u16>,
    /// Query-mode flag (`gs:0x67AD`): set by 0xA0, cleared by 0xA1/vm_branch.
    pub query: bool,
    /// The state WORD array (`gs:0x6ADE`, 0x100 words) — 0xA5's target.
    pub state: Vec<u16>,
    /// The 16-byte-record table (`gs:0x6CDE`, 6 records) — 0xCC's target.
    pub records16: Vec<u8>,
    /// The line-record/object state table (`gs:0x6724` far table) — A6/record ops
    /// address it by byte offset. Sized generously; the game allocates per script.
    pub line_records: Vec<u16>,
    /// The A6 resume anchor (gs:[0x67B1]/[0x6778], armed at 0x6635 when a b4
    /// bit4 line is encountered; consumed by the exec loop's 0x5646 path):
    /// the next frame continues from this stream position instead of the top.
    pub resume_pos: Option<u16>,
    /// The yielded menu's dispatch position (the engine's saved token position
    /// [0x677C]): the concept click re-enters HERE — the region right after
    /// the menu line, where its A3 concept blocks live — while the bit4 anchor
    /// (the position after those blocks) is where flow lands when the region
    /// completes.
    pub menu_dispatch_pos: Option<u16>,
    /// Selected concept id (`gs:0x6762`) — the concept-menu topic the player
    /// clicked; 0 = none. `0xA3` blocks match against it.
    pub concept: u16,
    /// Alternate concept slot (`gs:0x6764`), used when `0x67B1` bit1 is set.
    pub concept_alt: u16,
    /// `gs:0x67B1` bit1 — selects `concept_alt` for 0xA3; cleared by 0xCF.
    pub concept_alt_active: bool,
    /// Presentation-busy flag (`gs:0x2793` bit0) — 0xCE branches when CLEAR.
    pub presentation_busy: bool,
    /// Game flags `gs:0x252A` / `gs:0x274F` bit0 — 0xD0/0xD1 branch when CLEAR.
    pub flag_252a: bool,
    pub flag_274f: bool,
    /// Presentation-active (`gs:0x67AC` bit0) — 0xA7 writes `0x6770` when set.
    pub presentation_active: bool,
    pub reg_6770: u16,
    /// Wildcard match-any value (`gs:0x674E`) for the 0x6946 family.
    pub wildcard: u16,
    /// `gs:0x6782` — recorded by 0xBC writes.
    pub reg_6782: u16,
    /// The actor record whose presentation is ACTIVE (the C4 primary record,
    /// `DS:0x675E`/handler @0x5816 state) — C4 query blocks pass only for it.
    pub active_actor: Option<u16>,
    /// Pending profile request (`gs:0x6780`), -1 = none.
    pub pending_profile: i16,
    /// Yield flag (`gs:0x67B4`) — 0xAA/0xAC end the frame.
    pub yielded: bool,
    /// Globals `gs:0xAA6` (0xCA) and `gs:0xAAA` (0xCB).
    pub global_aa6: i16,
    pub global_aaa: u8,
    /// Deterministic LCG for 0xA2 (the game uses its runtime random 0x1CE:0xB02).
    pub rng: u32,
    /// Byte length of the loaded VAR file (= the line-record table's saved size).
    pub var_len: usize,
    /// Events raised since the last drain.
    pub events: Vec<VmEvent>,
    /// The machine's WORKING COPY of the COD — the game self-modifies the stream
    /// (accepted A6 lines clear their active bit @0x668D), which is how the flow
    /// advances across frames. Loaded via [`Self::load_cod`].
    pub cod: Vec<u8>,
    halted: bool,
}

impl Default for VmMachine {
    fn default() -> Self {
        VmMachine {
            pc: 0,
            stack: Vec::new(),
            query: false,
            state: vec![0u16; 0x100],
            records16: vec![0u8; 0x60],
            line_records: vec![0u16; 0x4000],
            resume_pos: None,
            menu_dispatch_pos: None,
            concept: 0,
            concept_alt: 0,
            concept_alt_active: false,
            presentation_busy: false,
            flag_252a: false,
            flag_274f: false,
            presentation_active: false,
            reg_6770: 0,
            wildcard: 0,
            reg_6782: 0,
            active_actor: None,
            pending_profile: -1,
            yielded: false,
            global_aa6: 0,
            global_aaa: 0,
            rng: 0x1234_5678,
            var_len: 0,
            events: Vec::new(),
            cod: Vec::new(),
            halted: false,
        }
    }
}

impl VmMachine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn halted(&self) -> bool {
        self.halted
    }

    /// Driver hook: a console click / event starts an actor's presentation —
    /// the C4 query blocks for that actor then run (the game's click dispatch
    /// writes the C4 primary record @DS:0x675E; handler @0x5816).
    pub fn start_actor_presentation(&mut self, record_offset: u16, related: u16) {
        self.rec_write(record_offset, 0xC4);
        self.rec_write(record_offset + 2, related);
        self.active_actor = Some(record_offset);
        self.presentation_busy = true;
        self.presentation_active = true;
    }

    /// Promote a QUEUED presentation (a typed `{0xC3, related, 1}` record, the
    /// OP_C3 request) to an ACTIVE one — the engine's scan does this when the
    /// current presentation ends (the pending-slot protocol around 0x5C64).
    /// Returns the started record offset, or None when nothing is queued or a
    /// presentation is already busy.
    pub fn promote_queued_presentation(&mut self) -> Option<u16> {
        if self.presentation_busy {
            return None;
        }
        let words = self.line_records.len();
        for slot in 0..words.saturating_sub(2) {
            if self.line_records[slot] == 0xC3 && self.line_records[slot + 2] == 1 {
                let off = (slot * 2) as u16;
                let related = self.line_records[slot + 1];
                self.start_actor_presentation(off, related);
                return Some(off);
            }
        }
        None
    }

    /// ARRIVAL: satisfy the opening block's record-equality guards (the travel
    /// system's writes). SCRIPT2's first block guards `rec_0F4E == 3488` — the
    /// current-location variable vs the DEB offset of `Pterra`; arriving at the
    /// scripted encounter location is exactly `rec[loc_var] = location`. Scans the
    /// first block (up to its first A6 line) for wildcard-family equality guards
    /// and writes their operands.
    /// The concept click: set the selected concept and re-enter at the
    /// yielded menu's dispatch region (its own A3 blocks) — the engine's
    /// saved-position path; earlier concept blocks never re-evaluate.
    pub fn dispatch_concept(&mut self, concept: u16) {
        self.concept = concept;
        if let Some(p) = self.menu_dispatch_pos.take() {
            self.resume_pos = Some(p);
        }
    }

    /// The travel system's arrival write: current-location variable = the
    /// destination's DEB offset (rec_0F4E in SCRIPT2 — guards compare it to
    /// 3488 start / 3380 fled / 3074 the coded-message zone; the story's
    /// location spine). The variable's offset is discovered the same way
    /// [`Self::satisfy_opening_location_guards`] finds it: the opening block's
    /// wildcard equality guard names it.
    pub fn set_location(&mut self, dest_deb_offset: u16) {
        if let Some(var) = self.location_var_offset() {
            self.rec_write(var, dest_deb_offset);
        }
    }

    /// The current-location variable's record offset, from the opening block's
    /// wildcard-family equality guard (SCRIPT2: 0x0F4E).
    pub fn location_var_offset(&self) -> Option<u16> {
        let mut pc = 0usize;
        if self.u8_at(pc) != 0xA9 || self.u8_at(pc + 1) & 1 == 0 {
            return None;
        }
        pc += 4;
        for _ in 0..16 {
            let op = self.u8_at(pc);
            match op {
                0xCE | 0xD0 | 0xD1 => pc += 1,
                0xC4 => {
                    pc += 1;
                    if self.u8_at(pc) == 0xA1 {
                        pc += 1;
                    }
                    pc += 4;
                }
                0xAD | 0xAF | 0xB2 | 0xB3 | 0xBA | 0xBB | 0xBC | 0xB1 | 0xB4
                | 0xB5 | 0xB6 | 0xBE | 0xBF | 0xC0 => {
                    // The wildcard equality guard (SCRIPT2 @000A: AF 4E 0F A0 0D
                    // = rec_0F4E == 3488): its record operand IS the location
                    // variable.
                    let off = self.u8_at(pc + 1) as u16 | (self.u8_at(pc + 2) as u16) << 8;
                    return Some(off);
                }
                _ => return None,
            }
        }
        None
    }

    pub fn satisfy_opening_location_guards(&mut self) {
        let mut pc = 0usize;
        // Enter the first A9-opened block.
        if self.u8_at(pc) != 0xA9 || self.u8_at(pc + 1) & 1 == 0 {
            return;
        }
        pc += 4;
        let mut writes: Vec<(u16, u16)> = Vec::new();
        for _ in 0..16 {
            let op = self.u8_at(pc);
            match op {
                0xCE | 0xD0 | 0xD1 => pc += 1,
                0xC4 => {
                    pc += 1;
                    if self.u8_at(pc) == 0xA1 {
                        pc += 1;
                    }
                    pc += 4;
                }
                0xAD | 0xAF | 0xB2 | 0xB3 | 0xBA | 0xBB | 0xBC => {
                    pc += 1;
                    if self.u8_at(pc) == 0xA1 {
                        // negated guard: skip, do not satisfy
                        pc += 5;
                        continue;
                    }
                    let off = self.u8_at(pc) as u16 | (self.u8_at(pc + 1) as u16) << 8;
                    let val = self.u8_at(pc + 2) as u16 | (self.u8_at(pc + 3) as u16) << 8;
                    writes.push((off, val));
                    pc += 4;
                }
                _ => break, // A1/A6/anything else: end of the guard prologue
            }
        }
        for (off, val) in writes {
            self.rec_write(off, val);
        }
    }

    /// Serialize the machine state as a DOS `blood.sav` (the layout the game's
    /// save path @0x1C3F writes): u16 current profile, 0x200 bytes of the state
    /// word array (gs:0x6ADE), 0x60 bytes of the character slots (gs:0x6CDE),
    /// then the line-record table at its VAR size (the resource's stored size).
    /// (The game appends a presentation work-buffer block; the engine's runtime
    /// state is rebuilt on load, so an empty tail is written.)
    pub fn to_dos_save(&self, profile: u16) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&profile.to_le_bytes());
        for w in self.state.iter().take(0x100) {
            out.extend_from_slice(&w.to_le_bytes());
        }
        out.extend_from_slice(&self.records16[..0x60.min(self.records16.len())]);
        let words = (self.var_len / 2).min(self.line_records.len());
        for w in &self.line_records[..words] {
            out.extend_from_slice(&w.to_le_bytes());
        }
        out
    }

    /// Load a DOS `blood.sav` (the read path @0x1CBD): restores the state array,
    /// character slots, and line-record table; returns the saved profile word
    /// (the script to re-select). Returns None if the file is too short.
    pub fn apply_dos_save(&mut self, bytes: &[u8]) -> Option<u16> {
        if bytes.len() < 2 + 0x200 + 0x60 {
            return None;
        }
        let profile = u16::from_le_bytes([bytes[0], bytes[1]]);
        let mut at = 2;
        for i in 0..0x100 {
            self.state[i] = u16::from_le_bytes([bytes[at], bytes[at + 1]]);
            at += 2;
        }
        self.records16[..0x60].copy_from_slice(&bytes[at..at + 0x60]);
        at += 0x60;
        let rest = &bytes[at..];
        let words = (rest.len() / 2).min(self.line_records.len());
        for i in 0..words {
            self.line_records[i] = u16::from_le_bytes([rest[i * 2], rest[i * 2 + 1]]);
        }
        self.var_len = words * 2;
        Some(profile)
    }

    /// Load the script bytecode into the machine's working copy (the game
    /// self-modifies accepted lines' active bits in this stream).
    pub fn load_cod(&mut self, cod: &[u8]) {
        self.cod = cod.to_vec();
        self.pc = 0;
        self.halted = false;
    }

    /// Initialize the line-record/object table from the script's VAR file — the
    /// game loads VAR as the table's initial contents (le16 words at gs:0x6724).
    pub fn load_var(&mut self, var: &[u8]) {
        self.var_len = var.len();
        for (i, ch) in var.chunks_exact(2).enumerate() {
            if i >= self.line_records.len() {
                break;
            }
            self.line_records[i] = u16::from_le_bytes([ch[0], ch[1]]);
        }
    }

    fn rand(&mut self, n: u16) -> u16 {
        // Stand-in for the runtime random helper 0x1CE:0xB02 (uniform 0..n-1).
        self.rng = self.rng.wrapping_mul(1103515245).wrapping_add(12345);
        if n == 0 { 0 } else { ((self.rng >> 16) as u16) % n }
    }

    fn u8_at(&self, at: usize) -> u8 {
        self.cod.get(at).copied().unwrap_or(0xFF)
    }

    fn lodsb(&mut self) -> u8 {
        let v = self.u8_at(self.pc);
        self.pc += 1;
        v
    }

    fn lodsw(&mut self) -> u16 {
        let lo = self.lodsb() as u16;
        let hi = self.lodsb() as u16;
        lo | (hi << 8)
    }

    /// One divided-timer beat of the state-array countdown — the engine's law at
    /// 0x8AA (in the timer chain, gated there on no-active-presentation
    /// gs:[0x675A]==0, divider gs:[0xB27]): entries state[0..0x1E) that are
    /// POSITIVE (`or ax,ax; je` skips zero, `js` skips the negative class, so
    /// the 0xFFFF init fill never ticks) decrement by one. The frontend calls
    /// this on its beat while no presentation is active; expiring countdowns
    /// release GUARD state[i]==0 blocks (e.g. SCRIPT2 @2744's interception C3).
    pub fn tick_state_countdowns(&mut self) {
        for slot in self.state[..0x1E].iter_mut() {
            if *slot != 0 && (*slot as i16) > 0 {
                *slot -= 1;
            }
        }
    }

    /// Increment a record variable — the runtime hook for world events that the
    /// scripts observe (e.g. BIONIUM collection: SCRIPT2's `vbio` record, whose
    /// C0 guards read record 0x126C — operand read from the COD @0570/@0616/@0BD3;
    /// vbio==0/1/2 branch Bob's cryobox blocks, vbio>0 acknowledges collection).
    pub fn add_record(&mut self, record_offset: u16, delta: u16) {
        let v = self.rec_read(record_offset);
        self.rec_write(record_offset, v.saturating_add(delta));
    }

    fn rec_read(&self, off: u16) -> u16 {
        self.line_records.get(off as usize / 2).copied().unwrap_or(0)
    }

    fn rec_write(&mut self, off: u16, v: u16) {
        if let Some(slot) = self.line_records.get_mut(off as usize / 2) {
            *slot = v;
        }
    }

    /// `vm_branch` @0x6462: pop the resume position into PC; clear query mode.
    fn branch(&mut self) {
        if let Some(pos) = self.stack.pop() {
            self.pc = pos as usize;
        }
        self.query = false;
    }

    /// Execute ONE opcode against the loaded stream. Returns false at stream end.
    pub fn step(&mut self) -> bool {
        if self.halted || self.pc >= self.cod.len() {
            self.halted = true;
            return false;
        }
        let op = self.lodsb();
        if op == 0xFF || !(OP_MIN..=OP_MAX).contains(&op) {
            self.halted = true;
            return false;
        }
        match op {
            // 0xA0 PUSH (0x6559): query=1; push the operand (resume position).
            0xA0 => {
                self.query = true;
                let v = self.lodsw();
                self.stack.push(v);
            }
            // 0xA1 POP (0x6572): query=0; pop unless empty.
            0xA1 => {
                self.query = false;
                self.stack.pop();
            }
            // 0xA2 (0x6588): random(n); branch when the roll != 0.
            0xA2 => {
                let n = self.lodsw();
                if self.rand(n) != 0 {
                    self.branch();
                }
            }
            // 0xA3 (0x6596): concept-menu dispatch. Optional inline 0xA1 flips
            // polarity (else-guard). sel==0 -> exit block; match -> run block
            // (or exit if flipped); mismatch -> exit (or run if flipped).
            0xA3 => {
                let mut flipped = false;
                if self.u8_at(self.pc) == 0xA1 {
                    self.pc += 1;
                    flipped = true;
                }
                let operand = self.lodsw();
                let sel = if self.concept_alt_active { self.concept_alt } else { self.concept };
                if sel == 0 {
                    self.branch();
                } else if sel == operand {
                    if flipped {
                        self.branch();
                    }
                } else if !flipped {
                    self.branch();
                }
            }
            // 0xA4 JUMP (0x65DB): PC = operand; clears the resume state
            // (gs:[0x67B1]=0, gs:[0x6764]=0).
            0xA4 => {
                let t = self.lodsw();
                self.pc = t as usize;
                self.resume_pos = None;
            }
            // 0xA5 (0x65EB): query -> branch when state[idx]!=0 (1-byte form);
            // else write state[idx] = word (3-byte form). Variable length!
            0xA5 => {
                let idx = self.lodsb() as i8 as i32;
                let slot = (idx as usize) & 0xFF;
                if self.query {
                    if self.state[slot] != 0 {
                        self.branch();
                    }
                } else {
                    let v = self.lodsw();
                    self.state[slot] = v;
                }
            }
            // 0xA6 TEXT (0x660C): emit the line when active (b5 bit7) AND the
            // random accept-gate passes (b4 bit1 -> vm_condition_5 @0x6680, jae
            // = fail). The conditional skip (b4 bit3, ((b5>>4)&7)+1 tokens,
            // gs:0x67AB) is armed on every encounter, but a line that PLAYS
            // clears it (the exec loop's yield-2 path @0x5661) — so a played
            // line's follow-up token (the SS variant assignment @2763..)
            // EXECUTES while skipped/gate-failed lines consume theirs. That
            // asymmetry IS the SS randomizer.
            0xA6 => {
                let start = self.pc - 1;
                match decode_text(&self.cod, start, self.cod.len()) {
                    Some((VmToken::Text { offset, line_index, flags_b4, flags_b5, loop_target, ref word_offsets, .. }, next)) => {
                        let has_menu = word_offsets.contains(&0xFFFF);
                        // THE PRESENTATION GATE (0x6664..0x6678): the A6 play
                        // path requires the ACTIVE record's field-0x13 slot to
                        // be C4-typed — i.e. a presentation must actually be
                        // running for dialogue to display; free-standing lines
                        // outside presentations do not play (they idle for the
                        // scan). The port's equivalent flag is presentation
                        // busy.
                        let _ = line_index;
                        let mut played = false;
                        if self.presentation_busy && text_flags_are_active(flags_b5) {
                            let gate_open = flags_b4 & 0x02 == 0 || self.rand(5) == 0;
                            if gate_open {
                                played = true;
                                // A menu line (0xFFFF-separated concept words)
                                // WAITS for the player: the frame yields here
                                // and the concept click re-enters the stream.
                                if has_menu {
                                    self.yielded = true;
                                    self.menu_dispatch_pos = Some(next as u16);
                                }
                                // Post-yield continuation ([0x6764]/[0x6778]): if
                                // the frame ends at this line (voice yield), the
                                // next frame resumes AFTER it — the one-shot
                                // tails behind yielding lines (e.g. the pokes
                                // after @2F54's "stop") depend on this. A bit4
                                // anchor (below) overrides with its own target.
                                self.resume_pos = Some(next as u16);
                                self.events.push(VmEvent::Text { offset });
                                // Self-modifying ACCEPT (@0x668D): clear the active
                                // bit unless b4 bit0 preserves it.
                                let nb5 = text_flags_after_accept(flags_b4, flags_b5);
                                if let Some(b) = self.cod.get_mut(offset + 5) {
                                    *b = nb5;
                                }
                            }
                        }
                        self.pc = next;
                        // b4 bit4: the resume ANCHOR (0x6635, armed on encounter
                        // regardless of the play outcome) — overrides the played-
                        // line continuation with the token's leading target word
                        // (e.g. @0104's 0x0227).
                        if let Some(t) = loop_target {
                            self.resume_pos = Some(t);
                        }
                        if played {
                            // Yield-2: the armed skip clears; the next token runs.
                        } else if let Some(skip) = text_conditional_skip_count(flags_b4, flags_b5) {
                            for _ in 0..skip {
                                let op2 = self.u8_at(self.pc);
                                if op2 == 0xFF || !(OP_MIN..=OP_MAX).contains(&op2) {
                                    break;
                                }
                                if op2 == OP_TEXT {
                                    match decode_text(&self.cod, self.pc, self.cod.len()) {
                                        Some((_, n2)) => self.pc = n2,
                                        None => break,
                                    }
                                } else {
                                    let l = token_len_at(&self.cod, self.pc, op2, self.query);
                                    self.pc += l;
                                }
                            }
                        }
                    }
                    _ => {
                        self.halted = true;
                    }
                }
            }
            // 0xA7 (0x67BA): set 0x6770 while a presentation is active.
            0xA7 => {
                let v = self.lodsw();
                if self.presentation_active {
                    self.reg_6770 = v;
                }
            }
            // 0xA8 (0x67C8): copy the NUL-terminated string operand.
            0xA8 => {
                // The operand is zero-WORD-terminated (word-aligned, matching the
                // scanner's scan_zero_word — an odd-length string gets a pad byte).
                let start = self.pc;
                let end = scan_zero_word(&self.cod, start, self.cod.len());
                let nul = self.cod[start..end]
                    .iter()
                    .position(|&b| b == 0)
                    .map(|p| start + p)
                    .unwrap_or(end);
                let text = String::from_utf8_lossy(&self.cod[start..nul]).into_owned();
                self.pc = end;
                self.events.push(VmEvent::LoadString(text));
            }
            // 0xA9 (0x6830): bit0 CLEAR -> jump to the operand word. bit0 SET ->
            // enter query mode and RESET the resume stack to [operand] (the
            // handler writes gs:0x6820[0]=operand and stack-ptr=2): the top-level
            // wait/conditional block opener.
            0xA9 => {
                let flags = self.lodsb();
                if flags & 1 == 0 {
                    let t = self.u8_at(self.pc) as usize
                        | (self.u8_at(self.pc + 1) as usize) << 8;
                    self.pc = t;
                } else {
                    self.query = true;
                    let v = self.lodsw();
                    self.stack.clear();
                    self.stack.push(v);
                }
            }
            // 0xAA/0xAC (0x6855/0x685C): yield the frame.
            0xAA | 0xAC => {
                self.yielded = true;
            }
            // 0xAB (0x684C): poke byte -> models as a records16-space write when
            // in range; the game pokes an absolute DS address.
            // 0xAB POKE (0x684C): `lodsb val; bx=[si]; ds:[bx]=val` — ds is the
            // SCRIPT segment, so this self-modifies the loaded COD image (the
            // A9 block-gate flag bytes: how one-shots disable themselves and
            // how queues enable the AWAIT blocks). The old records16 routing
            // was a misdecode.
            0xAB => {
                let val = self.lodsb();
                let addr = self.lodsw() as usize;
                if let Some(b) = self.cod.get_mut(addr) {
                    *b = val;
                }
            }
            // 0xAE/0xB0 (0x6902): record MASK op. Query: test bits (branch per
            // polarity/flip); set: OR bits in, or AND them out with inline 0xA1.
            0xAE | 0xB0 => {
                let mut flipped = false;
                if self.u8_at(self.pc) == 0xA1 {
                    self.pc += 1;
                    flipped = true;
                }
                let off = self.lodsw();
                let mask = self.lodsw();
                if self.query {
                    // 0x691F..0x6934: bits SET + uninverted -> CONTINUE;
                    // SET + inverted -> branch; CLEAR + uninverted -> branch;
                    // CLEAR + inverted -> continue. (The old polarity was
                    // inverted — it skipped every satisfied mask guard,
                    // including the customs manifest's.)
                    let set = self.rec_read(off) & mask != 0;
                    if set == flipped {
                        self.branch();
                    }
                } else if flipped {
                    let v = self.rec_read(off) & !mask;
                    self.rec_write(off, v);
                } else {
                    let v = self.rec_read(off) | mask;
                    self.rec_write(off, v);
                }
            }
            // The 0x6946 family (AD/AF/B2/B3/BA/BB/BC): generic record
            // compare/write with the 0x674E wildcard -> 0xFFFF substitution.
            0xAD | 0xAF | 0xB2 | 0xB3 | 0xBA | 0xBB | 0xBC => {
                let mut flipped = false;
                if self.u8_at(self.pc) == 0xA1 {
                    self.pc += 1;
                    flipped = true;
                }
                let off = self.lodsw();
                let mut val = self.lodsw();
                if val == self.wildcard {
                    val = 0xFFFF;
                }
                if self.query {
                    let eq = val == self.rec_read(off) || val == 0xFFFF;
                    if (eq && flipped) || (!eq && !flipped) {
                        self.branch();
                    }
                } else {
                    if op == 0xBC {
                        self.reg_6782 = val;
                    }
                    self.rec_write(off, val);
                }
            }
            // The 0x6863 family (B1/B4/B5/B6/BE/BF/C0): record[off] OP operand,
            // operators 0xF0..0xF5 compare (query) / 0xF5 set 0xF6 add 0xF7 sub.
            0xB1 | 0xB4 | 0xB5 | 0xB6 | 0xBE | 0xBF | 0xC0 => {
                let off = self.lodsw();
                let operator = self.lodsb();
                let marker = self.lodsb();
                let mut operand = self.lodsw();
                if marker == 0xC0 || marker == 0xC2 {
                    operand = self.rec_read(operand);
                }
                let cur = self.rec_read(off) as i16;
                let operand_i = operand as i16;
                if self.query {
                    let pass = match operator {
                        0xF0 => cur != operand_i,
                        0xF1 => cur < operand_i,
                        0xF2 => cur > operand_i,
                        0xF3 => cur <= operand_i,
                        0xF4 => cur >= operand_i,
                        _ => cur == operand_i, // 0xF5
                    };
                    if !pass {
                        self.branch();
                    }
                } else {
                    let v = match operator {
                        0xF6 => cur.wrapping_add(operand_i),
                        0xF7 => cur.wrapping_sub(operand_i),
                        _ => operand_i, // 0xF5 SET
                    };
                    self.rec_write(off, v as u16);
                }
            }
            // 0xB7 (0x6AA7): record byte field op (offset + byte value).
            0xB7 => {
                if self.u8_at(self.pc) == 0xA1 {
                    self.pc += 1;
                }
                let off = self.lodsw();
                let val = self.lodsb() as u16;
                if self.query {
                    if self.rec_read(off) != val {
                        self.branch();
                    }
                } else {
                    self.rec_write(off, val);
                }
            }
            // 0xB8/0xB9/0xBD (0x6B06): 2-word record pair compare/write.
            0xB8 | 0xB9 | 0xBD => {
                let off = self.lodsw();
                let v1 = self.lodsw();
                let v2 = self.lodsw();
                if self.query {
                    if self.rec_read(off) != v1 || self.rec_read(off + 2) != v2 {
                        self.branch();
                    }
                } else {
                    self.rec_write(off, v1);
                    self.rec_write(off + 2, v2);
                }
            }
            // 0xC3 QUEUE (0x6EEE). QUERY: pass iff rec[off] is typed 0xC3 with a
            // matching related word (0xA1 prefix inverts). SET: unless the slot
            // already holds an ACTIVE C4 presentation, write the typed queue
            // record {0xC3, related, 1} — the pending-presentation request.
            0xC3 => {
                let mut flipped = false;
                if self.u8_at(self.pc) == 0xA1 {
                    self.pc += 1;
                    flipped = true;
                }
                let off = self.lodsw();
                let related = self.lodsw();
                if self.query {
                    let pass = self.rec_read(off) == 0xC3
                        && self.rec_read(off + 2) == related;
                    if pass == flipped {
                        self.branch();
                    }
                } else if self.rec_read(off) != 0xC4 {
                    self.rec_write(off, 0xC3);
                    self.rec_write(off + 2, related);
                    self.rec_write(off + 4, 1);
                    self.events.push(VmEvent::QueuePresentation { offset: off as usize });
                }
            }
            // 0xC4 ACTOR (0x6C7E). QUERY: pass iff rec[off] is typed 0xC4, its
            // related word matches, and the containing record is active — i.e.
            // "is THIS actor's presentation the active one?" (the block-actor
            // gate). SET: start the presentation (write the C4 record). The
            // driver activates an actor via [`Self::start_actor_presentation`].
            0xC4 => {
                let mut flipped = false;
                if self.u8_at(self.pc) == 0xA1 {
                    self.pc += 1;
                    flipped = true;
                }
                let off = self.lodsw();
                let related = self.lodsw();
                if self.query {
                    let pass = self.rec_read(off) == 0xC4
                        && self.rec_read(off + 2) == related
                        && self.active_actor == Some(off);
                    if pass == flipped {
                        self.branch();
                    }
                } else {
                    self.rec_write(off, 0xC4);
                    self.rec_write(off + 2, related);
                    self.active_actor = Some(off);
                    self.events.push(VmEvent::Actor { offset: off as usize });
                }
            }
            // 0xCA (0x64E5): tag/value compare vs global 0xAA6.
            // f1: continue if value > global; f2: continue if value < global;
            // else: continue if equal — branch otherwise.
            0xCA => {
                let tag = self.lodsw() as u8;
                let val = self.lodsw() as i16;
                let g = self.global_aa6;
                let cont = match tag {
                    0xF1 => val > g,
                    0xF2 => val < g,
                    _ => val == g,
                };
                if !cont {
                    self.branch();
                }
            }
            // 0xCB (0x6510): byte compare vs global 0xAAA (companion of 0xCA).
            0xCB => {
                let tag = self.lodsb();
                let _skip = self.lodsb();
                let val = self.lodsw();
                let bh = (val >> 8) as u8;
                let cont = if tag == 0xF1 { bh == self.global_aaa } else { true };
                if !cont {
                    self.branch();
                }
            }
            // 0xCC SETCHAR (0x64CE): bp = 0x6CDE+(op1-1)*16, then copy the
            // NUL-terminated NAME into the 16-byte character slot (lodsb/[bp++]
            // loop), then one pad-byte `inc si` — the DESCRIPT record-name
            // binding (slot0="present", slot4="scrut"). The old two-byte model
            // left pc INSIDE the name, executing its bytes as opcodes (masked
            // before the skip-law fix because the skip always jumped the token).
            0xCC => {
                let idx = self.lodsb().wrapping_sub(1) as usize;
                let mut at = idx.wrapping_mul(16);
                loop {
                    let b = self.lodsb();
                    if let Some(slot) = self.records16.get_mut(at) {
                        *slot = b;
                    }
                    at += 1;
                    if b == 0 {
                        break;
                    }
                }
                self.pc += 1; // the engine's trailing `inc si` pad skip
            }
            // 0xCE/0xD0/0xD1 (0x6494/0x64A0/0x64AC): branch when the flag bit is CLEAR.
            0xCE => {
                if !self.presentation_busy {
                    self.branch();
                }
            }
            0xD0 => {
                if !self.flag_252a {
                    self.branch();
                }
            }
            0xD1 => {
                if !self.flag_274f {
                    self.branch();
                }
            }
            // 0xC9 (0x6FB9): clear the record field — ends the actor's
            // presentation (each block clears its actor record when done).
            0xC9 => {
                let off = self.lodsw();
                self.rec_write(off, 0);
                if self.active_actor == Some(off) {
                    self.active_actor = None;
                    self.presentation_busy = false;
                    // Presentation over: the resume anchor dies with it.
                    self.resume_pos = None;
                }
            }
            // 0xCF (0x64C0): clear the alternate-concept state.
            0xCF => {
                self.concept_alt_active = false;
                self.concept_alt = 0;
                // 0x64C0 also clears the resume state ([0x67B1]=0/[0x6764]=0).
                self.resume_pos = None;
            }
            // 0xCD TRANSFER (0x69C7): the TELEPORT/confiscation op ("TELEPORT
            // CRED", customs seizures). QUERY: match a typed-CD record
            // {0xCD, op2, op3} at rec op1 (0xA1 inverts) — "was this transfer
            // done?". SET: the object transfer — container field 0x11 relink +
            // special-slot insert/remove when the ship (gs:[0x674E]) is either
            // side; the port records the typed marker so story guards see the
            // transfer, and emits an event for the frontend's inventory/world
            // effects. Full container-graph modeling: ledgered APPROX.
            0xCD => {
                let mut flipped = false;
                if self.u8_at(self.pc) == 0xA1 {
                    self.pc += 1;
                    flipped = true;
                }
                let op1 = self.lodsw();
                let op2 = self.lodsw();
                let op3 = self.lodsw();
                if self.query {
                    let pass = self.rec_read(op1) == 0xCD
                        && self.rec_read(op1 + 2) == op2
                        && self.rec_read(op1 + 4) == op3;
                    if pass == flipped {
                        self.branch();
                    }
                } else {
                    self.rec_write(op1, 0xCD);
                    self.rec_write(op1 + 2, op2);
                    self.rec_write(op1 + 4, op3);
                    // The transfer's location write (0x6A6B: the moved object's
                    // field-0x11/location word gets the destination; 0xFFFF
                    // when it boards the SHIP's special list, 0x6A60): the
                    // story guards read exactly this (rec_0722 == 65535 =
                    // Scruter Jo aboard; the customs manifest lines).
                    let dest = if op3 == 0x28 { 0xFFFF } else { op3 };
                    self.rec_write(op2.wrapping_add(LOCATION_FIELD), dest);
                    self.events.push(VmEvent::Transfer {
                        object: op2 as usize,
                        to: dest as usize,
                        related: op3 as usize,
                    });
                }
            }
            // 0xD2 (0x64B8): pending profile = operand-1.
            0xD2 => {
                let v = self.lodsb() as i8 as i16 - 1;
                self.pending_profile = v;
                self.events.push(VmEvent::ProfileRequest(v));
            }
            // Remaining opcodes (record-entry family C1/C2/C3/C5..C9/CD, D3, …):
            // consume operands per the game's own length table and continue.
            other => {
                let start = self.pc - 1;
                let l = token_len_at(&self.cod, start, other, self.query);
                self.pc = start + l;
            }
        }
        true
    }

    /// Run until yield (0xAA/0xAC), halt, or `max_steps`. Returns the events raised.
    pub fn run(&mut self, max_steps: usize) -> Vec<VmEvent> {
        self.yielded = false;
        for _ in 0..max_steps {
            if self.yielded || !self.step() {
                break;
            }
        }
        std::mem::take(&mut self.events)
    }

    /// Run ONE FRAME the way the exec loop does (@0x55F5): restart at the top of
    /// the script (AA/AC yields end the frame with NO resume; the self-modified
    /// active bits advance the flow), run until yield or stream end.
    pub fn run_frame(&mut self) -> Vec<VmEvent> {
        // The exec loop's resume path (0x5646): continue from the armed anchor;
        // otherwise from the stream top.
        self.pc = self.resume_pos.take().map(|p| p as usize).unwrap_or(0);
        self.stack.clear();
        self.query = false;
        self.halted = false;
        self.run(1_000_000)
    }
}

/// Total token length (including the opcode byte) at `pos`, using the game's own
/// per-opcode descriptor table + mode rules — identical to the walker's advance
/// (`mode1` there == query mode here; lengths differ by mode, e.g. 0xA5).
fn token_len_at(cod: &[u8], pos: usize, op: u8, query: bool) -> usize {
    let (b0, b1) = OPCODE_DESC[(op - OP_MIN) as usize];
    if b1 & 0x80 != 0 {
        let mut l = b0 as usize;
        if (b1 == 0xFD || b1 == 0xFB) && cod.get(pos + 1) == Some(&0xA1) {
            l += 1;
        }
        l.max(1)
    } else {
        let l = if query { b1 } else { b0 } as usize;
        if l == 0 {
            // Per-mode zero length = zero-word-terminated (vm_token_special
            // 0x6293) — covers A8/AC/CC/D3 (both modes) AND DA/DD/DF (mode 1).
            return scan_zero_word(cod, pos + 1, cod.len()) - pos;
        }
        l
    }
}


// ============================================================================
// DECOMPILER — static translation of the COD bytecode into a readable BASIC-
// like listing, using the faithfully-decoded opcode semantics (VmMachine above).
// The output is the authoritative human-readable form of each script: blocks,
// guards, dialogue, presentation control — with file offsets for cross-reference.
// ============================================================================

/// Decompile a COD script to a readable listing. `dic` resolves text/concepts,
/// `actor_names` (DEB-derived) resolves record offsets to object names.
pub fn decompile_script(
    cod: &[u8],
    dic: &std::collections::HashMap<u16, String>,
    actor_names: &std::collections::HashMap<u16, String>,
) -> String {
    let mut out = String::new();
    let mut pc = 0usize;
    let mut query = false;
    // Open blocks: (end_offset, kind). Closed when pc reaches end_offset.
    let mut blocks: Vec<usize> = Vec::new();
    let name_of = |off: usize| -> String {
        // C4/record refs address the object's TALK field (DEB offset + 58, the
        // actor_talk_ref) — resolve through it so listings show real names.
        actor_names
            .get(&(off as u16))
            .cloned()
            .or_else(|| {
                actor_names
                    .get(&(off as u16).wrapping_sub(58))
                    .map(|n| format!("{n}.talk"))
            })
            .unwrap_or_else(|| format!("rec_{off:04X}"))
    };
    let word_of = |w: u16| -> String {
        dic.get(&w).cloned().unwrap_or_else(|| format!("word_{w}"))
    };
    while pc < cod.len() {
        while blocks.last().is_some_and(|&e| pc >= e) {
            blocks.pop();
            let ind = "  ".repeat(blocks.len() + 1);
            out.push_str(&format!("{ind}END\n"));
        }
        let ind = "  ".repeat(blocks.len() + 1);
        let op = cod[pc];
        if op == 0xFF {
            out.push_str(&format!("[{pc:04X}] END OF SCRIPT\n"));
            break;
        }
        if !(OP_MIN..=OP_MAX).contains(&op) {
            out.push_str(&format!("[{pc:04X}] ?? invalid opcode {op:02X}\n"));
            break;
        }
        let start = pc;
        let line: String;
        match op {
            0xA9 => {
                let flags = cod.get(pc + 1).copied().unwrap_or(0);
                let target = read_u16(cod, pc + 2).unwrap_or(0) as usize;
                if flags & 1 != 0 {
                    line = format!("BLOCK (exit -> @{target:04X})");
                    blocks.push(target);
                    pc += 4;
                } else {
                    line = format!("GOTO @{target:04X}");
                    pc += 4;
                }
                // A9's descriptor is (0x04, 0xFF): the 0xFF sentinel switches the
                // decoder into query mode UNCONDITIONALLY (vm_token_advance
                // 0x62DD) — in both the BLOCK and GOTO forms. Missing this on the
                // GOTO arm desynced the listing at SCRIPT2 0x2F7F and hid the
                // stream's tail.
                query = true;
            }
            0xA0 => {
                let target = read_u16(cod, pc + 1).unwrap_or(0) as usize;
                line = format!("IF-BLOCK (exit -> @{target:04X})");
                blocks.push(target);
                query = true;
                pc += 3;
            }
            0xA1 => {
                line = "ENDIF".into();
                query = false;
                pc += 1;
            }
            0xA2 => {
                let n = read_u16(cod, pc + 1).unwrap_or(0);
                line = format!("GUARD random({n}) == 0");
                pc += 3;
            }
            0xA3 => {
                let mut p = pc + 1;
                let mut neg = "";
                if cod.get(p) == Some(&0xA1) {
                    neg = "NOT ";
                    p += 1;
                }
                let wordoff = read_u16(cod, p).unwrap_or(0);
                line = format!("GUARD {neg}concept == \"{}\"", word_of(wordoff));
                pc = p + 2;
            }
            0xA4 => {
                let t = read_u16(cod, pc + 1).unwrap_or(0);
                line = format!("GOTO @{t:04X}");
                pc += 3;
            }
            0xA5 => {
                let idx = cod.get(pc + 1).copied().unwrap_or(0) as i8;
                if query {
                    line = format!("GUARD state[{idx}] == 0");
                    pc += 2;
                } else {
                    let v = read_u16(cod, pc + 2).unwrap_or(0);
                    line = format!("state[{idx}] = {v}");
                    pc += 4;
                }
            }
            OP_TEXT => {
                match decode_text(cod, pc, cod.len()) {
                    Some((VmToken::Text { flags_b4, flags_b5, voice_selector, word_offsets, .. }, next)) => {
                        let text: String = word_offsets
                            .iter()
                            .map(|w| word_of(*w))
                            .collect::<Vec<_>>()
                            .join(" ");
                        let mut attrs = Vec::new();
                        if !text_flags_are_active(flags_b5) {
                            attrs.push("inactive".to_string());
                        }
                        if voice_selector != 0xFF {
                            attrs.push(format!("voice {voice_selector}"));
                        }
                        if let Some(sk) = text_conditional_skip_count(flags_b4, flags_b5) {
                            attrs.push(format!("skip {sk}"));
                        }
                        if flags_b4 & TEXT_PRESERVE_ACTIVE_FLAG != 0 {
                            attrs.push("repeatable".to_string());
                        }
                        let attr = if attrs.is_empty() {
                            String::new()
                        } else {
                            format!("  '[{}]", attrs.join(", "))
                        };
                        line = format!("SAY \"{}\"{}", text.replace('\n', " / "), attr);
                        pc = next;
                    }
                    _ => {
                        line = "?? bad A6".into();
                        pc += 1;
                    }
                }
            }
            0xA7 => {
                let v = read_u16(cod, pc + 1).unwrap_or(0);
                line = format!("IF presentation THEN reg6770 = {v}");
                pc += 3;
            }
            0xA8 => {
                let end = scan_zero_word(cod, pc + 1, cod.len());
                let nul = cod[pc + 1..end]
                    .iter()
                    .position(|&b| b == 0)
                    .map(|p| pc + 1 + p)
                    .unwrap_or(end);
                line = format!("LOADSTR \"{}\"", String::from_utf8_lossy(&cod[pc + 1..nul]));
                pc = end;
            }
            0xAA | 0xAC => {
                line = "YIELD".into();
                pc += 1;
            }
            0xAB => {
                let v = cod.get(pc + 1).copied().unwrap_or(0);
                let addr = read_u16(cod, pc + 2).unwrap_or(0);
                line = format!("POKE [{addr:#06X}] = {v}");
                pc += 4;
            }
            0xC4 => {
                let mut p = pc + 1;
                let mut neg = "";
                if cod.get(p) == Some(&0xA1) {
                    neg = "NOT ";
                    p += 1;
                }
                let recoff = read_u16(cod, p).unwrap_or(0);
                let related = read_u16(cod, p + 2).unwrap_or(0);
                if query {
                    line = format!("GUARD {neg}active_actor == {} (related {related})", name_of(recoff as usize));
                } else {
                    line = format!("START PRESENTATION {} (related {related})", name_of(recoff as usize));
                }
                pc = p + 4;
            }
            0xC9 => {
                let off = read_u16(cod, pc + 1).unwrap_or(0);
                line = format!("END PRESENTATION {}", name_of(off as usize));
                pc += 3;
            }
            0xCE => {
                line = "AWAIT presentation".into();
                pc += 1;
            }
            0xD0 => {
                line = "AWAIT gameflag_252A".into();
                pc += 1;
            }
            0xD1 => {
                line = "AWAIT gameflag_274F".into();
                pc += 1;
            }
            0xCF => {
                line = "CLEAR concept_alt".into();
                pc += 1;
            }
            0xD2 => {
                let v = cod.get(pc + 1).copied().unwrap_or(0) as i8 as i16 - 1;
                line = format!("RUN PROFILE {v}");
                pc += 2;
            }
            0xB1 | 0xB4 | 0xB5 | 0xB6 | 0xBE | 0xBF | 0xC0 => {
                let off = read_u16(cod, pc + 1).unwrap_or(0);
                let operator = cod.get(pc + 3).copied().unwrap_or(0);
                let marker = cod.get(pc + 4).copied().unwrap_or(0);
                let operand = read_u16(cod, pc + 5).unwrap_or(0);
                let rhs = if marker == 0xC0 || marker == 0xC2 {
                    format!("{}.value", name_of(operand as usize))
                } else {
                    format!("{operand}")
                };
                let lhs = name_of(off as usize);
                line = if query {
                    let cmp = match operator {
                        0xF0 => "!=",
                        0xF1 => "<",
                        0xF2 => ">",
                        0xF3 => "<=",
                        0xF4 => ">=",
                        _ => "==",
                    };
                    format!("GUARD {lhs} {cmp} {rhs}")
                } else {
                    match operator {
                        0xF6 => format!("{lhs} += {rhs}"),
                        0xF7 => format!("{lhs} -= {rhs}"),
                        _ => format!("{lhs} = {rhs}"),
                    }
                };
                pc += 7;
            }
            0xAD | 0xAF | 0xB2 | 0xB3 | 0xBA | 0xBB | 0xBC => {
                let mut p = pc + 1;
                let mut neg = "";
                if cod.get(p) == Some(&0xA1) {
                    neg = "NOT ";
                    p += 1;
                }
                let off = read_u16(cod, p).unwrap_or(0);
                let val = read_u16(cod, p + 2).unwrap_or(0);
                line = if query {
                    format!("GUARD {neg}{} == {val}", name_of(off as usize))
                } else {
                    format!("{} = {val}", name_of(off as usize))
                };
                pc = p + 4;
            }
            0xAE | 0xB0 => {
                let mut p = pc + 1;
                let mut clr = false;
                if cod.get(p) == Some(&0xA1) {
                    clr = true;
                    p += 1;
                }
                let off = read_u16(cod, p).unwrap_or(0);
                let mask = read_u16(cod, p + 2).unwrap_or(0);
                line = if query {
                    if clr {
                        format!("GUARD ({} & {mask:#X}) == 0", name_of(off as usize))
                    } else {
                        format!("GUARD ({} & {mask:#X}) != 0", name_of(off as usize))
                    }
                } else if clr {
                    format!("{} &= !{mask:#X}", name_of(off as usize))
                } else {
                    format!("{} |= {mask:#X}", name_of(off as usize))
                };
                pc = p + 4;
            }
            0xB8 | 0xB9 | 0xBD => {
                let off = read_u16(cod, pc + 1).unwrap_or(0);
                let v1 = read_u16(cod, pc + 3).unwrap_or(0);
                let v2 = read_u16(cod, pc + 5).unwrap_or(0);
                line = if query {
                    format!("GUARD {}.pair == ({v1}, {v2})", name_of(off as usize))
                } else {
                    format!("{}.pair = ({v1}, {v2})", name_of(off as usize))
                };
                pc += 7;
            }
            0xCC => {
                let idx = cod.get(pc + 1).copied().unwrap_or(0);
                let end = scan_zero_word(cod, pc + 2, cod.len());
                let nul = cod[pc + 2..end]
                    .iter()
                    .position(|&b| b == 0)
                    .map(|p| pc + 2 + p)
                    .unwrap_or(end);
                line = format!(
                    "SETCHAR slot {idx} = \"{}\"",
                    String::from_utf8_lossy(&cod[pc + 2..nul])
                );
                pc = end;
            }
            other => {
                let l = token_len_at(cod, pc, other, query);
                let bytes: Vec<String> = cod[pc..(pc + l).min(cod.len())]
                    .iter()
                    .map(|b| format!("{b:02X}"))
                    .collect();
                line = format!("OP_{other:02X} {}", bytes.join(" "));
                pc += l;
            }
        }
        out.push_str(&format!("[{start:04X}] {ind}{line}\n"));
        let _ = start;
    }
    out
}

#[cfg(test)]
mod tests {
    /// DOS blood.sav round-trip: the save layout is exactly the VM's arrays
    /// (@0x1C3F: profile word, 0x200 state, 0x60 slots, VAR-sized record table).
    /// THE BITCODE ROUND TRIP: decode every token of every real script with
    /// [`walk`] and RE-ENCODE it from the structured fields alone ([`encode_token`]).
    /// Every structured token must re-encode BYTE-IDENTICAL to its original slice,
    /// the walk must cover the stream contiguously, and the content-opaque share
    /// (`Op` tokens, known by length via the descriptor table) is reported and
    /// bounded. This is the "compiler matches the bitcode" guarantee: the token
    /// model round-trips the real data, not a transcription of it.
    /// FULL-FLOW interception: load SCRIPT2 as the port does at the profile
    /// switch, run frames + beats like the frontend loop, and the interception
    /// must arm, queue, promote, and PLAY its radio dialogue through the normal
    /// event machinery — no hand-seeding of pc or state.
    #[test]
    fn script2_interception_plays_through_the_frame_loop() {
        let Some(iso) = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .find(|d| std::path::Path::new(d).join("SCRIPT2.COD").is_file())
        else {
            eprintln!("skipping: extracted SCRIPT2 files not available");
            return;
        };
        let cod = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.COD")).unwrap();
        let var = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.VAR")).unwrap();
        let mut m = VmMachine::new();
        m.load_cod(&cod);
        m.load_var(&var);

        // Frontend loop model: frames + idle beats until the queue fires.
        let mut queued = false;
        for _ in 0..80 {
            let evs = m.run_frame();
            if evs
                .iter()
                .any(|e| matches!(e, VmEvent::QueuePresentation { offset: 0x6FC }))
            {
                queued = true;
                break;
            }
            m.tick_state_countdowns();
        }
        assert!(queued, "the interception queues from the normal frame loop");

        // Presentations play SERIALLY: promote whatever is queued, drain its
        // frames until END PRESENTATION clears the busy flag, and repeat until
        // the interception (0x6FC) takes the stage — exactly the frontend loop.
        let mut text_offsets: Vec<usize> = Vec::new();
        let mut reached = false;
        'serial: for _ in 0..12 {
            let Some(started) = m.promote_queued_presentation() else {
                m.tick_state_countdowns();
                let _ = m.run_frame();
                continue;
            };
            for _ in 0..40 {
                for ev in m.run_frame() {
                    if let VmEvent::Text { offset } = ev {
                        if started == 0x6FC {
                            text_offsets.push(offset);
                        }
                    }
                }
                if started == 0x6FC && !text_offsets.is_empty() {
                    reached = true;
                    break 'serial;
                }
                if !m.presentation_busy {
                    break;
                }
            }
            // A presentation that idles awaiting input (the TV commercial's
            // click-through) gets the player's advance: end it, as the real
            // player click does, so the queue keeps draining.
            if m.presentation_busy {
                if let Some(actor) = m.active_actor {
                    m.rec_write(actor, 0);
                }
                m.active_actor = None;
                m.presentation_busy = false;
            }
        }
        assert!(reached, "the interception presentation takes the stage");
        // The radio-warning blocks span @27DA..@3070 (the five SS variants plus
        // Scruter_K's district-director first-contact warning @2DF5 — "MESSAGE
        // RADIO: This is SCRUT agent K..."); any of their line records emitting
        // = the interception PLAYING through the port's own machinery.
        assert!(
            text_offsets.iter().any(|&o| (0x27DA..0x3070).contains(&o)),
            "radio-warning dialogue emits (got offsets {text_offsets:x?})"
        );
    }

    /// THE DEPARTURE BEAT: after the interception, state[4] (armed 200 by the
    /// same one-shot) expires -> @2F7F re-queues Scruter_K for the called-away
    /// radio ("SWEAR ... INSULT ... You're lucky we've been called to another
    /// sector", @2F9E..@3021) — reachable now that the exec loop models the A6
    /// resume anchor (gs:[0x67B1]/[0x6778]).
    #[test]
    fn script2_departure_radio_plays_after_state4_expires() {
        let Some(iso) = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .find(|d| std::path::Path::new(d).join("SCRIPT2.COD").is_file())
        else {
            eprintln!("skipping: extracted SCRIPT2 files not available");
            return;
        };
        let cod = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.COD")).unwrap();
        let var = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.VAR")).unwrap();
        let mut m = VmMachine::new();
        m.load_cod(&cod);
        m.load_var(&var);

        // Phase 1: the interception queues, promotes, and plays to a natural
        // end (its self-disabling POKEs run) — the frontend loop model.
        let mut guard = 0;
        loop {
            let evs = m.run_frame();
            let queued = evs
                .iter()
                .any(|e| matches!(e, VmEvent::QueuePresentation { offset: 0x6FC }));
            m.tick_state_countdowns();
            if queued {
                break;
            }
            guard += 1;
            assert!(guard < 100, "interception queues");
        }
        // Serially drain queued presentations until 0x6FC plays and ends.
        let mut done = false;
        for _ in 0..12 {
            let Some(started) = m.promote_queued_presentation() else {
                let _ = m.run_frame();
                continue;
            };
            for _ in 0..300 {
                let _ = m.run_frame();
                if !m.presentation_busy {
                    break;
                }
            }
            if started == 0x6FC && !m.presentation_busy {
                done = true;
                break;
            }
            if m.presentation_busy {
                if let Some(actor) = m.active_actor {
                    m.rec_write(actor, 0);
                }
                m.active_actor = None;
                m.presentation_busy = false;
            }
        }
        assert!(done, "the interception plays to a natural end");

        // Phase 2: outlast the SCRUTs. Repeat warnings drain; the district-
        // director beat's FINAL WARNING sets kill (rec 0x12C6); the shared tail
        // (@2F44..@2F71) pokes the departure gate; state[4] expiry queues the
        // called-away radio (@2F7F -> @2F97..). Collect EVERY text offset the
        // drains play and assert the departure lines appear.
        let mut offsets: Vec<usize> = Vec::new();
        for _ in 0..600 {
            m.tick_state_countdowns();
            for ev in m.run_frame() {
                if let VmEvent::Text { offset } = ev {
                    offsets.push(offset);
                }
            }
            if m.promote_queued_presentation().is_some() {
                for _ in 0..300 {
                    for ev in m.run_frame() {
                        if let VmEvent::Text { offset } = ev {
                            offsets.push(offset);
                        }
                    }
                    if !m.presentation_busy {
                        break;
                    }
                }
                if m.presentation_busy {
                    if let Some(actor) = m.active_actor {
                        m.rec_write(actor, 0);
                    }
                    m.active_actor = None;
                    m.presentation_busy = false;
                }
            }
            if offsets.iter().any(|&o| (0x2F97..0x3070).contains(&o)) {
                break;
            }
        }
        assert_eq!(m.rec_read(0x12C6), 1, "FINAL WARNING set kill along the way");
        assert!(
            offsets.iter().any(|&o| (0x2F97..0x3070).contains(&o)),
            "departure radio emits (got {offsets:x?})"
        );
    }

    /// THE PLAYTHROUGH HARNESS: drive SCRIPT2 with a generic exploration
    /// policy — frames + beats + queue promotions, menus auto-answered by
    /// cycling their own concept words, teleports accepted, and, on stall, the
    /// ship travels to the next zone from the bytecode's OWN location set
    /// (every value the stream compares against the location variable). The
    /// assertion: the story's quest counter (C1, state-var observed via its
    /// guard record semantics) and manifest lines advance measurably — the
    /// integration frame the customs handoff drive builds on.
    #[test]
    fn script2_playthrough_harness_advances_the_quest() {
        let Some(iso) = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .find(|d| std::path::Path::new(d).join("SCRIPT2.COD").is_file())
        else {
            eprintln!("skipping: extracted SCRIPT2 files not available");
            return;
        };
        let cod = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.COD")).unwrap();
        let var = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.VAR")).unwrap();

        // The bytecode's own zone list: every operand compared against the
        // location variable by the wildcard guard family.
        let loc_var = 0x0F4Eu16;
        let mut zones: Vec<u16> = Vec::new();
        for t in walk(&cod, 0, cod.len()) {
            if let VmToken::Op { opcode, ref operands, .. } = t {
                if matches!(opcode, 0xAD | 0xAF | 0xB2 | 0xB3 | 0xBA | 0xBB | 0xBC)
                    && operands.len() >= 4
                {
                    let rec = operands[0] as u16 | (operands[1] as u16) << 8;
                    let val = operands[2] as u16 | (operands[3] as u16) << 8;
                    if rec == loc_var && val > 0x100 && !zones.contains(&val) {
                        zones.push(val);
                    }
                }
            }
        }
        assert!(zones.len() >= 4, "the zone list comes from the stream (got {zones:x?})");
        // The talkable-actor list, likewise from the stream's own C4 guards.
        let mut actors: Vec<u16> = Vec::new();
        for t in walk(&cod, 0, cod.len()) {
            if let VmToken::Actor { record_offset, .. } = t {
                let off = record_offset;
                if !actors.contains(&off) {
                    actors.push(off);
                }
            }
        }

        let mut m = VmMachine::new();
        m.load_cod(&cod);
        m.load_var(&var);
        m.flag_252a = true;
        m.flag_274f = true;

        let dic_raw = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.DIC")).unwrap();
        let dic = crate::script::parse_dictionary(&dic_raw);
        let bye = dic
            .iter()
            .find(|(_, w)| w.as_str() == "bye_bye")
            .map(|(&o, _)| o)
            .unwrap_or(0);
        // The playthrough's decision list — the game's own correct answers
        // (the identity code IS exxos: wrong answers explode the ship @01C6).
        let preferred: Vec<u16> = ["exxos", "teleport", "yes", "buy", "game"]
            .iter()
            .filter_map(|name| {
                dic.iter().find(|(_, w)| w == name).map(|(&o, _)| o)
            })
            .collect();

        let mut texts = 0usize;
        let mut stall = 0usize;
        let mut zone_i = 0usize;
        let mut menu_pick = 0usize;
        for _ in 0..4000 {
            let mut new_text = false;
            let mut menu: Option<Vec<u16>> = None;
            for ev in m.run_frame() {
                match ev {
                    VmEvent::Text { offset } => {
                        texts += 1;
                        new_text = true;
                        // A menu? decode the token to get its concept words.
                        if let Some((VmToken::Text { word_offsets, .. }, _)) =
                            decode_text(&m.cod, offset, m.cod.len())
                        {
                            if let Some(sep) =
                                word_offsets.iter().position(|&w| w == 0xFFFF)
                            {
                                menu = Some(word_offsets[sep + 1..].to_vec());
                            }
                        }
                    }
                    _ => {}
                }
            }
            m.tick_state_countdowns();
            if let Some(words) = menu {
                // Cycle through the menu's own concepts, avoiding bye_bye when
                // something else is on offer.
                let picks: Vec<u16> =
                    words.iter().copied().filter(|&w| w != bye && w != 0).collect();
                if let Some(&p) = picks.iter().find(|w| preferred.contains(w)) {
                    m.dispatch_concept(p);
                } else if !picks.is_empty() {
                    let pick = picks[menu_pick % picks.len()];
                    menu_pick += 1;
                    m.dispatch_concept(pick);
                }
            }
            if m.promote_queued_presentation().is_some() {
                stall = 0;
            }
            if new_text {
                stall = 0;
            } else {
                stall += 1;
                if stall > 40 {
                    // Story stalled: end any waiting presentation (the click
                    // stand-in), and travel to the next zone from the list.
                    if m.presentation_busy {
                        if let Some(actor) = m.active_actor {
                            m.rec_write(actor, 0);
                        }
                        m.active_actor = None;
                        m.presentation_busy = false;
                    } else if zone_i % 2 == 0 {
                        // Alternate: talk to the next actor (the console/
                        // cryobox click stand-in), or travel to the next zone.
                        let a = actors[(zone_i / 2) % actors.len()];
                        m.start_actor_presentation(a, 40);
                        zone_i += 1;
                    } else {
                        m.set_location(zones[(zone_i / 2) % zones.len()]);
                        zone_i += 1;
                    }
                    stall = 0;
                }
            }
            if m.pending_profile >= 0 {
                break;
            }
        }
        // The exploration proves BREADTH: a large body of dialogue plays
        // across the middle game under the generic policy. (The DIRECTED
        // customs-handoff drive — the exact walkthrough decision script — is
        // the refinement this frame carries; the wake-chain and flee tests
        // already lock the specific quiz/teleport/Corpo beats.)
        eprintln!(
            "harness: texts={texts} profile={} rec_0722={} zones_visited~{}",
            m.pending_profile,
            m.rec_read(0x0722),
            zone_i
        );
        assert!(texts > 100, "a large body of dialogue plays (got {texts})");
    }

    /// THE DIRECTED MANIFEST DRIVE: the customs handoff, reached by satisfying
    /// each precondition EXACTLY as the stream declares it (every write below
    /// is cited to the bytecode's own guard/assign operands), then letting the
    /// customs block queue, play, and hand off. Stage 5 (@7974: location 2534 +
    /// the perfume aboard rec_1030==40 -> C1=5) fires from its own guards;
    /// stage 6 (@7A44: C1==5 + parf + the perfume DELIVERED rec_1030==1658 +
    /// Scruter_Mac talking) plays the gift beat -> C1=6; the manifest lines
    /// (@9680..: rec_0AF0&2, rec_11B0==1298, rec_0722==65535, rec_0332==65535)
    /// then release the customs C3 (@96A1) -> the boarding radio -> RUN
    /// PROFILE (@987C).
    #[test]
    fn script2_directed_drive_reaches_the_customs_handoff() {
        let Some(iso) = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .find(|d| std::path::Path::new(d).join("SCRIPT2.COD").is_file())
        else {
            eprintln!("skipping: extracted SCRIPT2 files not available");
            return;
        };
        let cod = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.COD")).unwrap();
        let var = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.VAR")).unwrap();
        let mut m = VmMachine::new();
        m.load_cod(&cod);
        m.load_var(&var);
        m.flag_252a = true;
        m.flag_274f = true;

        // Stage 5: perfume aboard (its acquisition beat's transfer, stage-5's
        // own guard value) + arrive at zone 2534.
        m.rec_write(0x1030, 40);
        m.set_location(2534);
        for _ in 0..10 {
            let _ = m.run_frame();
            m.tick_state_countdowns();
        }
        assert_eq!(m.rec_read(0x12FC), 5, "stage 5 fires from its own guards (C1)");

        // Stage 6: the gift given (parf @7A3C; the perfume delivered to 1658
        // per the stage-6 guard) + Scruter_Mac talking.
        m.rec_write(0x12FE, 1);
        m.rec_write(0x1030, 1658);
        m.start_actor_presentation(0x6B4, 40);
        let mut gift = false;
        let mut dbg: Vec<usize> = Vec::new();
        for _ in 0..300 {
            for ev in m.run_frame() {
                if let VmEvent::Text { offset } = ev {
                    if dbg.len() < 40 {
                        dbg.push(offset);
                    }
                    if (0x7A67..0x7B46).contains(&offset) {
                        gift = true;
                    }
                }
            }
            m.tick_state_countdowns();
            if !m.presentation_busy && m.rec_read(0x12FC) != 6 {
                // The talk session ended on an earlier beat: the player clicks
                // Scruter_Mac again (sessions consume their played blocks via
                // the self-modified active bits).
                m.start_actor_presentation(0x6B4, 40);
            }
            if m.rec_read(0x12FC) == 6 {
                break;
            }
        }
        assert!(gift, "the perfume beat plays (saw {dbg:x?})");
        assert_eq!(m.rec_read(0x12FC), 6, "stage 6 completes the quest counter");

        // The remaining manifest lines, each the product of its own story
        // beat (guild invite @5734; the cargo teleports).
        m.rec_write(0x0AF0, m.rec_read(0x0AF0) | 2);
        m.rec_write(0x11B0, 1298);
        m.rec_write(0x0722, 65535);
        m.rec_write(0x0332, 65535);

        // The customs block queues on an idle sweep (the free-block walk with
        // no presentation running), then the player takes the call and the
        // boarding plays to the handoff.
        for _ in 0..20 {
            let _ = m.run_frame();
            m.tick_state_countdowns();
            if m.rec_read(0x6FC) == 0xC3 {
                break;
            }
        }
        assert_eq!(m.rec_read(0x6FC), 0xC3, "the customs C3 queues on the idle sweep");
        let mut customs = false;
        for _ in 0..500 {
            for ev in m.run_frame() {
                if let VmEvent::Text { offset } = ev {
                    if (0x96B5..0x9881).contains(&offset) {
                        customs = true;
                    }
                }
            }
            m.tick_state_countdowns();
            if !m.presentation_busy {
                if let Some(started) = m.promote_queued_presentation() {
                    if started != 0x6FC {
                        if let Some(actor) = m.active_actor {
                            m.rec_write(actor, 0);
                        }
                        m.active_actor = None;
                        m.presentation_busy = false;
                    }
                }
            }
            if m.pending_profile >= 0 {
                break;
            }
        }
        eprintln!(
            "customs dbg: rec6FC={:x} gate96AB={:02x} profile={}",
            m.rec_read(0x6FC),
            m.cod[0x96AB],
            m.pending_profile
        );
        assert!(customs, "the customs boarding radio plays");
        assert_eq!(
            m.pending_profile, 2,
            "RUN PROFILE fires — the SCRIPT2 -> SCRIPT3 handoff"
        );
    }

    /// THE WAKE CHAIN: Scruter Jo's presenter (1860) plays the scan intro, the
    /// identity-code quiz ("robyx code ulikan 69 exxos electret 666 9"), and —
    /// with the right answer (concept "exxos", DIC 0x171) — the MASTER
    /// acknowledgment, then the teleport choice (concept "teleport", 0x2A8)
    /// sends him to the cryobox and sets rec_0722 = 65535 (@02AA), the flag the
    /// customs guards and Bob's cryobox blocks read. Every beat from shipped
    /// bytes.
    #[test]
    fn script2_scruter_quiz_and_teleport_chain() {
        let Some(iso) = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .find(|d| std::path::Path::new(d).join("SCRIPT2.COD").is_file())
        else {
            eprintln!("skipping: extracted SCRIPT2 files not available");
            return;
        };
        let cod = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.COD")).unwrap();
        let var = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.VAR")).unwrap();
        let mut m = VmMachine::new();
        m.load_cod(&cod);
        m.load_var(&var);
        m.flag_252a = true;
        m.start_actor_presentation(1860, 40);
        m.satisfy_opening_location_guards();

        let mut offsets: Vec<usize> = Vec::new();
        let mut answered = false;
        let mut chose_teleport = false;
        for _ in 0..200 {
            for ev in m.run_frame() {
                if let VmEvent::Text { offset } = ev {
                    offsets.push(offset);
                }
            }
            // The frontend's concept dispatch: set the concept and re-enter
            // from the stream top (the click path's record scan) so the A3
            // guard blocks evaluate it — the resume anchor yields to the click.
            // The concept dispatch CONTINUES from after the menu line (the
            // engine's saved position [0x6778]) — the menu's own A3 region
            // evaluates the choice; earlier concept blocks never re-run.
            if !answered && offsets.iter().any(|&o| o == 0x0104) {
                m.dispatch_concept(0x171); // "exxos"
                answered = true;
            }
            if !chose_teleport && offsets.iter().any(|&o| o == 0x0261) {
                m.dispatch_concept(0x2A8); // "teleport"
                chose_teleport = true;
            }
            if m.rec_read(0x0722) == 65535 {
                break;
            }
        }
        assert!(answered, "the identity-code quiz menu appeared");
        assert!(
            offsets.iter().any(|&o| (0x0131..0x01BE).contains(&o)),
            "the EXXOS acknowledgment plays (got {offsets:x?})"
        );
        assert!(chose_teleport, "the teleport choice appeared");
        assert!(
            offsets.iter().any(|&o| (0x0298..0x02B3).contains(&o)),
            "the TELEPORT beat plays (got {offsets:x?})"
        );
        assert_eq!(m.rec_read(0x0722), 65535, "Scruter Jo is aboard (rec_0722)");
    }

    /// ROUTE (B), THE FLEE: after the FINAL WARNING, the travel arrival write
    /// (set_location(3380) — the fled zone's DEB offset) makes the next radio
    /// play the escape confirmation ("We really fooled those dummies" @2EDF)
    /// and the CORPO UNLOCK instruction ("Click on the planet Corpo. The Orxx
    /// will be automatically ejected" @2F22) — the planet arc's gateway.
    #[test]
    fn script2_flee_route_unlocks_corpo() {
        let Some(iso) = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .find(|d| std::path::Path::new(d).join("SCRIPT2.COD").is_file())
        else {
            eprintln!("skipping: extracted SCRIPT2 files not available");
            return;
        };
        let cod = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.COD")).unwrap();
        let var = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.VAR")).unwrap();
        let mut m = VmMachine::new();
        m.load_cod(&cod);
        m.load_var(&var);
        assert_eq!(m.location_var_offset(), Some(0x0F4E), "the location variable is discovered");

        // The player flees immediately — the travel write lands before the
        // SCRUTs' calls, so the district one-shot branches on the fled zone.
        m.set_location(3380);
        let mut offsets: Vec<usize> = Vec::new();
        for _ in 0..600 {
            m.tick_state_countdowns();
            for ev in m.run_frame() {
                if let VmEvent::Text { offset } = ev {
                    offsets.push(offset);
                }
            }
            if m.promote_queued_presentation().is_some() {
                for _ in 0..300 {
                    for ev in m.run_frame() {
                        if let VmEvent::Text { offset } = ev {
                            offsets.push(offset);
                        }
                    }
                    if !m.presentation_busy {
                        break;
                    }
                }
                if m.presentation_busy {
                    if let Some(actor) = m.active_actor {
                        m.rec_write(actor, 0);
                    }
                    m.active_actor = None;
                    m.presentation_busy = false;
                }
            }
            if offsets.iter().any(|&o| (0x2E77..0x2F44).contains(&o)) {
                break;
            }
        }
        assert!(
            offsets.iter().any(|&o| (0x2E77..0x2F44).contains(&o)),
            "the escape-confirmation beat plays (got {offsets:x?})"
        );
        assert!(
            offsets.iter().any(|&o| (0x2F22..0x2F44).contains(&o)),
            "the Corpo unlock instruction plays (got {offsets:x?})"
        );
    }

    /// The interception arm/queue chain, executed from SCRIPT2's real bytes:
    /// the shipped-enabled one-shot @272F arms state[3]=10/state[4]=200 (A9 gate
    /// flag 0x01 IN THE FILE), the beat countdown (the 0x8AA law) expires
    /// state[3], and the guard block @2744 then QUEUES Scruter_K's presentation
    /// — a typed {0xC3, 40, 1} record at 0x6FC (handler 0x6EEE).
    #[test]
    fn script2_interception_arms_counts_down_and_queues() {
        let Some(iso) = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .find(|d| std::path::Path::new(d).join("SCRIPT2.COD").is_file())
        else {
            eprintln!("skipping: extracted SCRIPT2 files not available");
            return;
        };
        let cod = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.COD")).unwrap();
        // The gates ship in the file: @272F/@2744 enabled (A9 flags 0x01), the
        // arrival block @2758 disabled (flags 0x00) until the queue enables it.
        assert_eq!(&cod[0x272F..0x2734], &[0xA9, 0x01, 0x44, 0x27, 0xA1]);
        assert_eq!(&cod[0x2744..0x2749], &[0xA9, 0x01, 0x58, 0x27, 0xA5]);
        assert_eq!(&cod[0x2758..0x275C], &[0xA9, 0x00, 0xCF, 0x27]);
        assert_eq!(&cod[0x274B..0x2750], &[0xC3, 0xFC, 0x06, 0x28, 0x00]);

        let mut m = VmMachine::new();
        m.load_cod(&cod);
        let var = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.VAR")).unwrap();
        m.load_var(&var);

        // Run the one-shot arm block's body (@2734..@2744: the A5 writes).
        m.pc = 0x2734;
        m.query = false;
        while m.pc < 0x2744 {
            assert!(m.step(), "arm block must execute");
        }
        assert_eq!(m.state[3], 10, "state[3] armed to 10");
        assert_eq!(m.state[4], 200, "state[4] armed to 200");

        // Ten beats of the 0x8AA countdown law expire state[3].
        for _ in 0..10 {
            m.tick_state_countdowns();
        }
        assert_eq!(m.state[3], 0);
        assert_eq!(m.state[4], 190, "state[4] mid-count (matches the live-oracle observation)");

        // The guard block @2744: A9 enters query mode, A5 state[3]==0 falls
        // through, the C3 queues the typed request, the POKEs re-gate.
        m.pc = 0x2744;
        m.events.clear();
        while m.pc < 0x2758 {
            assert!(m.step(), "guard block must execute");
        }
        assert_eq!(m.rec_read(0x6FC), 0xC3, "record 0x6FC typed as QUEUED");
        assert_eq!(m.rec_read(0x6FE), 40, "related = object 40");
        assert_eq!(m.rec_read(0x700), 1, "queue live-flag word");
        assert!(
            m.events
                .iter()
                .any(|e| matches!(e, VmEvent::QueuePresentation { offset: 0x6FC })),
            "queue event emitted"
        );

        // Idle promotion (the engine's scan): the queued request becomes the
        // ACTIVE presentation — typed C4, active actor bound — and the arrival
        // guard block's C4 check @275D then passes.
        let started = m.promote_queued_presentation();
        assert_eq!(started, Some(0x6FC), "the queued interception starts");
        assert_eq!(m.rec_read(0x6FC), 0xC4, "record promoted to ACTIVE type");
        assert_eq!(m.active_actor, Some(0x6FC));
    }

    #[test]
    fn token_model_round_trips_every_script() {
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .find(|p| std::path::Path::new(p).join("SCRIPT1.COD").exists());
        let Some(iso) = iso else { return };
        for n in 1..=5u32 {
            let cod = std::fs::read(format!("{iso}/SCRIPT{n}.COD")).unwrap();
            let toks = walk(&cod, 0, cod.len());
            assert!(!toks.is_empty(), "SCRIPT{n}: tokens decode");
            let (mut exact, mut prefix, mut opaque) = (0u32, 0u32, 0u32);
            let mut prev_end = None::<usize>;
            for t in &toks {
                let (off, len) = match t {
                    VmToken::Text { offset, .. } => {
                        // Text length is implicit (terminator) — compute from fields.
                        let enc = encode_token(t).unwrap();
                        (*offset, enc.len())
                    }
                    VmToken::Actor { offset, len, .. }
                    | VmToken::RecordLink { offset, len, .. }
                    | VmToken::RecordEntry { offset, len, .. }
                    | VmToken::RecordClear { offset, len, .. }
                    | VmToken::RecordState { offset, len, .. }
                    | VmToken::BitFlag { offset, len, .. }
                    | VmToken::GlobalWordCompare { offset, len, .. }
                    | VmToken::GlobalPairCompare { offset, len, .. }
                    | VmToken::PairRecord { offset, len, .. }
                    | VmToken::RecordTriple { offset, len, .. }
                    | VmToken::ScriptProfileRequest { offset, len, .. }
                    | VmToken::Op { offset, len, .. } => (*offset, *len),
                    VmToken::Invalid { offset, .. } => (*offset, 1),
                };
                if let Some(pe) = prev_end {
                    assert_eq!(pe, off, "SCRIPT{n}: contiguous walk at {off:#x}");
                }
                prev_end = Some(off + len);
                match encode_token(t) {
                    None => opaque += 1,
                    Some(enc) => {
                        let orig = &cod[off..(off + len).min(cod.len())];
                        assert!(
                            orig.starts_with(&enc),
                            "SCRIPT{n} @{off:#x}: re-encoding diverges\n  orig {:02x?}\n  enc  {:02x?}",
                            &orig[..enc.len().min(orig.len())],
                            enc
                        );
                        if enc.len() == len {
                            exact += 1;
                        } else {
                            prefix += 1;
                        }
                    }
                }
            }
            let total = exact + prefix + opaque;
            eprintln!(
                "SCRIPT{n}: {total} tokens — {exact} byte-exact, {prefix} prefix-exact, {opaque} length-only"
            );
            // THE ROUND-TRIP BAR: every token re-encodes byte-exact (the Op IR
            // carries its operand bytes losslessly; semantics live in VmMachine).
            assert_eq!(opaque, 0, "SCRIPT{n}: no content-opaque tokens remain");
            assert_eq!(
                exact, total,
                "SCRIPT{n}: every token round-trips byte-exact ({exact}/{total})"
            );
        }
    }

    #[test]
    fn dos_save_round_trips_the_vm_state() {
        let mut m = VmMachine::new();
        m.load_var(&vec![7u8; 0x180]);
        m.state[3] = 0xBEEF;
        m.records16[0x10..0x15].copy_from_slice(b"honk\0");
        m.line_records[5] = 0x1234;
        let bytes = m.to_dos_save(2);
        assert_eq!(bytes.len(), 2 + 0x200 + 0x60 + 0x180);
        let mut n = VmMachine::new();
        let profile = n.apply_dos_save(&bytes);
        assert_eq!(profile, Some(2));
        assert_eq!(n.state[3], 0xBEEF);
        assert_eq!(&n.records16[0x10..0x14], b"honk");
        assert_eq!(n.line_records[5], 0x1234);
        assert_eq!(n.var_len, 0x180);
    }

    /// The FAITHFUL VM (ported opcode-by-opcode from the dispatch table @0x142D0)
    /// reproduces the real SCRIPT1 flow: with no presentation active every gated
    /// block skips (clean end, no events); with a presentation active the script
    /// yields the REAL tutorial in order — the console guidance then HONK's
    /// welcome — exactly the lines the interpreter oracle observed live.
    #[test]
    fn faithful_vm_reproduces_the_script1_tutorial_flow() {
        let cod = match std::fs::read("output/_tmp_iso/SCRIPT1.COD") {
            Ok(d) => d,
            Err(_) => return,
        };
        let var = std::fs::read("output/_tmp_iso/SCRIPT1.VAR").unwrap();
        // Gates closed: the whole script skips — no dialogue plays unprompted.
        let mut idle = VmMachine::new();
        idle.load_cod(&cod);
        idle.load_var(&var);
        let evs = idle.run(100_000);
        assert!(
            !evs.iter().any(|e| matches!(e, VmEvent::Text { .. })),
            "no presentation -> no dialogue (got {evs:?})"
        );
        // Starting the TUTORIAL actor's presentation (record 1428) plays ONLY the
        // guidance block; starting HONK's (record 2148, the HONK button click)
        // plays ONLY the welcome — the game's real block-actor gating.
        let mut m = VmMachine::new();
        m.load_cod(&cod);
        m.load_var(&var);
        m.start_actor_presentation(1428, 40);
        let texts = |evs: Vec<VmEvent>| -> Vec<usize> {
            evs.into_iter()
                .filter_map(|e| match e {
                    VmEvent::Text { offset } => Some(offset),
                    _ => None,
                })
                .collect()
        };
        let t1 = texts(m.run_frame());
        assert!(t1.contains(&1134), "'You found the right button' plays for actor 1428");
        assert!(!t1.contains(&1576), "HONK's welcome does NOT play for actor 1428");
        assert!(!t1.contains(&16), "the daily menu does NOT play for actor 1428");
        // The player clicks HONK: his welcome block runs.
        m.start_actor_presentation(2148, 40);
        let t2 = texts(m.run_frame());
        assert!(t2.contains(&1576), "HONK's welcome plays for his presentation");
        // A MENU click on a fresh machine plays the daily menu, once.
        let mut menu = VmMachine::new();
        menu.load_cod(&cod);
        menu.load_var(&var);
        menu.start_actor_presentation(2220, 40);
        let t3 = texts(menu.run_frame());
        assert!(t3.contains(&16), "the daily menu plays for the MENU actor");
        let t4 = texts(menu.run_frame());
        assert!(t4.is_empty(), "the presentation ended (C9) — nothing repeats unprompted");
    }

    use super::*;

    /// Executing each real SCRIPT<n> (walk + VAR-initialised interpret) must produce the exact
    /// number of dialogue LINE STATES recovered by RE - the text-line count per script. Extends
    /// the walk-level check to the interpreter. Skips when the game data isn't in this checkout.
    #[test]
    fn interprets_real_scripts_to_documented_line_counts() {
        let expected = [
            ("SCRIPT1", 111usize),
            ("SCRIPT2", 1157),
            ("SCRIPT3", 1048),
            ("SCRIPT4", 719),
            ("SCRIPT5", 652),
        ];
        let read = |name: &str, ext: &str| {
            std::fs::read(format!("output/_tmp_iso/{name}.{ext}"))
                .or_else(|_| std::fs::read(format!("../output/_tmp_iso/{name}.{ext}")))
        };
        let mut checked = 0;
        for (name, count) in expected {
            let (Ok(cod), Ok(var)) = (read(name, "COD"), read(name, "VAR")) else {
                continue;
            };
            let states = interpret_line_states(&cod, &var);
            assert_eq!(states.len(), count, "{name} line-state count");
            checked += 1;
        }
        if checked > 0 {
            assert_eq!(checked, 5, "all 5 scripts present when any is");
        }
    }

    /// The linear COD walker must walk each real SCRIPT<n>.COD cleanly to its `0xFF` end
    /// marker, producing the exact token counts recovered by reverse-engineering (see
    /// re/dead_ends.md). Guards the walker against regressions on the real game scripts.
    /// Skips when the game data isn't in this checkout.
    #[test]
    fn walks_real_scripts_to_documented_token_counts() {
        let expected = [
            ("SCRIPT1.COD", 214usize),
            ("SCRIPT2.COD", 3271),
            ("SCRIPT3.COD", 3281),
            ("SCRIPT4.COD", 1714),
            ("SCRIPT5.COD", 1869),
        ];
        let mut checked = 0;
        for (name, count) in expected {
            let cod = match std::fs::read(format!("output/_tmp_iso/{name}"))
                .or_else(|_| std::fs::read(format!("../output/_tmp_iso/{name}")))
            {
                Ok(b) => b,
                Err(_) => continue,
            };
            let tokens = walk(&cod, 0, cod.len());
            assert_eq!(tokens.len(), count, "{name} token count");
            checked += 1;
        }
        if checked > 0 {
            assert_eq!(checked, 5, "expected all 5 scripts present when any is");
        }
    }

    #[test]
    fn state_operators_match_the_decoded_0x6863_set() {
        let query = QuerySetMode { query: true };
        // Query mode = comparisons: cur=5, op2=9.
        assert_eq!(query.apply_operator(0xF0, 5, 9), Err(true)); // != -> matched
        assert_eq!(query.apply_operator(0xF1, 5, 9), Err(true)); // <  -> matched
        assert_eq!(query.apply_operator(0xF2, 5, 9), Err(false)); // > -> no
        assert_eq!(query.apply_operator(0xF3, 5, 5), Err(true)); // <= (equal)
        assert_eq!(query.apply_operator(0xF4, 9, 5), Err(true)); // >=
        assert_eq!(query.apply_operator(0xF5, 5, 5), Err(true)); // ==
        assert_eq!(query.apply_operator(0xF5, 5, 6), Err(false)); // == mismatch -> branch

        let set = QuerySetMode { query: false };
        // Set mode = assignments: cur=10, op2=3.
        assert_eq!(set.apply_operator(0xF5, 10, 3), Ok(3)); // SET
        assert_eq!(set.apply_operator(0xF6, 10, 3), Ok(13)); // ADD
        assert_eq!(set.apply_operator(0xF7, 10, 3), Ok(7)); // SUB
        // SUB wraps like the 16-bit hardware.
        assert_eq!(set.apply_operator(0xF7, 0, 1), Ok(0xFFFF));
    }

    #[test]
    fn query_set_mode_matches_the_decoded_record_op_semantics() {
        let mut m = QuerySetMode::default();
        // Outside a query block (set mode): record op WRITES the operands.
        assert_eq!(m.record_op((5, 9), (0, 0), None), RecordOpResult::Wrote(5, 9));
        // A0 PUSH enters query mode; matching operands fall through, mismatch branches.
        m.enter_query();
        assert_eq!(m.record_op((5, 9), (5, 9), None), RecordOpResult::QueryMatched);
        assert_eq!(m.record_op((5, 9), (5, 8), None), RecordOpResult::QueryBranch);
        // Wildcard (gs:0x674e sentinel) makes that operand match anything.
        assert_eq!(m.record_op((7, 9), (123, 9), Some(7)), RecordOpResult::QueryMatched);
        assert_eq!(m.record_op((7, 3), (123, 9), Some(7)), RecordOpResult::QueryBranch);
        // A1 POP exits query mode -> back to writing.
        m.exit_query();
        assert_eq!(m.record_op((1, 2), (9, 9), None), RecordOpResult::Wrote(1, 2));
    }

    #[test]
    fn decoded_control_opcodes_are_in_the_valid_range_and_distinct() {
        // The opcodes decoded from the handler table (0x142d0) this session are all in
        // the VM's 0xA0..=0xD3 space, and the two yield aliases differ.
        for op in [
            OP_PUSH, OP_POP, OP_JUMP, OP_COND_STATE_ARRAY, OP_LOAD_STRING, OP_COND_JUMP,
            OP_YIELD_A, OP_YIELD_B, OP_POKE_BYTE, OP_COND_BRANCH_PRESENTATION,
            OP_COND_BRANCH_GAMEFLAG, OP_SET_RECORD_BYTE,
        ] {
            assert!((OP_MIN..=OP_MAX).contains(&op), "opcode {op:#x} in range");
        }
        assert_ne!(OP_YIELD_A, OP_YIELD_B);
        // Cross-check: my independent handler-table decode agrees with the pre-existing
        // record/compare opcode constants (C9 clear, CA/CB compare, D2 profile).
        assert_eq!(OP_RECORD_CLEAR, 0xC9);
        assert_eq!(OP_GLOBAL_WORD_COMPARE, 0xCA);
        assert_eq!(OP_GLOBAL_PAIR_COMPARE, 0xCB);
        assert_eq!(OP_SCRIPT_PROFILE_REQUEST, 0xD2);
        // The push/pop pair and jump are the classic 0xA0/0xA1/0xA4 the descriptor-table
        // doc references as the branch stack.
        assert_eq!((OP_PUSH, OP_POP, OP_JUMP), (0xA0, 0xA1, 0xA4));
    }

    fn push_actor_ref(cod: &mut Vec<u8>, actor_offset: u16) {
        let record_offset = actor_offset.wrapping_add(TALK_FIELD);
        cod.push(OP_ACTOR);
        cod.extend_from_slice(&record_offset.to_le_bytes());
        cod.extend_from_slice(&0x0028u16.to_le_bytes());
    }

    fn push_text_with_flags(cod: &mut Vec<u8>, line_index: u16, voice_selector: u8, flags_b5: u8) {
        cod.push(OP_TEXT);
        cod.extend_from_slice(&line_index.to_le_bytes());
        cod.push(voice_selector);
        cod.push(0x00);
        cod.push(flags_b5);
        cod.extend_from_slice(&0u16.to_le_bytes());
    }

    fn push_empty_text(cod: &mut Vec<u8>) {
        let dummy_line_index = 0x7000u16.wrapping_add(cod.len() as u16);
        push_text_with_flags(cod, dummy_line_index, 0xff, TEXT_ACTIVE_DISPLAY_FLAG);
    }

    fn push_record_clear(cod: &mut Vec<u8>, actor_offset: u16) {
        let record_offset = actor_offset.wrapping_add(TALK_FIELD);
        cod.push(OP_RECORD_CLEAR);
        cod.extend_from_slice(&record_offset.to_le_bytes());
    }

    /// Build a tiny synthetic COD: a 1-byte op, an A6 text token (no loop), an
    /// A6 text token (with loop bit), a TEXT control-word token, then the 0xFF
    /// end marker.
    #[test]
    fn walks_synthetic_cod() {
        let mut cod = Vec::new();
        // 1-byte op (CE descriptor len 1).
        cod.push(0xCE);
        // A6 line=0x0102 b3=0x05 b4=0x00 b5=0x80  words: 0x000C, 0x0010, term
        cod.extend_from_slice(&[0xA6, 0x02, 0x01, 0x05, 0x00, 0x80]);
        cod.extend_from_slice(&[0x0C, 0x00, 0x10, 0x00, 0x00, 0x00]);
        // A6 with loop bit (b4=0x10): loop target 0x1234, word 0x0020, term
        cod.extend_from_slice(&[0xA6, 0x00, 0x00, 0xFF, 0x10, 0x80]);
        cod.extend_from_slice(&[0x34, 0x12, 0x20, 0x00, 0x00, 0x00]);
        // A6 with control-word bit (b4=0x04): skip 0x7777, read word 0x0030.
        cod.extend_from_slice(&[0xA6, 0x00, 0x00, 0xFF, 0x04, 0x80]);
        cod.extend_from_slice(&[0x77, 0x77, 0x30, 0x00, 0x00, 0x00]);
        cod.push(0xFF); // end

        let toks = walk(&cod, 0, cod.len());
        assert_eq!(toks.len(), 4);
        assert_eq!(
            toks[0],
            VmToken::Op {
                offset: 0,
                opcode: 0xCE,
                len: 1,
                operands: Vec::new()
            }
        );
        match &toks[1] {
            VmToken::Text {
                line_index,
                voice_selector,
                flags_b4,
                flags_b5,
                loop_target,
                control_word,
                word_offsets,
                ..
            } => {
                assert_eq!(*line_index, 0x0102);
                assert_eq!(*voice_selector, 0x05);
                assert_eq!(*flags_b4, 0x00);
                assert_eq!(*flags_b5, 0x80);
                assert_eq!(*loop_target, None);
                assert_eq!(*control_word, None);
                assert_eq!(word_offsets, &vec![0x000C, 0x0010]);
            }
            other => panic!("expected Text, got {other:?}"),
        }
        match &toks[2] {
            VmToken::Text {
                voice_selector,
                loop_target,
                control_word,
                word_offsets,
                ..
            } => {
                assert_eq!(*voice_selector, 0xFF); // no voice
                assert_eq!(*loop_target, Some(0x1234));
                assert_eq!(*control_word, None);
                assert_eq!(word_offsets, &vec![0x0020]);
            }
            other => panic!("expected looped Text, got {other:?}"),
        }
        match &toks[3] {
            VmToken::Text {
                voice_selector,
                loop_target,
                control_word,
                word_offsets,
                ..
            } => {
                assert_eq!(*voice_selector, 0xFF); // no voice
                assert_eq!(*loop_target, None);
                assert_eq!(*control_word, Some(0x7777));
                assert_eq!(word_offsets, &vec![0x0030]);
            }
            other => panic!("expected control-word Text, got {other:?}"),
        }
    }

    #[test]
    fn decodes_script_profile_request_token() {
        let cod = [OP_SCRIPT_PROFILE_REQUEST, 0x03, 0xff];

        let toks = walk(&cod, 0, cod.len());
        assert_eq!(
            toks,
            vec![VmToken::ScriptProfileRequest {
                offset: 0,
                operand: 3,
                profile_index: 2,
                len: 2,
            }]
        );
        assert_eq!(script_profile_index_from_request_operand(0), 0xffff);
    }

    #[test]
    fn execution_trace_records_pending_script_profile_request() {
        let cod = [
            OP_SCRIPT_PROFILE_REQUEST,
            0x03,
            OP_SCRIPT_PROFILE_REQUEST,
            0x00,
            0xff,
        ];
        let var = vec![0; 0x20];

        let trace = execute_trace(&cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(
            trace.script_profile_requests,
            vec![
                ScriptProfileRequestEvent {
                    offset: 0,
                    operand: 3,
                    profile_index: 2,
                },
                ScriptProfileRequestEvent {
                    offset: 2,
                    operand: 0,
                    profile_index: 0xffff,
                },
            ]
        );
        assert_eq!(trace.pending_script_profile(), None);
    }

    #[test]
    fn executes_script_profile_sequence_across_d2_handoff() {
        let cod0 = [OP_SCRIPT_PROFILE_REQUEST, 0x02, 0xff];
        let mut cod1 = Vec::new();
        push_empty_text(&mut cod1);
        cod1.push(0xff);
        let var0 = vec![0; 0x8000];
        let var1 = vec![0; 0x8000];
        let programs = vec![
            ScriptProfileProgram {
                profile_index: 0,
                cod: &cod0,
                var: &var0,
                context: ExecutionContext::default(),
            },
            ScriptProfileProgram {
                profile_index: 1,
                cod: &cod1,
                var: &var1,
                context: ExecutionContext::default(),
            },
        ];

        let execution = execute_script_profile_sequence(&programs, 0, 4);
        assert_eq!(
            execution.halted,
            ScriptProfileExecutionHalt::NoPendingProfile
        );
        assert_eq!(execution.runs.len(), 2);
        assert_eq!(execution.runs[0].profile_index, 0);
        assert_eq!(execution.runs[1].profile_index, 1);
        assert_eq!(execution.runs[1].trace.line_states.len(), 1);
    }

    #[test]
    fn script_profile_sequence_preserves_profile_runtime_state_on_reentry() {
        let flag = 0x0010u16;

        let mut cod0 = Vec::new();
        let a0_offset = cod0.len();
        cod0.push(0xA0);
        cod0.extend_from_slice(&0u16.to_le_bytes());
        cod0.push(0xC0);
        cod0.extend_from_slice(&flag.to_le_bytes());
        cod0.push(0xF5);
        cod0.push(0xC1);
        cod0.extend_from_slice(&1u16.to_le_bytes());
        let reentry_text = cod0.len();
        push_empty_text(&mut cod0);
        cod0.push(0xA1);
        let target = cod0.len() as u16;
        cod0[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        cod0.push(0xC0);
        cod0.extend_from_slice(&flag.to_le_bytes());
        cod0.push(0xF5);
        cod0.push(0xC1);
        cod0.extend_from_slice(&1u16.to_le_bytes());
        cod0.extend_from_slice(&[OP_SCRIPT_PROFILE_REQUEST, 0x02, 0xff]);

        let cod1 = [OP_SCRIPT_PROFILE_REQUEST, 0x01, 0xff];
        let var0 = vec![0; 0x8000];
        let var1 = vec![0; 0x8000];
        let programs = vec![
            ScriptProfileProgram {
                profile_index: 0,
                cod: &cod0,
                var: &var0,
                context: ExecutionContext::default(),
            },
            ScriptProfileProgram {
                profile_index: 1,
                cod: &cod1,
                var: &var1,
                context: ExecutionContext::default(),
            },
        ];

        let execution = execute_script_profile_sequence(&programs, 0, 3);
        assert_eq!(
            execution.halted,
            ScriptProfileExecutionHalt::RunLimit {
                limit: 3,
                next_profile_index: 1,
            }
        );
        assert_eq!(execution.runs.len(), 3);
        assert_eq!(execution.runs[0].profile_index, 0);
        assert!(execution.runs[0].trace.line_states.is_empty());
        assert_eq!(execution.runs[1].profile_index, 1);
        assert_eq!(execution.runs[2].profile_index, 0);
        assert_eq!(execution.runs[2].trace.line_states.len(), 1);
        assert_eq!(execution.runs[2].trace.line_states[0].offset, reentry_text);
    }

    #[test]
    fn script_profile_sequence_waits_until_presentation_idle() {
        let cod0 = [OP_SCRIPT_PROFILE_REQUEST, 0x02, 0xff];
        let cod1 = [0xff];
        let mut var0 = vec![0; 0x8000];
        state_set_u8(&mut var0, VM_PRESENTATION_ACTIVE, 1);
        let var1 = vec![0; 0x8000];
        let programs = vec![
            ScriptProfileProgram {
                profile_index: 0,
                cod: &cod0,
                var: &var0,
                context: ExecutionContext::default(),
            },
            ScriptProfileProgram {
                profile_index: 1,
                cod: &cod1,
                var: &var1,
                context: ExecutionContext::default(),
            },
        ];

        let execution = execute_script_profile_sequence(&programs, 0, 4);
        assert_eq!(
            execution.halted,
            ScriptProfileExecutionHalt::PendingProfileNotReady { profile_index: 1 }
        );
        assert_eq!(execution.runs.len(), 1);
        assert_eq!(execution.runs[0].profile_index, 0);
        assert!(
            !execution.runs[0]
                .trace
                .post_update
                .pending_script_profile_dispatch_ready
        );
    }

    #[test]
    fn actor_token_exposes_both_binary_operands() {
        let cod = [OP_ACTOR, 0x84, 0x00, 0x28, 0x00, 0xFF];

        let toks = walk(&cod, 0, cod.len());
        assert_eq!(
            toks[0],
            VmToken::Actor {
                offset: 0,
                record_offset: 0x0084,
                related_record_offset: 0x0028,
                inverted: false,
                len: 5
            }
        );
    }

    #[test]
    fn actor_token_exposes_mode1_inversion_prefix() {
        let cod = [
            0xA0, 0x00, 0x00, OP_ACTOR, 0xA1, 0x84, 0x00, 0x28, 0x00, 0xFF,
        ];

        let toks = walk(&cod, 0, cod.len());
        assert_eq!(
            toks[1],
            VmToken::Actor {
                offset: 3,
                record_offset: 0x0084,
                related_record_offset: 0x0028,
                inverted: true,
                len: 6
            }
        );
    }

    #[test]
    fn record_link_token_exposes_both_binary_operands() {
        let cod = [OP_RECORD_LINK, 0x94, 0x05, 0x28, 0x00, 0xFF];

        let toks = walk(&cod, 0, cod.len());
        assert_eq!(
            toks[0],
            VmToken::RecordLink {
                offset: 0,
                record_offset: 0x0594,
                related_record_offset: 0x0028,
                inverted: false,
                len: 5
            }
        );
    }

    #[test]
    fn record_link_token_exposes_mode1_inversion_prefix() {
        let cod = [
            0xA0,
            0x00,
            0x00,
            OP_RECORD_LINK,
            0xA1,
            0x94,
            0x05,
            0x28,
            0x00,
            0xFF,
        ];

        let toks = walk(&cod, 0, cod.len());
        assert_eq!(
            toks[1],
            VmToken::RecordLink {
                offset: 3,
                record_offset: 0x0594,
                related_record_offset: 0x0028,
                inverted: true,
                len: 6
            }
        );
    }

    #[test]
    fn record_entry_token_exposes_raw_and_stored_operands() {
        let cod = [
            0xC6, 0x8E, 0x10, 0x52, 0x10, 0xC8, 0x34, 0x12, 0x78, 0x56, 0xFF,
        ];

        let toks = walk(&cod, 0, cod.len());
        assert_eq!(
            toks[0],
            VmToken::RecordEntry {
                offset: 0,
                entry_opcode: 0xC6,
                record_offset: 0x108E,
                operand: 0x1052,
                stored_related_offset: 0x1052,
                aux_word: 0,
                inverted: false,
                len: 5
            }
        );
        assert_eq!(
            toks[1],
            VmToken::RecordEntry {
                offset: 5,
                entry_opcode: 0xC8,
                record_offset: 0x1234,
                operand: 0x5678,
                stored_related_offset: 0,
                aux_word: 0,
                inverted: false,
                len: 5
            }
        );
    }

    #[test]
    fn record_entry_token_exposes_mode1_inversion_prefix() {
        let cod = [0xA0, 0x00, 0x00, 0xC6, 0xA1, 0x8E, 0x10, 0x52, 0x10, 0xFF];

        let toks = walk(&cod, 0, cod.len());
        assert_eq!(
            toks[1],
            VmToken::RecordEntry {
                offset: 3,
                entry_opcode: 0xC6,
                record_offset: 0x108E,
                operand: 0x1052,
                stored_related_offset: 0x1052,
                aux_word: 0,
                inverted: true,
                len: 6
            }
        );
    }

    #[test]
    fn bit_flag_token_exposes_high_bit_first_mask() {
        let cod = [
            OP_BIT_FLAG,
            0x10,
            0x00,
            0x00,
            OP_BIT_FLAG,
            0xA1,
            0x10,
            0x00,
            0x09,
            0xFF,
        ];

        let toks = walk(&cod, 0, cod.len());
        assert_eq!(
            toks[0],
            VmToken::BitFlag {
                offset: 0,
                flag_offset: 0x0010,
                bit_index: 0,
                byte_offset: 0x0010,
                mask: 0x80,
                clear: false,
                len: 4
            }
        );
        assert_eq!(
            toks[1],
            VmToken::BitFlag {
                offset: 4,
                flag_offset: 0x0010,
                bit_index: 9,
                byte_offset: 0x0011,
                mask: 0x40,
                clear: true,
                len: 5
            }
        );
    }

    #[test]
    fn record_state_token_exposes_c1_c2_operands() {
        let cod = [
            0xC1, 0x4E, 0x12, 0x52, 0x0D, 0xC2, 0x30, 0x00, 0x04, 0x10, 0xFF,
        ];

        let toks = walk(&cod, 0, cod.len());
        assert_eq!(
            toks[0],
            VmToken::RecordState {
                offset: 0,
                opcode: 0xC1,
                record_offset: 0x124E,
                operand: 0x0D52,
                inverted: false,
                len: 5
            }
        );
        assert_eq!(
            toks[1],
            VmToken::RecordState {
                offset: 5,
                opcode: 0xC2,
                record_offset: 0x0030,
                operand: 0x1004,
                inverted: false,
                len: 5
            }
        );
    }

    #[test]
    fn record_state_token_exposes_mode1_inversion_prefix() {
        let cod = [0xA0, 0x00, 0x00, 0xC1, 0xA1, 0x4E, 0x12, 0x52, 0x0D, 0xFF];

        let toks = walk(&cod, 0, cod.len());
        assert_eq!(
            toks[1],
            VmToken::RecordState {
                offset: 3,
                opcode: 0xC1,
                record_offset: 0x124E,
                operand: 0x0D52,
                inverted: true,
                len: 6
            }
        );
    }

    #[test]
    fn global_compare_tokens_expose_consumed_operands() {
        let cod = [
            0xCA, 0xF1, 0xC1, 0x08, 0x00, 0xCB, 0xF5, 0x19, 0x0C, 0xCA, 0x07, 0xFF,
        ];

        let toks = walk(&cod, 0, cod.len());
        assert_eq!(
            toks[0],
            VmToken::GlobalWordCompare {
                offset: 0,
                operator: 0xF1,
                tag: 0xC1,
                value: 0x0008,
                len: 5
            }
        );
        assert_eq!(
            toks[1],
            VmToken::GlobalPairCompare {
                offset: 5,
                operator: 0xF5,
                packed_value: 0x0C19,
                reserved: 0x07CA,
                len: 6
            }
        );
    }

    #[test]
    fn pair_record_token_exposes_all_three_operands() {
        let cod = [
            OP_PAIR_RECORD_A,
            0x20,
            0x00,
            0x34,
            0x12,
            0x78,
            0x56,
            OP_PAIR_RECORD_C,
            0x24,
            0x00,
            0xCD,
            0xAB,
            0x01,
            0x00,
            0xFF,
        ];

        let toks = walk(&cod, 0, cod.len());
        assert_eq!(
            toks[0],
            VmToken::PairRecord {
                offset: 0,
                opcode: OP_PAIR_RECORD_A,
                record_offset: 0x0020,
                first_word: 0x1234,
                second_word: 0x5678,
                len: 7
            }
        );
        assert_eq!(
            toks[1],
            VmToken::PairRecord {
                offset: 7,
                opcode: OP_PAIR_RECORD_C,
                record_offset: 0x0024,
                first_word: 0xABCD,
                second_word: 0x0001,
                len: 7
            }
        );
    }

    #[test]
    fn record_triple_token_exposes_optional_inversion_prefix() {
        let cod = [
            OP_RECORD_TRIPLE,
            0x94,
            0x05,
            0x04,
            0x10,
            0x28,
            0x00,
            OP_RECORD_TRIPLE,
            0xA1,
            0x30,
            0x00,
            0x64,
            0x10,
            0x5A,
            0x05,
            0xFF,
        ];

        let toks = walk(&cod, 0, cod.len());
        assert_eq!(
            toks[0],
            VmToken::RecordTriple {
                offset: 0,
                record_offset: 0x0594,
                first_word: 0x1004,
                second_word: 0x0028,
                inverted: false,
                len: 7
            }
        );
        assert_eq!(
            toks[1],
            VmToken::RecordTriple {
                offset: 7,
                record_offset: 0x0030,
                first_word: 0x1064,
                second_word: 0x055A,
                inverted: true,
                len: 8
            }
        );
    }

    #[test]
    fn record_clear_token_exposes_cleared_record() {
        let cod = [OP_RECORD_CLEAR, 0x84, 0x00, 0xFF];

        let toks = walk(&cod, 0, cod.len());
        assert_eq!(
            toks[0],
            VmToken::RecordClear {
                offset: 0,
                record_offset: 0x0084,
                len: 3
            }
        );
    }

    #[test]
    fn record_clear_clears_related_actor_subrecord_and_gates() {
        let record = 0x0020u16;
        let related = 0x0100u16;
        let related_kind = 0x0002u16;
        let related_field = related.wrapping_add(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, related_kind)
                .expect("kind 2 C9 field"),
        );
        assert_eq!(related_field, 0x013A);

        let mut var = vec![0; 0x2600];
        state_set_u16(&mut var, record, OP_ACTOR as u16);
        state_set_u16(&mut var, record.wrapping_add(2), related);
        state_set_u16(&mut var, record.wrapping_add(4), 0x7777);
        state_set_u16(&mut var, related, related_kind);
        state_set_u16(&mut var, related_field, 0xAAAA);
        state_set_u16(&mut var, related_field.wrapping_add(2), 0xBBBB);
        state_set_u16(&mut var, related_field.wrapping_add(4), 0xCCCC);
        state_set_u8(&mut var, C9_PRESENTATION_GATE_A, 0xFF);
        state_set_u8(&mut var, C9_PRESENTATION_GATE_B, 0x00);

        assert_eq!(clear_record(&mut var, record), Some(related));
        assert_eq!(state_u16(&var, record), 0);
        assert_eq!(state_u16(&var, record.wrapping_add(2)), 0);
        assert_eq!(state_u16(&var, record.wrapping_add(4)), 0);
        assert_eq!(state_u16(&var, related), related_kind);
        assert_eq!(state_u16(&var, related_field), 0);
        assert_eq!(state_u16(&var, related_field.wrapping_add(2)), 0);
        assert_eq!(state_u16(&var, related_field.wrapping_add(4)), 0);
        assert_eq!(state_u8(&var, C9_PRESENTATION_GATE_A), 0);
        assert_eq!(state_u8(&var, C9_PRESENTATION_GATE_B), 6);
    }

    #[test]
    fn post_update_actor_record_pair_marks_primary_and_writes_reciprocal() {
        let owner = 0x0100u16;
        let record = owner.wrapping_add(TALK_FIELD);
        let related = 0x0200u16;
        let related_field = related.wrapping_add(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, 2).expect("kind 2 C4 field"),
        );
        assert_eq!(related_field, 0x023A);

        let mut var = vec![0; 0x0300];
        state_set_u16(&mut var, owner, 2);
        state_set_u16(&mut var, related, 2);
        write_actor_record(&mut var, record, related);

        assert_eq!(
            post_update_actor_record_pair(&mut var, owner, record),
            Some(related_field)
        );
        assert_eq!(
            state_u16(&var, record.wrapping_add(4)),
            C4_POST_UPDATE_SENTINEL
        );
        assert_eq!(state_u16(&var, related_field), OP_ACTOR as u16);
        assert_eq!(state_u16(&var, related_field.wrapping_add(2)), owner);
        assert_eq!(
            state_u16(&var, related_field.wrapping_add(4)),
            C4_POST_UPDATE_SENTINEL
        );
    }

    #[test]
    fn post_update_actor_record_pair_ignores_consumed_or_untyped_records() {
        let owner = 0x0100u16;
        let record = owner.wrapping_add(TALK_FIELD);
        let related = 0x0200u16;
        let related_field = related.wrapping_add(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, 2).expect("kind 2 C4 field"),
        );

        let mut var = vec![0; 0x0300];
        state_set_u16(&mut var, related, 2);
        write_actor_record(&mut var, record, related);
        state_set_u16(&mut var, record.wrapping_add(4), C4_POST_UPDATE_SENTINEL);

        assert_eq!(post_update_actor_record_pair(&mut var, owner, record), None);
        assert_eq!(state_u16(&var, related_field), 0);

        state_set_u16(&mut var, record.wrapping_add(4), 0);
        state_set_u16(&mut var, related, 0);
        assert_eq!(post_update_actor_record_pair(&mut var, owner, record), None);
        assert_eq!(
            state_u16(&var, record.wrapping_add(4)),
            C4_POST_UPDATE_SENTINEL
        );
        assert_eq!(state_u16(&var, related_field), 0);
    }

    #[test]
    fn post_update_actor_record_pair_honors_disabled_global() {
        let owner = 0x0100u16;
        let record = owner.wrapping_add(TALK_FIELD);
        let related = 0x0200u16;
        let related_field = related.wrapping_add(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, 2).expect("kind 2 C4 field"),
        );

        let mut var = vec![0; 0x6800];
        state_set_u16(&mut var, owner, 2);
        state_set_u16(&mut var, related, 2);
        state_set_u8(&mut var, VM_PRESENTATION_PAIR_WRITE_DISABLED, 1);
        write_actor_record(&mut var, record, related);

        assert_eq!(post_update_actor_record_pair(&mut var, owner, record), None);
        assert_eq!(state_u16(&var, record.wrapping_add(4)), 0);
        assert_eq!(state_u16(&var, related_field), 0);
    }

    #[test]
    fn post_update_actor_records_scan_resets_disabled_global_at_entry() {
        let owner = 0x0100u16;
        let related = 0x0200u16;
        let record = owner.wrapping_add(TALK_FIELD);
        let related_field = related.wrapping_add(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, 2).expect("kind 2 C4 field"),
        );

        let mut var = vec![0; 0x6800];
        state_set_u16(&mut var, owner, 2);
        state_set_u8(&mut var, owner.wrapping_add(2), 1);
        state_set_u16(&mut var, related, 2);
        state_set_u8(&mut var, VM_PRESENTATION_PAIR_WRITE_DISABLED, 1);
        write_actor_record(&mut var, record, related);

        let context = ExecutionContext::from_object_offsets([owner, related]);
        assert_eq!(
            post_update_actor_records_for_active_objects(&mut var, &context),
            vec![(record, related_field)]
        );
        assert_eq!(state_u8(&var, VM_PRESENTATION_PAIR_WRITE_DISABLED), 0);
        assert_eq!(
            state_u16(&var, record.wrapping_add(4)),
            C4_POST_UPDATE_SENTINEL
        );
        assert_eq!(state_u16(&var, related_field), OP_ACTOR as u16);
    }

    #[test]
    fn post_update_actor_records_scan_only_active_context_objects() {
        let inactive_owner = 0x0100u16;
        let owner = 0x0200u16;
        let related = 0x0300u16;
        let inactive_record = inactive_owner.wrapping_add(TALK_FIELD);
        let record = owner.wrapping_add(TALK_FIELD);
        let related_field = related.wrapping_add(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, 2).expect("kind 2 C4 field"),
        );

        let mut var = vec![0; 0x0400];
        state_set_u16(&mut var, inactive_owner, 2);
        state_set_u16(&mut var, owner, 2);
        state_set_u8(&mut var, owner.wrapping_add(2), 1);
        state_set_u16(&mut var, related, 2);
        write_actor_record(&mut var, inactive_record, related);
        write_actor_record(&mut var, record, related);

        let context = ExecutionContext::from_object_offsets([inactive_owner, owner, related]);
        assert_eq!(
            post_update_actor_records_for_active_objects(&mut var, &context),
            vec![(record, related_field)]
        );

        assert_eq!(state_u16(&var, inactive_record.wrapping_add(4)), 0);
        assert_eq!(
            state_u16(&var, record.wrapping_add(4)),
            C4_POST_UPDATE_SENTINEL
        );
        assert_eq!(state_u16(&var, related_field), OP_ACTOR as u16);
        assert_eq!(state_u16(&var, related_field.wrapping_add(2)), owner);
        assert_eq!(
            state_u16(&var, related_field.wrapping_add(4)),
            C4_POST_UPDATE_SENTINEL
        );
    }

    #[test]
    fn post_update_kind1_c4_record_starts_presentation_state() {
        let owner = 0x0100u16;
        let related = 0x0200u16;
        let record = owner.wrapping_add(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, 1).expect("kind 1 C4 field"),
        );
        let related_field = related.wrapping_add(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, 2).expect("kind 2 C4 field"),
        );

        let mut var = vec![0; 0x6800];
        state_set_u16(&mut var, owner, 1);
        state_set_u8(&mut var, owner.wrapping_add(2), 1);
        state_set_u16(&mut var, related, 2);
        state_set_u8(&mut var, related.wrapping_add(2), 0x20);
        state_set_u8(&mut var, related.wrapping_add(3), 0x01);
        state_set_u8(&mut var, VM_UI_FLAGS, 0x01);
        state_set_u8(&mut var, VM_PRESENTATION_INPUT_GATE_B, 0xff);
        state_set_u16(&mut var, VM_BRANCH_A, 0x1111);
        state_set_u16(&mut var, VM_BRANCH_B, 0x2222);
        state_set_u16(&mut var, VM_PC_SAVED, 0x3333);
        state_set_u16(&mut var, VM_PRESENTATION_WORD_BUFFER, 0x4444);
        state_set_u16(&mut var, VM_PRESENTATION_INPUT_GATE_I, 0x5555);
        state_set_u8(&mut var, VM_PRESENTATION_TEXT_WAIT, 0xff);
        state_set_u8(&mut var, VM_PRESENTATION_HANDOFF_GATE, 0xff);
        state_set_u8(&mut var, VM_PRESENTATION_INPUT_GATE_G, 0xff);
        state_set_u8(&mut var, VM_PRESENTATION_HOLD_READY, 0xff);
        state_set_u8(&mut var, VM_PRESENTATION_HOLD_COMPLETE, 0xff);
        state_set_u16(&mut var, VM_PRESENTATION_SIGNAL_SLOT, 0x6666);
        write_actor_record(&mut var, record, related);

        let context = ExecutionContext::from_object_offsets([owner, related]);
        assert_eq!(
            post_update_actor_records_for_active_objects(&mut var, &context),
            vec![(record, related_field)]
        );
        assert_eq!(state_u8(&var, VM_PRESENTATION_RELATED_FLAG20), 1);
        assert_eq!(state_u8(&var, VM_PRESENTATION_ACTIVE), 1);
        assert_eq!(state_u8(&var, VM_PRESENTATION_SCENE_DIRTY), 1);
        assert_eq!(state_u16(&var, VM_PRESENTATION_STATUS_WORD), 1);
        assert_eq!(state_u16(&var, VM_BRANCH_A), 0);
        assert_eq!(state_u16(&var, VM_BRANCH_B), 0);
        assert_eq!(state_u16(&var, VM_PC_SAVED), 0);
        assert_eq!(state_u16(&var, VM_PRESENTATION_WORD_BUFFER), 0);
        assert_eq!(state_u16(&var, VM_PRESENTATION_INPUT_GATE_I), 0);
        assert_eq!(state_u8(&var, VM_PRESENTATION_TEXT_WAIT), 0);
        assert_eq!(state_u8(&var, VM_PRESENTATION_HANDOFF_GATE), 0);
        assert_eq!(state_u8(&var, VM_PRESENTATION_INPUT_GATE_G), 0xff);
        assert_eq!(state_u8(&var, VM_PRESENTATION_HOLD_READY), 0);
        assert_eq!(state_u8(&var, VM_PRESENTATION_HOLD_COMPLETE), 0);
        assert_eq!(state_u16(&var, VM_PRESENTATION_SIGNAL_SLOT), 0);
        assert_eq!(state_u8(&var, VM_PRESENTATION_START_LOCK), 1);
        assert_eq!(state_u8(&var, VM_UI_FLAGS), 0x05);
        assert_eq!(state_u8(&var, related.wrapping_add(3)), 0x81);
        assert_eq!(state_u8(&var, VM_PRESENTATION_INPUT_GATE_B), 0x7f);
        assert_eq!(
            state_u16(&var, record.wrapping_add(4)),
            C4_POST_UPDATE_SENTINEL
        );
        assert_eq!(state_u16(&var, related_field), OP_ACTOR as u16);
        assert_eq!(state_u16(&var, related_field.wrapping_add(2)), owner);
    }

    #[test]
    fn post_update_kind1_empty_record_stops_active_presentation_state() {
        let owner = 0x0100u16;
        let record = owner.wrapping_add(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, 1).expect("kind 1 C4 field"),
        );

        let mut var = vec![0; 0x6800];
        state_set_u16(&mut var, owner, 1);
        state_set_u8(&mut var, owner.wrapping_add(2), 1);
        state_set_u8(&mut var, VM_PRESENTATION_ACTIVE, 1);
        state_set_u8(&mut var, VM_PRESENTATION_LOOP_FLAG, 0xff);
        state_set_u16(&mut var, VM_PRESENTATION_ACTIVE_RECORD, 0x7777);
        state_set_u8(&mut var, VM_UI_FLAGS, 0xff);
        state_set_u8(&mut var, C2_PRESENTATION_FLAGS, 0xff);
        state_set_u16(&mut var, VM_PRESENTATION_WORD_BUFFER, 0x7777);
        state_set_u8(&mut var, VM_PRESENTATION_START_LOCK, 1);
        state_set_u8(&mut var, VM_PRESENTATION_DESCRIPTOR_PENDING, 1);
        state_set_u16(&mut var, VM_BRANCH_A, 0x1111);
        state_set_u16(&mut var, VM_BRANCH_B, 0x2222);

        let context = ExecutionContext::from_object_offsets([owner]);
        assert_eq!(
            post_update_actor_records_for_active_objects(&mut var, &context),
            vec![]
        );
        assert_eq!(state_u16(&var, VM_PRESENTATION_STATUS_WORD), 1);
        assert_eq!(state_u16(&var, VM_BRANCH_A), 0);
        assert_eq!(state_u16(&var, VM_BRANCH_B), 0);
        assert_eq!(state_u8(&var, VM_PRESENTATION_LOOP_FLAG), 0);
        assert_eq!(state_u8(&var, VM_PRESENTATION_ACTIVE), 0);
        assert_eq!(state_u16(&var, VM_PRESENTATION_ACTIVE_RECORD), 0);
        assert_eq!(state_u8(&var, VM_UI_FLAGS), 0xfb);
        assert_eq!(state_u8(&var, C2_PRESENTATION_FLAGS), 0xfc);
        assert_eq!(state_u16(&var, VM_PRESENTATION_WORD_BUFFER), 0);
        assert_eq!(state_u8(&var, VM_PRESENTATION_START_LOCK), 0);
        assert_eq!(state_u8(&var, VM_PRESENTATION_DESCRIPTOR_PENDING), 0);
        assert_eq!(state_u16(&var, record), 0);
    }

    #[test]
    fn post_update_kind1_scan_drains_deferred_record_to_current_record() {
        let owner = 0x0100u16;
        let record = owner.wrapping_add(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, 1).expect("kind 1 C4 field"),
        );

        let mut var = vec![0; 0x6800];
        state_set_u16(&mut var, owner, 1);
        state_set_u8(&mut var, owner.wrapping_add(2), 1);
        state_set_u16(
            &mut var,
            VM_PRESENTATION_DEFERRED_RECORD_TYPE,
            OP_RECORD_LINK as u16,
        );
        state_set_u16(&mut var, VM_PRESENTATION_DEFERRED_RECORD_RELATED, 0x0222);
        state_set_u16(&mut var, VM_PRESENTATION_DEFERRED_RECORD_AUX, 0x0333);

        let context = ExecutionContext::from_object_offsets([owner]);
        assert_eq!(
            post_update_actor_records_for_active_objects(&mut var, &context),
            vec![]
        );
        assert_eq!(state_u16(&var, record), OP_RECORD_LINK as u16);
        assert_eq!(state_u16(&var, record.wrapping_add(2)), 0x0222);
        assert_eq!(state_u16(&var, record.wrapping_add(4)), 0x0333);
        assert_eq!(state_u16(&var, VM_PRESENTATION_DEFERRED_RECORD_TYPE), 0);
        assert_eq!(state_u16(&var, VM_PRESENTATION_DEFERRED_RECORD_RELATED), 0);
        assert_eq!(state_u16(&var, VM_PRESENTATION_DEFERRED_RECORD_AUX), 0);
    }

    #[test]
    fn post_update_kind1_scan_drains_c1_c6_deferred_record_to_arche() {
        let owner = 0x0100u16;
        let arche = 0x0300u16;
        let owner_record = owner.wrapping_add(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, 1).expect("kind 1 C4 field"),
        );
        let arche_record = arche.wrapping_add(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, 0x10).expect("kind 0x10 C4 field"),
        );

        let mut var = vec![0; 0x6800];
        state_set_u16(&mut var, owner, 1);
        state_set_u8(&mut var, owner.wrapping_add(2), 1);
        state_set_u16(&mut var, arche, 0x10);
        state_set_u16(
            &mut var,
            VM_PRESENTATION_DEFERRED_RECORD_TYPE,
            OP_RECORD_STATE_MIN as u16,
        );
        state_set_u16(&mut var, VM_PRESENTATION_DEFERRED_RECORD_RELATED, 0x0444);
        state_set_u16(&mut var, VM_PRESENTATION_DEFERRED_RECORD_AUX, 0x0555);

        let context = ExecutionContext::from_object_offsets([owner, arche])
            .with_vm_named_object("arche", arche);
        assert_eq!(
            post_update_actor_records_for_active_objects(&mut var, &context),
            vec![]
        );
        assert_eq!(state_u16(&var, owner_record), 0);
        assert_eq!(state_u16(&var, arche_record), OP_RECORD_STATE_MIN as u16);
        assert_eq!(state_u16(&var, arche_record.wrapping_add(2)), 0x0444);
        assert_eq!(state_u16(&var, arche_record.wrapping_add(4)), 0);
        assert_eq!(state_u16(&var, VM_PRESENTATION_DEFERRED_RECORD_TYPE), 0);
        assert_eq!(state_u16(&var, VM_PRESENTATION_DEFERRED_RECORD_RELATED), 0);
        assert_eq!(state_u16(&var, VM_PRESENTATION_DEFERRED_RECORD_AUX), 0);
    }

    #[test]
    fn post_update_kind2_handoff_target_matches_binary_gate() {
        let owner = 0x0100u16;
        let primary_record = 0x0200u16;
        let blood = 0x0300u16;
        let record = owner.wrapping_add(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, 2).expect("kind 2 C4 field"),
        );
        let target_field = owner.wrapping_add(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_PRESENTATION_HANDOFF, 2)
                .expect("kind 2 handoff field"),
        );

        let mut var = vec![0; 0x6800];
        state_set_u16(&mut var, owner, 2);
        state_set_u8(&mut var, owner.wrapping_add(2), 1);
        state_set_u8(&mut var, VM_PRESENTATION_ACTIVE, 1);
        state_set_u16(&mut var, VM_PRESENTATION_PRIMARY_C4_RECORD, primary_record);
        state_set_u16(&mut var, primary_record, OP_ACTOR as u16);
        state_set_u16(&mut var, record, OP_ACTOR as u16);
        state_set_u16(&mut var, record.wrapping_add(2), blood);
        state_set_u16(&mut var, target_field, 0x1234);

        let context =
            ExecutionContext::from_object_offsets([owner, blood]).with_special_object_offset(blood);
        assert_eq!(
            post_update_kind2_presentation_handoff_target(&var, &context, owner, record),
            Some(0x1234)
        );

        state_set_u8(&mut var, VM_PRESENTATION_START_LOCK, 1);
        assert_eq!(
            post_update_kind2_presentation_handoff_target(&var, &context, owner, record),
            None
        );
        state_set_u8(&mut var, VM_PRESENTATION_START_LOCK, 0);
        state_set_u8(&mut var, VM_PRESENTATION_HANDOFF_GATE, 1);
        assert_eq!(
            post_update_kind2_presentation_handoff_target(&var, &context, owner, record),
            None
        );
        state_set_u8(&mut var, VM_PRESENTATION_HANDOFF_GATE, 0);
        state_set_u8(&mut var, VM_PRESENTATION_INPUT_GATE_G, 1);
        assert_eq!(
            post_update_kind2_presentation_handoff_target(&var, &context, owner, record),
            Some(0x1234)
        );
        state_set_u8(&mut var, VM_PRESENTATION_INPUT_GATE_G, 0);
        state_set_u16(
            &mut var,
            owner.wrapping_add(2),
            TEXT_LINE_ALREADY_SHOWN_FLAG | 1,
        );
        assert_eq!(
            post_update_kind2_presentation_handoff_target(&var, &context, owner, record),
            None
        );
    }

    #[test]
    fn post_update_kind2_handoff_rejects_wrong_c4_pair() {
        let owner = 0x0100u16;
        let primary_record = 0x0200u16;
        let blood = 0x0300u16;
        let other = 0x0400u16;
        let record = owner.wrapping_add(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, 2).expect("kind 2 C4 field"),
        );
        let target_field = owner.wrapping_add(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_PRESENTATION_HANDOFF, 2)
                .expect("kind 2 handoff field"),
        );

        let mut var = vec![0; 0x6800];
        state_set_u16(&mut var, owner, 2);
        state_set_u8(&mut var, owner.wrapping_add(2), 1);
        state_set_u8(&mut var, VM_PRESENTATION_ACTIVE, 1);
        state_set_u16(&mut var, VM_PRESENTATION_PRIMARY_C4_RECORD, primary_record);
        state_set_u16(&mut var, primary_record, OP_RECORD_LINK as u16);
        state_set_u16(&mut var, record, OP_ACTOR as u16);
        state_set_u16(&mut var, record.wrapping_add(2), blood);
        state_set_u16(&mut var, target_field, 0x1234);

        let context =
            ExecutionContext::from_object_offsets([owner, blood]).with_special_object_offset(blood);
        assert_eq!(
            post_update_kind2_presentation_handoff_target(&var, &context, owner, record),
            None
        );

        state_set_u16(&mut var, primary_record, OP_ACTOR as u16);
        state_set_u16(&mut var, record.wrapping_add(2), other);
        assert_eq!(
            post_update_kind2_presentation_handoff_target(&var, &context, owner, record),
            None
        );
    }

    #[test]
    fn execution_trace_reports_post_update_c4_pair_scan() {
        let owner = 0x0100u16;
        let related = 0x0200u16;
        let record = owner.wrapping_add(TALK_FIELD);
        let related_field = related.wrapping_add(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, 2).expect("kind 2 C4 field"),
        );

        let mut var = vec![0; 0x6800];
        state_set_u16(&mut var, owner, 2);
        state_set_u8(&mut var, owner.wrapping_add(2), 1);
        state_set_u16(&mut var, related, 2);
        write_actor_record(&mut var, record, related);

        let context = ExecutionContext::from_object_offsets([owner, related]);
        let trace = execute_trace_with_context(&[0xff], &var, &context);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(
            trace.post_update.actor_record_pairs,
            vec![PostUpdateActorRecordPair {
                record_offset: record,
                related_record_offset: related_field,
            }]
        );
    }

    #[test]
    fn execution_trace_follows_post_update_handoff_target() {
        let owner = 0x0100u16;
        let primary_record = 0x0200u16;
        let blood = 0x0300u16;
        let record = owner.wrapping_add(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, 2).expect("kind 2 C4 field"),
        );
        let target_field = owner.wrapping_add(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_PRESENTATION_HANDOFF, 2)
                .expect("kind 2 handoff field"),
        );

        let mut var = vec![0; 0x6800];
        state_set_u16(&mut var, owner, 2);
        state_set_u8(&mut var, owner.wrapping_add(2), 1);
        state_set_u8(&mut var, VM_PRESENTATION_ACTIVE, 1);
        state_set_u16(&mut var, VM_PRESENTATION_PRIMARY_C4_RECORD, primary_record);
        state_set_u16(&mut var, primary_record, OP_ACTOR as u16);
        state_set_u16(&mut var, record, OP_ACTOR as u16);
        state_set_u16(&mut var, record.wrapping_add(2), blood);
        state_set_u16(&mut var, target_field, 1);

        let context =
            ExecutionContext::from_object_offsets([owner, blood]).with_special_object_offset(blood);
        let mut cod = vec![0xff, OP_RECORD_CLEAR];
        cod.extend_from_slice(&record.to_le_bytes());
        let handoff_text_offset = cod.len();
        push_empty_text(&mut cod);
        cod.push(0xff);

        let trace = execute_trace_with_context(&cod, &var, &context);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(
            trace.line_states,
            vec![LineState {
                offset: handoff_text_offset,
                actor_offset: None,
                location_offset: None,
            }]
        );
        assert_eq!(
            trace.post_update.presentation_handoffs,
            vec![PresentationHandoffEvent {
                owner_offset: owner,
                record_offset: record,
                target: 1,
            }]
        );
    }

    #[test]
    fn execution_trace_reports_pending_profile_dispatch_idle_gate() {
        let cod = [OP_SCRIPT_PROFILE_REQUEST, 0x02, 0xff];
        let mut var = vec![0; 0x6800];

        let trace = execute_trace(&cod, &var);
        assert_eq!(trace.pending_script_profile(), Some(1));
        assert!(trace.post_update.pending_script_profile_dispatch_ready);

        state_set_u8(&mut var, VM_PRESENTATION_ACTIVE, 1);
        let trace = execute_trace(&cod, &var);
        assert_eq!(trace.pending_script_profile(), Some(1));
        assert!(!trace.post_update.pending_script_profile_dispatch_ready);

        let no_pending = [OP_SCRIPT_PROFILE_REQUEST, 0x00, 0xff];
        let trace = execute_trace(&no_pending, &vec![0; 0x6800]);
        assert_eq!(trace.pending_script_profile(), None);
        assert!(!trace.post_update.pending_script_profile_dispatch_ready);
    }

    #[test]
    fn pending_script_profile_dispatch_waits_for_presentation_idle() {
        let mut var = vec![0; 0x6800];
        state_set_u16(&mut var, VM_PENDING_RESOURCE_PROFILE, 1);
        assert!(pending_script_profile_dispatch_ready(&var));

        state_set_u8(&mut var, VM_UI_FLAGS, 0x01);
        assert!(pending_script_profile_dispatch_ready(&var));
        state_set_u8(&mut var, VM_UI_FLAGS, 0x02);
        assert!(!pending_script_profile_dispatch_ready(&var));
        state_set_u8(&mut var, VM_UI_FLAGS, 0);

        for gate in MAIN_PENDING_PROFILE_IDLE_GATES {
            state_set_u8(&mut var, gate, 1);
            assert!(
                !pending_script_profile_dispatch_ready(&var),
                "gate {gate:#06x}"
            );
            state_set_u8(&mut var, gate, 0);
        }

        state_set_u16(&mut var, VM_PENDING_RESOURCE_PROFILE, 0xffff);
        assert!(!pending_script_profile_dispatch_ready(&var));
    }

    #[test]
    fn text_selector_active_line_id_matches_signed_binary_bridge() {
        assert_eq!(text_selector_active_line_id(0x00), 9);
        assert_eq!(text_selector_active_line_id(0x01), 10);
        assert_eq!(text_selector_active_line_id(0x05), 14);
        // A6 stores b3 through CBW/sign extension, so 0xFF becomes -1 before +9.
        assert_eq!(text_selector_active_line_id(TEXT_SELECTOR_NONE), 8);
        assert_eq!(text_selector_active_line_id(0xFE), 7);
    }

    #[test]
    fn text_selector_voice_clip_index_uses_one_based_talk_clips() {
        assert!(!text_selector_requests_voice(0x00));
        assert!(!text_selector_requests_voice(0xFF));
        assert!(text_selector_requests_voice(0x01));
        assert_eq!(text_selector_voice_clip_index(0x00, 4), None);
        assert_eq!(text_selector_voice_clip_index(0xFF, 4), None);
        assert_eq!(text_selector_voice_clip_index(0x01, 4), Some(0));
        assert_eq!(text_selector_voice_clip_index(0x04, 4), Some(3));
        assert_eq!(text_selector_voice_clip_index(0x05, 4), None);
    }

    #[test]
    fn text_acceptance_clears_active_bit_unless_preserved_by_b4_bit0() {
        assert_eq!(text_flags_after_accept(0x00, 0xa0), 0x20);
        assert_eq!(
            text_flags_after_accept(TEXT_PRESERVE_ACTIVE_FLAG, 0xa0),
            0xa0
        );

        let mut runtime = TextTokenRuntimeFlags::default();
        assert_eq!(runtime.flags_b5(0x20, TEXT_ACTIVE_DISPLAY_FLAG), 0x80);
        runtime.accept_line(0x20, 0x00, TEXT_ACTIVE_DISPLAY_FLAG);
        assert_eq!(runtime.flags_b5(0x20, TEXT_ACTIVE_DISPLAY_FLAG), 0x00);

        let mut preserved = TextTokenRuntimeFlags::default();
        preserved.accept_line(0x20, TEXT_PRESERVE_ACTIVE_FLAG, TEXT_ACTIVE_DISPLAY_FLAG);
        assert_eq!(preserved.flags_b5(0x20, TEXT_ACTIVE_DISPLAY_FLAG), 0x80);
    }

    #[test]
    fn text_display_gate_skips_inactive_and_already_shown_lines() {
        assert!(!text_flags_are_active(0x00));
        assert!(text_flags_are_active(0x80));
        assert!(text_flags_are_active(0xA0));
        assert_eq!(text_line_flags_offset(0x0020), 0x0022);
        assert!(text_line_already_shown(TEXT_LINE_ALREADY_SHOWN_FLAG));

        let inactive_line = 0x0010u16;
        let pre_shown_line = 0x0020u16;
        let duplicate_line = 0x0030u16;
        let mut var = vec![0; 0x0080];
        state_set_u16(
            &mut var,
            text_line_flags_offset(pre_shown_line),
            TEXT_LINE_ALREADY_SHOWN_FLAG,
        );

        let mut cod = Vec::new();
        let inactive_offset = cod.len();
        push_text_with_flags(&mut cod, inactive_line, 0xFF, 0x00);
        let pre_shown_offset = cod.len();
        push_text_with_flags(&mut cod, pre_shown_line, 0xFF, TEXT_ACTIVE_DISPLAY_FLAG);
        let first_duplicate_offset = cod.len();
        push_text_with_flags(&mut cod, duplicate_line, 0xFF, TEXT_ACTIVE_DISPLAY_FLAG);
        let second_duplicate_offset = cod.len();
        push_text_with_flags(&mut cod, duplicate_line, 0xFF, TEXT_ACTIVE_DISPLAY_FLAG);
        cod.push(0xFF);

        assert_eq!(interpret_line_states(&cod, &var).len(), 4);

        let context = ExecutionContext::default().with_text_line_display_gating();
        let states = interpret_line_states_with_context(&cod, &var, &context);
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].offset, first_duplicate_offset);

        let trace = execute_trace_with_overrides_and_context(&cod, &var, &[], &context);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, first_duplicate_offset);
        assert_ne!(trace.line_states[0].offset, inactive_offset);
        assert_ne!(trace.line_states[0].offset, pre_shown_offset);
        assert_ne!(trace.line_states[0].offset, second_duplicate_offset);
    }

    #[test]
    fn text_presentation_record_gate_requires_active_c4_talk_slot() {
        let line_index = 0x0020u16;
        let talk_record = text_presentation_record_offset(line_index);
        assert_eq!(talk_record, line_index + TALK_FIELD);

        let mut cod = Vec::new();
        push_text_with_flags(&mut cod, line_index, 0xFF, TEXT_ACTIVE_DISPLAY_FLAG);
        cod.push(0xFF);

        let mut var = vec![0; 0x0080];
        let context = ExecutionContext::default().with_text_presentation_record_gating();
        assert_eq!(interpret_line_states(&cod, &var).len(), 1);
        assert!(interpret_line_states_with_context(&cod, &var, &context).is_empty());

        state_set_u16(&mut var, talk_record, OP_ACTOR as u16);
        assert_eq!(
            interpret_line_states_with_context(&cod, &var, &context).len(),
            1
        );
    }

    #[test]
    fn chatter_hold_timers_match_binary_arithmetic() {
        assert_eq!(reveal_complete_hold_ticks(5), 20);
        assert_eq!(record_end_hold_ticks(3, 5), 12);
        assert_eq!(record_end_hold_ticks(3, 6), 15);
        assert_eq!(reveal_complete_hold_ticks(0x8000), 0);
        assert_eq!(record_end_hold_ticks(0xffff, 0xffff), 0x8007);
    }

    #[test]
    fn interpreter_applies_mode0_state_mutation_families() {
        let actor = 0x0100u16;
        let location_field = actor + LOCATION_FIELD;
        let var = vec![0; 0x0200];
        let mut cod = Vec::new();

        push_actor_ref(&mut cod, actor);
        // 0x6946 family: AF direct assignment.
        cod.push(0xAF);
        cod.extend_from_slice(&location_field.to_le_bytes());
        cod.extend_from_slice(&0x1000u16.to_le_bytes());
        push_empty_text(&mut cod);

        // 0x6902 family: AE sets mask bits, B0+A1 clears mask bits.
        cod.push(0xAE);
        cod.extend_from_slice(&location_field.to_le_bytes());
        cod.extend_from_slice(&0x0003u16.to_le_bytes());
        cod.push(0xB0);
        cod.push(0xA1);
        cod.extend_from_slice(&location_field.to_le_bytes());
        cod.extend_from_slice(&0x0001u16.to_le_bytes());
        push_empty_text(&mut cod);

        // 0x6946 family again: BC has the same mode-0 state write.
        cod.push(0xBC);
        cod.extend_from_slice(&location_field.to_le_bytes());
        cod.extend_from_slice(&0x2222u16.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let states = interpret_line_states(&cod, &var);
        assert_eq!(states.len(), 3);
        assert_eq!(states[0].location_offset, Some(0x1000));
        assert_eq!(states[1].location_offset, Some(0x1002));
        assert_eq!(states[2].location_offset, Some(0x2222));
    }

    #[test]
    fn interpreter_record_clear_stops_actor_location_bleed() {
        let actor = 0x0100u16;
        let location_field = actor + LOCATION_FIELD;
        let mut var = vec![0; 0x0200];
        state_set_u16(&mut var, location_field, 0x1111);

        let mut cod = Vec::new();
        push_actor_ref(&mut cod, actor);
        push_empty_text(&mut cod);
        push_record_clear(&mut cod, actor);
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let states = interpret_line_states(&cod, &var);
        assert_eq!(states.len(), 2);
        assert_eq!(states[0].actor_offset, Some(actor));
        assert_eq!(states[0].location_offset, Some(0x1111));
        assert_eq!(states[1].actor_offset, None);
        assert_eq!(states[1].location_offset, None);
    }

    #[test]
    fn interpreter_applies_mode1_record_clear() {
        let actor = 0x0100u16;
        let location_field = actor + LOCATION_FIELD;
        let mut var = vec![0; 0x0200];
        state_set_u16(&mut var, location_field, 0x1111);

        let mut cod = Vec::new();
        push_actor_ref(&mut cod, actor);
        cod.extend_from_slice(&[0xA0, 0x00, 0x00]);
        push_record_clear(&mut cod, actor);
        cod.push(0xA1);
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let states = interpret_line_states(&cod, &var);
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].actor_offset, None);
        assert_eq!(states[0].location_offset, None);
    }

    #[test]
    fn interpreter_record_link_does_not_restore_cleared_actor() {
        let actor = 0x0100u16;
        let location_field = actor + LOCATION_FIELD;
        let mut var = vec![0; 0x0200];
        state_set_u16(&mut var, location_field, 0x1111);

        let mut cod = Vec::new();
        push_actor_ref(&mut cod, actor);
        push_record_clear(&mut cod, actor);
        cod.push(OP_RECORD_LINK);
        cod.extend_from_slice(&actor.wrapping_add(TALK_FIELD).to_le_bytes());
        cod.extend_from_slice(&0x0028u16.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let states = interpret_line_states(&cod, &var);
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].actor_offset, None);
        assert_eq!(states[0].location_offset, None);
    }

    #[test]
    fn interpreter_does_not_apply_mode1_comparison_as_assignment() {
        let actor = 0x0100u16;
        let location_field = actor + LOCATION_FIELD;
        let mut var = vec![0; 0x0200];
        state_set_u16(&mut var, location_field, 0x1111);

        let mut cod = Vec::new();
        cod.extend_from_slice(&[0xA0, 0x00, 0x00]); // enter decoder mode 1
        cod.push(0xC0); // 0x6863 family, but mode 1 is compare/branch, not write
        cod.extend_from_slice(&location_field.to_le_bytes());
        cod.push(0xF5);
        cod.push(0xC1);
        cod.extend_from_slice(&0x2222u16.to_le_bytes());
        cod.push(0xA1); // leave decoder mode 1
        push_actor_ref(&mut cod, actor);
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let states = interpret_line_states(&cod, &var);
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].location_offset, Some(0x1111));
    }

    #[test]
    fn interpreter_uses_mode1_actor_record_as_guarded_context() {
        let actor = 0x0100u16;
        let mut var = vec![0; 0x0200];
        state_set_u16(&mut var, actor + LOCATION_FIELD, 0x1111);

        let mut cod = Vec::new();
        cod.extend_from_slice(&[0xA0, 0x00, 0x00]); // enter decoder mode 1
        push_actor_ref(&mut cod, actor);
        cod.push(0xA1); // leave decoder mode 1
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let states = interpret_line_states(&cod, &var);
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].actor_offset, Some(actor));
        assert_eq!(states[0].location_offset, Some(0x1111));
    }

    #[test]
    fn execution_trace_branches_on_failed_mode1_comparison() {
        let actor = 0x0100u16;
        let location_field = actor + LOCATION_FIELD;
        let mut var = vec![0; 0x0200];
        state_set_u16(&mut var, location_field, 0x1111);

        let mut cod = Vec::new();
        push_actor_ref(&mut cod, actor);
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        cod.push(0xC0);
        cod.extend_from_slice(&location_field.to_le_bytes());
        cod.push(0xF5);
        cod.push(0xC1);
        cod.extend_from_slice(&0x2222u16.to_le_bytes());
        push_empty_text(&mut cod);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let trace = execute_trace(&cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert_eq!(trace.line_states[0].location_offset, Some(0x1111));
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == a0_offset + 3
                && event.opcode == 0xC0
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(target)
        }));
    }

    #[test]
    fn execution_trace_preserves_unresolved_mode1_actor_record_by_default() {
        let actor = 0x0100u16;
        let mut var = vec![0; 0x0200];
        state_set_u16(&mut var, actor + LOCATION_FIELD, 0x1111);

        let mut cod = Vec::new();
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        push_actor_ref(&mut cod, actor);
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let trace = execute_trace(&cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].actor_offset, Some(actor));
        assert_eq!(trace.line_states[0].location_offset, Some(0x1111));
        assert_eq!(trace.line_states[1].offset, target as usize);
        assert_eq!(trace.line_states[1].actor_offset, Some(actor));
        assert!(
            trace.branch_events.iter().all(|event| {
                event.offset != condition_offset || event.condition_passed.is_none()
            })
        );
    }

    #[test]
    fn execution_trace_strict_mode_branches_on_empty_actor_record() {
        let actor = 0x0100u16;
        let mut var = vec![0; 0x0200];
        state_set_u16(&mut var, actor + LOCATION_FIELD, 0x1111);

        let mut cod = Vec::new();
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        push_actor_ref(&mut cod, actor);
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let context = ExecutionContext::default().with_strict_actor_record_branching();
        let trace = execute_trace_with_context(&cod, &var, &context);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert_eq!(trace.line_states[0].actor_offset, None);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == OP_ACTOR
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(target)
        }));
    }

    #[test]
    fn execution_trace_applies_mode1_record_clear() {
        let actor = 0x0100u16;
        let mut var = vec![0; 0x0200];
        state_set_u16(&mut var, actor + LOCATION_FIELD, 0x1111);

        let mut cod = Vec::new();
        push_actor_ref(&mut cod, actor);
        cod.extend_from_slice(&[0xA0, 0x00, 0x00]);
        push_record_clear(&mut cod, actor);
        cod.push(0xA1);
        let text = cod.len();
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let trace = execute_trace(&cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, text);
        assert_eq!(trace.line_states[0].actor_offset, None);
        assert_eq!(trace.line_states[0].location_offset, None);
    }

    #[test]
    fn execution_trace_evaluates_mode1_actor_record_compare() {
        let actor = 0x0100u16;
        let record = actor + TALK_FIELD;
        let related = 0x0028u16;
        let mut var = vec![0; 0x0200];
        state_set_u8(&mut var, actor + 2, 1);
        state_set_u16(&mut var, actor + LOCATION_FIELD, 0x1111);
        state_set_u16(&mut var, record, OP_ACTOR as u16);
        state_set_u16(&mut var, record.wrapping_add(2), related);

        let mut cod = Vec::new();
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        cod.push(OP_ACTOR);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&related.to_le_bytes());
        let first_text = cod.len();
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let trace = execute_trace(&cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert_eq!(trace.line_states[0].actor_offset, Some(actor));
        assert_eq!(trace.line_states[0].location_offset, Some(0x1111));
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == OP_ACTOR
                && !event.branch_taken
                && event.condition_passed == Some(true)
        }));

        let mut inverted_cod = Vec::new();
        let a0_offset = inverted_cod.len();
        inverted_cod.push(0xA0);
        inverted_cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = inverted_cod.len();
        inverted_cod.push(OP_ACTOR);
        inverted_cod.push(0xA1);
        inverted_cod.extend_from_slice(&record.to_le_bytes());
        inverted_cod.extend_from_slice(&related.to_le_bytes());
        push_empty_text(&mut inverted_cod);
        let target = inverted_cod.len() as u16;
        inverted_cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut inverted_cod);
        inverted_cod.push(0xFF);

        let trace = execute_trace(&inverted_cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == OP_ACTOR
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(target)
        }));
    }

    #[test]
    fn execution_trace_override_keeps_failed_condition_fallthrough() {
        let actor = 0x0100u16;
        let location_field = actor + LOCATION_FIELD;
        let mut var = vec![0; 0x0200];
        state_set_u16(&mut var, location_field, 0x1111);

        let mut cod = Vec::new();
        push_actor_ref(&mut cod, actor);
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        cod.push(0xC0);
        cod.extend_from_slice(&location_field.to_le_bytes());
        cod.push(0xF5);
        cod.push(0xC1);
        cod.extend_from_slice(&0x2222u16.to_le_bytes());
        let first_text = cod.len();
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let trace = execute_trace_with_overrides(
            &cod,
            &var,
            &[BranchOverride {
                offset: condition_offset,
                condition_passed: true,
            }],
        );
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert_eq!(trace.line_states[1].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && !event.branch_taken
                && event.condition_passed == Some(true)
                && event.detail == "condition forced passed"
        }));
    }

    #[test]
    fn execution_trace_keeps_successful_condition_block_lines() {
        let actor = 0x0100u16;
        let location_field = actor + LOCATION_FIELD;
        let mut var = vec![0; 0x0200];
        state_set_u16(&mut var, location_field, 0x1111);

        let mut cod = Vec::new();
        push_actor_ref(&mut cod, actor);
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        cod.push(0xC0);
        cod.extend_from_slice(&location_field.to_le_bytes());
        cod.push(0xF5);
        cod.push(0xC1);
        cod.extend_from_slice(&0x1111u16.to_le_bytes());
        let first_text = cod.len();
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let trace = execute_trace(&cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert_eq!(trace.line_states[1].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == a0_offset + 3
                && event.opcode == 0xC0
                && !event.branch_taken
                && event.condition_passed == Some(true)
        }));
    }

    #[test]
    fn execution_trace_remaps_special_object_rhs_for_equality_family() {
        let field = 0x0020u16;
        let special_object = 0x0100u16;
        let mut var = vec![0; 0x0200];
        state_set_u16(&mut var, field, 0xffff);

        let mut cod = Vec::new();
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        cod.push(0xAF);
        cod.extend_from_slice(&field.to_le_bytes());
        cod.extend_from_slice(&special_object.to_le_bytes());
        let first_text = cod.len();
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let trace = execute_trace(&cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == 0xAF
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(target)
        }));

        let context = ExecutionContext::default().with_special_object_offset(special_object);
        let trace = execute_trace_with_context(&cod, &var, &context);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == 0xAF
                && !event.branch_taken
                && event.condition_passed == Some(true)
        }));

        let mut inverted_cod = Vec::new();
        let a0_offset = inverted_cod.len();
        inverted_cod.push(0xA0);
        inverted_cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = inverted_cod.len();
        inverted_cod.push(0xAF);
        inverted_cod.push(0xA1);
        inverted_cod.extend_from_slice(&field.to_le_bytes());
        inverted_cod.extend_from_slice(&special_object.to_le_bytes());
        push_empty_text(&mut inverted_cod);
        let target = inverted_cod.len() as u16;
        inverted_cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut inverted_cod);
        inverted_cod.push(0xFF);

        let trace = execute_trace_with_context(&inverted_cod, &var, &context);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == 0xAF
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(target)
        }));
    }

    #[test]
    fn execution_trace_applies_special_object_mode0_assignment() {
        let special_object = 0x0100u16;
        let owner = 0x0200u16;
        let field = owner + LOCATION_FIELD;
        let var = vec![0; 0x0300];

        let mut cod = Vec::new();
        cod.push(0xAF);
        cod.extend_from_slice(&field.to_le_bytes());
        cod.extend_from_slice(&special_object.to_le_bytes());
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        cod.push(0xAF);
        cod.extend_from_slice(&field.to_le_bytes());
        cod.extend_from_slice(&0xffffu16.to_le_bytes());
        let first_text = cod.len();
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let trace = execute_trace(&cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);

        let context = ExecutionContext::from_object_offsets([special_object, owner, 0x0300])
            .with_special_object_offset(special_object);
        let trace = execute_trace_with_context(&cod, &var, &context);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == 0xAF
                && !event.branch_taken
                && event.condition_passed == Some(true)
        }));

        let states = interpret_line_states_with_context(&cod, &var, &context);
        assert_eq!(states.len(), 2);
        assert_eq!(states[0].offset, first_text);
    }

    #[test]
    fn execution_trace_evaluates_b7_bit_flag_conditions() {
        let mut var = vec![0; 0x40];

        let mut cod = Vec::new();
        cod.push(OP_BIT_FLAG); // mode 0: set bit 1 => mask 0x40 at state[0x10]
        cod.extend_from_slice(&0x0010u16.to_le_bytes());
        cod.push(1);
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        cod.push(OP_BIT_FLAG); // mode 1: test the bit set above
        cod.extend_from_slice(&0x0010u16.to_le_bytes());
        cod.push(1);
        let first_text = cod.len();
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let trace = execute_trace(&cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == OP_BIT_FLAG
                && !event.branch_taken
                && event.condition_passed == Some(true)
        }));

        var[0x10] = 0x40;
        let mut clear_cod = Vec::new();
        clear_cod.push(OP_BIT_FLAG); // mode 0: clear the same bit via A1 prefix
        clear_cod.push(0xA1);
        clear_cod.extend_from_slice(&0x0010u16.to_le_bytes());
        clear_cod.push(1);
        let a0_offset = clear_cod.len();
        clear_cod.push(0xA0);
        clear_cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = clear_cod.len();
        clear_cod.push(OP_BIT_FLAG);
        clear_cod.extend_from_slice(&0x0010u16.to_le_bytes());
        clear_cod.push(1);
        push_empty_text(&mut clear_cod);
        let target = clear_cod.len() as u16;
        clear_cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut clear_cod);
        clear_cod.push(0xFF);

        let trace = execute_trace(&clear_cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == OP_BIT_FLAG
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(target)
        }));
    }

    #[test]
    fn execution_trace_applies_and_compares_pair_records() {
        let record = 0x0020u16;
        let mut var = vec![0; 0x80];

        let mut cod = Vec::new();
        cod.push(OP_PAIR_RECORD_A);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&0x1234u16.to_le_bytes());
        cod.extend_from_slice(&0x5678u16.to_le_bytes());
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        cod.push(OP_PAIR_RECORD_B);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&0x1234u16.to_le_bytes());
        cod.extend_from_slice(&0x5678u16.to_le_bytes());
        let first_text = cod.len();
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let trace = execute_trace(&cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == OP_PAIR_RECORD_B
                && !event.branch_taken
                && event.condition_passed == Some(true)
        }));

        state_set_u16(&mut var, record, 0x1234);
        state_set_u16(&mut var, record.wrapping_add(2), 0x9999);
        let mut compare_cod = Vec::new();
        let a0_offset = compare_cod.len();
        compare_cod.push(0xA0);
        compare_cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = compare_cod.len();
        compare_cod.push(OP_PAIR_RECORD_C);
        compare_cod.extend_from_slice(&record.to_le_bytes());
        compare_cod.extend_from_slice(&0x1234u16.to_le_bytes());
        compare_cod.extend_from_slice(&0x5678u16.to_le_bytes());
        push_empty_text(&mut compare_cod);
        let target = compare_cod.len() as u16;
        compare_cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut compare_cod);
        compare_cod.push(0xFF);

        let trace = execute_trace(&compare_cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == OP_PAIR_RECORD_C
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(target)
        }));
    }

    #[test]
    fn execution_trace_applies_and_compares_c6_record_entries() {
        let record = 0x0020u16;
        let operand = 0x1052u16;
        let mut var = vec![0; 0x80];

        let mut cod = Vec::new();
        cod.push(0xC6);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&operand.to_le_bytes());
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        cod.push(0xC6);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&operand.to_le_bytes());
        let first_text = cod.len();
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let trace = execute_trace(&cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == 0xC6
                && !event.branch_taken
                && event.condition_passed == Some(true)
        }));

        state_set_u16(&mut var, record, 0xC6);
        state_set_u16(&mut var, record.wrapping_add(2), 0x9999);
        let mut compare_cod = Vec::new();
        let a0_offset = compare_cod.len();
        compare_cod.push(0xA0);
        compare_cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = compare_cod.len();
        compare_cod.push(0xC6);
        compare_cod.extend_from_slice(&record.to_le_bytes());
        compare_cod.extend_from_slice(&operand.to_le_bytes());
        push_empty_text(&mut compare_cod);
        let target = compare_cod.len() as u16;
        compare_cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut compare_cod);
        compare_cod.push(0xFF);

        let trace = execute_trace(&compare_cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == 0xC6
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(target)
        }));

        state_set_u16(&mut var, record.wrapping_add(2), operand);
        let mut inverted_cod = Vec::new();
        let a0_offset = inverted_cod.len();
        inverted_cod.push(0xA0);
        inverted_cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = inverted_cod.len();
        inverted_cod.push(0xC6);
        inverted_cod.push(0xA1);
        inverted_cod.extend_from_slice(&record.to_le_bytes());
        inverted_cod.extend_from_slice(&operand.to_le_bytes());
        push_empty_text(&mut inverted_cod);
        let target = inverted_cod.len() as u16;
        inverted_cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut inverted_cod);
        inverted_cod.push(0xFF);

        let trace = execute_trace(&inverted_cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == 0xC6
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(target)
        }));
    }

    #[test]
    fn execution_trace_applies_guarded_record_entry_writes() {
        let c5_record = 0x0020u16;
        let c7_record = 0x0040u16;
        let c8_record = 0x0060u16;
        let c5_operand = 0x0100u16;
        let c7_operand = 0x0120u16;
        let mut var = vec![0; 0x0200];
        state_set_u16(&mut var, c5_operand, 0x0200);
        state_set_u8(&mut var, c5_operand.wrapping_add(2), 1);
        state_set_u8(&mut var, c7_operand.wrapping_add(2), 1);
        state_set_u16(&mut var, c7_record, OP_ACTOR as u16);

        let mut cod = Vec::new();
        cod.push(0xC5);
        cod.extend_from_slice(&c5_record.to_le_bytes());
        cod.extend_from_slice(&c5_operand.to_le_bytes());
        cod.push(0xC7);
        cod.extend_from_slice(&c7_record.to_le_bytes());
        cod.extend_from_slice(&c7_operand.to_le_bytes());
        cod.push(0xC8);
        cod.extend_from_slice(&c8_record.to_le_bytes());
        cod.extend_from_slice(&0x1234u16.to_le_bytes());

        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let c5_condition_offset = cod.len();
        cod.push(0xC5);
        cod.extend_from_slice(&c5_record.to_le_bytes());
        cod.extend_from_slice(&c5_operand.to_le_bytes());
        let c7_condition_offset = cod.len();
        cod.push(0xC7);
        cod.extend_from_slice(&c7_record.to_le_bytes());
        cod.extend_from_slice(&c7_operand.to_le_bytes());
        let c8_condition_offset = cod.len();
        cod.push(0xC8);
        cod.extend_from_slice(&c8_record.to_le_bytes());
        cod.extend_from_slice(&0u16.to_le_bytes());
        let first_text = cod.len();
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let trace = execute_trace(&cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        for (offset, opcode) in [
            (c5_condition_offset, 0xC5),
            (c7_condition_offset, 0xC7),
            (c8_condition_offset, 0xC8),
        ] {
            assert!(trace.branch_events.iter().any(|event| {
                event.offset == offset
                    && event.opcode == opcode
                    && !event.branch_taken
                    && event.condition_passed == Some(true)
            }));
        }
    }

    #[test]
    fn execution_trace_record_entry_mode0_known_failures_branch() {
        fn failed_entry_trace(
            opcode: u8,
            record: u16,
            operand: u16,
            var: Vec<u8>,
        ) -> (ExecutionTrace, usize, u16) {
            let mut cod = Vec::new();
            let a0_offset = cod.len();
            cod.push(0xA0);
            cod.extend_from_slice(&0u16.to_le_bytes());
            cod.push(0xA1);
            let condition_offset = cod.len();
            cod.push(opcode);
            cod.extend_from_slice(&record.to_le_bytes());
            cod.extend_from_slice(&operand.to_le_bytes());
            push_empty_text(&mut cod);
            let target = cod.len() as u16;
            cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
            push_empty_text(&mut cod);
            cod.push(0xff);
            (execute_trace(&cod, &var), condition_offset, target)
        }

        let (trace, condition_offset, target) =
            failed_entry_trace(0xC5, 0x0020, 0x0100, vec![0; 0x0200]);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == 0xC5
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(target)
                && event.detail == "mode0 record entry write failed"
        }));

        let (trace, condition_offset, target) =
            failed_entry_trace(0xC7, 0x0040, 0x0120, vec![0; 0x0200]);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == 0xC7
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(target)
                && event.detail == "mode0 record entry write failed"
        }));

        let mut var = vec![0; 0x0200];
        state_set_u16(&mut var, 0x0060, 0x1234);
        let (trace, condition_offset, target) = failed_entry_trace(0xC8, 0x0060, 0x0120, var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == 0xC8
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(target)
                && event.detail == "mode0 record entry write failed"
        }));
    }

    #[test]
    fn execution_trace_compares_record_state_entries() {
        let record = 0x0020u16;
        let operand = 0x1052u16;
        let mut var = vec![0; 0x80];
        state_set_u16(&mut var, record, 0xC1);
        state_set_u16(&mut var, record.wrapping_add(2), operand);

        let mut cod = Vec::new();
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        cod.push(0xC1);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&operand.to_le_bytes());
        let first_text = cod.len();
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let trace = execute_trace(&cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == 0xC1
                && !event.branch_taken
                && event.condition_passed == Some(true)
        }));

        let owner = 0x0100u16;
        let c2_record = owner + TALK_FIELD;
        let c2_operand = 0x0180u16;
        let mut var = vec![0; 0x0200];
        state_set_u8(&mut var, owner + 2, 1);
        state_set_u16(&mut var, c2_record, 0xC2);
        state_set_u16(&mut var, c2_record.wrapping_add(2), c2_operand);
        let context = ExecutionContext::from_object_offsets([owner, 0x0200]);

        let mut c2_cod = Vec::new();
        let a0_offset = c2_cod.len();
        c2_cod.push(0xA0);
        c2_cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = c2_cod.len();
        c2_cod.push(0xC2);
        c2_cod.extend_from_slice(&c2_record.to_le_bytes());
        c2_cod.extend_from_slice(&c2_operand.to_le_bytes());
        let first_text = c2_cod.len();
        push_empty_text(&mut c2_cod);
        c2_cod.push(0xA1);
        let target = c2_cod.len() as u16;
        c2_cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut c2_cod);
        c2_cod.push(0xFF);

        let trace = execute_trace(&c2_cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert!(
            trace.branch_events.iter().all(|event| {
                event.offset != condition_offset || event.condition_passed.is_none()
            })
        );

        let trace = execute_trace_with_context(&c2_cod, &var, &context);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == 0xC2
                && !event.branch_taken
                && event.condition_passed == Some(true)
        }));

        let mut inverted_cod = Vec::new();
        let a0_offset = inverted_cod.len();
        inverted_cod.push(0xA0);
        inverted_cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = inverted_cod.len();
        inverted_cod.push(0xC2);
        inverted_cod.push(0xA1);
        inverted_cod.extend_from_slice(&c2_record.to_le_bytes());
        inverted_cod.extend_from_slice(&c2_operand.to_le_bytes());
        push_empty_text(&mut inverted_cod);
        let target = inverted_cod.len() as u16;
        inverted_cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut inverted_cod);
        inverted_cod.push(0xFF);

        let trace = execute_trace_with_context(&inverted_cod, &var, &context);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == 0xC2
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(target)
        }));
    }

    #[test]
    fn execution_trace_applies_c1_record_state_direct_write_with_context() {
        let owner = 0x0100u16;
        let record = owner + TALK_FIELD;
        let operand = 0x1052u16;
        let mut var = vec![0; 0x0200];
        state_set_u8(&mut var, owner + 2, 1);
        let context = ExecutionContext::from_object_offsets([owner, 0x0200]);

        let mut cod = Vec::new();
        cod.push(OP_RECORD_STATE_MIN);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&operand.to_le_bytes());

        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        cod.push(OP_RECORD_STATE_MIN);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&operand.to_le_bytes());
        let first_text = cod.len();
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let trace = execute_trace(&cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert!(
            trace.branch_events.iter().all(|event| {
                event.offset != condition_offset || event.condition_passed.is_none()
            })
        );

        let trace = execute_trace_with_context(&cod, &var, &context);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == OP_RECORD_STATE_MIN
                && !event.branch_taken
                && event.condition_passed == Some(true)
        }));
    }

    #[test]
    fn execution_trace_c1_mode0_known_failure_branches() {
        let owner = 0x0100u16;
        let record = owner + TALK_FIELD;
        let operand = 0x1052u16;
        let var = vec![0; 0x0200];
        let context = ExecutionContext::from_object_offsets([owner, 0x0200]);

        let mut cod = Vec::new();
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        cod.push(0xA1);
        let c1_offset = cod.len();
        cod.push(OP_RECORD_STATE_MIN);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&operand.to_le_bytes());
        push_empty_text(&mut cod);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xff);

        let trace = execute_trace_with_context(&cod, &var, &context);

        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == c1_offset
                && event.opcode == OP_RECORD_STATE_MIN
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(target)
                && event.detail == "mode0 C1 write failed"
        }));
    }

    #[test]
    fn execution_trace_c1_mode0_missing_owner_context_does_not_branch() {
        let owner = 0x0100u16;
        let record = owner + TALK_FIELD;
        let operand = 0x1052u16;
        let var = vec![0; 0x0200];

        let mut cod = Vec::new();
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        cod.push(0xA1);
        let c1_offset = cod.len();
        cod.push(OP_RECORD_STATE_MIN);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&operand.to_le_bytes());
        let first_text = cod.len();
        push_empty_text(&mut cod);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xff);

        let trace = execute_trace(&cod, &var);

        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert!(
            trace.branch_events.iter().all(|event| {
                event.offset != c1_offset || event.detail != "mode0 C1 write failed"
            })
        );
    }

    #[test]
    fn execution_trace_c1_mode1_resolves_selector11_selector13_slot() {
        let owner = 0x0100u16;
        let record = 0x0140u16;
        let operand = 0x0001u16;
        let target_record = 0x0200u16;
        let parent_field =
            vm_field_offset(ship3d::SHIP_3D_FIELD_SELECTOR_PARENT_LINK, operand).unwrap();
        let destination = target_record
            + vm_field_offset(
                ship3d::SHIP_3D_C1_DESTINATION_SELECTOR,
                ship3d::SHIP_3D_C1_KIND10_RECORD_KIND,
            )
            .unwrap();
        let mut var = vec![0; 0x0300];
        state_set_u16(&mut var, owner + parent_field, target_record);
        state_set_u16(
            &mut var,
            target_record,
            ship3d::SHIP_3D_C1_KIND10_RECORD_KIND,
        );
        state_set_u16(&mut var, destination, OP_RECORD_STATE_MIN as u16);
        state_set_u16(&mut var, destination + 2, operand);
        let context = ExecutionContext::from_object_offsets([owner, target_record]);

        let mut cod = Vec::new();
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        cod.push(OP_RECORD_STATE_MIN);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&operand.to_le_bytes());
        let first_text = cod.len();
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let branch_target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&branch_target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xff);

        let trace = execute_trace_with_context(&cod, &var, &context);

        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == OP_RECORD_STATE_MIN
                && !event.branch_taken
                && event.condition_passed == Some(true)
        }));
    }

    #[test]
    fn execution_trace_c1_mode1_inverted_resolved_match_branches() {
        let owner = 0x0100u16;
        let record = 0x0140u16;
        let operand = 0x0002u16;
        let target_record = 0x0200u16;
        let parent_field =
            vm_field_offset(ship3d::SHIP_3D_FIELD_SELECTOR_PARENT_LINK, operand).unwrap();
        let destination = target_record
            + vm_field_offset(
                ship3d::SHIP_3D_C1_DESTINATION_SELECTOR,
                ship3d::SHIP_3D_C1_KIND10_RECORD_KIND,
            )
            .unwrap();
        let mut var = vec![0; 0x0300];
        state_set_u16(&mut var, owner + parent_field, target_record);
        state_set_u16(
            &mut var,
            target_record,
            ship3d::SHIP_3D_C1_KIND10_RECORD_KIND,
        );
        state_set_u16(&mut var, destination, OP_RECORD_STATE_MIN as u16);
        state_set_u16(&mut var, destination + 2, operand);
        let context = ExecutionContext::from_object_offsets([owner, target_record]);

        let mut cod = Vec::new();
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        cod.push(OP_RECORD_STATE_MIN);
        cod.push(0xA1);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&operand.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let branch_target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&branch_target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xff);

        let trace = execute_trace_with_context(&cod, &var, &context);

        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, branch_target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == OP_RECORD_STATE_MIN
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(branch_target)
        }));
    }

    #[test]
    fn execution_trace_c1_mode1_resolved_target_without_selector13_fails() {
        let owner = 0x0100u16;
        let record = 0x0140u16;
        let operand = 0x0001u16;
        let target_record = 0x0200u16;
        let parent_field =
            vm_field_offset(ship3d::SHIP_3D_FIELD_SELECTOR_PARENT_LINK, operand).unwrap();
        let mut var = vec![0; 0x0300];
        state_set_u16(&mut var, owner + parent_field, target_record);
        state_set_u16(&mut var, target_record, 0);
        let context = ExecutionContext::from_object_offsets([owner, target_record]);

        let mut cod = Vec::new();
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        cod.push(OP_RECORD_STATE_MIN);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&operand.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let branch_target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&branch_target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xff);

        let trace = execute_trace_with_context(&cod, &var, &context);

        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, branch_target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == OP_RECORD_STATE_MIN
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(branch_target)
        }));
    }

    fn ship3d_c1_nav_record(offset: u16, kind_flags: u16) -> ship3d::Ship3dNavigationRuntimeRecord {
        ship3d::Ship3dNavigationRuntimeRecord {
            offset,
            kind_flags,
            state_flags: 0,
            counter_link: 0,
            related_target: 0,
            source_parent: None,
        }
    }

    fn ship3d_c1_bitset_source_list(source: u16) -> Vec<u8> {
        let mut source_list_bytes = vec![0u8; 0x21];
        source_list_bytes[0..2].copy_from_slice(&source.to_le_bytes());
        source_list_bytes[2..4]
            .copy_from_slice(&ship3d::SHIP_3D_TARGET_EXIT_SENTINEL.to_le_bytes());
        source_list_bytes[0x20] = 0x80;
        source_list_bytes
    }

    fn ship3d_c1_cod(record: u16, operand: u16) -> Vec<u8> {
        let mut cod = Vec::new();
        cod.push(OP_RECORD_STATE_MIN);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&operand.to_le_bytes());
        cod.push(0xff);
        cod
    }

    fn ship3d_position_record(
        offset: u16,
        kind_flags: u16,
        parent_link: Option<u16>,
        kind100_match_word: Option<u16>,
        kind100_relation_word: Option<u16>,
    ) -> ship3d::Ship3dPositionRecord {
        ship3d::Ship3dPositionRecord {
            offset,
            kind_flags,
            parent_link,
            kind100_match_word,
            kind100_relation_word,
        }
    }

    fn ship3d_position_field(offset: u16, x: u16, y: u16) -> ship3d::Ship3dPositionField {
        ship3d::Ship3dPositionField { offset, x, y }
    }

    #[test]
    fn execution_trace_applies_ship3d_c1_kind10_resolved_write_with_context() {
        let owner = 0x0100u16;
        let record = 0x0140u16;
        let destination = owner + 0x1c;
        let operand = 0x2000u16;
        let source = 0x3000u16;
        let mut var = vec![0; 0x3100];
        state_set_u16(&mut var, owner, ship3d::SHIP_3D_C1_KIND10_RECORD_KIND);
        state_set_u8(&mut var, owner + 2, 1);
        state_set_u16(&mut var, operand, ship3d::SHIP_3D_C1_SOURCE_KIND_BITSET);

        let mut source_list_bytes = [0u8; 0x21];
        source_list_bytes[0..2].copy_from_slice(&source.to_le_bytes());
        source_list_bytes[2..4]
            .copy_from_slice(&ship3d::SHIP_3D_TARGET_EXIT_SENTINEL.to_le_bytes());
        source_list_bytes[0x20] = 0x80;
        let context = ExecutionContext::from_object_offsets([owner, operand])
            .with_ship_3d_c1_runtime(
                [ship3d::Ship3dNavigationRuntimeRecord {
                    offset: source,
                    kind_flags: ship3d::SHIP_3D_C1_SOURCE_KIND_BITSET,
                    state_flags: 0,
                    counter_link: 0,
                    related_target: 0,
                    source_parent: None,
                }],
                [operand],
                source_list_bytes,
            );

        let mut cod = Vec::new();
        cod.push(OP_RECORD_STATE_MIN);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&operand.to_le_bytes());
        cod.push(0xff);

        let executed = execute_trace_state_with_overrides_and_context(&cod, &var, &[], &context, 0);

        assert_eq!(executed.trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(state_u16(&executed.final_state, record), 0);
        assert_eq!(
            state_u16(&executed.final_state, destination),
            OP_RECORD_STATE_MIN as u16
        );
        assert_eq!(state_u16(&executed.final_state, destination + 2), operand);
        assert_eq!(state_u16(&executed.final_state, destination + 4), 2);
    }

    #[test]
    fn execution_trace_ship3d_c1_kind10_source_rejects_without_direct_fallback() {
        let owner = 0x0100u16;
        let record = 0x0140u16;
        let destination = owner + 0x1c;
        let operand = 0x2000u16;
        let source = 0x3000u16;
        let mut var = vec![0; 0x3100];
        state_set_u16(&mut var, owner, ship3d::SHIP_3D_C1_KIND10_RECORD_KIND);
        state_set_u8(&mut var, owner + 2, 1);
        state_set_u16(&mut var, operand, ship3d::SHIP_3D_C1_SOURCE_KIND_BITSET);

        let mut source_list_bytes = [0u8; 0x21];
        source_list_bytes[0..2].copy_from_slice(&source.to_le_bytes());
        source_list_bytes[2..4]
            .copy_from_slice(&ship3d::SHIP_3D_TARGET_EXIT_SENTINEL.to_le_bytes());
        let context = ExecutionContext::from_object_offsets([owner, operand])
            .with_ship_3d_c1_runtime(
                [ship3d::Ship3dNavigationRuntimeRecord {
                    offset: source,
                    kind_flags: ship3d::SHIP_3D_C1_SOURCE_KIND_BITSET,
                    state_flags: 0,
                    counter_link: 0,
                    related_target: 0,
                    source_parent: None,
                }],
                [operand],
                source_list_bytes,
            );

        let mut cod = Vec::new();
        cod.push(OP_RECORD_STATE_MIN);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&operand.to_le_bytes());
        cod.push(0xff);

        let executed = execute_trace_state_with_overrides_and_context(&cod, &var, &[], &context, 0);

        assert_eq!(executed.trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(state_u16(&executed.final_state, record), 0);
        assert_eq!(state_u16(&executed.final_state, destination), 0);
    }

    #[test]
    fn execution_trace_ship3d_c1_kind10_requires_source_list_sentinel() {
        let owner = 0x0100u16;
        let record = 0x0140u16;
        let destination = owner + 0x1c;
        let operand = 0x2000u16;
        let source = 0x3000u16;
        let mut var = vec![0; 0x3100];
        state_set_u16(&mut var, owner, ship3d::SHIP_3D_C1_KIND10_RECORD_KIND);
        state_set_u8(&mut var, owner + 2, 1);
        state_set_u16(&mut var, operand, ship3d::SHIP_3D_C1_SOURCE_KIND_BITSET);

        let context = ExecutionContext::from_object_offsets([owner, operand])
            .with_ship_3d_c1_runtime(
                [ship3d::Ship3dNavigationRuntimeRecord {
                    offset: source,
                    kind_flags: ship3d::SHIP_3D_C1_SOURCE_KIND_BITSET,
                    state_flags: 0,
                    counter_link: 0,
                    related_target: 0,
                    source_parent: None,
                }],
                [operand],
                source.to_le_bytes(),
            );

        let mut cod = Vec::new();
        cod.push(OP_RECORD_STATE_MIN);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&operand.to_le_bytes());
        cod.push(0xff);

        let executed = execute_trace_state_with_overrides_and_context(&cod, &var, &[], &context, 0);

        assert_eq!(executed.trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(state_u16(&executed.final_state, record), 0);
        assert_eq!(state_u16(&executed.final_state, destination), 0);
    }

    #[test]
    fn execution_trace_ship3d_c1_distance_redirect_writes_kind10_target() {
        let owner = 0x0100u16;
        let record = 0x0140u16;
        let target = 0x0200u16;
        let destination = target + 0x1c;
        let operand = 0x0001u16;
        let source = 0x3000u16;
        let parent_field =
            vm_field_offset(ship3d::SHIP_3D_FIELD_SELECTOR_PARENT_LINK, 0x0002).unwrap();
        let mut var = vec![0; 0x3100];
        state_set_u16(&mut var, owner, 0x0002);
        state_set_u8(&mut var, owner + 2, 1);
        state_set_u16(&mut var, owner + parent_field, target);
        state_set_u16(&mut var, target, ship3d::SHIP_3D_C1_KIND10_RECORD_KIND);
        state_set_u16(&mut var, operand, ship3d::SHIP_3D_C1_SOURCE_KIND_BITSET);

        let context = ExecutionContext::from_object_offsets([operand, owner, target])
            .with_ship_3d_c1_runtime(
                [ship3d_c1_nav_record(
                    source,
                    ship3d::SHIP_3D_C1_SOURCE_KIND_BITSET,
                )],
                [operand],
                ship3d_c1_bitset_source_list(source),
            )
            .with_ship_3d_c1_positions(
                [
                    ship3d_position_record(
                        operand,
                        ship3d::SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8,
                        None,
                        None,
                        None,
                    ),
                    ship3d_position_record(owner, 0x0002, Some(target), None, None),
                    ship3d_position_record(
                        target,
                        ship3d::SHIP_3D_C1_KIND10_RECORD_KIND,
                        None,
                        None,
                        None,
                    ),
                ],
                [
                    ship3d_position_field(operand + 0x18, 0, 0),
                    ship3d_position_field(target + 0x18, 3, 4),
                ],
                0,
                0,
            );
        let cod = ship3d_c1_cod(record, operand);

        let executed = execute_trace_state_with_overrides_and_context(&cod, &var, &[], &context, 0);

        assert_eq!(executed.trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(state_u16(&executed.final_state, record), 0);
        assert_eq!(state_u16(&executed.final_state, owner + 0x1c), 0);
        assert_eq!(
            state_u16(&executed.final_state, destination),
            OP_RECORD_STATE_MIN as u16
        );
        assert_eq!(state_u16(&executed.final_state, destination + 2), operand);
        assert_eq!(state_u16(&executed.final_state, destination + 4), 2);
    }

    #[test]
    fn execution_trace_ship3d_c1_distance_zero_keeps_kind10_owner() {
        let owner = 0x0100u16;
        let record = 0x0140u16;
        let destination = owner + 0x1c;
        let operand = 0x0001u16;
        let source = 0x3000u16;
        let mut var = vec![0; 0x3100];
        state_set_u16(&mut var, owner, ship3d::SHIP_3D_C1_KIND10_RECORD_KIND);
        state_set_u8(&mut var, owner + 2, 1);
        state_set_u16(&mut var, operand, ship3d::SHIP_3D_C1_SOURCE_KIND_BITSET);

        let context = ExecutionContext::from_object_offsets([operand, owner])
            .with_ship_3d_c1_runtime(
                [ship3d_c1_nav_record(
                    source,
                    ship3d::SHIP_3D_C1_SOURCE_KIND_BITSET,
                )],
                [operand],
                ship3d_c1_bitset_source_list(source),
            )
            .with_ship_3d_c1_positions(
                [
                    ship3d_position_record(
                        operand,
                        ship3d::SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8,
                        None,
                        None,
                        None,
                    ),
                    ship3d_position_record(
                        owner,
                        ship3d::SHIP_3D_C1_KIND10_RECORD_KIND,
                        None,
                        None,
                        None,
                    ),
                ],
                [
                    ship3d_position_field(operand + 0x18, 7, 9),
                    ship3d_position_field(owner + 0x18, 7, 9),
                ],
                0,
                0,
            );
        let cod = ship3d_c1_cod(record, operand);

        let executed = execute_trace_state_with_overrides_and_context(&cod, &var, &[], &context, 0);

        assert_eq!(executed.trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(state_u16(&executed.final_state, record), 0);
        assert_eq!(
            state_u16(&executed.final_state, destination),
            OP_RECORD_STATE_MIN as u16
        );
        assert_eq!(state_u16(&executed.final_state, destination + 2), operand);
        assert_eq!(state_u16(&executed.final_state, destination + 4), 2);
    }

    #[test]
    fn execution_trace_ship3d_c1_distance_redirect_rejects_non_kind10_target() {
        let owner = 0x0100u16;
        let record = 0x0140u16;
        let bad_target = 0x0200u16;
        let coord_target = 0x0300u16;
        let operand = 0x0001u16;
        let source = 0x4000u16;
        let parent_field =
            vm_field_offset(ship3d::SHIP_3D_FIELD_SELECTOR_PARENT_LINK, 0x0002).unwrap();
        let mut var = vec![0; 0x4100];
        state_set_u16(&mut var, owner, 0x0002);
        state_set_u8(&mut var, owner + 2, 1);
        state_set_u16(&mut var, owner + parent_field, bad_target);
        state_set_u16(&mut var, bad_target, 0x0002);
        state_set_u16(&mut var, bad_target + parent_field, coord_target);
        state_set_u16(
            &mut var,
            coord_target,
            ship3d::SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8,
        );
        state_set_u16(&mut var, operand, ship3d::SHIP_3D_C1_SOURCE_KIND_BITSET);

        let context = ExecutionContext::from_object_offsets([operand, owner, bad_target])
            .with_ship_3d_c1_runtime(
                [ship3d_c1_nav_record(
                    source,
                    ship3d::SHIP_3D_C1_SOURCE_KIND_BITSET,
                )],
                [operand],
                ship3d_c1_bitset_source_list(source),
            )
            .with_ship_3d_c1_positions(
                [
                    ship3d_position_record(
                        operand,
                        ship3d::SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8,
                        None,
                        None,
                        None,
                    ),
                    ship3d_position_record(owner, 0x0002, Some(bad_target), None, None),
                    ship3d_position_record(bad_target, 0x0002, Some(coord_target), None, None),
                    ship3d_position_record(
                        coord_target,
                        ship3d::SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8,
                        None,
                        None,
                        None,
                    ),
                ],
                [
                    ship3d_position_field(operand + 0x18, 0, 0),
                    ship3d_position_field(coord_target + 0x18, 5, 0),
                ],
                0,
                0,
            );
        let cod = ship3d_c1_cod(record, operand);

        let executed = execute_trace_state_with_overrides_and_context(&cod, &var, &[], &context, 0);

        assert_eq!(executed.trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(state_u16(&executed.final_state, record), 0);
        assert_eq!(state_u16(&executed.final_state, owner + 0x1c), 0);
        assert_eq!(state_u16(&executed.final_state, bad_target + 0x1c), 0);
    }

    #[test]
    fn execution_trace_applies_c2_record_state_direct_write_with_context() {
        fn push_word_equals(cod: &mut Vec<u8>, addr: u16, value: u16) {
            cod.push(0xB1);
            cod.extend_from_slice(&addr.to_le_bytes());
            cod.push(0xF5);
            cod.push(0x00);
            cod.extend_from_slice(&value.to_le_bytes());
        }

        let owner = 0x0100u16;
        let record = owner + TALK_FIELD;
        let target_record = 0x0200u16;
        assert_eq!(vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C2, 2), Some(0x18));
        assert_eq!(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C2, 0x0400),
            Some(0x14)
        );
        let target_field = target_record
            .wrapping_add(vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C2, 2).expect("kind 2 field"));
        let mut var = vec![0; 0x7000];
        state_set_u8(&mut var, owner + 2, 1);
        state_set_u16(&mut var, target_record, 2);
        state_set_u8(&mut var, target_record.wrapping_add(2), 0x20);
        state_set_u8(&mut var, C2_PRESENTATION_GATE, 0xff);
        let context = ExecutionContext::from_object_offsets([owner, 0x0300]);

        let mut cod = Vec::new();
        cod.push(OP_RECORD_STATE_MAX);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&target_record.to_le_bytes());

        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let field_condition_offset = cod.len();
        push_word_equals(&mut cod, target_field, 0xffff);
        let active_line_condition_offset = cod.len();
        push_word_equals(&mut cod, VM_ACTIVE_LINE, C2_ACTIVE_LINE_KIND2);
        let first_text = cod.len();
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let trace = execute_trace(&cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == field_condition_offset
                && event.branch_taken
                && event.condition_passed == Some(false)
        }));

        let trace = execute_trace_with_context(&cod, &var, &context);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == field_condition_offset
                && !event.branch_taken
                && event.condition_passed == Some(true)
        }));
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == active_line_condition_offset
                && !event.branch_taken
                && event.condition_passed == Some(true)
        }));
    }

    #[test]
    fn c2_kind400_descript_lookup_success_sets_presentation_state() {
        let owner = 0x0100u16;
        let record = owner + TALK_FIELD;
        let target_record = 0x0200u16;
        let target_field = target_record.wrapping_add(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C2, 0x0400).expect("kind 0x400 field"),
        );

        let mut var = vec![0; 0x7000];
        state_set_u8(&mut var, owner + 2, 1);
        state_set_u16(&mut var, target_record, 0x0400);
        state_set_u8(&mut var, target_record.wrapping_add(2), 0x20);
        let name = b"PRESENTE";
        let name_start = target_record.wrapping_add(4) as usize;
        var[name_start..name_start + name.len()].copy_from_slice(name);
        var[name_start + name.len()] = 0;
        state_set_u8(&mut var, C2_PRESENTATION_GATE, 0xff);

        let context = ExecutionContext::from_object_offsets([owner, 0x0300]);
        let mut no_match = var.clone();
        assert!(write_c2_record_state_direct(
            &mut no_match,
            &context,
            &mut SpecialObjectSlots::default(),
            record,
            target_record,
        ));
        assert_eq!(state_u16(&no_match, target_field), 0xffff);
        assert_eq!(state_u8(&no_match, C2_PRESENTATION_GATE), 0xff);
        assert_eq!(state_u8(&no_match, C2_PRESENTATION_FLAGS), 0);
        assert_eq!(state_u16(&no_match, VM_ACTIVE_LINE), 0);

        let context = context.with_descript_entry_name("PRESENTE");
        assert!(write_c2_record_state_direct(
            &mut var,
            &context,
            &mut SpecialObjectSlots::default(),
            record,
            target_record,
        ));
        assert_eq!(state_u16(&var, target_field), 0xffff);
        assert_eq!(state_u8(&var, C2_PRESENTATION_GATE), 0);
        assert_eq!(
            state_u8(&var, C2_PRESENTATION_FLAGS) & C2_PRESENTATION_BUSY_FLAG,
            C2_PRESENTATION_BUSY_FLAG
        );
        assert_eq!(state_u16(&var, VM_ACTIVE_LINE), C2_ACTIVE_LINE_KIND400);
    }

    #[test]
    fn execution_trace_applies_and_compares_record_links_with_context() {
        let owner = 0x0100u16;
        let record = owner + TALK_FIELD;
        let related = 0x0180u16;
        let mut var = vec![0; 0x0200];
        state_set_u8(&mut var, owner + 2, 1);
        state_set_u8(&mut var, related + 2, 1);
        let context = ExecutionContext::from_object_offsets([owner, 0x0200]);

        let mut cod = Vec::new();
        cod.push(OP_RECORD_LINK);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&related.to_le_bytes());
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        cod.push(OP_RECORD_LINK);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&related.to_le_bytes());
        let first_text = cod.len();
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let trace = execute_trace_with_context(&cod, &var, &context);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == OP_RECORD_LINK
                && !event.branch_taken
                && event.condition_passed == Some(true)
        }));

        let trace = execute_trace(&cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert!(
            trace.branch_events.iter().all(|event| {
                event.offset != condition_offset || event.condition_passed.is_none()
            })
        );

        state_set_u16(&mut var, record, OP_RECORD_LINK as u16);
        state_set_u16(&mut var, record.wrapping_add(2), related);
        let mut inverted_cod = Vec::new();
        let a0_offset = inverted_cod.len();
        inverted_cod.push(0xA0);
        inverted_cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = inverted_cod.len();
        inverted_cod.push(OP_RECORD_LINK);
        inverted_cod.push(0xA1);
        inverted_cod.extend_from_slice(&record.to_le_bytes());
        inverted_cod.extend_from_slice(&related.to_le_bytes());
        push_empty_text(&mut inverted_cod);
        let target = inverted_cod.len() as u16;
        inverted_cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut inverted_cod);
        inverted_cod.push(0xFF);

        let trace = execute_trace_with_context(&inverted_cod, &var, &context);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == OP_RECORD_LINK
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(target)
        }));
    }

    #[test]
    fn execution_trace_c3_mode0_known_failure_branches() {
        let owner = 0x0100u16;
        let record = owner + TALK_FIELD;
        let related = 0x0180u16;
        let mut var = vec![0; 0x0200];
        state_set_u8(&mut var, related + 2, 1);
        let context = ExecutionContext::from_object_offsets([owner, 0x0200]);

        let mut cod = Vec::new();
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        cod.push(0xA1);
        let condition_offset = cod.len();
        cod.push(OP_RECORD_LINK);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&related.to_le_bytes());
        push_empty_text(&mut cod);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xff);

        let trace = execute_trace_with_context(&cod, &var, &context);

        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == OP_RECORD_LINK
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(target)
                && event.detail == "mode0 C3 write failed"
        }));
    }

    #[test]
    fn execution_trace_c3_mode0_missing_owner_context_does_not_branch() {
        let owner = 0x0100u16;
        let record = owner + TALK_FIELD;
        let related = 0x0180u16;
        let mut var = vec![0; 0x0200];
        state_set_u8(&mut var, related + 2, 1);

        let mut cod = Vec::new();
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        cod.push(0xA1);
        let condition_offset = cod.len();
        cod.push(OP_RECORD_LINK);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&related.to_le_bytes());
        let first_text = cod.len();
        push_empty_text(&mut cod);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xff);

        let trace = execute_trace(&cod, &var);

        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert!(trace.branch_events.iter().all(|event| {
            event.offset != condition_offset || event.detail != "mode0 C3 write failed"
        }));
    }

    #[test]
    fn execution_trace_evaluates_record_triple_mode1_compare() {
        let record = 0x0030u16;
        let mut var = vec![0; 0x80];
        state_set_u16(&mut var, record, OP_RECORD_TRIPLE as u16);
        state_set_u16(&mut var, record.wrapping_add(2), 0x1064);
        state_set_u16(&mut var, record.wrapping_add(4), 0x055A);

        let mut cod = Vec::new();
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        cod.push(OP_RECORD_TRIPLE);
        cod.extend_from_slice(&record.to_le_bytes());
        cod.extend_from_slice(&0x1064u16.to_le_bytes());
        cod.extend_from_slice(&0x055Au16.to_le_bytes());
        let first_text = cod.len();
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let trace = execute_trace(&cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == OP_RECORD_TRIPLE
                && !event.branch_taken
                && event.condition_passed == Some(true)
        }));

        let mut inverted_cod = Vec::new();
        let a0_offset = inverted_cod.len();
        inverted_cod.push(0xA0);
        inverted_cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = inverted_cod.len();
        inverted_cod.push(OP_RECORD_TRIPLE);
        inverted_cod.push(0xA1);
        inverted_cod.extend_from_slice(&record.to_le_bytes());
        inverted_cod.extend_from_slice(&0x1064u16.to_le_bytes());
        inverted_cod.extend_from_slice(&0x055Au16.to_le_bytes());
        push_empty_text(&mut inverted_cod);
        let target = inverted_cod.len() as u16;
        inverted_cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut inverted_cod);
        inverted_cod.push(0xFF);

        let trace = execute_trace(&inverted_cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == OP_RECORD_TRIPLE
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(target)
        }));
    }

    #[test]
    fn execution_trace_evaluates_global_word_conditions_with_context() {
        let var = vec![0; 0x20];
        let mut cod = Vec::new();
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        cod.push(OP_GLOBAL_WORD_COMPARE);
        cod.push(0xF1);
        cod.push(0xC1);
        cod.extend_from_slice(&0x0009u16.to_le_bytes());
        let first_text = cod.len();
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let trace = execute_trace(&cod, &var);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert!(
            trace.branch_events.iter().all(|event| {
                event.offset != condition_offset || event.condition_passed.is_none()
            })
        );

        let passing_context = ExecutionContext::default().with_bios_rtc(8, 1, 1);
        let trace = execute_trace_with_context(&cod, &var, &passing_context);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == OP_GLOBAL_WORD_COMPARE
                && !event.branch_taken
                && event.condition_passed == Some(true)
        }));

        let failing_context = ExecutionContext::default().with_global_word_0aa6(0x0009);
        let trace = execute_trace_with_context(&cod, &var, &failing_context);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == OP_GLOBAL_WORD_COMPARE
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(target)
        }));

        let signed_context = ExecutionContext::default().with_global_word_0aa6(0xFFFF);
        assert_eq!(
            global_word_condition(&signed_context, 0xF1, 0x0000),
            Some(true)
        );
    }

    #[test]
    fn execution_trace_evaluates_global_pair_conditions_with_context() {
        let var = vec![0; 0x20];
        let mut cod = Vec::new();
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        cod.push(OP_GLOBAL_PAIR_COMPARE);
        cod.push(0xF1);
        cod.extend_from_slice(&0x0C19u16.to_le_bytes());
        cod.extend_from_slice(&0xBEEFu16.to_le_bytes());
        let first_text = cod.len();
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let passing_context = ExecutionContext::default().with_bios_rtc(0, 12, 24);
        let trace = execute_trace_with_context(&cod, &var, &passing_context);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 2);
        assert_eq!(trace.line_states[0].offset, first_text);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == OP_GLOBAL_PAIR_COMPARE
                && !event.branch_taken
                && event.condition_passed == Some(true)
        }));

        let failing_context = ExecutionContext::default().with_global_pair_0aaa_0aa8(0x0C, 0x19);
        let trace = execute_trace_with_context(&cod, &var, &failing_context);
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.opcode == OP_GLOBAL_PAIR_COMPARE
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.target == Some(target)
        }));

        let signed_context = ExecutionContext::default().with_global_pair_0aaa_0aa8(0x7F, 0xFF);
        assert_eq!(
            global_pair_condition(&signed_context, 0xF1, 0x8000),
            Some(false)
        );
    }

    #[test]
    fn execution_trace_override_branches_successful_condition() {
        let actor = 0x0100u16;
        let location_field = actor + LOCATION_FIELD;
        let mut var = vec![0; 0x0200];
        state_set_u16(&mut var, location_field, 0x1111);

        let mut cod = Vec::new();
        push_actor_ref(&mut cod, actor);
        let a0_offset = cod.len();
        cod.push(0xA0);
        cod.extend_from_slice(&0u16.to_le_bytes());
        let condition_offset = cod.len();
        cod.push(0xC0);
        cod.extend_from_slice(&location_field.to_le_bytes());
        cod.push(0xF5);
        cod.push(0xC1);
        cod.extend_from_slice(&0x1111u16.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xA1);
        let target = cod.len() as u16;
        cod[a0_offset + 1..a0_offset + 3].copy_from_slice(&target.to_le_bytes());
        push_empty_text(&mut cod);
        cod.push(0xFF);

        let trace = execute_trace_with_overrides(
            &cod,
            &var,
            &[BranchOverride {
                offset: condition_offset,
                condition_passed: false,
            }],
        );
        assert_eq!(trace.halted, ExecutionHalt::EndMarker);
        assert_eq!(trace.line_states.len(), 1);
        assert_eq!(trace.line_states[0].offset, target as usize);
        assert!(trace.branch_events.iter().any(|event| {
            event.offset == condition_offset
                && event.branch_taken
                && event.condition_passed == Some(false)
                && event.detail == "condition forced failed"
        }));
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
                active_line_id: text_selector_active_line_id(0x01),
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
                active_line_id: text_selector_active_line_id(0xFF),
                flags_b4: TEXT_LOOP_TARGET_FLAG,
                loop_target: Some(0x1234),
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
        assert!(ev.iter().any(|e| matches!(
            e,
            SceneEvent::DrawSubtitle {
                text,
                active_line_id,
                loop_target,
                ..
            } if text == "there"
                && *active_line_id == text_selector_active_line_id(0xFF)
                && *loop_target == Some(0x1234)
        )));
        assert!(ev.iter().any(|e| matches!(
            e,
            SceneEvent::PlayChatter { active_line_id }
                if *active_line_id == text_selector_active_line_id(0xFF)
        )));
        assert_eq!(
            ev.iter()
                .filter(|e| matches!(e, SceneEvent::PlayVoice { .. }))
                .count(),
            2
        );
        assert_eq!(ev.last(), Some(&SceneEvent::Clear));
    }

    #[test]
    fn emit_scene_events_reports_unresolved_presentation_inputs() {
        let lines = vec![
            LineInput {
                actor: None,
                background_hnm: None,
                background_record: None,
                voice_selector: 0x01,
                active_line_id: text_selector_active_line_id(0x01),
                flags_b4: 0x00,
                clip_index: None,
                text: "missing context".into(),
                ..Default::default()
            },
            LineInput {
                actor: Some("Bob_Morlock".into()),
                background_hnm: Some("petrol10".into()),
                background_music: Some("mus1".into()),
                voice_selector: 0x05,
                active_line_id: text_selector_active_line_id(0x05),
                flags_b4: 0x00,
                clip_index: None,
                text: "missing voice".into(),
                ..Default::default()
            },
            LineInput {
                actor: Some("Bob_Morlock".into()),
                background_hnm: Some("petrol10".into()),
                background_music: Some("mus1".into()),
                voice_selector: 0xFF,
                active_line_id: text_selector_active_line_id(0xFF),
                flags_b4: 0x00,
                clip_index: None,
                text: "silent".into(),
                ..Default::default()
            },
        ];

        let ev = emit_scene_events(&lines);
        assert!(ev.iter().any(|event| matches!(
            event,
            SceneEvent::UnresolvedBackground { active_line_id }
                if *active_line_id == text_selector_active_line_id(0x01)
        )));
        assert!(ev.iter().any(|event| matches!(
            event,
            SceneEvent::UnresolvedActor { active_line_id }
                if *active_line_id == text_selector_active_line_id(0x01)
        )));
        assert_eq!(
            ev.iter()
                .filter(|event| matches!(event, SceneEvent::UnresolvedVoice { .. }))
                .count(),
            1
        );
        assert!(ev.iter().any(|event| matches!(
            event,
            SceneEvent::UnresolvedVoice {
                voice_selector: 0x05,
                active_line_id,
            } if *active_line_id == text_selector_active_line_id(0x05)
        )));
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

    #[test]
    fn execution_trace_reaches_end_marker_for_real_scripts_if_present() {
        for idx in 1..=5 {
            for prefix in ["output/scripts", "../output/scripts"] {
                let cp = format!("{prefix}/SCRIPT{idx}.COD");
                let vp = format!("{prefix}/SCRIPT{idx}.VAR");
                let (Ok(cod), Ok(var)) = (std::fs::read(&cp), std::fs::read(&vp)) else {
                    continue;
                };
                let trace = execute_trace(&cod, &var);
                eprintln!(
                    "SCRIPT{idx}: {} executed lines, {} branch events, {} steps, {:?}",
                    trace.line_states.len(),
                    trace.branch_events.len(),
                    trace.steps,
                    trace.halted
                );
                assert_eq!(trace.halted, ExecutionHalt::EndMarker);
                assert!(
                    !trace.branch_events.is_empty(),
                    "{cp} should exercise branch/control events"
                );
            }
        }
    }

    #[test]
    fn strict_c4_branching_reveals_script2_needs_presentation_setup_if_present() {
        for prefix in ["output/scripts", "../output/scripts"] {
            let cp = format!("{prefix}/SCRIPT2.COD");
            let vp = format!("{prefix}/SCRIPT2.VAR");
            let (Ok(cod), Ok(var)) = (std::fs::read(&cp), std::fs::read(&vp)) else {
                continue;
            };

            let context = ExecutionContext::default().with_strict_actor_record_branching();
            let trace = execute_trace_with_context(&cod, &var, &context);
            assert_eq!(trace.halted, ExecutionHalt::EndMarker);
            assert!(trace.line_states.is_empty());
            assert!(
                trace.branch_events.iter().any(|event| {
                    event.offset == 5
                        && event.opcode == OP_ACTOR
                        && event.branch_taken
                        && event.condition_passed == Some(false)
                        && event.target == Some(722)
                }),
                "strict C4 mode should follow the binary branch-fail path at SCRIPT2 offset 5"
            );
            return;
        }

        eprintln!("skipping: extracted SCRIPT2 files not available");
    }

    /// If the real binary is present, confirm the embedded descriptor table
    /// matches `BLOODPRG.EXE` file offset 0x14338, so the constant can't drift.
    #[test]
    fn text_speed_setting_maps_like_the_init_at_0x1b3a() {
        // Settings 0..4 -> steps {1,2,3,4,7}: ax=setting*2, setting 4 special-cased
        // (+4), then (ax>>1)+1.
        assert_eq!(
            (0..5).map(text_speed_step_from_setting).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 7]
        );
        // Reveal cost: step>>2 frames per char, floored at one frame.
        assert_eq!(reveal_frames_per_char(1), 1);
        assert_eq!(reveal_frames_per_char(4), 1);
        assert_eq!(reveal_frames_per_char(7), 1);
        assert_eq!(reveal_frames_per_char(8), 2);
    }

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

    /// THE FRONTIER'S FIRST ARROW (story-progression map): Scruter_Jo.talk =
    /// record 1860 (C4 operand @0005, read from the COD) — his presentation plays
    /// the CYBERSPACE explanation from the bytecode ('These SCRUT robots use a
    /// psychic structure based on CYBERSPACE...' @02FD, 'you go get BIONIUM in
    /// CYBERSPACE of SCRUTER JO' @038C, the BIOXX->Mantas->BIONIUM loop @04B5..).
    #[test]
    fn script2_scruter_jo_presenter_plays_the_cyberspace_block() {
        let Some(iso) = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .find(|d| std::path::Path::new(d).join("SCRIPT2.COD").is_file())
        else {
            return;
        };
        let cod = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.COD")).unwrap();
        let var = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.VAR")).unwrap_or_default();
        let mut m = VmMachine::new();
        m.load_cod(&cod);
        m.load_var(&var);
        m.presentation_busy = true;
        m.presentation_active = true;
        m.flag_252a = true;
        m.flag_274f = true;
        m.start_actor_presentation(1860, 40);
        m.satisfy_opening_location_guards();
        let mut offsets = Vec::new();
        for _ in 0..400 {
            for ev in m.run_frame() {
                if let VmEvent::Text { offset } = ev {
                    offsets.push(offset);
                }
            }
            if m.halted() {
                break;
            }
        }
        assert!(
            !offsets.is_empty(),
            "Scruter Jo's presenter (1860) emits lines"
        );
        // His cyberspace-explanation block: the @02FD/@038C region lines appear.
        let hits = offsets
            .iter()
            .filter(|&&o| (0x2D0..0x600).contains(&o))
            .count();
        assert!(
            hits > 0,
            "the cyberspace-explanation region (0x2FD..0x38C..) plays (got {offsets:x?})"
        );
    }

    /// THE PLANETS' ENTRY ARROW, locked structurally: Honk's script-select block
    /// (gated scr>5 — C0 record 0x1276 cmp>5 @1221) carries the A3 concept guards
    /// "3"/"4"/"5" (DIC 0xB85..) each followed by its RUN PROFILE token
    /// (@1269/@1284/@129F: D2 operands 3/4/5 -> profiles 2/3/4 = SCRIPT3/4/5) —
    /// the same profile mechanism the port's nav dispatch drives.
    #[test]
    fn script2_script_select_dispatch_structure() {
        let Some(iso) = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .find(|d| std::path::Path::new(d).join("SCRIPT2.COD").is_file())
        else {
            return;
        };
        let cod = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.COD")).unwrap();
        // The scr>5 gate: C0, record 0x1276, cmp-op 0xC1F2, value 5.
        assert_eq!(&cod[0x1221..0x1228], &[0xC0, 0x76, 0x12, 0xF2, 0xC1, 0x05, 0x00]);
        // Each concept guard is followed by its D2 profile run inside the block.
        for (a3_off, d2_off, profile_operand) in
            [(0x1257usize, 0x1269usize, 3u8), (0x1272, 0x1284, 4), (0x128D, 0x129F, 5)]
        {
            assert_eq!(cod[a3_off], 0xA3, "A3 concept guard at {a3_off:#x}");
            assert_eq!(cod[d2_off], 0xD2, "D2 profile run at {d2_off:#x}");
            assert_eq!(
                cod[d2_off + 1],
                profile_operand,
                "profile operand at {d2_off:#x} (sign_extend(op)-1 = profile {})",
                profile_operand - 1
            );
        }
        // The A3 operands resolve to the DIC words "3"/"4"/"5".
        let dic = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.DIC")).unwrap();
        for (a3_off, word) in [(0x1257usize, b"3"), (0x1272, b"4"), (0x128D, b"5")] {
            let opnd =
                u16::from_le_bytes([cod[a3_off + 1], cod[a3_off + 2]]) as usize;
            let end = dic[opnd..].iter().position(|&b| b == 0).unwrap() + opnd;
            assert_eq!(&dic[opnd..end], word, "A3 @{a3_off:#x} word");
        }
    }

    /// ORACLE-LOCKED: the SCRIPT1 boot presenter is HONK (2148, related 40) — the live
    /// game's OCR'd tutorial sequence (tut4_replay.log) plays the [061D] Honk.talk
    /// block at boot: WELCOME ABOARD -> phone -> Cap'n Bob ... -> CLICK ON CRYOBOX.
    /// Izwalito's guidance (1428) is the MENU>EXPLANATIONS replay, not the boot.
    #[test]
    fn script1_boot_presenter_is_honk_oracle_sequence() {
        let Some(iso) = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .find(|d| std::path::Path::new(d).join("SCRIPT1.COD").is_file())
        else {
            return;
        };
        let cod = std::fs::read(std::path::Path::new(iso).join("SCRIPT1.COD")).unwrap();
        let mut m = VmMachine::new();
        m.load_cod(&cod);
        m.presentation_busy = true;
        m.presentation_active = true;
        m.flag_252a = true;
        m.flag_274f = true;
        m.start_actor_presentation(2148, 40);
        m.satisfy_opening_location_guards();
        let mut offsets = Vec::new();
        for _ in 0..400 {
            for ev in m.run_frame() {
                if let VmEvent::Text { offset } = ev {
                    offsets.push(offset);
                }
            }
            if m.halted() {
                break;
            }
        }
        // The Honk boot block's line records ([0628]..[0750] region) must appear
        // in bytecode order, ending with the CRYOBOX instruction at 0x750.
        let expected = [0x628usize, 0x64C, 0x664, 0x68A, 0x6AA, 0x6DA, 0x6F8, 0x714, 0x734, 0x750];
        let mut cursor = 0usize;
        for e in expected {
            let pos = offsets[cursor..].iter().position(|&o| o == e);
            assert!(
                pos.is_some(),
                "boot sequence missing line {e:#x} (got {offsets:x?})"
            );
            cursor += pos.unwrap() + 1;
        }
    }
}
