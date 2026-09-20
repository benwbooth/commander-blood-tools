import copy
import unittest

from native_dialogue_coverage import add_chapter, add_graph, summarize


class CoverageTests(unittest.TestCase):
    def setUp(self):
        self.profiles, self.sites = {}, {}
        self.graph = dict(game="cb", profile="SCRIPT1", resources=dict(cod_sha256="cod", dic_sha256="dic"),
                          cod=dict(text_sites=[dict(offset=100, text="First"), dict(offset=200, text="Second")]),
                          bas=dict(text_sites=[dict(offset=100, text="BAS")]))
        add_graph(self.graph, self.profiles, self.sites)
        self.plan = dict(game="commander_blood", initial_profile=0, cod_sha256="cod", dic_sha256="dic")
        self.chapter = dict(published_cod_sites=[100], expected_unpublished_cod_sites=[200],
                            ui_raster_evidence={"100": dict(ui_raster_frames=5, fully_revealed_ui_frames=2)})

    def apply(self, capture=0):
        add_chapter(self.plan, self.chapter, self.profiles, self.sites, capture)

    def test_bas_offsets_do_not_alias_cod(self):
        self.apply()
        counts = summarize(self.sites)["cb"]
        self.assertEqual(counts, dict(total=3, ui_fully_revealed=1,
                                     not_published_on_selected_branches=1, uncovered=1))
        self.assertFalse(self.sites[("cb", "SCRIPT1", "bas", 100)]["publications"])

    def test_captures_are_deduplicated_by_site_not_counted_as_new_text(self):
        self.apply()
        self.apply(1)
        self.assertEqual(summarize(self.sites)["cb"]["ui_fully_revealed"], 1)
        self.assertEqual(self.sites[("cb", "SCRIPT1", "cod", 100)]["ui_full"], [0, 1])

    def test_absent_branch_does_not_override_another_branches_publication(self):
        self.apply()
        self.chapter["expected_unpublished_cod_sites"] = []
        self.chapter["published_cod_sites"] = [200]
        self.apply(1)
        self.assertEqual(summarize(self.sites)["cb"]["published_without_ui_raster"], 1)

    def test_source_hashes_are_required(self):
        self.plan["cod_sha256"] = "different"
        with self.assertRaisesRegex(ValueError, "source hashes"):
            self.apply()

    def test_bas_has_independent_source_binding_and_site_accounting(self):
        self.chapter["published_bas_sites"] = [100]
        self.chapter["bas_ui_raster_evidence"] = {"100": dict(ui_raster_frames=3, fully_revealed_ui_frames=1)}
        with self.assertRaisesRegex(ValueError, "BAS hash"):
            self.apply()
        self.plan["bas_sha256"] = "bas"
        self.profiles[("cb", "SCRIPT1")]["bas_sha256"] = "bas"
        self.apply()
        self.assertEqual(summarize(self.sites)["cb"]["ui_fully_revealed"], 2)
        self.assertEqual(self.sites[("cb", "SCRIPT1", "bas", 100)]["publications"], [0])

    def test_unknown_site_is_rejected(self):
        self.chapter["published_cod_sites"] = [300]
        with self.assertRaisesRegex(ValueError, "unknown static COD site"):
            self.apply()

    def test_profile_offsets_do_not_alias(self):
        graph = copy.deepcopy(self.graph)
        graph["profile"] = "SCRIPT2"
        add_graph(graph, self.profiles, self.sites)
        self.apply()
        self.assertFalse(self.sites[("cb", "SCRIPT2", "cod", 100)]["publications"])

    def test_full_reveal_requires_raster_frames(self):
        self.chapter["ui_raster_evidence"]["100"]["ui_raster_frames"] = 0
        with self.assertRaisesRegex(ValueError, "exceeds raster"):
            self.apply()


if __name__ == "__main__":
    unittest.main()
