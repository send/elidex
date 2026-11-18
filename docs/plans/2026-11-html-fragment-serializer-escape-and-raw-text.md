# HTML fragment serializer — *escaping a string*, the raw-text parents, noscript always escaped (per-PR plan-memo)

Slot `#11-html-fragment-serializer-conformance` (ledger: open, owner L3). This PR closes item **(e)** and the
security part of item **(d)** (PM 724, 2026-11-13; scope re-cut by **PM 731**, option C; this facet's Re-eval
2026-11-20); items (a) (b) (c) (f) stay in the slot with Re-eval 2026-12-15, and (d)'s precise noscript rule moves to
`#11-scripting-disabled-predicate-no-document` (§9). Base: `origin/main` `c0578223` (rebased from `06bdfbbd` on
2026-11-15; `git diff --stat 06bdfbbd c0578223 -- crates/dom/elidex-dom-api crates/script/elidex-script-session
crates/script/elidex-js/src/vm/host crates/script/elidex-js/src/vm/tests/mod.rs` is empty). Every code fact below was
re-measured at `c0578223` on 2026-11-15; line numbers move, the commands do not. Owner lane: L3 DOM/form.
Security-relevant: both items are markup-injection (mXSS) surfaces of `innerHTML` / `outerHTML` / `getHTML` /
`XMLSerializer`.

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

## §0. Frame, premises, decisions

### §0.1 Frame

The HTML fragment serialization algorithm (HTML §13.3) has two decisions this PR makes exact or safe:

- **(e) Escaping a string** (`#escapingString`) — steps 1-5 (`webref body html serialising-html-fragments`, the
  "Escaping a string" list): 1 `&` → `&amp;`; 2 U+00A0 → `&nbsp;`; 3 `<` → `&lt;`; 4 `>` → `&gt;`; 5 *attribute mode
  only*: `"` → `&quot;`. Steps 1-4 apply in both modes.
- **(d) The Text-node branch of step 5.2** — "If the parent of current node is a style, script, xmp, iframe,
  noembed, noframes, or plaintext element, or if the parent of current node is a noscript element and scripting is
  enabled for the node, then append the value of current node's data literally. Otherwise, append the value of
  current node's data, escaped as described below." "*the node*" is the algorithm's input (the preamble: "takes as
  input a DOM Element, Document, or DocumentFragment referred to as the node"); when the Text node's parent is a
  noscript, the recursion that reaches it has that noscript as *the node*. This PR makes the unconditional list exact
  (adds plaintext, HTML namespace only) and takes the *otherwise* branch for every noscript (PM 731): the output is
  exact wherever scripting is disabled for the node, and an escaped (safe) approximation where it is enabled (§7 R1).

Three of the four serializer entries are §13.3's (`innerHTML` / `outerHTML` / `getHTML`). The fourth,
`XMLSerializer.serializeToString`, is not: its ground is the XML serialization (HTML §8.5.9 step 1 → DOM Parsing
§5.2.1), whose two escaping algorithms differ from (e). This PR gives that entry those two algorithms (D-6, PM 785);
the rest of its walk stays the HTML one until `#11-domparser-xml-real-parsing` (§9).

### §0.2 Premises re-measured at `c0578223`

- P1 (e): `escape_html` (`crates/dom/elidex-dom-api/src/util.rs:154`) maps `&`, `<`, `>` — no U+00A0.
  `escape_attr` (`util.rs:168`) maps `&`, `"` — no `<`, `>`, U+00A0. Two diverging copies of one spec algorithm.
  `util` is `pub(crate)` (`crates/dom/elidex-dom-api/src/lib.rs:40`), so both are crate-internal.
- P2 (d): `RAW_TEXT_ELEMENTS` (`crates/dom/elidex-dom-api/src/element/markup.rs:22`) = script, style, xmp, iframe,
  noembed, noframes, **noscript** — lacks plaintext, and noscript is unconditional. Read at `markup.rs:243` (entry,
  `serialize_inner_html_with_options`) and `markup.rs:445` (recursion, `serialize_node`), both by `TagType` string
  only — **no namespace check**, so an SVG `<style>` (foreign content, whose text the parser entity-decodes) is
  serialized literally too (see D-4).
- P3: the spec's "scripting is enabled for the node" is HTML §8.1.3.4 (`webref body html
  enabling-and-disabling-scripting`; dfn `#concept-n-script` / `#concept-n-noscript`): scripting is disabled for a
  platform object if it is disabled for the object's relevant settings object (whose sandbox condition is the
  *sandboxed scripts browsing context flag*, HTML §7.1.5 `#sandboxed-scripts-browsing-context-flag`), or the object
  implements Node and its node document's browsing context is null, or it is a Window whose associated Document's
  browsing context is null; enabled iff not disabled.
- P4: the ratified home of that answer is IBP-sandbox D8 / J12 / J13 (`docs/plans/2026-09-ibp-sandbox.md:202-205`,
  `:406`, `:408-413`): **one composition**, `EcsDom::scripting_disabled_for` (NEW: N4), over a `ScriptingSubject`;
  "no caller derives a node document for F10, evaluates a clause, or has a `None` branch"; one VM read route from N4.
  Until N4, HEAD's interim composition is `elidex_script_session::scripting_disabled_for_platform_object`
  (`crates/script/elidex-script-session/src/scripting.rs:74`), read by the event-handler compile gate
  (`crates/script/elidex-js/src/vm/host/event_handler_attrs.rs:543`); it fails **open** for a node with no node
  document (`is_some_and` at `scripting.rs:84-85`; ratified interim, IBP-sandbox `:278-284`). A serializer reading of
  that predicate would be a second composition or a second reading (Rev 1's `ScriptingContext` was both) — PM 731.
- P5: no document-level ECS fact "this Document has a browsing context" exists (`EcsDom::document_root()`,
  `crates/core/elidex-ecs/src/dom/mod.rs:491`, is a cache set by any `create_document_root` call). The node clause's
  ratified final representation is the enriched `#11-domparser-full-document-parse-fidelity` null-browsing-context
  marker (IBP-sandbox §6 OO → ECS row, `:661-663`; records `:706-710`).
- P6: the parse side of the same fact is a literal. HTML §13.4 *initialize an HTML fragment parser*
  (`#initialize-an-html-fragment-parser`, called from the *HTML fragment parsing algorithm*'s step 3) step 10: "Set
  parser's parser scripting disabled to true if scripting is disabled for contextDocument; otherwise false." elidex
  has no such reading. **Population, by property**: the production fragment-parse paths that set the parser's
  scripting flag without consulting the context document — a literal `scripting_disabled: <bool>`,
  `SetInnerHtmlOptions::default()`, `ParseFragmentOptions::default()`, or any other construction of those two option
  types (the `scripting_disabled` field of `SetInnerHtmlOptions` is forwarded to `ParseFragmentOptions` at
  `html_fragment.rs:89-91`; both `Default`s are derived, so `false`). Census (every construction of the two types, at `c0578223`):
  `git grep -n -P '\b(SetInnerHtmlOptions|ParseFragmentOptions)\s*(::default\(\)|\{)|scripting_disabled\s*:\s*(true|false)'
  -- crates ':!*tests*'` → 39 hits = 2 type definitions (`elidex-html-parser-strict/src/result.rs:102`,
  `html_fragment.rs:27`) + 1 comment (`elidex-form/src/inert_document.rs:129`) + 1 forwarding construction that reads
  its caller's value (`html_fragment.rs:89`) + 1 document (not fragment) conversion whose options' scripting field is
  never read (`elidex-html-parser/src/convert.rs:24`, `convert_document`; `convert.rs` reads only
  `opts.allow_declarative_shadow`) + 22 lines inside `#[cfg(test)]` modules (`result.rs:132-140` after `:126`;
  `html_fragment.rs:326-701` after `:277`) + **12 lines forming the 8 population members** below (a struct
  literal's constructor, `scripting_disabled` and `..default()` lines are one member). Complement: every
  non-test call of the parse / setter entry points (`git grep -n -P
  '\b(parse_fragment_progressive|parse_html_fragment|parse_fragment_strict|apply_set_inner_html|apply_set_outer_html|apply_insert_adjacent_html)\('
  -- crates ':!*tests*'`, 13 call lines outside `#[cfg(test)]`) passes a binding of, or forwards, one of those
  constructions, or takes no options and builds its own (F5, F6), so no inferred-type
  `Default::default()` lies outside the census. Paths in `crates/script/` are under
  `crates/script/elidex-js/src/vm/host/` (`dom_inner_html.rs`) or `crates/script/elidex-script-session/src/mutation/`:

  | # | Construction | Value | Entry API (production route) |
  |---|---|---|---|
  | F1 | `dom_inner_html.rs:403` `SetInnerHtmlOptions::default()` | enabled | `Element.innerHTML` setter (`set_inner_html_for` → `apply_set_inner_html`, `dom_inner_html.rs:146`) |
  | F2 | `dom_inner_html.rs:419-421` literal `scripting_disabled: false` | enabled | `Element.setHTMLUnsafe` |
  | F3 | `dom_inner_html.rs:458` `SetInnerHtmlOptions::default()` | enabled | `ShadowRoot.innerHTML` setter |
  | F4 | `dom_inner_html.rs:474-476` literal `scripting_disabled: false` | enabled | `ShadowRoot.setHTMLUnsafe` |
  | F5 | `html_fragment.rs:173` `ParseFragmentOptions::default()` | enabled | `Element.outerHTML` setter: `apply_set_outer_html`, VM `dom_inner_html.rs:361` |
  | F6 | `html_fragment.rs:204` `ParseFragmentOptions::default()` | enabled | `insertAdjacentHTML`: `apply_insert_adjacent_html`, reached only through the session flush of `Mutation::InsertAdjacentHtml` (`mutation/mod.rs:230-234`), recorded only by the `"insertAdjacentHTML"` `DomApiHandler` (`elidex-dom-api` `markup.rs:117`) |
  | F7 | `mutation/mod.rs:226` `SetInnerHtmlOptions::default()` | enabled | the session flush of `Mutation::SetInnerHtml`, recorded only by the `"innerHTML.set"` `DomApiHandler` (`markup.rs:56`) |
  | F8 | `inert_document.rs:146-148` literal `scripting_disabled: true` | disabled | DOMParser's inert build (`parse_into_inert_document`, called from `dom_parser.rs:295`) |

  F1-F7 parse with scripting enabled in every document, including a DOMParser document, where §8.1.3.4's node
  clause (P3: its browsing context is null) makes scripting disabled and step 10 hands that to the parser. F8's value
  is right for its only caller (an inert document has no browsing context) but is a literal standing in for the
  reading, so it belongs to the same population. F6 / F7 have no production dispatcher at the base: elidex-js has no
  `insertAdjacentHTML` native (`git grep -n -i 'insertAdjacent' -- crates/script crates/shell ':!*tests*'` → only
  `insertAdjacentElement` / `insertAdjacentText`), and neither handler name is dispatched (`git grep -n -F -e
  '"insertAdjacentHTML"' -e '"innerHTML.set"' -- crates` → handler + registration only); they are members of the
  class `#11-dom-api-handlers-unreachable-from-production` (§9). Not changed here (§9, §7 R2).
- P7 (the XMLSerializer path): `serialize_node_to_string` (`markup.rs:183`) has one production caller, the
  `XMLSerializer.serializeToString` native: `git grep -n -w 'serialize_node_to_string' -- crates` → 6 hits = the
  definition, 2 re-exports (`element/mod.rs:31`, `lib.rs:93`), 2 doc comments (`dom_parser.rs:38`, `:325`) and 1 call
  (`crates/script/elidex-js/src/vm/host/dom_parser.rs:404`) (2026-11-18). It runs the HTML walk with P1's two
  escapers, so a `div` whose `title` is `x\u{a0}y<z>` and whose text is `p\u{a0}q` serializes as
  `<div title="x\u{a0}y<z>">p\u{a0}q</div>` (driver probe at `27ca7d1e`, PM 785; it also follows from P1): U+00A0 raw
  in both positions, the attribute's `<` raw. This entry's ground is not §13.3: HTML §8.5.9 `serializeToString(root)`
  step 1 is "Return the XML serialization of root given false", and the XML serialization escapes by DOM Parsing
  §5.2.1.5 *XML serializing a Text node* (steps 3-5: `&`, `<`, `>`) and by *serializing an attribute value* in
  §5.2.1.1.3 (step 3: `&`, `"`, `<`, U+0009, U+000A, U+000D; not `>`). Neither replaces U+00A0, so D-1's function on
  this path would newly emit `&nbsp;`, an entity XML does not define.

### §0.3 Decisions (this PR)

- **D-1 — one escaping function with a mode for HTML §13.3** (One issue, one way). `escape_html` and `escape_attr`
  are deleted. On the three §13.3 entry points (`serialize_inner_html`, `serialize_inner_html_with_options`,
  `serialize_outer_html`) they are replaced by `push_escaped_string(out: &mut String, s: &str, mode: EscapingMode)`
  (NEW) with `EscapingMode { Text, Attribute }` (NEW), a private item of `markup.rs` (its only consumer;
  §13.3-specific, so it leaves `util.rs`); on `serialize_node_to_string` they are replaced by D-6's two functions.
  Rationale: the spec defines one algorithm with an attribute-mode flag; two copies are exactly how P1's divergence
  arose. `push_*` writes into the output buffer (the current functions allocate a `String` per
  text node / attribute and the caller copies it). One per-character `match` whose arms are spec steps 1-5, step 5
  guarded by `mode == Attribute`; each arm writes its entity with `push_str` (single pass, so no entity is re-scanned).
- **D-2 — one raw-text predicate.** `RAW_TEXT_ELEMENTS` becomes exactly the spec's unconditional list (style, script,
  xmp, iframe, noembed, noframes, **plaintext**; noscript removed), and both read sites (P2) call one function
  `text_children_literal(dom, parent) -> bool` (NEW) = `dom.is_html_namespace(parent)` ∧ tag ∈ list
  (`EcsDom::is_html_namespace`, `crates/core/elidex-ecs/src/dom/classify.rs:136`). The `in_raw_text: bool` threading
  of `serialize_node` stays.
- **D-3 — noscript text is always escaped (PM 731).** A noscript parent never makes its Text children literal, in any
  document. Basis: the ratified one composition (P4) does not exist until N4, and reading HEAD's interim predicate
  from the serializer would add a second reading of it (and Rev 1's `ScriptingContext` a second composition) beside
  D8 — One issue, one way; the security defect (noscript re-emitted literally in a scripting-disabled document) is
  closed structurally by this PR, with no input that could select the unsafe branch (Security by structure); unknown
  → the safe side. The serializer reads **no** scripting state: no `ScriptingContext`, no `enabled_for_node`, no
  `SerializeOptions.scripting`, no `HostData::document()` / `HostData::sandbox_flags()` read in the natives. The
  doc comment on `text_children_literal` cites step 5.2, says noscript is deliberately absent (PM 731), and names
  `#11-scripting-disabled-predicate-no-document`; `RAW_TEXT_ELEMENTS`' own comment only refers to that function (it
  does not repeat the slot id — DoD 6 counts one item).
  - **Candidates rejected (PM 731).** (B) keep Rev 1's context-based noscript as a declared D8 exception, registered
    as an interim consumer in N4/N5's census: a second composition is a change to the ratified D8 surface, and
    "interim consumer" is exactly the strangler state One issue, one way forbids. (A) wait for N4 and land the precise
    rule with the one composition: delays the security fix for a fidelity gain.
  - **What stays open, and who owns it.** The precise rule (raw iff scripting is enabled for the node) and the
    parse-side scripting decision (P6, population F1-F8) both read N4's `EcsDom::scripting_disabled_for` (NEW: N4);
    the obligation is a facet of `#11-scripting-disabled-predicate-no-document` (PM 731, ledger). The node clause's
    final representation stays with the ratified `#11-domparser-full-document-parse-fidelity` marker (P5). No facet on
    `#11-browsing-context-state-ecs-components`.
  - **The two readings (PM 728).** The slot's Defect names two readings of the §8.1.3.4 predicate: the canonical
    function's fail-open no-document reading (P4) and the serializer's fail-safe one (Rev 1). The serializer side is
    gone in this PR (it reads no predicate); the remaining one is aligned at IBP-sandbox N4
    (`docs/plans/2026-09-ibp-sandbox.md` J13, `:408-413`).
- **D-4 — the HTML-namespace check is part of (d) (decided, PM 728).** "a style, script, … element" in step 5.2
  means HTML elements: HTML §2.1.3 XML compatibility (`#xml`): "Except where otherwise stated, all elements defined or
  mentioned in this specification are in the HTML namespace". P2 shows the namespace-blind check. `<svg><style>` text
  containing `</style><img onerror=…>` is entity-decoded by the parser (foreign content) and today re-emitted
  literally — the same injection class as (d). Fixed by D-2's `is_html_namespace` conjunct (one predicate, no extra
  site).
- **D-5 — withdrawn (Rev 2).** Rev 1 made every public entry point take `&SerializeOptions` because its default meant
  "scripting unknown". With no scripting field (D-3) that rationale is gone; the entry points
  (`serialize_inner_html`, `serialize_outer_html`, `serialize_node_to_string`, `serialize_inner_html_with_options`)
  keep their signatures, and their callers are unchanged.
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
    exactly two private functions, `push_text_data` and `push_attribute_value` (NEW), each a two-arm `match` that
    calls `push_escaped_string(.., Text | Attribute)` or the corresponding D-6 function; the walk's three write
    sites (Text, `is` value, attribute loop) call those two. It is not a field of the public `SerializeOptions`
    (`getHTML`'s options), so no caller of a §13.3 entry point can select XML escaping.
  - **Basis.** Spec: HTML §8.5.9 step 1 + the two DOM Parsing algorithms (P7). One issue, one way, read as *one
    function per spec algorithm*: §13.3's *escaping a string* has exactly one form (D-1) and each DOM Parsing
    algorithm has exactly one. Security by structure: the raw `<` in XMLSerializer attribute values (P7) is closed.
  - **Candidates rejected (PM 785).** (b1) keep HEAD's two escapers for the XMLSerializer path: leaves a second
    copy of §13.3-like escaping, makes I-1 false, and keeps the attribute `<` defect. (b3) land the full XML
    serializer first: beyond PM 724's scope, and delays the mXSS fix. Accepting `&nbsp;` on this path (Rev 4): a
    regression of well-formedness. An XML mode on `push_escaped_string`: a second form of §13.3's algorithm, and
    neither DOM Parsing algorithm is *escaping a string* with a flag (different character sets).
  - **What does not change on this path.** Everything the walk decides: element / comment / doctype output, the
    void check, and the raw-text decision — D-2's predicate, the same on both families, so a Text child of an
    HTML-namespace script / style / … parent is emitted literally through XMLSerializer too (HEAD already does this
    for its own list), although §5.2.1.5 reads no parent. Those stay with
    `#11-domparser-xml-real-parsing` (§9). A noscript's Text child is escaped on this path by `push_xml_text`, which
    is exact for §5.2.1.5 (not R1's approximation).

### §0.4 Spec citation table

| Cite | Title | Anchor | Lookup (2026-11-15) |
|---|---|---|---|
| HTML §13.3 | Serializing HTML fragments | `#serialising-html-fragments`, `#escapingString` | `webref heading html 13.3`; `webref body html serialising-html-fragments` (step 5.2 Text branch; preamble "the node"); `webref dfn html 'escaping a string'` → `#escapingString` (steps 1-5) |
| HTML §2.1.3 | XML compatibility | `#xml` | `webref heading html 2.1.3`; `webref body html xml` ("all elements … are in the HTML namespace") |
| HTML §8.1.3.4 | Enabling and disabling scripting | `#enabling-and-disabling-scripting`, `#concept-n-script`, `#concept-n-noscript` | `webref heading html 8.1.3.4`; `webref body html enabling-and-disabling-scripting` |
| HTML §7.1.5 | Sandboxing | `#sandboxed-scripts-browsing-context-flag` | `webref dfn html 'sandboxed scripts browsing context flag'` |
| HTML §13.4 | Parsing HTML fragments | `#parsing-html-fragments`, `#initialize-an-html-fragment-parser`, `#html-fragment-parsing-algorithm` | `webref heading html 13.4` → `#parsing-html-fragments`; `webref body html initialize-an-html-fragment-parser` (*initialize an HTML fragment parser* step 10: "Set parser's parser scripting disabled to true if scripting is disabled for contextDocument; otherwise false"; step 11, contextElement noscript: "If parser's parser scripting disabled is false, then switch the tokenizer to the RAWTEXT state. Otherwise, leave the tokenizer in the data state."); `webref body html html-fragment-parsing-algorithm` (step 3 calls the initializer; the algorithm itself has steps 1-7) |
| HTML §13.2.6.4.7 | The "in body" insertion mode | `#parsing-main-inbody` | `webref heading html 13.2.6.4.7`; `webref body html parsing-main-inbody` ("A start tag whose tag name is "noscript", if parser scripting disabled is false" → "Follow the generic raw text element parsing algorithm.") |
| HTML §13.2.6.5 | The rules for parsing tokens in foreign content | `#parsing-main-inforeign` | `webref heading html 13.2.6.5`; `webref body html parsing-main-inforeign` (the breakout start-tag list: "b", "big", …, "br", …, "embed", …, "hr", …, "img", …, "meta", …) |
| HTML §4.12.3 | The template element | `#appropriate-template-contents-owner-document` | `webref dfn html 'appropriate template contents owner document'` (step 1.1.1: "Let newDocument be the result of creating a document that implements Document, … its browsing context is null"; step 1.2: "Set document to document's associated inert template document") |
| HTML §8.5.8 | The createContextualFragment() method | `#the-createcontextualfragment()-method` | `webref heading html 8.5.8` (the number the code's XMLSerializer cites carry today; fixed to §8.5.9 by this PR, §4.1 / §4.2 / §9) |
| HTML §8.5.9 | The XMLSerializer interface | `#the-xmlserializer-interface` | `webref heading html 8.5.9`; `webref body html the-xmlserializer-interface` (`serializeToString(root)` step 1: "Return the XML serialization of root given false.") |
| DOM Parsing §5.2.1 | XML Serialization | `#xml-serialization` | `webref heading DOM-Parsing 5.2.1` (2026-11-18; the module name of D-6; subsections §5.2.1.1 … §5.2.1.8) |
| DOM Parsing §5.2.1.5 | XML serializing a Text node | `#xml-serializing-a-text-node` (heading), `#dfn-xml-serializing-a-text-node` (algorithm) | `webref heading DOM-Parsing 5.2.1.5`; `webref dfn DOM-Parsing 'XML serializing a Text node'` → §5.2.1.5 `#dfn-xml-serializing-a-text-node`; `webref body DOM-Parsing dfn-xml-serializing-a-text-node` (2026-11-18: steps 1-6; 2 = the *require well-formed* check; 3-5 = "&" → "&amp;", "<" → "&lt;", ">" → "&gt;") |
| DOM Parsing §5.2.1.1.3 | Serializing an Element's attributes | `#serializing-an-element-s-attributes` (heading), `#dfn-serializing-an-attribute-value` (algorithm *serializing an attribute value*) | `webref heading DOM-Parsing 5.2.1.1.3`; `webref dfn DOM-Parsing 'serializing an attribute value'` → §5.2.1.1.3 `#dfn-serializing-an-attribute-value`; `webref body DOM-Parsing dfn-serializing-an-attribute-value` (2026-11-18: step 1 = the *require well-formed* check; 2 = null → empty string; 3 = replace "&", U+0022, "<", U+0009 → "&#9;", U+000A → "&#xA;", U+000D → "&#xD;"; ">" is not in the list) |

`webref coverage-map html 13.3` → spec = 1, "ok → single PR scope" (2026-11-15); `webref coverage-map DOM-Parsing
5.2.1.5` and `… 5.2.1.1.3` → each spec = 1, step = 1, "ok → single PR scope" (2026-11-18). §2.1.3 / §8.1.3.4 / §7.1.5 /
§13.4 / §13.2.6.4.7 / §13.2.6.5 / §4.12.3 / §8.5.8 / §8.5.9 and DOM Parsing §5.2.1 are definitional or context cites;
this PR implements §13.3 steps, DOM Parsing §5.2.1.5 steps 3-5 and *serializing an attribute value* step 3.

## §1. Shape and verdict

One PR, one algorithm crate (`elidex-dom-api` serializer) plus VM end-to-end cells. **Not edge-dense**: two
intersecting axes — (i) the character map of escaping, (ii) the raw-text decision for a Text node's parent (the
unconditional list and the HTML-namespace conjunct). D-6 adds a selection, (iii) which spec's escaping an entry uses;
it is fixed once per entry (I-3), replaces the map of (i) wholesale (the full matrix is 2 families × {Text,
attribute value} = four pure functions-or-modes, each with its own unit cells), and does not enter (ii) — the raw-text
decision is the walk's and is the same for both families (D-6). No scripting input, no new ECS state, no event /
mutation / identity interaction. One normal `/elidex-plan-review`.

## §2. Invariants

- I-1 (escaping): every non-literal Text datum passes through `push_text_data` and every attribute / `is` value
  through `push_attribute_value`; those two dispatch to exactly three escaping functions, one per spec algorithm —
  `push_escaped_string` (HTML §13.3 *escaping a string*), `push_xml_text` (DOM Parsing §5.2.1.5) and
  `push_xml_attribute_value` (*serializing an attribute value*). No other markup-escaping code exists in `crates/`
  (concept sweep, §5.1; DoD 1).
- I-3 (selection): the escaping family is fixed once per public entry and never changes inside a walk — the three
  §13.3 entry points use only `push_escaped_string`, `serialize_node_to_string` uses only the DOM Parsing pair
  (D-6). `serialize_node_to_string` has one production caller, the XMLSerializer native (P7), so "the XMLSerializer
  path" and "`serialize_node_to_string`" name the same population.
- I-2 (raw text): for a Text node whose parent is an Element and which is reached through one of the two read sites
  (P2: the entry `serialize_inner_html_with_options` with that Element as *the node*, or the `serialize_node`
  recursion from that Element), the Text is emitted literally iff `text_children_literal(parent)`; the predicate is
  computed in exactly one function used at both read sites, and is false for every noscript and every
  non-HTML-namespace parent. I-2 is about the walk, so it holds for both escaping families: on the XMLSerializer
  path "escaped" means `push_xml_text`, and a raw-text parent's Text is literal there too (the walk is the HTML one;
  D-6, §9). Outside that scope: a bare Text node passed to `serialize_outer_html` / `serialize_node_to_string`
  (`XMLSerializer.serializeToString(text)`) is escaped without consulting its parent (the walk starts with
  `in_raw_text = false`, `markup.rs:162`). That path's ground is not §13.3 (which takes "a DOM Element, Document, or
  DocumentFragment" as *the node*): HTML §8.5.9 `serializeToString(root)` step 1 returns "the XML serialization of
  root given false", and a Text root is serialized by DOM Parsing §5.2.1.5 *XML serializing a Text node*, which
  escapes the data and reads no parent. Since D-6 that is the function this entry runs (`push_xml_text`), so the
  bare-Text output is exact for §5.2.1.5 (at HEAD it was `escape_html`, which has the same three replacements).
- I-1 × I-2: an attribute value inside a noscript is serialized with `<`/`>` escaped, so re-parsing the output in a
  scripting-enabled context (where noscript is RAWTEXT) cannot close the noscript early — V5 pins this intersection.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §13.3 Serializing HTML fragments | Escaping a string steps 1-4 | text mode (Text data outside raw-text parents), §13.3 entry points | `push_escaped_string(.., Text)`, the `HtmlFragment` arm of `push_text_data` (`serialize_node` Text branch) | ✓ | yes |
| WHATWG HTML §13.3 Serializing HTML fragments | Escaping a string steps 1-5 | attribute mode (attribute values; the synthetic `is` value), §13.3 entry points | `push_escaped_string(.., Attribute)`, the `HtmlFragment` arm of `push_attribute_value` (the `is` and attribute-loop sites) | ✓ | yes |
| W3C DOM Parsing §5.2.1.5 XML serializing a Text node | steps 3-5 | *require well-formed* = false (`serialize_node_to_string`) | `push_xml_text`, the `XmlSerialization` arm of `push_text_data` | ✗ (step 2, *require well-formed* = true: no caller; `#11-domparser-xml-real-parsing`) | yes |
| W3C DOM Parsing §5.2.1.1.3 Serializing an Element's attributes | *serializing an attribute value* step 3 | *require well-formed* = false, non-null value (`serialize_node_to_string`) | `push_xml_attribute_value`, the `XmlSerialization` arm of `push_attribute_value` | ✗ (step 1, *require well-formed* = true: no caller; step 2, null: no input; the rest of §5.2.1.1.3 — prefixes, namespace declarations — is `#11-domparser-xml-real-parsing`) | yes |
| WHATWG HTML §13.3 Serializing HTML fragments | step 5.2 (Text node) | parent ∈ {style, script, xmp, iframe, noembed, noframes, plaintext}, HTML namespace → literal | `text_children_literal` | ✓ | yes |
| WHATWG HTML §13.3 Serializing HTML fragments | step 5.2 (Text node) | parent noscript → escaped in every document (PM 731) | `text_children_literal` (false) | ✗ (the "scripting is enabled for the node" → literal branch is not taken; N4, §9) | yes |
| WHATWG HTML §13.3 Serializing HTML fragments | step 5.2 (Text node) | otherwise (incl. same-named non-HTML-namespace parent) → escaped | `serialize_node` Text branch | ✓ | yes |

**Breadth**: K=2 specs (HTML, DOM Parsing), M=7 entries (verified 2026-11-18: preflight `total entries (M): 7`; the
two DOM Parsing rows are "unmapped-label" for preflight, whose label table has no DOM Parsing entry, so their
citations were verified by the §0.4 lookups instead). **Split decision**: single PR.

### §3.1 User-input touch audit

- Text data, attribute values and `is` values are author-controlled (`textContent`, `setAttribute`,
  `createElement(.., {is})`, any parse). All four serializer natives emit them: `innerHTML` / `getHTML` / `outerHTML`
  getters (`crates/script/elidex-js/src/vm/host/dom_inner_html.rs:116`, `:317`, `:339`) and
  `XMLSerializer.serializeToString` (`crates/script/elidex-js/src/vm/host/dom_parser.rs:404`). The natives' code
  is not edited (§4.2); exposure decreases through the shared serializer. On the XMLSerializer path specifically
  (D-6): Text output is unchanged (the same three replacements as HEAD's `escape_html`); attribute values gain `<` →
  `&lt;` (the P7 defect) and the U+0009 / U+000A / U+000D character references; U+00A0 stays raw.
- Adjacent pre-existing: the Comment branch (`markup.rs:382-386`, data verbatim per spec) and attribute-name
  filtering (`is_safe_attr_name`, `markup.rs:267`) — untouched; exposure unchanged.

## §4. Mechanism

### §4.1 `elidex-dom-api` — `markup.rs`

- `EscapingMode` + `push_escaped_string` (NEW, private), D-1. `util.rs` loses `escape_html` / `escape_attr` and their
  two tests (moved as cells, §6).
- `CharacterEscaping` + `push_text_data` / `push_attribute_value` (NEW, private), D-6. The walk's `opts:
  &SerializeOptions` parameter becomes one private `walk: &Walk<'_>` (NEW: `{ opts: &SerializeOptions, escaping:
  CharacterEscaping }`) in `serialize_node`, `emit_own_shadow_root_if_needed` and `emit_shadow_root_template`, so the
  selection travels with the options and no call site can drop it. The three §13.3 entry points build a `Walk` with
  `HtmlFragment`; `serialize_node_to_string` builds one with `XmlSerialization` and default options before its
  `NodeKind` match and uses it in both arms (it calls the private children / outer walks directly, no longer the
  public `serialize_inner_html` / `serialize_outer_html`).
- `serialize_node_to_string`'s docstring and the module header are rewritten for D-6: the XMLSerializer cite becomes
  HTML §8.5.9 (both say §8.5.8 at `c0578223`, `markup.rs:7` and `:168`; §8.5.8 is *The createContextualFragment()
  method*), and the docstring says the walk is the HTML one with DOM Parsing's two escapes (§0.4 numbers). The
  header's other numbers are stale by the same renumbering (it says §8.5.3 / .4 / .5 / .6 for what are now §8.5.4
  *HTML serialization methods*, §8.5.5 *The innerHTML property*, §8.5.6 *The outerHTML property*; `webref heading
  html 8.5.3` … `8.5.6`, 2026-11-18) — out of this PR's diff apart from the XMLSerializer line; they belong to the
  slot's citation pass (§9).
- `RAW_TEXT_ELEMENTS` per D-2 (doc comment per D-3).
- `text_children_literal` (NEW, private) per D-2/D-4; called at the two read sites (P2).
- `SerializeOptions`, the public entry points' signatures, `GetInnerHtml` and the re-export lists are unchanged
  (D-5 withdrawn).

### §4.1a `elidex-dom-api` — `xml_serialization.rs` (NEW)

`push_xml_text` and `push_xml_attribute_value` (`pub(crate)`), D-6; `lib.rs` gains `pub(crate) mod
xml_serialization;` and the `#[cfg(test)] mod xml_serialization_tests;` line. Each is one per-character `match` that
writes into the output buffer (single pass, like D-1). Unit cells live in the sibling
`xml_serialization_tests.rs` (the crate's `*_tests.rs` convention: `class_list_tests.rs`, `dialog_tests.rs`), which
also keeps DoD 1's entity grep free of test strings.

### §4.2 `elidex-js` — no native code change

The four natives' code is not edited (D-3: no context to marshal; D-6: the selection is made inside
`serialize_node_to_string`). Comments only, in `dom_parser.rs`: the module doc (`:1`, "`XMLSerializer` (HTML
§8.5.8)", and `:37-41`), the `serializeToString` docstring (`:322-…`, "HTML §8.5.8 … This is HTML serialization (boa parity)") and the inline
comment above the call (`:397-403`) describe the old escaping and the old section number; they are reworded to "HTML
§8.5.9; the HTML walk with DOM Parsing's two escapes (D-6); the rest of the XML serialization is
`#11-domparser-xml-real-parsing`". `elidex-script-session` is not edited.

### §4.3 ECS-native / layering check

No side-store, no new component, no new state read. OO → ECS mapping of the concepts this PR touches (the scripting
rows are IBP-sandbox §6's, `docs/plans/2026-09-ibp-sandbox.md:659-668`, not this PR's):

| Spec / OO concept | elidex (ECS) | This PR |
|---|---|---|
| "an HTML element" (§2.1.3, namespace) | `EcsDom::is_html_namespace(entity)` | read (existing) |
| element type's local name | `TagType` component | read (existing) |
| Text node's data | `TextContent` component | read (existing) |
| *escaping a string* (attribute-mode flag) | `push_escaped_string(.., EscapingMode)` — a pure function, no state | NEW |
| *XML serializing a Text node* / *serializing an attribute value* (*require well-formed* = false) | `push_xml_text` / `push_xml_attribute_value` — pure functions, no state | NEW |
| "which serialization this entry is" (HTML fragment vs XML) | `CharacterEscaping`, a value fixed by the entry and passed down the walk — not stored on any entity or document | NEW |
| "scripting is enabled for the node" | `EcsDom::scripting_disabled_for(.., ScriptingSubject)` (NEW: N4, D8); node clause = the `#11-domparser-full-document-parse-fidelity` marker | not read (D-3) |

Algorithms live in `elidex-dom-api` (§13.3 in `element/markup.rs`, DOM Parsing §5.2.1 in `xml_serialization.rs`);
`vm/host/` code is untouched (comments only, §4.2).

## §5. Populations

### §5.1 Escaping callers census

`git grep -n -w 'escape_attr\|escape_html' -- crates` → 11 hits (verified 2026-11-15): definitions `util.rs:154`,
`:168` (2); import `markup.rs:14` (1); production calls `markup.rs:373` (Text), `:421` (`is` value), `:436`
(attribute loop) (3); comment `markup.rs:411` (1); test message `tests_markup.rs:200` (1); unit asserts
`util.rs:225`, `:226`, `:231` (3). The 3 production calls become the two dispatch functions — `:373` →
`push_text_data`, `:421` and `:436` → `push_attribute_value` — which reach `push_escaped_string` on the §13.3 entry
points and D-6's pair on `serialize_node_to_string` (I-1, I-3); the comment and the test message are reworded.
Concept sweep for other markup-escaping code at the base: `git grep -n "'&' =>\|&amp;\|&quot;" -- crates ':!*tests*'
':!crates/dom/elidex-html-parser*'` → only `util.rs` (and an unrelated JS-lexer `b'&'`); adding the character
references of D-6 to the pattern (`&#9;\|&#xA;`) adds no hit (2026-11-18). So the base has exactly the two copies of
P1, and after this PR the same sweep finds exactly the three functions of I-1 (DoD 1).

### §5.2 Raw-text read sites

`git grep -n -w 'RAW_TEXT_ELEMENTS' -- crates` → definition `markup.rs:22`, reads `:243` and `:445` (verified
2026-11-15). Both reads become `text_children_literal`, which is then the only reader (DoD 1).

Entries that reach a Text node, by how its raw-text decision is made (`git grep -n 'serialize_node(' --
crates/dom/elidex-dom-api/src/element/markup.rs`, 2026-11-15): (1) the entry read `:243` (`in_raw_text` from the
node's own tag); (2) the recursion read `:445` (from the parent's tag); (3) `serialize_outer_html` `:160-162`,
which starts `serialize_node` with `in_raw_text = false` — for an Element it reaches Text children only through (2),
but for a bare Text node (via `serialize_node_to_string`, `:183`, the `XMLSerializer` path) it escapes with no parent
read — grounded in HTML §8.5.9 `serializeToString` step 1 → DOM Parsing §5.2.1.5 *XML serializing a Text node*, not
in §13.3, and since D-6 run by `push_xml_text` (I-2); (4) `:351` (a shadow root's children inside the emitted `<template shadowrootmode>`) and `:457` (children of
a non-Element node) pass `false`: their Text's parent is a ShadowRoot / Document / DocumentFragment, not an Element,
so step 5.2's *otherwise* (escaped) is exact. (3) and (4) are not `RAW_TEXT_ELEMENTS` reads; their raw-text
argument (`false`) is unchanged (I-2's scope note). D-6 changes none of these decisions: `serialize_node_to_string`
reaches the same five calls with `XmlSerialization` in its `Walk` (§4.1), so which Text is literal is identical on
both families and only the escaper differs. The grep gives 6 hits = 5 calls (`:162`, `:247` after the read `:243`, `:351`, `:449` after
the read `:445`, `:457`) + the definition `:356`, verified 2026-11-15.

### §5.3 Files touched (`wc -l` at `c0578223`)

| File | Lines | Change |
|---|---|---|
| `crates/dom/elidex-dom-api/src/element/markup.rs` | 459 | D-1, D-2, D-4, D-6's selection and `Walk` (~+70) |
| `crates/dom/elidex-dom-api/src/xml_serialization.rs` (NEW) | — | D-6's two functions (~60) |
| `crates/dom/elidex-dom-api/src/xml_serialization_tests.rs` (NEW) | — | XT / XA cells (~90) |
| `crates/dom/elidex-dom-api/src/lib.rs` | 132 | two `mod` lines |
| `crates/dom/elidex-dom-api/src/util.rs` | 233 | −2 fns, −2 tests (~−30) |
| `crates/dom/elidex-dom-api/src/element/tests_markup.rs` | 246 | new cells (~+120) |
| `crates/script/elidex-js/src/vm/host/dom_parser.rs` | 407 | comments only (§4.2) |
| `crates/script/elidex-js/src/vm/tests/tests_html_serialize_escape.rs` (NEW) | — | VM cells (~190) |
| `crates/script/elidex-js/src/vm/tests/mod.rs` | 210 | `mod tests_html_serialize_escape;` |

## §6. Test and coverage plan

Supported surface: §13.3 escaping, the step-5.2 Text branch, and the XMLSerializer path's two DOM Parsing escapes,
guarded by engine-independent unit cells in `elidex-dom-api` and VM end-to-end cells; no WPT subset is added by this
PR. U1-U6 and V1-V13 exercise the §13.3 entry points and are unchanged by D-6; the XMLSerializer path's cells are
XT / XA (unit) and X1-X3 (VM).

**Unit, `tests_markup.rs`** (fixtures: `EcsDom::create_document_root` = the cached document root, "bound-like";
`create_document_node` = a plain cache-free Document, "plain-document". Neither is distinguishable to this PR's
serializer, which reads no document state; see U6 for what N4 will make of each):
- U1 text mode, one cell per character: `&`, U+00A0, `<`, `>` escaped; `"` **not** escaped in text.
- U2 attribute mode, one cell per character: `&`, U+00A0, `<`, `>`, `"` escaped (attribute and `is` value sites).
- U3 composition, exact output: data `"a\u{a0}<&b"` → exactly `a&nbsp;&lt;&amp;b` (adjacent special characters,
  order and the unescaped neighbours preserved). U3 has no mutant of its own: the "arm writes its entity back
  through the escaper" mutant (e.g. `push_escaped_string(out, "&lt;", mode)`) is already red in U1/U2's
  single-character cells (`&amp;lt;` etc.) on the `<`, `>`, U+00A0 and `"` arms; on the `&` arm it recurses without
  bound (`&amp;` contains `&`), which aborts the test process rather than failing an assertion — still observed red,
  and also killed by U1's `&` cell. U3 is kept as the one cell asserting a multi-character output verbatim.
- U4 each unconditional raw parent (script, style, xmp, iframe, noembed, noframes, plaintext) with data `<&>`
  emitted literally, **through both entries**: U4-entry `serialize_inner_html(raw_parent, ..)` (read site
  `markup.rs:243`) and U4-recursion `serialize_outer_html(raw_parent, ..)` (read site `:445`); `plaintext` is the new
  row.
- U5 SVG-namespace `style` (and `script`) parent: data escaped (D-4), through both entries: U5-entry
  `serialize_inner_html(svg_style, ..)` (`:243`) and U5-recursion `serialize_outer_html(svg_root, ..)` (`:445`).
- U6 noscript with data `<p>&` → escaped (`&lt;p&gt;&amp;`) in both fixtures, through both read sites:
  `serialize_inner_html(noscript, ..)` (entry, `markup.rs:243`) and `serialize_outer_html(parent_of_noscript, ..)`
  (recursion, `:445`). Four test functions: `u6_noscript_escaped_bound_like_entry`,
  `u6_noscript_escaped_bound_like_recursion`, `u6_noscript_escaped_plain_document_entry`,
  `u6_noscript_escaped_plain_document_recursion`. **All four are accepted-regression cells that flip at obligation
  (1)**, and all four carry the slot id. Reason: what N4's `EcsDom::scripting_disabled_for` (NEW: N4) returns for a
  fixture is decided by the null-browsing-context marker of `#11-domparser-full-document-parse-fidelity`, whose
  writers are DOMParser's build and *destroy a document* (`docs/plans/2026-09-ibp-sandbox.md:706-710`). Neither unit
  fixture is built by a writer — `create_document_node` is a plain Document with no marker — so N4 returns "enabled"
  for both, and after obligation (1) step 5.2's literal branch applies to both. The plain-document pair is therefore
  **not** a DOMParser stand-in and makes no "spec-exact" claim (Rev 4 said it did). Its comment says: "plain
  Document, no null-browsing-context marker: flips at obligation (1) together with the bound-like pair; N4 may
  instead rebuild this fixture with the marker, after which the expectation (escaped) stays and is the disabled side
  of the precise rule". Until then the disabled side is pinned where a real DOMParser document exists: V2 (VM).

**VM end-to-end, `tests_html_serialize_escape.rs` (NEW)** (bound VM; real parse):
- V1 page noscript escaped (§7 R1): `div.innerHTML = '<noscript><p id=x></p></noscript>'` →
  `div.innerHTML === '<noscript>&lt;p id=x&gt;&lt;/p&gt;</noscript>'` (the setter parses with scripting enabled, so
  the noscript holds one Text node `<p id=x></p>`; today the getter returns the input unchanged).
- V2 DOMParser noscript escaped: `parseFromString('<noscript><p id=x></p></noscript>', 'text/html')` (bare form;
  raw text today per `dom_parser_bare_noscript_stays_rawtext_fragment_mode_boundary`, `tests_dom_parser.rs:546`) →
  its noscript's `innerHTML` is `&lt;p id=x&gt;&lt;/p&gt;`; `XMLSerializer` on the document likewise (through
  `push_xml_text`, D-6: the data has only `<` / `>`, which both families replace alike). V2 is the cell that pins the
  scripting-disabled side on a real DOMParser document: spec-exact (step 5.2's *otherwise*; §5.2.1.5 for the
  XMLSerializer half), it does not flip at N4 and does not name the slot.
- V4 mXSS, noscript text: in a DOMParser document, `ns.textContent = '</noscript><img src=x onerror=f()>'`; then
  `d.innerHTML = ns.outerHTML` on a page `div` → `d.querySelector('img') === null`. Red on old code: the old getter
  emits the text raw, so the page parse (noscript RAWTEXT) ends the noscript at the embedded `</noscript>` and builds
  the `img`.
- V5 mXSS, attribute `<`/`>`: in a DOMParser document, a noscript containing `<p title="</noscript><img src=x
  onerror=f()>">` (built by `createElement` / `setAttribute`); `d.innerHTML = ns.outerHTML` → no `img`. Red on old
  code independently of D-3: the `p` is an element child, so the old output already took the element path, and
  `escape_attr` left `<`/`>` in the attribute, which the page's RAWTEXT noscript reads as its end tag.
- V6 nbsp: `p.textContent = 'a b'` → `p.innerHTML === 'a&nbsp;b'`; `setAttribute('title', 'a b')` →
  `outerHTML` contains `title="a&nbsp;b"`.
- V7 plaintext: `<plaintext>` created by `createElement`, `textContent = '<b>'` → `innerHTML === '<b>'`.
- V10 / V11 (innerHTML) and V12 / V13 (outerHTML) round-trip growth (§7 R2), pinned expectations.
- X1-X3, the XMLSerializer path (D-6; exactly PM 785's three cells; a page `div` built by `createElement` /
  `setAttribute` / `textContent`, `s = new XMLSerializer()`):
  - **X1** U+00A0 is emitted raw, in Text and in an attribute value: `title` = `'x\u00a0y'`, text `'p\u00a0q'` →
    `s.serializeToString(div) === '<div title="x\u00a0y">p\u00a0q</div>'`, plus two separate assertions that the
    output contains no `&nbsp;` before and after the first `>` (so a one-site mutant is attributable). Root = the
    Element (the outer arm of `serialize_node_to_string`).
  - **X2** an attribute `<` becomes `&lt;`: `title` = `'a<b'`, the `div` appended to a `DocumentFragment` →
    `s.serializeToString(frag) === '<div title="a&lt;b"></div>'`. Root = the fragment (the children arm), so both
    arms of the entry are covered.
  - **X3** `>` in an attribute value is not replaced: `title` = `'a>b'` → `s.serializeToString(div) === '<div
    title="a>b"></div>'`.

VM test function names (used by DoD 6): `v1_page_noscript_escaped`, `v10_inner_html_round_trip_scripting_enabled`,
`v11_inner_html_round_trip_scripting_disabled`, `v12_outer_html_round_trip_scripting_enabled`,
`v13_outer_html_round_trip_scripting_disabled`. X cells: `x1_xml_serializer_nbsp_raw`,
`x2_xml_serializer_attr_lt_escaped`, `x3_xml_serializer_attr_gt_raw`.

**Unit, `xml_serialization_tests.rs` (NEW)** — the two functions themselves, one cell per replacement arm, the way
U1 / U2 cover `push_escaped_string`:
- XT1 `push_xml_text`, one cell per replaced character: `&` → `&amp;`, `<` → `&lt;`, `>` → `&gt;`.
- XT2 `push_xml_text`, not replaced (one cell per character, output equals input): `"`, U+00A0, U+0009, U+000A,
  U+000D.
- XT3 `push_xml_text` composition, exact output: `a&<>\u{a0}"b` → `a&amp;&lt;&gt;\u{a0}"b`.
- XA1 `push_xml_attribute_value`, one cell per replaced character: `&` → `&amp;`, `"` → `&quot;`, `<` → `&lt;`,
  U+0009 → `&#9;`, U+000A → `&#xA;`, U+000D → `&#xD;`.
- XA2 `push_xml_attribute_value`, not replaced (one cell per character): `>`, U+00A0.
- XA3 `push_xml_attribute_value` composition, exact output: `a&"<\t\n\r>\u{a0}b` →
  `a&amp;&quot;&lt;&#9;&#xA;&#xD;>\u{a0}b`.

Each accepted-regression cell (U6 — all four functions —, V1, V10, V11, V12, V13) carries a code comment naming
`#11-scripting-disabled-predicate-no-document`, which obligation flips it (§7), and "PM 731" (DoD 6), so the
flip at N4 is found by grepping the slot id. V2 does not name the slot (it does not flip). The X / XT / XA cells are
not accepted regressions and name no slot.

Rev 1's U7-U11, the `scripting.rs` unit cells and V3 are removed: each distinguished a scripting context that no
longer exists (bound / other / template / ownerless document, `None`, sandboxed); under D-3 all of them are U6.
(Cell numbers are not reused.)

**Existing tests whose expectations change** — census: `git grep -n -i 'noscript' -- crates/dom/elidex-dom-api
crates/script/elidex-js/src/vm/tests crates/shell` plus `&quot;|&lt;|&gt;|&amp;|nbsp` and `<svg…><style|script>`
in serializer tests (2026-11-15): `util.rs` `escape_html_chars` / `escape_attr_chars` are deleted with their
functions (their assertions are subsumed by U1/U2 with the same expected strings). `tests_markup.rs:20`
(`hello &lt;world&gt;`) and `:199` (`is="x&quot; onclick=&quot;evil"`) keep their expectations. No existing test
serializes a noscript, an SVG style/script, or an attribute containing `<`, `>` or U+00A0 (the noscript tests in
`tests_dom_parser.rs` assert parse shape only). XMLSerializer path (D-6): `git grep -n 'serializeToString' --
crates/script/elidex-js/src/vm/tests` → 13 lines, all in `tests_dom_parser.rs` (2026-11-18); the ones that assert
output use `class="test"`, `id="f"`, text `hello world` / `hi`, a comment and brand / argument errors — no `<`, `>`,
`&`, `"`, U+00A0, U+0009, U+000A or U+000D inside a serialized Text or attribute value. **No expectation flips.**

**Mutants** (each must turn a named cell red): drop each of the 5 escape arms (U1/U2); unguard step 5 (U1 `"`); an
arm re-escapes its own entity (U1/U2, see U3); remove plaintext (U4-entry, U4-recursion, V7); drop the namespace
conjunct in `text_children_literal` (U5-entry, U5-recursion); **one site not converted** — the entry site keeps
`RAW_TEXT_ELEMENTS.contains(tag)` instead of calling `text_children_literal` (U5-entry red, U5-recursion green), and
symmetrically the recursion site (U5-recursion red, U5-entry green); put noscript back in the raw list (U6/V1/V2/V4);
keep `escape_attr`'s old map at the attribute site (V5). D-6's functions: drop each of `push_xml_text`'s 3 arms
(XT1) and each of `push_xml_attribute_value`'s 6 arms (XA1); add a `>` arm to `push_xml_attribute_value` (XA2, X3);
add a U+00A0 → `&nbsp;` arm to either (XT2 / XA2, X1); add a `"` arm to `push_xml_text` (XT2). D-6's selection:
`serialize_node_to_string` passes `HtmlFragment` in its outer arm (X1, X3 red; X2 green) or in its children arm (X2
red; X1, X3 green); `push_text_data`'s `XmlSerialization` arm calls `push_escaped_string` (X1's Text assertion);
`push_attribute_value`'s `XmlSerialization` arm calls `push_escaped_string` (X1's attribute assertion, X3); and the
reverse, an `HtmlFragment` arm calling the DOM Parsing function (V6's Text half / V6's `title` half and U2's `>`
cell). Rev 1's per-native `None` /
`getHTML`-omits-context mutants (axis-3 IMP-2) disappear: no native passes a context any more, so there is no
per-native input to mutate; every native reaches the one serializer, whose mutants are above.

## §7. Accepted regressions (PM 731)

Copied verbatim into the PR body (§10 DoD 5). Each is pinned by a cell, so it is an expectation, not a hidden failure.
Both are on the §13.3 entry points. The XMLSerializer path has no accepted regression: its Text output is unchanged
and its attribute-value output moves to DOM Parsing's algorithm (D-6; X1-X3).

- **R1 — page noscript text is escaped (today raw).** In a document where scripting is enabled for the node (the
  bound page), step 5.2 makes noscript text literal; this PR escapes it until N4 makes the precise rule available
  (`#11-scripting-disabled-predicate-no-document`). Fidelity loss in the safe direction: a re-parse in a
  scripting-enabled context shows the entities as text. Cells: **U6** (unit, all four functions) and **V1** (pins
  `'<noscript>&lt;p id=x&gt;&lt;/p&gt;</noscript>'`).
- **R2 — a getter→setter round trip grows on noscript content.** Mechanism (either scripting value): the setter's
  fragment parse makes noscript content one Text node (RAWTEXT, because the parse runs with scripting enabled), the
  getter escapes it (D-3), the next set keeps the entities as literal text, and the next get escapes the `&` again.
  Scope: every entry whose setter re-parses a getter result in a context where noscript is RAWTEXT — `innerHTML`
  (= `innerHTML`), `setHTMLUnsafe`, `ShadowRoot.innerHTML`, `outerHTML` (= `outerHTML`), and `insertAdjacentHTML` of a
  getter result (P6 F1-F7). `insertAdjacentHTML` has no elidex-js native at the base (P6 F6), so it is not
  JS-observable here and has no cell; the others share F1's mechanism and are pinned through `innerHTML` and
  `outerHTML`. The root cause differs by the HTML §8.1.3.4 value for the context document — scripting enabled or
  disabled, i.e. what N4's `EcsDom::scripting_disabled_for` (NEW: N4) returns for it — not by document kind, and so
  does the N4 obligation that removes it:
  - **Scripting enabled for the context document** (e.g. a page document with a browsing context, scripts not
    sandboxed). The RAWTEXT parse is spec-correct here (§13.4 *initialize
    an HTML fragment parser* step 10 sets the parser's scripting disabled to false, since scripting is enabled for
    the context document). The growth comes **only** from this PR's serializer approximation (R1: noscript always
    escaped, where step 5.2 says literal). The parse-side fix does **not** fix it; only the precise serializer rule
    (raw iff scripting is enabled for the node) does — obligation (1) of `#11-scripting-disabled-predicate-no-document`
    (PM 731). Cells **V10** (innerHTML) and **V12** (outerHTML).
  - **Scripting disabled for the context document.** Examples: DOMParser documents; a page `<template>`'s content
    (its node document is the template's inert template document, HTML §4.12.3 *appropriate template contents owner
    document*, which has no browsing context); any other document with no browsing context. The fact that scripting
    is disabled comes from HTML §8.1.3.4's node clause (a Node whose node document's browsing context is null; P3);
    §13.4 *initialize an HTML fragment parser* step 10 passes it to the parser. With parser scripting disabled true,
    a noscript context element leaves the tokenizer in the data state (the initializer's step 11), and a nested
    `<noscript>` start tag is no longer "a start tag whose tag name is "noscript", if parser scripting disabled is
    false" of HTML §13.2.6.4.7 *in body* (the generic raw text element parsing algorithm), so noscript's content is
    built as elements. The parse side's fixed value (P6 F1-F7: scripting enabled regardless of the context document)
    is the root; the serializer's escaping is spec-exact here. Today's raw getter masks the parse
    defect as a stable round trip; this PR's exact getter exposes it. Fixed by the parse-side reading — obligation
    (2) of the same slot, over P6's full population. Cells **V11** (innerHTML) and **V13** (outerHTML), both on a
    real DOMParser document. **Gap the split depends on**: the split is by the spec value; obligation (2) fixes a
    given document only if N4's composition returns "disabled" for it, which under the ratified representation means
    the document carries the null-browsing-context marker. The marker's stated writers are DOMParser's build and
    *destroy a document* (`docs/plans/2026-09-ibp-sandbox.md:706-710`); the inert template document
    (`crates/core/elidex-ecs/src/dom/template_owner.rs:55`, a plain `create_document_node`) and other
    no-browsing-context documents have no stated writer, although HTML §4.12.3 step 1.1.1 creates the inert template
    document with "its browsing context is null". Until a writer covers them, N4 answers "enabled" for template
    content and its round trip is neither of the two cases above. Recorded as a writer obligation on the slot
    (§10 landing (ii)); no cell here depends on it (V11 / V13 use DOMParser documents).

  Cells (each comments the slot id and its obligation, §6):
  - **V10** (page `div`): `div.innerHTML = '<noscript><p></p></noscript>'; g1 = div.innerHTML; div.innerHTML = g1;
    g2 = div.innerHTML` → `g1 === '<noscript>&lt;p&gt;&lt;/p&gt;</noscript>'`,
    `g2 === '<noscript>&amp;lt;p&amp;gt;&amp;lt;/p&amp;gt;</noscript>'`. Flips at obligation (1).
  - **V11** (an element `h` of `new DOMParser().parseFromString('<body><div id=h></div></body>', 'text/html')`): the
    same steps and the same two pinned strings. Flips at obligation (2).
  - **V12** (page `div`, outerHTML): `div.innerHTML = '<section><noscript><p></p></noscript></section>'; g1 =
    div.innerHTML; div.firstChild.outerHTML = div.firstChild.outerHTML; g2 = div.innerHTML` →
    `g1 === '<section><noscript>&lt;p&gt;&lt;/p&gt;</noscript></section>'`,
    `g2 === '<section><noscript>&amp;lt;p&amp;gt;&amp;lt;/p&amp;gt;</noscript></section>'` (the outerHTML setter
    parses in the parent's context, P6 F5). Flips at obligation (1).
  - **V13** (the DOMParser element `h` of V11, outerHTML): V12's steps on `h` and the same two pinned strings. Flips
    at obligation (2).

## §8. 1000-line audit

All touched files are under 1000 lines at `c0578223` (§5.3). VM cells go to a NEW file rather than
`tests_dom_parser.rs` (823) or `tests_inner_html_mixin.rs` (749), which would approach the line.

## §9. Out of scope, and slots

- Slot items (a) void list, (b) step 1, (c) shadow-template attribute order + `shadowrootcustomelementregistry`,
  (f) attribute order (note: the spec says the order "is implementation-defined … must be stable", so (f) is a
  fidelity/allocation question, not an observable-deviation one — recorded for that facet, not decided here):
  stay in `#11-html-fragment-serializer-conformance`, Re-eval 2026-12-15. Item (a) also gains the void check's
  namespace defect (landing, §10): `VOID_ELEMENTS.contains` at `markup.rs:441` reads `TagType` only, so a
  non-HTML-namespace element with a void local name loses its children and end tag (same §2.1.3 reading as D-4,
  four lines from the raw-text read). **It stays out by PM 724's scope** — this PR is items (d) and (e) only, and the
  void list is item (a). **Why it is not a security issue** (unlike D-4, which is in (d) because it is the injection
  class): the void decision changes only whether children and the end tag are emitted. The namespace in which later
  siblings are re-parsed is decided by the start tag's name, which the void decision does not change. The serializer's
  unescaped outputs — the three character-data ones (a raw-text parent's Text data, `markup.rs:371`; comment data,
  `:384`; the doctype name, `:394`) and the names (tag names, `:400` / `:452`; attribute names, `:434`, filtered by
  `is_safe_attr_name`) — are each emitted, or not, independently of the void decision: it adds no unescaped
  output and changes the content of none. Its failure modes are dropped content (the children
  are not emitted at all) and, for the void names in `VOID_ELEMENTS` (`markup.rs:260`) that do not break out of
  foreign content — area, base, col, input, link, param, source, track, wbr (HTML §13.2.6.5's breakout start-tag list
  contains br, embed, hr, img and meta, and none of these) — an omitted end tag, so on re-parse the following siblings
  nest inside the element: a tree-shape (fidelity) defect, not an mXSS one.
- The XMLSerializer cite "HTML §8.5.8" (`markup.rs:7`, `:168`; `dom_parser.rs:1`, `:322`) is fixed to §8.5.9 by this PR,
  which rewrites those comments for D-6 (§4.1, §4.2). Out-of-diff observation (not changed here): the other section
  numbers in `markup.rs`'s module header are stale by the same renumbering (§4.1); recorded for the slot's citation
  pass.
- The rest of the XML serialization: `#11-domparser-xml-real-parsing` (PM 785). This PR lands only its two escaping
  functions (D-6). Staying there: the XML walk itself — void self-closing, namespace prefixes and declarations (the
  remainder of §5.2.1.1.3), the Attr-root contract, the doctype's public / system identifiers — and the
  *require well-formed* = true checks. One consequence of keeping the HTML walk that the slot's row does not name
  today: on the XMLSerializer path a Text child of an HTML-namespace raw-text parent (script, style, …) is still
  emitted literally (D-2's decision is the walk's), where §5.2.1.5 escapes every Text node; unchanged from HEAD, and
  added to the slot's row at landing (§10).
- The precise noscript rule and the parse-side scripting decision (P6): `#11-scripting-disabled-predicate-no-document`
  (PM 731 facet; trigger IBP-sandbox N4's per-PR plan). Hand-off population for obligation (2) = P6 F1-F8 (all eight
  constructions, with their entry APIs; F6 / F7 are reachable only through the handler class below, F8's value is
  already right but is a literal). Obligation (1) is the serializer side: `text_children_literal` gains the noscript
  conjunct "and scripting is enabled for the node" read from N4's composition, flipping U6 (all four functions; the
  plain-document pair unless N4 rebuilds its fixture with the marker, §6), V1, V10 and V12 (V2, on a real DOMParser
  document, is spec-exact and does not flip). Obligation (1) is about §13.3's step 5.2 and so applies to the
  `HtmlFragment` family only: on the XMLSerializer path a noscript's Text stays escaped (§5.2.1.5 reads no parent;
  D-6). Obligation (2) flips V11 / V13 (§7 R2). The node clause's final representation:
  `#11-domparser-full-document-parse-fidelity` (ratified marker). ⚠ That slot is overdue (Re-eval 2026-11-01).
- DOMParser's bare `<noscript>` parsing as raw text (html5ever fragment fallback ignores the scripting flag):
  `#11-domparser-full-document-parse-fidelity`; V2 serializes that parse shape and needs no change if the parse is
  later fixed (data would then be elements, serialized normally).
- `GetInnerHtml` (the `"innerHTML.get"` `DomApiHandler`, `markup.rs:128`): left untouched. It belongs to the class
  of registry handlers with no production dispatcher, whose home is the registered slot
  `#11-dom-api-handlers-unreachable-from-production` (ledger, PM 583/744); the class's census is that slot's, so this
  memo carries none. Deleting one member here would be a partial sweep of that class; and under D-3 `GetInnerHtml` is not a divergent
  reading (it calls the same `serialize_inner_html`, with the natives' output). P6 F6 / F7 (`"insertAdjacentHTML"`,
  `"innerHTML.set"`) are members of the same class on the parse side.

## §10. Edges, hand-offs, definition of done

DoD:
1. D-1..D-4 and D-6 implemented as §4; `escape_html` / `escape_attr` gone (`git grep -n -w
   'escape_attr\|escape_html' -- crates` → 0); exactly three escaping functions (I-1): `git grep -n -F -e '&amp;' -e
   '&lt;' -e '&gt;' -e '&quot;' -e '&nbsp;' -e '&#9;' -e '&#xA;' -e '&#xD;' -- crates ':!*tests*'
   ':!crates/dom/elidex-html-parser*'` → apart from comment lines, every hit is inside the body of
   `push_escaped_string` (`markup.rs`), `push_xml_text` or `push_xml_attribute_value` (`xml_serialization.rs`), and
   `&nbsp;` occurs only in the first; each of the three is called from exactly one arm of `push_text_data` /
   `push_attribute_value` (`git grep -n -w 'push_escaped_string\|push_xml_text\|push_xml_attribute_value' -- crates
   ':!*tests*'`: apart from comment lines, 3 definitions + 4 calls — `push_escaped_string` twice, once per mode);
   `CharacterEscaping::XmlSerialization` (NEW) is constructed only in `serialize_node_to_string` (`git grep -n -F
   'XmlSerialization' -- crates ':!*tests*'`: apart from comment lines, the variant, the two `match` arms and one
   construction); no scripting read added (`git grep -n
   'ScriptingContext\|enabled_for_node' -- crates` → 0; `git diff origin/main --
   crates/script/elidex-script-session` → empty; `git diff origin/main -- crates/script/elidex-js/src/vm/host`
   touches `dom_parser.rs` only and every changed line is a comment: `git diff origin/main --
   crates/script/elidex-js/src/vm/host | /usr/bin/grep -E '^[+-]' | /usr/bin/grep -v -E
   '^(\+\+\+|---)|^[+-][[:space:]]*//'` → no output);
   `RAW_TEXT_ELEMENTS` is read only inside `text_children_literal` (`git grep -n -w 'RAW_TEXT_ELEMENTS' -- crates`:
   apart from comment lines, exactly 2 hits — the `const` definition and one read, inside `fn text_children_literal`'s
   body).
2. Every cell of §6 present and green; every listed mutant observed red at the final head (mutation runs only at the
   final head; ≤2 in parallel).
3. `cargo test -p elidex-dom-api -p elidex-js --all-features`, then `mise run ci`.
4. Comments in `markup.rs` cite §13.3 step 5.2, "Escaping a string" and §2.1.3 by name; the docstrings of
   `push_xml_text` / `push_xml_attribute_value` carry DOM Parsing §5.2.1.5 `#dfn-xml-serializing-a-text-node` and
   §5.2.1.1.3 `#dfn-serializing-an-attribute-value` with the steps they implement and the *require well-formed*
   note (D-6); the XMLSerializer cite is HTML §8.5.9 in `markup.rs` and `dom_parser.rs` (`git grep -n -F '8.5.8' --
   crates/dom/elidex-dom-api/src/element/markup.rs crates/script/elidex-js/src/vm/host/dom_parser.rs` → 0) —
   numbers from §0.4's lookups only.
5. The PR body carries §7 (R1, R2 and their cells) verbatim.
6. Every accepted-regression cell (U6's four functions, V1, V10, V11, V12, V13) has a code comment naming
   `#11-scripting-disabled-predicate-no-document` and the obligation that flips it (§6 U6, §7 R2). Check:
   `git grep -n -F '#11-scripting-disabled-predicate-no-document' -- crates/dom/elidex-dom-api/src/element/tests_markup.rs
   crates/script/elidex-js/src/vm/tests/tests_html_serialize_escape.rs crates/dom/elidex-dom-api/src/element/markup.rs`
   → the hits' enclosing items are exactly these 10 (at least one hit in each, none elsewhere):
   `tests_markup.rs` `u6_noscript_escaped_bound_like_entry`, `u6_noscript_escaped_bound_like_recursion`,
   `u6_noscript_escaped_plain_document_entry`, `u6_noscript_escaped_plain_document_recursion`;
   `tests_html_serialize_escape.rs` `v1_page_noscript_escaped`, `v10_inner_html_round_trip_scripting_enabled`,
   `v11_inner_html_round_trip_scripting_disabled`, `v12_outer_html_round_trip_scripting_enabled`,
   `v13_outer_html_round_trip_scripting_disabled`; `markup.rs` the doc comment on `fn text_children_literal` (one
   item; `RAW_TEXT_ELEMENTS`' comment does not name the slot, D-3). No test comment claims "does not flip" for a
   unit fixture (`git grep -n -F 'does not flip' -- crates/dom/elidex-dom-api/src/element/tests_markup.rs` → 0).

Landing order: no recorded order between this PR and the memos / PRs that pin lines it moves; whichever lands second
re-pins. This PR does not edit `dom_inner_html.rs`, so line cites there (brand-check) are unaffected; in
`dom_parser.rs` it rewrites comments at four places (§4.2), so line cites into that file after `:37` may shift and
follow the same rule. It moves lines in:
- `crates/dom/elidex-dom-api/src/element/markup.rs`. The pin list is a census over in-flight branches, not a named
  list (re-run 2026-11-18; re-run before landing). The command quotes every expansion and passes file names
  NUL-separated, so it gives the same 8 lines under `bash` and `zsh` (checked with both; Rev 4's form relied on
  unquoted word splitting, which `zsh` does not do):
  ```sh
  git for-each-ref --format='%(refname:short)' refs/heads refs/remotes/origin | while IFS= read -r b; do
    [ "$(git rev-list --count "origin/main..$b" 2>/dev/null || echo 0)" -gt 0 ] || continue  # ahead of origin/main
    git diff --name-only -z "origin/main...$b" -- docs | xargs -0 git grep -n -P 'markup\.rs(:[0-9]|`)' "$b" --  # docs the branch changes
  done | grep -v html-fragment-serializer-escape | sort -u
  ```
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
- `crates/dom/elidex-dom-api/src/lib.rs`: this PR adds the two `mod xml_serialization…` lines (§4.1a), and
  AbortSignal.any PR-A (branch `abort-signal-any-flatten-pr-a`) adds `pub mod abort;` to the same file — P1's
  `lib.rs:40` and P7's `lib.rs:93` shift under either; the two insertions are at opposite ends of the `mod` list.
  PR-A also adds `mod tests_abort_dependent;` to `crates/script/elidex-js/src/vm/tests/mod.rs` (this PR adds `mod
  tests_html_serialize_escape;` to the same list — a textual neighbour, no semantic conflict).

Landing (pmo, after merge):
- `#11-html-fragment-serializer-conformance`: (e) closed; (d) closed except the precise noscript rule, which moves to
  `#11-scripting-disabled-predicate-no-document` (PM 731); (a) gains "the void check (`markup.rs` `VOID_ELEMENTS`) has
  no namespace check" (§9); (a) (b) (c) (f) open, Re-eval 2026-12-15. Item (b)'s text names
  `serialize_inner_html_with_options`, which this PR keeps (D-5 withdrawn), so it stands as written.
- `#11-scripting-disabled-predicate-no-document`: the PM 731 facet is already recorded; append:
  (i) "serializer side: noscript always escaped since this PR; accepted regressions R1/R2, split by the HTML
  §8.1.3.4 value for the context document (what N4's `EcsDom::scripting_disabled_for` returns): scripting
  **enabled** — U6 (four unit functions; two fixtures, the cached document root and a plain `create_document_node`
  Document, neither carrying the null-browsing-context marker, so N4 answers enabled for both) / V1 / V10 / V12,
  flip at obligation (1), the precise serializer rule, which applies to the HTML fragment entry points only (on the
  XMLSerializer path a noscript's Text is escaped by DOM Parsing §5.2.1.5, which reads no parent); scripting **disabled** — V11 / V13 (DOMParser
  documents), flip at obligation (2), the parse side; V2 (a real DOMParser document) is spec-exact and does not
  flip; each flipping cell's comment names this slot. N4 may rebuild U6's plain-document fixture with the marker, in
  which case that pair stays escaped as the disabled side";
  (ii) "obligation (2)'s population = the 8 production fragment-parse paths that set the parser's scripting flag
  without consulting the context document (serializer memo P6 F1-F8: Element / ShadowRoot `innerHTML` and
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
  holds after this PR: the serializer reads no predicate; the Resolution's serializer-side item (removing the
  serializer's separate reading) is done by this PR. What remains of the Defect is the canonical function's fail-open
  no-document reading (aligned at N4) and obligations (1) / (2)".
- `#11-dom-api-handlers-unreachable-from-production` (registered, PM 583/744): cite this PR in that slot (it left
  `GetInnerHtml` untouched pending the class, §9); the parse side (P6 F6 / F7, `"insertAdjacentHTML"` /
  `"innerHTML.set"`) also belongs to obligation (2)'s population.
- `#11-domparser-xml-real-parsing` (PM 785): the two escaping functions (parts of its XML serializer) landed in this
  PR. In the row (`project_open-defer-slots.md`, read 2026-11-18), replace
  - old: "real XML parsing (XML/XHTML, not html5ever) + XML **serialization** (void self-close, namespace prefixes) +
    `XMLSerializer.serializeToString` **Attr-root** contract."
  - new: "real XML parsing (XML/XHTML, not html5ever) + XML **serialization** (void self-close, namespace prefixes,
    Text escaped under raw-text parents, the *require well-formed* = true checks) +
    `XMLSerializer.serializeToString` **Attr-root** contract. **Landed in the serializer PR (PM 785)**: the two
    escaping functions (parts) — `push_xml_text` (DOM Parsing §5.2.1.5 *XML serializing a Text node*, steps 3-5) and
    `push_xml_attribute_value` (§5.2.1.1.3 *serializing an attribute value*, step 3), in
    `crates/dom/elidex-dom-api/src/xml_serialization.rs`; `serialize_node_to_string` already calls them, on the HTML
    walk. The XML serializer extends that module and uses both as they are."

  and in the same row's **Why deferred**, replace
  - old: "all 5 accepted MIME currently HTML-parsed + HTML-serialized"
  - new: "all 5 accepted MIME currently HTML-parsed + serialized by the HTML walk (with DOM Parsing's two escapes
    since the serializer PR)"

  The rest of the row (the boa-parity ground, Trigger, Date) is unchanged by this PR.
- No facet on `#11-browsing-context-state-ecs-components` (PM 731).

## §11. Open questions

- Q-1 — decided (PM 728): the HTML-namespace conjunct is part of (d) (D-4).
- Q-2 — decided (PM 728 / 731): the no-document reading is `#11-scripting-disabled-predicate-no-document`'s, aligned
  at N4; this PR adds no reading (D-3).
- Q-3 — decided here: `GetInnerHtml` untouched (§9). The class it belongs to is
  `#11-dom-api-handlers-unreachable-from-production` (registered, PM 583/744); §10's landing item cites this PR
  there.
- Q-4 — decided (PM 785): the XMLSerializer path uses DOM Parsing's two escaping algorithms as two functions, not a
  mode of `push_escaped_string` (D-6, option (b2)). No open question remains here.
