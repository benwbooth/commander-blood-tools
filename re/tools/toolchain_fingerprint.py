#!/usr/bin/env python3
"""Emit MZ/code-shape fingerprints for DOS toolchain identification.

This is intentionally not a detector. It records stable structural features that
can be compared against known compiler/linker samples:

* MZ header and relocation-table shape
* relocated far call/jump targets
* segment values stored at relocation sites
* prologue/epilogue and common codegen byte-pattern censuses
* toolchain/runtime marker string hits

Use it on BLOODPRG.EXE, INSTALL.EXE, and later on small programs compiled with
candidate 1990s DOS compilers.
"""

from __future__ import annotations

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE
]

import argparse
import collections
import hashlib
import json
import string
import struct
from pathlib import Path
from typing import Iterable

sys.path.insert(0, _HERE)

from mzfile import MZ  # noqa: E402

sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE
]


TOOLCHAIN_MARKERS = [
    b"Borland",
    b"Turbo",
    b"TLINK",
    b"Watcom",
    b"WATCOM",
    b"Microsoft",
    b"QuickBASIC",
    b"BRUN",
    b"BCOM",
    b"BASCOM",
    b"VBDOS",
    b"QBX",
    b"RTM",
    b"DOS/4G",
    b"DOS4G",
    b"CauseWay",
    b"Phar Lap",
    b"DJGPP",
    b"GO32",
]


BYTE_PATTERNS = {
    # C/Pascal-ish frames and near/far returns.
    "push_bp_mov_bp_sp": bytes.fromhex("55 8b ec"),
    "mov_bp_sp": bytes.fromhex("8b ec"),
    "enter": bytes.fromhex("c8"),
    "leave": bytes.fromhex("c9"),
    "near_ret": bytes.fromhex("c3"),
    "near_ret_imm": bytes.fromhex("c2"),
    "far_ret": bytes.fromhex("cb"),
    "far_ret_imm": bytes.fromhex("ca"),
    "iret": bytes.fromhex("cf"),
    # 386-in-16-bit and segment-register usage.
    "operand_size_prefix_66": bytes.fromhex("66"),
    "address_size_prefix_67": bytes.fromhex("67"),
    "fs_prefix_64": bytes.fromhex("64"),
    "gs_prefix_65": bytes.fromhex("65"),
    "xor_eax_eax": bytes.fromhex("66 33 c0"),
    "xor_ebx_ebx": bytes.fromhex("66 33 db"),
    "xor_esi_esi": bytes.fromhex("66 33 f6"),
    "xor_edi_edi": bytes.fromhex("66 33 ff"),
    "xor_ebp_ebp": bytes.fromhex("66 33 ed"),
    # String/memory idioms often owned by CRTs or compiler lowering.
    "rep_movsb": bytes.fromhex("f3 a4"),
    "rep_movsw_or_movsd": bytes.fromhex("f3 a5"),
    "rep_stosb": bytes.fromhex("f3 aa"),
    "rep_stosw_or_stosd": bytes.fromhex("f3 ab"),
    "cld": bytes.fromhex("fc"),
    "std": bytes.fromhex("fd"),
    # Process exit spellings.
    "int21_4c00": bytes.fromhex("b8 00 4c cd 21"),
    "int21_004c": bytes.fromhex("b8 4c 00 cd 21"),
}


PROLOGUE_START_BYTES = {
    0x06,  # push es
    0x0E,  # push cs
    0x16,  # push ss
    0x1E,  # push ds
    0x55,  # push bp
    0x56,  # push si
    0x57,  # push di
    0x60,  # pusha
}
PROLOGUE_START_BYTES.update(range(0x50, 0x58))  # push ax/cx/dx/bx/sp/bp/si/di


PRINTABLE = set(bytes(string.printable, "ascii")) - set(b"\x0b\x0c\r\n\t")


def h(n: int, width: int = 0) -> str:
    if width:
        return f"0x{n:0{width}x}"
    return f"0x{n:x}"


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def find_all(data: bytes, needle: bytes, start: int, end: int) -> list[int]:
    out: list[int] = []
    pos = start
    while pos < end:
        hit = data.find(needle, pos, end)
        if hit < 0:
            break
        out.append(hit)
        pos = hit + 1
    return out


def sample_hits(hits: list[int], limit: int) -> dict[str, object]:
    return {
        "count": len(hits),
        "first_file_offsets": [h(x, 6) for x in hits[:limit]],
    }


def counter_to_hex_map(counter: collections.Counter[int]) -> dict[str, int]:
    return {h(key, 4): value for key, value in sorted(counter.items())}


def relocation_profile(mz: MZ, sample_limit: int) -> dict[str, object]:
    site_offsets = [seg * 16 + off for seg, off in mz.relocs]
    stored_values: collections.Counter[int] = collections.Counter()
    site_segments: collections.Counter[int] = collections.Counter(seg for seg, _ in mz.relocs)
    for image_off in site_offsets:
        if image_off + 1 < len(mz.image):
            stored_values[struct.unpack_from("<H", mz.image, image_off)[0]] += 1

    order_changes = 0
    for prev, cur in zip(site_offsets, site_offsets[1:]):
        if cur < prev:
            order_changes += 1

    entries = [
        {
            "segment": h(seg, 4),
            "offset": h(off, 4),
            "image_offset": h(seg * 16 + off, 5),
            "file_offset": h(mz.image_to_file(seg * 16 + off), 6),
        }
        for seg, off in mz.relocs
    ]
    return {
        "count": len(mz.relocs),
        "table_file_offset": h(mz.e_lfarlc, 4),
        "site_order_is_monotonic": order_changes == 0,
        "site_order_backtracks": order_changes,
        "site_segment_histogram": counter_to_hex_map(site_segments),
        "stored_segment_value_histogram": counter_to_hex_map(stored_values),
        "first_entries": entries[:sample_limit],
        "last_entries": entries[-sample_limit:] if entries else [],
    }


def far_transfer_profile(mz: MZ, sample_limit: int) -> dict[str, object]:
    transfers = []
    segment_hist: collections.Counter[int] = collections.Counter()
    target_hist: collections.Counter[tuple[int, int]] = collections.Counter()
    for opcode, kind in ((0x9A, "call"), (0xEA, "jmp")):
        i = mz.header_size
        while i < mz.image_total - 5:
            if mz.data[i] == opcode:
                off, seg = struct.unpack_from("<HH", mz.data, i + 1)
                seg_operand_image = mz.file_to_image(i + 3)
                if seg_operand_image in mz.reloc_image_offsets:
                    target_file = mz.segoff_to_file(seg, off)
                    item = {
                        "site_file_offset": h(i, 6),
                        "kind": kind,
                        "target": f"{h(seg, 4)}:{h(off, 4)}",
                        "target_file_offset": h(target_file, 6),
                    }
                    transfers.append(item)
                    segment_hist[seg] += 1
                    target_hist[(seg, off)] += 1
            i += 1

    most_common_targets = [
        {
            "target": f"{h(seg, 4)}:{h(off, 4)}",
            "target_file_offset": h(mz.segoff_to_file(seg, off), 6),
            "count": count,
        }
        for (seg, off), count in target_hist.most_common(sample_limit)
    ]
    return {
        "count": len(transfers),
        "distinct_target_count": len(target_hist),
        "target_segment_histogram": counter_to_hex_map(segment_hist),
        "most_common_targets": most_common_targets,
        "first_transfers": transfers[:sample_limit],
    }


def segment_profile(mz: MZ) -> dict[str, object]:
    stored = collections.Counter()
    for image_off in mz.reloc_image_offsets:
        if image_off + 1 < len(mz.image):
            stored[struct.unpack_from("<H", mz.image, image_off)[0]] += 1
    far_segments = {
        int(seg, 16)
        for seg in far_transfer_profile(mz, 0)["target_segment_histogram"].keys()
    }
    bases = sorted(set(stored.keys()) | far_segments)
    return {
        "base_count": len(bases),
        "bases": [
            {
                "segment": h(seg, 4),
                "first_file_offset": h(mz.segoff_to_file(seg, 0), 6),
                "stored_reloc_uses": stored.get(seg, 0),
                "is_far_target_segment": seg in far_segments,
            }
            for seg in bases
        ],
    }


def byte_pattern_profile(mz: MZ, sample_limit: int) -> dict[str, object]:
    return {
        name: sample_hits(find_all(mz.data, pattern, mz.header_size, mz.image_total), sample_limit)
        for name, pattern in BYTE_PATTERNS.items()
    }


def ret_preceded_prologues(mz: MZ, sample_limit: int) -> dict[str, object]:
    starts: list[int] = []
    for off in range(mz.header_size + 1, mz.image_total):
        if mz.data[off] not in PROLOGUE_START_BYTES:
            continue
        prev = mz.data[off - 1]
        direct = prev in {0xC3, 0xCB, 0xCF}
        imm = off >= mz.header_size + 3 and mz.data[off - 3] in {0xC2, 0xCA}
        if direct or imm:
            starts.append(off)

    start_byte_hist = collections.Counter(mz.data[off] for off in starts)
    return {
        "count": len(starts),
        "start_byte_histogram": {h(k, 2): v for k, v in sorted(start_byte_hist.items())},
        "first_file_offsets": [h(off, 6) for off in starts[:sample_limit]],
    }


def marker_profile(mz: MZ, sample_limit: int) -> dict[str, object]:
    lower = mz.data.lower()
    out = {}
    for marker in TOOLCHAIN_MARKERS:
        hits = find_all(lower, marker.lower(), 0, len(lower))
        if hits:
            out[marker.decode("ascii", errors="replace")] = sample_hits(hits, sample_limit)
    return out


def printable_strings(data: bytes, min_len: int) -> Iterable[tuple[int, str]]:
    pos = 0
    while pos < len(data):
        end = pos
        while end < len(data) and data[end] in PRINTABLE and data[end] != 0:
            end += 1
        if end - pos >= min_len:
            text = data[pos:end].decode("latin1")
            if sum(ch.isalpha() for ch in text) >= 3:
                yield pos, text
        pos = max(end + 1, pos + 1)


def string_profile(mz: MZ, sample_limit: int) -> dict[str, object]:
    strings = list(printable_strings(mz.data, 5))
    long_strings = [(off, text) for off, text in strings if len(text) >= 16]
    return {
        "printable_string_count_min5": len(strings),
        "long_printable_string_count_min16": len(long_strings),
        "first_long_strings": [
            {"file_offset": h(off, 6), "text": text[:120]} for off, text in long_strings[:sample_limit]
        ],
    }


def profile(path: Path, sample_limit: int) -> dict[str, object]:
    mz = MZ(str(path))
    data = mz.data
    return {
        "path": str(path),
        "basename": path.name,
        "sha256": sha256(data),
        "mz": mz.summary(),
        "image_sha256": sha256(mz.image),
        "relocations": relocation_profile(mz, sample_limit),
        "segments": segment_profile(mz),
        "far_transfers": far_transfer_profile(mz, sample_limit),
        "byte_patterns": byte_pattern_profile(mz, sample_limit),
        "ret_preceded_prologue_candidates": ret_preceded_prologues(mz, sample_limit),
        "toolchain_marker_strings": marker_profile(mz, sample_limit),
        "strings": string_profile(mz, sample_limit),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="+", type=Path, help="DOS MZ executable(s) to profile")
    parser.add_argument(
        "--sample-limit",
        type=int,
        default=16,
        help="Maximum sample offsets/strings to include per feature; default 16",
    )
    args = parser.parse_args()

    profiles = [profile(path, args.sample_limit) for path in args.paths]
    print(json.dumps({"profiles": profiles}, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
