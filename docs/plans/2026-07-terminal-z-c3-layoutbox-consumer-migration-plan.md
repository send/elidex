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

**⚠ Layering split (R2-6 + R3-2, by *dependency-reachability* — SSoT `docs/design/en/12-dom-cssom.md:4,104`
+ `docs/architecture/core.md:16-22`).** Two constraints pull in opposite directions and the split must
satisfy BOTH: (i) CSSOM-View *algorithms* must NOT live in the storage crate (R2-6); (ii) the reductions
that **cross-crate consumers** need must be reachable by them — and **`elidex-layout-flex`/`-grid`,
`elidex-a11y`, `elidex-api-observers` do NOT depend on `elidex-dom-api`** (Cargo.toml-verified: all
`dom-api=0`, all `ecs=1 plugin=1`), so a dom-api-only home is unreachable for them (R3-2). Resolve by
splitting reductions by *kind*, not by lumping them all in dom-api:

- **Generic geometry (content-neutral) → low, in `elidex-ecs`/`elidex-plugin`** (every consumer already
  depends on both): `box_fragments` (the fold); `Rect`-sequence **union → Option** (boxless→None); first;
  **box-size** `(content_rect, border_box_size)`; **baseline** (`first_baseline` + `content().origin.y`).
  These are NOT CSSOM algorithms — just Rect/size folds — so they satisfy R2-6 while being reachable by
  flex/grid/a11y/observers/shell.
- **CSSOM-View-specific algorithms → `elidex-dom-api`** (only the CSSOM handlers consume them):
  `getBoundingClientRect`'s 4-branch (all-zero / first-rect / non-zero-filter), `offsetWidth` union-vs-
  `offsetTop` first, the `getClientRects` two-source dispatch, the scroll-area policy. `elidex-ecs` stays
  a pure store; the CSSOM branch semantics stay in the CSSOM layer.

The observer/a11y **boxless Option/None** need is met by the low `union→Option` helper (a Rect fold, not
a CSSOM policy); the CSSOM zero-rect branch is the dom-api layer's `getBoundingClientRect` wrapper. (This
corrects R1's "all reductions in `dom/geometry.rs`" AND R2's "all in dom-api" — the reachability split is
the both-constraints-satisfying home.)

**The projection primitive (on `EcsDom`, `elidex-ecs`):**

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

**⚠ The per-consumer reductions are NOT restated here — they live ONCE, in the canonical §3 table.**
(CLAUDE.md *one-issue-one-way*: "その種の処理は単一の正準形に一括収束させる… 混沌度 (= 決定の表面積) を
下げること自体が目的". Earlier drafts of this memo duplicated each consumer's fragment semantic across
§1/§3/§7/§9 — 6-23 sites per consumer. That duplicated decision surface is what forced N-site propagation
on every review fix (the R4/R5 sibling churn) and let errors drift in between sites (R5-1's RO origin,
C-3e's paged-gen misclassification). **§3 is now the single decision site**; §1/§7/§9 reference it.)

**Placement of the reductions** follows the layering split above (R2-6 + R3-2): *generic-geometry* folds
(union→Option / first / box-size / baseline — content-neutral) live LOW in `elidex-ecs`/`elidex-plugin`
(reachable by flex/grid/a11y/observers, which don't depend on dom-api); the *CSSOM-View-specific*
algorithms live in `elidex-dom-api`. Both call `EcsDom::box_fragments` and apply their reduction. Because
`box_fragments` yields the **full `BoxFragment`** (impl's `BoxModel` → `.border_box()` / `.padding_box()` /
`.border()` / `.first_baseline` / `.content()`), every facet is a fold over the one primitive — no
primitive change. (Single source-exception: `getClientRects` also draws on the `InlineClientRects`
component for line boxes the store does not hold — §3.)

**Boxless contract (I-boxless — cross-cutting invariant, PINNED)**: a boxless entity (display:none /
pre-layout: **no fragments AND no `LayoutBox`**) splits consumers into two classes that must NOT be
collapsed onto one helper:
- **spec-zero** — CSSOM mandates a concrete empty/zero result (an all-zero `DOMRect` / an empty rect list).
- **Option-None** — the consumer branches on *"is there a box at all?"*, and a zero-rect is **not** the
  same as no-box: IO treats `None` as the *required* initial false/ratio-0 observation
  (`intersection/mod.rs:298-345`, pinned `tests_core.rs:295-317`); a11y skips `set_bounds` entirely when
  there is no box (`tree.rs:121-126`). Feeding an Option-None consumer a zero-rect regresses it (a boxless
  target at the origin reads as an intersecting zero-area box; a boxless node gains a spurious `(0,0,0,0)`
  AX bound).

Which class each consumer is in — and every other per-consumer semantic — is **§3**.

**Home** (F8 + R2-6 + R3-2): the **`box_fragments` primitive + the generic-geometry folds** (union→Option
/ first / box-size / baseline — all content-neutral, reachable by every consumer) land in a **new
`crates/core/elidex-ecs/src/dom/geometry.rs` (NEW)** — NOT appended to `dom/mod.rs` (1073 LoC, CLAUDE.md
touch-time-split; `task_2924ead0`). The **CSSOM-View-specific algorithms** (getBoundingClientRect
4-branch / offsetWidth union-vs-offsetTop first / getClientRects dispatch / scroll-area) live in
**`elidex-dom-api`** alongside the geometry handlers (`element/layout_query.rs`). This satisfies both
R2-6 (CSSOM semantics out of the store) and R3-2 (generic folds reachable by flex/grid/a11y/observers,
which don't depend on dom-api).

---

## §2 Coupled-invariant matrix + N=1 fast-path

**Edge-dense coupled-invariant enumeration** (Pre-condition #3; R3-5 + plan-review Axis 3 — the crossings
are the edge density, consolidated here so per-slice implementers see them in one place rather than
scattered across §1/§3/§4/§5/§9):

| # | Invariant | Detailed in | Key intersection with others |
|---|---|---|---|
| I1 | N=1-vs-N fold (fragments-else-`LayoutBox`) | §1, §2 | × I7 coord (N=1 arm's `From` must be same doc-space) |
| I2 | box-model facet per API (border/padding/border-width/baseline/size) | §1, §3 | × I8 layering (generic facets low; CSSOM branch dom-api) |
| I3 | union-vs-first-vs-per-fragment reduction | §3, §9 | × I4 (offset*W/H union but offset*T/L first) |
| I4 | boxless Option/None vs spec-zero | §1 (I-boxless), §3 | × I2 (RO wants size-None, IO wants union-None, a11y skip) |
| I5 | router = `fragments_for`-presence, NOT `is_consumable` | §4 | × I6 (line-vs-column dispatch is a *different* router) |
| I6 | `getClientRects` two-source dispatch (`InlineClientRects` precedence, NOT union) | §1, §9 | × I1 (N=1 box_fragments would double-count inline) |
| I7 | coordinate space (`BoxFragment` abs == `LayoutBox` doc-space) | §5 (I-coord) | × I3 (all reductions feed the scroll-subtraction) |
| I8 | layering (generic geometry → ecs/plugin; CSSOM algo → dom-api) | §1 | × I2, × consumer dep-graph (flex/grid/a11y non-dom-api) |

(The full per-API routing is §3; per-branch enum §9; this matrix is the *crossings* view.)

**N=1 fast-path** (§5-Q3 of the anchor): the overwhelmingly common entity has **no** store fragments
(only `consumable`/mid-break boxes are pushed; `push_box` is called only by the multicol committer). The
fast-path must not allocate or change behavior for it.

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

## §3 CANONICAL consumer → fragment-semantic table (the single decision site)

**This table is the SSoT for "which fragment(s) does each consumer use, and how".** §1 (primitive +
layering), §7 (slices), §9 (spec branches) **reference** it and do not restate it — one decision, one home
(CLAUDE.md one-issue-one-way; the earlier duplication across 4 sections is what produced the R4/R5
propagation churn and the R5-1 / C-3e errors).

**Status** — `PINNED` = umbrella-owned decision, verified against live code/spec. `OPEN → C-3x` = a
per-consumer semantic this umbrella deliberately does **not** guess: the owning slice's `/elidex-plan-review`
pins it (the *known edge* is recorded so that review starts from it, not from zero). `OUT` = not a
box-geometry consumer / out of C-3 scope. **Not every consumer becomes N-aware** — the facet and the
union-vs-first-vs-per-fragment behavior differ per row and must not be collapsed.

| Consumer | Reduction (home) | Fragment semantic | Status | Spec | Slice |
|---|---|---|---|---|---|
| `getClientRects()` | `client_rects` (dom-api) | two-source **dispatch, not union**: `InlineClientRects`→per-**line** (suppresses the box projection, else the N=1 whole box double-counts); else store→per-**column**; else single box | **OPEN → C-3b** (R2-3/R6-1): for an inline split across **both** lines **and** columns, today's `InlineClientRects` is per-column/G11 state and true per-fragment inline rects are committed-next (`elidex-layout-block/src/inline/mod.rs:933-…`) — plain suppression may drop columns. C-3b pins the dispatch. | cssom-view §6 | C-3b |
| `getBoundingClientRect()` | `bounding_box` (dom-api) | **4-branch** over the rect list: empty→all-zero; all-zero-w/h→**first rect**; else union over **non-zero** rects only | PINNED | cssom-view §6 | C-3b |
| `offsetWidth`/`offsetHeight` | `offset_border_box_union` (dom-api) | **UNION** (axis-aligned bbox) of the principal box's fragment border boxes | PINNED | cssom-view §7 step 2 | C-3b |
| `offsetTop`/`offsetLeft` | principal box via `offset_from_parent` (dom-api) | **first** box (asymmetry vs W/H union) | PINNED | cssom-view §7 | C-3b |
| `clientWidth`/`clientHeight` | `principal_padding_box` (ecs) | first fragment **padding box** | PINNED | cssom-view §6 `#dom-element-clientwidth` | C-3b |
| `clientTop`/`clientLeft` | `principal_border_widths` (ecs) | first fragment **border widths** | PINNED | cssom-view §6 `#dom-element-clienttop` | C-3b |
| `scrollWidth`/`scrollHeight` | `principal_padding_box` (ecs) | first fragment padding box — **preserves today's pre-existing limitation** (spec = scrolling area = padding box + descendant overflow; `BoxFragment` has no overflow facet) | PINNED (limitation, no regression) | cssom-view §6 `#dom-element-scrollwidth` (**not** met, pre-existing) | C-3b |
| `scrollTop`/`scrollLeft` | `ScrollState` | unchanged (scroll *offset*, not a box read) | **OUT** | — | — |
| `ScrollIntoView` | (same `get_border_box` choke, dom-api) | reads target **and** scroll-container border boxes (`layout_query.rs:276,293`) | **OPEN → C-3b** (R6-2): for a multicol target, *which* fragment does it scroll to (first / nearest / the one in view)? C-3b pins. | cssom-view | C-3b |
| `IntersectionObserver` | `optional_bounding_box` (ecs) | 4-branch when boxed; **`None` when boxless** (the required initial false/ratio-0 entry) | PINNED (I-boxless) | intersection-observer §3.2.7 step 1 | C-3d |
| `ResizeObserver` | box-**size** projection (ecs), None-preserving — **not** a bbox | principal (first) fragment's content size + border-box size | **OPEN → C-3d** (R5-1/R6-3): `contentRect`'s **origin** is NOT the fragment's absolute `content()` and NOT `(0,0)` — the live `content_rect_local()` returns **padding offsets** `(padding.left, padding.top)` per RO §3.3.1. C-3d pins the exact origin (this umbrella will not guess it again). | resize-observer-1 §3.3.1 / §3.4 | C-3d |
| a11y bounds | `optional_bounding_box` (ecs) | bbox when boxed; **skip `set_bounds` when `None`** (no spurious 0-rect) | PINNED (I-boxless) | — | C-3c |
| hit-test | per-fragment over **one** transform reference box (`hit_test.rs:130-172`); result carries the **hit fragment** (not just the entity) | hit any fragment; downstream needs which | PINNED | — | C-3c |
| flex/grid baseline | `principal_baseline` + padding/border facets + `content().origin.y` (ecs) | principal fragment (the readers co-read origin.y, `lib.rs:477-478` / `position.rs:447-448`) | PINNED | — | C-3c |
| shell scroll extent | all-fragment/all-entity **max** over **index-filtered** `fragments_for` (NOT `nodes()` — orphan-node hazard, `fragment_tree.rs:177-189`) | every visible box incl. later columns | PINNED | — | C-3d |
| shell caret-scroll | principal fragment content width (`event_handlers.rs:400-405`) | principal | PINNED | — | C-3d |
| shell iframe click-routing | consumes C-3c's **hit fragment** (`event_handlers.rs:834-846`) | the hit column's origin | PINNED | — | C-3d |
| shell lazy-iframe visibility | — | **OPEN → C-3d** (R6-5): a *separate* scan over iframe entities (`content/iframe/lifecycle.rs:263-274`), **not** hit-test-driven — needs its own fragment policy (any-fragment-visible? principal?). C-3d pins. | **OPEN** | — | C-3d |
| shell URL-fragment nav | — | **OPEN → C-3d** (R4-2): `scroll_offset_for_fragment` (`content/scroll.rs:236`) — which column fragment does a multicol target scroll to? C-3d pins. | **OPEN** | — | C-3d |
| render paint **content** | `is_consumable` gate — **unchanged by C-3** | consumable→per-fragment content (C-1/C-2); **non-consumable → single box** (no per-column carrier exists) | PINNED (C-3 does not touch it) | css-break-3 §5.4 / css-multicol-1 §8.1 | — |
| render **geometry** readers (block-child class `walk.rs:708`, list-marker `:774`, root discovery `mod.rs:482,998`, inline-text anchor `paint/mod.rs:789`) | `box_fragments` geometry | per-fragment box geometry | PINNED | — | C-3e |
| render **paged-generation gate** (`walk.rs:108`) | — | reads `LayoutBox.layout_generation`; **`BoxFragment` DROPS that field** (the node's `fragmentainer` replaces it, `fragment_tree.rs`) → **NOT a box-geometry reader**, `box_fragments` cannot serve it | **OUT** of the box-geometry migration (R6-4 correction) | — | C-3e (separate treatment) |

**Cross-cutting, all rect rows**: element+ancestor **transforms** (cssom-view §6 getClientRects step 3,
which `getBoundingClientRect` is defined from) are applied **nowhere** in elidex today (`layout_query.rs`,
verified) — a **pre-existing gap**. Every rect reduction above operates on raw un-transformed fragment
rects; C-3 preserves that (no regression) and transform fidelity is **out of C-3 scope**.

**Net-new spec-correctness wins** (today the single last-column `LayoutBox` is returned = WRONG for
multicol): `getClientRects` per-column rects, `getBoundingClientRect`/IO 4-branch bbox, **`offsetWidth/
Height` union**, per-fragment hit-test. **Near-noop** (facet-preserving; first==single for N=1):
`offsetTop/Left`, `client*`, `scrollWidth/Height`, caret-scroll — routing-delta only, whose job is to stop
naming `LayoutBox` so C-4 can delete it.

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

> ⚠ **Decision-altitude, NOT a per-reader inventory** (re-frame): the `git grep -l 'LayoutBox'` counts are
> a LOWER BOUND — they miss the **qualified** path `elidex_plugin::LayoutBox` and bounding-rect closures.
> Three review rounds (R1/R2/R3) each surfaced *more* individual readers (ResizeObserver, flex `baseline.rs`,
> shell caret/iframe/fragment-nav, `ScrollIntoView`, `find_nearest_layout_box`, render walk sites …) — which
> is the **proof that hand-enumerating every reader in this umbrella does not converge**. So this memo owns
> the **clusters + their choke points** (below), and the **exhaustive per-reader inventory is C-3a's audit
> OUTPUT** (grep must cover ALL read shapes, not just direct gets: `get::<&(elidex_plugin::)?LayoutBox>`,
> **multi-component `query::<(..&LayoutBox..)>`** (e.g. `scroll.rs:137` reads `(&LayoutBox, &ComputedStyle)`, R4-1),
> and closure `rect_fn` sites, across ALL crates), consumed by each
> per-slice plan-review — NOT a list to complete here. The examples below are illustrative of each cluster,
> not exhaustive.

**Reader clusters + choke points (C-3 scope)** — this is the *WHERE* (which crates/files read `LayoutBox`,
and the choke each cluster funnels through). The *WHAT* (each consumer's fragment semantic) is **§3** and
is not restated here.

- **CSSOM geometry** — `crates/dom/elidex-dom-api/src/element/layout_query.rs`: **one choke =
  `get_border_box` (`:336`)**, through which `getBoundingClientRect` (`:26`), `offset*` (`:68`), `client*`,
  `scroll*`, `ScrollIntoView` (`:276,:293`), `offsetParent` (`:385`) ALL funnel — migrating the choke (+
  `get_padding_box`/border-width for `client*`, `offset_from_parent` `:81-89` for offset*) covers every
  handler that calls it, which is why enumerating each handler is impl-detail. `getClientRects` already
  exists (`:201-240`, two-source) — a **fix**, not an add.
- **Observer geometry (script-host + api)** — ResizeObserver (`elidex-js/src/vm/host/resize_observer.rs:405`,
  registry `elidex-api-observers/src/resize.rs:231-272`); IntersectionObserver host closure
  (`.../intersection_observer.rs:488-490`) + registry (`intersection/mod.rs:298-345`, boxless behavior
  pinned by `tests_core.rs:295-317`).
- **hit-test** — `elidex-layout/src/hit_test.rs` (transform basis `:130-172`; result type `:15-19`).
- **a11y** — `elidex-a11y/src/tree.rs:121-126` (`set_bounds` guarded on box presence).
- **shell** — five readers: scroll extent (`content/scroll.rs:133-148`, `compute_content_extent`),
  caret-scroll (`content/event_handlers.rs:400-405`), iframe click-coord xlate (`:834-846`), lazy-iframe
  visibility (`content/iframe/lifecycle.rs:263-274`), URL-fragment nav (`content/scroll.rs:236`).
- **flex/grid baseline cross-read** — `elidex-layout-flex/src/lib.rs:473-479` + `/src/baseline.rs:18-26`;
  `elidex-layout-grid/src/position.rs:444` (reads despite living in producer crates).
- **render** — paged-generation gate (`builder/walk.rs:108` — **not** a box-geometry read, §3),
  block-child classification (`:708`), list-marker positioning (`:774`), root discovery
  (`builder/mod.rs:482,998`), inline-text anchor `find_nearest_layout_box` (`builder/paint/mod.rs:789`,
  called from `builder/inline.rs:151`).

> The producer sites (layout-* *writers*) are a **C-4** concern (every producer must write the store's
> N=1 box for every entity before `LayoutBox` can be deleted, §5-Q3 of the anchor) — **out of C-3
> scope**. C-3 only moves *readers* (incl. the flex/grid baseline reads above, which are reads despite
> living in producer crates). **Reader-only invariant (F7)**: no C-3 sub-slice touches a producer
> write — carried into each C-3a…e plan-review.

---

## §7 Consumer-cluster sub-slicing (each a shippable, coordinated PR)

The migration is large and cross-crate; slice by consumer cluster, seam-first, each
behavior-neutral-or-spec-fix, in dependency order. **These slices assign clusters + call out the
non-obvious readers/edges — they are NOT exhaustive per-reader checklists** (R4-2): the C-3a audit
(§6) produces the definitive reader inventory per cluster, and each slice migrates *all* of its cluster's
audited readers, not only the ones named here.

**Each slice OWNS a set of §3 rows** (the semantics live there; this section owns *slice-level* concerns —
isolation, ordering, cross-slice API deps, and which OPEN rows the slice must pin at its plan-review).

- **C-3a — the projection seam** (`elidex-ecs`/`elidex-plugin`, new `dom/geometry.rs` (NEW)).
  Ships: `EcsDom::box_fragments` + the **generic-geometry folds** (§1 layering split) — **not** the
  CSSOM-View-specific algorithms (dom-api's). **Plus the complete read-site audit** (§6:
  `get::<&(elidex_plugin::)?LayoutBox>` + multi-component `query::<(..&LayoutBox..)>` (R4-1) + `rect_fn`
  closures, across all crates) — the audit, not this umbrella, owns the exhaustive reader inventory.
  Connected-not-dead via **unit tests** on the folds (N=1 fast-path / §2 behavior-neutral invariant /
  multi-fragment order / boxless→None) — not by migrating a consumer (offset* lives in dom-api, so it
  would break `elidex-ecs`-only isolation). The **derisking slice**; lowest blast radius.
- **C-3b — CSSOM geometry** (`elidex-dom-api`). Owns §3 rows: `getClientRects`, `getBoundingClientRect`,
  `offset*`, `client*`, `scrollWidth/Height`, `ScrollIntoView`. All funnel through the one
  `get_border_box` choke (§6). **Must PIN its two OPEN rows** at its plan-review: the multicol-split-inline
  `getClientRects` dispatch, and `ScrollIntoView`'s target fragment. The spec-heaviest slice → own
  `/elidex-plan-review` + Codex converge (dense cssom-view / 4-branch / two-source edges); its coupled
  invariants are §2's I2/I3/I4/I6/I7.
- **C-3c — hit-test + a11y + baseline** (`elidex-layout` + `elidex-a11y` + `elidex-layout-flex`/`-grid`).
  Owns §3 rows: hit-test, a11y bounds, flex/grid baseline. **Cross-slice API dep**: hit-test's result type
  gains the **hit fragment** — C-3d's iframe click-routing consumes it, so **order C-3c before C-3d**.
- **C-3d — observers + shell** (`elidex-js` host + `elidex-api-observers` + `elidex-shell`). Owns §3 rows:
  IntersectionObserver, ResizeObserver, shell scroll extent / caret-scroll / iframe click-routing /
  lazy-iframe visibility / URL-fragment nav. **Must PIN its three OPEN rows**: RO's `contentRect` exact
  origin, lazy-iframe's fragment policy, URL-fragment-nav's target fragment.
- **C-3e — render residual** (`elidex-render`). Owns §3 row: render **geometry** readers. ⚠ It does **not**
  touch the render paint-**content** path — `is_consumable` correctly gates the content carrier and
  non-consumable stays single-box (§3, R5-3) — and the **paged-generation gate is OUT** of this migration
  (it reads `layout_generation`, which `BoxFragment` drops; §3, R6-4): it needs separate treatment, not
  `box_fragments`.
- **→ C-4** (separate program): retire `LayoutBox` + legacy inline pipeline + `InlineClientRects`, once the
  C-3a audit's inventory shows zero `LayoutBox` reads outside producers, and producers write the store's
  N=1 box for every entity.

Ordering rationale: C-3a is the seam all others consume; C-3b is highest-value (spec fixes) and
proves the projection against the richest consumer; C-3c/d are mechanical once the seam + helpers
exist; C-3e closes render. Each slice is independently `/elidex-plan-review`'d per the anchor's §6 and
the edge-dense discipline.

**Coordination**: C-3b touches `elidex-dom-api` (contends with DOM/CSSOM lanes), C-3c the a11y +
layout lanes, C-3d the api/shell lanes. These are **not parallel-safe as code** and must be
PM-scheduled against the active lanes; C-3a (elidex-ecs, additive) is the most isolatable.

---

## §8 Open questions (each pinned by the owning sub-slice's plan-review)

**A. The `OPEN` rows of §3** — per-consumer semantics this umbrella deliberately does **not** guess. Each
is recorded with its *known edge* so the owning slice's `/elidex-plan-review` starts from it. (Two of the
umbrella's earlier guesses at exactly this kind of detail were wrong — R5-1's RO origin, C-3e's paged-gen
classification — which is why these stay OPEN rather than being re-guessed.)

| OPEN row (§3) | Question the slice must pin | Slice |
|---|---|---|
| `getClientRects` | multicol-split-inline dispatch: does `InlineClientRects` precedence drop column fragments? | C-3b |
| `ScrollIntoView` | which fragment does a multicol target scroll to (first / nearest / in-view)? | C-3b |
| `ResizeObserver` | `contentRect`'s exact origin (live `content_rect_local()` = padding offsets, RO §3.3.1) | C-3d |
| shell lazy-iframe visibility | which fragment determines visibility (any-visible / principal)? | C-3d |
| shell URL-fragment nav | which column fragment does a multicol target scroll to? | C-3d |

**B. Engine-mechanics questions (not per-consumer semantics):**
1. **`box_fragments` receiver**: `&EcsDom` vs `&mut EcsDom` (handlers hold `&mut` today). The read-only
   projection wants `&EcsDom`; confirm no borrow conflict with the handler signature. → C-3a.
2. **`accumulated_scroll_offset` per fragment**: for N>1, is the scroll offset identical across fragments
   (same scroll-ancestor chain)? Almost certainly yes (one entity, one chain) — verify, else the union
   must subtract per-fragment. → C-3b.
3. **hit-test z-order across fragments**: per-fragment hit must preserve paint order; confirm the
   `fragmentainer` iteration order matches paint order for hit resolution. → C-3c.

---

## §9 Spec coverage map (§3-discipline table — citations webref-verified 2026-07-13)

**Scope of this table**: it enumerates the **spec branches** each cited algorithm has (the
`/elidex-plan-review` §3-discipline gate) and where they dispatch. It does **not** restate the per-consumer
*fragment semantic* — that is the canonical §3 table (one decision, one home). Branch enumeration for the CSSOM readers C-3 migrates — the load-bearing correctness surface: the
**empty / no-box branches** the projection's `Option`/`None` arms must cover, and the **union-vs-first**
split. (Anchors: §6 `#extension-to-the-element-interface` — incl. `client*`/`scroll*` attributes
`#dom-element-clientwidth`/`#dom-element-scrollwidth`; §7 `#extensions-to-the-htmlelement-interface` —
`offset*`. §6.1 "Element Scrolling Members" is the scroll *methods*, NOT the client/scroll attributes.)

| Spec section | Step | Branch | Touch (dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSSOM VIEW §6 Extensions to the Element Interface | `getClientRects()` | (a) inline multi-line → per-**line** rects (`InlineClientRects`) | `layout_query` getClientRects (`:201-240`, two-source) → `client_rects` (NEW) | branches ✓ — §6 step-3 element+ancestor **transform** application is a **pre-existing gap** (current impl applies none, `layout_query.rs`), out of C-3 scope (R3-6) | no |
| CSSOM VIEW §6 Extensions to the Element Interface | `getClientRects()` | (b) multicol (no `InlineClientRects`) → per-**column** border boxes (`box_fragments`) | same, `client_rects` (NEW) — **mutually-exclusive dispatch**, `InlineClientRects` precedence, NOT a union | ✓ | no |
| CSSOM VIEW §6 Extensions to the Element Interface | `getClientRects()` | (c) no layout box → empty DOMRectList | same → empty | ✓ | no |
| CSSOM VIEW §6 "get the bounding box" | `getBoundingClientRect()` | (a) empty rect-list → **all-zero** DOMRect (x=y=w=h=0) | `bounding_box` (NEW), empty-arm | ✓ | no |
| CSSOM VIEW §6 "get the bounding box" | `getBoundingClientRect()` | (b) all rects zero-w/h → **first rect** | `bounding_box` (NEW) | ✓ | no |
| CSSOM VIEW §6 "get the bounding box" | `getBoundingClientRect()` | (c) else → union over **non-zero** rects only | `bounding_box` (NEW) | branches ✓ — element+ancestor **transforms** (§6 via getClientRects step 3) a **pre-existing gap** (impl applies none), out of C-3 scope (R5-2) | no |
| CSSOM VIEW §7 Extensions to the HTMLElement Interface | `offsetWidth`/`offsetHeight` | (a) no box → 0 | `offset_border_box_union` (NEW) None-arm | ✓ | no |
| CSSOM VIEW §7 Extensions to the HTMLElement Interface | `offsetWidth`/`offsetHeight` | (b) has box → **UNION (axis-aligned bbox) of the principal box's fragments** (step 2) | `offset_border_box_union` (NEW) | union ✓ (step-2 inline-split-by-block-descendant sub-source omitted — orthogonal to multicol) | no |
| CSSOM VIEW §7 Extensions to the HTMLElement Interface | `offsetTop`/`offsetLeft` | offsetParent-relative, **first** box | `offset_from_parent` (principal fragment) | ✓ | no |
| CSSOM VIEW §6 Extensions to the Element Interface | `clientWidth/Height` | **padding box** of principal fragment (inline→0 / root→viewport branches pre-existing, unchanged) | `principal_padding_box` (NEW) | routing-delta only | no |
| CSSOM VIEW §6 Extensions to the Element Interface | `clientTop/Left` | **border widths** of principal fragment | `principal_border_widths` (NEW) | routing-delta only | no |
| CSSOM VIEW §6 Extensions to the Element Interface | `scrollWidth/Height` | scrollWidth = **scrolling area** (padding box + descendant overflow) — elidex computes padding-box-only (pre-existing gap, `:159-170`); C-3 preserves it (scrollTop/Left offset = `ScrollState`, out of scope) | `principal_padding_box` (NEW) | pre-existing limitation, not §6-met | no |

**Not in the table** (reuse a row above, no new citation): IntersectionObserver target rect = the same
"get the bounding box" primitive (§3.2.7 step 1 — the §6 4-branch rows); hit-test / a11y bounds are
engine-internal (no CSSOM dfn — paint-consistency, not a spec algorithm). css-multicol-1 §8 / css-break-3
§5 are the *producer* basis (already the store's content), not a C-3 reader surface. Baseline
(`principal_baseline`) is engine-internal alignment (flex/grid), no CSSOM dfn.

**"Full enum?" honesty (F11)**: ✓ rows fully enumerate the cited algorithm's fragment-relevant branches;
`getClientRects` omits the pre-existing SVG-single-rect / table-box-substitution sub-branches (§6 steps
2–3) and `client*`/`scrollWidth/Height` are marked **"routing-delta only"** — the fragment routing
changes the box *source*, the pre-existing inline→0 / root→viewport branches are unchanged and NOT
re-enumerated here. **Cross-cutting (all `getClientRects`/`getBoundingClientRect` rows, R5-2)**: element+
ancestor **transform** application (§6 getClientRects step 3) is a **pre-existing gap** (impl applies none)
— every ✓ here is branch-enumeration over *un-transformed* fragment rects, and transform fidelity is out
of C-3 scope (§1 `bounding_box` note).

**Breadth**: K=1 spec (cssom-view), M=12 rows (verified 2026-07-13 via `.claude/tools/webref heading
cssom-view 6|7|6.1`, `dfn cssom-view getClientRects|getBoundingClientRect|offsetWidth`, `body cssom-view
dom-htmlelement-offsetwidth|dom-element-getboundingclientrect|dom-element-clientwidth` for step prose) →
single-PR by spec breadth; the **cross-crate reader spread (§7) is the split driver**, not spec breadth.

> Preflight note (soft-warn, non-blocking): the coverage-map helper emits the spec label `CSSOM VIEW`,
> which `preflight.py`'s `SPEC_LABEL_REVERSE` does not yet map (so its auto-verify parses 0 rows) — a
> tooling seam, not a citation error; all 8 rows were webref-verified manually as above.

### §9.1 User-input touch audit

The **CSSOM geometry** readers are not user-input sinks: values are layout-derived — the script
*triggers* the read (`getBoundingClientRect()`/`offsetWidth`) but supplies no data flowing into the
computation. **BUT two C-3 readers ARE user-input flows (R2-5, correcting an earlier over-broad "none")**:
- **hit-test** consumes the viewport coordinates of the input event (`hit_test.rs:46-75` takes the event
  `point`) — the migrated per-fragment test must handle attacker-influenced coordinates safely (no OOB
  fragment index, no panic on NaN/extreme coords).
- **iframe click routing** subtracts the iframe box from `MouseClickEvent` points
  (`event_handlers.rs:834-846`) — same event-coordinate flow, now through the C-3c hit fragment.

Both are pre-existing input flows the migration *re-routes* (not new sinks), but the audit must label
them user-input, not exclude them. Adjacent surface (`accumulated_scroll_offset`, `offset_from_parent`,
`layout_query.rs:30,82`): unchanged, exposure delta none.

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
