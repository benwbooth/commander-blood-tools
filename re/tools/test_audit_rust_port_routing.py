#!/usr/bin/env python3

import pathlib
import sys
import tempfile
import unittest


sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

from audit_rust_port_routing import (
    PortedRoutine,
    RoutingDisposition,
    production_text,
    retained,
    source_module,
    source_routed,
    validate_evidence_reference,
    validate_dispositions,
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

    def test_dispositions_require_current_missing_rows_and_real_evidence(self) -> None:
        routine = PortedRoutine(
            "xdb_scrut",
            "0x0017e1",
            pathlib.PurePosixPath(
                "crates/commander-blood-game/src/native/alien/slot2.rs"
            ),
            "begin_scrut_fade",
        )
        disposition = RoutingDisposition(
            component=routine.component,
            entry=routine.entry,
            disposition="native_unreachable",
            evidence=("proof.txt:1",),
            rationale="Static references prove that the callback has no caller.",
        )
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            (root / "proof.txt").write_text("reviewed evidence\n", encoding="utf-8")
            validate_dispositions(
                {disposition.key: disposition}, [routine], [routine], root
            )
            with self.assertRaisesRegex(ValueError, "stale disposition"):
                validate_dispositions(
                    {disposition.key: disposition}, [routine], [], root
                )

    def test_dispositions_reject_unknown_categories_and_bad_evidence(self) -> None:
        routine = PortedRoutine(
            "xdb_scrut",
            "0x0017e1",
            pathlib.PurePosixPath(
                "crates/commander-blood-game/src/native/alien/slot2.rs"
            ),
            "begin_scrut_fade",
        )
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            (root / "proof.txt").write_text("reviewed evidence\n", encoding="utf-8")
            unknown = RoutingDisposition(
                routine.component,
                routine.entry,
                "guess",
                ("proof.txt:1",),
                "Not an allowed classification.",
            )
            with self.assertRaisesRegex(ValueError, "unsupported disposition"):
                validate_dispositions({unknown.key: unknown}, [routine], [routine], root)

            missing = RoutingDisposition(
                routine.component,
                routine.entry,
                "native_unreachable",
                ("missing.txt:1",),
                "References were audited.",
            )
            with self.assertRaisesRegex(ValueError, "evidence path does not exist"):
                validate_dispositions({missing.key: missing}, [routine], [routine], root)

    def test_anchored_evidence_requires_one_stable_source_match(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            (root / "proof.rs").write_text(
                "fn translated_owner() {}\nfn other() {}\n",
                encoding="utf-8",
            )
            validate_evidence_reference("proof.rs#fn translated_owner", root)
            with self.assertRaisesRegex(ValueError, "anchor is absent"):
                validate_evidence_reference("proof.rs#fn missing_owner", root)

            (root / "ambiguous.rs").write_text(
                "fn owner() {}\nfn owner() {}\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(ValueError, "anchor is ambiguous"):
                validate_evidence_reference("ambiguous.rs#fn owner", root)


if __name__ == "__main__":
    unittest.main()
