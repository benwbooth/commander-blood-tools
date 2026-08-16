#!/usr/bin/env python3
"""Compile one recovered C source with archived Turbo C 2.01 under DOSBox-X."""

from __future__ import annotations

import argparse
import importlib.util
import os
from pathlib import Path
import shutil
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[2]
CORPUS_TOOL = ROOT / "re" / "tools" / "compiler_corpus.py"


def load_corpus_tool():
    spec = importlib.util.spec_from_file_location("compiler_corpus", CORPUS_TOOL)
    if spec is None or spec.loader is None:
        raise SystemExit(f"could not load source staging tool: {CORPUS_TOOL}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--toolchain", type=Path, required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--dosbox", default="dosbox-x")
    parser.add_argument("--flag", action="append", default=["-mm", "-O", "-Z"])
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    toolchain = args.toolchain.resolve()
    source = args.source.resolve()
    output = args.output.resolve()
    tcc = toolchain / "TC" / "TCC.EXE"
    include = toolchain / "TC" / "INCLUDE"
    if not tcc.is_file() or not include.is_dir():
        raise SystemExit(f"invalid Turbo C tree: {toolchain}")
    if not source.is_file():
        raise SystemExit(f"source does not exist: {source}")
    dosbox = shutil.which(args.dosbox) or args.dosbox
    if not Path(dosbox).is_file() and shutil.which(dosbox) is None:
        raise SystemExit(f"DOSBox executable not found: {args.dosbox}")

    output.parent.mkdir(parents=True, exist_ok=True)
    work = output.parent / f"{output.stem}.turbo_work"
    work.mkdir(parents=True, exist_ok=True)
    for path in (work / "PROBE.C", work / "PROBE.OBJ", work / "PROBE.LOG"):
        if path.exists():
            path.unlink()
    for path in work.glob("CB*.H"):
        path.unlink()

    corpus = load_corpus_tool()
    corpus.stage_dos_source_tree(source, work)
    flags = " ".join(args.flag)
    command = [
        str(dosbox),
        "--noprimaryconf",
        "--nolocalconf",
        "--exit",
        "-set",
        "sdl fullscreen=false",
        "-set",
        "sdl output=texture",
        "-c",
        f'mount c "{toolchain}"',
        "-c",
        f'mount d "{work}"',
        "-c",
        r"set PATH=C:\TC",
        "-c",
        "d:",
        "-c",
        f"TCC.EXE -c {flags} -IC:\\TC\\INCLUDE -oPROBE.OBJ PROBE.C > PROBE.LOG",
    ]
    environment = os.environ.copy()
    environment["SDL_AUDIODRIVER"] = "dummy"
    environment["SDL_VIDEODRIVER"] = "offscreen"
    process = subprocess.run(command, cwd=ROOT, capture_output=True, env=environment)
    log = work / "PROBE.LOG"
    if process.returncode != 0 or not (work / "PROBE.OBJ").is_file():
        diagnostics = process.stderr.decode(errors="replace")
        if log.is_file():
            diagnostics += "\n" + log.read_text(errors="replace")
        raise SystemExit(f"Turbo C compilation failed for {source}:\n{diagnostics}")
    shutil.copy2(work / "PROBE.OBJ", output)
    print(f"wrote Turbo C object: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
