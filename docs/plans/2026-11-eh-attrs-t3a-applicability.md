# T3-a `EH-attrs-at-creation-T3-a` — a name is processed only if it is an event handler content attribute of its element (per-PR plan-memo)

*Provenance: split from the T3-ab memo (`docs/plans/2026-11-eh-attrs-t3-applicability-and-body-prototype.md`, rev 1.1;
deleted by the same plan PR) by PM decision 767, following plan-review R3 wave 1's Axis 3 split verdict (finding W1-1;
PM 227). Its other half is the T3-b memo (`docs/plans/2026-11-eh-attrs-t3b-body-prototype.md`).*

Nested PR **T3-a** ("applicability") of the standalone program `EH-attrs-at-creation` (umbrella
`docs/plans/2026-10-event-handler-attrs-at-creation.md`, "the umbrella"; its in-place edits for this PR are carried by
the T3 memo's *Rev 8.<t>* entry, since the four memos land in one plan PR). It is a prerequisite of **T3**
(`docs/plans/2026-10-eh-attrs-t3-target-determination.md`, "the T3 memo", rev 6.3). It carries one decision, with its id
kept from the T3 memo rev 5: **D-T8** (which names are event handler content attributes of an element). The ids of
premises, rows and cells (H-A*, P-A*, B*, AP-*, FN-*, T-*, U-*) are kept from the T3-ab memo, so an id names the same
thing in the T3 memo, in the T3-b memo and here; ids this memo does not carry (H-A4, H-A11, B4, B10, FN-3, T-19) are
the T3-b memo's. Rev 1.1 (2026-11-18).

**Review numbering.** This memo's own plan-review rounds are numbered from **R1** (its first full five-axis review).
"T3's R1" and "T3's R2" are the reviews of the T3 memo rev 4.8.2 and rev 5; "R3 wave 1" is the review of the T3-ab
memo rev 1.1 (with the T3-pre memo rev 2.1) whose findings this memo folds (W1-*; listed in the Changelog).
**This memo's R1 is "R3 wave 2" in the PM's records** (PM decision 767 (3): wave 2 reviews T3 rev 6 and the two new
memos with five axes); its findings are folded in rev 1.1 (Changelog).

**Owner: Layout lane (L2)**, plan and implementation (PM decisions 741 (3), 767). No surface of another lane is
changed.

**Placement and order (PM decisions 741 (3), 767; measured in §8.1).** Row **T3-a** of the umbrella's §1.2. One edge:
**x15** T3-a → T3 (umbrella §1.3). **No edge with T3-b, L3's PR (is), PR (ig), PR (io), L3's tree-model program or
T3-pre**: T3-a uses no creation event, no dispatcher window, no walker and no parent or child link (§8.1, H-A9), and
it shares no code and no cell with T3-b (§1.3). Its prerequisites are all on main (slice a, N1, C0, C1; §8.1), so its
implementation branch can be cut from main today, and it lands in parallel with every unit between C1 and T3. The
user's order (PM decision 153: N1 → C0 → C1 → T3 → slice b) is kept: T3-a is a part of T3's scope landing earlier,
between C1 and T3. PM decision 168, which placed D-T8 in T3, is revised by PM decision 741 (3), and its placing of
applicability facet (2) with slice b by PM decision 770 (§8.3 item 1).

**Base.** main when T3-a's implementation branch is cut. Every code coordinate below is a `cee74989` coordinate (#626,
the T3 memo's frame) and is re-run at the base (§9 item 1). main is past `cee74989` (`653d7c24`, #636, on
2026-11-18, which moves html5ever from 0.39.0 to 0.40.1); none of the files this memo cites differs there (H-A10), and
H-A7's tolerant half is read at both html5ever versions.

**Scope.**
- **D-T8.** One predicate, `is_event_handler_content_attribute_of(dom, element, name)` (NEW), realises the local-name
  half of the attribute change steps' step 1 (WHATWG HTML §8.1.8.1 Event handlers: "localName is not the name of an
  event handler content attribute on element, then return"). Every reader of "the handler content attributes of this
  element" applies it: the processing function's snapshot, the D-8 restriction's count, and the consumer's
  AttributeChange arm. This closes facets (1) and (3) of `#11-event-handler-content-attribute-applicability` and T3
  rev 5's residual R-D2.
- Cells T-21, U-T8 (ids kept from the T3 memo rev 5), T-A1 to T-A4, U-A1 to U-A3 (§6); three existing tests rewritten
  (§5.2).

**Not in scope.** `body`'s interface (D-T11, the T3-b memo); target determination, every facet (T3: a `frameset`, the
reflecting six and an SVG `body` keep HEAD's target here, §1.2); the attribute change steps' step 1 namespace half
(applicability facet (2); unreachable at the base, and homed in `#11-attribute-namespace-model` by PM decision 770,
§8.3 item 1); the HTML-namespace test of step 1's class in
`is_body_or_frameset` (T3 memo D-T5); `document.createElementNS` (`#11-vm-create-element-ns-missing`, L3); the
completeness of the name table against `GlobalEventHandlers` and its partial extensions (a pre-existing table defect,
§7).

**Changelog.** *Rev 1.1* (2026-11-18; this memo's R1 = R3 wave 2: CRIT 0 / IMP 2 / MIN 9; PM decisions 756, 761,
770). **IMP (Axis 4; entered with the wave-1 fold):** class G's ground. "85 = HTML §8.1.8.2's lists (i) and (ii)"
was false: those lists give 76 names, of which the table has 71, and its other 14 G names are members that Pointer
Events §5 and CSS Transitions §6.3 add to the mixin by `partial interface mixin GlobalEventHandlers`. G is restated
as the table's members of `GlobalEventHandlers` including its partial extensions (§0.1, H-A2, D-T8's table, §3,
§4.4, §5.1); the names the table lacks are stated as a pre-existing defect outside T3-a (§7, §10 Q-A7). No outcome
changes: the predicate reads `HandlerScope`, not a list. **IMP (Axis 2; the T3-side gap of the dry-run):** U-T8's
target oracle, the rewritten `onstorage` test and every caller of the session harness are rewritten by T3; the list
and T3's expectations are the T3 memo §5.2's, and §8.4 hands them off (this memo's oracles hold at its own base).
**PM decision 770 (Q-A6 answered):** facet (2) moves from slice b to `#11-attribute-namespace-model` (owner L3); §7,
§8.3 item 1 (the landing list's old → new), §8.4, §4.4 and Q-A6 follow. **MINs:** MathML Core §2.2.3 is cited for
`MathMLElement includes GlobalEventHandlers` (H-A2, D-T8, §3, §4.4); the docstrings that say "attribute change steps
1–5.6" name step 1's namespace half as not realised (§4.4); the X-W timing of a foreign `frameset`'s pairs is a
base-equal row (§1.2, §2.1 AP-5, AP-10); §8.1's *Result* and Q-A3 no longer allow a landing after io and point to
the T3-pre memo DoD item 7 for the condition and to its §8.3 item 3 for the escalation; R-A1 and H-A9 say that L3's
slice `ta` deletes `get_parent` and cover this memo's new test files; H-A7's tolerant half has a command, at both
html5ever versions; U-T8 states its policy and takes a fresh DOM per (pair, leg) (§6.2); the review numbering maps
R1 to R3 wave 2; the base names `653d7c24` (H-A10).
*Rev 1* (2026-11-18; PM decision 767): new memo, the D-T8 half of the T3-ab memo rev 1.1, carried
across unchanged except for the folds below. The T3-ab memo's own history (rev 1, 2026-11-15, PM decision 741 (3):
what moved from the T3 memo rev 5; rev 1.1, PM decision 756) is in that file's last revision (`0f8113eb`).
Folded from R3 wave 1: **W1-3** (facet (2) restated as unreachable at the base, with its closure prerequisites: H-A8,
§4.4, §7, §8.3 item 1, §8.4, Q-A6); MINs: the predicate is `pub(crate)` and reader (c)'s file imports it (§0.4,
§4.1, §4.2, §5.3); U-T8 gains a removal leg for row OR-1a (§2.1, §6.2, §6.3); class G is HTML §8.1.8.2's lists (i)
and (ii), and the MathML row's ground quotes "as defined in the MathMLElement IDL" (§0.1, §0.4, H-A2); §1.3's K and M
are `webref coverage-map` output; this memo's review numbering is stated (above); the `event_handler_attr_lookup`
docstring row (§4.4); §8.3 item 1 has a Re-eval date; §8.1's no-op-window paragraph is a pointer to the T3-pre memo
DoD item 7. Also: Q-A4's routing is the slot `#11-clipboard-handlers-missing-on-window` (PM decision 764); the
reviewer's note on `SVGSVGElement includes WindowEventHandlers` is answered in D-T8 (no change to the SVG row).

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every code `file:line` is a **`cee74989`** coordinate, measured with `git grep -n -E '<pattern>' cee74989 -- <paths>`,
`git show cee74989:<path> | sed -n <range>p` or `/usr/bin/grep -n` (never the default ugrep; never `\b` under `git
grep -E`, which matches nothing). Paths are abbreviated after first use: `session/…` =
`crates/script/elidex-script-session/src/…`, `elidex-js/…` = `crates/script/elidex-js/src/…`, `ecs/…` =
`crates/core/elidex-ecs/src/…`. Spec text is `.claude/tools/webref body` / `idl` output of 2026-11-15, re-run
2026-11-18; every `§X.Y = title` pair is `webref heading` / `dfn` output (§3's footnote). Symbols written as `Type` +
method (`EcsDom::is_html_namespace`, `EcsDom::namespace_of`, `EcsDom::create_element_ns`, `test_support::elem`,
`test_support::install`) name
existing methods inside `impl` / `mod` blocks, which preflight's literal symbol grep does not find (its soft warnings).

**Name classes** (rows of `EVENT_HANDLER_ATTRS`, `session/event_handler_consumer.rs:79`, by their `HandlerScope`,
`:63`; counts by H-A1): **G** = `Global` (82) and `DocumentElement` (3), 85 names: the table's members of
`interface mixin GlobalEventHandlers` **including its partial extensions** (H-A2). 71 are in HTML's own IDL block
(HTML §8.1.8.2.1 IDL definitions), whose 76 attributes are HTML §8.1.8.2's lists (i) and (ii); the other 14 are added
to the mixin by `partial interface mixin GlobalEventHandlers` in Pointer Events §5 Extensions to the
GlobalEventHandlers mixin (10 names) and CSS Transitions §6.3 Event handlers on elements, Document objects, and Window
objects (4 names; its IDL is §7.1 IDL Definition). (Until rev 1.1: "85 names: HTML §8.1.8.2's lists (i) and (ii)
together", which is false: the lists give 76. The table also lacks members of the mixin, a pre-existing defect that
T3-a does not touch, §7.) List (i) is "must be supported by
all HTML elements"; list (ii), the six Window-reflecting names, is supported by all HTML elements other than `body` and
`frameset` as their own handlers, and exposed on `body` and `frameset` as the Window's. **W** = `Window` (18), the
WindowEventHandlers names, list (iii); **D** = `DocumentOnly` (2), `onreadystatechange` and `onvisibilitychange`, list
(iv). **Element classes** are by `Namespace` (`ecs/components.rs:62`: `Html`, `Svg`, `MathMl`) and, for HTML, by
whether the local name is `body` or `frameset`.

### §0.2 Premises re-measured

| ID | Premise | Command | Result at `cee74989` |
|---|---|---|---|
| H-A1 | The name table and its scopes | `for s in Global DocumentElement Window DocumentOnly; do git show cee74989:crates/script/elidex-script-session/src/event_handler_consumer.rs \| /usr/bin/grep -c "HandlerScope::$s),"; done` | 82, 3, 18, 2: 105 rows. `DocumentElement` is `oncopy`, `oncut`, `onpaste` (`:164-166`); `DocumentOnly` is `onreadystatechange`, `onvisibilitychange` (`:187-188`) |
| H-A2 | What each spec says about applicability | `.claude/tools/webref body html 'event-handlers-on-elements,-document-objects,-and-window-objects'`; `webref idl html GlobalEventHandlers`; `webref idl html DocumentAndElementEventHandlers`; `webref body SVG2 EventAttributes`; `webref body mathml-core attributes-common-to-html-and-mathml-elements`; `webref idl mathml-core MathMLElement`; `webref body mathml-core dom-and-javascript`; `webref body html event-handler-attributes`; `webref idl pointerevents4 GlobalEventHandlers`; `webref idl css-transitions-1 GlobalEventHandlers`; the set difference: `webref idl html GlobalEventHandlers \| /usr/bin/grep -oE 'on[a-z]+;'` against `git show cee74989:crates/script/elidex-script-session/src/event_handler_consumer.rs \| /usr/bin/grep -E 'HandlerScope::(Global\|DocumentElement)\),' \| /usr/bin/grep -oE '"on[a-z]+"'`, with `comm` | HTML §8.1.8.2, four lists: (i) "must be supported by all HTML elements, as both event handler content attributes and event handler IDL attributes" (`oncopy`, `oncut`, `onpaste` are in it, and in `interface mixin GlobalEventHandlers`; no `DocumentAndElementEventHandlers` IDL block exists); (ii) the six of the *Window-reflecting body element event handler set*: "must be supported by all HTML elements other than body and frameset elements … and with corresponding event handler content attributes and event handler IDL attributes exposed on all body and frameset elements"; (iii) the WindowEventHandlers names: "supported by Window objects … with corresponding event handler content attributes and event handler IDL attributes exposed on all body and frameset elements"; (iv) `onreadystatechange`, `onvisibilitychange`: "must be supported on Document objects as event handler IDL attributes" (no content attribute on any element). Lists (i) and (ii), 70 and 6 names, are the 76 attributes of HTML's own `GlobalEventHandlers` block. **The mixin has more members than those lists (rev 1.1):** other specs extend it with `partial interface mixin GlobalEventHandlers`, which a spec that defines new event types uses to add their handlers. The table's class G (85, H-A1) is 71 of HTML's 76 plus 14 from two such extensions: Pointer Events §5 Extensions to the GlobalEventHandlers mixin (`ongotpointercapture`, `onlostpointercapture`, `onpointercancel`, `onpointerdown`, `onpointerenter`, `onpointerleave`, `onpointermove`, `onpointerout`, `onpointerover`, `onpointerup`: 10) and CSS Transitions §6.3 Event handlers on elements, Document objects, and Window objects ("must be supported by all HTML elements, as both event handler content attributes and event handler IDL attributes; and that must be supported by all Document and Window objects, as event handler IDL attributes"; `ontransitioncancel`, `ontransitionend`, `ontransitionrun`, `ontransitionstart`: 4; the IDL is its §7.1 IDL Definition). For Pointer Events §5 the ground read here is the heading and the IDL extract: `webref body` cannot fetch its prose (the anchor is not in the rendered page). The 5 of HTML's 76 that the table lacks are `oncommand` and the four `onwebkit*` names (`onwebkitanimationend`, `onwebkitanimationiteration`, `onwebkitanimationstart`, `onwebkittransitionend`); it also lacks members of partial mixins (measured, not exhaustive: Pointer Events' `onpointerrawupdate`; CSS Animations' four `onanimation*`; Selection API's `onselectstart`, `onselectionchange`; Touch Events' four `ontouch*`): 0 of these 16 names are rows at `cee74989`. A pre-existing table defect, not T3-a's (§7). SVG2 §15.8: "For every event type that the user agent supports, SVG supports that as an event attribute … The event attributes are available on all SVG elements". MathML Core §2.1.4: "All event handler content attributes noted by HTML as being supported by all HTMLElements are supported by all MathML elements as well, as defined in the MathMLElement IDL"; MathML Core §2.2.3 DOM and JavaScript has that IDL: `interface MathMLElement : Element { }`, `MathMLElement includes GlobalEventHandlers`. So the set is the mixin's members, partial extensions included: "supported by all HTMLElements" alone would read as list (i), and "as defined in the MathMLElement IDL" fixes it to the mixin, which holds list (ii)'s six and the extensions' names too. HTML §8.1.8.1 attribute change steps step 1: "If namespace is not null, or localName is not the name of an event handler content attribute on element, then return." |
| H-A3 | The readers of "the handler content attributes of this element" | `git grep -n 'event_handler_attr_lookup(' cee74989 -- crates` | the definition `session/event_handler_processing/mod.rs:237` (`pub(crate)`) and four calls: the processing function's snapshot (`mod.rs:97`, in `process_handler_content_attributes`, `:76`), the D-8 restriction's count (`mod.rs:206`, in `is_only_deliverable_ordinary_name`, `:196`), the AttributeChange arm (`session/event_handler_consumer.rs:405`, in `handle_attribute_change`, `:397`) and `is_window_delegating_pair` (`mod.rs:219`), which reads the name's scope only and asks nothing about the element's attributes. None of the first three asks whether the name is one of the element's: every row of the table installs on every element. The consumer file imports from the processing module at `event_handler_consumer.rs:50-53` (`use crate::event_handler_processing::{…}`) |
| H-A5 | The namespace read that exists | `git show cee74989:crates/core/elidex-ecs/src/dom/classify.rs \| sed -n 67,138p` | `EcsDom::namespace_of` (`ecs/dom/classify.rs:76`: the `Namespace` component, absence = `Html`); `EcsDom::is_html_namespace` (`:136`: an element whose namespace is `Html`) |
| H-A6 | How a foreign element comes to exist | `git grep -n -i 'createElementNS' cee74989 -- crates`; `git grep -n 'create_element_ns(' cee74989 -- crates ':!*tests*'`; `git show cee74989:crates/script/elidex-js/src/vm/host/dom_parser.rs \| sed -n 45,56p` and `sed -n 252,262p` | No `createElementNS` in the VM (two comment hits; slot `#11-vm-create-element-ns-missing`, owner L3). Production callers of `EcsDom::create_element_ns` (`ecs/dom/mod.rs:435`): the strict parser's foreign-content path (`crates/dom/elidex-html-parser-strict/src/tree_builder/insert.rs:116`) and the tolerant parser's tree conversion (`crates/dom/elidex-html-parser/src/convert.rs:258`); `crates/dom/elidex-form/src/init.rs:264`, `:278` are tests. DOMParser HTML-parses every accepted MIME type (`#11-domparser-xml-real-parsing`). So a foreign element is created by the HTML parser's foreign content only, or by a clone of one |
| H-A7 | The HTML parser cannot create a foreign `body` | `.claude/tools/webref body html parsing-main-inforeign`; `git show cee74989:crates/dom/elidex-html-parser-strict/src/tree_builder/modes/foreign.rs \| sed -n 162,213p`; the tolerant half, for `v` in `0.39.0` (the `Cargo.lock` of `cee74989`) and `0.40.1` (of `653d7c24`, #636): `sed -n 1626p ~/.cargo/registry/src/*/html5ever-$v/src/tree_builder/rules.rs` and `/usr/bin/grep -n -A8 'fn unexpected_start_tag_in_foreign_content' ~/.cargo/registry/src/*/html5ever-$v/src/tree_builder/mod.rs` | HTML §13.2.6.5 The rules for parsing tokens in foreign content: a start tag named `body` (one of the listed names, "b", "big", "blockquote", "body", …) pops out of foreign content and is reprocessed as HTML content; `frameset` is not in the list. The strict parser has the same list (`BREAKOUT_TAGS`, `foreign.rs:165`, read at `:218`; strict rejects). The tolerant backend, html5ever, recovers as the spec says, in both versions: the foreign-content rule's start-tag list holds `<body>` (`tree_builder/rules.rs:1626`, the same line in 0.39.0 and 0.40.1) and calls `unexpected_start_tag_in_foreign_content` (`tree_builder/mod.rs:1882` in 0.39.0, `:1874` in 0.40.1), which pops to an HTML element or an integration point and reprocesses the tag in the current insertion mode. With H-A6: **no script can create an SVG or MathML `body` at the base**; an SVG or MathML `frameset`, `mi`, `rect` … is created by `innerHTML` and the document parse |
| H-A8 | (T3 rev 5's H-T19, restated: R3 wave 1 W1-3) No attribute namespace is modelled, so the namespace half of step 1 has no case at the base | `git show cee74989:crates/core/elidex-ecs/src/components.rs \| sed -n 86,91p`; `git show cee74989:crates/core/elidex-ecs/src/dom/mutation_event.rs \| sed -n 366,371p`; `git grep -n -i 'setAttributeNS' cee74989 -- crates`; `.claude/tools/webref body html adjust-foreign-attributes` | `Attributes` is `map: IndexMap<String, String>` (`ecs/components.rs:89-91`): a name and a value, no attribute namespace. `AttributeChange { node, name, old_value, new_value }` carries none. No `setAttributeNS` exists in the VM (two doc-comment hits, `crates/dom/elidex-dom-api/src/element/attrs.rs:321` and `tests_attrs.rs:333`). The parser's only namespaced attributes are those of *adjust foreign attributes* (HTML §13.2.6.1 Creating and inserting nodes: `xlink:*`, `xml:lang`, `xml:space`, `xmlns`, `xmlns:xlink`), none of whose local names is a handler name. So no attribute with a non-null namespace can exist: step 1's "namespace is not null" is false for every attribute elidex can represent, and the namespace half is **unreachable at the base**, not a reachable defect |
| H-A9 | (Q-10 for this slice) T3-a's planned code reads no parent and no child link | the reads of §4.2: `TagType`, `Namespace` (through `namespace_of`), `Attributes`, `HandlerAttrsProcessed`; `for f in <the two source files of §5.3>; do git show cee74989:$f \| /usr/bin/grep -c -E 'get_parent\|children_iter\|first_child\|last_child\|next_sibling\|prev_sibling\|traverse_descendants\|TreeRelation'; done` | the predicate reads components of the one entity it is given. `event_handler_processing/mod.rs`: 0. `event_handler_consumer.rs`: 2 lines, both `handle_insert`'s `traverse_descendants` (`:439` its doc, `:446` the call), which T3-a does not edit. So T3-a uses neither `tree_parent` nor `shadow_including_parent` of L3's tree-model program and no child iterator. **The new test files too (rev 1.1):** L3's slice `ta` deletes `EcsDom::get_parent`, tests included (the tree-model umbrella draft 2, `git show "1593a5ad:docs/plans/2026-11-tree-model-shadow-root-is-a-root-umbrella.md" \| sed -n 93p`, and its `ta` row, `:339`), so a `get_parent` call in a T3-a test would not compile once `ta` lands. The planned cells (§6) need none: they build elements, insert them, write attributes and read `EventListeners` or script values. The grep over those files is in DoD item 4 |
| H-A10 | The cited files on main | `git diff --stat cee74989 origin/main -- crates/script/elidex-script-session/src crates/core/elidex-ecs/src/dom/classify.rs crates/core/elidex-ecs/src/components.rs crates/script/elidex-js/src/vm/test_helpers.rs` | empty at `653d7c24` (#636) and at `origin/main` `1b176ea9` (verified 2026-11-18; rev 1: at `44a2d8cb`) |

### §0.3 Premise errors (reported)

- **P-A1 (T3 rev 5's D-T8).** D-T8 named three scopes ("the GlobalEventHandlers names, plus the WindowEventHandlers
  names on `body` / `frameset` only, and no Document-only name"); `HandlerScope` has four (H-A1). The fourth,
  `DocumentElement` (`oncopy`, `oncut`, `onpaste`), is in HTML's list (i) and in `GlobalEventHandlers` (H-A2), so it
  is in class G. Its enum doc ("`DocumentAndElementEventHandlers` — Document + Element") names a mixin the HTML IDL no
  longer has; the doc and what the VM installs for it are not T3-a's (§10 Q-A4, answered and slotted).
- **P-A2 (T3 rev 5's cells T-9, T-9b, T-21).** Their scripts call `document.createElementNS`, which the VM does not
  have (H-A6), and no other script path creates a foreign `body` (H-A7). The cells cannot run as written. T-21 (here)
  takes its element from the harness (§6.1); T-19 is the T3-b memo's and T-9, T-9b stay T3's, with the same harness
  elements.
- **P-A3 (T3 rev 5's rows FN-1b, FN-4, FN-5 and U-T8's "null-namespace `body`").** "Outside the HTML, SVG and MathML
  namespaces" has no elidex representation: `Namespace` has three variants and absence means `Html` (H-A5). The spec's
  "none" for such an element (H-A2: the three specs define event handler content attributes for their own namespaces
  only) is not a branch T3-a can write or test. The predicate's `match` on `Namespace` has no wildcard, so a fourth
  variant does not compile until its arm is decided (§4.2).
- **P-A4 (T3 rev 5's P-T14, kept).** T3 rev 3's FN-1 gave "element" for such an element; the spec gives "nothing".
  With P-A3 the row has no elidex case; it is recorded in the slot text (§8.3 item 1).
- **P-A5 (T3 rev 5's residual R-D2, reach).** R-D2 is about a MathML `body`, which no script can create (H-A7). It is
  a defect of fixture-built elements today and becomes reachable when `createElementNS` is wired
  (`#11-vm-create-element-ns-missing`). The script-reachable part of D-T8 is the rest of its table: class W and D
  names on HTML elements other than `body` / `frameset`, class D names on `body` / `frameset`, class W and D names
  on MathML elements other than `body` (§2.1).
- **P-A6 (the T3-ab memo rev 1.1 §8.3 item 1, facet (2); R3 wave 1 W1-3).** The slot text said that
  "`setAttributeNS(ns, 'onclick', X)` with a non-null `ns` installs one" and that facet (2) "closes at slice b, which
  adds the namespace to the event". Neither holds: no writer of a namespaced attribute exists and `Attributes` has no
  namespace to carry (H-A8), so nothing is installed that step 1 would refuse, and a namespace field on the event
  alone would have no value to hold. Restated in §8.3 item 1; the umbrella's and the T3 memo's wordings of the same
  premise point there. PM decision 770 draws the consequence: the facet's owner is the slot that models attribute
  namespaces, `#11-attribute-namespace-model` (L3), and PM decision 168's slice-b premise is revised.

### §0.4 Decisions (this PR)

- **D-T8 — applicability: the local-name half of the attribute change steps' step 1 (PM decisions 168, 741 (3)).**
  `pub(crate) fn is_event_handler_content_attribute_of(dom, element, name) -> bool` (NEW), in
  `session/event_handler_processing/mod.rs`, crate-visible as `event_handler_attr_lookup` beside it is (`mod.rs:237`)
  because reader (c) is in the sibling module `event_handler_consumer.rs`, is the one statement of which rows of
  `EVENT_HANDLER_ATTRS` are event handler content attributes of `element`:

  | element | G (85) | W (18) | D (2) | ground |
  |---|---|---|---|---|
  | HTML `body`, `frameset` | yes | yes | no | G: HTML §8.1.8.2 lists (i), (ii), and for the names the partial mixins add, the partial mixins of Pointer Events §5 and CSS Transitions §6.3 (the latter's words: "must be supported by all HTML elements, as both event handler content attributes and event handler IDL attributes"); W, D: HTML §8.1.8.2 lists (iii), (iv) |
  | other HTML | yes | no | no | the same |
  | SVG | yes | yes | yes | SVG2 §15.8 (every supported event type, on all SVG elements) |
  | MathML | yes | no | no | MathML Core §2.1.4 ("as defined in the MathMLElement IDL") with §2.2.3 (`MathMLElement includes GlobalEventHandlers`: the mixin's members, partial extensions included) |

  G is read from the table's `HandlerScope` (`Global`, `DocumentElement`), not from a list: the predicate answers for
  the rows the table has, and which members of the mixin the table holds is the table's question (H-A2; §7).

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
  affected: their surface is the prototype's (for `body`, the T3-b memo D-T11). Facet (2), a non-null attribute
  namespace, is unreachable at the base (H-A8) and is `#11-attribute-namespace-model`'s (PM decision 770; §8.3 item 1). Every outcome the predicate changes is toward the spec
  (§2.1).
  *The SVG row and `SVGSVGElement includes WindowEventHandlers`* (R3 wave 1, Axis 4 note; SVG2 §5.13.2 Interface
  SVGSVGElement): no change. The mixin gives the `svg` element's interface IDL attributes; the predicate answers for
  content attributes, and for those the row is already "yes" for every name on every SVG element, `svg` included, by
  §15.8. That the VM exposes no SVG interface is the T3-b memo §8.2 item 1's defect.
  *Rejected:* a test at the two event-facing sites only (rev 5's text): three readers, two answers (one issue, one
  way). A lookup that takes the element and replaces `event_handler_attr_lookup`: `is_window_delegating_pair` and the
  VM's install loop ask about the name alone, so the name-only lookup stays, and one function answering both
  questions would hide which is asked.

## §1. Shape and verdict

| Axis | Own invariant |
|---|---|
| B1 name class | G, W, D by `HandlerScope` (§0.1) |
| B2 element class | `Namespace` × (`body` / `frameset` or not), immutable per element |
| B3 readers of an element's handler content attributes | three, one predicate (D-T8 (a), (b), (c)) |

Coupled from outside, all on the base and none changed: B5 the once rule and its mark (umbrella K-1, I1); B6 the
policy filter (I6); B7 the D-8 restriction (umbrella D-8); B8 X-W's classifier `is_window_delegating_pair`, tag-only
until T3's D-T5; B9 HEAD's target rule `resolve_handler_target`, until T3. (B4, `body`'s interface, and B10, the
wrapper's prototype chosen once, are the T3-b memo's axes; T3-a does not meet them.)

### §1.1 Outcome changes at T3-a (toward the spec)

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
- A class W name on a MathML `body` installs nothing (base: on the Window, re-raw'd at every insertion), and its
  removal clears nothing (base: clears the Window's handler). Row FN-4; cells T-21, U-T8.

### §1.2 Is T3-a no worse than its base, and is T3 no worse landing after it?

Criterion: the umbrella's K-0 with T3-a's base as "HEAD": at every observable step T3-a's outcome equals the
base's, or the spec's where the base's does not. §2.1 enumerates element class × name class × reader. **Rows worse
than the base: 0.** Four kinds of row stay base-equal and off the spec, none changed by T3-a: an HTML `frameset`'s
and `body`'s Window-delegating pairs that HEAD targets at the element (T3 memo EW rows), an SVG `body`'s class W
content attribute, delivered to the Window by the tag-only target rule (T3 memo FN-2), the foreign half of X-W's
classifier (B8), each T3's; and the IDL side of a foreign `body` (FN-3), the T3-b memo's. The foreign half of B8 is
a timing, not a target: the tag-only classifier takes a foreign `body` or `frameset` × a Window-delegating name for
an X-W pair, so where the pair is still the element's event handler content attribute (an SVG `frameset` or `body` ×
class W and the reflecting six; a MathML `frameset` or `body` × the reflecting six) it is re-raw'd at every
insertion and not processed at the first handler write, where the spec has the ordinary once rule (T3 memo FN-6;
T3's D-T5 ends it). A foreign `frameset` is script-reachable (`innerHTML`, H-A7), so this row is too (rev 1.1: rev 1
named the foreign `body` only).

T3-a and T3-b are unordered: the base may or may not have T3-b, and no row of §2.1 reads a T3-b outcome. On a
MathML `body` × class W, T3-a decides the content attribute (FN-4) and T3-b the IDL accessor (FN-3); with only one
landed, one side is on the spec and the other base-equal, which is not worse than both off it, and no script can
build the element (H-A7).

The other direction (T3 on a base that has T3-a): T3 keeps its own no-worse analysis (T3 memo §1.2, §2.1); the rows
T3-a closes are equal on T3's base and after T3 (T3 memo §2.1, marked ‡). The two can land in this order only: T3
before T3-a would leave R-D2 open at T3's landing, which PM decision 153's strict reading does not allow, and T3's
namespaced step 1 without D-T8 would put row FN-4 on neither the base nor the spec (T3 memo §1.3, seam S1).

### §1.3 Edge density

*Verdict:* plan-review is mandatory (PM decisions 741 (3), 767: this memo's R1, five axes), whichever way the count
falls. Three own axes meet in one table (B1 × B2) read at three sites (B3), and five external ones bound it (B5 to
B9). The table is the spec's, fully enumerated by U-T8; no canonical algorithm is missing.

Breadth (all §3 rows): `.claude/tools/webref coverage-map html 8.1.8.1 html 8.1.8.2 SVG2 15.8 mathml-core 2.1.4` →
"**Breadth (requested)**: spec=3 (SVG2, html, mathml-core), step=4 entries", "**Split decision**: ok → single PR
scope" (output of 2026-11-18; K = 3, M = 4).

**The split that produced this memo (PM decision 767).** The T3-ab memo held D-T8 and D-T11 in one slice; R3 wave 1's
Axis 3 ruled them two (W1-1): they share no code (the session's predicate; the VM's `body` test) and no cell, and
neither needs the other (§1.2). T3-a is the D-T8 half: one intersection, B1 × B2 read through B3.

**A split inside D-T8** (two readers now, the D-8 count later; or HTML now, MathML later) would leave two answers to
one question on main between the parts, in either order (D-T8 *Rejected*).

T3-a is a narrowly-scoped per-PR slice under the approved EH-attrs umbrella (§1.2 row T3-a): three own axes, so the
rule's trigger (a) fires, and the recursion ends at its base case (c).
External couplings: slice a, N1, C0, C1 (landed); T3 (x15, after). Decisions: PM 153, 168, 741 (3), 767.

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
  `process_before_handler_write` with the receiver and an accessor's name; which accessors a foreign `body` has is
  the T3-b memo's (D-T11), and in either landing order the tag-only classifier returns early there for a
  Window-delegating pair, as on the base (T3's D-T5 ends it).
- **B3×B9** — the predicate returns before target determination at (a) and (c): step 1 before step 2, the spec's
  order. T3 replaces the target rule after it and does not move it (umbrella §4.1).

Pairs not listed do not intersect: B5 to B9 among themselves are the base's.

### §2.1 Enumeration: element class × name class (Base / T3-a / spec)

"Installs" = the content attribute, at any timing (bind, insertion, first write, `setAttribute` / `removeAttribute`),
writes or clears a handler; the target is HEAD's (`resolve_handler_target`), unchanged here. Verdict as in the T3
memo §2.1: **both**, **spec** (T3-a = spec ≠ base), **base** (T3-a = base ≠ spec). *Reach*: **S** a script can
build it; **F** harness-built only (H-A7).

| Row | Element | Names | Reach | Base | T3-a | Spec | Verdict | Cells |
|---|---|---|---|---|---|---|---|---|
| AP-1 | HTML `body`, `frameset` | G, W | S | installs | installs | installs (the target is T3's) | both (applicability) | U-T8 |
| AP-2 | HTML `body`, `frameset` | D | S | installs on the element | nothing | nothing | spec | U-T8 |
| AP-3 | other HTML | W | S | installs on the element | nothing | nothing | spec | T-A1, U-T8 |
| AP-4 | other HTML | D | S | installs on the element | nothing | nothing | spec | T-A1, U-T8 |
| AP-5 | SVG (any local name) | G, W, D | S (`body`: F) | installs (an SVG `body` × W: on the Window, T3 memo FN-2). Timing: once; an SVG `frameset` or `body` × W and the reflecting six at X-W's timing (every insertion, never the first write; the tag-only classifier, B8) | installs, same target, same timing | installs on the element, once | both; base for an SVG `body` × W's target and for the X-W timing of an SVG `frameset`'s and `body`'s Window-delegating names (T3: FN-2, FN-6) | T-A3, U-T8 |
| AP-6 | MathML other than `body`, `frameset` | W, D | S | installs on the element | nothing | nothing | spec | T-A2, U-T8 |
| FN-5 | MathML `frameset` | W, D | S | installs on the element | nothing | nothing | spec | U-T8 |
| FN-4 | MathML `body` | W | F | installs on the Window (tag-only target), re-raw'd at every insertion | nothing | nothing | spec (R-D2) | T-21, U-T8 |
| AP-9 | MathML `body` | D | F | installs on the element | nothing | nothing | spec | U-T8 |
| AP-10 | MathML (any local name) | G | S (`body`: F) | installs on the element. Timing: once; a MathML `frameset` or `body` × the reflecting six at X-W's timing (B8) | installs, same timing | installs, once | both; base for the X-W timing of a MathML `frameset`'s and `body`'s reflecting six (T3: FN-6) | T-A2, U-T8 |
| AP-7 | any element whose handler-named attributes are all rows AP-2 to AP-4, AP-6, FN-4, FN-5 or AP-9 | — (the mark) | S | marked at its first processing | not marked | no such state | both (not observable, B3×B5) | U-A2 |
| AP-8 | an unmarked element with exactly one deliverable ordinary attribute of its own and at least one handler-named attribute that is not its own; a handler write before its first insertion | — (D-8) | S | not processed at the write; the first insertion overwrites the write | processed at the write; the write stays | the write stays | spec | T-A4, U-A1 |
| OR-1a | every (element, name) pair, the attribute removed (`removeAttribute`: `old_value` some, `new_value` none) | G, W, D | S (a foreign `body`: F) | clears the handler on HEAD's target, for every pair | clears where the row above installs; nothing where it gives "nothing" (a handler set by other means stays) | the same | both where the row installs; spec elsewhere | U-T8 (iii) |

FN-3 (a foreign `body`'s interface) is the T3-b memo's row. FN-1b (an element outside the three namespaces) has no
elidex case (P-A3). The attribute-namespace half of step 1 has no case either (H-A8).

## §3. Spec coverage map

Skeleton: `.claude/tools/webref coverage-map html 8.1.8.1 html 8.1.8.2 SVG2 15.8 mathml-core 2.1.4` (§1.3).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.8.1 Event handlers | attribute change steps step 1, local-name half | localName not an event handler content attribute of element → return; the namespace half has no case at the base (H-A8) | `is_event_handler_content_attribute_of` (NEW) in `crates/script/elidex-script-session/src/event_handler_processing/mod.rs`, read at `:97`, `:206` and at `crates/script/elidex-script-session/src/event_handler_consumer.rs:405` | ✗ | yes |
| WHATWG HTML §8.1.8.2 Event handlers on elements, Document objects, and Window objects | the four lists of event handlers | lists (i), (ii): all HTML elements (G; the table's 14 G rows that partial mixins add to `GlobalEventHandlers` are members of the same mixin, footnote); list (iii): body and frameset only (W); list (iv): Document objects only, no content attribute (D) | the `Html` arm of the predicate, over `HandlerScope` (`crates/script/elidex-script-session/src/event_handler_consumer.rs:63`) | ✓ | yes |
| SVG2 §15.8 Event attributes | every supported event type is an event attribute of every SVG element | SVG element → every row | the `Svg` arm of the predicate | ✓ | yes |
| MathML Core §2.1.4 Attributes common to HTML and MathML elements | the event handler content attributes of the `MathMLElement` IDL (`MathMLElement includes GlobalEventHandlers`, MathML Core §2.2.3 DOM and JavaScript) | MathML element → G only | the `MathMl` arm of the predicate | ✓ | yes |

Numbers and titles (2026-11-18): `webref heading html 8.1.8.1` (Event handlers, `#event-handler-attributes`),
`8.1.8.2` (Event handlers on elements, Document objects, and Window objects), `8.1.8.2.1` (IDL definitions),
`13.2.6.5` (The rules for parsing tokens in foreign content, `#parsing-main-inforeign`); `webref dfn html 'adjust
foreign attributes'` → §13.2.6.1 Creating and inserting nodes; `webref heading SVG2 15.8` (Event attributes,
`#EventAttributes`); `webref body SVG2 InterfaceSVGSVGElement` (its heading: 5.13.2. Interface SVGSVGElement; the
line `SVGSVGElement includes WindowEventHandlers`); `webref heading mathml-core 2.1.4` (Attributes common to HTML and
MathML elements), `2.2.3` (DOM and JavaScript, `#dom-and-javascript`). Rev 1.1, for class G's ground (the sections
that extend the mixin; T3-a touches no site of theirs, so they are cited here and are not rows): `webref heading
pointerevents4 5` (Extensions to the GlobalEventHandlers mixin, `#extensions-to-the-globaleventhandlers-mixin`; the
same number and title under `pointerevents3`, which publishes no IDL extract), `webref idl pointerevents4
GlobalEventHandlers` (11 attributes; the table has 10, not `onpointerrawupdate`); `webref heading css-transitions-1
6.3` (Event handlers on elements, Document objects, and Window objects), `7.1` (IDL Definition,
`#interface-globaleventhandlers-idl`), `webref idl css-transitions-1 GlobalEventHandlers` (4 attributes); the other
partial mixins H-A2 names: `webref idl css-animations-1 GlobalEventHandlers`, `webref idl selection-api
GlobalEventHandlers`, `webref idl touch-events GlobalEventHandlers`. Step and list texts: `webref body html event-handler-attributes` (attribute change steps step 1),
`event-handlers-on-elements,-document-objects,-and-window-objects` (the four lists), `parsing-main-inforeign` (the
breakout start tags), `adjust-foreign-attributes`; `webref body SVG2 EventAttributes`; `webref body mathml-core
attributes-common-to-html-and-mathml-elements`; `webref idl html GlobalEventHandlers` (76 attributes, with `oncopy`, `oncut`,
`onpaste`), `webref body mathml-core dom-and-javascript` (the `MathMLElement` IDL and its `includes` lines), `webref idl html DocumentAndElementEventHandlers` (no such block), `webref idl html WindowEventHandlers`
(18 attributes), `webref idl mathml-core MathMLElement`.

### §3.1 User-input touch audit

Author-controlled inputs reaching the changed code: handler-named content attributes on any element (parser-baked,
`setAttribute`, `removeAttribute`), IDL handler writes on any element (through the D-8 entry), and foreign content in
markup (`<svg>`, `<math>`, through `innerHTML` and the document parse). Every change removes an effect: a name that
is not the element's event handler content attribute installs and clears nothing. No handler is delivered anywhere
it was not delivered on the base, no target changes, and no security boundary (CSP, sandbox, the policy filter) is
touched: a name removed by the predicate would have been installed or policy-excluded on the base, never the reverse.

## §4. Mechanism

### §4.1 Home / kind

| Fact | Home / kind | Status |
|---|---|---|
| applicability (attribute change steps' step 1, local name) | `is_event_handler_content_attribute_of` (NEW), `pub(crate)` free fn, `session/event_handler_processing/mod.rs`; a pure read of the element's `Namespace` and `TagType` components and of the name's `HandlerScope` row | D-T8 |
| its three readers | the snapshot of `process_handler_content_attributes`, the filter of `is_only_deliverable_ordinary_name`, the head of `handle_attribute_change` (which imports the predicate) | D-T8 (a), (b), (c) |

No component, store, trait, event variant, crate edge or `pub` item is added (the predicate is `pub(crate)`), and
nothing is deleted.

### §4.2 Shape

The two functions T3 rewrites after this PR are `process_handler_content_attributes` and `handle_attribute_change`.
Their final shape (signature and branch structure, with the owner of each part) is decided once, in the umbrella's
§4.1; T3-a writes its parts in that shape and changes neither signature.

```rust
// session/event_handler_processing/mod.rs
/// (NEW) D-T8. No wildcard: a new `Namespace` or `HandlerScope` does not compile until it is placed.
pub(crate) fn is_event_handler_content_attribute_of(dom: &EcsDom, element: Entity, name: &str) -> bool {
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

// (c) session/event_handler_consumer.rs: `is_event_handler_content_attribute_of` joins the
//     `use crate::event_handler_processing::{…}` list (`:50-53`); in handle_attribute_change, after the lookup's
//     `let … else { return }`
if !is_event_handler_content_attribute_of(dom, node, name) { return; }                 // (NEW), before the policy filter
```

Re-runnable after T3-a: `git grep -n 'is_event_handler_content_attribute_of(' -- crates ':!*tests*'` → the
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
object-oriented engines) → one predicate over the namespace and tag components. Applicability is a function of
immutable facts, so it is not cached in a component (nothing to invalidate; the side-store rule has no state to
home). Layering: the table and its three readers live in `elidex-script-session`; `vm/host/` is not edited.

### §4.4 Docstring citation table (Axis 4)

| Item | Cites |
|---|---|
| `is_event_handler_content_attribute_of` (NEW) | HTML §8.1.8.1 Event handlers, attribute change steps step 1 (local-name half); HTML §8.1.8.2 (lists (i) and (ii): all HTML elements; list (iii): body and frameset; list (iv): Document objects); for `HandlerScope::Global` rows that are not in HTML's lists, the partial `GlobalEventHandlers` mixins of Pointer Events §5 Extensions to the GlobalEventHandlers mixin and CSS Transitions §6.3 Event handlers on elements, Document objects, and Window objects; SVG2 §15.8 Event attributes; MathML Core §2.1.4 ("as defined in the MathMLElement IDL") and §2.2.3 DOM and JavaScript (`MathMLElement includes GlobalEventHandlers`); **Not realised:** step 1's namespace half (no attribute namespace is modelled) → `#11-attribute-namespace-model` (PM decision 770) |
| `event_handler_attr_lookup` | its docstring's "a known event-handler content attribute" (`mod.rs:234`) reads "a name in the event handler table (`EVENT_HANDLER_ATTRS`)": the lookup answers for the name alone; whether the name is an event handler content attribute *of an element* is `is_event_handler_content_attribute_of`'s |
| `process_handler_content_attributes` | its docstring's "attribute change steps 1–5.6" (`mod.rs:59-62`) gains "step 1's local-name half by `is_event_handler_content_attribute_of`", and its "Not realised" paragraph (`mod.rs:64-68`) gains step 1's namespace half ("no attribute namespace is modelled; `#11-attribute-namespace-model`"), so "1–5.6" no longer claims the half that is not there (R1 Axis 4); "`element` is marked iff it carries at least one ordinary handler content attribute" reads "of its own" |
| `is_only_deliverable_ordinary_name` | "handler content attribute of `element`" is the predicate's |
| `handle_attribute_change` | HTML §8.1.8.1 attribute change steps step 1, local-name half, before the A2 policy comment; the namespace half named as not realised, with the same slot |
| `HandlerScope::DocumentElement`'s doc | not edited (P-A1, §10 Q-A4) |

## §5. Populations, touched files, named risks

### §5.1 Behavioural population (re-runnable)

The predicate's domain is every row of `EVENT_HANDLER_ATTRS` (105, H-A1) × element class {HTML `body`, HTML
`frameset`, other HTML, SVG, MathML}. The pairs whose outcome changes are those the table answers "no": HTML `body` /
`frameset` × D (2 names each), other HTML × W and D (20 names), MathML × W and D (20 names). U-T8 derives its table
from `EVENT_HANDLER_ATTRS` and the scopes, not from a sample. Class G is the table's 85 rows, whatever their spec of
origin (H-A2: 71 from HTML's lists, 14 from two partial mixins); the names the mixin has and the table lacks are not
in the population, because no handler exists for them on any element today (§7).

### §5.2 Existing tests whose expectation changes (rewritten in the same commit)

Measured (verified 2026-11-15): `git grep -n -w -E '(<the 18 W names>|onreadystatechange|onvisibilitychange)'
cee74989 -- crates`, each hit that sets one as a content attribute read for its element; every other hit sets it on
a `body` or a `frameset`, or is a table, an IDL write or a non-DOM object's handler.

| Test | Base | T3-a | Why |
|---|---|---|---|
| `u3_i_excluded_ordinary_pair_not_wired` (`session/event_handler_processing/tests_insertion_and_write.rs:76`) | a `div` × `onstorage` under the core policy: never wired | the same assertions on an SVG `rect` × `onstorage` | `onstorage` is not a `div`'s event handler content attribute, so on a `div` the cell would pass without reaching the policy filter. The one policy-excluded name is class W, so an excluded *ordinary* pair exists on an SVG element only (B3×B6) |
| `u3_i_excluded_ordinary_pair_marks_element` (`:87`) | the `div` is marked after the first append | the SVG `rect` is marked | as above; on a `div` the element is no longer marked (AP-7) |
| `onstorage_content_attribute_gated_by_policy` (`session/event_handler_consumer.rs:507`) | a `div` × `onstorage` through the AttributeChange arm: wired under BrowserCompat, not under BrowserCore / App | the same on a `body` | its positive control would fail on a `div` (AP-3); on a `body` the pair is applicable and, with no Window in that bare `EcsDom`, lands on the element as before (`resolve_handler_target`'s fallback) |

**At T3** these three, U-T8 and every caller of the session harness are rewritten again (the target function and
the `active` parameter); the list and T3's expectations are the T3 memo §5.2's (§8.4). In particular the `body` of
`onstorage_content_attribute_gated_by_policy` is ownerless, and its positive control passes here only through
`resolve_handler_target`'s no-Window fallback, which T3 deletes (R1 Axis 3).

U-A3 (§6.2) adds the cell that the first two no longer are: a `div` × `onstorage` is neither wired nor marked under
either policy. Kept green and checked: slice a's cells (`tests_event_handler_attrs_once.rs`,
`tests_event_handler_attrs_window_delegating.rs`: every W name there is on a `body` or a `frameset`), N1's cells, the
shell E1–E8. Any other existing test that installs a handler through a non-applicable attribute is found by the full
run at the base and listed in the PR (§9 item 1).

### §5.3 Touched files

Sizes at `origin/main` `1b176ea9` (`git show origin/main:<path> | wc -l`, re-verified 2026-11-18 after #636; equal to `cee74989`'s).

- `crates/script/elidex-script-session/src/event_handler_processing/mod.rs` (410) — the predicate; readers (a), (b);
  `use elidex_ecs::Namespace`; `test_support` gains a foreign-element builder beside `elem`.
- `crates/script/elidex-script-session/src/event_handler_consumer.rs` (545) — reader (c); the predicate added to its
  `use crate::event_handler_processing::{…}` list; the rewritten policy test.
- tests: `session/event_handler_processing/tests_applicability.rs` (NEW; U-T8, U-A1 to U-A3),
  `crates/script/elidex-script-session/src/event_handler_processing/tests_insertion_and_write.rs` (346; the two U3 (i)
  cells), `elidex-js/vm/tests/tests_event_handler_applicability.rs` (NEW; T-21, T-A1 to T-A4),
  `crates/script/elidex-js/src/vm/test_helpers.rs` (326; the helper that builds the foreign `body` elements, §6.1, if
  T3-b has not added it). No new or edited test calls `EcsDom::get_parent` (H-A9; DoD item 4).
- docs: none beyond this plan PR (the umbrella and the T3 memo are edited by the plan PR, not by the implementation).

No file is over 1000 lines, so the touch-time split check does not fire. Not touched: `crates/core/elidex-ecs` (the
namespace readers exist); any production file of `crates/script/elidex-js` (`vm/host/` is the T3-b memo's).

### §5.4 Named risks

- **R-A1 — merge contact, no semantic contact.** L3's PR (io) edits `event_handler_processing/mod.rs`'s import line and
  `process_bound_document`'s walker call (`TemplateContentsWalk` → `WalkScope`); L3's tree-model slice `ta` **deletes**
  `EcsDom::get_parent`, tests included (until rev 1.1: "re-types `get_parent` callers"), and neither T3-a's two
  source files nor its new and edited test files call it (H-A9; DoD item 4), so `ta` and T3-a meet in no line in
  either landing order; T3-pre does not edit the event-handler
  consumer or the processing module (the T3-pre memo §5.3, *Not touched*); T3-b edits neither (the T3-b memo §5.3).
  T3-a's lines are the snapshot, the D-8 filter and the arm's head: a textual merge with io at most. With T3-b the one
  shared file is the test helper file (§6.1).
- **R-A2 — T3 rewrites the two functions after T3-a.** Bounded by the umbrella's §4.1: T3 adds its parts around
  T3-a's three lines and does not move them. **Stop condition (PM decision 741 (3)):** if T3's implementation finds
  it must move, re-order or re-write one of T3-a's three reader sites, or change the predicate's signature, it stops
  for the PM (a mis-drawn slice).
- **R-A3 — a test or page that relied on a non-applicable name.** In-tree: three tests (§5.2). For content: a handler
  on such an attribute never ran on a spec-conforming engine, so no page can depend on it by the spec; no browser
  measurement is taken or needed (§10).

## §6. Test plan

Red-first: each **red** cell is committed failing on T3-a's base before the fix commit.

### §6.1 VM integration (`tests_event_handler_applicability.rs`, NEW)

Every cell uses `with_page` / `run` (`elidex-js/vm/test_helpers.rs:168`, `:193`; `el` a connected element, `fn` counts
its calls in `fnRan`). **Foreign `body` elements are built by the harness** (P-A2): a helper beside `with_page` in
`elidex-js/vm/test_helpers.rs` (NEW helper; in the shared file because the T3-b memo's cell T-19 and T3's cells T-9
and T-9b use it too) creates, before the script, an SVG `body` and a MathML `body` with `EcsDom::create_element_ns`
owned by the bound document, and exposes their wrappers as the globals `svgBody` and `mathBody` (the precedent:
`setup_with_root_and_svg`, `elidex-js/vm/tests/tests_mutation_observer/mod.rs:136`, which states why: "JS
`createElementNS` is not VM-wired"). The wrappers go through the production `create_element_wrapper`. **The helper is
written once, in this shape, by whichever of T3-a and T3-b lands first; the other finds it on its base** (test
support only; this is not an edge, since either PR can write it).

| Cell | Kind | Script | Base | T3-a (= spec) |
|---|---|---|---|---|
| T-21 | **red (R-D2; FN-4; D-T8)** | `mathBody.setAttribute('onhashchange','window.hit = 1'); mathBody.dispatchEvent(new Event('hashchange')); r.push(window.hit === undefined, window.onhashchange === null)` | `true,false` (on the Window) | `true,true` |
| T-A1 | red (AP-3, AP-4; script-reachable) | `var d = document.createElement('div'); d.setAttribute('onhashchange','window.h = 1'); d.setAttribute('onreadystatechange','window.s = 1'); d.setAttribute('onclick','window.c = 1'); for (const t of ['hashchange','readystatechange','click']) d.dispatchEvent(new Event(t)); r.push(window.h === undefined, window.s === undefined, window.c === 1)` | `false,false,true` | `true,true,true` |
| T-A2 | red (AP-6, AP-10; a parser-built MathML element; static; confirmed red-first at implementation) | `el.innerHTML = '<math><mi onhashchange="window.h = 1" onclick="window.c = 1"></mi></math>'; var mi = el.firstChild.firstChild; mi.dispatchEvent(new Event('hashchange')); mi.dispatchEvent(new Event('click')); r.push(window.h === undefined, window.c === 1)` | `false,true` | `true,true` |
| T-A3 | guard (AP-5; a parser-built SVG element keeps every name; static; confirmed at implementation) | `el.innerHTML = '<svg><rect onhashchange="window.h = 1" onreadystatechange="window.s = 1"></rect></svg>'; var rc = el.firstChild.firstChild; rc.dispatchEvent(new Event('hashchange')); rc.dispatchEvent(new Event('readystatechange')); r.push(window.h === 1, window.s === 1)` | `true,true` | `true,true` (SVG treated as HTML or MathML: `false,false`) |
| T-A4 | red (AP-8; D-T8 (b)) | `el.innerHTML = '<div onclick="window.c = 1" onhashchange="window.h = 1"></div>'; var c = el.firstChild.cloneNode(); c.onclick = fn; el.append(c); c.dispatchEvent(new Event('click')); r.push(fnRan === 1, window.c === undefined)` | `false,false` (two deliverable names: not processed at the write, D-8; the first insertion re-raws `onclick` over `fn`) | `true,true` (one deliverable name: processed at the write, then `fn` replaces it; the insertion finds the clone marked). Equal after T3, which processes the clone at its creation |

T-21's outcome does not depend on T3-b: the script writes a content attribute and reads the Window, and touches no
accessor of `mathBody`.

### §6.2 Unit (session, `tests_applicability.rs`, NEW)

| Cell | Kind | Shape | Oracle |
|---|---|---|---|
| U-T8 | red (full enum; AP-1 to AP-6, AP-9, AP-10, FN-4, FN-5, OR-1a) | for every `EVENT_HANDLER_ATTRS` row × element {HTML `body`, HTML `frameset`, HTML `div`, SVG `body`, SVG `rect`, MathML `body`, MathML `frameset`, MathML `mi`}, **under the compat policy** (`compat()`, as U-A1; under the core policy every `onstorage` row returns at the policy filter in legs (i) and (iii), and the oracle below would fail on those rows for a reason that is not applicability): the predicate's value; and, for each pair, the entry on `resolve_handler_target`'s target (HEAD's) after (i) an `AttributeChange` set through the consumer, (ii) a first insertion of an element carrying the attribute, and (iii) a removal: on a fresh element, a handler placed on that target with `set_inline_handler` (`mod.rs:266`; standing for one set by other means), then an `AttributeChange` with `old_value` some and `new_value` none through the consumer. **State per leg:** legs (i) and (ii) each run on a fresh DOM per (pair, leg) (or assert the entry's before / after difference): with a Window in the harness every `body` × W pair writes the one Window entry, so on a shared DOM a MathML `body` × `onhashchange` row would read the entry an HTML `body` row left. Leg (iii) is self-contained | D-T8's table, computed from the row's `HandlerScope` and the element's class: (i) and (ii) leave an entry iff the predicate is true; (iii) clears the placed entry iff the predicate is true, and leaves it where the predicate is false. Base: every pair leaves an entry at (i) and (ii), and every pair's entry is cleared at (iii). The oracle reads HEAD's target rule and HEAD's insertion delivery, both of which T3 replaces: T3 rewrites this cell (§8.4; the T3 memo §5.2) |
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
| reader (c)'s predicate applied to a set only (a removal still clears for every name) | U-T8 (iii) |
| reader (b) without the predicate | T-A4, U-A1 |
| the predicate applied after the mark's count at (a) (a `continue` beside the policy filter instead of the snapshot's filter) | U-A3, U-A2 |

## §7. Out of scope and slots

| Item | Why not T3-a | Home |
|---|---|---|
| `body`'s interface; foreign elements' HTML interfaces | not applicability (the prototype's surface) | the T3-b memo D-T11 and its §8.2 item 1 |
| target determination, every facet; the namespace in `is_body_or_frameset` | T3 (PM decisions 25, 129) | the T3 memo D-T1 to D-T7 |
| an SVG `body`'s class W content attribute reaching the Window | the target rule (T3 memo FN-2, D-T5) | T3 |
| attribute change steps step 1, namespace half (applicability facet (2)) | no case at the base: no attribute namespace is modelled and no writer of a namespaced attribute exists (H-A8). It needs attribute-namespace modelling and a namespaced writer first | `#11-attribute-namespace-model` symptom (4) (owner L3; PM decisions 768, 770; §8.3 item 1; until rev 1.1: the applicability slot's facet (2), with slice b, PM decision 168) |
| the name table's completeness: 5 attributes of HTML's `GlobalEventHandlers` block (`oncommand`, four `onwebkit*`) and members of partial mixins (`onpointerrawupdate`, `onanimation*`, `onselectstart`, `onselectionchange`, `ontouch*`) have no row (H-A2) | pre-existing; a table and IDL-install defect (no content attribute and no accessor for the name on any element), not applicability: T3-a's predicate answers for the rows the table has | no slot found; reported to the driver for routing (§10 Q-A7) |
| `document.createElementNS` | not this program's | `#11-vm-create-element-ns-missing` (L3; §8.3 item 3) |
| an element outside the three namespaces | no representation (P-A3) | recorded in §8.3 item 1's text; arises with a fourth `Namespace` variant |
| `HandlerScope::DocumentElement`'s doc; `window.oncopy` / `oncut` / `onpaste` | not applicability | `#11-clipboard-handlers-missing-on-window` (owner L3, Re-eval 2026-12-31; PM decision 764; §10 Q-A4) |

Own deferrals: **0**. Residuals: **0**.

## §8. Edges, hand-offs, records

### §8.1 Placement (measured) and edges

**What T3-a needs on its base.** `EVENT_HANDLER_ATTRS` with `HandlerScope`, slice a's processing function, D-8 entry
and AttributeChange arm (`mod.rs`, `event_handler_consumer.rs`; landed by #563), `EcsDom::namespace_of` (older). All
are on main at `cee74989` and at `653d7c24` (H-A10).

**What T3-a does not need.**

| Unit | Does T3-a depend on it? | Why | Contact |
|---|---|---|---|
| T3-b | no (and T3-b does not depend on T3-a) | no shared production code; no row reads the other's outcome (§1.2) | the test helper of §6.1, written by whichever lands first |
| L3 PR (is), PR (ig) | no | no dispatcher window, no take / restore | none |
| T3-pre | no | no creation event; the `SubtreeCreated` arms are not touched | none (T3-pre does not edit T3-a's files) |
| L3's tree-model program (ta → tc / tm / tr / tf → te → ts) | no | no parent or child link read (H-A9) | none |
| L3 PR (io) | no | no walker call added | textual, in `mod.rs` (R-A1) |
| T3 | T3 depends on T3-a (x15) | T3's rewrite sits around T3-a's three reader sites; PM decision 153's R-D2 is closed before T3 | T3 is written on T3-a's landed code |

**Result.** T3-a can be cut from main now and depends on no unlanded unit; it is unordered against T3-b, is, ig,
T3-pre and the tree-model program. Two things bound its landing: it lands before T3's implementation branch is cut
(x15), and **before io lands**, the condition T3-a and T3-b carry for T3-pre's no-op window (the T3-pre memo DoD
item 7; PM decisions 756, 767 (1)). What happens when that condition is at risk (the triggers, who raises it and
what is asked of the PM) is the T3-pre memo §8.3 item 3's ledger text, and is not restated here. (Until rev 1.1 this
paragraph let T3-a land "before, between or after … io" with T3's cut as the only deadline, against DoD item 7; R1
Axis 5.)

**The no-op window of T3-pre's `FragmentParse` arms.** T3-a is not in that window's sequence; the condition it and
T3-b carry for the window is stated once, in the T3-pre memo DoD item 7 (PM decisions 756, 767), and its escalation
once, in the T3-pre memo §8.3 item 3, and not here.

**Edge (umbrella §1.3):** **x15** T3-a → T3, code predecessor + decision precondition (PM decisions 153, 741 (3),
767).

### §8.2 Slot texts (for the PM; defects only)

None. (The foreign-interface defect text is the T3-b memo §8.2 item 1.)

### §8.3 Records for existing slots (for the PM, exact texts; at T3-a's landing)

1. `#11-event-handler-content-attribute-applicability` — **register, as closed** (the ledger has no row; PM 377 set
   its registration at T3's landing, which PM decisions 741 (3) and 767 move to T3-a's; PM decision 770 moves facet
   (2) to another slot, so no facet stays open in this row) with this text: "pre-existing.
   The attribute change steps (HTML §8.1.8.1 Event handlers) step 1 return when the attribute's namespace is not
   null, or its local name is not the name of an event handler content attribute on the element. Before T3-a elidex
   realised neither test: (1) every reader looked the name up in `EVENT_HANDLER_ATTRS` regardless of the element
   (`event_handler_attr_lookup`, `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:237` at
   `cee74989`), so a WindowEventHandlers name on an HTML element other than `body` / `frameset` (HTML §8.1.8.2: they
   are content attributes of body and frameset elements only), or a Document-only name
   (`HandlerScope::DocumentOnly`) on any HTML element, installed a handler on that element; (2) the namespace test is
   not realised and has no case to fail on: `Attributes` is a name → value map with no attribute namespace
   (`crates/core/elidex-ecs/src/components.rs:89-91` at `cee74989`), `MutationEvent::AttributeChange` carries none
   (`crates/core/elidex-ecs/src/dom/mutation_event.rs:366-371`), and no writer of a namespaced attribute exists (no
   `setAttributeNS` in the VM; the parser's *adjust foreign attributes*, HTML §13.2.6.1, namespaces no handler
   name), so every attribute elidex can represent has a null namespace and **facet (2) is unreachable today**; (3)
   which names MathML elements accept was not modelled: every name installed, on a MathML `body` a
   WindowEventHandlers name reaching the Window (the tag-only target rule), where MathML Core §2.1.4 admits the
   names of the `MathMLElement` IDL (GlobalEventHandlers). SVG elements are not a defect: SVG2 §15.8 Event
   attributes makes every supported event type an event attribute of every SVG element. **Facets (1) and (3) closed
   by T3-a** (#<PR>, `<squash>`, `is_event_handler_content_attribute_of`; PM decisions 168, 741 (3), 767): an HTML
   element accepts the GlobalEventHandlers names (`HandlerScope::Global` and `DocumentElement`) and, on `body` /
   `frameset` only, the WindowEventHandlers names; an SVG element every name; a MathML element the
   GlobalEventHandlers names; applied by every reader of an element's handler content attributes (the processing
   function, the D-8 restriction's count, the AttributeChange arm; T3-a memo D-T8, §2.1, cells T-21, T-A1, T-A2,
   T-A4, U-T8). An element outside the HTML, SVG and MathML namespaces has no representation (`Namespace` has three
   variants); the spec gives such an element no event handler content attribute, and the predicate's exhaustive
   `match` forces that decision when a variant is added. **facet (2) → `#11-attribute-namespace-model` (PM 770).**
   It is unreachable today and becomes a reachable defect when both of its prerequisites exist: (a)
   attribute-namespace modelling (`Attributes` and the attribute mutation event carrying an attribute's namespace),
   and (b) a writer of namespaced attributes (`setAttributeNS`, or an XML parse); both are that slot's (its symptoms
   (2) and (4)). The PR that makes a namespaced attribute writable makes the three readers of T3-a's predicate return
   for a non-null namespace in the same PR. No facet remains in this row."
   **The ledger change, old → new (PM decision 770).** *Old* (the text this item handed over until rev 1.1, and the
   T3-ab memo rev 1.1 before it; PM decision 168's premise): "**Facet (2) remains, unreachable.** … **Owner:** Layout
   lane (program `EH-attrs-at-creation`) as registered (PM decision 168 kept it with slice b); whether it moves to
   the owner of (a) and (b) is put to the PM (T3-a memo Q-A6). **Trigger:** a plan that adds an attribute namespace to
   `Attributes`, or one that wires `setAttributeNS`; slice b's plan re-reads this row. **Re-eval:** 2026-12-31."
   *New:* "facet (2) → `#11-attribute-namespace-model` (PM 770)", as in the text above; the row carries no owner,
   trigger or Re-eval of its own for facet (2) (they are that slot's: owner L3, Re-eval 2027-01-15). Measured
   2026-11-18: `/usr/bin/grep -c 'event-handler-content-attribute-applicability' project_open-defer-slots.md` → 0
   (control: `/usr/bin/grep -c 'attribute-namespace-model'` on the same file → 2), so no ledger row holds the old
   wording today and the new line arrives with the registration. PM decision 168's slice-b premise is otherwise
   written only in this program's memos, which this plan PR re-points (umbrella §8.1 and §10.2; T3 memo §7, §8.2,
   §8.3 item 1, §10 O-7). One sentence of the receiving row for the PM to align in the same edit:
   `#11-attribute-namespace-model` symptom (2)'s "(Layout T3 R3 W1-3; T3-b facet (2) waits on this)" names T3-b,
   but the facet is this slot's (the T3-a memo's) → "(Layout T3 R3 W1-3; event handler content attribute
   applicability facet (2) waits on this, symptom (4))".
2. `#11-eh-attrs-creation-time-call-sites` — **no record.** Defect (1)'s "two or more deliverable ordinary handler
   content attributes" reads in the spec's sense after T3-a (an attribute that is not the element's event handler
   content attribute does not count, row AP-8); its text is unchanged.
3. `#11-vm-create-element-ns-missing` — append: "Since T3-a (#<PR>, `<squash>`) one outcome for a foreign `body` is
   pinned on a harness-built element only, because no script can create one (the HTML parser's foreign content
   breaks out at a `body` start tag, HTML §13.2.6.5): a WindowEventHandlers content attribute on a MathML `body`
   installs nothing (T3-a memo cell T-21). When `createElementNS` is wired that cell takes its script form."
   **Trigger unchanged** (PM decision 756); the reason is stated once, in the T3-b memo §8.3 item 1, and holds for
   this sentence as for T3-b's.

### §8.4 Hand-offs

- **T3** (x15): T3 is written on T3-a's landed code. It relies on the predicate with the signature of §4.2 and its
  three reader sites. It adds its own parts to the two functions as the umbrella's §4.1 states, and its
  `ProcessingTiming::Creation` (NEW: T3) path gets the applicability test through reader (a) with no edit (T3 memo
  A7×A12). T3's D-T5 adds the HTML namespace to `is_body_or_frameset`; the predicate's HTML arm calls it and is
  unaffected.
  **Cells of this memo that T3 rewrites (R1 Axis 2; the one list, with T3's expectations, is the T3 memo §5.2).**
  The predicate's values do not change at T3; what three of this memo's oracles read besides them does. (1) U-T8
  leg (ii): its "entry after the first insertion iff the predicate is true" reads HEAD's insertion delivery, which
  T3 ends for the pairs it retargets. (2) U-T8 legs (i) and (iii): they read the target from
  `resolve_handler_target`, which T3 deletes for its own target function. (3)
  `onstorage_content_attribute_gated_by_policy` (§5.2): its ownerless `body` lands on the element only through the
  deleted function's no-Window fallback. (4) U-A1 and every other caller of `test_support::install`, `with_policy`
  or `live_page`, and of `process_before_handler_write`: T3 adds the `active` parameter. The oracles above are stated
  for T3-a's base and hold there; T3's memo owns their T3-head form.
- **Facet (2)'s closer** (`#11-attribute-namespace-model`, owner L3; PM decision 770; §8.3 item 1): extends the
  predicate's callers with the attribute's namespace once attributes carry one.
- **L3's `#11-vm-create-element-ns-missing`**: §8.3 item 3.
- **T3-b**: none in production code; the test helper of §6.1.

## §9. Definition of done

1. T3-a's branch is cut from main (no prerequisite beyond what is on main, §8.1); every `cee74989` coordinate in
   §0.2 is re-measured there by its command; the full run of `elidex-script-session`, `elidex-js` and `elidex-shell`
   on the red commit lists every existing test the predicate changes, and any beyond §5.2's three is added to the PR
   description with its row of §2.1.
2. `/elidex-plan-review` passes on this memo (this memo's R1, five axes; PM decisions 741 (3), 767).
3. Red-first: T-21, T-A1, T-A2, T-A4, U-T8, U-A1, U-A2 committed failing; the fix makes them and §5.2's three
   rewritten tests green; the guards T-A3 and U-A3 are green at the head; the static-marked expectations (T-A2, T-A3)
   are confirmed by their runs.
4. Structural greps:
   - `git grep -n 'is_event_handler_content_attribute_of(' -- crates ':!*tests*'` → the definition and three calls
     (§4.2);
   - `git grep -n 'event_handler_attr_lookup(n).is_some()' -- crates` → 0 (reader (b));
   - `git diff --stat <base>..HEAD -- crates/script/elidex-js/src ':!crates/script/elidex-js/src/vm/tests'
     ':!crates/script/elidex-js/src/vm/test_helpers.rs'` → empty (no VM production file);
   - the predicate's `match` has no `_` arm and no catch-all binding (read at the diff);
   - no `get_parent` in the test files T3-a adds or edits (H-A9, R-A1): `git grep -n 'get_parent' --
     crates/script/elidex-script-session/src/event_handler_processing/tests_applicability.rs
     crates/script/elidex-script-session/src/event_handler_processing/tests_insertion_and_write.rs
     crates/script/elidex-js/src/vm/tests/tests_event_handler_applicability.rs
     crates/script/elidex-js/src/vm/test_helpers.rs` → 0 hits (two of the four are this PR's new files,
     `crates/script/elidex-script-session/src/event_handler_processing/tests_applicability.rs` (NEW) and
     `crates/script/elidex-js/src/vm/tests/tests_event_handler_applicability.rs` (NEW), §5.3). Control, valid while L3's slice `ta` has not landed:
     `git grep -c 'get_parent' -- crates/core/elidex-ecs/src/dom/tree/navigation.rs` → at least 1 (4 at
     `cee74989`); after `ta` the name does not exist and the compile is the check.
5. `cargo test -p elidex-script-session -p elidex-js -p elidex-shell --all-features` green; `mise run ci` before push.
6. §8.3's records (items 1 and 3; item 1 with its old → new for PM decision 770) handed to the PM in the PR
   description.
7. **Checker** (run; do not trust a stored verdict): `python3 .claude/skills/elidex-plan-review/preflight.py
   docs/plans/2026-11-eh-attrs-t3a-applicability.md` exit 0.

## §10. Open questions

**Decided:** the carve (PM decision 741 (3)) and its split into T3-a and T3-b (PM decision 767); D-T8's table for the
three namespaces (the specs, H-A2).

**Within the carve, the designer's (not new scope; flagged for the plan-review):** reader (b), the D-8 count, applies
the predicate (D-T8); `DocumentElement` is class G (P-A1); the foreign `body` cell is harness-built (P-A2); U-T8's
removal leg (iii).

**For the PM:**
- **Q-A1 (the shared functions' shape) — answered (PM decision 756): the designer's reading holds.** The umbrella's
  §4.1 states the final shape of the two functions once. T3-a writes its three reader sites in their final place and
  form and changes no signature; T3 later adds the `active` parameter, because its type `ActiveDocument` and its two
  marshalling points are T3's D-T3. R3 wave 1's Axis 3 confirmed that the umbrella's §4.1 states T3's added `active`
  parameter as the final shape.
- **Q-A2 (a further split) — answered (PM decision 767): two memos**, T3-a and T3-b, by R3 wave 1's Axis 3 verdict
  (PM 227). (Until the T3-ab memo rev 1.1: "one slice for now", PM decision 756.)
- **Q-A3 (the no-op window) — answered (PM decisions 756, 767): not an escalation**; the condition (T3-a and T3-b
  land before io lands) is the T3-pre memo DoD item 7's, and what is raised when it is at risk is the T3-pre memo
  §8.3 item 3's (§8.1).
- **Q-A4 (`window.oncopy` / `oncut` / `onpaste`) — answered and slotted (PM decision 764):
  `#11-clipboard-handlers-missing-on-window`, owner L3, Re-eval 2026-12-31.** Measured 2026-11-18 at `27ca7d1e`: the
  three names are `HandlerScope::DocumentElement` only (`git show
  27ca7d1e:crates/script/elidex-script-session/src/event_handler_consumer.rs | sed -n 164,166p`); Window's prototype
  installs the `Global` and `Window` scopes only (`git show 27ca7d1e:crates/script/elidex-js/src/vm/host/window.rs |
  sed -n 601,604p`), while Document (`git show 27ca7d1e:crates/script/elidex-js/src/vm/host/document.rs | sed -n
  347,349p`) and HTMLElement (`git show 27ca7d1e:crates/script/elidex-js/src/vm/host/html_element_proto.rs | sed -n
  137,138p`) install `DocumentElement`. The HTML IDL lists all three in `GlobalEventHandlers` (`.claude/tools/webref
  idl html GlobalEventHandlers`), which Window includes, and has no `DocumentAndElementEventHandlers` mixin. So the
  absence on Window is a pre-existing IDL-surface defect, outside this program and unchanged by T3-a (D-T8 reads
  content attributes, not the Window's accessors). (Until the T3-ab memo rev 1.1: "for the PM to slot".)
- **Q-A5 (the label) — answered (PM decisions 756, 767):** `T3-ab` was approved; its halves are `T3-a` and `T3-b`.
- **Q-A6 (facet (2)'s owner) — answered (PM decision 770): facet (2) moves from slice b to
  `#11-attribute-namespace-model`** (owner L3; registered by PM decision 768), and PM decision 168's premise that
  slice b adds a namespace to the attribute event is revised (W1-3: the facet needs attribute-namespace modelling
  and a namespaced writer, neither of which is in slice b's S-1 to S-7). Ground: one issue, one way. The ledger
  change is carried, old → new, by §8.3 item 1. (Until rev 1.1 this was an open question.)
- **Q-A7 (new in rev 1.1; for the driver to route; not blocking).** The name table lacks members of
  `GlobalEventHandlers` (H-A2): 5 attributes of HTML's own block (`oncommand`, `onwebkitanimationend`,
  `onwebkitanimationiteration`, `onwebkitanimationstart`, `onwebkittransitionend`) and, of the partial mixins
  measured, `onpointerrawupdate` (Pointer Events), `onanimationstart` / `onanimationiteration` / `onanimationend` /
  `onanimationcancel` (CSS Animations), `onselectstart` / `onselectionchange` (Selection API) and `ontouchstart` /
  `ontouchend` / `ontouchmove` / `ontouchcancel` (Touch Events). The list of partial mixins was not enumerated
  exhaustively. Pre-existing and outside T3-a (§7): T3-a opens no slot for it and does not depend on the answer;
  whether a ledger row covers it was not searched beyond the slot ids this memo cites.

No browser measurement is needed: every outcome in §2.1 is spec-defined (H-A2).
