#!/usr/bin/env python3
"""SessionStart hook: セッション開始時に jj の現在状態をコンテキストへ注入する。

レートリミット等で中断した後の新セッションでも、前回の作業状態
(未記述の変更・直近の change・ブックマーク) から即座に再開できるようにする。
"""
import os
import shutil
import subprocess
import sys


def run(root: str, args: list[str]) -> str:
    result = subprocess.run(
        args, cwd=root, capture_output=True, text=True, check=False
    )
    return result.stdout


def main() -> int:
    root = os.environ.get("CLAUDE_PROJECT_DIR", os.getcwd())
    if not shutil.which("jj") or not os.path.isdir(os.path.join(root, ".jj")):
        return 0

    print("## jj リポジトリ状態 (セッション開始時)")
    print("```")
    st = run(root, ["jj", "st"])
    print("\n".join(st.splitlines()[:20]))
    print("---")
    log = run(
        root,
        [
            "jj",
            "log",
            "--limit",
            "5",
            "--no-graph",
            "-T",
            'change_id.short() ++ " " ++ if(bookmarks, bookmarks ++ " ", "") ++ '
            'if(description, description.first_line(), "(no description)") ++ "\\n"',
        ],
    )
    print(log, end="")
    print("```")

    state = run(
        root,
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
    ).strip()
    if state == "dirty|nodesc":
        print(
            "⚠️ 前回セッションの未記述の変更が @ に残っています。"
            "内容を jj diff で確認し、作業を継続するか /jj-commit で確定してください。"
        )

    if os.path.isfile(os.path.join(root, "Cargo.toml")):
        print(
            "品質ゲート: cargo fmt --check / clippy -D warnings / cargo test を"
            "編集前に一度確認すると差分起因の失敗を切り分けやすい。"
        )

    return 0


if __name__ == "__main__":
    sys.exit(main())
