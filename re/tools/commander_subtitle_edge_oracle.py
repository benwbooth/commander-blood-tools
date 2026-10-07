#!/usr/bin/env python3
"""Observe original subtitle writes across the visible right edge, with real fonts."""

import os
import sys

# This directory's dis.py is a reverse-engineering tool, not Python's module.
_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [p for p in sys.path if os.path.abspath(p or os.curdir) != _HERE]

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_INSN, UC_HOOK_MEM_WRITE
from unicorn.x86_const import (
    UC_X86_INS_OUT, UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_ES, UC_X86_REG_GS,
    UC_X86_REG_SS, UC_X86_REG_SP, UC_X86_REG_SI, UC_X86_REG_BX, UC_X86_REG_DX,
    UC_X86_REG_EFLAGS,
)


ROOT = Path(__file__).resolve().parents[2]
TEXT, GLOBALS, OUTPUT, STACK = 0x20000, 0x40000, 0x60000, 0x90000
RETURN = 0x6FC0
GAMES = {
    "cb": dict(path="re/bin/BLOODPRG.EXE", entry=0x3630, data=0xD420, pointer=0x5219,
               cursor=0x5E58, mapping=0x70FA, glyphs=0x71AA,
               executable="7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823",
               routine="92b974665ae76fd5b36a0b4e503813b7aa8cf7f601076e459ceb0c4e0a5f3a21"),
    "bbb": dict(path="output/big-bug-bang/disc/BLOOD2PG.EXE", entry=0x39BE, data=0xF7F0, pointer=0x55E9,
                cursor=0x6228, mapping=0x7A00, glyphs=0x7AE8,
                executable="4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834",
                routine="e27d2ccbb3e251a936381ba92a13910153c6da7dfd5da6db5a50f3db635d3c09"),
}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def run(game, config, executable, text, x, y, cursor, name):
    assert text.endswith(b"\r") and len(text) <= 257 and text.count(b"\r") == 1
    entry = config["entry"]
    assert sha(executable) == config["executable"]
    assert sha(executable[entry:entry + 186]) == config["routine"]
    data = executable[config["data"]:config["data"] + 65536].ljust(65536, b"\0")
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    cpu.mem_write(0, executable)
    cpu.mem_write(TEXT, text)
    cpu.mem_write(GLOBALS, data)
    cpu.mem_write(STACK, data)
    cpu.mem_write(GLOBALS + config["pointer"], struct.pack("<HH", 0, OUTPUT // 16))
    cpu.mem_write(GLOBALS + config["cursor"], struct.pack("<H", cursor))
    cpu.mem_write(STACK + 0xFF00, struct.pack("<HH", RETURN, 0))
    for reg, value in ((UC_X86_REG_CS, 0), (UC_X86_REG_DS, TEXT // 16),
                       (UC_X86_REG_ES, OUTPUT // 16), (UC_X86_REG_GS, GLOBALS // 16),
                       (UC_X86_REG_SS, STACK // 16), (UC_X86_REG_SP, 0xFF00),
                       (UC_X86_REG_SI, 0), (UC_X86_REG_BX, x), (UC_X86_REG_DX, y),
                       (UC_X86_REG_EFLAGS, 2)):
        cpu.reg_write(reg, value)
    state = dict(mask=None, returned=False)
    pixels, ports, writes = bytearray(320 * 200), [], []

    def instruction(machine, address, size, _context):
        if address == RETURN:
            state["returned"] = True
            machine.emu_stop()
            return
        assert entry <= address and address + size <= entry + 186, hex(address)

    def port(_machine, address, size, value, _context):
        assert size == 1 and address in (0x3C4, 0x3C5)
        ports.append([address, value])
        if address == 0x3C4:
            assert value == 2
        else:
            state["mask"] = value & 15
            assert state["mask"] in (1, 2, 4, 8)

    def write(_machine, _access, address, size, value, _context):
        if not OUTPUT <= address < OUTPUT + 65536:
            return
        assert size == 1 and state["mask"] is not None
        plane = state["mask"].bit_length() - 1
        pixel = (address - OUTPUT) * 4 + plane
        assert 0 <= pixel < len(pixels), pixel
        pixels[pixel] = value
        writes.append([pixel, value])

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_INSN, port, None, 1, 0, UC_X86_INS_OUT)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start(entry, 0xFFFF0, count=1000000)
    assert state["returned"] and cpu.reg_read(UC_X86_REG_SP) == 0xFF04
    assert bytes(cpu.mem_read(TEXT, len(text))) == text
    expected, clipped = bytearray(len(pixels)), bytearray(len(pixels))
    wrapped = set()
    for index, character in enumerate(text[:-1]):
        if index > cursor:
            break
        glyph = data[config["mapping"] + character]
        if glyph & 128:
            continue
        color = 255 if (cursor - index) & 255 == 0 else 254 if (cursor - index) & 255 == 1 else 253
        for row, bits in enumerate(data[config["glyphs"] + glyph * 8:config["glyphs"] + glyph * 8 + 8]):
            for column in range(8):
                if not bits & (128 >> column):
                    continue
                gx, gy = x + index * 8 + column, y + row
                offset = gy * 320 + gx
                assert 0 <= offset < len(pixels)
                expected[offset] = color
                if gx < 320:
                    clipped[offset] = color
                else:
                    wrapped.add(offset)
    assert pixels == expected
    assert writes and ports
    return dict(game=game, name=name, text=list(text), origin=[x, y], reveal_cursor=cursor,
                indexed_pixels_sha256=sha(pixels), linear_address_model_sha256=sha(expected),
                horizontally_clipped_model_sha256=sha(clipped), wrapped_pixels=sorted(wrapped),
                clipping_matches_original=pixels == clipped, write_count=len(writes),
                writes_sha256=sha(json.dumps(writes).encode()), ports_sha256=sha(json.dumps(ports).encode()),
                executed_routine_sha256=config["routine"], executable_sha256=config["executable"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--trace", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    assert not args.out.exists()
    events = [json.loads(line) for line in args.trace.open()]
    observed = next(e for e in events if e.get("state") and e["state"]["bas_site"] == 8086
                    and e["state"]["text"]["kind"] == "subtitle_bytes")
    first = bytes(observed["state"]["text"]["content"]).split(b"\r")[0] + b"\r"
    assert first.startswith(b"Okay") and len(first) > 40
    cases = []
    for game, config in GAMES.items():
        executable = (ROOT / config["path"]).read_bytes()
        for cursor in (37, 38, 39, len(first) - 2):
            cases.append(run(game, config, executable, first, 10, 8, cursor, f"honk_long_line_cursor_{cursor}"))
        for x in range(316, 320):
            cases.append(run(game, config, executable, b"AA\r", x, 8, 2, f"right_edge_plane_{x % 4}"))
        cases.append(run(game, config, executable, b"A" * 256 + b"\r", 0, 0, 256, "length_byte_wrap_256"))
    result = dict(status="ORIGINAL_ROUTINE_EDGE_WRITES_VERIFIED", scope="drawing primitive, not a whole-game flow or timing proof",
                  oracle_sha256=sha(Path(__file__).read_bytes()), trace_sha256=sha(args.trace.read_bytes()),
                  authored_bas_site=8086, text_hex=first.hex(), cases=cases)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(dict(status=result["status"], cases=len(cases),
        cases_rejecting_horizontal_clipping=sum(not c["clipping_matches_original"] for c in cases),
        output_sha256=sha(args.out.read_bytes()))))


if __name__ == "__main__":
    main()
