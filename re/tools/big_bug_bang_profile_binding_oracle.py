#!/usr/bin/env python3
"""Execute the sequel's VM resource-binding loop and resource allocator.

These probes run the original near/far calls without replacing native helpers.
They start with synthetic resident-handle tables, not a simulated game boot or
DOS filesystem. Output contains synthetic identities and sizes, not game bytes.
"""

import argparse
import hashlib
import itertools
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import (
    UC_X86_REG_AX, UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_ES,
    UC_X86_REG_FS, UC_X86_REG_GS, UC_X86_REG_SS, UC_X86_REG_SP,
    UC_X86_REG_SI, UC_X86_REG_EBP,
)

EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER_BYTES = 2048
GLOBAL_SEGMENT = 0x3000
CATALOG_SEGMENT = 0x4000
POOL_SEGMENT = 0x5000
STACK_TOP = 0xFF00
RESOURCE_IDS = [2, 3, 4, 5, 6]  # VAR, DEB, COD, BAS, DIC in native order.
PROFILE_HANDLES = 0x6AE2
PROFILE_BINDINGS = 0x6AEC
VM_CODE_SEGMENT = 0x502
ALLOCATOR_CODE_SEGMENT = 0x4E1
POOL_FREE_BYTES = 0x0C3E
POOL_END_SEGMENT = 0x0C62
RETURN_LINEAR = 0x200


def machine(executable):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 1048576)
    # Unrelocated MZ module at load segment zero, so its far calls remain real.
    cpu.mem_write(0, executable[HEADER_BYTES:])
    for register, value in [(UC_X86_REG_DS, GLOBAL_SEGMENT),
                            (UC_X86_REG_ES, GLOBAL_SEGMENT),
                            (UC_X86_REG_GS, GLOBAL_SEGMENT),
                            (UC_X86_REG_SS, GLOBAL_SEGMENT),
                            (UC_X86_REG_FS, CATALOG_SEGMENT),
                            (UC_X86_REG_SP, STACK_TOP)]:
        cpu.reg_write(register, value)
    return cpu


def execute(cpu, start_file, stop_linear):
    reached = []
    writes = []

    def instruction(machine, address, _size, _context):
        if address == stop_linear:
            reached.append(address)
            machine.emu_stop()

    def memory_write(_cpu, _access, address, size, _value, _context):
        writes.append((address, size))

    code_hook = cpu.hook_add(UC_HOOK_CODE, instruction)
    write_hook = cpu.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    try:
        cpu.emu_start(start_file - HEADER_BYTES, 0, count=1000)
    finally:
        cpu.hook_del(code_hook)
        cpu.hook_del(write_hook)
    assert reached == [stop_linear], reached
    return writes


def binding_case(executable, resident):
    cpu = machine(executable)
    globals_before = bytearray([0xA4] * 65536)
    struct.pack_into("<5H", globals_before, PROFILE_HANDLES, *RESOURCE_IDS)
    cpu.mem_write(GLOBAL_SEGMENT * 16, bytes(globals_before))
    catalog = bytearray(65536)
    segments = [POOL_SEGMENT + index * 256 for index in range(5)]
    for identity, present, segment in zip(RESOURCE_IDS, resident, segments):
        struct.pack_into("<HHI", catalog, identity * 8, segment, 3 if present else 0, 4096)
    cpu.mem_write(CATALOG_SEGMENT * 16, bytes(catalog))
    cpu.reg_write(UC_X86_REG_CS, VM_CODE_SEGMENT)
    writes = execute(cpu, 0x5A64, 0x5A99 - HEADER_BYTES)
    after = bytearray(cpu.mem_read(GLOBAL_SEGMENT * 16, 65536))
    pairs = [struct.unpack_from("<HH", after, PROFILE_BINDINGS + index * 4) for index in range(5)]
    assert all(offset == 0 for offset, _segment in pairs)
    owners = [None if segment == GLOBAL_SEGMENT else RESOURCE_IDS[segments.index(segment)]
              for _offset, segment in pairs]
    # The loop and resolver may write only five pointers and far-call stack.
    allowed = [(GLOBAL_SEGMENT * 16 + PROFILE_BINDINGS, 20),
               (GLOBAL_SEGMENT * 16 + STACK_TOP - 6, 6)]
    assert all(any(lo <= address and address + size <= lo + count for lo, count in allowed)
               for address, size in writes)
    after[PROFILE_BINDINGS:PROFILE_BINDINGS + 20] = globals_before[PROFILE_BINDINGS:PROFILE_BINDINGS + 20]
    after[STACK_TOP - 6:STACK_TOP] = globals_before[STACK_TOP - 6:STACK_TOP]
    assert after == globals_before
    assert bytes(cpu.mem_read(CATALOG_SEGMENT * 16, 65536)) == catalog
    assert cpu.reg_read(UC_X86_REG_SP) == STACK_TOP
    return {"name": "bindings_" + "".join(str(int(x)) for x in resident),
            "resident": list(resident), "resolved_resources": owners}


def allocation_case(executable, sizes):
    cpu = machine(executable)
    catalog = bytearray(65536)
    struct.pack_into("<H", catalog, 0x800, 65535)
    cpu.mem_write(CATALOG_SEGMENT * 16, bytes(catalog))
    cpu.mem_write(GLOBAL_SEGMENT * 16 + POOL_FREE_BYTES, struct.pack("<I", 65536))
    cpu.mem_write(GLOBAL_SEGMENT * 16 + POOL_END_SEGMENT, struct.pack("<H", POOL_SEGMENT))
    cpu.mem_write(GLOBAL_SEGMENT * 16 + STACK_TOP, struct.pack("<HH", RETURN_LINEAR, 0))
    allocations = []
    for index, size in enumerate(sizes):
        cpu.reg_write(UC_X86_REG_CS, ALLOCATOR_CODE_SEGMENT)
        cpu.reg_write(UC_X86_REG_AX, RESOURCE_IDS[index])
        cpu.reg_write(UC_X86_REG_EBP, size)
        cpu.reg_write(UC_X86_REG_SP, STACK_TOP)
        writes = execute(cpu, 0x5610, RETURN_LINEAR)
        allowed = [(GLOBAL_SEGMENT * 16 + POOL_FREE_BYTES, 4),
                   (GLOBAL_SEGMENT * 16 + POOL_END_SEGMENT, 2),
                   (GLOBAL_SEGMENT * 16 + STACK_TOP - 20, 20),
                   (CATALOG_SEGMENT * 16 + RESOURCE_IDS[index] * 8, 8),
                   (CATALOG_SEGMENT * 16 + 0x800, (index + 2) * 2),
                   (CATALOG_SEGMENT * 16 + 0xC00, 4)]
        assert all(any(lo <= address and address + count <= lo + length for lo, length in allowed)
                   for address, count in writes), writes
        assert cpu.reg_read(UC_X86_REG_AX) == 0
        assert cpu.reg_read(UC_X86_REG_SI) == 0
        assert cpu.reg_read(UC_X86_REG_EBP) == size
        assert cpu.reg_read(UC_X86_REG_SP) == STACK_TOP + 4
        segment, flags, allocated = struct.unpack("<HHI", cpu.mem_read(CATALOG_SEGMENT * 16 + RESOURCE_IDS[index] * 8, 8))
        assert flags == 3
        assert cpu.reg_read(UC_X86_REG_DS) == segment
        allocations.append({"start_byte": (segment - POOL_SEGMENT) * 16, "allocated_bytes": allocated})
    remaining, = struct.unpack("<I", cpu.mem_read(GLOBAL_SEGMENT * 16 + POOL_FREE_BYTES, 4))
    assert remaining == 65536 - sum(x["allocated_bytes"] for x in allocations)
    return {"name": "allocation_" + "_".join(map(str, sizes)), "requested_bytes": sizes,
            "allocations": allocations}


def vectors(executable):
    for resident in itertools.product([False, True], repeat=5):
        yield binding_case(executable, resident)
    for size in [1, 15, 16, 17, 8367, 8368, 8369, 8370]:
        yield allocation_case(executable, [size, 20, 33])


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
    print(f"wrote {len(results)} original binding/allocation cases")


if __name__ == "__main__":
    main()
