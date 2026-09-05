#!/usr/bin/env python3
"""Execute original sequel CC/D7 and their presentation decision blocks.

The decision probes stop at explicit branch destinations before rendering or
audio calls. They verify control and writes, not the unexecuted media services.
No original executable or authored script bytes are written to the fixture.
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
    UC_X86_REG_SS, UC_X86_REG_SP, UC_X86_REG_SI, UC_X86_REG_IP,
)

EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
GLOBAL_SEGMENT = 0x3000
STACK_TOP = 0xFF00
RETURN_IP = 0x1000
SCRIPT_OFFSET = 0x100
ENDING = 0x6B73
PENDING_CHOICE = 0x2A84
SELECTED_CHOICE = 0x2A83
CHOICE_TABLE = 0x7086
QUERY = 0x6B83
PRIMARY = 0x0C36
REVERSE = 0x2A80
SHUTDOWN = 0x0D1D


def run(executable, name, entry, stops, writes, values, token=b"", stack_scratch=0):
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x60000)
    machine.mem_write(0, executable)
    before = bytearray([0xA4] * 65536)
    for offset, value in values.items():
        before[offset] = value
    before[SCRIPT_OFFSET:SCRIPT_OFFSET + len(token)] = token
    # The native VM's BP-relative globals use SS, shared with its data segment.
    struct.pack_into("<H", before, STACK_TOP, RETURN_IP)
    machine.mem_write(GLOBAL_SEGMENT * 16, bytes(before))
    for register, value in [(UC_X86_REG_CS, 0), (UC_X86_REG_DS, GLOBAL_SEGMENT),
                            (UC_X86_REG_GS, GLOBAL_SEGMENT), (UC_X86_REG_ES, GLOBAL_SEGMENT),
                            (UC_X86_REG_SS, GLOBAL_SEGMENT), (UC_X86_REG_SP, STACK_TOP),
                            (UC_X86_REG_SI, SCRIPT_OFFSET + 1)]:
        machine.reg_write(register, value)
    reached = []

    def instruction(cpu, address, _size, _context):
        if address in stops:
            reached.append(address)
            cpu.emu_stop()

    machine.hook_add(UC_HOOK_CODE, instruction)
    def memory_write(_cpu, _access, address, size, _value, _context):
        assert GLOBAL_SEGMENT * 16 <= address and address + size <= (GLOBAL_SEGMENT + 4096) * 16, (name, address)

    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    machine.emu_start(entry, 0, count=200)
    assert len(reached) == 1, (name, hex(machine.reg_read(UC_X86_REG_IP)))
    after = bytearray(machine.mem_read(GLOBAL_SEGMENT * 16, len(before)))
    changes = {str(offset): after[offset] for offset in writes}
    for offset in writes:
        after[offset] = before[offset]
    after[STACK_TOP - stack_scratch:STACK_TOP] = before[STACK_TOP - stack_scratch:STACK_TOP]
    assert before == after, f"{name}: unexpected write outside declared output"
    return {"name": name, "input": {str(k): v for k, v in values.items()},
            "token": list(token), "destination": reached[0], "output": changes,
            "next_script_offset": machine.reg_read(UC_X86_REG_SI) - SCRIPT_OFFSET}


def vectors(executable):
    for ending, query in itertools.product([0, 1, 128, 255], [0, 1]):
        yield run(executable, f"d7_{ending}_{query}", 0x6E67, [RETURN_IP], [ENDING],
                  {ENDING: ending, QUERY: query}, b"\xd7")
    for slot, query, name in itertools.product(range(1, 7), [0, 1], [b"present", b"end", b""]):
        token = bytes([0xCC, slot]) + name + b"\0\0"
        yield run(executable, f"cc_{slot}_{query}_{name.decode()}", 0x69E6,
                  [RETURN_IP], [PENDING_CHOICE, *range(CHOICE_TABLE, CHOICE_TABLE + 96)],
                  {PENDING_CHOICE: 255, QUERY: query,
                   **{CHOICE_TABLE + index * 16: 0 for index in range(6)}}, token)
    for ending, pending, primary in itertools.product([0, 1], [255, 0, 5], [0, 1]):
        yield run(executable, f"queued_{ending}_{pending}_{primary}", 0x8C14,
                  [0x8C33, 0x8C9D], [PENDING_CHOICE, SELECTED_CHOICE],
                  {ENDING: ending, PENDING_CHOICE: pending, PRIMARY: primary,
                   SELECTED_CHOICE: 2})
    for ending, reverse in itertools.product([0, 1], [0, 1]):
        yield run(executable, f"completed_{ending}_{reverse}", 0x8C75,
                  [0x8CA4, 0x8D81], [SHUTDOWN],
                  {ENDING: ending, REVERSE: reverse, SHUTDOWN: 0})
    for ending, pending in itertools.product([0, 1], [255, 0, 5]):
        yield run(executable, f"actor_{ending}_{pending}", 0x92E4,
                  [0x92F8, 0x935C], [0x0C2A, 0x0C2B],
                  {ENDING: ending, PENDING_CHOICE: pending, 0x0C2A: 23, 0x0C2B: 0})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    if hashlib.sha256(executable).hexdigest() != EXECUTABLE_SHA256:
        raise SystemExit("unsupported BLOOD2PG.EXE build")
    results = list(vectors(executable))
    args.output.write_text("".join(json.dumps(x, separators=(",", ":")) + "\n" for x in results))
    print(f"wrote {len(results)} original instruction/decision cases")


if __name__ == "__main__":
    main()
