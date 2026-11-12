import tempfile
from pathlib import Path
import unittest

import play_order

SOURCE = '''bloodscript 8
profile SCRIPT1

state {
    planet "Rondo" {
        active
    }
    character "Maxxon" {
        active
        location = "Rondo"
    }
}

logic {
    proc intro enabled {
        during bridge
        require presentation == Honk
    } then {
        say Honk presentation=text_only : "Rondo is at x345"
        globals.told = 1
    }
    proc later enabled {
        require globals.told == 1
    } then {
        say Honk presentation=text_only : "second round"
        when {
            require globals.done == 1
        } then {
            say Honk presentation=text_only : "third round"
        }
        globals.done = 1
    }
    proc never enabled {
        require globals.told == 7
    } then {
        say Honk presentation=text_only : "unreachable"
    }
    proc menu1 enabled {
        during bridge
        require presentation == menu
    } then {
        say menu presentation=text_only : "MENU"
        menu1.enabled = false
    }
    proc alias_2 as "alias" disabled {
        when {
            require globals.done == 1
        } then {
            alias_2.enabled = false
        }
    }
    global told = 0
    global done = 0
}
'''


class PlayOrderTest(unittest.TestCase):
    def analyse(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "source.blood"
            path.write_text(SOURCE)
            return play_order.analyse_profile(path)

    def test_rounds_follow_effects(self):
        analysis = self.analyse()
        lines = SOURCE.splitlines()
        def round_of(text):
            line = next(i for i, l in enumerate(lines) if text in l)
            return analysis["reached"].get(line)
        self.assertEqual(round_of('"Rondo is at x345"'), 0)
        self.assertEqual(round_of('"second round"'), 1)
        self.assertEqual(round_of('"third round"'), 2)  # needs its own proc's later effect
        self.assertIsNone(round_of('"unreachable"'))

    def test_state_changing_procs_precede_passive_ones_within_a_round(self):
        analysis = self.analyse()
        procs = {p.name: p for p in analysis["procs"]}
        self.assertTrue(play_order.drives(procs["intro"]))
        self.assertFalse(play_order.drives(procs["menu1"]))
        lines = SOURCE.splitlines()
        intro = analysis["keys"][next(i for i, l in enumerate(lines) if "x345" in l)]
        menu = analysis["keys"][next(i for i, l in enumerate(lines) if '"MENU"' in l)]
        self.assertLess(intro, menu)

    def test_guardless_and_aliased_procs_parse(self):
        procs = {p.name: p for p in self.analyse()["procs"]}
        self.assertEqual(procs["alias_2"].alias, "alias")
        self.assertEqual(procs["alias_2"].guards, [])
        self.assertFalse(procs["alias_2"].enabled)


if __name__ == "__main__":
    unittest.main()
