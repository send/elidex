//! Element wrapper objects for boa — provides DOM methods on element instances.

pub(crate) mod accessors;
mod attrs_and_content;
pub(crate) mod core;
mod shadow_and_canvas;
pub(crate) mod special_nodes;
pub(crate) mod tree_nav;

pub use core::{create_element_wrapper, extract_entity};
pub use special_nodes::resolve_object_ref;

use boa_engine::object::ObjectInitializer;
use crate::bridge::HostBridge;

/// Hidden property key storing the entity bits on element wrapper objects.
pub(crate) const ENTITY_KEY: &str = "__elidex_entity__";

/// Hidden property key for caching the `style` object on an element wrapper.
const STYLE_CACHE_KEY: &str = "__elidex_style__";

/// Hidden property key for caching the `classList` object on an element wrapper.
const CLASSLIST_CACHE_KEY: &str = "__elidex_classList__";

/// Hidden property key for caching the context2d object on a canvas element.
const CONTEXT2D_CACHE_KEY: &str = "__elidex_ctx2d__";

/// Hidden property key for caching the `dataset` object on an element wrapper.
const DATASET_CACHE_KEY: &str = "__elidex_dataset__";

/// Register all methods/accessors on an element object (called from `build_element_object`).
fn register_all_methods(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    attrs_and_content::register_content_accessors(init, bridge, realm);
    attrs_and_content::register_style_accessor(init, bridge, realm);
    attrs_and_content::register_class_list_accessor(init, bridge, realm);
    attrs_and_content::register_event_listener_methods(init, bridge);
    shadow_and_canvas::register_shadow_dom_methods(init, bridge, realm);
    shadow_and_canvas::register_canvas_method(init, bridge);
    super::element_form::register_form_accessors(init, bridge, realm);
    tree_nav::register_tree_nav_accessors(init, bridge, realm);
    tree_nav::register_node_info_accessors(init, bridge, realm);
    accessors::register_node_methods(init, bridge);
    accessors::register_child_parent_mixin_methods(init, bridge);
    accessors::register_element_extra_methods(init, bridge);
    accessors::register_element_extra_accessors(init, bridge, realm);
    accessors::register_dataset_accessor(init, bridge, realm);
    special_nodes::register_char_data_methods(init, bridge, realm);
    special_nodes::register_attr_node_methods(init, bridge);
}
