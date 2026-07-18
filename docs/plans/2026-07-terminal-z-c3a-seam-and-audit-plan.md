# Terminal-Z C-3a — the `box_fragments` seam + the reader audit (plan-memo)

**Status**: design anchor for **C-3a**, the first slice of the terminal-Z C-3 program (migrate non-paint
`LayoutBox` readers onto the fragment store, so C-4 can retire `LayoutBox`). **Reviewed: full 5-axis
`/elidex-plan-review` + two independent cumulative fix-delta re-gates + a Codex `/external-converge` loop;
all findings applied. (Round numbers live in this branch's git log — restating them here only drifts.)**
Doc-only. Written off a first-hand read of the LIVE store + a tool-authoritative dependency/write-site
check, 2026-07-14.

**Why this memo is C-3a-scoped, not a C-3 umbrella.** The prior C-3 umbrella (#463, closed) tried to pin
**each consumer's per-fragment contract** before the reader audit that determines it. That layer never
converged: every pass found another row whose contract had been *asserted but not verified against the live
reader*, and each structural collapse re-introduced the same defect class — a premise stated as "verified"
that was not. (§6.1 is the record of what happened; it is not restated here.) The umbrella's
own §4 had already said the quiet part: *"hand-enumerating every reader in this umbrella does not converge …
the exhaustive inventory is C-3a's audit output."* So this memo inverts the order: **C-3a ships the seam and
PRODUCES the audit; the per-consumer contracts are the audit's output**, owned by each downstream slice.
What this memo owns is verified against `cargo tree` / live code, or explicitly marked an audit question.

Predecessors MERGED: Z-1a (#313, standalone `FragmentTree`) / Z-1b (#316, per-column `InlineFlow`) / **C-1**
(#321 `48b0190b`, render consumes the store for `consumable` mid-break IFC entities) / **C-2** (#324
`b4e06897`, atomic-as-fragment).

---

> **Mandate invariant — the memo states what the audit MUST determine, never what it WILL find.**
> An enumeration that §4's sweep or an axis produces (which readers run in-layout; which producer paths leave
> a box on a boxless element; which consumers need a signal) is an **audit OUTPUT**. This memo does **not**
> pre-compute one — not as a list, not as a count, not "for readability". Naming an example to prove a class
> is **non-empty** is fine; naming it to define the class is not. If you catch yourself writing "the N cases
> are…" about something the audit determines, **delete it and state the mandate**.
> **This is the defect that killed PR #463** — §6.1 is the record. The rule is not "hand-list more
> carefully"; it is **do not hand-list**.
> ⚠ **Corollary — do not narrate the drafts.** "An earlier draft said X, which was wrong because Y" restates
> X, which makes a fresh drift site out of the fix itself; git holds that history. State the decision in its
> final form with the evidence that makes it checkable, and nothing about what it used to say.
>
> **Reading invariant — one fact, one home.** Every load-bearing fact in this memo is stated in exactly ONE
> section; every other section **points** at it. **Sections are deliberately NOT self-contained**: a second
> rendering of a fact is a *defect*, not a convenience, because the copies drift and each drift is a wrong
> decision by whoever read that section. This is not a style rule — it is the diagnosis of this memo's own
> review history (§6.1). CLAUDE.md *One issue, one way*: 単一の正準形に一括収束させ、strangler 中間状態を残さない。
> **If you must state a fact a second time to make a section readable, point instead. Never restate a count,
> a list, or a set** — a count is a copy of the thing counted (that copy is what drifted at §6.4).

## §0 Premise checks (this lineage has a fabrication history — every load-bearing fact is tool-verified)

The shared anchor memo once claimed the store was *relocated to `elidex-render` behind a dependency wall*
(a flaky-I/O fabrication), and the #463 umbrella asserted "verified" structural premises that were false
(§6.1). So every fact below is checked against an authoritative tool, not a prior memo:

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
  which is why C-3a places everything low (§1).
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
deletes.

**C-3a's deliverable set — the single statement of this slice's scope** (every other section points here;
Codex R14-re-gate found §1 and §6.3 disagreeing, and an implementer scopes from §1):
1. the **projection primitive** `box_fragments` + 2. the **frame-neutral folds** over it + 3. the **durable
reader audit** (§4 — a deliverable, not a by-product) + 4. **the layout-entry provenance writes** the §2
I-phase guard needs to be sound.
⚠ (4) makes C-3a **not `elidex-ecs`-only**: it is a seam requirement with a **writer side**, so C-3a carries a
bounded cross-crate tail at the `elidex-layout` entrypoints, and that tail is **indivisible** — see §6.3 for
why every entry (screen *and* paged) must participate and what it costs PM. Nothing else is consumer-facing.

**The projection primitive** — `EcsDom::box_fragments` (NEW), in a **new `crates/core/elidex-ecs/src/dom/
geometry.rs` (NEW)** (NOT appended to `dom/mod.rs`, which is **1073 LoC** — CLAUDE.md touch-time split;
`task_2924ead0`):

The **contract** C-3a's implementation must satisfy — **the memo pins the contract, NOT the Rust encoding**
(the exact signature is C-3a's implementation-plan-review decision; specifying a mechanism here is out of a
decision-record's altitude — R6 showed three glib mechanism-specs were each wrong):

```
impl EcsDom {
    /// (ILLUSTRATIVE signature — the CONTRACT is the five requirements below;
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
   pass" is a *guard failure*, never the boxless `None`/empty that means "no box." **The candidate encodings
   are listed here and only here** (every other section points at this list): a `Result::Err(InvalidPhase)`; a
   screen-geometry access token; a separate `try_*` accessor; or **folds defined only on an already-guarded
   projection** — impl's choice. ⚠ The last one is not interchangeable with the first three (Codex
   R15-re-gate): they are *per-call* guards that oblige **every** fold to re-discharge the check, while a
   guarded projection makes the propagation this requirement demands **structural** — which is what §2 asks for
   under CLAUDE.md *Security by structure, not review convention*, and what the illustrative signature above
   (`-> <phase-guarded projection>`) already assumes. A menu missing it would hand C-3a's plan-review only the
   options that cannot satisfy the constraint. Liveness is part of
   *box-absence*: a despawned entity (whose stale store index teardown does NOT clean — only the multicol
   committer calls `remove_entity`) reads as absent, checked via `world.contains` before trusting `fragments_for`.
4. **Pre-transform geometry** (§2 I-transform) — CSS transforms are a paint-time wrapper; a painted-geometry
   reader composes the chain itself.
5. **Box-absence and box-presence are MECHANICAL store facts, not "has / has-no associated CSS box" verdicts**
   (Codex R9-AA3; this requirement is that fact's only home — §3's row and §4 axis 3 point here and must not
   re-decide it). CSSOM consumers MUST branch on the distinction (`getClientRects()` returns an **empty** list
   when there is no associated box, cssom-view §6; `display:contents` generates **no box**, CSS Display 3 §2.5).
   Neither direction of the store fact is that predicate: **absence** is `{no associated box}` ∪ `{laid out after
   the last completed pass}` (§2 soundness-table row 4) — and **presence** only means a box was produced, which producer paths leave on spec-boxless
   elements. **What a true "has an associated CSS box" predicate requires, and what absence/presence is worth per
   reader, is §4 axis 3's to determine** against the producer paths it enumerates. The seam owes no extra facet;
   it reports the store faithfully, and the producer's presence is the lie. The class is non-empty (examples only
   — axis 3 enumerates and characterizes it): a `display:contents` element that is `position:absolute`, `fixed`,
   or `relative`, and a detached (parentless) element.

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
⚠ The prose below states each fold's **contract, not its signature** (Codex R7-Y1): every fold is derived from
`box_fragments` and therefore **inherits its phase guard** — requirement 3 **propagates to the folds**, so an
encoding that collapsed *invalid-phase* into the boxless "absent" would hand a migrated a11y/IO consumer the
boxless branch on stale page-relative geometry. The **encoding** is **C-3a's implementation-plan-review
decision**, like the primitive's (requirement 3 is where the candidate encodings are listed — once):

- `principal_fragment(entity)` → the first fragment (or the N=1 box); box-absent if boxless.
- `union_border_boxes(entity)` → the **plain** axis-aligned union of the fragment border boxes; box-absent if
  boxless. ⚠ **This is NOT the CSSOM-View "get the bounding box"** (Codex R1-T7): that algorithm returns the
  **first** rect when *every* rect has zero width **or** zero height (step 3, verbatim: *"If all rectangles in
  list have zero width or height, return the first rectangle in list"*), and otherwise unions only the rects
  *"of which the height or width is not zero"* (step 4) — i.e. it drops **only fully-degenerate 0×0** rects.
  A plain union differs from the spec algorithm — which is the point here: `union_border_boxes` is a
  **per-entity building block**, not a finished CSSOM reduction: **C-3b's `getBoundingClientRect` MUST build its
  own spec-shaped 4-step reduction, not reuse this fold**; and ⚠ **`offsetWidth/Height` is only *partly* this
  fold** (Codex R6-X3) — cssom-view §7 step 2 unions the principal box's own fragments, BUT for **an inline
  principal box split by a block-level descendant** it *"also include[s] fragments generated by the block-level
  descendants"* (webref-verified) — a **cross-entity** aggregation `box_fragments(entity)` (per-entity) cannot
  do. So `offsetWidth` = `union_border_boxes(entity)` **plus** the descendant boxes, aggregated by C-3b's
  dom-api offset algorithm (audit axis 6 home+shape: a cross-entity reader, like shell scroll-extent). The
  low fold stays **generic per-entity**; the cross-entity aggregation is the CSSOM layer's.
**C-3a ships NO `content_rect_local` relocation** (Codex R1-T9 + R2-U3 — dropped). The RO-named `LayoutBox::content_rect_local()`'s arithmetic is `Rect::new(padding.left, padding.top, …)` — **padding-only** — which is RO's contentRect
convention (RO §3.3.1: *"top is padding top, left is padding left"*), **NOT** a border-box-local content rect
(this codebase derives `border_box() = padding_box().expand(border())`, so the content origin relative to the
border box is `border + padding`, `boxes.rs:135-141`). Calling it "border-box-local" and baking padding-only
into a generic facet would give the wrong local origin for any bordered box. The correct home for a **local
frame is the reader** (I-frame): RO's contentRect **composes engine-side in `elidex-api-observers::resize`**, not the JS host (its facet math needs no dom-api, so unlike IO's `rect_fn` below it takes no host seam; C-3d, R22-JJ3)
from the fragment's **generic** `BoxModel` facets — `Rect::new(f.padding().left, f.padding().top,
f.content().size.width, f.content().size.height)`, byte-identical to today. So `elidex-plugin`'s `BoxModel`
stays **purely generic** (no RO-semantic helper below the floor). C-3a's scope is enumerated once, at the
top of §1.

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
| **`elidex-dom-api`** | **exactly the five crates §0 enumerates** (tool-verified in §0; not re-listed here) |

**Everything C-3a ships is geometry math, not a CSSOM algorithm**, so the floor places it **LOW**, where it is
directly callable by every consumer — **including the crates that cannot call dom-api at all** (§0's
tool-verified set is the authority on which those are) and that a dom-api home would therefore strand. **C-3a therefore has no floor/ceiling collision**, and LOW
placement is not merely convenient but *required* for the a11y / layout / render readers.

A collision the audit will have to handle — **an example, not the population** (axis 6 determines that) — is
**downstream and C-3d's**: IntersectionObserver's registry
is in `elidex-api-observers` (callability-`dom-api=0`) yet IO needs the CSSOM-View §6 "get the bounding box"
algorithm (IO §3.2.10 step 7), which the floor keeps in `elidex-dom-api`. Resolved by dependency injection —
the registry takes a `rect_fn(&EcsDom, Entity) -> Option<Rect>` closure
(`crates/api/elidex-api-observers/src/intersection/mod.rs`) and the live closure lives in the **directly**
dom-api-dependent `elidex-js` host (`crates/script/elidex-js/src/vm/host/intersection_observer.rs:488-490`),
which *can* call the dom-api algorithm. ⚠ That DI seam is *why* the closure has silently returned
`LayoutBox.border_box()` — not the §6 bounding box — for the life of the feature, uncatchable by any
`api-observers` test. **C-3d decides** whether to keep the seam (b) or add the acyclic `api-observers → dom-api`
edge and implement IO step 7 engine-side (c). **C-3a does not touch it.** (a11y reads a **single box's `border_box()`** today
(`crates/dom/elidex-a11y/src/tree.rs:123`, verified — not a union and not the dom-api 4-branch), so on **that**
shape it needs no dom-api-homed reduction. ⚠ A statement about today's read, **not a decision about a11y's
post-migration reduction**: which reduction a11y takes is **audit axis 1's** output, and §2 I-transform records
its geometry contract as *unresolved*. Which readers collide is **axis 6's** output — this paragraph forecloses
nothing.)

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
  routing-delta" under axis 4, and its geometry basis is I-transform's concern (below); the auditor must
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
   makes a **despawned** entity read empty **by construction**, not by teardown discipline (the by-construction fix
   over cleaning every teardown path — the store already has *"no incremental / staleness model; rebuild is the
   reconcile"*).

⇒ **REQUIREMENT (the memo's decision): `box_fragments` must FAIL its guard — distinctly from box-absence (§1
requirement 3) — unless the store reflects a COMPLETED SCREEN layout, and it must ENFORCE this, not merely
document it** (Codex R5-W3: a documented-only precondition is a footgun — a migrated reader silently consumes
page-relative geometry after a print render; CLAUDE.md *"Security by structure, not review convention"*).

⚠ **The enforcement PROTOCOL is C-3a's implementation plan-review, NOT this memo** (§6.3 — the rule this memo
already states and which R6-X4 → R8-Z1 → R9-AA1 each caught this memo violating: three rounds of hand-building
a protocol one rule at a time is exactly the enforcement-mechanism specification §6.3 routes away). What the
memo owes instead is the **soundness obligations** the C-3a protocol must discharge — each is a *live* hole a
naive design falls into, and each was found by review, so they are recorded here as the acceptance criteria,
not as a design:

| # | The protocol must not be fooled by | Why it is a real hole |
|---|---|---|
| 1 | **paged/print after a completed screen pass** (R8-Z1) | nothing distinguishes a screen-built from a paged-built store unless an entry marks it, so a stale *completed-screen* would stay green over page-relative fragments |
| 2 | **a re-entrant/second SCREEN pass, mid-flight** (R9-AA1) | `layout_tree` `clear()`s the store at the **top** of the pass (I-phase fact 2), so a stale *completed-screen* from the prior pass stays green while the store is empty/partial |
| 3 | **a probe pass** (I-phase fact 1) | the store holds the prior definitive pass's coords |
| 4 | **the DOM mutated after a completed screen pass** (re-gate #5) | the guard is **store-global provenance, not per-entity freshness**, so it stays green while a script-appended entity reads box-**absent** though it will get a box — elidex forces no reflow (`layout_tree` runs only from `shell/pipeline.rs`, never a read handler), so `box_fragments` returns the pre-mutation answer and C-3 **inherits** it (§1 requirement 5: absence is two-valued) |

(Reader-side guard; complements the producer-side C-4 gate item 3 paged-store **content** hygiene.)
**The seam contract, stated as a rule rather than a roster** (Codex R19-HH2): **any reader that runs *inside* a
layout algorithm must NOT be migrated onto `box_fragments`** — `box_fragments` is by contract unusable mid-pass,
so such a reader keeps reading the live `LayoutBox`. **Which readers those are is §4's sweep output, not this
memo's to list**. Whether the ones the sweep finds get an explicit live accessor or
simply stay on `LayoutBox` until a probe-visible store exists is **C-3c's** decision.

### I-boxless — de-universalized

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
- **No production path removes `LayoutBox`** (`remove_one::<LayoutBox>` appears only in two *test* files);
  layout *skips* `display:none` subtrees but never *clears* a stale box
  (`elidex-layout-block/src/inline/pack/boxes.rs:96-101`, slot `#11-inline-relayout-box-staleness`). So an
  element that *becomes* `display:none` keeps its box. C-3 does not fix this; the audit records it and C-4
  inherits the slot. *(How reachable box-absence is, is §1 requirement 5's — not this bullet's.)*

### I-frame — folds frame-neutral; the gate is LIVE

The folds return doc-space geometry or raw fragment facets; **consumer-local frames compose at the reader**,
arithmetic unchanged. **Each reader's frame is audit axis 1's output — this memo does not roster them** (an
earlier version's table both defined "the local-frame readers" and pinned three of their frames, two lines
above the note that says frames are not pinned here; its line numbers had already drifted from the copy §2
carried elsewhere). Examples, verified live, enough to show the frames genuinely differ:

| Reader | Frame | Composed from |
|---|---|---|
| RO `contentRect` | **padding-offset** (RO §3.3.1 — **not** border-box-local, which would be border+padding) | composed at the RO reader from generic facets — **the arithmetic and its byte-identity to today's `content_rect_local()` are pinned once, in §1's `content_rect_local` decision**; not restated here |
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
- **This is the gap's only DEFINITION; its MEMBERSHIP is audit axis 8's output** (mandate invariant — an
  earlier version rostered the raw readers here and counted them "all four", which a `git grep 'border_box()'`
  refutes: `elidex-render/src/builder/{paint,slice,transform,walk}.rs`, `resize_observer.rs:406` and
  `shell/content/scroll.rs` read it too). Keep the two facts apart, because they are **not** the same set:
  - **Reads raw `border_box()` today** — a code fact the sweep enumerates. Examples proving it non-empty:
    `getBoundingClientRect`/`getClientRects` (`layout_query.rs:340`), IntersectionObserver
    (`intersection_observer.rs:490`), a11y bounds (`tree.rs:123`).
  - **Contract is painted (post-transform) geometry** — a *cited* fact, and this memo establishes it **only**
    for gBCR/getClientRects (cssom-view §6 getClientRects step 3 applies element+ancestor transforms) and IO
    (§3.2.7 step 6 maps to the viewport's space). Anything else — a11y included — is **unresolved**, not
    "in": no citation was produced. Do not assert an uncited contract downstream; C-4 resolves it with a
    citation or acknowledges it as inherited.
  A reader is in the **gap** only where both hold, so axis 8 must answer both per reader — a roster here would
  pre-empt exactly that. Transform fidelity is out of C-3 scope (C-3 preserves current behavior) — tracked as a
  **C-4 gate prerequisite**, §5.

⚠ This basis is **invisible to the N=1 behavior-neutral test**: a `transform:rotate` on a non-fragmented
element is "N=1 routing-delta only" (axis 4) while its pre-transform gBCR/IO geometry is silently wrong.
⚠ **gBCR/IO, not a11y** — saying a11y's geometry is "wrong" presupposes the very contract this subsection says
was never cited (Codex R15-re-gate caught this line contradicting the fix four lines above it).
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
| RESIZE OBSERVER §3.3.1 content rect | — | the padding-offset convention (§1's `content_rect_local` decision is the home for the arithmetic and the byte-identity claim; not re-rendered here) | **C-3a ships nothing RO-specific** (R2-U3): RO's contentRect **composes engine-side** from the fragment's generic `padding()` + `content().size` facets — byte-identical to today's `content_rect_local()` | ✓ (SVG-no-box + multicol width note are RO reader policy, → C-3d) | no |
| CSS DISPLAY 3 §2.5 Box Generation | `contents` | *"the element itself does not generate any boxes"* → **no associated box** | → **§1 requirement 5** (its home) for what the seam does and does not express; not re-decided here. Producer paths that leave a box on such an element anyway are producer defects C-3 inherits — **axis 3 enumerates them**. | ✓ for the box-generation branch (`none` is out of C-3's reach — layout skips it; the `contents`-computes-to-`none` sub-case per Appendix B is unchanged) | no |
| CSSOM VIEW §6 `getClientRects()` | step 1 | *"does not have an associated box → return an **empty** DOMRectList and stop"* | the **consumer branch** requirement 5 exists to make expressible (C-3b implements the algorithm) | branch ✓ (steps 2-3 = SVG / per-fragment, C-3b's map) | no |

**Breadth**: K=3 (resize-observer-1, css-display-3, cssom-view-1), M=3 — still minimal because C-3a is a
**seam**: two rows are cited because CSSOM consumers must **branch** on the no-associated-box distinction
(requirement 5), so the branch belongs here, not left to C-3b (an uncited branch would let the seam ship with
no test for it, and force C-3b/C-3d back to duplicated style checks). The rest of the CSSOM-View map (get-the-bounding-box 4-step/3-branch,
offset*/client*/scroll*, getClientRects steps 2-3, plus the pre-existing transform gap — spec-*mandated-ignored*
for offset*/client*/RO, and a genuine gap whose membership §2 I-transform defines — not restated here) is
**C-3b's**; IO §3.2.7/§3.2.10 is
**C-3d's**. This memo does not restate them.

> ⚠ **The automated §3 citation-drift gate does NOT cover these rows — do not read the table as gate-verified**
> (Codex R13-CC1). `preflight.py` reports `parsed citations: 0` and `unrecognized labels: ['CSS DISPLAY 3',
> 'CSSOM VIEW', 'RESIZE OBSERVER']`: its `SPEC_LABEL_REVERSE` maps **no CSS module** beyond `selectors-4` /
> `geometry-1`, so **all three** rows are skipped. Each row was **manually** webref-verified — re-runnable:
> `.claude/tools/webref body resize-observer-1 content-rect-h` / `heading css-display-3 2` +
> `body css-display-3 box-generation` / `body cssom-view dom-element-getclientrects`.
> Closing the gap — extending `SPEC_LABEL_REVERSE` so the gate verifies these rows structurally instead of
> relying on this note — is **hand-off row 8** (§6.4), with an owner and a trigger. It is deliberately not a
> "separate follow-up" written here: an unowned prose follow-up is a dropped one (Codex R14-DD1).

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

1. **REQUIREMENT (the memo's decision): the inventory must be EXHAUSTIVE and must STAY current, and only the
   COMPILER can prove that — a grep cannot.** (Codex R1-T6 / R2-U2 / re-gate-V5 / R5-W2 each found a reference
   shape a grep-list missed — `&mut`, helper-params, generic bounds/type-aliases, then import aliases
   `use elidex_plugin::LayoutBox as LB; get::<&LB>`; grep structurally cannot follow aliases / re-exports /
   generic bounds, so any grep union claims a completeness it cannot deliver.) `git grep -nw 'LayoutBox'` +
   `git grep -nE 'dyn BoxModel|impl BoxModel'` remains the **human first-pass** for classification, explicitly
   **not** the proof.
   ⚠ **The enumeration METHOD and the standing check's shape are C-3a's implementation plan-review, NOT this
   memo** (§6.3 — the same rule R6-X2 → R7-Y2/R8-Z2 → R9-AA2/AA4 each caught this memo violating). The memo owes
   the **soundness obligations** the C-3a method must discharge — each a live hole found by review:

   | # | The proof must not be fooled by | Why it is a real hole |
   |---|---|---|
   | 1 | **`pub(crate)` as the boundary** (R6-X2) | `LayoutBox` is in `elidex-plugin` but the *producer* crates (`elidex-layout-*`) are **also external** to it — crate-privacy rejects the allowed producer writes too, so the boundary must **allowlist producers** |
   | 2 | **allowlisting `elidex-ecs` wholesale** (R9-AA4) | the seam itself (`box_fragments`) must read `LayoutBox` for the N=1 fallback (§1 req 2) — a broad `elidex-ecs` allowlist would let *future* low-level readers bypass the audit. The exception must be **seam-only**, so the proof stays *"every consumer read goes through `box_fragments`"* |
   | 3 | **a single compiler run** (R8-Z2) | a crate whose *dependency* failed is never type-checked (rustc needs the dep's metadata), so one run surfaces only the **first error layer** — an `elidex-dom-api` error hides `elidex-js`'s readers. `cargo check --keep-going` helps only *independent* crates. The method must **iterate to a no-new-errors fixed point** |
   | 4 | **running once, at C-3a, then rotting** (R7-Y2 + R9-AA2) | C-3b–C-3e plan their contracts **from this inventory**, so it must be exhaustive **at C-3a** (not deferred to C-4, or four slices plan from stale data) **and stay** exhaustive as slices land — a throwaway experiment leaves a later slice's new reader invisible until C-4. The freshness check must be **STRUCTURAL and mandatory** (a committed inventory + a standing check that diffs against it and fails). ⚠ **"each slice re-runs it" is NOT an acceptable substitute** (R11-BB1): that is a *review convention* — one slice forgetting leaves downstream planning on stale data until C-4, which is exactly what CLAUDE.md's *"Security by structure, not review convention"* forbids. A per-slice re-run is an **additional** workflow step, never the gate |
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
| 3 | **boxless** | spec-zero, or box-absent? — ⚠ and the **`display:contents` producer defects the audit must record** (Codex R7-Y5, **re-scoped at R17-FF1**): CSS Display 3 **§2.5** *"the element itself does not generate any boxes"* (webref-verified; live comments cite **§2.8** — `layout/mod.rs:71` and `elidex-layout-block/src/helpers.rs:355` both drift, and the axis must record whichever the sweep finds rather than trust this pair; webref: §2.8 = "The Root Element's Principal Box"). An *ordinary* such element is **already box-absent** (layout flattens it away before dispatch; that line never writes to the ECS), so `box_fragments` gives the spec answer by construction. **What this axis MUST determine — by sweep, not from this memo** (the mandate invariant above): **enumerate every producer path that leaves a `LayoutBox` on an element that has no associated CSS box — whether because it generates none *or because it is not connected*** (§1 requirement 5 gives the examples), and record, per reader, whether it would then read a real zero-sized box instead of taking its no-box branch (cssom-view §6 `getClientRects()` = empty list when there is no associated box). This memo names examples only to prove the class is non-empty (§1 requirement 5) — it does **not** enumerate it. C-3 **inherits** whatever the sweep finds (no regression). What the seam's answer is worth is **§1 requirement 5's**, not this axis's. This axis only **classifies**: per reader, does it need a true *"has an associated CSS box"* predicate, and is its zero-rect case spec-zero or box-absent? | I-boxless |
| 4 | **source vs routing** | does the migration change *which rects* feed it (⇒ a test), or only *which fragment*? (**everything is a source/behavior change at N>1** — the G11 last-column fact) | N=1 invariant limit |
| 5 | **reduction** | union / first / per-fragment / **not a geometry read** (e.g. the paged-gen gate reads `layout_generation`, which `BoxFragment` drops) / **a *selection* problem with no store signal** (the inline-text anchor) | — |
| 6 | **home + shape** | which crates must reach it (floor/ceiling)? and is it a **per-entity projection** or a **cross-entity aggregate** (e.g. shell scroll-extent is a `query` with a `display!=None` co-read — `box_fragments` cannot express it)? | layering |
| 7 | **identity / lifetime** | does the reader **retain** a store handle past the read? `FragmentId` is a generation-less index into a `Vec` that `clear()` resets each pass — a retained id re-aliases after relayout. Only plain values and `(entity, fragmentainer)` keys survive a pass — which is why §1 requirement 1 obliges each yielded box to **carry** its `fragmentainer` id, so a retained hit fragment expresses that key without bypassing the seam. *(How it is carried — tuple, field, self-identifying `BoxFragment` — is C-3a impl's, per §1; this axis does not pick.)* | I-phase |
| 8 | **transform basis** | does the reader's contract want **layout (pre-transform)** or **painted (post-transform)** geometry? `box_fragments` yields pre-transform (I-transform, which defines the gap's membership — this axis does not re-render it; note a11y's contract is *unresolved* there, so do not classify it as "wants painted"). Invisible to axis 4 — a transform on an N=1 element reads as "behavior-neutral". | I-transform |

**Known-hard seed edges** (audit INPUTS — questions the audit starts from, NOT determinations this memo
makes; each is a verified live reader):

1. **RO** — frame **padding-offset** composed at the reader (axis 1; the arithmetic is §1's, not restated: RO §3.3.1 top=padding top/left=padding
   left, *not* border-box-local — R2-U3), spec-zero (axis 3). Open: which fragment (RO §3.3.1 pins *width*
   to the first column, silent on height). → C-3d.
2. **IO** — needs the CSSOM-View §6 fold in **doc space** (axis 1), `None` preserved (axis 3), a **source-change**
   (axis 4). Note: IO §3.2.7 step 6 maps entry rects to **viewport** space and elidex hands script **doc-space**
   rects — a pre-existing deviation, **live** on scrolled pages; record, don't bless. Home = §1's floor/ceiling
   rule; whether other readers collide too is **axis 6's** to determine, not this seed's. → C-3d.
3. **`getClientRects`** — two-source dispatch (line vs column); the both-split case is **I-lines**. → C-3b.
4. **`getBoundingClientRect`** — a **source-change** (axis 4): today it never consults `getClientRects`. → C-3b.
5. **render inline-text anchor** (`find_nearest_layout_box`) — a **selection problem with no store signal**
   (axis 5): the fn returns one ancestor box; `box_fragments(ancestor)` yields N and nothing maps an inline
   run to its column (**I-lines**). → C-3e.
6. **render paged-generation gate** — **not a box-geometry read** (axis 5): reads `layout_generation`, which
   `BoxFragment` drops. Needs a re-home, not `box_fragments`. → C-3e / C-4.
7. **shell scroll-extent** — a **cross-entity aggregate** with a `display!=None` co-read (axis 6). → C-3d.
8. **flex/grid baseline** — **in-layout** (axis 2) *and* distinct local frames (axis 1) → stays on live
   `LayoutBox`. → C-3c. ⚠ A **seed, not the set**: the in-layout readers are whatever the sweep finds (§2's
   rule), and the flex/grid baseline sites are merely the ones known at authoring time.
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

1. Zero `LayoutBox` reads outside producers — proven the way §4 requires: **the compiler, not a grep**, with
   the C-3a audit inventory as the human record. *(§4 states the requirement and routes the enumeration method
   and the check's shape to C-3a's implementation plan-review.)* ⚠ "producers"
   must be defined precisely: some producer-crate reads are **in-layout** or **presence checks** (axes 2/5) whose meaning flips under a `clear()`ed store.
2. Producers write the store's N=1 box for every entity **AND** an empty `box_fragments` is **distinguishable**
   from boxless (a laid-this-pass marker / generation) — else the I-boxless × I-phase crossing breaks.
   *(→ hand-off row 1.)*
2b. **A PROBE-VISIBLE / current-pass geometry source exists for the in-layout readers** (Codex R11-BB2) —
   otherwise C-4 cannot delete `LayoutBox` at all. These readers run **inside** layout (§2 I-phase), where
   `box_fragments` is *by contract* unusable; item 2's "producers write the N=1 box" is a **post-commit** fact
   and does **not** give them a live source during a probe or mid-pass. So deleting `LayoutBox` on item 2 alone
   would either strand them or force them onto the screen-only seam the memo forbids. Either this prerequisite
   lands, or **`LayoutBox` survives C-4 for the in-layout readers**.
   ⚠ **Their set is §4's sweep output — this memo does not enumerate it** (Codex R18-GG3). That
   is exactly why §4's gate is a **complete `git grep -nw LayoutBox` sweep + the compiler**, not a hand-list: a
   PM ledger keyed to a hand-picked subset lets C-4 delete `LayoutBox` while stranding the rest.
   *(→ hand-off row 2 — deliberately its own row, not folded into row 1: provenance makes the guard sound,
   this makes the in-layout readers migratable.)*
3. **Paged-store CONTENT hygiene** — the paged path must clear/rebuild the store (its `fragmentainer` key is
   page-relative and it never clears, so it leaves incidental cross-page fragments). ⚠ Scope note (R8-Z1): this
   is **only** the store's *content*; the paged entries' **provenance invalidation** is **C-3a's** (§6.3) —
   the guard is unsound if split, so it is not deferred here. *(Committed-next per the code; → hand-off row 3.)*
4. **`layout_generation` re-homed** — it serves the paged-gen gate AND the box-staleness generation-bump;
   `BoxFragment` drops it and `fragmentainer` cannot take either role. *(→ hand-off row 4.)*
5. **Line-fragment mapping landed** (`FragmentContent::InlineLines`, I-lines) — required before
   `InlineClientRects` can be retired, since C-3b/C-3d *deepen* the dependency on it. *(Committed-next;
   → hand-off row 5.)*
6. **`#11-inline-relayout-box-staleness`** (+ its ledger sibling `#11-inline-align-clientrects-nonpersist-path`,
   which `project_open-defer-slots.md` folds into terminal-Z C-3/C-4) resolved or explicitly inherited.
7. **A design-doc slice for the fragment store** — it currently has **no design-doc home** (`git grep -li
   fragment_tree -- docs/design/` = zero; scoped to `docs/design/` per Codex R1-T5, since an unscoped `docs/`
   now matches this plan-memo itself), and `docs/design/en/15-rendering-pipeline.md` §15.4.1 ("Layer Tree as
   Independent Structure") still names `LayoutBox` as what the PaintSystem reads. *(→ hand-off row 6.)*
8. **The transform-basis gap recorded** (Codex R1-T2) — **its membership is §2 I-transform's, and this item
   does not re-render it**. What this *item* adds:
   whichever readers I-transform lists as the **cited** gap, C-4 must handle them; and for any raw-reader whose
   contract I-transform leaves **unresolved**, C-4 must resolve it *with a citation* rather than inherit an
   asserted one. C-3 preserves this, but C-4 must **not** retire `LayoutBox` while silently cementing it:
   either a `#11-*` slot (owner + re-eval trigger) or an explicit "inherited pre-existing gap" acknowledgement
   in the C-4 plan. *(→ hand-off row 7.)*

**The proposed `#11-*` slot for each gate item above is a row in §6.4's hand-off table** — the memo's single
record of everything that must outlive this PR. It is **not** duplicated here: this section is *informative*
(it characterises the gate), and a hand-off obligation is *normative*, so the record lives in the PM report.
Restating the slots — or their count — in this section is what made §6.4's count drift out of sync with the
table in the first place (Codex R13-CC2).

---

## §6 Report to PM (coordination)

1. **PR #463 closed**, re-anchored on this C-3a-first memo (the umbrella characterized consumers before the
   audit that determines them; three collapses each re-introduced an unverified-premise defect). Codex R1-R7
   history preserved on branch `terminal-z-c3-plan` @ `7204c12e`.
2. **Two shared-SoT corrections — hand-off rows 9 and 10** (§6.4 is the record: owner + trigger; this item is
   the *detail* PM needs to apply them). The memo does not edit the shared SoT itself. (a) the anchor memo's v2 retraction
   over-corrected to *"there is no `elidex-render` crate"* — it is real (`crates/core/elidex-render/`); only
   the *relocation* was fabricated. (b) the reader-crate lists should name **`elidex-js`** (the observer host),
   not `elidex-api-observers`. ⚠ Phrase it as *"the **current live** observer-geometry reader is the
   `elidex-js` host closure"* — **not** "api-observers is untouched by C-3" (Codex R11-BB4): §1 leaves C-3d the
   option (c) of adding the acyclic `api-observers → dom-api` edge and implementing IO §3.2.10 step 7
   engine-side, which **would** touch api-observers. A PM list that rules it out pre-empts a decision §1
   explicitly reserves for C-3d's plan-review.
3. **C-3a is the isolatable seed** (`elidex-ecs`-centred, additive, **no consumer migration**) and is the right
   first PR — at the scope §1 enumerates, which **includes the cross-crate provenance-write tail** (R7-Y3;
   §1 is that fact's home, this section is why it cannot shrink and what it costs PM).
   ⚠ **The provenance protocol is NOT divisible** (Codex R8-Z1): **every** layout entry must participate, and
   every entry **invalidates before laying out** — the screen entry additionally *publishes* completed-screen at
   completion. ⚠ **The screen entry's invalidation is not optional** (Codex R19-HH3): publishing only at completion is unsound: §2's own soundness table (R9-AA1) has `layout_tree` `clear()`ing the store at
   the **top** of the pass — so a **second screen pass** would leave the *prior* pass's green provenance
   standing over a cleared, half-rebuilt store, and any `box_fragments` read during it would collapse
   invalid-phase back into boxless geometry: exactly what the guard exists to prevent, and reachable without any
   paged render at all. Deferring the paged invalidation to `#11-paged-fragment-store-hygiene` would make the guard **unsound**: a paged render following a completed screen layout would leave the
   stale *completed-screen* provenance in place and `box_fragments` would return page-relative fragments under a
   **green** guard — precisely the failure §2 exists to prevent. Nothing outside the store can distinguish a
   screen-built from a paged-built store unless an entry marks it, so the paged entries are **in C-3a's scope**.
   (The slot remains, but for a *different* concern: the paged store's **content** hygiene — clear/rebuild —
   not the provenance protocol.) Without the whole protocol the guard degrades to the documented-only
   precondition §2 rejects, so the tail is in scope, not optional.
   **This memo pins the seam's CONTRACT + REQUIREMENTS; the ENFORCEMENT APPROACH is C-3a's implementation
   plan-review** — the phase-guard encoding (§1 requirement 3 lists the candidates; this section does not
   re-list them) and its propagation to the folds, the provenance representation, and the producer-allowlisting
   mechanism for the audit's compiler check
   are all decided there, against live code, per the per-slice plan-review discipline. Specifying them *here*
   is out of a decision-record's altitude and was the source of the R5→R7 finding cascade.
   Its deliverable is the seam **+ the durable audit artifact** (§4). The downstream slices are cross-crate and
   **not parallel-safe** with the CSS/script/shell lanes — schedule per §5.
4. **Hand-off table — the memo's SINGLE record of everything that must outlive this PR.** None of it blocks
   C-3a; all of it is lost if PM does not carry it.

   > **Hand-off invariant** (the root of Codex R13-CC2 / R14-DD1 / R14-DD2 — three findings, one class):
   > **every item that must outlive this PR is a ROW here, with an owner and a trigger. Prose may explain a
   > row; it must never *be* the record.** Each of those three was a separate prose hand-off — "follow-up
   > (separate, not this PR)", "still owed", a gate item with no row — and PM audits *this table* at landing,
   > not the prose, so a prose-only hand-off is a dropped hand-off. Patching them one at a time only produced
   > the next one (CC1's own fix *created* DD1). **Do not add a hand-off anywhere else in this memo, and do not
   > restate this table's contents or count** — a count is a copy of the row set, and keeping the copy in sync
   > is the CC2 defect, not a safeguard. CLAUDE.md *One issue, one way*: 単一の正準形に一括収束。§6.1 records
   > this same class — a duplicated decision surface — killing PR #463.

   ⚠ **Rows 1-7 and 12 are PROPOSALS, not registered slots** (Codex R7-Y4): the ledger's why/trigger/**date** triple is
   completed **by PM at registration** (C-3a landing) — the *why* is the gate item, the *trigger* is stated below
   and is deliberately **event-based** (these gate C-4, a program with no calendar date yet; an invented date
   would be false precision), and the *date* is the one PM stamps. Until then they are notes, not ledger entries.
   The memo does not create them (shared-SoT is PM-owned); it makes the defer auditable **now** (R5-W4), per the
   D-29 "ship 時に登録" precedent.

   | # | Hand-off item | What breaks if it is dropped | Owner → destination | Trigger |
   |---|---|---|---|---|
   | 1 | `#11-fragment-store-n1-coverage-marker` (gate item 2) — ⚠ **renamed** (Codex R14-re-gate): it was `#11-fragment-store-screen-provenance`, minted at R5 *before* R8-Z1 moved screen-provenance publishing **into C-3a's scope** (§6.3). Registering that string would tell a future session provenance is unbuilt while C-3a builds it — the SoT-pollution class §0 opens with. Same disambiguation §6.3 already applies to the paged slot | no producer writes the store's N=1 box for every entity, so "empty" stays ambiguous | PM → defer ledger | C-4 kickoff, or any slice needing that producer |
   | 2 | `#11-in-layout-probe-visible-geometry` (gate item **2b**) | **C-4 cannot delete `LayoutBox` for the readers that run INSIDE layout** — the set is §4's sweep output, not a hand-list (§4's sweep output). ⚠ distinct from row 1 (R13-CC2): item 2 is a *post-commit* fact; 2b is what they need **during** a probe/mid-pass | PM → defer ledger | C-4 kickoff — C-4 must land this or keep `LayoutBox` for them |
   | 3 | `#11-paged-fragment-store-hygiene` (gate item 3) | the paged store's content is never cleared/rebuilt | PM → defer ledger | when paged/print media folds into the store (committed-next per `fragment_tree.rs:73`) |
   | 4 | `#11-layout-generation-rehome` (gate item 4) | `layout_generation`'s dual role has no home once `BoxFragment` drops it | PM → defer ledger | C-3e (paged-gen gate reader) or C-4 — whichever touches `builder/walk.rs:108` first |
   | 5 | `#11-fragment-inline-lines` (gate item 5) | the store still cannot express `FragmentContent::InlineLines` (I-lines) | PM → defer ledger | the committed-next inline-line fold (tracked as terminal-Z dark-data work) |
   | 6 | `#11-fragment-store-design-doc` (gate item 7) | the store has no design-doc home and §15.4.1 keeps naming `LayoutBox` as what paint reads | PM → defer ledger | C-4 (when the paint path migrates) |
   | 7 | `#11-cssom-transform-fidelity` (gate item 8) | C-4 retires `LayoutBox` while silently cementing raw pre-transform geometry | PM → defer ledger | any slice closing the CSSOM/IO transform gap, else an inherited-gap acknowledgement in C-4's plan |
   | 8 | Map CSS modules in `preflight.py`'s `SPEC_LABEL_REVERSE` (`cssom-view` / `css-display-3` / `resize-observer-1`, and CSS modules generally) | **§3's citation gate stays blind** — `parsed citations: 0` for this memo *and* for every later plan citing a CSS module, so the structural webref gate silently degrades to manual convention (R14-DD1). CLAUDE.md already makes CSS modules webref-covered; the gap is the tooling's | PM → a tooling PR on `.claude/skills/elidex-plan-review/preflight.py` (**not** this doc-only PR: a shared-skill change every lane's plan-review runs) | before the next plan-memo citing a CSS module — C-3b at the latest |
   | 9 | Shared-SoT correction (a): `elidex-render` **is real** (detail → §6.2) | a later C-3 slice re-reads the anchor memo's over-correction and re-decides on a false crate premise — the exact defect class that collapsed #463 (R14-DD2) | PM → anchor memo | C-3a landing |
   | 10 | Shared-SoT correction (b): reader lists name **`elidex-js`** (detail + required phrasing → §6.2) | same class; and an "api-observers untouched" phrasing pre-empts the option (c) §1 reserves for C-3d's plan-review | PM → anchor memo | C-3a landing |
   | 11 | `MEMORY.md`'s Layout-lane line still says #463 "R7 待ち" | #463 is closed and re-anchored here (§6.1); the stale index line sends the next session to a dead PR | PM → `MEMORY.md` | C-3a landing |
   | 12 | `#11-find-roots-css-root-predicate` (§1 requirement 5 / axis 3) | `find_roots` treats every parentless-but-styled entity as a layout root (`dom/tree/navigation.rs` `root_entities`, excludes only `DocumentFragment`), so a **detached** element is re-laid against the viewport, so it has a real `LayoutBox` though it has no associated CSS box (cssom-view §6 = empty list) — C-3 inherits it (today's `getClientRects` has no connectedness guard); the seam reports presence faithfully, the producer's presence is the lie | PM → defer ledger | C-4, or any slice tightening `find_roots` to the CSS root element |

   (Gate item 6's two slots — `#11-inline-relayout-box-staleness` + `#11-inline-align-clientrects-nonpersist-path`
   — are **not** rows: they already exist in `project_open-defer-slots.md`, and the ledger folds them into
   terminal-Z C-3/C-4. Nothing to hand off.)
5. This memo is doc-only / parallel-safe.
