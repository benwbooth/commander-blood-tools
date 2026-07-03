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
pub const TEXT_SELECTOR_NONE: u8 = 0xFF;
pub const TEXT_SELECTOR_SILENT: u8 = 0x00;
pub const ACTIVE_LINE_ID_BIAS: u16 = 9;
pub const CHATTER_HOLD_EXTRA_TICKS: u16 = 6;
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
    if selector != TEXT_SELECTOR_NONE
        && selector != TEXT_SELECTOR_SILENT
        && one_based <= talk_clip_count
    {
        Some(one_based - 1)
    } else {
        None
    }
}

pub fn text_flags_are_active(flags_b5: u8) -> bool {
    flags_b5 & TEXT_ACTIVE_DISPLAY_FLAG != 0
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
//       mode 0; mode-1 direct compares are evaluated when host state has that
//       concrete record entry. Resolved-table fallback paths remain pending.
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
//       presentation record state, not a speaker change.
//   * 0xC5..=0xC8: record entries. Successful mode-0 writes are guarded per
//       handler (C6 is unconditional; C8 stores zero despite consuming an
//       operand), and mode-1 direct compares are evaluated when host state has a
//       concrete record entry. Guarded mode-0 failure branches still need the
//       fuller line-record table model before execution can be exact.
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
// selector-0x13 C4 record on the related object.
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
const VM_FIELD_OFFSET_SELECTOR_C2: u8 = 0x11;
const VM_FIELD_OFFSET_SELECTOR_C9_RELATED: u8 = 0x13;
const C2_ACTIVE_LINE_KIND2: u16 = 0x27;
const C2_ACTIVE_LINE_KIND400: u16 = 0x2B;
const C2_PRESENTATION_GATE: u16 = 0x1FB2;
const C2_PRESENTATION_FLAGS: u16 = 0x67AA;
const C2_PRESENTATION_BUSY_FLAG: u8 = 0x02;
const VM_ACTIVE_LINE: u16 = 0x6788;
const C9_PRESENTATION_GATE_A: u16 = 0x252A;
const C9_PRESENTATION_GATE_B: u16 = 0x2531;
const C4_POST_UPDATE_SENTINEL: u16 = 0xFFFF;

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
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExecutionContext {
    object_offsets: Vec<u16>,
    special_object_offset: Option<u16>,
    global_word_0aa6: Option<u16>,
    global_pair_0aaa_0aa8: Option<(u8, u8)>,
    descript_entry_names: Vec<Vec<u8>>,
    text_presentation_record_gating: bool,
    text_line_display_gating: bool,
    strict_actor_record_branching: bool,
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

fn post_update_actor_record_pair(
    state: &mut [u8],
    owner_offset: u16,
    record_offset: u16,
) -> Option<u16> {
    if state_u16(state, record_offset) != OP_ACTOR as u16
        || state_u16(state, record_offset.wrapping_add(4)) != 0
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

fn write_c1_record_state_direct(
    state: &mut [u8],
    context: &ExecutionContext,
    record_offset: u16,
    operand: u16,
) -> bool {
    if record_owner_is_active(state, context, record_offset) != Some(true) {
        return false;
    }
    if state_u16(state, record_offset) != 0 {
        return false;
    }
    state_set_u16(state, record_offset, OP_RECORD_STATE_MIN as u16);
    state_set_u16(state, record_offset.wrapping_add(2), operand);
    state_set_u16(state, record_offset.wrapping_add(4), 2);
    true
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
            write_c1_record_state_direct(&mut state, context, record_offset, operand);
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
                        flags_b5,
                        ..
                    },
                    next,
                )) => {
                    if text_runtime_gates_allow(&state, context, line_index, flags_b5) {
                        if context.text_line_display_gating {
                            mark_text_line_shown(&mut state, line_index);
                        }
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
    const STEP_LIMIT_MULTIPLIER: usize = 64;

    let mut state = var.to_vec();
    let mut actor: Option<u16> = None;
    let mut line_states = Vec::new();
    let mut branch_events = Vec::new();
    let mut script_profile_requests = Vec::new();
    let mut branch_stack: Vec<u16> = Vec::new();
    let mut special_slots = SpecialObjectSlots::default();
    let mut pos = 0usize;
    let mut mode1 = false;
    let end = cod.len();
    let step_limit = end.saturating_mul(STEP_LIMIT_MULTIPLIER).max(1024);
    let mut steps = 0usize;
    let mut halted = ExecutionHalt::EndMarker;

    while pos < end {
        if steps >= step_limit {
            halted = ExecutionHalt::StepLimit { limit: step_limit };
            break;
        }
        steps += 1;

        let token_start = pos;
        let op = cod[token_start];
        if op == 0xFF {
            halted = ExecutionHalt::EndMarker;
            break;
        }
        if !(OP_MIN..=OP_MAX).contains(&op) {
            halted = ExecutionHalt::InvalidOpcode {
                offset: token_start,
                byte: op,
            };
            break;
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
                break;
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
                    break;
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
            script_profile_requests.push(ScriptProfileRequestEvent {
                offset: token_start,
                operand,
                profile_index: script_profile_index_from_request_operand(operand),
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
                break;
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
            if record_owner_is_active(&state, context, record_offset) == Some(true)
                && state_u8(&state, related_record_offset.wrapping_add(2)) & 1 != 0
                && state_u16(&state, record_offset) != OP_ACTOR as u16
            {
                write_record_link(&mut state, record_offset, related_record_offset);
            }
        }
        if !mode1 && is_record_entry_opcode(op) {
            let record_offset = read_u16(cod, token_start + 1).unwrap_or(0);
            let operand = read_u16(cod, token_start + 3).unwrap_or(0);
            write_record_entry_mode0(&mut state, op, record_offset, operand);
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
            write_c1_record_state_direct(&mut state, context, record_offset, operand);
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
                        flags_b5,
                        ..
                    },
                    next,
                )) => {
                    if text_runtime_gates_allow(&state, context, line_index, flags_b5) {
                        if context.text_line_display_gating {
                            mark_text_line_shown(&mut state, line_index);
                        }
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
                    break;
                }
                _ => unreachable!("decode_text only returns TEXT tokens"),
            }
        }
        if VAR_TERMINATED.contains(&op) {
            pos = scan_zero_word(cod, token_start + 1, end);
            continue;
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
            (if mode1 { b1 } else { b0 } as usize).max(1)
        };
        pos = (token_start + len).min(end);
    }

    ExecutionTrace {
        line_states,
        branch_events,
        script_profile_requests,
        steps,
        halted,
    }
}

pub fn execute_script_profile_sequence(
    programs: &[ScriptProfileProgram<'_>],
    initial_profile_index: u16,
    run_limit: usize,
) -> ScriptProfileExecution {
    let mut runs = Vec::new();
    let mut next_profile_index = initial_profile_index;

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

        let trace = execute_trace_with_context(program.cod, program.var, &program.context);
        let pending = trace.pending_script_profile();
        runs.push(ScriptProfileRun {
            run_index,
            profile_index: program.profile_index,
            trace,
        });

        match pending {
            Some(profile_index) => next_profile_index = profile_index,
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
        flags: u8,
    },
    /// Subtitle chatter event from the dialogue display state machine (tb.snd).
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
        let var0 = vec![0; 0x20];
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
        assert_eq!(text_selector_voice_clip_index(0x00, 4), None);
        assert_eq!(text_selector_voice_clip_index(0xFF, 4), None);
        assert_eq!(text_selector_voice_clip_index(0x01, 4), Some(0));
        assert_eq!(text_selector_voice_clip_index(0x04, 4), Some(3));
        assert_eq!(text_selector_voice_clip_index(0x05, 4), None);
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

        let context =
            ExecutionContext::from_object_offsets([special_object, owner, 0x0300])
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
        assert!(trace.branch_events.iter().all(|event| {
            event.offset != condition_offset || event.condition_passed.is_none()
        }));

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
