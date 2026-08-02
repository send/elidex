# Plan — Slice B: make the `cite-audit` detector sound

## §0 Status

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **B**.
**Status**: ⚠ **DRAFT — re-sliced 2026-07-28 out of the superseded single-PR memo, NOT yet plan-reviewed.**
`/elidex-plan-review` is required before implementation, per the umbrella. The §4.0-§4.1, §4.6 and §5
bodies below are the measured substance and are carried **verbatim** from the pre-slice memo (measured
2026-07-28 at `bf580047`, now `26721cfa` after that day's rebase onto `96a8e47b`); the framing sections
(§0-§2, §7-§13) were rewritten at re-slice time to the A/B/C boundaries. Every count is re-derived at
B's kickoff, because **Slice A lands first and B rebases onto it**.

**Branch**: new, cut from Slice A's landed head. **Hard prerequisite**: Slice A.
**Nature**: a **developer-tooling** PR. Zero `crates/**` diff, zero engine behavior change. What changes
is what an enforcement tool *reports*.

### §0.1 What is B's, and what left

The carve `26721cfa` is a **provenance-preserving move**, not an implementation: it moved
`commands/cite_audit.py`, `spec_labels.py`, the `coverage_map.py` / `webref_data.py` / `cli.py` /
`DESIGN.md` edits and the `preflight.py` change out of PR-A0 onto this branch **unchanged**
(`git diff domform-submittable-category -- .claude/` → 0 lines, still true after the rebase).

B's content is **the detector**: nine measured under-report paths (§4.1) plus the touch-time items that
are literally the same edits (§4.6). Three concerns the pre-slice memo bundled have left:

| Left for | What | Where it went |
|---|---|---|
| **Slice A** | `preflight.py` fail-closed (ex-§4.2); the test relocation (ex-§4.4); the `mise` task + CI job (ex-§4.5); ex-invariant I5 | `2026-07-citation-hygiene-A-enforcement-plumbing.md` §4.1 / §4.4 / §4.3 |
| **Slice C** | `axes.md` (2)/(4), `CLAUDE.md` § "Spec citation", `DESIGN.md`'s reported-class contract (ex-§4.3) | `2026-07-citation-hygiene-C-policy-retirement.md` |

**B may not edit review policy** (umbrella constraint). Where a §4.1 fix changes what a reported class is
*called*, B ships the class and its landing note hands C the shipped names; naming them in `axes.md` is
C's edit.

The thesis stays: **a detector that silently under-reports cannot carry a sweep's exit criterion.** A
hand-authored grep alternation announces its own partiality; a checked-in detector reporting
`0 unresolved` announces completeness. Fixing that is the whole content of this PR.

---

## §0.5 Spec citation table

Every § ↔ title pair looked up with `.claude/tools/webref` on **2026-07-28**. Nothing quoted from memory. These are not citations *this PR implements* — this PR ships no spec logic. They are the citations the detector's regression fixtures use, each chosen because it is a real site in the tree that the current detector gets wrong.

| Cite | § | Exact title | Anchor | webref command |
|---|---|---|---|---|
| the section a suffixed range token silently collapses to | HTML §4.10.21 | Constraints | `#constraints` | `heading --exact html 4.10.21` |
| what `§4.10.21.2-4.10.21.3` actually names | HTML §4.10.21.2 | Constraint validation | `#constraint-validation` | `heading --exact html 4.10.21.2` |
| `§16.2-obsolete` (`ua.rs:68`) → phantom `§16` | HTML §16.2 | Non-conforming features | `#non-conforming-features` | `heading --exact html 16.2` |
| a `§` inside a Rust string literal (`build_entities.rs:68`) | HTML §13.5 | Named character references | `#named-character-references` | `heading --exact html 13.5` |
| the largest inherited-attribution cluster in the tree (13 cites) | HTML §2.1.4 | DOM trees | `#dom-trees` | `heading --exact html 2.1.4` |
| what 16 phantom `XHR §4.3` cites should say | XHR §4 | Interface FormData | `#interface-formdata` | `heading --exact xhr 4` |
| a CSS-module cite `cite-audit` cannot attribute today | CSS Text 3 §4.1.3 | Segment Break Transformation Rules | `#line-break-transform` | `heading --exact css-text-3 4.1.3` |

⚠ Three tokens deliberately **absent** from the table because they are not section numbers and any tool that treats them as such is wrong: `§attr-fs-method`, `§Deferred`, `§C1`. The shipped `_CITE_RE` already rejects all three (`TestAnchorRefsAreNotPhantomSections`); `preflight.SECTION_REF_RE` still does **not** (§4.6.3).

---

## §1 Ideal anchor — a detector's silence must be a proof, not an absence

The tool's docstring (`cite_audit.py:9-23`) states the thesis correctly: successive citation sweeps each shipped a detector that could only find what its author already suspected, so the *candidate set* must be derived rather than authored. That thesis is right, and this PR does not revisit it.

What the shipped implementation does not yet honour is the corollary: **a derived candidate set is only worth more than a grep if the derivation is sound.** A hand-authored grep alternation at least announces its own partiality — a reader sees four patterns and knows there is a fifth. A checked-in detector that reports `0 unresolved` announces completeness. When it under-reports, it converts a visible gap into an invisible one, and it does so under an exit criterion (`--strict`) that a sweep is entitled to trust.

So the ideal is not "a better regex". It is: **every class of input the detector declines to report must be a class it names.** Three consequences drive the whole edit set:

1. A token the grammar cannot parse must be **rejected and counted**, never silently truncated to a shorter token that happens to resolve (§4.1.1).
2. A citation the tool cannot attribute must land in a bucket the exit criterion can gate on (§4.1.5). "Unattributed" is a finding, not a residue.
3. A failure of the tool's *own* infrastructure (a corrupt cache, an unreachable catalog, an unimportable module) must be reported as itself, never as a property of the citations being audited (§4.1.6, §4.1.7). This is the sharpest one: today a truncated cache file makes the tool blame the author's citations. (The sibling case — an unimportable `_webref` making the plan-review gate report success having verified nothing — is the same principle applied to the gate, and is **Slice A's** §4.1.)

The same rule applies to this memo. §4.7 states, per claim, what mechanically checks it, and marks the rest UNCHECKED.

---

## §2 Coupled invariants

Four invariants intersect in `_attribute` and its consumers. They are listed here because the fixes cannot be applied one at a time without transiently breaking another.

- **I1 — token integrity.** The section token the detector reports must be the *whole* token the author wrote, or nothing. Today a suffixed token backtracks to a resolvable prefix (§4.1.1). Fixing I1 alone changes total cite counts, which is why §5 measures it.
- **I2 — attribution reach.** The set of labels the detector can recognise must equal the set `spec_labels.shortname_for` can resolve. Today the regex alternation is built from the 12 pinned `SPECS` while `shortname_for` reaches a 948-entry catalog (§4.1.2). Fixing I2 requires an *enumerable* label set, which `shortname_for` alone cannot supply — §4.1.2 solves that rather than papering over it.
- **I3 — label boundaries.** A label must match on token boundaries, not as a suffix of an identifier (§4.1.3). I3 is structurally guaranteed by I2's fix (whitespace-delimited probing) rather than patched separately — that is why they land together.
- **I4 — text classification.** A `§` is a citation only where a citation can live. Today extraction is never gated on `in_comment`, and `_COMMENT_RE` misclassifies both directions (§4.1.4). I4 changes which lines *end* an attribution block, so it moves cites between buckets and must be measured with I1-I3, not after them.

**I5 has left.** "A gate's exit code must distinguish 'verified nothing' from 'verified everything'" was independent of I1-I4 by construction — it lives in `preflight.main`'s control flow, not in `_attribute` — and it is **Slice A's** J1-J3. Its departure is the cleanest evidence that the A/B boundary is real rather than administrative: nothing in §4.1 references it, and §5's delta table does not move because of it.

---

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21 Constraints | resolution | a **resolvable prefix** of a suffixed token — the silent-pass vector | §4.1.1 — `_CITE_RE` atomic token; regression fixture in `test_cite_audit.py` | ✓ — every truncated token in the tree enumerated by the §4.0 census | no |
| WHATWG HTML §4.10.21.2 Constraint validation | resolution | the token the author actually wrote | §4.1.1 — after the fix this is the reported section, or the whole token is rejected and counted | ✓ | no |
| WHATWG HTML §16.2 Non-conforming features | resolution | word-suffixed token → phantom `§16` UNRESOLVED | §4.1.1 — `crates/css/elidex-style/src/ua.rs:68` | ✓ | no |
| WHATWG HTML §13.5 Named character references | attribution | a `§` inside a Rust **string literal** counted as a citation | §4.1.4 — `crates/dom/elidex-html-parser-strict/src/tokenizer/build_entities.rs:68` | ✓ for `crates/**/*.rs`; ✗ for non-Rust globs (§10-Q4) | no |
| WHATWG HTML §2.1.4 DOM trees | attribution | bucket (b) INHERITED — the largest such cluster in the tree | §4.1.3/§4.1.4 — block-boundary changes move cites in and out of this bucket | ✓ | no |
| WHATWG XHR §4 Interface FormData | per-spec coverage | 16 phantom `§4.3` cites invisible to a `spec=html` run | **Slice C** — the per-spec run requirement added to `axes.md`. B's job here is only to make `cite-audit xhr` trustworthy enough for C to mandate it | ✓ — the census enumerates all 10 attributed specs | no |
| CSS Text 3 §4.1.3 Segment Break Transformation Rules | attribution | a catalog-only label the alternation cannot see | §4.1.2 — `shortname_for` resolves it; `_LABEL_ALT` does not | ✓ under the §4.1.2 index rule | no |

**Breadth**: K=3 specs (`html`, `xhr`, `css-text-3`), M=7 rows → preflight verdict **ok (single PR scope)**.

**Why the breadth is small and honest**: this PR implements no spec algorithm. The rows are fixtures, not obligations. A larger table would be padding — CLAUDE.md's "Supported-surface testing" asks what guards the surface, and here the guard is the regression suite, not spec breadth.

### §3.1 User-input touch audit + the discovery method

**No user-input flow.** Nothing here is reachable from page content, script, or network. The tool's inputs are a developer-supplied `--root`/`--glob` and repository text. The one adversarial-ish input is a **corrupt HTTP cache file** under `$XDG_CACHE_HOME/elidex-webref/` — already treated as a trust boundary by `cache.py:70-81` for `.meta`, and extended to the body in §4.1.6.

**Discovery method** — this is the section `axes.md:179` currently governs, and the one **Slice C** rewrites. The candidate set for this PR was derived, not authored:

1. Every defect below was produced by **executing** the shipped tool against the real tree, not by reading it. Each has a command in §4.0 and a measured value.
2. The site lists are **regenerated by command**, never transcribed. Where a list appears in this memo it is a sample of a command's output, and the command is given.
3. The one enumeration that is *not* mechanically derivable — "have we found all nine classes?" — is marked UNCHECKED in §4.7 and is the reason §10-Q1 exists.

---

## §4 The edit set

### §4.0 The evidence base — one harness, every count

All measurements below: **2026-07-28**, branch `webref-cite-audit-tool` @ `bf580047`, scanning `crates/**/*.rs` at clean `origin/main` content (i.e. **unswept** — PR-A0's repairs are on a different branch, so these numbers are the pre-sweep tree).

**Baseline, straight from the tool:**

```sh
.claude/tools/webref cite-audit html --summary | head -5
.claude/tools/webref cite-audit xhr  --summary | head -8
```

| Quantity | Measured |
|---|---|
| `html`: distinct sections / cites | **412 / 2535** |
| `html`: UNRESOLVED sections / cites carrying them | **64 / 180** |
| attributed to another spec | **2737** |
| UNATTRIBUTED | **6832** |
| **total cites in `crates/**/*.rs`** | **12104** (2535 + 2737 + 6832) |
| `xhr`: `§4.3` / `§4.3.6` | **16 / 1 cites, both UNRESOLVED** |
| distinct specs the tree attributes to today | **10** (`html dom webcrypto ecma262 webidl fetch url streams xhr selectors-4`) |

**Census harness** — the three counts a plain `grep` cannot derive, because they depend on the detector's own regex semantics. Ships as `.claude/tools/_webref/census_underreport.py` so the numbers stay re-derivable after the fixes land (it is also how §5's before/after is produced):

```python
"""cite-audit under-report census. Run from the repo root:
     python3 .claude/tools/_webref/census_underreport.py crates '*.rs'"""
import re, sys
from collections import Counter
from pathlib import Path
sys.path.insert(0, ".claude/tools")
from _webref.commands import cite_audit as ca

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else "crates")
GLOB = sys.argv[2] if len(sys.argv) > 2 else "*.rs"
TOK = re.compile(r"§\s*([0-9A-Za-z][\w.\-]*)")
BND = re.compile(rf"(?<![\w-])({ca._LABEL_ALT})\s*$", re.IGNORECASE)
c = Counter()
for p in sorted(ROOT.rglob(GLOB)):
    if "target" in p.parts: continue
    t = p.read_text(encoding="utf-8", errors="ignore")
    if "§" not in t: continue
    for line in t.splitlines():
        cm = ca._COMMENT_RE.match(line) is not None
        if cm and ca._DANGLING_LABEL_RE.search(line) and not BND.search(line):
            c["D1.3 glued dangling carry"] += 1
        if "§" not in line: continue
        caps = {m.start(2): m.group(2) for m in ca._CITE_RE.finditer(line)}
        if caps and not cm:
            c["D1.4 cite on a non-comment line"] += len(caps)
        for tm in TOK.finditer(line):
            tok = tm.group(1).rstrip(".")
            hit = next((s for q, s in caps.items() if tm.start(1) <= q < tm.end(1)), None)
            if hit is not None and hit != tok:
                c["D1.1 truncated section token"] += 1
for k in sorted(c): print(f"  {c[k]:5}  {k}")
```

| Census output | `crates` `*.rs` | `docs` `*.md` |
|---|---|---|
| D1.1 truncated section token | **69** | **69** |
| D1.3 glued dangling carry | **31** | **1** |
| D1.4 cite on a non-comment line | **89** | **5674** |

The `docs/**/*.md` column is not decoration: it is the evidence for §10-Q4 (the comment model is Rust-specific while `--glob` advertises otherwise), and it is why this PR does **not** widen the default scan set.

### §4.1 Nine under-report paths — all confirmed by execution

Each subsection states the defect, the measurement, and the fix shape. None of these is deferred; a detector that under-reports cannot carry a sweep's exit criterion (locked, D1).

#### §4.1.1 — the trailing `(?![\w-])` truncates instead of rejecting

`cite_audit.py:79-83`. The number body `(\d+(?:\.\d+)*|[A-Z](?:\.\d+)*)` is greedy but **backtrackable**, so a lookahead placed after it does not reject a suffixed token — the body simply gives back characters until the lookahead is satisfied.

```sh
python3 - <<'PY'
import sys; sys.path.insert(0, ".claude/tools")
from _webref.commands.cite_audit import _CITE_RE
for s in ["/// HTML §4.10.21.2-4.10.21.3 step 7", "// §16.2-obsolete", "// §12.3-12.6"]:
    print(repr(s), "->", [m.group(2) for m in _CITE_RE.finditer(s)])
PY
```

Measured (`census_underreport.py`, verified 2026-07-28): `§4.10.21.2-4.10.21.3` → `4.10.21`; `§16.2-obsolete` → `16`; `§12.3-12.6` → `12`. **69 truncated-token sites under `crates`, 37 distinct (token → captured) pairs; 69 more under `docs`, 49 distinct pairs** (§4.0 census).

Both failure directions are live and both exit 0:

```sh
D=$(mktemp -d); printf '/// HTML §4.10.21.2-4.10.21.3 step 7\n' > $D/a.rs
.claude/tools/webref cite-audit html --root $D --strict --summary; echo "EXIT=$?"
```
→ `§4.10.21 [1 cite(s)] Constraints`, **EXIT=0**. The author cited §4.10.21.2 (Constraint validation); `--strict` certifies a section they did not cite. The mirror case, `§16.2-obsolete`, invents a phantom `§16` and reports it UNRESOLVED — noise pointing at a citation that does not exist.

**Fix**: make the number body **atomic** so it cannot give back characters, then reject on the lookahead:

```python
r"§\s*(?-i:(?>(\d+(?:\.\d+)*|[A-Z](?:\.\d+)*)))(?![\w-])"
```

⚠ **The brief listed three candidate fixes; one of them is wrong.** `(?=[^\w.-]|$)` also rejects the suffixed token, but it additionally rejects **sentence-final citations** — `§4.10.5.` followed by a space — because the trailing period is not in its allowed follow-set:

```sh
git grep -hoE '§ ?[0-9]+(\.[0-9]+)*\.($|[[:space:]])' -- 'crates/*.rs' | wc -l   # → 344
```

**344 cites in `crates/**/*.rs`** end a sentence. The atomic form accepts all 344 and rejects all 69 truncations; the lookahead form loses both. Python 3.11+ supports `(?>...)`; the toolchain is 3.14.6. **Only the atomic form is admissible.**

Rejected tokens must not vanish silently — that would trade an under-report for a different under-report. They become a **reported class** (`REJECTED-TOKEN`), which is also what makes the census re-derivable from the tool after this PR (§4.0).

#### §4.1.2 — the alternation bypasses the catalog widening

`cite_audit.py:47-49` builds `_LABEL_ALT` from `LABEL_TO_SHORTNAME` (the 12 pinned `SPECS`, 24 keys). `cite_audit.py:130` and `:145` then index that dict directly. Meanwhile `spec_labels.shortname_for` reaches upstream's 948-entry catalog. The detector therefore cannot see any label the catalog-widening exists to serve:

```sh
python3 - <<'PY'
import sys; sys.path.insert(0, ".claude/tools")
from _webref import spec_labels
from _webref.commands import cite_audit
print("shortname_for('CSS Text 3') =", spec_labels.shortname_for("CSS Text 3"))
print("_LABEL_ALT branches =", cite_audit._LABEL_ALT.count("|") + 1)
print("_attribute:", cite_audit._attribute(["/// CSS Text 3 §4.1.3 segment break"]))
PY
```
Measured: `shortname_for("CSS Text 3")` → `css-text-3`; `_LABEL_ALT` → **24** branches; `_attribute` → `[(1, '4.1.3', None)]` — **UNATTRIBUTED**. `DESIGN.md:51-57` and `spec_labels.py:112-121` both advertise that CSS-module rows resolve without hand-adding; for the *detector* they do not.

**The real constraint** (locked, D1.2): the alternation needs an *enumerable* label set; `shortname_for` alone cannot supply one. The answer is not to enumerate 948×2 labels into one `re.IGNORECASE` alternation — that would be slower than the regex this PR is already deleting, and it would bake a network fetch into module import (`spec_labels` is imported at load time by `cite-audit`, `coverage-map`, `cli`, and `preflight`; `preflight --no-verify` is documented as usable offline).

**Fix — invert the match.** Stop asking the regex to recognise labels:

1. `_CITE_RE` matches only `§<number>` (the §4.1.1 atomic grammar). No label half.
2. Attribution becomes a **bounded left-probe**: from the `§`, walk back over at most `MAX_LABEL_WORDS` whitespace-delimited tokens on the line, strip surrounding punctuation, drop a trailing `spec`, and ask `shortname_for` longest-candidate-first. First hit wins.
3. The same probe replaces `_DANGLING_LABEL_RE` for the end-of-line carry.
4. `shortname_for` gains a **lazy, deterministic reverse index** so each probe is a dict lookup rather than the current O(catalog) linear scan (`spec_labels.py:131-136`).

This is not a workaround; it removes the enumeration requirement rather than satisfying it. It also collapses three other items into one edit: I3 becomes structural (tokens are whitespace-delimited, so `EcsDom` is one token and `shortname_for("EcsDom")` is `None`), and the 48%-of-runtime `_DANGLING_LABEL_RE` scan disappears (§4.6.1). The cap `MAX_LABEL_WORDS` is a measured choice, not a magic number — see §10-Q2.

#### §4.1.3 — no left word boundary on the label half

`_DANGLING_LABEL_RE` (`:94`) and `_CITE_RE`'s label group both start unanchored, so any identifier ending in an alias carries one.

```sh
python3 - <<'PY'   # ships as census_underreport.py; inline here for the site list
import re, sys; sys.path.insert(0, ".claude/tools")
from pathlib import Path
from _webref.commands import cite_audit as ca
BND = re.compile(rf"(?<![\w-])({ca._LABEL_ALT})\s*$", re.IGNORECASE)
for p in sorted(Path("crates").rglob("*.rs")):
    if "target" in p.parts: continue
    for i, l in enumerate(p.read_text(encoding="utf-8", errors="ignore").splitlines(), 1):
        if ca._COMMENT_RE.match(l) and ca._DANGLING_LABEL_RE.search(l) and not BND.search(l):
            print(f"{p}:{i}  carries {ca._DANGLING_LABEL_RE.search(l).group(1)!r}")
PY
```

Measured: **31 glued dangling carries** in `crates/**/*.rs`. `…agent-scoped EcsDom` carries `dom`; `scriptURL` carries `url`; `Non-fetch` carries `fetch`; `innerHTML` carries `html`; `PR5-streams` carries `streams`. Running the corrected and current attributors side by side, **6 cites change spec** — five of them plan-memo pointers (`docs/plans/2026-06-agent-scoped-ecsdom-world.md §6`) currently mis-attributed to `dom`, correctly UNATTRIBUTED after the fix.

**Fix**: subsumed by §4.1.2's probe. Kept as its own numbered defect because it needs its own regression fixture — the probe could regress to substring matching without a test that pins `EcsDom`.

#### §4.1.4 — extraction is never gated on `in_comment`, and `_COMMENT_RE` is wrong in both directions

`cite_audit.py:117-146`. `in_comment` is computed and used to reset `block_spec` and to gate the carry, but **never to gate extraction** (`:126`). And `_COMMENT_RE = ^\s*(///|//!|//|\*|/\*)` is `^`-anchored, so it misclassifies both ways.

```sh
python3 .claude/tools/_webref/census_underreport.py crates '*.rs'
```
Measured: **89 cites extracted from non-comment lines**, breaking down as **58** inside `"…"` string literals, **19** in a trailing `// …` comment on a code line, **3** inside a `/* … */` body whose continuation lines carry no leading `*` (`crates/css/elidex-style/src/ua.rs:113`/`:115`/`:127`), and **9** other code lines (assertion messages spanning lines).

The `\*` branch is the sharpest: **225** lines in `crates/**/*.rs` match `^\s*\*` and are treated as comments; **221 of them are `*deref` statements** (`*d ^= *k;`, `*self.count.lock().unwrap() += 1;`). The branch is 98% false positives, and each one wrongly *continues* an attribution block that a real non-comment line should have ended.

**Fix**: replace the per-line regex with a small **stateful Rust-comment scanner** — line comments (`//`, `///`, `//!`) from their start position, block comments (`/* … */`, nestable) tracked across lines, string/char literals (including raw strings `r#"…"#`) excluded. Extraction is then gated on "this `§` is inside a comment span", and a trailing `// … §x` attributes to the enclosing block instead of resetting it. This is the one fix with real implementation weight; it is also the one whose absence makes every bucket count approximate.

#### §4.1.5 — `--strict` cannot fail on the UNATTRIBUTED bucket

`cite_audit.py:288`: `if args.strict and unresolved: sys.exit(1)`. UNATTRIBUTED is printed and never gated — **6832 of 12104 tree-wide cites** (§4.0), 56%.

```sh
D=$(mktemp -d); printf '/// bogus §4.10.79.1 with no label\n' > $D/a.rs
.claude/tools/webref cite-audit html --root $D --strict --summary; echo "EXIT=$?"
```
→ `UNATTRIBUTED: 1 cites`, **EXIT=0**. A citation to a section that does not exist in any spec passes the strict gate because no label precedes it.

**Fix**: `--strict` gates on unresolved **and** unattributed **and** the new rejected-token class. A sweep scopes with `--root` / `--prefix` to the set it actually swept; a tree-wide strict run is expected to fail today and that is the correct signal. **Slice C** turns dispositioning the bucket into a documented requirement rather than a hope; B's obligation is to make the bucket gateable at all.

#### §4.1.6 — the per-citation blanket `except Exception`

`cite_audit.py:271`, justified as "resolver raises varied lookup errors". The justification is false, and each clause is checkable:

- `resolver.lookup_heading` **returns `None`** on a miss — its docstring (`resolver.py:216-226`) says so explicitly ("Otherwise return None — even if prefix-matches exist").
- `NotFound` is absorbed by `try_fetch_data` (`webref_data.py:102-105`) before it reaches the caller.
- Network failures are `sys.exit` → `SystemExit`, a `BaseException`, which `except Exception` does not catch at all.

The only arrival is a **corrupt cached extract**, and that is one fact about one spec re-swallowed once per citation:

```sh
cp -R ~/.cache/elidex-webref /tmp/ct/elidex-webref
python3 -c "import pathlib;p=pathlib.Path('/tmp/ct/elidex-webref/97f1441d4ca2d1575daeca71927fd6803be22252');b=p.read_bytes();p.write_bytes(b[:len(b)//2])"
XDG_CACHE_HOME=/tmp/ct .claude/tools/webref cite-audit html --summary --strict; echo "EXIT=$?"
```
Measured: `UNRESOLVED sections: 412 cites carrying them: 2535`, **EXIT=1**. The single underlying fact, unswallowed, is `json.JSONDecodeError: Unterminated string starting at: line 4361 column 15 (char 146669)`. The tool tells the author their 412 citations drifted; the truth is one cache file is truncated.

**Fix**: hoist resolution out of the per-citation loop. Load the spec's headings extract **once**, build a `number → (title, anchor)` index once, then resolve every cited section against that index. A load failure is reported once, as itself, with the cache path and the remedy (`rm` the entry / `ELIDEX_WEBREF_NO_CACHE=1`), and exits non-zero without printing a citation report at all.

Hoisting is the same edit as three efficiency findings, so they land here rather than as a separate concern (§4.6.1).

#### §4.1.7 — `_catalog()` cannot catch what actually fails, and fails open when it can

`spec_labels.py:80-92`. The docstring promises "an offline run degrades to the pinned set". It does not:

```sh
python3 - <<'PY'
import sys, urllib.request, urllib.error, os
sys.path.insert(0, ".claude/tools")
os.environ["XDG_CACHE_HOME"] = "/tmp/empty-cache"
urllib.request.urlopen = lambda *a, **k: (_ for _ in ()).throw(urllib.error.URLError("offline"))
from _webref import spec_labels
try: print("returned", spec_labels.shortname_for("CSS Text 3"))
except SystemExit as e: print("SystemExit ESCAPED _catalog():", e)
PY
```
Measured: **`SystemExit ESCAPED _catalog()`**. `cache.py:131` raises `sys.exit` on `URLError`, and `SystemExit` is a `BaseException`.

The second half is worse. When `except Exception` *does* fire (a malformed `index.json`), `_catalog()` returns `{}` — and `shortname_for("CSS Text 3")` becomes `None`, which is exactly the CSS-module **fail-open** that `spec_labels.py:118-121` claims to have removed ("the plan-review gate previously failed **open** on those, soft-warning and skipping citation verification").

**Fix**: `_catalog()` returns a discriminated result — *available* (a possibly-empty dict) vs *unavailable* (with the cause). Callers act on the distinction: `cite-audit` puts a label it cannot resolve because the catalog is unreachable into a distinct `UNKNOWN-SPEC` class rather than silently into UNATTRIBUTED; `preflight` treats it as **Slice A's** capability precondition treats an unimportable tools tree — **not survivable without `--no-verify`** — which is why B appends to that precondition rather than inventing a second one. `SystemExit` is caught explicitly alongside `Exception`.

#### §4.1.8 — the catalog reverse lookup is first-wins ambiguous

`spec_labels.py:131-136`. The `("shortTitle", "title")` field loop is **inner**, so an earlier spec's `title` beats a later spec's exact `shortTitle`, and dict iteration order decides which spec claims a label.

```sh
python3 - <<'PY'
import sys; sys.path.insert(0, ".claude/tools")
from _webref import spec_labels
cat = spec_labels._catalog()
bad = [(s, spec_labels.label_for(s), spec_labels.shortname_for(spec_labels.label_for(s) or ""))
       for s in cat if spec_labels.shortname_for(spec_labels.label_for(s) or "") != s]
print(f"non-round-tripping: {len(bad)} / {len(cat)}")
def ser(x): return ((cat.get(x) or {}).get("series") or {}).get("shortname")
print("  same series:", sum(1 for s,_,b in bad if b and ser(s)==ser(b)),
      " different series:", sum(1 for s,_,b in bad if not b or ser(s)!=ser(b)))
for r in bad[:6]: print("   ", r)
PY
```

Measured: **203 of 948** catalog shortnames do not round-trip — **200** land in the same series at a different level, **3** land in a different series entirely. The dangerous shape is the level collision: `pointerevents4` → `Pointer Events` → `pointerevents3`; `wai-aria-1.3` → `WAI-ARIA` → `wai-aria` (1.2); `webaudio-1.1` → `Web Audio API 1.1` → `webaudio-1.0`; `cssom-1` → `CSSOM` → `cssom`. Consequence: `coverage-map` emits a label, `preflight` reads it back as a **different spec level**, and citation verification silently runs against the wrong document.

There is a second, smaller hole in the same function: the shortname branch is `if key in catalog` with `key` already lower-cased, so a mixed-case catalog shortname never round-trips — measured, `shortname_for("DOM-Level-2-Style")` → `None`.

**Fix — one deterministic index, three ordered rules**, replacing the linear scan:

1. `SPECS` pinned map wins, verbatim.
2. An exact **shortname** match (case-insensitively) wins next, resolving to that spec verbatim.
3. A title/shortTitle match resolves to that spec — **unless** the string equals the *series'* own title, in which case it resolves to `series.currentSpecification`.

Rule 3 is what collapses the level ambiguity structurally: the catalog carries `series.currentSpecification` for every entry, so `cssom`/`cssom-1`, `selectors`/`selectors-4`, `pointerevents`/`pointerevents4` each fold onto one shortname (**661 distinct series** vs 948 shortname keys). A label that names a *level* still resolves to that level.

Paired with it: **`label_for` must return a label that round-trips, or the shortname.** Measured, **747 of 948** round-trip under the index; the other 201 render as their shortname in `coverage-map` rows. Less pretty, never wrong — and it is the same last-resort `coverage_map._spec_label` already chose (`coverage_map.py` docstring: "The last-resort now returns the shortname itself, which `shortname_for` DOES round-trip"). The round-trip becomes a test over the whole catalog, not a sample.

#### §4.1.9 — `errors="ignore"` and `except OSError: continue` drop a whole file

`cite_audit.py:155-158`.

```sh
D=$(mktemp -d)
python3 -c "import pathlib;pathlib.Path('$D/a.rs').write_bytes('/// HTML §4.10.79.1 café\n'.encode('latin-1'))"
.claude/tools/webref cite-audit html --root $D --strict --summary; echo "latin-1 EXIT=$?"
python3 -c "import pathlib;pathlib.Path('$D/a.rs').write_text('/// HTML §4.10.79.1 cafe\n')"
.claude/tools/webref cite-audit html --root $D --strict --summary; echo "utf-8   EXIT=$?"
```
Measured: latin-1 → `0 cites`, **EXIT=0**; the same citation as UTF-8 → `1 cite`, **EXIT=1**.

**Honest weighting**: field realism is low. Rust source is UTF-8 by definition, and the shipped default (`--root crates --glob '*.rs'`) has no non-UTF-8 file. The trigger is a non-default `--glob`/`--root` or a permissions accident. This is **hardening, not a live gate hole** — it is in scope because a `--strict` gate must not have a silent path that removes a file from the audited set, not because it is currently firing.

**Fix**: decode strictly; on `UnicodeDecodeError` or `OSError`, record the path in a `SKIPPED` class and fail `--strict`.

### §4.2 What left, and where the seams are

Four subsections of the pre-slice memo lived here. They are **not summarised** — a summary beside the
real memo is a second decision surface. Each is now stated once, in its own slice's memo:

| ex-§ | Concern | Now |
|---|---|---|
| §4.2 | D2 — `preflight.py` fails open when `_webref` is unimportable | **Slice A** §4.1. A also *corrects* the fix this memo proposed: the tri-state sited in `shortname_from_label` still exits 0 for a memo whose §3 rows carry no spec label, measured. |
| §4.3 | D3 — retire `axes.md`'s "≥4 grep pattern" requirement | **Slice C**, blocked on B's reach measurement |
| §4.4 | D4 — move the consumer-derivation assertion off the tools package | **Slice A** §4.4 |
| §4.5 | D5 — wire the `_webref` suites into `mise` + CI | **Slice A** §4.3 |

**Two seams B must respect at kickoff**, both created by A landing first:

1. **`preflight.SECTION_REF_RE` is untouched by A** — deliberately, so B's one-grammar collapse (§4.6.3)
   is a single edit rather than a merge against A's changes to the same file. B rebases onto A and edits
   `preflight.py` for the grammar only.
2. **`test_preflight.py` will already exist** (A creates it with P1-P6). B's `parse_spec_cell` and
   catalog-availability cases are *additions* to that file, not a new file — check before writing.

### §4.6 Touch-time items folded in

CLAUDE.md's touch-time discipline: fix the seam while writing it, not after review finds it.

#### §4.6.1 — the hot path, all three arrivals, one edit

Profile of the shipped tool (`python3 -c "import cProfile …"` over `cite-audit html --summary`, 2026-07-28, **1.587 s** total):

| Site | Measured |
|---|---|
| `re.Pattern.search` (i.e. `_DANGLING_LABEL_RE`) | **0.762 s tottime, 48% of the run**, 115,225 calls → 393 carries |
| `json.decoder.raw_decode` | **0.197 s**, **413 calls** — the 293 KB `headings/html.json` re-parsed once per resolved section |
| `resolver.lookup_heading` | **0.329 s cumulative** for 412 lookups; `heading_number_title` called **296,903** times (412 × 1,236 headings) |

All three are removed by edits this PR already makes: §4.1.2 deletes `_DANGLING_LABEL_RE` outright, and §4.1.6's "load and index the extract once, outside the loop" is literally the fix for the other two. Nothing extra is being invented.

Note the in-code justification at `cite_audit.py:139-141` that forbids the cheap prefilter ("multi-word labels such as 'Web Cryptography API' do not end in any single alias") was **invalidated by the carve's own refactor**: `spec_labels.py:69-77` now includes `entry[1]`, the canonical multi-word label, as a parse key. Measured, an `endswith` prefilter over the current key set gives **0.767 s → 0.021 s (36×), identical output (393 == 393)**. It is recorded because it is evidence that a load-bearing perf comment can outlive its premise — but it is **not the fix shipped**, since §4.1.2 removes the regex entirely.

Whether `resolver.lookup_heading` itself is re-pointed at the shared index (benefiting `coverage-map` and `preflight` too) or the index stays local to `cite-audit` is §10-Q3.

#### §4.6.2 — the emitter signature

`cite_audit.py:175` / `:202`: `_emit_text` and `_emit_json` share a **9-positional-parameter** signature, four of whose arguments (`sections`, `unresolved`, `total_cites`, `unresolved_cites`) are pure derivations of `by_section` + `resolved`. This PR adds up to three new reported classes (`REJECTED-TOKEN`, `UNKNOWN-SPEC`, `SKIPPED`), so the signature would be edited in lockstep three times.

Compounding: `args.summary` and `args.show_unattributed` are read **only** in `_emit_text`, so `--format json --summary` still dumps every record and `--show-unattributed` is a no-op there — two advertised flags one output path ignores.

**Fix**: one `AuditResult` dataclass carrying the raw maps, with derived values as properties; both emitters take `(args, result)`. Both honour both flags.

#### §4.6.3 — one section-number grammar, not three

`preflight.SECTION_REF_RE` (`preflight.py:77`) is a third independent grammar alongside `cite_audit._CITE_RE` and `resolver.py:211`'s discriminator, and it carries the **phantom-section defect `cite_audit` already fixed**:

```sh
python3 -c "
import sys; sys.path.insert(0,'.claude/skills/elidex-plan-review'); sys.path.insert(0,'.claude/tools')
import preflight
for c in ['ECMA-262 §Deferred marker','WHATWG HTML §C1 note','WHATWG HTML §4.10.21.2-4.10.21.3 step 7']:
    print(repr(c), '->', preflight.parse_spec_cell(c))"
```
Measured: `§Deferred` → section `D`; `§C1` → section `C1`; both then reach `verify_citation` → non-zero → **HARD FAIL** on a memo whose `§Deferred` / `§C1` are internal markers. (The range case is the mirror image: `preflight` yields `4.10.21.2`, the *correct* first endpoint, while `cite_audit` yields `4.10.21`. Two grammars, opposite defects — the clearest possible statement that there should be one.)

**Fix**: `section_sort.py` — already the established home for section-number syntax and already shared by resolver / aoid / heading / inventory — exports one `SECTION_NUMBER_RE`. `cite_audit` and `preflight` both import it. `resolver.py:211`'s discriminator is a *routing* predicate (number vs AO name), not a token grammar, and stays. **Slice A left `preflight.SECTION_REF_RE` byte-identical on purpose** (A §4.2) so this stays one edit rather than a merge.

### §4.7 What is mechanically checked, and what is not

| Claim | What mechanically checks it |
|---|---|
| Suffixed tokens are rejected, not truncated | `test_cite_audit.py::TestTokenIntegrity` — fixtures for `§A.B-C.D`, `§N-word`, `§N.N-N` |
| Sentence-final `§N.N.` still parses | same class, fixture `§4.10.5. Next sentence` — the case that falsifies the lookahead variant |
| Rejected tokens are counted, not dropped | `--format json` `rejected_tokens` asserted non-empty for the above |
| Catalog-only labels are attributed | fixture `/// CSS Text 3 §4.1.3` → `css-text-3`, with the catalog stubbed so the test is offline-deterministic |
| A label does not match as an identifier suffix | fixtures `EcsDom`, `scriptURL`, `innerHTML`, `Non-fetch` |
| `§` in a string literal / raw string is not a citation | fixtures incl. `r#"… §4.10.5 …"#` |
| A trailing `// … §x` attributes to its block | fixture pair (code line + preceding labelled doc comment) |
| `*deref;` does not continue a comment block | fixture `*self.count += 1;` between two comment blocks |
| `--strict` fails on UNATTRIBUTED | `TestStrictExitCode` extension; **plus** a `cli.main` end-to-end case (below) |
| A corrupt cached extract is reported once, as itself | `test_cite_audit.py` with a truncated fixture extract; asserts the message names the cache and that **no** citation is reported UNRESOLVED |
| `_catalog()` distinguishes unavailable from empty | `test_spec_labels.py` with `urlopen` patched to raise `URLError` — asserts no `SystemExit` escapes and the result is *unavailable* |
| Every catalog shortname round-trips | `test_spec_labels.py` over all **948** entries under the §4.1.8 index rules |
| The suites run at all | **Slice A** — `mise run tools-test` + the GitHub `tools` job. B inherits enforcement rather than building it, which is why B's own exit criterion (§12) can be a red/green pair rather than "and something runs it" |
| The nine classes are *all* the under-report paths | **UNCHECKED.** Nine is what execution found; it is not a proof of exhaustion. §10-Q1 is the honest mitigation, and the `REJECTED-TOKEN` / `UNKNOWN-SPEC` / `SKIPPED` classes exist precisely so a tenth class surfaces as a count instead of as silence |
| The 2026-07-28 counts in this memo | **Re-derivable, not pinned.** Every one ships its command; none is asserted from memory. They will drift as the tree changes — that is expected, and §12's exit criterion does not depend on any of them |

**Coverage gap this PR must close, named explicitly**: no test drives `build_parser().parse_args()` → `args.func(args)` for any subcommand. Deleting the `--strict` argparse block from `cli.py` entirely leaves all 36 tests green — the CLI layer, where the original "`--strict` was a no-op" defect actually lived, is uncovered. Since §4.1.5 changes exactly what `--strict` gates on, at least one end-to-end `cli.main` case ships with it (§6-C1).

---

## §5 Behavior deltas

Nothing in `crates/**` changes. What changes is the tool's reported numbers, and they change a lot — which is why this is measured rather than asserted.

Prototype of §4.1.1 (atomic grammar) + §4.1.2 (left-probe over a series-normalized index, 6-word cap), run against the same tree:

```sh
python3 .claude/tools/_webref/census_underreport.py --delta crates '*.rs'   # ships with the fix
```

| Bucket | Before | After | Δ |
|---|---|---|---|
| UNATTRIBUTED | 6832 | **5733** | **−1099** |
| `html` | 2535 | **2499** | −36 |
| `dom` | 876 | 862 | −14 |
| `ecma262` | 577 | 567 | −10 |
| `webidl` | 337 | 334 | −3 |
| `cssom` (new) | 0 | **172** | +172 |
| `cssom-view` (new) | 0 | **153** | +153 |
| `FileAPI` (new) | 0 | **76** | +76 |
| **total cites** | 12104 | **12062** | −42 |
| **distinct attributed specs** | **10** | **65** | +55 |

Three deltas need naming, not just reporting:

1. **`html` loses 36 cites.** They were never HTML — they are CSSOM / CSSOM-View / FileAPI cites that the 12-spec alternation could not attribute, which then inherited `html` from an enclosing block. This *narrows* what `cite-audit html --strict` audits. That is the intended direction (a sweep should not be handed another spec's citations) but it moves the number a sweep's exit criterion reads, so any in-flight sweep must re-baseline.
2. **Total drops 42**, ≈ the 69 truncated-token sites minus those whose captured prefix was already the whole token elsewhere on the line. Those 42 do not disappear — they move to `REJECTED-TOKEN`.
3. **Attributed specs go 10 → 65.** UNATTRIBUTED is still 5733, and that is correct: the residue is genuinely bare `§N.N` in comment blocks that never name a spec, plus plan-memo pointers (`docs/plans/….md §6`). **Slice C** makes dispositioning it a documented requirement.

⚠ The exact after-numbers depend on two rules §10 leaves open (the probe word cap, Q2; and whether attribution widening is opt-in, Q6). Re-measure with the shipped rules; do not carry this table forward as fact.

---

## §6 Test plan

New/changed tests, by file. Every one must **fail at `bf580047`** — §12 makes that a runnable check rather than a promise.

**`test_cite_audit.py`** (36 today):
- **T1** `TestTokenIntegrity` — 6 fixtures: `§4.10.21.2-4.10.21.3`, `§16.2-obsolete`, `§12.3-12.6`, `§4.9.5-7` all REJECTED; `§4.10.5.` and `§4.10.5, and` accepted. Pins the atomic form and, by the first case, forbids the lookahead form.
- **T2** rejected tokens appear in `--format json` and in the text summary count.
- **T3** `TestCatalogWidening` — `/// CSS Text 3 §4.1.3` → `css-text-3`, catalog stubbed.
- **T4** `TestLabelBoundaries` — `EcsDom` / `scriptURL` / `innerHTML` / `PR5-streams` carry nothing.
- **T5** `TestCommentSpans` — string literal, raw string `r#"…"#`, trailing `//` on a code line, `/* */` body without leading `*`, `*deref;` statement. Five fixtures, one per measured cause.
- **T6** `--strict` exits 1 on an UNATTRIBUTED-only tree (the `§4.10.79.1` case).
- **T7** corrupt extract → single diagnostic naming the cache, **zero** sections reported UNRESOLVED, non-zero exit.
- **T8** non-UTF-8 file → `SKIPPED` class, `--strict` exits 1.
- **T9** emitter parity — `--format json --summary` omits per-cite records; `--show-unattributed` is honoured by both emitters.
- **C1** *(the coverage gap)* — one end-to-end `cli.main` case: `sys.argv` patched, `--strict` on a fixture tree, `SystemExit` code asserted. Mutation check: deleting the `--strict` argparse block must turn this red.

**`test_spec_labels.py`** (new):
- **S1** round-trip over **all 948** catalog entries under the §4.1.8 rules.
- **S2** level collisions resolve to the level named (`pointerevents4` ≠ `pointerevents3`); level-less series titles resolve to `series.currentSpecification`.
- **S3** mixed-case shortname (`DOM-Level-2-Style`) round-trips.
- **S4** `urlopen` raising `URLError` → no `SystemExit` escapes; result is *unavailable*, not `{}`.
- **S5** pinned `SPECS` win over the catalog for every pinned key.

**`test_preflight.py`** (created by Slice A — B **adds** to it, does not create it):
- **P4** catalog unavailable -> hard fail, and the remedy line does **not** say "add the spec to `spec_labels.py::SPECS`" (§4.1.7's discriminated `_catalog()` reaching the gate).
- **P5** `parse_spec_cell` on `§Deferred` / `§C1` yields no citation (shared `SECTION_NUMBER_RE`, §4.6.3).

WARN: A's P1-P6 already occupy that file. Read it before writing -- A's P5 pins the *tools-unavailable* remedy string and B's P4 pins the *catalog-unavailable* one: two causes, two strings, one file.

**Existing tests that must change**, not silently keep passing:
- `test_prefix_tolerant_resolver_is_pinned_to_an_exact_match` (`:398`) — its name and docstring state `lookup_section` is "prefix-tolerant", the **opposite** of `resolver.py:216-226`'s documented contract, and it passes with the `hit[0] == section` guard deleted, so it pins nothing. Renamed and made real against the shared index.
- `test_json_records_carry_relative_paths_for_both_classes` (`:388`) asserts two counts and nothing about paths, while `_record` (`:170`) emits unrelativized `str(path)`. Either the assertion becomes real (relativize, matching `agent_brief.py:59`) or the test is renamed to what it checks. **Recommendation: relativize** — machine-local absolute paths cannot be pasted into a memo or diffed across machines, which is the artifact this tool exists to produce.

---

## §7 Layering check

**CLAUDE.md "VM host/ is engine-bound only"** — not applicable; no `crates/**` diff.

**`DESIGN.md` generic-core / elidex-adapter split** — this is the live boundary, and every edit is placed against it:

| Edit | Layer | Placement |
|---|---|---|
| §4.1.1 token grammar | **generic** | `section_sort.SECTION_NUMBER_RE` — section-number syntax already lives there |
| §4.1.2 probe + reverse index | split: index **generic** (`spec_labels`), probe **adapter** (`cite_audit`) | the index is a property of upstream's catalog; "how a Rust comment names a spec" is elidex policy |
| §4.1.4 comment scanner | **adapter** | Rust-specific by construction; belongs with the tree-scanning policy, not the core |
| §4.1.6 index-once resolution | **generic** | `resolver` / `webref_data` — the caching and lookup layer |
| §4.1.7/§4.1.8 catalog contract | **generic** | `spec_labels` + `sources/webref_data` |
| §4.6.3 shared `SECTION_NUMBER_RE` | **generic**, with one adapter consumer | `section_sort` exports it; `preflight.py` imports it. This is B's only edit outside `_webref` |

**The A/B/C split is itself the layering statement.** Every row above is generic-core or adapter; the two
rows the pre-slice memo carried here — gate policy (`preflight.py`) and doc edits (`axes.md` /
`CLAUDE.md`) — were the **elidex-skill** and **elidex-policy** layers, and they are now A's and C's. A
plan that spans three `DESIGN.md` layers in one PR is the shape that boundary exists to prevent.

**One-issue-one-way**, three collapses in this PR: three §-number grammars → one (§4.6.3); two label-recognition mechanisms (regex alternation vs `shortname_for`) → one (§4.1.2); two resolution paths in `cite_audit` (per-citation `lookup_section` vs nothing) → one index (§4.1.6). One is deliberately **not** collapsed and is slotted instead: `agent_brief.py` remains a second whole-repo `§` scanner (§11).

---

## §8 Line-count budget

Verified 2026-07-28 (`wc -l`):

| File | Now | After (est.) | Note |
|---|---|---|---|
| `.claude/tools/_webref/commands/cite_audit.py` | 289 | ~330 | comment scanner + probe in, `_LABEL_ALT` + `_DANGLING_LABEL_RE` + 9-arg emitters out |
| `.claude/tools/_webref/spec_labels.py` | 136 | ~200 | reverse index + discriminated `_catalog()` |
| `.claude/tools/_webref/test_cite_audit.py` | 410 | ~560 | T1-T9, C1; −1 test moved to `test_preflight.py` |
| `.claude/tools/_webref/test_spec_labels.py` | — | ~110 | new (S1-S5) |
| `.claude/skills/elidex-plan-review/preflight.py` | A's landed size | +~10 | §4.6.3 shared grammar only — the fail-closed work is A's |
| `.claude/skills/elidex-plan-review/test_preflight.py` | A's landed size | +~30 | P4/P5 appended to A's file |
| `.claude/tools/_webref/census_underreport.py` | — | ~45 | new (§4.0) |
| `.claude/tools/_webref/resolver.py` | 280 | ~300 | heading index |
| `.claude/tools/_webref/section_sort.py` | 48 | ~55 | shared `SECTION_NUMBER_RE` |

**1000-line touch-time check** (CLAUDE.md, cohesion-based not count-based): no file in the touch set is within 400 lines of 1000. The largest, `test_cite_audit.py` at 410 → ~560, is the one to watch — it is already organised by defect class, so the seam (one module per invariant: token / attribution / comment-spans / gate) is pre-drawn if a later PR pushes it past ~800. Not split now: at ~560 a split would fragment a suite whose value is that one file states the detector's whole contract.

---

## §9 Edge-dense assessment

The pre-slice memo argued this shape was one PR on its own merits, and conceded in the same breath that
the trigger fires. That argument is **withdrawn** — it was the second recurrence of the same mistake, and
the umbrella is the answer to it.

Under the umbrella the **base case** applies: an approved umbrella's narrowly-scoped, plan-reviewed
per-PR slice is a terminal unit, and is not re-split for touching the same subsystem as A/C/D. B is not
re-split further.

What remains worth stating, because it is what makes B *safe* as one slice rather than merely permitted:
I1-I4 are four facets of one function, `_attribute(lines) -> [(lineno, section, spec)]`, with one output
schema and one test file; there is no cross-crate, cross-thread or cross-process invariant, no ownership
transfer, no ECS component, and no spec algorithm being implemented. `git diff --stat -- crates/` is
empty and stays empty, so a regression degrades a developer tool and cannot reach a page, a script or a
user. §5's delta table is the proof of intersection and also the complete edge matrix.

The ordering coupling that the pre-slice memo used to argue *against* splitting is now the umbrella's
ordering rule instead of an exemption: retiring the grep requirement before the detector is sound would
mandate an under-reporting detector (so C follows B), and the regression pins are unenforced until a
scheduler exists (so A precedes B).

## §10 Open questions for `/elidex-plan-review`

- **Q1 — is "nine classes" an audit or a sample?** Nine is what execution found; §4.7 marks exhaustion UNCHECKED. The mitigation is structural (every declined input becomes a *counted class*, so a tenth cause shows up as a number rather than as silence). Review should decide whether that is sufficient or whether a differential fuzz — generate comment lines, assert the detector's bucket against an independent reference implementation — belongs in this PR. **Recommendation: sufficient**, and slot the fuzz if review disagrees; a second implementation to check the first is a different kind of work.
- **Q2 — what caps the probe window (`MAX_LABEL_WORDS`)?** Measured: the catalog's longest label is **24 words** (an RFC title ending `| RFC Editor`); **169 of 1896** label keys exceed 6 words. Probing 24 tokens on every comment line is the cost profile §4.6.1 is deleting. **Recommendation: cap at 6, derived and asserted** — `assert MAX_LABEL_WORDS >= max(len(k.split()) for k in PINNED)` so the pinned set can never outgrow the cap silently, with the 169 measured exclusions stated in a comment. Review should confirm 6 rather than, say, 8.
- **Q3 — should `resolver.lookup_heading` itself use the shared index, or only `cite_audit`?** Using it everywhere makes `coverage-map` and `preflight` faster and is the one-issue-one-way answer; it also changes a generic-core function's memory profile (one index per spec per process). **Recommendation: yes, in `resolver`**, since `try_fetch_data` is already `@lru_cache`'d per process, so the extract is retained regardless — the index is strictly cheaper than re-scanning it.
- **Q4 — is `--glob` on non-Rust files supportable at all?** §4.1.4's comment scanner is Rust-specific. Measured, `census_underreport.py docs '*.md'` reports **5674** "cites on a non-comment line" — i.e. under a comment-gated extractor, a Markdown run would attribute nothing. Three options: (a) keep `--glob` and document it as Rust-only, failing loudly on a non-`.rs` glob; (b) add a `--syntax {rust,markdown,none}` selector; (c) make "no comment syntax" mean "the whole file is prose", which is the right model for `docs/**/*.md`. **Recommendation: (c) with an explicit `--syntax` override defaulting from the glob** — the `docs/` tree is a real audit target (69 truncated tokens there too, §4.0), and (a) would foreclose it.
- **Q6 — should attribution widening be opt-in?** §5 shows `cite-audit html` losing 36 cites to more specific catalog specs. That is more correct and it moves a number a sweep's exit criterion reads. Options: widen unconditionally (recommended — a sweep should not audit another spec's citations); or gate behind `--catalog-labels` and default off, keeping today's numbers stable. **Recommendation: widen unconditionally and state it in the PR description**, since the whole point is that the pinned 12 are not the citable universe. In-flight sweeps re-baseline; there is exactly one (PR-A0), and it is on a branch that has to rebase anyway (§13).
- **Q7 — should `--strict`'s new gates be one flag or three?** Gating unresolved + unattributed + rejected + skipped under one `--strict` is simplest and hardest to under-use. But it makes `--strict` unusable tree-wide until a full sweep lands, which may push authors to drop the flag entirely. Alternative: `--strict` keeps its meaning, and `--strict-attribution` / `--strict-tokens` add the new classes. **Recommendation: one flag.** A gate with three opt-in halves is a gate three ways to under-use; scoping is what `--root`/`--prefix` are for. Review should push back if the migration cost looks real.
- **Q8 — `SKIPPED` on `--strict`: fail, or warn?** §4.1.9 is honestly low-realism. Failing `--strict` on an unreadable file is the fail-closed rule; warning avoids a gate that a `chmod` accident can red. **Recommendation: fail**, consistent with **Slice A's** rule that a tool which cannot see everything must not report success.

---

## §11 Defer slots + per-PR ≤3 audit

**Two own deferrals** against a budget of ≤3 ([[feedback_defer_cap_policy]]). Both are non-spec tooling cleanups, so they are named `cleanup-webref-*` rather than `#11-*` — but they are **counted against the cap anyway**, because the discipline is restraint, not accounting. A third, `cleanup-webref-preflight-inprocess-resolution`, went to **Slice A** with the file it concerns; B must not re-register it.

| Slot | 4-question audit |
|---|---|
| **`cleanup-webref-agent-brief-attribution`** (NEW) | `commands/agent_brief.py:131` `_needles_for_entry` builds `f"§{value}"` needles and substring-matches with **no spec attribution at all** — a second whole-repo `§` scanner carrying exactly the defect class `cite_audit.py:9-23` and `DESIGN.md:82-87` name. Also `_read_text` sits *inside* the `for entry in entries` loop with no cache, so all candidate files are re-read once per changed entry. (1) Real gap? Yes, and it is CLAUDE.md "One issue, one way": every attribution fix in this PR must be re-derived for scanner #2 or silently is not. (2) Blocked by structure? Yes — collapsing them changes `agent-brief`'s output contract (its consumers expect impact *paths*, not attributed sections), so it is a redesign of a different command. (3) Non-regressing to defer? Yes — `agent-brief` is advisory ("findings still need agent judgment", `DESIGN.md:90-92`) and gates nothing. (4) Durable home? `project_open-defer-slots.md`. **Trigger**: the next `agent-brief` change, or a §-renumber snapshot large enough that the re-read cost bites. **Re-eval**: 2026-11-30. |
| **`cleanup-webref-audited-set-provenance`** (NEW) | `cite_audit._iter_cites` (`:150-163`) hand-rolls a tree walk that duplicates `agent_brief._candidate_files` + `_read_text` + `_within`, and the two disagree: `agent_brief` enforces root containment but has no `target` skip; `cite_audit` skips `target` but has no containment check. Neither consults `.gitignore`, so `--root .` walks `.git/` and any `output-*/`. `git ls-files -z -- <root>` is the right primitive. (1) Real gap? Yes — "which files are in the audited set" is a `--strict` gate's denominator, and today it is a name heuristic. (2) Blocked by structure? No, but **changing it changes every count in this memo**, which would make §5's before/after unreadable and this PR's own delta unverifiable. That is the reason to defer, and it expires the moment this PR lands. (3) Non-regressing? Yes — the default `--root crates --glob '*.rs'` is unaffected today (`target` is the only excluded directory that matters). (4) Durable home? `project_open-defer-slots.md`. **Trigger**: immediately after this PR lands, or the first `--root .` run. **Re-eval**: 2026-09-30. |

**Explicitly NOT deferred**, recorded so the absence is deliberate: the nine under-report paths (D1 — locked in scope), the emitter signature (§4.6.2), the three §-number grammars (§4.6.3), and all three efficiency findings (§4.6.1 — they are the same edit as §4.1.2 and §4.1.6, not extra scope). The four items the pre-slice memo also listed here (D2-D5) are not deferrals; they are **Slice A's and C's scope** (§4.2), and B ships without them because they land first, not because they were declined.

**Pre-existing defects surfaced but not owned by this PR** (a separate category from own-deferrals, per [[feedback_defer_cap_policy]]): the `webref_data.py` `@lru_cache` docstring/`--help` disagreement (`cache.py:46-47` and `cli.py:64-67` still say `ELIDEX_WEBREF_NO_CACHE=1` "bypass[es] entirely", which the new memo layer defeats). One-line doc fix, folded in with §4.1.6 since it touches the same file and the same claim.

---

## §12 Exit criterion

A runnable pair, plus one property only B can establish. None depends on any count in this memo.

**(1) Green — the enforcement suite passes, from the task Slice A wired into `[tasks.ci].depends`:**

```sh
mise run tools-test
```

B does not have to argue this command is trustworthy — that argument is A's, and it is A's exit
criterion. B inherits it.

**(2) Red — every new pin actually detects the defect it names:**

```sh
git worktree add /tmp/citeaudit-pre <A's landed head>
cp .claude/tools/_webref/test_*.py /tmp/citeaudit-pre/.claude/tools/_webref/
cd /tmp/citeaudit-pre && mise run tools-test; echo "EXPECT NON-ZERO: $?"
```

The new tests run against the **unfixed** detector at A's head. This must exit non-zero, with at least
one failure attributable to each of the nine classes (T1-T9) and to the coverage gap (C1). A test that
passes here pins nothing — the failure mode `test_prefix_tolerant_resolver_is_pinned_to_an_exact_match`
already demonstrates in-tree (§6).

**(3) The census is re-derivable from the tool, not from a script beside it.** After B, the three counts
`census_underreport.py` computes (§4.0) are reported classes of `cite-audit` itself
(`REJECTED-TOKEN` / `UNKNOWN-SPEC` / `SKIPPED`). The check is that the harness and the tool agree, and
then that the harness is no longer needed to answer the question — which is what makes B's output usable
as **Slice C's reach measurement** and **Slice D's baseline**.

---

## §13 Coordination

⚠ Every figure in this section is from **2026-07-28** and is a snapshot; re-derive at B's kickoff from
`git worktree list` and `gh pr list --state open`, not from this table.

| Lane | Overlap with B | Ordering rule |
|---|---|---|
| **Slice A** | total — B branches from A's landed head, edits `preflight.py` for §4.6.3 only, and appends to the `test_preflight.py` A creates | **A first** (umbrella) |
| **Slice C** | C's supersession claim is admissible only once B has measured the detector's reach; B's landing note hands C the shipped reported-class names | **B before C** |
| **PR-A0 / Slice D** (`domform-submittable-category`) | carries the identical 8 `.claude/` files by construction; must drop its `.claude/` half, and must re-baseline its §4 counts against §5's widened attribution | **B before D** — D's exit criterion is a command B makes trustworthy |
| **In-flight plan-memos** | §4.1.8 re-points spec labels to their current level (`CSSOM`→`cssom-1`, `Selectors`→`selectors-4`, `Pointer Events`→`pointerevents4`); memos in `elidex-wt-c3-plan` cite those labels | B's landing checklist re-verifies them and records the result |
| **VM P4** (`elidex-wt-vmp4`) / **Layout lane** (`elidex-wt-c4fix`) | `mise.toml` / `ci.yml` — **A's collision surface, not B's**; B touches neither | none |

**Carried edits (ship with B):**

1. **`project_open-defer-slots.md`** — register the 2 `cleanup-webref-*` slots (§11). A memory-file write,
   not a chip ([[reference_spawn-task-chips-not-durable]]).
2. **`.claude/tools/_webref/DESIGN.md`** — B ships the reported classes; the *contract paragraph* that
   points authors at them is **C's** edit. B's landing note names the shipped class names so C does not
   guess them.
3. **PR description** — must state the §5 attribution widening explicitly (Q6): it silently changes the
   number an in-flight sweep measures against, and Slice D is that sweep.
