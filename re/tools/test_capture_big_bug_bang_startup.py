#!/usr/bin/env python3
"""Synthetic memory tests; no original game bytes or running emulator needed."""

import argparse
import importlib.util
import io
from pathlib import Path
import struct
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location(
    "capture_big_bug_bang_startup", Path(__file__).with_name("capture_big_bug_bang_startup.py"))
capture = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(capture)


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

    def test_requires_both_vm_and_full_catalog_anchors(self):
        for offset in (capture.VM_FILE, capture.CATALOG_NAMES_FILE + 100):
            executable, guest, _globals, _catalog = fixture()
            guest[65536 + offset - capture.MZ_HEADER_SIZE] ^= 255
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
