#!/usr/bin/env python3

from __future__ import annotations

import collections
import importlib.util
import json
from pathlib import Path
import sys
import unittest

RE_ROOT = Path(__file__).resolve().parent.parent
TOOL_PATH = Path(__file__).with_name("build_function_graph.py")
SPEC = importlib.util.spec_from_file_location("build_function_graph", TOOL_PATH)
assert SPEC is not None and SPEC.loader is not None
GRAPH_TOOL = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = GRAPH_TOOL
SPEC.loader.exec_module(GRAPH_TOOL)


def indirect_counts(graph: dict[str, object]) -> collections.Counter[str]:
    return collections.Counter(
        json.dumps(site, separators=(",", ":")) for site in graph["indirect"]
    )


class FunctionGraphTests(unittest.TestCase):
    def test_commander_graph_matches_checked_in_baseline(self):
        expected = json.loads((RE_ROOT / "func_graph.json").read_text())
        actual = GRAPH_TOOL.build_function_graph(
            RE_ROOT / "bin" / "BLOODPRG.EXE"
        )

        self.assertEqual(expected["funcs"], actual["funcs"])
        self.assertEqual(expected["leaves"], actual["leaves"])
        self.assertEqual(expected["callgraph"], actual["callgraph"])
        self.assertEqual(indirect_counts(expected), indirect_counts(actual))


if __name__ == "__main__":
    unittest.main()
