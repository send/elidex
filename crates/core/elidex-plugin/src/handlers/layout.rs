//! Built-in [`LayoutModel`] implementations for core layout algorithms.

use crate::{
    Constraints, EdgeSizes, LayoutContext, LayoutModel, LayoutNode, LayoutResult, PluginRegistry,
    Rect,
};

// ---------------------------------------------------------------------------
// BlockLayoutModel
// ---------------------------------------------------------------------------

struct BlockLayoutModel;

impl LayoutModel for BlockLayoutModel {
    fn name(&self) -> &'static str {
        "block"
    }

    fn layout(
        &self,
        _node: &LayoutNode,
        children: &[LayoutNode],
        constraints: &Constraints,
        _ctx: &LayoutContext,
    ) -> LayoutResult {
        stub_result(children.len(), constraints)
    }
}

// ---------------------------------------------------------------------------
// FlexLayoutModel
// ---------------------------------------------------------------------------

struct FlexLayoutModel;

impl LayoutModel for FlexLayoutModel {
    fn name(&self) -> &'static str {
        "flex"
    }

    fn layout(
        &self,
        _node: &LayoutNode,
        children: &[LayoutNode],
        constraints: &Constraints,
        _ctx: &LayoutContext,
    ) -> LayoutResult {
        stub_result(children.len(), constraints)
    }
}

// ---------------------------------------------------------------------------
// GridLayoutModel
// ---------------------------------------------------------------------------

struct GridLayoutModel;

impl LayoutModel for GridLayoutModel {
    fn name(&self) -> &'static str {
        "grid"
    }

    fn layout(
        &self,
        _node: &LayoutNode,
        children: &[LayoutNode],
        constraints: &Constraints,
        _ctx: &LayoutContext,
    ) -> LayoutResult {
        stub_result(children.len(), constraints)
    }
}

// ---------------------------------------------------------------------------
// TableLayoutModel
// ---------------------------------------------------------------------------

struct TableLayoutModel;

impl LayoutModel for TableLayoutModel {
    fn name(&self) -> &'static str {
        "table"
    }

    fn layout(
        &self,
        _node: &LayoutNode,
        children: &[LayoutNode],
        constraints: &Constraints,
        _ctx: &LayoutContext,
    ) -> LayoutResult {
        stub_result(children.len(), constraints)
    }
}

/// Stub layout result — uses available width and 10px per child.
fn stub_result(child_count: usize, constraints: &Constraints) -> LayoutResult {
    let count = u16::try_from(child_count).unwrap_or(u16::MAX);
    LayoutResult {
        bounds: Rect::new(
            0.0,
            0.0,
            constraints.available_width.unwrap_or(0.0),
            f32::from(count) * 10.0,
        ),
        margin: EdgeSizes::default(),
        padding: EdgeSizes::default(),
        border: EdgeSizes::default(),
    }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// Creates a [`PluginRegistry`] pre-populated with built-in layout models.
///
/// Registers models for: `block`, `flex`, `grid`, `table`.
#[must_use]
pub fn create_layout_registry() -> PluginRegistry<dyn LayoutModel> {
    let mut registry: PluginRegistry<dyn LayoutModel> = PluginRegistry::new();
    registry.register_static("block", Box::new(BlockLayoutModel));
    registry.register_static("flex", Box::new(FlexLayoutModel));
    registry.register_static("grid", Box::new(GridLayoutModel));
    registry.register_static("table", Box::new(TableLayoutModel));
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DomSpecLevel, Size};

    fn test_ctx() -> LayoutContext {
        LayoutContext {
            viewport: Size {
                width: 1280.0,
                height: 720.0,
            },
            containing_block: Size {
                width: 320.0,
                height: 200.0,
            },
        }
    }

    #[test]
    fn layout_models_metadata() {
        let block = BlockLayoutModel;
        assert_eq!(block.name(), "block");
        assert_eq!(block.spec_level(), DomSpecLevel::Living);

        let flex = FlexLayoutModel;
        assert_eq!(flex.name(), "flex");

        let grid = GridLayoutModel;
        assert_eq!(grid.name(), "grid");

        let table = TableLayoutModel;
        assert_eq!(table.name(), "table");
        assert_eq!(table.spec_level(), DomSpecLevel::Living);
    }

    #[test]
    fn layout_registry_factory_and_resolve() {
        let registry = create_layout_registry();
        assert_eq!(registry.len(), 4);

        let block = registry.resolve("block").unwrap();
        assert_eq!(block.name(), "block");

        let constraints = Constraints {
            available_width: Some(500.0),
            ..Constraints::default()
        };
        let children = vec![LayoutNode::default(); 3];
        let result = block.layout(&LayoutNode::default(), &children, &constraints, &test_ctx());
        assert_eq!(result.bounds.size.width, 500.0);
        assert_eq!(result.bounds.size.height, 30.0);

        assert!(registry.resolve("unknown").is_none());
    }
}
