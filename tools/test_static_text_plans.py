import copy
import unittest

from static_text_plans import candidate


class StaticTextPlanTests(unittest.TestCase):
    def setUp(self):
        self.descriptions = {"cb.descript.0000": dict(authored=dict(
            name="Actor", name_bytes=list(b"Actor"), kind="Character"))}
        self.site = dict(id="cb.script2.cod.0000002a", game="cb", profile="SCRIPT2", kind="cod", offset=42,
                         content_kind="text", source_hashes=dict(cod_sha256="code", dic_sha256="dictionary", bas_sha256="bas"),
                         direct_description_candidates=["cb.descript.0000"],
                         authored=dict(flags_b4=0x20, flags_b5=0x80, presentation_selector=-1, record_offset=500,
                                       sections=[[dict(kind="dictionary", offset=25, text="Hello")]]))

    def test_source_binding_and_explicit_prepared_context(self):
        plan, reason = candidate(self.site, self.descriptions)
        self.assertIsNone(reason)
        self.assertEqual(plan["source"], "cod")
        self.assertEqual(plan["text_site"], 42)
        self.assertEqual(plan["initial_profile"], 1)
        self.assertEqual(plan["cod_sha256"], "code")
        self.assertEqual(plan["context"], dict(kind="source_default", actor_offset=500, descript_records=["Actor"]))
        self.assertNotIn("bas_sha256", plan)

    def test_bas_source_requires_independent_hash(self):
        self.site["kind"] = "bas"
        plan, reason = candidate(self.site, self.descriptions)
        self.assertIsNone(reason)
        self.assertEqual(plan["bas_sha256"], "bas")
        del self.site["source_hashes"]["bas_sha256"]
        with self.assertRaisesRegex(ValueError, "BAS candidate"):
            candidate(self.site, self.descriptions)

    def test_all_gate_bits_are_deferred_without_modifying_source(self):
        for bit in (2, 4, 16, 64):
            with self.subTest(bit=bit):
                source = copy.deepcopy(self.site)
                source["authored"]["flags_b4"] |= bit
                before = copy.deepcopy(source)
                self.assertEqual(candidate(source, self.descriptions), (None, "conditional_or_continuation_control"))
                self.assertEqual(source, before)

    def test_rejection_skip_does_not_gate_a_presented_line(self):
        self.site["authored"]["flags_b4"] |= 0x08
        before = copy.deepcopy(self.site)
        plan, reason = candidate(self.site, self.descriptions)
        self.assertIsNone(reason)
        self.assertEqual(plan["text_site"], self.site["offset"])
        self.assertEqual(self.site, before)

    def test_history_line_prepares_only_its_authored_candidates(self):
        authored = self.site["authored"]
        authored["flags_b4"] = 0x41
        authored["sections"].append([dict(kind="dictionary", offset=6, text="hello")])
        self.site["content_kind"] = "text_and_choices"
        plan, reason = candidate(self.site, self.descriptions)
        self.assertIsNone(reason)
        self.assertEqual(plan["context"]["history_concepts"], [6])
        self.assertIn("concept history", plan["title"])
        for bits in (0x42, 0x44, 0x50):
            authored["flags_b4"] = bits
            with self.subTest(bits=bits):
                self.assertIsNotNone(candidate(self.site, self.descriptions)[1])
        authored["flags_b4"] = 0x40
        authored["sections"].append([dict(kind="dictionary", offset=7, text="hi")])
        self.assertEqual(candidate(self.site, self.descriptions), (None, "text_and_choices"))

    def test_console_records_present_over_the_bridge(self):
        for name, choice in (("Honk", "horn"), ("menu", "radio")):
            site = copy.deepcopy(self.site)
            site["authored"]["record_name"] = name
            site["direct_description_candidates"] = []
            plan, reason = candidate(site, self.descriptions)
            with self.subTest(name=name):
                self.assertIsNone(reason)
                self.assertEqual(plan["context"], dict(kind="bridge_console",
                                                       actor_offset=site["authored"]["record_offset"],
                                                       choice=choice))
                self.assertIn("bridge console", plan["title"])

    def test_symbolic_inventory_and_control_remain_explicit(self):
        for kind in ("symbolic_text", "inventory_menu", "control_only", "text_and_choices"):
            self.site["content_kind"] = kind
            self.assertEqual(candidate(self.site, self.descriptions), (None, kind))

    def test_actor_context_is_never_guessed_from_similar_name(self):
        self.site["direct_description_candidates"] = []
        self.assertEqual(candidate(self.site, self.descriptions), (None, "no_direct_character_description"))

    def test_sequence_is_not_treated_as_a_character(self):
        self.descriptions["cb.descript.0000"]["authored"]["kind"] = "Sequence"
        self.assertEqual(candidate(self.site, self.descriptions), (None, "no_direct_character_description"))

    def test_unsigned_graph_selector_255_is_native_minus_one(self):
        self.site["authored"]["presentation_selector"] = 255
        self.assertIsNone(candidate(self.site, self.descriptions)[1])
        self.site["authored"]["presentation_selector"] = 254
        self.assertEqual(candidate(self.site, self.descriptions), (None, "non_character_presentation_selector"))


if __name__ == "__main__":
    unittest.main()
