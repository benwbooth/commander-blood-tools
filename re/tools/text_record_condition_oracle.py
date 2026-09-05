#!/usr/bin/env python3
"""Compare A6 record conditions by executing each game's original procedure.

Only the record-comparison controls are enabled. The procedure runs from its
entry through RET without patched callees; random/history/menu paths are not
covered by these cases. Original field matrices are loaded from each binary.
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
    UC_X86_REG_BP, UC_X86_REG_CX, UC_X86_REG_IP, UC_X86_REG_EFLAGS,
)

BUILDS = {
    "commander": ("7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823",
                  0x6339, 0x6433, 0xD420, 0x6D60),
    "sequel": ("4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834",
               0x6B28, 0x6C45, 0xF7F0, 0x7128),
}
GLOBAL_BASE = 0x30000
SOURCE_BASE = 0x40000
RECORD_BASE = 0x50000
RETURN_IP = 0x200
STACK_TOP = 0xFF00
SOURCE_OFFSET = 0x100
RECORD_OFFSET = 0x200
SEGMENT_SIZE = 65536


def run(executable, game, flags, record, operand):
    _digest, entry, end, data_start, matrix = BUILDS[game]
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x70000)
    machine.mem_write(0, executable)
    globals_before = bytearray(SEGMENT_SIZE)
    native_data = executable[data_start:]
    globals_before[:len(native_data)] = native_data
    struct.pack_into("<H", globals_before, STACK_TOP, RETURN_IP)
    machine.mem_write(GLOBAL_BASE, bytes(globals_before))
    source_before = bytearray([0xA7] * SEGMENT_SIZE)
    struct.pack_into("<H", source_before, SOURCE_OFFSET, operand)
    machine.mem_write(SOURCE_BASE, bytes(source_before))
    record_before = bytearray([0xB6] * SEGMENT_SIZE)
    selector = (((flags >> 8) >> 1) & 7) + 1
    field = globals_before[matrix + selector * 16 + 1]
    struct.pack_into("<H", record_before, RECORD_OFFSET + field, record)
    machine.mem_write(RECORD_BASE, bytes(record_before))
    preserved = {
        UC_X86_REG_CS: 0, UC_X86_REG_DS: SOURCE_BASE // 16,
        UC_X86_REG_ES: RECORD_BASE // 16, UC_X86_REG_GS: GLOBAL_BASE // 16,
        UC_X86_REG_SS: GLOBAL_BASE // 16, UC_X86_REG_SP: STACK_TOP,
        UC_X86_REG_SI: SOURCE_OFFSET, UC_X86_REG_DI: RECORD_OFFSET,
        UC_X86_REG_BP: 0x4567, UC_X86_REG_CX: flags,
    }
    for register, value in preserved.items():
        machine.reg_write(register, value)
    # Exercise both carry seeds; the routine must publish its own result.
    machine.reg_write(UC_X86_REG_EFLAGS, 2 | (operand & 1))
    executed = set()

    def instruction(_cpu, address, _size, _context):
        if not entry <= address < end:
            raise AssertionError(f"execution escaped procedure at {address:#x}")
        executed.add(address)

    def write(_cpu, _access, address, size, _value, _context):
        if not GLOBAL_BASE + STACK_TOP - 4 <= address < address + size <= GLOBAL_BASE + STACK_TOP:
            raise AssertionError(f"unexpected memory write at {address:#x}")

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, write)
    machine.emu_start(entry, RETURN_IP, count=200)
    assert machine.reg_read(UC_X86_REG_IP) == RETURN_IP and executed
    preserved[UC_X86_REG_SP] += 2
    for register, expected in preserved.items():
        assert machine.reg_read(register) == expected, (game, flags, register)
    globals_after = bytearray(machine.mem_read(GLOBAL_BASE, SEGMENT_SIZE))
    globals_after[STACK_TOP - 4:STACK_TOP] = globals_before[STACK_TOP - 4:STACK_TOP]
    assert globals_after == globals_before
    assert machine.mem_read(SOURCE_BASE, SEGMENT_SIZE) == source_before
    assert machine.mem_read(RECORD_BASE, SEGMENT_SIZE) == record_before
    return {"game": game, "flags": flags, "record": record, "operand": operand,
            "accepted": bool(machine.reg_read(UC_X86_REG_EFLAGS) & 1)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("commander", type=Path)
    parser.add_argument("sequel", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    results = []
    for game in BUILDS:
        executable = getattr(args, game).read_bytes()
        if hashlib.sha256(executable).hexdigest() != BUILDS[game][0]:
            raise SystemExit(f"unsupported {game} executable build")
        for flags, record, operand in itertools.product(
            [4, 132, 260, 388], [0, 1, 32767, 32768, 65534, 65535],
            [0, 1, 32767, 32768, 65534, 65535],
        ):
            results.append(run(executable, game, flags, record, operand))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("".join(json.dumps(row, separators=(",", ":")) + "\n" for row in results))
    print(f"wrote {len(results)} original record-condition cases")


if __name__ == "__main__":
    main()
