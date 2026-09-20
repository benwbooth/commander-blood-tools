import copy
import json
from pathlib import Path
import tempfile
import unittest

from native_dialogue_anthology import chapter_plans, reusable_chapters, validate_trace


class ChapterPlanTests(unittest.TestCase):
    def test_plan_set_preserves_order_and_is_relative_to_its_own_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            for name in ("one", "two", "three"):
                (base / f"{name}.json").write_text(json.dumps(dict(title=name)))
            manifest = base / "planning.json"
            manifest.write_text(json.dumps(dict(schema=1, plans=["three.json", "two.json"])))
            plans = chapter_plans([base / "one.json"], manifest)
            self.assertEqual([plan["title"] for _, plan in plans], ["one", "three", "two"])
            self.assertTrue(all(path.is_absolute() for path, _ in plans))
            with self.assertRaisesRegex(ValueError, "duplicate plan names"):
                chapter_plans([base / "two.json"], manifest)
            (base / "two.json").write_text(json.dumps(dict(title="three")))
            with self.assertRaisesRegex(ValueError, "duplicate chapter titles"):
                chapter_plans([], manifest)

    def test_empty_or_malformed_plan_set_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "no dialogue plans"):
            chapter_plans(None)
        with tempfile.TemporaryDirectory() as directory:
            manifest = Path(directory) / "planning.json"
            for value in (dict(schema=1, plans=[42]), dict(schema=2, plans=[])):
                manifest.write_text(json.dumps(value))
                with self.assertRaisesRegex(ValueError, "invalid chapter plan set"):
                    chapter_plans([], manifest)

    def test_reuse_requires_matching_sources_and_retains_only_completed_chapters(self):
        with tempfile.TemporaryDirectory() as directory:
            batch = Path(directory)
            provenance = dict(game="commander_blood", asset_manifest_sha256="assets", exporter_sha256="binary",
                              records=[dict(name=name, plan=dict(title=name)) for name in ("ok", "failed")])
            entry = dict(record="ok", path=str(batch / "ok"))
            coverage = dict(provenance=provenance, complete=False, failures=[dict(record="failed")], completed=[entry])
            (batch / "selection.json").write_text(json.dumps(provenance))
            (batch / "coverage.json").write_text(json.dumps(coverage))
            self.assertEqual(reusable_chapters([batch], provenance), [(dict(title="ok"), entry)])
            changed = {**provenance, "exporter_sha256": "other"}
            with self.assertRaisesRegex(ValueError, "source or exporter differs"):
                reusable_chapters([batch], changed)
            coverage["provenance"] = changed
            (batch / "coverage.json").write_text(json.dumps(coverage))
            with self.assertRaisesRegex(ValueError, "selection changed"):
                reusable_chapters([batch], provenance)

    def test_reuse_rejects_unselected_or_duplicate_completed_entries(self):
        with tempfile.TemporaryDirectory() as directory:
            batch = Path(directory)
            provenance = dict(game="commander_blood", asset_manifest_sha256="assets", exporter_sha256="binary",
                              records=[dict(name="ok", plan=dict(title="ok"))])
            (batch / "selection.json").write_text(json.dumps(provenance))
            for entries, error in (([dict(record="unknown")], "not in its selection"),
                                   ([dict(record="ok"), dict(record="ok")], "ambiguous reuse")):
                (batch / "coverage.json").write_text(json.dumps(dict(provenance=provenance, completed=entries)))
                with self.assertRaisesRegex(ValueError, error):
                    reusable_chapters([batch], provenance)


class DialogueTraceTests(unittest.TestCase):
    def setUp(self):
        self.plan = dict(initial_profile=0, required_cod_sites=[100, 200],
                         required_frame_boundary_cod_sites=[100],
                         choices=[dict(text_site=100, word_offset=4)],
                         end=dict(kind="profile_loaded", profile=1))
        self.rows = [dict(start_ns=index * 46_000_000, duration_ns=46_000_000)
                     for index in range(3)]
        self.runner = dict(chapter=self.plan, published_cod_sites=[100, 200],
                           choices=[dict(text_site=100, word_offset=4, requested_at_ns=46_000_000)],
                           publications=[dict(frame_end_ns=46_000_000,
                               publication=dict(profile=0, offset=100, subtitle=True)),
                               dict(frame_end_ns=138_000_000,
                               publication=dict(profile=0, offset=200, subtitle=True))],
                           final_profile=1)
        self.states = [dict(time_ns=46_000_000, state=dict(
            input=dict(buttons=0, previous_buttons=0, press_pending=0, primary_pressed=0),
            vm=dict(resource_profile=0), published_cod_text_site=100,
            subtitle_bytes=[65, 13],
            subtitle_raster=dict(expected_pixel_count=12, matching_pixel_count=12),
            inline_menu_raster=None,
            presentation=dict(active=1, text_display_active=1,
                              text_state=dict(subtitle_reveal_cursor=2))))]

    def verify(self):
        return validate_trace(self.plan, self.runner, self.states, self.rows)

    def test_preempted_publication_is_not_counted_as_raster_coverage(self):
        result = self.verify()
        self.assertEqual(result["published_without_ui_raster"], [200])
        self.assertEqual(result["published_without_full_ui_reveal"], [200])
        self.assertEqual(result["ui_raster_evidence"]["100"]["first_full_ui_ns"], 46_000_000)

    def test_partial_reveal_is_not_full_coverage(self):
        self.states[0]["state"]["presentation"]["text_state"]["subtitle_reveal_cursor"] = 1
        self.assertEqual(self.verify()["published_without_full_ui_reveal"], [100, 200])

    def test_bas_publications_and_rasters_do_not_alias_cod(self):
        self.runner["publications"].append(dict(frame_end_ns=138_000_000,
                                               publication=dict(profile=0, offset=100, subtitle=True, bas=True)))
        self.runner["published_bas_sites"] = [100]
        self.plan["required_bas_sites"] = [100]
        self.plan["required_frame_boundary_bas_sites"] = [100]
        state = copy.deepcopy(self.states[0])
        state["time_ns"] = 92_000_000
        state["state"]["published_cod_text_site"] = None
        state["state"]["published_bas_text_site"] = 100
        self.states.append(state)
        result = self.verify()
        self.assertEqual(result["published_bas_sites"], [100])
        self.assertEqual(result["bas_ui_raster_evidence"]["100"]["first_full_ui_ns"], 92_000_000)
        self.assertEqual(result["ui_raster_evidence"]["100"]["first_full_ui_ns"], 46_000_000)

    def test_ambiguous_cod_bas_source_is_rejected(self):
        self.states[0]["state"]["published_bas_text_site"] = 100
        with self.assertRaisesRegex(ValueError, "ambiguous COD/BAS"):
            self.verify()

    def test_bas_cannot_satisfy_cod_publication_requirement(self):
        self.runner["publications"][0]["publication"]["bas"] = True
        with self.assertRaisesRegex(ValueError, "publication accounting"):
            self.verify()

    def test_choice_source_is_part_of_the_contract(self):
        self.plan["choices"][0]["source"] = "bas_menu"
        with self.assertRaisesRegex(ValueError, "different semantic choices"):
            self.verify()
        self.runner["choices"][0]["source"] = "bas_menu"
        self.verify()

    def test_only_the_planned_final_choice_can_be_retried_within_the_bound(self):
        self.plan["max_exit_retries"] = 1
        self.plan["choices"][-1]["source"] = "bas_menu"
        self.runner["choices"][-1]["source"] = "bas_menu"
        self.runner["choices"].append(copy.deepcopy(self.runner["choices"][-1]))
        self.verify()
        self.runner["choices"][-1]["word_offset"] = 5
        with self.assertRaisesRegex(ValueError, "different semantic choices"):
            self.verify()
        self.runner["choices"][-1]["word_offset"] = 4
        self.runner["choices"].append(copy.deepcopy(self.runner["choices"][-1]))
        with self.assertRaisesRegex(ValueError, "retry limit"):
            self.verify()

    def test_other_profile_site_does_not_satisfy_requirement(self):
        self.runner["publications"][1]["publication"]["profile"] = 1
        with self.assertRaisesRegex(ValueError, "publication accounting"):
            self.verify()

    def test_missing_required_publication_is_rejected(self):
        self.runner["publications"].pop()
        self.runner["published_cod_sites"] = [100]
        with self.assertRaisesRegex(ValueError, "missing required publication"):
            self.verify()

    def test_any_pointer_input_is_rejected(self):
        for field in self.states[0]["state"]["input"]:
            state = copy.deepcopy(self.states)
            state[0]["state"]["input"][field] = 1
            with self.assertRaisesRegex(ValueError, "pointer input"):
                validate_trace(self.plan, self.runner, state, self.rows)

    def test_different_choice_is_rejected(self):
        self.runner["choices"][0]["word_offset"] = 5
        with self.assertRaisesRegex(ValueError, "different semantic choices"):
            self.verify()

    def test_raster_mismatch_is_rejected(self):
        self.states[0]["state"]["subtitle_raster"]["matching_pixel_count"] = 11
        with self.assertRaisesRegex(ValueError, "raster mismatch"):
            self.verify()

    def test_absent_raster_is_reported_not_invented(self):
        self.states[0]["state"]["subtitle_raster"] = None
        self.assertEqual(self.verify()["published_without_ui_raster"], [100, 200])

    def test_publication_timing_must_be_a_native_frame_end(self):
        self.runner["publications"][0]["frame_end_ns"] = 1
        with self.assertRaisesRegex(ValueError, "publication outside"):
            self.verify()

    def test_published_does_not_imply_retained_in_state_trace(self):
        self.states[0]["state"]["published_cod_text_site"] = None
        with self.assertRaisesRegex(ValueError, "frame-boundary text"):
            self.verify()

    def test_finished_presentation_gate_is_checked(self):
        self.plan["end"] = dict(kind="presentation_finished")
        with self.assertRaisesRegex(ValueError, "did not finish"):
            self.verify()

    def test_explicitly_unpublished_site_must_stay_unpublished(self):
        self.plan["expected_unpublished_cod_sites"] = [300]
        self.assertEqual(self.verify()["expected_unpublished_cod_sites"], [300])
        self.plan["expected_unpublished_cod_sites"] = [100]
        with self.assertRaisesRegex(ValueError, "declared absent"):
            self.verify()

    def test_contact_requires_native_cleanup(self):
        self.plan["entry"] = "contact"
        self.runner["contact_transition_closed"] = True
        state = self.states[0]["state"]
        state["contact_transition"] = dict(phase="Finish")
        state["presentation"]["navigation_rebuild_pending"] = False
        with self.assertRaisesRegex(ValueError, "contact transition did not finish"):
            self.verify()
        state["contact_transition"]["phase"] = "Inactive"
        self.verify()

    def test_bbb_contact_preparation_is_bound_to_its_cod_not_the_cb_manifest(self):
        self.plan.update(game="big_bug_bang", cod_sha256="source-cod", contact_procedure=90)
        self.runner["contact_preparation"] = dict(procedure_offset=90, manifest_sha256="cb-only")
        with self.assertRaisesRegex(ValueError, "authored COD guard"):
            self.verify()
        self.runner["contact_preparation"].update(cod_sha256="source-cod", guard_source="typed_cod_outer_guard")
        self.verify()
        self.runner["contact_preparation"]["cod_sha256"] = "other-cod"
        with self.assertRaisesRegex(ValueError, "authored COD guard"):
            self.verify()

    def test_contact_encounter_preparation_requires_matching_source_and_increment(self):
        self.plan.update(game="big_bug_bang", cod_sha256="source-cod", contact_procedure=90,
                         contact_encounter_guard=123)
        preparation = dict(procedure_offset=90, cod_sha256="source-cod",
                           guard_source="typed_cod_outer_guard")
        self.runner["contact_preparation"] = preparation
        with self.assertRaisesRegex(ValueError, "source-bound encounter"):
            self.verify()
        preparation["encounter_guard"] = dict(offset=123, at_presentation=2, before_entry=1)
        self.verify()
        for key, value in [("offset", 124), ("at_presentation", 0), ("at_presentation", 65536),
                           ("before_entry", 2)]:
            original = preparation["encounter_guard"][key]
            preparation["encounter_guard"][key] = value
            with self.assertRaisesRegex(ValueError, "source-bound encounter"):
                self.verify()
            preparation["encounter_guard"][key] = original
        del self.plan["contact_encounter_guard"]
        with self.assertRaisesRegex(ValueError, "unexpected encounter"):
            self.verify()

    def test_sequence_occurrences_keep_order(self):
        self.states[0]["state"]["video"] = dict(active_resource="SQ\\FIRST.HNM")
        self.states.append(copy.deepcopy(self.states[0]))
        self.states[-1]["time_ns"] = 92_000_000
        self.states[-1]["state"]["video"]["active_resource"] = "sq/second.hnm"
        self.assertEqual([row["resource"] for row in self.verify()["observed_sequence_resources"]],
                         ["SQ/FIRST.HNM", "SQ/SECOND.HNM"])

    def test_travel_requires_setup_provenance_and_native_return_to_bridge(self):
        self.plan["entry"] = "travel"
        self.plan["travel_setup"] = dict(planet="Moskito", destination="usine", procedure_offset=90)
        with self.assertRaisesRegex(ValueError, "prepared-travel provenance"):
            self.verify()
        self.runner["travel_preparation"] = dict(setup=self.plan["travel_setup"])
        self.runner["travel_transition_closed"] = True
        presentation = self.states[0]["state"]["presentation"]
        presentation["ship_flags"] = 17
        presentation["navigation_rebuild_pending"] = False
        presentation["text_state"]["sequence_active"] = False
        with self.assertRaisesRegex(ValueError, "travel transition did not finish"):
            self.verify()
        presentation["ship_flags"] = 0
        self.verify()
        presentation["text_state"]["sequence_active"] = True
        with self.assertRaisesRegex(ValueError, "travel transition did not finish"):
            self.verify()


if __name__ == "__main__":
    unittest.main()
