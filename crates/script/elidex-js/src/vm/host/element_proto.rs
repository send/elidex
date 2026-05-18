//! `Element.prototype` intrinsic (WHATWG DOM §4.9).
//!
//! Holds **Element-only** members — sibling tree navigation
//! (`nextElementSibling`, `previousElementSibling`), attribute
//! manipulation (`getAttribute`, `setAttribute`, …), selector
//! helpers (`matches`, `closest`), `tagName` / `id` / `className`,
//! and the `ChildNode` mixin method `remove()`.  The ParentNode
//! mixin's read surface (`children`, `firstElementChild`,
//! `querySelector`, …) is installed via
//! [`super::parentnode::VmInner::install_parent_node_readers`] and so
//! reaches Element / DocumentFragment / ShadowRoot uniformly.
//!
//! Node-common members — `parentNode`, `parentElement`,
//! `firstChild`, `nodeType`, `textContent`, `appendChild`,
//! `removeChild`, etc. — live on `Node.prototype` and so apply to
//! Text / Comment / Document / DocumentFragment wrappers too.
//!
//! ## Prototype chain
//!
//! ```text
//! element wrapper (HostObject)
//!   → Element.prototype        (this intrinsic)
//!     → Node.prototype         (Node-common accessors + mutation)
//!       → EventTarget.prototype
//!         → Object.prototype   (bootstrap)
//! ```
//!
//! Text and Comment wrappers skip `Element.prototype` and chain
//! directly to `Node.prototype` → `EventTarget.prototype`.  This
//! keeps Element-specific names off Text instances
//! (`textNode.getAttribute` is `undefined`, matching browsers)
//! while still exposing Node-common members on them.
//!
//! ## Why a shared prototype?
//!
//! The alternative — installing methods directly on each element
//! wrapper — would allocate one native-function per method per
//! element (tens of methods × thousands of elements).  A single
//! shared prototype matches browser engines (V8's `HTMLElement`
//! prototype chain, SpiderMonkey's `ElementProto`) and aligns with
//! how other intrinsics (`Array.prototype`, `Window.prototype`) are
//! structured elsewhere in the VM.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};
use super::super::{NativeFn, VmInner};
use super::dom_bridge::{
    coerce_first_arg_to_string, coerce_first_arg_to_string_id, invoke_dom_api, tree_nav_getter,
    wrap_entities_as_array,
};
use super::element_attrs::{
    native_element_get_attribute, native_element_get_attribute_names,
    native_element_get_attribute_node, native_element_get_attributes,
    native_element_get_class_list, native_element_get_class_name, native_element_get_id,
    native_element_get_tag_name, native_element_has_attribute, native_element_remove_attribute,
    native_element_remove_attribute_node, native_element_set_attribute,
    native_element_set_attribute_node, native_element_set_class_name, native_element_set_id,
    native_element_toggle_attribute,
};

use elidex_ecs::NodeKind;

impl VmInner {
    /// Allocate `Element.prototype` whose parent is
    /// `Node.prototype`.
    ///
    /// Called from `register_globals()` **after**
    /// `register_node_prototype` so the chain can climb through
    /// `Node.prototype` → `EventTarget.prototype` → `Object.prototype`.
    ///
    /// # Panics
    ///
    /// Panics if `node_prototype` has not been populated (would mean
    /// `register_node_prototype` was skipped or called in the wrong
    /// order).
    pub(in crate::vm) fn register_element_prototype(&mut self) {
        let node_proto = self
            .node_prototype
            .expect("register_element_prototype called before register_node_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(node_proto),
            extensible: true,
        });
        self.element_prototype = Some(proto_id);
        self.install_element_tree_nav(proto_id);
        self.install_element_attributes(proto_id);
        // ChildNode mixin (WHATWG §5.2.2) — `before` / `after` /
        // `replaceWith` / `remove`.  The same native fns are installed
        // on `CharacterData.prototype` from
        // `register_character_data_prototype`, matching WHATWG's
        // mixin-on-multiple-interfaces pattern.
        self.install_child_node_mixin(proto_id);
        // ParentNode mixin (WHATWG §4.2.6) — mutation (`prepend` /
        // `append` / `replaceChildren`) + read surface (`children` /
        // `firstElementChild` / `lastElementChild` /
        // `childElementCount` / `querySelector` / `querySelectorAll`).
        // Same install fns also run on `DocumentFragment.prototype`
        // (ShadowRoot inherits via that chain).  The Document wrapper
        // routes the same native bodies via `DOCUMENT_RO_ACCESSORS` +
        // `DOCUMENT_METHODS` per-bind rather than calling
        // `install_parent_node_readers` (Document has no shared proto).
        self.install_parent_node_mixin(proto_id);
        self.install_parent_node_readers(proto_id);
        self.install_element_matches(proto_id);
        self.install_element_shadow_dom(proto_id);
        self.install_element_inner_html(proto_id);
    }

    /// Install `attachShadow` method + `shadowRoot` accessor on
    /// `Element.prototype` (WHATWG DOM §4.2.14 / §4.8).  ShadowRoot
    /// wrapper / state-cache plumbing lives in
    /// [`super::shadow_root_proto`].
    fn install_element_shadow_dom(&mut self, proto_id: ObjectId) {
        let attach_shadow_sid = self.strings.intern("attachShadow");
        self.install_native_method(
            proto_id,
            attach_shadow_sid,
            super::element_shadow::native_element_attach_shadow,
            shape::PropertyAttrs::METHOD,
        );
        let shadow_root_sid = self.strings.intern("shadowRoot");
        self.install_accessor_pair(
            proto_id,
            shadow_root_sid,
            super::element_shadow::native_element_get_shadow_root,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }

    /// Install `innerHTML` / `outerHTML` accessors + `setHTMLUnsafe`
    /// and `getHTML` methods on `Element.prototype` (WHATWG HTML
    /// §4.4.5 / §4.4.6 / §4.4.7).  ShadowRoot's parallel set installs
    /// from [`super::shadow_root_proto`]; the shared native bodies
    /// live in [`super::dom_inner_html`].
    fn install_element_inner_html(&mut self, proto_id: ObjectId) {
        self.install_accessor_pair(
            proto_id,
            self.well_known.inner_html,
            super::dom_inner_html::native_element_get_inner_html,
            Some(super::dom_inner_html::native_element_set_inner_html),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.outer_html,
            super::dom_inner_html::native_element_get_outer_html,
            Some(super::dom_inner_html::native_element_set_outer_html),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_native_method(
            proto_id,
            self.well_known.set_html_unsafe,
            super::dom_inner_html::native_element_set_html_unsafe,
            shape::PropertyAttrs::METHOD,
        );
        self.install_native_method(
            proto_id,
            self.well_known.get_html,
            super::dom_inner_html::native_element_get_html,
            shape::PropertyAttrs::METHOD,
        );
    }

    /// Install Element-only sibling accessors from the
    /// NonDocumentTypeChildNode mixin (WHATWG DOM §4.4).  The four
    /// ParentNode-mixin reader accessors (`firstElementChild`,
    /// `lastElementChild`, `children`, `childElementCount`) install
    /// via [`Self::install_parent_node_readers`] from the shared
    /// [`super::parentnode`] module.  Node-level accessors
    /// (`parentNode`, `parentElement`, `firstChild`, `childNodes`, …)
    /// live on `Node.prototype`.
    fn install_element_tree_nav(&mut self, proto_id: ObjectId) {
        for (name_sid, getter) in [
            (
                self.well_known.next_element_sibling,
                native_element_get_next_element_sibling as NativeFn,
            ),
            (
                self.well_known.previous_element_sibling,
                native_element_get_previous_element_sibling,
            ),
        ] {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                None,
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }

    /// Install Element attribute-manipulation methods + `id` /
    /// `className` / `tagName` accessors on `proto_id`.
    fn install_element_attributes(&mut self, proto_id: ObjectId) {
        // `tagName` — read-only accessor (WHATWG §4.9, uppercase for
        // HTML).
        self.install_accessor_pair(
            proto_id,
            self.well_known.tag_name,
            native_element_get_tag_name,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // `id` / `className` — read/write accessors (WHATWG §3.5).
        // `WEBIDL_RO_ACCESSOR`'s `writable` bit is meaningless for
        // accessors; the RW-ness comes from the setter slot below.
        for (name_sid, getter, setter) in [
            (
                self.well_known.id,
                native_element_get_id as NativeFn,
                native_element_set_id as NativeFn,
            ),
            (
                self.well_known.class_name,
                native_element_get_class_name,
                native_element_set_class_name,
            ),
        ] {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                Some(setter),
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        // `attributes` accessor — returns a live `NamedNodeMap`
        // backed by the element's `Attributes` component
        // (WHATWG §4.9).
        self.install_accessor_pair(
            proto_id,
            self.well_known.attributes,
            native_element_get_attributes,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // `classList` accessor — returns an identity-preserving
        // `DOMTokenList` wrapper backed by the element's `class`
        // attribute (WHATWG DOM §3.5).  Defined on Element rather
        // than HTMLElement per the spec interface boundary.
        #[cfg(feature = "engine")]
        self.install_accessor_pair(
            proto_id,
            self.well_known.class_list,
            native_element_get_class_list,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // Attribute methods.
        for (name_sid, func) in [
            (
                self.well_known.get_attribute,
                native_element_get_attribute as NativeFn,
            ),
            (self.well_known.set_attribute, native_element_set_attribute),
            (
                self.well_known.remove_attribute,
                native_element_remove_attribute,
            ),
            (self.well_known.has_attribute, native_element_has_attribute),
            (
                self.well_known.get_attribute_names,
                native_element_get_attribute_names,
            ),
            (
                self.well_known.toggle_attribute,
                native_element_toggle_attribute,
            ),
            // Attr-typed methods — WHATWG §4.9.2.
            (
                self.well_known.get_attribute_node,
                native_element_get_attribute_node,
            ),
            (
                self.well_known.set_attribute_node,
                native_element_set_attribute_node,
            ),
            (
                self.well_known.remove_attribute_node,
                native_element_remove_attribute_node,
            ),
        ] {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }
    }

    /// Install `matches(selector)` / `closest(selector)` +
    /// `insertAdjacentElement` / `insertAdjacentText` +
    /// `getElementsByTagName` / `getElementsByClassName` on
    /// `proto_id`.  The ParentNode-mixin selector pair
    /// (`querySelector` / `querySelectorAll`) installs via
    /// [`Self::install_parent_node_readers`].
    fn install_element_matches(&mut self, proto_id: ObjectId) {
        for (name_sid, func) in [
            (self.well_known.matches, native_element_matches as NativeFn),
            (self.well_known.closest, native_element_closest),
            (
                self.well_known.insert_adjacent_element,
                super::element_insert_adjacent::native_element_insert_adjacent_element,
            ),
            (
                self.well_known.insert_adjacent_text,
                super::element_insert_adjacent::native_element_insert_adjacent_text,
            ),
            (
                self.well_known.get_elements_by_tag_name,
                native_element_get_elements_by_tag_name,
            ),
            (
                self.well_known.get_elements_by_class_name,
                native_element_get_elements_by_class_name,
            ),
        ] {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }
    }
}

// ---------------------------------------------------------------------------
// Natives: tree-navigation accessors
// ---------------------------------------------------------------------------

fn native_element_get_next_element_sibling(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    tree_nav_getter(ctx, this, elidex_ecs::EcsDom::next_element_sibling)
}

fn native_element_get_previous_element_sibling(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    tree_nav_getter(ctx, this, elidex_ecs::EcsDom::prev_element_sibling)
}

// ---------------------------------------------------------------------------
// Natives: matches / closest
// ---------------------------------------------------------------------------

fn native_element_matches(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "Element", "matches", |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(JsValue::Boolean(false));
    };
    let target_sid = coerce_first_arg_to_string_id(ctx, args)?;
    invoke_dom_api(ctx, "matches", entity, &[JsValue::String(target_sid)])
}

fn native_element_closest(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "Element", "closest", |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(JsValue::Null);
    };
    let target_sid = coerce_first_arg_to_string_id(ctx, args)?;
    invoke_dom_api(ctx, "closest", entity, &[JsValue::String(target_sid)])
}

// ---------------------------------------------------------------------------
// Element.prototype.getElementsByTagName / getElementsByClassName — WHATWG §4.2.6
// ---------------------------------------------------------------------------

/// `Element.prototype.getElementsByTagName(qualifiedName)` — WHATWG §4.2.6.2.
///
/// Scope is **descendants of the receiver only**; the receiver is
/// never a match candidate.  Shares
/// [`collect_descendants_by_tag_name`] with the Document-level form
/// so `*` handling and ASCII case-folding cannot drift.
pub(super) fn native_element_get_elements_by_tag_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // WebIDL brand check runs BEFORE argument conversion — otherwise
    // `Element.prototype.getElementsByTagName.call({}, {toString(){ ... }})`
    // would trigger user code via ToString even though the invalid
    // receiver should be a silent no-op.
    let Some(root) =
        super::event_target::require_receiver(ctx, this, "Element", "getElementsByTagName", |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(wrap_entities_as_array(ctx.vm, &[]));
    };
    let tag = coerce_first_arg_to_string(ctx, args)?;
    let id = ctx.vm.alloc_collection(elidex_dom_api::LiveCollection::new(
        root,
        elidex_dom_api::CollectionFilter::ByTagName(tag),
        elidex_dom_api::CollectionKind::HtmlCollection,
    ));
    Ok(JsValue::Object(id))
}

/// `Element.prototype.getElementsByClassName(classNames)` —
/// WHATWG §4.2.6.2.  Descendant-only, live.
pub(super) fn native_element_get_elements_by_class_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(root) = super::event_target::require_receiver(
        ctx,
        this,
        "Element",
        "getElementsByClassName",
        |k| k == NodeKind::Element,
    )?
    else {
        return Ok(wrap_entities_as_array(ctx.vm, &[]));
    };
    let class_str = coerce_first_arg_to_string(ctx, args)?;
    // ASCII whitespace per WHATWG DOM §4.2.6.2 — see the matching
    // comment on `native_document_get_elements_by_class_name`.
    let class_names: Vec<String> = class_str
        .split_ascii_whitespace()
        .map(str::to_owned)
        .collect();
    let id = ctx.vm.alloc_collection(elidex_dom_api::LiveCollection::new(
        root,
        elidex_dom_api::CollectionFilter::ByClassNames(class_names),
        elidex_dom_api::CollectionKind::HtmlCollection,
    ));
    Ok(JsValue::Object(id))
}
