# C0b-pre2 `EH-attrs-at-creation-C0b-pre2` — an element's Attr entities are one relation, written only in `elidex-ecs` (per-PR plan memo)

Prerequisite PR **C0b-pre2** of C0b in the C0 program (program memo `docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md`,
rev 3.3; "the program memo"; C0b memo `docs/plans/2026-11-eh-attrs-c0b-node-document-fact.md`, "the C0b memo"). It
carries program decision **D-21** alone. Program IDs (H-*, D-*, A*, edges c0*) are cited as the program memo's; this
memo's own IDs are H-p*, I-R* and cells AR*.

**Placement.** PM 258 (D-21 decided, in C0b); PM 283 (the C0b plan-review's SPLIT, Ax3 IMP-1: D-21 and everything only it
needs is carved out of C0b into this PR). Order: C0b-pre1 (D-14 (c)) → **C0b-pre2** → C0b (program §1). It touches
none of C0b-pre1's files; the order is PM 283's.

**Scope.** The element's materialized attribute list (`AttrEntityCache`) and each Attr's element
(`AttrData.owner_element`) are written together, only inside `elidex-ecs`, at every state including despawn; the
dom-api Attr handlers keep marshalling and error mapping; `AttrData` and `AttrEntityCache` become crate-private.

**Not in scope.** The Attr's node document at creation and its re-home by the adopt walk (C0b: D-11, D-3 (b)); the
VM's Attr model and MutationObserver records on the dom-api Attr path (`#11-dom-api-attr-entity-model-unification`);
Trusted Types (*set an attribute* 1, 5's verification).

**Changelog.** *Rev 3.3* (2026-11-03): first draft, carved from the C0b memo rev 3.2 (`15e99049`) D-21, P-10, U19 and
§5.2 per the C0b plan-review (Ax3 IMP-1, Ax2 MIN-2) and PM 283.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Coordinates are `27b50dfe` (program §0.1). At this PR's base every coordinate is re-measured (§9 item 1).

### §0.2 Premises

Program H-17 (the VM reaches no dom-api Attr entity) and H-23 (four partial writers) bind this PR. Added:

| id | claim | evidence (`27b50dfe`) |
|---|---|---|
| H-p1 | Both encodings are writable from any crate | `AttrData` (`crates/core/elidex-ecs/src/components.rs:661`, field `owner_element` `:667`) and `AttrEntityCache` (`:675`, field `entries` `:678`) are `pub` and re-exported (`crates/core/elidex-ecs/src/lib.rs:30`); outside `elidex-ecs` `AttrData` is named on 37 lines (`git grep -c AttrData -- 'crates/*.rs' ':!crates/core/elidex-ecs'`: `char_data/attr.rs` 15, `char_data/tests/attr.rs` 16, `node_methods/tests/contains_compare.rs` 3, comments 3) |
| H-p2 | `setAttributeNode` returns null even when it replaces an Attr | `SetAttributeNode` ends `Ok(JsValue::Null)` (`crates/dom/elidex-dom-api/src/char_data/attr.rs:183`); DOM *set an attribute* step 8 returns oldAttr |
| H-p3 | Despawn clears tree and shadow references, not Attr references | `destroy_entity` (`crates/core/elidex-ecs/src/dom/tree/teardown.rs:51`) clears the shadow host / root pair (`:56-71`) and the children's links (`:121-131`) before the despawn (`:133`); an Attr's `owner_element` keeps the dead element |

### §0.3 Decision — D-21 (program D-21; full statement here)

The spec's element *attribute list* and each attribute's *element* are one relation; elidex encodes it as the
element-side `AttrEntityCache` (name → materialized Attr entity) and the Attr-side `AttrData.owner_element`. **SoT:** the
cache is the element's materialized attribute list (C0b's adopt walk's set, D-3 (b)); `owner_element` its back-pointer
— two encodings of one relation written together by one module, as `link_node` writes both sides of `TreeRelation`
(program H-2).

- **(a) Writers** (in `crates/core/elidex-ecs/src/dom/attribute.rs`): `attr_node(element, name)` (NEW; get or
  materialize — `GetAttributeNode`'s cache logic moves here); `set_attr_node(element, attr)` (NEW; DOM *set an
  attribute* 2–8: the in-use check, then *append an attribute* 1–3 or *replace an attribute* 2–5 — cache entry,
  element, node document, the old Attr's element null — and the value and *handle attribute changes* through the
  existing `set_attribute` chokepoint, **in this order**: the cache entry is swapped and both elements written
  (*replace* 2–5; *append* 1–3) before the chokepoint runs (*replace* 6 / *append* 4), because the chokepoint's
  `sync_cached_attr_value` (`attribute.rs:245-256`) writes the new value into the Attr its cache entry names, so
  swapping first leaves the old Attr's value as it was; it returns the old Attr (*set an attribute* 8) or the Attr itself
  when it is already the element's (step 4)); the `remove_attribute` chokepoint (`attribute.rs:291`; *remove an
  attribute by name* 1–2) evicts a materialized Attr and nulls its element (*remove an attribute* 2–3; step 4 is the
  chokepoint's existing dispatch); `set_attr_value(attr, value)` (NEW; *set an existing attribute value* 1, 5: an
  element-less Attr takes the value, else the chokepoint changes the element's attribute and its sync writes the Attr).
- **(b) The reverse pointer at despawn.** `destroy_entity` nulls the element of every Attr in the despawned element's
  `AttrEntityCache` and, when the despawned entity is an Attr, removes it from its element's cache — beside the
  references it already clears (H-p3). *Why there, not "readers treat a dead owner as null":* the relation's one writer
  keeps both encodings true at every state, so no reader needs a liveness check and none can return a dead entity
  (hecs' generation makes a stale `Entity` unaliased, not meaningful); a reader-side rule would be a second derivation
  of the relation, repeated in every reader.
- **(c) Visibility (Ax2 MIN-2, as D-18 for `AssociatedDocument`).** `AttrData` and `AttrEntityCache` (types and fields)
  become `pub(crate)` and leave the re-export (`lib.rs:30`), so outside `elidex-ecs` no `insert_one`, `get::<&mut _>`,
  copy, `assign` or `remove_one` of either compiles. Readers outside: `attr_owner_element(attr)`, `attr_name(attr)`,
  `attr_value(attr)` (NEW, pub; `None` for a non-Attr, which the handlers map to their existing errors). A raw
  `world_mut().despawn` of an Attr or element bypasses (b); no production code does it (§5.1 P-p2).
- **(d) The node-document stamp.** *append* 3 / *replace* 4 and `attr_node`'s materialization stamp the Attr with the
  element's stored document, `set_associated_document(attr, d)` for `d = get_associated_document(element)` when `Some`
  (HEAD's API; unread in production, H-17). C0b switches the read to `node_document(element)` and the creation to
  `create_attribute(name, node_document_for_new_nodes(element))` (C0b D-11, D-21).
- **(e) The handlers** (`CreateAttribute`, `GetAttributeNode`, `SetAttributeNode`, `RemoveAttributeNode`, the Attr
  getters and `SetAttrValue` in `char_data/attr.rs`; `RemoveAttribute` `element/props.rs:92`) keep only marshalling and
  error mapping: `SetAttributeNode` returns the old Attr's wrapper or null (H-p2); the reads `attr.rs:145`, `:214`,
  `:317`, `:359` become `attr_owner_element`, `:265`, `:288`, `:330` the accessors; `RemoveAttribute`'s eviction
  (`props.rs:148`) is deleted (the chokepoint evicts). `RemoveAttributeNode`'s guard for "the Attr still pointing at a
  despawned element" (`attr.rs:221-225`, `require_live_element`) has no input left after (b) and is deleted with its
  comment (the chokepoint's Element guard, `attribute.rs:299`, covers a non-Element receiver; `:214` rejects an Attr not
  on `this`).

*Rejected:* the SoT as a query over `AttrData.owner_element` (a world scan per adopted element, and the cache stays for
identity: still two encodings, one of them then unmaintained); stamping inside the dom-api handlers (four partial
writers of one relation: H-23's shape).

## §1. Shape and verdict

One relation, one writer module. Plan-review is mandatory (program §1, *Edge-dense work* (b)); once it passes this is a
**terminal unit** (base case (c)): it is a narrowly scoped slice of the approved program whose invariants (§2) are
independent of the node-document fact's A1–A9 (C0b plan-review Ax3 IMP-1), and the one node-document write it makes
(D-21 (d)) is unread in production (H-17), so it opens no K-0 regression and no period of two node-document
derivations. **K-0 lens:** every changed outcome is a dom-api handler's (unreachable from the VM, H-17), each toward
the spec.

## §2. Coupled invariants (this PR's)

| id | invariant | pinned by |
|---|---|---|
| I-R1 relation | `attr_owner_element(a) == Some(e)` iff `e`'s cache entry for `a`'s name is `a`, at every state, despawn included | AR1, AR2, AR4, AR5 |
| I-R2 value | a materialized Attr's value is its element's attribute value; an element-less Attr keeps its own | AR2, AR6 |
| I-R3 one writer | both encodings are written only in `elidex-ecs` | compile (D-21 (c)); P-p1 |
| I-R4 stamp | an attached Attr carries its element's stored document | unread in production (H-17): C0b's U19 node-document legs |

| pair | why they intersect | pinned by |
|---|---|---|
| I-R1×I-R2 | on *replace* the chokepoint writes the Attr its cache entry names, so the entry swap must precede it | AR2 |
| I-R1×lifecycle | despawn of either side must clear the other's encoding | AR5 |
| I-R1×I-R3 | the relation holds only if no outside code writes one encoding | compile; P-p1 |
| I-R1×I-R4 | the stamp is written with the relation, not separately | C0b U19 |

## §3. Spec coverage map

Lookups (2026-11-03): `heading dom 4.9 / 4.9.2`; `body dom concept-element-attributes-set` (2–8), `-append` (1–4),
`-replace` (1–6), `-remove` (2–4), `-remove-by-name` (1–2), `-get-by-name`, `set-an-existing-attribute-value` (1, 5),
`concept-attribute-element`.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §4.9 Interface Element | *set an attribute* 2–8 | in use elsewhere → error; already this element's → return it; replace or append; return oldAttr | `set_attr_node` (NEW) in `crates/core/elidex-ecs/src/dom/attribute.rs`; `SetAttributeNode` `crates/dom/elidex-dom-api/src/char_data/attr.rs:118` | ✓ (1, 5: Trusted Types, not modelled) | yes |
| WHATWG DOM §4.9 Interface Element | *append an attribute* 1–4; *replace an attribute* 1–6 | list entry, element, node document, old Attr's element null, then handle attribute changes | `set_attr_node` (NEW); chokepoint `crates/core/elidex-ecs/src/dom/attribute.rs:124` | ✓ | yes |
| WHATWG DOM §4.9 Interface Element | *remove an attribute by name* 1–2; *remove an attribute* 2–4 | evict, element null, handle attribute changes | `remove_attribute` `crates/core/elidex-ecs/src/dom/attribute.rs:291`; `RemoveAttribute` `crates/dom/elidex-dom-api/src/element/props.rs:92`, `RemoveAttributeNode` `crates/dom/elidex-dom-api/src/char_data/attr.rs:188` | ✓ | yes |
| WHATWG DOM §4.9 Interface Element | *get an attribute by name* | the materialized Attr for the resolved name | `attr_node` (NEW); `GetAttributeNode` `crates/dom/elidex-dom-api/src/char_data/attr.rs:42` | ✓ | yes |
| WHATWG DOM §4.9.2 Interface Attr | *element*; *set an existing attribute value* 1, 5 | element-less → own value; else change the element's attribute | `attr_owner_element`, `set_attr_value` (NEW); `SetAttrValue` `crates/dom/elidex-dom-api/src/char_data/attr.rs:295`, `GetOwnerElement` `:341` | ✓ (3–4: Trusted Types) | yes |

### §3.1 User-input touch audit

Author markup and script reach these through attribute names and values; no new entry point, no new string storage.
What changes is which Attr entity an element's list names and the handlers' return value (H-p2).

## §4. Mechanism

### §4.1 Home / kind

All writers and readers in `elidex-ecs` `dom/attribute.rs` (and `destroy_entity`); no component, event or side-store
added; two types narrowed to `pub(crate)`.

### §4.2 Reconcile — HEAD → C0b-pre2

| site | HEAD | C0b-pre2 |
|---|---|---|
| `GetAttributeNode` `attr.rs:42-115` | cache logic, `owner_element` write, cache insert | `attr_node` |
| `SetAttributeNode` `attr.rs:118-185` | `owner_element` only; returns null | `set_attr_node`; returns the old Attr or null |
| `RemoveAttributeNode` `attr.rs:188-246` | nulls `owner_element`, keeps the cache entry; `require_live_element` | the chokepoint evicts and nulls; guard deleted |
| `RemoveAttribute` `props.rs:92-152` | evicts, keeps `owner_element` | the chokepoint does both |
| `SetAttrValue` `attr.rs:295-338` | writes `AttrData` then the chokepoint | `set_attr_value` |
| `destroy_entity` `teardown.rs:51` | no Attr references | both encodings cleared (D-21 (b)) |
| `AttrData`, `AttrEntityCache` `components.rs:661`, `:675`; `lib.rs:30` | `pub`, re-exported | `pub(crate)`, not re-exported; accessors |

### §4.3 ECS-native and Layering checks

One relation, two encodings on the entities they describe, one writer module (the `TreeRelation` idiom); no
side-store. The dom-api handlers lose their algorithm (CLAUDE.md *Layering mandate*: the relation is an engine
fact, not a binding's).

### §4.4 Docstring citation table (Axis 4)

| symbol | citation |
|---|---|
| `attr_node` (NEW) | DOM §4.9 *get an attribute by name* (`#concept-element-attributes-get-by-name`) |
| `set_attr_node` (NEW) | DOM §4.9 *set an attribute* steps 2–8 (`#concept-element-attributes-set`), *append an attribute* 1–4 (`#concept-element-attributes-append`), *replace an attribute* 1–6 (`#concept-element-attributes-replace`) |
| `remove_attribute` (doc) | DOM §4.9 *remove an attribute by name* 1–2 (`#concept-element-attributes-remove-by-name`) → *remove an attribute* 2–4 (`#concept-element-attributes-remove`) |
| `attr_owner_element` (NEW) | DOM §4.9.2 Attr's *element* (`#concept-attribute-element`) |
| `set_attr_value` (NEW) | DOM §4.9.2 *set an existing attribute value* 1, 5 (`#set-an-existing-attribute-value`) |
| `destroy_entity` (doc) | none (elidex lifecycle: the reverse pointer, D-21 (b)) |

## §5. Populations, tests that change, touched files

### §5.1 Populations

- **P-p1 writers outside `elidex-ecs`**: `git grep -n -E 'owner_element *=|AttrEntityCache' -- 'crates/*.rs'
  ':!crates/core/elidex-ecs'` → 23 lines at `27b50dfe`: `char_data/attr.rs` 9 (writes `:97`, `:180`, `:233`; cache `:3`,
  `:61`, `:77`, `:101-107`), `element/props.rs` 3, tests writing `owner_element` (`char_data/tests/attr.rs` 6,
  `node_methods/tests/contains_compare.rs` 3), a VM comment (`element_attrs.rs:374`) and a property-name table entry
  (`vm/well_known.rs:417`, not a write); at the head the last two only. `git grep -n AttrData -- 'crates/*.rs'
  ':!crates/core/elidex-ecs'` → 37 (H-p1) → comments only.
- **P-p2 raw despawns**: `git grep -n -E 'world_mut\(\)\.despawn' -- 'crates/*.rs' ':!crates/core/elidex-ecs'` → 1
  at `27b50dfe` (`crates/api/elidex-api-canvas/src/offscreen.rs:352`, after that file's first `#[cfg(test)]`, `:219`:
  an inline test); production hits (program §4.4's test filter) → 0, at the head too.

### §5.2 Existing tests whose expectation changes

`crates/dom/elidex-dom-api/src/char_data/tests/attr.rs` and `node_methods/tests/contains_compare.rs`: every `AttrData`
use (16 and 3 lines) is rewritten over `set_attr_node` and the accessors. `remove_attribute_node_on_non_element_errors_without_detaching`
(`char_data/tests/attr.rs:430-449`) builds an Attr whose element is a Document, a state `set_attr_node` cannot produce;
it is rewritten to the reachable shape — the Attr set on an element `e` through `set_attr_node`, `RemoveAttributeNode`
invoked with the Document as receiver → error, and `attr_owner_element(attr) == Some(e)` — not rebuilt through an
`elidex-ecs`-internal fixture. `set_attribute_node_returns_null` (`:370`) stays: no old Attr → null.

### §5.3 Touched files

| file | lines at `27b50dfe` | change | 1000-line decision |
|---|---|---|---|
| `crates/core/elidex-ecs/src/dom/attribute.rs` | 331 | writers, accessors, eviction | ≈ 430; < 1000 |
| `crates/core/elidex-ecs/src/components.rs`; `src/lib.rs` | 822; 51 | visibility; re-export narrowed | < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/teardown.rs` | 344 | D-21 (b) | < 1000 |
| `crates/dom/elidex-dom-api/src/char_data/attr.rs`; `element/props.rs` | 386; 153 | handlers hollowed | < 1000 |
| `crates/dom/elidex-dom-api/src/char_data/tests/attr.rs`; `node_methods/tests/contains_compare.rs` | 450; 295 | §5.2; cells AR1–AR6 | < 1000 (re-measure) |

### §5.4 Risks

The changed outcomes are the dom-api handlers' only (H-17): the replaced Attr is detached, `setAttributeNode`'s return
value, `getAttributeNode` after `setAttributeNode` returns the set Attr.

## §6. Test plan

Cells in `crates/dom/elidex-dom-api/src/char_data/tests/attr.rs`, through the handlers (`a`, `b` from `CreateAttribute`,
same name; `e` an element). **Red-first:** every red cell runs on the base in its *base formulation* — the same handler
calls, read through HEAD's `pub` `AttrData` / `AttrEntityCache` instead of the NEW accessors — with its control green,
and the failure is recorded in the PR.

| cell | kind | shape | expected | base formulation → base result; control |
|---|---|---|---|---|
| AR1 | red | `SetAttributeNode(e, a)`; `GetAttributeNode(e, name)` | the same wrapper as `a` (*append* 1) | wrappers compared → a new entity (no cache entry, H-23); control: two `GetAttributeNode` calls return one wrapper |
| AR2 | red | then `old = SetAttributeNode(e, b)` | `old` is `a` (*set* 8); `attr_owner_element(a) == None` (*replace* 5); `a`'s value unchanged, `b`'s is `e`'s attribute value (D-21 order); `GetAttributeNode` → `b` | `old` → null (H-p2), `a.owner_element` → `Some(e)`; control: `e`'s attribute value equals `b`'s |
| AR3 | red | then `SetAttributeNode(e, b)` again | returns `b` (*set* 4), nothing changes | → null; control: `e`'s attribute value unchanged |
| AR4 | red | `GetAttributeNode(e, name)` then `RemoveAttribute(e, name)`; separately `RemoveAttributeNode(e, b)` | the Attr's element null; `b` evicted (a later `GetAttributeNode` after `setAttribute` returns a new entity) | `owner_element` stays `Some(e)` (`RemoveAttribute`); the cache keeps `b` (`RemoveAttributeNode`); control: the attribute is gone from `Attributes` |
| AR5 | red | `destroy_entity(e)` with `a` materialized; separately `destroy_entity(attr)` | `attr_owner_element(a) == None`; `e`'s cache holds no entry for the despawned Attr | `owner_element` → the dead `e`; the entry stays; control: `dom.contains(e) == false` |
| AR6 | guard | `SetAttrValue(a, v)` on an attached and on an element-less Attr | attached: `e`'s attribute and `a` read `v`; element-less: only `a` | (green on base) |

Kept green: the rest of `char_data/tests/attr.rs` (InUse `:127`, non-Element receivers `:401`, the rewritten `:430`),
`cargo test --all-features -p elidex-ecs -p elidex-dom-api`, then `mise run ci`.

### §6.1 Mutation guards

| mutation | killed by |
|---|---|
| `set_attr_node` registers no cache entry | AR1 |
| the chokepoint runs before the entry swap (the old Attr's value overwritten) | AR2 |
| *set* 4's early return missing | AR3 |
| the chokepoint does not evict or null | AR4 |
| despawn leaves either encoding | AR5 |
| a dom-api write of either encoding | compile (D-21 (c)) |
| the stamp (D-21 (d)) missing | unobservable here (H-17); C0b's U19 |

## §7. Out of scope

The scope list above. The handlers' records stay as HEAD's (none; `#11-dom-api-attr-entity-model-unification`).

## §8. Edges

| # | edge | kind | note |
|---|---|---|---|
| c0-pre2 | C0b-pre2 → C0b | code + semantic | C0b's walk re-homes the cache this PR keeps faithful (D-3 (b)); C0b switches D-21 (d)'s read and `attr_node`'s creation (C0b D-11); file overlap `teardown.rs` (`destroy_entity` here, the fragment functions in C0b) |

## §9. Definition of done

1. Coordinates re-measured at the base; P-p1, P-p2 run and recorded.
2. D-21 (a)–(e) implemented with §4.4's citations; `require_live_element` gone from `RemoveAttributeNode`.
3. AR1–AR5 red on the base in their base formulation with their controls green; AR6 green on both; recorded in the PR.
4. At the head: P-p1 = the VM comment and the table entry; `AttrData` outside `elidex-ecs` in comments only; P-p2 = 0.
5. §6's kept-green commands and `mise run ci` green.
6. `python3 .claude/skills/elidex-plan-review/preflight.py` on this memo exits 0 (run).
