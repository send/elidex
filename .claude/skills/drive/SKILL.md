---
name: drive
description: elidex lane-driver session. Start or resume a lane with just `/drive <lane>`; also run it after a compaction or restart. The driver owns its lane's PRs, branches and plan memos, routes every confirmation meant for the human through the PM session (cross-session SendMessage), runs its own merges only on the PM's "merge at head <sha>", and writes nothing but its own lane SSoT block. The PM's kickoff brief carries only lane-specific facts.
user-invocable: true
argument-hint: <lane>
---

# drive — elidex lane driver session

lane driver は **1 つの lane を持つ長命 session**。lane の PR・branch・plan memo を進め、確認はすべて PM に
送り、記録は自分の lane SSoT の冒頭 block だけに書く。

**lane は `/drive <lane>` の起動だけで始め・再開できること**が本 skill の要件。PM の kickoff brief が持つのは
lane 固有の事実 (lane 名、持つ PR / branch / memo、最初の成果物、今日の自己 cap と disk の下限、先に読むもの) だけ。
手順は brief に書かれていなくても本 skill が正で、brief と本 skill が食い違ったら PM に報告する (回避しない)。

## 本 skill が書かないもの (SSoT はそれぞれ 1 箇所)

| 何 | 正 |
|---|---|
| elidex の設計原則・至上命題・根拠の順位・ブラウザ名タグの一覧・Workflow (fmt / CI / worktree 隔離 / merge) | repo `CLAUDE.md` |
| subagent の型・tier・昇格・上限での引き継ぎ | `~/.claude/CLAUDE.md`「サブエージェント振り分け」 |
| PM が自分で決めるもの / user に上げるものの一覧 | memory `feedback_consult-pm-before-asking-user.md` |
| lens の回し方の incident 集 | memory `feedback_decide-via-philosophy-before-asking.md` |
| push 前の gate の段と各段の判定 | repo `.claude/skills/pre-push/SKILL.md` (review の段は Stage 4 の表) |
| review / converge / plan-review の手順 | `/external-review` / `/external-converge` / `/elidex-plan-review` / `/elidex-review` |
| edge-dense の規則 (umbrella + per-PR plan、実装前 plan-review) | repo `CLAUDE.md`「Edge-dense work」 |
| PM の merge 検証の checklist、比例性の閾値、着地後の台帳の更新 | `/pm` §4 / §2 行 7 / §5 |
| 台帳・PORTFOLIO・MEMORY Active state の書き方 | `~/.claude/agents/pmo.md` (書くのは pmo、渡すのは PM) |
| Codex CLI で driver を務める時の道具の差分 (Codex の driver は本 skill と併せて読む) | `.claude/skills/drive/codex.md` |
| repo に依らない判断・確認・検証・測定・止まり方・引き継ぎの一般則 (lens block と `lens-attest.sh`、根拠の段、規則の読みの先例、relay の本文、承認前の全文読み、claim の確かめ、0 件の control、時刻の変換、測定の作法、PID での停止、`止まる理由`、引き継ぎの繰り越し照合) | `~/.claude/CLAUDE.md`「判断と確認」「検証の作法」「並行セッション」「止まり方と引き継ぎ」(PM 736 で `/pm` §12 と `/drive` §8 / §10 から移した) |

## 0. 置き場所 (固定値)

| 何 | パス |
|---|---|
| memory dir (`$MEM`) | `/Users/kazuaki/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory/` |
| PM folder (`$PMD`) | `/Users/kazuaki/repos/send.sh/elidex-pm-trial/` |
| lane SSoT | MEMORY.md「Active state」の自分の lane の行が `→ [[…]]` で指す `$MEM/project_*.md` の冒頭 block |
| PM の session 名 | MEMORY.md「Active state」冒頭の `PM session = <名> [<ref>]` |
| landing リスト | `$PMD/handoff/pending-at-landing/<name>.md` |
| git の共有元 | `/Users/kazuaki/repos/send.sh/elidex` |

driver が書くのは lane SSoT の冒頭 block と、自分の PR・branch・plan memo だけ。MEMORY Active state・台帳
(`project_open-defer-slots.md` / `m4-12-landings-ledger.md`)・PORTFOLIO は PM のもので、書くのは pmo。

## 1. 起動・再開 (`/drive <lane>`、compaction 後、restart 後)

1. **`git -C /Users/kazuaki/repos/send.sh/elidex fetch -q origin`** を先頭で走らせる (以降の `origin/main` /
   `ls-remote` の比較を最新にする)。
2. **lane SSoT を読む**: 引数の lane 名で MEMORY Active state の行を引き、その行が指す lane SSoT の冒頭 block
   (§9 の 10 項目) を読む。worktree のパスはこの block が持つので、worktree は読んでから使う。冒頭が 10 項目の
   block でない時 (移行期) は、今の冒頭から 10 項目に組み直して PM に報告する (§9)。
3. **PM を確かめる**: PM の session 名 (§0) を `ListAgents` で確かめる。無ければ user に直接聞く (§2 の唯一の例外)。
4. **repo `CLAUDE.md` を Read で通読する** (grep の断片や要約で代えない)。context に読み込まれた版は session
   起動時のものなので、自分の worktree の版を読み、`git -C <worktree> diff HEAD origin/main -- CLAUDE.md` が
   空でなければ origin/main の版 (`git show origin/main:CLAUDE.md`) も読む。
5. **block は claim — git と突き合わせる**:
   `git -C <worktree> log -1 --format=%h`、`git -C <worktree> status --short`、
   `git -C <worktree> log --oneline origin/main..HEAD`、`git -C <worktree> ls-remote origin <branch>`。
   食い違いは自分の block を直し、PM に報告する。自分が作っていない WIP / HEAD の移動を見たら止めて確かめる。
6. **memory の書き先**: auto-memory は session を起動した dir で束縛される。確認は **session の primary cwd** で
   走らせる (`git -C <worktree>` にしない — worktree の値は session の束縛を表さない):
   `git rev-parse --path-format=absolute --git-common-dir` が `/Users/kazuaki/repos/send.sh/elidex/.git` でなければ、
   この session の auto-memory は `$MEM` に束縛されない → memory は `$MEM` の絶対パスで書く。
7. **brief の前提は claim**: PM の brief と lane SSoT に書かれた事実 (sha、PR の状態、「済み」) は 1 件ずつ
   git / `gh` で確かめる。前提の誤りは PM に報告し、回避しない。
8. 走っている agent・背景 run が lane SSoT にあれば、その状態を先に確かめる (§8、§9)。
9. lane SSoT の「次の 1 手」から始める。commit するブランチは専用 worktree で
   (`git worktree add -b <branch> <dir> origin/main`、規則の正は repo `CLAUDE.md` Workflow)。

## 2. 人間への確認はすべて PM へ

- **対象**: push / PR の作成 / merge / GitHub への書き込み (`@codex review`、thread の返信・resolve を含む) /
  terminator の帰結 / ratify 済み規則の読み / lane 内の設計の問い / **repo `CLAUDE.md` の文言の承認**。
  選択肢と推奨を付けて PM に `SendMessage` する。PM が決めるか user に上げるかの一覧は memory
  `feedback_consult-pm-before-asking-user.md`。
- **PM の答えが go**。go が許すのは名指ししたものだけ (次の push・次の merge には及ばない)。
- **user に直接聞くのは PM の名が `ListAgents` に無い時だけ**。
- harness の許可ダイアログは、この session の user が答えるもの。PM 経由で迂回しない。
- **repo `CLAUDE.md` の文言**: 最終文言と前後の diff を PM に送る。その後の流れの正は `/pm` §3.4。
- **聞く前に**: `~/.claude/CLAUDE.md`「判断と確認」(lens block、規則の読みの先例) と `/pm` §3.3 の 3 (手段の前に目的)・
  6 (lens が棄却する案を選択肢に混ぜない) を自分の問いに当てる。
- **決定番号は PM だけが引く**。lane の記録・報告に書く PM 番号は PM の返信にあった番号だけ
  (memory `feedback_driver-merge-needs-pm-go-and-no-decision-numbers.md`、PM 519)。

## 3. Push gate

1. `/pre-push` を通す (段と各段の判定は skill が正)。skill の「On completion」の提案先は PM (§2)。
2. **CI の判定は log で**: `EXIT=0` かつ log に `ERROR task failed` が無いこと。背景タスクの通知の exit code は
   最後の文の値なので判定に使わない (memory `feedback_background-task-exit-code-is-the-last-command.md`)。
   赤い local CI のまま push しない。signal で死んだ run は void — pass でも失敗でもなく、走らせ直す (§8)。
3. **review の段**: `/pre-push` Stage 4 の表が正。表から外れる段は `/pm` §2 の review の段の行に従う。
4. push の go を PM に求める: gate の結果 (段ごとの結果と skip の理由)、head の sha、
   `git log --oneline origin/main..HEAD` を添える。
5. **commit / push の直前**: repo `CLAUDE.md` Workflow の「並行セッション / worktree 隔離」に従う (branch・スコープの目視と明示 ref の push)。

## 4. PR body

- **ratchet の行**: PR の head で `python3 .claude/tools/ecs_mission_ratchet.py` を走らせ、出力の
  `ecs-mission-ratchet: ok (N allowlisted, 0 new, 0 stale)` の行を PR body に置く (PM の merge 検証が読む、`/pm` §4)。
  new が 1 以上なら push 前の gate に戻る。
- 欠陥・受け入れ条件・テスト期待値をブラウザの振る舞いで定義した行は push 前に直す (根拠の順位は repo
  `CLAUDE.md`)。タグの使い方の妥当性は `/elidex-review` の Axis 2 sub-check 2d が見る。PR body の
  ブラウザ名の分類表は書かない (ratchet が CI で止める、PM 497)。

## 5. Review loop と比例性

- post-push の review は `/external-review` (既定の single-pass) か、合意した PR だけ `/external-converge`。
  手順は各 skill が正。`@codex review` を撃つのも PM の go の後 (§2)。
- **回数の報告**: review loop の round の報告と merge の報告に、その PR の **`@codex review` の回数** と
  **plan delta の回数** を書く。閾値と判定は PM が持つ (`/pm` §2 行 7) — driver は閾値で自分を止めない代わりに、
  PM が理由を問うたら答え、loop 継続の判定が出るまで次の round を撃たない。
- loop の終わり方は round を始める前に PM と合意し、lane SSoT に書く (§9 の「終わり方と今のカウント」)。
  「次に出たら user へ」を既定の終わり方にしない。
- **自分で wake できない mode** (Claude Desktop の prompting mode 等) なら、その旨を lane SSoT に書き、
  single-pass の verdict の poll を PM に頼む (PM 361)。

## 6. Merge (PM 110)

1. 「CI green、review loop 終了 (合意した終わり方)、head `<sha>`」を PM に報告する。Codex の dry・CI の緑・
   thread 0 は merge の go ではない。
2. PM の **`merge at head <sha>`** を待つ (その PR・その head だけを許す)。
3. 直前に head と `mergeStateStatus` を再確認する:
   `gh pr view <N> -R send/elidex --json headRefOid,mergeStateStatus`。head が動いていたら merge せず聞き直す。
   `DIRTY` / `BEHIND` は main を **merge で** 取り込む (push 済みの branch を rebase しない、force-push しない) →
   CI を待って再報告。
4. `gh pr merge <N> -R send/elidex --squash --match-head-commit <sha>`。flag の禁止は repo `CLAUDE.md` Workflow の「Git」に従う。
5. merge commit の sha を §7 の landing report として PM に送る。

## 7. Landing report

merge の後、PM に 1 通で送る:

- merge commit の sha (origin/main 上で `git log -1 <sha>` を確かめた後)
- 台帳に写す文面の場所: merge commit 上の file の `<path>:<lines>`、または landing リスト
  (`$PMD/handoff/pending-at-landing/<name>.md`)。文面は報告文に書き写さない (pmo は merge commit 上の file から写す)
- **この PR で carve / close / 撤回した slot ID の一覧** (無ければ「なし」と書く。cite の無い carve を拾うため)
- 他 lane への影響 (lint の変化、rebase の要否、共有 file への変更)
- 比例性の回数 (§5)
- slot の文案は PM が前提を確かめてから登録する (`/pm` §8)。lane が出す文案の前提は claim であって確認済みではない
- slot の文面 (BEGIN/END) に行番号を書かない。cite は「file + symbol」で書き、sha は台帳の出典欄にある merge commit の 1 つだけにする (squash merge の後に辿れない sha と、基準の混ざった行番号を生まないため) (PM 648)
- 件数は計測の出力として、コマンド・head・件数の 3 つで書き、固定の数字で書かない。母集団は性質で定義し、件数は「その head での測定値」とする (PM 659/662)

台帳・MEMORY Active state は書かない (PM が pmo に渡す)。自分の lane SSoT だけを更新する。

## 8. 測定の作法

- 規則 (toolchain を名指す測定、user の state に書く probe の隔離、並列度の上限、背景ジョブを PID で止める、
  clone の再利用と `df -h`) は `~/.claude/CLAUDE.md`「検証の作法」「並行セッション」が正 (PM 734 で本節に置き、
  PM 736 で global に移した)。incident の記録は memory `reference_session-rustup-toolchain-env-stale.md` /
  `feedback_mise-probes-isolate-state-dir.md` (PM 151 / 230) / `feedback_cap-delegate-experiment-parallelism.md` /
  `feedback_kill-by-pid-not-pattern.md` / `feedback_disk-budget-for-measurement-clones.md`。
- **disk の下限**: PM が新しい測定 batch を止めたら (`/pm` §2 行 3) 新しい batch を始めない。

## 9. Handoff (lane SSoT の冒頭 block)

lane SSoT の冒頭に、次の 10 項目を書く (冒頭が今これでない移行期は §1 の 2 で組み直す。PM も pmo も書かない。PM は git と突き合わせて clear してよいかを user に伝える):

1. head / branch / worktree
2. 未 push の commit
3. 待っているもの
4. 次の 1 手
5. 走っている agent・背景 run (PID、log の path)
6. 効いている PM / user の決定 (PM の返信にあった番号付き)
7. 終わり方と今のカウント (terminator の終わり方、round 数、`@codex review` / plan delta の回数、agent の昇格)
8. 退けた案 (試して落とした修正、撤回された決定 — fresh session が再提案しないため)
9. artifact の絶対パス (plan、findings の記録、凍結した copy。scratchpad は session ごとに別)
10. GitHub の状態 (PR 番号、最後の CI run、Codex を依頼した時刻、未解決の thread、poll の再開点)

- **いつ**: 長い待ち (Codex / CI の poll、長い背景 run、ScheduleWakeup、PM / user の返事待ち、日の終わり) に入る前。
  変わった所だけ更新する。idle の間に auto-compaction が走る。
- 走っている agent・背景 run がある間は clear しない。`/clear` `/restart` は user の操作。
- restart の後は session 名が変わる (ref は同じ)。新しい名を PM に知らせる。

## 10. 止まり方

- 規則 (`止まる理由`、Stop hook、「次は X」を書かずに tool call を出す、待ち以外が残れば同じ turn で続ける) は
  `~/.claude/CLAUDE.md`「止まり方と引き継ぎ」。driver は残作業の有無によらず **毎 turn** の最後の段落を
  `止まる理由: <…>` にする (PM の go 待ち / PM の判定待ち / Codex 待ち (head <sha>, <JST 時刻>) / CI 待ち /
  user の操作待ち / budget の cap)。
- **budget の停止**: 正は `/pm` §6 (新しい単位だけ止める、走行中の agent は止めない)。単位の境界で済 / 残を PM に報告して止まる。
