import copy
import unittest

from static_text_render import expected_menu_words, expected_reply_words, validate_report, validate_states


class PreparedReportTests(unittest.TestCase):
    def setUp(self):
        self.plan = dict(game="commander_blood", initial_profile=1, source="bas", text_site=42)
        self.report = dict(target="static_text_site", evidence_scope="prepared_source_site",
                           complete_native_presentation=False, complete_game=False,
                           completion="isolated_text_hold_completed", decoded_master_verified=True,
                           video_timestamps_verified=True, game="commander_blood", exporter_sha256="exporter",
                           runner=dict(plan=self.plan, story_execution="frozen", natural_chapter_close=False,
                                       gameplay_reachability="not_assessed",
                                       publication=dict(profile=1, offset=42, bas=True),
                                       authored=dict(source="bas", text_site=42)))

    def test_prepared_capture_has_distinct_completion(self):
        validate_report(self.plan, self.report, "exporter")

    def test_normal_flow_or_natural_close_claim_is_rejected(self):
        for field in ("complete_game", "complete_native_presentation"):
            report = copy.deepcopy(self.report)
            report[field] = True
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, "incorrectly claims"):
                validate_report(self.plan, report, "exporter")

    def test_code_namespace_must_not_alias_bas(self):
        self.report["runner"]["publication"]["bas"] = False
        with self.assertRaisesRegex(ValueError, "wrong source publication"):
            validate_report(self.plan, self.report, "exporter")

    def test_exporter_changes_are_rejected(self):
        with self.assertRaisesRegex(ValueError, "exporter changed"):
            validate_report(self.plan, self.report, "another-exporter")

    def test_context_changes_are_rejected(self):
        plan = dict(self.plan, context=dict(kind="another_context"))
        with self.assertRaisesRegex(ValueError, "prepared context differs"):
            validate_report(plan, self.report, "exporter")


class ExpectedMenuWordTests(unittest.TestCase):
    def test_original_dictionary_slots(self):
        authored = dict(flags_b4=0, sections=[[dict(text="Hello"), dict(text=",")], [dict(text="world")]])
        self.assertEqual(expected_menu_words(authored), ["Hello", ",", "world"])

    def test_localized_display_replaces_source_words(self):
        # BBB SCRIPT1 0x977: French source words, English display laid out by spaces.
        authored = dict(flags_b4=0, sections=[[dict(text="Bonjour"), dict(text="Commander.")]],
                        display=dict(sections=["Hello Commander. This is", "a recorded message."]))
        self.assertEqual(expected_menu_words(authored),
                         ["Hello", "Commander.", "This", "is", "a", "recorded", "message."])


    def test_history_trigger_section_is_not_displayed(self):
        authored = dict(flags_b4=0x41, sections=[[dict(text="Hello"), dict(text="friend")], [dict(text="hello")]],
                        display=dict(sections=["Hello friend", "hello"]))
        self.assertEqual(expected_menu_words(authored), ["Hello", "friend"])
        del authored["display"]
        self.assertEqual(expected_menu_words(authored), ["Hello", "friend"])


    def test_reply_rows_come_from_the_section_after_text_and_history(self):
        # CB SCRIPT2 COD 0x11B8 (Honk): question | talk remember bye_bye.
        authored = dict(flags_b4=0x30, sections=[[dict(text="What")], [dict(text="talk"), dict(text="bye_bye")]])
        self.assertEqual(expected_menu_words(authored), ["What"])
        self.assertEqual(expected_reply_words(authored), ["talk", "bye_bye"])
        authored = dict(flags_b4=0x50, sections=[[dict(text="Hi")], [dict(text="hello")], [dict(text="yes")]],
                        display=dict(sections=["Hi", "hello", "PLAY INSTRUCTIONS"]))
        self.assertEqual(expected_reply_words(authored), ["PLAY", "INSTRUCTIONS"])
        self.assertIsNone(expected_reply_words(dict(flags_b4=0x20, sections=[[]])))


class PreparedTraceTests(unittest.TestCase):
    def setUp(self):
        self.plan = dict(initial_profile=1, source="cod", text_site=42)
        self.rows = [dict(start_ns=0, duration_ns=68_000_000),
                     dict(start_ns=68_000_000, duration_ns=68_000_000)]
        self.state = dict(vm=dict(resource_profile=1),
                          input=dict(buttons=0, previous_buttons=0, press_pending=0, primary_pressed=0),
                          published_cod_text_site=42, published_bas_text_site=None,
                          presentation=dict(text_display_active=0,
                                            inline_menu=dict(display_words=["Hello", "world"], reveal_count=2),
                                            text_state=dict(hold_ready=True)),
                          inline_menu_raster=dict(expected_pixel_count=123, matching_pixel_count=123))
        self.states = [dict(time_ns=68_000_000, state=self.state)]

    def test_native_reveal_and_hold_are_required_before_endpoint(self):
        evidence = validate_states(self.plan, self.states, self.rows, "Hello\nworld", ["Hello", "world"])
        self.assertEqual(evidence["fully_revealed_ui_frames"], 1)
        self.assertEqual(evidence["first_full_ui_ns"], 68_000_000)
        self.assertTrue(evidence["native_hold_completed"])

    def test_subtitle_mode_checks_byte_cursor(self):
        self.state["presentation"]["text_display_active"] = 1
        self.state["subtitle_bytes"] = list(b"Hello world")
        self.state["subtitle_raster"] = self.state["inline_menu_raster"]
        self.state["presentation"]["text_state"]["subtitle_reveal_cursor"] = 11
        validate_states(self.plan, self.states, self.rows, "Hello world", ["Hello", "world"])
        self.state["presentation"]["text_state"]["subtitle_reveal_cursor"] = 10
        with self.assertRaisesRegex(ValueError, "no fully revealed"):
            validate_states(self.plan, self.states, self.rows, "Hello world", ["Hello", "world"])

    def test_endpoint_only_reveal_is_not_encoded_coverage(self):
        self.states[0]["time_ns"] = 136_000_000
        with self.assertRaisesRegex(ValueError, "no fully revealed"):
            validate_states(self.plan, self.states, self.rows, "Hello world", ["Hello", "world"])

    def test_wrong_text_is_not_coverage(self):
        with self.assertRaisesRegex(ValueError, "displayed menu words differ"):
            validate_states(self.plan, self.states, self.rows, "Different world", ["Different", "world"])
        self.state["presentation"]["text_display_active"] = 1
        self.state["subtitle_bytes"] = list(b"Hello world")
        self.state["subtitle_raster"] = self.state["inline_menu_raster"]
        self.state["presentation"]["text_state"]["subtitle_reveal_cursor"] = 11
        with self.assertRaisesRegex(ValueError, "displayed subtitle differs"):
            validate_states(self.plan, self.states, self.rows, "Different world", ["Hello", "world"])

    def test_menu_compares_dictionary_word_slots_not_subtitle_spacing(self):
        # SCRIPT1 0x564 lays punctuation out as its own menu word ("SENSATIONS ,").
        words = ["SENSATIONS", ",", "DIZZINESS", "..."]
        self.state["presentation"]["inline_menu"] = dict(display_words=words, reveal_count=4)
        validate_states(self.plan, self.states, self.rows, "SENSATIONS, DIZZINESS...", words)
        with self.assertRaisesRegex(ValueError, "displayed menu words differ"):
            validate_states(self.plan, self.states, self.rows, "SENSATIONS, DIZZINESS...", words[:-1])

    def test_wrong_glyphs_are_not_coverage(self):
        self.state["inline_menu_raster"]["matching_pixel_count"] = 122
        with self.assertRaisesRegex(ValueError, "glyph raster mismatch"):
            validate_states(self.plan, self.states, self.rows, "Hello world", ["Hello", "world"])

    def test_other_publication_is_not_coverage(self):
        self.state["published_bas_text_site"] = 42
        with self.assertRaisesRegex(ValueError, "another source site"):
            validate_states(self.plan, self.states, self.rows, "Hello world", ["Hello", "world"])

    def test_unfinished_hold_is_not_complete(self):
        self.state["presentation"]["text_state"]["hold_ready"] = False
        with self.assertRaisesRegex(ValueError, "hold did not complete"):
            validate_states(self.plan, self.states, self.rows, "Hello world", ["Hello", "world"])

    def test_reply_rows_must_open_and_match(self):
        presentation = self.state["presentation"]
        presentation["rendered_word_choices"] = ["yes", "no"]
        presentation["retained_word_choice"] = dict(phase="Selecting")
        evidence = validate_states(self.plan, self.states, self.rows, "Hello world", ["Hello", "world"], ["yes", "no"])
        self.assertEqual(evidence["reply_rows"], ["yes", "no"])
        with self.assertRaisesRegex(ValueError, "reply rows differ"):
            validate_states(self.plan, self.states, self.rows, "Hello world", ["Hello", "world"], ["no"])
        presentation["retained_word_choice"] = dict(phase="Opening")
        with self.assertRaisesRegex(ValueError, "never opened"):
            validate_states(self.plan, self.states, self.rows, "Hello world", ["Hello", "world"], ["yes", "no"])

    def test_pointer_input_is_rejected(self):
        self.state["input"]["press_pending"] = 1
        with self.assertRaisesRegex(ValueError, "pointer input"):
            validate_states(self.plan, self.states, self.rows, "Hello world", ["Hello", "world"])


if __name__ == "__main__":
    unittest.main()
