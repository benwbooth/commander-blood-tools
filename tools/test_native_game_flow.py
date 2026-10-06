import copy
import io
import json
from pathlib import Path
import tempfile
import struct
import unittest

from native_game_flow import (FlowRecorder, normal_actions, route_source, saved_checkpoints,
                             verified_predecessor)
from video_anthology import digest


def frame(number=0):
    return dict(schema=1, executable="modern-rust", frame=number,
                elapsed_ns=number * 1000, boundary="game", semantic=dict(
                    vm=dict(resource_profile=0, displayed_line=9, execution_enabled=1),
                    published_cod_text_site=42, published_bas_text_site=None,
                    presentation=dict(active_actor_presentation={"name": "Bob"},
                                      rendered_word_choices=[], waiting_for_input=False,
                                      active=1, screen_active=True,
                                      retained_word_choice={"phase": "Closed"},
                                      ship_scene={"dispatch_blocked": False},
                                      text_display_active=1,
                                      text_state={"subtitle_reveal_cursor": 1}),
                    video=dict(active_resource="BOB.HNM", source_open_or_draining=True,
                               decoded_frame_count=1),
                    subtitle_bytes=list(b"Hello"),
                    subtitle_raster=dict(expected_pixel_count=5, matching_pixel_count=5),
                    inline_menu_raster=None,
                    descript={"music": "BOB.VOC"}, navigation={"target": None},
                    audio={"loaded_navigation_music": None},
                    persistent=dict(object_locations=[dict(record=1, holder_raw=22)],
                                    state_array_hash="a", character_slots_hash="b"),
                    save_load={"completed_loads": 0}))


class FlowTests(unittest.TestCase):
    def checkpoint_fixture(self, root):
        root.mkdir()
        writable = root / "writable"
        writable.mkdir()
        (writable / "BLOOD.SAV").write_bytes(b"".join(
            struct.pack("<16s16s", b"flow", f"game{i + 1}.sav".encode()) for i in range(10)))
        (writable / "GAME1.SAV").write_bytes(b"unit-test save bytes")
        (root / "scenario.tsv").write_text("click 1 2\nwait 10\n")
        (root / "actions.jsonl").write_text("[]\n")
        (root / "game.log").write_text("test fixture\n")
        (root / "flow.md").write_text("unit-test fixture, not gameplay evidence\n")
        (root / "events.jsonl").write_text(json.dumps(dict(
            frame=10, state=dict(save_load=dict(completed_saves=1, active_slot=0)))) + "\n")
        provenance = dict(binary_sha256="binary", asset_manifest_sha256="assets", sources={},
                          scenario_sha256=digest(root / "scenario.tsv"))
        status = dict(schema=1, game="cb", status="observed_route", state_injection=False,
                      provenance=provenance, checkpoints=saved_checkpoints(writable, {0: 10}),
                      files={p.name: digest(p) for p in root.iterdir() if p.is_file()})
        manifest = root / "flow.json"
        manifest.write_text(json.dumps(status))
        return manifest, provenance, status

    def test_checkpoint_requires_unchanged_predecessor_and_native_save_event(self):
        with tempfile.TemporaryDirectory() as directory:
            manifest, provenance, status = self.checkpoint_fixture(Path(directory) / "parent")
            checkpoint, parent = verified_predecessor(manifest, 0, "cb", provenance)
            self.assertEqual(checkpoint["filename"], "GAME1.SAV")
            self.assertEqual(parent["manifest_sha256"], digest(manifest))
            with self.assertRaisesRegex(ValueError, "no witnessed save"):
                verified_predecessor(manifest, 1, "cb", provenance)
            with self.assertRaisesRegex(ValueError, "binary_sha256"):
                verified_predecessor(manifest, 0, "cb", {**provenance, "binary_sha256": "different"})
            event = manifest.parent / "events.jsonl"
            event.write_text(json.dumps(dict(frame=10, state=dict(save_load=dict(
                completed_saves=0, active_slot=0)))) + "\n")
            with self.assertRaisesRegex(ValueError, "evidence changed"):
                verified_predecessor(manifest, 0, "cb", provenance)
            status["files"]["events.jsonl"] = digest(event)
            manifest.write_text(json.dumps(status))
            with self.assertRaisesRegex(ValueError, "no matching native save event"):
                verified_predecessor(manifest, 0, "cb", provenance)

    def test_changed_save_and_failed_predecessor_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            manifest, provenance, status = self.checkpoint_fixture(Path(directory) / "parent")
            (manifest.parent / "writable/GAME1.SAV").write_bytes(b"changed")
            with self.assertRaisesRegex(ValueError, "checkpoint or directory changed"):
                verified_predecessor(manifest, 0, "cb", provenance)
            status["status"] = "failed"
            manifest.write_text(json.dumps(status))
            with self.assertRaisesRegex(ValueError, "not a completed normal-input"):
                verified_predecessor(manifest, 0, "cb", provenance)

    def test_only_one_expected_normal_load_is_accepted_and_saved_slots_are_recorded(self):
        with tempfile.TemporaryDirectory() as directory:
            writable = Path(directory)
            path = writable / "GAME1.SAV"
            path.write_bytes(b"test")
            checkpoint = dict(slot=0, filename=path.name, sha256=digest(path))
            recorder = FlowRecorder(io.StringIO(), checkpoint, writable)
            item = frame()
            item["semantic"]["save_load"] = dict(completed_loads=1, completed_saves=0, active_slot=0)
            recorder.record(item)
            self.assertTrue(recorder.loaded_checkpoint)
            item["frame"] = 1
            item["semantic"]["save_load"]["completed_saves"] = 1
            item["semantic"]["save_load"]["active_slot"] = 2
            recorder.record(item)
            self.assertEqual(recorder.saved_slots, {2: 1})
            item["frame"] = 2
            item["semantic"]["save_load"]["completed_loads"] = 2
            with self.assertRaisesRegex(ValueError, "without a validated predecessor"):
                recorder.record(item)

    def test_fragments_preserve_actions_and_each_source_hash(self):
        with tempfile.TemporaryDirectory() as directory:
            paths = [Path(directory) / name for name in ("start.tsv", "next.tsv")]
            paths[0].write_text("# new game\nwait 5")
            paths[1].write_text("choose yes\nwait 10\n")
            source, provenance = route_source(paths)
            self.assertEqual(normal_actions(source), ["wait 5", "choose yes", "wait 10"])
            self.assertEqual(len(provenance), 2)
            self.assertNotEqual(provenance[0]["sha256"], provenance[1]["sha256"])

    def test_only_player_input_and_waits_are_allowed(self):
        self.assertEqual(normal_actions("# new game\nclick 1 2\nwait 100\n"),
                         ["click 1 2", "wait 100"])
        for command in ("teleport Bob", "contact 1 2 0x1234", "alien", "poke 1 2"):
            with self.subTest(command=command), self.assertRaises(ValueError):
                normal_actions(command)

    def test_word_reveal_and_idle_frames_are_compacted_but_completion_is_not(self):
        output = io.StringIO()
        recorder = FlowRecorder(output)
        item = frame()
        recorder.record(item)
        item = copy.deepcopy(item)
        item["frame"] = 1
        item["semantic"]["presentation"]["text_state"]["subtitle_reveal_cursor"] = 2
        item["semantic"]["video"]["decoded_frame_count"] = 2
        recorder.record(item)
        self.assertEqual(recorder.events, 1)
        self.assertEqual(recorder.summary()["fully_revealed_sites"], [])
        item["frame"] = 2
        item["semantic"]["presentation"]["text_state"]["subtitle_reveal_cursor"] = 5
        recorder.record(item)
        self.assertEqual(recorder.events, 2)
        self.assertEqual(recorder.summary()["fully_revealed_sites"],
                         [dict(profile=1, domain="cod", offset=42)])

    def test_loaded_media_are_not_counted_as_played(self):
        recorder = FlowRecorder(io.StringIO())
        item = frame()
        item["semantic"]["video"]["decoded_frame_count"] = 0
        recorder.record(item)
        self.assertEqual(recorder.summary()["decoded_video_resources"], [])
        item["frame"] = 1
        item["semantic"]["video"]["decoded_frame_count"] = 1
        recorder.record(item)
        self.assertEqual(recorder.summary()["decoded_video_resources"], ["BOB.HNM"])

    def test_inline_dialogue_is_not_replaced_by_the_stale_subtitle_buffer(self):
        recorder = FlowRecorder(io.StringIO())
        item = frame()
        s = item["semantic"]
        s["presentation"]["text_display_active"] = 0
        s["presentation"]["inline_menu"] = dict(display_words=["Bob", "speaks"], reveal_count=2)
        s["inline_menu_raster"] = dict(expected_pixel_count=10, matching_pixel_count=10)
        recorder.record(item)
        self.assertEqual(recorder.previous["text"]["content"], ["Bob", "speaks"])
        self.assertEqual(len(recorder.revealed_sites), 1)

    def test_full_reveal_requires_pixels_and_matching_raster(self):
        for raster in (None, dict(expected_pixel_count=0, matching_pixel_count=0)):
            item = frame()
            item["semantic"]["presentation"]["text_state"]["subtitle_reveal_cursor"] = 5
            item["semantic"]["subtitle_raster"] = raster
            recorder = FlowRecorder(io.StringIO())
            recorder.record(item)
            self.assertFalse(recorder.revealed_sites)
        item["semantic"]["subtitle_raster"] = dict(expected_pixel_count=5, matching_pixel_count=4)
        with self.assertRaises(ValueError):
            FlowRecorder(io.StringIO()).record(item)

    def test_inventory_change_and_same_resource_restart_survive_compaction(self):
        output = io.StringIO()
        recorder = FlowRecorder(output)
        item = frame()
        item["semantic"]["video"]["decoded_frame_count"] = 30
        recorder.record(item)
        item = copy.deepcopy(item)
        item["frame"] = 1
        item["semantic"]["persistent"]["object_locations"][0]["holder_raw"] = 65535
        item["semantic"]["video"]["decoded_frame_count"] = 1
        recorder.record(item)
        event = json.loads(output.getvalue().splitlines()[-1])
        self.assertTrue(event["video_restart"])
        self.assertEqual(event["object_changes"]["1"]["holder_raw"], 65535)

    def test_cheats_saves_missing_fields_and_trace_gaps_fail_closed(self):
        for mutation in (lambda x: x.update(frame=2),
                         lambda x: x["semantic"]["save_load"].update(completed_loads=1),
                         lambda x: x["semantic"].update(subtitle_bytes=list(b"CHEAT MODE")),
                         lambda x: x["semantic"].pop("published_cod_text_site")):
            with self.subTest(mutation=mutation), self.assertRaises((ValueError, KeyError)):
                item = frame()
                mutation(item)
                FlowRecorder(io.StringIO()).record(item)


if __name__ == "__main__":
    unittest.main()
