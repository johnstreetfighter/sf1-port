import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import compare_captures


def make_dir(files: dict[str, bytes]) -> Path:
    d = Path(tempfile.mkdtemp())
    for name, content in files.items():
        (d / name).write_bytes(content)
    return d


class CompareCaptureDirsTest(unittest.TestCase):
    def test_identical_directories_report_identical(self):
        a = make_dir({"frames.tsv": b"1\tabc\n", "meta.tsv": b"dsw1\tdfff\n"})
        b = make_dir({"frames.tsv": b"1\tabc\n", "meta.tsv": b"dsw1\tdfff\n"})
        result = compare_captures.compare_capture_dirs(a, b)
        self.assertTrue(result.identical)
        self.assertIsNone(result.first_difference)

    def test_content_difference_reports_file_and_first_offset(self):
        a = make_dir({"frames.tsv": b"frame\t1\nframe\t2\n"})
        b = make_dir({"frames.tsv": b"frame\t1\nframe\t9\n"})
        result = compare_captures.compare_capture_dirs(a, b)
        self.assertFalse(result.identical)
        file, offset = result.first_difference
        self.assertEqual("frames.tsv", file)
        self.assertEqual(b"frame\t1\nframe\t".__len__(), offset)

    def test_missing_file_is_reported(self):
        a = make_dir({"frames.tsv": b"x", "pixels_000600.rgba": b"y"})
        b = make_dir({"frames.tsv": b"x"})
        result = compare_captures.compare_capture_dirs(a, b)
        self.assertFalse(result.identical)
        self.assertEqual("pixels_000600.rgba", result.first_difference[0])
    def test_tooling_artifacts_are_not_observations(self):
        a = make_dir(
            {
                "frames.tsv": b"1\tabc\n",
                "generated_driver.lua": b"out_dir = run-a",
                "summary.json": b'{"wall_seconds": 1}',
            }
        )
        b = make_dir(
            {
                "frames.tsv": b"1\tabc\n",
                "generated_driver.lua": b"out_dir = run-b",
                "summary.json": b'{"wall_seconds": 2}',
            }
        )
        result = compare_captures.compare_capture_dirs(a, b)
        self.assertTrue(result.identical)
        self.assertEqual(1, result.files_compared)


if __name__ == "__main__":
    unittest.main()
