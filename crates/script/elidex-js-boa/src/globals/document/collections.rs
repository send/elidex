//! Document collection helpers: `getElementsByClassName`, `getElementsByTagName`,
//! `getElementsByName`, and collection getters (forms, images, links, scripts).

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsValue, NativeFunction};
use elidex_ecs::Entity;

use crate::bridge::HostBridge;

/// Collect all descendant elements matching a class name (space-separated class list).
pub(crate) fn collect_elements_by_class(
    root: Entity,
    class_name: &str,
    dom: &elidex_ecs::EcsDom,
) -> Vec<Entity> {
    let target_classes: Vec<&str> = class_name.split_whitespace().collect();
    if target_classes.is_empty() {
        return Vec::new();
    }
    let mut results = Vec::new();
    walk_descendants(root, dom, &mut |entity| {
        if let Ok(attrs) = dom.world().get::<&elidex_ecs::Attributes>(entity) {
            if let Some(cls) = attrs.get("class") {
                let element_classes: Vec<&str> = cls.split_whitespace().collect();
                if target_classes.iter().all(|tc| element_classes.contains(tc)) {
                    results.push(entity);
                }
            }
        }
    });
    results
}

/// Collect all descendant elements matching a tag name (case-insensitive).
pub(crate) fn collect_elements_by_tag(
    root: Entity,
    tag: &str,
    dom: &elidex_ecs::EcsDom,
) -> Vec<Entity> {
    let tag_lower = tag.to_ascii_lowercase();
    let match_all = tag == "*";
    let mut results = Vec::new();
    walk_descendants(root, dom, &mut |entity| {
        if match_all {
            // "*" matches all elements.
            if dom.world().get::<&elidex_ecs::TagType>(entity).is_ok() {
                results.push(entity);
            }
        } else if let Ok(tt) = dom.world().get::<&elidex_ecs::TagType>(entity) {
            if tt.0.eq_ignore_ascii_case(&tag_lower) {
                results.push(entity);
            }
        }
    });
    results
}

/// Collect all descendant elements with a matching `name` attribute.
pub(super) fn collect_elements_by_name(
    root: Entity,
    name: &str,
    dom: &elidex_ecs::EcsDom,
) -> Vec<Entity> {
    let mut results = Vec::new();
    walk_descendants(root, dom, &mut |entity| {
        if let Ok(attrs) = dom.world().get::<&elidex_ecs::Attributes>(entity) {
            if attrs.get("name").is_some_and(|n| n == name) {
                results.push(entity);
            }
        }
    });
    results
}

/// Pre-order walk of all descendants (excluding root).
pub(super) fn walk_descendants(
    root: Entity,
    dom: &elidex_ecs::EcsDom,
    callback: &mut dyn FnMut(Entity),
) {
    let mut stack = Vec::new();
    // Push children last-to-first so first child is popped first (pre-order).
    let mut child = dom.get_last_child(root);
    while let Some(c) = child {
        stack.push(c);
        child = dom.get_prev_sibling(c);
    }

    while let Some(entity) = stack.pop() {
        callback(entity);
        // Push children last-to-first.
        let mut child = dom.get_last_child(entity);
        while let Some(c) = child {
            stack.push(c);
            child = dom.get_prev_sibling(c);
        }
    }
}

/// Convert a list of entities to a JS array of element wrappers.
pub(crate) fn entities_to_js_array(
    entities: &[Entity],
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsValue {
    let array = boa_engine::object::builtins::JsArray::new(ctx);
    for &entity in entities {
        let wrapper = super::traversal::resolve_entity_to_js(entity, bridge, ctx);
        let _ = array.push(wrapper, ctx);
    }
    array.into()
}

/// Register a document collection getter that returns elements matching a tag name.
pub(super) fn register_collection_getter(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
    js_name: &str,
    tag: &'static str,
) {
    let b = bridge.clone();
    let getter = NativeFunction::from_copy_closure_with_captures(
        move |_this, _args, bridge, ctx| {
            let doc = bridge.document_entity();
            let entities = bridge.with(|_session, dom| collect_elements_by_tag(doc, tag, dom));
            Ok(entities_to_js_array(&entities, bridge, ctx))
        },
        b,
    )
    .to_js_function(realm);
    init.accessor(
        js_string!(js_name),
        Some(getter),
        None,
        Attribute::CONFIGURABLE,
    );
}
