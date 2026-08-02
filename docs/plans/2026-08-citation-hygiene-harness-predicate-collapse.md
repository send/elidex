# Citation-hygiene harness — design note: the predicate collapse

**Subject**: the re-derivation harness on branch `citation-hygiene-harness`
([PR #505](https://github.com/send/elidex/pull/505)) — its validity predicate, its block set, and its own
verification.

**This note is not a description of the diff.** `/elidex-review` on #505 returned 2 CRIT / 31 IMP / 17 MIN
(carried from that review, not re-derived here), and the decision taken was **not** to patch them one by one.
So this note answers exactly three questions and stops.

⚠ **Draft 2.** Draft 1 was rewritten after `/elidex-plan-review` returned **4 CRIT**. Three roots, all of which
changed conclusions rather than wording: (R1) draft 1 hand-carried claims about the harness instead of
deriving them — eleven of twenty findings were that one defect, the same one the program exists to fight;
(R2) draft 1's recommendation was chosen over the alternative on a blocker that **does not exist**, and is
reversed here; (R3) draft 1 read block routing off prose rather than off the part files, which mis-routed four
blocks and authorised deleting one that A-i §13.1 explicitly orders not to silence. Draft 1's reasoning is in
`git log`, not restated here.

## §0.5 / §3. Spec coverage map

**No spec surface.** This note settles a design question about a shell harness under `docs/plans/`; it touches
no spec-defined behaviour and cites no spec.

⚠ Under the **pre-A-ii** gate this memo hard-fails (heading, no table) — A-ii's §4.2.5 marker is what makes
"no spec surface" a declarable state. Same position A-iii's memo is in, and deliberate for the same reason.
Verified: `python3 .claude/skills/elidex-plan-review/preflight.py <this file>` →
`HARD FAIL — Spec coverage map heading at line 13 but no markdown table follows it`, exit 1.

## §1 The measurements this note reasons from

**Every quantity below is a command. Re-run before citing; do not carry a digit forward.** `M1`/`M2`/`M7` run
in `elidex-wt-citeaudit` (branch `webref-cite-audit-tool` — where the memos are); `M3`–`M6` in
`elidex-wt-harness`. Readings below were taken at `12281e3b`; the only commit since is this note, which no
block reads, and M3–M6 reproduce identically at `b55be6d9`.

⚠ **The commands are given in fenced blocks, not table cells.** Draft 1 put two of them in cells, where the
`|` inside an ERE has to be written `\|` — and `grep -rlE 'a\|b'` matches neither. One of the two then
returned *nothing*, which reads as the opposite of the row it was cited to support. A command that must be
un-escaped before it runs is not a re-runnable command.

```bash
# M1 — who cites the harness at all
for m in Ai-spec-label-map Aii-gate-failure-semantics Aiii-suite-scheduler \
         B-detector-correctness C-policy-retirement umbrella; do
  printf '%-28s ' "$m"; grep -c 'rederive' "docs/plans/2026-07-citation-hygiene-$m.md"; done
# M2 — each memo's DECLARED block list (§15 is authoritative for A-i/A-ii/A-iii)
sed -n '/^## §15/,/^## /p' docs/plans/2026-07-citation-hygiene-A{i,ii,iii}-*.md
# M2b — the umbrella is NOT a §15 memo; it declares by invocation, in two forms
grep -noE 'rederive (suites|budget|lanes)|A-rederive\.sh (suites|budget|lanes)' \
     docs/plans/2026-07-citation-hygiene-umbrella.md
# M3 — which blocks are RED on the branch that ships them  (2>&1 required: the
#      failure diagnostics go to stderr, so a stdout-only capture reads clean)
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh all 2>&1 | tee /tmp/m3.txt
# M4 — are those REDs verdicts, or measurement failures?
grep -c 'MEASUREMENT FAILED' /tmp/m3.txt; grep -c '!FAILED(rc=' /tmp/m3.txt
# M5 — what runs the harness outside docs/plans/
git grep -lE 'A-rederive' -- . ':!docs/plans/'
grep -rlE -e rederive -e citation-hygiene mise.toml .github/ scripts/
# M6 — the harness's own view of its size
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh selfcheck
```

Readings:

- **M1** — A-i 27, A-ii 19, A-iii 10, umbrella 4, **B 0, C 0**. The carve's "serves four slices" premise is
  false at the two slices meant to be downstream of it.
- **M2 / M2b** — A-i: `citations keysets readers regions couplings budget` (+`lanes`, §13). A-ii: `citations
  column carvecolumn instruments remedies reloadstale armmatrix budget couplings marker lanes`. A-iii: `suites
  filters suiteset ruleset budget couplings lanes`. Umbrella: `suites` (:82), `budget` (:124), `lanes` (:141).
  ⚠ The umbrella's third citation uses the **full-path** invocation form; a `rederive <name>` regex alone
  misses it. Draft 1 missed `budget` here and assigned it to A-i alone.
- **M3** — `FAILED BLOCKS: partition(exit 1) keysets(exit 1) regions(exit 2) offline(exit 1) couplings(exit 1)
  bmemo(exit 1)`; run exit 1. **6 of 22.**
- **M4** — **0** and **0**. None is a measurement failure; each is a block correctly reporting that what it
  measures is absent.
- **M5** — no match, both. **0 tests, 0 CI, no `mise` task**; nothing outside `docs/plans/` names the harness.
- **M6** — `7 harness parts, 32 blocks, 22 on 'all's roster`.

### M7 — the block inventory, which §2 and §3 are generated from, not written against

Draft 1 enumerated §2's kind table by hand. It came out with 22 rows against a roster of 22 — and they were
**different** sets of 22. Equal cardinality is exactly what no count-based check catches, so the table is now
emitted:

```bash
python3 - <<'PY'   # run in elidex-wt-harness; reads the memos from ../elidex-wt-citeaudit
import re, pathlib
D = pathlib.Path("docs/plans"); disp = D/"2026-07-citation-hygiene-A-rederive.sh"
roster = re.search(r"^all\(\) \{ set -- (.*?)\n\s*local failed", disp.read_text(), re.S|re.M
                   ).group(1).replace("\\\n", " ").split()
al = re.search(r'AUTHOR_LOCAL="([^"]+)"', (D/"2026-07-citation-hygiene-A-rederive-common.sh"
                                           ).read_text()).group(1).split()
DEF = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*)\(\) \{"); defined = {}
for p in sorted(D.glob("2026-07-citation-hygiene-A-rederive*.sh")):
    for l in p.read_text().splitlines():
        m = DEF.match(l)
        if m: defined[m.group(1)] = p.name.replace("2026-07-citation-hygiene-A-rederive","").replace(".sh","") or "(disp)"
C = pathlib.Path("../elidex-wt-citeaudit/docs/plans"); cite = {}
for tag, fn in [("A-i","Ai-spec-label-map"),("A-ii","Aii-gate-failure-semantics"),("A-iii","Aiii-suite-scheduler")]:
    sec = re.search(r"^## §15.*?(?=^## |\Z)", (C/f"2026-07-citation-hygiene-{fn}.md").read_text(), re.S|re.M).group(0)
    for run in re.findall(r"`([^`]+)`", sec.replace("\n"," ")):
        toks = [t for t in run.replace("*","").split() if t in defined]
        if len(toks) >= 2:
            for t in toks: cite.setdefault(t, set()).add(tag)
    for x in re.findall(r"plus `([a-z_]+)`", sec):
        if x in defined: cite.setdefault(x, set()).add(tag)
for m in re.findall(r"rederive ([a-z_]+)|A-rederive\.sh ([a-z_]+)",
                    (C/"2026-07-citation-hygiene-umbrella.md").read_text()):
    b = m[0] or m[1]
    if b in defined: cite.setdefault(b, set()).add("umbrella")
for b in sorted(defined):
    r = "yes" if b in roster else ("author-local" if b in al else "no")
    print(f"{b:<14}{defined[b]:<11}{r:<14}{','.join(sorted(cite.get(b,[]))) or '(none)'}")
print(f"defined={len(defined)} roster={len(roster)} declared={len(cite)}")
print("undeclared AND on roster:", " ".join(sorted((set(defined)-set(cite)) & set(roster))))
PY
```

Reading: **33 defined, 22 on the roster, 18 declared by a memo.**
`undeclared AND on roster: anchors bmemo offline partition selfcheck timing`.

## §2 Q1 — what the canonical validity predicate is

**There is no single one, because the harness holds three kinds of block and only two of them have a predicate
at all.** That is the collapse: not one predicate replacing thirteen shapes, but a partition showing that most
of the shapes have nothing to be a predicate *of*.

| kind | what it produces | valid iff | discriminator (mechanical) |
|---|---|---|---|
| **J1 derive** | a quantity a memo cites | **the command ran** | declared by a memo (M7) and prints a number/listing with no stated expectation |
| **J2 assert** | a verdict on a written-down invariant | a **comparison against a stated expectation** ran *and* held | an expectation exists **in a memo or in the block** — not merely a liveness guard |
| **J3 instrument** | candidate mechanisms run side by side to *decide* an open question | — **malformed question** | **declared by no memo** (M7) *and* runs ≥2 candidates for one job |

⚠ **The J2 discriminator is "an expectation exists", not "the block contains `[ ... ]`".** Draft 1 used the
latter and it does not discriminate: `suiteset`'s `[ "$n" -gt 0 ]`, `filters`' `[ -n "$body" ]` and
`ruleset`'s `[ "$n_id" != 1 ]` are `_measure` **liveness guards** — "did this run?" — which §2 attributes to
J1 by construction. Under the `[ ... ]` test every `_measure` caller is J2 and the boundary vanishes. Only
`couplings` (K2/K3 absolutes) and `selfcheck` (every roster block states its status) carry a real expectation
in the block itself; the rest of J2 gets its expectation from a memo.

**J1 is closed *where `_measure` is used*, and not elsewhere.** `_measure` makes "the command did not run"
unrepresentable as a pass by construction. ⚠ Draft 1 said "J1 is closed and needs nothing further", which
`lanes` falsifies: 4 of its 6 commands bypass `_measure`, and `git log --grep` **exits 0 on no match**, so
§13's "two carve commits" limb prints nothing and passes. The correct statement is that J1 is closed **at the
call sites that route through `_measure`** — which is `selfcheck`'s own limitation, one level up.

**J2 is where umbrella `:92` bites** — *"a claim is admissible only if something mechanically checks it."* A
J2 block that prints two numbers side by side and never compares them is a J1 block wearing a verdict's
clothes. Three blocks are in that state:

- **`citations`** (`-common.sh:78`) prints the authoritative §-title beside the fixture's and never compares.
  Its own comment says it exists because nothing else would catch a fabricated title. **Keep and fix.**
- **`marker`** (`-Aii.sh:285`) prints `recognised: N` beside `a bare grep would report: M` and never compares,
  though its header states the invariant the comparison would check. It uses `_measure` nowhere, so §4's
  "J1 is covered by `_measure`" does not cover it either. **Keep and fix.** Draft 1 filed it J1.
- **`armmatrix`** (`-Aii.sh:213`) binds no row's status — 27 of 27 rows print `EXIT=1` and the block exits 0.

⚠ **`armmatrix` is J2, and draft 1's dissolution of it was wrong on its premise.** Draft 1 argued *"A-ii's
memo is unwritten; do not fix, do not ship."* The memo is **578 lines** and carries ten `→ rederive` pointers
to these blocks; its §5 is a 19-row table with an **expected-exit column** sourced to `rederive armmatrix`,
and its §6 states *"§5 owns the expected values, stated once."* The expectation the J2 verdict needs is
already written down. §1's own M1/M2 said so three sections above the claim. **`armmatrix` ships with A-ii and
its row status gets bound** — the same disposition as `citations`, not the opposite one.

*(`27 of 27`, measured: `sed -n '/^=== armmatrix ===/,/^=== suites ===/p' /tmp/m3.txt | grep -oE 'EXIT=[0-9]+' | sort | uniq -c`.)*

**What survives as J3 is two helpers, not eight blocks**: `_proto` and `_runner` — declared by no memo (M7),
graft candidate control flow, and exist to let an author choose. The three `rc -le 1` sites (`column:34`,
`carvecolumn:57`, `remedies:207`) remain genuinely unclosable by any status widening — `preflight.py` returns
1 for a real HARD FAIL, a missing fixture, an uncaught exception **and** a failed `cd`; the discriminator is
in the child's *stdout*, which those blocks print and never read. Their expectation lives in A-ii §5's column,
so they are J2 too, and the fix is to read the child's stdout rather than its status.

**Kinds for all 33** (from M7; kernel helpers are not shipped blocks and take a fourth label):

| kind | blocks |
|---|---|
| **kernel** | `_measure` `_measured` `_wtscan` `say` `fixtures` `all` `selfcheck` |
| **J1** | `keysets` `regions` `budget` `readers` `lanes` `timing` `anchors` `partition` `offline` `bmemo` `staleclaims` |
| **J2** | `citations` `couplings` `marker` `armmatrix` `column` `carvecolumn` `remedies` `instruments` `reloadstale` `suites` `suiteset` `filters` `ruleset` |
| **J3** | `_proto` `_runner` |

7 + 11 + 13 + 2 = 33. ⚠ **This table is checkable against M7 and must be regenerated with it**, not edited.

## §3 Q2 — which blocks earn existence

**Rule**: umbrella `:89` — *"a slice may not carry another slice's concern"* — applied one level deeper. **A
harness part ships in the PR of the slice that cites it, and no earlier.**

⚠ **Routing is read off the part files and M7's "declared by" column, never off prose.** Draft 1 quoted
`-B.sh`'s header rationale — which covers four blocks — and applied it to six, sending `anchors` and `timing`
to B although both are defined in `-Aii.sh` and cite A-ii's own §3.1 and §11 (A-ii §11's sole defer slot,
`#11-webref-preflight-inprocess-resolution`, is the axis `timing` measures). Routing A-ii's evidence into B's
PR is the violation `:89` exists to prevent.

| ships with | blocks | grounds (M7) |
|---|---|---|
| **kernel** (any branch) | `_measure` `_measured` `_wtscan` `say` `all` `selfcheck` `$REPO_ROOT` | belongs to no slice |
| **A-i** | `citations` `keysets` `readers` `regions` `couplings` `budget` `fixtures` (+`lanes`) | declared by A-i |
| **A-ii** | `column` `carvecolumn` `instruments` `remedies` `reloadstale` `armmatrix` `marker` `anchors` `timing` `_runner` `_proto` | declared by A-ii, or defined in `-Aii.sh` |
| **A-iii** | `suites` `filters` `suiteset` `ruleset` | declared by A-iii |
| **B** | `partition` `offline` `bmemo` `staleclaims` | defined in `-B.sh`; see the ⚠ below |

⚠ **Nothing is deleted for "no consumer".** Draft 1 had such a row and it was an artifact of reading routing
off prose: of its six members, `selfcheck` is kernel, `anchors`/`timing` are A-ii's, `staleclaims` is not even
on the roster, and `partition` is **explicitly owner-routed**. A-i §13.1: *"Do not 'fix' it by reverting the
roster — silence is what let it run broken for four commits."* Deleting the block is a stronger silencing than
the revert that memo forbids.

⚠ **`partition`'s RED is Slice B's, not A-i's.** Draft 1's cause table put it under "A-i creates
`spec_labels.py`". With A-i present it fails `AttributeError: no attribute '_catalog'` — the catalog
fall-through, which is B's. The program memo already records this. Verified:
`cd ../elidex-wt-citeaudit && bash docs/plans/2026-07-citation-hygiene-A-rederive.sh all 2>&1 | tail -1`
→ `FAILED BLOCKS: partition(exit 1)` — one block, not six.

⚠ **`budget` spans two slices.** It is a J1 file-size census *and* calls `_proto` to measure `preflight.py`'s
statement growth — A-ii's subject, which A-i has not touched since draft 3 and its §12(1) forbids. `_proto`
and that limb go to A-ii. **Open**: A-i §8 cites `budget` for its size claims; whether the residual census
still resolves §8 is not settled here and is A-i's to answer at landing.

### The measurement that decides the ordering

M3's six REDs are **one class**: each reads an artifact another slice creates, or an invariant another slice
discharges.

| RED block | reads | created/discharged by |
|---|---|---|
| `keysets` `regions` `offline` | `.claude/tools/_webref/spec_labels.py` — ABSENT | **A-i** |
| `partition` | `spec_labels._catalog()` — the fall-through | **B** |
| `bmemo` | `…-B-detector-correctness.md` — ABSENT | **B** |
| `couplings` | K2's two pre-existing sites | **A-i discharges them** |

```bash
ls .claude/tools/_webref/spec_labels.py docs/plans/*B-detector-correctness.md
git grep -nE '\.claude/(skills|tools)/[A-Za-z0-9_-]+/[A-Za-z0-9_.-]+' -- .claude/tools/
```

The second prints `_webref/cli.py:78` and `.claude/tools/webref:5`. Note `couplings` — the most-cited block
(M7: A-i, A-ii, A-iii) and §12(3)'s exit criterion — ships RED and stays RED until A-i lands, and three of
A-i's own six blocks are RED here.

**A harness cannot be stacked before the slice it measures.**

### Disposition: ▶ (a) — #505 ships the kernel only

Each slice's part lands in that slice's PR.

⚠ **Draft 1 recommended (b) — close #505 and return everything to #501 — and rejected (a) on an objection
that does not exist.** It claimed a kernel-only PR ships `selfcheck` "ranging over an empty roster". False in
both readings: `selfcheck` is itself a kernel block, so the roster is **1**; and a genuinely unparseable
roster is already guarded at `-integrity.sh:145-153`, which raises `SystemExit` rather than passing. The one
command that settles it was never run.

With the blocker gone, (a) wins under the lens and (b) does not:

- (a) applies `:89` **exactly** — every part lands with its citer, nothing is discarded.
- (b) discards ~818 lines that exist today (`-Aii.sh` 345 + `-Aiii.sh` 120 + `-B.sh` 137 + `_proto` 216) with
  **zero `#11-*` slot** and no record that A-ii/A-iii/B must re-create them — a pragmatic scope-cut.
- (b)'s own file set is incoherent: *"the kernel and `-Ai.sh` return to #501"* drops `citations` and
  `couplings`, which live in `-common.sh` — including the CRIT-1 block (a) is meant to keep and fix.
- (b) voids three status registers (MEMORY.md's L3 bullet, the program memo's carve section,
  `active-lane-detail.md:97-98`) and nothing in §6 authorises editing them.

⚠ **What (a) owes**: #505's own describing document. A-i §8 is canonical for the layout figures by the
program's single-home rule, and it is in the PR that lands second. Under (a) the kernel is small enough to
describe in its own PR body; the per-slice parts inherit their slice's memo.

## §4 Q3 — what verifies the harness

**Today: nothing** (M5). Verification is **per kind, and mostly by construction**:

| kind | verified by | why enough |
|---|---|---|
| **J1** | `_measure`, by type — **at the sites that use it** | "did not run" is unrepresentable as a pass. Not a blanket guarantee: see `lanes`. |
| **J2** | the expectation must be **in the block or in a named memo §**, and the block must compare against it | umbrella `:92` |
| **J3** | nothing — it does not ship as a verdict | |
| kernel | `selfcheck` | the one property spanning blocks |

**`selfcheck` keeps its property and gains one.** Today it enforces *every roster block ends in an explicit
`return`*. It gains a per-block `# kind:` declaration and checks the shape implied.

⚠ **The check must cover J1 too, or it does not catch the defect it replaces.** Draft 1 specified only the J2
shape — so `citations`, CRIT-1's own subject, would declare `# kind: J1` and pass with no comparison. J1's
shape is checkable in the same terms: a J1 block must route every reported quantity through `_measure`.

⚠ **Two real defects in the kernel, to fix rather than dissolve**: `selfcheck`'s parser recognises a
definition only when it closes with a bare `}`, so it silently drops `all()` (which closes `; }`) — M6 reports
32 against 33 defined; and its membership check runs one way only (roster → defined), so a defined-and-shipped
block that carries no kind is invisible to the mechanism proposed as its verifier.

⚠ **This note does not invent a provenance grammar.** Draft 1 proposed a `# planted:` comment — a second
spelling of *"a claim carries the command that falsifies it"*, which is exactly the decision surface the live
`stale-claim-detector` program owns (MEMORY.md's first Active-state bullet; it defines a machine-re-executed
`claim` annotation). Two spellings of one rule is the decision-surface duplication `one-issue-one-way`
forbids. **The harness adopts that program's grammar when it lands**; until then a J2 block's expectation is
named in its header, and no new annotation is introduced here.

**Decision: no CI wiring, and this closes the question.** The harness's consumers are memos under review and
its readers are reviewers, who run `all`. A script that creates git worktrees, calls `gh api` and reaches the
network does not belong in every lane's gate for a `docs/plans/` artifact; the scheduling concern that *is*
real is A-iii's, over the Python suites.

## §5 Claims vs checks

| claim | check | status |
|---|---|---|
| B and C cite the harness zero times | M1 | CHECKED |
| six roster blocks are declared by no memo | M7 | CHECKED |
| 6 of 22 roster blocks RED at `12281e3b`; 0 measurement failures | M3, M4 | CHECKED |
| each RED reads an artifact another slice creates / discharges | §3 table's `ls` + `git grep` + the citeaudit run | CHECKED |
| `partition`'s RED is B's, not A-i's | `all` on `webref-cite-audit-tool` → `partition(exit 1)` alone | CHECKED |
| nothing verifies the harness | M5 | CHECKED |
| the kind partition covers all 33 defined blocks | §2's table vs M7, 7+11+13+2=33 | CHECKED |
| A-i's six blocks are green on A-i's branch | ran all six on `webref-cite-audit-tool` → rc=0 each | CHECKED |
| (a)'s "empty roster" blocker does not exist | `-integrity.sh:145-153` + `selfcheck` ∈ kernel | CHECKED |
| `armmatrix`'s expectation is written down | A-ii §5's expected-exit column, §6 "§5 owns the expected values" | CHECKED |
| J2's "expectation exists" discriminator is decidable for every J2 block | — | **UNCHECKED** — decided by reading each block's memo §. `instruments` and `reloadstale` are the doubtful ones: both are cited by A-ii, but whether A-ii states an expectation for them, or only records their output, is not established here. |

## §6 What this note authorises, and what it does not

**Authorises** (after `/elidex-plan-review` passes on this draft): shipping #505 as **kernel-only**; moving
each slice's part into that slice's PR; binding the row status in `armmatrix`; adding the missing comparison
to `citations` and `marker`; reading the child's stdout at the three `rc -le 1` sites; the `# kind:`
declaration with **both** J1 and J2 shapes checked; and `selfcheck`'s two parser fixes.

**Does not authorise**: deleting any block; introducing a provenance annotation that duplicates the
`stale-claim-detector` grammar; editing the status registers (that is (a)'s landing checklist, and (a) leaves
them substantially intact); or patching the 33 `/elidex-review` findings individually — those that survive
this note's dispositions are absorbed by them, and the rest are named above as keep-and-fix.
