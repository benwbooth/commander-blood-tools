#!/usr/bin/env python3
"""Probe native sequel travel-option branches without replacing instructions.

This is a branch-level oracle, not a complete navigation execution. Stop before
resource loading, palette setup, teardown, and external calls. Run with python -P.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import (
    UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_ES, UC_X86_REG_GS,
    UC_X86_REG_SI, UC_X86_REG_DI, UC_X86_REG_IP, UC_X86_REG_EFLAGS,
)

EXE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
GLOBALS = 0x30000
OBJECTS = 0x50000
TARGET = 0x1234


def probe(executable, case):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    cpu.mem_write(0, executable)
    before = bytearray(0x10000)
    before[0xCF1] = case["travel_flag"]
    struct.pack_into("<H", before, 0x6B22, TARGET)
    struct.pack_into("<H", before, 0x6B3C,
                     TARGET if case.get("same_target", True) else TARGET + 2)
    struct.pack_into("<H", before, 0x6B3A, 0x55AA)
    before[0x2A26] = 0x55
    before[0x2A33] = 0x81
    before[0x2D03] = case.get("line_flags", 0)
    before[0x2A7F] = case.get("phase", 0)
    cpu.mem_write(GLOBALS, bytes(before))
    record = bytearray(0x40)
    struct.pack_into("<H", record, 0x14, case.get("resource", 0))
    cpu.mem_write(OBJECTS, bytes(record))
    for register, value in {
        UC_X86_REG_CS: 0, UC_X86_REG_DS: GLOBALS // 16,
        UC_X86_REG_GS: GLOBALS // 16, UC_X86_REG_ES: OBJECTS // 16,
        UC_X86_REG_SI: TARGET if case.get("same_target", True) else TARGET + 2,
        UC_X86_REG_DI: 0, UC_X86_REG_EFLAGS: 2,
    }.items():
        cpu.reg_write(register, value)
    entry, end, stops, writable = {
        "dispatch": (0x616B, 0x6184,
                     {0x6184: "dispatch_resource", 0x65DD: "skip_dispatch"}, set()),
        "arrival": (0x89C4, 0x89E5,
                    {0x89E5: "prepare_palette", 0x8A42: "return_without_palette"},
                    {0x2A33, 0x2D03}),
        "first_frame": (0x90B5, 0x90D2,
                        {0x9139: "continue_actor", 0x90EF: "reset_bridge"},
                        {0x6B3A, 0x6B3B, 0x2A26}),
        "completion": (0x90E8, 0x90EF,
                       {0x9125: "retain_deferred_action", 0x90EF: "reset_bridge"},
                       set()),
    }[case["gate"]]
    reached = None

    def instruction(machine, address, size, _context):
        nonlocal reached
        if address in stops:
            reached = stops[address]
            machine.emu_stop()
        else:
            assert entry <= address < address + size <= end, hex(address)

    def write(_machine, _access, address, size, _value, _context):
        assert all(offset - GLOBALS in writable
                   for offset in range(address, address + size)), hex(address)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start(entry, 0xFFFF, count=40)
    assert reached is not None, "instruction budget exhausted"
    assert cpu.reg_read(UC_X86_REG_IP) in stops
    assert bytes(cpu.mem_read(0, len(executable))) == executable
    assert bytes(cpu.mem_read(OBJECTS, len(record))) == record
    after = bytes(cpu.mem_read(GLOBALS, len(before)))
    assert all(a == b or index in writable
               for index, (a, b) in enumerate(zip(before, after)))
    return dict(case, outcome=reached, ui_flags=after[0x2A33],
                result_line_flags=after[0x2D03], countdown=after[0x2A26],
                deferred_action=struct.unpack_from("<H", after, 0x6B3A)[0])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    if hashlib.sha256(executable).hexdigest() != EXE_SHA256:
        raise ValueError("unrecognized executable; refusing fixed-offset probe")
    cases = [dict(gate="dispatch", travel_flag=flag, same_target=same, phase=phase)
             for flag in (0, 1) for same in (False, True)
             for phase in (0, 1, 3, 4, 5, 255)]
    cases += [dict(gate="arrival", travel_flag=flag, resource=resource, line_flags=line)
              for flag in (0, 1) for resource in (0, 0x1234)
              for line in (0, 2, 0x80, 0x82)]
    cases += [dict(gate="first_frame", travel_flag=flag, same_target=same)
              for flag in (0, 1) for same in (False, True)]
    cases += [dict(gate="completion", travel_flag=flag) for flag in (0, 1)]
    rows = [probe(executable, case) for case in cases]
    for row in rows:
        enabled = row["travel_flag"] != 0
        if row["gate"] == "dispatch":
            skip = not enabled and row["same_target"] and row["phase"] < 4
            assert row["outcome"] == ("skip_dispatch" if skip else "dispatch_resource")
        elif row["gate"] == "arrival":
            skip = not enabled and row["resource"] == 0
            assert row["outcome"] == ("return_without_palette" if skip else "prepare_palette")
            assert row["ui_flags"] == (0x85 if skip else 0x81)
            assert row["result_line_flags"] == (row["line_flags"] | (
                8 if skip and row["line_flags"] & 2 == 0 else 0))
        elif row["gate"] == "first_frame":
            reset = enabled and row["same_target"]
            assert row["outcome"] == ("reset_bridge" if reset else "continue_actor")
            assert row["countdown"] == (0x55 if reset else 8)
            assert row["deferred_action"] == (0xC1 if enabled else 0x55AA)
        else:
            assert row["outcome"] == ("reset_bridge" if enabled else "retain_deferred_action")
    with args.output.open("x") as output:
        for row in rows:
            output.write(json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n")
    print(f"verified {len(rows)} original travel-option branch cases")


if __name__ == "__main__":
    main()
