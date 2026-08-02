# Quantitative-claim staleness gate — executable provenance + an inventory ledger

Slot: `#11-stale-claim-detector-extension` (memory `project_stale-claim-detector-extension`).
Lesson of origin: `feedback_verified-claims-go-stale-under-own-later-edits` (PR #504, three recurrences).

> **Reading note.** This memo dogfoods the mechanism it proposes: every count below carries an inline
> `<!-- claim … -->` annotation in the form §4.1 specifies. Once PR-1 lands, those annotations are
> re-executed on every push — so if this memo's numbers rot, CI says so. That is the whole thesis in
> one artifact. The annotations inside fenced blocks are *examples* and are deliberately inert (§4.1.4).

---

## §0 Decision + scope

**What is broken.** A claim like "299 tests" or "the sole production caller" is a **cached measurement**.
The measurement is re-derivable from the tree; the claim is prose. Nothing binds the two, so an edit that
changes the measured set leaves the claim silently false. Tests stay green. Reviewers do not re-measure.
PR #504 shipped three of these and the gate caught none of them.

**What this plan builds.** Two mechanisms, disjoint by construction, plus one authoring-stage lint:

| | Mechanism | Trigger | Covers |
|---|---|---|---|
| **M1** | Executable provenance — a declarative `<!-- claim … -->` annotation, re-run repo-wide by a trip-wire | every push/PR | numeric claims (opt-in, zero backlog) |
| **M2** | Inventory-superlative ledger — a committed allowlist diffed in both directions | every push/PR | "the only / the sole" claims in `crates/**/*.rs` |
| **M3** | `grep_pass.py` Check 3 extension — presence lint, remedy re-pointed at M1 | plan-memo authoring | numeric + superlative claims in a plan-memo, pre-commit |

**Scope decision — split into two PRs.** PR-1 = M1 + M2 (one wire script, one self-test harness; M2's
remedy names M1's syntax, so shipping M2 first would emit a remedy with no mechanism behind it). PR-2 =
M3 (different language, different test harness, different lifecycle stage; it only *references* M1's
syntax). §5 carries the sequencing; §7 Q6 puts the split itself to plan-review.

**Mandatory plan-review.** §2 enumerates six intersecting invariant axes, so the CLAUDE.md
"Edge-dense work" rule applies as a rule, not as judgment: this memo must pass `/elidex-plan-review`
before implementation, and the work is split rather than bundled.

**Out of scope (named, not silently dropped).**

- Claims whose measurement needs the Rust toolchain ("only this test fails under this mutation").
  §4.4 argues these are not gate-able at all and the correct remedy is authoring discipline
  (`feedback_verified-claims-go-stale-under-own-later-edits` remedy 1: do not assert rank).
- Japanese-language claims. Every pattern here is English-only; `docs/design/ja/**` contributes zero
  hits to the M2 candidate set for exactly that reason, which is an **honest coverage hole**, not
  evidence of a clean tree. §6 records it.
- A repo-wide *presence* lint for numeric claims. §4.3 shows by measurement why it cannot land.

---

## §1 The decisive fact — what the existing detector cannot reach

`.claude/skills/elidex-plan-review/grep_pass.py` Check 3 already exists: it flags
`\d+\+? (callers|sites|modules|methods|entries|consumers|paths|files)` when no *verification artifact*
(`grep -X` literal / `verified YYYY-MM-DD` / an explicit line list) sits within ±200 chars, as a
soft-warn. Against #504's three failures:

| #504 failure | Site | Check 3 verdict | Why |
|---|---|---|---|
| "the only test that fails under this mutation" | `.rs` doc comment | not reachable | input is one plan-memo file |
| "297 tests" (two places, tree said 299) | plan-memo | silent | `tests` absent from the unit vocabulary |
| §5 touch-set measured one commit early | plan-memo | silent | unit implicit (bare numbers from `wc -l`) |
| "7 entries" | plan-memo | **fired** | …and was waved through, because soft-warn |

Two structural facts follow, and they set the design:

1. **Presence of an artifact is not the property we want.** The memo's own diagnosis:
   *"artifact は staleness を検出可能にするだけで誰も再実行しない."* A `verified 2026-08-02` phrase
   satisfies Check 3 forever and can never be falsified — it is a transcript, not a gate. The property
   we want is **the measurement still reproduces**, which requires *re-executing* it.
2. **The trigger is in the wrong place.** A claim lives where it is *read*; it breaks where the
   measured set is *edited*. Whoever adds the 300th test never opens the docstring that says 299. So
   the re-execution must be unconditional and repo-wide — i.e. a trip-wire on every push — not
   something the claim's author remembers to redo.

The in-repo **solved form of exactly this problem** is `.claude/tools/layout-box-reader-trip-wire.sh`
wire #1 (`docs/plans/2026-07-terminal-z-c3a-impl-plan.md` §3, decision D4): an inventory whose
exhaustiveness a later decision depends on is committed as an allowlist and diffed against the live
tree in both directions. M2 is that shape, re-aimed; M1 is its numeric counterpart.

---

## §2 Coupled invariants (the edge matrix — plan-review checks each axis independently)

Six axes. Pairwise intersections that actually bite are spelled out; the rest are independent.

- **A1 Backlog vs coverage.** A gate that reds today's tree cannot land; a gate scoped to avoid that
  covers less than the lesson asks for. This is the axis §4.3's measurements resolve.
- **A2 False positives vs teeth.** A superlative lint at ~90% FP is ignored, which is worse than
  absent (the withdrawn wire #5 precedent in `layout-box-reader-trip-wire.sh`: *"a gate that overclaims
  its bound is worse than an honest slot"*).
- **A3 Tamper surface.** The gate's own ledger and annotation set are editable by the very PR the gate
  is checking. This is why `scripts/trip-wires.sh` diffs `REQUIRED_WIRES` in both directions and why
  the CI job is ungated by the paths filter.
- **A4 Execution safety.** M1 re-executes provenance that came out of the repository, in CI, on
  fork-PR content. Arbitrary shell is disqualifying.
- **A5 Two triggers, one contract.** M1's syntax is consumed by the wire (bash) and referenced by
  Check 3 (Python). A hand-kept second copy of the grammar is precisely the divergence hazard
  `trip-wires.sh` was written to remove.
- **A6 Self-verification.** *A gate that cannot fire is indistinguishable from a clean tree.* Every
  assertion needs a positive and a negative control, per the driver's established discipline.

Intersections:

| Pair | The intersection |
|---|---|
| A1 × A2 | Narrowing the pattern to cut FPs is also what makes the backlog small enough to freeze. The two are solved by the *same* choice (superlative **+ countable noun**), so they must be decided together, not sequentially — §4.3. |
| A1 × A3 | Grandfathering means the ledger ships pre-populated, so "add a row" is the normal remedy. That makes deleting a row cheap and invisible — hence the both-directions diff, and hence keying rows on `(path, content)` so an *edit* forces reclassification while a *move* does not churn. |
| A3 × A6 | The self-test must run the driver against a fixture repo, not against the live tree — otherwise the control's own verdict depends on the tree it is validating. `scripts/trip-wires.sh`'s `TRIP_WIRES_SELFTEST` fixture is the pattern to copy. |
| A4 × A5 | The safe grammar (no shell, three verbs) is also the grammar Check 3 has to describe in its remedy string. Restricting the grammar for safety is what makes it cheap to state in two places — but §7 Q4 asks whether even that statement should be single-sourced. |
| A2 × A6 | A negative control for the superlative pattern must include a *known FP shape* (`console` contains the substring `sole`, which the unbounded pattern really did match — §4.3), so the boundary rule is pinned by a test rather than by a comment. |
| A1 × A4 | Opt-in annotations start the M1 backlog at zero, which is what lets M1 be a hard fail from day one. M2 cannot be, and that asymmetry is deliberate. |

---

## §3 Spec coverage map

**There is no external spec surface.** This PR touches no WHATWG / W3C / TC39 / CSS-WG algorithm: it
adds repository gate tooling under `.claude/tools/` and `.claude/skills/`. The table below exists
**solely to satisfy the plan-review preflight's structural gate** — the same accommodation
`docs/plans/2026-07-terminal-z-c3a-impl-plan.md` §7 makes, and for the same reason. The rows cite the
**in-repo governing contracts** this design is derived from; all labels are unmapped in
`preflight.py`'s `SPEC_LABEL_REVERSE`, so preflight will report them `unrecognized` (soft-warn). That
soft-warn is expected and correct here: extending `SPEC_LABEL_REVERSE` with in-repo documents would be
mislabelling repository docs as specifications.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| `docs/plans/2026-07-terminal-z-c3a-impl-plan.md` §3 (decision D4 — the reader-audit trip-wire) | allowlist diffed in both directions | (i) a live item absent from the allowlist → FAIL | M2's ledger check | ✓ | no |
| `docs/plans/2026-07-terminal-z-c3a-impl-plan.md` §3 | allowlist diffed in both directions | (ii) an allowlist row absent from the tree → FAIL | M2's ledger check | ✓ | no |
| `docs/plans/2026-07-terminal-z-c3a-seam-and-audit-plan.md` §4 (why grep suffices, and where it does not) | token-absence enumeration | grep misses what carries no token at the use site | M2's honest-bound statement in §6 | ✓ | no |
| `docs/plans/2026-07-app-mode-turn-completion-drain.md` §5 (the stale touch-set instance) | measurement taken one commit early | the numeric-claim class M1 executes | M1's `count` / `lines` verbs | ✓ | no |
| `.claude/skills/elidex-plan-review/SKILL.md` Pre-condition #2 §3 (Check 3's contract as documented) | artifact-presence rule | `verified YYYY-MM-DD` accepted as sufficient — the form M1 retires | M3's Check 3 edit | ✓ | no |

**Breadth**: K=0 external specs (5 in-repo contract rows) → single-PR scope on the preflight metric;
the split in §5 is driven by §2's axes, not by breadth.

### §3.1 User-input touch audit

**No user-input surface.** Nothing in this PR is compiled into the engine or reachable from page
content: the deliverables are a bash trip-wire, a committed TSV, and a Python lint used by a skill.
Exposure delta: zero new call sites on any input path.

The one adjacent trust question is **A4**, and it is not a *user*-input question but a *repository*-input
one: M1 executes provenance text drawn from the working tree, in CI, on fork-PR content. §4.1.3 bounds
it — the grammar admits no shell, and every value reaches `git grep` as an `argv` element after `--`,
never as a shell word.

---

## §4 The design

### §4.1 M1 — executable provenance

#### §4.1.1 The annotation

An HTML comment, adjacent to the claim it justifies, in one of three forms:

```
<!-- claim count=299 pattern="#\[test\]" in="crates/shell/elidex-shell/src/**/*.rs" -->
<!-- claim files=7  pattern="REQUIRED_WIRES" in="scripts/*.sh" -->
<!-- claim lines=1219 in="docs/plans/2026-07-app-mode-turn-completion-drain.md" -->
```

Three verbs, each a single `git` invocation:

| Verb | Asserts | Implementation |
|---|---|---|
| `count=N` | N matching **lines** | `git grep -cE -e <pattern> -- <pathspec>`, summed over files |
| `files=N` | N matching **files** | `git grep -lE -e <pattern> -- <pathspec>` \| `wc -l` |
| `lines=N` | a file's **length** | `git grep -c '' -- <path>` (tracked-file line count; no `wc`, so the pathspec discipline is uniform) |

Plus one optional flag token, `icase`, which adds `-i`. It is not cosmetic: elidex prose writes the
same claim as "the only", "the ONLY" and "the SOLE" in adjacent files, and a grammar without it forces
either `[Tt]he` character classes across every letter of every alternative or a silent undercount.
Authoring this memo produced the undercount first.

HTML-comment syntax because it is invisible in both rendered Markdown and rendered rustdoc, and legal
inside a `///` doc comment — one grammar for `.md` and `.rs` alike. It stays visible in the source,
which is where the person changing the count is working.

**Placement constraint**: an annotation may not sit inside a Markdown table cell. `pattern` values
routinely contain `|`, which a table cell requires escaping as `\|` — and `\|` reaching ERE is a
literal pipe, so the annotation would parse cleanly and measure something else. The wire therefore
**rejects** an annotation on a line whose content begins with `|`, rather than executing a pattern the
author cannot see is wrong. (This memo's §4.3 annotations sit in prose below their table for exactly
this reason; the constraint was found by writing them inside it.)

#### §4.1.2 Live vs pinned

A bare annotation is **live**: re-measured against the working tree, and a mismatch is a hard FAIL that
prints the asserted value, the actual value, and the claim site. Adding `at=<sha>` pins it:

```
<!-- claim count=299 pattern="#\[test\]" in="…" at=3045f256 -->
```

A pinned claim is re-run with `git grep <sha>` — still falsifiable in one second, which is the whole
point of remedy 3 in the origin lesson ("provenance は日付でなく再実行可能なコマンドで"), but it does
not red the build when the tree moves on. Landed plan memos are historical records and should pin;
claims a current decision rests on should stay live. §7 Q2 asks whether `at=` is a footgun that lets
everyone opt out of the teeth.

#### §4.1.3 Safety (axis A4)

No shell, ever. The wire parses the annotation with `sed`/`awk` into three scalars and passes them as
`argv`: `git grep -cE -e "$pattern" -- $pathspec`. The `-e` guard means a pattern beginning with `-`
is a pattern, not a flag; the `--` guard does the same for the pathspec. `pattern` may not contain a
double quote (the delimiter) — a claim that needs one is not expressible and must be reworded or
dropped, which is a documented limit rather than an escaping scheme. `at=` is constrained to
`[0-9a-f]{7,40}`.

#### §4.1.4 Inert examples (the self-documentation problem)

The grammar's own documentation must not execute — and this memo is itself the adversarial case, since
it both specifies the grammar and uses it. Three distinct forms of "a line mentioning the syntax"
showed up while writing it, and recognition is defined against all three:

1. **Fenced examples.** The wire tracks ```` ``` ```` fences per file and skips their contents — the
   same rule `grep_pass.py::_code_block_spans` already applies, so the two agree by construction
   rather than by convention.
2. **Inline-code mentions.** Prose that names the syntax in backticks is documentation, not a claim.
   Recognition is therefore **strict-grammar**, not substring: a line is a claim only if it matches
   the full production (verb, digits, quoted `pattern`, quoted `in`, optional `icase` / `at=`), and a
   backtick-wrapped mention is skipped outright. A prototype that keyed on the substring picked up two
   sentences of this memo's own prose and tried to execute them.
3. **Malformed annotations must not be silently ignored.** The corollary of strict matching: a line
   that *looks* like an annotation but does not parse is reported as **MALFORMED and fails**, never
   dropped. A typo'd annotation that vanishes is a claim with no gate behind it, which is the "gate
   that cannot fire" shape again.

This memo's live annotations sit in prose, outside fences and outside backticks, and the prototype in
§8 confirms all three are executed and reproduce.

#### §4.1.5 Scope

Repo-wide over tracked text files (`*.md`, `*.rs`, `*.sh`, `*.toml`). Opt-in, so the starting backlog
is zero and the check can be a hard fail from day one (axis A1 × A4).

### §4.2 M2 — the inventory-superlative ledger

An inventory superlative ("the only caller", "the sole production caller", "no other consumer reads
it") is a claim about a *set*, and it is exactly the class remedy 1 says not to write. It cannot be
made executable in general — "the only caller" would need call-graph knowledge grep does not have — so
the gate's job is not to verify it but to **force a decision when a new one appears**.

Shape, copied from `layout-box-reader-trip-wire.sh` wire #1:

- A committed TSV, rows keyed `classification <TAB> path <TAB> content`. Line-number-insensitive
  (moving a claim does not churn) and content-sensitive (editing one forces reclassification).
- Diffed against the live set in **both** directions: a live claim missing from the ledger FAILs, and
  so does a ledger row with no live counterpart.
- `classification` ∈ `provenanced` (an M1 annotation is adjacent — the good outcome) `|` `functional`
  (reworded so no set is asserted) `|` `grandfathered` (pre-existing, frozen) `|` `not-tree-dependent`
  (a mathematical or spec fact that no edit can falsify — e.g. `natives_bigint.rs:227`,
  "-2^63 is the only case where negation overflows").

A new superlative therefore costs one row and one choice: attach M1 provenance, or reword. That is the
behaviour change the lesson asks for, and it lands at the moment of writing.

### §4.3 Why M2 is narrow and M3 is not repo-wide — the measurements

Every number below was measured on `3045f256` with the command shown. `git grep`'s default ERE does
**not** support `\b` — a pattern using it matches nothing and reports a clean tree, which is how a
first pass of this survey produced four confident zeros. Word boundaries are spelled `([^A-Za-z]|$)`
throughout, and the wire must do the same.

```sh
UNIT='(callers?|sites?|modules?|methods?|entries|entry|consumers?|paths?|files?|tests?|lines?|cases?|variants?|fields?|readers?|writers?|impls?|users?|places?)'
SUP='(^|[^A-Za-z])(the only|the sole|no other|only remaining|sole)'
NUM='[0-9]+\+?[[:space:]]+([A-Za-z._]+[[:space:]]+){0,3}'
NARROW="${SUP}[[:space:]]+([A-Za-z._-]+[[:space:]]+){0,2}${UNIT}([^A-Za-z]|\$)"

# bare superlative, no countable noun required
git grep -nEi "^[[:space:]]*//[/!]?.*${SUP}([^A-Za-z]|\$)" -- 'crates/**/*.rs' | wc -l
git grep -nEi "${SUP}([^A-Za-z]|\$)"                       -- 'docs/**/*.md'   | wc -l
# superlative + countable noun  (= M2's candidate set)
git grep -nEi "^[[:space:]]*//[/!]?.*${NARROW}" -- 'crates/**/*.rs' | wc -l
git grep -nEi "${NARROW}"                       -- 'docs/**/*.md'   | wc -l
git grep -nEi "${NARROW}"                       -- '.claude/**'     | wc -l
# numeric claim + countable noun  (= a hypothetical repo-wide Check 3)
git grep -nE "^[[:space:]]*//[/!]?.*${NUM}${UNIT}([^A-Za-z]|\$)" -- 'crates/**/*.rs' | wc -l
git grep -nE "${NUM}${UNIT}([^A-Za-z]|\$)"                       -- 'docs/**/*.md'   | wc -l
```

| Candidate set | `crates/**/*.rs` comments | `docs/**/*.md` | `.claude/**` |
|---|---|---|---|
| bare superlative | 294 | 204 | — |
| **superlative + countable noun** | **62** | **32** (30 of them in `docs/plans/`) | **1** |
| numeric claim + countable noun | 260 | 293 | — |

The three load-bearing cells carry live provenance (the rest are re-derivable from the block above by
substituting the pathspec):

<!-- claim count=62 icase pattern="^[[:space:]]*//[/!]?.*(^|[^A-Za-z])(the only|the sole|no other|only remaining|sole)[[:space:]]+([A-Za-z._-]+[[:space:]]+){0,2}(callers?|sites?|modules?|methods?|entries|entry|consumers?|paths?|files?|tests?|lines?|cases?|variants?|fields?|readers?|writers?|impls?|users?|places?)([^A-Za-z]|$)" in="crates/**/*.rs" at=3045f256 -->
<!-- claim count=32 icase pattern="(^|[^A-Za-z])(the only|the sole|no other|only remaining|sole)[[:space:]]+([A-Za-z._-]+[[:space:]]+){0,2}(callers?|sites?|modules?|methods?|entries|entry|consumers?|paths?|files?|tests?|lines?|cases?|variants?|fields?|readers?|writers?|impls?|users?|places?)([^A-Za-z]|$)" in="docs/**/*.md" at=3045f256 -->
<!-- claim count=294 icase pattern="^[[:space:]]*//[/!]?.*(^|[^A-Za-z])(the only|the sole|no other|only remaining|sole)([^A-Za-z]|$)" in="crates/**/*.rs" at=3045f256 -->

The bare-superlative row initially read 329/245. Those were the counts of a pattern **without** the
leading `(^|[^A-Za-z])` boundary that the other rows carry — measured before the `console`-contains-`sole`
FP was found, and left un-remeasured when the boundary was added. Writing the annotation and running it
is what surfaced the inconsistency; no reviewer, and no amount of re-reading the prose, would have. This
is the mechanism catching its own author, in the same document that proposes it.

Three conclusions, each forced by a number rather than by taste:

1. **A bare-superlative lint is not viable** — the row-1 cells, and sampling shows most are ordinary
   prose ("the only honest way"). Axis A2 kills it.
2. **Requiring a countable noun makes it viable** — row 2 is an order of magnitude smaller, the same
   size class as the 126-row `layout-box-reader-allowlist.tsv` already in this repo, and inspection of all 62
   `crates/` hits shows the large majority are genuine inventory claims of exactly the load-bearing
   kind (`fragment_tree.rs:220` "The sole production caller is …", `walk.rs:118` "its sole remaining
   consumer is …", `multicol/lib.rs:515` "is the ONLY store-write site"). Residual FPs
   (`file_reader.rs:578` "the only realistic risk site" — a judgement, not a count) are absorbed by
   the `not-tree-dependent` classification rather than by loosening the pattern.
3. **A repo-wide numeric presence lint is not viable** — row 3 is the same order as row 1, and unlike
   row 1 no narrowing helps, because a numeric claim *already* names a countable noun. So the numeric class gets M1
   (opt-in, hard, repo-wide) plus M3 (presence lint, plan-memo-scoped, where the surface is one
   freshly written file). The A/B split is therefore *measured*, not stylistic: A and B cover
   disjoint surfaces with disjoint mechanisms and do not overlap at all.

Ledger scope recommendation: `crates/**/*.rs` only (62 rows). `docs/plans/` superlatives (30) are
covered at authoring by M3, and freezing landed plan memos would grow the ledger on every PR for no
decision benefit. §7 Q3 puts this to plan-review.

### §4.4 What no gate can reach

"Only this one test fails under this mutation" needs the suite to run. A trip-wire that invokes cargo
would cost minutes instead of ~1s and would falsify the standing statement in `.github/workflows/ci.yml`
that the wires need no toolchain — the property that lets the job stay ungated by the paths filter.
So this class is **not gate-able**, and the remedy stays authoring discipline: do not assert rank.
M2 is the closest mechanical approximation — the claim's *wording* trips the ledger, forcing the author
to classify it — and that is a nudge, not a proof. Saying so plainly is the wire #5 lesson.

### §4.5 M3 — Check 3 extension

Four edits to `grep_pass.py` + `SKILL.md`, all inside the existing structure:

1. **Unit vocabulary** — add `tests|lines|cases|variants|fields|readers|writers|rows|assertions` to
   `ENUM_CLAIM_RE`. (#504 failure 2.)
2. **Superlative rule** — the §4.3 `NARROW` pattern, applied to the plan-memo, same soft/hard plumbing.
   (#504 failure 1, at the one stage where the surface is small enough to lint.)
3. **Retire the date form** — drop `verified\s+\d{4}-\d{2}-\d{2}` from `VERIFICATION_ARTIFACT_RE` and
   accept an M1 annotation instead. Unfalsifiable provenance is the thing §1 identified as the root
   defect; keeping it as an accepted artifact would leave the escape hatch open. No backlog cost:
   Check 3 only ever sees the one memo passed to `preflight.py`.
4. **Remedy string** — point at the M1 grammar, so the fix an author reaches for is the one the wire
   will keep honest afterwards.

**A known FP shape, found by running preflight on this memo**: Check 3 fires on `"7 entries"` twice
here, both times where the memo *quotes* #504's claim rather than making one. A document about claims
will always trip a claim lint. Not worth a quoting heuristic (a quoted claim is textually
indistinguishable from a real one, and the remedy — an annotation — is harmless either way); worth
being written down so the next reader does not mistake it for a defect. §8 pins it as a test case.

Escalation to hard-fail (`--strict-enum` on by default, SKILL.md Pre-condition #2 "Migration phase 3")
is **deliberately not in this plan** — §9 records it as a slot with its trigger.

---

## §5 Decomposition

**PR-1 — `.claude/tools/claim-staleness-trip-wire.sh` (M1 + M2).**

1. Annotation parser + the three verbs + fence skipping (§4.1).
2. Wire #1 = re-run every live/pinned annotation; mismatch → FAIL naming site, asserted, actual.
3. Wire #2 = the superlative ledger diff, both directions, against
   `.claude/tools/claim-inventory-allowlist.tsv`.
4. `--regenerate` for the ledger, preserving classification by `(path, content)` — the
   `layout-box-reader-trip-wire.sh` affordance.
5. Self-test (axis A6): fixture repo, positive + negative control **per assertion**, each negative
   control asserting the diagnostic substring so one check cannot mask another. Controls must include
   the `console`-contains-`sole` shape and a `\b`-in-ERE regression.
6. Register in `scripts/trip-wires.sh` `REQUIRED_WIRES`, **same commit**.
7. Seed the ledger from the live tree; classify all 62 rows by hand.

**PR-2 — `grep_pass.py` Check 3 (M3).** The four edits in §4.5, plus `test_grep_pass.py` cases and the
SKILL.md Pre-condition #2 rewrite. Depends on PR-1 only for the grammar it names.

Both PRs take the **full pre-push gate** — `cargo fmt` → `mise run ci` → `/simplify` → `/code-review`
→ `/review` → `/elidex-review`. `mise run ci` is cargo-only, so for `.sh`/`.py` deliverables the
correctness and quality stages are the *only* net there is. `/review` needs a GitHub PR, so the order
is push → draft PR → `/review <n>`; `/code-review` and `/review` are user-invoked.

---

## §6 What this closes / does not close

**Closes.** All three #504 recurrences get a mechanical trigger: failure 1 via the M2 ledger (the
wording forces a row and a classification), failures 2 and 3 via M1 (re-executed every push) with M3
prompting the annotation at authoring time. The unfalsifiable `verified YYYY-MM-DD` artifact is retired.

**Does not close — stated, so no reader infers a bound that is not there.**

- **Opt-in is opt-in.** M1 checks the annotations that exist. Nothing forces a numeric claim in a
  `.rs` doc comment to carry one; only plan-memos get a presence lint (M3). A numeric docstring claim
  written without an annotation is still silent. Closing that needs a diff-scoped lint, which needs a
  base ref in CI — §9 slot.
- **English only.** `docs/design/ja/**` yields zero M2 candidates because the patterns are English.
  Zero hits there is a property of the pattern, not of the prose.
- **CI reds the PR; it does not block the merge.** No required-status-check rule exists on `main`
  (`layout-box-reader-trip-wire.sh` records the same caveat). The "CI 全 pass を目視確認してから squash
  merge" convention remains the final gate.
- **The ledger bounds appearance, not truth.** A grandfathered row that is already false stays false
  and stays green. M2 makes *new* claims cost a decision; it does not audit the existing 62.
- **The ledger is editable by the PR it gates.** Both-direction diffing makes a deletion a visible
  edit; it does not prevent one.

---

## §7 Open questions for `/elidex-plan-review` (decision-level)

1. **Is the annotation grammar right?** Three verbs (`count` / `files` / `lines`) with a `git grep`
   backing — is that enough for elidex's real claim shapes, or is a fourth verb needed (a `git diff
   --stat`-shaped "touch set" verb would have caught #504 failure 3 directly, at the cost of a base ref)?
2. **Is `at=` a footgun?** It makes historical memos stable, but nothing stops an author pinning a
   load-bearing current claim to dodge the teeth. Options: allow it freely; allow it only outside
   `crates/`; require a reason field; drop it and accept that landed memos red the build.
3. **Ledger scope** — `crates/**/*.rs` (62) as recommended, or also `docs/**/*.md` (32) and
   `.claude/**` (1)? The docs half is covered at authoring by M3 but is otherwise unfrozen forever.
4. **Where does the grammar live (axis A5)?** The wire implements it in bash; Check 3 names it in a
   remedy string. Is a prose SSoT enough, or should Check 3 shell out to the wire in a `--check-file`
   mode so there is exactly one parser? The latter costs `grep_pass.py` a subprocess dependency and
   a test-isolation problem; the former is a hand-kept second copy of a grammar, which is the exact
   hazard `trip-wires.sh` exists to remove.
5. **Should M1 be hard-fail on day one?** The backlog argument says yes (opt-in ⇒ zero rows). The
   counter is that a mis-authored pattern then reds `main` for someone who did not write it.
6. **Is the 2-PR split right?** Alternative framings: 3 PRs (M1 / M2 / M3), or 1 (all of it, on the
   grounds that M2's remedy and M3's remedy both name M1's grammar and shipping them apart means two
   rounds of prose about the same contract).
7. **Preflight's §3 gate has no honest input for a non-spec PR.** This memo satisfies it by citing
   in-repo contracts (§3) and accepting the unrecognized-label soft-warn, following
   `terminal-z-c3a-impl-plan.md` §7. Two plans have now made the same accommodation. Is that a gate
   defect worth its own slot, or is the accommodation the right permanent answer?

---

## §8 Test strategy

A throwaway prototype of wire #1's `count` verb (fence-skip, strict-grammar recognition, table-cell
rejection, `icase`, `at=`) was written and run against this memo while drafting it. It is what produced
findings §4.1.1 (table cells), §4.1.4 (2) (inline mentions) and the 329→294 correction — three defects
in a design that read as finished. It is **not** a deliverable; PR-1 writes the real wire.

**PR-1** — the wire's own self-test is the gate (there is no cargo test for a bash script), run by
`mise run trip-wires` ⊂ `mise run ci` and by the ungated CI job. Fixture repo under `mktemp -d`,
`TRIP_WIRES_SELFTEST`-style recursion guard. Required controls:

| # | Control | Asserts |
|---|---|---|
| 1 | annotation whose count matches the fixture | pass — the wire does not red a healthy tree |
| 2 | annotation whose count is off by one | fail, diagnostic naming asserted **and** actual |
| 3 | annotation inside a fenced block | ignored (§4.1.4 (1) — otherwise this memo self-executes) |
| 3b | backtick-wrapped mention in prose | ignored (§4.1.4 (2) — a substring-keyed prototype executed this memo's prose) |
| 3c | annotation with a typo'd verb | **MALFORMED → fail**, not silently skipped (§4.1.4 (3)) |
| 3d | annotation inside a Markdown table cell | rejected with a diagnostic (§4.1.1 placement constraint) |
| 4 | pinned `at=` annotation, tree moved on | pass |
| 5 | pattern beginning with `-` | treated as a pattern, not a flag (§4.1.3) |
| 6 | `pattern` containing `"` | rejected with a diagnostic, not silently mis-parsed |
| 7 | live superlative absent from the ledger | fail |
| 8 | ledger row with no live counterpart | fail |
| 9 | ledger row whose content was edited | fail (content-sensitive key) |
| 10 | ledger row whose line number moved | pass (line-insensitive key) |
| 11 | a comment containing `console` | **not** matched (the substring FP, §4.3) |
| 12 | a pattern using `\b` | rejected — ERE does not support it, and silent zero-matching is how this survey's first pass produced four false negatives |

Control 12 is not decoration: `\b` failing open is the exact shape of "a gate that cannot fire looks
like a clean tree", and it bit during this memo's own authoring.

**PR-2** — extend `test_grep_pass.py` (stdlib `unittest`, temp repo + temp plan-memo). Cases: each new
unit word fires; the superlative rule fires and respects the `console` boundary; `verified YYYY-MM-DD`
no longer satisfies the artifact requirement; an M1 annotation does; fenced-block occurrences are
skipped, matching the wire; and a *quoted* claim still fires (§4.5 — documented, not a defect).

**Acceptance test for the whole slot**: reconstruct #504's three claims as fixtures and confirm each is
now caught — failure 1 by the ledger, failures 2 and 3 by M1/M3.

---

## §9 Defer ledger

Three slots, each with a trigger rather than a date (per-PR ≤3 own deferrals — this is exactly 3):

1. **`#11-claim-annotation-diff-scoped-lint`** — a presence lint for numeric claims on *changed lines*
   of a PR, which would close the "opt-in is opt-in" hole in §6. Deferred because it needs a base ref
   in CI (`fetch-depth`, and an ambiguous base on push-to-`main`), which is a different gate shape from
   the stateless whole-tree wires. **Trigger**: a numeric `.rs` docstring claim goes stale after M1
   ships — i.e. the opt-in hole is demonstrated rather than hypothesised.
2. **`#11-check3-strict-enum-default`** — flip `--strict-enum` on by default (SKILL.md Pre-condition #2
   "Migration phase 3"). Deferred because escalation and vocabulary-widening in one PR make a
   regression un-bisectable. **Trigger**: two consecutive plan-memos land with a Check 3 soft-warn
   waved through, as #504's "7 entries" was.
3. **`#11-preflight-nonspec-coverage-map`** — §7 Q7: preflight's §3 hard gate assumes an external spec
   surface, and two plans have now worked around it in the same way. Deferred as out of scope here.
   **Trigger**: a third plan makes the same accommodation, or plan-review rules the accommodation wrong.
