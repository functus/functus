#!/usr/bin/env python3
"""汎用チェックポイント hook (PostToolUse / PreCompact / SessionEnd)。

jj st を実行して作業コピーをスナップショットする。これにより全ての編集が
jj の operation log (jj op log / jj evolog) に記録され、レートリミット等で
セッションがどこで中断されても作業内容が失われない。
"""
import os
import shutil
import subprocess
import sys


def main() -> int:
    root = os.environ.get("CLAUDE_PROJECT_DIR", os.getcwd())
    if not shutil.which("jj") or not os.path.isdir(os.path.join(root, ".jj")):
        return 0

    subprocess.run(["jj", "st"], cwd=root, capture_output=True, check=False)
    return 0


if __name__ == "__main__":
    sys.exit(main())
