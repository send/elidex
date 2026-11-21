# Tree model: a shadow root is a tree root; every climb chooses whether it crosses — program umbrella

Status: **umbrella, draft 3.5** (pre-implementation; ratified (PM 848, #648 `4918a6c7`)). Draft 1 was plan-reviewed (R1: CRIT 1 / IMP 21 / MIN 24) and
draft 2 answered it; draft 2 was plan-reviewed (R2: CRIT 0 / IMP 7 / MIN 27), and this draft answers R2 (fix
decisions F1 to F7) and writes in PM 758, 759, 761, 765, 768 and 794. Draft 3.1 closes Q-19 (PM 800; §1.6) and
Q-18 (PM 800; §2 CHILD × SHELL) and records the registrations of PM 790 and PM 794 (§7.1). Draft 3.2 moves
pre-remove step 1 out of `apply_remove_child` onto the `removeChild` path only (§1.6, Q-19). Draft 3.3 answers the
R3 plan-review of draft 3.2 (CRIT 0 / IMP 4 / MIN 23 / FP 1; the driver's fix decisions F1 to F4 and the MIN list):
`ta` gains a second declared behaviour change, Range's contained-node walk stops entering a shadow tree, which closes
the second script path of §0.1 item 6 (§1.6, F1); `sibling_order` stays a stored-order read until `ts`, and `tc`
declares the new storage position of its three engine insertions (§1.3, §2 ORD × RENDER, F2); the `shadow.rs` cite
targets of `ts` are corrected (§5.3, F3); the ledger quotes of §7.4 are re-read from the current rows (F4); `tc`'s
Tab-order descent skips a host whose tabindex is negative (§2 CHILD × SHELL). Draft 3.4 answers the focused
re-check of draft 3.3 (CRIT 0 / IMP 5 / MIN 13; the driver's decisions D1 to D3 and the MIN list) and PM 820:
Range's contained-node walk leaves `ta` for a new slice **`tq`** before it (tp → tq → ta, PM 820), so `ta` has one
declared behaviour change again and `#11-shadow-root-accepted-as-ref-or-old-child` closes in two steps (§1.6, D3);
`::before` generation stays a stored-order write until `ts`, which keeps it before the shadow tree (§1.3, §2 ORD ×
RENDER, D1, css-pseudo-4 §4.1 [P1]); `toString` is a named facet of the walk's change with its own probe (§1.6,
D2); PM 812 and PM 814 are recorded. Draft 3.5 applies the decided findings of the focused re-check of draft 3.4:
`::before` generation's stored-order write moves into one elidex-ecs method, `EcsDom::insert_as_first_stored_child`
(NEW: `tc`, population P-REL, deleted in `ts`), because outside elidex-ecs no raw child accessor exists after `tc`
and `read_rel` is `pub(super)` (§1.3, §1.6, §4, §5 `tc`, §5.3); (N7) is recorded as applied under PM 826 (§7.4); the
`toString` overshoot is "not fixed", its output differing only where the walk passes a host (§1.6, §3.1, §5);
the css-pseudo-4 row cites css-shadow-1 §4.1's flat-tree note for a host's actual content [C1] (§3); the P2
observation on `assignedNodes()` is cited as an instance of the ledger row's P2-1 / K4 (§7.1, §7.2, §7.4 (N8));
`tq`'s test files are all under 1000 lines (§5 `tq`, §8). Draft 3.5 is written on `2ac9fa44` (origin/main,
2026-11-18): `git diff --name-only b46a030c..2ac9fa44 -- crates` lists 5 elidex-js files (#642), none cited
here, so every anchor and count holds unchanged. Its changes are structural (a new slice, a declared change moved, child-side
naming), so `/elidex-plan-review` runs in full again on this umbrella, and on each per-PR plan before its code
(CLAUDE.md, *Edge-dense work*). Owner: L3. Base: `bfc63658`; every code anchor is at that commit. Draft 3.4 was
written on `b46a030c` (origin/main, 2026-11-18): `git diff --name-only bfc63658..b46a030c -- crates` prints nothing
(the one commit between them, #643, is docs-only), so every anchor and count holds there unchanged. Drafts 3 to 3.2 were written at `84fee8fa`: `git diff --name-only
84fee8fa..bfc63658 -- crates` lists 17 files (verified 2026-11-18; the VM's GC rooting and coercion files, the
event-handler applicability files of elidex-script-session and their tests), none of them cited here, and every
census count was re-measured at `bfc63658` with the same result (§8). Drafts 1 and 2 were written at `c0578223`
(`git diff --name-only c0578223..84fee8fa -- crates`: 10 files, none cited; verified 2026-11-18). Census data:
[2026-11-tree-model-census.md](2026-11-tree-model-census.md) (commands, counts, per-file tables, scripts, spec
lookups; §6 here says what each population is for).

**Changelog.** *Erratum: tsel before tq (PM 884, PM 891)* (2026-11-21; no rev bump; carried by the tsel plan PR, branch `tree-model-tsel-plan`). Status ratified (PM 848, #648 `4918a6c7`); §5.1 slice order tsel before tq and the "`tq` first" sentence; §5 `tq` row Deps.

**Rulings this memo implements (decided, not reopened here).**

- **PM 730**: (X) is canonical (a shadow root is a tree root; the host relation is held only by the link pair); a
  separate Edge-dense program placed before PR (io): is → ig → [this program] → io → ii (ids of the
  ecs-entity-lifetime program); the direction (every climb chooses, then the storage changes); a census defines its
  population by a property, not by a name.
- **PM 747**: Q-2 the Range contents program's RC-1 lands after `tc`, drops its own D13 part and uses `tc`'s
  accessors. Q-3 the slot `#11-shadow-root-is-a-tree-root` is registered. Q-5 a probe that misses is reported at
  that plan's review; merging a slice into a neighbour is the PM's call. Q-6 `tf` also flips the form-owner
  ancestor climb. Q-9 `ts` is not pulled earlier; `ta`'s plan measures the selector and style paths. Q-10 was
  answered by Layout (§7.2).
- **PM 748**: (a) `#11-template-contents-host-link` closes at a small PR after PR (io) (amends PM 718). (b) VM
  natives choose the elidex-ecs relation directly; the dom-api navigation handlers take the same relation in `ta`,
  mechanically; `te`'s DoD says no handler remains a caller of the transitional accessor; connecting or deleting the
  handlers is `#11-dom-api-handlers-unreachable-from-production`'s (registered). (c) the selector relation is not
  defined by this program.
- **PM 740 / 741 / 745** (timing): §7.2. PM 745's "PR-1" is slice `ta`; its "PR-2" is the slices `tc` to `ts`,
  split by cluster as §5 decides.
- **PM 758**: Q-11 L3 writes the trip-wire in `ta`'s PR; its place follows the `tools` job comment of
  `.github/workflows/ci.yml`, and CI/tools checks it at review. Q-14 the `isConnected` behaviour fix is a small PR
  after `ta`. Q-15 L3 cuts the touch-time splits as standalone prerequisite PRs; Layout confirms the seam of its
  file. Q-16 `#11-range-compare-points-shadow-index` closes at `tc`.
- **PM 759**: Q-12 the shell's link and label lookups belong to P1 `#11-element-lifecycle-hooks`.
- **PM 761**: Q-17 the DOM-walker program's slice 1 lands after `ta` (is → ig → tp → ta → walker slice 1; it may run
  beside `tc` and later slices); PM 694 stands (it stacks on PR (i) and uses `owned_entities`). L3 sets the dates,
  and tells the PM if they would pass the walker slot's Re-eval of 12-15 (§7.1).
- **PM 765 (2)**: `#11-shadow-mutation-observer-disconnected-cleanup` was re-triaged (§7.1).
- **PM 794**: the selector relation of §1.5 is owned by `#11-shadow-tree-style-isolation` (Layout, CV-details); no
  new slot. `#11-dom-selector-api-rejects-shadow-pseudos` is registered as a neighbour (§7.1).
- **PM 790**: (1) `#11-shadow-root-accepted-as-ref-or-old-child` is registered (§7.1); (3) the schedule: the
  umbrella's ratify ~11-23, `ta`'s landing ~12-01, PR (io) ~12-18, T3 ~12-22 or later, inside T3-pre's no-op cap of
  12-31; G9 (R-NAT's window and L3's hands) is reported by L3 at `ta`'s landing and decided by the PM (§7.2);
  (4) `#11-shadow-mutation-observer-disconnected-cleanup` is absorbed by the ecs-entity-lifetime program's PR (ii)
  (§7.1, §7.2).
- **PM 791**: `#11-fieldset-disabled-dynamic-insert` is closed by the reconciliation umbrella's Slice 2, paydown (c);
  Re-eval "reconciliation Slice 1's landing + Slice 2's plan-review", Re-eval-date 2027-01-31 (§7.1).
- **PM 800**: Q-19 option (b′) (§1.6): elidex-ecs's `is_child_of` stays storage maintenance until `ts`; the spec's
  child-of steps live in the mutation layer from `ta`. Q-18 (§2 CHILD × SHELL): the Tab order `tc` produces is a
  change toward HTML's order, not an interim regression.
- **PM 812**: `ta` gained a second declared change (Range's contained-node walk) and the scope of
  `#11-shadow-root-accepted-as-ref-or-old-child` widened to "script paths that unlink a host's shadow root" (the
  ledger row says "scope widened PM 812"). Draft 3.4 moves that change into `tq` (PM 820, below); the scope widening
  stands.
- **PM 814**: the ledger row of `#11-shadow-root-accepted-as-ref-or-old-child` was re-synced to draft 3.3 (the
  former landing item N1, §7.4: applied).
- **PM 820**: slice `tq` before `ta` (tp → tq → ta) is approved; the slot closes in two steps, the Range path at
  `tq` and the reference / old / removed child at `ta`, and closes at `ta` (§1.6, §5, §7.1, §7.4).
- **Layout's answer to Q-13**: shadow-including order for paint and hit-test is acceptable until P2 lands; no other
  rule (§2 ORD × RENDER).

## §0. Charter

### §0.1 Defects

1. **The backward fact of the host relation has two homes.** `attach_shadow_with_init` spawns the shadow root with
   `ShadowRoot { host, .. }` and then appends it to the host's child list
   (`crates/core/elidex-ecs/src/dom/shadow.rs:195-213`: `self.append_child(host, shadow_root_entity)` at `:210`), so
   "which element hosts this root" is stored in `ShadowRoot.host` **and** in `TreeRelation.parent`. The forward
   fact has two homes as well: `ShadowHost.shadow_root` and the host's child list.
2. **The code is split between two models**, and each side is wrong about the storage or about the spec:
   - *(X)-assuming, over (Y) storage*: `rev_version`, `is_host_including_ancestor_or_self`,
     `find_tree_root_composed` and `walk_parent` read the tree parent first and the `.host` branch second
     (`crates/core/elidex-ecs/src/dom/mod.rs:743-761`, `crates/core/elidex-ecs/src/dom/tree/walkers.rs:380-392`,
     `:436-439`, `crates/core/elidex-ecs/src/dom/tree/mutation.rs:418-421`). For an attached shadow root the stored
     parent is the host, so the `.host` branch is never entered: right by accident, and the documented branch dead.
   - *(Y)-assuming*: `is_connected`'s doc (`walkers.rs:226-228`), `root_entities`' doc
     (`crates/core/elidex-ecs/src/dom/tree/navigation.rs:359-360`), the VM's `parentNode` native
     (`crates/script/elidex-js/src/vm/host/node_proto.rs:323-326`), dom-api's `find_root`
     (`crates/dom/elidex-dom-api/src/node_methods/mod.rs:64-75`).
   - *Compensation added after the fact*: the strict parser's form-pointer walk
     (`crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs:303-310`), `ChildrenIter` / `ChildrenIterRev`
     (`dom/mod.rs:910-913`, `:944`).
3. **Under (Y) the unsafe direction is the default.** A climb that says nothing (`while let Some(p) =
   dom.get_parent(cur)`) crosses the shadow boundary; DOM's default relations (*parent*, *ancestor*, *root*) do not
   (§3). Readers predicted to cross against the spec (census C-2, class "P:"; **read from the code, not run**):
   MutationObserver's ancestor walks, Range and traversal, form association, contenteditable inheritance.
4. **Child-side compensation is per accessor and per caller.** Six accessors skip a shadow-root child (census
   C-4); the four raw accessors (`navigation.rs:22-42`) do not, and 108 non-test lines call them (census P-CH). The
   Range contents program already lists one resulting defect (D13,
   `docs/plans/2026-11-range-contents-program.md:731`).
5. **Derived relations are multiplied and unlabelled.** 60 read-only functions climb and 45 more call one of them
   (census C-6, by property). Among them are three roots and four ancestor tests; `is_ancestor_or_self` crosses,
   has a name that says it does not, and is called on 16 lines that need three different relations (census C-6,
   `git grep -n -P '\bis_ancestor_or_self\(' bfc63658 -- crates ':!*tests*'`).
6. **A script can hand the host's own shadow root to the host as a child, and the engine accepts it.** The "is this
   node a child of parent" checks read the stored parent, which for a shadow root is the host. Run at `84fee8fa`
   (census C-14): `host.removeChild(host.shadowRoot)` returns without throwing and takes the shadow root out of the
   host's child list while both links stay (`sr.host === host`, `host.shadowRoot === sr`, and the shadow content
   still reports `isConnected`); `host.insertBefore(n, sr)` and `host.replaceChild(n, sr)` succeed too. DOM requires
   "NotFoundError" in all three ([D13]): a shadow root's parent is null. **A second script path unlinks it without
   naming it**: Range's contained-node walk reads raw children, so it counts a host's shadow root as contained. Run
   by the driver at `bfc63658` (census C-14, second row): with a **closed** shadow root, `range.setStart(h, 0);
   range.setEnd(h.parentNode, idx(h) + 1); range.deleteContents()` does not throw, the shadow root is removed from
   the host's child list while the links stay (the shadow content still reports `isConnected`), and a
   `MutationObserver` on the host receives a record whose `removedNodes` holds the closed shadow root, so page
   script gets a reference that `mode: "closed"` withholds. DOM does not reach the removal at all: a node is
   *contained* in a range only if its root is the range's root ([D8]), and a shadow root's root is itself. Both
   paths are wrong in production today, with no error raised. The second path closes at `tq` and the first at `ta`
   (PM 820; §1.6).

### §0.2 Why a program, and why before PR (io)

More than three invariant axes intersect (§2), and the population that breaks is not enumerated by the compiler:
under a storage change alone, a crossing climb that relies on the stored parent keeps compiling and silently stops
at the boundary. That is the *Edge-dense* trigger; the slices of §5 are its base case. (`ta` is a base case and
not a program of its own. It applies two invariants: "every climb names its relation" (its population is the
compiler's error list) and "the spec's child-of steps ask the tree-parent question" (the five T2 lines and one new
pre-remove check, §1.6). Its §2 rows are pinned by existing cells except the one declared change of §1.6. Cutting it
by crate would leave `get_parent` beside the new names between the cuts. `tq` is a base case too: one function's
two child steps, §1.6.) PR (io)'s central invariant
is "each link's backward fact has one home"; for the shadow root that sentence is false at base (defect 1), and the
PR (io) plan itself leans on the tree parent (its §2 EDGE × adopt row). Landing PR (io) first would put a new seam
beside the old representation, the coexistence *One issue, one way* forbids.

## §1. The rule (the single home; every other site refers here)

> **R-TREE.**
> 1. **The tree relation holds tree children only.** `TreeRelation` is the node tree of DOM §1.1 / §4.2: an
>    entity's stored parent is its DOM *parent*, and a child list contains its DOM *children* and nothing else. A
>    shadow root has no parent and is in no child list, and elidex-ecs refuses to link one (§2 TREE × WR).
> 2. **The host relation is held only by the link pair**: `ShadowHost.shadow_root` (forward, read through
>    `get_shadow_root`) and the shadow root's host (backward). Every climb, and every reader outside elidex-ecs,
>    reads the backward fact through `shadow_host`; the readers inside elidex-ecs that hold the component for
>    another reason are a closed list (census C-5).
> 3. **Every upward relation is named.** No accessor is called "parent" without a qualifier, and a function's
>    name states the relation it computes. **A change of relation is a change of name**, so the compiler lists the
>    callers whenever a relation changes.
> 4. **Every child-side read is named too.** A reader takes *tree children* (DOM §1.1), or asks for the shadow
>    tree (`get_shadow_root`, then that root's tree children), or takes the flat tree's children
>    (`composed_children`). No accessor returns "the children" without saying which.

### §1.1 The steps

| Accessor | Relation | From `ta` to `ts` (over (Y) storage) | After `ts` |
|---|---|---|---|
| `EcsDom::tree_parent` (NEW) | DOM §1.1 *parent* | `None` for a shadow-root receiver (a marker test), else the stored parent | the stored parent |
| `EcsDom::shadow_including_parent` (NEW) | one step of DOM §4.8 *shadow-including ancestor* | the stored parent | `tree_parent(e)`, else `shadow_host(e)` |
| `EcsDom::get_parent` | — | **deleted** in `ta`; no alias, no deprecated wrapper, tests included | — |

- **Two names, not one accessor with a mode argument.** The two relations are the two DOM names; no caller passes
  the choice as a value, and an enum argument would add a branch to the selector and style loops. (Open to
  plan-review as a symmetry question against PR (io)'s `WalkScope`, §7.3 Q-8.)
- **The fail-safe direction.** With `get_parent` gone there is no unmarked step. A reader that picks the shorter
  name gets DOM's default relation and stops at the boundary.
- **Storage maintenance is not a climb.** Inside the `dom::tree` module, the code that keeps the links (detach,
  teardown, the child index) reads the stored link itself (population P-REL), not a named relation. Four P-UP lines
  become such reads in `ta` (census C-2, "raw"), and `ts` re-reads each. The three mutators' own check
  (`is_child_of`) is storage maintenance too until `ts` (PM 800). The spec's question about a node a script
  supplied ("is child's parent parent?") is not maintenance: it is asked in the engine-independent layer, one
  predicate per spec step (§1.6). `sibling_order` is a stored-order read of this kind until `ts`, and `::before` generation a stored-order
write (§1.3).
- **The interim cost** is one component lookup per `tree_parent` step, and one per child-side step from `tc`
  (§1.3). `ta`'s plan benchmarks the selector and style paths at its head against its base and escalates a
  regression beyond noise to the PM with the numbers (PM 747 Q-9).

### §1.2 Derived relations (by property)

A **derived relation** is a read-only function whose body reads a parent link, or calls such a function (census
C-6: the property, the script, the counts and a disposition for every member). R-TREE 3 applies to each. In `ta`
every member is **removed, renamed, or kept by a closed rule**:

- **One function per relation**, built from the two steps. Tree: the root (`find_tree_root`, kept), the inclusive
  ancestor test `EcsDom::is_tree_inclusive_ancestor` (NEW), the order `EcsDom::tree_order` (NEW).
  Shadow-including: the root `EcsDom::shadow_including_root` (NEW), the inclusive ancestor test
  `EcsDom::is_shadow_including_inclusive_ancestor` (NEW), *connected* (`is_connected`, kept). Host-including (DOM
  §4.7): one test, `EcsDom::is_host_including_inclusive_ancestor` (NEW). The spellings are `ta`'s plan's call; the
  qualifier is not.
- **Removed**: `is_ancestor`, `is_ancestor_or_self`, `walk_parent`, `ancestor_chain`, `tree_order_cmp`,
  `shadow_root_entity_for_slot`, dom-api's `find_root`. **Renamed**: `find_tree_root_composed`,
  `is_light_tree_ancestor_or_self`, `is_host_including_ancestor_or_self`. Every caller of these fails to compile
  and picks one of the functions above (or a pending name, §1.4, §1.5). In particular the 16 call lines of
  `is_ancestor_or_self` split into host-including 6, shadow-including 1, tree 1 and pending 8 (census C-6).
- **Kept** only when the name already states the relation the body computes at base, and no slice changes that
  relation: `find_tree_root`, `is_connected`, `root_entities`, and the single-hop or guarded helpers of census C-6
  (20 members, each listed with its reason). A member on no list is renamed: the unknown case falls on the
  enumerated side.
- **Continuation.** The census measures levels 1 and 2. A function whose body uses a removed or renamed name is
  reported by the compiler; `ta`'s plan lists each one, and renames it too when its callers do not all take one
  relation. The recursion ends at functions that mutate or implement one named spec algorithm.
- **What a new owned kind can widen.** `tree_parent` reads no link and `shadow_including_parent` reads exactly one.
  The host-including test is the only function that crosses every DocumentFragment host; after PR (io)'s follow-up
  adds the template contents' host (§7.2), it widens, and its callers after `ta` are exactly the *ensure pre-insert
  validity* sites: the three cycle guards in elidex-ecs, `child_node/mod.rs:260`, `:445`, and the six former
  `is_ancestor_or_self` lines (`git grep -n -P '\bis_host_including_ancestor_or_self\(' bfc63658 -- crates ':!*tests*'`
  for the base list). `focus/delegate.rs:88`, which uses that function for a shadow-including question today, moves
  to the shadow-including test.
- **Not introduced here: an ancestor iterator.** The climb loops keep their shape and their depth bound; this
  program changes which step each loop takes.

### §1.3 The child side

R-TREE 4 is applied in `tc` with the device of `ta`: **every child-side accessor is replaced by one set under new
names**, so every reader fails to compile in `tc` and is listed in its plan with the relation it takes.

- **What is renamed: all of it.** The four raw accessors, `next_exposed_sibling` / `prev_exposed_sibling`, the two
  iterators (`children_iter`, `children_iter_rev`), `children`, `child_list_uncapped`, `index_in_parent`, and the
  element helpers built on them (census C-4, C-13). Chosen over renaming only the raw four: a reader of
  `children_iter` has also never said which relation it wants, and a partial rename would leave the compiler
  listing only the readers that are wrong today. The scope is 314 non-test lines (§8; `.children(` over-counts).
- **The one set** reads the tree children: first child, last child, next sibling, previous sibling, the two
  iterators, the list, the uncapped list, the index. From `tc` to `ts` it holds the interim skip in the accessor
  layer, in two halves: a shadow-root **result** is skipped, and a shadow-root **receiver** has no siblings and no
  index (the guard the VM natives carry today, census C-5 role R, so `shadowRoot.nextSibling` is null at every
  head). `ts` deletes both halves with the storage.
- **One child-side reader takes the stored order, not the set: `sibling_order`** (`walkers.rs:308-324`, the helper
  of `tree_order_cmp`, which `ta` splits into `tree_order` and `pending_flat_tree_order`, §1.2, §2 ORD × RENDER).
  It walks the raw next-sibling chain to order two siblings of the stored list, and one of them can be a shadow
  root (the chain element of a node inside a shadow tree). It is a stored-order read until `ts`, like the
  maintenance code of §1.1 (population P-REL): `tc`'s rename puts its line on the compiler's list, and it takes the
  stored link (`read_rel`), not the one set. Reason: under the set's receiver half a shadow root has no siblings, so
  `sibling_order(sr, x)` and `sibling_order(x, sr)` would both answer "after", and the comparator that
  `walk.rs:620` and `hit_test.rs:235` pass to `sort_by` would not be a total order. That is a crash risk, not only
  a wrong order: the standard library's `sort_by` documents "May panic if `compare` does not implement a total
  order" (its `# Panics` section, rustc 1.99.1), and paint and hit-test run it on every frame and click. The
  flat-tree order it serves is P2's (§1.5); at `ts` the shadow root leaves the list and `pending_flat_tree_order`
  says where the shadow tree sits.
- **One child-side writer keeps the stored position too: `::before` generation** (`crates/css/elidex-style/src/pseudo.rs:72-74`:
  the raw first child at `:72`, `insert_before` at `:74`). It inserts the generated node before the originating
  element's first **stored** child, so on a host whose shadow root is stored first the node lands before the shadow
  root. That is already css-pseudo-4 §4.1's order: `::before` is "a styleable child pseudo-element immediately
  before the originating element's actual content" [P1], and a host's actual content is its shadow tree (the flat
  tree fills a shadow host "with their shadow tree children", css-shadow-1 §4.1's note [C1]). Layer 6
  reads the stored order until `ts` (above), so base paints and hit-tests a positioned `::before` before a
  positioned node of the shadow tree, as the spec places it. `tc`'s rename puts `:72` on the compiler's list. It
  does not take the one set: taking the first tree child would store the node after the shadow root and move it
  behind the shadow tree in layer 6 (draft 3.3 declared that move; D1 withdraws it). It cannot take the stored link
  itself either, unlike `sibling_order`: after `tc` no raw child accessor exists outside elidex-ecs, and `read_rel`
  is `pub(super)` (`crates/core/elidex-ecs/src/dom/tree/mod.rs:64`), so elidex-style has no way to read it (draft
  3.5). **So `tc` adds one elidex-ecs method that performs the write itself: `EcsDom::insert_as_first_stored_child`
  (NEW: `tc`)**, in `crates/core/elidex-ecs/src/dom/tree/mutation.rs` beside `insert_before` and `append_child`. It
  reads the parent's stored first child (`read_rel`) and inserts before it, or appends when there is none (the two
  branches `pseudo.rs:72-76` holds today) by delegating to `insert_before` / `append_child` themselves, so `adopt_for_insert`, `rev_version(parent)` and the `MutationEvent::Insert` dispatch stay exactly as today (a re-implementation over `link_node` would silently drop the version and the event), and `pseudo.rs` calls it for `::before` in place of those lines. The test fixture that copies those lines (`crates/css/elidex-style/src/generated_content.rs`, `p_with_before`, census C-3) calls the same method from `tc` and goes back with `pseudo.rs` at `ts`, so fixture and production keep one shape. It is
  storage maintenance inside `dom::tree`, so it belongs to population P-REL (census C-1, C-9), not to the child-side
  accessors: it returns no child, so it is not a fourth child-side reader name (§4's rejection holds), and the
  storage write stays in elidex-ecs (Layering). Fail-safe: it does not depend on the interim skip, so a host whose
  shadow root is stored first still gets its `::before`. It is a stored-order write until `ts`. At `ts` the shadow
  root leaves the list, so the first stored child is the first tree child and the method is the ordinary "insert
  before the first child, else append". **`ts` deletes it** (its P-REL line is re-read at `ts`'s head, §5.3): `pseudo.rs`
  goes back to the one set's first child and `insert_before` / `append_child`, so no stored-versus-tree name
  survives the storage. `pending_flat_tree_order` places the shadow tree just after its host, which would put the
  generated node, now an ordinary light child, after the shadow tree; `ts` keeps it first (§2 ORD × RENDER, §5.3
  `ts`).
- **A reader that wants the shadow tree asks for it**: `get_shadow_root(host)`, then that root's tree children.
  `tc` adds no "shadow-including children" accessor: DOM defines shadow-including *descendants* and *tree order*
  ([D4]), not children, and the two-step form is what the downward walks of `walkers.rs` already do. A reader
  that wants flat-tree children uses `composed_children`, which `tc` does not change.
- **Readers that cross downward today and must keep doing so.** At base only a raw accessor can return a shadow
  root as a child, so a walk built on the raw accessors enters a shadow tree without asking. For some walks that
  is the defect (§3.1 `tc`); for others it is what makes them correct, and with the one set they would silently
  stop at the host. The population is "a function that reads a raw first or last child and a raw sibling" (census
  C-13: 25 functions, 23 outside elidex-ecs), and the ones read as needing the descent are three:
  `collect_focusable_entities` (the shell's Tab order), `collect_iframe_entities` (the shell's iframe discovery)
  and `save_descendant_styles` (Layout's style save around a layout probe). `tc` moves each to the explicit
  two-step descent in the same PR. Its plan re-reads all 23 and probes each one it leaves on tree children with a
  shadow tree below the walk's root (§5.2).

### §1.4 The transitional accessor (declared, bounded by the program)

Slice `ta` must be behaviour-preserving (apart from §1.6), and some callers' spec relation is the tree parent
while their base behaviour crosses. Such a caller cannot take `tree_parent` in `ta` without changing behaviour, and
`shadow_including_parent` would record a wrong relation as chosen. It takes
`EcsDom::parent_crossing_pending_flip(e, Flip)` (NEW), which behaves as the deleted `get_parent`.

- **`Flip` (NEW) is an enum with one variant per flip slice**: `Flip::Tm` (NEW), `Flip::Tr` (NEW), `Flip::Tf`
  (NEW), `Flip::Te` (NEW). Each caller names the slice
  that flips it, so no pending line is without a slice. A flip slice deletes its variant: the compiler lists
  exactly that slice's backlog, and the slice cannot land with a line left behind.
- **Derived relations that are pending** carry the suffix `_pending_flip` from `ta` and lose it in their flip slice
  (17 functions at level 1, census C-6); the same holds for the crossing ancestor test and the crossing order that
  the pending callers of `is_ancestor_or_self` and `tree_order_cmp` take.
- **No new caller, mechanically.** `ta` lands a trip-wire with an allowlist (file, pending name, count), in the
  form of `.claude/tools/layout-box-reader-allowlist.tsv`: it fails when a count rises or a file that is not listed
  uses a pending name. It covers the names of this section and of §1.5. L3 writes it in `ta`'s PR, and its job
  follows the `tools` job comment of `.github/workflows/ci.yml` (a wire that declares no interpreter goes to
  `trip-wires`); CI/tools checks its form at review (PM 758 Q-11).
- **What the wire does not see**: within one file, one pending caller removed and another added under the same
  name. That is accepted. The wire's job is that the backlog does not grow and does not spread to a new file; the
  backlog itself is never read from the allowlist, because the slice that removes a name deletes it and the
  compiler lists the lines that exist then. A swapped-in line also shows the pending name in its PR's diff.
- The accessor and `Flip` are deleted by the last flip slice; `ts` requires that they do not exist.

This is a strangler state inside the program, declared with its removal condition; the alternatives are in §4.

### §1.5 Relations this program does not define

Three upward relations are not DOM's tree or shadow-including relations. Their readers climb the stored parent at
base. Taking `shadow_including_parent` or `tree_parent` for them would record a relation nobody chose, so in `ta`
each takes **its own pending name** (four names for the three relations), with base behaviour, owned outside this
program (census C-2 classes F, E, L):

| Relation | Spec | Pending name (body) | Readers at base | Owner |
|---|---|---|---|---|
| **Flat-tree parent** and flat-tree order | css-shadow-1 §4.1 [C1]; CSSOM View `offsetParent` ("the parent of the element in the flat tree") [V1]; HTML §6.3 inert ("flat tree descendants") [H10]; Selectors `:hover` [S2] | `EcsDom::pending_flat_tree_parent` (NEW: the shadow-including step); `EcsDom::pending_flat_tree_order` (NEW: §2 ORD × RENDER) | 21 lines: render, layout root tests, CSSOM View queries, the hidden-subtree test, a11y, shell hover and scroll (census C-2 "F", verified 2026-11-18) | `#11-shadow-slot-assignment-flat-tree` (P2, Layout) |
| **Event *get the parent*** | DOM §2.7 dfn; a node's: "returns the node's assigned slot, if node is assigned; otherwise node's parent" [D7]; a shadow root's: "returns null if event's composed flag is unset and shadow root is the root of event's path's first event path item's invocation target; otherwise shadow root's host" [D4]; a document's: "returns null if event's type attribute value is \"load\" or document does not have a browsing context; otherwise the document's relevant global object" [D11] | `EcsDom::pending_event_parent` (NEW: the shadow-including step). The `!composed` break at `event_dispatch/mod.rs:214` is the shadow root's null arm and stays | 4 lines: `build_propagation_path`, the shell's link and label lookups (census C-2 "E", verified 2026-11-18) | `#11-event-path-window-missing` (L3) owns the function and its Document, ShadowRoot and Node arms (`docs/plans/2026-11-event-path-window.md` §0.4 Decisions, D-1); the *assigned slot* arm inside the Node arm is P2's. The shell's two lookups approximate the activation target of DOM §2.9 and belong to P1 `#11-element-lifecycle-hooks` (PM 759) |
| **The selector tree's parent** | css-shadow-1 §3.2.2: "a shadow host also appears in its shadow tree, with the contents of the shadow tree treated as its children" [C2]; §3.2.1 [C3] | `EcsDom::pending_selector_parent` (NEW: the tree parent; the climb stops at the shadow root, as at base) | 7 lines in `selector/matching.rs` and `selector/traverse.rs` (census C-2 "L", verified 2026-11-18) | `#11-shadow-tree-style-isolation` (Layout, CV-details; PM 794). Its text already holds "`:host` as a non-subject compound in the shared combinator walk" (`docs/plans/2026-10-content-visibility.md:897`). No new slot |

These names outlive the program. Each owner replaces its name by the relation it defines; the trip-wire of §1.4
keeps the three populations from growing until then. `ta`'s landing list gives each owner slot the names it
receives (§7.4), so no backlog is recorded only in this memo.

### §1.6 The script paths that unlink a shadow root (the declared behaviour changes of `tq` and `ta`)

Two script paths reach a host's shadow root as if it were a child (§0.1 item 6). The first asks "is `child` a
child of `parent`?" about a node the script supplied; the second never asks, because a walk hands the shadow root
over as a contained node. Each closes in the algorithm the spec puts the step in: the second at `tq`, the first at
`ta` (PM 820).

**The first path: the child-of question.** "Is `child` a child of `parent`?" is the tree-parent question:
`tree_parent(child) == Some(parent)`. It is asked about a node that a **script supplied** (the reference child of
`insertBefore`, the old child of `replaceChild`, the child of `removeChild`), so the node can be a shadow root, and
at base the answer for a host and its own shadow root is "yes".

- **Decided (R2, F1; placement by PM 800, Q-19 (b′)):** in `ta` the spec's child-of steps ask the tree-parent
  question, each in the engine-independent algorithm it belongs to, where pre-insert and replace validity already
  live:
  - *ensure pre-insert validity* step 3 and *replace* step 1 ([D13]): the five lines outside elidex-ecs that ask it
    by hand take `tree_parent` (census C-2, class "T2": `child_node/mod.rs:268`, `:451`, `element/tree.rs:158`,
    `mutation/child_list.rs:284`, `:439`).
  - *pre-remove* step 1 ([D13]: "If child's parent is not parent, then throw a \"NotFoundError\" DOMException."):
    one new check, `tree_parent(child) == Some(parent)`, on the `removeChild` path only. *Pre-remove* is run by
    `removeChild` alone; the shared *remove* building block `apply_remove_child`
    (`crates/script/elidex-script-session/src/mutation/child_list.rs:356`) is also called by algorithms that run
    *remove* without *pre-remove*, so the check does not go there. Its home is a new validity function in
    `crates/dom/elidex-dom-api/src/child_node/mod.rs` (`ensure_pre_remove_validity`, NEW; the name follows its
    siblings), which the `RemoveChild` handler's `invoke` (`element/tree.rs:199`) calls before
    `apply_remove_child` and which returns "NotFoundError". Reason: the pre-insert and replace validity steps
    already live in that file as functions that the method's handler calls before the shared primitive
    (`ensure_pre_insertion_validity` `:251`, `ensure_replace_validity` `:437`), so pre-remove step 1 takes the
    same form and place (one home for the spec's validity steps, in the engine-independent layer); writing it
    inline in the handler would be the one validity step without that form. At `bfc63658` the script path is
    `native_node_remove_child` (`vm/host/node_proto.rs:712`) → `invoke_dom_api("removeChild")` → the `RemoveChild`
    handler → `apply_remove_child` → `EcsDom::remove_child`, and the only "is a child" test on that path is
    `is_child_of` inside the last call (`tree/mutation.rs:72`); the handler maps `None` to "NotFoundError".

  From `ta` on, `removeChild`, `insertBefore` and `replaceChild` throw "NotFoundError" for the host's own shadow
  root. This is not a compensating copy: it is the spec step in its algorithm's home, and it stays after `ts`, where
  it agrees with the storage by construction.
- **One predicate per spec step, asked at validation and re-checked at apply.** Pre-insert step 3 is asked by the
  validity function (`child_node/mod.rs:268`) and again by `apply_insert_before` (`mutation/child_list.rs:284`),
  the record-producing primitive that every insertion path calls; replace's step is asked by
  `ensure_replace_validity` (`:451`) and again by `apply_replace_child` (`child_list.rs:439`). Both copies ask the
  same `tree_parent` question, so there is one predicate per step; the second is the apply-time re-check, not a
  second home. The population of `:284` is every caller of `apply_insert_before`
  (`git grep -n -P '\bapply_insert_before\(' bfc63658 -- crates ':!*tests*'`: 9 lines, the definition `:266` and 8 calls), read for "can the
  reference be a shadow root": the `insertBefore` handler (`element/tree.rs:151`) passes the script's node, which
  the validity step refuses first from `ta`; the ChildNode / ParentNode mixins (`child_node/mutations.rs:44`)
  pass `viable_next_sibling` (`next_exposed_sibling`), `children_iter(..).next()` or `next_exposed_sibling`;
  `splitText` (`char_data/split_text.rs:177`) passes `next_exposed_sibling`; `insertAdjacentElement`
  (`element/tree.rs:550`) passes the receiver element, `children_iter(this).next()` or `next_exposed_sibling(this)`;
  `Range.insertNode` (`range/mutation.rs:557`) passes the start text node, its split tail, an index into
  `children_iter`, or `next_exposed_sibling`; `select.add` (`element/select.rs:200`) passes an element, because
  the VM's `normalize_before_arg` (`vm/host/html_select_proto.rs`) sends every non-element `before` to the `long`
  branch, or an item of the options collection; the self-reference advance (`child_list.rs:298`) passes
  `next_exposed_sibling`; the deferred-flush arm (`mutation/mod.rs:203`) has test constructors only. So no engine
  path passes a shadow root as the reference at base, and the re-check at `:284` changes no engine path. The
  population of `:439` is every caller of `apply_replace_child` (`git grep -n -P '\bapply_replace_child\(' bfc63658
  -- crates ':!*tests*'`: 4 lines, the definition `child_list.rs:410` and 3 calls), read the same way: `replaceWith`
  (`child_node/mutations.rs:235`) passes its receiver as the old child, a ChildNode, which a shadow root (a
  DocumentFragment) never is; the `replaceChild` handler (`element/tree.rs:289`) runs after
  `ensure_replace_validity`, which refuses the host's shadow root first from `ta`; the deferred-flush arm
  (`mutation/mod.rs:211`) is constructed by tests only (`git grep -n -P '\bReplaceChild\s*\{' bfc63658 -- crates`:
  the variant, its arm, the handler's `impl`, and two lines of `mutation/tests.rs`). So the re-check at `:439`
  changes no engine path either.
- **elidex-ecs is unchanged in `ta`.** `is_child_of` (`crates/core/elidex-ecs/src/dom/tree/mod.rs:57`) and the
  three mutators that call it (`remove_child`, `insert_before`, `replace_child`; `tree/mutation.rs:72`, `:116`,
  `:165`) stay a stored-link read, population P-REL (§1.1, *Storage maintenance is not a climb*), until `ts` makes a
  shadow root structurally not a child; then the same read refuses it with no edit. Rejected (PM 800): (a) the
  tree-parent equality in `is_child_of` from `ta`. Three engine callers pass a raw first child or raw next sibling
  as the reference child, which for a host can be its shadow root, and all three discard the result (`let _ =`), so
  each would silently insert nothing; so would any caller not found by reading. Also rejected: a separate guard PR
  before `ta` (a compensation `ts` deletes), and leaving the fix to `ts` (the defect would stay in production until
  the last slice).
- **The three engine callers keep base behaviour in `ta`** (they call the elidex-ecs mutators, which are
  unchanged), and the child-side rename of `tc` puts each raw read on the compiler's list. Two of them are `tc`'s
  and pick a tree child from then on, so neither passes a shadow root any more:
  `crates/script/elidex-script-session/src/mutation/html_fragment.rs:222` (`insertAdjacentHTML("afterbegin")`; raw
  first child at `:216`: the first tree child of the target), `:251` (`"afterend"`; raw next sibling at `:243`: the
  next tree sibling of the target). §5 `tc` and §3.1 carry them. The third, `crates/css/elidex-style/src/pseudo.rs:74`
  (`::before` generation; raw first child at `:72`), keeps the stored first child until `ts`: storing the node
  before the shadow root puts it where css-pseudo-4 §4.1 does (§1.3, D1). From `tc` it no longer passes a raw
  child at all: it calls `EcsDom::insert_as_first_stored_child` (NEW: `tc`, §1.3), which reads the stored first
  child inside elidex-ecs and reaches `is_child_of` through its own `insert_before`; `is_child_of` stays a
  stored-link read until `ts` (below), so the insertion still happens. So from `tc` the engine callers that pass a
  raw child as the reference are none. At `ts` the method is deleted (§1.3) and the caller takes the first tree
  child, which is never a shadow root.
- **The other callers of `apply_remove_child` take no pre-remove check.** They run *remove* without *pre-remove* in
  the spec, so the new check is not on their path and `apply_remove_child` is unchanged: `child_node/mutations.rs:180`,
  `:220`, `element/select.rs:260`, `:350` and `range/mutation.rs:182` pass a node found through an exposed accessor
  or the script's receiver, and keep base behaviour. `range/mutation.rs:218` is the second path below: the spec
  keeps the shadow root out of it by *contained*, not by a validity step, and `tq` fixes it there.
- **One exception, with its reason: the deferred-flush arm `Mutation::RemoveChild`** of `apply_mutation`
  (`mutation/mod.rs:204`). Its siblings `Mutation::InsertBefore` and `Mutation::ReplaceChild` reach the apply-time
  re-checks above; `Mutation::RemoveChild` reaches `apply_remove_child`, which has none, and `ta` adds none there.
  The reason is that no production code constructs it at base (`git grep -n -P '\bRemoveChild\s*\{' bfc63658 --
  crates`: the variant, its arm, the dom-api handler's `impl`, and tests). `#11-tree-mutation-record-pipeline`
  (ledger; it stays open as an input of P1 `#11-element-lifecycle-hooks`, PM ledger 2026-10-24) is the slot that
  would route `removeChild` through `record_mutation` and so make this arm the production path; **that slot then owns pre-remove step 1 on the arm** (the
  `ensure_pre_remove_validity` question asked before the mutation is recorded, or re-checked in the arm). §7.4
  hands the facet to the ledger.

**The second path: Range's contained-node walk** (R3, F1). `contained_top_level_nodes`
(`range/mutation.rs:89`) collects candidates with `next_in_preorder_global` (`range/mod.rs:573`), which steps by
the raw first child and the raw next sibling (`:574`, `:579`), so from a host it walks into the shadow root and its
tree. The candidate passes the `compare_points` filter, and `delete_contents` calls `apply_remove_child(h, sr)`
(`range/mutation.rs:216-218`); the probe of §0.1 item 6 shows the result. In DOM the shadow root is never a
candidate: *contained* requires "node's root is range's root" ([D8]).

- **Decided (R3, F1; placed in its own slice `tq` by D3, approved PM 820):** in `tq`, before `ta`,
  `next_in_preorder_global` takes the existing exposed steps, the ones that already skip a shadow root at base: the
  first child through `children_iter(current).next()` (`ChildrenIter` skips a `ShadowRoot`, `dom/mod.rs:911`) and
  the next sibling through `next_exposed_sibling` (`navigation.rs:155`). Two accessor swaps in one function
  (`range/mod.rs:574`, `:579`). This is the program's own relation (tree children, R-TREE 4) taken early on one
  function, not a compensation: `tc` renames these two calls with the rest of the child side, and the function's
  upward step (`range/mod.rs:582`) is `ta`'s (pending, `Flip::Tr`, census C-2). The "separate guard PR
  before `ta`" rejected above (R2; PM 800) was a compensation `ts` deletes (§7.3 Q-19); `tq` adds no guard and
  nothing in it is deleted later, so that rejection does not reach it. Reason for its own slice (D3): *Security by
  structure* and paydown (c), the closed-root exposure leaves production about a week before `ta` (PM 790 (3):
  `ta` ~12-01), and `ta` keeps one declared behaviour change.
- **What the edit reaches.** The function has three users: `contained_top_level_nodes` serves `deleteContents`
  (`:216`) and `extractContents` (`:370`), and `Range.toString` walks it directly (`range/mutation.rs:49`). For
  `deleteContents` the change is the declared one. `extractContents` at base passes the shadow root to
  `apply_append_child(frag, sr)`, which refuses a shadow root (`mutation/child_list.rs:202`, census C-5 role I), so
  its result is predicted unchanged and no longer depends on that refusal.
- **The `toString` facet** (D2). `toString` stops concatenating the text of a shadow tree under a host inside the
  range (predicted; DOM's Range *stringification behavior* step 4 appends "the data of all Text nodes that are
  contained in this", `dom#dom-range-stringifier`, looked up 2026-11-18). Two natives reach it:
  `Range.prototype.toString` (`vm/host/range_proto.rs:890`, `native_range_to_string` → `Range::to_string`) and
  `Selection.prototype.toString` (`vm/host/dom_selection_proto.rs:643` → the `Selection` method `to_string`,
  `crates/dom/elidex-dom-api/src/selection.rs:580` → `Range::to_string`). The facet is **probed** at `tq`'s base
  before its review (§5.2), and its cell uses a range whose end container **follows** the host in tree order (for
  example, start in a text node before the host, end in a text node after it), not the shape of the `deleteContents`
  probe: that shape ends in the host's parent, an ancestor of the start, and meets the defect below instead.
- **A separate, pre-existing defect, not this program's.** `Range::to_string` (`range/mutation.rs:47-58`) walks
  in preorder from the start container until it meets the end container. When the end container is an ancestor of
  the start container, preorder from the start never meets it, so the walk runs to the end of the document and
  appends every later text node. `tq` does not fix it (its output differs only where the overshooting walk passes a
  host, whose shadow tree `tq`'s walk no longer enters); it is a facet of `#11-range-full-impl` (L3), registered
  under PM 826 (§7.4 (N7)).
- Rejected: leaving it to `tc` (the rename would fix it, but the closed-root exposure and the link divergence would
  stay in production for one more slice); leaving it in `ta` (draft 3.3: `ta` would carry two declared changes and
  the exposure would stay until `ta`'s landing); a pre-remove-style check in `delete_contents` (the spec has no such
  step there: the node is never contained).

**One declared behaviour change each.** `tq`'s is the second path: a cell "`deleteContents` over a host neither
unlinks the shadow root nor reports it in a `MutationRecord` (closed mode)", the `toString` facet cell and the
`extractContents` cell above, and a mutant that restores a raw step. `ta`'s is the first path: a cell per method
(with the links and the child list checked after the refused call) and a mutant per site, the new check included.
Slot: `#11-shadow-root-accepted-as-ref-or-old-child` (registered, PM 790), scope widened to "script paths that
unlink a host's shadow root" (the reference / old / removed child, and Range deletion; PM 812). It closes in two
steps (PM 820): the Range path at `tq`, the reference / old / removed child at `ta`; the slot closes at `ta` (§7.1,
§7.4).

## §2. Coupled invariants (edge-matrix)

Elements: **TREE** the stored tree relation; **LINK** the link pair; **WR** writers of a tree edge (census C-9);
**UP-T** / **UP-S** / **UP-H** tree, shadow-including and host-including climbs; **DER** derived relations (§1.2);
**ND** the three relations of §1.5; **CHILD** the child-side set; **ORD** tree order and child index; **CONN**
connectedness; **ROOTS** `root_entities`; **VER** the version counter (`rev_version`) and its readers (live
collections, the shell's render-dirty test); **MEV** the ECS `MutationEvent` fire guards; **CE** custom-element
fan-out; **MO**; **RANGE** (Range, Selection, NodeIterator, TreeWalker); **FORM**; **EDIT** contenteditable;
**SEL**; **RENDER**; **A11Y**; **SHELL**; **PARSE**; **LIFE** teardown; **NAV** the VM navigation natives and the
dom-api handlers (census C-10).

| Pair | Intersection | PR | Pinned by |
|---|---|---|---|
| TREE × LINK | attach writes the link pair and no tree edge; the backward fact is read through `shadow_host` (R-TREE 2) | ta, ts | ts structure cells (attach; the child list of a host) |
| TREE × WR (the node half) | elidex-ecs has one site that writes a parent (`link_node`) and three callers; none refuses a shadow root as the **node** today. `ts` adds the refusal beside the cycle guard in `append_child`, `insert_before`, `replace_child` (they return `false`), and `link_node` asserts it | ts | ts cells: each of the three refuses a shadow-root node; mutant: remove one refusal |
| TREE × WR (the child half) | the mutators' "is a child of parent" check, and the five hand-written copies outside elidex-ecs, accept a shadow root as the **child** today (§0.1 item 6). From `ta` the spec's steps ask the tree-parent question in the engine-independent layer: the five T2 lines and one new pre-remove check on the `removeChild` path only (`ensure_pre_remove_validity`, NEW; `apply_remove_child` is unchanged; §1.6). The mutators' own check stays a stored-link read until `ts` (PM 800, Q-19 (b′)); the three engine callers that pass a raw child as the reference are `tc`'s. The second script path, Range's contained-node walk, hands the shadow root to `apply_remove_child` without a child-of question; from `tq` the walk takes the exposed steps and never reaches it (§1.6, the second path; PM 820) | tq (the Range walk); ta (engine-independent layer); ts (elidex-ecs, by the storage) | tq cells: `deleteContents` over a host (closed shadow root) leaves the links and the child list and reports no shadow root in a `MutationRecord`; the `toString` facet and `extractContents` (§1.6); mutant that restores a raw step. ta cells: `removeChild`, `insertBefore`, `replaceChild` with the host's shadow root throw "NotFoundError" and leave the links and the child list as they were; mutant per site |
| TREE × VER | attach reaches `rev_version(host)` today only through `append_child` (`mutation.rs:54`), and the shell repaints when the document root's version moves (`crates/shell/elidex-shell/src/content/event_loop.rs:444`, `crates/shell/elidex-shell/src/content/mod.rs:369`). The version is a shadow-including-descendants version (`rev_version` climbs by the shadow-including step), and attaching changes the host's shadow-including descendants, so `ts`'s attach calls `rev_version(host)` itself | ts | ts cell: `attachShadow` alone moves the document root's version; mutant: drop the call |
| TREE × UP-T / UP-S | `tree_parent(sr)` is `None` and `shadow_including_parent(sr)` is the host at every head (§1.1) | ta, ts | ta cells (accessor contracts); ts: the same cells, unchanged |
| UP-T × UP-S × DER | every climb takes exactly one named step; every derived relation is on a disposition list (§1.2) | ta | the compiler; ta re-runs census C-6 at its head and accounts for every row |
| UP-S × CONN | *connected* climbs by `shadow_including_parent` (Layout's item 1: its callers `html_fragment.rs:102`, `mutation.rs:44-46`, `:122` keep their answer) | ta, ts | existing connectedness cells; ts adds a nested-shadow case |
| UP-H × TREE | the cycle guards are the host-including test, so a host cannot be appended into its own shadow tree once the stored parent no longer crosses ([D5]) | ta, ts | ts cell: host into own shadow tree, nested two levels |
| CHILD × TREE | until `ts` the one set skips in the accessor layer (both halves, §1.3); after `ts` the skip is gone | tc, ts | tc cells per accessor, including a shadow-root receiver; ts: unchanged |
| CHILD × ORD | the index counts tree children only (D13 (a)); `index_in_parent` already skips shadow-root siblings and gates the `Insert` fire in `append_child` / `insert_before` (Layout's item 2, pinned by `crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs:755-777`) | tc | tc cell: index of a host's children with a shadow root attached before and after them |
| CHILD × NAV | the child and sibling handlers of dom-api are raw readers and unreachable from production (census C-10); `ta` does not touch them (it would have to copy the natives' marker guard into dom-api, two new compensation lines). In `tc` the natives and the handlers read the one set, the natives' own marker tests go, and `hasChildNodes` agrees between them | tc | existing VM navigation cells; a handler test that pins the raw answer is listed in tc's plan |
| CHILD × SHELL | the shell's Tab-order walk (`crates/shell/elidex-shell/src/content/event_handlers.rs:497-500`) and iframe discovery (`crates/shell/elidex-shell/src/content/iframe/lifecycle.rs:449-452`) are raw descendant walks: they reach focusable elements and iframes inside a shadow tree only because the shadow root is a raw child. `tc` moves both to the explicit descent (§1.3) so neither loses the shadow tree. The Tab order's position of a shadow tree's focusables then changes from "where the shadow root sat in the raw child list" (no step of HTML's order) to "right after the host", which is HTML §6.6.2's *flattened tabindex-ordered focus navigation scope* step 2.2 (a host that is not focusable is replaced in place by its scope's contents) and step 2.3 (a focusable host is followed by them) [H15]: a change toward the spec (PM 800, §7.3 Q-18). **The descent skips a host whose tabindex value is negative**: such a host is not in its owner's *tabindex-ordered focus navigation scope* ("all elements in owner's focus navigation scope that are themselves focus navigation scope owners, except the elements whose tabindex value is a negative integer", [H15]), so step 2 never expands its scope. The host's light children are still walked as at base; that half is **P2's residual, not a `tc` result**: by *associated focus navigation owner* step 2 ("If element's parent is a shadow host, then return element's assigned slot.") a host's child belongs to its assigned slot's scope, or to no scope when unassigned [H15], so under a negative-tabindex host it is not reached through the host either. The remaining differences (a slot as a scope owner, which includes the light children above; tabindex order inside a scope) are `#11-shadow-slot-assignment-flat-tree`'s (P2); the skip of the negative-tabindex host's shadow tree is `tc`'s, not among them | tc | tc cells: Tab reaches a focusable element in a shadow tree; **the focusables of a shadow tree come right after the host (§6.6.2 steps 2.2 and 2.3), for a focusable and a non-focusable host**; **a host with `tabindex="-1"` contributes no focusable of its shadow tree** (no cell pins its light children: P2's residual); an iframe in a shadow tree is collected; mutant: drop the descent; mutant: drop the negative-tabindex skip |
| CHILD × MO | a mutation record's previous / next sibling come from the exposed accessors (`crates/script/elidex-script-session/src/mutation/child_list.rs`), which fold into the set: no change predicted | tc | existing record cells |
| CHILD × VER (live collections) | `live_collection/mod.rs:443` re-tests a child that the iterator already filtered: deleted | tc | existing live-collection cells |
| CHILD × SEL | sibling combinators and structural pseudo-classes read element siblings, which a shadow root never is: no change predicted | tc | existing selector cells |
| CHILD × FORM | 15 raw child reads in elidex-form (`git grep -n -P '\b(get_first_child\|get_last_child\|get_next_sibling\|get_prev_sibling)\(' bfc63658 -- crates/dom/elidex-form ':!*tests*'`): a descendant walk that meets a host's shadow root as a child enters the shadow tree today (predicted) and stops entering in `tc`; the upward half of the same fact flips in `tf` | tc → tf | tc cells (downward); tf cells (upward); the interim is in §5.1 |
| CHILD × RANGE | Range boundary walks and 29 traversal lines read raw children (census C-3, `traversal.rs`): D13, and TreeWalker / NodeIterator entering a shadow tree (predicted); the upward half flips in `tr`. One Range walk moves earlier: `next_in_preorder_global` takes the exposed steps in `tq` (§1.6, the second path; PM 820), and `tc` renames them with the rest | tq (the contained-node walk); tc → tr | tq cells of §1.6; tc cells; RC-1 consumes the set (§7.2) |
| CHILD × RENDER, A11Y | layout and render read `composed_children` for content; their raw reads (census C-3, C-13) are re-read line by line: tree children, the explicit descent, or `composed_children`. One raw walk needs the descent to keep its answer: `save_descendant_styles` (`crates/layout/elidex-layout-block/src/helpers.rs:282`) saves the styles of a shadow tree's content today. `tc` renames 28 child-accessor lines in layout, render and style files (§7.2) | tc | no expectation changes predicted; tc's probe (§5.2), including a layout probe over a host with a shadow tree |
| CHILD × PARSE | the strict parser's append position (`crates/dom/elidex-html-parser-strict/src/tree_builder/insert.rs:212`, Layout's item 6) and `innerHTML` on a shadow root read the tree children of the node they were given: no change predicted | tc | the parser's existing shadow-context cells |
| UP-T × NAV | `parentNode` / `parentElement` of a shadow root are null through the native and the handler ([D7]); `contains` is the tree test in both (PM 748 (b)). These three handlers are unreachable from production (census C-10), so taking the native's relation changes no production path. The **reachable** handlers that `ta` re-types (`isConnected`, `getRootNode`, `compareDocumentPosition`, `closest`) keep the relation they compute at base | ta | existing VM cells; no new cell |
| UP-T × MO | *queue a mutation record* collects from the **inclusive ancestors** of the target ([D6]): an observer on the host or the document with `subtree: true` gets no record for a mutation inside the shadow tree; transient observers follow the same ancestors | tm | tm cells (host observer, document observer; an observer on the shadow root still gets them) |
| UP-T × RANGE | a live range's root is its start node's root; *get the common ancestor* climbs by parent; boundary points are compared within one root ([D8], [D9]); traversal stays inside its root ([D10]); `Selection.containsNode` reads the same boundary comparison ([X1]); a shadow-root argument to the sibling-slot methods has a null parent | tr | tr cells |
| UP-T × FORM | *reset the form owner* step 4, *disabled fieldset*, *nearest ancestor select*, the optgroup of *option disabled*, "nearest ancestor dialog", and `select.add`'s ancestor tests ("an ancestor of the select element", "a descendant of the select element", [H12]): tree ancestors ([H2] to [H5], [H8], [H9]) | tf | tf cells; `#11-form-owner-ecs-component` (§7.1) |
| UP-T × EDIT | the inherit state follows "the parent element's state" ([H11]), and *editable* is defined on the tree parent ("its parent is an editing host or editable", [E1]): an element in a shadow tree does not inherit editability from its host. `:read-write` matches "elements that are editing hosts or editable" ([H13]) and reads the same function | te | te cells (`isContentEditable`, `:read-write`) |
| UP-T × PARSE | the strict form-pointer walk is a tree climb ("going straight up the ancestor chain", [H14]); its guard folds into the step (Layout's item 5) | ta | the strict parser's existing shadow-context cell |
| ORD × DER | `tree_order` is DOM §1.1 tree order within one tree, with the entity-bits fallback across roots. `compare_document_position` and the radio group call it only for nodes of one tree root (census C-8 R-9), so they take it in `ta` with no change | ta | existing `compareDocumentPosition` and radio cells |
| ORD × FORM | the radio group's sort is over one tree root (above): it shares no cell with `tf` | ta | — |
| ORD × RANGE | Range's `compare_points` takes the crossing order as a pending name and flips with the rest of Range | tr | tr cells |
| ORD × RENDER (ND) | paint and hit-test layer 6 sort by tree order across shadow boundaries (`crates/core/elidex-render/src/builder/walk.rs:613-624`, `crates/layout/elidex-layout/src/hit_test.rs:228-239`, both through `tree_order_cmp`). The order they need is the flat tree's, which `#11-shadow-slot-assignment-flat-tree` (P2, Layout) owns. They take `pending_flat_tree_order`, which is the base function until `ts`. At `ts` the shadow root is no longer among the host's children, so the function must say where the shadow tree sits: DOM §4.8 *shadow-including tree order* (the shadow tree just after its host). **One residual**: a positioned shadow-tree node and a positioned light child that preceded the shadow root in the host's child list swap their order. **It is observable through user input**: hit-test layer 6 picks the click target, so the swap can change which element receives a pointer event. Neither order is the flat tree's; Layout accepts shadow-including order until P2 lands, and no layout, render or WPT test pins either (Layout's answer to Q-13). **Until `ts` the order between siblings is the stored order** (`sibling_order`, §1.3), so a slice that changes where a node is stored next to a shadow root changes layer 6. **`tc` does, for two insertions only** (declared, R3 F2, narrowed by D1): `insertAdjacentHTML("afterbegin")` / `("afterend")` (§1.6) take the first tree child or the next tree sibling as the reference, so a node that base stored just before the host's shadow root (when the shadow root was the host's first stored child, or the target's raw next sibling) is stored after it, and now sorts after a positioned node of the shadow tree in layer 6, where base sorted it before. That is the order `ts` produces (the shadow tree just after its host), so it is the same class as `ts`'s residual, accepted under Q-13 until P2; it is reached only through the session API (no VM native; census C-10 "U"), not from production. **`::before` is not among them** (D1): it stays a stored-order write until `ts`, through the one elidex-ecs write `EcsDom::insert_as_first_stored_child` (NEW: `tc`, §1.3), so it keeps base's order, which is css-pseudo-4 §4.1's ("immediately before the originating element's actual content", [P1]). **At `ts` the residual gains `::before`**: the generated node is then a light child that preceded the shadow root, the class above, and the plain shadow-including order would put it after the shadow tree. `ts` keeps it first: `pending_flat_tree_order` orders a host's generated `::before` node before the host's shadow tree (the generated node carries `PseudoElementMarker`, a unit marker with no before / after kind, `components.rs:184`, so `ts`'s plan states how it tells `::before` from `::after`: a kind on the marker, or the node's stored position as the host's first child), and a `ts` cell pins it. **Cell preconditions** (all layer-6 cells of this row): the host is a stacking context (otherwise a positioned shadow-tree node never reaches layer 6 through `children_iter`, Layout's own defect, §7.2 (a)), and the shadow tree has a default `<slot>`, so that the host's light children, the generated node included, are in the flat tree and so can reach layer 6 at all (whether a generated node should be a stored, slotted light child is P2's question, §7.1) | ta, tc, ts | tc cell: a stacking-context host whose shadow root is its first stored child, with a default slot, a positioned node in its shadow tree and a positioned node inserted with `insertAdjacentHTML("afterbegin")` (session level): layer-6 order and hit-test target; the same host with a positioned `::before`: unchanged from base (before the shadow tree). ts cells: existing paint / hit-test cells unchanged; the residual case pinned with shadow-including order, for paint and for the hit-test target; the positioned `::before` still before the shadow tree, for paint and for the hit-test target |
| UP-S × VER | `rev_version` propagates to shadow-including ancestors; its documented `.host` branch becomes the reachable one | ta, ts | existing live-collection cell across a shadow boundary |
| UP-S × CE, clone | the downward shadow-including walks reach the root through `get_shadow_root`, never through a child list (`walkers.rs:59-88`); the adopt walk climbs by `shadow_including_parent`. `for_each_shadow_inclusive_descendant`'s bound and order are `#11-capped-walker-deep-element-processing`'s subject and are not changed here | ta, ts | existing CE and adopt cells |
| ND × all | F, E and L readers take their pending names with base behaviour; the selector boundary lines (census C-5 role Ls) and the event break (role G) are not edited | ta | no expectation changes; the trip-wire |
| ROOTS × TREE | a shadow root becomes parentless; `root_entities` already excludes `DocumentFragment` kinds; the six "is this the root" tests are flat-tree tests (census C-8 R-3) and do not depend on the storage | ts | ts cell: `root_entities` with an attached shadow root |
| MEV × TREE | the fire guards test `is_shadow_root(node) \|\| is_shadow_root(parent)` (`mutation.rs:528`, `:582`). The **parent** half is relied on by `LiveRangeRegistry` (`crates/dom/elidex-dom-api/src/range/live.rs:38-54`) and by Layout's T3 (T-25, OR-3); it is not touched. The **node** half becomes unreachable in `ts` (nothing links a shadow root) and is deleted there; Layout is told when `ts`'s plan is written (Layout's item 7) | ts | ts cell: attach fires nothing; a child appended to a shadow root behaves as at base |
| LIFE × TREE | `destroy_entity`'s raw orphaning loop (`teardown.rs:137-143`) no longer meets the shadow root (Layout's item 4); the root is still stripped through the link and left as today. `destroy_entity(sr)` / `despawn_subtree(sr)` read "the external parent" once for two relations (`teardown.rs:90`, `:221`): `ts` splits it, the place in a child list (none) and the version bump (the host, by the shadow-including step) | ts | ts cells; existing `destroy.rs` cells keep their expectations |
| LIFE × LINK | this program adds no back-link and changes nothing that is reclaimed | — | structural |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §1.1 Trees | *parent*, *root*, *inclusive ancestor*, *index*, *tree order* | the default relations do not mention hosts | `tree_parent`, the tree test and order (ta); the child set (tc) | ✓ | yes |
| WHATWG DOM §2.7 Interface EventTarget | *get the parent* (dfn) | overridden by nodes, shadow roots and documents | none: `pending_event_parent` (§1.5) | ✓ | yes |
| WHATWG DOM §4.2.2 Shadow tree | "a node tree whose root is a shadow root"; *connected* | the root has a null parent | storage (ts); `is_connected` (ta) | ✓ | yes |
| WHATWG DOM §4.2.3 Mutation algorithms | *ensure pre-insert validity* steps 2 and 3; *replace* step 1; *pre-remove* step 1; *move* step 2 | host-including inclusive ancestor → `HierarchyRequestError`; "child is non-null and its parent is not parent" / "child's parent is not parent" → `NotFoundError` | the host-including test (ta, ts); the child-of question (ta, §1.6); *move* is not implemented (no `moveBefore`) | ✓ | yes |
| WHATWG DOM §4.3.2 Queuing a mutation record | step 2 | "the inclusive ancestors of target" | observer ancestor walks (tm) | ✓ | yes |
| WHATWG DOM §4.4 Interface Node | `parentNode`, `parentElement`, `isConnected`, `contains`; `compareDocumentPosition` step 6; a node's *get the parent* | a shadow root's parent is null; different roots are disconnected | natives and handlers (ta, tc); `tree_order` (ta) | ✓ | yes |
| WHATWG DOM §4.7 Interface DocumentFragment | *host*; *host-including inclusive ancestor* | the host is "in a different node tree" | the one host-including test (ta) | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *shadow-including root*, *ancestor*, *tree order*; *retarget*; a shadow root's *get the parent* | each crossing is written as "root's host" | `shadow_including_parent`, `shadow_host` (ta); *retarget*'s read (ta) | ✓ | yes |
| WHATWG DOM §4.9 Interface Element | `closest` step 3 | "inclusive ancestors that are elements" | `child_node/selectors.rs` (ta) | ✓ | yes |
| WHATWG DOM §5.2 Boundary points | *position* | "Assert: nodeA and nodeB have the same root" | `compare_points` (tr) | ✓ | yes |
| WHATWG DOM §5.6 Interface Range | *root*; *contained*; *get the common ancestor* step 2; `setStartBefore` steps 1-2 | within one tree ("node's root is range's root"); a null parent throws | the contained-node walk (tq, §1.6); Range climbs (tr) | ✓ | yes |
| WHATWG DOM §6 Traversal | NodeIterator and TreeWalker *root* | inclusive descendants of the root | traversal climbs (tr) and child reads (tc) | ✓ | yes |
| Selection API §3 Selection interface | `containsNode` | through the range's boundary points | `selection.rs` (tr) | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | "not children of the element itself" | the precedent, already (X)-shaped in elidex | none (comparison only; §4) | ✓ | no |
| WHATWG HTML §4.10.18.3 Association of controls and forms | *reset the form owner* step 4 | "has an ancestor form element" | form-owner climbs (tf) | ✓ | yes |
| WHATWG HTML §4.10.15 The fieldset element | *disabled fieldset* | "a descendant of another fieldset element" | fieldset climbs (tf) | ✓ | yes |
| WHATWG HTML §4.10.19.5 Enabling and disabling form controls: the disabled attribute | *disabled* | "a descendant of a fieldset element whose disabled attribute is specified" | fieldset climbs (tf) | ✓ | yes |
| WHATWG HTML §4.10.10 The option element | *nearest ancestor select* step 2; *disabled* (the optgroup ancestor) | ancestors in reverse tree order | option / select climbs (tf) | ✓ | yes |
| WHATWG HTML §4.10.22.3 Form submission algorithm | "nearest ancestor dialog element" | tree ancestor | `submit.rs` (tf) | ✓ | yes |
| WHATWG HTML §2.6.4.3 The HTMLOptionsCollection interface | `add(element, before)` steps 1-2 (HTML §4.10.7 The select element: `select.add` "must act like its namesake method on that same options collection") | "an ancestor of the select element"; "isn't a descendant of the select element" | `element/select.rs` (tf) | ✓ | yes |
| WHATWG HTML §4.16.3 Pseudo-classes | `:read-write` | "elements that are editing hosts or editable" | `selector/matching.rs`, the `:read-write` arm (te) | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | step 19 | "going straight up the ancestor chain, and including the element itself" | the strict form-pointer walk (ta) | ✓ | no |
| WHATWG HTML §6.8.1 Making document regions editable: The contenteditable content attribute | the inherit state | "the parent element's state" | contenteditable climbs (te) | ✓ | yes |
| execCommand §6 Common definitions | *editable* | "its parent is an editing host or editable" | contenteditable climbs (te) | ✓ | yes |
| WHATWG HTML §6.3 Inert subtrees | *inert* | "flat tree descendants" | none (§1.5) | ✓ | yes |
| WHATWG HTML §3.2.6.2 The lang and xml:lang attributes | *language* | "If the node's parent is a shadow root" | none: no `lang` climb exists in the code; wording evidence (below) | ✓ | no |
| WHATWG HTML §3.2.6.4 The dir attribute | *parent directionality* step 2 | "If parentNode is a shadow root" | none: no `dir` climb exists; wording evidence | ✓ | no |
| CSS Selectors L4 §14.1 Descendant combinator ( ) | combinators | ancestors in the selector's tree | none (§1.5; owner `#11-shadow-tree-style-isolation`, PM 794) | ✓ | yes |
| CSS Shadow Module Level 1 (css-shadow-1) §3.2.2 Selecting Shadow Hosts from within a Shadow Tree | the featureless host | "treated as replacing the shadow root node" | none (§1.5) | ✓ | yes |
| CSS Shadow Module Level 1 (css-shadow-1) §4.1 Flattening the DOM into an Element Tree | *flat tree* | — | none (§1.5) | ✓ | yes |
| CSSOM View (cssom-view-1) §7 Extensions to the HTMLElement Interface | `offsetParent` step 2 | "the parent of the element in the flat tree" | none (§1.5) | ✓ | yes |
| CSS Pseudo-Elements Level 4 (css-pseudo-4) §4.1 Generated Content Pseudo-elements: ::before and ::after | `::before` | "immediately before the originating element's actual content" [P1] (a host's actual content: its shadow tree, by css-shadow-1 §4.1's note that in the flat tree "shadow hosts are filled with their shadow tree children" [C1]) | `pseudo.rs:72` keeps the stored first child (tc, §1.3); `pending_flat_tree_order` keeps a host's `::before` before its shadow tree (ts, §2 ORD × RENDER) | ✓ | yes |

**Breadth**: K=8 specs, M=32 entries (verified 2026-11-18 with `.claude/tools/webref coverage-map`; the argument
list is in §8; draft 3.4 adds the css-pseudo-4 row, from K=7, M=31). The verdict is "split by default" (K ≥ 6 or
M ≥ 30), and the program is split (§5); the cut follows §0.2. The preflight's label map has no entry for Selection
API, css-shadow-1, cssom-view-1, execCommand and css-pseudo-4, so those six rows were checked by hand
(`.claude/tools/webref heading selection-api 3`, `heading css-shadow-1 3.2.2`, `heading css-shadow-1 4.1`,
`heading cssom-view-1 7`, `heading execCommand 6`, `heading css-pseudo-4 4.1`). Nine of the rows touch no code
(relations this program names but does not define, or wording evidence).

**How the spec words (X).** DOM's tree relations never reach a host. Where a spec crosses, it names the step: DOM
§4.8 writes "root's host" ([D4]); HTML's language rule says "If the node's parent is a shadow root: Use the
language of that shadow root's host" ([H6]); HTML's directionality rule says "If parentNode is a shadow root, then
return the directionality of parentNode's host" ([H7]). In each, the parent chain ends at the shadow root and the
host is a second, named step. Two relations do put the host above the shadow tree, under their own names: a shadow
root's event *get the parent*, and the selector tree in which the host replaces the shadow root (§1.5). Neither is
*parent*.

### §3.1 User-input touch audit

Script-observable changes, by slice; all are **predicted from the code** and probed before the plan's review
(§5.2).

- `tq`: one, declared (§1.6, PM 820). Range's contained-node walk no longer enters a shadow tree:
  `deleteContents` over a host no longer unlinks the shadow root or reports it in a `MutationRecord` (closed mode
  included); `Range.prototype.toString` and `Selection.prototype.toString` no longer include a shadow tree's text
  (predicted; probed before review, §5.2); `extractContents` is predicted unchanged. The pre-existing `toString`
  overshoot (an end container that is an ancestor of the start) is not fixed (its output differs only where the
  overshooting walk passes a host, §1.6).
- `ta`: one, declared (§1.6). `removeChild`, `insertBefore` and `replaceChild` given the host's own shadow root as
  the child throw "NotFoundError" instead of succeeding (the checks in the engine-independent layer, pre-remove
  step 1 on the `removeChild` path only; elidex-ecs unchanged, PM 800). Nothing else: the other callers of
  `apply_remove_child` run no *pre-remove* step and are unchanged (§1.6).
- `ts`: none for script. One residual reaches the user through input, not through script: the order of hit-test
  layer 6 for a positioned shadow-tree node against a positioned light child (§2 ORD × RENDER) decides the target
  of a pointer event, and so `event.target`. The flat-tree order belongs to `#11-shadow-slot-assignment-flat-tree`.
  A positioned `::before` is kept before the shadow tree, as at base (§2 ORD × RENDER).
- `tc`: any raw child or sibling reader that exposes a shadow root as a child of its host: Range boundary indices
  and walks (D13), TreeWalker / NodeIterator descending into a shadow tree, form descendant walks, `normalize`.
  The readers that must keep entering a shadow tree are moved to the explicit descent and are **not** meant to
  change (§1.3): Tab order, iframe discovery, the style save of a layout probe. The Tab order keeps its members
  except one, and a shadow tree's focusables move to right after the host: HTML §6.6.2 steps 2.2 / 2.3 [H15], a
  change toward the spec (PM 800, §7.3 Q-18); the exception is a host whose tabindex is negative, whose shadow
  tree's focusables leave the Tab order (HTML §6.6.2, *tabindex-ordered focus navigation scope*, [H15]; §2 CHILD ×
  SHELL); `tc`'s cells pin both. The host's light children keep their base position in the walk, a
  residual of P2 (§2 CHILD × SHELL). Two engine callers that pass a raw child to the elidex-ecs mutators (§1.6)
  pick tree children from `tc`: `insertAdjacentHTML("afterbegin")` / `("afterend")` insert before the target's
  first tree child / the target's next tree sibling, appending when there is none. Where base passed the host's
  shadow root as the reference, the insertion landed just before it in the raw list, which gives the same
  tree-child order; so both are predicted unchanged for script, and `tc`'s cells pin that (a host as the target,
  its shadow root first in the raw list). The cells are session-level: no VM native reaches the handler (census
  C-10 "U"). **They do change the stored position** relative to the shadow root, which layer 6 reads until `ts`
  (declared, §2 ORD × RENDER): an order change of the same class as `ts`'s residual, not reachable from
  production. The third caller, `::before` generation, keeps the stored first child until `ts` and does not change
  (§1.3, D1).
- `tm`: `MutationObserver` with `subtree: true` on a host, an ancestor of a host, or the document no longer
  receives records for mutations inside the shadow tree.
- `tr`: `Range.commonAncestorContainer`, boundary comparison, `isPointInRange` / `comparePoint`, contained-node
  enumeration and traversal steps no longer leave the shadow tree upward; `setStartBefore(shadowRoot)` and its
  siblings throw.
- `tf`: a form control inside a shadow tree has no form owner from a light-tree `<form>` around its host, is not
  disabled by a light-tree `<fieldset disabled>`, an `<option>` does not find a `<select>` or `<optgroup>` across
  the boundary, `method=dialog` does not find a dialog across it.
- `te`: `isContentEditable` and `:read-write` no longer inherit across the boundary.
- Not changed: the insertion guards that reject a shadow root as the inserted node (census C-5 role I); event
  paths, retargeting and `composedPath`; selector matching other than `:read-write`.

## §4. Lens block (basis order)

- **spec (first)**: §3. A shadow tree's root is a shadow root (DOM §4.2.2 Shadow tree), a root has a null parent
  (DOM §1.1 Trees), the host is "an element in a different node tree" (DOM §4.7 Interface DocumentFragment), every
  crossing is the explicit step "root's host" (DOM §4.8 Interface ShadowRoot), and HTML's template contents "are
  not children of the element itself" (HTML §4.12.3 The template element). The stored relation that matches all
  of these is (X). A shadow root offered as a child fails "its parent is not parent" (DOM §4.2.3 Mutation
  algorithms), and a shadow root is never *contained* in a range whose root is not the shadow root (DOM §5.6
  Interface Range); the two decide §1.6.
- **elidex principles (where the spec does not choose a representation)**:
  - *One issue, one way*: a template's contents and the inert template document are already (X)-shaped
    (`attach_template_contents` links the fragment and appends nothing, `dom/mod.rs:620-643`). After the program
    all three owned roots have one representation, "parent" has one meaning, and each relation has one function.
  - *Fail-safe direction*: the climb that says nothing must not cross; a derived relation on no list is renamed
    (§1.2); a pending line without a slice does not compile (§1.4).
  - *ECS-native*: a relation is a component, and two relations are two components. (Y) stores the host relation
    inside the tree component and subtracts it again at every reader. Under (X) the subtraction is gone after `ts`;
    between `ta` and `ts` it exists in exactly two accessor layers (§1.1, §1.3) instead of at each reader.
  - *Ideal over pragmatic*: delete `get_parent` rather than document a convention for it.
- **Rejected: (Y)** (PM 730); **making the template contents a tree child too** (contradicts HTML §4.12.3 and
  extends the compensation to a second kind).
- **Rejected: keeping `get_parent` with a documented convention.** The compiler then enumerates nothing; that is
  the failure the independent verification found. **Rejected: hand-listing the derived relations** (draft 1): the
  list missed `is_ancestor_or_self`. The population is defined by what the bodies do (census C-6).
- **Rejected: changing the storage first.** Every crossing climb that leans on the stored parent would stop at the
  boundary without a compile error.
- **Rejected for §1.4: a memo table of pending flips** (anchors drift; nothing fails when a site is forgotten);
  **a count-only check** (it misses a swap of one caller for another, and another lane can add a caller: R1);
  **flipping inside `ta`** (one PR would carry the MutationObserver, Range, form and editing cells).
- **Rejected for §1.5: `shadow_including_parent` for flat-tree, event and selector readers** (draft 1). It records
  a relation nobody chose, and the backlog would live only in a memo. Distinct names make the three populations
  compiler-enumerable for their owners.
- **Rejected for §1.3: renaming only the raw child accessors** (draft 2 left this open). The readers of the
  compensating accessors would never be listed, and R2 found two shell walks and one layout walk whose behaviour
  `tc` would have changed without a line in any plan. **Rejected: a new "shadow-including children" accessor**:
  it is not a DOM relation, and it would be a fourth child-side name.
- **Rejected for §1.6: a guard PR before `ta`; leaving the defect to `ts`; leaving the Range path to `tc`, or in
  `ta`** (reasons in §1.6). `tq` is not the rejected guard PR: it takes the program's own relation on one function
  and nothing in it is deleted later (§1.6, PM 820).
- **Rejected for §1.3: `::before` generation taking the first tree child in `tc`** (draft 3.3). It would move a
  positioned `::before` behind the shadow tree in layer 6, away from css-pseudo-4 §4.1's order, for a production
  path; the stored first child keeps the spec's order until `ts`, which keeps it explicitly (D1).
- **Rejected for §1.3 (draft 3.5): a public stored-first-child reader for `pseudo.rs`** (widening `read_rel`, or
  a `first_stored_child` accessor). It would be a fourth child-side name, the kind the rejection above refuses, and
  it would put a storage read outside elidex-ecs. The one write `EcsDom::insert_as_first_stored_child` returns no
  child and keeps the storage write in elidex-ecs (§1.3).
- **Rejected for the dom-api handlers: one rule for all of them.** A handler that production reaches keeps its
  spec relation; only an unreachable one may be re-typed to its native's relation without a cell (§2 UP-T × NAV).
  Routing the natives through the handlers is `#11-dom-api-handlers-unreachable-from-production`'s and R-NAT's,
  not this program's. The child-side handlers are not re-typed in `ta` at all (§2 CHILD × NAV): PM 748 (b)'s
  "in `ta`" is met for the parent side, and the child side follows one slice later, where the compiler lists it.

## §5. The program

| # | Slice | Primary module(s) | Slot | Tier | Deps |
|---|---|---|---|---|---|
| **tp** | **Touch-time splits**, each a standalone prerequisite PR with no behaviour change, cut by L3 (PM 758 Q-15). **tp-1** `traversal.rs` (1479 lines; seams: the shared preorder helpers, `TreeWalker` from `:153`, `NodeIterator` from `:1018`, the inline tests from `:1224`), before `ta`: `ta` and `tr` both edit it. **tp-2** the shell's `tests.rs` (1081 lines), before `ta`, if `ta`'s plan finds a seam and not a flat case list. **tp-3** `builder/mod.rs` (1002 lines; the margin-box code `:489-991`; `:993` is `find_roots`'s doc comment), before `tc`; Layout confirms the seam. The block has no child-side read (the file's two `get_first_child` calls, `:483` in `find_roots_mut` and `:999` in `find_roots`, are outside it), so "before `tc`" is only the order of touching the file, not a collision (Layout, checked at `1146380f`; the file is the same at `bfc63658`). **tp-4** the test files over 1000 lines that `tc` edits: `elidex-dom-api` `live_collection/tests.rs` (1137 lines; 29 child-side lines) and `elidex-js` `vm/tests/tests_custom_elements.rs` (1321 lines; 1 child-side line), each before `tc` if `tc`'s plan finds a cohesion seam; where the plan finds a flat case list instead, it says so and the file is not split (CLAUDE.md, *1000-line debt*: cohesion, not line count). | `elidex-dom-api` `traversal.rs`, `live_collection/tests.rs`; `elidex-shell` `tests.rs`; `elidex-render` `builder/mod.rs`; `elidex-js` `vm/tests/tests_custom_elements.rs` | — (CLAUDE.md, *1000-line debt*) | — | — |
| **tq** | **Range's contained-node walk takes tree children** (§1.6, the second path; D3, approved PM 820). `next_in_preorder_global` takes the two existing exposed child steps (`children_iter(current).next()`, `next_exposed_sibling`) in place of the raw ones (`range/mod.rs:574`, `:579`); its upward step (`:582`) is untouched and is `ta`'s. One declared behaviour change: `deleteContents` over a host neither unlinks the shadow root nor reports it in a `MutationRecord`; the `toString` facet (both natives, §1.6) and `extractContents` (unchanged) have cells. The `toString` overshoot of `range/mutation.rs:47-58` is not fixed (its output differs only where the overshooting walk passes a host; §1.6, §7.4 (N7)). **Per-PR plan-review: required.** `/elidex-plan-review` SKILL.md, *When to invoke*: the MANDATORY rule asks for a plan-review of each per-PR plan before its code, and its base case ("承認済 umbrella 配下で plan-review を通った narrowly-scoped per-PR slice は terminal 単位") makes a slice a terminal unit only once that slice's plan **has passed** plan-review: it ends the splitting, not the review. `tq` meets the trigger on its own (it intersects TREE × WR, CHILD × RANGE, LINK and MO of §2: ≥3 intersecting axes), so the *Skip OK* exception is closed to it. So `tq` has its own per-PR plan, run through `/elidex-plan-review` in full, and is then one PR, not split further | `elidex-dom-api` `range/mod.rs` (`next_in_preorder_global`); its cells in `range/` tests and the VM's Range / Selection tests. No file it touches is over 1000 lines at `2ac9fa44` (draft 3.5, §8: `range/tests.rs` 453, `range/live_tests.rs` 527, `range/tests_adjust.rs` 360, `selection_tests.rs` 367; the VM's `tests_range.rs` 443, `tests_range_stubs_validation.rs` 509, `tests_selection.rs` 787, `tests_mutation_observer/range_ops.rs` 586, `tests_shadow_dom/attach.rs` 727; `range/mod.rs` 589), so CLAUDE.md's *1000-line debt* rule asks for no prerequisite split; `tq`'s plan re-counts at its base and, if a file it adds cells to has crossed 1000, splits it as a `tp`-style prerequisite or puts the cells in a new test file | `#11-shadow-root-accepted-as-ref-or-old-child`: closes its Range path (PM 820); the slot stays open until `ta` | — | tsel (erratum, PM 884) (it lands before `ta`, PM 820; no code dependency on `tp`) |
| **ta** | **Every climb chooses.** The two steps, the transitional accessor with `Flip`, and the four pending names of §1.5 replace `get_parent`, which is deleted, tests included. Every derived relation is removed, renamed or kept by rule (§1.2). Every reader of the backward link that climbs or lives outside elidex-ecs goes through `shadow_host` (*retarget*, a11y, the `ShadowRoot.host` native). The parent-side dom-api handlers follow §2 UP-T × NAV. The spec's child-of steps ask the tree-parent question (§1.6, the first path), and the comment at `vm/host/node_proto.rs:722-725` (which says a shadow root "is never a `parent.children[i]`" and is rejected "naturally", the belief behind §0.1 item 6) is replaced by one that names `ensure_pre_remove_validity`. The trip-wire lands. Behaviour-preserving **except the one declared change** (§1.6). | `elidex-ecs` `dom/tree/*.rs`, `dom/mod.rs`, `dom/shadow.rs`; every crate of census C-2 (21 crates); in `elidex-js` `vm/host` the 25 files of census C-12 (verified 2026-11-18, §8) | `#11-shadow-root-is-a-tree-root`; closes `#11-shadow-root-accepted-as-ref-or-old-child` (registered, PM 790; its reference / old / removed child path here, its Range path at `tq`, PM 820) | — | tp-1, tp-2, tq |
| **tc** | **Every child-side read chooses** (R-TREE 4, §1.3): one accessor set under new names replaces every child-side accessor; the interim skip (result and receiver halves) lives in the accessor layer; caller-side skips and guards (census C-5 roles K, R) are deleted; the three walks that need a shadow tree take the explicit descent; two of the three engine callers that pass a raw child to the elidex-ecs mutators (`html_fragment.rs:222`, `:251`; §1.6) pick tree children, which moves the stored position of what they insert relative to a shadow root (declared: layer-6 order, §2 ORD × RENDER; session API only); `sibling_order` takes the stored link, not the set, and `::before` generation (`pseudo.rs:72-76`) calls the one new elidex-ecs write `EcsDom::insert_as_first_stored_child` (NEW: `tc`; population P-REL; deleted in `ts`) in place of its raw first child (§1.3); the Tab-order descent skips a host with a negative tabindex (§2 CHILD × SHELL). Observable where base exposes a shadow root as a child (§3.1). | `elidex-ecs` `dom/tree/navigation.rs`, `dom/tree/mutation.rs` (the new write), `dom/mod.rs`; `elidex-style` `pseudo.rs`; every file of census C-3 and C-4, among them shell files (`content/event_handlers.rs`, `content/iframe/lifecycle.rs`, `elidex-navigation` `loader.rs`, `resource.rs`) and layout, render and style files (§7.2) | as ta; closes `#11-range-compare-points-shadow-index` (D13; PM 758 Q-16) | — | ta, tp-3, tp-4 |
| **tr** | **Range, Selection and traversal climbs are tree climbs** (`Flip::Tr`), including the crossing ancestor test and order they call and the VM's `require_attached_node`. The null-parent step of `setStartBefore` and its siblings ([D8]) stays where it is today, in the VM host (`range_proto.rs:529`): `tr` changes the relation that line reads and does not move the step. Its move into dom-api (whose `set_start_before` does not carry the step) belongs to R-NAT's host chapter and `#11-dom-api-handlers-unreachable-from-production`. | `elidex-dom-api` `range/`, `selection.rs`, `traversal/`; `elidex-js` `vm/host/range_proto.rs` (`:432`, `:529`, `:783`, and the callers at `:749`, `:768`) | as ta; `#11-shadow-tree-live-range-tracking` (§7.1) | — | ta, tc |
| **tf** | **Form-association climbs are tree climbs** (`Flip::Tf`): the form owner's ancestor step (PM 747 Q-6; the plan cites HTML §4.10.18.3 through webref), fieldset disabledness, option / select / optgroup, the dialog of `method=dialog`, `select.add`'s ancestor tests ([H12]). | `elidex-form` `submit.rs`, `select/mod.rs`; `elidex-form-core` `fieldset.rs`; `elidex-dom-api` `element/option_disabled.rs`, `element/select.rs`; `elidex-js` `vm/host`: the nine `.form` getters that call `find_form_ancestor` (`html_{button,fieldset,input,label,legend,option,output,select,textarea}_proto.rs`) and `html_option_proto.rs:367` (`find_option_select`) (census C-12) | as ta; `#11-form-owner-ecs-component` (§7.1) | — | ta, tc |
| **tm** | **MutationObserver climbs are tree climbs** (`Flip::Tm`). | `elidex-api-observers` `mutation/mod.rs` | as ta | — | ta |
| **te** | **Contenteditable inheritance is a tree climb** (`Flip::Te`), with `:read-write`. The last flip: it deletes the transitional accessor and `Flip`. | `elidex-ecs` `dom/mod.rs`; `elidex-dom-api` `element/inheritance.rs`; `elidex-css` `selector/matching.rs` (the `:read-write` arm); `elidex-js` `vm/host/html_element_proto.rs:663` (`isContentEditable`) | as ta | — | ta for its own flip; tr, tf, tm for the deletion |
| **ts** | **Stop appending.** Attach writes the link pair, bumps the host's version, and links nothing; elidex-ecs refuses to link a shadow root as a node; the accessor-layer skips and `tree_parent`'s marker test are deleted; `shadow_including_parent` reads `shadow_host`; `pending_flat_tree_order` places a shadow tree just after its host and a host's `::before` node before it (§2 ORD × RENDER); the P-REL lines (41 at `bfc63658`, plus `EcsDom::insert_as_first_stored_child`'s; re-measured at `tc`'s head) are re-read against R-TREE; `EcsDom::insert_as_first_stored_child` is deleted and `pseudo.rs` (and its fixture) go back to `insert_before` / `append_child` before the first tree child (§1.3); the dead node half of the fire guards goes; the docs of §0.1 item 2 say one thing. Behaviour-preserving for script; one residual reaches the user through hit-testing (§3.1). Closes the slot. | `elidex-ecs` `dom/shadow.rs`, `dom/mod.rs`, `dom/tree/*.rs`, `components.rs`; `elidex-style` `pseudo.rs` | `#11-shadow-root-is-a-tree-root` (closes) | — | te |

### §5.1 Dependency order and shared cells

The table's last column is a **dependency** order, not a landing order:

- tp-1, tp-2 → ta. tp-3, tp-4 → tc.
- tq → ta (PM 820: tp → tq → ta). `tq` edits the same function as `ta` (`next_in_preorder_global`, whose upward
  step `ta` re-types), and `ta`'s acceptance assumes the Range path is closed.
- tsel → tq (erratum, tsel plan PR, PM 884): the order is tp → tsel → tq → ta; `tsel` ([2026-11-tree-model-tsel-selection-boundary-preconditions.md](2026-11-tree-model-tsel-selection-boundary-preconditions.md), slot `#11-selection-boundary-setter-skips-root-check`) makes Selection operations check the spec's tree preconditions (PM 890), and `tq`'s cross-root Selection `toString` cell expects `""` on that base.
- ta → tc, tm, and `te`'s own flip. (Outside this program: ta → the DOM-walker program's slice 1, PM 761.)
- tc → tr, tf.
- tr, tf, tm → te's deletion of the accessor and `Flip`; te → ts; ts → PR (io), rewritten on this base (§7.2).

So after `ta`, the slices `tc` and `tm` are independent of each other, and after `tc`, the slices `tr` and `tf`
are; they may be reviewed and landed side by side (the shape PM 747 assumed). Which run in parallel and when each
lands is the PM's schedule, not this memo's: the re-estimate went to the PM with the R2 report, and §7.2 points to
it.

- `tsel` then `tq` first among the behaviour slices (erratum, PM 884): `tq`'s two accessor swaps close the closed-root exposure of §0.1 item 6
  about a week before `ta` (PM 790 (3): `ta` ~12-01; PM 820).
- `ta` next: it creates the vocabulary and the compiler's lists. `tp-1` before it, because `ta` edits ten lines of
  `traversal.rs`.
- **`tr` and `tf` depend on `tc`.** Draft 1 said the flip slices share no cell; R1 showed three contacts. (1) Range
  and traversal read both sides: `tc` owns the downward cells ("does not enter a shadow tree through a child
  list") and `tr` the upward ones. (2) Form descendant walks and form ancestor climbs are two halves of one fact:
  `tc` downward, `tf` upward. Between `tc` and its partner slice a family is half-flipped (it no longer enters a
  shadow tree downward but still leaves it upward); `tc`'s plan lists, per family, the cross-boundary case that is
  inconsistent in that window and probes that it is wrong in value only (no panic, no corrupted link). Draft 3.2
  claimed that without an exception; R3 found one: Range's contained-node walk corrupted the host's links through
  `deleteContents` until `tc`. `tq` closes it (§1.6, the second path), so from `tq` the claim holds. The shorter
  that window the better, which is an input to the PM's schedule. (3) The order function: resolved in `ta` (§2
  ORD × DER, ORD × FORM), so `te` and `tf` no longer share it with `tr`.
- `tm` depends only on `ta`. `te`'s flip depends only on `ta`; it comes after the other flips because it deletes
  the accessor (PM 748 (b): its DoD names the handlers). If `te`'s flip is wanted earlier, the deletion moves to
  whichever flip lands last; that is a schedule choice and changes no cell.
- `ts` last: every reader states its relation and none depends on which storage is underneath.

**Why the flips are their own slices and not part of the storage PR.** Each flip cluster has its own spec rows,
cells and neighbouring slots; bundling them with `ts` would make one PR change the storage and four families of
script-visible behaviour. `te` was five families in draft 1; after the lookups of census C-8, four of them are
not flips (CSSOM View and the hidden-subtree test are flat-tree relations, presentational hints are already
guarded, tree order is decided in `ta`), and one family remains.

**Why `tc` is separate from `ts`.** The child side has its own population and observable fixes; moving the skip
into one accessor layer first means `ts` deletes a layer instead of editing every caller (§7.3 Q-1).

### §5.2 The probe gate

Every "P:" row and every "T, g" / "T1" equality of the census is a reading of the code.

- **When.** A probe is run at the plan's base **before** that plan's `/elidex-plan-review`, and its results (the
  script, the observed value, the predicted value) are in the plan. A plan with an unrun probe is not reviewable.
- **`ta` has probes too**: one per row whose equality is not "the callee is the same function": the guarded rows
  (`closest`, the cellpadding walk, `:root`, the strict form pointer, `root_entities`), the receiver proofs
  (ChildNode methods, `outerHTML`), the order rows (census C-8 R-9), and the shadow-including rows
  (`is_in_document`, the style root scan). Each is run at `ta`'s base and again at its head.
- **Receiver or argument.** A "T1" row is equal at base only if the entity whose parent is read can never be a
  shadow root. For a receiver that follows from the interface; for a node a script passed in it does not (R2:
  five "T1" lines were arguments, §1.6). `ta`'s plan states, for every "T1" line, which of the two it is, and
  probes every argument line with a shadow root.
- **`tq` probes its change** at its base: the `deleteContents` probe of census C-14 (second row), `extractContents`
  over the same host, and the **`toString` facet** (D2), through both natives (`Range.prototype.toString`,
  `Selection.prototype.toString`), with a range whose end container follows the host in tree order (§1.6; the
  `deleteContents` shape ends in an ancestor of the start and meets the pre-existing overshoot instead). Each is
  run again at its head.
- **`tc` probes the walks** of census C-13 that it leaves on tree children (§1.3).
- **A miss toward "no change"** (a predicted flip where base already does not cross): the line takes `tree_parent`
  in `ta` instead, and the flip slice shrinks; if a whole cluster empties, the PM decides at that plan's review
  whether the slice merges into a neighbour (PM 747 Q-5).
- **A miss toward "change"** (a predicted equality that differs at base): the line does not take the final name in
  `ta`. It takes the transitional accessor with the `Flip` variant of its family's slice; if no slice owns that
  family, the miss is reported to the PM at `ta`'s review with the probe (a new slice or a neighbour is the PM's
  call), and `ta` does not land with a behaviour change other than its one of §1.6.

### §5.3 Acceptance

- `tp`: no test changes its expectation; no file of a split exceeds 1000 lines.
- `tq`: its per-PR plan has passed `/elidex-plan-review` (§5 `tq`); `next_in_preorder_global` reads no raw child
  or sibling (its upward step unchanged); the probes of §5.2 ran at its base and head. No existing cell changes its
  expectation except the one declared change of §1.6, the second path: the `deleteContents` cell (closed mode: links,
  child list, no `MutationRecord` naming the shadow root), the `toString` facet cell through both natives (a range
  whose end container follows the host), the `extractContents` cell, and a mutant that restores a raw step, on the
  final head; any existing Range cell whose expectation includes a shadow tree's node is listed with its new value.
  The `toString` overshoot (§1.6) is not fixed (its output differs only where the overshooting walk passes a host).
- `ta`: `get_parent` does not exist; the plan's per-line table covers every line the compiler reported (P-UP and
  P-UP-T, re-measured at its base), every census C-6 row and every census C-12 row. No existing cell changes its
  expectation, with two listed exceptions: the cells of §1.6 for the first script path (new, with a mutant per
  site, on the final head), and
  a dom-api handler test that pins the disagreement of an **unreachable** handler with its VM native (PM 748 (b);
  reachability from census C-10; each test is listed). A reachable handler's cells do not change. The pending
  counts per name and per `Flip` variant are recorded in the allowlist; the benchmark of §1.1 is in the plan.
  elidex-ecs's `is_child_of` and the three mutators are unchanged (PM 800); pre-remove step 1 is in the new
  `ensure_pre_remove_validity` beside `ensure_pre_insertion_validity` and `ensure_replace_validity`, called only by
  the `RemoveChild` handler, and `apply_remove_child` is unchanged (§1.6). The comment at `node_proto.rs:722-725`
  no longer says the engine rejects a shadow root "naturally".
  **Before `ta` lands**: the §1.5 selector relation cites its owner `#11-shadow-tree-style-isolation` (PM 794), and
  the landing list of §7.4 is with the PM, and `tq` has landed (§5.1). (`#11-shadow-root-accepted-as-ref-or-old-child`
  is registered, PM 790.)
- `tc`, `tr`, `tf`, `tm`, `te`: probes run before review; each changed behaviour has a cell and a mutant on the
  final head; the slice's `Flip` variant and its `_pending_flip` names do not exist at its head. **`tc`**: no old
  child-side accessor name exists; the three walks of §1.3 have their cells (§2 CHILD × SHELL, CHILD × RENDER),
  among them "the focusables of a shadow tree come right after the host" (HTML §6.6.2 steps 2.2 and 2.3, [H15]);
  "a host with a negative tabindex contributes no focusable of its shadow tree" (§2 CHILD × SHELL); the two
  `insertAdjacentHTML` callers of §1.6 read tree children and have their cells, at session level; the layer-6
  cells of §2 ORD × RENDER (`tc`) for the stored position they change, and for a positioned `::before`
  (unchanged); `sibling_order` reads the stored link, and `::before` generation calls
  `EcsDom::insert_as_first_stored_child`, whose own cells (elidex-ecs: a host whose shadow root is stored first; a
  parent with no child) pin the stored position and that the insert bumps the parent's version and dispatches `MutationEvent::Insert` as `insert_before` / `append_child` do (§1.3).
  **`tc` and `ts`**: a test that pins the (Y) storage shape (a shadow root in its host's child list, e.g.
  `crates/script/elidex-script-session/src/mutation/tests.rs:487-493`, which asserts `get_last_child(host)` is the
  shadow root as the premise of its case) is rewritten to the (X) expectation of the slice that breaks it and
  listed in that slice's plan; it is deleted only where another cell already covers the same case. This is not a
  script-visible expectation change.
  **`te`** (or the last flip, §5.1): the transitional accessor and `Flip` do not exist, and no dom-api handler is a
  caller of a transitional name (PM 748 (b)).
- `ts`: no tree edge between a host and its shadow root, and the three mutators refuse one as a node; no marker
  test of census C-5 role K, R or B remains (roles G and Ls stay, by §1.5); attach moves the document root's
  version; `EcsDom::insert_as_first_stored_child` does not exist and `pseudo.rs` inserts `::before` before the first
  tree child (§1.3); no script-visible cell changes its expectation; **the residual of §2 ORD × RENDER has its cell, for
  paint and for the hit-test target**, and a positioned `::before` still sorts before the shadow tree (its cell,
  same two targets; css-pseudo-4 §4.1 [P1]); the census is re-run at its head and every population's remaining lines
  are accounted for; the four pending names of §1.5 are the only pending names left.
  - **The link fields' visibility.** `ts` narrows them from `pub(crate)` to the `dom::tree` module **if** the P-REL
    command at its head prints no reading line outside `crates/core/elidex-ecs/src/dom/tree/`. At `bfc63658` three
    do (`dom/mod.rs:756`, `:909`, `:943`: `rev_version` and the two iterators); `ts`'s plan moves each behind a
    `dom::tree` function or names the one that cannot move, and in that case the fields stay and the plan says why.
  - **The drifted cites of `shadow.rs`** are corrected in `ts`'s plan (targets looked up with webref, R3 F3; the
    lines are at `bfc63658`). `:58` and `:226` cite DOM §4.2.2.5 (Signaling slot change) for the validation of
    `slot_assign`, which implements HTML §4.12.4 The slot element, `assign(...nodes)` (`#dom-slot-assign`) [H16].
    `:236`, `:290` and `:351` cite "§4.2.2.5 step 3" for removing a node from another slot's manually assigned
    nodes, which is that method's step 3.1 ("If node's manual slot assignment refers to a slot, then remove node
    from that slot's manually assigned nodes.") [H16]; *manually assigned nodes* is HTML's (§4.12.4), and *manual
    slot assignment* is DOM §4.2.2.2 Slottables [D14]. `:381` cites DOM §4.2.2.5 for *find slottables* and the
    `assignedNodes()` algorithm: *find slottables* is DOM §4.2.2.3 Finding slots and slottables [D14], and
    `assignedNodes()` is HTML §4.12.4 The slot element [H16]. `:426` cites DOM §4.2.2.4 for *find a slot*, which is
    DOM §4.2.2.3 [D14].
- Program: R-TREE holds; §0.1 items 1 to 6 are gone; PR (io)'s "one home" sentence is true for the shadow root
  before PR (io) is written against it.

## §6. The census, and how each population maps to slices

Populations are defined by what the code does; commands, counts, controls and tables are in the census file.
Summary at `bfc63658` (non-test unless noted; re-measured 2026-11-18, every count equal to the one at `84fee8fa` and at `c0578223`):

| Population | Property | Count | Consumed by |
|---|---|---|---|
| P-UP | reads the stored parent through the public accessor | 143 lines (142 calls + the definition); 31 more under test paths (verified 2026-11-18, §8) | `ta`: all, by compile error (census C-2, predicted: T / S / H 61, of which 5 are the child-of question of §1.6; raw 4, inside a removed helper 4, pending-flip 41, F 21, E 4, L 7, and the definition) |
| P-DER | a read-only function that climbs, or calls one | level 1: 60 (of 114 climbing functions); level 2: 45 (of 113) | `ta` (§1.2) |
| P-REL | reads or writes a `TreeRelation` link field directly | 41 lines, all inside elidex-ecs | `ts`; `ta` for the climbs among them |
| P-WR | writes a parent link | one site, three callers | `ts` (the refusal) |
| P-CH | reads a raw child / sibling link | 108 lines; 34 more under test paths (verified 2026-11-18, §8) | `tc`, by compile error |
| P-CHX | reads through a compensating child-side accessor | six accessors (census C-4); with P-CH, 314 lines | `tc`, by compile error (§1.3); `ts` deletes the skip |
| P-CHW | a function that reads a raw first or last child and a raw sibling (it can walk a raw child list, the only place a shadow root is a child) | 25 functions of 54 that read a raw link; 23 outside elidex-ecs (census C-13) | `tc`: each re-read; three take the explicit descent (§1.3) |
| P-VM | a line under `vm/host` that calls a P-UP, P-CH / P-CHX or P-DER member | 58 lines in 25 files (verified 2026-11-18, §8): UP 4, CH 10, DER 46 (census C-12) | `ta` (its lines), then the flip slice of each PF member (§5 module lists) |
| P-NAV | a registered dom-api handler whose body calls such a member, with its reachability from production | 39 of 217 handlers: 21 reachable, 18 unreachable (census C-10, C-12) | `ta`, `tc` (§2 UP-T × NAV, CHILD × NAV) |
| P-MARK | tests or reads the `ShadowRoot` component | 63 lines (61 true): K 11, R 2, B 7, G 1, Ls 2, C 6, E 2, L 2, I 15, D 13 | K, R → `tc`; B, C → `ta`; E (node half) → `ts`; L → the lifetime program; G, Ls, I, D stay |
| P-HOST | reads the backward link field | 11 reading lines in shadow-related files | `ta` routes 7 through `shadow_host`; 3 stay inside elidex-ecs (census C-5) |

**What the census changed about the plan.** (1) Parent and child sides are separate populations with separate
instruments (`ta`, `tc`). (2) Direct field access cannot exist outside elidex-ecs, so `ts` is confined to one
crate. (3) Derived relations are a population of their own, found from function bodies; a hand list missed one
with 16 callers (verified 2026-11-18, §8; found by R1). (4) Thirteen rows that draft 1 left unclassified resolve to three relations this program does
not define, one new flip family and several equalities (census C-8). (5) One site writes a parent, so the
structural guard has one home. (6) The VM-host lines and the handler pairs are found by a script and no longer by
hand; the hand list of draft 2 missed 13 lines in 11 files (R2; each is a row of census C-12, verified
2026-11-18). (7) A "T1" equality holds for a receiver, not for a node a script supplied (R2; §1.6). (8) Some raw
child walks are correct only because they cross (§1.3).

**Ratchet.** After `ta` the population "unqualified parent read" is empty by construction, and the pending
populations can only shrink (the trip-wire). After `tc` the population "child-side read without a relation" is
empty the same way. After `ts`, direct field access is confined by visibility; whether the fields narrow from
`pub(crate)` to the `dom::tree` module follows the criterion of §5.3.

## §7. Slots, hand-offs and open questions

### §7.1 Slots

| Slot | Why deferred | Trigger | Re-eval |
|---|---|---|---|
| `#11-shadow-root-is-a-tree-root` (registered, PM 747; owner L3) | the storage cannot change before every reader names its relation (§0.2), so the two homes of §0.1 item 1 stay until the last slice | **this umbrella's ratify**; closed by `ts` | 2026-12-31 |
| `#11-shadow-root-accepted-as-ref-or-old-child` (registered, PM 790; owner L3; paydown (c)) | §0.1 item 6: wrong in production today. Scope widened by R3 (F1) and PM 812 to "script paths that unlink a host's shadow root": the reference / old / removed child, and Range deletion, which also hands a closed shadow root to page script through a `MutationRecord`; the ledger row was re-synced to draft 3.3 by PM 814. **Closes in two steps (PM 820)**: the Range path at `tq` (the walk takes tree children, §1.6) and the reference / old / removed child at `ta` (the spec's child-of steps in the engine-independent layer, §1.6, PM 800); the ledger text follows the split (§7.4) | `tq`, then `ta`; closed by `ta` | 2026-12-31 |
| `#11-shadow-tree-style-isolation` (Layout, CV-details; owner of the selector relation by PM 794) | owns `pending_selector_parent` (§1.5): the featureless host in the combinator walk is built by CV-details | the slot's own | the slot's own |
| `#11-dom-selector-api-rejects-shadow-pseudos` (registered, PM 794; owner L3) | neighbour: `querySelector` / `matches` reject `:host` and `::slotted` with a SyntaxError in every tree; not a relation, so not this program's | `ta`'s landing or `#11-shadow-tree-style-isolation`'s landing, whichever is first | 2027-01-15 |
| `#11-element-lifecycle-hooks` (P1) | owns the shell's link and label lookups (activation target, DOM §2.9), which carry `pending_event_parent` after `ta` (PM 759) | the slot's own | the slot's own |
| `#11-template-contents-host-link` | not this program's (its Re-eval-date 2026-12-15 in the ledger comes before PR (io), ~12-18 by PM 790 (3), and so before its close: re-dated at this umbrella's ratify, §7.4); closes at a small PR after PR (io) (PM 748 (a), amending PM 718), which extends the one host-including test. `ta` creates that test (`is_host_including_inclusive_ancestor`), which fires the slot's facet about the test; the disposition is already PM 748 (a), and `ta`'s plan records the firing | that PR | the slot's own |
| `#11-dom-api-handlers-unreachable-from-production` (registered, PM 744 / PM 583) | its trigger (b) is PM 748 (b): in `ta` the handlers take the VM natives' relations; connecting or deleting them is decided there | the slot's own | the slot's own |
| `#11-shadow-slot-assignment-flat-tree` (P2, Layout) | owns the flat-tree names of §1.5 and the *assigned slot* arm of the event parent; and, for the Tab order after `tc`, the differences from HTML §6.6.2 that `tc` leaves: a slot as a focus navigation scope owner, and tabindex order inside a scope (PM 800, Q-18; confirmed by Layout, ledger row "Added (PM 800, 2026-11-18, Layout)"); among the slot-scope differences, the light children of a host with a negative tabindex, which `tc` walks as base does (HTML §6.6.2 *associated focus navigation owner* step 2, [H15]; §2 CHILD × SHELL). Not among them: that host's shadow tree, which `tc` skips. **One observation for P2** (D1; not decided here, §7.4): a generated `::before` / `::after` box is stored as a light child of its originating element; whether a render-side fact belongs in the node tree at all is an ECS-native question for P2. That such a node is never a slottable is already the row's P2-1 ("pseudo-element entities never slottables, their flat-tree parent the originating element", K4), and `slot.assignedNodes()` possibly returning the `::before` text entity is an instance of it (unprobed: the generated node is a `NodeKind::Text` entity, `create_text_with_owner`, `dom/mod.rs:568`, and `EcsDom::assigned_nodes`' walk over the host's children, `crates/core/elidex-ecs/src/dom/shadow.rs:436-443`, matches every Text child to the default slot); §7.4 (N8) records it as such | the slot's own | the slot's own |
| `#11-event-path-window-missing` (L3) | owns `pending_event_parent` in `build_propagation_path` (§1.5) | the slot's own | the slot's own |
| `#11-capped-walker-deep-element-processing` | its trigger (an edit of the walkers in `walkers.rs`) is fired by `ta` and `ts` (R1 named ta / te / ts under draft 1's cut, where `te` held `ancestor_chain`); each plan records the firing; this program changes no bound and no walk order | fired | the slot's own |
| `#11-dom-walkers-iterative-over-ecs-links` | the DOM-walker program (recursive walkers to an iterative form); both edit `walkers.rs`. Its slice 1 lands **after `ta`** and may run beside `tc` and later slices; it still stacks on PR (i) and uses `owned_entities` (PM 761, PM 694). `ta` does not wait for it. L3 sets the slice's date and tells the PM if it would pass 12-15 (PM 761 (1)) | the slot's own | 2026-12-15 |
| `#11-shadow-tree-live-range-tracking` | text: live-range adjustment across a shadow-tree boundary; it is where the parent half of the fire guards belongs (§2 MEV × TREE). `tr`'s plan states whether `tr` fires its trigger ("Shadow DOM PR demand") and narrows it | tr's plan-review | 2026-Q4 (the date is in the roadmap row only; the ledger row carries the name alone; raised to the PM with the R3 report) |
| `#11-shadow-mutation-observer-disconnected-cleanup` | re-triaged by PM 765 (2): its premise (a host wrapper sweep) is stale, and the registration is now the `MutationObservedBy` component, so the defect is an observer that stays on a shadow root nobody reclaims. That is reclamation: **absorbed by the ecs-entity-lifetime program's PR (ii)** (PM 790 (4)), not by this program. `tm` changes which ancestors collect, not a registration's lifetime | PR (ii) | 2026-12-31 |
| `#11-range-compare-points-shadow-index` | D13: closed by `tc` (PM 747 Q-2: RC-1 drops its D13 part; PM 758 Q-16); the ledger's closing slice changes (§7.4) | `tc` | 2026-12-01 in the ledger, which lapses before `tc` lands (`ta` ~12-01, PM 790 (3)): re-dated at this umbrella's ratify (§7.4) |
| `#11-live-range-bridge-wide-sibling-index` | neighbour of `tc`: the index's sibling cap; `tc` moves the index into the set and does not change the cap | the slot's own | 2026-12-01 |
| `#11-form-owner-ecs-component` | `tf` edits `find_form_ancestor`, one of its three resolvers: **trigger-fired record in `tf`'s plan**; the `form`-attribute half and the component stay with the slot | fired by `tf` | the slot's own |
| `#11-form-elements-cross-tree` | association across trees by explicit mechanisms; `tf` only stops the ancestor step from crossing, and its plan cites the slot so the two do not contradict | the slot's own | M4-13 entry |
| `#11-fieldset-disabled-dynamic-insert` | neighbour of `tf` (re-deriving inherited disabledness on insert); `tf` changes the relation the derivation uses, not when it runs. Decided by PM 791: paydown (c), closed by the reconciliation umbrella's Slice 2 (`docs/plans/2026-07-form-derived-state-reconciliation.md`). Its Slice 1 adds `is_effectively_disabled` (NEW there; defined in that umbrella's §3.1, its consumers in §4.1), a climb of the fieldset ancestry that `tf` flips: a merge contact (§7.2) | reconciliation Slice 1's landing + Slice 2's plan-review | 2027-01-31 |
| `#11-vm-isconnected-bound-document` | `ta` edits the native (`node_proto.rs:455`): its trigger ("L3's next DOM node-surface touch") fires. The fix is a small PR after `ta` (the native calls `is_connected`), not part of `ta` (PM 758 Q-14) | fired by `ta` | re-dated to `ta`'s landing (§7.4) |
| `#11-attach-shadow-non-html-namespace` (registered, PM 768 (a); owner L3) | neighbour of `ts`: `attach_shadow_with_init` ignores the namespace (DOM *attach a shadow root* step 1), and the slot also folds the duplicated declarative-shadow-root attribute parsing. `ts` rewrites the same function for the link pair and adds no namespace check; whichever lands second rebases | the slot's own | 2026-12-31 |
| `#11-shell-effective-disabled-predicate-adoption` | L4's; `ta` renames two lines in `content/form_input.rs` and adopts nothing | the slot's own | the slot's own |
| `#11-shadowroot-detached-not-despawned-on-host-destroy` | the lifetime program's; `ts` leaves the stripped root as it is | PR (ii) | the slot's own |

Every pending name of §1.5 now has an owner slot (the selector relation by PM 794).

### §7.2 Hand-offs

- **Schedule.** The slice count (the splits, `tq`, `ta`, six more) is larger than the estimate PM 745 was given. The
  re-estimate, the parallel shape and their effect on the window below went to the PM with the R2 report; dates
  live in the PM's record (PM 790 (3): ratify ~11-23, `ta` ~12-01, PR (io) ~12-18, T3 ~12-22 or later; G9 is
  reported by L3 at `ta`'s landing), not in this memo. Two date rules bind this program: PM 761 (1) (the walker
  slice's date against its slot's 12-15, §7.1) and the cap of PM 740 / PM 741 (below).
- **ecs-entity-lifetime program** (L3; branch `ecs-shadow-root-lifetime-plan`). After `ts`: (1) PR (io) is
  rewritten. Its back-link folds `ShadowRoot.host`. The fold edits `shadow_host`'s body, the one writer in
  `attach_shadow_with_init`, and the three readers inside elidex-ecs that hold the component for another reason
  (census C-5: `shadow.rs:267`, `:421`, `teardown.rs:69`); it edits no climb and no other crate. (2) The "host"
  cluster leaves PR (io) for the small follow-up PR that closes `#11-template-contents-host-link` (PM 748 (a));
  that PR widens the host-including test, whose callers are listed in §1.2. (3) That umbrella's order line gains
  this program, and its next revision cites `#11-shadow-root-is-a-tree-root` (PM 747). (4) PR (ii) absorbs
  `#11-shadow-mutation-observer-disconnected-cleanup` (§7.1; acknowledged, PM 790 item 4). **PR (ii)'s plan DoD
  gains a check**: does the observer side hold target entities separately (outside the `MutationObservedBy`
  component), and if so, are they released when the shadow root is reclaimed.
- **Layout T3 / T3-pre** (L2). T3-pre lands after PR (ig); its FragmentParse arm is a no-op over the window
  ig → T3-pre → [this program] → io → T3, under the cap PM 740 / PM 741 set. **If this program's landing would
  push T3 past that cap, L3 escalates to the PM.** Layout's Q-10 answer: T3 / T3-pre add no parent climb and no raw
  child read; the seven existing functions they rely on are assigned in §2 (items 1, 2, 4, 5, 6, 7) and census C-2
  (item 3, `despawn_subtree`, raw, `ts`). The T3 memos landed on main (#638) and quote this program at draft 2
  (pinned to `1593a5ad`): `docs/plans/2026-11-eh-attrs-t3-pre-subtree-created-event.md` (its §8.1 and §8.2: the
  slice order, and its test U-T13 as a `get_parent` caller that `ta` re-types to the tree parent),
  `docs/plans/2026-11-eh-attrs-t3a-applicability.md` (H-A9) and `docs/plans/2026-10-eh-attrs-t3-target-determination.md`.
  Their pin keeps them true; what changed since is the dependency wording of §5.1, the `tp` parts and §1.6, none
  of which they rely on. The re-pin has one home, Layout's
  `docs/plans/2026-10-event-handler-attrs-at-creation.md` §8.5 *Re-pin at L3's ratification* (PM 778 / 779; it
  cites `1593a5ad` four times); L3 tells Layout when this draft is ratified, and that item moves the pins.
- **Layout: the Tab order (PM 800, Q-18).** Confirmed: P2 (`#11-shadow-slot-assignment-flat-tree`) owns the
  differences from HTML §6.6.2 that `tc` leaves (ledger row of the slot, "Added (PM 800, 2026-11-18, Layout)";
  §7.1). The negative-tabindex host is not one of them: `tc` skips it (§2 CHILD × SHELL), and Layout is told with
  item (a) below.
- **Layout: what it is told, and when.** (a) Before **`tc`'s plan review**: `tc` renames `children_iter` and every
  other child-side accessor, so every layout, render and style reader is on the compiler's list: 10 lines in 5
  files under `crates/layout`, 3 lines in 2 files of `elidex-render`, 15 lines in 5 files of `elidex-style`
  (verified 2026-11-18, §8; Layout confirmed these counts, and that `composed_children` (152 uses) is unchanged,
  at `1146380f`).
  `composed_children` keeps its name and its result. `save_descendant_styles` changes its body (§1.3). Layout's
  own defect (a positioned node in a shadow tree under a host that is not a stacking context never reaches layer
  6, through `children_iter`) is Layout's; `tc` must not change what `children_iter`'s successor returns there.
  **Draft 3.3 (R3 F2), narrowed in draft 3.4 (D1)**: `tc` changes the layer-6 order in one case, and only on the
  session API. Its two `insertAdjacentHTML` insertions ("afterbegin" / "afterend") store their node after the
  host's shadow root where base stored it before (when the shadow root was the host's first stored child or the
  target's raw next sibling), and `sibling_order` keeps reading the stored order until `ts` (§1.3), so a positioned
  inserted node now sorts after a positioned node of the shadow tree: the order `ts` gives (shadow-including), the
  same class as the residual Layout accepted under Q-13, and not reachable from production (no VM native).
  **`::before` generation no longer moves** (draft 3.3 said it did): it keeps the stored first child until `ts`,
  so a positioned `::before` stays before the shadow tree, which is css-pseudo-4 §4.1's order [P1]. `tc` carries
  the cells (§2 ORD × RENDER, with the stacking-context host and default slot as preconditions); neither
  `sibling_order` nor `::before` generation takes the one set (`pseudo.rs` calls the one new elidex-ecs write
  `EcsDom::insert_as_first_stored_child`, draft 3.5, §1.3; Layout's style count above is unchanged by it).
  (b) Before **`ts`'s plan review**: `tree_order_cmp` is a function Layout relies on (the same kind of reliance as
  Q-10's seven; consumers `walk.rs:613-624`, `hit_test.rs:228-239`), it becomes `pending_flat_tree_order` in `ta`
  with no change, and `ts` changes its answer for the residual of §2 ORD × RENDER, while keeping a host's `::before`
  before its shadow tree (how `ts` tells `::before` from `::after` is `ts`'s plan's, §2 ORD × RENDER); `ts` also
  changes `fire_after_insert`'s suppression (item 7). (b′) For P2, one observation recorded on
  `#11-shadow-slot-assignment-flat-tree` (§7.1, §7.4): whether a generated box belongs in the node tree at all (an
  ECS-native question). `assignedNodes()` possibly returning the `::before` text entity (unprobed) is an instance of
  the row's P2-1 / K4 ("pseudo-element entities never slottables") and is cited as such, not added.
  (c) When `ta` / `tc` rename the accessors T3's plan cites.
  (d) `tp-3`: Layout confirms the seam of `builder/mod.rs` before the split PR is cut (PM 758 Q-15).
- **Range contents program** (L3). RC-1 lands after `tc`, drops its own D13 part and uses `tc`'s accessors
  (PM 747 Q-2). **RC-1's plan revision gets a re-review of its changed sections** before its code.
- **Event-path plan** (`docs/plans/2026-11-event-path-window.md`). `ta` re-types the loop at
  `crates/script/elidex-script-session/src/event_dispatch/mod.rs:203-221` to `pending_event_parent`; that plan
  replaces the name with its *get the parent* function. Whichever lands second rebases one line.
- **DOM-walker program.** Slice 1 after `ta` (PM 761); the two edit `walkers.rs`, and the later one rebases.
- **VM P4 / R-NAT.** The contact is code, in R-NAT's host-chapter implementation: this program's VM-host lines are
  the 58 of census C-12, and in `crates/script/elidex-js/src/vm/host/range_proto.rs` they are three lines for `ta`
  and `tr` (`:432`, `:529`, `:783`) plus two callers `tr` lists (`:749`, `:768`). The local branch
  `vm-native-construct-plan` (not on origin; its tip moves, so it is named by branch) holds a plan and no code under
  `crates/` (`git diff --name-only bfc63658...vm-native-construct-plan -- crates` prints nothing on 2026-11-18), so there is nothing to rebase today; whichever implementation lands second rebases. When the two
  overlap is in the PM's schedule. The null-parent step that stays in the host until R-NAT is named in §5 `tr`.
- **Merge contacts** (branches that edit this program's files): `element-localname-ns`
  (`crates/dom/elidex-dom-api/src/tree_nav.rs`, `node_proto.rs`, `crates/core/elidex-ecs/src/dom/classify.rs`;
  `git diff --name-only bfc63658...element-localname-ns`); the reconciliation umbrella
  (`docs/plans/2026-07-form-derived-state-reconciliation.md`): its Slice 1 adds `is_effectively_disabled`, a
  fieldset-ancestry climb; landing after `ta` it takes the relation `ta` leaves on that climb (`Flip::Tf`) or, after
  `tf`, the tree climb, and whichever of Slice 1 and `tf` lands second rebases; the serializer program (plan only on
  `html-serializer-escape-conformance`; its code will walk children, so it lands on `tc`'s set or is rebased by
  it); `#11-attach-shadow-non-html-namespace` (the same function as `ts`, §7.1).
- **Shell (L4, no driver).** `ta` renames lines in `app/events.rs`, `app/hover.rs`, `content/scroll.rs` and
  `content/form_input.rs` (pending names, no behaviour), and one helper line of
  `crates/shell/elidex-shell/src/tests.rs` (`tp-2`). `tc` edits `content/event_handlers.rs` (`:497`, `:500`) and
  `content/iframe/lifecycle.rs` (`:449`, `:452`) with the descent of §1.3, and renames 6 lines in
  `elidex-navigation` (`loader.rs`, `resource.rs`; they read through an accessor that already skips, so no
  behaviour). No flip slice edits a shell file. L3 makes these edits; with no L4 driver, the PM is told at each
  plan's review.
- **Brand-check program** (Rev 5.3, frozen). No dependency: its marker reads are identity (census C-5 role I).
- **Near the 1000-line bound** (each slice re-measures at its base and splits first if its edit would cross):
  `elidex-layout-grid` `lib.rs` 991, `tree_nav.rs` 972, elidex-ecs `dom/mod.rs` 959, the shell's
  `content/event_handlers.rs` 956, `range_proto.rs` 906, `components.rs` 856.

### §7.3 Open questions

Answered and written in above: Q-2, Q-3, Q-5, Q-6, Q-9 (PM 747); Q-10 (Layout); Q-4 (§2 MEV × TREE); Q-7 (census
C-8); **Q-11** (PM 758: L3 writes the trip-wire in `ta`'s PR, §1.4); **Q-12** (PM 759: P1
`#11-element-lifecycle-hooks`, §1.5); **Q-13** (Layout: shadow-including order until P2, §2 ORD × RENDER);
**Q-14** (PM 758: a small PR after `ta`, §7.1); **Q-15** (PM 758: L3 cuts the splits, Layout confirms its seam,
§5 `tp`); **Q-16** (PM 758: the close moves to `tc`; PM 765 (2): the observer slot, §7.1); **Q-17** (PM 761: walker
slice 1 after `ta`, §7.1); the selector relation's owner (PM 794, §1.5); **Q-18** and **Q-19** (PM 800, below).
Remaining:

- **Q-1 (plan-review).** `tc` puts the skip into the accessor set and `ts` deletes it, against folding `tc` into
  `ts`. This memo keeps the separate slice (§5.1).
- **Q-8 (plan-review).** Two named steps versus one step with a scope argument (§1.1).
- **Q-18 (closed, PM 800).** The shell's Tab order after `tc` (§2 CHILD × SHELL). The walk takes the explicit
  descent, and a shadow tree's focusables come right after the host. That is not an interim regression but a change
  toward the spec: HTML §6.6.2 *flattened tabindex-ordered focus navigation scope* [H15] step 2.2 ("If item is not
  a focusable area, then replace item with all of the items in item's flattened tabindex-ordered focus navigation
  scope.") and step 2.3 ("Otherwise, insert the contents of item's flattened tabindex-ordered focus navigation
  scope after item."), a shadow host being a *focus navigation scope owner*. The base position (where the shadow
  root sits in the raw child list) matches no step. The remaining differences (a slot as a scope owner; tabindex
  order inside a scope) are P2's, `#11-shadow-slot-assignment-flat-tree` (Layout; confirmed, §7.2). A host whose
  tabindex is negative is not expanded at all (R3; HTML §6.6.2 *tabindex-ordered focus navigation scope*, [H15]):
  `tc`'s descent skips its shadow tree, so that is not a residual; its light children, which `tc` walks as base
  does, are (P2's slot scopes, *associated focus navigation owner* step 2, §2 CHILD × SHELL). `tc`'s cells include "the focusables of a shadow tree come right
  after the host (§6.6.2 steps 2.2 / 2.3)" and the negative-tabindex host.
- **Q-19 (closed: driver, lens converged; approved PM 800).** §1.6: the elidex-ecs half of the child-of question.
  Option (b′): `is_child_of` and the three mutators stay storage maintenance (a stored-link read, population
  P-REL, §1.1) until `ts`; in `ta` the spec's steps are asked in the engine-independent layer (pre-remove step 1
  as a `tree_parent` question in a new `ensure_pre_remove_validity` in dom-api `child_node/mod.rs`, called only
  by the `RemoveChild` handler; the step-3 checks of `mutation/child_list.rs:284`, `:439` and the dom-api T2 lines
  take `tree_parent`). The three engine callers keep base behaviour in `ta`; two are `tc`'s and `::before` generation keeps a stored-order write until `ts`, from `tc` through `EcsDom::insert_as_first_stored_child` (§1.3, drafts 3.4 and 3.5). Lens, all rows
  agreeing: spec, the "NotFoundError" cases are met in `ta` through the script-facing path, and *pre-remove* is run
  by `removeChild` alone; Layering, the check is in the algorithm layer, not in elidex-ecs storage; *One issue, one
  way*, one predicate per spec step, asked at validation time by a validity function of dom-api
  `child_node/mod.rs` (pre-insert step 3, replace, pre-remove step 1) and re-checked at apply time by
  `apply_insert_before` (`child_list.rs:284`, whose callers are audited in §1.6) and `apply_replace_child` (`:439`)
  with the same `tree_parent` question, and one meaning for elidex-ecs (storage); fail-safe direction, an unknown internal
  caller keeps base behaviour instead of silently losing an insert, and the callers of `apply_remove_child` that
  run no *pre-remove* keep base behaviour; ECS-native, the storage relation and the algorithm relation are
  separate. Draft 3.2 (driver correction): draft 3.1 put the check inside `apply_remove_child`, the *remove*
  building block that `ChildNode.remove` / `replaceWith`, the select-option code and Range also call; per spec
  they run no *pre-remove*, so the check moved to the `removeChild` path. Rejected: (a) the tree-parent equality in `is_child_of` from `ta` (silently breaks the three
  callers' inserts, whose results are discarded, and any caller not read); a separate guard PR (a compensation `ts`
  deletes). Evidence: `git grep -n -P '\.(remove_child|insert_before|replace_child)\(' bfc63658 -- crates ':!*tests*'`
  (45 lines; census C-9; draft 3.2 printed 53 here, which is the count of the `\b` form of the pattern, not of this
  command). The `Mutation::RemoveChild` flush arm is the one remove path without the check; its reason and its
  owner slot are in §1.6. R3 (F1) found a second script path that never asks the child-of question (Range's
  contained-node walk); it is closed in `tq` by the walk (PM 820), not by this option (§1.6), and changes nothing
  above.

### §7.4 Landing items (handed to the PM with the slice; each is "old → new")

Every "old" below is quoted from the current ledger row (re-read 2026-11-18 for draft 3.4, after PM 814).

- **Applied: (N1)** `#11-shadow-root-accepted-as-ref-or-old-child`, the draft 3.3 re-sync (Defect "replace step 1";
  the Range path; "Declared as ta's two behaviour changes"): applied by pmo under PM 814. The row now reads
  "scope widened PM 812 (Range path, R3 Axis 2, probe at bfc63658)". Not a pending edit; (N6) below re-syncs it
  to the `tq` / `ta` split.
- **Applied: (N7)** `#11-range-full-impl`, the `to_string` facet (§1.6, D2): applied under PM 826 (the row now
  reads "closes when all three land and the to_string facet is fixed" and carries the facet; re-read 2026-11-18
  for draft 3.5). Not a pending edit.
- **At this umbrella's ratify (now items).**
  (N2) `#11-range-compare-points-shadow-index`: "closed by its home slice" → "closed by the tree-model program's
  `tc` (PM 747 Q-2, PM 758 Q-16)"; "Re-eval 2026-12-01 or the home slice's landing." → "Re-eval: `tc`'s landing"
  with a date the PM sets from its schedule (2026-12-01 lapses before `tc`: `ta` lands ~12-01, PM 790 (3)).
  (N3) `#11-template-contents-host-link`: "**Re-eval-date**: 2026-12-15" → a date after the small PR that closes it
  (after PR (io), ~12-18 by PM 790 (3)), set by the PM.
  (N4) `#11-tree-mutation-record-pipeline`, text gains the facet: "When removeChild is routed through
  record_mutation, the deferred-flush arm Mutation::RemoveChild becomes the production removeChild path; it must
  then ask DOM pre-remove step 1 (ensure_pre_remove_validity, tree-model umbrella §1.6) before recording, or
  re-check it in the arm, as Mutation::InsertBefore / ReplaceChild re-check theirs."
  (N5) `#11-shadow-tree-live-range-tracking`: the ledger row holds the slot name only, and its date (2026-Q4) is in
  the roadmap row alone; the PM decides whether the ledger gets a Re-eval-date (PM 489: deadlines live in the
  ledger).
  (N8) `#11-shadow-slot-assignment-flat-tree`, text gains two observations (D1 and the negative-tabindex MIN; not
  decisions; draft 3.5 drops what the row's P2-1 already holds: "P2-1 K4's cases, current (K4, K5, K11):
  pseudo-element entities never slottables, their flat-tree parent the originating element"): "(1) A generated
  ::before / ::after node is stored as a light child of its originating element (elidex-style pseudo.rs); whether
  this render-side fact belongs in the node tree at all is an ECS-native question for P2. (A known instance of
  P2-1 / K4, unprobed: slot.assignedNodes() may return the ::before text entity, since EcsDom::assigned_nodes
  matches every Text child of the host to the default slot, elidex-ecs dom/shadow.rs:436-443.) (2) Tab order
  after the tree-model program's tc: the light children of a host with a negative tabindex are walked as at base;
  HTML §6.6.2 associated focus navigation owner step 2 puts them in their assigned slot's scope, which is P2's."
- **At `tq`'s landing or at this umbrella's ratify, whichever is first.** (N6)
  `#11-shadow-root-accepted-as-ref-or-old-child`, re-synced to the split (PM 820), four quotes from the current
  row: "Fix (L3, in `ta` of the tree-model program; PM 800 (b′)):" → "Fix (L3, tree-model program; PM 800 (b′),
  PM 820): the reference / old / removed child in `ta`, the Range path in `tq` (before `ta`)."; "The Range
  contained-node walk takes the tree-children step (the existing accessor that skips a shadow root) in ta." →
  "The Range contained-node walk takes the tree-children step (the existing accessor that skips a shadow root) in
  tq; this also stops Range.prototype.toString and Selection.prototype.toString from including a shadow tree's
  text (probed in tq)."; "Declared as ta's two behaviour changes (PM 800 (b′), PM 812), each with a cell and a
  mutant." → "Declared as one behaviour change of tq (the Range path; PM 812's change, placed in tq by PM 820) and
  one of ta (the child-of steps; PM 800 (b′)), each with cells and mutants."; "Trigger / closed by: ta." →
  "Trigger: tq (Range path closed), then ta; closed by: ta."
- **At `tq`.** `#11-shadow-root-accepted-as-ref-or-old-child`: gains "Range path closed by tq (<sha>, <date>)";
  the row stays open.
- **At `ta`.** (1) `#11-vm-isconnected-bound-document`: "Re-eval: L3's next DOM node-surface touch; backstop
  2026-12-15." → "Re-eval: `ta`'s landing date" (the trigger fires at `ta`; PM 758 Q-14).
  (2) `#11-shadow-slot-assignment-flat-tree`, text: gains "owns the callers of `pending_flat_tree_parent` and
  `pending_flat_tree_order`, listed by the trip-wire's allowlist". (3) `#11-event-path-window-missing`, text: gains
  "owns `pending_event_parent` in `build_propagation_path`". (4) `#11-element-lifecycle-hooks`, text: gains "owns
  the shell's two callers of `pending_event_parent` (link and label lookup)". (5) `#11-shadow-root-is-a-tree-root`:
  the heading "L3 — tree model (registered 2026-11-15, PM 747; umbrella 379acc15)" → "umbrella 379acc15" becomes
  the ratified sha; and the row's existing Fix line, "Slices ta → tc / tm / tr / tf → te → ts; ordered before the
  ecs-entity-lifetime program's PR (io) (PM 730)." → "Slices tp → tq → ta → tc / tm → tr / tf → te → ts (umbrella
  §5.1; tq by PM 820); the four pending names of umbrella §1.5 (pending_flat_tree_parent,
  pending_flat_tree_order, pending_event_parent, pending_selector_parent) outlive it; closed by ts. Ordered before
  the ecs-entity-lifetime program's PR (io) (PM 730)." (6) `#11-shadow-root-accepted-as-ref-or-old-child`:
  "**open (registered 2026-11-18, …)**" → closed by `ta` (the Range path at `tq`, the reference / old / removed
  child at `ta`; PM 820). (7) `#11-dom-selector-api-rejects-shadow-pseudos`: its trigger ("tree-model `ta` lands")
  fires. Draft 3.2's item "`#11-shadow-tree-style-isolation` gains the owner of `pending_selector_parent`" is
  dropped: the row already holds it ("PM 794 (2026-11-18): L3 の tree-model umbrella の pending_selector_parent の持ち主").
- **At `tc`.** `#11-range-compare-points-shadow-index`: closed.
- **At `ts`.** `#11-shadow-root-is-a-tree-root`: closed.

## §8. Measurements (provenance, at `bfc63658`, run 2026-11-18)

All counts are reproduced by the commands in the census file (C-1 to C-14); the ones this memo quotes. Each was
run at `bfc63658` and at `84fee8fa` for draft 3.3 with the same count (and at `c0578223` for drafts 1 and 2); the
scripts of C-11, C-12 and C-13 print the same rows at both revs (`diff` of the saved outputs: empty).

- P-UP: `git grep -n -P '\bget_parent\(' bfc63658 -- crates ':!*tests*' | wc -l` → 143; over `-- crates` → 174; 60
  non-test files, 21 crates.
- P-DER: the script of census C-11 → "level 1: 114 read-only 60 mutating 54", "level 2: 113 read-only 45 mutating
  68"; `is_ancestor` at level 1, `is_ancestor_or_self` at level 2.
- `git grep -n -P '\bis_ancestor_or_self\(' bfc63658 -- crates ':!*tests*' | wc -l` → 18 (the definition, one
  comment, 16 calls).
- P-CH: `git grep -n -P '\b(get_first_child|get_last_child|get_next_sibling|get_prev_sibling)\(' bfc63658 -- crates ':!*tests*' | wc -l` → 108; over `-- crates` → 142.
- P-MARK: `git grep -n -P '::<&?(\w+::)*(Ecs)?ShadowRoot>|\bis_shadow_root\(' bfc63658 -- crates ':!*tests*' | wc -l` → 63.
- P-REL: `git grep -n -P '\.(parent|first_child|last_child|next_sibling|prev_sibling)\b(?!\s*\()' bfc63658 -- crates/core/elidex-ecs/src ':!*tests*' | wc -l` → 41.
- P-WR: `git grep -n -P '\.parent\s*=[^=]' bfc63658 -- crates/core/elidex-ecs/src ':!*tests*' | wc -l` → 3;
  `git grep -n -P '\blink_node\(' bfc63658 -- crates ':!*tests*' | wc -l` → 4.
- P-HOST: `'\.host\b(?!\s*\()'` → 41 (14 in shadow-related files, 11 of them reads).
- Child-side scope of `tc`: `git grep -n -P '\b(get_first_child|get_last_child|get_next_sibling|get_prev_sibling|children_iter|children_iter_rev|child_list_uncapped|index_in_parent|next_exposed_sibling|prev_exposed_sibling)\(|\.children\(' bfc63658 -- crates ':!*tests*' | wc -l` → 314; the same over `-- crates/layout` → 10 (5 files), `-- crates/core/elidex-render` → 3 (2 files), `-- crates/css/elidex-style` → 15 (5 files), `-- crates/shell` → 10 (4 files) (verified 2026-11-18; the file counts with `git grep -l` and the same pattern).
- P-CHW: the script of census C-13 → "raw child reads: 104 lines in 54 functions, 25 files (definition lines excluded)" (verified 2026-11-18); 25 rows marked `walk`, 23 of them outside elidex-ecs.
- P-VM, P-NAV: the script of census C-12 → "VM host: 58 lines 25 files {'CH': 10, 'DER': 46, 'UP': 4}" (verified 2026-11-18) and "dom-api handlers: 39 of 217 registered; {'R': 21, 'U': 18}". Reachability is the handler census of `#11-dom-api-handlers-unreachable-from-production`, re-run on the `bfc63658` tree (a copy of its scripts outside the PM's directory, `run.sh <worktree>`): 217 registered, 161 reached, 56 unreachable, the same list as the PM's saved `unreachable.txt`.
- The mutators' callers (§7.3 Q-19, census C-9): `git grep -n -P '\.(remove_child|insert_before|replace_child)\(' bfc63658 -- crates ':!*tests*' | wc -l` → 45 (also 45 at `84fee8fa`; draft 3.2's 53 is the `\b` form).
- `apply_insert_before` (§1.6): `git grep -n -P '\bapply_insert_before\(' bfc63658 -- crates ':!*tests*' | wc -l` → 9 (the definition and 8 calls).
- `apply_replace_child` (§1.6, draft 3.4): `git grep -n -P '\bapply_replace_child\(' bfc63658 -- crates ':!*tests*' | wc -l` → 4 (the definition and 3 calls); `git grep -n -P '\bReplaceChild\s*\{' bfc63658 -- crates | wc -l` → 5; both the same at `b46a030c`.
- `tc`'s test files over 1000 lines (§5 `tp-4`): `wc -l` → 1137, 1321; child-side lines: the `tc` pattern above with `git grep -c` on each file → 29, 1.
- P-REL outside `dom/tree/` (§5.3): the P-REL command piped through `cut -d: -f2 | sort | uniq -c` → `dom/mod.rs` 3, `dom/mutation_event.rs` 1 (a doc comment), `fragment_tree.rs` 1 (the false positive).
- §0.1 item 6: two temporary VM tests, each in a scratch worktree and since removed (rustc 1.99.1): the child-of methods at `84fee8fa`, and Range `deleteContents` over a host with a closed shadow root at `bfc63658` (driver, R3); observed values in census C-14. `ta`'s plan re-runs the first and `tq`'s plan the second as their first probes (§5.2).
- File sizes: `wc -l` on the files of §5 `tp` and §7.2.
- `tq`'s test files (§5 `tq`, draft 3.5): `git ls-files crates | /usr/bin/grep -i -E 'range|selection' | /usr/bin/grep -i test | xargs wc -l` at `2ac9fa44` → the largest is `crates/script/elidex-js/src/vm/tests/tests_selection.rs` 787; the same `wc -l` over `tests_mutation_observer/range_ops.rs` 586 and `tests_shadow_dom/attach.rs` 727 (the other files that call `deleteContents` / `toString` in a Range or shadow test); none over 1000.
- Coverage map: `.claude/tools/webref coverage-map dom 1.1 dom 2.7 dom 4.2.2 dom 4.2.3 dom 4.3.2 dom 4.4 dom 4.7 dom 4.8 dom 4.9 dom 5.2 dom 5.6 dom 6 selection-api 3 html 4.12.3 html 4.10.18.3 html 4.10.15 html 4.10.19.5 html 4.10.10 html 4.10.22.3 html 2.6.4.3 html 4.16.3 html 13.4 html 6.8.1 html 6.3 html 3.2.6.2 html 3.2.6.4 execCommand 6 selectors-4 14.1 css-shadow-1 3.2.2 css-shadow-1 4.1 cssom-view-1 7 css-pseudo-4 4.1` (draft 3.4, verified 2026-11-18: "spec=8 … step=32 entries").
- Not measured: anything at runtime except §0.1 item 6 (§5.2); the per-call relation (census C-7).

## §9. Citation table

Each row was looked up with `.claude/tools/webref` (`dfn` / `heading` for the number ↔ title pair, `body` for the
quoted words): [D1] to [V1] of draft 2 on 2026-11-15, and [D5] again with [D13], [H12] to [H14], [E1] and [X1] on
2026-11-18; for draft 3.3, [D8]'s *contained*, [D13]'s *replace* step 1 and *pre-remove* step 1, [H15]'s bullet,
[H16] and [D14] on 2026-11-18; for draft 3.4, [P1], [H15]'s *associated focus navigation owner* and [H16]'s
`assignedNodes()` on 2026-11-18. Census C-8 holds the further lookups for single rows.

| ID | Citation | Anchor | Used by |
|---|---|---|---|
| [D1] | WHATWG DOM §1.1 Trees — *parent*, *root*, *inclusive ancestor*, *index*, *tree order* | `dom#concept-tree-parent`, `dom#concept-tree-order` | §1, §3 |
| [D2] | WHATWG DOM §4.2.2 Shadow tree — "shadow tree is a node tree whose root is a shadow root"; "A node is connected if its shadow-including root is a document" | `dom#concept-shadow-tree`, `dom#connected` | §1, §2, §3, §4 |
| [D3] | WHATWG DOM §4.7 Interface DocumentFragment — "host (null or an element in a different node tree)"; *host-including inclusive ancestor* | `dom#concept-documentfragment-host` | §1.2, §3, §4 |
| [D4] | WHATWG DOM §4.8 Interface ShadowRoot — *shadow-including root*; *shadow-including inclusive ancestor*; *shadow-including tree order*; *retarget* ("A's root is not a shadow-including inclusive ancestor of B"); a shadow root's *get the parent* (quoted in §1.5) | `dom#concept-shadow-including-root`, `dom#concept-shadow-including-inclusive-ancestor`, `dom#concept-shadow-including-tree-order`, `dom#retarget` | §1.1, §1.5, §3 |
| [D13] | WHATWG DOM §4.2.3 Mutation algorithms — *ensure pre-insert validity* step 3 ("If child is non-null and its parent is not parent, then throw a \"NotFoundError\" DOMException."); *replace* step 1 ("Ensure pre-insert validity given node, parent, child, and « child »."); *pre-remove* step 1 ("If child's parent is not parent, then throw a \"NotFoundError\" DOMException.") | `dom#concept-node-ensure-pre-insertion-validity`, `dom#concept-node-replace`, `dom#concept-node-pre-remove` | §0.1, §1.6, §2, §3 |
| [D5] | WHATWG DOM §4.2.3 Mutation algorithms — *ensure pre-insert validity* step 2 ("If node is a host-including inclusive ancestor of parent, then throw a \"HierarchyRequestError\" DOMException"); *move* step 2 ("… of newParent …") | `dom#concept-node-ensure-pre-insertion-validity`, `dom#move` | §1.2, §2, §3 |
| [D6] | WHATWG DOM §4.3.2 Queuing a mutation record — step 2 ("Let nodes be the inclusive ancestors of target") | `dom#queue-a-mutation-record` | §2, §3 |
| [D7] | WHATWG DOM §4.4 Interface Node — "The parentNode getter steps are to return this's parent"; "The contains(other) method steps are to return true if other is an inclusive descendant of this; otherwise false"; `compareDocumentPosition` step 6; a node's *get the parent* (quoted in §1.5) | `dom#dom-node-parentnode`, `dom#interface-node` | §1.5, §2, §3 |
| [D8] | WHATWG DOM §5.6 Interface Range — "The root of a live range is the root of its start node"; *contained*: "contained in a live range range if node's root is range's root, and (node, 0) is after range's start, and (node, node's length) is before range's end" (looked up 2026-11-18: `dfn dom contained` → §5.6, `body dom contained`); *get the common ancestor* step 2; `setStartBefore` steps 1-2 | `dom#concept-range-root`, `dom#contained`, `dom#get-the-common-ancestor`, `dom#dom-range-setstartbefore` | §0.1, §1.6, §2, §3 |
| [D9] | WHATWG DOM §5.2 Boundary points — "Assert: nodeA and nodeB have the same root." | `dom#concept-range-bp-position` | §2, §3 |
| [D10] | WHATWG DOM §6 Traversal — "Each NodeIterator and TreeWalker object also has an associated root (a node)" | `dom#traversal` | §2, §3 |
| [D11] | WHATWG DOM §2.7 Interface EventTarget — *get the parent* (dfn): "Nodes, shadow roots, and documents override the get the parent algorithm."; §4.5 Interface Document, a document's *get the parent* (quoted in §1.5) | `dom#get-the-parent`, `dom#interface-document` | §1.5, §3 |
| [D12] | WHATWG DOM §4.9 Interface Element — `closest` step 3 | `dom#dom-element-closest` | §3 |
| [H1] | WHATWG HTML §4.12.3 The template element — "The template contents of a template element are not children of the element itself" | `html#template-contents` | §3, §4 |
| [H2] | WHATWG HTML §4.10.18.3 Association of controls and forms — *reset the form owner* step 4 ("Otherwise, if element has an ancestor form element, then set newForm to the nearest such ancestor form element.") | `html#reset-the-form-owner` | §2, §3 |
| [H3] | WHATWG HTML §4.10.15 The fieldset element — *disabled fieldset* ("It is a descendant of another fieldset element whose disabled attribute is specified, and it is not a descendant of that fieldset element's first legend element child, if any.") | `html#concept-fieldset-disabled` | §2, §3 |
| [H4] | WHATWG HTML §4.10.10 The option element — *nearest ancestor select* | `html#option-element-nearest-ancestor-select` | §2, §3 |
| [H5] | WHATWG HTML §4.10.22.3 Form submission algorithm — "Let subject be form's nearest ancestor dialog element" | `html#form-submission-algorithm` | §2, §3 |
| [H6] | WHATWG HTML §3.2.6.2 The lang and xml:lang attributes — quoted in §3 | `html#language` | §3 |
| [H7] | WHATWG HTML §3.2.6.4 The dir attribute — *parent directionality* step 2, quoted in §3 | `html#parent-directionality` | §3 |
| [H8] | WHATWG HTML §4.10.19.5 Enabling and disabling form controls: the disabled attribute — "the element is a descendant of a fieldset element whose disabled attribute is specified, and the element is not a descendant of that fieldset element's first legend element child, if any." | `html#enabling-and-disabling-form-controls:-the-disabled-attribute` | §2, §3 |
| [H9] | WHATWG HTML §4.10.10 The option element — *disabled*: "If ancestor is an optgroup element, then return true if ancestor's disabled attribute is present; otherwise false." | `html#the-option-element` | §2, §3 |
| [H10] | WHATWG HTML §6.3 Inert subtrees — "the element and all its flat tree descendants" | `html#inert-subtrees` | §1.5, §3 |
| [H11] | WHATWG HTML §6.8.1 Making document regions editable: The contenteditable content attribute — "The inherit state indicates that the element is editable (or not) based on the parent element's state." | `html#contenteditable` | §2, §3 |
| [H12] | WHATWG HTML §2.6.4.3 The HTMLOptionsCollection interface — `add(element, before)` step 1 ("If element is an ancestor of the select element on which this is rooted, then throw a \"HierarchyRequestError\" DOMException.") and step 2 ("If before is an element, but that element isn't a descendant of the select element on which this is rooted, then throw a \"NotFoundError\" DOMException."); HTML §4.10.7 The select element: `add` "must act like its namesake method on that same options collection" | `html#the-htmloptionscollection-interface`, `html#the-select-element` | §2, §3 |
| [H13] | WHATWG HTML §4.16.3 Pseudo-classes — `:read-write`: "elements that are editing hosts or editable and are neither input elements nor textarea elements" | `html#pseudo-classes` | §2, §3 |
| [H15] | WHATWG HTML §6.6.2 Data model — *flattened tabindex-ordered focus navigation scope* step 2.2 ("If item is not a focusable area, then replace item with all of the items in item's flattened tabindex-ordered focus navigation scope.") and step 2.3 ("Otherwise, insert the contents of item's flattened tabindex-ordered focus navigation scope after item."); *focus navigation scope owner* ("a Document, a shadow host, a slot, or …"); *tabindex-ordered focus navigation scope*, first bullet ("It contains all elements in owner's focus navigation scope that are themselves focus navigation scope owners, except the elements whose tabindex value is a negative integer."). *associated focus navigation owner* step 2 ("If element's parent is a shadow host, then return element's assigned slot."). Looked up 2026-11-18 (`dfn` for the four terms → §6.6.2 Data model; `body` for the steps and the bullet) | `html#flattened-tabindex-ordered-focus-navigation-scope`, `html#focus-navigation-scope-owner`, `html#tabindex-ordered-focus-navigation-scope`, `html#associated-focus-navigation-owner` | §2, §3.1, §7.1, §7.3 |
| [H16] | WHATWG HTML §4.12.4 The slot element — `assign(...nodes)` step 3.1 ("If node's manual slot assignment refers to a slot, then remove node from that slot's manually assigned nodes."); *manually assigned nodes*; `assignedNodes(options)`. Looked up 2026-11-18 (`heading html 4.12.4`; `dfn html 'assign(...nodes)'`, `dfn html 'manually assigned nodes'` and `dfn html 'assignedNodes'` → §4.12.4; `body html dom-slot-assign`) | `html#dom-slot-assign`, `html#manually-assigned-nodes`, `html#dom-slot-assignednodes` | §5.3 |
| [D14] | WHATWG DOM §4.2.2.2 Slottables — *manual slot assignment*; §4.2.2.3 Finding slots and slottables — *find a slot*, *find slottables*; §4.2.2.4 Assigning slottables and slots; §4.2.2.5 Signaling slot change (the number ↔ title pairs of the drift list). Looked up 2026-11-18 (`heading dom 4.2.2.2` to `4.2.2.5`; `dfn dom 'manual slot assignment'`, `'find a slot'`, `'find slottables'`) | `dom#slottable-manual-slot-assignment`, `dom#find-a-slot`, `dom#find-slotables` | §5.3 |
| [H14] | WHATWG HTML §13.4 Parsing HTML fragments — step 19: "Set parser's form element pointer to the nearest node to contextElement that is a form element (going straight up the ancestor chain, and including the element itself, if it is a form element), if any." | `html#parsing-html-fragments` | §2, §3 |
| [E1] | execCommand §6 Common definitions — *editable*: "its parent is an editing host or editable" (the spec source marks "parent" as the tree's *parent*) | `execCommand#editable` | §2, §3 |
| [X1] | Selection API §3 Selection interface — `containsNode()` (number ↔ title and the method's anchor through `dfn`; `body` does not return this section's prose, so no words are quoted) | `selection-api#dom-selection-containsnode` | §2, §3 |
| [S1] | CSS Selectors L4 §14.1 Descendant combinator ( ) | `selectors-4#descendant-combinator` | §3 |
| [S2] | CSS Selectors L4 §9.1 The Pointer Hover Pseudo-class: :hover — "An element also matches :hover if one of its descendants in the flat tree (including non-element nodes, such as text nodes) matches the above conditions." | `selectors-4#hover-pseudo` | §1.5 |
| [C1] | CSS Shadow Module Level 1 (css-shadow-1) §4.1 Flattening the DOM into an Element Tree — *flat tree*; its Note: "the flat tree is the top-level DOM tree, but shadow hosts are filled with their shadow tree children instead of their light tree children" (draft 3.5; `heading css-shadow-1 4.1` → `#flattening`; `body css-shadow-1 flattening`, looked up 2026-11-18) | `css-shadow-1#flat-tree` | §1.3, §1.5, §3 |
| [C2] | CSS Shadow Module Level 1 (css-shadow-1) §3.2.2 Selecting Shadow Hosts from within a Shadow Tree — "For the purpose of Selectors, a shadow host also appears in its shadow tree, with the contents of the shadow tree treated as its children. (In other words, the shadow host is treated as replacing the shadow root node.)" | `css-shadow-1#host-element-in-tree` | §1.5, §3 |
| [C3] | CSS Shadow Module Level 1 (css-shadow-1) §3.2.1 Matching Selectors Against Shadow Trees — "the selector match list is initially the shadow host, followed by all children of the shadow tree's shadow root and their descendants" | `css-shadow-1#selectors-data-model` | §1.5 |
| [P1] | CSS Pseudo-Elements Level 4 (css-pseudo-4) §4.1 Generated Content Pseudo-elements: ::before and ::after — `::before`: "Represents a styleable child pseudo-element immediately before the originating element's actual content." Looked up 2026-11-18 (`heading css-pseudo-4 4.1`; `dfn css-pseudo-4 '::before'` → §4.1; `body css-pseudo-4 selectordef-before`) | `css-pseudo-4#selectordef-before` | §0, §1.3, §2, §3, §4, §5.3, §7.2 |
| [V1] | CSSOM View (cssom-view-1) §7 Extensions to the HTMLElement Interface — `offsetParent` step 2: "Let ancestor be the parent of the element in the flat tree and repeat these substeps" | `cssom-view-1#dom-htmlelement-offsetparent` | §1.5, §3 |
