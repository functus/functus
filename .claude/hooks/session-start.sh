#!/bin/bash
# SessionStart hook: セッション開始時に jj の現在状態をコンテキストへ注入する。
# レートリミット等で中断した後の新セッションでも、前回の作業状態
# (未記述の変更・直近の change・ブックマーク) から即座に再開できるようにする。
set -u

ROOT="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$ROOT" || exit 0
command -v jj >/dev/null 2>&1 || exit 0
[[ -d .jj ]] || exit 0

echo "## jj リポジトリ状態 (セッション開始時)"
echo '```'
jj st 2>/dev/null | head -20
echo '---'
jj log --limit 5 --no-graph -T 'change_id.short() ++ " " ++ if(bookmarks, bookmarks ++ " ", "") ++ if(description, description.first_line(), "(no description)") ++ "\n"' 2>/dev/null
echo '```'

STATE=$(jj log -r @ --no-graph -T 'if(empty, "empty", "dirty") ++ "|" ++ if(description, "described", "nodesc")' 2>/dev/null)
if [[ "$STATE" == "dirty|nodesc" ]]; then
  echo "⚠️ 前回セッションの未記述の変更が @ に残っています。内容を jj diff で確認し、作業を継続するか /jj-commit で確定してください。"
fi

if [[ -f Cargo.toml ]]; then
  echo "品質ゲート: cargo fmt --check / clippy -D warnings / cargo test を編集前に一度確認すると差分起因の失敗を切り分けやすい。"
fi

exit 0
