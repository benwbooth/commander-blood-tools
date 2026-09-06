#!/usr/bin/env python3
"""Capture the sequel's original main-loop decrement and script-tail reload."""

import argparse
import hashlib
import json
from pathlib import Path
import runpy
import struct

harness = runpy.run_path(str(Path(__file__).with_name("big_bug_bang_startup_tables_oracle.py")))
run = harness["run"]
SHA256 = harness["SHA256"]


def capture(executable):
    cases = []
    for reload in [0, 1, 10, 100, 65535]:
        for countdown in [0, 1, 2, 9, 10, 99, 100, 65535]:
            _, data = run(executable, 0x10CA, 0x10D5, [(0xCC6, 0xCC8)],
                          initial_data=[(0xCC4, struct.pack("<HH", reload, countdown))])
            after_begin = struct.unpack_from("<H", data, 0xCC6)[0]
            _, data = run(executable, 0x5B46, 0x5B56, [(0xCC6, 0xCC8)],
                          initial_data=[(0xCC4, struct.pack("<HH", reload, after_begin))])
            after_finish = struct.unpack_from("<H", data, 0xCC6)[0]
            cases.append(dict(reload=reload, countdown=countdown,
                              after_begin=after_begin, after_finish=after_finish))
    return dict(executable_sha256=SHA256, cases=cases)


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
    print(f"captured {len(result['cases'])} original simulation-clock cases")


if __name__ == "__main__":
    main()
