use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::vm;

pub const BLOODPRG_FILE_SIZE: usize = 86_680;
pub const BLOODPRG_SHA256: &str =
    "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823";
pub const VM_CODE_SEGMENT: u16 = 0x04da;
pub const FS_SEGMENT: u16 = 0x0bbf;
pub const DATA_SEGMENT: u16 = 0x0ce2;
pub const OPCODE_HANDLER_TABLE_FILE_OFFSET: usize = 0x142d0;
pub const OPCODE_LENGTH_TABLE_FILE_OFFSET: usize = 0x14338;
pub const RESOURCE_NAME_TABLE_FS_OFFSET: u16 = 0x0c04;
pub const RESOURCE_NAME_TABLE_FILE_OFFSET: usize = 0x0cdf4;
pub const RESOURCE_NAME_ENTRY_LEN: usize = 16;
pub const SCRIPT_RESOURCE_PROFILE_TABLE_FS_OFFSET: u16 = 0x11f4;
pub const SCRIPT_RESOURCE_PROFILE_TABLE_FILE_OFFSET: usize = 0x0d3e4;
pub const SCRIPT_RESOURCE_PROFILE_COUNT: usize = 5;
pub const SCRIPT_RESOURCE_PROFILE_SLOT_COUNT: usize = 5;
pub const SCRIPT_RESOURCE_PROFILE_STRIDE: usize = SCRIPT_RESOURCE_PROFILE_SLOT_COUNT * 2;
pub const DIALOGUE_FONT_ASCII_MAP_FILE_OFFSET: usize = 0x14c22;
pub const DIALOGUE_FONT_ADVANCES_FILE_OFFSET: usize = 0x14cd2;
pub const DIALOGUE_FONT_GLYPHS_FILE_OFFSET: usize = 0x14d28;
pub const DIALOGUE_FONT_ASCII_MAP_LEN: usize = 128;
pub const DIALOGUE_FONT_GLYPH_COUNT: usize = 86;
pub const DIALOGUE_FONT_GLYPH_HEIGHT: usize = 8;
pub const SND_ENTRY_SEGMENT: u16 = 0x0b1b;
pub const SND_ENTRY_OFFSET: u16 = 0x011d;
pub const SND_BANK_LOAD_SEGMENT: u16 = 0x0b1b;
pub const SND_BANK_LOAD_OFFSET: u16 = 0x0855;
pub const RENDER_SEGMENT: u16 = 0x0299;
pub const RENDER_VGA_DAC_PALETTE_LOAD_OFFSET: u16 = 0x0000;
pub const RENDER_VGA_DAC_PALETTE_CLEAR_OFFSET: u16 = 0x0016;
pub const RENDER_FIXED_8X8_TEXT_OFFSET: u16 = 0x00d6;
pub const RENDER_FONT_STRING_WIDTH_OFFSET: u16 = 0x013d;
pub const RENDER_UI_TEXT_OFFSET: u16 = 0x0176;
pub const RENDER_STRING_OFFSET: u16 = 0x0202;
pub const RENDER_FRAMEBUFFER_RECT_REMAP_OFFSET: u16 = 0x040e;
pub const RENDER_PLANAR_UI_TEXT_OFFSET: u16 = 0x0498;
pub const RENDER_PLANAR_DIALOGUE_TEXT_OFFSET: u16 = 0x05de;
pub const RENDER_SUBTITLE_REVEAL_OFFSET: u16 = 0x06a0;
pub const RENDER_SMALL_TEXT_OFFSET: u16 = 0x075a;
pub const RENDER_PLANAR_HORIZONTAL_LINE_OFFSET: u16 = 0x0a2b;
pub const RENDER_PLANAR_VERTICAL_LINE_OFFSET: u16 = 0x0b23;
pub const RENDER_RECT_OUTLINE_OFFSET: u16 = 0x0bb5;
pub const RENDER_DITHER_RECT_FILL_OFFSET: u16 = 0x0bf5;
pub const RENDER_RECT_FILL_OFFSET: u16 = 0x0cdc;
pub const RENDER_SCENE_BAND_FILL_OFFSET: u16 = 0x0deb;
pub const RENDER_SECONDARY_BAND_FILL_OFFSET: u16 = 0x0e2f;
pub const RENDER_FRAMEBUFFER_COPY_OFFSET: u16 = 0x0eb6;
pub const RENDER_SECONDARY_FRAMEBUFFER_COPY_OFFSET: u16 = 0x0ecb;
pub const RENDER_VGA_PLANAR_TO_LINEAR_COPY_OFFSET: u16 = 0x0ee0;
pub const RENDER_PLANAR_COPY_OFFSET: u16 = 0x0f3e;
pub const RENDER_RESOURCE_FILE_LOAD_OFFSET: u16 = 0x1037;
pub const RENDER_SPRITE_SLOT_LOAD_OFFSET: u16 = 0x11be;
pub const RENDER_SPRITE_SLOT_STATE_OFFSET: u16 = 0x1241;
pub const RENDER_SPRITE_SLOT_RESOURCE_LOAD_OFFSET: u16 = 0x1140;
pub const RENDER_SPRITE_SLOT_POSITION_OFFSET: u16 = 0x127d;
pub const RENDER_SPRITE_SLOT_RANGE_DIRTY_OFFSET: u16 = 0x12b0;
pub const RENDER_SPRITE_SLOT_EXTENT_OFFSET: u16 = 0x133d;
pub const RENDER_SPRITE_SLOT_COMMIT_RANGE_OFFSET: u16 = 0x1467;
pub const RENDER_SPRITE_SLOT_DIRTY_RANGE_RENDER_OFFSET: u16 = 0x14e1;
pub const RENDER_SPRITE_BLITTER_TABLE_OFFSET: u16 = 0x1592;
pub const RENDER_SPRITE_BLITTER_TABLE_FILE_OFFSET: usize = 0x004522;
pub const RENDER_SPRITE_BLITTER_ENTRY_COUNT: usize = 8;
pub const RENDER_SPRITE_BLIT_RAW_TRANSPARENT_OFFSET: u16 = 0x15a6;
pub const RENDER_SPRITE_BLIT_RLE_TRANSPARENT_OFFSET: u16 = 0x172c;
pub const RENDER_SPRITE_BLIT_RAW_OPAQUE_OFFSET: u16 = 0x1c18;
pub const RENDER_SPRITE_BLIT_RLE_OPAQUE_OFFSET: u16 = 0x1d46;
pub const RENDER_SPRITE_BLIT_SCALED_TRANSPARENT_OFFSET: u16 = 0x1fd2;
pub const RENDER_SPRITE_BLIT_NOOP_5_OFFSET: u16 = 0x210a;
pub const RENDER_SPRITE_BLIT_NOOP_6_OFFSET: u16 = 0x210b;
pub const RENDER_SPRITE_BLIT_NOOP_7_OFFSET: u16 = 0x210c;
pub const RENDER_DIRTY_RECTS_COPY_OFFSET: u16 = 0x210d;
pub const NAV_CODE_SEGMENT: u16 = 0x071e;
pub const NAV_ACTOR_SUBDISPATCH_TABLE_FILE_OFFSET: usize = 0x007eb4;
pub const NAV_ACTOR_SUBDISPATCH_ENTRY_COUNT: usize = 6;
pub const NAV_CHOICE_SUBDISPATCH_TABLE_FILE_OFFSET: usize = 0x008709;
pub const NAV_CHOICE_SUBDISPATCH_ENTRY_COUNT: usize = 5;

const SND_ENTRY_FAR_CALL: [u8; 5] = [
    0x9a,
    SND_ENTRY_OFFSET as u8,
    (SND_ENTRY_OFFSET >> 8) as u8,
    SND_ENTRY_SEGMENT as u8,
    (SND_ENTRY_SEGMENT >> 8) as u8,
];
const SND_BANK_LOAD_FAR_CALL: [u8; 5] = [
    0x9a,
    SND_BANK_LOAD_OFFSET as u8,
    (SND_BANK_LOAD_OFFSET >> 8) as u8,
    SND_BANK_LOAD_SEGMENT as u8,
    (SND_BANK_LOAD_SEGMENT >> 8) as u8,
];
const FAR_CALL_OPCODE: u8 = 0x9a;
const REGISTER_SOURCE_SCAN_BACK: usize = 32;
const DS_STRING_SCAN_MAX: usize = 64;
const RENDER_FAR_CALL_SEGMENT_BYTES: [u8; 2] =
    [(RENDER_SEGMENT & 0x00ff) as u8, (RENDER_SEGMENT >> 8) as u8];

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct SubdispatchEntry {
    pub index: usize,
    pub handler_offset: u16,
    pub handler_file_offset: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct SpriteBlitterDispatchEntry {
    pub mode: u8,
    pub handler_offset: u16,
    pub handler_file_offset: usize,
    pub name: &'static str,
    pub note: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ScriptResourceProfileSlot {
    pub slot: usize,
    pub resource_id: u16,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ScriptResourceProfile {
    pub profile_index: u8,
    pub d2_operand: u8,
    pub script_number: u8,
    pub slots: Vec<ScriptResourceProfileSlot>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BloodPrgInspection {
    pub summary: MzSummary,
    pub target_sha256: &'static str,
    pub known_symbols: Vec<BinarySymbol>,
    pub opcode_handlers: Vec<OpcodeHandler>,
    pub opcode_descriptors: Vec<OpcodeDescriptor>,
    pub vm_opcode_specs: Vec<VmOpcodeSpec>,
    pub nav_actor_subdispatch_handlers: Vec<SubdispatchEntry>,
    pub nav_choice_subdispatch_handlers: Vec<SubdispatchEntry>,
    pub snd_entry_call_sites: Vec<SndEntryCallSite>,
    pub snd_bank_load_call_sites: Vec<SndBankLoadCallSite>,
    pub render_call_sites: Vec<RenderCallSite>,
    pub sprite_blitter_dispatch: Vec<SpriteBlitterDispatchEntry>,
    pub script_resource_profiles: Vec<ScriptResourceProfile>,
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SndBankLoadCallSite {
    pub file_offset: usize,
    pub segment: u16,
    pub offset: u16,
    pub target_segment: u16,
    pub target_offset: u16,
    pub ax_value: Option<u16>,
    pub ax_source_file_offset: Option<usize>,
    pub ax_source: &'static str,
    pub ax_intervening_far_calls: u8,
    pub si_value: Option<u16>,
    pub si_source_file_offset: Option<usize>,
    pub si_source: &'static str,
    pub si_intervening_far_calls: u8,
    pub path_file_offset: Option<usize>,
    pub path: Option<String>,
    pub mode: &'static str,
    pub note: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RenderCallSite {
    pub file_offset: usize,
    pub segment: u16,
    pub offset: u16,
    pub target_segment: u16,
    pub target_offset: u16,
    pub target_file_offset: usize,
    pub target_name: &'static str,
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

    pub fn fs_to_file(&self, fs_offset: u16) -> usize {
        self.segoff_to_file(FS_SEGMENT, fs_offset)
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

    pub fn script_resource_profiles(&self) -> Result<Vec<ScriptResourceProfile>> {
        let table_file_offset = self.fs_to_file(SCRIPT_RESOURCE_PROFILE_TABLE_FS_OFFSET);
        (0..SCRIPT_RESOURCE_PROFILE_COUNT)
            .map(|profile_index| {
                let row_offset = table_file_offset + profile_index * SCRIPT_RESOURCE_PROFILE_STRIDE;
                let mut slots = Vec::with_capacity(SCRIPT_RESOURCE_PROFILE_SLOT_COUNT);
                for slot in 0..SCRIPT_RESOURCE_PROFILE_SLOT_COUNT {
                    let resource_id = u16_at(&self.data, row_offset + slot * 2)?;
                    slots.push(ScriptResourceProfileSlot {
                        slot,
                        resource_id,
                        name: self.resource_name(resource_id)?,
                    });
                }
                let script_number = profile_index as u8 + 1;
                Ok(ScriptResourceProfile {
                    profile_index: profile_index as u8,
                    d2_operand: script_number,
                    script_number,
                    slots,
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

    pub fn nav_actor_subdispatch_handlers(&self) -> Result<Vec<SubdispatchEntry>> {
        self.subdispatch_entries(
            NAV_ACTOR_SUBDISPATCH_TABLE_FILE_OFFSET,
            NAV_ACTOR_SUBDISPATCH_ENTRY_COUNT,
            "nav actor subdispatch table",
        )
    }

    pub fn nav_choice_subdispatch_handlers(&self) -> Result<Vec<SubdispatchEntry>> {
        self.subdispatch_entries(
            NAV_CHOICE_SUBDISPATCH_TABLE_FILE_OFFSET,
            NAV_CHOICE_SUBDISPATCH_ENTRY_COUNT,
            "nav choice subdispatch table",
        )
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

    pub fn snd_bank_load_call_sites(&self) -> Vec<SndBankLoadCallSite> {
        self.data
            .windows(SND_BANK_LOAD_FAR_CALL.len())
            .enumerate()
            .filter_map(|(file_offset, bytes)| {
                (bytes == SND_BANK_LOAD_FAR_CALL).then(|| {
                    let (segment, offset) = self.file_to_known_segoff(file_offset);
                    let (ax_value, ax_source_file_offset, ax_source, ax_intervening_far_calls) =
                        self.find_ax_source_before(file_offset);
                    let (si_value, si_source_file_offset, si_source, si_intervening_far_calls) =
                        self.find_si_source_before(file_offset);
                    let path = si_value.and_then(|value| self.ds_c_string(value));
                    let path_file_offset = si_value
                        .filter(|_| path.is_some())
                        .map(|value| self.ds_to_file(value));
                    SndBankLoadCallSite {
                        file_offset,
                        segment,
                        offset,
                        target_segment: SND_BANK_LOAD_SEGMENT,
                        target_offset: SND_BANK_LOAD_OFFSET,
                        ax_value,
                        ax_source_file_offset,
                        ax_source,
                        ax_intervening_far_calls,
                        si_value,
                        si_source_file_offset,
                        si_source,
                        si_intervening_far_calls,
                        path_file_offset,
                        path,
                        mode: snd_bank_load_mode(ax_value),
                        note: snd_bank_load_call_note(file_offset),
                    }
                })
            })
            .collect()
    }

    pub fn render_call_sites(&self) -> Vec<RenderCallSite> {
        self.data
            .windows(5)
            .enumerate()
            .filter_map(|(file_offset, bytes)| {
                (bytes[0] == FAR_CALL_OPCODE && bytes[3..5] == RENDER_FAR_CALL_SEGMENT_BYTES).then(
                    || {
                        let target_offset = u16::from_le_bytes([bytes[1], bytes[2]]);
                        let (segment, offset) = self.file_to_known_segoff(file_offset);
                        let (ax_value, ax_source_file_offset, ax_source, intervening_far_calls) =
                            self.find_ax_source_before(file_offset);
                        RenderCallSite {
                            file_offset,
                            segment,
                            offset,
                            target_segment: RENDER_SEGMENT,
                            target_offset,
                            target_file_offset: self.segoff_to_file(RENDER_SEGMENT, target_offset),
                            target_name: render_target_name(target_offset),
                            ax_value,
                            ax_source_file_offset,
                            ax_source,
                            intervening_far_calls,
                            note: render_call_site_note(file_offset, target_offset),
                        }
                    },
                )
            })
            .collect()
    }

    pub fn sprite_blitter_dispatch_entries(&self) -> Result<Vec<SpriteBlitterDispatchEntry>> {
        let bytes = self.slice(
            RENDER_SPRITE_BLITTER_TABLE_FILE_OFFSET,
            RENDER_SPRITE_BLITTER_ENTRY_COUNT * 2,
            "sprite blitter dispatch table",
        )?;
        Ok(bytes
            .chunks_exact(2)
            .enumerate()
            .map(|(mode, pair)| {
                let handler_offset = u16::from_le_bytes([pair[0], pair[1]]);
                SpriteBlitterDispatchEntry {
                    mode: mode as u8,
                    handler_offset,
                    handler_file_offset: self.segoff_to_file(RENDER_SEGMENT, handler_offset),
                    name: sprite_blitter_name(mode as u8, handler_offset),
                    note: sprite_blitter_note(mode as u8, handler_offset),
                }
            })
            .collect())
    }

    pub fn inspect(&self) -> Result<BloodPrgInspection> {
        Ok(BloodPrgInspection {
            summary: self.summary(),
            target_sha256: BLOODPRG_SHA256,
            known_symbols: KNOWN_SYMBOLS.to_vec(),
            opcode_handlers: self.opcode_handlers()?,
            opcode_descriptors: self.opcode_descriptors()?,
            vm_opcode_specs: self.vm_opcode_specs()?,
            nav_actor_subdispatch_handlers: self.nav_actor_subdispatch_handlers()?,
            nav_choice_subdispatch_handlers: self.nav_choice_subdispatch_handlers()?,
            snd_entry_call_sites: self.snd_entry_call_sites(),
            snd_bank_load_call_sites: self.snd_bank_load_call_sites(),
            render_call_sites: self.render_call_sites(),
            sprite_blitter_dispatch: self.sprite_blitter_dispatch_entries()?,
            script_resource_profiles: self.script_resource_profiles()?,
            dialogue_font: self.dialogue_font_tables()?,
        })
    }

    fn subdispatch_entries(
        &self,
        file_offset: usize,
        count: usize,
        label: &str,
    ) -> Result<Vec<SubdispatchEntry>> {
        let bytes = self.slice(file_offset, count * 2, label)?;
        Ok(bytes
            .chunks_exact(2)
            .enumerate()
            .map(|(index, pair)| {
                let handler_offset = u16::from_le_bytes([pair[0], pair[1]]);
                SubdispatchEntry {
                    index,
                    handler_offset,
                    handler_file_offset: self.segoff_to_file(NAV_CODE_SEGMENT, handler_offset),
                }
            })
            .collect())
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
        let window_start = call_file_offset.saturating_sub(REGISTER_SOURCE_SCAN_BACK);
        let mut source = (None, None, "unresolved", window_start);
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

    fn find_si_source_before(
        &self,
        call_file_offset: usize,
    ) -> (Option<u16>, Option<usize>, &'static str, u8) {
        let window_start = call_file_offset.saturating_sub(REGISTER_SOURCE_SCAN_BACK);
        let mut source = (None, None, "unresolved", window_start);
        for pos in window_start..call_file_offset {
            if pos + 3 <= call_file_offset && self.data.get(pos) == Some(&0xbe) {
                let lo = self.data[pos + 1];
                let hi = self.data[pos + 2];
                source = (
                    Some(u16::from_le_bytes([lo, hi])),
                    Some(pos),
                    "mov si, imm16",
                    pos + 3,
                );
            }
        }

        let intervening_far_calls = self.data[source.3..call_file_offset]
            .iter()
            .filter(|byte| **byte == FAR_CALL_OPCODE)
            .count()
            .min(u8::MAX as usize) as u8;

        (source.0, source.1, source.2, intervening_far_calls)
    }

    fn ds_c_string(&self, ds_offset: u16) -> Option<String> {
        let start = self.ds_to_file(ds_offset);
        let bytes = self.data.get(start..)?;
        let nul = bytes
            .iter()
            .take(DS_STRING_SCAN_MAX)
            .position(|byte| *byte == 0)?;
        std::str::from_utf8(&bytes[..nul]).ok().map(str::to_owned)
    }

    fn resource_name(&self, resource_id: u16) -> Result<String> {
        let start = self.fs_to_file(RESOURCE_NAME_TABLE_FS_OFFSET)
            + resource_id as usize * RESOURCE_NAME_ENTRY_LEN;
        let bytes = self.slice(start, RESOURCE_NAME_ENTRY_LEN, "resource name entry")?;
        let nul = bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(RESOURCE_NAME_ENTRY_LEN);
        let name = std::str::from_utf8(&bytes[..nul])
            .with_context(|| format!("resource name {resource_id} is not utf-8"))?;
        Ok(name.to_owned())
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
            rust_status: "execution-trace-ported",
            notes: "C4 consumes record/related words with optional mode1 A1 inversion; Rust writes direct mode0 records and exposes strict mode1 branch-fail behavior through ExecutionContext",
        },
        op if vm::is_record_state_opcode(op) => OpcodeMetadata {
            mnemonic: "record_state",
            family: "line-record",
            rust_status: "partially-executed",
            notes: "C1/C2 line-record state operations; Rust decodes A1 inversion, evaluates direct mode1 compares, applies the context-gated direct C1 mode0 write, and ports the C2 kind-field plus kind2/kind400 active-line mode0 paths while resolved-table side effects remain under RE",
        },
        vm::OP_RECORD_LINK => OpcodeMetadata {
            mnemonic: "record_link",
            family: "line-record",
            rust_status: "partially-executed",
            notes: "C3 line-record relation; Rust decodes A1 inversion, executes guarded mode0 writes and mode1 compares with DEB object context, and deliberately does not treat it as a speaker marker",
        },
        op if vm::is_record_entry_opcode(op) => OpcodeMetadata {
            mnemonic: "record_entry",
            family: "line-record",
            rust_status: "partially-executed",
            notes: "C5-C8 line-record entries; Rust decodes A1 inversion, executes successful guarded mode0 writes, and evaluates direct mode1 compares with concrete host-state evidence",
        },
        vm::OP_RECORD_CLEAR => OpcodeMetadata {
            mnemonic: "record_clear",
            family: "line-record",
            rust_status: "execution-trace-ported",
            notes: "C9 unconditionally clears a 6-byte line record in both VM modes; Rust clears C4 related actor subrecords/gates and stops matching actor/background context bleed",
        },
        vm::OP_GLOBAL_WORD_COMPARE => OpcodeMetadata {
            mnemonic: "global_word_compare",
            family: "global-condition",
            rust_status: "execution-trace-ported",
            notes: "CA compares a token u16 against RTC hour gs:0x0aa6; Rust evaluates branches when ExecutionContext supplies BIOS RTC values",
        },
        vm::OP_GLOBAL_PAIR_COMPARE => OpcodeMetadata {
            mnemonic: "global_pair_compare",
            family: "global-condition",
            rust_status: "execution-trace-ported",
            notes: "CB compares packed month/day against RTC globals gs:0x0aaa/0x0aa8; Rust evaluates branches when ExecutionContext supplies BIOS RTC values",
        },
        op if vm::is_pair_record_opcode(op) => OpcodeMetadata {
            mnemonic: "pair_record",
            family: "pair-record",
            rust_status: "execution-trace-ported",
            notes: "B8/B9/BD pair-record assignment and comparison family; Rust applies mode0 pair writes and execute_trace evaluates mode1 pair compares",
        },
        vm::OP_RECORD_TRIPLE => OpcodeMetadata {
            mnemonic: "record_triple",
            family: "line-record",
            rust_status: "execution-trace-ported",
            notes: "CD consumes record/first/second words with optional A1 inversion; Rust evaluates the direct mode1 compare while resolved-table mode0 side effects remain pending",
        },
        vm::OP_SCRIPT_PROFILE_REQUEST => OpcodeMetadata {
            mnemonic: "script_profile_request",
            family: "script-switch",
            rust_status: "decoded-token",
            notes: "D2 stores sign_extend(operand)-1 in gs:0x6780; the main loop later selects that script resource profile when idle",
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
                notes: "AD/AF/B2/B3/BA/BB/BC family; Rust applies mode0 assignments with blood/0xffff sentinel-list bookkeeping and execute_trace evaluates mode1 equality/inversion with the gs:0x674e RHS remap",
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
                rust_status: "execution-trace-ported",
                notes: "B8/B9/BD pair-record assignment and comparison family; Rust applies mode0 pair writes and mode1 compares",
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

fn snd_bank_load_mode(ax_value: Option<u16>) -> &'static str {
    match ax_value {
        Some(0) => "load_bank_into_memory",
        Some(1) => "extract_or_stream_bank",
        Some(_) => "unknown_nonzero_bank_mode",
        None => "unresolved_bank_mode",
    }
}

fn snd_bank_load_call_note(file_offset: usize) -> &'static str {
    match file_offset {
        0x000fe7 => "main startup/UI loop loads tb.snd into the in-memory clip table",
        0x007667 => "descriptor/presentation path extracts the current templated SND bank",
        0x008263 => "actor/object transition switches to radio.snd through the bank loader",
        0x0087ab => "navigation choice transition switches to radio.snd through the bank loader",
        0x008866 => "navigation choice handler reloads radio.snd through the bank loader",
        0x00b5dc => "presentation setup loads 3D.snd into the in-memory clip table",
        0x00b610 => "presentation setup restores tb.snd after temporary 3D.snd load",
        _ => "unclassified SND bank-loader call",
    }
}

fn render_target_name(target_offset: u16) -> &'static str {
    match target_offset {
        RENDER_VGA_DAC_PALETTE_LOAD_OFFSET => "vga_dac_palette_load",
        RENDER_VGA_DAC_PALETTE_CLEAR_OFFSET => "vga_dac_palette_clear",
        RENDER_FIXED_8X8_TEXT_OFFSET => "fixed_8x8_text_render",
        RENDER_FONT_STRING_WIDTH_OFFSET => "font_string_width_measure",
        RENDER_UI_TEXT_OFFSET => "ui_text_render_10row",
        RENDER_STRING_OFFSET => "render_string_entry",
        RENDER_FRAMEBUFFER_RECT_REMAP_OFFSET => "framebuffer_rect_palette_remap",
        RENDER_PLANAR_UI_TEXT_OFFSET => "planar_ui_text_render_10row",
        RENDER_PLANAR_DIALOGUE_TEXT_OFFSET => "planar_dialogue_text_render",
        RENDER_SUBTITLE_REVEAL_OFFSET => "subtitle_reveal_draw_wrapper",
        RENDER_SMALL_TEXT_OFFSET => "small_text_render",
        RENDER_PLANAR_HORIZONTAL_LINE_OFFSET => "planar_horizontal_line_draw",
        RENDER_PLANAR_VERTICAL_LINE_OFFSET => "planar_vertical_line_draw",
        RENDER_RECT_OUTLINE_OFFSET => "framebuffer_rect_outline",
        RENDER_DITHER_RECT_FILL_OFFSET => "framebuffer_dither_rect_fill",
        RENDER_RECT_FILL_OFFSET => "framebuffer_rect_fill_clipped",
        RENDER_SCENE_BAND_FILL_OFFSET => "scene_band_fill",
        RENDER_SECONDARY_BAND_FILL_OFFSET => "secondary_framebuffer_band_fill",
        RENDER_FRAMEBUFFER_COPY_OFFSET => "framebuffer_copy_full",
        RENDER_SECONDARY_FRAMEBUFFER_COPY_OFFSET => "secondary_framebuffer_copy_full",
        RENDER_VGA_PLANAR_TO_LINEAR_COPY_OFFSET => "vga_planar_to_linear_framebuffer_copy",
        RENDER_PLANAR_COPY_OFFSET => "planar_framebuffer_copy",
        RENDER_RESOURCE_FILE_LOAD_OFFSET => "resource_file_payload_load",
        RENDER_SPRITE_SLOT_RESOURCE_LOAD_OFFSET => "sprite_slot_resource_frame_load",
        RENDER_SPRITE_SLOT_LOAD_OFFSET => "sprite_slot_frame_load",
        RENDER_SPRITE_SLOT_STATE_OFFSET => "sprite_slot_state_update",
        RENDER_SPRITE_SLOT_POSITION_OFFSET => "sprite_slot_position_update",
        RENDER_SPRITE_SLOT_RANGE_DIRTY_OFFSET => "sprite_slot_range_mark_dirty",
        RENDER_SPRITE_SLOT_EXTENT_OFFSET => "sprite_slot_extent_update",
        RENDER_SPRITE_SLOT_COMMIT_RANGE_OFFSET => "sprite_slot_commit_dirty_range",
        RENDER_SPRITE_SLOT_DIRTY_RANGE_RENDER_OFFSET => "sprite_slot_dirty_range_render",
        RENDER_DIRTY_RECTS_COPY_OFFSET => "dirty_rects_copy_secondary_to_primary",
        _ => "unclassified_render_entry",
    }
}

fn render_call_site_note(file_offset: usize, target_offset: u16) -> &'static str {
    match (file_offset, target_offset) {
        (0x0016b0, RENDER_VGA_DAC_PALETTE_LOAD_OFFSET) => {
            "startup/presentation path loads 0x300 palette bytes from DS:0x5B58 into VGA DAC ports 0x3C8/0x3C9"
        }
        (0x00179a, RENDER_VGA_DAC_PALETTE_LOAD_OFFSET) => {
            "palette restore path loads 0x300 palette bytes from DS:0x5251 into VGA DAC ports 0x3C8/0x3C9"
        }
        (0x000c5a, RENDER_VGA_DAC_PALETTE_CLEAR_OFFSET) => {
            "video setup clears all VGA DAC entries before register/mode setup"
        }
        (0x001f34, RENDER_VGA_DAC_PALETTE_CLEAR_OFFSET) => {
            "presentation loop clears the VGA DAC palette before rebuilding framebuffer bands"
        }
        (0x0016c9, RENDER_FIXED_8X8_TEXT_OFFSET) => {
            "startup path renders DS:0x0159 fixed 8x8 text at AX/BX with color DL"
        }
        (0x007d53, RENDER_FIXED_8X8_TEXT_OFFSET) => {
            "navigation actor/object UI loop renders fixed 8x8 labels from a slot table"
        }
        (0x007329, RENDER_FONT_STRING_WIDTH_OFFSET) => {
            "dialogue line-layout path measures remaining text width using the dialogue font selector AX=1"
        }
        (0x00846c, RENDER_FONT_STRING_WIDTH_OFFSET) => {
            "menu/list layout path measures string widths using the 10-row UI font selector AX=0"
        }
        (0x008fcd, RENDER_FONT_STRING_WIDTH_OFFSET) => {
            "dialogue display path measures dialogue-font string width before right-aligning render_string"
        }
        (0x001507 | 0x001515 | 0x001520, RENDER_UI_TEXT_OFFSET) => {
            "startup/menu path renders 10-row UI text labels"
        }
        (0x001e4f, RENDER_UI_TEXT_OFFSET) => "menu prompt path renders one 10-row UI text label",
        (0x008597 | 0x0085ce, RENDER_UI_TEXT_OFFSET) => {
            "dialogue/menu list path renders 10-row UI text with active-line color switching"
        }
        (0x001ac6, RENDER_PLANAR_UI_TEXT_OFFSET) => {
            "startup/presentation path renders 10-row UI text through VGA plane masks into GS:0x521D"
        }
        (0x001eb1 | 0x0078c4 | 0x00851d | 0x008edc, RENDER_FRAMEBUFFER_RECT_REMAP_OFFSET) => {
            "applies a 256-byte palette remap table to an already-rendered clipped primary-framebuffer rectangle"
        }
        (0x0072f6, RENDER_PLANAR_DIALOGUE_TEXT_OFFSET) => {
            "dialogue line-layout path renders dialogue-font text through VGA plane masks into GS:0x5219"
        }
        (0x0094ee, RENDER_SUBTITLE_REVEAL_OFFSET) => {
            "subtitle reveal path draws current text using DS:0x5E5C/0x5E5E origin"
        }
        (0x00946d, RENDER_PLANAR_HORIZONTAL_LINE_OFFSET) => {
            "dialogue updater draws a clipped horizontal line from the line command table"
        }
        (0x009474, RENDER_PLANAR_VERTICAL_LINE_OFFSET) => {
            "dialogue updater draws a clipped vertical line from the line command table"
        }
        (0x007a94 | 0x007b18, RENDER_DITHER_RECT_FILL_OFFSET) => {
            "navigation/dialogue path fills a clipped strip with the binary pseudo-random black/0xEF dither pattern"
        }
        (0x008d14, RENDER_VGA_PLANAR_TO_LINEAR_COPY_OFFSET) => {
            "captures VGA A000:0xC000 planar page bytes into a linear RAM framebuffer before sprite/object composition"
        }
        (0x000fb9, RENDER_RESOURCE_FILE_LOAD_OFFSET) => {
            "loads resource index 0x2C by filename through the FS resource-name table"
        }
        (0x00597f | 0x0070cd, RENDER_RESOURCE_FILE_LOAD_OFFSET) => {
            "loads a high-bit resource index directly into the caller-provided ES:DI buffer"
        }
        (0x008d76 | 0x008d96 | 0x008df5 | 0x0095e7, RENDER_SPRITE_SLOT_RESOURCE_LOAD_OFFSET) => {
            "loads a resolver-backed resource frame into a 32-byte sprite slot at GS:0x6212"
        }
        (0x005990 | 0x0070dd | 0x007e7d | 0x0090d4, RENDER_SPRITE_SLOT_LOAD_OFFSET) => {
            "loads one sprite/frame table entry into a presentation slot"
        }
        (0x0059dc | 0x0059e4, RENDER_SPRITE_SLOT_STATE_OFFSET) => {
            "VM post-update presentation clear resets sprite slot state"
        }
        (0x00929c | 0x009cef, RENDER_SPRITE_SLOT_POSITION_OFFSET) => {
            "updates sprite slot screen position words +0x08/+0x0A and marks the slot dirty on change"
        }
        (0x008ad4, RENDER_SPRITE_SLOT_RANGE_DIRTY_OFFSET) => {
            "marks a contiguous sprite-slot range dirty by clearing active bits and setting the dirty bit"
        }
        (0x00926d | 0x009cd6, RENDER_SPRITE_SLOT_EXTENT_OFFSET) => {
            "updates sprite slot source extent words +0x0C/+0x0E and marks the slot dirty on change"
        }
        (0x007849 | 0x009575 | 0x00b1d0, RENDER_SPRITE_SLOT_COMMIT_RANGE_OFFSET) => {
            "commits dirty sprite-slot geometry into the previous-geometry fields before range rendering/copyback"
        }
        (0x00789a | 0x00957a | 0x00b9b5, RENDER_SPRITE_SLOT_DIRTY_RANGE_RENDER_OFFSET) => {
            "renders the requested sprite-slot range against dirty rectangles in GS:0x6612"
        }
        (0x00787f | 0x008ea0 | 0x00b1d8, RENDER_DIRTY_RECTS_COPY_OFFSET) => {
            "copies dirty rectangles from secondary framebuffer GS:0x5229 back into primary framebuffer GS:0x5221"
        }
        (_, RENDER_STRING_OFFSET) => "dialogue/UI text render call",
        (_, RENDER_SMALL_TEXT_OFFSET) => "small 5-row text render call",
        (_, RENDER_RECT_FILL_OFFSET) => "clipped fill rectangle in primary framebuffer",
        (_, RENDER_SCENE_BAND_FILL_OFFSET) => "fills the current clipped framebuffer band",
        (_, RENDER_SECONDARY_BAND_FILL_OFFSET) => {
            "fills the current clipped band in the secondary framebuffer"
        }
        (_, RENDER_FRAMEBUFFER_COPY_OFFSET) => {
            "copies a full 320x200 buffer into the primary framebuffer"
        }
        (_, RENDER_SECONDARY_FRAMEBUFFER_COPY_OFFSET) => {
            "copies a full 320x200 buffer into the secondary framebuffer"
        }
        (_, RENDER_PLANAR_COPY_OFFSET) => {
            "copies planar/interleaved image data into the primary framebuffer"
        }
        (_, RENDER_SPRITE_SLOT_RESOURCE_LOAD_OFFSET) => {
            "loads a resource-resolved frame into one 32-byte presentation sprite slot"
        }
        (_, RENDER_SPRITE_SLOT_STATE_OFFSET) => "updates one presentation sprite slot state",
        (_, RENDER_SPRITE_SLOT_POSITION_OFFSET) => {
            "updates one presentation sprite slot position and marks it dirty on change"
        }
        (_, RENDER_SPRITE_SLOT_RANGE_DIRTY_OFFSET) => {
            "marks a contiguous presentation sprite-slot range dirty"
        }
        (_, RENDER_SPRITE_SLOT_EXTENT_OFFSET) => {
            "updates one presentation sprite slot extent and marks it dirty on change"
        }
        (_, RENDER_SPRITE_SLOT_COMMIT_RANGE_OFFSET) => {
            "commits dirty presentation sprite-slot geometry for a slot range"
        }
        (_, RENDER_DIRTY_RECTS_COPY_OFFSET) => {
            "copies dirty rectangles from the secondary framebuffer into the primary framebuffer"
        }
        (_, RENDER_VGA_DAC_PALETTE_LOAD_OFFSET) => {
            "loads 0x300 palette bytes from DS:SI into VGA DAC ports 0x3C8/0x3C9"
        }
        (_, RENDER_VGA_DAC_PALETTE_CLEAR_OFFSET) => {
            "zeros all VGA DAC palette entries through ports 0x3C8/0x3C9"
        }
        (_, RENDER_FIXED_8X8_TEXT_OFFSET) => {
            "renders DS:SI through the fixed 8x8 glyph table at GS:0x5225 into the primary framebuffer"
        }
        (_, RENDER_FONT_STRING_WIDTH_OFFSET) => {
            "measures NUL-terminated text width using the UI or dialogue font advance tables"
        }
        (_, RENDER_UI_TEXT_OFFSET) => {
            "renders NUL-terminated text with the 10-row UI font tables at GS:0x7362/0x7412/0x7442"
        }
        (_, RENDER_PLANAR_UI_TEXT_OFFSET) => {
            "renders 10-row UI text through VGA plane masks into the GS:0x521D framebuffer"
        }
        (_, RENDER_PLANAR_DIALOGUE_TEXT_OFFSET) => {
            "renders dialogue-font text through VGA plane masks into the GS:0x5219 framebuffer"
        }
        (_, RENDER_FRAMEBUFFER_RECT_REMAP_OFFSET) => {
            "applies a palette remap lookup table to a clipped primary-framebuffer rectangle"
        }
        (_, RENDER_PLANAR_HORIZONTAL_LINE_OFFSET) => {
            "draws a clipped horizontal line into the GS:0x5219 planar framebuffer"
        }
        (_, RENDER_PLANAR_VERTICAL_LINE_OFFSET) => {
            "draws a clipped vertical line into the GS:0x5219 planar framebuffer"
        }
        (_, RENDER_RECT_OUTLINE_OFFSET) => {
            "draws a clipped rectangle outline using primary-framebuffer line helpers"
        }
        (_, RENDER_DITHER_RECT_FILL_OFFSET) => {
            "fills a clipped primary-framebuffer rectangle with the binary pseudo-random dither pattern"
        }
        (_, RENDER_VGA_PLANAR_TO_LINEAR_COPY_OFFSET) => {
            "copies four VGA read-map planes into one linear 320x200 framebuffer"
        }
        (_, RENDER_RESOURCE_FILE_LOAD_OFFSET) => {
            "loads a resource file payload addressed by the FS resource-name table"
        }
        (_, RENDER_SPRITE_SLOT_DIRTY_RANGE_RENDER_OFFSET) => {
            "renders active dirty sprite slots through the dirty-rectangle list"
        }
        _ => "unclassified render-segment call",
    }
}

fn sprite_blitter_name(mode: u8, handler_offset: u16) -> &'static str {
    match (mode, handler_offset) {
        (0, RENDER_SPRITE_BLIT_RAW_TRANSPARENT_OFFSET) => "sprite_blit_raw_transparent",
        (1, RENDER_SPRITE_BLIT_RLE_TRANSPARENT_OFFSET) => "sprite_blit_rle_transparent",
        (2, RENDER_SPRITE_BLIT_RAW_OPAQUE_OFFSET) => "sprite_blit_raw_opaque",
        (3, RENDER_SPRITE_BLIT_RLE_OPAQUE_OFFSET) => "sprite_blit_rle_opaque",
        (4, RENDER_SPRITE_BLIT_SCALED_TRANSPARENT_OFFSET) => "sprite_blit_scaled_transparent",
        (5, RENDER_SPRITE_BLIT_NOOP_5_OFFSET) => "sprite_blit_noop_mode5",
        (6, RENDER_SPRITE_BLIT_NOOP_6_OFFSET) => "sprite_blit_noop_mode6",
        (7, RENDER_SPRITE_BLIT_NOOP_7_OFFSET) => "sprite_blit_noop_mode7",
        _ => "unclassified_sprite_blitter",
    }
}

fn sprite_blitter_note(mode: u8, handler_offset: u16) -> &'static str {
    match (mode, handler_offset) {
        (0, RENDER_SPRITE_BLIT_RAW_TRANSPARENT_OFFSET) => {
            "uncompressed transparent sprite blit; source zero skips destination, nonzero pixels either copy directly or remap the destination through the selected palette table"
        }
        (1, RENDER_SPRITE_BLIT_RLE_TRANSPARENT_OFFSET) => {
            "RLE transparent sprite blit with the same zero-skip and optional destination-remap behavior as raw transparent mode"
        }
        (2, RENDER_SPRITE_BLIT_RAW_OPAQUE_OFFSET) => {
            "uncompressed opaque sprite blit; copies all source pixels directly with no zero transparency or destination remap"
        }
        (3, RENDER_SPRITE_BLIT_RLE_OPAQUE_OFFSET) => {
            "RLE opaque sprite blit; decodes run spans/fills/copies and writes all pixels without destination remap"
        }
        (4, RENDER_SPRITE_BLIT_SCALED_TRANSPARENT_OFFSET) => {
            "scaled transparent sprite blit; fixed-point source sampling over destination extents and zero source pixels skip destination"
        }
        (5, RENDER_SPRITE_BLIT_NOOP_5_OFFSET)
        | (6, RENDER_SPRITE_BLIT_NOOP_6_OFFSET)
        | (7, RENDER_SPRITE_BLIT_NOOP_7_OFFSET) => {
            "unused/no-op sprite blitter mode; handler is a single near return"
        }
        _ => {
            "handler entry is present in the sprite blitter table, but semantics are not yet named"
        }
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
        name: "bios_rtc_hour_to_vm_global",
        file_offset: 0x00093b,
        segment: Some(0x0000),
        offset: Some(0x033b),
        ds_offset: None,
        kind: "runtime-clock",
        comment: "BIOS int 1Ah AH=02h; BCD-decodes current hour into gs:0x0aa6 before VM execution",
    },
    BinarySymbol {
        name: "bios_rtc_date_to_vm_globals",
        file_offset: 0x000950,
        segment: Some(0x0000),
        offset: Some(0x0350),
        ds_offset: None,
        kind: "runtime-clock",
        comment: "BIOS int 1Ah AH=04h; BCD-decodes day/month into gs:0x0aa8/0x0aaa before VM execution",
    },
    BinarySymbol {
        name: "bcd_byte_to_decimal",
        file_offset: 0x000986,
        segment: Some(0x0000),
        offset: Some(0x0386),
        ds_offset: None,
        kind: "runtime-clock",
        comment: "helper converting one packed BCD byte to a decimal byte for BIOS RTC fields",
    },
    BinarySymbol {
        name: "vm_resource_profile_select",
        file_offset: 0x0053a0,
        segment: Some(VM_CODE_SEGMENT),
        offset: Some(0x0000),
        ds_offset: None,
        kind: "script-vm",
        comment: "select script/resource profile AX; frees old offsets, loads five new offsets, and clears VM globals",
    },
    BinarySymbol {
        name: "vm_resource_offsets_populate",
        file_offset: 0x0053c8,
        segment: Some(VM_CODE_SEGMENT),
        offset: Some(0x0028),
        ds_offset: None,
        kind: "script-vm",
        comment: "copy selected five-resource profile from FS:0x11f4 + AX*10 into DS:0x6712",
    },
    BinarySymbol {
        name: "vm_resource_offset_copy_loop",
        file_offset: 0x0053da,
        segment: Some(VM_CODE_SEGMENT),
        offset: Some(0x003a),
        ds_offset: None,
        kind: "script-vm",
        comment: "copy and validate each selected resource offset before VM pointer resolution",
    },
    BinarySymbol {
        name: "vm_run_wrapper",
        file_offset: 0x0055a4,
        segment: Some(VM_CODE_SEGMENT),
        offset: Some(0x0204),
        ds_offset: None,
        kind: "script-vm",
        comment: "refreshes runtime globals, resolves selected resource offsets, then enters the VM executor",
    },
    BinarySymbol {
        name: "vm_resource_ptr_resolve_loop",
        file_offset: 0x0055d9,
        segment: Some(VM_CODE_SEGMENT),
        offset: Some(0x0239),
        ds_offset: None,
        kind: "script-vm",
        comment: "resolves five DS:0x6712 resource offsets into far pointers at DS:0x671c",
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
        name: "resource_name_table",
        file_offset: RESOURCE_NAME_TABLE_FILE_OFFSET,
        segment: Some(FS_SEGMENT),
        offset: Some(RESOURCE_NAME_TABLE_FS_OFFSET),
        ds_offset: None,
        kind: "resource-data",
        comment: "16-byte resource filename entries indexed by resource ID",
    },
    BinarySymbol {
        name: "script_resource_profile_table",
        file_offset: SCRIPT_RESOURCE_PROFILE_TABLE_FILE_OFFSET,
        segment: Some(FS_SEGMENT),
        offset: Some(SCRIPT_RESOURCE_PROFILE_TABLE_FS_OFFSET),
        ds_offset: None,
        kind: "resource-data",
        comment: "five static script profiles of COD/BAS/VAR/DIC/DEB resource IDs",
    },
    BinarySymbol {
        name: "vm_resource_offsets",
        file_offset: 0x013b32,
        segment: None,
        offset: None,
        ds_offset: Some(0x6712),
        kind: "script-vm-data",
        comment: "five u16 resource offsets populated from the selected FS:0x11f4 profile",
    },
    BinarySymbol {
        name: "vm_resource_pointer_block",
        file_offset: 0x013b3c,
        segment: None,
        offset: None,
        ds_offset: Some(0x671c),
        kind: "script-vm-data",
        comment: "five far pointers resolved from DS:0x6712: exec COD, aux COD, state, DIC, DEB/object table",
    },
    BinarySymbol {
        name: "vm_state_ptr",
        file_offset: 0x013b44,
        segment: None,
        offset: None,
        ds_offset: Some(0x6724),
        kind: "script-vm-data",
        comment: "far pointer to runtime object/line-record state table",
    },
    BinarySymbol {
        name: "vm_dic_ptr",
        file_offset: 0x013b48,
        segment: None,
        offset: None,
        ds_offset: Some(0x6728),
        kind: "script-vm-data",
        comment: "DIC far pointer used by TEXT subtitle assembly",
    },
    BinarySymbol {
        name: "vm_deb_object_table_ptr",
        file_offset: 0x013b4c,
        segment: None,
        offset: None,
        ds_offset: Some(0x672c),
        kind: "script-vm-data",
        comment: "DEB/object table far pointer scanned as 20-byte records",
    },
    BinarySymbol {
        name: "vm_resource_profile_index",
        file_offset: 0x013b9e,
        segment: None,
        offset: None,
        ds_offset: Some(0x677e),
        kind: "script-vm-data",
        comment: "current selected resource profile index; avoids reload in 0x53a0 when AX matches",
    },
    BinarySymbol {
        name: "vm_pending_resource_profile",
        file_offset: 0x013ba0,
        segment: None,
        offset: None,
        ds_offset: Some(0x6780),
        kind: "script-vm-data",
        comment: "pending D2-requested resource profile index consumed by the main loop",
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
        comment: "B8/B9/BD pair-record assignment/compare family; stores or tests two words at a direct record offset",
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
        name: "vm_op_d2_script_profile_request",
        file_offset: 0x0064b8,
        segment: Some(VM_CODE_SEGMENT),
        offset: Some(0x1118),
        ds_offset: None,
        kind: "script-vm",
        comment: "D2 handler stores sign-extended operand minus one into DS:0x6780",
    },
    BinarySymbol {
        name: "vm_op_cd_record_triple",
        file_offset: 0x0069c7,
        segment: Some(0x04da),
        offset: Some(0x1627),
        ds_offset: None,
        kind: "script-vm",
        comment: "CD record-triple handler; consumes record/first/second words with optional A1 inverted compare prefix",
    },
    BinarySymbol {
        name: "vm_post_exec_record_update",
        file_offset: 0x005816,
        segment: Some(VM_CODE_SEGMENT),
        offset: Some(0x0476),
        ds_offset: None,
        kind: "script-vm",
        comment: "post-exec scan over DEB/object table and runtime state records",
    },
    BinarySymbol {
        name: "vm_post_update_c4_pair",
        file_offset: 0x005d8f,
        segment: Some(VM_CODE_SEGMENT),
        offset: Some(0x09ef),
        ds_offset: None,
        kind: "script-vm",
        comment: "post-update C4 pair path marks primary aux 0xffff and writes reciprocal selector-0x13 C4 record",
    },
    BinarySymbol {
        name: "vga_dac_palette_load",
        file_offset: 0x002f90,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_VGA_DAC_PALETTE_LOAD_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "loads 256 6-bit RGB DAC entries from DS:SI through VGA ports 0x3C8/0x3C9",
    },
    BinarySymbol {
        name: "vga_dac_palette_clear",
        file_offset: 0x002fa6,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_VGA_DAC_PALETTE_CLEAR_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "zeros all 256 VGA DAC palette entries through VGA ports 0x3C8/0x3C9",
    },
    BinarySymbol {
        name: "fixed_8x8_text_render",
        file_offset: 0x003066,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_FIXED_8X8_TEXT_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "renders DS:SI through the fixed 8x8 glyph table at GS:0x5225 into the primary framebuffer; AX=x, BX=y, DL=color, DH=max chars",
    },
    BinarySymbol {
        name: "font_string_width_measure",
        file_offset: 0x0030cd,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_FONT_STRING_WIDTH_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "measures NUL-terminated string width from UI font tables when AX=0 or dialogue font tables when AX!=0",
    },
    BinarySymbol {
        name: "ui_text_render_10row",
        file_offset: 0x003106,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_UI_TEXT_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "renders NUL-terminated strings with the 10-row UI font tables at GS:0x7362/0x7412/0x7442",
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
        name: "framebuffer_rect_palette_remap",
        file_offset: 0x00339e,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_FRAMEBUFFER_RECT_REMAP_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "clips a primary-framebuffer rectangle and replaces each pixel by table[pixel] using the 256-byte table at DS:SI",
    },
    BinarySymbol {
        name: "planar_ui_text_render_10row",
        file_offset: 0x003428,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_PLANAR_UI_TEXT_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "renders 10-row UI text through VGA plane masks into framebuffer pointer GS:0x521D",
    },
    BinarySymbol {
        name: "planar_dialogue_text_render",
        file_offset: 0x00356e,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_PLANAR_DIALOGUE_TEXT_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "renders dialogue-font text through VGA plane masks into framebuffer pointer GS:0x5219",
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
        name: "subtitle_reveal_draw_wrapper",
        file_offset: 0x003630,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SUBTITLE_REVEAL_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "subtitle reveal renderer called from 0x94ee with DS:0x5e5c/0x5e5e origin",
    },
    BinarySymbol {
        name: "small_text_render",
        file_offset: 0x0036ea,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SMALL_TEXT_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "renders a NUL-terminated string with the 5-row small font tables at 0x6fa8/0x7028",
    },
    BinarySymbol {
        name: "planar_horizontal_line_draw",
        file_offset: 0x0039bb,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_PLANAR_HORIZONTAL_LINE_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "draws a clipped horizontal line into framebuffer pointer GS:0x5219, honoring the render clip bounds",
    },
    BinarySymbol {
        name: "planar_vertical_line_draw",
        file_offset: 0x003ab3,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_PLANAR_VERTICAL_LINE_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "draws a clipped vertical line into framebuffer pointer GS:0x5219, honoring the render clip bounds",
    },
    BinarySymbol {
        name: "framebuffer_rect_outline",
        file_offset: 0x003b45,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_RECT_OUTLINE_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "draws a clipped rectangle outline in the primary framebuffer by calling the horizontal/vertical line helpers",
    },
    BinarySymbol {
        name: "framebuffer_dither_rect_fill",
        file_offset: 0x003b85,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_DITHER_RECT_FILL_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "clips a primary-framebuffer rectangle and fills it with the binary pseudo-random black/0xEF dither pattern",
    },
    BinarySymbol {
        name: "framebuffer_rect_fill_clipped",
        file_offset: 0x003c6c,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_RECT_FILL_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "clips and fills a rectangle in the primary framebuffer DS:0x5221",
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
        name: "secondary_framebuffer_band_fill",
        file_offset: 0x003dbf,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SECONDARY_BAND_FILL_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "fills framebuffer band using y-clip bounds DS:0x5239..0x523b and base DS:0x5229",
    },
    BinarySymbol {
        name: "framebuffer_copy_full",
        file_offset: 0x003e46,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_FRAMEBUFFER_COPY_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "copies 0x3e80 dwords from DS:SI into primary framebuffer DS:0x5221",
    },
    BinarySymbol {
        name: "secondary_framebuffer_copy_full",
        file_offset: 0x003e5b,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SECONDARY_FRAMEBUFFER_COPY_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "copies 0x3e80 dwords from DS:SI into secondary framebuffer DS:0x5229",
    },
    BinarySymbol {
        name: "vga_planar_to_linear_framebuffer_copy",
        file_offset: 0x003e70,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_VGA_PLANAR_TO_LINEAR_COPY_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "uses VGA Graphics Controller read-map select to copy four 0x3E80-byte planes from DS:SI into interleaved linear ES:DI bytes",
    },
    BinarySymbol {
        name: "planar_framebuffer_copy",
        file_offset: 0x003ece,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_PLANAR_COPY_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "copies planar/interleaved source data into primary framebuffer DS:0x5219",
    },
    BinarySymbol {
        name: "resource_file_payload_load",
        file_offset: 0x003fc7,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_RESOURCE_FILE_LOAD_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "looks up a resource filename in the FS:0x0C04 table and loads the payload either through the resource allocator or into caller-provided ES:DI",
    },
    BinarySymbol {
        name: "sprite_slot_resource_frame_load",
        file_offset: 0x0040d0,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SPRITE_SLOT_RESOURCE_LOAD_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "resolves a resource frame through 0x04B9:0x0190 and loads it into the 32-byte sprite slot selected by AX",
    },
    BinarySymbol {
        name: "sprite_slot_frame_load",
        file_offset: 0x00414e,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SPRITE_SLOT_LOAD_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "loads one frame-table entry into the 32-byte presentation sprite slot selected by AX",
    },
    BinarySymbol {
        name: "sprite_slot_state_update",
        file_offset: 0x0041d1,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SPRITE_SLOT_STATE_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "updates the state word for the 32-byte presentation sprite slot selected by AX",
    },
    BinarySymbol {
        name: "sprite_slot_position_update",
        file_offset: 0x00420d,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SPRITE_SLOT_POSITION_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "updates sprite slot +0x08/+0x0A screen position words and sets the dirty bit when they change",
    },
    BinarySymbol {
        name: "sprite_slot_range_mark_dirty",
        file_offset: 0x004240,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SPRITE_SLOT_RANGE_DIRTY_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "marks a contiguous range of 32-byte sprite slots dirty in the GS:0x6212 slot table",
    },
    BinarySymbol {
        name: "sprite_slot_extent_update",
        file_offset: 0x0042cd,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SPRITE_SLOT_EXTENT_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "updates sprite slot +0x0C/+0x0E extent words and sets dirty/source-change bits when they change",
    },
    BinarySymbol {
        name: "sprite_slot_commit_dirty_range",
        file_offset: 0x0043f7,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SPRITE_SLOT_COMMIT_RANGE_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "commits dirty sprite slot current geometry into previous-geometry fields across an AX..BX range",
    },
    BinarySymbol {
        name: "sprite_slot_dirty_range_render",
        file_offset: 0x004471,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SPRITE_SLOT_DIRTY_RANGE_RENDER_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "walks active sprite slots in an AX..BX range, intersects each slot with dirty rectangles at GS:0x6612, dispatches the selected blitter, and clears the dirty bit",
    },
    BinarySymbol {
        name: "sprite_blitter_dispatch_table",
        file_offset: RENDER_SPRITE_BLITTER_TABLE_FILE_OFFSET,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SPRITE_BLITTER_TABLE_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "eight near handler offsets selected by (slot_state >> 1) & 7 inside sprite_slot_dirty_range_render",
    },
    BinarySymbol {
        name: "sprite_blit_raw_transparent",
        file_offset: 0x004536,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SPRITE_BLIT_RAW_TRANSPARENT_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "mode 0 sprite blitter; uncompressed transparent source with optional destination palette remap",
    },
    BinarySymbol {
        name: "sprite_blit_rle_transparent",
        file_offset: 0x0046bc,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SPRITE_BLIT_RLE_TRANSPARENT_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "mode 1 sprite blitter; RLE transparent source with optional destination palette remap",
    },
    BinarySymbol {
        name: "sprite_blit_raw_opaque",
        file_offset: 0x004ba8,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SPRITE_BLIT_RAW_OPAQUE_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "mode 2 sprite blitter; uncompressed opaque source copy",
    },
    BinarySymbol {
        name: "sprite_blit_rle_opaque",
        file_offset: 0x004cd6,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SPRITE_BLIT_RLE_OPAQUE_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "mode 3 sprite blitter; RLE opaque source decode/copy",
    },
    BinarySymbol {
        name: "sprite_blit_scaled_transparent",
        file_offset: 0x004f62,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SPRITE_BLIT_SCALED_TRANSPARENT_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "mode 4 sprite blitter; scaled transparent source sampling",
    },
    BinarySymbol {
        name: "sprite_blit_noop_mode5",
        file_offset: 0x00509a,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SPRITE_BLIT_NOOP_5_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "mode 5 sprite blitter; unused single-byte near return",
    },
    BinarySymbol {
        name: "sprite_blit_noop_mode6",
        file_offset: 0x00509b,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SPRITE_BLIT_NOOP_6_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "mode 6 sprite blitter; unused single-byte near return",
    },
    BinarySymbol {
        name: "sprite_blit_noop_mode7",
        file_offset: 0x00509c,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_SPRITE_BLIT_NOOP_7_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "mode 7 sprite blitter; unused single-byte near return",
    },
    BinarySymbol {
        name: "dirty_rects_copy_secondary_to_primary",
        file_offset: 0x00509d,
        segment: Some(RENDER_SEGMENT),
        offset: Some(RENDER_DIRTY_RECTS_COPY_OFFSET),
        ds_offset: None,
        kind: "presentation",
        comment: "copies dirty rectangles described at ES:DI from secondary framebuffer GS:0x5229 to primary framebuffer GS:0x5221",
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
        name: "nav_actor_slot_update_loop",
        file_offset: 0x007d7b,
        segment: Some(NAV_CODE_SEGMENT),
        offset: Some(0x059b),
        ds_offset: None,
        kind: "presentation",
        comment: "walks six 0x18-byte navigation actor/object slots and dispatches via cs:0x06d4",
    },
    BinarySymbol {
        name: "nav_actor_subdispatch_call",
        file_offset: 0x007e09,
        segment: Some(NAV_CODE_SEGMENT),
        offset: Some(0x0629),
        ds_offset: None,
        kind: "presentation",
        comment: "indirect call through cs:0x06d4 actor slot table",
    },
    BinarySymbol {
        name: "nav_actor_subdispatch_table",
        file_offset: NAV_ACTOR_SUBDISPATCH_TABLE_FILE_OFFSET,
        segment: Some(NAV_CODE_SEGMENT),
        offset: Some(0x06d4),
        ds_offset: None,
        kind: "presentation",
        comment: "six u16 near offsets for navigation actor slot handlers",
    },
    BinarySymbol {
        name: "nav_choice_dispatch",
        file_offset: 0x0085e2,
        segment: Some(NAV_CODE_SEGMENT),
        offset: Some(0x0e02),
        ds_offset: None,
        kind: "presentation",
        comment: "navigation choice dispatch routine; rejects AL >= 5 before cs:0x0f29 table call",
    },
    BinarySymbol {
        name: "nav_choice_subdispatch_call",
        file_offset: 0x008700,
        segment: Some(NAV_CODE_SEGMENT),
        offset: Some(0x0f20),
        ds_offset: None,
        kind: "presentation",
        comment: "indirect call through cs:0x0f29 navigation choice table",
    },
    BinarySymbol {
        name: "nav_choice_subdispatch_table",
        file_offset: NAV_CHOICE_SUBDISPATCH_TABLE_FILE_OFFSET,
        segment: Some(NAV_CODE_SEGMENT),
        offset: Some(0x0f29),
        ds_offset: None,
        kind: "presentation",
        comment: "five u16 near offsets for navigation choice handlers",
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
        name: "snd_bank_loader",
        file_offset: 0x00c005,
        segment: Some(SND_BANK_LOAD_SEGMENT),
        offset: Some(SND_BANK_LOAD_OFFSET),
        ds_offset: None,
        kind: "audio",
        comment: "SND bank loader/extractor; AX=0 builds in-memory clip table, AX!=0 preserves table and can write son.snd",
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
        name: "snd_path_tb",
        file_offset: 0x00e11c,
        segment: None,
        offset: None,
        ds_offset: Some(0x0cfc),
        kind: "audio-data",
        comment: "static SND path string: sn\\tb.snd",
    },
    BinarySymbol {
        name: "snd_path_template",
        file_offset: 0x00e126,
        segment: None,
        offset: None,
        ds_offset: Some(0x0d06),
        kind: "audio-data",
        comment: "static SND path template: sn\\xxxxxxxxxxxx",
    },
    BinarySymbol {
        name: "snd_path_radio",
        file_offset: 0x00e136,
        segment: None,
        offset: None,
        ds_offset: Some(0x0d16),
        kind: "audio-data",
        comment: "static SND path string: sn\\radio.snd",
    },
    BinarySymbol {
        name: "snd_path_3d",
        file_offset: 0x00e143,
        segment: None,
        offset: None,
        ds_offset: Some(0x0d23),
        kind: "audio-data",
        comment: "static SND path string: sn\\3D.snd",
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
        assert_eq!(
            binary.fs_to_file(RESOURCE_NAME_TABLE_FS_OFFSET),
            RESOURCE_NAME_TABLE_FILE_OFFSET
        );
        assert_eq!(
            binary.fs_to_file(SCRIPT_RESOURCE_PROFILE_TABLE_FS_OFFSET),
            SCRIPT_RESOURCE_PROFILE_TABLE_FILE_OFFSET
        );
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
    fn script_resource_profiles_map_d2_operands_to_script_files() {
        let Some(binary) = fixture() else {
            eprintln!("skipping: BLOODPRG.EXE not available");
            return;
        };
        let profiles = binary
            .script_resource_profiles()
            .expect("script resource profiles");

        assert_eq!(profiles.len(), SCRIPT_RESOURCE_PROFILE_COUNT);
        for (idx, profile) in profiles.iter().enumerate() {
            let script_number = idx + 1;
            assert_eq!(profile.profile_index, idx as u8);
            assert_eq!(profile.d2_operand, script_number as u8);
            assert_eq!(profile.script_number, script_number as u8);
            let names: Vec<_> = profile
                .slots
                .iter()
                .map(|slot| slot.name.as_str())
                .collect();
            assert_eq!(
                names,
                vec![
                    format!("script{script_number}.cod"),
                    format!("script{script_number}.bas"),
                    format!("script{script_number}.var"),
                    format!("script{script_number}.dic"),
                    format!("script{script_number}.deb"),
                ]
            );
        }
        let ids: Vec<u16> = profiles[1]
            .slots
            .iter()
            .map(|slot| slot.resource_id)
            .collect();
        assert_eq!(ids, vec![0x25, 0x26, 0x27, 0x28, 0x29]);
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
    fn nav_subdispatch_tables_resolve_known_entry_points() {
        let Some(binary) = fixture() else {
            eprintln!("skipping: BLOODPRG.EXE not available");
            return;
        };

        assert_eq!(
            binary.segoff_to_file(NAV_CODE_SEGMENT, 0x06d4),
            NAV_ACTOR_SUBDISPATCH_TABLE_FILE_OFFSET
        );
        assert_eq!(
            binary.segoff_to_file(NAV_CODE_SEGMENT, 0x0f29),
            NAV_CHOICE_SUBDISPATCH_TABLE_FILE_OFFSET
        );

        let actor = binary
            .nav_actor_subdispatch_handlers()
            .expect("actor subdispatch table");
        let actor_offsets: Vec<u16> = actor.iter().map(|entry| entry.handler_offset).collect();
        assert_eq!(
            actor_offsets,
            vec![0x07bc, 0x06e0, 0x095a, 0x099e, 0x0a1b, 0x08a2]
        );
        assert_eq!(actor[0].handler_file_offset, 0x007f9c);
        assert_eq!(actor[5].handler_file_offset, 0x008082);

        let choice = binary
            .nav_choice_subdispatch_handlers()
            .expect("choice subdispatch table");
        let choice_offsets: Vec<u16> = choice.iter().map(|entry| entry.handler_offset).collect();
        assert_eq!(choice_offsets, vec![0x0f33, 0x0f4c, 0x0fdd, 0x1068, 0x108c]);
        assert_eq!(choice[0].handler_file_offset, 0x008713);
        assert_eq!(choice[4].handler_file_offset, 0x00886c);
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

        let profile_request = specs
            .iter()
            .find(|spec| spec.opcode == vm::OP_SCRIPT_PROFILE_REQUEST)
            .unwrap();
        assert_eq!(profile_request.mnemonic, "script_profile_request");
        assert_eq!(profile_request.family, "script-switch");
        assert_eq!(profile_request.handler_file_offset, 0x0064b8);
        assert_eq!(profile_request.len_mode0, 2);
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
    fn snd_bank_load_call_sites_recover_modes_and_paths() {
        let Some(binary) = fixture() else {
            eprintln!("skipping: BLOODPRG.EXE not available");
            return;
        };
        let sites = binary.snd_bank_load_call_sites();
        let got: Vec<_> = sites
            .iter()
            .map(|site| {
                (
                    site.file_offset,
                    site.segment,
                    site.offset,
                    site.ax_value,
                    site.si_value,
                    site.path_file_offset,
                    site.path.as_deref(),
                    site.mode,
                )
            })
            .collect();
        assert_eq!(
            got,
            vec![
                (
                    0x000fe7,
                    0x008b,
                    0x0137,
                    Some(0),
                    Some(0x0cfc),
                    Some(0x00e11c),
                    Some("sn\\tb.snd"),
                    "load_bank_into_memory",
                ),
                (
                    0x007667,
                    0x04da,
                    0x22c7,
                    Some(1),
                    Some(0x0d06),
                    Some(0x00e126),
                    Some("sn\\xxxxxxxxxxxx"),
                    "extract_or_stream_bank",
                ),
                (
                    0x008263,
                    0x071e,
                    0x0a83,
                    Some(1),
                    Some(0x0d16),
                    Some(0x00e136),
                    Some("sn\\radio.snd"),
                    "extract_or_stream_bank",
                ),
                (
                    0x0087ab,
                    0x071e,
                    0x0fcb,
                    Some(1),
                    Some(0x0d16),
                    Some(0x00e136),
                    Some("sn\\radio.snd"),
                    "extract_or_stream_bank",
                ),
                (
                    0x008866,
                    0x071e,
                    0x1086,
                    Some(1),
                    Some(0x0d16),
                    Some(0x00e136),
                    Some("sn\\radio.snd"),
                    "extract_or_stream_bank",
                ),
                (
                    0x00b5dc,
                    0x0a9a,
                    0x063c,
                    Some(0),
                    Some(0x0d23),
                    Some(0x00e143),
                    Some("sn\\3D.snd"),
                    "load_bank_into_memory",
                ),
                (
                    0x00b610,
                    0x0a9a,
                    0x0670,
                    Some(0),
                    Some(0x0cfc),
                    Some(0x00e11c),
                    Some("sn\\tb.snd"),
                    "load_bank_into_memory",
                ),
            ]
        );

        let template_bank = sites
            .iter()
            .find(|site| site.file_offset == 0x007667)
            .expect("templated descriptor SND bank call");
        assert_eq!(template_bank.ax_source, "mov ax, imm16");
        assert_eq!(template_bank.si_source, "mov si, imm16");
        assert!(template_bank.note.contains("templated SND bank"));
    }

    #[test]
    fn render_call_sites_recover_presentation_targets() {
        let Some(binary) = fixture() else {
            eprintln!("skipping: BLOODPRG.EXE not available");
            return;
        };

        let sites = binary.render_call_sites();
        assert_eq!(sites.len(), 143);

        let mut target_counts = std::collections::BTreeMap::new();
        for site in &sites {
            *target_counts.entry(site.target_offset).or_insert(0usize) += 1;
            assert_eq!(site.target_segment, RENDER_SEGMENT);
            assert_eq!(
                site.target_file_offset,
                binary.segoff_to_file(RENDER_SEGMENT, site.target_offset)
            );
        }
        assert_eq!(target_counts.len(), 32);
        assert_eq!(
            target_counts.get(&RENDER_SPRITE_SLOT_STATE_OFFSET),
            Some(&33)
        );
        assert_eq!(
            target_counts.get(&RENDER_SPRITE_SLOT_RESOURCE_LOAD_OFFSET),
            Some(&4)
        );
        assert_eq!(target_counts.get(&RENDER_SPRITE_SLOT_LOAD_OFFSET), Some(&4));
        assert_eq!(
            target_counts.get(&RENDER_SPRITE_SLOT_POSITION_OFFSET),
            Some(&2)
        );
        assert_eq!(
            target_counts.get(&RENDER_SPRITE_SLOT_RANGE_DIRTY_OFFSET),
            Some(&1)
        );
        assert_eq!(
            target_counts.get(&RENDER_SPRITE_SLOT_EXTENT_OFFSET),
            Some(&2)
        );
        assert_eq!(
            target_counts.get(&RENDER_SPRITE_SLOT_COMMIT_RANGE_OFFSET),
            Some(&3)
        );
        assert_eq!(target_counts.get(&RENDER_DIRTY_RECTS_COPY_OFFSET), Some(&3));
        assert_eq!(
            target_counts.get(&RENDER_VGA_DAC_PALETTE_LOAD_OFFSET),
            Some(&2)
        );
        assert_eq!(
            target_counts.get(&RENDER_VGA_DAC_PALETTE_CLEAR_OFFSET),
            Some(&2)
        );
        assert_eq!(target_counts.get(&RENDER_FIXED_8X8_TEXT_OFFSET), Some(&2));
        assert_eq!(
            target_counts.get(&RENDER_FONT_STRING_WIDTH_OFFSET),
            Some(&3)
        );
        assert_eq!(target_counts.get(&RENDER_UI_TEXT_OFFSET), Some(&6));
        assert_eq!(
            target_counts.get(&RENDER_FRAMEBUFFER_RECT_REMAP_OFFSET),
            Some(&7)
        );
        assert_eq!(target_counts.get(&RENDER_PLANAR_UI_TEXT_OFFSET), Some(&1));
        assert_eq!(
            target_counts.get(&RENDER_PLANAR_DIALOGUE_TEXT_OFFSET),
            Some(&1)
        );
        assert_eq!(target_counts.get(&RENDER_STRING_OFFSET), Some(&5));
        assert_eq!(target_counts.get(&RENDER_SUBTITLE_REVEAL_OFFSET), Some(&1));
        assert_eq!(target_counts.get(&RENDER_SMALL_TEXT_OFFSET), Some(&8));
        assert_eq!(
            target_counts.get(&RENDER_PLANAR_HORIZONTAL_LINE_OFFSET),
            Some(&1)
        );
        assert_eq!(
            target_counts.get(&RENDER_PLANAR_VERTICAL_LINE_OFFSET),
            Some(&1)
        );
        assert_eq!(target_counts.get(&RENDER_RECT_OUTLINE_OFFSET), Some(&4));
        assert_eq!(target_counts.get(&RENDER_DITHER_RECT_FILL_OFFSET), Some(&2));
        assert_eq!(target_counts.get(&RENDER_RECT_FILL_OFFSET), Some(&7));
        assert_eq!(target_counts.get(&RENDER_SCENE_BAND_FILL_OFFSET), Some(&10));
        assert_eq!(
            target_counts.get(&RENDER_SECONDARY_BAND_FILL_OFFSET),
            Some(&5)
        );
        assert_eq!(target_counts.get(&RENDER_FRAMEBUFFER_COPY_OFFSET), Some(&4));
        assert_eq!(
            target_counts.get(&RENDER_SECONDARY_FRAMEBUFFER_COPY_OFFSET),
            Some(&1)
        );
        assert_eq!(
            target_counts.get(&RENDER_VGA_PLANAR_TO_LINEAR_COPY_OFFSET),
            Some(&1)
        );
        assert_eq!(target_counts.get(&RENDER_PLANAR_COPY_OFFSET), Some(&6));
        assert_eq!(
            target_counts.get(&RENDER_RESOURCE_FILE_LOAD_OFFSET),
            Some(&4)
        );
        assert_eq!(
            target_counts.get(&RENDER_SPRITE_SLOT_DIRTY_RANGE_RENDER_OFFSET),
            Some(&7)
        );

        let target_name = |target_offset| {
            sites
                .iter()
                .find(|site| site.target_offset == target_offset)
                .map(|site| site.target_name)
                .expect("render target")
        };
        assert_eq!(
            target_name(RENDER_VGA_DAC_PALETTE_LOAD_OFFSET),
            "vga_dac_palette_load"
        );
        assert_eq!(
            target_name(RENDER_VGA_DAC_PALETTE_CLEAR_OFFSET),
            "vga_dac_palette_clear"
        );
        assert_eq!(
            target_name(RENDER_FIXED_8X8_TEXT_OFFSET),
            "fixed_8x8_text_render"
        );
        assert_eq!(
            target_name(RENDER_FONT_STRING_WIDTH_OFFSET),
            "font_string_width_measure"
        );
        assert_eq!(target_name(RENDER_UI_TEXT_OFFSET), "ui_text_render_10row");
        assert_eq!(
            target_name(RENDER_FRAMEBUFFER_RECT_REMAP_OFFSET),
            "framebuffer_rect_palette_remap"
        );
        assert_eq!(
            target_name(RENDER_PLANAR_UI_TEXT_OFFSET),
            "planar_ui_text_render_10row"
        );
        assert_eq!(
            target_name(RENDER_PLANAR_DIALOGUE_TEXT_OFFSET),
            "planar_dialogue_text_render"
        );
        assert_eq!(target_name(RENDER_SMALL_TEXT_OFFSET), "small_text_render");
        assert_eq!(
            target_name(RENDER_PLANAR_HORIZONTAL_LINE_OFFSET),
            "planar_horizontal_line_draw"
        );
        assert_eq!(
            target_name(RENDER_PLANAR_VERTICAL_LINE_OFFSET),
            "planar_vertical_line_draw"
        );
        assert_eq!(
            target_name(RENDER_RECT_OUTLINE_OFFSET),
            "framebuffer_rect_outline"
        );
        assert_eq!(
            target_name(RENDER_DITHER_RECT_FILL_OFFSET),
            "framebuffer_dither_rect_fill"
        );
        assert_eq!(
            target_name(RENDER_RECT_FILL_OFFSET),
            "framebuffer_rect_fill_clipped"
        );
        assert_eq!(
            target_name(RENDER_SECONDARY_BAND_FILL_OFFSET),
            "secondary_framebuffer_band_fill"
        );
        assert_eq!(
            target_name(RENDER_FRAMEBUFFER_COPY_OFFSET),
            "framebuffer_copy_full"
        );
        assert_eq!(
            target_name(RENDER_SECONDARY_FRAMEBUFFER_COPY_OFFSET),
            "secondary_framebuffer_copy_full"
        );
        assert_eq!(
            target_name(RENDER_VGA_PLANAR_TO_LINEAR_COPY_OFFSET),
            "vga_planar_to_linear_framebuffer_copy"
        );
        assert_eq!(
            target_name(RENDER_PLANAR_COPY_OFFSET),
            "planar_framebuffer_copy"
        );
        assert_eq!(
            target_name(RENDER_RESOURCE_FILE_LOAD_OFFSET),
            "resource_file_payload_load"
        );
        assert_eq!(
            target_name(RENDER_SPRITE_SLOT_RESOURCE_LOAD_OFFSET),
            "sprite_slot_resource_frame_load"
        );
        assert_eq!(
            target_name(RENDER_SPRITE_SLOT_POSITION_OFFSET),
            "sprite_slot_position_update"
        );
        assert_eq!(
            target_name(RENDER_SPRITE_SLOT_RANGE_DIRTY_OFFSET),
            "sprite_slot_range_mark_dirty"
        );
        assert_eq!(
            target_name(RENDER_SPRITE_SLOT_EXTENT_OFFSET),
            "sprite_slot_extent_update"
        );
        assert_eq!(
            target_name(RENDER_SPRITE_SLOT_COMMIT_RANGE_OFFSET),
            "sprite_slot_commit_dirty_range"
        );
        assert_eq!(
            target_name(RENDER_DIRTY_RECTS_COPY_OFFSET),
            "dirty_rects_copy_secondary_to_primary"
        );
        assert_eq!(
            target_name(RENDER_SPRITE_SLOT_DIRTY_RANGE_RENDER_OFFSET),
            "sprite_slot_dirty_range_render"
        );

        let palette_load = sites
            .iter()
            .find(|site| site.file_offset == 0x0016b0)
            .expect("startup palette load call");
        assert_eq!(
            palette_load.target_offset,
            RENDER_VGA_DAC_PALETTE_LOAD_OFFSET
        );
        assert_eq!(palette_load.target_file_offset, 0x002f90);
        assert!(palette_load.note.contains("DS:0x5B58"));

        let palette_clear = sites
            .iter()
            .find(|site| site.file_offset == 0x000c5a)
            .expect("video setup palette clear call");
        assert_eq!(
            palette_clear.target_offset,
            RENDER_VGA_DAC_PALETTE_CLEAR_OFFSET
        );
        assert_eq!(palette_clear.target_file_offset, 0x002fa6);
        assert!(palette_clear.note.contains("VGA DAC"));

        let fixed_text = sites
            .iter()
            .find(|site| site.file_offset == 0x0016c9)
            .expect("startup fixed text render call");
        assert_eq!(fixed_text.target_offset, RENDER_FIXED_8X8_TEXT_OFFSET);
        assert_eq!(fixed_text.ax_value, Some(0x82));
        assert!(fixed_text.note.contains("fixed 8x8 text"));

        let width_measure = sites
            .iter()
            .find(|site| site.file_offset == 0x007329)
            .expect("dialogue width measure call");
        assert_eq!(width_measure.target_offset, RENDER_FONT_STRING_WIDTH_OFFSET);
        assert_eq!(width_measure.ax_value, Some(1));
        assert!(width_measure.note.contains("dialogue font"));

        let ui_text = sites
            .iter()
            .find(|site| site.file_offset == 0x008597)
            .expect("dialogue/menu UI text render call");
        assert_eq!(ui_text.target_offset, RENDER_UI_TEXT_OFFSET);
        assert_eq!(ui_text.target_name, "ui_text_render_10row");
        assert!(ui_text.note.contains("10-row UI text"));

        let rect_remap = sites
            .iter()
            .find(|site| site.file_offset == 0x0078c4)
            .expect("framebuffer palette remap call");
        assert_eq!(
            rect_remap.target_offset,
            RENDER_FRAMEBUFFER_RECT_REMAP_OFFSET
        );
        assert_eq!(rect_remap.target_name, "framebuffer_rect_palette_remap");
        assert!(rect_remap.note.contains("palette remap"));

        let planar_dialogue_text = sites
            .iter()
            .find(|site| site.file_offset == 0x0072f6)
            .expect("dialogue planar text render call");
        assert_eq!(
            planar_dialogue_text.target_offset,
            RENDER_PLANAR_DIALOGUE_TEXT_OFFSET
        );
        assert!(planar_dialogue_text.note.contains("GS:0x5219"));

        let horizontal_line = sites
            .iter()
            .find(|site| site.file_offset == 0x00946d)
            .expect("dialogue horizontal line draw call");
        assert_eq!(
            horizontal_line.target_offset,
            RENDER_PLANAR_HORIZONTAL_LINE_OFFSET
        );
        assert!(horizontal_line.note.contains("horizontal line"));

        let rect_outline = sites
            .iter()
            .find(|site| site.file_offset == 0x007a62)
            .expect("navigation rectangle outline call");
        assert_eq!(rect_outline.target_offset, RENDER_RECT_OUTLINE_OFFSET);
        assert_eq!(rect_outline.ax_value, Some(0xef));
        assert_eq!(rect_outline.target_name, "framebuffer_rect_outline");

        let dither_fill = sites
            .iter()
            .find(|site| site.file_offset == 0x007b18)
            .expect("navigation dither rectangle fill call");
        assert_eq!(dither_fill.target_offset, RENDER_DITHER_RECT_FILL_OFFSET);
        assert_eq!(dither_fill.ax_value, Some(3));
        assert!(dither_fill.note.contains("black/0xEF"));

        let planar_capture = sites
            .iter()
            .find(|site| site.file_offset == 0x008d14)
            .expect("VGA planar-to-linear capture call");
        assert_eq!(
            planar_capture.target_offset,
            RENDER_VGA_PLANAR_TO_LINEAR_COPY_OFFSET
        );
        assert_eq!(planar_capture.ax_value, Some(0xa000));
        assert!(planar_capture.note.contains("A000:0xC000"));

        let direct_resource_load = sites
            .iter()
            .find(|site| site.file_offset == 0x00597f)
            .expect("direct-buffer resource payload load call");
        assert_eq!(
            direct_resource_load.target_offset,
            RENDER_RESOURCE_FILE_LOAD_OFFSET
        );
        assert_eq!(direct_resource_load.ax_value, Some(0x8007));
        assert!(direct_resource_load.note.contains("ES:DI"));

        let resource_load = sites
            .iter()
            .find(|site| site.file_offset == 0x008d96)
            .expect("resource-backed sprite slot load call");
        assert_eq!(
            resource_load.target_offset,
            RENDER_SPRITE_SLOT_RESOURCE_LOAD_OFFSET
        );
        assert_eq!(resource_load.ax_value, Some(5));
        assert!(resource_load.note.contains("GS:0x6212"));

        let slot_position = sites
            .iter()
            .find(|site| site.file_offset == 0x009cef)
            .expect("sprite slot position update call");
        assert_eq!(
            slot_position.target_offset,
            RENDER_SPRITE_SLOT_POSITION_OFFSET
        );
        assert!(slot_position.note.contains("+0x08/+0x0A"));

        let slot_range_dirty = sites
            .iter()
            .find(|site| site.file_offset == 0x008ad4)
            .expect("sprite slot range dirty call");
        assert_eq!(
            slot_range_dirty.target_offset,
            RENDER_SPRITE_SLOT_RANGE_DIRTY_OFFSET
        );
        assert_eq!(slot_range_dirty.ax_value, Some(21));

        let slot_extent = sites
            .iter()
            .find(|site| site.file_offset == 0x00926d)
            .expect("sprite slot extent update call");
        assert_eq!(slot_extent.target_offset, RENDER_SPRITE_SLOT_EXTENT_OFFSET);
        assert_eq!(slot_extent.ax_value, Some(0));
        assert!(slot_extent.note.contains("+0x0C/+0x0E"));

        let slot_commit = sites
            .iter()
            .find(|site| site.file_offset == 0x009575)
            .expect("sprite slot commit range call");
        assert_eq!(
            slot_commit.target_offset,
            RENDER_SPRITE_SLOT_COMMIT_RANGE_OFFSET
        );
        assert_eq!(slot_commit.ax_value, Some(21));

        let slot_render = sites
            .iter()
            .find(|site| site.file_offset == 0x00957a)
            .expect("dirty sprite slot range render call");
        assert_eq!(
            slot_render.target_offset,
            RENDER_SPRITE_SLOT_DIRTY_RANGE_RENDER_OFFSET
        );
        assert_eq!(slot_render.ax_value, Some(21));
        assert!(slot_render.note.contains("dirty rectangles"));

        let dirty_copy = sites
            .iter()
            .find(|site| site.file_offset == 0x00b1d8)
            .expect("dirty rect copyback call");
        assert_eq!(dirty_copy.target_offset, RENDER_DIRTY_RECTS_COPY_OFFSET);
        assert!(dirty_copy.note.contains("GS:0x5229"));

        let subtitle_reveal = sites
            .iter()
            .find(|site| site.file_offset == 0x0094ee)
            .expect("subtitle reveal render call");
        assert_eq!(subtitle_reveal.target_offset, RENDER_SUBTITLE_REVEAL_OFFSET);
        assert_eq!(subtitle_reveal.target_name, "subtitle_reveal_draw_wrapper");
        assert_eq!(subtitle_reveal.ax_source, "unresolved");
        assert_eq!(subtitle_reveal.intervening_far_calls, 0);
        assert!(subtitle_reveal.note.contains("DS:0x5E5C/0x5E5E"));

        let sprite_load = sites
            .iter()
            .find(|site| site.file_offset == 0x007e7d)
            .expect("actor subdispatch sprite load call");
        assert_eq!(sprite_load.target_offset, RENDER_SPRITE_SLOT_LOAD_OFFSET);
        assert_eq!(sprite_load.ax_value, Some(4));
        assert_eq!(sprite_load.target_name, "sprite_slot_frame_load");

        let sprite_state = sites
            .iter()
            .find(|site| site.file_offset == 0x0059dc)
            .expect("VM presentation clear sprite-state call");
        assert_eq!(sprite_state.target_offset, RENDER_SPRITE_SLOT_STATE_OFFSET);
        assert_eq!(sprite_state.ax_value, Some(4));
        assert!(sprite_state.note.contains("presentation clear"));
    }

    #[test]
    fn sprite_blitter_dispatch_table_recovers_internal_modes() {
        let Some(binary) = fixture() else {
            eprintln!("skipping: BLOODPRG.EXE not available");
            return;
        };

        let entries = binary
            .sprite_blitter_dispatch_entries()
            .expect("sprite blitter dispatch table");
        let expected = [
            (
                0,
                RENDER_SPRITE_BLIT_RAW_TRANSPARENT_OFFSET,
                0x004536,
                "sprite_blit_raw_transparent",
            ),
            (
                1,
                RENDER_SPRITE_BLIT_RLE_TRANSPARENT_OFFSET,
                0x0046bc,
                "sprite_blit_rle_transparent",
            ),
            (
                2,
                RENDER_SPRITE_BLIT_RAW_OPAQUE_OFFSET,
                0x004ba8,
                "sprite_blit_raw_opaque",
            ),
            (
                3,
                RENDER_SPRITE_BLIT_RLE_OPAQUE_OFFSET,
                0x004cd6,
                "sprite_blit_rle_opaque",
            ),
            (
                4,
                RENDER_SPRITE_BLIT_SCALED_TRANSPARENT_OFFSET,
                0x004f62,
                "sprite_blit_scaled_transparent",
            ),
            (
                5,
                RENDER_SPRITE_BLIT_NOOP_5_OFFSET,
                0x00509a,
                "sprite_blit_noop_mode5",
            ),
            (
                6,
                RENDER_SPRITE_BLIT_NOOP_6_OFFSET,
                0x00509b,
                "sprite_blit_noop_mode6",
            ),
            (
                7,
                RENDER_SPRITE_BLIT_NOOP_7_OFFSET,
                0x00509c,
                "sprite_blit_noop_mode7",
            ),
        ];

        assert_eq!(entries.len(), RENDER_SPRITE_BLITTER_ENTRY_COUNT);
        for (entry, (mode, handler_offset, handler_file_offset, name)) in
            entries.iter().zip(expected)
        {
            assert_eq!(entry.mode, mode);
            assert_eq!(entry.handler_offset, handler_offset);
            assert_eq!(entry.handler_file_offset, handler_file_offset);
            assert_eq!(entry.name, name);
            assert_eq!(
                entry.handler_file_offset,
                binary.segoff_to_file(RENDER_SEGMENT, handler_offset)
            );
        }

        assert!(entries[0].note.contains("transparent"));
        assert!(entries[1].note.contains("RLE"));
        assert!(entries[2].note.contains("opaque"));
        assert!(entries[4].note.contains("scaled"));
        for entry in &entries[5..] {
            let opcode = binary
                .slice(entry.handler_file_offset, 1, "sprite noop handler")
                .expect("sprite noop handler");
            assert_eq!(opcode, b"\xc3");
        }

        let inspect = binary.inspect().expect("inspection");
        assert_eq!(inspect.sprite_blitter_dispatch, entries);

        let symbol = KNOWN_SYMBOLS
            .iter()
            .find(|symbol| symbol.name == "sprite_blitter_dispatch_table")
            .expect("sprite blitter table symbol");
        assert_eq!(symbol.file_offset, RENDER_SPRITE_BLITTER_TABLE_FILE_OFFSET);
        assert_eq!(symbol.offset, Some(RENDER_SPRITE_BLITTER_TABLE_OFFSET));
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
