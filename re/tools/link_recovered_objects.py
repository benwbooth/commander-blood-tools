#!/usr/bin/env python3
"""Link recovered DOS objects and record the unresolved-symbol frontier."""

from __future__ import annotations

import argparse
from collections import Counter
import re
from pathlib import Path
import shutil
import subprocess


ROOT = Path(__file__).resolve().parents[2]
UNRESOLVED_RE = re.compile(r"^Error! E2028: (.+) is an undefined reference$")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--main-object", type=Path, required=True)
    parser.add_argument(
        "--object-dir",
        type=Path,
        action="append",
        required=True,
        help="directory to scan recursively for .OBJ files; repeatable",
    )
    parser.add_argument(
        "--extra-object",
        type=Path,
        action="append",
        default=[],
        help="explicit object to add without scanning its parent directory; repeatable",
    )
    parser.add_argument("--wlink", default="wlink")
    parser.add_argument("--output-dir", type=Path, default=ROOT / "output" / "link_probe")
    parser.add_argument("--name", default="BLOODPRG_LINK_PROBE.EXE")
    parser.add_argument(
        "--map",
        action="store_true",
        help="write a linker map beside the executable for entry/layout diagnostics",
    )
    parser.add_argument("--library", action="append", default=["clibm", "doslfnh"])
    return parser.parse_args()


def find_executable(value: str) -> str:
    resolved = shutil.which(value)
    if resolved is None:
        raise SystemExit(f"WLINK executable not found: {value}")
    return resolved


def main() -> int:
    args = parse_args()
    wlink = find_executable(args.wlink)
    main_object = args.main_object.resolve()
    if not main_object.is_file():
        raise SystemExit(f"main object does not exist: {main_object}")

    objects = sorted(
        path.resolve()
        for directory in args.object_dir
        for path in directory.resolve().rglob("*.OBJ")
    )
    extra_objects = [path.resolve() for path in args.extra_object]
    missing_extra = [path for path in extra_objects if not path.is_file()]
    if missing_extra:
        raise SystemExit(
            "extra object does not exist: "
            + ", ".join(str(path) for path in missing_extra)
        )
    objects.extend(extra_objects)
    if not objects:
        raise SystemExit("no .OBJ files found")

    output_dir = args.output_dir.resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    executable = output_dir / args.name
    response = [
        "system dos",
        f"name {executable}",
        "option quiet",
        f"file {main_object}",
        *(f"file {path}" for path in objects),
        *(f"library {library}" for library in args.library),
    ]
    if args.map:
        response.insert(3, f"option map={output_dir / 'link.map'}")
    process = subprocess.run(
        [wlink],
        cwd=ROOT,
        input="\n".join(response) + "\n",
        text=True,
        capture_output=True,
        check=False,
    )
    log = output_dir / "link.log"
    log.write_text(
        "$ " + " ".join([wlink, "< response"])
        + "\n"
        + process.stdout
        + process.stderr,
        encoding="utf-8",
    )

    symbols = [
        match.group(1)
        for line in (process.stdout + process.stderr).splitlines()
        if (match := UNRESOLVED_RE.match(line))
    ]
    counts = Counter(symbols)
    report = output_dir / "unresolved.tsv"
    report.write_text(
        "symbol\treferences\n"
        + "".join(f"{symbol}\t{counts[symbol]}\n" for symbol in sorted(counts)),
        encoding="ascii",
    )

    if process.returncode == 0 and executable.is_file() and not symbols:
        print(f"linked {executable} from {len(objects)} recovered objects")
        print(f"wrote {log}")
        return 0

    print(
        f"link unresolved: {len(counts)} unique symbol(s), "
        f"{len(symbols)} reference(s) across {len(objects)} object(s)"
    )
    print(f"wrote {report}")
    print(f"wrote {log}")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
