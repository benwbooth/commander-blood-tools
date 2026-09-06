#!/usr/bin/env python3
"""Capture native palette copy, effect selection/frames and artwork traversal.

Runs only the bounded original instructions, not an emulated game startup.
The executable is never patched; all memory writes are explicitly guarded.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import *

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER = 0x800
DATA_FILE = 0xF7F0
GLOBALS = 0x30000


def run(executable, start, end, writable=(), registers=None, visit=None, initial_data=()):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    data = bytearray(0x10000)
    source = executable[DATA_FILE:]
    assert len(source) <= len(data)
    data[:len(source)] = source
    for offset, value in initial_data:
        assert 0 <= offset <= offset + len(value) <= len(data)
        data[offset:offset + len(value)] = value
    cpu.mem_write(0, module)
    cpu.mem_write(GLOBALS, bytes(data))
    initial = {UC_X86_REG_CS: 0, UC_X86_REG_DS: GLOBALS // 16,
               UC_X86_REG_ES: GLOBALS // 16, UC_X86_REG_GS: GLOBALS // 16,
               UC_X86_REG_SS: GLOBALS // 16, UC_X86_REG_SP: 0xFF00,
               UC_X86_REG_EFLAGS: 2}
    initial.update(registers or {})
    for register, value in initial.items():
        cpu.reg_write(register, value)

    def instruction(_cpu, address, size, _context):
        file = address + HEADER
        assert start <= file < file + size <= end, hex(file)
        if visit:
            visit(cpu, file)

    def write(_cpu, _access, address, size, _value, _context):
        assert any(GLOBALS + lo <= address < address + size <= GLOBALS + hi
                   for lo, hi in writable), (hex(address), size)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start(start - HEADER, end - HEADER, count=4096)
    assert cpu.reg_read(UC_X86_REG_IP) == end - HEADER
    assert cpu.reg_read(UC_X86_REG_SP) == 0xFF00
    assert bytes(cpu.mem_read(0, len(module))) == module
    after = bytes(cpu.mem_read(GLOBALS, len(data)))
    for offset, (before, result) in enumerate(zip(data, after)):
        assert before == result or any(lo <= offset < hi for lo, hi in writable)
    return cpu, after


def capture(executable):
    cpu, data = run(executable, 0xADA4, 0xADB0, [(0x5621, 0x5921)])
    assert cpu.reg_read(UC_X86_REG_CX) == 0
    assert cpu.reg_read(UC_X86_REG_SI) == 0x6228
    assert cpu.reg_read(UC_X86_REG_DI) == 0x5921
    palette = [list(data[p:p + 3]) for p in range(0x5621, 0x5921, 3)]
    assert max(max(color) for color in palette) <= 63
    effects = []
    for index in range(10):
        cpu, data = run(executable, 0x9DF8, 0x9E05, [(0x2A8F, 0x2A91)],
                        {UC_X86_REG_AX: index})
        pointer = cpu.reg_read(UC_X86_REG_SI)
        operation, count = data[0x2A8F:0x2A91]
        assert count > 0
        frames = []
        for frame in range(count):
            fields = []

            def visit(cpu, file):
                if file in (0x9E0E, 0x9E11, 0x9E1D, 0x9E25):
                    fields.append(cpu.reg_read(UC_X86_REG_AX))

            cpu, _ = run(executable, 0x9E0D, 0x9E2B, [(0x2A8D, 0x2A8F)],
                         {UC_X86_REG_SI: pointer + frame * 8, UC_X86_REG_DI: 0}, visit)
            assert len(fields) == 4
            assert cpu.reg_read(UC_X86_REG_DI) == (fields[0] + 320 * fields[1]) & 0xFFFF
            assert cpu.reg_read(UC_X86_REG_BP) == fields[3]
            frames.append(dict(origin=fields[:2], size=fields[2:]))
        effects.append(dict(index=index, pointer=pointer - 2, operation=operation, frames=frames))

    artwork = []

    def visit_artwork(cpu, file):
        if file == 0x8025:
            pointer = cpu.reg_read(UC_X86_REG_BP)
            record = bytes(cpu.mem_read(GLOBALS + pointer, 22))
            resource, entity = struct.unpack_from("<HH", record, 16)
            artwork.append(dict(pointer=pointer, name=list(record[:16].split(b"\0", 1)[0]),
                                resource_id=resource, entity_id=entity, active=bool(record[20])))

    cpu, _ = run(executable, 0x801B, 0x802E,
                 [(0x2F97 + i * 22 + 20, 0x2F97 + i * 22 + 21) for i in range(42)],
                 visit=visit_artwork)
    assert len(artwork) == 42
    assert cpu.reg_read(UC_X86_REG_BP) == 0x3333
    return dict(executable_sha256=SHA256, palette=palette, effects=effects, artwork=artwork)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    result = capture(executable)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print("captured 256 palette entries, 10 effect sequences and 42 artwork rows")


if __name__ == "__main__":
    main()
