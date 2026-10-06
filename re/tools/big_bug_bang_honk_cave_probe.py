#!/usr/bin/env python3
"""Execute one original Honk VM pass using an earned, unmodified save as input.

This is a bounded instruction oracle, not a normal-gameplay route witness.
Only the actor/player activity flags, C4 records, and frame globals are seeded.
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

HEADER, CODE_BASE = 0x800, 0x5020
GLOBALS, SOURCE, STATE, DICTIONARY = 0x30000, 0x40000, 0x50000, 0x60000
SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("disc", type=Path)
    parser.add_argument("save", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--start", type=lambda s: int(s, 0), default=0x2375)
    parser.add_argument("--end", type=lambda s: int(s, 0), default=0x3BD8)
    args = parser.parse_args()
    executable = (args.disc / "BLOOD2PG.EXE").read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    saved = args.save.read_bytes()
    assert struct.unpack_from("<H", saved)[0] == 1
    cod = bytearray((args.disc / "SCRIPT2.COD").read_bytes())
    dic = (args.disc / "SCRIPT2.DIC").read_bytes()
    deb = (args.disc / "SCRIPT2.DEB").read_bytes()
    var = bytearray(saved[610:610 + 8368])
    assert len(var) == 8368
    patches = saved[610 + 8368:]
    for offset, active in struct.iter_unpack("<HB", patches):
        assert cod[offset - 1] == 0xA9
        cod[offset] = active
    # The same active presentation records consumed by native frame 1908.
    for offset in (0x115A, 0x2A):
        struct.pack_into("<H", var, offset, (struct.unpack_from("<H", var, offset)[0] & 0x7FFF) | 1)
    struct.pack_into("<HHH", var, 0x1192, 0xC4, 0x28, 0)
    player_action = 0x28 + executable[0xF7F0 + 0x7128 + 19 * 16]
    struct.pack_into("<HHH", var, player_action, 0xC4, 0x1158, 0)
    before = bytearray(0x10000)
    native = executable[0xF7F0:]
    before[:len(native)] = native
    for offset, address in ((0x6AF4, SOURCE), (0x6AEC, STATE), (0x6AFC, DICTIONARY),
                            (0x6AF0, 0x70000)):
        struct.pack_into("<HH", before, offset, 0, address // 16)
    struct.pack_into("<Hh", before, 0x6B50, 1, -1)
    before[0x6B7E] = 1
    before[0x6B87] = 2
    before[0x2A33] = 1
    before[0x6B92] = 1
    before[0x2200] = 1
    struct.pack_into("<HH", before, 0x6B4A, args.end, args.start)
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    cpu.mem_write(GLOBALS, bytes(before))
    cpu.mem_write(SOURCE, bytes(cod))
    cpu.mem_write(STATE, bytes(var))
    cpu.mem_write(DICTIONARY, dic)
    cpu.mem_write(0x70000, deb)
    for register, value in ((UC_X86_REG_CS, CODE_BASE // 16), (UC_X86_REG_DS, GLOBALS // 16),
                            (UC_X86_REG_GS, GLOBALS // 16), (UC_X86_REG_SS, GLOBALS // 16),
                            (UC_X86_REG_SP, 0xFF00)):
        cpu.reg_write(register, value)
    entries, yields, writes, terminal = [], [], [], []
    source = None

    def instruction(machine, address, size, _context):
        nonlocal source
        address += HEADER
        assert (0x5800 <= address < address + size <= 0x8830
                or 0x3163 <= address < address + size <= 0x31B4), (hex(address), hex(source or 0))
        if address == 0x5B3D:
            terminal.append(machine.reg_read(UC_X86_REG_SI))
            machine.emu_stop()
        elif address == 0x5AC1:
            source = machine.reg_read(UC_X86_REG_SI)
            if source == args.end:
                terminal.append(source)
                machine.emu_stop()
                return
            entries.append(dict(source=source, opcode=cpu.mem_read(SOURCE + source, 1)[0]))
        elif address == 0x5AD7:
            yields.append(dict(source=source, signal=cpu.mem_read(GLOBALS + 0x6B8A, 1)[0]))

    def write(_machine, _access, address, size, value, _context):
        if STATE <= address < STATE + len(var):
            writes.append(dict(source=source, offset=address - STATE, size=size, value=value))

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start(0x5AA6 - HEADER, 0, count=1000000)
    assert terminal and cpu.reg_read(UC_X86_REG_SP) == 0xFF00
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert bytes(cpu.mem_read(DICTIONARY, len(dic))) == dic
    assert bytes(cpu.mem_read(0x70000, len(deb))) == deb
    after = bytes(cpu.mem_read(GLOBALS, len(before)))
    result = dict(inputs={name: hashlib.sha256((args.disc / name).read_bytes()).hexdigest()
                          for name in ("BLOOD2PG.EXE", "SCRIPT2.COD", "SCRIPT2.DIC", "SCRIPT2.DEB")},
                  save_sha256=hashlib.sha256(saved).hexdigest(), entries=entries,
                  yields=yields, writes=writes, terminal=terminal,
                  objet_enabled_after=cpu.mem_read(SOURCE + 0x3361, 1)[0],
                  actor=list(struct.unpack("<HHH", cpu.mem_read(STATE + 0x1192, 6))),
                  player=list(struct.unpack("<HHH", cpu.mem_read(STATE + player_action, 6))),
                  vm=after[0x6B7E], request=after[0x6B80], menu=after[0x6B86],
                  locked=after[0x6B8D], subtitle=after[0x6234])
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({key: value for key, value in result.items() if key not in ("entries", "writes", "yields")}))
    print("nonzero yields", [row for row in yields if row["signal"]])
    print("action writes", [row for row in writes if row["offset"] in (player_action, 0x1192)])


if __name__ == "__main__":
    main()
