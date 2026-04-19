//! Document host globals — `document.getElementById`,
//! `document.createElement`, `document.createTextNode`, and the
//! getters for `body` / `head` / `documentElement` / `title` / `URL` /
//! `readyState` (WHATWG DOM §4.5 + HTML §3.2.9, §7.1, §12.2.8).
//!
//! # Scope
//!
//! - `getElementById(id)` — pre-order DFS from the document root
//!   (WHATWG DOM §4.2.4 "document descendants").  Uses
//!   `EcsDom::find_by_id`.
//! - `createElement(tag)` / `createTextNode(data)` — allocate ECS
//!   entities; the Element form returns a freshly-cached wrapper,
//!   the Text form returns a host object keyed on the text entity
//!   (no text-specific wrapper methods yet — PR4c lands textContent,
//!   splitText, etc.).
//! - `body` / `head` / `documentElement` — tree walk from the
//!   document root looking for the first `<html>` child, then within
//!   that for `<head>` / `<body>`.  Phase 2 returns `null` when the
//!   structure is missing rather than synthesising fallback nodes.
//! - `title` (get) — concatenates text children of the first
//!   `<title>` element descending from `<head>`; setter lands in PR4c
//!   alongside the rest of the attribute-manipulation surface.
//! - `URL` / `documentURI` — `VmInner::navigation.current_url`.
//! - `readyState` — stub returning `"complete"` (the VM has no notion
//!   of document loading state yet; the shell owns that in PR6).

#![cfg(feature = "engine")]

use super::super::value::{JsValue, NativeContext, ObjectId, VmError};
use super::super::Vm;
use super::dom_bridge::{
    coerce_first_arg_to_string, parse_dom_selector, query_selector_in_subtree_all,
    query_selector_in_subtree_first, wrap_entities_as_array, wrap_entity_or_null,
};

use elidex_ecs::{Attributes, Entity, TagType};

// ---------------------------------------------------------------------------
// Tree walk from the document root.
// ---------------------------------------------------------------------------

/// Locate the `<html>` root child of the bound document.  Returns
/// `None` if there is no document entity or the document has no
/// `<html>` child (e.g. empty tree).
fn find_html_root(ctx: &mut NativeContext<'_>) -> Option<Entity> {
    let doc = ctx.vm.host_data.as_deref()?.document_entity_opt()?;
    let dom = ctx.host().dom();
    dom.first_child_with_tag(doc, "html")
}

// ---------------------------------------------------------------------------
// getElementById
// ---------------------------------------------------------------------------

pub(super) fn native_document_get_element_by_id(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }

    let target = coerce_first_arg_to_string(ctx, args)?;

    let matched: Option<Entity> = {
        let doc = ctx
            .vm
            .host_data
            .as_deref()
            .and_then(|hd| hd.document_entity_opt());
        let dom = ctx.host().dom();
        doc.and_then(|d| dom.find_by_id(d, &target))
    };
    Ok(wrap_entity_or_null(ctx.vm, matched))
}

// ---------------------------------------------------------------------------
// querySelector / querySelectorAll
// ---------------------------------------------------------------------------

pub(super) fn native_document_query_selector(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }

    let selector_str = coerce_first_arg_to_string(ctx, args)?;
    let selectors = parse_dom_selector(&selector_str, "querySelector")?;

    let matched: Option<Entity> = {
        let doc = ctx
            .vm
            .host_data
            .as_deref()
            .and_then(|hd| hd.document_entity_opt());
        let dom = ctx.host().dom();
        doc.and_then(|d| query_selector_in_subtree_first(dom, d, &selectors))
    };
    Ok(wrap_entity_or_null(ctx.vm, matched))
}

pub(super) fn native_document_query_selector_all(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }

    let selector_str = coerce_first_arg_to_string(ctx, args)?;
    let selectors = parse_dom_selector(&selector_str, "querySelectorAll")?;

    // Phase 1: collect entities while DOM is borrowed.
    let entities: Vec<Entity> = {
        let doc = ctx
            .vm
            .host_data
            .as_deref()
            .and_then(|hd| hd.document_entity_opt());
        let dom = ctx.host().dom();
        match doc {
            Some(d) => query_selector_in_subtree_all(dom, d, &selectors),
            None => Vec::new(),
        }
    };

    Ok(wrap_entities_as_array(ctx.vm, &entities))
}

// ---------------------------------------------------------------------------
// getElementsByTagName / getElementsByClassName
// ---------------------------------------------------------------------------

pub(super) fn native_document_get_elements_by_tag_name(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }

    let tag = coerce_first_arg_to_string(ctx, args)?;
    let match_all = tag == "*";
    let entities: Vec<Entity> = {
        let doc = ctx
            .vm
            .host_data
            .as_deref()
            .and_then(|hd| hd.document_entity_opt());
        let dom = ctx.host().dom();
        match doc {
            Some(d) => {
                let mut result = Vec::new();
                dom.traverse_descendants(d, |entity| {
                    if match_all {
                        if dom.world().get::<&TagType>(entity).is_ok() {
                            result.push(entity);
                        }
                    } else if let Ok(tt) = dom.world().get::<&TagType>(entity) {
                        if tt.0.eq_ignore_ascii_case(&tag) {
                            result.push(entity);
                        }
                    }
                    true
                });
                result
            }
            None => Vec::new(),
        }
    };

    Ok(wrap_entities_as_array(ctx.vm, &entities))
}

pub(super) fn native_document_get_elements_by_class_name(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }

    let class_str = coerce_first_arg_to_string(ctx, args)?;

    let target_classes: Vec<&str> = class_str.split_whitespace().collect();
    if target_classes.is_empty() {
        return Ok(wrap_entities_as_array(ctx.vm, &[]));
    }

    let entities: Vec<Entity> = {
        let doc = ctx
            .vm
            .host_data
            .as_deref()
            .and_then(|hd| hd.document_entity_opt());
        let dom = ctx.host().dom();
        match doc {
            Some(d) => {
                let mut result = Vec::new();
                dom.traverse_descendants(d, |entity| {
                    if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
                        if let Some(cls) = attrs.get("class") {
                            if target_classes
                                .iter()
                                .all(|tc| cls.split_whitespace().any(|ec| ec == *tc))
                            {
                                result.push(entity);
                            }
                        }
                    }
                    true
                });
                result
            }
            None => Vec::new(),
        }
    };

    Ok(wrap_entities_as_array(ctx.vm, &entities))
}

// ---------------------------------------------------------------------------
// createElement / createTextNode
// ---------------------------------------------------------------------------
//
// Unbound callers get `null` (not a throw) to match the no-op
// behaviour of `getElementById` / `body` / `head` etc.  In practice
// the only way to reach these natives while unbound is to retain a
// `document` reference across an `unbind()` boundary; throwing
// TypeError would crash the listener instead of producing the same
// "document is detached" semantics that other methods already surface.

pub(super) fn native_document_create_element(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }

    // WHATWG DOM §4.5 createElement normalises to lowercase in the
    // "HTML document" branch.  We treat every bind as HTML.
    let tag = coerce_first_arg_to_string(ctx, args)?.to_ascii_lowercase();

    let new_entity = {
        let dom = ctx.host().dom();
        dom.create_element(tag, Attributes::default())
    };
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(new_entity)))
}

pub(super) fn native_document_create_text_node(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }

    let data = coerce_first_arg_to_string(ctx, args)?;

    let new_entity = {
        let dom = ctx.host().dom();
        dom.create_text(data)
    };
    // Text nodes share the same HostObject wrapper surface as
    // elements — the prototype chain climbs through
    // `EventTarget.prototype` either way.  PR4c will install
    // text-specific own-properties (`data`, `length`, `splitText`,
    // etc.) on a dedicated `Text.prototype`; until then the wrapper
    // simply identifies the underlying entity.
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(new_entity)))
}

/// `document.createComment(data)` — WHATWG DOM §4.5.  Allocates a
/// Comment entity with the coerced `data` string and returns its
/// wrapper.  Unbound receiver → `null` (matches the rest of the
/// document method family).
pub(super) fn native_document_create_comment(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let data = coerce_first_arg_to_string(ctx, args)?;
    let new_entity = ctx.host().dom().create_comment(data);
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(new_entity)))
}

/// `document.createDocumentFragment()` — WHATWG DOM §4.5.  Allocates
/// a `DocumentFragment` entity that is **not** linked into any tree
/// and returns its wrapper.  Unbound receiver → `null`.
pub(super) fn native_document_create_document_fragment(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let new_entity = ctx.host().dom().create_document_fragment();
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(new_entity)))
}

// ---------------------------------------------------------------------------
// Getters: documentElement / head / body / title / URL / readyState
// ---------------------------------------------------------------------------

pub(super) fn native_document_get_document_element(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let html = find_html_root(ctx);
    Ok(wrap_entity_or_null(ctx.vm, html))
}

pub(super) fn native_document_get_head(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let head = find_html_root(ctx).and_then(|html| {
        let dom = ctx.host().dom();
        dom.first_child_with_tag(html, "head")
    });
    Ok(wrap_entity_or_null(ctx.vm, head))
}

pub(super) fn native_document_get_body(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let body = find_html_root(ctx).and_then(|html| {
        let dom = ctx.host().dom();
        dom.first_child_with_tag(html, "body")
    });
    Ok(wrap_entity_or_null(ctx.vm, body))
}

pub(super) fn native_document_get_title(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let title_text: String = {
        let Some(html) = find_html_root(ctx) else {
            return Ok(JsValue::String(ctx.vm.well_known.empty));
        };
        let dom = ctx.host().dom();
        let Some(head) = dom.first_child_with_tag(html, "head") else {
            return Ok(JsValue::String(ctx.vm.well_known.empty));
        };
        let Some(title) = dom.first_child_with_tag(head, "title") else {
            return Ok(JsValue::String(ctx.vm.well_known.empty));
        };
        // Concat all text-node children (matches WHATWG §3.2.9
        // "descendant text content", but for title we only walk
        // immediate children — title cannot legally contain nested
        // elements).
        let mut out = String::new();
        for child in dom.children_iter(title) {
            if let Ok(text) = dom.world().get::<&elidex_ecs::TextContent>(child) {
                out.push_str(&text.0);
            }
        }
        out
    };
    let sid = ctx.vm.strings.intern(&title_text);
    Ok(JsValue::String(sid))
}

pub(super) fn native_document_get_url(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let url = ctx.vm.navigation.current_url.clone();
    let sid = ctx.vm.strings.intern(&url);
    Ok(JsValue::String(sid))
}

pub(super) fn native_document_get_ready_state(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Phase 2: scripts run after parse completes (eval is synchronous
    // from the shell's perspective), so `complete` is honest.  The
    // shell wires real lifecycle transitions in PR6.
    Ok(JsValue::String(ctx.vm.well_known.complete))
}

// ---------------------------------------------------------------------------
// Installation — add the own-properties to the document wrapper at bind time.
// ---------------------------------------------------------------------------

impl Vm {
    /// Re-install the document-only own-properties each bind cycle.
    /// Called by `install_document_global` after the HostObject
    /// wrapper has been refreshed.
    ///
    /// We re-install rather than once-at-VM-init because the
    /// document wrapper is cached per-Entity in
    /// `HostData::wrapper_cache` — successive binds with the same
    /// document entity return the same wrapper ObjectId, so
    /// properties survive without any work, but the first bind
    /// *must* populate it.  The "already installed" check below
    /// keeps rebind cycles O(1).
    pub(in crate::vm) fn install_document_methods_if_needed(&mut self, doc_wrapper: ObjectId) {
        // Fast path: skip if this specific document entity already
        // has its wrapper patched.  A per-entity set (rather than a
        // VM-wide flag) is load-bearing — a single `Vm` can be bound
        // to multiple document entities over its lifetime and each
        // produces a **distinct** wrapper via `wrapper_cache`.  A
        // global flag would skip install on every document after the
        // first, leaving `getElementById` etc. missing on later ones.
        let doc_entity = self
            .host_data()
            .expect("install_document_methods_if_needed requires HostData")
            .document();
        let already_installed = self
            .host_data()
            .is_some_and(|hd| hd.document_methods_installed.contains(&doc_entity));
        if already_installed {
            return;
        }

        self.inner.install_methods(doc_wrapper, DOCUMENT_METHODS);
        self.inner
            .install_ro_accessors(doc_wrapper, DOCUMENT_RO_ACCESSORS);
        // ParentNode mixin (WHATWG §5.2.4) — `prepend` / `append` /
        // `replaceChildren`.  Shares the natives with `Element.prototype`
        // via `install_parent_node_mixin`.
        self.inner.install_parent_node_mixin(doc_wrapper);

        if let Some(hd) = self.host_data() {
            hd.document_methods_installed.insert(doc_entity);
        }
    }
}

// Method + accessor tables are file-scope constants so they are not
// rebuilt on every bind and so the `install_document_methods_if_needed`
// body reads top-down.
const DOCUMENT_METHODS: &[(&str, super::super::NativeFn)] = &[
    ("getElementById", native_document_get_element_by_id),
    ("querySelector", native_document_query_selector),
    ("querySelectorAll", native_document_query_selector_all),
    (
        "getElementsByTagName",
        native_document_get_elements_by_tag_name,
    ),
    (
        "getElementsByClassName",
        native_document_get_elements_by_class_name,
    ),
    ("createElement", native_document_create_element),
    ("createTextNode", native_document_create_text_node),
    ("createComment", native_document_create_comment),
    (
        "createDocumentFragment",
        native_document_create_document_fragment,
    ),
];

const DOCUMENT_RO_ACCESSORS: &[(&str, super::super::NativeFn)] = &[
    ("documentElement", native_document_get_document_element),
    ("head", native_document_get_head),
    ("body", native_document_get_body),
    ("title", native_document_get_title),
    ("URL", native_document_get_url),
    ("documentURI", native_document_get_url),
    ("readyState", native_document_get_ready_state),
];
