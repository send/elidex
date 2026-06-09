//! The standalone fragment tree — a layout-output structure separate from the
//! ECS `World` (§15.4.1 "Layer Tree as Independent Structure",
//! `docs/design/en/15-rendering-pipeline.md`).
//!
//! It is **not** an [`EcsDom`](crate::EcsDom) component because the
//! entity↔fragment relationship is **N:M**: one entity (a multicol-spanning
//! box) produces N box fragments (one per column it spans), and — once the
//! committed-next program generalizes this — anonymous line and block boxes are
//! fragments with **no** entity at all. The N:M relation does not fit the ECS
//! "one component per entity" model (§15.4.1, the same reason the layer tree is
//! standalone), so the fragment tree is a sibling field of [`EcsDom`], built by
//! layout as output and read by render.
//!
//! **Scope (Z-1a / Z-1b / C-1):** the tree is populated with multicol mid-break
//! **box** fragments (Z-1b folds the per-column inline **text** into the run-start's
//! `InlineFlow` component — the converged `emit_inline_flow` sink; see
//! `memory/terminal-z-z1b-consume-delta.md`, Option D). **Render now consumes this
//! store** (terminal-Z C-1, the first render consumer): for an entity flagged
//! [`is_consumable`](FragmentTree::is_consumable) (a direct-child IFC mid-break) the
//! paint walk reads its per-column box fragments and emits per-column chrome + clip +
//! content. Still *dark data* (committed-next): the per-column inline-line fold
//! ([`FragmentContent::InlineLines`], when it lands), block / flex / grid / table box
//! fragments, the entity-less line / anonymous-block nodes, and making render walk the
//! tree as **primary** (C-1 consumes it as a per-entity router, not yet the walk root).
//! The node types here are the Z-final shape (a tree of fragments); Z-1a populates it
//! flat (no nesting yet).

use std::collections::HashMap;

use elidex_plugin::{BoxModel, EdgeSizes, Rect, Vector};
use hecs::Entity;

/// Index of a node in a [`FragmentTree`]'s arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FragmentId(pub u32);

/// The standalone fragment tree: an arena of [`FragmentNode`]s. Cleared and
/// rebuilt each layout pass (full-from-root relayout is the reconcile — no
/// incremental / staleness model). Root nodes are those with no
/// [`parent`](FragmentNode::parent); Z-1a populates only flat roots.
#[derive(Clone, Debug, Default)]
pub struct FragmentTree {
    nodes: Vec<FragmentNode>,
    /// Entity → box-fragment ids index (D-Z7). Keyed on **box-root** entities
    /// only (`entity == Some(e)` nodes inserted via [`push_box`](Self::push_box));
    /// anonymous line/anon-block nodes (`entity: None`, arriving with the
    /// committed-next inline fold) are **not** indexed — they are reached via
    /// their box parent's [`children`](FragmentNode::children) arena link. Makes
    /// the render router / [`shift_entity`](Self::shift_entity) / `fragments_for`
    /// O(1)-per-entity (the Z-1a `fragments_for` O(nodes) scan the paint walk was
    /// forbidden from calling).
    index: HashMap<Entity, Vec<FragmentId>>,
}

/// One node of the [`FragmentTree`]. The fields are the Z-final tree shape;
/// Z-1a populates box fragments **flat** (`parent: None`, `children: []`, every
/// node a root), with nesting (line / anonymous-block children) added by the
/// committed-next program.
#[derive(Clone, Debug)]
pub struct FragmentNode {
    /// This node's id (its index in the arena).
    pub id: FragmentId,
    /// Parent node, or `None` for a root. Flat (always `None`) in Z-1a.
    pub parent: Option<FragmentId>,
    /// Child nodes in order. Empty in Z-1a.
    pub children: Vec<FragmentId>,
    /// The DOM entity this fragment realizes. `None` for anonymous fragments
    /// (line boxes, anonymous block boxes) — the entity-less half of the N:M
    /// relation, arriving with the committed-next generalization. Z-1a box
    /// fragments are always `Some`.
    pub entity: Option<Entity>,
    /// Fragmentainer index this fragment lives in (multicol column index; 0 for
    /// the first column). Unifies with paged media's page-number generation
    /// when paged folds into the store (committed-next).
    pub fragmentainer: u32,
    /// Whether render consumes this fragment's entity **per-fragment** (the C-1
    /// fragment-walk router signal): set iff this box's mid-break IFC lines were
    /// drained into a per-column `InlineFlow` carrier (`flow_groups` non-empty) at
    /// [`push_box`](FragmentTree::push_box) time — a **direct-child IFC mid-break**,
    /// the one category whose per-column chrome + clip + content render must emit
    /// (css-break-3 §5.4, css-multicol-1 §8.1). A **nested-block / deeper-IFC**
    /// mid-break writes box geometry but no carrier ⇒ `false` ⇒ render's single
    /// `LayoutBox` arm (G11). Box-fragment *presence* alone is NOT the signal — the
    /// carrier-drain is. Stored **per node** (not a per-entity side-set) so an upsert
    /// re-lay that replaces this `(entity, fragmentainer)` with a non-consumable
    /// snapshot updates the flag in place; [`is_consumable`](FragmentTree::is_consumable)
    /// ORs over the entity's *current* nodes, so it cannot go stale.
    pub consumable: bool,
    /// What this node carries.
    pub content: FragmentContent,
}

/// What a [`FragmentNode`] carries. Z-1a / Z-1b need only [`Box`](Self::Box); the
/// per-column IFC line fold (`InlineLines`) and the block / flex / grid / table
/// specializations arrive in the committed-next program (the first render consumer
/// of this store) — variants are *added*, never reshaping the existing one.
#[derive(Clone, Debug)]
pub enum FragmentContent {
    /// A box fragment: this entity's box model for this fragmentainer, in
    /// **absolute** coords (already column-offset, so render reads it without a
    /// transform).
    ///
    /// Ancestor reposition (Z-1b-0, P2): an ancestor subtree shift
    /// (`shift_descendants` — relpos / margin-collapse / an outer multicol's
    /// column shift) moves these coords too, via
    /// [`shift_entity`](FragmentTree::shift_entity), so a multicol nested inside a
    /// later-shifted subtree stays absolute-correct. The multicol's OWN
    /// column-positioning shift does NOT move them (they are born-absolute, offset
    /// baked at commit) — it uses the fragment-excluding shifter.
    Box(BoxFragment),
}

/// Box-model geometry for one `(entity, fragmentainer)` box fragment — the
/// box-model fields of [`elidex_plugin::LayoutBox`] minus its component-era
/// `layout_generation` (the node's [`fragmentainer`](FragmentNode::fragmentainer)
/// replaces it).
#[derive(Clone, Debug)]
pub struct BoxFragment {
    /// Content area (absolute coords).
    pub content: Rect,
    /// Padding widths.
    pub padding: EdgeSizes,
    /// Border widths.
    pub border: EdgeSizes,
    /// Margin widths.
    pub margin: EdgeSizes,
    /// First baseline offset from the content-box top edge (`None` if none).
    pub first_baseline: Option<f32>,
}

impl BoxModel for BoxFragment {
    fn content(&self) -> Rect {
        self.content
    }
    fn padding(&self) -> EdgeSizes {
        self.padding
    }
    fn border(&self) -> EdgeSizes {
        self.border
    }
    fn margin(&self) -> EdgeSizes {
        self.margin
    }
}

impl From<&elidex_plugin::LayoutBox> for BoxFragment {
    /// Project a [`LayoutBox`](elidex_plugin::LayoutBox) to its box-fragment
    /// geometry, dropping the component-era `layout_generation` (the node's
    /// `fragmentainer` discriminates instead). The single source of the
    /// `LayoutBox`↔`BoxFragment` field correspondence — a new box-model field on
    /// `LayoutBox` surfaces here.
    fn from(lb: &elidex_plugin::LayoutBox) -> Self {
        Self {
            content: lb.content,
            padding: lb.padding,
            border: lb.border,
            margin: lb.margin,
            first_baseline: lb.first_baseline,
        }
    }
}

impl FragmentTree {
    /// Remove all nodes — called at the start of each layout pass (the tree is
    /// rebuilt from scratch every pass; full-from-root relayout is the reconcile).
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.index.clear();
    }

    /// `true` if the tree has no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Upsert a root box fragment for `entity` in fragmentainer `fragmentainer`,
    /// returning its id. Z-1a fragments are flat roots (no parenting yet).
    ///
    /// **Upsert, not append** (last definitive write wins, keyed on
    /// `(entity, fragmentainer)`): a subtree can be laid out more than once within
    /// a single layout pass — a multicol child attempted in one column then
    /// deferred and re-laid in the next, a table cell's height-probe + final
    /// relayout, a flex item's measure + reposition — and each definitive lay
    /// re-commits the same `(entity, fragmentainer)` fragment. Replacing rather
    /// than appending keeps exactly one fragment per `(entity, fragmentainer)` at
    /// its final position, mirroring how the `InlineFlow`/`LayoutBox` components
    /// use `insert_one` (replace). (Throwaway *probe* re-lays are suppressed
    /// upstream by `is_probe`; this dedups the unavoidable definitive re-lays.)
    ///
    /// `consumable` records (per node) whether render consumes this `(entity,
    /// fragmentainer)` fragment **per-fragment** — `true` iff this box's mid-break IFC
    /// lines were drained into a per-column carrier (`flow_groups` non-empty = a
    /// direct-child IFC mid-break), `false` for nested-block / deeper-IFC mid-break
    /// (box geometry only, no per-column content). See
    /// [`is_consumable`](Self::is_consumable). Stored on the node: an upsert re-lay
    /// that replaces this `(entity, fragmentainer)` with a non-consumable snapshot
    /// **updates the flag in place** (no stale per-entity latch), and `is_consumable`
    /// ORs over the entity's current nodes.
    #[allow(clippy::cast_possible_truncation)]
    pub fn push_box(
        &mut self,
        entity: Entity,
        fragmentainer: u32,
        box_fragment: BoxFragment,
        consumable: bool,
    ) -> FragmentId {
        // Replace an existing (entity, fragmentainer) node if one is present — the
        // `consumable` flag is overwritten alongside the geometry, so a re-lay that
        // drops the carrier flips the node to non-consumable (no stale latch).
        if let Some(ids) = self.index.get(&entity) {
            for &id in ids {
                if self.nodes[id.0 as usize].fragmentainer == fragmentainer {
                    self.nodes[id.0 as usize].content = FragmentContent::Box(box_fragment);
                    self.nodes[id.0 as usize].consumable = consumable;
                    return id;
                }
            }
        }
        let id = FragmentId(self.nodes.len() as u32);
        self.nodes.push(FragmentNode {
            id,
            parent: None,
            children: Vec::new(),
            entity: Some(entity),
            fragmentainer,
            consumable,
            content: FragmentContent::Box(box_fragment),
        });
        self.index.entry(entity).or_default().push(id);
        id
    }

    /// All box fragments for `entity` (one per fragmentainer it spans), in
    /// insertion (fragmentainer) order. Empty for a non-fragmented / non-store
    /// entity — the **positive presence** of a fragment is the render router
    /// (Z-1b), never `LayoutBox`-absence.
    ///
    /// O(1)-keyed via the D-Z7 entity index (box-roots only), so the committed-next
    /// render router may call it per entity inside the paint walk. Anonymous
    /// `InlineLines` child nodes are NOT returned here — they are reached via a
    /// box node's [`children`](FragmentNode::children) arena link.
    pub fn fragments_for(&self, entity: Entity) -> impl Iterator<Item = &FragmentNode> {
        self.index
            .get(&entity)
            .into_iter()
            .flatten()
            .map(move |id| &self.nodes[id.0 as usize])
    }

    /// Whether render consumes `entity`'s box fragments **per-fragment** (the C-1
    /// fragment-walk router signal). `true` iff a [`push_box`](Self::push_box) for
    /// this entity passed `consumable = true` (its mid-break IFC lines were drained
    /// into a per-column carrier — a direct-child IFC mid-break). `false` for a
    /// non-store entity, or a nested-block / deeper-IFC mid-break whose box fragments
    /// exist but carry no per-column content (those ride render's single `LayoutBox`
    /// arm). `consumable ⟹ fragments_for non-empty` (the flag lives on a node, only
    /// set alongside a push). Reads the entity's **current** nodes, so a re-lay that
    /// replaced a carrier snapshot with a non-consumable one is reflected at once — no
    /// stale latch; the store is also cleared each pass.
    #[must_use]
    pub fn is_consumable(&self, entity: Entity) -> bool {
        self.fragments_for(entity).any(|n| n.consumable)
    }

    /// Shift all box fragments of `entity` by physical `delta` (P2 — keep store
    /// coords absolute-correct after an ancestor subtree shift). Mirrors the
    /// `LayoutBox`/`InlineFlow` arms of block layout's `shift_descendants`, called
    /// per visited entity; O(1) per entity via the index. A box origin shifts by
    /// the raw physical `delta` (a [`Rect`] origin is physical, exactly like
    /// `LayoutBox.content.origin`). The anonymous `InlineLines` child nodes (the
    /// committed-next inline fold) are reached via the box node's `children` arena
    /// link and shift with the writing-mode-projected delta — added with that
    /// variant; Z-1a / Z-1b has box nodes only.
    pub fn shift_entity(&mut self, entity: Entity, delta: Vector) {
        let Some(ids) = self.index.get(&entity) else {
            return;
        };
        // The id list is tiny (one box per spanned column); clone it so we can
        // mutate `self.nodes` without holding the `self.index` borrow. Every
        // indexed id is a box root (the index keys box-roots only), so the match
        // is irrefutable today; the committed-next `InlineLines` variant turns this
        // into a match whose lines arm applies the WM-projected delta (those nodes
        // are reached via `children`, not the index).
        for id in ids.clone() {
            let FragmentContent::Box(bf) = &mut self.nodes[id.0 as usize].content;
            bf.content.origin += delta;
        }
    }

    /// All nodes (the committed-next render-walk entry; Z-1a tests read it).
    /// Root nodes are those whose [`parent`](FragmentNode::parent) is `None`.
    #[must_use]
    pub fn nodes(&self) -> &[FragmentNode] {
        &self.nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::{EdgeSizes, Point, Rect, Size};

    fn box_at(x: f32) -> BoxFragment {
        BoxFragment {
            content: Rect::from_origin_size(Point::new(x, 0.0), Size::new(100.0, 50.0)),
            padding: EdgeSizes::default(),
            border: EdgeSizes::default(),
            margin: EdgeSizes::default(),
            first_baseline: None,
        }
    }

    fn entity(world: &mut hecs::World) -> Entity {
        world.spawn(())
    }

    #[test]
    fn push_box_is_a_flat_root_with_fragmentainer_stamp() {
        let mut w = hecs::World::new();
        let e = entity(&mut w);
        let mut tree = FragmentTree::default();
        assert!(tree.is_empty());

        let id = tree.push_box(e, 1, box_at(300.0), false);

        assert!(!tree.is_empty());
        assert_eq!(tree.nodes().len(), 1);
        let node = &tree.nodes()[0];
        assert_eq!(node.id, id);
        assert_eq!(
            node.parent, None,
            "Z-1a fragments are flat roots (no parent)"
        );
        assert!(node.children.is_empty(), "flat: no children in Z-1a");
        assert_eq!(node.entity, Some(e));
        assert_eq!(node.fragmentainer, 1);
        let FragmentContent::Box(bf) = &node.content;
        assert_eq!(bf.content.origin.x, 300.0);
    }

    #[test]
    fn fragments_for_returns_all_columns_of_one_entity_in_order() {
        let mut w = hecs::World::new();
        let span = entity(&mut w);
        let other = entity(&mut w);
        let mut tree = FragmentTree::default();
        // A spanning entity with one fragment per column it crosses.
        tree.push_box(span, 0, box_at(0.0), false);
        tree.push_box(other, 0, box_at(0.0), false); // interleaved unrelated node
        tree.push_box(span, 1, box_at(300.0), false);

        let cols: Vec<u32> = tree.fragments_for(span).map(|n| n.fragmentainer).collect();
        assert_eq!(
            cols,
            vec![0, 1],
            "all of span's fragments, in insertion order"
        );
        assert_eq!(
            tree.fragments_for(other).count(),
            1,
            "fragments_for is entity-scoped"
        );
        let unrelated = entity(&mut w);
        assert_eq!(
            tree.fragments_for(unrelated).count(),
            0,
            "absent entity ⇒ no fragments (the positive-presence render router signal)"
        );
    }

    #[test]
    fn clear_empties_the_tree_for_the_next_pass() {
        let mut w = hecs::World::new();
        let e = entity(&mut w);
        let mut tree = FragmentTree::default();
        tree.push_box(e, 0, box_at(0.0), false);
        tree.push_box(e, 1, box_at(300.0), false);
        assert_eq!(tree.nodes().len(), 2);

        tree.clear();

        assert!(tree.is_empty());
        assert_eq!(tree.nodes().len(), 0);
        assert_eq!(tree.fragments_for(e).count(), 0);
        // The D-Z7 index is cleared alongside the arena (else a stale id would
        // dangle into the rebuilt arena next pass).
        let again = entity(&mut w);
        tree.push_box(again, 0, box_at(0.0), false);
        assert_eq!(tree.fragments_for(again).count(), 1);
        assert_eq!(
            tree.fragments_for(e).count(),
            0,
            "old entity gone after clear"
        );
    }

    #[test]
    fn shift_entity_moves_all_box_fragments_of_one_entity() {
        use elidex_plugin::Vector;
        let mut w = hecs::World::new();
        let span = entity(&mut w);
        let other = entity(&mut w);
        let mut tree = FragmentTree::default();
        tree.push_box(span, 0, box_at(0.0), false);
        tree.push_box(other, 0, box_at(10.0), false);
        tree.push_box(span, 1, box_at(300.0), false);

        tree.shift_entity(span, Vector::new(5.0, 7.0));

        let xs: Vec<f32> = tree
            .fragments_for(span)
            .map(|n| {
                let FragmentContent::Box(bf) = &n.content;
                (bf.content.origin.x, bf.content.origin.y)
            })
            .map(|(x, _)| x)
            .collect();
        assert_eq!(
            xs,
            vec![5.0, 305.0],
            "both of span's fragments shifted by +5 x"
        );
        let ys: Vec<f32> = tree
            .fragments_for(span)
            .map(|n| {
                let FragmentContent::Box(bf) = &n.content;
                bf.content.origin.y
            })
            .collect();
        assert_eq!(ys, vec![7.0, 7.0], "both shifted by +7 y");
        // The unrelated entity is untouched (index is entity-scoped).
        let FragmentContent::Box(o) = &tree.fragments_for(other).next().unwrap().content;
        assert_eq!((o.content.origin.x, o.content.origin.y), (10.0, 0.0));
    }

    #[test]
    fn shift_entity_absent_is_noop() {
        use elidex_plugin::Vector;
        let mut w = hecs::World::new();
        let e = entity(&mut w);
        let absent = entity(&mut w);
        let mut tree = FragmentTree::default();
        tree.push_box(e, 0, box_at(0.0), false);
        tree.shift_entity(absent, Vector::new(5.0, 5.0));
        let FragmentContent::Box(bf) = &tree.fragments_for(e).next().unwrap().content;
        assert_eq!((bf.content.origin.x, bf.content.origin.y), (0.0, 0.0));
    }

    #[test]
    fn consumable_flag_is_recorded_per_entity_and_latches_or() {
        let mut w = hecs::World::new();
        let ifc = entity(&mut w); // direct-child IFC mid-break (consumable)
        let nested = entity(&mut w); // nested-block mid-break (box only)
        let mut tree = FragmentTree::default();

        // The IFC span: one column without a carrier (false) then one with (true).
        // The flag must LATCH to true (OR across the entity's per-column pushes).
        tree.push_box(ifc, 0, box_at(0.0), false);
        tree.push_box(ifc, 1, box_at(300.0), true);
        // The nested-block span: box fragments but never a carrier ⇒ NOT consumable.
        tree.push_box(nested, 0, box_at(0.0), false);
        tree.push_box(nested, 1, box_at(300.0), false);

        assert!(
            tree.is_consumable(ifc),
            "any column with a drained carrier ⇒ entity is consumable (OR latch)"
        );
        assert!(
            !tree.is_consumable(nested),
            "box-fragment presence without a carrier ⇒ NOT consumable (rides LayoutBox arm)"
        );
        let absent = entity(&mut w);
        assert!(
            !tree.is_consumable(absent),
            "non-store entity ⇒ not consumable"
        );
    }

    #[test]
    fn clear_resets_the_consumable_set() {
        let mut w = hecs::World::new();
        let e = entity(&mut w);
        let mut tree = FragmentTree::default();
        tree.push_box(e, 0, box_at(0.0), true);
        assert!(tree.is_consumable(e));

        tree.clear();

        assert!(
            !tree.is_consumable(e),
            "consumable set is cleared with the arena each pass (no staleness)"
        );
    }

    #[test]
    fn upsert_relay_flips_consumable_in_place_no_stale_latch() {
        // Codex PR#321 R1 (F5): a definitive re-lay can replace an (entity,
        // fragmentainer) node within one pass — a mid-break child attempted in one
        // column then re-laid whole. The per-node flag must be OVERWRITTEN on the
        // upsert, so an entity that drops its carrier flips to non-consumable. A
        // per-entity OR-latch side-set could not un-latch and would leave it stale.
        let mut w = hecs::World::new();
        let e = entity(&mut w);
        let mut tree = FragmentTree::default();

        // First definitive lay: this column carriers ⇒ consumable.
        tree.push_box(e, 0, box_at(0.0), true);
        assert!(tree.is_consumable(e));

        // Re-lay replaces (e, 0) with a non-carrier snapshot ⇒ must flip to false.
        tree.push_box(e, 0, box_at(0.0), false);
        assert_eq!(tree.fragments_for(e).count(), 1, "upsert, not append");
        assert!(
            !tree.is_consumable(e),
            "the upsert overwrites the node's consumable flag in place — no stale latch"
        );
    }
}
