# Plan — make the `cite-audit` detector sound, make the plan-review gate fail closed, retire the method they supersede

## §0 Status

**Branch**: `webref-cite-audit-tool`. **Worktree**: `/Users/kazuaki/repos/send.sh/elidex-wt-citeaudit`.
**Base**: `6b33854d`. **Head**: `bf580047` (the carve). ⚠ `origin/main` has since moved to `96a8e47b` (#490, #488) — rebase before implementation (§13).
**Nature**: a **developer-tooling** PR. Zero `crates/**` diff, zero engine behavior change. What changes is what an enforcement tool *reports* and when a gate *fails*.
**Status**: plan-memo. `/elidex-plan-review` **required before implementation** (§9 argues this is not edge-dense enough to need further splitting, but the plan-review itself is not optional — it is the rule's remedy, and the carve commit's own message names a pending plan-review as the reason the split was mandatory).

### §0.1 What `bf580047` already did, and what is left

`bf580047` is a **provenance-preserving carve**, not an implementation: it moved `commands/cite_audit.py`, `spec_labels.py`, the `coverage_map.py` / `webref_data.py` / `cli.py` / `DESIGN.md` edits and the `.claude/skills/elidex-plan-review/preflight.py` change out of PR-A0 (`docs/plans/2026-07-constraint-validation-citation-sweep.md`, branch `domform-submittable-category`) onto this branch **unchanged**:

```sh
git diff domform-submittable-category -- .claude/          # → empty
```

The carve was forced, not chosen. CLAUDE.md "Edge-dense work = multi-PR program" binds: the tooling is general-purpose, the dependency is one-directional (the sweep needs a detector; the detector does not need the sweep), and the base case that would exempt a single PR requires a passed `/elidex-plan-review` that the sweep memo still lists as pending.

This memo covers the remainder: **the detector under-reports on nine measured paths, the gate it feeds fails open, and the method it supersedes is still mandated in `axes.md`.** A detector that silently under-reports cannot carry a sweep's exit criterion. Fixing that is the whole content of this PR.

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
3. A failure of the tool's *own* infrastructure (a corrupt cache, an unreachable catalog, an unimportable module) must be reported as itself, never as a property of the citations being audited (§4.1.6, §4.1.7, §4.2). This is the sharpest one: today a truncated cache file makes the tool blame the author's citations, and an unimportable `_webref` makes the plan-review gate report success having verified nothing.

The same rule applies to this memo. §4.7 states, per claim, what mechanically checks it, and marks the rest UNCHECKED.

---

## §2 Coupled invariants

Four invariants intersect in `_attribute` and its consumers. They are listed here because the fixes cannot be applied one at a time without transiently breaking another.

- **I1 — token integrity.** The section token the detector reports must be the *whole* token the author wrote, or nothing. Today a suffixed token backtracks to a resolvable prefix (§4.1.1). Fixing I1 alone changes total cite counts, which is why §5 measures it.
- **I2 — attribution reach.** The set of labels the detector can recognise must equal the set `spec_labels.shortname_for` can resolve. Today the regex alternation is built from the 12 pinned `SPECS` while `shortname_for` reaches a 948-entry catalog (§4.1.2). Fixing I2 requires an *enumerable* label set, which `shortname_for` alone cannot supply — §4.1.2 solves that rather than papering over it.
- **I3 — label boundaries.** A label must match on token boundaries, not as a suffix of an identifier (§4.1.3). I3 is structurally guaranteed by I2's fix (whitespace-delimited probing) rather than patched separately — that is why they land together.
- **I4 — text classification.** A `§` is a citation only where a citation can live. Today extraction is never gated on `in_comment`, and `_COMMENT_RE` misclassifies both directions (§4.1.4). I4 changes which lines *end* an attribution block, so it moves cites between buckets and must be measured with I1-I3, not after them.

**Gate-side**, one further invariant: **I5 — a gate's exit code must distinguish "verified nothing" from "verified everything"** (§4.2). It is independent of I1-I4 and could ship separately; §9 argues why it should not.

---

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21 Constraints | resolution | a **resolvable prefix** of a suffixed token — the silent-pass vector | §4.1.1 — `_CITE_RE` atomic token; regression fixture in `test_cite_audit.py` | ✓ — every truncated token in the tree enumerated by the §4.0 census | no |
| WHATWG HTML §4.10.21.2 Constraint validation | resolution | the token the author actually wrote | §4.1.1 — after the fix this is the reported section, or the whole token is rejected and counted | ✓ | no |
| WHATWG HTML §16.2 Non-conforming features | resolution | word-suffixed token → phantom `§16` UNRESOLVED | §4.1.1 — `crates/css/elidex-style/src/ua.rs:68` | ✓ | no |
| WHATWG HTML §13.5 Named character references | attribution | a `§` inside a Rust **string literal** counted as a citation | §4.1.4 — `crates/dom/elidex-html-parser-strict/src/tokenizer/build_entities.rs:68` | ✓ for `crates/**/*.rs`; ✗ for non-Rust globs (§10-Q4) | no |
| WHATWG HTML §2.1.4 DOM trees | attribution | bucket (b) INHERITED — the largest such cluster in the tree | §4.1.3/§4.1.4 — block-boundary changes move cites in and out of this bucket | ✓ | no |
| WHATWG XHR §4 Interface FormData | per-spec coverage | 16 phantom `§4.3` cites invisible to a `spec=html` run | §4.3 — the per-spec run requirement added to `axes.md` | ✓ — the census enumerates all 10 attributed specs | no |
| CSS Text 3 §4.1.3 Segment Break Transformation Rules | attribution | a catalog-only label the alternation cannot see | §4.1.2 — `shortname_for` resolves it; `_LABEL_ALT` does not | ✓ under the §4.1.2 index rule | no |

**Breadth**: K=3 specs (`html`, `xhr`, `css-text-3`), M=7 rows → preflight verdict **ok (single PR scope)**.

**Why the breadth is small and honest**: this PR implements no spec algorithm. The rows are fixtures, not obligations. A larger table would be padding — CLAUDE.md's "Supported-surface testing" asks what guards the surface, and here the guard is the regression suite, not spec breadth.

### §3.1 User-input touch audit + the discovery method

**No user-input flow.** Nothing here is reachable from page content, script, or network. The tool's inputs are a developer-supplied `--root`/`--glob` and repository text. The one adversarial-ish input is a **corrupt HTTP cache file** under `$XDG_CACHE_HOME/elidex-webref/` — already treated as a trust boundary by `cache.py:70-81` for `.meta`, and extended to the body in §4.1.6.

**Discovery method** — this is the section `axes.md:179` currently governs, and the one §4.3 rewrites. The candidate set for this PR was derived, not authored:

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

**Fix**: `--strict` gates on unresolved **and** unattributed **and** the new rejected-token class. A sweep scopes with `--root` / `--prefix` to the set it actually swept; a tree-wide strict run is expected to fail today and that is the correct signal. §4.3(a) makes dispositioning the bucket a documented requirement rather than a hope.

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

**Fix**: `_catalog()` returns a discriminated result — *available* (a possibly-empty dict) vs *unavailable* (with the cause). Callers act on the distinction: `cite-audit` puts a label it cannot resolve because the catalog is unreachable into a distinct `UNKNOWN-SPEC` class rather than silently into UNATTRIBUTED; `preflight` treats it as §4.2 treats an unimportable tools tree — **not survivable without `--no-verify`**. `SystemExit` is caught explicitly alongside `Exception`.

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

### §4.2 D2 — `preflight.py` must fail CLOSED

`preflight.py:56-60` and `:232-237`. The import guard sets `_shortname_for = None` on any failure, and `shortname_from_label` then returns `None` for **every** row — which the row loop (`:353-358`) classifies as *unmapped*, a documented soft-warn. So `citations` stays empty, the verify loop never runs, and the gate **exits 0 having verified nothing**.

The in-code comment claims it "degrade[s] the same way the pre-existing `WEBREF.is_file()` check does". Measured, the two behave oppositely:

```sh
# sandbox: a repo skeleton with the skill + tools trees, so REPO_ROOT resolves
SB=/tmp/sb; rm -rf $SB; mkdir -p $SB/.claude/skills $SB/.claude/tools
cp -R .claude/skills/elidex-plan-review $SB/.claude/skills/
cp -R .claude/tools/_webref $SB/.claude/tools/; cp .claude/tools/webref $SB/.claude/tools/
M=../elidex-wt-submittable/docs/plans/2026-07-form-submittable-category-repair.md
python3 $SB/.claude/skills/elidex-plan-review/preflight.py $M --no-grep-pass; echo "A EXIT=$?"
mv $SB/.claude/tools/webref $SB/.claude/tools/webref.bak
python3 $SB/.claude/skills/elidex-plan-review/preflight.py $M --no-grep-pass; echo "B EXIT=$?"
mv $SB/.claude/tools/webref.bak $SB/.claude/tools/webref
mv $SB/.claude/tools/_webref $SB/.claude/tools/_webref.bak
python3 $SB/.claude/skills/elidex-plan-review/preflight.py $M --no-grep-pass; echo "C EXIT=$?"
```

| Case | Result |
|---|---|
| **A** both present | 21 rows, 21 parsed citations, **15 unique citations verified**, EXIT 0 |
| **B** `webref` CLI missing (pre-existing check) | `❌ HARD FAIL — citation verification: 15 failure(s)`, **EXIT 1** |
| **C** `_webref` unimportable (the new import) | `parsed citations: 0`, `unmapped-label rows: 21`, no verify section at all, **EXIT 0** |

⚠ The brief said "21/21" for case B. The memo has 21 data rows but `seen_pairs` (`preflight.py:382-388`) dedups to **15 unique citations**, so the correct figure is **15 of 15**. The asymmetry it describes is exactly as stated.

Case C's stderr also carries a **wrong-cause remedy**: `(add the spec to .claude/tools/_webref/spec_labels.py::SPECS)` — the file that failed to import. An author following it edits a file the gate cannot read.

**Fix**, three parts:

1. **Distinguish the two causes.** `shortname_from_label` returns a tri-state: mapped / genuinely-unmapped / **tools-unavailable**. The last propagates a `TOOLS_UNAVAILABLE` condition to `main`.
2. **Fail closed on it.** `TOOLS_UNAVAILABLE` is a HARD FAIL under the same rule as `WEBREF.is_file()`, suppressed only by `--no-verify`. A structural (`--no-verify --no-grep-pass`) run still works, which is the only degradation the comment was right to want.
3. **Fix the remedy line.** Unmapped label → "add the spec to `spec_labels.py::SPECS`, or check the label spelling". Tools unavailable → name the import error and the path, and point at `--no-verify`.

This also inherits §4.1.7: with a discriminated `_catalog()`, "the catalog is unreachable" reaches `preflight` as its own condition rather than as 21 unmapped rows.

### §4.3 D3 — retire the superseded method in this PR

CLAUDE.md "One issue, one way": a better mechanism must **replace** the old one, not coexist with it. Today the detector and the method it supersedes coexist, and only the old one is mandated:

```sh
grep -rn 'cite-audit' .claude/skills/ CLAUDE.md   # → no matches (exit 1)
```

- `.claude/skills/elidex-review/axes.md:179` still MIN-flags a citation-sweep plan-memo that does not document "**≥4 grep pattern**" of hand-authored discovery alternations — precisely failure mode #2 in `cite_audit.py:13` ("enumeration-only-by-known-pattern"). A plan-review agent applying Axis 4 to *this memo* would MIN-flag it for not doing the thing the tool retires.
- `CLAUDE.md` §"Spec citation" (`:39-51`) documents `heading` / `dfn` / `aoid` / `body` / `css` / `specs` and never mentions `cite-audit`.

Both files are git-tracked and editable in-branch. Note `axes.md:172` is **not** superseded — it checks author-written number↔title pairs, which `cite-audit` never compares. Complementary, not duplicated; it stays.

**Edit 1 — `axes.md:179`.** Replace requirement (2) "**≥4 grep pattern**" with the detector, and add the two requirements the detector's own blind spots imply:

> (2) **discovery = `cite-audit`, not a grep alternation** — `.claude/tools/webref cite-audit <spec> --root <tree> [--prefix N]`, output pasted or summarised in the memo;
> (2a) **attribution coverage** — the UNATTRIBUTED / UNKNOWN-SPEC / REJECTED-TOKEN counts must be **dispositioned**, not merely reported: each is either in-scope-and-repaired, out-of-scope-with-a-reason, or slotted. A memo that prints the number and moves on has not audited it.
> (2b) **one run per cited spec** — `cite-audit <shortname>` for **every** spec the touched files cite, not only `html`.

(2b) is load-bearing, not belt-and-braces. PR-A0 ran only `spec=html`. Measured on the same tree:

```sh
.claude/tools/webref cite-audit xhr --summary
```
→ `§4.3 [16 cite(s)] *** UNRESOLVED ***` and `§4.3.6 [1 cite(s)] *** UNRESOLVED ***`. XHR has no `§4.3`; §4 is *Interface FormData* with no subsections. **17 phantom cites, invisible to a `spec=html` run**, including a module header. The tree attributes to **10** distinct specs today (§4.0), so a single-spec run sees at most one tenth of the attributed surface.

**Edit 2 — `CLAUDE.md` §"Spec citation".** Add one paragraph: `cite-audit` is the discovery instrument for citation-sweep work; the per-spec and attribution-coverage requirements; and the fact that its `--strict` exit code is a gate, not a report.

### §4.4 D4 — assert the consumer's derivation from the consumer side

`test_cite_audit.py:289-296` does `sys.path.insert(... "skills"/"elidex-plan-review")` + `importlib.import_module("preflight")` inside a test method. The *tools* package's test therefore hard-codes the *consumer skill's* directory layout and module name — the one edge that would block `DESIGN.md`'s stated goal ("keep its drift-detection core generic enough to move to a standalone repository later"; "Keep new generic behavior free of elidex-specific file paths"). Extracting `_webref` would take a test that imports `preflight` with it, or drop the assertion.

Two further defects at the same site: the `sys.path.insert` is **never undone** (it runs on every invocation of the method and leaks into the rest of the process), and `coverage_map_label` (`:313-316`) re-does an `importlib.import_module` already performed at `:295`.

**Fix**: move the `preflight.shortname_from_label(label) == short` assertion into a new `.claude/skills/elidex-plan-review/test_preflight.py`, beside `preflight.py` — the same directory that already holds `test_grep_pass.py`, so the home exists and the dependency direction is right (consumer depends on library, not the reverse). `test_cite_audit.py` keeps the `coverage_map` half, imported at module top level like `spec_labels` at `:27`, with no `sys.path` mutation inside a test.

`test_preflight.py` is also the natural home for D2's regression: assert that an unimportable `_webref` produces a non-zero exit (§6-P2).

### §4.5 D5 — wire the `_webref` suites into CI

Nothing runs them today. Measured:

```sh
for f in .claude/tools/_webref/test_*.py; do
  echo -n "$f: "; python3 -m unittest discover -s .claude/tools/_webref -p "$(basename $f)" -t .claude/tools 2>&1 | grep -E '^Ran '
done
python3 .claude/skills/elidex-plan-review/test_grep_pass.py 2>&1 | tail -3
grep -n 'depends' mise.toml | grep ci
grep -n 'claude' .github/workflows/ci.yml   # → no matches
```

| Suite | Tests |
|---|---|
| `.claude/tools/_webref/test_cite_audit.py` | **36** |
| `.claude/tools/_webref/test_inventory_diff.py` | **6** |
| `.claude/tools/_webref/test_agent_brief.py` | **5** |
| `.claude/tools/_webref/test_refresh.py` | **1** |
| `.claude/skills/elidex-plan-review/test_grep_pass.py` | **35** |
| **total** (verified 2026-07-28, commands above) | **83, across 5 files** |

`mise.toml` `[tasks.ci].depends` = `check lint test-all doc deny trip-wires ci-sweep` — all cargo, plus four `.sh` trip-wires (`layout-box-reader-trip-wire.sh` landed in #488). No `.pre-commit-config.yaml`, no `.githooks`, no Python anywhere. So `test_cite_audit.py` (410 lines) and every regression pin this PR adds for defects 1-9 are unenforced. The PR's own thesis is that a claim is admissible only if something mechanically checks it; an unhooked enforcement suite is that same defect one level up.

**Edit 1 — a `mise` task, folded into `ci`** (the locked half):

```toml
[tasks.tools-test]
description = "Run the Python suites for .claude tooling (webref + plan-review gate)."
run = """
python3 -m unittest discover -s .claude/tools/_webref -p 'test_*.py' -t .claude/tools
python3 -m unittest discover -s .claude/skills/elidex-plan-review -p 'test_*.py'
"""
```
with `tools-test` added to `[tasks.ci].depends`.

**Edit 2 — the GitHub half.** ⚠ The locked decision is under-specified here, and the gap is exactly this PR's shape. Measured:

- `.github/workflows/ci.yml` **never invokes `mise`** — the `check` job runs `cargo fmt` / `clippy` / `nextest` / doc-tests directly. Adding a `mise` task therefore covers the local pre-push gate only. (`trip-wires` has the same property today; that is a precedent, not a justification.)
- The `changes` path filter's `rust` set is `crates/** | Cargo.toml | Cargo.lock | rust-toolchain.toml | .rustfmt.toml | clippy.toml | mise.toml | .github/workflows/**`. **`.claude/**` is not in it.** This PR touches `mise.toml`, so it happens to trigger `check` — but the *next* `.claude/**`-only PR (a one-line `cite_audit.py` fix) would trigger nothing at all.

So Edit 1 alone leaves the claim "the regression pins are mechanically checked" false for the file class this PR exists to protect. Edit 2 closes it: a `tools` filter (`.claude/tools/**`, `.claude/skills/**`, `.github/workflows/**`) and a small `tools` job that runs the two `unittest discover` lines on `ubuntu-latest`. It is ~20 lines of YAML and no new dependency (`python3` is preinstalled on GitHub runners). §10-Q5 asks review to confirm the CI-topology change rather than assume it.

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

**Fix**: `section_sort.py` — already the established home for section-number syntax and already shared by resolver / aoid / heading / inventory — exports one `SECTION_NUMBER_RE`. `cite_audit` and `preflight` both import it. `resolver.py:211`'s discriminator is a *routing* predicate (number vs AO name), not a token grammar, and stays.

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
| `preflight` fails closed when `_webref` is unimportable | `test_preflight.py::test_tools_unavailable_is_hard_fail` (D2 case C) |
| `preflight` still exits 0 under `--no-verify --no-grep-pass` | same file — pins the one degradation that must survive |
| Consumers derive from `SPECS` | `test_preflight.py` (preflight half) + `test_cite_audit.py` (coverage_map half) — D4 |
| The suites run at all | `mise run ci` → `tools-test`; GitHub `tools` job (§4.5 Edit 2) |
| `axes.md` / `CLAUDE.md` name the detector | `grep -q 'cite-audit' .claude/skills/elidex-review/axes.md CLAUDE.md` — **UNCHECKED by a test**; a doc assertion, verified by the exit criterion's grep, not pinned against future edits |
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
3. **Attributed specs go 10 → 65.** UNATTRIBUTED is still 5733, and that is correct: the residue is genuinely bare `§N.N` in comment blocks that never name a spec, plus plan-memo pointers (`docs/plans/….md §6`). §4.3(a) makes dispositioning it a documented requirement.

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

**`test_preflight.py`** (new, §4.4):
- **P1** the derivation assertion moved from `test_cite_audit.py:302`.
- **P2** `_webref` unimportable → **exit 1** (D2 case C, inverted).
- **P3** `--no-verify --no-grep-pass` still exits 0 with the tools tree absent — the one degradation that must survive.
- **P4** catalog unavailable → hard fail, and the remedy line does **not** say "add the spec to `spec_labels.py::SPECS`".
- **P5** `parse_spec_cell` on `§Deferred` / `§C1` yields no citation (shared `SECTION_NUMBER_RE`, §4.6.3).

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
| §4.2 gate policy | **elidex skill** | `preflight.py`; consumes the library, adds no generic behavior |
| §4.3 doc edits | **elidex** | `axes.md`, `CLAUDE.md` |

One honest exception: §4.4 moves an assertion **out** of the generic package's tests into the elidex skill's tests, which is a layering *improvement* — it removes the generic core's only hard-coded dependency on an elidex skill's directory layout, the exact thing `DESIGN.md:156-157` asks for.

**One-issue-one-way**, three collapses in this PR: three §-number grammars → one (§4.6.3); two label-recognition mechanisms (regex alternation vs `shortname_for`) → one (§4.1.2); two resolution paths in `cite_audit` (per-citation `lookup_section` vs nothing) → one index (§4.1.6). One is deliberately **not** collapsed and is slotted instead: `agent_brief.py` remains a second whole-repo `§` scanner (§11 D-1).

---

## §8 Line-count budget

Verified 2026-07-28 (`wc -l`):

| File | Now | After (est.) | Note |
|---|---|---|---|
| `.claude/tools/_webref/commands/cite_audit.py` | 289 | ~330 | comment scanner + probe in, `_LABEL_ALT` + `_DANGLING_LABEL_RE` + 9-arg emitters out |
| `.claude/tools/_webref/spec_labels.py` | 136 | ~200 | reverse index + discriminated `_catalog()` |
| `.claude/tools/_webref/test_cite_audit.py` | 410 | ~560 | T1-T9, C1; −1 test moved to `test_preflight.py` |
| `.claude/tools/_webref/test_spec_labels.py` | — | ~110 | new (S1-S5) |
| `.claude/skills/elidex-plan-review/preflight.py` | 489 | ~510 | tri-state + fail-closed + remedy text |
| `.claude/skills/elidex-plan-review/test_preflight.py` | — | ~120 | new (P1-P5) |
| `.claude/tools/_webref/census_underreport.py` | — | ~45 | new (§4.0) |
| `.claude/tools/_webref/resolver.py` | 280 | ~300 | heading index |
| `.claude/tools/_webref/section_sort.py` | 48 | ~55 | shared `SECTION_NUMBER_RE` |
| `.claude/skills/elidex-review/axes.md` | 227 | ~232 | §4.3 Edit 1 |
| `CLAUDE.md` | 92 | ~94 | §4.3 Edit 2 |
| `mise.toml` | 131 | ~140 | `tools-test` |
| `.github/workflows/ci.yml` | 126 | ~150 | `tools` filter + job |

**1000-line touch-time check** (CLAUDE.md, cohesion-based not count-based): no file in the touch set is within 400 lines of 1000. The largest, `test_cite_audit.py` at 410 → ~560, is the one to watch — it is already organised by defect class, so the seam (one module per invariant: token / attribution / comment-spans / gate) is pre-drawn if a later PR pushes it past ~800. Not split now: at ~560 a split would fragment a suite whose value is that one file states the detector's whole contract.

---

## §9 Edge-dense assessment

CLAUDE.md's trigger: work is edge-dense when it binds **≥3 intersecting invariant axes** *or* touches a subsystem with **no canonical algorithm**. Applying it honestly rather than asserting a verdict:

**The trigger arguably fires.** §2 lists five invariants (I1-I5), and I1-I4 genuinely intersect: I4 changes which lines end an attribution block, which changes what I3's carry can attach to, which changes what I2 can resolve, over tokens I1 decides exist. The §5 delta table is the proof of intersection — no single fix's effect is readable in isolation. And there is no canonical algorithm for "what is a citation in a comment"; §4.1.4's scanner is a design, not a transcription.

**Three properties nevertheless argue against splitting further:**

1. **One subsystem, one function, one output schema.** Every one of I1-I4 is a facet of `_attribute(lines) -> [(lineno, section, spec)]`. There is no cross-crate, cross-thread, or cross-process invariant; no ownership transfer; no ECS component; no spec algorithm being implemented. The edge matrix is not merely enumerable — it is enumerable *in one file's test suite*, which is the property the edge-dense rule exists to protect. Contrast #339 (the incident the rule comes from), where the edges were spread across subsystems and only discoverable in review.
2. **Zero engine blast radius.** `git diff --stat -- crates/` is empty and stays empty. A regression here degrades a developer tool; it cannot reach a page, a script, or a user.
3. **The pieces have a hard ordering coupling, so splitting makes things worse, not better.** §4.3 retires the "≥4 grep pattern" requirement. Landing that *before* §4.1 would mandate a detector that under-reports on nine measured paths as the sole discovery method — strictly worse than the status quo it replaces. If §4.1 lands first and §4.3 does not follow, both methods stay mandated at once — the coexistence CLAUDE.md's "One issue, one way" forbids. §4.5 is coupled the same way: without it, §4.1's regression pins are unenforced, which is the PR's own thesis inverted.

**And the base case does not apply**, so it is not being leaned on: there is no approved umbrella here. The single-PR shape rests on (1)-(3), not on the exemption.

**What genuinely was separable has been separated.** The carve itself (`bf580047`) is separation #1: the detector left a content sweep. Three further items are separated into slots rather than bundled (§11): the `agent_brief` second scanner, the file-set provenance change, and the in-process preflight collapse. Each was tempting — all three are ≤30 lines — and each was declined for a stated reason, not for budget.

**Verdict: one PR.** With one non-negotiable: `/elidex-plan-review` on this memo before implementation. That is the rule's actual remedy, and it is what the carve commit's message names as the reason the split was mandatory in the first place. PR-A0's asymmetry — the sibling memo recorded its gate, the sweep memo did not — is exactly the failure not to repeat.

---

## §10 Open questions for `/elidex-plan-review`

- **Q1 — is "nine classes" an audit or a sample?** Nine is what execution found; §4.7 marks exhaustion UNCHECKED. The mitigation is structural (every declined input becomes a *counted class*, so a tenth cause shows up as a number rather than as silence). Review should decide whether that is sufficient or whether a differential fuzz — generate comment lines, assert the detector's bucket against an independent reference implementation — belongs in this PR. **Recommendation: sufficient**, and slot the fuzz if review disagrees; a second implementation to check the first is a different kind of work.
- **Q2 — what caps the probe window (`MAX_LABEL_WORDS`)?** Measured: the catalog's longest label is **24 words** (an RFC title ending `| RFC Editor`); **169 of 1896** label keys exceed 6 words. Probing 24 tokens on every comment line is the cost profile §4.6.1 is deleting. **Recommendation: cap at 6, derived and asserted** — `assert MAX_LABEL_WORDS >= max(len(k.split()) for k in PINNED)` so the pinned set can never outgrow the cap silently, with the 169 measured exclusions stated in a comment. Review should confirm 6 rather than, say, 8.
- **Q3 — should `resolver.lookup_heading` itself use the shared index, or only `cite_audit`?** Using it everywhere makes `coverage-map` and `preflight` faster and is the one-issue-one-way answer; it also changes a generic-core function's memory profile (one index per spec per process). **Recommendation: yes, in `resolver`**, since `try_fetch_data` is already `@lru_cache`'d per process, so the extract is retained regardless — the index is strictly cheaper than re-scanning it.
- **Q4 — is `--glob` on non-Rust files supportable at all?** §4.1.4's comment scanner is Rust-specific. Measured, `census_underreport.py docs '*.md'` reports **5674** "cites on a non-comment line" — i.e. under a comment-gated extractor, a Markdown run would attribute nothing. Three options: (a) keep `--glob` and document it as Rust-only, failing loudly on a non-`.rs` glob; (b) add a `--syntax {rust,markdown,none}` selector; (c) make "no comment syntax" mean "the whole file is prose", which is the right model for `docs/**/*.md`. **Recommendation: (c) with an explicit `--syntax` override defaulting from the glob** — the `docs/` tree is a real audit target (69 truncated tokens there too, §4.0), and (a) would foreclose it.
- **Q5 — does the GitHub `tools` job belong in this PR?** §4.5 Edit 2 changes CI topology (a new path filter + a new job) for every future PR, which is more blast radius than anything else here. But without it the claim "the regression pins are checked" is false for `.claude/**`-only PRs — this PR's own file class. **Recommendation: include it**, because the alternative is shipping the exact defect D5 names. Review may split it; if so it must land immediately after, not into the slot ledger.
- **Q6 — should attribution widening be opt-in?** §5 shows `cite-audit html` losing 36 cites to more specific catalog specs. That is more correct and it moves a number a sweep's exit criterion reads. Options: widen unconditionally (recommended — a sweep should not audit another spec's citations); or gate behind `--catalog-labels` and default off, keeping today's numbers stable. **Recommendation: widen unconditionally and state it in the PR description**, since the whole point is that the pinned 12 are not the citable universe. In-flight sweeps re-baseline; there is exactly one (PR-A0), and it is on a branch that has to rebase anyway (§13).
- **Q7 — should `--strict`'s new gates be one flag or three?** Gating unresolved + unattributed + rejected + skipped under one `--strict` is simplest and hardest to under-use. But it makes `--strict` unusable tree-wide until a full sweep lands, which may push authors to drop the flag entirely. Alternative: `--strict` keeps its meaning, and `--strict-attribution` / `--strict-tokens` add the new classes. **Recommendation: one flag.** A gate with three opt-in halves is a gate three ways to under-use; scoping is what `--root`/`--prefix` are for. Review should push back if the migration cost looks real.
- **Q8 — `SKIPPED` on `--strict`: fail, or warn?** §4.1.9 is honestly low-realism. Failing `--strict` on an unreadable file is the fail-closed rule; warning avoids a gate that a `chmod` accident can red. **Recommendation: fail**, consistent with D2's rule that a tool which cannot see everything must not report success.

---

## §11 Defer slots + per-PR ≤3 audit

Three **own** deferrals; the per-PR budget is ≤3 own deferrals ([[feedback_defer_cap_policy]]). All three are non-spec tooling cleanups, so they are named `cleanup-webref-*` rather than `#11-*` — but they are **counted against the cap anyway**, because the discipline is restraint, not accounting.

| Slot | 4-question audit |
|---|---|
| **`cleanup-webref-agent-brief-attribution`** (NEW) | `commands/agent_brief.py:131` `_needles_for_entry` builds `f"§{value}"` needles and substring-matches with **no spec attribution at all** — a second whole-repo `§` scanner carrying exactly the defect class `cite_audit.py:9-23` and `DESIGN.md:82-87` name. Also `_read_text` sits *inside* the `for entry in entries` loop with no cache, so all candidate files are re-read once per changed entry. (1) Real gap? Yes, and it is CLAUDE.md "One issue, one way": every attribution fix in this PR must be re-derived for scanner #2 or silently is not. (2) Blocked by structure? Yes — collapsing them changes `agent-brief`'s output contract (its consumers expect impact *paths*, not attributed sections), so it is a redesign of a different command. (3) Non-regressing to defer? Yes — `agent-brief` is advisory ("findings still need agent judgment", `DESIGN.md:90-92`) and gates nothing. (4) Durable home? `project_open-defer-slots.md`. **Trigger**: the next `agent-brief` change, or a §-renumber snapshot large enough that the re-read cost bites. **Re-eval**: 2026-11-30. |
| **`cleanup-webref-audited-set-provenance`** (NEW) | `cite_audit._iter_cites` (`:150-163`) hand-rolls a tree walk that duplicates `agent_brief._candidate_files` + `_read_text` + `_within`, and the two disagree: `agent_brief` enforces root containment but has no `target` skip; `cite_audit` skips `target` but has no containment check. Neither consults `.gitignore`, so `--root .` walks `.git/` and any `output-*/`. `git ls-files -z -- <root>` is the right primitive. (1) Real gap? Yes — "which files are in the audited set" is a `--strict` gate's denominator, and today it is a name heuristic. (2) Blocked by structure? No, but **changing it changes every count in this memo**, which would make §5's before/after unreadable and this PR's own delta unverifiable. That is the reason to defer, and it expires the moment this PR lands. (3) Non-regressing? Yes — the default `--root crates --glob '*.rs'` is unaffected today (`target` is the only excluded directory that matters). (4) Durable home? `project_open-defer-slots.md`. **Trigger**: immediately after this PR lands, or the first `--root .` run. **Re-eval**: 2026-09-30. |
| **`cleanup-webref-preflight-inprocess-resolution`** (NEW) | `preflight.verify_citation` (`:254`) forks a Python subprocess **and** an HTTP conditional-GET per unique citation — measured **0.092 s per citation as a subprocess vs 0.0008 s in-process** (0.040 s cold), so a 20-citation §3 table pays ~1.8 s of pure overhead on a gate that runs on every plan review. The altitude cost is the same defect: this PR's carve added the `sys.path.insert` + import seam to reach the shared library in-process **for the label map**, then left citation resolution going out through the CLI — two ways to reach `resolver.lookup_section` in one file. (1) Real gap? Yes, one-issue-one-way. (2) Blocked by structure? **Yes, and this is the substantive reason**: in-process resolution means `cache.py`'s `sys.exit` on network failure aborts the *whole gate* mid-run instead of failing one citation. Whether a plan-review gate should be usable offline is a policy question §4.2 does not settle and §10 does not ask — it needs its own decision, and answering it inside a fail-closed PR would smuggle a second policy in. (3) Non-regressing? Yes — pre-existing, and §4.2 makes the gate's *correctness* independent of its speed. (4) Durable home? `project_open-defer-slots.md`. **Trigger**: the offline-gate policy decision, or a §3 table large enough that the gate's runtime is noticed. **Re-eval**: 2026-11-30. |

**Explicitly NOT deferred**, recorded so the absence is deliberate: the nine under-report paths (D1 — locked in scope), the `preflight` fail-open (D2), `axes.md`/`CLAUDE.md` (D3), the test relocation (D4), CI wiring (D5), the emitter signature (§4.6.2), the three §-number grammars (§4.6.3), and all three efficiency findings (§4.6.1 — they are the same edit as §4.1.2 and §4.1.6, not extra scope).

**Pre-existing defects surfaced but not owned by this PR** (a separate category from own-deferrals, per [[feedback_defer_cap_policy]]): the `webref_data.py` `@lru_cache` docstring/`--help` disagreement (`cache.py:46-47` and `cli.py:64-67` still say `ELIDEX_WEBREF_NO_CACHE=1` "bypass[es] entirely", which the new memo layer defeats). One-line doc fix, folded in with §4.1.6 since it touches the same file and the same claim.

---

## §12 Exit criterion

A runnable pair. Both must hold; neither depends on any count in this memo.

**(1) Green — the enforcement suite runs and passes, from the task that CI depends on:**

```sh
mise run tools-test
```

Trustworthy because of this PR's own fixes: the task exists and is in `[tasks.ci].depends` (§4.5 Edit 1), a GitHub `tools` job runs the same two lines on a `.claude/**`-only diff (§4.5 Edit 2), and the suite drives `cli.main` end-to-end so the `--strict` wiring is covered rather than bypassed (§6-C1). Before this PR, this command does not exist.

**(2) Red — every new pin actually detects the defect it names:**

```sh
git worktree add /tmp/citeaudit-pre bf580047
cp .claude/tools/_webref/test_*.py            /tmp/citeaudit-pre/.claude/tools/_webref/
cp .claude/skills/elidex-plan-review/test_*.py /tmp/citeaudit-pre/.claude/skills/elidex-plan-review/
cd /tmp/citeaudit-pre && mise run tools-test; echo "EXPECT NON-ZERO: $?"
```

The new tests run against the **carved, unfixed** tool. This must exit non-zero, with at least one failure attributable to each of the nine classes (T1-T9), to D2 (P2), and to the coverage gap (C1). A test that passes here pins nothing — the failure mode `test_prefix_tolerant_resolver_is_pinned_to_an_exact_match` already demonstrates in-tree (§6).

**(3) The retirement actually happened** — one grep, because §4.7 marks it as the one doc claim no test pins:

```sh
grep -q 'cite-audit' .claude/skills/elidex-review/axes.md \
  && grep -q 'cite-audit' CLAUDE.md \
  && ! grep -q '≥4 grep pattern' .claude/skills/elidex-review/axes.md \
  && echo RETIRED
```

Today (2026-07-28) this prints nothing: `grep -rn 'cite-audit' .claude/skills/ CLAUDE.md` returns **no matches**.

---

## §13 Coordination

Re-derived 2026-07-28 from `git worktree list`, `git diff --name-only origin/main..<branch> -- .claude/ mise.toml .github/`, and `gh pr list --state open` — not from memory.

⚠ **`origin/main` moved during this memo's authoring**: base `6b33854d` → **`96a8e47b`** (#490 `258b799e` shell traversal_queue split, #488 `96a8e47b` LayoutBox reader seam). **Rebase before implementing.** #488 added a **fourth** trip-wire and edited `mise.toml`, which is this PR's collision surface.

| Lane | State | Overlap with this PR | Ordering rule |
|---|---|---|---|
| **PR-A0** (`domform-submittable-category` @ `d3173bed`) | the carve source; still carries the identical 8 `.claude/` files | **Total, by construction** — `git diff domform-submittable-category -- .claude/` is empty today. PR-A0 must **drop** those files (rebase/revert the `.claude/` half) once this lands, or the two PRs conflict on every line. It must also re-baseline its §4 counts against §5's widened attribution. | **This PR lands first.** PR-A0 then rebases onto it and re-runs `cite-audit` per spec (§4.3(b)) — which is how its 17 phantom `XHR §4.3` cites become visible. |
| **PR-A** (`domform-submittable-category`, same branch) | plan-memo `2026-07-form-submittable-category-repair.md` | Indirect: it is the memo §4.2 measures `preflight` against (21 rows / 15 unique citations). No file overlap. | none |
| **Slice 1** (`elidex-wt-slice1` @ `7261c2fa`) | 2 `.claude/` files vs origin/main | Not the `_webref` tree. Verify at rebase. | none |
| **VM P4** (`elidex-wt-vmp4` @ `7cca56a9`, **PR #489 open**) | 2 `.claude/` files; `mise.toml` | `mise.toml` only. `[tasks.tools-test]` is a new block; conflict risk is low but real. | whichever lands second rebases |
| **Layout C-3a** (`terminal-z-c3a-impl`) | **merged as #488** — the `mise.toml` + trip-wire change is already on `origin/main` | Resolved by rebasing onto `96a8e47b`. | done |
| **C-3 plan** (`elidex-wt-c3-plan` @ `7204c12e`) | 7 `.claude/` files; `mise.toml` | Verify at rebase; likely the same #488 trip-wire files. | whichever lands second rebases |

**Carried edits (ship with this PR):**

1. **`project_open-defer-slots.md`** — register the 3 new `cleanup-webref-*` slots (§11). Registration per [[reference_spawn-task-chips-not-durable]] — a memory-file write, not a chip.
2. **`MEMORY.md`** — the L3 row currently names PR-A0 as the next thing on `domform-submittable-category`; add this PR as PR-A0's new hard prereq, with branch + worktree.
3. **`.claude/tools/_webref/DESIGN.md`** — the `cite_audit.py` bullet (`:58-63`) describes discovery but not the gate; add the reported-class contract (UNRESOLVED / UNATTRIBUTED / UNKNOWN-SPEC / REJECTED-TOKEN / SKIPPED) and the `--strict` semantics, since `axes.md` will now point authors here.
4. **PR description** — must state the §5 attribution widening explicitly (Q6), because it silently changes the number an in-flight sweep is measuring against.
