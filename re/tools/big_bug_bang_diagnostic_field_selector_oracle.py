#!/usr/bin/env python3
"""Probe BBB's unchanged horizontal-arrow diagnostic selector handlers."""

from __future__ import annotations

import argparse
import hashlib
import itertools
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_16
from unicorn.x86_const import (
    UC_X86_REG_AX,
    UC_X86_REG_BX,
    UC_X86_REG_CS,
    UC_X86_REG_CX,
    UC_X86_REG_DI,
    UC_X86_REG_DS,
    UC_X86_REG_DX,
    UC_X86_REG_ES,
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)


EXECUTABLE_SHA256 = (
    "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
)
MODE_OFFSET = 0x6B7C
SELECTOR_OFFSET = 0x6B7D
GLOBALS = 0x30000
STACK = 0x90000
STACK_POINTER = 0xFF00
RETURN_ADDRESS = 0xF000
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

HANDLERS = {
    "next": (0x2495, 0x24A3, 1, "866763e2d92d916bf36f3aa3b53f23f2b30ccfb6f9d24dca3ee84356e92ec915"),
    "previous": (0x24A3, 0x24B1, -1, "4f8a2730f721f1657941638fa450d2825461671358b2a2a47994590127a6d4f5"),
}


def run_case(executable: bytes, direction: str, mode: int, selector: int) -> dict[str, object]:
    entry, end, delta, _digest = HANDLERS[direction]
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    cpu.mem_write(0, executable)

    before = bytearray((index * 37 + 11) & 0xFF for index in range(0x10000))
    before[MODE_OFFSET] = mode
    before[SELECTOR_OFFSET] = selector
    cpu.mem_write(GLOBALS, bytes(before))
    cpu.mem_write(
        STACK + STACK_POINTER,
        struct.pack("<H", RETURN_ADDRESS) + STACK_SENTINEL,
    )

    registers = {
        UC_X86_REG_AX: 0x1234,
        UC_X86_REG_BX: 0x2345,
        UC_X86_REG_CX: 0x3456,
        UC_X86_REG_DX: 0x4567,
        UC_X86_REG_SI: 0x5678,
        UC_X86_REG_DI: 0x6789,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: GLOBALS // 16,
        UC_X86_REG_ES: 0x4000,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
    }
    for register, value in registers.items():
        cpu.reg_write(register, value)

    reached_return = []

    def instruction(machine, address, size, _context):
        if address == RETURN_ADDRESS:
            reached_return.append(address)
            machine.emu_stop()
            return
        assert entry <= address < address + size <= end

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.emu_start(entry, 0, count=32)
    assert reached_return == [RETURN_ADDRESS]

    for register, value in registers.items():
        if register == UC_X86_REG_SP:
            assert cpu.reg_read(register) == STACK_POINTER + 2
        else:
            assert cpu.reg_read(register) == value
    assert bytes(cpu.mem_read(0, len(executable))) == executable
    assert bytes(cpu.mem_read(STACK + STACK_POINTER + 2, len(STACK_SENTINEL))) == STACK_SENTINEL

    after = bytes(cpu.mem_read(GLOBALS, len(before)))
    expected = before[:]
    changed = mode & 3 != 0
    if changed:
        expected[SELECTOR_OFFSET] = (selector + delta) & 0xFF
    assert after == expected
    return {
        "direction": direction,
        "mode": mode,
        "selector_before": selector,
        "selector_after": after[SELECTOR_OFFSET],
        "changed": changed,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    executable = args.executable.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise ValueError(f"unrecognized BLOOD2PG.EXE SHA-256 {digest}")
    for entry, end, _delta, expected in HANDLERS.values():
        actual = hashlib.sha256(executable[entry:end]).hexdigest()
        if actual != expected:
            raise ValueError(f"handler {entry:#x} bytes changed")

    cases = [
        run_case(executable, direction, mode, selector)
        for direction, mode, selector in itertools.product(
            HANDLERS, (0, 1, 2, 3, 4, 0x80, 0xFF), (0, 1, 0x15, 0xFF)
        )
    ]
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(json.dumps(case, sort_keys=True) + "\n" for case in cases)
    )
    print(f"verified {len(cases)} original diagnostic selector cases")


if __name__ == "__main__":
    main()
