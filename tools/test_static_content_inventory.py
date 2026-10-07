import copy
import unittest

from static_content_inventory import (join_inventory, validate_description_commands,
                                      validate_media_manifest, validate_profile_sources)


class StaticContentInventoryTests(unittest.TestCase):
    def setUp(self):
        self.site = dict(offset=10, record_name="Actor", record_offset=50,
                         sections=[[dict(kind="dictionary", offset=2, text="Hello")]],
                         spoken_operands=[dict(kind="dictionary", offset=2, text="Hello")],
                         choice_operands=[], text="Hello", flags_b4=0, flags_b5=128)
        self.graph = dict(game="cb", profile="SCRIPT1", resources=dict(cod_sha256="cod", dic_sha256="dic"),
                          cod=dict(text_sites=[self.site]), bas=None)
        self.media = dict(cb=dict(game="commander_blood",
                                 descript=dict(sha256="descript", ordered_commands_exported=True),
                                 records=[dict(name="Actor", name_bytes=list(b"Actor"), kind="Character",
                                               commands=[dict(kind="talk", resource="PE/TALK.HNM")])],
                                 videos=[dict(name="PE/TALK.HNM", sha256="video")]))

    def result(self):
        return join_inventory([self.graph], self.media)

    def test_inventory_is_not_render_or_route_coverage(self):
        result = self.result()
        self.assertTrue(result["static_inventory_complete"])
        self.assertFalse(result["render_complete"])
        self.assertFalse(result["full_game_complete"])
        self.assertEqual(result["counts"]["cb"]["rendered_sites"], 0)
        site = result["sites"][0]
        self.assertEqual(site["render_status"], "not_rendered")
        self.assertEqual(site["gameplay_reachability"], "not_assessed")
        self.assertEqual(site["direct_description_candidates"], ["cb.descript.0000"])

    def test_duplicate_words_never_drop_source_sites(self):
        duplicate = dict(self.site, offset=20)
        self.graph["cod"]["text_sites"].append(duplicate)
        result = self.result()
        self.assertEqual(len(result["sites"]), 2)
        self.assertEqual(result["counts"]["cb"]["authored_request_groups"], 1)
        self.assertNotEqual(result["sites"][0]["id"], result["sites"][1]["id"])

    def test_render_context_changes_candidate_identity(self):
        original = self.result()["sites"][0]["authored_request_fingerprint"]
        for key, value in (("record_name", "AnotherActor"), ("flags_b4", 32),
                           ("control_word", 22), ("chatter", True), ("active_line", 12)):
            with self.subTest(key=key):
                graph = copy.deepcopy(self.graph)
                graph["cod"]["text_sites"][0][key] = value
                changed = join_inventory([graph], self.media)["sites"][0]
                self.assertNotEqual(changed["authored_request_fingerprint"], original)

    def test_profile_and_bas_namespace_are_distinct(self):
        self.graph["bas"] = dict(text_sites=[copy.deepcopy(self.site)])
        graph = copy.deepcopy(self.graph)
        graph["profile"] = "SCRIPT2"
        sites = join_inventory([self.graph, graph], self.media)["sites"]
        self.assertEqual(len({site["id"] for site in sites}), 4)
        self.assertEqual(len({site["authored_request_fingerprint"] for site in sites}), 4)

    def test_dynamic_values_stay_symbolic(self):
        number = dict(kind="state_number", offset=123)
        self.site["sections"][0].append(number)
        self.site["spoken_operands"].append(number)
        site = self.result()["sites"][0]
        self.assertEqual(site["content_kind"], "symbolic_text")
        self.assertEqual(site["state_number_operands"], [123])
        self.assertEqual(site["authored"], self.site)

    def test_empty_inventory_generator_is_not_empty_control(self):
        self.site.update(sections=[[], [dict(kind="inventory_choices")]], spoken_operands=[],
                         choice_operands=[dict(kind="inventory_choices")])
        self.assertEqual(self.result()["sites"][0]["content_kind"], "inventory_menu")

    def test_all_ordered_commands_are_retained(self):
        self.media["cb"]["records"][0]["commands"] += [
            dict(kind="sequence_subtitle", first_visible_frame=30, text_bytes=[255, 65]),
            dict(kind="location_layout", top_row=42)]
        result = self.result()
        self.assertEqual(result["descriptions"][0]["authored"], self.media["cb"]["records"][0])
        self.assertEqual(result["counts"]["cb"]["description_commands"], 3)

    def test_legacy_partial_catalog_is_rejected(self):
        self.media["cb"]["descript"]["ordered_commands_exported"] = False
        with self.assertRaisesRegex(ValueError, "full ordered"):
            self.result()

    def test_duplicate_sites_are_rejected(self):
        self.graph["cod"]["text_sites"].append(copy.deepcopy(self.site))
        with self.assertRaisesRegex(ValueError, "duplicate authored"):
            self.result()

    def test_name_case_is_not_guessed(self):
        self.site["record_name"] = "ACTOR"
        self.assertEqual(self.result()["sites"][0]["direct_description_candidates"], [])

    def test_duplicate_description_names_preserve_directory_order(self):
        self.media["cb"]["records"].append(copy.deepcopy(self.media["cb"]["records"][0]))
        result = self.result()
        self.assertEqual(len(result["descriptions"]), 2)
        self.assertEqual(result["sites"][0]["direct_description_candidates"],
                         ["cb.descript.0000", "cb.descript.0001"])

    def test_translation_ids_do_not_prevent_request_grouping(self):
        self.site["display"] = dict(catalog_id="bbb.script1.cod.0000000a", language="en", text="Hello")
        second = copy.deepcopy(self.site)
        second["offset"] = 20
        second["display"]["catalog_id"] = "bbb.script1.cod.00000014"
        self.graph["cod"]["text_sites"].append(second)
        result = self.result()
        self.assertEqual(result["counts"]["cb"]["authored_request_groups"], 1)
        self.assertNotEqual(result["sites"][0]["authored"]["display"]["catalog_id"],
                            result["sites"][1]["authored"]["display"]["catalog_id"])


class DescriptionByteValidationTests(unittest.TestCase):
    def setUp(self):
        command = b"\x0d\x01\0Text\xff\0"
        self.image = (b"\x01\0" + b"test".ljust(16, b"\0") + b"\x15\0\x04"
                      + (len(command) + 3).to_bytes(2, "little") + command + b"\xff")
        self.catalog = dict(records=[dict(name_bytes=list(b"test"), kind="Sequence", commands=[dict(
            source_offset=23, source_byte_count=len(command), source_bytes=list(command))])])

    def test_lossless_command_stream_matches_source(self):
        validate_description_commands(self.catalog, self.image)

    def test_omitted_command_is_rejected(self):
        self.catalog["records"][0]["commands"] = []
        with self.assertRaisesRegex(ValueError, "incomplete"):
            validate_description_commands(self.catalog, self.image)

    def test_modified_original_byte_is_rejected(self):
        self.catalog["records"][0]["commands"][0]["source_bytes"][3] = 0
        with self.assertRaisesRegex(ValueError, "command bytes"):
            validate_description_commands(self.catalog, self.image)

    def test_changed_directory_order_is_rejected(self):
        self.catalog["records"][0]["name_bytes"] = list(b"other")
        with self.assertRaisesRegex(ValueError, "directory name"):
            validate_description_commands(self.catalog, self.image)


class ImportedSourceBindingTests(unittest.TestCase):
    def setUp(self):
        self.video = dict(resource_name="PE/ACTOR.HNM", path="resources/PE/ACTOR.HNM", byte_count=123, sha256="video")
        self.description = dict(resource_name="DESCRIPT.DES", path="resources/DESCRIPT.DES", byte_count=12, sha256="descript")
        self.manifest = dict(resources=[self.video, self.description])
        self.catalog = dict(game="commander_blood", videos=[dict(self.video, name="PE/ACTOR.HNM")],
                            descript={key: self.description[key] for key in ("path", "byte_count", "sha256")})

    def test_complete_media_inventory_matches_original(self):
        self.assertEqual(set(validate_media_manifest(self.catalog, self.manifest)), {"PE/ACTOR.HNM", "DESCRIPT.DES"})

    def test_truncated_video_catalog_is_rejected(self):
        self.catalog["videos"] = []
        with self.assertRaisesRegex(ValueError, "complete imported HNM census"):
            validate_media_manifest(self.catalog, self.manifest)

    def test_changed_resource_hash_path_or_size_is_rejected(self):
        for field, value in (("sha256", "another"), ("byte_count", 124), ("path", "wrong")):
            catalog = copy.deepcopy(self.catalog)
            catalog["videos"][0][field] = value
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, "metadata differs"):
                validate_media_manifest(catalog, self.manifest)

    def test_script_graph_must_match_the_media_revision(self):
        graph = dict(game="cb", profile="SCRIPT2", resources=dict(cod_sha256="cod", dic_sha256="dic", bas_sha256="bas"))
        resources = {f"SCRIPT2.{kind.upper()}": dict(sha256=kind) for kind in ("cod", "dic", "bas")}
        validate_profile_sources(graph, resources)
        for kind in ("cod", "dic", "bas"):
            changed = copy.deepcopy(resources)
            changed[f"SCRIPT2.{kind.upper()}"]["sha256"] = "changed"
            with self.subTest(kind=kind), self.assertRaisesRegex(ValueError, "differs from imported"):
                validate_profile_sources(graph, changed)


if __name__ == "__main__":
    unittest.main()
