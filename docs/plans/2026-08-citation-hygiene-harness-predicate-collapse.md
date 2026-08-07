# Citation-hygiene harness — design note: the predicate collapse

**Subject**: the re-derivation harness on branch `citation-hygiene-harness`
([PR #505](https://github.com/send/elidex/pull/505)) — its validity predicate, its block set, and its own
verification.

**This note is not a description of the diff.** `/elidex-review` on #505 returned 2 CRIT / 31 IMP / 17 MIN
(carried from that review, not re-derived here), and the decision taken was **not** to patch them one by one.
So this note answers exactly three questions and stops.

⚠ **Draft 3.** Drafts 1 and 2 were reviewed by `/elidex-plan-review` (4 CRIT, then 3 CRIT — no improvement).
Both wrote §2 and §3 **by hand against the harness**, and both reproduced, inside their own fix, the defect the
program exists to fight. Draft 3 changes the method rather than the wording: **§2 and §3 are now the output of
`rederive inventory`**, a block added in `372d6f52`. What earlier drafts got wrong is in `git log`, which is
the canonical site for it, and is not restated here.

The one conclusion that survived contact with the generator is the one this note is named for, and it is
**stronger** than drafts 1–2 stated it: the collapse is not a three-way partition of predicates. There is one.

## §0.5 / §3. Spec coverage map

**No spec surface.** This note settles a design question about a shell harness under `docs/plans/`; it touches
no spec-defined behaviour and cites no spec.

⚠ Under the **pre-A-ii** gate this memo hard-fails (heading, no table) — A-ii's §4.2.5 marker is what makes
"no spec surface" a declarable state. Same position A-iii's memo is in, and deliberate for the same reason.
Re-run rather than trusting a quoted line number: `python3 .claude/skills/elidex-plan-review/preflight.py
docs/plans/2026-08-citation-hygiene-harness-predicate-collapse.md` → `HARD FAIL`, exit 1.

## §1 The measurements this note reasons from

**Every quantity below is a command. Re-run before citing; do not carry a digit forward.** `M1`/`M2b` run in
`elidex-wt-citeaudit` (branch `webref-cite-audit-tool` — where the memos are); the rest in
`elidex-wt-harness`. Readings were taken at **`372d6f52`**, memos at **`2497eb09`**.

⚠ **This note is a `.md`; `inventory` reads only the `.sh` parts.** Committing it therefore cannot move any
number in it. That is not a courtesy — it is the only reason a count may appear here at all
(`memory/feedback_document-landing-invalidates-its-own-measurements.md`). Every figure that ranges over the
*harness* is stamped with the commit above, because editing a block does move them.

```bash
# M1 — who cites the harness at all
for m in Ai-spec-label-map Aii-gate-failure-semantics Aiii-suite-scheduler \
         B-detector-correctness C-policy-retirement umbrella; do
  printf '%-28s ' "$m"; grep -c 'rederive' "docs/plans/2026-07-citation-hygiene-$m.md"; done
# M2b — the umbrella is not a §15 memo; it declares by invocation, in two forms
grep -noE 'rederive (suites|budget|lanes)|A-rederive\.sh (suites|budget|lanes)' \
     docs/plans/2026-07-citation-hygiene-umbrella.md
# M3 — which blocks are RED on the branch that ships them  (2>&1 required: the
#      failure diagnostics go to stderr, so a stdout-only capture reads clean)
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh all 2>&1 | tee /tmp/m3.txt
#      `| tee` returns TEE's status, so the run's own exit code is PIPESTATUS[0].
echo "run exit=${PIPESTATUS[0]}"
# M4 — are those REDs verdicts, or measurement failures?
grep -c 'MEASUREMENT FAILED' /tmp/m3.txt; grep -c '!FAILED(rc=' /tmp/m3.txt
# M5 — what runs the harness outside docs/plans/
git grep -lE 'A-rederive' -- . ':!docs/plans/'
grep -rlE -e rederive -e citation-hygiene mise.toml .github/ scripts/
# M6 — the harness's own view of its size
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh selfcheck
# M7 — THE BLOCK TABLE. §2 and §3 below are this command's output.
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh inventory ../elidex-wt-citeaudit/docs/plans
```

Readings:

- **M1** — A-i 27, A-ii 19, A-iii 10, umbrella 4, **B 0, C 0**. The carve's "serves four slices" premise is
  false at the two slices meant to be downstream of it.
- **M2b** — `suites` (:82), `budget` (:124), `lanes` (:141). ⚠ The third uses the **full-path** invocation
  form; a `rederive <name>` regex alone misses it.
- **M3** — `FAILED BLOCKS: inventory(exit 1) partition(exit 1) keysets(exit 1) regions(exit 2) offline(exit 1)
  couplings(exit 1) bmemo(exit 1)`; run exit 1. **7 of 23.**
- **M4** — **0** and **0**. None is a measurement failure; each is a block correctly reporting that what it
  measures is absent. `inventory`'s is the loudest of them: it cannot find the memos, on the branch that
  ships the harness those memos describe.
- **M5** — no match, both. **0 tests, 0 CI, no `mise` task**; nothing outside `docs/plans/` names the harness.
- **M6** — `7 harness parts, 33 blocks, 23 on 'all's roster`. ⚠ Against M7's **34**: `selfcheck`'s
  line-oriented parser recognises a definition only when it closes with a bare `}`, so it drops `all()`, which
  closes `; }`. Two derivations of one quantity disagreeing, in the program whose theme is that. §4 fixes it
  by deleting the second parser, not by patching it.

## §2 Q1 — what the canonical validity predicate is

**There is exactly one, and the harness already has it.** Drafts 1 and 2 answered this with a three-kind
partition (derive / assert / instrument). That partition is **withdrawn**, and the reason is a measurement
rather than a change of taste.

**Three code signals were put in `inventory` to see whether any of them reproduces a kind assignment. None
does.** `meas` (`_measure` call sites), `vrd` (the block prints a verdict line), `cmp` (bracket comparisons
against a literal):

```bash
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh inventory ../elidex-wt-citeaudit/docs/plans \
  | awk 'NR>1 && NF>8 {print $5+0, $6, $7+0, $1}' | sort -k1,1nr
```

- `cmp` does not discriminate, because it cannot tell an **invariant** from a **liveness guard**. `couplings`'
  `[ "$n_head" = 0 ]` fails when the *subject* is wrong; `suiteset`'s `[ "$n" -gt 0 ]` and `ruleset`'s
  `[ "$n_id" != 1 ]` fail when the *measurement* is wrong. Same column, opposite meanings.
- `meas` does not discriminate either: it says which blocks route a measurement through the primitive, which
  is a fact about diligence, not about kind.
- `vrd` is the only signal that lands cleanly, and it lands on **two blocks: `couplings` and `selfcheck`.**
- The one signal that tracks the "instrument" idea at all is `cand` — **occurrences of the word "candidate" in
  the block's comments**. It is prose. Nothing in the *code* distinguishes a block running rival mechanisms
  from one measuring a quantity, which is why `inventory` labels that column and no routing tier uses it.

⇒ **A taxonomy whose discriminator is a taste judgement is not a predicate; it is a second thing to get
wrong.** Withdraw it, and the question has a single answer:

> **The harness's validity predicate is `_measure`'s: a quantity that was not measured must not be printable
> as a measurement.** The "thirteen bespoke shapes" the re-gate counted are thirteen local spellings of that
> one predicate. Two blocks carry an additional **invariant** — an expectation written down *outside* the
> block (`couplings`: K2/K3 are absolutes, "MUST BE 0"; `selfcheck`: every roster block states its own status)
> — and both already express it the same way, as a `VERDICT:` line plus a return status. There is nothing to
> unify: the exception is already uniform, and it has two members.

**What this leaves genuinely open, and what it hands off.** `_measure` closes the predicate **at the call
sites that route through it, and nowhere else** — `lanes` is the standing counter-example: 4 of its 6 commands
bypass it, and `git log --grep` exits 0 on no match, so §13's "two carve commits" limb prints nothing and
passes. That is `selfcheck`'s limitation one level up, and §4 is where it is answered.

Three sites remain unclosable by any status widening — `column:34`, `carvecolumn:57`, `remedies:207`, all
`[ "$rc" -le 1 ]`. `preflight.py` returns 1 for a real HARD FAIL, a missing fixture, an uncaught exception
**and** a failed `cd`; the discriminator is in the child's *stdout*, which those blocks print and never read.
All three are A-ii's (M7), so under §3 they leave with A-ii's part and the fix is A-ii's to make. The same is
true of `armmatrix`, whose 27 rows print `EXIT=` and bind no status: **removing the block discharges that
CRIT.** `citations` is the one that stays — it ships with A-i, it prints the authoritative §-title beside the
fixture's and never compares them, and its own comment records that nothing else would catch a fabricated
title. **Keep and fix.**

## §3 Q2 — which blocks earn existence

**Rule**: umbrella `:89` — *"a slice may not carry another slice's concern"* — applied one level deeper. **A
harness part ships in the PR of the slice that cites it, and no earlier.** M7 computes that in four tiers,
each printed beside its row so a routing decision can be checked rather than believed:

| tier | rule |
|---|---|
| **T0** | defined in the dispatcher → **kernel**. It is the invocation surface every memo cites blocks through; nothing calls it. |
| **T1** | a memo declares it → the **earliest declarer** in the forced order, **umbrella first**. |
| **T2** | else, defined in a slice part → that slice. |
| **T3** | else → the earliest ship-with among its **command-position callers**; none → kernel. |

**The umbrella ranks first and is not a slice.** It has landed, so a block it cites must exist from the first
harness PR onward. Ranked after A-i, `suites` routed to a slice with no branch while `:82` cites it today.

M7's tally at `372d6f52` — **blocks / prose lines**:

| ships with | blocks | lines | what it is |
|---|---|---|---|
| kernel | 4 | 369 | `all` `say` `selfcheck` `inventory` |
| umbrella | 6 | 396 | `_measure` `_measured` `_proto` `budget` `lanes` `suites` |
| **A-i** | 7 | 415 | `citations` `couplings` `keysets` `readers` `regions` `fixtures` `_wtscan` |
| A-ii | 10 | 335 | `column` `carvecolumn` `instruments` `remedies` `reloadstale` `armmatrix` `marker` `anchors` `timing` `_runner` |
| A-iii | 3 | 71 | `suiteset` `filters` `ruleset` |
| B | 4 | 128 | `partition` `offline` `bmemo` `staleclaims` |

**Two rows in that table are the answer, and one is a defect the table detected.**

- **`_proto` — 228 of the umbrella row's 396 lines — is a graft of A-ii's subject.** It routes here only
  because `budget` calls it, and `budget` is declared by four documents. That is M7 surfacing what drafts 1–2
  asserted in prose: **`budget` spans two slices.** The `-- preflight.py's LOGIC growth under A --` limb and
  `_proto` are A-ii's; the residual size census is the umbrella's. Splitting the limb moves 228 lines out of
  the first PR, and it is the single largest routing consequence in this note. **Open**, unchanged from draft
  2: whether the residual census still resolves A-i §8's size claims is A-i's to answer at landing.
- **A-ii + A-iii + B = 17 blocks / 534 lines, and none of those three slices has a branch.** Add `_proto` and
  it is **762 of 1714 prose lines — 44% of the harness — machinery for slices that cannot receive it.**
  M1 sharpens it: **B cites the harness zero times**, and `-B.sh`'s four blocks are declared by no memo at
  all, and two of them — `partition` and `bmemo` — are RED for a reason **A-i's landing does not fix**
  (§3's RED table routes both to B).

### The measurement that decides the ordering

M3's REDs are one class: each reads an artifact another slice creates, or an invariant another slice
discharges.

| RED block | reads | created/discharged by |
|---|---|---|
| `keysets` `regions` `offline` | `.claude/tools/_webref/spec_labels.py` — ABSENT | **A-i** |
| `partition` | `spec_labels._catalog()` — the fall-through | **B** |
| `bmemo` | `…-B-detector-correctness.md` — ABSENT | **B** |
| `couplings` | K2's two pre-existing sites | **A-i discharges them** |
| `inventory` | all six slice memos — ABSENT | **the memos are in #501** |

```bash
ls .claude/tools/_webref/spec_labels.py docs/plans/*B-detector-correctness.md
git grep -nE '\.claude/(skills|tools)/[A-Za-z0-9_-]+/[A-Za-z0-9_.-]+' -- .claude/tools/
```

The second prints `_webref/cli.py:78` and `.claude/tools/webref:5`. `couplings` — §12(3)'s exit criterion and
one of the two blocks that carries an invariant at all — ships RED and stays RED until A-i lands.

**A harness cannot be stacked before the slice it measures.** M7 now says how much of it that is: of the
1714 prose lines on this branch, **415 are A-i's and 762 belong to slices with no branch.**

### Disposition: close #505; the harness returns to #501, 762 lines lighter

Draft 2 chose *(a) #505 ships the kernel only*, having reversed away from *(b) close #505* on an objection
("`selfcheck` would range over an empty roster") that measurement showed does not exist. **(a) does not
survive the routing table either, and the reason is new**: kernel-only is **4 blocks / 369 lines**, of which
`selfcheck` and `inventory` are whole-harness checks with nothing to range over and `say` prints a header.
A PR of a dispatcher and two checks over an empty set is not a slice of the artifact; it is the artifact's
shell. And it leaves the other 1345 lines needing a destination that, for three of the five, **does not
exist**.

What the table supports instead:

1. **Close #505.** Its content minus the removals is exactly *the harness A-i needs*, which is what #501
   already carries **byte-identically**. Two PRs for one artifact, the first not self-contained — its only
   describing document, A-i §8, is in the PR that would land second — is the decision-surface duplication
   `one-issue-one-way` forbids. Closing is also the *simpler* mechanic: #501 is unblocked immediately, with
   no merge and no reconciliation. (Draft 2's §3 planned a `git merge origin/main` after #505 landed; under
   this disposition there is nothing to re-join.)
2. **Apply the removals on #501**: `-Aii.sh`, `-Aiii.sh` minus nothing it still owns, `-B.sh`, and `budget`'s
   `_proto` limb. **762 lines.** Each departure is recorded as a slot, not discarded — that was draft 2's
   strongest objection to (b) and it is answered rather than dismissed:
   - `#11-citation-hygiene-Aii-harness-part` — 10 blocks + `_proto`, and the three `rc -le 1` sites and
     `armmatrix`'s unbound rows go with them as A-ii's to fix.
   - `#11-citation-hygiene-Aiii-harness-part` — `suiteset` `filters` `ruleset`.
   - `#11-citation-hygiene-B-harness-part` — `partition` `offline` `bmemo` `staleclaims`.
   Content recoverable at **`372d6f52`**, named in each slot. ⚠ `partition` is **not** deleted for being RED:
   A-i §13.1 forbids exactly that (*"silence is what let it run broken for four commits."*). It leaves because
   it is Slice B's block, and it arrives with Slice B.
3. **`inventory` travels with the harness.** It is on this branch only; `372d6f52` cherry-picks onto
   `webref-cite-audit-tool` cleanly, the harness being otherwise byte-identical.

**What the carve bought, so that closing it is not read as a revert**: it isolated the harness for a review it
would never have received inside A-i (2 CRIT / 31 IMP / 17 MIN), and it produced this routing table. #501
comes back with the harness **44% smaller** and the ownership question settled. The carve's stated premise —
*"the harness serves four slices"* — is what M1 and M7 falsified: **B and C cite it zero times, and 415 of its
lines are A-i's own.**

## §4 Q3 — what verifies the harness

**Today: nothing** (M5). Three things verify it, and there is no fourth:

| what | property | scope |
|---|---|---|
| **`_measure`** | a quantity that was not measured is unrepresentable as a pass | per call site — **and only there** |
| **`selfcheck`** | every roster block STATES its own exit status | whole harness |
| **`inventory`** | every block routes to a slice that can receive it | whole harness |

**`inventory` is the new one, and it is the check whose absence produced 762 lines.** No count-based check
could have caught it: the harness's block count, its roster and its file layout were all internally
consistent the whole time. What was never derived was *whose* each block is.

**`selfcheck` loses its parser rather than gaining a fix.** M6-vs-M7 is two derivations of one quantity
disagreeing (33 vs 34) because `selfcheck` reads the parts with a line-oriented regex and `inventory` reads
them with `declare -f` — bash parsing bash. Patching the regex to accept `; }` keeps two parsers of one thing;
**re-expressing `selfcheck`'s check over `declare -f` deletes the second**, fixes the `; }` case by
construction, and fixes its one-way membership check (roster → defined, never defined → roster) in the same
move. That is `one-issue-one-way`, and it is authorised below.

⚠ **No `# kind:` declaration.** Draft 2 proposed one, checked by `selfcheck` in two shapes, one per kind.
§2 withdraws the taxonomy it would declare, so the annotation has nothing to say. ⚠ **And no provenance
annotation either** — a `# planted:` comment is a second spelling of *"a claim carries the command that
falsifies it"*, which is the decision surface the `stale-claim-detector` program owns. That program's v1 is
blocked at 3 CRIT and its v2 is unpushed, so this note does not defer to a grammar that may not arrive: it
introduces **no** annotation, and a block's expectation is named in its header, as `couplings`' and
`selfcheck`'s already are.

**Decision: no CI wiring, and this closes the question.** The harness's consumers are memos under review and
its readers are reviewers, who run `all`. A script that creates git worktrees, calls `gh api` and reaches the
network does not belong in every lane's gate for a `docs/plans/` artifact; the scheduling concern that *is*
real is A-iii's, over the Python suites.

## §5 Claims vs checks

| claim | check | status |
|---|---|---|
| B and C cite the harness zero times | M1 | CHECKED |
| 7 of 23 roster blocks RED at `372d6f52`; 0 measurement failures | M3, M4 | CHECKED |
| each RED reads an artifact another slice creates / discharges | §3's `ls` + `git grep` + M1 | CHECKED |
| nothing verifies the harness | M5 | CHECKED |
| two derivations of the block count disagree (33 / 34) | M6 vs M7 | CHECKED |
| no code signal reproduces a kind assignment | M7's `meas` / `vrd` / `cmp` columns | CHECKED |
| exactly two blocks carry an invariant | M7's `vrd` column → `couplings` `selfcheck` | CHECKED |
| 762 of 1714 prose lines belong to slices with no branch | M7's ships-with tally (A-ii + A-iii + B + `_proto`) | CHECKED |
| #501's harness is this branch's minus `inventory` | `git diff --stat webref-cite-audit-tool 372d6f52 -- 'docs/plans/*A-rederive*'` → `-integrity.sh` +231 and the dispatcher's roster line, nothing else | CHECKED |
| `armmatrix` binds no row's status | 27 rows, all `EXIT=1`, block exits 0 (`sed -n '/^=== armmatrix ===/,/^=== suites ===/p' /tmp/m3.txt \| grep -oE 'EXIT=[0-9]+' \| sort \| uniq -c`) | CHECKED |
| `git log --grep` exits 0 on no match, so `lanes`' §13 limb passes silently | `git log --oneline --grep='zzz-no-such-subject-zzz'; echo $?` → 0 | CHECKED |
| A-i §8's size claims survive `budget` losing the `_proto` limb | — | **UNCHECKED** — A-i's, at landing (§3) |

## §6 What this note authorises, and what it does not

**Authorises** (after `/elidex-plan-review` passes on this draft): closing #505; cherry-picking `372d6f52`
onto `webref-cite-audit-tool`; removing `-Aii.sh`, `-B.sh`, A-iii's three blocks and `budget`'s `_proto` limb
under the three named slots; adding the missing comparison to `citations`; and re-expressing `selfcheck`'s
check over `declare -f` so the harness has one parser.

**Does not authorise**: deleting a block without a slot naming its recovery commit; introducing a `# kind:`
or provenance annotation; wiring the harness into CI; touching A-ii's `rc -le 1` sites or `armmatrix` (they
leave with A-ii's part, and arrive back as A-ii's to fix); or patching the 33 `/elidex-review` findings
individually — those on removed blocks are discharged by removal, and the two that survive are named above.

**Status registers this changes**, since draft 2 correctly noted nothing had authorised editing them and the
disposition has now moved: `MEMORY.md`'s L3 bullet (which records #501 as blocked on #505),
`project_citation-hygiene-program.md`'s carve section, and `active-lane-detail.md:97-98`. They are updated at
execution, not now.
