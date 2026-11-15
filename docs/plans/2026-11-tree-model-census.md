# Tree-model program: census data (at `c0578223`)

Data file of [2026-11-tree-model-shadow-root-is-a-root-umbrella.md](2026-11-tree-model-shadow-root-is-a-root-umbrella.md)
(§6 there states what each population is for and which slice consumes it). This file holds the commands, the counts
and the per-file tables. It is not a plan: it has no slice table and no slot table.

**Rules of this census (PM 730).**
- A population is defined by a **property** of the code ("reads the stored parent of an entity"), not by a name that
  one round happened to look at. The command is the instrument that approximates the property; where the instrument
  is narrower or wider than the property, the row says so.
- Every command is `git grep -n -P` (PCRE). `-E` is never used: `\b` and `\s` are silent zeros under `git grep -E`
  on this toolchain.
- Every command runs from the repository root against the tree `c0578223`. "Non-test" is the pathspec
  `-- crates ':!*tests*'`: it removes files and directories whose path contains `tests`. An inline
  `#[cfg(test)] mod tests` inside a non-test file is therefore counted as non-test (known over-count; the per-PR plan
  of the consuming slice subtracts by reading).
- Each population has a **positive control**: one line that must be in the output. A count without its control
  line present is not a measurement.
- Counts are lines, not call expressions. A line with two calls counts once.

## C-1. Populations

| Id | Property (what the code does) | Command (after `git grep -n -P`, before `c0578223 -- crates ':!*tests*'` unless noted) | Count | Positive control |
|---|---|---|---|---|
| P-UP | reads the stored parent of an entity through the public accessor | `'\bget_parent\('` | 143 lines (1 is the definition, `navigation.rs:16`; 142 call lines) | `crates/api/elidex-api-observers/src/mutation/mod.rs:215` |
| P-UP-T | the same, in test paths | `'\bget_parent\('` over `-- crates`, minus P-UP | 174 − 143 = 31 lines | any line under a `tests` path; the two totals are both printed |
| P-REL | reads or writes a tree link field of `TreeRelation` directly (no accessor) | `'\.(parent\|first_child\|last_child\|next_sibling\|prev_sibling)\b(?!\s*\()'` over `-- crates/core/elidex-ecs/src ':!*tests*'` | 41 lines (1 false positive: `fragment_tree.rs:512`, a layout fragment's `parent`) | `crates/core/elidex-ecs/src/dom/mod.rs:756` (`rev_version`'s `rel.parent`) |
| P-CH | reads a raw child or sibling link through the public accessors | `'\b(get_first_child\|get_last_child\|get_next_sibling\|get_prev_sibling)\('` | 108 lines (34 more in test paths) | `crates/dom/elidex-dom-api/src/range/mod.rs:558` (the Range boundary walk; 4 lines in that file) |
| P-CHX | reads a child or sibling through a compensating accessor (skips shadow roots) | one command per accessor, C-4 | C-4 | C-4 |
| P-MARK | tests or reads the `ShadowRoot` component of an entity | `'::<&?(\w+::)*(Ecs)?ShadowRoot>\|\bis_shadow_root\('` | 63 lines (2 false positives: `parse_state.rs:66`, `:335`, a method of the strict parser's own target enum) | `crates/core/elidex-ecs/src/dom/mod.rs:911` (`ChildrenIter`'s skip) |
| P-HOST | reads the backward link field `ShadowRoot.host` | `'\.host\b(?!\s*\()'` | 41 lines, of which 14 are in shadow-related files (C-5); the rest are URL / cookie / origin `host` fields | `crates/core/elidex-ecs/src/dom/mod.rs:761` |
| P-HOSTFN | reads the backward link through its accessor | `'\bshadow_host\('` | 2 lines (the definition `shadow.rs:521`, one caller `html_fragment.rs:81`) | the definition line |
| P-DER | calls a derived relation built on the above (root, ancestor test, connectedness, order) | one command per function, C-6 | C-6 | C-6 |

**Why P-REL is confined to one crate.** The link fields of `TreeRelation` are `pub(crate)`
(`crates/core/elidex-ecs/src/components.rs:151-155`), so the compiler already confines direct field access to
elidex-ecs. The crate-wide form of the P-REL command (55 lines) only adds same-named fields of other types (a
`MutationRecord`'s `next_sibling`, the VM's `well_known` atoms, compiler scopes); none of them is a `TreeRelation`.

**Agreement with the independent verification** (`23aa745a`): P-UP 143 / 31, P-CH 108, P-MARK 63, P-HOST 41 and
P-HOSTFN 2 are the same numbers at `c0578223`. P-REL, P-CHX and P-DER were not measured there.

## C-2. P-UP by file, with a preliminary relation

The relation column is **preliminary** (file-level reading of the call's purpose, not a per-call adjudication). The
binding classification is made by the compiler in slice `ta` (umbrella §5): `get_parent` is deleted, so each of these
lines fails to compile until it names a relation, and the `ta` plan lists each line with its choice and its proof.

Legend: **T** = the spec relation is the tree parent. **S** = shadow-including (tree parent, else host).
**H** = host-including (DOM §4.7). **F** = flat-tree parent or event *get the parent* (slot-aware relations this
program does not define; the line keeps base behaviour through the shadow-including step, umbrella §1.4).
**g** = the line or its loop already carries a marker guard (compensation). **x** = at base the climb crosses a
shadow boundary with no guard although the relation is T (a pending flip, umbrella §1.3). **1** = single hop on a
receiver that is, by the file's reading, never a shadow root (T, no behaviour difference). **?** = not classified at
file level.

| File (under `crates/`) | Lines | Preliminary | Slice that flips (if x) |
|---|---|---|---|
| `api/elidex-api-observers/src/mutation/mod.rs` | 2 | T, x (`:215` transient observers, `:524` *queue a mutation record*) | tm |
| `core/elidex-ecs/src/dom/mod.rs` | 1 | `:378` `is_contenteditable`: ? (HTML editing host inheritance) | te |
| `core/elidex-ecs/src/dom/shadow.rs` | 3 | `:280` 1; `:490`, `:495` T (climb to the tree root, which is the shadow root) | — |
| `core/elidex-ecs/src/dom/tree/mod.rs` | 1 | `:51` `is_ancestor` (the `append_child` cycle guard): H | — (ts) |
| `core/elidex-ecs/src/dom/tree/mutation.rs` | 2 | `:419` `walk_parent`: S (explicit hop already written); `:683` 1 | — |
| `core/elidex-ecs/src/dom/tree/navigation.rs` | 3 | `:16` definition; `:62` `index_in_parent` T; `:378` `root_entities` T | — |
| `core/elidex-ecs/src/dom/tree/teardown.rs` | 2 | `:90`, `:221`: the node's place in a child list (T); for a shadow root the base value is the host | — (ts) |
| `core/elidex-ecs/src/dom/tree/walkers.rs` | 6 | `:245` `is_connected` S; `:295` `ancestor_chain` (tree order) T, x; `:364` `is_light_tree_ancestor_or_self` T, g; `:380` H; `:415` `find_tree_root` T, g; `:436` S | te (`:295`) |
| `core/elidex-render/src/builder/paint/mod.rs` | 1 | F | — |
| `core/elidex-render/src/builder/walk.rs` | 2 | `:232` root test ?; `:499` F | — |
| `css/elidex-css/src/selector/matching.rs` | 6 | `:26`, `:40`, `:45` T, g; `:189`, `:192`, `:194` 1 (structural pseudo-classes) | — |
| `css/elidex-css/src/selector/traverse.rs` | 1 | ? | — |
| `css/elidex-style/src/generated_content.rs` | 2 | 1 (a pseudo-element's originating element) | — |
| `css/elidex-style/src/walk.rs` | 1 | `:126` root test ? | — |
| `dom/elidex-a11y/src/tree.rs` | 2 | F, g (`:237` reads the host explicitly) | — |
| `dom/elidex-dom-api/src/char_data/split_text.rs` | 3 | 1 | — |
| `dom/elidex-dom-api/src/child_node/mod.rs` | 2 | 1 | — |
| `dom/elidex-dom-api/src/child_node/mutations.rs` | 5 | 1, except a shadow-root receiver (ChildNode is not on ShadowRoot; proof owed in `ta`) | — |
| `dom/elidex-dom-api/src/child_node/selectors.rs` | 1 | ? | — |
| `dom/elidex-dom-api/src/element/inheritance.rs` | 2 | S by spec (HTML §3.2.6.2: "If the node's parent is a shadow root: use the language of that shadow root's host") | — |
| `dom/elidex-dom-api/src/element/layout_query.rs` | 6 | ? (CSSOM View `offsetParent` and scroll ancestors; relation looked up in `te`) | te |
| `dom/elidex-dom-api/src/element/markup.rs` | 1 | 1 (`outerHTML` setter's parent test) | — |
| `dom/elidex-dom-api/src/element/option_disabled.rs` | 2 | T, x (optgroup ancestor) | tf |
| `dom/elidex-dom-api/src/element/select.rs` | 3 | 1 | — |
| `dom/elidex-dom-api/src/element/table_mutation/mod.rs` | 7 | 1 | — |
| `dom/elidex-dom-api/src/element/tree.rs` | 3 | 1 | — |
| `dom/elidex-dom-api/src/focus/predicate.rs` | 2 | `:126` 1; `:313` ? (inert / disabled ancestors) | te |
| `dom/elidex-dom-api/src/focus/sot.rs` | 1 | ? | te |
| `dom/elidex-dom-api/src/node_methods/mod.rs` | 1 | `:67` `find_root`: S (a third shadow-including root) | — |
| `dom/elidex-dom-api/src/range/boundary.rs` | 5 | T, x | tr |
| `dom/elidex-dom-api/src/range/mod.rs` | 4 | T, x (`:126`, `:137` root and common ancestor; `:552`, `:582` walk) | tr |
| `dom/elidex-dom-api/src/range/mutation.rs` | 2 | T, x | tr |
| `dom/elidex-dom-api/src/selection.rs` | 1 | T, x | tr |
| `dom/elidex-dom-api/src/traversal.rs` | 10 | T, x (bounded by the traversal root in most paths; proof per line in `tr`) | tr |
| `dom/elidex-dom-api/src/tree_nav.rs` | 2 | T, x on a shadow-root receiver (`parentNode` / `parentElement` handlers have no guard; the VM natives guard, `node_proto.rs:328`, `:345`) | tc |
| `dom/elidex-dom-compat/src/presentational.rs` | 2 | T, x (table ancestors of a cell) | te |
| `dom/elidex-form-core/src/fieldset.rs` | 4 | T, x (*disabled fieldset*: "descendant of") | tf |
| `dom/elidex-form/src/select/mod.rs` | 4 | T, x (*nearest ancestor select*) | tf |
| `dom/elidex-form/src/submit.rs` | 3 | T, x (*reset the form owner* step 4; "nearest ancestor dialog") | tf |
| `dom/elidex-html-parser-strict/src/tree_builder/mod.rs` | 1 | T, g (`:308`) | — |
| `layout/*` (8 files) | 10 | root tests (`is_none()`, 6) ?; table row / section parent (3) 1; `shift.rs:188` ? | — |
| `script/elidex-js/src/vm/host/html_legend_proto.rs` | 1 | 1 | — |
| `script/elidex-js/src/vm/host/node_proto.rs` | 2 | T, g (`:328`, `:345`) | — |
| `script/elidex-js/src/vm/host/range_proto.rs` | 1 | ? | tr |
| `script/elidex-script-session/src/event_dispatch/mod.rs` | 1 | F, g (`:214` stops a non-composed path at the shadow root) | — |
| `script/elidex-script-session/src/mutation/child_list.rs` | 4 | 1 | — |
| `script/elidex-script-session/src/mutation/html_fragment.rs` | 5 | 1 (`:80-81` already resolves a shadow-root receiver through `shadow_host`) | — |
| `script/elidex-wasm-runtime/src/runtime.rs` | 1 | 1 (an inline test assertion) | — |
| `shell/elidex-shell/src/app/events.rs`, `app/hover.rs`, `content/scroll.rs` | 3 | F | — |
| `shell/elidex-shell/src/content/form_input.rs` | 2 | T, x (form ancestor) | tf |
| `shell/elidex-shell/src/content/iframe/lifecycle.rs` | 1 | ? | — |

Total: 143 lines in 60 files (`git grep -n -P '\bget_parent\(' c0578223 -- crates ':!*tests*' | cut -d: -f2 | sort -u | wc -l`
gives 60). The independent verification's script classification of the 142 calls (in a loop / single, shadow-aware /
not) gave 12 / 47 / 18 / 65; that script is not reproduced here, because `ta` replaces the approximation with the
compiler's list.

## C-3. P-CH by file

`git grep -n -P '\b(get_first_child|get_last_child|get_next_sibling|get_prev_sibling)\(' c0578223 -- crates ':!*tests*' | cut -d: -f2 | uniq -c`

| File (under `crates/`) | Lines |
|---|---|
| `core/elidex-ecs/src/dom/tree/navigation.rs` | 18 (4 definitions and the element / exposed-sibling helpers built on them) |
| `core/elidex-ecs/src/dom/tree/walkers.rs` | 2 |
| `core/elidex-render/src/builder/mod.rs` | 2 |
| `css/elidex-style/src/counter.rs`, `generated_content.rs`, `pseudo.rs` | 2, 1, 1 |
| `dom/elidex-dom-api/src/child_node/mod.rs` | 4 |
| `dom/elidex-dom-api/src/node_methods/core.rs` | 2 |
| `dom/elidex-dom-api/src/range/mod.rs` | 4 |
| `dom/elidex-dom-api/src/traversal.rs` | 29 |
| `dom/elidex-dom-api/src/tree_nav.rs` | 9 |
| `dom/elidex-dom-compat/src/presentational.rs` | 2 |
| `dom/elidex-form-core/src/fieldset.rs` | 2 |
| `dom/elidex-form/src/fieldset.rs`, `init.rs`, `label.rs`, `select/mod.rs`, `submit.rs` | 2, 1, 2, 8, 2 |
| `dom/elidex-html-parser-strict/src/tree_builder/insert.rs` | 1 |
| `layout/elidex-layout-block/src/helpers.rs` | 4 |
| `layout/elidex-layout-table/src/helpers.rs` | 2 |
| `layout/elidex-layout/src/layout/mod.rs` | 1 |
| `script/elidex-script-session/src/mutation/html_fragment.rs` | 3 |
| `shell/elidex-shell/src/content/event_handlers.rs`, `content/iframe/lifecycle.rs` | 2, 2 |

Total 108. Each line is one of: **guarded** (a marker test on the entity it obtained, or a filter such as
`is_element` that a `DocumentFragment` fails), or **unguarded** (at base it can hand a shadow root to its consumer as
if it were a child of the host). The split is not measured here: it needs the consumer read per line, and slice `tc`
makes it unnecessary to guess, since it removes the raw / exposed distinction (umbrella §1.2) and every line then
reads the tree children. Known unguarded reader, from the Range contents program's own defect table: D13 (a), (b)
(`docs/plans/2026-11-range-contents-program.md:731`).

## C-4. P-CHX: compensating child-side accessors

Command per row: `git grep -n -P '\b<pattern>' c0578223 -- crates ':!*tests*' | wc -l`.

| Accessor | Pattern | Lines | What compensates |
|---|---|---|---|
| `children_iter` | `children_iter\(` | 86 | `ChildrenIter::next` skips a `ShadowRoot` entity (`crates/core/elidex-ecs/src/dom/mod.rs:911`) |
| `children_iter_rev` | `children_iter_rev\(` | 9 | `ChildrenIterRev::next` (`mod.rs:944`) |
| `children` | `\.children\(` | 59 (over-count: any `.children(` receiver) | its own skip at `navigation.rs:265` |
| `child_list_uncapped` | `child_list_uncapped\(` | 13 | skip at `navigation.rs:292` |
| `index_in_parent` | `index_in_parent\(` | 9 | skips shadow-root predecessors (`navigation.rs:70`) |
| `next_exposed_sibling` / `prev_exposed_sibling` | `(next\|prev)_exposed_sibling\(` | 30 | skip at `navigation.rs:158`, `:172` |

Under the rule (umbrella §1.2) none of these needs its skip, and the last row has no reason to exist beside the raw
sibling accessors.

## C-5. P-MARK and P-HOST by role

The 61 true P-MARK lines, read once and grouped by what the marker test is for. Roles **K** and **B** are the
compensation the program deletes; **C** becomes the body of the one crossing step; **I** and **D** stay.

| Role | Meaning | Lines (file:line under `crates/`) |
|---|---|---|
| **K** child-side compensation: "this child is a shadow root, skip it" | `core/elidex-ecs/src/dom/mod.rs:911`, `:944`; `dom/shadow.rs:366`, `:467`; `dom/tree/navigation.rs:70`, `:158`, `:172`, `:265`, `:292`; `css/elidex-style/src/walk.rs:519`; `dom/elidex-dom-api/src/live_collection/mod.rs:443`; `script/elidex-js/src/vm/host/node_proto.rs:384`, `:397` (13). Two of them re-test a child that a compensating accessor already filtered (`walk.rs:519` over `children`, `live_collection/mod.rs:443` over `children_iter`) |
| **B** parent-side compensation: "the climb reached a shadow root, stop" | `core/elidex-ecs/src/dom/tree/walkers.rs:357`, `:410`, `:416`; `dom/shadow.rs:492`; `css/elidex-css/src/selector/matching.rs:34`, `:49`; `dom/elidex-html-parser-strict/src/tree_builder/mod.rs:308`; `script/elidex-js/src/vm/host/node_proto.rs:328`, `:345`; `script/elidex-script-session/src/event_dispatch/mod.rs:214` (10) |
| **C** crossing hop: reads `.host` to continue upward | `core/elidex-ecs/src/dom/mod.rs:761`; `dom/tree/mutation.rs:420`; `dom/tree/walkers.rs:385`, `:438`; `dom/elidex-a11y/src/tree.rs:237` (5). The three elidex-ecs ones that read the tree parent first are the branches the independent verification found unreachable for an attached shadow root |
| **E** event suppression: "node or parent is a shadow root, fire nothing" | `core/elidex-ecs/src/dom/tree/mutation.rs:528`, `:582` (2). Why a shadow-root **parent** suppresses the fire is not established by this census (umbrella §7.3 Q-4) |
| **L** lifetime: the strip | `core/elidex-ecs/src/dom/tree/teardown.rs:69`, `:81` (2); owned by the ecs-entity-lifetime program |
| **I** identity: "is this node a ShadowRoot" for dispatch, brand or kind | `core/elidex-ecs/src/dom/tree/navigation.rs:89`, `:90` (the predicate); `dom/classify.rs:119` (`is_document_fragment` excludes a shadow root); `dom/elidex-dom-api/src/child_node/mod.rs:322`, `document.rs:345`, `node_methods/clone.rs:115`, `range/mutation.rs:502`; `script/elidex-js/src/vm/host_data/engine_feature/entity_kind.rs:42`, `:114`, `:173`; `script/elidex-script-session/src/mutation/child_list.rs:190`, `mutation/html_fragment.rs:80`; `event_dispatch/mod.rs:173`, `:183`, `:595`, `:635` (16). Four of these reject inserting a shadow root as a node (`child_node/mod.rs:322`, `document.rs:345`, `range/mutation.rs:502`, `child_list.rs:190`): they stay, the spec forbids the insertion under either model |
| **D** data: reads another field of the component (mode, slot assignment, delegates focus, clonable, serializable) | `core/elidex-ecs/src/dom/shadow.rs:266`, `:271`, `:406`, `:419`, `:523` (`:266` and `:523` read `host` itself: the link, P-HOST); `dom/elidex-dom-api/src/element/markup.rs:307`, `focus/delegate.rs:101`, `node_methods/clone.rs:453`; `script/elidex-js/src/vm/host/element_shadow.rs:122`, `shadow_root_proto.rs:188`; `script/elidex-script-session/src/mutation/html_fragment.rs:464`, `:504`, `:630` (13) |

13 + 10 + 5 + 2 + 2 + 16 + 13 = 61. The role of each line is from a single read at line level; `tc` (role K) and
`ta` (roles B, C) re-derive their rows from the compiler's list and treat any disagreement with this table as a
finding against the table.

P-HOST in shadow-related files (14 lines): `core/elidex-ecs/src/dom/shadow.rs` 5, `dom/tree/walkers.rs` 2,
`dom/tree/teardown.rs` 1, `dom/tree/mutation.rs` 1, `dom/mod.rs` 1, `dom/elidex-a11y/src/tree.rs` 2,
`script/elidex-script-session/src/event_dispatch/mod.rs` 1, `script/elidex-js/src/vm/host/shadow_root_proto.rs` 1
(`git grep -n -P '\.host\b(?!\s*\()' c0578223 -- crates ':!*tests*' | cut -d: -f2 | sort | uniq -c`). The
ecs-entity-lifetime program's PR (io) plan §10 is the census of this field's readers; this program does not repeat it.

## C-6. P-DER: derived relations

Command per row: `git grep -n -P '\b<pattern>' c0578223 -- crates ':!*tests*' | wc -l` (definition lines included).

| Function | Pattern | Lines | Relation it computes at base | Model its doc or body assumes |
|---|---|---|---|---|
| `EcsDom::find_tree_root` | `find_tree_root\(` | 24 | tree root | (Y): stops at the marker (`walkers.rs:410`, `:416`) |
| `EcsDom::find_tree_root_composed` | `find_tree_root_composed\(` | 3 | shadow-including root | (X): the `.host` branch (`walkers.rs:438`) is entered only for a parentless shadow root |
| `find_root` (dom-api `node_methods/mod.rs:64`) | counted in P-UP | 1 body line | shadow-including root | (Y): relies on the stored parent crossing |
| `EcsDom::is_connected` | `is_connected\(` | 26 | connected | (Y): doc `walkers.rs:226-228` |
| `EcsDom::is_host_including_ancestor_or_self` | `is_host_including_ancestor_or_self\(` | 4 | host-including inclusive ancestor (shadow roots only) | (X) body, (Y) storage; its doc cites "DOM 4.2.1", the relation is defined in §4.7 |
| `EcsDom::is_light_tree_ancestor_or_self` | counted in P-MARK role B | — | inclusive ancestor | (Y): marker stop (`walkers.rs:357`) |
| `is_ancestor` (`tree/mod.rs:40`) | counted in P-UP | — | the cycle guard of `append_child` | (Y): crosses through the stored parent, which is what makes host-into-own-shadow-tree a cycle today |
| `EcsDom::tree_order_cmp` | `tree_order_cmp\(` | 6 | tree order, falling back to entity bits across trees | (Y): `ancestor_chain` crosses, so a shadow-tree node and a light node compare as one tree |
| `EcsDom::root_entities` | `root_entities\(` | 5 | parentless non-fragment entities | (Y): doc `navigation.rs:359-360` |
| `EcsDom::get_shadow_root` | `get_shadow_root\(` | 25 | the forward link | neutral |
| `EcsDom::composed_children` | `composed_children\(` | 24 | flat-tree children (downward) | neutral; owned by `#11-shadow-slot-assignment-flat-tree` |

## C-7. What this census does not establish

1. The per-call relation of the 142 P-UP calls (C-2 is file-level and preliminary).
2. The guarded / unguarded split of P-CH (C-3).
3. Any runtime behaviour. Every "x" in C-2 is a prediction from reading the code; none was executed. Each flip
   slice's plan starts with a probe that runs the base behaviour (umbrella §5, acceptance).
4. Test-path lines beyond their counts (P-UP-T 31, P-CH 34): which of them pin the (Y) storage as a contract (a
   shadow root found in its host's raw child list, `get_parent(sr) == Some(host)`).
