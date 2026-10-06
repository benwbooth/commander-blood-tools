import gzip
import json
from pathlib import Path
import tempfile
import unittest

from native_capture_storage import compact_chapter, open_trace, trace_digest
from video_anthology import digest


class StorageTests(unittest.TestCase):
    def chapter(self, root):
        for name, data in (("master.mkv", b"verified master"), ("video.mkv", b"redundant video"),
                           ("audio.f32le", b"pcm"),
                           ("native-state.jsonl", b'{"frame":0}\n' * 1000)):
            (root / name).write_bytes(data)
        report = dict(complete_native_presentation=True, decoded_master_verified=True,
                      video_timestamps_verified=True, audio_f32le_sha256=digest(root / "audio.f32le"),
                      rgba_sha256="verified-rgba")
        (root / "report.json").write_text(json.dumps(report))
        return dict(path=str(root), master_sha256=digest(root / "master.mkv"),
                    report_sha256=digest(root / "report.json"),
                    audio_sha256=report["audio_f32le_sha256"], rgba_sha256=report["rgba_sha256"],
                    native_state_sha256=digest(root / "native-state.jsonl"))

    def test_dry_run_is_read_only_and_apply_is_round_trip_checked_and_idempotent(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            entry = self.chapter(root)
            before = {path.name: path.read_bytes() for path in root.iterdir()}
            audit = compact_chapter(entry)
            self.assertGreater(audit["redundant_media_bytes"], 0)
            self.assertEqual({path.name: path.read_bytes() for path in root.iterdir()}, before)
            result = compact_chapter(entry, apply=True)
            self.assertGreater(result["saved_bytes"], 0)
            self.assertEqual((root / "master.mkv").read_bytes(), before["master.mkv"])
            self.assertFalse((root / "audio.f32le").exists())
            self.assertFalse((root / "video.mkv").exists())
            with open_trace(root / "native-state.jsonl", "rb") as trace:
                self.assertEqual(trace.read(), before["native-state.jsonl"])
            self.assertEqual(trace_digest(root / "native-state.jsonl"), entry["native_state_sha256"])
            self.assertEqual(compact_chapter(entry, apply=True)["saved_bytes"], 0)

    def test_changed_master_pcm_report_or_trace_prevents_deletion(self):
        for name in ("master.mkv", "audio.f32le", "report.json", "native-state.jsonl"):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                entry = self.chapter(root)
                with (root / name).open("ab") as output:
                    output.write(b" ")
                with self.assertRaises(ValueError):
                    compact_chapter(entry, apply=True)
                self.assertTrue((root / "video.mkv").exists())
                self.assertTrue((root / "audio.f32le").exists())
                self.assertTrue((root / "native-state.jsonl").exists())

    def test_conflicting_compressed_trace_is_not_overwritten(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            entry = self.chapter(root)
            with gzip.open(root / "native-state.jsonl.gz", "wb") as output:
                output.write(b"wrong trace")
            with self.assertRaisesRegex(ValueError, "differs"):
                compact_chapter(entry, apply=True)
            self.assertTrue((root / "native-state.jsonl").exists())
            self.assertTrue((root / "audio.f32le").exists())

    def test_failed_and_symlinked_captures_are_ineligible(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            entry = self.chapter(root)
            report = json.loads((root / "report.json").read_text())
            report["decoded_master_verified"] = False
            (root / "report.json").write_text(json.dumps(report))
            with self.assertRaisesRegex(ValueError, "lacks verified"):
                compact_chapter(entry, apply=True)
            (root / "video.mkv").unlink()
            (root / "video.mkv").symlink_to(root / "master.mkv")
            with self.assertRaisesRegex(ValueError, "symlink"):
                compact_chapter(entry, apply=True)


if __name__ == "__main__":
    unittest.main()
