#!/usr/bin/env python3
"""Diff two capture_pterra_boundary.py snapshots (original vs relinked).

Reports:
  * guest CS:IP of each side at the boundary
  * every interrupt vector whose bytes differ (the INT-1 storm leaves
    clobbered vectors; catching WHICH vectors differ at the boundary says
    whether corruption already happened before or during materialization)
  * every differing word in the resource band DS:0x0A40..0x0B00 with the
    known label name where labels.csv documents one

Usage:
  python3 -P re/tools/diff_pterra_snapshots.py original.json relinked.json
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

KNOWN_LABELS = {
    "0x0a42": "startup_dos_pool",
    "0x0a46": "resource_free_bytes",
    "0x0a7c": "resource_copy_buffer(off)",
    "0x0a7e": "resource_copy_buffer(seg)",
    "0x0a84": "resource_copy_file_handle",
    "0x0a86": "resource_archive_cache_handle",
    "0x0a8a": "resource_archive_offset",
    "0x0a8e": "resource_archive_remaining",
    "0x0a92": "snd_source_remaining",
    "0x0a96": "alien_overlay_slot",
    "0x0aa4": "resource_index_ptr",
    "0x0b21": "timer_hook_active",
    "0x0b22": "timer_divider",
    "0x0b29": "timer_tick_count",
    "0x0b35": "subtitle_state",
    "0x0bb3": "snd_bank_memory",
    "0x0bb7": "snd_stream_storage",
    "0x0db8": "resource_frame_presented",
}


def main() -> None:
    original = json.loads(Path(sys.argv[1]).read_text())
    relinked = json.loads(sys.argv[2] and Path(sys.argv[2]).read_text())

    def fmt_cpu(side):
        cpu = side.get("cpu") or {}
        if not cpu:
            return "<none>"
        return (f"{cpu['cs']:04x}:{cpu['ip']:04x} "
                f"ds={cpu['ds']:04x} gs={cpu['gs']:04x} "
                f"ax={cpu['ax']:04x} bx={cpu['bx']:04x} "
                f"cx={cpu['cx']:04x} dx={cpu['dx']:04x}")

    print("original cpu:", fmt_cpu(original))
    print("relinked cpu:", fmt_cpu(relinked))

    ivt_o = bytes.fromhex(original["ivt"])
    ivt_r = bytes.fromhex(relinked["ivt"])
    vector_diffs = []
    for vector in range(256):
        a = ivt_o[vector * 4:vector * 4 + 4]
        b = ivt_r[vector * 4:vector * 4 + 4]
        if a != b:
            vector_diffs.append(
                (vector, a.hex(), b.hex(),
                 int.from_bytes(a, "little"), int.from_bytes(b, "little")))
    print(f"\n{len(vector_diffs)} interrupt vectors differ:")

    def validity(value: int, cpu: dict) -> str:
        """A vector is IN-IMAGE when its segment sits near the CPU's own
        DS/CS; a clobbered vector usually points at segment 0 or wild
        low memory."""
        if value == 0:
            return "NULL"
        segment = value >> 16
        anchor = cpu.get("ds", 0) if cpu else 0
        if anchor and abs(segment - anchor) <= 0x2000:
            return "in-image"
        if 0xF000 <= segment <= 0xFFFF or segment < 0x0050:
            return "rom/dos"
        return "WILD"

    for vector, _hex_a, _hex_b, val_a, val_b in vector_diffs[:48]:
        verdict_a = validity(val_a, original.get("cpu") or {})
        verdict_b = validity(val_b, relinked.get("cpu") or {})
        flags = []
        if verdict_a != verdict_b:
            flags.append(f"{verdict_a}->{verdict_b}")
        if "WILD" in (verdict_a, verdict_b) or "NULL" in (
                verdict_a, verdict_b):
            flags.insert(0, "*** SUSPECT ***")
        suffix = f"  [{' '.join(flags)}]" if flags else ""
        print(f"  vec {vector:#04x}: original={val_a:#08x} "
              f"({verdict_a}) relinked={val_b:#08x} ({verdict_b}){suffix}")

    band_o = original["resource_band"]
    band_r = relinked["resource_band"]
    print("\nresource band differences:")
    count = 0
    for offset in sorted(band_o, key=lambda text: int(text, 16)):
        if band_o[offset] != band_r[offset]:
            label = KNOWN_LABELS.get(offset, "")
            print(f"  {offset}: original={band_o[offset]:#06x} "
                  f"relinked={band_r[offset]:#06x} {label}")
            count += 1
    print(f"({count} words differ)")

    back_o = bytes.fromhex(original["back_buffer_area"])
    back_r = bytes.fromhex(relinked["back_buffer_area"])
    if back_o == back_r:
        print("back-buffer pointer area identical")
    else:
        print("back-buffer pointer area DIFFERS:",
              back_o.hex(), "vs", back_r.hex())


if __name__ == "__main__":
    main()
