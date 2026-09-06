#!/usr/bin/env python3
"""Run complete inventory transfer and DESCRIPT lookup with DOS file boundaries.

All game instructions and entered helpers are unmodified. INT 21 file operations
read an owned in-memory database; no descriptor application result is substituted.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_INTR, UC_HOOK_MEM_WRITE
from unicorn.x86_const import *

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER = 0x800
CODE_FILE_BASE = 0x5820
GLOBALS = 0x30000
COD = 0x40000
VAR = 0x50000
SCRATCH = 0x60000
SIZE = 0x10000
STACK = 0xFF00
RETURN = 0x200
ITEM = 0x100
RECIPIENT = 0x200
SAVED_LINE = 0x104
RANGES = [(0x5C41, 0x5D5D), (0x65E8, 0x6606), (0x6633, 0x6644),
          (0x8450, 0x8560), (0x8584, 0x85A0), (0x8654, 0x866B),
          (0x86B1, 0x86C6), (0x2A13, 0x2A4F), (0x2B43, 0x2B69)]


def word(data, offset):
    return struct.unpack_from("<H", data, offset)[0]


def synthetic_database(name, payload):
    return struct.pack("<H16sHBH", 1, name, 21, 15, len(payload) + 2) + payload


def run(executable, name, database, database_missing=False, gate=0):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    before = bytearray(SIZE)
    native_data = executable[0xF7F0:]
    before[:len(native_data)] = native_data
    for offset, value in [(0x6AF6, COD // 16), (0x6AEE, VAR // 16),
                          (0x6B34, ITEM + 4), (0x6B36, 0x7654),
                          (0x6B94, SAVED_LINE), (0x6B5A, 0x1234)]:
        struct.pack_into("<H", before, offset, value)
    for offset, value in [(0x6B87, 2), (0x6B7E, 1), (0x2200, 1), (0x6B8D, 1),
                          (0x6B80, 2 if gate == 2 else 0), (0x2A33, 1 if gate == 1 else 0),
                          (0xCEA, 1), (0xCE9, 1), (0x7086, 0)]:
        before[offset] = value
    struct.pack_into("<HH", before, 0xCB4, 0, SCRATCH // 16)
    struct.pack_into("<HH", before, 0x6BDC, ITEM + 4, 0)
    struct.pack_into("<17H", before, 0x70E6, ITEM, 0, ITEM, *([0] * 13), 0xFFFF)
    struct.pack_into("<H", before, STACK, RETURN)
    before[0x2718:0x2726] = b"previous.hnm\0\0"
    before[0x1066:0x106D] = b"BEFORE\0"
    before[0x6234] = 0
    struct.pack_into("<H", before, 0x6228, 0x1234)
    cpu.mem_write(GLOBALS, bytes(before))
    cod = bytearray(SIZE)
    struct.pack_into("<H", cod, SAVED_LINE - 2, RECIPIENT)
    cod[SAVED_LINE + 2] = 0x21
    cpu.mem_write(COD, bytes(cod))
    var = bytearray(SIZE)
    struct.pack_into("<HH", var, ITEM, 0x400, 0x12)
    assert len(name) < 16
    var[ITEM + 4:ITEM + 4 + len(name)] = name
    struct.pack_into("<H", var, ITEM + 20, 0xFFFF)
    struct.pack_into("<H", var, RECIPIENT, 2)
    cpu.mem_write(VAR, bytes(var))
    registers = {UC_X86_REG_CS: (CODE_FILE_BASE - HEADER) // 16,
                 UC_X86_REG_DS: GLOBALS // 16, UC_X86_REG_GS: GLOBALS // 16,
                 UC_X86_REG_SS: GLOBALS // 16, UC_X86_REG_ES: VAR // 16,
                 UC_X86_REG_SP: STACK, UC_X86_REG_SI: 0x111, UC_X86_REG_DI: 0x222}
    for register, value in registers.items():
        cpu.reg_write(register, value)
    calls = []
    writes = set()
    io = []
    position = 0
    opened = False

    def cstring(address):
        return bytes(cpu.mem_read(address, 256)).split(b"\0", 1)[0]

    def instruction(_cpu, address, size, _context):
        address += HEADER
        assert any(start <= address and address + size <= end for start, end in RANGES), hex(address)
        if address in [0x5C41, 0x65E8, 0x6633, 0x8450, 0x86B1, 0x8654, 0x2A13, 0x2B43]:
            calls.append(address)

    def write(_cpu, _access, address, size, _value, _context):
        assert (GLOBALS <= address < address + size <= GLOBALS + SIZE
                or address == COD + SAVED_LINE + 2 and size == 1
                or address == VAR + ITEM + 2 and size == 1
                or address == VAR + ITEM + 20 and size == 2), (hex(address), size)
        if GLOBALS <= address < GLOBALS + SIZE:
            writes.update(range(address - GLOBALS, address - GLOBALS + size))

    def interrupt(_cpu, number, _context):
        nonlocal position, opened
        assert number == 0x21
        ax = cpu.reg_read(UC_X86_REG_AX)
        bx = cpu.reg_read(UC_X86_REG_BX)
        count = cpu.reg_read(UC_X86_REG_CX)
        dx = cpu.reg_read(UC_X86_REG_DX)
        destination = cpu.reg_read(UC_X86_REG_DS) * 16 + dx
        if ax == 0x3D00:
            assert cstring(destination) == b"descript.des"
            assert not opened
            opened = not database_missing
            cpu.reg_write(UC_X86_REG_AX, 5 if opened else 2)
            flags = cpu.reg_read(UC_X86_REG_EFLAGS)
            cpu.reg_write(UC_X86_REG_EFLAGS, flags & ~1 if opened else flags | 1)
            io.append(["open", int(opened)])
        elif ax == 0x3F00:
            assert opened and bx == 5
            assert (destination in [GLOBALS + 0xCA6, GLOBALS + 0xCA8] and count == 2
                    or destination == SCRATCH and count <= SIZE)
            data = database[position:position + count]
            cpu.mem_write(destination, data)
            position += len(data)
            cpu.reg_write(UC_X86_REG_AX, len(data))
            io.append(["read", count, len(data)])
        elif ax == 0x4200:
            assert opened and bx == 5 and count == 0
            position = dx
            cpu.reg_write(UC_X86_REG_AX, position)
            cpu.reg_write(UC_X86_REG_DX, 0)
            io.append(["seek", position])
        elif ax == 0x3E00:
            assert opened and bx == 5
            opened = False
            io.append(["close"])
        else:
            raise AssertionError(hex(ax))

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.hook_add(UC_HOOK_INTR, interrupt)
    cpu.emu_start(0x5C41 - HEADER, CODE_FILE_BASE - HEADER + RETURN, count=30000)
    assert cpu.reg_read(UC_X86_REG_IP) == RETURN
    assert not opened
    registers[UC_X86_REG_SP] += 2
    for register, value in registers.items():
        assert cpu.reg_read(register) == value, register
    after = bytes(cpu.mem_read(GLOBALS, SIZE))
    cod_after = bytearray(cpu.mem_read(COD, SIZE))
    var_after = bytearray(cpu.mem_read(VAR, SIZE))
    result = dict(name=list(name), database=list(database), database_missing=database_missing,
                  gate=gate, calls=calls, io=io, vm_enabled=after[0x6B7E],
                  c2_gate=after[0x2200], start_locked=after[0x6B8D],
                  request=after[0x6B80], active_line=word(after, 0x6B5A),
                  selected=word(after, 0x6B34), resume=after[0x6B87], alternate=word(after, 0x6B36),
                  saved_line=word(after, 0x6B94), choices_head=word(after, 0x6BDC),
                  slots=list(struct.unpack_from("<16H", after, 0x70E6)),
                  holder=word(var_after, ITEM + 20), object_flags=word(var_after, ITEM + 2),
                  line_flags=cod_after[SAVED_LINE + 2],
                  video=list(cstring(GLOBALS + 0x2718)), caption=list(cstring(GLOBALS + 0x1066)),
                  subtitle_active=after[0x6234], subtitle_cursor=word(after, 0x6228))
    allowed = [(0x6B34, 4), (0x6B94, 2), (0x6B87, 1), (0x6BDC, 2), (0x70E6, 32),
               (0x6B7E, 1), (0x2200, 1), (0x6B80, 1), (0x6B5A, 2), (STACK - 128, 128),
               (0x2A89, 1), (0x156C, 1), (0x21FB, 4), (0x1568, 2), (0x1166, 4),
               (0xD20, 1), (0xCEB, 1), (0xCA6, 4), (0x2718, 14),
               (0x1066, 256), (0x6234, 1), (0x6228, 2)]
    assert all(any(start <= address < start + length for start, length in allowed)
               for address in writes), sorted(hex(address) for address in writes
               if not any(start <= address < start + length for start, length in allowed))
    restored = bytearray(after)
    for start, length in allowed:
        restored[start:start + length] = before[start:start + length]
    assert restored == before
    cod_after[SAVED_LINE + 2] = cod[SAVED_LINE + 2]
    var_after[ITEM + 2:ITEM + 4] = var[ITEM + 2:ITEM + 4]
    var_after[ITEM + 20:ITEM + 22] = var[ITEM + 20:ITEM + 22]
    assert cod_after == cod and var_after == var
    assert cpu.mem_read(0, len(module)) == module
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--resources", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    rows = []
    for label, payload in [(b"objet", b"\x10objet.hnm\xff"),
                           (b"trait\x82", b"\x10traite.hnm\x0f"),
                           (b"empty", b"\xff"),
                           (b"caption", b"\x05CAPTION\0\x10objet.hnm\xff")]:
        database = synthetic_database(label, payload)
        for gate in [0, 1, 2]:
            rows.append(run(executable, label, database, gate=gate))
        rows.append(run(executable, label.upper(), database))
        rows.append(run(executable, label, database, database_missing=True))
    if args.resources:
        database = (args.resources / "DESCRIPT.DES").read_bytes()
        var = (args.resources / "SCRIPT1.VAR").read_bytes()
        directory = (args.resources / "SCRIPT1.DEB").read_bytes()
        for offset in range(0, len(directory), 20):
            _, record, kind = struct.unpack_from("<16sHH", directory, offset)
            if kind != 1:
                break
            if word(var, record) != 0x400:
                continue
            label = var[record + 4:record + 20].split(b"\0", 1)[0]
            row = run(executable, label, database)
            assert row["active_line"] == 43 and row["vm_enabled"] == 0
            row.pop("database")
            row["database_sha256"] = hashlib.sha256(database).hexdigest()
            row["authored"] = True
            rows.append(row)
    args.output.write_text("".join(json.dumps(row, separators=(",", ":")) + "\n" for row in rows))
    print(f"captured {len(rows)} complete inventory descriptor transfers")


if __name__ == "__main__":
    main()
