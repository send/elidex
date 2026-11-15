# T3-ab `EH-attrs-at-creation-T3-ab` — a name is processed only if it is an event handler content attribute of its element; `HTMLBodyElement` is an HTML `body`'s interface (per-PR plan-memo)

Nested PR **T3-ab** ("applicability and `body`'s interface") of the standalone program `EH-attrs-at-creation`
(umbrella `docs/plans/2026-10-event-handler-attrs-at-creation.md`, "the umbrella"; its in-place edits for this PR are
carried by the T3 memo's *Rev 8.<t>* entry, since the three memos land in one plan PR). It is a prerequisite of **T3**
(`docs/plans/2026-10-eh-attrs-t3-target-determination.md`, "the T3 memo", rev 6), carved out of T3 rev 5 by **PM
decision 741 (3)** after plan-review R2 on T3 rev 5 (`38451533`; finding T1). It carries two decisions, with their ids
kept from the T3 memo rev 5: **D-T8** (which names are event handler content attributes of an element) and **D-T11**
(the VM gives `HTMLBodyElement.prototype` to an HTML `body` only). The label `T3-ab` is this memo's proposal; the PM
assigns the final one. Rev 1 (2026-11-15).

**Owner: Layout lane (L2)**, plan and implementation (PM decision 741 (3)). No surface of another lane is changed.

**Placement and order (PM decision 741 (3); measured in §8.1).** Row **T3-ab** of the umbrella's §1.2 (added by this
plan PR). One edge: **x15** T3-ab → T3 (umbrella §1.3). **No edge with L3's PR (is), PR (ig), PR (io), L3's tree-model
program or T3-pre**: T3-ab uses no creation event, no dispatcher window, no walker and no parent or child link (§8.1,
H-A9). Its prerequisites are all on main (slice a, N1, C0, C1; §8.1), so its implementation branch can be cut from
main today, and it lands in parallel with every unit between C1 and T3. The user's order (PM decision 153: N1 → C0 →
C1 → T3 → slice b) is kept: T3-ab is a part of T3's scope landing earlier, between C1 and T3. PM decision 168, which
placed D-T8 in T3, is revised by PM decision 741 (3).

**Base.** main when T3-ab's implementation branch is cut. Every code coordinate below is a `cee74989` coordinate (#626,
the T3 memo's frame) and is re-run at the base (§9 item 1). main is past `cee74989` (`f51cb55b` on 2026-11-15); of the
files this memo cites, one differs there, `crates/script/elidex-js/src/vm/host_data/mod.rs` (3 lines deleted; not a
cited line), and none moved (H-A10).

**Scope.**
- **D-T8.** One predicate, `is_event_handler_content_attribute_of(dom, element, name)` (NEW), realises the local-name
  half of the attribute change steps' step 1 (WHATWG HTML §8.1.8.1 Event handlers: "localName is not the name of an
  event handler content attribute on element, then return"). Every reader of "the handler content attributes of this
  element" applies it: the processing function's snapshot, the D-8 restriction's count, and the consumer's
  AttributeChange arm. This closes facets (1) and (3) of `#11-event-handler-content-attribute-applicability` and T3
  rev 5's residual R-D2.
- **D-T11.** `HTMLBodyElement.prototype` and its receiver check require the HTML namespace as well as the local name
  `body`. This closes T3 rev 5's residual R-D1.
- Cells T-19, T-21, U-T8 (ids kept from the T3 memo rev 5) and new T-A1 to T-A4, U-A1 to U-A3 (§6); three existing
  tests rewritten (§5.2).

**Not in scope.** Target determination, every facet (T3: a `frameset`, the reflecting six and an SVG `body` keep
HEAD's target here, §1.2); the attribute change steps' step 1 namespace half (applicability facet (2), slice b); the
HTML-namespace test of step 1's class in `is_body_or_frameset` (T3 memo D-T5); the interfaces of foreign elements
other than `body` (§8.2 item 1); `document.createElementNS` (`#11-vm-create-element-ns-missing`, L3); the
`HTMLFrameSetElement` interface (`#11-htmlframesetelement-interface`, compat, PM decision 130).

**Changelog.** *Rev 1* (2026-11-15): new memo, PM decision 741 (3). Moved from the T3 memo rev 5 without restating
there: D-T8 and D-T11, H-T18, H-T19, H-T22, H-T23 (here H-A4, H-A8, H-A2), P-T14 (here P-A4), rows FN-1b, FN-3, FN-4,
FN-5 and OR-1's local-name half, cells T-19, T-21, U-T8 and their mutants, the §3 rows for SVG2 §15.8, MathML Core
§2.1.4 and DOM §4.5, the slot texts for the applicability slot and for foreign-element interfaces, and §9 item 4's
namespace grep. New against rev 5's text: the D-8 restriction's count as a third reader (D-T8, §4.2; rev 5 named two);
`HandlerScope::DocumentElement`'s place in the predicate (P-A1); the reach of each changed outcome (H-A6, H-A7: no
script can create a foreign `body` at the base, so cells T-19 and T-21 build it in the harness, P-A2); no element
outside the three namespaces is representable (P-A3); the three existing tests that change (§5.2, measured; rev 5
deferred the list to implementation); cells T-A1 to T-A4, U-A1 to U-A3; the direct-`EcsDom`-call count of DoD item 4
(R2 Axis 1 MIN).

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every code `file:line` is a **`cee74989`** coordinate, measured with `git grep -n -E '<pattern>' cee74989 -- <paths>`,
`git show cee74989:<path> | sed -n <range>p` or `/usr/bin/grep -n` (never the default ugrep; never `\b` under `git
grep -E`, which matches nothing). Paths are abbreviated after first use: `session/…` =
`crates/script/elidex-script-session/src/…`, `elidex-js/…` = `crates/script/elidex-js/src/…`, `ecs/…` =
`crates/core/elidex-ecs/src/…`. Spec text is `.claude/tools/webref body` / `idl` output of 2026-11-15; every `§X.Y =
title` pair is `webref heading` / `dfn` output (§3's footnote). Symbols written as `Type` + method
(`HostData::dom_shared`, `HostData::tag_matches_ascii_case`, `EcsDom::is_html_namespace`, `EcsDom::namespace_of`,
`EcsDom::create_element_ns`, `test_support::elem`) name existing methods inside `impl` / `mod` blocks, which
preflight's literal symbol grep does not find (its soft warnings).

**Name classes** (rows of `EVENT_HANDLER_ATTRS`, `session/event_handler_consumer.rs:79`, by their `HandlerScope`,
`:63`; counts by H-A1): **G** = `Global` (82) and `DocumentElement` (3), 85 names, the ones HTML §8.1.8.2 says "must
be supported by all HTML elements" (the six Window-reflecting names are among `Global`'s); **W** = `Window` (18), the
WindowEventHandlers names; **D** = `DocumentOnly` (2), `onreadystatechange` and `onvisibilitychange`. **Element
classes** are by `Namespace` (`ecs/components.rs:62`: `Html`, `Svg`, `MathMl`) and, for HTML, by whether the local
name is `body` or `frameset`.

### §0.2 Premises re-measured

| ID | Premise | Command | Result at `cee74989` |
|---|---|---|---|
| H-A1 | The name table and its scopes | `for s in Global DocumentElement Window DocumentOnly; do git show cee74989:crates/script/elidex-script-session/src/event_handler_consumer.rs \| /usr/bin/grep -c "HandlerScope::$s),"; done` | 82, 3, 18, 2: 105 rows. `DocumentElement` is `oncopy`, `oncut`, `onpaste` (`:164-166`); `DocumentOnly` is `onreadystatechange`, `onvisibilitychange` (`:187-188`) |
| H-A2 | What each spec says about applicability | `.claude/tools/webref body html 'event-handlers-on-elements,-document-objects,-and-window-objects'`; `webref idl html GlobalEventHandlers`; `webref idl html DocumentAndElementEventHandlers`; `webref body SVG2 EventAttributes`; `webref body mathml-core attributes-common-to-html-and-mathml-elements`; `webref idl mathml-core MathMLElement`; `webref body html event-handler-attributes` | HTML §8.1.8.2, four lists: (i) "must be supported by all HTML elements, as both event handler content attributes and event handler IDL attributes" (`oncopy`, `oncut`, `onpaste` are in it, and in `interface mixin GlobalEventHandlers`; no `DocumentAndElementEventHandlers` IDL block exists); (ii) the six of the *Window-reflecting body element event handler set*: "must be supported by all HTML elements other than body and frameset elements … and with corresponding event handler content attributes and event handler IDL attributes exposed on all body and frameset elements"; (iii) the WindowEventHandlers names: "supported by Window objects … with corresponding event handler content attributes and event handler IDL attributes exposed on all body and frameset elements"; (iv) `onreadystatechange`, `onvisibilitychange`: "must be supported on Document objects as event handler IDL attributes" (no content attribute on any element). SVG2 §15.8: "For every event type that the user agent supports, SVG supports that as an event attribute … The event attributes are available on all SVG elements". MathML Core §2.1.4: "All event handler content attributes noted by HTML as being supported by all HTMLElements are supported by all MathML elements as well, as defined in the MathMLElement IDL"; `MathMLElement includes GlobalEventHandlers`. HTML §8.1.8.1 attribute change steps step 1: "If namespace is not null, or localName is not the name of an event handler content attribute on element, then return." |
| H-A3 | The readers of "the handler content attributes of this element" | `git grep -n 'event_handler_attr_lookup(' cee74989 -- crates` | the definition `session/event_handler_processing/mod.rs:237` and four calls: the processing function's snapshot (`mod.rs:97`, in `process_handler_content_attributes`, `:76`), the D-8 restriction's count (`mod.rs:206`, in `is_only_deliverable_ordinary_name`, `:196`), the AttributeChange arm (`session/event_handler_consumer.rs:405`, in `handle_attribute_change`, `:397`) and `is_window_delegating_pair` (`mod.rs:219`), which reads the name's scope only and asks nothing about the element's attributes. None of the first three asks whether the name is one of the element's: every row of the table installs on every element |
| H-A4 | (T3 rev 5's H-T18) Foreign elements take HTML prototypes | `git show cee74989:crates/script/elidex-js/src/vm/host_data/engine_feature/entity_kind.rs \| sed -n 124,198p`; `git show cee74989:crates/script/elidex-js/src/vm/host/elements.rs \| sed -n 291,296p`; `git grep -n -e 'require_body_receiver(' -e 'tag_matches_ascii_case(entity, "body")' cee74989 -- crates/script/elidex-js/src` | `prototype_kind_for` (`entity_kind.rs:150`) returns `PrototypeKind::Element` for every `TagType` entity; `HostData::tag_matches_ascii_case` (`:124-133`) reads no namespace. The `body` arm of `tag_specific_t2b_prototype` (`elidex-js/vm/host/elements.rs:294-296`) and `require_body_receiver` (`elidex-js/vm/host/event_handler_attrs.rs:418-432`, the test at `:427`; called by `native_body_weh_get` `:445` and `native_body_weh_set` `:468`) are its two `body` readers. So an SVG or MathML `body` gets `HTMLBodyElement.prototype` and passes its brand check |
| H-A5 | The namespace read that exists | `git show cee74989:crates/core/elidex-ecs/src/dom/classify.rs \| sed -n 67,138p`; `git show cee74989:crates/script/elidex-js/src/vm/host_data/engine_feature/bind.rs \| sed -n 230,246p`; `git grep -n 'dom_shared()' cee74989 -- crates/script/elidex-js/src/vm/host` | `EcsDom::namespace_of` (`ecs/dom/classify.rs:76`: the `Namespace` component, absence = `Html`); `EcsDom::is_html_namespace` (`:136`: an element whose namespace is `Html`), the gate `EcsDom::is_base_element` uses (`:39-46`). `HostData::dom_shared` (`bind.rs:242`) is the existing `&HostData → &EcsDom` path, with 10 callers in `vm/host/` (custom elements, slot and table prototypes) |
| H-A6 | How a foreign element comes to exist | `git grep -n -i 'createElementNS' cee74989 -- crates`; `git grep -n 'create_element_ns(' cee74989 -- crates ':!*tests*'`; `git show cee74989:crates/script/elidex-js/src/vm/host/dom_parser.rs \| sed -n 45,56p` and `sed -n 252,262p` | No `createElementNS` in the VM (two comment hits; slot `#11-vm-create-element-ns-missing`, owner L3). Production callers of `EcsDom::create_element_ns` (`ecs/dom/mod.rs:435`): the strict parser's foreign-content path (`crates/dom/elidex-html-parser-strict/src/tree_builder/insert.rs:116`) and the tolerant parser's tree conversion (`crates/dom/elidex-html-parser/src/convert.rs:258`); `crates/dom/elidex-form/src/init.rs:264`, `:278` are tests. DOMParser HTML-parses every accepted MIME type (`#11-domparser-xml-real-parsing`). So a foreign element is created by the HTML parser's foreign content only, or by a clone of one |
| H-A7 | The HTML parser cannot create a foreign `body` | `.claude/tools/webref body html parsing-main-inforeign`; `git show cee74989:crates/dom/elidex-html-parser-strict/src/tree_builder/modes/foreign.rs \| sed -n 162,213p` | HTML §13.2.6.5 The rules for parsing tokens in foreign content: a start tag named `body` (one of the listed names, "b", "big", "blockquote", "body", …) pops out of foreign content and is reprocessed as HTML content; `frameset` is not in the list. The strict parser has the same list (`BREAKOUT_TAGS`, `foreign.rs:165`, read at `:218`; strict rejects, and the tolerant backend, html5ever, recovers as the spec says). With H-A6: **no script can create an SVG or MathML `body` at the base**; an SVG or MathML `frameset`, `mi`, `rect` … is created by `innerHTML` and the document parse |
| H-A8 | (T3 rev 5's H-T19) The AttributeChange event carries no namespace | `git show cee74989:crates/core/elidex-ecs/src/dom/mutation_event.rs \| sed -n 366,371p` | `AttributeChange { node, name, old_value, new_value }`: the namespace half of step 1 cannot be tested at the consumer (facet (2), slice b) |
| H-A9 | (Q-10 for this slice) T3-ab's planned code reads no parent and no child link | the reads of §4.2: `TagType`, `Namespace` (through `namespace_of` / `is_html_namespace`), `Attributes`, `HandlerAttrsProcessed`; `for f in <the four source files of §5.3>; do git show cee74989:$f \| /usr/bin/grep -c -E 'get_parent\|children_iter\|first_child\|last_child\|next_sibling\|prev_sibling\|traverse_descendants\|TreeRelation'; done` | the predicate and both `body` tests read components of the one entity they are given. Of the four files, three have 0 link reads; `event_handler_consumer.rs` has `handle_insert`'s `traverse_descendants` (`:446`), which T3-ab does not edit. So T3-ab uses neither `tree_parent` nor `shadow_including_parent` of L3's tree-model program and no child iterator |
| H-A10 | The cited files on main | `git diff --stat cee74989 f51cb55b -- crates/script/elidex-script-session/src crates/script/elidex-js/src/vm/host/elements.rs crates/script/elidex-js/src/vm/host/event_handler_attrs.rs crates/script/elidex-js/src/vm/host_data crates/core/elidex-ecs/src/dom/classify.rs crates/core/elidex-ecs/src/components.rs` | one file, `vm/host_data/mod.rs`, 3 deletions (verified 2026-11-15) |
| H-A11 | The direct `EcsDom` method calls in the two VM files | `for f in crates/script/elidex-js/src/vm/host/event_handler_attrs.rs crates/script/elidex-js/src/vm/host/elements.rs; do git show cee74989:$f \| /usr/bin/grep -c -E 'dom_shared\(\)\.\|\.dom\(\)\.'; done` | 0 and 0 (`event_handler_attrs.rs` passes `host.dom()` as an argument to two session functions, `:542`, `:610`; it calls no `EcsDom` method itself). Same at `f51cb55b` |

### §0.3 Premise errors (reported)

- **P-A1 (T3 rev 5's D-T8).** D-T8 named three scopes ("the GlobalEventHandlers names, plus the WindowEventHandlers
  names on `body` / `frameset` only, and no Document-only name"); `HandlerScope` has four (H-A1). The fourth,
  `DocumentElement` (`oncopy`, `oncut`, `onpaste`), is in HTML's "all HTML elements" list and in `GlobalEventHandlers`
  (H-A2), so it is in class G. Its enum doc ("`DocumentAndElementEventHandlers` — Document + Element") names a mixin
  the HTML IDL no longer has; the doc and what the VM installs for it are not T3-ab's (§10 Q-A4, a scout question).
- **P-A2 (T3 rev 5's cells T-9, T-9b, T-19, T-21).** Their scripts call `document.createElementNS`, which the VM does
  not have (H-A6), and no other script path creates a foreign `body` (H-A7). The cells cannot run as written. T-19
  and T-21 (here) take their elements from the harness (§6.1); T-9 and T-9b stay T3's and take the same harness
  elements (T3 memo §6.1).
- **P-A3 (T3 rev 5's rows FN-1b, FN-4, FN-5 and U-T8's "null-namespace `body`").** "Outside the HTML, SVG and MathML
  namespaces" has no elidex representation: `Namespace` has three variants and absence means `Html` (H-A5). The spec's
  "none" for such an element (H-A2: the three specs define event handler content attributes for their own namespaces
  only) is not a branch T3-ab can write or test. The predicate's `match` on `Namespace` has no wildcard, so a fourth
  variant does not compile until its arm is decided (§4.2).
- **P-A4 (T3 rev 5's P-T14, kept).** T3 rev 3's FN-1 gave "element" for such an element; the spec gives "nothing".
  With P-A3 the row has no elidex case; it is recorded in the slot text (§8.3 item 1).
- **P-A5 (T3 rev 5's residuals R-D1 and R-D2, reach).** Both are about a foreign `body`, which no script can create
  (H-A7). They are defects of fixture-built elements today and become reachable when `createElementNS` is wired
  (`#11-vm-create-element-ns-missing`). The script-reachable part of D-T8 is the rest of its table: class W and D
  names on HTML elements other than `body` / `frameset`, class D names on `body` / `frameset`, class W and D names
  on MathML elements other than `body` (§2.1).

### §0.4 Decisions (this PR)

- **D-T8 — applicability: the local-name half of the attribute change steps' step 1 (PM decisions 168, 741 (3)).**
  `is_event_handler_content_attribute_of(dom, element, name) -> bool` (NEW), session-private, in
  `session/event_handler_processing/mod.rs`, is the one statement of which rows of `EVENT_HANDLER_ATTRS` are event
  handler content attributes of `element`:

  | element | G (85) | W (18) | D (2) | ground |
  |---|---|---|---|---|
  | HTML `body`, `frameset` | yes | yes | no | HTML §8.1.8.2 lists (i), (ii), (iii), (iv) |
  | other HTML | yes | no | no | HTML §8.1.8.2 lists (i), (ii), (iii), (iv) |
  | SVG | yes | yes | yes | SVG2 §15.8 (every supported event type, on all SVG elements) |
  | MathML | yes | no | no | MathML Core §2.1.4 (the names HTML supports on all HTML elements) |

  **Every reader of "the handler content attributes of this element" applies it** (H-A3), so a name that is not one
  has no effect anywhere: (a) the processing function's snapshot keeps only the element's handler content attributes,
  so such a name is not delivered, does not count toward the mark and never reaches the policy filter; (b) the D-8
  restriction counts only them, so such a name does not keep an element from being processed at its first handler
  write; (c) the AttributeChange arm returns for such a name before the policy filter, with no set and no clear. T3
  rev 5 named (a) and (c); (b) is the same question asked a third time, and leaving it out would let a name with no
  handler meaning change when an element is processed. `is_window_delegating_pair` is not a reader of this kind
  (H-A3) and is not changed.
  The HTML arm's class test is the existing `is_body_or_frameset`; inside the HTML arm its tag-only test is exact, so
  D-T8 does not depend on T3's D-T5, and D-T5's later namespace clause changes nothing here. The IDL accessors are not
  affected: their surface is the prototype's (D-T11 for `body`). Facet (2), a non-null attribute namespace, stays
  with slice b (H-A8). Every outcome the predicate changes is toward the spec (§2.1).
  *Rejected:* a test at the two event-facing sites only (rev 5's text): three readers, two answers (one issue, one
  way). A lookup that takes the element and replaces `event_handler_attr_lookup`: `is_window_delegating_pair` and the
  VM's install loop ask about the name alone, so the name-only lookup stays, and one function answering both
  questions would hide which is asked.
- **D-T11 — `HTMLBodyElement` is chosen for an HTML `body` only (PM decisions 153, 741 (3); R-D1).** The `body` arm of
  `tag_specific_t2b_prototype` (`elements.rs:294`) and `require_body_receiver` (`event_handler_attrs.rs:427`) also
  require the HTML namespace, each read as `dom_shared().is_html_namespace(entity)` on the `HostData` it already holds
  (H-A5; a component read for interface selection and for a brand check: marshalling). No `HostData` method is added
  and no `host_data/` file is edited. A foreign `body` then takes `HTMLElement.prototype`, the fallback every foreign
  element without an HTML namesake already takes (H-A4). Every member it loses is one the spec does not give it:
  `HTMLBodyElement`'s own attributes and its 18 WindowEventHandlers accessors are on neither `SVGElement` nor
  `MathMLElement` (`webref idl SVG2 SVGElement`; `MathMLElement : Element`, H-A2), and `HTMLElement.prototype` keeps
  the GlobalEventHandlers accessors both foreign interfaces include. So the change is toward the spec at every
  observation (row FN-3; cell T-19). The two sites stay two tests of one fact pair (local name, namespace): one
  chooses a prototype, the other checks a receiver, and DoD item 4 pairs each with its namespace read.
  *Why not every arm (the general fix):* for a tag that is also an SVG or MathML element name (`a`, `script`, `style`,
  `title`, …) the fallback `HTMLElement.prototype` would remove members the foreign interface has (e.g. `type` on
  `SVGScriptElement`), which the HTML interface supplies today: worse than the base. The general fix needs the SVG and
  MathML interface families; §8.2 item 1 is its defect text. *Not chosen:* every arm without a foreign namesake
  (`div`, `iframe`, …): toward the spec too, but a change of every prototype arm is not this slice's (§8.2 item 1
  lists it as the fix's first step).

## §1. Shape and verdict

| Axis | Own invariant |
|---|---|
| B1 name class | G, W, D by `HandlerScope` (§0.1) |
| B2 element class | `Namespace` × (`body` / `frameset` or not), immutable per element |
| B3 readers of an element's handler content attributes | three, one predicate (D-T8 (a), (b), (c)) |
| B4 `body`'s interface | prototype choice and receiver check, HTML namespace (D-T11) |

Coupled from outside, all on the base and none changed: B5 the once rule and its mark (umbrella K-1, I1); B6 the
policy filter (I6); B7 the D-8 restriction (umbrella D-8); B8 X-W's classifier `is_window_delegating_pair`, tag-only
until T3's D-T5; B9 HEAD's target rule `resolve_handler_target`, until T3; B10 the wrapper's prototype is chosen once,
at first allocation (C1-pre D-3).

### §1.1 Outcome changes at T3-ab (toward the spec)

Script-reachable (H-A7):
- A class W or class D name on an HTML element other than `body` / `frameset` installs nothing:
  `div.setAttribute('onhashchange', X)`, `<p onreadystatechange=X>` (base: a handler on the element, run by a dispatch
  at it). Rows AP-3, AP-4; cell T-A1.
- A class D name on an HTML `body` / `frameset` installs nothing (base: a handler on the element). Row AP-2.
- A class W or class D name on a MathML element other than `body` installs nothing: `<math><mi onhashchange=X>`, a
  MathML `frameset` (base: a handler on the element). Rows AP-6, FN-5; cell T-A2.
- An element whose other handler-named attributes are not its event handler content attributes is processed at its
  first handler write when one ordinary attribute remains (base: not processed there, so its first insertion
  overwrites the write). Row AP-8; cells T-A4, U-A1.

Harness-built elements only, until `#11-vm-create-element-ns-missing` is wired (P-A5):
- A class W name on a MathML `body` installs nothing (base: on the Window, re-raw'd at every insertion). Row FN-4;
  cell T-21.
- An SVG or MathML `body` does not get `HTMLBodyElement.prototype` (base: it does, with the 18 WindowEventHandlers
  accessors acting on the Window). Row FN-3; cell T-19.

### §1.2 Is T3-ab no worse than its base, and is T3 no worse landing after it?

Criterion: the umbrella's K-0 with T3-ab's base as "HEAD": at every observable step T3-ab's outcome equals the
base's, or the spec's where the base's does not. §2.1 enumerates element class × name class × reader. **Rows worse
than the base: 0.** Three rows stay base-equal and off the spec, each T3's and each unchanged by T3-ab: an HTML
`frameset`'s and `body`'s Window-delegating pairs that HEAD targets at the element (T3 memo EW rows), an SVG `body`'s
class W content attribute, delivered to the Window by the tag-only target rule (T3 memo FN-2), and the foreign-`body`
half of X-W's classifier (B8). For an SVG `body` × class W, T3-ab changes the IDL side only (FN-3, to the spec); its
content attribute still reaches the Window, as on the base, until T3: one side on the spec and one base-equal is not
worse than both off it, and no script can build the element (H-A7).

The other direction (T3 on a base that has T3-ab): T3 keeps its own no-worse analysis (T3 memo §1.2, §2.1); the rows
T3-ab closes are equal on T3's base and after T3 (T3 memo §2.1, marked ‡). The two can land in this order only: T3
before T3-ab would leave R-D open at T3's landing, which PM decision 153's strict reading does not allow, and T3's
namespaced step 1 without D-T8 and D-T11 would put rows FN-4 and FN-3 on neither the base nor the spec (T3 memo §1.3,
seam S1).

### §1.3 Edge density

*Verdict:* plan-review is mandatory (PM decision 741 (3): R1, five axes), whichever way the count falls. Four own
axes meet in one table (B1 × B2) read at three sites (B3) plus a two-site VM test (B4), and six external ones bound
it (B5 to B10). The table is the spec's, fully enumerated by U-T8; no canonical algorithm is missing.

Breadth (all §3 rows): `.claude/tools/webref coverage-map html 8.1.8.1 html 8.1.8.2 SVG2 15.8 mathml-core 2.1.4
mathml-core 2.2.3 dom 4.5` → K = 4 specs, M = 6 entries, "K≥4 or M≥20 → 分割推奨" (verified 2026-11-15). K = 4
comes from the four specs one predicate branches over (HTML, SVG2, MathML Core) plus DOM for the interface.

**The split, tested in both directions.** The only seam is D-T8 | D-T11: they share no code (the session's predicate;
two VM tests) and no cell.
- *D-T11 first, D-T8 later:* no row worse than the base (FN-3 moves to the spec; nothing else changes).
- *D-T8 first, D-T11 later:* no row worse than the base (§2.1's AP and FN rows move to the spec; FN-3 stays base-equal).
- So a further split is possible in either order, and neither half needs the other. It would give K = 3, M = 4 (D-T8)
  and K = 4, M = 3 (D-T11: DOM, HTML, and the two foreign interfaces' IDL), so it does not lower K below the
  threshold for D-T11.
- What keeping them together costs and buys: one plan-review and one PR instead of two, for a D-T11 that is two
  conjuncts and one cell and whose reach is harness-only (P-A5); against that, a reviewer reads two unrelated
  mechanisms in one PR. PM decision 741 (3) names one slice; this memo follows it and does not argue that the split
  is impossible. Whether to split further is the PM's (§10 Q-A2).

**A split inside D-T8** (two readers now, the D-8 count later; or HTML now, MathML later) would leave two answers to
one question on main between the parts, in either order (D-T8 *Rejected*).

T3-ab is a narrowly-scoped per-PR slice under the approved EH-attrs umbrella (§1.2 row T3-ab), the rule's base case.
External couplings: slice a, N1, C0, C1 (landed); T3 (x15, after). Decisions: PM 153, 168, 741 (3).

## §2. Coupled invariants

- **B1×B2** — the table of D-T8, one `match` on (`Namespace`, `HandlerScope`) with no wildcard on either.
- **B2×B3** — the three readers call the one predicate with the same (element, name); none keeps a private copy of a
  row of the table.
- **B3×B5** — an element is marked iff it carries at least one ordinary event handler content attribute *of its own*
  (umbrella K-1's "handler content attribute", now the spec's term). An element whose only handler-named attributes
  are not its own is not marked and has nothing delivered (row AP-7; cell U-A2). The mark is read only by the
  processing function and the D-8 entry, so an unmarked element with nothing to deliver is not observable.
- **B3×B6** — applicability is tested before the policy filter at all three readers: a name that is not the
  element's never reaches it. A policy-excluded ordinary pair still marks (umbrella I1×I6); since the one
  policy-excluded name is `onstorage` (class W), such a pair now exists only on an SVG element (§5.2, cell U-A3).
- **B3×B7** — D-8 (a)'s "the element's only deliverable ordinary handler content attribute" counts applicable names
  only (row AP-8).
- **B2×B8** — `is_window_delegating_pair` stays tag-only: a MathML `body` × class W passes it, and no reader reaches
  it with that pair, because the predicate returns first at (a), (b) and (c). The IDL setters reach
  `process_before_handler_write` with the receiver and an accessor's name: for a foreign `body` after D-T11 that is a
  class G name on `HTMLElement.prototype`'s pair; with a reflecting name the tag-only classifier returns early there,
  as on the base (T3's D-T5 ends it).
- **B3×B9** — the predicate returns before target determination at (a) and (c): step 1 before step 2, the spec's
  order. T3 replaces the target rule after it and does not move it (umbrella §4.1).
- **B4×B10** — the namespace and the local name never change after creation, and the prototype is chosen once: an
  entity has `HTMLBodyElement.prototype` iff it passes `require_body_receiver`.
- **B4×B1** — after D-T11 a foreign `body` has no class W accessor; its class G accessors are `HTMLElement.prototype`'s
  normal pair, as for every other foreign element.
- **B2×B4** — D-T8's HTML arm and D-T11 read the same namespace fact through `EcsDom::namespace_of`; they are
  different questions (which names are content attributes; which interface) and stay separate tests.

Pairs not listed do not intersect: B5 to B10 among themselves are the base's.

### §2.1 Enumeration: element class × name class (Base / T3-ab / spec)

"Installs" = the content attribute, at any timing (bind, insertion, first write, `setAttribute` / `removeAttribute`),
writes or clears a handler; the target is HEAD's (`resolve_handler_target`), unchanged here. Verdict as in the T3
memo §2.1: **both**, **spec** (T3-ab = spec ≠ base), **base** (T3-ab = base ≠ spec). *Reach*: **S** a script can
build it; **F** harness-built only (H-A7).

| Row | Element | Names | Reach | Base | T3-ab | Spec | Verdict | Cells |
|---|---|---|---|---|---|---|---|---|
| AP-1 | HTML `body`, `frameset` | G, W | S | installs | installs | installs (the target is T3's) | both (applicability) | U-T8 |
| AP-2 | HTML `body`, `frameset` | D | S | installs on the element | nothing | nothing | spec | U-T8 |
| AP-3 | other HTML | W | S | installs on the element | nothing | nothing | spec | T-A1, U-T8 |
| AP-4 | other HTML | D | S | installs on the element | nothing | nothing | spec | T-A1, U-T8 |
| AP-5 | SVG (any local name) | G, W, D | S (`body`: F) | installs (an SVG `body` × W: on the Window, T3 memo FN-2) | installs, same target | installs on the element | both; base for an SVG `body` × W's target (T3) | T-A3, U-T8 |
| AP-6 | MathML other than `body`, `frameset` | W, D | S | installs on the element | nothing | nothing | spec | T-A2, U-T8 |
| FN-5 | MathML `frameset` | W, D | S | installs on the element | nothing | nothing | spec | U-T8 |
| FN-4 | MathML `body` | W | F | installs on the Window (tag-only target), re-raw'd at every insertion | nothing | nothing | spec (R-D2) | T-21, U-T8 |
| AP-9 | MathML `body` | D | F | installs on the element | nothing | nothing | spec | U-T8 |
| AP-10 | MathML (any local name) | G | S (`body`: F) | installs on the element | installs | installs | both | T-A2, U-T8 |
| AP-7 | any element whose handler-named attributes are all rows AP-2 to AP-4, AP-6, FN-4, FN-5 or AP-9 | — (the mark) | S | marked at its first processing | not marked | no such state | both (not observable, B3×B5) | U-A2 |
| AP-8 | an unmarked element with exactly one deliverable ordinary attribute of its own and at least one handler-named attribute that is not its own; a handler write before its first insertion | — (D-8) | S | not processed at the write; the first insertion overwrites the write | processed at the write; the write stays | the write stays | spec | T-A4, U-A1 |
| FN-3 | SVG or MathML `body` | W, IDL (`el.on<T>`, get and set); `instanceof HTMLBodyElement`; the accessor called on it | F | the body pair: reads and writes the Window; `true`; succeeds | an ordinary property; `false`; `TypeError` | an ordinary property (no such attribute on `SVGElement` / `MathMLElement`); `false`; `TypeError` | spec (R-D1) | T-19 |
| OR-1a | every other (element, name) pair | G on HTML; `removeAttribute` | S | as its row above, clearing instead of setting | same | same | as its row | U-T8 |

FN-1b (an element outside the three namespaces) has no elidex case (P-A3). The attribute-namespace half of step 1 is
base-equal on every row (facet (2), slice b).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.8.1 Event handlers | attribute change steps step 1, local-name half | localName not an event handler content attribute of element → return; the namespace half not touched (slice b) | `is_event_handler_content_attribute_of` (NEW) in `crates/script/elidex-script-session/src/event_handler_processing/mod.rs`, read at `:97`, `:206` and at `crates/script/elidex-script-session/src/event_handler_consumer.rs:405` | ✗ | yes |
| WHATWG HTML §8.1.8.2 Event handlers on elements, Document objects, and Window objects | the four lists of event handlers | all HTML elements (G); body and frameset only (W); Document objects only, no content attribute (D) | the `Html` arm of the predicate, over `HandlerScope` (`crates/script/elidex-script-session/src/event_handler_consumer.rs:63`) | ✓ | yes |
| SVG2 §15.8 Event attributes | every supported event type is an event attribute of every SVG element | SVG element → every row | the `Svg` arm of the predicate | ✓ | yes |
| MathML Core §2.1.4 Attributes common to HTML and MathML elements | the event handler content attributes HTML supports on all HTML elements | MathML element → G only | the `MathMl` arm of the predicate | ✓ | yes |
| MathML Core §2.2.3 DOM and JavaScript | `MathMLElement : Element`, `includes GlobalEventHandlers` | no WindowEventHandlers IDL attribute, no `HTMLBodyElement` member on a MathML element | the two `body` tests of D-T11 (below) | ✗ | yes |
| WHATWG DOM §4.5 Interface Document | *element interface* ("Element, unless stated otherwise") | HTML's `HTMLBodyElement` is the interface of `body` in the HTML namespace only | `crates/script/elidex-js/src/vm/host/elements.rs:294`; `require_body_receiver` `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs:418` | ✗ | yes |

Numbers and titles (2026-11-15): `webref heading html 8.1.8.1` (Event handlers, `#event-handler-attributes`),
`8.1.8.2` (Event handlers on elements, Document objects, and Window objects), `8.1.8.2.1` (IDL definitions),
`13.2.6.5` (The rules for parsing tokens in foreign content, `#parsing-main-inforeign`); `webref heading dom 4.5`
(Interface Document); `webref heading SVG2 15.8` (Event attributes, `#EventAttributes`); `webref heading mathml-core
2.1.4` (Attributes common to HTML and MathML elements), `2.2.3` (DOM and JavaScript); `webref dfn dom 'element
interface'` → §4.5 `#concept-element-interface`. Step and list texts: `webref body html event-handler-attributes`
(attribute change steps step 1), `event-handlers-on-elements,-document-objects,-and-window-objects` (the four lists),
`parsing-main-inforeign` (the breakout start tags); `webref body SVG2 EventAttributes`; `webref body mathml-core
attributes-common-to-html-and-mathml-elements`; `webref body dom concept-element-interface`; `webref idl html
GlobalEventHandlers` (with `oncopy`, `oncut`, `onpaste`), `webref idl html DocumentAndElementEventHandlers` (no such
block), `webref idl html WindowEventHandlers` (18 attributes), `webref idl mathml-core MathMLElement`, `webref idl SVG2
SVGElement`. The `SVGElement` IDL is cited by its lookup, not by a section number.

### §3.1 User-input touch audit

Author-controlled inputs reaching the changed code: handler-named content attributes on any element (parser-baked,
`setAttribute`, `removeAttribute`), IDL handler writes on any element (through the D-8 entry), foreign content in
markup (`<svg>`, `<math>`, through `innerHTML` and the document parse), and, once `createElementNS` exists, a foreign
`body`. Every change removes an effect: a name that is not the element's event handler content attribute installs
nothing, and a foreign `body` loses an HTML interface. No handler is delivered anywhere it was not delivered on the
base, no target changes, and no security boundary (CSP, sandbox, the policy filter) is touched: a name removed by
the predicate would have been installed or policy-excluded on the base, never the reverse.

## §4. Mechanism

### §4.1 Home / kind

| Fact | Home / kind | Status |
|---|---|---|
| applicability (attribute change steps' step 1, local name) | `is_event_handler_content_attribute_of` (NEW), free fn, session-private, `session/event_handler_processing/mod.rs`; a pure read of the element's `Namespace` and `TagType` components and of the name's `HandlerScope` row | D-T8 |
| its three readers | the snapshot of `process_handler_content_attributes`, the filter of `is_only_deliverable_ordinary_name`, the head of `handle_attribute_change` | D-T8 (a), (b), (c) |
| `body`'s interface | the `body` arm of `tag_specific_t2b_prototype` and `require_body_receiver`, each adding `EcsDom::is_html_namespace` through `HostData::dom_shared` | D-T11 |

No component, store, trait, event variant, crate edge or `pub` item is added, and nothing is deleted.

### §4.2 Shape

The two functions T3 rewrites after this PR are `process_handler_content_attributes` and `handle_attribute_change`.
Their final shape (signature and branch structure, with the owner of each part) is decided once, in the umbrella's
§4.1; T3-ab writes its parts in that shape and changes neither signature.

```rust
// session/event_handler_processing/mod.rs
/// (NEW) D-T8. No wildcard: a new `Namespace` or `HandlerScope` does not compile until it is placed.
fn is_event_handler_content_attribute_of(dom: &EcsDom, element: Entity, name: &str) -> bool {
    let Some((_, scope)) = event_handler_attr_lookup(name) else { return false };
    match (dom.namespace_of(element), scope) {
        (Namespace::Svg, _) => true,                                                    // SVG2 §15.8
        (Namespace::Html | Namespace::MathMl,
            HandlerScope::Global | HandlerScope::DocumentElement) => true,              // G
        (Namespace::Html, HandlerScope::Window) => is_body_or_frameset(dom, element),   // W
        (Namespace::MathMl, HandlerScope::Window) => false,
        (Namespace::Html | Namespace::MathMl, HandlerScope::DocumentOnly) => false,     // D
    }
}

// (a) process_handler_content_attributes: the snapshot is the element's event handler content attributes
attrs.iter()
    .filter(|(name, _)| is_event_handler_content_attribute_of(dom, element, name))     // (NEW)
    .filter_map(|(name, value)| event_handler_attr_lookup(name).map(/* unchanged */))

// (b) is_only_deliverable_ordinary_name: the filter's first conjunct
is_event_handler_content_attribute_of(dom, element, n)      // replaces `event_handler_attr_lookup(n).is_some()`

// (c) session/event_handler_consumer.rs, handle_attribute_change, after the lookup's `let … else { return }`
if !is_event_handler_content_attribute_of(dom, node, name) { return; }                 // (NEW), before the policy filter

// elidex-js/vm/host/elements.rs, tag_specific_t2b_prototype
if host.tag_matches_ascii_case(entity, "body") && host.dom_shared().is_html_namespace(entity) { … }
// elidex-js/vm/host/event_handler_attrs.rs, require_body_receiver
if !(host.tag_matches_ascii_case(entity, "body") && host.dom_shared().is_html_namespace(entity)) { /* Illegal invocation */ }
```

Re-runnable after T3-ab: `git grep -n 'is_event_handler_content_attribute_of(' -- crates ':!*tests*'` → the
definition and three calls ((a), (b), (c)); `git grep -n 'event_handler_attr_lookup(' -- crates ':!*tests*'` → the
definition and four calls (the predicate, the snapshot's `filter_map`, the AttributeChange arm,
`is_window_delegating_pair`), so no reader of an element's attributes asks the name-only lookup alone.

In (a) the snapshot closure holds a shared borrow of `Attributes` from `dom.world()`, and the predicate takes `&EcsDom`:
two shared borrows, as `is_only_deliverable_ordinary_name` already does (`mod.rs:196-211`). An element's `TagType`
and `Namespace` do not change while its attributes are iterated.

A non-element reaches none of the three: (a) and (b) return when the entity has no `Attributes`; (c) is called with
the element of an `AttributeChange`. `namespace_of`'s `Html` default for a non-element is not relied on.

### §4.3 ECS-native and Layering checks

The predicate is a pure function of two components of one entity and a static table: a query, with no state of its
own. "Each element class declares the handler attributes it supports" (per-class tables on an inheritance chain in
object-oriented engines) → one predicate over the namespace and tag components. "The subclass prototype is picked by
tag name" → the same pick with the namespace component read beside the tag. Applicability is a function of
immutable facts, so it is not cached in a component (nothing to invalidate; the side-store rule has no state to
home). Layering: the table and its three readers live in `elidex-script-session`; `vm/host/` gains two component
reads through the existing `HostData::dom_shared` path, one for prototype selection and one for a brand check, both
of the kinds the Layering mandate names for `vm/host/`. The two `is_html_namespace` calls are the only direct
`EcsDom` method calls T3-ab adds there (H-A11: 0 and 0 on the base, 1 and 1 after; DoD item 4).

### §4.4 Docstring citation table (Axis 4)

| Item | Cites |
|---|---|
| `is_event_handler_content_attribute_of` (NEW) | HTML §8.1.8.1 Event handlers, attribute change steps step 1 (local-name half); HTML §8.1.8.2 (the lists: all HTML elements; body and frameset; Document objects); SVG2 §15.8 Event attributes; MathML Core §2.1.4 (the `MathMLElement` IDL: GlobalEventHandlers); the namespace half → `#11-event-handler-content-attribute-applicability` facet (2), slice b |
| `process_handler_content_attributes` | its docstring's "attribute change steps 1–5.6" gains "step 1's local-name half by `is_event_handler_content_attribute_of`"; "`element` is marked iff it carries at least one ordinary handler content attribute" reads "of its own" |
| `is_only_deliverable_ordinary_name` | "handler content attribute of `element`" is the predicate's |
| `handle_attribute_change` | HTML §8.1.8.1 attribute change steps step 1, before the A2 policy comment |
| the `body` arm, `require_body_receiver` | DOM §4.5 *element interface*; HTML `HTMLBodyElement` (HTML §8.1.8.2: content and IDL attributes "exposed on all body and frameset elements"); the foreign-interface defect (§8.2 item 1) |
| `HandlerScope::DocumentElement`'s doc | not edited (P-A1, §10 Q-A4) |

## §5. Populations, touched files, named risks

### §5.1 Behavioural population (re-runnable)

The predicate's domain is every row of `EVENT_HANDLER_ATTRS` (105, H-A1) × element class {HTML `body`, HTML
`frameset`, other HTML, SVG, MathML}. The pairs whose outcome changes are those the table answers "no": HTML `body` /
`frameset` × D (2 names each), other HTML × W and D (20 names), MathML × W and D (20 names). U-T8 derives its table
from `EVENT_HANDLER_ATTRS` and the scopes, not from a sample. D-T11's population is the entities with local name
`body` and a namespace other than HTML: at the base, harness-built only (H-A7).

### §5.2 Existing tests whose expectation changes (rewritten in the same commit)

Measured (verified 2026-11-15): `git grep -n -w -E '(<the 18 W names>|onreadystatechange|onvisibilitychange)'
cee74989 -- crates`, each hit that sets one as a content attribute read for its element; every other hit sets it on
a `body` or a `frameset`, or is a table, an IDL write or a non-DOM object's handler.

| Test | Base | T3-ab | Why |
|---|---|---|---|
| `u3_i_excluded_ordinary_pair_not_wired` (`session/event_handler_processing/tests_insertion_and_write.rs:76`) | a `div` × `onstorage` under the core policy: never wired | the same assertions on an SVG `rect` × `onstorage` | `onstorage` is not a `div`'s event handler content attribute, so on a `div` the cell would pass without reaching the policy filter. The one policy-excluded name is class W, so an excluded *ordinary* pair exists on an SVG element only (B3×B6) |
| `u3_i_excluded_ordinary_pair_marks_element` (`:87`) | the `div` is marked after the first append | the SVG `rect` is marked | as above; on a `div` the element is no longer marked (AP-7) |
| `onstorage_content_attribute_gated_by_policy` (`session/event_handler_consumer.rs:507`) | a `div` × `onstorage` through the AttributeChange arm: wired under BrowserCompat, not under BrowserCore / App | the same on a `body` | its positive control would fail on a `div` (AP-3); on a `body` the pair is applicable and, with no Window in that bare `EcsDom`, lands on the element as before (`resolve_handler_target`'s fallback) |

U-A3 (§6.2) adds the cell that the first two no longer are: a `div` × `onstorage` is neither wired nor marked under
either policy. Kept green and checked: slice a's cells (`tests_event_handler_attrs_once.rs`,
`tests_event_handler_attrs_window_delegating.rs`: every W name there is on a `body` or a `frameset`), N1's cells, the
shell E1–E8. Any other existing test that installs a handler through a non-applicable attribute is found by the full
run at the base and listed in the PR (§9 item 1).

### §5.3 Touched files

Sizes at `f51cb55b` (`git show f51cb55b:<path> | wc -l`, verified 2026-11-15; equal to `cee74989`'s).

- `crates/script/elidex-script-session/src/event_handler_processing/mod.rs` (410) — the predicate; readers (a), (b);
  `use elidex_ecs::Namespace`; `test_support` gains a foreign-element builder beside `elem`.
- `crates/script/elidex-script-session/src/event_handler_consumer.rs` (545) — reader (c); its import; the rewritten
  policy test.
- `crates/script/elidex-js/src/vm/host/elements.rs` (455) — the `body` arm.
- `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` (683) — `require_body_receiver`.
- tests: `session/event_handler_processing/tests_applicability.rs` (NEW; U-T8, U-A1 to U-A3),
  `crates/script/elidex-script-session/src/event_handler_processing/tests_insertion_and_write.rs` (346; the two U3 (i)
  cells), `elidex-js/vm/tests/tests_event_handler_applicability.rs` (NEW; T-19, T-21, T-A1 to T-A4),
  `crates/script/elidex-js/src/vm/test_helpers.rs` (the helper that builds the foreign `body` elements, §6.1).
- docs: none beyond this plan PR (the umbrella and the T3 memo are edited by the plan PR, not by the implementation).

No file is over 1000 lines, so the touch-time split check does not fire. Not touched: `crates/core/elidex-ecs` (the
namespace readers exist), `elidex-js/vm/host_data/` (D-T11), `elidex-js/vm/host/html_passive_protos.rs` (the body
pair's install is unchanged; only who receives the prototype changes).

### §5.4 Named risks

- **R-A1 — merge contact, no semantic contact.** L3's PR (io) edits `event_handler_processing/mod.rs`'s import line and
  `process_bound_document`'s walker call (`TemplateContentsWalk` → `WalkScope`); L3's tree-model slice `ta` re-types
  `get_parent` callers, of which T3-ab's four files have none (H-A9); T3-pre does not edit the event-handler consumer
  or the processing module (the T3-pre memo §5.3, *Not touched*). T3-ab's lines are the snapshot, the D-8 filter and the arm's
  head: a textual merge with io at most.
- **R-A2 — T3 rewrites the two functions after T3-ab.** Bounded by the umbrella's §4.1: T3 adds its parts around
  T3-ab's three lines and does not move them. **Stop condition (PM decision 741 (3)):** if T3's implementation finds
  it must move, re-order or re-write one of T3-ab's three reader sites, or change the predicate's signature, it stops
  for the PM (a mis-drawn slice).
- **R-A3 — a test or page that relied on a non-applicable name.** In-tree: three tests (§5.2). For content: a handler
  on such an attribute never ran on a spec-conforming engine, so no page can depend on it by the spec; no browser
  measurement is taken or needed (§10).
- **R-A4 — D-T11's reach is harness-only until `createElementNS` lands.** Its cell (T-19) is real code through the
  real wrapper path, but its element comes from the harness. When `#11-vm-create-element-ns-missing` is wired, T-19's
  script form (`document.createElementNS(…, 'body')`) becomes writable; §8.3 item 3 records that for the slot.

## §6. Test plan

Red-first: each **red** cell is committed failing on T3-ab's base before the fix commit.

### §6.1 VM integration (`tests_event_handler_applicability.rs`, NEW)

Every cell uses `with_page` / `run` (`elidex-js/vm/test_helpers.rs:168`, `:193`; `el` a connected element, `fn` counts
its calls in `fnRan`). **Foreign `body` elements are built by the harness** (P-A2): a helper beside `with_page` in
`elidex-js/vm/test_helpers.rs` (NEW helper; in the shared file because T3's cells T-9 and T-9b reuse it) creates,
before the script, an SVG `body` and a MathML `body` with `EcsDom::create_element_ns` owned by the bound document, and
exposes their wrappers as the globals `svgBody` and `mathBody` (the precedent: `setup_with_root_and_svg`,
`elidex-js/vm/tests/tests_mutation_observer/mod.rs:136`, which states why: "JS `createElementNS` is not VM-wired").
The wrappers go through the production `create_element_wrapper`, so D-T11's arm is the code under test.

| Cell | Kind | Script | Base | T3-ab (= spec) |
|---|---|---|---|---|
| T-19 | **red (R-D1; FN-3; D-T11)** | `svgBody.onhashchange = fn; svgBody.dispatchEvent(new Event('hashchange')); r.push(fnRan === 0, window.onhashchange === null, svgBody instanceof HTMLBodyElement); mathBody.onhashchange = fn; r.push(window.onhashchange === null); try { Object.getOwnPropertyDescriptor(HTMLBodyElement.prototype,'onhashchange').get.call(svgBody); r.push(false) } catch (e) { r.push(e instanceof TypeError) }` | `true,false,true,false,false` | `true,true,false,true,true` |
| T-21 | **red (R-D2; FN-4; D-T8)** | `mathBody.setAttribute('onhashchange','window.hit = 1'); mathBody.dispatchEvent(new Event('hashchange')); r.push(window.hit === undefined, window.onhashchange === null)` | `true,false` (on the Window) | `true,true` |
| T-A1 | red (AP-3, AP-4; script-reachable) | `var d = document.createElement('div'); d.setAttribute('onhashchange','window.h = 1'); d.setAttribute('onreadystatechange','window.s = 1'); d.setAttribute('onclick','window.c = 1'); for (const t of ['hashchange','readystatechange','click']) d.dispatchEvent(new Event(t)); r.push(window.h === undefined, window.s === undefined, window.c === 1)` | `false,false,true` | `true,true,true` |
| T-A2 | red (AP-6, AP-10; a parser-built MathML element; static; confirmed red-first at implementation) | `el.innerHTML = '<math><mi onhashchange="window.h = 1" onclick="window.c = 1"></mi></math>'; var mi = el.firstChild.firstChild; mi.dispatchEvent(new Event('hashchange')); mi.dispatchEvent(new Event('click')); r.push(window.h === undefined, window.c === 1)` | `false,true` | `true,true` |
| T-A3 | guard (AP-5; a parser-built SVG element keeps every name; static; confirmed at implementation) | `el.innerHTML = '<svg><rect onhashchange="window.h = 1" onreadystatechange="window.s = 1"></rect></svg>'; var rc = el.firstChild.firstChild; rc.dispatchEvent(new Event('hashchange')); rc.dispatchEvent(new Event('readystatechange')); r.push(window.h === 1, window.s === 1)` | `true,true` | `true,true` (SVG treated as HTML or MathML: `false,false`) |
| T-A4 | red (AP-8; D-T8 (b)) | `el.innerHTML = '<div onclick="window.c = 1" onhashchange="window.h = 1"></div>'; var c = el.firstChild.cloneNode(); c.onclick = fn; el.append(c); c.dispatchEvent(new Event('click')); r.push(fnRan === 1, window.c === undefined)` | `false,false` (two deliverable names: not processed at the write, D-8; the first insertion re-raws `onclick` over `fn`) | `true,true` (one deliverable name: processed at the write, then `fn` replaces it; the insertion finds the clone marked). Equal after T3, which processes the clone at its creation |

### §6.2 Unit (session, `tests_applicability.rs`, NEW)

| Cell | Kind | Shape | Oracle |
|---|---|---|---|
| U-T8 | red (full enum; AP-1 to AP-6, AP-9, AP-10, FN-4, FN-5) | for every `EVENT_HANDLER_ATTRS` row × element {HTML `body`, HTML `frameset`, HTML `div`, SVG `body`, SVG `rect`, MathML `body`, MathML `frameset`, MathML `mi`}: the predicate's value; and, for each pair, the entry on the element after (i) an `AttributeChange` through the consumer and (ii) a first insertion of an element carrying it | D-T8's table, computed from the row's `HandlerScope` and the element's class: (i) and (ii) leave an entry, on `resolve_handler_target`'s target (HEAD's), iff the predicate is true. Base: every pair leaves one |
| U-A1 | red (AP-8; D-T8 (b)) | an unmarked `div` with `onclick=A` and `onhashchange=B`; `process_before_handler_write(dom, div, "onclick", compat())` | marked, `click` raw `A`, no `hashchange` entry. Base: unmarked, no entry |
| U-A2 | red (AP-7; B3×B5) | a `div` whose only handler-named attribute is `onhashchange`; first insertion under the compat policy | unmarked, no entry. Base: marked, `hashchange` raw on the `div` |
| U-A3 | guard (B3×B6: applicability precedes the policy filter) | a `div` × `onstorage` under the core policy and under the compat policy; first insertion, and an `AttributeChange` | neither wired nor marked under either. Mutant "the predicate applied after the mark's count (a `continue` beside the policy filter instead of the snapshot's filter)": marked under both |

The D-8 entry and the mark are also run through the rewritten U3 (i) cells (§5.2) on an SVG `rect`.

### §6.3 Mutation guards

| Mutant | Killed by |
|---|---|
| the predicate always true (the base) | T-21, T-A1, T-A2, U-T8 |
| MathML treated as SVG (every name) | T-21, T-A2, U-T8 (MathML rows) |
| SVG treated as HTML or MathML | T-A3, U-T8 (SVG rows), the rewritten U3 (i) cells |
| class W accepted on every HTML element | T-A1, U-T8 (`div` rows) |
| class W refused on `frameset` (the HTML arm testing `body` only) | U-T8 (HTML `frameset` rows) |
| class D accepted on HTML `body` / `frameset` | U-T8 (AP-2 rows) |
| `DocumentElement` treated as D (`oncopy`, `oncut`, `onpaste` refused) | U-T8 (those three rows × HTML and MathML) |
| reader (a) without the predicate (processing at bind / insertion installs every name) | U-T8 (ii), U-A2 |
| reader (c) without the predicate (`setAttribute` installs every name) | T-21, T-A1, U-T8 (i) |
| reader (b) without the predicate | T-A4, U-A1 |
| the predicate applied after the mark's count at (a) (a `continue` beside the policy filter instead of the snapshot's filter) | U-A3, U-A2 |
| the `body` arm tag-only | T-19 (its third and fifth values) |
| `require_body_receiver` tag-only | T-19 (its fifth value: the accessor called on a foreign `body`) |
| D-T11 applied to the receiver check but written with `\|\|` (a non-`body` HTML element passes) | the existing F-24 `body_weh_accessor_rejects_non_body_receiver` (`elidex-js/vm/tests/tests_event_handler_attrs.rs:467`), kept green |

## §7. Out of scope and slots

| Item | Why not T3-ab | Home |
|---|---|---|
| target determination, every facet; the namespace in `is_body_or_frameset` | T3 (PM decisions 25, 129) | the T3 memo D-T1 to D-T7 |
| an SVG `body`'s class W content attribute reaching the Window | the target rule (T3 memo FN-2, D-T5) | T3 |
| attribute change steps step 1, namespace half | needs `MutationEvent::AttributeChange` to carry a namespace (H-A8) | `#11-event-handler-content-attribute-applicability` facet (2), slice b (§8.3 item 1) |
| foreign elements' HTML interfaces other than `body`'s | needs the SVG / MathML interface families (D-T11) | proposed defect text (§8.2 item 1; PM to route) |
| `document.createElementNS` | not this program's | `#11-vm-create-element-ns-missing` (L3; §8.3 item 3) |
| an element outside the three namespaces | no representation (P-A3) | recorded in §8.3 item 1's text; arises with a fourth `Namespace` variant |
| `HandlerScope::DocumentElement`'s doc and IDL install | not applicability | §10 Q-A4 (scout question first) |

Own deferrals: **0**. Residuals: **0**.

## §8. Edges, hand-offs, records

### §8.1 Placement (measured) and edges

**What T3-ab needs on its base.** `EVENT_HANDLER_ATTRS` with `HandlerScope`, slice a's processing function, D-8 entry
and AttributeChange arm (`mod.rs`, `event_handler_consumer.rs`; landed by #563), `EcsDom::namespace_of` /
`is_html_namespace` and `HostData::dom_shared` (older). All are on main at `cee74989` and at `f51cb55b` (H-A10).

**What T3-ab does not need.**

| Unit | Does T3-ab depend on it? | Why | Contact |
|---|---|---|---|
| L3 PR (is), PR (ig) | no | no dispatcher window, no take / restore | none |
| T3-pre | no | no creation event; the `SubtreeCreated` arms are not touched | none (T3-pre does not edit T3-ab's files) |
| L3's tree-model program (ta → tc / tm / tr / tf → te → ts) | no | no parent or child link read (H-A9) | none |
| L3 PR (io) | no | no walker call added | textual, in `mod.rs` (R-A1) |
| T3 | T3 depends on T3-ab (x15) | T3's rewrite sits around T3-ab's three reader sites; PM decision 153's R-D is closed before T3 | T3 is written on T3-ab's landed code |

**Result.** T3-ab can be cut from main now and can land before, between or after any of is, ig, T3-pre, the tree-model
program and io; it must land before T3's implementation branch is cut. With L3's estimates as a premise, not a
commitment (ig ≈ 11-22 to 11-24; the tree-model program after it; io ≈ 12-15; T3 no earlier than 12-17), T3's cut
waits on io in every case, and T3-ab's own path (plan-review R1, implementation, pre-push gate) has from now until
then.

**The no-op window of T3-pre's `FragmentParse` arms** (the T3-pre memo DoD item 7; sequence ig → T3-pre → tree-model
program → io → T3). Facts, without a ruling: (1) T3-ab is not in that sequence and has no edge to any unit in it
except T3; (2) the window's two ends are T3-pre's landing and T3's landing, and T3-ab changes neither date as long as
it lands before io does, since T3's cut waits on io regardless; (3) it would lengthen the window only if T3-ab were
still unlanded when io lands, which is a question of the Layout lane's capacity (T3-ab and T3-pre's implementation
are both Layout's and overlap from ig's landing), not of dependency; (4) DoD item 7's escalation clause "another
unit placed before T3" matches T3-ab literally. Whether (4) is an escalation or is discharged by (1) to (3) is the
PM's (§10 Q-A3).

**Edge (umbrella §1.3, added by this plan PR):** **x15** T3-ab → T3, code predecessor + decision precondition (PM
decisions 153, 741 (3)).

### §8.2 Slot texts (for the PM; defects only)

1. **Foreign elements take HTML interfaces** (pre-existing, engine-wide; PM to route; the ledger has no row for it,
   checked 2026-10-31 by T3 rev 2 and not re-checked here; candidate home: the master roadmap's `M4-13:#svg` row).
   Text: "The VM chooses an element wrapper's prototype by tag name alone: `prototype_kind_for` returns
   `PrototypeKind::Element` for every `TagType` entity
   (`crates/script/elidex-js/src/vm/host_data/engine_feature/entity_kind.rs:164-165` at `cee74989`), and the per-tag
   selection (`crates/script/elidex-js/src/vm/host/elements.rs:216-`) uses `tag_matches_ascii_case`
   (`entity_kind.rs:124-133`), which reads no namespace, as do the per-interface receiver checks that call it (e.g.
   `html_style_proto.rs:90`). So an SVG or MathML element whose local name is an HTML tag (`a`, `script`, `style`,
   `title`, `div`, …) gets that HTML interface's prototype, and every other foreign element gets
   `HTMLElement.prototype`; no `SVGElement` / `MathMLElement` interface is exposed (WHATWG DOM §4.5 *element
   interface*; MathML Core §2.2.3; SVG2). Since T3-ab (#<PR>, `<squash>`) the `body` arm and `HTMLBodyElement`'s
   receiver check also require the HTML namespace (T3-ab memo D-T11), so a foreign `body` takes
   `HTMLElement.prototype`. The rest is ordered by what it needs: (i) arms whose local name is not an SVG or MathML
   element name can require the HTML namespace with no new interface (toward the spec at every member); (ii) arms
   whose local name is also a foreign element name (`a`, `script`, `style`, `title`, …) need the foreign interfaces
   first, since `HTMLElement.prototype` lacks members the foreign interface has (e.g. `SVGScriptElement`'s `type`).
   **Trigger:** an SVG / MathML DOM interface PR, the `M4-13:#svg` program's plan, or a WPT `svg/` / `mathml/` subset.
   **Re-eval:** 2026-12-31. ⚠ No remedy reviewed."

### §8.3 Records for existing slots (for the PM, exact texts; at T3-ab's landing)

1. `#11-event-handler-content-attribute-applicability` — **register** (the ledger has no row; PM 377 set its
   registration at T3's landing, which PM decision 741 (3) moves to T3-ab's) with this text: "pre-existing. The
   attribute change steps (HTML §8.1.8.1 Event handlers) step 1 return when the attribute's namespace is not null,
   or its local name is not the name of an event handler content attribute on the element. Before T3-ab elidex
   realised neither test: (1) every reader looked the name up in `EVENT_HANDLER_ATTRS` regardless of the element
   (`event_handler_attr_lookup`, `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:237` at
   `cee74989`), so a WindowEventHandlers name on an HTML element other than `body` / `frameset` (HTML §8.1.8.2: they
   are content attributes of body and frameset elements only), or a Document-only name
   (`HandlerScope::DocumentOnly`) on any HTML element, installed a handler on that element; (2)
   `MutationEvent::AttributeChange` carries no namespace
   (`crates/core/elidex-ecs/src/dom/mutation_event.rs:366-371` at `cee74989`), so `setAttributeNS(ns, 'onclick', X)`
   with a non-null `ns` installs one; (3) which names MathML elements accept was not modelled: every name installed,
   on a MathML `body` a WindowEventHandlers name reaching the Window (the tag-only target rule), where MathML Core
   §2.1.4 admits only the names HTML supports on all HTML elements (the `MathMLElement` IDL: GlobalEventHandlers). SVG
   elements are not a defect: SVG2 §15.8 Event attributes makes every supported event type an event attribute of
   every SVG element. **Facets (1) and (3) closed by T3-ab** (#<PR>, `<squash>`,
   `is_event_handler_content_attribute_of`; PM decisions 168, 741 (3)): an HTML element accepts the names HTML
   supports on all HTML elements (`HandlerScope::Global` and `DocumentElement`) and, on `body` / `frameset` only, the
   WindowEventHandlers names; an SVG element every name; a MathML element the names HTML supports on all HTML
   elements; applied by every reader of an element's handler content attributes (the processing function, the D-8
   restriction's count, the AttributeChange arm; T3-ab memo D-T8, §2.1, cells T-21, T-A1, T-A2, T-A4, U-T8). An
   element outside the HTML, SVG and MathML namespaces has no representation (`Namespace` has three variants); the
   spec gives such an element no event handler content attribute, and the predicate's exhaustive `match` forces that
   decision when a variant is added. **Facet (2) remains** (a non-null attribute namespace). **Owner:** Layout lane
   (program `EH-attrs-at-creation`); **closes at slice b**, which adds the namespace to the event and re-runs the
   writer census (S-7). **Trigger:** slice b's plan. ⚠ No remedy reviewed."
2. `#11-eh-attrs-creation-time-call-sites` — **no record.** Defect (1)'s "two or more deliverable ordinary handler
   content attributes" reads in the spec's sense after T3-ab (an attribute that is not the element's event handler
   content attribute does not count, row AP-8); its text is unchanged.
3. `#11-vm-create-element-ns-missing` — append: "Since T3-ab (#<PR>, `<squash>`) two outcomes for a foreign `body`
   are pinned on harness-built elements only, because no script can create one (the HTML parser's foreign content
   breaks out at a `body` start tag, HTML §13.2.6.5): a WindowEventHandlers content attribute on a MathML `body`
   installs nothing, and an SVG or MathML `body` does not get `HTMLBodyElement.prototype` (T3-ab memo cells T-21,
   T-19). When `createElementNS` is wired those cells take their script form. The same wiring makes a non-lowercase
   HTML-namespace local name creatable (`createElementNS(html, 'BODY')`), which the ASCII-case-insensitive tag
   predicates (`tag_matches_ascii_case`, the session's `is_body_or_frameset`, `EcsDom::is_base_element`) would then
   treat as the lowercase element (T3 memo §7)."

### §8.4 Hand-offs

- **T3** (x15): T3 is written on T3-ab's landed code. It relies on: the predicate with the signature of §4.2, its
  three reader sites, and D-T11's two tests. It adds its own parts to the two functions as the umbrella's §4.1
  states, and its `ProcessingTiming::Creation` (NEW: T3) path gets the applicability test through reader (a) with no edit (T3
  memo A7×A12). T3's D-T5 adds the HTML namespace to `is_body_or_frameset`; the predicate's HTML arm calls it and is
  unaffected.
- **Slice b**: closes facet (2), extending the predicate's callers with the attribute's namespace once the event
  carries it (H-A8).
- **L3's `#11-vm-create-element-ns-missing`**: §8.3 item 3.

## §9. Definition of done

1. T3-ab's branch is cut from main (no prerequisite beyond what is on main, §8.1); every `cee74989` coordinate in
   §0.2 is re-measured there by its command; the full run of `elidex-script-session`, `elidex-js` and `elidex-shell`
   on the red commit lists every existing test the predicate changes, and any beyond §5.2's three is added to the PR
   description with its row of §2.1.
2. `/elidex-plan-review` passes on this memo (R1, five axes; PM decision 741 (3)).
3. Red-first: T-19, T-21, T-A1, T-A2, T-A4, U-T8, U-A1, U-A2 committed failing; the fix makes them and §5.2's three
   rewritten tests green; the guards T-A3 and U-A3 are green at the head; the static-marked expectations (T-A2, T-A3)
   are confirmed by their runs.
4. Structural greps:
   - `git grep -n 'is_event_handler_content_attribute_of(' -- crates ':!*tests*'` → the definition and three calls
     (§4.2);
   - `git grep -n 'event_handler_attr_lookup(n).is_some()' -- crates` → 0 (reader (b));
   - `/usr/bin/grep -n 'tag_matches_ascii_case(entity, "body")' crates/script/elidex-js/src/vm/host/elements.rs
     crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` → two hits, each on a line (or in an expression) that
     also reads `is_html_namespace` (D-T11);
   - the direct `EcsDom` method calls in `vm/host/` (R2 Axis 1): `for f in
     crates/script/elidex-js/src/vm/host/event_handler_attrs.rs crates/script/elidex-js/src/vm/host/elements.rs; do
     /usr/bin/grep -c -E 'dom_shared\(\)\.|\.dom\(\)\.' $f; done` → 1 and 1 (base: 0 and 0, H-A11), each the
     `is_html_namespace` read;
   - the predicate's `match` has no `_` arm and no catch-all binding (read at the diff).
5. `cargo test -p elidex-script-session -p elidex-js -p elidex-shell --all-features` green; `mise run ci` before push.
6. §8.2 item 1 and §8.3's records (items 1 and 3) handed to the PM in the PR description.
7. **Checker** (run; do not trust a stored verdict): `python3 .claude/skills/elidex-plan-review/preflight.py
   docs/plans/2026-11-eh-attrs-t3-applicability-and-body-prototype.md` exit 0.

## §10. Open questions

**Decided:** the carve (PM decision 741 (3)); D-T8's table for the three namespaces (the specs, H-A2); D-T11 (PM
decision 153's R-D, T3 rev 2 to rev 5).

**Within the carve, the designer's (not new scope; flagged for the plan-review):** reader (b), the D-8 count, applies
the predicate (D-T8); `DocumentElement` is class G (P-A1); the foreign `body` cells are harness-built (P-A2).

**For the PM:**
- **Q-A1 (the shared functions' shape).** The umbrella's §4.1 states the final shape of the two functions once.
  T3-ab writes its three reader sites in their final place and form and changes no signature; T3 later adds the
  `active` parameter, because its type `ActiveDocument` and its two marshalling points are T3's D-T3. If PM decision
  741 (3)'s condition means that T3-ab must also write the final *signature*, D-T3's type, constructor and
  marshalling would have to move into T3-ab with the parameter unread until T3; this memo does not do that. A ruling
  is asked on which reading holds.
- **Q-A2 (a further split).** D-T8 and D-T11 can land as two PRs in either order with no row worse (§1.3). One slice
  or two?
- **Q-A3 (the no-op window).** §8.1's four facts; is T3-ab an escalation under the T3-pre memo DoD item 7?
- **Q-A4 (scout question, then the PM's routing).** `HandlerScope::DocumentElement`'s doc says "Document + Element"
  for `oncopy`, `oncut`, `onpaste`, which the HTML IDL now lists in `GlobalEventHandlers` (also on Window). Does
  `window.oncopy` have an accessor in the VM (`git grep -n 'DocumentElement' -- crates/script/elidex-js/src/vm`)? If
  not, that is a pre-existing IDL-surface defect outside this program.
- **Q-A5 (the label).** `T3-ab` is a proposal.

No browser measurement is needed: every outcome in §2.1 is spec-defined (H-A2).
