import importlib.util
import unittest
from pathlib import Path


HOOK_PATH = Path(__file__).parents[1] / "pr_description_reminder.py"
SPEC = importlib.util.spec_from_file_location("pr_description_reminder", HOOK_PATH)
reminder = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(reminder)


class BookmarkTests(unittest.TestCase):
    def test_quoted_bookmark_is_unquoted(self) -> None:
        self.assertEqual(
            reminder.explicit_bookmarks('jj git push --bookmark "feature/foo"'),
            ["feature/foo"],
        )

    def test_short_bookmark(self) -> None:
        self.assertEqual(reminder.explicit_bookmarks("jj git push -b topic"), ["topic"])

    def test_plain_push_output_does_not_capture_json_delimiters(self) -> None:
        match = reminder.PUSHED_BOOKMARK.search('bookmark: main\\n"')
        self.assertIsNotNone(match)
        self.assertEqual(match.group(1), "main")


if __name__ == "__main__":
    unittest.main()
