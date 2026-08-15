# Citation-hygiene harness — the predicate collapse, and what the block table says

**Subject**: the re-derivation harness on branch `citation-hygiene-harness`
([PR #505](https://github.com/send/elidex/pull/505)) — its validity predicate, its block ownership, and its
own verification. **Analysis only.** What to *do* about #505 is a separate memo,
`2026-08-citation-hygiene-harness-disposition.md`, and this note is its input.

**Why the execution plan is not here.** A design analysis and a PR-disposition plan are two slices, and
CLAUDE.md's edge-dense rule says so: the execution plan that once sat in §6 bundled PR topology, a 762-line
removal, harness self-verification semantics and a CRIT detector fix into one authorised action set without
declaring the trigger. It moved to the disposition memo, which declares it. **This note authorises nothing.**

**Every quantity here is a command, and no digit is carried forward.** That rule is not stylistic: the
subjects of these figures — the harness on this branch, memos on another — move under the note, and a
transcribed tally is falsified by the next commit
(`memory/feedback_verified-claims-go-stale-under-own-later-edits.md`).

## §0.5 / §3. Spec coverage map

**No spec surface in this note.** It settles what the harness's validity predicate is and who owns each
block; it changes no spec-defined behaviour and cites no spec.

⚠ **This is a boundary, not an exemption.** One block *does* have spec content — `citations` runs four
WHATWG/W3C §-number↔title lookups, and the fix it needs (§2) is precisely a comparison of §-titles. Those four
pairs are the disposition memo's to carry as a §3 table, since it is the memo that authorises the change.
Verified against webref, so the successor memo starts from ground truth rather than from the fixture:

| Spec section | Step | Branch | Touch | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21 Constraints | title compare | **read the fixtures, not this cell** (see the disposition's §0.5) | `citations` | ✓ | no |
| WHATWG HTML §4.10.21.2 Constraint validation | title compare | fixture `labelled` | `citations` | ✓ | no |
| WHATWG Fetch §2.2.5 Requests | title compare | fixture `alias` | `citations` | ✓ | no |
| CSSOM View 1 §4.2 The MediaQueryList Interface | title compare | fixture `allunmapped`/`malformed` | `citations` | ✓ | no |

**Breadth**: K=3 specs, M=4 entries — the complete set. The command finds the call site, rather than this
table naming a line number the collapse is about to move:
`grep -n 'webref heading --exact' docs/plans/2026-07-citation-hygiene-A-rederive-*.sh` → 4.

⚠ **Two resolvers, and the column that matters is not the one this table shows.** All four §-numbers
resolve in **webref** (`.claude/tools/webref heading --exact html 4.10.21`, `… html 4.10.21.2`,
`… fetch 2.2.5`, `… cssom-view-1 4.2` — rc=0 each; re-derive the titles rather than trusting the cells).
What some of them do not resolve in is `preflight.SPEC_LABEL_REVERSE`, the **gate's pinned map**, which is a
different resolver — and the Spec column above carries each spec's *authoritative* label while the Branch
column names a fixture that deliberately carries a **different** spelling, because aliasing is what the
fixture set exists to exercise. Measured on the labels the fixtures actually write:

```bash
(cd .claude/skills/elidex-plan-review && python3 -c "
import preflight as p
for lab in ['WHATWG HTML','HTML','Fetch','CSSOM VIEW']: print(lab, '->', p.shortname_from_label(lab))
print('pinned-map keys:', len(p.SPEC_LABEL_REVERSE))")
```

`WHATWG HTML` and `HTML` resolve; **`Fetch` and `CSSOM VIEW` do not.** That is the point rather than a
defect — a row that resolved would not exercise the state its fixture exists for — and the count is not this
page's to carry. ⚠ The harness's own comment beside the `allunmapped` fixture calls it a *"24-key pinned
map"*; the command above measures fewer. Correcting the comment is the disposition memo's §5, not this
note's.

## §1 The measurements this note reasons from

**Every quantity below is a command. Re-run before citing; do not carry a digit forward.** `M1`/`M2b` run in
`elidex-wt-citeaudit` (branch `webref-cite-audit-tool` — where the memos are); the rest in
`elidex-wt-harness`.

⚠ **Why a count may appear here at all.** `inventory` resolves its part set by hardcoded stem and its memo
set by hardcoded filename, and `selfcheck` globs `…A-rederive*.sh` only — **no reader of either check can
reach this note's filename**, so committing this note cannot move a number in it. (`inventory` does read six
memo `.md` files, as M4 says; "it reads only the `.sh` parts" is not the reason.) Any figure ranging over the
*harness* must still be stamped with the commit it was read at, because editing a block moves it.
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

- **M1** — **B and C cite the harness zero times.** The carve's "serves four slices" premise is false at the
  two slices meant to be downstream of it.
- **M2b** — `suites`, `budget`, `lanes`. ⚠ One uses the **full-path** invocation form; a `rederive <name>`
  regex alone misses it.
- **M3** — seven RED blocks: `inventory` `partition` `keysets` `regions` `offline` `couplings` `bmemo`; run
  exit 1.
- **M4** — **0** and **0**. None is a measurement failure; each is a block correctly reporting that what it
  measures is absent. `inventory`'s is the loudest: it cannot find the memos, on the branch that ships the
  harness those memos describe.
- **M5** — no match, both. ⚠ This establishes exactly one thing: **nothing outside `docs/plans/` invokes the
  harness** — no CI job, no `mise` task, no test. It does **not** establish "nothing verifies the harness";
  §4 names the verifiers and they all run today.
- **M6 vs M7** — **two derivations of the block count disagree.** `selfcheck`'s line-oriented parser
  recognises a definition only when it closes with a bare `}`, so it drops `all()`, which closes `; }` — but
  that is no longer the whole of the gap. Read both; do not carry the pair.

## §2 Q1 — what the canonical validity predicate is

**There is exactly one, and the harness already has it.** The earlier three-kind partition (derive / assert /
instrument) is **withdrawn**, on a measurement rather than a change of taste.

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
  (*"`instruments` measures all three candidates so the choice is not taken on faith"*). ⚠ A `cand` column
  counting that word was tried and removed: its three hits for its own row were all inside `inventory`, so it
  ranked **itself** joint-top, above `instruments`. A metric that ranks itself is not evidence; the citation
  is the claim.

⇒ **A taxonomy whose discriminator is a taste judgement is not a predicate; it is a second thing to get
wrong.** Withdraw it, and the question has one answer:

> **The harness's validity predicate is `_measure`'s: a quantity that was not measured must not be printable
> as a measurement.** Every locally-spelled shape in the harness is a spelling of that one predicate. Two
> blocks carry an additional **invariant** — an expectation written down *outside* the block (`couplings`:
> K2/K3 are absolutes, "MUST BE 0"; `selfcheck`: every roster block states its own status) — and both already
> express it the same way, as a `VERDICT:` line plus a return status. There is nothing to unify: the exception
> is already uniform, and it has two members.

**What this leaves genuinely open.** `_measure` closes the predicate **at the call sites that route through
it, and nowhere else**. `lanes` is the standing counter-example: of nine substantive commands, two route
through `_measure` (M7: `meas=2`); four are `|| failed=1`-guarded bypasses, and `git log --grep` exits 0 on no
match, so §13's "two carve commits" limb prints nothing and passes; the remaining three — two
`$(git worktree list …)` substitutions and a `git -C … rev-parse` — bypass `_measure` **unguarded**, so a
failure yields an empty loop, `failed` stays 0 and the block returns 0. `lanes` ships with the umbrella (§3),
i.e. in the first harness PR. **This note does not fix it and does not hand it to §4**; it is a named
obligation for the disposition memo.

Three further sites are unclosable by any status widening — `-Aii.sh:34` (`column`), `:57` (`carvecolumn`),
`:207` (`remedies`, spelled `[ "$pfrc" -le 1 ]`). `preflight.py` returns 1 for a real HARD FAIL, a missing
fixture, an uncaught exception **and** a failed `cd`; the discriminator is in the child's *stdout*, which
those blocks print and never read. `armmatrix` is the same class from the other side: 27 rows print `EXIT=`
and the block binds none of them. **All four are A-ii's** by §3. Of the two Codex findings publicly promised
a fix in #505, **R3-F2 is `column`, which is in this set; R3-F3 is `ruleset`, which ships A-iii and is not.**
The disposition memo's §5 routes both.

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
| **T3** | else → read its **command-position callers**, assigned only once **every** caller is resolved. Callers in **more than one** group → kernel, because a block two groups call is shared infrastructure; exactly one → that group; none → kernel. |

⚠ **T3 is order-independent and layer-representable only because both were fixed and verified.** It once
assigned from the callers resolved so far, over an unordered set, and gave two different answers across
`PYTHONHASHSEED` (now verified deterministic across seeds 0–9); and it was "earliest caller wins", which made
the measurement primitive's own layer unrepresentable — declaring `_measure` the `kernel` it is went
binding-RED. The row above is the rule implemented at HEAD. ⚠ It is **not** the rule after PR-1a: the
boundary test currently ranges over `ORDER`, which excludes `kernel`, so a `kernel` caller is dropped from the
count of groups crossed. The disposition memo's D13 measures the widening and carries it; this note only
records that the row will move.

**M7's tally is a command, not a table here.** The groups are `kernel`, `umbrella`, `A-i`, `A-ii`, `A-iii`,
`B`; who is in each, and how large each is, comes from:

```bash
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh inventory ../elidex-wt-citeaudit/docs/plans
#   the `ships-with (blocks, prose lines)` line, and the `ships` column per row
```

⚠ **Those are block-body lines; the harness is larger.** `inventory` prints the reconciliation itself —
`LINES: <total> in <n> files = <attributed> + <unattributed>` — cross-checkable against
`wc -l docs/plans/2026-07-citation-hygiene-A-rederive*.sh | tail -1`. The remainder is each part's preamble
and the dispatcher outside `all`, including the `_measure` rationale that §2's whole answer rests on. A
removal deletes **files**, so any share-of-the-harness figure is over the file total, not the attributed one.

### The finding: the routing unit is not the shipping unit

Every column above routes a **block**. A PR adds and removes **files**. Those are different partitions, and
M7 measures the disagreement:

```bash
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh inventory ../elidex-wt-citeaudit/docs/plans
#   read the two lists under `ROUTING UNIT (block) vs SHIPPING UNIT (file)`
```

`MOVE LIST` is the declared blocks whose file contradicts their declaration — what a PR can carry out.
`NO VERDICT` is the undeclared ones, where the tiers are guessing. ⚠ **They are printed apart because they
authorise different things**: merged, a list of guesses reads as a work list, which is exactly how one was
quoted as one.

**This is Q2's real answer, and it is why a file-granular removal was wrong rather than merely
under-specified.** It authorised a file-granular action against a block-granular table. `suites` is the
sharpest instance: `-Aiii.sh` holds it, the umbrella cites it at `:82`, and A-i §13 already carries its
relocation to `-common.sh` as **owed**. ⚠ That relocation is quoted here as what §13 *says*, not as where the
block goes: the disposition memo's D11 measures `suites` declaring `umbrella`, so the owed move is discharged
at a destination §13 does not name.

A second, independent form of the same gap: **the block set is written down in many places, and the count is
not this page's to carry either.** The enumeration is now `rederive homes`, which derives the sites rather
than listing them and prints what it cannot see. What a hand-listed set costs, measured in a
`git clone --local` sandbox: deleting `-Aii.sh` and `-B.sh` and narrowing only the source loop leaves
`selfcheck` reporting **12 blocks that no longer exist** (9 from `-Aii.sh`, 3 from `-B.sh`) as failing its
return-discipline check, and `inventory` unable to source at all. ⚠ **The deletion set belongs in the
sentence**: the same measurement over a set that also drops A-iii's three blocks returns a different number,
and a digit detached from its deletion set is not a wrong answer to a right question — it is what happens
when one fact has five homes.

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

Nothing **runs** it outside `docs/plans/` (M5). What verifies it is a set the harness itself reports, and
⚠ **that set is not closed by this note** — it has gained members within a single day's commits. Re-derive
rather than reading the table below:

```bash
sed -n "/^all() { set --/,/local failed/p" docs/plans/2026-07-citation-hygiene-A-rederive.sh
grep -c 'DEF = re.compile' docs/plans/2026-07-citation-hygiene-A-rederive-*.sh
```

| what | property | scope |
|---|---|---|
| **`_measure`** | a quantity that was not measured is unrepresentable as a pass | per call site — **and only there** |
| **`selfcheck`** | every roster block STATES its own exit status | whole harness |
| **`inventory`** | every block routes to a slice that can receive it, and to the file it lives in | whole harness |

**`inventory` is the new one, and it is the check whose absence produced the routing/shipping gap above.** No
count-based check could have caught it: the block count, the roster and the file layout were each internally
consistent the whole time. What was never derived was *whose* each block is.

**`selfcheck` and `inventory` do not have one parser, and the note must not claim they will.** Re-expressing
`selfcheck` over `declare -f` does not "delete the second parser". Measured: the `DEF = re.compile(…)` sites
are all in `-audit.sh`, and the one that would be deleted is `selfcheck`'s. The survivor is `inventory`'s
**line-oriented** parser over the raw files, and it is the source of every `ln` in §3 — `declare -f` strips
comments, so it cannot count prose lines at all. The honest collapse is narrower and still worth taking:
`selfcheck`'s **body** analysis moves to `declare -f` (bash parsing bash; verified that normalisation
preserves the trailing `return` in the multi-line, short and one-liner forms), and the raw-file parser stays
as the single **prose** reader. Two readers, two questions, one home each — not one parser.

⚠ **No `# kind:` declaration and no provenance annotation.** §2 withdraws the taxonomy a `# kind:` comment
would declare. A `# planted:` comment is a second spelling of *"a claim carries the command that falsifies
it"*, the decision surface the `stale-claim-detector` program owns; that program's v1 is blocked and v2 is
unpushed, so this note defers to no grammar that may not arrive. A block's expectation is named in its
header, as `couplings`' and `selfcheck`'s already are.

**No CI wiring.** The harness's consumers are memos under review and its readers are reviewers, who run `all`.
A script that creates git worktrees, calls `gh api` and reaches the network does not belong in every lane's
gate for a `docs/plans/` artifact; the scheduling concern that *is* real is A-iii's, over the Python suites.

## §5 What this note does not get to claim

**A claim is made once, in §1 or §2, beside the command that produces it.** There is no separate table
stamping each one `CHECKED`: a second copy of a claim is not a check on the first, it is a second thing to
keep true, and it drifts — which is why the table that used to sit here ended up carrying three digits its
own commands had already falsified.

Two claims have **no** command, and are recorded here because a missing check is not the same as an absent
claim (umbrella `:92`):

| claim | status |
|---|---|
| the `declared by` column is stable under memo edits | **UNCHECKED, and known false.** It parses §15 **prose**: a code span naming ≥2 known blocks is a declaration list, so a purely typographic rewrite of A-i's §15 (one span per name) moves A-i's row substantially. The memos are under active revision on another branch and every T1 figure depends on them. **The disposition memo must make the declaration machine-readable before acting on any ships-with cell** — which is what its `# ships-with:` rule does |
| A-i §8's layout paragraph after any harness change | **UNCHECKED.** §8 is the program's declared single home for the layout and states part count, per-part sizes, the line total, the block count and the `_measure` call-site census. The `inventory` commits alone already falsify them. A-i's to re-derive, at landing |

## §6 What this note settles, and what it hands over

**Settles** (no action authorised, because none is needed to state a fact):

1. The validity predicate is `_measure`'s, singular; the two invariant-carrying blocks are `couplings` and
   `selfcheck` and are already uniform. The J1/J2/J3 taxonomy is withdrawn.
2. Block ownership is what `rederive inventory` prints, by four stated tiers.
3. The routing unit and the shipping unit disagree, and the block set has many homes — both sizes are
   commands (`rederive inventory`, `rederive homes`), not digits this note carries.
   **No file-granular action on the harness is sound until that is collapsed.**
4. `selfcheck` and `inventory` answer two different questions and keep two readers; the collapse available is
   `selfcheck`'s body analysis moving to `declare -f`.
5. No CI wiring, no `# kind:` annotation, no provenance grammar.

**Hands to `2026-08-citation-hygiene-harness-disposition.md`**, which is gated by its own
`/elidex-plan-review`: what happens to #505; the routing/shipping collapse and the single home for the block
set; making the `declared by` column machine-readable; `citations`' comparison against §0.5's table; `lanes`'
unguarded bypasses; **the five status-discrimination sites — the four A-ii ones (`column`, `carvecolumn`,
`remedies`, `armmatrix`) plus `ruleset`, which ships A-iii — two of which (`column`, `ruleset`) were promised
a fix in #505**; the complete register sweep; and whatever defer slots survive that work, with
own/pre-existing classification, trigger and calendar date, registered in `project_open-defer-slots.md`.

**Explicitly does not authorise**: deleting or moving any block or file; closing or retargeting any PR;
editing any status register.
