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
    parser.add_argument("--text-output", type=Path)
    parser.add_argument("--resources", type=Path)
    parser.add_argument("--audit", type=Path)
    parser.add_argument("--authored-text-output", type=Path)
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
    if args.text_output:
        text_rows = [run_text(executable, row) for row in rows]
        single = next(row for row in rows if row["name"] == "single_slot_0")
        text_rows.extend(run_text(executable, single, gate) for gate in
                         ["inactive", "subtitle", "menu", "shown", "wrong_record"])
        args.text_output.write_text("".join(json.dumps(row, sort_keys=True) + "\n" for row in text_rows))
        print(f"captured {len(text_rows)} complete native inventory A6 handlers")
    if args.authored_text_output:
        assert args.resources and args.audit
        authored_rows = []
        for profile in json.loads(args.audit.read_text()):
            if not profile["inventory_markers"]:
                continue
            images = {suffix: (args.resources / f"SCRIPT{profile['profile']}.{suffix}").read_bytes()
                      for suffix in ["COD", "VAR", "DIC", "DEB"]}
            inventory = None
            for cursor in range(0, len(images["DEB"]) - 19, 20):
                offset, kind = struct.unpack_from("<HH", images["DEB"], cursor + 16)
                if kind != 1:
                    break
                if offset and struct.unpack_from("<H", images["VAR"], offset)[0] == 0x400:
                    inventory = offset
                    break
            assert inventory is not None, profile["profile"]
            for occurrence in profile["inventory_markers"]:
                assert occurrence["flags"] == 0x8030 and occurrence["word_byte"] == 12
                condition = {"name": f"SCRIPT{profile['profile']}:{occurrence['token_byte']}",
                             "slots": [inventory] + [0] * 15, "kinds": []}
                row = run_text(executable, condition, authored=(images, occurrence["token_byte"], inventory))
                row["profile"] = profile["profile"]
                row["token_byte"] = occurrence["token_byte"]
                row["offered_object"] = inventory
                row["image_sha256"] = {suffix: hashlib.sha256(data).hexdigest() for suffix, data in images.items()}
                row["subtitle_sha256"] = hashlib.sha256(bytes(row.pop("subtitle"))).hexdigest()
                authored_rows.append(row)
        assert len(authored_rows) == 46
        args.authored_text_output.write_text("".join(json.dumps(row, sort_keys=True) + "\n" for row in authored_rows))
        print(f"captured {len(authored_rows)} original authored inventory A6 handlers")


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


def run_text(executable, condition, gate="none", authored=None):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x70000)
    cpu.mem_write(0, executable)
    dictionary_base = 0x60000
    source_origin = 0xFF if authored is None else authored[1]
    line_offset = 0x700 if authored is None else struct.unpack_from("<H", authored[0]["COD"], source_origin + 1)[0]
    before = bytearray(SIZE)
    data = executable[0xF7F0:]
    before[:len(data)] = data
    struct.pack_into("<H", before, STACK, RETURN)
    struct.pack_into("<HH", before, 0x6AEC, 0, STATE // 16)
    struct.pack_into("<HH", before, 0x6AFC, 0, dictionary_base // 16)
    struct.pack_into("<H", before, 0x6B94, 0x1234)
    struct.pack_into("<H", before, 0x6B36, 32)
    struct.pack_into("<H", before, 0x21F9, 85)
    struct.pack_into("<H", before, 0x6228, 0x2222)
    before[0x6B87] = 2
    before[0x6B8A] = 9
    before[0x6B8F] = 0
    before[0xF49] = 0
    before[0xF48] = 1
    before[0x6B86] = int(gate == "menu")
    before[0x6234] = int(gate == "subtitle")
    before[0x6B92] = 1
    before[0x6B80] = 0x40
    before[0x2201] = 1
    before[0x6BDC:0x6BDC + 34] = bytes(34)
    before[0x1066:0x106B] = b"OLD\r\0"
    struct.pack_into("<17H", before, 0x70E6, *condition["slots"], 65535)
    cpu.mem_write(GLOBALS, bytes(before))
    source = bytearray(SIZE)
    control = 0x30 if gate == "inactive" else 0x8030
    encoded = struct.pack("<BHbHH6H", 0xA6, line_offset, -3, control, 0x789,
                          32, 48, 64, 65535, 65534, 0)
    if authored is not None:
        assert len(authored[0]["COD"]) <= SIZE
        source[:len(authored[0]["COD"])] = authored[0]["COD"]
        encoded = bytes(source[source_origin:source_origin + 16])
        assert encoded[0] == 0xA6 and encoded[12:] == b"\xFE\xFF\0\0"
    source[source_origin:source_origin + len(encoded)] = encoded
    cpu.mem_write(SOURCE, bytes(source))
    state = bytearray(SIZE)
    for offset, kind in condition["kinds"]:
        struct.pack_into("<H", state, offset, kind)
    line_flags = 0x8020 if gate == "shown" else 0x20
    if authored is not None:
        assert len(authored[0]["VAR"]) <= SIZE
        state[:len(authored[0]["VAR"])] = authored[0]["VAR"]
        assert struct.unpack_from("<H", state, line_offset)[0] == 2
        line_flags = struct.unpack_from("<H", state, line_offset + 2)[0] & ~0x8000
        struct.pack_into("<H", state, authored[2] + 20, 65535)
    else:
        struct.pack_into("<H", state, line_offset, 2)
    struct.pack_into("<H", state, line_offset + 2, line_flags)
    action_offset = before[0x7128 + 19 * 16 + 1]
    action = 195 if gate == "wrong_record" else 196
    struct.pack_into("<H", state, line_offset + action_offset, action)
    cpu.mem_write(STATE, bytes(state))
    dictionary = bytearray(SIZE)
    for offset, word in [(32, b"CHOISISSEZ"), (48, b"UN"), (64, b"OBJET")]:
        dictionary[offset:offset + len(word) + 1] = word + b"\0"
    if authored is not None:
        assert len(authored[0]["DIC"]) <= SIZE
        dictionary = bytearray(SIZE)
        dictionary[:len(authored[0]["DIC"])] = authored[0]["DIC"]
    cpu.mem_write(dictionary_base, bytes(dictionary))
    registers = {
        UC_X86_REG_CS: 0, UC_X86_REG_DS: SOURCE // 16, UC_X86_REG_ES: STATE // 16,
        UC_X86_REG_GS: GLOBALS // 16, UC_X86_REG_SS: GLOBALS // 16,
        UC_X86_REG_SP: STACK, UC_X86_REG_SI: source_origin + 1, UC_X86_REG_DI: 0x333,
    }
    for register, value in registers.items():
        cpu.reg_write(register, value)
    cpu.reg_write(UC_X86_REG_EFLAGS, 2)
    global_outputs = [
        (0x6B4E, 2), (0x6B87, 1), (0x6B36, 2), (0x6B4A, 2),
        (0x21F9, 2), (0x6B8A, 1), (0x6B8F, 1), (0x6B94, 2),
        (0x6BDC, 34), (0xF49, 1), (0xF48, 1), (0x6B86, 1),
        (0x6234, 1), (0x6228, 2), (0x6B92, 1), (0x6B80, 1),
        (0x1066, 128), (STACK - 32, 32),
    ]
    writable = [(GLOBALS + start, length) for start, length in global_outputs]
    writable += [(SOURCE + source_origin + 5, 1), (STATE + line_offset + 2, 2)]
    visited = set()

    def instruction(_cpu, address, _size, _context):
        assert any(start <= address < end for start, end in
                   [(0x6B28, 0x6E0B), (0x6E4D, 0x6E67), (0x68A5, 0x68B5)]), hex(address)
        visited.add(address)

    def write(_cpu, _access, address, size, _value, _context):
        assert any(start <= address < address + size <= start + length
                   for start, length in writable), (hex(address), size)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start(0x6C89, RETURN, count=5000)
    assert cpu.reg_read(UC_X86_REG_IP) == RETURN and 0x6E53 in visited
    if gate == "none":
        assert {0x6B28, 0x6C45, 0x68A5}.issubset(visited)
    else:
        assert 0x6B28 not in visited
    registers[UC_X86_REG_SP] += 2
    registers[UC_X86_REG_SI] = source_origin + len(encoded)
    del registers[UC_X86_REG_ES]  # A6 deliberately retains its final working ES.
    for register, value in registers.items():
        assert cpu.reg_read(register) == value, (gate, register, cpu.reg_read(register), value)
    after = bytearray(cpu.mem_read(GLOBALS, SIZE))
    source_after = bytearray(cpu.mem_read(SOURCE, SIZE))
    state_after = bytearray(cpu.mem_read(STATE, SIZE))
    choices = []
    for offset in range(0x6BDC, 0x6BDC + 34, 2):
        value = struct.unpack_from("<H", after, offset)[0]
        if value == 0:
            break
        choices.append(value)
    else:
        raise AssertionError("missing choice terminator")
    result = {
        "condition": condition["name"], "gate": gate, "encoded": list(encoded),
        "line_flags_before": line_flags, "action": action,
        "control": struct.unpack_from("<H", source_after, source_origin + 4)[0],
        "line_flags": struct.unpack_from("<H", state_after, line_offset + 2)[0],
        "choices": choices, "resume": after[0x6B87],
        "resume_target": struct.unpack_from("<H", after, 0x6B4A)[0],
        "alternate": struct.unpack_from("<H", after, 0x6B36)[0],
        "saved_line": struct.unpack_from("<H", after, 0x6B94)[0],
        "selector": struct.unpack_from("<h", after, 0x21F9)[0],
        "yield": after[0x6B8A], "spoken": after[0x6B8F],
        "voice": after[0xF49], "chatter": after[0xF48],
        "menu_deferred": after[0x6B86], "subtitle_active": after[0x6234],
        "hold_ready": after[0x6B92], "request_flags": after[0x6B80],
        "subtitle_cursor": struct.unpack_from("<H", after, 0x6228)[0],
        "subtitle": list(after[0x1066:0x10E6].split(b"\0", 1)[0]),
    }
    for start, length in global_outputs:
        after[start:start + length] = before[start:start + length]
    source_after[source_origin + 5] = source[source_origin + 5]
    state_after[line_offset + 2:line_offset + 4] = state[line_offset + 2:line_offset + 4]
    assert after == before
    assert source_after == source
    assert state_after == state
    assert cpu.mem_read(dictionary_base, SIZE) == dictionary
    assert cpu.mem_read(0, len(executable)) == executable
    return result


if __name__ == "__main__":
    main()
