#!/bin/bash
# PostToolUse (Edit|Write) hook: Rust ファイル編集後に fmt + clippy で品質を維持する。
# 問題があれば exit 2 で Claude にフィードバックし、その場で修正させる。
set -u

INPUT=$(cat)
FILE=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# Rust ファイル以外は対象外
[[ "$FILE" == *.rs ]] || exit 0

# Cargo プロジェクトがまだ無ければスキップ
ROOT="${CLAUDE_PROJECT_DIR:-$(pwd)}"
[[ -f "$ROOT/Cargo.toml" ]] || exit 0

cd "$ROOT" || exit 0

# 1) rustfmt: 自動整形(失敗しても続行)
cargo fmt --quiet 2>/dev/null

# 2) clippy: 警告をエラー扱いで検査
CLIPPY_OUT=$(cargo clippy --quiet --all-targets --message-format short -- -D warnings 2>&1)
if [[ $? -ne 0 ]]; then
  echo "clippy で問題が検出されました。修正してください:" >&2
  echo "$CLIPPY_OUT" | grep -E 'error|warning' | head -30 >&2
  exit 2
fi

exit 0
