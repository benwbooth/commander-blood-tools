#!/usr/bin/env python3
"""Run the original sequel pre-frame pass and its unmodified position helpers."""

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import *

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER = 0x800
GLOBALS = 0x30000
VAR = 0x40000
DIRECTORY = 0x60000
STACK = 0xFF00
RETURN = 0x200
RANGES = [(0x6038, 0x6104), (0x67B8, 0x6822), (0x6633, 0x6644)]


def put(data, offset, value):
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def fixture():
    objects = [(512, 38, "orxx"), (128, 26, "home"), (128, 26, "empty"),
               (2, 74, "Honk"), (2, 74, "second"), (2, 74, "nested"),
               (8, 30, "body"), (16, 36, "arche")]
    state = bytearray()
    directory = bytearray()
    offsets = {}
    for kind, size, name in objects:
        offsets[name] = len(state)
        directory.extend(struct.pack("<16sHH", name.encode(), len(state), 1))
        record = bytearray(size)
        put(record, 0, kind)
        put(record, 2, 0xA015)
        state.extend(record)
    directory.extend(bytes(20))
    for name in ["home", "empty"]:
        put(state, offsets[name] + 20, offsets["body"])
        put(state, offsets[name] + 24, 0x1234)
    for name in ["Honk", "second", "nested"]:
        put(state, offsets[name] + 24, offsets["home"])
    put(state, offsets["nested"] + 24, offsets["second"])
    for name in ["orxx", "body", "arche"]:
        put(state, offsets[name] + 22, 0xFFFF)
        put(state, offsets[name] + 24, 100)
        put(state, offsets[name] + 26, 200)
    return state, bytes(directory), offsets


def run(executable, name, state, directory, offsets, request=0, text=0, post=0, paused=False):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    globals_data = bytearray(0x10000)
    struct.pack_into("<HH", globals_data, 0x6AEC, 0, VAR // 16)
    struct.pack_into("<HH", globals_data, 0x6AF0, 0, DIRECTORY // 16)
    for field, value in [(0x6B20, offsets["orxx"]), (0x6B22, offsets["arche"]),
                         (0x6B24, offsets["Honk"]), (0x6B6A, post)]:
        put(globals_data, field, value)
    globals_data[0x6B80] = request
    globals_data[0x6234] = text
    globals_data[0x7128:0x7288] = executable[0x16918:0x16A78]
    # The paused entry is immediately after resource binding. Supply exactly
    # the seven saved registers and far return consumed by its real epilogue.
    stack_words = [0, 0, 0, 0, 0, 0, 0, RETURN, 0] if paused else [RETURN]
    struct.pack_into(f"<{len(stack_words)}H", globals_data, STACK, *stack_words)
    cpu.mem_write(GLOBALS, bytes(globals_data))
    cpu.mem_write(VAR, bytes(state))
    cpu.mem_write(DIRECTORY, directory)
    for reg, value in [(UC_X86_REG_CS, 0), (UC_X86_REG_DS, VAR // 16),
                       (UC_X86_REG_GS, GLOBALS // 16), (UC_X86_REG_SS, GLOBALS // 16),
                       (UC_X86_REG_SP, STACK)]:
        cpu.reg_write(reg, value)
    calls = []
    allowed = RANGES + ([(0x5A99, 0x5AA6), (0x5B56, 0x5B65)] if paused else [])

    def instruction(_cpu, address, size, _context):
        address += HEADER
        assert any(start <= address and address + size <= end for start, end in allowed), hex(address)
        if address in [0x6038, 0x67B8, 0x6633]:
            calls.append(address)

    def write(_cpu, _access, address, size, _value, _context):
        assert (VAR <= address < address + size <= VAR + len(state)
                or GLOBALS + STACK - 128 <= address < address + size <= GLOBALS + STACK), hex(address)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start((0x5A99 if paused else 0x6038) - HEADER, RETURN, count=10000)
    assert cpu.reg_read(UC_X86_REG_IP) == RETURN, name
    assert cpu.reg_read(UC_X86_REG_SP) == STACK + 2 * len(stack_words), name
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert bytes(cpu.mem_read(DIRECTORY, len(directory))) == directory
    assert bytes(cpu.mem_read(GLOBALS, STACK - 128)) == globals_data[:STACK - 128]
    if paused:
        assert cpu.reg_read(UC_X86_REG_AX) == 0
    return dict(name=name, directory=list(directory), state_before=list(state),
                state_after=list(cpu.mem_read(VAR, len(state))), request=request,
                text=text, post=post, offsets=offsets, paused=paused, calls=calls)


def vectors(executable):
    for paused in [False, True]:
        for request, text, honk_post in [(0, 0, False), (1, 0, False), (2, 0, False),
                                         (4, 0, False), (0, 1, False), (0, 1, True), (0, 2, False)]:
            state, directory, offsets = fixture()
            yield run(executable, f"pause{int(paused)}_r{request}_t{text}_h{int(honk_post)}",
                      state, directory, offsets, request, text,
                      offsets["Honk"] if honk_post else 0, paused)
    for name, edits in [
        ("none_occupy", [("Honk", 2, 1), ("second", 2, 1), ("nested", 2, 1)]),
        ("first_only", [("second", 2, 1)]),
        ("split_locations", [("second", 24, "empty")]),
        ("sentinel_parent", [("second", 24, 0xFFFF)]),
        ("zero_parent", [("second", 24, 0)]),
        ("fallback_match", [("orxx", 24, 300)]),
        ("no_position_match", [("orxx", 24, 300), ("arche", 24, 400)]),
    ]:
        state, directory, offsets = fixture()
        for obj, field, value in edits:
            put(state, offsets[obj] + field, offsets[value] if isinstance(value, str) else value)
        yield run(executable, name, state, directory, offsets)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    rows = list(vectors(executable))
    args.output.write_text("".join(json.dumps(row, separators=(",", ":")) + "\n" for row in rows))
    print(f"wrote {len(rows)} native state-processor vectors")


if __name__ == "__main__":
    main()
