# VM P4 umbrella — §5's 1000-line touch-time check (§5 (tail), carved out)

**Carve-out of** `docs/plans/2026-07-vm-p4-es-language-completeness.md` — the VM P4 ES-language-completeness umbrella — whose
**§5 (tail)** this file *is*. Moved **verbatim** at `39cc365a` by the umbrella's second touch-time
prereq split; the seam derivation, the ranges and the content-free proof are in `docs/plans/2026-10-vm-p4-memo-seam-split.md`
(not linked, so it stays outside the checker's population). Not a rewrite: every figure, line
anchor and revision reference below is stated about the umbrella at `39cc365a`.

**Reading this file.** Section numbers are the umbrella's; an unqualified section number names
that section of the family, located by the entry's carved-out stubs. `<this file>` in a command
means `git show 39cc365a:docs/plans/2026-07-vm-p4-es-language-completeness.md`. "above" / "below" read in family section order.
**Tense.** "It is discharged here" below is the first carve's statement, written in the
umbrella; this file's own carve is discharged by the split PR that the plan above records.

**1000-line touch-time check — the standalone prereq split is withdrawn (2026-07-27), on a narrower
ground than the first statement of this section claimed.** Rounds 2-9 all carried a mandated
"standalone prereq split branch" for `vm/dispatch.rs`.

⚠ **The 2026-07-27 reversal shipped after the [§15](2026-07-vm-p4-umbrella-review-rounds.md) convergence call, ungated, and its measurements did
not survive re-derivation** — caught by this document's own plan-review at PR-B (`#506`). Its
originals were 1036/1113 (93%), 68 arms ≤8, 13 arms >20, a `run()`-residue enumeration that omitted
two top-level items, and — the figure the argument turned on — "29 arms using inline loop control
flow (16 `continue`, 13 `return`)", which summed two occurrence counts as though the sets were
disjoint.

**Every arm-classification figure has been removed rather than corrected.** Two independent passes
agreed exactly on every number taken straight off `wc -l` or `grep -c`, and disagreed on every number
requiring an arm-boundary convention (arm span, size buckets, control-flow arms) — the convention is
a choice, so those figures were argument dressed as measurement. What remains is re-derivable in one
command each:

| Measure | `f7d9b5ce` (rounds 2-9 basis) | `658cc302` (0a landed) |
|---|---|---|
| `wc -l vm/dispatch.rs` | 1112 | **1103** |
| `wc -l vm/dispatch_{class,helpers,ic,iter,objects}.rs` | 1725 across 5 files | **1888** |

*(An arm **count** is deliberately absent. `grep -c '^ *Op::.*=>'` returns 113 / 117 — it counts
pattern lines, and multi-pattern arms like `Op::IncElem | Op::DecElem =>` are one arm. Any figure
that reconciles the two encodes an arm-boundary convention, which is the class of number this
section got wrong. The dispatch match is order-100 arms; nothing below needs it sharper.)*

Two conclusions, and they are **not** the same conclusion:

1. **The match is not split.** CLAUDE.md's discipline is cohesion judgment, not line-count mechanics,
   and names the exemption: *「一枚岩の cohesive unit・巨大 generated table・**flat な case table** は
   対象外」*. A ~100-arm opcode dispatch table is that case, and that clause is the whole of the
   argument. *(The reversal also argued the match "cannot be split without a behavioural rewrite"
   because N arms `continue`/`return` the loop and read loop-local state. Removed: Slice 0a itself
   relocated `Op::IncElem | Op::DecElem` — an arm carrying two inline `continue`s — into
   `dispatch_helpers.rs::op_inc_dec_elem`, by returning `Result` and calling `throw_error` at the call
   site. No control-flow enum, no semantic rewrite. The premise was false.)*

2. **The file is NOT exempt from reduction, and the earlier text claimed it was.** "The match cannot
   be split without a behavioural rewrite" was read as covering the file; it does not follow, and per
   conclusion 1 it was not even true of the match. The `dispatch_*.rs` family is the in-tree seam that
   absorbs arm bodies, and Slice 0a moved the numbers in the right direction while implementing an
   unrelated feature: `dispatch_helpers.rs` 237→391, `dispatch_objects.rs` 429→438, `dispatch.rs`
   1112→1103.

**Decision: no standalone prereq-split PR — because the debt is discharged continuously, not because
the file cannot be reduced.** The forward rule for every slice that touches this file: extract the
*arm body* into the existing `dispatch_*.rs` family, never grow an arm in place — **choosing the
destination by I-5's predicate (the `engine` feature gate + host-binding dependency), not by the
file-family name.** Measured at `658cc302`: `dispatch_{class,helpers,ic}.rs` have zero
`feature = "engine"` occurrences; `dispatch_iter.rs` has 3 (`:114`, `:127`, `:153`) and
`dispatch_objects.rs` 2. ⚠ **The predicate is the *region*, not the file** — those three gates are
narrow bands **inside a 370-line file**, and everything outside them, including `iter_close` at
`:354` (which [§6.2a-2](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md) calls the canonical §7.4.11 implementation), is core-only. *(370 is the
**whole** `dispatch_iter.rs`, not a remainder: `git show <rev>:…/vm/dispatch_iter.rs | wc -l` gives
370 at `f7d9b5ce`, `658cc302` and HEAD alike. An earlier reading of this sentence as "the rest of
the file, 370 lines" double-counted.)* A file-level reading would forbid
the children umbrella **1a**'s derivation mints for the call-spread VM infrastructure, and Slice **Pa**, from the file both their module columns name, and Pa's whole deliverable is
re-signaturing that function. Read the predicate in **both directions**: do not place a core-only
body *inside* an engine-gated region, **and** do not extract an engine-bound arm body into a
currently core-only file by wrapping it in a fresh `#[cfg(feature = "engine")]`. That rule is the
discharge mechanism, so a slice that adds an arm and leaves the file larger has not complied.
(The withdrawn text also argued the reversal "takes a PR off the critical path and unblocks the T0 fix
immediately" — schedule is judgment-supporting information, not a design constraint (CLAUDE.md
*Ideal over pragmatic*), so it is removed rather than restated.)

**Files the watch list still owns** (`#11-d17b-dispatch-expr-file-growth`): `vm/interpreter.rs` 1366
(**umbrellas 1a and 3**, through the children each mints for the work), `vm/value.rs` 1187 (**umbrella 3**, through the children it mints for the work). **Not on any watch list and >1000**:
`vm/object_kind.rs` 1700 (**umbrellas 6, 7 and 10**, through the children each mints for the work — and see the cross-lane note in §8),
`vm/gc/collect.rs` 2074 (**umbrella 7b**, through the children it mints for the work), `vm/gc/trace.rs` 1255 (**umbrellas 7a and 7b**, through the children each mints for the work — 7a's unconditional
marking arm for the `[[MapData]]` / `[[SetData]]` variant as well as 7b's ephemeron fixpoint; ⚠ the
attribution to 7b alone was read off 7a's "No GC-tracing change", which that row now withdraws as
false — and **every instance §4 I-7 enumerates is a further reader of that same file**, since what I-7
obliges is an arm in it. The instances are **not listed here**: this note carried a copy for one
revision and the copy was wrong within that revision — it named Slice N after the obligation had moved
to N's slot, and it omitted Slice C after C's row gained the obligation. A second copy of an
enumeration drifts from the first by construction, so the list has one home and this is a pointer to
it). ⚠ **Two more, and the reason they were missing is that this list was curated rather than derived.**
The population is *every file a §5 `Primary module(s)` cell names*, and it is one command — over this
document's blob `2718d8d4`,
`git show 2718d8d4:<this file> | sed -n '/^| # | Slice | Primary/,/^$/p' | grep '^| \*\*' | perl -pe 's/\\\|/\x01/g' | cut -d'|' -f4`,
then `git show 658cc302:crates/script/elidex-js/src/<f> | wc -l` for each `.rs` token it emits: **63**
rows and **7** files over 1000. *(The **47** distinct files that stood here is withdrawn rather than recounted: the cells emit **50** distinct `.rs` tokens of which **44** resolve under `crates/script/elidex-js/src` at `658cc302`, and no tokenisation of the command's output returns 47 — a convention-dependent figure, the class this section removes.)* Five are already named above; the two that were not are
**`vm/well_known.rs` 1588** (Slice **8d**) and **`vm/host/structured_clone.rs` 1077** (Slice **Pc**).
Pc's own cell now discharges the second as a mandatory prereq split; 8d's plan-review owes the first a
verdict. *(Anchored to the blob because the input is this document: the same revision removes
`structured_clone.rs` from **1a**'s module cell and leaves it in **Pc**'s, so the set of 7 is unchanged
by it — re-run the command rather than reading the figure forward.)* Each needs a cohesion verdict in its own
slice's memo — and this reversal is the precedent: **measure the file's shape before assuming a
split**.

**And the verdict this check has never stated about the document it is written in — stated, and
discharged in the commit carrying this paragraph.** This memo was **3714** lines at `f754c873` and
**3857** at `3526b722`, the split's parent — `git show f754c873:<this file> \| wc -l` and
`git show 3526b722:<this file> \| wc -l`, anchored to blobs because the split moves the answer — and
CLAUDE.md's *1000-line debt = touch-time split* discipline is a cohesion
judgment, not a `crates/**` scope: the sibling citation-hygiene lane recorded exactly this verdict
when its own umbrella crossed **1007** lines ([[project_citation-hygiene-program]]). ⚠ **From the
split on, `<this file>` in every command in this document means the umbrella alone**: the three
carve-outs below are separate files, and no command written here reaches into them. This document
reaches for the rule **ten** times (`grep -c '1000-line' <this file>` → **10**, re-derived at the
split, which moved it **12**→**10** by carrying two mentions into the round-disposition carve-out;
⚠ the same grep over the umbrella **and** its three carve-outs returns **15**, not 12 — the two that
moved plus **three** new ones, one per carve-out head naming the discipline that produced it, which is
the figure this very sentence would have carried stale had it been reasoned instead of run), and eight of the ten
name a `.rs` path or a per-file scope; the two that do not are the sentence above and this one, i.e. this passage, which is the first place in the document that does name this file. That is the omission shape this
section has now named twice — a rule swept over the population that motivated it rather than over
the population it reaches.

**Verdict: the seams are real, and the split landed here — a standalone, content-preserving prereq
commit, not a deferral.** The disposition this passage carried until then owed the carve to "the next
PR making a substantive edit to this document"; that was **wrong**, and its own evidence says so.
**This PR is that touch**: it *creates* the file — `git cat-file -e origin/main:<this file>` fails and
`2b4789eb` is the adding commit — so introducing a 3857-line memo *is* the substantive touch the rule
fires on, and there is no later PR for the obligation to be owed to. The cost the deferral argument
named is paid rather than avoided: every finding, round number and line anchor in the carved sections
is stated *about the pre-split document*, so each carve-out's head records that and **not one anchor
was re-derived by the move** — the carve relocates lines and rewrites pointers, and changes nothing
else. The three seams, boundaries by `awk '/^## §/{print NR}'` and sizes by `sed -n` plus `wc -l`,
first taken over the blob `git show f754c873:<this file>` and **re-measured over `3526b722`** before
carving:

- **§§16-18 — Slice 0a's landing record and its two review gates, 388 lines** (3327-3714 at
  `f754c873`; **3470-3857** at `3526b722`) → `docs/plans/2026-07-vm-p4-slice-0a-landing-record.md`. A record of
  a *landed* PR is a different document kind from a forward plan, and nothing in it is scheduled: §5's
  three specification fields are read from none of it.
- **§§10-15 — six round dispositions and the convergence call, 271 lines** (3056-3326 at `f754c873`;
  **3199-3469** at `3526b722`) → `docs/plans/2026-07-vm-p4-umbrella-review-rounds.md`. An append-only
  review archive, and its own headings run out of order ([§13](2026-07-vm-p4-umbrella-review-rounds.md) precedes [§12](2026-07-vm-p4-umbrella-review-rounds.md)), which is what append-only
  looks like rather than what a designed section looks like.
- **[§6](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md) — Slice 1a + 1b detail, 492 lines** (1945-2436 at `f754c873`; **2030-2521** at `3526b722`) →
  `docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md`. The *asymmetry* is the seam, not the size: §5
  states that deriving a slice's design belongs to that slice's own mandatory plan-review, and [§6](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md) is
  one slice pair's design memo living inside the umbrella. No other slice has one here.

The plan does cite the record — [§13](2026-07-vm-p4-umbrella-review-rounds.md)'s enumerate-by-command rule, [§16](2026-07-vm-p4-slice-0a-landing-record.md)'s Slice 0a record, [§17](2026-07-vm-p4-slice-0a-landing-record.md)/[§18](2026-07-vm-p4-slice-0a-landing-record.md)'s
ratification of decision 5 — so the carve is a **pointer rewrite**, not a clean cut, and that is
part of what it cost. **The rewrite is done, and its extent is now measurable without the convention
this passage refused to trust.** A bare `§NN` count still cannot separate this document's own section
numbers from ECMA-262 clause numbers of the same spelling — so the population was taken
**structurally** and narrowed by a **derived** section set plus a **named** exclusion list, never by a
vocabulary of section names. Structurally: every `§<n>` token in the pre-split file
(`grep -oE '§[0-9][0-9]*(\.[0-9]+)*[a-zA-Z]*(-[0-9]+)?' <this file> \| wc -l` → **2392** at
`3526b722`), narrowed to the tokens this document uses as **its own** headings
(`grep -oE '^#{2,3} §[0-9][0-9.a-zA-Z-]*' <this file>`, whose carved subset is `§6`, `§6.1`, `§6.2`,
`§6.2a`, `§6.2a-2`, `§6.2a-3`, `§6.3`, `§6.4`, `§6.5`, `§10`-`§18`, `§18.1` and `§18.2`), then minus
the bodies being carved out, leaves **87** occurrences. Named exclusion list: **seven** of the 87 are
spec clauses of colliding spelling, named rather than pattern-matched away — WHATWG URL `§6.2` (four
occurrences), W3C IndexedDB `§6.1`-`§6.6` (one), WHATWG URL `§6` (one) and ECMA-262 `§14` (one). Line
anchors for them are deliberately **not** carried, because this very commit moves line numbers; the
residual command below is what locates them. The other **80** are references to a carved section, and
each is now a markdown link naming its carve-out, so the extent is re-derived **from the artifact
rather than from the convention**:
`grep -oE '\]\(2026-07-vm-p4-(slice-1a-1b-call-spread-detail|umbrella-review-rounds|slice-0a-landing-record)\.md\)' <this file> \| wc -l`
→ **84** in the umbrella as it now stands — **80** until the umbrella-naming sweep of this revision,
which added four: three in the `edge NN` namespace declaration at §6's carve-out stub and one in the
evidence-anchor rule, each a link that did not exist before because the sentence carrying it did not.
⚠ *A figure written "as it now stands" is invalidated by the next edit to the file it counts, and it
was — by an edit in this same PR, four commits after the sentence was written. The re-run is the
claim; the number beside it is only the last time anyone ran it.* *(The rewrite of this passage was
separately net zero, dropping eight such references and reintroducing eight — measured, not
assumed.)* The **residual** — a
carved heading token in running prose that is neither a link nor a backticked spelling under
discussion — is the check that does not depend on that arithmetic, and it returns the seven named
collisions and nothing else:
`python3 -c "import re,sys;B=chr(96);L=open(sys.argv[1]).read().split(chr(10));t=re.compile('§[0-9][0-9]*(\.[0-9]+)*[a-zA-Z]*(-[0-9]+)?');H={'§6','§6.1','§6.2','§6.2a','§6.2a-2','§6.2a-3','§6.3','§6.4','§6.5','§10','§11','§12','§13','§14','§15','§16','§17','§18','§18.1','§18.2'};print(sum(1 for l in L if not l.startswith(chr(35)) for m in t.finditer(l) if m.group(0) in H and l[:m.start()].count(B)%2==0 and l[m.start()-1:m.start()]!='['))" <this file>` → **7**.

**What is *not* a seam: §5 itself, 1020 lines and over the threshold alone.** Its conventions
passage and its table are one unit — the conventions define what each cell of the table means and
the table is the only reader of them — so CLAUDE.md's *「一枚岩の cohesive unit」* exemption is why it
stays whole. That is a reason about §5's shape and **not** a reason the file stays whole, which is
the same two-conclusions-are-not-one-conclusion the `vm/dispatch.rs` reversal above turns on.
*(**1020** is the `f754c873` figure; at `3526b722` §5 spans **954-2029** = **1076** lines. The
exemption is a judgment about shape, which neither figure moves.)*

**Owner and discharge, and why this mints no slot.** The obligation is CLAUDE.md's touch-time one —
*「触った時点で bounded 化する」* — which is discharged by the PR that touches the file, not scheduled
from a ledger, so it is not a deferral and §8's Forward rule (which is stated over *surfaces this
program does not implement*) does not reach it. **It is discharged here**: the commit carrying this
paragraph carves the three seams and does nothing else, ahead of the rest of this PR's edits, exactly
as §5 requires of a `crates/**` file. The earlier reading — that the converge rounds "only append to
§§10-18" and so are not that PR — mistook the *appends* for the touch; the touch is the introduction
of the file, which is this PR.

Globals this program does not implement are registered in §8's table, like every other deferral.
`Intl`, `Atomics` and `WeakRef`/`FinalizationRegistry` moved there; this paragraph used to carry
their dispositions in prose, inside a section about `vm/dispatch.rs` line counts, which is not a
place a deferral can be retired from.

---

