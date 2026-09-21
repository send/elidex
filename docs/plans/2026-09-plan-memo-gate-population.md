# Sub-umbrella — "what is each gate's POPULATION?": the five §8 carves routed to Slice 2's plan-review

**Status**: FROZEN after plan-review round 3 (the loop's terminator was reached; there is no
round 4). A SUB-UMBRELLA: it holds the decisions that survived rounds 1–3 (§0.1), each slice's
question / property / population definition, and owned obligations (§0.3). It intends to design no
mechanics; the mechanics questions the rounds raised are in each slice's "OPEN FOR <slice>'s per-PR
plan-review" list (§4) — whether that list is complete is UNREVIEWED. Slices ρ, α, β, γ each get
their own per-PR plan memo and plan-review.
**Landing medium**: its own docs PR, stacked on PR #510 (`4e46d95f`), opened only after #510 merges
(the user's call). At this writing it is neither committed nor pushed; it changes no `.py` file.

**Parent**: [the umbrella](2026-08-plan-memo-umbrella-checker.md), §8 list items (5), (11), (12),
(14), (15). Item (6) (Slice 3) is owned by "its own PR" and is out of scope.

**Conventions**. A figure is re-runnable (its command beside it) or **[prototype]** (an uncommitted
scratch copy of `.claude/tools/`; not re-runnable here; the slice re-measures it). An enumeration is
a **SEED** (command beside it; the population is the stated property, never the list). Corpus claims
are properties (zero / nonzero). **CORPUS**: the SHA-256-distinct contents of `docs/plans/*.md` in
this worktree at HEAD `4e46d95f` plus this memo, and in `elidex-wt-vmp4plan` at HEAD `190d2adb`
(read-only; `git -C <worktree> rev-parse --short HEAD`). An unmeasured complement is **UNREVIEWED**.
`<umbrella>` = `docs/plans/2026-08-plan-memo-umbrella-checker.md`; `<ledger>` = its `-rounds.md`.

## §0 The answer

Two of the five carves are one population question; three are not.
- **(11) + (15)** — *which positions declare rows the census cannot see?* Both put rows outside
  `ids` and the keep-set, so naming sites beside them are lost at rc 0 (§1 U2 / U3). Slice γ.
- **(12)** a READING question and **(14)** a MEASURE question, through one kind classifier. Slice β.
- **(5)** no gate: the id grammar assigns a decoration run by position. Slice α.
- **ρ** — readings as values: a prerequisite α, β and γ consume (§5).

### §0.1 DECIDED

Each item: *reviewed N* = present in the text plan-review round N reviewed, and not refuted by that
round's decision record (no reviewer output survives to say more).

1. The five carves are not one question; (11) + (15) are one — *reviewed 2, 3.*
2. This memo is a sub-umbrella; ρ, α, β, γ each get a per-PR plan and plan-review — *reviewed 2, 3.*
3. (12)'s root is the kind reading's dependence on the keep-set, not the §6.2 demotion tag; the R42
   twins are kept — *reviewed 1, 2, 3.*
4. Kind WORDS are read keep-invariantly; the attribution SUBJECT is read on the naming reading by
   design (R20 / R24 / R27) — *reviewed 3* (round 2's record narrowed it from a refuted universal).
5. (14)'s residue compares the full classification (kind, attributed id, spellings); the census is
   written from the stream classification only — *reviewed 2, 3* (the full classification also 1).
6. (12) + (14) are one slice (one classifier) — *reviewed 1, 2, 3.*
7. β's population is every site matching a kind-phrase pattern against text, `DECLARES` included
   (SEED §1) — *reviewed 3.*
8. (15)'s scope is every memo, main and linked — *reviewed 2, 3.*
9. γ splits the RAW row line before rendering cells (§1 S1) — *reviewed 3.* S1 is stated per cell as `r(cell)`; under γ OPEN 2 reading B the per-cell reading needs ρ OPEN 2's (formerly 3) source→stream map.
10. A ratified control whose measure moves keeps both halves (schema = 1 and its original second
    property) — *reviewed 3.*
11. The refused (11) predicates stay refused: every-body-row id shape (C11 nonzero) and header
    near-miss — *reviewed 1, 2, 3.*
12. Order edges ρ → β, ρ → γ, α → β, β → γ, α → Slice 2, β → Slice 2 #6 (reasons in §5; ρ → γ's reason *restated at freeze*) — *reviewed 3*.
13. Landing medium as in Status — *reviewed 3.*

**Fixed at the freeze (not survivors)**: α's property is CM 0.31.2 §6.2 incl. rules 9 / 10; it settles
D1–D5 only (§4.1). (11) is γ's PRIMARY OPEN question; U2 / U3 are loss witnesses (§4.3). §5's rule "a P12
consumer is ordered after ρ" is written down at the freeze, and the edge **ρ → α** is derived from it (α's
composer `OWNS_TWO` reads `_stream`, a P12 consumer in the SEED); no round reviewed either.

### §0.2 Premises found false

1. Ledger: "(6) Slice 3 … is the fifth" — it is (11) (`grep -n 'fifth and is a different' <ledger>`);
   "four of them are the same question" — two are (`grep -n 'same question\|one question' <ledger>`).
2. `**Owner**: Slice 2's plan-review` stands on (11), (12), (14), (15) only
   (`grep -n "Owner\*\*: Slice 2's plan-review" <umbrella>`); (5) is routed by its Trigger paragraph.
3. (12) is not specific to the demotion: outside any image the verdict flips on whether an unrelated
   row declares `a` (§1 K3 / K4, K5 / K6).
4. (14)'s "same KIND and same ATTRIBUTION" omits the spellings, a gating output. [prototype] comparing
   those two only: ``**UMBRELLA, not a terminal unit.** Also KIND UNDETER`MINED`.`` beside a row
   spelling `KIND-UNDETERMINED` goes rc 2 → rc 0; its clean twin is rc 1.
5. (15) "FOUR ratified controls": [prototype, a rejected per-line arm] seven went red; three of the
   four named are `sites` / `id` measures whose rc half is implicit
   (`grep -n 'res.rc != 2' .claude/tools/plan_memo_selftest_harness.py`).
6. (15) "this checker's own documents do [show a table's shape in prose]" — NOT refuted, UNREVIEWED: C15 is zero, but whether that zero bounds (15)'s population waits on γ OPEN 2.
7. (5) is not about a rejected core: an accepted left core takes the decoration too (§1 D3, D5).
8. (5)'s "3 disagreements" is a report cap (`grep -n 'len(bad) < 3' .claude/tools/plan_memo_selftest_invariants.py`).
9. The umbrella's §3 fails the preflight: `python3 .claude/skills/elidex-plan-review/preflight.py <umbrella>; echo $?` → exit 1.
10. "141 memos" is undeduplicated; the corpus is the CORPUS command's output.
11. This memo's earlier drafts: "(11) closed"; "γ scope undecided"; "one site per kind question"; `\.mask\b` as a population; "the scan ≡ grammar property is unchanged" (α);
    "population ← roles" (`grep -n '^from plan_memo_' .claude/tools/plan_memo_roles.py`).
12. **For the user, recorded only**: the umbrella's option-(d) rationale, accepted by the user, rests
    partly on two reasons refuted here — (12)'s fix "overturns a control this plan RATIFIED"
    (`grep -n 'overturns a control this plan' <umbrella>`; premise 3) and (15)'s "FOUR ratified
    controls" (premise 5). This memo does not rewrite that rationale.

### §0.3 Obligations (each: owner — event trigger — Re-eval 2026-12-31)

- Premises 1–5, 7–10, umbrella and ledger (anchors above) — this memo's docs PR — #510 merge — Re-eval 2026-12-31.
- Premise 6 — γ — γ OPEN 2 decided in γ's per-PR plan-review (the docs PR has landed by then) — Re-eval 2026-12-31.
- Reassign §8's four Slice-2-plan-review Owner lines (premise 2) — docs PR — #510 merge — Re-eval 2026-12-31.
- Slot close condition → "the landing of the LAST of Slice 2, ρ, α, β, γ", at both sites
  (`grep -n 'CLOSE −1' <umbrella>`: the header and §6's Slice 2 acceptance) — docs PR — #510 merge —
  Re-eval 2026-12-31.
- The same condition in the slot SoT ledger
  (`grep -n 'umbrella-checker-prereq' ~/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory/project_open-defer-slots.md`)
  — the session that records this plan's approval — approval of this plan — Re-eval 2026-12-31.
- Umbrella §7's "`_attributed_to_other` (moves to roles)" and its connective-grammar placement
  (`grep -n 'moves to roles\|connective grammar' <umbrella>`) — β (its PR carries the amendment) —
  β's per-PR plan-review answers its OPEN item 3 — Re-eval 2026-12-31.
- The umbrella's 141 / 151 / 511 / 507 figures (`grep -nE '\b(141|151|511|507)\b' <umbrella>`) and
  the same figures in `.py` docstrings, control and mutant names (`grep -nE '\b(141|151|511|507)\b'
  .claude/tools/plan*.py`; complement: "Example 151" in `plan_memo_selftest_mutants_pr510.py` is a
  CommonMark example number), and population's "That half is §8's" — γ — γ's landing — Re-eval 2026-12-31.
- The three "(R32 §2.5/GFM)" control names cite a GFM §2.5 that does not exist
  (`grep -n 'R32 §2.5/GFM' .claude/tools/plan_memo_selftest_*.py`) — γ — γ's landing — Re-eval 2026-12-31.
- Umbrella §8 back-link here (`grep -c 'gate-population' <umbrella>` is 0) — docs PR — #510 merge — Re-eval 2026-12-31.
- The ledger's resume pointer (`grep -n 'STARTS HERE (2026-09-21, head .799349db' <ledger>`) — docs PR — #510
  merge — Re-eval 2026-12-31.
- `MEMORY.md` VM P4 Active-state line (`grep -n 'VM P4 lane — carve PR' ~/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory/MEMORY.md`) — the session that records this plan's approval — approval of this plan — Re-eval 2026-12-31.
- Premise 12 surfaced to the user — the session that delivers this memo — this memo's delivery — Re-eval 2026-12-31.
- The §8 enumeration and counts as each carve closes (§6 sweep) — each slice — its landing — Re-eval 2026-12-31.

## §1 Reproductions at `4e46d95f`

**PROBE** (from `.claude/tools/`, fed to `python3 -` on stdin; `run` → `(rc, reported sites)`; it prints the
D, K1–K6, R and U rows only — K7, R4's SCHEMA count, T1 and S1 are measured by their own row's expression):

```python
import plan_memo_selftest_harness as H
from plan_memo_selftest_cases import build
M = H.load()
def run(t, p="", s=None):
    r, rep = H.run_on(M, t, p, s); return r.rc, len(rep)
HD = "| %s | Slice | Primary module(s) | Slot | Tier | Deps |\n|---|---|---|---|---|%s\n"
ROW = "| **%s** | %s | `w.rs` | — | T1 | %s |\n"; T = "Terminal. Acceptance: must return 6."
T1S = HD % ("#", "") + ROW % ("Wz", "**UMBRELLA, not a terminal unit.**", "**Tq**") + ROW % ("Tq", T, "—")
SIB = lambda h: HD % (h, "---|") + ROW % ("Wz", T, "—") + ROW % ("Tq", T, "—")
U1S = lambda h: HD % (h, "---|") + ROW % ("7z", T, "—")
DOC = "| Round | Finding |\n|---|---|\n| **Wz** | a review round |\n| **Tq** | another round |\n"
L = "See [the walk](slice-9z-sib.md). "; B, P = build, "prefix**C** owns it."; U = lambda t, s: run(B(), L + t, s)
print({k: run(*a) for k, a in dict(D1=(B(), P), D2=(B(), "prefix **C** owns it."), D3=(B(), "7z**C** owns it."), D4=(B(), "prefix`C` owns it."), D5=(B(), "9z**C** owns it."), D6=(B(), "**9z****C** owns it."), K1=(B(i7z="**a**", suz="![UMBRELLA, not **a** terminal unit](img.png)", duz="**a**"),), K2=(B(i7z="**a**", suz="![UMBRELLA, not *a* terminal unit](img.png)", duz="**a**"),), K3=(B(i7z="**a**", suz="UMBRELLA, not **a** terminal unit", duz="**a**"),), K4=(B(suz="UMBRELLA, not **a** terminal unit", duz="**7z**"),), K5=(B(i7z="**a**", suz="UMBRELLA, not `a` terminal unit", duz="**a**"),), K6=(B(suz="UMBRELLA, not `a` terminal unit", duz="**7z**"),), R1=(B(suz="Slice 7z — **UMBRELLA, not a `terminal` unit.**"),), R2=(B(suz="Slice 7z — **UMBRELLA, not a `terminal` unit.** Then **UMBRELLA, not a terminal unit.**", duz="**9z**"),), R3=(B(suz="Slice 7z — **UMBRELLA, not a `terminal` unit.** Then **UMBRELLA, not a terminal unit.**"),), R4=(B(suz="KIND UNDETERMINED.  Also KIND UNDETER`MINED`.", duz="**9z**"),)).items()})
print(dict(U1=(run(B(), L, U1S("No.")), run(B(), L, U1S("#"))), U2=(U("**Wz**9z owns it.", SIB("#")), U("**Wz**9z owns it.", SIB("No.")), U("**Wz**9z owns it.", DOC)), U3=(U("`Tq / 9z` owns it.", SIB("#")), U("`Tq / 9z` owns it.", SIB("No.")), U("`Tq / 9z` owns it.", DOC))))
```

| Row | Carve | Fixture | Observed |
|---|---|---|---|
| D1 | (5) | `build()`, `prefix**C** owns it.` | (0, 0) |
| D2 | (5) | `build()`, `prefix **C** owns it.` | (0, 1) |
| D3 | (5) | `build()`, `7z**C** owns it.` | (0, 0) |
| D4 | (5) | `build()`, `` prefix`C` owns it. `` | (0, 0) |
| D5 | (5) | `build()`, `9z**C** owns it.` | (0, 1) — `C` lost |
| D6 | (5) | `build()`, `**9z****C** owns it.` | (0, 1) |
| K1 | (12) | `build(i7z="**a**", suz="![UMBRELLA, not **a** terminal unit](img.png)", duz="**a**")` | (0, 0) |
| K2 | (12) | `build(i7z="**a**", suz="![UMBRELLA, not *a* terminal unit](img.png)", duz="**a**")` | (1, 0) |
| K3 | (12) | `build(i7z="**a**", suz="UMBRELLA, not **a** terminal unit", duz="**a**")` | (0, 0) |
| K4 | (12) | `build(suz="UMBRELLA, not **a** terminal unit", duz="**7z**")` | (1, 0) |
| K5 | (12) | ``build(i7z="**a**", suz="UMBRELLA, not `a` terminal unit", duz="**a**")`` | (0, 0) |
| K6 | (12) | ``build(suz="UMBRELLA, not `a` terminal unit", duz="**7z**")`` | (2, 0) |
| K7 | (12) | `build(i7z="**an**", suz="This row is **an** umbrella.")` vs the same without `i7z`: `UMBRELLA-MARK?` count | 0 vs 1 (seed) |
| R1 | (14) | ``build(suz="Slice 7z — **UMBRELLA, not a `terminal` unit.**")`` | (2, 0) |
| R2 | (14) | R1's `suz` + ` Then **UMBRELLA, not a terminal unit.**`, `duz="**9z**"` | (1, 1) |
| R3 | (14) | R2 without `duz` | (0, 0) |
| R4 | (14) | ``build(suz="KIND UNDETERMINED.  Also KIND UNDETER`MINED`.", duz="**9z**")`` | (1, 1), SCHEMA 0 |
| T1 | (15) | `r = H.run_on(M, build(), L, T1S)[0]`; `r.rc`, `{"Wz", "Tq"} & set(r.population.ids)` | 0, `set()` |
| U1 | (11) | `run(build(), L, U1S("No."))` vs `U1S("#")` | (0, 0) vs (2, 0) |
| U2 | (11) witness | `run(build(), L + "**Wz**9z owns it.", SIB(h))` | `#` (0, 1); `No.` (0, 0) |
| U3 | (11) witness | ``run(build(), L + "`Tq / 9z` owns it.", SIB(h))`` | `#` (0, 1); `No.` (0, 0) |
| U2-doc / U3-doc | (11) partner | U2's / U3's prose, sibling `DOC` | (0, 0) / (0, 0) |
| S1 | (15) | `line = "\| # \| Slice \| Primary module(s) \| Slot \| Tier&#124;Deps \|"`: split RAW then render each cell, vs render then split, compared to the slice header | False vs True |

S1: `[r(c.text) for c in split_row(line)]` vs `[c.text for c in split_row(r(line))]`, `r` = `Lexed` + `resolve({})` + `rendered`; cmark-gfm forms no table from that line over a 6-cell delimiter (a 5-cell delimiter forms one).

**CORPUS**, **C15** (paragraph lines whose cells render a schema header, relexed per line; ZERO over
`ds`; whether that zero bounds γ's population is UNREVIEWED — γ OPEN 2), **C11** (unbound tables whose
every body row's first cell starts with a row-kind token; NONZERO), from `.claude/tools/` via `python3 -`:

```python
import glob, hashlib
fs = sorted(glob.glob("../../docs/plans/*.md") + glob.glob("../../../elidex-wt-vmp4plan/docs/plans/*.md"))
ds = sorted({hashlib.sha256(open(f, "rb").read()).hexdigest(): f for f in fs}.values())
from plan_memo_memo import Memo; from plan_memo_blocks import split_row
from plan_memo_lexer import Lexed; from plan_memo_stream import rendered; from plan_memo_tables import SCHEMAS
def lx(t, d): x = Lexed(t); x.resolve(d); return x
print(sum(sum([rendered(lx(c.text, m.defs)) for c in split_row(t)] == s.header for s in SCHEMAS)
          for m in map(Memo, ds) for p in m.paragraphs for _, t in p.lines))
from plan_memo_ids import tokens, ROW_KINDS
def idc(c): ts = list(tokens(c.text)); return bool(ts) and ts[0].start == 0 and ts[0].kind in ROW_KINDS
print(sum(1 for m in map(Memo, ds) for t in m.tables
          if t.schema is None and t.rows and all(r.cells and idc(r.cells[0]) for r in t.rows)))
```

The shipped claim gate is ZERO over `ds`: `for f in <ds>; do python3 plan-memo-umbrella-check.py "$f" | grep -c 'binding to NO schema'; done`.

**SEEDS** for P8 and P12 (from `.claude/tools/`; 7 P8 sites and 22 calls / 21 slot accesses at
`4e46d95f`, verified 2026-09-21):

```python
import ast, glob
M = {"search", "finditer", "match", "fullmatch", "findall"}
F = {"stream", "_readings", "split_units", "kind_disagreements", "rendered", "_stream", "dispose"}
for f in sorted(p for p in glob.glob("plan*.py") if "selftest" not in p):
    t = ast.parse(open(f).read())
    lv = {x.id for n in ast.walk(t) if isinstance(n, (ast.For, ast.comprehension))
          and getattr(n.iter, "id", None) == "KIND_PHRASES" for x in ast.walk(n.target)
          if isinstance(x, ast.Name)} - {"name", "n", "_n", "_rx"}
    for n in ast.walk(t):
        if isinstance(n, ast.Call) and isinstance(n.func, ast.Attribute) and n.func.attr in M \
           and getattr(n.func.value, "id", None) in {"MARKER_RE", "UNDETERMINED", "POINTER", "DECLARES"} | lv:
            print("P8", f, n.lineno)
        if isinstance(n, ast.Call) and getattr(n.func, "id", getattr(n.func, "attr", None)) in F:
            print("P12 call", f, n.lineno)
        if isinstance(n, ast.Attribute) and n.attr in {"mask", "tokens", "field"} \
           and getattr(n.value, "id", None) in {"lx", "self", "row", "lexed"}:
            print("P12 slot", f, n.lineno)
```

## §2 Coupled invariants (edge-dense)

- **P1 Decoration assignment** (α) — defined by CM §6.2 (§4.1).
- **P2 Implementation ≡ spec rows** (α) — the scan's verdicts on D1–D5 equal the §6.2 oracle.
- **P3 Scan cost** (α) — one decoration walk per side per id (R29-2).
- **P4 Decoration exception** (ρ, α, β, γ) — in the naming reading an id-only `**` pair or code span
  stands, keyed by the keep-set; `**9z**7z` is two ids (ratified).
- **P5 Kind words are keep-invariant** (β); complement by design: the attribution subject.
- **P6 Straddle residue** (β) — readings disagreeing across a blank are rc 2; a phrase quoted whole
  is a quotation.
- **P7 Kind classification** (β, Slice 2 #6) — first marker decides; marker > undetermined > pointer.
- **P8 Kind-phrase match sites** (β) — every site matching a kind-phrase pattern (`KIND_PHRASES`,
  `DECLARES`) against text; complement: population's `{n: False …}` defaults dict matches nothing; a
  pattern of another name is UNREVIEWED.
- **P9 Table admission + binding** (γ) — shape over RAW text; schema over the rendered cells of a
  line split RAW first.
- **P10 Phase-1 block context** (γ) — paragraph text, raw extent, or container content.
- **P11 Could-not-scan is never clean over (15)'s population** (γ); (11)'s predicate is OPEN.
- **P12 A reading is a value** (ρ) — no consumer reads a disposition another consumer wrote; the
  population is every consumer of a disposition and every write-then-read slot (SEED §1). Which
  readings exist is OPEN (ρ item 1).

Pairs listed; every unlisted pair is UNREVIEWED. P1×P4: the grammar reads what the naming reading
keeps standing. P1×P7: the attribution and licensing composers read decoration. P4×P5: the exception
belongs to the naming reading. P4×P6: a declared id must not hide a straddle (K5). P4×P9 / P4×P11:
rows outside `ids` leave the keep-set (U2 / U3). P5×P7: kind words keep-free, subject on the naming
reading. P5×P8: P5 at every P8 site (K7 measures one; the rest UNREVIEWED). P6×P7: the residue compares (kind, attributed id,
spellings). P6×P12 / P10×P12: the residue's readings and γ's schema-test reading are P12 consumers.
P9×P10: split RAW, then render (S1).

## §3. Spec coverage map

Section numbers verified against `curl -s https://spec.commonmark.org/0.31.2/` and
`curl -s https://github.github.com/gfm/` (`grep -oE '<span class="number">N</span>[^<]*'`), 2026-09-21.

| Spec section | Step | Branch | Touch (site @ `4e46d95f`) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CommonMark 0.31.2 §6.2 Emphasis and strong emphasis | which run decorates an id | a run belongs to the core its §6.2 pair encloses; an unpaired run to neither; rules 9 / 10 apply | `plan_memo_ids.tokens` and the composers | ✗ — α | no |
| CommonMark 0.31.2 §6.1 Code spans | backtick decoration | the span decorates the id it encloses | `plan_memo_ids.tokens` | ✗ — α | no |
| CommonMark 0.31.2 §6.2 (local policy: decoration exception) | naming reading | an id-only pair stands | `plan_memo_stream.dispose` | ✓ unchanged | no |
| CommonMark 0.31.2 §6.2 + §6.4 Images | kind reading | kind words read keep-free, in and out of alt text | the P8 sites | ✗ — β | no |
| CommonMark 0.31.2 §6.1 Code spans / §6.5 Autolinks | residue gate | compare (kind, attributed id, spellings) | `Population._kind_residue` | ✗ — β | no |
| GFM 0.29-gfm §4.10 Tables (extension) | recognition and cell split | RAW shape; split RAW before rendering | `plan_memo_tables.admit_table` | ✗ — γ | no |
| GFM 0.29-gfm §4.10 Tables (local policy: could-not-scan) | declaration outside the census | (15) header formed no table; (11) OPEN | `Population` | ✗ — γ | no |
| CommonMark 0.31.2 §4.8 Paragraphs / §5.1 Block quotes | γ population | paragraph lines incl. lazy ones | `plan_memo_memo.Memo` | ✗ — γ | no |
| CommonMark 0.31.2 §4.4 Indented code blocks (+ §4.5, §4.6) | γ complement | raw extents | `plan_memo_blocks.raw_extent` | ✓ — γ | no |
| CommonMark 0.31.2 §3.1 Precedence | γ boundary | a header inside a multi-line span with a delimiter row following forms a table | `admit_table` | under γ OPEN 2 reading B only | no |
| CommonMark 0.31.2 §2.5 Entity and numeric character references | delimiter `&#45;` | no table forms | `admit_table` | ✗ — γ | no |

### §3.1 User-input touch audit

No opcode / native / ECS surface. Input = repository Markdown.

## §4 Slice seeds

### §4.0 ρ — readings as values (P12)

**Property**: no consumer reads a disposition another consumer wrote. **Population**: P12.
OPEN FOR ρ's per-PR plan-review:
1. **Which readings exist.** Reading A (an earlier round): four values, keep ∈ {naming, ∅} × reader
   ∈ {F, T}. Reading B: those four plus `dispose`'s internal intermediate reading, which decides the
   decoration exception. Refuting measurement for A: for ``A **9z** and `9z` here.`` with keep
   {`9z`}, the intermediate reading `dispose` builds is ``A 9z and `9z` here.``, which equals none of
   the four; for `A **9z** here.` (no code span) it equals one of them. Command (from `.claude/tools/`):
   wrap `plan_memo_stream.stream` to record every `reader=True` call during `dispose(lx, keep)`,
   then compare the first recorded string with `str(stream(lx, rd))` after `dispose(lx, k)` for the
   four `(k, rd)`.
2. The `stream()` signature; the owner of the inverse source → stream map; `Block` plumbing; which
   value `_unkeyed` and roles' `Deps`-emptiness reader take.

### §4.1 α — (5) decoration assignment

**Property (fixed at the freeze)**: a decoration run belongs to the core the CM 0.31.2 §6.2 pair (or
§6.1 span) encloses; a run §6.2 does not pair belongs to neither; rules 9 / 10 apply. **Population**:
every decoration run with an id core on both sides. **Oracle per row** (`printf '%s\n' '<row>' | cmark`,
cmark 0.31.2):

| Row | cmark | Verdict the property gives |
|---|---|---|
| D1 | `prefix<strong>C</strong> owns it.` | `C` decorated — 1 site |
| D2 | `prefix <strong>C</strong> owns it.` | 1 site (unchanged) |
| D3 | `7z<strong>C</strong> owns it.` | `C` decorated — 1 site |
| D4 | `prefix<code>C</code> owns it.` | `C` decorated — 1 site |
| D5 | `9z<strong>C</strong> owns it.` | `9z` and `C` — 2 sites |
| D6 | `<strong>9z****C</strong> owns it.` | one pair encloses BOTH cores — not decided by the property (OPEN 1); "1 site" is interpretation, not oracle-derived |

**Controls**: D1, D3, D4, D5 positive; D2 expects 1 site; D6 waits on OPEN 1; `prefixC`, `prefix**C` negative; ratified
`**9z**7z` families kept. The differential test (linear walk ≡ naive reference) is secondary.
OPEN FOR α's per-PR plan-review:
1. **PRIMARY — which core owns a §6.2 pair's decoration when the pair encloses more than one core**
   (D6: cmark gives one `<strong>` around `9z****C`).
2. The fate of `Token.balanced`, `balanced(m)` and `decorated_id`; composer mechanics (`OWNS_TWO`'s
   separator decoration, `_APPOSITIVE`'s own `DECOR`). An earlier round decided "one grammar
   function"; the next round found that leaving `decorated_id`'s fate open contradicts it, and that
   a differential test over `tokens` alone cannot see a composer's half. Both readings stand: every
   composer delegates decoration to one function, or `decorated_id` survives as a composer entry
   point with its own control. Reproduction: `grep -nE 'decorated_id\(' .claude/tools/plan_memo_*.py | grep -v selftest`
   — the composer sites.

### §4.2 β — (12) the kind reading, (14) the kind measure

**Properties**: decided items 3–7. **Population**: P8. **Complement**: `_claims` stays PRESENCE;
the naming question (R42 twins). [prototype] K1 / K3 → rc 1, K5 → rc 2, R2 / R3 → rc 2, R4 SCHEMA 0;
a keep-free reading for the attribution subject turned R20 / R24 / R27 red.
**Controls**: positive K1, K3, K5, the unbound twin of K3, R2, R3, premise 4's fixture, a `kindcell`
emphasis case (the umbrella's recorded hole); a property control (each canonical kind spelling × each
short-id word × each decoration mark, declared vs not) whose complement is the attribution subject,
with its negative (`` Slice `#11-zz-alpha` — **UMBRELLA…** `` stays attributed); negatives R4,
quoted whole (SCHEMA 0 and KIND-SPELLING 0), R42 twins.
OPEN FOR β's per-PR plan-review:
1. Which naming variant the attribution subject reads.
2. How the marker offset reaches attribution.
3. **Where attribution, the connective grammar and the residue comparison live.** Reading A
   (umbrella §7): attribution and connectives in roles. Reading B (an earlier round): attribution at
   or below the tables level, the residue comparison in population, one site. Refuting measurements:
   for A, roles imports tables (`grep -n '^from plan_memo_tables' .claude/tools/plan_memo_roles.py`),
   so attribution in roles makes population import roles — the census under the assertion layer; for
   B, umbrella §7 places the connective grammar in roles (`grep -n 'connective grammar' <umbrella>`),
   so B needs that amendment or a roles → tables call that tables cannot return.

### §4.3 γ — (11) + (15) declaration sites the census cannot see

**Property for (15)**: a paragraph line whose RAW-split cells render a schema header but formed no
table is rc 2, in every memo. **Complement**: raw extents; admitted tables.
**Witnesses**: T1; U2 / U3 are LOSS WITNESSES of (11), not pinned positives; U2-doc / U3-doc give the
same (0, 0) beside a documentation table keyed by ids.
**Controls**: T1 positive; negatives indented, fenced. Moved ratified
controls keep both halves; which move is γ's to re-derive (seed:
`grep -ln 'header and delimiter of unequal width\|delimiter cell is >=1 hyphen\|one-cell delimiter row\|whose DELIMITER row is lazy\|then the lazy schema header\|R32 §2.5/GFM' .claude/tools/plan_memo_selftest_cases*.py`).
OPEN FOR γ's per-PR plan-review:
1. **PRIMARY — (11)'s predicate**: which tables declare rows? Any predicate must say how it treats
   U2 / U3 versus U2-doc / U3-doc; loss is measured on `ids` and the keep-set. Obstacle, measured: the
   `DOC` rows are id-keyed (C11's `idc` holds on every `DOC` body row), so an id-shape predicate
   cannot separate them, and header near-miss is refused (DECIDED 11). Satisfiability: UNREVIEWED.
2. **Which reading γ asks.** Reading A (an earlier round): one producer "row line → rendered cells"
   shared with `Table.bind`. Reading B: each paragraph's own Phase-2 reading, line-sliced, never a
   relex. Refuting measurement for A: `Table.bind`'s cells are relexed per cell; on the paragraph
   `` x `start `` / slice header / `` end` y `` (no delimiter row) a per-line relex reports 1 header
   while the paragraph's own `Lexed` holds that line inside one code span (`p.lexed.code`), and
   cmark-gfm renders one `<code>`. Command: C15's per-line expression over `Memo(<that file>)`.
   Consequences under reading B only: the complement gains "a line inside a multi-line code span with
   no delimiter row following" (with one, CM §3.1: the table forms); negatives "multi-line span" and
   "the §3.1 pair"; the mutant "per-line relex" turns the multi-line negative red.
3. The shared producer's name and home, if any.

## §5 Order

Import DAG (`grep -n '^from plan_memo_\|^import plan_memo_' .claude/tools/plan*.py`, selftest
excluded; the one indented import is the checker's lazy self-test import): tables → {blocks, ids,
stream}; population → {memo, sibling, stream, tables}; roles → {ids, stream, tables}; nothing imports
roles but the checker. Rule (*stated at freeze*): a P12 consumer is ordered after ρ. Edges: **ρ → β** (β's readings are P12 values); **ρ → γ** (γ's schema test
calls `rendered` or a paragraph reading — a P12 consumer either way; *restated at freeze*); **α → β** (both change the attribution composer: α its decoration, β its inputs);
**β → γ** (β fixes the hidden-claim instance of (11); γ measures (11) after it); **α → Slice 2** (its
licensing and appositive composers read decoration); **β → Slice 2 #6** (its connectives go into
the attribution β's classifier calls); **ρ → α** (α's composer `OWNS_TWO` reads `_stream`, a P12 consumer (SEED); *derived at freeze by this rule*); γ vs Slice 2: no edge found (UNREVIEWED). The
program is Slice 2 + ρ + α + β + γ; the slot closes at the last landing.

## §6 Acceptance (each slice's plan refines it)

Common: `--self-test` and `--mutants` 0 / 0; #506 `--worklist` site set identical or its delta
enumerated; the CORPUS sweep (rc, FATAL per file) identical or enumerated; new controls and mutants
in NEW modules (`wc -l .claude/tools/plan_memo_selftest_*.py | sort -n | tail -4`); `mise run
trip-wires` rc 0; the self-test re-run after the last memo edit. **Sweep at each landing**: STATEMENT
(`grep -n 'id grammar releases a REJECTED\|DEMOTION TAG IS RECORDED\|RESIDUE GATE COMPARES PRESENCE\|PRODUCED NO TABLE\|UNBOUND TABLE THAT MAKES NO KIND' <umbrella>`),
OBLIGATION (`grep -n 'must get\|Fourteen own\|Ten of the fourteen own' <umbrella>`), DOWNSTREAM
(`grep -nE '\((5|11|12|14|15)\)' <umbrella> <ledger>` and the `MEMORY.md` Active-state line).
Mutants: α — greedy decoration (D1 / D3 red), decoration only after a rejected core (D3 red); β —
kind words under the naming keep (K1 / K3 red), subject under keep ∅ (R20 / R24 / R27 red),
presence comparison (R2 red), count comparison (R4 red), spellings dropped (premise 4 red), census
from the reader classification (quoted-whole KIND-SPELLING red); γ — render-then-split (S1 red),
linked-only (a main-memo positive red).

## §7 Touch sets (planned; controls and mutants in new modules)

- **ρ**: `plan_memo_stream.py`, `plan_memo_population.py`, `plan_memo_roles.py`, `plan_memo_tables.py`, `plan-memo-umbrella-check.py`.
- **α**: `plan_memo_ids.py`, `plan_memo_tables.py`, `plan_memo_roles.py`, `plan_memo_selftest_invariants.py` (split first).
- **β**: `plan_memo_population.py`, `plan_memo_stream.py`, `plan_memo_tables.py`, `plan_memo_roles.py`, `plan_memo_selftest_ratchets.py`.
- **γ**: `plan_memo_population.py`, `plan_memo_tables.py`, `plan_memo_stream.py`, the moved controls' modules (§4.3 seed).
