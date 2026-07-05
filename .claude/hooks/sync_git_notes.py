#!/usr/bin/env python3
"""Stop hook: 実装経緯 (jj change description) を git notes として記録する。

jj では change_id は不変だが、commit_id は `jj desc` / `jj sign` / rebase 等の
たびに書き換わる。git notes は commit_id にしか付けられないため、そのままでは
notes が古い (もはや到達不能な) commit にぶら下がったまま孤立してしまう。

そこで change_id と commit_id が常に 1:1 対応する性質を利用し、change_id を
キーにしたローカルの対応表 (.claude/state/git-notes-map.json) を介して commit_id
の変化を追跡する。commit_id が変わった change については、note を新しい
commit_id に付け直し、古い commit_id からは取り除く。これにより、jj の操作で
コミットが書き換えられても、実装経緯の説明 (description) が常に「今その
change を表しているコミット」に追従し続ける。

note の内容は change description そのもの (Conventional Commits + 経緯 + 実装内容)
をそのまま反映する。素の git log/show では見えにくい「なぜ」の説明を、
`git notes show <commit>` や `git log --notes=functus-rationale` からも
参照できるようにすることが目的。
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile

NOTES_REF = "refs/notes/functus-rationale"
STATE_REL_PATH = os.path.join(".claude", "state", "git-notes-map.json")

RECORD_SEP = "\x1e"
FIELD_SEP = "\x1f"


def run(root: str, args: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(
        args, cwd=root, capture_output=True, text=True, check=False
    )


def load_state(root: str) -> dict:
    path = os.path.join(root, STATE_REL_PATH)
    if not os.path.isfile(path):
        return {}
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except (json.JSONDecodeError, OSError):
        return {}


def save_state(root: str, state: dict) -> None:
    path = os.path.join(root, STATE_REL_PATH)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(state, f, ensure_ascii=False, indent=2, sort_keys=True)
        f.write("\n")


def list_described_mutable_changes(root: str) -> list[tuple[str, str]]:
    """記述済み (description が空でない) かつ mutable (書き換えられうる) な
    change の (change_id, commit_id) 一覧を返す。immutable になった change は
    commit_id がもう変わらないため対象から外し、同期コストを抑える。
    """
    template = f'change_id ++ "{FIELD_SEP}" ++ commit_id ++ "{RECORD_SEP}"'
    result = run(
        root,
        [
            "jj",
            "log",
            "-r",
            'mutable() ~ description(exact:"")',
            "--no-graph",
            "-T",
            template,
        ],
    )
    if result.returncode != 0:
        return []

    pairs = []
    for record in result.stdout.split(RECORD_SEP):
        record = record.strip()
        if not record:
            continue
        parts = record.split(FIELD_SEP)
        if len(parts) != 2:
            continue
        pairs.append((parts[0], parts[1]))
    return pairs


def get_description(root: str, change_id: str) -> str:
    result = run(
        root,
        ["jj", "log", "-r", change_id, "--no-graph", "-T", "description"],
    )
    return result.stdout


def sync_note(root: str, commit_id: str, note_text: str) -> None:
    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".txt", delete=False, encoding="utf-8"
    ) as tmp:
        tmp.write(note_text)
        tmp_path = tmp.name
    try:
        run(
            root,
            [
                "git",
                "notes",
                f"--ref={NOTES_REF}",
                "add",
                "-f",
                "-F",
                tmp_path,
                commit_id,
            ],
        )
    finally:
        os.unlink(tmp_path)


def remove_note(root: str, commit_id: str) -> None:
    run(
        root,
        ["git", "notes", f"--ref={NOTES_REF}", "remove", "--ignore-missing", commit_id],
    )


def main() -> int:
    raw = sys.stdin.read()
    try:
        payload = json.loads(raw) if raw else {}
    except json.JSONDecodeError:
        payload = {}

    if payload.get("stop_hook_active", False):
        return 0

    root = os.environ.get("CLAUDE_PROJECT_DIR", os.getcwd())
    if not shutil.which("jj") or not shutil.which("git"):
        return 0
    if not os.path.isdir(os.path.join(root, ".jj")):
        return 0
    if not os.path.isdir(os.path.join(root, ".git")):
        return 0

    state = load_state(root)
    changed = False

    for change_id, commit_id in list_described_mutable_changes(root):
        prev = state.get(change_id)
        if prev is not None and prev.get("commit_id") == commit_id:
            continue  # commit_id が前回と同じ = すでに正しい commit に note 済み

        note_text = get_description(root, change_id)
        sync_note(root, commit_id, note_text)

        if prev is not None and prev.get("commit_id") not in (None, commit_id):
            remove_note(root, prev["commit_id"])

        state[change_id] = {"commit_id": commit_id}
        changed = True

    if changed:
        save_state(root, state)

    return 0


if __name__ == "__main__":
    sys.exit(main())
