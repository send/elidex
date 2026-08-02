# UMBRELLA — quantitative-claim staleness gate

Slot: `#11-stale-claim-detector-extension` (memory `project_stale-claim-detector-extension`).
Lesson of origin: `feedback_verified-claims-go-stale-under-own-later-edits` (PR #504, three recurrences).

**This is an umbrella.** It owns the problem statement, the measurements, and the decisions that bind
all slices. Each slice ships as its own PR under its own plan-memo and its own `/elidex-plan-review`,
per CLAUDE.md "Edge-dense work = multi-PR program". Nothing is implemented from this file.

| Slice | PR | Plan-memo | Ships |
|---|---|---|---|
| **S1** | 1 | `2026-08-claim-gate-s1-provenance.md` | the `<!-- claim … -->` grammar + the re-run wire |
| **S2** | 2 | `2026-08-claim-gate-s2-inventory-ledger.md` | the superlative ledger (one ledger, shared with S1) |
| **S3** | 3 | `2026-08-claim-gate-s3-check3.md` | `grep_pass.py` Check 3 + SKILL.md Pre-condition #2 |

Order is forced: S2's remedy and S3's remedy both name S1's grammar, and S2 stores S1's annotation
inventory. Each slice is a terminal unit under this umbrella (the base case in CLAUDE.md's rule) — it
is not re-split further.

---

## §0 The problem, and why v1 was withdrawn

A claim like "299 tests" or "the sole production caller" is a **cached measurement**. The measurement
is re-derivable from the tree; the claim is prose; nothing binds them. An edit that changes the
measured set leaves the claim silently false — tests stay green, reviewers do not re-measure. #504
shipped three and the gate caught none.

### §0.1 v1's postmortem is evidence, not embarrassment

The first version of this memo (`4ab23de1`) failed `/elidex-plan-review` with 3 CRIT / 31 IMP. Two
CRIT were design gaps (§1 D1, D2 below). The third, and the pattern behind roughly a quarter of all
findings, was this: **v1's claims about itself were never executed.** Its three specimen annotations
were all false (`count=299` measures 80 — the pathspec said `src/` where 299 is the whole crate;
`files=7` measures 1; `lines=1219` measures 1251, 1219 being a line *number*). It predicted preflight
would report `K=0 → single-PR` where the tool prints `K=4, SPLIT-RECOMMENDED`. It asserted
"`mise run ci` is cargo-only" where `mise.toml:116` lists `trip-wires` among its dependencies. It
cited a `§3` inside `SKILL.md` Pre-condition #2 that does not exist — a section number synthesised to
satisfy preflight's row parser, which is the fabricated-citation shape this project forbids outright.

Meanwhile v1's **empirical survey reproduced exactly**, all eight cells, verified independently by two
reviewers. The split is the whole thesis: *measured* claims held; *self-descriptive* claims rotted
within a single authoring session. v1 is retained in git history as the acceptance-test corpus for
this program, alongside #504's three.

The operative lesson for every slice below: **a claim this program's own documents make about the tree
must be executed before it is written, and the execution must be the thing that is committed.**

---

## §1 Umbrella decisions (bind every slice; a slice may not reopen these)

Each replaces a v1 defect. Slice plan-memos derive from these; they do not re-litigate them.

**D1 — no `at=` pinning. Provenance is live-only.**
v1 let an annotation pin its measurement to a sha. It is unimplementable: `grep -c 'fetch-depth'
.github/workflows/ci.yml` → 0, so every `actions/checkout@v6` runs at the default depth 1 and a pinned
sha cannot be resolved once `main` advances. Both possible verdicts are disqualifying — hard-fail on
legitimate annotations, or "unresolvable ⇒ skip", which silently retires every pinned claim. Deepening
the checkout would trade away the "no toolchain, ~1s" property that the ungated CI job's rationale
rests on. And a pinned claim is, by construction, the unfalsifiable-transcript form the origin lesson
names. Live-only also deletes v1's self-contradiction, where the reading note advertised "live"
provenance over three `at=`-pinned annotations.
*Consequence*: a landed document's annotation keeps being checked. That is intended, and it is why S1
must land before anything else annotates.

**D2 — inertness is defined per surface, and the gate excludes its own files.**
v1 scoped M1 to `.md`/`.rs`/`.sh`/`.toml` while defining all three inertness rules (fenced blocks,
backticked mentions, strict grammar) against Markdown only. Two consequences, both fatal: a rustdoc
example executes — `git grep -cE '^[[:space:]]*//[/!][[:space:]]*```' -- 'crates/**/*.rs'` sums to
**138** fence lines a line-start-anchored matcher cannot see — and S1's own wire, which must carry
literal example and fixture annotations, hard-fails its own gate on landing. Therefore: strip a
leading `//[/!]?` before fence matching in `.rs`, and exclude `.claude/tools/**` (the gate's own
files) from the scan by pathspec. The exclusion is the same shape that makes
`layout-box-reader-trip-wire.sh` immune — every one of its greps is scoped to `crates/**/*.rs` while
it lives elsewhere — but here it is explicit rather than incidental, and §4 records it as a bound.

**D3 — every recogniser tier is specified, including the lax one.**
v1 required a MALFORMED verdict while defining only the strict production, so MALFORMED could never
fire and its test exercised a rule that did not exist. Three tiers: **strict** (the full production →
execute), **lax** (`<!--[[:space:]]*claim[^A-Za-z]` matched but not strict, not backticked, not fenced
→ **MALFORMED, fail**), **mention** (backticked or fenced → ignore). A typo'd annotation must fail,
never vanish.

**D4 — every measurement carries a liveness precondition.**
`git grep -cE 'fn ' -- 'crates/**/*.rss'` exits 1 with no output — indistinguishable from a correct
pathspec matching nothing, so a typo'd `in=` makes its claim unfalsifiable. `git ls-files --
'crates/**/*.rss'` returns 0 lines, so the discriminator exists: **`in=` must match ≥1 tracked file or
the annotation fails** with its own diagnostic.

**D5 — the pattern body is linted, on the right reasoning.**
git's default ERE supports no PCRE escapes: **`\X` is the literal `X`**. Measured against
`crates/core/elidex-ecs/src/lib.rs`, `\d` and plain `d` both give **23**; `\b` and plain `b` both give
**15**; `\bfn\b` and `bfnb` both give nothing. So the lint's framing is **not** "these match nothing" —
it is "their ERE meaning is not the author's meaning", and whether a given one appears to under- or
over-match is just whether that literal happens to occur. Reject `\` followed by any of `bBdDsSwW<>`
with that diagnostic. Not hypothetical: v1's first survey pass used `\b` and reported four confident
zeros — a `\b` that silently became the literal `b`.
*(This wording is S1's correction of v2's first draft, which presented the two cases as opposite
failure directions rather than one rule. The reject set is a strict superset of the six escapes v2
first named, reached by applying this decision's own reasoning — not by widening its scope.)*

**D6 — the pathspec is always quoted.**
v1's §4.1.3 stated the A4 invariant ("every value reaches `git grep` as an argv element … never as a
shell word") and then illustrated it with `-- $pathspec`, unquoted. Measured under `/bin/bash`
3.2.57: `docs/**/*.md` unquoted splits into **79** words and the same annotation yields **30** where
the quoted form yields **32**. Silent divergence, no error, on a load-bearing claim.

**D7 — the countable-noun vocabulary is single-sourced; the regexes cannot be.**
This is the corrected form of v1's axis A5, which aimed at the annotation grammar (a remedy string —
prose drift at worst) and missed the copy that actually diverges: the superlative/unit pattern is
executed by `git grep -E` in S2's wire and re-implemented in `grep_pass.py` for S3. The two dialects
are **mutually** untranslatable — ERE has no `\b`, Python `re` has no `[[:space:]]` — so the copies
cannot be textually identical and "keep them in sync" is not available. Resolution: commit the
**word lists** (dialect-neutral alternations) as data read by both sides; each composes its own
boundary syntax; and a **cross-implementation control** runs both over one corpus and asserts
identical verdicts. That control is S3's, and it is the reason S3 may not land before S2.
*Corrected premise*: v1's Q4 priced the single-parser option as costing `grep_pass.py` a subprocess
dependency. `grep_pass.py:35` already reads `import subprocess`.

**D8 — one ledger, two row kinds, both diffed in both directions.**
v1 gave M2 a both-directions allowlist and M1 nothing, so deleting an inconvenient annotation turned
the wire green with no record — while naming annotation-set tampering as a live axis. One committed
TSV holds both annotation sites and superlative sites; `scripts/trip-wires.sh`'s own rationale for
diffing `REQUIRED_WIRES` in both directions applies unchanged, and it explicitly rejects the
"auto-enrolling additions is the safe direction" theory this would otherwise repeat.

**D9 — the artifact collapse is completed, not partial.**
`VERIFICATION_ARTIFACT_RE` (`grep_pass.py:77-79`) accepts three spellings: a `grep -[rlnEFhio]+`
literal, `verified \d{4}-\d{2}-\d{2}`, and a `lines? \d+(/\d+)+` list. v1 retired only the date form.
§0's argument — an artifact makes staleness *detectable* and nobody re-executes it — applies verbatim
to the other two, so retiring one of three leaves an author choosing among three accepted forms of
which one has a mechanism. Per `feedback_one-issue-one-way` that relocates the decision tax rather
than removing it. **All three are retired; the S1 annotation becomes the sole accepted artifact.**
Backlog cost is zero: `run_grep_pass(plan_memo, …)` only ever sees the one memo it is handed.

**D10 — a document may not assert a tree-wide count that its own landing changes.**
This is the program's central authoring defect, and it took six instances to see as structural rather
than careless. v1 shipped three false specimen annotations. v2 restated two inherited figures without
re-measuring (§6). S1 then measured *every* tree-wide number correctly — and six of them were false in
the commit that landed them, because the measurement's subject includes the document doing the
measuring. Confirmed at S1's own head: `layering trip-wire` 14 in 9 files → **16 in 11**; tracked
`.md`/`.rs`/`.sh`/`.toml` 1636/1641 → **1637/1642**; `set -- docs/**/*.md` 79 → **80** words;
stage-1 scan names "exactly one" file → **two**; the umbrella's annotations at lines 156,157 →
**160,161**.

The rule "measure, then write" cannot fix this, because the write is what invalidates the
measurement. Three permitted forms, and no fourth:

1. **An annotation the wire re-executes.** Self-healing, and the preferred form — but only for counts
   the grammar can express. `set --` word counts and `--untracked` deltas are `git`-invocation shapes
   no verb covers, so this is not always available.
2. **A property instead of a count** — "more than one file", "no tracked file lacks a trailing
   newline". Immune by construction, and usually the number was never load-bearing anyway.
3. **A count whose measurement base is stated as an index *including* this document**, taken after
   `git add` and re-taken before push.

⚠ The tempting fourth — exclude the program's own memos from the measured set — is **rejected**: it
would make the gate lie about its own surface, which is the failure mode D2's self-exclusion is
already carefully bounded to avoid.

Note what did **not** rot in S1: its acceptance criterion "3 strict / 3 mention / 0 malformed"
reproduces exactly, because that one figure was computed against the landed tree. The author
anticipated self-inclusion for annotations and for nothing else — which is precisely why this needs to
be a decision rather than a habit.

---

## §2 The measurements the decisions rest on

Measured at `3045f256`. `git grep`'s default ERE does not support `\b` (D5), so boundaries are written
`([^A-Za-z]|$)` throughout, and the wire must do the same.

```sh
UNIT='(callers?|sites?|modules?|methods?|entries|entry|consumers?|paths?|files?|tests?|lines?|cases?|variants?|fields?|readers?|writers?|impls?|users?|places?)'
SUP='(^|[^A-Za-z])(the only|the sole|no other|only remaining|sole)'
NUM='(^|[^0-9])[0-9]+\+?[[:space:]]+([A-Za-z._]+[[:space:]]+){0,3}'
NARROW="${SUP}[[:space:]]+([A-Za-z._-]+[[:space:]]+){0,2}${UNIT}([^A-Za-z]|\$)"
git grep -nEi "^[[:space:]]*//[/!]?.*${SUP}([^A-Za-z]|\$)" -- 'crates/**/*.rs' | wc -l   # bare superlative
git grep -nEi "^[[:space:]]*//[/!]?.*${NARROW}"            -- 'crates/**/*.rs' | wc -l   # + countable noun
git grep -nE  "^[[:space:]]*//[/!]?.*${NUM}${UNIT}([^A-Za-z]|\$)" -- 'crates/**/*.rs' | wc -l  # numeric
```

| Candidate set | `crates/**/*.rs` comments | `docs/**/*.md` | `.claude/**` |
|---|---|---|---|
| bare superlative | 294 | 204 | — |
| **superlative + countable noun** | **62** | **32** (30 in `docs/plans/`) | **1** |
| numeric claim + countable noun | 260 | 293 | — |

<!-- claim count=62 icase pattern="^[[:space:]]*//[/!]?.*(^|[^A-Za-z])(the only|the sole|no other|only remaining|sole)[[:space:]]+([A-Za-z._-]+[[:space:]]+){0,2}(callers?|sites?|modules?|methods?|entries|entry|consumers?|paths?|files?|tests?|lines?|cases?|variants?|fields?|readers?|writers?|impls?|users?|places?)([^A-Za-z]|$)" in="crates/**/*.rs" -->
<!-- claim count=294 icase pattern="^[[:space:]]*//[/!]?.*(^|[^A-Za-z])(the only|the sole|no other|only remaining|sole)([^A-Za-z]|$)" in="crates/**/*.rs" -->

(The two annotations above are **live** per D1 — once S1 lands they are re-executed on every push, and
this table reds the build if the tree moves under it. That is the intended behaviour for an umbrella
whose slice decisions are taken against these numbers.)

Three conclusions, each forced by a number:

1. **A bare-superlative lint is not viable.** Row 1, and sampling shows most are ordinary prose ("the
   only honest way"). A gate that overclaims its bound is worse than an honest slot — the withdrawn
   wire #5 in `layout-box-reader-trip-wire.sh` is the in-repo precedent.
2. **Requiring a countable noun makes it viable.** Row 2 is an order of magnitude smaller and the same
   size class as the existing 126-row `layout-box-reader-allowlist.tsv`. **S2 must publish the
   classification split of all 62 as a measurement, not as "the large majority"** — v1 killed row 1 on
   a quantified argument and accepted row 2 on an unquantified one, in a memo about unquantified prose.
3. **A repo-wide numeric presence lint is not viable.** Row 3 is the same order as row 1, and unlike
   row 1 no narrowing helps, because a numeric claim already names a countable noun.

Ledger scope: `crates/**/*.rs` (62 rows). `docs/plans/` superlatives are covered at authoring by S3.
Note for S2: the 62 include 10 rows in test code, and the precedent
(`layout-box-reader-trip-wire.sh`) *excludes* test code and says why — S2 owns stating why this
program inverts that, given #504 failure 1 was itself a test-side claim.

---

## §3 Spec coverage map

**No external spec surface.** This program touches no WHATWG / W3C / TC39 / CSS-WG algorithm; it ships
repository gate tooling. The table exists to satisfy the preflight structural gate.

⚠ Two honest notes v1 got wrong. First, the precedent it cited is only half-shared: both
`terminal-z-c3a-impl-plan.md` §7 and `terminal-z-c3a-seam-and-audit-plan.md` §3 cite **genuine
external specs** and share only the "reproduced solely to satisfy the gate, labels unmapped ⇒
soft-warn" shape. **Substituting in-repo documents for spec rows is novel here**, and the §4 slot's
trigger is written accordingly. Second, preflight counts unmapped labels toward K: it reports
`K=4, SPLIT-RECOMMENDED`, not the `K=0` v1 predicted. That verdict is **answered by this document
being an umbrella with three slice PRs** — i.e. the tool's recommendation is taken, not overridden.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| `docs/plans/2026-07-terminal-z-c3a-impl-plan.md` §3 (decision D4 — the reader-audit trip-wire) | allowlist diffed both directions | (i) live item absent from the allowlist → FAIL | S2's ledger check | ✓ | no |
| `docs/plans/2026-07-terminal-z-c3a-impl-plan.md` §3 | allowlist diffed both directions | (ii) allowlist row absent from the tree → FAIL | S2's ledger check; D8 extends it to annotations | ✓ | no |
| `docs/plans/2026-07-terminal-z-c3a-seam-and-audit-plan.md` §4 (the audit — grep is triage, the compiler is the proof) | token-absence enumeration | grep misses what carries no token at the use site | §4's honest-bound statement | ✓ | no |
| `docs/plans/2026-07-app-mode-turn-completion-drain.md` §5 (its own table was measured one commit early and three counts had drifted) | measurement taken against a stale head | the numeric-claim class S1 executes | S1's `count` / `files` / `lines` verbs | ✓ | no |
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §0.1 (v1's postmortem — the fourth instance, self-inflicted) | self-descriptive claim written unexecuted | the acceptance corpus every slice tests against | S1 controls; S3 test cases | ✓ | no |

**Breadth**: preflight reports K=4 / M=5 → SPLIT-RECOMMENDED, which this umbrella's three-slice
structure implements.

### §3.1 User-input touch audit

**No user-input surface.** Deliverables are a bash wire, a committed TSV and a Python lint under
`.claude/` and `scripts/`; nothing compiles into the engine. Exposure delta on any input path: zero.

The adjacent trust question is *repository* input, not user input: S1 evaluates provenance drawn from
the working tree, in CI, on fork-PR content (`ci.yml` triggers on `pull_request`). D5/D6 bound the
pattern body and the quoting; values reach `git grep` as argv after `--`; the residual runtime bound
is the job's `timeout-minutes: 2`, which S1 must name rather than leave implicit.

---

## §4 What this program does not close

Stated here once, so no slice has to infer a bound it does not have.

- **The ledger cannot catch #504 failure 1 in the form it occurred.** That claim was written at
  `725cbe56` and became false at `39638c9d` when a second pin was added *elsewhere*; the ledger key is
  `(path, content)` and neither changed. S2 forces a *decision at authoring time* for new superlatives
  — a nudge, not a proof — and S2's plan-memo must say so where v1's §6/§8 instead claimed all three
  #504 failures get a mechanical trigger. Claims of the form "only this test fails under this
  mutation" need the suite to run; a cargo-invoking wire would cost minutes and falsify the
  no-toolchain property the ungated job depends on. The remedy for that class stays authoring
  discipline: do not assert rank.
- **S1 is opt-in.** It executes the annotations that exist. A numeric claim in a `.rs` docstring
  written without one is still silent — see §5.
- **English only.** `docs/design/ja/**` yields zero candidates because the patterns are English. Zero
  hits there is a property of the pattern, not of the prose.
- **`//` line comments only.** Measured: the anchored pattern gives 62 and the unanchored 63, the sole
  delta being a test assertion string, and block/trailing comment forms give 0 today. A forward-looking
  bound, not a present hole.
- **CI reds the PR; it does not block the merge.** No required-status-check rule on `main` — the same
  caveat `layout-box-reader-trip-wire.sh` records. The visual-confirmation-before-squash convention
  remains the final gate.
- **62 grandfathered rows ship frozen.** A row that is already false stays false and stays green. S2
  hand-classifies all 62 (§2 conclusion 2), so any it finds stale should be recorded rather than
  silently re-frozen.
- **The gate does not scan its own files** (D2). Necessary for S1 to land at all; stated as a bound
  because it is also a tamper path.

---

## §5 Defer ledger

Re-audited after v1 booked three slots at the per-PR cap. Two of v1's three fail the create-time
slot-fit audit against a precedent in the very file S3 edits — `SKILL.md`'s Pre-condition #3 records a
comparable deferral and states it is "**deliberately NOT a `#11-*` platform slot** … skill-infra not a
platform gap". They become maintenance notes in their existing homes instead:

- **`--strict-enum` default-on** — already owned by `SKILL.md:85` ("Phase 3 で `--strict-enum` default
  on 検討"). Minting a `#11-*` slot would create a second tracking home for a decision that has one.
  S3 updates that line; it does not open a slot. ⚠ S3 must also confront the substantive point: #504
  failure 4 *fired* and was waved through **because** it was soft-warn, so leaving the plumbing
  soft-warn while adding vocabulary reproduces the miss. Fold the escalation into S3 or state which
  accepted defer category holds it.
- **preflight's §3 gate has no honest input for a non-spec PR** — pre-existing (this is the third plan
  to work around it, and the first to do so by substituting in-repo documents). Not an *own* deferral,
  so it does not consume this program's cap. ⚠ `origin/webref-cite-audit-tool` is in flight and
  already edits `SPEC_LABEL_REVERSE`'s ownership; reconcile before opening any slot here.

One genuine slot survives:

**`#11-claim-annotation-diff-scoped-lint`** — a presence lint for numeric claims on a PR's *changed
lines*, which is what would close §4's "S1 is opt-in" hole for `.rs` docstrings. This is the origin
lesson's own heaviest item ("#504 で最も重かったのは code 側"), so deferring it needs a real reason and
a fireable trigger, which v1 had neither of: it is blocked on the same CI shape as D1 — a diff scope
needs a base ref, and every checkout is depth 1. **Trigger**: the first job in
`.github/workflows/ci.yml` that requires `fetch-depth: 0` for any reason; at that point the blocker is
paid for and this becomes a slice. **Re-eval date**: 2027-02-02 (6 months), as the calendar fallback
the defer policy requires alongside the trigger.

---

## §6 Per-slice hand-off

Each slice plan-memo derives from §1's decisions, cites §2's measurements rather than re-deriving
them, and owns the questions listed here. All three take the full pre-push gate — `cargo fmt` →
`mise run ci` → `/simplify` → `/code-review` → `/review` → `/elidex-review`. ⚠ `mise run ci` is **not**
cargo-only: `mise.toml:116` lists `trip-wires`, so from S1 onward it exercises the deliverable itself;
for S3's Python, however, no runner exists in `mise.toml`, `ci.yml` or `scripts/` today, and S3 owns
either wiring `test_grep_pass.py` in or recording the gap. `/review` needs a GitHub PR, so the order
is push → draft PR → `/review <n>`; `/code-review` and `/review` are user-invoked.

This umbrella's plan-memo ships with **S1**.

- **S1** — the grammar as a complete per-verb production (`lines=` takes no `pattern`, and must
  resolve to exactly one tracked file, which v1 left as a pathspec silently summing file lengths);
  D2–D6 as production rules; the wire, its registration in `REQUIRED_WIRES` in the same commit, and a
  fixture self-test with a positive and a negative control per assertion. Controls must include the
  `console`-contains-`sole` substring shape, `\b`-fails-empty **and** `\d`-over-matches, a typo'd
  pathspec, a MALFORMED annotation, a table-cell annotation, and a rustdoc-fenced annotation. Name the
  file's seams before writing, not after: the nearest comparable wire is 493 lines.

  ⚠ **Two figures in this bullet were wrong in v2's first draft, and S1's own measurement pass caught
  them — one commit after §0.1 was written.** Both were carried over from v1 without re-measuring,
  which is the §0.1 defect recurring under the discipline that names it. Recorded rather than quietly
  patched, because it is the fifth instance and it says something the first four did not: restating an
  inherited number is the same act as asserting a new one, and inheritance is what makes it feel safe.
  - The **"~1s"** claim is *already false before S1 adds anything*: `bash scripts/trip-wires.sh`
    measures **1.20 / 1.20 / 1.28 s** warm here (S1's independent pass got 1.39–1.60 on a busier
    machine). So S1 does not "budget against ~1s" — it deletes an unverifiable number and anchors to
    the one machine-enforced bound, `timeout-minutes: 2`. The CI-side runtime is not measurable from a
    workstation and must be labelled so rather than estimated.
  - The **"layering" label** is at **6 set-level occurrences across 5 files**, not 3: `ci.yml:145` and
    `:153`, `CLAUDE.md:66`, `CONTRIBUTING.md:19`, `mise.toml:100`, `scripts/trip-wires.sh:4`
    (`git grep -niE 'layering trip-wire'` returns 14 in total; the other 8 are per-wire
    self-descriptions a new wire only adds to). `CONTRIBUTING.md:19` also asserts that a drifted
    trip-wire **allowlist** reds the PR — a mechanism S1 does not ship and S2 does, so that sentence
    is S2's to touch, not S1's.
- **S2** — the ledger under D8, the classification enum (including an honest home for a new
  tree-dependent superlative that cannot carry provenance, which v1's four classes had no slot for and
  which v1 instead absorbed into `not-tree-dependent` by stretching it to cover a judgement), the
  measured FP split of all 62, and the test-code inversion note from §2.
- **S3** — Check 3's vocabulary, the superlative rule, D9's complete artifact retirement, D7's shared
  word lists and the cross-implementation control, plus the soft-vs-hard question §5 flags. Known and
  accepted FP: a document *about* claims trips a claim lint — v1's own preflight run fires three times
  on quoted instances of #504's `"7 entries"`. Not worth a quoting heuristic; worth a test case.
