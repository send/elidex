use super::shadow::VALID_SHADOW_HOST_TAGS;
use super::*;
use crate::components::{
    AttrData, Attributes, CommentData, DocTypeData, NodeKind, ShadowHost, ShadowRootMode,
    SlotAssignment, TextContent, TreeRelation,
};

fn elem(dom: &mut EcsDom, tag: &'static str) -> Entity {
    dom.create_element(tag, Attributes::default())
}

mod associated_document;
mod clone;
mod creation;
mod destroy;
mod node_kind;
mod shadow_dom;
mod traversal;
mod tree_ops;
mod version;
