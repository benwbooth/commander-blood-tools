import copy
import io
import json
from pathlib import Path
import tempfile
import unittest

from native_flow_coverage import (check_lineages, inline_template, join_sites, lineage_segments,
                                  matches_text, read_witness, report_markdown, safe_child, scan_events, wording)
from native_game_flow import FlowRecorder
from test_native_game_flow import frame
from video_anthology import digest


class CoverageTests(unittest.TestCase):
    def events(self, records):
        output = io.StringIO()
        recorder = FlowRecorder(output)
        for record in records:
            recorder.record(record)
        return output.getvalue(), recorder.summary()

    def scan(self, records):
        source, summary = self.events(records)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "events.jsonl"
            path.write_text(source)
            return scan_events(path, summary)

    def test_reaudits_actor_and_raster_instead_of_trusting_old_site_summary(self):
        first = frame()
        first["semantic"]["presentation"]["text_state"]["subtitle_reveal_cursor"] = 5
        hidden = copy.deepcopy(first)
        hidden["frame"] = 1
        hidden["semantic"]["published_cod_text_site"] = 99
        hidden["semantic"]["presentation"]["active_actor_presentation"] = None
        hidden["semantic"]["presentation"]["active"] = 0
        source, summary = self.events([first, hidden])
        summary["fully_revealed_sites"] = [dict(profile=1, domain="cod", offset=99)]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "events.jsonl"
            path.write_text(source)
            evidence = scan_events(path, summary)
        self.assertEqual(set(evidence["full"]), {(1, "cod", 42)})
        self.assertEqual(evidence["full"][1, "cod", 42][0]["text"], "Hello")

    def test_repeated_site_after_a_load_is_retained(self):
        first = frame()
        first["semantic"]["presentation"]["text_state"]["subtitle_reveal_cursor"] = 5
        hidden = copy.deepcopy(first)
        hidden["frame"] = 1
        hidden["semantic"]["presentation"]["active"] = 0
        repeat = copy.deepcopy(first)
        repeat["frame"] = 2
        evidence = self.scan([first, hidden, repeat])
        self.assertEqual([e["frame"] for e in evidence["full"][1, "cod", 42]], [0, 2])

    def test_zero_pixel_text_is_not_raster_evidence(self):
        first = frame()
        first["semantic"]["presentation"]["text_state"]["subtitle_reveal_cursor"] = 5
        first["semantic"]["subtitle_raster"] = dict(expected_pixel_count=0, matching_pixel_count=0)
        self.assertEqual(self.scan([first])["full"], {})

    def test_changed_event_order_raster_and_final_state_are_rejected(self):
        source, summary = self.events([frame()])
        for mutation, message in ((lambda e: e.update(event=3), "nonmonotonic"),
                                  (lambda e: e["text_raster"].update(matching_pixel_count=4), "raster"),
                                  (lambda e: e["state"].update(cod_site=99), "final event")):
            with self.subTest(message=message), tempfile.TemporaryDirectory() as directory:
                event = json.loads(source)
                mutation(event)
                path = Path(directory) / "events.jsonl"
                path.write_text(json.dumps(event) + "\n")
                with self.assertRaisesRegex(ValueError, message):
                    scan_events(path, summary)

    def lineage(self):
        root, child = Path("/flow/root/flow.json"), Path("/flow/child/flow.json")
        provenance = dict(binary_sha256="old", asset_manifest_sha256="assets", sources={"source": "hash"})
        first = dict(path=root, sha256="root-hash", manifest=dict(
            game="cb", status="observed_route", provenance=provenance,
            checkpoints=[dict(slot=0, saved_at_frame=20)],
            observations=dict(observed_frames=40)), evidence=dict(loads=[]))
        second = dict(path=child, sha256="child-hash", manifest=dict(
            game="cb", status="observed_route", provenance=copy.deepcopy(provenance),
            predecessor=dict(manifest=str(root), manifest_sha256="root-hash", slot=0),
            checkpoints=[], observations=dict(observed_frames=30, loaded_checkpoint=True)),
            evidence=dict(loads=[dict(frame=10, slot=0)]))
        return root, child, {root: first, child: second}

    def test_lineage_uses_saved_boundary_not_parent_process_end(self):
        root, child, rows = self.lineage()
        check_lineages(rows)
        segments = lineage_segments(child, rows)
        self.assertEqual([(s["start_frame"], s["end_frame"]) for s in segments], [(0, 20), (10, 29)])
        self.assertEqual([s["witness"] for s in segments], [str(root), str(child)])

    def test_lineage_rejects_tampered_manifest_missing_slot_and_cycle(self):
        for mode in ("hash", "slot", "cycle", "load", "game", "assets", "ending"):
            with self.subTest(mode=mode):
                root, child, rows = self.lineage()
                parent = rows[child]["manifest"]["predecessor"]
                if mode == "hash":
                    parent["manifest_sha256"] = "wrong"
                elif mode == "slot":
                    rows[root]["manifest"]["checkpoints"] = []
                elif mode == "cycle":
                    parent["manifest"] = str(child)
                elif mode == "load":
                    rows[child]["evidence"]["loads"][0]["slot"] = 1
                elif mode == "game":
                    rows[child]["manifest"]["game"] = "bbb"
                elif mode == "assets":
                    rows[child]["manifest"]["provenance"]["asset_manifest_sha256"] = "other"
                else:
                    rows[root]["manifest"]["status"] = "observed_ending"
                with self.assertRaises(ValueError):
                    check_lineages(rows)

    def test_runtime_change_needs_exact_hashes_and_reason(self):
        root, child, rows = self.lineage()
        rows[child]["manifest"]["provenance"]["binary_sha256"] = "new"
        with self.assertRaisesRegex(ValueError, "runtime"):
            check_lineages(rows)
        update = dict(reason="verified repair", previous_binary_sha256="old", current_binary_sha256="new")
        rows[child]["manifest"]["predecessor"]["runtime_update"] = update
        check_lineages(rows)
        update["previous_binary_sha256"] = "wrong"
        with self.assertRaisesRegex(ValueError, "runtime"):
            check_lineages(rows)

    def test_matching_wording_does_not_mark_unseen_site_witnessed(self):
        path = Path("/flow/one/flow.json")
        texts = ["Hello", "Hello", "hello", "Hello <state:2>", "..."]
        sites = {("cb", 1, "cod", offset): dict(game="cb", profile=1, domain="cod", offset=offset,
                  text=text, inline_text=text, dynamic="<state:" in text, witnesses=[],
                  rejected_presentations=[]) for offset, text in enumerate(texts)}
        evidence = dict(frame=3, text="Hello", actor={"name": "Bob"}, video="bob.hnm")
        witnesses = {path: dict(manifest=dict(game="cb"), evidence=dict(full={(1, "cod", 0): [evidence]}))}
        join_sites(sites, witnesses, {"cb": [dict(witness=str(path), start_frame=0, end_frame=2)]})
        self.assertEqual([site["status"] for site in sites.values()], [
            "witnessed_on_other_normal_route", "unobserved_site_with_witnessed_wording",
            "unobserved_wording_or_dynamic_site", "unobserved_wording_or_dynamic_site",
            "unobserved_empty_or_control_text"])
        self.assertEqual(wording(" A\n B "), "A B")

    def test_later_occurrence_inside_main_segment_is_used(self):
        path = Path("/flow/one/flow.json")
        key = "cb", 1, "cod", 42
        sites = {key: dict(game="cb", text="Hello", inline_text="Hello", dynamic=False,
                          witnesses=[], rejected_presentations=[])}
        evidence = dict(text="Hello", actor={"name": "Bob"}, video="bob.hnm")
        witnesses = {path: dict(manifest=dict(game="cb"), evidence=dict(full={key[1:]: [
            dict(frame=3, **evidence), dict(frame=12, **evidence)]}))}
        join_sites(sites, witnesses, {"cb": [dict(witness=str(path), start_frame=10, end_frame=15)]})
        self.assertEqual(sites[key]["status"], "witnessed_on_successful_route")
        self.assertEqual(sites[key]["witnesses"][0]["frame"], 12)

    def test_text_matching_uses_authored_inline_tokens_and_numeric_markers(self):
        site = dict(text="Hello...", inline_text="Hello ...", dynamic=False)
        self.assertTrue(matches_text(site, dict(text="Hello ...", text_kind="inline_words")))
        self.assertTrue(matches_text(site, dict(text="Hello...\r", text_kind="subtitle_bytes")))
        self.assertFalse(matches_text(site, dict(text="yes no", text_kind="inline_words")))
        self.assertEqual(inline_template(dict(spoken_operands=[dict(kind="dictionary", text="Count"),
            dict(kind="state_number", offset=20), dict(kind="dictionary", text="!")])), "Count <state:20> !")
        site = dict(text="Count <state:20> !", inline_text="Count <state:20> !", dynamic=True)
        self.assertTrue(matches_text(site, dict(text="Count -123 !")))
        self.assertFalse(matches_text(site, dict(text="Count 123 or 456 !")))
        self.assertFalse(matches_text(site, dict(text="Count unknown !")))

    def test_retained_offset_with_other_text_is_not_covered(self):
        path = Path("/flow/one/flow.json")
        key = "cb", 1, "cod", 42
        sites = {key: dict(game="cb", text="Hello", inline_text="Hello", dynamic=False,
                          witnesses=[], rejected_presentations=[])}
        witnesses = {path: dict(manifest=dict(game="cb"), evidence=dict(full={key[1:]: [
            dict(frame=3, text="yes no", actor={"name": "Bob"}, video="bob.hnm")]}))}
        join_sites(sites, witnesses, {})
        self.assertEqual(sites[key]["status"], "unobserved_wording_or_dynamic_site")
        self.assertEqual(sites[key]["witnesses"], [])
        self.assertEqual(len(sites[key]["rejected_presentations"]), 1)

    def test_unresolved_report_groups_keep_evidence_categories_separate(self):
        def site(actor, status, game="cb", profile=2):
            return dict(game=game, profile=profile, record=actor, domain="bas", status=status)

        result = dict(witnesses=[], successful_routes={"cb": []}, counts={"cb": {}}, sites=[
            site("Yoko", "unobserved_wording_or_dynamic_site"),
            site("Yoko", "unobserved_wording_or_dynamic_site"),
            site("Yoko", "unobserved_site_with_witnessed_wording"),
            site("Yoko", "unobserved_empty_or_control_text"),
            site("Yoko", "witnessed_on_successful_route"),
            site("Honk", "unobserved_wording_or_dynamic_site"),
            site(None, "unobserved_empty_or_control_text", profile=1),
            site("Other game", "unobserved_wording_or_dynamic_site", game="bbb")])
        report = report_markdown(result, "cb")
        yoko = "| SCRIPT2 | Yoko | BAS | 2 | 1 | 1 |"
        honk = "| SCRIPT2 | Honk | BAS | 1 | 0 | 0 |"
        self.assertIn(yoko, report)
        self.assertIn(honk, report)
        self.assertLess(report.index(yoko), report.index(honk))
        self.assertIn("| SCRIPT1 | Unattributed | BAS | 0 | 0 | 1 |", report)
        self.assertNotIn("Other game", report)
        self.assertIn("not distinct clips, feasible branches, or duration estimates", report)

    def test_empty_unresolved_report_does_not_claim_scene_completeness(self):
        result = dict(witnesses=[], successful_routes={"cb": []}, counts={"cb": {}}, sites=[])
        report = report_markdown(result, "cb")
        self.assertIn("this alone is not full scene-coverage proof", report)
        self.assertIn("Not complete or render-ready", report)

    def witness_fixture(self, root):
        events, summary = self.events([frame()])
        files = {"events.jsonl": events, "actions.jsonl": json.dumps(dict(
            phase="after", action_index=1, action="wait 1")) + "\n", "scenario.tsv": "wait 1\n",
            "game.log": "", "flow.md": "fixture"}
        for name, content in files.items():
            (root / name).write_text(content)
        manifest = dict(schema=1, game="cb", status="observed_route", state_injection=False,
                        observations=summary, checkpoints=[],
                        files={name: digest(root / name) for name in files},
                        provenance=dict(scenario_sha256=digest(root / "scenario.tsv"), sources={
                            "re/vm/profiles/script1.blood": "source-hash"}))
        path = root / "flow.json"
        path.write_text(json.dumps(manifest))
        return path

    def test_hash_bound_witness_accepts_and_detects_changed_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = self.witness_fixture(root)
            row = read_witness(path, {("cb", 1): "source-hash"})
            self.assertEqual(row["sha256"], digest(path))
            with self.assertRaisesRegex(ValueError, "sources differ"):
                read_witness(path, {("cb", 1): "wrong"})
            (root / "events.jsonl").write_text("changed")
            with self.assertRaisesRegex(ValueError, "changed evidence"):
                read_witness(path, {("cb", 1): "source-hash"})

    def test_missing_hash_and_unsafe_paths_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = self.witness_fixture(root)
            manifest = json.loads(path.read_text())
            del manifest["files"]["events.jsonl"]
            path.write_text(json.dumps(manifest))
            with self.assertRaisesRegex(ValueError, "missing hashed"):
                read_witness(path, {})
            with self.assertRaisesRegex(ValueError, "unsafe"):
                safe_child(root, "../outside")
            (root / "link").symlink_to(path)
            with self.assertRaisesRegex(ValueError, "unsafe"):
                safe_child(root, "link")

    def test_legacy_manifest_is_not_silently_treated_as_current_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            path = self.witness_fixture(Path(directory))
            manifest = json.loads(path.read_text())
            del manifest["observations"]["final_state"]
            path.write_text(json.dumps(manifest))
            with self.assertRaisesRegex(ValueError, "legacy"):
                read_witness(path, {})


if __name__ == "__main__":
    unittest.main()
