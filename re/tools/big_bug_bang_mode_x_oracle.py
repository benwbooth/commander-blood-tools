#!/usr/bin/env python3
"""Verify BBB's unchanged Mode X initialization and DAC-clear helper."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import sys
from typing import Any

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE]

from unicorn import (  # noqa: E402
    Uc,
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_INSN,
    UC_HOOK_INTR,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
)
from unicorn.x86_const import (  # noqa: E402
    UC_X86_INS_IN,
    UC_X86_INS_OUT,
    UC_X86_REG_AX,
    UC_X86_REG_BP,
    UC_X86_REG_BX,
    UC_X86_REG_CS,
    UC_X86_REG_DS,
    UC_X86_REG_EAX,
    UC_X86_REG_EBP,
    UC_X86_REG_EBX,
    UC_X86_REG_ECX,
    UC_X86_REG_EDI,
    UC_X86_REG_EDX,
    UC_X86_REG_EFLAGS,
    UC_X86_REG_ES,
    UC_X86_REG_ESI,
    UC_X86_REG_FS,
    UC_X86_REG_GS,
    UC_X86_REG_IP,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_EXECUTABLE = ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE"
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_mode_x.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER_SIZE = 0x800

MODE_X_ENTRY = 0x0E21
MODE_X_END = 0x0EBB
PALETTE_CLEAR_ENTRY = 0x3326
PALETTE_CLEAR_END = 0x333B
ROUTINES = (
    (
        MODE_X_ENTRY,
        MODE_X_END,
        "vga_mode_x_initialize",
        "95ae27bfac44ef4bc3544d9a1f8d6d0a1590cd109bbea541b21144c4885a9558",
    ),
    (
        PALETTE_CLEAR_ENTRY,
        PALETTE_CLEAR_END,
        "vga_dac_clear",
        "ce29048be80390a42628857ed4689ef168bf4364ca84cda03b34bddda6a697ab",
    ),
)

GLOBALS_SEGMENT = 0x3000
DECOY_SEGMENT = 0x4000
STACK_SEGMENT = 0x9000
VIDEO_SEGMENT = 0xA000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFE00
RETURN_OFFSET = 0x7000
STACK_SENTINEL = bytes.fromhex("a55a6996c33c5aa5")
BDA_CRTC_OFFSET = 0x0463
CRTC_BASE = 0x0C96
FONT_POINTER = 0x55F5
SAVED_VIDEO_MODE = 0x5602
PALETTE_WRITE_COUNT = 769
PALETTE_COMPONENT_COUNT = 768

GENERAL_REGISTERS = {
    "eax": UC_X86_REG_EAX,
    "ebx": UC_X86_REG_EBX,
    "ecx": UC_X86_REG_ECX,
    "edx": UC_X86_REG_EDX,
    "esi": UC_X86_REG_ESI,
    "edi": UC_X86_REG_EDI,
    "ebp": UC_X86_REG_EBP,
}
SEGMENT_REGISTERS = {
    "ds": UC_X86_REG_DS,
    "es": UC_X86_REG_ES,
    "fs": UC_X86_REG_FS,
    "gs": UC_X86_REG_GS,
    "ss": UC_X86_REG_SS,
}
LOGIC_FLAGS = {"cf": 0x001, "pf": 0x004, "zf": 0x040, "sf": 0x080, "of": 0x800}
CASES: tuple[dict[str, Any], ...] = (
    {
        "name": "color_crtc",
        "saved_mode": 3,
        "crtc": 0x03D4,
        "reads": (0xFF, 0xA5, 0x5A, 0xC3, 0x3C, 0x96),
        "font": (0x1234, 0xF000),
    },
    {
        "name": "mono_crtc",
        "saved_mode": 7,
        "crtc": 0x03B4,
        "reads": (0x10, 0x02, 0x08, 0x40, 0x00, 0x20),
        "font": (0x5678, 0xE000),
    },
    {
        "name": "wrapping_crtc_port",
        "saved_mode": 0xFF,
        "crtc": 0xFFFF,
        "reads": (0x00, 0xFF, 0x04, 0xFF, 0x80, 0xDF),
        "font": (0xFFFF, 0x0000),
    },
)


def loaded(file_offset: int) -> int:
    return file_offset - HEADER_SIZE


def initial_registers(case_index: int) -> dict[str, int]:
    return {
        "eax": 0xA5A51234 + case_index,
        "ebx": 0xB6B62468 + case_index,
        "ecx": 0xC7C7369C + case_index,
        "edx": 0xD8D855AA + case_index,
        "esi": 0xE9E96789 + case_index,
        "edi": 0xFAFA789A + case_index,
        "ebp": 0xABCD1357 + case_index,
        "ds": DECOY_SEGMENT,
        "es": 0x4800,
        "fs": 0x5000,
        "gs": GLOBALS_SEGMENT,
        "ss": STACK_SEGMENT,
    }


def snapshot_registers(machine: Uc) -> dict[str, int]:
    result = {
        name: machine.reg_read(register) for name, register in GENERAL_REGISTERS.items()
    }
    result.update(
        {
            name: machine.reg_read(register)
            for name, register in SEGMENT_REGISTERS.items()
        }
    )
    return result


def assert_unchanged_outside(
    before: bytes | bytearray, after: bytes | bytearray, allowed: set[int]
) -> None:
    differences = {
        index for index, (old, new) in enumerate(zip(before, after)) if old != new
    }
    assert differences <= allowed, sorted(hex(index) for index in differences)


def mode_x_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    module = executable[HEADER_SIZE:]
    for case_index, case in enumerate(CASES):
        name = str(case["name"])
        saved_mode = int(case["saved_mode"])
        crtc = int(case["crtc"])
        font_offset, font_segment = tuple(case["font"])
        scripted_reads = list(case["reads"])
        initial = initial_registers(case_index)

        module_before = bytearray(module)
        struct.pack_into("<H", module_before, BDA_CRTC_OFFSET, crtc)
        globals_before = bytearray(
            (index * 37 + case_index * 11 + 5) & 0xFF for index in range(SEGMENT_SIZE)
        )
        decoy_before = bytes(
            (index * 19 + case_index * 7 + 0x69) & 0xFF for index in range(SEGMENT_SIZE)
        )
        video_before = bytes((0xA5 ^ case_index,)) * SEGMENT_SIZE
        stack_before = bytearray(
            (index * 13 + case_index * 17 + 0x3C) & 0xFF
            for index in range(SEGMENT_SIZE)
        )
        stack_before[CALLER_SP : CALLER_SP + 4] = struct.pack("<HH", RETURN_OFFSET, 0)
        stack_before[CALLER_SP + 4 : CALLER_SP + 4 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0xB1000)
        machine.mem_write(0, module)
        machine.mem_write(BDA_CRTC_OFFSET, struct.pack("<H", crtc))
        machine.mem_write(GLOBALS_SEGMENT * 16, bytes(globals_before))
        machine.mem_write(DECOY_SEGMENT * 16, decoy_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        machine.mem_write(VIDEO_SEGMENT * 16, video_before)
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0293)

        interrupts: list[dict[str, int]] = []
        inputs: list[list[int]] = []
        outputs: list[list[int]] = []
        helper_calls: list[dict[str, int]] = []
        write_counts = {"global": 0, "stack": 0, "video": 0}
        allowed_global_offsets = {
            CRTC_BASE,
            CRTC_BASE + 1,
            FONT_POINTER,
            FONT_POINTER + 1,
            FONT_POINTER + 2,
            FONT_POINTER + 3,
            SAVED_VIDEO_MODE,
        }
        allowed_global_addresses = {
            GLOBALS_SEGMENT * 16 + offset for offset in allowed_global_offsets
        }
        allowed_stack_offsets = set(range(CALLER_SP - 24, CALLER_SP))
        allowed_stack_addresses = {
            STACK_SEGMENT * 16 + offset for offset in allowed_stack_offsets
        }
        allowed_video_addresses = set(
            range(VIDEO_SEGMENT * 16, VIDEO_SEGMENT * 16 + 0xFFFF)
        )
        expected_input_ports = [
            0x03CF,
            0x03CF,
            0x03C5,
            (crtc + 1) & 0xFFFF,
            (crtc + 1) & 0xFFFF,
            (crtc + 1) & 0xFFFF,
        ]

        def instruction(cpu: Uc, address: int, size: int, _context: Any) -> None:
            spans = (
                (loaded(MODE_X_ENTRY), loaded(MODE_X_END)),
                (loaded(PALETTE_CLEAR_ENTRY), loaded(PALETTE_CLEAR_END)),
            )
            assert any(
                start <= address and address + size <= end for start, end in spans
            ), hex(address)
            if address == loaded(PALETTE_CLEAR_ENTRY):
                sp = cpu.reg_read(UC_X86_REG_SP)
                helper_calls.append(
                    {
                        "return_offset": struct.unpack(
                            "<H", cpu.mem_read(STACK_SEGMENT * 16 + sp, 2)
                        )[0],
                        "return_segment": struct.unpack(
                            "<H", cpu.mem_read(STACK_SEGMENT * 16 + sp + 2, 2)
                        )[0],
                    }
                )

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            _value: int,
            _context: Any,
        ) -> None:
            touched = set(range(address, address + size))
            if touched <= allowed_global_addresses:
                write_counts["global"] += 1
            elif touched <= allowed_stack_addresses:
                write_counts["stack"] += 1
            elif touched <= allowed_video_addresses:
                write_counts["video"] += size
            else:
                raise AssertionError(hex(address))

        def interrupt(cpu: Uc, number: int, _context: Any) -> None:
            assert number == 0x10
            call_index = len(interrupts)
            interrupts.append(
                {
                    "number": number,
                    "ax": cpu.reg_read(UC_X86_REG_AX),
                    "bx": cpu.reg_read(UC_X86_REG_BX),
                }
            )
            if call_index == 0:
                cpu.reg_write(UC_X86_REG_AX, 0x0F00 | saved_mode)
                cpu.reg_write(UC_X86_REG_BX, 0x55A5)
            elif call_index == 1:
                cpu.reg_write(UC_X86_REG_BX, 0x66B6)
            elif call_index == 2:
                cpu.reg_write(UC_X86_REG_BP, font_offset)
                cpu.reg_write(UC_X86_REG_ES, font_segment)
            else:
                raise AssertionError(f"{name}: unexpected BIOS call")

        def input_port(_cpu: Uc, port: int, size: int, _context: Any) -> int:
            assert scripted_reads, f"{name}: extra port read"
            assert [port, size] == [expected_input_ports[len(inputs)], 1]
            value = scripted_reads.pop(0)
            inputs.append([port, size, value])
            return value

        def output_port(
            _cpu: Uc, port: int, size: int, value: int, _context: Any
        ) -> None:
            outputs.append([port, size, value])

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.hook_add(UC_HOOK_INTR, interrupt)
        machine.hook_add(UC_HOOK_INSN, input_port, None, 1, 0, UC_X86_INS_IN)
        machine.hook_add(UC_HOOK_INSN, output_port, None, 1, 0, UC_X86_INS_OUT)
        machine.emu_start(loaded(MODE_X_ENTRY), RETURN_OFFSET, count=100_000)

        expected_interrupts = [
            {
                "number": 0x10,
                "ax": 0x0F00 | (initial["eax"] & 0xFF),
                "bx": initial["ebx"] & 0xFFFF,
            },
            {"number": 0x10, "ax": 0x0013, "bx": 0x55A5},
            {"number": 0x10, "ax": 0x1130, "bx": 0x03B6},
        ]
        assert interrupts == expected_interrupts
        assert helper_calls == [{"return_offset": 0x065A, "return_segment": 0}], (
            helper_calls
        )
        assert not scripted_reads
        first, second, third, fourth, fifth, sixth = tuple(case["reads"])
        expected_mode_outputs = [
            [0x03CE, 1, 5],
            [0x03CF, 1, first & 0xEF],
            [0x03CE, 1, 6],
            [0x03CF, 1, second & 0xFD],
            [0x03C4, 1, 4],
            [0x03C5, 1, (third & 0xF7) | 4],
            [crtc, 1, 0x14],
            [(crtc + 1) & 0xFFFF, 1, fourth & 0xBF],
            [crtc, 1, 0x17],
            [(crtc + 1) & 0xFFFF, 1, fifth | 0x40],
            [crtc, 1, 0x11],
            [(crtc + 1) & 0xFFFF, 1, sixth | 0x20],
            [0x03C4, 2, 0x0F02],
        ]
        expected_palette_outputs = [[0x03C8, 1, 0]] + [
            [0x03C9, 1, 0]
        ] * PALETTE_COMPONENT_COUNT
        assert outputs == expected_palette_outputs + expected_mode_outputs

        globals_after = bytes(machine.mem_read(GLOBALS_SEGMENT * 16, SEGMENT_SIZE))
        globals_expected = bytearray(globals_before)
        struct.pack_into("<H", globals_expected, CRTC_BASE, crtc)
        struct.pack_into(
            "<HH", globals_expected, FONT_POINTER, font_offset, font_segment
        )
        globals_expected[SAVED_VIDEO_MODE] = saved_mode
        assert globals_after == bytes(globals_expected)
        video_after = bytes(machine.mem_read(VIDEO_SEGMENT * 16, SEGMENT_SIZE))
        assert video_after[:0xFFFF] == bytes(0xFFFF)
        assert video_after[0xFFFF:] == video_before[0xFFFF:]
        assert bytes(machine.mem_read(0, len(module))) == bytes(module_before)
        assert bytes(machine.mem_read(DECOY_SEGMENT * 16, SEGMENT_SIZE)) == decoy_before
        assert snapshot_registers(machine) == initial
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4
        stack_after = bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
        assert_unchanged_outside(stack_before, stack_after, allowed_stack_offsets)
        assert (
            stack_after[CALLER_SP : CALLER_SP + 4 + len(STACK_SENTINEL)]
            == stack_before[CALLER_SP : CALLER_SP + 4 + len(STACK_SENTINEL)]
        )
        assert write_counts == {"global": 4, "stack": 12, "video": 0xFFFF}
        expected_flags = {"cf": False, "pf": True, "zf": True, "sf": False, "of": False}
        assert observed_flags(machine, LOGIC_FLAGS) == expected_flags

        rows.append(
            {
                "name": name,
                "saved_mode": saved_mode,
                "crtc_base": crtc,
                "font": {"offset": font_offset, "segment": font_segment},
                "interrupts": interrupts,
                "helper_call": helper_calls[0],
                "port_inputs": inputs,
                "mode_port_outputs": expected_mode_outputs,
                "palette_clear": {
                    "write_count": PALETTE_WRITE_COUNT,
                    "zero_data_writes": PALETTE_COMPONENT_COUNT,
                },
                "cleared_bytes": 0xFFFF,
                "defined_flags": expected_flags,
                "write_counts": write_counts,
            }
        )
    return rows


def observed_flags(machine: Uc, masks: dict[str, int]) -> dict[str, bool]:
    value = machine.reg_read(UC_X86_REG_EFLAGS)
    return {name: bool(value & mask) for name, mask in masks.items()}


def build_fixture(executable: bytes) -> dict[str, Any]:
    routines = []
    for entry, end, operation, expected_hash in ROUTINES:
        digest = hashlib.sha256(executable[entry:end]).hexdigest()
        assert digest == expected_hash, hex(entry)
        routines.append(
            {
                "operation": operation,
                "entry": f"0x{entry:04x}",
                "end": f"0x{end:04x}",
                "body_sha256": digest,
            }
        )
    return {
        "format": "big_bug_bang_mode_x_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "routines": routines,
        "cases": mode_x_cases(executable),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    executable = args.executable.read_bytes()
    if hashlib.sha256(executable).hexdigest() != EXECUTABLE_SHA256:
        raise SystemExit("unsupported BLOOD2PG.EXE build; refusing fixed-offset oracle")
    fixture = build_fixture(executable)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(fixture, indent=2) + "\n")
    print(f"verified {len(fixture['cases'])} BBB Mode X initialization cases")


if __name__ == "__main__":
    main()
