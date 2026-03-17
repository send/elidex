use super::*;
use crate::components::{
    AttrData, Attributes, CommentData, DocTypeData, NodeKind, ShadowRootMode, SlotAssignment,
    TextContent,
};

fn elem(dom: &mut EcsDom, tag: &'static str) -> Entity {
    dom.create_element(tag, Attributes::default())
}

mod creation;
mod destroy;
mod node_kind;
mod shadow_dom;
mod tree_ops;
mod version;
