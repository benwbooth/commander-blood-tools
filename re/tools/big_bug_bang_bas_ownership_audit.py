#!/usr/bin/env python3
"""Audit every native path that can enter BBB's separate BAS stream."""

import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import sys

# re/tools/dis.py shadows the stdlib module imported by Capstone through
# inspect, so remove this script directory until Capstone is loaded.
_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE
]

from capstone import CS_AC_WRITE, CS_ARCH_X86, CS_MODE_16, Cs
from capstone.x86_const import X86_OP_IMM, X86_OP_MEM, X86_OP_REG


EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
CODE_START = 0x0800
CODE_END = 0xE190
FIELD_RESOLVER = 0x6633
BAS_DISPATCH = 0x5BAE
BAS_CONTINUE = 0x6104
BAS_TEXT_SCAN = 0x83E2
BAS_POINTER_LOAD = bytes.fromhex("65 c5 36 f8 6a")

# These four resolver sites have an instruction between MOV AX and CALL or a
# one-instruction selector branch. Every other site has MOV AX,imm16 directly
# before the call.
NON_ADJACENT_SELECTORS = {
    0x671C: (0x6711, bytes.fromhex("b8 09 00 3b 10 74 01 40 bb 00 01 e8 14 ff"), (9, 10)),
    0x6764: (0x6759, bytes.fromhex("b8 09 00 3b 11 74 01 40 bb 00 01 e8 cc fe"), (9, 10)),
    0x6841: (0x683B, bytes.fromhex("b8 05 00 bb 02 00 e8 ef fd"), (5,)),
    0x7854: (0x784E, bytes.fromhex("b8 13 00 bb 10 00 e8 dc ed"), (19,)),
}

# Decoding at every byte deliberately over-approximates possible instruction
# starts. These candidates begin inside the listed real instructions.
FALSE_DISP26_WRITE_STARTS = {
    0x11F0: (0x11EC, bytes.fromhex("f6 06 92 6b 01")),
    0x16FA: (0x16F6, bytes.fromhex("9a c7 0b 03 08")),
    0x7014: (0x7011, bytes.fromhex("f7 04 02 00")),
    0x77ED: (0x77EB, bytes.fromhex("83 f8 01")),
    0x79B3: (0x79B2, bytes.fromhex("0a d2")),
    0x8067: (0x8064, bytes.fromhex("81 fb 00 01")),
    0x83BC: (0x83B7, bytes.fromhex("65 f6 06 91 6b 01")),
    0x89C8: (0x89C4, bytes.fromhex("f6 06 f1 0c 01")),
    0xAC5F: (0xAC5B, bytes.fromhex("f6 06 91 6b 01")),
    0xC273: (0xC272, bytes.fromhex("03 d2")),
}

# Decoding one byte before these instructions treats the ModR/M byte of the
# preceding MOV SI,BX as a REP prefix for the ADD. The real instructions below
# all advance the DOS DTA pointer to its file-size field.
FALSE_LITERAL26_REGISTER_STARTS = {
    offset - 1: (offset - 2, bytes.fromhex("8b f3"))
    for offset in (0x2C6E, 0x2CA7, 0x2D97, 0x2E5F, 0x2FA9, 0xB7D6)
}

DTA_FILE_SIZE_ADVANCES = {
    0x2C6E: (0x2C67, bytes.fromhex(
        "b8 00 2f cd 21 8b f3 83 c6 1a b9 18 00 b8 00 4e cd 21 72 04 66 26 8b"
    )),
    0x2CA7: (0x2CA0, bytes.fromhex(
        "b8 00 2f cd 21 8b f3 83 c6 1a 33 c9 b8 00 4e cd 21 66 26 8b 04 66 65"
    )),
    0x2D97: (0x2D90, bytes.fromhex(
        "b8 00 2f cd 21 8b f3 83 c6 1a 33 c9 b8 00 4e cd 21 66 26 8b 04 66 65"
    )),
    0x2E5F: (0x2E58, bytes.fromhex(
        "b8 00 2f cd 21 8b f3 83 c6 1a 33 c9 b8 00 4e cd 21 66 26 8b 0c 66 65"
    )),
    0x2FA9: (0x2FA2, bytes.fromhex(
        "b8 00 2f cd 21 8b f3 83 c6 1a 33 c9 b8 00 4e cd 21 26 8b 2c 07 8b cd"
    )),
    0xB7D6: (0xB7CF, bytes.fromhex(
        "b8 00 2f cd 21 8b f3 83 c6 1a 33 c9 b8 00 4e cd 21 66 26 8b 04 66 a3"
    )),
}

FILE_SEEK_ADVANCE = (
    0xD623,
    0xD613,
    bytes.fromhex(
        "b8 00 42 66 33 c9 65 8b 0e 84 0c 65 8b 16 82 0c "
        "83 c2 1a cd 21"
    ),
)


def near_calls(image, target):
    calls = []
    for offset in range(CODE_START, min(CODE_END, len(image)) - 2):
        if image[offset] != 0xE8:
            continue
        displacement = int.from_bytes(image[offset + 1:offset + 3], "little", signed=True)
        if (offset + 3 + displacement) & 0xFFFF == target:
            calls.append(offset)
    return calls


def resolver_selectors(image, call):
    special = NON_ADJACENT_SELECTORS.get(call)
    if special is not None:
        start, expected, selectors = special
        if image[start:start + len(expected)] != expected:
            raise ValueError(f"changed field-resolver context at {call:#06x}")
        return selectors
    if image[call - 3] != 0xB8:
        raise ValueError(f"unclassified field-resolver input at {call:#06x}")
    return (int.from_bytes(image[call - 2:call], "little"),)


def find_bytes(image, needle):
    offsets = []
    start = 0
    while (offset := image.find(needle, start, CODE_END)) >= 0:
        offsets.append(offset)
        start = offset + 1
    return offsets


def potential_disp26_writes(image):
    decoder = Cs(CS_ARCH_X86, CS_MODE_16)
    decoder.detail = True
    candidates = []
    for offset in range(CODE_START, min(CODE_END, len(image))):
        instruction = next(decoder.disasm(image[offset:offset + 15], offset, count=1), None)
        if instruction is None:
            continue
        if any(
            operand.type == X86_OP_MEM
            and operand.mem.disp == 26
            and operand.access & CS_AC_WRITE
            for operand in instruction.operands
        ):
            candidates.append((offset, instruction.mnemonic, instruction.op_str))
    return candidates


def potential_literal26_register_writes(image):
    """Find address-sized register writes that use literal 26.

    This deliberately decodes at every byte. It covers ADD/SUB-style pointer
    arithmetic, MOV of a literal offset, and LEA with displacement 26 without
    depending on a single linear-disassembly choice.
    """
    decoder = Cs(CS_ARCH_X86, CS_MODE_16)
    decoder.detail = True
    candidates = []
    for offset in range(CODE_START, min(CODE_END, len(image))):
        instruction = next(decoder.disasm(image[offset:offset + 15], offset, count=1), None)
        if instruction is None or len(instruction.operands) < 2:
            continue
        destination, source = instruction.operands[:2]
        writes_address_register = (
            destination.type == X86_OP_REG
            and destination.size >= 2
            and destination.access & CS_AC_WRITE
        )
        uses_literal_26 = source.type == X86_OP_IMM and source.imm == 26
        uses_displacement_26 = (
            instruction.mnemonic == "lea"
            and source.type == X86_OP_MEM
            and source.mem.disp == 26
        )
        if writes_address_register and (uses_literal_26 or uses_displacement_26):
            candidates.append((offset, instruction.mnemonic, instruction.op_str))
    return candidates


def audit(image):
    digest = hashlib.sha256(image).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise ValueError(f"unrecognized BLOOD2PG.EXE SHA-256 {digest}")

    calls = near_calls(image, FIELD_RESOLVER)
    if len(calls) != 54:
        raise ValueError(f"expected 54 field-resolver calls, found {len(calls)}")
    selectors = {call: resolver_selectors(image, call) for call in calls}
    selector_2_calls = [call for call, values in selectors.items() if 2 in values]
    if selector_2_calls != [0x5E59, 0x8420]:
        raise ValueError(f"changed selector-2 call sites: {selector_2_calls}")
    if image[0x5E5C:0x5E66] != bytes.fromhex("66 98 67 8b 1c 30 0b db 74 03"):
        raise ValueError("BAS handoff no longer reads selector 2 before dispatch")
    if image[0x8423:0x842C] != bytes.fromhex("03 f8 26 8b 05 0b c0 74 19"):
        raise ValueError("BAS text scan no longer reads selector 2")

    bas_pointer_loads = find_bytes(image, BAS_POINTER_LOAD)
    if bas_pointer_loads != [0x5BBA, 0x6109, 0x8413]:
        raise ValueError(f"changed BAS pointer consumers: {bas_pointer_loads}")
    call_graph = {
        BAS_DISPATCH: near_calls(image, BAS_DISPATCH),
        BAS_CONTINUE: near_calls(image, BAS_CONTINUE),
        BAS_TEXT_SCAN: near_calls(image, BAS_TEXT_SCAN),
    }
    expected_call_graph = {
        BAS_DISPATCH: [0x5E66],
        BAS_CONTINUE: [0x5C07],
        BAS_TEXT_SCAN: [0x63EE, 0x6416],
    }
    if call_graph != expected_call_graph:
        raise ValueError(f"changed BAS call graph: {call_graph}")

    writes = potential_disp26_writes(image)
    write_offsets = [offset for offset, _mnemonic, _operands in writes]
    expected_writes = sorted([0x481F, 0x4820, *FALSE_DISP26_WRITE_STARTS])
    if write_offsets != expected_writes:
        raise ValueError(f"changed displacement-26 write candidates: {writes}")
    for candidate, (start, expected) in FALSE_DISP26_WRITE_STARTS.items():
        if image[start:start + len(expected)] != expected:
            raise ValueError(f"changed containing instruction for false start {candidate:#06x}")
        if not start < candidate < start + len(expected):
            raise ValueError(f"false start {candidate:#06x} is not inside its instruction")
    sound_slot = bytes.fromhex(
        "50 57 c1 e0 05 bf e2 65 03 f8 65 8b 05 0a c0 79 18 "
        "65 89 5d 18 65 89 4d 1a 65 89 55 1c 65 89 6d 1e"
    )
    if image[0x480A:0x480A + len(sound_slot)] != sound_slot:
        raise ValueError("changed non-script GS:0x65E2 sound-slot write")

    literal26_register_writes = potential_literal26_register_writes(image)
    literal26_offsets = [offset for offset, _mnemonic, _operands in literal26_register_writes]
    expected_literal26_offsets = sorted([
        *FALSE_LITERAL26_REGISTER_STARTS,
        *DTA_FILE_SIZE_ADVANCES,
        FILE_SEEK_ADVANCE[0],
    ])
    if literal26_offsets != expected_literal26_offsets:
        raise ValueError(
            f"changed literal-26 register-write candidates: {literal26_register_writes}"
        )
    for candidate, (start, expected) in FALSE_LITERAL26_REGISTER_STARTS.items():
        if image[start:start + len(expected)] != expected:
            raise ValueError(f"changed containing instruction for false start {candidate:#06x}")
        if not start < candidate < start + len(expected):
            raise ValueError(f"false start {candidate:#06x} is not inside its instruction")
    for offset, (start, expected) in DTA_FILE_SIZE_ADVANCES.items():
        if image[start:start + len(expected)] != expected:
            raise ValueError(f"changed DOS DTA file-size context at {offset:#06x}")
    seek_offset, seek_start, seek_expected = FILE_SEEK_ADVANCE
    if image[seek_start:seek_start + len(seek_expected)] != seek_expected:
        raise ValueError(f"changed DOS file-seek context at {seek_offset:#06x}")

    counts = Counter()
    for values in selectors.values():
        for selector in values:
            counts[selector] += 1
    return {
        "format": "big-bug-bang-bas-ownership-v1",
        "executable_sha256": digest,
        "field_resolver": {
            "file_offset": FIELD_RESOLVER,
            "call_count": len(calls),
            "possible_selector_counts": {str(key): counts[key] for key in sorted(counts)},
            "selector_2_calls": [
                {"file_offset": 0x5E59, "access": "read_for_bas_dispatch"},
                {"file_offset": 0x8420, "access": "read_for_bas_text_scan"},
            ],
        },
        "bas_pointer_loads": bas_pointer_loads,
        "bas_call_graph": {
            f"{target:#06x}": callers for target, callers in sorted(call_graph.items())
        },
        "direct_displacement_26_write_audit": {
            "potential_instruction_starts": write_offsets,
            "false_starts_inside_other_instructions": sorted(FALSE_DISP26_WRITE_STARTS),
            "actual_write": {
                "file_offset": 0x481F,
                "segment": "gs",
                "base": 0x65E2,
                "record_stride": 32,
                "ownership": "non_script_sound_slot_table",
            },
        },
        "literal_26_address_formation_audit": {
            "potential_instruction_starts": literal26_offsets,
            "false_starts_inside_other_instructions": sorted(
                FALSE_LITERAL26_REGISTER_STARTS
            ),
            "dos_dta_file_size_advances": sorted(DTA_FILE_SIZE_ADVANCES),
            "dos_file_seek_advance": seek_offset,
        },
        "conclusion": (
            "The pinned executable has no native semantic mutation that can "
            "introduce a nonzero actor selector 2. Both selector-2 resolver "
            "calls read it, every BAS consumer is downstream of those reads, "
            "and direct or literal-26 address formation finds no script-state writer."
        ),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    report = audit(args.executable.read_bytes())
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(
        f"verified {report['field_resolver']['call_count']} resolver calls, "
        f"{len(report['bas_pointer_loads'])} BAS consumers, and no actor selector-2 mutation"
    )


if __name__ == "__main__":
    main()
