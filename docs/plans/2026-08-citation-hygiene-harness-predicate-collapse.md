# Citation-hygiene harness — the predicate collapse, and what the block table says

**Subject**: the re-derivation harness on branch `citation-hygiene-harness`
([PR #505](https://github.com/send/elidex/pull/505)) — its validity predicate, its block ownership, and its
own verification. **Analysis only.** What to *do* about #505 is a separate memo,
`2026-08-citation-hygiene-harness-disposition.md`, and this note is its input.

⚠ **Draft 4, and the split is the change.** Drafts 1–3 were reviewed by `/elidex-plan-review`
(4 CRIT → 3 CRIT → **9 CRIT**). The count going up is not the signal; **where the findings landed** is.
R1 and R2 hit §2 and §3 for being hand-written. Draft 3 answered that by promoting the tables into a harness
block, `rederive inventory` — and three axes then verified, independently and cell by cell, that §2 and §3
really are its output. Every one of R3's nine CRITs landed in §6, the *execution plan*. Axis 5 put it exactly:
**"§2/§3 stopped being hand-written; §6 has not."**

So §6 leaves. A design analysis and a PR-disposition plan are two slices, and CLAUDE.md's edge-dense rule
says so: the withdrawn §6 bundled PR topology, a 762-line removal, harness self-verification semantics and a
CRIT detector fix into one authorised action set, and did not declare the trigger. R3's CRITs are addressed in
the disposition memo, which is where they belong. What earlier drafts got wrong is in `git log`.

## §0.5 / §3. Spec coverage map

**No spec surface in this note.** It settles what the harness's validity predicate is and who owns each
block; it changes no spec-defined behaviour and cites no spec.

⚠ **This is a boundary, not an exemption.** One block *does* have spec content — `citations` runs four
WHATWG/W3C §-number↔title lookups, and the fix it needs (§2) is precisely a comparison of §-titles. Those four
pairs are the disposition memo's to carry as a §3 table, since it is the memo that authorises the change.
Verified against webref at the time of writing, so the successor memo starts from ground truth rather than
from the fixture:

| Spec section | Step | Branch | Touch | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21 Constraints | title compare | fixture `labelled`/`dedup`/`malformed` | `citations` (`-common.sh:85`) | ✓ | no |
| WHATWG HTML §4.10.21.2 Constraint validation | title compare | fixture `labelled` | `citations` (`-common.sh:86`) | ✓ | no |
| WHATWG Fetch §2.2.5 Requests | title compare | fixture `alias` | `citations` (`-common.sh:87`) | ✓ | no |
| CSSOM View 1 §4.2 The MediaQueryList Interface | title compare | fixture `allunmapped`/`malformed` | `citations` (`-common.sh:88`) | ✓ | no |

**Breadth**: K=3 specs, M=4 entries (verified — the table above is the complete set;
`grep -n 'webref heading --exact' docs/plans/2026-07-citation-hygiene-A-rederive-common.sh | wc -l` → 4,
at `-common.sh:85-88`) → single PR.

⚠ **`CSSOM View 1` is unrecognized by the gate, and that is the point rather than a defect.** The
pre-A-i pinned map has no key for it (`python3 -c "import preflight; print([k for k in
preflight.SPEC_LABEL_REVERSE if 'cssom' in k.lower()])"` → `[]`), which is exactly why `-common.sh:37-45`
picks `CSSOM VIEW` for the `allunmapped` fixture — *"absent from the 24-key pinned map, so this is
all-unmapped AFTER A"*. A row that resolves would not exercise the state the fixture exists for.
Re-derive the titles rather than trusting the cells: `.claude/tools/webref heading --exact html 4.10.21`,
`… html 4.10.21.2`, `… fetch 2.2.5`, `… cssom-view-1 4.2`.

## §1 The measurements this note reasons from

**Every quantity below is a command. Re-run before citing; do not carry a digit forward.** `M1`/`M2b` run in
`elidex-wt-citeaudit` (branch `webref-cite-audit-tool` — where the memos are); the rest in
`elidex-wt-harness`. Readings taken at **`945dd03a`**, memos at **`2497eb09`**.

⚠ **Why a count may appear here at all.** `inventory` resolves its part set by hardcoded stem
(`-integrity.sh` `PARTS`) and its memo set by hardcoded filename, and `selfcheck` globs
`…A-rederive*.sh` only — **no reader of either check can reach this note's filename**, so committing this
note cannot move a number in it. (Draft 3 justified the same conclusion by saying `inventory` "reads only the
`.sh` parts", which is false: it reads six memo `.md` files, as M4 in this very section says.) Every figure
that ranges over the *harness* is stamped with the commit above, because editing a block does move them —
`945dd03a` is three commits past draft 3's `372d6f52`, and every figure below moved.
Ref: `memory/feedback_document-landing-invalidates-its-own-measurements.md`.

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
#      ⚠ run this under bash. `| tee` returns TEE's status, so the run's own code is
#      PIPESTATUS[0] -- and zsh spells that `pipestatus`, 1-indexed, expanding
#      `${PIPESTATUS[0]}` to the EMPTY STRING with no diagnostic. A command that did
#      not report, reading as no problem, is `_measure`'s charter inverted.
echo "run exit=${PIPESTATUS[0]}"
# M4 — are those REDs verdicts, or measurement failures?
grep -c 'MEASUREMENT FAILED' /tmp/m3.txt; grep -c '!FAILED(rc=' /tmp/m3.txt
# M5 — what INVOKES the harness outside docs/plans/
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
  measures is absent. `inventory`'s is the loudest: it cannot find the memos, on the branch that ships the
  harness those memos describe.
- **M5** — no match, both. ⚠ This establishes exactly one thing: **nothing outside `docs/plans/` invokes the
  harness** — no CI job, no `mise` task, no test. It does **not** establish "nothing verifies the harness";
  §4 names three verifiers and all three run today. Draft 3 marked the wider claim CHECKED against this
  narrower command.
- **M6** — `7 harness parts, 33 blocks, 23 on 'all's roster`. ⚠ Against M7's **34**: `selfcheck`'s
  line-oriented parser recognises a definition only when it closes with a bare `}`, so it drops `all()`, which
  closes `; }`. Two derivations of one quantity disagreeing, in the program whose theme is that.

## §2 Q1 — what the canonical validity predicate is

**There is exactly one, and the harness already has it.** Drafts 1 and 2 answered with a three-kind partition
(derive / assert / instrument). That partition is **withdrawn**, on a measurement rather than a change of
taste, and R3 did not disturb the withdrawal.

Three **code** signals were put in `inventory` to test whether any reproduces a kind assignment. None does —
`meas` (`_measure` call sites), `vrd` (the block prints a verdict line), `cmp` (bracket comparisons):

```bash
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh inventory ../elidex-wt-citeaudit/docs/plans \
  | awk 'NR>1 && NF>8 {print $5+0, $6, $7+0, $1}' | sort -k1,1nr
```

- `cmp` does not discriminate, because it cannot tell an **invariant** from a **liveness guard**. `couplings`'
  `[ "$n_head" = 0 ]` fails when the *subject* is wrong; `suiteset`'s `[ "$n" -gt 0 ]` and `ruleset`'s
  `[ "$n_id" != 1 ]` fail when the *measurement* is wrong. Same column, opposite meanings.
- `meas` does not discriminate either: it reports which blocks route a measurement through the primitive,
  which is a fact about diligence, not about kind.
- `vrd` is the only signal that lands cleanly, and it lands on **two blocks: `couplings` and `selfcheck`**.
- The "instrument" idea has **no** code signal at all. It is written down once, in prose, at `-Aii.sh:64-76`
  (*"`instruments` measures all three candidates so the choice is not taken on faith"*). ⚠ Draft 3 offered a
  `cand` column counting that word instead of citing the site; the column's three hits for its own row were
  all inside `inventory`, so it ranked **itself** joint-top, above `instruments`. A metric that ranks itself
  is not evidence. Removed in `ff44f30c`; the citation is the claim.

⇒ **A taxonomy whose discriminator is a taste judgement is not a predicate; it is a second thing to get
wrong.** Withdraw it, and the question has one answer:

> **The harness's validity predicate is `_measure`'s: a quantity that was not measured must not be printable
> as a measurement.** Every locally-spelled shape in the harness is a spelling of that one predicate. Two
> blocks carry an additional **invariant** — an expectation written down *outside* the block (`couplings`:
> K2/K3 are absolutes, "MUST BE 0"; `selfcheck`: every roster block states its own status) — and both already
> express it the same way, as a `VERDICT:` line plus a return status. There is nothing to unify: the exception
> is already uniform, and it has two members.

⚠ Draft 3 wrote *"the thirteen bespoke shapes the re-gate counted"*. `grep -rn 'thirteen'` over both trees
returns only the note itself; the figure has no locatable source and is dropped rather than re-asserted. The
claim above does not need it — it is a statement about the two exceptions, which M7 measures.

**What this leaves genuinely open.** `_measure` closes the predicate **at the call sites that route through
it, and nowhere else**. `lanes` is the standing counter-example, and larger than draft 3 said: of nine
substantive commands, two route through `_measure` (M7: `meas=2`); four are `|| failed=1`-guarded bypasses,
and `git log --grep` exits 0 on no match, so §13's "two carve commits" limb prints nothing and passes; the
remaining three — two `$(git worktree list …)` substitutions and a `git -C … rev-parse` — bypass `_measure`
**unguarded**, so a failure yields an empty loop, `failed` stays 0 and the block returns 0. `lanes` ships with
the umbrella (§3), i.e. in the first harness PR. **This note does not fix it and does not hand it to §4**;
it is a named obligation for the disposition memo.

Three further sites are unclosable by any status widening — `-Aii.sh:34` (`column`), `:57` (`carvecolumn`),
`:207` (`remedies`, spelled `[ "$pfrc" -le 1 ]`). `preflight.py` returns 1 for a real HARD FAIL, a missing
fixture, an uncaught exception **and** a failed `cd`; the discriminator is in the child's *stdout*, which
those blocks print and never read. `armmatrix` is the same class from the other side: 27 rows print `EXIT=`
and the block binds none of them. All four are A-ii's by §3, and two of them are Codex findings **publicly
promised a fix in #505** — the disposition memo owns re-homing that promise.

`citations` is the one that is neither: it ships with A-i, it prints the authoritative §-title beside the
fixture's and never compares them, and its own comment records that nothing else would catch a fabricated
title. **Keep and fix**, against §0.5's table.

## §3 Q2 — which blocks earn existence, and why nothing can act on the answer yet

**Rule**: umbrella `:89` — *"A slice may not carry another slice's concern."* ⚠ **Stated as an extension, not
sourced as an application.** `:89` continues *"Specifically: A may not change detector semantics; B may not
edit review policy; C may not repair citations"* — every instance constrains what a slice may **change**.
Reading it as a rule about **which PR an artifact ships in** is a new rule. It is a good one and the same
lens generates it, but §3 asserts it rather than inheriting it, and the disposition memo must argue it rather
than cite `:89`. (Two tells that the inheritance does not hold: T1 ranks the **umbrella** first, and the
umbrella is not a slice at all.)

M7 computes ownership in four tiers, each printed beside its row so a routing decision can be checked:

| tier | rule |
|---|---|
| **T0** | defined in the dispatcher → **kernel**. It is the invocation surface every memo cites blocks through; nothing calls it. |
| **T1** | a memo declares it → the **earliest declarer** in the forced order, **umbrella first** (it has landed, so a block it cites must exist from the first harness PR onward). |
| **T2** | else, defined in a slice part → that slice. |
| **T3** | else → the earliest ship-with among its **command-position callers**, assigned only once **every** caller is resolved; none → kernel. |

⚠ T3 was **order-dependent** until `ff44f30c`: it assigned from the callers resolved so far, over an unordered
set, and gave two different answers across `PYTHONHASHSEED`. Fixed and verified deterministic across seeds
0–9. The rule above is now the rule implemented.

M7's tally at `945dd03a` — **blocks / block-body lines**:

| ships with | blocks | lines | what it is |
|---|---|---|---|
| kernel | 4 | 371 | `all` `say` `selfcheck` `inventory` |
| umbrella | 6 | 396 | `_measure` `_measured` `_proto` `budget` `lanes` `suites` |
| **A-i** | 7 | 415 | `citations` `couplings` `keysets` `readers` `regions` `fixtures` `_wtscan` |
| A-ii | 10 | 335 | `column` `carvecolumn` `instruments` `remedies` `reloadstale` `armmatrix` `marker` `anchors` `timing` `_runner` |
| A-iii | 3 | 71 | `suiteset` `filters` `ruleset` |
| B | 4 | 128 | `partition` `offline` `bmemo` `staleclaims` |

⚠ **These are block-body lines; the harness is larger, and neither figure is transcribed here.**
`inventory` prints the reconciliation itself — `LINES: <total> in <n> files = <attributed> + <unattributed>`,
cross-checkable against `wc -l docs/plans/2026-07-citation-hygiene-A-rederive*.sh | tail -1`. The remainder is
each part's preamble and the dispatcher outside `all`, including `-integrity.sh`'s 82-line `_measure`
rationale — the text §2's whole answer rests on. A removal deletes **files**, so any share-of-the-harness
figure is over the file total, not over the attributed total.

⚠ **Draft 4 transcribed those digits and they were stale on arrival.** They were read at `ff44f30c`; the very
next commit added 16 lines to `inventory`, and the note stamped itself with *that* commit while carrying the
earlier reading — including in a `§5` row marked CHECKED and a parenthetical reading "verified 2026-08-08".
Four reviewers found it independently. The class is `feedback_verified-claims-go-stale-under-own-later-edits`
and the only fix that holds is the one applied here: **do not carry the digit.** The `ships-with` table above
is kept because its rows are block ownership, which the disposition memo acts on; the `kernel` row's line
count moves with any kernel edit, so read it from the command, not from this page.

### The finding: the routing unit is not the shipping unit

Every column above routes a **block**. A PR adds and removes **files**. Those are different partitions, and
M7 now measures the disagreement:

```
   _measure      lives in integrity (no slice)  ships with umbrella    22 lines
   _measured     lives in integrity (no slice)  ships with umbrella     4 lines
   _proto        lives in common    (no slice)  ships with umbrella   228 lines
   _wtscan       lives in common    (no slice)  ships with A-i         40 lines
   budget        lives in common    (no slice)  ships with umbrella    59 lines
   citations     lives in common    (no slice)  ships with A-i         21 lines
   couplings     lives in common    (no slice)  ships with A-i        130 lines
   fixtures      lives in common    (no slice)  ships with A-i         53 lines
   lanes         lives in common    (no slice)  ships with umbrella    40 lines
   suites        lives in Aiii      (A-iii   )  ships with umbrella    43 lines
   10 of 34 blocks / 640 lines cannot be moved or removed at FILE granularity.
```

**This is Q2's real answer, and it is why draft 3's §6 was wrong rather than merely under-specified.** It
authorised a file-granular removal against a block-granular table. Three of R3's nine CRITs are this one gap,
and the program memo had named it as draft 3's third precondition — *reconcile the routing unit with the
shipping unit* — which draft 3 did not attempt. `suites` is the sharpest instance: `-Aiii.sh` holds it,
the umbrella cites it at `:82`, and A-i §13 already carries its relocation to `-common.sh` as **owed**.

A second, independent form of the same gap: **the block set is written down four to six times** — the
dispatcher's `for _part in …` source loop, `all`'s 23-name roster, `inventory`'s `PARTS`, `AUTHOR_LOCAL`, and
the dispatcher's header prose. Measured in a `git clone --local` sandbox, deleting `-Aii.sh` and `-B.sh` and
narrowing only the source loop: `selfcheck` reports **12 blocks that no longer exist** as failing its
return-discipline check (9 from `-Aii.sh`, 3 from `-B.sh`), and `inventory` cannot source at all. ⚠ Draft 4
said 15, which is the count when A-iii's three blocks go too — a digit taken from a review report and
attached to a different deletion set than the one that produced it. The deletion set is now in the sentence. Neither is a wrong answer to a right question;
both are what happens when one fact has five homes.

⇒ **Nothing is authorised to move until that is collapsed.** The collapse is the disposition memo's first
section, not this note's.

### The measurement that decides any future ordering

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
# run under bash: zsh aborts the `ls` on `no matches found` before it reports anything
ls .claude/tools/_webref/spec_labels.py docs/plans/*B-detector-correctness.md
git grep -nE '\.claude/(skills|tools)/[A-Za-z0-9_-]+/[A-Za-z0-9_.-]+' -- .claude/tools/
```

The second prints `_webref/cli.py:78` and `.claude/tools/webref:5`. `couplings` — §12(3)'s exit criterion and
one of the two blocks carrying an invariant at all — ships RED and stays RED until A-i lands.
**A harness cannot be stacked before the slice it measures.**

## §4 Q3 — what verifies the harness

Nothing **runs** it outside `docs/plans/` (M5). Three things verify it, and there is no fourth:

| what | property | scope |
|---|---|---|
| **`_measure`** | a quantity that was not measured is unrepresentable as a pass | per call site — **and only there** |
| **`selfcheck`** | every roster block STATES its own exit status | whole harness |
| **`inventory`** | every block routes to a slice that can receive it, and to the file it lives in | whole harness |

**`inventory` is the new one, and it is the check whose absence produced the 640-line gap above.** No
count-based check could have caught it: the block count, the roster and the file layout were each internally
consistent the whole time. What was never derived was *whose* each block is.

**`selfcheck` and `inventory` do not have one parser, and the note must stop claiming they will.** Draft 3
said re-expressing `selfcheck` over `declare -f` "deletes the second parser". Measured: there are two
`DEF = re.compile(…)` sites in `-integrity.sh`, and the one that would be deleted is `selfcheck`'s. The
survivor is `inventory`'s **line-oriented** parser over the raw files, and it is the source of every `ln` in
§3 — `declare -f` strips comments, so it cannot count prose lines at all. The honest collapse is narrower and
still worth taking: `selfcheck`'s **body** analysis moves to `declare -f` (bash parsing bash; verified that
normalisation preserves the trailing `return` in the multi-line, short and one-liner forms), and the raw-file
parser stays as the single **prose** reader. Two readers, two questions, one home each — not one parser.

⚠ **No `# kind:` declaration and no provenance annotation.** §2 withdraws the taxonomy a `# kind:` comment
would declare. A `# planted:` comment is a second spelling of *"a claim carries the command that falsifies
it"*, the decision surface the `stale-claim-detector` program owns; that program's v1 is blocked at 3 CRIT
and v2 is unpushed, so this note defers to no grammar that may not arrive. A block's expectation is named in
its header, as `couplings`' and `selfcheck`'s already are.

**No CI wiring.** The harness's consumers are memos under review and its readers are reviewers, who run `all`.
A script that creates git worktrees, calls `gh api` and reaches the network does not belong in every lane's
gate for a `docs/plans/` artifact; the scheduling concern that *is* real is A-iii's, over the Python suites.

## §5 Claims vs checks

Rows are marked UNCHECKED rather than omitted (umbrella `:92`).

| claim | check | status |
|---|---|---|
| B and C cite the harness zero times | M1 | CHECKED |
| 7 of 23 roster blocks RED at `945dd03a`; 0 measurement failures | M3, M4 | CHECKED |
| each RED reads an artifact another slice creates / discharges | §3's `ls` + `git grep` + M1 | CHECKED |
| nothing outside `docs/plans/` **invokes** the harness | M5 | CHECKED — and this is all M5 shows |
| two derivations of the block count disagree (33 / 34) | M6 vs M7 | CHECKED |
| no **code** signal reproduces a kind assignment | M7's `meas` / `vrd` / `cmp` columns | CHECKED |
| the "instrument" idea exists only in prose | `-Aii.sh:64-76`, quoted in §2 | CHECKED |
| exactly two blocks carry an invariant | M7's `vrd` column → `couplings` `selfcheck` | CHECKED |
| 10 blocks / 640 lines cannot move at file granularity | M7's routing-vs-shipping report | CHECKED |
| the harness is 1935 lines = 1716 attributed + 219 unattributed | M7's LINES line, vs `wc -l …A-rederive*.sh` | CHECKED |
| T3 is order-independent | 10 `PYTHONHASHSEED` values, identical output (`ff44f30c`) | CHECKED |
| `declare -f` preserves the trailing `return` | `declare -f couplings\|filters\|_measured \| tail -2` | CHECKED |
| `lanes` bypasses `_measure` at 7 of 9 commands, 3 unguarded | source read + M7 `meas=2` | CHECKED |
| #501's harness is this branch's minus the `inventory` commits | `git diff --stat webref-cite-audit-tool HEAD -- 'docs/plans/*A-rederive*'` | CHECKED |
| `citations`' four §-title pairs are the authoritative ones | `.claude/tools/webref heading --exact …` ×4 (§0.5) | CHECKED |
| **the `declared by` column is stable under memo edits** | — | **UNCHECKED, and known false.** It parses §15 **prose**: a code span naming ≥2 known blocks is a declaration list. A purely typographic rewrite of A-i's §15 (one span per name) moves A-i from 7/415 to 3/171. The memos are under active revision on another branch and every figure here depends on them. **The disposition memo must make the declaration machine-readable before acting on any ships-with cell.** |
| A-i §8's layout paragraph after any harness change | — | **UNCHECKED.** §8 is the program's declared single home for the layout and states part count, per-part sizes, the 1711 total, the block count and the `_measure` call-site census. The `inventory` commits alone already falsify all of them. A-i's, at landing. |

## §6 What this note settles, and what it hands over

**Settles** (no action authorised, because none is needed to state a fact):

1. The validity predicate is `_measure`'s, singular; the two invariant-carrying blocks are `couplings` and
   `selfcheck` and are already uniform. The J1/J2/J3 taxonomy is withdrawn.
2. Block ownership is what `rederive inventory` prints, by four stated tiers.
3. The routing unit and the shipping unit disagree at 10 blocks / 640 lines, and the block set has four to six
   homes. **No file-granular action on the harness is sound until that is collapsed.**
4. `selfcheck` and `inventory` answer two different questions and keep two readers; the collapse available is
   `selfcheck`'s body analysis moving to `declare -f`.
5. No CI wiring, no `# kind:` annotation, no provenance grammar.

**Hands to `2026-08-citation-hygiene-harness-disposition.md`**, which is gated by its own
`/elidex-plan-review`: what happens to #505; the routing/shipping collapse and the single home for the block
set; making the `declared by` column machine-readable; `citations`' comparison against §0.5's table; `lanes`'
seven bypasses; the four A-ii sites including the two Codex findings promised a fix in #505; the complete
register sweep; and whatever defer slots survive that work, with own/pre-existing classification, trigger and
calendar date, registered in `project_open-defer-slots.md`.

**Explicitly does not authorise**: deleting or moving any block or file; closing or retargeting any PR;
editing any status register.
