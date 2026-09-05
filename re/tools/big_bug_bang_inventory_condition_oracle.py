#!/usr/bin/env python3
"""Execute the original sequel A6 inventory condition and all entered helpers.

Synthetic roster and VAR records exercise the authored 0x8030 controls. No
callee is patched; this does not cover subsequent selection or presentation.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import (
    UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_ES, UC_X86_REG_GS,
    UC_X86_REG_SS, UC_X86_REG_SP, UC_X86_REG_SI, UC_X86_REG_DI,
    UC_X86_REG_BP, UC_X86_REG_BX, UC_X86_REG_CX, UC_X86_REG_AX,
    UC_X86_REG_IP, UC_X86_REG_EFLAGS,
)

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
GLOBALS = 0x30000
SOURCE = 0x40000
STATE = 0x50000
SIZE = 65536
STACK = 0xFF00
RETURN = 0x200
ENTRY = 0x6B28
RANGES = [(0x6B28, 0x6C89), (0x68A5, 0x68B5)]
OUTPUTS = [(0x6BDC, 34), (0x6B8A, 1), (0x6B87, 1),
           (0x6B8F, 1), (0x6B94, 2), (STACK - 18, 18)]


def run(executable, name, slots, kinds, flags=0x8030):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x70000)
    cpu.mem_write(0, executable)
    globals_before = bytearray(SIZE)
    data = executable[0xF7F0:]
    globals_before[:len(data)] = data
    struct.pack_into("<HH", globals_before, 0x6AEC, 0, STATE // 16)
    struct.pack_into("<H", globals_before, STACK, RETURN)
    struct.pack_into("<H", globals_before, 0x6B4E, 0x3456)
    struct.pack_into("<H", globals_before, 0x6B94, 0x1234)
    globals_before[0x6B87] = 2
    globals_before[0x6B8A] = 0
    globals_before[0x6B8F] = 0
    globals_before[0x6BDC:0x6BDC + 34] = b"\xA5" * 34
    assert len(slots) == 16
    struct.pack_into("<17H", globals_before, 0x70E6, *slots, 65535)
    cpu.mem_write(GLOBALS, bytes(globals_before))
    source = bytearray(SIZE)
    # The real scan helper advances bytewise to the section separator.
    struct.pack_into("<5H", source, 0x100, 32, 48, 65535, 65534, 0)
    cpu.mem_write(SOURCE, bytes(source))
    state = bytearray(SIZE)
    for offset, kind in kinds.items():
        struct.pack_into("<H", state, offset, kind)
    cpu.mem_write(STATE, bytes(state))
    registers = {
        UC_X86_REG_CS: 0, UC_X86_REG_DS: SOURCE // 16,
        UC_X86_REG_ES: STATE // 16, UC_X86_REG_GS: GLOBALS // 16,
        UC_X86_REG_SS: GLOBALS // 16, UC_X86_REG_SP: STACK,
        UC_X86_REG_SI: 0x100, UC_X86_REG_DI: 0x200,
        UC_X86_REG_BP: 0x4567, UC_X86_REG_BX: 0x6789,
        UC_X86_REG_CX: flags,
    }
    for register, value in registers.items():
        cpu.reg_write(register, value)
    cpu.reg_write(UC_X86_REG_EFLAGS, 3)
    visited = set()

    def instruction(_cpu, address, _size, _context):
        assert any(start <= address < end for start, end in RANGES), hex(address)
        visited.add(address)

    def write(_cpu, _access, address, size, _value, _context):
        assert any(GLOBALS + start <= address < address + size <= GLOBALS + start + length
                   for start, length in OUTPUTS), (hex(address), size)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start(ENTRY, RETURN, count=2000)
    assert cpu.reg_read(UC_X86_REG_IP) == RETURN
    assert {0x68A5, 0x6C45, 0x6C88}.issubset(visited)
    registers[UC_X86_REG_SP] += 2
    for register, value in registers.items():
        assert cpu.reg_read(register) == value, register
    after = bytearray(cpu.mem_read(GLOBALS, SIZE))
    choices = []
    for offset in range(0x6BDC, 0x6BDC + 34, 2):
        value = struct.unpack_from("<H", after, offset)[0]
        if value == 0:
            break
        choices.append(value)
    else:
        raise AssertionError("missing choice terminator")
    result = {
        "name": name, "slots": slots, "kinds": sorted(kinds.items()), "flags": flags,
        "choices": choices, "accepted": bool(cpu.reg_read(UC_X86_REG_EFLAGS) & 1),
        "resume": after[0x6B87], "yield": after[0x6B8A], "spoken": after[0x6B8F],
        "saved_line": struct.unpack_from("<H", after, 0x6B94)[0],
        "ax": cpu.reg_read(UC_X86_REG_AX),
    }
    for start, length in OUTPUTS:
        after[start:start + length] = globals_before[start:start + length]
    assert after == globals_before
    assert cpu.mem_read(SOURCE, SIZE) == source
    assert cpu.mem_read(STATE, SIZE) == state
    assert cpu.mem_read(0, len(executable)) == executable
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("selection_output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    cases = [("empty", [0] * 16, {})]
    for slot in range(16):
        slots = [0] * 16
        slots[slot] = 0x200
        cases.append((f"single_slot_{slot}", slots, {0x200: 0x400}))
    slots = [0x200 + index * 80 for index in range(16)]
    for name, kinds in [
        ("full", [0x400] * 16),
        ("no_inventory", [1, 2, 4, 8, 16, 32, 64, 128] * 2),
        ("mixed", [0x400 if index % 3 == 0 else 2 for index in range(16)]),
        ("kind_mask", [0x400 | (1 << index) for index in range(16)]),
    ]:
        cases.append((name, slots, dict(zip(slots, kinds))))
    cases.append(("duplicate_slots", [0x200, 0, 0x200] + [0] * 13, {0x200: 0x400}))
    rows = [run(executable, name, slots, kinds) for name, slots, kinds in cases]
    args.output.write_text("".join(json.dumps(row, sort_keys=True) + "\n" for row in rows))
    print(f"captured {len(rows)} complete native inventory conditions")
    selections = []
    for row in rows:
        for choice in row["choices"]:
            # Only valid single-kind inventory records enter the transfer tests.
            if dict(row["kinds"])[choice - 4] != 0x400:
                continue
            for audio_gate in ["global", "dialogue"]:
                selections.append(run_selection(executable, row, choice, audio_gate))
    args.selection_output.write_text("".join(json.dumps(row, sort_keys=True) + "\n" for row in selections))
    print(f"captured {len(selections)} complete native gated inventory selections")


def run_selection(executable, condition, choice, audio_gate):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x70000)
    cpu.mem_write(0, executable)
    before = bytearray(SIZE)
    data = executable[0xF7F0:]
    before[:len(data)] = data
    saved_line = condition["saved_line"]
    target = 0x780
    selected = choice - 4
    # Use the executable's real selector-17 field-matrix entry.
    holder_offset = before[0x7128 + 17 * 16 + 10]
    struct.pack_into("<H", before, STACK, RETURN)
    struct.pack_into("<H", before, 0x6AF6, SOURCE // 16)
    struct.pack_into("<H", before, 0x6AEE, STATE // 16)
    struct.pack_into("<H", before, 0x6B34, choice)
    struct.pack_into("<H", before, 0x6B36, 0x7654)
    struct.pack_into("<H", before, 0x6B94, saved_line)
    before[0x6B87] = condition["resume"]
    before[0x6B8A] = condition["yield"]
    before[0x6B8F] = condition["spoken"]
    before[0x2A33] = int(audio_gate == "global")
    before[0x6B80] = 2 * int(audio_gate == "dialogue")
    choices = condition["choices"]
    struct.pack_into(f"<{len(choices) + 1}H", before, 0x6BDC, *choices, 0)
    struct.pack_into("<17H", before, 0x70E6, *condition["slots"], 65535)
    cpu.mem_write(GLOBALS, bytes(before))
    source = bytearray(SIZE)
    struct.pack_into("<H", source, saved_line - 2, target)
    source[saved_line + 2] = 0x21
    cpu.mem_write(SOURCE, bytes(source))
    state = bytearray(SIZE)
    for offset, kind in condition["kinds"]:
        struct.pack_into("<H", state, offset, kind)
    struct.pack_into("<H", state, selected + holder_offset, 65535)
    state[selected + 2] = 0x12
    cpu.mem_write(STATE, bytes(state))
    preserved = {
        UC_X86_REG_CS: 0, UC_X86_REG_DS: GLOBALS // 16,
        UC_X86_REG_ES: STATE // 16, UC_X86_REG_GS: GLOBALS // 16,
        UC_X86_REG_SS: GLOBALS // 16, UC_X86_REG_SP: STACK,
        UC_X86_REG_SI: 0x111, UC_X86_REG_DI: 0x222, UC_X86_REG_CX: 0x9876,
    }
    for register, value in preserved.items():
        cpu.reg_write(register, value)
    cpu.reg_write(UC_X86_REG_EFLAGS, 2)
    global_outputs = [(0x6B34, 2), (0x6B36, 2), (0x6B94, 2), (0x6B87, 1),
                      (0x6BDC, 2), (0x70E6, 32), (STACK - 14, 14)]
    writable = [(GLOBALS + start, length) for start, length in global_outputs]
    writable += [(SOURCE + saved_line + 2, 1), (STATE + selected + 2, 1),
                 (STATE + selected + holder_offset, 2)]
    visited = set()

    def instruction(_cpu, address, _size, _context):
        assert any(start <= address < end for start, end in
                   [(0x5C41, 0x5D5D), (0x65E8, 0x6606), (0x6633, 0x6644)]), hex(address)
        visited.add(address)

    def write(_cpu, _access, address, size, _value, _context):
        assert any(start <= address < address + size <= start + length
                   for start, length in writable), (hex(address), size)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start(0x5C41, RETURN, count=1000)
    assert cpu.reg_read(UC_X86_REG_IP) == RETURN
    assert {0x65E8, 0x6633, 0x5D5C}.issubset(visited)
    assert 0x5CF0 not in visited and 0x5CCA not in visited
    preserved[UC_X86_REG_SP] += 2
    for register, value in preserved.items():
        assert cpu.reg_read(register) == value, register
    after = bytearray(cpu.mem_read(GLOBALS, SIZE))
    source_after = bytearray(cpu.mem_read(SOURCE, SIZE))
    state_after = bytearray(cpu.mem_read(STATE, SIZE))
    result = {
        "condition": condition["name"], "choice": choice, "audio_gate": audio_gate,
        "target": target, "holder_offset": holder_offset,
        "holder": struct.unpack_from("<H", state_after, selected + holder_offset)[0],
        "object_flags": state_after[selected + 2], "line_flags": source_after[saved_line + 2],
        "slots_before": condition["slots"],
        "slots_after": list(struct.unpack_from("<16H", after, 0x70E6)),
        "selected": struct.unpack_from("<H", after, 0x6B34)[0],
        "alternate": struct.unpack_from("<H", after, 0x6B36)[0],
        "saved_line": struct.unpack_from("<H", after, 0x6B94)[0],
        "resume": after[0x6B87], "yield": after[0x6B8A], "spoken": after[0x6B8F],
        "pending_head": struct.unpack_from("<H", after, 0x6BDC)[0],
    }
    for start, length in global_outputs:
        after[start:start + length] = before[start:start + length]
    source_after[saved_line + 2] = source[saved_line + 2]
    for start, length in [(selected + 2, 1), (selected + holder_offset, 2)]:
        state_after[start:start + length] = state[start:start + length]
    assert after == before
    assert source_after == source
    assert state_after == state
    assert cpu.mem_read(0, len(executable)) == executable
    return result


if __name__ == "__main__":
    main()
