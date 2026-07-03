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
pub const OP_RECORD_LINK: u8 = 0xC3;
pub const OP_ACTOR: u8 = 0xC4;
pub const OP_RECORD_ENTRY_MIN: u8 = 0xC5;
pub const OP_RECORD_ENTRY_MAX: u8 = 0xC8;
pub const OP_RECORD_CLEAR: u8 = 0xC9;
pub const TEXT_SELECTOR_NONE: u8 = 0xFF;
pub const TEXT_SELECTOR_SILENT: u8 = 0x00;
pub const ACTIVE_LINE_ID_BIAS: u16 = 9;
pub const CHATTER_HOLD_EXTRA_TICKS: u16 = 6;

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

pub fn is_record_entry_opcode(opcode: u8) -> bool {
    (OP_RECORD_ENTRY_MIN..=OP_RECORD_ENTRY_MAX).contains(&opcode)
}

pub fn record_entry_stored_related_offset(opcode: u8, operand: u16) -> u16 {
    if opcode == 0xC8 { 0 } else { operand }
}

/// `0xB7` addresses bits high-bit-first inside each byte: bit 0 is mask 0x80,
/// bit 7 is mask 0x01, then bit 8 starts the next byte at mask 0x80.
pub fn bit_flag_byte_offset(base_offset: u16, bit_index: u8) -> u16 {
    base_offset.wrapping_add((bit_index >> 3) as u16)
}

pub fn bit_flag_mask(bit_index: u8) -> u8 {
    0x80u8 >> (bit_index & 7)
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
        } else if op == OP_RECORD_LINK {
            let record_offset = read_u16(cod, pos + 1).unwrap_or(0);
            let related_record_offset = read_u16(cod, pos + 3).unwrap_or(0);
            out.push(VmToken::RecordLink {
                offset: pos,
                record_offset,
                related_record_offset,
                len,
            });
        } else if is_record_entry_opcode(op) {
            let record_offset = read_u16(cod, pos + 1).unwrap_or(0);
            let operand = read_u16(cod, pos + 3).unwrap_or(0);
            out.push(VmToken::RecordEntry {
                offset: pos,
                entry_opcode: op,
                record_offset,
                operand,
                stored_related_offset: record_entry_stored_related_offset(op, operand),
                aux_word: 0,
                len,
            });
        } else if op == OP_ACTOR {
            let record_offset = read_u16(cod, pos + 1).unwrap_or(0);
            let related_record_offset = read_u16(cod, pos + 3).unwrap_or(0);
            out.push(VmToken::Actor {
                offset: pos,
                record_offset,
                related_record_offset,
                len,
            });
        } else if op == OP_RECORD_CLEAR {
            let record_offset = read_u16(cod, pos + 1).unwrap_or(0);
            out.push(VmToken::RecordClear {
                offset: pos,
                record_offset,
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
//       (`state[op2]`). Writes `state[op1]` in mode 0 only.
//   * 0x6902 family (AE/B0), 5 bytes plus optional A1 prefix:
//       set/clear a bit mask in `state[op1]` in mode 0.
//   * 0x6946 family (AD/AF/B2/B3/BA/BB/BC), 5 bytes:
//       direct `state[op1] = op2` in mode 0. The DOS handler also updates
//       table-side bookkeeping for sentinel object values; that side effect is
//       not needed for line-location recovery and is not modeled here.
//   * 0xB7, 4 bytes plus optional A1 prefix:
//       set/clear/test one high-bit-first byte flag in the state area.
//   * 0xC4: actor/record reference. The first operand is the destination record
//       offset and doubles as object_offset + 0x3A (talk field) for speaker
//       tracking; the second operand is a related record offset consumed by the
//       DOS handler.
//   * 0xC3: record link. The handler writes {0x00C3, related, 1}; this is
//       presentation record state, not a speaker change.
//   * 0xC5..=0xC8: record entries. The handlers write {opcode, related, 0};
//       C8 is the special empty-record marker and stores related=0.
//   * 0xC9: record clear. The handler zeroes a 6-byte record and, when the
//       previous entry was 0xC4, clears the related actor subrecord too.
// NOTE: this is a LINEAR pass — it does not yet evaluate the 0xAF-family
// conditionals/branches; branch-mode comparison handlers are intentionally
// treated as non-mutating until the PC/branch helper at 0x6462 is modeled.
// Adequate for deterministic cutscene runs; see REVERSE.md for the caveat.

const ASSIGN_7: [u8; 7] = [0xB1, 0xB4, 0xB5, 0xB6, 0xBE, 0xBF, 0xC0];
const BITMASK_5: [u8; 2] = [0xAE, 0xB0];
const ASSIGN_5: [u8; 7] = [0xAD, 0xAF, 0xB2, 0xB3, 0xBA, 0xBB, 0xBC];
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
    pub steps: usize,
    pub halted: ExecutionHalt,
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
            if let Some(record_offset) = read_u16(cod, pos + 1) {
                actor = Some(record_offset.wrapping_sub(TALK_FIELD));
            }
        }
        if op == OP_RECORD_CLEAR {
            if let Some(record_offset) = read_u16(cod, pos + 1) {
                if actor.map(|a| a.wrapping_add(TALK_FIELD)) == Some(record_offset) {
                    actor = None;
                }
            }
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
            state_set_u16(&mut state, op1, value);
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

/// Execute the subset of VM control flow that has been tied to concrete handler
/// code. This follows A0/A1 condition blocks and direct A4/A9 jumps, while still
/// using the same bounded state model as `interpret_line_states`.
pub fn execute_trace(cod: &[u8], var: &[u8]) -> ExecutionTrace {
    execute_trace_with_overrides(cod, var, &[])
}

/// Execute a concrete VM path, optionally forcing selected condition outcomes.
/// Overrides are keyed by conditional opcode offset and are applied only after a
/// real condition has been decoded at that offset.
pub fn execute_trace_with_overrides(
    cod: &[u8],
    var: &[u8],
    overrides: &[BranchOverride],
) -> ExecutionTrace {
    const STEP_LIMIT_MULTIPLIER: usize = 64;

    let mut state = var.to_vec();
    let mut actor: Option<u16> = None;
    let mut line_states = Vec::new();
    let mut branch_events = Vec::new();
    let mut branch_stack: Vec<u16> = Vec::new();
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
                let value = read_u16(cod, p + 2).unwrap_or(0);
                // The DOS handler maps RHS == gs:0x674e to 0xffff before this
                // compare. `execute_trace` does not yet receive that runtime
                // special-object value, so inspected traces report the direct
                // equality result until the object-table model is wired in.
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

        if op == OP_ACTOR {
            if let Some(record_offset) = read_u16(cod, token_start + 1) {
                actor = Some(record_offset.wrapping_sub(TALK_FIELD));
            }
        }
        if op == OP_RECORD_CLEAR {
            if let Some(record_offset) = read_u16(cod, token_start + 1) {
                if actor.map(|a| a.wrapping_add(TALK_FIELD)) == Some(record_offset) {
                    actor = None;
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
            state_set_u16(&mut state, op1, value);
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

        if op == OP_TEXT {
            let location_offset = actor.map(|a| state_u16(&state, a.wrapping_add(LOCATION_FIELD)));
            line_states.push(LineState {
                offset: token_start,
                actor_offset: actor,
                location_offset,
            });
            match decode_text(cod, token_start, end) {
                Some((_, next)) => {
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
        steps,
        halted,
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

    fn push_empty_text(cod: &mut Vec<u8>) {
        cod.extend_from_slice(&[OP_TEXT, 0x00, 0x00, 0xff, 0x00, 0x80]);
        cod.extend_from_slice(&0u16.to_le_bytes());
    }

    fn push_record_clear(cod: &mut Vec<u8>, actor_offset: u16) {
        let record_offset = actor_offset.wrapping_add(TALK_FIELD);
        cod.push(OP_RECORD_CLEAR);
        cod.extend_from_slice(&record_offset.to_le_bytes());
    }

    /// Build a tiny synthetic COD: a 1-byte op, an A6 text token (no loop), an
    /// A6 text token (with loop bit), then the 0xFF end marker.
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
    fn actor_token_exposes_both_binary_operands() {
        let cod = [OP_ACTOR, 0x84, 0x00, 0x28, 0x00, 0xFF];

        let toks = walk(&cod, 0, cod.len());
        assert_eq!(
            toks[0],
            VmToken::Actor {
                offset: 0,
                record_offset: 0x0084,
                related_record_offset: 0x0028,
                len: 5
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
                len: 5
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
                len: 5
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
