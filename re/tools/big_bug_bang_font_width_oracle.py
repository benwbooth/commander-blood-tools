#!/usr/bin/env python3
"""Execute the original Big Bug Bang dual-face text-width routine.

The fixed-offset routine is the complete procedure at file 0x344d..0x3485.
It selects the square-caps tables when AX is zero and the main-font tables
otherwise, reads the NUL-terminated DS:SI string, and returns the 16-bit
accumulator minus two in AX. Unicorn executes the original bytes offline;
no native call or helper is replaced.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_INTR, UC_HOOK_MEM_WRITE, UC_MODE_16
from unicorn.x86_const import (
    UC_X86_REG_CS,
    UC_X86_REG_DS,
    UC_X86_REG_ES,
    UC_X86_REG_EAX,
    UC_X86_REG_EBP,
    UC_X86_REG_EBX,
    UC_X86_REG_ECX,
    UC_X86_REG_EDI,
    UC_X86_REG_EDX,
    UC_X86_REG_ESI,
    UC_X86_REG_ESP,
    UC_X86_REG_FS,
    UC_X86_REG_GS,
    UC_X86_REG_IP,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)


EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
WIDTH_START = 0x344D
WIDTH_END = 0x3486
DATA_FILE_START = 0xF7F0
SQUARE_MAP = 0x7CF8
SQUARE_ADVANCES = 0x7DE0
MAIN_MAP = 0x81E6
MAIN_ADVANCES = 0x82CE
TABLE_SIZE = 256
REQUIRED_DATA_END = max(
    SQUARE_MAP + TABLE_SIZE,
    SQUARE_ADVANCES + TABLE_SIZE,
    MAIN_MAP + TABLE_SIZE,
    MAIN_ADVANCES + TABLE_SIZE,
)

RETURN_IP = 0x0200
CODE_SEGMENT = 0
DATA_SEGMENT = 0x7000
SOURCE_SEGMENT = 0x2000
STACK_SEGMENT = 0x6000
SOURCE_OFFSET = 0x0400
STACK_POINTER = 0xFFF0
SOURCE_REGION_SIZE = 0x5000

GPR_NAMES = ("eax", "ebx", "ecx", "edx", "esi", "edi", "ebp", "esp")
GPR_REGS = {
    "eax": UC_X86_REG_EAX,
    "ebx": UC_X86_REG_EBX,
    "ecx": UC_X86_REG_ECX,
    "edx": UC_X86_REG_EDX,
    "esi": UC_X86_REG_ESI,
    "edi": UC_X86_REG_EDI,
    "ebp": UC_X86_REG_EBP,
    "esp": UC_X86_REG_ESP,
}
SEGMENT_REGS = {
    "cs": UC_X86_REG_CS,
    "ds": UC_X86_REG_DS,
    "es": UC_X86_REG_ES,
    "fs": UC_X86_REG_FS,
    "gs": UC_X86_REG_GS,
    "ss": UC_X86_REG_SS,
}


def run(executable, name, face, text):
    if face not in ("square_caps", "main"):
        raise ValueError(face)
    if any(byte < 0 or byte >= 232 for byte in text):
        raise ValueError(f"{name}: input outside supported byte domain 0..231")

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x100000)
    machine.mem_write(0, executable)

    native_data = executable[DATA_FILE_START:]
    if len(native_data) < REQUIRED_DATA_END:
        raise ValueError("BLOOD2PG.EXE is too short for the native data image")
    machine.mem_write(DATA_SEGMENT * 16, native_data)

    source_before = bytearray((index * 29 + 7) & 0xFF for index in range(SOURCE_REGION_SIZE))
    source_before[SOURCE_OFFSET:SOURCE_OFFSET + len(text)] = bytes(text)
    source_before[SOURCE_OFFSET + len(text)] = 0
    machine.mem_write(SOURCE_SEGMENT * 16, bytes(source_before))

    return_cs = CODE_SEGMENT
    machine.mem_write(STACK_SEGMENT * 16 + STACK_POINTER,
                      struct.pack("<HH", RETURN_IP, return_cs))

    selector = 0 if face == "square_caps" else 1
    initial = {
        "eax": 0xCAFE0000 | selector,
        "ebx": 0xA1B20000 | 0x1357,
        "ecx": 0xC3D40000 | 0x2468,
        "edx": 0xE5F60000 | 0x369A,
        "esi": SOURCE_OFFSET,
        "edi": 0x789A,
        "ebp": 0xBCDE,
        "esp": STACK_POINTER,
    }
    for register, value in initial.items():
        machine.reg_write(GPR_REGS[register], value)
    for register, value in {
        "cs": CODE_SEGMENT,
        "ds": SOURCE_SEGMENT,
        "es": 0x3000,
        "fs": 0x4000,
        "gs": DATA_SEGMENT,
        "ss": STACK_SEGMENT,
    }.items():
        machine.reg_write(SEGMENT_REGS[register], value)

    executed = set()
    writes = []
    interrupts = []

    def code_hook(_cpu, address, _size, _context):
        executed.add(address)

    def write_hook(_cpu, _access, address, size, _value, _context):
        writes.append((address, size))

    def interrupt_hook(cpu, number, _context):
        interrupts.append(number)
        cpu.emu_stop()

    machine.hook_add(UC_HOOK_CODE, code_hook)
    machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    machine.hook_add(UC_HOOK_INTR, interrupt_hook)
    instruction_limit = max(10000, (len(text) + 1) * 20)
    machine.emu_start(WIDTH_START, RETURN_IP, count=instruction_limit)

    if interrupts:
        raise AssertionError(f"{name}: unexpected interrupt {interrupts}")
    if machine.reg_read(SEGMENT_REGS["cs"]) != return_cs:
        raise AssertionError(f"{name}: far return did not restore CS")
    return_stack_pointer = STACK_POINTER + 4
    if machine.reg_read(UC_X86_REG_SP) != return_stack_pointer:
        raise AssertionError(f"{name}: stack pointer not restored")
    if machine.reg_read(UC_X86_REG_IP) != RETURN_IP:
        raise AssertionError(f"{name}: routine did not return")
    if not executed or not executed.issubset(range(WIDTH_START, WIDTH_END)):
        raise AssertionError(f"{name}: execution escaped width routine: {sorted(executed)}")

    expected_eax = ((sum_width(executable, face, text)) - 2) & 0xFFFF
    actual_eax = machine.reg_read(UC_X86_REG_EAX)
    expected = dict(initial)
    expected["eax"] = expected_eax
    expected["esp"] = return_stack_pointer
    for register in GPR_NAMES:
        actual = machine.reg_read(GPR_REGS[register])
        if actual != expected[register]:
            raise AssertionError(f"{name}: {register} changed to {actual:#x}, expected {expected[register]:#x}")
    for register in ("ds", "es", "fs", "gs", "ss"):
        expected_segment = {
            "ds": SOURCE_SEGMENT,
            "es": 0x3000,
            "fs": 0x4000,
            "gs": DATA_SEGMENT,
            "ss": STACK_SEGMENT,
        }[register]
        if machine.reg_read(SEGMENT_REGS[register]) != expected_segment:
            raise AssertionError(f"{name}: {register} changed")

    if machine.mem_read(SOURCE_SEGMENT * 16, SOURCE_REGION_SIZE) != bytes(source_before):
        raise AssertionError(f"{name}: source fixture was modified")
    if machine.mem_read(DATA_SEGMENT * 16, len(native_data)) != native_data:
        raise AssertionError(f"{name}: native data image was modified")
    allowed_stack = range(STACK_SEGMENT * 16 + STACK_POINTER - 8,
                          STACK_SEGMENT * 16 + STACK_POINTER)
    if any(address not in allowed_stack or address + size > allowed_stack.stop
           or size not in (1, 2) for address, size in writes):
        raise AssertionError(f"{name}: unexpected memory writes {writes}")

    return {"name": name, "face": face, "text": list(text), "width": actual_eax}


def sum_width(executable, face, text):
    """Compute only the expected 16-bit arithmetic from the native tables."""
    map_offset, advances_offset = ((SQUARE_MAP, SQUARE_ADVANCES)
                                   if face == "square_caps" else (MAIN_MAP, MAIN_ADVANCES))
    mapping = executable[DATA_FILE_START + map_offset:DATA_FILE_START + map_offset + TABLE_SIZE]
    advances = executable[DATA_FILE_START + advances_offset:DATA_FILE_START + advances_offset + TABLE_SIZE]
    if len(mapping) != TABLE_SIZE or len(advances) != TABLE_SIZE:
        raise ValueError("native font table is truncated")
    total = 0
    for byte in text:
        if byte == 0:
            break
        total = (total + advances[mapping[byte]]) & 0xFFFF
    return total


def cases():
    faces = (("square_caps", 0), ("main", 1))
    for face, _selector in faces:
        for byte in range(1, 232):
            yield f"{face}_char_{byte:03d}", face, [byte]
        yield f"{face}_empty", face, []
        yield f"{face}_embedded_nul", face, [ord("A"), 0, ord("B")]
        yield f"{face}_leading_nul", face, [0, ord("A")]

        ordinary = [
            b"BIG BUG BANG",
            b"Commander Blood 2",
            b"WIDTH TEST: 0123456789",
            b"The quick brown fox jumps over the lazy dog.",
            b"SAVE SLOT 01 / SAVE SLOT 02",
        ]
        for index, text in enumerate(ordinary):
            yield f"{face}_ordinary_{index}", face, list(text)

        french = [
            [0x82, 0x83, 0x84, 0x85, 0x87, 0x88, 0x89, 0x8A, 0x8B, 0x8C, 0x8D, 0x8E],
            [0x90, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A],
            list(b"FRANCAIS ") + [0x82, 0x85, 0x87, 0x8A, 0x8E, 0x90, 0x94, 0x97],
        ]
        for index, text in enumerate(french):
            yield f"{face}_french_extended_{index}", face, text

        long_texts = [
            list((b"ABCDEFGHIJKLMNOPQRSTUVWXYZ" * 40)),
            list((b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 " * 64)),
            [32] * 4096,
            list((b"BIG BUG BANG " * 1024)),
        ]
        for index, text in enumerate(long_texts):
            yield f"{face}_long_wrap_{index}", face, text


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    executable = args.executable.read_bytes()
    if hashlib.sha256(executable).hexdigest() != EXECUTABLE_SHA256:
        raise SystemExit("unsupported BLOOD2PG.EXE build; refusing fixed-offset oracle")
    if len(executable) < DATA_FILE_START + REQUIRED_DATA_END:
        raise SystemExit("BLOOD2PG.EXE does not contain the required native data image")

    results = []
    for name, face, text in cases():
        results.append(run(executable, name, face, text))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("".join(json.dumps(item, separators=(",", ":")) + "\n" for item in results))

    table_info = []
    for face, map_offset, advances_offset in (
        ("square_caps", SQUARE_MAP, SQUARE_ADVANCES),
        ("main", MAIN_MAP, MAIN_ADVANCES),
    ):
        mapping = executable[DATA_FILE_START + map_offset:DATA_FILE_START + map_offset + TABLE_SIZE]
        advances = executable[DATA_FILE_START + advances_offset:DATA_FILE_START + advances_offset + TABLE_SIZE]
        table_info.append(f"{face} map={hashlib.sha256(mapping).hexdigest()[:12]} advances={hashlib.sha256(advances).hexdigest()[:12]}")
    print(f"wrote {len(results)} original font-width vectors")
    print("; ".join(table_info))


if __name__ == "__main__":
    main()
