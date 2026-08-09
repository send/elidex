# S1 — the `claim` annotation grammar + the re-execution wire

> ## Revision 3 — the memo is now machine-checked
>
> Round 2 returned three CRITs. The decisive fact was not any of them: **each was detectable in
> one command**, and two review rounds and ten independent agent passes were needed because the
> author never ran those commands. The remedy had been a prose rule seven times in this program,
> and had failed seven times, because the author believes the rule was followed.
>
> So this revision shipped `.claude/tools/claim-gate-plan-check.py` **before** it changed any prose.
> It reproduces all three CRITs mechanically, carries a positive and a negative control for every
> check it makes, and is run as part of the gate in §5. Every change below is one of its findings, or
> one of round 2's that it was extended to catch.
>
> ```
> $ python3 .claude/tools/claim-gate-plan-check.py --self-test | tail -1
> $ python3 .claude/tools/claim-gate-plan-check.py \
>     docs/plans/2026-08-claim-gate-s1-provenance.md \
>     docs/plans/2026-08-quantitative-claim-staleness-gate.md
> claim-gate plan check (bootstrap): 0 finding(s)
> ```
>
> The probe count is deliberately not written here. A first draft of this header said "29 probes",
> and by the time the revision was finished it was 34 — the failure this whole slice exists for,
> committed inside the paragraph announcing the fix, in a figure no check covered because it was a
> claim about the checker rather than about the tree. The command above is the artifact (D10 form 2).
>
> Building it produced three findings of its own, two of which corrected *this memo's critic*
> rather than the memo: `lines=` must use `awk 'END{print NR}'` semantics, not
> `len(text.split("\n"))`, which is off by one on every newline-terminated file **in the direction
> that reads as staleness** — so §8.2's pasted 1251 and 647 were right; and a citation in the umbrella
> gives a line number against a bare `SKILL.md`, which names three tracked files. The third is §4.6's,
> below.
>
> → memory `feedback_prose-rules-cannot-fix-unexecuted-claims`, `project_stale-claim-gate-v1-review`.


Slice **S1** of the umbrella `docs/plans/2026-08-quantitative-claim-staleness-gate.md`. That file owns
the problem statement, the ten binding decisions (§1 D1–D10) and the measurements (§2). This memo does
not reopen them and does not re-derive their numbers.

**Provenance discipline** (umbrella §1 D10, and the reason v1 and the first two S1 drafts all failed
review): every number, `file:line`, §-reference and statement about tool behaviour below came from a
command run in this worktree and pasted next to it. A figure this memo's own landing would invalidate is
replaced by a **property** (D10 form 2) or by a live annotation the wire re-executes (form 1); counts
that survive as counts are stated against a base **including this file** (form 3). Convention-dependent
quantities — whose value depends on how one chose to count rather than on what `wc -l` / `grep -c` /
`git` report — are **removed**, not re-measured.

**That paragraph is not the mechanism, and round 2 is the proof.** It was already present, in those
words, when a claim read off a source comment landed directly beneath it and carried §6's bound and all
of §7 Q3. A contract stating that commands were run is written by an author who believes they were.
The mechanism is `claim-gate-plan-check.py`: pasted commands are re-executed and their pasted answers
compared, tables are checked for exhaustiveness and for the properties their own preambles assert, and
a fact about how two programs behave is measured by running them rather than by reading either one.
Where a claim below cannot be put under that checker, it is **removed**, not restated more carefully.

Measurement base: the index at head `ef18b828` **with this memo `git add`ed**. `git version 2.55.0`,
`GNU bash, version 3.2.57(1)-release (arm64-apple-darwin25)` — the wire runs under `bash`, and §4.4's
argv rule is a `bash` fact, not a `zsh` one.

---

## §0 Decision + scope

**Decision.** S1 ships one registered trip-wire that re-executes every `claim` annotation in the tracked
tree on every push, the complete grammar it executes, and a self-test carrying a positive and a negative
control for every clause of that grammar. Three verbs — `count`, `files`, `lines` — each with its own
production; D2–D6 become production rules, not prose; registration in `REQUIRED_WIRES`
(`scripts/trip-wires.sh:85-90`) lands in the same commit as the wire.

**In scope**: the grammar; the three recogniser tiers, defined for every scanned surface; the three
verbs; the wire; its invocation of its own self-test; registration; the two prose sites carrying a stale
runtime number; the set-level label the new wire joins; and the two set-level sites §4.6 found that a
fifth wire falsifies.

**Already landed, ahead of this memo's approval**: `.claude/tools/claim-gate-plan-check.py`, which
checks *this memo* rather than the tree. It is not the deliverable and does not overlap it — the wire
executes `claim` annotations anywhere in the tracked tree, the checker validates one document's internal
consistency and re-runs the commands it pastes. They meet at one seam, deliberately: the checker's
annotation executor is a **bootstrap** that retires itself the moment
`.claude/tools/claim-provenance-trip-wire.sh` exists, at which point it shells out to the wire instead
of classifying anything. That is a code path, not a promise, so the two implementations cannot coexist.

**Out of scope, by the umbrella's own slicing**: the tamper ledger (D8 → S2), the superlative vocabulary
and `grep_pass.py` Check 3 (D7/D9 → S3), and `CONTRIBUTING.md:19-20`'s "a drifted trip-wire *allowlist*
reds the PR" sentence, which umbrella §6 routes explicitly to S2 as a mechanism S2 ships and S1 does not.
S1 therefore lands a gate that is opt-in and whose annotations can be deleted without a record — a bound
in §6, not a half-ledger, since "new seam plus a partial legacy" is the strangler shape CLAUDE.md *One
issue, one way* forbids.

**Why this is a terminal unit.** CLAUDE.md's edge-dense rule makes an approved umbrella's
narrowly-scoped per-PR slice the base case: one wire, one grammar, one self-test, one registration line.

---

## §1 The decisive fact: a helper is not a wire, so nothing runs it

The driver discovers what to run by glob, not by reference:

```
$ sed -n '102p' scripts/trip-wires.sh
WIRE_GLOB='.claude/tools/*-trip-wire.sh'
$ git ls-files -- '.claude/tools/*.sh'
.claude/tools/layout-box-reader-trip-wire.sh
.claude/tools/native-ctor-guard-trip-wire.sh
.claude/tools/wasm-runtime-trip-wire.sh
.claude/tools/wasm-vm-trip-wire.sh
```

Every tracked `*.sh` directly under `.claude/tools/` matches `WIRE_GLOB` today, so the driver's loop
(`scripts/trip-wires.sh:188-205`) and its both-directions retention diff (`:207-237`) cover all of them.
A file named `claim-provenance-selftest.sh` matches neither. Consequences, both of which shape S1:

1. **A split-out self-test is not executed by anything.** The first draft specified eleven controls in
   such a file and no invocation for it. `scripts/trip-wires.sh:104-111` already names the outcome:
   *"a check nobody re-runs is a transcript, not a gate."* So the wire must invoke its self-test itself,
   under a recursion guard, in the shape `scripts/trip-wires.sh:113-184` uses — and that invocation needs
   its own control (§8 C18), because an invocation nobody proves is the same transcript one level up.
2. **A split-out helper is outside the retention diff.** `scripts/trip-wires.sh:76-84` records why the
   diff runs in *both* directions. A `test -f` guard reproduces exactly the asymmetry that amendment
   closed — it catches a helper *leaving*, not one *arriving* unreferenced.

Both are closed by one choice, taken here rather than left to plan-review: **two files, not three.** The
wire carries the grammar inline; only the self-test is split, and the wire invokes it by name, so its
absence is an execution failure rather than a silently smaller gate. The cohesion argument supports that
one seam — the self-test must call the grammar's stages directly, not only through the wire's exit code —
and nothing more. So does line count: all four existing wires are single files (`wc -l`: 493 / 121 / 84 /
73), the 493-line one inlines its own probes, and S1's projected total is nowhere near the ~700–800 band
`feedback_touch-time-split-means-while-writing` watches.

---

## §2 Coupled invariants

Seven invariants hold simultaneously. Each is a way the wire can be green while being wrong.

| # | Invariant | What it forces |
|---|---|---|
| **I1** | **Total tier assignment** (D3) — every candidate line lands in exactly one of *strict* / *malformed* / *mention*; none may vanish | The lax recogniser is the outer net and strict is a subset of it. A typo cannot fall out of the world. |
| **I2** | **Per-surface inertness** (D2) — the mention rules must never swallow a strict annotation, the fence arm is defined per surface, and the gate excludes its own files | Fence tracking is a per-file state machine, not a line predicate; `.rs` needs a leading `//[/!]?` strip first; `.sh`/`.toml` have no fence arm at all (§4.2); `.claude/tools/**` is dropped by pathspec. |
| **I3** | **Liveness precondition** (D4) — `in=` must resolve to ≥1 tracked file; `lines=` to exactly 1 | A `git ls-files` probe per annotation, before the measurement, using the *same* pathspec string. |
| **I4** | **Dialect fidelity** (D5) — the pattern body's ERE meaning is the author's meaning | A parse-time reject-list on the pattern body, with a diagnostic naming the offending escape verbatim. |
| **I5** | **Argv fidelity** (D6) — every parsed value reaches `git grep` as one argv element | No `eval`, no unquoted expansion, no reconstructing a command string. Probeable: §4.4. |
| **I6** | **Provable firing** — every grammar clause has a positive *and* a negative control; the negative asserts a diagnostic substring unique to it | The self-test, the wire's invocation of it, and the `REQUIRED_WIRES` line, all in one commit. |
| **I7** | **Cost is bounded by the annotation count, not by the tree** | One tree-wide `git grep`, then per-annotation work. It is a *structural* bound, not a wall-clock one — see §4.5. |

**Pairwise intersections**, load-bearing first.

| Pair | Intersection |
|---|---|
| I1 × I2 | The inertness rules run *inside* tier assignment, so an over-eager one silently converts strict → mention. This is where "the gate exists but executes nothing" lives; every mention-arm control in §8 therefore pairs a wrong-count inert annotation (must stay green) with an identical non-inert sibling (must red). |
| I1 × I4 | The reject-list runs on a body already parsed as strict. A pattern containing `\d` must be **malformed with a dialect diagnostic**, not "strict, then a count mismatch" — otherwise the author reads a mismatch and edits the count. |
| I2 × I7 | Fence state needs a per-file pass, which is why the scan is two-stage: one tree-wide `git grep` for candidate lines, then a tier pass over *only the files that had a candidate*. A whole-tree tier pass would break I7. |
| I3 × I4 | Both are "the annotation is unfalsifiable" failures in different halves of one command, and both look like an honest zero. Measured: `git grep -cE 'fn ' -- 'crates/**/*.rss'` exits 1 with no output, exactly like a correct pathspec matching nothing. They must produce *different* diagnostics or the author fixes the wrong half. |
| I3 × I5 | The liveness probe and the measurement must receive the **same** pathspec as one argv element, or the probe validates something the measurement never runs. |
| I5 × I7 | The no-`eval` rule forbids the one shape that would batch several annotations into a single `git grep`. Serial execution is forced; §4.5 states what actually bounds it. |
| I6 × I7 | **The bound is on the wire, not on its self-test.** I7 says the wire's work is one tree-wide `git grep` plus a term per annotation. The self-test is not the wire: C10's positive runs `in="crates/**/*.rs"` and C19's corpus row runs `in="crates/shell/elidex-shell/src/**/*.rs"`, both tree-scale, and both deliberate — the acceptance corpus is only worth having because it runs against the real tree (§7 Q2). Stating I7 without this exemption makes the corpus a violation of it. |
| I6 × I3 | The fixture must be a real git repo: `git ls-files` has no `--no-index`. `git init -q . && git add -A` suffices — both `git ls-files` and `git grep` see added-but-uncommitted files, verified in §4.4's argv measurement, which ran against exactly such a fixture. |
| I1 × I3 | A malformed annotation must fail *before* the liveness probe, or a typo'd verb plus a typo'd pathspec reports the pathspec. Order: tier → dialect → liveness → measure. |
| I1 × I5 | Parsing is what produces the argv elements; a parser returning one joined string re-creates D6's hazard one level in. The parser emits each field separately. |
| I1 × I6 | Every tier needs both directions: strict-that-passes / strict-that-fails, malformed-that-fails, mention-that-is-ignored-*while-wrong*. |
| I1 × I7 | Tier assignment is the one stage touching every candidate line; it is a single pass over the candidate files only. |
| I4 × I5 | Both bound what reaches git, at different sites: I4 rejects at parse time with a visible diagnostic, I5 is invisible-by-construction in the executor. Kept separate for that reason. |
| I4 × I6 | A ban that cannot fire is indistinguishable from a clean tree — the precedent is `layout-box-reader-trip-wire.sh:276-299`'s `ban_control`. One control per rejected escape class, each with its own diagnostic substring. |
| I3 × I7 | The liveness probe is one `git ls-files` per annotation, i.e. inside I7's per-annotation term, not a second tree pass. |
| I2 × I6 | D2's self-exclusion is both a *correctness precondition* and a *two-directional* claim. The self-test's own fixture annotations must be inert in the real repo — they are, by the `.claude/tools/**` exclusion, which is therefore load-bearing for S1 landing rather than a convenience. And the control must assert both directions: an annotation under `.claude/tools/` ignored, and the identical annotation outside it red. Asserting only the first would pass on a wire that scanned nothing at all. |

The remaining four pairs are independent, each for a stated reason: **I2×I3** (inertness decides
*whether* an annotation runs, liveness *whether its target exists*), **I2×I4** and **I2×I5** (the
reject-list and the executor act only on lines already classified strict), **I4×I7** (a parse-time string
test). **I5×I6** was on this list and is not independent: §4.4 makes I5 directly probeable and §8 C12
probes it.

That the table above is exhaustive is not a claim to take on trust — with seven invariants it must
account for all 21 pairs exactly once, orientation-insensitively, and `claim-gate-plan-check.py`'s
`PAIR` check derives the invariant set from this section's own `**I**n` rows and enumerates them. Two
revisions asserted this section was complete while it stood at 20, with `I6×I2` and `I2×I6` present as
separate rows and `I6×I7` — the one pair that turned out to need an *exemption* rather than a note —
missing entirely.

---

## §3 Spec coverage map

**No external spec surface.** S1 touches no WHATWG / W3C / TC39 / CSS-WG algorithm; it ships repository
gate tooling. **This table exists to satisfy the preflight structural gate**, whose row parser hard-fails
without a `§<number>` in the first cell. The rows are the in-repo governing contracts S1 must satisfy —
the same substitution umbrella §3 makes and, as it records, a novel one. Every §-number below was
confirmed present by `grep -nE '^#{1,4} ' <file>`.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D2 — inertness per surface) | classify a candidate line | (i) `.rs` rustdoc fence, after the `//[/!]?` strip → mention | §4.2 tier pass, `.rs` mode | ✓ | no |
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D2) | classify a candidate line | (ii) `.sh`/`.toml`: no fence arm, backtick arm only | §4.2 tier pass, surface table | ✓ | no |
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D2) | scan | (iii) annotation under `.claude/tools/**` → not scanned; identical annotation outside → scanned | §4.1 scan pathspec | ✓ | no |
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D3 — three tiers) | classify a candidate line | (iv) lax-but-not-strict → MALFORMED, fail | §4.2 fall-through arm | ✓ | no |
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D4 — liveness) | resolve `in=` | (v) no tracked file → fail; `lines=` resolving to ≠1 → fail | §4.4 `git ls-files` probe | ✓ | no |
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D5 — pattern-body lint) | lint the pattern body | (vi) `\` + `[bBdDsSwW<>]` → MALFORMED, diagnostic names the escape | §4.3 reject-list | ✓ | no |
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D6 — the pathspec is always quoted) | execute the measurement | (vii) every value as one argv element, no `eval` | §4.4 executor | ✓ | no |
| `docs/plans/2026-07-terminal-z-c3a-impl-plan.md` §3 (Decision D4 — the standing gate; its "runs once then rots" hole → standing in `mise run trip-wires` ⊂ `mise run ci` and in the ungated CI job) | register the wire, invoke the self-test | (viii) present in the glob but absent from `REQUIRED_WIRES` → fail; self-test helper missing → fail | §5 + `scripts/trip-wires.sh:207-237` | ✓ | no |

**Breadth**: preflight's verdict lines for this memo are pasted verbatim in §8.3.

### §3.1 User-input touch audit

**No user-input surface.** The deliverable is two bash files under `.claude/tools/` and one line in
`scripts/trip-wires.sh`; nothing compiles into the engine, so the exposure delta on any input path is
zero. The adjacent trust question is *repository* input: the wire evaluates annotation text drawn from
the working tree, in CI, on fork-PR content (`.github/workflows/ci.yml` triggers on `pull_request`).
Three structural bounds — **I4** rejects PCRE escapes at parse time, **I5** forbids `eval` so an
annotation body cannot become a shell word, and the pattern reaches `git grep -E`, a regex engine rather
than an interpreter. The residual is regex cost on attacker-chosen input, where
`timeout-minutes: 2` (`.github/workflows/ci.yml:146`) caps a hang and nothing finer (§4.5).

---

## §4 The design

### §4.1 The scan (two stages)

Stage 1 is one tree-wide `git grep` for the lax recogniser over the D2 surface:

```
git grep -nE '<!--[[:space:]]*claim[^A-Za-z]' \
  -- '*.md' '*.rs' '*.sh' '*.toml' ':(exclude).claude/tools/**'
```

The `:(exclude)` magic was verified to actually exclude: `git grep -lE 'strip_comments' -- '*.sh'` → 4
files; with `':(exclude).claude/tools/**'` appended → 0. Stage 2 runs the tier pass over **only the files
stage 1 named**, which is what keeps I2's per-file fence state from costing I7 a whole-tree pass.

⚠ D2's exclusion governs where annotations are *looked for*, not what one may *measure*: one of §5's live
annotations targets a file under `.claude/tools/` and is executed normally.

### §4.2 The three tiers (D3), per surface

A **candidate** is a line matched by the lax recogniser above. For each candidate, let *k* be the number
of `` ` `` characters appearing on that line before the `<!--`.

- **mention** — the `<!--` is inside a fenced block (fence arm), **or** *k* is odd (backtick arm).
  Ignored.
- **strict** — not a mention, and the text from `<!--` onward matches §4.4's production. Executed.
- **malformed** — anything else. **Fails**, naming file, line and the first field that did not parse.

The **fence arm is defined per surface** — "fence" is not a universal concept, and specifying it for two
of four surfaces while marking the row a full enum is how the first draft left `.sh`/`.toml` unruled:

| Surface | Fence arm | Backtick arm | Evidence |
|---|---|---|---|
| `.md` | a line whose first non-space run is ` ``` ` or `~~~` opens/closes; only a matching marker closes | yes | `preflight.py:87-115` `_fence_state_array`, the in-repo shape being followed |
| `.rs` | same, **after** stripping a leading `//`, `///` or `//!` | yes | see the two commands below |
| `.sh` | **none** | yes — the sole inert spelling | the fence-marker sum below is **0** |
| `.toml` | **none** | yes — the sole inert spelling | same command |

The `.rs` strip is load-bearing, not cosmetic: a line-start-anchored fence matcher sees **zero** of a
non-empty set. The property is what matters here, not the 138, which belongs to whatever `crates/` looked
like at this memo's base.

```
git grep -cE '^[[:space:]]*//[/!][[:space:]]*```' -- 'crates/**/*.rs' | awk -F: '{s+=$NF} END{print s}' → 138
git grep -cE '^[[:space:]]*```'                   -- 'crates/**/*.rs' | awk -F: '{s+=$NF} END{print s}' →   0
git grep -cE '(```|~~~)' -- '*.sh' '*.toml' ':(exclude).claude/tools/**' | awk -F: '{s+=$NF} END{print s+0}' → 0
```

For `.sh` and `.toml` the consequence is stated rather than left implicit: **a documentation-example
annotation in a shell or TOML comment must be backticked.** That is a rule, not a hole — it has an inert
spelling and §8 C5 proves both directions. Narrowing the scan to `.md`/`.rs` instead was rejected: it
would make an annotation in `mise.toml` silently unseen rather than checked, the direction D3 forbids.

### §4.3 The pattern-body lint (D5)

D5 is binding and is not reopened, nor is its reject set re-derived here: **`\` followed by any of
`bBdDsSwW<>`**, on D5's stated reasoning that `\X` is the literal `X` in POSIX ERE and therefore that the
escape's ERE meaning is not the author's meaning. S1 implements that set as written.

S1 adds one requirement D5 does not state, and it came from building the controls rather than reading the
decision: **the diagnostic must name the offending escape verbatim** — *pattern contains `\d`; `git grep
-E` is POSIX ERE, where `\d` is the literal `d`, not a digit class. Write `[0-9]`, `[[:space:]]`, or
`([^A-Za-z]|$)` for a boundary.* The first draft gave every escape one diagnostic and keyed two controls
on the shared substring `POSIX ERE`, so each would have passed if the *other* arm fired — defeating the
purpose of `st_probe`'s `$3` (`scripts/trip-wires.sh:130-136`: status alone "cannot tell 'root
verification works' from 'retention masked its absence'"). Named, `contains \b` and `contains \d`
discriminate.

### §4.4 The three verbs

```
annotation := "<!--" WS "claim" WS verb WS "-->"
verb       := count-verb | files-verb | lines-verb
count-verb := "count=" INT WS [ "icase" WS ] "pattern=" QSTR WS "in=" QSTR
files-verb := "files=" INT WS [ "icase" WS ] "pattern=" QSTR WS "in=" QSTR
lines-verb := "lines=" INT WS                                  "in=" QSTR
QSTR := '"' <any run of characters, none of which is '"'> '"'
WS   := one or more spaces or tabs        INT := [0-9]+
```

**Anchoring: `annotation` is NOT anchored to either end of the line** — it may begin at any column and be
followed by anything, which is what makes an annotation inside a GFM table cell strict rather than
malformed, and §8 C1 depends on it. (The first draft's production was silent on this, so its table-cell
control could not be derived from the text it was meant to test.) Field order is fixed; an annotation is
one line, because `git grep` is line-based; `pattern=` may not contain a `"`.

| Verb | Executed as | Semantics |
|---|---|---|
| `count` | `git grep -nE[i] -e "$pattern" -- "$spec" \| wc -l` | **matching lines**, not matches: a line with two matches counts once. `grep -cE 'sole'` on a line reading `sole sole` gives 1 where `grep -oE … \| wc -l` gives 2 — the wire commits to the former, because that is how the umbrella's landed annotations were produced (§2's block pipes `git grep -n` into `wc -l`), so the other choice would falsify them. |
| `files` | `git grep -lE[i] -e "$pattern" -- "$spec" \| wc -l` | files containing ≥1 match. |
| `lines` | `awk 'END{print NR}' "$file"` | takes **no** `pattern`; `in=` must resolve to exactly one tracked file. |

`lines` uses `awk 'END{print NR}'` rather than `wc -l` for a structural reason, not a measured one:
`wc -l` counts newlines, so the first file shipped without a trailing newline is undercounted by exactly
1 — silently, in the direction that makes a claim look stale.

Three execution rules, each measured:

```
$ git grep -cE 'fetch-depth' -- '.github/workflows/ci.yml'; echo "exit=$?"     → exit=1  (no output)
$ git grep -nE 'fetch-depth' -- '.github/workflows/ci.yml' | wc -l; echo "$?"  → 0 / exit=0
$ git grep -nE '-> u32' -- 'crates/**/*.rs'                  → error: unknown switch `>'
$ git grep -lE -e '-> u32' -- 'crates/**/*.rs' | wc -l       → 24
$ git ls-files -- 'crates/**/*.rss' | wc -l                  → 0   (git grep -cE: exit 1, no output)
```

1. **`count=0` is legal and must not abort.** `git grep` exits 1 on no match, which `set -euo pipefail`
   turns into a dead wire. Piping into `wc -l` absorbs it (the pipeline's status is `wc`'s) — which is
   why `count` and `files` are spelled as pipelines above.
2. **`-e` is not optional**, and belongs to D6's family: a pattern beginning with `-` is otherwise read
   as an option and the run dies before measuring anything.
3. **Liveness (D4) runs before every measurement.** `git ls-files -- "$spec" | wc -l` must be ≥1
   (`lines=`: exactly 1). That is the discriminator D4 names: a typo'd pathspec makes `git grep -cE` exit
   1 with no output — indistinguishable from a correct pathspec matching nothing — while `git ls-files`
   separates the two.

**Argv (D6), and how I5 becomes probeable.** The parser emits `$verb`, `$expected`, `$icase`,
`$pattern`, `$spec` as separate shell variables; the executor spells the call literally with every
expansion double-quoted and never builds a command string. No `eval`, anywhere. The first draft called
I5 unprobeable ("enforced by code shape and by review"). It is not — D6's hazard reproduces hermetically
in a three-file fixture (`docs/a.md`, `docs/x/b.md`, `docs/x/y/c.md`, `git add`ed), under `/bin/bash`:

```
p="docs/**/*.md"
git grep -lE -e alpha -- "$p" | wc -l   → 2      # one argv element: git's pathspec, * crosses /
set -- $p; echo "$# word(s): $*"        → 1 word(s): docs/x/b.md
git grep -lE -e alpha -- $p   | wc -l   → 1      # what a shell-globbing executor computes
```

No error, no warning — just a different answer. Two things this makes precise. The expansion is a
*single* word, so it is pathname expansion, not word splitting: "quote it in case it contains spaces" is
the wrong reason and would have missed it. And it must be run under `bash` — `zsh` neither splits nor
globs an unquoted parameter expansion by default, so the same probe under this environment's login shell
reports 2/2 and proves nothing. §8 C12 is this fixture.

---

### §4.5 What bounds the wire's cost, and what does not

The first draft carried a runtime table and anchored the ungated job's budget to `timeout-minutes: 2`.
Both are withdrawn. **The runtime numbers are removed rather than re-measured**: three independent passes
over the same table disagreed while each stayed internally consistent (two sit side by side in umbrella
§6, one annotated "on a busier machine"), which is not noise around a true value but different
conventions about what to include, and a fourth pass picks a fourth convention. **And
`timeout-minutes: 2` is not a budget** — it is orders of magnitude above anything the wires do, so it
catches a hang and nothing else; presenting it as the anchor replacing "~1s" would read as *stronger*
than the number it replaced while bounding strictly less. Stated plainly: **I7 is not machine-bounded at
a useful granularity, and S1 does not make it so.**

What *is* asserted is structural, not measured: **the wire's work is one tree-wide `git grep` plus a term
per annotation — never a term per tracked file.** That is visible in the code shape (§4.1's two stages,
§4.4's per-annotation probe and measurement) and it is what keeps the ungated job's real justification —
no toolchain, no cache (`.github/workflows/ci.yml:128-130`) — true as annotations accumulate. The wire
header records a re-runnable escalation trigger rather than a prediction: *if the per-annotation stage
ever dominates a local `mise run trip-wires`, batch by `in=` before adding annotations.* §9 audits that
and declines to mint a slot.

**The bound is on the wire, and the self-test is exempt** (I6×I7). C10's positive and C19's corpus row
both run tree-scale pathspecs, deliberately: the acceptance corpus is only worth having because it runs
against the real tree (§7 Q2), and a hermetic re-enactment would drop the "known input, known verdict"
property that is its whole value. Left unstated, I7 reads as forbidding the corpus that §8 then ships.
The self-test runs once per wire invocation, not once per annotation, so it does not scale with either
the tree or the annotation count.

### §4.6 The stale prose claim

Two sites carry a runtime number nobody re-executed — the failure class this program exists for. Both
lose the number and keep the falsifiable part (no toolchain, no cache), which is what the ungated job's
rationale actually rests on. Neither gains a replacement: per §4.5 there is no honest one, and the
CI-side figure is not measurable from a workstation at all.

```
$ grep -n '~1s' .github/workflows/ci.yml CLAUDE.md
.github/workflows/ci.yml:129:  # are filtered because each pays for a Rust toolchain; the wires need none (~1s for
CLAUDE.md:94:… wire は toolchain 不要 (~1s) なので filter は何も買わない。…
```

The set-level label the new wire joins is the second prose surface: S1's wire gates documentation
provenance, not layering, so either the label widens or the deliverable is mis-filed. Edit set — the
**pre-edit** state, which §5 commit 3 is what changes:

```
$ git grep -niE 'layering trip-wire' -- ':(exclude)docs/plans/**' ':(exclude).claude/tools/**'
.github/workflows/ci.yml:145:    name: Layering trip-wires
.github/workflows/ci.yml:153:      - name: Run layering trip-wires
CLAUDE.md:66:… `trip-wires` (layering trip-wires = `scripts/trip-wires.sh`、CI の同名 job と同一) …
CONTRIBUTING.md:19:   layering trip-wires — the last of these also run as their own CI job, so a drifted
mise.toml:100:description = "Run every layering trip-wire (.claude/tools/*-trip-wire.sh)."
scripts/trip-wires.sh:4:# run every layering trip-wire. The glob below DISCOVERS what is present;
```

`CONTRIBUTING.md:19` is **S2's**, per umbrella §6 — its sentence continues into `:20` with "a drifted
trip-wire *allowlist* reds the PR", a mechanism S2 ships. S1 edits the other four files; each wire's own
header keeps saying what that wire is, and only the set-level label moves. §7 Q1 asks for the wording.
(The two exclusions are what keep this listing stable under its own landing — the memo lives in
`docs/plans/`, the wire and self-test in `.claude/tools/`.)

**Two more sites, which that command cannot reach.** Round 2 found them and the checker now reports
them, which is the more useful fact: `claim-gate-plan-check.py`'s `SITES` pass enumerates every line in
the five governing files that speaks about the wires *collectively*, and lists the ones this memo does
not cite. It finds 13; two carry a claim a fifth wire falsifies:

| Site | Today | Why the greps above miss it | S1's edit |
|---|---|---|---|
| `.github/workflows/ci.yml:152` | "No toolchain step: the wires are grep-only." | contains neither `~1s` nor `layering trip-wire` | keep "no toolchain step" (that is the load-bearing half and stays true); drop "grep-only", which the new wire's `git ls-files` probe, `awk` and self-test invocation falsify |
| `mise.toml:102-103` | "The wires cover disjoint layering / discipline surfaces (wasm-runtime tiers, VM-side wasm, native-ctor guards, the LayoutBox reader audit)" | same | the parenthetical **enumerates the wire set**, so it goes stale on every addition; replace it with the property it was reaching for — each script's own header documents its surface, which `:104-105` already says |

**The other five `SITES` reports, left alone deliberately.** A report is only useful if every row is
disposed of, so: `.github/workflows/ci.yml:131-132` states why the job is ungated (gating it would mean
listing `.claude/tools/**` in the path filter) — a fifth wire does not touch that argument;
`CLAUDE.md:52` lists `trip-wires` among `mise run ci`'s dependencies, which stays true;
`scripts/trip-wires.sh:25` and `scripts/trip-wires.sh:59` are the driver's own internals (`bash` 3.2, and the both-directions
retention message), which describe the mechanism rather than the membership. None is falsified by an
addition to the set.

This is the general shape and it is worth naming, because it is why the edit set was wrong twice: a
grep proves a *paste* matches its command, never that the command was the right one. `TRANSCRIPT`
re-runs §4.6's two listings and confirms both are complete for what they ask; `SITES` exists because
that is not the same as being complete. Both edited sites were reachable only by asking "what else
describes the *set*", which is a concept, not a string
(memory `feedback_semantic-sibling-selfseed-and-regate-breadth`).

---

## §5 Decomposition and sequencing

Two files, per §1:

| File | Owns | Projected |
|---|---|---|
| `.claude/tools/claim-provenance-trip-wire.sh` | header rationale, the self-test invocation + guard, the stage-1 scan, the tier pass, the field parser, the D5 reject-list, the D4 probe, the three verb executors, verdict and exit | ~350 |
| `.claude/tools/claim-provenance-selftest.sh` | the fixture repos and every control in §8 | ~200 |

**Three live annotations, one per verb**, each chosen so that S1's own landing cannot invalidate it (D10
form 1). Once S1 lands the wire re-executes all three on every push, so each reds the build if its
measured set moves.

The nearest comparable wire is 493 lines
<!-- claim lines=493 in=".claude/tools/layout-box-reader-trip-wire.sh" -->
— `lines=` against a pathspec resolving to exactly one tracked file, and §1's line-count argument made
falsifiable.

<!-- claim files=1 pattern="REQUIRED_WIRES" in="scripts/*.sh" -->
The registered wire set has exactly one home — the property `scripts/trip-wires.sh:10-17` argues for
("One script rather than the same loop inlined at both call sites"). It is also the corrected form of one
of v1's three false annotations (`files=7`, which measured 1 — umbrella §0.1). S1 adds no file under
`scripts/`, so the value survives its landing.

<!-- claim count=0 pattern="fetch-depth" in=".github/workflows/*.yml" -->
No CI job requires a deep checkout — the blocker D1 rests on, and the firing trigger umbrella §5 gives
`#11-claim-annotation-diff-scoped-lint` ("the first job that requires `fetch-depth: 0`"). As an
annotation that trigger stops being a prose promise: the day a job adds one, the wire reds.

Two corrections to the form this took in revision 2, both of the same kind — **the annotation must
cover what the sentence claims**, and a figure beside it must not be the thing that goes stale first.

- The pathspec was `.github/workflows/ci.yml`, while the sentence says *no CI job*. There are three
  workflow files (`git ls-files -- '.github/workflows/*.yml' | wc -l` → 3), and `audit.yml` and
  `codeql.yml` were outside the gate the sentence promised. Widened to `'.github/workflows/*.yml'`,
  which is still one `QSTR` and therefore still one argv element (I5).
- Revision 2 added "All five `actions/checkout@v6` uses run at the default depth". Five is
  convention-dependent — it is five in `ci.yml` and seven across the workflow directory — and the
  version is a moving target a Dependabot PR rewrites. It bounds nothing the annotation does not
  already bound, so it is **removed** rather than re-measured
  (memory `feedback_convention-dependent-figures-are-argument`).

**Commit sequencing (one PR, ordered commits):**

1. `claim-provenance-trip-wire.sh` + `claim-provenance-selftest.sh` + **the `REQUIRED_WIRES` line**, in
   one commit. The registration cannot trail: the unregistered-wire check (`scripts/trip-wires.sh:231-237`)
   fails the moment the file appears, so any split here is a red intermediate commit.
2. §4.6's `~1s` deletion at `.github/workflows/ci.yml` and `CLAUDE.md`, both sites, one commit.
3. §4.6's set-label widening at the four files, one commit.

Gate: `claim-gate-plan-check.py` (both memos, and `--self-test`) → `cargo fmt --all` → `mise run ci` →
`/simplify` → `/code-review` → `/review` → `/elidex-review`. The checker runs **first** because every
stage after it reads the memo, and a stale figure in the memo costs a reviewer's round rather than a
second. `mise run ci` is not cargo-only — `mise.toml:116` lists `trip-wires` among its `depends`, so
from this PR onward the local gate exercises the deliverable. `/review` needs a PR, so the order is
push → draft PR → `/review <n>`.

---

## §6 What S1 does not close

- **Annotation deletion is unrecorded.** D8's ledger is S2's. Until it lands, deleting an annotation
  turns the wire green with no trace — the theory `scripts/trip-wires.sh:76-84` explicitly rejects for
  `REQUIRED_WIRES`. S1 ships no partial ledger; the fix is S2, in the forced order.
- **Check 3 rejects the strongest artifact until S3 lands.** `grep_pass.py:76-80`'s
  `VERIFICATION_ARTIFACT_RE` accepts exactly three spellings — a `grep -[rlnEFhio]+` literal,
  `verified YYYY-MM-DD`, and a `lines N/N/N` list — and **not** a `claim` annotation. So between S1 and
  S3 there are four accepted provenance forms, of which the only *executed* one is the one the tool
  rejects; this memo passes Check 3 by pasting `git grep -n…` literals, not by carrying annotations.
  D9 retires the other three and admits the annotation; **S3 closes this window.**
- **The tier pass would be a third fence-tracking implementation.** That is the whole of the bound, and
  revision 2 stated it as something larger and false: that the two in-repo trackers already behave
  differently on a mismatched marker, sourced from `grep_pass.py:254`'s `— content` comment — which
  means content *of the still-open block*, i.e. the same verdict `preflight.py:111` reaches by a
  different route. That claim was the sole premise for accepting a third copy, and for all of §7 Q3.

  The relation between the two is now **measured, not described**: `claim-gate-plan-check.py`'s `FENCE`
  check runs both over a corpus carrying a mismatched marker in each direction and an unclosed fence at
  EOF, and reports the verdict every run. The check also **forbids this memo from restating that verdict
  in either polarity**, because a boolean the memo's own edits can invalidate is exactly what D10 form 2
  says to cite rather than write out — and because a checker that reads English polarity would be the
  same unexecuted-claim medium one level down. So the bound is stated without the number of behaviours:
  a third copy of an algorithm is a third place to fix a bug, D7's shared-word-list remedy does not
  transfer to an algorithm, and §7 Q3 now asks the question that survives — whether to have a third copy
  at all.
- **`in=` is exactly one pathspec element.** One `QSTR` → one argv element (that is I5). Git does accept
  an *exclude-only* pathspec (verified: `git grep -lniE 'layering trip-wire' -- ':(exclude)docs/plans/**'`
  returns hits), so one exclusion is expressible; two are not — which is why §4.6's edit-set listing is a
  pasted command rather than a live annotation. Found while building §5's annotations, per the rule that
  a gap met while writing a control is a finding about the spec, not a detail to complete.
- **No verb expresses a wall-clock, environment or process claim.** All three measure the tracked tree,
  so "~1s for the whole set" is not re-expressible as an annotation — which is why §4.6 deletes it
  instead of converting it.
- **S1 is opt-in** (umbrella §4): it executes the annotations that exist, and a numeric claim written
  without one stays silent. Umbrella §5's `#11-claim-annotation-diff-scoped-lint` owns that, blocked on
  the depth-1 checkout §5's `count=0` annotation now watches.
- **The gate does not scan its own files** (D2) — necessary for S1 to land, and also a tamper path.
- **Untracked files are invisible**: `git grep` without `--untracked` reads the index, so an annotation
  is inert until staged. That is the right scope (the gate governs the committed tree, matching D4's
  tracked-file rule) and it is why D10 form 3's base here is an index, not a commit.
- **Single-line annotations only** (§4.4), and **`count` counts matching lines, not matches** — an author
  who meant "occurrences" measures something else, correctly and forever.
- **The wire re-executes; it does not judge.** A pattern that measures the wrong thing stays green. The
  canonical instance is a substring swallowing its own boundary — `console` contains `sole` — where bare
  and boundaried are both "correct" re-executions and only the author knows which was meant. §8 C0 keeps
  a control for it precisely because the wire cannot.
- **A pattern may match its own annotation, and this is permitted.** If `in=` covers the annotating file,
  the annotation's own line is measured and must be counted in; §8 C17 pins both directions. (The first
  draft said both at once — its §6 permitted this and its control 10 failed it.)
- **CI reds the PR; it does not block the merge.** No required-status-check rule on `main` — the same
  caveat `layout-box-reader-trip-wire.sh:60-63` and `.github/workflows/ci.yml:140-143` already record.

---

## §7 Open questions for plan-review

**Q1 — set-label wording.** §4.6 widens the label at four files; is "repository trip-wires" the right
replacement for "layering trip-wires", or should S1's wire get its own driver? Recommendation: same
driver, label widened, one commit. A second driver costs a second ungated CI job and a second place a
wire can go missing — and §1 shows the *first* driver's glob is already the thing that decides what runs.

**Q2 — should the acceptance corpus run against the live tree?** §8 C19–C22 re-execute v1's three false
annotations and one of #504's `lines=` claims verbatim against the real repo, so the self-test asserts
FAIL and a diagnostic naming the historical asserted value. They are self-defending (if the tree ever
agrees with a wrong historical figure the control reds and says the row went vacuous), but they do couple
the wire's self-test to files other lanes own. The alternative is hermetic re-enactments of the same
three *shapes*, which is deterministic but no longer "known input, known verdict". Recommendation: keep
them live — umbrella §3 rows 4–5 route these exact inputs here, and umbrella §2's worked instance is the
argument that a real firing beats a synthetic one.

**Q3 — should the wire have its own fence tracker at all?** Revision 2 asked a different question,
resting on a premise §6 now records as false, so this is a fresh one. The wire is `bash`; the two
existing trackers are Python inside `.claude/skills/elidex-plan-review/`. Options: (a) a third
implementation in `awk`, controlled by §8 C2/C3/C5 — what this memo specifies; (b) the wire shells out
to a small Python helper that imports `preflight._fence_state_array`, making one implementation the
single source and deleting the question; (c) drop the fence arm and require every illustrative
annotation to be backticked, which §4.2 already requires for `.sh`/`.toml`, reducing four surface rules
to one.

Recommendation: **(a), and note that (c) is the one worth arguing about.** (b) puts a Python
interpreter in the path of a wire whose entire justification for running ungated is that it needs no
toolchain (`.github/workflows/ci.yml:128-130`), and couples a repository gate to a skill directory's
internals. (c) is genuinely simpler and would retire a whole column of §4.2's surface table, but it
changes what authors may write in `.md` — the surface where nearly every illustrative annotation lives —
and that is a decision for plan-review, not a detail. Whichever way (a)/(c) goes, no fence
reconciliation work is created, so nothing is routed to S3 (§9).

---

## §8 Test strategy

Every control below derives from a **specific clause of §4's text**, named in column 2, and was built by
reading that clause rather than by describing what the wire is meant to do. Where a clause could not be
turned into a control without inventing a rule, the gap is recorded as a finding — §4.4 (anchoring), §4.3
(the diagnostic must name the escape), §6 (`in=` is one pathspec) — instead of being quietly completed.

Controls live in `claim-provenance-selftest.sh`, which the wire invokes from its own preamble under a
recursion guard, in the shape `scripts/trip-wires.sh:113-184` uses. Hermetic controls run against a
`mktemp -d` fixture repo built with `git init -q . && git add -A`; no commit is needed, since both
`git ls-files` and `git grep` see added-but-uncommitted files (§4.4's argv block is such a fixture). The
probe helper follows `scripts/trip-wires.sh:137`'s `st_probe` — `$1` expected `pass`/`fail`, `$2` label,
**`$3` a substring the diagnostic must contain** — every call an operand of `||` per the `set -e` note at
`scripts/trip-wires.sh:163-164`.

**What `$3` has to discriminate.** Revision 2 asserted that no two `$3` values in the table below share
a substring. That was false where it stood — bare `asserted=` was the whole `$3` of nine controls, and
sat inside five others — and, more usefully, it was the **wrong requirement**, generalised from R1's
real finding about C8 and C9 without asking what made those two a problem. What made them a problem was
that they shared a *fixture*: two arms in one wire invocation, each keyed on `POSIX ERE`, so either
would satisfy the other's assertion. Two controls in **separate** fixtures cannot mask each other at
all, because each invocation can only emit its own diagnostics.

So the property is scoped to the invocation, and the table below carries the **Fixture** column that
makes it decidable:

> Within one fixture, no negative arm's `$3` is a substring of another's — and each is specific enough
> to separate the failure the control intends from every *other* failure that fixture can produce. A
> wrong-count failure prints `asserted=<N>`; a dead pathspec prints `matches no tracked file`; a
> rejected escape names the escape. Those are already disjoint kinds, which is why bare `asserted=` is
> adequate in a one-annotation fixture and inadequate the moment two live in one.

`claim-gate-plan-check.py`'s `UNIQ` check enforces the scoped form, grouped by the Fixture column. The
`live` group is the one where this bites: C19–C22 run four annotations against the real tree in a single
invocation, so their `$3` values must be — and are — mutually non-substring.

### §8.1 Controls

| # | §4 clause under test | Fixture | Positive (must pass) | Negative (must fail; `$3`) |
|---|---|---|---|---|
| C0 | §6 "the wire does not judge" | own | fixture with `console.log` and `the sole caller`; boundaried pattern, true count | bare `sole` pattern carrying the boundaried count (`asserted=`) |
| C1 | §4.4 "not anchored to either end" | own | annotation inside a GFM table cell, true count → executes | same cell, wrong count (`asserted=`) |
| C2 | §4.2 fence arm, `.md` | own | ` ``` `-fenced annotation with a **deliberately wrong** count → ignored | identical line unfenced (`asserted=`) |
| C3 | §4.2 fence arm, `.rs` after the `//[/!]?` strip | own | rustdoc-fenced annotation, deliberately wrong count → ignored | unfenced sibling in the same `.rs` (`asserted=`) |
| C4 | §4.2 backtick arm, odd *k* | own | backticked annotation, deliberately wrong count → ignored | same line with the backticks removed (`asserted=`) |
| C5 | §4.2 `.sh` has no fence arm | own | backticked annotation in a `#` comment in a `.sh` → ignored | same `#` comment unbackticked, wrong count (`asserted=`) |
| C6 | §4.2 malformed fall-through | own | — | `<!-- claim cont=3 pattern="x" in="a.md" -->` (`MALFORMED`) |
| C7 | §4.4 `lines-verb` takes no `pattern` | own | `lines=` alone, true length | `lines=5 pattern="fn" in="a.md"` (`MALFORMED`) |
| C8 | §4.3 reject `\d` | escapes | `[0-9]` accepted, true count | pattern containing `\d` (`contains \d`) |
| C9 | §4.3 reject `\b` | escapes | `([^A-Za-z]\|$)` accepted, true count | pattern containing `\b` (`contains \b`) |
| C10 | §4.4 liveness, `in=` ≥ 1 | own | `in="crates/**/*.rs"` | `in="crates/**/*.rss"` (`matches no tracked file`) |
| C11 | §4.4 liveness, `lines=` exactly 1 | own | one-file pathspec | `in=".claude/tools/wasm-*-trip-wire.sh"`, which resolves to 2 (`resolves to more than one`) |
| C12 | §4.4 argv fidelity (I5) | own | §4.4's three-file fixture, `files=2 … in="docs/**/*.md"` → passes only if the pathspec reached git as one argv element | `files=1` with the same pathspec (`asserted=1`) — the value a shell-globbing executor would compute |
| C13 | §4.4 `-e` is not optional | own | `pattern="-> u32"`, true count | same pattern, wrong count (`asserted=`) — without `-e` the run dies on `unknown switch` and neither direction is reachable |
| C14 | §4.4 `count` = matching lines | own | fixture line reading `sole sole`, `count=1` | `count=2` on the same fixture (`asserted=2`) |
| C15 | §4.4 `count=0` must not abort | own | `count=0` on a pattern that matches nothing → passes, wire continues | `count=1` on the same (`asserted=1`) |
| C16 | §4.1 D2 self-exclusion, **both** directions | own | wrong-count annotation under `.claude/tools/` → not scanned, stays green | the byte-identical annotation one directory up (`asserted=`) |
| C17 | §6 self-matching is permitted | own | annotation whose pattern matches its own line, count **including** that line | same annotation, count excluding it (`asserted=`) |
| C18 | §1 the wire invokes its self-test | own | wire + helper copied into a fixture dir → runs | wire copied **without** the helper (`self-test helper`) |
| C19 | acceptance corpus — v1 `count` (umbrella §0.1) | live | — | `count=299 pattern="#\[test\]" in="crates/shell/elidex-shell/src/**/*.rs"` (`asserted=299`) — the pathspec says `src/` where 299 was the whole crate |
| C20 | acceptance corpus — v1 `files` | live | — | `files=7 pattern="REQUIRED_WIRES" in="scripts/*.sh"` (`asserted=7`) |
| C21 | acceptance corpus — v1 `lines` | live | — | `lines=1219 in="docs/plans/2026-07-app-mode-turn-completion-drain.md"` (`asserted=1219`) — 1219 was a line *number* |
| C22 | acceptance corpus — #504 §5, the row umbrella §3 row 4 routes here | live | — | `lines=632 in="crates/shell/elidex-shell/src/app/drain_host/mod.rs"` (`asserted=632`) |
| C23 | §4.4 the optional `icase` keyword | own | fixture carrying `SOLE` and `sole`; `count=2 icase pattern="sole"` | the same annotation **without** `icase`, still asserting 2 (`asserted=2 `) — proves the keyword is read, not ignored |
| C24 | §4.2 `.toml` has no fence arm | own | backticked annotation in a `#` comment in a `.toml` → ignored | same `#` comment unbackticked, wrong count (`asserted=`) |
| C25 | §4.3 the reject set is **all ten** of `bBdDsSwW<>` | own | — | ten annotations, one per escape, each asserted MALFORMED with its own escape named (`contains \`, ten times, each call passing the specific escape) |
| C26 | §8.2's live-tree property, mechanised | live | `claim-gate-plan-check.py --self-test` exits 0 | the checker's own probes are the negative arms (`probes ok`) |

C23–C25 close three clause-shaped holes round 2 named: the `icase` keyword, the `.toml` surface (C5's
row claimed both `.sh` and `.toml` and its fixture is a `.sh`), and the reject set, of which two of ten
characters were controlled. C25 is one row rather than ten because the ten differ only in the character
fed to the same clause — a loop over the set, asserting the diagnostic names *the escape it was given*,
which is the property §4.3 states and which ten hand-written rows would restate ten times.

C26 is the checker itself, run as a control. It is in the `live` group because it reads the tracked
tree, and it is the reason §8.2's acceptance property is no longer only prose.

C2–C5 and C16 all use the **wrong-count-inert / right-shape-live** pairing rather than a merely-ignored
annotation, so a regression in an inertness rule reds the *positive* control instead of leaving it
vacuously green.

C19–C22 are the corpus umbrella §3 rows 4–5 route to S1: known inputs whose correct verdict is known
independently of this program. They run against the live tree (§7 Q2). Their asserted values are
historical constants, so the assertion — FAIL, with the diagnostic naming that constant — does not
depend on any live figure; if the tree ever drifted into agreement the control would red and say the
corpus row had gone vacuous, which is the honest outcome.

### §8.2 Live-tree acceptance

The umbrella carries two live annotations and this memo carries three (§5). Once the wire lands it
executes all five on every push, so they are an acceptance criterion rather than a side effect. The
criterion is stated as a **property**, because the tier populations are exactly the figure D10 forbids
asserting:

> Every strict annotation in the tracked tree passes; no candidate line classifies MALFORMED; every
> illustrative annotation in either memo — fenced or backticked — classifies as *mention*; and at least
> one strict annotation exists for each of the three verbs.

The last clause stops the criterion being satisfiable by a wire that classifies everything as a mention.
The property was **executed**, not asserted: a throwaway classifier transcribed from §4.2/§4.4's literal
text, plus a throwaway executor for the three verbs, run over both memos at the base declared at the top
of this file (an index containing it — D10 form 3).

```
$ git grep -nE '<!--[[:space:]]*claim[^A-Za-z]' \
    -- '*.md' '*.rs' '*.sh' '*.toml' ':(exclude).claude/tools/**' | wc -l
       8
$ python3 <prototype-classifier> <this memo> <the umbrella>
--- {'strict': 5, 'mention': 3}
$ bash <prototype-executor>
OK      lines asserted=493  actual=493   in=.claude/tools/layout-box-reader-trip-wire.sh
OK      files asserted=1    actual=1     in=scripts/*.sh
OK      count asserted=0    actual=0     in=.github/workflows/ci.yml
```

Zero MALFORMED, one strict annotation per verb, C6's malformed specimen correctly inert. The per-line
listing is deliberately **not** pasted: every line of it carries a literal `<!-- claim …`, so pasting it
would add candidate lines to the tree being counted and the block would disagree with itself on the next
run — D10's mechanism reaching the transcript instead of the claim. (The same thing happened once more
here: §8.3's prose originally *named* two glob shapes and so contributed two of the soft findings it was
describing.) Building the two prototypes produced one finding each, both folded back into §4: the
backtick arm must be evaluated against the backticks *before* the `<!--`, and the `lines` executor cannot
share an argument position with `pattern` — the production's own "`lines=` takes no `pattern`" clause
surfacing in the implementation, and why C7 exists.

The same executor over §8.1's corpus rows reproduces all four expected failures:

```
FAIL    count asserted=299   actual=80     in=crates/shell/elidex-shell/src/**/*.rs
FAIL    files asserted=7     actual=1      in=scripts/*.sh
FAIL    lines asserted=1219  actual=1251   in=docs/plans/2026-07-app-mode-turn-completion-drain.md
FAIL    lines asserted=632   actual=647    in=crates/shell/elidex-shell/src/app/drain_host/mod.rs
```

The last row is the one to read twice, and its story is not the one it first looks like. `632` did not
go stale: it was **false at the instant #504 landed**, and it is still false today. Measured —

```
git show 3045f256:crates/shell/elidex-shell/src/app/drain_host/mod.rs | awk 'END{print NR}'   → 647
awk 'END{print NR}' crates/shell/elidex-shell/src/app/drain_host/mod.rs                       → 647
git log --oneline 3045f256..HEAD -- crates/shell/elidex-shell/src/app/drain_host/mod.rs | wc -l → 0
```

The file has not been touched since, so the 15-line gap was there at land time. And the column heading
in that table is literally `Lines at HEAD`.

**Revision 2 said this was the one row the correction left behind. It was not, and the real shape is
worse.** Re-measuring every numeric row of that table against `3045f256` — which is the table's own
declared base — gives:

| Row | Asserted | At `3045f256` | Today |
|---|---|---|---|
| `app/drain_host/mod.rs` | 632 | **647** | 647 |
| `app/drain_host/host.rs` | 358 | 358 | 358 |
| `app/events.rs` | 230 | 230 | 230 |
| `app/mod.rs` | 897 | 897 | 897 |
| `app_test_support.rs` | 317 | 317 | 317 |
| `app_turn_completion_tests.rs` | 648 | **662** | 662 |
| `app_history_phase_sep_tests.rs` | 503 | **505** | 505 |
| `elidex-script-session/src/engine.rs` | 709 | **718** | 718 |
| `elidex-js/src/engine.rs` | 830 | 830 | 830 |
| `elidex-js/src/tests_engine_s1c.rs` | 430 | **437** | 437 |

**Five of ten were false at land, and every one of the five is an undercount.** That direction is the
finding. A table that had drifted randomly would miss in both directions; a table that misses in one
direction was measured *once, early*, and never re-measured as the branch kept moving — which is
precisely the failure mode this slice exists to make impossible, caught in the artifact that first
described it.

It also corrects what revision 2 drew from this. #504's §5 drift is the origin lesson's failure 3 — it
was found and it was corrected — and the correction fixed the numbers the finding named while leaving
**five** untouched, not one. That is still the
fix-the-cited-site-leave-the-mirror-sites shape (`feedback_findings-cluster-in-self-added-scope`), but
at five out of ten it is no longer a missed mirror site: the review found one instance of a systematic
error and treated it as an isolated one. Nobody re-ran the column.

The acceptance corpus takes one of the five (C22). The other four are the same input shape with
different constants and would add no discriminating power to the self-test, so they are named here
rather than duplicated as C-rows — the corpus is chosen for coverage of *verdict shapes*, not for
counting instances.

Umbrella §2 records the complementary case — its own two annotations moving when `658cc302` landed
(`FAIL    count asserted=62  actual=64   in=crates/**/*.rs`). The wire's verdict format is taken from
that output, which is why every `$3` above is spelled `asserted=`.

### §8.3 Preflight

```
$ python3 .claude/skills/elidex-plan-review/preflight.py \
    docs/plans/2026-08-claim-gate-s1-provenance.md
  total entries  (M):   8  (data rows, breadth basis)
  malformed rows:       0  (no §<number> reference)
  unmapped-label rows:  8  (has §<number>, label not in SPEC_LABEL_REVERSE)
  unique specs (K):     2
  split decision:       ok (single PR scope)
  grep-pass:            0 hard, 7 soft
EXIT=0
```

All 7 soft findings are one shape: a `crates/…**/…` pathspec read as a filesystem path by Check 1's
matcher, which cannot resolve a glob. The `⚠ unrecognized labels` line for the two in-repo documents §3
substitutes for spec rows is printed *outside* that tally and is not one of the 7 — a decomposition the
first draft got wrong in both directions. Zero Check 3 findings: every enumeration claim in this memo has
a `git grep -n…` literal within its vicinity, which is the only artifact form Check 3 accepts today (§6).

The soft count is 7 rather than revision 2's 5 because this revision added two `crates/…**/…`
pathspecs — §2's I6×I7 row and §4.5's exemption, both naming the tree-scale controls. **This block is a
pasted transcript and therefore a claim**, so it is one `claim-gate-plan-check.py` re-derives: its
`PREFLIGHT` check re-runs preflight against this memo and compares M, K, the malformed and
unmapped-label counts, the hard/soft split and the exit code with what is pasted here. It caught this
exact drift within the revision that caused it — which is the whole difference between a figure and a
figure with a checker.

---

## §9 Defer ledger

**S1 books zero new slots.** The create-time slot-fit audit was applied to the one candidate — *"batch
annotation execution by `in=` when the annotation count grows"*, which I5 × I7 forbids structurally.
*(1) platform gap?* No, a performance property of one script. *(2) repeat signal?* No. *(3) existing
home?* Yes, the wire's own header, where the sibling wires keep comparable notes. *(4) second tracking
home?* Yes. → **Not a slot**: a header note with a re-runnable escalation trigger, the disposition
`.claude/skills/elidex-plan-review/SKILL.md:91` records for its own deferred parser ("deliberately NOT a
`#11-*` platform slot … skill-infra not a platform gap").

**Not S1's deferrals** (owned elsewhere, listed so no one re-books them): D8's ledger → S2;
`CONTRIBUTING.md:19-20`'s allowlist sentence → S2 (umbrella §6); D7/D9 and Check 3, including §6's
four-form window → S3; `#11-claim-annotation-diff-scoped-lint` → booked by umbrella §5, its
`fetch-depth: 0` trigger now watched by §5's `count=0` annotation.

Revision 2's list ended that S3 item with "and the fence-tracker reconciliation", which was a defer to
a slice that does not accept it: umbrella §6's S3 line reads *"D7's shared word lists and the
cross-implementation control"* and names no fence work. So the routing was a claim about another
document that the other document does not make — the same class as the rest of round 2's findings, in
the one section whose job is to be exact about who owns what. It is withdrawn rather than re-routed,
because there is nothing left to route: the premise that there was a reconciliation to do was §6's
false one, and §7 Q3 now asks whether to have a third tracker at all, which is a decision inside S1.

**Own deferrals: 0 of the per-PR ≤3 budget** — unchanged, and now honestly so. Revision 2 reached zero
partly by routing the fence question to S3; with that withdrawn, the count stands on its own.
