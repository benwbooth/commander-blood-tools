#!/usr/bin/env python3
"""Run the original sequel's pending-profile gate, stopping before resource loading.

The VM returns AX=0 on its nonfatal path. This oracle starts at that main-loop
boundary with AL=0 and records which native branch is taken without replacing
instructions or invoking resource services. No original bytes are exported.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_AX, UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_ES, UC_X86_REG_IP

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER = 0x800
GLOBALS = 0x30000
ENTRY = 0x116D
LOAD = 0x118A
DEFER = 0x11C6
FLAGS = (0x27B7, 0x29C4, 0x29C5, 0x2A7A, 0x2A2D)
RESET_FIELDS = {
    "active": (0x6B82, 1), "menu": (0x6B86, 1), "subtitle": (0x6234, 1),
    "choice": (0x2A77, 1), "locked": (0x6B8D, 1), "request": (0x6B80, 1),
    "vm": (0x6B7E, 1), "ready": (0x6B92, 1), "complete": (0x6B91, 1),
    "countdown": (0xD3F, 2), "words": (0x6BDC, 2), "ui": (0x2A33, 2),
}


def run(executable, requested, mask, active_value, noise):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    globals_before = bytearray([noise] * 0x10000)
    struct.pack_into("<h", globals_before, 0x6B52, requested)
    flags = [active_value if mask & (1 << index) else 0 for index in range(5)]
    for address, value in zip(FLAGS, flags):
        globals_before[address] = value
    cpu.mem_write(GLOBALS, bytes(globals_before))
    cpu.reg_write(UC_X86_REG_CS, 0)
    cpu.reg_write(UC_X86_REG_DS, GLOBALS // 16)
    cpu.reg_write(UC_X86_REG_AX, 0)
    terminal = None

    def instruction(machine, address, size, _context):
        nonlocal terminal
        position = address + HEADER
        if position in (LOAD, DEFER):
            terminal = position
            machine.emu_stop()
        else:
            assert ENTRY <= position and position + size <= LOAD, hex(position)

    def write(_machine, _access, address, _size, _value, _context):
        raise AssertionError(f"unexpected write at {address:#x}")

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start(ENTRY - HEADER, DEFER + 1 - HEADER, count=32)
    assert terminal in (LOAD, DEFER), "gate did not reach either native boundary"
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert bytes(cpu.mem_read(GLOBALS, len(globals_before))) == globals_before
    return dict(requested=requested, flags=flags, noise=noise, load=terminal == LOAD)


def run_reset(executable, noise):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    before = bytes([noise] * 0x10000)
    cpu.mem_write(GLOBALS, before)
    for register, value in [(UC_X86_REG_CS, 0), (UC_X86_REG_DS, GLOBALS // 16),
                            (UC_X86_REG_ES, GLOBALS // 16)]:
        cpu.reg_write(register, value)
    written = set()

    def instruction(_machine, address, size, _context):
        assert 0x588F - HEADER <= address and address + size <= 0x5906 - HEADER

    def write(_machine, _access, address, size, _value, _context):
        assert GLOBALS <= address and address + size <= GLOBALS + len(before)
        written.update(range(address - GLOBALS, address + size - GLOBALS))

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start(0x588F - HEADER, 0x5906 - HEADER, count=200)
    assert cpu.reg_read(UC_X86_REG_IP) == 0x5906 - HEADER
    assert bytes(cpu.mem_read(0, len(module))) == module
    after = bytes(cpu.mem_read(GLOBALS, len(before)))
    assert all(a == b or index in written for index, (a, b) in enumerate(zip(before, after)))

    def fields(data):
        return {name: int.from_bytes(data[offset:offset + size], "little")
                for name, (offset, size) in RESET_FIELDS.items()}

    return dict(input=fields(before), output=fields(after), written=sorted(written))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--reset-output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    if hashlib.sha256(executable).hexdigest() != SHA256:
        raise ValueError("unrecognized BLOOD2PG executable")
    cases = [run(executable, request, mask, value, noise)
             for request in (-1, 0, 1, 16)
             for mask in range(32)
             for value in (1, 128)
             for noise in (0, 255)]
    with args.output.open("x") as output:
        for case in cases:
            output.write(json.dumps(case, sort_keys=True, separators=(",", ":")) + "\n")
    print(f"verified {len(cases)} original profile gates")
    if args.reset_output:
        with args.reset_output.open("x") as output:
            for noise in (0, 1, 2, 127, 255):
                output.write(json.dumps(run_reset(executable, noise), sort_keys=True, separators=(",", ":")) + "\n")
        print("verified 5 original profile-reset cases")


if __name__ == "__main__":
    main()
