#!/usr/bin/env python3
"""Execute CB's original subtitle assembler, including dictionary offset one."""

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE
from unicorn.x86_const import (
    UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_ES, UC_X86_REG_GS,
    UC_X86_REG_SS, UC_X86_REG_SP, UC_X86_REG_SI, UC_X86_REG_DI,
)

SHA256 = "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
HEADER = 0x600
GLOBALS, SOURCE, DICTIONARY = 0x30000, 0x40000, 0x60000


def run(executable, dictionary, words, name):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    cpu.mem_write(0, executable[HEADER:])
    source = struct.pack(f"<{len(words)}H", *words)
    cpu.mem_write(SOURCE, source)
    cpu.mem_write(DICTIONARY, dictionary)
    cpu.mem_write(GLOBALS + 0x672A, struct.pack("<H", DICTIONARY // 16))
    for reg, value in [(UC_X86_REG_CS, 0x4DA), (UC_X86_REG_DS, SOURCE // 16),
                       (UC_X86_REG_ES, GLOBALS // 16), (UC_X86_REG_GS, GLOBALS // 16),
                       (UC_X86_REG_SS, GLOBALS // 16), (UC_X86_REG_SP, 0xFF00),
                       (UC_X86_REG_SI, 0)]:
        cpu.reg_write(reg, value)
    ended = []

    def instruction(machine, address, size, _context):
        offset = address + HEADER
        if offset == 0x673D:
            ended.append(True)
            machine.emu_stop()
            return
        assert any(start <= offset and offset + size <= end for start, end in
                   [(0x66CD, 0x673D), (0x67A7, 0x67BD)]), hex(offset)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.emu_start(0x66CD - HEADER, 0, count=10000)
    assert ended == [True]
    output = bytes(cpu.mem_read(GLOBALS + 0xE18, cpu.reg_read(UC_X86_REG_DI) - 0xE18))
    assert output[-1:] == b"\0"
    assert bytes(cpu.mem_read(SOURCE, len(source))) == source
    assert bytes(cpu.mem_read(DICTIONARY, len(dictionary))) == dictionary
    return dict(name=name, words=words, output=list(output[:-1]),
                cursor=cpu.reg_read(UC_X86_REG_SI))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("resources", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    dictionary = (args.resources / "SCRIPT2.DIC").read_bytes()
    code = (args.resources / "SCRIPT2.COD").read_bytes()
    assert dictionary[1:6] == b"talk\0"
    site = 0x6FF6
    assert code[site] == 0xA6
    words, cursor = [], site + 6
    while True:
        word = struct.unpack_from("<H", code, cursor)[0]
        words.append(word)
        cursor += 2
        if word == 0:
            break
    cases = [run(executable, dictionary, row, name) for name, row in [
        ("morning_ark_instructions", words),
        ("dictionary_one_alone", [1, 0]),
        ("dictionary_one_repeated", [1, 1, 1, 0]),
        ("dictionary_one_terminated_by_section", [1, 0xFFFF, 0]),
    ]]
    result = dict(executable_sha256=SHA256,
                  dictionary_sha256=hashlib.sha256(dictionary).hexdigest(),
                  code_sha256=hashlib.sha256(code).hexdigest(),
                  authored_site=site, dictionary=list(dictionary), cases=cases)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(cases)} original Commander subtitle cases")


if __name__ == "__main__":
    main()
