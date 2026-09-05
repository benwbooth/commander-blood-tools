#!/usr/bin/env python3
"""Execute the sequel object chooser, list, transitions and entered font helpers.

Captures semantic frames, not planar VGA pixels. The original disabled-sound
gate avoids device playback; no code or callee is patched.
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
VAR = 0x40000
CHUNKY = 0x50000
SIZE = 0x10000
STACK = 0xFF00
RETURN = 0x200
ENTRY = 0x9B45
RANGES = [(0x9B45, 0x9C5E), (0x958A, 0x9778), (0x344D, 0x3486),
          (0x37A8, 0x38FC), (0x3F13, 0x4002), (0x20CE, 0x2142),
          (0xD05D, 0xD070), (0xD330, 0xD33A), (0x3486, 0x3512), (0x371E, 0x37A8)]


def word(data, offset):
    return struct.unpack_from("<H", data, offset)[0]


def run(executable, name, labels, selected, inventory=True):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    globals_data = bytearray(SIZE)
    native_data = executable[0xF7F0:]
    globals_data[:len(native_data)] = native_data
    for offset, value in [(0x6B94, 0x1234 if inventory else 0), (0x6B36, 0x5678), (0x6B34, 0),
                          (0x5605, 0), (0x5607, 319), (0x5609, 0), (0x560B, 199),
                          (0x0CBE, 225), (0x0C22, 0), (0x0C24, 0), (0x0C2C, 1)]:
        struct.pack_into("<H", globals_data, offset, value)
    for offset, value in [(0x6B82, 1), (0x2A77, 1), (0x6B80, 1), (0x6B90, 0),
                          (0x6B87, 2), (0x6B86, 1), (0x6234, 1), (0x6B91, 1),
                          (0x0CE7, 0), (0x0C36, 0), (0x2A33, 0), (0x6B7E, 1)]:
        globals_data[offset] = value
    struct.pack_into("<HH", globals_data, 0x6AEC, 0, VAR // 16)
    struct.pack_into("<HH", globals_data, 0x6AFC, 0, VAR // 16)
    struct.pack_into("<HH", globals_data, 0x55E9, 0, 0xA000)
    struct.pack_into("<HH", globals_data, 0x55ED, 0, 0xB000)
    struct.pack_into("<HH", globals_data, 0x55F1, 0, CHUNKY // 16)
    struct.pack_into("<4H", globals_data, 0x279F, 0, 100, 0, 0)
    var = bytearray(SIZE)
    choices = []
    for index, label in enumerate(labels):
        offset = 0x100 + index * 32
        choices.append(offset + 4)
        struct.pack_into("<H", var, offset, 0x400)
        assert len(label) < 16
        var[offset + 4:offset + 4 + len(label)] = label
    struct.pack_into(f"<{len(choices) + 1}H", globals_data, 0x6BDC, *choices, 0)
    cpu.mem_write(GLOBALS, bytes(globals_data))
    cpu.mem_write(VAR, bytes(var))
    frames = []
    draws = []
    backgrounds = []
    calls = []
    changed = set()

    def instruction(_cpu, address, _size, _context):
        file_address = address + HEADER
        assert any(start <= file_address < end for start, end in RANGES), hex(file_address)
        if file_address in [0x958A, 0x344D, 0x37A8, 0x3F13, 0x20CE, 0xD05D, 0x3486, 0x371E]:
            calls.append(file_address)
        if file_address in [0x37A8, 0x3486]:
            segment = cpu.reg_read(UC_X86_REG_DS)
            offset = cpu.reg_read(UC_X86_REG_SI)
            raw = bytes(cpu.mem_read(segment * 16 + offset, 256)).split(b"\0", 1)[0]
            draws.append({"text": list(raw), "x": cpu.reg_read(UC_X86_REG_BX),
                          "y": cpu.reg_read(UC_X86_REG_DX), "color": cpu.reg_read(UC_X86_REG_AX) & 255})
        if file_address in [0x3F13, 0x371E]:
            backgrounds.append([cpu.reg_read(register) for register in
                                [UC_X86_REG_BX, UC_X86_REG_CX, UC_X86_REG_DX, UC_X86_REG_BP]])

    def write(_cpu, _access, address, size, _value, _context):
        assert (GLOBALS <= address < address + size <= GLOBALS + SIZE
                or 0xA0000 <= address < address + size <= 0xB0000
                or CHUNKY <= address < address + size <= CHUNKY + SIZE), (hex(address), size)
        if GLOBALS <= address < GLOBALS + SIZE:
            changed.update(range(address - GLOBALS, address - GLOBALS + size))

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    clicked = False
    for frame in range(14):
        before = bytes(cpu.mem_read(GLOBALS, SIZE))
        phase = before[0x6B90]
        # Wait until a complete opening update has run before clicking a row.
        if phase == 2 and selected is not None and not clicked:
            # Hit rows begin four pixels below the cancel-enabled rectangle top.
            height = (18 if inventory else 8) + len(labels) * 11
            y = ((200 - height) // 2) + selected * 11 + 6
            cpu.mem_write(GLOBALS + 0x0C22, struct.pack("<HH", 225, y))
            cpu.mem_write(GLOBALS + 0x0C36, b"\1")
            clicked = True
        else:
            cpu.mem_write(GLOBALS + 0x0C36, b"\0")
        cpu.mem_write(GLOBALS + STACK, struct.pack("<HH", RETURN, 0))
        registers = {UC_X86_REG_CS: 0x803, UC_X86_REG_DS: GLOBALS // 16,
                     UC_X86_REG_GS: GLOBALS // 16, UC_X86_REG_ES: 0x7000,
                     UC_X86_REG_SS: GLOBALS // 16, UC_X86_REG_SP: STACK,
                     UC_X86_REG_AX: 111, UC_X86_REG_BX: 222, UC_X86_REG_CX: 333,
                     UC_X86_REG_DX: 444, UC_X86_REG_SI: 555, UC_X86_REG_DI: 666,
                     UC_X86_REG_BP: 777}
        for register, value in registers.items():
            cpu.reg_write(register, value)
        cpu.reg_write(UC_X86_REG_EFLAGS, 2)
        draws.clear()
        backgrounds.clear()
        calls.clear()
        cpu.emu_start(ENTRY - HEADER, RETURN, count=1000000)
        assert cpu.reg_read(UC_X86_REG_IP) == RETURN
        registers[UC_X86_REG_CS] = 0
        registers[UC_X86_REG_SP] += 4
        for register, value in registers.items():
            assert cpu.reg_read(register) == value, (name, frame, register)
        after = bytes(cpu.mem_read(GLOBALS, SIZE))
        frames.append({"phase_before": phase, "phase": after[0x6B90],
                       "active": after[0x2A77], "resume": after[0x6B87],
                       "saved_line": word(after, 0x6B94), "alternate": word(after, 0x6B36),
                       "selected": word(after, 0x6B34), "pending_selection": word(after, 0x6B68),
                       "choices_head": word(after, 0x6BDC), "cancel": after[0xCE6],
                       "rect": list(struct.unpack_from("<4H", after, 0x2D4B)),
                       "target": list(struct.unpack_from("<4H", after, 0x279F)),
                       "step": after[0xCE4], "request": after[0x6B80],
                       "deferred": after[0x6B86], "subtitle": after[0x6234],
                       "hold": after[0x6B91], "ui": after[0x2A33],
                       "vm_enabled": after[0x6B7E] != 0,
                       "draws": list(draws), "backgrounds": list(backgrounds), "calls": list(calls),
                       "pointer": list(struct.unpack_from("<2H", after, 0xC22)),
                       "pressed": after[0xC36] != 0})
        if after[0x2A77] == 0 or (phase == 2 and selected is None):
            break
    assert cpu.mem_read(VAR, SIZE) == var
    assert cpu.mem_read(0, len(module)) == module
    if selected is not None:
        assert frames[-1]["active"] == 0, name
        assert frames[-1]["selected"] == (choices[selected] if selected < len(choices) else 0), name
        assert frames[-1]["saved_line"] == (0x1234 if inventory and selected < len(choices) else 0), name
    allowed = [(0x6B90, 2), (0x6B7E, 1), (0x2A33, 1), (0xCE3, 4), (0xCBE, 2),
               # The original REP STOSW uses the width-array byte span as CX.
               (0x2A87, 2), (0x2D4B, 8 + 4 * (len(labels) + 1)),
               (0x279F, 8), (0x2A67, 1), (0x2A6D, 2), (0x6B68, 2),
               (0xC2A, 4), (0x6B34, 4), (0x6B94, 2), (0x6B87, 1),
               (0x2A77, 1), (0x6B86, 1), (0x6234, 1), (0x6B80, 1),
               (0x6BDC, 2), (STACK - 128, 128)]
    assert all(any(start <= address < start + length for start, length in allowed)
               for address in changed), sorted(hex(address) for address in changed
               if not any(start <= address < start + length for start, length in allowed))
    return {"name": name, "inventory": inventory, "labels": [list(label) for label in labels],
            "selection": selected, "frames": frames}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--order-output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    rows = [run(executable, name, labels, selected) for name, labels, selected in [
        ("one_wait", [b"OBJET"], None),
        ("one_select", [b"OBJET"], 0),
        ("one_cancel", [b"OBJET"], 1),
        ("two_second", [b"CLE", b"CARTE"], 1),
        ("two_cancel", [b"CLE", b"CARTE"], 2),
        ("wide_choice", [b"ABCDEFGHIJKLMNO", b"CLE"], 0),
        ("accented_choice", [b"trait\x82"], 0),
        ("full_roster_last", [f"OBJET{i:02}".encode() for i in range(16)], 15),
        ("full_roster_cancel", [f"OBJET{i:02}".encode() for i in range(16)], 16),
    ]]
    rows += [run(executable, "dictionary_select", [b"OUI", b"NON"], 1, inventory=False),
             run(executable, "dictionary_wait", [b"OUI", b"NON"], None, inventory=False)]
    args.output.write_text("".join(json.dumps(row, sort_keys=True) + "\n" for row in rows))
    print(f"captured {len(rows)} complete native inventory chooser sequences")
    if args.order_output:
        order = [run_order(executable, presented, inventory)
                 for presented in [False, True] for inventory in [False, True]]
        args.order_output.write_text("".join(json.dumps(row, sort_keys=True) + "\n" for row in order))
        print("captured 4 native chooser/base-frame call-order cases")


def run_order(executable, presented, inventory):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    before = bytearray(SIZE)
    before[0x1006] = int(presented)
    struct.pack_into("<H", before, 0x6B94, 0x1234 if inventory else 0)
    struct.pack_into("<HH", before, 0x55E9, 0, 0xA000)
    struct.pack_into("<HH", before, 0x55F1, 0, VAR // 16)
    cpu.mem_write(GLOBALS, bytes(before))
    source = bytes(range(256)) * 256
    cpu.mem_write(VAR, source)
    for register, value in {UC_X86_REG_CS: 0, UC_X86_REG_DS: GLOBALS // 16,
                            UC_X86_REG_GS: GLOBALS // 16, UC_X86_REG_SS: GLOBALS // 16,
                            UC_X86_REG_SP: STACK}.items():
        cpu.reg_write(register, value)
    events = []

    def instruction(_cpu, address, _size, _context):
        offset = address + HEADER
        assert any(start <= offset < end for start, end in
                   [(0x1384, 0x13B0), (0x9B45, 0x9B52), (0x9C59, 0x9C5E),
                    (0x434B, 0x43E4)]), hex(offset)
        if offset in [0x9B45, 0x434B]:
            events.append("chooser" if offset == 0x9B45 else "submit")

    def write(_cpu, _access, address, size, _value, _context):
        assert (GLOBALS + STACK - 64 <= address < address + size <= GLOBALS + STACK
                or 0xA0000 <= address < address + size <= 0xB0000), (hex(address), size)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start(0x1384 - HEADER, 0x13B0 - HEADER, count=300000)
    assert cpu.reg_read(UC_X86_REG_IP) == 0x13B0 - HEADER
    assert cpu.reg_read(UC_X86_REG_SP) == STACK
    after = bytearray(cpu.mem_read(GLOBALS, SIZE))
    after[STACK - 64:STACK] = before[STACK - 64:STACK]
    assert after == before
    assert cpu.mem_read(VAR, SIZE) == source
    assert cpu.mem_read(0, len(module)) == module
    return {"presented": presented, "inventory": inventory, "events": events}


if __name__ == "__main__":
    main()
