//! Column filling algorithms: sequential and balanced.
//!
//! CSS Multi-column Layout Level 1 §7 (column-fill) and §8 (overflow columns).

use elidex_ecs::{BoxFragment, ColumnFlowSlice, EcsDom, Entity, ImageData, InlineFlowLine};
use elidex_layout_block::block::stack_block_children;
use elidex_layout_block::{
    BreakToken, BreakTokenData, FragmentainerContext, FragmentationType, LayoutEnv, LayoutInput,
};
use elidex_plugin::{ComputedStyle, CssSize, LayoutBox, Overflow, Point, WritingModeContext};

use crate::algo::ColumnGeometry;
use crate::ColumnLayoutCtx;

/// Shared environment for column fill algorithms.
pub struct ColumnFillEnv<'a> {
    pub input: &'a LayoutInput<'a>,
    pub geom: &'a ColumnGeometry,
    pub env: &'a LayoutEnv<'a>,
    pub parent_entity: Entity,
    pub col_ctx: &'a ColumnLayoutCtx,
}

/// Result of laying out content into a single column.
#[derive(Clone, Debug)]
pub struct ColumnFragment {
    /// Entities laid out in this column (direct children).
    pub children: Vec<Entity>,
    /// Per-column snapshots of the **spanning** (mid-break) children laid out in
    /// this column, captured at column-0 base coords **before** the next column's
    /// layout overwrites their `LayoutBox` (the G11 last-column-wins gap).
    /// `position_column_fragments` commits each (definitive-pass-only), offset to
    /// this column's inline position. Empty for a column whose children all fit
    /// whole (those use the shifted `LayoutBox`/`InlineFlow`, I-multicol).
    pub box_snapshots: Vec<FragmentSnapshot>,
}

/// One mid-break spanning child's per-column snapshot: its box-model geometry
/// (committed to the standalone fragment-tree box store; render's fragment-walk
/// consumes it per-column for a `consumable` direct-child IFC mid-break, terminal-Z
/// C-1) **and** the drained IFC line carrier
/// ([`ColumnFlowSlice`]) — this column's per-run-start [`InlineFlowLine`]s at
/// column-0 base, which `position_column_fragments` folds (offset per column) into
/// the run-start's `InlineFlow` (the sink render's `emit_inline_flow` consumes,
/// Z-1b Option D). `flow_groups` is empty unless the multicol's **direct child**
/// IS the IFC container that broke mid-column (the carrier is written on the IFC
/// `parent_entity` and drained here keyed on the spanning direct child — they must
/// coincide). A deeper IFC (the direct child is a block wrapping a separate IFC
/// container) writes its carrier on that inner container, which this drain (keyed
/// on the direct child) never reaches: the carrier leaks (benign — render never
/// reads it) and the inner IFC stays legacy/G11. Likewise a nested-block mid-break
/// (the direct child breaks at a *block-child* boundary, not mid-IFC). Both are
/// committed-next, no regression (these were legacy before Z-1b too).
#[derive(Clone, Debug)]
pub struct FragmentSnapshot {
    /// The spanning (mid-break) child entity (= the IFC container `parent_entity`).
    pub entity: Entity,
    /// Box-model geometry at column-0 base (Z-1a box store, dark).
    pub box_fragment: BoxFragment,
    /// Drained per-run-start IFC lines for this column (Z-1b InlineFlow source).
    pub flow_groups: Vec<(Entity, Vec<InlineFlowLine>)>,
}

/// Snapshot an entity's current `LayoutBox` as a [`BoxFragment`] (box-model
/// geometry minus the component-era `layout_generation`). `None` if the entity
/// has no `LayoutBox` (e.g. it was suppressed) — captured at this column's
/// base coords; the column inline-offset is applied at commit.
fn snapshot_box(dom: &EcsDom, entity: Entity) -> Option<BoxFragment> {
    let lb = dom.world().get::<&LayoutBox>(entity).ok()?;
    Some(BoxFragment::from(&*lb))
}

/// Maximum number of binary search iterations for balanced fill.
const MAX_BALANCE_ITERATIONS: u32 = 10;

/// Maximum number of overflow columns to prevent infinite loops.
const MAX_OVERFLOW_COLUMNS: u32 = 1000;

/// Extract `child_index` from a break token's mode data.
fn extract_child_index(token: Option<&BreakToken>) -> Option<usize> {
    token
        .and_then(|bt| bt.mode_data.as_ref())
        .and_then(|md| match md {
            BreakTokenData::Block { child_index, .. } => Some(*child_index),
            _ => None,
        })
}

/// Fill columns sequentially (column-fill: auto).
///
/// Each column is filled to `column_height` before overflowing to the next.
/// Overflow columns beyond the nominal column-count are created, up to
/// `max_columns` (CSS §8).
///
/// `max_columns` is capped at [`MAX_OVERFLOW_COLUMNS`] (1000) as a safety
/// limit. Pass `u32::MAX` for "unlimited" (overflow columns allowed up to the
/// cap), as the auto path and the balanced *definitive* pass do: `column-count`
/// is a balance *target*, never a hard layout cap, so content that does not fit
/// spills into overflow columns (CSS Multicol L1 §8.2) rather than being
/// dropped. The balanced *search* probe instead passes `count + 1` — the
/// minimal cap that still decides feasibility (`used_columns <= count`).
///
/// Uses `stack_block_children` with `FragmentainerContext` for proper
/// fragmentation. Break tokens are passed between columns so that
/// mid-child breaks (nested content) are correctly resumed.
pub fn fill_columns_sequential(
    dom: &mut EcsDom,
    children: &[Entity],
    env: &ColumnFillEnv<'_>,
    column_height: f32,
    max_columns: u32,
    probe: bool,
) -> Vec<ColumnFragment> {
    let mut fragments = Vec::new();
    let mut col_index: u32 = 0;
    let mut carry_break_token: Option<BreakToken> = None;
    // The child the PREVIOUS column broke mid-way on — it continues into this
    // column, so its per-column box fragment must be snapshotted here too (the
    // `next == prev` mid-break test below only catches the column a child
    // *breaks out of*, not the column it *resumes and finishes in*). `None`
    // when the previous column ended at a clean child boundary.
    let mut carry_midbreak: Option<Entity> = None;

    // Safety limit to prevent infinite loops.
    let total_limit = max_columns.min(MAX_OVERFLOW_COLUMNS);

    loop {
        if col_index >= total_limit {
            break;
        }

        let frag_ctx = FragmentainerContext {
            available_block_size: column_height,
            fragmentation_type: FragmentationType::Column,
        };

        // Always lay out at column 0's position. position_column_fragments
        // in lib.rs shifts columns 1+ to their correct inline offset afterward.
        let col_pos = base_column_position(env.col_ctx);

        let col_input = build_column_input(
            env.input,
            env.geom,
            &frag_ctx,
            col_pos,
            env.col_ctx.wm,
            carry_break_token.as_ref(),
            probe,
        );

        let result = stack_block_children(
            dom,
            children,
            &col_input,
            env.env.layout_child,
            true, // Each column establishes a BFC
            env.parent_entity,
        );

        // Determine which children ended up in this column.
        let next_child_idx =
            extract_child_index(result.break_token.as_ref()).unwrap_or(children.len());
        let prev_child_idx = extract_child_index(carry_break_token.as_ref()).unwrap_or(0);

        // Precise per-column "the break-out child was laid PARTIALLY here" signal
        // (P3): the break token carries a `child_break_token`, which
        // `stack_block_children` sets ONLY when it fragmented a child mid-way. A
        // clean inter-sibling break (`break-before`/`break-after: column`) or a
        // monolithic deferral leaves it `None` and lays the child whole in ONE
        // column (where the I-multicol shifted `LayoutBox` suffices). This single
        // signal drives BOTH the `col_children` shift membership below AND the
        // box-fragment snapshot — converged (Z-1b-0). Using the coarse
        // `result.break_token.is_some()` positional proxy here instead would
        // misclassify a `break-before: column` resume point (where `next == prev`
        // but no child split) as mid-break and additively double-shift the
        // deferred child.
        let break_out_child = result
            .break_token
            .as_ref()
            .filter(|bt| bt.child_break_token.is_some())
            .and_then(|_| children.get(next_child_idx).copied());

        // Build the children list for this column fragment.
        //
        // Mid-child break (next == prev AND the child genuinely split here): the
        // child is fragmented mid-way and only partially laid out in this column.
        // Include it so that position_column_fragments shifts it with this column.
        //
        // When next > prev, children[prev..next] fit here; a child at `next` that
        // broke out belongs to the NEXT column — the ECS model has one LayoutBox
        // per entity, so the next column's layout overwrites it (G11).
        let is_midbreak = next_child_idx == prev_child_idx && break_out_child.is_some();

        let col_children = if is_midbreak {
            // Mid-child break: only this partially-laid-out child.
            vec![children[next_child_idx]]
        } else {
            children[prev_child_idx.min(children.len())..next_child_idx.min(children.len())]
                .to_vec()
        };

        // Snapshot the per-column box fragments of this column's SPANNING children,
        // before the next column overwrites their `LayoutBox` (G11). This column's
        // two spanning slots are the child that breaks OUT of it (`break_out_child`,
        // the precise signal above) and the one that continued INTO it
        // (`carry_midbreak` = the prior column's break-out); they coincide for the
        // middle column of a 3+-column span, so the break-out slot is dropped when
        // it equals the carry. The column inline-offset is applied at commit in
        // `position_column_fragments` (render's C-1 fragment-walk then consumes these
        // per column for a `consumable` direct-child IFC mid-break).
        let mut box_snapshots = Vec::new();
        let dedup_break_out = break_out_child.filter(|&b| Some(b) != carry_midbreak);
        for entity in carry_midbreak.into_iter().chain(dedup_break_out) {
            if let Some(bf) = snapshot_box(dom, entity) {
                // Drain (get + remove) the IFC line carrier this mid-break container
                // wrote for THIS column (the IFC re-ran for this column just above,
                // overwriting any prior column's carrier). `position_column_fragments`
                // folds these per-run-start lines into the run-start's `InlineFlow`,
                // offset to the column's inline position (Z-1b Option D). Drained-only
                // — render never reads `ColumnFlowSlice`; empty for a non-IFC span.
                let flow_groups = dom
                    .world_mut()
                    .remove_one::<ColumnFlowSlice>(entity)
                    .map(|c| c.0)
                    .unwrap_or_default();
                box_snapshots.push(FragmentSnapshot {
                    entity,
                    box_fragment: bf,
                    flow_groups,
                });
            }
        }
        carry_midbreak = break_out_child;

        fragments.push(ColumnFragment {
            children: col_children,
            box_snapshots,
        });

        carry_break_token = result.break_token;
        if carry_break_token.is_none() {
            break;
        }

        col_index += 1;
    }

    fragments
}

/// Fill columns with balanced heights (column-fill: balance).
///
/// Uses binary search to find the minimum column height at which the content
/// fits into `geom.count` columns. `geom.count` is the balance *target* the
/// feasibility test compares against (`used_columns <= count`), never a hard
/// layout cap: when even `max_height` cannot fit the content in `geom.count`
/// columns, the resolved height saturates at `max_height` and the surplus spills
/// into overflow columns in the inline direction (CSS Multicol L1 §8.2) — the
/// content is laid out, never dropped. The search probe is capped at `count + 1`
/// (the minimal cap that decides feasibility); the definitive final pass is
/// uncapped (`u32::MAX`) so it lays out the *full* child set.
///
/// Laying out the *full* child set in the definitive pass is also what
/// reconciles inline state. The unconstrained probe (Step 1) persists a
/// generation-0 `InlineFlow` on each child's IFC run-start at the column-0
/// position. Since I-multicol (#291) a whole-in-column run persists its
/// `InlineFlow`, shifted to its column's inline offset by the column shift, so
/// the definitive pass re-lays-out each child in its column and *overwrites*
/// that probe flow with the real per-column one (the per-run overwrite-safety
/// the balanced path relies on — see `multicol_balanced_persists_one_fragment_per_run`).
/// A child the definitive pass skipped is never overwritten, so its probe flow
/// is stranded at column-0, which render paints as a ghost. The overwrite holds
/// up to the [`MAX_OVERFLOW_COLUMNS`] runaway-safety cap (shared with the auto
/// path): pathological content needing more columns than that is truncated, the
/// same deliberate tradeoff that path already makes.
///
/// Returns `(fragments, resolved_column_height)`.
pub fn fill_columns_balanced(
    dom: &mut EcsDom,
    children: &[Entity],
    env: &ColumnFillEnv<'_>,
    max_height: Option<f32>,
) -> (Vec<ColumnFragment>, f32) {
    if children.is_empty() {
        return (Vec::new(), 0.0);
    }

    // Step 1: Unconstrained probe to get total content height.
    let total_height = probe_total_height(dom, children, env);

    if total_height <= 0.0 {
        return (Vec::new(), 0.0);
    }

    let tallest_monolithic = find_tallest_monolithic(dom, children, env.col_ctx.wm);

    // Step 2: Binary search bounds.
    #[allow(clippy::cast_precision_loss)]
    let mut low = (total_height / env.geom.count as f32)
        .max(tallest_monolithic)
        .max(1.0);
    let mut high = if let Some(max_h) = max_height {
        total_height.min(max_h)
    } else {
        total_height
    };

    if low > high {
        low = high;
    }

    // Step 3: Binary search — converge `high` toward the minimum fitting height.
    //
    // Feasibility of a height `mid` is "does the content fit in `<= count`
    // columns?". The probe is capped at `count + 1` — the minimal cap that
    // answers it: `used_columns <= count` ⇒ feasible (lower `high`); exactly
    // `count + 1` ⇒ needs more than `count` columns, infeasible (raise `low`).
    // Capping at `count` instead would make `used_columns <= count` vacuously
    // true and collapse the search to `low`; capping higher (e.g. `u32::MAX`)
    // just lays out overflow columns the feasibility test ignores. (The
    // definitive final pass below uses `u32::MAX` — it alone must lay out the
    // full child set.)
    let probe_cap = env.geom.count.saturating_add(1);
    for _ in 0..MAX_BALANCE_ITERATIONS {
        if (high - low).abs() < 1.0 {
            break;
        }

        let mid = f32::midpoint(low, high);
        // Search probe = a throwaway feasibility pass; mark probe=true so a nested
        // multicol suppresses its store write (P1).
        let frags = fill_columns_sequential(dom, children, env, mid, probe_cap, true);

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let used_columns = frags.len() as u32;

        if used_columns <= env.geom.count {
            high = mid;
        } else {
            low = mid;
        }
    }

    // Definitive pass at converged `high` to produce the final fragments and
    // ensure children's LayoutBoxes match. Unlike the bounded search probe, this
    // is uncapped (`u32::MAX`) because it must lay out the *full* child set: when
    // `high` (saturated at `max_height`) cannot fit the content in `geom.count`
    // columns, the surplus spills into overflow columns (§8.2) instead of being
    // dropped — which also overwrites every child's column-0 probe `InlineFlow`
    // with its real per-column flow (see the function doc).
    // Definitive pass: probe=false. (It still inherits an ANCESTOR's probe via
    // `env.input.is_probe` inside `build_column_input` — this `false` only says
    // "not a balanced search probe of THIS multicol".)
    let best_frags = fill_columns_sequential(dom, children, env, high, u32::MAX, false);

    (best_frags, high)
}

/// Probe total content height by laying out with infinite available block size.
fn probe_total_height(dom: &mut EcsDom, children: &[Entity], env: &ColumnFillEnv<'_>) -> f32 {
    let is_horizontal = env.col_ctx.wm.is_horizontal();
    let col_pos = base_column_position(env.col_ctx);

    let col_input = LayoutInput {
        containing: CssSize {
            width: if is_horizontal {
                env.geom.width
            } else {
                env.input.containing.width
            },
            height: if is_horizontal {
                None
            } else {
                Some(env.geom.width)
            },
        },
        offset: col_pos,
        viewport: env.input.viewport,
        ..LayoutInput::probe(env.env, env.geom.width)
    };

    let result = stack_block_children(
        dom,
        children,
        &col_input,
        env.env.layout_child,
        true,
        env.parent_entity,
    );
    result.height
}

/// Find the tallest monolithic child (lower bound for balanced column height).
///
/// Returns the largest block-axis extent among monolithic children.
/// Monolithic elements: overflow != visible, replaced elements (`ImageData`).
///
/// Note: absolute/fixed children are already excluded by `split_segments`
/// before reaching this function.
fn find_tallest_monolithic(dom: &EcsDom, children: &[Entity], wm: WritingModeContext) -> f32 {
    let is_horizontal = wm.is_horizontal();
    let mut tallest: f32 = 0.0;
    for &child in children {
        let is_monolithic = dom
            .world()
            .get::<&ComputedStyle>(child)
            .ok()
            .is_some_and(|style| {
                style.overflow_x != Overflow::Visible || style.overflow_y != Overflow::Visible
            })
            || dom.world().get::<&ImageData>(child).is_ok();

        if is_monolithic {
            if let Ok(lb) = dom.world().get::<&LayoutBox>(child) {
                let block_extent = if is_horizontal {
                    lb.content.size.height
                } else {
                    lb.content.size.width
                };
                tallest = tallest.max(block_extent);
            }
        }
    }
    tallest
}

/// Compute the base (column 0) physical position.
///
/// All columns are laid out at this position; `position_column_fragments`
/// in `lib.rs` shifts columns 1+ to their correct inline offset afterward.
fn base_column_position(ctx: &ColumnLayoutCtx) -> Point {
    if ctx.wm.is_horizontal() {
        Point::new(
            ctx.content_origin.x,
            ctx.content_origin.y + ctx.block_offset,
        )
    } else {
        Point::new(
            ctx.content_origin.x + ctx.block_offset,
            ctx.content_origin.y,
        )
    }
}

/// Build a `LayoutInput` for a single column.
///
/// `probe` marks a balanced-fill **search** probe (a throwaway feasibility pass);
/// it is OR'd with the ancestor's `input.is_probe` so a multicol nested in this
/// column inherits the throwaway flag and suppresses its own fragment-tree store
/// write (P1).
fn build_column_input<'a>(
    input: &LayoutInput<'a>,
    geom: &ColumnGeometry,
    frag_ctx: &'a FragmentainerContext,
    col_pos: Point,
    wm: WritingModeContext,
    break_token: Option<&'a BreakToken>,
    probe: bool,
) -> LayoutInput<'a> {
    let is_horizontal = wm.is_horizontal();
    LayoutInput {
        containing: CssSize {
            width: if is_horizontal {
                geom.width
            } else {
                input.containing.width
            },
            // Column box is an independent formatting context (§3.1):
            // percentage heights resolve against the column height, not the parent's.
            height: if is_horizontal {
                Some(frag_ctx.available_block_size)
            } else {
                Some(geom.width)
            },
        },
        containing_inline_size: geom.width,
        offset: col_pos,
        font_db: input.font_db,
        depth: input.depth + 1,
        float_ctx: None, // Each column resets float context (§3)
        viewport: input.viewport,
        fragmentainer: Some(frag_ctx),
        break_token,
        subgrid: None,
        layout_generation: input.layout_generation,
        // Throwaway iff this is a search probe OR an ancestor is already probing
        // — so a nested multicol suppresses its store write (P1).
        is_probe: probe || input.is_probe,
    }
}
