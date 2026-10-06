#!/usr/bin/env python3
"""Compact verified native captures while retaining masters and original evidence.

Only coverage/assembly entries with an unchanged, previously verified master are
eligible. Failed captures and source assets are never cleanup candidates.
"""

import argparse
from concurrent.futures import ThreadPoolExecutor
import gzip
import hashlib
import json
from pathlib import Path
import shutil
import tempfile

from video_anthology import digest, save_json


def require(condition, message):
    if not condition:
        raise ValueError(message)


def open_trace(path, mode="rt"):
    path = Path(path)
    if path.exists():
        return path.open(mode)
    return gzip.open(path.with_suffix(path.suffix + ".gz"), mode)


def trace_digest(path):
    with open_trace(path, "rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def verify_optional_pcm(path, expected):
    path = Path(path) / "audio.f32le"
    if path.exists():
        require(digest(path) == expected, f"damaged raw audio: {path}")


def compress_trace(path, expected=None):
    compressed = path.with_suffix(path.suffix + ".gz")
    if not path.exists():
        actual = trace_digest(path)
        require(expected is None or actual == expected, "compressed trace changed")
        return actual, 0
    before = path.stat()
    actual = digest(path)
    require(expected is None or actual == expected, "native trace changed")
    if compressed.exists():
        with gzip.open(compressed, "rb") as source:
            require(hashlib.file_digest(source, "sha256").hexdigest() == actual,
                    "existing compressed trace differs")
        added_bytes = 0
    else:
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix=".trace-", delete=False) as target:
            temporary = Path(target.name)
            try:
                with path.open("rb") as source, gzip.GzipFile(
                        filename="", fileobj=target, mode="wb", mtime=0, compresslevel=3) as output:
                    shutil.copyfileobj(source, output)
                target.flush()
                with gzip.open(temporary, "rb") as source:
                    require(hashlib.file_digest(source, "sha256").hexdigest() == actual,
                            "compressed trace did not round-trip")
                require(not compressed.exists(), "compressed trace appeared during compaction")
                temporary.replace(compressed)
            finally:
                temporary.unlink(missing_ok=True)
        added_bytes = compressed.stat().st_size
    after = path.stat()
    require((before.st_ino, before.st_size, before.st_mtime_ns) ==
            (after.st_ino, after.st_size, after.st_mtime_ns), "trace changed during compaction")
    path.unlink()
    return actual, before.st_size - added_bytes


def compact_chapter(entry, apply=False):
    path = Path(entry["path"]).resolve()
    for name in ("master.mkv", "report.json", "native-state.jsonl", "native-state.jsonl.gz",
                 "audio.f32le", "video.mkv", "storage.json"):
        require(not (path / name).is_symlink(), f"capture artifact is a symlink: {path / name}")
    report = json.loads((path / "report.json").read_text())
    require(report.get("complete_native_presentation") and report.get("decoded_master_verified")
            and report.get("video_timestamps_verified"), "chapter lacks verified native media")
    require(report["audio_f32le_sha256"] == entry["audio_sha256"]
            and report["rgba_sha256"] == entry["rgba_sha256"], "chapter report differs from coverage")
    if "report_sha256" in entry:
        require(digest(path / "report.json") == entry["report_sha256"], "chapter report changed")
    names = [name for name in ("video.mkv", "audio.f32le") if (path / name).is_file()]
    trace = path / "native-state.jsonl"
    raw_bytes = sum((path / name).stat().st_size for name in names)
    trace_bytes = trace.stat().st_size if trace.exists() else 0
    require(trace.exists() or trace.with_suffix(".jsonl.gz").exists(), "missing native trace")
    require((path / "master.mkv").is_file(), "missing chapter master")
    result = dict(path=str(path), redundant_media_bytes=raw_bytes, raw_trace_bytes=trace_bytes,
                  applied=apply, saved_bytes=0)
    if not apply:
        return result
    require(digest(path / "master.mkv") == entry["master_sha256"], "chapter master changed")
    verify_optional_pcm(path, entry["audio_sha256"])
    trace_hash, saved = compress_trace(trace, entry.get("native_state_sha256"))
    save_json(path / "storage.json", dict(schema=1, policy="verified_master_compressed_trace",
              master_sha256=entry["master_sha256"], native_state_sha256=trace_hash,
              audio_sha256=entry["audio_sha256"], redundant_files=["video.mkv", "audio.f32le"],
              evidence="master matches the saved full-decode verification; gzip round-trip verified"))
    for name in names:
        (path / name).unlink()
    result["saved_bytes"] = raw_bytes + saved
    return result


def selected_entries(root=None, manifests=()):
    entries = {}
    documents = list(manifests)
    if root is not None:
        root = root.resolve()
        documents.extend(sorted(root.rglob("coverage.json")))
    for document in documents:
        data = json.loads(document.read_text())
        for entry in data.get("sources", data.get("completed", [])):
            if not isinstance(entry, dict) or not all(key in entry for key in
                    ("path", "master_sha256", "audio_sha256", "rgba_sha256")):
                continue
            path = Path(entry["path"]).resolve()
            require(root is None or path.is_relative_to(root), "capture escapes requested root")
            previous = entries.get(path, {})
            for key in ("master_sha256", "audio_sha256", "rgba_sha256", "report_sha256",
                        "native_state_sha256"):
                require(key not in previous or key not in entry or previous[key] == entry[key],
                        f"conflicting saved verification for {path}: {key}")
            entries[path] = {**previous, **entry, "path": str(path)}
    return list(entries.values())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, help="scan native coverage reports under this directory")
    parser.add_argument("--manifest", type=Path, action="append", default=[])
    parser.add_argument("--apply", action="store_true", help="default is a read-only size audit")
    parser.add_argument("--jobs", type=int, default=2)
    parser.add_argument("--out", type=Path, required=True, help="JSON audit/cleanup receipt")
    args = parser.parse_args()
    require(args.root or args.manifest, "select a root or manifest")
    require(1 <= args.jobs <= 8, "jobs must be between 1 and 8")
    entries = selected_entries(args.root, args.manifest)
    results, failures = [], []

    def run(entry):
        try:
            return compact_chapter(entry, args.apply), None
        except (ValueError, KeyError, OSError, EOFError) as error:
            return None, dict(path=entry["path"], error=str(error))

    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        for index, (result, failure) in enumerate(pool.map(run, entries)):
            if failure:
                failures.append(failure)
            else:
                results.append(result)
            if (index + 1) % 25 == 0:
                print(f"Checked {index + 1}/{len(entries)} captures", flush=True)
    summary = dict(schema=1, applied=args.apply, candidates=len(entries), chapters=results,
                   failures=failures, saved_bytes=sum(row["saved_bytes"] for row in results),
                   redundant_media_bytes=sum(row["redundant_media_bytes"] for row in results),
                   raw_trace_bytes=sum(row["raw_trace_bytes"] for row in results))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    save_json(args.out, summary)
    print(json.dumps({key: value for key, value in summary.items() if key != "chapters"}), flush=True)
    require(not failures, "some captures could not be compacted; see the receipt")


if __name__ == "__main__":
    main()
