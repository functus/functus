#!/usr/bin/env python3
"""PostToolUse hook: Rust ファイル編集後に fmt + clippy で品質を維持する。

問題があれば exit 2 で Claude にフィードバックし、その場で修正させる。
"""
import json
import os
import subprocess
import sys


def main() -> int:
    raw = sys.stdin.read()
    try:
        payload = json.loads(raw) if raw else {}
    except json.JSONDecodeError:
        payload = {}

    file_path = payload.get("tool_input", {}).get("file_path", "") or ""
    if not file_path.endswith(".rs"):
        return 0

    root = os.environ.get("CLAUDE_PROJECT_DIR", os.getcwd())
    if not os.path.isfile(os.path.join(root, "Cargo.toml")):
        return 0

    # 1) rustfmt: 自動整形(失敗しても続行)
    subprocess.run(
        ["cargo", "fmt", "--quiet"], cwd=root, capture_output=True, check=False
    )

    # 2) clippy: 警告をエラー扱いで検査
    result = subprocess.run(
        [
            "cargo",
            "clippy",
            "--quiet",
            "--all-targets",
            "--message-format",
            "short",
            "--",
            "-D",
            "warnings",
        ],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        print("clippy で問題が検出されました。修正してください:", file=sys.stderr)
        lines = [
            line
            for line in result.stdout.splitlines() + result.stderr.splitlines()
            if "error" in line or "warning" in line
        ]
        print("\n".join(lines[:30]), file=sys.stderr)
        return 2

    return 0


if __name__ == "__main__":
    sys.exit(main())
