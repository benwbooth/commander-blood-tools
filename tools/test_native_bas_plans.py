import copy
import unittest

from native_bas_plans import plan_topics, travel_procedure


def word(offset, text):
    return dict(offset=offset, text=text)


def site(offset, node, topic, flags=0x40):
    return dict(offset=offset, selector_node=node, flags_b4=flags, recent_choice_count=0,
                choice_operands=[dict(kind="dictionary", offset=topic)])


class BasPlanTests(unittest.TestCase):
    def setUp(self):
        self.contact = dict(script="SCRIPT2", procedure_offset=90, contact_object="Bob",
                            texts=[dict(opcode_offset=100)])
        nodes = [dict(offset=10, menu_offset=14,
                      menu_choices=[word(1, "bye_bye"), word(2, "topic"), word(3, "nested"), word(4, "random")]),
                 dict(offset=20, menu_offset=24,
                      menu_choices=[word(1, "bye_bye"), word(5, "detail"), word(6, "back")]),
                 dict(offset=30, menu_offset=34, menu_choices=[])]
        edges = [dict(from_node=node["offset"], choice=choice,
                      to_node={3: 20, 6: 10}.get(choice["offset"]))
                 for node in nodes for choice in node["menu_choices"]]
        self.graph = dict(game="cb", profile="SCRIPT2", resources=dict(cod_sha256="cod", dic_sha256="dic", bas_sha256="bas"),
                          cod=dict(text_sites=[dict(offset=100)]),
                          bas=dict(control_flow=dict(lists=[dict(entrypoint=dict(object_name="Bob", root_node=10),
                                                                  node_offsets=[10, 20, 30])], nodes=nodes),
                                   choice_edges=edges,
                                   text_sites=[site(100, 10, 2), site(110, 10, 2), site(120, 20, 5), site(130, 10, 4, 0x42)]))

    def test_each_topic_answer_requires_another_native_selection(self):
        plans, report = plan_topics(self.graph, self.contact)
        plan = plans[0][2]
        self.assertEqual(plan["required_bas_sites"], [100, 110])
        self.assertEqual([row["word_offset"] for row in plan["choices"]], [2, 2, 1])
        self.assertTrue(all(row["text_site"] == 14 for row in plan["choices"]))
        self.assertEqual(plan["contact_procedure"], 90)
        self.assertEqual(plan["bas_sha256"], "bas")

    def test_nested_menu_uses_the_first_match_and_preserves_the_entry_path(self):
        plans, report = plan_topics(self.graph, self.contact)
        nested = plans[1][2]
        self.assertEqual([(row["text_site"], row["word_offset"]) for row in nested["choices"]],
                         [(14, 3), (24, 5), (24, 1)])
        self.assertEqual(report["unvisited_menu_nodes"], [30])

    def test_random_sites_stay_unplanned_and_explicit(self):
        _, report = plan_topics(self.graph, self.contact)
        self.assertEqual(report["targeted_bas_sites"], [100, 110, 120])
        self.assertEqual(report["not_targeted_bas_sites"], [130])
        self.assertEqual(report["deferred"][0]["sites"], [130])

    def test_no_unique_exit_does_not_invent_a_close_command(self):
        self.graph["bas"]["control_flow"]["nodes"][1]["menu_choices"].pop(0)
        plans, report = plan_topics(self.graph, self.contact)
        self.assertEqual(len(plans), 1)
        self.assertIn(120, report["not_targeted_bas_sites"])

    def test_bas_source_hash_is_required(self):
        del self.graph["resources"]["bas_sha256"]
        with self.assertRaisesRegex(ValueError, "BAS source hash"):
            plan_topics(self.graph, self.contact)

    def test_profile_mismatch_is_rejected(self):
        self.contact["script"] = "SCRIPT3"
        with self.assertRaisesRegex(ValueError, "different profiles"):
            plan_topics(self.graph, self.contact)

    def test_ambiguous_actor_ownership_is_rejected(self):
        self.graph["bas"]["control_flow"]["lists"].append(copy.deepcopy(self.graph["bas"]["control_flow"]["lists"][0]))
        with self.assertRaisesRegex(ValueError, "one BAS selector list"):
            plan_topics(self.graph, self.contact)

    def test_travel_setup_replaces_onboard_contact_preparation(self):
        setup = dict(planet="world", destination="place", procedure_offset=90)
        plans, _ = plan_topics(self.graph, self.contact, setup)
        plan = plans[0][2]
        self.assertEqual(plan["entry"], "travel")
        self.assertEqual(plan["travel_setup"], setup)
        self.assertNotIn("contact_procedure", plan)
        setup["procedure_offset"] = 91
        with self.assertRaisesRegex(ValueError, "different procedure"):
            plan_topics(self.graph, self.contact, setup)

    def test_travel_procedure_must_have_an_authored_closed_travel_guard(self):
        self.graph["cod"]["instructions"] = [
            dict(offset=90, procedure="entry", instruction=dict(ConditionalBlock={})),
            dict(offset=94, procedure="entry", instruction=dict(FlagBranch=dict(opcode=0xD0))),
            dict(offset=95, procedure="entry", instruction=dict(GuardPop={})),
        ]
        self.graph["cod"]["text_sites"][0].update(procedure="entry", record_name="Bob")
        self.assertEqual(travel_procedure(self.graph, 90), self.contact)
        self.graph["cod"]["instructions"][1]["instruction"]["FlagBranch"]["opcode"] = 0xD1
        with self.assertRaisesRegex(ValueError, "no authored travel guard"):
            travel_procedure(self.graph, 90)
        self.graph["cod"]["instructions"].pop()
        with self.assertRaisesRegex(ValueError, "no closed entry guard"):
            travel_procedure(self.graph, 90)
        with self.assertRaisesRegex(ValueError, "not an authored procedure entry"):
            travel_procedure(self.graph, 91)


if __name__ == "__main__":
    unittest.main()
