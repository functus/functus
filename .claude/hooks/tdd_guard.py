#!/usr/bin/env python3
"""Stop hook: TDD ガード。

1) src/ の Rust コードが変更されているのにテストの追加・変更が無い場合は停止をブロック
2) cargo test が失敗する (Red のまま) 状態での停止もブロック

例外: change description に [skip-tdd] が含まれる場合はスキップ。
"""
import json
import os
import re
import shutil
import subprocess
import sys
from typing import Optional

# コメントと実装を一致させる: #[test] 系属性、proptest!、#[cfg(test)] mod tests のいずれかを
# テストの追加・削除とみなす。diff のコンテキスト行は対象にしない。
# 近くに既存の #[test] があるだけの実装変更を「テスト変更あり」と
# 誤判定しないため。既存テストの本文だけを変更する場合は tests/ 配下に置くか、
# [skip-tdd] で意図を明示する。
TEST_MARKERS = re.compile(
    r"("
    r"#\[(tokio::)?test\]"
    r"|#\[rstest\]"
    r"|#\[cfg\(test\)\]"
    r"|proptest!\s*[({]"
    r"|mod\s+tests\b"
    r")"
)

def is_changed_hunk_line(line: str) -> bool:
    """diff の実際の追加・削除行かどうかを判定する。"""
    return bool(line) and line[0] in ("+", "-") and not line.startswith(("+++", "---"))


class RustTestScope:
    """Rust の test 属性に続くブロック内かを追跡する軽量スキャナ。"""

    def __init__(self) -> None:
        self.depth = 0
        self.test_depths: list[int] = []
        self.pending_test = False
        self.in_block_comment = False
        self.quote: Optional[str] = None
        self.raw_end: Optional[str] = None

    def code_only(self, source: str) -> str:
        """文字列・行コメント・ブロックコメントの内側を空白化する。"""
        result: list[str] = []
        index = 0
        escaped = False
        while index < len(source):
            pair = source[index:index + 2]
            char = source[index]
            if self.in_block_comment:
                if pair == "*/":
                    self.in_block_comment = False
                    index += 2
                else:
                    index += 1
                continue
            if self.raw_end:
                end = source.find(self.raw_end, index)
                if end < 0:
                    return "".join(result)
                index = end + len(self.raw_end)
                self.raw_end = None
                continue
            if self.quote:
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == self.quote:
                    self.quote = None
                index += 1
                continue
            if pair == "//":
                break
            if pair == "/*":
                self.in_block_comment = True
                index += 2
                continue
            raw = re.match(r'r(#+)?"', source[index:])
            if raw:
                hashes = raw.group(1) or ""
                self.raw_end = '"' + hashes
                index += len(raw.group(0))
                continue
            if char == '"' or (char == "'" and re.match(r"'(?:\\.|[^\\'])'", source[index:])):
                self.quote = char
                index += 1
                continue
            result.append(char)
            index += 1
        return "".join(result)

    def feed(self, source: str) -> bool:
        source = self.code_only(source)
        was_in_test = bool(self.test_depths)
        marker = bool(TEST_MARKERS.search(source))
        if marker:
            self.pending_test = True
        opens = source.count("{")
        closes = source.count("}")
        if self.pending_test and opens:
            self.test_depths.append(self.depth + 1)
            self.pending_test = False
        elif self.pending_test and ";" in source:
            # `#[cfg(test)] mod tests;` は外部モジュール宣言で、後続ブロックはテストではない。
            self.pending_test = False
        self.depth += opens - closes
        while self.test_depths and self.depth < self.test_depths[-1]:
            self.test_depths.pop()
        return marker or was_in_test or bool(self.test_depths)


def has_test_changes(git_diff: str) -> bool:
    old_scope = RustTestScope()
    new_scope = RustTestScope()
    is_rust_file = False
    for line in git_diff.splitlines():
        if line.startswith("diff --git "):
            old_scope, new_scope = RustTestScope(), RustTestScope()
            is_rust_file = line.rsplit(" b/", 1)[-1].endswith(".rs")
            continue
        if not is_rust_file:
            continue
        if not line or line.startswith(("+++", "---", "@@")):
            continue
        prefix, source = line[0], line[1:]
        if prefix == " ":
            old_scope.feed(source)
            new_scope.feed(source)
        elif prefix == "-":
            if old_scope.feed(source):
                return True
        elif prefix == "+":
            if new_scope.feed(source):
                return True
    return False

VIOLATION_MESSAGE = """TDD 違反: src/ の Rust コードが変更されていますが、テストの追加・変更がありません。
Red → Green → Refactor に従ってください:
  1. 期待する振る舞いを表すテストを書き、意図した理由で失敗することを確認する
  2. テストを通す最小限の実装を書く
テストが原理的に不要な変更 (doc のみ・再エクスポートのみ等) の場合のみ、
description に [skip-tdd] を含めてください (.claude/rules/tdd.md 参照)。"""


def run(root: str, args: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(
        args, cwd=root, capture_output=True, text=True, check=False
    )


def main() -> int:
    raw = sys.stdin.read()
    try:
        payload = json.loads(raw) if raw else {}
    except json.JSONDecodeError:
        payload = {}

    if payload.get("stop_hook_active", False):
        return 0

    root = os.environ.get("CLAUDE_PROJECT_DIR", os.getcwd())
    if not shutil.which("jj") or not os.path.isdir(os.path.join(root, ".jj")):
        return 0

    desc = run(root, ["jj", "log", "-r", "@", "--no-graph", "-T", "description"]).stdout
    skip_test_change_check = "[skip-tdd]" in desc

    name_only = run(root, ["jj", "diff", "--name-only"]).stdout
    files = [line for line in name_only.splitlines() if line.strip()]
    if not files:
        return 0

    rust_changed = [f for f in files if f.endswith(".rs")]
    src_changed = [f for f in rust_changed if re.search(r"(^|/)src/.*\.rs$", f)]
    test_files = [f for f in files if re.search(r"(^|/)tests/.*\.rs$", f)]
    if not rust_changed:
        return 0

    # test スコープを追跡できるよう、変更ファイルの全コンテキストを取得する。
    git_diff = run(root, ["jj", "diff", "--git", "--context", "100000"]).stdout
    has_test_diff = has_test_changes(git_diff)

    if src_changed and not skip_test_change_check and not (test_files or has_test_diff):
        print(VIOLATION_MESSAGE, file=sys.stderr)
        return 2

    if os.path.isfile(os.path.join(root, "Cargo.toml")):
        test_result = run(root, ["cargo", "test", "--quiet"])
        if test_result.returncode != 0:
            print(
                "cargo test が失敗しています (Red の状態)。"
                "Green まで完了させてから停止してください:",
                file=sys.stderr,
            )
            combined = test_result.stdout.splitlines() + test_result.stderr.splitlines()
            failures = [
                line
                for line in combined
                if re.search(r"FAILED|panicked|error(\[|:)", line)
            ]
            print("\n".join(failures[:15]), file=sys.stderr)
            return 2

    return 0


if __name__ == "__main__":
    sys.exit(main())
