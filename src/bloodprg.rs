use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::vm;

pub const BLOODPRG_FILE_SIZE: usize = 86_680;
pub const BLOODPRG_SHA256: &str =
    "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823";
pub const VM_CODE_SEGMENT: u16 = 0x04da;
pub const DATA_SEGMENT: u16 = 0x0ce2;
pub const OPCODE_HANDLER_TABLE_FILE_OFFSET: usize = 0x142d0;
pub const OPCODE_LENGTH_TABLE_FILE_OFFSET: usize = 0x14338;
pub const DIALOGUE_FONT_ASCII_MAP_FILE_OFFSET: usize = 0x14c22;
pub const DIALOGUE_FONT_ADVANCES_FILE_OFFSET: usize = 0x14cd2;
pub const DIALOGUE_FONT_GLYPHS_FILE_OFFSET: usize = 0x14d28;
pub const DIALOGUE_FONT_ASCII_MAP_LEN: usize = 128;
pub const DIALOGUE_FONT_GLYPH_COUNT: usize = 86;
pub const DIALOGUE_FONT_GLYPH_HEIGHT: usize = 8;
pub const SND_ENTRY_SEGMENT: u16 = 0x0b1b;
pub const SND_ENTRY_OFFSET: u16 = 0x011d;

const SND_ENTRY_FAR_CALL: [u8; 5] = [
    0x9a,
    SND_ENTRY_OFFSET as u8,
    (SND_ENTRY_OFFSET >> 8) as u8,
    SND_ENTRY_SEGMENT as u8,
    (SND_ENTRY_SEGMENT >> 8) as u8,
];
const FAR_CALL_OPCODE: u8 = 0x9a;
const AX_SOURCE_SCAN_BACK: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct MzHeader {
    pub e_cblp: u16,
    pub e_cp: u16,
    pub e_crlc: u16,
    pub e_cparhdr: u16,
    pub e_minalloc: u16,
    pub e_maxalloc: u16,
    pub e_ss: u16,
    pub e_sp: u16,
    pub e_ip: u16,
    pub e_cs: u16,
    pub e_lfarlc: u16,
}

impl MzHeader {
    pub fn header_size(self) -> usize {
        self.e_cparhdr as usize * 16
    }

    pub fn image_total(self) -> usize {
        if self.e_cblp == 0 {
            self.e_cp as usize * 512
        } else {
            (self.e_cp as usize - 1) * 512 + self.e_cblp as usize
        }
    }

    pub fn load_size(self) -> usize {
        self.image_total().saturating_sub(self.header_size())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MzSummary {
    pub file_size: usize,
    pub header: MzHeader,
    pub header_size: usize,
    pub image_total: usize,
    pub load_size: usize,
    pub entry_file_offset: usize,
    pub trailing_bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct BinarySymbol {
    pub name: &'static str,
    pub file_offset: usize,
    pub segment: Option<u16>,
    pub offset: Option<u16>,
    pub ds_offset: Option<u16>,
    pub kind: &'static str,
    pub comment: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct OpcodeDescriptor {
    pub opcode: u8,
    pub len_mode0: u8,
    pub len_mode1_or_sentinel: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct OpcodeHandler {
    pub opcode: u8,
    pub handler_offset: u16,
    pub handler_file_offset: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct VmOpcodeSpec {
    pub opcode: u8,
    pub mnemonic: &'static str,
    pub family: &'static str,
    pub handler_offset: u16,
    pub handler_file_offset: usize,
    pub len_mode0: u8,
    pub len_mode1_or_sentinel: u8,
    pub mode_control: bool,
    pub variable_length: bool,
    pub rust_status: &'static str,
    pub notes: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DialogueFontTables {
    pub ascii_map_file_offset: usize,
    pub advances_file_offset: usize,
    pub glyphs_file_offset: usize,
    pub ascii_map: Vec<u8>,
    pub advances: Vec<u8>,
    pub glyph_rows: Vec<[u8; DIALOGUE_FONT_GLYPH_HEIGHT]>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BloodPrgInspection {
    pub summary: MzSummary,
    pub target_sha256: &'static str,
    pub known_symbols: Vec<BinarySymbol>,
    pub opcode_handlers: Vec<OpcodeHandler>,
    pub opcode_descriptors: Vec<OpcodeDescriptor>,
    pub vm_opcode_specs: Vec<VmOpcodeSpec>,
    pub snd_entry_call_sites: Vec<SndEntryCallSite>,
    pub dialogue_font: DialogueFontTables,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SndEntryCallSite {
    pub file_offset: usize,
    pub segment: u16,
    pub offset: u16,
    pub target_segment: u16,
    pub target_offset: u16,
    pub ax_value: Option<u16>,
    pub ax_source_file_offset: Option<usize>,
    pub ax_source: &'static str,
    pub intervening_far_calls: u8,
    pub note: &'static str,
}

pub struct BloodPrg {
    data: Vec<u8>,
    header: MzHeader,
}

impl BloodPrg {
    pub fn parse_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let data = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        Self::parse(data)
    }

    pub fn parse(data: Vec<u8>) -> Result<Self> {
        if data.len() < 0x20 {
            bail!("BLOODPRG.EXE is too small for an MZ header");
        }
        if data.get(0..2) != Some(b"MZ") && data.get(0..2) != Some(b"ZM") {
            bail!("BLOODPRG.EXE is not an MZ executable");
        }
        let header = MzHeader {
            e_cblp: u16_at(&data, 0x02)?,
            e_cp: u16_at(&data, 0x04)?,
            e_crlc: u16_at(&data, 0x06)?,
            e_cparhdr: u16_at(&data, 0x08)?,
            e_minalloc: u16_at(&data, 0x0a)?,
            e_maxalloc: u16_at(&data, 0x0c)?,
            e_ss: u16_at(&data, 0x0e)?,
            e_sp: u16_at(&data, 0x10)?,
            e_ip: u16_at(&data, 0x14)?,
            e_cs: u16_at(&data, 0x16)?,
            e_lfarlc: u16_at(&data, 0x18)?,
        };
        if header.image_total() > data.len() {
            bail!(
                "MZ image extends past file: image_total={} file_size={}",
                header.image_total(),
                data.len()
            );
        }
        Ok(Self { data, header })
    }

    pub fn summary(&self) -> MzSummary {
        let entry_file_offset = self.segoff_to_file(self.header.e_cs, self.header.e_ip);
        MzSummary {
            file_size: self.data.len(),
            header: self.header,
            header_size: self.header.header_size(),
            image_total: self.header.image_total(),
            load_size: self.header.load_size(),
            entry_file_offset,
            trailing_bytes: self.data.len().saturating_sub(self.header.image_total()),
        }
    }

    pub fn segoff_to_file(&self, segment: u16, offset: u16) -> usize {
        self.header.header_size() + segment as usize * 16 + offset as usize
    }

    pub fn ds_to_file(&self, ds_offset: u16) -> usize {
        self.segoff_to_file(DATA_SEGMENT, ds_offset)
    }

    pub fn opcode_handlers(&self) -> Result<Vec<OpcodeHandler>> {
        let bytes = self.slice(
            OPCODE_HANDLER_TABLE_FILE_OFFSET,
            vm::OPCODE_DESC.len() * 2,
            "opcode handler table",
        )?;
        Ok(bytes
            .chunks_exact(2)
            .enumerate()
            .map(|(idx, pair)| {
                let handler_offset = u16::from_le_bytes([pair[0], pair[1]]);
                OpcodeHandler {
                    opcode: vm::OP_MIN + idx as u8,
                    handler_offset,
                    handler_file_offset: self.segoff_to_file(VM_CODE_SEGMENT, handler_offset),
                }
            })
            .collect())
    }

    pub fn opcode_descriptors(&self) -> Result<Vec<OpcodeDescriptor>> {
        let bytes = self.slice(
            OPCODE_LENGTH_TABLE_FILE_OFFSET,
            vm::OPCODE_DESC.len() * 2,
            "opcode length table",
        )?;
        Ok(bytes
            .chunks_exact(2)
            .enumerate()
            .map(|(idx, pair)| OpcodeDescriptor {
                opcode: vm::OP_MIN + idx as u8,
                len_mode0: pair[0],
                len_mode1_or_sentinel: pair[1],
            })
            .collect())
    }

    pub fn vm_opcode_specs(&self) -> Result<Vec<VmOpcodeSpec>> {
        let handlers = self.opcode_handlers()?;
        let descriptors = self.opcode_descriptors()?;
        handlers
            .iter()
            .zip(descriptors.iter())
            .map(|(handler, descriptor)| {
                if handler.opcode != descriptor.opcode {
                    bail!(
                        "opcode handler/descriptor mismatch: handler={:#04x} descriptor={:#04x}",
                        handler.opcode,
                        descriptor.opcode
                    );
                }
                let meta = opcode_metadata(handler.opcode, handler.handler_file_offset);
                Ok(VmOpcodeSpec {
                    opcode: handler.opcode,
                    mnemonic: meta.mnemonic,
                    family: meta.family,
                    handler_offset: handler.handler_offset,
                    handler_file_offset: handler.handler_file_offset,
                    len_mode0: descriptor.len_mode0,
                    len_mode1_or_sentinel: descriptor.len_mode1_or_sentinel,
                    mode_control: descriptor.len_mode1_or_sentinel & 0x80 != 0,
                    variable_length: descriptor.len_mode0 == 0
                        && descriptor.len_mode1_or_sentinel == 0,
                    rust_status: meta.rust_status,
                    notes: meta.notes,
                })
            })
            .collect()
    }

    pub fn dialogue_font_tables(&self) -> Result<DialogueFontTables> {
        let ascii_map = self
            .slice(
                DIALOGUE_FONT_ASCII_MAP_FILE_OFFSET,
                DIALOGUE_FONT_ASCII_MAP_LEN,
                "dialogue font ASCII map",
            )?
            .to_vec();
        let advances = self
            .slice(
                DIALOGUE_FONT_ADVANCES_FILE_OFFSET,
                DIALOGUE_FONT_GLYPH_COUNT,
                "dialogue font advances",
            )?
            .to_vec();
        let glyph_bytes = self.slice(
            DIALOGUE_FONT_GLYPHS_FILE_OFFSET,
            DIALOGUE_FONT_GLYPH_COUNT * DIALOGUE_FONT_GLYPH_HEIGHT,
            "dialogue font glyph rows",
        )?;
        let glyph_rows = glyph_bytes
            .chunks_exact(DIALOGUE_FONT_GLYPH_HEIGHT)
            .map(|chunk| {
                let mut rows = [0u8; DIALOGUE_FONT_GLYPH_HEIGHT];
                rows.copy_from_slice(chunk);
                rows
            })
            .collect();
        Ok(DialogueFontTables {
            ascii_map_file_offset: DIALOGUE_FONT_ASCII_MAP_FILE_OFFSET,
            advances_file_offset: DIALOGUE_FONT_ADVANCES_FILE_OFFSET,
            glyphs_file_offset: DIALOGUE_FONT_GLYPHS_FILE_OFFSET,
            ascii_map,
            advances,
            glyph_rows,
        })
    }

    pub fn snd_entry_call_sites(&self) -> Vec<SndEntryCallSite> {
        self.data
            .windows(SND_ENTRY_FAR_CALL.len())
            .enumerate()
            .filter_map(|(file_offset, bytes)| {
                (bytes == SND_ENTRY_FAR_CALL).then(|| {
                    let (segment, offset) = self.file_to_known_segoff(file_offset);
                    let (ax_value, ax_source_file_offset, ax_source, intervening_far_calls) =
                        self.find_ax_source_before(file_offset);
                    SndEntryCallSite {
                        file_offset,
                        segment,
                        offset,
                        target_segment: SND_ENTRY_SEGMENT,
                        target_offset: SND_ENTRY_OFFSET,
                        ax_value,
                        ax_source_file_offset,
                        ax_source,
                        intervening_far_calls,
                        note: snd_entry_call_note(file_offset),
                    }
                })
            })
            .collect()
    }

    pub fn inspect(&self) -> Result<BloodPrgInspection> {
        Ok(BloodPrgInspection {
            summary: self.summary(),
            target_sha256: BLOODPRG_SHA256,
            known_symbols: KNOWN_SYMBOLS.to_vec(),
            opcode_handlers: self.opcode_handlers()?,
            opcode_descriptors: self.opcode_descriptors()?,
            vm_opcode_specs: self.vm_opcode_specs()?,
            snd_entry_call_sites: self.snd_entry_call_sites(),
            dialogue_font: self.dialogue_font_tables()?,
        })
    }

    fn slice(&self, file_offset: usize, len: usize, label: &str) -> Result<&[u8]> {
        let end = file_offset
            .checked_add(len)
            .ok_or_else(|| anyhow::anyhow!("{label} offset overflow"))?;
        self.data
            .get(file_offset..end)
            .ok_or_else(|| anyhow::anyhow!("{label} extends past BLOODPRG.EXE"))
    }

    fn find_ax_source_before(
        &self,
        call_file_offset: usize,
    ) -> (Option<u16>, Option<usize>, &'static str, u8) {
        let window_start = call_file_offset.saturating_sub(AX_SOURCE_SCAN_BACK);
        let mut source = (None, None, "unresolved", 0usize);
        for pos in window_start..call_file_offset {
            if pos + 3 <= call_file_offset && self.data.get(pos) == Some(&0xb8) {
                let lo = self.data[pos + 1];
                let hi = self.data[pos + 2];
                source = (
                    Some(u16::from_le_bytes([lo, hi])),
                    Some(pos),
                    "mov ax, imm16",
                    pos + 3,
                );
            } else if pos + 2 <= call_file_offset
                && self.data.get(pos..pos + 2) == Some(&[0x33, 0xc0])
            {
                source = (Some(0), Some(pos), "xor ax, ax", pos + 2);
            }
        }

        let intervening_far_calls = self.data[source.3..call_file_offset]
            .iter()
            .filter(|byte| **byte == FAR_CALL_OPCODE)
            .count()
            .min(u8::MAX as usize) as u8;

        (source.0, source.1, source.2, intervening_far_calls)
    }

    fn file_to_known_segoff(&self, file_offset: usize) -> (u16, u16) {
        let (segment, base) = KNOWN_CODE_SEGMENTS
            .iter()
            .copied()
            .take_while(|(_, base)| *base <= file_offset)
            .last()
            .unwrap_or((0, self.header.header_size()));
        (segment, (file_offset - base) as u16)
    }
}

fn u16_at(data: &[u8], offset: usize) -> Result<u16> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or_else(|| anyhow::anyhow!("missing u16 at file offset 0x{offset:05x}"))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

#[derive(Clone, Copy)]
struct OpcodeMetadata {
    mnemonic: &'static str,
    family: &'static str,
    rust_status: &'static str,
    notes: &'static str,
}

fn opcode_metadata(opcode: u8, handler_file_offset: usize) -> OpcodeMetadata {
    match opcode {
        0xa0 => OpcodeMetadata {
            mnemonic: "control_a0",
            family: "control-flow",
            rust_status: "execution-trace-ported",
            notes: "condition block start; Rust execute_trace models the A0 target stack used by branch helper 0x6462",
        },
        0xa1 => OpcodeMetadata {
            mnemonic: "control_a1",
            family: "control-flow",
            rust_status: "execution-trace-ported",
            notes: "condition block end; Rust execute_trace models the DOS stack-pop behavior while the token walker models length/mode effect",
        },
        0xa4 => OpcodeMetadata {
            mnemonic: "jump",
            family: "control-flow",
            rust_status: "execution-trace-ported",
            notes: "direct SI jump to the u16 operand; modeled by execute_trace",
        },
        0xa9 => OpcodeMetadata {
            mnemonic: "jump_or_condition_reset",
            family: "control-flow",
            rust_status: "execution-trace-ported",
            notes: "flagged direct jump / condition-stack reset; modeled by execute_trace for inspected script paths",
        },
        vm::OP_TEXT => OpcodeMetadata {
            mnemonic: "text",
            family: "dialogue-text",
            rust_status: "partially-ported",
            notes: "TEXT token shape, dictionary words, line state, and subtitle assembly rules are represented in Rust",
        },
        vm::OP_ACTOR => OpcodeMetadata {
            mnemonic: "actor_object_ref",
            family: "object-reference",
            rust_status: "partially-ported",
            notes: "binary handler consumes two u16 operands; current Rust line-state model tracks the talk/object reference needed for dialogue",
        },
        op if vm::is_record_state_opcode(op) => OpcodeMetadata {
            mnemonic: "record_state",
            family: "line-record",
            rust_status: "token-ported",
            notes: "C1/C2 line-record state operations; Rust exposes raw record and operand words while deeper table side effects remain under RE",
        },
        vm::OP_RECORD_LINK => OpcodeMetadata {
            mnemonic: "record_link",
            family: "line-record",
            rust_status: "token-ported",
            notes: "C3 line-record relation; Rust exposes both operands and deliberately does not treat it as a speaker marker",
        },
        op if vm::is_record_entry_opcode(op) => OpcodeMetadata {
            mnemonic: "record_entry",
            family: "line-record",
            rust_status: "token-ported",
            notes: "C5-C8 line-record entries; Rust exposes the raw operand and recovered stored related-record slot",
        },
        vm::OP_RECORD_CLEAR => OpcodeMetadata {
            mnemonic: "record_clear",
            family: "line-record",
            rust_status: "token-ported",
            notes: "C9 clears a 6-byte line record; Rust uses matching clears to stop actor/background context bleed",
        },
        vm::OP_GLOBAL_WORD_COMPARE => OpcodeMetadata {
            mnemonic: "global_word_compare",
            family: "global-condition",
            rust_status: "token-ported",
            notes: "CA compares a token u16 against gs:0x0aa6; Rust exposes the operands, runtime globals are not yet wired into execute_trace",
        },
        vm::OP_GLOBAL_PAIR_COMPARE => OpcodeMetadata {
            mnemonic: "global_pair_compare",
            family: "global-condition",
            rust_status: "token-ported",
            notes: "CB compares a packed token pair against gs:0x0aaa/0x0aa8; Rust exposes operands while runtime globals remain pending",
        },
        _ => match handler_file_offset {
            0x006863 => OpcodeMetadata {
                mnemonic: "state_assign_or_signed_compare",
                family: "state-assign-compare",
                rust_status: "execution-trace-ported",
                notes: "B1/B4/B5/B6/BE/BF/C0 family; Rust applies mode0 mutations and execute_trace evaluates mode1 signed comparisons through the A0/A1 branch stack",
            },
            0x006902 => OpcodeMetadata {
                mnemonic: "bitmask_set_or_test",
                family: "bitmask-set-test",
                rust_status: "execution-trace-ported",
                notes: "AE/B0 family; Rust applies mode0 bit set/clear mutations and execute_trace evaluates mode1 bit tests with optional A1 inversion",
            },
            0x006946 => OpcodeMetadata {
                mnemonic: "equality_assign_or_test",
                family: "equality-assign",
                rust_status: "execution-trace-ported",
                notes: "AD/AF/B2/B3/BA/BB/BC family; Rust applies mode0 assignments and execute_trace evaluates mode1 equality/inversion except the gs:0x674e-to-0xffff RHS remap is still pending",
            },
            0x006aa7 => OpcodeMetadata {
                mnemonic: "bit_set_or_test",
                family: "bit-set-test",
                rust_status: "execution-trace-ported",
                notes: "B7 high-bit-first byte flag set/clear/test family; Rust applies mode0 mutations and execute_trace evaluates mode1 tests with optional A1 inversion",
            },
            0x006b06 => OpcodeMetadata {
                mnemonic: "pair_record_assign_or_compare",
                family: "pair-record",
                rust_status: "not-ported",
                notes: "B8/B9/BD pair-record assignment and comparison family",
            },
            0x0053a0 => OpcodeMetadata {
                mnemonic: "segment_entry_or_noop",
                family: "control-flow",
                rust_status: "not-ported",
                notes: "D3 handler points at the VM segment base; variable token skip is modeled by token_advance",
            },
            _ => OpcodeMetadata {
                mnemonic: "unclassified_handler",
                family: "unclassified",
                rust_status: "not-ported",
                notes: "handler entry is mapped from BLOODPRG.EXE, but semantics are not yet named",
            },
        },
    }
}

const KNOWN_CODE_SEGMENTS: &[(u16, usize)] = &[
    (0x0000, 0x000600),
    (0x008b, 0x000eb0),
    (0x0299, 0x002f90),
    (0x04da, 0x0053a0),
    (0x071e, 0x0077e0),
    (0x0971, 0x009d10),
    (0x0a9a, 0x00afa0),
    (0x0b1b, 0x00b7b0),
];

fn snd_entry_call_note(file_offset: usize) -> &'static str {
    match file_offset {
        0x005d71 => "VM/presentation C4 handoff sound; constant clip 6",
        0x007a2a => "presentation state start sound; constant clip 1",
        0x007bf8 => "presentation state step sound; AX=1 carried across setup call",
        0x007f67 => "presentation/UI state sound; constant clip 5",
        0x00804e => "presentation/UI state sound; constant clip 5",
        0x0080e5 => "presentation/UI state sound; constant clip 3",
        0x00815b => "presentation/UI state sound; constant clip 5",
        0x008235 => "C4 actor/object transition sound; constant clip 2",
        0x008534 => "text/presentation render path sound; constant clip 0",
        0x0086ec => "presentation transition sound; constant clip 4",
        _ => "unclassified SND entry call",
    }
}

pub const KNOWN_SYMBOLS: &[BinarySymbol] = &[
    BinarySymbol {
        name: "entry",
        file_offset: 0x000600,
        segment: Some(0x0000),
        offset: Some(0x0000),
        ds_offset: None,
        kind: "startup",
        comment: "MZ entry point; initializes segment registers before main startup",
    },
    BinarySymbol {
        name: "vm_exec_loop",
        file_offset: 0x0055f5,
        segment: Some(0x04da),
        offset: Some(0x0255),
        ds_offset: None,
        kind: "script-vm",
        comment: "main script interpreter loop; dispatches opcodes A0..D3 through the handler table",
    },
    BinarySymbol {
        name: "vm_dispatch",
        file_offset: 0x005627,
        segment: Some(0x04da),
        offset: Some(0x0287),
        ds_offset: None,
        kind: "script-vm",
        comment: "call gs:[0x6eb0 + (opcode - 0xa0) * 2]",
    },
    BinarySymbol {
        name: "vm_handler_table",
        file_offset: OPCODE_HANDLER_TABLE_FILE_OFFSET,
        segment: None,
        offset: None,
        ds_offset: Some(0x6eb0),
        kind: "data",
        comment: "52 near handler offsets for opcodes A0..D3, dispatched in VM segment 0x04da",
    },
    BinarySymbol {
        name: "vm_token_special",
        file_offset: 0x006293,
        segment: Some(VM_CODE_SEGMENT),
        offset: Some(0x0ef3),
        ds_offset: None,
        kind: "script-vm",
        comment: "zero-word variable-token skip helper for A8/AC/CC/D3",
    },
    BinarySymbol {
        name: "vm_token_advance",
        file_offset: 0x0062b6,
        segment: Some(0x04da),
        offset: Some(0x0f16),
        ds_offset: None,
        kind: "script-vm",
        comment: "decode and skip one compiled-BASIC token using DS:0x6f18 descriptor table",
    },
    BinarySymbol {
        name: "vm_branch_fail",
        file_offset: 0x006462,
        segment: Some(VM_CODE_SEGMENT),
        offset: Some(0x10c2),
        ds_offset: None,
        kind: "script-vm",
        comment: "branch/control helper used by conditional handlers on failed tests",
    },
    BinarySymbol {
        name: "vm_op_a6_text",
        file_offset: 0x00660c,
        segment: Some(0x04da),
        offset: Some(0x126c),
        ds_offset: None,
        kind: "script-vm",
        comment: "TEXT token handler; consumes line index, selector, flags, loop target, and dictionary words",
    },
    BinarySymbol {
        name: "vm_op_a6_assemble_text",
        file_offset: 0x0066cd,
        segment: Some(VM_CODE_SEGMENT),
        offset: Some(0x132d),
        ds_offset: None,
        kind: "script-vm",
        comment: "TEXT dictionary-word assembly with punctuation-aware spacing and 35-character wrapping",
    },
    BinarySymbol {
        name: "vm_assign_compare_6863",
        file_offset: 0x006863,
        segment: Some(VM_CODE_SEGMENT),
        offset: Some(0x14c3),
        ds_offset: None,
        kind: "script-vm",
        comment: "B1/B4/B5/B6/BE/BF/C0 assignment and signed-comparison family",
    },
    BinarySymbol {
        name: "vm_bitmask_6902",
        file_offset: 0x006902,
        segment: Some(VM_CODE_SEGMENT),
        offset: Some(0x1562),
        ds_offset: None,
        kind: "script-vm",
        comment: "AE/B0 bitmask set/test family",
    },
    BinarySymbol {
        name: "vm_equality_assign_6946",
        file_offset: 0x006946,
        segment: Some(VM_CODE_SEGMENT),
        offset: Some(0x15a6),
        ds_offset: None,
        kind: "script-vm",
        comment: "AD/AF/B2/B3/BA/BB/BC equality-test and assignment family",
    },
    BinarySymbol {
        name: "vm_bit_set_test_6aa7",
        file_offset: 0x006aa7,
        segment: Some(VM_CODE_SEGMENT),
        offset: Some(0x1707),
        ds_offset: None,
        kind: "script-vm",
        comment: "B7 high-bit-first byte flag set/clear/test family",
    },
    BinarySymbol {
        name: "vm_pair_record_6b06",
        file_offset: 0x006b06,
        segment: Some(VM_CODE_SEGMENT),
        offset: Some(0x1766),
        ds_offset: None,
        kind: "script-vm",
        comment: "B8/B9/BD pair-record assignment and comparison family",
    },
    BinarySymbol {
        name: "vm_op_c1_record_state",
        file_offset: 0x006b4c,
        segment: Some(0x04da),
        offset: Some(0x17ac),
        ds_offset: None,
        kind: "script-vm",
        comment: "C1 line-record state handler; consumes record+operand words and may write/test {0xc1, operand, 2}",
    },
    BinarySymbol {
        name: "vm_op_c4_actor",
        file_offset: 0x006c7e,
        segment: Some(0x04da),
        offset: Some(0x18de),
        ds_offset: None,
        kind: "script-vm",
        comment: "actor/record handler; consumes record+related u16 operands and writes a 6-byte record entry",
    },
    BinarySymbol {
        name: "vm_op_c2_record_state",
        file_offset: 0x006e34,
        segment: Some(0x04da),
        offset: Some(0x1a94),
        ds_offset: None,
        kind: "script-vm",
        comment: "C2 line-record state handler; consumes record+operand words and can drive special active-line ids",
    },
    BinarySymbol {
        name: "vm_op_c3_record_link",
        file_offset: 0x006eee,
        segment: Some(0x04da),
        offset: Some(0x1b4e),
        ds_offset: None,
        kind: "script-vm",
        comment: "record-link handler; consumes record+related u16 operands and writes {0xc3, related, 1}",
    },
    BinarySymbol {
        name: "vm_op_c5_record_entry",
        file_offset: 0x006d18,
        segment: Some(0x04da),
        offset: Some(0x1978),
        ds_offset: None,
        kind: "script-vm",
        comment: "record-entry handler; writes {0xc5, related, 0} when related type is 0x0200",
    },
    BinarySymbol {
        name: "vm_op_c6_record_entry",
        file_offset: 0x006d80,
        segment: Some(0x04da),
        offset: Some(0x19e0),
        ds_offset: None,
        kind: "script-vm",
        comment: "record-entry handler; writes {0xc6, operand, 0}",
    },
    BinarySymbol {
        name: "vm_op_c7_record_entry",
        file_offset: 0x006dcf,
        segment: Some(0x04da),
        offset: Some(0x1a2f),
        ds_offset: None,
        kind: "script-vm",
        comment: "record-entry handler; writes {0xc7, related, 0} when related record is active",
    },
    BinarySymbol {
        name: "vm_op_c8_record_entry",
        file_offset: 0x006f62,
        segment: Some(0x04da),
        offset: Some(0x1bc2),
        ds_offset: None,
        kind: "script-vm",
        comment: "record-entry handler; consumes operand but writes {0xc8, 0, 0} when record is empty",
    },
    BinarySymbol {
        name: "vm_op_c9_record_clear",
        file_offset: 0x006fb9,
        segment: Some(0x04da),
        offset: Some(0x1c19),
        ds_offset: None,
        kind: "script-vm",
        comment: "record-clear handler; zeros a 6-byte record and clears related 0xc4 actor subrecord",
    },
    BinarySymbol {
        name: "vm_op_ca_global_word_compare",
        file_offset: 0x0064e5,
        segment: Some(0x04da),
        offset: Some(0x1145),
        ds_offset: None,
        kind: "script-vm",
        comment: "CA global condition handler; compares token value to gs:0x0aa6",
    },
    BinarySymbol {
        name: "vm_op_cb_global_pair_compare",
        file_offset: 0x006510,
        segment: Some(0x04da),
        offset: Some(0x1170),
        ds_offset: None,
        kind: "script-vm",
        comment: "CB global pair condition handler; compares packed token value to gs:0x0aaa/0x0aa8",
    },
    BinarySymbol {
        name: "render_string_entry",
        file_offset: 0x003192,
        segment: Some(0x0299),
        offset: Some(0x0202),
        ds_offset: None,
        kind: "presentation",
        comment: "dialogue string renderer entry; BX=x, DX=y, SI=ASCIIZ string, DL=color",
    },
    BinarySymbol {
        name: "render_string_glyph_loop",
        file_offset: 0x0031c8,
        segment: Some(0x0299),
        offset: Some(0x0238),
        ds_offset: None,
        kind: "presentation",
        comment: "dialogue glyph blitter using the embedded ASCII map, advances, and 8-byte glyph rows",
    },
    BinarySymbol {
        name: "scene_band_fill",
        file_offset: 0x003d7b,
        segment: Some(0x0299),
        offset: Some(0x0deb),
        ds_offset: None,
        kind: "presentation",
        comment: "fills framebuffer band using y-clip bounds DS:0x5239..0x523b and base DS:0x5221",
    },
    BinarySymbol {
        name: "dlg_line_activate",
        file_offset: 0x0011e8,
        segment: Some(0x008b),
        offset: Some(0x0338),
        ds_offset: None,
        kind: "presentation",
        comment: "stores active dialogue line DS:0x6788 = signed DS:0x1fab + 9",
    },
    BinarySymbol {
        name: "dlg_chatter_hold_consume",
        file_offset: 0x00115d,
        segment: Some(0x008b),
        offset: Some(0x02ad),
        ds_offset: None,
        kind: "presentation",
        comment: "tests and clears DS:0x67bb line-complete hold flag",
    },
    BinarySymbol {
        name: "dlg_clear_a",
        file_offset: 0x001a5e,
        segment: Some(0x008b),
        offset: Some(0x0bae),
        ds_offset: None,
        kind: "presentation",
        comment: "dialogue/scene clear path; resets active-line state and calls common stop routine",
    },
    BinarySymbol {
        name: "dlg_reveal_update",
        file_offset: 0x0093f8,
        segment: Some(0x071e),
        offset: Some(0x1c18),
        ds_offset: None,
        kind: "presentation",
        comment: "animated subtitle reveal updater; advances visible-text pointer and countdown",
    },
    BinarySymbol {
        name: "dlg_reveal_complete_hold",
        file_offset: 0x0094ba,
        segment: Some(0x071e),
        offset: Some(0x1cda),
        ds_offset: None,
        kind: "presentation",
        comment: "when reveal reaches NUL, sets DS:0x0b35=DS:0x0aca*4 and DS:0x67bb=1",
    },
    BinarySymbol {
        name: "dlg_frame_update",
        file_offset: 0x009e81,
        segment: Some(0x0971),
        offset: Some(0x0171),
        ds_offset: None,
        kind: "presentation",
        comment: "per-frame dialogue display updater keyed by active line DS:0x6788",
    },
    BinarySymbol {
        name: "dlg_clear_b",
        file_offset: 0x00b521,
        segment: Some(0x0a9a),
        offset: Some(0x0581),
        ds_offset: None,
        kind: "presentation",
        comment: "dialogue/scene clear-state variant",
    },
    BinarySymbol {
        name: "snd_scene_cleanup",
        file_offset: 0x0012e8,
        segment: Some(0x008b),
        offset: Some(0x0438),
        ds_offset: None,
        kind: "audio",
        comment: "close/delete per-scene son.snd and mus.snd temp files",
    },
    BinarySymbol {
        name: "dlg_record_end_hold",
        file_offset: 0x0072a8,
        segment: Some(0x04da),
        offset: Some(0x1f08),
        ds_offset: None,
        kind: "presentation",
        comment: "record iteration end sets DS:0x0b35=DS:0x27cf*(DS:0x0aca/2)+6 and DS:0x67bb=1",
    },
    BinarySymbol {
        name: "snd_entry",
        file_offset: 0x00b8cd,
        segment: Some(0x0b1b),
        offset: Some(0x011d),
        ds_offset: None,
        kind: "audio",
        comment: "SND subsystem entry reached before clip playback",
    },
    BinarySymbol {
        name: "snd_clip_player",
        file_offset: 0x00b9de,
        segment: Some(0x0b1b),
        offset: Some(0x022e),
        ds_offset: None,
        kind: "audio",
        comment: "SND clip player; AX is the clip index into the in-memory clip table",
    },
    BinarySymbol {
        name: "son_snd_name",
        file_offset: 0x00d4c6,
        segment: None,
        offset: None,
        ds_offset: Some(0x00a6),
        kind: "data",
        comment: "son.snd temp voice/SFX bank filename",
    },
    BinarySymbol {
        name: "mus_snd_name",
        file_offset: 0x00d4ce,
        segment: None,
        offset: None,
        ds_offset: Some(0x00ae),
        kind: "data",
        comment: "mus.snd temp music bank filename",
    },
    BinarySymbol {
        name: "vm_opcode_lengths",
        file_offset: OPCODE_LENGTH_TABLE_FILE_OFFSET,
        segment: None,
        offset: None,
        ds_offset: Some(0x6f18),
        kind: "data",
        comment: "two-byte descriptors for opcodes A0..D3",
    },
    BinarySymbol {
        name: "font_ascii_map",
        file_offset: DIALOGUE_FONT_ASCII_MAP_FILE_OFFSET,
        segment: None,
        offset: None,
        ds_offset: Some(0x7802),
        kind: "data",
        comment: "dialogue font ASCII-to-glyph-index map",
    },
    BinarySymbol {
        name: "font_advances",
        file_offset: DIALOGUE_FONT_ADVANCES_FILE_OFFSET,
        segment: None,
        offset: None,
        ds_offset: Some(0x78b2),
        kind: "data",
        comment: "dialogue font per-glyph advance widths",
    },
    BinarySymbol {
        name: "font_glyphs",
        file_offset: DIALOGUE_FONT_GLYPHS_FILE_OFFSET,
        segment: None,
        offset: None,
        ds_offset: Some(0x7908),
        kind: "data",
        comment: "dialogue font glyph rows; 86 glyphs, 8 bytes each",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Option<BloodPrg> {
        for path in ["re/bin/BLOODPRG.EXE", "../re/bin/BLOODPRG.EXE"] {
            if let Ok(binary) = BloodPrg::parse_file(path) {
                return Some(binary);
            }
        }
        None
    }

    #[test]
    fn parses_mz_header_and_address_conversions() {
        let Some(binary) = fixture() else {
            eprintln!("skipping: BLOODPRG.EXE not available");
            return;
        };
        let summary = binary.summary();
        assert_eq!(summary.file_size, BLOODPRG_FILE_SIZE);
        assert_eq!(summary.header_size, 0x600);
        assert_eq!(summary.load_size, 0x14c98);
        assert_eq!(summary.entry_file_offset, 0x600);
        assert_eq!(summary.trailing_bytes, 0);
        assert_eq!(binary.ds_to_file(0x6f18), OPCODE_LENGTH_TABLE_FILE_OFFSET);
        assert_eq!(
            binary.ds_to_file(0x7802),
            DIALOGUE_FONT_ASCII_MAP_FILE_OFFSET
        );
        assert_eq!(
            binary.ds_to_file(0x78b2),
            DIALOGUE_FONT_ADVANCES_FILE_OFFSET
        );
        assert_eq!(binary.ds_to_file(0x7908), DIALOGUE_FONT_GLYPHS_FILE_OFFSET);
    }

    #[test]
    fn opcode_table_matches_decompiled_vm_constants() {
        let Some(binary) = fixture() else {
            eprintln!("skipping: BLOODPRG.EXE not available");
            return;
        };
        let descriptors = binary.opcode_descriptors().expect("opcode table");
        assert_eq!(descriptors.len(), vm::OPCODE_DESC.len());
        for (idx, descriptor) in descriptors.iter().enumerate() {
            let (mode0, mode1) = vm::OPCODE_DESC[idx];
            assert_eq!(descriptor.opcode, vm::OP_MIN + idx as u8);
            assert_eq!(
                descriptor.len_mode0, mode0,
                "opcode {:02x}",
                descriptor.opcode
            );
            assert_eq!(
                descriptor.len_mode1_or_sentinel, mode1,
                "opcode {:02x}",
                descriptor.opcode
            );
        }
    }

    #[test]
    fn handler_table_resolves_known_opcode_entry_points() {
        let Some(binary) = fixture() else {
            eprintln!("skipping: BLOODPRG.EXE not available");
            return;
        };
        let handlers = binary.opcode_handlers().expect("handler table");
        assert_eq!(handlers.len(), vm::OPCODE_DESC.len());

        let text = handlers
            .iter()
            .find(|handler| handler.opcode == vm::OP_TEXT)
            .expect("A6 handler");
        assert_eq!(text.handler_offset, 0x126c);
        assert_eq!(text.handler_file_offset, 0x00660c);

        let actor = handlers
            .iter()
            .find(|handler| handler.opcode == vm::OP_ACTOR)
            .expect("C4 handler");
        assert_eq!(actor.handler_offset, 0x18de);
        assert_eq!(actor.handler_file_offset, 0x006c7e);
    }

    #[test]
    fn opcode_specs_name_known_handler_families() {
        let Some(binary) = fixture() else {
            eprintln!("skipping: BLOODPRG.EXE not available");
            return;
        };
        let specs = binary.vm_opcode_specs().expect("opcode specs");
        assert_eq!(specs.len(), vm::OPCODE_DESC.len());

        let text = specs
            .iter()
            .find(|spec| spec.opcode == vm::OP_TEXT)
            .unwrap();
        assert_eq!(text.mnemonic, "text");
        assert_eq!(text.family, "dialogue-text");
        assert_eq!(text.handler_file_offset, 0x00660c);
        assert!(text.variable_length);

        let actor = specs
            .iter()
            .find(|spec| spec.opcode == vm::OP_ACTOR)
            .unwrap();
        assert_eq!(actor.mnemonic, "actor_object_ref");
        assert_eq!(actor.family, "object-reference");
        assert_eq!(actor.handler_file_offset, 0x006c7e);
        assert!(actor.mode_control);

        let assign = specs.iter().find(|spec| spec.opcode == 0xb1).unwrap();
        assert_eq!(assign.family, "state-assign-compare");
        assert_eq!(assign.handler_file_offset, 0x006863);
        assert_eq!(assign.len_mode0, 7);
        assert!(!assign.mode_control);

        let equality = specs.iter().find(|spec| spec.opcode == 0xaf).unwrap();
        assert_eq!(equality.family, "equality-assign");
        assert_eq!(equality.handler_file_offset, 0x006946);
        assert!(equality.mode_control);
    }

    #[test]
    fn known_symbols_have_consistent_addresses() {
        let Some(binary) = fixture() else {
            eprintln!("skipping: BLOODPRG.EXE not available");
            return;
        };
        for symbol in KNOWN_SYMBOLS {
            if let (Some(segment), Some(offset)) = (symbol.segment, symbol.offset) {
                assert_eq!(
                    binary.segoff_to_file(segment, offset),
                    symbol.file_offset,
                    "{}",
                    symbol.name
                );
            }
            if let Some(ds_offset) = symbol.ds_offset {
                assert_eq!(
                    binary.ds_to_file(ds_offset),
                    symbol.file_offset,
                    "{}",
                    symbol.name
                );
            }
        }
    }

    #[test]
    fn snd_entry_call_sites_recover_constant_ax_indices() {
        let Some(binary) = fixture() else {
            eprintln!("skipping: BLOODPRG.EXE not available");
            return;
        };
        let sites = binary.snd_entry_call_sites();
        let got: Vec<_> = sites
            .iter()
            .map(|site| {
                (
                    site.file_offset,
                    site.segment,
                    site.offset,
                    site.ax_value,
                    site.intervening_far_calls,
                )
            })
            .collect();
        assert_eq!(
            got,
            vec![
                (0x005d71, 0x04da, 0x09d1, Some(6), 0),
                (0x007a2a, 0x071e, 0x024a, Some(1), 0),
                (0x007bf8, 0x071e, 0x0418, Some(1), 1),
                (0x007f67, 0x071e, 0x0787, Some(5), 0),
                (0x00804e, 0x071e, 0x086e, Some(5), 0),
                (0x0080e5, 0x071e, 0x0905, Some(3), 0),
                (0x00815b, 0x071e, 0x097b, Some(5), 0),
                (0x008235, 0x071e, 0x0a55, Some(2), 0),
                (0x008534, 0x071e, 0x0d54, Some(0), 0),
                (0x0086ec, 0x071e, 0x0f0c, Some(4), 0),
            ]
        );

        let text_sound = sites
            .iter()
            .find(|site| site.file_offset == 0x008534)
            .expect("text/presentation SND call");
        assert_eq!(text_sound.ax_source, "xor ax, ax");
        assert!(text_sound.note.contains("constant clip 0"));
    }

    #[test]
    fn extracts_dialogue_font_tables_from_binary() {
        let Some(binary) = fixture() else {
            eprintln!("skipping: BLOODPRG.EXE not available");
            return;
        };
        let font = binary.dialogue_font_tables().expect("font tables");
        assert_eq!(font.ascii_map.len(), DIALOGUE_FONT_ASCII_MAP_LEN);
        assert_eq!(font.advances.len(), DIALOGUE_FONT_GLYPH_COUNT);
        assert_eq!(font.glyph_rows.len(), DIALOGUE_FONT_GLYPH_COUNT);

        let glyph_a = font.ascii_map[b'A' as usize] as usize;
        let glyph_m = font.ascii_map[b'M' as usize] as usize;
        let glyph_e = font.ascii_map[b'e' as usize] as usize;
        assert_eq!(glyph_a, 0);
        assert_eq!(font.advances[glyph_m], 0x0a);
        assert_eq!(font.advances[glyph_e], 0x08);
        assert_eq!(
            font.glyph_rows[glyph_a],
            [0x00, 0x7e, 0x82, 0x82, 0x82, 0xfe, 0x82, 0x00]
        );
    }
}
