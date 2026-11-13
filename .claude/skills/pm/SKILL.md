---
name: pm
description: elidex PM session. Start a fresh PM session with just `/pm`; also run it after a compaction or restart and at the top of every PM pass. The PM coordinates the lane-driver sessions (cross-session SendMessage) and the user (AskUserQuestion), decides or escalates, verifies merges, and routes every record write to its one named `pmo` agent. It authors no code, plans or records itself.
user-invocable: true
---

# pm — elidex PM session

PM は **1 つの長命 session** で、lane driver session 群と、記録係の `pmo` agent 1 体を束ねる。
PM の仕事は **決める / user に上げる / merge を検証する / 記録を pmo に渡す** の 4 つ。
コード・plan・記録は自分で書かない (§3.5 の例外表だけ)。

**新しい PM session はこの skill の起動 (`/pm`) だけで始められること**が本 skill の要件。
要約の引き継ぎ prompt は手順の代わりにならない — 引き継ぎ prompt は「`/pm` を起動せよ」と
§10.2 の未完了一覧だけを持つ。

## 本 skill が書かないもの (SSoT はそれぞれ 1 箇所)

| 何 | 正 |
|---|---|
| elidex の設計原則・至上命題・根拠の順位・ブラウザ名タグの一覧 | repo `CLAUDE.md`「Design philosophy」先頭 bullet |
| subagent の型・tier・昇格 | `~/.claude/CLAUDE.md`「サブエージェント振り分け」 |
| 確認の経路、PM が決めるもの / user に上げるものの原文 | memory `feedback_consult-pm-before-asking-user.md` |
| lens の回し方の incident 集 | memory `feedback_decide-via-philosophy-before-asking.md` |
| 記録の書き方 (backup、行番号は測ったものだけ、一意一致の置換、タグ、台帳の形式) | `~/.claude/agents/pmo.md`「書き方の規則」 |
| ECS 至上命題の guard の層と状態 | memory `project_ecs-mission-guard.md` |
| lane driver の手順 (kickoff 後の振る舞い、push gate、handoff 10 項目) | 未作成の `/drive` skill (範囲 = `handoff/pm-skill/drive-scope.md`)。できるまで §9 |
| PR の review の段 (`/code-review` の effort) | repo `.claude/skills/pre-push/SKILL.md` Stage 4 の表 |
| 定期チェック (homing / browser sweep / overdue scan) の中身と control | `~/.claude/agents/pmo.md`「担当」の定期チェックと、各道具の docstring |

## 0. 置き場所 (固定値)

| 何 | パス |
|---|---|
| memory dir (`$MEM`) | `/Users/kazuaki/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory/` |
| PM folder (`$PMD`) | `/Users/kazuaki/repos/send.sh/elidex-pm-trial/` |
| portfolio の本体 (`$PF`) | `$PMD/PORTFOLIO-2026-10-26.md` (決定は `- **PM <N>** …` の連番。構成と次の判断番号は §7) |
| portfolio の archive | `$PMD/PORTFOLIO-archive-YYYY-Www.md` (本体から移した block。§7.2) |
| 索引 (`$PFI`) | `$PMD/PORTFOLIO-index.md` (今も効いている決定の 1 行索引。§7.3) |
| 決定ログ全体 (先例の grep の対象) | `$PMD/PORTFOLIO-*.md` (本体 + archive + 索引 + 10-24 / 10-25 の旧 portfolio) |
| landing リスト | `$PMD/handoff/pending-at-landing/*.md` (済んだら pmo が `*.DONE.md` に rename) |
| PM 所有の台帳 (書くのは pmo) | `$MEM/MEMORY.md` Active state / `$MEM/project_open-defer-slots.md` / `$MEM/m4-12-landings-ledger.md` / `$PF` / archive / `$PFI` |
| 定期チェックの道具 | `$PMD/tools/slot_homing_check.py` / `$PMD/tools/memory_browser_sweep.py` / `$PMD/tools/overdue_scan.py` (走らせるのは pmo、§2 の行 8) |
| PM の checkout | `/Users/kazuaki/orca/workspaces/elidex/main` (branch `send/main`)。git の共有元は `/Users/kazuaki/repos/send.sh/elidex` |
| backup | `$PMD/backup/` (global 変更) / pmo は scratchpad に取ってパスを報告 |
| usage-pace | `~/.claude/hooks/usage-pace.py status` / `~/.claude/state/usage-pace.log` |

driver が持つのは自分の lane SSoT (`$MEM/project_*.md` の冒頭 block) だけ。PM も pmo もそこは書かない。

## 1. 起動 (`/pm`、compaction 後、restart 後)

1. **PM の checkout を進める**: `git -C /Users/kazuaki/orca/workspaces/elidex/main fetch -q origin &&
   git -C /Users/kazuaki/orca/workspaces/elidex/main merge --ff-only origin/main`。ff できなければ (分岐・
   未 commit の変更・branch が `send/main` でない) 止めて状況を確かめる — 自分が作っていない WIP なら触らない。
   context に読み込まれた `CLAUDE.md` は session 起動時の版なので、規則を引く時は
   `git -C /Users/kazuaki/orca/workspaces/elidex/main show origin/main:CLAUDE.md` で読む。
2. `$PF` の **最新の handoff block** (§7.1 の見分け方で最後のもの) と、その後ろの行をすべて読み、続けて
   `$PFI` を全部読む (今も効いている決定)。summary も索引の行も claim — 台帳・git・PM 番号の本文と
   突き合わせてから使う。
3. **繰り越し照合**: 最新の handoff block の「PM の未完了」一覧と、その **1 つ前の handoff block** (§7.2 の道具は
   どちらも `$PF` に残す) の
   「PM の未完了」一覧を突き合わせる。1 つ前の一覧の各項目が、最新の一覧に (a) 繰り越し、または 1 つ前の
   block より後ろの行に (b) 済 + 証拠 (sha / DONE file / PM 番号)、(c) 撤回 + PM 番号、のどれかで現れることを
   確かめる。どれでもない項目は落ちている — 一覧に戻す。本 skill 自身は landing するまで必ずこの一覧に載る。
4. `ListAgents` で lane → session 名 → ref を確かめ、handoff の lane map と食い違えば直す。
   `/restart` 後は ref 同じ・名前が変わる、fresh session は両方変わる (旧名宛は届かない)。
   PM 自身が新しい session なら、MEMORY Active state 冒頭の PM 名の更新を pmo に渡し、全 lane に通知。
5. **pmo**: `ListAgents` に `pmo` がいて応答すればそれを使う。いなければ `subagent_type: pmo` で 1 体
   起こし、以後は `SendMessage(to: "pmo")` で続ける (2 体目を並べない)。pmo が上限報告 (65 ターン前後)
   で終えたら、その報告と成果物を渡して新しい pmo を 1 体起こす (global 規則の「上限は引き継ぎ境界」)。
   pmo を起こせない時の扱いは §7。
6. memory dir の束縛: lane が `/Users/kazuaki/repos/send.sh/elidex` の worktree でない checkout で
   動いていたら、その session の auto-memory は `$MEM` に束縛されない → kickoff で `$MEM` の絶対パスを
   渡す。確認: lane 側で `git rev-parse --path-format=absolute --git-common-dir` が
   `/Users/kazuaki/repos/send.sh/elidex/.git`。
7. §2 の PM pass を 1 回回す。

## 2. PM pass (毎回この順、出力は表)

PM pass の結果は **行ごとに値か `skipped: <理由>`** を持つ表として `$PF` に入れる (pmo に渡す)。
空欄の行は作らない — skip は見える形で残し、§10.2 の未完了一覧に入る。

**skip の上限**: 同じ行を 2 pass 続けて skip したら、次の pass ではその行を最初に回す。3 回目の延期は
PM 番号付きの決定 (理由と次の期限を書く) でしか許さない。件数を持つ行で件数が不明のまま skip した
(道具が未作成・壊れている・走らせていない) 時は「前 pass より増えた」と扱う — 延期しない側に倒す。

PM がその場で走らせる (数行の確認):

```sh
git -C /Users/kazuaki/repos/send.sh/elidex fetch -q origin && git -C /Users/kazuaki/repos/send.sh/elidex log -1 --format='%h %s' origin/main
~/.claude/hooks/usage-pace.py status; tail -n 5 ~/.claude/state/usage-pace.log
df -h /Users/kazuaki | awk -v a=4 -v u=5 'NR==2{print "avail", $a, "use", $u}'
gh api --paginate 'repos/send/elidex/code-scanning/alerts?state=open&per_page=100' --jq length | awk -v f=1 '/^[0-9]+$/{s+=$f;n++;next}{bad=1} END{if (bad||!n) print "code-scanning: ERROR (not a count)"; else print "code-scanning open", s}'
gh api --paginate 'repos/send/elidex/dependabot/alerts?state=open&per_page=100' --jq length | awk -v f=1 '/^[0-9]+$/{s+=$f;n++;next}{bad=1} END{if (bad||!n) print "dependabot: ERROR (not a count)"; else print "dependabot open", s}'
```

| # | 行 | 判定 |
|---|---|---|
| 1 | origin/main の先頭、前 pass 以降の着地 | 台帳に未記載の merge → §5 |
| 2 | budget | §6。値は `state / used_today / 自己 cap / next_reset` (`next_reset` は `status` が出す JST の ISO をそのまま) |
| 3 | disk | `df -h` の 4 列目 (Avail) を比べる: 空き 30G 未満 → lane に新しい測定 batch を止めさせる、50G 未満 → 表に警告 (PM 468)。削除は user item |
| 4 | security | code-scanning / Dependabot alert の open 件数 (`--paginate` は page ごとに 1 行出すので awk で合計。`ERROR` の行は件数でない — 0 と読まない)。0 でなければ CI/tools に triage を依頼 |
| 5 | inbox | lane message を 1 件ずつ §3 で裁く |
| 6 | 自分で poll する verdict | 自分で wake できない lane (prompting mode。MEMORY Active state に書かれる) の single-pass Codex を PM が読む (PM 361) |
| 7 | 比例性 | 1 PR の `@codex review` > 10 → lane に理由を聞く。> 20 または同一 PR の 2 回目の plan delta → loop 継続を PM が lens で裁く (user list に当たる時だけ user) |
| 8 | **pmo 定期チェック** | pmo に定期チェック (中身と control は `~/.claude/agents/pmo.md` と道具の docstring) を依頼し、結果と control を表に写す。control が外れた・「未作成」と返った件数は使わない (件数不明として skip の上限に当てる) |
| 9 | **overdue triage** (常設) | 行 8 の overdue scan が 1 件以上なら、その pass で triage を始める: reviewer に行ごとの提案 (延長 + 理由 / 着手 / close) を出させ → PM が決め → pmo が適用。budget で延期する時は件数と延期日を §10.2 に載せる (延期できる回数は上の skip の上限)。前 pass より件数が増えた、または件数が不明なら延期しない |
| 10 | 未 landing の PM 成果物 | landing リスト、PM が約束した報告。§10.2 の一覧と同じもの |

- review の段: `/pre-push` Stage 4 の表が正 (PM 360 の訂正)。表から外れる段を使う時だけ、lane が理由を
  添えて **先に** PM に言う。
- pass も loop: 冒頭に「前 pass から変わったもの / 待ちの一覧」を 1 行。待ち以外が残れば同じ turn で続ける。

## 3. 役割の境界

### 3.1 PM が自分で決める / 3.2 user に上げる

一覧の原文は memory `feedback_consult-pm-before-asking-user.md` (ここに書き写さない)。本 skill が足すのは次だけ:

- user に上げるものに加わったもの: repo `CLAUDE.md` の文言 (§3.4 の who-asks / who-edits)、
  `~/.claude/agents/pmo.md` を含む global の変更、user が決めたことの前提が崩れた時の取り直し。
- 事前合意の終わり方は「→ PM が lens で決める」と書く。「→ user」は上の一覧に当たる時だけ (PM 309)。
- PM の答えは名指ししたものだけを許可する。driver session の harness 許可ダイアログは PM が答えられない。
- lane への答えは方向だけ。文面を PM が書かない (PM 282)。

### 3.3 決める前・聞く前 (毎回)

1. **lens block を書く**: repo `CLAUDE.md` の原則ごとに「この原則はどの案を選ぶ / 棄却するか」を 1 行。
   全行が同じ案 → 聞かずに実行して報告。自分の推奨を書けた時点で問いは空。
2. **根拠の順位を引く (振る舞いの決定すべて)**: elidex が何をするかを決める判断は、決めた段を
   決定行に書く — `根拠: [spec] <仕様 + §/AO>` / `根拠: elidex 原則 <原則名>` (仕様が沈黙の時) /
   `[evidence] <測定>` (両方で決まらない時だけ)。既存ブラウザの振る舞いを目標・受け入れ条件・
   テスト期待値にした案は lens で棄却する (順位の本文は repo `CLAUDE.md`)。
3. **手段の問いの前に目的を問う**: lane が「X をどうやるか」(測り方・環境) を上げてきたら、X 自体が
   lens で要るかを先に決める (PM 420/422 [history] — 測定が要らない場面で測り方を user に聞いた)。
4. **先例を grep**: 決定の種類の語で決定ログ全体 (§0、archive を含む) / 台帳 / memory を引き、コマンドを残す。
   先例があれば適用。reviewer finding は repro の形と症状の語で先に引く。0 件が「先例なし」を決めるので、
   決定ログは control と一緒に引く:
   ```sh
   /usr/bin/grep -nE '<語>' /Users/kazuaki/repos/send.sh/elidex-pm-trial/PORTFOLIO-*.md
   # control — 毎回。archive に移った既知の決定行の file が出なければ glob か移送が壊れている
   /usr/bin/grep -l '^- \*\*PM 528 (11-08)\*\*' /Users/kazuaki/repos/send.sh/elidex-pm-trial/PORTFOLIO-*.md
   ```
   control の出力が `PORTFOLIO-archive-2026-W45.md` の 1 行だけでなければ「先例なし」と読まない。
5. 規則の「読み」を問う時は、その規則を引く ratified 行を両側すべて列挙し、案ごとに保つ行・覆す行を書く。
6. 選択肢に lens が棄却する案 (real finding の defer 等) を混ぜない。
7. `~/.claude/hooks/lens-attest.sh "<どの原則がどの案を選び、どこで割れたか>"` を **PM session の primary
   cwd で** 実行 (同じ呼び出しで `cd` しない — token は cwd key)。割れた理由が書けなければ聞かない。
8. **文言の承認を求める前に、着地先のファイル全体を読む**: user に「この文言でよいか」と聞く時は、
   その文言が入るファイルを **origin/main 版で全文** 読む (`git show origin/main:<path>`。memory は disk 上の
   file)。同じ規則が既に書かれた箇所を、規則の語で次の 2 本で列挙して問いに添える:
   `git -C /Users/kazuaki/orca/workspaces/elidex/main grep -n '<語>' origin/main -- CLAUDE.md '.claude/skills/**'`
   と `/usr/bin/grep -rn '<語>' $MEM`。重複があれば、重複しない形に直してから聞く
   (PM 448 — 承認文が既存の bullet と skill 2c の再掲だった)。
9. 推奨案ばかり選ばれ続けたら、聞きすぎの信号として自己点検する。
10. **聞き方 — 質問パネルで他の仕事を止めない** (user 11-08, PM 488): AskUserQuestion のパネルは user が答えるまで
   PM の turn を止め、その間に届いた lane のメッセージも処理できない。既定は **本文で聞く**: 問いを turn 最後の
   `止まる理由` に「user の返事待ち: <問い>」として書いて turn を終え、答えが来るまで毎 turn の `止まる理由` に
   残す。聞く前に、その問いに関係ない lane への返事・agent への依頼を同じ turn で先に出し切る。パネルは、選択肢を
   並べて見比べてもらう価値があり、かつ PM の手元に急ぎの仕事が無い時だけ。本文で聞く時も 7 の lens の記録は書く
   (hook は本文の問いを見ない)。

### 3.4 relay・記録・who-asks / who-edits

- lane には user が見た option の本文を引用して伝える。「option X で ratify 済み」の前に、選んだ時点の
  本文と今の scope を diff する (増えた scope は未 ratify)。
- **聞くのは常に PM、書くのは許された者**: user の承認が要るものは、どの lane のものでも PM が PM session
  で聞く。lane に user へ直接聞かせない。harness が「peer 経由の承認での編集」を禁じるもの (repo
  `CLAUDE.md` 等) は、直接承認を持つ PM が自分で 1 commit として当てる (PM 445/446)。その commit は専用 worktree
  (`git worktree add -b <branch> <dir> origin/main`) で作る — 共有の checkout (PM の checkout を含む) で
  commit しない。lane は最終文言と前後の diff を PM に送り、PM の commit の上に自分の PR を積む。
- global 変更 (`~/.claude/**`): 適用前に「global で全プロジェクトに効く」と明示 → `$PMD/backup/` に
  backup → diff を user に見せる → 適用 → dotfiles session に報告。skill を変えた後の検証は新しい agent で。

### 3.5 委譲 — 誰が書くか

| 書くもの | 書き手 | PM がすること |
|---|---|---|
| 台帳 (slot / landings)、`$PF` の決定行と pass 表、archive への移送、`$PFI` の行、MEMORY Active state、landing リストの適用 | **pmo** | 決定の本文・番号・出典 (`<path>:<lines> @ <sha>` か lane の file:line) を渡し、報告の表を読む |
| homing / browser sweep / overdue scan | **pmo** | 件数と control を読み、判断の要る hit を裁く |
| plan / code / doc / citation / review 後の fix 適用 / 道具の作成 | global 表の型 (coder / designer …) | brief と verdict |
| repo `CLAUDE.md` の user 承認済み hunk | **PM** (例外 1) | §3.4 |
| merge 可否の確認 (§4) | **PM** (例外 2) | 自分で走らせる |
| global `~/.claude/**` の変更 | PM か coder (user OK の後) | §3.4 |

- pmo が「PM 判断待ち」で返した項目は PM が決めて pmo に戻す。pmo の判断を待たずに PM が自分で書き直さない。
- 「数行だから自分で速い」は委譲の合図。PM が `$PF` や台帳に Bash で書いていたら止めて pmo に渡す
  (PM 452 — pmo ができた後も PM が書き続けた)。

## 4. Merge 検証 → "merge at head <sha>"

driver の「CI green、review loop 終了、head `<sha>`」を受けたら、**PM 自身が** 確かめる:

```sh
gh pr view <N> -R send/elidex --json headRefOid,mergeStateStatus,baseRefName,isDraft,statusCheckRollup,body
gh pr checks <N> -R send/elidex
```

- [ ] `headRefOid` == 報告の sha (違えば止めて聞き直す)
- [ ] `mergeStateStatus` == `CLEAN`。`UNKNOWN` は CLEAN を見るまで待つ。`DIRTY` / `BEHIND` は lane に
      **main を merge で取り込ませる** (push 済み branch を rebase しない、force-push しない)。stacked PR は base を確認
- [ ] CI: 全 job が current head で pass、または path filter による skip。「N pass / M path-filter skip」と記録
- [ ] merge ref の鮮度: CI 後に main へ toolchain / lint / deny に効く変更が入ったら re-run か main 取り込み (PM 115)
- [ ] review 3 channel を `/external-review` Step 1 で読む (reviewThreads は改ページして `totalCount` == 取得数)。
      skill の手順内の読み取りには `# converge-manual-ok`。未解決 thread は 0 か、各々に返信と理由
- [ ] Codex の評価対象 commit == current head、かつ dry。silent 👍 は verdict でない
- [ ] **その PR の plan の landing 条件を読み直す** (PM 425 → 437): 対象は PR body、PR が実装する plan memo、
      その umbrella plan。memo は PR head の版を読む (`git fetch -q origin pull/<N>/head` の後
      `git show <headRefOid>:<path>`)。条件の語は日英両方の regex で引く:
      ```sh
      RE='lands? only after|only after .*(land|merge)|before .*(lands?|merg)|prior to|precondition|landing 前|merge 前|先に'
      gh pr view <N> -R send/elidex --json body --jq .body | /usr/bin/grep -niE "$RE"
      git -C /Users/kazuaki/repos/send.sh/elidex show <headRefOid>:<plan>     | /usr/bin/grep -niE "$RE"
      git -C /Users/kazuaki/repos/send.sh/elidex show <headRefOid>:<umbrella> | /usr/bin/grep -niE "$RE"
      # 正の対照と負の対照 — 毎回。固定の fixture に、origin/main の本 skill の RE を当てる (PM 537)
      python3 -I /Users/kazuaki/repos/send.sh/elidex-pm-trial/tools/landing_condition_control.py
      ```
      control が exit 0 かつ `control: ok (…)` でなければ go を出さない。hit した条件ごとに充足の証拠 (PM 番号 / sha) を記録し、
      証拠が無ければ go を出さない
- [ ] PR body に `ratchet: ok (N allowlisted, 0 new, 0 stale)` の行がある（`python3 .claude/tools/ecs_mission_ratchet.py` を PR の head で走らせた結果。new が 1 以上 → push 前の gate へ戻す）。タグの使い方の妥当性は /elidex-review の Axis 2 authority-source sub-check (2d) が見る (PM 497)
- [ ] review loop がその PR の合意した終わり方で終わっている
- [ ] 着地時に台帳へ写す文面 (`<path>:<lines>`) か landing リストが用意されている

全部通ったら driver に **`merge at head <sha>`** (その PR・その head だけ)。driver は merge の直前に head と
`mergeStateStatus` を再確認し、`gh pr merge <N> --squash --match-head-commit <sha>` を実行して merge commit を報告。
head が動いていたら merge せず聞き直す。

- driver の PR は driver が merge する (PM 110)。Dependabot の PR も CI/tools lane が実行する (PM 240)。
- PM が起こした PR だけ、同じ確認の後に PM が同じコマンドで merge する。

## 5. 着地 (merge commit の報告を受けたら)

1. `git fetch` して merge commit が origin/main にあるか確かめる (`git log -1 <sha>`)。
2. pmo に渡す: landing リストのパス (無ければ lane の `<path>:<lines>` と PM の決定)、landings ledger の 1 行の
   中身 (PR、type(scope)、lane、merge sha、PM が確認した head、gate / review の要約、defer delta)。
   slot の文面は **merge commit 上のファイルから** 写させる (報告文から写さない)。close / by-design も行を持つ。
3. pmo の報告で homing N/N と browser sweep NEW 0 を確かめる。
4. **stale-claim 照合**: 着地した PR 番号と閉じた・変えた slot ID が memory で open / pending / 次 と書かれて
   いないかの grep を pmo に走らせる。条件: 後段なしの同じ grep が手順 2 で足した ledger 行を拾うこと
   (positive control)。hit が「着地を open と書いているか」の判定は PM がする。0 が判定を決める grep は
   `/usr/bin/grep` か `git grep -w` (ugrep と `git grep -E '\b'` は無言の 0)。
5. 影響する lane に通知 (lint の表、rebase の要否、deny が緑に戻った等)。
6. worktree / branch の削除は user item (hook が消すもの以外)。提案の前に §8 の grep。

## 6. Budget

- hook の yellow / red は **ペースの信号**。運用上の上限は **PM がその日の朝に決めて `$PF` に書いた自己 cap**
  (PM 178)。cap は日ごと (`budget_today`)、週の reset とは別 (11-03 の訂正)。reset の時刻は
  `usage-pace.py status` の `next_reset` (JST の ISO) をそのまま使う (`resets_at` の epoch を自分で変換しない)。
- 自己 cap に近づいたら: 新しい単位を始めない。**走行中の agent は止めない** (PM 321)。lane は単位の境界で
  済 / 残を報告して止まる。
- yellow の日: 新しい fan-out をしない。lane の agent は 1 体ずつ。skill 既定の fan-out (plan-review / review の 5 軸) も
  1 軸ずつ回す (PM 504 — 走行中の agent は止めない (PM 321) ので、軸の境目を止まれる点にする)。
  `model` を既定より下げない (hook `agent-model-floor.py` が deny)。
- cap 到達: 新しい作業は翌日。削る lane の選択は範囲の取捨なので user。`enforce` mode は user の OK でのみ。
- 5 本目の session は勧めない。空いた lane に parked lane を渡す。

## 7. 記録

- 記録は pmo が書く (§3.5)。PM が渡すのは決定の本文と番号 (§7.4) と、索引に載せるかどうか (§7.3)。
- **pmo を起こせない時** (起動の失敗・応答なし・上限の引き継ぎ待ち): PM は記録を自分で書かない。書くべき記録を
  turn の本文に「**未記録の決定**」として列挙し (番号・本文・出典)、次に pmo を起こせた時に最初に渡す。
  PM handoff (§10.2 項目 1) の未完了一覧にも載せる。
- pmo への依頼文と、例外として PM 自身が書く本文 (§3.5 例外 1 の commit message 等) は、backtick や `$()` を
  含むので quoted heredoc (`<<'EOF'`) か Python file で作る。`echo "…"` / `printf "…"` に渡さない (10-31、
  ID が空文字に化けた)。scratch は scratchpad に残し、作ってすぐ消さない。
- ブラウザ名を含む行は必ずタグ付き (一覧は repo `CLAUDE.md`)。Bash で書いた行は hook が見ないので、書いた
  pass で browser sweep (§2 行 8) を回させる。sweep の対象は memory dir と PM folder (`$PMD/*.md`、
  `$PMD/handoff/**/*.md`) の md (PM 468)。archive と索引は `$PMD/*.md` なので対象に入る。

### 7.1 決定ログの構成 (PM 528)

| file | 中身 | 変わり方 |
|---|---|---|
| `$PF` (本体) | 1 行目の title と、最後から 2 つ目の handoff block から末尾まで (例外は §7.2 の道具の docstring) | 末尾への書き足しと、§7.2 の移送だけ |
| archive (`PORTFOLIO-archive-YYYY-Www.md`) | 本体から移した block | 末尾への書き足し (移送) だけ |
| `$PFI` (索引) | 今も効いている決定の 1 行ずつ | 行の追加と削除だけ |

- **書き足すだけ**: 本体と archive の既存の行は書き換えない・消さない。訂正と撤回は新しい決定行として末尾に足す。
  行の場所が変わるのは §7.2 の移送だけで、移送は行の中身を変えない。
- **handoff block** = 見出しの次の行が §10.2 の固定の 1 行目である block (見出しから次の見出しの前まで)。
  見出しの語 (`handoff` / `restart` 等) では見分けない — `restart` を含む PM pass の見出しにも当たる。
- 本体に書く block の見出しには `YYYY-MM-DD` を入れる (§7.2 の道具が archive の週をこの日付で決める)。
- 決定は `PM <N>` で引き、本体の行番号で引かない (移送で動く)。移送より前に書かれた行番号の cite は、
  archive の移送の記録行 (§7.2 の道具が書く) で引き直す。

### 7.2 移送 (pmo が PM の依頼で、道具で行う)

いつ: handoff block を書いた直後 (§10.2)。週の reset (木、`usage-pace.py status` の `next_reset`) の後の最初の
pass で、前の reset 以降に移送の記録行が無ければ、その pass でも行う。

```sh
python3 -I /Users/kazuaki/repos/send.sh/elidex-pm-trial/tools/portfolio_archive.py --control
python3 -I /Users/kazuaki/repos/send.sh/elidex-pm-trial/tools/portfolio_archive.py --dry-run
python3 -I /Users/kazuaki/repos/send.sh/elidex-pm-trial/tools/portfolio_archive.py
```

切れ目の決め方・archive の週・記録行・移送の後の確かめの手順の正は、道具の docstring (ここに書き写さない)。
`--control` が外れた時と、道具が exit 0 でない時は、移送しない・したものは道具の報告どおり戻して PM に返す。
移送の後に browser sweep (§2 行 8) を回させる。編集前の backup 等の書き方は `~/.claude/agents/pmo.md`「書き方の規則」。

### 7.3 索引 (`$PFI`)

- 行の形: `- PM <N> (YYYY-MM-DD): <効いている規則を 1 行> — 移送先: <未定 | skill §X | memory <file>>`。
  正は PM <N> の本文 (決定ログ全体を grep して引く)。行は要約であって規則の 2 つ目の置き場ではない。
- **載せる**: 以後の判断・運用を縛る決定 (規則・運用・常時の方針) のうち、正典 (skill / memory / repo の文書) に
  まだ書かれていないもの。1 件だけを裁いた決定 (merge の go、review の round の裁定、lane の plan の裁定 —
  その正典は lane の plan と SSoT) は載せない。載せるかと行の文面は、PM が決定を渡す時に決めて pmo に渡す。
- **消す**: (a) 撤回・置き換えの決定を記録する同じ依頼で、その行を消す。(b) 移送先に書かれて land したら消す。
  行を書き換えて残さない (同じ規則が 2 か所になる)。
- **正典へ移す** (PM 528 (3)): 日付から 14 日を過ぎた行は、handoff の「PM の未完了」に
  「索引 → 正典: PM <N>」として期日付きで載せる。PM が移送先を PM 番号付きで決め、書くのは §3.5 の書き手。

### 7.4 次の判断番号 (PM 464)

次の判断番号は **`$PF` (本体) の最後の `Next decision: <数字>` の数字** だけが正。読み方は 1 つ:
`/usr/bin/grep -oE 'Next decision: [0-9]+' /Users/kazuaki/repos/send.sh/elidex-pm-trial/PORTFOLIO-2026-10-26.md | tail -n 1`。
出力が空なら番号を決めず、pmo に本体と直前の移送を確かめさせる。archive・索引・glob (`PORTFOLIO-*.md`) からは読まない
(archive も `Next decision:` を含み、glob では本体より後ろに並ぶ)。§7.2 の道具は最後の番号の行を本体に残し、移送の前後で同じことを確かめる。
番号は渡す時に決める。MEMORY Active state と handoff block は番号を持たず、ここを指す。

## 8. Pre-flight (提案・指示・slot 文面を出す前の 1 行チェック)

- [ ] **時刻**: epoch / tz 付きの ISO (`Z` を含む) はコマンドで JST に変換 (`date` は GNU で `date -r` は file 扱い):
      `python3 -c 'import sys,datetime as d,zoneinfo as z;a=sys.argv[1];t=d.datetime.fromtimestamp(float(a),d.timezone.utc) if a.replace(".","").isdigit() else d.datetime.fromisoformat(a.replace("Z","+00:00"));t.tzinfo or sys.exit("no timezone in "+a+" -- add one first");print(t.astimezone(z.ZoneInfo("Asia/Tokyo")).strftime("%Y-%m-%d %H:%M %Z (%a)"))' <value>`
      タイムゾーンの無い ISO は渡さない — 出所の tz を確かめて付けてから渡す (付いていなければ上の式が止まる)。
      budget の reset は `usage-pace.py status` の `next_reset` が既に JST の ISO。
- [ ] **PM が書く slot / 決定の文面**: 欠陥は仕様か elidex の原則に対して書く。「ブラウザ X と違う」を欠陥の
      定義にしない。ブラウザ名は `[evidence]` / `[contrast]` / `[history]` 等のタグ付きでだけ現れる
      (PM 421/424 [history] — PM が書いた slot が既存ブラウザ寄りの framing だった。PM 440 の
      `#11-implicit-submission-commits-change` は欠陥がブラウザの振る舞いでしか定義されていなかった)
- [ ] **lane を止める・parked と書く前**: MEMORY / lane SSoT の再開条件を読み、満たされていないことを確かめる
- [ ] **削除を提案する前**: `git grep -n '<name>' origin/main -- docs/plans` と台帳・決定ログ全体 (§3.3 の 4) を引き、
      `git diff --stat origin/main...<branch>` と branch にしか無い memo を添える。tip の sha でも grep する
- [ ] **agent の事実を使う前**: git で 1 件確かめ (`git show <sha>:<path>` 等)、どれを確かめたか書く。
      agent 間・要約と表が食い違ったら自分で測る
- [ ] **driver にツールを走らせる前**: 対象範囲 (docstring / SCHEMAS) を読み、対象外なら指示しない (PM 172)
- [ ] **toolchain を名指す測定**: `rustc --version` を同じコマンドで (session の `RUSTUP_TOOLCHAIN` が古いことがある)
- [ ] **driver の分類** (「行は要らない」「superseded」「関係ない」) は規則と突き合わせてから採る
- [ ] **lane の「済み」**: head / branch / 未 push を git で突き合わせる
- [ ] **自分の背景ジョブを止める時**: PID で止める。`pkill -f` しない (並行 lane の同じコマンドも殺す)

## 9. Kickoff brief (lane を起こす・再開させる時 — `/drive` ができるまで)

`/drive` が landing したら、brief は「`/drive` を起動、lane = <名>、最初の成果物 = <X>」と lane 固有の事実だけになる
(`drive-scope.md`)。それまでは `$PMD/KICKOFF-ci-tools-lane-2026-10-24.md` を型にし、次を必ず含める:

- 最初に repo `CLAUDE.md` を Read で通読する
- 持つもの / 持たないもの (PR・branch・memo。共有台帳は PM、書くのは pmo)
- 先に読むもの (claim として — 前提が誤っていたら PM に報告し、回避しない)
- **人間への確認はすべて PM へ** (repo `CLAUDE.md` の文言承認を含む。lane が user に直接聞くのは PM 名が
  `ListAgents` に無い時だけ)。PM 名は MEMORY Active state 冒頭
- merge の分担 (§4)、着地時の文面は `<path>:<lines>` で渡す
- PR body に `ratchet: ok (… 0 new …)` の 1 行、push gate は `EXIT=0` かつ `ERROR task failed` 無し
- handoff は lane SSoT 冒頭に (`drive-scope.md` の 10 項目)
- review の段、比例性 trigger、今日の自己 cap と disk の下限、連絡先、memory の書き先 (`$MEM` の絶対パス)
- user の状態に書く probe は隔離する: mise なら `MISE_DATA_DIR` / `MISE_CACHE_DIR` / `MISE_STATE_DIR` /
  `MISE_CONFIG_DIR` を隔離し、隔離していない state dir で `mise trust` しない。終わったら
  `~/.local/state/mise/trusted-configs` に scratchpad 向きの entry が無いことを確かめる (PM 151 / 230、
  memory `feedback_mise-probes-isolate-state-dir.md`)

起動は `orca worktree create --agent claude` / `orca agent launch` (`orca-cli` skill)、Desktop の lane は user が貼る。

## 10. Handoff

### 10.1 lane handoff

lane が自分の lane SSoT 冒頭に書く (項目は `drive-scope.md`)。PM は書かず、git と突き合わせる:
`git -C <worktree> log -1 --format=%h`、`git status --short`、`git log --oneline origin/main..HEAD`、
`git ls-remote origin <branch>`。一致したら user に「clear / restart してよい lane」を伝える。
`/clear` `/restart` は user の操作。PM が `orca terminal send` で `/clear` を打つのは、その都度 user の OK を得て、
`orca terminal read` で idle かつ許可画面が無いのを見てからだけ (Desktop lane には届かない)。

### 10.2 PM handoff (`$PF` に新しい block、pmo が書く)

いつ: 長い待ちの前、context が大きくなった時、日の区切り。block の **1 行目は固定** (§7.1 がこの行で handoff block を見分ける):

```
- **次の PM session**: `/pm` を起動する。この block は §1 の入力であって手順の代わりではない。
```

続けて:

1. **PM の未完了** (§1 step 3 で照合される一覧): 未適用の landing リスト、未 landing の PM 成果物、
   約束した報告、延期した triage (件数と延期日)、§2 で `skipped` になった行 (連続 skip の回数)、
   pmo に渡せていない「未記録の決定」(§7)、索引の「索引 → 正典」の行 (§7.3)
2. PM の session 名、pmo の有無、budget の状態 (次の判断番号は書かない — §7.4)
3. main の状態 (先頭、今日の着地、赤)
4. lane → session 名 [ref] → Orca terminal handle、lane ごとの待ちと依存
5. user の返事待ちの問い (PM が聞いたもの。lane が聞いたものは無いはず — あれば §3.4 違反として直す)
6. 今日の索引の変化: 足した行・消した行の PM 番号だけ (規則の本文は書かない — 正は索引の行が指す PM 番号の本文)

block を書いたら、同じ依頼で pmo に §7.2 の移送をさせる。

handoff を渡す引き継ぎ prompt は「`/pm` を起動」と block の位置だけを書く。要約を prompt に書かない。
compaction と clear の使い分け (日中は compaction、日の区切りで clear) は memory `feedback_compaction-vs-clear-for-pm.md`。

## 11. 止まり方

- **毎 turn の最後の段落は `止まる理由: <…>`** (lane X の報告待ち / user の判断待ち / Codex 待ち (head <sha>,
  <JST 時刻>) / budget の cap / pmo の報告待ち)。Stop hook (`announced-next-step-gate.py`) は残作業に触れて理由の
  無い停止と未来形の約束を止め、`止まる理由` を通す。
- 「次は X」と書きかけたら、書かずに X の tool call を出す。
- lane の terminator は「次に出たら user へ」を既定にしない (§3.1)。

## 12. 一般核 (後で global に移せる部分)

1. 決める前に lens block。収束したら聞かずに実行し報告。聞くのは割れた理由を書けた時だけ。
2. 振る舞いの決定は根拠の段を書く。規則の読みは両側の先例を全部示す。relay は user が見た本文で。
3. 報告・要約・agent の結果は claim。1 件は自分で確かめる。
4. 時刻はコマンドで変換。0 件が判定を決める検索には control を付ける。
5. 聞くのは常に PM、書くのは許された者。記録は記録係に、実行は委譲。
6. 文言の承認を求める前に着地先を全文読む。
7. handoff は手順の起動を名指しし、未完了一覧を繰り越し照合する。
8. turn は停止理由でだけ終える。loop には事前に終わり方を決める。
