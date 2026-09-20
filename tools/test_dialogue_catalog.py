import unittest

from dialogue_catalog import bas_dot, cod_dot, fence, summarize


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
