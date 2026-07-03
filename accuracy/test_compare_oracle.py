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

    def test_region_metrics_isolate_hud_panel_difference(self) -> None:
        reference = Image.new("RGB", compare_oracle.NATIVE_SIZE, (0, 0, 0))
        generated = Image.new("RGB", compare_oracle.NATIVE_SIZE, (0, 0, 0))
        x, y, w, h = compare_oracle.SCREEN_REGIONS["hud_panel"]
        for py in range(y, y + h):
            for px in range(x, x + w):
                generated.putpixel((px, py), (30, 0, 0))

        regions = compare_oracle.region_metrics(reference, generated)

        self.assertEqual(regions["top_bar"]["mean_abs"], 0.0)
        self.assertEqual(regions["scene_band"]["mean_abs"], 0.0)
        self.assertEqual(regions["bottom_bar"]["mean_abs"], 0.0)
        self.assertEqual(regions["hud_panel"]["mean_abs"], 10.0)

    def test_compare_uses_capture_manifest_crop_and_frame_path(self) -> None:
        with tempfile.TemporaryDirectory(prefix="commander-blood-manifest-test-") as tmp:
            root = Path(tmp)
            capture_dir = root / "captures"
            capture_dir.mkdir()
            reference = capture_dir / "frame_01.png"
            generated = root / "generated.png"

            reference_image = Image.new("RGB", (800, 600), (0, 0, 0))
            crop_image = Image.new("RGB", compare_oracle.NATIVE_SIZE, (40, 50, 60))
            reference_image.paste(crop_image, (10, 20))
            reference_image.save(reference)
            crop_image.save(generated)

            manifest = capture_dir / "capture-manifest.tsv"
            manifest.write_text(
                "\n".join(
                    [
                        "frame\telapsed_s\tepoch_s\tdisplay\tcapture_kind\tcrop_x\tcrop_y\tcrop_w\tcrop_h\tnative_w\tnative_h",
                        "frame_01.png\t4\t123456\t:98\thost-root\t10\t20\t320\t200\t320\t200",
                    ]
                )
                + "\n"
            )

            metrics = compare_oracle.compare_paths(
                Path("frame_01.png"),
                generated,
                generated_time=0.0,
                ref_crop="auto",
                out_dir=root / "comparison",
                reference_manifest=manifest,
                max_mean_abs=0.0,
            )

            self.assertEqual(metrics["status"], "pass")
            self.assertEqual(metrics["mean_abs"], 0.0)
            self.assertEqual(metrics["reference"], str(reference))
            self.assertEqual(metrics["reference_crop"], [10, 20, 320, 200])
            self.assertEqual(metrics["reference_manifest"]["elapsed_s"], 4.0)

    def test_compare_uses_capture_manifest_path_column(self) -> None:
        with tempfile.TemporaryDirectory(prefix="commander-blood-manifest-test-") as tmp:
            root = Path(tmp)
            capture_dir = root / "captures"
            manifest_dir = root / "meta"
            capture_dir.mkdir()
            manifest_dir.mkdir()
            reference = capture_dir / "frame_02.png"
            generated = root / "generated.png"

            Image.new("RGB", compare_oracle.NATIVE_SIZE, (70, 80, 90)).save(reference)
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (70, 80, 90)).save(generated)

            manifest = manifest_dir / "capture-manifest.tsv"
            manifest.write_text(
                "\n".join(
                    [
                        "frame\tpath\telapsed_s\tepoch_s\tdisplay\tcapture_kind\tcrop_x\tcrop_y\tcrop_w\tcrop_h\tnative_w\tnative_h",
                        f"frame_02.png\t{reference}\t8\t123457\t:98\thost-root\t0\t0\t320\t200\t320\t200",
                    ]
                )
                + "\n"
            )

            metrics = compare_oracle.compare_paths(
                Path("frame_02.png"),
                generated,
                generated_time=0.0,
                ref_crop="auto",
                out_dir=root / "comparison",
                reference_manifest=manifest,
                max_mean_abs=0.0,
            )

            self.assertEqual(metrics["status"], "pass")
            self.assertEqual(metrics["reference"], str(reference))
            self.assertEqual(metrics["reference_manifest"]["path"], str(reference))


if __name__ == "__main__":
    unittest.main()
