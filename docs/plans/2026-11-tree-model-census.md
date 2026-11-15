# Tree-model program: census data (at `c0578223`)

Data file of [2026-11-tree-model-shadow-root-is-a-root-umbrella.md](2026-11-tree-model-shadow-root-is-a-root-umbrella.md)
(§6 there states what each population is for and which slice consumes it). This file holds the commands, the counts
and the per-file tables. It is not a plan: it has no slice table and no slot table.

Draft 2 (after the R1 plan-review at `379acc15`, PM 747 / PM 748): C-2 carries the **resolved** relation of every
line that draft 1 marked "?" (C-8, the section headed §C8, records each lookup); C-6 is replaced by a census over function bodies (derived
relations by property, script in C-11); C-9 (writers) and C-10 (the VM native and dom-api handler pairs) are new.

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
| P-CH | reads a raw child or sibling link through the public accessors | `'\b(get_first_child\|get_last_child\|get_next_sibling\|get_prev_sibling)\('` | 108 lines (34 more in test paths, verified 2026-11-15: the same command over `-- crates` prints 142) | `crates/dom/elidex-dom-api/src/range/mod.rs:558` (the Range boundary walk; 4 lines in that file) |
| P-CHX | reads a child or sibling through a compensating accessor (skips shadow roots) | one command per accessor, C-4 | C-4 | C-4 |
| P-MARK | tests or reads the `ShadowRoot` component of an entity | `'::<&?(\w+::)*(Ecs)?ShadowRoot>\|\bis_shadow_root\('` | 63 lines (2 false positives: `parse_state.rs:66`, `:335`, a method of the strict parser's own target enum) | `crates/core/elidex-ecs/src/dom/mod.rs:911` (`ChildrenIter`'s skip) |
| P-HOST | reads the backward link field `ShadowRoot.host` | `'\.host\b(?!\s*\()'` | 41 lines, of which 14 are in shadow-related files (C-5); the rest are URL / cookie / origin `host` fields | `crates/core/elidex-ecs/src/dom/mod.rs:761` |
| P-HOSTFN | reads the backward link through its accessor | `'\bshadow_host\('` | 2 lines (the definition `shadow.rs:521`, one caller `html_fragment.rs:81`) | the definition line |
| P-DER | is a **derived relation**: a read-only function (no `&mut` in its signature) whose body reads a parent link (level 1), or calls a level-1 member (level 2) | the script of C-11, `python3 <script> c0578223` | level 1: 114 climbing functions, 60 read-only and 54 mutating; level 2: 113 functions that call a read-only level-1 name, 45 read-only and 68 mutating (C-6) | `is_ancestor` at level 1 and `is_ancestor_or_self` at level 2 through it; negative control: `get_shadow_root` is not in the output |
| P-WR | writes a parent link | `'\.parent\s*=[^=]'` over `-- crates/core/elidex-ecs/src ':!*tests*'` | 3 lines: one clear (`tree/mod.rs:86`), one link (`tree/mutation.rs:457`, in `link_node`), one doc comment (`tree/teardown.rs:54`) (C-9) | `crates/core/elidex-ecs/src/dom/tree/mutation.rs:457` |

**Why P-REL is confined to one crate.** The link fields of `TreeRelation` are `pub(crate)`
(`crates/core/elidex-ecs/src/components.rs:151-155`), so the compiler already confines direct field access to
elidex-ecs. The crate-wide form of the P-REL command (55 lines) only adds same-named fields of other types (a
`MutationRecord`'s `next_sibling`, the VM's `well_known` atoms, compiler scopes); none of them is a `TreeRelation`.

**Agreement with the independent verification** (`23aa745a`): P-UP 143 / 31, P-CH 108, P-MARK 63, P-HOST 41 and
P-HOSTFN 2 are the same numbers at `c0578223`. P-REL, P-CHX, P-DER and P-WR were not measured there. The R1
plan-review (Axis 5) re-ran every count of draft 1 at `c0578223` and they reproduced.

## C-2. P-UP by file, with the resolved relation

Draft 1 left 13 rows at "?". Every one is resolved here from the spec text (C-8 has the lookups); a row whose
relation this program does not define says so instead of borrowing a relation. The column is still a file-level
reading: the binding list is the compiler's in slice `ta` (umbrella §5), where `get_parent` is deleted and the plan
lists each line with its choice and its proof. A disagreement between that list and this table is a finding against
this table.

Legend (what the line takes in `ta`; names are in umbrella §1):
- **T** = `tree_parent`, with the reason base behaviour is equal: **1** the receiver is never a shadow root;
  **g** an existing guard is folded into the choice; **h** a dom-api navigation handler that takes the VM native's
  relation (PM 748 (b)).
- **S** = `shadow_including_parent`. **H** = the host-including test (DOM §4.7).
- **P:tm / P:tr / P:tf / P:te** = the transitional `parent_crossing_pending_flip`, tagged with the slice that flips
  it: the spec relation is the tree parent and base crosses a shadow boundary with no guard (**predicted from the
  code, not run**).
- **F / E / L** = a relation this program does not define (umbrella §1.5): the flat-tree parent, the event *get the
  parent*, the selector tree's parent. The line takes that relation's own pending name and keeps base behaviour.
- **raw** = a storage-maintenance read inside the `dom::tree` module: it becomes a module-private read of the stored
  link (population P-REL) and is re-read by `ts`.

| File (under `crates/`) | Lines | Resolved | Spec basis (C-8 row) |
|---|---|---|---|
| `api/elidex-api-observers/src/mutation/mod.rs` | 2 | P:tm (`:215` transient observers, `:524` *queue a mutation record*) | [D6] |
| `core/elidex-ecs/src/dom/mod.rs` | 1 | `:378` `is_contenteditable`: P:te | R-1 |
| `core/elidex-ecs/src/dom/shadow.rs` | 3 | `:280` T1; `:490`, `:495` T (the climb to a slot's tree root; folds into `find_tree_root`) | [D1] |
| `core/elidex-ecs/src/dom/tree/mod.rs` | 1 | `:51` `is_ancestor`: removed; its three callers (the cycle guards of `append_child`, `insert_before`, `replace_child`) take H | [D5] |
| `core/elidex-ecs/src/dom/tree/mutation.rs` | 2 | `:419` `walk_parent`: removed, the adopt walk takes S; `:683` `detach_with_hook`: raw | [D4] |
| `core/elidex-ecs/src/dom/tree/navigation.rs` | 3 | `:16` the definition (deleted); `:62` `index_in_parent`: raw; `:378` `root_entities`: T, g (it already excludes `DocumentFragment` kinds) | [D1] |
| `core/elidex-ecs/src/dom/tree/teardown.rs` | 2 | `:90`, `:221`: raw. One read serves two relations today (the node's place in a child list, and the node whose version is bumped); `ts` splits it (umbrella §2 LIFE × TREE) | — |
| `core/elidex-ecs/src/dom/tree/walkers.rs` | 6 | `:245` `is_connected` S; `:295` `ancestor_chain`: split, R-9; `:364` `is_light_tree_ancestor_or_self` T, g; `:380` H; `:415` `find_tree_root` T, g; `:436` S | [D1], [D2], [D3], [D4] |
| `core/elidex-render/src/builder/paint/mod.rs` | 1 | F (`find_nearest_layout_box`) | R-3 |
| `core/elidex-render/src/builder/walk.rs` | 2 | `:232` "root is always a stacking context": F; `:499` multicol ancestor: F | R-3 |
| `css/elidex-css/src/selector/matching.rs` | 6 | L (`:26`, `:40`, `:45` combinators with the marker stop; `:189`, `:192`, `:194` structural pseudo-classes) | R-4 |
| `css/elidex-css/src/selector/traverse.rs` | 1 | L (`:root`) | R-4 |
| `css/elidex-style/src/generated_content.rs` | 2 | T1 (a pseudo-element's originating element) | — |
| `css/elidex-style/src/walk.rs` | 1 | `:126` the root scan of the style walk: S (a shadow root is reached through its host, so it is not a walk root) | R-5 |
| `dom/elidex-a11y/src/tree.rs` | 2 | F; the explicit hop at `:237-238` reads the host through `shadow_host` | R-3 |
| `dom/elidex-dom-api/src/char_data/split_text.rs` | 3 | T1 | — |
| `dom/elidex-dom-api/src/child_node/mod.rs` | 2 | T1 | — |
| `dom/elidex-dom-api/src/child_node/mutations.rs` | 5 | T1 (the ChildNode mixin is on DocumentType, Element and CharacterData, not on ShadowRoot; the proof is owed line by line in `ta`) | — |
| `dom/elidex-dom-api/src/child_node/selectors.rs` | 1 | `closest`: T, g (the walk stops at the first non-element parent) | R-6 |
| `dom/elidex-dom-api/src/element/inheritance.rs` | 2 | `is_content_editable`: P:te. Draft 1 read this file as the `lang` inheritance; it is contenteditable. No `lang` or `dir` climb exists in the code | R-1 |
| `dom/elidex-dom-api/src/element/layout_query.rs` | 6 | F (`scrollIntoView`'s scrolling-box ancestors, `offsetParent`, the scroll offset of `getBoundingClientRect`) | R-2 |
| `dom/elidex-dom-api/src/element/markup.rs` | 1 | T1 (`outerHTML` setter's parent test) | — |
| `dom/elidex-dom-api/src/element/option_disabled.rs` | 2 | P:tf (optgroup ancestor) | [H9] |
| `dom/elidex-dom-api/src/element/select.rs` | 3 | T1 | — |
| `dom/elidex-dom-api/src/element/table_mutation/mod.rs` | 7 | T1 | — |
| `dom/elidex-dom-api/src/element/tree.rs` | 3 | T1 | — |
| `dom/elidex-dom-api/src/focus/predicate.rs` | 2 | `:126` T1 (a summary's parent details); `:313` `is_in_hidden_subtree`: F | R-7 |
| `dom/elidex-dom-api/src/focus/sot.rs` | 1 | `is_in_document`: S | R-8 |
| `dom/elidex-dom-api/src/node_methods/mod.rs` | 1 | `:67` `find_root`: removed, its two callers take the one shadow-including root | [D4] |
| `dom/elidex-dom-api/src/range/boundary.rs` | 5 | P:tr | [D8] |
| `dom/elidex-dom-api/src/range/mod.rs` | 4 | P:tr (`:126`, `:137` root and common ancestor; `:552`, `:582` walk) | [D8] |
| `dom/elidex-dom-api/src/range/mutation.rs` | 2 | P:tr | [D8] |
| `dom/elidex-dom-api/src/selection.rs` | 1 | P:tr | [D8] |
| `dom/elidex-dom-api/src/traversal.rs` | 10 | P:tr (bounded by the traversal root in most paths; proof per line in `tr`) | [D10] |
| `dom/elidex-dom-api/src/tree_nav.rs` | 2 | T, h (`GetParentNode`, `GetParentElement`; the VM natives guard the receiver at `node_proto.rs:328`, `:345`) | [D7] |
| `dom/elidex-dom-compat/src/presentational.rs` | 2 | T, g (the cell-to-table walk breaks at the first parent with no `TagType`, which a shadow root is) | R-10 |
| `dom/elidex-form-core/src/fieldset.rs` | 4 | P:tf (*disabled fieldset*: "descendant of") | [H3], [H8] |
| `dom/elidex-form/src/select/mod.rs` | 4 | P:tf (*nearest ancestor select*) | [H4] |
| `dom/elidex-form/src/submit.rs` | 3 | P:tf (*reset the form owner* step 4; "nearest ancestor dialog") | [H2], [H5] |
| `dom/elidex-html-parser-strict/src/tree_builder/mod.rs` | 1 | T, g (`:308`) | [H2] |
| `layout/*` (8 files, verified 2026-11-15 with the P-UP command over `-- crates/layout`) | 10 | six "is this the root" tests (`block/mod.rs:627`, `elidex-layout-flex/src/lib.rs:510`, `elidex-layout-grid/src/lib.rs:625`, `elidex-layout-multicol/src/lib.rs:341`, `elidex-layout-table/src/lib.rs:791`, `elidex-layout/src/hit_test.rs:187`): F; `shift.rs:188` (the IFC element of a run): F; table row / section parent (`fragment.rs:74`, `:81`, `lib.rs:130`): T1 | R-3 |
| `script/elidex-js/src/vm/host/html_legend_proto.rs` | 1 | T1 | — |
| `script/elidex-js/src/vm/host/node_proto.rs` | 2 | T, g (`:331`, `:348`; the receiver guards at `:328`, `:345` fold into `tree_parent`) | [D7] |
| `script/elidex-js/src/vm/host/range_proto.rs` | 1 | `:529` `require_attached_node`: P:tr (a shadow-root argument has a null parent by spec; base finds the host) | R-11 |
| `script/elidex-script-session/src/event_dispatch/mod.rs` | 1 | E (`:217`; the break at `:214` stays, C-5 role G) | [D11] |
| `script/elidex-script-session/src/mutation/child_list.rs` | 4 | T1 | — |
| `script/elidex-script-session/src/mutation/html_fragment.rs` | 5 | T1 (`:80-81` already resolves a shadow-root receiver through `shadow_host`) | — |
| `script/elidex-wasm-runtime/src/runtime.rs` | 1 | T1 (an inline test assertion) | — |
| `shell/elidex-shell/src/app/events.rs` | 1 | E (the link whose activation behaviour a click runs) | R-12 |
| `shell/elidex-shell/src/app/hover.rs`, `content/scroll.rs` | 2 | F (`:hover` chain; scroll target) | R-3 |
| `shell/elidex-shell/src/content/form_input.rs` | 2 | E (`find_label_ancestor`: the label whose activation behaviour a click runs). Draft 1 read it as a form-ancestor climb | R-12 |
| `shell/elidex-shell/src/content/iframe/lifecycle.rs` | 1 | T1 (`:604`, an inline test: the iframe's parent in the document tree) | — |

Total: 143 lines in 60 files (`git grep -n -P '\bget_parent\(' c0578223 -- crates ':!*tests*' | cut -d: -f2 | sort -u | wc -l`
gives 60). **Predicted backlog** (lines, from this table; `ta`'s compiler list is binding): P:tm 2, P:tr 23, P:tf 13,
P:te 3, in total 41; F 21, E 4, L 7. No row is left unclassified, and no pending line is without a slice or an
owner.

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
makes it unnecessary to guess, since it replaces the raw and the "exposed" accessors by one set with new names
(umbrella §1.3): every one of these lines fails to compile in `tc` and is listed in its plan with the consumer read. Known unguarded reader, from the Range contents program's own defect table: D13 (a), (b)
(`docs/plans/2026-11-range-contents-program.md:731`).

## C-4. P-CHX: compensating child-side accessors

Command per row: `git grep -n -P '\b<pattern>' c0578223 -- crates ':!*tests*' | wc -l`.

| Accessor | Pattern | Lines | What compensates |
|---|---|---|---|
| `children_iter` | `children_iter\(` | 86 | `ChildrenIter`'s `next` skips a `ShadowRoot` entity (`crates/core/elidex-ecs/src/dom/mod.rs:911`) |
| `children_iter_rev` | `children_iter_rev\(` | 9 | `ChildrenIterRev`'s `next` (`mod.rs:944`) |
| `children` | `\.children\(` | 59 (over-count: any `.children(` receiver) | its own skip at `navigation.rs:265` |
| `child_list_uncapped` | `child_list_uncapped\(` | 13 | skip at `navigation.rs:292` |
| `index_in_parent` | `index_in_parent\(` | 9 | skips shadow-root predecessors (`navigation.rs:70`) |
| `next_exposed_sibling` / `prev_exposed_sibling` | `(next\|prev)_exposed_sibling\(` | 30 | skip at `navigation.rs:158`, `:172` |

Under the rule (umbrella §1.3) none of these needs its skip, and the last row has no reason to exist beside the raw
sibling accessors.

## C-5. P-MARK and P-HOST by role

The 61 true P-MARK lines, read once and grouped by what the marker test is for. Roles **K**, **R** and **B** are
the compensation the program deletes; **C** becomes a read through `shadow_host`; **G**, **Ls**, **I** and **D**
stay. Draft 2 splits three roles that draft 1 had merged (R1 plan-review): receiver guards (**R**) out of K, the
spec's own conditional step (**G**) and the selector boundary (**Ls**) out of B, and *retarget*'s hop out of I into
C.

| Role | Meaning | Lines (file:line under `crates/`) |
|---|---|---|
| **K** child-side compensation: "this child is a shadow root, skip it" | `core/elidex-ecs/src/dom/mod.rs:911`, `:944`; `dom/shadow.rs:366`, `:467`; `dom/tree/navigation.rs:70`, `:158`, `:172`, `:265`, `:292`; `css/elidex-style/src/walk.rs:519`; `dom/elidex-dom-api/src/live_collection/mod.rs:443` (11). Two of them re-test a child that a compensating accessor already filtered (`walk.rs:519` over `children`, `live_collection/mod.rs:443` over `children_iter`) |
| **R** receiver guard on the child side: "a shadow root has no siblings" | `script/elidex-js/src/vm/host/node_proto.rs:384`, `:397` (2). They test the **receiver**, not a child: `next_exposed_sibling` / `prev_exposed_sibling` skip a shadow-root result but do not refuse a shadow-root receiver. `tc` moves this guard into the one accessor set, `ts` deletes it with the storage |
| **B** parent-side compensation: "the climb reached a shadow root, stop" | `core/elidex-ecs/src/dom/tree/walkers.rs:357`, `:410`, `:416`; `dom/shadow.rs:492`; `dom/elidex-html-parser-strict/src/tree_builder/mod.rs:308`; `script/elidex-js/src/vm/host/node_proto.rs:328`, `:345` (7) |
| **G** the spec's conditional step, not compensation | `script/elidex-script-session/src/event_dispatch/mod.rs:214` (1): `!composed && ShadowRoot → break` is the null arm of a shadow root's *get the parent* (DOM §4.8). It stays at every head |
| **Ls** the selector boundary | `css/elidex-css/src/selector/matching.rs:34`, `:49` (2): the combinator climb stops at the shadow root. Whether the featureless host belongs there is pending (PM 748 (c)); this program does not change the lines |
| **C** crossing hop: reads `.host` to continue upward | `core/elidex-ecs/src/dom/mod.rs:761`; `dom/tree/mutation.rs:420`; `dom/tree/walkers.rs:385`, `:438`; `dom/elidex-a11y/src/tree.rs:237`; `script/elidex-script-session/src/event_dispatch/mod.rs:595` (*retarget* step 2, the `.host` read is at `:605`) (6). The three elidex-ecs ones that read the tree parent first are the branches the independent verification found unreachable for an attached shadow root |
| **E** event suppression: "node or parent is a shadow root, fire nothing" | `core/elidex-ecs/src/dom/tree/mutation.rs:528`, `:582` (2). The **parent** half is relied on by `LiveRangeRegistry` (`dom/elidex-dom-api/src/range/live.rs:38-54`) and by Layout's T3; the **node** half becomes unreachable in `ts` (umbrella §2 MEV × TREE) |
| **L** lifetime: the strip | `core/elidex-ecs/src/dom/tree/teardown.rs:69`, `:81` (2); owned by the ecs-entity-lifetime program |
| **I** identity: "is this node a ShadowRoot" for dispatch, brand or kind | `core/elidex-ecs/src/dom/tree/navigation.rs:89`, `:90` (the predicate); `dom/classify.rs:119` (`is_document_fragment` excludes a shadow root); `dom/elidex-dom-api/src/child_node/mod.rs:322`, `document.rs:345`, `node_methods/clone.rs:115`, `range/mutation.rs:502`; `script/elidex-js/src/vm/host_data/engine_feature/entity_kind.rs:42`, `:114`, `:173`; `script/elidex-script-session/src/mutation/child_list.rs:190`, `mutation/html_fragment.rs:80`; `event_dispatch/mod.rs:173`, `:183`, `:635` (15). Four of these reject inserting a shadow root as a node (`child_node/mod.rs:322`, `document.rs:345`, `range/mutation.rs:502`, `child_list.rs:190`): they stay, the spec forbids the insertion under either model |
| **D** data: reads another field of the component (mode, slot assignment, delegates focus, clonable, serializable) | `core/elidex-ecs/src/dom/shadow.rs:266`, `:271`, `:406`, `:419`, `:523` (`:266` and `:523` read `host` itself: the link, P-HOST); `dom/elidex-dom-api/src/element/markup.rs:307`, `focus/delegate.rs:101`, `node_methods/clone.rs:453`; `script/elidex-js/src/vm/host/element_shadow.rs:122`, `shadow_root_proto.rs:188`; `script/elidex-script-session/src/mutation/html_fragment.rs:464`, `:504`, `:630` (13) |

11 + 2 + 7 + 1 + 2 + 6 + 2 + 2 + 15 + 13 = 61 (K, R, B, G, Ls, C, E, L, I, D). The role of each line is from a
single read at line level; `tc` (roles K, R) and `ta` (roles B, C) re-derive their rows from the compiler's list and
treat any disagreement with this table as a finding against the table.

P-HOST in shadow-related files (14 lines): `core/elidex-ecs/src/dom/shadow.rs` 5, `dom/tree/walkers.rs` 2,
`dom/tree/teardown.rs` 1, `dom/tree/mutation.rs` 1, `dom/mod.rs` 1, `dom/elidex-a11y/src/tree.rs` 2,
`script/elidex-script-session/src/event_dispatch/mod.rs` 1, `script/elidex-js/src/vm/host/shadow_root_proto.rs` 1
(`git grep -n -P '\.host\b(?!\s*\()' c0578223 -- crates ':!*tests*' | cut -d: -f2 | sort | uniq -c`). Two of the 14
are doc comments (`shadow.rs:512`, `:518`) and one is a doc comment in `tree.rs:223`, so 11 lines read the field.

**Who still reads the field after `ta`** (the input to "what PR (io)'s fold has to edit", umbrella §7.2). `ta`
routes through `shadow_host` every reader that climbs or lives outside elidex-ecs: `dom/mod.rs:761`,
`tree/mutation.rs:420`, `tree/walkers.rs:386`, `:439`, `elidex-a11y` `tree.rs:238`, `event_dispatch/mod.rs:605`,
`shadow_root_proto.rs:203` (7). Three readers remain, all inside elidex-ecs and all holding the component for
another reason: `shadow.rs:267` and `:421` (slot assignment reads the mode and the host from one borrow),
`tree/teardown.rs:69` (the strip, the lifetime program's). With `shadow_host` itself (`shadow.rs:525`) and the one
writer (`shadow.rs:198`, the component literal in `attach_shadow_with_init`), that is the complete list. The
ecs-entity-lifetime program's PR (io) plan §10 is the census of this field's readers at its base; this program does
not repeat it.

## C-6. P-DER: derived relations, by property

**Property.** A *derived relation* is a read-only function (no `&mut` in its signature) whose body reads a parent
link (level 1), or whose body calls a level-1 member (level 2). A caller of such a function obtains a relation
without writing a step, so deleting `get_parent` does not make the compiler report it. Draft 1 listed these
functions by hand and missed `is_ancestor_or_self` (R1 plan-review, CRIT): it has 16 call lines outside elidex-ecs
(`git grep -n -P '\bis_ancestor_or_self\(' c0578223 -- crates ':!*tests*'` prints 18 lines: the definition, one
comment, 16 calls) and none of them calls `get_parent`.

**Instrument.** The script of C-11 (`python3 <script> c0578223`, run 2026-11-15). It splits every non-test `.rs`
file under `crates/` into function bodies, marks a body that contains `get_parent(` or, inside elidex-ecs, a read of
a `.parent` field, and then marks every function whose body calls a read-only level-1 name. Limits: braces inside
string or character literals are counted as braces; a call is matched by bare name, so two functions with one name
are one name (three generic names, `invoke`, `walk`, `notify`, are excluded from the level-2 seed for that
reason); an inline `#[cfg(test)] mod` is cut. Level 3 and beyond are **not** measured by name (the same script
iterated gives 710 functions at level 3, most of them through shared names such as `matches`); from level 2 on,
the population is continued by the compiler in `ta` (umbrella §1.2).

**Counts.** Level 1: 114 functions climb; 60 are read-only (the table below), 54 mutate (the complement: handlers,
natives, mutation and layout entry points, whose climbing lines are P-UP lines and fail to compile in `ta`).
Level 2: 113 functions call a read-only level-1 name; 45 are read-only (second table), 68 mutate.

Disposition in `ta`: **RM** removed (folded into another function); **RN** renamed to a name that states the
relation; **PF** renamed with the suffix `_pending_flip` (the spec relation is the tree parent, base crosses; the
flip slice renames it back); **ND** kept, its step takes the pending name of a relation this program does not define
(umbrella §1.5); **K** kept, because the relation it computes at base is the relation its name states and no slice
changes it. A member that is in none of these lists is renamed: the unknown case falls on the enumerated side.

Level 1, read-only (60):

| Function (file under `crates/`) | Relation at base | Disposition |
|---|---|---|
| `is_ancestor` (`core/elidex-ecs/src/dom/tree/mod.rs:40`) | inclusive ancestor through the stored parent: crosses a shadow root, not a template's contents | RM. The three cycle guards take the host-including test |
| `walk_parent` (`dom/tree/mutation.rs:418`) | one shadow-including step | RM (`shadow_including_parent`) |
| `ancestor_chain` (`dom/tree/walkers.rs:291`) | the crossing chain under `tree_order_cmp` | RM, split by C-8 R-9 |
| `shadow_root_entity_for_slot` (`dom/shadow.rs:489`) | tree root of a slot | RM (`find_tree_root`) |
| `find_root` (`dom/elidex-dom-api/src/node_methods/mod.rs:64`) | shadow-including root | RM (the one shadow-including root) |
| `find_tree_root_composed` (`dom/tree/walkers.rs:432`) | shadow-including root | RN (the spec's name; "composed" is not the relation). 2 callers (verified 2026-11-15): `consumer.rs:408`, `node_proto.rs:455` |
| `is_host_including_ancestor_or_self` (`walkers.rs:373`) | host-including inclusive ancestor, shadow hosts only | RN. 3 callers (verified 2026-11-15) re-choose: `child_node/mod.rs:260`, `:445` take H; `focus/delegate.rs:88` takes the shadow-including test (its own comment names that relation), so the template host added after PR (io) does not widen it |
| `is_light_tree_ancestor_or_self` (`walkers.rs:348`) | inclusive ancestor | RN (the tree test; "light tree" is not the relation: it is also the test inside a shadow tree). 3 callers (verified 2026-11-15): `equality.rs:193`, `:198`, `node_proto.rs:647` |
| `index_in_parent` (`dom/tree/navigation.rs:61`) | index among tree children | RN in `tc` (it joins the one child-side set); in `ta` only its parent read changes (raw) |
| `is_contenteditable` (`core/elidex-ecs/src/dom/mod.rs:364`), `is_content_editable` (`dom/elidex-dom-api/src/element/inheritance.rs:42`) | contenteditable inheritance, crossing | PF (te) |
| `is_option_disabled` (`element/option_disabled.rs:30`), `is_fieldset_disabled`, `is_in_first_legend` (`dom/elidex-form-core/src/fieldset.rs:46`, `:28`), `find_option_select`, `find_options_container` (`dom/elidex-form/src/select/mod.rs:200`, `:245`), `find_dialog_ancestor`, `find_form_ancestor` (`dom/elidex-form/src/submit.rs:236`, `:263`) | form-association ancestors, crossing | PF (tf), 7 functions |
| `intersects_node` (`dom/elidex-dom-api/src/range/boundary.rs:235`), `child_index_in_ancestor`, `common_ancestor_container`, `next_in_preorder_global` (`range/mod.rs:549`, `:121`, `:573`), `node_fully_contained` (`selection.rs:651`), `next_in_preorder`, `next_in_preorder_skip_subtree`, `prev_in_preorder` (`traversal.rs:87`, `:110`, `:124`) | Range and traversal relations, crossing | PF (tr), 8 functions |
| `find_nearest_layout_box`, `multicol_container_writing_mode` (elidex-render), `is_sectioning_content_descendant` (elidex-a11y), `find_offset_parent`, `accumulated_scroll_offset` (`layout_query.rs`), `is_in_hidden_subtree` (`focus/predicate.rs:303`), `collect_hover_chain`, `find_scroll_target` (elidex-shell) | flat-tree ancestors, approximated by the stored parent | ND (F), 8 functions |
| `build_propagation_path` (`event_dispatch/mod.rs:203`), `find_link_ancestor_with_target` (`app/events.rs:204`), `find_label_ancestor` (`content/form_input.rs:308`) | event *get the parent* chain | ND (E), 3 functions |
| `match_components`, `match_pseudo_class` (`selector/matching.rs:12`, `:185`), `is_root_element` (`selector/traverse.rs:24`) | the selector tree's parent | ND (L), 3 functions |
| `find_tree_root` (`walkers.rs:408`), `is_connected` (`walkers.rs:232`), `root_entities` (`navigation.rs:372`) | tree root; connected; parentless non-fragment entities | K. Each name is the relation, and it is the same relation at every head (the marker stop, then the storage). 24, 26 and 5 lines call them (`git grep -n -P '\b<name>\(' c0578223 -- crates ':!*tests*' \| wc -l`, definitions included) |
| `is_child_of` (`dom/tree/mod.rs:57`) | one stored-link equality, `pub(super)` | K as a raw read (P-REL); re-read by `ts` |
| `find_roots` (`css/elidex-style/src/walk.rs:119`), `is_in_document` (`focus/sot.rs:69`) | shadow-including (C-8 R-5, R-8) | K; the step takes `shadow_including_parent` |
| `resolve_pseudo_content`, `ensure_pre_insertion_validity`, `ensure_replace_validity`, `ancestor_table`, `cell_index`, `section_row_index`, `resolve_adjacent_site`, `is_first_summary_of_details`, `get_cellpadding_hints`, `compute_table_fragmentation`, `collect_row_group_infos`, `capture_move_source`, `source_removal_record`, `apply_set_outer_html_replaces_element_in_parent` | a single hop, or a walk with a guard (C-2: T1 / T, g) | K, 14 functions |

5 + 4 + 17 + 14 + 20 = 60 (RM, RN, PF, ND, K).

Level 2, read-only (45), by what the row needs. A level-2 member is renamed only where its callers do not all take
one relation (the delegate passes the choice through); otherwise its body is one of the compiler's lines in `ta`.

| Function | Calls | Disposition |
|---|---|---|
| `is_ancestor_or_self` (`walkers.rs:330`) | `is_ancestor` | RM. Its 16 call lines choose: host-including 6 (`element/tree.rs:33`, `range/mutation.rs:486`, `:490`, `child_list.rs:216`, `:310`, `:432`: *ensure pre-insert validity* step 2); shadow-including 1 (`event_dispatch/mod.rs:616`, *retarget*'s "shadow-including inclusive ancestor"); tree, as the VM native does, 1 (`node_methods/core.rs:41`, the `Contains` handler, PM 748 (b)); pending 8 (`range/mod.rs:188`, `:196`, `:525`, `:532`, `range/mutation.rs:123`, `traversal.rs:1054` → tr; `element/select.rs:156`, `:174` → tf) |
| `tree_order_cmp` (`walkers.rs:256`) | `ancestor_chain` | RM, split by C-8 R-9. Its 5 call lines choose |
| `find_tree_root` (the delegate at `event_dispatch/mod.rs:611`), `find_root_non_composed` (`node_methods/mod.rs:79`), `find_link_ancestor` (`app/events.rs:196`), `offset_from_parent` (`layout_query.rs:384`) | one member each | body lines of `ta`. (`is_in_subtree_of`, `event_dispatch/mod.rs:615`, is level 3 through `is_ancestor_or_self`: the compiler reports it when that function is removed, and it becomes the shadow-including test) |
| `compare_document_position` (`equality.rs:141`), `find_radio_group_scoped` (`radio.rs:109`) | `find_tree_root`, then the order | tree order, proved equal at base (C-8 R-9) |
| `compare_points`, `compare_point`, `is_point_in_range`, `contained_top_level_nodes`, `to_string` (elidex-dom-api `range/`) | Range members | listed in the `tr` plan with their cells |
| `resolve_form_owner` (`radio.rs:15`), `find_option_index_in_tree`, `select_get_value`, `select_selected_index`, `build_form_submission`, `is_constraint_validation_candidate` | form members | listed in the `tf` plan with their cells |
| `match_form_pseudo_class` (`selector/matching.rs:235`) | `is_contenteditable` | listed in the `te` plan (`:read-write`) |
| the remaining 25 (`assigned_nodes`, the render / layout / hit-test / style root finders, `matches`, `match_simple`, `determine_role`, `adopt_pair`, `in_document_light_tree`, `row_index`, `get_the_focusable_area`, `is_focusable`, `tab_index_default_for`, `current_focus`, `get_presentational_hints`, `resolve_input_list`, `resolve_label_for`, `build_dispatch_plan`, `composed_path_for_js`, `retarget`, `convert_arg_source_records`, and others the script prints) | a K or ND member | no change of relation; they compile unchanged or with the renamed callee |

## C-7. What this census does not establish

1. The per-call relation of the 142 P-UP calls (C-2 is file-level; `ta`'s compiler list is binding).
2. The guarded / unguarded split of P-CH (C-3).
3. Any runtime behaviour. Every "P:" in C-2 and every "T, g" / "T1" equality is a reading of the code; none was
   executed. The probe gate (umbrella §5.2) runs them before the consuming plan's review.
4. Test-path lines beyond their counts (P-UP-T 31, P-CH 34): which of them pin the (Y) storage as a contract (a
   shadow root found in its host's raw child list, `get_parent(sr) == Some(host)`).
5. P-DER beyond level 2 (C-6, *Instrument*).
6. P-MARK's instrument sees a marker test written as a component query or as `is_shadow_root(`. It does not see the
   VM's brand predicates built on the same component (`is_shadow_root_entity`, `brand_shadow_root`,
   `read_shadow_component` in `crates/script/elidex-js/src/vm/host/`); they are identity or data reads (roles I
   and D) and no slice changes them.

## §C8. Spec coverage map for the rows draft 1 left at "?" (C-8)

Each row was looked up with `.claude/tools/webref` on 2026-11-15 (the `body` command is in the row); the quoted
words are the spec's. "Base" is the reading of the code at `c0578223`. The row id (R-n) is what C-2 and the umbrella
cite.

| Spec section | Step | Branch (the spec's words) | Touch (site → relation; base) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §6.8.1 Making document regions editable: The contenteditable content attribute | the inherit state | "The inherit state indicates that the element is editable (or not) based on the parent element's state." (`body html contenteditable`) | **R-1**: `is_contenteditable` (`core/elidex-ecs/src/dom/mod.rs:364`), `is_content_editable` (`element/inheritance.rs:42`) → tree parent (a child of a shadow root has no parent element); base crosses to the host: P:te | ✓ | yes |
| CSSOM View (cssom-view-1) §7 Extensions to the HTMLElement Interface | `offsetParent` step 2; §6.1 *scroll a target into view* | "Let ancestor be the parent of the element in the flat tree"; "For each ancestor element or viewport that establishes a scrolling box" (`body cssom-view-1 extensions-to-the-htmlelement-interface`, `body cssom-view-1 extension-to-the-element-interface`) | **R-2**: `layout_query.rs` (6 lines) → flat tree (not defined here); base reads the stored parent: F | ✓ | yes |
| CSS Shadow Module Level 1 (css-shadow-1) §4.1 Flattening the DOM into an Element Tree | *flat tree*; Selectors L4 §9.1 `:hover` | "the rest of CSS needs a single unified tree structure to work with"; "one of its descendants in the flat tree" (`body css-shadow-1 flattening`, `body selectors-4 the-hover-pseudo`) | **R-3**: render `paint/mod.rs:795`, `walk.rs:232`, `:499`; the layout root tests and `shift.rs:188`; a11y `tree.rs`; shell hover and scroll → flat tree (not defined here); "is this the root" is "has no flat-tree parent"; base reads the stored parent: F | ✓ | yes |
| CSS Shadow Module Level 1 (css-shadow-1) §3.2.2 Selecting Shadow Hosts from within a Shadow Tree | the host in its shadow tree; §3.2.1 the match list | "For the purpose of Selectors, a shadow host also appears in its shadow tree, with the contents of the shadow tree treated as its children. (In other words, the shadow host is treated as replacing the shadow root node.)"; "the selector match list is initially the shadow host, followed by all children of the shadow tree's shadow root and their descendants" (`body css-shadow-1 host-element-in-tree`, `body css-shadow-1 selectors-data-model`) | **R-4**: `selector/matching.rs` (6), `selector/traverse.rs` (1) → the selector tree's parent (not defined here; pending, PM 748 (c)); base is the tree parent with a stop at the shadow root: L | ✓ | yes |
| WHATWG DOM §4.9 Interface Element | `closest` step 3 | "Let elements be this's inclusive ancestors that are elements, in reverse tree order." (`body dom dom-element-closest`) | **R-6**: `child_node/selectors.rs` → tree parent; base stops at the first non-element parent: T, g | ✓ | yes |
| WHATWG HTML §15.1 Introduction | *being rendered* | "if it has any associated CSS layout boxes"; "The presence of the hidden attribute normally means the element is not being rendered" (`body html being-rendered`) | **R-7**: `focus/predicate.rs:313` `is_in_hidden_subtree` → a rendering fact, so the flat tree (not defined here); base reads the stored parent, and its doc says so on purpose: F | ✓ | yes |
| WHATWG DOM §4.2.2 Shadow tree | *connected* | "A node is connected if its shadow-including root is a document" (`body dom concept-shadow-tree`) | **R-8**: `focus/sot.rs:69` `is_in_document` (the same question, for one given document) → shadow-including; base reads the stored parent: S, equal | ✓ | yes |
| WHATWG DOM §1.1 Trees | *tree order*; §4.4 `compareDocumentPosition` step 6; §5.2 *position* | "If node1 or node2 is null, or node1's root is not node2's root, then return the result of adding DOCUMENT_POSITION_DISCONNECTED, …"; "Assert: nodeA and nodeB have the same root." (`body dom interface-node`, `body dom boundary-points`) | **R-9**: `tree_order_cmp` and its 5 call lines (`git grep -n -P '\btree_order_cmp\(' c0578223 -- crates ':!*tests*'` prints 6 lines with the definition). Tree order for `compare_document_position` (`equality.rs:203`: it returns before the call when the tree roots differ) and for `find_radio_group_scoped` (`radio.rs:143`: the group is filtered to one tree root first): equal at base, taken in `ta`. Range's `compare_points` (`range/mod.rs:538`): pending, tr. Paint and hit-test layer order (`builder/walk.rs:620`, `hit_test.rs:235`): the flat tree's order (not defined here). Base: one crossing chain for all five | ✓ | yes |
| WHATWG HTML §15.3.8 Tables | `cellpadding` | "any td and th elements that have corresponding cells in the table corresponding to the table element" (`body html tables-2`) | **R-10**: `presentational.rs` `get_cellpadding_hints` → tree parent (the table model is built from children); base breaks at a parent without `TagType`: T, g | ✓ | no |
| WHATWG DOM §5.6 Interface Range | `setStartBefore` steps 1-2 | "Let parent be node's parent. If parent is null, then throw an \"InvalidNodeTypeError\" DOMException." (`body dom dom-range-setstartbefore`) | **R-11**: `range_proto.rs:529` → tree parent (a shadow root's parent is null); base finds the host for a shadow-root argument: P:tr | ✓ | yes |
| WHATWG DOM §2.9 Dispatching events | *dispatch*, the activation target | "Let parent be the result of invoking target's get the parent with event."; "If isActivationEvent is true, event's bubbles attribute is true, activationTarget is null, and parent has activation behavior, then set activationTarget to parent." (`body dom dispatching-events`) | **R-12**: `app/events.rs:204`, `content/form_input.rs:308` → the event path (not defined here); base reads the stored parent: E | ✓ | yes |

**R-5** has no spec row: `css/elidex-style/src/walk.rs:126` is an engine scan for walk roots, used when no document
root is cached. An entity is a walk root when no other walk reaches it, and the style walk reaches a shadow root
through its host, so the relation is shadow-including; base reads the stored parent: S, equal.

The remaining file-level rows of draft 1 that were not "?" keep their relation, with two corrections recorded in
C-2: `element/inheritance.rs` is contenteditable, not `lang`; `content/form_input.rs` is a label lookup, not a
form-ancestor lookup.

## C-9. P-WR: who writes a tree edge

`git grep -n -P '\.parent\s*=[^=]' c0578223 -- crates/core/elidex-ecs/src ':!*tests*'` prints 3 lines. One site
writes `Some(parent)`: `link_node` (`crates/core/elidex-ecs/src/dom/tree/mutation.rs:449`, the write at `:457`).
`git grep -n -P '\blink_node\(' c0578223 -- crates ':!*tests*'` prints its definition and three callers:
`append_child` (`mutation.rs:53`), `insert_before` (`:132`), `replace_child` (`:214`). No function outside the
`dom::tree` module can write a link field (`pub(crate)` fields, and `link_node` / `update_rel` are `pub(super)`).

At base nothing in elidex-ecs refuses to link a shadow root: the four insertion guards of C-5 role I are in
elidex-dom-api and elidex-script-session, and `attach_shadow_with_init` itself calls `append_child(host, sr)`. Slice
`ts` adds the refusal at the three callers' validity check, beside the cycle guard (umbrella §2 TREE × WR).

## C-10. The VM natives and the dom-api handlers that answer the same question

PM 748 (b): a VM native chooses the elidex-ecs relation directly (marshalling); the dom-api navigation handler for
the same member takes the **same** relation in `ta`, mechanically, with no new cell: in `ta` the handler calls what
the native calls. Whether a handler is connected or deleted is `#11-dom-api-handlers-unreachable-from-production`'s.
The last column names the slice in which the native and the handler together move to a new name.

| Member | VM native (`crates/script/elidex-js/src/vm/host/`) | dom-api handler (`crates/dom/elidex-dom-api/src/`) | Relation both take in `ta`; later move |
|---|---|---|---|
| `parentNode`, `parentElement` | `node_proto.rs:318`, `:339` (receiver guard, then `get_parent`) | `tree_nav.rs` `GetParentNode` `:78`, `GetParentElement` `:101` (no guard) | tree parent |
| `firstChild`, `lastChild`, `childNodes` | `node_proto.rs` (`children_iter`, `children_iter_rev`) | `tree_nav.rs:125`, `:148` (raw) | tree children: the handlers take the natives' iterators in `ta`; both move to the one set in `tc` |
| `nextSibling`, `previousSibling` | `node_proto.rs:384`, `:397` (receiver guard, then the exposed sibling) | `tree_nav.rs:171`, `:194` (raw) | tree siblings: the handlers take the natives' guard and exposed accessor in `ta`; both move to the one set in `tc` |
| element child / sibling members | — | `tree_nav.rs:217`, `:241`, `:266`, `:291` (raw, filtered by `is_element`, which a shadow root fails) | tree children, unchanged in `ta`; the one set in `tc` |
| `contains` | `node_proto.rs:647` (`is_light_tree_ancestor_or_self`) | `node_methods/core.rs:41` (`is_ancestor_or_self`, crossing) | tree inclusive ancestor |
| `isConnected` | `node_proto.rs:455` (`find_tree_root_composed`, compared with the bound document: `#11-vm-isconnected-bound-document`) | `node_methods/core.rs:197` (`find_root`) | connected; the native's bound-document comparison is that slot's |
| `getRootNode` | — | `node_methods/core.rs:225` (`find_root` / `find_root_non_composed`) | shadow-including root or tree root, by the option |
| Range sibling-slot methods | `range_proto.rs:529` (`require_attached_node`) | `range/boundary.rs:82-110` | pending in `ta`, tagged `Flip::Tr` (NEW); the tree parent in `tr` |
| Range root comparisons | `range_proto.rs:722`, `static_range_proto.rs:382` (`find_tree_root`) | `range/boundary.rs:158`, `:201` | tree root, unchanged |

## C-11. The P-DER script

Run from the repository root: save the block as a file and run `python3 <file> c0578223`. It prints the two count
lines and one row per function (level, RO / MUT, `file:line`, name, the level-1 name it calls).

```python
import re, subprocess, sys
rev = sys.argv[1]
git = lambda *a: subprocess.run(('git',) + a, capture_output=True, text=True).stdout
FN = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+(\w+)')
TESTMOD = re.compile(r'\s*(pub\s+)?mod\s+\w+\s*\{')
fns = []                                    # (file, line, name, signature, body)
for f in git('ls-tree', '-r', '--name-only', rev, '--', 'crates').split('\n'):
    if not f.endswith('.rs') or 'tests' in f:
        continue
    lines = git('show', f'{rev}:{f}').split('\n')
    for i, l in enumerate(lines):           # cut an inline `#[cfg(test)] mod x {` to the end of the file
        if l.strip().startswith('#[cfg(test)]') and i + 1 < len(lines) and TESTMOD.match(lines[i + 1]):
            lines = lines[:i]
            break
    code = [re.sub(r'//.*$', '', l) for l in lines]
    for i, l in enumerate(code):
        m = FN.match(l)
        if not m:
            continue
        depth, started, text = 0, False, []
        for l2 in code[i:]:
            if not started and ';' in l2 and '{' not in l2:
                break                       # a declaration without a body
            depth += l2.count('{') - l2.count('}')
            started = started or '{' in l2
            text.append(l2)
            if started and depth <= 0:
                break
        if started:
            sig, _, body = '\n'.join(text).partition('{')
            fns.append((f, i + 1, m.group(1), sig, body))
ECS = 'crates/core/elidex-ecs/'
def climbs(f, body):                        # the property: the body reads a parent link
    return bool(re.search(r'\bget_parent\(', body)) or (
        f.startswith(ECS) and bool(re.search(r'\.parent\b(?!\s*\()(?!\s*=[^=])', body)))
l1 = [x for x in fns if x[2] != 'get_parent' and climbs(x[0], x[4])]
ro = lambda x: '&mut' not in x[3]           # read-only: no `&mut` in the signature
names = sorted({x[2] for x in l1 if ro(x)} - {'invoke', 'walk', 'notify'})
call = re.compile(r'\b(' + '|'.join(names) + r')\(')
l1keys = {(x[0], x[2]) for x in l1}
l2 = [(x, call.search(x[4]).group(1)) for x in fns if (x[0], x[2]) not in l1keys and call.search(x[4])]
print('level 1:', len(l1), 'read-only', sum(map(ro, l1)), 'mutating', len(l1) - sum(map(ro, l1)))
print('level 2:', len(l2), 'read-only', sum(ro(x) for x, _ in l2), 'mutating', sum(not ro(x) for x, _ in l2))
for tag, rows in (('1', [(x, '') for x in l1]), ('2', l2)):
    for x, via in sorted(rows):
        print(tag, 'RO' if ro(x) else 'MUT', f'{x[0]}:{x[1]}', x[2], via, sep='\t')
```
