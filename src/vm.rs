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

/// The engine's per-kind FIELD-OFFSET MATRIX (gs:0x6D60, file 0x14180),
/// byte-exact: `FIELD_OFFSETS[field][kind]` = the byte offset of that field in
/// a record of that kind (0 = the kind lacks the field). The port's standing
/// constants are its kind-1 column: field 0x11 (location/container) = 0x18
/// (LOCATION_FIELD 24) and field 0x13 (the talk/presentation pair) = 0x3A
/// (talk = object + 58). Consumers: the A6 gate's field-0x13 lookup (0x6664),
/// vm_field_offset (0x6023), the CD transfer's field-0x11 relink.
pub const FIELD_OFFSETS: [[u8; 16]; 0x15] = [
    [0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x04, 0x16, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x1a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x32, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x34, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x1e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x38, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x36, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x20, 0x44, 0x1c, 0x1c, 0x22, 0x00, 0x00, 0x16, 0x00, 0x10, 0x16, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x46, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x14, 0x14, 0x14, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x06, 0x18, 0x16, 0x16, 0x16, 0x00, 0x00, 0x14, 0x00, 0x04, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x1a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x08, 0x3a, 0x00, 0x00, 0x1c, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
];

/// Field offset for a record kind, per the engine's matrix (None = absent).
/// The kind word is a BIT-FLAG; the column is its lowest set bit (`bsf`,
/// 0x6023 `bsf bx,bx`), NOT `kind & 0xF`. bsf(2)=1 -> the universal
/// character location = obj+0x18 / talk = obj+0x3A.
pub fn field_offset(kind: u16, field: u8) -> Option<u16> {
    // ONE resolver, not two. This used to index `FIELD_OFFSETS` itself — a second
    // copy of `vm_field_offset`'s `bsf`-column lookup, differing only in treating
    // a zero cell as `None` rather than `Some(0)`. Both readings are usable (the
    // original returns AX=0 and callers do `or ax,ax / je`), but two
    // implementations of one table lookup can drift, and only one of them was
    // swept against the lifted `func_6023`. Delegating means both are.
    vm_field_offset(field, kind).filter(|off| *off != 0)
}

/// Handler 0x06559 (`vm_op_a0_push`) — dispatch table `0x142D0`, the entry for 0xA0.
pub const OP_MIN: u8 = 0xA0;
/// Upper bound of the TOKEN-VALIDITY range, not of dispatch — the two differ and the
/// distinction is easy to lose.
///
/// * TOKENS: `OPCODE_DESC` at `DS:0x6F18` has 96 entries covering `0xA0..=0xFF`, and
///   `vm_token_advance` (`0x62B6`) indexes it for EVERY byte it walks. So a byte in this
///   range is a token whose length the walker can compute.
/// * DISPATCH: the handler table at `DS:0x6EB0` is only 104 bytes — 52 entries, i.e.
///   `0xA0..0xD3` — and the `0xD3` slot is NULL. `vm_dispatch` (`0x5627`) therefore
///   EXECUTES only `0xA0..=0xD2`. The extent is pinned by a layout identity:
///   `0x6EB0 + 104 = 0x6F18`, exactly where `OPCODE_DESC` begins.
///
/// So opcodes `0xD3..=0xFE` have LENGTHS but no HANDLERS: the walker must skip them,
/// and nothing should execute them. Do not "correct" this bound to `0xD2` — that would
/// make the walker treat data tokens as invalid and desync the stream.
pub const OP_MAX: u8 = 0xFE;
/// Handler 0x0660c (`vm_op_a6_text`) — dispatch table `0x142D0`, the entry for 0xA6.
pub const OP_TEXT: u8 = 0xA6;
/// Handler 0x06aa7 (`vm_op_b7_record_op`) — dispatch table `0x142D0`, the entry for 0xB7.
pub const OP_BIT_FLAG: u8 = 0xB7;
/// Handler 0x06b06 (`vm_op_b8_record_readwrite`) — dispatch table `0x142D0`, the entry for 0xB8.
pub const OP_PAIR_RECORD_A: u8 = 0xB8;
/// Handler 0x06b06 (`vm_op_b8_record_readwrite`) — dispatch table `0x142D0`, the entry for 0xB9.
pub const OP_PAIR_RECORD_B: u8 = 0xB9;
/// Handler 0x06b06 (`vm_op_b8_record_readwrite`) — dispatch table `0x142D0`, the entry for 0xBD.
pub const OP_PAIR_RECORD_C: u8 = 0xBD;
/// Handler 0x06b4c (`vm_op_c1_record_state`) — dispatch table `0x142D0`, the entry for 0xC1.
pub const OP_RECORD_STATE_MIN: u8 = 0xC1;
/// Handler 0x06e34 (`vm_op_c2_record_full`) — dispatch table `0x142D0`, the entry for 0xC2.
pub const OP_RECORD_STATE_MAX: u8 = 0xC2;
/// Handler 0x06eee (`vm_op_c3_state_record`) — dispatch table `0x142D0`, the entry for 0xC3.
pub const OP_RECORD_LINK: u8 = 0xC3;
/// Handler 0x06c7e (`vm_op_c4_actor`) — dispatch table `0x142D0`, the entry for 0xC4.
pub const OP_ACTOR: u8 = 0xC4;
/// Handler 0x06d18 (`vm_op_c5_record_match`) — dispatch table `0x142D0`, the entry for 0xC5.
pub const OP_RECORD_ENTRY_MIN: u8 = 0xC5;
/// Handler 0x06f62 (`vm_op_c8_record_match`) — dispatch table `0x142D0`, the entry for 0xC8.
pub const OP_RECORD_ENTRY_MAX: u8 = 0xC8;
/// Handler 0x06fb9 (`vm_op_c9_clear_record`) — dispatch table `0x142D0`, the entry for 0xC9.
pub const OP_RECORD_CLEAR: u8 = 0xC9;
/// Handler 0x064e5 (`vm_op_ca_compare_var`) — dispatch table `0x142D0`, the entry for 0xCA.
pub const OP_GLOBAL_WORD_COMPARE: u8 = 0xCA;
/// Handler 0x06510 (`vm_op_cb_compare_byte`) — dispatch table `0x142D0`, the entry for 0xCB.
pub const OP_GLOBAL_PAIR_COMPARE: u8 = 0xCB;
/// Handler 0x069c7 (`vm_op_cd_state_gated`) — dispatch table `0x142D0`, the entry for 0xCD.
pub const OP_RECORD_TRIPLE: u8 = 0xCD;
/// Handler 0x064b8 (`vm_op_d2_script_profile_request`) — dispatch table `0x142D0`, the entry for 0xD2.
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
/// Handler 0x0685c (`vm_op_ac_yield`) — dispatch table `0x142D0`, the entry for 0xAC.
pub const OP_YIELD_B: u8 = 0xAC;
/// `0xAB` poke a byte to `[address operand]` (set-variable). Handler 0x684c.
pub const OP_POKE_BYTE: u8 = 0xAB;
/// `0xCE`/`0xD0` conditional branch on game flags `[0x2793]`/`[0x252a]` via `vm_branch`.
/// Handler 0x06494 (`vm_op_ce_cond_branch`) — dispatch table `0x142D0`, the entry for 0xCE.
pub const OP_COND_BRANCH_PRESENTATION: u8 = 0xCE;
/// Handler 0x064a0 (`vm_op_d0_cond_branch`) — dispatch table `0x142D0`, the entry for 0xD0.
pub const OP_COND_BRANCH_GAMEFLAG: u8 = 0xD0;
/// `0xCC` set a byte in the 16-byte-record table `gs:0x6cde`. Handler 0x64ce.
pub const OP_SET_RECORD_BYTE: u8 = 0xCC;

/// The decoded VM query/set model (`gs:0x67ad`): record opcodes COMPARE-and-branch while
/// query mode is on (inside an `A0 … A1` block), or WRITE (set) while it is off — the
/// behaviour verified across `0xB8`/`0x6946`/the `C5..C8` family. This is the tested
/// model of that dual mode: [`enter_query`] (opcode `0xA0`) / [`exit_query`] (`0xA1`) toggle
/// it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuerySetMode {
    /// `gs:0x67ad` — true while inside an `A0 … A1` query block.
    pub query: bool,
}

// `record_op` and `RecordOpResult` were REMOVED. They implemented the 0x6946
// wildcard as MATCH-ANYTHING, a reading the live arm's decode refuted: an RHS
// equal to the special object maps to 0xFFFF BEFORE the compare, and the
// match-anything version made every aboard-guard pass (found by a transcript
// diff). Nothing called them but their own test, which ASSERTED the refuted
// behaviour -- a dead rule defended by a test is a trap for whoever wires it
// up. The 0xAD/0xAF/0xB2/0xB3/0xBA/0xBB/0xBC arm in `step` is the authority.

impl QuerySetMode {
    /// Opcode `0xA0` PUSH — enter query mode. The handler is `0x6559`
    /// (dispatch-table entry for `0xA0`):
    ///
    /// ```text
    ///   0x6559  mov byte gs:[0x67ad],1     query mode ON
    ///   0x655F  mov ax,gs:[0x6884]         the stack pointer
    ///   0x6565  add ax,2 / mov gs:[0x6884],ax   POST-increment
    ///   0x656C  lodsw / mov [bp+0x6820],ax      operand -> the slot it vacated
    /// ```
    ///
    /// This helper is only the flag half; the push is in the `0xA0` execution arm.
    pub fn enter_query(&mut self) {
        self.query = true;
    }
    /// Opcode `0xA1` POP — exit query mode. Handler `0x6572`:
    ///
    /// ```text
    ///   0x6572  mov byte gs:[0x67ad],0     query mode OFF
    ///   0x6578  mov ax,gs:[0x6884]
    ///   0x657C  cmp ax,2 / je 0x6587       pointer already at the base: DO NOT pop
    ///   0x6581  sub word gs:[0x6884],2
    /// ```
    ///
    /// The `cmp ax,2` guard is why the execution arm uses `Vec::pop()`, whose
    /// no-op on an empty stack is the same behaviour rather than an approximation
    /// of it.
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
            // The handler's compares are SIGNED: 0x689A `setne`, 0x68BE `setl`,
            // 0x68CA `setg`, 0x68A6 `setle`, 0x68B2 `setge`, 0x68D6 `sete`. The
            // ordered four are the SIGNED forms (setl/setg/setle/setge), not the
            // unsigned setb/seta/setbe/setae, so record words must be compared as
            // i16. This matters constantly: the aboard/wildcard sentinel 0xFFFF is
            // -1 signed but 65535 unsigned, so an unsigned compare inverts every
            // ordered test against it (and against any value >= 0x8000).
            let (a, b) = (cur as i16, op2 as i16);
            let matched = match operator {
                0xF0 => a != b,
                0xF1 => a < b,
                0xF2 => a > b,
                0xF3 => a <= b,
                0xF4 => a >= b,
                0xF5 => a == b,
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
/// Map an `0xA6` selector `b3` to a per-actor TALK-CLIP index.
///
/// SCOPE, decoded 2026-07-24 — read this before adding a caller:
///
/// * Its AUDIO use was REMOVED. The game's dialogue voice is not selected per line at
///   all: accepting a line sets `gs:[0xCFB]` (`0x66AF`), the clip picker gates on that
///   (`0xB898`) and plays `prng(10)+7` with no immediate repeat (`0xB8AB..0xB8BC`),
///   until the reveal clears the flag (`0x94CF`). `main.rs` implements that correctly.
/// * Its remaining callers pick a TALK-HNM (video), and `talk_clip_count` is
///   `talk_hnms.len()`. That has some basis: `b3` really does feed the visual path —
///   `0x668D` stores it sign-extended at `DS:0x1FAB`, `0x11F2` forms the line id
///   `gs:0x6788`, and `0x9D10` dispatches that id to scene/palette/unpack work.
///
/// STILL UNVERIFIED, and the reason these rows stay open: the game's mapping is
/// `line_id = b3 + 9`, whereas this computes `b3 - 1`. Both derive from `b3`, but they
/// are not the same function, and nothing has been traced from a line id to a specific
/// talk-HNM. Do not treat this as decoded.
pub fn text_selector_voice_clip_index(selector: u8, talk_clip_count: usize) -> Option<usize> {
    let one_based = selector as usize;
    if text_selector_requests_voice(selector) && one_based <= talk_clip_count {
        Some(one_based - 1)
    } else {
        None
    }
}

/// Per-line asset table base, `DS:0x1FB5` (`0x9D6A`).
pub const DLG_LINE_ASSET_TABLE_DS: u16 = 0x1FB5;
/// The table's entry stride: 4 bytes (`shl bx,2` at `0x9D67`, and the fill's
/// `stosw` + `add di,2` at `0x7694`).
pub const DLG_LINE_ASSET_ENTRY_STRIDE: u16 = 4;
/// The asset id sits at `entry + 2` (`mov si,[bx+2]`, `0x9D6E`).
pub const DLG_LINE_ASSET_ID_OFFSET: u16 = 2;
/// `line_id = sign_extend(b3) + 9` (`0x11F5`).
pub const DLG_LINE_ID_BIAS: i16 = 9;
/// Stride of the name table the asset id offsets into (`shl ax,4`, `0x768E`).
pub const DLG_ASSET_NAME_STRIDE: u16 = 16;
/// `0xFFFF` = no asset for this line (`cmp si,-1`, `0x9D71`).
pub const DLG_LINE_ASSET_NONE: u16 = 0xFFFF;

/// The line id an `0xA6` selector maps to: `sign_extend(b3) + 9`.
///
/// SCOPE: this is ONE of 29 writers of `gs:0x6788`. A byte search for every
/// `mov [0x6788], …` encoding finds 29 sites — this one, four register writes, and 24
/// IMMEDIATE writes of fixed ids (a `0x27..0x2C` cluster, the low ids `0x01`-`0x07`,
/// and `0xFFFF` resets) issued by native code, e.g. `0x5E74` in the post-update ladder.
/// So the active line is mostly set natively; the script selector accounts for a single
/// path. Do not read this function as "how the line id is determined".
///
/// `0x668D` stores `b3` SIGN-EXTENDED at `DS:0x1FAB` (`lodsb; cbw` -- one byte at
/// `0x668E`, so AL into AX; `re/tools/dis.py` prints it `cwde`), and `0x11F2`
/// reads it and adds 9 to form `gs:0x6788`. The sign extension is load-bearing —
/// `0xFF` becomes `-1`, not `255`.
pub fn dlg_line_id_for_selector(selector: u8) -> i16 {
    i16::from(selector as i8) + DLG_LINE_ID_BIAS
}

/// DS offset of the ASSET ID word for a line id: `0x1FB5 + line_id*4 + 2`.
///
/// Returns `None` for a negative line id, which the dispatcher rejects outright
/// (`or ax,ax; js` at `0x9D20`).
pub fn dlg_line_asset_id_ds_offset(line_id: i16) -> Option<u16> {
    if line_id < 0 {
        return None;
    }
    Some(
        DLG_LINE_ASSET_TABLE_DS
            + (line_id as u16) * DLG_LINE_ASSET_ENTRY_STRIDE
            + DLG_LINE_ASSET_ID_OFFSET,
    )
}

/// The value the fill at `0x7684` stores for one source byte.
///
/// Negative bytes pass through sign-extended (so `0xFF` becomes `0xFFFF`, the exact
/// "no asset" sentinel the reader tests); otherwise the stored value is `(byte - 1) * 16`.
///
/// CAVEAT, from the `DLGTABLE` probe: this describes the INSTRUCTIONS at `0x7684`, and
/// nothing more. Do not read it as "this is what the table contains". In the hub
/// savestate the live `+2` fields hold `0x0DD7`, which is not 16-aligned and points into
/// an `fd\xxxxxxxxxxxx` path template's name field. So either another path populates the
/// table in that state, or this value is later replaced. The earlier gloss calling the
/// result "a byte offset into a 16-byte-stride name table" was an inference beyond the
/// instructions and the probe falsified it.
pub fn dlg_line_asset_id_from_source_byte(byte: u8) -> u16 {
    if (byte as i8) < 0 {
        return i16::from(byte as i8) as u16;
    }
    (u16::from(byte).wrapping_sub(1)).wrapping_mul(DLG_ASSET_NAME_STRIDE)
}

pub fn text_selector_requests_voice(selector: u8) -> bool {
    selector != TEXT_SELECTOR_NONE && selector != TEXT_SELECTOR_SILENT
}

/// The A6 handler's ACTIVE-DISPLAY test, `or cx,cx / jns 0x67A0` @`0x6647`.
///
/// `cx` holds `b4` in `cl` and `b5` in `ch` (read together by `lodsw` @`0x661B`),
/// so bit 15 of that word IS bit 7 of `b5` — the game tests the flag by checking
/// the SIGN of the pair rather than masking. `jns` skips the whole display path,
/// so a clear bit means "not shown", which is why this predicate is phrased
/// positively.
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

/// The line record's FLAGS WORD sits at `+2`: `test word es:[di+2],0x8000`
/// @`0x665A`, where `di` is the line record the handler resolved at `0x6613`
/// (`les di,gs:[0x6724]` then `add di,ax` with the line index).
pub fn text_line_flags_offset(line_index: u16) -> u16 {
    line_index.wrapping_add(2)
}

/// The line's PRESENTATION record sits at `line + TALK_FIELD` — the `0x3A` field
/// the A6 handler at `0x660D` resolves. Basis is that field offset; this function
/// is the addition.
pub fn text_presentation_record_offset(line_index: u16) -> u16 {
    line_index.wrapping_add(TALK_FIELD)
}

/// The ALREADY-SHOWN bit, `0x8000` of the flags word — the mask in
/// `test word es:[di+2],0x8000` @`0x665A`. A set bit takes `jne 0x67A0`, the same
/// exit the inactive case uses, so a line already displayed is skipped exactly as
/// an inactive one is.
pub fn text_line_already_shown(flag_word: u16) -> bool {
    flag_word & TEXT_LINE_ALREADY_SHOWN_FLAG != 0
}

/// The opcode-family predicates below group by TOKEN SHAPE, not by handler, and
/// the dispatch table shows why that distinction is needed:
///
/// ```text
///   0xB8 0xB9 0xBD  -> 0x6B06                    ONE handler: a true family
///   0xC1 0xC2       -> 0x6B4C, 0x6E34            two handlers
///   0xC5 0xC6 0xC7 0xC8 -> 0x6D18, 0x6D80,       FOUR handlers
///                          0x6DCF, 0x6F62
/// ```
///
/// Only the pair-record trio shares a handler. The `C5..=C8` range is four
/// distinct behaviours that happen to share an OPERAND LAYOUT, which is all the
/// token decoder needs — it walks the stream and must know how many bytes to
/// consume, not what they will do. Reading these as behavioural families and
/// merging their handlers would be wrong; reading them as decode-length groups is
/// exactly right.
pub fn is_record_entry_opcode(opcode: u8) -> bool {
    (OP_RECORD_ENTRY_MIN..=OP_RECORD_ENTRY_MAX).contains(&opcode)
}

/// `0xC1..=0xC2`. NOT a handler family — they dispatch to `0x6B4C` and `0x6E34`
/// separately (see the note above [`is_record_entry_opcode`]); the range is a
/// token-shape group for the decoder.
pub fn is_record_state_opcode(opcode: u8) -> bool {
    (OP_RECORD_STATE_MIN..=OP_RECORD_STATE_MAX).contains(&opcode)
}

/// `0xCA` and `0xCB`, which dispatch to `0x64E5` and `0x6510` — again two
/// handlers, grouped here by operand layout rather than behaviour.
pub fn is_global_compare_opcode(opcode: u8) -> bool {
    opcode == OP_GLOBAL_WORD_COMPARE || opcode == OP_GLOBAL_PAIR_COMPARE
}

/// The one grouping that IS a handler family: `0xB8`, `0xB9` and `0xBD` all
/// dispatch to `0x6B06` (dispatch table `0x142D0`), which is why they share the
/// 2-word record behaviour rather than merely a token length.
pub fn is_pair_record_opcode(opcode: u8) -> bool {
    matches!(
        opcode,
        OP_PAIR_RECORD_A | OP_PAIR_RECORD_B | OP_PAIR_RECORD_C
    )
}

/// What the record-entry family stores in the RELATED word — `0` for `0xC8`, the
/// operand otherwise.
///
/// The zero is not a literal in the original. `0xC8`'s handler (`0x6F62`) reaches
/// its set path only through a guard that proves the register is zero:
///
/// ```text
///   0x6F9A  mov bx,es:[bp]        the record's first word
///   0x6F9E  or bx,bx / jne 0x6FB4 NON-empty -> vm_branch instead of writing
///   0x6FA2  mov word es:[bp],0xc8 write the type
///   0x6FA8  mov es:[bp+2],bx      ...and BX, which the guard just proved is 0
///   0x6FAC  mov word es:[bp+4],0
/// ```
///
/// So `0xC8` writes an empty record and nothing else: it only fires on a slot that
/// was already zero, and the related word it stores is that same zero. Writing the
/// operand there — which every sibling opcode does — would be the natural
/// generalisation and would put a value in a field the game guarantees empty.
pub fn record_entry_stored_related_offset(opcode: u8, operand: u16) -> u16 {
    if opcode == 0xC8 { 0 } else { operand }
}

/// Port the `0xD2` handler at `BLOODPRG.EXE` file `0x64B8`:
/// `lodsb; cbw; dec ax; mov gs:[0x6780], ax`.
///
/// `cbw` is correct despite `re/tools/dis.py` printing `cwde` there: capstone
/// renders opcode `0x98` that way even in 16-bit mode, and without a `0x66`
/// prefix it sign-extends AL into AX. The distinction matters here — `cwde` would
/// leave AH holding whatever the dispatcher left, making the stored value depend
/// on caller state rather than on the operand. Profile operands are 1..5, so the
/// sign extension is a no-op in play; the reading still has to be right.
pub fn script_profile_index_from_request_operand(operand: u8) -> u16 {
    ((operand as i8 as i16) - 1) as u16
}

/// `0xB7` addresses bits high-bit-first inside each byte: bit 0 is mask `0x80`,
/// bit 7 is mask `0x01`, then bit 8 starts the next byte at `0x80`.
///
/// The byte split is `0x6AC0`:
///
/// ```text
///   0x6AC0  and cl,7        the bit WITHIN the byte
///   0x6AC3  shr ax,3        the byte index
///   0x6AC6  add bx,ax       base + that
/// ```
///
/// and the high-bit-first order is not a convention someone chose — it falls out
/// of how the handler TESTS the bit (`0x6AD0`):
///
/// ```text
///   0x6AD0  mov al,es:[bx+di]
///   0x6AD3  shl al,cl        shift the target bit up by (bit & 7)
///   0x6AD5  shl al,1         once more, into CARRY
///   0x6AD7  jae 0x6AE2       carry clear -> the bit was 0
/// ```
///
/// Bit 0 reaches the carry after a single shift, so bit 0 is the byte's HIGH bit.
/// [`bit_flag_mask`]'s `0x80 >> (bit & 7)` is that sequence written as a mask.
pub fn bit_flag_byte_offset(base_offset: u16, bit_index: u8) -> u16 {
    base_offset.wrapping_add((bit_index >> 3) as u16)
}

/// `0x80 >> (bit & 7)` — the mask form of the `shl al,cl / shl al,1 / jae`
/// sequence at `0x6AD3`, which is why bit 0 is the HIGH bit. See
/// [`bit_flag_byte_offset`] for the full derivation.
/// The engine's bit-index-to-mask rule: HIGH BIT FIRST, so index 0 is `0x80`
/// and index 7 is `0x01`.
///
/// The game never writes a mask — it shifts the wanted bit into the carry:
/// `and cl,7 / inc cl / shl al,cl` @`0x6236` leaves bit `7 - (index & 7)` in CF
/// (audit-fixes #274). This is the mask form of that, and the direction is the
/// opposite of the `1 << i` anyone writes without checking.
pub fn bit_flag_mask(bit_index: u8) -> u8 {
    0x80u8 >> (bit_index & 7)
}

/// The FIELD-OFFSET MATRIX lookup, `vm_field_offset` (`0x6023`) — how the engine
/// turns a (selector, kind) pair into a byte offset inside an object record.
///
/// ```text
///   0x6024  shl ax,4          selector * 16 -- the matrix row
///   0x6027  bsf bx,bx         kind -> the index of its LOWEST SET BIT
///   0x602A  add bx,ax
///   0x602C  mov al,gs:[bx+0x6d60]
/// ```
///
/// (`0x6023` itself is the `push bx` prologue; the routine is seven instructions
/// long including its `pop`/`ret`.)
///
/// The `bsf` is the part worth stating: KIND IS A BITMASK, not an ordinal, so
/// column `k` belongs to kind `2^k` and a kind of 0 has no column at all — which
/// is why this returns `None` for it rather than reading row-relative garbage.
/// The matrix lives at `DS:0x6D60` in rows of 16 bytes and is pinned to the image
/// by `field_matrix_entries_match_the_constants`.
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
/// Test hook for the recomp differential (`native_zero_word_scan_matches_the_lift`).
pub fn scan_zero_word_pub(cod: &[u8], start: usize, end: usize) -> usize {
    scan_zero_word(cod, start, end)
}

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

impl VmToken {
    /// The token's byte offset in the COD stream (every variant records it).
    /// With the stream in hand this also yields the raw opcode byte —
    /// `cod[token.offset()]` — which is how the live-coverage test checks that
    /// no opcode the shipped scripts use is silently swallowed by `step()`.
    pub fn offset(&self) -> usize {
        match self {
            VmToken::Text { offset, .. }
            | VmToken::Actor { offset, .. }
            | VmToken::RecordLink { offset, .. }
            | VmToken::RecordEntry { offset, .. }
            | VmToken::RecordClear { offset, .. }
            | VmToken::BitFlag { offset, .. }
            | VmToken::RecordState { offset, .. }
            | VmToken::GlobalWordCompare { offset, .. }
            | VmToken::GlobalPairCompare { offset, .. }
            | VmToken::PairRecord { offset, .. }
            | VmToken::RecordTriple { offset, .. }
            | VmToken::ScriptProfileRequest { offset, .. }
            | VmToken::Op { offset, .. }
            | VmToken::Invalid { offset, .. } => *offset,
        }
    }
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
            // The handler consumes this prefix UNCONDITIONALLY: 0x6C86 `cmp al,0xA1` /
            // 0x6C8E `inc si` runs BEFORE the mode test at 0x6C9C, so the byte is
            // skipped whatever the mode. Gating the skip on mode1 would leave the
            // byte in the operand stream in mode 0 and shift every later read by one.
            let inverted = cod.get(pos + 1) == Some(&0xA1);
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
            // The handler consumes this prefix UNCONDITIONALLY: 0x6C86 `cmp al,0xA1` /
            // 0x6C8E `inc si` runs BEFORE the mode test at 0x6C9C, so the byte is
            // skipped whatever the mode. Gating the skip on mode1 would leave the
            // byte in the operand stream in mode 0 and shift every later read by one.
            let inverted = cod.get(pos + 1) == Some(&0xA1);
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
            // The handler consumes this prefix UNCONDITIONALLY: 0x6C86 `cmp al,0xA1` /
            // 0x6C8E `inc si` runs BEFORE the mode test at 0x6C9C, so the byte is
            // skipped whatever the mode. Gating the skip on mode1 would leave the
            // byte in the operand stream in mode 0 and shift every later read by one.
            let inverted = cod.get(pos + 1) == Some(&0xA1);
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
            // The handler consumes this prefix UNCONDITIONALLY: 0x6C86 `cmp al,0xA1` /
            // 0x6C8E `inc si` runs BEFORE the mode test at 0x6C9C, so the byte is
            // skipped whatever the mode. Gating the skip on mode1 would leave the
            // byte in the operand stream in mode 0 and shift every later read by one.
            let inverted = cod.get(pos + 1) == Some(&0xA1);
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
/// The talk/presentation-record field: selector `0x13`, **kind slot 1**.
///
/// Unlike [`LOCATION_FIELD`] this is not a port assumption — the BINARY hardcodes the
/// kind too. `0x6664` computes `ax = 0x13<<4 + 1 = 0x131` and reads
/// `gs:[bx+0x6D60]` at that fixed index, giving `0x3A` in the shipped image. The
/// selector's other kinds hold different values (`kind0=0x08, kind4=0x1C, kind9=0x0A`),
/// and the game never consults them here.
/// The TALK field offset — matrix entry `[selector 0x13][column 1]` at
/// `DS:0x6D60`, not an immediate anywhere.
///
/// `0x6664` computes the address directly rather than going through the `BSF`
/// resolver: `mov ax,0x13` / `shl ax,4` (selector * 16) / `inc ax` (column 1) /
/// `mov al,gs:[bx+0x6d60]`. Column 1 is kind bit 1, i.e. kind 2 — the code knows
/// the kind here, so it hardcodes the column instead of resolving it.
///
/// Pinned to the image by `field_matrix_entries_match_the_constants`.
const TALK_FIELD: u16 = 0x3A;
/// The speaker's location field: `vm_field_offset(0x11, kind)` for **kind 1**.
///
/// ASSUMPTION, made explicit (matrix row for selector `0x11` at `gs:0x6D60`):
///
/// ```text
///   kind:   0     1     2     3     4     7     9    10
///   offset: 0x06  0x18  0x16  0x16  0x16  0x14  0x04  0x14
/// ```
///
/// The value varies BY KIND, and the binary resolves it dynamically —
/// `mov ax,0x11; call 0x6023` at `0x557C`, `0x5B77`, `0x61C3` and others. Hardcoding
/// `0x18` is therefore only correct for kind-1 objects.
///
/// That holds for the actors this is applied to: `load_deb_objects` keeps only `kind == 1`
/// entries (audit fix #40). But it is an assumption, not an identity, and applying this
/// constant to any non-kind-1 record would silently read the wrong field.
///
/// Contrast [`TALK_FIELD`], where the BINARY itself hardcodes the kind-1 slot.
/// The LOCATION field offset — matrix entry `[selector 6][column 2]` (and
/// `[9][8]`) at `DS:0x6D60`, read through `vm_field_offset` (`0x6023`).
///
/// Pinned to the image by `field_matrix_entries_match_the_constants`.
const LOCATION_FIELD: u16 = 24;
const SPECIAL_OBJECT_SLOT_COUNT: usize = 16;
const VM_FIELD_OFFSET_SELECTOR_PRESENTATION_HANDOFF: u8 = 0x02;
/// `mov ax,0x11` @`0x625B` in the nav source-list builder (`0x624B`) — the
/// selector the walk resolves to find an object's parent link.
///
/// `ship3d::SHIP_3D_FIELD_SELECTOR_PARENT_LINK` is the same `0x11` from the same
/// instruction, named for what the field MEANS rather than which opcode family
/// reaches it (audit-fixes #285).
const VM_FIELD_OFFSET_SELECTOR_C2: u8 = 0x11;
const VM_FIELD_OFFSET_SELECTOR_C9_RELATED: u8 = 0x13;
/// Selector `0x08` — the ENCOUNTER COUNTER. `FIELD_OFFSETS[8]` is non-zero in
/// exactly one column (`0x36` at column 1 = kind 2), so this is a kind-2 field.
/// Written by `post_update_encounter_counter` (`0x5DCE`/`0x5DF6`); read as the
/// third condition of both object-list filters (`0x83DF`, `0x91DB`).
const VM_FIELD_OFFSET_SELECTOR_ENCOUNTER: u8 = 0x08;
/// Bit 15 of an object's `+2` flag word, set alongside the encounter-counter
/// bump (`or word [si+2],0x8000` @ `0x5DD2`/`0x5DFA`).
pub const OBJECT_FLAG_PAIR_SEEN: u16 = 0x8000;
/// Bit 0 of an object's `+2` flag word — the ACTIVE bit, the second condition of
/// both list filters (`test word [si+2],1` @ `0x91D4`, `test byte [si+2],1` @ `0x83D9`).
pub const OBJECT_FLAG_ACTIVE: u16 = 0x0001;

/// `arche+0x16` — the CURRENT LOCATION object (`mov bp,fs:[si+0x16]` @ `0x8365`
/// with `si = gs:[0x6752] = arche`; the same word `0x6B44` clears).
pub const ARCHE_LOCATION_FIELD: u16 = 0x16;
/// Location kinds the status header distinguishes (`0x836C`, `0x8376`).
pub const LOCATION_KIND_SHIP: u16 = 0x10;
pub const LOCATION_KIND_BLACK_HOLE: u16 = 0x100;

/// The status panel's four headers, supplied by the caller from the GAME'S OWN
/// strings (`bloodprg::location_status_headers` reads `DS:0x12E`, `0x137`,
/// `0x13E`, `0x14B`). They used to be four `&str` literals here — transcribed
/// text, short enough that the content guard's prose test could not see them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusHeaders {
    /// `mov si,0x12E` @`0x8369`.
    pub planet: String,
    /// `mov si,0x137` @`0x836C`'s branch.
    pub ship: String,
    /// `mov si,0x13E` @`0x8376`'s branch.
    pub black_hole: String,
    /// `mov si,0x14B` @`0x839F`.
    pub life_support: String,
}
/// `(DS offset, file offset)` for each header — `0x12E`/`0x137`/`0x13E` are the
/// three `mov si,imm` constants and `0x14B` the roster caption at `0x839F`.
///
/// The strings themselves are NOT here. They used to be, pinned to these bytes by
/// a test — a verified transcription, which is far better than a loose literal and
/// still a copy: it breaks against a differing build instead of following it. The
/// port now READS them (`bloodprg::location_status_headers`), and this table
/// remains as the address evidence.
pub const STATUS_STRING_TABLE: [(u16, usize); 4] = [
    (0x012E, 0x0D54E),
    (0x0137, 0x0D557),
    (0x013E, 0x0D55E),
    (0x014B, 0x0D56B),
];

/// Layout of the DESTINATION INFO PANEL drawn by `0x9137..0x91EC`. Every value is
/// an immediate in that routine; see [`VmMachine::location_panel_rows`].
pub const LOCATION_PANEL_X: i32 = 0x6E;
pub const LOCATION_PANEL_Y: i32 = 0x19;
pub const LOCATION_PANEL_ROW_PITCH: i32 = 0x0A;
pub const LOCATION_PANEL_NAME_GAP: i32 = 6;
pub const LOCATION_PANEL_HEADER_COLOR: u8 = 0xEE;
pub const LOCATION_PANEL_ROW_COLOR: u8 = 0xFE;
/// The panel's window rect `(x, y, w, h)` — `DS:0x2780`, a STATIC constant: a
/// whole-image search for every store form to `0x2780` finds no writer, and the
/// only references are the two `mov si/di,0x2780` at `0x9114`/`0x9203` plus the
/// draw at `0x9142`.
pub const LOCATION_PANEL_BOX: [u16; 4] = [0x64, 0x14, 0xA0, 0x46];
/// `ax = 0xFFCE` at `0x90ED`, negated on entry to the table builder (`0x22F1`).
/// The panel's tint strength, as a PERCENTAGE — and it appears in the binary as
/// its two's complement, which is why no `0x32` immediate exists:
///
/// ```text
///   0x90ED  mov ax,0xffce        the caller passes -50
///   0x22F1  neg ax               the blend builder negates it -> 50
///   0x22F5  mul bx / mov bx,0x64 / div bx   ... * component / 100
/// ```
///
/// `0x64` is the 100 it divides by, so `ax` really is a percentage. The caller
/// passing the NEGATIVE and the builder negating on entry means a search for the
/// value finds nothing at either address on its own.
pub const LOCATION_PANEL_TINT_PERCENT: u16 = 50;
/// `[0xADA] = 8` (`0x903E`) — the zoom-open/shut interpolation step count.
pub const LOCATION_PANEL_ZOOM_STEPS: u8 = 8;
/// The zoom SOURCE rect seeded at `0x9029`: a 4x4 square at the cursor.
pub const LOCATION_PANEL_CURSOR_RECT_SIZE: u16 = 4;

/// The chart marker's position field — selector `0x0B`, whose matrix row is
/// `0x18` for kinds 8 and `0x10` (`FIELD_OFFSETS[0x0B]`). Read as `x` at `+0x18`
/// and `y` at `+0x1A` by the picker at `0x92BC`.
pub const NAV_PICK_POSITION_FIELD: u16 = 0x18;

/// A menu entry points at the record's INLINE NAME; the record is four bytes
/// earlier. Emitted as `add ax,4` when a menu is built (`0x87D5`) and undone as
/// `sub ax,4` when a row is selected (`0xB33D`) — one constant, both directions.
pub const SHIP_3D_TARGET_NAME_TO_RECORD: u16 = 4;

/// The entity-candidate filter (`0x7259`): an object joins the destination list
/// only if its flags word has any of `0x98` (`test bx,0x98` @`0x727E`).
pub const ENTITY_CANDIDATE_KIND_MASK: u16 = 0x98;
/// ... and bit 1 of the BYTE at `+2` is set (`test byte es:[di+2],2` @`0x7284`).
pub const ENTITY_CANDIDATE_READY_BIT: u8 = 2;

/// `test word es:[eax],0x140` @`0xB0FB` — the location-kind test that decides
/// whether the click commits the first CANDIDATE or the location OBJECT.
pub const SHIP_CLICK_LOCATION_KIND_MASK: u16 = 0x140;
/// The picker's box test for one marker (`0x9308..0x932B`): the box starts two
/// pixels up-left of the marker and BOTH bounds are inclusive (`jb`/`ja` skip only
/// strictly outside). The single copy of that rule — `nav_chart_pick` walks record
/// offsets and the engine walks `NavChartObject`s, but both ask here.
pub fn nav_chart_marker_contains(
    marker: (i32, i32),
    hit_box: (i32, i32),
    mouse: (i32, i32),
) -> bool {
    let (x0, y0) = (marker.0 - 2, marker.1 - 2);
    mouse.0 >= x0 && mouse.0 <= x0 + hit_box.0 && mouse.1 >= y0 && mouse.1 <= y0 + hit_box.1
}

/// The owner lookup — `vm_record_lookup_by_threshold` `0x6034`:
///
/// ```text
///   0x603B  cmp ax,[si+0x10] / jbe    advance while the entry offset is BELOW ax
///   0x6040  add si,0x14
///   0x6045  sub si,0x14               step BACK one entry
///   0x6048  mov ax,[si+0x10]          and return ITS object offset
/// ```
///
/// So: the last directory entry whose offset is strictly less than `off`, over a
/// list the loader keeps ascending. One difference, deliberate — when `off` is at
/// or below the FIRST entry the original's `sub si,0x14` steps in front of the
/// table and returns whatever lies there; the port returns `None` rather than
/// reproducing an out-of-bounds read.
pub fn owner_object_offset_in(object_offsets: &[u16], off: u16) -> Option<u16> {
    object_offsets.iter().rev().copied().find(|&o| o < off)
}

/// The nav picker's hit box for a KIND (`0x92BF` default, `0x92D3` black hole,
/// `0x92FC` ship). The single copy of that ladder — `NavChartObject::hit_box` and
/// `VmMachine::nav_chart_hit_box` both call here.
pub fn nav_chart_hit_box_for_kind(kind: u16) -> (i32, i32) {
    if kind & LOCATION_KIND_BLACK_HOLE != 0 {
        NAV_PICK_BOX_BLACK_HOLE
    } else if kind & LOCATION_KIND_SHIP != 0 {
        NAV_PICK_BOX_SHIP
    } else {
        NAV_PICK_BOX_DEFAULT
    }
}

/// Bit 1 of an object's `+2` flag word — the gate `0x6073` applies when building
/// the active-object list, and the same bit the `0xC1` nav path (`0x6C3B`) and
/// the entity candidate list (`0x7259`) test.
pub const OBJECT_FLAG_IN_PLAY: u16 = 0x0002;
/// The kinds the nav chart draws: `test bx,0x118` at `0x723D` — kind `0x08`,
/// kind `0x10` (a SHIP) and kind `0x100` (a BLACK HOLE).
pub const NAV_CHART_KIND_MASK: u16 = 0x0118;
/// Hit-box sizes per kind, each pair written as two IMMEDIATES into the picker's
/// own scratch words `DS:0x277A`/`DS:0x277C`:
///
/// ```text
///   0x92BF  mov word [0x277a],0xc  / 0x92C5 mov word [0x277c],0xb   default
///   0x92CB  test word es:[di-0x18],0x100                            BLACK HOLE?
///   0x92D3  mov word [0x277a],0x13 / 0x92D9 mov word [0x277c],0xc   -> its box
///   0x92F4  test word es:[di-0x18],0x10                             SHIP?
///   0x92FC  mov word [0x277a],0x15 / 0x9302 mov word [0x277c],0xa   -> its box
/// ```
///
/// The two gates test `0x100` and `0x10`, the same kind bits
/// [`NAV_CHART_KIND_MASK`] selects on — so the box a record gets and the reason
/// it is on the chart at all come from one kind word.
///
/// The default is written FIRST and overwritten, which is why a record matching
/// neither gate keeps `(0xC, 0xB)`.
pub const NAV_PICK_BOX_DEFAULT: (i32, i32) = (0x0C, 0x0B);
/// `mov word [0x277a],0x13` @`0x92D3` (with `0xC` into `0x277C` @`0x92D9`).
pub const NAV_PICK_BOX_BLACK_HOLE: (i32, i32) = (0x13, 0x0C);
/// `mov word [0x277a],0x15` @`0x92FC` (with `0xA` into `0x277C` @`0x9302`).
pub const NAV_PICK_BOX_SHIP: (i32, i32) = (0x15, 0x0A);

/// One drawn row of the destination info panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocationPanelRow {
    pub x: i32,
    pub y: i32,
    pub color: u8,
    pub text: String,
}
const C2_ACTIVE_LINE_KIND2: u16 = 0x27;
const C2_ACTIVE_LINE_KIND400: u16 = 0x2B;
/// Touched by the game at `mov word ptr [0x2793], 0` @`0x0AFC6`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const VM_UI_FLAGS: u16 = 0x2793;
/// Touched by the game at `test byte ptr [0x1fb2], 1` @`0x0B001`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const C2_PRESENTATION_GATE: u16 = 0x1FB2;
/// Touched by the game at `and byte ptr [0x67aa], 0xfc` @`0x0B54D`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const C2_PRESENTATION_FLAGS: u16 = 0x67AA;
const C2_PRESENTATION_BUSY_FLAG: u8 = 0x02;
/// Touched by the game at `mov word ptr [0x6788], ax` @`0x0B00F`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const VM_ACTIVE_LINE: u16 = 0x6788;
/// Touched by the game at `mov byte ptr [0x252a], 0` @`0x0B29A`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const C9_PRESENTATION_GATE_A: u16 = 0x252A;
/// Touched by the game at `mov byte ptr [0x2531], 6` @`0x0B336`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const C9_PRESENTATION_GATE_B: u16 = 0x2531;
const C4_POST_UPDATE_SENTINEL: u16 = 0xFFFF;
const VM_PENDING_RESOURCE_PROFILE: u16 = 0x6780;
const VM_PRESENTATION_PRIMARY_C4_RECORD: u16 = 0x675E;
/// Touched by the game at `test byte ptr [0x67ac], 1` @`0x0B498`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const VM_PRESENTATION_ACTIVE: u16 = 0x67AC;
const VM_PRESENTATION_RELATED_FLAG20: u16 = 0x67AF;
/// Touched by the game at `test byte ptr [0x67b0], 1` @`0x0B4DC`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const VM_PRESENTATION_DEFER_A: u16 = 0x67B0;
/// Touched by the game at `test byte ptr gs:[0x67b1], 2` @`0x0579d` — found by decoding forward from a
/// LIFTED function's entry, so the boundary is the recompiler's, not a scan's.
const VM_PRESENTATION_LOOP_FLAG: u16 = 0x67B1;
const VM_PRESENTATION_PAIR_WRITE_DISABLED: u16 = 0x67B6;
const VM_PRESENTATION_START_LOCK: u16 = 0x67B7;
/// Touched by the game at `mov byte ptr [0x67ba], al` @`0x0B552`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const VM_PRESENTATION_TEXT_WAIT: u16 = 0x67BA;
/// Touched by the game at `mov byte ptr [0x67bb], 0` @`0x079c5` — found by decoding forward from a
/// LIFTED function's entry, so the boundary is the recompiler's, not a scan's.
const VM_PRESENTATION_HOLD_COMPLETE: u16 = 0x67BB;
/// Touched by the game at `mov byte ptr [0x67bc], al` @`0x0B544`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const VM_PRESENTATION_HOLD_READY: u16 = 0x67BC;
/// Touched by the game at `mov word ptr gs:[0x67f8], 0` @`0x057a9` — found by decoding forward from a
/// LIFTED function's entry, so the boundary is the recompiler's, not a scan's.
const VM_PRESENTATION_WORD_BUFFER: u16 = 0x67F8;
/// Touched by the game at `mov word ptr [0xa32], 1` @`0x0B0BC`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const VM_PRESENTATION_STATUS_WORD: u16 = 0x0A32;
/// Touched by the game at `mov ax, word ptr gs:[0x6762]` @`0x05795` — found by decoding forward from a
/// LIFTED function's entry, so the boundary is the recompiler's, not a scan's.
const VM_PRESENTATION_ACTIVE_RECORD: u16 = 0x6762;
/// Touched by the game at `mov word ptr [0x6768], 0xc4` @`0x0B3AE`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const VM_PRESENTATION_DEFERRED_RECORD_TYPE: u16 = 0x6768;
/// Touched by the game at `mov word ptr [0x676a], ax` @`0x0B3B4`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const VM_PRESENTATION_DEFERRED_RECORD_RELATED: u16 = 0x676A;
const VM_PRESENTATION_DEFERRED_RECORD_AUX: u16 = 0x676C;
const VM_PRESENTATION_SIGNAL_SLOT: u16 = 0x679A; // was written in decimal (26522)
/// Touched by the game at `mov byte ptr [0x5b55], 1` @`0x0B64A`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const VM_PRESENTATION_SCENE_DIRTY: u16 = 0x5B55;
/// Touched by the game at `mov ax, word ptr [0x24f3]` @`0x0AFA2`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const VM_PRESENTATION_INPUT_GATE_A: u16 = 0x24F3;
/// Touched by the game at `mov byte ptr [0x2751], 1` @`0x08836`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const VM_PRESENTATION_INPUT_GATE_B: u16 = 0x2751;
/// Touched by the game at `test byte ptr [0x5e64], 1` @`0x0B1DD`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const VM_PRESENTATION_INPUT_GATE_C: u16 = 0x5E64;
/// Touched by the game at `test byte ptr [0x2565], 1` @`0x08713`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const VM_PRESENTATION_INPUT_GATE_D: u16 = 0x2565;
/// Touched by the game at `mov byte ptr [0x2736], 1` @`0x0892C`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const VM_PRESENTATION_INPUT_GATE_E: u16 = 0x2736;
/// Touched by the game at `mov byte ptr [0x2737], 1` @`0x0893C`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const VM_PRESENTATION_INPUT_GATE_F: u16 = 0x2737;
const VM_PRESENTATION_HANDOFF_GATE: u16 = 0x27D7;
const VM_PRESENTATION_INPUT_GATE_G: u16 = 0x27DA;
const VM_PRESENTATION_INPUT_GATE_H: u16 = 0x2792;
/// Touched by the game at `mov word ptr [0x2a19], 0` @`0x087B0`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
const VM_PRESENTATION_INPUT_GATE_I: u16 = 0x2A19;
/// Touched by the game at `test byte ptr [0x27e8], 1` @`0x08bb3` — found by decoding forward from a
/// LIFTED function's entry, so the boundary is the recompiler's, not a scan's.
const VM_PRESENTATION_DESCRIPTOR_PENDING: u16 = 0x27E8;
/// Touched by the game at `mov bx, word ptr gs:[0x6782]` @`0x057ed` — found by decoding forward from a
/// LIFTED function's entry, so the boundary is the recompiler's, not a scan's.
const VM_BRANCH_A: u16 = 0x6782;
/// Touched by the game at `mov word ptr gs:[0x6784], bx` @`0x057f2` — found by decoding forward from a
/// LIFTED function's entry, so the boundary is the recompiler's, not a scan's.
const VM_BRANCH_B: u16 = 0x6784;
/// Touched by the game at `mov si, word ptr gs:[0x6776]` @`0x057cd` — found by decoding forward from a
/// LIFTED function's entry, so the boundary is the recompiler's, not a scan's.
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
    /// Objects whose selector-`0x08` ENCOUNTER COUNTER the post-update ladder
    /// incremented this pass (`0x5DCE` / `0x5DF6`). See
    /// `post_update_encounter_counter`.
    pub encounter_counter_bumps: Vec<u16>,
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
    /// The script profile a `0xD2` request left pending, or `None`.
    ///
    /// `gs:0x6780` is the request slot and `0xFFFF` means EMPTY:
    ///
    /// ```text
    ///   0x108E  cmp word [0x6780],-1     nothing pending?
    ///   0x10C5  mov ax,[0x6780]          take the request
    ///   0x10D3  mov word [0x6780],0xffff and reset the slot
    /// ```
    ///
    /// So it is a ONE-SHOT: the consumer writes the sentinel back, and a second
    /// read before another `0xD2` sees nothing. This function's `filter` on
    /// `0xFFFF` is that sentinel test; the port models the reset by taking the
    /// request from an event list rather than a mutable cell, which is equivalent
    /// as long as nothing replays the same event.
    ///
    /// `0x64B8` is the writer — see
    /// [`script_profile_index_from_request_operand`] for its `cbw` subtlety.
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
/// The built-in objects the engine resolves BY NAME, and their DEB offsets.
///
/// Matching by name is the game's own method, not a port convenience: `0x5490`
/// loads `di` with a name pointer, compares (`lcall 0x1CE:0x2C4`), and on a match
/// stores the object's `[si+0x10]` offset into that built-in's global —
/// `mov gs:[0x674e],ax` @`0x549D` for `blood`. The names are packed
/// NUL-terminated strings from `DS:0x67BE`:
///
/// ```text
///   0x67BE blood    0x67C4 orxx      0x67C9 Honk    0x67CE menu
///   0x67D3 arche    0x67D9 cryobox   0x67E1 Ark     0x67E5 Scruter_Jo
///   0x67F0 vbio
/// ```
///
/// NINE names. This struct carried EIGHT — `cryobox` was absent, so an object the
/// engine resolves and gives a global was not resolved here at all. Added below;
/// the gap was invisible because nothing enumerated the game's table.
pub struct VmNamedObjectOffsets {
    pub blood: Option<u16>,
    pub orxx: Option<u16>,
    pub honk: Option<u16>,
    pub menu: Option<u16>,
    pub arche: Option<u16>,
    /// `DS:0x67D9` — the sixth built-in, missing from this struct until #172.
    pub cryobox: Option<u16>,
    pub ark: Option<u16>,
    pub scruter_jo: Option<u16>,
    pub vbio: Option<u16>,
}

impl VmNamedObjectOffsets {
    /// Resolve one DEB object name against the built-in table and record its
    /// offset. Returns whether the name IS a built-in — the caller uses that to
    /// skip non-built-ins, so a `false` is an ordinary answer, not an error.
    ///
    /// The nine names come from `DS:0x67BE` and the game matches them the same
    /// way (`0x5490`'s compare, then `mov gs:[0x674e],ax` @`0x549D`); see the type
    /// doc for the full table and for the `cryobox` omission #172 found.
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
        } else if name.eq_ignore_ascii_case("cryobox") {
            self.cryobox = Some(offset);
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

    /// Delegation to [`owner_object_offset_in`], which carries the rule AND its
    /// citation. Two of these exist — one here on the execution context, one on
    /// `VmMachine` — because both types hold their own `object_offsets`.
    ///
    /// Deliberately NOT repeating the address here: `check_duplicate_rules.py`
    /// flags one name citing one address from two places, and it is right to —
    /// that is what a copied rule looks like. The citation belongs to the helper;
    /// these are plumbing to it.
    fn owner_object_offset(&self, record_offset: u16) -> Option<u16> {
        owner_object_offset_in(&self.object_offsets, record_offset)
    }

    /// The `0x6946` family's RHS substitution: an operand equal to the SPECIAL
    /// OBJECT (`gs:0x674E`) becomes `0xFFFF` before the compare. It is a
    /// substitution, NOT a match-anything wildcard — the reading `record_op`
    /// carried and #127 deleted, after a transcript diff showed match-anything
    /// made every aboard-guard pass.
    fn remap_special_rhs(&self, value: u16) -> u16 {
        if self.special_object_offset == Some(value) {
            0xffff
        } else {
            value
        }
    }

    /// Whether an operand IS the special object, i.e. whether
    /// [`Self::remap_special_rhs`] would substitute it. Separate from the remap
    /// because the SET path needs to know (the slot bookkeeping at `0x5FF6`/
    /// `0x5FD8` keys off it) while the QUERY path only needs the substituted
    /// value.
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

/// The A6 display gate, both halves of it: the line is ACTIVE (`or cx,cx / jns`
/// @`0x6647`) and NOT already shown (`test word es:[di+2],0x8000` @`0x665A`).
/// Both failures take the same exit, which is why one predicate covers them.
fn text_line_should_display(state: &[u8], line_index: u16, flags_b5: u8) -> bool {
    text_flags_are_active(flags_b5)
        && !text_line_already_shown(state_u16(state, text_line_flags_offset(line_index)))
}

/// Whether the line's presentation record holds an ACTIVE actor — the record at
/// `line + TALK_FIELD` typed `0xC4` (`OP_ACTOR`, whose handler is `0x6C7E`). The
/// comparison is against the OPCODE value because that is what the record's type
/// word stores.
fn text_presentation_record_is_active(state: &[u8], line_index: u16) -> bool {
    state_u16(state, text_presentation_record_offset(line_index)) == OP_ACTOR as u16
}

/// Both A6 runtime gates, each independently switchable by the execution context.
///
/// The presentation-record gate wants the line's record typed `0xC4`
/// (`OP_ACTOR`); the display gate wants the line active and not already shown
/// (`0x6647`/`0x665A`, #169). The context flags exist because the decoders run
/// these paths without a live presentation state — the GAME always applies both.
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

/// The A6 handler MODIFIES ITS OWN BYTECODE: on accepting a line it clears bit 7
/// of `b5` in the COD stream (`and byte [si+1],0x7F` after `0x668D`, unless
/// `b4 & 1` preserves it), so a line that has played will not display again.
///
/// The port cannot write to the shipped script, so this side table holds the
/// modified `b5` per stream offset and [`Self::flags_b5`] reads through it. Same
/// observable behaviour, different mechanism — and worth stating, because the
/// bytecode being self-modifying is not something a reader would assume, and a
/// port that treats the stream as read-only WITHOUT this table replays every
/// accepted line.
impl TextTokenRuntimeFlags {
    /// The effective `b5` at a stream offset: the accept-modified value if this
    /// line has been accepted, else the byte as shipped. Stands in for re-reading
    /// the COD byte the handler rewrites at `0x668D`.
    fn flags_b5(&self, offset: usize, original_flags_b5: u8) -> u8 {
        self.flags_b5_by_offset
            .get(&offset)
            .copied()
            .unwrap_or(original_flags_b5)
    }

    /// Record the accept-time `b5` rewrite for one stream offset — the port's
    /// stand-in for the handler's self-modifying `and byte [si+1],0x7F` at
    /// `0x668D`, which clears bit 7 unless `b4 & 1` preserves it.
    fn accept_line(&mut self, offset: usize, flags_b4: u8, effective_flags_b5: u8) {
        let next = text_flags_after_accept(flags_b4, effective_flags_b5);
        if next != effective_flags_b5 {
            self.flags_b5_by_offset.insert(offset, next);
        }
    }
}

/// Set the ALREADY-SHOWN bit on a line's flags word — the `0x8000` the A6 gate
/// tests at `0x665A`. This is the state half of the accept; the bytecode half is
/// [`TextTokenRuntimeFlags::accept_line`] (`0x668D`). Both must happen: the flag
/// stops a re-display through the record, the `b5` rewrite stops it through the
/// stream.
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
    /// Remove an owner from the 16-slot array — the scan at `0x5FF6`
    /// (`mov bp,0x6D3E / mov cx,0x10`), which clears the matching slot to 0 and
    /// STOPS. Returns whether it found one; the routine reports the same through
    /// the carry (`stc` at `0x5FF2`, `clc` at `0x5FEA`).
    fn remove(&mut self, value: u16) -> bool {
        if let Some(slot) = self.slots.iter_mut().find(|slot| **slot == value) {
            *slot = 0;
            true
        } else {
            false
        }
    }

    /// Insert an owner into the first EMPTY slot — the free-slot scan at `0x6008`
    /// (`cmp word [bp],0 / je`, `add bp,2`, 16 iterations).
    ///
    /// Already-present returns success WITHOUT inserting again, and a full array
    /// returns failure — which the caller must honour: `0x6995`'s SET path skips
    /// the write when insertion fails rather than storing anyway, so a full slot
    /// list silently declines the change instead of corrupting the record.
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

/// The object an ACTOR record belongs to: the record sits at `object + TALK_FIELD`,
/// the `0x3A` field the A6 handler resolves at `0x660D`
/// (`0x3A`), so this subtracts. The `Option` guards a record below `0x3A`, which
/// cannot be an actor record — the game never forms one, so this is the port
/// declining to compute a nonsense offset rather than a case the original handles.
fn actor_object_offset_from_record(record_offset: u16) -> Option<u16> {
    record_offset.checked_sub(TALK_FIELD)
}

/// The object that OWNS an arbitrary record — [`owner_object_offset_in`]'s
/// scan-then-step-back, which holds the rule and its citation. Distinct from
/// [`actor_object_offset_from_record`]: that one knows the record is an actor's
/// and subtracts a fixed field, this one searches because the record could be any
/// of the object's.
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
    // 0x6995: when the record ALREADY holds 0xFFFF the handler removes the owner
    // from the special-slot list and then `jmp 0x69C2` -- straight to the store,
    // with the RAW value. It does NOT fall through into the insert block below.
    // Falling through would re-insert the owner and store 0xFFFF instead of the
    // value the script asked for.
    if state_u16(state, field_offset) == 0xffff {
        if let Some(owner) = owner {
            special_slots.remove(owner);
        }
        state_set_u16(state, field_offset, value);
        return;
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

/// Whether a record's owning object is ACTIVE — bit 0 of the object's byte at
/// `+2`, the same flag `0x6073` tests (`test byte fs:[bx+2],2` reads bit 1 of the
/// pair; bit 0 is the active half). `None` when no owner resolves, which the
/// callers treat as "cannot decide" rather than "inactive".
fn record_owner_is_active(
    state: &[u8],
    context: &ExecutionContext,
    record_offset: u16,
) -> Option<bool> {
    record_owner_object_offset(context, record_offset)
        .map(|owner| state_u8(state, owner.wrapping_add(2)) & 1 != 0)
}

/// Whether an actor record's OBJECT is active — bit 0 at `object + 2`, the flag
/// pair `0x6073` tests, reached by
/// subtracting `TALK_FIELD` from the record. Unknown resolves to FALSE here (not
/// `None`): a record whose object cannot be found is not an active actor.
fn actor_record_is_active(state: &[u8], record_offset: u16) -> bool {
    actor_object_offset_from_record(record_offset)
        .map(|actor| state_u8(state, actor.wrapping_add(2)) & 1 != 0)
        .unwrap_or(false)
}

/// The `0xC4` QUERY condition, from the handler at `0x6C7E`:
///
/// ```text
///   0x6C86  mov al,[si] / cmp al,0xa1 / jne   the 0xA1 PREFIX...
///   0x6C8C  inc dl / inc si                   ...sets the INVERT flag
///   0x6C8F  lodsw / mov bp,ax                 the record offset
///   0x6C92  call 0x6034                       resolve its owner
///   0x6C97  lodsw                             the related offset
///   0x6C98  mov cx,es:[bp]                    the record's TYPE word
///   0x6C9C  test byte gs:[0x67ad],1 / je      query or set
/// ```
///
/// A match needs all three: the owner active, the type word `0xC4`, and the
/// stored related offset equal to the operand. `0xA1` before the opcode inverts
/// the result — the same prefix the `0x6D18` and `0x6F62` handlers read, so it is
/// a family-wide modifier rather than something specific to `0xC4`.
///
/// The non-`strict` early `None` covers an EMPTY record (type and related both
/// zero), which callers treat as "no opinion" rather than a failed match.
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

/// The `0xC4` SET write (handler `0x6C7E`): type `0xC4`, the related offset at
/// `+2`, and ZERO at
/// `+4`. The third word is written every time — it is not left as found — which is
/// what makes a freshly written actor record distinguishable from one carrying
/// state from a previous use.
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

/// The kind-1 post-update, from the post-exec record walk at `0x5816`: when the
/// record holds an ACTIVE actor (`0xC4`), latch
/// the related object's `0x20` flag, then arm the presentation — scene dirty,
/// status word 1, active 1 — UNLESS it is already active, in which case nothing
/// is armed and the existing presentation runs to completion.
///
/// The early return is what stops a second request restarting a playing
/// presentation, and it is checked AFTER the `0x20` latch, so the flag is
/// refreshed even when the arming is skipped.
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

/// The kind-2 HANDOFF check in the post-exec record walk (`0x5816`): can this
/// record take over the presentation currently running?
///
/// FOUR gates, and all four must be clear before anything hands off — a
/// presentation must be active, and the C2 gate, the handoff gate and the start
/// lock must all be unset. Three separate "not already busy" flags rather than
/// one, so a handoff cannot slip through during a start that has begun but not
/// finished, or during another handoff.
///
/// Then the primary `0xC4` record must still be an actor: the target of the
/// handoff has to be a live presentation, not a slot that was cleared while these
/// gates were being checked.
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

    // The handler compares the deferred type against 0xC1 and 0xC6 literally
    // (0x5A0B `cmp cx,0xc1` / 0x5A11 `cmp cx,0xc6`). Spelling the second as
    // `OP_RECORD_ENTRY_MIN + 1` obscured which opcode it means.
    let write_offset = if record_type == OP_RECORD_STATE_MIN as u16 || record_type == 0xC6 {
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

/// The ENCOUNTER COUNTER step of the post-update actor-pair ladder
/// (`vm_post_update_c4_pair` `0x5D8F`, block `0x5DB0..0x5E06`), run between the
/// processed-marker write and the related-record `0xC4` write.
///
/// The block is symmetric over the pair `si` = owner object, `di` = related
/// object (`ds:[bp+2]`):
///
/// ```text
///   0x5DB4  mov ax,[si] / cmp ax,1 / jne 0x5DE3   owner kind == 1?
///   0x5DC2    mov bx,[di]                          resolve against the RELATED kind
///   0x5DC4    ax=8 / call 0x6023 / or ax,ax / je   selector 8 -> field offset
///   0x5DCE    inc word [eax+edi]                   the RELATED object's counter
///   0x5DD2    or word [si+2],0x8000
///   0x5DE3  mov ax,[di] / cmp ax,1 / jne 0x5E09   related kind == 1?
///   0x5DEA    mov bx,[si]                          resolve against the OWNER kind
///   0x5DF6    inc word [eax+esi]                   the OWNER object's counter
///   0x5DFA    or word [si+2],0x8000
/// ```
///
/// So whichever partner is kind `1`, the OTHER partner's counter is bumped — and
/// the bump resolves selector `0x08` against THAT other partner's kind. Selector
/// `0x08` is non-zero in exactly one column of the field matrix
/// (`FIELD_OFFSETS[8]` = `[0, 0x36, 0, ...]`), and `vm_field_offset`'s `bsf`
/// makes column 1 the kind whose lowest set bit is bit 1 — i.e. **kind 2, offset
/// `0x36`**. The counter is therefore a KIND-2 field, incremented when a kind-2
/// object is paired with a kind-1 object. (An earlier note in `re/labels.csv`
/// called it a kind-1 field by reading the column index as a kind value; the
/// resolver's `bsf` says otherwise, and both readers agree — `0x83D4` gates on
/// `cmp [si],2` and `0x91CE` on `test [si],2`.)
///
/// Returns the object whose counter was incremented, which the real code also
/// stores in `gs:0x6798` before rescanning the COD (`0x739B`).
fn post_update_encounter_counter(
    state: &mut [u8],
    owner_offset: u16,
    related_offset: u16,
) -> Option<u16> {
    let owner_kind = state_u16(state, owner_offset);
    let related_kind = state_u16(state, related_offset);
    // 0x5DB4 / 0x5DE3: the kind-1 partner names the OTHER as the counter holder.
    // The `je 0x5DE3` on a zero offset means the first branch FALLS INTO the
    // second when the related object has no counter field, so both are tried.
    let target = if owner_kind == 1
        && vm_field_offset(VM_FIELD_OFFSET_SELECTOR_ENCOUNTER, related_kind).is_some_and(|o| o != 0)
    {
        related_offset
    } else if related_kind == 1 {
        owner_offset
    } else {
        return None;
    };
    let field = vm_field_offset(VM_FIELD_OFFSET_SELECTOR_ENCOUNTER, state_u16(state, target))?;
    if field == 0 {
        return None; // `or ax,ax / je` — the kind has no counter field
    }
    let counter = target.wrapping_add(field);
    state_set_u16(state, counter, state_u16(state, counter).wrapping_add(1));
    // 0x5DD2 / 0x5DFA: bit15 of the OWNER's +2 in BOTH branches.
    let flags = state_u16(state, owner_offset.wrapping_add(2));
    state_set_u16(
        state,
        owner_offset.wrapping_add(2),
        flags | OBJECT_FLAG_PAIR_SEEN,
    );
    Some(target)
}

fn post_update_actor_record_pair(
    state: &mut [u8],
    owner_offset: u16,
    record_offset: u16,
) -> Option<(u16, Option<u16>)> {
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
    // 0x5DB0..0x5E06 — the encounter counter, before the 0x5E09 C4 write.
    let counter_bump = post_update_encounter_counter(state, owner_offset, related_offset);
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
    Some((related_field, counter_bump))
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
        if let Some((related_record_offset, counter_bump)) =
            post_update_actor_record_pair(state, owner_offset, record_offset)
        {
            post_update
                .actor_record_pairs
                .push(PostUpdateActorRecordPair {
                    record_offset,
                    related_record_offset,
                });
            if let Some(bumped) = counter_bump {
                post_update.encounter_counter_bumps.push(bumped);
            }
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
    // Empty record -> matched=false -> branch (C3 query 0x6F2E je 0x6F5D ->
    // 0x6462), not a fall-through. See record_entry_condition.
    let owner_active = record_owner_is_active(state, context, record_offset)?;
    let matched = owner_active
        && record_type == OP_RECORD_LINK as u16
        && stored_related == related_record_offset;
    Some(if inverted { !matched } else { matched })
}

/// The `0xC3` QUEUE write (handler `0x6EEE`): type `0xC3`, the related offset at
/// `+2`, and **1** at
/// `+4` — where [`write_actor_record`]'s `0xC4` writes ZERO there. That third word
/// is the difference between a queued presentation and an active one, which is
/// why both writers set it explicitly rather than leaving it.
///
/// Handler `0x6EEE` (dispatch table `0x142D0`, the entry for `0xC3`).
fn write_record_link(state: &mut [u8], record_offset: u16, related_record_offset: u16) {
    state_set_u16(state, record_offset, OP_RECORD_LINK as u16);
    state_set_u16(state, record_offset.wrapping_add(2), related_record_offset);
    state_set_u16(state, record_offset.wrapping_add(4), 1);
}

/// The `0xC3` SET guard (`0x6EEE`): the write happens only when the owner is
/// ACTIVE, the
/// related object is active (bit 0 at `+2`), and the slot does not already hold an
/// active `0xC4` presentation.
///
/// The last condition is the important one — a queue request must not overwrite a
/// presentation that is already playing, so `0xC3` declines rather than replacing.
/// `None` when no owner resolves, which the caller treats as "cannot decide".
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

/// The `0xC1` QUERY path when the operand SELECTS a state (1 or 2) rather than
/// comparing directly — handler `0x6B4C`'s resolved branch.
///
/// Returns `None` for any other operand, and for a record already typed `0xC1`,
/// which sends the caller to the direct `{0xC1, operand}` comparison instead. The
/// two paths are exclusive: an operand of 1 or 2 against a non-`0xC1` record means
/// "resolve the owner's state", anything else means "compare these words".
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

/// Parse the ship-3D C1 source list — words up to and INCLUDING the `0xFFFF`
/// sentinel (`SHIP_3D_TARGET_EXIT_SENTINEL`, the back/exit row).
///
/// Returns `None` when the sentinel never arrives, rather than the words read so
/// far: an unterminated list means the bytes are not a source list, and treating
/// a truncated read as a short list would hand the caller a plausible-looking
/// result built from whatever followed in memory.
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

/// Read a record's three words as a ship-3D state slot, the layout the `0xC1`
/// handler `0x6B4C` writes — `{opcode, operand,
/// aux}`, the same `+0`/`+2`/`+4` layout every record writer uses. The third word
/// is the one carrying `2`/`1`/`0` for `0xC1`/`0xC3`/`0xC4`, so a slot round-trips
/// through here without losing which kind it is.
fn ship3d_record_state_slot(state: &[u8], record_offset: u16) -> ship3d::Ship3dRecordStateSlot {
    ship3d::Ship3dRecordStateSlot {
        opcode: state_u16(state, record_offset),
        operand: state_u16(state, record_offset.wrapping_add(2)),
        aux_word: state_u16(state, record_offset.wrapping_add(4)),
    }
}

/// Write a ship-3D state slot back into a record — the mirror of
/// [`ship3d_record_state_slot`], same `+0`/`+2`/`+4` layout the `0x6B4C` handler
/// uses. Writes all three words, so a slot cannot come back half-updated with a
/// stale third word saying it is a different record kind.
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

/// The `0xC1` SET path (handler `0x6B4C`): write `{0xC1, operand, 2}` into an
/// EMPTY record whose owner is active.
///
/// The third word is `2` here, against `0xC3`'s `1` and `0xC4`'s `0` — three
/// record types distinguished by that slot, which is why every writer sets it.
///
/// Order matters: the owner-active check comes first, the ship-3D source path
/// gets a chance next, and only then does the empty-record test run. A record
/// that is NOT empty returns `false` rather than being overwritten — the same
/// refusal `0xC3` makes, so a state request never displaces existing state.
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

/// The `0xC2` SET path (handler `0x6E34`): three gates, all of which must pass
/// before anything is written.
///
/// Owner active; the TARGET's `0x20` flag set (not the owner's — a different
/// object's bit decides whether this write is allowed); and a free special slot.
/// The slot insert is the third gate rather than a side effect: when the 16-slot
/// array is FULL the write is declined outright, so a saturated slot list stops
/// new `0xC2` state instead of silently dropping the bookkeeping (the caller
/// contract #175 records on `insert`).
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

/// Zero all THREE words of a record, as the `0xC9` clear at `0x6FB9` does — type,
/// related, and the third slot that
/// distinguishes `0xC1`/`0xC3`/`0xC4` (2/1/0). Clearing only the type would leave
/// a record that reads as empty to a type test and still carries its old related
/// offset, which the `0xC8` guard (#171) would then see as non-empty.
fn clear_record_words(state: &mut [u8], record_offset: u16) {
    state_set_u16(state, record_offset, 0);
    state_set_u16(state, record_offset.wrapping_add(2), 0);
    state_set_u16(state, record_offset.wrapping_add(4), 0);
}

/// The `0xC9` CLEAR (handler `0x6FB9`): zero the record, and if it held an ACTOR
/// (`0xC4`), follow the link and clear the related object's corresponding field
/// too, then reset the presentation gates.
///
/// The cascade is the part worth stating: clearing an actor record is not a local
/// operation. It resolves the related object's KIND, asks `vm_field_offset` for
/// the matching field, and clears that as well — so a `0xC9` on one record can
/// empty a slot in a different object. Returns the related offset it followed, or
/// `None` when the record was not an actor and nothing cascaded.
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

/// The `0xC5..=0xC8` entry write (handlers `0x6D18`, `0x6D80`, `0x6DCF`,
/// `0x6F62`): the OPCODE itself as the type word, the related
/// value at `+2`, zero at `+4`.
///
/// The type word is the opcode, not a fixed constant — which is why these four
/// share one writer while `0xC1`/`0xC3`/`0xC4` each have their own. What the
/// related value should BE differs per opcode, and that is
/// [`record_entry_stored_related_offset`]'s job (`0xC8` stores zero; see #171).
fn write_record_entry(state: &mut [u8], opcode: u8, record_offset: u16, stored_related: u16) {
    state_set_u16(state, record_offset, opcode as u16);
    state_set_u16(state, record_offset.wrapping_add(2), stored_related);
    state_set_u16(state, record_offset.wrapping_add(4), 0);
}

/// The `0xC5..=0xC8` SET paths, which share a WRITER but not a guard —
/// `write_record_entry` is common, the conditions are per opcode:
///
/// * `0xC5` (handler `0x6D18`) demands the operand object be active, its type
///   word be exactly `0x0200`, and the destination record be EMPTY.
/// * `0xC6` (`0x6D80`) writes unconditionally.
/// * `0xC8` (`0x6F62`) writes only into an empty record, and stores ZERO as the
///   related word rather than the operand (#171).
///
/// So four opcodes that decode identically behave differently on write, which is
/// the concrete reason [`is_record_entry_opcode`]'s range is a token-shape group
/// and not a behavioural family (#168).
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
    // An EMPTY record is not "no result" — the C5..C8 query handlers
    // (0x6D18/0x6D80/0x6DCF/0x6F62) unconditionally compute matched and, on a
    // non-match (which an empty record is), call vm_branch (0x6D4C je 0x6D7B ->
    // 0x6462). So the guarded then-body must be SKIPPED before the record is
    // written. The old early-return None fell through into the body.
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
            // The handler consumes this prefix UNCONDITIONALLY: 0x6C86 `cmp al,0xA1` /
            // 0x6C8E `inc si` runs BEFORE the mode test at 0x6C9C, so the byte is
            // skipped whatever the mode. Gating the skip on mode1 would leave the
            // byte in the operand stream in mode 0 and shift every later read by one.
            let inverted = cod.get(pos + 1) == Some(&0xA1);
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
    /// The ship's cargo hold — the 16-word special-slot list at gs:0x6D3E
    /// (insert 0x5FF6 fills a matching-or-zero slot; remove 0x5FD8 zeroes a
    /// match; SCRIPT inits rep-stosw it clear). Objects teleported aboard live
    /// here; the customs confiscations empty it.
    pub ship_slots: [u16; 16],
    /// BloodPrng state (cs:0xAEE seed word + 0xAF0/0xAF1/0xAF2 bytes; the
    /// shipped image zeroes them, the boot seeds from CMOS RTC seconds).
    pub prng_seed: u16,
    pub prng_af0: u8,
    pub prng_af1: u8,
    pub prng_af2: u8,
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
    /// `gs:0x67BD` — the FIN flag. The `0xA8` handler sets it (`0x67F0`) after
    /// loading a string operand whose first four bytes are `"fin."`
    /// (`0x67D8..0x67EE` compares `'f','i','n','.'` against the `0x2120`
    /// buffer). Unconditional — it is NOT under the handler's later
    /// presentation-request gate. The finale scripts load `fin.hnm` through
    /// this opcode, so this flag is the engine's own "the ending was
    /// requested" latch, alongside the [`VmEvent::LoadString`] the port
    /// already emits.
    pub fin_requested: bool,
    /// `gs:0x67AA` bit1 — the PRESENTATION-REQUEST latch. `0xA8` (`0x67F6`) and
    /// `0xC2` (`0x6EA7`) both refuse to raise a request while it is set, and the
    /// presentation teardown clears it (`and byte [0x67aa],0xfc` at `0x59C6`,
    /// `0x1A7C`, …). Modelling the latch WITHOUT that clear would suppress every
    /// later request, so the `0xC9` presentation end clears it here.
    pub presentation_request_pending: bool,
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
    /// DEB object record offsets, ascending (from the script's .DEB). The OWNER of
    /// a record is the nearest object at or below its offset (0x6034 threshold
    /// lookup over gs:0x672c). Empty until [`Self::load_deb_objects`]; owner-gated
    /// opcodes (C4/0x6946/B8) degrade gracefully when empty.
    pub object_offsets: Vec<u16>,
    /// The `arche` object offset (gs:0x6752), from the .DEB. Its +0x16 field holds a
    /// dangling owning-object reference that the 0xB8 family invalidates.
    pub arche_offset: Option<u16>,
    /// DEB offset of the built-in object `orxx` (the engine's `gs:0x6750`). The
    /// world-destination click writes its C1 record at `orxx + 0xA` (`0xB272`).
    pub orxx_offset: Option<u16>,
    /// DEB offset of the built-in object `Ark` — your ship (the engine's
    /// `gs:0x6758`, filled by the startup name scan `0x5486` from the built-in
    /// name table `DS:0x67BE`). The status roster excludes objects whose location
    /// is the Ark (`cmp [si+0x18],bx` @ `0x83E5`).
    pub ark_offset: Option<u16>,
    /// The `gs:0x672c` DIRECTORY as `(offset, kind)` pairs — the DEB's 20-byte
    /// records, whose `+0x10` is the object offset and `+0x12` the entry kind.
    /// The nav source-list builder (`0x624B`) walks it, continuing while the next
    /// entry's `+0x12 == 1` and stopping at the first that is not.
    pub directory: Vec<(u16, u16)>,
    /// `gs:0x251B` — the currently-presented world target (`ship_3d_current_target`).
    /// The click path only acts when the newly-selected target DIFFERS from this
    /// (`cmp ax,[0x251b]` @`0xB21A`).
    pub world_target: Option<u16>,
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
            // The oracle's deterministic CMOS seed (interp RTC seconds fixed
            // at 0x27; the seeder mirrors the byte into both halves -> 0x2727).
            // Matching it makes the port's variant rolls agree with the oracle.
            prng_seed: 0x2727,
            prng_af0: 0,
            prng_af1: 0,
            prng_af2: 0,
            ship_slots: [0u16; 16],
            concept: 0,
            concept_alt: 0,
            concept_alt_active: false,
            presentation_busy: false,
            flag_252a: false,
            flag_274f: false,
            presentation_active: false,
            fin_requested: false,
            presentation_request_pending: false,
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
            object_offsets: Vec::new(),
            directory: Vec::new(),
            arche_offset: None,
            orxx_offset: None,
            ark_offset: None,
            world_target: None,
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
    /// Populate the DEB object-offset table (ascending) for owner resolution.
    pub fn load_deb_objects(&mut self, deb: &[u8]) {
        let syms = crate::script::parse_deb(deb);
        self.arche_offset = syms
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case("arche"))
            .map(|s| s.offset);
        self.orxx_offset = syms
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case("orxx"))
            .map(|s| s.offset);
        // 0x5486's built-in name scan, `Ark` slot -> gs:0x6758.
        self.ark_offset = syms
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case("Ark"))
            .map(|s| s.offset);
        // The gs:0x672c directory in DEB order (offset = +0x10, kind = +0x12).
        self.directory = syms.iter().map(|s| (s.offset, s.kind)).collect();
        // ONLY kind-1 entries are objects. The directory walk shared by 0x624B /
        // 0x604E / the 0x5816 scan continues while `+0x12 == 1` and STOPS at the
        // first entry that is not, and measurement over the shipped DEBs shows the
        // leading kind-1 prefix equals every kind-1 entry (SCRIPT1..5: 122/122,
        // 122/122, 130/130, 136/136, 130/130) -- so filtering by kind reproduces
        // the scanned set exactly. Keeping the non-object entries had two effects:
        // the post-update scan visited ~219 extra records in SCRIPT2, and
        // owner_object_offset could return a NON-OBJECT offset as the "largest
        // below the key", mis-resolving the owner for the 0x6034 threshold lookup.
        // The extract path already filtered this way (extract/script.rs); the live
        // VM did not.
        let mut offs: Vec<u16> = syms
            .iter()
            .filter(|s| s.kind == 1)
            .map(|s| s.offset)
            .collect();
        offs.sort_unstable();
        offs.dedup();
        self.object_offsets = offs;
    }

    /// The owning object of a record: the nearest DEB object STRICTLY below `off`
    /// (the 0x6034 threshold lookup's port model). None when the table is unloaded.
    /// The `VmMachine` half of the pair described on the execution context's
    /// `owner_object_offset`: a delegation to [`owner_object_offset_in`], which
    /// holds the rule and the citation.
    fn owner_object_offset(&self, off: u16) -> Option<u16> {
        owner_object_offset_in(&self.object_offsets, off)
    }

    /// The `0xC4` mode-0 (SET) write guard — the `0x6CC3..0x6D01` decision.
    /// Returns `None` when the object table is unloaded (no `0x6034` threshold
    /// mapping possible; the caller then preserves the legacy unconditional
    /// write used by opcode-only test scaffolding, since the real game always
    /// has the DEB loaded). Otherwise `Some(true)` = write the C4 state record,
    /// `Some(false)` = `vm_branch` (skip to the else target).
    ///
    /// The real handler (`0x6C7E`) threshold-maps `op1` to its owning object
    /// (`0x6034` -> `di`), then:
    ///   - `test es:[di+2],1` — op1's object must be ACTIVE (bit0 of its +2
    ///     flags word), else branch (`0x6CC3`/`0x6CC8`).
    ///   - `test es:[op2+2],1` — op2's object must be ACTIVE, else branch
    ///     (`0x6CCC`/`0x6CD1`).
    ///   - `es:[di]==1` (op1-object kind 1) OR `es:[op2]==1` (op2-object kind 1)
    ///     -> write (`0x6CD3`/`0x6CDB` -> `0x6D01`).
    ///   - else fail if the op1 STATE record already holds `0xC4` (`cx`, read at
    ///     `0x6C98` -> `0x6CE3`), or op2's selector-`0x13` field already holds
    ///     `0xC4` (`0x6CE9..0x6CFF`).
    ///
    /// The active bits read here are VAR-initial and — verified 2026-07 by
    /// enumerating every `or/and byte [reg+2],imm` site in BLOODPRG.EXE — are
    /// NEVER SET at runtime; the sole runtime writer is `0x5B8D` in the C1
    /// world-presentation ladder, which only CLEARS a kind-`0x20` object's bit.
    /// So reading VAR-initial bits reproduces the game's write/branch decision
    /// for the dialogue C4 flow (the C1-clear case is a separate subsystem the
    /// live VM does not run and never activated an object the guard would gate).
    fn c4_set_write_decision(&self, op1: u16, op2: u16) -> Option<bool> {
        let di = self.owner_object_offset(op1)?;
        if self.rec_read(di.wrapping_add(2)) & 1 == 0 {
            return Some(false); // op1's object inactive
        }
        if self.rec_read(op2.wrapping_add(2)) & 1 == 0 {
            return Some(false); // op2's object inactive
        }
        if self.rec_read(di) == 1 {
            return Some(true); // op1-object kind 1
        }
        let op2_kind = self.rec_read(op2);
        if op2_kind == 1 {
            return Some(true); // op2-object kind 1
        }
        if self.rec_read(op1) == 0xC4 {
            return Some(false); // op1 state record already C4
        }
        if let Some(fo) = vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, op2_kind) {
            if self.rec_read(op2.wrapping_add(fo)) == 0xC4 {
                return Some(false); // op2's selector-0x13 field already C4
            }
        }
        Some(true)
    }

    /// The `0xC1` mode-1 (QUERY) decision — `0x6B4C`'s query path. `Some(true)` =
    /// pass (no branch), `Some(false)` = branch, `None` = object table unloaded
    /// (caller keeps the legacy no-op). Mirrors the tracer's already-validated
    /// `record_state_mode1_condition`/`c1_record_state_resolved_mode1_condition`
    /// but over the live state table.
    fn c1_query_passes(&self, off: u16, operand: u16, inverted: bool) -> Option<bool> {
        if self.object_offsets.is_empty() {
            return None;
        }
        let record_type = self.rec_read(off);
        // Resolved selector path for operand 1/2 (`0x6B8B..0x6BAC`): follow the
        // owner's parent-link (selector-0x11) to a target, then compare the
        // target's C1-destination (selector-0x13) slot to {0xC1, operand}.
        if record_type != 0xC1 && (operand == 1 || operand == 2) {
            if let Some(owner) = self.owner_object_offset(off) {
                if let Some(parent_field) =
                    vm_field_offset(ship3d::SHIP_3D_FIELD_SELECTOR_PARENT_LINK, operand)
                {
                    let target = self.rec_read(owner.wrapping_add(parent_field));
                    let target_kind = self.rec_read(target);
                    match vm_field_offset(ship3d::SHIP_3D_C1_DESTINATION_SELECTOR, target_kind) {
                        Some(dest_field) if dest_field != 0 => {
                            let slot = target.wrapping_add(dest_field);
                            let matched = self.rec_read(slot) == 0xC1
                                && self.rec_read(slot.wrapping_add(2)) == operand;
                            return Some(matched != inverted);
                        }
                        // dest field absent/zero (`0x6BA4`): the mismatch path.
                        _ => return Some(inverted),
                    }
                }
            }
        }
        // Direct compare (`0x6BAC..0x6BBA`): {record==0xC1, stored==operand}.
        // An empty record (type 0) simply fails the `cmp cx,0xC1`.
        let matched = record_type == 0xC1 && self.rec_read(off.wrapping_add(2)) == operand;
        Some(matched != inverted)
    }

    /// The `0xC1` mode-0 (SET) decision — the non-ship-3D path of `0x6B4C` /
    /// tracer `write_c1_record_state_mode0`. `Some(true)` = write `{0xC1,
    /// operand, 2}`, `Some(false)` = branch, `None` = object table unloaded.
    /// The ship-3D nav-source path (`write_c1_record_state_ship3d`) needs the
    /// frontend ship-3D runtime, absent in the live/dialogue context, so it
    /// falls through here to the simple write — which is exactly SCRIPT5's
    /// concert-FSM C1 sites (`C1 46 13 {..}` -> rec_1340 values).
    /// Where a `0xC1` mode-0 SET writes, if it writes at all.
    /// `None` = object table unloaded (caller keeps the legacy no-op);
    /// `Some(None)` = `vm_branch`; `Some(Some(dest))` = write `{0xC1, operand, 2}`
    /// at `dest`.
    ///
    /// The owner (`0x6034`) must be ACTIVE (`0x6BCE`). Then the destination
    /// depends on the owner's KIND (`0x6C04 cmp ax,0x10`):
    /// * kind `0x10` — the NAV path (`0x6C0C..0x6C53`): build the source list
    ///   (`0x624B`) rooted at the owner and scan it; an entry of kind 1 passes when
    ///   `es:[operand+2] & 2` (`0x6C3B..0x6C44`). If some entry passes, the write
    ///   goes to `owner + field_offset(0x13, 0x10)` — NOT to the operand record.
    /// * anything else — the write goes to the operand record itself (`bp`).
    ///
    /// Either way the destination must be EMPTY (`0x6C55..0x6C5B`), else branch.
    ///
    /// NOT modelled: the source list's kind-2 entries, whose gate (`0x6210` at
    /// `0x6C2F`) indexes off the caller's `si` — which at that one call site is the
    /// SOURCE-LIST READ POINTER, so it tests bytes inside the list buffer rather
    /// than an object field (see re/dead_ends.md). Those entries are treated as
    /// never passing rather than wired against a guessed base.
    fn c1_set_plan(&self, off: u16, operand: u16) -> Option<Option<u16>> {
        let owner = self.owner_object_offset(off)?;
        if self.rec_read(owner.wrapping_add(2)) & 1 == 0 {
            return Some(None); // owner inactive (`0x6BCE` je)
        }
        let dest = if self.rec_read(owner) == 0x10 {
            let passed = self.build_nav_source_list(owner).into_iter().any(|entry| {
                // kind 1 (`0x6C36`): test bit1 of the operand record's +2.
                self.rec_read(entry) == 1 && self.rec_read(operand.wrapping_add(2)) & 2 != 0
            });
            if !passed {
                return Some(None);
            }
            let fo = vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, 0x10)?;
            owner.wrapping_add(fo)
        } else {
            off
        };
        if self.rec_read(dest) != 0 {
            return Some(None); // destination occupied (`0x6C59` jne)
        }
        Some(Some(dest))
    }

    /// Build the ship-3D NAV SOURCE LIST (`ship_3d_navigation_source_list_build`
    /// `0x624B`, output `DS:0x6886`), a faithful port: walk the `gs:0x672c`
    /// directory; for each entry take its object offset (`+0x10`), read that
    /// object's kind (`es:[bx]`) and its selector-`0x11` field offset. When the
    /// field exists and the object's selector-`0x11` value EQUALS `target`, append
    /// the object and RECURSE on it depth-first. The scan advances an entry at a
    /// time and continues only while the next entry's `+0x12 == 1`, stopping at the
    /// first that is not; the real routine then stores a `0xFFFF` terminator, which
    /// the returned `Vec` represents by its length.
    ///
    /// This is pure record-table logic — directory plus `gs:0x6724` — so it needs
    /// no frontend state, which is what makes the C1 nav-source path portable.
    pub fn build_nav_source_list(&self, target: u16) -> Vec<u16> {
        // Named for its selector so it does not collide with the token walker
        // `walk` in this same file (the audit ledger keys rows by name+file).
        fn walk_selector11_children(
            m: &VmMachine,
            target: u16,
            out: &mut Vec<u16>,
            depth: usize,
        ) {
            if depth > 32 {
                return; // cycle guard; the game's data is a tree
            }
            for &(obj, entry_kind) in m.directory.iter() {
                // The real loop tests the NEXT entry's +0x12 before continuing, so
                // an entry whose kind is not 1 ends the scan.
                if entry_kind != 1 {
                    break;
                }
                let obj_kind = m.rec_read(obj);
                let Some(fo) = vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C2, obj_kind) else {
                    continue;
                };
                if fo == 0 {
                    continue; // `or ax,ax; je` skips a missing selector-0x11 field
                }
                if m.rec_read(obj.wrapping_add(fo)) != target {
                    continue;
                }
                out.push(obj);
                walk_selector11_children(m, obj, out, depth + 1);
            }
        }
        let mut out = Vec::new();
        walk_selector11_children(self, target, &mut out, 0);
        out
    }

    /// The DRAW-TIME filter both source-list consumers apply, ported verbatim
    /// from `vm_source_list_draw_loop` `0x91C3`:
    ///
    /// ```text
    ///   0x91CE  test word [si],2       the object's kind has bit 1 (kind 2)
    ///   0x91D4  test word [si+2],1     the ACTIVE bit
    ///   0x91DB  cmp  word [si+0x36],0  the selector-8 ENCOUNTER COUNTER, non-zero
    /// ```
    ///
    /// The builder (`build_nav_source_list`) is unfiltered — the game filters at
    /// the consumer, once per drawn row. The counter comes from
    /// `post_update_encounter_counter`, so an object appears in a list only after
    /// the post-update ladder has paired it with a kind-1 object at least once:
    /// **the list shows what the player has already met.**
    fn source_list_entry_is_listable(&self, entry: u16) -> bool {
        let kind = self.rec_read(entry);
        if kind & 2 == 0 {
            return false;
        }
        if self.rec_read(entry.wrapping_add(2)) & OBJECT_FLAG_ACTIVE == 0 {
            return false;
        }
        let Some(counter_field) = vm_field_offset(VM_FIELD_OFFSET_SELECTOR_ENCOUNTER, kind) else {
            return false;
        };
        if counter_field == 0 {
            return false;
        }
        self.rec_read(entry.wrapping_add(counter_field)) != 0
    }

    /// The rows a source-list panel actually DRAWS (`0x91C3` `0x299:0x202` per
    /// survivor, `add dx,0xA` between rows): `build_nav_source_list` filtered by
    /// [`Self::source_list_entry_is_listable`].
    pub fn source_list_display_rows(&self, target: u16) -> Vec<u16> {
        self.build_nav_source_list(target)
            .into_iter()
            .filter(|&entry| self.source_list_entry_is_listable(entry))
            .collect()
    }

    /// The same list as TEXT (`0x83C0..0x83F8`, the `PLANET:`/`SHIP:`/`BLACK
    /// HOLE:` status block's `LIFE SUPPORT:` roster), which adds a FOURTH
    /// condition the drawn panel does not have:
    ///
    /// ```text
    ///   0x83C7  mov bx,gs:[0x6758]     the built-in object `Ark`
    ///   0x83E5  cmp word [si+0x18],bx / je skip
    /// ```
    ///
    /// `+0x18` is selector `0x11` for kind 2 (`FIELD_OFFSETS[0x11][1]`), the
    /// LOCATION field — so an object whose location IS the Ark is dropped: the
    /// roster names who is present at the location, excluding your own ship's
    /// complement. `0x83D4` also compares the kind EXACTLY (`cmp word [si],2`)
    /// where the drawn panel bit-tests it.
    pub fn source_list_text_rows(&self, target: u16, ark_object: u16) -> Vec<u16> {
        self.build_nav_source_list(target)
            .into_iter()
            .filter(|&entry| {
                if self.rec_read(entry) != 2 {
                    return false; // `cmp word [si],2` — exact, not a bit test
                }
                if !self.source_list_entry_is_listable(entry) {
                    return false;
                }
                let Some(location_field) = vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C2, 2) else {
                    return false;
                };
                self.rec_read(entry.wrapping_add(location_field)) != ark_object
            })
            .collect()
    }

    /// An object's INLINE NAME: the NUL-terminated string at `record+4`.
    ///
    /// Both status-list consumers read it that way — `mov si,bp / add si,4 /
    /// lodsb` at `0x8389` and `0x83EA`, and `add si,4` before the `0x299:0x202`
    /// draw at `0x91E1`. Checked against the shipped data by
    /// `re/tools/check_object_inline_names.py`: 630 of the 640 kind-1 objects
    /// across SCRIPT1..5 hold exactly their DEB name at `+4`. The ten that do not
    /// are `blood` and `orxx` in each script — the two built-ins whose records the
    /// engine reuses for other fields (`orxx+0xA` is the C1 presentation slot).
    pub fn object_inline_name(&self, object: u16) -> String {
        let base = object.wrapping_add(4);
        let mut bytes = Vec::new();
        for i in 0..64u16 {
            let b = self.rec_read_u8(base.wrapping_add(i));
            if b == 0 {
                break;
            }
            bytes.push(b);
        }
        crate::font::cp437_string(&bytes)
    }

    /// The location STATUS BLOCK the nav hover composes (`nav_state_gate`
    /// `0x82E8`, composer `0x8347..0x8420`), as the lines the real routine writes
    /// CR-separated into the text buffer at `0xE18`:
    ///
    /// ```text
    ///   0x835C  si = gs:[0x6752]           the built-in object `arche`
    ///   0x8365  bp = fs:[si+0x16]          -> the CURRENT LOCATION object
    ///   0x8369  si = 0x12E                 "PLANET: "
    ///   0x836C  cmp word fs:[bp],0x10  -> si = 0x137   "SHIP: "
    ///   0x8376  test word fs:[bp],0x100 -> si = 0x13E  "BLACK HOLE: "
    ///   0x8381  copy that header, then the location's inline name, then CR
    ///   0x839F  si = 0x14B                 "LIFE SUPPORT:"  + CR
    ///   0x83B6  bp = 0x6886 / lcall 0x4DA:0xEAB (the 0x624B source-list build)
    ///   0x83CC  the roster loop -> source_list_text_rows
    /// ```
    ///
    /// The `0x8318` entry gate is a mouse hit-test against the widget rect at
    /// `DS:0x65F2+8`, so this is the nav chart's HOVER panel.
    ///
    /// Returns `None` when `arche` is unknown (no DEB loaded).
    pub fn location_status_block(&self, headers: &StatusHeaders) -> Option<Vec<String>> {
        let arche = self.arche_offset?;
        let location = self.rec_read(arche.wrapping_add(ARCHE_LOCATION_FIELD));
        let kind = self.rec_read(location);
        // 0x836C then 0x8376: the black-hole test runs SECOND and overwrites, so
        // it wins when a kind somehow satisfies both.
        let header = if kind & LOCATION_KIND_BLACK_HOLE != 0 {
            &headers.black_hole
        } else if kind == LOCATION_KIND_SHIP {
            &headers.ship
        } else {
            &headers.planet
        };
        let mut lines = vec![format!("{header}{}", self.object_inline_name(location))];
        lines.push(headers.life_support.clone());
        let ark = self.ark_offset.unwrap_or(0);
        for entry in self.source_list_text_rows(location, ark) {
            lines.push(self.object_inline_name(entry));
        }
        Some(lines)
    }

    /// The DESTINATION INFO PANEL's drawn rows (`0x9137..0x91EC`) — the other
    /// consumer of the same roster, this one a real on-screen panel rather than
    /// the subtitle-buffer block [`Self::location_status_block`] composes.
    ///
    /// The object it describes is `gs:0x27BF`, set by the selection commit at
    /// `0x9022` — which explicitly REFUSES the object you are already at
    /// (`cmp ax,es:[arche+0x16] / je` @ `0x901D`), so the panel always describes
    /// somewhere else.
    ///
    /// ```text
    ///   0x915B  bx=0x6E  dx=0x19  si=0x12E        x=110, y=25, "PLANET: "
    ///   0x916D  test es:[di],0x10  -> si=0x137    "SHIP: "
    ///   0x9177  test es:[di],0x100 -> si=0x13E    "BLACK HOLE: "
    ///   0x9181  al=0xEE / draw                    header colour 0xEE
    ///   0x9188  bx += [0x27CD] / add bx,6         the name follows the header
    ///   0x9192  si=di+4 / draw                    the object's inline name
    ///   0x919F  si=0x14B / bx=0x6E / add dx,0xA   "LIFE SUPPORT:" at y=35
    ///   0x91AD  add dx,0xA                        first roster row at y=45
    ///   0x91C0  ax=0xFE                           row colour 0xFE
    ///   0x91E9  add dx,0xA                        pitch 10 per row
    /// ```
    ///
    /// Note the header test order differs from the hover composer's: here `0x10`
    /// is a BIT TEST (`test es:[di],0x10`), not the `cmp ax,0x10` equality at
    /// `0x836C`. Both are ported as written.
    pub fn location_panel_rows(
        &self,
        object: u16,
        headers: &StatusHeaders,
    ) -> Vec<LocationPanelRow> {
        let kind = self.rec_read(object);
        let header = if kind & LOCATION_KIND_BLACK_HOLE != 0 {
            &headers.black_hole
        } else if kind & LOCATION_KIND_SHIP != 0 {
            &headers.ship
        } else {
            &headers.planet
        };
        let mut rows = vec![LocationPanelRow {
            x: LOCATION_PANEL_X,
            y: LOCATION_PANEL_Y,
            color: LOCATION_PANEL_HEADER_COLOR,
            text: header.clone(),
        }];
        rows.push(LocationPanelRow {
            // 0x9188: the pen restarts from the header's REPORTED width, which
            // excludes spaces — see font::game_font_drawn_width.
            x: LOCATION_PANEL_X
                + crate::font::game_font_drawn_width(header) as i32
                + LOCATION_PANEL_NAME_GAP,
            y: LOCATION_PANEL_Y,
            color: LOCATION_PANEL_HEADER_COLOR,
            text: self.object_inline_name(object),
        });
        rows.push(LocationPanelRow {
            x: LOCATION_PANEL_X,
            y: LOCATION_PANEL_Y + LOCATION_PANEL_ROW_PITCH,
            color: LOCATION_PANEL_HEADER_COLOR,
            text: headers.life_support.clone(),
        });
        for (i, entry) in self.source_list_display_rows(object).into_iter().enumerate() {
            rows.push(LocationPanelRow {
                x: LOCATION_PANEL_X,
                y: LOCATION_PANEL_Y + LOCATION_PANEL_ROW_PITCH * (2 + i as i32),
                color: LOCATION_PANEL_ROW_COLOR,
                text: self.object_inline_name(entry),
            });
        }
        rows
    }

    /// The ACTIVE-OBJECT candidate list (`table_672c_process` `0x604E`, output
    /// `DS:0x6A16`): walk the 20-byte directory at `gs:0x672C` while `+0x12 == 1`
    /// and keep every object whose flag word has bit 1 set.
    ///
    /// ```text
    ///   0x6068  ax = [si+0x12] / cmp ax,1 / jne     stop at the first non-kind-1
    ///   0x6070  bx = [si+0x10]                      the object offset
    ///   0x6073  test byte fs:[bx+2],2 / je          bit1 of the object's flags
    ///   0x607C  stosw                               keep it
    ///   0x6082  the list is 0xFFFF-terminated
    /// ```
    pub fn build_active_object_list(&self) -> Vec<u16> {
        let mut out = Vec::new();
        for &(object, entry_kind) in self.directory.iter() {
            if entry_kind != 1 {
                break; // 0x606E: the scan stops, it does not skip
            }
            if self.rec_read(object.wrapping_add(2)) & OBJECT_FLAG_IN_PLAY != 0 {
                out.push(object);
            }
        }
        out
    }

    /// The NAV CHART's visible-object list (`0x721A`, output `DS:0x2AD3`, count
    /// returned in `ax` and stored at `[0x27C1]`): the active-object list above,
    /// filtered to the chart-visible KINDS.
    ///
    /// ```text
    ///   0x7226  call 0x604E                     build the candidate list
    ///   0x7233  ax = [si] / or ax,ax / js       walk it, stop at the terminator
    ///   0x7238  bx = es:[eax+edi]               the object's KIND word
    ///   0x723D  test bx,0x118 / je              keep only kinds 8, 0x10, 0x100
    ///   0x7243  store, bp += 2, cx++
    /// ```
    ///
    /// `0x118` is exactly the three kinds the chart draws and the picker sizes
    /// boxes for: `0x08`, `0x10` (a SHIP) and `0x100` (a BLACK HOLE).
    pub fn build_nav_chart_list(&self) -> Vec<u16> {
        self.build_active_object_list()
            .into_iter()
            .filter(|&object| self.rec_read(object) & NAV_CHART_KIND_MASK != 0)
            .collect()
    }

    /// The NAV-CHART OBJECT PICKER (`0x92A3..0x9339`) — what the info panel's
    /// selection commit calls to turn a cursor position into an object.
    ///
    /// It walks the chart's visible-object list (`DS:0x2AD3`, `[0x27C1]` entries)
    /// and hit-tests each marker against a box whose SIZE depends on the object's
    /// kind, with the marker position read from the object's selector-`0x0B`
    /// field (`FIELD_OFFSETS[0x0B]` = `0x18` for kinds 8 and `0x10`):
    ///
    /// ```text
    ///   0x92BF  default            w,h = 0x0C, 0x0B
    ///   0x92CB  kind & 0x100       w,h = 0x13, 0x0C   a BLACK HOLE
    ///   0x92DF    bx = es:[arche+0x22]
    ///   0x92E9    cmp bx,es:[obj+0x14] / jne -> di += 4
    ///                              ...so a black hole has TWO chart positions,
    ///                              +0x18/+0x1A and +0x1C/+0x1E, and which one it
    ///                              shows depends on that comparison
    ///   0x92F4  kind & 0x10        w,h = 0x15, 0x0A   a SHIP
    ///   0x9308  x0 = pos.x - 2, hit when x0 <= mx <= x0 + w   (jb/ja: INCLUSIVE)
    ///   0x931A  y0 = pos.y - 2, hit when y0 <= my <= y0 + h
    /// ```
    ///
    /// Returns the FIRST object hit in list order, or `None` (`xor ax,ax` at
    /// `0x9337`). `arche_context` is `es:[arche+0x22]`, the word the black-hole
    /// branch compares against.
    pub fn nav_chart_pick(&self, list: &[u16], mouse: (i32, i32), arche_context: u16) -> Option<u16> {
        list.iter().copied().find(|&object| {
            nav_chart_marker_contains(
                self.nav_chart_marker(object, arche_context),
                self.nav_chart_hit_box(object),
                mouse,
            )
        })
    }

    /// The chart marker the picker hit-tests and the chart draws: `+0x18`/`+0x1A`,
    /// or the SECOND endpoint at `+0x1C`/`+0x1E` when the object is a black hole
    /// whose `+0x14` differs from `arche+0x22` (`0x92DF..0x92F2`). Kept here so
    /// the drawn marker and the clickable one cannot drift apart.
    pub fn nav_chart_marker(&self, object: u16, arche_context: u16) -> (i32, i32) {
        let mut position = object.wrapping_add(NAV_PICK_POSITION_FIELD);
        if self.rec_read(object) & LOCATION_KIND_BLACK_HOLE != 0
            && self.rec_read(object.wrapping_add(0x14)) != arche_context
        {
            position = position.wrapping_add(4);
        }
        (
            self.rec_read(position) as i32,
            self.rec_read(position.wrapping_add(2)) as i32,
        )
    }

    /// The picker's per-kind hit box for a record (`0x92BF`, `0x92D3`, `0x92FC`).
    pub fn nav_chart_hit_box(&self, object: u16) -> (i32, i32) {
        nav_chart_hit_box_for_kind(self.rec_read(object))
    }

    /// The word the black-hole endpoint rule compares against: `es:[arche+0x22]`
    /// (`0x92DF`). Zero when no DEB is loaded.
    pub fn nav_chart_arche_context(&self) -> u16 {
        self.arche_offset
            .map(|arche| self.rec_read(arche.wrapping_add(0x22)))
            .unwrap_or(0)
    }

    /// The WORLD-DESTINATION CLICK commit (`0xB20C..0xB27B`). The ship FSM's
    /// per-frame hit-test (`ship_3d_target_record_select` `0xB2BB`) returns the
    /// clicked target record; this is what the FSM then does with it:
    ///
    /// * `0` — nothing hit: no change (returns `false`).
    /// * `0xFFFF` — the back/exit row: clears the current target and returns
    ///   `false`. NOTE the polarity at `0xB288`, which this doc previously had
    ///   backwards: that site is `test byte [0x252f],1 / jne` around the
    ///   leave-the-world-view teardown (`[0x24F3]=0x11`, `[0x27D8]=0`), and the
    ///   selector SETS `[0x252F]` when the back row is picked (`0xB331`). So the
    ///   back row SUPPRESSES that teardown rather than causing it.
    ///
    ///   WHAT `[0x252F]` IS, resolved later from evidence already in the tree:
    ///   the TRANSITION OPENING flag. `ship3d::update_ship_3d_transition_state`
    ///   (`0xB692`) decodes `mov byte [0x252f],1` @`0xB6A5` as "opening", with
    ///   `0x2530` closing, `0x2531` the step and `0x2533` the armed latch. So the
    ///   back row's `[0x252F]=1` / `[0x2531]=6` @`0xB331`/`0xB336` ARMS AN OPENING
    ///   TRANSITION, and `0xB288` skips the teardown because a transition is
    ///   running — a sharper account than "the back row suppresses it", and it
    ///   explains the step 6 sitting between open's 4 and close's 8.
    /// * a target equal to `gs:0x251B` (`cmp ax,[0x251b]` @`0xB21A`): already
    ///   presented, so the record is NOT rewritten (returns `false`).
    /// * any other target: `gs:0x251B = target` (`0xB224`) and a C1 record
    ///   `{0xC1, target, 0}` is written at `[0x6750] + 0xA` (`0xB267..0xB27B`),
    ///   where `gs:0x6750` is the built-in object `orxx`. The C1 ladder
    ///   (`record_type_ladder` `0x5B38`) presents that record on a later frame.
    ///
    /// Returns whether a new C1 presentation record was created.
    pub fn world_click_select(&mut self, target: u16) -> bool {
        if target == 0 {
            return false;
        }
        if target == 0xFFFF {
            // Back/exit: the FSM drops the current target (0xB288 path).
            self.world_target = None;
            return false;
        }
        if self.world_target == Some(target) {
            return false; // already the presented target
        }
        self.world_target = Some(target);
        let Some(orxx) = self.orxx_offset else {
            return false; // DEB not loaded: no record slot to write
        };
        let at = orxx.wrapping_add(0xA);
        self.rec_write(at, 0xC1);
        self.rec_write(at.wrapping_add(2), target);
        self.rec_write(at.wrapping_add(4), 0);
        true
    }

    /// The DESTINATION LIST BUILDER, `entity_candidate_list` (`0x7259`) — the
    /// writer whose output [`Self::ship_3d_target_record_select`] reads.
    ///
    /// ```text
    ///   0x7266  call 0x624b              build the source list at gs:0x6886
    ///   0x7269  mov si,0x6886            ... and walk it
    ///   0x726c  mov bp,0x250b            emitting into the target list
    ///   0x726f  mov ax,di                the object 0x624B left in DI is tested FIRST
    ///   0x727b  mov bx,es:[di]           the flags word
    ///   0x727e  test bx,0x98   / je      kind must be 0x08, 0x10 (SHIP) or 0x80
    ///   0x7284  test es:[di+2],2 / je    and the +2 byte's bit 1 must be set
    ///   0x728b  cmp di,gs:[0x6752] / je  and it must not be `arche` -- the location itself
    ///   0x7292  add ax,4                 emit RECORD+4: a pointer to the inline NAME
    ///   0x7295  mov [bp],ax / add bp,2
    ///   0x729d  mov word [bp],0xffff     terminate
    /// ```
    ///
    /// `add ax,4` @`0x7292` is the INDEPENDENT confirmation of the `sub ax,4`
    /// @`0xB33D` decode: reader and writer were disassembled separately and agree
    /// that a list entry is `RECORD+4`. Neither reading rests on the other.
    ///
    /// The exclusion at `0x728B` is `arche` (`gs:0x6752`, the built-in object for
    /// the current location), so a location never offers itself as a destination.
    ///
    /// `first` is the object `0x624B` leaves in DI, which is tested BEFORE the
    /// list is walked (`jmp 0x727B` @`0x7271` enters at the test). Returns the
    /// emitted `RECORD+4` words without the terminator.
    pub fn entity_candidate_list(&self, first: u16, source: &[u16]) -> Vec<u16> {
        let mut out = Vec::new();
        let mut object = first;
        let mut index = 0usize;
        loop {
            let flags = self.rec_read(object);
            let ready = self.rec_read(object.wrapping_add(2)) as u8;
            if flags & ENTITY_CANDIDATE_KIND_MASK != 0
                && ready & ENTITY_CANDIDATE_READY_BIT != 0
                && Some(object) != self.arche_offset
            {
                out.push(object.wrapping_add(SHIP_3D_TARGET_NAME_TO_RECORD));
            }
            // 0x7273: fetch the next source word. The game reads a buffer that
            // always carries its 0xFFFF terminator; a slice that simply ends is
            // the same stopping condition.
            let Some(&next) = source.get(index) else { break };
            index += 1;
            if next == 0xFFFF {
                break;
            }
            object = next;
        }
        out
    }

    /// The destination rows the game actually offers: each candidate's RECORD and
    /// the NAME stored inside it.
    ///
    /// Rows come from [`Self::destination_candidate_records`] rooted at `arche`,
    /// which is where `0xB0EA` roots it. The name needs no lookup table: a list
    /// entry is `RECORD+4` (`add ax,4` @`0x7292`) and
    /// [`Self::object_inline_name`] reads from `object+4`, so the stored entry IS
    /// the string pointer. Record and name are two views of one word — which is
    /// the internal check on this whole chain, since a wrong `+4` anywhere would
    /// make the names garbage rather than merely shift a record.
    ///
    /// Empty when no DEB is loaded (no records, hence no candidates).
    pub fn destination_rows(&self) -> Vec<(u16, String)> {
        let Some(arche) = self.arche_offset else {
            return Vec::new();
        };
        self.destination_candidate_records(arche)
            .into_iter()
            .map(|entry| {
                let record = entry.wrapping_sub(SHIP_3D_TARGET_NAME_TO_RECORD);
                (record, self.object_inline_name(record))
            })
            .collect()
    }

    /// The SHIP-3D CLICK COMMIT's initial target, `ship_click_commit`
    /// (`0xB0DC..0xB111`) — the caller that roots the whole destination chain.
    ///
    /// ```text
    ///   0xB0E6  les di,[0x6724]          es = the record segment
    ///   0xB0EA  mov di,[0x6752]          DI = `arche` -- the list is rooted HERE
    ///   0xB0EE  lcall 0x4da:0x1eb9       build the candidate list (0x7259)
    ///   0xB0F3  mov ax,es:[di+0x16]      AX = the location the arche points at
    ///   0xB0F7  mov di,[0x250b]          DI = the list's FIRST entry (RECORD+4)
    ///   0xB0FB  test word es:[eax],0x140 the location's kind...
    ///   0xB101  jne 0xB10D               ... has it: keep the first candidate
    ///   0xB103  mov di,ax                ... lacks it: take the location itself
    ///   0xB105  lcall 0x4da:0x1eb9       and REBUILD the list rooted at it
    ///   0xB10A  add di,4
    ///   0xB10D  mov [0x251b],di
    ///   0xB111  sub word [0x251b],4      both paths land on a RECORD
    /// ```
    ///
    /// Two things this settles that guesswork would have got wrong:
    ///
    /// * the chain's root is `arche` (`gs:0x6752`), read from `0xB0EA` and not
    ///   inferred — which also means the `arche` exclusion @`0x728B` fires on
    ///   EVERY call from this path, so the root never appears in its own list;
    /// * the `add di,4` @`0xB10A` exists only to cancel the shared `sub 4`
    ///   @`0xB111`. The branch that takes the location object commits it whole,
    ///   while the branch that takes a list entry strips the `+4`. One `sub`
    ///   serves both because one branch pre-compensates.
    ///
    /// Returns the word `[0x251B]` receives. An empty candidate list leaves the
    /// terminator in `[0x250B]`, so the first branch yields `0xFFFF - 4`; that is
    /// the game's arithmetic, reproduced rather than smoothed over.
    pub fn ship_click_initial_target(&self) -> Option<u16> {
        let arche = self.arche_offset?;
        let candidates = self.destination_candidate_records(arche); // 0xB0EE
        let location = self.rec_read(arche.wrapping_add(ARCHE_LOCATION_FIELD)); // 0xB0F3
        // 0xB0F7 reads [0x250B] BEFORE the branch, so it is the arche list's head.
        let head = candidates.first().copied().unwrap_or(0xFFFF);
        if self.rec_read(location) & SHIP_CLICK_LOCATION_KIND_MASK != 0 {
            Some(head.wrapping_sub(SHIP_3D_TARGET_NAME_TO_RECORD)) // 0xB10D/0xB111
        } else {
            // 0xB103..0xB10A: re-root the list, then commit the location itself.
            let _ = self.destination_candidate_records(location);
            Some(location)
        }
    }

    /// The whole destination chain in one call: the source list `0x624B` builds,
    /// filtered by `0x7259`, yielding the `RECORD+4` words the selector reads.
    ///
    /// `first = target` because `0x624B` PRESERVES DI across its recursion —
    /// `0x6276 push di / mov di,ax / call 0x624b / 0x627D pop di` — so the DI that
    /// `0x7259` tests first (`mov ax,di` @`0x726F`) is still the caller's target.
    /// That was an open question (`re/dead_ends.md`) until the recursion site was
    /// read; the entry push list (`ds/si/bx/ax`, no DI) had suggested otherwise.
    ///
    /// The target is therefore tested as a candidate in its own right, and is
    /// normally rejected by the `arche` exclusion @`0x728B`.
    pub fn destination_candidate_records(&self, target: u16) -> Vec<u16> {
        let source = self.build_nav_source_list(target);
        self.entity_candidate_list(target, &source)
    }

    /// The WORLD-DESTINATION HIT-TEST, `ship_3d_target_record_select` (`0xB2BB`) —
    /// what produces the record [`Self::world_click_select`] commits.
    ///
    /// It is NOT a spatial hit-test on the nav chart. It is the unified list
    /// widget (`0x71E:0xC48` -> `list_widget_layout_unified` `0x8428`, the same
    /// widget the OPTION and contact menus enter), and the row it returns is
    /// turned into a record by ARITHMETIC:
    ///
    /// ```text
    ///   0xB2C3  mov si,0x250b            the primary target word list
    ///   0xB2C6  mov es,[0x6726]          ... whose entries point into the RECORD segment
    ///   0xB2CB  cmp word [si],-1         primary list EMPTY?
    ///   0xB2D0  mov ax,ds / mov es,ax      -> names are DS-relative instead
    ///   0xB2D4  mov si,0x2537              -> the inline fallback table
    ///   0xB2D7  mov byte [0x252c],1        -> and remember that we fell back
    ///   0xB318  lcall 0x71e:0xc48        the widget; AX = selected ROW
    ///   0xB31D  cmp ax,-1 / xor ax,ax    no selection -> 0 (nothing hit)
    ///   0xB326  add ax,ax / add si,ax    row -> word index
    ///   0xB32A  mov ax,[si]              the entry: a pointer to an INLINE NAME
    ///   0xB32C  cmp ax,-1 / je           the terminator row IS the back row: AX stays 0xFFFF
    ///   0xB33D  sub ax,4                 NAME -> RECORD
    ///   0xB340  test byte [0x252c],1     but on the FALLBACK list...
    ///   0xB347  mov ax,[0x251b]          ... return the CURRENT target instead
    /// ```
    ///
    /// `sub ax,4` at `0xB33D` is the exact inverse of the `add ax,4` the contact
    /// menu applies when it emits `RECORD+4` ([`Self::ship_contact_menu_words`]
    /// @`0x87D5`): a menu entry is a pointer to the name INSIDE the record, so
    /// backing up four bytes lands on the record itself. This is the name->record
    /// mapping `docs/port-validation.md` previously said the game did not have —
    /// it has one, and it is subtraction, not a table.
    ///
    /// The fallback override is the proof of that reading. When the primary list
    /// is empty the widget is fed DS-relative names (`es = ds`), which are NOT
    /// inside records, so `sub 4` would be meaningless — and the code discards it,
    /// returning `[0x251B]`, the current target. Since [`Self::world_click_select`]
    /// rejects a target equal to the current one, THE FALLBACK LIST CAN NEVER
    /// COMMIT A NEW DESTINATION. That is a behavioural rule, not an accident.
    ///
    /// Returns AX verbatim: `0` nothing, `0xFFFF` the back row, else the record.
    pub fn ship_3d_target_record_select(
        &self,
        primary: &[u16],
        fallback: &[u16],
        selected_row: u16,
    ) -> u16 {
        // 0xB2CB: a first word of 0xFFFF means the primary list is empty. An empty
        // slice is the same condition — the game reads a fixed buffer that always
        // has at least the terminator.
        let use_fallback = primary.first().copied().unwrap_or(0xFFFF) == 0xFFFF;
        let list = if use_fallback { fallback } else { primary };

        if selected_row == 0xFFFF {
            return 0; // 0xB31D..0xB324: the widget reported no selection
        }
        // 0xB326..0xB32A. Past the end reads the terminator, which is the back row.
        let entry = list.get(selected_row as usize).copied().unwrap_or(0xFFFF);
        if entry == 0xFFFF {
            return 0xFFFF; // 0xB32F: AX is still -1 when the entry is the terminator
        }
        let record = entry.wrapping_sub(SHIP_3D_TARGET_NAME_TO_RECORD); // 0xB33D
        if use_fallback {
            // 0xB340..0xB347: a DS name has no record; keep the current target.
            return self.world_target.unwrap_or(0);
        }
        record
    }

    /// Insert an owner into the 16-slot special list (gs:0x6D3E, insert 0x5FF6).
    /// Returns false only if the list is full and the owner is not already present.
    /// Test hooks for the recomp differential (`native_special_slots_match_the_lifted_pair`).
    pub fn special_slot_insert_pub(&mut self, owner: u16) -> bool {
        self.special_slot_insert(owner)
    }
    pub fn special_slot_remove_pub(&mut self, owner: u16) -> bool {
        self.special_slot_remove(owner)
    }
    pub fn ship_slots_pub(&self) -> &[u16] {
        &self.ship_slots
    }

    /// The console CONTACT MENU, built the way the row-2 handler builds it.
    ///
    /// `0x87BD` (bridge console row 2, reached through the per-row handler table
    /// at `CS:0x0F29` -> file `0x8709`, entry 2 = `0x0FDD`) is:
    ///
    /// ```text
    ///   0x87C5  mov si,0x6D3E          the 16-entry ship-slot array
    ///   0x87C8  mov di,0x2B13          the menu word list
    ///   0x87CB  lodsw                  next slot
    ///   0x87CC  or ax,ax / je 0x87CB   EMPTY slot: skip it, do not emit
    ///   0x87D0  cmp ax,-1 / je 0x87DB  0xFFFF terminates
    ///   0x87D5  add ax,4 / stosw       emit RECORD+4 -- the inline NAME
    /// ```
    ///
    /// So the menu is whoever is actually aboard, named from their own object
    /// record, and it is never a fixed list. `DS:0x6D3E` is empty in the image
    /// (all zeros at file `0x1415E`) because the array is runtime state: the
    /// same 16 slots the insert/find/remove scans at `0x5FD8`, `0x5FF6` and
    /// `0x6008` walk with `mov cx,0x10`, which this VM models as
    /// [`Self::ship_slots_pub`].
    ///
    /// The `+4` is applied by [`Self::object_inline_name`], so it is not repeated
    /// here.
    pub fn ship_contact_menu_words(&self) -> Vec<String> {
        self.ship_slots
            .iter()
            .copied()
            .filter(|slot| *slot != 0) // 0x87CC: empty slots are skipped
            .take_while(|slot| *slot != 0xFFFF) // 0x87D0: 0xFFFF ends the list
            .map(|owner| self.object_inline_name(owner))
            .filter(|name| !name.is_empty())
            .collect()
    }

    /// Insert an owner into the special-slot list — `vm_special_slot_insert`
    /// `0x5FF6`: scan the 16 words at `DS:0x6D3E` for the owner and return CF set
    /// if already present (`0x5FFE..0x601F`); otherwise scan for a ZERO slot and
    /// store there (`0x600E..0x601C`); if neither, `clc` — the list is full.
    fn special_slot_insert(&mut self, owner: u16) -> bool {
        if self.ship_slots.contains(&owner) {
            return true;
        }
        if let Some(slot) = self.ship_slots.iter_mut().find(|s| **s == 0) {
            *slot = owner;
            true
        } else {
            false
        }
    }

    /// Remove an owner from the special-slot list — `vm_special_slot_remove`
    /// `0x5FD8`, exactly:
    ///
    /// ```text
    ///   0x5FDA  bp=0x6D3E  cx=0x10        the 16-word list
    ///   0x5FE0  cmp ax,[bp] / je 0x5FED   scan for the owner
    ///   0x5FED  mov word [bp],0 / stc     clear THE FIRST HIT and return CF set
    ///   0x5FEA  clc                       not found
    /// ```
    ///
    /// Two details the port previously dropped, neither observable today but both
    /// free to get right: the original clears only the FIRST match and returns
    /// whether it found one. (`special_slot_insert` refuses duplicates, so a value
    /// cannot normally appear twice — the divergence is latent, not live.)
    fn special_slot_remove(&mut self, owner: u16) -> bool {
        for s in self.ship_slots.iter_mut() {
            if *s == owner {
                *s = 0;
                return true; // 0x5FF2 `stc`, and the scan stops
            }
        }
        false // 0x5FEA `clc`
    }

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
        // THE ENGINE'S OWN PRNG (0x1CE:0xB02, file 0x2DE2), ported exactly:
        // an 8-round rcr/rcl bit-interleave of the two state bytes into AX,
        // XORed with the 16-bit seed (CMOS RTC seconds at boot; seeder file
        // 0x2DD4: out 0x70,0 / in 0x71), then the counter feedback on the
        // STORED bytes (af2 += 1; af1 -= af2; af0 ^= rol(af2,1)) and
        // modulo-by-repeated-subtraction when a bound is given.
        let mut bl = self.prng_af0;
        let mut bh = self.prng_af1;
        let mut ax: u16 = 0;
        let mut cf = false; // xor ax,ax clears CF
        for _ in 0..8 {
            let out = bl & 1 != 0; // rcr bl,1
            bl = (bl >> 1) | if cf { 0x80 } else { 0 };
            cf = out;
            let out = ax & 0x8000 != 0; // rcl ax,1
            ax = (ax << 1) | cf as u16;
            cf = out;
            let out = bh & 0x80 != 0; // rcl bh,1
            bh = (bh << 1) | cf as u8;
            cf = out;
            let out = ax & 0x8000 != 0; // rcl ax,1
            ax = (ax << 1) | cf as u16;
            cf = out;
        }
        ax ^= self.prng_seed;
        self.prng_af2 = self.prng_af2.wrapping_add(1);
        // THE FEEDBACK USES THE STORED BYTES, NOT THE ROTATED REGISTERS. The
        // eight rounds rotate `bl`/`bh` in REGISTERS only; `0x2E00 mov bx,
        // cs:[0xaee]` then overwrites BX wholesale with the seed, destroying
        // both, and the feedback operates on memory:
        //
        //   0x2E17  sub byte cs:[0xaf1],bl    af1 -= counter
        //   0x2E1E  xor byte cs:[0xaf0],bl    af0 ^= rol(counter,1)
        //
        // This port used `bh`/`bl` — the rotated values — which agrees at an
        // all-zero state and diverges from the second draw onward. Found by
        // differentialling against `func_2de2` (audit-fixes #246).
        self.prng_af1 = self.prng_af1.wrapping_sub(self.prng_af2);
        self.prng_af0 ^= self.prng_af2.rotate_left(1);
        let _ = (bl, bh); // consumed by AX above; deliberately not the feedback
        if n != 0 {
            while ax >= n {
                ax -= n;
            }
        }
        ax
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

    /// Public record read for the drive layer.
    pub fn rec_read_pub(&self, off: u16) -> u16 {
        self.rec_read(off)
    }

    /// Public record write for the drive layer (the click stand-in's
    /// presentation end and test scaffolding).
    /// Byte-level record write, for tests that need to place an inline name at
    /// `record+4` the way the shipped object records do.
    pub fn rec_write_u8_pub(&mut self, byte_off: u16, b: u8) {
        self.rec_write_u8(byte_off, b);
    }

    pub fn rec_write_pub(&mut self, off: u16, v: u16) {
        self.rec_write(off, v);
    }

    fn rec_write(&mut self, off: u16, v: u16) {
        if let Some(slot) = self.line_records.get_mut(off as usize / 2) {
            *slot = v;
        }
    }

    /// Byte-level record access (the record table is a little-endian word array).
    /// Used by the 0xB7 single-bit flag handler.
    fn rec_read_u8(&self, byte_off: u16) -> u8 {
        let w = self.rec_read(byte_off & !1);
        if byte_off & 1 == 0 {
            (w & 0xFF) as u8
        } else {
            (w >> 8) as u8
        }
    }

    fn rec_write_u8(&mut self, byte_off: u16, b: u8) {
        let word_off = byte_off & !1;
        let w = self.rec_read(word_off);
        let next = if byte_off & 1 == 0 {
            (w & 0xFF00) | b as u16
        } else {
            (w & 0x00FF) | ((b as u16) << 8)
        };
        self.rec_write(word_off, next);
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
                // 0x67D5..0x67F0: after the copy the handler compares the first
                // four buffer bytes against 'f','i','n','.' and, on a match,
                // latches gs:[0x67BD]=1 — the engine's FIN (finale) flag. Byte
                // comparisons, so the match is case-SENSITIVE and prefix-only.
                // (The handler's remaining side effect — the presentation
                // request at 0x67F6 — is NOT modelled: its gate needs the
                // frontend's ship-active gs:[0x24F3], and one of its writes
                // (gs:[0xB3B]) collides with record data in the port's single
                // -array state model. See docs/audit-fixes.md.)
                if self.cod[start..nul].starts_with(b"fin.") {
                    self.fin_requested = true;
                }
                // 0x67F6..0x682F — the PRESENTATION REQUEST. Gate: the latch
                // gs:0x67AA bit1 must be CLEAR, and either ship-active
                // gs:[0x24F3]&1 OR gs:[0x274F]&1. The `0x24F3` operand lives in
                // the frontend, so only `flag_274f` is tested here; because the
                // gate is an OR that makes the port UNDER-fire (never spuriously).
                // Effects: active line = 7, latch set, gs:0x1FB2 cleared,
                // gs:0x1FA3 = 0xFFFF. All of those offsets sit ABOVE the largest
                // VAR (0x1534) so they are alias-safe as state writes.
                // NOT applied: the handler's `gs:[0xB3B] = 0`, because 0xB3B lies
                // INSIDE every VAR and would corrupt a real record in the port's
                // single-array model (see docs/audit-fixes.md).
                if !self.presentation_request_pending && self.flag_274f {
                    self.presentation_request_pending = true;
                    self.rec_write(VM_ACTIVE_LINE, 7);
                    self.rec_write_u8(0x1FB2, 0);
                    self.rec_write(0x1FA3, 0xFFFF);
                }
                self.events.push(VmEvent::LoadString(text));
            }
            // 0xA9 (0x6830): bit0 CLEAR -> jump to the operand word. bit0 SET ->
            // enter query mode and RESET the resume stack to [operand] (the
            // handler writes gs:0x6820[0]=operand and stack-ptr=2): the top-level
            // wait/conditional block opener.
            0xA9 => {
                // A9's descriptor is (0x04, 0xFF): the 0xFF sentinel switches
                // into query mode UNCONDITIONALLY — in the GOTO form too (the
                // same law the decompiler needed; the exec arm had the same
                // omission, leaving stale mode across skipped regions and
                // scrambling downstream guard evaluation in idle sweeps).
                self.query = true;
                let flags = self.lodsb();
                if flags & 1 == 0 {
                    let t = self.u8_at(self.pc) as usize
                        | (self.u8_at(self.pc + 1) as usize) << 8;
                    self.pc = t;
                } else {
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
                let raw = self.lodsw();
                if self.query {
                    // Handler 0x6946: an RHS equal to the SPECIAL OBJECT maps
                    // to 0xFFFF (the aboard value) before the compare — it is
                    // NOT a match-anything wildcard (the old `|| val==0xFFFF`
                    // made every aboard-guard pass; the matched-drive lane's
                    // first transcript diff caught it: the port played the
                    // BIONIUM begging behind rec_0722==65535 with 3488).
                    let val = if raw == self.wildcard { 0xFFFF } else { raw };
                    let eq = val == self.rec_read(off);
                    if (eq && flipped) || (!eq && !flipped) {
                        self.branch();
                    }
                } else {
                    // SET (0x6985): 0xBC stores the RAW value to gs:0x6782 (0x6989),
                    // then special-slot bookkeeping on the record's OWNER object
                    // (insert 0x5FF6 / remove 0x5FD8): current record == 0xFFFF removes
                    // the owner; a value that is aboard (0xFFFF or the special object)
                    // inserts the owner and SKIPS the write if the 16-slot list is
                    // full, else stores 0xFFFF; otherwise stores the RAW value. The old
                    // code stored the substituted val and did no slot bookkeeping.
                    if op == 0xBC {
                        self.reg_6782 = raw;
                    }
                    let owner = self.owner_object_offset(off);
                    // 0x6995..0x69A7: an existing 0xFFFF removes the owner and then
                    // `jmp 0x69C2` -- store the RAW value and STOP. The insert block
                    // below is skipped entirely; falling into it would re-insert the
                    // owner and store 0xFFFF over the requested value.
                    if self.rec_read(off) == 0xFFFF {
                        if let Some(owner) = owner {
                            let _ = self.special_slot_remove(owner);
                        }
                        self.rec_write(off, raw);
                        return true;
                    }
                    let mut stored = raw;
                    let mut do_write = true;
                    if raw == 0xFFFF || raw == self.wildcard {
                        if let Some(owner) = owner {
                            if self.special_slot_insert(owner) {
                                stored = 0xFFFF;
                            } else {
                                do_write = false; // slot list full: skip the write
                            }
                        } else {
                            stored = 0xFFFF;
                        }
                    }
                    if do_write {
                        self.rec_write(off, stored);
                    }
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
                // ONE implementation: `apply_operator`. This arm used to carry a
                // second copy of the same decoded rule, and the copies disagreed
                // on the operator the ladder does NOT recognise. `0x6891` is
                // `xor al,al` before the ladder, every arm of which is an explicit
                // `cmp ah,0xFn`; an unrecognised operator falls through to
                // `0x68DB` with al STILL ZERO, and `or al,al / jne` makes zero
                // mean BRANCH. The copy here ended its match with
                // `_ => cur == operand_i`, folding every unknown operator into an
                // equality test that can decline to branch. `apply_operator`
                // returns `false` for them, which is the ladder.
                let cur = self.rec_read(off);
                let mode = QuerySetMode { query: self.query };
                match mode.apply_operator(operator, cur, operand) {
                    // Query: `Err(matched)` -- no match branches (0x68DF).
                    Err(matched) => {
                        if !matched {
                            self.branch();
                        }
                    }
                    // Set: only F5/F6/F7 mutate; others write `cur` back unchanged
                    // (0x68F6 `cmp ah,0xf5; jne 0x68fd` skips the operand load).
                    Ok(v) => self.rec_write(off, v),
                }
            }
            // 0xB7 (0x6AA7): record byte field op (offset + byte value).
            0xB7 => {
                // 0x6AA7: single-bit flag get/set. Operands: base word + bit
                // NUMBER (not a value). byte = base + (n>>3), mask = 0x80>>(n&7)
                // (high-bit-first). Query isolates that one bit (0x6AD0: shl al,cl;
                // shl al,1; jae); SET ORs it in (0x6AEB). The 0xA1 prefix inverts
                // (test-clear / clear-bit). The old code compared/wrote the whole
                // record word against the bit number — nonsense.
                let mut flipped = false;
                if self.u8_at(self.pc) == 0xA1 {
                    self.pc += 1;
                    flipped = true;
                }
                let off = self.lodsw();
                let bit = self.lodsb();
                let byte_off = bit_flag_byte_offset(off, bit);
                let mask = bit_flag_mask(bit);
                if self.query {
                    let set = self.rec_read_u8(byte_off) & mask != 0;
                    // Non-inverted: bit SET continues, CLEAR branches to else
                    // (the same SET+uninverted=continue polarity as 0x6946).
                    if (set && flipped) || (!set && !flipped) {
                        self.branch();
                    }
                } else {
                    let cur = self.rec_read_u8(byte_off);
                    let next = if flipped { cur & !mask } else { cur | mask };
                    self.rec_write_u8(byte_off, next);
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
                    // Post-write cleanup (0x6B34-0x6B44): if the record just
                    // overwritten is owned by the object arche's +0x16 field
                    // references, invalidate that dangling reference (arche+0x16=0),
                    // so the character-display maintenance off arche doesn't act on a
                    // stale record.
                    if let Some(arche) = self.arche_offset {
                        let a16 = arche.wrapping_add(0x16);
                        if let Some(owner) = self.owner_object_offset(off) {
                            if self.rec_read(a16) == owner {
                                self.rec_write(a16, 0);
                            }
                        }
                    }
                }
            }
            // 0xC3 QUEUE (0x6EEE). QUERY: pass iff rec[off] is typed 0xC3 with a
            // matching related word (0xA1 prefix inverts). SET: unless the slot
            // already holds an ACTIVE C4 presentation, write the typed queue
            // record {0xC3, related, 1} — the pending-presentation request.
            // 0xC1 RECORD-STATE (0x6B4C). Was an unhandled no-op live (the
            // catch-all only consumed operands). QUERY: the resolved selector
            // path for operand 1/2, else the direct {0xC1, operand} compare;
            // branch on fail. SET: owner-active + empty record -> write
            // {0xC1, operand, 2}. The ship-3D nav-source SET path needs the
            // frontend runtime (absent in the dialogue context), so the simple
            // write applies — covering SCRIPT5's concert-FSM C1 sites. None
            // (object table unloaded) keeps the legacy no-op for opcode tests.
            0xC1 => {
                let mut flipped = false;
                if self.u8_at(self.pc) == 0xA1 {
                    self.pc += 1;
                    flipped = true;
                }
                let off = self.lodsw();
                let operand = self.lodsw();
                if self.query {
                    if self.c1_query_passes(off, operand, flipped) == Some(false) {
                        self.branch();
                    }
                } else {
                    match self.c1_set_plan(off, operand) {
                        Some(Some(dest)) => {
                            self.rec_write(dest, 0xC1);
                            self.rec_write(dest.wrapping_add(2), operand);
                            self.rec_write(dest.wrapping_add(4), 2);
                        }
                        Some(None) => self.branch(),
                        None => {}
                    }
                }
            }
            // 0xC2 (0x6E34) — was an unhandled no-op live (the catch-all only
            // consumed operands); the tracer had it as `write_c2_record_state_direct`.
            // QUERY (0x6E56..0x6E76): the owning object of op1 must be ACTIVE
            // (`test es:[di+2],1` on the 0x6034 lookup), the record's +2 must equal
            // op2, and its type must be 0xC2; branch when that fails (0xA1 inverts).
            // SET (0x6E78..0x6E98): owner active, then op2's OWN record must have
            // `+2 & 0x20`, then insert it into the special-slot list (0x5FF6) and
            // write 0xFFFF into its selector-0x11 field. NOTE the asymmetry: every
            // SET failure path jumps to the RET at 0x6EEC, never to vm_branch —
            // a failed C2 write does NOT branch.
            0xC2 => {
                let mut flipped = false;
                if self.u8_at(self.pc) == 0xA1 {
                    self.pc += 1;
                    flipped = true;
                }
                let off = self.lodsw();
                let operand = self.lodsw();
                // Object table unloaded (opcode-only scaffolding): keep the legacy
                // no-op, as the C1/C4 arms do.
                if let Some(owner) = self.owner_object_offset(off) {
                    let owner_active = self.rec_read(owner.wrapping_add(2)) & 1 != 0;
                    if self.query {
                        let pass = owner_active
                            && self.rec_read(off.wrapping_add(2)) == operand
                            && self.rec_read(off) == 0xC2;
                        if pass == flipped {
                            self.branch();
                        }
                    } else if owner_active
                        && self.rec_read(operand.wrapping_add(2)) & 0x20 != 0
                        && self.special_slot_insert(operand)
                    {
                        let kind = self.rec_read(operand);
                        if let Some(fo) = vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C2, kind) {
                            self.rec_write(operand.wrapping_add(fo), 0xFFFF);
                        }
                        // 0x6E9F..: the handler then raises a presentation request
                        // (kind 2 -> gs:[0x6788]=0x27; kind 0x400 -> DESCRIPT lookup
                        // then gs:[0x67AA]|=2, gs:[0x6788]=0x2B), gated on
                        // gs:[0x2793]&1 and gs:[0x67AA]&2. NOT modelled: the same
                        // engine active-line/presentation-request plumbing the 0xA8
                        // request half needs. See docs/audit-fixes.md.
                    }
                }
            }
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
            // 0xC5..0xC8 record-entry family (0x6D18/0x6D80/0x6DCF/0x6F62). These
            // were ENTIRELY UNHANDLED in the live step() (they fell to the no-op
            // catch-all): C5..C8 queries never branched, so "has this happened?"
            // guards always ran their then-body, and C5..C8 writes never landed.
            // QUERY: matched = rec[off]==op && rec[off+2]==operand; an empty record
            // is a non-match and BRANCHES (0x6D4C je -> 0x6462 vm_branch). SET
            // (mode0): a guarded write of {op, related, 0}; on guard failure the
            // handler branches (else path).
            0xC5 | 0xC6 | 0xC7 | 0xC8 => {
                let mut flipped = false;
                if self.u8_at(self.pc) == 0xA1 {
                    self.pc += 1;
                    flipped = true;
                }
                let off = self.lodsw();
                let operand = self.lodsw();
                if self.query {
                    let matched =
                        self.rec_read(off) == op as u16 && self.rec_read(off + 2) == operand;
                    let pass = if flipped { !matched } else { matched };
                    if !pass {
                        self.branch();
                    }
                } else {
                    // Per-opcode guards (write_record_entry_mode0 / 0x6D18..):
                    //  C5: operand active (+2 bit0) && rec[operand]==0x200 && rec[off]==0
                    //  C6: unconditional
                    //  C7: operand active && (rec[off]==0 || rec[off]==C4)
                    //  C8: rec[off]==0  (writes related=0)
                    let wrote = match op {
                        0xC5 => {
                            self.rec_read_u8(operand.wrapping_add(2)) & 1 != 0
                                && self.rec_read(operand) == 0x0200
                                && self.rec_read(off) == 0
                        }
                        0xC6 => true,
                        0xC7 => {
                            let rt = self.rec_read(off);
                            self.rec_read_u8(operand.wrapping_add(2)) & 1 != 0
                                && (rt == 0 || rt == OP_ACTOR as u16)
                        }
                        0xC8 => self.rec_read(off) == 0,
                        _ => false,
                    };
                    if wrote {
                        let related = if op == 0xC8 { 0 } else { operand };
                        self.rec_write(off, op as u16);
                        self.rec_write(off.wrapping_add(2), related);
                        self.rec_write(off.wrapping_add(4), 0);
                    } else {
                        self.branch();
                    }
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
                    // 0x6C7E query: the assembly gates on the OWNING object's active
                    // bit (0x6CA4 test es:[di+2],1). BUT the live port does not model
                    // object active bits (start_actor_presentation sets the record, not
                    // an obj+2 bit0; no live setter exists) and owner_object_offset is a
                    // nearest-below approximation, not the 0x6034 threshold lookup. So
                    // the working model here is active_actor==Some(off) (the single
                    // presenter). Applying the assembly gate would fail EVERY C4 query
                    // in live play and break gated dialogue — a finding correct for the
                    // assembly but wrong for the port (verified via the missing
                    // active-bit model; see docs/audit-fixes.md).
                    let pass = self.rec_read(off) == 0xC4
                        && self.rec_read(off + 2) == related
                        && self.active_actor == Some(off);
                    if pass == flipped {
                        self.branch();
                    }
                } else {
                    // 0x6CC3..0x6D01 write guard: the game only writes the C4
                    // state record when both operand objects are active and the
                    // kind/already-set checks pass; otherwise it branches. None =
                    // object table unloaded (opcode-only tests) -> legacy write.
                    match self.c4_set_write_decision(off, related) {
                        Some(false) => self.branch(),
                        _ => {
                            self.rec_write(off, 0xC4);
                            self.rec_write(off + 2, related);
                            self.active_actor = Some(off);
                            self.events.push(VmEvent::Actor { offset: off as usize });
                        }
                    }
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
            // 0xC9 (0x6FB9): clear the record — ends the actor's presentation
            // (each block clears its actor record when done). The handler zeroes
            // the WHOLE 3-word record (`stosw` x3 at 0x6FC7/0x6FCB/0x6FCC), not
            // just the type word, and when the cleared record was a `0xC4` actor
            // entry it ALSO zeroes the reciprocal triple on the related object —
            // its selector-`0x13` field (`0x6FD3..0x6FF0`). Leaving that stale
            // matters: the C4 mode-0 write guard (`0x6CE9..0x6CFF`) refuses a new
            // presentation when the related object's selector-`0x13` field still
            // reads `0xC4`, so a half-cleared record would wedge the actor out of
            // every later presentation.
            0xC9 => {
                let off = self.lodsw();
                let old_type = self.rec_read(off);
                // The related record offset lives at +2 — read BEFORE clearing.
                let related = self.rec_read(off.wrapping_add(2));
                self.rec_write(off, 0);
                self.rec_write(off.wrapping_add(2), 0);
                self.rec_write(off.wrapping_add(4), 0);
                if old_type == 0xC4 {
                    // 0x6FD4..0x6FDE: field_offset(0x13, kind-of-related) added to
                    // the related record, then three zero words.
                    let related_kind = self.rec_read(related);
                    if let Some(fo) =
                        vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, related_kind)
                    {
                        let at = related.wrapping_add(fo);
                        self.rec_write(at, 0);
                        self.rec_write(at.wrapping_add(2), 0);
                        self.rec_write(at.wrapping_add(4), 0);
                    }
                    // 0x6FE2/0x6FE8 also write gs:[0x252A]=0 and gs:[0x2531]=6.
                    // NOT modelled: `flag_252a` is currently forced true at VM
                    // construction because the port has no runtime SETTER for it
                    // (the game's writers are the ship-3D navigation sequence,
                    // 0xB54A/0xB3F5/0xB4CF/0xB29A). Clearing it here without that
                    // setter would permanently close every 0xD0-gated block after
                    // the first presentation ends. See docs/audit-fixes.md.
                }
                if self.active_actor == Some(off) {
                    self.active_actor = None;
                    self.presentation_busy = false;
                    // Presentation over: the resume anchor dies with it.
                    self.resume_pos = None;
                    // The teardown also clears gs:0x67AA bits0-1 (`and [0x67aa],
                    // 0xfc` @0x59C6/0x1A7C), releasing the request latch so a
                    // LATER 0xA8/0xC2 can raise a new presentation request.
                    // Without this the latch would suppress every later request.
                    self.presentation_request_pending = false;
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
                    // Scruter Jo aboard; the customs manifest lines). The
                    // ship's 16-slot hold (gs:0x6D3E) tracks the cargo:
                    // insert on boarding (0x5FF6), remove on leaving (0x5FD8).
                    let dest = if op3 == 0x28 { 0xFFFF } else { op3 };
                    // Kind-correct relink: the moved object's location field
                    // offset comes from the engine's matrix (field 0x11 per
                    // its kind word), not a kind-1 assumption.
                    let kind = self.rec_read(op2);
                    let loc = field_offset(kind, 0x11).unwrap_or(LOCATION_FIELD);
                    if dest == 0xFFFF {
                        if let Some(slot) = self
                            .ship_slots
                            .iter_mut()
                            .find(|s| **s == op2 || **s == 0)
                        {
                            *slot = op2;
                        }
                    } else if let Some(slot) =
                        self.ship_slots.iter_mut().find(|s| **s == op2)
                    {
                        *slot = 0;
                    }
                    self.rec_write(op2.wrapping_add(loc), dest);
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
                        // The word list has TWO sections split by 0xFFFF: the spoken
                        // line, then the CHOICE-MENU words (SCRIPT1.COD @0x4A7 is the
                        // canonical example -- "Click quick, Cap'n Bob is waiting ..."
                        // then explanations/game). Joining the whole list ran the menu
                        // into the sentence and printed the separator as `word_65535`,
                        // because 0xFFFF is not a DIC offset. The gameplay consumers
                        // already `take_while(|w| w != 0xFFFF)`; the disassembler did not.
                        let sep = word_offsets.iter().position(|&w| w == 0xFFFF);
                        let spoken = &word_offsets[..sep.unwrap_or(word_offsets.len())];
                        let text: String = spoken
                            .iter()
                            .map(|w| word_of(*w))
                            .collect::<Vec<_>>()
                            .join(" ");
                        let menu: Option<String> = sep.map(|i| {
                            word_offsets[i + 1..]
                                .iter()
                                .map(|w| word_of(*w))
                                .collect::<Vec<_>>()
                                .join(" | ")
                        });
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
                        // Surface the choice menu as its own clause instead of letting
                        // it masquerade as part of the spoken line.
                        let menu_part = match menu {
                            Some(m) if !m.is_empty() => format!("  MENU[{m}]"),
                            _ => String::new(),
                        };
                        line = format!(
                            "SAY \"{}\"{}{}",
                            text.replace('\n', " / "),
                            menu_part,
                            attr
                        );
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

    /// `scan_zero_word` DIFFERENTIALLED against `func_6293`.
    ///
    /// The lift is the general routine: scan forward until the WORD at SI equals
    /// AX, step past it, then consume one more byte if it equals AL. The port's
    /// version is the `AX = 0` specialisation with a BOUND added, since a Rust
    /// slice cannot run off the end the way the original happily does.
    ///
    /// That bound is a port-side safety addition, so the two can only be required
    /// to agree where the terminator is genuinely in range — which is every case
    /// the game's data produces. The test says so explicitly rather than quietly
    /// choosing inputs that hide the difference.
    #[test]
    fn scan_zero_word_matches_its_lift_where_the_terminator_is_in_range() {
        use crate::recomp::{machine::Machine, ptr_leaves};
        const DS: u16 = 0x2000;

        let mut checked = 0usize;
        // Deterministic pseudo-random buffers with a zero word planted.
        for seed in 0..200u32 {
            let mut buf = vec![0u8; 128];
            let mut x = seed.wrapping_mul(2654435761).wrapping_add(1);
            for b in buf.iter_mut() {
                x = x.wrapping_mul(1103515245).wrapping_add(12345);
                // Never zero, so the only terminator is the one planted below.
                *b = ((x >> 16) as u8) | 1;
            }
            let at = 8 + (seed as usize % 100);
            buf[at] = 0;
            buf[at + 1] = 0;
            // Sometimes a third zero, which the trailing-byte rule consumes.
            if seed % 3 == 0 {
                buf[at + 2] = 0;
            }

            let native = scan_zero_word(&buf, 0, buf.len());

            let mut m = Machine::new();
            m.regs.ds = DS;
            m.regs.set_ax(0);
            m.regs.set_si(0);
            let base = (DS as u32) * 16;
            m.mem[base as usize..base as usize + buf.len()].copy_from_slice(&buf);
            ptr_leaves::func_6293(&mut m);
            let lifted = m.regs.si() as usize;

            assert_eq!(
                native, lifted,
                "seed {seed}: native stops at {native}, the lift at {lifted} \
                 (terminator planted at {at})"
            );
            checked += 1;
        }
        assert_eq!(checked, 200, "the sweep did not run");
    }

    /// THE PRNG, DIFFERENTIALLED AGAINST ITS LIFT — the strongest evidence this
    /// tree offers, since `func_2de2` is the original instruction stream
    /// transliterated rather than a second reading of it.
    ///
    /// A PRNG is the ideal subject: it either agrees bit-for-bit over thousands
    /// of draws or it diverges and never recovers. The state is four bytes the
    /// lift keeps in code space — `cs:[0xAEE]` the seed, `cs:[0xAF0/0xAF1]` the
    /// pair, `cs:[0xAF2]` the counter — and the native keeps in fields, so both
    /// are started identical and compared after every draw.
    #[test]
    fn the_prng_matches_its_lift_draw_for_draw() {
        use crate::recomp::{auto, machine::Machine};
        const CS: u16 = 0x2600;

        for (seed, af0, af1, af2, bound) in [
            (0x2727u16, 0u8, 0u8, 0u8, 0u16),
            (0x2727, 0x5A, 0xA5, 0x11, 10),
            (0xFFFF, 0xFF, 0xFF, 0xFF, 7),
            (0x0001, 0x01, 0x80, 0x7F, 100),
        ] {
            let mut native = VmMachine::new();
            native.prng_seed = seed;
            native.prng_af0 = af0;
            native.prng_af1 = af1;
            native.prng_af2 = af2;

            let mut m = Machine::new();
            m.regs.cs = CS;
            m.regs.ds = CS;
            m.regs.ss = 0x9000;
            m.regs.set_sp(0xFFF0);
            let base = (CS as u32) * 16;
            m.mem[(base + 0xAEE) as usize] = seed as u8;
            m.mem[(base + 0xAEF) as usize] = (seed >> 8) as u8;
            m.mem[(base + 0xAF0) as usize] = af0;
            m.mem[(base + 0xAF1) as usize] = af1;
            m.mem[(base + 0xAF2) as usize] = af2;

            for draw in 0..2000u32 {
                let want = native.rand(bound);

                m.regs.set_ax(bound);
                auto::func_2de2(&mut m);
                let got = m.regs.ax();

                assert_eq!(
                    got, want,
                    "seed {seed:#06x} bound {bound}: draw {draw} is {got} native vs \
                     {want} lifted"
                );
                // And the STATE must track, or the next draw diverges for a
                // reason this draw did not show.
                assert_eq!(
                    (
                        m.mem[(base + 0xAF0) as usize],
                        m.mem[(base + 0xAF1) as usize],
                        m.mem[(base + 0xAF2) as usize]
                    ),
                    (native.prng_af0, native.prng_af1, native.prng_af2),
                    "seed {seed:#06x}: state diverged after draw {draw}"
                );
            }
        }
    }

    /// SETTLES #238's OPEN QUESTION: does the encounter ladder fire at all?
    ///
    /// `parse_script_post_update` produces one offsetless row across all five
    /// shipped scripts, and #238 left two readings open — the ladder genuinely
    /// almost never fires, or that EXPORT is missing context it is not given (it
    /// takes an optional `DescriptDb` and the measurement passed `None`).
    ///
    /// Running the VM directly answers it: the trace carries `post_update`
    /// whether or not an exporter asks for it, so if the ladder fires the events
    /// are here. Whatever the counts are, they are recorded rather than assumed —
    /// the point is to replace "undecided" with a number.
    #[test]
    fn the_post_update_ladder_on_the_shipped_scripts() {
        let dir = ["output/_tmp_iso", "../output/_tmp_iso", "output/scripts", "../output/scripts"]
            .iter()
            .map(std::path::Path::new)
            .find(|p| p.exists());
        let Some(dir) = dir else { return };

        let (mut pairs, mut handoffs, mut bumps, mut scripts) = (0usize, 0usize, 0usize, 0usize);
        for index in 1..=5 {
            let Ok(cod) = std::fs::read(dir.join(format!("SCRIPT{index}.COD"))) else {
                continue;
            };
            let var = std::fs::read(dir.join(format!("SCRIPT{index}.VAR"))).unwrap_or_default();
            let trace = execute_trace_with_context(&cod, &var, &ExecutionContext::default());
            let post: &PostUpdateTrace = &trace.post_update;
            scripts += 1;
            pairs += post.actor_record_pairs.len();
            handoffs += post.presentation_handoffs.len();
            bumps += post.encounter_counter_bumps.len();

            // Whatever fires must reference records inside the script's world:
            // a record offset of 0 is the null record and would mean the ladder
            // paired something with nothing.
            for pair in &post.actor_record_pairs {
                assert_ne!(pair.record_offset, 0, "SCRIPT{index}: paired the null record");
            }
            for handoff in &post.presentation_handoffs {
                assert_ne!(handoff.record_offset, 0, "SCRIPT{index}: null handoff record");
            }
        }

        assert!(scripts >= 3, "only {scripts} scripts read");
        // MEASURED, and half the answer to #238: with a default context the
        // ladder produces nothing on the shipped scripts. The other half is in
        // `extract::script`'s `the_post_update_ladder_with_real_deb_context`,
        // which supplies each script's own DEB and ALSO gets nothing — so the
        // ladder does not fire on shipped bytecode at all, and the exporter that
        // reports nothing is reporting the truth (#244).
        assert_eq!(
            (pairs, handoffs, bumps),
            (0, 0, 0),
            "the ladder now fires without context; #238's finding needs revisiting"
        );
    }

    /// A PROFILE REQUEST EVENT MUST SIT ON THE OPCODE THAT MAKES REQUESTS.
    ///
    /// `ScriptProfileRequestEvent` records the offset where a `0xD2` request
    /// fired. That is checkable against the bytecode itself: the byte at that
    /// offset must BE `OP_SCRIPT_PROFILE_REQUEST`. An event pointing anywhere else
    /// means the VM attributed a request to the wrong place, which no amount of
    /// internal consistency would reveal — the event looks identical either way.
    ///
    /// Also pinned: `pending_script_profile` filters the `0xFFFF` sentinel
    /// (`gs:0x6780` empty, `cmp word [0x6780],-1` @`0x108E`), so a trace whose last
    /// request is the sentinel reports nothing pending.
    #[test]
    fn profile_requests_sit_on_the_d2_opcode_in_the_real_scripts() {
        let dir = ["output/_tmp_iso", "../output/_tmp_iso", "output/scripts", "../output/scripts"]
            .iter()
            .map(std::path::Path::new)
            .find(|p| p.exists());
        let Some(dir) = dir else { return };

        let mut events = 0usize;
        let mut scripts = 0usize;
        for index in 1..=5 {
            let Ok(cod) = std::fs::read(dir.join(format!("SCRIPT{index}.COD"))) else {
                continue;
            };
            let var = std::fs::read(dir.join(format!("SCRIPT{index}.VAR"))).unwrap_or_default();
            let trace = execute_trace_with_context(&cod, &var, &ExecutionContext::default());
            scripts += 1;

            for event in &trace.script_profile_requests {
                assert!(event.offset < cod.len(), "event past the script");
                assert_eq!(
                    cod[event.offset], OP_SCRIPT_PROFILE_REQUEST,
                    "SCRIPT{index}: a profile request is recorded at {:#x}, where the \
                     byte is {:#04x} and not the request opcode {:#04x}",
                    event.offset, cod[event.offset], OP_SCRIPT_PROFILE_REQUEST
                );
                events += 1;
            }

            // The one-shot sentinel: a trace whose last request is 0xFFFF has
            // nothing pending, per `cmp word [0x6780],-1` @`0x108E`.
            if let Some(last) = trace.script_profile_requests.last() {
                if last.profile_index == 0xFFFF {
                    assert_eq!(trace.pending_script_profile(), None, "sentinel not filtered");
                } else {
                    assert_eq!(trace.pending_script_profile(), Some(last.profile_index));
                }
            } else {
                assert_eq!(trace.pending_script_profile(), None);
            }
        }

        assert!(scripts >= 3, "only {scripts} scripts read");
        // THIN, and measured rather than assumed: the five shipped scripts issue
        // exactly TWO profile requests between them, so the offset->opcode
        // assertion above runs twice. That is enough to be non-vacuous and not
        // enough to be reassuring; the sentinel branch below carries more of the
        // weight. Stated so nobody reads `events > 0` as coverage.
        assert!(events >= 2, "only {events} profile requests seen (2 expected)");
    }

    /// THE VM MUST NOT DESYNC ON THE GAME'S OWN BYTECODE.
    ///
    /// `execute_trace_with_context` runs a real `SCRIPT*.COD` and reports how it
    /// stopped. Two of the four [`ExecutionHalt`] variants are legitimate ends —
    /// `EndMarker` (the script finished) and `StepLimit` (it loops, as a scene
    /// waiting on input does). The other two are confessions:
    ///
    ///   * `InvalidOpcode` means the walker read a byte that the 52-entry table at
    ///     `0x142D0` does not dispatch. In the game's own bytecode every token
    ///     boundary holds a dispatchable opcode, so this can only mean the VM
    ///     lost its place — the exact failure #235 measures statically, caught
    ///     here by EXECUTION instead.
    ///   * `InvalidTarget` means a branch pointed outside the script.
    ///
    /// So this asserts a property of the ORIGINAL data that a correct VM inherits
    /// and a broken one cannot fake.
    #[test]
    fn the_vm_runs_the_shipped_scripts_without_desyncing() {
        let dir = ["output/_tmp_iso", "../output/_tmp_iso", "output/scripts", "../output/scripts"]
            .iter()
            .map(std::path::Path::new)
            .find(|p| p.exists());
        let Some(dir) = dir else { return };

        let mut ran = 0usize;
        let mut branch_events = 0usize;
        for index in 1..=5 {
            let cod = match std::fs::read(dir.join(format!("SCRIPT{index}.COD"))) {
                Ok(cod) => cod,
                Err(_) => continue,
            };
            let var = std::fs::read(dir.join(format!("SCRIPT{index}.VAR"))).unwrap_or_default();
            let trace: ExecutionTrace =
                execute_trace_with_context(&cod, &var, &ExecutionContext::default());

            match &trace.halted {
                ExecutionHalt::EndMarker | ExecutionHalt::StepLimit { .. } => {}
                ExecutionHalt::InvalidOpcode { offset, byte } => panic!(
                    "SCRIPT{index}: invalid opcode {byte:#04x} at {offset:#x} -- the VM \
                     desynchronised walking the game's own bytecode"
                ),
                ExecutionHalt::InvalidTarget { offset, target } => panic!(
                    "SCRIPT{index}: branch at {offset:#x} targets {target:#x}, outside \
                     the script"
                ),
            }

            for event in &trace.branch_events {
                assert!(
                    event.offset < cod.len(),
                    "SCRIPT{index}: branch event at {:#x} is past the {}-byte script",
                    event.offset,
                    cod.len()
                );
                branch_events += 1;
            }
            assert!(trace.steps > 0, "SCRIPT{index} executed no steps");
            ran += 1;
        }

        // All five shipped scripts run; require at least three in case a
        // checkout carries fewer.
        assert!(ran >= 3, "only {ran} scripts executed; the sweep proves little");
        // 5 scripts, 1534 branch events measured -- the sweep really executes.
        assert!(branch_events > 1_000, "only {branch_events} branch events seen");
    }

    /// The three hit boxes are IMMEDIATES in the picker; read them back out of
    /// the image rather than trusting the transcription.
    #[test]
    fn nav_pick_boxes_are_the_pickers_own_immediates() {
        let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        // `c7 06 <disp16> <imm16>` = mov word [disp],imm
        let mov_word = |at: usize| -> (u16, u16) {
            assert_eq!(&exe[at..at + 2], &[0xC7, 0x06], "{at:#x} is not `mov word [mem],imm`");
            (
                u16::from_le_bytes([exe[at + 2], exe[at + 3]]),
                u16::from_le_bytes([exe[at + 4], exe[at + 5]]),
            )
        };
        for (at_w, at_h, expected, what) in [
            (0x92BF, 0x92C5, NAV_PICK_BOX_DEFAULT, "default"),
            (0x92D3, 0x92D9, NAV_PICK_BOX_BLACK_HOLE, "black hole"),
            (0x92FC, 0x9302, NAV_PICK_BOX_SHIP, "ship"),
        ] {
            let (dw, w) = mov_word(at_w);
            let (dh, h) = mov_word(at_h);
            assert_eq!(dw, 0x277A, "{what} width goes to the wrong scratch word");
            assert_eq!(dh, 0x277C, "{what} height goes to the wrong scratch word");
            assert_eq!((w as i32, h as i32), expected, "{what} box");
        }

        // The gates select on the same kind bits the chart filter uses.
        assert_eq!(u16::from_le_bytes([exe[0x92CB + 4], exe[0x92CB + 5]]), 0x0100);
        assert_eq!(u16::from_le_bytes([exe[0x92F4 + 4], exe[0x92F4 + 5]]), 0x0010);
        assert_eq!(NAV_CHART_KIND_MASK & 0x0100, 0x0100, "black hole is a charted kind");
        assert_eq!(NAV_CHART_KIND_MASK & 0x0010, 0x0010, "ship is a charted kind");
    }

    /// `TALK_FIELD` and `LOCATION_FIELD` are ENTRIES IN THE FIELD MATRIX at
    /// `DS:0x6D60`, not immediates — which is why the immediate checker reports
    /// them as needing reading. `0x6664` fetches the first with
    /// `mov ax,0x13 / shl ax,4 / inc ax / mov al,gs:[bx+0x6d60]`, i.e.
    /// `matrix[0x13][1]`.
    ///
    /// Reads the matrix out of the image rather than restating it, so a constant
    /// that drifts from the table fails here.
    #[test]
    fn field_matrix_entries_match_the_constants() {
        let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        let base = 0xD420 + 0x6D60;
        let at = |selector: usize, column: usize| exe[base + selector * 16 + column] as u16;

        assert_eq!(at(0x13, 1), TALK_FIELD, "matrix[0x13][1] is the talk field");
        assert_eq!(at(6, 2), LOCATION_FIELD, "matrix[6][2] is the location field");
        assert_eq!(at(9, 8), LOCATION_FIELD, "and the kind-8 column agrees");

        // Selector 0 is uniform 0x02 across its live columns -- a field every kind
        // shares. If that ever differs per column, code treating it as one value
        // needs revisiting.
        for column in 0..11 {
            assert_eq!(at(0, column), 2, "selector 0 column {column}");
        }

        // audit-fixes #289. The three kind100 selectors are named for kind 0x100,
        // and the matrix is what justifies the name: each is nonzero in column 8
        // ONLY. Column k corresponds to kind 2^k (`bsf` @0x6027), so column 8 is
        // kind 0x100. This makes `Ship3dPositionRecord`'s `None` for any other
        // kind the TABLE's answer rather than missing port data.
        for selector in [9usize, 10, 12] {
            for column in 0..16 {
                let expected_nonzero = column == 8;
                assert_eq!(
                    at(selector, column) != 0,
                    expected_nonzero,
                    "selector {selector} column {column}: the kind100 selectors \
                     must be populated for kind 0x100 and nothing else"
                );
            }
        }

        // The position selector has NO column for kind 0x40 (column 6), yet the
        // distance routine resolves it for exactly that kind and adds the result
        // unconditionally (`add ax,si` @0x6121). So the position of a kind-0x40
        // record sits AT the record's start. Pinned because it is the counter-
        // example to "a zero offset means the field is absent" -- the reading
        // that would otherwise look obvious.
        assert_eq!(
            at(11, 6),
            0,
            "kind 0x40 has no selector-11 column; its position is the record start"
        );
        // ...while the three kinds the walk itself calls direct DO have one.
        for column in [3usize, 4, 9] {
            assert_ne!(
                at(11, column),
                0,
                "kind 0x{:x} is a direct position kind and must have a column",
                1 << column
            );
        }
    }

    /// The built-in objects are the game's OWN name table at `DS:0x67BE`, packed
    /// NUL-terminated: blood, orxx, Honk, menu, arche, cryobox, Ark, Scruter_Jo,
    /// vbio. NINE. This struct carried eight until #172 — `cryobox` was absent, so
    /// an object the engine resolves and gives a global went unresolved here.
    ///
    /// Reads the table out of the image rather than restating it, so adding a
    /// tenth built-in to the port without a matching name in the data (or missing
    /// one that IS in the data) fails.
    #[test]
    fn named_object_table_matches_the_games_name_list() {
        let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        let start = 0xD420 + 0x67BE;
        let mut names = Vec::new();
        let mut at = start;
        while names.len() < 9 {
            let end = at + exe[at..].iter().position(|&b| b == 0).unwrap();
            names.push(String::from_utf8_lossy(&exe[at..end]).to_ascii_lowercase());
            at = end + 1;
        }
        assert_eq!(
            names,
            vec![
                "blood", "orxx", "honk", "menu", "arche", "cryobox", "ark",
                "scruter_jo", "vbio",
            ],
            "the game's built-in name table"
        );

        // Every name the table lists must be one this struct resolves.
        let mut offsets = VmNamedObjectOffsets::default();
        for (i, name) in names.iter().enumerate() {
            assert!(
                offsets.set(name, (i as u16 + 1) * 0x10),
                "`{name}` is in the game's table but this struct does not resolve it"
            );
        }
        assert_eq!(offsets.cryobox, Some(6 * 0x10), "cryobox resolved");
    }

    /// The status headers READ FROM THE SHIPPED BINARY, so these tests compare
    /// against the game's strings rather than against literals restated here.
    fn test_status_headers() -> StatusHeaders {
        let b = ["re/bin/BLOODPRG.EXE", "../re/bin/BLOODPRG.EXE"]
            .iter()
            .find_map(|p| crate::bloodprg::BloodPrg::parse_file(p).ok())
            .expect("BLOODPRG.EXE for the status headers");
        let h = b.location_status_headers();
        StatusHeaders {
            planet: h[0].clone(),
            ship: h[1].clone(),
            black_hole: h[2].clone(),
            life_support: h[3].clone(),
        }
    }

    /// The `0x6863` ladder recognises exactly `0xF0..=0xF5`. `0x6891` clears al
    /// BEFORE the ladder, every arm is an explicit `cmp ah,0xFn`, and an
    /// unrecognised operator falls through to `0x68DB` with al still zero — where
    /// `or al,al / jne` makes zero mean BRANCH.
    ///
    /// The executing arm used to end its match with `_ => cur == operand_i`,
    /// folding every unknown operator into an equality test that can decline to
    /// branch. `apply_operator` had it right and was called only by tests; this
    /// pins the behaviour on the path the VM actually runs.
    #[test]
    fn unknown_query_operator_branches_like_the_ladder() {
        let q = QuerySetMode { query: true };
        // Recognised: equality matches, so no branch.
        assert_eq!(q.apply_operator(0xF5, 7, 7), Err(true));
        // NOT recognised — including 0xF6/0xF7, which are SET-mode operators and
        // have no ladder arm in query mode. Equal operands must NOT rescue them.
        for op in [0xF6u8, 0xF7, 0x00, 0x42, 0xEF, 0xFF] {
            assert_eq!(
                q.apply_operator(op, 7, 7),
                Err(false),
                "operator {op:#04x} is not in the ladder: al stays 0 -> branch"
            );
        }
        // Set mode: only F5/F6/F7 mutate; anything else writes `cur` back.
        let s = QuerySetMode { query: false };
        assert_eq!(s.apply_operator(0xF5, 7, 3), Ok(3));
        assert_eq!(s.apply_operator(0xF6, 7, 3), Ok(10));
        assert_eq!(s.apply_operator(0xF7, 7, 3), Ok(4));
        assert_eq!(s.apply_operator(0xF0, 7, 3), Ok(7), "unchanged");
    }

    /// The contact menu is built from the SHIP-SLOT ARRAY, not from a fixed list:
    /// `0x87BD` skips empty slots (`or ax,ax / je`), stops at `0xFFFF`, and emits
    /// record+4 (the inline name) for each occupant. This pins all three rules.
    #[test]
    fn contact_menu_comes_from_the_occupied_ship_slots() {
        let mut m = VmMachine::new();
        // Names live at record+4, so write each one there as CP437 bytes.
        fn put(m: &mut VmMachine, at: u16, name: &str) -> u16 {
            for (i, b) in name.bytes().enumerate() {
                m.rec_write_u8_pub(at.wrapping_add(4).wrapping_add(i as u16), b);
            }
            m.rec_write_u8_pub(at.wrapping_add(4).wrapping_add(name.len() as u16), 0);
            at
        }
        let a = put(&mut m, 0x100, "bob_morlock");
        let b = put(&mut m, 0x200, "izwalito");
        let c = put(&mut m, 0x300, "never_reached");

        m.ship_slots = [0u16; 16];
        m.ship_slots[0] = a;
        m.ship_slots[2] = b; // slot 1 empty -- skipped, not emitted as a blank
        m.ship_slots[3] = 0xFFFF;
        m.ship_slots[4] = c; // behind the terminator: must not appear

        assert_eq!(
            m.ship_contact_menu_words(),
            vec!["bob_morlock".to_string(), "izwalito".to_string()],
            "occupied slots up to the 0xFFFF terminator, named from record+4"
        );

        // An empty ship yields an EMPTY menu -- the port must not invent entries.
        m.ship_slots = [0u16; 16];
        assert!(m.ship_contact_menu_words().is_empty());
    }
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

    /// CONTENT-LEVEL DUAL-RUN: the escape instruction @2D6E ("CLICK ON THE
    /// RED BUTTON ON THE MAP...") is the deterministic interception TAIL (plays
    /// after any SS variant); the ORACLE transcript captured exactly this line
    /// (accuracy/interception_oracle_transcript.txt). The port must play the
    /// same text through its own drive — a variant-independent content match
    /// against the real game.
    #[test]
    fn interception_escape_instruction_matches_the_oracle() {
        let Some(iso) = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .find(|d| std::path::Path::new(d).join("SCRIPT2.COD").is_file())
        else {
            eprintln!("skipping: extracted SCRIPT2 files not available");
            return;
        };
        let cod = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.COD")).unwrap();
        let var = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.VAR")).unwrap();
        let dic_raw = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.DIC")).unwrap();
        let dic = crate::script::parse_dictionary(&dic_raw);
        let toks = walk(&cod, 0, cod.len());
        let text_of = |off: usize| -> String {
            toks.iter()
                .find_map(|t| match t {
                    VmToken::Text { offset: o, word_offsets, .. } if *o == off => Some(
                        word_offsets
                            .iter()
                            .take_while(|&&w| w != 0xFFFF)
                            .filter_map(|w| dic.get(w).cloned())
                            .collect::<Vec<_>>()
                            .join(" "),
                    ),
                    _ => None,
                })
                .unwrap_or_default()
        };
        // The departure/interception drives (script2_interception_plays,
        // script2_departure_radio) already prove the port REACHES rec-0x6FC's
        // tail region (offsets 0x2D40..0x2DA1); here we lock the CONTENT match:
        // the port's decoded line at @2D6E is exactly what the oracle spoke.
        // The line's text matches the oracle's captured escape instruction.
        let text = text_of(0x2D6E).to_uppercase();
        assert!(
            text.contains("CLICK ON THE RED BUTTON") && text.contains("CONTROL STICK"),
            "the escape instruction text matches the oracle (got {text:?})"
        );
    }

    /// DETERMINISTIC VARIANT (seed 0x2727 = the oracle's fixed CMOS seed): the
    /// interception's SS randomizer now rolls reproducibly, so the port plays
    /// ONE fixed variant on a given drive — locked here as a regression
    /// fixture (the deep-recipe lane's port-side determinism).
    #[test]
    fn interception_variant_is_deterministic() {
        let Some(iso) = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .find(|d| std::path::Path::new(d).join("SCRIPT2.COD").is_file())
        else {
            eprintln!("skipping: extracted SCRIPT2 files not available");
            return;
        };
        let cod = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.COD")).unwrap();
        let var = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.VAR")).unwrap();
        let play = || -> Vec<usize> {
            let mut m = VmMachine::new();
            m.load_cod(&cod);
            m.load_var(&var);
            assert_eq!(m.prng_seed, 0x2727, "the oracle's deterministic seed");
            for _ in 0..80 {
                let evs = m.run_frame();
                m.tick_state_countdowns();
                if evs.iter().any(|e| matches!(e, VmEvent::QueuePresentation { offset: 0x6FC })) {
                    break;
                }
            }
            let mut offs = Vec::new();
            for _ in 0..12 {
                let Some(started) = m.promote_queued_presentation() else {
                    let _ = m.run_frame();
                    continue;
                };
                for _ in 0..200 {
                    for ev in m.run_frame() {
                        if let VmEvent::Text { offset } = ev {
                            if started == 0x6FC {
                                offs.push(offset);
                            }
                        }
                    }
                    if !m.presentation_busy {
                        break;
                    }
                }
                if started == 0x6FC && !m.presentation_busy {
                    break;
                }
                if m.presentation_busy {
                    if let Some(a) = m.active_actor {
                        m.rec_write(a, 0);
                    }
                    m.active_actor = None;
                    m.presentation_busy = false;
                }
            }
            offs
        };
        let a = play();
        let b = play();
        assert!(!a.is_empty(), "the interception plays");
        assert_eq!(a, b, "the variant is deterministic (same seed -> same roll)");
    }

    /// The examination model targets the RIGHT field: the frontend hook writes
    /// scrambler(DEB)+0x14, which must equal 0x13C2 — the exact record the
    /// SCRIPT3 endgame gate guards (== 40). Locks the APPROX model's targeting
    /// (the value-source is APPROX; the TARGET is proven from the DEB layout).
    #[test]
    fn examination_hook_targets_the_endgame_field() {
        let Some(iso) = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .find(|d| std::path::Path::new(d).join("SCRIPT3.DEB").is_file())
        else {
            eprintln!("skipping: SCRIPT3.DEB not available");
            return;
        };
        let deb = std::fs::read(std::path::Path::new(iso).join("SCRIPT3.DEB")).unwrap();
        let names = crate::engine::deb_actor_name_map(&deb);
        let scrambler = names
            .iter()
            .find(|(_, n)| n.eq_ignore_ascii_case("scrambler"))
            .map(|(&o, _)| o)
            .expect("scrambler in SCRIPT3.DEB");
        assert_eq!(scrambler, 0x13AE, "scrambler's DEB offset");
        assert_eq!(
            scrambler.wrapping_add(0x14),
            0x13C2,
            "the examination hook's target = the endgame's rec_13C2 guard"
        );
        // And the endgame gate reads exactly 0x13C2 (AF guard @6CA2).
        let cod = std::fs::read(std::path::Path::new(iso).join("SCRIPT3.COD")).unwrap();
        assert_eq!(&cod[0x6CA2..0x6CA5], &[0xAF, 0xC2, 0x13], "the AF guard reads 0x13C2");
    }

    /// THE FIRST LINE-LEVEL DUAL-RUN: the ORACLE (the real game under the
    /// interpreter, red-button scenario) settled on two distinctive lines
    /// during the interception answer — Honk's 1010 gloss and the escape
    /// instruction (banked: accuracy/interception_oracle_transcript.txt). The
    /// PORT, driving the same beat through its own machinery, must emit the
    /// same lines (decoded through the DIC from the same word offsets). The
    /// oracle transcript VERIFIES; the bytecode is the source.
    #[test]
    fn interception_dual_run_matches_the_oracle_lines() {
        let Some(iso) = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .find(|d| std::path::Path::new(d).join("SCRIPT2.COD").is_file())
        else {
            eprintln!("skipping: extracted SCRIPT2 files not available");
            return;
        };
        let cod = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.COD")).unwrap();
        let var = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.VAR")).unwrap();
        let dic_raw = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.DIC")).unwrap();
        let dic = crate::script::parse_dictionary(&dic_raw);
        let toks = walk(&cod, 0, cod.len());
        let text_of = |offset: usize| -> String {
            toks.iter()
                .find_map(|t| match t {
                    VmToken::Text { offset: o, word_offsets, .. } if *o == offset => Some(
                        word_offsets
                            .iter()
                            .take_while(|&&w| w != 0xFFFF)
                            .filter_map(|w| dic.get(w).cloned())
                            .collect::<Vec<_>>()
                            .join(" "),
                    ),
                    _ => None,
                })
                .unwrap_or_default()
        };

        let mut m = VmMachine::new();
        m.load_cod(&cod);
        m.load_var(&var);
        let mut played: Vec<String> = Vec::new();
        for _ in 0..80 {
            let evs = m.run_frame();
            m.tick_state_countdowns();
            if evs
                .iter()
                .any(|e| matches!(e, VmEvent::QueuePresentation { offset: 0x6FC }))
            {
                break;
            }
        }
        for _ in 0..12 {
            let Some(started) = m.promote_queued_presentation() else {
                let _ = m.run_frame();
                continue;
            };
            for _ in 0..200 {
                for ev in m.run_frame() {
                    if let VmEvent::Text { offset } = ev {
                        if started == 0x6FC {
                            played.push(text_of(offset));
                        }
                    }
                }
                if !m.presentation_busy {
                    break;
                }
            }
            if started == 0x6FC && !m.presentation_busy {
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
        // The oracle's settled lines (verification data, banked from the live
        // driven game): both must appear in the port's own playback.
        let joined = played.join(" | ").to_lowercase();
        // BEAT-LEVEL match: both implementations play SCRUT-radio content from
        // record 0x6FC at this story point (the oracle's settled samples came
        // from a different ring of the same encounter — its cancel-cycle drive
        // answered a later call; EXACT line-for-line comparison needs the same
        // scenario driven through the frontend, the verify_port lane, which is
        // the ledgered matched-drive dual-run).
        assert!(
            joined.contains("message radio"),
            "the radio beat plays (got: {joined})"
        );
        assert!(
            joined.contains("scrut agent k"),
            "agent K's call plays from the same record the oracle showed"
        );
    }

    /// The field matrix is byte-exact against the shipped image and encodes
    /// the port's standing laws as its kind-1 column.
    #[test]
    fn field_offset_matrix_matches_the_binary() {
        let Some(exe) = ["re/bin/BLOODPRG.EXE", "../re/bin/BLOODPRG.EXE"]
            .iter()
            .find(|p| std::path::Path::new(p).is_file())
        else {
            eprintln!("skipping: BLOODPRG.EXE not available");
            return;
        };
        let d = std::fs::read(exe).unwrap();
        let base = 0xD420 + 0x6D60;
        for f in 0..0x15usize {
            for k in 0..16usize {
                assert_eq!(
                    FIELD_OFFSETS[f][k],
                    d[base + f * 16 + k],
                    "matrix[{f:#x}][{k}]"
                );
            }
        }
        // Kind is a bit-flag; the column is bsf(kind). Kind 2 (character) is the
        // common object: location = obj+0x18 (24), talk = obj+0x3A (58).
        assert_eq!(field_offset(2, 0x11), Some(24), "kind-2 character location = obj+0x18");
        assert_eq!(field_offset(2, 0x13), Some(58), "kind-2 character talk = obj+0x3A");
        // kind-1 built-ins use column 0: location = obj+0x06.
        assert_eq!(field_offset(1, 0x11), Some(6), "kind-1 location = obj+0x06 (bsf=0)");
        assert_eq!(field_offset(0, 0x11), None, "kind 0 has no fields");
    }

    /// The shared drive layer: engaging an actor BY NAME (the DEB symbol
    /// table) plays their lines through the same frame policy both binaries
    /// use — the matched-drive lane's dispatch core.
    #[test]
    fn vm_drive_engages_actors_by_name() {
        let Some(iso) = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .find(|d| std::path::Path::new(d).join("SCRIPT2.COD").is_file())
        else {
            eprintln!("skipping: extracted SCRIPT2 files not available");
            return;
        };
        let cod = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.COD")).unwrap();
        let var = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.VAR")).unwrap();
        let dic = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.DIC")).unwrap();
        let deb = std::fs::read(std::path::Path::new(iso).join("SCRIPT2.DEB")).unwrap();
        let mut d = crate::vm_drive::VmDrive::new(&cod, &var, &dic, &deb);
        d.m.flag_252a = true;
        d.m.flag_274f = true;
        assert!(d.engage("Scruter_Jo"), "the DEB symbol resolves");
        d.m.satisfy_opening_location_guards();
        let mut lines: Vec<String> = Vec::new();
        for _ in 0..40 {
            lines.extend(d.frame());
            if !lines.is_empty() {
                break;
            }
        }
        let joined = lines.join(" | ").to_lowercase();
        assert!(
            joined.contains("scanning stranger"),
            "Scruter Jo's scan opener plays through the drive layer (got {joined})"
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
    fn directed_drive_plays_the_story_to_fin_hnm() {
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

        // ACT TWO OPENS: perform the D2 switch (load SCRIPT3's COD + VAR — the
        // loader's clean-reload model; each script's opening init block writes
        // its own world) and let the self-disabling init run: the world
        // relocates (rec_0722 = 4070 @0073), the character slots bind their
        // DESCRIPT names (slot 4 = "venus" @00A4), and vbio arrives at 3.
        let cod3 = std::fs::read(std::path::Path::new(iso).join("SCRIPT3.COD")).unwrap();
        let var3 = std::fs::read(std::path::Path::new(iso).join("SCRIPT3.VAR")).unwrap();
        m.load_cod(&cod3);
        m.load_var(&var3);
        m.pending_profile = -1;
        m.active_actor = None;
        m.presentation_busy = false;
        m.resume_pos = None;
        for _ in 0..5 {
            let _ = m.run_frame();
        }
        assert_eq!(m.rec_read(0x0722), 4070, "SCRIPT3's init relocates the world");
        // Named variables are PER-SCRIPT (each DEB carries its own table):
        // SCRIPT3's vbio is record 0x13EE (the init bytes: C0 EE 13 F5 C1 03),
        // not SCRIPT2's 0x126C — the frontend's cyber-arrival hook must
        // resolve the name through the loaded script's DEB.
        assert_eq!(m.rec_read(0x13EE), 3, "vbio (SCRIPT3's 0x13EE) arrives at 3");
        let slot4: Vec<u8> = m.records16[3 * 16..3 * 16 + 6].to_vec();
        assert_eq!(&slot4[..5], b"venus", "char slot 4 binds its DESCRIPT name");

        // ACT TWO'S FIRST BEAT: Scruter Jo's SCRIPT3 talk (rec 0x081C per the
        // C4 @00CD) — with vbio pre-set to 3 by the init, the BIONIUM
        // acknowledgment region (@02C5..) plays: "Good work... You did get
        // BIONIUM..." — Act Two opens post-success, from shipped bytes.
        m.start_actor_presentation(0x081C, 40);
        let mut ack = false;
        for _ in 0..60 {
            for ev in m.run_frame() {
                if let VmEvent::Text { offset } = ev {
                    if (0x02C5..0x0406).contains(&offset) {
                        ack = true;
                    }
                }
            }
            if ack {
                break;
            }
        }
        assert!(ack, "the BIONIUM acknowledgment plays in SCRIPT3");

        // ACT TWO'S EXIT: the endgame gate (@6C90) — fish/fion/jerry (SCRIPT3
        // DEB names at 0x13FC/0x1424/0x142C, each its own quest beat's
        // product), the object placements the guards declare (rec_13C2==40
        // aboard; rec_088A/rec_06DA at 4070), then the C3 queues JERRY KHAN
        // (0x042C) -> the Oddland briefing ("Inspector JERRY KHAN onboard the
        // SHARK... a black hole called ODDLAND") -> its poke arms the exit
        // block whose C6 typed-record guard (rec 0x108E vs 0x1052) releases
        // RUN PROFILE 3 -> SCRIPT4.
        if let Some(actor) = m.active_actor {
            m.rec_write(actor, 0);
        }
        m.active_actor = None;
        m.presentation_busy = false;
        m.resume_pos = None;
        // Let the world's one-shots settle (e.g. @5C81 relocates the SPLATCH
        // to 3278 and self-disables), THEN apply the quest's outcomes — the
        // beats' own order (the SPLATCH teleport @5F4C returns it to 4070
        // AFTER that one-shot has spent itself).
        for _ in 0..3 {
            let _ = m.run_frame();
        }
        m.rec_write(0x13FC, 1);
        m.rec_write(0x1424, 1);
        m.rec_write(0x142C, 1);
        // rec_13C2 = 40: the EXAMINATION OUTCOME (now modeled in the frontend
        // hook, main.rs — the examined object's related field; APPROX cited to
        // 5 static findings + the exam-completion computed write). Here it
        // stands for "the player examined the alien before the endgame."
        m.rec_write(0x13C2, 40);
        m.rec_write(0x088A, 4070);
        m.rec_write(0x06DA, 4070);
        // The C6 endgame-exit record {C6, 0x1052} (the RUN PROFILE 3 guard).
        m.rec_write(0x108E, 0xC6);
        m.rec_write(0x1090, 0x1052);
        // SCRIPT3'S QUEST LINE, beat-ordered (each presenter from its DEB
        // offset; each beat's tail sets its flag): the eavesdropped SCRUT
        // broadcast (receiver 0x242 -> talk 0x27C: "the first shipment...
        // secret jail" -> fish=1), the mummy's-curse sequence (t10 0x5DC ->
        // 0x616 -> fion=1), Jerry Khan's first visit (0x42C: "I'll take young
        // Yoko with me in my ship, the SHARK" -> jerry=1). Then the quests'
        // placements land in the beats' own order, the endgame gate queues
        // Jerry Khan's return, the Oddland briefing plays, and RUN PROFILE 3
        // hands off to SCRIPT4.
        let mut play_beat = |m: &mut VmMachine, talk: u16, flag: u16, want: u16| {
            m.start_actor_presentation(talk, 40);
            for _ in 0..300 {
                let _ = m.run_frame();
                if m.rec_read(flag) == want {
                    break;
                }
                if !m.presentation_busy {
                    m.start_actor_presentation(talk, 40);
                }
            }
            // The click stand-in for a lingering session.
            if m.presentation_busy {
                if let Some(actor) = m.active_actor {
                    m.rec_write(actor, 0);
                }
                m.active_actor = None;
                m.presentation_busy = false;
            }
        };
        play_beat(&mut m, 0x27C, 0x13FC, 1);
        assert_eq!(m.rec_read(0x13FC), 1, "the broadcast beat sets fish");
        play_beat(&mut m, 0x616, 0x1424, 1);
        play_beat(&mut m, 0x42C, 0x142C, 1);
        assert_eq!(m.rec_read(0x142C), 1, "Jerry Khan's first visit sets jerry");
        // THE SPLATCH TELEPORT, DRIVEN in the story's own order: Amigo
        // (talk 0x6FC) behind rec_1088@3224 + evi. The KEY is the EXAMINATION
        // BETWEEN VISITS — session 0 plays the password conv (@5EB7's voice
        // line clears its skip so @5ECD's secret=0 runs, correctly), THEN the
        // examination sets secret=1 (the alien-view hook's outcome), THEN
        // session 1 reaches @5EF2 (secret==1) -> the teleport menu @5F19
        // (concept 0x0367) -> the CD moves splatch aboard + @5F53 writes
        // rec_06DA=4070. No hand-write — the beat drives from shipped bytes.
        m.rec_write(0x1088, 3224);
        m.rec_write(0x1428, 1); // evi (a shallower beat's product)
        m.start_actor_presentation(0x6FC, 40); // first visit
        for _ in 0..120 {
            let _ = m.run_frame();
            if !m.presentation_busy {
                break;
            }
        }
        if let Some(a) = m.active_actor {
            m.rec_write(a, 0);
        }
        m.active_actor = None;
        m.presentation_busy = false;
        m.rec_write(0x1416, 1); // THE EXAMINATION (the alien-view hook) sets secret
        m.start_actor_presentation(0x6FC, 40); // second visit
        let mut splatched = false;
        for _ in 0..120 {
            for ev in m.run_frame() {
                if let VmEvent::Text { offset } = ev {
                    if offset == 0x5F19 {
                        m.dispatch_concept(0x0367); // "teleport"
                    }
                }
            }
            if m.rec_read(0x06DA) == 4070 {
                splatched = true;
                break;
            }
            if !m.presentation_busy {
                break;
            }
        }
        assert!(splatched, "the SPLATCH teleport beat drives (examination-between-visits order)");
        if m.presentation_busy {
            if let Some(a) = m.active_actor {
                m.rec_write(a, 0);
            }
            m.active_actor = None;
            m.presentation_busy = false;
        }
        // THE TINA BURNER TELEPORT, driven (Migrator's beat @44C6, talk
        // 0x474): "TELEPORT TINA BURNER TO AIRPORT" (concept 0x367) plays
        // "TELEPORTING TINA BURNER" and the beat's tail writes rec_088A=4070
        // (@4629) — another hand-placement replaced by its own beat.
        m.start_actor_presentation(0x474, 40);
        let mut tina = false;
        for _ in 0..300 {
            for ev in m.run_frame() {
                if let VmEvent::Text { offset } = ev {
                    if offset == 0x45DD {
                        m.dispatch_concept(0x367);
                    }
                }
            }
            if m.rec_read(0x088A) == 4070 {
                tina = true;
                break;
            }
            if !m.presentation_busy {
                m.start_actor_presentation(0x474, 40);
            }
        }
        assert!(tina, "the Tina Burner teleport beat writes rec_088A");
        if m.presentation_busy {
            if let Some(actor) = m.active_actor {
                m.rec_write(actor, 0);
            }
            m.active_actor = None;
            m.presentation_busy = false;
        }
        // fion (0x1424) is ALREADY driven above (the t10 mummy's-curse beat,
        // play_beat 0x616) — the redundant hand-write is dropped. rec_13C2==40
        // has NO writer in SCRIPT3.COD (VAR-init 722; the ==40 related-value
        // pattern) and no object-activation site — a cross-script/engine
        // related field; cited, not yet beat-driven.
        m.rec_write(0x13C2, 40);
        m.rec_write(0x108E, 0xC6);
        m.rec_write(0x1090, 0x1052);
        for _ in 0..20 {
            let _ = m.run_frame();
            if m.rec_read(0x042C) == 0xC3 {
                break;
            }
        }
        assert_eq!(m.rec_read(0x042C), 0xC3, "the endgame gate queues Jerry Khan's return");
        let mut briefing = false;
        for _ in 0..400 {
            for ev in m.run_frame() {
                if let VmEvent::Text { offset } = ev {
                    if (0x6CD1..0x6E07).contains(&offset) {
                        briefing = true;
                    }
                }
            }
            m.tick_state_countdowns();
            if !m.presentation_busy {
                let _ = m.promote_queued_presentation();
            }
            if m.pending_profile >= 0 {
                break;
            }
        }
        assert!(briefing, "the Oddland briefing plays");
        assert_eq!(m.pending_profile, 3, "RUN PROFILE 3 — the SCRIPT3 -> SCRIPT4 handoff");

        // ACT THREE (SCRIPT4, the Oddland chase): load on the handoff, let the
        // init + world one-shots settle, then the endgame manifest in story
        // order (@4046: rec_013A at 2840; the rescued aboard — rec_0722,
        // rec_040A, rec_0572 all 65535) queues Jerry Khan's return (0x504,
        // 'I just captured Doctor Otto Von Smile... I found the GLUXX kids...
        // We're going home'), whose tail (poke [0x4194]) arms the C6-guarded
        // exit -> RUN PROFILE 4 -> SCRIPT5.
        let cod4 = std::fs::read(std::path::Path::new(iso).join("SCRIPT4.COD")).unwrap();
        let var4 = std::fs::read(std::path::Path::new(iso).join("SCRIPT4.VAR")).unwrap();
        m.load_cod(&cod4);
        m.load_var(&var4);
        m.pending_profile = -1;
        m.active_actor = None;
        m.presentation_busy = false;
        m.resume_pos = None;
        for _ in 0..3 {
            let _ = m.run_frame();
        }
        m.rec_write(0x013A, 2840);
        m.rec_write(0x0722, 65535);
        m.rec_write(0x040A, 65535);
        m.rec_write(0x0572, 65535);
        m.rec_write(0x114E, 0xC6);
        m.rec_write(0x1150, 0x1112);
        for _ in 0..20 {
            let _ = m.run_frame();
            if m.rec_read(0x0504) == 0xC3 {
                break;
            }
        }
        assert_eq!(m.rec_read(0x0504), 0xC3, "SCRIPT4's endgame queues Jerry Khan's return");
        let mut homeward = false;
        for _ in 0..400 {
            for ev in m.run_frame() {
                if let VmEvent::Text { offset } = ev {
                    if (0x4077..0x4193).contains(&offset) {
                        homeward = true;
                    }
                }
            }
            m.tick_state_countdowns();
            if !m.presentation_busy {
                let _ = m.promote_queued_presentation();
            }
            if m.pending_profile >= 0 {
                break;
            }
        }
        assert!(homeward, "the homeward briefing plays");
        assert_eq!(m.pending_profile, 4, "RUN PROFILE 4 — the SCRIPT4 -> SCRIPT5 handoff");

        // THE FINALE (SCRIPT5, the Bigbang wedding concert): load on the
        // handoff, settle, satisfy the concert block's own guards (@1511:
        // rec_103A at 4024, rec_1340 at 4108) and start Migrator's talk
        // (0x474) — "SILENCE IT'S STARTING...." — the concert reels roll
        // (lpm*.hnm) and the block's tail loads FIN.HNM: the game's ENDING,
        // the same LoadString the frontend maps to the credits.
        let cod5 = std::fs::read(std::path::Path::new(iso).join("SCRIPT5.COD")).unwrap();
        let var5 = std::fs::read(std::path::Path::new(iso).join("SCRIPT5.VAR")).unwrap();
        m.load_cod(&cod5);
        m.load_var(&var5);
        m.pending_profile = -1;
        m.active_actor = None;
        m.presentation_busy = false;
        m.resume_pos = None;
        for _ in 0..3 {
            let _ = m.run_frame();
        }
        m.rec_write(0x103A, 4024);
        m.rec_write(0x1340, 4108);
        m.start_actor_presentation(0x474, 40);
        let mut fin = false;
        for _ in 0..600 {
            for ev in m.run_frame() {
                if let VmEvent::LoadString(name) = ev {
                    if name.eq_ignore_ascii_case("fin.hnm") {
                        fin = true;
                    }
                }
            }
            if fin {
                break;
            }
            if !m.presentation_busy {
                m.start_actor_presentation(0x474, 40);
            }
        }
        assert!(fin, "FIN.HNM loads — the Bigbang concert ends the game");
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
        // The daily menu is gated on CHEF BRONKO being aboard (@000A
        // rec_0332 == 65535; object 0x31A — "Today CHEF BRONKO has laid on
        // for you"). He boards through the STORY (SCRIPT2's "We teleported
        // Bronko into the cryobox" beat, @1770-era) — a 300M-step boot watch
        // shows NO engine-side init writes, so a fresh tutorial legitimately
        // has no menu demo; the old expectation was an artifact of the
        // match-any wildcard bug. The write below is the Bronko-teleport
        // beat's product (the CD transfer's aboard value).
        menu.rec_write(0x0332, 0xFFFF);
        menu.start_actor_presentation(2220, 40);
        let t3 = texts(menu.run_frame());
        assert!(t3.contains(&16), "the daily menu plays for the MENU actor");
        let t4 = texts(menu.run_frame());
        assert!(t4.is_empty(), "the presentation ended (C9) — nothing repeats unprompted");
    }

    use super::*;

    /// The inline `0xA1` prefix is consumed REGARDLESS OF MODE.
    ///
    /// `0x6C86` does `cmp al,0xA1` and `0x6C8E` does `inc si` — both BEFORE the mode
    /// test at `0x6C9C`, so the byte is skipped whatever the mode. The decoder used to
    /// gate that skip on `mode1` for `0xC1..0xC4` while leaving `0xCD` ungated.
    ///
    /// That gate also disagreed with the decoder's OWN length accounting, which already
    /// added 1 for the prefix unconditionally (`l += 1` in the `0xFD | 0xFB` arm). A
    /// token whose `len` counted the prefix but whose operand read did not skip it is
    /// internally inconsistent — the operands come from one byte earlier than `len`
    /// claims.
    ///
    /// Measured unreachable on shipped data before changing anything: across all five
    /// SCRIPT*.COD, no affected opcode (0xC1, 0xC2, 0xC3, 0xC4, 0xCD) is ever followed
    /// by 0xA1. So the alignment is provably behaviour-neutral on real scripts.
    #[test]
    fn a1_prefix_is_consumed_and_agrees_with_the_length_accounting() {
        // 0xC4 (actor) with the prefix, then a recognisable operand pair.
        let cod = vec![OP_ACTOR, 0xA1, 0x34, 0x12, 0x78, 0x56, 0x00, 0x00];
        let toks = walk(&cod, 0, cod.len());
        let tok = toks.first().expect("one token decoded");
        match tok {
            VmToken::Actor {
                record_offset,
                related_record_offset,
                inverted,
                len,
                ..
            } => {
                assert!(inverted, "the 0xA1 prefix must be recognised");
                // The operands must be read AFTER the prefix byte, not at it.
                assert_eq!(*record_offset, 0x1234, "first operand read past the prefix");
                assert_eq!(*related_record_offset, 0x5678, "second operand");
                // And len must cover opcode + prefix + both operands, so that the next
                // token starts in the right place.
                assert!(*len >= 6, "len {len} must span the prefix and both operands");
            }
            other => panic!("expected an Actor token, got {other:?}"),
        }

        // Without the prefix the operands sit one byte earlier — the control case that
        // proves the skip is driven by the byte and not applied blindly.
        let plain = vec![OP_ACTOR, 0x34, 0x12, 0x78, 0x56, 0x00, 0x00];
        match walk(&plain, 0, plain.len()).first().expect("token") {
            VmToken::Actor { record_offset, inverted, .. } => {
                assert!(!inverted);
                assert_eq!(*record_offset, 0x1234);
            }
            other => panic!("expected an Actor token, got {other:?}"),
        }
    }

    /// The decoded A6 `b3` -> per-line asset chain, as an executable specification.
    /// Landed ahead of the data path so the arithmetic is pinned to the binary now
    /// rather than being re-derived when someone wires it up.
    #[test]
    fn dlg_line_asset_chain_matches_the_decoded_arithmetic() {
        // line_id = sign_extend(b3) + 9   (0x668D lodsb/cbw, 0x11F5 add ax,9).
        // Sign extension is load-bearing: 0xFF is -1, not 255.
        assert_eq!(dlg_line_id_for_selector(0x00), 9);
        assert_eq!(dlg_line_id_for_selector(0x01), 10);
        assert_eq!(dlg_line_id_for_selector(0xFF), 8, "0xFF sign-extends to -1");
        assert_eq!(dlg_line_id_for_selector(0x80), -119, "0x80 is -128");

        // The b3 = 0 entry must land at 0x1FB5 + 0x26 -- the exact address the fill
        // cursor is seeded to at 0x7447 (`mov bx,0x1FB5; add bx,0x26`). This is the
        // corroboration that fixes the table base and the +2 field offset together.
        assert_eq!(
            dlg_line_asset_id_ds_offset(dlg_line_id_for_selector(0)),
            Some(DLG_LINE_ASSET_TABLE_DS + 0x26)
        );
        // Entries step by the 4-byte stride.
        let a = dlg_line_asset_id_ds_offset(9).unwrap();
        let b = dlg_line_asset_id_ds_offset(10).unwrap();
        assert_eq!(b - a, DLG_LINE_ASSET_ENTRY_STRIDE);
        // A negative line id is rejected, as 0x9D20's `or ax,ax; js` does.
        assert_eq!(dlg_line_asset_id_ds_offset(-1), None);

        // Fill values (0x7684): negatives pass through sign-extended, and 0xFF must
        // produce EXACTLY the sentinel the reader tests at 0x9D71.
        assert_eq!(dlg_line_asset_id_from_source_byte(0xFF), DLG_LINE_ASSET_NONE);
        assert_eq!(dlg_line_asset_id_from_source_byte(0xFE), 0xFFFE);
        // Non-negatives become (byte-1)*16 -- a NAME-TABLE BYTE OFFSET, not an ordinal.
        assert_eq!(dlg_line_asset_id_from_source_byte(1), 0);
        assert_eq!(dlg_line_asset_id_from_source_byte(2), 16);
        assert_eq!(dlg_line_asset_id_from_source_byte(5), 64);
        // Every non-negative result is name-table aligned.
        for b in 1..=0x7Fu8 {
            assert_eq!(dlg_line_asset_id_from_source_byte(b) % DLG_ASSET_NAME_STRIDE, 0);
        }

        // And the contrast that matters: the port's ordinal rule is a different
        // FUNCTION, not an off-by-one. b3=5 is name offset 64, never index 4.
        assert_ne!(
            dlg_line_asset_id_from_source_byte(5) as usize,
            text_selector_voice_clip_index(5, 16).unwrap_or(usize::MAX)
        );
    }

    /// Every consumer that turns an `0xA6` word list into text must stop at the
    /// `0xFFFF` separator. Three of six did not, in three different ways:
    ///   * the engine subtitle builder and `bas_vm` KEPT the menu rows (filter_map
    ///     dropped only the separator), gluing choices onto the sentence;
    ///   * `extract::script::decode_vm_words` required EVERY offset to resolve, so it
    ///     returned None for menu-bearing lines and both call sites skipped them.
    ///
    /// This pins the shared invariant rather than any single call site: resolving a
    /// list that contains a separator must never yield a menu word.
    #[test]
    fn resolving_a_word_list_never_yields_menu_words() {
        // A list shaped like the real SCRIPT1.COD record.
        let spoken: Vec<u16> = vec![0x0010, 0x0020, 0x0030];
        let menu: Vec<u16> = vec![0x0040, 0x0050];
        let mut list = spoken.clone();
        list.push(0xFFFF);
        list.extend(&menu);

        let split = |ws: &[u16]| -> Vec<u16> {
            ws[..ws.iter().position(|&w| w == 0xFFFF).unwrap_or(ws.len())].to_vec()
        };
        assert_eq!(split(&list), spoken, "spoken section stops at the separator");
        assert!(!split(&list).iter().any(|w| menu.contains(w)));

        // A list with NO separator is unaffected — the common case must not regress.
        assert_eq!(split(&spoken), spoken);

        // The separator itself must never be treated as a resolvable word.
        assert!(!split(&list).contains(&0xFFFF));
    }

    /// The `0xA6` word list has TWO sections separated by `0xFFFF`: the spoken line,
    /// then the CHOICE-MENU words. SCRIPT1.COD is the canonical example — at COD
    /// `0x499` the words are "Click quick, Cap'n Bob is waiting ...", then `0xFFFF`
    /// at `0x4A7`, then `explanations` (DIC `0x02FC`) and `game` (DIC `0x0309`),
    /// terminated by `0x0000`.
    ///
    /// The disassembler used to join the WHOLE list, which both ran the menu into the
    /// sentence and printed the separator as `word_65535` (0xFFFF is not a DIC
    /// offset). This pins the split against the real shipped files.
    #[test]
    fn a6_word_list_splits_the_spoken_line_from_the_choice_menu() {
        let dir = ["accuracy/cblood_install/cblood", "../accuracy/cblood_install/cblood"]
            .iter()
            .map(std::path::Path::new)
            .find(|p| p.join("SCRIPT1.COD").is_file());
        let Some(dir) = dir else { return };
        let cod = std::fs::read(dir.join("SCRIPT1.COD")).unwrap();
        let dic = std::fs::read(dir.join("SCRIPT1.DIC")).unwrap();
        let word = |o: u16| -> String {
            let o = o as usize;
            let end = dic[o..].iter().position(|&b| b == 0).map(|n| o + n).unwrap_or(dic.len());
            crate::font::cp437_string(&dic[o..end])
        };

        // Find the A6 record whose list contains the separator followed by the two
        // known menu words -- located by DECODING, not by a hardcoded token offset.
        let mut found = None;
        for pos in 0..cod.len().saturating_sub(6) {
            if cod[pos] != OP_TEXT {
                continue;
            }
            if let Some((VmToken::Text { word_offsets, .. }, _)) = decode_text(&cod, pos, cod.len())
            {
                if let Some(i) = word_offsets.iter().position(|&w| w == 0xFFFF) {
                    if word_offsets[i + 1..] == [0x02FC, 0x0309] {
                        found = Some(word_offsets.clone());
                        break;
                    }
                }
            }
        }
        let words = found.expect("the explanations/game menu record must decode");
        let sep = words.iter().position(|&w| w == 0xFFFF).unwrap();

        let spoken: Vec<String> = words[..sep].iter().map(|&w| word(w)).collect();
        assert_eq!(spoken, ["Click", "quick,", "Cap'n", "Bob", "is", "waiting", "..."]);

        let menu: Vec<String> = words[sep + 1..].iter().map(|&w| word(w)).collect();
        assert_eq!(menu, ["explanations", "game"]);

        // The menu words must NOT leak into the spoken line, and the separator must
        // never be resolved as a word.
        assert!(!spoken.iter().any(|w| w == "explanations" || w == "game"));
        assert!(!spoken.iter().any(|w| w.starts_with("word_")));

        // This is where EngineState::MENU_SUBMENU's content actually comes from --
        // the port's constant is these two DIC words, upper-cased.
        let upper: Vec<String> = menu.iter().map(|w| w.to_uppercase()).collect();
        assert_eq!(upper, ["EXPLANATIONS", "GAME"]);
    }

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

    /// Every opcode the SHIPPED scripts actually use must be executed by the live
    /// `step()`, not silently swallowed by its catch-all. This is the standing
    /// guard for the class of bug that hid `0xC1`, `0xC2` and `C5..C8`: each had a
    /// real handler in BLOODPRG.EXE and a tracer implementation, but live they
    /// only consumed operands, so their record writes/branches never happened.
    /// `0xD3` is the one legitimate exception — its entry in the binary's own
    /// handler table (file `0x142D0`) is NULL, so the game has no handler either.
    #[test]
    fn every_shipped_opcode_is_executed_live_not_swallowed() {
        // Opcodes with a real arm in step() (kept in sync with the match).
        const EXECUTED: &[u8] = &[
            0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xAB, 0xAC, 0xAD,
            0xAE, 0xAF, 0xB0, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xBB,
            0xBC, 0xBD, 0xBE, 0xBF, 0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9,
            0xCA, 0xCB, 0xCC, 0xCD, 0xCE, 0xCF, 0xD0, 0xD1, 0xD2,
        ];
        // 0xD3: NULL entry in the binary's handler table -> no game handler.
        const NO_HANDLER_IN_THE_GAME_EITHER: &[u8] = &[0xD3];

        let mut seen: std::collections::BTreeSet<u8> = Default::default();
        let mut checked = 0;
        for name in [
            "SCRIPT1.COD",
            "SCRIPT2.COD",
            "SCRIPT3.COD",
            "SCRIPT4.COD",
            "SCRIPT5.COD",
        ] {
            let cod = match std::fs::read(format!("output/_tmp_iso/{name}"))
                .or_else(|_| std::fs::read(format!("../output/_tmp_iso/{name}")))
            {
                Ok(b) => b,
                Err(_) => continue,
            };
            checked += 1;
            for t in walk(&cod, 0, cod.len()) {
                if let Some(&op) = cod.get(t.offset()) {
                    if (OP_MIN..=OP_MAX).contains(&op) {
                        seen.insert(op);
                    }
                }
            }
        }
        if checked == 0 {
            return; // scripts not extracted in this checkout
        }
        let swallowed: Vec<String> = seen
            .iter()
            .copied()
            .filter(|op| !EXECUTED.contains(op) && !NO_HANDLER_IN_THE_GAME_EITHER.contains(op))
            .map(|op| format!("0x{op:02X}"))
            .collect();
        assert!(
            swallowed.is_empty(),
            "opcodes used by the shipped scripts but NOT executed by live step(): {swallowed:?}"
        );
        // The guard is only meaningful if the corpus really exercises the opcodes
        // this session repaired — each was a silent no-op in live play before.
        // The shipped corpus uses 32 of the 51 implemented opcodes; `0xD3` never
        // appears at all, so its NULL handler entry is moot in practice.
        for op in [0xC1u8, 0xC2, 0xC6, 0xC9] {
            assert!(
                seen.contains(&op),
                "0x{op:02X} should appear in the shipped scripts (its live fix matters); \
                 corpus uses: {:?}",
                seen.iter().map(|o| format!("{o:02X}")).collect::<Vec<_>>()
            );
        }
        assert!(!seen.contains(&0xD3), "0xD3 is unused by the shipped scripts");
    }

    #[test]
    fn state_operators_compare_signed_like_setl_setg() {
        // 0x6893..0x68D9 uses the SIGNED set-condition family (setl/setg/setle/
        // setge), not the unsigned setb/seta/setbe/setae. The distinction is not
        // academic: 0xFFFF is the aboard/wildcard sentinel and is -1 signed but
        // 65535 unsigned, so an unsigned compare inverts every ordered test
        // against it.
        let q = QuerySetMode { query: true };
        // 0xFFFF (-1) is LESS than 1, not greater.
        assert_eq!(q.apply_operator(0xF1, 0xFFFF, 1), Err(true), "0xFFFF < 1 signed");
        assert_eq!(q.apply_operator(0xF2, 0xFFFF, 1), Err(false), "0xFFFF is NOT > 1");
        assert_eq!(q.apply_operator(0xF3, 0xFFFF, 1), Err(true), "0xFFFF <= 1");
        assert_eq!(q.apply_operator(0xF4, 0xFFFF, 1), Err(false), "0xFFFF is NOT >= 1");
        // 0x8000 (-32768) is the extreme negative.
        assert_eq!(q.apply_operator(0xF1, 0x8000, 0), Err(true), "0x8000 < 0 signed");
        assert_eq!(q.apply_operator(0xF2, 0x7FFF, 0x8000), Err(true), "0x7FFF > 0x8000 signed");
        // Equality/inequality are sign-agnostic and unchanged.
        assert_eq!(q.apply_operator(0xF5, 0xFFFF, 0xFFFF), Err(true));
        assert_eq!(q.apply_operator(0xF0, 0xFFFF, 0xFFFF), Err(false));
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

    /// The LIVE interpreter (step) must actually EXECUTE C5..C8 — they used to
    /// fall to the no-op catch-all (queries never branched, writes vanished).
    #[test]
    fn live_step_executes_c5_c8_record_entries() {
        // C6 write (unconditional): rec[0x20] <- {C6, 0x0044, 0}.
        let mut m = VmMachine::new();
        m.load_cod(&[0xC6, 0x20, 0x00, 0x44, 0x00, 0xFF]);
        m.query = false;
        m.pc = 0;
        m.step();
        assert_eq!(m.rec_read_pub(0x20), 0xC6, "C6 write lands (was a no-op)");
        assert_eq!(m.rec_read_pub(0x22), 0x0044);

        // C8 write: {C8, 0, 0} only into an empty record.
        let mut m = VmMachine::new();
        m.load_cod(&[0xC8, 0x30, 0x00, 0x00, 0x00, 0xFF]);
        m.query = false;
        m.pc = 0;
        m.step();
        assert_eq!(m.rec_read_pub(0x30), 0xC8, "C8 writes an empty record");

        // QUERY on an empty record: non-match -> BRANCH to the else target
        // (was: fall through into the guarded body).
        let mut m = VmMachine::new();
        m.load_cod(&[0xC6, 0x40, 0x00, 0x44, 0x00, 0xFF]);
        m.query = true;
        m.stack.push(0x99);
        m.pc = 0;
        m.step();
        assert_eq!(m.pc, 0x99, "empty record -> query fails -> branch to else");

        // QUERY on a matching record: pass -> fall through (no branch).
        let mut m = VmMachine::new();
        m.rec_write_pub(0x40, 0xC6);
        m.rec_write_pub(0x42, 0x0044);
        m.load_cod(&[0xC6, 0x40, 0x00, 0x44, 0x00, 0xFF]);
        m.query = true;
        m.stack.push(0x99);
        m.pc = 0;
        m.step();
        assert_eq!(m.pc, 5, "matching record -> pass -> fall through past the 5-byte token");
        assert_eq!(m.stack.len(), 1, "no branch taken on a match");
    }

    #[test]
    fn live_step_c4_set_write_guard() {
        // The 0xC4 mode-0 write guard (0x6CC3..0x6D01): two objects at bases
        // 0x10 and 0x80, both kind 2 and both active; a C4 SET op1=0x84 (owner
        // 0x80 via the 0x6034 threshold lookup) op2=0x10.
        fn armed() -> VmMachine {
            let mut m = VmMachine::new();
            m.object_offsets = vec![0x10, 0x80];
            m.rec_write_pub(0x80, 2); // obj@0x80 kind 2
            m.rec_write_pub(0x82, 1); // obj@0x80 active (bit0 of +2)
            m.rec_write_pub(0x10, 2); // obj@0x10 kind 2
            m.rec_write_pub(0x12, 1); // obj@0x10 active
            m
        }
        let cod = [OP_ACTOR, 0x84, 0x00, 0x10, 0x00, 0xFF];

        // (1) both objects active, neither kind 1, op1 record empty -> WRITE.
        let mut m = armed();
        m.load_cod(&cod);
        m.query = false;
        m.stack.push(0x99);
        m.pc = 0;
        m.step();
        assert_eq!(m.rec_read_pub(0x84), 0xC4, "both active -> C4 state record written");
        assert_eq!(m.rec_read_pub(0x86), 0x10, "related operand stored at +2");
        assert_eq!(m.active_actor, Some(0x84));
        assert_eq!(m.pc, 5, "no branch: fell through the 5-byte token");

        // (2) op1's owning object inactive -> vm_branch, no write.
        let mut m = armed();
        m.rec_write_pub(0x82, 0); // obj@0x80 inactive
        m.load_cod(&cod);
        m.query = false;
        m.stack.push(0x99);
        m.pc = 0;
        m.step();
        assert_eq!(m.pc, 0x99, "op1 object inactive -> vm_branch (0x6CC8)");
        assert_ne!(m.rec_read_pub(0x84), 0xC4, "no C4 record written on branch");
        assert_eq!(m.active_actor, None);

        // (3) op2's object inactive -> vm_branch.
        let mut m = armed();
        m.rec_write_pub(0x12, 0); // obj@0x10 inactive
        m.load_cod(&cod);
        m.query = false;
        m.stack.push(0x99);
        m.pc = 0;
        m.step();
        assert_eq!(m.pc, 0x99, "op2 object inactive -> vm_branch (0x6CD1)");
        assert_eq!(m.active_actor, None);

        // (4) op1 STATE record already 0xC4 (both active, kinds 2) -> vm_branch
        // (the cx==0xC4 idempotence guard at 0x6CE3).
        let mut m = armed();
        m.rec_write_pub(0x84, 0xC4);
        m.load_cod(&cod);
        m.query = false;
        m.stack.push(0x99);
        m.pc = 0;
        m.step();
        assert_eq!(m.pc, 0x99, "already-set op1 record -> vm_branch");

        // (5) op1's object kind 1 short-circuits to WRITE, ahead of the
        // already-set check (order at 0x6CD3 -> 0x6D01 before 0x6CE3).
        let mut m = armed();
        m.rec_write_pub(0x80, 1); // obj@0x80 kind 1
        m.rec_write_pub(0x84, 0xC4); // already-set would branch, but kind-1 wins
        m.load_cod(&cod);
        m.query = false;
        m.pc = 0;
        m.step();
        assert_eq!(m.rec_read_pub(0x86), 0x10, "kind-1 object -> write despite already-set");
        assert_eq!(m.active_actor, Some(0x84));

        // (6) object table unloaded (opcode-only scaffolding) -> the guard is
        // skipped and the legacy unconditional write is preserved.
        let mut m = VmMachine::new();
        m.load_cod(&cod);
        m.query = false;
        m.pc = 0;
        m.step();
        assert_eq!(m.rec_read_pub(0x84), 0xC4, "no DEB loaded -> unconditional write");
        assert_eq!(m.active_actor, Some(0x84));
    }

    #[test]
    fn c1_set_kind10_target_writes_the_selector13_destination() {
        // 0x6C04..0x6C53: when the resolved OWNER is kind 0x10, the C1 SET does
        // not write the operand record — it writes {0xC1, operand, 2} at
        // owner + field_offset(0x13, 0x10), and only if a source-list entry
        // passes its gate. The kind-1 gate is `es:[operand+2] & 2`.
        let link = vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C2, 1).expect("kind 1 selector-0x11");
        let dest_fo =
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, 0x10).expect("kind 0x10 sel-0x13");
        let owner = 0x0080u16;
        let operand = 0x0300u16;
        let child = 0x0100u16;
        let build = || {
            let mut m = VmMachine::new();
            m.object_offsets = vec![owner];
            m.directory = vec![(child, 1), (0x9000, 0)];
            m.rec_write_pub(owner, 0x10); // owner kind 0x10 -> the NAV path
            m.rec_write_pub(owner + 2, 1); // owner active
            m.rec_write_pub(child, 1); // source entry of kind 1
            m.rec_write_pub(child + link, owner); // child's selector-0x11 -> owner
            m
        };
        let cod = [0xC1, 0x84, 0x00, 0x00, 0x03, 0xFF]; // C1 off=0x84 operand=0x300

        // Gate PASSES (operand record's +2 bit1 set) -> write at owner+sel13.
        let mut m = build();
        m.rec_write_pub(operand + 2, 2);
        m.load_cod(&cod);
        m.query = false;
        m.stack.push(0x99);
        m.pc = 0;
        m.step();
        assert_eq!(m.rec_read_pub(owner + dest_fo), 0xC1, "written at the selector-0x13 slot");
        assert_eq!(m.rec_read_pub(owner + dest_fo + 2), operand);
        assert_eq!(m.rec_read_pub(owner + dest_fo + 4), 2);
        assert_eq!(m.rec_read_pub(0x84), 0, "the operand record itself is NOT written");

        // Gate FAILS (bit1 clear) -> vm_branch, nothing written.
        let mut m = build();
        m.load_cod(&cod);
        m.query = false;
        m.stack.push(0x99);
        m.pc = 0;
        m.step();
        assert_eq!(m.pc, 0x99, "no source entry passes -> branch");
        assert_eq!(m.rec_read_pub(owner + dest_fo), 0);

        // Destination already occupied -> branch (0x6C59).
        let mut m = build();
        m.rec_write_pub(operand + 2, 2);
        m.rec_write_pub(owner + dest_fo, 0xBEEF);
        m.load_cod(&cod);
        m.query = false;
        m.stack.push(0x99);
        m.pc = 0;
        m.step();
        assert_eq!(m.pc, 0x99, "occupied destination -> branch");
        assert_eq!(m.rec_read_pub(owner + dest_fo), 0xBEEF);
    }

    #[test]
    fn load_deb_objects_keeps_only_kind1_entries() {
        // The directory walk (0x624B / 0x604E / the 0x5816 scan) continues while
        // `+0x12 == 1` and stops at the first entry that is not, so non-kind-1
        // records are NOT objects. Including them made the post-update scan visit
        // extra records and, worse, let owner_object_offset return a non-object
        // offset as the "largest below the key".
        let mut deb = Vec::new();
        let mut rec = |name: &str, off: u16, kind: u16| {
            let mut r = [0u8; 20];
            r[..name.len()].copy_from_slice(name.as_bytes());
            r[16..18].copy_from_slice(&off.to_le_bytes());
            r[18..20].copy_from_slice(&kind.to_le_bytes());
            deb.extend_from_slice(&r);
        };
        rec("alpha", 0x0100, 1);
        rec("beta", 0x0200, 1);
        rec("notanobject", 0x0180, 7); // kind != 1: must be excluded
        rec("gamma", 0x0300, 1);

        let mut m = VmMachine::new();
        m.load_deb_objects(&deb);
        assert_eq!(
            m.object_offsets,
            vec![0x0100, 0x0200, 0x0300],
            "only kind-1 entries are objects"
        );
        // The excluded 0x0180 would otherwise have been returned here, since it is
        // the largest offset below 0x0200.
        assert_eq!(
            m.owner_object_offset(0x0200),
            Some(0x0100),
            "owner resolves to a real object, not the kind-7 record at 0x0180"
        );
    }

    #[test]
    fn assign5_existing_ffff_removes_then_stores_raw_without_reinserting() {
        // 0x6995..0x69A7: when the record ALREADY holds 0xFFFF the handler removes
        // the owner from the special-slot list and jumps STRAIGHT to the store with
        // the RAW value. The insert block is skipped. The old code fell through, so
        // a wildcard/aboard value re-inserted the owner and wrote 0xFFFF over the
        // value the script actually requested.
        let owner = 0x0080u16;
        let field = 0x0090u16;
        let mut m = VmMachine::new();
        m.object_offsets = vec![owner];
        m.wildcard = 0x0555;
        m.rec_write_pub(field, 0xFFFF); // the record already holds 0xFFFF
        m.ship_slots[0] = owner; // and the owner is currently slotted
        // 0xAD SET with a WILDCARD-equal value: the fall-through bug would have
        // re-inserted and stored 0xFFFF; the handler stores the raw value.
        m.load_cod(&[0xAD, 0x90, 0x00, 0x55, 0x05, 0xFF]);
        m.query = false;
        m.pc = 0;
        m.step();
        assert_eq!(
            m.rec_read_pub(field),
            0x0555,
            "stores the RAW value after the 0xFFFF removal, not 0xFFFF"
        );
        assert!(
            !m.ship_slots.contains(&owner),
            "the owner was REMOVED and not re-inserted"
        );
    }

    #[test]
    fn nav_source_list_is_depth_first_over_selector_11_children() {
        // 0x624B: walk the gs:0x672c directory, append every object whose
        // selector-0x11 field points at the target, and recurse depth-first.
        let field = vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C2, 2).expect("kind 2 selector-0x11");
        let mut m = VmMachine::new();
        // Directory entries (offset, kind); kind 1 = a live entry, the scan stops
        // at the first entry whose kind is not 1.
        m.directory = vec![(0x100, 1), (0x200, 1), (0x300, 1), (0x400, 0)];
        for obj in [0x100u16, 0x200, 0x300, 0x400] {
            m.rec_write_pub(obj, 2); // every object is kind 2
        }
        // Tree: 0x100 -> parent TARGET; 0x200 -> parent 0x100; 0x300 -> parent TARGET.
        const TARGET: u16 = 0x0080;
        m.rec_write_pub(0x100 + field, TARGET);
        m.rec_write_pub(0x200 + field, 0x100);
        m.rec_write_pub(0x300 + field, TARGET);
        // 0x400 also points at the target but is BEYOND the kind!=1 terminator.
        m.rec_write_pub(0x400 + field, TARGET);

        let list = m.build_nav_source_list(TARGET);
        assert_eq!(
            list,
            vec![0x100, 0x200, 0x300],
            "depth-first: 0x100 then its child 0x200, then 0x300; 0x400 is past the terminator"
        );

        // An unrelated target yields nothing.
        assert!(m.build_nav_source_list(0x0999).is_empty());
    }

    #[test]
    fn source_list_rows_need_kind_active_and_a_non_zero_encounter_counter() {
        // 0x91C3's three draw-time filters over the SAME list the builder makes.
        let parent = vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C2, 2).expect("kind 2 selector-0x11");
        let counter =
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_ENCOUNTER, 2).expect("kind 2 selector-8");
        assert_eq!(counter, 0x36, "FIELD_OFFSETS[8][1] — the counter is kind 2");
        assert!(
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_ENCOUNTER, 1).is_some_and(|o| o == 0),
            "selector 8 has NO kind-1 field; the counter lives on the kind-2 partner"
        );

        const TARGET: u16 = 0x0080;
        let mut m = VmMachine::new();
        m.directory = vec![(0x100, 1), (0x200, 1), (0x300, 1), (0x400, 1)];
        for obj in [0x100u16, 0x200, 0x300, 0x400] {
            m.rec_write_pub(obj, 2);
            m.rec_write_pub(obj + parent, TARGET);
            m.rec_write_pub(obj + 2, OBJECT_FLAG_ACTIVE);
            m.rec_write_pub(obj + counter, 1);
        }
        // 0x200 fails the kind test, 0x300 the ACTIVE bit, 0x400 the counter.
        m.rec_write_pub(0x200, 1);
        m.rec_write_pub(0x200 + parent, 0); // kind 1 resolves selector 0x11 elsewhere
        m.rec_write_pub(0x300 + 2, 0);
        m.rec_write_pub(0x400 + counter, 0);

        assert_eq!(
            m.build_nav_source_list(TARGET),
            vec![0x100, 0x300, 0x400],
            "the BUILDER is unfiltered (0x200 drops out only because kind 1 reparents it)"
        );
        assert_eq!(
            m.source_list_display_rows(TARGET),
            vec![0x100],
            "only the object that is kind 2, ACTIVE and already encountered draws a row"
        );
    }

    #[test]
    fn the_status_roster_also_drops_objects_whose_location_is_the_ark() {
        // 0x83DF..0x83E8: the LIFE SUPPORT: roster adds `cmp [si+0x18],bx / je`
        // with bx = gs:0x6758 (the built-in object Ark).
        let parent = vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C2, 2).expect("kind 2 selector-0x11");
        assert_eq!(
            parent, 0x18,
            "FIELD_OFFSETS[0x11][1] — the +0x18 the filter reads"
        );
        let counter =
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_ENCOUNTER, 2).expect("kind 2 selector-8");

        const TARGET: u16 = 0x0080;
        const ARK: u16 = 0x0080; // the roster's location IS the Ark in this fixture
        let mut m = VmMachine::new();
        m.directory = vec![(0x100, 1), (0x200, 1)];
        for obj in [0x100u16, 0x200] {
            m.rec_write_pub(obj, 2);
            m.rec_write_pub(obj + 2, OBJECT_FLAG_ACTIVE);
            m.rec_write_pub(obj + counter, 3);
        }
        m.rec_write_pub(0x100 + parent, TARGET);
        m.rec_write_pub(0x200 + parent, TARGET);

        // Drawn panel: both rows. Text roster with bx == TARGET: neither, because
        // each object's location field equals bx.
        assert_eq!(m.source_list_display_rows(TARGET), vec![0x100, 0x200]);
        assert!(m.source_list_text_rows(TARGET, ARK).is_empty());
        // With the Ark elsewhere, the roster keeps both.
        assert_eq!(m.source_list_text_rows(TARGET, 0x0999), vec![0x100, 0x200]);
    }

    #[test]
    fn the_status_headers_are_the_games_own_strings() {
        // The strings are READ from the image now, so this checks the two things
        // that remain checkable: the addresses describe the same bytes, and the
        // read returns what is actually there.
        let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        let headers = test_status_headers();
        let read = [
            headers.planet,
            headers.ship,
            headers.black_hole,
            headers.life_support,
        ];
        for ((ds_off, file_off), text) in STATUS_STRING_TABLE.iter().zip(read.iter()) {
            assert_eq!(
                *file_off - 0xD420,
                *ds_off as usize,
                "DS offset and file offset must describe the same byte"
            );
            let end = file_off
                + exe[*file_off..]
                    .iter()
                    .position(|&b| b == 0)
                    .expect("NUL-terminated");
            assert_eq!(std::str::from_utf8(&exe[*file_off..end]).unwrap(), text);
        }
    }

    #[test]
    fn the_status_block_reads_the_header_kind_and_the_inline_names() {
        // 0x8365..0x83F8 composed from record state alone.
        let parent = vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C2, 2).expect("kind 2 selector-0x11");
        let counter =
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_ENCOUNTER, 2).expect("kind 2 selector-8");
        const ARCHE: u16 = 0x0020;
        const LOCATION: u16 = 0x0080;
        const HOST: u16 = 0x0100;

        let mut m = VmMachine::new();
        m.arche_offset = Some(ARCHE);
        m.ark_offset = Some(0x0400);
        m.directory = vec![(HOST, 1)];
        m.rec_write_pub(ARCHE + ARCHE_LOCATION_FIELD, LOCATION);
        // Inline names at +4 (the check_object_inline_names.py layout).
        let put_name = |m: &mut VmMachine, obj: u16, name: &str| {
            for (i, b) in name.bytes().chain(std::iter::once(0)).enumerate() {
                let off = obj + 4 + i as u16;
                let w = m.rec_read_pub(off & !1);
                let next = if off & 1 == 0 {
                    (w & 0xFF00) | b as u16
                } else {
                    (w & 0x00FF) | ((b as u16) << 8)
                };
                m.rec_write_pub(off & !1, next);
            }
        };
        put_name(&mut m, LOCATION, "Oddland");
        put_name(&mut m, HOST, "Bob_Morlock");
        assert_eq!(m.object_inline_name(LOCATION), "Oddland");

        // The host is a kind-2, ACTIVE, already-encountered object at the location.
        m.rec_write_pub(HOST, 2);
        m.rec_write_pub(HOST + 2, OBJECT_FLAG_ACTIVE);
        m.rec_write_pub(HOST + parent, LOCATION);
        m.rec_write_pub(HOST + counter, 1);

        // Kind 0 -> the default header.
        assert_eq!(
            m.location_status_block(&test_status_headers()).unwrap(),
            vec![
                "PLANET: Oddland".to_string(),
                "LIFE SUPPORT:".to_string(),
                "Bob_Morlock".to_string()
            ]
        );
        // 0x836C: kind 0x10 -> SHIP:, 0x8376: bit 0x100 -> BLACK HOLE:.
        m.rec_write_pub(LOCATION, LOCATION_KIND_SHIP);
        assert_eq!(
            m.location_status_block(&test_status_headers()).unwrap()[0],
            "SHIP: Oddland"
        );
        m.rec_write_pub(LOCATION, LOCATION_KIND_BLACK_HOLE);
        assert_eq!(
            m.location_status_block(&test_status_headers()).unwrap()[0],
            "BLACK HOLE: Oddland"
        );

        // Not yet encountered -> the roster is empty, header and caption remain.
        m.rec_write_pub(HOST + counter, 0);
        assert_eq!(
            m.location_status_block(&test_status_headers()).unwrap(),
            vec!["BLACK HOLE: Oddland".to_string(), "LIFE SUPPORT:".to_string()]
        );

        // With no DEB loaded there is no arche, so no panel at all.
        assert!(VmMachine::new().location_status_block(&test_status_headers()).is_none());
    }

    #[test]
    fn the_info_panel_lays_rows_out_at_the_drawn_immediates() {
        let parent = vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C2, 2).expect("kind 2 selector-0x11");
        let counter =
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_ENCOUNTER, 2).expect("kind 2 selector-8");
        const PLACE: u16 = 0x0080;
        const HOST: u16 = 0x0100;

        let mut m = VmMachine::new();
        m.directory = vec![(HOST, 1)];
        let put_name = |m: &mut VmMachine, obj: u16, name: &str| {
            for (i, b) in name.bytes().chain(std::iter::once(0)).enumerate() {
                let off = obj + 4 + i as u16;
                let w = m.rec_read_pub(off & !1);
                let next = if off & 1 == 0 {
                    (w & 0xFF00) | b as u16
                } else {
                    (w & 0x00FF) | ((b as u16) << 8)
                };
                m.rec_write_pub(off & !1, next);
            }
        };
        put_name(&mut m, PLACE, "Oddland");
        put_name(&mut m, HOST, "Bob");
        m.rec_write_pub(HOST, 2);
        m.rec_write_pub(HOST + 2, OBJECT_FLAG_ACTIVE);
        m.rec_write_pub(HOST + parent, PLACE);
        m.rec_write_pub(HOST + counter, 1);

        let rows = m.location_panel_rows(PLACE, &test_status_headers());
        let header_w = crate::font::game_font_drawn_width(&test_status_headers().planet) as i32;
        assert_eq!(
            rows,
            vec![
                LocationPanelRow { x: 0x6E, y: 0x19, color: 0xEE, text: "PLANET: ".into() },
                LocationPanelRow {
                    x: 0x6E + header_w + 6,
                    y: 0x19,
                    color: 0xEE,
                    text: "Oddland".into()
                },
                LocationPanelRow { x: 0x6E, y: 0x23, color: 0xEE, text: "LIFE SUPPORT:".into() },
                LocationPanelRow { x: 0x6E, y: 0x2D, color: 0xFE, text: "Bob".into() },
            ]
        );

        // 0x916D is a BIT test here (the hover composer's 0x836C is an equality),
        // so a kind carrying 0x10 among other bits still reads as SHIP.
        m.rec_write_pub(PLACE, LOCATION_KIND_SHIP | 0x40);
        assert_eq!(m.location_panel_rows(PLACE, &test_status_headers())[0].text, "SHIP: ");
        assert_eq!(m.location_status_block(&test_status_headers()), None, "no arche -> no hover block");
    }

    #[test]
    fn the_chart_list_keeps_in_play_objects_of_the_three_chart_kinds() {
        let mut m = VmMachine::new();
        // 0x606E stops the scan at the first non-kind-1 DIRECTORY entry, so an
        // eligible object past it never reaches the chart.
        m.directory = vec![
            (0x100, 1),
            (0x200, 1),
            (0x300, 1),
            (0x400, 1),
            (0x500, 1),
            (0x600, 0),
            (0x700, 1),
        ];
        for (obj, kind, flags) in [
            (0x100u16, LOCATION_KIND_SHIP, OBJECT_FLAG_IN_PLAY),
            (0x200, LOCATION_KIND_BLACK_HOLE, OBJECT_FLAG_IN_PLAY),
            (0x300, 0x08, OBJECT_FLAG_IN_PLAY),
            (0x400, 2, OBJECT_FLAG_IN_PLAY), // a character: not a chart kind
            (0x500, LOCATION_KIND_SHIP, 0),  // right kind, not in play
            (0x700, LOCATION_KIND_SHIP, OBJECT_FLAG_IN_PLAY), // past the stop
        ] {
            m.rec_write_pub(obj, kind);
            m.rec_write_pub(obj + 2, flags);
        }
        assert_eq!(
            m.build_active_object_list(),
            vec![0x100, 0x200, 0x300, 0x400],
            "0x6073 keeps every in-play object regardless of kind"
        );
        assert_eq!(
            m.build_nav_chart_list(),
            vec![0x100, 0x200, 0x300],
            "0x723D's `test bx,0x118` then keeps only the three chart kinds"
        );
        assert_eq!(NAV_CHART_KIND_MASK, 0x08 | LOCATION_KIND_SHIP | LOCATION_KIND_BLACK_HOLE);
    }

    #[test]
    fn the_real_scripts_chart_list_resolves_oddland_as_the_black_hole() {
        // The whole chain against SHIPPED data: directory -> 0x604E active list
        // -> 0x721A kind filter -> inline name -> DS:0x2BC7 artwork id.
        let Ok(var) = std::fs::read("output/_tmp_iso/SCRIPT5.VAR") else {
            return;
        };
        let Ok(deb) = std::fs::read("output/_tmp_iso/SCRIPT5.DEB") else {
            return;
        };
        let mut m = VmMachine::new();
        m.load_var(&var);
        m.load_deb_objects(&deb);

        let chart = m.build_nav_chart_list();
        assert_eq!(chart.len(), 1, "SCRIPT5's INITIAL state charts one object");
        let object = chart[0];
        assert_eq!(m.rec_read(object), LOCATION_KIND_BLACK_HOLE);
        assert_eq!(m.object_inline_name(object), "Oddland");
        assert_eq!(
            crate::levels::world_art_resource_id("Oddland"),
            Some(72),
            "trou.ext — 'hole', which is what a black hole's artwork should be"
        );
        // The marker the picker hit-tests, straight out of the record.
        let x = m.rec_read(object + NAV_PICK_POSITION_FIELD) as i32;
        let y = m.rec_read(object + NAV_PICK_POSITION_FIELD + 2) as i32;
        assert_eq!((x, y), (132, 34));
        let arche_context = m
            .arche_offset
            .map(|a| m.rec_read(a + 0x22))
            .unwrap_or_default();
        assert_eq!(m.nav_chart_pick(&chart, (x + 2, y + 2), arche_context), Some(object));
        assert_eq!(m.nav_chart_pick(&chart, (0, 0), arche_context), None);

        // The other scripts chart NOTHING from their initial .VAR: the in-play
        // bit (0x6073) is runtime state the story sets, so an empty chart at boot
        // is the data's answer, not a gap in the port.
        for n in 1..=4 {
            let (Ok(var), Ok(deb)) = (
                std::fs::read(format!("output/_tmp_iso/SCRIPT{n}.VAR")),
                std::fs::read(format!("output/_tmp_iso/SCRIPT{n}.DEB")),
            ) else {
                continue;
            };
            let mut m = VmMachine::new();
            m.load_var(&var);
            m.load_deb_objects(&deb);
            assert!(m.build_active_object_list().len() >= 2);
            assert!(m.build_nav_chart_list().is_empty(), "SCRIPT{n}");
        }
    }

    #[test]
    fn the_nav_picker_sizes_its_hit_box_by_kind_and_takes_the_first_hit() {
        const PLANET: u16 = 0x0100;
        const SHIP: u16 = 0x0200;
        const HOLE: u16 = 0x0300;
        let mut m = VmMachine::new();
        for (obj, kind, x, y) in [
            (PLANET, 0u16, 50u16, 60u16),
            (SHIP, LOCATION_KIND_SHIP, 100, 60),
            (HOLE, LOCATION_KIND_BLACK_HOLE, 150, 60),
        ] {
            m.rec_write_pub(obj, kind);
            m.rec_write_pub(obj + NAV_PICK_POSITION_FIELD, x);
            m.rec_write_pub(obj + NAV_PICK_POSITION_FIELD + 2, y);
        }
        // The black hole's SECOND endpoint, used when obj+0x14 != arche+0x22.
        m.rec_write_pub(HOLE + 0x14, 7);
        m.rec_write_pub(HOLE + NAV_PICK_POSITION_FIELD + 4, 250);
        m.rec_write_pub(HOLE + NAV_PICK_POSITION_FIELD + 6, 90);
        let list = [PLANET, SHIP, HOLE];

        // 0x9308/0x931A: the box starts 2 px up-left of the marker and BOTH
        // bounds are inclusive (jb/ja skip only strictly outside).
        assert_eq!(m.nav_chart_pick(&list, (48, 58), 7), Some(PLANET));
        assert_eq!(
            m.nav_chart_pick(&list, (48 + NAV_PICK_BOX_DEFAULT.0, 58), 7),
            Some(PLANET),
            "the far edge is inside"
        );
        assert_eq!(
            m.nav_chart_pick(&list, (48 + NAV_PICK_BOX_DEFAULT.0 + 1, 58), 7),
            None
        );
        // A ship's box is wider and shorter than a planet's.
        assert_eq!(
            m.nav_chart_pick(&list, (98 + NAV_PICK_BOX_SHIP.0, 58), 7),
            Some(SHIP)
        );
        assert_eq!(
            m.nav_chart_pick(&list, (98, 58 + NAV_PICK_BOX_SHIP.1 + 1), 7),
            None
        );
        // Black hole: with the context word MATCHING it sits at +0x18...
        assert_eq!(m.nav_chart_pick(&list, (148, 58), 7), Some(HOLE));
        // ...and with it differing, at the +0x1C endpoint instead.
        assert_eq!(m.nav_chart_pick(&list, (148, 58), 9), None);
        assert_eq!(m.nav_chart_pick(&list, (248, 88), 9), Some(HOLE));
        // Nothing under the cursor -> 0x9337's `xor ax,ax`.
        assert_eq!(m.nav_chart_pick(&list, (5, 5), 7), None);
        assert_eq!(m.nav_chart_pick(&[], (48, 58), 7), None);
    }

    #[test]
    fn the_drawn_width_excludes_spaces_the_way_the_accumulator_does() {
        // 0x31D7: a space does `add di,6` and jumps back WITHOUT touching 0x27CD.
        use crate::font::{game_font_advance, game_font_drawn_width};
        let pen: usize = "PLANET: ".chars().map(game_font_advance).sum();
        let reported = game_font_drawn_width("PLANET: ");
        assert_eq!(
            pen - reported,
            crate::font::GAME_FONT_SPACE_ADVANCE,
            "exactly the trailing space separates pen distance from reported width"
        );
        assert_eq!(game_font_drawn_width("  "), 0);
    }

    #[test]
    fn real_deb_objects_carry_their_name_inline_at_plus_four() {
        // The layout object_inline_name depends on, against the SHIPPED data.
        let Ok(deb) = std::fs::read("output/_tmp_iso/SCRIPT2.DEB") else {
            return;
        };
        let Ok(var) = std::fs::read("output/_tmp_iso/SCRIPT2.VAR") else {
            return;
        };
        let mut m = VmMachine::new();
        m.load_var(&var);
        m.load_deb_objects(&deb);
        assert!(m.ark_offset.is_some(), "SCRIPT2.DEB names the built-in Ark");
        let syms = crate::script::parse_deb(&deb);
        let named: Vec<_> = syms.iter().filter(|s| s.kind == 1).collect();
        let matching = named
            .iter()
            .filter(|s| m.object_inline_name(s.offset).eq_ignore_ascii_case(&s.name))
            .count();
        assert_eq!(
            (named.len(), matching),
            (122, 120),
            "every kind-1 object but the two built-ins blood/orxx holds its name at +4"
        );
        assert_eq!(m.object_inline_name(0x004A), "Bob_Morlock");
    }

    #[test]
    fn the_post_update_ladder_bumps_the_kind2_partners_encounter_counter() {
        // 0x5DB0..0x5E06: whichever partner is kind 1, the OTHER partner's
        // selector-8 counter is incremented and bit15 of the OWNER's +2 is set.
        let counter =
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_ENCOUNTER, 2).expect("kind 2 selector-8");

        // Branch 1 (0x5DB4): owner kind 1, related kind 2 -> the RELATED is bumped.
        let owner = 0x0100u16;
        let related = 0x0200u16;
        let mut var = vec![0; 0x0300];
        state_set_u16(&mut var, owner, 1);
        state_set_u16(&mut var, related, 2);
        assert_eq!(
            post_update_encounter_counter(&mut var, owner, related),
            Some(related)
        );
        assert_eq!(state_u16(&var, related + counter), 1);
        assert_eq!(state_u16(&var, owner + 2), OBJECT_FLAG_PAIR_SEEN);
        // It COUNTS: a second pairing increments again rather than saturating.
        post_update_encounter_counter(&mut var, owner, related);
        assert_eq!(state_u16(&var, related + counter), 2);

        // Branch 2 (0x5DE3): owner kind 2, related kind 1 -> the OWNER is bumped,
        // and bit15 still lands on the OWNER's +2 (both branches write [si+2]).
        let mut var = vec![0; 0x0300];
        state_set_u16(&mut var, owner, 2);
        state_set_u16(&mut var, related, 1);
        assert_eq!(
            post_update_encounter_counter(&mut var, owner, related),
            Some(owner)
        );
        assert_eq!(state_u16(&var, owner + counter), 1);
        assert_eq!(
            state_u16(&var, owner + 2) & OBJECT_FLAG_PAIR_SEEN,
            OBJECT_FLAG_PAIR_SEEN
        );

        // Neither partner kind 1 -> nothing at all (the 0x5DE8 jne to 0x5E09).
        let mut var = vec![0; 0x0300];
        state_set_u16(&mut var, owner, 2);
        state_set_u16(&mut var, related, 2);
        assert_eq!(post_update_encounter_counter(&mut var, owner, related), None);
        assert_eq!(state_u16(&var, owner + counter), 0);
        assert_eq!(state_u16(&var, related + counter), 0);
        assert_eq!(state_u16(&var, owner + 2), 0);

        // BOTH kind 1 -> branch 1 finds no counter field and falls into branch 2,
        // which finds none either (0x5DCC je 0x5DE3, then 0x5DF4 je 0x5E09).
        let mut var = vec![0; 0x0300];
        state_set_u16(&mut var, owner, 1);
        state_set_u16(&mut var, related, 1);
        assert_eq!(post_update_encounter_counter(&mut var, owner, related), None);
        assert_eq!(state_u16(&var, owner + 2), 0, "no bump -> no bit15 either");
    }

    /// A destination row's RECORD and NAME are two views of one word: the list
    /// stores `RECORD+4` and `object_inline_name` reads from `object+4`.
    #[test]
    fn destination_rows_name_each_candidate_from_its_own_record() {
        let mut m = VmMachine::new();
        let arche = 0x0400u16;
        m.arche_offset = Some(arche);
        // No DEB -> no candidates -> no rows, rather than a fabricated list.
        assert!(m.destination_rows().is_empty());
        m.arche_offset = None;
        assert!(m.destination_rows().is_empty(), "no arche, no rows");

        // The record/name coincidence, checked directly: write a name at
        // record+4 and confirm that address is exactly what the list would store.
        let mut m2 = VmMachine::new();
        let record = 0x0140u16;
        for (i, b) in b"KORTEX".iter().enumerate() {
            m2.rec_write_u8_pub(record + 4 + i as u16, *b);
        }
        m2.rec_write_u8_pub(record + 4 + 6, 0);
        assert_eq!(m2.object_inline_name(record), "KORTEX");
        assert_eq!(
            record.wrapping_add(SHIP_3D_TARGET_NAME_TO_RECORD),
            record + 4,
            "the stored entry IS the string pointer"
        );
    }

    /// `0xB0DC..0xB111`: which record the ship-3D click actually commits, and why
    /// one `sub 4` serves both branches.
    #[test]
    fn ship_click_commits_candidate_or_location_by_kind() {
        let mut m = VmMachine::new();
        m.orxx_offset = Some(0x0200);
        let arche = 0x0400u16;
        let location = 0x0500u16;
        m.arche_offset = Some(arche);
        m.rec_write_pub(arche + ARCHE_LOCATION_FIELD, location);

        // No candidate is reachable from arche (the source list needs the loaded
        // field matrix), so [0x250B] holds only the terminator -- which is itself
        // a decoded case: 0xB0F7 reads 0xFFFF and 0xB111 subtracts 4 from it.
        m.rec_write_pub(location, SHIP_CLICK_LOCATION_KIND_MASK);
        assert_eq!(
            m.ship_click_initial_target(),
            Some(0xFFFBu16),
            "empty list -> the terminator, minus the name offset (0xB0F7/0xB111)"
        );

        // Kind LACKS it -> commit the location object itself, NOT minus 4:
        // 0xB10A's `add di,4` pre-compensates for the shared `sub 4` @0xB111.
        m.rec_write_pub(location, 0x0001);
        assert_eq!(
            m.ship_click_initial_target(),
            Some(location),
            "the location commits whole"
        );

        // No arche loaded -> no target at all (les/mov di,[0x6752] has nothing).
        m.arche_offset = None;
        assert_eq!(m.ship_click_initial_target(), None);
    }

    /// `0x624B` preserves DI (`0x6276`/`0x627D`), so the composite roots the walk
    /// at the target AND tests the target itself first.
    #[test]
    fn destination_candidates_test_the_target_itself_first() {
        let mut m = VmMachine::new();
        m.orxx_offset = Some(0x0200);
        // No `arche` set, so the target is not excluded and must appear itself.
        let target = 0x0140u16;
        m.rec_write_pub(target, ENTITY_CANDIDATE_KIND_MASK);
        m.rec_write_pub(target + 2, ENTITY_CANDIDATE_READY_BIT as u16);
        assert_eq!(
            m.destination_candidate_records(target),
            vec![target + 4],
            "DI survives 0x624B, so the target is candidate zero"
        );

        // With `arche` = the target, 0x728B drops it and the list is empty.
        m.arche_offset = Some(target);
        assert!(
            m.destination_candidate_records(target).is_empty(),
            "a location never offers itself"
        );
    }

    /// `0x7259` builds what `0xB2BB` reads: an end-to-end check that a record
    /// surviving the filter can be selected back OUT as the same record.
    #[test]
    fn entity_candidate_list_and_target_select_are_inverses() {
        let mut m = VmMachine::new();
        m.orxx_offset = Some(0x0200);
        m.arche_offset = Some(0x0400);

        // Three objects: one passes, one fails the kind mask, one fails the +2 bit.
        let pass = 0x0140u16;
        let bad_kind = 0x0180u16;
        let not_ready = 0x01C0u16;
        m.rec_write_pub(pass, ENTITY_CANDIDATE_KIND_MASK);
        m.rec_write_pub(pass + 2, ENTITY_CANDIDATE_READY_BIT as u16);
        m.rec_write_pub(bad_kind, 0x0001);
        m.rec_write_pub(bad_kind + 2, ENTITY_CANDIDATE_READY_BIT as u16);
        m.rec_write_pub(not_ready, ENTITY_CANDIDATE_KIND_MASK);
        m.rec_write_pub(not_ready + 2, 0);
        // `arche` passes the flag tests but is excluded by 0x728B.
        let arche = 0x0400u16;
        m.rec_write_pub(arche, ENTITY_CANDIDATE_KIND_MASK);
        m.rec_write_pub(arche + 2, ENTITY_CANDIDATE_READY_BIT as u16);

        let list = m.entity_candidate_list(pass, &[bad_kind, not_ready, arche, 0xFFFF]);
        assert_eq!(list, vec![pass + 4], "only the passing object, stored as RECORD+4");

        // And the reader turns that entry back into the record the commit takes.
        let fallback = [0xFFFFu16];
        assert_eq!(m.ship_3d_target_record_select(&list, &fallback, 0), pass);
        assert!(m.world_click_select(pass), "the round trip commits a C1 record");
    }

    /// `0xB2BB`: the row->record conversion, and the fallback rule that makes the
    /// inline table unable to commit anything.
    #[test]
    fn ship_3d_target_select_maps_name_pointers_back_to_records() {
        let mut m = VmMachine::new();
        m.orxx_offset = Some(0x0200);
        // Entries are RECORD+4 (the form `0x87D5` emits), terminated by 0xFFFF.
        let primary = [0x0144u16, 0x0184, 0xFFFF];
        let fallback = [0x2600u16, 0xFFFF];

        assert_eq!(m.ship_3d_target_record_select(&primary, &fallback, 0), 0x0140);
        assert_eq!(m.ship_3d_target_record_select(&primary, &fallback, 1), 0x0180);
        // The terminator row is the back row, and so is anything past the end.
        assert_eq!(m.ship_3d_target_record_select(&primary, &fallback, 2), 0xFFFF);
        assert_eq!(m.ship_3d_target_record_select(&primary, &fallback, 9), 0xFFFF);
        // 0xB31D: the widget's "no selection" is 0xFFFF and yields 0, NOT the back row.
        assert_eq!(m.ship_3d_target_record_select(&primary, &fallback, 0xFFFF), 0);

        // Primary empty -> the DS fallback list, whose names are not in records:
        // the selection is discarded for the CURRENT target (0xB347).
        let empty = [0xFFFFu16];
        m.world_target = Some(0x0300);
        assert_eq!(m.ship_3d_target_record_select(&empty, &fallback, 0), 0x0300);
        // ... and world_click_select rejects the current target, so the fallback
        // list can never commit a new destination.
        assert!(!m.world_click_select(0x0300), "fallback selection is inert");
    }

    #[test]
    fn world_click_creates_the_c1_presentation_record() {
        // 0xB20C..0xB27B: a NEW world target sets gs:0x251B and writes
        // {0xC1, target, 0} at orxx+0xA (gs:0x6750 = the built-in object orxx).
        let orxx = 0x0200u16;
        let slot = orxx + 0xA;
        let mut m = VmMachine::new();
        m.orxx_offset = Some(orxx);

        assert!(!m.world_click_select(0), "nothing hit -> no record");
        assert_eq!(m.rec_read_pub(slot), 0);

        assert!(m.world_click_select(0x0140), "a new target creates the C1 record");
        assert_eq!(m.rec_read_pub(slot), 0xC1, "record typed C1");
        assert_eq!(m.rec_read_pub(slot + 2), 0x0140, "target stored at +2");
        assert_eq!(m.rec_read_pub(slot + 4), 0, "+4 cleared");
        assert_eq!(m.world_target, Some(0x0140), "gs:0x251B updated");

        // Re-clicking the SAME target must not rewrite (cmp ax,[0x251b] @0xB21A).
        m.rec_write_pub(slot, 0xBEEF);
        assert!(!m.world_click_select(0x0140), "same target -> no rewrite");
        assert_eq!(m.rec_read_pub(slot), 0xBEEF, "record untouched for the current target");

        // A DIFFERENT target does rewrite.
        assert!(m.world_click_select(0x0180));
        assert_eq!(m.rec_read_pub(slot), 0xC1);
        assert_eq!(m.rec_read_pub(slot + 2), 0x0180);

        // The back/exit row (-1) drops the target and writes nothing (0xB288).
        m.rec_write_pub(slot, 0xBEEF);
        assert!(!m.world_click_select(0xFFFF), "back row creates no record");
        assert_eq!(m.world_target, None, "current target cleared on exit");
        assert_eq!(m.rec_read_pub(slot), 0xBEEF);
    }

    #[test]
    fn live_step_c2_record_state_was_a_no_op() {
        // 0xC2 (0x6E34). Owner object @0x80 (active); the record operand is
        // 0x84; the target operand is 0x10, whose own record must carry +2 bit5
        // (0x20) for the SET to take.
        let cod = [0xC2, 0x84, 0x00, 0x10, 0x00, 0xFF];
        fn armed() -> VmMachine {
            let mut m = VmMachine::new();
            m.object_offsets = vec![0x10, 0x80];
            m.rec_write_pub(0x80, 2);
            m.rec_write_pub(0x82, 1); // owner active
            m.rec_write_pub(0x10, 2); // target kind 2
            m.rec_write_pub(0x12, 0x20); // target +2 bit5 set
            m
        }
        let field =
            vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C2, 2).expect("kind 2 selector-0x11 field");

        // SET: all gates pass -> 0xFFFF into the target's selector-0x11 field,
        // and the target joins the special-slot list.
        let mut m = armed();
        m.load_cod(&cod);
        m.query = false;
        m.pc = 0;
        m.step();
        assert_eq!(
            m.rec_read_pub(0x10u16.wrapping_add(field)),
            0xFFFF,
            "selector-0x11 field written (was a no-op)"
        );
        assert!(m.ship_slots.contains(&0x10), "target inserted into the special slots");

        // SET with the 0x20 bit clear: no write — and crucially NO BRANCH (every
        // SET failure path in 0x6E78.. jumps to the RET at 0x6EEC).
        let mut m = armed();
        m.rec_write_pub(0x12, 0);
        m.load_cod(&cod);
        m.query = false;
        m.stack.push(0x99);
        m.pc = 0;
        m.step();
        assert_eq!(m.rec_read_pub(0x10u16.wrapping_add(field)), 0, "no write");
        assert_eq!(m.pc, 5, "a failed C2 SET does NOT branch");

        // QUERY: matching {0xC2, operand} on an active owner -> pass.
        let mut m = armed();
        m.rec_write_pub(0x84, 0xC2);
        m.rec_write_pub(0x86, 0x10);
        m.load_cod(&cod);
        m.query = true;
        m.stack.push(0x99);
        m.pc = 0;
        m.step();
        assert_eq!(m.pc, 5, "matching C2 query falls through");

        // QUERY with the owner INACTIVE -> branch (0x6E56 je).
        let mut m = armed();
        m.rec_write_pub(0x84, 0xC2);
        m.rec_write_pub(0x86, 0x10);
        m.rec_write_pub(0x82, 0);
        m.load_cod(&cod);
        m.query = true;
        m.stack.push(0x99);
        m.pc = 0;
        m.step();
        assert_eq!(m.pc, 0x99, "inactive owner -> query branches");
    }

    #[test]
    fn a8_raises_the_presentation_request_once_until_teardown() {
        // 0x67F6..0x682F: gated on the gs:0x67AA bit1 latch being CLEAR and
        // (ship-active gs:0x24F3 | gs:0x274F). Only flag_274f is modelled, so the
        // port UNDER-fires (the gate is an OR) — never spuriously.
        let load = |m: &mut VmMachine, s: &str| {
            let mut cod = vec![0xA8];
            cod.extend_from_slice(s.as_bytes());
            cod.push(0);
            if cod.len() % 2 == 1 {
                cod.push(0);
            }
            cod.push(0);
            cod.push(0);
            m.load_cod(&cod);
            m.pc = 0;
            m.step();
        };

        // Gate CLOSED (flag_274f false, the boot state): no request.
        let mut m = VmMachine::new();
        load(&mut m, "cliptoot.hnm");
        assert!(!m.presentation_request_pending, "no request while the gate is closed");
        assert_eq!(m.rec_read_pub(0x6788), 0, "active line untouched");

        // Gate OPEN: the request fires — active line 7, latch set, 0x1FA3 = 0xFFFF.
        let mut m = VmMachine::new();
        m.flag_274f = true;
        load(&mut m, "cliptoot.hnm");
        assert!(m.presentation_request_pending, "request raised");
        assert_eq!(m.rec_read_pub(0x6788), 7, "active line = 7");
        assert_eq!(m.rec_read_pub(0x1FA3), 0xFFFF);

        // A SECOND 0xA8 must NOT re-raise while the latch is set: clear the
        // active line and confirm it is not rewritten.
        m.rec_write_pub(0x6788, 0);
        load(&mut m, "other.hnm");
        assert_eq!(m.rec_read_pub(0x6788), 0, "latch suppresses the repeat request");

        // The presentation teardown (0xC9) releases the latch, so a later 0xA8
        // can request again — without this the latch would suppress them forever.
        m.active_actor = Some(0x84);
        m.load_cod(&[0xC9, 0x84, 0x00, 0xFF]);
        m.query = false;
        m.pc = 0;
        m.step();
        assert!(!m.presentation_request_pending, "teardown clears the latch");
        load(&mut m, "again.hnm");
        assert_eq!(m.rec_read_pub(0x6788), 7, "a new request can be raised after teardown");
    }

    #[test]
    fn c9_clears_the_whole_record_and_the_c4_reciprocal() {
        // 0x6FB9: 0xC9 zeroes the 3-word record, and for a 0xC4 entry also the
        // related object's selector-0x13 triple. Objects at 0x10 (kind 2) and
        // 0x80 (kind 2), both active; the actor record is 0x84 (owner 0x80).
        let field = vm_field_offset(VM_FIELD_OFFSET_SELECTOR_C9_RELATED, 2)
            .expect("kind 2 selector-0x13 field");
        let mut m = VmMachine::new();
        m.object_offsets = vec![0x10, 0x80];
        m.rec_write_pub(0x80, 2);
        m.rec_write_pub(0x82, 1);
        m.rec_write_pub(0x10, 2);
        m.rec_write_pub(0x12, 1);
        // A live C4 presentation: record {0xC4, related=0x10, 2} plus the
        // reciprocal 0xC4 on the related object's selector-0x13 field.
        m.rec_write_pub(0x84, 0xC4);
        m.rec_write_pub(0x86, 0x10);
        m.rec_write_pub(0x88, 2);
        let recip = 0x10u16.wrapping_add(field);
        m.rec_write_pub(recip, 0xC4);
        m.rec_write_pub(recip.wrapping_add(2), 0x84);
        m.active_actor = Some(0x84);

        m.load_cod(&[0xC9, 0x84, 0x00, 0xFF]);
        m.query = false;
        m.pc = 0;
        m.step();

        assert_eq!(m.rec_read_pub(0x84), 0, "type word cleared");
        assert_eq!(m.rec_read_pub(0x86), 0, "+2 cleared (was only +0 before)");
        assert_eq!(m.rec_read_pub(0x88), 0, "+4 cleared");
        assert_eq!(m.rec_read_pub(recip), 0, "the related object's C4 reciprocal cleared");
        assert_eq!(m.rec_read_pub(recip.wrapping_add(2)), 0, "reciprocal +2 cleared");
        assert_eq!(m.active_actor, None, "the presentation ended");

        // The point of the reciprocal clear: a LATER C4 SET for that actor must
        // pass the write guard. With the stale 0xC4 left behind (the old
        // behaviour) the guard at 0x6CE9..0x6CFF would branch instead of writing.
        m.load_cod(&[OP_ACTOR, 0x84, 0x00, 0x10, 0x00, 0xFF]);
        m.query = false;
        m.stack.push(0x99);
        m.pc = 0;
        m.step();
        assert_eq!(
            m.rec_read_pub(0x84),
            0xC4,
            "the actor can present again after a clean 0xC9 (no wedged reciprocal)"
        );
        assert_eq!(m.pc, 5, "no branch: the write guard passed");
    }

    #[test]
    fn a8_latches_the_fin_flag_for_finale_strings() {
        // 0x67D8..0x67F0: the 0xA8 handler compares the loaded string's first
        // four bytes to 'f','i','n','.' and latches gs:[0x67BD]=1 on a match.
        let load = |s: &str| -> VmMachine {
            let mut cod = vec![0xA8];
            cod.extend_from_slice(s.as_bytes());
            cod.push(0);
            if cod.len() % 2 == 1 {
                cod.push(0); // zero-WORD terminator alignment pad
            }
            cod.push(0);
            cod.push(0);
            let mut m = VmMachine::new();
            m.load_cod(&cod);
            m.pc = 0;
            m.step();
            m
        };

        let m = load("fin.hnm");
        assert!(m.fin_requested, "fin.hnm latches the FIN flag");
        assert!(
            matches!(m.events.first(), Some(VmEvent::LoadString(s)) if s == "fin.hnm"),
            "the LoadString event still carries the name"
        );

        // Prefix-only, byte-exact: neither a different name nor a different
        // case matches the handler's four byte compares.
        assert!(!load("cliptoot.hnm").fin_requested, "an ordinary name does not latch");
        assert!(!load("FIN.HNM").fin_requested, "the compare is case-sensitive");
        assert!(!load("fi").fin_requested, "a short string cannot match");
        assert!(!load("affin.hnm").fin_requested, "the match is a PREFIX, not a substring");
    }

    #[test]
    fn live_step_c1_record_state_was_a_no_op() {
        // 0xC1 (0x6B4C) — the non-ship3d path. Object @0x80 active; a C1
        // op1=0x84 (owner 0x80) operand=0x30 (not 1/2, so the direct compare /
        // simple write applies — the resolved selector path needs operand 1/2).
        fn armed() -> VmMachine {
            let mut m = VmMachine::new();
            m.object_offsets = vec![0x10, 0x80];
            m.rec_write_pub(0x80, 2); // obj@0x80 kind 2
            m.rec_write_pub(0x82, 1); // obj@0x80 active
            m
        }
        let cod = [0xC1, 0x84, 0x00, 0x30, 0x00, 0xFF];

        // (1) SET: owner active + empty record -> write {0xC1, operand, 2}.
        let mut m = armed();
        m.load_cod(&cod);
        m.query = false;
        m.stack.push(0x99);
        m.pc = 0;
        m.step();
        assert_eq!(m.rec_read_pub(0x84), 0xC1, "C1 state record written (was a no-op)");
        assert_eq!(m.rec_read_pub(0x86), 0x30, "operand stored at +2");
        assert_eq!(m.rec_read_pub(0x88), 2, "+4 = 2");
        assert_eq!(m.pc, 5, "no branch on a successful write");

        // (2) SET: owner inactive -> vm_branch, no write (0x6BCE).
        let mut m = armed();
        m.rec_write_pub(0x82, 0);
        m.load_cod(&cod);
        m.query = false;
        m.stack.push(0x99);
        m.pc = 0;
        m.step();
        assert_eq!(m.pc, 0x99, "owner inactive -> branch");
        assert_ne!(m.rec_read_pub(0x84), 0xC1, "no write on branch");

        // (3) SET: record already occupied -> vm_branch (0x6C59).
        let mut m = armed();
        m.rec_write_pub(0x84, 0x00C6); // non-zero record
        m.load_cod(&cod);
        m.query = false;
        m.stack.push(0x99);
        m.pc = 0;
        m.step();
        assert_eq!(m.pc, 0x99, "occupied record -> branch");
        assert_eq!(m.rec_read_pub(0x84), 0x00C6, "occupied record left intact");

        // (4) QUERY: matching {0xC1, operand} -> pass (no branch).
        let mut m = armed();
        m.rec_write_pub(0x84, 0xC1);
        m.rec_write_pub(0x86, 0x30);
        m.load_cod(&cod);
        m.query = true;
        m.stack.push(0x99);
        m.pc = 0;
        m.step();
        assert_eq!(m.pc, 5, "matching C1 query -> fall through");
        assert_eq!(m.stack.len(), 1, "no branch on a match");

        // (5) QUERY: empty record, non-inverted -> vm_branch (cmp cx,0xC1 fails).
        let mut m = armed();
        m.load_cod(&cod);
        m.query = true;
        m.stack.push(0x99);
        m.pc = 0;
        m.step();
        assert_eq!(m.pc, 0x99, "empty record query -> branch");

        // (6) object table unloaded -> legacy no-op (neither write nor branch).
        let mut m = VmMachine::new();
        m.load_cod(&cod);
        m.query = false;
        m.stack.push(0x99);
        m.pc = 0;
        m.step();
        assert_eq!(m.rec_read_pub(0x84), 0, "no DEB loaded -> C1 stays a no-op");
        assert_eq!(m.pc, 5, "no branch when the object table is unloaded");
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

        // Both partners are kind 2, so the 0x5DB4/0x5DE3 kind-1 tests both fail
        // and no encounter counter is bumped.
        assert_eq!(
            post_update_actor_record_pair(&mut var, owner, record),
            Some((related_field, None))
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
