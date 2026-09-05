#!/usr/bin/env python3
"""Execute the complete sequel CC panel-activation helper and camera request gate.

The activation helper has no callees. The camera gate stops before the common
line helper; these probes do not replace it with a fake result.
"""

import argparse
import hashlib
import itertools
import json
from pathlib import Path
import runpy
from types import SimpleNamespace

reference = SimpleNamespace(**runpy.run_path(str(Path(__file__).with_name("big_bug_bang_presentation_oracle.py"))))
PANEL_ACTIVE = 0x2A81
CAMERA_ACTIVE = 0x2A25
OVERVIEW_ACTIVE = 0x2A30
CAMERA_SLOT = 0x2CBB
PANEL_SLOT = CAMERA_SLOT + 48
UI_FLAGS = 0x2A33
HAND_SELECTOR = 0x0C2A


def vectors(executable):
    for pending, panel, camera, overview, flags in itertools.product([255, 0, 5], [0, 1], [0, 1], [0, 1], range(16)):
        yield reference.run(executable, f"activate_{pending}_{panel}_{camera}_{overview}_{flags}",
                            0x8A48, [reference.RETURN_IP],
                            [CAMERA_SLOT, PANEL_SLOT, OVERVIEW_ACTIVE, UI_FLAGS],
                            {reference.PENDING_CHOICE: pending, PANEL_ACTIVE: panel,
                             CAMERA_ACTIVE: camera, OVERVIEW_ACTIVE: overview,
                             CAMERA_SLOT: flags, PANEL_SLOT: 15 - flags,
                             UI_FLAGS: 160 | ((flags & 1) << 2)}, stack_scratch=2)
    for pending, primary, hand in itertools.product([255, 0, 5], [0, 1], [10, 23]):
        yield reference.run(executable, f"camera_{pending}_{primary}_{hand}",
                            0x91F5, [0x9207], [HAND_SELECTOR, HAND_SELECTOR + 1, reference.PRIMARY],
                            {reference.PENDING_CHOICE: pending, reference.PRIMARY: primary,
                             HAND_SELECTOR: hand, HAND_SELECTOR + 1: 0})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    if hashlib.sha256(executable).hexdigest() != reference.EXECUTABLE_SHA256:
        raise SystemExit("unsupported BLOOD2PG.EXE build")
    results = list(vectors(executable))
    args.output.write_text("".join(json.dumps(x, separators=(",", ":")) + "\n" for x in results))
    print(f"wrote {len(results)} original activation/camera-gate cases")


if __name__ == "__main__":
    main()
