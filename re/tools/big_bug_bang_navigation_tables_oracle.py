#!/usr/bin/env python3
"""Capture sequel confirmation hits, navigation labels/wipes and travel names.

Only bounded original instructions execute, using the shared guarded capture
harness. These data and selection captures do not exercise full UI workflows.
"""

import argparse
import hashlib
import json
from pathlib import Path
import runpy
import struct

from unicorn.x86_const import *

harness = runpy.run_path(str(Path(__file__).with_name("big_bug_bang_startup_tables_oracle.py")))
run = harness["run"]
SHA256 = harness["SHA256"]


def capture(executable):
    regions = []
    for pointer in [0x27A7, 0x27AF]:
        x, y, width, height = struct.unpack_from("<hhhh", executable, 0xF7F0 + pointer)
        cases = []
        for pressed in [0, 1]:
            for px in [x - 1, x, x + width, x + width + 1]:
                for py in [y - 1, y, y + height, y + height + 1]:
                    cpu, _ = run(executable, 0x93F7, 0x9424, [(0xFEFE, 0xFF00)],
                                 {UC_X86_REG_BP: pointer}, initial_data=[
                                     (0xC22, struct.pack("<hh", px, py)), (0xC36, bytes([pressed]))])
                    cases.append(dict(point=[px, py], pressed=bool(pressed),
                                      hit=bool(cpu.reg_read(UC_X86_REG_EFLAGS) & 1)))
        regions.append(dict(origin=[x, y], size=[width, height], cases=cases))

    labels = []
    for index, pointer in enumerate([0x12D, 0x137, 0x142, 0x14E]):
        start, end = (0x94E3, 0x94EB) if index < 3 else (0x9504, 0x950C)
        cpu, data = run(executable, start, end, [(0xE000, 0xE020)],
                        {UC_X86_REG_SI: pointer, UC_X86_REG_DI: 0xE000})
        count = cpu.reg_read(UC_X86_REG_DI) - 0xE000
        assert 0 < count < 32
        labels.append(list(data[0xE000:0xE000 + count]))

    endpoints = []
    for index in range(9):
        cpu, _ = run(executable, 0xA028, 0xA03B, [(0x2A2C, 0x2A2D)],
                     initial_data=[(0x2A26, bytes([index + 1]))])
        pointer = cpu.reg_read(UC_X86_REG_SI)
        cpu, _ = run(executable, 0xAB10, 0xAB16, registers={UC_X86_REG_SI: pointer})
        endpoints.append([cpu.reg_read(UC_X86_REG_BX), cpu.reg_read(UC_X86_REG_CX)])

    hyperspace = []
    for index in [*range(10), 0xFFFF]:
        cpu, data = run(executable, 0x9D14, 0x9D2F, [(0x216E, 0x2170), (0x2358, 0x2368)],
                        initial_data=[(0x216E, struct.pack("<H", index))])
        count = cpu.reg_read(UC_X86_REG_DI) - 0x2358
        assert 1 < count <= 16
        assert data[0x2358 + count - 1] == 0
        after = struct.unpack_from("<H", data, 0x216E)[0]
        assert after == (index + 1) & 0xFFFF
        hyperspace.append(dict(index=index, next_index=after, name=list(data[0x2358:0x2358 + count - 1])))
    return dict(executable_sha256=SHA256, regions=regions, labels=labels,
                wipe_endpoints=endpoints, hyperspace=hyperspace)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    result = capture(executable)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print("captured 64 confirmation hits, 4 labels, 9 endpoints and 11 travel selections")


if __name__ == "__main__":
    main()
