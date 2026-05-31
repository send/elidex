use super::shadow::VALID_SHADOW_HOST_TAGS;
use super::*;
use crate::components::{
    AttrData, Attributes, CommentData, DocTypeData, Namespace, NodeKind, ShadowHost, ShadowRoot,
    ShadowRootMode, SlotAssignment, SlotAssignmentMode, TextContent, TreeRelation,
};
use crate::dom::shadow::{ShadowAttachError, ShadowInit, SlotAssignError};

fn elem(dom: &mut EcsDom, tag: &'static str) -> Entity {
    dom.create_element(tag, Attributes::default())
}

mod associated_document;
mod clone;
mod creation;
mod destroy;
mod equality;
mod mutation_hook;
mod namespace;
mod node_kind;
mod shadow_dom;
mod traversal;
mod tree_ops;
mod version;
