#!/bin/bash
# PreToolUse (Bash) hook: このリポジトリは jj (Jujutsu) で管理しているため、
# git commit / git checkout 等の履歴改変系 git コマンドをブロックして jj へ誘導する。
set -u

INPUT=$(cat)
CMD=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

if echo "$CMD" | grep -qE '(^|[;&|[:space:]])git[[:space:]]+(commit|checkout|switch|rebase|merge|cherry-pick|reset|stash)([[:space:]]|$)'; then
  echo "このリポジトリは jj (Jujutsu) で管理しています。git の履歴操作は使わず、以下を使ってください:" >&2
  echo "  - 変更の記述: jj desc -m \"type(scope): 説明\"  (Conventional Commits + 経緯を本文に)" >&2
  echo "  - 新しい変更の開始 (マイクロコミット): jj new" >&2
  echo "  - ブランチ操作: jj bookmark / jj edit" >&2
  echo "詳細は /jj-commit スキルを参照してください。" >&2
  exit 2
fi

exit 0
