#!/usr/bin/env python3
"""Capture original sequel option effects, stopping at DOS audio calls.

The UI chooser has already returned an authored row. Music startup is captured
at its far-call boundary; DOS audio driver execution is outside this fixture.
No original instructions are patched or replaced by callback stubs.
"""

import argparse
import hashlib
import json
from pathlib import Path
import runpy
import struct

from unicorn.x86_const import UC_X86_REG_AX, UC_X86_REG_SI

harness = runpy.run_path(str(Path(__file__).with_name("big_bug_bang_startup_tables_oracle.py")))
run = harness["run"]
SHA256 = harness["SHA256"]


def word(value):
    return struct.pack("<H", value)


def capture(executable):
    cases = []
    writable = [(0x2829, 0x282B), (0x281B, 0x281D), (0xCF1, 0xCF2),
                (0x27BD, 0x27C1), (0xDAA, 0xDAB), (0xDAD, 0xDAE),
                (0xF7E, 0xF7F), (0x29C4, 0x29C7), (0xD1D, 0xD1E),
                (0xC36, 0xC37), (0xC38, 0xC39), (0x2CB9, 0x2CBB),
                (0x2A33, 0x2A34)]
    for selected in [*range(8), -1, -32768]:
        for supported in [False, True]:
            for music in [False, True]:
                for travel in [False, True]:
                    initial = [(0xCE7, bytes([supported])), (0xDAD, bytes([music])),
                               (0xCF1, bytes([travel])), (0x281B, b"\0\x41"),
                               (0x2829, b"\0\x82"), (0x29C4, b"\0\0\0"),
                               (0xD1D, b"\0"), (0xC36, b"\x01"), (0xC38, b"\x01"),
                               (0x2CB9, word(4)), (0x2A33, b"\xa5"),
                               (0xDAA, b"\x55"), (0xF7E, b"\x7f"),
                               (0x27BD, word(0x27EF if travel else 0x27F9)),
                               (0x27BF, word(0x27E3 if music else 0x27D8))]
                    starts_music = selected == 3 and supported and not music
                    end = 0x9AF9 if starts_music else 0x9B43
                    cpu, data = run(executable, 0x9A67, end, writable,
                                    {UC_X86_REG_AX: selected & 0xFFFF}, initial_data=initial)
                    if starts_music:
                        # Resume only the post-driver branch, with all captured globals intact.
                        assert cpu.reg_read(UC_X86_REG_SI) == 0xF8B
                        _, data = run(executable, 0x9B03, 0x9B43, writable,
                                      initial_data=[(0, data)])
                    music_label = struct.unpack_from("<H", data, 0x27BF)[0]
                    travel_label = struct.unpack_from("<H", data, 0x27BD)[0]
                    assert music_label in (0x27D8, 0x27E3)
                    assert travel_label == (0x27EF if data[0xCF1] else 0x27F9)
                    cases.append(dict(selected=selected, supported=supported, music=music,
                                      travel=travel, simulation_active=data[0x2829],
                                      simulation_phase=data[0x282A], text_active=data[0x281B],
                                      text_phase=data[0x281C], travel_after=bool(data[0xCF1]),
                                      music_after=bool(data[0xDAD]), music_label_off=music_label == 0x27E3,
                                      save=bool(data[0x29C4]), load=bool(data[0x29C5]),
                                      panel=bool(data[0x29C6]), quit=data[0xD1D] == 2,
                                      primary=bool(data[0xC36]), secondary=bool(data[0xC38]),
                                      menu_open=struct.unpack_from("<H", data, 0x2CB9)[0] != 0,
                                      modal=bool(data[0x2A33] & 4), stream_starts=int(starts_music)))
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
    print(f"captured {len(result['cases'])} original options-handler cases")


if __name__ == "__main__":
    main()
