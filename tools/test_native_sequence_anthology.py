import copy
import json
import hashlib
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

from native_sequence_anthology import (assembly_entries, concatenate_timelines,
                                       dialogue_source_order, replace_travel_entries,
                                       validate_playlist, validate_timing, mux_assembly,
                                       verify_media)
from video_anthology import digest, metadata_text


class NativeSequenceTests(unittest.TestCase):
    @unittest.skipUnless(shutil.which("ffmpeg") and shutil.which("ffprobe"), "requires ffmpeg/ffprobe")
    def test_streamed_audio_assembly_preserves_mixed_cadence_and_single_frame_tail(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            listing, timelines, chapters = [], [], []
            audio = hashlib.sha256()
            video = hashlib.sha256()
            offset = 0
            for index, durations in enumerate(([46_000_000, 68_000_000], [68_000_000])):
                duration = sum(durations)
                path = root / f"part{index}.mkv"
                starts = [sum(durations[:i]) for i in range(len(durations))]
                select = "+".join(f"eq(n\\,{start // 1_000_000})" for start in starts)
                subprocess.run(["ffmpeg", "-nostdin", "-v", "error", "-f", "lavfi", "-i",
                                f"color=c=red:size=16x16:rate=1000:duration={duration / 1e9}",
                                "-f", "lavfi", "-i", f"sine=frequency={440 + index}:sample_rate=48000:duration={duration / 1e9}",
                                "-vf", f"select={select}", "-fps_mode", "vfr", "-c:v", "ffv1",
                                "-pix_fmt", "bgra", "-c:a", "pcm_f32le", str(path)], check=True)
                decoded = {}
                for kind, format_name, codec, hasher in (("a", "f32le", "pcm_f32le", audio),
                                                        ("v", "rawvideo", "rawvideo", video)):
                    args = ["ffmpeg", "-nostdin", "-v", "error", "-i", str(path),
                            "-map", f"0:{kind}:0", f"-c:{kind}", codec]
                    if kind == "v":
                        args += ["-pix_fmt", "rgba", "-fps_mode", "passthrough"]
                    decoded[kind] = subprocess.check_output(args + ["-f", format_name, "pipe:1"])
                    hasher.update(decoded[kind])
                rows = [dict(start_ns=start, duration_ns=length,
                             sample_start=start * 48000 // 1_000_000_000,
                             sample_count=length * 48000 // 1_000_000_000)
                        for start, length in zip(starts, durations)]
                for frame_index, row in enumerate(rows):
                    row["rgba_sha256"] = hashlib.sha256(decoded["v"][frame_index * 1024:(frame_index + 1) * 1024]).hexdigest()
                    first_sample = row["sample_start"]
                    row["audio_sha256"] = hashlib.sha256(decoded["a"][first_sample * 4:(first_sample + row["sample_count"]) * 4]).hexdigest()
                timelines.append((rows, duration, duration * 48000 // 1_000_000_000))
                listing += [f"file '{path}'", f"duration {duration / 1e9:.9f}"]
                chapters.append(dict(start_ns=offset, end_ns=offset + duration, title=f"Part {index}"))
                offset += duration
            (root / "concat.txt").write_text("\n".join(listing) + "\n")
            (root / "chapters.ffmeta").write_text(metadata_text(chapters))
            rows, duration, _ = concatenate_timelines(timelines)
            mux_assembly(root / "concat.txt", root / "chapters.ffmeta", root / "master.mkv", rows)
            hashes, encoded_chapters = verify_media(root / "master.mkv", rows, duration, width=16, height=16)
            self.assertEqual(hashes["audio_sha256"], audio.hexdigest())
            self.assertEqual(hashes["rgba_sha256"], video.hexdigest())
            self.assertEqual(len(encoded_chapters), 2)
            self.assertFalse((root / "audio.f32le").exists())

    def test_travel_replacement_requires_every_original_plan_and_keeps_order(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            entries = [dict(record="opening", path=str(root / "opening"))]
            plan = dict(title="travel", entry="travel", initial_profile=3)
            for title, chapter in (("travel", plan), ("contact", dict(title="contact", entry="contact"))):
                path = root / title
                path.mkdir()
                (path / "report.json").write_text(json.dumps(dict(runner=dict(chapter=chapter))))
                entries.append(dict(record=title, path=str(path),
                                    report_sha256=digest(path / "report.json")))
            base = dict(game="commander_blood", sources=entries,
                        chapters=[dict(title=entry["record"]) for entry in entries])
            new = root / "new"
            new.mkdir()
            report = dict(runner=dict(chapter=plan, travel_music="ITE2.VOC"))
            (new / "report.json").write_text(json.dumps(report))
            replacement = dict(record="travel", path=str(new),
                               report_sha256=digest(new / "report.json"))
            batch = root / "batch"
            batch.mkdir()
            coverage = dict(complete=True, failures=[], provenance=dict(
                game="commander_blood", records=[dict(name="travel")]), completed=[replacement])
            (batch / "coverage.json").write_text(json.dumps(coverage))
            game, result = replace_travel_entries(base, [batch])
            self.assertEqual(game, "commander_blood")
            self.assertEqual([entry["record"] for entry in result], ["opening", "travel", "contact"])
            self.assertEqual(result[1]["path"], str(new))
            self.assertEqual(result[2]["path"], str(root / "contact"))
            coverage["completed"] = []
            coverage["provenance"]["records"] = []
            (batch / "coverage.json").write_text(json.dumps(coverage))
            with self.assertRaisesRegex(ValueError, "missing 1 travel replacement"):
                replace_travel_entries(base, [batch])
            coverage["completed"] = [replacement]
            coverage["provenance"]["records"] = [dict(name="travel")]
            (batch / "coverage.json").write_text(json.dumps(coverage))
            report["runner"]["chapter"]["initial_profile"] = 4
            (new / "report.json").write_text(json.dumps(report))
            replacement["report_sha256"] = digest(new / "report.json")
            (batch / "coverage.json").write_text(json.dumps(coverage))
            with self.assertRaisesRegex(ValueError, "travel plan changed"):
                replace_travel_entries(base, [batch])

    def test_dialogue_source_order_keeps_sequence_prefix_and_stable_branches(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            entries = [dict(record="opening", path=str(root / "opening"))]
            for name, profile, anchor in [("late", 1, 500), ("first", 0, 120),
                                          ("middle", 1, 100), ("late answer", 1, 500)]:
                path = root / name
                path.mkdir()
                plan = dict(title=name, initial_profile=profile,
                            travel_setup=dict(procedure_offset=anchor))
                (path / "report.json").write_text(json.dumps(dict(
                    target="dialogue_chapter", runner=dict(chapter=plan))))
                entries.append(dict(record=name, path=str(path),
                                    report_sha256=digest(path / "report.json")))
            ordered = dialogue_source_order(entries)
            self.assertEqual([entry["record"] for entry in ordered],
                             ["opening", "first", "middle", "late", "late answer"])
            (root / "middle" / "report.json").write_text("{}")
            with self.assertRaisesRegex(ValueError, "report changed"):
                dialogue_source_order(entries)

    def test_assembly_collects_complete_same_game_batches_without_duplicate_captures(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            batches = [root / "one", root / "two"]
            for index, batch in enumerate(batches):
                batch.mkdir()
                coverage = dict(complete=True, failures=[], provenance=dict(game="commander_blood",
                                records=[dict(name=f"chapter {index}")]),
                                completed=[dict(record=f"chapter {index}", path=str(root / f"capture-{index}"))])
                (batch / "coverage.json").write_text(json.dumps(coverage))
            game, entries = assembly_entries(batches)
            self.assertEqual(game, "commander_blood")
            self.assertEqual([entry["record"] for entry in entries], ["chapter 0", "chapter 1"])
            changed = json.loads((batches[1] / "coverage.json").read_text())
            changed["provenance"]["game"] = "big_bug_bang"
            (batches[1] / "coverage.json").write_text(json.dumps(changed))
            with self.assertRaisesRegex(ValueError, "different games"):
                assembly_entries(batches)
            changed["provenance"]["game"] = "commander_blood"
            changed["completed"][0]["path"] = str(root / "capture-0")
            (batches[1] / "coverage.json").write_text(json.dumps(changed))
            with self.assertRaisesRegex(ValueError, "duplicate native chapter"):
                assembly_entries(batches)
            changed["completed"][0]["path"] = str(root / "capture-1")
            changed["complete"] = False
            (batches[1] / "coverage.json").write_text(json.dumps(changed))
            with self.assertRaisesRegex(ValueError, "incomplete"):
                assembly_entries(batches)

    def test_native_timestamps_accept_mixed_intervals(self):
        rows = [{"start_ns": 0}, {"start_ns": 46_000_000}, {"start_ns": 114_000_000}]
        probe = {"streams": [{"time_base": "1/1000"}],
                 "frames": [{"best_effort_timestamp": value} for value in [0, 46, 114]],
                 "format": {"duration": "0.182000"}}
        validate_timing(probe, rows, 182_000_000)
        for index in range(3):
            changed = copy.deepcopy(probe)
            changed["frames"][index]["best_effort_timestamp"] += 1
            with self.assertRaises(ValueError):
                validate_timing(changed, rows, 182_000_000)
        with self.assertRaises(ValueError):
            validate_timing(probe, rows, 183_000_000)
        with self.assertRaises(ValueError):
            validate_timing(probe, rows[:-1], 182_000_000)

    def test_join_offsets_every_native_interval_without_resampling(self):
        rows = [dict(frame=0, start_ns=0, duration_ns=46_000_000, sample_start=0,
                     sample_count=2208, rgba_sha256="pixels", audio_sha256="sound")]
        joined, duration, samples = concatenate_timelines([(rows, 46_000_000, 2208)] * 2)
        self.assertEqual(duration, 92_000_000)
        self.assertEqual(samples, 4416)
        self.assertEqual(joined[1], dict(rows[0], frame=1, start_ns=46_000_000, sample_start=2208))
        self.assertEqual(rows[0]["start_ns"], 0)

    def test_observed_playlist_requires_full_order_and_keeps_repeated_clips(self):
        record = {"references": [{"role": "sequence", "resource": "SQ/A.HNM"},
                                 {"role": "music", "resource": "MU/A.VOC"},
                                 {"role": "sequence", "resource": "SQ/B.HNM"},
                                 {"role": "sequence", "resource": "SQ/A.HNM"}]}
        report = {"runner": {"authored_videos": ["a.hnm", "b.hnm", "a.hnm"]}}

        def states(names):
            return [{"state": {"video": {"active_resource": name}}} for name in names]

        observed = states([None, "SQ\\a.hnm", "SQ\\a.hnm", None, "SQ\\b.hnm", "SQ\\a.hnm", None])
        self.assertEqual(validate_playlist(record, report, observed)["observed_clips"], ["SQ/A.HNM", "SQ/B.HNM", "SQ/A.HNM"])
        for wrong in (["SQ/A.HNM"], ["SQ/B.HNM", "SQ/A.HNM", "SQ/A.HNM"],
                      ["SQ/A.HNM", "SQ/B.HNM", "SQ/A.HNM", "SQ/C.HNM"]):
            with self.assertRaises(ValueError):
                validate_playlist(record, report, states(wrong))
        report["runner"]["authored_videos"].reverse()
        report["runner"]["authored_videos"].pop()
        with self.assertRaises(ValueError):
            validate_playlist(record, report, observed)

    def test_missing_authored_source_is_explicit_and_never_substituted(self):
        record = {"references": [{"role": "sequence", "resource": "SQ/MISSING.HNM"}]}
        report = {"runner": {"authored_videos": ["missing.hnm"]}}
        result = validate_playlist(record, report, [], set())
        self.assertEqual(result["missing_authored_resources"], ["SQ/MISSING.HNM"])
        self.assertEqual(result["observed_clips"], [])
        with self.assertRaises(ValueError):
            validate_playlist(record, report, [], {"SQ/MISSING.HNM"})

    def test_repeated_filename_uses_native_list_position(self):
        record = {"references": [{"role": "sequence", "resource": "SQ/A.HNM"}] * 2}
        report = {"runner": {"authored_videos": ["a.hnm"] * 2}}
        states = [{"state": {"video": {"active_resource": "SQ/A.HNM"},
                             "sequence_caption": {"remaining_scene_lines": remaining}}}
                  for remaining in [1, 1, 0, 0]]
        self.assertEqual(validate_playlist(record, report, states)["observed_clips"], ["SQ/A.HNM"] * 2)

    def test_caption_clock_restarts_and_dropped_reachable_cues_fail(self):
        record = {"references": [{"role": "sequence", "resource": "SQ/A.HNM"}]}
        report = {"runner": {"authored_videos": ["a.hnm"],
                             "caption_cues": [{"authored_frame": 1, "display_text": "Hello"}]}}
        states = [{"time_ns": 0, "state": {"video": {"active_resource": "SQ/A.HNM",
                   "queue_metrics": {"sequence_index": 3}}, "sequence_caption": {"cue_index": 0}}}]
        self.assertEqual(validate_playlist(record, report, states)["caption_first_native_state_ns"], {0: 0})
        missing = copy.deepcopy(states)
        missing[0]["state"]["sequence_caption"]["cue_index"] = None
        with self.assertRaises(ValueError):
            validate_playlist(record, report, missing)
        restart = copy.deepcopy(states[0])
        restart["state"]["video"]["queue_metrics"]["sequence_index"] = 1
        with self.assertRaises(ValueError):
            validate_playlist(record, report, states + [restart])


if __name__ == "__main__":
    unittest.main()
