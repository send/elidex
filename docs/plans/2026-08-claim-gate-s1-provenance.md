# S1 — the `claim` annotation grammar + the re-execution wire

Slice **S1** of the umbrella `docs/plans/2026-08-quantitative-claim-staleness-gate.md`. That file owns
the problem statement, the nine binding decisions (§1 D1–D9) and the measurements (§2). This memo does
not reopen any of them and does not re-derive the numbers; it cites them and adds only what S1 must
decide for itself.

Everything below that is a number, a `file:line`, a §-reference, or a statement about what a tool does
was produced by a command pasted next to it, run in this worktree. Head at authoring time:
`26b1067a` (three commits ahead of the umbrella's measurement base `3045f256`), `git version 2.55.0`,
`GNU bash, version 3.2.57(1)-release (arm64-apple-darwin25)`.

---

## §0 Decision + scope

**Decision.** S1 ships one registered trip-wire that re-executes every `claim` annotation in the
tracked tree on every push, plus the complete grammar it executes, plus a fixture self-test with a
positive and a negative control per assertion. Three verbs — `count`, `files`, `lines` — each with its
own complete production. D2–D6 become production rules, not prose. Registration in `REQUIRED_WIRES`
lands in the same commit as the wire.

**In scope**: the grammar; the recogniser tiers; the three verbs; the wire; its self-test; its
registration; the runtime budget and the two prose sites that claim it; the set-level "layering" label
the new wire joins.

**Out of scope, by the umbrella's own slicing**: the tamper ledger (D8 → S2), the superlative
vocabulary and `grep_pass.py` Check 3 (D7/D9 → S3). S1 therefore lands a gate that is opt-in and
whose annotations can be deleted without a record. That is stated as a bound in §6, not patched with a
half-ledger — a partial ledger in S1 plus the real one in S2 is the strangler shape CLAUDE.md
*One issue, one way* forbids.

**Why this is a terminal unit.** CLAUDE.md's edge-dense rule makes an approved umbrella's
narrowly-scoped per-PR slice the base case. S1 is that: one wire, one grammar, one commit's worth of
registration. The umbrella's own breadth verdict was answered by splitting into S1/S2/S3 — and it
reproduces: `python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-08-quantitative-claim-staleness-gate.md`
prints `K=4 … SPLIT-RECOMMENDED`, `EXIT=0`, matching what its §3 claims for itself.

---

## §1 The decisive fact: the two claims S1 is measured against are already false

The umbrella §6 hands S1 two coupled-site claims. Both were measured, and neither survives.

**(a) "~1s for the whole set" is false before S1 adds anything.** Measured, three warm runs:

```
for i in 1 2 3; do /usr/bin/time -p bash scripts/trip-wires.sh >/dev/null; done
  → real 1.39 / 1.43 / 1.60
env TRIP_WIRES_SELFTEST=1 /usr/bin/time -p bash scripts/trip-wires.sh   → real 1.09
```

The four wires alone are `0.51 + 0.42 + 0.02 + 0.06 = 1.01 s`
(`for w in .claude/tools/*-trip-wire.sh; do /usr/bin/time -p bash "$w"; done`), and the driver's own
fixture self-test adds the rest. So the sentence at `.github/workflows/ci.yml:129-130` ("the wires need
none (~1s for the whole set, no toolchain, no cache)") and its sibling at `CLAUDE.md:94`
("wire は toolchain 不要 (~1s)") are the exact failure class this program exists for: a cached
measurement nobody re-executed. This is measured on an arm64 macOS laptop, not on `ubuntu-latest`;
**the CI-side number is unverified and unverifiable from here**, which is itself the argument against
carrying a number in prose at all.

**(b) The set-level "layering" label is at five files, not three.** The umbrella §6 says three
(`trip-wires.sh`, `mise.toml`, `ci.yml`). Measured:

```
git grep -niE 'layering trip-wire' -- '*'   → 14 occurrences across 9 files
```

Eight of those are each wire's *own* header self-description (in the four `.claude/tools/*-trip-wire.sh`
files) — a new wire *adds* to that set rather than having to update it. The **set-level** label, which
describes what the whole driver runs and therefore what S1's wire joins, is at six occurrences in five
files: `scripts/trip-wires.sh:4`, `mise.toml:100`, `.github/workflows/ci.yml:145`,
`.github/workflows/ci.yml:153`, `CLAUDE.md:66`, `CONTRIBUTING.md:19`. The umbrella missed the last two.

`CONTRIBUTING.md:18-20` additionally narrows the set-level claim a second way — "*a drifted trip-wire
**allowlist** reds the PR*" — and S1's wire has no allowlist at all.

**What follows.** S1's wire is not a *layering* wire; it gates documentation provenance. So S1 must
either widen the set label or mis-file its own deliverable, and it must do so at five sites — one of
which asserts a mechanism (an allowlist) the new member does not have. §4.6 takes the decision.

---

## §2 Coupled invariants

Seven invariants hold simultaneously. Each is a way the wire can be green while being wrong.

| # | Invariant | What it forces |
|---|---|---|
| **I1** | **Total tier assignment** (D3) — every `<!--…claim…` occurrence lands in exactly one of *strict* / *malformed* / *mention*; none may vanish | The lax recogniser is the outer net and the strict production is a strict subset of it. A typo cannot fall out of the world. |
| **I2** | **Per-surface inertness** (D2) — mention-tier rules must never swallow a strict annotation, and the gate excludes its own files | Fence tracking is a per-file state machine, not a line predicate; `.rs` needs a leading `//[/!]?` strip first; `.claude/tools/**` is dropped by pathspec. |
| **I3** | **Liveness precondition** (D4) — `in=` must resolve to ≥1 tracked file; `lines=` to exactly 1 | A separate `git ls-files` probe per annotation, before the measurement, using the *same* pathspec string. |
| **I4** | **Dialect fidelity** (D5) — the pattern body's ERE meaning is the author's meaning | A parse-time reject-list on the pattern body, with a diagnostic that names the mechanism. |
| **I5** | **Argv fidelity** (D6) — every parsed value reaches `git grep` as one argv element | No `eval`, no unquoted expansion, no reconstructing a command string. |
| **I6** | **Provable firing** — every assertion has a positive *and* a negative control, and the negative asserts the diagnostic substring | The fixture self-test; plus the `REQUIRED_WIRES` line in the same commit, so the wire cannot silently leave the set. |
| **I7** | **Ungated-job budget** — added wall time must preserve the property that makes the CI job ungated (no toolchain, no cache, no network) | Bounds the scan shape: one tree-wide grep, then per-annotation work proportional to the annotation count, not to the tree. |

**Pairwise intersections.** All 21 pairs, load-bearing ones first.

| Pair | Intersection |
|---|---|
| I1 × I2 | The fence/backtick rules run *inside* tier assignment, so an over-eager inertness rule silently converts strict → mention. This is where "the gate exists but executes nothing" lives; §8 gives it a negative control (a fenced annotation carrying a deliberately wrong value must stay green, and an unfenced sibling with the same wrong value must red). |
| I1 × I4 | The reject-list runs on a body already parsed as strict. A pattern containing `\d` must be **malformed-with-a-dialect-diagnostic**, not "strict then failing"; otherwise the author reads a count mismatch and edits the count. |
| I2 × I7 | Fence state needs a per-file pass, which is why the scan is two-stage: one tree-wide `git grep` for candidate lines, then an `awk` pass over *only the files that had a candidate* (today: one file). Whole-tree `awk` would blow I7. |
| I3 × I4 | Both are "the annotation is unfalsifiable" failures in different halves of the same command; a typo'd pathspec and a `\b` both yield an honest-looking zero. Measured: `git grep -cE 'fn ' -- 'crates/**/*.rss'` exits 1 with no output, exactly like `git grep -cE 'zzznotpresentzzz' -- 'crates/**/*.rs'`. They must produce *different* diagnostics or the author fixes the wrong half. |
| I3 × I5 | The liveness probe and the measurement must be handed the **same** pathspec string as one argv element, or the probe validates something the measurement never runs. `git ls-files` and `git grep` share git's pathspec semantics — verified: `git ls-files -- 'crates/*.rs' \| wc -l` and `git ls-files -- 'crates/**/*.rs' \| wc -l` both give 1404, i.e. `*` crosses `/` identically for both. |
| I5 × I7 | The no-`eval` rule forbids the one shape that would let the wire batch several annotations into a single `git grep`. Serial execution is therefore forced, and I7 is what bounds how many annotations that stays acceptable for (§4.5). |
| I6 × I2 | The self-test's own fixture annotations must be inert in the real repo. They are, by D2's `.claude/tools/**` exclusion — which is why the exclusion is a *correctness* precondition for S1 landing at all, not a convenience. |
| I6 × I3 | The fixture must be a real git repo, because `git ls-files` has no `--no-index`. Measured cost of `git init -q . && git add -A` on a two-file fixture: `real 0.03`. No commit is needed — `git ls-files` and `git grep` both see added-but-uncommitted files (verified). |
| I6 × I7 | Controls are runtime. Every added probe is paid on every push and every local `mise run ci`. §8 keeps the probe count to the mandated set plus two. |
| I1 × I3 | A malformed annotation must fail *before* the liveness probe runs, or a typo'd verb plus a typo'd pathspec reports the pathspec. Order: tier → dialect → liveness → measure. |
| I1 × I5 | Parsing is what produces the argv elements; a parser that returns a single string re-creates D6's hazard one level in. The parser emits each field separately. |
| I1 × I6 | Every tier needs both directions: strict-that-passes / strict-that-fails, malformed-that-fails / mention-that-is-ignored. Four of §8's probes exist only for this pair. |
| I1 × I7 | Tier assignment is the one stage that touches every candidate line; it is a single `awk` pass over the candidate files only. |
| I4 × I5 | Both bound what reaches git, at different sites: I4 rejects at parse time with a visible diagnostic, I5 is invisible-by-construction in the executor. Kept separate for that reason. |
| I4 × I6 | The reject-list is a ban, and a ban that cannot fire is indistinguishable from a clean tree — the precedent is `layout-box-reader-trip-wire.sh`'s `ban_control`. A positive control per rejected escape is mandatory. |
| I3 × I7 | `git ls-files -- <spec>` measured at `real 0.00` per probe — below the noise floor. |

The remaining five pairs are independent, each for a stated reason: **I2×I3** (inertness decides
*whether* an annotation runs, liveness *whether its target exists* — no shared state), **I2×I4** and
**I2×I5** (inertness is a classification; the reject-list and the executor both act only on lines
already classified strict), **I4×I7** (a parse-time string test), **I5×I6** (argv discipline has no
runtime-observable surface to probe; it is enforced by code shape — no `eval` — and by review).

---

## §3 Spec coverage map

**No external spec surface.** S1 touches no WHATWG / W3C / TC39 / CSS-WG algorithm; it ships repository
gate tooling. **This table exists to satisfy the preflight structural gate**, whose row parser requires
a `§<number>` in every first cell or it hard-fails. The rows are therefore the in-repo governing
contracts S1 must satisfy — the same substitution the umbrella §3 makes, and, as it records, a novel
one. Every §-number below was confirmed present by reading the file's heading list
(`grep -nE '^#{1,4} ' <file>`).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D2 — inertness is per surface; the gate excludes its own files) | classify a candidate line | (i) `.rs` rustdoc fence, after the `//[/!]?` strip → mention | §4.2 tier `awk`, `rs=1` mode | ✓ | no |
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D2) | classify a candidate line | (ii) annotation under `.claude/tools/**` → not scanned | §4.1 scan pathspec `:(exclude).claude/tools/**` | ✓ | no |
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D3 — three recogniser tiers) | classify a candidate line | (iii) lax-but-not-strict → MALFORMED, fail | §4.2 tier `awk`, fall-through arm | ✓ | no |
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D4 — liveness precondition) | resolve `in=` | (iv) no tracked file → fail; `lines=` resolving to ≠1 → fail | §4.4 `git ls-files` probe | ✓ | no |
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D5 — pattern-body lint) | lint the pattern body | (v) a PCRE escape → MALFORMED with a dialect diagnostic | §4.3 reject-list | ✓ | no |
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D6 — the pathspec is always quoted) | execute the measurement | (vi) every value as one argv element, no `eval` | §4.4 executor | ✓ | no |
| `docs/plans/2026-07-terminal-z-c3a-impl-plan.md` §3 (Decision D4 — the standing gate; its hole-4 row: "runs once then rots" → standing in `mise run trip-wires` ⊂ `mise run ci` and in the ungated CI job) | register the wire | (vii) present in the glob but absent from `REQUIRED_WIRES` → fail | §4.5 + `scripts/trip-wires.sh:85-90` | ✓ | no |

**Breadth**: `python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-08-claim-gate-s1-provenance.md`
reports `K=2, M=7 → ok (single PR scope)`, `EXIT=0`, `0 hard` grep-pass findings. Exact verdict lines are
in §8.4.

### §3.1 User-input touch audit

**No user-input surface.** The deliverable is three bash files under `.claude/tools/` and one line in
`scripts/trip-wires.sh`. Nothing compiles into the engine; exposure delta on any input path is zero.

The adjacent trust question is *repository* input: the wire evaluates annotation text drawn from the
working tree, in CI, on fork-PR content (`.github/workflows/ci.yml` triggers on `pull_request`). Three
bounds, all structural: **I4** rejects PCRE escapes at parse time; **I5** forbids `eval` so an
annotation body cannot become a shell word; and the pattern is passed to `git grep -E`, which is a
regex engine, not an interpreter. The residual is regex cost on attacker-chosen input, bounded by the
job's `timeout-minutes: 2` (`.github/workflows/ci.yml:146`) — the only machine-enforced runtime bound
this job has, which §4.6 makes load-bearing.

---

## §4 The design

### §4.1 The scan (two stages)

Stage 1 is one tree-wide `git grep` for the lax recogniser over the D2 surface:

```
git grep -nE '<!--[[:space:]]*claim[^A-Za-z]' \
  -- '*.md' '*.rs' '*.sh' '*.toml' ':(exclude).claude/tools/**'
```

Measured: `real 0.05 / 0.04 / 0.04` over 1636 tracked files
(`git ls-files -- '*.md' '*.rs' '*.sh' '*.toml' ':(exclude).claude/tools/**' | wc -l` → 1636; without the
exclusion, 1641). The `:(exclude)` magic was verified to actually exclude:
`git grep -lE 'strip_comments' -- '*.sh'` → 4, and with `':(exclude).claude/tools/**'` appended → 0.

Stage 2 runs the tier `awk` over **only the files stage 1 named** — today exactly one,
`docs/plans/2026-08-quantitative-claim-staleness-gate.md`. That is what keeps I2 (per-file fence state)
from costing I7 (a whole-tree `awk`).

⚠ D2's exclusion governs where annotations are *looked for*. It does not restrict what an annotation may
*measure*: the live annotation in §5 of this memo targets a file under `.claude/tools/` and is
executed normally.

### §4.2 The three tiers (D3)

Prototyped as a throwaway `awk` script and run against real files before this section was written
(invoked `awk -v rs=<0|1> -f <prototype> <file>`; the prototype is scratch, not a deliverable — the
deliverable is `claim-grammar.sh` per §5):

- **mention** — the `<!--` occurrence is inside a fenced block, or an odd number of backticks precedes
  it on its line. Ignored.
- **strict** — matches the full per-verb production below. Executed.
- **malformed** — matched the lax recogniser, is not a mention, and is not strict. **Fails**, with a
  diagnostic naming the file, line and the first field that did not parse.

Fence tracking is the `preflight.py` `_fence_state_array` shape: ``` and `~~~` both open, only a
matching marker closes, and a marker line is never itself an annotation. For `.rs`, a leading
`//`, `///` or `//!` is stripped **before** fence matching (D2). That strip is load-bearing, not
cosmetic — measured on the live tree, a line-start-anchored fence matcher sees zero of the 138 rustdoc
fence lines:

```
git grep -cE '^[[:space:]]*//[/!][[:space:]]*```' -- 'crates/**/*.rs' | awk -F: '{s+=$NF} END{print s}' → 138
git grep -cE '^[[:space:]]*```'                   -- 'crates/**/*.rs' | awk -F: '{s+=$NF} END{print s}' →   0
```

Run against the live umbrella memo, the prototype classifies line 12 as **mention** (it is both
backticked and inside a markdown table cell) and lines 156 and 157 as **strict** — which is the
required behaviour for both of the required table-cell and backticked-mention controls, on real data.
Run against a synthetic `.rs` fixture with `rs=0` (i.e. the strip disabled), the rustdoc-fenced
annotation is misclassified **strict** — that is the negative control for D2 and it reproduces.

### §4.3 The pattern-body lint (D5)

D5 is binding and is not reopened. The measurement below refines only its **diagnostic wording**, and
it agrees with D5's own closing sentence ("ERE reads it as an escaped literal").

Every candidate escape was measured against `crates/core/elidex-ecs/src/lib.rs` with `git grep -cE`:

| body | result | plain equivalent | result |
|---|---|---|---|
| `\b` | 15 | `b` | 15 |
| `\bfn\b` | *(no output)* | `bfnb` | *(no output)* |
| `\d` | 23 | `d` | 23 |
| `\s` | 27 | `s` | 27 |
| `\sfn` | *(no output)* | `sfn` | *(no output)* |
| `\<` | 1 | `<` | 1 |
| `\D` / `\S` / `\W` | 15 / 13 / 2 | `D` / `S` / `W` | 15 / 13 / 2 |

So the mechanism is uniform: **`\X` is the literal `X`**. `\bfn\b` matching nothing is not "fails
empty" — it is the literal string `bfnb`, which is absent. D5's two named directions are both instances
of the one rule, and the honest reject set is `\` followed by any of `bBdDsSwW<>` — a strict superset
of D5's six, reached by applying D5's own reasoning rather than by widening its scope. §7 Q3 asks
plan-review to confirm that reading.

Diagnostic: *`git grep -E` is POSIX ERE — `\d` is the literal `d`, not a digit class. Write `[0-9]`,
`[[:space:]]`, and `([^A-Za-z]|$)` for a boundary.*

### §4.4 The three verbs

```
strict  := "<!--" WS "claim" WS ( count-verb | files-verb | lines-verb ) WS "-->"

count-verb := "count=" INT WS [ "icase" WS ] "pattern=" QSTR WS "in=" QSTR
files-verb := "files=" INT WS [ "icase" WS ] "pattern=" QSTR WS "in=" QSTR
lines-verb := "lines=" INT WS                                  "in=" QSTR

QSTR := '"' <any run of characters except '"'> '"'
WS   := one or more spaces or tabs        INT := [0-9]+
```

Field order is fixed. An annotation must be on one line — `git grep` is line-based, and a grammar that
straddles lines would need the multi-line scan §4.1 exists to avoid. `pattern=` may not contain a `"`.

| Verb | Executed as | Semantics |
|---|---|---|
| `count` | `git grep -nE[i] -e "$pattern" -- "$spec" \| wc -l` | **matching lines**, not matches: a line with two matches counts once. |
| `files` | `git grep -lE[i] -e "$pattern" -- "$spec" \| wc -l` | files containing ≥1 match. |
| `lines` | `awk 'END{print NR}' "$file"` | takes **no** `pattern`; `in=` must resolve to exactly one tracked file. |

The `-e` is not optional and belongs to D6's family: a pattern beginning with `-` is otherwise read as
an option. Measured — `git grep -nE '-> u32' -- 'crates/**/*.rs'` dies with ``error: unknown switch `>'``,
while `git grep -nE -e '-> u32' -- 'crates/**/*.rs' | wc -l` gives 30.

`count` is defined as matching lines because that is how the umbrella's authoritative numbers were
produced (its §2 code block pipes `git grep -n` into `wc -l`), and the two spellings agree:

```
git grep -nEi "<narrow>" -- 'crates/**/*.rs' | wc -l                          → 62
git grep -cEi "<narrow>" -- 'crates/**/*.rs' | awk -F: '{s+=$NF} END{print s}' → 62
```

`lines` uses `awk 'END{print NR}'` rather than `wc -l`: they agree on every tracked file today (audited
all 1641 `.md`/`.rs`/`.sh`/`.toml` files with `tail -c 1`; **zero** lack a trailing newline), but
`wc -l` counts newlines, so the first file that ships without one would be undercounted by exactly 1 —
silently, in the direction that makes a claim look stale.

**Liveness (D4), run before every measurement.** `git ls-files -- "$spec" | wc -l` must be ≥1
(`lines=`: exactly 1). This is the discriminator D4 names, and it is real:
`git ls-files -- 'crates/**/*.rss' | wc -l` → 0 while `git ls-files -- 'crates/**/*.rs' | wc -l` → 1404,
whereas both pathspecs make `git grep -cE` exit 1 with no output. The probe costs `real 0.00`.

**Argv (D6).** The parser emits `$verb`, `$expected`, `$icase`, `$pattern`, `$spec` as separate shell
variables; the executor spells the call literally with every expansion double-quoted and never builds a
command string. No `eval`, anywhere. D6's measured hazard reproduces at this head: with `p='docs/**/*.md'`,
`set -- $p` gives **79** words, and the umbrella's narrow pattern yields 33 quoted against 31 unquoted
(the umbrella recorded 32/30 at `3045f256`; `git grep … 3045f256 -- 'docs/**/*.md'` reproduces its 32
exactly, and the +1 is this program's own memos entering the tree).

### §4.5 Runtime budget (I7)

| Component | Measured |
|---|---|
| existing four wires | 1.01 s (sum of individual runs) |
| existing driver, self-test included | 1.39 / 1.43 / 1.60 s |
| S1 stage-1 scan | 0.04–0.05 s |
| S1 per strict annotation (worst measured: the whole-`crates` icase superlative regex) | 0.14 s |
| S1 liveness probe | 0.00 s |
| S1 fixture-repo construction | 0.03 s |

Today's two live annotations therefore cost ≈0.33 s. The self-test's probe count is the only
unmeasured term — **projected** ≤0.5 s at the §8 probe count, which puts the driver at ≈2.3 s. That
projection is explicitly not a measurement and must be re-run against the built wire before push.

The per-annotation term is linear and unbatched (I5 × I7). The wire header will record the re-runnable
trigger rather than a prediction: *if stage 3 exceeds one second, batch by `in=` before adding
annotations.* §9 applies the slot-fit audit to that and declines to mint a slot.

### §4.6 The two prose claims (§1's consequence)

**(a) Delete the "~1s", keep the property.** Both `.github/workflows/ci.yml:129-130` and `CLAUDE.md:94`
lose the number and keep the falsifiable part — no toolchain, no cache, no network — anchored to the
bound that is actually machine-enforced, `timeout-minutes: 2` at `.github/workflows/ci.yml:146`. This
follows `feedback_verified-claims-go-stale-under-own-later-edits`: provenance is a re-runnable command
or a machine-checked bound, never a remembered number. Both sites move in the same commit; §7 Q1 asks
plan-review whether to instead re-measure on `ubuntu-latest` and keep a number, which S1 cannot do from
here.

**(b) The set label.** The new wire is not a layering wire. Widen the set-level label at all five
files to "repository trip-wires" (or the wording plan-review prefers), and fix `CONTRIBUTING.md:19`'s
narrower "a drifted trip-wire *allowlist* reds the PR", which does not describe the new member. Each
wire's own header keeps saying what that wire is; only the set-level label moves. §7 Q2.

---

## §5 Decomposition and sequencing

The nearest comparable wire is 493 lines
<!-- claim lines=493 in=".claude/tools/layout-box-reader-trip-wire.sh" -->
— and that annotation is live: once S1 lands it is re-executed on every push, so this sentence reds the
build if that file is edited. It is here deliberately, as S1's own acceptance evidence, and it exercises
the `lines=` verb against a pathspec that resolves to exactly one tracked file
(`git ls-files -- '.claude/tools/layout-box-reader-trip-wire.sh' | wc -l` → 1;
`awk 'END{print NR}'` → 493).

**Seams, named before writing** (CLAUDE.md touch-time split: cut the seam while writing, not when a
reviewer counts lines):

| File | Owns | Projected |
|---|---|---|
| `.claude/tools/claim-grammar.sh` (sourced) | the tier `awk`, the field parser, the D5 reject-list, the three verb executors, the D4 probe. Pure: takes fields, returns a measurement or a diagnostic. | ~200 |
| `.claude/tools/claim-provenance-trip-wire.sh` | the registered wire: header rationale, the stage-1 scan, the per-annotation loop, verdict and exit. | ~150 |
| `.claude/tools/claim-provenance-selftest.sh` | the fixture repo and every control in §8. | ~200 |

The split is what lets the *grammar* be exercised by the self-test directly rather than only through
the wire's exit code — which is what makes a per-tier negative control expressible at all.

⚠ **Retention hazard the split creates.** `scripts/trip-wires.sh` discovers `.claude/tools/*-trip-wire.sh`
(`WIRE_GLOB`, `scripts/trip-wires.sh:102`) and diffs that against `REQUIRED_WIRES`
(`scripts/trip-wires.sh:85-90`). Neither helper matches the glob, so neither is protected by that
mechanism. Mitigation, in the house idiom (`test -f "$ALLOWLIST" || { red …; exit 1; }`,
`layout-box-reader-trip-wire.sh:195`): the wire hard-fails with its own diagnostic if either helper is
missing, so deleting one reds rather than silently disabling the controls. §7 Q4 asks whether that is
sufficient or whether the helpers should be folded back into one file.

**Commit sequencing (one PR, ordered commits):**

1. `claim-grammar.sh` + `claim-provenance-selftest.sh` + `claim-provenance-trip-wire.sh` + **the
   `REQUIRED_WIRES` line**, in one commit. The registration cannot trail: `scripts/trip-wires.sh`'s
   unregistered-wire check fails the moment the file appears, so any split here is a red intermediate
   commit.
2. §4.6(a) — the "~1s" edit at `.github/workflows/ci.yml` and `CLAUDE.md`, both sites, one commit.
3. §4.6(b) — the set-label edit at the five files, one commit.
4. This memo's line-count annotation re-verified against the landed wire.

Gate: `cargo fmt --all` → `mise run ci` → `/simplify` → `/code-review` → `/review` → `/elidex-review`.
`mise run ci` is not cargo-only — `mise.toml:116` lists `trip-wires` among its `depends`, so from this
PR onward the local gate exercises the deliverable. `/review` needs a PR, so the order is push → draft
PR → `/review <n>`.

---

## §6 What S1 does not close

- **Annotation deletion is unrecorded.** D8's ledger is S2's. Until it lands, deleting an annotation
  turns the wire green with no trace — the umbrella names this as the exact theory it rejects for
  `REQUIRED_WIRES`. S1 does not ship a partial ledger; the fix is S2, in the forced order.
- **S1 is opt-in** (umbrella §4). It executes the annotations that exist. A numeric claim written
  without one is still silent. The umbrella's `#11-claim-annotation-diff-scoped-lint` slot owns that,
  blocked on the same depth-1 checkout as D1 (`grep -c 'fetch-depth' .github/workflows/ci.yml` → 0;
  five `actions/checkout@v6` uses, none with a depth).
- **The gate does not scan its own files** (D2). Necessary for S1 to land; also a tamper path.
- **Untracked files are invisible**, because `git grep` without `--untracked` searches the index. That
  is the right scope — the gate governs the committed tree, matching D4's tracked-file rule — but it
  means an annotation is inert until staged. Measured while writing this memo:
  `git grep -nE '<!--[[:space:]]*claim[^A-Za-z]' -- <scan surface>` returned 3 lines in 1 file, and the
  same command with `--untracked` returned 6 in 2, the difference being this still-unstaged file.
- **Single-line annotations only** (§4.4). A forward-looking bound: every candidate line in the tree
  (all 6, per the `--untracked` scan above) is single-line.
- **`count` counts matching lines, not matches.** An annotation whose author meant "occurrences" is
  measuring something else, correctly and forever.
- **The wire re-executes; it does not judge.** A pattern that measures the wrong thing stays green.
  The `console`-contains-`sole` shape is the canonical instance: `git grep -nE 'sole' -- 'crates/**/*.rs'`
  gives 246 lines, 130 of them containing `console`, while the boundaried form gives 82 — all three are
  "correct" re-executions and only the author knows which was meant. §8 keeps a control for it precisely
  because the wire cannot.
- **A pattern can match its own annotation.** If `in=` covers the annotating file, the annotation's own
  line is measured. Today's live annotations avoid it by construction. §8 adds a control; the grammar
  does not forbid it.
- **CI reds the PR; it does not block the merge.** No required-status-check rule on `main` — the same
  caveat `layout-box-reader-trip-wire.sh:60-63` and `.github/workflows/ci.yml:140-143` already record.

---

## §7 Open questions for plan-review

**Q1 — the "~1s" replacement.** §4.6(a) deletes the number and anchors to `timeout-minutes: 2`. The
alternative is to keep a number, re-measured on `ubuntu-latest`, which this session cannot do.
Recommendation: delete — a number no local run can reproduce is the defect class, not a fix for it.

**Q2 — set-label scope.** Is a non-layering wire in `trip-wires` the right home at all, or does S1 want
its own driver? Recommendation: same driver, label widened at all six occurrences in one commit, and
drop "allowlist" from `CONTRIBUTING.md:19`. A second driver costs a second ungated CI job and a second
place a wire can go missing.

**Q3 — the D5 reject set.** §4.3 measures a uniform escaped-literal mechanism and proposes rejecting
`\` + `[bBdDsSwW<>]`, a superset of D5's six. Confirm this is derivation from D5's stated reasoning and
not a reopening of a binding decision.

**Q4 — three files or one.** §5 splits at ~200/150/200 lines against a 493-line single-file precedent,
putting two helpers outside `WIRE_GLOB`'s protection with only a hard-fail-if-missing check. Is that
mitigation the right shape, or should `claim-grammar.sh` be inlined and only the self-test split?

**Q5 — `count` semantics.** Matching lines vs occurrences. Recommendation: lines, stated in the wire
header and the diagnostic — it is how the umbrella's two landed annotations were produced, so the other
choice falsifies them.

---

## §8 Test strategy

All controls live in `claim-provenance-selftest.sh`, against a `mktemp -d` fixture repo built with
`git init -q . && git add -A` (0.03 s; no commit needed — verified that `git ls-files` and `git grep`
both see added-but-uncommitted files). The probe helper follows `scripts/trip-wires.sh`'s `st_probe`:
`$1` expected `pass`/`fail`, `$2` label, **`$3` a substring the diagnostic must contain**. `$3` is not
decoration — every failing assertion exits non-zero, so status alone cannot tell which check fired, and
a masking check would read as a pass. Every probe call is an operand of `||`, per the `set -e` note the
two sibling files both record.

### §8.1 Mandated controls

| # | Assertion | Positive | Negative (diagnostic substring) |
|---|---|---|---|
| 1 | substring vs boundary (`console` ⊃ `sole`) | fixture contains `console.log`; boundaried pattern with its true count → pass | bare `sole` pattern carrying the boundaried count → fail (`count mismatch`) |
| 2 | `\b` rejected | `([^A-Za-z]\|$)` boundary accepted → pass | pattern containing `\b` → fail (`POSIX ERE`) |
| 3 | `\d` rejected | `[0-9]` accepted → pass | pattern containing `\d` → fail (`POSIX ERE`) |
| 4 | typo'd pathspec | `in="crates/**/*.rs"` → pass | `in="crates/**/*.rss"` → fail (`matches no tracked file`) |
| 5 | MALFORMED | the same line backticked → pass (mention) | `<!-- claim cont=3 … -->` → fail (`MALFORMED`) |
| 6 | table-cell annotation | unbackticked annotation in a `\| … \|` row, correct count → pass (executes) | same cell, wrong count → fail (`count mismatch`, naming file:line) |
| 7 | rustdoc-fenced annotation | a `/// ```…``` `-fenced annotation carrying a **deliberately wrong** count → pass (ignored) | an unfenced sibling in the same `.rs` file with the same wrong count → fail (`count mismatch`) |

Control 7's positive is the strong form: the fenced annotation is wrong on purpose, so any regression
in the D2 `//[/!]?` strip turns the *positive* control red rather than leaving it vacuously green. The
`awk` prototype already exhibits both directions on a synthetic fixture (§4.2).

### §8.2 Added controls (two, beyond the mandated set)

| # | Assertion | Positive | Negative |
|---|---|---|---|
| 8 | `lines=` resolves to exactly one file | one-file pathspec, correct `awk END{NR}` → pass | a pathspec resolving to more than one → fail (`resolves to more than one tracked file`). Shape verified live: `git ls-files -- '.claude/tools/wasm-*-trip-wire.sh' \| wc -l` → 2 |
| 9 | `lines=` takes no `pattern` | `lines=` alone → pass | `lines=5 pattern="fn"` → fail (`MALFORMED`). Prototype already classifies this malformed |
| 10 | self-matching pattern | annotation whose `in=` excludes its own file → pass | annotation whose pattern matches its own line, `in=` covering it → fail, diagnostic names self-inclusion (§6) |
| 11 | gate self-exclusion (D2) | an annotation under `.claude/tools/` with a wrong count stays green | — (the exclusion has no failing direction; asserted as a pass-only invariant, stated as such) |

Control 11 is honestly one-directional and is labelled so in the file. Per the withdrawn wire #5
precedent in `layout-box-reader-trip-wire.sh:31-39`, a control that cannot fail must not be described as
if it bounds anything.

### §8.3 Live-tree acceptance

The umbrella carries two live annotations
(`docs/plans/2026-08-quantitative-claim-staleness-gate.md:156,157`). S1's wire executes them the moment
it lands, so they are an acceptance criterion, not a side effect. Both reproduce at this head:

```
git grep -nEi "^[[:space:]]*//[/!]?.*<narrow>" -- 'crates/**/*.rs' | wc -l → 62   (annotation says 62)
git grep -nEi "^[[:space:]]*//[/!]?.*<sup>"    -- 'crates/**/*.rs' | wc -l → 294  (annotation says 294)
```

Both also reproduce at the umbrella's stated base: `git grep … 3045f256 -- 'crates/**/*.rs'` → 62.
Line 12 of that file — `` `<!-- claim … -->` `` inside a table cell — must classify as *mention*; the
prototype does. Adding this memo's `lines=493` annotation (§5) and its two backticked mentions, the
tree as landed holds six candidate lines across the two memos — **3 strict, 3 mention, 0 malformed** —
which is the acceptance criterion, verified with the `awk` prototype against both memos as they stand.

### §8.4 Preflight

```
$ python3 .claude/skills/elidex-plan-review/preflight.py \
    docs/plans/2026-08-claim-gate-s1-provenance.md
  total entries  (M):   7  (data rows, breadth basis)
  malformed rows:       0  (no §<number> reference)
  unique specs (K):     2
  split decision:       ok (single PR scope)
  grep-pass:            0 hard, 13 soft
EXIT=0
```

All 13 soft findings are the two known shapes the umbrella also produces: 12 × `crates/**/*.rs` reading
as a glob to Check 1's path matcher, and 1 × unmapped in-repo labels — the latter by construction, since
§3 substitutes repository documents for spec rows. Zero Check 3 findings: every numeric claim in this
memo carries its command.

---

## §9 Defer ledger

**S1 books zero new slots.** The create-time slot-fit audit was applied to the one candidate.

**Candidate — "batch annotation execution by `in=` when the annotation count grows".** §4.5 measures
0.14 s per annotation, unbatched by I5 × I7. Audit: *(1) Is it a platform gap?* No — it is a
performance property of one script. *(2) Is there a repeat signal?* No — two annotations exist.
*(3) Does it have an existing home?* Yes: the wire's own header, where the sibling wires keep their
comparable notes. *(4) Would a slot create a second tracking home?* Yes. → **Not a slot.** It becomes a
header note with a re-runnable trigger ("if stage 3 exceeds one second…"), which is the same disposition
`.claude/skills/elidex-plan-review/SKILL.md` Pre-condition #3 records for its own deferred parser
("deliberately NOT a `#11-*` platform slot … skill-infra not a platform gap").

**Not S1's deferrals** (owned elsewhere, listed so no one re-books them): D8's ledger → S2; D7/D9 and
Check 3 → S3; `#11-claim-annotation-diff-scoped-lint` → already booked by the umbrella §5 with its
`fetch-depth: 0` trigger and its 2027-02-02 re-evaluation date.

**Own deferrals: 0 of the per-PR ≤3 budget.**
