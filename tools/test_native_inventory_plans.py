import copy
import unittest

from native_inventory_plans import plan_inventory, travel_template


def instruction(offset, kind, **fields):
    return dict(offset=offset, procedure="gifts", instruction={kind: fields})


class InventoryPlanTests(unittest.TestCase):
    def setUp(self):
        self.template = dict(game="big_bug_bang", initial_profile=16, target="Cyberquizz", entry="travel",
            cod_sha256="cod", dic_sha256="dic", choices=[], required_cod_sites=[400],
            required_frame_boundary_cod_sites=[400], travel_setup=dict(procedure_offset=90))
        self.graph = dict(game="bbb", profile="script17", resources=dict(cod_sha256="cod", dic_sha256="dic"),
            symbols=[dict(kind=1, offset=7352, name="technologie"), dict(kind=1, offset=7640, name="atomique")],
            cod=dict(instructions=[instruction(90, "ConditionalBlock", target=500),
                instruction(100, "GuardPush", target=150),
                instruction(103, "SharedBitState", field_offset=7354, mask=64, inverted=False, opcode=174),
                instruction(108, "GuardPop"), instruction(109, "Text"),
                instruction(140, "SharedBitState", field_offset=7354, mask=64, inverted=True, opcode=174)],
                text_sites=[dict(offset=109, procedure="gifts", flags_b4=8, choice_operands=[],
                                 dynamic=False, record_name="Cyberquizz"),
                            dict(offset=400, procedure="gifts", choice_operands=[dict(kind="inventory_choices")],
                                 record_name="Cyberquizz")]))
        self.labels = dict(technologie="technology", atomique="nuclear")

    def plan(self):
        return plan_inventory(self.graph, self.template, 400, self.labels)

    def test_source_record_identity_and_reaction_are_required(self):
        before = copy.deepcopy(self.template)
        plans, report = self.plan()
        item, plan = plans[0]
        self.assertEqual(item, 7352)
        self.assertEqual(plan["title"], "Cyberquizz: give technology")
        self.assertEqual(plan["travel_setup"]["stage_aboard_inventory"], [7352])
        self.assertEqual(plan["choices"], [dict(source="inventory", text_site=400, inventory_item=7352)])
        self.assertEqual(plan["required_frame_boundary_cod_sites"], [109, 400])
        self.assertEqual(report["items_without_simple_flag_guard"], [7640])
        self.assertEqual(self.template, before)

    def test_non_ascii_display_names_do_not_change_native_identity(self):
        self.graph["symbols"][0]["name"] = "trait\u00e9"
        self.labels["trait\u00e9"] = "treaty"
        plans, _ = self.plan()
        self.assertEqual(plans[0][1]["title"], "Cyberquizz: give treaty")
        self.assertEqual(plans[0][1]["choices"][0]["inventory_item"], 7352)

    def test_compound_and_nested_guards_are_deferred(self):
        original = copy.deepcopy(self.graph)
        for row in [instruction(104, "SharedState"), instruction(120, "GuardPush", target=140)]:
            self.graph = copy.deepcopy(original)
            self.graph["cod"]["instructions"].append(row)
            self.graph["cod"]["instructions"].sort(key=lambda row: row["offset"])
            plans, report = self.plan()
            self.assertFalse(plans)
            self.assertEqual(len(report["deferred"]), 1)

    def test_enclosing_condition_and_missing_consumption_are_deferred(self):
        original = copy.deepcopy(self.graph)
        self.graph["cod"]["instructions"].insert(1, instruction(95, "GuardPush", target=200))
        self.assertFalse(self.plan()[0])
        self.graph = original
        self.graph["cod"]["instructions"].pop()
        self.assertFalse(self.plan()[0])

    def test_dynamic_or_random_reaction_is_not_declared_covered(self):
        reaction = self.graph["cod"]["text_sites"][0]
        for key, bad in [("dynamic", True), ("flags_b4", 2), ("record_name", "Bioquizz")]:
            original = reaction[key]
            reaction[key] = bad
            plans, report = self.plan()
            self.assertFalse(plans)
            self.assertEqual(report["deferred"][0]["sites"], [109])
            reaction[key] = original

    def test_wrong_source_owner_or_disabled_procedure_is_rejected(self):
        for field, value, error in [("cod_sha256", "wrong", "source hash"),
                                    ("target", "Bioquizz", "actor's inventory"),
                                    ("initial_profile", 0, "different profiles")]:
            original = self.template[field]
            self.template[field] = value
            with self.assertRaisesRegex(ValueError, error):
                self.plan()
            self.template[field] = original
        self.template["travel_setup"]["procedure_offset"] = 80
        with self.assertRaisesRegex(ValueError, "not enabled"):
            self.plan()
        self.template["travel_setup"]["supporting_procedures"] = [90]
        self.assertTrue(self.plan()[0])

    def test_existing_choices_and_inventory_are_not_overwritten(self):
        self.template["choices"] = [dict(text_site=123, word_offset=4)]
        with self.assertRaisesRegex(ValueError, "unstocked travel template"):
            self.plan()
        self.template["choices"] = []
        self.template["travel_setup"]["stage_aboard_inventory"] = [7352]
        with self.assertRaisesRegex(ValueError, "unstocked travel template"):
            self.plan()


class InventoryTemplateTests(unittest.TestCase):
    def setUp(self):
        self.graph = dict(game="bbb", profile="SCRIPT16",
            resources=dict(cod_sha256="cod", dic_sha256="dic"),
            symbols=[dict(kind=1, name=name, offset=offset) for name, offset in
                     [("Mega_Paul", 1258), ("blood", 40), ("Cyberock", 5098), ("Cyberland", 5128)]],
            cod=dict(instructions=[instruction(4452, "ConditionalBlock", target=5593),
                instruction(4456, "FlagBranch", opcode=0xD0),
                instruction(4457, "Actor", record_offset=1316, related_record_offset=40, inverted=False),
                instruction(4462, "GuardPop")],
                text_sites=[dict(offset=5540, procedure="gifts", record_name="Mega_Paul",
                                 record_offset=1258, choice_operands=[dict(kind="inventory_choices")])]))

    def template(self):
        return travel_template(self.graph, 5540, "Cyberock", "Cyberland")

    def test_derives_only_prepared_entry_from_authored_guard(self):
        original = copy.deepcopy(self.graph)
        template = self.template()
        self.assertEqual(template["initial_profile"], 15)
        self.assertEqual(template["target"], "Mega_Paul")
        self.assertEqual(template["cod_sha256"], "cod")
        self.assertEqual(template["travel_setup"], dict(planet="Cyberock", destination="Cyberland",
            procedure_offset=4452, stage_actor_at_destination=True))
        self.assertEqual(template["choices"], [])
        self.assertEqual(template["required_cod_sites"], [])
        self.assertEqual(self.graph, original)

    def test_additional_outer_condition_is_not_guessed(self):
        self.graph["cod"]["instructions"].insert(3, instruction(4461, "SharedState"))
        with self.assertRaisesRegex(ValueError, "additional preparation"):
            self.template()

    def test_wrong_actor_activity_and_player_are_rejected(self):
        rows = self.graph["cod"]["instructions"]
        activity = rows[1]["instruction"]["FlagBranch"]
        actor = rows[2]["instruction"]["Actor"]
        for target, key, value in [(activity, "opcode", 0xD1), (actor, "inverted", True),
                                   (actor, "record_offset", 1390), (actor, "related_record_offset", 74)]:
            previous = target[key]
            target[key] = value
            with self.assertRaisesRegex(ValueError, "additional preparation"):
                self.template()
            target[key] = previous

    def test_unclosed_guard_is_rejected(self):
        self.graph["cod"]["instructions"].pop()
        with self.assertRaisesRegex(ValueError, "closed entry guard"):
            self.template()

    def test_unbound_menu_and_unknown_destination_are_rejected(self):
        with self.assertRaisesRegex(ValueError, "named actor inventory"):
            travel_template(self.graph, 5541, "Cyberock", "Cyberland")
        with self.assertRaisesRegex(ValueError, "unknown prepared travel"):
            travel_template(self.graph, 5540, "Cyberock", "Nowhere")
        self.graph["cod"]["text_sites"][0]["record_offset"] = 1332
        with self.assertRaisesRegex(ValueError, "symbol table"):
            self.template()

    def test_other_game_and_profile_are_rejected(self):
        for key, value in [("game", "cb"), ("profile", "SCRIPT0"), ("profile", "SCRIPT18")]:
            previous = self.graph[key]
            self.graph[key] = value
            with self.assertRaisesRegex(ValueError, "BBB profile"):
                self.template()
            self.graph[key] = previous


if __name__ == "__main__":
    unittest.main()
