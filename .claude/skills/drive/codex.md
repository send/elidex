# drive/codex.md — Codex CLI で lane driver を動かす時の差分

Codex CLI session が `/drive` の lane driver を務める時 (PM 872 / 873) の、**道具の差分だけ**を置く。手順と規則の正は
`/drive` (`.claude/skills/drive/SKILL.md`) と repo `CLAUDE.md` のままで、本 file は書き写さない。読む順は
repo `CLAUDE.md` → `/drive` → 本 file (PM の kickoff brief が指示する)。食い違いを見つけたら PM に報告する。

名前の出所: `merge-pr` / git hook / go token の名前は CI/tools の P1 plan memo
(`docs/plans/2026-11-agent-neutral-git-guards-p1a.md` / `-p1b.md`、branch `ci-agent-neutral-guards-p1` @ `d9627a36`、**main に無い、
plan-review 中で未 ratify** — PM 893 / 908) から取った。ratify・着地で変わったら本 file を直す。

**lane dir** = `$PMD/handoff/codex-lanes/<lane>/`。下の表の file (lane SSoT、gate file) はすべてここに置く。

## 1. 道具の対応表

| `/drive` が使う Claude の道具・段 | Codex driver の手段 |
|---|---|
| `SendMessage` で PM へ (`/drive` §2、§3 の 4、§6 の 1、§7) | Orca orchestration: driver → PM は `orca orchestration ask`、PM → driver は `send` / `reply`、作業単位の完了の報告は `worker_done` (PM 913)。成果物は lane dir の file のままで、message は file を指す。lane driver の sandbox の設定 (workspace-write + add-dir) のまま届くかは、pilot の kickoff で確かめる (PM 913)。`orca orchestration` の allow rule が前提 (PM 920) |
| `ListAgents` で PM を確かめる (`/drive` §1 の 3、§2)、restart 後の session 名 (`/drive` §9 の最後) | kickoff と restart の後に、nonce 付きの `ask` / `reply` を 1 往復させる (PM 913)。restart の後はこの往復で新しい宛先 (Orca の dispatch) を PM に知らせる。返事が無ければ止まる (`/drive` §10 の `止まる理由`) |
| harness の許可ダイアログ (`/drive` §2) | Codex の承認プロンプト。扱いは `/drive` §2 と同じ |
| lens の記録・`AskUserQuestion` (`~/.claude/CLAUDE.md`「判断と確認」) | 使わない。user への問いは無く、問いは PM 宛ての message に lens block を書く (`/drive` §2「聞く前に」) |
| subagent (`~/.claude/CLAUDE.md`「サブエージェント振り分け」) | 無し。driver が自分で作業する。review の fan-out は下の helper gate に回す。`/drive` §9 の項目 5・7 の「agent」は背景 run だけになる |
| `/pre-push` (`/drive` §3 の 1) | Stage 1–2 は driver が自分で回す (判定は `/drive` §3 の 2)。Stage 3–5 は **helper gate** (PM 873 (7)): driver は commit して PM に「gate 依頼 head `<sha>`」を送り、結果 (helper の動きは `/pm` §2) を lane dir の `gate-<sha>.md` で受ける。`/simplify` の patch を当て、`/pre-push`「Fix → re-verify」に従い、finding ごとの disposition を同じ file に書く。head が変われば新しい head で gate を依頼し直す (PM の確認は `/pm` §4)。Skip-OK は `/pre-push` のまま |
| `/elidex-plan-review` (repo `CLAUDE.md`「Edge-dense work」) | helper gate と同じ流れ (`/pm` §2)。driver は finding ごとの fix と disposition を書く |
| `/external-review` / `/external-converge` (`/drive` §5) | post-push の Codex の verdict は PM から届く (`/pm` §2)。driver は fix を当てて disposition を書く。`@codex review` は `/drive` §5 のまま |
| `ScheduleWakeup` / 自分での poll (`/drive` §5、§9) | 自分で wake できない mode: `/drive` §5 の最後の bullet |
| Claude の PreToolUse / Stop の guard (`~/.claude/settings.json`) | **P1a 着地まで**: Codex の `hooks.json` から Claude の global hook script をそのまま呼ぶ (Bash の PreToolUse の guard は PM 911、Stop gate は PM 912)。amend / force-push の deny は global の script に切り出して Claude と Codex の両方から呼ぶ (user の diff 承認の後、PM 912)。P1a の git hook (`pre-push` / `pre-commit`) が入ったらそちらが正。`止まる理由` の規則は `/drive` §10 のまま |
| ECS-mission guard (Claude の編集 hook) | **P1a 着地まで**: Codex の編集 (apply_patch) は hook から見えない。PR body の ratchet の行 (`/drive` §4) と CI の `tools` job が担う (PM 911)。P1a の `pre-push` が入ったらそれも加わる |
| `git` / `gh` (fetch、`worktree add`、add、commit、push、`gh` の API) | workspace-write の sandbox は `.git` を書けず、ネットワークも前提にしない。走らせるのは execpolicy の allow rule に載った形だけ (PM 877 / 878、置き場は global の `~/.codex/rules`、一覧は P2 の下書き。適用は P2 の hooks と同じ user の diff 承認の後、PM 912)。PreToolUse の guard は allow rule より先に発火する (PM 912)。rule は argv の prefix で照合するので、`git -c k=v commit` のように間に token を挟む形や `cd … &&` で始まる複合コマンドは外れ、network の無い sandbox の中で失敗する。git (push / commit / fetch / `worktree add`) は plain な形で、workdir を指定した単独のコマンドで打つ (PM 920) (照合は `codex execpolicy check --rules <file> <argv…>` で事前に確かめられる) |
| merge (`/drive` §6 の 4) | **P1b 着地まで**: Codex driver は merge しない (merge するのは PM、`/pm` §4)。§6 の 1 の報告に `/drive` §7 の landing report の項目 (merge commit の sha を除く) を同梱する (PM 918)。P1b の後は `/drive` §6 に戻る (wrapper `merge-pr` と go token の形は P1b memo §4.4 / §7.2、PM 873 (6)) |
| memory の書き込み、lane SSoT (`/drive` §0、§1 の 2・6、§9) | `--add-dir` で書けるのは lane dir の 1 つだけ (`$MEM` は開かない)。lane SSoT は lane dir の `ssot.md` で、`/drive` §9 の 10 項目を書く。`$MEM` の lane の file の冒頭はそれを指す pointer だけ (`/pm` §9、置き場は 1 つのまま、PM 912) — `/drive` §1 の 2 では pointer を辿って `ssot.md` を読む。`.git` 以外の dir への add-dir は P2 で試験する (効かなければ PM 912 を見直す) |
| commit するブランチ用の worktree (`/drive` §1 の 9) | lane ごとに root dir を 1 つ置き、session は `codex -C <lane root>` で起動する。per-PR worktree はその下の subdir に `git worktree add` で作る (allow list に載る、PM 912)。lane root は elidex の guard の範囲 (`~/.claude/hooks/git-hook-bypass-guard.py` の `in_scope`) の中に置く |
| budget (`/drive` §10、`/pm` §6) | pilot の間は Codex 側の cap を置かない (PM 912)。machine の負荷 (local の `mise run ci` は machine 全体で同時に 1 本、PM 912。排他の確認の `pgrep` は allow rule が前提、PM 920) と disk の下限 (`/drive` §8) は効く |
