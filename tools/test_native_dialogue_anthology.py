import copy
import unittest

from native_dialogue_anthology import validate_trace


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

    def test_sequence_occurrences_keep_order(self):
        self.states[0]["state"]["video"] = dict(active_resource="SQ\\FIRST.HNM")
        self.states.append(copy.deepcopy(self.states[0]))
        self.states[-1]["time_ns"] = 92_000_000
        self.states[-1]["state"]["video"]["active_resource"] = "sq/second.hnm"
        self.assertEqual([row["resource"] for row in self.verify()["observed_sequence_resources"]],
                         ["SQ/FIRST.HNM", "SQ/SECOND.HNM"])


if __name__ == "__main__":
    unittest.main()
