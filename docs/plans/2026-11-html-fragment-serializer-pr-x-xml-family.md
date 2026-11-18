# HTML fragment serializer PR-X — the escaping-family selection and the XML family (per-PR plan-memo)

Slice **PR-X** of the umbrella
[2026-11-html-fragment-serializer-umbrella.md](2026-11-html-fragment-serializer-umbrella.md) (slot
`#11-html-fragment-serializer-conformance`; PM 785, option (b2); split and order by PM 797). Lands **first**; PR-H
([2026-11-html-fragment-serializer-pr-h-escaping-and-raw-text.md](2026-11-html-fragment-serializer-pr-h-escaping-and-raw-text.md))
is stacked on it. Base: `origin/main` `c0578223`; every code fact below was measured there (2026-11-15, or
2026-11-18 where marked); line numbers move, the commands do not. Owner lane: L3 DOM/form. Security-relevant: closes
the two markup-injection defects of the `XMLSerializer` path (P7); the §13.3 entries' defects close at PR-H
(umbrella S-3).

Stated in the umbrella and only referred to here: the split, the order and the exposure window (S-1..S-3), the
shared premises P1 / P2 with the HEAD literal set, I-1, I-3, the pair matrix, the cross-lane pins and the ledger
edits. IDs are program-wide (umbrella §1 index); an ID not defined in this memo is defined in the file that index
names. Revision history: the umbrella (Rev 1-6 of the retired single memo; Rev 7 = this split).

## §0. Frame, premises, decisions

### §0.1 Frame

Three of the four serializer entries are HTML §13.3's (`innerHTML` / `outerHTML` / `getHTML`). The fourth,
`XMLSerializer.serializeToString`, is not: its ground is the XML serialization (HTML §8.5.9 step 1 → DOM Parsing
§5.2.1), whose two escaping algorithms differ from §13.3's *escaping a string*. This PR gives that entry those two
algorithms (D-6, PM 785) and takes §13.3's literal (raw-text) Text branch off it, since §5.2.1.5 escapes every Text
node; the rest of its walk stays the HTML one until `#11-domparser-xml-real-parsing` (§9).

To do that the walk must know which serialization an entry is, so this PR introduces that selection (axis (iii) of
the umbrella) for all four entries. On the three §13.3 entries the selected arms run HEAD's code unchanged; making
them exact is PR-H.

### §0.2 Premises re-measured at `c0578223`

P1 (HEAD's two escapers) and P2 (HEAD's raw-text list, its two namespace-blind read sites, the HEAD literal set):
umbrella §0.2.

- P7 (the XMLSerializer path): `serialize_node_to_string` (`markup.rs:183`) has one production caller, the
  `XMLSerializer.serializeToString` native: `git grep -n -w 'serialize_node_to_string' -- crates` → 6 hits = the
  definition, 2 re-exports (`element/mod.rs:31`, `lib.rs:93`), 2 doc comments (`dom_parser.rs:38`, `:325`) and 1 call
  (`crates/script/elidex-js/src/vm/host/dom_parser.rs:404`) (2026-11-18). It runs the HTML walk with P1's two
  escapers, so a `div` whose `title` is `x\u{a0}y<z>` and whose text is `p\u{a0}q` serializes as
  `<div title="x\u{a0}y<z>">p\u{a0}q</div>` (driver probe at `27ca7d1e`, PM 785; it also follows from P1): U+00A0 raw
  in both positions, the attribute's `<` raw. This entry's ground is not §13.3: HTML §8.5.9 `serializeToString(root)`
  step 1 is "Return the XML serialization of root given false", and the XML serialization escapes by DOM Parsing
  §5.2.1.5 *XML serializing a Text node* (steps 3-5: `&`, `<`, `>`) and by *serializing an attribute value* in
  §5.2.1.1.3 (step 3: `&`, `"`, `<`, U+0009, U+000A, U+000D; not `>` — the Note under the list is ruled on in D-6).
  Neither replaces U+00A0, so D-1's function on this path would newly emit `&nbsp;`, an entity XML does not define.
  And the walk's raw-text decision (P2's two read sites) does not look at which entry is running, so on this path a
  Text child of any `RAW_TEXT_ELEMENTS` parent (script, style, …) is emitted literally — e.g. a `script` whose text
  is `</script><img src=x onerror=f()>` serializes as `<script></script><img src=x onerror=f()></script>` (follows
  from P2's read at `markup.rs:445`) — although §5.2.1.5 reads no parent and escapes every Text node (steps 3-5); the
  literal branch is §13.3 step 5.2's, an algorithm this entry does not run.

### §0.3 Decisions (this PR)

- **D-6 — the XMLSerializer path escapes with DOM Parsing's two algorithms, one function each (PM 785, option
  (b2)).** `serialize_node_to_string` keeps the HTML walk, but the two escapes it applies are the XML
  serialization's own (P7):
  - `push_xml_text(out: &mut String, data: &str)` (NEW) = DOM Parsing §5.2.1.5 *XML serializing a Text node*
    (`#dfn-xml-serializing-a-text-node`) steps 3-5: `&` → `&amp;`, `<` → `&lt;`, `>` → `&gt;`. Nothing else is
    replaced (U+00A0 and `"` stay).
  - `push_xml_attribute_value(out: &mut String, value: &str)` (NEW) = *serializing an attribute value*
    (`#dfn-serializing-an-attribute-value`, in DOM Parsing §5.2.1.1.3 *Serializing an Element's attributes*) step 3:
    `&` → `&amp;`, `"` → `&quot;`, `<` → `&lt;`, U+0009 → `&#9;`, U+000A → `&#xA;`, U+000D → `&#xD;`. Nothing else
    is replaced (`>` and U+00A0 stay).
  - **`>` in an attribute value — ruling.** Step 3's list, the normative text, has no `>`; the Note under it says
    the algorithm goes beyond XML's AttValue grammar "by also replacing ">" characters". The two disagree, and
    elidex's basis order decides by the normative text: the numbered steps are normative, a Note is informative,
    so `>` is **not** replaced (XA2, X3). The Note's stated ground is an observation about existing
    implementations, which is not a basis for core behaviour (CLAUDE.md, basis order), so it is not weighed.
    Safety does not depend on `>` (I-1 × I-2, §2). If DOM Parsing moves `>` into the steps, the change is one arm
    plus flipping XA2's `>` cell and X3. The docstring of `push_xml_attribute_value` states this ruling and its
    basis (DoD 4).
  - **Which part of each algorithm.** Both functions are the algorithm as run with *require well-formed* = false,
    the only value an elidex caller passes (HTML §8.5.9 step 1). The steps that act only when it is true (Text
    step 2, attribute-value step 1: throw if the string has a character outside XML's Char production) run before
    any replacement and are not written here, because no caller would reach them; attribute-value step 2 (null →
    the empty string) has no input (`&str`). A caller that passes true puts its check in front of the same function.
    Each docstring states this, with the § number and anchor from §0.4.
  - **Home.** `crates/dom/elidex-dom-api/src/xml_serialization.rs` (NEW, `pub(crate) mod`), named after DOM Parsing
    §5.2.1 *XML Serialization*, at the crate's top level and not under `element/`: §5.2.1.5 is a Text-node algorithm
    and the section's siblings (§5.2.1.2 Document … §5.2.1.8 ProcessingInstruction) are not element-scoped, so this
    is the module `#11-domparser-xml-real-parsing`'s serializer grows in, calling both functions as they are.
  - **Selection, made once.** A private `enum CharacterEscaping { HtmlFragment, XmlSerialization }` (NEW,
    `markup.rs`) is fixed by each public entry and carried through the walk unchanged: `serialize_node_to_string`
    passes `XmlSerialization` (in both of its arms), the three §13.3 entry points pass `HtmlFragment`. It is read in
    exactly two private functions, `push_text_data(walk, out, data, in_raw_text)` and `push_attribute_value(walk,
    out, value)` (NEW); the walk's three write sites (Text, `is` value, attribute loop) call those two. Each is one
    `match` with one arm per family:
    - `push_text_data`: `HtmlFragment` → HEAD's Text branch, unchanged — the data verbatim if `in_raw_text`, else
      `escape_html` (P1); `XmlSerialization` → `push_xml_text`, which does **not** read `in_raw_text` (§5.2.1.5 has
      no parent test). So a Text node is literal iff the family is `HtmlFragment` ∧ the walk's `in_raw_text` — the
      selection gates the raw-text decision by that one conjunct, and the raw-text decision is read only on the
      `HtmlFragment` family (I-2, umbrella §2 matrix).
    - `push_attribute_value`: `HtmlFragment` → `escape_attr` (P1), unchanged; `XmlSerialization` →
      `push_xml_attribute_value`.

    It is not a field of the public `SerializeOptions` (`getHTML`'s options), so no caller of a §13.3 entry point can
    select XML escaping.
  - **Staging (umbrella S-2).** In this PR the `HtmlFragment` arms keep HEAD's behaviour: `escape_html` /
    `escape_attr` (P1), and an `in_raw_text` computed by HEAD's two `RAW_TEXT_ELEMENTS` reads (P2), noscript and the
    namespace-blind match included. The three §13.3 entries' output is therefore byte-identical to HEAD's. PR-H
    replaces the bodies of those two arms (D-1) and the predicate behind `in_raw_text` (D-2..D-4); it does not
    touch the `XmlSerialization` arms, the enum, `Walk` or `xml_serialization.rs`. **After this PR the XML family's
    output does not depend on the raw-text list** — neither on HEAD's `RAW_TEXT_ELEMENTS` nor on what PR-H makes of
    it — because the `XmlSerialization` arm reads no `in_raw_text`. That is what leaves PR-H a change on two axes,
    (i) × (ii), on one family.
  - **Basis.** Spec: HTML §8.5.9 step 1 + the two DOM Parsing algorithms (P7). One issue, one way, read as *one
    function per spec algorithm*: each DOM Parsing algorithm has exactly one function, and a family's Text has
    exactly one algorithm (§13.3 step 5.2 or §5.2.1.5, never a mix); §13.3's *escaping a string* gets its one form
    at PR-H (D-1; umbrella I-1, staging). Security by structure: the raw `<` in XMLSerializer attribute values and
    the literal Text under a raw-text parent on that path (P7) are both closed — no input reaches a literal Text
    through XMLSerializer.
  - **Candidates rejected (PM 785).** (b1) keep HEAD's two escapers for the XMLSerializer path: leaves a second
    copy of §13.3-like escaping, makes I-1 false, and keeps the attribute `<` defect. (b3) land the full XML
    serializer first: beyond PM 724's scope, and delays the mXSS fix. Accepting `&nbsp;` on this path (Rev 4): a
    regression of well-formedness. An XML mode on PR-H's `push_escaped_string` (NEW: PR-H): a second form of
    §13.3's algorithm, and neither DOM Parsing algorithm is *escaping a string* with a flag (different character
    sets). Keeping the literal raw-text branch on this path until `#11-domparser-xml-real-parsing` (Rev 5): a
    deviation from §5.2.1.5 in the unsafe direction, deferred to a slot whose trigger does not cover it, with no
    cell pinning it — while the fix is the one match arm above.
  - **What does not change on this path.** The rest of what the walk decides: element / comment / doctype output
    and the void check. The walk still computes `in_raw_text` on both families (the threading is shared, P2; from
    HEAD's reads in this PR, from D-2's predicate after PR-H); only the `XmlSerialization` arm ignores it. Those
    stay with `#11-domparser-xml-real-parsing` (§9). Every Text — a noscript's, a script's or style's, a bare Text
    root — is escaped on this path by `push_xml_text`, which is exact for §5.2.1.5 (not PR-H's R1 approximation;
    X4).

### §0.4 Spec citation table

| Cite | Title | Anchor | Lookup (2026-11-15) |
|---|---|---|---|
| HTML §8.5.4 / §8.5.5 / §8.5.6 / §8.5.7 | HTML serialization methods / The innerHTML property / The outerHTML property / The insertAdjacentHTML() method | `#html-serialization-methods`, `#the-innerhtml-property`, `#the-outerhtml-property`, `#the-insertadjacenthtml()-method` | `webref heading html 8.5.4` … `8.5.7` (2026-11-18; `8.5.3` → *HTML streaming methods*, which `markup.rs`'s header names as §8.5.3 *HTML serialization methods* at `c0578223`; the header's numbers fixed by this PR, §4.1) |
| HTML §8.5.8 | The createContextualFragment() method | `#the-createcontextualfragment()-method` | `webref heading html 8.5.8` (the number the code's XMLSerializer cites carry today; fixed to §8.5.9 by this PR, §4.1 / §4.2 / §9) |
| HTML §8.5.9 | The XMLSerializer interface | `#the-xmlserializer-interface` | `webref heading html 8.5.9`; `webref body html the-xmlserializer-interface` (`serializeToString(root)` step 1: "Return the XML serialization of root given false.") |
| DOM Parsing §5.2.1 | XML Serialization | `#xml-serialization` | `webref heading DOM-Parsing 5.2.1` (2026-11-18; the module name of D-6; subsections §5.2.1.1 … §5.2.1.8) |
| DOM Parsing §5.2.1.5 | XML serializing a Text node | `#xml-serializing-a-text-node` (heading), `#dfn-xml-serializing-a-text-node` (algorithm) | `webref heading DOM-Parsing 5.2.1.5`; `webref dfn DOM-Parsing 'XML serializing a Text node'` → §5.2.1.5 `#dfn-xml-serializing-a-text-node`; `webref body DOM-Parsing dfn-xml-serializing-a-text-node` (2026-11-18: steps 1-6; 2 = the *require well-formed* check; 3-5 = "&" → "&amp;", "<" → "&lt;", ">" → "&gt;") |
| DOM Parsing §5.2.1.1.3 | Serializing an Element's attributes | `#serializing-an-element-s-attributes` (heading), `#dfn-serializing-an-attribute-value` (algorithm *serializing an attribute value*) | `webref heading DOM-Parsing 5.2.1.1.3`; `webref dfn DOM-Parsing 'serializing an attribute value'` → §5.2.1.1.3 `#dfn-serializing-an-attribute-value`; `webref body DOM-Parsing dfn-serializing-an-attribute-value` (2026-11-18: step 1 = the *require well-formed* check; 2 = null → empty string; 3 = replace "&", U+0022, "<", U+0009 → "&#9;", U+000A → "&#xA;", U+000D → "&#xD;"; ">" is not in the list; the Note under step 3 says ">" is also replaced — informative, so not followed: the steps decide, D-6 ruling) |

HTML §13.3 *Serializing HTML fragments* (the algorithm this entry does not run; step 5.2's literal branch) and
HTML §2.1.3 are cited from PR-H's table (PR-H §0.4), which holds their lookups.

`webref coverage-map DOM-Parsing 5.2.1.5` and `… 5.2.1.1.3` → each spec = 1, step = 1, "ok → single PR scope"
(2026-11-18). §8.5.4-§8.5.8 and DOM Parsing §5.2.1 are definitional or context cites (the header fix, the module
name); this PR implements HTML §8.5.9 step 1's choice of serialization as far as escaping goes, DOM Parsing §5.2.1.5
steps 3-5 and *serializing an attribute value* step 3.

## §1. Shape and verdict

One PR, one algorithm crate (`elidex-dom-api` serializer) plus VM end-to-end cells and comment-only edits in one
native file. It is a terminal slice of the umbrella (S-1, S-2): it introduces axis (iii) and makes axis (i) exact
on the XML family, with axis (ii) held at HEAD and gated off that family. No scripting input, no new ECS state, no
event / mutation / identity interaction. One normal `/elidex-plan-review`.

## §2. Invariants

I-1 (with its staging: what holds between this PR and PR-H), I-3 and the three-axis pair matrix are the umbrella's
(§2). This PR pins the matrix's (ii) × (iii) row (XC1 vs X4), the `XmlSerialization` side of (i) × (iii) (XT, XA,
X1-X3, X5) and the `XmlSerialization` side of (i) × (ii) (X5). Its own invariants:

- I-2 (raw text), **`XmlSerialization`** (`serialize_node_to_string`): no Text is emitted literally — every Text,
  whatever its parent (including a script / style / … parent), goes through `push_xml_text` (§5.2.1.5 reads no
  parent; D-6, X4). The walk computes `in_raw_text` on both families; only `push_text_data`'s `HtmlFragment` arm
  reads it. A bare Text node passed to `serialize_node_to_string` (`XMLSerializer.serializeToString(text)`) is
  escaped without consulting its parent (the walk starts with `in_raw_text = false`, `markup.rs:162`). That path's
  ground is not §13.3 (which takes "a DOM Element, Document, or DocumentFragment" as *the node*): HTML §8.5.9
  `serializeToString(root)` step 1 returns "the XML serialization of root given false", and a Text root is
  serialized by DOM Parsing §5.2.1.5 *XML serializing a Text node*, which escapes the data and reads no parent.
  Since D-6 that is the function this entry runs (`push_xml_text`), so the bare-Text output is exact for §5.2.1.5
  (at HEAD it was `escape_html`, which has the same three replacements).
- I-2, **`HtmlFragment`, in this PR**: unchanged from HEAD — a Text is literal iff HEAD's read says so (P2's HEAD
  literal set). XC1 pins one member of that set (script) through both read sites; the exact predicate is PR-H's
  (PR-H §2).
- I-1 × I-2, **`XmlSerialization`**: an attribute value inside a noscript is serialized with `<` escaped and `>`
  raw (*serializing an attribute value*, D-6 ruling). Re-parsing the output in a scripting-enabled context (where
  noscript is RAWTEXT) closes the noscript only at a `</noscript` sequence, which needs a raw `<`, so the argument
  holds through `<` alone and does not depend on `>`. X5 pins it (`</noscript>` in an attribute value →
  `&lt;/noscript>`). The `HtmlFragment` side is PR-H's (V5).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.5.9 The XMLSerializer interface | `serializeToString(root)` step 1 | "the XML serialization of root given false" → the entry's escaping family is DOM Parsing's, *require well-formed* = false | `serialize_node_to_string` builds a `Walk` with `CharacterEscaping::XmlSerialization` and uses it in both of its arms | ✗ (escaping only; the XML serialization's walk is `#11-domparser-xml-real-parsing`) | yes |
| W3C DOM Parsing §5.2.1.5 XML serializing a Text node | steps 3-5 | *require well-formed* = false, every Text whatever its parent — an HTML raw-text parent included (`serialize_node_to_string`) | `push_xml_text`, the `XmlSerialization` arm of `push_text_data` (does not read `in_raw_text`) | ✗ (step 2, *require well-formed* = true: no caller; `#11-domparser-xml-real-parsing`) | yes |
| W3C DOM Parsing §5.2.1.1.3 Serializing an Element's attributes | *serializing an attribute value* step 3 | *require well-formed* = false, non-null value (`serialize_node_to_string`) | `push_xml_attribute_value`, the `XmlSerialization` arm of `push_attribute_value` | ✗ (step 1, *require well-formed* = true: no caller; step 2, null: no input; the rest of §5.2.1.1.3 — prefixes, namespace declarations — is `#11-domparser-xml-real-parsing`) | yes |

**Breadth**: K=2 specs (HTML, DOM Parsing), M=3 entries (verified 2026-11-18: preflight `total entries (M): 3`; the
two DOM Parsing rows are "unmapped-label" for
preflight, whose label table has no DOM Parsing entry, so their citations are verified by the §0.4 lookups
instead). **Split decision**: single PR (a slice of the umbrella).

### §3.1 User-input touch audit

- Text data, attribute values and `is` values are author-controlled (`textContent`, `setAttribute`,
  `createElement(.., {is})`, any parse). This PR changes what one native emits: `XMLSerializer.serializeToString`
  (`crates/script/elidex-js/src/vm/host/dom_parser.rs:404`). The native's code is not edited (§4.2); exposure
  decreases through the shared serializer. Text output is unchanged (the same three replacements as HEAD's
  `escape_html`) except under a parent of P2's HEAD literal set — script, style, xmp, iframe, noembed, noframes or
  noscript, **in any namespace** — whose Text was literal at HEAD and is now escaped (P7 defect; X4 pins script and
  style, and the arm reads no parent, so the family-gate mutant of §6 covers the rest). A plaintext parent's Text
  was already escaped at HEAD and stays so. Attribute values gain `<` → `&lt;` (the P7 defect) and the U+0009 /
  U+000A / U+000D character references; U+00A0 stays raw.
- The three §13.3 natives (`innerHTML` / `getHTML` / `outerHTML` getters,
  `crates/script/elidex-js/src/vm/host/dom_inner_html.rs:116`, `:317`, `:339`) reach the same walk through the
  `HtmlFragment` arms, whose output this PR does not change (D-6, staging); exposure unchanged until PR-H.
- Adjacent pre-existing: the Comment branch (`markup.rs:382-386`, data verbatim per spec) and attribute-name
  filtering (`is_safe_attr_name`, `markup.rs:267`) — untouched; exposure unchanged.

## §4. Mechanism

### §4.1 `elidex-dom-api` — `markup.rs`

- `CharacterEscaping` + `push_text_data` / `push_attribute_value` (NEW, private), D-6; the Text write site passes
  its `in_raw_text` to `push_text_data`, whose `HtmlFragment` arm alone reads it. The walk's `opts:
  &SerializeOptions` parameter becomes one private `walk: &Walk<'_>` (NEW: `{ opts: &SerializeOptions, escaping:
  CharacterEscaping }`) in `serialize_node`, `emit_own_shadow_root_if_needed` and `emit_shadow_root_template`, so the
  selection travels with the options and no call site can drop it. The three §13.3 entry points build a `Walk` with
  `HtmlFragment`; `serialize_node_to_string` builds one with `XmlSerialization` and default options before its
  `NodeKind` match and uses it in both arms (it calls the private children / outer walks directly, no longer the
  public `serialize_inner_html` / `serialize_outer_html`).
- The `HtmlFragment` arms call `escape_html` / `escape_attr` as HEAD's three write sites did; the `use
  crate::util::{escape_attr, escape_html}` import, `RAW_TEXT_ELEMENTS` and its two reads are not edited.
- `serialize_node_to_string`'s docstring and the module header are rewritten for D-6: the XMLSerializer cite becomes
  HTML §8.5.9 (both say §8.5.8 at `c0578223`, `markup.rs:7` and `:168`; §8.5.8 is *The createContextualFragment()
  method*), and the docstring says the walk is the HTML one with DOM Parsing's two escapes and every Text escaped
  (§0.4 numbers). The header's other numbers are stale by the same renumbering and are fixed in this PR, in the same
  header block: at `c0578223` it pairs `§8.5.3` with *HTML serialization methods*, `§8.5.4` with *The innerHTML
  property*, `§8.5.5` with *The outerHTML property* and `§8.5.6` with *The insertAdjacentHTML() method*; they are
  now **§8.5.4** *HTML serialization methods*, **§8.5.5** *The innerHTML property*,
  **§8.5.6** *The outerHTML property*, **§8.5.7** *The insertAdjacentHTML() method*
  (`webref heading html 8.5.3` … `8.5.9`, 2026-11-18: §8.5.3 is now *HTML streaming methods*, §8.5.8 *The
  createContextualFragment() method*, §8.5.9 *The XMLSerializer interface*). The rewritten header writes each
  number with its title on one line (DoD 4's check).
- `SerializeOptions`, the public entry points' signatures, `GetInnerHtml` and the re-export lists are unchanged.

### §4.1a `elidex-dom-api` — `xml_serialization.rs` (NEW)

`push_xml_text` and `push_xml_attribute_value` (`pub(crate)`), D-6; `lib.rs` gains `pub(crate) mod
xml_serialization;` and the `#[cfg(test)] mod xml_serialization_tests;` line. Each is one per-character `match` that
writes into the output buffer (single pass, like D-1). Unit cells live in the sibling
`xml_serialization_tests.rs` (the crate's `*_tests.rs` convention: `class_list_tests.rs`, `dialog_tests.rs`), which
also keeps DoD 1's entity grep free of test strings.

### §4.2 `elidex-js` — no native code change

The four natives' code is not edited (D-6: the selection is made inside `serialize_node_to_string`). Comments only,
in `dom_parser.rs`: the module doc (`:1`, "`XMLSerializer` (HTML
§8.5.8)", and `:37-41`), the `serializeToString` docstring (`:322-…`, "HTML §8.5.8 … This is HTML serialization (boa parity)") and the inline
comment above the call (`:397-403`) describe the old escaping and the old section number; they are reworded to "HTML
§8.5.9; the HTML walk with DOM Parsing's two escapes, every Text escaped (D-6); the rest of the XML serialization is
`#11-domparser-xml-real-parsing`". `elidex-script-session` is not edited.

### §4.3 ECS-native / layering check

No side-store, no new component, no new state read. OO → ECS mapping of the concepts this PR touches:

| Spec / OO concept | elidex (ECS) | This PR |
|---|---|---|
| Text node's data | `TextContent` component | read (existing) |
| *XML serializing a Text node* / *serializing an attribute value* (*require well-formed* = false) | `push_xml_text` / `push_xml_attribute_value` — pure functions, no state | NEW |
| "which serialization this entry is" (HTML fragment vs XML) | `CharacterEscaping`, a value fixed by the entry and passed down the walk — not stored on any entity or document | NEW |

Algorithms live in `elidex-dom-api` (the walk and its selection in `element/markup.rs`, DOM Parsing §5.2.1 in
`xml_serialization.rs`); `vm/host/` code is untouched (comments only, §4.2).

## §5. Populations

### §5.1 Escaping callers census

`git grep -n -w 'escape_attr\|escape_html' -- crates` → 11 hits (verified 2026-11-15): definitions `util.rs:154`,
`:168` (2); import `markup.rs:14` (1); production calls `markup.rs:373` (Text), `:421` (`is` value), `:436`
(attribute loop) (3); comment `markup.rs:411` (1); test message `tests_markup.rs:200` (1); unit asserts
`util.rs:225`, `:226`, `:231` (3). In this PR the 3 production calls become the two dispatch functions — `:373` →
`push_text_data`, `:421` and `:436` → `push_attribute_value` — whose `HtmlFragment` arms hold the only remaining
calls (one `escape_html`, one `escape_attr`) and whose `XmlSerialization` arms call D-6's pair (umbrella I-1,
staging; I-3). The definitions, the import, the comment, the test message and the unit asserts are not edited here
(PR-H deletes or rewords them), so the same grep gives 10 hits after this PR (11 − 1: three calls became two;
DoD 1).
Concept sweep for other markup-escaping code at the base: `git grep -n "'&' =>\|&amp;\|&quot;" -- crates ':!*tests*'
':!crates/dom/elidex-html-parser*'` → only `util.rs` (and an unrelated JS-lexer `b'&'`); adding the character
references of D-6 to the pattern (`&#9;\|&#xA;`) adds no hit (2026-11-18). So the base has exactly the two copies of
P1, and after this PR the same sweep finds them plus the two functions of `xml_serialization.rs` (DoD 1).

### §5.2 The walk's `in_raw_text` computations — unchanged

The census of the raw-text reads and of the five `serialize_node` calls is PR-H §5.2 (measured at `c0578223`, this
PR's base). D-6 changes none of these computations: `serialize_node_to_string`
reaches the same five calls with `XmlSerialization` in its `Walk` (§4.1), so the `in_raw_text` each call computes is
identical on both families; what differs is whether it is read — only `push_text_data`'s `HtmlFragment` arm reads it,
so on `XmlSerialization` no Text is literal (I-2).

### §5.3 Files touched (`wc -l` at `c0578223`)

| File | Lines | Change |
|---|---|---|
| `crates/dom/elidex-dom-api/src/element/markup.rs` | 459 | D-6's selection, `Walk`, the two dispatch functions, header / docstring cites (~+45) |
| `crates/dom/elidex-dom-api/src/xml_serialization.rs` (NEW) | — | D-6's two functions (~60) |
| `crates/dom/elidex-dom-api/src/xml_serialization_tests.rs` (NEW) | — | XT / XA cells (~90) |
| `crates/dom/elidex-dom-api/src/lib.rs` | 132 | two `mod` lines |
| `crates/dom/elidex-dom-api/src/element/tests_markup.rs` | 246 | XC1 (~+25) |
| `crates/script/elidex-js/src/vm/host/dom_parser.rs` | 407 | comments only (§4.2) |
| `crates/script/elidex-js/src/vm/tests/tests_html_serialize_escape.rs` (NEW) | — | X1-X5 (~80); PR-H adds its V cells to this file |
| `crates/script/elidex-js/src/vm/tests/mod.rs` | 210 | `mod tests_html_serialize_escape;` |

## §6. Test and coverage plan

Supported surface: the XMLSerializer path's two DOM Parsing escapes and the family gate, guarded by
engine-independent unit cells in `elidex-dom-api` and VM end-to-end cells; no WPT subset is added by this PR.

**Unit, `xml_serialization_tests.rs` (NEW)** — the two functions themselves, one cell per replacement arm:
- XT1 `push_xml_text`, one cell per replaced character: `&` → `&amp;`, `<` → `&lt;`, `>` → `&gt;`.
- XT2 `push_xml_text`, not replaced (one cell per character, output equals input): `"`, U+00A0, U+0009, U+000A,
  U+000D.
- XT3 `push_xml_text` composition, exact output: `a&<>\u{a0}"b` → `a&amp;&lt;&gt;\u{a0}"b`.
- XA1 `push_xml_attribute_value`, one cell per replaced character: `&` → `&amp;`, `"` → `&quot;`, `<` → `&lt;`,
  U+0009 → `&#9;`, U+000A → `&#xA;`, U+000D → `&#xD;`.
- XA2 `push_xml_attribute_value`, not replaced (one cell per character): `>`, U+00A0.
- XA3 `push_xml_attribute_value` composition, exact output: `a&"<\t\n\r>\u{a0}b` →
  `a&amp;&quot;&lt;&#9;&#xA;&#xD;>\u{a0}b`.

**Unit, `tests_markup.rs`** — the contrast cell of the (ii) × (iii) pair (NEW in Rev 7; Rev 6's contrast was U4,
which is PR-H's and whose new row is plaintext, not literal at HEAD):
- **XC1** a parent that is literal at HEAD stays literal on the `HtmlFragment` family: an HTML `script` element
  with Text data `<&>`, **through both read sites** — XC1-entry `serialize_inner_html(script, ..)` → exactly `<&>`
  (read site `markup.rs:243`), XC1-recursion `serialize_outer_html(script, ..)` → exactly `<script><&></script>`
  (read site `:445`). Green at HEAD, after this PR and after PR-H (script is in §13.3 step 5.2's unconditional
  list, so the expectation is spec-exact, not a pin of a HEAD defect). Functions:
  `xc1_script_text_literal_entry`, `xc1_script_text_literal_recursion`. PR-H generalises these two functions over
  the seven parents of its list; they are U4 from then on (no duplicate assertion remains).

**VM end-to-end, `tests_html_serialize_escape.rs` (NEW)** (bound VM; real parse):
- X1-X5, the XMLSerializer path (D-6; X1-X3 are PM 785's three cells, X4 and X5 were added by Rev 6; elements built
  by `createElement` / `setAttribute` / `textContent`, `s = new XMLSerializer()`):
  - **X1** U+00A0 is emitted raw, in Text and in an attribute value: `title` = `'x\u00a0y'`, text `'p\u00a0q'` →
    `s.serializeToString(div) === '<div title="x\u00a0y">p\u00a0q</div>'`, plus two separate assertions that the
    output contains no `&nbsp;` before and after the first `>` (so a one-site mutant is attributable). Root = the
    Element (the outer arm of `serialize_node_to_string`).
  - **X2** an attribute `<` becomes `&lt;`: `title` = `'a<b'`, the `div` appended to a `DocumentFragment` →
    `s.serializeToString(frag) === '<div title="a&lt;b"></div>'`. Root = the fragment (the children arm), so both
    arms of the entry are covered.
  - **X3** `>` in an attribute value is not replaced: `title` = `'a>b'` → `s.serializeToString(div) === '<div
    title="a>b"></div>'`.
  - **X4** Text under an HTML raw-text parent is escaped (§5.2.1.5; the (ii) × (iii) cell). Detached elements built
    by `createElement` (never connected, so nothing runs). Element root (outer arm): `sc` = a `script` with
    `textContent = '</script><img src=x onerror=f()>'` → `s.serializeToString(sc) ===
    '<script>&lt;/script&gt;&lt;img src=x onerror=f()&gt;</script>'`. Fragment root (children arm): `st` = a `style`
    with `textContent = '</style><img src=x onerror=f()>'`, appended to a `DocumentFragment` →
    `s.serializeToString(frag) === '<style>&lt;/style&gt;&lt;img src=x onerror=f()&gt;</style>'`. For each output
    `o`, `d.innerHTML = o` on a page `div` → `d.querySelector('img') === null`. Red on old code: the Text was literal,
    so the re-parse ends the script / style at the embedded end tag and builds the `img`.
  - **X5** I-1 × I-2 on `XmlSerialization`: a `noscript` with a `p` child whose `title` is `'</noscript><img src=x
    onerror=f()>'` → `s.serializeToString(ns) === '<noscript><p title="&lt;/noscript>&lt;img src=x
    onerror=f()>"></p></noscript>'` (`<` escaped, `>` raw), and `d.innerHTML` of that output on a page `div` (noscript
    RAWTEXT) → `d.querySelector('img') === null`. Red on old code: `escape_attr` left `<` raw, so the page parse ends
    the noscript inside the attribute.

X1 and X3 are green at HEAD too (HEAD's escapers leave U+00A0, and an attribute value's `>`, raw, as DOM Parsing
does); they pin the family against a later change of the §13.3 escapers (PR-H's U+00A0 → `&nbsp;` and attribute
`>` → `&gt;`) and are the cells the `&nbsp;` / `>` arm mutants turn red. X2, X4 and X5 are red at HEAD.

VM test function names: `x1_xml_serializer_nbsp_raw`,
`x2_xml_serializer_attr_lt_escaped`, `x3_xml_serializer_attr_gt_raw`, `x4_xml_serializer_raw_text_parent_text_escaped`,
`x5_xml_serializer_noscript_attr_lt_escaped`.

The X / XT / XA / XC cells are not accepted regressions and name no slot.

**Existing tests whose expectations change** — XMLSerializer path (D-6): `git grep -n 'serializeToString' --
crates/script/elidex-js/src/vm/tests` → 13 lines, all in `tests_dom_parser.rs` (2026-11-18); the ones that assert
output use `class="test"`, `id="f"`, text `hello world` / `hi`, a comment and brand / argument errors — no `<`, `>`,
`&`, `"`, U+00A0, U+0009, U+000A or U+000D inside a serialized Text or attribute value, and none of the 11 test
functions that call `serializeToString` builds or parses a script / style / xmp / iframe / noembed / noframes /
noscript element — P2's HEAD literal set, the parents whose Text this PR stops emitting literally — in any
namespace (checked per function at `c0578223`, re-checked 2026-11-18 with noscript in the set: the 11 are the ten
`xml_serializer_*` functions, `dom_parser_and_serializer_no_op_after_unbind_without_throwing` and no other; the
file's noscript tests, `tests_dom_parser.rs:516-637`, call no `serializeToString`), so X4's change meets no existing
expectation. The §13.3 entries' output is unchanged (D-6, staging), so no test of `innerHTML` / `outerHTML` /
`getHTML` is affected. **No expectation flips.**

**Mutants** (each must turn a named cell red):
- D-6's functions: drop each of `push_xml_text`'s 3 arms (XT1) and each of `push_xml_attribute_value`'s 6 arms
  (XA1); add a `>` arm to `push_xml_attribute_value` (XA2, X3); add a U+00A0 → `&nbsp;` arm to either (XT2 / XA2,
  X1); add a `"` arm to `push_xml_text` (XT2); drop the `<` arm of `push_xml_attribute_value` (XA1, X2, X5).
- D-6's selection, against the escapers the `HtmlFragment` arms call in this PR: `serialize_node_to_string` passes
  `HtmlFragment` in its outer arm (X4's element half, X5 red; X2, X4's fragment half green) or in its children arm
  (X2, X4's fragment half red; X4's element half, X5 green); `push_attribute_value`'s `XmlSerialization` arm calls
  `escape_attr` (X2, X5). Under the two entry mutants X1 and X3 stay green in this PR, for the reason given above;
  Rev 6 listed them red because its `HtmlFragment` arms were already `push_escaped_string`'s — that form of the
  mutants is re-run at PR-H (PR-H §6).
- **The family gate** (the (ii) × (iii) conjunct): drop it, so the `XmlSerialization` arm emits the data verbatim
  when `in_raw_text` (X4, both halves, red; XC1 green); and the reverse, the `HtmlFragment` arm ignoring
  `in_raw_text` (XC1-entry, XC1-recursion red; X4 green).
- **Not observable in this PR, and what checks each instead.** (a) `push_text_data`'s `XmlSerialization` arm calls
  `escape_html` on every Text (still ignoring `in_raw_text`): `escape_html` and `push_xml_text` have the same three
  replacements (P7), so no output differs. DoD 1's grep requires the one `escape_html` call to be in the
  `HtmlFragment` arm; at PR-H the function is deleted, and Rev 6's form of this mutant (the arm calling
  `push_escaped_string`; X1's Text assertion red) is PR-H's. (b) The reverse, an `HtmlFragment` arm calling the DOM
  Parsing function: for Text the output is identical for the same reason; for an attribute value the §13.3
  entries' output would change (`<` and the three character references) with no cell of this PR red — DoD 1's grep
  (one `push_xml_attribute_value` call, in the `XmlSerialization` arm; one `escape_attr` call, in the
  `HtmlFragment` arm) is this PR's check, and PR-H's V6 / U2 are the executable one (PR-H §6). Whether this PR
  should carry a cell for it is Q-5.

## §7. Accepted regressions

None. The XMLSerializer path's Text output changes only under a parent of P2's HEAD literal set (script, style, xmp,
iframe, noembed, noframes, noscript, in any namespace), from literal to §5.2.1.5's escaping (X4), and its
attribute-value output moves to DOM Parsing's algorithm (D-6; X1-X3, X5) — both toward the spec. The §13.3 entries'
output does not change. (The program's accepted regressions R1 / R2 are PR-H's, on the §13.3 entries.)

## §8. 1000-line audit

All touched files are under 1000 lines at `c0578223` (§5.3). VM cells go to a NEW file rather than
`tests_dom_parser.rs` (823) or `tests_inner_html_mixin.rs` (749), which would approach the line.

## §9. Out of scope, and slots

- The `HtmlFragment` family — §13.3's *escaping a string*, the raw-text list, noscript, the namespace conjunct:
  **PR-H**, same slot (`#11-html-fragment-serializer-conformance` (d) / (e)). This PR leaves the §13.3 entries'
  mXSS defects open; the window and its bound are umbrella S-3.
- The XMLSerializer cite "HTML §8.5.8" (`markup.rs:7`, `:168`; `dom_parser.rs:1`, `:322`) is fixed to §8.5.9 by this PR,
  which rewrites those comments for D-6 (§4.1, §4.2). The other section numbers in `markup.rs`'s module header,
  stale by the same renumbering, are fixed in the same header block by this PR (§8.5.4-§8.5.7, §4.1; DoD 4).
- The rest of the XML serialization: `#11-domparser-xml-real-parsing` (PM 785). This PR lands only its two escaping
  functions (D-6). Staying there: the XML walk itself — void self-closing, namespace prefixes and declarations (the
  remainder of §5.2.1.1.3), the Attr-root contract, the doctype's public / system identifiers — and the
  *require well-formed* = true checks. Not staying there: the Text under an HTML raw-text parent on the
  XMLSerializer path — escaped by this PR (D-6, I-2, X4), since keeping §13.3's literal branch on this path was a
  deviation from §5.2.1.5 in the unsafe direction.

## §10. Definition of done

1. D-6 implemented as §4. Exactly two NEW escaping functions, each called from exactly one arm: `git grep -n -w
   'push_xml_text\|push_xml_attribute_value' -- crates ':!*tests*'` → apart from comment lines, 2 definitions + 2
   calls, the calls inside the `XmlSerialization` arms of `push_text_data` / `push_attribute_value`. HEAD's two
   escapers each called from exactly one arm: `git grep -n -w 'escape_attr\|escape_html' -- crates` → the 10 hits
   of §5.1, the 2 calls inside the `HtmlFragment` arms. Entities: `git grep -n -F -e '&amp;' -e '&lt;' -e '&gt;' -e
   '&quot;' -e '&nbsp;' -e '&#9;' -e '&#xA;' -e '&#xD;' -- crates ':!*tests*' ':!crates/dom/elidex-html-parser*'` →
   apart from comment lines, every hit is in `util.rs` (unchanged from the base) or inside the body of
   `push_xml_text` / `push_xml_attribute_value` (`xml_serialization.rs`), and `&nbsp;` has no hit.
   `CharacterEscaping::XmlSerialization` (NEW) is constructed only in `serialize_node_to_string` (`git grep -n -F
   'XmlSerialization' -- crates ':!*tests*'`: apart from comment lines, the variant, the two `match` arms and one
   construction). `in_raw_text` is read for output only in `push_text_data`'s `HtmlFragment` arm (inspection of the
   `match`; X4, XC1 and the family-gate mutants of §6 are the executable check). The raw-text list is untouched:
   `git grep -n -w 'RAW_TEXT_ELEMENTS' -- crates` → the base's 3 hits (definition + 2 reads), and `git diff
   origin/main -- crates/dom/elidex-dom-api/src/util.rs` → empty. `git diff origin/main --
   crates/script/elidex-script-session` → empty; `git diff origin/main -- crates/script/elidex-js/src/vm/host`
   touches `dom_parser.rs` only and every changed line is a comment: `git diff origin/main --
   crates/script/elidex-js/src/vm/host | /usr/bin/grep -E '^[+-]' | /usr/bin/grep -v -E
   '^(\+\+\+|---)|^[+-][[:space:]]*//'` → no output.
2. Every cell of §6 present and green; every listed mutant observed red at the final head (mutation runs only at the
   final head; ≤2 in parallel).
3. `cargo test -p elidex-dom-api -p elidex-js --all-features`, then `mise run ci`.
4. The docstrings of `push_xml_text` / `push_xml_attribute_value` carry DOM Parsing §5.2.1.5
   `#dfn-xml-serializing-a-text-node` and §5.2.1.1.3 `#dfn-serializing-an-attribute-value` with the steps they
   implement and the *require well-formed* note (D-6), and `push_xml_attribute_value`'s also the `>` ruling — step
   3's list is normative and has no `>`, the Note that says `>` is replaced is informative and not followed (D-6);
   the XMLSerializer cite is HTML §8.5.9 in `markup.rs` and `dom_parser.rs`, and `markup.rs`'s module header names
   §8.5.4 / §8.5.5 / §8.5.6 / §8.5.7 (§4.1): `git grep -n -F -e '8.5.3' -e '8.5.8' --
   crates/dom/elidex-dom-api/src/element/markup.rs crates/script/elidex-js/src/vm/host/dom_parser.rs` → 0, and for
   each of the five strings `§8.5.4 HTML serialization methods`, `§8.5.5 The innerHTML property`, `§8.5.6 The
   outerHTML property`, `§8.5.7 The insertAdjacentHTML() method`, `§8.5.9 The XMLSerializer interface`: `git grep -n
   -F -e '<string>' -- crates/dom/elidex-dom-api/src/element/markup.rs | /usr/bin/grep -c -E '^[^:]+:[0-9]+://!'` →
   exactly 1. **The count is of module-header lines (`//!`) only.** `serialize_node_to_string`'s docstring (`///`)
   cites "HTML §8.5.9" and may write the title after it; it is not a `//!` line, so it is not counted and cannot
   fail the check (Rev 6 counted every line of the file, which a docstring carrying the title would have made 2).
   Numbers from §0.4's lookups only.
5. The PR body states that the §13.3 entries (`innerHTML` / `outerHTML` / `getHTML`) are unchanged by this PR and
   that their defects close at PR-H, with the window's bound (umbrella S-3).

Cross-lane pins and the landing (ledger) items of this PR: umbrella §4.2 and §5.

## §11. Open questions

- Q-4 — decided (PM 785): the XMLSerializer path uses DOM Parsing's two escaping algorithms as two functions, not a
  mode of `push_escaped_string` (D-6, option (b2)).
- Q-5 — **open (raised by the split, Rev 7)**: mutant (b) of §6 — an `HtmlFragment` arm of `push_attribute_value`
  calling `push_xml_attribute_value` — has no red cell in this PR; in Rev 6's single PR it was killed by V6 / U2,
  which are now PR-H's. This PR checks it by DoD 1's grep and PR-H kills it executably. A cell that would kill it
  here without pinning a HEAD defect exists (an attribute value containing U+0009 through `outerHTML` stays raw —
  exact for §13.3 at HEAD and after PR-H); Rev 7 does not add it, because the split was to change no design
  content beyond XC1. For the plan-review of this memo, or the PM, to decide.
