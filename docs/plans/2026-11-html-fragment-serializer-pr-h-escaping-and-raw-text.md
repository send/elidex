# HTML fragment serializer PR-H — *escaping a string*, the raw-text parents, noscript always escaped (per-PR plan-memo)

Slice **PR-H** of the umbrella
[2026-11-html-fragment-serializer-umbrella.md](2026-11-html-fragment-serializer-umbrella.md) (slot
`#11-html-fragment-serializer-conformance`, items (e) and the security part of (d); PM 724; scope re-cut by PM 731,
option C; split and order by PM 797). Lands **second**, stacked on PR-X
([2026-11-html-fragment-serializer-pr-x-xml-family.md](2026-11-html-fragment-serializer-pr-x-xml-family.md)).
Base: PR-X's head while stacked, `origin/main` once PR-X has landed (written `<base>` below). Every code fact below
was measured at `c0578223`, PR-X's base (2026-11-15, or 2026-11-18 where marked); PR-X moves lines in `markup.rs`,
so the cites are re-measured at `<base>` before implementation — the commands do not move (umbrella §4.1). Owner
lane: L3 DOM/form. Security-relevant: closes the markup-injection (mXSS) defects of `innerHTML` / `outerHTML` /
`getHTML` (umbrella S-3).

Stated in the umbrella and only referred to here: the split, the order and the exposure window (S-1..S-3), the
shared premises P1 / P2 with the HEAD literal set, I-1, I-3, the pair matrix, the cross-lane pins and the ledger
edits. IDs are program-wide (umbrella §1 index); an ID not defined in this memo is defined in the file that index
names. Revision history: the umbrella (Rev 1-6 of the retired single memo; Rev 7 = this split; Rev 8 = R1 fixes).

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

These are the three §13.3 entries (`innerHTML` / `outerHTML` / `getHTML`). The fourth serializer entry,
`XMLSerializer.serializeToString`, is on its own escaping family since PR-X and reads no raw-text decision, so this
PR edits only the `HtmlFragment` arms of the dispatch PR-X introduced and the predicate behind `in_raw_text`.

### §0.2 Premises re-measured at `c0578223`

P1 (HEAD's two escapers) and P2 (HEAD's raw-text list, its two namespace-blind read sites, the HEAD literal set):
umbrella §0.2. P7 (the XMLSerializer path): PR-X §0.2.

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
- P5: no general document-level ECS fact "this Document has a browsing context" exists (`EcsDom::document_root()`,
  `crates/core/elidex-ecs/src/dom/mod.rs:491`, is a cache set by any `create_document_root` call). One document kind
  carries the null case by construction: the inert template document, which
  `appropriate_template_contents_owner_document` creates with `create_document_node` and marks
  `InertTemplateDocument` (`crates/core/elidex-ecs/src/dom/template_owner.rs:55-56`; the marker's doc: "an inert
  template document, whose browsing context is null", `crates/core/elidex-ecs/src/components.rs:388-394`). The node
  clause's ratified final representation is the enriched `#11-domparser-full-document-parse-fidelity`
  null-browsing-context marker (IBP-sandbox §6 OO → ECS row, `:661-663`; records `:706-710`; PM 795: owner L2
  Layout, lands at IBP-sandbox N5), which must read or merge that existing component rather than add a second
  writer of the same fact — a facet recorded on that slot at this PR's landing (umbrella §5).
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

### §0.3 Decisions (this PR)

- **D-1 — one escaping function with a mode for HTML §13.3** (One issue, one way). `escape_html` and `escape_attr`
  are deleted. Their only callers at `<base>` are the `HtmlFragment` arms of `push_text_data` /
  `push_attribute_value` (PR-X D-6), i.e. the three §13.3 entry points (`serialize_inner_html`,
  `serialize_inner_html_with_options`, `serialize_outer_html`); there they are replaced by
  `push_escaped_string(out: &mut String, s: &str, mode: EscapingMode)` (NEW) with `EscapingMode { Text, Attribute }`
  (NEW), a private item of `markup.rs` (its only consumer; §13.3-specific, so it leaves `util.rs`).
  `serialize_node_to_string` already runs D-6's two functions (PR-X) and is not touched.
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
- **D-6** (the family selection and the XML family) is PR-X's. What this PR does to its dispatch is exactly:
  `push_text_data`'s `HtmlFragment` arm becomes §13.3 step 5.2's Text branch — the data verbatim if `in_raw_text`,
  else `push_escaped_string(.., Text)`; `push_attribute_value`'s `HtmlFragment` arm becomes `push_escaped_string(..,
  Attribute)`. The `XmlSerialization` arms, `CharacterEscaping`, `Walk` and `xml_serialization.rs` are not edited,
  and the XML family's output does not depend on the raw-text list this PR changes (PR-X D-6, staging).

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

HTML §8.5.9 and DOM Parsing §5.2.1.5 / §5.2.1.1.3 (cited where this memo says what the XMLSerializer path does: V2's
second half, §5.2, §9) are in PR-X's table (PR-X §0.4), which holds their lookups.

`webref coverage-map html 13.3` → spec = 1, "ok → single PR scope" (2026-11-15). §2.1.3 / §8.1.3.4 / §7.1.5 /
§13.4 / §13.2.6.4.7 / §13.2.6.5 / §4.12.3 are definitional or context cites; this PR implements §13.3 steps.

## §1. Shape and verdict

One PR, one algorithm crate (`elidex-dom-api` serializer) plus VM end-to-end cells. It is a terminal slice of the
umbrella (S-1, S-2): two axes, (i) the character map of escaping and (ii) the raw-text decision for a Text node's
parent (the unconditional list and the HTML-namespace conjunct), on one family; axis (iii) is fixed by PR-X and
pinned by PR-X's cells, which this PR leaves green and unedited (DoD 1). No scripting input, no new ECS state, no
event / mutation / identity interaction. One normal `/elidex-plan-review`.

## §2. Invariants

I-1 (whose end state this PR reaches), I-3 and the three-axis pair matrix are the umbrella's (§2). This PR pins the
`HtmlFragment` side of the matrix's (i) × (ii) row (U4, U6, V5) and of its (i) × (iii) row (U1-U3, V6), and takes
over the literal side of (ii) × (iii) (U4, generalising PR-X's XC1). Its own invariants:

- I-2 (raw text), **`HtmlFragment`** (the three §13.3 entry points): for a Text node whose parent is an Element and
  which is reached through one of the two read sites (P2: the entry `serialize_inner_html_with_options` with that
  Element as *the node*, or the `serialize_node` recursion from that Element), the Text is emitted literally iff
  `text_children_literal(parent)`; the predicate is computed in exactly one function used at both read sites, and
  is false for every noscript and every non-HTML-namespace parent. Outside this scope: a bare Text node passed to
  `serialize_outer_html` is escaped without consulting its parent (the walk starts with `in_raw_text = false`,
  `markup.rs:162`); §13.3 takes "a DOM Element, Document, or DocumentFragment" as *the node*, so that input is not
  step 5.2's (the `XMLSerializer` route to a bare Text, and the `XmlSerialization` half of I-2, are PR-X §2).
- I-1 × I-2, **`HtmlFragment`**: an attribute value inside a noscript is serialized with `<` and `>` escaped
  (*escaping a string*, attribute mode). Re-parsing the output in a scripting-enabled context (where noscript is
  RAWTEXT) closes the noscript only at a `</noscript` sequence, which needs a raw `<`, so the argument holds through
  `<` alone and does not depend on `>`. V5 pins it. The `XmlSerialization` side is PR-X's (X5).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §13.3 Serializing HTML fragments | Escaping a string steps 1-4 | text mode (Text data outside raw-text parents), §13.3 entry points | `push_escaped_string(.., Text)`, the `HtmlFragment` arm of `push_text_data` (`serialize_node` Text branch) | ✓ | yes |
| WHATWG HTML §13.3 Serializing HTML fragments | Escaping a string steps 1-5 | attribute mode (attribute values; the synthetic `is` value), §13.3 entry points | `push_escaped_string(.., Attribute)`, the `HtmlFragment` arm of `push_attribute_value` (the `is` and attribute-loop sites) | ✓ | yes |
| WHATWG HTML §13.3 Serializing HTML fragments | step 5.2 (Text node) | parent ∈ {style, script, xmp, iframe, noembed, noframes, plaintext}, HTML namespace → literal (§13.3 entry points only) | `text_children_literal` + the `HtmlFragment` arm of `push_text_data` | ✓ | yes |
| WHATWG HTML §13.3 Serializing HTML fragments | step 5.2 (Text node) | parent noscript → escaped in every document (PM 731) | `text_children_literal` (false) | ✗ (the "scripting is enabled for the node" → literal branch is not taken; N4, §9) | yes |
| WHATWG HTML §13.3 Serializing HTML fragments | step 5.2 (Text node) | otherwise (incl. same-named non-HTML-namespace parent) → escaped | `serialize_node` Text branch | ✓ | yes |

**Breadth**: K=1 spec (HTML), M=5 entries (verified 2026-11-18: preflight `total entries (M): 5`, `parsed
citations: 5`, `citation verify: ok`). **Split decision**: single PR (a slice of the umbrella).

### §3.1 User-input touch audit

- Text data, attribute values and `is` values are author-controlled (`textContent`, `setAttribute`,
  `createElement(.., {is})`, any parse). This PR changes what three natives emit: the `innerHTML` / `getHTML` /
  `outerHTML` getters (`crates/script/elidex-js/src/vm/host/dom_inner_html.rs:116`, `:317`, `:339`). The natives'
  code is not edited (§4.2); exposure decreases through the shared serializer. `XMLSerializer.serializeToString`
  (`crates/script/elidex-js/src/vm/host/dom_parser.rs:404`) is not changed by this PR (PR-X §3.1).
- Adjacent pre-existing: the Comment branch (`markup.rs:382-386`, data verbatim per spec) and attribute-name
  filtering (`is_safe_attr_name`, `markup.rs:267`) — untouched; exposure unchanged.

## §4. Mechanism

### §4.1 `elidex-dom-api` — `markup.rs`

- `EscapingMode` + `push_escaped_string` (NEW, private), D-1. `util.rs` loses `escape_html` / `escape_attr` and their
  two tests (moved as cells, §6).
- The `HtmlFragment` arms of `push_text_data` / `push_attribute_value` (PR-X) call `push_escaped_string` (D-1, and
  D-6's line in §0.3); the `use crate::util::{escape_attr, escape_html}` import goes. The `XmlSerialization` arms,
  `CharacterEscaping`, `Walk` and `xml_serialization.rs` are not edited.
- `RAW_TEXT_ELEMENTS` per D-2 (doc comment per D-3).
- `text_children_literal` (NEW, private) per D-2/D-4; called at the two read sites (P2).
- The comment at the `is` site that names `escape_attr` (`markup.rs:411` at `c0578223`) and the test message
  `tests_markup.rs:200` are reworded (§5.1).
- `SerializeOptions`, the public entry points' signatures, `GetInnerHtml` and the re-export lists are unchanged
  (D-5 withdrawn).

### §4.2 `elidex-js` — no change

No file under `crates/script/elidex-js/src/vm/host` is edited, comments included (D-3: no context to marshal; the
comments PR-X wrote in `dom_parser.rs` describe the XMLSerializer path, which this PR does not change).
`elidex-script-session` is not edited.

### §4.3 ECS-native / layering check

No side-store, no new component, no new state read. OO → ECS mapping of the concepts this PR touches (the scripting
rows are IBP-sandbox §6's, `docs/plans/2026-09-ibp-sandbox.md:659-668`, not this PR's):

| Spec / OO concept | elidex (ECS) | This PR |
|---|---|---|
| "an HTML element" (§2.1.3, namespace) | `EcsDom::is_html_namespace(entity)` | read (existing) |
| element type's local name | `TagType` component | read (existing) |
| Text node's data | `TextContent` component | read (existing) |
| *escaping a string* (attribute-mode flag) | `push_escaped_string(.., EscapingMode)` — a pure function, no state | NEW |
| "scripting is enabled for the node" | `EcsDom::scripting_disabled_for(.., ScriptingSubject)` (NEW: N4, D8); node clause = the `#11-domparser-full-document-parse-fidelity` marker | not read (D-3) |

**The namespace read and its writers.** D-4 makes `text_children_literal` read the parent's namespace through
`EcsDom::is_html_namespace` → `namespace_of`, which returns the `Namespace` component if present and
`Namespace::Html` if absent (`crates/core/elidex-ecs/src/dom/classify.rs:76-79`, at `c0578223`): the component marks
only the foreign exception. Its production writers (`git grep -n -w 'Namespace' c0578223 -- crates ':!*tests*'`,
read per hit, 2026-11-18): `EcsDom::create_element_ns` inserts it for a non-HTML namespace
(`crates/core/elidex-ecs/src/dom/mod.rs:444`), reached from the strict parser's *insert a foreign element*
(`elidex-html-parser-strict/src/tree_builder/insert.rs:116`) and the tolerant parser's conversion
(`elidex-html-parser/src/convert.rs:258`); a clone copies it (`crates/core/elidex-ecs/src/dom/tree_clone.rs:308`
reads the source's, `:359-360` adds it to the clone). `elidex-custom-elements/src/created_element.rs` takes the
namespace as a parameter and writes no `Namespace`. elidex-js has no `createElementNS` native (`git grep -n -F
'createElementNS' c0578223 -- crates/script ':!*tests*'` → 0; control: `'"createElement"'` → `document.rs:373`). So an element is foreign to this read iff a parser
or a clone of a parsed foreign element made it so; any other element reads as HTML, and on the raw-text decision
that is the literal (unsafe) side for a foreign element whose local name is in the list. That is the engine-wide
representation, not this PR's (D-4 reads it as it is); a future foreign-element creator must write the component
through `create_element_ns`.

Algorithms live in `elidex-dom-api` (§13.3 in `element/markup.rs`); `vm/host/` is untouched (§4.2).

## §5. Populations

### §5.1 Escaping callers census

At `c0578223`, and what PR-X does to it: PR-X §5.1 (11 hits; PR-X turns the 3 production calls into 2, inside the
`HtmlFragment` arms). So at `<base>` `git grep -n -w 'escape_attr\|escape_html' -- crates` is expected to give 10
hits (re-measure before implementation): 2 definitions and 3 unit asserts in `util.rs`, the import and 2 calls in
`markup.rs`, the comment at the `is` site, the test message in `tests_markup.rs`. This PR replaces the 2 calls
(D-1), deletes the definitions, their tests and the import, and rewords the comment and the test message → 0 hits
(DoD 1). After this PR the concept sweep of PR-X §5.1 finds exactly the three functions of I-1 (DoD 1).

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
argument (`false`) is unchanged (I-2's scope note). The grep gives 6 hits = 5 calls (`:162`, `:247` after the read
`:243`, `:351`, `:449` after the read `:445`, `:457`) + the definition `:356`, verified 2026-11-15. PR-X changes
none of these computations (PR-X §5.2), and this PR changes only the two reads: the `in_raw_text` they feed is read
by `push_text_data`'s `HtmlFragment` arm alone.

### §5.3 Files touched (`wc -l` at `c0578223`)

`markup.rs`, `tests_markup.rs` and the VM test file are longer at `<base>` by PR-X's additions (PR-X §5.3).

| File | Lines | Change |
|---|---|---|
| `crates/dom/elidex-dom-api/src/element/markup.rs` | 459 | D-1, D-2, D-4 (~+25) |
| `crates/dom/elidex-dom-api/src/util.rs` | 233 | −2 fns, −2 tests (~−30) |
| `crates/dom/elidex-dom-api/src/element/tests_markup.rs` | 246 | new cells (~+120; XC1's two functions become U4) |
| `crates/script/elidex-js/src/vm/tests/tests_html_serialize_escape.rs` (NEW: PR-X) | — | V cells (~+110) |

## §6. Test and coverage plan

Supported surface: §13.3 escaping and the step-5.2 Text branch, guarded by engine-independent unit cells in
`elidex-dom-api` and VM end-to-end cells; no WPT subset is added by this PR. U1-U6 and V1-V13 exercise the §13.3
entry points. PR-X's cells (XT / XA, X1-X5, XC2) are not edited and stay green — the independence check of DoD 1.
XC2 (U+0009 raw in an attribute value through `outerHTML`) is exact for D-1 too: *escaping a string* has no U+0009
step.

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

**VM end-to-end, `tests_html_serialize_escape.rs` (NEW: PR-X)** (bound VM; real parse):
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

VM test function names (used by DoD 6): `v1_page_noscript_escaped`, `v10_inner_html_round_trip_scripting_enabled`,
`v11_inner_html_round_trip_scripting_disabled`, `v12_outer_html_round_trip_scripting_enabled`,
`v13_outer_html_round_trip_scripting_disabled`.

**U4 and PR-X's XC1.** XC1 (PR-X) is U4's script row through both entries. This PR generalises XC1's two functions
over the seven parents; they are U4-entry / U4-recursion from then on, and no second assertion of the script row
remains. **V2 and PR-X.** V2's `XMLSerializer` assertion is already green at `<base>` (PR-X escapes every Text on
that path); V2's `innerHTML` assertion is the one this PR turns green.

Each accepted-regression cell (U6 — all four functions —, V1, V10, V11, V12, V13) carries a code comment naming
`#11-scripting-disabled-predicate-no-document`, which obligation flips it (§7), and "PM 731" (DoD 6), so the
flip at N4 is found by grepping the slot id. V2 does not name the slot (it does not flip).

Rev 1's U7-U11, the `scripting.rs` unit cells and V3 are removed: each distinguished a scripting context that no
longer exists (bound / other / template / ownerless document, `None`, sandboxed); under D-3 all of them are U6.
(Cell numbers are not reused.)

**Existing tests whose expectations change** — census: `git grep -n -i 'noscript' -- crates/dom/elidex-dom-api
crates/script/elidex-js/src/vm/tests crates/shell` plus `&quot;|&lt;|&gt;|&amp;|nbsp` and `<svg…><style|script>`
in serializer tests (2026-11-15): `util.rs` `escape_html_chars` / `escape_attr_chars` are deleted with their
functions (their assertions are subsumed by U1/U2 with the same expected strings). `tests_markup.rs:20`
(`hello &lt;world&gt;`) and `:199` (`is="x&quot; onclick=&quot;evil"`) keep their expectations. No existing test
serializes a noscript, an SVG style/script, or an attribute containing `<`, `>` or U+00A0 (the noscript tests in
`tests_dom_parser.rs` assert parse shape only). The XMLSerializer path's tests: PR-X §6 (this PR does not change
that path). **No expectation flips.**

**Mutants** (each must turn a named cell red): drop each of the 5 escape arms (U1/U2); unguard step 5 (U1 `"`); an
arm re-escapes its own entity (U1/U2, see U3); remove plaintext (U4-entry, U4-recursion, V7); drop the namespace
conjunct in `text_children_literal` (U5-entry, U5-recursion); **one site not converted** — the entry site keeps
`RAW_TEXT_ELEMENTS.contains(tag)` instead of calling `text_children_literal` (U5-entry red, U5-recursion green), and
symmetrically the recursion site (U5-recursion red, U5-entry green); put noscript back in the raw list (U6/V1/V2/V4);
keep `escape_attr`'s old map at the attribute site (V5).
**The selection, re-run at this PR's head** (PR-X ran these against HEAD's escapers, PR-X §6; with
`push_escaped_string` in the `HtmlFragment` arms they have the red sets Rev 6 listed): `serialize_node_to_string`
passes `HtmlFragment` in its outer arm (X1, X3, X4's element half, X5 red; X2, X4's fragment half green) or in its
children arm (X2, X4's fragment half red; X1, X3, X4's element half, X5 green); `push_text_data`'s
`XmlSerialization` arm calls `push_escaped_string` (X1's Text assertion); `push_attribute_value`'s
`XmlSerialization` arm calls `push_escaped_string` (X1's attribute assertion, X3); and the reverse, an
`HtmlFragment` arm calling the DOM Parsing function (V6's Text half / V6's `title` half, U2's `>` cell and XC2) —
mutant (a), which PR-X could not observe, and mutant (b), whose attribute half PR-X kills with XC2 (PR-X §6).
**The family gate, re-run at this PR's head** (the (ii) × (iii) conjunct; PR-X ran it with XC1): drop it, so the
`XmlSerialization` arm emits the data verbatim when `in_raw_text` (X4, both halves, red; U4 green); and the
reverse, the `HtmlFragment` arm ignoring `in_raw_text` (U4-entry, U4-recursion, V7 red; X4 green).
Rev 1's per-native `None` / `getHTML`-omits-context mutants (axis-3 IMP-2) disappear: no native passes a context
any more, so there is no per-native input to mutate; every native reaches the one serializer, whose mutants are
above.

## §7. Accepted regressions (PM 731)

Copied verbatim into the PR body (§10 DoD 5). Each is pinned by a cell, so it is an expectation, not a hidden failure.
Both are on the §13.3 entry points. The XMLSerializer path has no accepted regression (PR-X §7) and is not changed
by this PR.

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
    *destroy a document* (`docs/plans/2026-09-ibp-sandbox.md:706-710`). The inert template document already carries
    the same fact in another component: `InertTemplateDocument`, inserted at its creation
    (`crates/core/elidex-ecs/src/dom/template_owner.rs:55-56`; HTML §4.12.3 step 1.1.1, "its browsing context is
    null"; P5). So the obligation is not a new writer but that the marker read or merge that component (one fact, one
    component); until it does, N4 answers "enabled" for template content and its round trip is neither of the two
    cases above. Recorded as a facet of `#11-domparser-full-document-parse-fidelity`, which PM 795 made the home of
    the marker's obligations (umbrella §5, PR-H items); other no-browsing-context documents, if any are created,
    fall under the same slot. No cell here depends on it (V11 / V13 use DOMParser documents).

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

All touched files are under 1000 lines at `c0578223` (§5.3) and stay so with PR-X's additions (PR-X §5.3: `markup.rs`
~+45, `tests_markup.rs` ~+25, the VM test file ~80). VM cells go to PR-X's NEW file rather than
`tests_dom_parser.rs` (823) or `tests_inner_html_mixin.rs` (749), which would approach the line.

## §9. Out of scope, and slots

- The XML family (`serialize_node_to_string`, `xml_serialization.rs`) and the family selection: PR-X, landed
  before this PR and not edited by it. The rest of the XML serialization: `#11-domparser-xml-real-parsing` (PR-X
  §9).
- Slot items (a) void list, (b) step 1, (c) shadow-template attribute order + `shadowrootcustomelementregistry`,
  (f) attribute order (note: the spec says the order "is implementation-defined … must be stable", so (f) is a
  fidelity/allocation question, not an observable-deviation one — recorded for that facet, not decided here):
  stay in `#11-html-fragment-serializer-conformance`, Re-eval 2026-12-15. Item (a) also gains the void check's
  namespace defect (landing, umbrella §5): `VOID_ELEMENTS.contains` at `markup.rs:441` reads `TagType` only, so a
  non-HTML-namespace element with a void local name loses its children and end tag (same §2.1.3 reading as D-4,
  four lines from the raw-text read). **It stays out by PM 724's scope** — this PR is items (d) and (e) only, and the
  void list is item (a). **Why it is not a security issue** (unlike D-4, which is in (d) because it is the injection
  class): the void decision changes only whether children and the end tag are emitted. The namespace in which later
  siblings are re-parsed is decided by the start tag's name, which the void decision does not change. The serializer's
  unescaped outputs — the three character-data ones (a raw-text parent's Text data on the `HtmlFragment` family,
  `markup.rs:371`; comment data, `:384`; the doctype name, `:394`) and the names (tag names, `:400` / `:452`; attribute names, `:434`, filtered by
  `is_safe_attr_name`) — are each emitted, or not, independently of the void decision: it adds no unescaped
  output and changes the content of none. Its failure modes are dropped content (the children
  are not emitted at all) and, for the void names in `VOID_ELEMENTS` (`markup.rs:260`) that do not break out of
  foreign content — area, base, col, input, link, param, source, track, wbr (HTML §13.2.6.5's breakout start-tag list
  contains br, embed, hr, img and meta, and none of these) — an omitted end tag, so on re-parse the following siblings
  nest inside the element: a tree-shape (fidelity) defect, not an mXSS one.
- The precise noscript rule and the parse-side scripting decision (P6): `#11-scripting-disabled-predicate-no-document`
  (PM 731 facet; trigger IBP-sandbox N4's per-PR plan). Hand-off population for obligation (2) = P6 F1-F8 (all eight
  constructions, with their entry APIs; F6 / F7 are reachable only through the handler class below, F8's value is
  already right but is a literal). Obligation (1) is the serializer side: `text_children_literal` gains the noscript
  conjunct "and scripting is enabled for the node" read from N4's composition, flipping U6 (all four functions; the
  plain-document pair unless N4 rebuilds its fixture with the marker, §6), V1, V10 and V12 (V2, on a real DOMParser
  document, is spec-exact and does not flip). Obligation (1) is about §13.3's step 5.2 and so applies to the
  `HtmlFragment` family only: on the XMLSerializer path a noscript's Text stays escaped (§5.2.1.5 reads no parent;
  D-6). Obligation (2) flips V11 / V13 (§7 R2). The node clause's final representation:
  `#11-domparser-full-document-parse-fidelity` (ratified marker; Re-eval-date 2027-01-31, extended by PM 795, which
  places its content at IBP-sandbox N5, owner L2 Layout).
- DOMParser's bare `<noscript>` parsing as raw text (html5ever fragment fallback ignores the scripting flag):
  `#11-domparser-full-document-parse-fidelity`; V2 serializes that parse shape and needs no change if the parse is
  later fixed (data would then be elements, serialized normally).
- `GetInnerHtml` (the `"innerHTML.get"` `DomApiHandler`, `markup.rs:128`): left untouched. It belongs to the class
  of registry handlers with no production dispatcher, whose home is the registered slot
  `#11-dom-api-handlers-unreachable-from-production` (ledger, PM 583/744); the class's census is that slot's, so this
  memo carries none. Deleting one member here would be a partial sweep of that class; and under D-3 `GetInnerHtml` is not a divergent
  reading (it calls the same `serialize_inner_html`, with the natives' output). P6 F6 / F7 (`"insertAdjacentHTML"`,
  `"innerHTML.set"`) are members of the same class on the parse side.

## §10. Definition of done

1. D-1..D-4 implemented as §4; `escape_html` / `escape_attr` gone (`git grep -n -w
   'escape_attr\|escape_html' -- crates` → 0); exactly three escaping functions (umbrella I-1): `git grep -n -F -e
   '&amp;' -e '&lt;' -e '&gt;' -e '&quot;' -e '&nbsp;' -e '&#9;' -e '&#xA;' -e '&#xD;' -- crates ':!*tests*'
   ':!crates/dom/elidex-html-parser*'` → apart from comment lines, every hit is inside the body of
   `push_escaped_string` (`markup.rs`), `push_xml_text` or `push_xml_attribute_value` (`xml_serialization.rs`), and
   `&nbsp;` occurs only in the first; each of the three is called from exactly one arm of `push_text_data` /
   `push_attribute_value` (`git grep -n -w 'push_escaped_string\|push_xml_text\|push_xml_attribute_value' -- crates
   ':!*tests*'`: apart from comment lines, 3 definitions + 4 calls — `push_escaped_string` twice, once per mode);
   no scripting read added (`git grep -n 'ScriptingContext\|enabled_for_node' -- crates` → 0);
   `in_raw_text` is read for output only in `push_text_data`'s `HtmlFragment` arm (inspection of the
   `match`; X4, U4 and the family-gate mutants of §6 are the executable check);
   `RAW_TEXT_ELEMENTS` is read only inside `text_children_literal` (`git grep -n -w 'RAW_TEXT_ELEMENTS' -- crates`:
   apart from comment lines, exactly 2 hits — the `const` definition and one read, inside `fn text_children_literal`'s
   body).
   **Independence of the XML family**: `git diff <base>` restricted to PR-X's two files `xml_serialization.rs` /
   `xml_serialization_tests.rs` (both under `crates/dom/elidex-dom-api/src/`), to
   `crates/script/elidex-script-session` and to `crates/script/elidex-js/src/vm/host` → empty; `git diff <base> --
   crates/script/elidex-js/src/vm/tests/tests_html_serialize_escape.rs | /usr/bin/grep -c -E '^-[^-]'` → 0 (lines
   are only added, so X1-X5 and XC2 are unedited); `git grep -n -F 'XmlSerialization' -- crates ':!*tests*'` → the
   same non-comment hits as at `<base>` (the variant, the two `match` arms, one construction); X1-X5, XC2, XT1-XT3
   and XA1-XA3 green.
2. Every cell of §6 present and green; every listed mutant observed red at the final head (mutation runs only at the
   final head; ≤2 in parallel).
3. `cargo test -p elidex-dom-api -p elidex-js --all-features`, then `mise run ci`.
4. Comments in `markup.rs` cite §13.3 step 5.2, "Escaping a string" and §2.1.3 by name — numbers from §0.4's
   lookups only; PR-X DoD 4's header-cite check is re-run and still passes.
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

Cross-lane pins and the landing (ledger) items of this PR: umbrella §4.2 and §5.

## §11. Open questions

- Q-1 — decided (PM 728): the HTML-namespace conjunct is part of (d) (D-4).
- Q-2 — decided (PM 728 / 731): the no-document reading is `#11-scripting-disabled-predicate-no-document`'s, aligned
  at N4; this PR adds no reading (D-3).
- Q-3 — decided here: `GetInnerHtml` untouched (§9). The class it belongs to is
  `#11-dom-api-handlers-unreachable-from-production` (registered, PM 583/744); umbrella §5's landing item cites this PR
  there.
