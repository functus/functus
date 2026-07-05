#!/bin/bash
# Stop hook: TDD ガード。
# 1) src/ の Rust コードが変更されているのにテストの追加・変更が無い場合は停止をブロック
# 2) cargo test が失敗する (Red のまま) 状態での停止もブロック
# 例外: change description に [skip-tdd] が含まれる場合はスキップ。
set -u

INPUT=$(cat)
STOP_ACTIVE=$(echo "$INPUT" | jq -r '.stop_hook_active // false')
[[ "$STOP_ACTIVE" == "true" ]] && exit 0

ROOT="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$ROOT" || exit 0
command -v jj >/dev/null 2>&1 || exit 0
[[ -d .jj ]] || exit 0

# escape hatch
DESC=$(jj log -r @ --no-graph -T 'description' 2>/dev/null)
echo "$DESC" | grep -q '\[skip-tdd\]' && exit 0

FILES=$(jj diff --name-only 2>/dev/null)
[[ -z "$FILES" ]] && exit 0

# src/ 配下の Rust コード変更を検出
SRC_CHANGED=$(echo "$FILES" | grep -E '(^|/)src/.*\.rs$' || true)
[[ -z "$SRC_CHANGED" ]] && exit 0

# テストの追加・変更があるか: tests/ 配下のファイル、または diff に #[test] / mod tests の追加
HAS_TEST_FILE=$(echo "$FILES" | grep -E '(^|/)tests/.*\.rs$' || true)
HAS_TEST_DIFF=$(jj diff --git 2>/dev/null | grep -E '^\+.*(#\[(tokio::|rs)?test\]|#\[cfg\(test\)\]|proptest!)' || true)

if [[ -z "$HAS_TEST_FILE" && -z "$HAS_TEST_DIFF" ]]; then
  echo "TDD 違反: src/ の Rust コードが変更されていますが、テストの追加・変更がありません。" >&2
  echo "Red → Green → Refactor に従ってください:" >&2
  echo "  1. 期待する振る舞いを表すテストを書き、意図した理由で失敗することを確認する" >&2
  echo "  2. テストを通す最小限の実装を書く" >&2
  echo "テストが原理的に不要な変更 (doc のみ・再エクスポートのみ等) の場合のみ、" >&2
  echo "description に [skip-tdd] を含めてください (.claude/rules/tdd.md 参照)。" >&2
  exit 2
fi

# cargo test が Red のままなら停止させない
if [[ -f Cargo.toml ]]; then
  TEST_OUT=$(cargo test --quiet 2>&1)
  if [[ $? -ne 0 ]]; then
    echo "cargo test が失敗しています (Red の状態)。Green まで完了させてから停止してください:" >&2
    echo "$TEST_OUT" | grep -E 'FAILED|panicked|error(\[|:)|test .* \.\.\. FAILED' | head -15 >&2
    exit 2
  fi
fi

exit 0
