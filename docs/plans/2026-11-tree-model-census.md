# Tree-model program: census data (at `84fee8fa`)

Data file of [2026-11-tree-model-shadow-root-is-a-root-umbrella.md](2026-11-tree-model-shadow-root-is-a-root-umbrella.md)
(§6 there states what each population is for and which slice consumes it). This file holds the commands, the counts
and the per-file tables. It is not a plan: it has no slice table and no slot table.

Draft 3 (after the R2 plan-review of draft 2; base moved from `c0578223` to `84fee8fa`): the VM-host lines and the
handler pairs are derived by a script (C-10, C-12) and carry reachability; C-13 groups the raw child reads by
function and marks the walks; C-2 separates a script-supplied node from a receiver (class "T2"); C-14 records the
one probe that was run. Draft 2 (after the R1 plan-review, PM 747 / PM 748): C-2 carries the **resolved** relation of every
line that draft 1 marked "?" (C-8, the section headed §C8, records each lookup); C-6 is replaced by a census over function bodies (derived
relations by property, script in C-11); C-9 (writers) and C-10 (the VM native and dom-api handler pairs) are new.

**Rules of this census (PM 730).**
- A population is defined by a **property** of the code ("reads the stored parent of an entity"), not by a name that
  one round happened to look at. The command is the instrument that approximates the property; where the instrument
  is narrower or wider than the property, the row says so.
- Every command is `git grep -n -P` (PCRE). `-E` is never used: `\b` and `\s` are silent zeros under `git grep -E`
  on this toolchain.
- Every command runs from the repository root against the tree `84fee8fa`. "Non-test" is the pathspec
  `-- crates ':!*tests*'`: it removes files and directories whose path contains `tests`. An inline
  `#[cfg(test)] mod tests` inside a non-test file is therefore counted as non-test (known over-count; the per-PR plan
  of the consuming slice subtracts by reading).
- Each population has a **positive control**: one line that must be in the output. A count without its control
  line present is not a measurement.
- Counts are lines, not call expressions. A line with two calls counts once.

## C-1. Populations

| Id | Property (what the code does) | Command (after `git grep -n -P`, before `84fee8fa -- crates ':!*tests*'` unless noted) | Count | Positive control |
|---|---|---|---|---|
| P-UP | reads the stored parent of an entity through the public accessor | `'\bget_parent\('` | 143 lines (1 is the definition, `navigation.rs:16`; 142 call lines) | `crates/api/elidex-api-observers/src/mutation/mod.rs:215` |
| P-UP-T | the same, in test paths | `'\bget_parent\('` over `-- crates`, minus P-UP | 174 − 143 = 31 lines | any line under a `tests` path; the two totals are both printed |
| P-REL | reads or writes a tree link field of `TreeRelation` directly (no accessor) | `'\.(parent\|first_child\|last_child\|next_sibling\|prev_sibling)\b(?!\s*\()'` over `-- crates/core/elidex-ecs/src ':!*tests*'` | 41 lines (1 false positive: `fragment_tree.rs:512`, a layout fragment's `parent`) | `crates/core/elidex-ecs/src/dom/mod.rs:756` (`rev_version`'s `rel.parent`) |
| P-CH | reads a raw child or sibling link through the public accessors | `'\b(get_first_child\|get_last_child\|get_next_sibling\|get_prev_sibling)\('` | 108 lines (34 more in test paths, verified 2026-11-18: the same command over `-- crates` prints 142) | `crates/dom/elidex-dom-api/src/range/mod.rs:558` (the Range boundary walk; 4 lines in that file) |
| P-CHX | reads a child or sibling through a compensating accessor (skips shadow roots) | one command per accessor, C-4 | C-4 | C-4 |
| P-MARK | tests or reads the `ShadowRoot` component of an entity | `'::<&?(\w+::)*(Ecs)?ShadowRoot>\|\bis_shadow_root\('` | 63 lines (2 false positives: `parse_state.rs:66`, `:335`, a method of the strict parser's own target enum) | `crates/core/elidex-ecs/src/dom/mod.rs:911` (`ChildrenIter`'s skip) |
| P-HOST | reads the backward link field `ShadowRoot.host` | `'\.host\b(?!\s*\()'` | 41 lines, of which 14 are in shadow-related files (C-5); the rest are URL / cookie / origin `host` fields | `crates/core/elidex-ecs/src/dom/mod.rs:761` |
| P-HOSTFN | reads the backward link through its accessor | `'\bshadow_host\('` | 2 lines (the definition `shadow.rs:521`, one caller `html_fragment.rs:81`) | the definition line |
| P-DER | is a **derived relation**: a read-only function (no `&mut` in its signature) whose body reads a parent link (level 1), or calls a level-1 member (level 2) | the script of C-11, `python3 <script> 84fee8fa` | level 1: 114 climbing functions, 60 read-only and 54 mutating; level 2: 113 functions that call a read-only level-1 name, 45 read-only and 68 mutating (C-6) | `is_ancestor` at level 1 and `is_ancestor_or_self` at level 2 through it; negative control: `get_shadow_root` is not in the output |
| P-CHW | is a function that reads a raw first or last child **and** a raw sibling: it can walk a whole raw child list, the only place where a shadow root appears as a child | the script of C-13, `python3 <script> 84fee8fa` | 104 reading lines (P-CH minus the 4 definition lines) in 54 functions of 25 files; 25 functions are walks, 23 of them outside elidex-ecs | `collect_focusable_entities` is printed as `walk`; negative control: `find_roots` (one raw read on a root) is printed as `hop` |
| P-VM | is a line under `crates/script/elidex-js/src/vm/host/` that calls a P-UP accessor, a child-side accessor (P-CH, P-CHX, or an elidex-ecs function that reads a raw link), or a read-only P-DER member | the script of C-12 | 58 lines in 25 files: UP 4, CH 10, DER 46 | `node_proto.rs:331` (UP), `html_input_proto.rs:876` (DER, `find_form_ancestor`), `parentnode.rs:159` (CH, passed as a path, not called) |
| P-NAV | is a registered dom-api handler whose `impl DomApiHandler` block calls such a member | the script of C-12 | 39 of 217 registered handlers; 21 reachable from production, 18 not (C-10) | `parentNode.get` (unreachable), `isConnected.get` (reachable) |
| P-WR | writes a parent link | `'\.parent\s*=[^=]'` over `-- crates/core/elidex-ecs/src ':!*tests*'` | 3 lines: one clear (`tree/mod.rs:86`), one link (`tree/mutation.rs:457`, in `link_node`), one doc comment (`tree/teardown.rs:54`) (C-9) | `crates/core/elidex-ecs/src/dom/tree/mutation.rs:457` |

**Why P-REL is confined to one crate.** The link fields of `TreeRelation` are `pub(crate)`
(`crates/core/elidex-ecs/src/components.rs:151-155`), so the compiler already confines direct field access to
elidex-ecs. The crate-wide form of the P-REL command (55 lines) only adds same-named fields of other types (a
`MutationRecord`'s `next_sibling`, the VM's `well_known` atoms, compiler scopes); none of them is a `TreeRelation`.

**Agreement with the independent verification** (`23aa745a`): P-UP 143 / 31, P-CH 108, P-MARK 63, P-HOST 41 and
P-HOSTFN 2 are the same numbers at `c0578223`. P-REL, P-CHX, P-DER and P-WR were not measured there. The R1
plan-review (Axis 5) re-ran every count of draft 1 at `c0578223` and they reproduced.

**Re-measured at `84fee8fa`** (2026-11-18, after the rebase of draft 3). Every command of C-1, C-4, C-9 and the
script of C-11 gives the same count as at `c0578223`, and the P-UP line list is the same list
(`git grep -n -P '\bget_parent\(' <rev> -- crates ':!*tests*'` for both revs, the rev prefix removed, `diff`: empty).
`git diff --name-only `c0578223`..84fee8fa -- crates` lists 10 files, none of which is in a table of this census, so
the `file:line` anchors are unchanged.

## C-2. P-UP by file, with the resolved relation

Draft 1 left 13 rows at "?". Every one is resolved here from the spec text (C-8 has the lookups); a row whose
relation this program does not define says so instead of borrowing a relation. The column is still a file-level
reading: the binding list is the compiler's in slice `ta` (umbrella §5), where `get_parent` is deleted and the plan
lists each line with its choice and its proof. A disagreement between that list and this table is a finding against
this table.

Legend (what the line takes in `ta`; names are in umbrella §1):
- **T** = `tree_parent`, with the reason base behaviour is equal: **1** the entity whose parent is read is a
  **receiver** that is never a shadow root; **g** an existing guard is folded into the choice; **h** a dom-api
  navigation handler that is unreachable from production and takes the VM native's relation (PM 748 (b); C-10).
- **T2** = `tree_parent`, and base behaviour is **not** equal: the entity is a node that a script supplied (a
  reference child, an old child), it can be a shadow root, and at base the check answers "yes, a child" for a host
  and its own shadow root (C-14). Taking `tree_parent` is the declared behaviour change of `ta` (umbrella §1.6).
  Draft 2 had these five lines under "T1"; R2 found them. Every remaining "T1" row owes, in `ta`'s plan, the
  statement "receiver" or "argument" per line (umbrella §5.2).
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
| `dom/elidex-dom-api/src/child_node/mod.rs` | 2 | **T2** (`:268` *ensure pre-insert validity* step 3, the reference child; `:451` the same step through *replace*, the old child) | [D13] |
| `dom/elidex-dom-api/src/child_node/mutations.rs` | 5 | T1 (the ChildNode mixin is on DocumentType, Element and CharacterData, not on ShadowRoot; the proof is owed line by line in `ta`) | — |
| `dom/elidex-dom-api/src/child_node/selectors.rs` | 1 | `closest`: T, g (the walk stops at the first non-element parent) | R-6 |
| `dom/elidex-dom-api/src/element/inheritance.rs` | 2 | `is_content_editable`: P:te. Draft 1 read this file as the `lang` inheritance; it is contenteditable. No `lang` or `dir` climb exists in the code | R-1 |
| `dom/elidex-dom-api/src/element/layout_query.rs` | 6 | F (`scrollIntoView`'s scrolling-box ancestors, `offsetParent`, the scroll offset of `getBoundingClientRect`) | R-2 |
| `dom/elidex-dom-api/src/element/markup.rs` | 1 | T1 (`outerHTML` setter's parent test) | — |
| `dom/elidex-dom-api/src/element/option_disabled.rs` | 2 | P:tf (optgroup ancestor) | [H9] |
| `dom/elidex-dom-api/src/element/select.rs` | 3 | T1 | — |
| `dom/elidex-dom-api/src/element/table_mutation/mod.rs` | 7 | T1 | — |
| `dom/elidex-dom-api/src/element/tree.rs` | 3 | `:158` **T2** (the reference child of `insertBefore`, re-tested for the empty-fragment case); `:504`, `:519` T1 (the receiver of `insertAdjacent*`) | [D13] |
| `dom/elidex-dom-api/src/focus/predicate.rs` | 2 | `:126` T1 (a summary's parent details); `:313` `is_in_hidden_subtree`: F | R-7 |
| `dom/elidex-dom-api/src/focus/sot.rs` | 1 | `is_in_document`: S | R-8 |
| `dom/elidex-dom-api/src/node_methods/mod.rs` | 1 | `:67` `find_root`: removed, its two callers take the one shadow-including root | [D4] |
| `dom/elidex-dom-api/src/range/boundary.rs` | 5 | P:tr | [D8] |
| `dom/elidex-dom-api/src/range/mod.rs` | 4 | P:tr (`:126`, `:137` root and common ancestor; `:552`, `:582` walk) | [D8] |
| `dom/elidex-dom-api/src/range/mutation.rs` | 2 | P:tr | [D8] |
| `dom/elidex-dom-api/src/selection.rs` | 1 | P:tr (`containsNode`, Selection API §3 Selection interface) | [D8], [X1] |
| `dom/elidex-dom-api/src/traversal.rs` | 10 | P:tr (bounded by the traversal root in most paths; proof per line in `tr`) | [D10] |
| `dom/elidex-dom-api/src/tree_nav.rs` | 2 | T, h (`GetParentNode`, `GetParentElement`; the VM natives guard the receiver at `node_proto.rs:328`, `:345`) | [D7] |
| `dom/elidex-dom-compat/src/presentational.rs` | 2 | T, g (the cell-to-table walk breaks at the first parent with no `TagType`, which a shadow root is) | R-10 |
| `dom/elidex-form-core/src/fieldset.rs` | 4 | P:tf (*disabled fieldset*: "descendant of") | [H3], [H8] |
| `dom/elidex-form/src/select/mod.rs` | 4 | P:tf (*nearest ancestor select*) | [H4] |
| `dom/elidex-form/src/submit.rs` | 3 | P:tf (*reset the form owner* step 4; "nearest ancestor dialog") | [H2], [H5] |
| `dom/elidex-html-parser-strict/src/tree_builder/mod.rs` | 1 | T, g (`:308`, `set_fragment_form_pointer`: HTML §13.4 Parsing HTML fragments step 19, "going straight up the ancestor chain") | [H14] |
| `layout/*` (8 files, verified 2026-11-18 with the P-UP command over `-- crates/layout`) | 10 | six "is this the root" tests (`block/mod.rs:627`, `elidex-layout-flex/src/lib.rs:510`, `elidex-layout-grid/src/lib.rs:625`, `elidex-layout-multicol/src/lib.rs:341`, `elidex-layout-table/src/lib.rs:791`, `elidex-layout/src/hit_test.rs:187`): F; `shift.rs:188` (the IFC element of a run): F; table row / section parent (`fragment.rs:74`, `:81`, `lib.rs:130`): T1 | R-3 |
| `script/elidex-js/src/vm/host/html_legend_proto.rs` | 1 | T1 | — |
| `script/elidex-js/src/vm/host/node_proto.rs` | 2 | T, g (`:331`, `:348`; the receiver guards at `:328`, `:345` fold into `tree_parent`) | [D7] |
| `script/elidex-js/src/vm/host/range_proto.rs` | 1 | `:529` `require_attached_node`: P:tr (a shadow-root argument has a null parent by spec; base finds the host). The step stays in the host until R-NAT (umbrella §5 `tr`) | R-11 |
| `script/elidex-script-session/src/event_dispatch/mod.rs` | 1 | E (`:217`; the break at `:214` stays, C-5 role G) | [D11] |
| `script/elidex-script-session/src/mutation/child_list.rs` | 4 | `:284`, `:439` **T2** (the reference child and the old child, re-checked for the deferred flush); `:26`, `:63` T1 (the old parent of the node being moved; a shadow root is rejected as that node before, C-5 role I: proof owed in `ta`) | [D13] |
| `script/elidex-script-session/src/mutation/html_fragment.rs` | 5 | T1 (`:80-81` already resolves a shadow-root receiver through `shadow_host`) | — |
| `script/elidex-wasm-runtime/src/runtime.rs` | 1 | T1 (an inline test assertion) | — |
| `shell/elidex-shell/src/app/events.rs` | 1 | E (the link whose activation behaviour a click runs) | R-12 |
| `shell/elidex-shell/src/app/hover.rs`, `content/scroll.rs` | 2 | F (`:hover` chain; scroll target) | R-3 |
| `shell/elidex-shell/src/content/form_input.rs` | 2 | E (`find_label_ancestor`: the label whose activation behaviour a click runs). Draft 1 read it as a form-ancestor climb | R-12 |
| `shell/elidex-shell/src/content/iframe/lifecycle.rs` | 1 | T1 (`:604`, an inline test: the iframe's parent in the document tree) | — |

Total: 143 lines in 60 files (`git grep -n -P '\bget_parent\(' 84fee8fa -- crates ':!*tests*' | cut -d: -f2 | sort -u | wc -l`
gives 60). **Predicted backlog** (lines, from this table; `ta`'s compiler list is binding): P:tm 2, P:tr 23, P:tf 13,
P:te 3, in total 41; F 21, E 4, L 7; T2 5. No row is left unclassified, and no pending line is without a slice or
an owner. The elidex-ecs half of the T2 question is not a P-UP line: `is_child_of` reads the field (C-6, C-9).

## C-3. P-CH by file

`git grep -n -P '\b(get_first_child|get_last_child|get_next_sibling|get_prev_sibling)\(' 84fee8fa -- crates ':!*tests*' | cut -d: -f2 | uniq -c`

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

**The walks (P-CHW; script in C-13).** Stopping a raw reader from seeing a shadow root fixes the readers whose
relation is the tree's, and it breaks a reader that is correct only because it crosses. The second kind can only
be a function that walks a raw child list, so the population is measured by that property: of the 54 functions that
read a raw link, 25 read a first or last child **and** a sibling. Two are elidex-ecs element helpers
(`first_element_child`, `last_element_child`: filtered by `is_element`). The other 23, with the relation each
needs (a **reading**, not run; `tc`'s plan re-reads all 23 and probes the ones it leaves on tree children):

| Relation the walk needs | Functions (file under `crates/`) | In `tc` |
|---|---|---|
| **The explicit descent** (the walk must keep reaching a shadow tree) | `collect_focusable_entities` (`shell/elidex-shell/src/content/event_handlers.rs:456`, Tab order), `collect_iframe_entities` (`shell/elidex-shell/src/content/iframe/lifecycle.rs:437`, iframe discovery), `save_descendant_styles` (`layout/elidex-layout-block/src/helpers.rs:282`, the style save around a layout probe) (3) | `get_shadow_root`, then that root's tree children (umbrella §1.3); cells in umbrella §2 CHILD × SHELL, CHILD × RENDER |
| **Tree children; entering a shadow tree is the defect** | Range and traversal: `child_index_in_ancestor`, `next_in_preorder_global` (`range/mod.rs`), `next_in_preorder`, `traverse_children`, `traverse_siblings`, `traverse_children_filtered`, `traverse_siblings_filtered`, `step_with_filter_previous_node` (`traversal.rs`); `normalize_entity` (`node_methods/core.rs:89`); form: `disable_descendants` (`elidex-form/src/fieldset.rs:35`), `find_first_descendant_control` (`label.rs:129`), `walk_options`, `init_select_options`, `get_option_text` (`select/mod.rs`), `walk_form_descendants` (`submit.rs:437`) (15) | the one set; observable (umbrella §3.1 `tc`) |
| **Tree children; no shadow root can be met or it is filtered** (predicted) | `count_li_children` (`elidex-style/src/counter.rs:276`), `collect_text_recursive` (`elidex-dom-compat/src/presentational.rs:468`), `first_legend_child` (`elidex-form-core/src/fieldset.rs:12`), `collect_col_info` (`elidex-layout-table/src/helpers.rs:88`), `apply_insert_adjacent_html` (`mutation/html_fragment.rs:193`) (5) | the one set; no change predicted; each probed |

3 + 15 + 5 = 23. Limits of the instrument: a function that reads one raw link and is driven in a loop by its caller
is printed as `hop` (29 functions; the nine `tree_nav.rs` handlers and the three `find_roots` are of this kind and
do not descend); a walk built on a compensating accessor never meets a shadow root and is not in this population,
because it cannot rely on crossing.

## C-4. P-CHX: compensating child-side accessors

Command per row: `git grep -n -P '\b<pattern>' 84fee8fa -- crates ':!*tests*' | wc -l`.

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
| **Ls** the selector boundary | `css/elidex-css/src/selector/matching.rs:34`, `:49` (2): the combinator climb stops at the shadow root. Whether the featureless host belongs there is `#11-shadow-tree-style-isolation`'s (PM 794); this program does not change the lines |
| **C** crossing hop: reads `.host` to continue upward | `core/elidex-ecs/src/dom/mod.rs:761`; `dom/tree/mutation.rs:420`; `dom/tree/walkers.rs:385`, `:438`; `dom/elidex-a11y/src/tree.rs:237`; `script/elidex-script-session/src/event_dispatch/mod.rs:595` (*retarget* step 2, the `.host` read is at `:605`) (6). The three elidex-ecs ones that read the tree parent first are the branches the independent verification found unreachable for an attached shadow root |
| **E** event suppression: "node or parent is a shadow root, fire nothing" | `core/elidex-ecs/src/dom/tree/mutation.rs:528`, `:582` (2). The **parent** half is relied on by `LiveRangeRegistry` (`dom/elidex-dom-api/src/range/live.rs:38-54`) and by Layout's T3; the **node** half becomes unreachable in `ts` (umbrella §2 MEV × TREE) |
| **L** lifetime: the strip | `core/elidex-ecs/src/dom/tree/teardown.rs:69`, `:81` (2); owned by the ecs-entity-lifetime program |
| **I** identity: "is this node a ShadowRoot" for dispatch, brand or kind | `core/elidex-ecs/src/dom/tree/navigation.rs:89`, `:90` (the predicate); `dom/classify.rs:119` (`is_document_fragment` excludes a shadow root); `dom/elidex-dom-api/src/child_node/mod.rs:322`, `document.rs:345`, `node_methods/clone.rs:115`, `range/mutation.rs:502`; `script/elidex-js/src/vm/host_data/engine_feature/entity_kind.rs:42`, `:114`, `:173`; `script/elidex-script-session/src/mutation/child_list.rs:190`, `mutation/html_fragment.rs:80`; `event_dispatch/mod.rs:173`, `:183`, `:635` (15). Four of these reject inserting a shadow root as a node (`child_node/mod.rs:322`, `document.rs:345`, `range/mutation.rs:502`, `child_list.rs:190`): they stay, the spec forbids the insertion under either model |
| **D** data: reads another field of the component (mode, slot assignment, delegates focus, clonable, serializable) | `core/elidex-ecs/src/dom/shadow.rs:266`, `:271`, `:406`, `:419`, `:523` (`:266` and `:523` read `host` itself: the link, P-HOST); `dom/elidex-dom-api/src/element/markup.rs:307`, `focus/delegate.rs:101`, `node_methods/clone.rs:453`; `script/elidex-js/src/vm/host/element_shadow.rs:122`, `shadow_root_proto.rs:188`; `script/elidex-script-session/src/mutation/html_fragment.rs:464`, `:504`, `:630` (13) |

11 + 2 + 7 + 1 + 2 + 6 + 2 + 2 + 15 + 13 = 61 (K, R, B, G, Ls, C, E, L, I, D). The role of each line is from a
single read at line level; `tc` (roles K, R) and `ta` (roles B, C) re-derive their rows from the compiler's list and
treat any disagreement with this table as a finding against the table. `ta` adds no role-R line: the dom-api
sibling handlers are not given the natives' receiver guard in `ta`; they wait for the one set in `tc` (umbrella §2
CHILD × NAV).

P-HOST in shadow-related files (14 lines): `core/elidex-ecs/src/dom/shadow.rs` 5, `dom/tree/walkers.rs` 2,
`dom/tree/teardown.rs` 1, `dom/tree/mutation.rs` 1, `dom/mod.rs` 1, `dom/elidex-a11y/src/tree.rs` 2,
`script/elidex-script-session/src/event_dispatch/mod.rs` 1, `script/elidex-js/src/vm/host/shadow_root_proto.rs` 1
(`git grep -n -P '\.host\b(?!\s*\()' 84fee8fa -- crates ':!*tests*' | cut -d: -f2 | sort | uniq -c`). Two of the 14
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
(`git grep -n -P '\bis_ancestor_or_self\(' 84fee8fa -- crates ':!*tests*'` prints 18 lines: the definition, one
comment, 16 calls) and none of them calls `get_parent`.

**Instrument.** The script of C-11 (`python3 <script> 84fee8fa`, run 2026-11-15). It splits every non-test `.rs`
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
| `find_tree_root_composed` (`dom/tree/walkers.rs:432`) | shadow-including root | RN (the spec's name; "composed" is not the relation). 2 callers (verified 2026-11-18): `consumer.rs:408`, `node_proto.rs:455` |
| `is_host_including_ancestor_or_self` (`walkers.rs:373`) | host-including inclusive ancestor, shadow hosts only | RN. 3 callers (verified 2026-11-18) re-choose: `child_node/mod.rs:260`, `:445` take H; `focus/delegate.rs:88` takes the shadow-including test (its own comment names that relation), so the template host added after PR (io) does not widen it |
| `is_light_tree_ancestor_or_self` (`walkers.rs:348`) | inclusive ancestor | RN (the tree test; "light tree" is not the relation: it is also the test inside a shadow tree). 3 callers (verified 2026-11-18): `equality.rs:193`, `:198`, `node_proto.rs:647` |
| `index_in_parent` (`dom/tree/navigation.rs:61`) | index among tree children | RN in `tc` (it joins the one child-side set); in `ta` only its parent read changes (raw) |
| `is_contenteditable` (`core/elidex-ecs/src/dom/mod.rs:364`), `is_content_editable` (`dom/elidex-dom-api/src/element/inheritance.rs:42`) | contenteditable inheritance, crossing | PF (te) |
| `is_option_disabled` (`element/option_disabled.rs:30`), `is_fieldset_disabled`, `is_in_first_legend` (`dom/elidex-form-core/src/fieldset.rs:46`, `:28`), `find_option_select`, `find_options_container` (`dom/elidex-form/src/select/mod.rs:200`, `:245`), `find_dialog_ancestor`, `find_form_ancestor` (`dom/elidex-form/src/submit.rs:236`, `:263`) | form-association ancestors, crossing | PF (tf), 7 functions |
| `intersects_node` (`dom/elidex-dom-api/src/range/boundary.rs:235`), `child_index_in_ancestor`, `common_ancestor_container`, `next_in_preorder_global` (`range/mod.rs:549`, `:121`, `:573`), `node_fully_contained` (`selection.rs:651`), `next_in_preorder`, `next_in_preorder_skip_subtree`, `prev_in_preorder` (`traversal.rs:87`, `:110`, `:124`) | Range and traversal relations, crossing | PF (tr), 8 functions |
| `find_nearest_layout_box`, `multicol_container_writing_mode` (elidex-render), `is_sectioning_content_descendant` (elidex-a11y), `find_offset_parent`, `accumulated_scroll_offset` (`layout_query.rs`), `is_in_hidden_subtree` (`focus/predicate.rs:303`), `collect_hover_chain`, `find_scroll_target` (elidex-shell) | flat-tree ancestors, approximated by the stored parent | ND (F), 8 functions |
| `build_propagation_path` (`event_dispatch/mod.rs:203`), `find_link_ancestor_with_target` (`app/events.rs:204`), `find_label_ancestor` (`content/form_input.rs:308`) | event *get the parent* chain | ND (E), 3 functions |
| `match_components`, `match_pseudo_class` (`selector/matching.rs:12`, `:185`), `is_root_element` (`selector/traverse.rs:24`) | the selector tree's parent | ND (L), 3 functions |
| `find_tree_root` (`walkers.rs:408`), `is_connected` (`walkers.rs:232`), `root_entities` (`navigation.rs:372`) | tree root; connected; parentless non-fragment entities | K. Each name is the relation, and it is the same relation at every head (the marker stop, then the storage). 24, 26 and 5 lines call them (`git grep -n -P '\b<name>\(' 84fee8fa -- crates ':!*tests*' \| wc -l`, definitions included) |
| `is_child_of` (`dom/tree/mod.rs:57`) | one stored-link equality, `pub(super)`; the check of `remove_child`, `insert_before`, `replace_child` (`tree/mutation.rs:72`, `:116`, `:165`) on a child a caller supplied | K by name. Draft 2 kept it as a raw read; R2 found that the child can be script-supplied (`removeChild` has no other check). It stays a raw read (storage maintenance, P-REL) until `ts`; the spec's child-of steps, pre-remove step 1 included, are asked in the mutation layer from `ta` (umbrella §1.6, Q-19 closed by PM 800; C-9) |
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
   and D) and no slice changes them, with one exception that C-5 already counts through P-HOST: the `host` read
   at `shadow_root_proto.rs:203`, which `ta` routes through `shadow_host`.
7. The relation of each P-CHW walk (C-3): read, not run, except where C-14 says so.
8. P-NAV sees a member called inside the handler's `impl` block. A handler that reaches a member only through a
   helper outside the block is not in the 39 (the compiler reports the helper in `ta` or `tc`).
9. The IDL member that joins a VM native to a handler in C-10 is read from the native's install line; the
   handler's side of the join is the registry literal, which the script reads.

## §C8. Spec coverage map for the rows draft 1 left at "?" (C-8)

Each row was looked up with `.claude/tools/webref` on 2026-11-15 (the `body` command is in the row); the quoted
words are the spec's. "Base" is the reading of the code at `84fee8fa`. The row id (R-n) is what C-2 and the umbrella
cite.

| Spec section | Step | Branch (the spec's words) | Touch (site → relation; base) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §6.8.1 Making document regions editable: The contenteditable content attribute | the inherit state | "The inherit state indicates that the element is editable (or not) based on the parent element's state." (`body html contenteditable`) | **R-1**: `is_contenteditable` (`core/elidex-ecs/src/dom/mod.rs:364`), `is_content_editable` (`element/inheritance.rs:42`) → tree parent (a child of a shadow root has no parent element); base crosses to the host: P:te | ✓ | yes |
| CSSOM View (cssom-view-1) §7 Extensions to the HTMLElement Interface | `offsetParent` step 2; §6.1 *scroll a target into view* | "Let ancestor be the parent of the element in the flat tree"; "For each ancestor element or viewport that establishes a scrolling box" (`body cssom-view-1 extensions-to-the-htmlelement-interface`, `body cssom-view-1 extension-to-the-element-interface`) | **R-2**: `layout_query.rs` (6 lines) → flat tree (not defined here); base reads the stored parent: F | ✓ | yes |
| CSS Shadow Module Level 1 (css-shadow-1) §4.1 Flattening the DOM into an Element Tree | *flat tree*; Selectors L4 §9.1 `:hover` | "the rest of CSS needs a single unified tree structure to work with"; "one of its descendants in the flat tree" (`body css-shadow-1 flattening`, `body selectors-4 the-hover-pseudo`) | **R-3**: render `paint/mod.rs:795`, `walk.rs:232`, `:499`; the layout root tests and `shift.rs:188`; a11y `tree.rs`; shell hover and scroll → flat tree (not defined here); "is this the root" is "has no flat-tree parent"; base reads the stored parent: F | ✓ | yes |
| CSS Shadow Module Level 1 (css-shadow-1) §3.2.2 Selecting Shadow Hosts from within a Shadow Tree | the host in its shadow tree; §3.2.1 the match list | "For the purpose of Selectors, a shadow host also appears in its shadow tree, with the contents of the shadow tree treated as its children. (In other words, the shadow host is treated as replacing the shadow root node.)"; "the selector match list is initially the shadow host, followed by all children of the shadow tree's shadow root and their descendants" (`body css-shadow-1 host-element-in-tree`, `body css-shadow-1 selectors-data-model`) | **R-4**: `selector/matching.rs` (6), `selector/traverse.rs` (1) → the selector tree's parent (not defined here; owner `#11-shadow-tree-style-isolation`, PM 794); base is the tree parent with a stop at the shadow root: L | ✓ | yes |
| WHATWG DOM §4.9 Interface Element | `closest` step 3 | "Let elements be this's inclusive ancestors that are elements, in reverse tree order." (`body dom dom-element-closest`) | **R-6**: `child_node/selectors.rs` → tree parent; base stops at the first non-element parent: T, g | ✓ | yes |
| WHATWG HTML §15.1 Introduction | *being rendered* | "if it has any associated CSS layout boxes"; "The presence of the hidden attribute normally means the element is not being rendered" (`body html being-rendered`) | **R-7**: `focus/predicate.rs:313` `is_in_hidden_subtree` → a rendering fact, so the flat tree (not defined here); base reads the stored parent, and its doc says so on purpose: F | ✓ | yes |
| WHATWG DOM §4.2.2 Shadow tree | *connected* | "A node is connected if its shadow-including root is a document" (`body dom concept-shadow-tree`) | **R-8**: `focus/sot.rs:69` `is_in_document` (the same question, for one given document) → shadow-including; base reads the stored parent: S, equal | ✓ | yes |
| WHATWG DOM §1.1 Trees | *tree order*; §4.4 `compareDocumentPosition` step 6; §5.2 *position* | "If node1 or node2 is null, or node1's root is not node2's root, then return the result of adding DOCUMENT_POSITION_DISCONNECTED, …"; "Assert: nodeA and nodeB have the same root." (`body dom interface-node`, `body dom boundary-points`) | **R-9**: `tree_order_cmp` and its 5 call lines (`git grep -n -P '\btree_order_cmp\(' 84fee8fa -- crates ':!*tests*'` prints 6 lines with the definition). Tree order for `compare_document_position` (`equality.rs:203`: it returns before the call when the tree roots differ) and for `find_radio_group_scoped` (`radio.rs:143`: the group is filtered to one tree root first): equal at base, taken in `ta`. Range's `compare_points` (`range/mod.rs:538`): pending, tr. Paint and hit-test layer order (`builder/walk.rs:620`, `hit_test.rs:235`): the flat tree's order (not defined here). Base: one crossing chain for all five | ✓ | yes |
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

`git grep -n -P '\.parent\s*=[^=]' 84fee8fa -- crates/core/elidex-ecs/src ':!*tests*'` prints 3 lines. One site
writes `Some(parent)`: `link_node` (`crates/core/elidex-ecs/src/dom/tree/mutation.rs:449`, the write at `:457`).
`git grep -n -P '\blink_node\(' 84fee8fa -- crates ':!*tests*'` prints its definition and three callers:
`append_child` (`mutation.rs:53`), `insert_before` (`:132`), `replace_child` (`:214`). No function outside the
`dom::tree` module can write a link field (`pub(crate)` fields, and `link_node` / `update_rel` are `pub(super)`).

At base nothing in elidex-ecs refuses to link a shadow root as the **node**: the four insertion guards of C-5
role I are in elidex-dom-api and elidex-script-session, and `attach_shadow_with_init` itself calls
`append_child(host, sr)`. Slice `ts` adds the refusal at the three callers' validity check, beside the cycle guard
(umbrella §2 TREE × WR, the node half).

Nothing refuses a shadow root as the **child** either (the reference child, the old child, the removed child):
`is_child_of` compares the stored parent, which is the host. `git grep -n -P '\bis_child_of\(' 84fee8fa -- crates`
prints the definition and its three callers (`tree/mutation.rs:72`, `:116`, `:165`).

**Who passes a child to the three mutators.**
`git grep -n -P '\.(remove_child|insert_before|replace_child)\(' 84fee8fa -- crates ':!*tests*' | wc -l` → 53
(calls and inline tests; the definitions are not matched). Read once for "can the child argument be a shadow
root": the script paths (`mutation/child_list.rs:318`, `:340`, `:369`, `:458`, `:465`, `:509`, `:565`) pass what
the script supplied; and three engine callers pass a **raw** first child or next sibling, which for a host can be
its shadow root: `css/elidex-style/src/pseudo.rs:74` (the raw read is `:72`),
`script/elidex-script-session/src/mutation/html_fragment.rs:222` (raw read `:216`), `:251` (raw read `:243`). All
three discard the result. The other callers pass an element found by a filtered search, a node they just created,
or a text child (a reading, not run). R2's fix decision assumed only scripts supply such a child; this list is why
umbrella §1.6 leaves elidex-ecs unchanged in `ta` (Q-19 (b′), PM 800): the three callers keep base behaviour and
pick tree children at `tc`.

## C-10. The VM natives and the dom-api handlers that answer the same question

Draft 2 built this table by hand and R2 found 13 VM-host lines and three members missing. It is now the output of
the script in C-12 (P-VM: 58 lines; P-NAV: 39 handlers), grouped by reading. **Reach** is the handler census of
`#11-dom-api-handlers-unreachable-from-production` (its scripts and outputs are in the PM's hand-off directory
`handoff/l3-dom-handler-census/`, `run.sh`; re-run on the `84fee8fa` tree: 217 registered, 161 reached, 56
unreachable, the same list as on 2026-11-15): **R** some production entry reaches the handler, **U** none does. It
is not "the literal occurs somewhere" (PM 583).

**The rule (PM 748 (b), narrowed by R2).** A VM native chooses the elidex-ecs relation directly (marshalling). An
**unreachable** handler for the same member takes the native's relation, mechanically, with no new cell; a test
that pins its old answer is listed in the slice's plan. A **reachable** handler keeps the relation it computes at
base; if that differs from the native's, the difference is the handler slot's or R-NAT's, not this program's.

Parent side and derived relations (`ta`):

| Member | Reach | dom-api handler (`crates/dom/elidex-dom-api/src/`) | VM native (`crates/script/elidex-js/src/vm/host/`) | In `ta` |
|---|---|---|---|---|
| `parentNode`, `parentElement` | U | `tree_nav.rs:66`, `:89` (`get_parent`, no guard) | `node_proto.rs:331`, `:348` (receiver guard, then `get_parent`) | both: tree parent |
| `contains` | U | `node_methods/core.rs:25` (`is_ancestor_or_self`, crossing) | `node_proto.rs:647` (`is_light_tree_ancestor_or_self`) | both: the tree inclusive-ancestor test |
| `isConnected` | R (`html_dialog_proto.rs:364`) | `node_methods/core.rs:185` (`find_root`) | `node_proto.rs:455` (`find_tree_root_composed`, compared with the bound document) | each keeps its own: the handler *connected*; the native's comparison is `#11-vm-isconnected-bound-document`'s (a small PR after `ta`) |
| `getRootNode` | R (`node_methods_extras.rs:98`) | `node_methods/core.rs:210` (`find_root` / `find_root_non_composed`) | the native calls the handler | shadow-including root or tree root, by the option, as at base |
| `compareDocumentPosition` | R (`node_methods_extras.rs:136`) | `node_methods/core.rs:58` (`compare_document_position`) | the native calls the handler | tree order within one root (C-8 R-9), as at base |
| `closest` | R (`element_proto.rs:385`) | `child_node/selectors.rs:51` | the native calls the handler | T, g |
| `insertBefore`, `replaceChild`, `replaceWith` | R | `element/tree.rs:97`, `:235`, `child_node/mutations.rs:196` (`ensure_pre_insertion_validity`, `ensure_replace_validity`, `get_parent`) | the natives call the handlers | H for the cycle test; **T2** for the child-of question (umbrella §1.6): the one change |
| `appendChild`, `append`, `prepend`, `before`, `after`, `remove`, `replaceChildren`, `insertAdjacentElement`, `insertAdjacentText` | R | `element/tree.rs:43`, `:352`, `:412`, `child_node/mutations.rs:77`, `:128`, `:166`, `:255`, `:291`, `:322` | the natives call the handlers | H; T1 for the receiver's parent |
| `options.add`, `options.remove`, `options.length` | R | `element/select.rs:130`, `:222`, `:277` | the natives call the handlers | `add`'s two ancestor tests: pending, `Flip::Tf` ([H12]); the rest T1 |
| `insertAdjacentHTML` | U | `element/markup.rs:70` | — | T1 |
| `getBoundingClientRect`, `getClientRects`, `offsetParent`, `scrollIntoView` | U | `element/layout_query.rs:14`, `:203`, `:96`, `:259` | — | F: the flat-tree pending names (umbrella §1.5) |

Child side (`tc`; not touched in `ta`, umbrella §2 CHILD × NAV):

| Member | Reach | dom-api handler | VM native | In `tc` |
|---|---|---|---|---|
| `firstChild`, `lastChild` | U | `tree_nav.rs:113`, `:136` (raw) | `node_proto.rs:363`, `:372` (`children_iter`, `children_iter_rev`) | both: the one set |
| `nextSibling`, `previousSibling` | U | `tree_nav.rs:159`, `:182` (raw) | `node_proto.rs:387`, `:400` (receiver guard at `:384`, `:397`, then the exposed sibling) | both: the one set, which holds the receiver half |
| `hasChildNodes` | U | `tree_nav.rs:472` (raw: true for a host whose only child is its shadow root) | `node_proto.rs:616` (`children_iter`: false) | both: the one set (the handler's answer changes; it is unreachable) |
| `firstElementChild`, `lastElementChild`, `nextElementSibling`, `previousElementSibling` | U | `tree_nav.rs:205`, `:229`, `:253`, `:278` (raw, filtered by `is_element`) | `parentnode.rs:159`, `:167`, `element_proto.rs:341`, `:349` (the elidex-ecs element helpers) | both: the one set |
| `childElementCount` | U | `tree_nav.rs:444` (`children_iter`) | `parentnode.rs:204` (`children_iter`) | both: the one set |
| `documentElement`, `title` setter; `before`, `prepend`, `options.length` | R | `char_data/document_props.rs:178`, `:307`; `child_node/mutations.rs:77`, `:255`; `element/select.rs:277` (`children_iter`) | the natives call the handlers | the one set; no change (the iterator already skips) |

The other VM-host lines (P-VM, by the slice that edits them):

| Slice | Lines (`crates/script/elidex-js/src/vm/host/`) |
|---|---|
| `ta` (a renamed or removed callee, or a P-UP line) | `html_legend_proto.rs:87` (T1); `range_proto.rs:529` (pending, `Flip::Tr`); and the lines of the first table |
| `tr` | `range_proto.rs:432` (`common_ancestor_container`), `:783` (`intersects_node`): `_pending_flip` names from `ta`; `:749` (`is_point_in_range`), `:768` (`compare_point`): level-2 callers, listed in `tr`'s plan |
| `tf` | the nine `.form` getters that call `find_form_ancestor`: `html_button_proto.rs:398`, `html_fieldset_proto.rs:193`, `html_input_proto.rs:876`, `html_label_proto.rs:177`, `html_legend_proto.rs:96`, `html_option_proto.rs:370`, `html_output_proto.rs:186`, `html_select_proto.rs:399`, `html_textarea_proto.rs:641`; `html_option_proto.rs:367` (`find_option_select`); level-2 callers: `html_option_proto.rs:348`, `html_options_collection.rs:158`, `html_select_proto.rs:501`, `:562`, `html_form_proto.rs:636`, `validity_state.rs:175`, `:405`, `:446`, `:468`. These getters pick the ancestor relation in the host and not through `resolve_form_owner`: that is `#11-form-owner-ecs-component`'s (umbrella §7.1) |
| `te` | `html_element_proto.rs:663` (`is_content_editable`) |
| none (a K or ND member; the line compiles unchanged or with the renamed callee) | `document_accessors.rs:479`, `:527`, `html_element_proto.rs:290`, `:302`, `:303`, `:887`, `event_target_dispatch.rs:119`, `:120`, `html_slot_proto.rs:327`, `:346`, `html_table_cell_proto.rs:112`, `html_table_row_proto.rs:109`, `:127`, `html_details_proto.rs:164`, `html_dialog_proto.rs:289`, `html_input_proto.rs:941`, `range_proto.rs:722`, `static_range_proto.rs:382` |

Lines: 4 (UP) + 10 (CH) + 44 (DER; 46 calls, two lines hold two) = 58.

## C-11. The P-DER script

Run from the repository root: save the block as a file and run `python3 <file> 84fee8fa`. It prints the two count
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

## C-12. The P-VM / P-NAV script

Run from the repository root: `python3 <file> 84fee8fa <saved output of C-11 at the same rev> <unreachable.txt>`
(the third argument is the handler census's list of unreachable registered names, one per line, C-10). It prints
the two count lines, one `V` row per VM-host call (kind, file, line, enclosing function, member) and one `H` row
per handler (registered name, R / U, `file:line` of the `impl`, the members it calls). Limits: calls are matched
by bare name, and `to_string` and `matches` are dropped from the P-DER names because std and unrelated types share
them; a member passed as a path (`EcsDom::first_element_child`) is counted; the enclosing function is the last
`fn` line above.

```python
import re, subprocess, sys, collections
rev, pder, unreach = sys.argv[1:4]          # C-11's saved output at rev; the handler census's unreachable.txt
git = lambda *a: subprocess.run(('git',) + a, capture_output=True, text=True).stdout
show = lambda f: [re.sub(r'//.*$', '', l) for l in git('show', f'{rev}:{f}').split('\n')]
files = lambda *p: [f for f in git('ls-tree', '-r', '--name-only', rev, '--', *p).split('\n')
                    if f.endswith('.rs') and 'tests' not in f]
FN = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?fn\s+(\w+)')
UP = {'get_parent'}
RAW = {'get_first_child', 'get_last_child', 'get_next_sibling', 'get_prev_sibling'}
CH = RAW | {'children_iter', 'children_iter_rev', 'children', 'child_list_uncapped', 'index_in_parent',
            'next_exposed_sibling', 'prev_exposed_sibling'}
raw = re.compile(r'\b(' + '|'.join(RAW) + r')\(')
cur = None                                  # child-side level 1: an elidex-ecs function that reads a raw link
for f in files('crates/core/elidex-ecs/src'):
    for l in show(f):
        m = FN.match(l)
        cur = m.group(1) if m else cur
        if not m and cur and raw.search(l):
            CH.add(cur)
DER = {l.split('\t')[3] for l in open(pder) if l[:1] in '12' and l.split('\t')[1] == 'RO'} - CH
DER -= {'to_string', 'matches'}             # names shared with std / unrelated methods: not decidable by name
pat = re.compile(r'(?:\b|::)(' + '|'.join(sorted(UP | CH | DER)) + r')(?:\(|\s*[,)]|\s*$)')
kind = lambda n: 'UP' if n in UP else 'CH' if n in CH else 'DER'
def hits(lines, lo, hi):
    out = []
    for i in range(lo, hi):
        l = lines[i]
        if FN.match(l):
            continue
        for m in pat.finditer(l):
            if m.group(0).endswith('(') or m.group(0).startswith('::'):
                out.append((i + 1, m.group(1)))
    return out
HOST = 'crates/script/elidex-js/src/vm/host/'
vm = []
for f in files(HOST):
    lines, fn = show(f), '-'
    for i, l in enumerate(lines):
        m = FN.match(l)
        fn = m.group(1) if m else fn
        vm += [(kind(n), f[len(HOST):], ln, fn, n) for ln, n in hits(lines, i, i + 1)]
print('VM host:', len({r[1:3] for r in vm}), 'lines', len({r[1] for r in vm}), 'files',
      dict(sorted(collections.Counter(r[0] for r in vm).items())))
for r in sorted(vm):
    print('V', *r, sep='\t')
reg = dict((t, n) for n, t in re.findall(r'reg\(\s*"([^"]+)",\s*Box::new\((?:\w+::)*(\w+)',
           git('show', f'{rev}:crates/dom/elidex-dom-api/src/registry.rs')))
dead = set(open(unreach).read().split())
hs = []
for f in files('crates'):
    lines = show(f)
    for i, l in enumerate(lines):
        m = re.match(r'\s*impl\s+DomApiHandler\s+for\s+(\w+)', l)
        if not m or m.group(1) not in reg:
            continue
        depth, j = 0, i
        while True:
            depth += lines[j].count('{') - lines[j].count('}')
            j += 1
            if depth <= 0 and '{' in ''.join(lines[i:j]):
                break
        h = hits(lines, i, j)
        if h:
            name = reg[m.group(1)]
            hs.append((name, 'U' if name in dead else 'R', f'{f[len("crates/"):]}:{i + 1}',
                       ','.join(sorted({kind(n) + ':' + n for _, n in h}))))
print('dom-api handlers:', len(hs), 'of', len(reg), 'registered;',
      dict(sorted(collections.Counter(r[1] for r in hs).items())))
for r in sorted(hs):
    print('H', *r, sep='\t')
```

## C-13. The P-CHW script

Run from the repository root: `python3 <file> 84fee8fa`. It prints the count line and one row per function that
reads a raw child or sibling link (file, function, its line, the number of reading lines, `walk` or `hop`, the
lines).

```python
import re, subprocess, sys, collections
rev = sys.argv[1]
git = lambda *a: subprocess.run(('git',) + a, capture_output=True, text=True).stdout
RAW = re.compile(r'\b(get_first_child|get_last_child|get_next_sibling|get_prev_sibling)\(')
FN = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+(\w+)')
rows = collections.OrderedDict()
for f in sorted({l.split(':')[1] for l in git('grep', '-n', '-P', RAW.pattern, rev, '--', 'crates', ':!*tests*').split('\n') if l}):
    cur, start = '-', 0
    for i, l in enumerate(git('show', f'{rev}:{f}').split('\n'), 1):
        m = FN.match(l)
        if m:
            cur, start = m.group(1), i
        if RAW.search(l) and not (m and m.group(1) in RAW.pattern):
            k = (f, cur, start)
            rows.setdefault(k, []).append((i, sorted(set(RAW.findall(l)))))
lines = sum(len(v) for v in rows.values())
print('raw child reads:', lines, 'lines in', len(rows), 'functions,', len({k[0] for k in rows}), 'files (definition lines excluded)')
for (f, fn, st), v in rows.items():
    kinds = sorted({n for _, ns in v for n in ns})
    # a walk = the function reads a first/last child and a sibling: it can visit a whole child list
    walk = any('child' in n for n in kinds) and any('sibling' in n for n in kinds)
    print(f[len('crates/'):], fn, st, len(v), 'walk' if walk else 'hop', ','.join(str(i) for i, _ in v), sep='\t')
```

## C-14. Probes that were run

One, by the driver, for umbrella §0.1 item 6: a temporary VM test at `84fee8fa` (rustc 1.99.1) in a scratch
worktree, removed afterwards. With `h` a connected host, `sr = h.attachShadow({mode: "open"})` holding one
`<span>`, and `n` a new element:

| Call | Observed at `84fee8fa` | DOM ([D13] of the umbrella) |
|---|---|---|
| `h.removeChild(sr)` | no exception; `h.childNodes.length` is 0; `sr.host === h` and `h.shadowRoot === sr` still hold; the span's `isConnected` is still true | "NotFoundError" (*pre-remove* step 1: a shadow root's parent is null) |
| `h.insertBefore(n, sr)` | no exception; `n` is inserted | "NotFoundError" (*ensure pre-insert validity* step 3) |
| `h.replaceChild(n, sr)` | no exception; `sr` is replaced in the child list | "NotFoundError" (*replace* step 1, through the same step 3) |

The test is not in the tree; `ta`'s plan re-creates it as its first probe and keeps it as the cells of umbrella
§1.6. Everything else in this census that speaks of behaviour is a reading (C-7).
