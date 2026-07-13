# Terminal-Z C-3 — LayoutBox consumer migration (cross-crate architecture plan-memo)

**Status**: pre-`/elidex-plan-review` design anchor. Doc-only; no code churn. Written off a
first-hand code-read of the LIVE store (`crates/core/elidex-ecs/src/fragment_tree.rs`) +
consumer surface, 2026-07-13. First-principles anchor for the C-3 slice of the terminal-Z
committed-next program (`memory/terminal-z-committed-next-fragment-walk-plan.md`).

Predecessors MERGED: Z-1a (#313/#314, standalone `FragmentTree`) / Z-1b (#316, per-column
`InlineFlow`) / **C-1** (#321 `48b0190b`, render consumes the store for `consumable` mid-break IFC
entities: per-column chrome+clip+content) / **C-2** (#324 `b4e06897`, atomic-as-fragment). C-3 = migrate
the remaining
**non-paint** `LayoutBox` readers (CSSOM geometry / IntersectionObserver / hit-test / a11y /
baseline / shell) off the single per-entity `LayoutBox` onto the fragment store, so C-4 can
retire `LayoutBox` + the legacy inline pipeline.

---

## §0 Premise correction (READ FIRST — supersedes the shared memo UPDATE 2026-07-13)

The `terminal-z-committed-next-fragment-walk-plan` **UPDATE 2026-07-13** states: *"FragmentStore
RELOCATED elidex-ecs → elidex-render (#413); elidex-layout does NOT depend on elidex-render …
even in-layout consumers cannot read it … C-3 needs a FragmentStore-access architecture decision
(consumers depend on elidex-render vs low-crate projection)."* **That premise is WRONG** — it was
authored from tool output captured during a harness I/O-instability window that fabricated content
for non-existent paths (e.g. reads of a purported `elidex-render` src/fragment_tree.rs and a
`css-tables` crate — neither is a tracked path). The authoritative live state (`git ls-files`,
first-hand read):

- **The fragment store lives in `crates/core/elidex-ecs/src/fragment_tree.rs`** — the single tracked
  `fragment_tree.rs` in the repo. It is **not** in `elidex-render`, and **no relocation ever occurred**:
  the UPDATE's cited "`#413`" is in fact "MutationObserver transient registered observers" (commit
  `bcee298b`), unrelated to any fragment move — corroborating that the relocation citation is itself
  fabricated (as is the `css-tables` crate it names, which is not tracked).
- It is a **sibling field of `EcsDom`**: `EcsDom { … fragment_tree: FragmentTree }`
  (`crates/core/elidex-ecs/src/dom/mod.rs:50,75`) with `fragment_tree()` / `fragment_tree_mut()`
  accessors (`:148,:154`).
- `elidex-ecs` is the **lowest core crate** — every consumer crate already depends on it. There is
  **no dependency wall** and **no relocation is needed**: every C-3 consumer can already reach the
  store via `EcsDom`.

**Consequence for the (a)/(b)/(c) framing PM posed** ("consumers depend on elidex-render" vs
"low-crate projection" vs "other"): options (a) and (b) were framed around a store-in-render that
doesn't exist. With the store already in the universally-depended-upon `elidex-ecs`, the store-access
question **dissolves**. The real, remaining decision is **the shape of the consumer-facing projection
and where its method lives** — answered in §1 (option ≈ c: a projection method on `EcsDom`).

**What the UPDATE got RIGHT and still holds**: C-3 is genuinely **cross-crate as CODE** (readers span
dom-api / api-observers / layout-* / a11y / shell / render) and therefore **not layout-only and not
parallel-safe with the CSS/script/shell lanes** — it must be scheduled as coordinated sub-slices
(§7), not a single PR. Only the *dependency-wall / relocate-store* mechanism was wrong.

> Action: PM to correct the shared memo UPDATE (I do not edit the shared SoT). See §11.

---

## §1 Ideal architecture (first-principles)

**End-state**: geometry consumers read an entity's box geometry as **the sequence of its box
fragments**, obtained through a single fragment-aware projection on `EcsDom` — never by reaching for
the raw `LayoutBox` component. The common non-fragmented entity is a **1-fragment** sequence; a
multicol mid-break entity is an **N-fragment** sequence. `LayoutBox` becomes an internal producer
detail that C-4 can delete once no consumer names it.

**Why a projection on `EcsDom` (not each consumer touching the store/component directly):**

1. **`EcsDom` already owns both stores.** The `World` (holding the `elidex_plugin::LayoutBox`
   per-entity component) and the `FragmentTree` (the N:M sibling field) are both fields of `EcsDom`
   in `elidex-ecs` (`dom/mod.rs:50`). The fold "N fragments if present, else the single `LayoutBox`"
   can only be expressed where both are in scope — that is `EcsDom`. Putting it anywhere else
   re-plumbs one of the two stores across a crate boundary for no reason.
2. **`BoxModel` already unifies the two geometries.** Both `elidex_plugin::LayoutBox` and
   `elidex_ecs::BoxFragment` implement `BoxModel` (content/padding/border/margin), and
   `impl From<&LayoutBox> for BoxFragment` is *already* the single source of the field correspondence
   (`fragment_tree.rs:131,146`). So the projection can yield a uniform `BoxModel` (or `BoxFragment`)
   sequence with **zero new type machinery** — the N=1 case borrows the `LayoutBox` and projects it
   through the existing `From`.
3. **One-issue-one-way.** The dual-read ("has fragments? → tree; else → component") is a *decision
   surface* every consumer would otherwise re-implement (18-ish sites across 6 crates, each a chance
   to use the wrong signal — see §4). Concentrating it in ONE `EcsDom` method makes every consumer a
   thin adapter over a single fold. This is the CLAUDE.md "sole Script↔ECS boundary" / "EcsDom is the
   single owner/reader-projection" discipline applied to geometry.
4. **ECS-native.** The store is already the canonical N:M home (§15.4.1); the projection is the read
   side of that boundary. No component-ization of the N:M relation (which §15.4.1 forbids), no
   side-store, no new registry.

**The projection primitive (proposed):**

```
impl EcsDom {
    /// Border/padding/content boxes for `entity`, one per box fragment, in
    /// fragmentainer order. Fragment store is authoritative when present
    /// (positive presence is the router — never LayoutBox-absence); otherwise
    /// the single LayoutBox component projected as one fragment. Empty iff the
    /// entity has neither (no layout box) — callers map that to the spec's
    /// "no associated CSS layout box" branch.
    pub fn box_fragments(&self, entity: Entity) -> impl Iterator<Item = BoxFragment> + '_;
}
```

with **spec-anchored** helpers layered on top — each defined by *its consumer's CSSOM algorithm*, NOT
a generic border-box reduction (the review's root finding: a generic-`Rect` helper set silently drops
box-model facets and mis-branches; each helper must encode the exact §-algorithm). Because
`box_fragments` yields the **full `BoxFragment`** (which impl's `BoxModel` → `.border_box()` /
`.padding_box()` / `.border()` / `.first_baseline` all derivable), every facet below is a fold over the
same primitive — no primitive change, only the correct reductions (**one exception**: `client_rects`
also draws on the `InlineClientRects` component for line boxes the store does not hold — see its bullet):

- `principal_padding_box` / `principal_border_widths` / `principal_baseline(entity) -> Option<_>` — the
  **first (principal)** fragment's padding box / raw border widths / `first_baseline` (client*/baseline
  semantics, §3). Note the flex/grid baseline readers co-read the principal fragment's
  `content().origin.y` alongside the baseline (`lib.rs:477-478` / `position.rs:447-448`), so C-3c takes
  both off the same principal `BoxFragment` (both are on it) — `principal_baseline` alone is insufficient.
- `offset_border_box_union(entity) -> Option<Rect>` — **union (axis-aligned bbox) of the principal
  box's fragment border boxes** (`offsetWidth`/`offsetHeight`, CSSOM VIEW §7 step 2 — a UNION, not the
  first fragment; note `offsetTop/Left` are first-box, so width/height and top/left take *different*
  helpers).
- `bounding_box(entity) -> Rect` — the full **4-branch** "get the bounding box" (CSSOM VIEW §6): empty
  list → all-zero rect; all rects zero-w/h → first rect; else union over the **non-zero** rects only.
  For `getBoundingClientRect` (which spec-mandates the concrete zero rect for boxless). **IntersectionObserver
  does NOT share this** — see the boxless contract below.
- `optional_bounding_box(entity) -> Option<Rect>` — same union but **`None` for a boxless entity** (no
  fragments, no `LayoutBox`), for the observer/a11y consumers (below).
- `client_rects(entity) -> impl Iterator<Item = Rect>` — the `getClientRects` rect list. ⚠ **two
  sources, mutually-exclusive dispatch — NOT a union** (F6 + re-check): `box_fragments` returns an N=1
  whole-border-box for *every* entity with a `LayoutBox` (§2), **including inline-multi-line ones**, so
  a literal per-line ∪ per-column would double-count the whole box and regress the common inline case.
  **Precedence rule** (generalizing the live if/else, `layout_query.rs:219-238`): if the entity has
  `InlineClientRects` → return its per-**line** rects and **suppress** the `box_fragments` projection;
  else if it has store fragments → per-**column** border boxes; else the single `LayoutBox` border box.
  The store holds NO line-box fragments, so line-boxes stay on `InlineClientRects` until C-4. See the §9
  dispatch table (authoritative) and §7-C-3b.

**Boxless contract (I-boxless, load-bearing — P1/P2 from the Codex review)**: a boxless entity
(display:none / pre-layout: no fragments AND no `LayoutBox`) splits consumers into two classes that must
NOT be collapsed onto one helper:
- **spec-zero** (`getBoundingClientRect`→all-zero `Rect`, `getClientRects`→empty list): CSSOM mandates a
  concrete empty/zero result. → `bounding_box` / `client_rects`.
- **Option-None** (IntersectionObserver, ResizeObserver, a11y bounds): these branch on *"is there a
  box?"* — a zero-rect is NOT the same as no-box. IO treats `None` as the required initial false/ratio-0
  observation; a11y skips `set_bounds` when there is no box. Feeding them `bounding_box`'s zero-rect
  regresses both (a boxless origin target reads as an intersecting zero-area box; a boxless node gains a
  spurious `(0,0,0,0)` AX bound). → `optional_bounding_box` (None-preserving), never `bounding_box`.

Consumers call the helper matching their spec, never the raw component or the raw tree. `scrollTop/Left`
(scroll *offset*) read `ScrollState`, unchanged — **out of C-3 scope**. `scrollWidth/Height` route to
`principal_padding_box` only as a **behavior-neutral preservation of today's pre-existing limitation**:
CSSOM VIEW §6.1 scrollWidth **step 7** returns the *scrolling area* width (padding box **extended by
descendant overflow**, ≥ clientWidth), but elidex already computes it padding-box-only
(`layout_query.rs:159-170`), and `BoxFragment` carries no scrollable-overflow facet — so C-3 keeps the
padding-box value (no regression) and does NOT claim §6.1 correctness. Full scrolling-area fidelity is a
separate pre-existing gap, out of C-3 scope.

**Home** (F8): the projection `impl EcsDom` block lands in a **new `crates/core/elidex-ecs/src/dom/
geometry.rs`** (it needs only `EcsDom`'s private `world` + `fragment_tree`), NOT appended to
`dom/mod.rs` (already 1073 LoC — CLAUDE.md touch-time-split; the program carries `task_2924ead0`).

---

## §2 N=1 fast-path (§5-Q3 of the anchor)

The overwhelmingly common entity has **no** store fragments (only `consumable`/mid-break boxes are
pushed; `push_box` is called only by the multicol committer). The fast-path must not allocate or
change behavior for it.

- `box_fragments` checks `fragment_tree().fragments_for(entity)` first. That index lookup is
  **O(1)** (the D-Z7 `HashMap<Entity, Vec<FragmentId>>` index, `fragment_tree.rs:52,260`) and returns
  an **empty** iterator for the non-fragmented entity → the method yields the single `LayoutBox`
  (component get + `From<&LayoutBox>` borrow projection), **no Vec, no heap**.
- Iterator-based (not `Vec`-returning) keeps the N=1 path a borrow. The union/first helpers
  short-circuit on the 1-element case.
- **Behavior-neutral invariant** for C-3a: for every non-fragmented entity, each helper reduces to the
  single `LayoutBox`'s corresponding facet bit-for-bit — `offset_border_box_union` / `bounding_box` /
  the sole `client_rects` element / `principal_*` all == today's `get_border_box(LayoutBox)`-derived
  value (the N=1 sequence has one element, so union==first==that element). This is the regression gate —
  the seam slice changes call *routing*, not values, for N=1.

Open: confirm the borrow/lifetime composes with `EcsDom`'s access model (the geometry handlers take
`&mut EcsDom` today, `layout_query.rs:24`; the projection needs only `&EcsDom`). Likely trivial (read
path), verify at C-3a impl.

---

## §3 Per-fragment vs principal-box consumer split (spec-driven — the crux of "which consumers become N-aware")

**Not every consumer becomes N-aware.** CSSOM View assigns different box semantics per API; the plan's
correctness hinges on routing each consumer to the right helper:

Each consumer routes to its **own** CSSOM algorithm (not a shared border-box reduction) — the box-model
facet and the union-vs-first-vs-per-fragment behavior differ per row and must NOT be collapsed:

| Consumer | Helper (facet + reduction) | Fragmented (N>1) behavior | Spec |
|---|---|---|---|
| `getClientRects()` | `client_rects` — **two-source, dispatch (not union)** | `InlineClientRects`→per-**line** (suppresses box projection); else store→per-**column**; else single box | CSSOM VIEW §6 getClientRects |
| `getBoundingClientRect()` | `bounding_box` — **4-branch** | empty→all-zero; all-zero-w/h→**first rect**; else union over **non-zero** rects only | CSSOM VIEW §6 "get the bounding box" |
| `IntersectionObserver` + `ResizeObserver` | `optional_bounding_box` (**None-preserving**, NOT `bounding_box`) | 4-branch when boxed; **`None` when boxless** (initial false/ratio-0) | intersection-observer §3.2.7 step 1 "get the bounding box for target" |
| `offsetWidth`/`offsetHeight` | `offset_border_box_union` | **UNION** (axis-aligned bbox) of principal box's fragment border boxes | CSSOM VIEW §7 offsetWidth **step 2** |
| `offsetTop`/`offsetLeft` | principal (first) box, offsetParent-relative | **first** box | CSSOM VIEW §7 (asymmetry: Top/Left first, Width/Height union) |
| `clientWidth`/`clientHeight` | `principal_padding_box` | first box **padding** box | CSSOM VIEW §6.1 |
| `clientTop`/`clientLeft` | `principal_border_widths` | first box **border widths** | CSSOM VIEW §6.1 |
| `scrollWidth`/`scrollHeight` | `principal_padding_box` (**pre-existing padding-box limitation**, not §6.1 scrolling-area) | first box padding box (today's value, behavior-neutral) | CSSOM VIEW §6.1 step 7 (scrolling area — NOT met, pre-existing) |
| `scrollTop`/`scrollLeft` (scroll **offset**) | `ScrollState` — **OUT OF C-3 SCOPE** | unchanged (not a LayoutBox read) | — |
| baseline (flex/grid cross-read) | `principal_baseline` | first fragment's `first_baseline` | — (engine-internal alignment) |
| hit-test | per-fragment (`client_rects`-style) | hit any fragment | — (paint-consistent) |
| a11y bounds | `optional_bounding_box` (**None-preserving**) | bbox when boxed; **skip `set_bounds` when None** (no spurious 0-rect) | — (AX node bounds) |
| render paint walk | already C-1/C-2 (`is_consumable`) | per-fragment content | css-break-3 §5.4 / css-multicol-1 §8.1 |

**Net-new spec-correctness wins** (today the single last-column `LayoutBox` = WRONG for multicol):
`getClientRects` per-column-∪-per-line, `getBoundingClientRect`/IO 4-branch bbox, **`offsetWidth/Height`
union** (a genuine fix, NOT near-noop — the review corrected an earlier "first-box" mis-statement),
per-fragment hit-test. **Genuinely near-noop** (facet-preserving, first==single for N=1): `offsetTop/
Left`, `client*`, `scrollWidth/Height` — routing-delta only, their job is to stop naming `LayoutBox` so
C-4 can delete it (§9 marks these "routing-delta only", not "full enum").

---

## §4 Router-signal correctness (a subtle CORRECTNESS trap the plan must pin)

The store carries **two** distinct signals; C-3 consumers must use the right one:

- **`is_consumable(entity)`** (`fragment_tree.rs:279`) — `true` iff a `push_box` passed
  `consumable=true`, i.e. the box's mid-break IFC lines were drained into a per-column carrier (a
  *direct-child IFC mid-break*). This is the **render paint** router (C-1): it decides per-fragment
  *content* emission. A nested-block / deeper-IFC mid-break has box fragments but `consumable=false`.
- **`fragments_for(entity)` non-empty** — the box was fragmented at all (has ≥1 store box fragment),
  regardless of carrier drain.

**For CSSOM / hit-test / a11y the correct router is `fragments_for non-empty`, NOT `is_consumable`.**
A nested-block mid-break (`consumable=false`) still occupies **N column boxes** that `getClientRects`
must return and hit-test must probe. Using `is_consumable` here would silently drop the geometry of
every non-carrier multicol fragment. The `box_fragments` primitive therefore routes on *presence*
(`fragments_for`), and the paint walk's `is_consumable` stays a paint-only concern. The plan states
this explicitly so no sub-slice copies the render router by reflex.

---

## §5 Coordinate space

**Load-bearing INVARIANT (I-coord)** — `BoxFragment.content` origin is in the **same physical/document
space** as `LayoutBox.content` origin. This is not an assumption to "verify later"; it is the invariant
every migrated CSSOM reader depends on (all feed the existing `accumulated_scroll_offset`
document→viewport subtraction, `layout_query.rs:30`, which requires document-space input). It holds by
construction, on two citations:
- N=1 arm: `From<&LayoutBox> for BoxFragment` copies `content` **verbatim** (`fragment_tree.rs:154`) —
  identical space trivially.
- N>1 arm: `shift_entity`'s contract equates the fragment origin with `LayoutBox.content.origin`
  physical space (`fragment_tree.rs:286-288`, mirroring block layout's `shift_descendants` LayoutBox
  arm `:285`), and `BoxFragment.content` is born-absolute with the column offset baked at commit
  (`:99-109`).

So the **existing scroll-subtraction + `BoxModel` border-box derivation apply unchanged** per fragment —
the projection swaps *which* box(es) feed the existing conversion, not the conversion. **C-3b gate**: a
regression test asserting a multicol element's `getBoundingClientRect` union in viewport coords is the
invariant's executable check; C-3b does not land until it passes (this is the invariant's proof, not a
deferral).

---

## §6 Consumer surface (corrected — real crates/paths, verify reader/writer split per sub-slice)

`LayoutBox` (= `elidex_plugin::LayoutBox`, `crates/core/elidex-plugin/src/layout_types/boxes.rs`)
reference counts by crate (`git grep -l 'LayoutBox'`, readers **and** writers):

| crate | refs | role (readers = C-3 targets; writers = C-4 producer concern) |
|---|---|---|
| `crates/layout/elidex-layout-block` | 25 | mostly **producer** (inline pack writes LayoutBox) + hit-test/baseline readers |
| `crates/core/elidex-render` | 24 | paint walk (C-1/C-2 in progress; single-`LayoutBox` arm G11 remains) |
| `crates/layout/elidex-layout-multicol` | 10 | producer (the `push_box` committer) |
| `crates/layout/elidex-layout-table` | 9 | producer |
| `crates/script/elidex-js` | 7 | **observer-geometry READERS** (not just a CSSOM bridge): ResizeObserver `vm/host/resize_observer.rs:405` + IntersectionObserver `vm/host/intersection_observer.rs:488-490` read `LayoutBox` before the api-observers registry |
| `crates/shell/elidex-shell` | 6 | **multiple readers**: scroll-extent aggregate (`content/scroll.rs:133-148`), iframe click-coord xlate (`content/event_handlers.rs:834-840`), lazy-iframe visibility (`content/iframe/lifecycle.rs:263-274`) — NOT just viewport/scroll |
| `crates/layout/elidex-layout-flex` | 6 | producer **+ baseline READER** (`src/lib.rs:473-479` reads another entity's `LayoutBox.first_baseline` for alignment) |
| `crates/layout/elidex-layout-grid` | 4 | producer **+ baseline READER** (`src/position.rs:444`) |
| `crates/layout/elidex-layout` | 4 | hit-test / baseline (reader) |
| `crates/core/elidex-plugin` | 4 | the `LayoutBox` type + `BoxModel` (definition) |
| `crates/core/elidex-ecs` | 2 | the `From<&LayoutBox> for BoxFragment` projection + store |
| `crates/dom/elidex-dom-api` | 1 | **CSSOM geometry** (`element/layout_query.rs`) — the primary reader cluster |
| `crates/dom/elidex-a11y` | 1 | AX bounds (reader) |

> ⚠ **The `git grep -l 'LayoutBox'` counts are a LOWER BOUND** — they miss reads via the **qualified**
> path `elidex_plugin::LayoutBox` (e.g. `resize_observer.rs:405`) and reads via a bounding-rect closure.
> So **C-3a's first deliverable is a complete read-site audit** (grep `get::<&(elidex_plugin::)?LayoutBox>`
> + closure `rect_fn` sites across ALL crates), because an unmigrated reader silently blocks C-4. The
> Codex review of this memo already surfaced five the grep missed (below).

**Reader clusters to migrate (C-3 scope)** — grounded, incl. the Codex-surfaced misses:
- **CSSOM geometry** — `crates/dom/elidex-dom-api/src/element/layout_query.rs`: `get_border_box`
  (`:26,:68,:82`), `get_padding_box` (`:121-131`)/border-width (`:135-151`), `offset_from_parent`
  (`:81-89`, the offset* home — **lives here, not in `elidex-ecs`**, see §7-C-3a). `getClientRects`
  **already exists** (`:201-240`, two-source) — a **fix** (resolves §8-Q2).
- **Observer geometry (script-host + api)** — `getClientRects`-independent: **ResizeObserver**
  (`elidex-js/src/vm/host/resize_observer.rs:405`), **IntersectionObserver** host closure
  (`.../intersection_observer.rs:488-490`) + the `elidex-api-observers` registry
  (`intersection/mod.rs`). ⚠ **Option/None-preserving**: `gather_observations` treats `rect_fn → None`
  (boxless target: display:none / pre-layout) as the *required* initial false/ratio-0 entry
  (`intersection/mod.rs:298-345`, pinned `tests_core.rs:295-317`). The projection MUST return
  `Option<Rect>` (None for boxless), NOT `bounding_box`'s all-zero `Rect` — else a boxless target at the
  origin reads as an intersecting zero-area box (P1).
- **hit-test** — `elidex-layout/src/hit_test.rs`: per-fragment, but the **transform/perspective basis is
  the element's single border box** (`:130-172`); C-3c must fix ONE transform reference box and test the
  projected fragments in that transformed space (not recompute the transform per raw fragment rect) (P2).
- **a11y** — `elidex-a11y/src/tree.rs:121-126`: **only calls `set_bounds` when a `LayoutBox` exists** →
  needs an **Option-returning** bounds helper, NOT `bounding_box`'s zero-rect (else boxless nodes flip
  from "no bounds" to a real `(0,0,0,0)` in the AccessKit tree) (P2).
- **shell** — three readers: scroll extent is a **document-wide max over EVERY visible box**
  (`content/scroll.rs:133-148`, `compute_content_extent`) → needs an **all-fragment/all-entity extent**
  projection, NOT the principal box (multicol later columns would be dropped, P2); iframe click-coord
  xlate (`content/event_handlers.rs:834-840`) + lazy-iframe visibility (`content/iframe/lifecycle.rs:263-274`).
- **flex/grid baseline cross-read** — `elidex-layout-flex/src/lib.rs:473-479` + `/src/baseline.rs:18-26`
  (align-items:baseline; reads padding/border **and** `first_baseline` for a margin-box cross-start
  offset) + `elidex-layout-grid/src/position.rs:444`. Needs `principal_baseline` **plus** the principal
  fragment's padding/border facets + `content().origin.y` (P2) — not raw baseline alone.
- **render** — the residual single-`LayoutBox` arm (folds into C-4, not C-3).

> The producer sites (layout-* *writers*) are a **C-4** concern (every producer must write the store's
> N=1 box for every entity before `LayoutBox` can be deleted, §5-Q3 of the anchor) — **out of C-3
> scope**. C-3 only moves *readers* (incl. the flex/grid baseline reads above, which are reads despite
> living in producer crates). **Reader-only invariant (F7)**: no C-3 sub-slice touches a producer
> write — carried into each C-3a…e plan-review.

---

## §7 Consumer-cluster sub-slicing (each a shippable, coordinated PR)

The migration is large and cross-crate; slice by consumer cluster, seam-first, each
behavior-neutral-or-spec-fix, in dependency order:

- **C-3a — the projection seam** (`elidex-ecs` only, new `dom/geometry.rs`): add `EcsDom::box_fragments`
  + the spec-anchored helper set (§1), **plus the complete read-site audit** (§6). The helpers are
  connected-not-dead via **unit tests** exercising each fold (union / 4-branch / two-source / N=1
  fast-path / the §2 behavior-neutral invariant) against fixture entities — NOT by migrating a consumer
  (offset* was floated as the proof, but it lives in `elidex-dom-api`'s `offset_from_parent`, `:81-89`,
  so migrating it would break C-3a's `elidex-ecs`-only isolation; the offset* migration is C-3b's, P2).
  The **derisking slice**; lowest blast radius.
- **C-3b — CSSOM geometry** (`elidex-dom-api`): the spec-heavy slice. Route `getBoundingClientRect`→
  `bounding_box` (4-branch), `offsetWidth/Height`→`offset_border_box_union` (**union**), `offsetTop/Left`
  → principal box (keeping `offset_from_parent`'s offset-parent walk, `:81-89`), `clientWidth/
  Height`→`principal_padding_box`, `clientTop/Left`→`principal_border_widths`, `scrollWidth/Height`→
  padding box (pre-existing limitation, §1). **`getClientRects` = FIX** (`:201-240` exists): reconcile its two sources — per-**line**
  `InlineClientRects` (kept until C-4) with per-**column** `box_fragments` — do NOT regress inline
  multi-line rects by routing to columns alone (F6). Own `/elidex-plan-review` + Codex converge (dense
  cssom-view + 4-branch + two-source edges); its coupled invariants (union-vs-first, facet-per-API,
  line-vs-column, I-coord) enumerated in that per-PR plan's §2.
- **C-3c — hit-test + a11y + baseline** (`elidex-layout` + `elidex-a11y` + `elidex-layout-flex`/`-grid`):
  - **hit-test**: fix ONE transform reference box (the element's border box, `hit_test.rs:130-172`) as
    today, then inverse-point-test the projected per-fragment rects in that transformed space — do NOT
    recompute transform/perspective per raw fragment (would shift transform-origin basis, P2).
  - **a11y**: an **Option-returning** bounds helper; keep `tree.rs:121-126`'s "set_bounds only when a box
    exists" guard — do NOT feed `bounding_box`'s zero-rect (boxless nodes must stay "no bounds", P2).
  - **baseline**: `elidex-layout-flex` (`lib.rs:473-479` **and** `baseline.rs:18-26`, the
    align-items:baseline margin-box offset reading padding/border **+** `first_baseline`) + `-grid`
    (`position.rs:444`) → `principal_baseline` **plus** the principal fragment's padding/border facets +
    `content().origin.y` (P2). (Reads despite living in producer crates — §6.)
- **C-3d — observers + shell** (`elidex-js` host + `elidex-api-observers` + `elidex-shell`):
  - **IntersectionObserver** (`api/intersection/mod.rs` + host `intersection_observer.rs:488-490`) +
    **ResizeObserver** (host `resize_observer.rs:405`) → an **`Option<Rect>`-preserving** projection
    (None for boxless), NOT `bounding_box`'s zero-rect — keeps the required initial false/ratio-0
    observation for boxless targets (`intersection/mod.rs:298-345`, pinned `tests_core.rs:295-317`, P1).
  - **shell scroll extent** (`content/scroll.rs:133-148`): an **all-fragment/all-entity max-extent**
    projection (every visible box, incl. later multicol columns) — NOT the principal box (P2).
  - **shell iframe/event** (`content/event_handlers.rs:834-840` click-coord xlate,
    `content/iframe/lifecycle.rs:263-274` lazy visibility): migrate too, else fragmented iframes keep a
    stale single box for event routing / lazy-load (P2).
- **C-3e — render residual** : fold the single-`LayoutBox` paint arm (G11) into the fragment walk for
  the non-`consumable` mid-break case (closes the last non-C-4 reader).
- **→ C-4** (separate program): retire `LayoutBox` + legacy inline pipeline + `InlineClientRects`,
  once §6's reader table has zero `LayoutBox` refs outside producers, and producers write the store's
  N=1 box for every entity.

Ordering rationale: C-3a is the seam all others consume; C-3b is highest-value (spec fixes) and
proves the projection against the richest consumer; C-3c/d are mechanical once the seam + helpers
exist; C-3e closes render. Each slice is independently `/elidex-plan-review`'d per the anchor's §6 and
the edge-dense discipline.

**Coordination**: C-3b touches `elidex-dom-api` (contends with DOM/CSSOM lanes), C-3c the a11y +
layout lanes, C-3d the api/shell lanes. These are **not parallel-safe as code** and must be
PM-scheduled against the active lanes; C-3a (elidex-ecs, additive) is the most isolatable.

---

## §8 Open questions (settle at each sub-slice's plan-review)

1. **`box_fragments` receiver**: `&EcsDom` vs `&mut EcsDom` (handlers hold `&mut` today). Read-only
   projection wants `&EcsDom`; confirm no borrow conflict with the handler signature.
2. **RESOLVED (was: getClientRects add-vs-fix)**: the handler **exists** (`layout_query.rs:201-240`,
   two-source: `InlineClientRects` per-line + border-box fallback) → C-3b is a **fix + two-source
   reconciliation** (§7-C-3b), not an add. (Earlier "not found" was a truncated `:1-120` read.)
3. **RESOLVED (was: offset* union-vs-first)**: CSSOM VIEW §7 splits them — `offsetWidth/Height` = UNION
   of the principal box's fragments (step 2), `offsetTop/Left` = first box. Both encoded in §3 (not the
   earlier "offset* = first" collapse).
4. **RESOLVED (was: baseline projection)**: `principal_baseline(entity)` (first fragment's
   `first_baseline`) is **committed** to the helper set (§1) and its readers (flex/grid cross-read)
   assigned to C-3c (§7) — no longer conditional.
5. **`accumulated_scroll_offset` per fragment**: for N>1, is the scroll offset identical across
   fragments (same scroll ancestors)? Almost certainly yes (one entity, one ancestor chain) — verify,
   else the union must subtract per-fragment.
6. **hit-test z-order across fragments**: per-fragment hit must preserve paint order; confirm the
   fragment iteration order (`fragmentainer` order) matches paint order for hit resolution.

---

## §9 Spec coverage map (§3-discipline table — citations webref-verified 2026-07-13)

Branch enumeration for the CSSOM readers C-3 migrates — the load-bearing correctness surface: the
**empty / no-box branches** the projection's `Option`/`None` arms must cover, and the **union-vs-first**
split. (Anchors: §6 `#extension-to-the-element-interface`, §7 `#extensions-to-the-htmlelement-interface`,
§6.1 `#element-scrolling-members`.)

| Spec section | Step | Branch | Touch (dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSSOM VIEW §6 Extensions to the Element Interface | `getClientRects()` | (a) inline multi-line → per-**line** rects (`InlineClientRects`) | `layout_query` getClientRects (`:201-240`, two-source) → `client_rects` (NEW) | ✓ | no |
| CSSOM VIEW §6 Extensions to the Element Interface | `getClientRects()` | (b) multicol (no `InlineClientRects`) → per-**column** border boxes (`box_fragments`) | same, `client_rects` (NEW) — **mutually-exclusive dispatch**, `InlineClientRects` precedence, NOT a union | ✓ | no |
| CSSOM VIEW §6 Extensions to the Element Interface | `getClientRects()` | (c) no layout box → empty DOMRectList | same → empty | ✓ | no |
| CSSOM VIEW §6 "get the bounding box" | `getBoundingClientRect()` | (a) empty rect-list → **all-zero** DOMRect (x=y=w=h=0) | `bounding_box` (NEW), empty-arm | ✓ | no |
| CSSOM VIEW §6 "get the bounding box" | `getBoundingClientRect()` | (b) all rects zero-w/h → **first rect** | `bounding_box` (NEW) | ✓ | no |
| CSSOM VIEW §6 "get the bounding box" | `getBoundingClientRect()` | (c) else → union over **non-zero** rects only | `bounding_box` (NEW) | ✓ | no |
| CSSOM VIEW §7 Extensions to the HTMLElement Interface | `offsetWidth`/`offsetHeight` | (a) no box → 0 | `offset_border_box_union` (NEW) None-arm | ✓ | no |
| CSSOM VIEW §7 Extensions to the HTMLElement Interface | `offsetWidth`/`offsetHeight` | (b) has box → **UNION (axis-aligned bbox) of the principal box's fragments** (step 2) | `offset_border_box_union` (NEW) | union ✓ (step-2 inline-split-by-block-descendant sub-source omitted — orthogonal to multicol) | no |
| CSSOM VIEW §7 Extensions to the HTMLElement Interface | `offsetTop`/`offsetLeft` | offsetParent-relative, **first** box | `offset_from_parent` (principal fragment) | ✓ | no |
| CSSOM VIEW §6.1 Element Scrolling Members | `clientWidth/Height` | **padding box** of principal fragment (inline→0 / root→viewport branches pre-existing, unchanged) | `principal_padding_box` (NEW) | routing-delta only | no |
| CSSOM VIEW §6.1 Element Scrolling Members | `clientTop/Left` | **border widths** of principal fragment | `principal_border_widths` (NEW) | routing-delta only | no |
| CSSOM VIEW §6.1 Element Scrolling Members | `scrollWidth/Height` | spec step 7 = **scrolling area** (padding box + descendant overflow) — elidex computes padding-box-only (pre-existing gap, `:159-170`); C-3 preserves it (scrollTop/Left offset = `ScrollState`, out of scope) | `principal_padding_box` (NEW) | pre-existing limitation, not §6.1-met | no |

**Not in the table** (reuse a row above, no new citation): IntersectionObserver target rect = the same
"get the bounding box" primitive (§3.2.7 step 1 — the §6 4-branch rows); hit-test / a11y bounds are
engine-internal (no CSSOM dfn — paint-consistency, not a spec algorithm). css-multicol-1 §8 / css-break-3
§5 are the *producer* basis (already the store's content), not a C-3 reader surface. Baseline
(`principal_baseline`) is engine-internal alignment (flex/grid), no CSSOM dfn.

**"Full enum?" honesty (F11)**: ✓ rows fully enumerate the cited algorithm's fragment-relevant branches;
`getClientRects` omits the pre-existing SVG-single-rect / table-box-substitution sub-branches (§6 steps
2–3) and `client*`/`scrollWidth/Height` are marked **"routing-delta only"** — the fragment routing
changes the box *source*, the pre-existing inline→0 / root→viewport branches are unchanged and NOT
re-enumerated here.

**Breadth**: K=1 spec (cssom-view), M=12 rows (verified 2026-07-13 via `.claude/tools/webref heading
cssom-view 6|7|6.1`, `dfn cssom-view getClientRects|getBoundingClientRect|offsetWidth`, `body cssom-view
dom-htmlelement-offsetwidth|dom-element-getboundingclientrect|dom-element-clientwidth` for step prose) →
single-PR by spec breadth; the **cross-crate reader spread (§7) is the split driver**, not spec breadth.

> Preflight note (soft-warn, non-blocking): the coverage-map helper emits the spec label `CSSOM VIEW`,
> which `preflight.py`'s `SPEC_LABEL_REVERSE` does not yet map (so its auto-verify parses 0 rows) — a
> tooling seam, not a citation error; all 8 rows were webref-verified manually as above.

### §9.1 User-input touch audit

No C-3 reader is a user-controllable-input sink: geometry values are layout-derived — the script
*triggers* the read (`getBoundingClientRect()`/`offsetWidth`) but supplies no data flowing into the
computation. The migration changes *which box(es)* feed the existing document→viewport conversion, not
any parse/coerce of script input. Adjacent pre-existing surface (`accumulated_scroll_offset`,
`offset_from_parent`, `layout_query.rs:30,82`): unchanged, exposure delta none.

---

## §10 Gate plan

**Umbrella review status**: 5-axis `/elidex-plan-review` on THIS memo **complete 2026-07-13** — 0 CRIT,
6 IMP + 7 MIN, all applied above (Axis 1 layering clean; Axis 2 2a ECS-native shape clean; Axis 5 §0
premise-correction verified). The IMPs were projection-helper completeness + spec-accuracy (offsetWidth
union, bounding 4-branch, client* facets, getClientRects two-source, coord invariant, baseline slice) —
not architecture. → PM schedules C-3a first (elidex-ecs seam, most isolatable) → **per-slice
`/elidex-plan-review`** (C-3b spec-dense especially) → impl → `/pre-push` → `/external-converge`. LESSON
carried from #316/C-1: a fragment-consuming path has dense per-fragment edges
(union/rect/first/router-signal/coord/line-vs-column) — expect a multi-round Codex converge per slice,
esp. C-3b.

---

## §11 Report to PM (coordination)

1. **✅ RESOLVED (PM, 2026-07-13)** — the shared memo UPDATE was replaced with a v2 fabrication-retraction
   + the campaign memo Lane 2 aligned. The "store relocated to elidex-render / dependency wall / layout
   cannot read it" finding was a flaky-I/O fabrication. LIVE: `FragmentTree` in `elidex-ecs` (`EcsDom`
   sibling field), universally reachable, no wall, no relocation (`elidex-layout` depends on
   `elidex-ecs` via `elidex-ecs.workspace = true`). The (a)/(b) store-placement options dissolve; the
   decision is the projection shape (§1, ≈ option c). The v2 also retracts the "§1/§3/§5 cite STALE"
   claim — those cites are correct and non-stale.
2. **C-3 is cross-crate as code** (dom-api/api-observers/layout/a11y/shell/render) — not layout-only,
   not parallel-safe with CSS/script/shell lanes. Schedule as coordinated sub-slices (§7); C-3a
   (elidex-ecs, additive) is the isolatable seed.
3. **✅ RESOLVED (PM, 2026-07-13)** — the UPDATE's other fabricated numbers were corrected in the SoT:
   `#425`/`#430` were misattributions (#425 = Dependabot, #430 = s5-3a keepalive); git-confirmed
   **C-1 = #321 `48b0190b`**, **C-2 = #324 `b4e06897`**, and `#413 = bcee298b` (MutationObserver
   transient observers, unrelated to any relocation). Crate name corrected `css-tables` → the tracked
   `crates/css/elidex-css-table/`.
4. This plan-memo is doc-only/parallel-safe; the impl sub-slices are the coordination surface.
