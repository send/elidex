# T3 `EH-attrs-at-creation-T3` — *determine the target of an event handler* follows the spec (per-PR plan-memo)

Nested PR **T3** of the standalone program `EH-attrs-at-creation` (umbrella
`docs/plans/2026-10-event-handler-attrs-at-creation.md`, approved by #551 `b9122157` at rev 8, carried to rev 8.3 in place by
#570; "the umbrella"). It closes slot `#11-event-handler-target-determination` (all four facets) and is the prerequisite
x3 of the umbrella's slice b. Siblings cited by ID: slice a (`docs/plans/2026-10-eh-attrs-slice-a.md`, landed by #563
`3cd84a04`; "the slice-a plan"), IBP-sandbox N1 (`docs/plans/2026-10-ibp-sandbox-handler-gates.md`, rev 4.5, merged as
a plan by #570, implementation in flight; "the N1 memo"), and the IBP-sandbox node memo
(`docs/plans/2026-09-ibp-sandbox.md`; "the node memo").

**Placement (evidence).** T3 sits under the **EH-attrs umbrella**, not under IBP-sandbox:
- umbrella §1.2 nested-PR table, row **T3** `EH-attrs-at-creation-T3` ("owner: Layout lane, this program; PM decision
  25"; scope: facet (1) with facets (3) and (4), facet (2) "rides with it unless its plan splits it off"; prerequisite:
  slice a landed); edges x3 (T3 → slice b, semantic precondition) and x6 (slice a → T3, scheduling) in §1.3; §8.1's
  first bullet;
- the ledger row (`project_open-defer-slots.md`, section "Event handlers — pre-existing slot", the sub-bullet **"Owner
  re-routed 2026-10-25 (PM decision 25)"**): "the owner is now the EH-attrs-at-creation program (Layout lane). T3 is a
  standalone prerequisite PR of slice b, outside the IBP-sandbox DAG". The row's first paragraph still reads "Owner:
  the IBP-sandbox program"; the sub-bullet supersedes it (§8.3 item 1 asks the PM to fold it);
- the N1 memo §7 row "*determine the target of an event handler*": "reassigned by PM decision 25 to the EH-attrs
  program, whose T3 PR carries facets 1, 3 and 4".

**Scheduling.** PM decisions 122/123: N1's implementation → T3 → C1 → slice b. T3's implementation branch is cut from
main **after N1's implementation lands**; this memo's code coordinates are `74e15df1` (pre-N1) and are re-measured at that
base (§9 item 1).

**Scope.** One function realises *determine the target of an event handler* (WHATWG HTML §8.1.8.1 Event handlers),
steps 1–4, and every caller the spec gives it calls that function: the attribute change steps (content attributes, at
every processing timing of slice a), the event handler IDL attribute getter and setter (both VM accessor pairs).
Facets: (1) step 3, a node document that is not an active document → null; (2) step 4, the node document's relevant
global object; (3) step 1's `frameset`; (4) step 2's Window-reflecting body element event handler set. Plus the step-1
class test made one predicate (HTML namespace), with its sibling `document.body` site (D-T5).

**Not in scope.** X-W's timing (slice a's interim; removed by slice b's S-5), the D-8 restriction, creation-time
processing (slice b), C1 (`#11-template-contents-owner-document`; interactions noted only, §2 A3×A8), insertion
adoption (`#11-cross-document-adopt-on-insert`), the `HTMLFrameSetElement` interface (§7, proposed slot), the
attribute change steps' step 1 (§7, proposed slot), multi-realm (B1; §8.2 hand-off).

**Changelog.** *Rev 1* (2026-10-31): first draft, pre-plan-review.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every code `file:line` is a **`74e15df1`** coordinate (origin/main, #570), measured on the worktree with
`git grep -n -E '<pattern>' 74e15df1 -- <paths>` or `/usr/bin/grep -n` (never the default ugrep). Spec text is
`.claude/tools/webref body` output of 2026-10-31; every `§X.Y = title` pair is `webref heading` / `dfn` output (§3's
footnote). Paths are abbreviated after first use: `session/…` = `crates/script/elidex-script-session/src/…`,
`elidex-js/…` = `crates/script/elidex-js/src/…`, `ecs/…` = `crates/core/elidex-ecs/src/…`. N1's implementation will move
lines in `elidex-js/vm/host/event_handler_attrs.rs` and `session/scripting.rs` (N1 memo §5.3); every coordinate in those two
files is re-measured at T3's base (§9 item 1).

Symbols written as `Type` + method (`EcsDom::window_entity`, `EcsDom::is_base_element`, `HostData::window_entity`,
`test_support::install`) name existing methods or functions inside `impl` / `mod` blocks, which preflight's literal
symbol grep does not find (its soft warnings). `EcsDom::effective_node_document` (planned, N4) is the node memo's name
for N4's future home of D-T4's function.

### §0.2 Premises re-measured

| ID | Premise | Command | Result at `74e15df1` |
|---|---|---|---|
| H-T1 | One target function, two content-attribute callers | `git grep -n 'resolve_handler_target(' 74e15df1 -- crates` | defined `session/event_handler_processing/mod.rs:249`; called by `process_handler_content_attributes` `mod.rs:130` (all three timings: Bind, Insertion, HandlerWrite) and by the consumer's AttributeChange arm `session/event_handler_consumer.rs:420`. Nothing else |
| H-T2 | HEAD's target rule | `sed -n 244,262p session/event_handler_processing/mod.rs` | `scope == HandlerScope::Window && is_body_element(origin)` → `dom.window_entity().unwrap_or(origin)`, else `origin` (`:249-255`); `is_body_element` is a tag-only, ASCII-case-insensitive `body` test (`:258-262`). So: no step 3; step 4 = the World's one `NodeKind::Window` entity (`ecs/dom/mod.rs:450`, a linear scan); no `frameset`; the reflecting six (`HandlerScope::Global` rows) stay on the element |
| H-T3 | The IDL path determines its target separately | `sed -n 323,358p` and `sed -n 418,483p elidex-js/vm/host/event_handler_attrs.rs` | normal pair (`native_event_handler_get` `:323`, `native_event_handler_set` `:344`): target = the receiver, always. Body pair (`native_body_weh_get` `:439`, `native_body_weh_set` `:461`), installed on `HTMLBodyElement.prototype` for `HandlerScope::Window` rows only (`install_body_weh_delegation` `:207-214`, called once, `elidex-js/vm/host/html_passive_protos.rs:66`): target = `ctx.host().window_entity()` (`:448`, `:471`), no step 3. The reflecting six on a `<body>` use `HTMLElement.prototype`'s normal pair (`elidex-js/vm/host/html_element_proto.rs:134`). A `frameset` gets no body pair (prototype chosen by tag `body` only, `elidex-js/vm/host/elements.rs:264`) |
| H-T4 | Steps 1–2 already have one predicate, used by timings only | `sed -n 213,232p session/event_handler_processing/mod.rs` | `is_window_delegating_pair` (`:216`): `is_body_or_frameset` (`:228`, tag-only) ∧ (`HandlerScope::Window` row ∨ `WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET` `:42`). Read by `process_handler_content_attributes` (`:111`), `process_before_handler_write` (`:156`) and `is_only_deliverable_ordinary_name` (`:208`) — **not** by `resolve_handler_target`. So at HEAD the X-W classifier and the target rule disagree on the reflecting six and on `frameset` (the classifier says Window-delegating; the target stays the element) |
| H-T5 | The spec's three callers | `.claude/tools/webref body html event-handler-attributes` | the getter steps 1–2 (determine; null → return null), the setter steps 1–2 (determine; null → return), the attribute change steps steps 2–3 (determine; null → return). Step 3's note: a `body` created in an active document but not connected still targets the Window |
| H-T6 | "Active document" in HEAD's model | `sed -n 12,82p session/scripting.rs`; the B1 decision `docs/plans/2026-06-agent-scoped-ecsdom-world.md` §5 req 4 | `scripting_disabled_for_platform_object`'s docstring (`:19-30`): "`bound_document` is the active document of the engine's (single) browsing context"; a Node whose node document is not the bound document has a null browsing context. B1 req 4: same-agent iframes are today a separate `EcsDom` + separate `Vm`. So in one `EcsDom` the bound document is the only active document, and its relevant global object is the one Window (`webref body html navigables`: a navigable's active window "will always equal the navigable's active document's relevant global object"). **Unverified at `74e15df1`**: Q-1 (§10) |
| H-T7 | The node-document derivation already has one rule | `sed -n 95,114p session/scripting.rs` | clause (b) derives the **effective node document** inline (`:104-110`): composed tree root if it is a Document, else `owner_document`; `None` → fail open. The node memo plans its promotion to `EcsDom::effective_node_document` by N4 (node memo §0.4 D8, §4 "Node document" row). No other derivation of a node document for a handler path exists (`git grep -n 'find_tree_root_composed' 74e15df1 -- crates` → `scripting.rs:104` and `elidex-js/vm/host/node_proto.rs:455`, the `isConnected` getter) |
| H-T8 | Insertion does not adopt | `sed -n 35,54p session/scripting.rs`; `webref body dom mutation-algorithms` (insert step "Adopt node into parent's node document") | `EcsDom::append_child` relinks without re-homing `AssociatedDocument`; slot `#11-cross-document-adopt-on-insert` |
| H-T9 | DOMParser documents are in the page `EcsDom`, built with the dispatcher detached, and reach the consumer after the build | `sed -n 108,120p` and `sed -n 195,200p crates/dom/elidex-form/src/inert_document.rs` | `parse_into_inert_document` takes the dispatcher (`:112`), builds on `create_document_node()` (`:119`), restores it (`:198-199`); later `setAttribute` and inserts on its nodes fire the consumer (N1 memo H-P13) |
| H-T10 | The consumer has no bound document today | `sed -n 96,145p elidex-js/vm/consumer_dispatcher.rs`; `sed -n 292,316p elidex-js/vm/vm_api.rs` | `ConsumerDispatcher::new` builds `EventHandlerAttributeConsumer::with_policy(policy)` (`:110`); the bound `document` reaches the consumer only as `initialize_consumers(dom, document)` (`:141`, called `vm_api.rs:315`) for the bind pass. The Window entity is resolved before the dispatcher is built (`vm_api.rs:224-229`) |
| H-T11 | `EcsDom::window_entity`'s production readers | `git grep -n -E '\.window_entity\(\)' 74e15df1 -- crates` minus `HostData` / `runtime` receivers | one: `resolve_handler_target` (`mod.rs:251`). Tests: `session/event_handler_processing/tests_insertion_and_write.rs` (3) |
| H-T12 | The body-or-frameset class test is written three times | `git grep -n 'eq_ignore_ascii_case("body")' 74e15df1 -- crates` | `mod.rs:231` (`is_body_or_frameset`), `mod.rs:261` (`is_body_element`), `crates/dom/elidex-dom-api/src/char_data/document_props.rs:173` (`first_body_or_frameset_child`, the `document.body` getter and the VM focus fallback). None checks the HTML namespace; the precedent `EcsDom::is_base_element` (`ecs/dom/classify.rs:39`) does |
| H-T13 | Slice a's window pins that name T3 | `git grep -n 'T3' 74e15df1 -- crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs_window_delegating.rs` | C17 (`:39`), C18 (`:64`), C19b (`:122`), C19c (`:132`), P3 (`:172`) say "reached after T3 and slice b" or "T3 facet 3" |
| H-T14 | `<body onload>` never runs at HEAD | `sed -n 462,474p crates/shell/elidex-shell/src/pipeline.rs` | the document `load` event is dispatched at the Window entity (`:470`), non-bubbling; `<body onload>` lands on the body element (H-T2), so it never runs. Same for an initial `<body onerror|onfocus|onblur|onresize|onscroll>` against Window-targeted events |

### §0.3 Premise errors (reported)

- **P-T1.** The ledger row's coordinate "`resolve_handler_target` (`…/event_handler_processing/mod.rs:249`) … code as of
  `fd6955e8`" mixes frames: at `fd6955e8` the function is `session/event_handler_consumer.rs:328`
  (`git show fd6955e8:crates/script/elidex-script-session/src/event_handler_processing/mod.rs` fails: the file was added by
  #563); `mod.rs:249` is the `3cd84a04` / `74e15df1` position. §8.3 item 1 carries the correction.
- **P-T2.** The slot records **four** facets, not three (facet (4), the reflecting set, was added 2026-10-25), and its
  owner is the EH-attrs program (re-routed sub-bullet), although its first paragraph still says IBP-sandbox.
- **P-T3.** The Reachability example (`onload` → `onhashchange`) is already corrected in the ledger as read on
  2026-10-31 (line 807 reads `setAttribute('onhashchange', …)`).

### §0.4 Decisions (this PR)

- **D-T1 — one function, every spec caller.** `determine_event_handler_target(dom, target, name, active) ->
  Option<Entity>` (NEW), in `session/event_handler_processing/mod.rs`, `pub`, is the only realisation of the algorithm.
  `None` is the spec's null. Callers: the attribute change steps at `process_handler_content_attributes` (every timing)
  and the AttributeChange arm; the VM getter and setter, both pairs. `resolve_handler_target` and `is_body_element` are
  deleted. The body pair keeps its own receiver brand check (`require_body_receiver`; WebIDL `HTMLBodyElement`) but no
  longer chooses a target. The VmObject handler backend (`native_vm_event_handler_get` / `_set`) does not call it: its
  receivers are never `body` / `frameset` elements, so step 1 returns the receiver by type (stated in its docstring).
- **D-T2 — steps 1–2 are `is_window_delegating_pair`.** The function calls slice a's classifier for steps 1–2 and returns
  `Some(target)` when it is false. The X-W classifier and the target rule then agree by construction (H-T4's
  disagreement is the defect of facets (3) and (4)). Step 2's name test is by name: the `scope` parameter leaves target
  determination (`HandlerScope` stays the IDL-install key).
- **D-T3 — steps 3–4 over a marshalled `ActiveDocument` (NEW).** `pub struct ActiveDocument { document: Entity, window:
  Entity }` — the active document of the engine's single browsing context and its relevant global object (H-T6). Step
  3: the target's effective node document (D-T4) equals `active.document`, else `None`. Step 4: `active.window`. The
  consumer receives it at construction from `Vm::bind` (`document` and the resolved Window, H-T10); the VM accessors
  build it from `HostData` (`document_entity_opt()`, `window_entity()`). `EcsDom::window_entity` loses its last
  production reader and is deleted (H-T11; dead-code rule). *Rejected:* (A) a component relating the Document and its
  Window, written at bind — the ECS-native end state, but it duplicates `HostData`'s `document_entity` /
  `window_entity` pair, which B1 §5 req 5 migrates as one cluster; the user's ruling (iv) of 2026-09-28
  (`project_browsing-context-state-ecs-components`) moved only the sandbox flag set ahead of B1, by a discriminator
  (read without a script runtime and immutable per Document) that activeness does not meet (a document stops being
  active; every reader here runs while bound). (D) `document_root()` keying — forbidden by slice a's D-5 / cell U12 and
  the node memo ("No reader keys on `document_root()`"). Facet (2) is therefore **implemented in full in HEAD's model**
  (one active document and one Window per `EcsDom`, H-T6); B1's multi-realm World replaces the source of
  `ActiveDocument` at its two marshalling points, not the function (§8.2 hand-off). If Q-1 finds a second Window or a
  second active document in one `EcsDom`, that is a defect to state before plan-review, not a scope cut.
- **D-T4 — one node-document derivation.** The effective-node-document rule (H-T7) is extracted from clause (b) into
  `effective_node_document(dom, node) -> Option<Entity>` (NEW), a `pub` free function in `session/scripting.rs`, called by
  clause (b) (code-identical) and by D-T1's step 3. Unresolvable (`None`) → treated as `active.document`: the same
  direction as clause (b)'s fail-open, on the same ground ("such nodes belong to the bound document's realm by
  construction", `scripting.rs:55-58`), and HEAD's outcome (Window). N4 keeps its row: it moves this one function into
  `EcsDom::effective_node_document` in `ecs/dom/document_policy.rs` as the node memo plans; no ratified text is edited.
  *Rejected:* landing `EcsDom::effective_node_document` here (it edits N4's approved row), and `owner_document` alone
  (wrong in both directions for a connected node, H-T8; cell T-3 kills it).
- **D-T5 — one step-1 class predicate, HTML namespace included.** `EcsDom::is_body_or_frameset_element` (NEW) in
  `ecs/dom/classify.rs`, following `is_base_element` (tag ASCII-case-insensitive ∧ `is_html_namespace`). Called by
  `is_window_delegating_pair`, by `process_handler_content_attributes`' marked shortcut (`mod.rs:85`), and by
  `first_body_or_frameset_child` (`document_props.rs:169`), whose `document.body` getter has the same class test
  ("either a body element or a frameset element", HTML §3.1.7 DOM tree accessors). Session `is_body_or_frameset` is
  deleted. Observable change at the sibling: a foreign-namespace `body` child of `<html>` is no longer `document.body`
  (cell T-13). The VM's prototype selection and `require_body_receiver` (`tag_matches_ascii_case`) are interface brand
  checks, not this predicate, and are not touched (Q-4).
- **D-T6 — order inside the IDL setter.** receiver brand check → determine (D-T1) → `None`: return → slice a's
  `marshal_before_handler_write` (D-6) → activate. Determination precedes D-6 per the setter's steps 1–2; the reorder
  changes nothing observable, because `None` occurs only for Window-delegating pairs, for which
  `process_before_handler_write` returns at once (`mod.rs:156`). The getter: brand check → determine → `None`: null →
  read (N1's reconcile, unchanged).
- **D-T7 — X-W's timing is not changed.** Slice a's timings keep delivering Window-delegating pairs at Bind and
  Insertion (`mod.rs:120-124`); only their target changes. The consequence for the pairs HEAD targeted at the element is
  §1's verdict and O-1.

## §1. Shape and verdict

| Axis | Own invariant |
|---|---|
| A1 class (step 1) | `body` / `frameset` = one predicate, HTML namespace (D-T5) |
| A2 names (step 2) | `WindowEventHandlers` members ∪ the reflecting six = `is_window_delegating_pair` (D-T2) |
| A3 activeness (step 3) | effective node document (D-T4) == `active.document`, else null |
| A4 global (step 4) | `active.window` (D-T3) |
| A5 callers | attribute change steps at every timing + AttributeChange arm; getter; setter, both pairs (D-T1, D-T6) |

Coupled from outside: A6 slice a's X-W timing (D-T7), A7 N1's compile-gate node clause (same derivation, D-T4), A8 C1
(template contents report the bound document).

**Outcome changes at T3** (all toward the spec, except the window of O-1):
- WindowEventHandlers name on a `body` / `frameset` whose effective node document is not active (DOMParser, a node
  inserted into one): HEAD → page Window; T3 → null at every caller (facet 1).
- reflecting six on `body`, and all 24 Window-delegating names on `frameset`, in the active document: HEAD → the element;
  T3 → the Window (facets 3, 4). In a non-active document: HEAD → the element (from N1: getter null); T3 → null.
- an initial `<body onload>` (and the other five) runs (H-T14; closes umbrella K-0's E-G1 (i)).
- a foreign-namespace `body` is not step 1's class; nor `document.body` (D-T5).

**The O-1 window (worse than HEAD on one write path).** For the pairs HEAD targeted at the element (reflecting six on
`body`; all 24 names on `frameset`), X-W's re-raw at every insertion now lands on the **Window**:
- **W1**: `window.on<T> = fn`, then an insertion of that `body` / `frameset` (or of an ancestor) re-raws its attribute
  over `fn`. HEAD kept `fn` (its re-raw hit the element). Spec: `fn`. (Writes through `body.on<T>` are overwritten at
  HEAD and at T3 alike.)
- **W2**: such an attribute set while the element was outside an active document (null at T3) is delivered to the page
  Window at its first insertion into the active document. HEAD delivered it to the element; spec: nothing.
Both are the class HEAD already has for the 18 WindowEventHandlers names on `body` (cells C8, C17; umbrella §1.5 X-W),
extended to the newly Window-targeted pairs, and both close at slice b's S-5 (X-W removed; S-3 then binds slice b
against its own base, which contains T3). Pins T-14, T-15. Whether T3 may land with them is **O-1** (PM / user).

**Edge density: edge-dense** (*verdict*). Five own axes meet in one algorithm with three spec callers, and three
external couplings (A6–A8) change outcomes at the intersections (§2: W1/W2 at A4×A6, C8h's mechanism at A3×A7, C18 at
A3×A8). So CLAUDE.md "Edge-dense work" applies: plan-review is mandatory (rule (b)). **Split: none** — T3 is a
narrowly-scoped per-PR slice under the approved EH-attrs umbrella (§1.2 row T3), the rule's base case (c). Splitting
by facet or by caller would leave two target rules live (content attribute vs IDL, or body vs frameset), the strangler
one-issue-one-way forbids; D-T5's `document.body` sibling is a separate commit in the same PR (same predicate, same
commit series). Breadth: K = 2 specs, M = 10 rows (§3); `webref coverage-map html 8.1.8.1 html 8.1.8.2 html 7.3.2 html
8.1.3.3.4 dom 4.4 html 16.3.2` → "ok → single PR scope".

## §2. Coupled invariants

- **A1×A2** — steps 1–2 are one conjunction (`is_window_delegating_pair`); a frameset passes A1 for every A2 name.
- **A1×A5** — every caller reaches A1 through D-T1 only; the VM brand checks are separate (interfaces, D-T5).
- **A1×A6** — the marked shortcut (`mod.rs:85`) uses the same predicate, so a marked foreign `body` returns early.
- **A2×A5** — the IDL normal pair now asks A2 too: `body.onload` (HTMLElement accessor) → Window (cell T-5).
- **A2×A6** — X-W's classifier and the target agree (H-T4 resolved); a pair is Window-delegating iff it targets the
  Window or null.
- **A3×A5** — null at each caller: content attribute → no write and no clear; getter → null; setter → return.
- **A3×A6** — an insertion into a non-active document delivers nothing (T-3); into the active document → Window.
- **A3×A7** — one derivation (D-T4): N1's compile gate and step 3 cannot disagree on a node's document.
- **A3×A8** — a `body` in template contents reports the bound document (C2 approximation), so it is active and targets
  the Window; spec: inert → null. C18 keeps HEAD's outcome; C1's, not T3's.
- **A4×A5** — `ActiveDocument` has two marshalling points (consumer construction, VM `HostData`), one source each.
- **A4×A6** — re-raw at insertion now writes the Window for the newly Window-targeted pairs: W1, W2 (O-1).
- **A2×A7** — N1's C8h (`dd.body.setAttribute('onfocus', 'A')`) keeps its outcome (getter null, nothing runs) but by
  step 3 now: no entry is written (cell U-T3).
- **A5×A7** — the getter determines before N1's reconcile (getter step 1 before step 3).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.8.1 Event handlers | *determine the target of an event handler* steps 1–4 | 1 not body/frameset → self; 2 not WEH and not reflecting → self; 3 effective node document ≠ active document → null; 4 → active Window | `determine_event_handler_target` (NEW) in `crates/script/elidex-script-session/src/event_handler_processing/mod.rs`, replacing `resolve_handler_target` (`:249`) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | event handler IDL attribute getter steps 1–3 | null target → null; else get the current value (N1's reconcile) | `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` `native_event_handler_get` `:323`, `native_body_weh_get` `:439` | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | event handler IDL attribute setter steps 1–4 | null target → return; null value → deactivate; else activate | `native_event_handler_set` `:344`, `native_body_weh_set` `:461` | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | attribute change steps steps 2–5 | null target → return (no set, no clear); step 1 and step 5.1 not touched (§7) | `crates/script/elidex-script-session/src/event_handler_consumer.rs:420`; `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:129-132` | ✗ | yes |
| WHATWG HTML §8.1.8.2 Event handlers on elements, Document objects, and Window objects | the WEH table and the Window-reflecting body element event handler set | content and IDL attributes on body and frameset act on the Window | `is_window_delegating_pair` `mod.rs:216`; `WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET` `mod.rs:42` (unchanged) | ✓ | yes |
| WHATWG HTML §7.3.2 Browsing contexts | *active document* | in HEAD's model: the bound document (H-T6) | `ActiveDocument` (NEW), marshalled at `crates/script/elidex-js/src/vm/vm_api.rs:292` and in `event_handler_attrs.rs` | ✗ | no |
| WHATWG HTML §8.1.3.3.4 Relevant | *relevant global object* of the node document | active document → its Window (navigable's active window, §7.3.1) | `ActiveDocument::window` (NEW) | ✗ | no |
| WHATWG DOM §4.4 Interface Node | *node document* | effective node document (composed root Document, else `owner_document`; `None` → active) | `effective_node_document` (NEW) extracted from `crates/script/elidex-script-session/src/scripting.rs:104-110` | ✗ | yes |
| WHATWG HTML §16.3.2 Frames | `frameset` element; `HTMLFrameSetElement includes WindowEventHandlers` | content attributes and inherited GlobalEventHandlers accessors → step 1; the interface itself absent (§7) | `EcsDom::is_body_or_frameset_element` (NEW), `crates/core/elidex-ecs/src/dom/classify.rs` | ✗ | yes |
| WHATWG HTML §3.1.7 DOM tree accessors | *the body element* | first `html` child that is a body or frameset element (HTML namespace) | `crates/dom/elidex-dom-api/src/char_data/document_props.rs:169` | ✓ | no |

Numbers and titles (2026-10-31): `webref heading html 8.1.8.1` (Event handlers, `#event-handler-attributes`),
`8.1.8.2` (Event handlers on elements, Document objects, and Window objects), `7.3.2` (Browsing contexts, `#windows`),
`7.3.1` (Navigables), `8.1.3.3.4` (Relevant), `16.3.2` (Frames), `3.1.7` (DOM tree accessors), `7.2.2` (The Window
object), `8.1.3.4` (Enabling and disabling scripting); `webref heading dom 4.4` (Interface Node), `4.2.3` (Mutation
algorithms). `webref dfn html`: 'determine the target of an event handler' → §8.1.8.1
`#determining-the-target-of-an-event-handler`; 'Window-reflecting body element event handler set' → §8.1.8.2;
'active document' → §7.3.2 `#active-document` (and for=navigable §7.3.1 `#nav-document`); 'relevant global object' →
§8.1.3.3.4 `#concept-relevant-global`; 'associated Document' → §7.2.2 `#concept-document-window`; 'The body element' →
§3.1.7; 'WindowEventHandlers' → §8.1.8.2.1; 'frameset' → §16.3.2 `#frameset`; `webref dfn dom 'node document'` → §4.4
`#concept-node-document`. Step texts: `webref body html event-handler-attributes`,
`event-handlers-on-elements,-document-objects,-and-window-objects`, `frames`, `navigables`, `relevant`,
`dom-tree-accessors`; `webref body dom mutation-algorithms`.

### §3.1 User-input touch audit

Author-controlled inputs reaching the changed code: handler content attributes on `body` / `frameset` (parser-baked
and `setAttribute`; in a DOMParser document they reach the consumer only after the build, H-T9); IDL handler writes
on `body` / `frameset` / any element (`el.on<T> = …`); `DOMParser` documents; tree moves between documents (the
effective node document is re-read at each determination); `createElementNS` with a foreign namespace and local name
`body` / `frameset`; the order of `window.on<T>` writes and insertions (W1). Every change narrows where a handler lands
(null instead of the page Window for a non-active document) or moves it to the spec's target in the same realm (the
active document's own Window). No case delivers a handler to a Window other than the bound document's, and no null is
turned into a delivery, except W2 (an attribute set while outside the active document, delivered at insertion into it:
same realm, and the class HEAD already has for WindowEventHandlers names, C17). The unresolvable node-document case
fails toward HEAD's Window (D-T4), which only nodes of the bound realm can reach. Security boundaries (CSP, sandbox)
are not changed; CSP stays `#11-csp-inline-handler-enforcement`.

## §4. Mechanism

### §4.1 Home / kind

| Fact | Home / kind | Status |
|---|---|---|
| target determination | `determine_event_handler_target` (NEW), free fn, `session/event_handler_processing/mod.rs` | D-T1 |
| active-document context | `ActiveDocument` (NEW), `Copy` struct of two entities, same module; held by `EventHandlerAttributeConsumer` (constructor `EventHandlerAttributeConsumer::new(policy, active)`, replacing `with_policy`; `Default` derive removed); built per call by the VM accessors | D-T3 |
| node document | `effective_node_document` (NEW), `session/scripting.rs`; N4 later moves it to `EcsDom` | D-T4 |
| step-1 class | `EcsDom::is_body_or_frameset_element` (NEW), `ecs/dom/classify.rs` | D-T5 |
| steps 1–2 | `is_window_delegating_pair` (kept; body now calls D-T5) | D-T2 |
| deleted | `resolve_handler_target`, `is_body_element`, session `is_body_or_frameset`, `EcsDom::window_entity` | D-T1, D-T3, D-T5 |

No component, store, trait or crate edge is added (`elidex-js` already depends on `elidex-script-session`).

### §4.2 Shape

```rust
// session/event_handler_processing/mod.rs
pub fn determine_event_handler_target(          // (NEW)
    dom: &EcsDom, target: Entity, name: &str, active: ActiveDocument,
) -> Option<Entity> {
    if !is_window_delegating_pair(dom, target, name) {   // steps 1–2
        return Some(target);
    }
    let node_document = effective_node_document(dom, target).unwrap_or(active.document);
    (node_document == active.document).then_some(active.window)   // steps 3–4
}
```

Call sites after T3 (re-runnable: `/usr/bin/grep -rn 'determine_event_handler_target(' crates` → exactly the five
below plus the definition):
1. `process_handler_content_attributes` — `None` → `continue` (no write); signature gains `active` (so do
   `process_bound_document` and `process_before_handler_write`; the latter's VM caller marshals it).
2. `handle_attribute_change` — `None` → return before set / clear.
3. `native_event_handler_get` / 4. `native_event_handler_set` — the normal pair, with `ActiveDocument` from `HostData`.
5. the body pair's shared core (`native_body_weh_get` / `_set` call one helper after `require_body_receiver`).

The VM builds `ActiveDocument` through one host helper (`ActiveDocument` from `host.document_entity_opt()` and
`host.window_entity()`; `None` = unbound → the accessor's existing silent no-op). It reads `HostData` only (marshalling,
Layering mandate); no `EcsDom::*` call is added in `vm/host/`.

### §4.3 ECS-native and Layering checks

The algorithm is a free function over the ECS DOM (a system-like read: tag/namespace components, tree relation,
`AssociatedDocument`), returning an entity the caller writes `EventListeners` on — the existing component. OO → ECS
idiom: the "override the accessor on the subclass prototype" pattern (`HTMLBodyElement.prototype`'s delegation pair)
is replaced, for the target choice, by one query on the receiver entity; the prototype install remains only as the
WebIDL surface. `ActiveDocument` is a per-call value, not a store (D-T3 records why it is not a component yet). Layering:
steps 1–4 live in `elidex-script-session`; `vm/host/event_handler_attrs.rs` brand-checks, marshals `HostData` into
`ActiveDocument`, and executes the result on the listener store.

### §4.4 Docstring citation table (Axis 4)

| Item | Cites |
|---|---|
| `determine_event_handler_target` (NEW) | HTML §8.1.8.1 *determine the target of an event handler* steps 1–4; step 3 = "not an active document" in the single-browsing-context model (H-T6); B1 hand-off |
| `ActiveDocument` (NEW) | HTML §7.3.2 *active document*; §8.1.3.3.4 *relevant global object*; §7.3.1 (active window = active document's relevant global) |
| `effective_node_document` (NEW) | DOM §4.4 *node document*; the adopt-equivalent proxy (`#11-cross-document-adopt-on-insert`); N4's planned home |
| `EcsDom::is_body_or_frameset_element` (NEW) | HTML §8.1.8.1 step 1; §3.1.7 *the body element*; §16.3.2 `frameset` |
| `native_event_handler_get` / `_set`, body pair | §8.1.8.1 getter steps 1–3 / setter steps 1–4 |
| `handle_attribute_change`, `process_handler_content_attributes` | §8.1.8.1 attribute change steps 2–3 (the docstring's "Not realised: step 3" at `mod.rs:64-68` is removed) |

## §5. Populations, touched files, named risks

### §5.1 Behavioural population (re-runnable)

The pairs whose target changes are exactly those `is_window_delegating_pair` accepts and HEAD's rule did not, plus the
non-active ones: `EVENT_HANDLER_ATTRS` rows with `HandlerScope::Window` (18; the slice-a ratchet test
`tests_insertion_and_write.rs:318-328` equates them with `webref idl html WindowEventHandlers`) and the six of
`WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET` (ratchet `:331-342`), on `body` and `frameset`. Unit cell U-T1 derives
its table from those two lists (not a sample).

### §5.2 Existing tests whose expectation changes (red at T3 by design, rewritten in the same commit)

| Test | HEAD expectation | T3 expectation | Why |
|---|---|---|---|
| C17 `c17_domparser_body_weh_idl_write_then_append` (`tests_event_handler_attrs_window_delegating.rs:39`) | `true,true,true,true,true,true` | `true,false,true,true,true,true` | step 1's setter returns (null target); the append still re-raws (X-W) — window pin until slice b |
| C19b `c19b_body_reflecting_pair_reraw_at_insertion` (`:122`) | `true,true,true,true` | `true,true,false,true` | `onfocus` targets the Window; the setter returns while in `dp`; dispatch at `b` runs nothing. Equals the spec → becomes a guard |
| C19c `c19c_frameset_reflecting_pair_reraw_at_insertion` (`:132`) | same as C19b | same as C19b | facet 3 + 4; spec-equal → guard |
| P3 `p3_frameset_clone_classified_by_tag_and_name` (`:172`) | `true,true` | `true,false` | `onhashchange` on a `frameset` targets the Window; `hashchange` at `f` runs nothing. Spec-equal |
| U2, C8, C18, C19, C20 (i), C20 (ii), U3 (ii), P1 (i) | — | unchanged | active document (C8, C19 final, U2, P1), C1 (C18), ordinary pair (C20 (ii)) |

Kept green and checked: `tests_event_handler_attrs.rs` F-12 (`:235`), F-12b (`:446`), F-14 (`:263`); N1's C8h (outcome
unchanged, mechanism now step 3); `tests_call_listener.rs:120`; the shell E1–E8 (`content_eh_attrs_bind_tests.rs`).

### §5.3 Touched files

Sizes and paths verified 2026-10-31 against `74e15df1` by the §0.2 commands and `wc -l` on each file.

- `crates/script/elidex-script-session/src/event_handler_processing/mod.rs` — D-T1/D-T2/D-T3; deletions; harness
  (`test_support::install` / `init_fresh` take `ActiveDocument`).
- `crates/script/elidex-script-session/src/event_handler_consumer.rs` — constructor, both arms.
- `crates/script/elidex-script-session/src/scripting.rs` — D-T4 extraction.
- `crates/script/elidex-script-session/src/lib.rs` — exports.
- `crates/core/elidex-ecs/src/dom/classify.rs` — D-T5; `crates/core/elidex-ecs/src/dom/mod.rs` — delete
  `window_entity` (930 lines; a deletion, no growth).
- `crates/dom/elidex-dom-api/src/char_data/document_props.rs` — D-T5 sibling.
- `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` — accessors (745 lines + N1's changes; < 1000).
- `crates/script/elidex-js/src/vm/consumer_dispatcher.rs`, `crates/script/elidex-js/src/vm/vm_api.rs` — marshal
  `ActiveDocument` at bind.
- tests: `session/event_handler_processing/tests_target.rs` (NEW), `tests_bind.rs`, `tests_insertion_and_write.rs`
  (harness); `elidex-js/vm/tests/tests_event_handler_target.rs` (NEW), `tests_event_handler_attrs_window_delegating.rs`
  (§5.2); `crates/shell/elidex-shell/src/content_eh_attrs_bind_tests.rs` (cell T-E1; 346 lines).
- docs: the umbrella, if O-3 is answered yes (§8.4).

### §5.4 Named risks

- **R1 — W1/W2** (§1, O-1): the only outcomes worse than HEAD; pinned; closed by S-5.
- **R2 — inherited proxy facets.** Step 3 inherits the effective-node-document proxy's known imperfections
  (adopt-then-remove; a live node appended into a detached foreign subtree; their reverses; `scripting.rs:62-72`).
  At each, T3's outcome is HEAD's (Window) or the spec's; e.g. a `body` created in the active document, appended into a
  DOMParser document and removed again keeps the bound owner, so T3 targets the Window where the spec (after adoption
  into the DOMParser document) returns null — HEAD-equal. Recorded on `#11-cross-document-adopt-on-insert` (§8.3 item 3).
- **R3 — merge contact with N1** in `event_handler_attrs.rs` and `scripting.rs` (N1 narrows clause (b)'s arguments to
  `Entity`); T3 is written on N1's landed base (§9 item 1).
- **R4 — `ActiveDocument` staleness.** The consumer's copy is fixed at construction; it is rebuilt at every bind (slice a
  H13), and HEAD's bind passes the same document each time (Q-1). A bind with a different document rebuilds it.
- **R5 — `onerror` on `<body>`** now lands on the Window and compiles with the one-argument form, where the spec gives a
  Window `onerror` five parameters: `#11-event-handler-attribute-edge-cases` facet 2 becomes reachable through
  `<body onerror>` (§8.3 item 4).

## §6. Test plan

Red-first: each **red** cell is committed failing on T3's base before the fix commit. Every VM cell uses
`with_page` / `run` (`elidex-js/vm/test_helpers.rs:166`, `:191`; `dp` is a DOMParser document with an empty body, `fn`
counts its calls).

### §6.1 VM integration (`tests_event_handler_target.rs`, NEW)

| Cell | Kind | Script | HEAD | T3 (= spec unless marked) |
|---|---|---|---|---|
| T-1 | red (facet 1, IDL) | `dd = new DOMParser().parseFromString('<body></body>','text/html'); dd.body.onhashchange = fn; r.push(window.onhashchange === null, dd.body.onhashchange === null)` | `false,false` (`fn` on the Window) | `true,true` |
| T-2 | red (facet 1, attribute change steps) | `dd.body.setAttribute('onhashchange','window.hit = 1'); window.dispatchEvent(new Event('hashchange')); r.push(window.onhashchange === null, window.hit === undefined)` | `false,false` | `true,true` |
| T-3 | red (facet 1, Insertion timing; kills `owner_document`-only) | `b = document.createElement('body'); b.setAttribute('onhashchange','window.hit = 1'); window.onhashchange = fn; dp.body.append(b); r.push(window.onhashchange === fn)` | `false` (re-raw) | `true` (null target at the insertion into `dp`) |
| T-4 | red (facet 4, content attribute) | `document.body.setAttribute('onload','window.hit = 1'); r.push(typeof window.onload === 'function', document.body.onload === window.onload); window.dispatchEvent(new Event('load')); r.push(window.hit === 1)` | `false,false,false` | `true,true,true` |
| T-5 | red (facet 4, IDL through `HTMLElement.prototype`) | `document.body.onfocus = fn; r.push(window.onfocus === fn, document.body.onfocus === fn); window.dispatchEvent(new Event('focus')); r.push(fnRan === 1)` | `false,true,false` | `true,true,true` |
| T-6 | red (facet 3, content attribute) | `f = document.createElement('frameset'); f.setAttribute('onhashchange','window.hit = 1'); window.dispatchEvent(new Event('hashchange')); r.push(window.hit === 1)` | `false` | `true` |
| T-6b | guard (facet 3, non-active) | `dd = parseFromString('<frameset></frameset>','text/html'); dd.body.setAttribute('onhashchange','window.hit = 1'); r.push(window.onhashchange === null, dd.body.tagName === 'FRAMESET')` | `true,true` | `true,true` |
| T-7 | red (facets 3 + 4, IDL) | `f = document.createElement('frameset'); f.onload = fn; r.push(window.onload === fn, f.onload === fn)` | `false,true` | `true,true` |
| T-8a | guard (step 3 note) | `b = document.createElement('body'); b.onhashchange = fn; r.push(window.onhashchange === fn)` | `true` | `true` |
| T-8b | red (step 3 note, reflecting) | `b = document.createElement('body'); b.setAttribute('onresize','window.hit = 1'); r.push(typeof window.onresize === 'function')` | `false` | `true` |
| T-9 | red (step 1 namespace) | `s = document.createElementNS('http://www.w3.org/2000/svg','body'); s.setAttribute('onhashchange','window.hit = 1'); r.push(window.onhashchange === null)` | `false` (tag-only) | `true` |
| T-13 | red (D-T5 sibling) | `s = document.createElementNS('http://www.w3.org/2000/svg','body'); document.documentElement.insertBefore(s, document.body); r.push(document.body !== s, document.body.tagName === 'BODY')` | `false,false` | `true,true` |
| T-14 | **window pin W1** (O-1) | `document.body.setAttribute('onresize','window.hit = 1'); window.onresize = fn; document.documentElement.append(document.body); r.push(typeof window.onresize === 'function', window.onresize !== fn)` | `true,false` (`fn` kept) | `true,true` (re-raw over `fn`). Spec: `true,false`; slice b (S-5) inverts it |
| T-15 | **window pin W2** (O-1) | `dd = parseFromString('<body></body>','text/html'); dd.body.setAttribute('onscroll','window.hit = 1'); r.push(window.onscroll === null); document.documentElement.append(dd.body); r.push(window.onscroll === null)` | `true,true` (element-targeted) | `true,false` (delivered at the insertion). Spec: `true,true`; slice b inverts it |

### §6.2 Unit (session, `tests_target.rs`, NEW)

| Cell | Kind | Shape | Oracle |
|---|---|---|---|
| U-T1 | red (full enum) | for every `EVENT_HANDLER_ATTRS` name and every reflecting name × tag {`body`, `frameset`, `div`, foreign `body`} × node-document state {bound connected, bound detached, foreign connected (a `create_document_node()` document), foreign-owned detached, bound-owned in a foreign tree, unresolvable}: `determine_event_handler_target` | the spec table: not Window-delegating → `Some(target)`; Window-delegating and active (or unresolvable) → `Some(window)`; else `None`. HEAD's rule fails every `frameset`, reflecting, foreign-namespace and non-active WEH row |
| U-T2 | red (bind pass, facet 4) | pre-bind page with `<body onload=A>`; `init_fresh` | the Window's `load` entry is raw `A`; the body has no `load` entry. HEAD: the reverse |
| U-T3 | red (C8h's store) | foreign document's `body`, `setAttribute('onfocus', 'A')` through the consumer | no `focus` entry on the body and none on the Window. HEAD: raw `A` on the body |
| U-T4 | guard (D-5 keying) | U12's shape: bound document `d2 = create_document_node()` ≠ `document_root()`; a `body` in `d2` with `onhashchange=A`; `active = {d2, w}` | delivered to `w`; a `document_root()`-keyed step 3 returns `None` |
| U-T5 | guard (D-T4 refactor) | `scripting.rs` tests (`:117-`) | unchanged and green |

### §6.3 End to end (shell)

| Cell | Kind | Markup | HEAD | T3 |
|---|---|---|---|---|
| T-E1 | red (H-T14) | `build_main_document("<body onload=\"document.body.dataset.l=1\">…")`, then `probe_body_attr(&pipeline, "data-l")` (the E4 pattern, `content_eh_attrs_bind_tests.rs:165-181`) | `None` | `Some("1")` (Q-6) |

### §6.4 Mutation guards

| Mutant | Killed by |
|---|---|
| step 3 dropped | T-1, T-2, T-3, U-T1, U-T3 |
| step 3 by `owner_document` alone | T-3, U-T1 (bound-owned in a foreign tree) |
| step 3 keyed on `document_root()` | U-T4 |
| reflecting set dropped from step 2 | T-4, T-5, T-8b, U-T1, U-T2 |
| `frameset` dropped from step 1 | T-6, T-7, P3, U-T1 |
| class test without namespace | T-9, T-13, U-T1 |
| an IDL accessor bypassing D-T1 | T-1 (body pair), T-5 / T-7 (normal pair) |
| unresolvable → `None` | U-T1 (unresolvable row) |

## §7. Out of scope and slots

| Item | Why not T3 | Home |
|---|---|---|
| X-W's re-raw at insertion; the D-8 restriction; W1/W2 | slice a's interim timing (D-T7) | slice b S-5; recorded on `#11-eh-attrs-creation-time-call-sites` defect (7) (§8.3 item 2) |
| template contents report the bound document (C18 keeps HEAD's outcome) | C1 | `#11-template-contents-owner-document` |
| insertion adoption; R2's facets | DOM §4.2.3 | `#11-cross-document-adopt-on-insert` (§8.3 item 3) |
| **the `HTMLFrameSetElement` interface**: a `frameset` has no `HTMLFrameSetElement.prototype`, so its 18 WindowEventHandlers IDL attributes and `cols` / `rows` read `undefined` (after T3 its content attributes and the inherited reflecting six act on the Window) | an interface surface, not target determination; core vs compat placement of an obsolete element (HTML §16.3) is a decision | proposed slot `#11-htmlframesetelement-interface` (§8.1; O-2) |
| **attribute change steps step 1**: a handler name that is not an event handler content attribute *of that element* (a WindowEventHandlers name on a non-`body` / non-`frameset` element, a Document-only name on any element) still creates a handler on the element (e.g. `div.setAttribute('onhashchange', X)`, then `hashchange` dispatched at the `div` runs `X`); spec: return | a different step of a different algorithm; T3 does not read it | proposed slot `#11-event-handler-content-attribute-applicability` (§8.1) |
| attribute change steps step 5.1 (CSP) | not modelled | `#11-csp-inline-handler-enforcement` |
| multi-realm step 4 | B1 | hand-off (§8.2) |
| the ASCII-case-insensitive tag compare (an XML-document `createElementNS(html, 'BODY')` counts as `body`) | engine-wide tag-predicate convention (`is_base_element` precedent) | not changed; noted |

Own deferrals: **0** (W1/W2 are recorded under the program's existing slot, pending O-1).

## §8. Edges, hand-offs, records

### §8.1 Proposed slot texts (for the PM; defects only)

- **`#11-htmlframesetelement-interface`** — pre-existing. elidex has no `HTMLFrameSetElement` interface: a `frameset`
  element's wrapper has no `HTMLFrameSetElement.prototype` (prototype selection by tag, `elements.rs:264`, has no
  `frameset` arm), so the 18 `WindowEventHandlers` IDL attributes (`HTMLFrameSetElement includes WindowEventHandlers`,
  HTML §16.3.2 Frames) and the reflected `cols` / `rows` read `undefined`. After T3 a `frameset`'s handler content attributes and its inherited GlobalEventHandlers accessors
  for the reflecting six act on the Window. **Owner**: PM to route; core vs compat placement (obsolete element, §16.3)
  is part of the decision. **Trigger**: a PR adding HTML element interfaces, or a compat hit. ⚠ No remedy reviewed.
- **`#11-event-handler-content-attribute-applicability`** — pre-existing. The attribute change steps' step 1 (HTML
  §8.1.8.1) returns when the local name is not an event handler content attribute *on that element*; elidex's consumer
  looks the name up in `EVENT_HANDLER_ATTRS` regardless of the element (`event_handler_attr_lookup`,
  `event_handler_processing/mod.rs:237`), so a WindowEventHandlers name on an element other than `body` / `frameset`, or a
  Document-only name on any element, installs a handler on that element, and an event of that type dispatched at it
  runs the attribute. **Owner**: EH-attrs program (Layout lane) or PM. **Trigger**: slice b's plan (it rewrites the
  processing entry), or a WPT event-handler subset. ⚠ No remedy reviewed.

### §8.2 Hand-offs

- **B1** (`docs/plans/2026-06-agent-scoped-ecsdom-world.md` §5 req 5 / req 7): `ActiveDocument` is the single-browsing-
  context form of steps 3–4. Under a multi-realm World, "active" and "relevant global" become per document; B1's plan
  replaces `ActiveDocument`'s two marshalling points (the consumer's construction in `Vm::bind`, the VM accessor
  helper) with a per-document resolution, and `determine_event_handler_target`'s body; no caller changes. BFCache
  (a document kept non-active with its Window) is the case that makes this a real distinction.
- **N4** (node memo §0.4 D8, §4 "Node document"): `effective_node_document` exists from T3 as a session function with
  both readers; N4's move into `EcsDom` relocates one function and its two calls.
- **Slice b**: S-5's removal of X-W inverts T-14, T-15 and C17's remaining pin; S-1's creation-time processing reads
  `ActiveDocument` at each creation path (a DOMParser `body` then gets `None`, closing x3's reason).

### §8.3 Records for existing slots (for the PM, exact texts)

1. `#11-event-handler-target-determination` — **at T3's landing: close**, with this record: "Closed by T3 (#<PR>,
   `<squash>`): facets (1)–(4) realised by `determine_event_handler_target`, the only caller of steps 1–4 (attribute
   change steps, IDL getter and setter). Residuals homed elsewhere: template contents → `#11-template-contents-owner-document`;
   proxy facets of step 3 → `#11-cross-document-adopt-on-insert`; multi-realm step 4 → B1; the `frameset` interface →
   `#11-htmlframesetelement-interface`." **Before landing (any time)**: in the first paragraph, "(`…/event_handler_processing/mod.rs:249`)
   … code as of `fd6955e8`" → "(`crates/script/elidex-script-session/src/event_handler_consumer.rs:328` at `fd6955e8`;
   `event_handler_processing/mod.rs:249` since #563)", and "in three ways" → "in four ways (facet (4) below)", and
   "**Owner**: the IBP-sandbox program (Layout lane), since its N-series rewrites the handler path." → "**Owner**: the
   EH-attrs-at-creation program (Layout lane), T3 (PM decision 25; sub-bullet below)." (P-T1, P-T2.)
2. `#11-eh-attrs-creation-time-call-sites` defect (7) — **at T3's landing**, replace "(7) a `<body>` / `<frameset>`
   handler that delegates to the Window (X-W) is re-raw'd at every insertion over any write, and a `<body>` outside an
   active document delivers it to the page Window." by "(7) a `<body>` / `<frameset>` handler that delegates to the
   Window (X-W) is re-raw'd onto the Window at every insertion of the element into the active document, over any write
   (since T3 this includes the Window-reflecting six on `<body>` and every Window-delegating name on `<frameset>`, which
   HEAD re-raw'd onto the element: a `window.on<T>` write is now overwritten by a later insertion of that element, T3
   memo W1), and such an attribute set while the element was outside an active document is delivered to the page
   Window at its first insertion into the active document (W2; spec: nothing)." Any N1 qualifier on (7) applied at N1's
   landing (N1 memo §8 item 9 (ii), "(7)'s pairs that HEAD targets at the element") becomes vacuous at T3: no
   Window-delegating pair is element-targeted after T3; the PM drops that clause. (Conditional on O-1.)
3. `#11-cross-document-adopt-on-insert` — add to its consumer list: "the event-handler target determination's step 3
   (T3, `determine_event_handler_target`, through `effective_node_document`); its proxy facets give HEAD's target
   (the page Window) or the spec's."
4. `#11-event-handler-attribute-edge-cases` — append to facet (2): "Reachable through `<body onerror>` since T3 (the
   reflecting set now targets the Window)."
5. Umbrella K-0's E-G1 (i) and the ledger row's facet (4) bullet: closed by T3 (the record of item 1 covers both).

### §8.4 Umbrella text touched (only if O-3 is yes)

X-W *Status* (§0.5), §1.5's X-W bullet, §7 G3 / G4, §8.1's first bullet: "T3 then slice b close it" → T3 closed the
target half (facets 1–4); X-W's timing and W1/W2 remain to S-5.

## §9. Definition of done

1. T3's branch is cut from main after N1's implementation lands; every `74e15df1` coordinate in §0.2 is re-measured
   there by its command and the memo's coordinates updated before implementation starts.
2. `/elidex-plan-review` passes on this memo (mandatory, §1).
3. Red-first: T-1–T-9, T-13, U-T1–U-T3, T-E1 committed failing; the fix makes them and §5.2's rewritten cells green;
   T-14 / T-15 assert T3's outcome with a comment naming S-5 as the inverter (subject to O-1).
4. `/usr/bin/grep -rn 'resolve_handler_target\|is_body_element' crates` and `/usr/bin/grep -rn 'fn window_entity' crates/core/elidex-ecs`
   → 0 hits (`HostData::window_entity` and the `ScriptEngine` trait method stay); `determine_event_handler_target(` → the definition and the five call
   sites of §4.2.
5. `cargo test -p elidex-script-session -p elidex-js -p elidex-ecs -p elidex-dom-api -p elidex-shell --all-features`
   green; `mise run ci` before push.
6. §8.1 / §8.3 texts handed to the PM in the PR description.

## §10. Open questions

**For the PM / user:**
- **O-1** — W1/W2 (§1): T3 lands with outcomes worse than HEAD on one write path (a `window.on<T>` write overwritten
  by a later insertion of a `body` / `frameset` carrying the attribute, for the pairs HEAD targeted at the element), and
  W2. Recommended: accept, as X-W's existing class extended to the newly Window-targeted pairs, closed by S-5; the user's
  option text for X-W said `<body onload>` becomes correct "T3 か 2 本目の PR のとき". Rejected alternatives: a
  timing-dependent target (two target rules), and removing X-W for the six alone (a partial S-5). If not accepted, T3
  waits for slice b or absorbs S-5's X-W removal (a re-slice of the umbrella).
- **O-2** — `#11-htmlframesetelement-interface`: register, owner, core or compat.
- **O-3** — does T3's PR carry the umbrella's in-place edits of §8.4 (as N1's PR did under PM decision 93)?
- **O-4** — confirm D-T3 (marshalled `ActiveDocument`, no component before B1) against the side-store rule and ruling (iv).
- **O-5** — confirm D-T5's `document.body` sibling in T3 (one predicate) rather than a separate PR.

**Facts needed before plan-review (scout):**
- **Q-1** — at `74e15df1`: is `Vm::bind`'s `document` always the one document of its `EcsDom` that has a browsing
  context; can one `EcsDom` hold two `NodeKind::Window` entities or two bound documents; does any navigation reuse a
  `Vm` / Window with a new document; are same-origin, srcdoc and about:blank iframes always a separate `EcsDom` + `Vm`?
- **Q-2** — which production element-creation paths leave `AssociatedDocument` unset (the unresolvable case of D-T4)?
- **Q-3** — which prototype a `frameset` wrapper gets (`HTMLElement` or `HTMLUnknownElement`), so T-7's `f.onload`
  reaches `HTMLElement.prototype`'s accessor.
- **Q-4** — is `tag_specific_html_prototype` reached for a foreign-namespace element (does an SVG `body` get
  `HTMLBodyElement.prototype`)? If so, T-9's IDL variant needs the brand check aligned (out of D-T5 today).
- **Q-5** — does `MutationEvent::AttributeChange` carry a namespace (attribute change steps step 1's "namespace is not
  null"), and is that recorded anywhere?
- **Q-6** — does `build_main_document` in `content_eh_attrs_bind_tests.rs` run the document `load` dispatch
  (`pipeline.rs:470`) before returning, so T-E1 observes `onload`?

No browser measurement is needed: every outcome in §6 is spec-defined (the unresolvable node document is an elidex
state with no spec counterpart, decided by D-T4's direction rule).
