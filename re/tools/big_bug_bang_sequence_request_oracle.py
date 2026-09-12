#!/usr/bin/env python3
"""Execute Big Bug Bang's inherited A8 sequence-request handler.

The original instructions run unmodified under Unicorn. The fixture contains
semantic inputs and outputs only, not executable bytes. Run with ``python3 -P``
so the adjacent dis.py cannot shadow Python's standard library.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import (
    UC_X86_REG_CS,
    UC_X86_REG_DS,
    UC_X86_REG_EFLAGS,
    UC_X86_REG_GS,
    UC_X86_REG_IP,
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER = 0x800
CODE_BASE = 0x5020
DISPATCH_TABLE = 0x16A78
HANDLER = (0x6E7C, 0x6EE4)
GLOBALS = 0x30000
SOURCE = 0x40000
STACK = 0xFF00
RETURN = 0x1800

BUFFER = 0x2372
SHIP_FLAGS = 0x2745
SCENE_GATE = 0x29DD
FINALE = 0x6B93
REQUEST_FLAGS = 0x6B80
ACTIVE_LINE = 0x6B5A
PRESENTATION_GATE = 0x2200
LOADED_IMAGE = 0x21F1
MOUSE_IDLE_LOW = 0x0D45

BASENAMES = (
    ("empty", b"", 0x40),
    ("ordinary", b"ship", 0x140),
    ("exact_finale", b"fin.", 0x240),
    ("finale_prefix", b"fin.ale", 0x340),
    ("case_sensitive", b"Fin.", 0x440),
    ("overlap_not_prefix", b"ffin.", 0x540),
    ("high_bytes", bytes((0x80, 0xFE, 0xFF)), 0x640),
    ("segment_wrap", b"fin.", 0xFFFC),
)

CONTEXTS = (
    ("idle", 0x00, 0x0000, 0x00),
    ("pending", 0x02, 0x0001, 0x01),
    ("pending_preserves_bits", 0xFF, 0xFFFF, 0xFF),
    ("ship", 0xA0, 0x0001, 0x00),
    ("ship_low_bit", 0x81, 0x8001, 0x80),
    ("scene", 0x10, 0x8000, 0x01),
    ("scene_low_bit", 0x80, 0x0000, 0x81),
    ("unrelated_bits", 0x10, 0x8000, 0x80),
)


def word(data, offset):
    return struct.unpack_from("<H", data, offset)[0]


def in_range(address, size, span):
    start, end = span
    start -= HEADER
    end -= HEADER
    return start <= address and address + size <= end


def write_segment_bytes(cpu, base, offset, data):
    for index, value in enumerate(data):
        cpu.mem_write(base + ((offset + index) & 0xFFFF), bytes((value,)))


def execute(executable, basename, pad, script_start, before):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    cpu.mem_write(GLOBALS, bytes(before))
    cpu.mem_write(GLOBALS + BUFFER, b"\xCC" * (len(basename) + 2))
    operand = basename + b"\0" + bytes((pad,))
    write_segment_bytes(cpu, SOURCE, script_start, operand)
    struct.pack_into("<H", before, STACK, RETURN)
    cpu.mem_write(GLOBALS + STACK, bytes(before[STACK:STACK + 2]))

    for register, value in (
        (UC_X86_REG_CS, CODE_BASE // 16),
        (UC_X86_REG_DS, SOURCE // 16),
        (UC_X86_REG_GS, GLOBALS // 16),
        (UC_X86_REG_SS, GLOBALS // 16),
        (UC_X86_REG_SP, STACK),
        (UC_X86_REG_SI, script_start),
        (UC_X86_REG_EFLAGS, 2),
    ):
        cpu.reg_write(register, value)

    allowed = [
        (GLOBALS + BUFFER, len(basename) + 1),
        (GLOBALS + FINALE, 1),
        (GLOBALS + REQUEST_FLAGS, 1),
        (GLOBALS + ACTIVE_LINE, 2),
        (GLOBALS + PRESENTATION_GATE, 1),
        (GLOBALS + LOADED_IMAGE, 2),
        (GLOBALS + MOUSE_IDLE_LOW, 1),
    ]

    def instruction(_cpu, address, size, _context):
        assert in_range(address, size, HANDLER), hex(address + HEADER)

    def write_hook(_cpu, _access, address, size, _value, _context):
        assert any(
            first <= address and address + size <= first + length
            for first, length in allowed
        ), hex(address)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.emu_start(HANDLER[0] - HEADER, CODE_BASE + RETURN, count=1000)
    assert cpu.reg_read(UC_X86_REG_IP) == RETURN
    assert cpu.reg_read(UC_X86_REG_SP) == STACK + 2
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert bytes(cpu.mem_read(GLOBALS + BUFFER, len(basename) + 1)) == basename + b"\0"
    assert cpu.mem_read(GLOBALS + BUFFER + len(basename) + 1, 1) == b"\xCC"
    for index, value in enumerate(operand):
        actual = cpu.mem_read(SOURCE + ((script_start + index) & 0xFFFF), 1)[0]
        assert actual == value
    return cpu, bytes(cpu.mem_read(GLOBALS, len(before)))


def vectors(executable):
    encoded = struct.unpack_from("<H", executable, DISPATCH_TABLE + (0xA8 - 0xA0) * 2)[0]
    assert HEADER + CODE_BASE + encoded == HANDLER[0]
    rows = []
    for basename_index, (basename_name, basename, script_start) in enumerate(BASENAMES):
        for context_index, (context_name, request, ship, scene) in enumerate(CONTEXTS):
            before = bytearray(0x10000)
            finale_before = (basename_index + context_index) & 1
            before[FINALE] = finale_before
            before[REQUEST_FLAGS] = request
            struct.pack_into("<H", before, SHIP_FLAGS, ship)
            before[SCENE_GATE] = scene
            struct.pack_into("<H", before, ACTIVE_LINE, 0)
            before[PRESENTATION_GATE] = 1
            struct.pack_into("<H", before, LOADED_IMAGE, 0x1357)
            before[MOUSE_IDLE_LOW] = 1
            pad = (0xA5 + basename_index * len(CONTEXTS) + context_index) & 0xFF
            cpu, after = execute(executable, basename, pad, script_start, before)

            is_finale = basename.startswith(b"fin.")
            raised = request & 2 == 0 and (ship & 1 != 0 or scene & 1 != 0)
            finale_after = 1 if is_finale else finale_before
            request_after = request | (2 if raised else 0)
            active_line_after = 7 if raised else 0
            presentation_gate_after = 0 if raised else 1
            loaded_image_after = 0xFFFF if raised else 0x1357
            mouse_idle_after = 0 if raised else 1
            cursor = (script_start + len(basename) + 2) & 0xFFFF

            assert after[FINALE] == finale_after
            assert after[REQUEST_FLAGS] == request_after
            assert word(after, ACTIVE_LINE) == active_line_after
            assert after[PRESENTATION_GATE] == presentation_gate_after
            assert word(after, LOADED_IMAGE) == loaded_image_after
            assert after[MOUSE_IDLE_LOW] == mouse_idle_after
            assert cpu.reg_read(UC_X86_REG_SI) == cursor
            rows.append(
                {
                    "name": f"{basename_name}_{context_name}",
                    "basename_hex": basename.hex(),
                    "pad": pad,
                    "script_start": script_start,
                    "cursor": cursor,
                    "finale_before": finale_before,
                    "finale_after": finale_after,
                    "request_before": request,
                    "request_after": request_after,
                    "ship_flags": ship,
                    "scene_gate": scene,
                    "raised": raised,
                    "active_line_after": active_line_after,
                    "presentation_gate_after": presentation_gate_after,
                    "loaded_image_after": loaded_image_after,
                    "mouse_idle_after": mouse_idle_after,
                }
            )
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    if hashlib.sha256(executable).hexdigest() != SHA256:
        raise SystemExit("unsupported BLOOD2PG.EXE build; refusing fixed-offset oracle")
    rows = vectors(executable)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(f"captured {len(rows)} original A8 sequence-request cases")


if __name__ == "__main__":
    main()
