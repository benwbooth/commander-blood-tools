#!/usr/bin/env python3
"""Capture original scene completion and cancellation, with no patched callees.

The active-scene continuation starts after its image/start branch with the eight
saved registers and far return expected by the original epilogue. The queue has
no open file or newly decoded frame; its real service and status helpers run.
Cancellation executes its entire far routine, including buffered/empty release.
These captures do not exercise HNM decoding or the line-five display clear.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import *

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER = 0x800
GLOBALS = 0x30000
STACK = 0xFF00
RETURN = 0x200
RANGES = [(0xB67C, 0xB6A5), (0xB6C4, 0xB763), (0xB924, 0xB942),
          (0xB997, 0xB9EF), (0xBAC7, 0xBADC), (0xBBF5, 0xBC04)]
FIELDS = {
    "vm": (0x6B7E, 1), "line": (0x6B5A, 2), "displayed": (0x6B5C, 2),
    "gate": (0x2200, 1), "request": (0x6B80, 1), "redraw": (0x2A78, 1),
    "ship": (0x2745, 2), "blocked": (0x277F, 1), "overlay": (0xCEC, 1),
    "sound": (0xCED, 1), "finale": (0x6B93, 1), "navigation_sound": (0xD1D, 1),
    "entry": (0xFFD, 2), "read": (0xFAE, 2), "palette": (0x561F, 2),
    "depth_opening": (0x2781, 1), "depth_step": (0x2783, 1),
    "queue_status": (0xFAD, 1), "buffered": (0xFE8, 2),
}


def unpack(data):
    return {name: int.from_bytes(data[offset:offset + size], "little")
            for name, (offset, size) in FIELDS.items()}


def run(executable, name, mode, overrides):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    initial = dict(vm=0, line=43, displayed=7, gate=0x81, request=0xA7,
                   redraw=0, ship=0x88, blocked=0, overlay=0x81, sound=0,
                   finale=0x81, navigation_sound=0, entry=21, read=1, palette=75,
                   depth_opening=0, depth_step=0, queue_status=3, buffered=0)
    initial.update(overrides)
    data = bytearray(0x10000)
    for field, value in initial.items():
        offset, size = FIELDS[field]
        data[offset:offset + size] = value.to_bytes(size, "little")
    # No file handle or forced queue update: B997 takes its ordinary empty path.
    saved = [0] * 8 if mode == "dispatch" else []
    struct.pack_into(f"<{len(saved) + 2}H", data, STACK, *saved, RETURN, 0)
    cpu.mem_write(GLOBALS, bytes(data))
    for reg, value in [(UC_X86_REG_CS, 0), (UC_X86_REG_DS, GLOBALS // 16),
                       (UC_X86_REG_GS, GLOBALS // 16), (UC_X86_REG_SS, GLOBALS // 16),
                       (UC_X86_REG_SP, STACK)]:
        cpu.reg_write(reg, value)
    calls = []
    allowed_writes = {offset + i for offset, size in FIELDS.values() for i in range(size)}
    allowed_writes.add(0xFFA)

    def instruction(_cpu, address, size, _context):
        address += HEADER
        assert any(a <= address and address + size <= b for a, b in RANGES), hex(address)
        if address in [0xB997, 0xBBF5, 0xBAC7, 0xB924]:
            calls.append(address)

    def write(_cpu, _access, address, size, _value, _context):
        offset = address - GLOBALS
        assert (STACK - 64 <= offset and offset + size <= STACK
                or all(i in allowed_writes for i in range(offset, offset + size))), hex(address)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    entry = 0xB67C if mode == "dispatch" else 0xB731
    cpu.emu_start(entry - HEADER, RETURN, count=2000)
    assert cpu.reg_read(UC_X86_REG_CS) == 0
    assert cpu.reg_read(UC_X86_REG_IP) == RETURN
    assert cpu.reg_read(UC_X86_REG_SP) == STACK + 2 * (len(saved) + 2)
    assert bytes(cpu.mem_read(0, len(module))) == module
    result = bytes(cpu.mem_read(GLOBALS, len(data)))
    for i, (before, after) in enumerate(zip(data, result)):
        assert before == after or i in allowed_writes or STACK - 64 <= i < STACK, hex(i)
    return dict(name=name, mode=mode, input=initial, output=unpack(result), calls=calls)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    cases = []
    for status in [0, 1, 2, 3]:
        for blocked in [0, 1]:
            for vm in [0, 1]:
                cases.append((f"dispatch_status{status}_blocked{blocked}_vm{vm}", "dispatch",
                              dict(queue_status=status, blocked=blocked, vm=vm)))
    for name, fields in [
        ("no_ship", dict(ship=0x80)),
        ("low_flags_clear", dict(overlay=0x80, finale=0x80, sound=1, navigation_sound=1)),
        ("palette_reset", dict(queue_status=1, line=39)),
        ("palette_unchanged", dict(queue_status=1, line=39, entry=20)),
        ("depth_start", dict(queue_status=0, entry=9)),
        ("depth_no_ship", dict(queue_status=0, entry=9, ship=0x80)),
    ]:
        cases.append((name, "dispatch", fields))
    for gate in [0, 1, 2, 0x81]:
        for vm in [0, 1]:
            for buffered in [0, 8]:
                cases.append((f"cancel_gate{gate}_vm{vm}_buffered{buffered}", "cancel",
                              dict(gate=gate, vm=vm, buffered=buffered, queue_status=0)))
    rows = [run(executable, *case) for case in cases]
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("".join(json.dumps(row, sort_keys=True) + "\n" for row in rows))
    print(f"wrote {len(rows)} unpatched scene completion captures")


if __name__ == "__main__":
    main()
