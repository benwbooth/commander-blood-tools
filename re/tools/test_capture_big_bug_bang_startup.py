#!/usr/bin/env python3
"""Synthetic memory tests; no original game bytes or running emulator needed."""

import argparse
import importlib.util
import io
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location(
    "capture_big_bug_bang_startup", Path(__file__).with_name("capture_big_bug_bang_startup.py"))
capture = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(capture)
sys.path.insert(0, str(capture.ROOT / "tools"))
import native_flow_coverage as flow_audit
sys.path.pop(0)


def fixture(profile=0):
    executable = bytearray(98190)
    executable[capture.GLOBAL_FILE:capture.GLOBAL_FILE + 44] = bytes(range(1, 45))
    executable[capture.VM_FILE:capture.VM_FILE + 16] = bytes(range(91, 107))
    for identity in range(capture.RESOURCE_COUNT):
        start = capture.CATALOG_NAMES_FILE + identity * capture.NAME_SIZE
        name = f"R{identity:03}.BIN".encode("ascii")
        executable[start:start + len(name)] = name
    for index in range(17):
        struct.pack_into("<5H", executable, capture.PROFILE_TABLE_FILE + index * 10,
                         *(2 + index * 5 + offset for offset in range(5)))
    guest = bytearray(capture.GUEST_BYTES)
    module = 65536
    guest[module:module + len(executable) - capture.MZ_HEADER_SIZE] = executable[capture.MZ_HEADER_SIZE:]
    globals_base = module + capture.GLOBAL_FILE - capture.MZ_HEADER_SIZE
    catalog = module + capture.CATALOG_SEGMENT_FILE - capture.MZ_HEADER_SIZE
    struct.pack_into("<H", guest, globals_base + capture.PROFILE_INDEX, profile)
    handles = [2, *(3 + profile * 5 + offset for offset in range(4))]
    struct.pack_into("<5H", guest, globals_base + capture.PROFILE_HANDLES, *handles)
    linear = 327680
    for index, size in enumerate((8368, 9008, 2048, 0, 1024)):
        if size:
            struct.pack_into("<HHI", guest, catalog + handles[index] * 8, linear // 16, 3, size)
        pointer = linear if size else previous
        struct.pack_into("<HH", guest, globals_base + capture.PROFILE_BINDINGS + index * 4, 0, pointer // 16)
        previous = pointer
        linear += size
    struct.pack_into("<H", guest, 327680 + capture.TIME_OFFSET, 24930)
    return executable, guest, globals_base, catalog


class StartupCaptureTests(unittest.TestCase):
    def native_checkpoint_fixture(self, root):
        disc, writable = root / "disc", root / "writable"
        disc.mkdir()
        writable.mkdir()
        (disc / "BLOOD.DAT").write_bytes(b"archive")
        (disc / "SCRIPT1.VAR").write_bytes(b"var")
        (writable / "BLOOD.SAV").write_bytes(b"directory")
        (writable / "GAME1.SAV").write_bytes(b"earned save")
        assets = dict(game="big_bug_bang", schema_version=1, source_archive_byte_count=7,
                      source_archive_sha256=flow_audit.digest(disc / "BLOOD.DAT"),
                      resources=[dict(resource_name="SCRIPT1.VAR", origin="loose_file",
                                      sha256=flow_audit.digest(disc / "SCRIPT1.VAR"))], companions=[])
        asset_manifest = root / "assets.json"
        asset_manifest.write_text(json.dumps(assets))
        manifest = root / "flow.json"
        checkpoint = dict(slot=0, filename="GAME1.SAV", saved_at_frame=123,
                          directory_sha256=flow_audit.digest(writable / "BLOOD.SAV"),
                          sha256=flow_audit.digest(writable / "GAME1.SAV"))
        row = dict(path=manifest, sha256="audited-manifest-hash", manifest=dict(
            game="bbb", status="observed_route", checkpoints=[checkpoint],
            provenance=dict(asset_manifest_sha256=flow_audit.digest(asset_manifest))))
        return manifest, asset_manifest, disc, row

    def test_native_checkpoint_stages_only_audited_bytes_without_claiming_original_load(self):
        with tempfile.TemporaryDirectory() as directory:
            manifest, assets, disc, row = self.native_checkpoint_fixture(Path(directory))
            with mock.patch.object(flow_audit, "read_witness", return_value=row) as read, \
                    mock.patch.object(flow_audit, "check_lineages", wraps=flow_audit.check_lineages) as check:
                files, reference = capture.native_checkpoint_files(manifest, 0, assets, disc)
            self.assertEqual(files, {"BLOOD.SAV": b"directory", "GAME1.SAV": b"earned save"})
            self.assertEqual(len(read.call_args.args[1]), 17)
            check.assert_called_once_with({manifest: row})
            self.assertEqual(reference["checkpoint"]["saved_at_frame"], 123)
            self.assertEqual(len(reference["validation_tools"]), 3)
            self.assertFalse(reference["original_load_verified"])
            self.assertFalse(reference["original_playthrough_witness"])

    def test_native_checkpoint_rejects_disc_mismatch_before_auditing(self):
        for name in ("BLOOD.DAT", "SCRIPT1.VAR"):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                manifest, assets, disc, row = self.native_checkpoint_fixture(Path(directory))
                (disc / name).write_bytes(b"changed")
                with mock.patch.object(flow_audit, "read_witness", return_value=row) as read:
                    with self.assertRaisesRegex(ValueError, "disc"):
                        capture.native_checkpoint_files(manifest, 0, assets, disc)
                    read.assert_not_called()

    def test_native_checkpoint_rejects_failed_audit(self):
        with tempfile.TemporaryDirectory() as directory:
            manifest, assets, disc, row = self.native_checkpoint_fixture(Path(directory))
            with mock.patch.object(flow_audit, "read_witness", side_effect=ValueError("bad evidence")):
                with self.assertRaisesRegex(ValueError, "bad evidence"):
                    capture.native_checkpoint_files(manifest, 0, assets, disc)

    def test_native_checkpoint_rejects_wrong_game_ending_assets_and_unwitnessed_slot(self):
        for mode in ("game", "ending", "assets", "slot"):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                manifest, assets, disc, row = self.native_checkpoint_fixture(Path(directory))
                state = row["manifest"]
                if mode == "game":
                    state["game"] = "cb"
                elif mode == "ending":
                    state["status"] = "observed_ending"
                elif mode == "assets":
                    state["provenance"]["asset_manifest_sha256"] = "different"
                else:
                    state["checkpoints"] = []
                with mock.patch.object(flow_audit, "read_witness", return_value=row):
                    with self.assertRaises(ValueError):
                        capture.native_checkpoint_files(manifest, 0, assets, disc)

    def test_native_checkpoint_rejects_cycle(self):
        with tempfile.TemporaryDirectory() as directory:
            manifest, assets, disc, row = self.native_checkpoint_fixture(Path(directory))
            row["manifest"]["predecessor"] = dict(manifest=str(manifest))
            with mock.patch.object(flow_audit, "read_witness", return_value=row):
                with self.assertRaisesRegex(ValueError, "cyclic"):
                    capture.native_checkpoint_files(manifest, 0, assets, disc)

    def test_native_checkpoint_rechecks_bytes_after_audit(self):
        with tempfile.TemporaryDirectory() as directory:
            manifest, assets, disc, row = self.native_checkpoint_fixture(Path(directory))
            (manifest.parent / "writable" / "GAME1.SAV").write_bytes(b"changed")
            with mock.patch.object(flow_audit, "read_witness", return_value=row):
                with self.assertRaisesRegex(ValueError, "changed after audit"):
                    capture.native_checkpoint_files(manifest, 0, assets, disc)

    def test_cli_requires_paired_native_checkpoint_flags(self):
        for flags in (["--native-checkpoint", "flow.json"],
                      ["--native-asset-manifest", "assets.json"], ["--native-slot", "2"]):
            with self.subTest(flags=flags), mock.patch.object(sys, "argv", ["capture", "disc", "out", *flags]), \
                    mock.patch.object(sys, "stderr", new_callable=io.StringIO), \
                    mock.patch.object(capture, "capture") as run:
                with self.assertRaises(SystemExit) as stopped:
                    capture.main()
                self.assertEqual(stopped.exception.code, 2)
                run.assert_not_called()

    def test_private_key_resolution_checks_base_level_on_explicit_display(self):
        xlib = mock.Mock()
        xlib.XOpenDisplay.return_value = 123
        xlib.XStringToKeysym.side_effect = [65507, 65479]
        xlib.XKeysymToKeycode.side_effect = [37, 76]
        xlib.XkbKeycodeToKeysym.side_effect = [65507, 65479]
        mappings = "0-1 r-xp 0 0 1 /lib/libX11.so.6\n1-2 r--p 0 0 1 /lib/libX11.so.6\n"
        with mock.patch.object(capture.Path, "read_text", return_value=mappings), \
                mock.patch.object(capture.ctypes, "CDLL", return_value=xlib):
            resolved = capture.private_key_sequence({"DISPLAY": ":123"}, 789, ["Control_L", "F10"])
        self.assertEqual(resolved["sequence"], "37+76")
        self.assertEqual(resolved["keycodes"], [37, 76])
        xlib.XOpenDisplay.assert_called_once_with(b":123")
        xlib.XCloseDisplay.assert_called_once_with(123)

    def test_private_key_resolution_rejects_wrong_symbol_and_closes_connection(self):
        xlib = mock.Mock()
        xlib.XOpenDisplay.return_value = 123
        xlib.XStringToKeysym.return_value = 65479
        xlib.XKeysymToKeycode.return_value = 64
        xlib.XkbKeycodeToKeysym.return_value = 65513
        with mock.patch.object(capture.Path, "read_text", return_value="0-1 r-xp 0 0 1 /lib/libX11.so.6\n"), \
                mock.patch.object(capture.ctypes, "CDLL", return_value=xlib):
            with self.assertRaisesRegex(RuntimeError, "base-level"):
                capture.private_key_sequence({"DISPLAY": ":123"}, 789, ["F10"])
        xlib.XCloseDisplay.assert_called_once_with(123)

    def test_private_key_resolution_rejects_missing_or_ambiguous_xlib(self):
        for mappings in ("", "0-1 r-xp 0 0 1 /a/libX11.so.6\n1-2 r-xp 0 0 1 /b/libX11.so.6\n"):
            with self.subTest(mappings=mappings), \
                    mock.patch.object(capture.Path, "read_text", return_value=mappings), \
                    mock.patch.object(capture.ctypes, "CDLL") as loader:
                with self.assertRaisesRegex(RuntimeError, "one mapped Xlib"):
                    capture.private_key_sequence({"DISPLAY": ":123"}, 789, ["F10"])
                loader.assert_not_called()

    def test_private_key_resolution_does_not_use_default_display_after_open_failure(self):
        xlib = mock.Mock()
        xlib.XOpenDisplay.return_value = None
        with mock.patch.object(capture.Path, "read_text", return_value="0-1 r-xp 0 0 1 /lib/libX11.so.6\n"), \
                mock.patch.object(capture.ctypes, "CDLL", return_value=xlib):
            with self.assertRaisesRegex(RuntimeError, "cannot open private"):
                capture.private_key_sequence({"DISPLAY": ":123"}, 789, ["F10"])
        xlib.XStringToKeysym.assert_not_called()
        xlib.XCloseDisplay.assert_not_called()

    def test_initial_time_word_belongs_to_directory_not_var(self):
        executable, guest, _globals, _catalog = fixture()
        state = capture.inspect_guest(guest, executable)
        self.assertEqual(state["status"], "profile_bound")
        time = state["time_storage"]
        self.assertEqual(time["value"], 24930)
        self.assertFalse(time["belongs_to_var"])
        self.assertEqual([(r["id"], r["offset"]) for r in time["owners"]], [(3, 0)])
        self.assertEqual(state["bindings"]["bas"]["owners"], [4])

    def test_noninitial_profile_retains_original_var(self):
        executable, guest, _globals, _catalog = fixture(profile=1)
        state = capture.inspect_guest(guest, executable)
        self.assertEqual(state["status"], "profile_bound")
        self.assertEqual(state["bindings"]["var"]["handle"], 2)
        self.assertEqual(state["time_storage"]["owners"][0]["id"], 8)

    def test_unmapped_neighbor_is_reported_without_fabricating_ownership(self):
        executable, guest, _globals, catalog = fixture()
        # The directory moved; the observed word is now unowned space.
        struct.pack_into("<H", guest, catalog + 3 * 8, 28672)
        struct.pack_into("<HH", guest, _globals + capture.PROFILE_BINDINGS + 4, 0, 28672)
        state = capture.inspect_guest(guest, executable)
        self.assertEqual(state["status"], "profile_bound")
        self.assertEqual(state["time_storage"]["owners"], [])

    def test_incomplete_profile_binding_does_not_claim_startup(self):
        executable, guest, globals_base, _catalog = fixture()
        struct.pack_into("<H", guest, globals_base + capture.PROFILE_HANDLES + 4, 150)
        state = capture.inspect_guest(guest, executable)
        self.assertEqual(state["status"], "module_found")
        self.assertFalse(state["bindings_consistent"])
        self.assertNotIn("time_storage", state)

    def test_requires_both_vm_and_fixed_catalog_anchors(self):
        for offset in (capture.VM_FILE, capture.CATALOG_NAMES_FILE + 100):
            executable, guest, _globals, _catalog = fixture()
            guest[65536 + offset - capture.MZ_HEADER_SIZE] ^= 255
            self.assertEqual(capture.inspect_guest(guest, executable)["status"], "module_not_found")

    def test_descript_sprite_slot_may_change_and_reports_live_resource_name(self):
        executable, guest, _globals, catalog = fixture()
        name = 65536 + capture.CATALOG_NAMES_FILE - capture.MZ_HEADER_SIZE + 7 * capture.NAME_SIZE
        guest[name:name + capture.NAME_SIZE] = b"gluant1.spr\0".ljust(capture.NAME_SIZE, b"\0")
        struct.pack_into("<HHI", guest, catalog + 7 * capture.HANDLE_SIZE, 40000, 3, 64)
        state = capture.inspect_guest(guest, executable)
        self.assertEqual(state["status"], "profile_bound")
        sprite = next(r for r in state["resident_resources"] if r["id"] == 7)
        self.assertEqual(sprite["name"], "gluant1.spr")

    def test_dynamic_catalog_slot_still_requires_a_bounded_printable_name(self):
        for field in (b"a" * 16, b"\xff\0".ljust(16, b"\0"), b"\0" * 16,
                      b"a\nb\0".ljust(16, b"\0")):
            with self.subTest(field=field):
                executable, guest, _globals, _catalog = fixture()
                name = 65536 + capture.CATALOG_NAMES_FILE - capture.MZ_HEADER_SIZE + 7 * capture.NAME_SIZE
                guest[name:name + capture.NAME_SIZE] = field
                self.assertEqual(capture.inspect_guest(guest, executable)["status"], "module_not_found")

    def test_loader_may_uppercase_catalog_in_place(self):
        executable, guest, _globals, _catalog = fixture()
        begin = capture.CATALOG_NAMES_FILE
        end = begin + capture.RESOURCE_COUNT * capture.NAME_SIZE
        executable[begin:end] = executable[begin:end].lower()
        self.assertEqual(capture.inspect_guest(guest, executable)["status"], "profile_bound")

    def test_rejects_truncated_ram(self):
        executable, guest, _globals, _catalog = fixture()
        with self.assertRaises(ValueError):
            capture.inspect_guest(guest[:-1], executable)

    def test_multiple_modules_are_not_arbitrarily_chosen(self):
        executable, guest, _globals, _catalog = fixture()
        module = executable[capture.MZ_HEADER_SIZE:]
        guest[262144:262144 + len(module)] = module
        state = capture.inspect_guest(guest, executable)
        self.assertEqual(state["status"], "ambiguous_modules")
        self.assertEqual(len(state["candidates"]), 2)

    def test_var_mutation_changes_snapshot_identity(self):
        executable, guest, _globals, _catalog = fixture()
        first = capture.inspect_guest(guest, executable)
        guest[327682] ^= 1
        second = capture.inspect_guest(guest, executable)
        self.assertNotEqual(first["var_sha256"], second["var_sha256"])

    def test_mouse_poll_observation_preserves_shared_slot_values(self):
        executable, guest, globals_base, _catalog = fixture()
        struct.pack_into("<3H", guest, globals_base + 0xC22, 720, 150, 3)
        self.assertEqual(capture.inspect_guest(guest, executable)["mouse_poll"],
                         {"x": 720, "y": 150, "buttons": 3})

    def test_bridge_observation_reads_original_steering_fields(self):
        executable, guest, globals_base, _catalog = fixture()
        guest[globals_base + 0x2A33] = 4
        for offset, value in ((0x2A35, 45), (0x2A37, 90), (0x2A3B, 180), (0x2A47, 360)):
            struct.pack_into("<H", guest, globals_base + offset, value)
        self.assertEqual(capture.inspect_guest(guest, executable)["bridge"],
                         {"ui_flags": 4, "frame": 45, "mouse_arc": 90,
                          "seek_target": 180, "frame_angle_bias": 360})

    def test_private_move_has_no_click_and_observes_only_after_motion(self):
        env = {"DISPLAY": ":123"}
        with mock.patch.object(capture.subprocess, "check_output", return_value="456\n"), \
                mock.patch.object(capture, "private_mouse_locked", return_value=True), \
                mock.patch.object(capture.subprocess, "run") as run, mock.patch.object(capture.time, "sleep"):
            event = capture.private_move(env, 789, [-480, 0], lambda: {"bridge": {"frame": 45}})
        self.assertEqual([call.args[0] for call in run.call_args_list], [
            ["xdotool", "windowfocus", "--sync", "456"],
            ["xdotool", "mousemove_relative", "--", "-480", "0"]])
        self.assertTrue(all(call.kwargs["env"] == env for call in run.call_args_list))
        self.assertEqual(event["after_move"], {"bridge": {"frame": 45}})
        self.assertFalse(event["guest_memory_written"])

    def test_private_move_requires_valid_deltas_and_capture(self):
        with mock.patch.object(capture.subprocess, "check_output", return_value="456\n") as search, \
                mock.patch.object(capture.subprocess, "run") as run, \
                mock.patch.object(capture, "private_mouse_locked", return_value=False):
            for motion in ([], [1], [1, 2, 3], [32768, 0], [0, -32769], [1.0, 0], [True, 0]):
                with self.assertRaises(ValueError):
                    capture.private_move({"DISPLAY": ":123"}, 789, motion)
            search.assert_not_called()
            with self.assertRaises(RuntimeError):
                capture.private_move({"DISPLAY": ":123"}, 789, [-480, 0])
            self.assertEqual(len(run.call_args_list), 1)

    def test_private_move_packets_preserve_signed_total_and_private_display(self):
        env = {"DISPLAY": ":123"}
        with mock.patch.object(capture, "private_window", return_value="456"), \
                mock.patch.object(capture, "private_mouse_locked", return_value=True), \
                mock.patch.object(capture.subprocess, "run") as run, \
                mock.patch.object(capture.time, "sleep") as sleep:
            event = capture.private_move(env, 789, [-7, 5], steps=3)
        self.assertEqual(event["relative_motion_packets"], [[-3, 1], [-2, 2], [-2, 2]])
        self.assertEqual([call.args[0][-2:] for call in run.call_args_list],
                         [["-3", "1"], ["-2", "2"], ["-2", "2"]])
        self.assertTrue(all(call.kwargs["env"] == env for call in run.call_args_list))
        self.assertEqual(sleep.call_args_list, [mock.call(0.02)] * 3 + [mock.call(0.15)])

    def test_private_move_rejects_invalid_steps_before_input(self):
        with mock.patch.object(capture, "private_window") as window:
            for steps in (0, 129, 1.5, True):
                with self.assertRaisesRegex(ValueError, "motion steps"):
                    capture.private_move({"DISPLAY": ":123"}, 789, [-7, 5], steps=steps)
            window.assert_not_called()

    def test_private_steering_uses_observed_frame_and_centers_before_success(self):
        states = [{"status": "profile_bound", "profile": 0, "bridge": {"frame": frame, "mouse_arc": arc}}
                  for frame, arc in ((90, 180), (50, 70), (45, 60), (45, 90))]
        observe = mock.Mock(side_effect=states)
        with mock.patch.object(capture, "private_mouse_locked", return_value=True), \
                mock.patch.object(capture, "private_move", return_value={"guest_memory_written": False}) as move:
            result = capture.private_steer({"DISPLAY": ":123"}, 789, 45, observe)
        self.assertTrue(result["target_verified"])
        self.assertFalse(result["guest_memory_written"])
        self.assertEqual(len(result["iterations"]), 4)
        self.assertEqual([call.args[2] for call in move.call_args_list], [[-240, 0], [-40, 0], [120, 0]])
        self.assertEqual([call.kwargs["steps"] for call in move.call_args_list], [30, 5, 15])

    def test_private_steering_is_bounded_without_claiming_a_stalled_target(self):
        observe = mock.Mock(return_value={"status": "profile_bound", "profile": 0,
                                         "bridge": {"frame": 90, "mouse_arc": 180}})
        with mock.patch.object(capture, "private_mouse_locked", return_value=True), \
                mock.patch.object(capture, "private_move", return_value={}) as move:
            result = capture.private_steer({"DISPLAY": ":123"}, 789, 45, observe, limit=1)
        self.assertFalse(result["target_verified"])
        self.assertEqual(observe.call_count, 2)
        move.assert_called_once()

    def test_private_steering_rejects_invalid_requests_before_input(self):
        with mock.patch.object(capture, "private_mouse_locked") as locked:
            for target, limit in ((-1, 48), (180, 48), (True, 48), (45, 0), (45, 49)):
                with self.assertRaises(ValueError):
                    capture.private_steer({"DISPLAY": ":123"}, 789, target, mock.Mock(), limit)
            locked.assert_not_called()
        with mock.patch.object(capture, "private_mouse_locked", return_value=False), \
                mock.patch.object(capture, "private_move") as move:
            with self.assertRaisesRegex(RuntimeError, "confirmed mouse capture"):
                capture.private_steer({"DISPLAY": ":123"}, 789, 45, mock.Mock())
            move.assert_not_called()

    def test_private_steering_requires_valid_feedback_and_stable_profile(self):
        for state in ({}, {"status": "profile_bound", "bridge": {"frame": 180, "mouse_arc": 0}}):
            with mock.patch.object(capture, "private_mouse_locked", return_value=True), \
                    mock.patch.object(capture, "private_move") as move:
                with self.assertRaisesRegex(RuntimeError, "bound original bridge"):
                    capture.private_steer({"DISPLAY": ":123"}, 789, 45, lambda: state)
                move.assert_not_called()
        observe = mock.Mock(side_effect=[{"status": "profile_bound", "profile": profile,
                                          "bridge": {"frame": 90, "mouse_arc": 180}} for profile in (0, 1)])
        with mock.patch.object(capture, "private_mouse_locked", return_value=True), \
                mock.patch.object(capture, "private_move", return_value={}) as move:
            with self.assertRaisesRegex(RuntimeError, "profile change"):
                capture.private_steer({"DISPLAY": ":123"}, 789, 45, observe)
            move.assert_called_once()

    def test_private_bridge_pointer_moves_without_clicking_or_changing_view(self):
        observe = mock.Mock(side_effect=[
            {"status": "profile_bound", "profile": 0,
             "bridge": {"frame": 45, "mouse_arc": arc, "frame_angle_bias": 200},
             "mouse_poll": {"x": 2000, "y": y}} for arc, y in ((89, 150), (83, 128), (81, 119))])
        with mock.patch.object(capture, "private_mouse_locked", return_value=True), \
                mock.patch.object(capture, "private_move", return_value={}) as move:
            result = capture.private_bridge_point({"DISPLAY": ":123"}, 789, [125, 118], observe)
        self.assertTrue(result["target_verified"])
        self.assertFalse(result["guest_memory_written"])
        self.assertEqual(result["iterations"][-1]["logical_pointer"], [124, 119])
        self.assertEqual([call.args[2] for call in move.call_args_list], [[-31, -32], [-7, -10]])

    def test_private_bridge_pointer_converges_with_menu_motion_scaling(self):
        position = [104.0, 110.0]

        def observe():
            return {"status": "profile_bound", "profile": 0,
                    "bridge": {"frame": 45, "mouse_arc": int(position[0]) // 4 + 50,
                               "frame_angle_bias": 200},
                    "mouse_poll": {"x": int(position[0]), "y": int(position[1])}}

        def move(_env, _pid, motion, steps):
            for i, value in enumerate(motion):
                position[i] += value * 2 / 3
            return {}

        with mock.patch.object(capture, "private_mouse_locked", return_value=True), \
                mock.patch.object(capture, "private_move", side_effect=move):
            result = capture.private_bridge_point({"DISPLAY": ":123"}, 789, [100, 115], observe)
        self.assertTrue(result["target_verified"])
        self.assertEqual(len(result["iterations"]), 2)

    def test_private_bridge_pointer_decodes_wrapped_negative_bias(self):
        observe = mock.Mock(return_value={"status": "profile_bound", "profile": 0,
                                          "bridge": {"frame": 0, "mouse_arc": 0, "frame_angle_bias": 65376},
                                          "mouse_poll": {"x": 2160, "y": 100}})
        with mock.patch.object(capture, "private_mouse_locked", return_value=True), \
                mock.patch.object(capture, "private_move") as move:
            result = capture.private_bridge_point({"DISPLAY": ":123"}, 789, [160, 100], observe)
        self.assertTrue(result["target_verified"])
        move.assert_not_called()

    def test_private_bridge_pointer_rejects_invalid_request_or_unbound_feedback(self):
        with mock.patch.object(capture, "private_mouse_locked") as locked:
            for target in ([], [1], [1, 2, 3], [-1, 0], [320, 0], [0, 200], [1.0, 0], [True, 0]):
                with self.assertRaises(ValueError):
                    capture.private_bridge_point({"DISPLAY": ":123"}, 789, target, mock.Mock())
            for limit in (0, 33, True):
                with self.assertRaises(ValueError):
                    capture.private_bridge_point({"DISPLAY": ":123"}, 789, [160, 100], mock.Mock(), limit)
            locked.assert_not_called()
        with mock.patch.object(capture, "private_mouse_locked", return_value=True), \
                mock.patch.object(capture, "private_move") as move:
            with self.assertRaisesRegex(RuntimeError, "bound original bridge"):
                capture.private_bridge_point({"DISPLAY": ":123"}, 789, [160, 100], lambda: {})
            move.assert_not_called()

    def test_private_bridge_pointer_is_bounded_and_rejects_camera_changes(self):
        state = {"status": "profile_bound", "profile": 0,
                 "bridge": {"frame": 45, "mouse_arc": 89, "frame_angle_bias": 200},
                 "mouse_poll": {"y": 150}}
        with mock.patch.object(capture, "private_mouse_locked", return_value=True), \
                mock.patch.object(capture, "private_move", return_value={}) as move:
            result = capture.private_bridge_point({"DISPLAY": ":123"}, 789, [125, 118], lambda: state, limit=1)
        self.assertFalse(result["target_verified"])
        move.assert_called_once()
        changed = dict(state, bridge=dict(state["bridge"], frame=46))
        with mock.patch.object(capture, "private_mouse_locked", return_value=True), \
                mock.patch.object(capture, "private_move", return_value={}) as move:
            with self.assertRaisesRegex(RuntimeError, "changed profile or camera"):
                capture.private_bridge_point({"DISPLAY": ":123"}, 789, [125, 118], mock.Mock(side_effect=[state, changed]))
            move.assert_called_once()

    def test_private_recapture_releases_moves_and_recaptures_without_clicking(self):
        env = {"DISPLAY": ":123"}
        with mock.patch.object(capture.subprocess, "check_output", return_value="456\n"), \
                mock.patch.object(capture, "private_key_sequence", return_value={"sequence": "37+76"}), \
                mock.patch.object(capture, "private_mouse_locked", side_effect=[True, False, True]), \
                mock.patch.object(capture.subprocess, "run") as run, mock.patch.object(capture.time, "sleep"):
            event = capture.private_recapture(env, 789, [400, 316], lambda: {"profile": 0})
        self.assertEqual([call.args[0] for call in run.call_args_list], [
            ["xdotool", "windowfocus", "--sync", "456"],
            ["xdotool", "keydown", "37+76"],
            ["xdotool", "keyup", "37+76"],
            ["xdotool", "mousemove", "400", "316"],
            ["xdotool", "keydown", "37+76"],
            ["xdotool", "keyup", "37+76"]])
        self.assertTrue(all(call.kwargs["env"] == env for call in run.call_args_list))
        self.assertEqual(event["after_recapture"], {"profile": 0})
        self.assertTrue(event["release_verified"] and event["mouse_capture_verified"])
        self.assertFalse(event["guest_memory_written"])

    def test_private_recapture_restores_capture_after_failed_motion(self):
        with mock.patch.object(capture.subprocess, "check_output", return_value="456\n"), \
                mock.patch.object(capture, "private_key_sequence", return_value={"sequence": "37+76"}), \
                mock.patch.object(capture, "private_mouse_locked", side_effect=[True, False, True]), \
                mock.patch.object(capture.subprocess, "run", side_effect=[None, None, None, ValueError("move"), None, None]) as run, \
                mock.patch.object(capture.time, "sleep"):
            with self.assertRaisesRegex(ValueError, "move"):
                capture.private_recapture({"DISPLAY": ":123"}, 789, [400, 316])
        self.assertEqual(run.call_args.args[0], ["xdotool", "keyup", "37+76"])

    def test_private_recapture_requires_each_lock_transition(self):
        for states, expected_calls in (([False], 1), ([True, True], 3), ([True, False, False], 6)):
            with self.subTest(states=states), \
                    mock.patch.object(capture.subprocess, "check_output", return_value="456\n"), \
                    mock.patch.object(capture, "private_key_sequence", return_value={"sequence": "37+76"}), \
                    mock.patch.object(capture, "private_mouse_locked", side_effect=states), \
                    mock.patch.object(capture.subprocess, "run") as run, mock.patch.object(capture.time, "sleep"):
                observe = mock.Mock()
                with self.assertRaises(RuntimeError):
                    capture.private_recapture({"DISPLAY": ":123"}, 789, [400, 316], observe)
                self.assertEqual(run.call_count, expected_calls)
                observe.assert_not_called()

    def test_private_recapture_rejects_invalid_position_before_input(self):
        with mock.patch.object(capture.subprocess, "run") as run:
            for position in ([], [1], [1, 2, 3], [-1, 0], [800, 0], [0, 600], [1.0, 0], [True, 0]):
                with self.assertRaises(ValueError):
                    capture.private_recapture({"DISPLAY": ":123"}, 789, position)
            run.assert_not_called()

    def test_private_click_uses_supplied_display_without_pointer_motion(self):
        env = {"DISPLAY": ":123", "SDL_VIDEODRIVER": "x11"}
        with mock.patch.object(capture.subprocess, "check_output", return_value="456\n") as search, \
                mock.patch.object(capture.subprocess, "run") as run, \
                mock.patch.object(capture.time, "sleep"):
            event = capture.private_click(env, 789)
        self.assertEqual(search.call_args.kwargs["env"], env)
        self.assertEqual([call.args[0] for call in run.call_args_list], [
            ["xdotool", "windowfocus", "--sync", "456"],
            ["xdotool", "mousedown", "1"],
            ["xdotool", "mouseup", "1"],
        ])
        self.assertTrue(all(call.kwargs["env"] == env for call in run.call_args_list))
        self.assertFalse(event["pointer_moved"])

    def test_click_refuses_ambiguous_target_before_sending_input(self):
        with mock.patch.object(capture.subprocess, "check_output", return_value="456\n457\n"), \
                mock.patch.object(capture.subprocess, "run") as run:
            with self.assertRaises(RuntimeError):
                capture.private_click({"DISPLAY": ":123"}, 789)
            run.assert_not_called()

    def test_private_key_observes_and_releases_on_only_the_private_display(self):
        env = {"DISPLAY": ":123"}
        calls = []
        def observe():
            calls.append("observe")
            return {"profile": 1}
        with mock.patch.object(capture.subprocess, "check_output", return_value="456\n") as search, \
                mock.patch.object(capture, "private_key_sequence", return_value={"sequence": "73"}), \
                mock.patch.object(capture.subprocess, "run", side_effect=lambda command, **kwargs: calls.append(command)) as run, \
                mock.patch.object(capture.time, "sleep"):
            event = capture.private_key(env, 789, "F7", observe)
        self.assertEqual(calls, [["xdotool", "windowfocus", "--sync", "456"],
                                 ["xdotool", "keydown", "73"], "observe", ["xdotool", "keyup", "73"]])
        self.assertEqual(search.call_args.kwargs["env"], env)
        self.assertTrue(all(call.kwargs["env"] == env for call in run.call_args_list))
        self.assertEqual(event["during_press"], {"profile": 1})
        self.assertEqual(event["key"], "F7")
        self.assertFalse(event["guest_memory_written"])

    def test_failed_key_observation_still_releases_key(self):
        with mock.patch.object(capture.subprocess, "check_output", return_value="456\n"), \
                mock.patch.object(capture, "private_key_sequence", return_value={"sequence": "73"}), \
                mock.patch.object(capture.subprocess, "run") as run, mock.patch.object(capture.time, "sleep"):
            with self.assertRaisesRegex(ValueError, "observation failed"):
                capture.private_key({"DISPLAY": ":123"}, 789, "F7",
                                    mock.Mock(side_effect=ValueError("observation failed")))
        self.assertEqual(run.call_args.args[0], ["xdotool", "keyup", "73"])

    def test_private_key_rejects_ambiguous_window_and_unlisted_keys(self):
        with mock.patch.object(capture.subprocess, "check_output", return_value="456\n457\n") as search, \
                mock.patch.object(capture.subprocess, "run") as run:
            for key in ("F12", "a", "--window", "ctrl+F7"):
                with self.assertRaises(ValueError):
                    capture.private_key({"DISPLAY": ":123"}, 789, key)
            search.assert_not_called()
            with self.assertRaises(RuntimeError):
                capture.private_key({"DISPLAY": ":123"}, 789, "F7")
            run.assert_not_called()

    def test_secondary_click_is_recorded_and_released_on_the_private_display(self):
        env = {"DISPLAY": ":123"}
        with mock.patch.object(capture.subprocess, "check_output", return_value="456\n"), \
                mock.patch.object(capture.subprocess, "run") as run, \
                mock.patch.object(capture.time, "sleep"):
            event = capture.private_click(env, 789, button=3)
        self.assertEqual([call.args[0] for call in run.call_args_list], [
            ["xdotool", "windowfocus", "--sync", "456"],
            ["xdotool", "mousedown", "3"], ["xdotool", "mouseup", "3"],
        ])
        self.assertTrue(all(call.kwargs["env"] == env for call in run.call_args_list))
        self.assertEqual(event["kind"], "private_x11_secondary_click")
        self.assertEqual(event["button"], 3)
        self.assertFalse(event["guest_memory_written"])

    def test_invalid_click_button_is_rejected_before_any_input(self):
        with mock.patch.object(capture.subprocess, "check_output") as search:
            for button in (0, 2, 4):
                with self.assertRaises(ValueError):
                    capture.private_click({"DISPLAY": ":123"}, 789, button=button)
            search.assert_not_called()

    def test_press_observation_runs_before_release(self):
        calls = []
        def observe():
            calls.append("observe")
            return {"mouse_poll": {"buttons": 1}}
        with mock.patch.object(capture.subprocess, "check_output", return_value="456\n"), \
                mock.patch.object(capture.subprocess, "run", side_effect=lambda command, **kwargs: calls.append(command[1])), \
                mock.patch.object(capture.time, "sleep"):
            event = capture.private_click({"DISPLAY": ":123"}, 789, observe_press=observe)
        self.assertEqual(calls, ["windowfocus", "mousedown", "observe", "mouseup"])
        self.assertEqual(event["during_press"], {"mouse_poll": {"buttons": 1}})

    def test_failed_press_observation_still_releases_button(self):
        with mock.patch.object(capture.subprocess, "check_output", return_value="456\n"), \
                mock.patch.object(capture.subprocess, "run") as run, \
                mock.patch.object(capture.time, "sleep"):
            with self.assertRaisesRegex(ValueError, "observation failed"):
                capture.private_click({"DISPLAY": ":123"}, 789, button=3,
                                      observe_press=mock.Mock(side_effect=ValueError("observation failed")))
        self.assertEqual(run.call_args.args[0], ["xdotool", "mouseup", "3"])

    def test_positioned_click_records_coordinates_and_moves_only_private_pointer(self):
        env = {"DISPLAY": ":123"}
        with mock.patch.object(capture.subprocess, "check_output", return_value="456\n"), \
                mock.patch.object(capture.subprocess, "run") as run, \
                mock.patch.object(capture.time, "sleep"):
            event = capture.private_click(env, 789, [400, 445])
        self.assertEqual([call.args[0] for call in run.call_args_list], [
            ["xdotool", "windowfocus", "--sync", "456"],
            ["xdotool", "mousemove", "400", "445"],
            ["xdotool", "mousedown", "1"],
            ["xdotool", "mouseup", "1"],
        ])
        self.assertTrue(all(call.kwargs["env"] == env for call in run.call_args_list))
        self.assertIsNone(event["pointer_moved"])
        self.assertTrue(event["pointer_move_requested"])
        self.assertEqual(event["position"], [400, 445])
        self.assertFalse(event["guest_memory_written"])

    def test_captured_relative_motion_precedes_press_on_private_display(self):
        env = {"DISPLAY": ":123"}
        with mock.patch.object(capture.subprocess, "check_output", return_value="456\n"), \
                mock.patch.object(capture, "private_mouse_locked", return_value=True), \
                mock.patch.object(capture.subprocess, "run") as run, mock.patch.object(capture.time, "sleep"):
            event = capture.private_click(env, 789, capture_mouse=True, relative_motion=[-300, 20])
        self.assertEqual([call.args[0] for call in run.call_args_list], [
            ["xdotool", "windowfocus", "--sync", "456"],
            ["xdotool", "mousemove_relative", "--", "-300", "20"],
            ["xdotool", "mousedown", "1"], ["xdotool", "mouseup", "1"],
        ])
        self.assertTrue(all(call.kwargs["env"] == env for call in run.call_args_list))
        self.assertEqual(event["relative_motion_requested"], [-300, 20])

    def test_relative_motion_rejects_ambiguous_or_invalid_requests_before_input(self):
        with mock.patch.object(capture.subprocess, "check_output") as search:
            for kwargs in ({"relative_motion": [1, 0]},
                           {"capture_mouse": True, "relative_motion": [1, 0], "position": [1, 1]},
                           {"capture_mouse": True, "relative_motion": [32768, 0]},
                           {"capture_mouse": True, "relative_motion": [1.5, 0]}):
                with self.assertRaises(ValueError):
                    capture.private_click({"DISPLAY": ":123"}, 789, **kwargs)
            search.assert_not_called()

    def test_positioned_click_rejects_out_of_display_before_input(self):
        with mock.patch.object(capture.subprocess, "check_output") as search:
            for position in ([-1, 0], [0, -1], [800, 0], [0, 600], [0]):
                with self.assertRaises(ValueError):
                    capture.private_click({"DISPLAY": ":123"}, 789, position)
            search.assert_not_called()

    def test_relative_mouse_requires_confirmed_private_capture_before_click(self):
        env = {"DISPLAY": ":123"}
        with mock.patch.object(capture.subprocess, "check_output", return_value="456\n"), \
                mock.patch.object(capture, "private_mouse_locked", side_effect=[False, True]), \
                mock.patch.object(capture.subprocess, "run") as run, mock.patch.object(capture.time, "sleep"):
            event = capture.private_click(env, 789, capture_mouse=True)
        self.assertEqual([call.args[0] for call in run.call_args_list], [
            ["xdotool", "windowfocus", "--sync", "456"], ["xdotool", "click", "1"],
            ["xdotool", "mousedown", "1"], ["xdotool", "mouseup", "1"],
        ])
        self.assertTrue(all(call.kwargs["env"] == env for call in run.call_args_list))
        self.assertTrue(event["mouse_capture_requested"])
        self.assertTrue(event["capture_click_sent"])

    def test_relative_mouse_refuses_unconfirmed_capture(self):
        with mock.patch.object(capture.subprocess, "check_output", return_value="456\n"), \
                mock.patch.object(capture, "private_mouse_locked", return_value=False), \
                mock.patch.object(capture.subprocess, "run") as run, mock.patch.object(capture.time, "sleep"):
            with self.assertRaises(RuntimeError):
                capture.private_click({"DISPLAY": ":123"}, 789, capture_mouse=True)
        self.assertEqual(len(run.call_args_list), 2)

    def test_relative_mouse_does_not_click_to_acquire_an_existing_lock(self):
        with mock.patch.object(capture.subprocess, "check_output", return_value="456\n"), \
                mock.patch.object(capture, "private_mouse_locked", return_value=True), \
                mock.patch.object(capture.subprocess, "run") as run, mock.patch.object(capture.time, "sleep"):
            event = capture.private_click({"DISPLAY": ":123"}, 789, capture_mouse=True)
        self.assertEqual(len(run.call_args_list), 3)
        self.assertFalse(event["capture_click_sent"])
        self.assertTrue(event["mouse_capture_verified"])

    def test_private_mouse_lock_reader_checks_boolean_representation(self):
        for value in (0, 1, 2):
            with self.subTest(value=value), \
                    mock.patch.object(capture, "locate_symbols", return_value=(None, {"mouselocked": (0, 1)})), \
                    mock.patch("builtins.open", return_value=io.BytesIO(bytes([value]))):
                if value == 2:
                    with self.assertRaises(ValueError):
                        capture.private_mouse_locked(789)
                else:
                    self.assertEqual(capture.private_mouse_locked(789), bool(value))

    def test_host_mouse_snapshot_keeps_capture_and_last_motion_flags_separate(self):
        fields = ["mouselocked", "user_cursor_locked", "user_cursor_emulation", "user_cursor_x",
                  "user_cursor_y", "user_cursor_sw", "user_cursor_sh"]
        memory = struct.pack("<BB5i", 1, 0, 2, -10, 150, 640, 400)
        symbols = {name: (index if index < 2 else 2 + 4 * (index - 2), 1 if index < 2 else 4)
                   for index, name in enumerate(fields)}
        observed = capture.read_host_mouse(io.BytesIO(memory), symbols)
        self.assertEqual(list(observed.values()), [True, False, 2, -10, 150, 640, 400])
        with self.assertRaisesRegex(ValueError, "cursor bool"):
            capture.read_host_mouse(io.BytesIO(bytes([2]) + memory[1:]), symbols)

    def test_cli_preserves_ordered_click_schedule(self):
        argv = ["capture", "disc", "output", "--seconds", "105", "--click-after", "35",
                "--click-after", "70", "--click-position", "400", "445"]
        with mock.patch.object(sys, "argv", argv), mock.patch.object(capture, "capture", return_value=True) as run:
            with self.assertRaises(SystemExit) as stopped:
                capture.main()
        self.assertEqual(stopped.exception.code, 0)
        self.assertEqual(run.call_args.args[0].click_after, [35.0, 70.0])
        self.assertEqual(run.call_args.args[0].click_position, [400, 445])

    def test_cli_preserves_bounded_motion_steps(self):
        argv = ["capture", "disc", "output", "--relative-mouse", "--click-after", "30",
                "--move-after", "35", "-600", "0", "--motion-steps", "100"]
        with mock.patch.object(sys, "argv", argv), mock.patch.object(capture, "capture", return_value=True) as run:
            with self.assertRaises(SystemExit) as stopped:
                capture.main()
        self.assertEqual(stopped.exception.code, 0)
        self.assertEqual(run.call_args.args[0].motion_steps, 100)

    def test_cli_rejects_invalid_or_unused_motion_steps(self):
        for arguments in (["--motion-steps", "0"], ["--motion-steps", "129"], ["--motion-steps", "2"]):
            with self.subTest(arguments=arguments), \
                    mock.patch.object(sys, "argv", ["capture", "disc", "output", *arguments]), \
                    mock.patch.object(capture, "capture") as run, mock.patch.object(sys, "stderr", io.StringIO()):
                with self.assertRaises(SystemExit) as stopped:
                    capture.main()
                self.assertEqual(stopped.exception.code, 2)
                run.assert_not_called()

    def test_cli_preserves_steering_schedule(self):
        argv = ["capture", "disc", "output", "--relative-mouse", "--click-after", "30",
                "--steer-after", "35", "45"]
        with mock.patch.object(sys, "argv", argv), mock.patch.object(capture, "capture", return_value=True) as run:
            with self.assertRaises(SystemExit) as stopped:
                capture.main()
        self.assertEqual(stopped.exception.code, 0)
        self.assertEqual(run.call_args.args[0].steer_after, [(35.0, 45)])

    def test_cli_preserves_pointer_schedule(self):
        argv = ["capture", "disc", "output", "--relative-mouse", "--click-after", "30",
                "--point-after", "35", "125", "118"]
        with mock.patch.object(sys, "argv", argv), mock.patch.object(capture, "capture", return_value=True) as run:
            with self.assertRaises(SystemExit) as stopped:
                capture.main()
        self.assertEqual(stopped.exception.code, 0)
        self.assertEqual(run.call_args.args[0].point_after, [(35.0, [125, 118])])

    def test_cli_rejects_invalid_pointer_schedule(self):
        cases = [["--point-after", "35", "125", "118"],
                 ["--relative-mouse", "--click-after", "40", "--point-after", "35", "125", "118"],
                 ["--relative-mouse", "--click-after", "30", "--point-after", "35", "320", "118"],
                 ["--relative-mouse", "--click-after", "30", "--point-after", "60", "125", "118"],
                 ["--relative-mouse", "--click-after", "30", "--point-after", "35", "125", "118", "--point-after", "34", "125", "118"],
                 ["--relative-mouse", "--click-after", "30", "--point-after", "35", "125", "118", "--steer-after", "35", "45"]]
        for arguments in cases:
            with self.subTest(arguments=arguments), \
                    mock.patch.object(sys, "argv", ["capture", "disc", "output", *arguments]), \
                    mock.patch.object(capture, "capture") as run, mock.patch.object(sys, "stderr", io.StringIO()):
                with self.assertRaises(SystemExit) as stopped:
                    capture.main()
                self.assertEqual(stopped.exception.code, 2)
                run.assert_not_called()

    def test_cli_rejects_invalid_steering_schedule(self):
        cases = [["--steer-after", "35", "45"],
                 ["--relative-mouse", "--click-after", "40", "--steer-after", "35", "45"],
                 ["--relative-mouse", "--click-after", "30", "--steer-after", "35", "180"],
                 ["--relative-mouse", "--click-after", "30", "--steer-after", "60", "45"],
                 ["--relative-mouse", "--click-after", "30", "--steer-after", "35", "45", "--steer-after", "34", "45"],
                 ["--relative-mouse", "--click-after", "30", "--steer-after", "35", "45", "--move-after", "35", "10", "0"]]
        for arguments in cases:
            with self.subTest(arguments=arguments), \
                    mock.patch.object(sys, "argv", ["capture", "disc", "output", *arguments]), \
                    mock.patch.object(capture, "capture") as run, mock.patch.object(sys, "stderr", io.StringIO()):
                with self.assertRaises(SystemExit) as stopped:
                    capture.main()
                self.assertEqual(stopped.exception.code, 2)
                run.assert_not_called()

    def test_cli_preserves_captured_relative_motion(self):
        argv = ["capture", "disc", "output", "--seconds", "65", "--click-after", "45",
                "--relative-mouse", "--relative-motion", "-300", "20"]
        with mock.patch.object(sys, "argv", argv), mock.patch.object(capture, "capture", return_value=True) as run:
            with self.assertRaises(SystemExit) as stopped:
                capture.main()
        self.assertEqual(stopped.exception.code, 0)
        self.assertEqual(run.call_args.args[0].relative_motion, [-300, 20])
        self.assertTrue(run.call_args.args[0].relative_mouse)

    def test_cli_preserves_key_schedule(self):
        argv = ["capture", "disc", "output", "--key-after", "30", "F7", "--key-after", "45", "Return"]
        with mock.patch.object(sys, "argv", argv), mock.patch.object(capture, "capture", return_value=True) as run:
            with self.assertRaises(SystemExit) as stopped:
                capture.main()
        self.assertEqual(stopped.exception.code, 0)
        self.assertEqual(run.call_args.args[0].key_after, [(30.0, "F7"), (45.0, "Return")])

    def test_cli_preserves_explicit_process_local_sdl_override(self):
        argv = ["capture", "disc", "output", "--sdl-library", "/library/libSDL2.so"]
        with mock.patch.object(sys, "argv", argv), mock.patch.object(capture, "capture", return_value=True) as run:
            with self.assertRaises(SystemExit) as stopped:
                capture.main()
        self.assertEqual(stopped.exception.code, 0)
        self.assertEqual(run.call_args.args[0].sdl_library, Path("/library/libSDL2.so"))

    def test_cli_preserves_explicit_sdl_mouse_mode(self):
        argv = ["capture", "disc", "output", "--sdl-library", "/library/libSDL2.so",
                "--sdl-mouse-warp", "--relative-mouse", "--click-after", "30"]
        with mock.patch.object(sys, "argv", argv), mock.patch.object(capture, "capture", return_value=True) as run:
            with self.assertRaises(SystemExit) as stopped:
                capture.main()
        self.assertEqual(stopped.exception.code, 0)
        self.assertTrue(run.call_args.args[0].sdl_mouse_warp)

    def test_cli_rejects_mouse_warp_without_explicit_library_and_capture(self):
        for options in ([], ["--sdl-library", "/library/libSDL2.so"],
                        ["--relative-mouse", "--click-after", "30"]):
            with self.subTest(options=options), \
                    mock.patch.object(sys, "argv", ["capture", "disc", "output", "--sdl-mouse-warp", *options]), \
                    mock.patch.object(sys, "stderr", new_callable=io.StringIO), \
                    mock.patch.object(capture, "capture") as run:
                with self.assertRaises(SystemExit) as stopped:
                    capture.main()
                self.assertEqual(stopped.exception.code, 2)
                run.assert_not_called()

    def test_sdl_override_rejects_missing_or_invalid_library_before_startup(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "BLOOD2PG.EXE").write_bytes(b"fixture")
            library = root / "SDL.so"
            args = argparse.Namespace(disc=root, sdl_library=library, native_checkpoint=None)
            with mock.patch.object(capture, "EXECUTABLE_SHA256", capture.hashlib.sha256(b"fixture").hexdigest()), \
                    mock.patch.object(capture.subprocess, "Popen") as launch:
                with self.assertRaises(FileNotFoundError):
                    capture.capture(args)
                library.write_bytes(b"not an ELF library")
                with self.assertRaisesRegex(ValueError, "ELF64"):
                    capture.capture(args)
                launch.assert_not_called()

    def test_cli_preserves_movement_schedule(self):
        argv = ["capture", "disc", "output", "--relative-mouse", "--click-after", "30",
                "--move-after", "40", "-480", "0", "--move-after", "50", "0", "-90"]
        with mock.patch.object(sys, "argv", argv), mock.patch.object(capture, "capture", return_value=True) as run:
            with self.assertRaises(SystemExit) as stopped:
                capture.main()
        self.assertEqual(stopped.exception.code, 0)
        self.assertEqual(run.call_args.args[0].move_after, [(40.0, [-480, 0]), (50.0, [0, -90])])

    def test_cli_preserves_recapture_schedule(self):
        argv = ["capture", "disc", "output", "--relative-mouse", "--click-after", "30",
                "--recapture-after", "40", "400", "316", "--recapture-after", "50", "330", "352"]
        with mock.patch.object(sys, "argv", argv), mock.patch.object(capture, "capture", return_value=True) as run:
            with self.assertRaises(SystemExit) as stopped:
                capture.main()
        self.assertEqual(stopped.exception.code, 0)
        self.assertEqual(run.call_args.args[0].recapture_after, [(40.0, [400, 316]), (50.0, [330, 352])])

    def test_cli_rejects_unusable_recapture_schedule(self):
        prefix = ["--relative-mouse", "--click-after", "30"]
        for options in (["--recapture-after", "40", "400", "316"],
                        [*prefix, "--recapture-after", "20", "400", "316"],
                        [*prefix, "--recapture-after", "30", "400", "316"],
                        [*prefix, "--recapture-after", "60", "400", "316"],
                        [*prefix, "--recapture-after", "40", "800", "316"],
                        [*prefix, "--recapture-after", "40", "400", "600"],
                        [*prefix, "--recapture-after", "40", "-1", "316"],
                        [*prefix, "--recapture-after", "40", "1.5", "316"],
                        [*prefix, "--recapture-after", "nan", "400", "316"],
                        [*prefix, "--recapture-after", "40", "400", "316", "--recapture-after", "39", "400", "316"],
                        [*prefix, "--recapture-after", "40", "400", "316", "--key-after", "40", "F7"],
                        [*prefix, "--recapture-after", "40", "400", "316", "--move-after", "40", "1", "0"]):
            with self.subTest(options=options), \
                    mock.patch.object(sys, "argv", ["capture", "disc", "output", *options]), \
                    mock.patch.object(sys, "stderr", new_callable=io.StringIO), \
                    mock.patch.object(capture, "capture") as run:
                with self.assertRaises(SystemExit) as stopped:
                    capture.main()
                self.assertEqual(stopped.exception.code, 2)
                run.assert_not_called()

    def test_cli_rejects_unusable_movement_schedule(self):
        for options in (["--move-after", "40", "0", "0"],
                        ["--relative-mouse", "--click-after", "30", "--move-after", "20", "0", "0"],
                        ["--relative-mouse", "--click-after", "30", "--move-after", "30", "0", "0"],
                        ["--relative-mouse", "--click-after", "30", "--move-after", "60", "0", "0"],
                        ["--relative-mouse", "--click-after", "30", "--move-after", "40", "32768", "0"],
                        ["--relative-mouse", "--click-after", "30", "--move-after", "40", "1.5", "0"],
                        ["--relative-mouse", "--click-after", "30", "--move-after", "nan", "0", "0"],
                        ["--relative-mouse", "--click-after", "30", "--move-after", "40", "0", "0",
                         "--move-after", "35", "0", "0"],
                        ["--relative-mouse", "--click-after", "30", "--move-after", "40", "0", "0",
                         "--key-after", "40", "F7"]):
            with self.subTest(options=options), \
                    mock.patch.object(sys, "argv", ["capture", "disc", "output", *options]), \
                    mock.patch.object(sys, "stderr", new_callable=io.StringIO), \
                    mock.patch.object(capture, "capture") as run:
                with self.assertRaises(SystemExit) as stopped:
                    capture.main()
                self.assertEqual(stopped.exception.code, 2)
                run.assert_not_called()

    def test_cli_rejects_invalid_key_schedule(self):
        for options in (["60", "F7"], ["nan", "F7"], ["inf", "F7"], ["0", "F7"],
                        ["bad", "F7"], ["20", "F12"], ["20", "F7", "--key-after", "20", "Return"],
                        ["20", "F7", "--key-after", "10", "Return"],
                        ["20", "F7", "--click-after", "20"]):
            with self.subTest(options=options), \
                    mock.patch.object(sys, "argv", ["capture", "disc", "output", "--key-after", *options]), \
                    mock.patch.object(sys, "stderr", new_callable=io.StringIO), \
                    mock.patch.object(capture, "capture") as run:
                with self.assertRaises(SystemExit) as stopped:
                    capture.main()
                self.assertEqual(stopped.exception.code, 2)
                run.assert_not_called()

    def test_cli_rejects_unusable_relative_motion(self):
        for options in (["--relative-motion", "1", "0"],
                        ["--relative-mouse", "--relative-motion", "32768", "0"],
                        ["--relative-mouse", "--relative-motion", "1", "0", "--click-position", "1", "1"]):
            argv = ["capture", "disc", "output", "--click-after", "45", *options]
            with self.subTest(options=options), mock.patch.object(sys, "argv", argv), \
                    mock.patch.object(sys, "stderr", new_callable=io.StringIO), \
                    mock.patch.object(capture, "capture") as run:
                with self.assertRaises(SystemExit) as stopped:
                    capture.main()
                self.assertEqual(stopped.exception.code, 2)
                run.assert_not_called()
    def test_cli_rejects_invalid_input_schedule_before_capture(self):
        for flags in (["--click-after", "60"],
                      ["--click-button", "3"],
                      ["--relative-mouse"],
                      ["--click-after", "35", "--click-after", "20"],
                      ["--click-after", "35", "--click-after", "35"],
                      ["--click-position", "400", "445"],
                      ["--click-after", "35", "--click-position", "800", "0"]):
            with self.subTest(flags=flags), mock.patch.object(sys, "argv", ["capture", "disc", "output", *flags]), \
                    mock.patch.object(sys, "stderr", io.StringIO()), mock.patch.object(capture, "capture") as run:
                with self.assertRaises(SystemExit) as stopped:
                    capture.main()
                self.assertEqual(stopped.exception.code, 2)
                run.assert_not_called()

    def test_read_exact_reports_short_read(self):
        with self.assertRaises(ValueError):
            capture.read_exact(io.BytesIO(b"ab"), 1, 2)

    def test_nonfinite_or_nonpositive_capture_duration_is_rejected(self):
        for value in ("nan", "inf", "0", "-1"):
            with self.assertRaises(argparse.ArgumentTypeError):
                capture.positive_seconds(value)
        self.assertEqual(capture.positive_seconds("0.5"), 0.5)


if __name__ == "__main__":
    unittest.main()
