import importlib.util
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch


HOOK_PATH = Path(__file__).parents[1] / "sync_git_notes.py"
SPEC = importlib.util.spec_from_file_location("sync_git_notes", HOOK_PATH)
notes = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(notes)


class DescriptionTests(unittest.TestCase):
    def test_failed_read_is_not_an_empty_description(self) -> None:
        failed = subprocess.CompletedProcess([], 1, stdout="", stderr="failure")
        with patch.object(notes, "run", return_value=failed):
            self.assertIsNone(notes.get_description("/tmp", "change"))

    def test_successful_empty_description_is_preserved(self) -> None:
        succeeded = subprocess.CompletedProcess([], 0, stdout="", stderr="")
        with patch.object(notes, "run", return_value=succeeded):
            self.assertEqual(notes.get_description("/tmp", "change"), "")


if __name__ == "__main__":
    unittest.main()
