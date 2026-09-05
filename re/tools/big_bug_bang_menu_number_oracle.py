#!/usr/bin/env python3
"""Run the sequel inline-menu renderer with original number/font helpers.

Captures text, layout, cursor and countdown effects, not VGA plane pixels.
No native helper is replaced. Synthetic dictionary/state/menu inputs exercise
number substitution, lookahead scratch reuse and encoded-word reveal timing.
"""

import argparse
import hashlib
import itertools
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import (
    UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_ES, UC_X86_REG_GS,
    UC_X86_REG_SS, UC_X86_REG_SP, UC_X86_REG_SI, UC_X86_REG_DI,
    UC_X86_REG_AX, UC_X86_REG_BX, UC_X86_REG_CX, UC_X86_REG_DX,
    UC_X86_REG_BP, UC_X86_REG_IP, UC_X86_REG_EFLAGS,
)

EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
MZ_HEADER_SIZE = 2048
DATA_FILE_START = 0xF7F0
GLOBAL_BASE = 0x30000
STATE_BASE = 0x40000
DICTIONARY_BASE = 0x50000
MENU_BASE = 0x60000
SEGMENT_SIZE = 65536
STACK_TOP = 0xFF00
MENU_START = 256
RETURN_IP = 512
ENTRY = 0x82C6
ALLOWED_CODE = [(0x82C6, 0x83E2), (0x2832, 0x286B), (0x38FC, 0x39BE), (0x344D, 0x3486)]
GLOBAL_OUTPUTS = {0x2A6D: 2, 0x2A71: 2, 0x2A73: 2, 0x0D3F: 2, 0x6B91: 1}


def word(data, offset):
    return struct.unpack_from("<H", data, offset)[0]


def run(executable, name, menu, numbers, reveal, countdown=0, delay=4, scratch=b""):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    # Native far-call segments now address the load module, not MZ file offsets.
    module = executable[MZ_HEADER_SIZE:]
    cpu.mem_write(0, module)
    globals_before = bytearray(SEGMENT_SIZE)
    native_data = executable[DATA_FILE_START:]
    globals_before[:len(native_data)] = native_data
    for offset, pointer in [(0x6AEC, STATE_BASE), (0x6AFC, DICTIONARY_BASE),
                            (0x55E9, 0xA0000), (0x55ED, 0xA0000)]:
        struct.pack_into("<HH", globals_before, offset, 0, pointer // 16)
    struct.pack_into("<HH", globals_before, 0x6B1A, MENU_START, MENU_BASE // 16)
    for offset, value in [(0x5609, 0), (0x560B, 199), (0x2A73, MENU_START + reveal * 2),
                          (0x2A6F, len(menu)), (0x0D3F, countdown), (0x0CC2, delay)]:
        struct.pack_into("<H", globals_before, offset, value)
    globals_before[0x6B86] = 1
    globals_before[0x6B92] = 0
    globals_before[0x6B91] = 0
    struct.pack_into("<HH", globals_before, STACK_TOP, RETURN_IP, 0)
    cpu.mem_write(GLOBAL_BASE, bytes(globals_before))
    state = bytearray(SEGMENT_SIZE)
    for offset, value in numbers.items():
        struct.pack_into("<H", state, offset, value & 65535)
    cpu.mem_write(STATE_BASE, bytes(state))
    dictionary = bytearray(SEGMENT_SIZE)
    for offset, text in [(1, scratch), (16, b"VALUE"), (32, b","), (48, b"NEXT"),
                          (64, b"ABCDEFGHIJKLMNOPQRSTUVWXYZ"), (96, b".")]:
        dictionary[offset:offset + len(text) + 1] = text + b"\0"
    cpu.mem_write(DICTIONARY_BASE, bytes(dictionary))
    source = bytearray(SEGMENT_SIZE)
    for index, value in enumerate([*menu, 0]):
        struct.pack_into("<H", source, MENU_START + index * 2, value)
    cpu.mem_write(MENU_BASE, bytes(source))
    registers = {UC_X86_REG_CS: 0x502, UC_X86_REG_DS: GLOBAL_BASE // 16,
                 UC_X86_REG_ES: 0x7000, UC_X86_REG_GS: GLOBAL_BASE // 16,
                 UC_X86_REG_SS: GLOBAL_BASE // 16, UC_X86_REG_SP: STACK_TOP,
                 UC_X86_REG_SI: 123, UC_X86_REG_DI: 234, UC_X86_REG_AX: 345,
                 UC_X86_REG_BX: 456, UC_X86_REG_CX: 567, UC_X86_REG_DX: 678,
                 UC_X86_REG_BP: 789}
    for register, value in registers.items():
        cpu.reg_write(register, value)
    cpu.reg_write(UC_X86_REG_EFLAGS, 2)
    draws = []
    visited_helpers = set()
    layout_y = 8

    def string_at(segment, offset):
        data = bytes(cpu.mem_read(segment * 16 + offset, 256))
        return list(data[:data.index(0)])

    def instruction(_cpu, address, _size, _context):
        nonlocal layout_y
        file_address = address + MZ_HEADER_SIZE
        if not any(start <= file_address < end for start, end in ALLOWED_CODE):
            raise AssertionError(f"{name}: unexpected instruction {file_address:#x}")
        if file_address in (0x2832, 0x38FC, 0x344D):
            visited_helpers.add(file_address)
        if file_address == 0x38FC:
            draws.append({"text": string_at(cpu.reg_read(UC_X86_REG_DS), cpu.reg_read(UC_X86_REG_SI)),
                          "position": [cpu.reg_read(UC_X86_REG_BX), cpu.reg_read(UC_X86_REG_DX)]})
        elif file_address == 0x8340:
            draws[-1]["width"] = word(cpu.mem_read(GLOBAL_BASE, SEGMENT_SIZE), 0x2A6D)
        elif file_address == 0x838E:
            layout_y = cpu.reg_read(UC_X86_REG_DX)

    def write(_cpu, _access, address, size, _value, _context):
        allowed = [(GLOBAL_BASE, GLOBAL_BASE + SEGMENT_SIZE),
                   (DICTIONARY_BASE + 1, DICTIONARY_BASE + 8), (0xA0000, 0xB0000),
                   (0x2826 - MZ_HEADER_SIZE, 0x2832 - MZ_HEADER_SIZE)]
        if not any(start <= address and address + size <= end for start, end in allowed):
            raise AssertionError(f"{name}: unexpected write {address:#x}/{size}")

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start(ENTRY - MZ_HEADER_SIZE, RETURN_IP, count=300000)
    assert cpu.reg_read(UC_X86_REG_CS) == 0 and cpu.reg_read(UC_X86_REG_IP) == RETURN_IP, name
    registers[UC_X86_REG_CS] = 0
    registers[UC_X86_REG_SP] += 4
    for register, value in registers.items():
        assert cpu.reg_read(register) == value, (name, register)
    after = bytearray(cpu.mem_read(GLOBAL_BASE, SEGMENT_SIZE))
    output = {"draws": draws, "x": word(after, 0x2A71), "y": layout_y, "reveal": (word(after, 0x2A73) - MENU_START) // 2,
              "countdown": word(after, 0x0D3F), "complete": bool(after[0x6B91]),
              "scratch": string_at(DICTIONARY_BASE // 16, 1), "helpers": sorted(visited_helpers)}
    for offset, length in GLOBAL_OUTPUTS.items():
        after[offset:offset + length] = globals_before[offset:offset + length]
    after[STACK_TOP - 128:STACK_TOP] = globals_before[STACK_TOP - 128:STACK_TOP]
    assert after == globals_before, name
    assert cpu.mem_read(STATE_BASE, SEGMENT_SIZE) == state, name
    assert cpu.mem_read(MENU_BASE, SEGMENT_SIZE) == source, name
    return {"name": name, "menu": menu, "numbers": numbers, "reveal": reveal,
            "countdown": countdown, "delay": delay, "scratch": list(scratch), "output": output}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    if hashlib.sha256(executable).hexdigest() != EXECUTABLE_SHA256:
        raise SystemExit("unsupported sequel executable")
    results = []
    for value, reveal in itertools.product([0, 1, -1, 32767, -32768], range(7)):
        results.append(run(executable, f"number_{value}_{reveal}", [16, 1, 128, 32, 48], {128: value}, reveal))
    for reveal, countdown, scratch in itertools.product(range(6), [0, 3], [b"", b"-32768"]):
        results.append(run(executable, f"adjacent_{reveal}_{countdown}_{scratch.decode()}",
                           [64, 1, 128, 1, 130, 96], {128: 7, 130: -8}, reveal,
                           countdown=countdown, scratch=scratch))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("".join(json.dumps(row, separators=(",", ":")) + "\n" for row in results))
    print(f"wrote {len(results)} original inline-menu numeric cases")


if __name__ == "__main__":
    main()
