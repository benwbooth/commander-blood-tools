#!/usr/bin/env python3
"""Execute Honk's departure block in the original VM, before presentation scans.

The full original COD/DIC/VAR assets are loaded. Only the documented conversation
records, count, text gates, and resume entry are seeded; instructions are not
patched. These vectors cover block semantics, not a whole-game visual oracle.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_GS, UC_X86_REG_SS, UC_X86_REG_SP, UC_X86_REG_SI

HEADER, CODE_BASE = 0x800, 0x5020
GLOBALS, SOURCE, STATE, DICTIONARY = 0x30000, 0x40000, 0x50000, 0x60000
SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"


def run(executable, cod, var, dic, count, gate):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    state = bytearray(var)
    struct.pack_into("<H", state, 0x1F0A, count)
    struct.pack_into("<H", state, 0x115A, 0x8000 if gate == "shown" else 0)
    struct.pack_into("<HHH", state, 0x1192, 0xC4, 0x28, 0)
    player_action = 0x28 + executable[0xF7F0 + 0x7128 + 19 * 16]
    struct.pack_into("<HHH", state, player_action, 0xC4, 0x1158, 0)
    globals_before = bytearray(0x10000)
    native = executable[0xF7F0:]
    globals_before[:len(native)] = native
    for offset, pointer, segment in [(0x6AF4, 0, SOURCE), (0x6AEC, 0, STATE), (0x6AFC, 0, DICTIONARY)]:
        struct.pack_into("<HH", globals_before, offset, pointer, segment // 16)
    struct.pack_into("<Hh", globals_before, 0x6B50, 1, -1)
    globals_before[0x6B7E] = 1
    globals_before[0x6B87] = 2
    struct.pack_into("<HH", globals_before, 0x6B4A, 0x34BD, 0x3478)
    globals_before[0x6B86] = int(gate == "menu")
    globals_before[0x6234] = int(gate == "subtitle")
    cpu.mem_write(GLOBALS, bytes(globals_before))
    cpu.mem_write(SOURCE, cod)
    cpu.mem_write(STATE, bytes(state))
    cpu.mem_write(DICTIONARY, dic)
    for register, value in [(UC_X86_REG_CS, CODE_BASE // 16), (UC_X86_REG_DS, GLOBALS // 16),
                            (UC_X86_REG_GS, GLOBALS // 16), (UC_X86_REG_SS, GLOBALS // 16), (UC_X86_REG_SP, 0xFF00)]:
        cpu.reg_write(register, value)
    entries, yields, ended = [], [], []

    def instruction(machine, address, size, _context):
        address += HEADER
        assert 0x5800 <= address < address + size <= 0x8830, hex(address)
        if address == 0x5B3D:
            ended.append(machine.reg_read(UC_X86_REG_SI))
            machine.emu_stop()
            return
        if address == 0x5AC1:
            source = machine.reg_read(UC_X86_REG_SI)
            assert 0x3478 <= source < 0x34BD, hex(source)
            entries.append(dict(source=source, opcode=cod[source]))
        elif address == 0x5AD7:
            yields.append(cpu.mem_read(GLOBALS + 0x6B8A, 1)[0])

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.emu_start(0x5AA6 - HEADER, 0, count=10000)
    assert ended == [0x34BD]
    assert cpu.reg_read(UC_X86_REG_SP) == 0xFF00
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert bytes(cpu.mem_read(DICTIONARY, len(dic))) == dic
    after = bytes(cpu.mem_read(GLOBALS, len(globals_before)))
    after_state = bytes(cpu.mem_read(STATE, len(state)))
    after_code = bytes(cpu.mem_read(SOURCE, len(cod)))
    assert all(a == b or i in (0x3488, 0x34AE) for i, (a, b) in enumerate(zip(cod, after_code)))
    allowed_state = {0x115A, 0x115B, 0x1F0A, 0x1F0B, 0x1EE4, 0x1EE5, *range(0x1192, 0x1198), *range(player_action, player_action + 6)}
    assert all(a == b or i in allowed_state for i, (a, b) in enumerate(zip(state, after_state)))
    return dict(count=count, gate=gate, entries=entries, yields=yields,
                state_before_sha256=hashlib.sha256(state).hexdigest(),
                state_after_sha256=hashlib.sha256(after_state).hexdigest(),
                code_after_sha256=hashlib.sha256(after_code).hexdigest(),
                count_after=struct.unpack_from("<H", after_state, 0x1F0A)[0],
                actor_record=list(struct.unpack_from("<HHH", after_state, 0x1192)),
                player_record=list(struct.unpack_from("<HHH", after_state, player_action)),
                vm=after[0x6B7E], subtitle=after[0x6234], menu=after[0x6B86],
                request=after[0x6B80], shown=struct.unpack_from("<H", after_state, 0x115A)[0])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("disc", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = (args.disc / "BLOOD2PG.EXE").read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    cod, var, dic = [(args.disc / name).read_bytes() for name in ("SCRIPT2.COD", "SCRIPT1.VAR", "SCRIPT2.DIC")]
    cases = [run(executable, cod, var, dic, count, gate) for count in (5, 6) for gate in ("none", "shown", "menu", "subtitle")]
    result = dict(inputs={name: hashlib.sha256((args.disc / name).read_bytes()).hexdigest()
                          for name in ("BLOOD2PG.EXE", "SCRIPT2.COD", "SCRIPT1.VAR", "SCRIPT2.DIC", "SCRIPT2.DEB")},
                  cases=cases)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(cases)} original Honk departure block cases")


if __name__ == "__main__":
    main()
