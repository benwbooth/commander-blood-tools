import json
from pathlib import Path
import tempfile
import unittest

from rebuild_anthology_travel import assign_workers, prepare, travel_plans
from video_anthology import digest


class RebuildTravelTests(unittest.TestCase):
    def test_source_bound_plans_are_prepared_once_and_partitioned(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            sources = []
            for index, entry in enumerate(("travel", "contact", "travel")):
                path = root / f"chapter-{index}"
                path.mkdir()
                plan = dict(game="commander_blood", entry=entry, title=f"title-{index}")
                (path / "report.json").write_text(json.dumps(dict(runner=dict(chapter=plan))))
                sources.append(dict(record=plan["title"], path=str(path),
                                    duration_ns=(index + 1) * 100,
                                    report_sha256=digest(path / "report.json")))
            manifest = dict(game="commander_blood", sources=sources,
                            chapters=[dict(title=entry["record"]) for entry in sources])
            plans = travel_plans(manifest)
            self.assertEqual([item[2]["title"] for item in plans], ["title-0", "title-2"])
            self.assertEqual([len(group) for group in assign_workers(plans, 2)], [1, 1])
            groups = prepare(manifest, root / "out", 2)
            self.assertEqual(sum(map(len, groups)), 2)
            self.assertEqual([path for group in groups for path in group if path.exists()],
                             [path for group in groups for path in group])
            self.assertEqual(prepare(manifest, root / "out", 2), groups)
            sources[0]["report_sha256"] = "changed"
            with self.assertRaisesRegex(ValueError, "source report changed"):
                travel_plans(manifest)


if __name__ == "__main__":
    unittest.main()
