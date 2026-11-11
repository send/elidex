use super::shadow::VALID_SHADOW_HOST_TAGS;
use super::*;
use crate::components::{
    AttrData, Attributes, CommentData, DocTypeData, Namespace, NodeKind, ShadowHost, ShadowRoot,
    ShadowRootMode, SlotAssignment, SlotAssignmentMode, TextContent, TreeRelation,
};
use crate::dom::shadow::{ShadowAttachError, ShadowInit, SlotAssignError};

fn elem(dom: &mut EcsDom, tag: &'static str) -> Entity {
    dom.create_element_ownerless_fixture(tag, Attributes::default())
}

mod adopt_notification;
mod adopt_on_insert;
mod associated_document;
mod attribute_reconcile;
mod clone;
mod creation;
mod destroy;
mod dispatch_guard;
mod equality;
mod mutation_hook;
mod mutation_hook_fire_helpers;
mod mutation_hook_specialised;
mod namespace;
mod node_kind;
mod shadow_dom;
mod splice_utf16;
mod template_owner;
mod traversal;
mod tree_ops;
mod version;
