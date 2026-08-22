# Plan — Slice C: retire the superseded discovery method

## §0 Status

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **C**.
**Status**: ⚠ **DRAFT — authored at re-slice time (2026-07-28), NOT yet plan-reviewed.**
`/elidex-plan-review` is required before implementation, per the umbrella. This memo exists so that C's
content has one home and Slice A's boundary (`docs/plans/2026-07-citation-hygiene-A-enforcement-plumbing.md`
§4.2) is auditable rather than inferred — **not** as a review-ready plan. Its §3 table, its counts and its
edit set are all to be re-derived at C's kickoff, against B's landed detector.

**Branch**: new, cut from Slice B's landed head. **Hard prerequisite**: Slice B.

⚠ `preflight.py` exits **1 by design** on this memo until A-ii lands: the section below declares **No spec
surface** (A-ii §4.2.5), and the pre-A-ii gate reads that as "no table". The umbrella's slice table says
exactly this of C, as it does of A-iii; an earlier revision of this paragraph instead said the section did
not exist yet and told A's cross-lane sweep to skip the file, which was the declaration's absence, not a
description of it (Codex R17). The sweep expects EXIT 1 here, with the marker present.

---

## §0.5 / §3. Spec coverage map

**No spec surface** — C touches no spec-defined behaviour: three documentation files (§1). Under A-ii's
§4.2.5 this declaration is the whole of C's §3; fixture rows in a memo with no spec surface are the
workaround §4.2.5 retires (§3 item 4 below).

---

## §1 What C is

CLAUDE.md's "One issue, one way": a better mechanism must **replace** the old one, not coexist with it.
Today the detector and the method it supersedes coexist, and **only the old one is mandated**:

```sh
grep -rn 'cite-audit' .claude/skills/ CLAUDE.md    # → no matches (exit 1, verified 2026-07-28)
```

Three sites, all git-tracked and editable in-branch:

| Site | Current state | C's edit |
|---|---|---|
| `.claude/skills/elidex-review/axes.md:179` | requirement **(2) "≥4 grep pattern"** and **(4) "各 pattern の件数明記"** mandate hand-authored discovery alternations — precisely failure mode #2 in `cite_audit.py:13` ("enumeration-only-by-known-pattern") | replace (2)/(4) with the detector, plus the two requirements the detector's own blind spots imply: attribution-bucket **disposition**, and **one run per cited spec** |
| `CLAUDE.md` § "Spec citation" | documents `heading` / `dfn` / `aoid` / `body` / `css` / `specs`; never mentions `cite-audit` | one paragraph: `cite-audit` is the discovery instrument for citation-sweep work, and its `--strict` exit code is a gate, not a report |
| `.claude/tools/_webref/DESIGN.md` | the `cite_audit.py` bullet describes discovery but not the gate | the reported-class contract and `--strict` semantics, since `axes.md` will point authors here |

Requirements **(1)** (真値先確定 via `dfn`/`aoid`) and **(3)** (engine-independent crates in scope) are
**not** superseded and stay. `axes.md:172` is **not** superseded either — it checks author-written
number ↔ title pairs, which `cite-audit` never compares. Complementary, not duplicated.

---

## §2 Why C is blocked on B, and why that ordering is forced

Retiring a discovery method rests on a **supersession claim**: the replacement reaches at least as far as
what it replaces. That claim is admissible only once something has **measured** the replacement's reach.
Today it is false, and the shape of its falsity is known from B's evidence base:

- `cite-audit` requires the literal `§` glyph, so AO-name citations, `per <spec>` prose lines, and
  spec-URL citations are outside its reach entirely.
- Its label alternation is built from 12 pinned specs while the catalog carries 948, so a CSS-module or
  FileAPI citation is **UNATTRIBUTED**, not attributed-and-checked.
- `--strict` cannot fail on the UNATTRIBUTED bucket at all.

Retiring the grep requirement **before** those land would mandate, as the sole discovery method, a
detector that under-reports on nine measured paths — strictly worse than the status quo it replaces, and
it would convert a visible gap into an invisible one. That is the whole thesis of the program applied to
the program itself.

Conversely, if B lands and C never follows, both methods stay mandated at once — the coexistence
"One issue, one way" forbids. So C is neither optional nor first: it is exactly last.

---

## §3 What C must re-derive at kickoff (nothing here is carried forward as fact)

1. **The reach measurement** B produces — the per-class counts that make the supersession claim
   admissible, and the classes `cite-audit` still cannot see, which C must state as the retirement's
   stated residue rather than omit.
2. **The `axes.md` replacement wording**, against B's landed reported classes. The draft wording in the
   superseded single-PR memo names `UNATTRIBUTED / UNKNOWN-SPEC / REJECTED-TOKEN`; whether those are the
   shipped class names is B's outcome, not C's assumption.
3. **The per-spec requirement's evidence.** The single-`spec=html` run is what hid 17 phantom `XHR §4.3`
   citations; the exact figure is re-derived on B's detector, since B's attribution widening moves it.
4. **A `§3` spec coverage map.** C ships no spec logic, and C is ordered after A-ii, so it declares
   **No spec surface** (A-ii §4.2.5) exactly as A-iii does — not fixture rows. Fixture rows in a memo with
   no spec surface are the fabricated-citation workaround §4.2.5 exists to retire, and they make the gate
   report verified citations for behaviour C does not implement (Codex R8; an earlier revision said
   "fixture rows, as Slice A's was").
5. **The self-referential check**: a plan-review agent applying Axis 4 to *C's own memo* must not
   MIN-flag it for failing to do the thing C retires. If it does, C's own wording is the counter-example
   and the edit is incomplete.

---

## §4 Exit criterion (shape only — re-derive the commands at kickoff)

```sh
grep -q 'cite-audit' .claude/skills/elidex-review/axes.md \
  && grep -q 'cite-audit' CLAUDE.md \
  && ! grep -q '≥4 grep pattern' .claude/skills/elidex-review/axes.md \
  && grep -q '^## Reported classes' .claude/tools/_webref/DESIGN.md \
  && echo RETIRED
```

Today (2026-07-28) this prints nothing. The fourth clause is §1's third site: `DESIGN.md` gaining the
reported-class and `--strict` contract that `axes.md` will point readers to — a chain checking only
`axes.md` and `CLAUDE.md` printed `RETIRED` with that contract still unwritten (Codex R14). ⚠ The needle
is a heading **C itself writes** (`## Reported classes`, the section §1's third row promises), not the
option name: B's tree already carries `cite-audit html --strict` in `DESIGN.md`'s usage block, so a
`--strict` grep was satisfied before C touched the file and let C skip its third edit (Codex R15). Measured
absent at B's base and at A-i's head; C re-measures at kickoff. Note that this is a **doc assertion pinned by a grep, not by a
test** — the honest statement of what checks it, in the umbrella's "claims vs checks" sense, is
`UNCHECKED by a test`. C must carry that row explicitly rather than omit it.

---

## §5 Boundary

C **may not repair citations** (that is D) and **may not change detector semantics** (that is B). Its
entire diff is three documentation files. If C finds itself editing `.claude/tools/_webref/commands/`,
the finding belongs to B and C is mis-scoped.
