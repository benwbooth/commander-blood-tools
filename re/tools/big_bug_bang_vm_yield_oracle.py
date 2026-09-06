#!/usr/bin/env python3
"""Run original A6 handlers and their outer VM loop up to the post-scan boundary.

No instruction or handler-table entry is replaced. Resource binding and pre-frame
state preparation precede this entry; selection and actor scans follow the stop.
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
CODE_BASE = 0x5020
GLOBALS = 0x30000
SOURCE = 0x40000
STATE = 0x50000
DICTIONARY = 0x60000
STACK = 0xFF00
RANGES = [(0x5AA6, 0x5B3D), (0x6B28, 0x6E67), (0x68A5, 0x68B5), (0x6993, 0x69AC)]


def word(data, offset):
    return struct.unpack_from("<H", data, offset)[0]


def run(executable, mode, gate, locked, twice):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    var = bytearray()
    deb = bytearray()
    for name, kind, size in [(b"blood", 1, 34), (b"actor", 2, 74), (b"item", 0x400, 24)]:
        deb.extend(struct.pack("<16sHH", name, len(var), 1))
        record = bytearray(size)
        struct.pack_into("<H", record, 0, kind)
        if kind == 0x400:
            struct.pack_into("<H", record, 20, 0xFFFF)
        var.extend(record)
    deb.extend(bytes(20))
    line = 34
    struct.pack_into("<H", var, line + 2, 0x8000 if gate == "shown" else 0)
    action_offset = executable[0xF7F0 + 0x7128 + 19 * 16 + 1]
    struct.pack_into("<H", var, line + action_offset, 195 if gate == "wrong_record" else 196)
    dic = bytes(32) + b"TEXTE\0"
    control = 0x8030 if mode == "inventory" else 0x8000
    if gate == "inactive":
        control &= 0x7FFF
    payload = struct.pack("<3H", 32, 0xFFFF, 0xFFFE) if mode == "inventory" else struct.pack("<H", 32)
    instruction = struct.pack("<BHbH", 0xA6, line, -3, control)
    if mode == "inventory":
        instruction += struct.pack("<H", 16 * (2 if twice else 1))
    instruction += payload + bytes(2)
    assert len(instruction) == (16 if mode == "inventory" else 10)
    cod = instruction * (2 if twice else 1) + b"\xFF"
    before = bytearray(0x10000)
    native_data = executable[0xF7F0:]
    before[:len(native_data)] = native_data
    for offset, value in [(0x6AF4, SOURCE), (0x6AEC, STATE), (0x6AFC, DICTIONARY)]:
        struct.pack_into("<HH", before, offset, 0, value // 16)
    before[0x6B7E] = 1
    before[0x6B8D] = locked
    before[0x2200] = 1
    before[0x6B8A] = 9
    before[0x6B87] = 0
    before[0x6B81] = 0
    before[0x6B8F] = int(mode == "subtitle")
    before[0x6B86] = int(gate == "menu")
    before[0x6234] = int(gate == "subtitle")
    before[0x6B80] = 0x40
    before[0x6B92] = 1
    before[0x6BDC:0x6BDC + 34] = bytes(34)
    struct.pack_into("<17H", before, 0x70E6, 0 if gate == "empty" else 108, *([0] * 15), 0xFFFF)
    cpu.mem_write(GLOBALS, bytes(before))
    cpu.mem_write(SOURCE, cod)
    cpu.mem_write(STATE, bytes(var))
    cpu.mem_write(DICTIONARY, dic)
    for reg, value in [(UC_X86_REG_CS, CODE_BASE // 16), (UC_X86_REG_DS, GLOBALS // 16),
                       (UC_X86_REG_GS, GLOBALS // 16), (UC_X86_REG_SS, GLOBALS // 16),
                       (UC_X86_REG_SP, STACK)]:
        cpu.reg_write(reg, value)
    entries = []
    yields = []
    ended = False
    global_ranges = [(0x6B4E, 2), (0x6B87, 2), (0x6B36, 2), (0x6B4A, 4), (0x21F9, 2),
                     (0x6B8A, 1), (0x6B8D, 1), (0x6B7E, 1), (0x6B8F, 1), (0x6B94, 2),
                     (0x6B81, 1), (0x6BDC, 34), (0xF49, 1), (0xF48, 1), (0x6B86, 1),
                     (0x6234, 1), (0x6228, 2), (0x6B92, 1), (0x6B80, 1), (0x1066, 128),
                     (0xF47, 1), (0x2201, 1), (0x2A6F, 2), (0x2A73, 2), (0x6B1A, 4)]
    allowed = [(GLOBALS + offset, size) for offset, size in global_ranges]
    allowed += [(GLOBALS + STACK - 64, 64), (STATE + line + 2, 2)]
    allowed += [(SOURCE + i * len(instruction) + 5, 1) for i in range(2 if twice else 1)]

    def instruction_hook(_cpu, address, size, _context):
        nonlocal ended
        address += HEADER
        assert any(a <= address and address + size <= b for a, b in RANGES), hex(address)
        if address == 0x6C89:
            entries.append(cpu.reg_read(UC_X86_REG_SI) - 1)
        if address == 0x5AD7:
            yields.append(cpu.mem_read(GLOBALS + 0x6B8A, 1)[0])
        if address == 0x5AC4 and cpu.reg_read(UC_X86_REG_AL) == 0xFF:
            ended = True

    def write(_cpu, _access, address, size, _value, _context):
        assert any(a <= address and address + size <= a + n for a, n in allowed), hex(address)

    cpu.hook_add(UC_HOOK_CODE, instruction_hook)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start(0x5AA6 - HEADER, 0x5B3D - HEADER, count=10000)
    assert cpu.reg_read(UC_X86_REG_IP) == 0x5B3D - HEADER - CODE_BASE
    assert cpu.reg_read(UC_X86_REG_SP) == STACK
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert bytes(cpu.mem_read(DICTIONARY, len(dic))) == dic
    after = bytes(cpu.mem_read(GLOBALS, len(before)))
    for i, (a, b) in enumerate(zip(before, after)):
        assert a == b or any(offset <= i < offset + size for offset, size in global_ranges) or STACK - 64 <= i < STACK
    return dict(name=f"{mode}_{gate}_lock{locked}_twice{int(twice)}", mode=mode, gate=gate,
                locked_before=locked, cod=cod.hex(), var=var.hex(), deb=deb.hex(), dic=dic.hex(),
                cod_after=bytes(cpu.mem_read(SOURCE, len(cod))).hex(),
                var_after=bytes(cpu.mem_read(STATE, len(var))).hex(),
                vm=after[0x6B7E], start_locked=after[0x6B8D], c2_gate=after[0x2200],
                yield_signals=yields, entries=entries, end_marker=ended,
                cursor=cpu.reg_read(UC_X86_REG_SI) - int(ended), resume=after[0x6B87],
                saved_cursor=word(after, 0x6B4C), request=after[0x6B80])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    rows = [run(executable, mode, gate, locked, twice)
            for mode in ["menu", "subtitle", "inventory"]
            for gate in (["none", "inactive", "shown", "wrong_record", "menu", "subtitle", "empty"]
                         if mode == "inventory" else ["none", "inactive", "shown", "wrong_record", "menu", "subtitle"])
            for locked in [0, 1] for twice in [False, True]]
    assert {signal for row in rows for signal in row["yield_signals"]} == {0, 2, 3}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("".join(json.dumps(row, sort_keys=True) + "\n" for row in rows))
    print(f"captured {len(rows)} original A6 and outer-loop yield cases")


if __name__ == "__main__":
    main()
