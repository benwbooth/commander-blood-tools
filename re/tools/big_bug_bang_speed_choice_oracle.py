#!/usr/bin/env python3
"""Capture the sequel simulation-speed selection tail, without UI callbacks.

Runs unmodified original instructions with the real CS-relative value table.
Transition rendering, main-loop timing and production startup are not exercised.
"""

import argparse
import hashlib
import json
from pathlib import Path
import runpy
import struct

from unicorn.x86_const import UC_X86_REG_AX, UC_X86_REG_CS, UC_X86_REG_SI

harness = runpy.run_path(str(Path(__file__).with_name("big_bug_bang_startup_tables_oracle.py")))
run = harness["run"]
SHA256 = harness["SHA256"]
DATA_FILE = harness["DATA_FILE"]


def capture(executable):
    values = list(struct.unpack_from("<HHH", executable, 0x1D12))
    pointers = struct.unpack_from("<HHHH", executable, DATA_FILE + 0x282B)
    assert pointers[-1] == 0xFFFF
    labels = []
    for pointer in pointers[:-1]:
        field = executable[DATA_FILE + pointer:DATA_FILE + pointer + 32]
        assert b"\0" in field
        labels.append(list(field.split(b"\0", 1)[0]))
    cases = []
    for selected in [0, 1, 2, 3, -1, -32768]:
        for previous in [0, 1, 10, 100, 65535]:
            for ui_flags in [4, 165, 255]:
                _, data = run(
                    executable, 0x1D6F, 0x1D90,
                    [(0xCC4, 0xCC6), (0x2829, 0x282A), (0x2A33, 0x2A34)],
                    {UC_X86_REG_CS: 0x77, UC_X86_REG_AX: selected & 0xFFFF,
                     UC_X86_REG_SI: 0x282B},
                    initial_data=[(0xCC4, struct.pack("<H", previous)),
                                  (0x2829, b"\x01"), (0x2A33, bytes([ui_flags]))])
                cases.append(dict(selected=selected, previous=previous, ui_flags=ui_flags,
                                  result=struct.unpack_from("<H", data, 0xCC4)[0],
                                  active=data[0x2829], final_ui_flags=data[0x2A33]))
    return dict(executable_sha256=SHA256, values=values, labels=labels,
                initial_value=struct.unpack_from("<H", executable, DATA_FILE + 0xCC4)[0],
                cases=cases)


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
    print(f"captured {len(result['cases'])} simulation-speed selection cases")


if __name__ == "__main__":
    main()
