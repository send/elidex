# HTML fragment serializer — escaping families, raw-text parents, entry selection (umbrella plan-memo)

Slot `#11-html-fragment-serializer-conformance` (ledger: open, owner L3). The program closes item **(e)** and the
security part of item **(d)** (PM 724, 2026-11-13; scope re-cut by **PM 731**, option C; the XMLSerializer path by
**PM 785**, option (b2); split into this umbrella and two per-PR plans, and their order, by **PM 797**); items (a)
(b) (c) (f) stay in the slot with Re-eval 2026-12-15, and (d)'s precise noscript rule moves to
`#11-scripting-disabled-predicate-no-document` (§5, PR-H §9). Owner lane: L3 DOM/form. Security-relevant: both items
are markup-injection (mXSS) surfaces of `innerHTML` / `outerHTML` / `getHTML` / `XMLSerializer`.

Slices — each is a per-PR plan-memo, self-sufficient for its own `/elidex-plan-review`:

- **PR-X** (lands first) — the escaping-family selection and the XML family:
  [2026-11-html-fragment-serializer-pr-x-xml-family.md](2026-11-html-fragment-serializer-pr-x-xml-family.md)
- **PR-H** (lands second, stacked on PR-X) — HTML §13.3 *escaping a string* and the raw-text parents:
  [2026-11-html-fragment-serializer-pr-h-escaping-and-raw-text.md](2026-11-html-fragment-serializer-pr-h-escaping-and-raw-text.md)

Base: `origin/main` `c0578223` (the branch's merge-base; rebased from `06bdfbbd` on 2026-11-15). Every code fact in
the three memos was measured at `c0578223` (2026-11-15, or 2026-11-18 where marked); line numbers move, the commands
do not. `origin/main` was `1146380f` on 2026-11-18: `git diff --stat c0578223 origin/main --
crates/dom/elidex-dom-api crates/script/elidex-script-session crates/script/elidex-js/src/vm/host
crates/script/elidex-js/src/vm/tests/mod.rs` touches, among the files the three memos cite, only
`crates/script/elidex-js/src/vm/tests/mod.rs` (+1 line).

**This memo states each program-level rule once** (the split, the order, the exposure window, the shared premises
P1 / P2, I-1, I-3, the pair matrix, the cross-lane pins, the ledger edits). The per-PR memos refer here for those
and state everything about their own slice themselves. **IDs are program-wide** (P, D, S, I, U, V, X, XT, XA, XC, R,
F, Q): an ID keeps the number it had in Rev 6, and §1's index names the one file that defines it.

Rev 1-6 are revisions of the single per-PR memo `docs/plans/2026-11-html-fragment-serializer-escape-and-raw-text.md`,
which Rev 7 retires (deleted, not kept as a pointer: its whole content now lives in these three files, nothing in
the repository links to it — `git grep -n -F 'html-fragment-serializer' -- ':!docs/plans/2026-11-html-fragment-serializer-*'`
→ 0 hits, 2026-11-18 — and a pointer file would be a fourth place to keep true). Its last state is `git show
418c7e6a:docs/plans/2026-11-html-fragment-serializer-escape-and-raw-text.md`; its history is `git log 418c7e6a --
docs/plans/2026-11-html-fragment-serializer-escape-and-raw-text.md`.

**Rev 1** (2026-11-13) — first draft (context-based noscript via a `ScriptingContext`).
**Rev 2** (2026-11-15) — PM 731 option C: noscript text is always escaped; no scripting composition is added
(`ScriptingContext`, `SerializeOptions.scripting`, the natives' context construction and every `bound_document` /
`sandbox_flags` read are gone); D-5 withdrawn with it; accepted regressions (§7); plan-review fixes (axes 1-5).
**Rev 3** (2026-11-15) — R2 plan-review fixes: §7 R2's cause stated per document kind, with the N4 obligation that
fixes each cell; P6's parse-side population defined by property and measured (F1-F8), R2 extended to outerHTML (V12 /
V13); the §13.4 cite is *initialize an HTML fragment parser* step 10; I-2 scoped (bare Text entry); per-site mutants
and a `RAW_TEXT_ELEMENTS` read-site grep; the void check's exclusion argued; Q-3 replaced by the provisional slot
`#11-dom-api-handlers-unreachable-from-production`; landing-order pins and ledger facets.
**Rev 4** (2026-11-15) — focused re-check MINs: §7 R2 / §10 cells split by the §8.1.3.4 value for the context
document (not by document kind); §9's void-check rationale restated without the false universal; U6's DOMParser-like
fixture is spec-exact and does not flip, DoD 6 enumerated; cite fixes (§13.4 initializer step 11, §13.2.6.4.7, HTML
§8.5.9 → DOM Parsing §5.2.1.5); landing-order pins built by a census over in-flight branches; the handler slot is
registered.
**Rev 5** (2026-11-18) — PM 785, option (b2): the XMLSerializer path (`serialize_node_to_string`) escapes with DOM
Parsing's own two algorithms, one NEW function each (D-6), not with `push_escaped_string` (whose U+00A0 → `&nbsp;`
would emit an entity XML does not define); P7, I-1 / I-2 / I-3, §3-§6, §9 and §10 follow. Rev 4 focused-review fixes:
U6's plain-document fixture is not a DOMParser document in N4's terms, so all four U6 functions name the slot and V2
pins the disabled side (DoD 6: 10 items); the inert-template-document marker gap is stated; the pin census command no
longer depends on the shell's word splitting and was re-run; §9's unescaped outputs are listed in full; HTML §4.12.3
step numbers.
**Rev 6** (2026-11-18) — focused re-check fixes on Rev 5: on the XMLSerializer family every Text is escaped by DOM
Parsing §5.2.1.5, including a Text child of an HTML raw-text parent — §13.3's literal branch is read on the
`HtmlFragment` family only (D-6, I-2, X4; no longer deferred to `#11-domparser-xml-real-parsing`); the three axes'
pair matrix is written out (§2) and §1 says why the PR stays one slice; the `>` ruling for *serializing an attribute
value* (steps normative, Note informative: not replaced; D-6, §0.4); I-1 × I-2 per family (X5); `markup.rs`'s module
header numbers fixed in this PR (§4.1, DoD 4); `xargs -0 -r` in the pin census.
**Rev 7** (2026-11-18) — PM 797: the Rev 6 focused re-check ruled SPLIT under the Edge-dense rule (S-1). The memo is
restructured into this umbrella + PR-X + PR-H with no change of design content: every decision, cell, mutant and
citation of Rev 6 is in exactly one of the three files (§1 index). Changes that are not pure relocation, each a
consequence of the split or an open MIN of the Rev 6 re-check: (1) Rev 6 §1's "it remains a single terminal slice"
is withdrawn (S-1); (2) PR-X gains the contrast cell XC1 — the literal side of the (ii) × (iii) pair on a parent that
is literal at HEAD, since Rev 6's contrast cell U4 is PR-H's — and its coverage map gains the HTML §8.5.9 step 1
row (the ground of the selection, a context cite in Rev 6); (3) the selection mutants are stated per PR, against the
escapers each PR's `HtmlFragment` arms actually call (PR-X §6, PR-H §6); (4) MIN, Axis 3: the set of parents whose
Text HEAD emits literally is corrected and stated once (P2); (5) MIN, Axis 4: the header-cite check counts module
header lines only (PR-X DoD 4); (6) the pin census excludes this program's memos by their new common name prefix
(§4.2; re-run, the same 8 lines).

## §0. Frame, shared premises, program decisions

### §0.1 Frame

One serializer walk in `elidex-dom-api` (`element/markup.rs`) serves four entries. Three are HTML §13.3's
(`innerHTML` / `outerHTML` / `getHTML`); the fourth, `XMLSerializer.serializeToString`, is the XML serialization's
(HTML §8.5.9 step 1 → DOM Parsing §5.2.1). At the base the walk decides three things wrongly or unsafely, and they
are the program's three axes:

- **(i) the character map of escaping** — HEAD's two escapers (P1) are neither HTML §13.3's *escaping a string* nor
  DOM Parsing's two algorithms.
- **(ii) the raw-text decision for a Text node's parent** — HEAD's list (P2) holds noscript unconditionally, lacks
  plaintext, and is matched without a namespace check.
- **(iii) the entry's selection of escaping family** — HEAD has none: the XMLSerializer entry runs the §13.3-like
  escapers and the §13.3 literal branch.

PR-X introduces (iii) and gives the XML family its two algorithms; PR-H makes (i) and (ii) exact or safe on the
HTML fragment family. The spec text and the design of each are in the per-PR memos (§0.1 / §0.3 of each).

### §0.2 Shared premises (re-measured at `c0578223`)

- P1 (e): `escape_html` (`crates/dom/elidex-dom-api/src/util.rs:154`) maps `&`, `<`, `>` — no U+00A0.
  `escape_attr` (`util.rs:168`) maps `&`, `"` — no `<`, `>`, U+00A0. Two diverging copies of one spec algorithm.
  `util` is `pub(crate)` (`crates/dom/elidex-dom-api/src/lib.rs:40`), so both are crate-internal.
- P2 (d): `RAW_TEXT_ELEMENTS` (`crates/dom/elidex-dom-api/src/element/markup.rs:22`) = script, style, xmp, iframe,
  noembed, noframes, **noscript** — lacks plaintext, and noscript is unconditional. Read at `markup.rs:243` (entry,
  `serialize_inner_html_with_options`) and `markup.rs:445` (recursion, `serialize_node`), both by `TagType` string
  only — **no namespace check**, so an SVG `<style>` (foreign content, whose text the parser entity-decodes) is
  serialized literally too (see D-4).
  **The HEAD literal set** (stated here once; the per-PR memos refer to it): the list above is read by `TagType`
  alone at both sites (`RAW_TEXT_ELEMENTS.contains(&tag.0.as_str())`, `git show c0578223:crates/dom/elidex-dom-api/src/element/markup.rs |
  sed -n '22,24p;243p;445p'`, re-read 2026-11-18), and neither site looks at which entry is running. So at HEAD the
  parents whose Text children are emitted literally are exactly **{script, style, xmp, iframe, noembed, noframes,
  noscript} in any namespace**, on all four entries, the XMLSerializer one included; a **plaintext** parent's Text
  is already escaped at HEAD. (Rev 6 described this set with plaintext and without noscript, and as HTML-namespace
  only, in its §3.1, §6 census and §7 preamble; corrected by Rev 7 in the sentences PR-X now owns.)

P3-P6 (the scripting predicate and the parse-side population F1-F8) are PR-H's (§0.2 there); P7 (the XMLSerializer
path) is PR-X's.

### §0.3 Program decisions

- **S-1 — the work is split (Edge-dense rule; PM 797).** It binds three intersecting invariant axes ((i), (ii),
  (iii) of §0.1; their pairwise intersections are §2's matrix), and no approved umbrella stood above the Rev 6 memo,
  so the rule's base case (a narrowly-scoped slice under an approved umbrella is a terminal unit) did not apply to
  it. Rev 6 §1 argued for one terminal slice on the ground that (iii) multiplies no walk state; the Rev 6 focused
  Axis 3 re-check ruled that the count of axes, not the size of their product, is the rule's trigger. That argument
  is withdrawn. Each of the two slices below is a terminal unit once this umbrella is approved.
- **S-2 — two slices, PR-X before PR-H.**
  - **PR-X** = axis (iii) and the XML half of (i): D-6 — `CharacterEscaping`, `Walk`, the two dispatch functions,
    `xml_serialization.rs`, the family gate (no XML Text is literal), and the comment / header cite fixes. Its
    `HtmlFragment` arms keep calling HEAD's `escape_html` / `escape_attr`, and HEAD's raw-text list is unchanged, so
    the three §13.3 entries' output is byte-identical to HEAD's.
  - **PR-H** = the HTML half of (i) and axis (ii): D-1..D-5 — it replaces only the bodies of the `HtmlFragment`
    arms and the predicate behind `in_raw_text`, and deletes `util.rs`'s two escapers.
  - **Why this order.** After PR-X the XML family's output no longer depends on the raw-text list (its arm reads no
    `in_raw_text`; PR-X D-6). PR-H is then a change on two axes, (i) × (ii), on one family, with (iii) fixed and
    pinned: PR-X's cells stay green and unedited through PR-H (PR-H DoD 1, the independence check). In the other
    order the first PR would change the raw-text list while the XMLSerializer entry still read it, and every (ii)
    change would have to be argued on both families.
  - **Dependency.** PR-H depends on PR-X (it edits arms PR-X creates). PR-X depends on nothing in PR-H: it is
    landable alone, and alone it only moves the XMLSerializer path toward DOM Parsing (PR-X §7).
- **S-3 — the exposure window (PM 797).** PR-X closes the XMLSerializer path's two injection defects (P7: an
  attribute value's raw `<`; literal Text under a parent of P2's HEAD literal set). The defects on the three §13.3
  entries **close only at PR-H**: noscript Text re-emitted literally (D-3; V1, V2, V4), the namespace-blind raw-text
  match that emits an SVG `style` / `script`'s Text literally (D-4; U5), and with them the attribute value's raw
  `<` / `>` on those entries (D-1; V5). Between PR-X's landing and PR-H's, `innerHTML` / `outerHTML` / `getHTML`
  behave exactly as at HEAD — the window adds no exposure and removes none on those entries. It is bounded by
  process: PR-H is stacked on PR-X and is filed immediately after PR-X lands; PR-H's implementation may be stacked
  on PR-X's branch before PR-X lands. **If PR-X's landing is expected past 2026-12-05, the driver reports to the
  PM, who re-decides the order** (PM 797). PR-X's PR body states this window (PR-X DoD 5).
- **S-4 — review units.** The umbrella and both per-PR memos are written together (PM 797). Each per-PR memo goes
  through its own full `/elidex-plan-review` before its implementation; the coupled-invariant enumeration the
  skill's Pre-condition #3 asks of an edge-dense plan is §2 of this memo, and each per-PR §2 names the rows it pins.

## §1. Slices and the ID index

| Slice | Axes it moves | Decisions | Cells | Accepted regressions | Depends on |
|---|---|---|---|---|---|
| PR-X | (iii) introduced; (i) on the XML family; (ii) unchanged, gated off the XML family | D-6 | XT1-XT3, XA1-XA3, X1-X5, XC1 | none | — |
| PR-H | (i) on the HTML fragment family; (ii) | D-1, D-2, D-3, D-4 (D-5 withdrawn) | U1-U6, V1, V2, V4-V7, V10-V13 | R1, R2 | PR-X |

| IDs | Defined in |
|---|---|
| P1, P2 (and the HEAD literal set) | this memo §0.2 |
| P3-P6, F1-F8 | PR-H §0.2 |
| P7 | PR-X §0.2 |
| S-1..S-4 | this memo §0.3 |
| D-1..D-5 | PR-H §0.3 |
| D-6 | PR-X §0.3 |
| I-1, I-3, the pair matrix | this memo §2 |
| I-2, I-1 × I-2 | per family: `HtmlFragment` in PR-H §2, `XmlSerialization` in PR-X §2 |
| U1-U6, V1-V13 (cell numbers absent from that range in Rev 6 are not reused) | PR-H §6 |
| XT1-XT3, XA1-XA3, X1-X5, XC1 | PR-X §6 |
| R1, R2 | PR-H §7 |
| Q-1..Q-3 | PR-H §11 |
| Q-4, Q-5 | PR-X §11 |

## §2. Program invariants and the pair matrix

- I-1 (escaping), end state after PR-H: every Text datum passes through `push_text_data` (NEW: PR-X) (which writes
  it verbatim only in its `HtmlFragment` arm with `in_raw_text`, I-2) and every attribute / `is` value through
  `push_attribute_value` (NEW: PR-X); those two dispatch to exactly three escaping functions, one per spec algorithm —
  `push_escaped_string` (NEW: PR-H) (HTML §13.3 *escaping a string*), `push_xml_text` (NEW: PR-X) (DOM Parsing
  §5.2.1.5) and `push_xml_attribute_value` (NEW: PR-X) (*serializing an attribute value*). No other markup-escaping
  code exists in `crates/` (concept sweep, PR-X §5.1; PR-H DoD 1).
  **Staging.** After PR-X and before PR-H the dispatch is already final and the `XmlSerialization` arms already call
  the DOM Parsing pair; the `HtmlFragment` arms call HEAD's `escape_html` / `escape_attr` (P1), each from exactly
  one arm (PR-X DoD 1). So the interim has four escaping functions behind one dispatch, not a second dispatch:
  PR-H swaps the bodies of two arms and deletes the two HEAD functions; no call site outside the two dispatch
  functions changes between the PRs.
- I-3 (selection), from PR-X on: the escaping family is fixed once per public entry and never changes inside a
  walk — the three §13.3 entry points use only the `HtmlFragment` arms, `serialize_node_to_string` uses only the
  DOM Parsing pair (D-6), and the raw-text decision is read on the `HtmlFragment` family only.
  `serialize_node_to_string` has one production caller, the XMLSerializer native (P7), so "the XMLSerializer path"
  and "`serialize_node_to_string`" name the same population.
- I-2 (raw text) and I-1 × I-2 are stated per family, each in the memo of the PR that makes that family exact:
  `XmlSerialization` in PR-X §2 (no Text is literal), `HtmlFragment` in PR-H §2 (literal iff
  `text_children_literal(parent)` (NEW: PR-H)).

**Pair matrix of the three axes (§0.1)** — each intersection in one line, with the cells that pin it and the PR
that lands each cell:

| Pair | Intersection | Cells (PR) | Exact from |
|---|---|---|---|
| (i) × (ii) | A literal Text bypasses escaping entirely; every other Text goes through the family's Text function, and an attribute value is never literal (no raw-text decision reads it): the noscript argument of I-1 × I-2 holds through `<` | U4, U6, V5 (PR-H); X5 (PR-X) | `XmlSerialization`: PR-X; `HtmlFragment`: PR-H |
| (i) × (iii) | The family picks the whole character map, never mixed in one walk: `HtmlFragment` → `push_escaped_string(Text \| Attribute)` (HEAD's `escape_html` / `escape_attr` between the PRs), `XmlSerialization` → `push_xml_text` / `push_xml_attribute_value` | U1-U3, V6 (PR-H) vs XT1-XT3, XA1-XA3, X1-X3, X5 (PR-X) | the selection: PR-X; the `HtmlFragment` map: PR-H |
| (ii) × (iii) | The raw-text decision is read only on the `HtmlFragment` family: literal iff `HtmlFragment` ∧ the walk's `in_raw_text`; through XMLSerializer every Text is escaped, a script / style parent included | XC1 (literal on `HtmlFragment`, PR-X; generalised to U4 by PR-H) vs X4 (escaped on `XmlSerialization`, PR-X) | PR-X (the gate); the predicate behind `in_raw_text`: PR-H |

## §3. Spec coverage, by slice

This memo implements no spec step of its own; each per-PR memo carries the preflight-parsable `§3. Spec coverage
map` and the citation table (§0.4) for its slice.

| Slice | Rows | Specs |
|---|---|---|
| PR-X | 3: HTML §8.5.9 `serializeToString` step 1; DOM Parsing §5.2.1.5 steps 3-5; DOM Parsing §5.2.1.1.3 *serializing an attribute value* step 3 | HTML, DOM Parsing |
| PR-H | 5: HTML §13.3 *escaping a string* (text mode, attribute mode); §13.3 step 5.2 (literal list, noscript, otherwise) | HTML |

## §4. Order, stacking, cross-lane pins

### §4.1 Order and stacking

PR-X, then PR-H (S-2); the window and the 2026-12-05 report are S-3. PR-H's premises cite `markup.rs` lines at
`c0578223`; PR-X moves them, so PR-H re-measures its cites at its own base (PR-X's head while stacked, `origin/main`
once PR-X has landed) before implementation — by the commands its §0.2 / §5 give, which do not move.

### §4.2 Cross-lane pins

No recorded order between this program's PRs and the memos / PRs that pin lines they move; whichever lands second
re-pins. Neither PR edits `dom_inner_html.rs`, so line cites there (brand-check) are unaffected. PR-X rewrites
comments at four places in `dom_parser.rs` (PR-X §4.2), so line cites into that file after `:37` may shift and
follow the same rule. The program moves lines in:
- `crates/dom/elidex-dom-api/src/element/markup.rs` (both PRs). The pin list is a census over in-flight branches,
  not a named list (re-run 2026-11-18 with the Rev 7 filter; re-run before each PR lands). The command quotes every
  expansion and passes file names
  NUL-separated, so it gives the same 8 lines under `bash` and `zsh` (checked with both; Rev 4's form relied on
  unquoted word splitting, which `zsh` does not do); `xargs -0 -r` runs nothing for a branch that changes no file
  under `docs` (without `-r`, GNU `xargs` would run `git grep` once with no paths, i.e. over the whole tree of that
  ref; re-run with `-r` on 2026-11-18: the same 8 lines):
  ```sh
  git for-each-ref --format='%(refname:short)' refs/heads refs/remotes/origin | while IFS= read -r b; do
    [ "$(git rev-list --count "origin/main..$b" 2>/dev/null || echo 0)" -gt 0 ] || continue  # ahead of origin/main
    git diff --name-only -z "origin/main...$b" -- docs | xargs -0 -r git grep -n -P 'markup\.rs(:[0-9]|`)' "$b" --  # docs the branch changes
  done | grep -v html-fragment-serializer- | sort -u
  ```
  (The `grep -v` drops this program's own memos; Rev 6's pattern was the retired memo's name.)
  Result (8 lines): branch `ecs-shadow-root-lifetime-plan` @`23aa745a` — the pr-ig memo
  (`docs/plans/2026-11-ecs-entity-lifetime-pr-ig-dispatcher-take-guard.md:154`, `markup.rs:117`) and the pr-io memo
  (`docs/plans/2026-11-ecs-entity-lifetime-pr-io-owned-entities-walk-scope.md:982` / `:1045`, `markup.rs:304` /
  `:256`); branch `tree-model-shadow-root-plan` @`1593a5ad` — `docs/plans/2026-11-tree-model-census.md` `:93` (the
  `markup.rs` row of its count table) and `:197` (`markup.rs:307`). The other three hits are the same line,
  `docs/plans/2026-11-focus-fixup-canonical-steps.md:105`, on three refs, and that line is byte-identical to
  `origin/main`'s line 105 on each (`git show <ref>:<path> | sed -n 105p`, compared by hash): on
  `layout-focus-fixup-canonical` / `origin/layout-focus-fixup-canonical` because that branch landed by squash (#603
  `1580af13`); on `layout-focus-fixup-ungated` @`b800c6e7`, which is **not** landed and not an ancestor of
  `origin/main` — it is the superseded earlier variant of the same work, kept as a local branch with no PR (`gh pr
  list --state all --head layout-focus-fixup-ungated` → none), not an in-flight one. So the only copy of that pin
  that will be read is `origin/main`'s — a landed, dated pin. (Whether the local branch should be deleted is the
  lane owner's / PM's call, not this memo's.)
- `crates/dom/elidex-dom-api/src/lib.rs`: PR-X adds the two `mod xml_serialization…` lines (PR-X §4.1a), and
  AbortSignal.any PR-A (branch `abort-signal-any-flatten-pr-a`) adds `pub mod abort;` to the same file — P1's
  `lib.rs:40` and P7's `lib.rs:93` shift under either; the two insertions are at opposite ends of the `mod` list.
  PR-A also adds `mod tests_abort_dependent;` to `crates/script/elidex-js/src/vm/tests/mod.rs` (PR-X adds `mod
  tests_html_serialize_escape;` to the same list — a textual neighbour, no semantic conflict).

## §5. Ledger and slot edits (pmo, after each merge)

**At PR-X's landing:**
- `#11-html-fragment-serializer-conformance`: add "PR-X landed (the XMLSerializer path escapes by DOM Parsing);
  items (d) and (e) stay open until PR-H — the §13.3 entries behave as before in between (umbrella S-3)". No item
  closes.
- `#11-domparser-xml-real-parsing` (PM 785): the two escaping functions (parts of its XML serializer) landed in
  PR-X. In the row (`project_open-defer-slots.md`, read 2026-11-18), replace
  - old: "real XML parsing (XML/XHTML, not html5ever) + XML **serialization** (void self-close, namespace prefixes) +
    `XMLSerializer.serializeToString` **Attr-root** contract."
  - new: "real XML parsing (XML/XHTML, not html5ever) + XML **serialization** (void self-close, namespace prefixes,
    the *require well-formed* = true checks) +
    `XMLSerializer.serializeToString` **Attr-root** contract. **Landed in serializer PR-X (PM 785)**: the two
    escaping functions (parts) — `push_xml_text` (DOM Parsing §5.2.1.5 *XML serializing a Text node*, steps 3-5) and
    `push_xml_attribute_value` (§5.2.1.1.3 *serializing an attribute value*, step 3), in
    `crates/dom/elidex-dom-api/src/xml_serialization.rs`; `serialize_node_to_string` already calls them, on the HTML
    walk without §13.3's literal raw-text branch (every Text escaped by §5.2.1.5). The XML serializer extends that
    module and uses both as they are."

  and in the same row's **Why deferred**, replace
  - old: "all 5 accepted MIME currently HTML-parsed + HTML-serialized"
  - new: "all 5 accepted MIME currently HTML-parsed + serialized by the HTML walk (with DOM Parsing's two escapes
    since serializer PR-X)"

  The rest of the row (the boa-parity ground, Trigger, Date) is unchanged by PR-X.

**At PR-H's landing:**
- `#11-html-fragment-serializer-conformance`: (e) closed; (d) closed except the precise noscript rule, which moves to
  `#11-scripting-disabled-predicate-no-document` (PM 731); (a) gains "the void check (`markup.rs` `VOID_ELEMENTS`) has
  no namespace check" (PR-H §9); (a) (b) (c) (f) open, Re-eval 2026-12-15. Item (b)'s text names
  `serialize_inner_html_with_options`, which PR-H keeps (D-5 withdrawn), so it stands as written.
- `#11-scripting-disabled-predicate-no-document`: the PM 731 facet is already recorded; append:
  (i) "serializer side: noscript always escaped since PR-H; accepted regressions R1/R2, split by the HTML
  §8.1.3.4 value for the context document (what N4's `EcsDom::scripting_disabled_for` returns): scripting
  **enabled** — U6 (four unit functions; two fixtures, the cached document root and a plain `create_document_node`
  Document, neither carrying the null-browsing-context marker, so N4 answers enabled for both) / V1 / V10 / V12,
  flip at obligation (1), the precise serializer rule, which applies to the HTML fragment entry points only (on the
  XMLSerializer path a noscript's Text is escaped by DOM Parsing §5.2.1.5, which reads no parent); scripting **disabled** — V11 / V13 (DOMParser
  documents), flip at obligation (2), the parse side; V2 (a real DOMParser document) is spec-exact and does not
  flip; each flipping cell's comment names this slot. N4 may rebuild U6's plain-document fixture with the marker, in
  which case that pair stays escaped as the disabled side";
  (ii) "obligation (2)'s population = the 8 production fragment-parse paths that set the parser's scripting flag
  without consulting the context document (PR-H memo P6 F1-F8: Element / ShadowRoot `innerHTML` and
  `setHTMLUnsafe` setters, `outerHTML` setter, `insertAdjacentHTML` and the `Mutation::SetInnerHtml` flush (both via
  unreachable handlers), DOMParser's inert build), measured by construction of `SetInnerHtmlOptions` /
  `ParseFragmentOptions`, not by vocabulary. Their defect shows wherever scripting is **disabled** for the context
  document: DOMParser documents, a page `<template>`'s content (inert template document), and other documents with
  no browsing context. **Writer obligation this depends on**: obligation (2) reaches a document only if N4's
  composition answers disabled for it; the null-browsing-context marker's stated writers are DOMParser's build and
  *destroy a document*, so the inert template document (`template_owner.rs`, HTML §4.12.3 *appropriate template
  contents owner document* step 1.1.1: 'its browsing context is null') and any other document created with a null
  browsing context need a writer too, or N4 answers enabled for them and template content keeps the round-trip
  growth. A future production fragment-parse path (e.g. `#11-range-create-contextual-fragment`'s
  `createContextualFragment`) joins this population, measured by the same construction census";
  (iii) "the Defect's premise 'the same predicate is read two ways' (the serializer's fail-safe reading) no longer
  holds after PR-H: the serializer reads no predicate; the Resolution's serializer-side item (removing the
  serializer's separate reading) is done by PR-H. What remains of the Defect is the canonical function's fail-open
  no-document reading (aligned at N4) and obligations (1) / (2)".
- `#11-dom-api-handlers-unreachable-from-production` (registered, PM 583/744): cite PR-H in that slot (it left
  `GetInnerHtml` untouched pending the class, PR-H §9); the parse side (P6 F6 / F7, `"insertAdjacentHTML"` /
  `"innerHTML.set"`) also belongs to obligation (2)'s population.
- No facet on `#11-browsing-context-state-ecs-components` (PM 731).

## §6. Out of scope

Each per-PR memo's §9 carries the out-of-scope items of its slice with their slots and the argument for each: the
rest of the XML serialization (`#11-domparser-xml-real-parsing`) in PR-X §9; slot items (a) (b) (c) (f) and the
void check's namespace defect, the precise noscript rule and the parse-side scripting decision
(`#11-scripting-disabled-predicate-no-document`), DOMParser's bare `<noscript>` parse
(`#11-domparser-full-document-parse-fidelity`) and `GetInnerHtml`
(`#11-dom-api-handlers-unreachable-from-production`) in PR-H §9. Nothing is out of scope at the program level that
is not in one of those two lists.

## §7. Definition of done (program), open items

1. PR-X landed with its DoD (PR-X §10); §5's PR-X items applied.
2. PR-H landed with its DoD (PR-H §10), X1-X5 / XT / XA green and unedited at its head; §5's PR-H items applied.
3. I-1's end state holds at PR-H's head (PR-H DoD 1).

Open: none at the program level. The per-PR open question is Q-5 (PR-X §11). Standing condition: S-3's 2026-12-05
report.
