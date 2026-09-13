#!/usr/bin/env python3
"""PreToolUse (Bash) hook: このリポジトリは jj (Jujutsu) で管理しているため、
git commit / checkout 等の履歴改変系 git コマンドをブロックして jj へ誘導する。
"""
import json
import os
import re
import shlex
import sys

BLOCKED_SUBCOMMANDS = {
    "commit", "checkout", "switch", "rebase", "merge", "cherry-pick", "reset", "revert", "stash"
}
OPTIONS_WITH_VALUE = {"-C", "-c", "--git-dir", "--work-tree", "--namespace", "--exec-path"}

GUIDANCE = """このリポジトリは jj (Jujutsu) で管理しています。git の履歴操作は使わず、以下を使ってください:
  - 変更の記述: jj desc -m "type(scope): 説明"  (Conventional Commits + 経緯を本文に)
  - 新しい変更の開始 (マイクロコミット): jj new
  - ブランチ操作: jj bookmark / jj edit / jj split
詳細は /jj-commit スキルを参照してください。"""


def contains_blocked_git_command(command: str) -> bool:
    try:
        lexer = shlex.shlex(command, posix=True, punctuation_chars=";&|()")
        lexer.whitespace_split = True
        lexer.commenters = ""
        tokens = list(lexer)
    except ValueError:
        # 壊れた引用符のコマンドは Bash 自身が実行できない。
        return False

    variables: dict[str, str] = {}
    command_position = True
    index = 0
    while index < len(tokens):
        token = tokens[index]
        if token in {";", "&", "&&", "|", "||", "("}:
            command_position = True
            index += 1
            continue
        if token == ")":
            command_position = False
            index += 1
            continue
        if command_position and re.fullmatch(r"[A-Za-z_]\w*=.*", token):
            name, value = token.split("=", 1)
            variables[name] = value
            index += 1
            continue
        if not command_position:
            index += 1
            continue

        executable = variables.get(token[1:], token) if token.startswith("$") else token
        if executable in {"bash", "sh", "zsh"} and index + 2 < len(tokens) and tokens[index + 1] == "-c":
            if contains_blocked_git_command(tokens[index + 2]):
                return True
        if executable == "eval" and index + 1 < len(tokens):
            if contains_blocked_git_command(" ".join(tokens[index + 1:])):
                return True
        if os.path.basename(executable) != "git":
            command_position = False
            index += 1
            continue

        cursor = index + 1
        while cursor < len(tokens):
            candidate = tokens[cursor]
            if candidate == "--":
                cursor += 1
                break
            option = candidate.split("=", 1)[0]
            if not candidate.startswith("-"):
                break
            cursor += 1
            if option in OPTIONS_WITH_VALUE and "=" not in candidate:
                cursor += 1
        if cursor < len(tokens):
            candidate = tokens[cursor]
            candidate = variables.get(candidate[1:], candidate) if candidate.startswith("$") else candidate
        if cursor < len(tokens) and candidate in BLOCKED_SUBCOMMANDS:
            return True
        command_position = False
        index = cursor
    return False


def main() -> int:
    raw = sys.stdin.read()
    try:
        payload = json.loads(raw) if raw else {}
    except json.JSONDecodeError:
        # 入力が解析できない場合は安全側(ブロック)には倒さず、判断できないので許可する。
        # 実行対象コマンドが不明なままブロックすると通常の Bash 操作まで止めてしまうため。
        return 0

    command = payload.get("tool_input", {}).get("command", "") or ""
    if contains_blocked_git_command(command):
        print(GUIDANCE, file=sys.stderr)
        return 2

    return 0


if __name__ == "__main__":
    sys.exit(main())
