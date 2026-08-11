#!/usr/bin/env python3
"""Export recovered native routines as assembly plus Borland C++ work files.

This is a decompilation workspace generator, not a decompiler.  It preserves a
one-to-one mapping between each currently recovered routine entry and:

* an assembly dump rooted at the recovered entrypoint
* a Borland C++ source file for the future faithful translation

The C++ side deliberately fails to compile for untranslated routines.  That is
intentional: a compile-clean no-op body would destroy the evidence trail and make
later DOSBox runs meaningless.
"""

from __future__ import annotations

import argparse
import collections
import csv
import hashlib
import json
import os
import re
import shutil
import struct
import sys
from pathlib import Path
from typing import Iterable

_HERE_STR = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE_STR
]

from dataclasses import dataclass, field

import capstone
from capstone.x86_const import X86_OP_IMM

_HERE = Path(_HERE_STR)
sys.path.insert(0, str(_HERE))

from indirect_dispatch_atlas import STATIC_TABLES  # noqa: E402
from mzfile import MZ, load_labels  # noqa: E402

sys.path[:] = [
    path for path in sys.path if Path(path or os.curdir).resolve() != _HERE
]


RE_ROOT = _HERE.parent
PROJECT_ROOT = RE_ROOT.parent
DEFAULT_BLOODPRG = RE_ROOT / "bin" / "BLOODPRG.EXE"
DEFAULT_GRAPH = RE_ROOT / "func_graph.json"
DEFAULT_XDB_DIR = PROJECT_ROOT / "output" / "_tmp_dat"
DEFAULT_ASM_OUT = RE_ROOT / "assembly"
DEFAULT_CPP_OUT = RE_ROOT / "borland"
DEFAULT_MANIFEST = RE_ROOT / "routine_recovery_manifest.json"

RETURN_MNEMONICS = {"ret", "retf"}

ALIEN_XDBS = {"amer", "croolis", "scrut"}

GS_BYTE_STORE_IMM8_TRANSLATIONS = {
    ("bloodprg", 0x006855): (0x67B4, 1, "VM opcode 0xaa yield flag store"),
    ("bloodprg", 0x00685C): (0x67B4, 1, "VM opcode 0xac yield flag store"),
    ("bloodprg", 0x007542): (0x0B16, 1, "byte parser dispatch 0x01 flag store"),
    ("bloodprg", 0x007549): (0x0B16, 1, "byte parser dispatch 0x02 flag store"),
    ("bloodprg", 0x007550): (0x0B16, 1, "byte parser dispatch 0x0f flag store"),
    ("bloodprg", 0x007557): (0x0B16, 1, "byte parser dispatch 0x04 flag store"),
}

XDB_ACTOR_FIELD_SUB_TRANSLATIONS = {
    ("xdb_amer", 0x000B0F): 0x1BC2,
    ("xdb_croolis", 0x000B50): 0x1B2E,
    ("xdb_scrut", 0x000B55): 0x1BE3,
    ("xdb_scrut", 0x000B65): None,
}

XDB_ADD_CS99_IF_NONNEG_TRANSLATIONS = {
    ("xdb_amer", 0x000B1F),
    ("xdb_croolis", 0x000B60),
}

XDB_JUMP_OR_INIT_TRANSLATIONS = {
    ("xdb_amer", 0x001BEA): 0x1C34,
    ("xdb_croolis", 0x001B46): 0x1B85,
    ("xdb_scrut", 0x001BFB): 0x1C45,
}

BLOODPRG_CONTROL_BYTE_COPY_TRANSLATIONS = {
    ("bloodprg", 0x007629): 0x20B8,
    ("bloodprg", 0x00766F): 0x24C6,
    ("bloodprg", 0x0076C0): 0x2460,
    ("bloodprg", 0x0076D5): 0x247A,
}

BLOODPRG_VM_FLAG_BRANCH_TRANSLATIONS = {
    ("bloodprg", 0x006494): 0x2793,
    ("bloodprg", 0x0064A0): 0x252A,
    ("bloodprg", 0x0064AC): 0x274F,
}

XDB_MOUSE_RANGE_TRANSLATIONS = {
    ("xdb_amer", 0x000336),
    ("xdb_croolis", 0x00034B),
    ("xdb_scrut", 0x00034B),
}

XDB_MOUSE_POSITION_TRANSLATIONS = {
    ("xdb_amer", 0x000347),
    ("xdb_croolis", 0x00035C),
    ("xdb_scrut", 0x00035C),
}

XDB_FIELD_DELTA_TRANSLATIONS = {
    ("xdb_amer", 0x001B5F),
    ("xdb_croolis", 0x001ACB),
    ("xdb_scrut", 0x001B80),
}

XDB_FIELD_DELTA_SAR4_TRANSLATIONS = {
    ("xdb_amer", 0x001B8F),
    ("xdb_croolis", 0x001AFB),
    ("xdb_scrut", 0x001BB0),
}

BLOODPRG_FULL_DWORD_COPY_TRANSLATIONS = {
    ("bloodprg", 0x003E46): 0x5221,
    ("bloodprg", 0x003E5B): 0x5229,
}

MANUAL_TRANSLATIONS = {
    ("bloodprg", 0x00093B): (
        "translated_rtc_time_read",
        "mechanical translation of BIOS RTC time read plus BCD conversion call",
    ),
    ("bloodprg", 0x000950): (
        "translated_rtc_date_read",
        "mechanical translation of BIOS RTC date read plus decoded GS stores",
    ),
    ("bloodprg", 0x000B32): (
        "translated_detect_cdrom",
        "mechanical translation of MSCDEX int 2fh probe and GS flag store",
    ),
    ("bloodprg", 0x000BFF): (
        "translated_install_ctrl_break_handler",
        "mechanical translation of DOS int 21h vector setup preserving AX/DX/DS",
    ),
    ("bloodprg", 0x000CC0): (
        "translated_set_video_mode_saved",
        "mechanical translation of BIOS int 10h video mode restore preserving AX",
    ),
    ("bloodprg", 0x000CEF): (
        "translated_mouse_reset_hide",
        "mechanical translation of mouse int 33h reset/hide/mickey-ratio setup",
    ),
    ("bloodprg", 0x000D0E): (
        "translated_poll_mouse",
        "mechanical translation of mouse int 33h poll and movement-change stores",
    ),
    ("bloodprg", 0x000D4A): (
        "translated_mouse_set_hrange",
        "mechanical translation of mouse int 33h horizontal/vertical range setup",
    ),
    ("bloodprg", 0x000D61): (
        "translated_print_string_dos",
        "mechanical translation of DOS int 21h character-output loop",
    ),
    ("bloodprg", 0x0025A4): (
        "translated_string_compare",
        "mechanical translation of saved-register byte string compare",
    ),
    ("bloodprg", 0x00267D): (
        "translated_kbd_read_int16",
        "mechanical translation of BIOS int 16h keyboard poll/read",
    ),
    ("bloodprg", 0x002DD3): (
        "translated_cmos_rtc_read",
        "mechanical translation of CMOS port select/read and CS state store",
    ),
    ("bloodprg", 0x002F90): (
        "translated_vga_palette_write",
        "mechanical translation of VGA DAC port upload loop",
    ),
    ("bloodprg", 0x002FA6): (
        "translated_vga_dac_clear",
        "mechanical translation of VGA DAC zero-fill loop",
    ),
    **{
        key: (
            "translated_fullscreen_dword_copy",
            f"mechanical translation of full-screen REP MOVSD to GS far pointer {dest:#06x}",
        )
        for key, dest in BLOODPRG_FULL_DWORD_COPY_TRANSLATIONS.items()
    },
    ("bloodprg", 0x001FBC): (
        "translated_flag_test_a2e",
        "mechanical translation of two flag-bit propagation blocks",
    ),
    ("bloodprg", 0x005320): (
        "translated_resource_handle_resolve",
        "mechanical translation of FS resource-handle table resolver",
    ),
    ("bloodprg", 0x005288): (
        "translated_resource_release",
        "mechanical translation of resource loaded-flag test plus CS-pushed free call",
    ),
    ("bloodprg", 0x00178B): (
        "translated_render_present_if_dirty",
        "mechanical translation of dirty-flag gated display far-call sequence",
    ),
    ("bloodprg", 0x001397): (
        "translated_flag_gated_ae6_a",
        "mechanical translation of MSCDEX drive-status gate preserving saved registers",
    ),
    ("bloodprg", 0x003B45): (
        "translated_composite_draw_a",
        "mechanical translation of same-segment far-return draw wrapper",
    ),
    ("bloodprg", 0x0027C3): (
        "translated_set_ds_gs_check_ae0",
        "mechanical translation of GS DOS-drive setup gate preserving AX/DX/DS",
    ),
    ("bloodprg", 0x0027E9): (
        "translated_dos_set_drive_and_chdir",
        "mechanical translation of GS DOS-drive restore gate preserving AX/DX/DS",
    ),
    ("bloodprg", 0x00577A): (
        "translated_value_scan_match",
        "mechanical translation of linked value scan preserving SI",
    ),
    ("bloodprg", 0x005FD8): (
        "translated_vm_special_slot_remove",
        "mechanical translation of 16-word sentinel-list remove preserving loop flags",
    ),
    ("bloodprg", 0x005FF6): (
        "translated_vm_special_slot_insert",
        "mechanical translation of 16-word sentinel-list insert/present probe",
    ),
    ("bloodprg", 0x006023): (
        "translated_vm_field_offset",
        "mechanical translation of selector/kind bit-scan field-offset table lookup",
    ),
    ("bloodprg", 0x006293): (
        "translated_vm_token_special",
        "mechanical translation of VM token stream scanner",
    ),
    **{
        key: (
            "translated_vm_flag_clear_branch",
            f"mechanical translation of TEST GS:{off:#06x},1 plus conditional call to VM branch helper",
        )
        for key, off in BLOODPRG_VM_FLAG_BRANCH_TRANSLATIONS.items()
    },
    ("bloodprg", 0x006588): (
        "translated_vm_op_a2_random_branch",
        "mechanical translation of LODSW, PRNG far call, OR AX,AX, conditional VM branch call",
    ),
    ("bloodprg", 0x0064B8): (
        "translated_vm_op_d2_script_profile_request",
        "mechanical translation of lodsb/cbw/dec plus GS profile request store",
    ),
    ("bloodprg", 0x0064C0): (
        "translated_vm_op_cf_clear_state",
        "mechanical translation of two GS state-clearing stores",
    ),
    ("bloodprg", 0x0064CE): (
        "translated_vm_op_cc_set_record_byte",
        "mechanical translation of BP-indexed byte copy loop",
    ),
    ("bloodprg", 0x006559): (
        "translated_vm_op_a0_push",
        "mechanical translation of VM stack push handler",
    ),
    ("bloodprg", 0x006572): (
        "translated_vm_op_a1_pop",
        "mechanical translation of VM stack pop handler",
    ),
    ("bloodprg", 0x0065DB): (
        "translated_vm_op_a4_jump",
        "mechanical translation of VM jump handler",
    ),
    ("bloodprg", 0x0067BA): (
        "translated_vm_op_a7_set_if_presentation",
        "mechanical translation of conditional presentation-state store",
    ),
    ("bloodprg", 0x00684C): (
        "translated_vm_op_ab_poke_byte",
        "mechanical translation of VM byte poke handler",
    ),
    ("bloodprg", 0x006830): (
        "translated_vm_op_a9_cond_jump",
        "mechanical translation of VM A9 conditional jump handler",
    ),
    **{
        key: (
            "translated_gs_byte_store_imm8",
            f"mechanical translation of {note} to GS:{off:#06x}",
        )
        for key, (off, _value, note) in GS_BYTE_STORE_IMM8_TRANSLATIONS.items()
    },
    ("bloodprg", 0x007612): (
        "translated_credit_presenter_b_cryo_copy",
        "mechanical translation of NUL-terminated copy to ES:0x0e18 plus GS state stores",
    ),
    **{
        key: (
            "translated_control_byte_copy",
            f"mechanical translation of control-byte-terminated copy to ES:{dest:#06x}",
        )
        for key, dest in BLOODPRG_CONTROL_BYTE_COPY_TRANSLATIONS.items()
    },
    ("bloodprg", 0x007754): (
        "translated_gs_cursor_control_byte_copy",
        "mechanical translation of GS-cursor control-byte copy and cursor advance",
    ),
    ("bloodprg", 0x007776): (
        "translated_prefixed_string_append",
        "mechanical translation of MOVSW-prefixed string append through GS cursor",
    ),
    ("bloodprg", 0x007788): (
        "translated_fs_name_area_read",
        "mechanical translation of ES=FS control-byte copy preserving ES",
    ),
    ("bloodprg", 0x0076BA): (
        "translated_lodsw_store_gs_1fa5",
        "mechanical translation of lodsw plus GS:0x1fa5 store",
    ),
    ("bloodprg", 0x009F80): (
        "translated_lookup_table_1fb5",
        "mechanical translation of AX*4 table lookup at DS:0x1fb5",
    ),
    ("bloodprg", 0x00A141): (
        "translated_close_file_d5b",
        "mechanical translation of DOS close gate plus list bound reset call",
    ),
    ("bloodprg", 0x00A622): (
        "translated_list_d8c_read",
        "mechanical translation of queue read helper and conditional LES result fetch",
    ),
    ("bloodprg", 0x00A2DD): (
        "translated_resource_empty_close_gate",
        "mechanical translation of queue-empty flag update plus close call",
    ),
    ("bloodprg", 0x008713): (
        "translated_nav_choice_handler_0",
        "mechanical translation of phase-bit guarded navigation state stores",
    ),
    ("bloodprg", 0x001D74): (
        "translated_copy_abc_to_671c",
        "mechanical translation of GS far-pointer record copy loop",
    ),
    ("bloodprg", 0x0041D1): (
        "translated_entity_flag_state_transition",
        "mechanical translation of GS entity flag state transition preserving AX/BX",
    ),
    ("bloodprg", 0x004240): (
        "translated_range_count",
        "mechanical translation of inclusive range entity-flag update loop",
    ),
    ("bloodprg", 0x008269): (
        "translated_mouse_hit_test",
        "mechanical translation of DS:SI rectangle hit test and SS:BP flag OR",
    ),
    ("bloodprg", 0x008295): (
        "translated_region_record_hittest",
        "mechanical translation of SS:BP rectangle hit test returning carry",
    ),
    ("bloodprg", 0x008848): (
        "translated_nav_choice_handler_3",
        "mechanical translation of phase-bit guarded navigation state stores plus radio far call",
    ),
    ("bloodprg", 0x00963F): (
        "translated_matrix_table_clear_2a1b",
        "mechanical translation of six SS:BP-stride zero stores preserving pushed registers",
    ),
    ("bloodprg", 0x008C96): (
        "translated_vm_segment_call_wrapper",
        "mechanical translation of VM far-call wrapper and GS dword-copy postamble",
    ),
    ("bloodprg", 0x00933A): (
        "translated_back_buffer_copy_from",
        "mechanical translation of GS far-pointer row copy to back buffer",
    ),
    ("bloodprg", 0x002665): (
        "translated_strlen_es_di",
        "mechanical translation of ES:DI NUL scan preserving CX/DI",
    ),
    ("bloodprg", 0x00A40B): (
        "translated_gs_d5f_compare_zero_or_one",
        "mechanical translation of two-stage GS:0x0d5f byte compare",
    ),
    ("bloodprg", 0x00A117): (
        "translated_flag_gated_2751_copy",
        "mechanical translation of GS:0x2751-gated 0x60 dword copy",
    ),
    ("bloodprg", 0x00A38E): (
        "translated_queue_d8c_wrap",
        "mechanical translation of ring-buffer pointer wrap and count update",
    ),
    ("bloodprg", 0x00A3AD): (
        "translated_queue_d8c_empty_check",
        "mechanical translation of queue head/tail/capacity comparisons",
    ),
    ("bloodprg", 0x00A634): (
        "translated_flag_test_b17",
        "mechanical translation of DS=GS flag-byte test preserving AX/DS",
    ),
    ("bloodprg", 0x00A734): (
        "translated_queue_d8c_enqueue",
        "mechanical translation of two DS queue adds plus CLC",
    ),
    ("bloodprg", 0x00A73E): (
        "translated_list_d8c_bounds_init",
        "mechanical translation of list bound initialization plus fall-through tail stores",
    ),
    ("bloodprg", 0x00A744): (
        "translated_list_d8c_bounds_tail",
        "mechanical translation of list bound tail stores at DS:0x0d62..0x0d66",
    ),
    ("bloodprg", 0x00A757): (
        "translated_list_d8c_init",
        "mechanical translation of list D8C initialization stores",
    ),
    ("bloodprg", 0x00A778): (
        "translated_list_d8c_call_a0c3",
        "mechanical translation of LES setup plus near call to 0xa0c3",
    ),
    ("bloodprg", 0x00AD96): (
        "translated_gfx_scanline_advance",
        "mechanical translation of row-counter/scanline advance with zero-row epilogue",
    ),
    ("bloodprg", 0x00A7E6): (
        "translated_mem_copy_words_4",
        "mechanical translation of push ds/pop es plus four MOVSW instructions",
    ),
    ("bloodprg", 0x00BB9D): (
        "translated_snd_driver_call",
        "mechanical translation of GS-based indirect sound-driver far call",
    ),
    ("bloodprg", 0x00BD8D): (
        "translated_ems_page_offset_split",
        "mechanical translation of DOS seek/read EMS-page split helper",
    ),
    **{
        key: (
            "translated_xdb_actor_field_sub_0f",
            "mechanical translation of XDB actor field subtract and optional CS slot update",
        )
        for key in XDB_ACTOR_FIELD_SUB_TRANSLATIONS
    },
    **{
        key: (
            "translated_xdb_add_cs99_if_nonnegative",
            "mechanical translation of XDB CS:0x99 SAR/JS/add sequence",
        )
        for key in XDB_ADD_CS99_IF_NONNEG_TRANSLATIONS
    },
    **{
        key: (
            "translated_xdb_jump_or_init_method",
            "mechanical translation of XDB method jump-or-initialize sequence",
        )
        for key in XDB_JUMP_OR_INIT_TRANSLATIONS
    },
    **{
        key: (
            "translated_xdb_mouse_range",
            "mechanical translation of XDB int 33h mouse range helper",
        )
        for key in XDB_MOUSE_RANGE_TRANSLATIONS
    },
    **{
        key: (
            "translated_xdb_mouse_position",
            "mechanical translation of XDB int 33h mouse position helper",
        )
        for key in XDB_MOUSE_POSITION_TRANSLATIONS
    },
    **{
        key: (
            "translated_xdb_field_delta",
            "mechanical translation of XDB slot field-delta propagation loop",
        )
        for key in XDB_FIELD_DELTA_TRANSLATIONS
    },
    **{
        key: (
            "translated_xdb_field_delta_sar4",
            "mechanical translation of XDB slot SAR4 field-delta propagation loop",
        )
        for key in XDB_FIELD_DELTA_SAR4_TRANSLATIONS
    },
    ("xdb_manu3", 0x00017C): (
        "translated_manu3_selector_wrapper",
        "mechanical translation of near call to MANU3 selector followed by far return",
    ),
    ("xdb_manu3", 0x000181): (
        "translated_manu3_anim_select",
        "mechanical translation of MANU3 sequence-table selection and tail jump",
    ),
}

MANU3_CODE_SEEDS = {
    0x0000: "manu3 external far-call API entry",
    0x0121: "manu3 self-relocation/init entry",
    0x0150: "manu3 no-cursor per-frame entry",
    0x017C: "manu3 selector wrapper entry",
    0x0181: "manu3 animation selector",
    0x019B: "manu3 tween stepper",
    0x01DF: "manu3 tween constructor",
    0x0270: "manu3 matrix builder",
    0x0477: "manu3 transform routine",
    0x0549: "manu3 entity projector",
    0x06F6: "manu3 face builder",
    0x0700: "manu3 face bucket sorter",
    0x0775: "manu3 span renderer init",
    0x0848: "manu3 span setup region",
    0x0849: "manu3 span insertion routine",
    0x0C2A: "manu3 affine fill routine",
    0x0D7D: "manu3 face activation routine",
    0x0D93: "manu3 gradient setup routine",
}


def h(n: int, width: int = 0) -> str:
    if width:
        return f"0x{n:0{width}x}"
    return f"0x{n:x}"


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def slug(text: str, fallback: str = "routine", limit: int = 64) -> str:
    text = text.lower()
    text = re.sub(r"[^a-z0-9_]+", "_", text)
    text = re.sub(r"_+", "_", text).strip("_")
    if not text:
        text = fallback
    return text[:limit].strip("_") or fallback


def rel(path: Path) -> str:
    return str(path.relative_to(PROJECT_ROOT))


def make_md() -> capstone.Cs:
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    md.detail = True
    md.skipdata = False
    return md


def immediate_operand(insn: capstone.CsInsn) -> int | None:
    if not insn.operands:
        return None
    op = insn.operands[0]
    if op.type != X86_OP_IMM:
        return None
    return int(op.imm)


def load_graph(path: Path) -> dict[str, object]:
    if not path.exists():
        return {"funcs": [], "leaves": [], "callgraph": {}, "indirect": []}
    with path.open() as fh:
        return json.load(fh)


def direct_far_transfers(mz: MZ) -> list[dict[str, int | str]]:
    transfers: list[dict[str, int | str]] = []
    for opcode, kind in ((0x9A, "call"), (0xEA, "jmp")):
        i = mz.header_size
        while i < mz.image_total - 5:
            if mz.data[i] == opcode:
                off, seg = struct.unpack_from("<HH", mz.data, i + 1)
                seg_operand_image = mz.file_to_image(i + 3)
                if seg_operand_image in mz.reloc_image_offsets:
                    transfers.append(
                        {
                            "kind": kind,
                            "site": i,
                            "target_segment": seg,
                            "target_offset": off,
                            "target_file": mz.segoff_to_file(seg, off),
                        }
                    )
            i += 1
    return sorted(transfers, key=lambda x: (int(x["site"]), str(x["kind"])))


def segment_bases_from_relocs(mz: MZ, transfers: Iterable[dict[str, int | str]]) -> list[int]:
    bases = set()
    for image_off in mz.reloc_image_offsets:
        if image_off + 1 < len(mz.image):
            bases.add(struct.unpack_from("<H", mz.image, image_off)[0])
    for transfer in transfers:
        bases.add(int(transfer["target_segment"]))
    return sorted(bases)


def segment_for_file(mz: MZ, bases: list[int], file_off: int) -> tuple[int, int]:
    image_off = mz.file_to_image(file_off)
    candidates = [seg for seg in bases if seg * 16 <= image_off]
    seg = max(candidates) if candidates else 0
    return seg, image_off - seg * 16


def static_table_targets(mz: MZ) -> dict[int, list[str]]:
    targets: dict[int, list[str]] = collections.defaultdict(list)
    for table in STATIC_TABLES:
        table_file = int(table["table_file_offset"])
        target_base = int(table["target_base_file_offset"])
        index_base = int(table["index_base"])
        index_prefix = str(table["index_prefix"])
        for idx in range(int(table["entry_count"])):
            raw = struct.unpack_from("<H", mz.data, table_file + idx * 2)[0]
            target = target_base + raw
            selector = index_base + idx
            if index_prefix in {"opcode", "byte"}:
                key = f"{index_prefix}_0x{selector:02x}"
            else:
                key = f"{index_prefix}_{selector}"
            targets[target].append(f"{table['name']}:{key}")
    return targets


@dataclass
class LabelInfo:
    name: str
    comment: str = ""


@dataclass
class Routine:
    module: str
    artifact_path: Path
    entry: int
    address_kind: str
    group: str
    provenance: set[str] = field(default_factory=set)
    labels: list[LabelInfo] = field(default_factory=list)
    incoming: list[str] = field(default_factory=list)
    seg_off: str | None = None
    instructions: list[capstone.CsInsn] = field(default_factory=list)
    direct_callees: set[int] = field(default_factory=set)
    indirect_calls: list[str] = field(default_factory=list)
    tail_jumps: set[int] = field(default_factory=set)
    boundary_reason: str = "not_decoded"
    terminal: str | None = None
    first_bytes: str = ""
    byte_count: int = 0
    cxx_status: str = "untranslated"
    cxx_reason: str = ""
    asm_path: Path | None = None
    cpp_path: Path | None = None

    @property
    def label_slug(self) -> str:
        if self.labels:
            return slug(self.labels[0].name)
        return "routine"

    @property
    def func_name(self) -> str:
        mod = slug(self.module)
        return f"cb_{mod}_{self.entry:06x}_{self.label_slug}"

    @property
    def file_stem(self) -> str:
        return f"func_{self.entry:06x}_{self.label_slug}"


def add_label(labels: list[LabelInfo], name: str, comment: str = "") -> None:
    if not name:
        return
    for label in labels:
        if label.name == name and label.comment == comment:
            return
    labels.append(LabelInfo(name=name, comment=comment))


def parse_xdb_labels(labels_csv: Path) -> dict[str, dict[int, list[LabelInfo]]]:
    by_module: dict[str, dict[int, list[LabelInfo]]] = collections.defaultdict(
        lambda: collections.defaultdict(list)
    )
    with labels_csv.open(newline="") as fh:
        for row in csv.reader(fh):
            if not row or row[0].strip().startswith("#"):
                continue
            addr = row[0].strip()
            if not addr.lower().startswith("xdb:"):
                continue
            name = row[1].strip() if len(row) > 1 else ""
            comment = row[2].strip() if len(row) > 2 else ""
            parts = addr.split(":")
            if len(parts) == 2:
                module = "manu3"
                off_s = parts[1]
            elif len(parts) == 3:
                module = parts[1].lower()
                off_s = parts[2]
            else:
                continue
            try:
                off = int(off_s, 16)
            except ValueError:
                continue
            by_module[module][off].append(LabelInfo(name=name, comment=comment))
    return {module: dict(offsets) for module, offsets in by_module.items()}


def decode_routine(
    routine: Routine,
    data: bytes,
    max_bytes: int,
    protected_entries: set[int] | None = None,
) -> None:
    if not (0 <= routine.entry < len(data)):
        routine.boundary_reason = "entry_out_of_range"
        return

    md = make_md()
    end = min(len(data), routine.entry + max_bytes)
    window = data[routine.entry:end]
    routine.first_bytes = window[:16].hex(" ")

    protected_entries = protected_entries or set()
    blocks = collections.deque([routine.entry])
    visited_blocks: set[int] = set()
    insn_by_addr: dict[int, capstone.CsInsn] = {}
    terminals: list[str] = []
    decode_stops: list[str] = []

    def enqueue(target: int) -> None:
        if not (routine.entry <= target < end):
            return
        if target in protected_entries and target != routine.entry:
            routine.tail_jumps.add(target)
            return
        if target not in visited_blocks:
            blocks.append(target)

    while blocks:
        block = blocks.popleft()
        if block in visited_blocks:
            continue
        visited_blocks.add(block)
        pos = block
        while pos < end:
            if pos in insn_by_addr:
                break
            decoded = list(md.disasm(data[pos:end], pos, count=1))
            if not decoded or decoded[0].address != pos:
                decode_stops.append(f"decode_stop_at_{h(pos)}")
                break
            insn = decoded[0]
            insn_by_addr[insn.address] = insn
            next_pos = insn.address + insn.size

            if insn.mnemonic == "call":
                target = immediate_operand(insn)
                if target is not None and 0 <= target < len(data):
                    routine.direct_callees.add(target)
                else:
                    routine.indirect_calls.append(
                        f"{h(insn.address)}: {insn.mnemonic} {insn.op_str}".strip()
                    )
            elif insn.mnemonic == "lcall":
                routine.indirect_calls.append(
                    f"{h(insn.address)}: {insn.mnemonic} {insn.op_str}".strip()
                )

            if insn.mnemonic in {"jmp", "ljmp"}:
                target = immediate_operand(insn)
                if target is not None and 0 <= target < len(data):
                    if routine.entry <= target < end and not (
                        target in protected_entries and target != routine.entry
                    ):
                        enqueue(target)
                    else:
                        routine.tail_jumps.add(target)
                terminals.append(f"{insn.mnemonic} {insn.op_str}".strip())
                break

            if insn.mnemonic in {"ret", "retf", "iret"}:
                terminals.append(insn.mnemonic)
                break

            if insn.group(capstone.CS_GRP_JUMP):
                target = immediate_operand(insn)
                if target is not None:
                    enqueue(target)
                enqueue(next_pos)
                break

            if next_pos in protected_entries and next_pos != routine.entry:
                break

            pos = next_pos

    routine.instructions = [insn_by_addr[addr] for addr in sorted(insn_by_addr)]
    if routine.instructions:
        last_end = max(insn.address + insn.size for insn in routine.instructions)
        routine.byte_count = max(0, last_end - routine.entry)
    else:
        routine.byte_count = 0

    terminal_counts = collections.Counter(terminals)
    if terminal_counts:
        routine.terminal = ", ".join(
            f"{term}:{count}" for term, count in sorted(terminal_counts.items())
        )
    else:
        routine.terminal = None
    if decode_stops:
        routine.boundary_reason = ",".join(sorted(set(decode_stops))[:4])
    elif blocks:
        routine.boundary_reason = f"cfg_incomplete_blocks_{len(visited_blocks)}"
    elif routine.byte_count >= max_bytes:
        routine.boundary_reason = f"max_bytes_{max_bytes}"
    else:
        routine.boundary_reason = (
            f"cfg_blocks_{len(visited_blocks)}_terminals_{sum(terminal_counts.values())}"
        )
    classify_cxx_translation(routine)


def classify_cxx_translation(routine: Routine) -> None:
    insns = routine.instructions
    manual = MANUAL_TRANSLATIONS.get((routine.module, routine.entry))
    if manual is not None:
        routine.cxx_status = manual[0]
        routine.cxx_reason = manual[1]
        return
    if len(insns) == 1 and insns[0].mnemonic in RETURN_MNEMONICS:
        routine.cxx_status = "translated_empty_return"
        routine.cxx_reason = f"single {insns[0].mnemonic} instruction"
        return
    if not insns:
        routine.cxx_status = "blocked_decode"
        routine.cxx_reason = routine.boundary_reason
        return
    routine.cxx_status = "untranslated"
    routine.cxx_reason = "requires human/mechanical translation from assembly"


def bloodprg_routines(exe: Path, graph_path: Path) -> tuple[list[Routine], dict[str, object]]:
    mz = MZ(str(exe))
    graph = load_graph(graph_path)
    _, file_labels = load_labels()
    transfers = direct_far_transfers(mz)
    bases = segment_bases_from_relocs(mz, transfers)
    by_far_target: dict[int, list[dict[str, int | str]]] = collections.defaultdict(list)
    for transfer in transfers:
        by_far_target[int(transfer["target_file"])].append(transfer)

    static_targets = static_table_targets(mz)
    graph_funcs = {int(x) for x in graph.get("funcs", [])}
    far_targets = set(by_far_target)
    all_entries = sorted(graph_funcs | far_targets | set(static_targets) | {mz.entry_file})

    routines = []
    for entry in all_entries:
        if not (0 <= entry < len(mz.data)):
            continue
        seg, off = segment_for_file(mz, bases, entry)
        routine = Routine(
            module="bloodprg",
            artifact_path=exe,
            entry=entry,
            address_kind="file_offset",
            group=f"seg_{seg:04x}",
            seg_off=f"{seg:04x}:{off:04x}",
        )
        if entry == mz.entry_file:
            routine.provenance.add("mz_entry")
        if entry in graph_funcs:
            routine.provenance.add("recursive_graph")
        if entry in by_far_target:
            routine.provenance.add("relocation_proven_far_transfer_target")
            for transfer in by_far_target[entry]:
                routine.incoming.append(
                    f"{transfer['kind']}@{h(int(transfer['site']), 6)}"
                    f"->{int(transfer['target_segment']):04x}:{int(transfer['target_offset']):04x}"
                )
        if entry in static_targets:
            routine.provenance.add("static_dispatch_table_target")
            routine.incoming.extend(static_targets[entry])
        label = file_labels.get(entry)
        if label:
            add_label(routine.labels, label[0], label[1])
        routines.append(routine)

    metadata = {
        "path": rel(exe),
        "sha256": sha256_file(exe),
        "entry_count": len(routines),
        "grouping_evidence": (
            "Grouped by recovered MZ relative segment base. This is loader/linkage "
            "evidence, not proof of original object-file translation units."
        ),
    }
    return routines, metadata


def alien_delta_pointer(data: bytes) -> int | None:
    # Entry bytes: 8c c8; 2e 03 06 <disp16>; mov ds,ax ...
    pat = bytes.fromhex("8c c8 2e 03 06")
    idx = data.find(pat, 0, min(len(data), 0x40))
    if idx < 0 or idx + len(pat) + 2 > len(data):
        return None
    return struct.unpack_from("<H", data, idx + len(pat))[0]


def alien_method_table_entries(data: bytes) -> tuple[int | None, list[tuple[int, int, int]]]:
    ptr = alien_delta_pointer(data)
    if ptr is None or ptr + 2 > len(data):
        return None, []
    delta = struct.unpack_from("<H", data, ptr)[0]
    table = delta * 16 + 0x103A
    if table < 0 or table + 2 > len(data):
        return table, []
    entries: list[tuple[int, int, int]] = []
    for idx in range(64):
        off = table + idx * 2
        if off + 2 > len(data):
            break
        target = struct.unpack_from("<H", data, off)[0]
        if target in {0x0000, 0xFFFF}:
            if idx >= 15:
                break
            continue
        if 0 <= target < len(data):
            entries.append((target, idx, off))
    return table, entries


def xdb_seed_entries(module: str, data: bytes) -> dict[int, list[str]]:
    seeds: dict[int, list[str]] = collections.defaultdict(list)
    seeds[0x0000].append("overlay_entry_0")
    if module in ALIEN_XDBS:
        seeds[0x00A3].append("alien_body_entry_00a3")
        table, entries = alien_method_table_entries(data)
        for target, idx, slot_off in entries:
            if table is None:
                seeds[target].append("alien_method_table_103a")
            else:
                seeds[target].append(
                    f"alien_method_table_103a_slot_{idx}@{h(slot_off)}"
                )
    if module == "manu3":
        for off, reason in MANU3_CODE_SEEDS.items():
            seeds[off].append(reason)
    return dict(seeds)


def xdb_routines(xdb_paths: Iterable[Path], max_decode_bytes: int) -> tuple[list[Routine], dict[str, object]]:
    xdb_labels = parse_xdb_labels(RE_ROOT / "labels.csv")
    all_routines: list[Routine] = []
    metadata: dict[str, object] = {}

    for path in sorted(xdb_paths):
        module = path.stem.lower()
        data = path.read_bytes()
        labels = xdb_labels.get(module, {})
        seeds = xdb_seed_entries(module, data)

        for off, label_infos in labels.items():
            for label in label_infos:
                # Labels are attached to known code entries but do not create code
                # by themselves. Several XDB labels intentionally name data cells.
                if off in seeds:
                    seeds[off].append(f"label:{label.name}")

        queue = collections.deque(sorted(seeds))
        discovered: dict[int, set[str]] = {
            off: set(reasons) for off, reasons in seeds.items()
        }
        decoded_once: set[int] = set()

        while queue:
            entry = queue.popleft()
            if entry in decoded_once or not (0 <= entry < len(data)):
                continue
            decoded_once.add(entry)
            temp = Routine(
                module=f"xdb_{module}",
                artifact_path=path,
                entry=entry,
                address_kind="overlay_offset",
                group="pending",
            )
            decode_routine(temp, data, max_decode_bytes)
            for target in sorted(temp.direct_callees):
                if target not in discovered:
                    discovered[target] = {f"direct_call_from_{h(entry)}"}
                    queue.append(target)
                else:
                    discovered[target].add(f"direct_call_from_{h(entry)}")

        for entry in sorted(discovered):
            if not (0 <= entry < len(data)):
                continue
            provenance = discovered[entry]
            group = "direct_calls"
            if any("method_table" in p for p in provenance):
                group = "method_table_103a"
            if any("overlay_entry" in p or "body_entry" in p for p in provenance):
                group = "entry"
            if module == "manu3" and entry in MANU3_CODE_SEEDS:
                group = "manu3_labeled"
            routine = Routine(
                module=f"xdb_{module}",
                artifact_path=path,
                entry=entry,
                address_kind="overlay_offset",
                group=group,
                provenance=set(provenance),
            )
            for label in labels.get(entry, []):
                add_label(routine.labels, label.name, label.comment)
            all_routines.append(routine)

        metadata[module] = {
            "path": rel(path),
            "sha256": sha256_file(path),
            "entry_count": len(discovered),
            "grouping_evidence": (
                "Grouped by overlay entry/API seeds, alien method table entries, "
                "manu3 hand-labeled code seeds, and recursively discovered direct "
                "near calls. No original object-file translation-unit boundary has "
                "been proven for XDB overlays."
            ),
        }

    return all_routines, metadata


def decode_all(routines: Iterable[Routine], data_by_path: dict[Path, bytes], max_bytes: int) -> None:
    routines = list(routines)
    entries_by_path: dict[Path, set[int]] = collections.defaultdict(set)
    for routine in routines:
        entries_by_path[routine.artifact_path].add(routine.entry)
    for routine in routines:
        decode_routine(
            routine,
            data_by_path[routine.artifact_path],
            max_bytes,
            protected_entries=entries_by_path[routine.artifact_path],
        )


def routine_qualifier(routine: Routine) -> str:
    if any(insn.mnemonic == "retf" for insn in routine.instructions):
        return "CB_FAR"
    if "relocation_proven_far_transfer_target" in routine.provenance:
        return "CB_FAR"
    if routine.entry == 0 and routine.module.startswith("xdb_"):
        return "CB_FAR"
    return "CB_NEAR"


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8", newline="\n")


def asm_text(routine: Routine, data: bytes) -> str:
    lines = [
        "; Commander Blood recovered routine assembly",
        f"; module: {routine.module}",
        f"; artifact: {rel(routine.artifact_path)}",
        f"; artifact_sha256: {sha256_file(routine.artifact_path)}",
        f"; {routine.address_kind}: {h(routine.entry, 6)}",
    ]
    if routine.seg_off:
        lines.append(f"; seg_off: {routine.seg_off}")
    lines.extend(
        [
            f"; group: {routine.group}",
            f"; provenance: {', '.join(sorted(routine.provenance)) or 'unknown'}",
        ]
    )
    if routine.labels:
        for label in routine.labels:
            lines.append(f"; label: {label.name}")
            if label.comment:
                lines.append(f"; label_comment: {label.comment}")
    if routine.incoming:
        for incoming in sorted(routine.incoming):
            lines.append(f"; incoming: {incoming}")
    lines.extend(
        [
            f"; byte_count: {routine.byte_count}",
            f"; boundary: {routine.boundary_reason}",
            f"; terminal: {routine.terminal or 'none'}",
            f"; direct_callees: {', '.join(h(x, 6) for x in sorted(routine.direct_callees)) or 'none'}",
            f"; indirect_calls: {len(routine.indirect_calls)}",
        ]
    )
    if routine.cpp_path:
        lines.append(f"; cxx_source: {rel(routine.cpp_path)}")
    if routine.byte_count:
        blob = data[routine.entry : routine.entry + routine.byte_count]
        lines.append(f"; routine_bytes_sha256: {sha256_bytes(blob)}")
    lines.append("")

    prev_end: int | None = None
    for insn in routine.instructions:
        if prev_end is not None and insn.address != prev_end:
            lines.append(f"; -- non-contiguous block: next {h(insn.address, 6)} --")
        byte_s = insn.bytes.hex(" ").upper()
        op_s = f" {insn.op_str}" if insn.op_str else ""
        lines.append(f"{insn.address:06X}:  {byte_s:<28} {insn.mnemonic:<8}{op_s}")
        prev_end = insn.address + insn.size
    if not routine.instructions:
        lines.append("; no instructions decoded")
    lines.append("")
    return "\n".join(lines)


def control_byte_copy_body(setup: list[str], post_null_store: list[str]) -> list[str]:
    return [
        *setup,
        "    for (;;) {",
        "        cb_set_lo8(m->ax, m->read8(m->ds, m->si));",
        "        cb_advance_u16(m->si, 1, m->df);",
        "        cb_u8 value = cb_lo8(m->ax);",
        "        m->set_logic8_flags(value);",
        "        if ((value & 0x80u) != 0) {",
        "            break;",
        "        }",
        "        cb_u8 cmp_result = (cb_u8)(value - 0x20);",
        "        m->set_sub8_flags(value, 0x20, cmp_result);",
        "        if (value < 0x20u) {",
        "            break;",
        "        }",
        "        m->write8(m->es, m->di, value);",
        "        cb_advance_u16(m->di, 1, m->df);",
        "    }",
        "    cb_u16 before_dec = m->si;",
        "    m->si = (cb_u16)(m->si - 1);",
        "    m->set_dec16_flags(before_dec, m->si);",
        "    m->write8(m->es, m->di, 0);",
        *post_null_store,
        "    return;",
    ]


def xdb_field_delta_body(sar4: bool) -> list[str]:
    body = [
        "    m->push16(m->ds);",
        "    m->si = m->read16(m->ds, (cb_u16)(m->di + 0x38));",
        "    m->bx = m->read16(m->ds, (cb_u16)(m->di + 0x3a));",
        "    m->ax = m->read16(m->ds, (cb_u16)(m->si + 0x36));",
        "    cb_u16 before_add = m->si;",
        "    m->si = (cb_u16)(m->si + 4);",
        "    m->set_add16_flags(before_add, 4, m->si);",
        "    m->si = (cb_u16)(m->si & 0x0ffcu);",
        "    m->set_logic16_flags(m->si);",
        "    m->write16(m->ds, (cb_u16)(m->di + 0x38), m->si);",
    ]
    if sar4:
        body.extend(
            [
                "    cb_u16 before_sar = m->ax;",
                "    if ((before_sar & 0x8000u) != 0) {",
                "        m->ax = (cb_u16)((before_sar >> 4) | 0xf000u);",
                "    } else {",
                "        m->ax = (cb_u16)(before_sar >> 4);",
                "    }",
                "    m->set_sar16_flags(before_sar, 4, m->ax);",
            ]
        )
    body.extend(
        [
            "    m->write16(m->ds, (cb_u16)(m->di + 0x3a), m->ax);",
            "    cb_u16 before_sub = m->ax;",
            "    m->ax = (cb_u16)(m->ax - m->bx);",
            "    m->set_sub16_flags(before_sub, m->bx, m->ax);",
            "    m->ds = m->read16(m->fs, 0x0002);",
            "    m->si = m->read16(m->fs, (cb_u16)(m->di + 0x1c));",
            "    m->cx = m->read16(m->fs, (cb_u16)(m->di + 0x20));",
            "    for (;;) {",
            "        cb_u16 field_value = m->read16(m->ds, m->si);",
            "        cb_u16 field_result = (cb_u16)(field_value + m->ax);",
            "        m->write16(m->ds, m->si, field_result);",
            "        m->set_add16_flags(field_value, m->ax, field_result);",
            "        before_add = m->si;",
            "        m->si = (cb_u16)(m->si + 0x14);",
            "        m->set_add16_flags(before_add, 0x14, m->si);",
            "        m->cx = (cb_u16)(m->cx - 1);",
            "        if (m->cx == 0) {",
            "            break;",
            "        }",
            "    }",
            "    m->ds = m->pop16();",
            "    return;",
        ]
    )
    return body


def translated_cpp_body(routine: Routine) -> list[str] | None:
    if routine.cxx_status == "translated_empty_return":
        return [
            "    (void)m;",
            "    return;",
        ]

    key = (routine.module, routine.entry)

    if key == ("bloodprg", 0x00093B):
        return [
            "    m->push16(m->ax);",
            "    m->push16(m->cx);",
            "    m->push16(m->dx);",
            "    cb_set_hi8(m->ax, 2);",
            "    m->interrupt(0x1a);",
            "    cb_set_lo8(m->ax, cb_hi8(m->cx));",
            "    m->call_near(0x0986);",
            "    m->ax = (cb_u16)(cb_i16)(cb_i8)cb_lo8(m->ax);",
            "    m->write16(m->gs, 0x0aa6, m->ax);",
            "    m->dx = m->pop16();",
            "    m->cx = m->pop16();",
            "    m->ax = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x000950):
        return [
            "    m->push16(m->ax);",
            "    m->push16(m->cx);",
            "    m->push16(m->dx);",
            "    cb_set_hi8(m->ax, 4);",
            "    m->interrupt(0x1a);",
            "    cb_set_lo8(m->ax, cb_lo8(m->dx));",
            "    m->call_near(0x0986);",
            "    m->ax = (cb_u16)(cb_i16)(cb_i8)cb_lo8(m->ax);",
            "    m->write16(m->gs, 0x0aa8, m->ax);",
            "    cb_set_lo8(m->ax, cb_hi8(m->dx));",
            "    m->call_near(0x0986);",
            "    m->ax = (cb_u16)(cb_i16)(cb_i8)cb_lo8(m->ax);",
            "    m->write16(m->gs, 0x0aaa, m->ax);",
            "    cb_set_lo8(m->ax, cb_lo8(m->cx));",
            "    m->call_near(0x0986);",
            "    m->ax = (cb_u16)(cb_i16)(cb_i8)cb_lo8(m->ax);",
            "    cb_u8 ch_value = cb_hi8(m->cx);",
            "    cb_u8 cmp_ch = (cb_u8)(ch_value - 0x13);",
            "    m->set_sub8_flags(ch_value, 0x13, cmp_ch);",
            "    cb_u16 before_add = m->ax;",
            "    if (cmp_ch == 0) {",
            "        m->ax = (cb_u16)(m->ax + 0x076c);",
            "        m->set_add16_flags(before_add, 0x076c, m->ax);",
            "    } else {",
            "        m->ax = (cb_u16)(m->ax + 0x07d0);",
            "        m->set_add16_flags(before_add, 0x07d0, m->ax);",
            "    }",
            "    m->write16(m->gs, 0x0aac, m->ax);",
            "    m->dx = m->pop16();",
            "    m->cx = m->pop16();",
            "    m->ax = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x000B32):
        return [
            "    m->ax = 0x1500;",
            "    m->bx = 0;",
            "    m->set_logic16_flags(m->bx);",
            "    m->interrupt(0x2f);",
            "    m->set_logic16_flags(m->bx);",
            "    m->write8(m->gs, 0x0ae6, m->bx != 0 ? 1 : 0);",
            "    return;",
        ]

    if key == ("bloodprg", 0x000BFF):
        return [
            "    m->push16(m->ax);",
            "    m->push16(m->dx);",
            "    m->push16(m->ds);",
            "    m->ax = m->cs;",
            "    m->ds = m->ax;",
            "    m->ax = 0x2523;",
            "    m->dx = 0x0619;",
            "    m->interrupt(0x21);",
            "    cb_set_lo8(m->ax, 0x24);",
            "    m->dx = 0x061a;",
            "    m->interrupt(0x21);",
            "    m->ds = m->pop16();",
            "    m->dx = m->pop16();",
            "    m->ax = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x000CC0):
        return [
            "    m->push16(m->ax);",
            "    m->ax = 0;",
            "    m->set_logic16_flags(m->ax);",
            "    cb_set_lo8(m->ax, m->read8(m->gs, 0x5232));",
            "    m->interrupt(0x10);",
            "    m->ax = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x000CEF):
        return [
            "    m->push16(m->ax);",
            "    m->push16(m->bx);",
            "    m->push16(m->cx);",
            "    m->push16(m->dx);",
            "    m->push16(m->es);",
            "    m->ax = 0;",
            "    m->set_logic16_flags(m->ax);",
            "    m->interrupt(0x33);",
            "    m->ax = 2;",
            "    m->interrupt(0x33);",
            "    m->cx = 0x000c;",
            "    m->dx = 0x000c;",
            "    m->ax = 0x000f;",
            "    m->interrupt(0x33);",
            "    m->es = m->pop16();",
            "    m->dx = m->pop16();",
            "    m->cx = m->pop16();",
            "    m->bx = m->pop16();",
            "    m->ax = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x000D0E):
        return [
            "    m->push16(m->ax);",
            "    m->push16(m->bx);",
            "    m->push16(m->cx);",
            "    m->push16(m->dx);",
            "    m->ax = 3;",
            "    m->interrupt(0x33);",
            "    m->write16(m->gs, 0x0a2a, m->cx);",
            "    m->write16(m->gs, 0x0a2c, m->dx);",
            "    m->write16(m->gs, 0x0a2e, m->bx);",
            "    cb_u16 old_x = m->read16(m->gs, 0x0a38);",
            "    cb_u16 cmp_x = (cb_u16)(m->cx - old_x);",
            "    m->set_sub16_flags(m->cx, old_x, cmp_x);",
            "    if (cmp_x != 0) {",
            "        m->write16(m->gs, 0x0a38, m->cx);",
            "        m->write16(m->gs, 0x0a3a, m->dx);",
            "        m->write16(m->gs, 0x0b3b, 0);",
            "    } else {",
            "        cb_u16 old_y = m->read16(m->gs, 0x0a3a);",
            "        cb_u16 cmp_y = (cb_u16)(m->dx - old_y);",
            "        m->set_sub16_flags(m->dx, old_y, cmp_y);",
            "        if (cmp_y != 0) {",
            "            m->write16(m->gs, 0x0a38, m->cx);",
            "            m->write16(m->gs, 0x0a3a, m->dx);",
            "            m->write16(m->gs, 0x0b3b, 0);",
            "        }",
            "    }",
            "    m->dx = m->pop16();",
            "    m->cx = m->pop16();",
            "    m->bx = m->pop16();",
            "    m->ax = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x000D4A):
        return [
            "    m->push16(m->ax);",
            "    m->push16(m->bx);",
            "    m->push16(m->cx);",
            "    m->push16(m->dx);",
            "    m->cx = m->ax;",
            "    m->dx = m->bx;",
            "    m->ax = 7;",
            "    m->interrupt(0x33);",
            "    m->dx = m->pop16();",
            "    m->cx = m->pop16();",
            "    m->ax = 8;",
            "    m->interrupt(0x33);",
            "    m->bx = m->pop16();",
            "    m->ax = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x000D61):
        return [
            "    m->push16(m->ax);",
            "    m->push16(m->dx);",
            "    m->push16(m->si);",
            "    cb_set_hi8(m->ax, 2);",
            "    for (;;) {",
            "        cb_set_lo8(m->ax, m->read8(m->ds, m->si));",
            "        cb_advance_u16(m->si, 1, m->df);",
            "        m->set_logic8_flags(cb_lo8(m->ax));",
            "        if (cb_lo8(m->ax) == 0) {",
            "            break;",
            "        }",
            "        cb_set_lo8(m->dx, cb_lo8(m->ax));",
            "        m->interrupt(0x21);",
            "    }",
            "    m->si = m->pop16();",
            "    m->dx = m->pop16();",
            "    m->ax = m->pop16();",
            "    return;",
        ]

    if key in GS_BYTE_STORE_IMM8_TRANSLATIONS:
        off, value, _note = GS_BYTE_STORE_IMM8_TRANSLATIONS[key]
        return [
            f"    m->write8(m->gs, {h(off)}, {value});",
            "    return;",
        ]

    if key == ("bloodprg", 0x007612):
        return [
            "    m->di = 0x0e18;",
            "    for (;;) {",
            "        cb_set_lo8(m->ax, m->read8(m->ds, m->si));",
            "        cb_advance_u16(m->si, 1, m->df);",
            "        m->write8(m->es, m->di, cb_lo8(m->ax));",
            "        cb_advance_u16(m->di, 1, m->df);",
            "        m->set_logic8_flags(cb_lo8(m->ax));",
            "        if (cb_lo8(m->ax) == 0) {",
            "            break;",
            "        }",
            "    }",
            "    m->write8(m->gs, 0x5e64, 1);",
            "    m->write16(m->gs, 0x5e58, 0);",
            "    return;",
        ]

    if key in BLOODPRG_CONTROL_BYTE_COPY_TRANSLATIONS:
        dest = BLOODPRG_CONTROL_BYTE_COPY_TRANSLATIONS[key]
        return control_byte_copy_body([f"    m->di = {h(dest)};"], [])

    if key == ("bloodprg", 0x007754):
        return control_byte_copy_body(
            ["    m->di = m->read16(m->gs, 0x131a);"],
            [
                "    cb_u16 cursor = m->read16(m->gs, 0x131a);",
                "    cb_u16 cursor_result = (cb_u16)(cursor + 0x10);",
                "    m->write16(m->gs, 0x131a, cursor_result);",
                "    m->set_add16_flags(cursor, 0x10, cursor_result);",
                "    cb_u8 count = m->read8(m->gs, 0x131e);",
                "    cb_u8 count_result = (cb_u8)(count + 1);",
                "    m->write8(m->gs, 0x131e, count_result);",
                "    m->set_inc8_flags(count, count_result);",
            ],
        )

    if key == ("bloodprg", 0x007776):
        return [
            "    m->di = m->read16(m->gs, 0x0f18);",
            "    cb_u16 prefix = m->read16(m->ds, m->si);",
            "    m->write16(m->es, m->di, prefix);",
            "    cb_advance_u16(m->si, 2, m->df);",
            "    cb_advance_u16(m->di, 2, m->df);",
            "    for (;;) {",
            "        cb_set_lo8(m->ax, m->read8(m->ds, m->si));",
            "        cb_advance_u16(m->si, 1, m->df);",
            "        m->write8(m->es, m->di, cb_lo8(m->ax));",
            "        cb_advance_u16(m->di, 1, m->df);",
            "        m->set_logic8_flags(cb_lo8(m->ax));",
            "        if (cb_lo8(m->ax) == 0) {",
            "            break;",
            "        }",
            "    }",
            "    m->write16(m->gs, 0x0f18, m->di);",
            "    return;",
        ]

    if key == ("bloodprg", 0x007788):
        return control_byte_copy_body(
            [
                "    cb_u16 saved_es = m->es;",
                "    m->ax = m->fs;",
                "    m->es = m->ax;",
                "    m->di = 0x0c74;",
            ],
            [
                "    m->write8(m->gs, 0x27e8, 1);",
                "    m->es = saved_es;",
            ],
        )

    if key == ("bloodprg", 0x0025A4):
        return [
            "    cb_u16 saved_ax = m->ax;",
            "    cb_u16 saved_si = m->si;",
            "    cb_u16 saved_di = m->di;",
            "",
            "    for (;;) {",
            "        cb_set_lo8(m->ax, m->read8(m->ds, m->si));",
            "        cb_advance_u16(m->si, 1, m->df);",
            "        cb_u8 left = cb_lo8(m->ax);",
            "        cb_u8 right = m->read8(m->es, m->di);",
            "        cb_u8 cmp_result = (cb_u8)(left - right);",
            "        m->set_sub8_flags(left, right, cmp_result);",
            "        if (cmp_result != 0) {",
            "            m->cf = 0;",
            "            m->di = saved_di;",
            "            m->si = saved_si;",
            "            m->ax = saved_ax;",
            "            return;",
            "        }",
            "        m->di = (cb_u16)(m->di + 1);",
            "        m->set_logic8_flags(left);",
            "        if (left == 0) {",
            "            m->cf = 1;",
            "            m->di = saved_di;",
            "            m->si = saved_si;",
            "            m->ax = saved_ax;",
            "            return;",
            "        }",
            "    }",
        ]

    if key == ("bloodprg", 0x00267D):
        return [
            "    m->ax = 0x0100;",
            "    m->interrupt(0x16);",
            "    if (!m->zf) {",
            "        m->ax = 0;",
            "        m->set_logic16_flags(m->ax);",
            "        m->interrupt(0x16);",
            "        return;",
            "    }",
            "    m->ax = 0;",
            "    m->set_logic16_flags(m->ax);",
            "    return;",
        ]

    if key == ("bloodprg", 0x002DD3):
        return [
            "    m->push16(m->ax);",
            "    m->ax = 0;",
            "    m->set_logic16_flags(m->ax);",
            "    m->out8(0x0070, cb_lo8(m->ax));",
            "    cb_set_lo8(m->ax, m->in8(0x0071));",
            "    cb_set_hi8(m->ax, cb_lo8(m->ax));",
            "    m->write16(m->cs, 0x0aee, m->ax);",
            "    m->ax = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x002F90):
        return [
            "    m->push16(m->ax);",
            "    m->push16(m->cx);",
            "    m->push16(m->dx);",
            "    m->push16(m->si);",
            "    m->dx = 0x03c8;",
            "    cb_set_lo8(m->ax, 0);",
            "    m->set_logic8_flags(cb_lo8(m->ax));",
            "    m->out8(m->dx, cb_lo8(m->ax));",
            "    cb_u8 dl_before = cb_lo8(m->dx);",
            "    cb_u8 dl_after = (cb_u8)(dl_before + 1);",
            "    cb_set_lo8(m->dx, dl_after);",
            "    m->set_inc8_flags(dl_before, dl_after);",
            "    m->cx = 0x0300;",
            "    while (m->cx != 0) {",
            "        cb_u8 value = m->read8(m->ds, m->si);",
            "        m->out8(m->dx, value);",
            "        cb_advance_u16(m->si, 1, m->df);",
            "        m->cx = (cb_u16)(m->cx - 1);",
            "    }",
            "    m->si = m->pop16();",
            "    m->dx = m->pop16();",
            "    m->cx = m->pop16();",
            "    m->ax = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x002FA6):
        return [
            "    m->push16(m->ax);",
            "    m->push16(m->cx);",
            "    m->push16(m->dx);",
            "    m->dx = 0x03c8;",
            "    cb_set_lo8(m->ax, 0);",
            "    m->set_logic8_flags(cb_lo8(m->ax));",
            "    m->out8(m->dx, cb_lo8(m->ax));",
            "    cb_u8 dl_before = cb_lo8(m->dx);",
            "    cb_u8 dl_after = (cb_u8)(dl_before + 1);",
            "    cb_set_lo8(m->dx, dl_after);",
            "    m->set_inc8_flags(dl_before, dl_after);",
            "    m->cx = 0x0300;",
            "    while (m->cx != 0) {",
            "        m->out8(m->dx, cb_lo8(m->ax));",
            "        m->cx = (cb_u16)(m->cx - 1);",
            "    }",
            "    m->dx = m->pop16();",
            "    m->cx = m->pop16();",
            "    m->ax = m->pop16();",
            "    return;",
        ]

    if key in BLOODPRG_FULL_DWORD_COPY_TRANSLATIONS:
        dest_ptr = BLOODPRG_FULL_DWORD_COPY_TRANSLATIONS[key]
        return [
            "    m->push16(m->cx);",
            "    m->push16(m->es);",
            "    m->push16(m->di);",
            "    m->push16(m->si);",
            "    m->df = 0;",
            f"    m->di = m->read16(m->gs, {h(dest_ptr)});",
            f"    m->es = m->read16(m->gs, {h(dest_ptr + 2)});",
            "    m->cx = 0x3e80;",
            "    while (m->cx != 0) {",
            "        cb_u32 value = m->read32(m->ds, m->si);",
            "        m->write32(m->es, m->di, value);",
            "        cb_advance_u16(m->si, 4, m->df);",
            "        cb_advance_u16(m->di, 4, m->df);",
            "        m->cx = (cb_u16)(m->cx - 1);",
            "    }",
            "    m->si = m->pop16();",
            "    m->di = m->pop16();",
            "    m->es = m->pop16();",
            "    m->cx = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x001FBC):
        return [
            "    m->ax = m->read16(m->ds, 0x0a2e);",
            "    cb_u8 al = cb_lo8(m->ax);",
            "    cb_u8 test1 = (cb_u8)(al & 1);",
            "    m->set_logic8_flags(test1);",
            "    if (test1 != 0) {",
            "        al = (cb_u8)(al & m->read8(m->ds, 0x0a30));",
            "        cb_set_lo8(m->ax, al);",
            "        m->set_logic8_flags(al);",
            "        if (al == 0) {",
            "            m->write8(m->ds, 0x0a3e, 1);",
            "            m->write8(m->ds, 0x0a40, 1);",
            "        }",
            "    }",
            "    cb_u8 test2 = (cb_u8)(al & 2);",
            "    m->set_logic8_flags(test2);",
            "    if (test2 != 0) {",
            "        al = (cb_u8)(al & m->read8(m->ds, 0x0a30));",
            "        cb_set_lo8(m->ax, al);",
            "        m->set_logic8_flags(al);",
            "        if (al == 0) {",
            "            m->write8(m->ds, 0x0a3f, 1);",
            "            m->write8(m->ds, 0x0a40, 1);",
            "        }",
            "    }",
            "    m->ax = m->read16(m->ds, 0x0a2e);",
            "    m->write16(m->ds, 0x0a30, m->ax);",
            "    return;",
        ]

    if key == ("bloodprg", 0x005320):
        return [
            "    cb_u16 saved_bx = m->bx;",
            "    cb_u16 table_off = (cb_u16)(m->ax << 3);",
            "    m->bx = table_off;",
            "    m->ax = 0;",
            "    m->set_logic16_flags(m->ax);",
            "    cb_u16 flags = m->read16(m->fs, (cb_u16)(m->bx + 2));",
            "    cb_u16 test_result = (cb_u16)(flags & 3);",
            "    m->set_logic16_flags(test_result);",
            "    if (test_result != 0) {",
            "        m->ax = m->read16(m->fs, m->bx);",
            "        m->ds = m->ax;",
            "        m->si = 0;",
            "        m->set_logic16_flags(m->si);",
            "        m->ax = 1;",
            "    }",
            "    m->bx = saved_bx;",
            "    return;",
        ]

    if key == ("bloodprg", 0x005288):
        return [
            "    m->push16(m->bx);",
            "    m->bx = m->ax;",
            "    m->bx = (cb_u16)(m->bx << 3);",
            "    cb_u16 test_result = (cb_u16)(m->read16(m->fs, (cb_u16)(m->bx + 2)) & 3);",
            "    m->set_logic16_flags(test_result);",
            "    if (test_result != 0) {",
            "        m->push16(m->cs);",
            "        m->call_near(0x529c);",
            "    }",
            "    m->bx = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x00178B):
        return [
            "    cb_u8 test_result = (cb_u8)(m->read8(m->ds, 0x5b55) & 1);",
            "    m->set_logic8_flags(test_result);",
            "    if (test_result != 0) {",
            "        m->call_far(0x0000, 0x05d7);",
            "        m->si = 0x5251;",
            "        m->call_far(0x0299, 0x0000);",
            "        m->write8(m->ds, 0x5b55, 0);",
            "        m->write8(m->ds, 0x0a40, 0);",
            "        m->write8(m->ds, 0x0a3e, 0);",
            "    }",
            "    return;",
        ]

    if key == ("bloodprg", 0x001397):
        return [
            "    m->push16(m->ax);",
            "    m->push16(m->es);",
            "    m->push16(m->bx);",
            "    m->push16(m->cx);",
            "    cb_u8 test_result = (cb_u8)(m->read8(m->gs, 0x0ae6) & 1);",
            "    m->set_logic8_flags(test_result);",
            "    if (test_result != 0) {",
            "        m->ax = m->gs;",
            "        m->es = m->ax;",
            "        m->bx = 0x0b72;",
            "        m->write8(m->es, m->bx, 0x0d);",
            "        m->write8(m->es, (cb_u16)(m->bx + 2), 0x85);",
            "        m->ax = 0x1510;",
            "        m->cx = 0;",
            "        m->set_logic16_flags(m->cx);",
            "        cb_set_lo8(m->cx, m->read8(m->gs, 0x01b9));",
            "        m->interrupt(0x2f);",
            "    }",
            "    m->cx = m->pop16();",
            "    m->bx = m->pop16();",
            "    m->es = m->pop16();",
            "    m->ax = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x0027C3):
        return [
            "    m->push16(m->ax);",
            "    m->push16(m->ds);",
            "    m->push16(m->dx);",
            "    m->ax = m->gs;",
            "    m->ds = m->ax;",
            "    cb_u8 test_result = (cb_u8)(m->read8(m->ds, 0x0ae0) & 1);",
            "    m->set_logic8_flags(test_result);",
            "    if (test_result == 0) {",
            "        cb_set_hi8(m->ax, 0x0e);",
            "        cb_set_lo8(m->dx, m->read8(m->ds, 0x01b8));",
            "        m->interrupt(0x21);",
            "        m->dx = 0x01ba;",
            "        cb_set_hi8(m->ax, 0x3b);",
            "        m->interrupt(0x21);",
            "        m->write8(m->ds, 0x0ae0, 1);",
            "    }",
            "    m->dx = m->pop16();",
            "    m->ds = m->pop16();",
            "    m->ax = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x0027E9):
        return [
            "    m->push16(m->ax);",
            "    m->push16(m->ds);",
            "    m->push16(m->dx);",
            "    m->ax = m->gs;",
            "    m->ds = m->ax;",
            "    cb_u8 test_result = (cb_u8)(m->read8(m->ds, 0x0ae0) & 1);",
            "    m->set_logic8_flags(test_result);",
            "    if (test_result != 0) {",
            "        cb_set_hi8(m->ax, 0x0e);",
            "        cb_set_lo8(m->dx, m->read8(m->ds, 0x01b9));",
            "        m->interrupt(0x21);",
            "        m->dx = 0x01da;",
            "        cb_set_hi8(m->ax, 0x3b);",
            "        m->interrupt(0x21);",
            "        m->write8(m->ds, 0x0ae0, 0);",
            "    }",
            "    m->dx = m->pop16();",
            "    m->ds = m->pop16();",
            "    m->ax = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x003B45):
        return [
            "    m->push16(m->cx);",
            "    m->push16(m->cs);",
            "    m->call_near(0x32ac);",
            "    cb_u16 tmp = m->bp;",
            "    m->bp = m->dx;",
            "    m->dx = tmp;",
            "    m->push16(m->cs);",
            "    m->call_near(0x3321);",
            "    cb_u16 before_add = m->bx;",
            "    m->bx = (cb_u16)(m->bx + m->bp);",
            "    m->set_add16_flags(before_add, m->bp, m->bx);",
            "    cb_u16 before_dec = m->bx;",
            "    m->bx = (cb_u16)(m->bx - 1);",
            "    m->set_dec16_flags(before_dec, m->bx);",
            "    m->push16(m->cs);",
            "    m->call_near(0x3321);",
            "    cb_u16 before_sub = m->bx;",
            "    m->bx = (cb_u16)(m->bx - m->bp);",
            "    m->set_sub16_flags(before_sub, m->bp, m->bx);",
            "    cb_u16 before_inc = m->bx;",
            "    m->bx = (cb_u16)(m->bx + 1);",
            "    m->set_inc16_flags(before_inc, m->bx);",
            "    tmp = m->bp;",
            "    m->bp = m->dx;",
            "    m->dx = tmp;",
            "    before_add = m->cx;",
            "    m->cx = (cb_u16)(m->cx + m->bp);",
            "    m->set_add16_flags(before_add, m->bp, m->cx);",
            "    before_dec = m->cx;",
            "    m->cx = (cb_u16)(m->cx - 1);",
            "    m->set_dec16_flags(before_dec, m->cx);",
            "    m->push16(m->cs);",
            "    m->call_near(0x32ac);",
            "    m->cx = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x00577A):
        return [
            "    cb_u16 saved_si = m->si;",
            "    m->bx = m->ax;",
            "    for (;;) {",
            "        m->ax = m->read16(m->ds, m->si);",
            "        cb_advance_u16(m->si, 2, m->df);",
            "        cb_u16 cmp_result = (cb_u16)(m->ax - m->bx);",
            "        m->set_sub16_flags(m->ax, m->bx, cmp_result);",
            "        if (cmp_result == 0) {",
            "            cb_u16 before_add = m->si;",
            "            m->si = (cb_u16)(m->si + 2);",
            "            m->set_add16_flags(before_add, 2, m->si);",
            "            m->ax = m->si;",
            "            m->si = saved_si;",
            "            return;",
            "        }",
            "        m->si = m->read16(m->ds, m->si);",
            "        m->set_logic16_flags(m->si);",
            "        if (m->si == 0) {",
            "            m->ax = m->si;",
            "            m->si = saved_si;",
            "            return;",
            "        }",
            "    }",
        ]

    if key == ("bloodprg", 0x006023):
        return [
            "    m->push16(m->bx);",
            "    m->ax = (cb_u16)(m->ax << 4);",
            "    cb_u16 bsf_source = m->bx;",
            "    if (bsf_source != 0) {",
            "        cb_u16 bit_index = 0;",
            "        while (((bsf_source >> bit_index) & 1u) == 0) {",
            "            bit_index = (cb_u16)(bit_index + 1);",
            "        }",
            "        m->bx = bit_index;",
            "        m->zf = 0;",
            "    } else {",
            "        m->zf = 1;",
            "    }",
            "    cb_u16 before_add = m->bx;",
            "    m->bx = (cb_u16)(m->bx + m->ax);",
            "    m->set_add16_flags(before_add, m->ax, m->bx);",
            "    cb_set_lo8(m->ax, m->read8(m->gs, (cb_u16)(m->bx + 0x6d60)));",
            "    m->ax = (cb_u16)(cb_i16)(cb_i8)cb_lo8(m->ax);",
            "    m->bx = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x005FD8):
        return [
            "    m->push16(m->cx);",
            "    m->push16(m->bp);",
            "    m->bp = 0x6d3e;",
            "    m->cx = 0x0010;",
            "    int found = 0;",
            "    while (m->cx != 0) {",
            "        cb_u16 slot = m->read16(m->ds, m->bp);",
            "        cb_u16 cmp_result = (cb_u16)(m->ax - slot);",
            "        m->set_sub16_flags(m->ax, slot, cmp_result);",
            "        if (cmp_result == 0) {",
            "            m->write16(m->ds, m->bp, 0);",
            "            m->cf = 1;",
            "            found = 1;",
            "            break;",
            "        }",
            "        cb_u16 before_add = m->bp;",
            "        m->bp = (cb_u16)(m->bp + 2);",
            "        m->set_add16_flags(before_add, 2, m->bp);",
            "        m->cx = (cb_u16)(m->cx - 1);",
            "    }",
            "    if (found == 0) {",
            "        m->cf = 0;",
            "    }",
            "    m->bp = m->pop16();",
            "    m->cx = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x005FF6):
        return [
            "    m->push16(m->cx);",
            "    m->push16(m->bp);",
            "    m->bp = 0x6d3e;",
            "    m->cx = 0x0010;",
            "    int found = 0;",
            "    while (m->cx != 0) {",
            "        cb_u16 slot = m->read16(m->ds, m->bp);",
            "        cb_u16 cmp_result = (cb_u16)(m->ax - slot);",
            "        m->set_sub16_flags(m->ax, slot, cmp_result);",
            "        if (cmp_result == 0) {",
            "            found = 1;",
            "            break;",
            "        }",
            "        cb_u16 before_add = m->bp;",
            "        m->bp = (cb_u16)(m->bp + 2);",
            "        m->set_add16_flags(before_add, 2, m->bp);",
            "        m->cx = (cb_u16)(m->cx - 1);",
            "    }",
            "    if (found == 0) {",
            "        m->bp = 0x6d3e;",
            "        m->cx = 0x0010;",
            "        while (m->cx != 0) {",
            "            cb_u16 slot = m->read16(m->ds, m->bp);",
            "            cb_u16 cmp_result = (cb_u16)(slot - 0);",
            "            m->set_sub16_flags(slot, 0, cmp_result);",
            "            if (cmp_result == 0) {",
            "                m->write16(m->ds, m->bp, m->ax);",
            "                found = 1;",
            "                break;",
            "            }",
            "            cb_u16 before_add = m->bp;",
            "            m->bp = (cb_u16)(m->bp + 2);",
            "            m->set_add16_flags(before_add, 2, m->bp);",
            "            m->cx = (cb_u16)(m->cx - 1);",
            "        }",
            "    }",
            "    if (found != 0) {",
            "        m->cf = 1;",
            "    } else {",
            "        m->cf = 0;",
            "    }",
            "    m->bp = m->pop16();",
            "    m->cx = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x006293):
        return [
            "    for (;;) {",
            "        cb_u16 current = m->read16(m->ds, m->si);",
            "        cb_u16 cmp_word = (cb_u16)(m->ax - current);",
            "        m->set_sub16_flags(m->ax, current, cmp_word);",
            "        if (cmp_word == 0) {",
            "            cb_u16 before_add = m->si;",
            "            m->si = (cb_u16)(m->si + 2);",
            "            m->set_add16_flags(before_add, 2, m->si);",
            "            cb_u8 right = m->read8(m->ds, m->si);",
            "            cb_u8 cmp_byte = (cb_u8)(cb_lo8(m->ax) - right);",
            "            m->set_sub8_flags(cb_lo8(m->ax), right, cmp_byte);",
            "            if (cmp_byte == 0) {",
            "                cb_u16 before_inc = m->si;",
            "                m->si = (cb_u16)(m->si + 1);",
            "                m->set_inc16_flags(before_inc, m->si);",
            "            }",
            "            return;",
            "        }",
            "        m->si = (cb_u16)(m->si + 1);",
            "    }",
        ]

    if key in BLOODPRG_VM_FLAG_BRANCH_TRANSLATIONS:
        flag_off = BLOODPRG_VM_FLAG_BRANCH_TRANSLATIONS[key]
        return [
            f"    cb_u8 test_result = (cb_u8)(m->read8(m->gs, {h(flag_off)}) & 1);",
            "    m->set_logic8_flags(test_result);",
            "    if (test_result == 0) {",
            "        m->call_near(0x6462);",
            "    }",
            "    return;",
        ]

    if key == ("bloodprg", 0x006588):
        return [
            "    m->ax = m->read16(m->ds, m->si);",
            "    cb_advance_u16(m->si, 2, m->df);",
            "    m->call_far(0x01ce, 0x0b02);",
            "    m->set_logic16_flags(m->ax);",
            "    if (m->ax != 0) {",
            "        m->call_near(0x6462);",
            "    }",
            "    return;",
        ]

    if key == ("bloodprg", 0x0064B8):
        return [
            "    cb_set_lo8(m->ax, m->read8(m->ds, m->si));",
            "    cb_advance_u16(m->si, 1, m->df);",
            "    m->ax = (cb_u16)(cb_i16)(cb_i8)cb_lo8(m->ax);",
            "    cb_u16 before_dec = m->ax;",
            "    m->ax = (cb_u16)(m->ax - 1);",
            "    m->set_dec16_flags(before_dec, m->ax);",
            "    m->write16(m->gs, 0x6780, m->ax);",
            "    return;",
        ]

    if key == ("bloodprg", 0x0064CE):
        return [
            "    m->bp = 0x6cde;",
            "    cb_set_lo8(m->ax, m->read8(m->ds, m->si));",
            "    cb_advance_u16(m->si, 1, m->df);",
            "    cb_set_lo8(m->ax, (cb_u8)(cb_lo8(m->ax) - 1));",
            "    m->ax = (cb_u16)(cb_i16)(cb_i8)cb_lo8(m->ax);",
            "    m->ax = (cb_u16)(m->ax << 4);",
            "    m->bp = (cb_u16)(m->bp + m->ax);",
            "    for (;;) {",
            "        cb_set_lo8(m->ax, m->read8(m->ds, m->si));",
            "        cb_advance_u16(m->si, 1, m->df);",
            "        m->write8(m->ss, m->bp, cb_lo8(m->ax));",
            "        m->bp = (cb_u16)(m->bp + 1);",
            "        cb_u8 test_result = cb_lo8(m->ax);",
            "        m->set_logic8_flags(test_result);",
            "        if (test_result == 0) {",
            "            cb_u16 before_inc = m->si;",
            "            m->si = (cb_u16)(m->si + 1);",
            "            m->set_inc16_flags(before_inc, m->si);",
            "            return;",
            "        }",
            "    }",
        ]

    if key == ("bloodprg", 0x0064C0):
        return [
            "    m->write8(m->gs, 0x67b1, 0);",
            "    m->write16(m->gs, 0x6764, 0);",
            "    return;",
        ]

    if key == ("bloodprg", 0x006559):
        return [
            "    m->write8(m->gs, 0x67ad, 1);",
            "    m->ax = m->read16(m->gs, 0x6884);",
            "    m->bp = m->ax;",
            "    cb_u16 before_add = m->ax;",
            "    m->ax = (cb_u16)(m->ax + 2);",
            "    m->set_add16_flags(before_add, 2, m->ax);",
            "    m->write16(m->gs, 0x6884, m->ax);",
            "    m->ax = m->read16(m->ds, m->si);",
            "    cb_advance_u16(m->si, 2, m->df);",
            "    m->write16(m->ss, (cb_u16)(m->bp + 0x6820), m->ax);",
            "    return;",
        ]

    if key == ("bloodprg", 0x006572):
        return [
            "    m->write8(m->gs, 0x67ad, 0);",
            "    m->ax = m->read16(m->gs, 0x6884);",
            "    cb_u16 cmp_result = (cb_u16)(m->ax - 2);",
            "    m->set_sub16_flags(m->ax, 2, cmp_result);",
            "    if (cmp_result == 0) {",
            "        return;",
            "    }",
            "    cb_u16 stack_ptr = m->read16(m->gs, 0x6884);",
            "    cb_u16 result = (cb_u16)(stack_ptr - 2);",
            "    m->write16(m->gs, 0x6884, result);",
            "    m->set_sub16_flags(stack_ptr, 2, result);",
            "    return;",
        ]

    if key == ("bloodprg", 0x0065DB):
        return [
            "    m->si = m->read16(m->ds, m->si);",
            "    m->write8(m->gs, 0x67b1, 0);",
            "    m->write16(m->gs, 0x6764, 0);",
            "    return;",
        ]

    if key == ("bloodprg", 0x0067BA):
        return [
            "    m->ax = m->read16(m->ds, m->si);",
            "    cb_advance_u16(m->si, 2, m->df);",
            "    cb_u8 test_result = (cb_u8)(m->read8(m->gs, 0x67ac) & 1);",
            "    m->set_logic8_flags(test_result);",
            "    if (test_result != 0) {",
            "        m->write16(m->gs, 0x6770, m->ax);",
            "    }",
            "    return;",
        ]

    if key == ("bloodprg", 0x00684C):
        return [
            "    cb_set_lo8(m->ax, m->read8(m->ds, m->si));",
            "    cb_advance_u16(m->si, 1, m->df);",
            "    m->bx = m->read16(m->ds, m->si);",
            "    m->write8(m->ds, m->bx, cb_lo8(m->ax));",
            "    cb_u16 before_add = m->si;",
            "    m->si = (cb_u16)(m->si + 2);",
            "    m->set_add16_flags(before_add, 2, m->si);",
            "    return;",
        ]

    if key == ("bloodprg", 0x0076BA):
        return [
            "    m->ax = m->read16(m->ds, m->si);",
            "    cb_advance_u16(m->si, 2, m->df);",
            "    m->write16(m->gs, 0x1fa5, m->ax);",
            "    return;",
        ]

    if key == ("bloodprg", 0x008269):
        return [
            "    m->push16(m->ax);",
            "    cb_u8 test_result = (cb_u8)(m->read8(m->ds, 0x0a3e) & 1);",
            "    m->set_logic8_flags(test_result);",
            "    if (test_result == 0) {",
            "        m->ax = m->pop16();",
            "        return;",
            "    }",
            "    m->ax = m->read16(m->ds, 0x0a2a);",
            "    cb_u16 rect_x = m->read16(m->ds, m->si);",
            "    cb_u16 cmp_x_min = (cb_u16)(m->ax - rect_x);",
            "    m->set_sub16_flags(m->ax, rect_x, cmp_x_min);",
            "    if ((cb_i16)m->ax < (cb_i16)rect_x) {",
            "        m->ax = m->pop16();",
            "        return;",
            "    }",
            "    cb_u16 width = m->read16(m->ds, (cb_u16)(m->si + 4));",
            "    cb_u16 before_sub = m->ax;",
            "    m->ax = (cb_u16)(m->ax - width);",
            "    m->set_sub16_flags(before_sub, width, m->ax);",
            "    rect_x = m->read16(m->ds, m->si);",
            "    cb_u16 cmp_x_max = (cb_u16)(m->ax - rect_x);",
            "    m->set_sub16_flags(m->ax, rect_x, cmp_x_max);",
            "    if ((cb_i16)m->ax > (cb_i16)rect_x) {",
            "        m->ax = m->pop16();",
            "        return;",
            "    }",
            "    m->ax = m->read16(m->ds, 0x0a2c);",
            "    cb_u16 rect_y = m->read16(m->ds, (cb_u16)(m->si + 2));",
            "    cb_u16 cmp_y_min = (cb_u16)(m->ax - rect_y);",
            "    m->set_sub16_flags(m->ax, rect_y, cmp_y_min);",
            "    if ((cb_i16)m->ax < (cb_i16)rect_y) {",
            "        m->ax = m->pop16();",
            "        return;",
            "    }",
            "    cb_u16 height = m->read16(m->ds, (cb_u16)(m->si + 6));",
            "    before_sub = m->ax;",
            "    m->ax = (cb_u16)(m->ax - height);",
            "    m->set_sub16_flags(before_sub, height, m->ax);",
            "    rect_y = m->read16(m->ds, (cb_u16)(m->si + 2));",
            "    cb_u16 cmp_y_max = (cb_u16)(m->ax - rect_y);",
            "    m->set_sub16_flags(m->ax, rect_y, cmp_y_max);",
            "    if ((cb_i16)m->ax > (cb_i16)rect_y) {",
            "        m->ax = m->pop16();",
            "        return;",
            "    }",
            "    cb_u8 flags = (cb_u8)(m->read8(m->ss, m->bp) | 8);",
            "    m->write8(m->ss, m->bp, flags);",
            "    m->set_logic8_flags(flags);",
            "    m->ax = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x008295):
        return [
            "    m->push16(m->ax);",
            "    cb_u8 test_result = (cb_u8)(m->read8(m->ds, 0x0a3e) & 1);",
            "    m->set_logic8_flags(test_result);",
            "    if (test_result == 0) {",
            "        m->cf = 0;",
            "        m->ax = m->pop16();",
            "        return;",
            "    }",
            "    m->ax = m->read16(m->ds, 0x0a2a);",
            "    cb_u16 rect_x = m->read16(m->ss, m->bp);",
            "    cb_u16 cmp_x_min = (cb_u16)(m->ax - rect_x);",
            "    m->set_sub16_flags(m->ax, rect_x, cmp_x_min);",
            "    if ((cb_i16)m->ax < (cb_i16)rect_x) {",
            "        m->cf = 0;",
            "        m->ax = m->pop16();",
            "        return;",
            "    }",
            "    cb_u16 width = m->read16(m->ss, (cb_u16)(m->bp + 4));",
            "    cb_u16 before_sub = m->ax;",
            "    m->ax = (cb_u16)(m->ax - width);",
            "    m->set_sub16_flags(before_sub, width, m->ax);",
            "    rect_x = m->read16(m->ss, m->bp);",
            "    cb_u16 cmp_x_max = (cb_u16)(m->ax - rect_x);",
            "    m->set_sub16_flags(m->ax, rect_x, cmp_x_max);",
            "    if ((cb_i16)m->ax > (cb_i16)rect_x) {",
            "        m->cf = 0;",
            "        m->ax = m->pop16();",
            "        return;",
            "    }",
            "    m->ax = m->read16(m->ds, 0x0a2c);",
            "    cb_u16 rect_y = m->read16(m->ss, (cb_u16)(m->bp + 2));",
            "    cb_u16 cmp_y_min = (cb_u16)(m->ax - rect_y);",
            "    m->set_sub16_flags(m->ax, rect_y, cmp_y_min);",
            "    if ((cb_i16)m->ax < (cb_i16)rect_y) {",
            "        m->cf = 0;",
            "        m->ax = m->pop16();",
            "        return;",
            "    }",
            "    cb_u16 height = m->read16(m->ss, (cb_u16)(m->bp + 6));",
            "    before_sub = m->ax;",
            "    m->ax = (cb_u16)(m->ax - height);",
            "    m->set_sub16_flags(before_sub, height, m->ax);",
            "    rect_y = m->read16(m->ss, (cb_u16)(m->bp + 2));",
            "    cb_u16 cmp_y_max = (cb_u16)(m->ax - rect_y);",
            "    m->set_sub16_flags(m->ax, rect_y, cmp_y_max);",
            "    if ((cb_i16)m->ax > (cb_i16)rect_y) {",
            "        m->cf = 0;",
            "        m->ax = m->pop16();",
            "        return;",
            "    }",
            "    m->cf = 1;",
            "    m->ax = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x008713):
        return [
            "    cb_u8 test_result = (cb_u8)(m->read8(m->ds, 0x2565) & 1);",
            "    m->set_logic8_flags(test_result);",
            "    if (test_result != 0) {",
            "        m->ax = m->read16(m->ds, 0x6754);",
            "        m->write16(m->ds, 0x676a, m->ax);",
            "        m->write16(m->ds, 0x6768, 0x00c3);",
            "        m->write8(m->ds, 0x2565, 0);",
            "    }",
            "    return;",
        ]

    if key == ("bloodprg", 0x008848):
        return [
            "    cb_u8 test_result = (cb_u8)(m->read8(m->ds, 0x2565) & 1);",
            "    m->set_logic8_flags(test_result);",
            "    if (test_result != 0) {",
            "        m->ax = m->read16(m->ds, 0x6756);",
            "        m->write16(m->ds, 0x676a, m->ax);",
            "        m->write16(m->ds, 0x6768, 0x00c3);",
            "        m->write8(m->ds, 0x2565, 0);",
            "        m->si = 0x0d16;",
            "        m->ax = 1;",
            "        m->call_far(0x0b1b, 0x0855);",
            "    }",
            "    return;",
        ]

    if key == ("bloodprg", 0x00963F):
        return [
            "    m->push16(m->ax);",
            "    m->push16(m->cx);",
            "    m->push16(m->bp);",
            "    m->bp = 0x2a1b;",
            "    m->cx = 6;",
            "    while (m->cx != 0) {",
            "        m->write16(m->ss, m->bp, 0);",
            "        cb_u16 before_add = m->bp;",
            "        m->bp = (cb_u16)(m->bp + 0x18);",
            "        m->set_add16_flags(before_add, 0x18, m->bp);",
            "        m->cx = (cb_u16)(m->cx - 1);",
            "    }",
            "    m->bp = m->pop16();",
            "    m->cx = m->pop16();",
            "    m->ax = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x008C96):
        return [
            "    m->push16(m->bp);",
            "    m->push16(m->ax);",
            "    m->push16(m->ds);",
            "    m->push16(m->es);",
            "    m->push16(m->di);",
            "    m->push16(m->si);",
            "    m->push16(m->cx);",
            "    m->call_far(0x04da, 0x1c53);",
            "    m->ax = m->gs;",
            "    m->ds = m->ax;",
            "    m->es = m->ax;",
            "    m->si = 0x53d1;",
            "    m->di = 0x5cd8;",
            "    m->cx = 0x0030;",
            "    while (m->cx != 0) {",
            "        cb_u32 value = m->read32(m->ds, m->si);",
            "        m->write32(m->es, m->di, value);",
            "        cb_advance_u16(m->si, 4, m->df);",
            "        cb_advance_u16(m->di, 4, m->df);",
            "        m->cx = (cb_u16)(m->cx - 1);",
            "    }",
            "    m->write16(m->ds, 0x2f65, 0x2710);",
            "    m->write16(m->ds, 0x2f67, 0x2ee0);",
            "    m->write16(m->ds, 0x2f69, 0);",
            "    m->cx = m->pop16();",
            "    m->si = m->pop16();",
            "    m->di = m->pop16();",
            "    m->es = m->pop16();",
            "    m->ds = m->pop16();",
            "    m->ax = m->pop16();",
            "    m->bp = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x00933A):
        return [
            "    m->push16(m->es);",
            "    m->push16(m->di);",
            "    m->push16(m->ds);",
            "    m->push16(m->si);",
            "    m->push16(m->cx);",
            "    m->push16(m->ax);",
            "    m->di = m->read16(m->gs, 0x5229);",
            "    m->es = m->read16(m->gs, 0x522b);",
            "    m->si = m->read16(m->gs, 0x0abc);",
            "    m->ds = m->read16(m->gs, 0x0abe);",
            "    m->ax = m->cx;",
            "    cb_u8 old_ah = cb_hi8(m->ax);",
            "    cb_set_hi8(m->ax, cb_lo8(m->ax));",
            "    cb_set_lo8(m->ax, old_ah);",
            "    m->cx = (cb_u16)(m->cx << 6);",
            "    cb_u16 before_add = m->ax;",
            "    m->ax = (cb_u16)(m->ax + m->cx);",
            "    m->set_add16_flags(before_add, m->cx, m->ax);",
            "    m->di = m->ax;",
            "    before_add = m->di;",
            "    m->di = (cb_u16)(m->di + m->bx);",
            "    m->set_add16_flags(before_add, m->bx, m->di);",
            "    m->si = m->di;",
            "    m->cx = m->dx;",
            "    while (m->cx != 0) {",
            "        cb_u8 value = m->read8(m->ds, m->si);",
            "        m->write8(m->es, m->di, value);",
            "        cb_advance_u16(m->si, 1, m->df);",
            "        cb_advance_u16(m->di, 1, m->df);",
            "        m->cx = (cb_u16)(m->cx - 1);",
            "    }",
            "    m->ax = m->pop16();",
            "    m->cx = m->pop16();",
            "    m->si = m->pop16();",
            "    m->ds = m->pop16();",
            "    m->di = m->pop16();",
            "    m->es = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x006830):
        return [
            "    cb_set_lo8(m->ax, m->read8(m->ds, m->si));",
            "    cb_advance_u16(m->si, 1, m->df);",
            "    cb_u8 test_result = (cb_u8)(cb_lo8(m->ax) & 1);",
            "    m->set_logic8_flags(test_result);",
            "    if (test_result == 0) {",
            "        m->si = m->read16(m->ds, m->si);",
            "        return;",
            "    }",
            "    m->write8(m->gs, 0x67ad, 1);",
            "    m->ax = m->read16(m->ds, m->si);",
            "    cb_advance_u16(m->si, 2, m->df);",
            "    m->write16(m->gs, 0x6820, m->ax);",
            "    m->write16(m->gs, 0x6884, 2);",
            "    return;",
        ]

    if key == ("bloodprg", 0x009F80):
        return [
            "    m->bx = 0x1fb5;",
            "    for (int i = 0; i != 4; ++i) {",
            "        cb_u16 before_add = m->bx;",
            "        m->bx = (cb_u16)(m->bx + m->ax);",
            "        m->set_add16_flags(before_add, m->ax, m->bx);",
            "    }",
            "    m->bx = m->read16(m->ds, m->bx);",
            "    return;",
        ]

    if key == ("bloodprg", 0x001D74):
        return [
            "    m->push16(m->ds);",
            "    m->push16(m->si);",
            "    m->push16(m->es);",
            "    m->push16(m->di);",
            "    m->push16(m->cx);",
            "    m->cx = m->ax;",
            "    m->si = m->read16(m->gs, 0x0abc);",
            "    m->ds = m->read16(m->gs, 0x0abe);",
            "    m->di = m->read16(m->gs, 0x671c);",
            "    m->es = m->read16(m->gs, 0x671e);",
            "    for (;;) {",
            "        m->ax = m->read16(m->ds, m->si);",
            "        cb_advance_u16(m->si, 2, m->df);",
            "        m->di = m->ax;",
            "        cb_u8 value = m->read8(m->ds, m->si);",
            "        m->write8(m->es, m->di, value);",
            "        cb_advance_u16(m->si, 1, m->df);",
            "        cb_advance_u16(m->di, 1, m->df);",
            "        cb_u16 before_sub = m->cx;",
            "        m->cx = (cb_u16)(m->cx - 3);",
            "        m->set_sub16_flags(before_sub, 3, m->cx);",
            "        if (m->cx == 0) {",
            "            break;",
            "        }",
            "    }",
            "    m->cx = m->pop16();",
            "    m->di = m->pop16();",
            "    m->es = m->pop16();",
            "    m->si = m->pop16();",
            "    m->ds = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x0041D1):
        return [
            "    m->push16(m->ax);",
            "    m->push16(m->bx);",
            "    m->ax = (cb_u16)(m->ax << 5);",
            "    m->bx = 0x6212;",
            "    cb_u16 before_add = m->bx;",
            "    m->bx = (cb_u16)(m->bx + m->ax);",
            "    m->set_add16_flags(before_add, m->ax, m->bx);",
            "    m->ax = m->read16(m->gs, m->bx);",
            "    cb_u8 al = cb_lo8(m->ax);",
            "    m->set_logic8_flags(al);",
            "    if ((al & 0x80u) != 0) {",
            "        cb_u8 test_result = (cb_u8)(al & 1);",
            "        m->set_logic8_flags(test_result);",
            "        if (test_result != 0) {",
            "            al = (cb_u8)(al & 0xfeu);",
            "            m->set_logic8_flags(al);",
            "            al = (cb_u8)(al | 2);",
            "            m->set_logic8_flags(al);",
            "            cb_set_lo8(m->ax, al);",
            "        }",
            "    }",
            "    m->write16(m->gs, m->bx, m->ax);",
            "    m->bx = m->pop16();",
            "    m->ax = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x004240):
        return [
            "    m->push16(m->ax);",
            "    m->push16(m->bx);",
            "    m->push16(m->cx);",
            "    m->push16(m->ds);",
            "    m->push16(m->si);",
            "    m->cx = m->bx;",
            "    cb_u16 before_sub = m->cx;",
            "    m->cx = (cb_u16)(m->cx - m->ax);",
            "    m->set_sub16_flags(before_sub, m->ax, m->cx);",
            "    cb_u16 before_inc = m->cx;",
            "    m->cx = (cb_u16)(m->cx + 1);",
            "    m->set_inc16_flags(before_inc, m->cx);",
            "    m->bx = m->gs;",
            "    m->ds = m->bx;",
            "    m->si = 0x6212;",
            "    m->ax = (cb_u16)(m->ax << 5);",
            "    cb_u16 before_add = m->si;",
            "    m->si = (cb_u16)(m->si + m->ax);",
            "    m->set_add16_flags(before_add, m->ax, m->si);",
            "    for (;;) {",
            "        m->ax = m->read16(m->ds, m->si);",
            "        cb_u8 al = cb_lo8(m->ax);",
            "        m->set_logic8_flags(al);",
            "        if ((al & 0x80u) != 0) {",
            "            al = (cb_u8)(al & 0x7eu);",
            "            m->set_logic8_flags(al);",
            "            al = (cb_u8)(al | 2);",
            "            m->set_logic8_flags(al);",
            "            cb_set_lo8(m->ax, al);",
            "            m->write16(m->ds, m->si, m->ax);",
            "        }",
            "        before_add = m->si;",
            "        m->si = (cb_u16)(m->si + 0x20);",
            "        m->set_add16_flags(before_add, 0x20, m->si);",
            "        m->cx = (cb_u16)(m->cx - 1);",
            "        if (m->cx == 0) {",
            "            break;",
            "        }",
            "    }",
            "    m->si = m->pop16();",
            "    m->ds = m->pop16();",
            "    m->cx = m->pop16();",
            "    m->bx = m->pop16();",
            "    m->ax = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x00A141):
        return [
            "    m->bx = m->read16(m->ds, 0x0d5b);",
            "    m->set_logic16_flags(m->bx);",
            "    if (m->bx != 0) {",
            "        cb_u16 reserved = m->read16(m->ds, 0x0a86);",
            "        cb_u16 cmp_result = (cb_u16)(m->bx - reserved);",
            "        m->set_sub16_flags(m->bx, reserved, cmp_result);",
            "        if (cmp_result != 0) {",
            "            m->write16(m->ds, 0x0d5b, 0);",
            "            cb_set_hi8(m->ax, 0x3e);",
            "            m->interrupt(0x21);",
            "            m->call_near(0xa73e);",
            "        }",
            "    }",
            "    m->cx = 0;",
            "    m->set_logic16_flags(m->cx);",
            "    return;",
        ]

    if key == ("bloodprg", 0x00A2DD):
        return [
            "    cb_u8 flags = m->read8(m->ds, 0x0d5f);",
            "    cb_u8 flags_result = (cb_u8)(flags | 1);",
            "    m->write8(m->ds, 0x0d5f, flags_result);",
            "    m->set_logic8_flags(flags_result);",
            "    cb_u16 count = m->read16(m->ds, 0x0d9a);",
            "    m->set_sub16_flags(count, 0, count);",
            "    if (count == 0) {",
            "        flags = m->read8(m->ds, 0x0d5f);",
            "        flags_result = (cb_u8)(flags | 2);",
            "        m->write8(m->ds, 0x0d5f, flags_result);",
            "        m->set_logic8_flags(flags_result);",
            "        m->call_near(0xa141);",
            "    }",
            "    return;",
        ]

    if key == ("bloodprg", 0x00A622):
        return [
            "    m->cx = 2;",
            "    m->call_near(0xa664);",
            "    if (!m->cf) {",
            "        m->si = m->read16(m->gs, 0x0d8c);",
            "        m->es = m->read16(m->gs, 0x0d8e);",
            "        m->ax = m->read16(m->es, (cb_u16)(m->si - 2));",
            "    }",
            "    return;",
        ]

    if key == ("bloodprg", 0x00A40B):
        return [
            "    cb_u8 first_value = m->read8(m->gs, 0x0d5f);",
            "    cb_u8 first_result = first_value;",
            "    m->set_sub8_flags(first_value, 0, first_result);",
            "    if (first_result == 0) {",
            "        return;",
            "    }",
            "    cb_u8 second_value = m->read8(m->gs, 0x0d5f);",
            "    cb_u8 second_result = (cb_u8)(second_value - 1);",
            "    m->set_sub8_flags(second_value, 1, second_result);",
            "    return;",
        ]

    if key == ("bloodprg", 0x00A117):
        return [
            "    m->push16(m->ds);",
            "    m->push16(m->si);",
            "    cb_u8 test_result = (cb_u8)(m->read8(m->gs, 0x2751) & 1);",
            "    m->set_logic8_flags(test_result);",
            "    if (test_result == 0) {",
            "        m->cx = m->es;",
            "        m->ds = m->cx;",
            "        m->si = 0x5251;",
            "        m->di = 0x5851;",
            "        m->cx = 0x0060;",
            "        while (m->cx != 0) {",
            "            cb_u32 value = m->read32(m->ds, m->si);",
            "            m->write32(m->es, m->di, value);",
            "            cb_advance_u16(m->si, 4, m->df);",
            "            cb_advance_u16(m->di, 4, m->df);",
            "            m->cx = (cb_u16)(m->cx - 1);",
            "        }",
            "    }",
            "    m->si = m->pop16();",
            "    m->ds = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x00A38E):
        return [
            "    cb_u16 before_si = m->si;",
            "    m->si = (cb_u16)(m->si + m->ax);",
            "    m->set_add16_flags(before_si, m->ax, m->si);",
            "    int wrap = m->cf;",
            "    if (!wrap) {",
            "        cb_u16 limit = m->read16(m->ds, 0x5233);",
            "        cb_u16 cmp_result = (cb_u16)(m->si - limit);",
            "        m->set_sub16_flags(m->si, limit, cmp_result);",
            "        if (m->si <= limit) {",
            "            wrap = 0;",
            "        } else {",
            "            wrap = 1;",
            "        }",
            "    }",
            "    if (wrap) {",
            "        m->cx = 0;",
            "        m->set_logic16_flags(m->cx);",
            "        cb_u16 old_head = m->read16(m->ds, 0x0d8c);",
            "        m->write16(m->ds, 0x0d8c, m->cx);",
            "        m->cx = old_head;",
            "        m->write16(m->ds, 0x0d98, m->cx);",
            "    }",
            "    cb_u16 before_sub = m->ax;",
            "    m->ax = (cb_u16)(m->ax - 2);",
            "    m->set_sub16_flags(before_sub, 2, m->ax);",
            "    m->write16(m->ds, 0x0da0, m->ax);",
            "    cb_u16 count = m->read16(m->ds, 0x0d62);",
            "    cb_u16 count_result = (cb_u16)(count + 1);",
            "    m->write16(m->ds, 0x0d62, count_result);",
            "    m->set_inc16_flags(count, count_result);",
            "    return;",
        ]

    if key == ("bloodprg", 0x00A3AD):
        return [
            "    m->ax = m->read16(m->ds, 0x0d8c);",
            "    m->bx = m->read16(m->ds, 0x0d90);",
            "    cb_u16 cmp_head_tail = (cb_u16)(m->ax - m->bx);",
            "    m->set_sub16_flags(m->ax, m->bx, cmp_head_tail);",
            "    if (m->ax < m->bx) {",
            "        cb_u16 before_add = m->ax;",
            "        m->ax = (cb_u16)(m->ax + m->cx);",
            "        m->set_add16_flags(before_add, m->cx, m->ax);",
            "        before_add = m->ax;",
            "        m->ax = (cb_u16)(m->ax + 0x12);",
            "        m->set_add16_flags(before_add, 0x12, m->ax);",
            "        cb_u16 cmp_tail_limit = (cb_u16)(m->bx - m->ax);",
            "        m->set_sub16_flags(m->bx, m->ax, cmp_tail_limit);",
            "        if (m->bx < m->ax) {",
            "            return;",
            "        }",
            "    }",
            "    m->ax = m->read16(m->ds, 0x0d9a);",
            "    cb_u16 before_add = m->ax;",
            "    m->ax = (cb_u16)(m->ax + 0x0a);",
            "    m->set_add16_flags(before_add, 0x0a, m->ax);",
            "    before_add = m->ax;",
            "    m->ax = (cb_u16)(m->ax + m->cx);",
            "    m->set_add16_flags(before_add, m->cx, m->ax);",
            "    if (m->cf) {",
            "        return;",
            "    }",
            "    cb_u16 capacity = m->read16(m->ds, 0x0d98);",
            "    cb_u16 cmp_capacity = (cb_u16)(capacity - m->ax);",
            "    m->set_sub16_flags(capacity, m->ax, cmp_capacity);",
            "    return;",
        ]

    if key == ("bloodprg", 0x00A634):
        return [
            "    cb_u16 saved_ax = m->ax;",
            "    cb_u16 saved_ds = m->ds;",
            "    m->ax = m->gs;",
            "    m->ds = m->ax;",
            "    cb_u8 test_result = (cb_u8)(m->read8(m->ds, 0x0b17) & 1);",
            "    m->set_logic8_flags(test_result);",
            "    m->ds = saved_ds;",
            "    m->ax = saved_ax;",
            "    return;",
        ]

    if key == ("bloodprg", 0x00A734):
        return [
            "    cb_u16 head = m->read16(m->ds, 0x0d8c);",
            "    cb_u16 head_result = (cb_u16)(head + m->ax);",
            "    m->write16(m->ds, 0x0d8c, head_result);",
            "    m->set_add16_flags(head, m->ax, head_result);",
            "    cb_u16 count = m->read16(m->ds, 0x0d9a);",
            "    cb_u16 count_result = (cb_u16)(count + m->ax);",
            "    m->write16(m->ds, 0x0d9a, count_result);",
            "    m->set_add16_flags(count, m->ax, count_result);",
            "    m->cf = 0;",
            "    return;",
        ]

    if key == ("bloodprg", 0x00A73E):
        return [
            "    m->write16(m->ds, 0x0d60, 0);",
            "    m->write16(m->ds, 0x0d62, 0);",
            "    m->write16(m->ds, 0x0d64, 0xffff);",
            "    m->write16(m->ds, 0x0d66, 0xffff);",
            "    return;",
        ]

    if key == ("bloodprg", 0x00A744):
        return [
            "    m->write16(m->ds, 0x0d62, 0);",
            "    m->write16(m->ds, 0x0d64, 0xffff);",
            "    m->write16(m->ds, 0x0d66, 0xffff);",
            "    return;",
        ]

    if key == ("bloodprg", 0x00A757):
        return [
            "    m->ax = m->read16(m->ds, 0x0a7e);",
            "    m->write16(m->ds, 0x0d8e, m->ax);",
            "    m->write16(m->ds, 0x0d92, m->ax);",
            "    m->ax = 0;",
            "    m->set_logic16_flags(m->ax);",
            "    m->write16(m->ds, 0x0d8c, m->ax);",
            "    m->write16(m->ds, 0x0d90, m->ax);",
            "    m->write16(m->ds, 0x0d9a, m->ax);",
            "    m->write16(m->ds, 0x0da0, m->ax);",
            "    m->write16(m->ds, 0x0d96, m->ax);",
            "    m->ax = m->read16(m->ds, 0x5233);",
            "    m->write16(m->ds, 0x0d98, m->ax);",
            "    return;",
        ]

    if key == ("bloodprg", 0x00A778):
        return [
            "    m->si = m->read16(m->ds, 0x0d8c);",
            "    m->es = m->read16(m->ds, 0x0d8e);",
            "    m->si = m->read16(m->ds, 0x0d9e);",
            "    m->call_near(0xa0c3);",
            "    return;",
        ]

    if key == ("bloodprg", 0x00AD96):
        return [
            "    cb_u16 row_off = (cb_u16)(m->bp - 6);",
            "    cb_u8 row_count = (cb_u8)(m->read8(m->ss, row_off) - 1);",
            "    m->write8(m->ss, row_off, row_count);",
            "    if (row_count == 0) {",
            "        cb_u16 before_add = m->sp;",
            "        m->sp = (cb_u16)(m->sp + 2);",
            "        m->set_add16_flags(before_add, 2, m->sp);",
            "        m->sp = m->bp;",
            "        m->bp = m->pop16();",
            "        m->ds = m->pop16();",
            "        return;",
            "    }",
            "    m->di = m->read16(m->ss, (cb_u16)(m->bp - 8));",
            "    cb_u16 before_add = m->di;",
            "    m->di = (cb_u16)(m->di + 0x0140);",
            "    m->set_add16_flags(before_add, 0x0140, m->di);",
            "    m->cx = m->read16(m->ss, (cb_u16)(m->bp - 0x0a));",
            "    m->write16(m->ss, (cb_u16)(m->bp - 8), m->di);",
            "    return;",
        ]

    if key == ("bloodprg", 0x00BB9D):
        return [
            "    m->push16(m->ax);",
            "    m->push16(m->ds);",
            "    m->push16(m->es);",
            "    m->ax = m->gs;",
            "    m->ds = m->ax;",
            "    m->ax = 0;",
            "    m->set_logic16_flags(m->ax);",
            "    cb_u16 target_off = m->read16(m->ds, 0x0cdf);",
            "    cb_u16 target_seg = m->read16(m->ds, 0x0ce1);",
            "    m->call_far(target_seg, target_off);",
            "    m->write8(m->ds, 0x0ba0, 0);",
            "    m->es = m->pop16();",
            "    m->ds = m->pop16();",
            "    m->ax = m->pop16();",
            "    return;",
        ]

    if key == ("bloodprg", 0x00BD8D):
        return [
            "    m->push16(m->ds);",
            "    m->push16(m->ax);",
            "    m->push16(m->bx);",
            "    m->push16(m->cx);",
            "    m->push16(m->dx);",
            "    m->cx = m->ax;",
            "    m->cx = (cb_u16)(m->cx >> 2);",
            "    m->ax = (cb_u16)(m->ax << 14);",
            "    m->dx = m->ax;",
            "    m->bx = m->read16(m->gs, 0x0c49);",
            "    m->ax = 0x4200;",
            "    m->interrupt(0x21);",
            "    m->cx = 0x4000;",
            "    cb_set_hi8(m->ax, 0x3f);",
            "    m->push16(m->es);",
            "    m->ds = m->pop16();",
            "    m->dx = m->di;",
            "    m->interrupt(0x21);",
            "    m->dx = m->pop16();",
            "    m->cx = m->pop16();",
            "    m->bx = m->pop16();",
            "    m->ax = m->pop16();",
            "    m->ds = m->pop16();",
            "    return;",
        ]

    if key in XDB_ACTOR_FIELD_SUB_TRANSLATIONS:
        cs_slot = XDB_ACTOR_FIELD_SUB_TRANSLATIONS[key]
        body = [
            "    m->si = m->read16(m->ds, (cb_u16)(m->di + 0x16));",
            "    cb_u16 before_add = m->si;",
            "    m->si = (cb_u16)(m->si + 0x5e);",
            "    m->set_add16_flags(before_add, 0x5e, m->si);",
            "    cb_u16 field_addr = (cb_u16)(m->si + 0x52);",
            "    cb_u16 field_value = m->read16(m->ds, field_addr);",
            "    cb_u16 sub_result = (cb_u16)(field_value - 0x0f);",
            "    m->write16(m->ds, field_addr, sub_result);",
            "    m->set_sub16_flags(field_value, 0x0f, sub_result);",
        ]
        if cs_slot is not None:
            body.append(f"    m->write16(m->cs, {h(cs_slot)}, m->si);")
        body.extend(
            [
                "    return;",
            ]
        )
        return body

    if key in XDB_ADD_CS99_IF_NONNEG_TRANSLATIONS:
        return [
            "    m->si = m->read16(m->ds, (cb_u16)(m->di + 0x16));",
            "    m->ax = m->read16(m->cs, 0x0099);",
            "    cb_u16 before_sar = m->ax;",
            "    m->ax = (cb_u16)((before_sar >> 1) | (before_sar & 0x8000u));",
            "    m->set_sar16_flags(before_sar, 1, m->ax);",
            "    if ((m->ax & 0x8000u) == 0) {",
            "        cb_u16 field_addr = (cb_u16)(m->si + 0x00b0);",
            "        cb_u16 field_value = m->read16(m->ds, field_addr);",
            "        cb_u16 add_result = (cb_u16)(field_value + m->ax);",
            "        m->write16(m->ds, field_addr, add_result);",
            "        m->set_add16_flags(field_value, m->ax, add_result);",
            "    }",
            "    return;",
        ]

    if key in XDB_JUMP_OR_INIT_TRANSLATIONS:
        init_target = XDB_JUMP_OR_INIT_TRANSLATIONS[key]
        return [
            "    m->bx = m->read16(m->ds, (cb_u16)(m->di + 0x36));",
            "    m->set_logic16_flags(m->bx);",
            "    if (m->bx != 0) {",
            "        m->jump_near(m->bx);",
            "        return;",
            "    }",
            f"    m->write16(m->ds, (cb_u16)(m->di + 0x36), {h(init_target)});",
            "    m->write16(m->ds, (cb_u16)(m->di + 0x38), 0);",
            "    m->write16(m->ds, (cb_u16)(m->di + 0x3a), 0);",
            "    return;",
        ]

    if key in XDB_MOUSE_RANGE_TRANSLATIONS:
        return [
            "    m->push16(m->cx);",
            "    m->ax = 8;",
            "    m->cx = 0;",
            "    m->set_logic16_flags(m->cx);",
            "    m->interrupt(0x33);",
            "    m->ax = 7;",
            "    m->dx = m->pop16();",
            "    m->cx = 0;",
            "    m->set_logic16_flags(m->cx);",
            "    m->interrupt(0x33);",
            "    return;",
        ]

    if key in XDB_MOUSE_POSITION_TRANSLATIONS:
        return [
            "    m->write16(m->ds, 0x002a, m->cx);",
            "    m->write16(m->ds, 0x002c, m->dx);",
            "    m->ax = 4;",
            "    m->interrupt(0x33);",
            "    return;",
        ]

    if key in XDB_FIELD_DELTA_TRANSLATIONS:
        return xdb_field_delta_body(sar4=False)

    if key in XDB_FIELD_DELTA_SAR4_TRANSLATIONS:
        return xdb_field_delta_body(sar4=True)

    if key == ("xdb_manu3", 0x00017C):
        return [
            "    m->call_near(0x0181);",
            "    return;",
        ]

    if key == ("xdb_manu3", 0x000181):
        return [
            "    m->bx = (cb_u16)(m->bx & 0x001f);",
            "    m->set_logic16_flags(m->bx);",
            "    cb_u16 before_add = m->bx;",
            "    m->bx = (cb_u16)(m->bx + m->bx);",
            "    m->set_add16_flags(before_add, before_add, m->bx);",
            "    m->di = m->read16(m->ds, 0x2306);",
            "    m->write16(m->ds, 0x102c, 0);",
            "    cb_u16 addend = m->read16(m->ds, (cb_u16)(m->bx + m->di));",
            "    before_add = m->di;",
            "    m->di = (cb_u16)(m->di + addend);",
            "    m->set_add16_flags(before_add, addend, m->di);",
            "    m->write16(m->ds, 0x102e, m->di);",
            "    m->bx = 0x1032;",
            "    m->jump_near(0x01df);",
            "    return;",
        ]

    if routine.cxx_status == "translated_mem_copy_words_4":
        return [
            "    m->es = m->ds;",
            "    for (int i = 0; i != 4; ++i) {",
            "        cb_u16 value = m->read16(m->ds, m->si);",
            "        m->write16(m->es, m->di, value);",
            "        if (m->df) {",
            "            m->si = (cb_u16)(m->si - 2);",
            "            m->di = (cb_u16)(m->di - 2);",
            "        } else {",
            "            m->si = (cb_u16)(m->si + 2);",
            "            m->di = (cb_u16)(m->di + 2);",
            "        }",
            "    }",
            "    return;",
        ]

    if routine.cxx_status == "translated_strlen_es_di":
        return [
            "    cb_u16 saved_cx = m->cx;",
            "    cb_u16 saved_di = m->di;",
            "    cb_u16 scan_cx = 0xffffu;",
            "    cb_u16 scan_di = m->di;",
            "",
            "    while (scan_cx != 0) {",
            "        cb_u8 value = m->read8(m->es, scan_di);",
            "        scan_di = (cb_u16)(m->df ? scan_di - 1 : scan_di + 1);",
            "        scan_cx = (cb_u16)(scan_cx - 1);",
            "        if (value == 0) {",
            "            break;",
            "        }",
            "    }",
            "",
            "    cb_u16 neg_cx = (cb_u16)(0 - scan_cx);",
            "    cb_u16 before_sub = neg_cx;",
            "    m->ax = (cb_u16)(before_sub - 2);",
            "    m->set_sub16_flags(before_sub, 2, m->ax);",
            "    m->di = saved_di;",
            "    m->cx = saved_cx;",
            "    return;",
        ]

    return None


def cpp_text(routine: Routine) -> str:
    asm_path = rel(routine.asm_path) if routine.asm_path else ""
    qualifier = routine_qualifier(routine)
    body = translated_cpp_body(routine)
    lines = [
        "// Commander Blood Borland C++ translation unit",
        f"// module: {routine.module}",
        f"// {routine.address_kind}: {h(routine.entry, 6)}",
        f"// assembly: {asm_path}",
        f"// provenance: {', '.join(sorted(routine.provenance)) or 'unknown'}",
        f"// status: {routine.cxx_status}",
        f"// reason: {routine.cxx_reason}",
        "",
        '#include "recovered.hpp"',
        "",
    ]
    for label in routine.labels:
        lines.append(f"// label: {label.name}")
    if routine.labels:
        lines.append("")

    lines.append(f'extern "C" void {qualifier} {routine.func_name}(CbMachine* m)')
    lines.append("{")
    if body is not None:
        lines.extend(body)
    else:
        lines.append(
            f'#error "Untranslated routine {routine.module}:{h(routine.entry, 6)}; see {asm_path}"'
        )
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


def header_text() -> str:
    return """#ifndef CB_RECOVERED_HPP
#define CB_RECOVERED_HPP

#if defined(__BORLANDC__)
#define CB_NEAR near
#define CB_FAR far
#else
#define CB_NEAR
#define CB_FAR
#endif

typedef unsigned char cb_u8;
typedef signed char cb_i8;
typedef unsigned short cb_u16;
typedef signed short cb_i16;
typedef unsigned long cb_u32;
typedef signed long cb_i32;

struct CbMachine {
    cb_u16 ax;
    cb_u16 bx;
    cb_u16 cx;
    cb_u16 dx;
    cb_u16 si;
    cb_u16 di;
    cb_u16 bp;
    cb_u16 sp;
    cb_u16 ds;
    cb_u16 es;
    cb_u16 fs;
    cb_u16 gs;
    cb_u16 ss;
    cb_u16 cs;
    int cf;
    int zf;
    int sf;
    int of;
    int pf;
    int af;
    int df;

    cb_u8 read8(cb_u16 seg, cb_u16 off) const;
    cb_u16 read16(cb_u16 seg, cb_u16 off) const;
    cb_u32 read32(cb_u16 seg, cb_u16 off) const;
    void write8(cb_u16 seg, cb_u16 off, cb_u8 value);
    void write16(cb_u16 seg, cb_u16 off, cb_u16 value);
    void write32(cb_u16 seg, cb_u16 off, cb_u32 value);
    cb_u8 in8(cb_u16 port);
    void out8(cb_u16 port, cb_u8 value);
    void set_logic8_flags(cb_u8 value);
    void set_logic16_flags(cb_u16 value);
    void set_add16_flags(cb_u16 left, cb_u16 right, cb_u16 result);
    void set_sub8_flags(cb_u8 left, cb_u8 right, cb_u8 result);
    void set_sub16_flags(cb_u16 left, cb_u16 right, cb_u16 result);
    void set_inc8_flags(cb_u8 before, cb_u8 result);
    void set_inc16_flags(cb_u16 before, cb_u16 result);
    void set_dec16_flags(cb_u16 before, cb_u16 result);
    void set_sar16_flags(cb_u16 before, unsigned count, cb_u16 result);
    void push16(cb_u16 value);
    cb_u16 pop16();
    void call_near(cb_u16 off);
    void call_far(cb_u16 seg, cb_u16 off);
    void interrupt(cb_u8 vector);
    void jump_near(cb_u16 off);
};

inline cb_u8 cb_lo8(cb_u16 value)
{
    return (cb_u8)(value & 0xffu);
}

inline cb_u8 cb_hi8(cb_u16 value)
{
    return (cb_u8)((value >> 8) & 0xffu);
}

inline void cb_set_lo8(cb_u16& reg, cb_u8 value)
{
    reg = (cb_u16)((reg & 0xff00u) | value);
}

inline void cb_set_hi8(cb_u16& reg, cb_u8 value)
{
    reg = (cb_u16)((reg & 0x00ffu) | ((cb_u16)value << 8));
}

inline void cb_advance_u16(cb_u16& reg, cb_u16 amount, int direction_flag)
{
    if (direction_flag) {
        reg = (cb_u16)(reg - amount);
    } else {
        reg = (cb_u16)(reg + amount);
    }
}

#endif
"""


def readme_assembly_text(counts: dict[str, int]) -> str:
    lines = [
        "# Recovered Assembly Dumps",
        "",
        "Generated by `python3 re/tools/export_routine_sources.py --clean`.",
        "",
        "Each `.asm` file is rooted at a recovered routine entrypoint and includes",
        "the provenance that made that entrypoint eligible. BLOODPRG routines are",
        "grouped by recovered MZ relative segment. XDB routines are grouped by",
        "entry/API seeds, method tables, manu3 labeled code seeds, and direct-call",
        "discovery. These groups are not claimed to be original compiler",
        "translation units unless future evidence proves that.",
        "",
        "Routine counts:",
        "",
    ]
    for module, count in sorted(counts.items()):
        lines.append(f"- `{module}`: {count}")
    lines.append("")
    return "\n".join(lines)


def readme_borland_text(counts: dict[str, int], translated: int) -> str:
    return f"""# Borland C++ Translation Workspace

Generated by `python3 re/tools/export_routine_sources.py --clean`.

The current choice is Borland C++ source (`.cpp`) because the overlays have
C++-shaped method-table dispatch, while still using plain `extern "C"` function
names for controllable linkage. This is a working choice, not proof of the
original compiler.

There is one C++ source file per recovered assembly routine. Translated files
take a `CbMachine*` context so register, segment, memory, and flag effects can
be represented explicitly while the original calling convention is still being
recovered. Every untranslated file deliberately contains `#error` until that
routine has been translated from its assembly dump. That stop gate prevents
untranslated routines from silently compiling as no-ops.

Recovered routine files: {sum(counts.values())}
Mechanically translated files: {translated}

`translated_sources.lst` lists the files that should parse today. Verify that
subset with `python3 re/tools/check_translated_borland_subset.py`. This is a
host C++ syntax preflight only. A DOSBox game run still requires a checked-in or
documented 16-bit Borland-compatible toolchain, a DOS build harness, and removal
of the remaining `#error` stop gates through faithful routine translation.
"""


def clean_generated(path: Path) -> None:
    marker = path / ".generated_by_export_routine_sources"
    if not path.exists():
        return
    if marker.exists():
        shutil.rmtree(path)
        return
    if any(path.iterdir()):
        raise SystemExit(f"{path} exists and is not marked as generated; refusing to remove it")
    path.rmdir()


def prepare_output(path: Path, clean: bool) -> None:
    if clean:
        clean_generated(path)
    path.mkdir(parents=True, exist_ok=True)
    (path / ".generated_by_export_routine_sources").write_text(
        "generated by re/tools/export_routine_sources.py\n", encoding="utf-8"
    )


def write_outputs(
    routines: list[Routine],
    data_by_path: dict[Path, bytes],
    asm_out: Path,
    cpp_out: Path,
    manifest_path: Path,
    metadata: dict[str, object],
    clean: bool,
) -> dict[str, object]:
    prepare_output(asm_out, clean)
    prepare_output(cpp_out, clean)
    write_text(cpp_out / "include" / "recovered.hpp", header_text())

    counts = collections.Counter(r.module for r in routines)
    translated = 0
    translated_paths = []
    index_rows = []
    manifest_entries = []

    for routine in sorted(routines, key=lambda r: (r.module, r.group, r.entry)):
        module_dir = "bloodprg" if routine.module == "bloodprg" else f"xdb/{routine.module[4:]}"
        asm_path = asm_out / module_dir / routine.group / f"{routine.file_stem}.asm"
        cpp_path = cpp_out / module_dir / routine.group / f"{routine.file_stem}.cpp"
        routine.asm_path = asm_path
        routine.cpp_path = cpp_path
        if routine.cxx_status.startswith("translated"):
            translated += 1
            translated_paths.append(rel(cpp_path))

        write_text(asm_path, asm_text(routine, data_by_path[routine.artifact_path]))
        write_text(cpp_path, cpp_text(routine))

        index_rows.append(
            [
                routine.module,
                h(routine.entry, 6),
                routine.group,
                " | ".join(sorted(routine.provenance)),
                " | ".join(label.name for label in routine.labels),
                rel(asm_path),
                rel(cpp_path),
                routine.cxx_status,
                routine.boundary_reason,
            ]
        )
        manifest_entries.append(
            {
                "module": routine.module,
                "entry": h(routine.entry, 6),
                "address_kind": routine.address_kind,
                "seg_off": routine.seg_off,
                "group": routine.group,
                "provenance": sorted(routine.provenance),
                "labels": [
                    {"name": label.name, "comment": label.comment}
                    for label in routine.labels
                ],
                "incoming": sorted(routine.incoming),
                "byte_count": routine.byte_count,
                "boundary_reason": routine.boundary_reason,
                "terminal": routine.terminal,
                "direct_callees": [h(x, 6) for x in sorted(routine.direct_callees)],
                "indirect_call_count": len(routine.indirect_calls),
                "asm_path": rel(asm_path),
                "cpp_path": rel(cpp_path),
                "cxx_status": routine.cxx_status,
                "cxx_reason": routine.cxx_reason,
            }
        )

    for root in (asm_out, cpp_out):
        index_path = root / "routine_index.tsv"
        with index_path.open("w", newline="", encoding="utf-8") as fh:
            writer = csv.writer(fh, delimiter="\t", lineterminator="\n")
            writer.writerow(
                [
                    "module",
                    "entry",
                    "group",
                    "provenance",
                    "labels",
                    "asm_path",
                    "cpp_path",
                    "cxx_status",
                    "boundary",
                ]
            )
            writer.writerows(index_rows)

    write_text(asm_out / "README.md", readme_assembly_text(dict(counts)))
    write_text(cpp_out / "README.md", readme_borland_text(dict(counts), translated))
    write_text(
        cpp_out / "translated_sources.lst",
        "\n".join(sorted(translated_paths)) + ("\n" if translated_paths else ""),
    )

    manifest = {
        "generator": "re/tools/export_routine_sources.py",
        "routine_count": len(routines),
        "module_counts": dict(sorted(counts.items())),
        "translated_count": translated,
        "untranslated_count": len(routines) - translated,
        "metadata": metadata,
        "entries": manifest_entries,
    }
    write_text(manifest_path, json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bloodprg", type=Path, default=DEFAULT_BLOODPRG)
    parser.add_argument("--graph", type=Path, default=DEFAULT_GRAPH)
    parser.add_argument("--xdb-dir", type=Path, default=DEFAULT_XDB_DIR)
    parser.add_argument("--asm-out", type=Path, default=DEFAULT_ASM_OUT)
    parser.add_argument("--cpp-out", type=Path, default=DEFAULT_CPP_OUT)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--max-bytes", type=int, default=8192)
    parser.add_argument("--clean", action="store_true")
    args = parser.parse_args()

    xdb_paths = [args.xdb_dir / f"{name}.xdb" for name in ("amer", "croolis", "manu3", "scrut")]
    missing = [path for path in [args.bloodprg, args.graph, *xdb_paths] if not path.exists()]
    if missing:
        raise SystemExit("missing inputs:\n" + "\n".join(str(path) for path in missing))

    blood, blood_meta = bloodprg_routines(args.bloodprg, args.graph)
    xdb, xdb_meta = xdb_routines(xdb_paths, args.max_bytes)
    routines = blood + xdb
    data_by_path = {path: path.read_bytes() for path in {r.artifact_path for r in routines}}
    decode_all(routines, data_by_path, args.max_bytes)

    manifest = write_outputs(
        routines,
        data_by_path,
        args.asm_out,
        args.cpp_out,
        args.manifest,
        clean=args.clean,
        metadata={"bloodprg": blood_meta, "xdb": xdb_meta},
    )
    print(
        json.dumps(
            {
                "routine_count": manifest["routine_count"],
                "module_counts": manifest["module_counts"],
                "translated_count": manifest["translated_count"],
                "untranslated_count": manifest["untranslated_count"],
                "asm_out": str(args.asm_out),
                "cpp_out": str(args.cpp_out),
                "manifest": str(args.manifest),
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
