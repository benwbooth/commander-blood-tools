#!/usr/bin/env python3
"""Execute original CB A6 and its outer loop, stopping before presentation scan."""

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import (
    UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_GS, UC_X86_REG_SS,
    UC_X86_REG_SP, UC_X86_REG_IP, UC_X86_REG_SI,
)

SHA256 = "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
HEADER, CODE_BASE, DATA_START = 0x600, 0x4DA0, 0xD420
GLOBALS, SOURCE, STATE, DICTIONARY, STACK = 0x30000, 0x40000, 0x50000, 0x60000, 0xFF00
RANGES = [(0x55F8, 0x568A), (0x660C, 0x67BD), (0x6339, 0x6433), (0x647B, 0x6494)]


def run(executable, subtitle, gate, locked):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    var, deb = bytearray(), bytearray()
    for name, kind, size in [(b"blood", 1, 34), (b"actor", 2, 72)]:
        deb.extend(struct.pack("<16sHH", name, len(var), 1))
        record = bytearray(size)
        struct.pack_into("<H", record, 0, kind)
        var.extend(record)
    deb.extend(bytes(20))
    struct.pack_into("<H", var, 36, 0x8000 if gate == "shown" else 0)
    action_offset = executable[DATA_START + 0x6D60 + 19 * 16 + 1]
    struct.pack_into("<H", var, 34 + action_offset, 195 if gate == "wrong_record" else 196)
    dic = bytes(32) + b"TEXTE\0"
    cod = struct.pack("<BHbHHH", 0xA6, 34, -3, 0 if gate == "inactive" else 0x8000, 32, 0) + b"\xFF"
    before = bytearray(0x10000)
    native_data = executable[DATA_START:]
    before[:len(native_data)] = native_data
    for offset, value in [(0x671C, SOURCE), (0x6724, STATE), (0x6728, DICTIONARY)]:
        struct.pack_into("<HH", before, offset, 0, value // 16)
    for offset, value in [(0x67A8, 1), (0x67B7, locked), (0x67AB, 0), (0x67B1, 0),
                          (0x67B9, int(subtitle)), (0x67B0, int(gate == "menu")),
                          (0x5E64, int(gate == "subtitle")), (0x67BC, 1)]:
        before[offset] = value
    cpu.mem_write(GLOBALS, bytes(before))
    cpu.mem_write(SOURCE, cod)
    cpu.mem_write(STATE, bytes(var))
    cpu.mem_write(DICTIONARY, dic)
    for reg, value in [(UC_X86_REG_CS, CODE_BASE // 16), (UC_X86_REG_DS, GLOBALS // 16),
                       (UC_X86_REG_GS, GLOBALS // 16), (UC_X86_REG_SS, GLOBALS // 16),
                       (UC_X86_REG_SP, STACK)]:
        cpu.reg_write(reg, value)
    yields = []

    def instruction(_cpu, address, size, _context):
        address += HEADER
        assert any(a <= address and address + size <= b for a, b in RANGES), hex(address)
        if address == 0x562A:
            yields.append(cpu.mem_read(GLOBALS + 0x67B4, 1)[0])

    def write(_cpu, _access, address, size, _value, _context):
        allowed = [(GLOBALS, 0x10000), (SOURCE + 5, 1), (STATE + 36, 2)]
        assert any(a <= address and address + size <= a + n for a, n in allowed), hex(address)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start(0x55F8 - HEADER, 0x568A - HEADER, count=10000)
    assert cpu.reg_read(UC_X86_REG_IP) == 0x568A - HEADER - CODE_BASE
    assert cpu.reg_read(UC_X86_REG_SP) == STACK
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert bytes(cpu.mem_read(DICTIONARY, len(dic))) == dic
    after = bytes(cpu.mem_read(GLOBALS, len(before)))
    return dict(name=f"{'subtitle' if subtitle else 'menu'}_{gate}_lock{locked}",
                subtitle=subtitle, gate=gate, locked_before=bool(locked),
                cod=cod.hex(), var=var.hex(), deb=deb.hex(), dic=dic.hex(),
                var_after=bytes(cpu.mem_read(STATE, len(var))).hex(),
                start_locked=bool(after[0x67B7]), vm=bool(after[0x67A8]),
                yield_signals=yields, cursor=cpu.reg_read(UC_X86_REG_SI) - 1)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    rows = [run(executable, subtitle, gate, locked)
            for subtitle in [False, True]
            for gate in ["none", "inactive", "shown", "wrong_record", "menu", "subtitle"]
            for locked in [0, 1]]
    assert {signal for row in rows for signal in row["yield_signals"]} == {0, 2}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("".join(json.dumps(row, sort_keys=True) + "\n" for row in rows))
    print(f"captured {len(rows)} original CB A6 and outer-loop cases")


if __name__ == "__main__":
    main()
