# Terminal-Z C-3a — implementation plan (the seam + audit the design memo routed here)

**Status**: implementation plan for **C-3a**. Its **design anchor / SSoT is the merged decision-record**
`docs/plans/2026-07-terminal-z-c3a-seam-and-audit-plan.md` (main @ `283bcc0d`, PR #470). That memo pins the
CONTRACTS + soundness OBLIGATIONS and **deliberately routes four mechanism decisions to this plan-review**
(memo §6.3): the phase-guard ENCODING (§1 req 3) + its propagation to the folds, the provenance
REPRESENTATION, and the producer-ALLOWLISTING mechanism for the audit's compiler check. This doc **decides**
those four, against live code, and is the input to the mandatory `/elidex-plan-review`.

**One-fact-one-home discipline (inherited from the design memo).** This plan does **not** restate the memo's
contracts — it **cites** them (`memo §N`) and states only the NEW implementation decisions. A second rendering
of a memo fact here is a drift site (the defect class that ran the #470 converge long —
`memory/feedback_semantic-sibling-selfseed-and-regate-breadth.md`). Where this plan must reference a contract,
it points; it does not copy.

**Ships with the impl PR** (plan-memo bundled with implementation, per
`memory/feedback_plan-memo-author-in-worktree.md`).

---

## §0 Scope, parallel-safety, split assessment

**The four deliverables (memo §1's single scope statement — pointer, not restatement):**
1. the projection primitive `box_fragments`; 2. the frame-neutral folds; 3. the durable reader audit (§4 of
the memo) + its compiler gate; 4. the layout-entry provenance writes the §2 I-phase guard needs.

**Parallel-safety (re-verified 2026-07-26 on live main `de235992`; the original 2026-07-18 `a139363f`
verification is superseded — #480/#479/#469/#471 have all merged into base).** C-3a edits `elidex-ecs`
(seam + phase field), `elidex-layout` (screen + paged provenance writes, §2 — the paged write is in
`layout_fragmented_with_tokens`, NOT `elidex-render`), `elidex-plugin` (**`Rect::union` only** — see the
constraint below), and `elidex-render` (`builder/walk.rs`: the local `union_rect` helper deleted and its
fold routed through `Rect::union`, plus one integration test that the interleaved paged driver leaves phase
`Invalid`, §5). Plus a grep trip-wire script + committed allowlist (D4, §3) + docs. **No new crate.**

Open non-dependabot PRs at re-verification — #484 (`elidex-ecs/src/dom/tree_clone.rs`, docs-only retag),
#485 (`elidex-shell` content test helpers) — touch **no** file C-3a touches; #485 was additionally checked
against the D4 gate (its head keeps all ten shell allowlist rows content-identical and introduces no token
into `content_test_support.rs`, which the `*_tests.rs` exclusion does **not** cover), so no allowlist drift.
dependabot #486's 34-crate bump may mechanically conflict on `Cargo.lock` — a trivial re-run, not a design
concern.

⚠ **The `elidex-plugin` constraint, restated to what it actually protects.** An earlier draft of this
section asserted "C-3a does NOT edit `elidex-plugin`" full stop; the shipped diff **does** edit it
(`layout_types/rect.rs`, +13, `Rect::union`), so that flat claim was false and is withdrawn. The
constraint's substance is unchanged and still holds: **no change to the `LayoutBox` / `BoxModel` surface**
— `BoxModel` stays purely generic (memo §1), the projection reads `elidex_plugin::LayoutBox` without
modifying it, and the D4 trip-wire is placed OUTSIDE `elidex-plugin`. `Rect::union` is a generic geometry
primitive on an unrelated type, added so render's hand-rolled `union_rect` could be deleted rather than
duplicated (One-issue-one-way). **Standing obligation**: any `elidex-plugin` edit — including this one —
must be re-checked against whatever CSS-lane PR is then in flight. Discharged here: no open PR touches
`crates/core/elidex-plugin/**`.

**Split assessment (CLAUDE.md touch-time / edge-dense).**
- `dom/mod.rs` is **1073 LoC**; the seam goes in a **NEW `dom/geometry.rs`**, not appended (memo §1;
  `task_2924ead0`). No separate split-PR needed — the seam is new-file by construction. `fragment_tree.rs`
  (568) and `elidex-layout/src/layout/mod.rs` (406) stay far under 1000 after their small additions.
- C-3a is a plan-review-gated slice under the approved terminal-Z umbrella ⇒ a **terminal unit** (base-case
  rule, `memory/feedback_edge-dense-mandatory-plan-review-and-split.md`). It does **no consumer migration**
  (that is C-3b–e). The D4 gate is a lightweight grep trip-wire (§3) — a shell script + allowlist, sibling to
  the existing `.claude/tools/*-trip-wire.sh` — so it **ships in C-3a with no stacked prereq** (the earlier
  dylint-infra size question is dissolved by the mechanism revision in §3).

---

## §1 Decision D1+D2 — phase-guard encoding = a guarded projection; folds inherit it structurally

**Decision.** Adopt memo §1 req-3 candidate **(d): folds defined only on an already-guarded projection**.
Concretely, a two-level API where the phase guard is discharged **once** (the phase is a DOM-global fact, not
per-entity) and the projection type is the *proof* that the folds require:

```rust
// crates/core/elidex-ecs/src/dom/geometry.rs  (NEW)

/// A read-only view of the DOM's box geometry, obtainable ONLY when the fragment
/// store reflects a COMPLETED SCREEN layout pass (memo §2 I-phase). Holding one is
/// the structural proof the phase guard passed — every fold is a method here, so a
/// caller cannot reach a fold without having discharged the guard.
pub struct ScreenGeometry<'a> {
    dom: &'a EcsDom, // borrows both `world` (liveness) and `fragment_tree` (fragments+phase)
}

impl EcsDom {
    /// The phase gate. `None` ⇒ the store is NOT a completed screen pass
    /// (mid-pass / re-entrant / paged / never-laid) — a signal DISTINCT from
    /// box-absence (memo §1 req 3). `Some` ⇒ every fold below is sound.
    #[must_use]
    pub fn screen_geometry(&self) -> Option<ScreenGeometry<'_>> {
        self.fragment_tree.is_completed_screen().then_some(ScreenGeometry { dom: self })
    }
}

impl<'a> ScreenGeometry<'a> {
    /// The projection primitive (memo §1). Box-absence = an EMPTY sequence
    /// (per-entity), NEVER conflated with the phase failure handled at the gate.
    pub fn box_fragments(&self, entity: Entity) -> impl Iterator<Item = FragmentView> + '_ { … }

    pub fn principal_fragment(&self, entity: Entity) -> Option<BoxFragment> { … }   // first, or N=1 box
    pub fn union_border_boxes(&self, entity: Entity) -> Option<Rect> { … }          // plain AABB union
}
```

**Why (d), not (a) `Result::Err(InvalidPhase)` / (b) an access token / (c) a `try_*` accessor.** Memo §1 req 3
(its only home) already argues the last candidate is *not interchangeable* with the first three: a/b/c are
**per-call** guards that oblige **every** fold to re-discharge the check, whereas the guarded projection makes
the propagation **structural** — you literally cannot name `principal_fragment` without a `ScreenGeometry` in
hand. This is the CLAUDE.md *Security by structure, not review convention* choice, and it is what discharges
**D2 (fold propagation)** for free: the folds inherit the guard because they are defined *on the proof*, so an
encoding that collapsed invalid-phase into the boxless branch (memo §2 **I-boxless × I-phase** crossing / §1
req 3's two-signal separation) is unrepresentable.

**This design satisfies memo §1 req 3's two-signal separation exactly:**
- **phase-invalid** → `screen_geometry()` returns `None` at the gate (DOM-global, checked once).
- **box-absent** → an empty `box_fragments` / `None` fold **inside** a valid projection (per-entity).
The two can never alias: they are returned by different calls at different levels.

**Liveness (memo §2 I-phase fact 4, teardown-stale).** `box_fragments` guards each entity with
`self.dom.contains(entity)` (the existing `EcsDom::contains`, `dom/mod.rs:327`) BEFORE trusting the store, so a
despawned entity whose stale `FragmentTree` index entry survives reads **empty by construction** — box-absent,
not a phantom. Liveness is per-entity (inside `box_fragments`), distinct from the DOM-global phase gate.

**Router = presence (memo §2 I-router / §1 req 2).** Inside a valid projection, `box_fragments(e)` routes on
`fragment_tree.fragments_for(e)` being non-empty → yield those N `FragmentView`s; else → the single `LayoutBox`
component as one fragment `(fragmentainer None, BoxFragment::from(&lb))`. Never routes on `LayoutBox`-absence, never
on `is_consumable` (a paint-only signal).

**Fragment identity (memo §1 req 1).** Each yielded `FragmentView` **carries** its `fragmentainer` id (yielded,
not inferred — a span starting in a later column has `fragmentainer ≠ enumeration index`). Shape:

```rust
/// One box fragment of an entity, carrying its stable fragmentainer id (memo §1 req 1)
/// so a C-3c retained-hit / C-3d iframe-routing caller has the `(entity, fragmentainer)`
/// key without bypassing the seam. Owned + lightweight (BoxFragment is 5 plain fields);
/// the N=1 arm synthesizes it, the N>1 arm clones the store node's — so one uniform item
/// type across both arms (no borrow/owned iterator split).
pub struct FragmentView {
    /// `Some(n)` = store-sourced and authoritative. `None` = the N=1 fallback arm, which
    /// has no fragmentainer to report. NOT `u32`-defaulting-to-0: the store fragments only
    /// entities it breaks, so a non-spanning child inside a later multicol column falls
    /// through the fallback arm — a `0` there is a fabricated column the consumer cannot
    /// distinguish from a real column 0, which would silently mis-key exactly the two
    /// per-column consumers req 1 exists for (C-3c hit-test, C-3d iframe routing).
    pub fragmentainer: Option<u32>,
    pub box_model: BoxFragment,
}
```

`BoxFragment` already impls `BoxModel` and `From<&LayoutBox>` (`fragment_tree.rs:131,146`) ⇒ **zero new type
machinery** (memo §1 "Why on `EcsDom`" pt 2). The folds are pure `Rect`/size math over `FragmentView` (memo §1).

**N=1 behavior-neutral gate (memo §1 "The N=1 behavior-neutral invariant").** For a non-fragmented entity,
`box_fragments` yields exactly one `FragmentView { fragmentainer: None, box_model: From<&LayoutBox> }`, and both
folds reduce to that single box's facet **bit-for-bit** (union == first == the one box). Regression-pinned in §5.
⚠ Strictly N=1 only: at N>1 every fold changes value (the single `LayoutBox` is the G11 last-column box) — that
is the migration's point, and each N>1 consumer is C-3b–e's own test, not C-3a's.

**Naming.** `ScreenGeometry` names the *phase* (memo §2 I-phase: "POST-LAYOUT, SCREEN-PASS ONLY"). Its folds are
**doc-space / raw-facet** (memo §2 I-frame), NOT viewport space — the doc-comment must say so to avoid a
"screen = viewport coords" misread. Name is a plan-review bikeshed; the *contract* is fixed.

---

## §2 Decision D3 — provenance representation = a phase field on `FragmentTree`, driven at the layout entries

**Decision.** A private phase discriminant on `FragmentTree` (the store owns its own validity — the cohesive,
ECS-native home), written by the layout entrypoints and read by `screen_geometry()`:

```rust
// crates/core/elidex-ecs/src/fragment_tree.rs
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum StorePhase {
    /// Not trustworthy as screen geometry: mid-pass, re-entrant, paged/print, or never laid.
    #[default]
    Invalid,
    /// The store reflects a COMPLETED screen layout pass — the only state `screen_geometry()` accepts.
    CompletedScreen,
}
// FragmentTree gains: `phase: StorePhase` + `invalidate(&mut self)` /
// `publish_completed_screen(&mut self)` / `is_completed_screen(&self) -> bool`.
```

**Driver writes (the indivisible cross-crate tail, memo §1 (4) / §6.3).** Every layout entry **invalidates
before laying out**; **only the screen entry publishes completed-screen at completion**:

⚠ **This section originally specified an ENTRY-mark protocol (an explicit `invalidate()` at each layout
entry). That mechanism was superseded mid-review and the entry marks are gone** — Codex PR#488 R1 showed the
protocol left the `LayoutBox` **component** half uncovered (a direct post-publish `dispatch_layout_child`
rewrote components while `CompletedScreen` stood, so the projection served a mixed generation), and the fix
for that — a mark at the `dispatch_layout_child` bracket — subsumes every entry mark, since every layout
entry reaches the algorithms only through it. Keeping both would be the "new seam + N legacy
implementations" strangler state CLAUDE.md *One issue, one way* forbids, and it demonstrably cost a round:
the entry-mark framing is what led R2 to ask for an invalidate before the paged entry's zero-write early
returns. The rows below are kept as the **historical decision record**; the shipped mechanism is the one
rule stated immediately after.

**The shipped rule — one sentence.** *Any write to either geometry source invalidates the phase; only
`layout_tree` publishes, at completion.* Realised in exactly four places, no others:

| Source | Site | Mechanism |
|---|---|---|
| store contents | `FragmentTree::clear` | invalidates — an emptied store is definitionally not a completed pass |
| store contents | `push_box` / `remove_entity` / `shift_entity` | `invalidate_on_write()`, and **only when they actually write** (a no-op must not demote a published store) |
| `LayoutBox` component | `elidex_layout::dispatch_layout_child` | invalidates at its bracket — the single entry every layout algorithm is reached through, covering all 14 `insert_one` sites across 6 crates |
| — | `layout_tree`, after its root loop | the sole `publish_completed_screen()` |

*Superseded entry-mark rows (historical):*

| Entry | Crate / site | Write |
|---|---|---|
| **screen** | `elidex-layout` `layout_tree` | ~~`invalidate()` at top~~ — dropped; `clear()` on the next line invalidates by construction. `publish_completed_screen()` after the root loop remains |
| **paged** | `elidex-layout` `layout_fragmented_with_tokens` | ~~`invalidate()` at top~~ — dropped; its fragmentainer loop calls `dispatch_layout_child`, so the bracket covers it |

**Why `layout_fragmented_with_tokens` is the COMPLETE paged locus (plan-review re-check — verified live).** The
paged store has **three** writers, and this one fn covers all three:
- **interleaved driver Phase 1** (`render/builder/mod.rs:345`) calls it directly → covered;
- **`layout_paged`** (`layout/mod.rs:167`, `pub`, the layout-crate paged entry) reaches it via `layout_fragmented`
  (`:207`) → covered;
- **interleaved driver Phase 2** (`render/builder/mod.rs:366`, a per-page **direct** `dispatch_layout_child`
  that bypasses this fn) writes **after** Phase 1, so it **inherits** Phase 1's `Invalid` — and Phase-1-before-
  Phase-2 is a **structural data dependency** (Phase 1 computes the page count Phase 2 loops over), not an
  incidental ordering, so this is by-structure, not review-convention.
⚠ **A render-driver-top placement would be INCOMPLETE** — it brackets both interleaved phases but **misses
`layout_paged`** (which never routes through the driver); and a Phase-2-only site would miss Phase 1 +
`layout_paged`. `layout_fragmented_with_tokens` is the **only single site** covering all three (Phase 1 +
`layout_paged` directly, Phase 2 by structural inheritance). (The legacy `build_paged_display_lists`,
`render/builder/mod.rs:192`, takes `dom: &EcsDom` — **read-only paint from pre-laid fragments**, not a store
writer, so it needs no invalidate.) ⇒ **C-3a does NOT edit `elidex-render`** for provenance; the paged write
stays in `elidex-layout`, matching the memo's "elidex-layout entrypoints".

**Why an explicit paged invalidate is REQUIRED at all (memo §6.3 — the load-bearing subtlety).** The paged path
does **not** `clear()` and may write **zero** `push_box`es (e.g. a page with no multicol content). Without the
paged `invalidate()`, a prior screen pass's `CompletedScreen` would stay green over a page-relative store
(soundness hole 1). So the paged invalidation cannot be deferred to `#11-paged-fragment-store-hygiene` (that
slot is the store's *content* hygiene — clear/rebuild — a distinct concern, memo §5 gate item 3).
Invalidation ≠ content-clearing.

**Soundness — the memo's §2 table (three holes) discharged by this mechanism:**

| Hole (memo §2) | How the mechanism forecloses it |
|---|---|
| 1. paged/print after a completed screen pass | `layout_fragmented_with_tokens` `invalidate()`s before Phase 1 (and before `layout_paged`'s writes) ⇒ the stale `CompletedScreen` is cleared even on a zero-write page; Phase 2 inherits the `Invalid` |
| 2. re-entrant / second SCREEN pass mid-flight | the new `layout_tree` `invalidate()`s at its top (before `clear()`) ⇒ pass-1's `CompletedScreen` is gone while pass-2's store is empty/partial |
| 3. a probe pass | probes run *inside* `layout_tree` (flex/grid/table re-measure via `dispatch_layout_child(is_probe:true)`), which is bracketed by top-`invalidate()` → completion-`publish` ⇒ any read during a probe sees `Invalid` |

**Single-publisher invariant.** **Only `layout_tree` may call `publish_completed_screen()`.** Every other store
mutation (paged `layout_fragmented_with_tokens` + Phase 2, screen multicol) at most `invalidate()`s or leaves
phase untouched — so a redundant invalidate is always harmless and a stray publish is the only way to reintroduce
a hole. Pin it with a test that BOTH paged entries (the interleaved driver AND `layout_paged`) leave phase
`Invalid` **whenever they lay anything out** (§5 item 6), and the impl must confirm no site other than
`layout_tree` publishes.

⚠ **That qualifier is load-bearing, and an earlier draft omitted it** (Codex PR#488 R2 read the
unqualified form and asked for an `invalidate()` before the paged entries' validation returns). The
invariant is about the store's **CONTENTS**, not about which mode is executing: `CompletedScreen` means
"the store reflects a completed screen pass". A paged attempt that bails on `roots.is_empty()` or a
non-positive content area writes **nothing** (`find_roots` / `find_roots_mut` both take `&EcsDom`), so the
store still holds exactly that completed screen pass and `Some` is the truthful answer. Invalidating there
would be a **spurious demotion** — the same defect removed from `remove_entity` in R1 under the rule *a
mutator that writes nothing must not demote a published store* — and would force a full relayout before
any screen-geometry read whenever a print attempt no-ops. State the requirement at WRITE granularity, which
is the granularity the mechanism actually has. Pinned by
`a_zero_write_paged_early_return_leaves_the_phase_alone`. **Note** `layout_fragmented_with_tokens` is NOT on the normal screen path (screen multicol commits via `elidex-layout-multicol`, not `layout_fragmented`), so this adds no screen-path entanglement.

**Additive to the existing render consumer (no regression).** Render reads `fragment_tree().is_consumable()` /
`.fragments_for()` / `.is_empty()` **directly** today (`render/builder/walk.rs:207–294`, the C-1 consumer).
C-3a does **not** migrate it (that is C-3e/C-4). The phase field and its methods are **purely additive** — the
existing store methods are unchanged, and the paint walk runs after `layout_tree` completes (so it observes a
`CompletedScreen` store regardless, though it does not consult phase). Adding `phase: StorePhase` to
`FragmentTree` keeps its `Clone, Debug, Default` derives (`StorePhase: Default = Invalid`). **Verify at impl:**
no `FragmentTree` construction site relies on field-exhaustive literals (it is constructed via `::default()`),
so the new field is backward-safe.

**Defense-in-depth (adopted — `/code-review` overturned the initial call).** `clear()` **also** sets
`Invalid`, making a cleared store intrinsically non-completed. The initial draft kept `clear()`
arena-only (one auditable locus per entry); `/code-review` flagged that this rests the "cleared store ≠
completed pass" invariant on caller ordering (a future teardown/navigation `clear()` without a preceding
`invalidate()` would strand a stale `CompletedScreen`). Coupling the invalidate into `clear()` closes it
**by construction** (*Security by structure*) and does not weaken the single-publisher rule — only
`layout_tree` re-`publish`es, after clearing + laying out. The entries still `invalidate()` explicitly
(entry-level protocol); `clear()`'s invalidate is the backstop.

---

## §3 Decision D4 — the audit's standing gate = a `LayoutBox`-reader trip-wire + a name-introduction ban

**Decision.** The reader-audit's exhaustiveness/freshness gate (memo §4 req 1) is a **grep trip-wire**
(`.claude/tools/layout-box-reader-trip-wire.sh`, wired into the existing `mise run trip-wires` task) checked
against a **committed allowlist**, made **exhaustive by construction** by an accompanying **name-introduction
ban**. This is the CLAUDE.md *One-issue-one-way* choice: it converges on the repo's existing standing-check
mechanism (`.claude/tools/*-trip-wire.sh`, `native-ctor-guard-trip-wire.sh` is the closest sibling — token +
exclusion-allowlist) rather than introducing a second, heavier mechanism, and it stays on the **stable
toolchain**.

**Why this REVISES the memo's "only the COMPILER can prove — a grep cannot" premise (§4 req 1) — with the
premise's ONE surviving grep-miss handled by grep breadth, not hand-waved.** The memo routed the enumeration
**method** to this plan-review; its "grep cannot" premise names three miss-classes — **aliases / re-exports /
generic bounds**. A focused re-check (Axis 2/3 adversarial) confirmed the ban defeats the first two but
**refuted** a first draft that claimed generic bounds "contain the token": a `fn f<T: BoxModel>(x: &T)` reader,
instantiated with `LayoutBox`, has **no** `LayoutBox` token, and a bare `T: BoxModel` / `where T: BoxModel`
bound is **neither `dyn` nor `impl`** — so the narrow `dyn BoxModel|impl BoxModel` grep misses it too. The fix
is **grep breadth, not a claim**: the machine gate greps bare **`git grep -nw BoxModel`** (all bound/`dyn`/`impl`
uses carry the `BoxModel` token; `-w` excludes `BoxFragment`), accepting allowlist noise (25 `BoxModel` sites
today) in exchange for catching every generic-bound reader. So the three miss-classes are handled as:
- **Present-token — caught by the two token greps directly**: `&mut LayoutBox`, helper-params
  `fn f(lb: &LayoutBox)`, fully-qualified `elidex_plugin::LayoutBox`, `&dyn BoxModel` — all carry `LayoutBox`
  or `BoxModel`.
- **Generic bounds — caught by the BROADENED `-nw BoxModel` grep** (`<T: BoxModel>` / `where T: BoxModel` carry
  the `BoxModel` token; the narrow `dyn|impl` grep did **not**, which is why the gate is bare `BoxModel`).
- **Aliases / type-aliases / aliased re-exports — defeated by the name-ban** (of **both** `LayoutBox` AND
  `BoxModel`): grep-able patterns `LayoutBox|BoxModel as`, `type\s+\w+\s*=.*(LayoutBox|BoxModel)`, aliased
  `pub use`. Same "ban the evasion idioms" strategy as `native-ctor-guard-trip-wire.sh`. ⚠ The ban is
  line-oriented, so a rustfmt-unwrapped multi-line `type X =\n …LayoutBox;` could slip the *ban* grep — but the
  alias **definition line still carries the token**, so the read-diff catches it as an unclassified read
  (backstop); rustfmt single-lines these in practice.

**Verified this session (grep + broadened-grep + ban is exhaustive TODAY):**
- **Zero aliasing** — `git grep -nE 'LayoutBox as |type \w+ *=.*LayoutBox|pub use .*LayoutBox'` returns only
  the canonical export `elidex-plugin/src/layout_types/mod.rs:7` (token present). No use-site loses the token.
- **Zero generic-bound readers** — `git grep -nE ': *BoxModel|where.*BoxModel'` (minus the trait def/impls)
  returns none. The broadened gate is a *future*-proofing (the shape is idiomatic here — `render/builder/walk.rs`
  runs a `&dyn BoxModel` geometry-agnostic loop; a `dyn→generic` refactor would add exactly this reader shape).
- **Zero token-hiding macros** — the `macro_rules!` in `LayoutBox`/`BoxModel`-token files (`impl_layout_handler!`
  `elidex-dom-api/.../layout_query.rs:40`, `impl_string_map!` `elidex-ecs/src/components.rs:11`) both either take
  their body as `$body:expr` at the **call site** or carry only doc-comment tokens — neither expands to a
  token-less geometry read. The trip-wire guards against a *new* `macro_rules!` in a reader-token file without an
  allowlist classification, so a token-hiding macro cannot land silently.

**The one residual `dylint` would additionally have caught** is a *future macro* that expands to a **token-less**
`LayoutBox`/`BoxModel` read (source contains neither token). It is **verifiably absent today** and guarded (the
new-macro guard above); if one is ever introduced, escalate D4 to a `dylint` HIR lint (documented
future-escalation, not needed now — a nightly-toolchain + lint-crate cost the stable trip-wire avoids). Generic
bounds are **not** in this residual — the broadened `-nw BoxModel` grep catches them on the stable toolchain.

**How the trip-wire + allowlist discharges the four §4 soundness holes:**

| Hole (memo §4) | Mechanism |
|---|---|
| 1. `pub(crate)` rejects producers too | the allowlist is **data, not crate-visibility** — `elidex-layout-*` **producer** reads are listed as `producer` entries (excluded like `native-ctor-guard`'s SoT/test exclusions), so external-crate producers are permitted without weakening privacy anywhere |
| 2. allowlisting `elidex-ecs` wholesale | the seam's own N=1 `LayoutBox` read (§1 req 2) is a **single** allowlisted line in `dom/geometry.rs`, tagged `seam` — NOT a blanket `elidex-ecs` exclusion, so a *future* low-level reader still trips |
| 3. a single compiler run hides dep-blocked crates | **does not apply to grep** — the trip-wire reads source files directly, independent of compilation order, so there is no first-error-layer masking (a strict improvement over any compile-error / dylint mechanism, which *would* need `--keep-going` + a fixed point) |
| 4. runs once then rots | the trip-wire is **standing** in `mise run trip-wires` (⊂ `mise run ci` — the pre-push gate; NOT GitHub CI, see §6), diffs live `LayoutBox`/`BoxModel` reads against the committed allowlist, and **exits non-zero on any read not in it** — a new reader forces a classification. Not a per-slice re-run (the review convention the memo rejects); it is `git`-enforced on every push |

**Allowlist shape** (committed alongside the audit doc; the doc is the human record, this its machine-checked sibling):
```
# each entry: crate, path:line, reader-kind, classification, downstream-slice
elidex-render , builder/transform.rs:19 , helper-param    , pending-migration , C-3e
elidex-dom-api, layout_query.rs:30      , get<&LayoutBox> , pending-migration , C-3b
elidex-a11y   , tree.rs:123             , get<&LayoutBox> , pending-migration , C-3c
elidex-ecs    , dom/geometry.rs         , From<&LayoutBox>, seam              , —
elidex-layout-block, block/children/shift.rs:164 , get<&mut LayoutBox>, producer , —
…
```
At **C-3a** the allowlist enumerates the current exhaustive reader set (mostly `pending-migration` + the
`producer`/`seam` permanents) → the trip-wire passes. As C-3b–e migrate readers, their rows are **deleted**; by
C-4 only `producer` + `seam` remain → that is C-4 gate item 1's *"zero reads outside producers"*, now enforced
by the same trip-wire (memo §5). So the C-3a mechanism **is** the C-4 gate, tightening monotonically.

**Scope: the trip-wire ships IN C-3a (no stacked prereq).** It is a shell script + a committed allowlist on the
**stable toolchain** — no toolchain change, no new crate, no separate infra PR (the dylint-infra size question
that would have forced an A-vs-B split is **dissolved**). ⚠ Honest sizing: it is **not** literally as light as
the existing `*-trip-wire.sh` — those are single-crate **idiom-blacklists** (grep a few known-bad strings +
`grep -v` carve-outs), whereas this is a **workspace-wide positive allowlist-diff** over a proliferating token
(`LayoutBox`/`BoxModel` — 104 files, 710 token occurrences, verified 2026-07-18 via
`git grep -lwE 'LayoutBox|BoxModel'`) **plus** the name-ban — a heavier, more maintenance-prone data structure
(the `path:line` keys drift on edits above a reader ⇒ allowlist churn; the impl may prefer a line-insensitive
key). It is still **trip-wire-class** (shell + grep + committed allowlist, `mise run trip-wires`) and
stable-toolchain, which is the One-issue-one-way point — just not zero-cost. C-3a's diff = seam + provenance +
audit-doc + trip-wire + allowlist, all reviewable together.

**The audit DOC** (`docs/audits/2026-07-layoutbox-reader-inventory.md`, memo §4): one row per reader, columns =
the **8 classification axes** (memo §4 table) + `(file:line, crate, reader-kind, classification, slice)`.
Produced by the human first-pass `git grep -nw LayoutBox` + `git grep -nw BoxModel`, **classified**, then held
exhaustive/durable by the trip-wire (the machine check the memo's "compiler, not a grep-completeness claim"
requirement demanded — satisfied here by **broadened-grep + name-ban**, which a plain `git grep` is not, per
the premise-revision above).

---

## §4 The folds — exact semantics C-3a ships (and what it deliberately does NOT)

Pointer, not restatement — memo §1 is the home:
- `principal_fragment` = first fragment (or the N=1 box); box-absent → `None`.
- `union_border_boxes` = the **plain axis-aligned union** of fragment border boxes; box-absent → `None`. **NOT**
  the CSSOM-View "get the bounding box" 4-step reduction (`cssom-view-1 §6`, `#element-get-the-bounding-box`:
  that drops rects with zero width or height and returns-first when all-degenerate) — C-3b builds its own spec-shaped
  reduction ON this, not by reusing it.
- C-3a ships **NO** `content_rect_local` relocation, **NO** CSSOM-View algorithm, **NO** RO-specific helper,
  **NO** frame-baking / source-choosing / transform-composing fold (memo §1 + §2 I-frame/I-transform). Those are
  per-consumer contracts the audit has not yet determined — pre-committing them is the #463 failure mode.

`BoxModel` in `elidex-plugin` stays **purely generic** (no RO/CSSOM helper below the layering floor) — the D0
parallel-safety constraint (§0) and memo §1 layering both require it.

---

## §5 Test matrix (the acceptance gate)

All in `dom/geometry.rs` `#[cfg(test)]` unless noted. Anchored on the memo's invariants:

1. **N=1 behavior-neutral** (memo §1) — a non-fragmented entity: `box_fragments` yields one `FragmentView`
   `{fragmentainer:0}` whose `box_model` is `From<&LayoutBox>` bit-for-bit; `principal_fragment == that box`;
   `union_border_boxes == that box's border_box`. The no-regression proof for the common entity.
2. **N>1 routing** (memo §1 strict-limit / §2 I-router) — a 2-column mid-break entity: `box_fragments` yields
   both columns in fragmentainer order with the correct `fragmentainer` ids (1st ≠ enumeration index case
   covered); `principal_fragment == first column` (not the G11 last-column box); `union_border_boxes == union
   of both`.
3. **Phase gate — the 3 soundness holes** (memo §2 table) — drive a `FragmentTree` through: (a) mid-pass
   (invalidated, not yet published) ⇒ `screen_geometry() == None`; (b) published-then-re-invalidated (paged
   after screen) ⇒ `None`; (c) published ⇒ `Some`, folds valid. Assert **phase-invalid ≠ box-absent**: a
   valid projection over a boxless entity gives `Some(proj)` + empty `box_fragments`, whereas an invalid phase
   gives `None` at the gate — the two are observably different call results.
4. **Liveness / teardown-stale** (memo §2 fact 4) — push a fragment for `e`, despawn `e` (leaving the stale
   index entry), publish: `box_fragments(e)` is **empty** (guarded by `contains`), not a phantom.
5. **box-absence vs box-presence is a mechanical store fact** (memo §1 req 5) — a valid projection reports
   presence faithfully for a producer-left box on a spec-boxless element; C-3a asserts it does **not** add a
   "has associated CSS box" predicate (that is C-3b/axis-3). A record-the-store test, not a spec-branch test.
6. **Provenance driver integration** — (a) `elidex-layout` test: after `layout_tree` `screen_geometry()` is
   `Some`; after `layout_paged` it is `None` (exercises the `layout_fragmented_with_tokens` invalidate on the
   layout-side paged entry). (b) `elidex-render` integration test: after `build_paged_display_lists_interleaved`
   it is `None` (exercises the production interleaved path — Phase 1 invalidate + Phase 2's structural
   inheritance, §2). Both guard that **no** paged path publishes `CompletedScreen` (single-publisher), not just
   the store methods.

(The scrolled-page falsifiability note, memo §2 I-frame, is **C-3b's** `getBoundingClientRect` test — C-3a
ships no scroll-subtracting reader, so it is out of this matrix.)

---

## §6 Touch list, landing checklist, hand-off

**Files touched:**
- `crates/core/elidex-ecs/src/fragment_tree.rs` — `StorePhase` + phase methods.
- `crates/core/elidex-ecs/src/dom/geometry.rs` **(NEW)** — `ScreenGeometry`, `FragmentView`, `box_fragments`,
  folds, tests; `mod geometry;` wired in `dom/mod.rs`.
- `crates/layout/elidex-layout/src/layout/mod.rs` — screen provenance at `layout_tree` (invalidate + publish) +
  paged provenance `invalidate()` at `layout_fragmented_with_tokens` (§2, complete over all 3 paged writers);
  layout-side driver integration test.
- `crates/core/elidex-plugin/src/layout_types/rect.rs` — `Rect::union` (the generic smallest-enclosing-rect
  primitive). No `LayoutBox`/`BoxModel` surface change — see the §0 constraint.
- `crates/core/elidex-render/src/builder/walk.rs` — the local `union_rect` helper deleted, its paged-multicol
  fold routed through `Rect::union` via `Iterator::reduce` (One-issue-one-way).
- `crates/core/elidex-render/src/builder/tests/paged.rs` — integration test (interleaved paged driver leaves
  phase `Invalid`, §5 item 6b).
- `docs/audits/2026-07-layoutbox-reader-inventory.md` **(NEW)** — the audit + its machine-checked allowlist sibling.
- `.claude/tools/layout-box-reader-trip-wire.sh` **(NEW)** — the D4 trip-wire (name-introduction ban with a
  positive control + the classification-vocabulary wire), wired into `mise.toml` `[tasks.trip-wires]`; placed
  OUTSIDE `elidex-plugin` (§0 constraint); no new crate, stable toolchain.
- `docs/plans/2026-07-terminal-z-c3a-impl-plan.md` — this plan (bundled).

**Landing checklist — the §6.4 hand-off actions at C-3a-IMPL landing.** The merged memo's §6.4 table is the
record; the memo header fixes **registration at C-3a landing** (the *trigger* column is each slot's
**resolution** event, not its registration timing — memo §6.4: *"the ledger's why/trigger/date triple is
completed by PM at registration (C-3a landing) … Until then they are notes, not ledger entries"* / D-29 "ship
時に登録" precedent). ⚠ This **corrects** the handoff memo (`project_c3a-implementation-next.md`), which said
"register as its trigger fires" — that framing is superseded by the design SoT. So at THIS PR's landing, PM:
- **Register rows 1–8 and 12 into the defer ledger** (`project_open-defer-slots.md`) — the why/trigger/date
  triple, with each row's memo-§6.4 **trigger** (C-4 / C-3b / C-3e) recorded as the *resolution* event. These
  are the C-3/C-4 **program** hand-offs the merged memo pre-enumerated and ratified — not this PR's own deferred
  work, so they are outside the per-PR ≤3 governance (they are executing the ratified plan, not new defers).
  (Row 8 `#11-preflight-css-module-labels` registers here too; its resolution trigger is *before C-3b*. C-3a's
  §7 CSS-module citations do **not** independently fire it — they are the memo's *inherited* citations, not
  NEW ones, and this plan's preflight already ran clean, soft-warn only.)
- **Apply row 9** — shared-SoT correction (a): *"there is no `elidex-render` crate"* is wrong
  (`crates/core/elidex-render/` is real; only the FragmentStore *relocation* was fabricated). ⚠ **The
  over-correction is duplicated** — fix **every** surface, not "the anchor memo" alone (semantic-sibling
  discipline, the plan's own preamble): verified live in **`terminal-z-committed-next-fragment-walk-plan.md:21`**
  (the anchor memo) **AND `project_post-boa-deletion-paydown-campaign.md:19`** (the ACTIVE 4-lane campaign SoT,
  linked from MEMORY.md "▶ CURRENT" → more likely to be re-read). At landing, grep the concept across the
  memory dir (`grep -rniE 'no .{0,4}elidex-render' memory/`) and correct each hit to "elidex-render is real; it
  does not own the FragmentStore (which lives `elidex-ecs/src/fragment_tree.rs`)".
- **Apply row 10** — shared-SoT correction (b): reader-crate lists name **`elidex-js`** (the live observer-host
  reader), phrased *"the current live observer-geometry reader is the `elidex-js` host closure"* — NOT
  "api-observers untouched" (that pre-empts C-3d's option (c), memo §6.2).
- **Verify row 11** — MEMORY.md Layout-lane line (already done per handoff memo).

**New hand-off created by C-3a — THREE slots + one cross-lane obligation.** (An earlier draft said "none";
that was written before the second `/elidex-review` pass, and is withdrawn.) The audit *produces* the
classified inventory downstream slices cite, and beyond the memo's §6.4 pre-enumerated set it surfaces:

1. **`#11-layoutbox-absence-unreachable`** — there is no `LayoutBox` **removal** path, so the seam's
   "box-absent" branch fires only for a never-laid entity; a `display:none`-toggled element keeps its last
   box forever and the N=1 fallback yields it as live geometry. Inherited (C-3a changes no producer), but it
   makes the audit's per-reader axis-3 rows describe a state today's engine cannot produce for a once-laid
   target — IO/RO are the exposed readers. Resolution trigger = C-4, or any slice needing a truthful
   box-absent signal. Same missing fact as §6.4 row 1 from the component side.
2. **`#11-layoutbox-field-typed-reader-coverage`** — a `LayoutBox`-typed BINDING (struct field or fn
   param) carries the token at its declaration but not at any read through it. Wire #1 forces a row for a
   NEW binding; the *reads through* an existing one are enumerated by hand only. A wire #5 that claimed to
   bound the family was written and then withdrawn — its regex missed `&'a LayoutBox` and a new field in an
   already-listed file passed every wire — so this is genuinely unmachined. Resolution trigger = C-4, which
   must not read a green gate as covering it.
3. **`#11-layoutbox-trip-wire-not-in-ci`** — the D4 gate runs only in the local `mise run ci`; no GitHub
   workflow invokes it (see the ⚠ below). Until it is in a workflow its verdict is a pre-push habit rather
   than an enforced invariant, which is load-bearing because C-4's delete decision reads it. Resolution
   trigger = before C-4, or the next `.github/workflows` touch.

*(A further candidate — the fallback fragment reporting a fabricated column `0` — was **fixed in-slice**
rather than slotted: `FragmentView::fragmentainer` is now `Option<u32>`, so "unknown" is unrepresentable
as a column index. It was a now-or-never call — the type has zero production consumers today, and the
same change after C-3b–e adopt the seam would be a breaking one.)*

**Cross-lane obligation (not a slot — a standing CI fact PM must carry into the campaign SoT):** the D4
trip-wire is wired into `mise run ci` and greps **all** of `crates/**/*.rs`. Any PR in any lane that adds or
edits a non-test `LayoutBox`/`BoxModel` line must now update the allowlist *and* this audit. That blast
radius is **semantic, not textual** — a concurrent PR can conflict with no line — so §0's file-overlap
parallel-safety analysis does not cover it.

⚠ **And the gate cannot catch it either: it does not run in CI.** `.github/workflows/ci.yml` runs only
cargo fmt/clippy/nextest/doc/deny; its sole `mise` reference is the string `'mise.toml'` inside the
paths-filter list. So a lane that adds a reader merges all-green and main carries a stale allowlist until
someone happens to run `mise run ci` locally. An earlier draft of this paragraph claimed such a PR would
"red CI on main" — false, and withdrawn. Until the gate is in a workflow its verdict is a pre-push habit,
not an enforced invariant, which matters because C-4's delete decision is taken against it. New slot
**`#11-layoutbox-trip-wire-not-in-ci`** (resolution trigger: before C-4, or the next `.github/workflows`
touch).

---

## §7. Spec coverage map

C-3a ships geometry primitives, not CSSOM algorithms ⇒ minimal spec surface. **The design memo §3 is the
authoritative home**; the table below is reproduced **solely to satisfy the plan-review preflight's structural
gate** (a `## §N Spec coverage map` table is a mechanical hard-gate) — it is not a second decision surface, and
the memo governs on any divergence. This plan implements **none** of these algorithms, so it adds no new
citations; it inherits the memo's. All three labels are CSS modules unmapped in `preflight.py`'s
`SPEC_LABEL_REVERSE`, so preflight reports them `unrecognized` (soft-warn) — each was **manually webref-verified**
in the #470 converge (memo §3 gate note); closing that tooling gap is memo §6.4 row 8 (owner + trigger there).

| Spec section | Step | Branch | Touch (C-3a code) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| RESIZE OBSERVER §3.3.1 content rect | — | padding-offset convention | **C-3a ships nothing RO-specific** — the seam yields generic facets; RO composition is C-3d's reader | ✓ | no |
| CSS DISPLAY 3 §2.5 Box Generation | `contents` | element generates no box → no associated box | → memo §1 req 5 (its home): the seam reports mechanical store presence; producer paths that leave a box on such an element are producer defects axis 3 enumerates | ✓ (box-generation branch) | no |
| CSSOM VIEW §6 `getClientRects()` | step 1 | no associated box → empty `DOMRectList` | **C-3b's branch** — the seam gives mechanical presence; the no-associated-box *predicate* is audit axis 3's, not C-3a's | step 1 predicate → C-3b | no |

(Preflight computes K/M from the table; not restated here to avoid a count-copy drift site.) The remaining
CSSOM-View / IO / RO algorithm surface and the transform-basis gap are C-3b–e per memo §3 / §5 — this plan does
not restate them.

---

## §8 Within-PR sequencing

1. `fragment_tree.rs` phase field + methods (leaf; no dependents break).
2. `dom/geometry.rs` seam + folds + `mod geometry;` (depends on 1).
3. `elidex-layout` screen + paged provenance writes (§2) + layout & render driver-integration tests (depend on 1).
4. audit doc + committed allowlist + the D4 trip-wire (`.claude/tools/layout-box-reader-trip-wire.sh` +
   `mise.toml` wiring) — all in this PR (no stacked prereq, §3).
5. `/pre-push` gate (incl. `mise run trip-wires` exercising the new gate) → PR (bundling this plan) →
   `/external-converge` (Codex) → merge → apply §6 landing rows.
