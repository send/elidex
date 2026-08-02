//! Persisted inline-layout geometry components ([`InlineFlow`] and friends).
//!
//! The layout->render handoff for inline text: one `InlineFlow` per render-run
//! group within an inline formatting context, its per-fragmentainer
//! [`InlineFragment`]s, the [`InlineFlowLine`]s inside them, the
//! [`InlineFlowRun`] payload, and the multicol [`ColumnFlowSlice`]. Split out of
//! `components.rs` for the same reason as the sibling `inline_style` module — to
//! keep the shared component-definition bucket under the 1000-line limit — and on
//! the same seam the file already grouped them by.

use super::{Entity, Point};

/// Persisted collapsed + positioned inline runs for one anonymous inline
/// formatting context (CSS 2 §9.2.1.1), keyed on the run-start entity.
///
/// Produced once by layout (`elidex-layout-block`'s `LinePacker`), consumed by
/// render's display-list builder — the single source of inline-text geometry
/// (One-issue-one-way: render no longer re-collects / re-collapses / re-measures
/// / re-positions the DOM text). Stored on the first top-level child of the
/// inline run (`run[0]`), the same entity both passes derive as the run start.
///
/// Lives in `elidex-ecs` (not `elidex-plugin`, where `LayoutBox` lives) because
/// `InlineFlowRun` references the style-owning `Entity` and `elidex-plugin` does
/// not depend on `elidex-ecs`. The referenced entities are same-`EcsDom` DOM
/// entities (not per-VM identity handles), so intra-world references are sound.
///
/// Coordinates are stored along the **inline** and **block** axes, but layout
/// applies the writing-mode projection (the same `is_vertical` rule as
/// `static_positions` / inline `LayoutBox`es) at persist, so each scalar already
/// holds the **absolute physical coordinate for its axis**: for horizontal,
/// `inline_start` = physical x and `block_start` = physical y; for vertical,
/// `inline_start` = physical y and `block_start` = physical x. Render therefore
/// reads them without a coordinate transform, selecting the right field per writing
/// mode (no vertical-rl block-axis reversal — matching the box convention).
///
/// **Fragmentation (slice 4 / I).** A run that spans fragmentainers (paged-media
/// pages; columns, once I-multicol lands) carries one [`InlineFragment`] per
/// fragmentainer it spans — the inline portion of the project's fragment tree.
/// A 1-entity → N-fragment relation fits a component, unlike the N:M
/// entity↔layer relation that forces the layer tree into a standalone structure
/// (`docs/design/en/15-rendering-pipeline.md` §15.4.1 "Layer Tree as Independent
/// Structure"). A non-fragmented run has a single fragment at generation `0`. The
/// entity→fragments relation is 1:N (still one component on the run-start entity).
#[derive(Debug, Clone, PartialEq)]
pub struct InlineFlow {
    /// One entry per fragmentainer this IFC-run spans (a single entry for the
    /// non-fragmented common case). Render paints the fragment(s) whose
    /// [`generation`](InlineFragment::generation) matches the page being walked
    /// (`expected_generation`), all of them off the paged path.
    pub fragments: Vec<InlineFragment>,
}

impl InlineFlow {
    /// A flow with a single [`InlineFragment`] — the common case: one
    /// fragmentainer, or one paged page's per-page slice (paged writes the Vec at
    /// length-1-per-page, replacing it each page's full re-layout). `generation`
    /// is the fragmentainer discriminator (paged page number; `0` otherwise).
    #[must_use]
    pub fn single(generation: u32, lines: Vec<InlineFlowLine>) -> Self {
        Self {
            fragments: vec![InlineFragment { generation, lines }],
        }
    }
}

/// One fragmentainer's worth of an [`InlineFlow`] — the per-(IFC-run,
/// fragmentainer) collapsed + positioned line set. The inline node of the
/// project's fragment tree (the 1:N entity→fragment relation that, unlike the
/// N:M layer tree of `docs/design/en/15-rendering-pipeline.md` §15.4.1, fits an
/// ECS component).
#[derive(Debug, Clone, PartialEq)]
pub struct InlineFragment {
    /// Fragmentainer discriminator, consumed exactly like render's per-page
    /// `expected_generation` gate: the paged-media page number, or `0` for
    /// non-paged content (a non-fragmented run, or — once I-multicol lands —
    /// multicol columns, which coexist on one surface at absolute coords). Off the
    /// paged path staleness is reconciled by layout explicitly removing the
    /// [`InlineFlow`] component, not by generation comparison.
    pub generation: u32,
    /// This fragment's line boxes in block order, continuation-rebased so the
    /// first kept line sits at the fragmentainer's block-start (absolute,
    /// already-projected coords — render reads them without a transform).
    pub lines: Vec<InlineFlowLine>,
}

/// One positioned line box within an [`InlineFragment`].
#[derive(Debug, Clone, PartialEq)]
pub struct InlineFlowLine {
    /// Absolute block-axis offset of this line box's block-start edge — physical y
    /// (line top) for horizontal, physical x (column block-start edge) for vertical.
    pub block_start: f32,
    /// Line box block size (CSS 2 §10.8 line height calculations). Horizontal render
    /// places each run's baseline at `block_start + ascent` (the leading-naive
    /// legacy behaviour) and does not yet read this (a later slice distributes
    /// half-leading, CSS 2 §10.8.1). Vertical render **does** consume it: the glyph
    /// column center is `block_start + block_size / 2`.
    pub block_size: f32,
    /// Logical-order paintable members on this line ([`InlineFlowRun::Text`] runs
    /// and [`InlineFlowRun::AtomicBox`] inline-level boxes, interleaved in order).
    pub runs: Vec<InlineFlowRun>,
    /// `text-align: justify` within-run extra advance: render's `place_glyphs` adds
    /// this once per within-run `is_word_separator` cluster (CSS Text 3 §6.4). `0.0`
    /// for every non-justified line (the common case). Layout bakes the *between-run*
    /// expansion into each run's `inline_start` separately, so the two are disjoint;
    /// the layout/render split rationale lives on the producer (`pack::bake_justify`).
    pub justify_word_spacing: f32,
}

/// One paintable member of an [`InlineFlowLine`], stored in logical order.
///
/// A [`Text`](InlineFlowRun::Text) run is shaped and emitted at its `inline_start`;
/// an [`AtomicBox`](InlineFlowRun::AtomicBox) is painted by `walk()`-ing the entity
/// at its own (absolute) `LayoutBox`. The members are *stored* in logical order, but
/// render's current consumer (`emit_inline_flow`) does not yet paint them in a single
/// interleaved pass: it emits all `Text` runs first, then `walk()`s the collected
/// `AtomicBox` entities after the `InlineFlow` read borrow drops (`walk()` needs
/// `&mut` access). In the common case the members occupy disjoint positions, so the
/// order is not visually observable; where they DO overlap (e.g. negative margins
/// pulling an atomic box over adjacent text), the text-before-atomic order changes
/// which paints on top. A future bidi pass (slice 4) that needs strict visual order
/// would replace this with a single interleaved walk.
#[derive(Debug, Clone, PartialEq)]
pub enum InlineFlowRun {
    /// A contiguous same-style collapsed text run.
    Text {
        /// Element/pseudo entity whose `ComputedStyle` paints this run (render
        /// re-reads colour / font / decoration / transform / opacity / spacing
        /// from it — layout owns geometry, render owns paint-time style).
        entity: Entity,
        /// Collapsed text (CSS Text 3 §4.1.1 Phase I), this line, this style-run.
        text: String,
        /// Absolute inline-axis start, `text-align` already applied — physical x
        /// for horizontal, physical y (pen top) for vertical.
        inline_start: f32,
    },
    /// A static (non-positioned) atomic inline-level box (CSS Display 3 §A
    /// `#atomic-inline`: `inline-block`/`-flex`/`-grid`/`-table` — an inline-level
    /// box that establishes its own formatting context and cannot split across
    /// lines). Render paints it by `walk()`-ing the entity at its **own** absolute
    /// `LayoutBox`, which layout repositions to `inline_start` (this line's
    /// `block_start`) at persist — so render reads the box, not this field. The
    /// box is the single source of the rendered rect (size + padding/border/margin
    /// live only there); `inline_start` records the text-align-baked inline
    /// position layout used to place the box (parallel to [`Text`](Self::Text)).
    AtomicBox {
        /// The atomic inline-level element whose `LayoutBox` holds its geometry.
        entity: Entity,
        /// Absolute inline-axis start, `text-align` already applied — the position
        /// layout repositioned the atomic's `LayoutBox` to (render paints via the
        /// box, so it does not re-read this).
        inline_start: f32,
    },
}

impl InlineFlowRun {
    /// The member's style/paint entity, common to both variants.
    #[must_use]
    pub fn entity(&self) -> Entity {
        match self {
            Self::Text { entity, .. } | Self::AtomicBox { entity, .. } => *entity,
        }
    }

    /// The member's absolute inline-axis start, common to both variants.
    #[must_use]
    pub fn inline_start(&self) -> f32 {
        match self {
            Self::Text { inline_start, .. } | Self::AtomicBox { inline_start, .. } => *inline_start,
        }
    }

    /// Mutable access to the run's inline-axis start, common to both variants
    /// (used by layout to fold IFC-local → absolute and bake the `text-align`
    /// offset uniformly across text and atomic members).
    pub fn inline_start_mut(&mut self) -> &mut f32 {
        match self {
            Self::Text { inline_start, .. } | Self::AtomicBox { inline_start, .. } => inline_start,
        }
    }

    /// The collapsed text of a [`Text`](Self::Text) member, or `None` for an
    /// [`AtomicBox`](Self::AtomicBox) (which carries no text — its content is
    /// painted by walking the box).
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text { text, .. } => Some(text),
            Self::AtomicBox { .. } => None,
        }
    }
}

/// Transient per-column IFC-line carrier for a multicol **mid-break** inline
/// formatting context (Z-1b, Option D — `memory/terminal-z-z1b-consume-delta.md`).
///
/// Written by `layout_inline_context_fragmented` on the **IFC container** entity
/// (the multicol direct child that breaks mid-column) for the
/// `frag_is_column && !column_is_whole` case, carrying — **per run-start group**
/// (the same `group_key` the converged [`InlineFlow`] persist uses) — this
/// column's folded [`InlineFlowLine`]s at column-0 base coords. Multicol fill
/// **drains** it (get + remove) into the column's fragment snapshot, and
/// `position_column_fragments` folds each column's lines (offset to the column's
/// inline position) into the run-start's `InlineFlow::single` — the sink the
/// existing `emit_inline_flow` consumes.
///
/// **Never read by render** — it lives only between the IFC layout (write) and the
/// multicol fill (drain) within one layout pass (transport, not state), so it is a
/// component (per-entity, `Send + Sync`, not a per-VM identity handle — the
/// side-store→component rule), not a side-store. A stray write that is never
/// drained is benign (render never reads it); the IFC reconciles it insert-or-
/// remove each pass, mirroring `clear_inline_flows`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColumnFlowSlice {
    /// Per-run-start folded lines for this column (Z-1b Option D). The sink
    /// `position_column_fragments` builds each run-start's `InlineFlow` from.
    pub flow_groups: Vec<(Entity, Vec<InlineFlowLine>)>,
    /// Mid-break atomics to reposition at the multicol seam (terminal-Z C-2), as
    /// `(entity, inline_abs, block_abs, unoffset_origin)` — the on-line target (IFC-
    /// absolute physical coords at **column-0 base**) plus the reposition delta basis
    /// (the un-offset margin-box origin `layout_atomic_items` returned, which differs
    /// from the box origin under an asymmetric writing mode, so it cannot be
    /// reconstructed at the seam). `position_column_fragments` adds the column's
    /// inline offset to `inline_abs` and moves each atomic's `LayoutBox` there,
    /// preserving any baked relative offset (basis = un-offset origin).
    ///
    /// Holds BOTH **static** atomics (also `AtomicBox` flow members in `flow_groups`,
    /// but their box is repositioned via this record so the seam needs no second walk
    /// of the runs) and **relpos/sticky** atomics (NOT flow members — render Layer 6
    /// paints the positioned box, so a member would double-paint). One uniform record
    /// for both: the seam moves every atomic the same way, regardless of kind.
    pub atomic_repositions: Vec<(Entity, f32, f32, Point)>,
}
