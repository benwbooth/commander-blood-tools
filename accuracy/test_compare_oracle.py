#!/usr/bin/env python3

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from PIL import Image

import compare_oracle


class CompareOracleTests(unittest.TestCase):
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
                        "scenario_id\treference\tgenerated\tgenerated_time\tref_crop\tmax_mean_abs\tout_dir\tnotes",
                        f"same\t{reference}\t{same}\t0\tauto\t0\t\tpixel-identical",
                        f"different\t{reference}\t{different}\t0\tauto\t1\t\tintentional failure",
                        f"unchecked\t{reference}\t{different}\t0\tauto\t\t\tno threshold yet",
                    ]
                )
                + "\n"
            )

            scenarios = compare_oracle.load_scenarios(scenario_file)
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


if __name__ == "__main__":
    unittest.main()
