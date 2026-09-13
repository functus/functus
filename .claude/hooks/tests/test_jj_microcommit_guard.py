import importlib.util
import unittest
from pathlib import Path


HOOK_PATH = Path(__file__).parents[1] / "jj_microcommit_guard.py"
SPEC = importlib.util.spec_from_file_location("jj_microcommit_guard", HOOK_PATH)
guard = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(guard)


class DescriptionTests(unittest.TestCase):
    def test_markdown_blank_lines_are_allowed(self) -> None:
        description = """fix(core): example

## 経緯

reason

## 実装内容

implementation
"""
        self.assertIsNotNone(guard.DESCRIPTION_PATTERN.search(description))

    def test_missing_sections_are_rejected(self) -> None:
        self.assertIsNone(guard.DESCRIPTION_PATTERN.search("fix(core): example"))

    def test_text_before_header_is_rejected(self) -> None:
        description = "prefix\nfix(core): example\n\n## 経緯\nreason\n\n## 実装内容\nwork"
        self.assertIsNone(guard.DESCRIPTION_PATTERN.search(description))


if __name__ == "__main__":
    unittest.main()
