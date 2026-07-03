#!/usr/bin/env python3

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from PIL import Image

import compare_oracle


class CompareOracleTests(unittest.TestCase):
    def test_scan_range_includes_end_timestamp(self) -> None:
        self.assertEqual(compare_oracle.parse_scan_range("0:1:0.5"), (0.0, 1.0, 0.5))
        self.assertEqual(compare_oracle.scan_times(0.0, 1.0, 0.5), [0.0, 0.5, 1.0])
        with self.assertRaises(ValueError):
            compare_oracle.parse_scan_range("1:0:0.5")
        with self.assertRaises(ValueError):
            compare_oracle.parse_scan_range("0:1:0")

    def test_batch_scenarios_report_pass_fail_and_unchecked(self) -> None:
        with tempfile.TemporaryDirectory(prefix="commander-blood-oracle-test-") as tmp:
            root = Path(tmp)
            reference = root / "reference.png"
            same = root / "same.png"
            different = root / "different.png"
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (10, 20, 30)).save(reference)
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (10, 20, 30)).save(same)
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (20, 20, 30)).save(different)

            scenario_file = root / "scenarios.tsv"
            scenario_file.write_text(
                "\n".join(
                    [
                        "scenario_id\treference\tgenerated\tgenerated_time\tref_crop\tmax_mean_abs\tscan_start\tscan_end\tscan_step\tout_dir\tnotes",
                        f"same\t{reference}\t{same}\t0\tauto\t0\t\t\t\t\tpixel-identical",
                        f"different\t{reference}\t{different}\t0\tauto\t1\t\t\t\t\tintentional failure",
                        f"unchecked\t{reference}\t{different}\t0\tauto\t\t\t\t\t\tno threshold yet",
                    ]
                )
                + "\n"
            )

            scenarios = compare_oracle.load_scenarios(scenario_file)
            self.assertIsNone(scenarios[0].scan_start)
            results, exit_code = compare_oracle.run_scenarios(
                scenarios,
                out_root=root / "comparisons",
                summary_out=root / "summary.json",
            )

            statuses = {result["scenario_id"]: result["status"] for result in results}
            self.assertEqual(statuses["same"], "pass")
            self.assertEqual(statuses["different"], "fail")
            self.assertEqual(statuses["unchecked"], "unchecked")
            self.assertEqual(exit_code, 2)
            self.assertTrue((root / "summary.json").exists())

    def test_candidate_search_ranks_best_generated_frame(self) -> None:
        with tempfile.TemporaryDirectory(prefix="commander-blood-candidate-test-") as tmp:
            root = Path(tmp)
            reference = root / "reference.png"
            close = root / "candidate-close.png"
            far = root / "candidate-far.png"
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (10, 20, 30)).save(reference)
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (11, 20, 30)).save(close)
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (100, 20, 30)).save(far)

            candidates = compare_oracle.candidate_paths_from_globs(
                [str(root / "candidate-*.png")]
            )
            summary = compare_oracle.search_candidate_videos(
                reference,
                candidates,
                start=0.0,
                end=0.0,
                step=1.0,
                ref_crop="auto",
                out_dir=root / "candidate-search",
                top_n=2,
            )

            self.assertEqual(summary["best"]["generated"], str(close))
            self.assertAlmostEqual(summary["best"]["best_mean_abs"], 1.0 / 3.0)
            self.assertEqual(summary["top_count"], 2)
            self.assertTrue((root / "candidate-search" / "candidate-search.json").exists())
            self.assertTrue((root / "candidate-search" / "best" / "comparison.json").exists())


if __name__ == "__main__":
    unittest.main()
