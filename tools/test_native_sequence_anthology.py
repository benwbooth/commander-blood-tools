import copy
import unittest

from native_sequence_anthology import concatenate_timelines, validate_playlist, validate_timing


class NativeSequenceTests(unittest.TestCase):
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
