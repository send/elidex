//! The terminal-Z C-3 box-geometry seam — a phase-guarded projection of an
//! entity's box geometry as its **sequence of box fragments**, so geometry
//! consumers stop reading the raw [`LayoutBox`](elidex_plugin::LayoutBox)
//! component directly (which C-4 makes a producer-internal detail).
//!
//! The common non-fragmented entity is a **1-fragment** sequence (the single
//! `LayoutBox`); a multicol mid-break entity is **N-fragment** (one per column).
//! Both are yielded as a uniform [`FragmentView`] carrying the fragment's stable
//! `fragmentainer` id.
//!
//! # Two-level guard (plan-memo §1)
//!
//! The store has different authority windows than `LayoutBox` (probe-lag,
//! mid-pass emptiness, page-relative paged coords, teardown-stale index). So a
//! reader gets geometry through a **phase-guarded projection**:
//!
//! 1. [`EcsDom::screen_geometry`] is the gate — it returns [`ScreenGeometry`]
//!    only when the store reflects a COMPLETED SCREEN pass
//!    ([`FragmentTree::is_completed_screen`](crate::FragmentTree::is_completed_screen)).
//!    A phase failure is `None` **here**, a signal distinct from per-entity
//!    box-absence (plan-memo §1 req 3 / §2 I-phase).
//! 2. The projection's folds ([`box_fragments`](ScreenGeometry::box_fragments) et
//!    al.) are the read side. They can only be reached with a `ScreenGeometry` in
//!    hand, so the phase guard **propagates to every fold by construction** — the
//!    fold cannot be called on an unguarded store.
//!
//! `LayoutBox` and `BoxFragment` already both implement
//! [`BoxModel`](elidex_plugin::BoxModel), and `impl From<&LayoutBox> for
//! BoxFragment` is the single field correspondence, so the projection needs zero
//! new type machinery (plan-memo §1).

use elidex_plugin::{BoxModel, LayoutBox, Rect};
use hecs::Entity;

use super::EcsDom;
use crate::fragment_tree::{BoxFragment, FragmentContent};

/// One box fragment of an entity, carrying its stable **`fragmentainer` id**
/// alongside the box-model geometry.
///
/// The id is **yielded, not inferred**: a span that starts in a later column has
/// `fragmentainer != enumeration index`, and a downstream retained-hit (C-3c) /
/// iframe click-routing (C-3d) reader needs the real `(entity, fragmentainer)`
/// key without bypassing the seam (plan-memo §1 req 1). Owned + lightweight
/// (`BoxFragment` is five plain fields), so the N=1 fallback arm and the N>1
/// store arm yield one uniform item type with no borrow/owned iterator split.
#[derive(Clone, Debug, PartialEq)]
pub struct FragmentView {
    /// Fragmentainer index this fragment lives in (multicol column; 0 for the
    /// non-fragmented N=1 box).
    pub fragmentainer: u32,
    /// The box-model geometry for this `(entity, fragmentainer)` fragment.
    pub box_model: BoxFragment,
}

/// A phase-guarded read view of the DOM's box geometry — obtainable **only** via
/// [`EcsDom::screen_geometry`], and only when the fragment store reflects a
/// completed screen pass. Holding one is the structural proof the phase guard
/// passed, so the folds defined on it inherit that guard (plan-memo §1 D1+D2).
///
/// The folds return **doc-space / raw-facet** geometry (plan-memo §2 I-frame),
/// NOT viewport-space — "screen" names the *phase* (post-layout, screen pass),
/// not a coordinate frame. A consumer-local frame (scroll subtraction,
/// offsetParent-relative, …) composes at the reader, in a later slice.
pub struct ScreenGeometry<'a> {
    dom: &'a EcsDom,
}

impl EcsDom {
    /// The phase gate for the box-geometry seam (plan-memo §1/§2).
    ///
    /// Returns `Some(ScreenGeometry)` iff the fragment store reflects a COMPLETED
    /// SCREEN layout pass; `None` otherwise (mid-pass, re-entrant, probe, paged /
    /// print, or never laid). That `None` is a signal **distinct** from a valid
    /// projection reporting a boxless entity — the phase failure fails the whole
    /// view here (checked once, since the phase is store-global), whereas
    /// box-absence is a per-entity empty result *within* a valid view.
    #[must_use]
    pub fn screen_geometry(&self) -> Option<ScreenGeometry<'_>> {
        self.fragment_tree()
            .is_completed_screen()
            .then_some(ScreenGeometry { dom: self })
    }
}

impl ScreenGeometry<'_> {
    /// The projection primitive: an entity's box geometry as its sequence of box
    /// fragments, each carrying its `fragmentainer` id (plan-memo §1).
    ///
    /// - **Router = presence** (plan-memo §2 I-router): store-authoritative when the
    ///   entity has fragments; otherwise the single `LayoutBox` component as one
    ///   fragment `(fragmentainer 0, From<&LayoutBox>)`. Never routes on
    ///   `LayoutBox`-absence, never on `is_consumable` (a paint-only signal).
    /// - **Box-absence** (empty result) is a mechanical store fact: the entity has
    ///   neither store fragments nor a `LayoutBox` (plan-memo §1 req 5) — the seam
    ///   reports the store faithfully and adds no "has an associated CSS box"
    ///   predicate (that is a downstream slice's, per audit axis 3).
    /// - **Liveness** (plan-memo §2 I-phase fact 4): a despawned entity whose stale
    ///   `FragmentTree` index entry survives reads **empty by construction** — the
    ///   `world.contains` check runs *before* the store is trusted, so no phantom.
    pub fn box_fragments(&self, entity: Entity) -> impl Iterator<Item = FragmentView> {
        self.collect(entity).into_iter()
    }

    /// The **first** fragment (or the single N=1 box); box-absent → `None`
    /// (plan-memo §1). At N=1 this is byte-identical to the `LayoutBox`; at N>1 it
    /// is the **first-column** box (not the G11 last-column box the raw component
    /// holds), which is the multicol fix the migration exists for.
    #[must_use]
    pub fn principal_fragment(&self, entity: Entity) -> Option<BoxFragment> {
        self.collect(entity)
            .into_iter()
            .next()
            .map(|fv| fv.box_model)
    }

    /// The **plain axis-aligned union** of the fragment border boxes; box-absent →
    /// `None`. A generic per-entity building block — deliberately **NOT** the
    /// CSSOM-View "get the bounding box" 4-step reduction (`cssom-view-1 §6`, which
    /// drops rects with zero width *or* height and returns-first when all are degenerate);
    /// a downstream slice builds that spec-shaped reduction *on* this fold rather
    /// than reusing it (plan-memo §1/§4).
    #[must_use]
    pub fn union_border_boxes(&self, entity: Entity) -> Option<Rect> {
        let frags = self.collect(entity);
        let mut boxes = frags.iter().map(|fv| fv.box_model.border_box());
        let first = boxes.next()?;
        let (mut min_x, mut min_y) = (first.origin.x, first.origin.y);
        let (mut max_x, mut max_y) = (first.right(), first.bottom());
        for r in boxes {
            min_x = min_x.min(r.origin.x);
            min_y = min_y.min(r.origin.y);
            max_x = max_x.max(r.right());
            max_y = max_y.max(r.bottom());
        }
        Some(Rect::new(min_x, min_y, max_x - min_x, max_y - min_y))
    }

    /// Shared collection for the primitive + folds. Applies the liveness guard,
    /// routes on fragment presence, and yields owned [`FragmentView`]s (cloning the
    /// store node's `BoxFragment`, or synthesizing the N=1 box from the component).
    fn collect(&self, entity: Entity) -> Vec<FragmentView> {
        // Liveness first (plan-memo §2 fact 4): a despawned entity reads empty even
        // if its stale fragment index entry survives teardown.
        if !self.dom.contains(entity) {
            return Vec::new();
        }
        let mut out: Vec<FragmentView> = self
            .dom
            .fragment_tree()
            .fragments_for(entity)
            .map(|node| {
                let FragmentContent::Box(bf) = &node.content;
                FragmentView {
                    fragmentainer: node.fragmentainer,
                    box_model: bf.clone(),
                }
            })
            .collect();
        if out.is_empty() {
            // N=1 fallback: the single `LayoutBox` as one fragment in column 0. A
            // boxless entity has no `LayoutBox` here → stays empty (box-absent).
            if let Ok(lb) = self.dom.world().get::<&LayoutBox>(entity) {
                out.push(FragmentView {
                    fragmentainer: 0,
                    box_model: BoxFragment::from(&*lb),
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EcsDom;
    use elidex_plugin::{EdgeSizes, Point, Rect, Size};

    fn layout_box(x: f32, y: f32, w: f32, h: f32) -> LayoutBox {
        LayoutBox {
            content: Rect::new(x, y, w, h),
            padding: EdgeSizes::default(),
            border: EdgeSizes::default(),
            margin: EdgeSizes::default(),
            first_baseline: None,
            layout_generation: 0,
        }
    }

    fn box_fragment(x: f32, y: f32, w: f32, h: f32) -> BoxFragment {
        BoxFragment {
            content: Rect::from_origin_size(Point::new(x, y), Size::new(w, h)),
            padding: EdgeSizes::default(),
            border: EdgeSizes::default(),
            margin: EdgeSizes::default(),
            first_baseline: None,
        }
    }

    /// Spawn a bare entity (no DOM tree wiring needed for a geometry read).
    fn spawn(dom: &mut EcsDom) -> Entity {
        dom.world_mut().spawn(())
    }

    #[test]
    fn phase_gate_is_distinct_from_box_absence() {
        let mut dom = EcsDom::new();
        let e = spawn(&mut dom);
        dom.world_mut()
            .insert_one(e, layout_box(0.0, 0.0, 10.0, 10.0))
            .unwrap();

        // Phase Invalid (default) ⇒ the whole view is None at the gate — NOT an
        // empty per-entity result.
        assert!(
            dom.screen_geometry().is_none(),
            "a non-completed store yields None at the gate"
        );

        // A boxless entity, spawned before the view borrows `dom` immutably.
        let boxless = spawn(&mut dom);

        // Publish a completed screen pass ⇒ the view opens, and box-absence is now a
        // per-entity fact *inside* it (a different, observable call result).
        dom.fragment_tree_mut().publish_completed_screen();
        let geom = dom.screen_geometry().expect("completed screen ⇒ Some");
        assert_eq!(geom.box_fragments(e).count(), 1, "e has its LayoutBox");
        assert_eq!(
            geom.box_fragments(boxless).count(),
            0,
            "box-absent is empty WITHIN a valid view — not a gate failure"
        );
    }

    #[test]
    fn n1_is_behavior_neutral_with_the_layoutbox() {
        let mut dom = EcsDom::new();
        let e = spawn(&mut dom);
        let lb = layout_box(5.0, 7.0, 100.0, 50.0);
        dom.world_mut().insert_one(e, lb.clone()).unwrap();
        dom.fragment_tree_mut().publish_completed_screen();
        let geom = dom.screen_geometry().unwrap();

        let frags: Vec<_> = geom.box_fragments(e).collect();
        assert_eq!(frags.len(), 1, "non-fragmented ⇒ exactly one fragment");
        assert_eq!(frags[0].fragmentainer, 0, "the N=1 box is column 0");
        assert_eq!(
            frags[0].box_model,
            BoxFragment::from(&lb),
            "bit-for-bit From<&LayoutBox>"
        );
        assert_eq!(
            geom.principal_fragment(e),
            Some(BoxFragment::from(&lb)),
            "principal == the one box"
        );
        assert_eq!(
            geom.union_border_boxes(e),
            Some(lb.border_box()),
            "union of one == that box's border box"
        );
    }

    #[test]
    fn n_gt_1_yields_all_columns_with_their_own_fragmentainer_ids() {
        let mut dom = EcsDom::new();
        let e = spawn(&mut dom);
        // A 2-column mid-break: first column at x=0, second at x=300.
        dom.fragment_tree_mut()
            .push_box(e, 0, box_fragment(0.0, 0.0, 100.0, 50.0), false);
        dom.fragment_tree_mut()
            .push_box(e, 1, box_fragment(300.0, 0.0, 100.0, 50.0), false);
        dom.fragment_tree_mut().publish_completed_screen();
        let geom = dom.screen_geometry().unwrap();

        let frags: Vec<_> = geom.box_fragments(e).collect();
        assert_eq!(frags.len(), 2, "both columns yielded (presence-routed)");
        assert_eq!(
            frags.iter().map(|f| f.fragmentainer).collect::<Vec<_>>(),
            vec![0, 1],
            "each carries its own fragmentainer id"
        );
        assert_eq!(
            geom.principal_fragment(e).unwrap().content.origin.x,
            0.0,
            "principal == FIRST column (not the G11 last-column box)"
        );
        let u = geom.union_border_boxes(e).unwrap();
        assert_eq!(
            (u.origin.x, u.right()),
            (0.0, 400.0),
            "union spans both columns"
        );
    }

    #[test]
    fn fragmentainer_id_is_yielded_not_inferred() {
        // A span whose only fragment lives in a LATER column: the id must be the
        // stored `fragmentainer` (3), not the enumeration index (0).
        let mut dom = EcsDom::new();
        let e = spawn(&mut dom);
        dom.fragment_tree_mut()
            .push_box(e, 3, box_fragment(900.0, 0.0, 100.0, 50.0), false);
        dom.fragment_tree_mut().publish_completed_screen();
        let geom = dom.screen_geometry().unwrap();

        let frags: Vec<_> = geom.box_fragments(e).collect();
        assert_eq!(frags.len(), 1);
        assert_eq!(
            frags[0].fragmentainer, 3,
            "yielded from the store node, not inferred from position"
        );
    }

    #[test]
    fn despawned_entity_reads_empty_despite_a_stale_index() {
        // Teardown-stale (plan-memo §2 fact 4): despawn removes the ECS entity but
        // NOT the FragmentTree index entry. The liveness guard makes it read empty.
        let mut dom = EcsDom::new();
        let e = spawn(&mut dom);
        dom.fragment_tree_mut()
            .push_box(e, 0, box_fragment(0.0, 0.0, 10.0, 10.0), false);
        dom.fragment_tree_mut().publish_completed_screen();
        // Sanity: while live, it reads its fragment.
        assert_eq!(dom.screen_geometry().unwrap().box_fragments(e).count(), 1);

        dom.world_mut().despawn(e).unwrap();
        // The stale index entry survives (teardown does not clean the store)...
        assert_eq!(dom.fragment_tree().fragments_for(e).count(), 1);
        // ...but the seam reads empty by construction (contains() is false).
        assert_eq!(
            dom.screen_geometry().unwrap().box_fragments(e).count(),
            0,
            "despawned ⇒ empty, no phantom fragment"
        );
    }

    #[test]
    fn boxless_entity_folds_to_none() {
        let mut dom = EcsDom::new();
        let e = spawn(&mut dom); // no LayoutBox, no fragments
        dom.fragment_tree_mut().publish_completed_screen();
        let geom = dom.screen_geometry().unwrap();
        assert_eq!(geom.box_fragments(e).count(), 0);
        assert_eq!(geom.principal_fragment(e), None);
        assert_eq!(geom.union_border_boxes(e), None);
    }
}
