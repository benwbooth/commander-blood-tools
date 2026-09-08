#!/usr/bin/env python3
"""Run original A8, completion-flag propagation, and main-loop exit gate.

These are bounded branch-entry probes, not video playback or DOS cleanup.
No instructions or callees are replaced.
"""
import argparse
import hashlib
import json
from pathlib import Path

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE
from unicorn.x86_const import (
    UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_GS, UC_X86_REG_SS,
    UC_X86_REG_SP, UC_X86_REG_SI, UC_X86_REG_IP,
)

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER = 0x800
DATA = 0x30000


def probe(executable, basename):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    cpu.mem_write(DATA, bytes(0x10000))
    cpu.mem_write(DATA + 0x8000, basename.encode("ascii") + b"\0\0")
    cpu.mem_write(DATA + 0xFF00, b"\x00\x02")
    cpu.mem_write(DATA + 0x2745, b"\x01\x00")
    for register, value in [(UC_X86_REG_CS, 0), (UC_X86_REG_DS, DATA // 16),
                            (UC_X86_REG_GS, DATA // 16), (UC_X86_REG_SS, DATA // 16),
                            (UC_X86_REG_SP, 0xFF00), (UC_X86_REG_SI, 0x8000)]:
        cpu.reg_write(register, value)
    allowed = [(0x6E7C, 0x6EE4), (0xB6CE, 0xB6D8), (0x1140, 0x1149)]

    def instruction(_cpu, address, size, _context):
        address += HEADER
        if address in (0x1149, 0x13D8):
            cpu.emu_stop()
            return
        assert any(a <= address and address + size <= b for a, b in allowed), hex(address)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.emu_start(0x6E7C - HEADER, 0x200, count=1000)
    assert cpu.reg_read(UC_X86_REG_IP) == 0x200
    assert cpu.reg_read(UC_X86_REG_SP) == 0xFF02
    finale = cpu.mem_read(DATA + 0x6B93, 1)[0]
    assert bytes(cpu.mem_read(DATA + 0x2372, len(basename) + 1)) == basename.encode() + b"\0"
    cpu.emu_start(0xB6CE - HEADER, 0xB6D8 - HEADER, count=10)
    exit_flag = cpu.mem_read(DATA + 0xD1D, 1)[0]
    cpu.emu_start(0x1140 - HEADER, 0xFFFF, count=10)
    target = cpu.reg_read(UC_X86_REG_IP) + HEADER
    assert target in (0x1149, 0x13D8)
    assert bytes(cpu.mem_read(0, len(module))) == module
    return dict(basename=basename, finale=finale, exit_flag=exit_flag,
                main_loop_target=target, shutdown=target == 0x13D8)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    cases = [probe(executable, name) for name in ("fin.hnm", "fin.other", "FIN.HNM", "affin.hnm", "venus06.hnm")]
    with args.output.open("x") as output:
        json.dump(dict(executable_sha256=SHA256, cases=cases), output, indent=2)
        output.write("\n")


if __name__ == "__main__":
    main()
