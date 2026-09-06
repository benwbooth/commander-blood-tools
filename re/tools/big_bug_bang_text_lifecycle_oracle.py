#!/usr/bin/env python3
"""Observe the original text-hold coordinator's VM-resume write.

Runs unmodified BLOOD2PG instructions from file 0x11D2 to 0x1280 with explicit
synthetic pre-frame state. This is boundary evidence, not gameplay reachability.
"""

import argparse
import hashlib
import itertools
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_GS, UC_X86_REG_SS, UC_X86_REG_SP

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER = 0x800
GLOBALS = 0x30000
ENTRY = 0x11D2 - HEADER
STOP = 0x1280 - HEADER
FIELDS = {
    "active": (0x6B82, 1), "menu": (0x6B86, 1), "subtitle": (0x6234, 1),
    "ready": (0x6B92, 1), "complete": (0x6B91, 1), "words": (0x6BDC, 2),
    "countdown": (0xD3F, 2), "secondary": (0xC37, 1), "queue": (0x2200, 1),
    "vm": (0x6B7E, 1), "locked": (0x6B8D, 1), "choice": (0x2A77, 1),
    "owner": (0x6B6C, 2), "request": (0x6B80, 1),
}
WRITABLE = {byte for name in ("ready", "complete", "menu", "subtitle", "vm", "locked",
                              "choice", "owner", "request")
            for byte in range(FIELDS[name][0], sum(FIELDS[name]))}


def run(executable, values):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    cpu.mem_write(0, executable[HEADER:])
    before = bytearray(65536)
    native_data = executable[0xF7F0:]
    before[:len(native_data)] = native_data
    initial = dict(values, locked=1, choice=0, owner=0, request=3)
    for name, value in initial.items():
        offset, size = FIELDS[name]
        struct.pack_into("<B" if size == 1 else "<H", before, offset, value)
    cpu.mem_write(GLOBALS, bytes(before))
    cpu.reg_write(UC_X86_REG_CS, 0)
    for register in (UC_X86_REG_DS, UC_X86_REG_GS, UC_X86_REG_SS):
        cpu.reg_write(register, GLOBALS // 16)
    cpu.reg_write(UC_X86_REG_SP, 0xFF00)
    ended = False
    writes = []

    def code_hook(uc, address, _size, _data):
        nonlocal ended
        if address == STOP:
            ended = True
            uc.emu_stop()
        elif not ENTRY <= address < STOP:
            raise AssertionError(f"coordinator escaped at {address + HEADER:#x}")

    def write_hook(_uc, _access, address, size, value, _data):
        offset = address - GLOBALS
        if any(byte not in WRITABLE for byte in range(offset, offset + size)):
            raise AssertionError(f"unexpected write {address:#x}/{size}")
        writes.append({"offset": offset, "size": size, "value": value})

    cpu.hook_add(UC_HOOK_CODE, code_hook)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.emu_start(ENTRY, STOP + 1, count=200)
    if not ended:
        raise AssertionError("coordinator did not reach the post-hold boundary")
    after = bytes(cpu.mem_read(GLOBALS, 65536))
    if bytes(cpu.mem_read(0, len(executable) - HEADER)) != executable[HEADER:]:
        raise AssertionError("native module was modified")
    if any(left != right and index not in WRITABLE
           for index, (left, right) in enumerate(zip(before, after))):
        raise AssertionError("unaccounted global mutation")
    output = {name: struct.unpack_from("<B" if size == 1 else "<H", after, offset)[0]
              for name, (offset, size) in FIELDS.items()}
    return {"input": initial, "output": output, "writes": writes}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    if hashlib.sha256(executable).hexdigest() != SHA256:
        raise ValueError("unrecognized BLOOD2PG executable")
    names = ("active", "menu", "subtitle", "ready", "complete", "words", "countdown",
             "secondary", "queue", "vm")
    domains = [(0, 1)] * 6 + [(0, 1, 255, 256)] + [(0, 1)] * 3
    count = 0
    with args.output.open("x") as output:
        for values in itertools.product(*domains):
            vector = run(executable, dict(zip(names, values)))
            output.write(json.dumps(vector, sort_keys=True, separators=(",", ":")) + "\n")
            count += 1
    print(f"verified {count} native text-hold coordinator cases")


if __name__ == "__main__":
    main()
