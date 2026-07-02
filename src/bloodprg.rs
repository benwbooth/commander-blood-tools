use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
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
    pub dialogue_font: DialogueFontTables,
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

    pub fn inspect(&self) -> Result<BloodPrgInspection> {
        Ok(BloodPrgInspection {
            summary: self.summary(),
            target_sha256: BLOODPRG_SHA256,
            known_symbols: KNOWN_SYMBOLS.to_vec(),
            opcode_handlers: self.opcode_handlers()?,
            opcode_descriptors: self.opcode_descriptors()?,
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
}

fn u16_at(data: &[u8], offset: usize) -> Result<u16> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or_else(|| anyhow::anyhow!("missing u16 at file offset 0x{offset:05x}"))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
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
        comment: "B7 bit set/test family",
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
        name: "vm_op_c4_actor",
        file_offset: 0x006c7e,
        segment: Some(0x04da),
        offset: Some(0x18de),
        ds_offset: None,
        kind: "script-vm",
        comment: "actor/object reference handler",
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
        comment: "stores active dialogue line DS:0x6788 = DS:0x1fab + 9",
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
