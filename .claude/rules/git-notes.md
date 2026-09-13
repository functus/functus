<!-- paths は意図的に未設定: 特定のファイル種別ではなく git notes という機構そのものの
     説明であり、ファイル Glob でのスコープ対象がなく無条件読み込みのままにする -->

# git notes による実装経緯の記録

Change description(Conventional Commits + 「## 経緯」「## 実装内容」)は、jj の操作
(`jj desc` の上書き、`jj sign`、rebase 等)によってコミットが書き換えられても、
`git notes` を通じて常に「今そのコミットを表しているコミット」に追従する。

## 仕組み

- ノートは `refs/notes/functus-rationale` という note ref に記録される
- Stop hook (`sync_git_notes.py`) が、mutable (まだ書き換えられうる) かつ
  description が設定済みの change をすべて走査し、各 change の
  現在の description を、その change が **今** 指している commit_id に note として付与する
- jj は change_id が不変で commit_id だけが書き換わる。この 1:1 対応を利用し、
  ローカルの対応表 `.claude/state/git-notes-map.json` (git 管理外) で
  「前回どの commit_id に note を付けたか」を change_id ごとに記録する。
  commit_id が変わっていたら、新しい commit_id に note を付け直し、
  古い commit_id からは note を取り除く
- immutable になった change (push 済みで書き換えられなくなったもの) は
  commit_id がもう変わらないため、以降は同期対象から外れる(既に付いた note がそのまま有効)

## 参照方法

```bash
git notes --ref=functus-rationale show <commit>      # 特定コミットの経緯を見る
git log --notes=functus-rationale                    # ログに経緯を併記して見る
```

## 共有について

`refs/notes/functus-rationale` はデフォルトでは push/fetch されない。
チームで共有したい場合は明示的に:

```bash
git push origin refs/notes/functus-rationale
git fetch origin refs/notes/functus-rationale:refs/notes/functus-rationale
```

Stop hook は自動では push しない(push はユーザーが明示的に判断する操作のため)。

## この仕組みに触れる際の注意

- `.claude/state/git-notes-map.json` を手動で編集・削除しない
  (削除すると次回同期時に古い commit へ note が残存したままになりうる。実害はないが孤立ノートが増える)
- note の内容を description と別物にしたい場合は、`sync_git_notes.py` の
  `get_description` を差し替える(現状は description をそのまま note にしている)
