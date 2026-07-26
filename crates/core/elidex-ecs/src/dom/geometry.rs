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
    /// Fragmentainer index this fragment lives in (multicol column), or `None` when
    /// the store carries no fragmentainer for it.
    ///
    /// `Some(n)` is **store-sourced and authoritative**. `None` is the N=1 fallback
    /// arm: the store fragments only the entities it breaks (spanning mid-breaks), so
    /// a **non-spanning** child lying wholly inside a later multicol column has no
    /// store fragment and its column is genuinely unknown here; a non-multicol
    /// element has no fragmentainer at all. Both are honestly `None`.
    ///
    /// This is an `Option`, not a `u32` defaulting to `0`, because a consumer cannot
    /// otherwise tell the two apart: a per-column-keyed reader (C-3c hit-test, C-3d
    /// iframe click-routing — the memo §1 req 1 consumers) would key a later-column
    /// child on a fabricated column `0` with no way to detect it. Making "unknown"
    /// unrepresentable-as-`0` is the same by-construction treatment the phase guard
    /// gets, applied to this axis.
    /// (The `box_model` geometry is correct either way — it is absolute doc-space.)
    pub fragmentainer: Option<u32>,
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
    ///   fragment `(fragmentainer None, From<&LayoutBox>)` — see [`FragmentView::fragmentainer`]
    ///   for why the fallback reports no column rather than `0`. Never routes on
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
        self.collect(entity)
            .iter()
            .map(|fv| fv.box_model.border_box())
            .reduce(|a, b| a.union(&b))
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
                    fragmentainer: Some(node.fragmentainer),
                    box_model: bf.clone(),
                }
            })
            .collect();
        // Order by fragmentainer, ASCENDING — the seam's contract, not the store's.
        // `fragments_for` yields the `index` Vec in `push_box` APPEND order
        // (`fragment_tree.rs`: `index.entry(entity).or_default().push(id)`), i.e.
        // producer call order, NOT column order. `principal_fragment` promises the
        // FIRST-COLUMN box — that being the whole multicol fix this migration exists
        // for, since the raw component holds the G11 last-column box — so leaving the
        // order to producers would make the seam's headline guarantee depend on every
        // current and future producer's loop order, and the documented
        // `remove_entity` + re-push re-commit path can already reorder them. Sorting
        // here makes the guarantee hold by construction and keeps the folds mutually
        // consistent: an order-independent reduction (the union) would otherwise
        // silently disagree with the one that takes `.next()`.
        out.sort_by_key(|fv| fv.fragmentainer);
        if out.is_empty() {
            // N=1 fallback: the single `LayoutBox` as one fragment with NO
            // fragmentainer — the store knows of none, and inventing `0` would be
            // indistinguishable from a real column 0 (see `FragmentView`). A boxless
            // entity has no `LayoutBox` here → stays empty (box-absent).
            if let Ok(lb) = self.dom.world().get::<&LayoutBox>(entity) {
                out.push(FragmentView {
                    fragmentainer: None,
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
    // Not re-exported through `super::*` — only the field-correspondence test needs it.
    use elidex_plugin::EdgeSizes;

    fn layout_box(x: f32, y: f32, w: f32, h: f32) -> LayoutBox {
        LayoutBox {
            content: Rect::new(x, y, w, h),
            ..Default::default()
        }
    }

    /// The store-side fixture. Built through the `From<&LayoutBox>` correspondence
    /// (the seam's single field mapping) rather than re-spelling the fields, so the
    /// two fixtures cannot drift. The conversion itself is pinned **field by field**
    /// against a literal by [`from_layoutbox_maps_every_field`] — which is what makes
    /// routing the fixture through `From` safe: the seam's N=1 arm uses the same
    /// conversion, so without that test both sides of every N=1 assertion would run
    /// through it and a dropped field would be invisible.
    fn box_fragment(x: f32, y: f32, w: f32, h: f32) -> BoxFragment {
        BoxFragment::from(&layout_box(x, y, w, h))
    }

    #[test]
    fn from_layoutbox_maps_every_field() {
        // The ONE non-tautological check of `From<&LayoutBox> for BoxFragment`. Every
        // other N=1 assertion in this module compares `From(lb)` against `From(lb)`, so
        // a conversion that silently dropped `margin` or `first_baseline` would pass
        // them all. Distinct non-default values per field so a mis-wired mapping (e.g.
        // padding copied into border) cannot coincide.
        let lb = LayoutBox {
            content: Rect::new(1.0, 2.0, 3.0, 4.0),
            padding: EdgeSizes::new(5.0, 6.0, 7.0, 8.0),
            border: EdgeSizes::new(9.0, 10.0, 11.0, 12.0),
            margin: EdgeSizes::new(13.0, 14.0, 15.0, 16.0),
            first_baseline: Some(17.0),
            layout_generation: 18,
        };
        let bf = BoxFragment::from(&lb);
        assert_eq!(bf.content, lb.content, "content");
        assert_eq!(bf.padding, lb.padding, "padding");
        assert_eq!(bf.border, lb.border, "border");
        assert_eq!(bf.margin, lb.margin, "margin");
        assert_eq!(bf.first_baseline, lb.first_baseline, "first_baseline");
        // `layout_generation` is deliberately NOT carried: it is a paged-render gate
        // stamp on the component, not fragment geometry (hand-off row 4,
        // `#11-layout-generation-rehome`). `BoxFragment` has no such field.
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
        assert_eq!(
            frags[0].fragmentainer, None,
            "the N=1 fallback has NO fragmentainer — not a fabricated column 0"
        );
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
        // A 2-column mid-break: col 0 at (x=0,y=0,100×50), col 1 at (x=300,y=20,100×60).
        // The DIFFERING y exercises the union's min_y/max_y fold (not just x/right).
        dom.fragment_tree_mut()
            .push_box(e, 0, box_fragment(0.0, 0.0, 100.0, 50.0), false);
        dom.fragment_tree_mut()
            .push_box(e, 1, box_fragment(300.0, 20.0, 100.0, 60.0), false);
        dom.fragment_tree_mut().publish_completed_screen();
        let geom = dom.screen_geometry().unwrap();

        let frags: Vec<_> = geom.box_fragments(e).collect();
        assert_eq!(frags.len(), 2, "both columns yielded (presence-routed)");
        assert_eq!(
            frags.iter().map(|f| f.fragmentainer).collect::<Vec<_>>(),
            vec![Some(0), Some(1)],
            "each carries its own store-sourced fragmentainer id"
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
            "union spans both columns (x: min origin, max right)"
        );
        assert_eq!(
            (u.origin.y, u.bottom()),
            (0.0, 80.0),
            "union spans both columns (y: min 0, max 20+60=80 — exercises min_y/max_y)"
        );
    }

    #[test]
    fn fragments_are_column_ordered_regardless_of_push_order() {
        // Regression pin for the `collect` sort. Verified to FAIL without it:
        // `left: [Some(1), Some(0)]` — `fragments_for` returns the index Vec in
        // `push_box` append order, so `principal_fragment` would hand back the
        // column-1 box while claiming to be "the FIRST-COLUMN box (not the G11
        // last-column box the raw component holds)", which is the exact promise
        // C-3b (clientTop/scrollIntoView) and C-3d (ResizeObserver) migrate onto.
        // Producers happen to commit ascending today; the documented `remove_entity`
        // + re-push re-commit path does not have to, so the guarantee must not rest
        // on producer loop order.
        let mut dom = EcsDom::new();
        let e = spawn(&mut dom);
        dom.fragment_tree_mut()
            .push_box(e, 1, box_fragment(300.0, 20.0, 100.0, 60.0), false);
        dom.fragment_tree_mut()
            .push_box(e, 0, box_fragment(0.0, 0.0, 100.0, 50.0), false);
        dom.fragment_tree_mut().publish_completed_screen();
        let geom = dom.screen_geometry().unwrap();

        assert_eq!(
            geom.box_fragments(e)
                .map(|f| f.fragmentainer)
                .collect::<Vec<_>>(),
            vec![Some(0), Some(1)],
            "yielded in ascending fragmentainer order, not push order"
        );
        assert_eq!(
            geom.principal_fragment(e).unwrap().content.origin.x,
            0.0,
            "principal == the FIRST COLUMN even though column 1 was pushed first"
        );
        // The order-independent fold must agree with the order-dependent one.
        assert_eq!(
            geom.union_border_boxes(e).map(|u| (u.origin.x, u.right())),
            Some((0.0, 400.0)),
            "union is unchanged by ordering — the two folds stay consistent"
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
            frags[0].fragmentainer,
            Some(3),
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
