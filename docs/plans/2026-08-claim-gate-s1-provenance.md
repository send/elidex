# S1 — the `claim` annotation grammar + the re-execution wire

> ## Revision 6 — a figure that outlived three changes to its own subject, a rule the checker never stopped enforcing, and six citations naming lines nobody re-read
>
> This revision started from a question about the review rather than the memo: rounds 2, 3 and 4 returned
> more CRITs than the round before, and each round's roots were traceable to what the *previous* revision
> had added. Two measurements, taken before round 5 rather than after:
>
> - **§5's `~350`/`~200` stood unchanged in every commit this memo has ever had**, while the design of the
>   wire and of the self-test changed underneath it three times — the fence arm and the second scan stage
>   removed, `--one` added, `--one` withdrawn for a clone fixture to build and tear down. It was never a
>   projection; it was inherited. §1's "nowhere near the ~700–800 band" rested on it. Both are gone (§5),
>   for the reason §4.5's runtime table and §5's "five uses" went before them.
> - **Option (c) was swept into this memo and into the umbrella, and never into the checker.**
>   `classify()` went on putting a fenced annotation in `mention`; the per-surface table and the `.rs`
>   rustdoc strip were still there; and a `--self-test` probe *asserted* the withdrawn tiering, so the gate
>   certified the rule the memo had deleted. In the same file, `UMBRELLA`'s D2 entry already keyed on
>   "there is no fence arm" as proof the slice had withdrawn it. Nothing was red, because the tree carries
>   no fenced annotation for either half to be wrong about.
>
> That second one is round 4's shape — *the sites that state a change were swept, the sites that depend on
> it were not* — occurring inside the mechanism built to make that shape impossible, at the one site §0 and
> §6 had already written down as a duplicate. So the duplicate gets a keeper rather than a fifth sweep:
> `TIERS` derives each classifier branch's tier from §3's own prose and decides it by running `classify()`
> on a specimen, so a branch added without one fails and a tier changed on either side alone fails.
>
> Round 4's remaining findings are closed here too: §4.2 now says a malformed annotation fails the run
> **without ending it** (without which `escapes` and C25 could never go green); §4.1's heading no longer
> promises a second stage its body deletes; and §4.6's `trip-wires.sh:59` label — wrong since revision 3,
> which is the revision that was *answering* round 3's finding about it — is re-derived from the report
> rather than relabelled again.
>
> **`TIERS` is the fourth check of this kind, and revision 5 built the first three.** Its three roots were
> round 4's shape as well: the umbrella still mandated `.rs` fence matching and still defined the mention
> tier as "backticked or fenced" after this slice had deleted the fence arm — which is not a slice's
> change to make, so the umbrella was amended in its own file, marked as a decision change; §8 defined
> what a Fixture value *means* and never re-derived the assignments against it, so round 3's C10/C11
> finding was relabelled rather than closed; and four revisions shipped four false counts about this
> program's own tooling, each in the paragraph crediting it. Revision 4's remedy for that last one was a
> **prose rule** — "this memo states no quantity about its own tooling" — falsified eighteen lines above
> itself by "three commits" (five, and five when written). That settles the method for all of them: a rule
> cannot stop this class, because the author believes the rule was followed. So each root became a check,
> written before its prose and probed in both directions — `TOOLING` re-derives any numeral in a sentence
> about the tooling from git, `UMBRELLA` fails a slice that withdraws what a binding decision still
> mandates (and recognises an amendment, so the decision's history is kept rather than rewritten),
> `FIXTURE` derives §8.1's Fixture column from each row's own `in=` pathspec. Revision 5 also withdrew
> `--one`, revision 4's injected-annotation entry point, replaced by a `git clone --local` fixture that
> lets the acceptance corpus run through **the scan** instead of around it (§4.1.1).
>
> **This memo states no quantity about its own tooling**, and `TOOLING` is what holds it to that now. The
> artifact is these two commands (D10 form 2), re-run against this file at every revision since:
>
> ```
> $ python3 .claude/tools/claim-gate-plan-check.py --self-test | tail -1
> $ python3 .claude/tools/claim-gate-plan-check.py \
>     docs/plans/2026-08-claim-gate-s1-provenance.md \
>     docs/plans/2026-08-quantitative-claim-staleness-gate.md
> ```
>
> **What round 5 returned, recorded because the shape is the program's own subject.** Two were claims of
> this revision's own making. It split `CONTRIBUTING.md:19`/`:20` and gave S1 the label half, on the reading
> that umbrella §6 had misrouted the sentence. The umbrella had not: it lists `:19` among the label sites
> and then carves it out in the same breath. The premise came from §5's account of the umbrella instead
> of the umbrella. **Withdrawn** (§0, §4.6, §5, §6, §9). And it asserted that a quoted span attributed to
> a named file "occurs zero times" across both memos, offered as the ground for shipping no check for the
> class. The probe behind that zero matched within a single line, and every such quotation here wraps —
> §1's `scripts/trip-wires.sh:104-111`, §4.3's `scripts/trip-wires.sh:130-136` and more. **Withdrawn**, with the
> line-scope defect noted: the same defect had been fixed in the `FENCE` ban hours earlier in this same
> revision, and the replacement probe reintroduced it.
>
> **The other six findings were about *where* something is: five citations that named the wrong line, and
> one header stacked where its predecessor should have been swapped out.** Two sites attributed
> `ban_control` to §4.3, which is the pattern-body lint and never mentions it; the precedent is introduced
> in §2's I4×I6 row, against `layout-box-reader-trip-wire.sh:276-299`. §4.6's edit-set row credited
> `mise.toml:104-105` with "each script's own header documents its surface", which is `:103-104` — `:105`
> is the separate sentence about the driver owning the registry. §4.6's quotation of the ungated job's
> argument was attributed to `.github/workflows/ci.yml:131-132` and runs on into `:133`. §8.3 gave one of
> the two added `crates/…**/…` pathspecs to §4.5's exemption, which names C10 and C19 in prose and carries
> no pathspec literal at all; both are in §2's I6×I7 row, and both have been there since revision 3 rather
> than "this revision". §1 pasted the names its own `git ls-files` listing returns and said every tracked
> script in that directory matches `WIRE_GLOB` — a listing and a sentence S1's own commit 1 falsifies, by
> adding a tracked `.sh` there that does not match; §1 now runs the membership test instead of enumerating
> today's answer to it.
>
> The five share a method, and that is why they are a population rather than five accidents: each was
> written by re-reading the prose around the citation instead of the line the citation names — §4.6's
> "relabelling is not re-measuring", arriving at five more sites. `CITE` proves a `path:LINE` is tracked,
> unambiguous and in range; nothing proves it is the line meant, and that gap is stated here rather than
> left to read as covered.
>
> The sixth has a different root and is this block. Revision 6's header was appended above revision 5's
> instead of replacing it: `git show --stat 676ab9df` reports 45 insertions and no deletions, while
> `git show <commit>:<this file> | grep -c '^> ## '` returns 1 at every commit from round 2 up to that one
> and 2 at it and after. Two status blocks at the front door is the duplicated-decision-surface shape,
> with the stale block indistinguishable from the current one and no rule saying which is read. This is
> the single block; what revision 5's still owned is folded into it above, and the record of what earlier
> revisions got wrong is kept, because that record is this memo's thesis rather than its clutter.
>
> → memory `feedback_prose-rules-cannot-fix-unexecuted-claims`,
> `feedback_findings-cluster-in-self-added-scope`,
> `feedback_convention-dependent-figures-are-argument`, `project_stale-claim-gate-v1-review`.


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

Measurement base, stated as a **property** rather than a hash: *the working tree at this branch's head,
with this memo `git add`ed*. It was a hash — `ef18b828` — from revision 3 until round 5 measured it:
`git merge-base --is-ancestor ef18b828 HEAD` is false and `git branch -a --contains ef18b828` is empty.
The prerequisite branch was rewritten underneath it, so the base every D10 form-3 count anchors to had
become a commit no reader could check out. A hash pinning a branch that is still being rewritten is not
a base; it is a claim with the same lifetime as the branch, which is the failure this memo is about.
`git version 2.55.0`, `GNU bash, version 3.2.57(1)-release (arm64-apple-darwin25)` — the wire runs under
`bash`, and §4.4's argv rule is a `bash` fact, not a `zsh` one.

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

**Not in scope, and not in this PR: `.claude/tools/claim-gate-plan-check.py`.** It checks *this memo*;
the wire checks *the tree*. Different subject, different lifecycle, different gate — and revision 3
bundled them, and round 3 charged most of its CRITs to that bundling. It is a **prerequisite PR** that
lands first (`claim-gate-plan-check`); S1 is stacked on it.

Revision 3 claimed the checker's annotation executor was a self-retiring bootstrap: *"a code path, not
a promise, so the two implementations cannot coexist."* **That was false and is withdrawn.** The
`WIRE.exists()` guard covered one function; the self-test called the classifier and executor directly,
so the second transcription of §4.2/§4.4 survived the wire's arrival — and the checker's `PASTE`
verification silently switched off at the same moment, because it consumes the payload that function
stops returning.

The honest replacement is a **bound, recorded in §6**, not a better-timed retirement: **the checker
carries a second transcription of §4.2's tiers and §4.4's verbs, and will keep carrying it until the
wire exists and delegation to it is implemented *and proven by a control*.** Writing the retirement
before the wire is what produced the false claim — the code path existed, and nothing exercised it.
S1's landing is the trigger; the prerequisite PR owns the work; a probe that plants a wire and asserts
the checker's own grammar is unreachable is what closes it.

**Out of scope, by the umbrella's own slicing**: the tamper ledger (D8 → S2), the superlative vocabulary
and `grep_pass.py` Check 3 (D7/D9 → S3), and **the whole `CONTRIBUTING.md:19` sentence**, which umbrella
§6 routes to S2: it counts `:19` among the six set-level label sites and then carves it out — *"`:19`
**also** asserts that a drifted trip-wire **allowlist** reds the PR — a mechanism S1 does not ship and S2
does, so that sentence is S2's to touch, not S1's."* Revision 6 read that as a misrouting and split the
sentence at `:19`/`:20`, giving S1 the label half. **That is withdrawn.** The umbrella considered the
split — the word "also" is it weighing both claims in one line — and decided one sentence has one owner.
A slice cannot reverse that (D2's amendment note states the rule), and amending the umbrella to permit it
has no merit: S2 must rewrite the sentence anyway when it ships the allowlist mechanism, so S1 editing
half of it buys a two-touch sequence on one sentence and nothing else. The residue is real and is a bound
in §6: between S1 and S2, `CONTRIBUTING.md` is the one governing file still calling the set "layering
trip-wires". The forced order makes that window exactly one PR.

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
$ git ls-files -- '.claude/tools/*.sh' ':(exclude).claude/tools/*-trip-wire.sh'
```

The second command *is* the membership test: it lists every tracked script in that directory which the
driver's loop (`scripts/trip-wires.sh:188-205`) and its both-directions retention diff (`:207-237`) will
never see, because `WIRE_GLOB` decides membership **by name, not by directory**. It prints nothing before
S1 and prints `claim-provenance-selftest.sh` after — and that transition, not the set of names it returns
on any one day, is what §1 is about: a helper can sit beside the wires and still not be one.

The command replaces a pasted listing of the four names, and the reason is D10 rather than taste.
Revision 1 pasted them and wrote *"every tracked `*.sh` directly under `.claude/tools/` matches
`WIRE_GLOB` today"*; S1's own commit 1 adds a tracked script there that does not match, so the listing
and the sentence are both falsified by this memo's landing — the preamble's literal prohibition, in the
section whose subject is the file that breaks it. §4.6 keeps its listing stable under the same landing by
excluding what S1 adds (`':(exclude).claude/tools/**'`, stated there); that device is unavailable here,
because the excluded file *is* the argument. So the figure goes and the property stays (D10 form 2).
Consequences, both of which shape S1:

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
and nothing more. So does every wire that exists: all four are single files (`wc -l`: 493 / 121 / 84 /
73) and the 493-line one inlines its own probes. What that does **not** buy is a prediction of S1's own
size, and none is offered — `feedback_touch-time-split-means-while-writing` applies the split discipline
*while writing*, not before, so what belongs here is a re-runnable trigger rather than a projection:
**run `wc -l` on both files at every commit of this PR, and if either reaches the ~700–800 band, split it
at the seam then.** §5 records why the projection it used to carry was withdrawn.

---

## §2 Coupled invariants

Seven invariants hold simultaneously. Each is a way the wire can be green while being wrong.

| # | Invariant | What it forces |
|---|---|---|
| **I1** | **Total tier assignment** (D3) — every candidate line lands in exactly one of *strict* / *malformed* / *mention*; none may vanish | The lax recogniser is the outer net and strict is a subset of it. A typo cannot fall out of the world. |
| **I2** | **Inertness** (D2) — the mention rule must never swallow a strict annotation, and the gate excludes its own files | One rule on all four surfaces: backtick it (§4.2). It is a **line predicate** — *k* backticks before the `<!--` — so no per-file state exists and no surface needs its own arm. `.claude/tools/**` is dropped by pathspec. |
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
| I2 × I7 | Because inertness is a line predicate, the scan is **one** command: `git grep -n` prints `path:line:content`, and the tier pass is a transform of that output rather than a second pass over anything. Revisions 1–3 were two-stage *because* the fence arm needed per-file state; removing it (§4.2) removed the stage, and I7's tree term is now exactly one invocation. |
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
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D2 — inertness) | classify a candidate line | (i) odd *k* backticks before the `<!--` → mention, on every surface | §4.2 tier pass, backtick arm | ✓ | no |
| `docs/plans/2026-08-quantitative-claim-staleness-gate.md` §1 (D2) | classify a candidate line | (ii) inside a fenced block with even *k* → **strict, and executed** (there is no fence arm) | §4.2 tier pass | ✓ | no |
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

### §4.1 The scan

The scan is one tree-wide `git grep` for the lax recogniser over the D2 surface:

```
git grep -nE '<!--[[:space:]]*claim[^A-Za-z]' \
  -- '*.md' '*.rs' '*.sh' '*.toml' ':(exclude).claude/tools/**'
```

The `:(exclude)` magic was verified to actually exclude: `git grep -lE 'strip_comments' -- '*.sh'` → 4
files; with `':(exclude).claude/tools/**'` appended → 0.

**That is the whole scan — one command, not two.** Revisions 1–3 specified a second stage, a tier pass
over the files the first named, and justified it by the fence arm's per-file state. With the fence arm
gone (§4.2) inertness is a line predicate, and `git grep -n` already prints `path:line:content`, so the
tier pass reads the content it was given. The second stage was a consequence of the rule (c) deleted,
and it goes with it — which is the *downstream* face of a withdrawal, the one a sweep of the statements
alone leaves behind (memory `feedback_sweep-obligations-not-only-statements`).

**The name goes with the stage.** Revisions 1–4 called this command "stage 1", and a numbered stage in a
one-command scan names a sibling that no longer exists — the same leftover, one level down in the prose.
Everywhere below and in §5 it is **the scan**; no argument changes, because every use of the old name was
already about this one command.

⚠ D2's exclusion governs where annotations are *looked for*, not what one may *measure*: one of §5's live
annotations targets a file under `.claude/tools/` and is executed normally.

### §4.1.1 The acceptance corpus runs against a clone, not a second entry point

The scan is the wire's only way to acquire an annotation, and round 3 found that this left the acceptance
corpus with **no execution path at all**. C19–C22 supply a known-false annotation and assert the wire
fails on it against a real tree; every placement the scan can see in *this* worktree is closed:

| Placement | Why it cannot work |
|---|---|
| a tracked file here | the annotations are false by construction, so the real gate reds on every push |
| `claim-provenance-selftest.sh` | D2's `.claude/tools/**` exclusion makes it inert — and §2 I2×I6 depends on exactly that |
| a `mktemp -d` fixture | the corpus's tree-scale pathspecs resolve to 0 there, so the D4 liveness probe fires first with a diagnostic the control does not assert |
| an untracked file here | `git grep` without `--untracked` does not see it (§6) |

Every row of that table objects to *this* worktree, not to a real tree. **So the fixture is a real
tree: `git clone --local` of this repository.** The annotation is planted in a tracked file of the
clone, the wire runs there unmodified, and the real gate never sees it.

Three properties are what this rests on, and **each is asserted by a control rather than pasted here**:
the corpus's tree-scale pathspecs resolve in the clone (C10's positive); C19's historical constant
re-measures in the clone to the value it has in this worktree, so the corpus keeps "known input, known
verdict" (§7 Q2); and — the reason this and not a second entry point — **the scan runs in the clone**,
acquiring the planted annotation the same way it acquires every other (C19–C22).

Revision 5 pasted those three as measured figures, plus the clone's wall-clock, its size and a tracked-file
count. All six are gone. The tracked-file count was already false when round 5 measured it (the
prerequisite's own modules had moved it), which is D10's literal prohibition — a tree-wide count this
program's landing changes; the wall-clock and size figures are §4.5's withdrawn class, restored two
sections after being withdrawn. And the whole block was **invisible to the checker**: `git clone` and
`git -C` are not among the programs it re-runs, and until round 5 a non-runnable arrow line was dropped
rather than reported, so four unverifiable figures sat inside a green run. That hole is closed in the
prerequisite; what it now reports is that these lines cannot be re-run — and the preamble's rule for a
claim the checker cannot hold is *remove it*, which is what this is.

**Revision 4 specified `--one '<annotation text>'` instead, and it is withdrawn.** Four things were
wrong with it and only the last is fatal on its own:

- It asserted "it bypasses no rule". It bypasses two — D2's `:(exclude).claude/tools/**` and the
  four-suffix restriction both live in the scan's pathspec and cannot apply to a bare string.
- Its four consumers are all negative-only, so **no arm asserted it can PASS**: a `--one` that always
  failed after echoing the parsed expected value satisfied C19–C22 completely.
- §8's `live` row said the four run in one invocation while §4.1.1 said one annotation per call, and
  §4.1.1's I7 argument depended on the latter.
- It is a code path **in the shipped wire whose only caller is a test** — CLAUDE.md's *dead code は
  接続するか削除*, and a second acquisition path beside the scan, which is *One issue, one way* at the
  wire's own front door.

Cost, stated as a bound rather than a figure: **one clone per self-test invocation**, not per annotation
and not per control, reused by every `clone` control in the run. §2 I6×I7 exempts the self-test from I7
for exactly this class of expense, and the exemption is bounded by that count. No ratio against the
wire's own grep is given: §4.5 withdrew the runtime class for this memo and a ratio is two withdrawn
figures in a trench coat.

**Two rules round 5 found missing, decided here rather than deferred.**

**(a) The wire resolves its scan root from `git rev-parse --show-toplevel`, run in the invoking cwd.**
Nothing said which repository the scan reads, and the default a reader would assume is the driver's:
`scripts/trip-wires.sh:24-56` derives root from `$0` and says *"don't inherit the caller's cwd"*. That is
right **for the driver**, whose job is to find the wire *set* — it must anchor to where the wires are.
It is wrong for this wire, whose subject is *the repository it is pointed at*. Under the driver's rule
the `clone` rows would scan the wire's own tree wherever they ran, and — this is why it had to be found
by asking rather than by testing — they would **still go green**, because C19–C22 assert FAIL with a
historical constant and the real tree produces exactly that diagnostic. The corpus would never touch the
clone and nothing would say so. Two roots, two rules, each stated where its reason lives; that is not a
second entry point (the withdrawn `--one` was one, because it was a second way to *acquire an
annotation* — this is one way, pointed at a different tree).

**(b) The clone is shared for cost, not for isolation: one clone directory, one wire invocation per
control**, with the control's annotation planted and removed between invocations. §8's own masking model
decides this — *"two controls in separate invocations cannot mask each other at all, because each
invocation can only emit its own diagnostics"* — and the alternative reading breaks a stated property:
one invocation over all six `clone` rows exits non-zero by construction (C19–C22 are false by design), so
C10's and C11's **positive** arms could not be read from exit status, which is the only assertion form §8
gives a positive. Consequence, stated because it retires a requirement rather than adding one: §8's
*"Shared ⇒ the `$3` values of all `clone` rows must be mutually non-substring"* is **withdrawn** — it was
inherited verbatim from revision 4's `live` row, which was negatives-only and therefore never had a
positive to strand.

### §4.2 The three tiers (D3), per surface

A **candidate** is a line matched by the lax recogniser above. For each candidate, let *k* be the number
of `` ` `` characters appearing on that line before the `<!--`.

- **mention** — *k* is odd (the backtick arm). Ignored.
- **strict** — not a mention, and the text from `<!--` onward matches §4.4's production. Executed.
- **malformed** — anything else. **Fails**, naming file, line and the first field that did not parse.

**A malformed annotation fails the run without ending it.** Classification and processing are
per-candidate: every candidate line is classified, every strict annotation is measured, every diagnostic
accumulates, and the wire exits non-zero **once, at the end**, after the last candidate — never at the
first bad one. Under `set -euo pipefail` that is a constraint on the code shape rather than a preference:
no arm of the per-annotation stage may call `exit`, and a failing parse or measurement has to be recorded
and returned instead. **`exit` is not the only way out, and it is not the one that nearly shipped**: under
`errexit` + `pipefail` an unguarded pipeline whose left side exits non-zero ends the wire just as
completely, and §4.4 rule 1 spelled every `count` and `files` measurement that way for six revisions. So
the rule is on the *status* rather than on the keyword — every command whose non-zero status is a datum
rather than a failure is captured and branched on (§4.4 rule 1), never left bare. Two of §8's fixtures are unbuildable without it — `escapes` carries C8's `\d` and
C9's `\b` as two negative arms of **one** wire invocation, and C25 drives all ten rejected escapes through
one fixture — so a fail-fast wire would emit the first arm's `$3` and leave every later arm unobservable,
green because it never ran. That is §2 I4×I6's `ban_control` shape (a control that cannot fire is
indistinguishable from a clean tree) arriving through the driver rather than through the diagnostic.

**There is one inertness rule, on all four surfaces: backtick it.** Revisions 1–3 carried a second
rule — a fenced block also makes an annotation inert — defined for `.md` and `.rs` and absent for
`.sh`/`.toml`, which have no fence concept. §7 Q3 put the three options to plan-review and round 3
answered with a measurement rather than a preference:

```
git grep -nE '<!--[[:space:]]*claim[^A-Za-z]' -- '*.md' '*.rs' '*.sh' '*.toml' ':(exclude).claude/tools/**'
  → 8 candidates: 5 strict, 3 mention — and all 3 mentions are BACKTICKED. Fenced mentions: 0.
```

So the fence arm's stated cost of removal — "it changes what authors may write in `.md`" — is **zero
against the tree that exists**, and every illustrative annotation already conforms to the rule that
replaces it. Against that, the fence arm bought: a third implementation of fence tracking (§6 records
what a third copy of an algorithm costs), a per-surface table with two rows saying "none", two controls,
and this open question. *One issue, one way* decides it — the backtick rule is the canonical form
because it is the one that already had to exist for `.sh` and `.toml`, and generalising it retires a
rule rather than adding one.

| Surface | Inert spelling |
|---|---|
| `.md` / `.rs` / `.sh` / `.toml` | backtick the annotation (*k* odd before the `<!--`) |

Two consequences, both stated rather than left implicit:

- **An annotation inside a fenced block is live.** A pasted transcript that contains annotation text is
  executed. This forecloses one form — a fenced multi-line paste of example annotations — and it is a
  form D10 already forecloses for a different reason (§8.2 declines exactly that paste because it would
  change the count it reports). Backticked-inline and GFM-table-cell forms both remain, and §8 C1 pins
  the table-cell case.
- **The `.rs` rustdoc strip disappears with the fence arm**, along with the 138-fence-line measurement
  that justified it. A rustdoc example annotation is inert by backticking, like every other surface.

Narrowing the scan to `.md`/`.rs` was rejected in revision 1 and stays rejected: it would make an
annotation in `mise.toml` silently unseen rather than checked, the direction D3 forbids.

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
| `count` | `git grep -nE[i] -e "$pattern" -- "$spec"`, captured; count non-empty lines (rule 1) | **matching lines**, not matches: a line with two matches counts once. `grep -cE 'sole'` on a line reading `sole sole` gives 1 where `grep -oE … \| wc -l` gives 2 — the wire commits to the former, because that is how the umbrella's landed annotations were produced (§2's block pipes `git grep -n` into `wc -l`), so the other choice would falsify them. |
| `files` | `git grep -lE[i] -e "$pattern" -- "$spec"`, captured; count non-empty lines | files containing ≥1 match. |
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

1. **`count=0` is legal and must not abort — and the pipeline does NOT absorb it.** Revisions 1–6 said
   *"piping into `wc -l` absorbs it (the pipeline's status is `wc`'s)"*. That is true in a plain shell
   and **false under `pipefail`**, where the status is the rightmost *non-zero* one — git's. The wire
   declares `set -euo pipefail`, so the spelling above kills it on the first `count=0` annotation, and
   §5 ships one (`count=0 pattern="fetch-depth"`). Measured, both directions:

   ```
   $ bash -c 'git grep -nE "fetch-depth" -- ".github/workflows/ci.yml" | wc -l; echo rc=$?'
   0 / rc=0                                    # plain shell — what revisions 1–6 pasted
   $ bash -c 'set -euo pipefail; git grep -nE "fetch-depth" -- ".github/workflows/ci.yml" | wc -l; echo SURVIVED'
   0                                           # SURVIVED never prints; script exits 1
   ```

   This is why five review rounds passed over it: **the pasted evidence was honest.** It records a plain
   shell, in which it is true, and the checker re-runs pasted commands in a plain shell — so the
   provenance mechanism confirmed the paste, and the paste was never the claim. Nothing that reads this
   memo can find this class; running `bash -c 'set -euo pipefail; …'` finds it in one line.

   The measurement is therefore **captured, not piped**:

   ```
   if out=$(git grep -nE[i] -e "$pattern" -- "$spec"); then
     n=$(printf '%s' "$out" | grep -c .)
   else
     n=0                       # exit 1 = no match; any other status is rule 4's business
   fi
   ```

   Measured in both directions: `n=0` for the no-match case and `n=15` for `REQUIRED_WIRES` in
   `scripts/*.sh`. Two spellings that look equivalent and are not — `git grep -cE … || true` yields the
   **empty string**, not `0`, because `-c` prints nothing when it exits 1 (rule 3's own observation, one
   step further); and `n=$(… | wc -l) || rc=$?` survives but carries `wc`'s leading whitespace into a
   numeric comparison.
4. **A measurement command that fails for any reason other than "no match" is its own outcome.** `git`
   exits 1 on no match, but 128 on a malformed ERE that §4.3's reject-list does not cover (`foo(` —
   parentheses not balanced) and 129 on an unknown switch. Under the `else` branch above all three
   collapse to `n=0`, so a `count=0` annotation goes green on a pattern git refused to run, and any other
   value reports `asserted=N actual=0` — I1×I4's stated hazard verbatim, where "the author reads a
   mismatch and edits the count". The status is therefore split: 0 → the count, 1 → zero, **anything else
   → a fourth outcome kind with its own diagnostic naming git's stderr**, alongside malformed and
   dead-pathspec. §8 needs a control for it; it has none.
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
per annotation — never a term per tracked file.** That is visible in the code shape (§4.1's single
tree-wide command, §4.4's per-annotation probe and measurement) and it is what keeps the ungated job's
real justification — no toolchain, no cache (`.github/workflows/ci.yml:128-130`) — true as annotations
accumulate. The wire header records a re-runnable escalation trigger rather than a prediction: *if the
per-annotation stage ever dominates a local `mise run trip-wires`, batch by `in=` before adding
annotations.* §9 audits that and declines to mint a slot.

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

`CONTRIBUTING.md:19` is **S2's, whole**, and so is its continuation at `CONTRIBUTING.md:20` ("a drifted
trip-wire *allowlist* reds the PR") — one sentence, one owner. Umbrella §6 lists `:19` among the six
label sites and then carves it out, because the same sentence also asserts the allowlist mechanism S2
ships. This section's criterion (the set-level label) would claim `:19`; the umbrella's disposition
overrides, and a slice does not reverse a hand-off in its own file (§0). So S1 edits the other four
files; each wire's own header keeps saying what that wire is, and only the set-level label moves.
§7 Q1 asks for the wording.

⚠ This is a **reversal within this memo's own history**, recorded rather than smoothed over: revision 6
split the sentence and gave S1 the `:19` half, on the reading that revision 3 had misrouted it. It had
not. The premise was taken from §5's account of the umbrella instead of from the umbrella, which is the
`feedback_ao-name-not-section-number-in-briefs` 追補 4 shape — a claim about another document, sourced
from a third document that describes it. §6 records the one-PR window this leaves.
(The two exclusions are what keep this listing stable under its own landing — the memo lives in
`docs/plans/`, the wire and self-test in `.claude/tools/`.)

**Two more sites, which that command cannot reach.** Round 2 found them and the checker now reports
them, which is the more useful fact: `claim-gate-plan-check.py`'s `SITES` pass enumerates every line in
the five governing files that speaks about the wires *collectively*, and lists the ones this memo does
not cite. Every row it lists is disposed of below — two by an edit, the rest by being named as unaffected — and it fails the run if any is left undisposed, so the population is a property of this section rather than a figure in it. (Revision 3 wrote "It finds 13", which was the *previous* revision of the checker's output; the shipped one reports 7 against the memo that sentence was written from and 0 against this one. Same class as the withdrawn probe count: a claim about the checker, in the paragraph crediting the checker.) The two carrying a claim a fifth wire falsifies:

| Site | Today | Why the greps above miss it | S1's edit |
|---|---|---|---|
| `.github/workflows/ci.yml:152` | "No toolchain step: the wires are grep-only." | contains neither `~1s` nor `layering trip-wire` | keep "no toolchain step" (that is the load-bearing half and stays true); drop "grep-only", which the new wire's `git ls-files` probe, `awk` and self-test invocation falsify |
| `mise.toml:102-103` | "The wires cover disjoint layering / discipline surfaces (wasm-runtime tiers, VM-side wasm, native-ctor guards, the LayoutBox reader audit)" | same | the parenthetical **enumerates the wire set**, so it goes stale on every addition; replace it with the property it was reaching for — each script's own header documents its surface, which `mise.toml:103-104` already says (the range was cited as `:104-105` until round 5 read it: `:105` is the next sentence, about the driver owning the one registry) |

**The other five `SITES` reports, left alone deliberately.** A report is only useful if every row is
disposed of, and a row is disposed of only if the line it names was read at that line. Re-derived from
the report, in the order it emits: `.github/workflows/ci.yml:131-133` carries the ungated job's own
argument — "Gating them would mean listing BOTH `.claude/tools/**` … and `scripts/**` … in a filter" —
which a fifth wire does not touch. (The report emits `:131` and `:132` as two of its rows; the citation
is the quoted *sentence's* span, which runs on into `:133`, and it read `:131-132` until round 5 checked
the quotation against the lines. `:133` is not itself a report row — it speaks about a filter, not about
the wires.) `CLAUDE.md:52` lists `trip-wires` among `mise run ci`'s dependencies,
which stays true; and the two driver internals are

```
$ sed -n '25p;59p' scripts/trip-wires.sh
# `shopt` here: /bin/bash on stock macOS is 3.2, and the wires already record that
# be reported as "the wires are gone".
```

— `scripts/trip-wires.sh:25`, the `bash` 3.2 constraint (no arrays, no `shopt`, because stock macOS ships
3.2), and `scripts/trip-wires.sh:59`, the **resolved-root verification** message: "the root is wrong" must
never surface as "the wires are gone". All five describe the mechanism rather than the membership, so none
is falsified by an addition to the set.

**`:59` was labelled "the both-directions retention message", and it is not that line.** The retention
rationale is at `scripts/trip-wires.sh:76-84`, the passage §1 and §6 both already cite. The label entered
in the revision that was answering round 3's finding about it:

```
$ git log --oneline -S'trip-wires.sh:59' 2656d83d -- docs/plans/2026-08-claim-gate-s1-provenance.md
dd5f22d7 docs(plan): S1 revision 3 — every change is a finding the checker produced
```

— that is, the fix rewrote the words and never re-read the line, so the same report named it again. Which
is why the listing above is re-derived from `SITES` and spot-checked with a pasted `sed` rather than
hand-corrected a second time: relabelling is not re-measuring, and only one of the two survives being run.

This is the general shape and it is worth naming, because it is why the edit set was wrong twice: a
grep proves a *paste* matches its command, never that the command was the right one. `TRANSCRIPT`
re-runs every listing pasted above and confirms each is complete for what it asks; `SITES` exists because
that is not the same as being complete. Both edited sites were reachable only by asking "what else
describes the *set*", which is a concept, not a string
(memory `feedback_semantic-sibling-selfseed-and-regate-breadth`).

---

## §5 Decomposition and sequencing

Two files, per §1:

| File | Owns |
|---|---|
| `.claude/tools/claim-provenance-trip-wire.sh` | header rationale, the self-test invocation + guard, the scan, the tier pass, the field parser, the D5 reject-list, the D4 probe, the three verb executors, verdict and exit |
| `.claude/tools/claim-provenance-selftest.sh` | the fixture repos and every control in §8, including the `clone` fixture's construction, reuse and teardown on the lifecycle §4.1.1 fixes |

**The `Projected` column is gone, and it is the same class this memo has already removed twice.** It
carried `~350` and `~200`, and neither figure was ever a projection: it was written once and inherited
by every revision since —

```
$ git log --oneline -S'~350' 2656d83d -- docs/plans/2026-08-claim-gate-s1-provenance.md
494e83a9 docs(plan): S1 rewrite — close plan-review round 1 (3 CRIT / 39 IMP)
```

— while the design of the two files it projected changed underneath it three times: revision 4 deleted
the fence arm *and* the second scan stage from the wire, revision 4 added a `--one` entry point, and
revision 5 withdrew `--one` and gave the self-test a `git clone --local` fixture to construct, share and
tear down. A number that survives three changes to its own subject is not tracking that subject. Per
§4.5's runtime table and §5's "five uses" — the two the memo already withdrew for it — a figure whose
value depends on the author's convention rather than on what a command reports is **removed, not
re-measured** (memory `feedback_convention-dependent-figures-are-argument`). What replaces it is §1's
authoring trigger, which is re-runnable and applies while the files are being written rather than before.

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
3. §4.6's set-label widening, one commit — at the **four** files. `CONTRIBUTING.md:19` is not among
   them: umbrella §6 routes that whole sentence to S2 (§0, §4.6). Revision 6 put it here and that is
   withdrawn. The cost is real and is booked as a bound in §6 rather than paid by reversing the
   umbrella — for one PR, the contributor-facing document is the last governing file still calling the
   set "layering trip-wires".
4. §4.6's two set-level *claim* edits — `ci.yml:152`'s "grep-only" and `mise.toml:102-103`'s four-wire
   enumeration — one commit. Revision 3 put both in §0's scope and gave neither a commit, so an
   implementer following §5 literally would have shipped S1 with `ci.yml:152` still false.

Gate: `claim-gate-plan-check.py` (both memos, and `--self-test`) → `cargo fmt --all` → `mise run ci` →
`/simplify` → `/code-review` → `/review` → `/elidex-review`. The checker runs **first** because every
stage after it reads the memo, and a stale figure in the memo costs a reviewer's round rather than a
second. `mise run ci` is not cargo-only — `mise.toml:116` lists `trip-wires` among its `depends`, so
from this PR onward the local gate exercises the deliverable. `/review` needs a PR, so the order is
push → draft PR → `/review <n>`.

---

## §6 What S1 does not close

- **`CONTRIBUTING.md` keeps the old set label for one PR.** Umbrella §6 gives that whole sentence to
  S2 because it also asserts the allowlist mechanism S2 ships (§0, §4.6), so after S1's commit 3 the
  contributor-facing document is the last governing file still calling the set "layering trip-wires".
  The forced S1 → S2 order bounds the window at one PR. Stated as the cost of not reversing a binding
  hand-off from inside the slice — which is what revision 6 did before this was withdrawn.
- **Annotation deletion is unrecorded.** D8's ledger is S2's. Until it lands, deleting an annotation
  turns the wire green with no trace — the theory `scripts/trip-wires.sh:76-84` explicitly rejects for
  `REQUIRED_WIRES`. S1 ships no partial ledger; the fix is S2, in the forced order.
- **Check 3 rejects the strongest artifact until S3 lands.** `grep_pass.py:76-80`'s
  `VERIFICATION_ARTIFACT_RE` accepts exactly three spellings — a `grep -[rlnEFhio]+` literal,
  `verified YYYY-MM-DD`, and a `lines N/N/N` list — and **not** a `claim` annotation. So between S1 and
  S3 there are four accepted provenance forms, of which the only *executed* one is the one the tool
  rejects; this memo passes Check 3 by pasting `git grep -n…` literals, not by carrying annotations.
  D9 retires the other three and admits the annotation; **S3 closes this window.**
- **No third fence tracker — the whole question is gone** (§7 Q3, §4.2). Revision 2 accepted one on a
  premise that was false: that the two in-repo trackers already behave differently on a mismatched
  marker, sourced from `grep_pass.py:254`'s `— content` comment, which means content *of the still-open
  block*. `claim-gate-plan-check.py`'s `FENCE` check now measures that relation by running both, and
  **forbids this memo from restating the verdict in either polarity** — a boolean the memo's own edits
  can invalidate is what D10 form 2 says to cite rather than write out, and a checker that read English
  polarity would be the same unexecuted-claim medium one level down. What remains is not a bound but a
  deletion: with (c) adopted there is no fence tracking in the wire at all.

- **The checker carries a second transcription of §4.2 and §4.4 until the wire lands** (§0). Not a
  bootstrap that retires itself — revision 3 asserted that and it was false — but a stated debt whose
  trigger is S1's own landing and whose owner is the prerequisite PR. The closing condition is a control
  that plants a wire and asserts the checker's grammar path is unreachable, because the untested
  retirement path is precisely what let the false claim stand.
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
- **Untracked files are invisible; tracked ones are read from the working tree.** Revision 3 said
  `git grep` "reads the index, so an annotation is inert until staged". Only the first half is right,
  and the second half is backwards. Measured:

  ```
  git init -q .; printf 'alpha\n' > a.txt; git add -A; git commit -qm x
  printf 'alpha\nalpha\n' > a.txt          # worktree 2, index 1
  git grep -c alpha -- a.txt          → a.txt:2      # working tree
  git grep --cached -c alpha -- a.txt → a.txt:1      # index
  ```

  So the scope is **tracked files, at their working-tree content**. `--untracked` adds untracked files;
  it is not what makes a modified tracked file visible. Three consequences the old wording got wrong:
  an edited annotation is live without staging; the gate does not govern "the committed tree"; and the
  preamble's base is a working tree with this file in it, not an index (D10 form 3 unaffected in
  substance — the base is still declared and still includes this file — but stated correctly).
  A fourth is new and is a real asymmetry, recorded here rather than fixed: the D4 liveness probe reads
  the **index** (`git ls-files`) while all three verbs read the **working tree**, so a file deleted in
  the worktree but still in the index passes liveness and then measures nothing.
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

**Q3 — CLOSED by round 3: the wire has no fence tracker.** The question was whether the wire should
carry a third implementation of fence tracking (a), shell out to the Python one (b), or drop the fence
arm entirely (c). Round 3 measured the quantity the recommendation for (a) had assumed instead of
measured — how many annotations actually rely on the fence arm — and the answer is zero (§4.2). With
(c)'s only stated cost at zero and (a) costing a third copy of an algorithm, *One issue, one way*
decides it. **(c) is adopted**; §4.2 now has a single inertness rule and no per-surface table.

(b) is rejected on a ground worth recording, because C26 was violating it: it puts a Python interpreter
in the path of a wire whose entire justification for running ungated is that it needs no toolchain
(`.github/workflows/ci.yml:128-130`), and couples a repository gate to a skill directory's internals.
That rejection now holds without exception — §8.1's C26, which ran the memo-checker inside the wire's
self-test and did both of those things harder than (b) would, is deleted (§8.1).

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

**What a Fixture value means, because `UNIQ`'s whole verdict rests on it.** Round 3 found the column
being read as load-bearing while nothing wrote its meaning, and the shape §8 cites for the self-test —
`scripts/trip-wires.sh:113-184` — does the opposite: **one** `mktemp -d`, mutated between probes. Under
that shape all ten bare `asserted=` rows share an invocation and collide. So the model is stated:

| Fixture | Means |
|---|---|
| `own` | **its own `mktemp -d` and its own wire invocation**, torn down after. Two `own` rows cannot mask each other by construction, whatever their `$3`. |
| `escapes` | one fixture carrying both C8's and C9's annotations, so their diagnostics must discriminate — this is the pair R1's finding was actually about. Both arms are malformed, so the row exists at all only under §4.2's fail-without-ending rule: a wire that stopped at the first malformed annotation would never reach C9's. |
| `I7` | C27's counting-shim fixture; one invocation, instrumented. |
| `clone` | a `git clone --local` of this repository, built **once per self-test run** and reused by every `clone` row, but with **one wire invocation per row** — the annotation is planted in a tracked file of the clone and removed after, so the scan acquires it (§4.1.1(b)). The sharing is cost, not scope: by the model above, separate invocations cannot mask each other, so `clone` rows carry no `$3` requirement beyond every other row's. |

So the property is scoped to the invocation, and the table below carries the **Fixture** column that
makes it decidable:

> Within one fixture, no negative arm's `$3` is a substring of another's — and each is specific enough
> to separate the failure the control intends from every *other* failure that fixture can produce. A
> wrong-count failure prints `asserted=<N>`; a dead pathspec prints `matches no tracked file`; a
> rejected escape names the escape. Those are already disjoint kinds, which is why bare `asserted=` is
> adequate in a one-annotation fixture and inadequate the moment two live in one.

`claim-gate-plan-check.py`'s `UNIQ` check enforces the scoped form, grouped by the Fixture column, and its
`FIXTURE` check verifies the column against each row's own `in=` pathspec — a label a reader must trust is
not a specification. `escapes` is now the only group the `$3` requirement bites in: revision 5 read the
`clone` group as a second such group and round 5 found that reading strands C10's and C11's positive arms,
so §4.1.1(b) fixes the clone as one invocation per row and the requirement falls away with it. `UNIQ`'s
grouping is unaffected — a group of one-row invocations has nothing to collide.

### §8.1 Controls

| # | §4 clause under test | Fixture | Positive (must pass) | Negative (must fail; `$3`) |
|---|---|---|---|---|
| C0 | §6 "the wire does not judge" | own | fixture with `console.log` and `the sole caller`; boundaried pattern, true count | bare `sole` pattern carrying the boundaried count (`asserted=`) |
| C1 | §4.4 "not anchored to either end" | own | annotation inside a GFM table cell, true count → executes | same cell, wrong count (`asserted=`) |
| C4 | §4.2 backtick arm, odd *k* | own | backticked annotation, deliberately wrong count → ignored | same line with the backticks removed (`asserted=`) |
| C5 | §4.2 the backtick arm on `.sh` | own | backticked annotation in a `#` comment in a `.sh` → ignored | same `#` comment unbackticked, wrong count (`asserted=`) |
| C6 | §4.2 malformed fall-through | own | — | `<!-- claim cont=3 pattern="x" in="a.md" -->` (`MALFORMED`) |
| C7 | §4.4 `lines-verb` takes no `pattern` | own | `lines=` alone, true length | `lines=5 pattern="fn" in="a.md"` (`MALFORMED`) |
| C8 | §4.3 reject `\d` | escapes | `[0-9]` accepted, true count | pattern containing `\d` (`contains \d`) |
| C9 | §4.3 reject `\b` | escapes | `([^A-Za-z]\|$)` accepted, true count | pattern containing `\b` (`contains \b`) |
| C10 | §4.4 liveness, `in=` ≥ 1 | clone | `in="crates/**/*.rs"` | `in="crates/**/*.rss"` (`matches no tracked file`) |
| C11 | §4.4 liveness, `lines=` exactly 1 | clone | one-file pathspec | `in=".claude/tools/wasm-*-trip-wire.sh"`, which resolves to 2 (`resolves to more than one`) |
| C12 | §4.4 argv fidelity (I5) | own | §4.4's three-file fixture, `files=2 … in="docs/**/*.md"` → passes only if the pathspec reached git as one argv element | `files=1` with the same pathspec (`asserted=1`) — the value a shell-globbing executor would compute |
| C13 | §4.4 `-e` is not optional | own | `pattern="-> u32"`, true count | same pattern, wrong count (`asserted=`) — without `-e` the run dies on `unknown switch` and neither direction is reachable |
| C14 | §4.4 `count` = matching lines | own | fixture line reading `sole sole`, `count=1` | `count=2` on the same fixture (`asserted=2`) |
| C15 | §4.4 `count=0` must not abort | own | `count=0` on a pattern that matches nothing → passes, wire continues | `count=1` on the same (`asserted=1`) |
| C16 | §4.1 D2 self-exclusion, **both** directions | own | wrong-count annotation under `.claude/tools/` → not scanned, stays green | the byte-identical annotation one directory up (`asserted=`) |
| C17 | §6 self-matching is permitted | own | annotation whose pattern matches its own line, count **including** that line | same annotation, count excluding it (`asserted=`) |
| C18 | §1 the wire invokes its self-test | own | wire + helper copied into a fixture dir → runs | wire copied **without** the helper (`self-test helper`) |
| C19 | acceptance corpus — v1 `count` (umbrella §0.1) | clone | — | `count=299 pattern="#\[test\]" in="crates/shell/elidex-shell/src/**/*.rs"` (`asserted=299`) — the pathspec says `src/` where 299 was the whole crate |
| C20 | acceptance corpus — v1 `files` | clone | — | `files=7 pattern="REQUIRED_WIRES" in="scripts/*.sh"` (`asserted=7`) |
| C21 | acceptance corpus — v1 `lines` | clone | — | `lines=1219 in="docs/plans/2026-07-app-mode-turn-completion-drain.md"` (`asserted=1219`) — 1219 was a line *number* |
| C22 | acceptance corpus — #504 §5, the row umbrella §3 row 4 routes here | clone | — | `lines=632 in="crates/shell/elidex-shell/src/app/drain_host/mod.rs"` (`asserted=632`) |
| C23 | §4.4 the optional `icase` keyword | own | fixture carrying `SOLE` and `sole`; `count=2 icase pattern="sole"` | the same annotation **without** `icase`, still asserting 2 (`asserted=2 `) — proves the keyword is read, not ignored |
| C24 | §4.2 the backtick arm on `.toml` | own | backticked annotation in a `#` comment in a `.toml` → ignored | same `#` comment unbackticked, wrong count (`asserted=`) |
| C25 | §4.3 the reject set is **all ten** of `bBdDsSwW<>`, read under §4.2's fail-without-ending rule | own | — | ten annotations, one per escape, each asserted MALFORMED with its own escape named (`contains \`, ten times, each call passing the specific escape) |
| C27 | §4.5 I7's structural half | I7 | a fixture with 3 annotations and a counting `git` shim first on `PATH`: exactly one tree-wide `git grep`, and a per-annotation term that scales with 3, never with the file count | the same fixture at 6 annotations, asserting the 3-annotation invocation count (`git invocations`) |

C23–C25 close three clause-shaped holes round 2 named: the `icase` keyword, the `.toml` surface (C5's
row claimed both `.sh` and `.toml` and its fixture is a `.sh`), and the reject set, of which two of ten
characters were controlled. C25 is one row rather than ten because the ten differ only in the character
fed to the same clause — a loop over the set, asserting the diagnostic names *the escape it was given*,
which is the property §4.3 states and which ten hand-written rows would restate ten times.

**C27 is round 3's, and it closes the last invariant with no control.** I7 was the only one of the seven
enforced by §4.5's sentence *"That is visible in the code shape"* — which is verbatim the reasoning
§4.4 records as the first draft's error about I5 (*"The first draft called I5 unprobeable ('enforced by
code shape and by review'). It is not"*) and then refutes with a hermetic fixture. The lesson was
applied to I5 and not to its sibling. The wall-clock half of I7 stays unbounded and §4.5 stays right to
withdraw the runtime table; the **structural** half is probeable in exactly C12's shape.

**Deleted in revision 4**: C2 and C3 (the `.md` and `.rs` fence arms — §4.2 has no fence arm), and C26,
which ran `claim-gate-plan-check.py --self-test` inside the wire's self-test. C26 was two defects at
once: it put a Python interpreter and a dependency on `.claude/skills/elidex-plan-review/` internals
into the wire's path, which is the ground §7 Q3 rejects option (b) on and which §4.6 keeps
`ci.yml:152`'s "No toolchain step" true against; and its negative arm named no input, asserting a `$3`
of `probes ok` that the checker prints on success. A control that cannot red is §2 I4×I6's `ban_control`.

C4, C5, C16 and C24 all use the **wrong-count-inert / right-shape-live** pairing rather than a merely-ignored
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
> illustrative annotation in either memo is **backticked** and classifies as *mention*; and at least
> one strict annotation exists for each of the three verbs.

Revisions 1–3 wrote that middle clause as "fenced or backticked", which (c) makes false in the direction
that matters: a fenced annotation is now **strict and executed** (§4.2). The property is also why (c)
costs nothing today rather than merely little — it is satisfied by the tree as it stands, with no
annotation to rewrite.

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

The soft count is 7 rather than revision 2's 5 because revision 3 added two `crates/…**/…` pathspecs,
**both of them in §2's I6×I7 row** — the two tree-scale controls that row exempts from I7, named there in
one sentence. The attribution stood as "§2's I6×I7 row and §4.5's exemption" until round 5 read §4.5:
that paragraph names C10 and C19 in prose and carries no pathspec literal at all, so it contributes
nothing to this tally. (Neither literal is repeated here. Pasting one would add a soft finding to the
count this block reports — the self-invalidation §8.2's per-line listing declines for the same reason.)
The sentence also said "this revision", inherited unchanged from the revision where it was true.
**This block is a
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
the whole `CONTRIBUTING.md:19` sentence → S2 (umbrella §6, §0, §4.6); D7/D9 and Check 3, including §6's
four-form window → S3; `#11-claim-annotation-diff-scoped-lint` → booked by umbrella §5, its
`fetch-depth: 0` trigger now watched by §5's `count=0` annotation.

Revision 2's list ended that S3 item with "and the fence-tracker reconciliation", which was a defer to
a slice that does not accept it: umbrella §6's S3 line reads *"D7's shared word lists and the
cross-implementation control"* and names no fence work. So the routing was a claim about another
document that the other document does not make — the same class as the rest of round 2's findings, in
the one section whose job is to be exact about who owns what. It is withdrawn rather than re-routed,
because there is nothing left to route: the premise that there was a reconciliation to do was §6's
false one, and §7 Q3 is now closed by adopting (c) — there is no third tracker to reconcile.

**Two findings this memo made about other documents, routed rather than left.** Round 3's point is that
§9's job is to be exact about who owns what, and that a memo which *proves* a landed document false
while booking zero deferrals has hidden one in prose:

- **#504's `Lines at HEAD` table is false in five of ten rows** (§8.2), all undercounts, all still
  false on `main`. C22 takes one as corpus; the other four are not corpus material and are not S1's to
  edit — `docs/plans/2026-07-app-mode-turn-completion-drain.md` belongs to the **L4 lane**. Routed
  there with the measurement attached, and with the trigger that makes it non-optional: S1 is opt-in
  (§6), so the wire will never red on them. They have to be corrected by hand, once.
- **Where the plan-memo checkers run is not S1's decision to make, and S1 was about to make it a third
  time.** Revision 4 routed the question ("`claim-gate-plan-check.py` is registered in no gate") to the
  prerequisite PR. Round 4 measured what that owner is: a branch with no PR, no plan-memo and no ledger
  row, whose diff registers nothing — so the obligation was routed nowhere, which is the shape this very
  section names one paragraph earlier.

  It is worse than an empty owner. Two decision surfaces for *the same question* already exist:
  the prerequisite PR's docstring ("collapse them when a **third** memo wants one"), and
  `.claude/skills/elidex-plan-review/SKILL.md`'s deferred maintenance note, which books "put the
  plan-memo consistency checkers on a loop" as **one PR** touching `scripts/trip-wires.sh`,
  `mise.toml` and `.github/workflows/ci.yml` — three of S1's five edit files — and commits to
  generalising `plan-xcheck.py` as part of it. The two answers are opposite.

  `SKILL.md` is the canonical site: it is the skill's own authoring home, it already carries the note,
  and it already carries a re-eval date. **S1 books nothing and decides nothing here.** The one fact S1
  contributes is that a third checker now exists and belongs in that note. The complication, stated
  because it is load-bearing: that note is **not on `main`** — it lives only in `layout-decorated-inline`
  (`git grep -c 'plan-xcheck' origin/main -- .claude/skills/elidex-plan-review/SKILL.md` → no hits;
  the note is at `0d4d474c`). So the collapse cannot be executed from here. The rule that can be:
  **whichever of the two branches lands first adds `claim-gate-plan-check.py` to that note**, and the
  prerequisite PR's docstring withdraws its competing sentence rather than restating it.
  (memory `feedback_duplicated-decision-surface-blocks-converge`.)

**Own deferrals: 0 of the per-PR ≤3 budget** — unchanged. Revision 2 reached zero partly by routing the
fence question to S3; with that withdrawn and Q3 closed, the count stands on its own.
