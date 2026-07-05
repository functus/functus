#!/bin/bash
# Stop hook: マイクロコミット運用のガード。
# 作業コピー (@) に変更があるのに description が無いまま停止しようとしたらブロックし、
# jj desc (Conventional Commits + 経緯) → jj new を促す。
set -u

INPUT=$(cat)

# 無限ループ防止: すでに Stop hook で継続中なら許可
STOP_ACTIVE=$(echo "$INPUT" | jq -r '.stop_hook_active // false')
[[ "$STOP_ACTIVE" == "true" ]] && exit 0

ROOT="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$ROOT" || exit 0
command -v jj >/dev/null 2>&1 || exit 0
[[ -d .jj ]] || exit 0

STATE=$(jj log -r @ --no-graph -T 'if(empty, "empty", "dirty") ++ "|" ++ if(description, "described", "nodesc")' 2>/dev/null)
[[ -z "$STATE" ]] && exit 0

if [[ "$STATE" == "dirty|nodesc" ]]; then
  echo "作業コピー (@) に未記述の変更があります。マイクロコミット運用に従ってください:" >&2
  echo "  1. jj diff で変更内容を確認する" >&2
  echo "  2. /jj-commit スキルに従い、Conventional Commits 形式 + 実装の経緯を含む description を jj desc で記述する" >&2
  echo "  3. jj new で次の変更を開始する" >&2
  exit 2
fi

exit 0
