# Terminal-Z C-3a — the `box_fragments` seam + the reader audit (plan-memo)

**Status**: design anchor for **C-3a**, the first slice of the terminal-Z C-3 program (migrate non-paint
`LayoutBox` readers onto the fragment store, so C-4 can retire `LayoutBox`). **Reviewed: full 5-axis
`/elidex-plan-review` + a cumulative fix-delta re-gate + Codex `/external-converge` (R1–R5); all findings
applied.**
Doc-only. Written off a first-hand read of the LIVE store + a tool-authoritative dependency/write-site
check, 2026-07-14.

**Why this memo is C-3a-scoped, not a C-3 umbrella.** The prior C-3 umbrella (#463, closed) tried to pin
**each consumer's per-fragment contract** before the reader audit that determines it. Across seven Codex
rounds + two structural collapses + a 5-axis re-gate, that layer never converged: every pass found another
row whose contract had been *asserted but not verified against the live reader*, and each collapse
re-introduced the same defect class (a structural premise stated as "verified" that was not). The umbrella's
own §4 had already said the quiet part: *"hand-enumerating every reader in this umbrella does not converge …
the exhaustive inventory is C-3a's audit output."* So this memo inverts the order: **C-3a ships the seam and
PRODUCES the audit; the per-consumer contracts are the audit's output**, owned by each downstream slice.
What this memo owns is verified against `cargo tree` / live code, or explicitly marked an audit question.

Predecessors MERGED: Z-1a (#313, standalone `FragmentTree`) / Z-1b (#316, per-column `InlineFlow`) / **C-1**
(#321 `48b0190b`, render consumes the store for `consumable` mid-break IFC entities) / **C-2** (#324
`b4e06897`, atomic-as-fragment).

---

## §0 Premise checks (this lineage has a fabrication history — every load-bearing fact is tool-verified)

The shared anchor memo once claimed the store was *relocated to `elidex-render` behind a dependency wall*
(a flaky-I/O fabrication), and the #463 umbrella twice asserted a "verified" structural premise that was
false. So every fact below is checked against an authoritative tool, not a prior memo:

- **The fragment store lives in `crates/core/elidex-ecs/src/fragment_tree.rs`** — the single tracked
  `fragment_tree.rs` (`git ls-files`). It is a **sibling field of `EcsDom`** (`crates/core/elidex-ecs/src/
  dom/mod.rs:50`) with `fragment_tree()` / `fragment_tree_mut()` accessors (`:148,:154`). No relocation, no
  dependency wall: `elidex-ecs` is a low, universally-depended-upon core crate (below only `elidex-plugin`,
  which it depends on) — **every** consumer already depends on it, and it is the only crate owning both the
  `World` and the `FragmentTree`.
- **`elidex-dom-api` CALLABILITY is a DIRECT-dependency fact, not transitive reachability** (Codex R1-T8).
  A Rust crate can `use` only its **direct** dependencies, so where a reduction may live is governed by the
  **direct** dep edge — `cargo tree --invert` proves *linkage*, `grep elidex-dom-api */Cargo.toml` proves
  *callability*. Direct `elidex-dom-api` dependents (`grep -rl 'elidex-dom-api' crates/*/*/Cargo.toml`):
  **`elidex-form`, `elidex-js` (via `engine`), `elidex-shell`, `elidex-wasm-runtime`** (R2-U1) (+ dom-api
  itself). **`elidex-a11y`, `elidex-layout(-block/-flex/-grid/-multicol/-table)`, `elidex-render`,
  `elidex-api-observers` do NOT declare it** — a11y / layout / render only *link* to dom-api transitively (via
  `elidex-form`; `elidex-layout`'s path is `layout-block → form`), which does **not** let them call it, and
  `elidex-api-observers` does not reach dom-api **at all** (deps = ecs/plugin/script-session; not in dom-api's
  reverse tree). ⇒ a dom-api-placed reduction is callable by
  `{dom-api, form, elidex-js, shell, wasm-runtime}` **only**. **Every box-geometry consumer** directly depends
  on `elidex-ecs` + `elidex-plugin` (all `ecs=1 plugin=1`), so a **LOW** reduction is callable by all of them —
  which is why C-3a places everything low (§1). ⚠ This corrects **two** errors of the same shape: the #463 table's `grep -c` direct-dep
  count *mislabeled transitive*, **and** this memo's own earlier §0 draft (+ #463's re-gate) over-swung the
  other way — *"a11y/layout/render reach dom-api transitively, so it's reachable"* — which conflates linkage
  with callability. For callability, a11y/layout/render **are** dom-api-unreachable (as §1's table states);
  the earlier "only api-observers is dom-api=0" line was wrong.
- **`ScrollState` IS written in production**: `crates/shell/elidex-shell/src/content/mod.rs:242-249`
  (`echo_viewport_scroll`) calls `insert_one(self.pipeline.document, self.viewport_scroll.clone())` on the
  **document root** on every scroll commit — a **type-inferred** insert (`grep ScrollState` misses it). So
  `accumulated_scroll_offset` (`layout_query.rs:399-416`) is **non-zero on any scrolled page**. (The #463
  memo's "I-frame gate is inert / always (0,0)" claim was false and self-contradictory — it also wrote that
  mis-wiring would "regress every scrolled page.")

---

## §1 The seam C-3a ships

**End-state (C-3 whole)**: geometry consumers read an entity's box geometry as **the sequence of its box
fragments**, never the raw `LayoutBox` component. The common non-fragmented entity is a **1-fragment**
sequence; a multicol mid-break entity is **N-fragment**. `LayoutBox` becomes a producer-internal detail C-4
deletes. **C-3a ships the projection + the frame-neutral folds + the reader audit** — nothing consumer-facing
beyond that.

**The projection primitive** — `EcsDom::box_fragments` (NEW), in a **new `crates/core/elidex-ecs/src/dom/
geometry.rs` (NEW)** (NOT appended to `dom/mod.rs`, which is **1073 LoC** — CLAUDE.md touch-time split;
`task_2924ead0`):

The **contract** C-3a's implementation must satisfy — **the memo pins the contract, NOT the Rust encoding**
(the exact signature is C-3a's implementation-plan-review decision; specifying a mechanism here is out of a
decision-record's altitude — R6 showed three glib mechanism-specs were each wrong):

```
impl EcsDom {
    /// (ILLUSTRATIVE signature — the CONTRACT is the four requirements below;
    ///  the exact encoding is C-3a impl.)
    fn box_fragments(&self, entity: Entity) -> <phase-guarded projection>;
}
```

1. **Fragment identity** — each yielded box carries its **`fragmentainer` id** (the stable
   `(entity, fragmentainer)` key the store keys on, `fragment_tree.rs:75,113,179`), so a retained hit fragment
   (C-3c) / iframe click-routing (C-3d) has the key §4 audit axis 7 requires without bypassing the seam. A span
   starting in a later column has fragmentainer ≠ enumeration index, so the id must be **yielded, not inferred**.
2. **Router = presence** (§2 I-router) — store-authoritative when `fragments_for` is non-empty (never
   LayoutBox-absence); else the single `LayoutBox` as one fragment `(fragmentainer 0, box)` via `From<&LayoutBox>`.
3. **Phase-invalidity is a DISTINCT signal from box-absence** (§2 I-phase; R6-X1) — a mid-layout or paged/print
   store must be **unreadable as screen geometry**, and that failure must **NOT** collapse to the same value as
   a boxless/despawned entity (whose downstream policy is a11y-skip / IO-RO-zero). So "not a completed screen
   pass" is a *guard failure* (a `Result::Err(InvalidPhase)` / a screen-geometry access token / a separate
   `try_*` accessor — impl's choice), never the boxless `None`/empty that means "no box." Liveness is part of
   *box-absence*: a despawned entity (whose stale store index teardown does NOT clean — only the multicol
   committer calls `remove_entity`) reads as absent, checked via `world.contains` before trusting `fragments_for`.
4. **Pre-transform geometry** (§2 I-transform) — CSS transforms are a paint-time wrapper; a painted-geometry
   reader composes the chain itself.

**Why on `EcsDom`** (not each consumer reading the store/component directly):

1. **`EcsDom` owns both stores** — the `World` (holding `elidex_plugin::LayoutBox`) and the `FragmentTree`
   (the N:M sibling field) are both its fields (`dom/mod.rs:50`); the fold "N fragments else the single
   `LayoutBox`" can only be expressed where both are in scope.
2. **`BoxModel` already unifies the two geometries** — both `LayoutBox` and `BoxFragment` impl `BoxModel`,
   and `impl From<&LayoutBox> for BoxFragment` is already the single field correspondence
   (`fragment_tree.rs:131,146`). The projection yields a uniform `BoxFragment` sequence with **zero new type
   machinery**.
3. **One-issue-one-way** — the dual-read ("has fragments? → tree; else → component") is a decision surface
   every consumer would otherwise re-implement, each a chance to use the wrong signal (§2 I-router).
4. **ECS-native** — the store is the canonical home for an **N:M** relation (one entity → N column fragments;
   and, at committed-next, *entity-less* line / anonymous-block fragments), which does not fit hecs's
   one-component-per-entity model. The projection is the read side; no side-store, no registry, no
   component-ization of the N:M relation.

**The frame-neutral folds C-3a ships** (all in `dom/geometry.rs`, all pure Rect/size math over the primitive).
⚠ **The `-> Option<…>` signatures below are ILLUSTRATIVE, not normative** (Codex R7-Y1): every fold is derived
from `box_fragments` and therefore **inherits its phase guard** — requirement 3 above (phase-invalidity is a
DISTINCT signal from box-absence) **propagates to the folds**, so a plain `Option` that collapses
*invalid-phase* into the boxless `None` would hand a migrated a11y/IO consumer the boxless branch on stale
page-relative geometry. The **encoding** (a `Result`, a token-gated accessor, folds defined only on an
already-guarded projection) is **C-3a's implementation-plan-review decision**, like the primitive's:

- `principal_fragment(entity)` → the first fragment (or the N=1 box); box-absent if boxless.
- `union_border_boxes(entity)` → the **plain** axis-aligned union of the fragment border boxes; box-absent if
  boxless. ⚠ **This is NOT the CSSOM-View "get the bounding box"** (Codex R1-T7): that algorithm
  returns the **first** rect when all rects are zero-sized, and otherwise unions **only** rects with non-zero
  width AND height (cssom-view §6 steps 3-4, webref-verified). A plain union that includes zero-sized fragments
  would move/expand `getBoundingClientRect` for a mixed zero/non-zero element. `union_border_boxes` is a
  **per-entity building block**, not a finished CSSOM reduction: **C-3b's `getBoundingClientRect` MUST build its
  own spec-shaped 4-step reduction, not reuse this fold**; and ⚠ **`offsetWidth/Height` is only *partly* this
  fold** (Codex R6-X3) — cssom-view §7 step 2 unions the principal box's own fragments, BUT for **an inline
  principal box split by a block-level descendant** it *"also include[s] fragments generated by the block-level
  descendants"* (webref-verified) — a **cross-entity** aggregation `box_fragments(entity)` (per-entity) cannot
  do. So `offsetWidth` = `union_border_boxes(entity)` **plus** the descendant boxes, aggregated by C-3b's
  dom-api offset algorithm (audit axis 6 home+shape: a cross-entity reader, like shell scroll-extent). The
  low fold stays **generic per-entity**; the cross-entity aggregation is the CSSOM layer's.
**C-3a ships NO `content_rect_local` relocation** (Codex R1-T9 + R2-U3 — dropped). R1 proposed moving the
RO-named `LayoutBox::content_rect_local()` to a generic `BoxModel` default; **R2-U3 showed that is wrong**:
its arithmetic is `Rect::new(padding.left, padding.top, …)` — **padding-only** — which is RO's contentRect
convention (RO §3.3.1: *"top is padding top, left is padding left"*), **NOT** a border-box-local content rect
(this codebase derives `border_box() = padding_box().expand(border())`, so the content origin relative to the
border box is `border + padding`, `boxes.rs:135-141`). Calling it "border-box-local" and baking padding-only
into a generic facet would give the wrong local origin for any bordered box. The correct home for a **local
frame is the reader** (I-frame): RO's contentRect **composes at the RO reader** (the `elidex-js` host / C-3d)
from the fragment's **generic** `BoxModel` facets — `Rect::new(f.padding().left, f.padding().top,
f.content().size.width, f.content().size.height)`, byte-identical to today. So `elidex-plugin`'s `BoxModel`
stays **purely generic** (no RO-semantic helper below the floor), and C-3a ships only `box_fragments` +
`principal_fragment` + `union_border_boxes`.

C-3a ships **no CSSOM-View algorithm, no RO-specific helper, and no frame-baking or source-choosing fold** —
those pre-commit per-consumer contracts the audit has not yet determined (the #463 failure mode). Downstream
slices build their reductions **on** these folds (§4 seeds).

### Layering — the FLOOR does the work; C-3a has no layering tension

The layering rule is **two-sided**: a **FLOOR by kind** (a spec / OM algorithm never lives below
`elidex-dom-api`; SSoT `docs/design/en/12-dom-cssom.md:4,104` + `docs/architecture/core.md:16-22`) and a
**CEILING by DIRECT-dependency callability** (a reduction must be **directly** depended-on by every crate that
calls it — §0: transitive linkage is not callability). Direct-callability of each candidate home:

| home | crates that can call it (direct dependents) |
|---|---|
| **`elidex-ecs` / `elidex-plugin`** (LOW) | **every** consumer (all `ecs=1 plugin=1`) |
| **`elidex-dom-api`** | `dom-api` (self), `elidex-form`, **`elidex-js`** (`engine`), `elidex-shell`, `elidex-wasm-runtime` — **NOT** a11y / layout(-flex/-grid/-multicol/-table) / render / api-observers |

**Everything C-3a ships is geometry math, not a CSSOM algorithm**, so the floor places it **LOW**, where it is
directly callable by every consumer — including the callability-`dom-api=0` crates (a11y, layout, render,
api-observers) that a dom-api home would strand. **C-3a therefore has no floor/ceiling collision**, and LOW
placement is not merely convenient but *required* for the a11y / layout / render readers.

The one genuine collision in the whole program is **downstream and C-3d's**: IntersectionObserver's registry
is in `elidex-api-observers` (callability-`dom-api=0`) yet IO needs the CSSOM-View §6 "get the bounding box"
algorithm (IO §3.2.10 step 7), which the floor keeps in `elidex-dom-api`. Resolved by dependency injection —
the registry takes a `rect_fn(&EcsDom, Entity) -> Option<Rect>` closure
(`crates/api/elidex-api-observers/src/intersection/mod.rs`) and the live closure lives in the **directly**
dom-api-dependent `elidex-js` host (`crates/script/elidex-js/src/vm/host/intersection_observer.rs:488-490`),
which *can* call the dom-api algorithm. ⚠ That DI seam is *why* the closure has silently returned
`LayoutBox.border_box()` — not the §6 bounding box — for the life of the feature, uncatchable by any
`api-observers` test. **C-3d decides** whether to keep the seam (b) or add the acyclic `api-observers → dom-api`
edge and implement IO step 7 engine-side (c). **C-3a does not touch it.** (Every other dom-api-homed reduction —
the CSSOM algorithms — is consumed only by `dom-api` itself + the `elidex-js` host, both direct callers; a11y
takes the LOW union, not the dom-api 4-branch. So C-3d is the *only* collision, re-derived on direct edges.)

---

## §2 Seam invariants (owned by C-3a — the read contract of the primitive)

| # | Invariant | Statement (all verified this session) |
|---|---|---|
| **I-router** | router = presence | `box_fragments` routes on **`fragments_for(entity)` non-empty**, never on `is_consumable` (a paint-only signal) and never on `LayoutBox`-absence. A nested-block mid-break (`consumable=false`) still occupies N column boxes that CSSOM/hit-test/a11y must see. |
| **I-coord** | primitive space | `BoxFragment.content` origin is in the **same document space** as `LayoutBox.content` origin — by construction: N=1 copies `content` verbatim (`fragment_tree.rs:154`); N>1's `shift_entity` equates the fragment origin with `LayoutBox.content.origin` physical space (`:286-288`). |
| **I-phase** | write-visibility window | `box_fragments` is **POST-LAYOUT, SCREEN-PASS ONLY** (below). |
| **I-boxless** | existence | empty sequence → `None`; **but this is not a universal consumer policy** (below). |
| **I-frame** | derived-helper basis | the folds are **frame-neutral** (doc-space or raw facets); consumer-local frames compose **at the reader** (below). |
| **I-transform** | transform basis | the primitive yields **PRE-TRANSFORM layout geometry**; CSS transforms are a paint-time wrapper, not baked in (below). |
| **I-lines** | store expressivity | the store holds **box** fragments only; **line fragments do not exist in it yet** (below). |

**Coupled-invariant crossings** (the intersections a downstream implementer must hold at once):

- **I-router × I-phase** — both answer "which store speaks"; presence-routing means a store that is empty
  *mid-pass* (I-phase) is read as boxless (I-boxless) — a wrong answer for an in-layout reader.
- **I-boxless × I-phase** — an empty `box_fragments` is ambiguous between "no box at all" and "not yet
  committed this pass"; the disambiguator is the `LayoutBox` fallback arm, which **C-4 deletes** → see the
  C-4 gate (§5).
- **I-coord × I-frame** — I-coord pins the *primitive's* origin (doc-space); I-frame pins the *derived
  helpers'* basis (which is often local). Conflating them is the #463 root (RO / baseline).
- **I-lines × source-change** — a consumer newly sourced from `getClientRects` (which draws on the
  line-level `InlineClientRects`) inherits the line-expressivity gap.
- **I-transform × N=1 (audit axis 4)** — a transform on a non-fragmented element is "behavior-neutral N=1
  routing-delta" under axis 4, yet its pre-transform gBCR/IO/a11y geometry is silently wrong; the auditor must
  hold I-transform against axis 4 or mis-mark a transformed reader "fully classified" (→ audit axis 8, §4).

### I-phase — the load-bearing one (four facts, all in live code)

`LayoutBox` and `FragmentTree` have **different authority windows**; `crates/layout/elidex-layout-block/src/
block/children/shift.rs:113-127` says so outright:

> *"under a probe we move ONLY the `LayoutBox` … `FragmentTree` box store: its write IS `is_probe`-guarded,
> so during a probe it holds the prior definitive pass's coords. Shifting it here would corrupt that
> definitive value."*

1. **Probe-lag** — `lb.content.origin += delta` is unguarded (`shift.rs:163-167`, every pass incl. probes);
   `shift_entity` is inside `if !is_probe` (`:218`), `push_box` likewise (`elidex-layout-multicol/src/lib.rs:
   541`). So during a **2-pass flex·grid·table re-measure** (a probe — the docstring names it), the store holds
   the *prior definitive pass's* geometry while `LayoutBox` holds the working value.
2. **Within-pass emptiness** — `clear()` runs at the **top** of `layout_tree`, before any root is laid
   (`elidex-layout/src/layout/mod.rs:325`), and `push_box` only commits in the definitive pass. So an entity
   has **no store fragments until its own definitive commit** — mid-layout, `box_fragments` → empty → `None`.
3. **Paged incoherence** — the paged/print path (`build_paged_display_lists_interleaved` →
   `layout_fragmented_with_tokens`) **does NOT `clear()`** (the `:315-324` docstring: *"does NOT clear here and
   may leave incidental dark fragments … committed-next"*), and its `fragmentainer` key is **page-relative**,
   so page 2 col 0 upserts over page 1 col 0.
4. **Teardown-stale** (Codex R1-T1) — the store is a **side-store teardown does not clean**: `destroy_entity` /
   `despawn_subtree` (`crates/core/elidex-ecs/src/dom/tree/teardown.rs:51,190`) despawn the ECS entity but
   leave its `FragmentTree` index entry (the only production `remove_entity` caller is the multicol committer,
   `elidex-layout-multicol/src/lib.rs:491`). So a despawned fragmented entity's index survives until the next
   screen `clear()`, and presence-routing would hand a **retained/stale `Entity` handle** stale geometry. →
   the seam's **liveness guard** (`box_fragments` checks `world.contains(entity)` before trusting the store)
   makes "empty iff no live box" hold **by construction**, not by teardown discipline (the by-construction fix
   over cleaning every teardown path — the store already has *"no incremental / staleness model; rebuild is the
   reconcile"*).

⇒ **`box_fragments` must be valid only after a completed SCREEN layout pass, and only for a live entity — and
it must ENFORCE this, not merely document it** (Codex R5-W3). A documented-only "screen-pass-only" precondition
is a footgun: a migrated reader can call the seam after a paged/print render and silently consume page-relative
geometry. **Requirements** (the mechanism — an `EcsDom` provenance enum, a generation counter, an access token
— is C-3a impl; the memo pins the requirements, not the state machine):
- (a) an **`EcsDom` layout-provenance signal** distinguishes *completed-screen* from *mid-layout / paged*;
- (b) **every non-screen layout entry actively sets a non-screen provenance BEFORE laying out** (Codex R6-X4) —
  NOT merely "does not set screen": a paged render after a prior screen pass would otherwise leave the stale
  *screen* provenance in place and pass the guard on page-relative fragments;
- (c) `box_fragments` **fails the guard** (distinct from box-absence, §1 requirement 3) unless the provenance is
  *completed-screen*.
(Reader-side guard; complements the producer-side C-4 gate item 3 paged-store hygiene.) The three **in-layout** baseline
readers (`elidex-layout-flex/src/baseline.rs:18`, `/src/lib.rs:474`, `elidex-layout-grid/src/position.rs:444`
— all `get::<&LayoutBox>` *inside* the layout algorithms) **must NOT be migrated onto `box_fragments`**; they
keep reading the live `LayoutBox`. Whether they get an explicit live accessor or simply stay on `LayoutBox`
until C-4 provides a probe-visible store is **C-3c's** decision — recorded here so the seam contract is
unambiguous.

### I-boxless — de-universalized, and barely reachable in production

Empty → `None`, **for the consumers that branch on box-presence** (a11y skips `set_bounds`; IO short-circuits
to ratio 0). Two corrections the #463 memo got wrong:

- **Not a universal class — `ResizeObserver` is spec-ZERO.** RO §3.3.1: *"observation will fire when watched
  Element display gets set to none"*; live `crates/api/elidex-api-observers/src/resize.rs:250-256` already
  does `size_fn(...).unwrap_or((Rect::default(), Size::ZERO))` with the comment *"box-less target … is NOT
  skipped"*. RO's `Option` is the **helper signature**, never the **reader policy**.
- **The `None` arm is an elidex invariant, NOT a spec branch.** Read literally, "get the bounding box" step 2
  returns an **all-zero `DOMRect`** for the empty list, and IO §3.2.10 steps 11-12 then report a boxless
  edge-adjacent target as *isIntersecting=true, ratio 1* (webref-verified: there is **no boxless guard**).
  elidex's `None` short-circuit is a deliberate deviation matching browsers (pinned:
  `elidex-api-observers/src/intersection/tests_core.rs:295-317`).
- ⚠ **Barely reachable**: **no production path removes `LayoutBox`** (`remove_one::<LayoutBox>` appears only
  in two *test* files); layout *skips* `display:none` subtrees but never *clears* a stale box
  (`elidex-layout-block/src/inline/pack/boxes.rs:96-101`, slot `#11-inline-relayout-box-staleness`). So an
  element that *becomes* `display:none` keeps its box, and the pinning tests synthesize a state the engine
  cannot reach. C-3 does not fix this; the audit records it and C-4 inherits the slot.

### I-frame — folds frame-neutral; the gate is LIVE

The folds return doc-space geometry or raw fragment facets; **consumer-local frames compose at the reader**,
arithmetic unchanged. The local-frame readers (audit inputs, verified live):

| Reader | Frame | Composed from |
|---|---|---|
| RO `contentRect` | **padding-offset** (top=padding top / left=padding left, RO §3.3.1 — **not** border-box-local, which would be border+padding) | composed at the RO reader: `Rect::new(f.padding().left, f.padding().top, f.content().size…)` from generic facets |
| flex `read_item_baselines` (`baseline.rs:18-26`) | margin-box cross-start-local — **no content-origin term** | its own arithmetic |
| flex/grid container baseline fallback (`lib.rs:477`, `position.rs:447`) | container-content-relative (a *difference*) | `content.origin.y − <container origin>.y` |
| `getBoundingClientRect` / `getClientRects` | **doc-space − `accumulated_scroll_offset`** (a *handler* step, not a fold; `layout_query.rs:30,215`) | the fragment's `BoxModel` facets |

⚠ The CSSOM family is **not one frame** — `offsetTop/Left` are offsetParent-relative (no scroll term,
`layout_query.rs:384`), `client*`/`scroll*` are frame-agnostic border-width/size reads. The row above is
**illustrative of the scroll-subtracting readers only**; each consumer's exact frame is **audit axis 1's
output**, not pinned here (pinning the whole family to "subtract scroll" is the per-consumer over-reach this
re-anchor exists to stop).

⚠ **The scroll subtraction is LIVE** (§0): `ScrollState` is inserted on the document root, so
`accumulated_scroll_offset` is non-zero on any scrolled page. A shared bounding-box fold must therefore be
**doc-space**, and each consumer applies its own frame (`getBoundingClientRect` subtracts; IO does not, since
its registry is doc-space). The C-3b regression test that checks a multicol `getBoundingClientRect` in
viewport coords **must exercise a scrolled page** (the production `ScrollState` provides the offset) — this is
falsifiable, not inert.

### I-transform — the primitive is pre-transform; the basis is a per-reader contract

CSS transforms are applied **at paint time as a display-list `PushTransform` wrapper** computed from
`lb.border_box()` (`crates/core/elidex-render/src/builder/transform.rs:17-25`) — they are **not** baked into
the layout box. So `box_fragments` yields **pre-transform layout geometry**, and the transform basis a reader
needs is **per-consumer and spec-mandated**:

- **layout (pre-transform) is CORRECT** for `offsetWidth/Height` (cssom-view §7 — *"ignoring any transforms
  that apply to the element and its ancestors"*), `client*`/`offsetTop`/`offsetLeft` (same §6/§7 "ignoring any
  transforms" clause), and ResizeObserver (resize-observer-1 §3.3.1 — *"observations will not be triggered by
  CSS transforms"*). (Codex R1-T4: these are the traceable anchors; the exact per-branch step cites are C-3b's
  / C-3d's coverage map, since C-3a implements none of these algorithms.)
- **painted (post-transform) is REQUIRED** for `getBoundingClientRect` / `getClientRects` (cssom-view §6
  getClientRects step 3 applies element+ancestor transforms) and IntersectionObserver — yet all of these read
  **raw** `border_box()` today (`layout_query.rs:340`, `intersection_observer.rs:490`, and a11y bounds
  `tree.rs:123`), a **pre-existing gap** (transform fidelity is out of C-3 scope; C-3 preserves current
  behavior — tracked as a **C-4 gate prerequisite**, §5).

⚠ This basis is **invisible to the N=1 behavior-neutral test**: a `transform:rotate` on a non-fragmented
element is "N=1 routing-delta only" (axis 4) while its pre-transform gBCR/IO/a11y geometry is silently wrong.
So the audit must classify it **explicitly** (axis 8, §4) — otherwise a reader is marked "fully classified"
while the transform contract is never captured, and C-4 could retire `LayoutBox` with the gap cemented. C-3a
does not *fix* the gap; it makes the basis an explicit audit output so the migration is transparent about
preserving it.

### I-lines — one root, not three symptoms

`FragmentContent` has a single `Box` variant; the entity index is keyed on **box-root entities only**; the
entity-less line / anonymous-block nodes (`FragmentContent::InlineLines`) are committed-next **dark data**
(`fragment_tree.rs:45-53, 93-111`). ⇒ the store **cannot express line fragments**, which is why inline
geometry lives in the parallel `InlineClientRects` component. This ONE gap is the root of several downstream
seeds (§4). `box_fragments`' domain is **box fragments only**; any line-level need is out of its contract.

### The N=1 behavior-neutral invariant — and its strict N>1 limit

C-3a's regression gate: for every **non-fragmented** entity, each fold reduces to the single `LayoutBox`'s
facet **bit-for-bit** (union == first == that one element), and the local-frame readers are unchanged because
`From<&LayoutBox>` copies `padding`/`content`/`first_baseline` verbatim. This is the seam's proof of no
regression for the overwhelmingly common entity.

⚠ **It holds ONLY at N=1.** At N>1 the single `LayoutBox` is the **G11 last-column box**
(`crates/core/elidex-plugin/src/layout_types/boxes.rs:116` *"the per-entity, G11 last-column box"*;
`elidex-layout-multicol/src/fill.rs:198` *"the next column's layout overwrites it (G11)"*). So **every**
fragment-sourced reduction changes value at N>1 — first-fragment readers go *last-column → first-column*,
union readers go *last-column → union of N*. **No consumer migration is "behavior-neutral" at N>1**; that is
the point of the migration (it fixes the multicol bug), and every N>1 consumer needs its own test. The audit's
source-vs-routing axis must treat N>1 accordingly — "routing-delta only" is an N=1 statement.

---

## §3 Spec coverage map

C-3a ships **geometry primitives, not CSSOM algorithms** (those are C-3b+), so its own spec surface is
minimal — the CSSOM-View branch enumeration belongs to the downstream slices that implement each algorithm,
and the per-consumer spec characterization is **the audit's output** (§4), not pre-filled here.

| Spec section | Step | Branch | Touch (C-3a code) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| RESIZE OBSERVER §3.3.1 content rect | — | `top`=padding top / `left`=padding left / `w,h`=content size | **C-3a ships nothing RO-specific** (R2-U3): RO's contentRect **composes at the reader** (C-3d host) from the fragment's generic `padding()` + `content().size` facets — byte-identical to today's `content_rect_local()` | ✓ (SVG-no-box + multicol width note are RO reader policy, → C-3d) | no |

**Breadth**: K=1 (resize-observer-1), M=1 — because C-3a is a **seam**, not a spec slice. The CSSOM-View map
(get-the-bounding-box 4-step/3-branch, offset*/client*/scroll*, getClientRects two-source, plus the
pre-existing transform gap — which is spec-*mandated-ignored* for offset*/client*/RO and a genuine gap only
for getClientRects/gBCR/IO) is **C-3b's** coverage map; IO §3.2.7/§3.2.10 is **C-3d's**. This memo does not
restate them.

> Preflight note (soft-warn): the coverage-map helper emits the label `RESIZE OBSERVER`, which
> `preflight.py`'s `SPEC_LABEL_REVERSE` does not map — a tooling seam, not a citation error; the row was
> webref-verified.

---

## §4 The audit — C-3a's central deliverable

C-3a produces the **exhaustive, classified `LayoutBox`-reader inventory**. This is the artifact every
downstream slice cites to pin its consumers' contracts, and the thing C-4's "zero `LayoutBox` reads outside
producers" gate is checked against. **It must be a durable, citable artifact** — committed as
`docs/audits/2026-07-layoutbox-reader-inventory.md` (NEW) alongside C-3a, and backed by the **compiler-based
gate** (below) — not a throwaway analysis (a non-durable inventory would let each slice re-derive the
classification, the very churn this re-anchor removes).

**The recipe is a human first-pass grep + classify, gated by a COMPILER check — NOT a grep-completeness claim**
(Codex R1-T6 / R2-U2 / re-gate-V5 / R5-W2 each found a reference shape a grep-list missed — a grep is
structurally non-exhaustive, so it cannot *be* the gate; §4's "exhaustive inventory" thesis demands the
compiler). So:

1. **The exhaustive use-site set is a COMPILER fact, not a grep fact** (Codex R1-T6 / R2-U2 / re-gate-V5 /
   R5-W2 each found a reference shape a grep-list missed — `&mut`, helper-params, generic bounds/type-aliases,
   then **import aliases** `use elidex_plugin::LayoutBox as LB; get::<&LB>`; grep structurally cannot follow
   aliases / re-exports / generic bounds, so any grep union claims a completeness it cannot deliver — the
   anti-pattern §4's own "exhaustive inventory" thesis forbids). Therefore:
   - **The SOUND enumeration is compiler-based** (requirement; the mechanism is impl's): the compiler must be
     made to **reject a `LayoutBox` reference from any non-producer crate**, so the workspace failing to compile
     enumerates every remaining consumer *by construction* (no alias/bound/shape escapes). ⚠ Plain `pub(crate)`
     does **NOT** express this (Codex R6-X2): `LayoutBox` lives in `elidex-plugin` but the *producer* crates
     (`elidex-layout-*`) are **also external** to it, so crate-privacy would reject the allowed producer writes
     too. It needs a **producer-allowlisting** mechanism — a sealed producer-facing API, a `deny`-by-default lint
     (clippy/dylint) with a producer-module allowlist, or module visibility separating producers from consumers.
   - **⚠ C-3a RUNS this check — it is not deferred to C-4** (Codex R7-Y2). §4's inventory is advertised as
     *exhaustive* and **C-3b–C-3e plan their contracts from it**; if C-3a shipped only the grep first-pass, the
     alias/generic/re-export cases would surface at C-4 — *after* four slices had planned from an incomplete
     inventory. So the compiler check is **the C-3a audit's exhaustiveness proof** (run once as a throwaway
     experiment if the permanent gate lands later); C-4 re-runs the same check as the deletion gate.
   - **`git grep -nw 'LayoutBox'` + `git grep -nE 'dyn BoxModel|impl BoxModel'` is the C-3a-audit HUMAN
     first-pass** — fast, catches the common shapes to classify — but is **explicitly NOT the sound gate** and
     must not be presented as exhaustive.
2. **Classify each hit** by the 8 axes. The reference *shapes* the sweep surfaces — illustrative, not
   exhaustive — include: `get::<&LayoutBox>` (shared read); **`get::<&mut …LayoutBox>`** (R1-T6 — mostly
   producer writes, but the C-4 "zero reads outside producers" gate can't be *proven* without classifying each
   as producer-write vs read-modify-write, e.g. `shift.rs:164`, `layout/mod.rs:112`); multi-component
   `query::<(..&LayoutBox..)>` (e.g. `scroll.rs:137` = `(&LayoutBox, &ComputedStyle)`); closure / `rect_fn`
   sites (injected observer geometry); **helper-signature params** `fn …(lb: &LayoutBox)` (R2-U2 — e.g.
   `render/builder/transform.rs:19`, `render/builder/form.rs` ×many; the caller `get`s and passes it down);
   and trait-erased `&dyn BoxModel`. These are the *common* shapes the first-pass grep surfaces for
   classification; a shape the grep misses (import alias, generic bound, re-export) is caught by the
   **compiler gate** (step 1), which is what makes coverage exhaustive — the grep is triage, the compiler is
   the proof.

**Eight classification axes** — a reader's contract is not pinned until ALL eight are answered against the
live reader (the #463 lesson: a read-site list is necessary but not sufficient; the contract axes are where it
went wrong):

| # | Axis | Question | Invariant |
|---|---|---|---|
| 1 | **frame** | doc-space, or a local frame the reader composes? | I-frame |
| 2 | **phase** | **in-layout** (must NOT use `box_fragments`) / **screen-post-layout** (valid) / **paged-post-layout** (INVALID — the paged path does not `clear()` and its `fragmentainer` is page-relative, I-phase fact 3; a render-residual reader under `paged:true` — e.g. `paint/mod.rs`, `form.rs` helpers — is "post-layout" yet reads page-relative geometry). Trinary, not binary — a binary post-vs-in-layout split marks a paged reader "fully classified" while nothing captures its paged-store invalidity | I-phase |
| 3 | **boxless** | spec-zero, or box-absent? — ⚠ and `display:contents` is a **pre-existing spec deviation the audit must record** (Codex R7-Y5): CSS Display 3 **§2.5** *"the element itself does not generate any boxes"* (webref-verified; the live comment at `layout/mod.rs:74` cites §2.8 — drifted), yet elidex synthesises a **zero-size `LayoutBox`**, so `box_fragments` yields **one zero-rect fragment, not box-absent** — a consumer migrating off `LayoutBox` would treat it as a real zero-sized box instead of taking its no-box / spec-zero branch (cssom-view §6 `getClientRects()` = empty list when there is no associated box). C-3 **inherits** the deviation (no regression); the audit records, per reader, whether it needs a true *"has an associated CSS box"* signal — and if any does, that is a C-3b/C-3d ask, not a silent inheritance. | I-boxless |
| 4 | **source vs routing** | does the migration change *which rects* feed it (⇒ a test), or only *which fragment*? (**everything is a source/behavior change at N>1** — the G11 last-column fact) | N=1 invariant limit |
| 5 | **reduction** | union / first / per-fragment / **not a geometry read** (e.g. the paged-gen gate reads `layout_generation`, which `BoxFragment` drops) / **a *selection* problem with no store signal** (the inline-text anchor) | — |
| 6 | **home + shape** | which crates must reach it (floor/ceiling)? and is it a **per-entity projection** or a **cross-entity aggregate** (e.g. shell scroll-extent is a `query` with a `display!=None` co-read — `box_fragments` cannot express it)? | layering |
| 7 | **identity / lifetime** | does the reader **retain** a store handle past the read? `FragmentId` is a generation-less index into a `Vec` that `clear()` resets each pass — a retained id re-aliases after relayout. Only plain values and `(entity, fragmentainer)` keys survive a pass — which is why the seam **yields `(fragmentainer, BoxFragment)`** (§1, R5-W1) so a retained hit fragment expresses that key without bypassing the seam. | I-phase |
| 8 | **transform basis** | does the reader's contract want **layout (pre-transform)** or **painted (post-transform)** geometry? `box_fragments` yields pre-transform (I-transform); gBCR/getClientRects/IO want painted but read raw today (pre-existing gap). Invisible to axis 4 — a transform on an N=1 element reads as "behavior-neutral". | I-transform |

**Known-hard seed edges** (audit INPUTS — questions the audit starts from, NOT determinations this memo
makes; each is a verified live reader):

1. **RO** — frame **padding-offset** composed at the reader (axis 1; RO §3.3.1 top=padding top/left=padding
   left, *not* border-box-local — R2-U3), spec-zero (axis 3). Open: which fragment (RO §3.3.1 pins *width*
   to the first column, silent on height). → C-3d.
2. **IO** — needs the CSSOM-View §6 fold in **doc space** (axis 1), `None` preserved (axis 3), a **source-change**
   (axis 4). Note: IO §3.2.7 step 6 maps entry rects to **viewport** space and elidex hands script **doc-space**
   rects — a pre-existing deviation, **live** on scrolled pages; record, don't bless. Home = the one genuine
   collision (§1). → C-3d.
3. **`getClientRects`** — two-source dispatch (line vs column); the both-split case is **I-lines**. → C-3b.
4. **`getBoundingClientRect`** — a **source-change** (axis 4): today it never consults `getClientRects`. → C-3b.
5. **render inline-text anchor** (`find_nearest_layout_box`) — a **selection problem with no store signal**
   (axis 5): the fn returns one ancestor box; `box_fragments(ancestor)` yields N and nothing maps an inline
   run to its column (**I-lines**). → C-3e.
6. **render paged-generation gate** — **not a box-geometry read** (axis 5): reads `layout_generation`, which
   `BoxFragment` drops. Needs a re-home, not `box_fragments`. → C-3e / C-4.
7. **shell scroll-extent** — a **cross-entity aggregate** with a `display!=None` co-read (axis 6). → C-3d.
8. **flex/grid baseline (×3)** — **in-layout** (axis 2) *and* three distinct local frames (axis 1) → stays on
   live `LayoutBox`. → C-3c.
9. **`ScrollIntoView` (C-3b) and shell URL-fragment nav (C-3d)** are the **same algorithm** (WHATWG HTML
   §7.4.6.4 "scroll to the fragment" **step 3 substep 5** — *"Scroll target into view, with behavior 'auto',
   block 'start', and inline 'nearest'"* — is the CSSOM-View "scroll a target into view" (§6.1); webref-verified, and
   Codex R1-T3-corrected from the bare "step 3", which only sets the target element) — **one shared helper**,
   decided once, not twice.

---

## §5 Downstream map (informative — refined when each slice is planned)

C-3a is the seed; the rest is sketched so C-3a knows what it enables. **The slice ordering is derived from
NEED, not from crate reachability** (the #463 error): the only hard cross-slice edges are **C-3b → C-3d** (IO's
target rect *is* C-3b's `get_the_bounding_box` fold) and **C-3c → C-3d** (iframe click-routing consumes C-3c's
hit fragment). C-3b, C-3c, C-3e are mutually independent given the C-3a seam; any further ordering is a
review-capacity choice.

```
C-3a (seam + audit) ──┬── C-3b  CSSOM geometry (elidex-dom-api)          ──┐
                      ├── C-3c  hit-test + a11y + baseline (layout/a11y) ──┴── C-3d  observers + shell
                      └── C-3e  render residual (elidex-render)
                                                          → C-4 (retire LayoutBox + legacy inline + InlineClientRects)
```

**C-4 retirement gate** (each item is a real prerequisite; the ones without an owner are flagged for PM):

1. Zero `LayoutBox` reads outside producers — proven by the **compiler-based gate** (§4: `LayoutBox` made
   producer-inaccessible + the workspace compiles), with the C-3a audit inventory as the human record. ⚠ "producers"
   must be defined precisely: several producer-crate reads are **in-layout** and one is a **presence check**
   (`inline/pack/boxes.rs:62`) whose meaning flips under a `clear()`ed store.
2. Producers write the store's N=1 box for every entity **AND** an empty `box_fragments` is **distinguishable**
   from boxless (a laid-this-pass marker / generation) — else in-layout readers (and the I-boxless × I-phase
   crossing) break. *(No owner — needs a slot.)*
3. **Paged-store hygiene** — the paged path must clear/rebuild or stamp provenance, else post-print reads
   return page-relative geometry. *(Committed-next per the code; no owner — needs a slot.)*
4. **`layout_generation` re-homed** — it serves the paged-gen gate AND the box-staleness generation-bump;
   `BoxFragment` drops it and `fragmentainer` cannot take either role. *(No owner — needs a slot.)*
5. **Line-fragment mapping landed** (`FragmentContent::InlineLines`, I-lines) — required before
   `InlineClientRects` can be retired, since C-3b/C-3d *deepen* the dependency on it. *(Committed-next; no
   owner — needs a slot.)*
6. **`#11-inline-relayout-box-staleness`** (+ its ledger sibling `#11-inline-align-clientrects-nonpersist-path`,
   which `project_open-defer-slots.md` folds into terminal-Z C-3/C-4) resolved or explicitly inherited.
7. **A design-doc slice for the fragment store** — it currently has **no design-doc home** (`git grep -li
   fragment_tree -- docs/design/` = zero; scoped to `docs/design/` per Codex R1-T5, since an unscoped `docs/`
   now matches this plan-memo itself), and `docs/design/en/15-rendering-pipeline.md` §15.4.1 ("Layer Tree as
   Independent Structure") still names `LayoutBox` as what the PaintSystem reads.
8. **The transform-basis gap recorded** (Codex R1-T2, I-transform §2) — `getBoundingClientRect` /
   `getClientRects` / IO / a11y-bounds read raw pre-transform `border_box()` today though their contract is
   painted geometry. C-3 preserves this, but C-4 must **not** retire `LayoutBox` while silently cementing it:
   either a `#11-*` slot (owner + re-eval trigger) or an explicit "inherited pre-existing gap" acknowledgement
   in the C-4 plan. *(No owner — needs a slot.)*

**Proposed `#11-*` slots for PM registration** (Codex R5-W4 — the defer content spelled out HERE, not deferred
to the landing memo; PM owns the ledger, this is the proposal + re-eval trigger for each of gate items 2-5, 7,
8). The memo does **not** create them (shared-SoT is PM-owned); it makes the defer auditable now.
⚠ **These are PROPOSALS, not registered slots** (Codex R7-Y4): the ledger's why/trigger/**date** triple is
completed **by PM at registration** (C-3a landing, §6.4) — the *why* is the gate item, the *trigger* is stated
below and is deliberately **event-based** (these gate a program, C-4, that has no calendar date yet; an invented
date would be false precision), and the *date* is the registration date PM stamps. Until then they are notes,
not ledger entries.

| Proposed slot | Gate item | Re-evaluation trigger |
|---|---|---|
| `#11-fragment-store-screen-provenance` | 2 (distinguish-empty + laid-marker) | C-4 kickoff, or any slice that needs a producer to write the store's N=1 box for every entity |
| `#11-paged-fragment-store-hygiene` | 3 (paged store not cleared) | when paged/print media folds into the store (committed-next per `fragment_tree.rs:73`) |
| `#11-layout-generation-rehome` | 4 (`layout_generation` dual-role) | C-3e (paged-gen gate reader) or C-4, whichever touches `builder/walk.rs:108` first |
| `#11-fragment-inline-lines` | 5 (`FragmentContent::InlineLines`) | the committed-next inline-line fold (already tracked as terminal-Z dark-data work) |
| `#11-fragment-store-design-doc` | 7 (no design-doc home) | C-4 (when the paint path migrates and §15.4.1's `LayoutBox` mention goes stale) |
| `#11-cssom-transform-fidelity` | 8 (raw pre-transform geometry) | any slice that closes the CSSOM/IO transform gap, else inherited-gap acknowledgement at C-4 |

(Gate item 6's two slots — `#11-inline-relayout-box-staleness` + `#11-inline-align-clientrects-nonpersist-path`
— already exist in `project_open-defer-slots.md`; the ledger folds them into terminal-Z C-3/C-4.)

---

## §6 Report to PM (coordination)

1. **PR #463 closed**, re-anchored on this C-3a-first memo (the umbrella characterized consumers before the
   audit that determines them; three collapses each re-introduced an unverified-premise defect). Codex R1-R7
   history preserved on branch `terminal-z-c3-plan` @ `7204c12e`.
2. **Two shared-SoT corrections still owed** (I do not edit the shared SoT): (a) the anchor memo's v2 retraction
   over-corrected to *"there is no `elidex-render` crate"* — it is real (`crates/core/elidex-render/`); only
   the *relocation* was fabricated. (b) the reader-crate lists should name **`elidex-js`** (the observer host),
   not `elidex-api-observers` (which is geometry-agnostic and untouched by C-3).
3. **C-3a is the isolatable seed** (`elidex-ecs`-centred, additive, **no consumer migration**) and is the right
   first PR. ⚠ **It is not `elidex-ecs`-ONLY** (Codex R7-Y3): enforcing the I-phase guard (§2) has a **writer
   side** — the screen layout entry must *publish* completed-screen provenance — so C-3a carries a **bounded
   cross-crate tail** at the layout entrypoint (`elidex-layout`). Actively invalidating on the *paged* entries
   is the `#11-paged-fragment-store-hygiene` slot (§5). Without this the guard degrades to the documented-only
   precondition §2 rejects, so the tail is in scope, not optional.
   **This memo pins the seam's CONTRACT + REQUIREMENTS; the ENFORCEMENT APPROACH is C-3a's implementation
   plan-review** — the phase-guard encoding (`Result` / token / guarded projection) and its propagation to the
   folds, the provenance representation, and the producer-allowlisting mechanism for the audit's compiler check
   are all decided there, against live code, per the per-slice plan-review discipline. Specifying them *here*
   is out of a decision-record's altitude and was the source of the R5→R7 finding cascade.
   Its deliverable is the seam **+ the durable audit artifact** (§4). The downstream slices are cross-crate and
   **not parallel-safe** with the CSS/script/shell lanes — schedule per §5.
4. **Six C-4 prerequisites currently have no owner** (§5 items 2-5, the design-doc slice, and the
   transform-basis gap). They gate `LayoutBox` deletion — none blocks C-3a. **The six proposed `#11-*` slots +
   their re-eval triggers are spelled out in the §5 table** (R5-W4 — the defer content is auditable now, not
   deferred to the landing memo); PM registers them in `project_open-defer-slots.md` at C-3a landing (the D-29
   "ship 時に登録" precedent), so PM owns tracked slot IDs from the moment the seed lands.
5. This memo is doc-only / parallel-safe.
