# drive/codex.md — Codex CLI で lane driver を動かす時の差分

Codex CLI session が `/drive` の lane driver を務める時 (PM 872 / 873) の、**道具の差分だけ**を置く。手順と規則の正は
`/drive` (`.claude/skills/drive/SKILL.md`) と repo `CLAUDE.md` のままで、本 file は書き写さない。読む順は
repo `CLAUDE.md` → `/drive` → 本 file (PM の kickoff brief が指示する)。食い違いを見つけたら PM に報告する。

名前の出所: `merge-pr` / git hook / go token の名前は CI/tools の P1 plan memo
(`docs/plans/2026-11-agent-neutral-git-guards-p1a.md` / `-p1b.md`、branch `ci-agent-neutral-guards-p1` @ `75c00109`、**main に無い、
plan-review 中で未 ratify** — PM 893 / 908) から取った。ratify・着地で変わったら本 file を直す。下の「P1a / P1b 着地まで」の行は P1a / P1b の PR が外す (PM 935)。

**lane dir** = `$PMD/handoff/codex-lanes/<lane>/`。下の表の file (lane SSoT、gate file) はすべてここに置く。

## 1. 道具の対応表

| `/drive` が使う Claude の道具・段 | Codex driver の手段 |
|---|---|
| `SendMessage` で PM へ (`/drive` §2、§3 の 4、§6 の 1、§7) | Orca orchestration: driver → PM は `orca orchestration ask`、PM → driver は `send` / `reply`、作業単位の完了の報告は `worker_done` (PM 913)。成果物は lane dir の file のままで、message は file を指す。lane driver の sandbox の設定 (workspace-write + add-dir) のまま届くかは、pilot の kickoff で確かめる (PM 913)。`orca orchestration` の allow rule が前提 (PM 920) |
| `ListAgents` で PM を確かめる (`/drive` §1 の 3、§2)、restart 後の session 名 (`/drive` §9 の最後) | kickoff と restart の後に、nonce 付きの `ask` / `reply` を 1 往復させる (PM 913)。restart の後はこの往復で新しい宛先 (Orca の dispatch) を PM に知らせる。返事が無ければ止まる (`/drive` §10 の `止まる理由`) |
| harness の許可ダイアログ (`/drive` §2) | Codex の承認プロンプト。扱いは `/drive` §2 と同じ |
| lens の記録・`AskUserQuestion` (`~/.claude/CLAUDE.md`「判断と確認」) | 使わない。user への問いは無く、問いは PM 宛ての message に lens block を書く (`/drive` §2「聞く前に」) |
| subagent (`~/.claude/CLAUDE.md`「サブエージェント振り分け」) | 無し。driver が自分で作業する。review の fan-out は下の helper gate に回す。`/drive` §9 の項目 5 の「agent」は背景 run だけになり、項目 7 の「agent の昇格」は該当しない |
| `/pre-push` (`/drive` §3 の 1) | Stage 1–2 は driver が自分で回す (判定は `/drive` §3 の 2)。Stage 3–5 は **helper gate** (PM 873 (7)): driver は commit して PM に「gate 依頼 head `<sha>`」を送り、結果 (helper の動きは `/pm` §2) を lane dir の `gate-<sha>.md` で受ける。`/simplify` の patch を当て、`/pre-push`「Fix → re-verify」に従い、finding ごとの disposition を同じ file に書く。gate を依頼し直すのは `/simplify` の patch か Stage 4 の fix を当てた後だけで (PM 935)、ほかの fix は `/pre-push`「Fix → re-verify」のまま (PM 934。helper の動きは `/pm` §2、PM の確認は `/pm` §4)。Skip-OK は `/pre-push` のままで、その時は gate を依頼せず、skip の理由を `/drive` §3 の 4 の依頼に書く |
| `/elidex-plan-review` (repo `CLAUDE.md`「Edge-dense work」) | helper gate と同じ流れ (`/pm` §2)。driver は finding ごとの fix と disposition を書く。advisory の下見 (`/pm` §2、PM 948) が届いた時も、同じく fix と disposition を書く |
| `/external-review` / `/external-converge` (`/drive` §5) | post-push の Codex の verdict は PM から届く (`/pm` §2)。driver は fix を当てて disposition を書く。`@codex review` は `/drive` §5 のまま |
| `ScheduleWakeup` / 自分での poll (`/drive` §5、§9) | 自分で wake できない mode: `/drive` §5 の最後の bullet |
| Claude の PreToolUse / Stop の guard (`~/.claude/settings.json`) | **P1a 着地まで**: Codex の `hooks.json` から Claude の global hook script をそのまま呼ぶ (Bash の PreToolUse の guard、PM 911)。Stop gate は Codex の adapter (`~/.codex/hooks/announced-next-step-codex-adapter.py`) を通す (P2、PM 912)。amend / force-push の deny は global の script に切り出して Claude と Codex の両方から呼ぶ (user の diff 承認の後、PM 912)。P1a の git hook (`pre-push` / `pre-commit`) が入ったらそちらが正。`止まる理由` の規則は `/drive` §10 のまま |
| ECS-mission guard (Claude の編集 hook) | **P1a 着地まで**: Codex の編集 (apply_patch) は hook から見えない。PR body の ratchet の行 (`/drive` §4) と CI の `tools` job が担う (PM 911)。P1a の `pre-push` が入ったらそれも加わる |
| `git` / `gh` (fetch、`worktree add`、add、commit、push、`gh` の API) | workspace-write の sandbox は `.git` を書けず、ネットワークも前提にしない。走らせるのは execpolicy の allow rule に載った形だけ (PM 877 / 878、置き場は global の `~/.codex/rules`、一覧は P2 の下書き。適用は P2 の hooks と同じ user の diff 承認の後、PM 912)。PreToolUse の guard は allow rule より先に発火する (PM 912)。rule は argv の prefix で照合するので、`git -c k=v commit` / `git -C <dir> fetch` (`/drive` §1 の 1・5 の形) のように間に token を挟む形や `cd … &&` で始まる複合コマンドは外れ、network の無い sandbox の中で失敗する。git (push / commit / fetch / `merge origin/main` / `worktree add` 等) は plain な形で、workdir を指定した単独のコマンドで打つ (PM 920) (照合は `codex execpolicy check --rules <file> <argv…>` で事前に確かめられる) |
| merge (`/drive` §6 の 4) | **P1b 着地まで**: Codex driver は PR を merge しない (`gh pr merge` は PM、`/pm` §4。`/drive` §6 の 3 の main の取り込みは driver がする)。§6 の 1 の報告に `/drive` §7 の landing report の項目 (merge commit の sha を除く) を同梱する (PM 918)。P1b の後は `/drive` §6 に戻る (wrapper `merge-pr` と go token の形は P1b memo §4.4 / §7.2、PM 873 (6)) |
| memory の書き込み、lane SSoT (`/drive` §0、§1 の 2・6、§9) | `--add-dir` で書けるのは lane dir の 1 つだけ (`$MEM` は開かない)。lane SSoT は lane dir の `ssot.md` で、`/drive` §9 の 10 項目を書く。`$MEM` の lane の file の冒頭はそれを指す pointer だけ (`/pm` §9、置き場は 1 つのまま、PM 912) — `/drive` §1 の 2 では pointer を辿って `ssot.md` を読む。`.git` 以外の dir への add-dir は P2 で試験する (効かなければ PM 912 を見直す) |
| commit するブランチ用の worktree (`/drive` §1 の 9) | lane ごとに root dir を 1 つ置き、session は `-C <lane root>` で起動される (起動の形は `/pm` §9)。per-PR worktree はその下の subdir に `git worktree add` で作る (allow list に載る、PM 912)。lane root は elidex の guard の範囲 (`~/.claude/hooks/git-hook-bypass-guard.py` の `in_scope`) の中に置く |
| `mise run ci` (`/drive` §3、`/pre-push` Stage 2) | machine 全体で同時に 1 本で、排他の確認は `/drive` §8 (allow rule が前提、PM 920)。確認と CI は別々の単独コマンドで打つ。shell は login=false で走らせる (login shell は macOS の path_helper が PATH を並べ替え、`python3` が Xcode のものになる)。Stage 2 は 1 task ずつ `~/.codex/bin/ci-step <task> <log>` で走らせる (execpolicy の prefix rule と承認の回数のため、PM 947。global の wrapper で、受け付けるのは `[tasks.ci]` の task と `ci` だけ。log は `…/handoff/codex-lanes/` 配下の任意の `.log` で、wrapper の `case` の `*` が `/` にも一致するため別の lane の dir や深いパスも通る。`..` を含むパスは断る)。判定は `/drive` §3 の 2 |
| budget (`/drive` §10、`/pm` §6) | Codex lane は gate を待つ間も、gate の要らない仕事 (webref の確認、次の plan memo の草稿、次の spec group の cite の修正の commit、環境の調査) を続ける。push と実装は gate と PM の go を待つ (PM 945)。P5 の判定までは Codex 側の cap を置かない (PM 912 / 935)。machine の負荷 (local の `mise run ci` の同時 1 本は上の行) と disk の下限 (`/drive` §8) は効く |
