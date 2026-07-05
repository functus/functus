#!/bin/bash
# 汎用チェックポイント hook (PostToolUse / PreCompact / SessionEnd)。
# jj st を実行して作業コピーをスナップショットする。これにより全ての編集が
# jj の operation log (jj op log / jj evolog) に記録され、レートリミット等で
# セッションがどこで中断されても作業内容が失われない。
set -u

ROOT="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$ROOT" || exit 0
command -v jj >/dev/null 2>&1 || exit 0
[[ -d .jj ]] || exit 0

jj st > /dev/null 2>&1

exit 0
