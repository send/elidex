# drive/codex.md — Codex CLI で lane driver を動かす時の差分

Codex CLI session が `/drive` の lane driver を務める時 (PM 872 / 873) の、**道具の差分だけ**を置く。手順と規則の正は
`/drive` (`.claude/skills/drive/SKILL.md`) と repo `CLAUDE.md` のままで、本 file は書き写さない。読む順は
repo `CLAUDE.md` → `/drive` → 本 file (PM の kickoff brief が指示する、`/pm` §9)。本 file と `/drive` が食い違ったら
`/drive` が正で、食い違いを PM に報告する。

名前の出所: `merge-pr` / git hook / go token の名前は CI/tools の P1 plan memo
(`docs/plans/2026-11-agent-neutral-git-guards-p1.md`、branch `ci-agent-neutral-guards-p1` @ `8a30239d`、**main に無い、
plan-review 中で未 ratify**、P1a / P1b に分割中 — PM 893 / 908) から取った。ratify・着地で変わったら本 file を直す。

## 1. 道具の対応表

| `/drive` が使う Claude の道具・段 | Codex driver の手段 | 状態 |
|---|---|---|
| `SendMessage` で PM へ (`/drive` §2、§3 の 4、§6 の 1、§7) | file mailbox `$PMD/handoff/codex-lanes/<lane>/to-pm.md` に追記 (各 message は `## <連番> <JST 時刻> <from>` + 本文)。PM からは `to-driver.md` に追記され、合図は terminal への打鍵で来る (PM 873 (8))。返事の正は file 側 | mailbox が正 (PM 911)。Orca orchestration の到達は未検証 (pilot の kickoff で PM が確かめる、PM 877) |
| `ListAgents` で PM を確かめる (`/drive` §1 の 3、§2) | kickoff と restart の後に PM と nonce を往復させる (PM が nonce を `to-driver.md` に書き、driver が `to-pm.md` に同じ nonce を返す)。返事が無ければ止まる (`/drive` §10 の `止まる理由`) | — |
| harness の許可ダイアログ (`/drive` §2) | Codex の承認プロンプト。扱いは `/drive` §2 と同じ | — |
| lens の記録・`AskUserQuestion` (`~/.claude/CLAUDE.md`「判断と確認」) | 使わない。user への問いは無く、問いは PM 宛ての message に lens block を書く (`/drive` §2「聞く前に」) | — |
| subagent (`~/.claude/CLAUDE.md`「サブエージェント振り分け」) | 無し。driver が自分で作業する。review の fan-out は下の helper gate に回す。`/drive` §9 の項目 5・7 の「agent」は背景 run だけになる | — |
| `/pre-push` Stage 1–2 (`/drive` §3 の 1–2) | driver が自分で走らせる。CI の判定は `/drive` §3 の 2 | — |
| `/pre-push` Stage 3–5 (`/simplify` / `/code-review` / `/elidex-review`) | **helper gate** (PM 873 (7))。driver は Stage 1–2 を通して commit し、`to-pm.md` に「gate 依頼 head `<sha>`」を書く。PM が起こす Claude helper が committed head で Stage 3–5 を回し、結果を `$PMD/handoff/codex-lanes/<lane>/gate-<sha>.md` に書く (`/simplify` の変更は patch として)。driver は patch を当て、`/pre-push`「Fix → re-verify」に従い、finding ごとの disposition を同じ file に書く。head が変われば新しい head で gate を依頼し直す (PM は gate file の head == merge する head を `/pm` §4 で確かめる)。Skip-OK は `/pre-push` のまま | — |
| `/elidex-plan-review` (repo `CLAUDE.md`「Edge-dense work」) | helper gate と同じ流れで Claude helper が回す (PM 912)。driver は finding ごとの fix と disposition を書く | — |
| `/external-review` / `/external-converge` (`/drive` §5) | post-push の Codex の verdict は PM が読み、mailbox で渡す (自分で wake できない mode と同じ扱い、PM 361 / 912)。driver は fix を当てて disposition を書く。`@codex review` を撃つのは `/drive` §5 のまま PM の go の後 | — |
| `ScheduleWakeup` / 自分での poll (`/drive` §5、§9) | 自分で wake できない mode として扱う: `/drive` §5 の最後の bullet (lane SSoT に書き、single-pass の verdict の poll は PM、`/pm` §2 行 6) | — |
| Claude の PreToolUse guard (`~/.claude/settings.json`) | **P1a 着地まで**: Codex の `hooks.json` から Claude の global hook script をそのまま呼ぶ (Bash の PreToolUse の guard、PM 911)。P1a の git hook (`pre-push` / `pre-commit`) が入ったらそちらが正 | 暫定 (PM 911)。Stop gate も含む (P2、PM 912)。amend / force-push の deny は global の script に切り出して Claude と Codex の両方から呼ぶ (user の diff 承認の後、PM 912) |
| ECS-mission guard (Claude の編集 hook) | **P1a 着地まで**: Codex の編集 (apply_patch) は hook から見えない。PR body の ratchet の行 (`/drive` §4) と CI の `tools` job が担う (PM 911)。P1a の `pre-push` が入ったらそれも加わる | 暫定 (PM 911) |
| `git` / `gh` (fetch、`worktree add`、add、commit、push、`gh` の API) | workspace-write の sandbox は `.git` を書けず、ネットワークも前提にしない。走らせるのは execpolicy の allow rule に載った形だけ (PM 877 / 878、置き場は global の `~/.codex/rules`、一覧は P2 の下書き。適用は P2 の hooks と同じ user の diff 承認の後、PM 912)。rule は argv の prefix で照合するので、`git -c k=v commit` のように間に token を挟んだ形は外れて sandbox の中で失敗する — plain な形で書く (照合は `codex execpolicy check --rules <file> <argv…>` で事前に確かめられる) | PreToolUse の guard は allow rule より先に発火する (PM 912) |
| merge (`/drive` §6 の 4) | **P1b 着地まで**: Codex driver は merge しない。§6 の 1 の報告までを行い、PM が `/pm` §4 の確認の後に PM session から merge する (PM 911)。merge commit の sha は PM から受け、`/drive` §7 の landing report を送る。P1b の後は `/drive` §6 に戻り、step 4 を wrapper の literal な形 `bash /Users/kazuaki/repos/send.sh/elidex/.claude/tools/agent/merge-pr <PR>` (`ELIDEX_PM_DIR` は起動環境から) で走らせる。go は token file `$ELIDEX_PM_DIR/handoff/codex-lanes/go/<PR>-<40 桁の head sha>` (PM 873 (6)) | 暫定 (PM 911)。wrapper は P1 memo §4.4 (未 ratify) |
| memory の書き込み、lane SSoT (`/drive` §0、§1 の 6、§9) | `--add-dir` で書けるのは `$PMD/handoff/codex-lanes/<lane>/` の 1 つだけ (`$MEM` は開かない)。lane SSoT はこの dir の `ssot.md` で、`/drive` §9 の 10 項目を書く。`$MEM` の lane の file の冒頭には、それを指す pointer の 1 行を pmo が書く (置き場は 1 つのまま)。mailbox と gate file も同じ dir (PM 912) | `.git` 以外の dir への add-dir は P2 で試験する (効かなければ PM 912 を見直す) |
| commit するブランチ用の worktree (`/drive` §1 の 9) | lane ごとに root dir を 1 つ置き、session は `codex -C <lane root>` で起動する。per-PR worktree はその下の subdir に `git worktree add` で作る (allow list に載る、PM 912)。lane root は elidex の guard の範囲 (`~/.claude/hooks/git-hook-bypass-guard.py` の `in_scope`) の中に置く | — |
| Stop hook (`止まる理由` の機械的な強制、`/drive` §10) | 規則は `/drive` §10 のまま。Codex の `hooks.json` から同じ Stop gate を呼ぶ (P2、PM 912) | — |
| budget (`/drive` §10、`/pm` §6) | pilot の間は Codex 側の cap を置かない (PM 912)。machine の負荷 (local の `mise run ci` は machine 全体で同時に 1 本) と disk の下限 (`/drive` §8) は効く。helper gate と PM の relay の Claude 側の token は Claude の cap に数える | — |
| restart 後の session 名 (`/drive` §9 の最後) | PM が wake に使う宛先 (Orca の terminal handle、または Codex の thread id) を `to-pm.md` で知らせる | — |
