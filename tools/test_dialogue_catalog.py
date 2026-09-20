from copy import deepcopy
import unittest

from dialogue_catalog import apply_english, bas_dot, cod_dot, display_choices, display_text, fence, profile_markdown, summarize


class EnglishOverlayTests(unittest.TestCase):
    def setUp(self):
        self.words = [dict(kind="dictionary", offset=5, text="Bonjour")]
        self.choices = [dict(kind="dictionary", offset=8, text="JOUER"),
                        dict(kind="dictionary", offset=16, text="EXPLICATIONS")]
        self.graph = dict(game="bbb", profile="SCRIPT1", resources=dict(cod_sha256="a" * 64, dic_sha256="b" * 64),
                          cod=dict(text_sites=[dict(offset=2, text="Bonjour", spoken_operands=self.words,
                                                   choice_operands=self.choices, sections=[self.words, self.choices])]))
        self.translation = dict(format="bbb-cod-display-translation-v1", language="en", profile="SCRIPT1",
                                cod_sha256="a" * 64, dic_sha256="b" * 64,
                                messages={"bbb.script1.cod.00000002": ["Hello", "PLAY INSTRUCTIONS"]})

    def test_overlay_preserves_original_fields_and_choice_ids(self):
        original = deepcopy(self.graph)
        apply_english(self.graph, self.translation)
        site = self.graph["cod"]["text_sites"][0]
        self.assertEqual(display_text(site), "Hello")
        self.assertEqual([word["text"] for word in display_choices(site)], ["PLAY", "INSTRUCTIONS"])
        self.assertEqual([word["offset"] for word in display_choices(site)], [8, 16])
        site.pop("display")
        self.graph.pop("localization")
        self.assertEqual(self.graph, original)

    def test_rejects_wrong_resources_profile_missing_and_extra_sites(self):
        for key, value in (("cod_sha256", "c" * 64), ("dic_sha256", "d" * 64),
                           ("profile", "SCRIPT2"), ("language", "fr"), ("messages", {})):
            translated = deepcopy(self.translation)
            translated[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                apply_english(self.graph, translated)
        translated = deepcopy(self.translation)
        translated["messages"]["bbb.script1.cod.00000099"] = ["extra"]
        with self.assertRaisesRegex(ValueError, "exactly"):
            apply_english(self.graph, translated)
        self.assertNotIn("display", self.graph["cod"]["text_sites"][0])

    def test_rejects_choice_count_section_count_and_nonprintable_text(self):
        for sections in (["Hello", "PLAY"], ["Hello"], ["Hello\nthere", "PLAY INSTRUCTIONS"]):
            with self.subTest(sections=sections), self.assertRaises(ValueError):
                apply_english(self.graph, dict(self.translation, messages={"bbb.script1.cod.00000002": sections}))

    def test_keeps_ordered_live_numbers_and_inventory_symbolic(self):
        site = self.graph["cod"]["text_sites"][0]
        numbers = [dict(kind="state_number", offset=7922), dict(kind="state_number", offset=8362)]
        inventory = [dict(kind="inventory_choices")]
        site.update(spoken_operands=numbers, choice_operands=inventory, sections=[numbers, inventory])
        self.translation["messages"]["bbb.script1.cod.00000002"] = ["Credits: <state:7922> Bionium: <state:8362>", "<inventory_choices>"]
        apply_english(self.graph, self.translation)
        self.assertEqual(display_choices(site), inventory)
        self.assertIn("<state:7922>", display_text(site))
        for prose in ("<state:8362> <state:7922>", "<state:7922>", "<state:7922>, <state:8362>"):
            self.translation["messages"]["bbb.script1.cod.00000002"][0] = prose
            with self.subTest(prose=prose), self.assertRaises(ValueError):
                apply_english(self.graph, self.translation)
        self.translation["messages"]["bbb.script1.cod.00000002"] = ["<state:7922> <state:8362>", "treaty"]
        with self.assertRaisesRegex(ValueError, "generator"):
            apply_english(self.graph, self.translation)

    def test_empty_text_and_empty_trailing_sections_are_retained(self):
        site = self.graph["cod"]["text_sites"][0]
        site.update(text="", spoken_operands=[], choice_operands=[], sections=[[], []])
        self.translation["messages"]["bbb.script1.cod.00000002"] = ["", ""]
        apply_english(self.graph, self.translation)
        self.assertEqual(display_text(site), "")
        self.assertEqual(site["display"]["sections"], ["", ""])

    def test_failed_later_site_does_not_partially_localize_graph(self):
        self.graph["cod"]["text_sites"].append(dict(self.graph["cod"]["text_sites"][0], offset=20))
        self.translation["messages"]["bbb.script1.cod.00000014"] = ["Hello", "PLAY"]
        original = deepcopy(self.graph)
        with self.assertRaises(ValueError):
            apply_english(self.graph, self.translation)
        self.assertEqual(self.graph, original)

    def test_markdown_and_dot_use_english_display_fields(self):
        site = self.graph["cod"]["text_sites"][0]
        site.update(block=0, procedure="intro", record_name="Honk", dynamic=False,
                    resume_offset=None, skip_next_if_not_shown=None, control_word=None, recent_choice_count=0)
        self.graph["bas"] = None
        self.graph["cod"].update(instructions=[dict(offset=2, block=0)], control_flow=dict(
            blocks=[dict(start=0, procedure="intro")], edges=[], procedure_count=1, unresolved_guard_branches=[]))
        apply_english(self.graph, self.translation)
        for output in (profile_markdown(self.graph), cod_dot(self.graph)):
            self.assertIn("Hello", output)
            self.assertIn("PLAY", output)
            self.assertNotIn("Bonjour", output)
            self.assertNotIn("JOUER", output)
        self.assertEqual(site["text"], "Bonjour")


class DialogueExportTests(unittest.TestCase):
    def test_fence_cannot_be_closed_by_authored_text(self):
        self.assertEqual(fence("a ``` b"), "````text\na ``` b\n````")

    def test_cod_dot_keeps_guard_failure_and_resume_distinct(self):
        graph = {"cod": {"text_sites": [], "instructions": [], "control_flow": {
            "blocks": [dict(start=0, procedure='say "hello"'), dict(start=10, procedure="next")],
            "edges": [dict(from_block=0, to_block=10, from_instruction=4, kind="guard_failure"),
                      dict(from_block=10, to_block=0, from_instruction=10, kind="frame_resume")]}}}
        dot = cod_dot(graph)
        self.assertIn('guard_failure @0004', dot)
        self.assertIn('frame_resume @000A', dot)
        self.assertIn('style="dashed"', dot)
        self.assertIn(r'\"hello\"', dot)

    def test_missing_bas_match_is_explicit_not_an_end_node(self):
        graph = {"bas": {"control_flow": {"nodes": [dict(offset=2, selector_name="talk", list_index=0)],
                                          "lists": [dict(entrypoint=dict(object_name="Honk"))]},
                         "choice_edges": [dict(from_node=2, to_node=None, choice=dict(text="bye"))]}}
        self.assertIn('no local selector match', bas_dot(graph))
        self.assertIn('n2 -> unresolved0', bas_dot(graph))

    def test_summary_counts_sites_not_unique_strings_or_paths(self):
        site = dict(choice_operands=[dict(text="yes")], dynamic=False)
        graph = {"cod": {"text_sites": [site, site], "control_flow": {
            "procedure_count": 1, "unresolved_guard_branches": []}}, "bas": None}
        result = summarize(graph)
        self.assertEqual(result["text_sites"], 2)
        self.assertEqual(result["inline_choice_sites"], 2)


if __name__ == "__main__":
    unittest.main()
