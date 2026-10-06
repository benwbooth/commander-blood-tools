import copy
import io
import json
from pathlib import Path
import tempfile
import struct
import unittest

from native_game_flow import (ActionRecorder, BBB_SUCCESS_ENDING_OFFSET, BBB_SUCCESS_SEQUENCES,
                             CB_CONCERT_SEQUENCES, FlowRecorder, normal_actions, route_source, saved_checkpoints,
                             scene_state, revealed_text_sites, verified_predecessor,
                             verify_route_completion)
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
    def test_action_snapshot_references_preserve_clocks_and_resolve_without_chains(self):
        output = io.StringIO()
        recorder = ActionRecorder(output)
        source = frame()
        source.update(action="move 100 50", action_index=12, phase="after", steps=100,
                      completion_clock=dict(steps=42), semantic_observation_deferred=True)
        original = copy.deepcopy(source)
        recorder.record(source)
        self.assertEqual(source, original)
        source.update(action="key 72", action_index=13, completion_clock=dict(steps=43))
        recorder.record(source)
        source.update(action="key 1", action_index=14, completion_clock=dict(steps=100),
                      semantic_observation_deferred=False)
        recorder.record(source)
        records = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(len(records), 3)
        self.assertIn("scene", records[0])
        for index in (1, 2):
            self.assertEqual(records[index]["snapshot_ref"], dict(record_index=0))
            self.assertNotIn("scene", records[index])
            self.assertEqual(records[index]["schema"], 2)
            self.assertEqual(records[index]["action_index"], 12 + index)
        self.assertEqual(records[1]["completion_clock"], dict(steps=43))
        self.assertEqual(records[2]["completion_clock"], dict(steps=100))
        self.assertFalse(records[2]["semantic_observation_deferred"])
        self.assertEqual(records[0]["scene"], scene_state(original["semantic"]))

    def test_action_snapshot_references_never_hide_state_scene_or_bridge_changes(self):
        output = io.StringIO()
        recorder = ActionRecorder(output)
        source = frame()
        recorder.record(source)
        source["semantic"]["persistent"]["state_array_hash"] = "changed"
        recorder.record(source)
        source["semantic"]["presentation"]["bridge_frame"] = 45
        recorder.record(source)
        source["semantic"]["presentation"]["rendered_word_choices"] = ["yes"]
        recorder.record(source)
        recorder.record(source)
        records = [json.loads(line) for line in output.getvalue().splitlines()]
        for record in records[:4]:
            self.assertIn("scene", record)
            self.assertNotIn("snapshot_ref", record)
        self.assertEqual(records[4]["snapshot_ref"], dict(record_index=3))
        self.assertEqual(records[0]["state_array_hash"], "a")
        self.assertEqual(records[3]["scene"]["choices"], ["yes"])

    def test_global_deltas_are_lossless_without_unchanged_frame_churn(self):
        output = io.StringIO()
        recorder = FlowRecorder(output)
        record = frame()
        record["semantic"]["persistent"]["script_globals"] = {
            "A27": dict(source_offset=7968, raw=4, signed=4),
            "negative": dict(source_offset=10, raw=65535, signed=-1),
        }
        recorder.record(record)
        record["frame"] = 1
        recorder.record(copy.deepcopy(record))
        self.assertEqual(recorder.events, 1)
        record["frame"] = 2
        record["semantic"]["persistent"]["script_globals"]["A27"]["raw"] = 5
        record["semantic"]["persistent"]["script_globals"]["A27"]["signed"] = 5
        del record["semantic"]["persistent"]["script_globals"]["negative"]
        recorder.record(record)
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(events[0]["global_changes"]["negative"]["signed"], -1)
        self.assertEqual(events[1]["global_changes"], {"A27": dict(source_offset=7968, raw=5, signed=5)})
        self.assertEqual(events[1]["removed_globals"], ["negative"])
        self.assertIsNone(events[1]["state"])
        self.assertEqual(recorder.summary()["final_globals"], record["semantic"]["persistent"]["script_globals"])

    def test_old_trace_does_not_claim_global_evidence(self):
        recorder = FlowRecorder(io.StringIO())
        recorder.record(frame())
        self.assertIsNone(recorder.summary()["final_globals"])

    def test_scene_retains_menu_geometry_and_pending_call_without_raster_churn(self):
        semantic = frame()["semantic"]
        presentation = semantic["presentation"]
        presentation["retained_word_choice"]["rows"] = [dict(
            kind="Cancel", item_index=None, position=[180, 117], matching_text_pixels=42)]
        presentation["pending_presentation_owner"] = {"name": "Scruter_K", "record": 26}
        state = scene_state(semantic)
        self.assertEqual(state["choice_rows"], [dict(
            kind="Cancel", item_index=None, position=[180, 117])])
        self.assertEqual(state["pending_call"], {"name": "Scruter_K", "record": 26})
        presentation["retained_word_choice"]["rows"][0]["matching_text_pixels"] = 7
        self.assertEqual(scene_state(semantic), state)

    def test_scene_retains_read_only_selector_diagnostics(self):
        semantic = frame()["semantic"]
        semantic["dialogue_selector_root"] = 4328
        semantic["dialogue_selector"] = dict(selected_concept=11703, current_control=1,
            parent_control=None, current_body=4332, parent_body=None, history=[None] * 8)
        state = scene_state(semantic)
        self.assertEqual(state["dialogue_selector_root"], 4328)
        self.assertEqual(state["dialogue_selector"], semantic["dialogue_selector"])
        self.assertIsNone(scene_state(frame()["semantic"])["dialogue_selector"])

    def ending_state(self):
        return dict(profile=1, active_video=None, video_open=False, ending=dict(
            ending_active=True, last_assignment=dict(code_offset=40724, query_mode=False)))

    def test_natural_ending_can_interrupt_only_an_expected_final_wait(self):
        actions = ["choose abandon", "wait 1800"]
        state = self.ending_state()
        result = verify_route_completion(actions, actions[:-1], 0, state, 40724)
        self.assertTrue(result["interrupted_final_wait"])
        self.assertEqual(result["code_offset"], 40724)
        result = verify_route_completion(actions, actions, 0, state, 40724)
        self.assertFalse(result["interrupted_final_wait"])
        with self.assertRaisesRegex(ValueError, "completed"):
            verify_route_completion(actions, actions[:-1], 0, state)
        for remaining in (["choose yes"], ["wait 10", "choose yes"], ["park 4 45"]):
            with self.subTest(remaining=remaining), self.assertRaisesRegex(ValueError, "passive wait"):
                verify_route_completion(actions[:1] + remaining, actions[:1], 0, state, 40724)

    def test_expected_ending_rejects_wrong_opcode_query_crash_and_open_video(self):
        actions = ["choose abandon", "wait 1800"]
        for mutation in (lambda s: s.update(profile=2),
                         lambda s: s["ending"].update(ending_active=False),
                         lambda s: s["ending"]["last_assignment"].update(code_offset=40723),
                         lambda s: s["ending"]["last_assignment"].update(query_mode=True),
                         lambda s: s.update(video_open=True),
                         lambda s: s.pop("video_open"),
                         lambda s: s.pop("active_video"),
                         lambda s: s.update(active_video="SQ\\bobb.hnm")):
            state = self.ending_state()
            mutation(state)
            with self.subTest(state=state), self.assertRaises(ValueError):
                verify_route_completion(actions, actions[:-1], 0, state, 40724)
        with self.assertRaisesRegex(RuntimeError, "exited 1"):
            verify_route_completion(actions, actions[:-1], 1, self.ending_state(), 40724)
        with self.assertRaisesRegex(ValueError, "expected SCRIPT2"):
            verify_route_completion(actions, actions[:-1], 0, None, 40724)

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

    def test_bbb_success_requires_all_fifteen_ending_clips(self):
        actions = ["click 100 111", "wait 4000"]
        state = self.ending_state()
        state["ending"]["last_assignment"]["code_offset"] = BBB_SUCCESS_ENDING_OFFSET
        runs = [dict(profile=1, resource="SQ\\" + name, observed_decoded_frames=30,
                     ended_at_frame=index + 1,
                     end_reason="source_closed" if index == 14 else "replaced")
                for index, name in enumerate(BBB_SUCCESS_SEQUENCES)]
        result = verify_route_completion(actions, actions[:-1], 0, state,
                                         BBB_SUCCESS_ENDING_OFFSET, sequence_runs=runs)
        self.assertEqual(result["kind"], "bbb_success")
        self.assertEqual(result["sequence_runs"], 15)
        self.assertTrue(result["interrupted_final_wait"])
        self.assertEqual(verify_route_completion(
            actions, actions[:-1], 0, state, BBB_SUCCESS_ENDING_OFFSET,
            sequence_runs=[dict(resource="SQ\\cryorad.hnm")] + runs), result)
        for mutation in (lambda r: r.clear(), lambda r: r.pop(5), lambda r: r.reverse(),
                         lambda r: r[0].update(profile=0),
                         lambda r: r[0].update(observed_decoded_frames=0),
                         lambda r: r[-1].update(resource="SQ\\fin.hnm"),
                         lambda r: r[-1].update(end_reason="replaced"),
                         lambda r: r[3].update(end_reason="source_closed"),
                         lambda r: r[3].update(ended_at_frame=None)):
            changed = copy.deepcopy(runs)
            mutation(changed)
            with self.subTest(runs=changed), self.assertRaisesRegex(ValueError, "successful ending"):
                verify_route_completion(actions, actions[:-1], 0, state,
                                        BBB_SUCCESS_ENDING_OFFSET, sequence_runs=changed)

    def concert_fixture(self):
        state = dict(profile=4, cod_site=5989, actor=dict(record=17),
                     navigation=dict(record=97), active_video=None, video_open=False)
        runs = [dict(profile=4, cod_site=site, resource="SQ\\" + name,
                     observed_decoded_frames=30, ended_at_frame=index + 1,
                     end_reason="source_closed")
                for index, (site, name) in enumerate(CB_CONCERT_SEQUENCES)]
        objects = [dict(record=125, target_record=17), dict(record=98, target_record=94)]
        return dict(final_state=state, expect_cb_ending=True, sequence_runs=runs, final_objects=objects)

    def test_cb_concert_requires_earned_ordered_decoded_closed_sequences(self):
        actions = ["choose teleport", "wait 2500"]
        result = verify_route_completion(actions, actions[:-1], 0, **self.concert_fixture())
        self.assertEqual(result["kind"], "cb_concert")
        self.assertEqual(result["sequence_runs"], 22)
        self.assertTrue(result["interrupted_final_wait"])
        for mutation in (lambda f: f["final_state"].update(profile=3),
                         lambda f: f["final_state"].update(cod_site=5988),
                         lambda f: f["final_state"].update(actor=dict(record=3)),
                         lambda f: f["final_state"].update(navigation=dict(record=94)),
                         lambda f: f["final_objects"][0].update(target_record=None),
                         lambda f: f["final_objects"][1].update(target_record=53),
                         lambda f: f["sequence_runs"].pop(5),
                         lambda f: f["sequence_runs"].reverse(),
                         lambda f: f["sequence_runs"][-1].update(observed_decoded_frames=0),
                         lambda f: f["sequence_runs"][-1].update(end_reason="replaced"),
                         lambda f: f["sequence_runs"][-1].update(ended_at_frame=None),
                         lambda f: f["sequence_runs"][-1].update(resource="SQ\\bobb.hnm"),
                         lambda f: f["final_state"].update(video_open=True),
                         lambda f: f["final_state"].update(active_video="SQ\\fin.hnm")):
            fixture = self.concert_fixture()
            mutation(fixture)
            with self.subTest(fixture=fixture), self.assertRaises(ValueError):
                verify_route_completion(actions, actions[:-1], 0, **fixture)
        with self.assertRaisesRegex(ValueError, "passive wait"):
            verify_route_completion(actions, [], 0, **self.concert_fixture())
        with self.assertRaisesRegex(RuntimeError, "exited 1"):
            verify_route_completion(actions, actions[:-1], 1, **self.concert_fixture())
        with self.assertRaisesRegex(ValueError, "mutually exclusive"):
            verify_route_completion(actions, actions[:-1], 0, ending_offset=1,
                                    **self.concert_fixture())

    def test_sequence_decoder_counts_survive_compaction_and_close_without_retained_media(self):
        recorder = FlowRecorder(io.StringIO())
        item = frame()
        item["semantic"]["video"]["active_resource"] = "SQ\\fin.hnm"
        for number in range(4):
            item["frame"] = number
            item["semantic"]["video"]["decoded_frame_count"] = number
            recorder.record(copy.deepcopy(item))
        self.assertEqual(recorder.events, 1)
        item["frame"] = 4
        item["semantic"]["video"].update(active_resource=None, decoded_frame_count=0,
                                          source_open_or_draining=False)
        recorder.record(item)
        self.assertEqual(recorder.sequence_runs, [dict(
            resource="SQ\\fin.hnm", profile=0, cod_site=42, started_at_frame=0,
            observed_decoded_frames=3, ended_at_frame=4, end_reason="source_closed")])
        self.assertIsNone(recorder.sequence_run)

    def test_sequence_restarts_and_replacements_are_not_reported_as_closed(self):
        recorder = FlowRecorder(io.StringIO())
        item = frame()
        item["semantic"]["video"].update(active_resource="SQ\\bobb.hnm", decoded_frame_count=20)
        recorder.record(copy.deepcopy(item))
        item["frame"] = 1
        item["semantic"]["video"]["decoded_frame_count"] = 1
        recorder.record(copy.deepcopy(item))
        item["frame"] = 2
        item["semantic"]["video"]["active_resource"] = "PE\\aamig.hnm"
        recorder.record(item)
        self.assertEqual([run["end_reason"] for run in recorder.sequence_runs], ["replaced", "replaced"])
        self.assertEqual([run["observed_decoded_frames"] for run in recorder.sequence_runs], [20, 1])
        self.assertIsNone(recorder.sequence_run)

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
            for invalid_status in ("failed", "observed_ending"):
                status["status"] = invalid_status
                manifest.write_text(json.dumps(status))
                with self.subTest(status=invalid_status), self.assertRaisesRegex(
                        ValueError, "not a completed normal-input"):
                    verified_predecessor(manifest, 0, "cb", provenance)

    def test_runtime_update_is_explicit_and_keeps_script_and_asset_checks(self):
        with tempfile.TemporaryDirectory() as directory:
            manifest, provenance, _ = self.checkpoint_fixture(Path(directory) / "parent")
            upgraded = {**provenance, "binary_sha256": "new-binary"}
            with self.assertRaisesRegex(ValueError, "binary_sha256"):
                verified_predecessor(manifest, 0, "cb", upgraded)
            _, reference = verified_predecessor(manifest, 0, "cb", upgraded,
                                                runtime_update="tested descriptor-loader fix")
            self.assertEqual(reference["runtime_update"], dict(
                reason="tested descriptor-loader fix", previous_binary_sha256="binary",
                current_binary_sha256="new-binary"))
            for key in ("asset_manifest_sha256", "sources"):
                with self.subTest(key=key), self.assertRaisesRegex(ValueError, key):
                    verified_predecessor(manifest, 0, "cb", {**upgraded, key: "changed"},
                                         runtime_update="tested descriptor-loader fix")
            bad_update = {**reference["runtime_update"], "previous_binary_sha256": "wrong"}
            with self.assertRaisesRegex(ValueError, "runtime update hashes changed"):
                verified_predecessor(manifest, 0, "cb", upgraded, runtime_update=bad_update)
            child, _, status = self.checkpoint_fixture(Path(directory) / "child")
            status.update(provenance=upgraded, predecessor=reference,
                          observations=dict(loaded_checkpoint=True))
            child.write_text(json.dumps(status))
            verified_predecessor(child, 0, "cb", upgraded)
            status["predecessor"]["runtime_update"] = bad_update
            child.write_text(json.dumps(status))
            with self.assertRaisesRegex(ValueError, "runtime update hashes changed"):
                verified_predecessor(child, 0, "cb", upgraded)

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

    def test_navigation_caption_cannot_complete_the_retained_dialogue_site(self):
        recorder = FlowRecorder(io.StringIO())
        item = frame()
        recorder.record(item)
        item["frame"] = 1
        semantic = item["semantic"]
        semantic["presentation"].update(active_actor_presentation=None, active=0)
        semantic["subtitle_bytes"] = list(b"planet ekatomb")
        semantic["presentation"]["text_state"]["subtitle_reveal_cursor"] = 14
        recorder.record(item)
        self.assertTrue(recorder.previous["text"]["complete"])
        self.assertEqual(recorder.summary()["published_sites"],
                         [dict(profile=1, domain="cod", offset=42)])
        self.assertEqual(recorder.summary()["fully_revealed_sites"], [])

    def test_revealed_site_attribution_requires_actor_activity_profile_and_completion(self):
        item = frame()
        item["semantic"]["presentation"]["text_state"]["subtitle_reveal_cursor"] = 5
        active = scene_state(item["semantic"])
        for mutate in (lambda s: s.update(actor=None),
                       lambda s: s["lifecycle"].update(active=0),
                       lambda s: s.update(profile=None),
                       lambda s: s["text"].update(complete=False)):
            state = copy.deepcopy(active)
            mutate(state)
            with self.subTest(state=state):
                self.assertEqual(revealed_text_sites(state), set())
        self.assertEqual(revealed_text_sites(active), {(0, "cod", 42)})
        active.update(cod_site=None, bas_site=100)
        self.assertEqual(revealed_text_sites(active), {(0, "bas", 100)})

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
