#!/usr/bin/env python3

from __future__ import annotations

import csv
import tempfile
import unittest
from pathlib import Path

import retrofit_capture_manifest


class RetrofitCaptureManifestTests(unittest.TestCase):
    def test_write_manifest_sorts_frames_and_records_crop_metadata(self) -> None:
        with tempfile.TemporaryDirectory(prefix="commander-blood-retrofit-test-") as tmp:
            root = Path(tmp)
            capture_dir = root / "captures"
            capture_dir.mkdir()
            for name in ["frame_10.png", "frame_02.png", "frame_01.png"]:
                (capture_dir / name).write_bytes(b"png placeholder")

            manifest = root / "capture-manifest.tsv"
            count = retrofit_capture_manifest.write_capture_manifest(
                capture_dir,
                manifest,
                crop=(1, 2, 3, 4),
                native_size=(5, 6),
                interval_s=2.5,
                display=":98",
                capture_kind="host-root-retrofit",
                epoch_s=123,
            )

            self.assertEqual(count, 3)
            with manifest.open(newline="") as f:
                rows = list(csv.DictReader(f, delimiter="\t"))

            self.assertEqual(
                [row["frame"] for row in rows],
                ["frame_01.png", "frame_02.png", "frame_10.png"],
            )
            self.assertEqual(rows[0]["elapsed_s"], "2.5")
            self.assertEqual(rows[1]["elapsed_s"], "5")
            self.assertEqual(rows[2]["elapsed_s"], "7.5")
            self.assertEqual(rows[0]["display"], ":98")
            self.assertEqual(rows[0]["capture_kind"], "host-root-retrofit")
            self.assertEqual(
                [
                    rows[0]["crop_x"],
                    rows[0]["crop_y"],
                    rows[0]["crop_w"],
                    rows[0]["crop_h"],
                ],
                ["1", "2", "3", "4"],
            )
            self.assertEqual([rows[0]["native_w"], rows[0]["native_h"]], ["5", "6"])
            self.assertEqual(rows[0]["epoch_s"], "123")
            self.assertEqual(
                rows[0]["path"],
                str((capture_dir / "frame_01.png").resolve()),
            )

    def test_main_writes_default_manifest_path(self) -> None:
        with tempfile.TemporaryDirectory(prefix="commander-blood-retrofit-test-") as tmp:
            capture_dir = Path(tmp) / "captures"
            capture_dir.mkdir()
            (capture_dir / "frame_01.png").write_bytes(b"png placeholder")

            exit_code = retrofit_capture_manifest.main(
                [
                    str(capture_dir),
                    "--crop",
                    "10,20,320,200",
                    "--native-size",
                    "320,200",
                    "--interval",
                    "4",
                    "--display",
                    ":99",
                ]
            )

            self.assertEqual(exit_code, 0)
            manifest = capture_dir / "capture-manifest.tsv"
            self.assertTrue(manifest.exists())
            with manifest.open(newline="") as f:
                rows = list(csv.DictReader(f, delimiter="\t"))
            self.assertEqual(rows[0]["elapsed_s"], "4")
            self.assertEqual(rows[0]["display"], ":99")

    def test_empty_capture_dir_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory(prefix="commander-blood-retrofit-test-") as tmp:
            with self.assertRaisesRegex(ValueError, "no frame_\\*.png"):
                retrofit_capture_manifest.write_capture_manifest(
                    Path(tmp),
                    Path(tmp) / "capture-manifest.tsv",
                )

    def test_native_size_dimensions_must_be_positive(self) -> None:
        with self.assertRaisesRegex(SystemExit, "2"):
            retrofit_capture_manifest.main(["--native-size", "0,200"])


if __name__ == "__main__":
    unittest.main()
