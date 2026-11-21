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
| `/external-review` / `/external-converge` / `/elidex-plan-review` (`/drive` §5、repo `CLAUDE.md`「Edge-dense work」) | **未決 (Q3)** | — |
| `ScheduleWakeup` / 自分での poll (`/drive` §5、§9) | 自分で wake できない mode として扱う: `/drive` §5 の最後の bullet (lane SSoT に書き、single-pass の verdict の poll は PM、`/pm` §2 行 6) | — |
| Claude の PreToolUse guard (`~/.claude/settings.json`) | **P1a 着地まで**: Codex の `hooks.json` から Claude の global hook script をそのまま呼ぶ (Bash の PreToolUse の guard、PM 911)。P1a の git hook (`pre-push` / `pre-commit`) が入ったらそちらが正 | 暫定 (PM 911)。対象の範囲は Q5 / Q6 |
| ECS-mission guard (Claude の編集 hook) | **P1a 着地まで**: Codex の編集 (apply_patch) は hook から見えない。PR body の ratchet の行 (`/drive` §4) と CI の `tools` job が担う (PM 911)。P1a の `pre-push` が入ったらそれも加わる | 暫定 (PM 911) |
| `git` / `gh` (fetch、`worktree add`、add、commit、push、`gh` の API) | workspace-write の sandbox は `.git` を書けず、ネットワークも前提にしない。走らせるのは execpolicy の allow rule に載った形だけ (PM 877 / 878、置き場は global の `~/.codex/rules`、一覧は P2 の下書き)。rule は argv の prefix で照合するので、`git -c k=v commit` のように間に token を挟んだ形は外れて sandbox の中で失敗する — plain な形で書く (照合は `codex execpolicy check --rules <file> <argv…>` で事前に確かめられる) | allow list の適用時期は Q4 |
| merge (`/drive` §6 の 4) | **P1b 着地まで**: Codex driver は merge しない。§6 の 1 の報告までを行い、PM が `/pm` §4 の確認の後に PM session から merge する (PM 911)。merge commit の sha は PM から受け、`/drive` §7 の landing report を送る。P1b の後は `/drive` §6 に戻り、step 4 を wrapper の literal な形 `bash /Users/kazuaki/repos/send.sh/elidex/.claude/tools/agent/merge-pr <PR>` (`ELIDEX_PM_DIR` は起動環境から) で走らせる。go は token file `$ELIDEX_PM_DIR/handoff/codex-lanes/go/<PR>-<40 桁の head sha>` (PM 873 (6)) | 暫定 (PM 911)。wrapper は P1 memo §4.4 (未 ratify) |
| memory の書き込み (`/drive` §1 の 6、§9 の lane SSoT) | Claude の auto-memory は無いので、常に `$MEM` の絶対パスで書く (`/drive` §1 の 6 の「束縛されない」側)。sandbox の外なので書ける経路が要る | **未決 (Q1)** |
| commit するブランチ用の worktree (`/drive` §1 の 9) | session は `codex -C <lane worktree>` で起動し、その dir が書ける root になる | per-PR worktree の扱いは **未決 (Q2)** |
| Stop hook (`止まる理由` の機械的な強制、`/drive` §10) | 規則は `/drive` §10 のまま。Codex の Stop adapter は P2 | 範囲は Q6 |
| budget (`/drive` §10、`/pm` §6) | `usage-pace` は Claude の quota で、Codex driver は対象外。cap は kickoff brief が持つ | **未決 (Q7)** |
| restart 後の session 名 (`/drive` §9 の最後) | PM が wake に使う宛先 (Orca の terminal handle、または Codex の thread id) を `to-pm.md` で知らせる | — |

## 2. 未決 (PM の判断待ち — 決まるまで本 file は land しない)

決まったら表の該当行に結論と PM 番号を書き、その Q を本節から消す。

- **Q1** lane SSoT (`$MEM`)・mailbox・gate file への書き込みの経路。`--add-dir` (apply_patch の編集も通る) か、別の経路か。`$MEM` 全体を開くと、driver が書かない MEMORY.md・台帳・他 lane の SSoT も書ける root になる。
- **Q2** per-PR worktree が `-C` の root の外にある時: 起動し直すか、`--add-dir` か、lane worktree の中で branch を作るか。
- **Q3** post-push の `/external-review` (`/external-converge`) と、edge-dense の plan に要る `/elidex-plan-review` (5 軸の並列起動) を誰が回すか。Codex driver が skill の本文を読んで手で辿るか、helper に回すか。
- **Q4** execpolicy の allow list の適用時期。PM 878 は「P1 の着地の後の P2」、PM 911 は pilot を P1 の前に始める。allow list が無いと commit・fetch・`gh` が sandbox で失敗する。
- **Q5** 暫定の guard の範囲: amend / force-push の deny は `settings.json` の inline jq で script ではない — 呼べる script が無い。
- **Q6** 暫定の guard に Stop gate (`announced-next-step-gate.py`) を含めるか (PM 911 は Bash の PreToolUse と書く)。
- **Q7** Codex driver の budget の cap と、その測り方。
