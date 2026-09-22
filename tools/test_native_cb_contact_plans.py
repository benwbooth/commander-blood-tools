import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

from native_cb_contact_plans import build
import test_native_bas_plans as bas_fixtures
from video_anthology import digest, save_json


class AllContactPlanTests(unittest.TestCase):
    def test_source_order_and_state_warning(self):
        fixture = bas_fixtures.BasPlanTests()
        fixture.setUp()
        graph = fixture.graph
        graph["cod"]["instructions"] = [
            dict(offset=90, procedure="entry_a", instruction=dict(ConditionalBlock={})),
            dict(offset=91, procedure="entry_b", instruction=dict(ConditionalBlock={})),
        ]
        graph["cod"]["text_sites"] = [
            dict(offset=100, procedure="entry_a", choice_operands=[]),
            dict(offset=101, procedure="entry_b", choice_operands=[dict(kind="dictionary")]),
        ]
        with TemporaryDirectory() as temp:
            root = Path(temp)
            catalog = root / "catalog"
            profile = catalog / "cb" / "script2"
            profile.mkdir(parents=True)
            save_json(profile / "graph.json", graph)
            relative = "cb/script2/graph.json"
            save_json(catalog / "catalog.json", dict(
                profiles=[dict(game="cb", profile="SCRIPT2", directory="cb/script2")],
                artifacts={relative: digest(profile / "graph.json")}))
            manifest = root / "contacts.json"
            save_json(manifest, dict(procedures=[
                dict(script="SCRIPT2", procedure_offset=91, contact_object="Bob", entry_class="direct",
                     texts=[dict(opcode_offset=101)]),
                dict(script="SCRIPT2", procedure_offset=90, contact_object="Bob", entry_class="conditioned",
                     texts=[dict(opcode_offset=100)]),
            ]))
            out = root / "plans"
            build(catalog, manifest, out)
            planning = json.loads((out / "planning.json").read_text())
            self.assertEqual([row["procedure_offset"] for row in planning["contacts"]], [90, 91])
            self.assertEqual(len(planning["plans"]), 4)
            self.assertEqual(len({json.loads((out / name).read_text())["title"]
                                  for name in planning["plans"]}), 4)
            self.assertEqual(planning["contacts"][1]["cod_choice_offsets"], [101])
            self.assertIn("does not prove", planning["contacts"][0]["state_resolution"])
            with self.assertRaisesRegex(ValueError, "already exists"):
                build(catalog, manifest, out)


if __name__ == "__main__":
    unittest.main()
