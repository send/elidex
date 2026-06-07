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
//! **Scope (Z-1a):** the tree is populated with multicol mid-break **box**
//! fragments only, as *dark data* — render does not yet consume it. Z-1b adds
//! the per-column inline-line fold ([`FragmentContent::InlineLines`], when it
//! lands) and the render consume; the committed-next program adds block / flex /
//! grid / table box fragments + the entity-less line / anonymous-block nodes and
//! makes render walk the tree as primary. The node types here are the Z-final
//! shape (a tree of fragments); Z-1a populates it flat (no nesting yet).

use elidex_plugin::{EdgeSizes, Rect};
use hecs::Entity;

/// Index of a node in a [`FragmentTree`]'s arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FragmentId(pub u32);

/// The standalone fragment tree: an arena of [`FragmentNode`]s plus the ids of
/// the root nodes. Cleared and rebuilt each layout pass (full-from-root
/// relayout is the reconcile — no incremental / staleness model).
#[derive(Clone, Debug, Default)]
pub struct FragmentTree {
    nodes: Vec<FragmentNode>,
    roots: Vec<FragmentId>,
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
    /// What this node carries.
    pub content: FragmentContent,
}

/// What a [`FragmentNode`] carries. Z-1a needs only [`Box`](Self::Box); the
/// per-column IFC line fold (`InlineLines`) arrives in Z-1b and the block / flex
/// / grid / table specializations in the committed-next program — variants are
/// *added*, never reshaping the existing one.
#[derive(Clone, Debug)]
pub enum FragmentContent {
    /// A box fragment: this entity's box model for this fragmentainer, in
    /// **absolute** coords (already column-offset, so render reads it without a
    /// transform).
    ///
    /// Caveat for the Z-1b render-consume integration: these coords are committed
    /// at the multicol container's frame and are **not** re-shifted by an
    /// ancestor's `shift_descendants` (which moves `LayoutBox`/`InlineFlow` but
    /// does not walk this tree). For a multicol nested inside a later-shifted
    /// subtree (outer multicol column shift, margin-collapse, relpos), the
    /// fragment would need shifting too — harmless while the tree is dark data
    /// (Z-1a), but a prerequisite to settle when Z-1b makes it painted.
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
        self.roots.clear();
    }

    /// `true` if the tree has no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Push a root box fragment for `entity` in fragmentainer `fragmentainer`,
    /// returning its id. Z-1a fragments are flat roots (no parenting yet).
    #[allow(clippy::cast_possible_truncation)]
    pub fn push_box(
        &mut self,
        entity: Entity,
        fragmentainer: u32,
        box_fragment: BoxFragment,
    ) -> FragmentId {
        let id = FragmentId(self.nodes.len() as u32);
        self.nodes.push(FragmentNode {
            id,
            parent: None,
            children: Vec::new(),
            entity: Some(entity),
            fragmentainer,
            content: FragmentContent::Box(box_fragment),
        });
        self.roots.push(id);
        id
    }

    /// All box fragments for `entity` (one per fragmentainer it spans), in
    /// insertion (fragmentainer) order. Empty for a non-fragmented / non-store
    /// entity — the **positive presence** of a fragment is the render router
    /// (Z-1b), never `LayoutBox`-absence.
    ///
    /// This is an O(nodes) scan, fine for the layout-side queries and tests that
    /// use it today. The Z-1b render consume must NOT call this per entity inside
    /// the paint walk (that would make the walk O(entities × fragments)) — it
    /// should iterate [`nodes`](Self::nodes) once, or build an entity→fragment
    /// index, when the consumer lands.
    pub fn fragments_for(&self, entity: Entity) -> impl Iterator<Item = &FragmentNode> {
        self.nodes.iter().filter(move |n| n.entity == Some(entity))
    }

    /// All nodes (the committed-next render-walk entry; Z-1a tests read it).
    #[must_use]
    pub fn nodes(&self) -> &[FragmentNode] {
        &self.nodes
    }

    /// The root node ids (every node in Z-1a, since the tree is flat).
    #[must_use]
    pub fn roots(&self) -> &[FragmentId] {
        &self.roots
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

        let id = tree.push_box(e, 1, box_at(300.0));

        assert!(!tree.is_empty());
        assert_eq!(tree.roots(), &[id], "Z-1a fragments are flat roots");
        let node = &tree.nodes()[0];
        assert_eq!(node.id, id);
        assert_eq!(node.parent, None, "flat: no parent in Z-1a");
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
        tree.push_box(span, 0, box_at(0.0));
        tree.push_box(other, 0, box_at(0.0)); // interleaved unrelated node
        tree.push_box(span, 1, box_at(300.0));

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
        tree.push_box(e, 0, box_at(0.0));
        tree.push_box(e, 1, box_at(300.0));
        assert_eq!(tree.nodes().len(), 2);

        tree.clear();

        assert!(tree.is_empty());
        assert!(tree.roots().is_empty());
        assert_eq!(tree.fragments_for(e).count(), 0);
    }
}
