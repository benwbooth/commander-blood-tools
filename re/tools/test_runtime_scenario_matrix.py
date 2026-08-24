#!/usr/bin/env python3
"""Unit tests for the deterministic runtime scenario matrix."""
from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


MATRIX_PATH = Path(__file__).with_name("runtime_scenario_matrix.py")
SPEC = importlib.util.spec_from_file_location(
    "runtime_scenario_matrix", MATRIX_PATH
)
assert SPEC is not None and SPEC.loader is not None
matrix = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = matrix
SPEC.loader.exec_module(matrix)


def valid_teleport_report(profile: int) -> dict[str, object]:
    handles = [profile * 5 + index + 1 for index in range(5)]
    return {
        "verdict": "TELEPORTS-COMPLETE",
        "anomalies": [],
        "teleports": [
            {
                "target": profile,
                "completed_sample": 10,
                "completed_state": {
                    "profile": profile,
                    "request": -1,
                    "execution_enabled": 1,
                    "handles": handles,
                    "expected_handles": handles,
                    "images": [f"1234:{index:04x}" for index in range(5)],
                    "blockers": {
                        "vm_ui": 0,
                        "presentation": 0,
                        "ship": 0,
                    },
                },
            }
        ],
    }


def valid_radio_report() -> dict[str, object]:
    return {
        "verdict": "RADIO-PROBE-COMPLETE",
        "anomalies": [],
        "radio_probe": {
            "completed_sample": 20,
            "checkpoints": [
                {
                    "menu_words_offset": offset if offset is not None else 0x2BDB,
                    "subtitle": text,
                }
                for offset, text in matrix.SCRIPT2_RADIO_CHECKPOINTS
            ],
        },
    }


def valid_bob_report() -> dict[str, object]:
    return {
        "verdict": "BOB-PROBE-COMPLETE",
        "anomalies": [],
        "bob_probe": {
            "completed_sample": 30,
            "checkpoints": [
                {"menu_words_offset": offset, "subtitle": text}
                for offset, text in matrix.SCRIPT1_BOB_CHECKPOINTS
            ],
        },
    }


def valid_contact_report(selector: str) -> dict[str, object]:
    return {
        "verdict": "CONTACT-PROBE-COMPLETE",
        "anomalies": [],
        "contact_probe": {
            "selector": selector,
            "completed_sample": 40,
            "completion_reason": "line-target",
            "contact_object_offset": 0x004A,
            "setup": {"selected_object": 0x004A},
            "checkpoints": [
                {"menu_words_offset": 0x078E, "subtitle": "Good day COMMANDER"}
            ],
        },
    }


def valid_pterra_report() -> dict[str, object]:
    return {
        "mode": "authentic-save-pterra",
        "errors": [],
        "title_transition_confirmed": True,
        "title_transition_evidence": [
            "startup-presentation-line",
            "native-gameplay-load-boundary",
            "authentic-save-loaded",
        ],
        "authentic_save_loaded": True,
        "fault_detected": False,
        "dos_read_overflow_detected": False,
        "integrity_fault_detected": False,
        "hang_detected": False,
        "pterra_unlock_requested": True,
        "pterra_unlock_completed": True,
        "pterra_nav_chart_started": True,
        "pterra_nav_chart_active": True,
        "pterra_nav_chart_selected": True,
        "pterra_nav_panel_close_confirmed": True,
        "pterra_map_command_generated": True,
        "pterra_map_command_consumed": True,
        "pterra_map_destination_committed": True,
        "pterra_ship_navigation_activated": True,
        "pterra_map_setup": {
            "chart_object_offsets": [0x0D34, 0x0DA0],
            "pterra_marker": [201, 93],
            "generated_via": "deferred-record",
            "panel_close_confirmed": True,
        },
        "pterra_travel_command_generated": True,
        "pterra_travel_command_consumed": True,
        "pterra_target_row": 1,
        "pterra_travel_setup": {
            "entry": "native-current-location-entity",
            "entity_index": 31,
            "entity_rect": [120, 70, 80, 60],
            "entity_click_count": 1,
            "pterra_access_count_before": 0,
            "intro_hold_dismissed": True,
            "target_name_offsets": [0x0F60, 0x0DA4],
            "pterra_target_row": 1,
            "target_click_evidence": {
                "adapter": "guest-primary-edge",
                "point": [202, 115],
            },
        },
        "scruter_scene_requested": True,
        "scruter_scene_active_seen": True,
        "scruter_scene_completed": True,
        "scruter_sound_bank_loaded": True,
        "scruter_streamed_clip_count_before": 0,
        "scruter_streamed_clip_count": 19,
        "destination_committed": True,
        "pter_reached": True,
        "pter": {"cpu": {}},
        "pter_completed": True,
        "pter_choice_results": [0x0171, 0x02A8],
        "pter_sustained": True,
        "post_pter": {"cpu": {}, "duration_seconds": 5.0},
        "marker": {"path": "PTERRA1D.LBM"},
    }


class ScenarioSelectionTests(unittest.TestCase):
    def test_defaults_exclude_only_authentic_pterra(self) -> None:
        self.assertEqual(
            [scenario.name for scenario in matrix.selected_scenarios(None, False)],
            [
                "teleport-0",
                "teleport-1",
                "teleport-2",
                "teleport-3",
                "teleport-4",
                "script2-radio",
                "script1-bob-first-contact",
            ],
        )

    def test_focused_selection_is_deduplicated_and_canonical(self) -> None:
        self.assertEqual(
            [
                scenario.name
                for scenario in matrix.selected_scenarios(
                    ["script2-radio", "teleport-2", "teleport-2"], True
                )
            ],
            ["teleport-2", "script2-radio", "authentic-pterra"],
        )

    def test_full_contact_selection_adds_all_manifest_procedures(self) -> None:
        selected = matrix.selected_scenarios(None, False, True)
        self.assertEqual(sum(scenario.kind == "contact" for scenario in selected), 65)
        self.assertEqual(len(selected), 65)
        self.assertEqual(
            len({scenario.contact_selector for scenario in selected}), 65
        )
        self.assertTrue(
            all("@" in scenario.contact_selector for scenario in selected)
        )


class ReportValidationTests(unittest.TestCase):
    def test_teleport_requires_exact_completed_profile_state(self) -> None:
        scenario = matrix.SCENARIO_BY_NAME["teleport-3"]
        report = valid_teleport_report(3)
        self.assertEqual(matrix.validate_report(scenario, report), [])
        report["teleports"][0]["completed_state"]["request"] = 3
        self.assertIn(
            "completed request was not cleared",
            matrix.validate_report(scenario, report),
        )

    def test_teleport_allows_intro_ui_busy_but_no_other_blocker(self) -> None:
        scenario = matrix.SCENARIO_BY_NAME["teleport-4"]
        report = valid_teleport_report(4)
        state = report["teleports"][0]["completed_state"]
        state["blockers"]["vm_ui"] = 4
        self.assertEqual(matrix.validate_report(scenario, report), [])
        state["blockers"]["render"] = 1
        self.assertIn(
            "profile handoff blockers contain unexpected state",
            matrix.validate_report(scenario, report),
        )

    def test_pterra_rejects_a_report_that_did_not_enter_procedure(self) -> None:
        report = valid_pterra_report()
        report["pter_reached"] = False
        self.assertIn(
            "Pterra procedure was not reached",
            matrix.validate_report(
                matrix.SCENARIO_BY_NAME["authentic-pterra"], report
            ),
        )

    def test_pterra_requires_confirmed_native_title_transition(self) -> None:
        report = valid_pterra_report()
        report["title_transition_confirmed"] = False
        report["title_transition_evidence"] = []
        errors = matrix.validate_report(
            matrix.SCENARIO_BY_NAME["authentic-pterra"], report
        )
        self.assertIn("native title transition was not confirmed", errors)
        self.assertIn("native title transition evidence is invalid", errors)

    def test_pterra_requires_full_encounter_and_post_liveness(self) -> None:
        report = valid_pterra_report()
        report["pter_completed"] = False
        report["pter_choice_results"] = [0x1234]
        report["pter_sustained"] = False
        report["post_pter"] = None
        errors = matrix.validate_report(
            matrix.SCENARIO_BY_NAME["authentic-pterra"], report
        )
        self.assertIn(
            "Pterra procedure was not completed", errors
        )
        self.assertIn(
            "Pterra scripted choices are not exxos then teleport", errors
        )
        self.assertIn(
            "Pterra post-encounter liveness was not sustained", errors
        )

    def test_pterra_requires_native_streamed_sound_bank(self) -> None:
        report = valid_pterra_report()
        report["scruter_sound_bank_loaded"] = False
        report["scruter_streamed_clip_count"] = 0
        errors = matrix.validate_report(
            matrix.SCENARIO_BY_NAME["authentic-pterra"], report
        )
        self.assertIn(
            "native Scruter_Jo streamed sound bank was not loaded", errors
        )
        self.assertIn(
            "Scruter_Jo streamed clip count is not positive", errors
        )

    def test_pterra_requires_completed_native_scruter_transition(self) -> None:
        report = valid_pterra_report()
        report["scruter_scene_completed"] = False
        errors = matrix.validate_report(
            matrix.SCENARIO_BY_NAME["authentic-pterra"], report
        )
        self.assertIn(
            "native Scruter_Jo Pterra lifecycle did not complete",
            errors,
        )

    def test_pterra_requires_both_native_c1_commands(self) -> None:
        report = valid_pterra_report()
        report["pterra_map_command_consumed"] = False
        report["pterra_travel_command_generated"] = False
        errors = matrix.validate_report(
            matrix.SCENARIO_BY_NAME["authentic-pterra"], report
        )
        self.assertIn(
            "native VM did not consume the map Pterra C1 command", errors
        )
        self.assertIn(
            "native ship HUD did not generate the Orxx Pterra C1 command",
            errors,
        )

    def test_pterra_requires_native_ship_navigation_input(self) -> None:
        report = valid_pterra_report()
        report["pterra_ship_navigation_activated"] = False
        report["pterra_travel_setup"].pop("entity_rect")
        errors = matrix.validate_report(
            matrix.SCENARIO_BY_NAME["authentic-pterra"], report
        )
        self.assertIn(
            "native current-location interaction did not activate ship navigation",
            errors,
        )
        self.assertIn(
            "native ship navigation lacks current-location input evidence",
            errors,
        )

    def test_pterra_requires_exact_script_choices(self) -> None:
        report = valid_pterra_report()
        report["pter_choice_results"] = [0x02A8, 0x0171]
        errors = matrix.validate_report(
            matrix.SCENARIO_BY_NAME["authentic-pterra"], report
        )
        self.assertIn(
            "Pterra scripted choices are not exxos then teleport", errors
        )

    def test_radio_requires_ordered_semantic_checkpoints(self) -> None:
        scenario = matrix.SCENARIO_BY_NAME["script2-radio"]
        report = valid_radio_report()
        self.assertEqual(matrix.validate_report(scenario, report), [])
        checkpoints = report["radio_probe"]["checkpoints"]
        checkpoints[1], checkpoints[2] = checkpoints[2], checkpoints[1]
        self.assertTrue(matrix.validate_report(scenario, report))

    def test_bob_requires_all_first_contact_checkpoints(self) -> None:
        scenario = matrix.SCENARIO_BY_NAME["script1-bob-first-contact"]
        report = valid_bob_report()
        self.assertEqual(matrix.validate_report(scenario, report), [])
        report["bob_probe"]["checkpoints"].pop()
        self.assertIn(
            "Bob checkpoint count is 3, expected 4",
            matrix.validate_report(scenario, report),
        )

    def test_contact_requires_manifest_identity_and_a_real_checkpoint(self) -> None:
        scenario = matrix.CONTACT_SCENARIOS[0]
        assert scenario.contact_selector is not None
        report = valid_contact_report(scenario.contact_selector)
        self.assertEqual(matrix.validate_report(scenario, report), [])
        report["contact_probe"]["selector"] = "SCRIPT5:not-this-one"
        self.assertIn(
            "contact selector does not match the scenario",
            matrix.validate_report(scenario, report),
        )


class MatrixExecutionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.cd_dir = self.root / "cd"
        self.install_parent = self.root / "source-install"
        self.output_dir = self.root / "matrix"
        self.cd_dir.mkdir()
        (self.cd_dir / "BPRG_RE.EXE").write_bytes(b"MZ")
        (self.install_parent / "cblood").mkdir(parents=True)
        (self.install_parent / "cblood" / "GAME1.SAV").write_bytes(b"save")
        (self.install_parent / "cblood" / "CONFIG.DAT").write_bytes(b"config")

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def args(self, *extra: str):
        parser = matrix.build_parser()
        args = parser.parse_args(
            [
                "--cd-dir",
                str(self.cd_dir),
                "--install-parent",
                str(self.install_parent),
                "--output-dir",
                str(self.output_dir),
                "--display-base",
                "120",
                *extra,
            ]
        )
        matrix._validate_arguments(parser, args)
        return args

    @staticmethod
    def successful_subprocess(command: list[str], **_: object):
        if "--report" in command:
            report_path = Path(command[command.index("--report") + 1])
            if "--teleport-profile" in command:
                profile = int(command[command.index("--teleport-profile") + 1])
                report = valid_teleport_report(profile)
            elif "--script1-bob-probe" in command:
                report = valid_bob_report()
            elif "--contact-probe" in command:
                selector = command[command.index("--contact-probe") + 1]
                report = valid_contact_report(selector)
            else:
                report = valid_radio_report()
        else:
            report_path = Path(command[command.index("--output") + 1])
            report = valid_pterra_report()
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(json.dumps(report), encoding="utf-8")
        return subprocess.CompletedProcess(command, 0, "tool stdout", "")

    @mock.patch.object(matrix.subprocess, "run")
    def test_contact_timeout_covers_setup_and_dialogue_windows(
        self, run: mock.Mock
    ) -> None:
        run.side_effect = self.successful_subprocess
        scenario = matrix.CONTACT_SCENARIOS[0]
        args = self.args(
            "--scenario",
            scenario.name,
            "--contact-seconds",
            "40",
            "--subprocess-grace-seconds",
            "5",
        )

        exit_code, aggregate = matrix.run_matrix(args)

        self.assertEqual(exit_code, 0)
        self.assertEqual(aggregate["results"][0]["timeout_seconds"], 85)
        self.assertEqual(run.call_args.kwargs["timeout"], 85)

    @mock.patch.object(matrix.subprocess, "run")
    def test_runs_focused_scenarios_on_isolated_installs_and_displays(
        self, run: mock.Mock
    ) -> None:
        run.side_effect = self.successful_subprocess
        args = self.args(
            "--scenario",
            "teleport-2",
            "--scenario",
            "script2-radio",
        )

        exit_code, aggregate = matrix.run_matrix(args)

        self.assertEqual(exit_code, 0)
        self.assertEqual(aggregate["status"], "PASS")
        self.assertEqual(
            [result["display"] for result in aggregate["results"]],
            [":122", ":125"],
        )
        installs = [
            Path(result["install_parent"]) for result in aggregate["results"]
        ]
        self.assertNotEqual(installs[0], installs[1])
        for install in installs:
            self.assertEqual(
                (install / "cblood" / "CONFIG.DAT").read_bytes(), b"config"
            )
        self.assertEqual(run.call_count, 2)
        written = json.loads(
            (self.output_dir / "matrix.json").read_text(encoding="utf-8")
        )
        self.assertEqual(written["status"], "PASS")

    @mock.patch.object(matrix.subprocess, "run")
    def test_nonzero_subprocess_fails_even_with_a_valid_report(
        self, run: mock.Mock
    ) -> None:
        def failed(command: list[str], **kwargs: object):
            result = self.successful_subprocess(command, **kwargs)
            return subprocess.CompletedProcess(command, 7, result.stdout, "failure")

        run.side_effect = failed
        exit_code, aggregate = matrix.run_matrix(
            self.args("--scenario", "teleport-0")
        )

        self.assertEqual(exit_code, 1)
        result = aggregate["results"][0]
        self.assertEqual(result["status"], "FAIL")
        self.assertIn("subprocess exited with status 7", result["validation_errors"])

    @mock.patch.object(matrix.subprocess, "run")
    def test_missing_report_fails_closed(self, run: mock.Mock) -> None:
        run.return_value = subprocess.CompletedProcess([], 0, "", "")

        exit_code, aggregate = matrix.run_matrix(
            self.args("--scenario", "script2-radio")
        )

        self.assertEqual(exit_code, 1)
        result = aggregate["results"][0]
        self.assertEqual(result["status"], "FAIL")
        self.assertTrue(
            any(
                "did not create report" in error
                for error in result["validation_errors"]
            )
        )
        per_scenario = json.loads(
            (self.output_dir / "results" / "script2-radio.json").read_text(
                encoding="utf-8"
            )
        )
        self.assertEqual(per_scenario["status"], "FAIL")

    @mock.patch.object(matrix.subprocess, "run")
    def test_malformed_report_fails_closed(self, run: mock.Mock) -> None:
        def malformed(command: list[str], **_: object):
            report_path = Path(command[command.index("--report") + 1])
            report_path.parent.mkdir(parents=True, exist_ok=True)
            report_path.write_text("{not-json", encoding="utf-8")
            return subprocess.CompletedProcess(command, 0, "", "")

        run.side_effect = malformed
        exit_code, aggregate = matrix.run_matrix(
            self.args("--scenario", "teleport-1")
        )

        self.assertEqual(exit_code, 1)
        self.assertTrue(
            any(
                "cannot read tool report" in error
                for error in aggregate["results"][0]["validation_errors"]
            )
        )

    @mock.patch.object(matrix.subprocess, "run")
    def test_subprocess_timeout_fails_closed(self, run: mock.Mock) -> None:
        run.side_effect = subprocess.TimeoutExpired("watchdog", 1.0)

        exit_code, aggregate = matrix.run_matrix(
            self.args("--scenario", "teleport-4")
        )

        self.assertEqual(exit_code, 1)
        result = aggregate["results"][0]
        self.assertTrue(result["process"]["timed_out"])
        self.assertTrue(
            any(
                "subprocess exceeded" in error
                for error in result["validation_errors"]
            )
        )

    @mock.patch.object(matrix.subprocess, "run")
    def test_authentic_pterra_uses_existing_capture_driver_flags(
        self, run: mock.Mock
    ) -> None:
        run.side_effect = self.successful_subprocess
        source_markers = [
            self.install_parent / "cblood" / f"PTERRA1{suffix}.LBM"
            for suffix in "DFG"
        ]
        for marker in source_markers:
            marker.write_bytes(b"old capture")

        exit_code, aggregate = matrix.run_matrix(
            self.args(
                "--scenario", "authentic-pterra",
                "--dosbox", "dosbox-staging-test",
            )
        )

        self.assertEqual(exit_code, 0)
        command = aggregate["results"][0]["command"]
        self.assertIn("--manual-pterra", command)
        self.assertIn("--open-load-menu", command)
        self.assertIn("--trigger-pterra-after-load", command)
        self.assertIn("--drive-authentic-save", command)
        self.assertEqual(
            command[command.index("--dosbox") + 1],
            "dosbox-staging-test",
        )
        self.assertEqual(aggregate["results"][0]["display"], ":127")
        self.assertEqual(
            aggregate["results"][0]["removed_stale_artifacts"],
            ["PTERRA1D.LBM", "PTERRA1F.LBM", "PTERRA1G.LBM"],
        )
        copied_cblood = Path(
            aggregate["results"][0]["install_parent"]
        ) / "cblood"
        self.assertFalse(any(copied_cblood.glob("PTERRA1[DFG].LBM")))
        self.assertTrue(all(marker.is_file() for marker in source_markers))

    @mock.patch.object(matrix.subprocess, "run")
    def test_bob_probe_uses_its_named_watchdog_mode(
        self, run: mock.Mock
    ) -> None:
        run.side_effect = self.successful_subprocess

        exit_code, aggregate = matrix.run_matrix(
            self.args(
                "--scenario", "script1-bob-first-contact",
                "--dosbox", "dosbox-staging-test",
            )
        )

        self.assertEqual(exit_code, 0)
        result = aggregate["results"][0]
        self.assertEqual(result["status"], "PASS")
        self.assertEqual(result["display"], ":126")
        self.assertIn("--script1-bob-probe", result["command"])
        self.assertEqual(
            result["command"][result["command"].index("--dosbox") + 1],
            "dosbox-staging-test",
        )

    @mock.patch.object(matrix.subprocess, "run")
    def test_generated_contact_uses_manifest_watchdog_mode(
        self, run: mock.Mock
    ) -> None:
        run.side_effect = self.successful_subprocess
        scenario = matrix.CONTACT_SCENARIOS[0]

        exit_code, aggregate = matrix.run_matrix(
            self.args("--scenario", scenario.name)
        )

        self.assertEqual(exit_code, 0)
        result = aggregate["results"][0]
        self.assertIn("--contact-probe", result["command"])
        self.assertIn("--contact-manifest", result["command"])
        self.assertIn(scenario.contact_selector, result["command"])

    @mock.patch.object(matrix.subprocess, "run")
    def test_parallel_contacts_keep_canonical_result_order(
        self, run: mock.Mock
    ) -> None:
        run.side_effect = self.successful_subprocess
        first, second = matrix.CONTACT_SCENARIOS[:2]
        args = self.args(
            "--scenario",
            second.name,
            "--scenario",
            first.name,
            "--jobs",
            "2",
        )

        exit_code, aggregate = matrix.run_matrix(args)

        self.assertEqual(exit_code, 0)
        self.assertEqual(
            aggregate["selected_scenarios"], [first.name, second.name]
        )


if __name__ == "__main__":
    unittest.main()
