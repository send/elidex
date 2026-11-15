# Tree model: a shadow root is a tree root; every climb chooses whether it crosses — program umbrella

Status: **umbrella, draft 2** (pre-implementation; draft 1 `379acc15` was plan-reviewed, R1: CRIT 1 / IMP 21 /
MIN 24, root clusters R1 to R6; this draft answers them and writes in PM 747 and PM 748). `/elidex-plan-review` runs
again on this umbrella and on each per-PR plan before its code (CLAUDE.md, *Edge-dense work*). Owner: L3. Base:
`c0578223`; every code anchor is at that commit (`git diff --stat c0578223..origin/main` on 2026-11-15 touches no
file under `crates/` or `docs/plans/`). Census data: [2026-11-tree-model-census.md](2026-11-tree-model-census.md)
(commands, counts, per-file tables, spec lookups; §6 here says what each population is for).

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
   `git grep -n -P '\bis_ancestor_or_self\(' c0578223 -- crates ':!*tests*'`).

### §0.2 Why a program, and why before PR (io)

More than three invariant axes intersect (§2), and the population that breaks is not enumerated by the compiler:
under a storage change alone, a crossing climb that relies on the stored parent keeps compiling and silently stops
at the boundary. That is the *Edge-dense* trigger; the slices of §5 are its base case. PR (io)'s central invariant
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
  become such reads in `ta` (census C-2, "raw"), and `ts` re-reads each.
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
  `is_ancestor_or_self` lines (`git grep -n -P '\bis_host_including_ancestor_or_self\(' c0578223 -- crates ':!*tests*'`
  for the base list). `focus/delegate.rs:88`, which uses that function for a shadow-including question today, moves
  to the shadow-including test.
- **Not introduced here: an ancestor iterator.** The climb loops keep their shape and their depth bound; this
  program changes which step each loop takes.

### §1.3 The child side

One accessor set reads the tree children: first child, last child, next sibling, previous sibling, the two
iterators, `children`, `child_list_uncapped`, and the index. The four raw accessors and
`next_exposed_sibling` / `prev_exposed_sibling` are **replaced by one set with new names** in `tc`, so every P-CH
line fails to compile there and is listed in `tc`'s plan (the same device as `ta`). From `tc` to `ts` the set holds
the interim skip in the accessor layer, in two halves: a shadow-root **result** is skipped, and a shadow-root
**receiver** has no siblings and no index (the guard the VM natives carry today, census C-5 role R, so
`shadowRoot.nextSibling` is null at every head). `ts` deletes both halves with the storage. A reader that wants the
shadow tree asks for it (`get_shadow_root(host)`, then that root's children); a reader that wants flat-tree
children uses `composed_children`.

### §1.4 The transitional accessor (declared, bounded by the program)

Slice `ta` must be behaviour-preserving, and some callers' spec relation is the tree parent while their base
behaviour crosses. Such a caller cannot take `tree_parent` in `ta` without changing behaviour, and
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
  uses a pending name. It covers the names of this section and of §1.5 (authoring lane: §7.3 Q-11).
- The accessor and `Flip` are deleted by the last flip slice; `ts` requires that they do not exist.

This is a strangler state inside the program, declared with its removal condition; the alternatives are in §4.

### §1.5 Relations this program does not define

Three upward relations are not DOM's tree or shadow-including relations. Their readers climb the stored parent at
base. Taking `shadow_including_parent` or `tree_parent` for them would record a relation nobody chose, so in `ta`
each takes **its own pending name**, with base behaviour, owned outside this program (census C-2 classes F, E, L):

| Relation | Spec | Pending name (body) | Readers at base | Owner |
|---|---|---|---|---|
| **Flat-tree parent** and flat-tree order | css-shadow-1 §4.1 [C1]; CSSOM View `offsetParent` ("the parent of the element in the flat tree") [V1]; HTML §6.3 inert ("flat tree descendants") [H10]; Selectors `:hover` [S2] | `EcsDom::pending_flat_tree_parent` (NEW: the shadow-including step); `EcsDom::pending_flat_tree_order` (NEW: §2 ORD × RENDER) | 21 lines: render, layout root tests, CSSOM View queries, the hidden-subtree test, a11y, shell hover and scroll (census C-2 "F", verified 2026-11-15) | `#11-shadow-slot-assignment-flat-tree` (P2, Layout) |
| **Event *get the parent*** | DOM §2.7 dfn; a node's: "returns the node's assigned slot, if node is assigned; otherwise node's parent" [D7]; a shadow root's: "returns null if event's composed flag is unset and shadow root is the root of event's path's first event path item's invocation target; otherwise shadow root's host" [D4]; a document's: "returns null if event's type attribute value is \"load\" or document does not have a browsing context; otherwise the document's relevant global object" [D11] | `EcsDom::pending_event_parent` (NEW: the shadow-including step). The `!composed` break at `event_dispatch/mod.rs:214` is the shadow root's null arm and stays | 4 lines: `build_propagation_path`, the shell's link and label lookups (census C-2 "E", verified 2026-11-15) | `#11-event-path-window-missing` (L3) owns the function and its Document, ShadowRoot and Node arms (`docs/plans/2026-11-event-path-window.md:366`); the *assigned slot* arm inside the Node arm is P2's. The shell's two lookups: §7.3 Q-12 |
| **The selector tree's parent** | css-shadow-1 §3.2.2: "a shadow host also appears in its shadow tree, with the contents of the shadow tree treated as its children" [C2]; §3.2.1 [C3] | `EcsDom::pending_selector_parent` (NEW: the tree parent; the climb stops at the shadow root, as at base) | 7 lines in `selector/matching.rs` and `selector/traverse.rs` (census C-2 "L", verified 2026-11-15) | pending (PM 748 (c)): L3 probes and drafts a slot separately |

These names outlive the program. Each owner replaces its name by the relation it defines; the trip-wire of §1.4
keeps the three populations from growing until then.

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
| TREE × WR | elidex-ecs has one site that writes a parent (`link_node`) and three callers; none refuses a shadow root today. `ts` adds the refusal beside the cycle guard in `append_child`, `insert_before`, `replace_child` (they return `false`), and `link_node` asserts it | ts | ts cells: each of the three refuses a shadow-root node; mutant: remove one refusal |
| TREE × VER | attach reaches `rev_version(host)` today only through `append_child` (`mutation.rs:54`), and the shell repaints when the document root's version moves (`crates/shell/elidex-shell/src/content/event_loop.rs:444`, `crates/shell/elidex-shell/src/content/mod.rs:369`). The version is a shadow-including-descendants version (`rev_version` climbs by the shadow-including step), and attaching changes the host's shadow-including descendants, so `ts`'s attach calls `rev_version(host)` itself | ts | ts cell: `attachShadow` alone moves the document root's version; mutant: drop the call |
| TREE × UP-T / UP-S | `tree_parent(sr)` is `None` and `shadow_including_parent(sr)` is the host at every head (§1.1) | ta, ts | ta cells (accessor contracts); ts: the same cells, unchanged |
| UP-T × UP-S × DER | every climb takes exactly one named step; every derived relation is on a disposition list (§1.2) | ta | the compiler; ta re-runs census C-6 at its head and accounts for every row |
| UP-S × CONN | *connected* climbs by `shadow_including_parent` (Layout's item 1: its callers `html_fragment.rs:102`, `mutation.rs:44-46`, `:122` keep their answer) | ta, ts | existing connectedness cells; ts adds a nested-shadow case |
| UP-H × TREE | the cycle guards are the host-including test, so a host cannot be appended into its own shadow tree once the stored parent no longer crosses ([D5]) | ta, ts | ts cell: host into own shadow tree, nested two levels |
| CHILD × TREE | until `ts` the one set skips in the accessor layer (both halves, §1.3); after `ts` the skip is gone | tc, ts | tc cells per accessor, including a shadow-root receiver; ts: unchanged |
| CHILD × ORD | the index counts tree children only (D13 (a)); `index_in_parent` already skips shadow-root siblings and gates the `Insert` fire in `append_child` / `insert_before` (Layout's item 2, pinned by `crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs:755-777`) | tc | tc cell: index of a host's children with a shadow root attached before and after them |
| CHILD × NAV | in `ta` the child and sibling handlers take what the natives call (PM 748 (b)); in `tc` both read the one set and the natives' own marker tests go (census C-10) | ta, tc | existing VM navigation cells |
| CHILD × MO | a mutation record's previous / next sibling come from the exposed accessors (`crates/script/elidex-script-session/src/mutation/child_list.rs`), which fold into the set: no change predicted | tc | existing record cells |
| CHILD × VER (live collections) | `live_collection/mod.rs:443` re-tests a child that the iterator already filtered: deleted | tc | existing live-collection cells |
| CHILD × SEL | sibling combinators and structural pseudo-classes read element siblings, which a shadow root never is: no change predicted | tc | existing selector cells |
| CHILD × FORM | 15 raw child reads in elidex-form (`git grep -n -P '\b(get_first_child\|get_last_child\|get_next_sibling\|get_prev_sibling)\(' c0578223 -- crates/dom/elidex-form ':!*tests*'`): a descendant walk that meets a host's shadow root as a child enters the shadow tree today (predicted) and stops entering in `tc`; the upward half of the same fact flips in `tf` | tc → tf | tc cells (downward); tf cells (upward); the interim is in §5.1 |
| CHILD × RANGE | Range boundary walks and 29 traversal lines read raw children (census C-3, `traversal.rs`): D13, and TreeWalker / NodeIterator entering a shadow tree (predicted); the upward half flips in `tr` | tc → tr | tc cells; RC-1 consumes the set (§7.2) |
| CHILD × RENDER, A11Y | layout and render read `composed_children` for content; their 9 raw reads (census C-3) are re-read line by line: tree children or `composed_children`, no third kind | tc | no expectation changes predicted; tc's probe (§5.2) |
| CHILD × PARSE | the strict parser's append position (`crates/dom/elidex-html-parser-strict/src/tree_builder/insert.rs:212`, Layout's item 6) and `innerHTML` on a shadow root read the tree children of the node they were given: no change predicted | tc | the parser's existing shadow-context cells |
| UP-T × NAV | `parentNode` / `parentElement` of a shadow root are null through the native and the handler ([D7]); `contains` is the tree test in both (PM 748 (b)) | ta | existing VM cells; no new cell |
| UP-T × MO | *queue a mutation record* collects from the **inclusive ancestors** of the target ([D6]): an observer on the host or the document with `subtree: true` gets no record for a mutation inside the shadow tree; transient observers follow the same ancestors | tm | tm cells (host observer, document observer; an observer on the shadow root still gets them) |
| UP-T × RANGE | a live range's root is its start node's root; *get the common ancestor* climbs by parent; boundary points are compared within one root ([D8], [D9]); traversal stays inside its root ([D10]); a shadow-root argument to the sibling-slot methods has a null parent | tr | tr cells |
| UP-T × FORM | *reset the form owner* step 4, *disabled fieldset*, *nearest ancestor select*, the optgroup of *option disabled*, "nearest ancestor dialog", and `select.add`'s ancestor tests: tree ancestors ([H2] to [H5], [H8], [H9]) | tf | tf cells; `#11-form-owner-ecs-component` (§7.1) |
| UP-T × EDIT | the inherit state follows "the parent element's state" ([H11]): an element in a shadow tree does not inherit editability from its host. `:read-write` reads the same function | te | te cells (`isContentEditable`, `:read-write`) |
| UP-T × PARSE | the strict form-pointer walk is a tree climb; its guard folds into the step (Layout's item 5) | ta | the strict parser's existing shadow-context cell |
| ORD × DER | `tree_order` is DOM §1.1 tree order within one tree, with the entity-bits fallback across roots. `compare_document_position` and the radio group call it only for nodes of one tree root (census C-8 R-9), so they take it in `ta` with no change | ta | existing `compareDocumentPosition` and radio cells |
| ORD × FORM | the radio group's sort is over one tree root (above): it shares no cell with `tf` | ta | — |
| ORD × RANGE | Range's `compare_points` takes the crossing order as a pending name and flips with the rest of Range | tr | tr cells |
| ORD × RENDER (ND) | paint and hit-test layer 6 sort by tree order across shadow boundaries (`crates/core/elidex-render/src/builder/walk.rs:620`, `crates/layout/elidex-layout/src/hit_test.rs:235`). The order they need is the flat tree's. They take `pending_flat_tree_order`, which is the base function until `ts`. At `ts` the shadow root is no longer among the host's children, so the function must say where the shadow tree sits: DOM §4.8 *shadow-including tree order* (the shadow tree just after its host). **One residual**: a shadow-tree node against a light child that preceded the shadow root in the host's child list (§7.3 Q-13) | ta, ts | ts cells: existing paint / hit-test cells unchanged; the residual case pinned with its decided order |
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
| WHATWG DOM §4.2.3 Mutation algorithms | *ensure pre-insert validity* step 2; *move* step 2 | host-including inclusive ancestor → `HierarchyRequestError` | the host-including test (ta, ts); *move* is not implemented (no `moveBefore`) | ✓ | yes |
| WHATWG DOM §4.3.2 Queuing a mutation record | step 2 | "the inclusive ancestors of target" | observer ancestor walks (tm) | ✓ | yes |
| WHATWG DOM §4.4 Interface Node | `parentNode`, `parentElement`, `isConnected`, `contains`; `compareDocumentPosition` step 6; a node's *get the parent* | a shadow root's parent is null; different roots are disconnected | natives and handlers (ta, tc); `tree_order` (ta) | ✓ | yes |
| WHATWG DOM §4.7 Interface DocumentFragment | *host*; *host-including inclusive ancestor* | the host is "in a different node tree" | the one host-including test (ta) | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *shadow-including root*, *ancestor*, *tree order*; *retarget*; a shadow root's *get the parent* | each crossing is written as "root's host" | `shadow_including_parent`, `shadow_host` (ta); *retarget*'s read (ta) | ✓ | yes |
| WHATWG DOM §4.9 Interface Element | `closest` step 3 | "inclusive ancestors that are elements" | `child_node/selectors.rs` (ta) | ✓ | yes |
| WHATWG DOM §5.2 Boundary points | *position* | "Assert: nodeA and nodeB have the same root" | `compare_points` (tr) | ✓ | yes |
| WHATWG DOM §5.6 Interface Range | *root*; *get the common ancestor* step 2; `setStartBefore` steps 1-2 | within one tree; a null parent throws | Range climbs (tr) | ✓ | yes |
| WHATWG DOM §6 Traversal | NodeIterator and TreeWalker *root* | inclusive descendants of the root | traversal climbs (tr) and child reads (tc) | ✓ | yes |
| Selection API §3 Selection interface | `containsNode` | through the range's boundary points | `selection.rs` (tr) | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | "not children of the element itself" | the precedent, already (X)-shaped in elidex | none (comparison only; §4) | ✓ | no |
| WHATWG HTML §4.10.18.3 Association of controls and forms | *reset the form owner* step 4 | "has an ancestor form element" | form-owner climbs (tf) | ✓ | yes |
| WHATWG HTML §4.10.15 The fieldset element | *disabled fieldset* | "a descendant of another fieldset element" | fieldset climbs (tf) | ✓ | yes |
| WHATWG HTML §4.10.19.5 Enabling and disabling form controls: the disabled attribute | *disabled* | "a descendant of a fieldset element whose disabled attribute is specified" | fieldset climbs (tf) | ✓ | yes |
| WHATWG HTML §4.10.10 The option element | *nearest ancestor select* step 2; *disabled* (the optgroup ancestor) | ancestors in reverse tree order | option / select climbs (tf) | ✓ | yes |
| WHATWG HTML §4.10.22.3 Form submission algorithm | "nearest ancestor dialog element" | tree ancestor | `submit.rs` (tf) | ✓ | yes |
| WHATWG HTML §6.8.1 Making document regions editable: The contenteditable content attribute | the inherit state | "the parent element's state" | contenteditable climbs (te) | ✓ | yes |
| WHATWG HTML §6.3 Inert subtrees | *inert* | "flat tree descendants" | none (§1.5) | ✓ | yes |
| WHATWG HTML §3.2.6.2 The lang and xml:lang attributes | *language* | "If the node's parent is a shadow root" | none: no `lang` climb exists in the code; wording evidence (below) | ✓ | no |
| WHATWG HTML §3.2.6.4 The dir attribute | *parent directionality* step 2 | "If parentNode is a shadow root" | none: no `dir` climb exists; wording evidence | ✓ | no |
| CSS Selectors L4 §14.1 Descendant combinator ( ) | combinators | ancestors in the selector's tree | none (§1.5, pending PM 748 (c)) | ✓ | yes |
| CSS Shadow Module Level 1 (css-shadow-1) §3.2.2 Selecting Shadow Hosts from within a Shadow Tree | the featureless host | "treated as replacing the shadow root node" | none (§1.5) | ✓ | yes |
| CSS Shadow Module Level 1 (css-shadow-1) §4.1 Flattening the DOM into an Element Tree | *flat tree* | — | none (§1.5) | ✓ | yes |
| CSSOM View (cssom-view-1) §7 Extensions to the HTMLElement Interface | `offsetParent` step 2 | "the parent of the element in the flat tree" | none (§1.5) | ✓ | yes |

**Breadth**: K=6 specs, M=27 entries (verified 2026-11-15 with `.claude/tools/webref coverage-map`; the argument
list is in §8). The verdict is "split by default" (K ≥ 6), and the program is split (§5); the cut follows §0.2.
The preflight's label map has no entry for Selection API, css-shadow-1 and cssom-view-1, so those four rows were
checked by hand (`.claude/tools/webref heading selection-api 3`, `heading css-shadow-1 3`, `heading css-shadow-1 4.1`,
`heading cssom-view-1 7`). Also nine of the rows touch no code
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

- `ta`, `ts`: none intended. `ts` has one render-order residual (§2 ORD × RENDER).
- `tc`: any raw child or sibling reader that exposes a shadow root as a child of its host: Range boundary indices
  and walks (D13), TreeWalker / NodeIterator descending into a shadow tree, form descendant walks.
- `tm`: `MutationObserver` with `subtree: true` on a host, an ancestor of a host, or the document no longer
  receives records for mutations inside the shadow tree.
- `tr`: `Range.commonAncestorContainer`, boundary comparison, `isPointInRange` / `comparePoint`, contained-node
  enumeration and traversal steps no longer leave the shadow tree upward; `setStartBefore(shadowRoot)` and its
  siblings throw.
- `tf`: a form control inside a shadow tree has no form owner from a light-tree `<form>` around its host, is not
  disabled by a light-tree `<fieldset disabled>`, an `<option>` does not find a `<select>` or `<optgroup>` across
  the boundary, `method=dialog` does not find a dialog across it.
- `te`: `isContentEditable` and `:read-write` no longer inherit across the boundary.
- Not changed: the insertion guards that reject a shadow root as a node (census C-5 role I); event paths,
  retargeting and `composedPath`; selector matching other than `:read-write`.

## §4. Lens block (basis order)

- **spec (first)**: §3. A shadow tree's root is a shadow root (DOM §4.2.2), a root has a null parent (§1.1), the
  host is "an element in a different node tree" (§4.7), every crossing is the explicit step "root's host" (§4.8),
  and HTML's template contents "are not children of the element itself" (§4.12.3). The stored relation that matches
  all of these is (X).
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
- **Rejected: (Y)** (PM 730); **making the template contents a tree child too** (contradicts §4.12.3 and extends
  the compensation to a second kind).
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

## §5. The program

| # | Slice | Primary module(s) | Slot | Tier | Deps |
|---|---|---|---|---|---|
| **tp** | **Touch-time split of `traversal.rs`** (1479 lines; seams: the shared preorder helpers, `TreeWalker` from `:153`, `NodeIterator` from `:1018`, the inline tests from `:1224`). Standalone, no behaviour change. `ta` and `tr` both edit this file. | `elidex-dom-api` `traversal.rs` | — (CLAUDE.md, *1000-line debt*) | — | — |
| **ta** | **Every climb chooses.** The two steps, the transitional accessor with `Flip`, and the three pending names of §1.5 replace `get_parent`, which is deleted, tests included. Every derived relation is removed, renamed or kept by rule (§1.2). Every reader of the backward link that climbs or lives outside elidex-ecs goes through `shadow_host` (*retarget*, a11y, the `ShadowRoot.host` native). The dom-api navigation handlers take the VM natives' relations (PM 748 (b)). The trip-wire lands. Behaviour-preserving. | `elidex-ecs` `dom/tree/*.rs`, `dom/mod.rs`, `dom/shadow.rs`; every crate of census C-2 (21 crates) | `#11-shadow-root-is-a-tree-root` | — | tp |
| **tc** | **One child-side accessor set** (§1.3), under new names; the interim skip (result and receiver halves) lives in the accessor layer; caller-side skips and guards (census C-5 roles K, R) are deleted. Observable where base exposes a shadow root as a child (§3.1). | `elidex-ecs` `dom/tree/navigation.rs`, `dom/mod.rs`; every file of census C-3 | as ta; closes D13 (§7.1) | — | ta |
| **tr** | **Range, Selection and traversal climbs are tree climbs** (`Flip::Tr`), including the crossing ancestor test and order they call and the VM's `require_attached_node`. | `elidex-dom-api` `range/`, `selection.rs`, `traversal/`; `elidex-js` `vm/host/range_proto.rs` | as ta; `#11-shadow-tree-live-range-tracking` (§7.1) | — | ta, tc |
| **tf** | **Form-association climbs are tree climbs** (`Flip::Tf`): the form owner's ancestor step (PM 747 Q-6; the plan cites HTML §4.10.18.3 through webref), fieldset disabledness, option / select / optgroup, the dialog of `method=dialog`, `select.add`'s ancestor tests. | `elidex-form` `submit.rs`, `select/mod.rs`; `elidex-form-core` `fieldset.rs`; `elidex-dom-api` `element/option_disabled.rs`, `element/select.rs` | as ta; `#11-form-owner-ecs-component` (§7.1) | — | ta, tc |
| **tm** | **MutationObserver climbs are tree climbs** (`Flip::Tm`). | `elidex-api-observers` `mutation/mod.rs` | as ta | — | ta |
| **te** | **Contenteditable inheritance is a tree climb** (`Flip::Te`), with `:read-write`. The last flip: it deletes the transitional accessor and `Flip`. | `elidex-ecs` `dom/mod.rs`; `elidex-dom-api` `element/inheritance.rs`; `elidex-css` `selector/matching.rs` (the `:read-write` arm) | as ta | — | ta, tc, tr, tf, tm |
| **ts** | **Stop appending.** Attach writes the link pair, bumps the host's version, and links nothing; elidex-ecs refuses to link a shadow root; the accessor-layer skips and `tree_parent`'s marker test are deleted; `shadow_including_parent` reads `shadow_host`; the P-REL lines (41) are re-read against R-TREE; the dead node half of the fire guards goes; the docs of §0.1 item 2 say one thing. Behaviour-preserving for script. Closes the slot. | `elidex-ecs` `dom/shadow.rs`, `dom/mod.rs`, `dom/tree/*.rs`, `components.rs` | `#11-shadow-root-is-a-tree-root` (closes) | — | te |

### §5.1 Order and shared cells

tp → ta → tc → tr → tf → tm → te → ts, then PR (io) rewritten on this base (§7.2).

- `ta` first: it creates the vocabulary and the compiler's lists. `tp` before it, because `ta` edits ten lines of
  `traversal.rs`.
- **`tr` and `tf` depend on `tc`.** Draft 1 said the flip slices share no cell; R1 showed three contacts. (1) Range
  and traversal read both sides: `tc` owns the downward cells ("does not enter a shadow tree through a child
  list") and `tr` the upward ones. (2) Form descendant walks and form ancestor climbs are two halves of one fact:
  `tc` downward, `tf` upward. Between `tc` and its partner slice a family is half-flipped (it no longer enters a
  shadow tree downward but still leaves it upward); `tc`'s plan lists, per family, the cross-boundary case that is
  inconsistent in that window and probes that it is wrong in value only (no panic, no corrupted link). `tr` and
  `tf` follow `tc` directly to keep the window short. (3) The order function: resolved in `ta` (§2 ORD × DER,
  ORD × FORM), so `te` and `tf` no longer share it with `tr`.
- `tm` and `te` depend only on `ta`; `te` is placed last among the flips because it deletes the accessor
  (PM 748 (b): its DoD names the handlers).
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
- **A miss toward "no change"** (a predicted flip where base already does not cross): the line takes `tree_parent`
  in `ta` instead, and the flip slice shrinks; if a whole cluster empties, the PM decides at that plan's review
  whether the slice merges into a neighbour (PM 747 Q-5).
- **A miss toward "change"** (a predicted equality that differs at base): the line does not take the final name in
  `ta`. It takes the transitional accessor with the `Flip` variant of its family's slice; if no slice owns that
  family, the miss is reported to the PM at `ta`'s review with the probe (a new slice or a neighbour is the PM's
  call), and `ta` does not land with a behaviour change.

### §5.3 Acceptance

- `tp`: no test changes its expectation; no file of the split exceeds 1000 lines.
- `ta`: `get_parent` does not exist; the plan's per-line table covers every line the compiler reported (P-UP and
  P-UP-T, re-measured at its base) and every census C-6 row; no existing cell changes its expectation, except a
  dom-api handler test that pins a handler's disagreement with the VM native (PM 748 (b); each is listed); the pending
  counts per name and per `Flip` variant are recorded in the allowlist; the benchmark of §1.1 is in the plan.
- `tc`, `tr`, `tf`, `tm`, `te`: probes run before review; each changed behaviour has a cell and a mutant on the
  final head; the slice's `Flip` variant and its `_pending_flip` names do not exist at its head. **`te`**: the
  transitional accessor and `Flip` do not exist, and no dom-api handler is a caller of a transitional name
  (PM 748 (b)).
- `ts`: no tree edge between a host and its shadow root, and the three mutators refuse one; no marker test of
  census C-5 role K, R or B remains (roles G and Ls stay, by §1.5); attach moves the document root's version; no
  script-visible cell changes its expectation; the DOM section cites in `shadow.rs`'s docs that R1 found drifted
  are re-derived with webref in its plan; the census is re-run at its head and every population's remaining
  lines are accounted for; the three pending names of §1.5 are the only pending names left.
- Program: R-TREE holds; §0.1 items 1 to 5 are gone; PR (io)'s "one home" sentence is true for the shadow root
  before PR (io) is written against it.

## §6. The census, and how each population maps to slices

Populations are defined by what the code does; commands, counts, controls and tables are in the census file.
Summary at `c0578223` (non-test unless noted, verified 2026-11-15):

| Population | Property | Count | Consumed by |
|---|---|---|---|
| P-UP | reads the stored parent through the public accessor | 143 lines (142 calls + the definition); 31 more under test paths (verified 2026-11-15, §8) | `ta`: all, by compile error (census C-2, predicted: T / S / H 61, raw 4, inside a removed helper 4, pending-flip 41, F 21, E 4, L 7, and the definition) |
| P-DER | a read-only function that climbs, or calls one | level 1: 60 (of 114 climbing functions); level 2: 45 (of 113) | `ta` (§1.2) |
| P-REL | reads or writes a `TreeRelation` link field directly | 41 lines, all inside elidex-ecs | `ts`; `ta` for the climbs among them |
| P-WR | writes a parent link | one site, three callers | `ts` (the refusal) |
| P-CH | reads a raw child / sibling link | 108 lines; 34 more under test paths (verified 2026-11-15, §8) | `tc`, by compile error |
| P-CHX | reads through a compensating child-side accessor | six accessors | `tc` (fold), `ts` (delete the skip) |
| P-MARK | tests or reads the `ShadowRoot` component | 63 lines (61 true): K 11, R 2, B 7, G 1, Ls 2, C 6, E 2, L 2, I 15, D 13 | K, R → `tc`; B, C → `ta`; E (node half) → `ts`; L → the lifetime program; G, Ls, I, D stay |
| P-HOST | reads the backward link field | 11 reading lines in shadow-related files | `ta` routes 7 through `shadow_host`; 3 stay inside elidex-ecs (census C-5) |

**What the census changed about the plan.** (1) Parent and child sides are separate populations with separate
instruments (`ta`, `tc`). (2) Direct field access cannot exist outside elidex-ecs, so `ts` is confined to one
crate. (3) Derived relations are a population of their own, found from function bodies; a hand list missed one
with 16 callers (verified 2026-11-15, §8; found by R1). (4) Thirteen rows that draft 1 left unclassified resolve to three relations this program does
not define, one new flip family and several equalities (census C-8). (5) One site writes a parent, so the
structural guard has one home.

**Ratchet.** After `ta` the population "unqualified parent read" is empty by construction, and the pending
populations can only shrink (the trip-wire). After `ts`, direct field access is confined by visibility; narrowing
the fields from `pub(crate)` to the `dom::tree` module is `ts`'s call.

## §7. Slots, hand-offs and open questions

### §7.1 Slots

| Slot | Why deferred | Trigger | Re-eval |
|---|---|---|---|
| `#11-shadow-root-is-a-tree-root` (registered, PM 747; owner L3) | the program's own slot | **this umbrella's ratify**; closed by `ts` | 2026-12-31 |
| `#11-template-contents-host-link` | not this program's; closes at a small PR after PR (io) (PM 748 (a), amending PM 718), which extends the one host-including test | that PR | the slot's own |
| `#11-dom-api-handlers-unreachable-from-production` (registered, PM 744 / PM 583) | its trigger (b) is PM 748 (b): in `ta` the handlers take the VM natives' relations; connecting or deleting them is decided there | the slot's own | the slot's own |
| `#11-shadow-slot-assignment-flat-tree` (P2, Layout) | owns the flat-tree names of §1.5 and the *assigned slot* arm of the event parent | the slot's own | the slot's own |
| `#11-event-path-window-missing` (L3) | owns `pending_event_parent` in `build_propagation_path` (§1.5) | the slot's own | the slot's own |
| `#11-capped-walker-deep-element-processing` | its trigger (an edit of the walkers in `walkers.rs`) is fired by `ta` and `ts` (R1 named ta / te / ts under draft 1's cut, where `te` held `ancestor_chain`); each plan records the firing; this program changes no bound and no walk order | fired | the slot's own |
| `#11-dom-walkers-iterative-over-ecs-links` | the DOM-walker program (recursive walkers to an iterative form); contact only: both edit `walkers.rs`; order and its slice-1 date: §7.3 Q-17 | the slot's own | the slot's own |
| `#11-shadow-tree-live-range-tracking` | text: live-range adjustment across a shadow-tree boundary; it is where the parent half of the fire guards belongs (§2 MEV × TREE). `tr`'s plan states whether `tr` fires its trigger ("Shadow DOM PR demand") and narrows it | tr's plan-review | 2026-Q4 |
| `#11-shadow-mutation-observer-disconnected-cleanup` | text: an observer registered on a ShadowRoot is not disconnected after the host is collected. Not `tm`'s subject (which ancestors collect, not registration lifetime). **Its Re-eval, 2026-11-10, is overdue** (§7.3 Q-16) | the slot's own | overdue |
| `#11-range-compare-points-shadow-index` | D13: closed by `tc` (PM 747 Q-2: RC-1 drops its D13 part); the ledger's closing slice changes (§7.3 Q-16) | `tc` | the slot's own |
| `#11-live-range-bridge-wide-sibling-index` | neighbour of `tc`: the index's sibling cap; `tc` moves the index into the set and does not change the cap | the slot's own | 2026-12-01 |
| `#11-form-owner-ecs-component` | `tf` edits `find_form_ancestor`, one of its three resolvers: **trigger-fired record in `tf`'s plan**; the `form`-attribute half and the component stay with the slot | fired by `tf` | the slot's own |
| `#11-form-elements-cross-tree` | association across trees by explicit mechanisms; `tf` only stops the ancestor step from crossing, and its plan cites the slot so the two do not contradict | the slot's own | M4-13 entry |
| `#11-fieldset-disabled-dynamic-insert` | neighbour of `tf` (re-deriving inherited disabledness on insert); `tf` changes the relation the derivation uses, not when it runs | the slot's own | the slot's own |
| `#11-vm-isconnected-bound-document` | `ta` edits the native (`node_proto.rs:455`): its trigger ("L3's next DOM node-surface touch") fires. `ta` is behaviour-preserving, so the fix is not in `ta` (§7.3 Q-14) | fired by `ta` | 2026-12-15 |
| `#11-shell-effective-disabled-predicate-adoption` | L4's; `ta` renames two lines in `content/form_input.rs` and adopts nothing | the slot's own | the slot's own |
| `#11-shadowroot-detached-not-despawned-on-host-destroy` | the lifetime program's; `ts` leaves the stripped root as it is | PR (ii) | the slot's own |

The selector relation of §1.5 has no slot yet: pending (PM 748 (c)); L3 probes and drafts one separately.

### §7.2 Hand-offs

- **ecs-entity-lifetime program** (L3; branch `ecs-shadow-root-lifetime-plan`). After `ts`: (1) PR (io) is
  rewritten. Its back-link folds `ShadowRoot.host`. The fold edits `shadow_host`'s body, the one writer in
  `attach_shadow_with_init`, and the three readers inside elidex-ecs that hold the component for another reason
  (census C-5: `shadow.rs:267`, `:421`, `teardown.rs:69`); it edits no climb and no other crate. (Draft 1 said
  "one edit"; R1 found the direct readers.) (2) The "host" cluster leaves PR (io) for the small follow-up PR that
  closes `#11-template-contents-host-link` (PM 748 (a)); that PR widens the host-including test, whose callers are
  listed in §1.2. (3) That umbrella's order line gains this program, and its next revision cites
  `#11-shadow-root-is-a-tree-root` (PM 747).
- **Layout T3 / T3-pre** (L2). T3-pre lands after PR (ig); its FragmentParse arm is a no-op over the window
  ig → T3-pre → [this program] → io → T3, with the cap 2026-12-31 (PM 740, PM 741). **If this program's landing
  would push T3 past that cap, L3 escalates to the PM.** Layout's Q-10 answer (2026-11-15): T3 / T3-pre add no
  parent climb and no raw child read; the seven existing functions they rely on are assigned in §2 (items 1, 2, 4,
  5, 6, 7) and census C-2 (item 3, `despawn_subtree`, raw, `ts`). L3 notifies Layout when `ts`'s plan changes
  `fire_after_insert`'s suppression (item 7), and when `ta` / `tc` rename the accessors T3's plan cites.
- **Layout, render and hit-test order** (§2 ORD × RENDER): the residual at `ts` needs Layout's answer (§7.3 Q-13).
  `crates/core/elidex-render/src/builder/mod.rs` is 1002 lines and `tc` edits two of them (`:483`, `:999`); its
  margin-box code (`:489-993`) is a cohesion seam, so a standalone split precedes `tc` (§7.3 Q-15).
- **Range contents program** (L3). RC-1 lands after `tc`, drops its own D13 part and uses `tc`'s accessors
  (PM 747 Q-2). **RC-1's plan revision gets a re-review of its changed sections** before its code.
- **Event-path plan** (`docs/plans/2026-11-event-path-window.md`). `ta` re-types the loop at
  `crates/script/elidex-script-session/src/event_dispatch/mod.rs:203-221` to `pending_event_parent`; that plan
  replaces the name with its *get the parent* function. Whichever lands second rebases one line.
- **VM P4 / R-NAT.** The R-NAT host chapter's window (2026-11-26 to 12-02) runs in parallel with `ta` (PM 745).
  R-NAT touches `crates/script/elidex-js/src/vm/host/range_proto.rs` (local branch `vm-native-construct-plan`, tip
  `e47adadc` on 2026-11-15, not on origin), which `ta` edits on one line and `tr` on the same function: whichever
  lands second rebases. Draft 1's check covered the `origin/vm-*` branches only. The VM host lines of this program
  are in census C-10.
- **Merge contacts** (branches that edit this program's files): `element-localname-ns`
  (`crates/dom/elidex-dom-api/src/tree_nav.rs`, `node_proto.rs`, `crates/core/elidex-ecs/src/dom/classify.rs`;
  `git diff --name-only c0578223...element-localname-ns`); the serializer program (plan only on
  `html-serializer-escape-conformance`; its code will walk children, so it lands on `tc`'s set or is rebased by
  it).
- **Shell (L4, no driver).** `ta` renames lines in `app/events.rs`, `app/hover.rs`, `content/scroll.rs` and
  `content/form_input.rs` (pending names, no behaviour), and one helper line of
  `crates/shell/elidex-shell/src/tests.rs` (1081 lines, §7.3 Q-15). No flip slice edits a shell file.
- **Brand-check program** (Rev 5.3, frozen). No dependency: its marker reads are identity (census C-5 role I).
- **Near the 1000-line bound** (each slice re-measures at its base and splits first if its edit would cross):
  `tree_nav.rs` 972, elidex-ecs `dom/mod.rs` 959, `range_proto.rs` 906, `components.rs` 856.

### §7.3 Open questions

Answered since draft 1 and written in above: Q-2, Q-3, Q-5, Q-6, Q-9 (PM 747); Q-10 (Layout); Q-4 (§2 MEV × TREE:
the consumers are `LiveRangeRegistry` and T3, the defect's home is `#11-shadow-tree-live-range-tracking`); Q-7
(census C-8). Remaining:

- **Q-1 (plan-review).** `tc` puts the skip into the accessor set and `ts` deletes it, against folding `tc` into
  `ts`. This memo keeps the separate slice (§5.1).
- **Q-8 (plan-review).** Two named steps versus one step with a scope argument (§1.1).
- **Q-11 (PM).** The trip-wire of §1.4 lands with `ta` (the names do not exist before). Is it authored in `ta`'s PR
  by L3 with CI/tools reviewing its form, or by CI/tools as a landing item?
- **Q-12 (PM).** The shell's link and label lookups (census C-8 R-12) approximate the dispatch algorithm's
  activation target. Their owner after `ta`: the event-path plan, or P1 (element lifecycle hooks, which owns
  activation behaviour)?
- **Q-13 (Layout, through the PM).** The residual of §2 ORD × RENDER: at `ts`, a positioned shadow-tree node and a
  positioned slotted light child that preceded the shadow root in the host's child list swap their order in layer
  6. Neither order is the flat tree's. Accept shadow-including order until P2, or does Layout want another rule?
- **Q-14 (PM).** `#11-vm-isconnected-bound-document`'s trigger fires at `ta`. A small PR after `ta` (the native
  calls `is_connected`), or a new date?
- **Q-15 (PM).** Touch-time splits outside L3's files: `builder/mod.rs` (Layout's) before `tc`; the shell's
  `tests.rs` (L4's, no driver; whether it has a seam or is a flat case list is a read `ta`'s plan makes). Who
  cuts them? Also: `tp` puts one more PR before PM 745's "PR-1".
- **Q-16 (PM, ledger).** `#11-shadow-mutation-observer-disconnected-cleanup` is past its Re-eval;
  `#11-range-compare-points-shadow-index` now closes at `tc`.
- **Q-17 (PM).** The DOM-walker program's slice 1 also edits `walkers.rs`, and its target date is relative to a
  lifetime-program landing (PM 694): its order against `ta`, and the date.

## §8. Measurements (provenance, at `c0578223`, run 2026-11-15)

All counts are reproduced by the commands in the census file (C-1 to C-11); the ones this memo quotes:

- P-UP: `git grep -n -P '\bget_parent\(' c0578223 -- crates ':!*tests*' | wc -l` → 143; over `-- crates` → 174; 60
  non-test files, 21 crates.
- P-DER: the script of census C-11 → "level 1: 114 read-only 60 mutating 54", "level 2: 113 read-only 45 mutating
  68"; `is_ancestor` at level 1, `is_ancestor_or_self` at level 2.
- `git grep -n -P '\bis_ancestor_or_self\(' c0578223 -- crates ':!*tests*' | wc -l` → 18 (the definition, one
  comment, 16 calls).
- P-CH: `git grep -n -P '\b(get_first_child|get_last_child|get_next_sibling|get_prev_sibling)\(' c0578223 -- crates ':!*tests*' | wc -l` → 108; over `-- crates` → 142.
- P-MARK: `git grep -n -P '::<&?(\w+::)*(Ecs)?ShadowRoot>|\bis_shadow_root\(' c0578223 -- crates ':!*tests*' | wc -l` → 63.
- P-REL: `git grep -n -P '\.(parent|first_child|last_child|next_sibling|prev_sibling)\b(?!\s*\()' c0578223 -- crates/core/elidex-ecs/src ':!*tests*' | wc -l` → 41.
- P-WR: `git grep -n -P '\.parent\s*=[^=]' c0578223 -- crates/core/elidex-ecs/src ':!*tests*' | wc -l` → 3;
  `git grep -n -P '\blink_node\(' c0578223 -- crates ':!*tests*' | wc -l` → 4.
- P-HOST: `'\.host\b(?!\s*\()'` → 41 (14 in shadow-related files, 11 of them reads).
- File sizes: `wc -l` on the files of §5 `tp` and §7.2.
- Coverage map: `.claude/tools/webref coverage-map dom 1.1 dom 2.7 dom 4.2.2 dom 4.2.3 dom 4.3.2 dom 4.4 dom 4.7 dom 4.8 dom 4.9 dom 5.2 dom 5.6 dom 6 selection-api 3 html 4.12.3 html 4.10.18.3 html 4.10.15 html 4.10.19.5 html 4.10.10 html 4.10.22.3 html 6.8.1 html 6.3 html 3.2.6.2 html 3.2.6.4 selectors-4 14.1 css-shadow-1 3.2.2 css-shadow-1 4.1 cssom-view-1 7`.
- Not measured: anything at runtime (§5.2); the per-call relation (census C-7).

## §9. Citation table

Each row was looked up with `.claude/tools/webref` on 2026-11-15 (`dfn` / `heading` for the number ↔ title pair,
`body` for the quoted words). Census C-8 holds the further lookups for single rows.

| ID | Citation | Anchor | Used by |
|---|---|---|---|
| [D1] | WHATWG DOM §1.1 Trees — *parent*, *root*, *inclusive ancestor*, *index*, *tree order* | `dom#concept-tree-parent`, `dom#concept-tree-order` | §1, §3 |
| [D2] | WHATWG DOM §4.2.2 Shadow tree — "shadow tree is a node tree whose root is a shadow root"; "A node is connected if its shadow-including root is a document" | `dom#concept-shadow-tree`, `dom#connected` | §1, §2, §3, §4 |
| [D3] | WHATWG DOM §4.7 Interface DocumentFragment — "host (null or an element in a different node tree)"; *host-including inclusive ancestor* | `dom#concept-documentfragment-host` | §1.2, §3, §4 |
| [D4] | WHATWG DOM §4.8 Interface ShadowRoot — *shadow-including root*; *shadow-including inclusive ancestor*; *shadow-including tree order*; *retarget* ("A's root is not a shadow-including inclusive ancestor of B"); a shadow root's *get the parent* (quoted in §1.5) | `dom#concept-shadow-including-root`, `dom#concept-shadow-including-inclusive-ancestor`, `dom#concept-shadow-including-tree-order`, `dom#retarget` | §1.1, §1.5, §3 |
| [D5] | WHATWG DOM §4.2.3 Mutation algorithms — *ensure pre-insert validity* step 2 ("If node is a host-including inclusive ancestor of parent, then throw a \"HierarchyRequestError\" DOMException"); *move* step 2 ("… of newParent …") | `dom#concept-node-ensure-pre-insertion-validity`, `dom#move` | §1.2, §2, §3 |
| [D6] | WHATWG DOM §4.3.2 Queuing a mutation record — step 2 ("Let nodes be the inclusive ancestors of target") | `dom#queue-a-mutation-record` | §2, §3 |
| [D7] | WHATWG DOM §4.4 Interface Node — "The parentNode getter steps are to return this's parent"; "The contains(other) method steps are to return true if other is an inclusive descendant of this; otherwise false"; `compareDocumentPosition` step 6; a node's *get the parent* (quoted in §1.5) | `dom#dom-node-parentnode`, `dom#interface-node` | §1.5, §2, §3 |
| [D8] | WHATWG DOM §5.6 Interface Range — "The root of a live range is the root of its start node"; *get the common ancestor* step 2; `setStartBefore` steps 1-2 | `dom#concept-range-root`, `dom#get-the-common-ancestor`, `dom#dom-range-setstartbefore` | §2, §3 |
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
| [S1] | CSS Selectors L4 §14.1 Descendant combinator ( ) | `selectors-4#descendant-combinator` | §3 |
| [S2] | CSS Selectors L4 §9.1 The Pointer Hover Pseudo-class: :hover — "An element also matches :hover if one of its descendants in the flat tree (including non-element nodes, such as text nodes) matches the above conditions." | `selectors-4#hover-pseudo` | §1.5 |
| [C1] | CSS Shadow Module Level 1 (css-shadow-1) §4.1 Flattening the DOM into an Element Tree — *flat tree* | `css-shadow-1#flat-tree` | §1.5, §3 |
| [C2] | CSS Shadow Module Level 1 (css-shadow-1) §3.2.2 Selecting Shadow Hosts from within a Shadow Tree — "For the purpose of Selectors, a shadow host also appears in its shadow tree, with the contents of the shadow tree treated as its children. (In other words, the shadow host is treated as replacing the shadow root node.)" | `css-shadow-1#host-element-in-tree` | §1.5, §3 |
| [C3] | CSS Shadow Module Level 1 (css-shadow-1) §3.2.1 Matching Selectors Against Shadow Trees — "the selector match list is initially the shadow host, followed by all children of the shadow tree's shadow root and their descendants" | `css-shadow-1#selectors-data-model` | §1.5 |
| [V1] | CSSOM View (cssom-view-1) §7 Extensions to the HTMLElement Interface — `offsetParent` step 2: "Let ancestor be the parent of the element in the flat tree and repeat these substeps" | `cssom-view-1#dom-htmlelement-offsetparent` | §1.5, §3 |
