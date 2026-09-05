#!/usr/bin/env python3
"""Execute Big Bug Bang's original D3 handler to record Rust regression vectors.

Unicorn is a research-only test oracle, never a dependency of the game. The
golden results come from the original instructions, not a second arithmetic
implementation. No original machine-code bytes are written to the output.
Run with python3 -P so re/tools/dis.py cannot shadow Python's standard library.
"""

import argparse
import hashlib
import json
from pathlib import Path
import random
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_INTR
from unicorn.x86_const import (
    UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_ES, UC_X86_REG_GS,
    UC_X86_REG_SS, UC_X86_REG_SP, UC_X86_REG_SI, UC_X86_REG_IP,
)

EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HANDLER_START = 0x7408
RETURN_IP = 0x200
STATE_POINTER_OFFSET = 0x6AEC
QUERY_MODE_OFFSET = 0x6B83
CODE_SEGMENT = 0x2000
GLOBAL_SEGMENT = 0x3000
STATE_SEGMENT = 0x4000
STACK_SEGMENT = 0x5000
STATE_BASE = 0x180
SCRIPT_OFFSET = 0x40
TARGET_OFFSET = 2
MULTIPLIER_OFFSET = 4
DIVISOR_OFFSET = 6
MULTIPLY_DIVIDE_OPCODE = 0xD3
INDIRECT_MODES = {0xC0, 0xC2}
STATE_BYTES = 32


def run(executable, name, current, multiplier, divisor, modes, query, alias=None):
    state = bytearray(range(STATE_BYTES))
    for offset, value in [(TARGET_OFFSET, current), (MULTIPLIER_OFFSET, multiplier), (DIVISOR_OFFSET, divisor)]:
        struct.pack_into("<H", state, offset, value)
    addresses = [MULTIPLIER_OFFSET, DIVISOR_OFFSET]
    if alias is not None:
        addresses[alias] = TARGET_OFFSET
    operands = [addresses[i] if modes[i] in INDIRECT_MODES else value
                for i, value in enumerate([multiplier, divisor])]
    token = struct.pack("<BHBHBH", MULTIPLY_DIVIDE_OPCODE, TARGET_OFFSET,
                        modes[0], operands[0], modes[1], operands[1])
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 1048576)
    machine.mem_write(0, executable)
    machine.mem_write(CODE_SEGMENT * 16 + SCRIPT_OFFSET, token)
    machine.mem_write(GLOBAL_SEGMENT * 16 + STATE_POINTER_OFFSET, struct.pack("<HH", STATE_BASE, STATE_SEGMENT))
    machine.mem_write(GLOBAL_SEGMENT * 16 + QUERY_MODE_OFFSET, bytes([query]))
    machine.mem_write(STATE_SEGMENT * 16 + STATE_BASE, bytes(state))
    stack_pointer = 0xFFF0
    machine.mem_write(STACK_SEGMENT * 16 + stack_pointer, struct.pack("<H", RETURN_IP))
    for register, value in [
        (UC_X86_REG_CS, 0), (UC_X86_REG_DS, CODE_SEGMENT),
        (UC_X86_REG_GS, GLOBAL_SEGMENT), (UC_X86_REG_SS, STACK_SEGMENT),
        (UC_X86_REG_SP, stack_pointer), (UC_X86_REG_ES, 0),
        (UC_X86_REG_SI, SCRIPT_OFFSET + 1),
    ]:
        machine.reg_write(register, value)
    interrupts = []

    def interrupt(cpu, number, _context):
        interrupts.append(number)
        cpu.emu_stop()

    machine.hook_add(UC_HOOK_INTR, interrupt)
    machine.emu_start(HANDLER_START, RETURN_IP, count=100)
    assert interrupts in ([], [0]), interrupts
    if not interrupts:
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_IP, "handler did not return"
        assert machine.reg_read(UC_X86_REG_SI) == SCRIPT_OFFSET + len(token), "operand consumption changed"
    query_after = machine.mem_read(GLOBAL_SEGMENT * 16 + QUERY_MODE_OFFSET, 1)[0]
    assert query_after == query
    after = bytes(machine.mem_read(STATE_SEGMENT * 16 + STATE_BASE, STATE_BYTES))
    return {"name": name, "query_mode": query, "token": list(token),
            "state_before": list(state), "state_after": list(after),
            "divide_error": bool(interrupts)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    if hashlib.sha256(executable).hexdigest() != EXECUTABLE_SHA256:
        raise SystemExit("unsupported BLOOD2PG.EXE build; refusing fixed-offset oracle")
    cases = [(0, 0, 1), (7, 5, 3), (32768, 2, 2), (65535, 65535, 65535),
             (65535, 2, 1), (1, 1, 0), (0, 0, 0)]
    vectors = []
    for query in [0, 1]:
        for modes in [(0, 0), (0xC1, 0xA1), (0xC0, 0), (0, 0xC2), (0xC2, 0xC0)]:
            for index, values in enumerate(cases):
                name = f"q{query}_m{modes[0]}_{modes[1]}_edge{index}"
                vectors.append(run(executable, name, *values, modes, query))
        for alias in [0, 1]:
            vectors.append(run(executable, f"q{query}_alias{alias}", 43, 2, 3,
                               (0xC0, 0xC2), query, alias))
    randomizer = random.Random(20260905)
    for index in range(40):
        values = [randomizer.randrange(65536) for _ in range(3)]
        modes = [randomizer.choice([0, 0xC0, 0xC2, 0xFF]) for _ in range(2)]
        vectors.append(run(executable, f"random{index}", *values, modes, index % 2))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("".join(json.dumps(vector, separators=(",", ":")) + "\n" for vector in vectors))
    print(f"wrote {len(vectors)} original-handler vectors ({sum(v['divide_error'] for v in vectors)} divide errors)")


if __name__ == "__main__":
    main()
