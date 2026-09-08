#!/usr/bin/env python3
"""Capture the original post-speech idle restart, without patched instructions.

Runs file 0x11D2..0x1321 with synthetic queue/text boundaries. This verifies
scheduler decisions, not decoder correctness or gameplay reachability.
"""

import argparse
import hashlib
import itertools
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_GS, UC_X86_REG_SS, UC_X86_REG_IP

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER = 0x800
GLOBALS = 0x30000
ENTRY = 0x11D2
STOP = 0x1321
FIELDS = {
    "active": (0x6B82, 1), "sequence": (0x277C, 1), "scene": (0x29DD, 1),
    "gate": (0x2200, 1), "entry": (0xFFD, 2), "read": (0xFAE, 2),
    "line": (0x6B5A, 2), "request": (0x6B80, 1), "countdown": (0xD3F, 2),
    "owner": (0x6B6C, 2), "subtitle": (0x6234, 1), "menu": (0x6B86, 1),
    "ready": (0x6B92, 1), "complete": (0x6B91, 1), "words": (0x6BDC, 2),
    "choice": (0x2A77, 1), "locked": (0x6B8D, 1), "vm": (0x6B7E, 1),
    "pending": (0x2201, 1), "selector": (0x21F9, 2), "chatter": (0xF48, 1),
    "word_offset": (0x6B1A, 2), "word_segment": (0x6B1C, 2),
}
WRITABLE_NAMES = ("line", "gate", "pending", "chatter", "word_offset", "word_segment",
                  "ready", "complete", "menu", "subtitle", "vm", "locked", "choice",
                  "owner", "request")
WRITABLE = {offset + i for name in WRITABLE_NAMES
            for offset, size in [FIELDS[name]] for i in range(size)}


def run(executable, name, overrides):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    initial = dict(active=1, sequence=1, scene=0, gate=0, entry=59, read=0,
                   line=0xFFFF, request=0, countdown=0, owner=0x6234, subtitle=1,
                   menu=0, ready=1, complete=0, words=1, choice=0, locked=1, vm=0,
                   pending=0, selector=0xFFFF, chatter=1, word_offset=0, word_segment=0)
    initial.update(overrides)
    before = bytearray(0x10000)
    for field, value in initial.items():
        offset, size = FIELDS[field]
        struct.pack_into("<B" if size == 1 else "<H", before, offset, value)
    cpu.mem_write(GLOBALS, bytes(before))
    cpu.reg_write(UC_X86_REG_CS, 0)
    for register in (UC_X86_REG_DS, UC_X86_REG_GS, UC_X86_REG_SS):
        cpu.reg_write(register, GLOBALS // 16)
    writes = []

    def instruction(_cpu, address, size, _context):
        assert ENTRY <= address + HEADER < address + HEADER + size <= STOP

    def write(_cpu, _access, address, size, value, _context):
        offset = address - GLOBALS
        assert all(i in WRITABLE for i in range(offset, offset + size)), hex(address)
        writes.append([offset, size, value])

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start(ENTRY - HEADER, STOP - HEADER, count=200)
    assert cpu.reg_read(UC_X86_REG_IP) == STOP - HEADER
    assert bytes(cpu.mem_read(0, len(module))) == module
    after = bytes(cpu.mem_read(GLOBALS, len(before)))
    assert all(a == b or i in WRITABLE for i, (a, b) in enumerate(zip(before, after)))
    output = {name: int.from_bytes(after[offset:offset + size], "little")
              for name, (offset, size) in FIELDS.items()}
    return dict(name=name, input=initial, output=output, writes=writes)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    cases = []
    for gate, line, counters, owner in itertools.product(
            (0, 1), (0xFFFF, 8, 12), ((0, 0), (59, 0), (59, 59), (0, 0xFFFF)),
            (0, 0x6234, 0x6B86)):
        cases.append((f"gate{gate}_line{line}_queue{counters}_owner{owner}",
                      dict(gate=gate, line=line, entry=counters[0], read=counters[1], owner=owner)))
    for name, overrides in [
        ("inactive", dict(active=0)), ("no_scene_or_sequence", dict(sequence=0)),
        ("contact", dict(sequence=0, scene=1)),
        ("primary_request", dict(request=1)), ("secondary_request", dict(request=2)),
        ("both_requests", dict(request=3)), ("pending_text", dict(request=1, pending=1, selector=4)),
        ("pending_text_idle", dict(request=1, pending=1)),
        ("countdown", dict(countdown=1, entry=0)),
        ("countdown_low_byte_zero", dict(countdown=256, entry=0)),
    ]:
        cases.append((name, overrides))
    for gate, scene in itertools.product((0, 1), repeat=2):
        cases.append((f"text_hold_gate{gate}_scene{scene}",
                      dict(gate=gate, scene=scene, ready=0, complete=1, countdown=1)))
    rows = [run(executable, name, values) for name, values in cases]
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("".join(json.dumps(row, sort_keys=True) + "\n" for row in rows))
    print(f"verified {len(rows)} unpatched idle scheduler cases")


if __name__ == "__main__":
    main()
