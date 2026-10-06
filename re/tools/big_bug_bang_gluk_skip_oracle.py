#!/usr/bin/env python3
"""Probe Gluk's authored skip tail with unmodified DOS code and disc scripts.

Starts at the final rejected A6 in the post-callback block, with its earlier
greeting still displayed. This is a bounded VM witness, not a full DOS UI run.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import (
    UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_GS, UC_X86_REG_SS,
    UC_X86_REG_SP, UC_X86_REG_SI,
)

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER, CODE_BASE = 0x800, 0x5020
GLOBALS, SOURCE, STATE, DICTIONARY, STACK = 0x30000, 0x40000, 0x50000, 0x60000, 0xFF00
START, STOP = 15709, 15761
LABELS = {"A27": 7968, "A28": 7970, "A29": 7972, "A34": 7982}
RANGES = [(0x5AA6, 0x5B3D), (0x6B28, 0x6E67), (0x68A5, 0x68B5),
          (0x68C8, 0x694B), (0x697A, 0x69AC), (0x744B, 0x750A), (0x6A75, 0x6AA4)]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def words(data):
    return {name: struct.unpack_from("<H", data, offset)[0] for name, offset in LABELS.items()}


def run(executable, cod, dic, var):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    before = bytearray(0x10000)
    data = executable[0xF7F0:]
    before[:len(data)] = data
    for offset, base, cursor in [(0x6AF4, SOURCE, START), (0x6AEC, STATE, 0),
                                  (0x6AFC, DICTIONARY, 0)]:
        struct.pack_into("<HH", before, offset, cursor, base // 16)
    for offset, value in [(0x6B7E, 1), (0x6B8D, 1), (0x6B86, 1),
                          (0x6B87, 0), (0x6B81, 0), (0x6B83, 0)]:
        before[offset] = value
    struct.pack_into("<H", before, 0x6C2C, 2)
    struct.pack_into("<H", before, 0x6C04, STOP)
    cpu.mem_write(GLOBALS, bytes(before))
    cpu.mem_write(SOURCE, cod)
    cpu.mem_write(STATE, var)
    cpu.mem_write(DICTIONARY, dic)
    for register, value in [(UC_X86_REG_CS, CODE_BASE // 16),
                            (UC_X86_REG_DS, GLOBALS // 16), (UC_X86_REG_GS, GLOBALS // 16),
                            (UC_X86_REG_SS, GLOBALS // 16), (UC_X86_REG_SP, STACK)]:
        cpu.reg_write(register, value)
    entries, entry_queries, writes = [], [], []
    global_ranges = [(0x6B88, 1), (0x6B8A, 1), (0x6B81, 1), (0x6B83, 1),
                     (0x6C2C, 2), (0x6B4E, 2)]
    allowed = [(GLOBALS + offset, size) for offset, size in global_ranges]
    allowed += [(GLOBALS + STACK - 64, 64)]
    allowed += [(STATE + LABELS[name], 2) for name in ("A27", "A34")]
    stopped = False

    def instruction(_cpu, address, size, _context):
        nonlocal stopped
        address += HEADER
        assert any(a <= address and address + size <= b for a, b in RANGES), hex(address)
        if address == 0x5AC1:
            cursor = cpu.reg_read(UC_X86_REG_SI)
            if cursor == STOP:
                stopped = True
                cpu.emu_stop()
            else:
                entries.append(cursor)
                entry_queries.append(cpu.mem_read(GLOBALS + 0x6B83, 1)[0])

    def write(_cpu, _access, address, size, value, _context):
        assert any(a <= address and address + size <= a + n for a, n in allowed), hex(address)
        if STATE <= address < STATE + len(var):
            writes.append(dict(offset=address - STATE, size=size, value=value))

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start(0x5AA6 - HEADER, 0x5B3D - HEADER, count=10000)
    assert stopped and cpu.reg_read(UC_X86_REG_SP) == STACK
    for address, source in [(0, module), (SOURCE, cod), (DICTIONARY, dic)]:
        assert bytes(cpu.mem_read(address, len(source))) == source
    after = bytes(cpu.mem_read(STATE, len(var)))
    assert all(a == b or any(offset <= i < offset + 2 for offset in (7968, 7982))
               for i, (a, b) in enumerate(zip(var, after)))
    assert entries == [15709, 15746] and entry_queries == [0, 1], (entries, entry_queries)
    assert words(var)["A27"] == 4 and after == var and not writes
    return dict(instruction_entries=entries, stopped_before=STOP, globals_before=words(var),
                entry_query_modes=entry_queries, controlled_root_guard_target=STOP,
                globals_after=words(after), state_writes=writes,
                query_after=cpu.mem_read(GLOBALS + 0x6B83, 1)[0],
                executable_sha256=digest(executable), cod_sha256=digest(cod), dic_sha256=digest(dic),
                var_sha256=digest(var), unmodified_executable=True, unmodified_scripts=True,
                scope="bounded rejected-A6 tail with an earlier menu active; not a full DOS playthrough")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("disc", type=Path)
    parser.add_argument("earned_save", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = (args.disc / "BLOOD2PG.EXE").read_bytes()
    assert digest(executable) == SHA256
    cod = (args.disc / "SCRIPT13.COD").read_bytes()
    dic = (args.disc / "SCRIPT13.DIC").read_bytes()
    var_size = (args.disc / "SCRIPT13.VAR").stat().st_size
    saved = args.earned_save.read_bytes()
    result = run(executable, cod, dic, saved[610:610 + var_size])
    result["earned_save_sha256"] = digest(saved)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
