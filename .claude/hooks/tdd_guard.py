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

# コメントと実装を一致させる: #[test] 系属性、proptest!、#[cfg(test)] mod tests のいずれかを
# テストの追加・変更とみなす。追加行 (+) だけでなく削除行 (-) とコンテキスト行 (先頭が空白) も
# 対象にする。既存テストの本文だけを書き換える(#[test] 行自体には触れない)変更でも、
# diff のハンク内にその #[test] 行がコンテキストとして含まれていれば検出できるようにするため。
TEST_MARKERS = re.compile(
    r"("
    r"#\[(tokio::)?test\]"
    r"|#\[rstest\]"
    r"|#\[cfg\(test\)\]"
    r"|proptest!\s*\("
    r"|mod\s+tests\b"
    r")"
)

# diff のハンク本文行 (+/-/コンテキスト) かどうかを判定する。
# `+++`/`---` のファイルヘッダ行は除外する。
def is_hunk_body_line(line: str) -> bool:
    if not line or line.startswith("+++") or line.startswith("---"):
        return False
    return line[0] in ("+", "-", " ")

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
    if "[skip-tdd]" in desc:
        return 0

    name_only = run(root, ["jj", "diff", "--name-only"]).stdout
    files = [line for line in name_only.splitlines() if line.strip()]
    if not files:
        return 0

    src_changed = [f for f in files if re.search(r"(^|/)src/.*\.rs$", f)]
    if not src_changed:
        return 0

    test_files = [f for f in files if re.search(r"(^|/)tests/.*\.rs$", f)]

    # コンテキスト行を広めに取り、既存テスト本文の変更でも #[test] 属性行が
    # 同じハンクに含まれやすくする。
    git_diff = run(root, ["jj", "diff", "--git", "--context", "20"]).stdout
    has_test_diff = any(
        TEST_MARKERS.search(line[1:])
        for line in git_diff.splitlines()
        if is_hunk_body_line(line)
    )

    if not test_files and not has_test_diff:
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
