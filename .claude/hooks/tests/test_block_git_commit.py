import importlib.util
import unittest
from pathlib import Path


HOOK_PATH = Path(__file__).parents[1] / "block_git_commit.py"
SPEC = importlib.util.spec_from_file_location("block_git_commit", HOOK_PATH)
block_git_commit = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(block_git_commit)


class GitCommandTests(unittest.TestCase):
    def test_quoted_global_option_is_blocked(self) -> None:
        self.assertTrue(
            block_git_commit.contains_blocked_git_command(
                'git -C "path with spaces" commit -m test'
            )
        )

    def test_command_substitution_is_blocked(self) -> None:
        self.assertTrue(block_git_commit.contains_blocked_git_command("$(git commit)"))

    def test_nested_shell_is_blocked(self) -> None:
        self.assertTrue(
            block_git_commit.contains_blocked_git_command("bash -c 'git commit -m test'")
        )

    def test_variable_command_is_blocked(self) -> None:
        self.assertTrue(block_git_commit.contains_blocked_git_command("g=git; $g commit"))

    def test_fully_variable_command_is_blocked(self) -> None:
        self.assertTrue(
            block_git_commit.contains_blocked_git_command("g=git; s=commit; $g $s")
        )

    def test_echoing_words_is_allowed(self) -> None:
        self.assertFalse(block_git_commit.contains_blocked_git_command("echo git commit"))

    def test_revert_is_blocked(self) -> None:
        self.assertTrue(block_git_commit.contains_blocked_git_command("git revert HEAD"))

    def test_read_only_git_command_is_allowed(self) -> None:
        self.assertFalse(block_git_commit.contains_blocked_git_command("git log --oneline"))


if __name__ == "__main__":
    unittest.main()
