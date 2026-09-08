#!/usr/bin/env python3
"""Execute BBB's unchanged Honk choice handler, recording the SND loader call."""

import argparse
import hashlib
import itertools
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_IP, UC_X86_REG_AX, UC_X86_REG_SI

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
GLOBALS = 0x30000


def run(executable, phase, source):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    cpu.mem_write(0, executable)
    before = bytearray(0x10000)
    before[0x27B7] = phase
    struct.pack_into("<H", before, 0x6B24, source)
    struct.pack_into("<HH", before, 0x6B3A, 0xC5, 0x300)
    before[0xF64:0xF71] = executable[0xF7F0 + 0xF64:0xF7F0 + 0xF71]
    assert before[0xF64:0xF71] == b"sn\\radio.snd\0"
    cpu.mem_write(GLOBALS, bytes(before))
    cpu.reg_write(UC_X86_REG_CS, 0)
    cpu.reg_write(UC_X86_REG_DS, GLOBALS // 16)
    calls, reached = [], []

    def instruction(machine, address, size, _context):
        if address == 0x98CB:
            assert machine.reg_read(UC_X86_REG_AX) == 1
            assert machine.reg_read(UC_X86_REG_SI) == 0xF64
            assert machine.reg_read(UC_X86_REG_DS) == GLOBALS // 16
            calls.append("sn\\radio.snd")
            # The unchanged handler's external loader boundary, not a patched body.
            machine.reg_write(UC_X86_REG_IP, 0x98D0)
        elif address == 0x98D0:
            reached.append(address)
            machine.emu_stop()
        else:
            assert 0x98AD <= address < address + size <= 0x98CB

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.emu_start(0x98AD, 0, count=30)
    assert reached == [0x98D0]
    assert bytes(cpu.mem_read(0, len(executable))) == executable
    after = bytes(cpu.mem_read(GLOBALS, len(before)))
    expected = before[:]
    if phase & 1:
        expected[0x27B7] = 0
        struct.pack_into("<HH", expected, 0x6B3A, 0xC3, source)
    assert after == expected
    assert len(calls) == phase & 1
    return dict(phase=phase, source=source, phase_after=after[0x27B7],
                deferred_kind=struct.unpack_from("<H", after, 0x6B3A)[0],
                deferred_link=struct.unpack_from("<H", after, 0x6B3C)[0], calls=calls)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    assert struct.unpack_from("<H", executable, 0x98A3)[0] + 0x8830 == 0x98AD
    cases = [run(executable, *case) for case in itertools.product(range(256), (0x200, 0x400))]
    args.output.write_text("[\n" + ",\n".join(json.dumps(case, sort_keys=True) for case in cases) + "\n]\n")
    print(f"verified {len(cases)} original Honk activation/bank-call cases")


if __name__ == "__main__":
    main()
