#!/usr/bin/env python3
"""PostToolUse (Bash) hook: `jj git push` で Open な PR のブックマークを
更新したときに、PR 説明文を見直すべきかどうかの検討を促すリマインダー。

.claude/rules/code-review-response.md の「PR 説明文を追随して更新する」を
補完するもの。説明文が実際に乖離しているかどうかは意味的な判断が必要で
機械的に断定できないため、ここでは判定・強制はせず、push のたびに
「検討することを思い出させる」だけに留める(ブロックしない)。
"""
import json
import re
import shutil
import subprocess
import sys

BOOKMARK_FLAG = re.compile(r"--bookmark[= ]+(\S+)")


def run(args: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(args, capture_output=True, text=True, check=False)


def main() -> int:
    raw = sys.stdin.read()
    try:
        payload = json.loads(raw) if raw else {}
    except json.JSONDecodeError:
        return 0

    command = payload.get("tool_input", {}).get("command", "") or ""
    if "jj git push" not in command:
        return 0

    if not shutil.which("gh"):
        return 0

    bookmarks = BOOKMARK_FLAG.findall(command)
    if not bookmarks:
        return 0

    open_prs = []
    for bookmark in bookmarks:
        result = run(
            [
                "gh",
                "pr",
                "list",
                "--head",
                bookmark,
                "--state",
                "open",
                "--json",
                "number,url,title",
            ]
        )
        if result.returncode != 0 or not result.stdout.strip():
            continue
        try:
            prs = json.loads(result.stdout)
        except json.JSONDecodeError:
            continue
        for pr in prs:
            open_prs.append((bookmark, pr))

    if not open_prs:
        return 0

    lines = [
        "この push は Open な PR のブックマークを更新しました。"
        "PR 説明文を見直すべきか検討してください"
        "(.claude/rules/code-review-response.md の「PR 説明文を追随して更新する」参照):"
    ]
    for bookmark, pr in open_prs:
        lines.append(f"  - #{pr['number']} ({bookmark}): {pr['title']} — {pr['url']}")

    print(
        json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "PostToolUse",
                    "additionalContext": "\n".join(lines),
                }
            }
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
