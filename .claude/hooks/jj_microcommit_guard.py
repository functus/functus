#!/usr/bin/env python3
"""Stop hook: マイクロコミット運用のガード。

作業コピー (@) に変更があるのに description が無いまま停止しようとしたらブロックし、
jj desc (Conventional Commits + 経緯) -> jj new を促す。
"""
import json
import os
import shutil
import subprocess
import sys

GUIDANCE = """作業コピー (@) に未記述の変更があります。マイクロコミット運用に従ってください:
  1. jj diff で変更内容を確認する
  2. /jj-commit スキルに従い、Conventional Commits 形式 + 実装の経緯を含む description を jj desc で記述する
  3. jj new で次の変更を開始する"""


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

    result = subprocess.run(
        [
            "jj",
            "log",
            "-r",
            "@",
            "--no-graph",
            "-T",
            'if(empty, "empty", "dirty") ++ "|" ++ '
            'if(description, "described", "nodesc")',
        ],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    state = result.stdout.strip()
    if not state:
        return 0

    if state == "dirty|nodesc":
        print(GUIDANCE, file=sys.stderr)
        return 2

    return 0


if __name__ == "__main__":
    sys.exit(main())
