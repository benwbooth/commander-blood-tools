#!/usr/bin/env python3
from __future__ import annotations

import csv
import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest


TOOL = Path(__file__).with_name("oracle_branch_coverage.py")
SPEC = importlib.util.spec_from_file_location("oracle_branch_coverage", TOOL)
assert SPEC is not None and SPEC.loader is not None
coverage = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = coverage
SPEC.loader.exec_module(coverage)


class OracleBranchCoverageTests(unittest.TestCase):
    def make_fixture(self, root: Path, status: str) -> tuple[Path, dict[str, bytes]]:
        asm = root / "func.asm"
        # cmp ax,0; je taken; inc bx; ret; dec bx; ret
        image = bytes.fromhex("83f800740243c34bc3")
        asm.write_text(
            "; overlay_offset: 0x000000\n"
            "; routine_entry: 0x000000\n"
            "; byte_count: 9\n\n"
            "00000000 <routine>:\n",
            encoding="ascii",
        )
        manifest = root / "manifest.tsv"
        with manifest.open("w", encoding="ascii", newline="") as stream:
            writer = csv.writer(stream, delimiter="\t", lineterminator="\n")
            writer.writerow(("entry", "source", "asm_path", "function", "status", "notes"))
            writer.writerow(
                ("xdb_demo:0x000000", "func.c", "func.asm", "demo", status, "")
            )
        return manifest, {"demo": image}

    def test_complete_taken_and_fallthrough_edges(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest, images = self.make_fixture(
                root, "natural_candidate_oracle_verified_codegen_accepted"
            )
            recorder = coverage.CoverageRecorder({9: "demo"})
            recorder.record_trace("demo", [0, 3, 7, 8])
            recorder.record_trace("demo", [0, 3, 5, 6])
            rows = coverage.build_report(root, manifest, images, recorder)
            self.assertEqual(rows[0]["coverage_status"], "complete")
            self.assertEqual(rows[0]["covered_branch_edge_count"], "2")
            coverage.require_complete_direct_coverage(rows)

    def test_patched_helper_instruction_is_not_credited(self) -> None:
        canonical = bytes.fromhex("90c390")
        patched = bytes.fromhex("90ccc3")
        recorder = coverage.CoverageRecorder(
            {len(canonical): "demo"}, {"demo": canonical}
        )
        hook = recorder.hook_for(patched, 0)
        hook(None, 0, 1, None)
        hook(None, 1, 1, None)
        hook(None, 2, 1, None)
        self.assertEqual(recorder.instructions["demo"], {0})
        self.assertEqual(recorder.edges["demo"], {(0, 1)})

    def test_missing_edge_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest, images = self.make_fixture(
                root, "natural_candidate_oracle_verified_codegen_mismatch"
            )
            recorder = coverage.CoverageRecorder({9: "demo"})
            recorder.record_trace("demo", [0, 3, 7, 8])
            rows = coverage.build_report(root, manifest, images, recorder)
            self.assertEqual(rows[0]["coverage_status"], "branch_incomplete")
            self.assertEqual(rows[0]["missing_branch_edges"], "0x0003->0x0005")
            with self.assertRaises(SystemExit):
                coverage.require_complete_direct_coverage(rows)

    def test_non_oracle_routine_is_reported_not_failed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest, images = self.make_fixture(
                root, "natural_candidate_compiled_raw_control_flow_reviewed"
            )
            recorder = coverage.CoverageRecorder({9: "demo"})
            rows = coverage.build_report(root, manifest, images, recorder)
            self.assertEqual(
                rows[0]["coverage_status"], "no_direct_original_execution"
            )
            coverage.require_complete_direct_coverage(rows)

    def test_review_manifest_pins_missing_edge_set(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest, images = self.make_fixture(
                root, "natural_candidate_oracle_verified_codegen_mismatch"
            )
            recorder = coverage.CoverageRecorder({9: "demo"})
            recorder.record_trace("demo", [0, 3, 7, 8])
            rows = coverage.build_report(root, manifest, images, recorder)
            digest = __import__("hashlib").sha256(
                rows[0]["missing_branch_edges"].encode("ascii")
            ).hexdigest()
            reviews = root / "reviews.tsv"
            reviews.write_text(
                "module\tentry\tmissing_edges_sha256\tdisposition\tevidence\n"
                f"demo\t0x000000\t{digest}\tdirected_vectors_required\t"
                "fallthrough case remains to be added\n",
                encoding="ascii",
            )
            coverage.require_reviewed_direct_coverage(rows, reviews)
            reviews.write_text(
                reviews.read_text(encoding="ascii").replace(digest, "0" * 64),
                encoding="ascii",
            )
            with self.assertRaises(SystemExit):
                coverage.require_reviewed_direct_coverage(rows, reviews)


if __name__ == "__main__":
    unittest.main()
