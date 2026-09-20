import json
from fractions import Fraction
from pathlib import Path
import struct
import tempfile
import unittest

import video_anthology as anthology


class AnthologyTests(unittest.TestCase):
    def test_category_hints_do_not_claim_reachability(self):
        self.assertEqual(anthology.categorize("PL/TERRA.HNM", []), ["planetary"])
        self.assertEqual(anthology.categorize("UNKNOWN.HNM", []), ["unclassified"])
        refs = [{"role": "talk"}, {"role": "sequence"}]
        self.assertEqual(anthology.categorize("PE/TALK.HNM", refs), ["conversation", "sequence"])

    def test_transcript_retains_text_through_idle_clip_changes(self):
        scenes = [dict(time_ns=t, state=dict(subtitle=text, resource=resource, profile=0))
                  for t, text, resource in [(10, "hello", "a"), (20, "hello", "b"),
                                             (30, "", "b"), (40, "bye", "b")]]
        turns = anthology.transcript(scenes, 50)
        self.assertEqual([(t["start_ns"], t["end_ns"], t["text"]) for t in turns],
                         [(10, 30, "hello"), (40, 50, "bye")])

    def test_inline_spoken_words_are_transcribed_without_choice_labels(self):
        scenes = [dict(time_ns=10, state=dict(subtitle="", inline_dialogue="Hello Bob", choices=["bye"])),
                  dict(time_ns=20, state=dict(subtitle="Answer\rHere", inline_dialogue="Hello Bob"))]
        self.assertEqual([t["text"] for t in anthology.transcript(scenes, 30)],
                         ["Hello Bob", "Answer\nHere"])

    def test_timestamped_pcm_preserves_source_samples_and_callback_times(self):
        import av

        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            samples = struct.pack("<8f", 0.0, 0.0, 0.25, -0.25, 0.5, -0.5, 0.75, -0.75)
            (root / "audio.f32le").write_bytes(samples)
            events = [dict(kind="audio", sample=2, samples=4, time_ns=100_000_000),
                      dict(kind="audio", sample=6, samples=2, time_ns=200_000_000)]
            (root / "timeline.jsonl").write_text("\n".join(map(json.dumps, events)))
            path = anthology.timestamp_audio(root, 48000)
            with av.open(str(path)) as media:
                decoded = list(media.decode(audio=0))
                self.assertEqual([f.pts * f.time_base for f in decoded], [Fraction(1, 10), Fraction(1, 5)])
                self.assertEqual(b"".join(bytes(f.planes[0])[:f.samples * 4] for f in decoded), samples[8:])

    def test_partial_route_is_never_accepted_as_complete(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "scenario.tsv").write_text("# route\nwait 10\n\nwait 20\n")
            trace = root / "runtime-trace.jsonl"
            trace.write_text(json.dumps(dict(action_index=1)) + "\n")
            with self.assertRaisesRegex(ValueError, "before completing"):
                anthology.verify_route_complete(root)
            trace.write_text(json.dumps(dict(action_index=2)) + "\n")
            anthology.verify_route_complete(root)

    def test_srt_keeps_hours_and_milliseconds(self):
        result = anthology.srt_text([dict(start_ns=3_661_002_000_000,
                                          end_ns=3_662_003_000_000, text="hello")])
        self.assertIn("01:01:01,002 --> 01:01:02,003", result)

    def test_chapter_escaping_prevents_metadata_injection(self):
        result = anthology.metadata_text([dict(start_ns=0, end_ns=12,
                                                title="Bob=one;#two\n[CHAPTER]")])
        self.assertIn("title=Bob\\=one\\;\\#two [CHAPTER]", result)
        self.assertEqual(result.count("\n[CHAPTER]\n"), 1)

    def test_manifest_hash_is_not_enough_to_validate_assets(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source = root / "movie.HNM"
            source.write_bytes(b"abc")
            manifest = dict(schema_version=1, resources=[dict(path=source.name,
                              byte_count=3, sha256=anthology.digest(source))])
            anthology.save_json(root / "manifest.json", manifest)
            self.assertEqual(anthology.inventory(root)[0], manifest)
            source.write_bytes(b"abd")
            with self.assertRaisesRegex(ValueError, "integrity"):
                anthology.inventory(root)

    def test_asset_paths_cannot_escape_store(self):
        with tempfile.TemporaryDirectory() as folder:
            with self.assertRaisesRegex(ValueError, "escapes"):
                anthology.asset_path(Path(folder), "../elsewhere")

    def test_companion_bytes_are_also_verified(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source = root / "GAME.EXE"
            source.write_bytes(b"original")
            anthology.save_json(root / "manifest.json", dict(schema_version=1, resources=[],
                                companions=[dict(path=source.name, byte_count=8, sha256=anthology.digest(source))]))
            source.write_bytes(b"modified")
            with self.assertRaisesRegex(ValueError, "integrity"):
                anthology.inventory(root)

    def test_missing_or_changed_master_invalidates_resume(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source = root / "master.mkv"
            source.write_bytes(b"test-data-for-hash-only")
            state = dict(artifacts={source.name: anthology.digest(source)})
            self.assertTrue(anthology.check_artifacts(root, state))
            source.write_bytes(b"changed")
            self.assertFalse(anthology.check_artifacts(root, state))
            source.unlink()
            self.assertFalse(anthology.check_artifacts(root, state))
            self.assertFalse(anthology.check_artifacts(root, dict(artifacts={})))

    def test_job_ids_are_unique_and_cannot_escape_output(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "jobs.json"
            job = dict(id="test", game="cb", category="conversation", title="Bob")
            anthology.save_json(path, dict(schema=1, jobs=[job, job]))
            with self.assertRaisesRegex(ValueError, "duplicate"):
                anthology.load_jobs(path)
            job["id"] = "../outside"
            anthology.save_json(path, dict(schema=1, jobs=[job]))
            with self.assertRaisesRegex(ValueError, "invalid"):
                anthology.load_jobs(path)


if __name__ == "__main__":
    unittest.main()
