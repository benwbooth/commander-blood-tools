#!/usr/bin/env python3
"""Syntax-check the currently translated Borland C++ routine subset.

This is a host-side preflight.  It does not prove Borland C++ code generation or
DOS runtime behavior; it only verifies that files marked translated in the
manifest no longer trip the intentional `#error` stop gates and parse as C++03.
"""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

_HERE_STR = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE_STR
]

RE_ROOT = Path(_HERE_STR).parent
PROJECT_ROOT = RE_ROOT.parent
DEFAULT_MANIFEST = RE_ROOT / "routine_recovery_manifest.json"
DEFAULT_INCLUDE = RE_ROOT / "borland" / "include"


def translated_entries(manifest_path):
    with manifest_path.open(encoding="utf-8") as fh:
        manifest = json.load(fh)
    return [
        entry
        for entry in manifest["entries"]
        if entry["cxx_status"].startswith("translated")
    ]


def compile_one(compiler, include_dir, source_path):
    cmd = [
        compiler,
        "-std=c++03",
        "-I",
        str(include_dir),
        "-fsyntax-only",
        str(source_path),
    ]
    return subprocess.run(
        cmd,
        cwd=str(PROJECT_ROOT),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--include-dir", type=Path, default=DEFAULT_INCLUDE)
    parser.add_argument("--compiler", default="g++")
    args = parser.parse_args()

    failures = []
    checked = 0
    for entry in translated_entries(args.manifest):
        source_path = PROJECT_ROOT / entry["cpp_path"]
        if not source_path.exists():
            failures.append(
                {
                    "source": str(source_path),
                    "status": "missing_source",
                    "stderr": "",
                }
            )
            continue
        source_text = source_path.read_text(encoding="utf-8")
        if "#error" in source_text:
            failures.append(
                {
                    "source": str(source_path),
                    "status": "contains_stop_gate",
                    "stderr": "",
                }
            )
            continue
        result = compile_one(args.compiler, args.include_dir, source_path)
        checked += 1
        if result.returncode != 0:
            failures.append(
                {
                    "source": str(source_path),
                    "status": "compile_failed",
                    "stdout": result.stdout,
                    "stderr": result.stderr,
                }
            )

    summary = {
        "checked": checked,
        "compiler": args.compiler,
        "failures": failures,
        "manifest": str(args.manifest),
        "status": "ok" if not failures else "failed",
    }
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0 if not failures else 1


if __name__ == "__main__":
    raise SystemExit(main())
