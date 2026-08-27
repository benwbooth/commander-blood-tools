#!/usr/bin/env python3

import pathlib
import sys
import unittest


sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

from audit_rust_port_routing import (
    PortedRoutine,
    production_text,
    retained,
    source_module,
    source_routed,
    without_function_body,
)


class RustPortRoutingAuditTests(unittest.TestCase):
    def test_source_paths_become_exact_crate_modules(self) -> None:
        self.assertEqual(
            source_module(
                pathlib.PurePosixPath(
                    "crates/commander-blood-game/src/native/alien/control.rs"
                )
            ),
            "commander_blood_game::native::alien::control",
        )
        self.assertEqual(
            source_module(
                pathlib.PurePosixPath("crates/commander-blood-game/src/native/alien/mod.rs")
            ),
            "commander_blood_game::native::alien",
        )
        self.assertEqual(
            source_module(
                pathlib.PurePosixPath("crates/commander-blood-formats/src/archive.rs")
            ),
            "commander_blood_formats::archive",
        )

    def test_ledger_rows_qualify_free_and_inherent_methods(self) -> None:
        free = PortedRoutine(
            "xdb_amer",
            "0x000734",
            pathlib.PurePosixPath(
                "crates/commander-blood-game/src/native/alien/starfield.rs"
            ),
            "generate_starfield",
        )
        method = PortedRoutine(
            "xdb_amer",
            "0x000223",
            pathlib.PurePosixPath("crates/commander-blood-game/src/native/alien/control.rs"),
            "AlienCameraControl::step",
        )
        self.assertEqual(
            free.qualified_symbol,
            "commander_blood_game::native::alien::starfield::generate_starfield",
        )
        self.assertEqual(
            method.qualified_symbol,
            "commander_blood_game::native::alien::control::AlienCameraControl::step",
        )

    def test_retention_is_module_specific_and_accepts_trait_forms(self) -> None:
        symbols = {
            "commander_blood_game::native::alien::starfield::generate_starfield",
            "<commander_blood_game::native::alien::control::AlienCameraControl as "
            "some_crate::Control>::step",
        }
        self.assertTrue(
            retained(
                "commander_blood_game::native::alien::starfield::generate_starfield",
                symbols,
            )
        )
        self.assertTrue(
            retained(
                "commander_blood_game::native::alien::control::AlienCameraControl::step",
                symbols,
            )
        )
        self.assertFalse(
            retained(
                "commander_blood_game::native::bloodprg::starfield::generate_starfield",
                symbols,
            )
        )

    def test_source_routing_ignores_tests_reexports_and_recursion(self) -> None:
        path = pathlib.PurePosixPath(
            "crates/commander-blood-game/src/native/alien/starfield.rs"
        )
        routine = PortedRoutine("xdb_amer", "0x000734", path, "generate_starfield")
        source = production_text(
            "pub use other::generate_starfield;\n"
            "pub fn generate_starfield() { generate_starfield(); }\n"
            "#[cfg(test)] mod tests { fn test() { generate_starfield(); } }\n"
        )
        self.assertFalse(source_routed(routine, {path: source}))

        caller_path = pathlib.PurePosixPath(
            "crates/commander-blood-game/src/native/alien/scene.rs"
        )
        self.assertTrue(
            source_routed(
                routine,
                {path: source, caller_path: "fn frame() { generate_starfield(); }"},
            )
        )
        collision_path = pathlib.PurePosixPath(
            "crates/commander-blood-game/src/runtime/other.rs"
        )
        self.assertFalse(
            source_routed(
                routine,
                {
                    path: source,
                    caller_path: "fn frame() { generate_starfield(); }",
                    collision_path: "fn generate_starfield() {}",
                },
            )
        )

    def test_function_removal_preserves_neighboring_callers(self) -> None:
        source = "fn target() { target(); }\nfn caller() { target(); }\n"
        stripped = without_function_body(source, "target")
        self.assertNotIn("fn target", stripped)
        self.assertIn("fn caller() { target(); }", stripped)


if __name__ == "__main__":
    unittest.main()
