#!/usr/bin/env python3

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from PIL import Image

import compare_oracle


TIMELINE_HEADER = "\t".join(
    [
        "mp4",
        "label",
        "segment_index",
        "start_time",
        "end_time",
        "duration",
        "reveal_complete_time",
        "subtitle_hold_end_time",
        "active_line_id",
        "subtitle_chars",
        "has_voice",
        "voice_index",
        "voice_duration",
        "voice_sample_rate",
        "has_talk_hnm",
        "talk_hnm",
        "play_chatter",
        "text",
    ]
)


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
                        "scenario_id\treference\treference_manifest\tgenerated\tgenerated_time\tref_crop\tmax_mean_abs\tscan_start\tscan_end\tscan_step\tout_dir\tnotes",
                        f"same\t{reference}\t\t{same}\t0\tauto\t0\t\t\t\t\tpixel-identical",
                        f"different\t{reference}\t\t{different}\t0\tauto\t1\t\t\t\t\tintentional failure",
                        f"unchecked\t{reference}\t\t{different}\t0\tauto\t\t\t\t\t\tno threshold yet",
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

    def test_batch_scenarios_parse_reference_manifest_column(self) -> None:
        with tempfile.TemporaryDirectory(prefix="commander-blood-oracle-test-") as tmp:
            root = Path(tmp)
            reference = root / "frame_01.png"
            generated = root / "generated.png"
            manifest = root / "capture-manifest.tsv"
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (10, 20, 30)).save(reference)
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (10, 20, 30)).save(generated)
            manifest.write_text(
                "\n".join(
                    [
                        "frame\tpath\telapsed_s\tepoch_s\tdisplay\tcapture_kind\tcrop_x\tcrop_y\tcrop_w\tcrop_h\tnative_w\tnative_h",
                        f"frame_01.png\t{reference}\t1\t123456\t:98\thost-root\t0\t0\t320\t200\t320\t200",
                    ]
                )
                + "\n"
            )

            scenario_file = root / "scenarios.tsv"
            scenario_file.write_text(
                "\n".join(
                    [
                        "scenario_id\treference\treference_manifest\tgenerated\tgenerated_time\tref_crop\tmax_mean_abs\tscan_start\tscan_end\tscan_step\tout_dir\tnotes",
                        f"manifested\tframe_01.png\t{manifest}\t{generated}\t0\tauto\t0\t\t\t\t\tmanifest crop",
                    ]
                )
                + "\n"
            )

            scenarios = compare_oracle.load_scenarios(scenario_file)
            self.assertEqual(scenarios[0].reference, Path("frame_01.png"))
            self.assertEqual(scenarios[0].reference_manifest, manifest)

            results, exit_code = compare_oracle.run_scenarios(
                scenarios,
                out_root=root / "comparisons",
            )
            self.assertEqual(exit_code, 0)
            self.assertEqual(results[0]["status"], "pass")
            self.assertEqual(results[0]["reference_manifest"]["path"], str(reference))

    def test_thresholded_scans_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory(prefix="commander-blood-oracle-test-") as tmp:
            root = Path(tmp)
            reference = root / "reference.png"
            generated = root / "generated.png"
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (10, 20, 30)).save(reference)
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (10, 20, 30)).save(generated)

            with self.assertRaisesRegex(ValueError, "fixed generated timestamp"):
                compare_oracle.scan_generated_times(
                    reference,
                    generated,
                    start=0.0,
                    end=1.0,
                    step=1.0,
                    ref_crop="auto",
                    out_dir=root / "comparison",
                    max_mean_abs=0.0,
                    scenario_id="thresholded-scan",
                )

    def test_timeline_scan_uses_sidecar_event_times(self) -> None:
        with tempfile.TemporaryDirectory(prefix="commander-blood-oracle-test-") as tmp:
            root = Path(tmp)
            reference = root / "reference.png"
            generated = root / "generated.png"
            timeline = root / "generated.timeline.tsv"
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (10, 20, 30)).save(reference)
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (10, 20, 30)).save(generated)
            timeline.write_text(
                "\n".join(
                    [
                        TIMELINE_HEADER,
                        "generated.png\ttest\t0\t0.000000\t1.500000\t1.500000\t0.250000\t1.583333\t0x000a\t3\tfalse\t\t\t\tfalse\t\ttrue\tabc",
                    ]
                )
                + "\n"
            )

            self.assertEqual(
                compare_oracle.load_timeline_times(timeline),
                [0.0, 0.25, 1.5, 1.583333],
            )
            metrics = compare_oracle.scan_generated_timeline(
                reference,
                generated,
                generated_timeline=timeline,
                ref_crop="auto",
                out_dir=root / "comparison",
            )

            self.assertEqual(metrics["status"], "unchecked")
            self.assertEqual(metrics["scan_source"], "timeline")
            self.assertEqual(metrics["generated_timeline"], str(timeline))
            self.assertEqual(metrics["scan_count"], 4)
            self.assertEqual(metrics["best_generated_time"], 0.0)
            self.assertTrue((root / "comparison" / "scan.json").exists())

    def test_batch_scenarios_parse_generated_timeline_column(self) -> None:
        with tempfile.TemporaryDirectory(prefix="commander-blood-oracle-test-") as tmp:
            root = Path(tmp)
            reference = root / "reference.png"
            generated = root / "generated.png"
            timeline = root / "generated.timeline.tsv"
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (10, 20, 30)).save(reference)
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (10, 20, 30)).save(generated)
            timeline.write_text(
                "\n".join(
                    [
                        TIMELINE_HEADER,
                        "generated.png\ttest\t0\t0.000000\t0.500000\t0.500000\t0.250000\t0.500000\t0x000a\t3\tfalse\t\t\t\tfalse\t\tfalse\tabc",
                    ]
                )
                + "\n"
            )

            scenario_file = root / "scenarios.tsv"
            scenario_file.write_text(
                "\n".join(
                    [
                        "scenario_id\treference\treference_manifest\tgenerated\tgenerated_timeline\tgenerated_time\tref_crop\tmax_mean_abs\tscan_start\tscan_end\tscan_step\tout_dir\tnotes",
                        f"timeline\t{reference}\t\t{generated}\t{timeline}\t0\tauto\t\t\t\t\t\ttimeline scan",
                    ]
                )
                + "\n"
            )

            scenarios = compare_oracle.load_scenarios(scenario_file)
            self.assertEqual(scenarios[0].generated_timeline, timeline)
            results, exit_code = compare_oracle.run_scenarios(
                scenarios,
                out_root=root / "comparisons",
            )

            self.assertEqual(exit_code, 0)
            self.assertEqual(results[0]["status"], "unchecked")
            self.assertEqual(results[0]["scan_source"], "timeline")

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

    def test_candidate_glob_cli_at_fixed_time_without_scan(self) -> None:
        # Regression: `--candidate-glob` without `--scan-generated` or
        # `--candidate-timeline` must rank candidates at the fixed
        # `--generated-time` instead of crashing in parse_scan_range(None).
        import io
        import json as _json
        import sys as _sys
        from contextlib import redirect_stdout

        with tempfile.TemporaryDirectory(prefix="commander-blood-candidate-cli-") as tmp:
            root = Path(tmp)
            reference = root / "reference.png"
            close = root / "candidate-close.png"
            far = root / "candidate-far.png"
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (10, 20, 30)).save(reference)
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (11, 20, 30)).save(close)
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (100, 20, 30)).save(far)

            argv = [
                "compare_oracle.py",
                "--reference",
                str(reference),
                "--candidate-glob",
                str(root / "candidate-*.png"),
                "--candidate-top",
                "2",
                "--out-dir",
                str(root / "candidate-search"),
            ]
            saved = _sys.argv
            buf = io.StringIO()
            try:
                _sys.argv = argv
                with redirect_stdout(buf):
                    exit_code = compare_oracle.main()
            finally:
                _sys.argv = saved

            self.assertEqual(exit_code, 0)
            summary = _json.loads(buf.getvalue())
            self.assertEqual(summary["best"]["generated"], str(close))
            self.assertEqual(summary["scan_start"], 0.0)

    def test_candidate_search_can_use_candidate_timelines(self) -> None:
        with tempfile.TemporaryDirectory(prefix="commander-blood-candidate-test-") as tmp:
            root = Path(tmp)
            reference = root / "reference.png"
            close = root / "candidate-close.png"
            far = root / "candidate-far.png"
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (10, 20, 30)).save(reference)
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (11, 20, 30)).save(close)
            Image.new("RGB", compare_oracle.NATIVE_SIZE, (100, 20, 30)).save(far)
            for candidate in [close, far]:
                compare_oracle.default_generated_timeline(candidate).write_text(
                    "\n".join(
                        [
                            TIMELINE_HEADER,
                            f"{candidate.name}\ttest\t0\t0.000000\t0.500000\t0.500000\t0.250000\t0.500000\t0x000a\t3\tfalse\t\t\t\tfalse\t\tfalse\tabc",
                        ]
                    )
                    + "\n"
                )

            summary = compare_oracle.search_candidate_videos(
                reference,
                [far, close],
                start=None,
                end=None,
                step=None,
                ref_crop="auto",
                out_dir=root / "candidate-timeline-search",
                top_n=2,
                candidate_timeline="auto",
            )

            self.assertEqual(summary["scan_source"], "candidate_timeline")
            self.assertEqual(summary["candidate_timeline"], "auto")
            self.assertEqual(summary["best"]["generated"], str(close))
            self.assertEqual(summary["best"]["scan_source"], "timeline")
            self.assertEqual(
                summary["best"]["generated_timeline"],
                str(compare_oracle.default_generated_timeline(close)),
            )
            self.assertEqual(summary["best"]["scan_count"], 3)
            self.assertTrue(
                (root / "candidate-timeline-search" / "candidate-search.json").exists()
            )

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
