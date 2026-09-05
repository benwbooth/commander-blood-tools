import struct
import unittest

from compare_game_discs import archive_entries, compare, mz_header, native_vm_table


class ComparisonTests(unittest.TestCase):
    def test_archive_reads_packed_extent(self):
        data = b"\0\0" + struct.pack("<16siiB", b"A.XDB", 3, 28, 0) + b"\0abc"
        self.assertEqual(archive_entries(data), {"A.XDB": b"abc"})

    def test_archive_rejects_invalid_extent(self):
        data = b"\0\0" + struct.pack("<16siiB", b"A.XDB", 3, 200, 0) + b"\0"
        with self.assertRaisesRegex(ValueError, "invalid archive extent"):
            archive_entries(data)

    def test_archive_requires_terminator(self):
        with self.assertRaisesRegex(ValueError, "missing archive directory terminator"):
            archive_entries(b"\0\0")

    def test_identical_duplicates_are_collapsed(self):
        entry = struct.pack("<16siiB", b"A.XDB", 3, 53, 0)
        self.assertEqual(archive_entries(b"\0\0" + entry * 2 + b"\0abc"), {"A.XDB": b"abc"})

    def test_conflicting_duplicates_are_rejected(self):
        first = struct.pack("<16siiB", b"A.XDB", 3, 53, 0)
        second = struct.pack("<16siiB", b"A.XDB", 3, 56, 0)
        with self.assertRaisesRegex(ValueError, "conflicting duplicate"):
            archive_entries(b"\0\0" + first + second + b"\0abcdef")

    def test_archive_rejects_truncated_entry(self):
        with self.assertRaisesRegex(ValueError, "truncated archive directory"):
            archive_entries(b"\0\0A")

    def test_renamed_reuse_and_modified_content_are_distinct(self):
        result = compare({"A": b"a", "B": b"b"}, {"A": b"c", "C": b"b"})
        self.assertEqual(result["same_name_changed"], ["A"])
        self.assertEqual(result["sequel_reused_any_name"], ["C"])
        self.assertEqual(result["sequel_reused_payload_bytes"], 1)

    def test_mz_entry_is_relative_to_header(self):
        words = [23117, 0, 1, 2, 4, 0, 0, 0, 0, 0, 3, 2, 28, 0]
        self.assertEqual(mz_header(struct.pack("<14H", *words))["entry_file_offset"], 99)
        self.assertIsNone(mz_header(b"not an executable"))

    def test_unknown_build_does_not_use_fixed_vm_offsets(self):
        self.assertIsNone(native_vm_table(b"not the known game binary"))


if __name__ == "__main__":
    unittest.main()
