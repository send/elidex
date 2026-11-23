# drive/codex.md — Codex CLI で lane driver を動かす時の差分

Codex CLI session が `/drive` の lane driver を務める時 (PM 872 / 873) の、**道具の差分だけ**を置く。手順と規則の正は
`/drive` (`.claude/skills/drive/SKILL.md`) と repo `CLAUDE.md` のままで、本 file は書き写さない。読む順は
repo `CLAUDE.md` → `/drive` → 本 file (PM の kickoff brief が指示する)。食い違いを見つけたら PM に報告する。

名前の出所: git hook の名前は CI/tools の P1 plan memo
(`docs/plans/2026-11-agent-neutral-git-guards-p1a.md`、branch `ci-agent-neutral-guards-p1` @ `75c00109`、**main に無い、
plan-review 中で未 ratify** — PM 893 / 908) から取った。ratify・着地で変わったら本 file を直す。下の「P1a 着地まで」の行は P1a の PR が外す (PM 935)。merge は P1b の `merge-pr` / go token でなく、global の `~/.codex/bin/pr-merge` (PM 1003)。

**lane dir** = `$PMD/handoff/codex-lanes/<lane>/`。下の表の file (lane SSoT、gate file) はすべてここに置く。

## 1. 道具の対応表

| `/drive` が使う Claude の道具・段 | Codex driver の手段 |
|---|---|
| `SendMessage` で PM へ (`/drive` §2、§3 の 4、§6 の 1、§7) | Orca orchestration: driver → PM は `orca orchestration ask`、PM → driver は `send` / `reply`、作業単位の完了の報告は `worker_done` (PM 913)。成果物は lane dir の file のままで、message は file を指す。lane driver の sandbox の設定 (workspace-write + add-dir) のまま届くかは、pilot の kickoff で確かめる (PM 913)。`orca orchestration` の allow rule が前提 (PM 920) |
| `ListAgents` で PM を確かめる (`/drive` §1 の 3、§2)、restart 後の session 名 (`/drive` §9 の最後) | kickoff と restart の後に、nonce 付きの `ask` / `reply` を 1 往復させる (PM 913)。restart の後はこの往復で新しい宛先 (Orca の dispatch) を PM に知らせる。返事が無ければ止まる (`/drive` §10 の `止まる理由`) |
| harness の許可ダイアログ (`/drive` §2) | Codex の承認プロンプト。扱いは `/drive` §2 と同じ |
| lens の記録・`AskUserQuestion` (`~/.claude/CLAUDE.md`「判断と確認」) | 使わない。user への問いは無く、問いは PM 宛ての message に lens block を書く (`/drive` §2「聞く前に」) |
| subagent (`~/.claude/CLAUDE.md`「サブエージェント振り分け」) | 無し。driver が自分で作業する。review の fan-out は下の独立 Codex session の gate に回す (PM 994)。`/drive` §9 の項目 5 の「agent」は背景 run だけになり、項目 7 の「agent の昇格」は該当しない |
| `/pre-push` (`/drive` §3 の 1) | Stage 1–2 は driver が自分で回す (判定は `/drive` §3 の 2)。Stage 3–5 は **独立 Codex session の gate** (PM 873 (7) → PM 994 で Claude helper から替わった): driver は commit して PM に「gate 依頼 head `<sha>`」を送る。PM が新しい Codex session を起こし、その session が `/pre-push` の Stage 3–5 を自分で回して、結果を lane dir の `gate-<sha>.md` に書く (段の区切りと再依頼の範囲の正は `/pm` §2 の gate 依頼の項。起こす主体だけ PM 994)。driver はそれを受ける。`/simplify` の patch を当て、`/pre-push`「Fix → re-verify」に従い、finding ごとの disposition を同じ file に書く。gate を依頼し直すのは `/simplify` の patch か Stage 4 の fix を当てた後だけで (PM 935)、ほかの fix は `/pre-push`「Fix → re-verify」のまま (PM 934。PM の確認は `/pm` §4)。Skip-OK は `/pre-push` のままで、その時は gate を依頼せず、skip の理由を `/drive` §3 の 4 の依頼に書く |
| `/elidex-plan-review` (repo `CLAUDE.md`「Edge-dense work」) | 独立 Codex session の gate と同じ流れ (binding も advisory な下見も、PM 994)。手順は skill file のまま、gate file は lane dir。範囲を広げる指摘の扱いは `/drive` §5 (PM 959 (4)) |
| `/external-review` / `/external-converge` (`/drive` §5) | post-push の Codex の verdict は PM から届く (`/pm` §2)。driver は fix を当てて disposition を書く。`@codex review` は `/drive` §5 のまま |
| `ScheduleWakeup` / 自分での poll (`/drive` §5、§9) | 自分で wake できない mode: `/drive` §5 の最後の bullet |
| Claude の PreToolUse / Stop の guard (`~/.claude/settings.json`) | **P1a 着地まで**: Codex の `hooks.json` から Claude の global hook script をそのまま呼ぶ (Bash の PreToolUse の guard、PM 911)。Stop gate は Codex の adapter (`~/.codex/hooks/announced-next-step-codex-adapter.py`) を通す (P2、PM 912)。amend / force-push の deny は global の script に切り出して Claude と Codex の両方から呼ぶ (user の diff 承認の後、PM 912)。P1a の git hook (`pre-push` / `pre-commit`) が入ったらそちらが正。`止まる理由` の規則は `/drive` §10 のまま |
| ECS-mission guard (Claude の編集 hook) | **P1a 着地まで**: Codex の編集 (apply_patch) は hook から見えない。PR body の ratchet の行 (`/drive` §4) と CI の `tools` job が担う (PM 911)。P1a の `pre-push` が入ったらそれも加わる |
| `git` / `gh` (fetch、`worktree add`、add、commit、push、`gh` の API) | workspace-write の sandbox は `.git` を書けず、ネットワークも前提にしない。走らせるのは execpolicy の allow rule に載った形だけ (PM 877 / 878、置き場は global の `~/.codex/rules`、一覧は P2 の下書き。適用は P2 の hooks と同じ user の diff 承認の後、PM 912)。PreToolUse の guard は allow rule より先に発火する (PM 912)。rule は argv の prefix で照合するので、`git -c k=v commit` / `git -C <dir> fetch` (`/drive` §1 の 1・5 の形) のように間に token を挟む形や `cd … &&` で始まる複合コマンドは外れ、network の無い sandbox の中で失敗する。git (push / commit / fetch / `merge origin/main` / `worktree add` 等) は plain な形で、workdir を指定した単独のコマンドで打つ (PM 920) (照合は `codex execpolicy check --rules <file> <argv…>` で事前に確かめられる) |
| merge (`/drive` §6 の 3・4) | merge は `~/.codex/bin/pr-merge <PR> <40-hex head>` だけ (global の wrapper、allow rule が前提、PM 1003)。script は OPEN・非 draft・CLEAN・head 一致、CI 全 check が成功系、最新の Codex review が head で findings 0、unresolved thread 0 のどれかが外れると exit 1 で merge しない (`--squash --match-head-commit`、`--auto` なし、`--dry-run` あり)。**exit 1 は PM に上げ、`gh pr merge` に戻らない**。allow rule は session 起動時に読まれるので、rule より古い session は 1 回 prompt になる (user が承認する、PM 1003 addendum)。いつ merge してよいかは lane で違う: CS は下の §3 (PM 996)、L1 (plan memo) は今までどおり PM の `merge at head <sha>` を待ってから打つ (`/drive` §6)。`/drive` §6 の 3 の main の取り込みは driver がする。§6 の 5 の landing report は merge の後に送る |
| memory の書き込み、lane SSoT (`/drive` §0、§1 の 2・6、§9) | `--add-dir` で書けるのは lane dir の 1 つだけ (`$MEM` は開かない)。lane SSoT は lane dir の `ssot.md` で、`/drive` §9 の 10 項目を書く。`$MEM` の lane の file の冒頭はそれを指す pointer だけ (`/pm` §9、置き場は 1 つのまま、PM 912) — `/drive` §1 の 2 では pointer を辿って `ssot.md` を読む。`.git` 以外の dir への add-dir は P2 で試験する (効かなければ PM 912 を見直す) |
| commit するブランチ用の worktree (`/drive` §1 の 9) | lane ごとに root dir を 1 つ置き、session は `-C <lane root>` で起動される (起動の形は `/pm` §9)。per-PR worktree はその下の subdir に `git worktree add` で作る (allow list に載る、PM 912)。lane root は elidex の guard の範囲 (`~/.claude/hooks/git-hook-bypass-guard.py` の `in_scope`) の中に置く |
| `mise run ci` (`/drive` §3、`/pre-push` Stage 2) | machine 全体で同時に 1 本で、排他の確認は `/drive` §8 (allow rule が前提、PM 920)。確認と CI は別々の単独コマンドで打つ。shell は login=false で走らせる (login shell は macOS の path_helper が PATH を並べ替え、`python3` が Xcode のものになる)。Stage 2 は 1 task ずつ `~/.codex/bin/ci-step <task> <log>` で走らせる (execpolicy の prefix rule と承認の回数のため、PM 947。global の wrapper で、受け付けるのは `[tasks.ci]` の task と `ci` だけ。log は `…/handoff/codex-lanes/<lane>/<name>.log` の 1 段だけで、深い dir・lane の dir が無いもの・`..`・`.log` でないものは断る (PM 974。呼び出し元の lane は見ないので、別の lane の dir にも書ける))。判定は `/drive` §3 の 2 |
| budget (`/drive` §10、`/pm` §6) | Codex lane は gate を待つ間も、gate の要らない仕事 (webref の確認、次の plan memo の草稿、次の spec group の cite の修正の commit、環境の調査) を続ける。push と実装は gate と PM の go を待つ (PM 945)。ただし CS は clean な gate の後は PM の push go を待たない (下の §3)。P5 の判定までは Codex 側の cap を置かない (PM 912 / 935)。machine の負荷 (local の `mise run ci` の同時 1 本は上の行) と disk の下限 (`/drive` §8) は効く |

## 2. 作業単位 = 1 goal (PM 1023)

- 作業単位の頭で `create_goal` を呼ぶ。objective の完了条件は「PM への Orca orchestration の報告 (`ask` か `worker_done`) を送ったこと」。
- 待ち (CI、gate、CI の slot) で turn を終えない。同じ単位かその次の分類を続ける。報告の後に `update_goal(status: complete)` を呼び、次の単位は PM が送る。
- Orca の spec の先頭の `/goal` は goal mode を始めない (11-24 に測定)。始めるのは tool call。`paused` / `blocked` は tool の説明が許す時だけ。

## 3. CS lane の comment-only PR (PM 996 / 997)

CS は comment だけの PR を自分で検証して merge する (merge の打ち方は §1 の merge 行)。L1 (plan memo) は対象外で、PM の `merge at head <sha>` のまま。

| 項目 | 内容 |
|---|---|
| merge 直前の checklist (1 PR 1 行、`handoff/codex-lanes/cs/` の log に残す) | head == 独立 gate が clean とした sha / `mergeStateStatus` CLEAN (`DIRTY` / `BEHIND` は main を merge で取り込み Stage 2 をやり直す、rebase・force-push しない) / current head の全 check が pass / Codex が 3 channel (review body、inline、issue comment) で current head を評価して findings 0 (silent 👍 は verdict でない) / 変更が comment 行だけ (markdown の cite 文は comment に数える。code・config・test の期待値は数えない。PM が PR ごとに承認した例外だけ別) / PR body に ratchet の行 |
| PM に上げる | check の失敗、Codex の findings、comment 以外の変更 |
| push | gate が clean なら PM の push go を待たずに push する |
| gate が MIN だけ | fix せず進め、MIN は CS の inventory に記録して同じ spec group の次の batch で直す |
| `@codex review` | head が動いて verdict が古い head のものだけの時、head ごとに 1 回。2 回目は PM |
| sibling の取りこぼし | 絞った再確認で IMP 以上の新しい sibling が出たら、直すのをやめて該当 commit を PR から外し、clean な commit を出す |
| 外した commit を出す時 (PM 996 addendum) | 外した commit を出す時と clean な commit だけを出す時は、origin/main から作った branch に cherry-pick し、各 commit の `git patch-id --stable` が gate で見た commit と一致することを log に残せば gate をやり直さない。一致しなければ PM |
| batch の大きさ (試行、数 PR の後に見直す) | 1 PR に spec group 4–6 個 (comment 約 60 行、同じ spec 文書を優先)、group ごとに 1 commit。Stage 2 と独立 gate は PR ごとに 1 回で、gate は commit ごとに verdict を出し、IMP の commit は直すか次の PR に回す。semantics の行と dependency の更新は別 PR |
