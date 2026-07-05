#!/usr/bin/env python3
"""PreToolUse (Bash) hook: このリポジトリは jj (Jujutsu) で管理しているため、
git commit / checkout 等の履歴改変系 git コマンドをブロックして jj へ誘導する。
"""
import json
import re
import sys

# git サブコマンドの前に `-C <dir>` や `-c key=val` 等のグローバルオプションが
# 挟まるケースも許容するため、`git` の後ろは任意個数のオプション+引数を許す。
BLOCKED_SUBCOMMANDS = r"(commit|checkout|switch|rebase|merge|cherry-pick|reset|stash)"
GIT_OPTION = r"(?:-[A-Za-z]|--[A-Za-z][A-Za-z-]*)(?:[=\s]\S+)?"
PATTERN = re.compile(
    r"(^|[;&|]|\s)git(?:\s+" + GIT_OPTION + r")*\s+" + BLOCKED_SUBCOMMANDS + r"(\s|$)"
)

GUIDANCE = """このリポジトリは jj (Jujutsu) で管理しています。git の履歴操作は使わず、以下を使ってください:
  - 変更の記述: jj desc -m "type(scope): 説明"  (Conventional Commits + 経緯を本文に)
  - 新しい変更の開始 (マイクロコミット): jj new
  - ブランチ操作: jj bookmark / jj edit / jj split
詳細は /jj-commit スキルを参照してください。"""


def main() -> int:
    raw = sys.stdin.read()
    try:
        payload = json.loads(raw) if raw else {}
    except json.JSONDecodeError:
        # 入力が解析できない場合は安全側(ブロック)には倒さず、判断できないので許可する。
        # 実行対象コマンドが不明なままブロックすると通常の Bash 操作まで止めてしまうため。
        return 0

    command = payload.get("tool_input", {}).get("command", "") or ""
    if PATTERN.search(command):
        print(GUIDANCE, file=sys.stderr)
        return 2

    return 0


if __name__ == "__main__":
    sys.exit(main())
