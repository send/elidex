# C0c `EH-attrs-at-creation-C0c` — the adoption notification (`adoptedCallback`) and `document.adoptNode` (per-PR plan memo)

Per-PR slice **C0c** of the C0 program (program memo `docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md`, rev 3.1;
"the program memo"), under the umbrella `docs/plans/2026-10-event-handler-attrs-at-creation.md`. It carries program
decisions **D-3 (c), D-6, D-13, D-19**, after its prerequisite pure-move splits **D-14 (b), (d), (e)** ((d) / (e) unless L3
RC-0a has done them), on C0b's head. It closes
slot `#11-custom-elements-adopted-callback` (D-17 #227) and, as the program's last PR, slot
`#11-cross-document-adopt-on-insert` (program R-1, R-2). Program IDs are cited as the program memo's; C0b's as the C0b
memo's (`docs/plans/2026-11-eh-attrs-c0b-node-document-fact.md`, "the C0b memo").

**Placement.** PM 158 (`adoptNode` in the C0 program); PM 244 (C0c = D-3 (c), D-6 with its consumer arms and the VM
flush, and D-13, after D-14 (b); any time after C0b and before C1). The `Adopt` variant ships with its consumer (no
consumer-less event).

**Scope.**
1. The adopt walk records one contents segment per template contents fragment it reaches (D-3 (c)).
2. `MutationEvent::Adopt` (NEW) with `AdoptedContents` (NEW), dispatched once per adopt call after the walk and before
   the link (D-6).
3. The custom-element consumer's `Adopt` arm and the VM flush's `adoptedCallback` invocation (D-6); explicit no-op
   arms in the exhaustive matches (`range/live.rs`, `mutation_hook.rs`).
4. `TemplateContentsWalk::Include` in the spec's order (D-19).
5. `document.adoptNode(node)` (D-13): VM native → dom-api `AdoptNode` → session `apply_adopt_node` → `EcsDom::adopt`.

**Not in scope.** `importNode` (proposed `#11-import-node`); registry steps 3.2 / 3.3.2 — scoped half `#11-shadow-scoped-custom-element-registry`, non-scoped half (3.3.2: a null-registry document's element adopted into a registry-bearing one) L3's proposed `#11-document-null-registry-adopt-reset`, which hooks this PR's `Adopt` (§7); the `img` adopting steps
(proposed `#11-img-adopting-steps-relevant-mutations`, whose hook this PR provides); the engine-wide reaction drain
point (`#11-ce-reactions-method-scope-drain`, L3's proposal); C1's `f` values of the segments (C1 D-C5, T25).

**Changelog.** *Rev 3.1* (2026-11-03, with the program memo's rev 3.1): §1 states why `adoptNode` stays in this PR (R2
Ax3 MIN-7); D-14 (e)'s L3 side is RC-0a (L3 #583 `55d35a16`). *Rev 1* (2026-11-03): first draft, carved from the C0 memo rev 2.1 (`d262ca8c`) D-3 (c) / D-6 / D-13 /
T6, T7, T14–T16, T18 / U13 per plan-review R1's SPLIT and PM 244; D-19 decided (R1 Ax1 MIN-2); §4.7's layering table
added (Ax1 MIN-1); `[CEReactions]` drain point stated (Ax4 MIN).

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Coordinates are `27b50dfe` (program §0.1). C0c's base is C0b's head plus D-14 (b), which moves
`crates/script/elidex-js/src/vm/host/document.rs`'s node factories (`:278-421`) to `vm/host/document_factories.rs`
(NEW) and its accessors (`:423-938`) to `vm/host/document_accessors.rs` (NEW). L3 RC-2a may have landed (edge c0e);
its files (`crates/core/elidex-ecs/src/dom/mutation_event.rs`, `crates/dom/elidex-custom-elements/src/consumer.rs`,
`crates/dom/elidex-dom-api/src/range/live.rs`) are in the re-measure scope (§9 item 1).

### §0.2 Premises

Program H-9, H-12, H-15, H-16 bind this PR. Added:

| id | claim | evidence (`27b50dfe`) |
|---|---|---|
| H-c1 | The CE consumer's dispatch entry | `CustomElementReactionConsumer::handle` (`crates/dom/elidex-custom-elements/src/consumer.rs:75-96`): `Insert`, `Remove`, `AttributeChange`, `_ => {}`; `handle_insert` `:98` |
| H-c2 | The flush's reaction loop and its callback helper | `flush_ce_reactions` (`crates/script/elidex-js/src/vm/host/custom_elements/flush.rs:39`); the `AttributeChanged` arm calls `invoke_callback(&mut ctx, entity, cb_sid, &args)` (`:131`); the `Adopted` arm is a no-op (`:133-138`) |
| H-c3 | Where reactions drain | `flush_ce_reactions` is called at script-execution and event checkpoints: `crates/script/elidex-js/src/vm/interpreter.rs:54`, `crates/script/elidex-js/src/engine.rs:165`, `crates/script/elidex-js/src/vm/natives_timer.rs:281`, `crates/script/elidex-js/src/vm/host/custom_elements/define.rs:204` (`git grep -n 'flush_ce_reactions()' -- crates/script`); not at a `[CEReactions]` method's return (L3 memo Rev 5, E2 measurement) |
| H-c4 | The VM → engine-independent dispatch | `invoke_dom_api` (`crates/script/elidex-js/src/vm/host/dom_bridge.rs:462`) calls a registered dom-api handler; handlers push session records (`push_records`, e.g. `crates/dom/elidex-dom-api/src/element/tree.rs:285`) |
| H-c5 | Custom elements inside template contents exist | P-4 (C1 memo P-C14): an element upgraded in a document and appended into `tpl.content` stays custom (adoption does not un-upgrade), so per-node pairs are load-bearing |

### §0.3 Premise errors

- **P-4 (program, restated).** "Elements in contents can be custom only if upgraded there" (C0 rev 1) was wrong (H-c5);
  folded as per-segment pairs (D-3 (c), D-6).
- **P-c1 (rev 2 Q-1, now decided).** One event per contents segment would follow the spec's structure (each template's
  adopting steps are a nested *adopt*), but a consumer would then emit contents callbacks after every outer one, out of
  spec order; one event per adopt call carrying all segments lets the consumer walk once in spec order (D-6).

### §0.4 Decisions (this PR; full statements in the program memo)

- **D-3 (c) — segments.** The walk (C0b's private helper in `tree/mutation.rs`) collects, for each contents fragment it
  reaches, `AdoptedContents { fragment, documents }` with `old` = the fragment's node document read before the walk
  writes it and `new` = the target it writes (C2: `document`; C1: `f(document)`), `documents = None` when `old` is
  `None` or equals `new` (the nested adopt's step 3 guard), in walk order.
- **D-6 — the event and its consumers.** `MutationEvent::Adopt { node, documents: Option<(Entity, Entity)>, contents:
  &'a [AdoptedContents] }`; `adopt` dispatches it after the walk, before returning to the primitive (so before
  `link_node` and `Insert`), iff `documents` or any segment's `documents` is `Some`. **Consumer:**
  `CustomElementReactionConsumer::handle` gains `MutationEvent::Adopt { .. } => self.handle_adopt(..)`, which walks
  `node`'s subtree with `for_each_uncapped_shadow_inclusive(node, TemplateContentsWalk::Include, …)` (spec order after
  D-19), tracks the innermost segment it is inside (entering a contents fragment selects that segment's pair; leaving
  it restores the enclosing pair), and enqueues `CustomElementReaction::Adopted { entity, old_document, new_document }`
  for each element in `CEState::Custom` whose current pair is `Some` (*adopt* 3.3.3: "enqueue a custom element callback
  reaction with inclusiveDescendant, callback name "adoptedCallback", and « oldDocument, document »"). **Flush:** the
  `Adopted` arm (`flush.rs:133-138`) resolves the two Document wrappers and calls `invoke_callback(&mut ctx, entity,
  adopted_callback_sid, &[old, new])` (H-c2), with the same error routing as `connectedCallback`. **Other consumers:**
  `range/live.rs:113-163` (the one exhaustive production match) gains an explicit no-op `Adopt` arm (live ranges are
  updated by adopt step 2's `Remove`); the test match `crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs:68-148`
  likewise; the others have `_ => {}` (program P-6). *Batch equivalence:* the spec interleaves "set node document" and
  "enqueue" per descendant; enqueueing runs no script (reactions run at the next drain, H-c3), so re-homing first and
  enqueueing after, in the same order, is unobservable. *Idiom table:* OO engines' per-node-class "moved to new
  document" virtual override (`adoptedCallback` dispatch inside the adopt loop) → one event on the mutation stream and
  one consumer arm per system that cares, as `connectedCallback` already is.
- **D-19 — the walker's order.** In `for_each_uncapped_shadow_inclusive` (`crates/core/elidex-ecs/src/dom/tree/walkers.rs:99`),
  `Include` pushes the contents fragment **after** the light children and the shadow root (so it pops first, right
  after the template node), instead of before them (`:112-118`); its docstring (`:86-91`) changes from "visited after
  the template's own subtree" to "visited right after the template (DOM §4.5 *adopt* step 3.4 runs the template's
  adopting steps inside the per-descendant loop)". `despawn_subtree` (`teardown.rs:197`) and the adopt walk are
  order-insensitive (H-16), so only the consumer observes it (U13, U18).
- **D-13 — `document.adoptNode(node)`.** *VM* (marshalling): `native_document_adopt_node` (NEW) in
  `vm/host/document_factories.rs` (NEW after D-14 (b)), registered on `Document.prototype`; brand-checks `this`,
  unwraps `node` (non-Node → `TypeError` per WebIDL), calls `invoke_dom_api("adoptNode", …)`, maps a `DomApiError` to
  its DOMException, returns the same wrapper. *dom-api*: `AdoptNode` (NEW) in `crates/dom/elidex-dom-api/src/document.rs`,
  registered in `registry.rs`: step 1 a Document → `NotSupportedError`; step 2 a ShadowRoot → `HierarchyRequestError`;
  step 3 `push_records(session, apply_adopt_node(dom, node, this))`; step 4 return node. A host-linked DocumentFragment
  (template contents) is adoptable: DOM's `adoptNode` has no exclusion (`webref body dom dom-document-adoptnode`, run
  2026-11-03). *Session*: `apply_adopt_node(dom, node, document) -> Vec<MutationRecord>` (NEW) in
  `mutation/child_list.rs`: if `node` has a parent, capture `source_removal_record(dom, node)` (H-7) pre-write; call
  `dom.adopt(node, document)`; return the record (adopt step 2's *remove* queues it; adoption itself queues none).
  *`[CEReactions]`* (`webref idl dom Document`: `[CEReactions] Node adoptNode(Node node)`): the reactions it enqueues
  (`Adopted`, and `Disconnected` from step 2's removal) drain at the next checkpoint like every other `[CEReactions]`
  method's (H-c3); the method-scope drain is the engine-wide gap L3 proposes as `#11-ce-reactions-method-scope-drain`
  (L3 memo §9, PM 224), not this PR's.
- **D-14 (b), (d), (e) — prerequisites.** As program D-14: (b) `vm/host/document.rs` (1197); (d) `range/live.rs` (1021):
  its inline test module (from `:498`) to a sibling file — L3 RC-0a's planned move, done by whichever lands first; (e)
  `dom/tests/mutation_hook.rs` (1143): the specialised-hook scenarios from the `:973` section rule to a sibling file —
  done by whichever of L3 RC-0a and C0c's prereq lands first (RC-0a's scope since L3 `55d35a16`). Each its own PR (or commit of L3's), a pure move.

## §1. Shape and verdict

**Axes (program §2): A4 notification, with A1, A2, A3, A5, A7, A8, A9 at the 8 pairs marked C0c.** Edge-dense as a
slice of the program; plan-review mandatory; a terminal unit once passed (base case (c)). Why `adoptNode` travels with
the notification (program §1; the coupling is ordering, not mechanism): before the notification it would ship a new
`[CEReactions]` method without its callback (T15); after it, as its own PR, it would carry no mechanism of its own (one
session function over C0b's `adopt` plus marshalling, D-13) and no invariant the notification does not share — its
pairs A1×A9 and A4×A9 are pinned by T15 / T16, which need both halves; PM 158 put it in the program as the adopt
algorithm's own entry. Splitting it buys no smaller decision surface for one more per-PR memo and review.

**K-0 lens.** Adds `adoptedCallback` and `adoptNode` where HEAD has none (R-5). Same-document moves enqueue nothing
(T14).

## §2. Coupled invariants (program §2.2 pairs with PR = C0c)

| pair | intersection | pinned by |
|---|---|---|
| A1×A4 | each pair's `old` is a stamp read before the walk, `new` the stamp written; the consumer reads no tree position | U13, T18 |
| A1×A9 | `adoptNode`'s record is the removal's; the stamp change is no record | T16 |
| A2×A4 | `Adopt` precedes `Insert`; in `replace_child`, `Adopt(new_child)` precedes `Remove(old_child)` | U4 (C0c part), T7 |
| A3×A4 | the consumer visits the walk's set in spec order (D-19), each node with its segment's pair | U13, U18, T18 |
| A4×A5 | builds adopt nothing (C0b D-10) and run with the dispatcher taken, so no `adoptedCallback` from a build | U9 (C0b) re-run with an `Adopt` counter |
| A4×A7 | `adoptedCallback` runs at the next drain, after the insertion completes; slice a's processing is inside `Insert` | T6 |
| A4×A8 | after C0b every script-reachable custom element has a node document, so the silent re-home cannot swallow a callback | U2 (C0b) re-run: no `Adopt` for an ownerless root |
| A4×A9 | `adoptNode` fires `Adopt` like an insertion's adoption; removal record, then `Adopted` | T15, T16 |

**I-A4 (axis invariant).** `Adopt` fires once per adopt call iff some pair is a (live, different) document pair, before
that root's `Insert`; every custom element of the adopted subtree whose pair is `Some` gets exactly one `Adopted`
reaction with its own pair, in spec order (contents right after their template), enqueued before its `Connected`.

## §3. Spec coverage map

Lookups (2026-11-03): `heading dom 4.5 / 4.8 / 4.3.2`; `heading html 4.12.3 / 4.13.6`; `body dom concept-node-adopt`
(3, 3.3.3, 3.4), `dom-document-adoptnode` (1–4); `dfn html 'enqueue a custom element callback reaction'` → §4.13.6;
`dfn html CEReactions` → §4.13.6 `#cereactions`; `dfn dom 'shadow-including tree order'` → §4.8; `idl dom Document`.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §4.5 Interface Document | *adopt* 3.3.3 | custom → enqueue `adoptedCallback` « oldDocument, document »: one event per adopt call, each node its own pair (the root's outside contents; a segment's inside; the innermost for nested contents) | `MutationEvent::Adopt` (NEW) in `crates/core/elidex-ecs/src/dom/mutation_event.rs:107`; consumer `crates/dom/elidex-custom-elements/src/consumer.rs:75` | ✓ | yes |
| WHATWG DOM §4.5 Interface Document | *adopt* 3.4 (template adopting steps) | each contents fragment's (old, new) recorded where the walk writes it | the walk's contents edge (C0b's private helper in `crates/core/elidex-ecs/src/dom/tree/mutation.rs`) | ✓ | yes |
| WHATWG DOM §4.5 Interface Document | `adoptNode` 1–4 | Document → NotSupportedError; ShadowRoot → HierarchyRequestError; adopt; return node | dom-api `AdoptNode` (NEW) in `crates/dom/elidex-dom-api/src/document.rs`; session `apply_adopt_node` (NEW); VM native (NEW) | ✓ | yes |
| WHATWG DOM §4.3.2 Queuing a mutation record | adopt step 2's *remove* record | the source record of `adoptNode` on a parented node | `source_removal_record` `crates/script/elidex-script-session/src/mutation/mod.rs:341` | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *shadow-including tree order*; adopt 3.4's position | contents right after their template | `for_each_uncapped_shadow_inclusive` `crates/core/elidex-ecs/src/dom/tree/walkers.rs:99` (D-19) | ✓ | no |
| WHATWG HTML §4.13.6 Custom element reactions | *enqueue a custom element callback reaction*; `[CEReactions]` | `adoptedCallback` with « oldDocument, document »; drain point | consumer arm; flush `crates/script/elidex-js/src/vm/host/custom_elements/flush.rs:133` | ✗ (method-scope drain: L3's proposed slot) | yes |
| WHATWG HTML §4.12.3 The template element | *adopting steps* 1–2 | the contents' pair (C2: the template's; C1 sets `new` = `f`) | segment recording (D-3 (c)) | ✗ (C1's `f`) | yes |

### §3.1 User-input touch audit

Two new script entry points: `adoptedCallback` (author code, through the existing flush and error routing) and
`document.adoptNode` (author argument, marshalled into the engine-independent path). `Adopt` carries entities only.

## §4. Mechanism

### §4.2 Reconcile — C0b's head → C0c

| site | C0b | C0c |
|---|---|---|
| `adopt` (`tree/mutation.rs`) | walk, no event | walk + segments; `Adopt` iff some pair |
| `mutation_event.rs:107` | seven variants | + `Adopt`, `AdoptedContents` |
| `walkers.rs:112-118` | contents after the subtree | contents right after the template |
| `consumer.rs:75-96` | no `Adopt` arm | `handle_adopt` |
| `range/live.rs:113-163`; `mutation_hook.rs:68-148` | exhaustive without `Adopt` | explicit no-op arm |
| `flush.rs:133-138` | no-op | `adoptedCallback(old, new)` |
| `vm/host/document_factories.rs` (after D-14 (b)) | — | `adoptNode` native |
| dom-api `document.rs`, `registry.rs` | — | `AdoptNode` |
| session `mutation/child_list.rs` | — | `apply_adopt_node` |

### §4.7 Layering: VM site → engine-independent API

| VM site (marshalling only) | engine-independent API | engine call |
|---|---|---|
| `adoptNode` native (NEW) | `invoke_dom_api("adoptNode")` (`dom_bridge.rs:462`) → dom-api `AdoptNode` (NEW) | session `apply_adopt_node` (NEW) → `EcsDom::adopt` |
| flush `Adopted` arm (`flush.rs:133-138`) | `invoke_callback` (`flush.rs:131`'s helper) with two Document wrappers | — |

No algorithm in `vm/host/`. The consumer arm lives in `elidex-custom-elements` (engine-independent), beside
`handle_insert`.

### §4.8 Docstring citation table

| symbol | citation |
|---|---|
| `MutationEvent::Adopt`, `AdoptedContents` (NEW) | DOM §4.5 *adopt* steps 3.3.3, 3.4 (`#concept-node-adopt`); HTML §4.12.3 template *adopting steps* |
| `handle_adopt` (NEW) | DOM §4.5 *adopt* 3.3.3; HTML §4.13.6 *enqueue a custom element callback reaction* (`#enqueue-a-custom-element-callback-reaction`); DOM §4.8 *shadow-including tree order* |
| flush `Adopted` arm (doc) | HTML §4.13.6 (`adoptedCallback` « oldDocument, document ») |
| `TemplateContentsWalk::Include` (doc) | DOM §4.8 *shadow-including tree order* (`#concept-shadow-including-tree-order`); DOM §4.5 *adopt* 3.4 |
| `AdoptNode`, `apply_adopt_node`, the native (NEW) | DOM §4.5 `adoptNode(node)` steps 1–4 (`#dom-document-adoptnode`); `[CEReactions]` HTML §4.13.6 (`#cereactions`) |
| `range/live.rs` `Adopt` arm | DOM §4.5 *adopt* step 2 (the removal already updated live ranges) |

### §4.9 Comment sites

The flush's "v1 single-document VM never enqueues Adopted — the slot is reserved by
`#11-custom-elements-adopted-callback`" (`flush.rs:134-137`, false after C0c); the consumer's "other variants are
no-ops" (`consumer.rs:73-74`); `walkers.rs:86-91` (D-19) and `:112-113` ("Pushed first, so popped last"); the
`CustomElementReaction::Adopted` docstring (`reaction.rs:21-25`) if it names the slot.

## §5. Populations, touched files, risks

- **P-6** (program §5): `MutationEvent` consumers, 11 files; re-run at C0c's base; if RC-2a has landed, its
  `CloneCreated` arms are in the same matches (c0e).
- **P-c1 Include callers**: `git grep -n 'TemplateContentsWalk::Include' -- 'crates/*.rs'` → `teardown.rs:197`, the
  adopt walk, and (C0c) the consumer; each caller's order-sensitivity stated.
- **P-c2 adoptNode reach**: `/usr/bin/grep -rn 'adoptNode\|adopt_node' crates --include='*.rs'` → 0 at `27b50dfe`
  (H-15); at C0c's head the native, the handler, the session function, tests.

| file | lines at `27b50dfe` | change | 1000-line decision |
|---|---|---|---|
| `crates/core/elidex-ecs/src/dom/mutation_event.rs` | 352 | `Adopt`, `AdoptedContents` (NEW) | < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/mutation.rs` | 467 (≈ 560 at C0b) | segments, dispatch | < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/walkers.rs` | 392 | D-19 | < 1000 |
| `crates/dom/elidex-custom-elements/src/consumer.rs` | 329 | `handle_adopt` | < 1000 |
| `crates/dom/elidex-dom-api/src/range/live.rs` | 1021 | no-op arm | **split first** (D-14 (d), unless L3 RC-0a did) |
| `crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs` | 1143 | no-op arm in the recorder | **split first** (D-14 (e), unless L3 RC-0a did) |
| `crates/script/elidex-js/src/vm/host/custom_elements/flush.rs` | 237 | the arm | < 1000 |
| `crates/script/elidex-js/src/vm/host/document.rs` | 1197 | **split first (D-14 (b))**; native in `document_factories.rs` (NEW) | prereq PR |
| `crates/dom/elidex-dom-api/src/document.rs` 665, `registry.rs` 785 | as listed | `AdoptNode` | < 1000 |
| `crates/script/elidex-script-session/src/mutation/child_list.rs` (NEW: D-14 (a)) | (C0a's base) | `apply_adopt_node` | re-measure |
| `crates/core/elidex-ecs/src/dom/tests/adopt_on_insert.rs` (NEW: C0b), `crates/script/elidex-js/src/vm/tests/tests_insertion_adoption.rs` (NEW: C0b) | — | cells | re-measure |

**Risks.** R-5 `adoptedCallback` and `adoptNode` run / reach author code where HEAD ran none (spec). R-10 the
`MutationEvent` census with RC-2a (c0e).

## §6. Test plan

**Red-first** as the C0b memo §6. `dp`, `dp2` as there.

### §6.1 VM integration (`tests_insertion_adoption.rs`)

| cell | kind | script | base (C0b's head) | C0c = spec |
|---|---|---|---|---|
| T6 | red | `log = []; customElements.define('x-a', class extends HTMLElement { adoptedCallback(o, n) { log.push(['a', o === document, n === dp]) } connectedCallback() { log.push(['c', this.ownerDocument === dp]) } }); e = document.createElement('x-a'); dp.body.append(e); log` | `[['c', true]]` (C0b adopts; no callback) | `[['a', true, true], ['c', true]]` |
| T7 | red | continue T6 with a `disconnectedCallback` pushing `'d'`: `log = []; document.body.append(e); log`; variant `document.body.replaceChild(e2, old)` with `old` custom in `document` and `e2` custom in `dp` | `['d', 'c']` | `['d', ['a', …], 'c']`; in the variant `Adopted(e2)` precedes `Disconnected(old)` |
| T14 | guard (K-0) | same-document move: `q = document.createElement('x-a'); document.body.append(q); log = []; document.body.prepend(q); log` | `['d', 'c']` | `['d', 'c']` (no `Adopted`) |
| T15 | red | `n = dp.getElementById('d'); r = document.adoptNode(n); [r === n, n.ownerDocument === document, n.parentNode]`; `document.adoptNode(dp)` → `NotSupportedError`; `document.adoptNode(h.shadowRoot)` → `HierarchyRequestError`; a custom `x-a` adopted → `adoptedCallback(dp, document)` | `TypeError` (H-15) | `[true, true, null]`; the two DOMExceptions; the callback |
| T16 | red | `MutationObserver` on `d`'s old parent in `dp`, `{childList: true}`: `document.adoptNode(d); mo.takeRecords()` | `TypeError` | one record: `removedNodes [d]`, target the old parent, siblings pre-removal |
| T18 | red (per-node pairs; P-4) | `t = dp.createElement('template'); dp.body.append(t); e = document.createElement('x-a'); t.content.append(e); document.adoptNode(t.content); log = []; dp2.body.append(t); log` | `TypeError` | `e`'s callback has `old = document` (its contents' node document), `new = dp2` (C2), not the root pair (`dp`, `dp2`); C1 turns `new` into `dp2`'s inert document |

### §6.2 Unit

| cell | kind | shape | expected |
|---|---|---|---|
| U4 | red (`adopt_on_insert.rs`) | C0b's U4 recorder | `Remove(old parent)`, `Adopt`, `Insert`; replace: `Remove(new_child's old parent)`, `Adopt`, `Remove(old_child)`, `Insert` |
| U5 | guard | same-document move | no `Adopt` |
| U2′ | guard | C0b's U2 (ownerless root under `D`) | no `Adopt` (silent re-home) |
| U10 | red | `EcsDom::adopt(fragment, D)` on a parentless fragment with children | one `Adopt`, `documents = Some((X, D))` |
| U13 | red (order; `elidex-custom-elements`) | a template with a custom light child `L` and a custom element `C` in its contents, adopted | reactions `C` then `L`; `C` with its segment's pair |
| U18 | red (D-19; `elidex-ecs`) | `for_each_uncapped_shadow_inclusive(root, Include, …)` over a template with contents and light children, and a shadow host | template, its contents subtree, then its light children; host, its shadow tree, its light children |
| U19 | guard | nested contents: a template inside contents of a template, a custom element in the inner contents | the inner element gets the innermost segment's pair |
| U9′ | guard (×2) | C0b's U9 with an `Adopt` counter | 0 |

### §6.3 Mutation guards

| mutation | killed by |
|---|---|
| CE arm missing | T6, U13 |
| `Adopted` after `Connected` / `Adopt` after `Insert` | T6, T7, U4 |
| root pair used for contents | T18, U13, U19 |
| contents visited after the template's subtree | U13, U18 |
| `Adopt` on a same-document move / for an ownerless root | T14, U5, U2′ |
| `adoptNode` without the removal record | T16 |
| `adoptNode` accepts a Document or ShadowRoot | T15 |

### §6.4 Kept green

C0b's §6 cells; `cargo test --all-features -p elidex-ecs -p elidex-custom-elements -p elidex-dom-api -p
elidex-script-session -p elidex-js`; `mise run ci`.

## §7. Out of scope

Program §7: `importNode`, the scoped registries, the `img` adopting steps (this PR provides its hook: the `Adopt` event),
the method-scope reaction drain.

| item | where | defect |
|---|---|---|
| adopt step 3.3.2, non-scoped half | L3's proposed `#11-document-null-registry-adopt-reset` (L3 Rev 6.1 H8, PM 249); it hooks `MutationEvent::Adopt` (D-6), so its consumer arm lands after this PR | an element from a null-registry document adopted into a registry-bearing one keeps the null registry |

## §8. Edges and records

| # | edge | kind | note |
|---|---|---|---|
| c0-pre-b | D-14 (b), (d), (e) → C0c | code predecessor | pure moves; (d) / (e) shared with L3 RC-0a |
| c0-bc | C0b → C0c | code + semantic | `adopt`, the walk, the stamps the pairs read |
| c0b | C0c → C1 | semantic + code | C1's per-segment pair values (D-C5, T25) need the segments and the consumer; C1's edge rows are C1's |
| c0e | C0c ↔ L3 RC-2a | census collision | `CloneCreated` and `Adopt` share the enum, `handle`, the exhaustive matches; whichever lands second re-runs P-6 and adds its arm to every exhaustive match |

**Records carried by C0c's PR.** The umbrella row's "C0c: planned." → "C0c: landed (#<PR>)." with a changelog line
(program R-3). **For the PM:** program R-1 (close `#11-cross-document-adopt-on-insert`), R-2 (close
`#11-custom-elements-adopted-callback`), R-6 (b) / (f) triggers fire at this landing.

## §9. Definition of done

1. D-14 (b), (d), (e) landed (or (d) / (e) by L3); coordinates re-measured at the base, RC-2a's files included; P-6, P-c1, P-c2 re-run and recorded.
2. D-3 (c), D-6, D-13, D-19 implemented with §4.8's citations.
3. §6 cells present; every red cell red on the base (C0b's head) with its control green, recorded in the PR.
4. P-c2 at the head lists exactly the native, the handler, the session function and tests.
5. §6.4 green; `mise run ci` green.
6. `python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-11-eh-attrs-c0c-adopt-notification-adoptnode.md`
   and on the program memo exit 0 (run).
7. The umbrella row's C0c clause updated; the PM has applied R-1 and R-2.
