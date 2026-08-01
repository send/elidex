//! Styled run collection — walks the inline subtree gathering text runs and
//! atomic boxes into the ordered [`InlineItem`] list an IFC is packed from.

use elidex_ecs::{EcsDom, Entity, PseudoElementMarker, TextContent};
use elidex_plugin::{ComputedStyle, Display, Position, TextTransform};
use elidex_text::apply_text_transform;

use super::whitespace::collapse_inline_whitespace;
use super::{InlineItem, StyledRun};
use crate::MAX_LAYOUT_DEPTH;

/// Returns true if `display` is an atomic inline-level box that establishes
/// its own formatting context (e.g. `inline-block`, `inline-flex`).
fn is_atomic_inline(display: Display) -> bool {
    matches!(
        display,
        Display::InlineBlock | Display::InlineFlex | Display::InlineGrid | Display::InlineTable
    )
}

/// Render's `run[0]` for the inline run rooted at the element whose composed
/// children are `children` — it MUST match `paint_non_sc` / Layer-5 grouping
/// exactly (`crates/core/elidex-render/src/builder/walk.rs`), since layout persists
/// the group's `InlineFlow` on this entity and render reads it off `run.first()`.
/// Render's run builder: skip `is_positioned` children (`position != static` —
/// relpos/sticky/abspos/fixed, painted in another layer, NO flush); FLUSH (end the
/// run) at the first `is_block_child` (a block-level child SPLITS the run); push
/// **everything else** — so the first non-positioned, non-block child is `run[0]`,
/// **including a `display:none` element or a non-styled node (text/comment)** (both
/// generate no box but render still pushes them as run members). Returns `None` if
/// a block precedes any such child (the pre-block run is empty). Mirroring render's
/// predicate verbatim (NOT a stricter "skip display:none/non-text" filter) is what
/// keeps the persist key and `run[0]` in agreement.
fn first_eligible_child(dom: &EcsDom, children: &[Entity]) -> Option<Entity> {
    for &child in children {
        // Borrowed read (no `ComputedStyle` clone like `try_get_style`) — this scan
        // only reads `position`/`display`, in a per-child loop on the inline-collect path.
        match dom.world().get::<&ComputedStyle>(child).ok() {
            // Mirrors render: `is_positioned` (position != static) → skip without
            // flushing; `is_block_level` (render's `is_block_child`, which the box
            // it gets in-flow satisfies) → flush, ending the pre-block run; anything
            // else (incl. `display:none`) → render's `run[0]`.
            Some(style) => {
                if style.position != Position::Static {
                    continue;
                }
                if crate::block::is_block_level(style.display) {
                    return None;
                }
                return Some(child);
            }
            // A non-styled node (text or comment) — render pushes it into the inline
            // run unconditionally (not positioned, not a block child), so it is a
            // valid `run[0]`.
            None => return Some(child),
        }
    }
    None
}

/// Whether any DIRECT composed child of a positioned inline is block-level. Such a
/// subtree is anonymous-block-in-inline (CSS 2 §9.2.1.1): render `paint_non_sc`
/// SPLITS the run on the block (multiple runs + a separate `walk(block)`), so the
/// single-sub-flow-per-positioned-root model would over-collect and double-paint
/// the post-block content. A positioned subtree like this gets **no** sub-flow
/// (its content falls to render's legacy path, fail-safe — the anonymous-block-in-
/// inline feature owns it). Direct children only: a block nested in a *static*
/// inline within the subtree is flow-consumed (flattened) by render, not split, so
/// it stays safe in the sub-flow.
fn has_direct_block_child(dom: &EcsDom, children: &[Entity]) -> bool {
    children.iter().any(|&c| {
        // Borrowed read (no `ComputedStyle` clone) — only `display` is read here.
        dom.world()
            .get::<&ComputedStyle>(c)
            .is_ok_and(|s| crate::block::is_block_level(s.display))
    })
}

/// The render-run-group key a `position:relative`/`sticky` inline (`child`, the
/// sub-flow's run-parent) persists its members under: a per-subtree sub-flow keyed
/// on the subtree's first eligible child (= render's `run[0]` for `walk(child)`).
/// Returns `None` — no sub-flow, members fall to render's legacy path — when the
/// subtree is **not a single linear inline run render can consume** in the IFC
/// root's writing mode. The single boundary (One-issue-one-way: future cases —
/// float-in-positioned, etc. — land here):
/// - a writing mode differing from the IFC root's → layout projects every group
///   with the root's axis, but render reads a sub-flow's axis off the span (its
///   `emit_inline_run` run-parent), so the sub-flow would be transposed (CSS
///   Writing Modes 4 §3.2 would blockify it to inline-block, which gates anyway).
/// - a direct block child → render splits the run (anonymous-block-in-inline, CSS 2
///   §9.2.1.1); the single-sub-flow model would over-collect and double-paint.
///
/// Both the block-split check and the key use **`composed_children_flat`** — the
/// SAME `display:contents`-flattened child list render's `walk(child)` iterates
/// ([walk.rs]) — NOT the raw `composed_children`. Otherwise a `display:contents`
/// first child (or a block nested inside one) would key/gate differently than
/// render's `run[0]`: a key mismatch silently drops to legacy, and a missed
/// block-in-contents would over-collect into a sub-flow render then splits →
/// double-paint. (Members are still collected by the raw recursion under the
/// returned key — the key is what render reads off, so it must match render.)
fn positioned_subflow_key(
    dom: &EcsDom,
    child: Entity,
    style: &ComputedStyle,
    root_horizontal: bool,
) -> Option<Entity> {
    if style.writing_mode.is_horizontal() != root_horizontal {
        return None;
    }
    let flat = crate::composed_children_flat(dom, child);
    if has_direct_block_child(dom, &flat) {
        return None;
    }
    first_eligible_child(dom, &flat)
}

/// Recursively collect inline items (text runs + atomic boxes) from inline children.
///
/// Text nodes produce a run inheriting the nearest ancestor element's style.
/// Inline elements use their own style for their children. `display: none`
/// elements are skipped. Atomic inline-level boxes (`inline-block`, `inline-flex`, etc.)
/// produce placeholder items — they are laid out separately and placed as
/// atomic units in the inline flow. Recursion stops at [`MAX_LAYOUT_DEPTH`].
///
/// Also reports the **candidate-key set** for staleness reconciliation: a superset
/// of every entity that could carry an `InlineFlow` for this IFC in any pass = the
/// raw (unfiltered) direct children of the IFC parent plus the raw direct children
/// of every inline element recursed into (each is some run-parent's direct child,
/// hence a potential `run[0]`). The caller clears `InlineFlow` on candidates it does
/// not persist (see the reconcile in `layout_inline_context_fragmented`).
///
/// The top-level members are tagged with the **realigned** top-level run-start key
/// ([`first_eligible_child`] of `children` — render's Layer-5 `run[0]`, which is NOT
/// `children.first()` when a leading child is positioned), threaded into the walk as
/// the initial group key; the caller persists each group from the packer's buckets.
pub(crate) fn collect_inline_items(
    dom: &EcsDom,
    children: &[Entity],
    parent_style: &ComputedStyle,
    parent_entity: Entity,
) -> (Vec<InlineItem>, Vec<Entity>, Option<Entity>) {
    let mut items = Vec::new();
    // Candidate keys: seed with the IFC parent's raw direct children, then collect
    // every recursed inline element's raw direct children during the walk.
    let mut candidate_keys: Vec<Entity> = children.to_vec();
    let top_level_key = first_eligible_child(dom, children);
    // The IFC root's writing-mode axis — the projection axis used for every group
    // (gates sub-flows whose positioned root overrides writing-mode; see the relpos
    // branch in `collect_inline_items_inner`).
    let root_horizontal = parent_style.writing_mode.is_horizontal();
    collect_inline_items_inner(
        dom,
        children,
        parent_style,
        parent_entity,
        0,
        &mut items,
        top_level_key,
        &mut candidate_keys,
        root_horizontal,
    );
    collapse_inline_whitespace(&mut items);
    apply_text_transforms(&mut items);
    // `top_level_key` (render's `run[0]`) is the justification target group:
    // `text-align: justify` distributes free space over the top-level run group only
    // (see `FlowAlign::top_level_key`), so it is returned for the caller's `FlowAlign`.
    (items, candidate_keys, top_level_key)
}

/// Apply CSS `text-transform` (CSS Text 3 §2.1) to each text run's collapsed
/// text, in place, *after* §4.1.1 white-space collapse and *before* the line
/// packer measures/breaks it (§2.1.2 Order of Operations). Because the packer
/// reads `run.text` for both break opportunities and width measurement, the
/// transformed text drives line breaking and the persisted glyph positions, and
/// render paints `run.text` verbatim (no re-transform). Each run is transformed
/// independently — §2.1.1's "inline box boundaries must not introduce a word
/// boundary" across runs is a pre-existing gap, matching render's prior
/// per-segment behavior.
fn apply_text_transforms(items: &mut [InlineItem]) {
    for item in items {
        if let InlineItem::Text(run) = item {
            if run.text_transform != TextTransform::None {
                if let std::borrow::Cow::Owned(transformed) =
                    apply_text_transform(&run.text, run.text_transform)
                {
                    run.text = transformed;
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_inline_items_inner(
    dom: &EcsDom,
    children: &[Entity],
    parent_style: &ComputedStyle,
    parent_entity: Entity,
    depth: u32,
    items: &mut Vec<InlineItem>,
    // Render-run-group this level's members persist under (the run-start `run[0]`
    // render reads `InlineFlow` off). `None` = not recorded (positioned subtree
    // with a direct block child → legacy; see `has_direct_block_child`).
    group_key: Option<Entity>,
    // Superset of every entity that could carry an `InlineFlow` for this IFC
    // (every run-parent's raw direct children) — the caller's staleness clear set.
    candidate_keys: &mut Vec<Entity>,
    // Whether the IFC root's writing mode is horizontal (the projection axis the
    // persist block uses for ALL groups). A positioned inline whose writing mode
    // differs gets no sub-flow (render would read it with the wrong axis).
    root_horizontal: bool,
) {
    if depth >= MAX_LAYOUT_DEPTH {
        return;
    }
    for &child in children {
        if let Some(style) = crate::try_get_style(dom, child) {
            if style.display == Display::None {
                continue;
            }
            // CSS 2.1 §9.3.1/§9.6: absolutely positioned elements are removed from flow.
            // Insert a zero-width placeholder to record static position (CSS 2.1 §10.6.5).
            if crate::positioned::is_absolutely_positioned(&style) {
                items.push(InlineItem::Placeholder(child));
                continue;
            }
            // Atomic inline-level box (CSS Display 3 §A `#atomic-inline`):
            // placeholder with zero size (filled later by `layout_atomic_items`).
            // A *static* atomic converges into its group's `InlineFlow` as an
            // `AtomicBox` member — render paints it by `walk()`-ing the entity at
            // its own (repositioned) `LayoutBox`. A *relative/sticky* atomic
            // (`positioned`) is painted in render's Layer 6 from its own `LayoutBox`
            // (CSS 2 §9.4.3 in-flow advance, Layer-6 paint), so it is NOT a flow
            // member (that would double-paint with Layer 6) — `LinePacker` records
            // its on-line position separately and layout repositions its box
            // preserving the relative offset (slice 3p-b-2). The `position` check
            // lives here because this arm `continue`s before the inline-element
            // relpos sub-flow handling below. The static atomic carries the current
            // `group_key` so a static atomic inside a relpos sub-flow becomes that
            // sub-flow's `AtomicBox` member (repositioned per group at persist);
            // `group_key` is ignored for a positioned atomic (never a flow member).
            if is_atomic_inline(style.display) {
                items.push(InlineItem::Atomic {
                    entity: child,
                    inline_size: 0.0,
                    block_size: 0.0,
                    group_key,
                    positioned: matches!(style.position, Position::Relative | Position::Sticky),
                });
                continue;
            }
            // Pseudo-element: use its resolved generated text directly with its
            // own style (skip child recursion). The pre-layout generated-content
            // pass has already resolved `content` (incl. counter()) into the
            // pseudo's `TextContent`, so layout measures the real text. bidi and
            // text-transform no longer gate: the run persists in logical order and
            // render reorders for paint (slice 4) / transform is applied in-place
            // after collapse (no gate).
            if dom.world().get::<&PseudoElementMarker>(child).is_ok() {
                if let Ok(tc) = dom.world().get::<&TextContent>(child) {
                    if !tc.0.is_empty() {
                        items.push(InlineItem::Text(StyledRun::from_style(
                            child,
                            tc.0.clone(),
                            &style,
                            group_key,
                        )));
                    }
                }
                continue;
            }
            // Inline element: use its own style for its children. Every inline
            // element's raw direct children are candidate `InlineFlow` keys (any
            // could become a run-start `run[0]` in some pass) — record before
            // recursing so the caller can clear stale flows on them.
            let grandchildren = dom.composed_children(child);
            candidate_keys.extend_from_slice(&grandchildren);
            // CSS 2 §9.4.3: a relative/sticky positioned inline stays in-flow in the
            // IFC, but render paints its whole subtree in Layer 6 via `walk(child)`.
            // Slice 3p-b converges it as a **sub-flow** keyed on the subtree's first
            // eligible child (the parent flow advances past it, leaving the in-flow
            // gap) — unless the subtree is not single-linear-representable, in which
            // case `positioned_subflow_key` returns `None` and it falls to render's
            // legacy path. A non-positioned inline stays in the enclosing group.
            let child_group = if matches!(style.position, Position::Relative | Position::Sticky) {
                positioned_subflow_key(dom, child, &style, root_horizontal)
            } else {
                group_key
            };
            collect_inline_items_inner(
                dom,
                &grandchildren,
                &style,
                child,
                depth + 1,
                items,
                child_group,
                candidate_keys,
                root_horizontal,
            );
        } else if let Ok(tc) = dom.world().get::<&TextContent>(child) {
            // Text node: produce a run with the parent element's style. Neither bidi
            // nor text-transform gates persistence any more: the run persists in
            // logical order (render reorders RTL runs visually — slice 4) and
            // text-transform is applied in-place after collapse
            // (`apply_text_transforms`, threaded via `StyledRun::text_transform`).
            if !tc.0.is_empty() {
                items.push(InlineItem::Text(StyledRun::from_style(
                    parent_entity,
                    tc.0.clone(),
                    parent_style,
                    group_key,
                )));
            }
        }
    }
}
