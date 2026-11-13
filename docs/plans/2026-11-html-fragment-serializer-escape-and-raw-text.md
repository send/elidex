# HTML fragment serializer — *escaping a string* and the raw-text parents, incl. fail-safe noscript (per-PR plan-memo)

Slot `#11-html-fragment-serializer-conformance` (ledger: open, owner L3). This PR closes items **(d)** and **(e)**
only (PM 724, 2026-11-13; this facet's Re-eval 2026-11-20); items (a) (b) (c) (f) stay in the slot with Re-eval
2026-12-15. Base: `origin/main` `06bdfbbd`. Every code fact below was measured at `06bdfbbd` on 2026-11-13; line
numbers move, the commands do not. Owner lane: L3 DOM/form. Security-relevant: both items are markup-injection
(mXSS) surfaces of `innerHTML` / `outerHTML` / `getHTML` / `XMLSerializer`.

**Rev 1** (2026-11-13) — first draft.

## §0. Frame, premises, decisions

### §0.1 Frame

The HTML fragment serialization algorithm (HTML §13.3) has two decisions this PR makes exact:

- **(e) Escaping a string** — steps 1-5 (`webref body html serialising-html-fragments`, the "Escaping a string"
  list): 1 `&` → `&amp;`; 2 U+00A0 → `&nbsp;`; 3 `<` → `&lt;`; 4 `>` → `&gt;`; 5 *attribute mode only*: `"` →
  `&quot;`. Steps 1-4 apply in both modes.
- **(d) The Text-node branch of step 5.2** — "If the parent of current node is a style, script, xmp, iframe,
  noembed, noframes, or plaintext element, or if the parent of current node is a noscript element and scripting is
  enabled for the node, then append the value of current node's data literally. Otherwise, append the value of
  current node's data, escaped as described below." "*the node*" is the algorithm's input (the preamble: "takes as
  input a DOM Element, Document, or DocumentFragment referred to as the node"); when the Text node's parent is a
  noscript, the recursion that reaches it has that noscript as *the node* (or, for step 3, a template's contents).

### §0.2 Premises re-measured at `06bdfbbd`

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
  platform object if (1) it is disabled for the object's relevant settings object, or (2) "The object implements
  Node, and object's node document's browsing context is null", or (3) a Window whose associated Document's browsing
  context is null; enabled iff not disabled. So DOMParser documents and inert template documents (no browsing
  context) are scripting-disabled.
- P4: elidex already has the canonical, engine-independent composition of §8.1.3.4:
  `elidex_script_session::scripting_disabled_for_platform_object(dom, target, bound_document, sandbox_flags)`
  (`crates/script/elidex-script-session/src/scripting.rs:74`), used today by the event-handler compile gate
  (`crates/script/elidex-js/src/vm/host/event_handler_attrs.rs:543`, inputs from `HostData::document()` at
  `crates/script/elidex-js/src/vm/host_data/engine_feature/bind.rs:465` and `HostData::sandbox_flags()` at
  `crates/script/elidex-js/src/vm/host_data/engine_feature/context_config.rs:111`). Clause (2) is modelled as
  "node document ≠ the bound document" (single-browsing-context model). **Its unknown case is not fail-safe for
  this consumer**: a node with no node document yields "not disabled" (`is_some_and` at `scripting.rs:84-85`).
- P5: no document-level ECS fact "this Document has a browsing context" exists. `EcsDom::document_root()`
  (`crates/core/elidex-ecs/src/dom/mod.rs:491`) is a cache set by any `create_document_root` call (tests, parsers)
  and carries no sandbox flags, so it is not that fact. The parser has the fact only as an option
  (`scripting_disabled`, `crates/dom/elidex-html-parser-strict/src/result.rs:123`; set true for DOMParser at
  `crates/dom/elidex-form/src/inert_document.rs:147`). The positive fact's registered home is
  `#11-browsing-context-state-ecs-components` (C1 memo `docs/plans/2026-10-eh-attrs-c1-template-owner-document.md`
  D-C9, record R-8, PM 613).
- P6: `elidex-dom-api` already depends on `elidex-script-session` and `elidex-plugin`
  (`crates/dom/elidex-dom-api/Cargo.toml` lines 14/16), so the serializer can call P4 without a new edge.

### §0.3 Decisions (this PR)

- **D-1 — one escaping function with a mode** (One issue, one way). `escape_html` and `escape_attr` are deleted and
  replaced by `push_escaped_string(out: &mut String, s: &str, mode: EscapingMode)` (NEW) with `EscapingMode
  { Text, Attribute }` (NEW), a private item of `markup.rs` (its only consumer; §13.3-specific, so it leaves
  `util.rs`). Rationale: the spec defines one algorithm with an attribute-mode flag; two copies are exactly how
  P1's divergence arose. `push_*` writes into the output buffer (the current functions allocate a `String` per
  text node / attribute and the caller copies it). The match arms are spec steps 1-5 in order, step 5 guarded by
  `mode == Attribute`.
- **D-2 — one raw-text predicate.** `RAW_TEXT_ELEMENTS` becomes the spec's unconditional list (style, script, xmp,
  iframe, noembed, noframes, **plaintext**; noscript removed), and both read sites (P2) call one function
  `text_children_literal(dom, parent, opts) -> bool` (NEW) = `dom.is_html_namespace(parent)` ∧ (tag ∈ list ∨ (tag
  = noscript ∧ scripting known enabled for `parent`)). The `in_raw_text: bool` threading of `serialize_node` stays.
- **D-3 — where "scripting is enabled for the node" is answered (fail-safe; PM 724).** Raw only when scripting is
  **known** enabled; every unknown escapes. The answer is P4's existing composition, fed by the engine:
  - `ScriptingContext { bound_document: Entity, sandbox_flags: SandboxingFlagSet }` (NEW) in
    `elidex-script-session::scripting`, next to the predicate, with `ScriptingContext::enabled_for_node(&self,
    dom, node) -> bool` (NEW) = `dom.node_document(node).is_some() && !scripting_disabled_for_platform_object(dom,
    node, self.bound_document, self.sandbox_flags)`. The §8.1.3.4 composition stays in one function; the only
    addition is the fail-safe treatment of "no node document" (P4).
  - `SerializeOptions` gains `scripting: Option<ScriptingContext>` (NEW field); `None` (the `Default`) = unknown =
    escape. The VM natives (§4.3) fill it from `HostData::document()` + `HostData::sandbox_flags()`; every other
    caller (the `GetInnerHtml` `DomApiHandler`, unit tests) passes `None` and escapes noscript text.
  - Outcomes: bound page document, unsandboxed → raw (today's output kept); bound page under a sandbox without
    `allow-scripts` → escaped (clause 1); DOMParser document / inert template document / any non-bound document →
    escaped (clause 2); ownerless node → escaped (fail-safe); no context → escaped (fail-safe).
  - **Candidates rejected.** (B) a marker component on Documents that have a browsing context, written at bind:
    that is the positive fact `#11-browsing-context-state-ecs-components` owns (P5) — minting it here would put a
    second source of truth for "has a browsing context" beside the bound-document input the event-handler gate
    reads (two ways for one fact), and decide that program's writer/lifetime questions (bind/unbind, B1 root set)
    from a serializer PR. (C) always escape noscript text: safe, but regresses the bound page whose output is
    correct today, while the exact answer is computable now from P4. (D) `document_root()` as "the bound
    document": unsound (P5).
  - **What stays unknown, and who owns it.** Only the *input* of clause (2) is interim: elidex answers "this
    document has a browsing context" as "it is the bound document" because no ECS fact exists. When
    `#11-browsing-context-state-ecs-components` lands the fact, `ScriptingContext::bound_document` (NEW) retires in favour
    of reading it, and the serializer is unchanged (it only calls `enabled_for_node`). Landing appends this as a
    facet of that slot (§9). Clause (1)'s user-disable and WebDriver BiDi conditions are not modelled engine-wide
    (as in P4's doc comment); unaffected here.
- **D-4 — the HTML-namespace check is part of (d).** "a style, script, … element" in step 5.2 means HTML elements;
  P2 shows the namespace-blind check. `<svg><style>` text containing `</style><img onerror=…>` is entity-decoded by
  the parser (foreign content) and today re-emitted literally — the same injection class as (d). Fixed by D-2's
  `is_html_namespace` conjunct (one predicate, no extra site). **PM to ratify as part of (d)**; if declined it
  becomes slot item (g) and the conjunct is dropped (§10 Q-1).
- **D-5 — every public serializer entry point takes `&SerializeOptions`.** Today `serialize_inner_html` /
  `serialize_outer_html` / `serialize_node_to_string` hard-code `SerializeOptions::default()` and only
  `serialize_inner_html_with_options` takes options. With D-3 the default means "scripting unknown", so a
  convenience form would let an engine caller silently lose fidelity. The entry points become
  `serialize_inner_html(entity, dom, opts)`, `serialize_outer_html(entity, dom, opts)`,
  `serialize_node_to_string(entity, dom, opts)`; `serialize_inner_html_with_options` is deleted (its body becomes
  `serialize_inner_html`). Every caller states its context (15 sites, verified 2026-11-13 by the §5.2 grep).

### §0.4 Spec citation table

| Cite | Title | Anchor | Lookup (2026-11-13) |
|---|---|---|---|
| HTML §13.3 | Serializing HTML fragments | `#serialising-html-fragments` | `webref body html serialising-html-fragments` (step 5.2 Text branch; Escaping a string steps 1-5; preamble "the node") |
| HTML §8.1.3.4 | Enabling and disabling scripting | `#enabling-and-disabling-scripting`, `#concept-n-script`, `#concept-n-noscript` | `webref body html enabling-and-disabling-scripting`; `webref dfn html 'scripting is disabled'` |

`webref coverage-map html 13.3 html 8.1.3.4` → spec = 1, step = 2 entries (verified 2026-11-13), "ok → single PR scope".

## §1. Shape and verdict

One PR, one crate pair of algorithm changes (`elidex-dom-api` serializer, `elidex-script-session` context type) plus
the VM natives that fill the context. **Not edge-dense** (PM agreed): two axes — (i) the character map of escaping,
(ii) the raw-text decision for a Text node's parent. The scripting answer is an *input* to axis (ii) computed by an
existing canonical function, not a third invariant axis; no new ECS state, no event/mutation/identity interaction.
One normal `/elidex-plan-review`.

## §2. Invariants

- I-1 (escaping): every non-literal Text datum and every attribute / `is` value passes through
  `push_escaped_string`; no other escaping code exists in `crates/` (concept sweep, §5.1).
- I-2 (raw text): a Text child is emitted literally iff `text_children_literal(parent)`; the predicate is computed
  in exactly one function used at both read sites.
- I-3 (fail-safe): `text_children_literal` returns true for a noscript parent only if `opts.scripting` is `Some`
  and `enabled_for_node` is true; every other path escapes.
- I-1 × I-3: an attribute value inside a noscript in a scripting-disabled document is serialized with `<`/`>`
  escaped, so re-parsing the output in a scripting-enabled context (where noscript is RAWTEXT) cannot close the
  noscript early — the attribute cell of §6 pins this intersection.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §13.3 Serializing HTML fragments | Escaping a string steps 1-4 | text mode (Text data outside raw-text parents) | `push_escaped_string(.., Text)` in `serialize_node` Text branch | ✓ | yes |
| WHATWG HTML §13.3 Serializing HTML fragments | Escaping a string steps 1-5 | attribute mode (attribute values; the synthetic `is` value) | `push_escaped_string(.., Attribute)` at the `is` and attribute-loop sites | ✓ | yes |
| WHATWG HTML §13.3 Serializing HTML fragments | step 5.2 (Text node) | parent ∈ {style, script, xmp, iframe, noembed, noframes, plaintext}, HTML namespace → literal | `text_children_literal` | ✓ | yes |
| WHATWG HTML §13.3 Serializing HTML fragments | step 5.2 (Text node) | parent noscript ∧ scripting enabled for the node → literal | `text_children_literal` → `ScriptingContext::enabled_for_node` | ✓ | yes |
| WHATWG HTML §13.3 Serializing HTML fragments | step 5.2 (Text node) | otherwise (incl. same-named non-HTML-namespace parent) → escaped | `serialize_node` Text branch | ✓ | yes |
| WHATWG HTML §13.3 Serializing HTML fragments | step 3 (template → template contents) | noscript inside template contents (inert template document) | `serialization_children_root` (unchanged) feeding clause (2) below | ✓ | yes |
| WHATWG HTML §8.1.3.4 Enabling and disabling scripting | scripting is disabled for a platform object, clause 1 (relevant settings object) | sandboxed scripts flag set / unset | `scripting_disabled_for_platform_object` (unchanged) | ✗ (user-disable, WebDriver BiDi: not modelled engine-wide) | no |
| WHATWG HTML §8.1.3.4 Enabling and disabling scripting | clause 2 (Node, node document's browsing context null) | bound document / other document / no node document (elidex-only, fail-safe) | `ScriptingContext::enabled_for_node` | ✓ | yes |

**Breadth**: K=1 spec, M=8 entries (verified 2026-11-13: preflight `total entries (M): 8`). **Split decision**: single PR.

### §3.1 User-input touch audit

- Text data, attribute values and `is` values are author-controlled (`textContent`, `setAttribute`,
  `createElement(.., {is})`, any parse). All four serializer natives emit them: `innerHTML` / `getHTML` / `outerHTML`
  getters (`crates/script/elidex-js/src/vm/host/dom_inner_html.rs:116`, `:317`, `:339`) and
  `XMLSerializer.serializeToString` (`crates/script/elidex-js/src/vm/host/dom_parser.rs:404`). Exposure decreases.
- Adjacent pre-existing: the Comment branch (`markup.rs:382-386`, data verbatim per spec) and attribute-name
  filtering (`is_safe_attr_name`, `markup.rs:267`) — untouched; exposure unchanged.

## §4. Mechanism

### §4.1 `elidex-script-session` — `scripting.rs`

Add `ScriptingContext` (NEW; `Clone, Copy, Debug`) and `ScriptingContext::enabled_for_node` (NEW) as in D-3; re-export
from `lib.rs` beside `scripting_disabled_for_platform_object` (`crates/script/elidex-script-session/src/lib.rs:66`).
Doc comment: cites §8.1.3.4 clause (2), states the fail-safe "no node document ⇒ not enabled" and why it differs
from the event-handler gate's polarity (that gate's reading is IBP-sandbox N4's, not this PR's; §10 Q-2).
`scripting_disabled_for_platform_object` is not edited.

### §4.2 `elidex-dom-api` — `markup.rs`

- `EscapingMode` + `push_escaped_string` (NEW, private), D-1. `util.rs` loses `escape_html` / `escape_attr` and their
  two tests (moved as cells, §6).
- `RAW_TEXT_ELEMENTS` per D-2 (doc comment cites step 5.2 and says noscript is conditional, decided by
  `text_children_literal`).
- `text_children_literal` (NEW, private) per D-2/D-3/D-4; called at the two read sites (P2).
- `SerializeOptions.scripting` (NEW field) per D-3; public entry points per D-5; re-exports updated
  (`crates/dom/elidex-dom-api/src/element/mod.rs:31-32`, `crates/dom/elidex-dom-api/src/lib.rs:92-93`).
- `GetInnerHtml`'s `invoke` passes `&SerializeOptions::default()` (no engine context on the `SessionCore` path ⇒
  noscript escapes, fail-safe).

### §4.3 `elidex-js` — the four natives (marshalling only; Layering mandate)

Each native builds `ScriptingContext { bound_document: host.document(), sandbox_flags: host.sandbox_flags() }` from
`HostData` (the natives already hold it bound, e.g. `dom_inner_html.rs:140-143`) and passes it in
`SerializeOptions.scripting`; `getHTML` adds it to the options it already builds (`dom_inner_html.rs:312-315`). One
private helper per file is acceptable if two natives share a file (`dom_inner_html.rs` has three). No algorithm in
`vm/host/`: the decision is `enabled_for_node`, in `elidex-script-session`.

### §4.4 ECS-native / layering check

No side-store, no new component. The interim input (`bound_document`) is engine state already passed the same way to
the event-handler gate; its ECS replacement is owned by `#11-browsing-context-state-ecs-components` (D-3). Algorithms
live in `elidex-dom-api` (§13.3) and `elidex-script-session` (§8.1.3.4); `vm/host/` only marshals.

## §5. Populations

### §5.1 Escaping callers census

`git grep -n -w 'escape_attr\|escape_html' -- crates` → 11 hits (verified 2026-11-13): definitions `util.rs:154`,
`:168` (2); import `markup.rs:14` (1); production calls `markup.rs:373` (Text), `:421` (`is` value), `:436`
(attribute loop) (3); comment `markup.rs:411` (1); test message `tests_markup.rs:200` (1); unit asserts
`util.rs:225`, `:226`, `:231` (3). All 3 production calls become `push_escaped_string`; the comment and the test
message are reworded to the new name. Concept sweep for other HTML-escaping code: `git grep -n "'&' =>\|&amp;\|&quot;"
-- crates` minus parser/tests → only `util.rs` (and an unrelated JS-lexer `b'&'`), so there is no third
copy.

### §5.2 Serializer entry-point call sites (D-5)

`git grep -n -P '\b(serialize_inner_html|serialize_inner_html_with_options|serialize_outer_html|serialize_node_to_string)\(' -- crates | grep -v 'pub fn'`
→ 15 sites (verified 2026-11-13): `markup.rs` lines 142/151/187/191 (4, internal); `tests_markup.rs` lines
107/142/160/176/197/223/244 (7, pass `&SerializeOptions::default()`); `dom_inner_html.rs` lines 116/317/339 (3) and
`dom_parser.rs` line 404 (1) (VM, pass the context). ⚠ `git grep -E '\b…'` returns a silent 0 here; use `-P`.

### §5.3 Files touched (`wc -l` at `06bdfbbd`)

| File | Lines | Change |
|---|---|---|
| `crates/dom/elidex-dom-api/src/element/markup.rs` | 459 | D-1..D-5 (~+60) |
| `crates/dom/elidex-dom-api/src/util.rs` | 233 | −2 fns, −2 tests (~−30) |
| `crates/dom/elidex-dom-api/src/element/tests_markup.rs` | 246 | 7 call sites (verified 2026-11-13, §5.2) + new cells (~+150) |
| `crates/dom/elidex-dom-api/src/element/mod.rs`, `src/lib.rs` | — | re-export lists |
| `crates/script/elidex-script-session/src/scripting.rs` | 245 | `ScriptingContext` + tests (~+60) |
| `crates/script/elidex-script-session/src/lib.rs` | — | re-export |
| `crates/script/elidex-js/src/vm/host/dom_inner_html.rs` | 488 | 3 natives |
| `crates/script/elidex-js/src/vm/host/dom_parser.rs` | 407 | 1 native |
| `crates/script/elidex-js/src/vm/tests/tests_html_serialize_escape.rs` (NEW) | — | VM cells (~150) |

## §6. Test and coverage plan

Supported surface: §13.3 escaping and the step-5.2 Text branch, guarded by engine-independent unit cells in
`elidex-dom-api` / `elidex-script-session` and VM end-to-end cells; no WPT subset is added by this PR.

**Unit, `tests_markup.rs`** (fixtures: `EcsDom::create_document_root` = bound document; `create_document_node` =
DOMParser-like; `create_element_with_owner`; `create_element_ownerless_fixture`):
- U1 text mode, one cell per character: `&`, U+00A0, `<`, `>` escaped; `"` **not** escaped in text.
- U2 attribute mode, one cell per character: `&`, U+00A0, `<`, `>`, `"` escaped (attribute and `is` value sites).
- U3 order: data `"\u{a0}<&"` → exactly `&nbsp;&lt;&amp;` (step 1 runs first, so an entity produced by steps 2-5
  is never re-escaped to `&amp;nbsp;`; kills a sequential-replace form that runs `&` last).
- U4 each unconditional raw parent (script, style, xmp, iframe, noembed, noframes, plaintext) with data `<&>`
  emitted literally; `plaintext` is the new row.
- U5 SVG-namespace `style` (and `script`) parent: data escaped (D-4).
- U6 noscript, context `Some(bound)`, noscript owned by the bound document → literal.
- U7 noscript owned by a second document (DOMParser-like), context `Some(bound)` → escaped.
- U8 noscript inside a `<template>`'s contents (node document = inert template document), context `Some(bound)` →
  escaped.
- U9 ownerless noscript, context `Some(bound)` → escaped (fail-safe; kills the `is_some()` mutant).
- U10 noscript in the bound document, context `None` → escaped (fail-safe default).
- U11 noscript in the bound document, sandbox flags without `allow-scripts` → escaped (clause 1).
- Both entry points: U6/U7 run through `serialize_inner_html(noscript, ..)` (entry read site, `markup.rs:243`) and
  `serialize_outer_html(parent_of_noscript, ..)` (recursion read site, `:445`).

**Unit, `scripting.rs`**: `enabled_for_node` for bound / other document / ownerless / sandboxed.

**VM end-to-end, `tests_html_serialize_escape.rs` (NEW)** (bound VM; real parse):
- V1 page noscript stays raw: `div.innerHTML = '<noscript><p id=x></p></noscript>'` → `div.innerHTML` round-trips
  unchanged (bound document, scripting enabled).
- V2 DOMParser noscript escaped: `parseFromString('<noscript><p id=x></p></noscript>', 'text/html')` (bare form;
  raw text today per `dom_parser_bare_noscript_stays_rawtext_fragment_mode_boundary`) → its noscript's `innerHTML`
  is `&lt;p id=x&gt;&lt;/p&gt;`; `XMLSerializer` on the document likewise.
- V3 template noscript escaped: `t.innerHTML = '<noscript>a<b</noscript>'` on a page `<template>` → serialized
  noscript data escaped.
- V4 mXSS, noscript text: in a DOMParser document, `ns.textContent = '</noscript><img src=x onerror=f()>'`; then
  `d.innerHTML = ns.outerHTML` on a page `div` → `d.querySelector('img') === null`.
- V5 mXSS, attribute `<`/`>`: in a DOMParser document, a noscript containing `<p title="</noscript><img src=x
  onerror=f()>">` (built by `createElement` / `setAttribute`); `d.innerHTML = ns.outerHTML` → no `img`.
- V6 nbsp: `p.textContent = 'a b'` → `p.innerHTML === 'a&nbsp;b'`; `setAttribute('title', 'a b')` →
  `outerHTML` contains `title="a&nbsp;b"`.
- V7 plaintext: `<plaintext>` created by `createElement`, `textContent = '<b>'` → `innerHTML === '<b>'`.

**Existing tests whose expectations change** — census: `git grep -n "escape_attr\|escape_html\|noscript" -- crates`
plus `&quot;|&lt;|&gt;|&amp;|nbsp` in serializer tests (2026-11-13): `util.rs` `escape_html_chars` /
`escape_attr_chars` are deleted with their functions (their assertions are subsumed by U1/U2 with the same expected
strings). `tests_markup.rs:20` (`hello &lt;world&gt;`) and `:199` (`is="x&quot; onclick=&quot;evil"`) keep their
expectations. No VM test serializes a noscript or an attribute containing `<`, `>` or U+00A0 (the four noscript
tests in `tests_dom_parser.rs` assert parse shape only). **No expectation flips.**

**Mutants** (each must turn a named cell red): drop each of the 5 escape arms (U1/U2); unguard step 5 (U1 `"`);
run the `&` replacement after the others (U3); remove plaintext (U4/V7); drop the namespace conjunct (U5); noscript always raw
(U7/V4); noscript never raw (U6/V1); drop `node_document.is_some()` (U9); natives pass `None` (V1); `getHTML` omits
the context (a `getHTML` variant of V1).

## §7. 1000-line audit

All touched files are under 1000 lines at `06bdfbbd` (§5.3); the largest after the PR is `dom_inner_html.rs` (~500).
VM cells go to a NEW file rather than `tests_dom_parser.rs` (823) or `tests_inner_html_mixin.rs` (749), which
would approach the line.

## §8. Out of scope, and slots

- Slot items (a) void list, (b) step 1, (c) shadow-template attribute order + `shadowrootcustomelementregistry`,
  (f) attribute order (note: the spec says the order "is implementation-defined … must be stable", so (f) is a
  fidelity/allocation question, not an observable-deviation one — recorded for that facet, not decided here):
  stay in `#11-html-fragment-serializer-conformance`, Re-eval 2026-12-15.
- The ECS fact "Document has a browsing context": `#11-browsing-context-state-ecs-components` (D-3).
- DOMParser's bare `<noscript>` parsing as raw text (html5ever fragment fallback ignores the scripting flag):
  `#11-domparser-full-document-parse-fidelity`; V2 serializes that parse shape and needs no change if the parse is
  later fixed (data would then be elements, serialized normally).
- The event-handler gate's ownerless polarity (§10 Q-2): not touched.

## §9. Edges, hand-offs, definition of done

DoD:
1. D-1..D-5 implemented as §4; `escape_html` / `escape_attr` gone (`git grep -n -w 'escape_attr\|escape_html' --
   crates` → 0).
2. Every cell of §6 present and green; every listed mutant observed red at the final head (mutation runs only at the
   final head; ≤2 in parallel).
3. `cargo test -p elidex-dom-api -p elidex-script-session -p elidex-js --all-features`, then `mise run ci`.
4. Comments in `markup.rs` cite §13.3 step 5.2 and "Escaping a string" by name; `scripting.rs` cites §8.1.3.4 —
   numbers from §0.4's lookups only.

Landing (pmo, after merge):
- `#11-html-fragment-serializer-conformance`: (d) (as ratified, D-4) and (e) closed by this PR; (a) (b) (c) (f) open,
  Re-eval 2026-12-15.
- `#11-browsing-context-state-ecs-components`: append facet — "`ScriptingContext::bound_document` (NEW)
  (`elidex-script-session::scripting`) is the interim input of §8.1.3.4 clause (2) for the serializer (and the
  event-handler gate's `bound_document` argument); retire both onto the positive fact when it lands."

## §10. Open questions

- Q-1 (PM): ratify D-4 (HTML-namespace conjunct) as part of (d), or split to slot item (g).
- Q-2 (PM, report only): the event-handler compile gate treats a node with no node document as "scripting not
  disabled" (P4) — i.e. it compiles the handler. That is the fail-open direction for a security gate; this PR does
  not change it (IBP-sandbox N4 owns the gate's reading). Whether it needs a slot is PM's call.
- Q-3 (implementer, scout-able): is the `"innerHTML.get"` `DomApiHandler` (`crates/dom/elidex-dom-api/src/registry.rs:153`)
  reachable from any production dispatch? If yes, its `None` context escapes page noscript text (fail-safe, fidelity
  loss); if not, it is dead code for a separate cleanup — not decided here.
