# Tree model: a shadow root is a tree root; every climb chooses whether it crosses — program umbrella

Status: **umbrella, draft 1 (pre-implementation, not yet plan-reviewed)**. `/elidex-plan-review` runs on this
umbrella and on each per-PR plan before its code (CLAUDE.md, *Edge-dense work*; §2 has more than 3 intersecting
axes). Owner: L3. Base: `c0578223`; every code anchor is at that commit. Census data:
[2026-11-tree-model-census.md](2026-11-tree-model-census.md) (commands, counts, per-file tables; §6 here says what
each population is for).

**Ruling this memo implements: PM 730** (adopting the independent verification of the ecs-entity-lifetime PR (io)
plan at `23aa745a`). Decided there and not reopened here:

1. **(X) is canonical**: a shadow root is a tree root (no parent) and the host relation is held only by the link
   component pair. **(Y)** ("keep the shadow root as the host's tree child and treat `TreeRelation.parent` as the
   backward fact") is rejected (§4).
2. **A separate program, placed before PR (io)**: is → ig → [this program] → io → ii (the ids are the
   ecs-entity-lifetime program's). PR (is) and PR (ig) do not depend on this program.
3. **Edge-dense**: umbrella + per-PR plans, each plan-reviewed before code. Owner L3.
4. **Direction** (designed in §1 and §5, not prescribed by the ruling): first a behaviour-preserving PR in which
   every `get_parent` caller chooses between the tree parent and the shadow-including parent, so the compiler
   enumerates the population; then the PR that stops appending the shadow root under its host and deletes the
   child-list compensation; climbs that must not cross change observably to the spec, with cells.
5. **Lesson**: a census defines its population by a property, not by a name (§6).

## §0. Charter

### §0.1 Defects

1. **The backward fact of the host relation has two homes.** `attach_shadow_with_init` spawns the shadow root with
   `ShadowRoot { host, .. }` and then appends it to the host's child list
   (`crates/core/elidex-ecs/src/dom/shadow.rs:195-213`: `self.append_child(host, shadow_root_entity)` at `:210`), so
   "which element hosts this root" is stored in `ShadowRoot.host` **and** in `TreeRelation.parent`. The forward
   fact has two homes as well: `ShadowHost.shadow_root` and the host's child list.
2. **The code is split between two models.** Some sites are written for (X) and some for (Y), and each kind is
   wrong about the storage or about the spec:
   - *(X)-assuming, over (Y) storage*: `rev_version` documents "When the parent chain ends at a ShadowRoot (no
     parent), jump to the shadow host" (`crates/core/elidex-ecs/src/dom/mod.rs:743-761`);
     `is_host_including_ancestor_or_self` and `find_tree_root_composed`
     (`crates/core/elidex-ecs/src/dom/tree/walkers.rs:380-392`, `:436-439`) and `walk_parent`
     (`crates/core/elidex-ecs/src/dom/tree/mutation.rs:418-421`) read the tree parent first and the `.host` branch
     second. For an attached shadow root the stored parent is the host, so the `.host` branch is never entered:
     the result is right by accident, and the branch that the doc describes is dead.
   - *(Y)-assuming*: `is_connected`'s doc, "`get_parent` … returns `Some(host)` for a `ShadowRoot`, so the walk is
     shadow-including" (`walkers.rs:226-228`); `root_entities`' doc, "A shadow root is never parentless"
     (`crates/core/elidex-ecs/src/dom/tree/navigation.rs:359-360`); the VM's `parentNode` native, "a shadow root is
     not a tree child of its host even though the ECS edge points back"
     (`crates/script/elidex-js/src/vm/host/node_proto.rs:323-326`); dom-api's `find_root`, a shadow-including root
     computed by plain `get_parent` (`crates/dom/elidex-dom-api/src/node_methods/mod.rs:64-75`).
   - *Compensation added after the fact*: the strict parser's form-pointer walk, "`get_parent` returns a
     `ShadowRoot`'s host … so stop at the shadow root"
     (`crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs:303-310`); the selector combinators' "Stop at
     shadow boundary" (`crates/css/elidex-css/src/selector/matching.rs:33-36`, `:48-51`); `ChildrenIter` and
     `ChildrenIterRev`, "Skip ShadowRoot entities -- not exposed as children" (`dom/mod.rs:910-913`, `:944`).
3. **Under (Y) the unsafe direction is the default.** A climb that says nothing (`while let Some(p) =
   dom.get_parent(cur)`) crosses the shadow boundary. The spec's default relation (*parent*, *ancestor*, *root*)
   does not cross (§3); crossing is a separate named step. So every reader that was written without thinking about
   shadow trees is silently on the wrong side, and the compensation is opt-in per reader. Readers predicted to
   cross against the spec (census C-2, class "x"; **read from the code, not run**): MutationObserver's ancestor
   walks, Range and traversal, the form owner, fieldset disabledness, the option's select, tree order.
4. **Child-side compensation is per accessor and per caller.** Six accessors skip a shadow-root child (census
   C-4); the four raw accessors (`get_first_child`, `get_last_child`, `get_next_sibling`, `get_prev_sibling`,
   `navigation.rs:22-42`) do not, and 108 non-test lines call them (census P-CH). A raw reader with no guard of its
   own hands the shadow root to its consumer as a child of the host; the Range contents program already lists one
   such defect (D13, `docs/plans/2026-11-range-contents-program.md:731`).
5. **Derived relations are multiplied by the split.** Three roots (`find_tree_root`, `find_tree_root_composed`,
   dom-api's `find_root`) and three ancestor tests (`is_ancestor`, `is_light_tree_ancestor_or_self`,
   `is_host_including_ancestor_or_self`) exist because each caller had to pick a model (census C-6).

### §0.2 Why a program

More than three invariant axes intersect (§2: storage, child-list accessors, crossing climbs, non-crossing climbs,
lifetime, and more), and the population that breaks is not enumerated by the compiler: under a storage change
alone, a crossing climb that relies on the stored parent keeps compiling and silently stops at the boundary. That
is the *Edge-dense* trigger (a rule, not a judgment); the single-PR base case exists only for a per-PR slice under a
ratified umbrella. The slices of §5 are that base case.

### §0.3 Why before PR (io)

PR (io)'s central invariant is "each link's backward fact has one home" (ecs-entity-lifetime umbrella §2, OWN ×
back-link). For the shadow root that sentence is false at base (defect 1) and stays false while the tree parent is a
second home; the PR (io) plan itself leans on the tree parent (its §2 EDGE × adopt row deletes `walk_parent`'s host
fallback on the ground that "`walk_parent` read the tree parent first, and a shadow root is a tree child of its
host"). Landing PR (io) first would put a new seam beside the old representation, the
coexistence *One issue, one way* forbids. After this program the shadow root, the template's contents and the inert
template document are the same shape (an owned root reached only through its link), and PR (io) is rewritten on that
base (§7.2).

## §1. The rule (the single home; every other site refers here)

> **R-TREE.**
> 1. **The tree relation holds tree children only.** `TreeRelation` is the node tree of DOM §1.1 / §4.2: an
>    entity's stored parent is its DOM *parent*, and a child list contains its DOM *children* and nothing else. A
>    shadow root has no parent and is in no child list.
> 2. **The host relation is held only by the link pair**: `ShadowHost.shadow_root` on the host (forward) and the
>    shadow root's host (backward), each read through one function (`get_shadow_root`, `shadow_host`).
> 3. **Every upward step names the relation it follows.** There is no accessor called "parent" without a
>    qualifier, and no accessor whose answer depends on which kind of entity it was handed in a way its name does
>    not say.

### §1.1 The climb API

| Accessor | Relation | Definition (final, after slice `ts`) |
|---|---|---|
| `EcsDom::tree_parent` (NEW) | DOM §1.1 *parent* | the stored parent |
| `EcsDom::shadow_including_parent` (NEW) | one step of DOM §4.8 *shadow-including ancestor* | `tree_parent(e)`, else `shadow_host(e)` |
| `EcsDom::get_parent` | — | **deleted** in slice `ta`; no alias, no deprecated wrapper, tests included |

- **Two names, not one accessor with a mode argument.** The two relations are the two the spec names; a caller
  never needs to pass the choice as a value, and an enum argument would add a branch to hot loops (selector
  matching, style) for no caller. PR (io)'s `WalkScope` is an argument because one downward walker serves several
  scopes; an upward step has no such sharing. (Open to plan-review as a symmetry question, §7.3 Q-8.)
- **The fail-safe direction.** With `get_parent` gone there is no unmarked step. A reader that picks the shorter
  name gets the spec's default relation and stops at the boundary. A new owned kind (a template's contents, the
  inert template document) can never widen an existing climb: `tree_parent` reads no link, and
  `shadow_including_parent` reads exactly one.
- **The link is read in one place.** `shadow_including_parent` and every derived relation that crosses call
  `shadow_host` (`shadow.rs:521`); no climb reads `ShadowRoot.host` itself. PR (io) then moves that one read onto
  its back-link without touching a climb.
- **Host-including is not a general step.** DOM §4.7 *host-including inclusive ancestor* crosses every
  DocumentFragment host (a shadow root's and a template contents'). It has one consumer class (*ensure pre-insert
  validity* step 2 and *replace*), so it stays one function, `is_host_including_ancestor_or_self`, written as
  "tree climb, then the fragment's host". At this program's end it knows the shadow host only, as at base; the
  contents' host arrives with the follow-up PR after PR (io) (§7.2).
- **Derived relations: one function each**, built from the two steps: tree root, shadow-including root, inclusive
  ancestor, shadow-including inclusive ancestor, host-including inclusive ancestor, connected. The duplicates of
  §0.1 item 5 fold into these (dom-api's `find_root` and `is_light_tree_ancestor_or_self`'s marker stop go;
  `is_ancestor`, the `append_child` cycle guard, becomes the host-including test, which is what DOM §4.2.3 asks
  for). Names of surviving functions are the per-PR plans' call.
- **Not introduced here: an ancestor iterator.** The hand-rolled climb loops with their own depth guards are the
  DOM-walker program's population (`#11-dom-walkers-iterative-over-ecs-links`); this program changes which step
  each loop takes, not the loop.

### §1.2 The child side

One accessor set reads the tree children: first child, last child, next sibling, previous sibling, the two
iterators, `children`, `child_list_uncapped`, `index_in_parent`. The distinction "raw" versus "exposed" disappears:
`next_exposed_sibling` / `prev_exposed_sibling` fold into the sibling accessors, and every marker test that exists
only to skip a shadow-root child (census C-5 role K) is deleted. A reader that wants the shadow tree asks for it:
`get_shadow_root(host)` and then that root's children.

### §1.3 The transitional accessor (declared, bounded by the program)

`EcsDom::parent_crossing_pending_flip` (NEW) is transitional: it has the behaviour of the deleted `get_parent` over (Y)
storage. Slice `ta` must be behaviour-preserving, and some callers' spec relation is the tree parent while their
base behaviour crosses (census C-2 class "x"). Such a caller cannot take `tree_parent` in `ta` without changing
behaviour, and taking `shadow_including_parent` would record a wrong relation as if it were chosen. It takes the
transitional accessor instead, so that **the backlog of observable changes is a compiler-enumerable population**,
not a list in a memo.

- Its callers are flipped to `tree_parent` by the flip slices (`tc`, `tm`, `tr`, `tf`, `te`), each with its cells.
- It is deleted by the last flip slice. Slice `ts` requires that it does not exist.
- No new caller may be added after `ta`; each flip plan re-runs `git grep -n -P '\bparent_crossing_pending_flip\('`
  and its count only falls.

This is a strangler state inside the program, declared here with its removal condition; the alternatives are in §4.

### §1.4 Relations this program does not define

Two upward relations are slot-aware and have their own owners: the **flat-tree parent** (css-shadow-1 §4.1; style
inheritance, layout, paint, hit testing, hover and scroll chains, accessibility) and the event **get the parent**
(DOM: "A node's get the parent algorithm, given an event, returns the node's assigned slot, if node is assigned;
otherwise node's parent"; a Document's parent is its Window). At base those readers climb the stored parent, which
equals the shadow-including step. In `ta` they take `shadow_including_parent` (behaviour unchanged, the crossing now
written down) and are listed as class F in the census. Making them slot-aware is `#11-shadow-slot-assignment-flat-tree`
(P2, Layout) and the event-path plan's (`docs/plans/2026-11-event-path-window.md`); R-TREE gives both a storage in
which "parent" already means the tree parent (§7.2).

## §2. Coupled invariants (edge-matrix)

Elements:
- **TREE**: the stored tree relation (R-TREE 1). **LINK**: the link pair and its two readers (R-TREE 2).
- **UP-T**: climbs whose relation is the tree parent. **UP-S**: shadow-including climbs. **UP-H**: the
  host-including test. **FLAT**: flat-tree and event-parent climbs (§1.4).
- **CHILD**: the child-side accessor set (§1.2). **ORD**: tree order and child index (`tree_order_cmp`,
  `index_in_parent`). **CONN**: connectedness. **ROOTS**: `root_entities` (render / layout / hit-test root discovery).
- **LIVE**: live collections and `rev_version`'s upward propagation. **MEV**: the ECS `MutationEvent` fire guards
  (`mutation.rs:528`, `:582`). **CE**: custom-element reaction fan-out over shadow-including descendants.
- **MO**: MutationObserver records. **RANGE**: Range, Selection, NodeIterator, TreeWalker. **FORM**: form owner,
  fieldset disabledness, option / select / optgroup, legend. **EL**: element-level HTML climbs (focus predicates,
  inert and dialog ancestors, contenteditable, lang / dir, CSSOM View queries, presentational hints).
- **SEL**: selector matching. **RENDER**: style, layout and paint walks. **A11Y**. **SHELL**: input, hover, scroll.
- **PARSE**: parser guards (the strict form pointer; `innerHTML` on a shadow root). **LIFE**: teardown
  (`destroy_entity`, `despawn_subtree`). **CLONE**: clone and adopt walks. **WR**: the VM's navigation natives and
  the dom-api handlers behind them.

| Pair | Intersection | PR | Pinned by |
|---|---|---|---|
| TREE × LINK | attach writes the link pair and no tree edge; the host's child list never contains the root; the backward fact is read only through `shadow_host` | ts | ts structure cells (attach; raw child list of a host) |
| TREE × UP-T | `tree_parent(sr)` is `None` at every head from `ta` on: by a marker test over (Y) storage until `ts`, by the storage after | ta, ts | ta cell (accessor contract); ts removes the marker test with no cell change |
| TREE × UP-S | `shadow_including_parent(sr)` is the host at every head: the stored parent until `ts`, `shadow_host` after | ta, ts | ta cell; ts: the same cell, unchanged expectation |
| UP-T × UP-S | every climb is exactly one of the two (or the transitional accessor until its flip); none mixes steps inside one loop except the host-including test, which is one function | ta | the compiler (`get_parent` is gone); ta census re-run |
| UP-S × CONN | *connected* = the shadow-including root is a document: `is_connected` climbs by `shadow_including_parent`; a node in a shadow tree of a connected host stays connected across `ts` | ta, ts | existing connectedness cells; ts adds a nested-shadow case |
| UP-H × TREE | the cycle guard of `append_child` is the host-including test, so appending a host into its own shadow tree is still rejected once the stored parent no longer crosses (DOM §4.2.3 *ensure pre-insert validity* step 2) | ta (choice), ts (storage) | ts cell: host into own shadow tree, nested two levels |
| CHILD × TREE | until `ts` the one accessor set skips a shadow-root child in the accessor layer; after `ts` there is nothing to skip and the skip is deleted | tc, ts | tc cells per accessor; ts: unchanged expectations |
| CHILD × ORD | a child's index and the order of siblings count tree children only; a host's shadow root shifts no index (Range contents program D13 (a)) | tc | tc cell: index of a host's children with a shadow root attached before and after them |
| CHILD × WR | `firstChild` / `lastChild` / `nextSibling` / `previousSibling` / `childNodes` read the one accessor set; the natives' own marker tests go | tc | existing VM navigation cells; tc adds the dom-api handler path |
| UP-T × WR | `parentNode` / `parentElement` of a shadow root are null through every path (DOM §4.4: "return this's parent"); the dom-api handlers have no guard at base (`crates/dom/elidex-dom-api/src/tree_nav.rs:78`, `:101`) | ta (transitional), tc (flip) | tc cell through the handler |
| UP-T × MO | *queue a mutation record* step 2 collects from the **inclusive ancestors** of the target: an observer on the host or the document with `subtree: true` gets no record for a mutation inside the shadow tree; transient observers follow the same ancestors | tm | tm cells (host observer, document observer, observer on the shadow root itself still gets them) |
| UP-T × RANGE | a live range's root is the root of its start node; *get the common ancestor* climbs by parent; boundary-point order is defined within one tree; traversal stays inside its root | tr | tr cells; coordination with the Range contents program's RC-1 (§7.2) |
| UP-T × FORM | *reset the form owner* step 4 ("has an ancestor form element"), *disabled fieldset* ("a descendant of another fieldset"), *nearest ancestor select*: all tree ancestors, so a control in a shadow tree is not owned or disabled by light-tree ancestors of its host | tf | tf cells; intersects `#11-form-owner-ecs-component` (§7.2) |
| UP-T × EL | per climb, the relation is looked up, not assumed: lang and dir **cross** by spec (HTML §3.2.6.2, §3.2.6.4 name the shadow root's host), so they are UP-S in `ta`; the rest are decided in the `te` plan | ta, te | te cells per climb |
| UP-T × SEL | combinators match tree ancestors and siblings; the marker stop in `matching.rs` is replaced by `tree_parent`, and the shadow root (not an element) ends the climb | ta | existing selector cells, unchanged |
| UP-T × PARSE | the strict form-pointer walk is a tree climb; its guard goes | ta | the strict parser's existing shadow-context cell |
| UP-T × ORD | `tree_order_cmp` compares within one tree and falls back for different roots; at base its `ancestor_chain` crosses, so a shadow-tree node and a light node are ordered as one tree | te | te cell; consumers (`compareDocumentPosition`, Range) listed in the te plan |
| UP-S × LIVE | `rev_version` propagates to shadow-including ancestors so collections rooted at or above the host invalidate; its documented `.host` branch becomes the reachable one | ta, ts | existing live-collection cell across a shadow boundary; ts keeps it green |
| UP-S × CE, CLONE | the downward shadow-including walks reach the root through `get_shadow_root`, never through a child list (already true: `for_each_shadow_inclusive_descendant`, `walkers.rs:59-88`); the adopt walk's `walk_parent` is `shadow_including_parent` | ta, ts | existing CE and adopt cells, unchanged |
| FLAT × UP-S | class F readers take the shadow-including step, as at base; nothing in this program makes them slot-aware | ta | no expectation changes; hand-off §7.2 |
| ROOTS × TREE | a shadow root becomes parentless; `root_entities` already excludes `DocumentFragment` kinds, so it is still not a render root; the doc's reason changes | ts | ts cell: `root_entities` with an attached shadow root |
| RENDER × TREE | "is this the root" tests written as `tree parent is none` (six layout lines, two render / style lines, census C-2) must not become true for an entity reached inside a shadow tree | ta (choice per line), ts | ts cell per test, or a proof that the receiver is never a shadow root |
| MEV × TREE | attach fires no `Insert` for the shadow root (it is not inserted); the guards for a shadow-root **parent** are examined before they are touched (§7.3 Q-4) | ts | ts cell: attach fires nothing; a child appended to a shadow root behaves as at base |
| LIFE × TREE | `destroy_entity(host)` no longer finds the root in the child list it orphans; the root is left as it is today (alive, stripped, parentless): the lifetime program's defect, unchanged. `destroy_entity(sr)` and `despawn_subtree(sr)` read "the external parent" (`teardown.rs:90`, `:221`) for index, connectedness and the version bump: the version bump goes to the host through the shadow-including step; the Remove fire is already suppressed for a shadow root | ts | ts cells; existing `destroy.rs` cells that pin the stripped root keep their expectations |
| LIFE × LINK | this program neither adds a back-link nor changes what is reclaimed; `ShadowRoot.host` stays the backward home until PR (io) folds it | — | structural (no slice touches the strip) |
| SHELL, A11Y × FLAT | as FLAT × UP-S; `a11y/tree.rs` already hops explicitly through the host | ta | unchanged |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §1.1 Trees | *parent*, *root*, *inclusive ancestor*, *index* | the default relations do not mention hosts | `tree_parent`, the child accessor set (ta, tc) | ✓ | yes |
| WHATWG DOM §4.2.2 Shadow tree | "a node tree whose root is a shadow root"; *connected* | the root has a null parent; connectedness goes through the shadow-including root | storage (ts); `is_connected` (ta) | ✓ | yes |
| WHATWG DOM §4.7 Interface DocumentFragment | *host* ("null or an element in a different node tree"); *host-including inclusive ancestor* | the host is not in the fragment's tree; the ancestor test crosses explicitly | the one host-including function (ta); template contents' host: not here (§7.2) | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *shadow-including root*, *shadow-including descendant / ancestor*, *retarget* step 2; "A shadow root's get the parent algorithm … otherwise shadow root's host" | each crossing is written as "root's host" | `shadow_including_parent`, `shadow_host` (ta) | ✓ | yes |
| WHATWG DOM §4.2.3 Mutation algorithms | *ensure pre-insert validity* step 2 | node is a host-including inclusive ancestor of parent → `HierarchyRequestError` | `is_ancestor` → host-including (ta, ts) | ✓ | yes |
| WHATWG DOM §4.3.2 Queuing a mutation record | step 2 | "the inclusive ancestors of target" | observer ancestor walks (tm) | ✓ | yes |
| WHATWG DOM §4.4 Interface Node | `parentNode` ("return this's parent"), `parentElement`, `isConnected` | a shadow root's parent is null | navigation handlers (tc) | ✓ | yes |
| WHATWG DOM §5.6 Interface Range | *root* of a live range; *get the common ancestor* step 2 ("set container to container's parent") | within one tree | Range, Selection, traversal climbs (tr) | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | "The template contents of a template element are not children of the element itself"; establishing step 2 ("host is the template element") | the precedent: an owned root reached by a link, already (X)-shaped in elidex | none (comparison only; §4) | ✓ | no |
| WHATWG HTML §4.10.18.3 Association of controls and forms | *reset the form owner* step 4 | "has an ancestor form element … the nearest such ancestor" | form-owner climbs (tf) | ✓ | yes |
| WHATWG HTML §4.10.15 The fieldset element | *disabled fieldset* | "a descendant of another fieldset element whose disabled attribute is specified" | fieldset climbs (tf) | ✓ | yes |
| WHATWG HTML §4.10.10 The option element | *nearest ancestor select* step 2 | "For each ancestor of element's ancestors, in reverse tree order" | option / select climbs (tf) | ✓ | yes |
| WHATWG HTML §4.10.22.3 Form submission algorithm | "Let subject be form's nearest ancestor dialog element" | tree ancestor | dialog climb in `submit.rs` (tf) | ✓ | yes |
| WHATWG HTML §3.2.6.2 The lang and xml:lang attributes | *language* | "If the node's parent is a shadow root: use the language of that shadow root's host" | lang inheritance: UP-S (ta) | ✓ | yes |
| WHATWG HTML §3.2.6.4 The dir attribute | *parent directionality* step 2 | "If parentNode is a shadow root, then return the directionality of parentNode's host" | dir inheritance: UP-S (ta) | ✓ | yes |
| CSS Selectors L4 §14.1 Descendant combinator ( ) | combinators | ancestors in the element tree; no crossing | `matching.rs` (ta) | ✓ | yes |

**Breadth**: K=3 specs (DOM, HTML, Selectors), M=16 entries (verified 2026-11-15:
`.claude/tools/webref coverage-map dom 1.1 dom 4.2.2 dom 4.7 dom 4.8 dom 4.2.3 dom 4.3.2 dom 4.4 dom 5.6 html 4.12.3 html 4.10.18.3 html 4.10.15 html 4.10.10 html 3.2.6.2 html 3.2.6.4 html 4.10.22.3 selectors-4 14.1`,
verdict "ok → single PR scope"). The split into slices is not a breadth verdict; it follows from
§0.2. css-shadow-1 §4.1 (*flat tree*) is cited in §1.4 only as the relation this program does not define.

**How the spec words (X).** Both HTML rows that cross say "the node's parent **is a shadow root**" and then name
"that shadow root's host": the parent chain ends at the shadow root, and the host is reached by a second, named
step. No spec text reaches a host by "parent".

### §3.1 User-input touch audit

Script-observable behaviour changes, by slice. All are **predicted from the code** (census C-7 item 3); each flip
plan opens with a probe that runs the base behaviour, and a prediction that fails shrinks the slice.

- `ta`, `ts`: none intended. `ta`'s acceptance is that no existing cell changes its expectation; `ts`'s is the same,
  except elidex-ecs structure cells that pin the shadow root inside its host's raw child list.
- `tc`: `shadowRoot.parentNode` / `parentElement` through the dom-api handler path; any raw child / sibling reader
  that exposes a shadow root as a child of its host (Range boundary indices and walks, D13; others found by the tc
  plan's read of census C-3).
- `tm`: `MutationObserver` with `subtree: true` on a host, an ancestor of a host, or the document no longer receives
  records for mutations inside the shadow tree.
- `tr`: `Range.commonAncestorContainer`, boundary comparison, `isPointInRange` / `comparePoint`, contained-node
  enumeration and `TreeWalker` / `NodeIterator` steps no longer leave the shadow tree upward.
- `tf`: a form control inside a shadow tree has no form owner from a light-tree `<form>` around its host; it is not
  disabled by a light-tree `<fieldset disabled>` around its host; an `<option>` does not find a `<select>` across the
  boundary; `method=dialog` does not find a dialog across it.
- `te`: per climb (focus predicates, contenteditable, CSSOM View queries, presentational hints, tree order), decided
  in the te plan after the relation of each is looked up.
- Adjacent pre-existing code whose exposure does not change: the insertion guards that reject a shadow root as a
  node (census C-5 role I); event retargeting and `composedPath` (they read `find_tree_root` and the marker, not the
  stored parent).

## §4. Lens block (basis order)

- **spec (first)**: §3. A shadow tree's root is a shadow root (DOM §4.2.2), a root has a null parent (§1.1), the
  host is "an element in a different node tree" (§4.7), every crossing is the explicit step "root's host" (§4.8),
  *queue a mutation record* uses plain inclusive ancestors (§4.3.2), and HTML's template contents "are not children
  of the element itself" (§4.12.3). The stored relation that matches all of these is (X).
- **elidex principles (where the spec does not choose a representation)**:
  - *One issue, one way*: a template's contents and the inert template document are already (X)-shaped
    (`attach_template_contents` links the fragment and appends nothing, `dom/mod.rs:620-643`); the shadow root is the
    outlier. After the program all three owned roots have one representation, and "parent" has one meaning.
  - *Fail-safe direction*: the climb that says nothing must not cross. (X) makes the spec's default the code's
    default and the crossing the marked case (§1.1).
  - *ECS-native*: a relation is a component, and two relations are two components. (Y) stores the host relation
    inside the tree component and then subtracts it again with marker tests at every reader ("a child, but hidden").
    (X) needs no subtraction: `TreeRelation` is the tree, the link pair is the host relation, and a system that wants
    both asks for both.
  - *Ideal over pragmatic*: delete `get_parent` rather than document a convention for it.
- **Rejected: (Y)**, keeping the shadow root as the host's tree child (PM 730). It contradicts §4.7's "different
  node tree", leaves two representations for three owned roots, makes the unsafe direction the default, and needs a
  compensation per iterator and per climb (23 marker lines today, census C-5 roles K and B).
- **Rejected: making the template contents a tree child too** (unify on (Y)). It extends the same compensation to a
  second kind and contradicts §4.12.3's "not children".
- **Rejected: keeping `get_parent` with a documented convention** ("`get_parent` is the tree parent; call
  `shadow_including_parent` to cross"). The compiler then enumerates nothing: the 142 existing calls keep compiling
  with a changed meaning, which is the failure the independent verification found (a name-defined census that never
  saw them). A rename with no survivor is the only form in which the population is closed.
- **Rejected: changing the storage first.** Every crossing climb that leans on the stored parent would stop at the
  boundary without a compile error or, for most, a failing cell.
- **Rejected for §1.3: recording pending flips in a memo table instead of a transitional accessor.** File and line
  anchors drift with every PR, and nothing fails when a pending site is forgotten. **Also rejected: flipping inside
  `ta`.** `ta` would then carry the MutationObserver, Range, forms and focus cells in one PR, the bundle the
  *Edge-dense* rule exists to prevent.

## §5. The program

| # | Slice | Primary module(s) | Slot | Tier | Deps |
|---|---|---|---|---|---|
| **ta** | **Every climb chooses.** `tree_parent` (NEW), `shadow_including_parent` (NEW) and the transitional accessor (§1.3) replace `get_parent`, which is deleted, tests included. Each call names its relation in the plan's per-line table (census C-2 is its seed, not its answer): the tree parent with a proof that base behaviour is equal (the receiver is never a shadow root, or an existing guard is folded into the choice); the shadow-including parent where the spec relation crosses or is class F; the transitional accessor where the relation is the tree parent and base crosses. Derived relations fold to one function each (§1.1). Behaviour-preserving. | `elidex-ecs` `dom/tree/navigation.rs`, `walkers.rs`, `mutation.rs`, `tree/mod.rs`, `dom/mod.rs`; every crate of census C-2 (21 crates) | `#11-shadow-root-is-a-tree-root` (NEW, proposed name; §7.1) | — | — |
| **tc** | **One child-side accessor set.** The raw and "exposed" accessors become one set that reads tree children (the skip lives in the accessor layer until `ts`); `next_exposed_sibling` / `prev_exposed_sibling` fold in; caller-side skips (census C-5 role K) are deleted; the `parentNode` / `parentElement` handlers flip to `tree_parent`. Observable where base exposes a shadow root as a child or as a parent-bearing node (§3.1). | `elidex-ecs` `dom/tree/navigation.rs`, `dom/mod.rs` (iterators); `elidex-dom-api` `tree_nav.rs`, `live_collection/mod.rs`, `range/mod.rs`; `elidex-style` `walk.rs`; `elidex-js` `vm/host/node_proto.rs` | as ta | — | ta |
| **tm** | **MutationObserver climbs are tree climbs.** The two ancestor walks flip from the transitional accessor to `tree_parent`. | `elidex-api-observers` `mutation/mod.rs` | as ta; touches `#11-shadow-mutation-observer-disconnected-cleanup` (§7.1) | — | ta |
| **tr** | **Range, Selection and traversal climbs are tree climbs.** | `elidex-dom-api` `range/`, `selection.rs`, `traversal.rs`; `elidex-js` `vm/host/range_proto.rs` | as ta; `#11-range-compare-points-shadow-index`, `#11-shadow-tree-live-range-tracking` (§7.1) | — | ta |
| **tf** | **Form-association climbs are tree climbs**: form owner, fieldset disabledness, option / select / optgroup, the dialog of `method=dialog`, the shell's form-ancestor lookup. | `elidex-form` `submit.rs`, `select/mod.rs`; `elidex-form-core` `fieldset.rs`; `elidex-dom-api` `element/option_disabled.rs`; `elidex-shell` `content/form_input.rs` | as ta; `#11-form-owner-ecs-component`, `#11-form-elements-cross-tree` (§7.1) | — | ta |
| **te** | **The remaining element-level climbs**, each with its relation looked up first: focus predicates, contenteditable, CSSOM View queries, presentational hints, tree order. Deletes the transitional accessor (last flip). | `elidex-dom-api` `focus/`, `element/layout_query.rs`; `elidex-dom-compat` `presentational.rs`; `elidex-ecs` `dom/mod.rs`, `walkers.rs` (`ancestor_chain`) | as ta | — | ta, tc, tm, tr, tf |
| **ts** | **Stop appending.** `attach_shadow_with_init` writes the link pair and no tree edge; the accessor-layer skips and `tree_parent`'s marker test are deleted; `shadow_including_parent` reads `shadow_host`; the elidex-ecs field readers (census P-REL, 41 lines) are re-read against R-TREE; teardown's "external parent" reads, the cycle guard, `root_entities`' and `is_connected`'s docs, and the `(X)`/`(Y)` docs of §0.1 item 2 say one thing. Behaviour-preserving for script. Closes the slot. | `elidex-ecs` `dom/shadow.rs`, `dom/mod.rs`, `dom/tree/*.rs`, `components.rs` | `#11-shadow-root-is-a-tree-root` (closes) | — | te |

**Order.** `ta` first: it creates the vocabulary and the compiler's list. `tc`, `tm`, `tr` and `tf` each depend
only on `ta` and share no cell, so their mutual order is free; the proposed order is tc → tm → tr → tf (smallest
blast radius first, and `tr` after `tc` because Range reads both sides). `te` is last among the flips because it
deletes the transitional accessor. `ts` is last: by then every reader states its relation and no reader depends on
which storage is underneath, so the storage change is local to elidex-ecs. Then PR (io) of the ecs-entity-lifetime
program, rewritten on this base (§7.2).

**Why the non-crossing changes are their own slices, and not part of the storage PR.** By the *Edge-dense* base
case a slice is narrow and terminal. Each flip cluster has its own spec rows (§3), its own cells and its own owner
of adjacent slots (MutationObserver, Range program, form owner, focus), and none shares a mutant with another or
with the storage change. Bundling them with `ts` would make one PR change the storage **and** five families of
script-visible behaviour, so a regression could not be attributed. With the flips first, `ts` has a one-line
acceptance: nothing script-visible changes.

**Why `tc` is separate from `ts`.** The child side has its own population (census P-CH, P-CHX) and its own
observable fixes. Moving the skip into the one accessor set first means `ts` deletes one layer instead of editing
every caller, and the fixes are reviewed with their cells rather than as a side effect of a storage change. The
cost is a skip that lives in the accessor layer from `tc` to `ts` (§7.3 Q-1 asks plan-review to confirm this over
folding `tc` into `ts`).

**Acceptance.**
- `ta`: `get_parent` does not exist; the plan's per-line table covers every line the compiler reported (its count
  equals census P-UP + P-UP-T call lines at its base, re-measured); no existing cell changes its expectation; the
  transitional accessor's caller count is recorded as the backlog.
- `tc`, `tm`, `tr`, `tf`, `te`: each opens with a probe of the predicted base behaviour (run, not read); each
  changed behaviour has a cell and a mutant on the final head; the transitional caller count only falls; after `te`
  the accessor does not exist.
- `ts`: no tree edge between a host and its shadow root; no marker test of census C-5 role K or B remains
  (`ta` / `tc` removed the callers', `ts` the accessor layer's); no script-visible cell changes its expectation;
  the census is re-run at its head and each population's remaining lines are accounted for.
- Program: R-TREE holds; §0.1 items 1 to 5 are gone; PR (io)'s "one home" sentence is true for the shadow root
  before PR (io) is written against it.

## §6. The census, and how each population maps to slices

Populations are defined by what the code does; the commands, counts, controls and per-file tables are in the census
file. Summary at `c0578223` (non-test unless noted):

| Population | Property | Count | Consumed by |
|---|---|---|---|
| P-UP | reads the stored parent through the public accessor | 143 lines (142 calls + the definition); 174 − 143 = 31 further lines under test paths (§8, verified 2026-11-15) | `ta`: all of them, by compile error |
| P-REL | reads a `TreeRelation` link field directly | 41 lines, all inside elidex-ecs (the fields are `pub(crate)`) | `ts`: re-read each; `ta` for the climbs among them (`rev_version`) |
| P-CH | reads a raw child / sibling link | 108 lines; 142 − 108 = 34 further lines under test paths (§8, verified 2026-11-15) | `tc` |
| P-CHX | reads through a compensating child-side accessor | six accessors (census C-4) | `tc` (fold), `ts` (delete the skip) |
| P-MARK | tests or reads the `ShadowRoot` component | 63 lines (61 true), by role: K 13, B 10, C 5, E 2, L 2, I 16, D 13 | K → `tc`; B, C → `ta`; E → `ts` (after §7.3 Q-4); L → the lifetime program; I, D stay |
| P-HOST / P-HOSTFN | reads the backward link | 14 lines in shadow-related files; 2 for `shadow_host` | `ta` routes climbs through `shadow_host`; the field's other readers are PR (io)'s census |
| P-DER | calls a derived relation | per function (census C-6) | `ta` (fold duplicates) |

**What the census changed about the plan.** (1) The parent side and the child side are separate populations with
separate instruments, hence `ta` and `tc`. (2) Direct field readers cannot exist outside elidex-ecs, so `ts` is
confined to one crate once the accessors are in place. (3) P-MARK by role shows which marker tests are compensation
(23 lines, K + B) and which are identity or data and stay (29). (4) The independent verification's numbers
(143 / 31, 108, 63, 41, 2) reproduce at `c0578223`.

**Ratchet.** After `ta` the population "unqualified parent read" is empty by construction (no such function). After
`ts`, direct field access is confined by visibility; narrowing the fields from `pub(crate)` to the `dom::tree`
module, so that the compiler also confines P-REL inside elidex-ecs, is `ts`'s call.

## §7. Slots, hand-offs and open questions

### §7.1 Slots

| Slot | Why deferred | Trigger | Re-eval |
|---|---|---|---|
| `#11-shadow-root-is-a-tree-root` (NEW, proposed name; registration and ledger text are the PM's) | the program's own slot: R-TREE does not hold at base (§0.1) | closed by `ts`'s landing | 2026-12-31 |
| `#11-template-contents-host-link` | not this program's: the template contents' DocumentFragment host. PM 730 splits the "host" cluster out of PR (io); a small PR after PR (io) closes it, and extends the one host-including function (§1.1) | that PR's landing | the slot's own |
| `#11-shadow-slot-assignment-flat-tree` (P2, Layout) | owns the flat-tree parent and the event *get the parent* change (§1.4); class F readers stay on the shadow-including step until then | the slot's own | the slot's own |
| `#11-event-path-window-missing` | `build_propagation_path` is a class F climb; its plan adds the Window above the Document, on the same loop `ta` re-types | the slot's own | the slot's own |
| `#11-shadow-mutation-observer-disconnected-cleanup`, `#11-shadow-tree-live-range-tracking` | neighbours of `tm` and `tr`; their ledger rows have no text beyond the id, so whether a flip closes, narrows or leaves them is decided in the tm / tr plans | tm's / tr's plan-review | the slots' own |
| `#11-range-compare-points-shadow-index` | Range contents program D13: a host's shadow root counted as a child. Under `tc` the defect cannot be expressed; which program lands the fix is §7.3 Q-2 | RC-1 or `tc`, whichever lands first | the slot's own |
| `#11-form-owner-ecs-component`, `#11-form-elements-cross-tree` | `tf` changes the ancestor half of the form owner; the three divergent resolvers and the `form`-attribute half stay with the first slot | the slots' own | the slots' own |
| `#11-vm-isconnected-bound-document` | VM `isConnected` compares against the bound document, not the shadow-including root. Not this program's; `ta` gives it the one `is_connected` | the slot's own | the slot's own |
| `#11-dom-walkers-iterative-over-ecs-links` | owns climb loops and their depth guards (§1.1, last bullet) | the slot's own | the slot's own |
| `#11-shadowroot-detached-not-despawned-on-host-destroy` | the lifetime program's; `ts` leaves the stripped root exactly as it is (§2 LIFE × TREE) | PR (ii) | the slot's own |

### §7.2 Hand-offs

- **ecs-entity-lifetime program** (L3; umbrella and PR (io) plan on branch `ecs-shadow-root-lifetime-plan`,
  `23aa745a`). After `ts`: (1) PR (io) is rewritten. Its back-link `OwnedBy` folds `ShadowRoot.host`, and the fold is
  one edit at `shadow_host`, since no climb reads the field (§1.1). Its claim "removed `walk_parent`'s host fallback
  because the tree parent is read first" is replaced by "the adopt walk climbs by `shadow_including_parent`". Its
  link census's missing population (the parent readers) is this program's P-UP, already closed. (2) The "host"
  cluster (the template contents' host, the pre-insert cycle cell) leaves PR (io) for the small follow-up PR that
  closes `#11-template-contents-host-link` (PM 730). (3) That umbrella's order line gains this program between PR
  (ig) and PR (io); the edit is that program's, at its next revision.
- **Layout T3 / T3-pre** (L2; plan not on main, not read in full for this draft). T3 "lands after PR (io)", so it
  lands after this program. Its plan cites `build_propagation_path`'s parent climb and the child iterators
  (`children_iter_rev`, `traverse_descendants`); Layout re-anchors those on `shadow_including_parent` / the one
  accessor set at `ta`'s / `tc`'s head. Whether T3's `SubtreeCreated` walks read a parent or a raw child link
  anywhere else is a question for Layout (§7.3 Q-10). The event-path plan
  (`docs/plans/2026-11-event-path-window.md`) touches the loop at
  `crates/script/elidex-script-session/src/event_dispatch/mod.rs:203-221`, which `ta` re-types: whichever lands
  second rebases one line.
- **Range contents program** (L3, `docs/plans/2026-11-range-contents-program.md`). RC-1 plans "exposed child list
  for every Range index, length and walk (D13)". After `tc` there is no other child list, so that part of RC-1
  becomes "use the accessors". Order question: §7.3 Q-2.
- **Brand-check program** (Rev 5.3, frozen). No dependency in either direction: its marker reads are identity
  (census C-5 role I, `entity_kind.rs`), which R-TREE does not touch.
- **VM P4.** No overlap found: no file under `docs/plans/` whose name contains `vm` mentions `get_parent` or a
  shadow root (`git grep -l -i -P 'get_parent|shadow root|ShadowRoot' c0578223 -- 'docs/plans/*vm*'` prints
  nothing), and none of the 16 `origin/vm-*` branches touches elidex-ecs `dom/` or the VM host files of census C-2
  (`git diff --name-only c0578223...<branch>` per branch, 2026-11-15). The VM host lines of this program (four files,
  census C-2) are `ta` / `tc` / `tr` edits of one line each.

### §7.3 Open questions (for the PM unless marked)

- **Q-1 (plan-review).** `tc` puts the shadow-root skip into the one accessor set and `ts` deletes it. The
  alternative folds `tc` into `ts` (no interim layer, but a storage PR with script-visible fixes). This memo chooses
  the separate slice (§5).
- **Q-2.** `tc` / `tr` against the Range contents program's RC-1 (D13 and its "exposed child list"): which lands
  first, and does RC-1's plan get re-cut after `tc`? Both are L3.
- **Q-3.** The slot name and its registration (§7.1 row 1); the landing of this umbrella relative to the lifetime
  umbrella's next revision.
- **Q-4 (fact needed before `ts`'s plan; a scout read).** `fire_after_insert` and `fire_after_remove` return early
  when the **parent** is a shadow root (`mutation.rs:528`, `:582`). Why, and which consumers (custom-element
  reactions, live ranges, form state) therefore see nothing for a direct child of a shadow root? If that is a defect
  it is not R-TREE's; `ts` must only not change it by accident.
- **Q-5.** Every "x" is a prediction. If a flip plan's probe shows base already non-crossing for its whole cluster,
  the slice reduces to the accessor rename and may merge into a neighbour; the PM is told at that plan's review.
- **Q-6.** `tf` and `#11-form-owner-ecs-component`: flip the ancestor climb in `tf` as planned, or leave all form
  owner behaviour to that slot's program and let `tf` only cover fieldset / option / dialog?
- **Q-7 (facts for the `ta` plan; scout reads).** (a) Per call, the relation of the lines marked "?" in census C-2
  (root tests in layout / render / style, `layout_query.rs`, focus, `selector/traverse.rs`,
  `iframe/lifecycle.rs`). (b) Whether a shadow-root entity can reach the six layout "is root" tests. (c) Which test
  lines pin (Y) storage as a contract (census C-7 item 4).
- **Q-8 (plan-review).** Two named steps versus one step with a scope argument, mirroring PR (io)'s `WalkScope`
  (§1.1).
- **Q-9.** From `ta` to `ts`, `tree_parent` costs one extra component lookup per step (the marker test over (Y)
  storage). It is on the selector and style paths. Measure in the `ta` plan and accept, or shorten the window by
  moving `ts` forward (it only needs the transitional accessor gone)?

- **Q-10 (Layout).** Do T3 / T3-pre read a stored parent or a raw child link outside `build_propagation_path` and
  the child iterators (§7.2)?

## §8. Measurements (provenance, at `c0578223`, run 2026-11-15)

All counts are reproduced by the commands in the census file (C-1 to C-6); the ones this memo quotes:

- P-UP: `git grep -n -P '\bget_parent\(' c0578223 -- crates ':!*tests*' | wc -l` → 143;
  `git grep -n -P '\bget_parent\(' c0578223 -- crates | wc -l` → 174; in 60 non-test files and 21 crates
  (`… | cut -d: -f2 | sort -u | wc -l`; `… | cut -d: -f2 | cut -d/ -f2-3 | sort -u | wc -l`).
- P-CH: `git grep -n -P '\b(get_first_child|get_last_child|get_next_sibling|get_prev_sibling)\(' c0578223 -- crates ':!*tests*' | wc -l` → 108; the same over `-- crates` → 142.
- P-MARK: `git grep -n -P '::<&?(\w+::)*(Ecs)?ShadowRoot>|\bis_shadow_root\(' c0578223 -- crates ':!*tests*' | wc -l` → 63.
- P-REL: `git grep -n -P '\.(parent|first_child|last_child|next_sibling|prev_sibling)\b(?!\s*\()' c0578223 -- crates/core/elidex-ecs/src ':!*tests*' | wc -l` → 41.
- P-HOST, P-HOSTFN: `'\.host\b(?!\s*\()'` → 41; `'\bshadow_host\('` → 2.
- Field visibility: `git grep -n -A6 'pub struct TreeRelation' c0578223 -- crates/core/elidex-ecs/src/components.rs`
  (five `pub(crate)` link fields).
- Not measured: anything at runtime (§3.1); the per-call relation (census C-7).

## §9. Citation table

Each row was looked up with `.claude/tools/webref` on 2026-11-15 (`dfn` for the number ↔ title pair, `body` for the
quoted words).

| ID | Citation | Anchor | Used by |
|---|---|---|---|
| [D1] | WHATWG DOM §1.1 Trees — *parent*, *root*, *inclusive ancestor*, *index* | `dom#concept-tree-parent` | §1, §3 |
| [D2] | WHATWG DOM §4.2.2 Shadow tree — "shadow tree is a node tree whose root is a shadow root"; "A shadow root is always attached to another node tree through its host"; "A node is connected if its shadow-including root is a document" | `dom#concept-shadow-tree`, `dom#connected` | §1, §2, §3, §4 |
| [D3] | WHATWG DOM §4.7 Interface DocumentFragment — "A DocumentFragment node has an associated host (null or an element in a different node tree)"; *host-including inclusive ancestor* | `dom#concept-documentfragment-host`, `dom#concept-tree-host-including-inclusive-ancestor` | §1.1, §3, §4 |
| [D4] | WHATWG DOM §4.8 Interface ShadowRoot — *shadow-including root* ("its root's host's shadow-including root, if the object's root is a shadow root; otherwise its root"); *retarget* step 2 ("Set A to A's root's host"); "A shadow root's get the parent algorithm, given an event, returns null if event's composed flag is unset and shadow root is the root of event's path's first event path item's invocation target; otherwise shadow root's host" | `dom#concept-shadow-including-root`, `dom#retarget`, `dom#interface-shadowroot` | §1.1, §1.4, §3 |
| [D5] | WHATWG DOM §4.2.3 Mutation algorithms — *ensure pre-insert validity* step 2 ("If node is a host-including inclusive ancestor of parent, then throw a \"HierarchyRequestError\" DOMException") | `dom#concept-node-ensure-pre-insertion-validity` | §1.1, §2, §3 |
| [D6] | WHATWG DOM §4.3.2 Queuing a mutation record — step 2 ("Let nodes be the inclusive ancestors of target") | `dom#queue-a-mutation-record` | §2, §3 |
| [D7] | WHATWG DOM §4.4 Interface Node — "The parentNode getter steps are to return this's parent"; "A node's get the parent algorithm, given an event, returns the node's assigned slot, if node is assigned; otherwise node's parent" | `dom#dom-node-parentnode`, `dom#interface-node` | §1.4, §2, §3 |
| [D8] | WHATWG DOM §5.6 Interface Range — "The root of a live range is the root of its start node"; *get the common ancestor* step 2 | `dom#concept-range-root`, `dom#get-the-common-ancestor` | §2, §3 |
| [H1] | WHATWG HTML §4.12.3 The template element — "The template contents of a template element are not children of the element itself"; establishing step 2 ("Create a DocumentFragment object whose node document is document and host is the template element") | `html#template-contents` | §3, §4 |
| [H2] | WHATWG HTML §4.10.18.3 Association of controls and forms — *reset the form owner* step 4 | `html#reset-the-form-owner` | §2, §3 |
| [H3] | WHATWG HTML §4.10.15 The fieldset element — *disabled fieldset* | `html#concept-fieldset-disabled` | §2, §3 |
| [H4] | WHATWG HTML §4.10.10 The option element — *nearest ancestor select* step 2 | `html#option-element-nearest-ancestor-select` | §2, §3 |
| [H5] | WHATWG HTML §4.10.22.3 Form submission algorithm — "Let subject be form's nearest ancestor dialog element" | `html#form-submission-algorithm` | §3 |
| [H6] | WHATWG HTML §3.2.6.2 The lang and xml:lang attributes — "If the node's parent is a shadow root: Use the language of that shadow root's host" | `html#language` | §2, §3 |
| [H7] | WHATWG HTML §3.2.6.4 The dir attribute — *parent directionality* step 2 | `html#parent-directionality` | §2, §3 |
| [S1] | CSS Selectors L4 §14.1 Descendant combinator ( ) | `selectors-4#descendant-combinator` | §2, §3 |
| [C1] | CSS Shadow Module Level 1 (css-shadow-1) §4.1 Flattening the DOM into an Element Tree — *flat tree* | `css-shadow-1#flat-tree` | §1.4 |
