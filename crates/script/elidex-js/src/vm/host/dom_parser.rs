//! `DOMParser` (HTML §8.5.1) + `XMLSerializer` (HTML §8.5.8)
//! interfaces.
//!
//! Both are WebIDL interfaces rooted at `Object` — not `EventTarget`,
//! not `Node`. Prototype chains:
//!
//! ```text
//! DOMParser instance (ObjectKind::DomParser, payload-free)
//!   → DOMParser.prototype  (this module)
//!     → Object.prototype
//! XMLSerializer instance (ObjectKind::XmlSerializer, payload-free)
//!   → XMLSerializer.prototype (this module)
//!     → Object.prototype
//! ```
//!
//! ## Design (S5-1)
//!
//! Unlike the boa engine — which returned a fake closure-backed
//! plain-object stub from `parseFromString` — the VM returns a **real**
//! [`elidex_ecs::NodeKind::Document`] entity:
//!
//! 1. [`EcsDom::create_document_node`] spawns a throwaway `Document`
//!    entity in the bound `EcsDom` **without** clobbering the page's
//!    `document_root`.
//! 2. A synthesized `<html>` element becomes its `documentElement`
//!    child, and the markup is fragment-parsed into that element via the
//!    engine-indep [`elidex_script_session::apply_set_inner_html`] seam
//!    (the same §11.3 strict-first fragment parse `innerHTML` uses — the
//!    layering mandate keeps the `elidex_html_parser` call out of
//!    `vm/host/`). Parsing in `<html>` context lets html5ever synthesize
//!    `<head>` / `<body>` so `doc.body` / `doc.head` work on real markup.
//! 3. The Document entity is wrapped with `create_element_wrapper` +
//!    `install_document_methods_for_entity`, so the returned object gets
//!    `querySelector` / `querySelectorAll` / `getElementById` / `body` /
//!    `head` / `documentElement` for free (the per-entity Document
//!    own-properties; `prototype_kind_for` routes `NodeKind::Document` →
//!    `Node.prototype`).
//!
//! `serializeToString` reuses the engine-indep
//! [`elidex_dom_api::serialize_node_to_string`] node-kind-dispatching
//! serializer (element → outer markup; Document / DocumentFragment →
//! children markup; comment → `<!--data-->`; character data → escaped
//! text) rather than hand-building tags.
//!
//! ## Deferred
//!
//! - Full §13.4 document parse (doctype + exact html/head/body
//!   construction from arbitrary markup) — slot
//!   `#11-domparser-full-document-parse-fidelity`. The fragment-parse
//!   approach here is boa-parity-bounded; cross-`EcsDom` adoption of a
//!   true `parse_html` document tree is out of scope for this narrow-
//!   additive PR.
//! - Real XML parsing + XML serialization (self-closing void elements,
//!   namespace prefixes) — slot `#11-domparser-xml-real-parsing`. All
//!   accepted MIME types are HTML-parsed (boa parity).

#![cfg(feature = "engine")]

use elidex_ecs::Attributes;
use elidex_script_session::{apply_set_inner_html, SetInnerHtmlOptions};

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::VmInner;

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Allocate `DOMParser.prototype`, install `parseFromString`, and
    /// expose the `DOMParser` constructor on `globals`.
    ///
    /// Runs during `register_globals()` after `register_prototypes`
    /// (needs `object_prototype`).
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — indicates a mis-ordered
    /// registration pass.
    pub(in crate::vm) fn register_dom_parser_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_dom_parser_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        let parse_sid = self.strings.intern("parseFromString");
        self.install_native_method(
            proto_id,
            parse_sid,
            native_dom_parser_parse_from_string,
            PropertyAttrs::METHOD,
        );

        let ctor =
            self.create_constructor_only_function("DOMParser", native_dom_parser_constructor);
        self.wire_ctor_prototype(ctor, proto_id);
        let name_sid = self.strings.intern("DOMParser");
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }

    /// Allocate `XMLSerializer.prototype`, install `serializeToString`,
    /// and expose the `XMLSerializer` constructor on `globals`.
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None`.
    pub(in crate::vm) fn register_xml_serializer_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_xml_serializer_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        let serialize_sid = self.strings.intern("serializeToString");
        self.install_native_method(
            proto_id,
            serialize_sid,
            native_xml_serializer_serialize_to_string,
            PropertyAttrs::METHOD,
        );

        let ctor = self
            .create_constructor_only_function("XMLSerializer", native_xml_serializer_constructor);
        self.wire_ctor_prototype(ctor, proto_id);
        let name_sid = self.strings.intern("XMLSerializer");
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }

    /// Wire `ctor.prototype = proto` (BUILTIN, non-enumerable) and
    /// `proto.constructor = ctor` (METHOD) — the standard WebIDL
    /// interface-object ↔ prototype back-reference pair (mirrors the
    /// Blob / TextEncoder install).
    fn wire_ctor_prototype(&mut self, ctor: ObjectId, proto_id: ObjectId) {
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor)),
            PropertyAttrs::METHOD,
        );
    }
}

// ---------------------------------------------------------------------------
// Brand checks
// ---------------------------------------------------------------------------

/// WebIDL branded-receiver gate for `DOMParser.prototype.*`. Throws a
/// TypeError ("illegal invocation") on a non-branded receiver — boa
/// skipped this, the VM enforces it for spec fidelity (mirrors
/// `require_blob_or_file_this`).
fn require_dom_parser_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<(), VmError> {
    if let JsValue::Object(id) = this {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::DomParser) {
            return Ok(());
        }
    }
    Err(VmError::type_error(format!(
        "Failed to execute '{method}' on 'DOMParser': illegal invocation"
    )))
}

/// WebIDL branded-receiver gate for `XMLSerializer.prototype.*`.
fn require_xml_serializer_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<(), VmError> {
    if let JsValue::Object(id) = this {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::XmlSerializer) {
            return Ok(());
        }
    }
    Err(VmError::type_error(format!(
        "Failed to execute '{method}' on 'XMLSerializer': illegal invocation"
    )))
}

// ---------------------------------------------------------------------------
// DOMParser
// ---------------------------------------------------------------------------

fn native_dom_parser_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };
    // Promote the pre-allocated Ordinary instance to DomParser — do not
    // touch `prototype` so the `new.target.prototype` chain installed by
    // `do_new` survives (Blob / TextEncoder lesson).
    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::DomParser;
    Ok(JsValue::Object(inst_id))
}

/// `DOMParser.prototype.parseFromString(string, type)` (HTML §8.5.1).
///
/// Returns a real `Document` entity (see module docs). Both arguments
/// are `ToString`-coerced (WebIDL `DOMString` / `[LegacyNullToEmptyString]`
/// is not applied — boa ToString'd both too).
fn native_dom_parser_parse_from_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_dom_parser_this(ctx, this, "parseFromString")?;

    let markup_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let type_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let markup_sid = super::super::coerce::to_string(ctx.vm, markup_arg)?;
    let type_sid = super::super::coerce::to_string(ctx.vm, type_arg)?;

    // D3 — MIME validation, boa parity. The accepted set is matched
    // verbatim (no `;`-parameter stripping — boa matched the raw
    // string), and ALL accepted types are HTML-parsed (no real XML
    // parser yet).
    // Deferred → slot `#11-domparser-xml-real-parsing`.
    let mime = ctx.vm.strings.get_utf8(type_sid);
    match mime.as_str() {
        "text/html"
        | "text/xml"
        | "application/xml"
        | "application/xhtml+xml"
        | "image/svg+xml" => {}
        other => {
            return Err(VmError::type_error(format!(
                "Failed to execute 'parseFromString' on 'DOMParser': \
                 unsupported MIME type '{other}'"
            )));
        }
    }

    // Resolve the markup to an owned String first — the string-pool
    // borrow conflicts with the `&mut EcsDom` borrow held inside
    // `with_session_and_dom`.
    let markup = ctx.vm.strings.get_utf8(markup_sid);

    // Build the throwaway Document + its <html> documentElement, then
    // fragment-parse the markup INTO the <html> element in-place via the
    // engine-indep `apply_set_inner_html` seam (the same §11.3
    // strict-first fragment-parse `innerHTML` uses — CLAUDE.md layering
    // mandate: no `elidex_html_parser` call from `vm/host/`). No
    // cross-EcsDom adoption. Parsing in <html> context lets html5ever
    // synthesize <head>/<body> so `doc.body`/`doc.head` work on real
    // markup.
    // Deferred → slot `#11-domparser-full-document-parse-fidelity`: a
    // true §13.4 full-document parse (doctype + exact html/head/body
    // construction) is deferred; the fragment-parse here is
    // boa-parity-bounded.
    let Some(doc_entity) = ctx.host_if_bound().map(|host| {
        host.with_session_and_dom(|_session, dom| {
            // §13.4 inert-document discipline: a DOMParser document has no
            // browsing context and scripting is disabled, so its construction
            // must fire NO mutation reactions. `apply_set_inner_html`'s final
            // append/remove run against a target rooted at a `NodeKind::Document`
            // (so `is_connected` is true) and would otherwise drive the live
            // page dispatcher that `Vm::bind` installs — firing Insert events,
            // custom-element reactions, and Range side effects for an inert
            // document. This is JS-OBSERVABLE: a custom element in the parsed
            // markup whose name matches a registered definition would be
            // `try-to-upgrade`'d on the connected Insert (constructor runs →
            // `finalize_success` enqueues Connected → `connectedCallback`
            // fires) — exactly the inert-document violation §13.4 forbids
            // (verified by `dom_parser_inert_document_no_custom_element_upgrade`:
            // with this suppression removed, the parsed `<x-test>`'s constructor
            // + connectedCallback fire; with it in place, neither does). Take the
            // dispatcher out for the build (mirroring
            // `EcsDom::begin_detached_fragment`, `crates/core/elidex-ecs/src/dom/
            // tree/teardown.rs:325`) and restore it afterwards so the RETURNED
            // document is live (observers/mutations JS adds after
            // `parseFromString` still fire).
            let saved_dispatcher = dom.take_mutation_dispatcher();
            // Throwaway Document — does NOT clobber the page's cached
            // `document_root` (see `EcsDom::create_document_node`).
            let doc = dom.create_document_node();
            let html_el = dom.create_element_with_owner("html", Attributes::default(), Some(doc));
            let _ = dom.append_child(doc, html_el);
            // Replaces <html>'s (empty) children with the parsed
            // fragment. The returned MutationRecord is intentionally
            // dropped: this is a throwaway document with no registered
            // MutationObserver, so no §4.3 record delivery applies.
            let _ = apply_set_inner_html(dom, html_el, &markup, SetInnerHtmlOptions::default());
            if let Some(dispatcher) = saved_dispatcher {
                dom.set_mutation_dispatcher(dispatcher);
            }
            doc
        })
    }) else {
        // Unbound VM (wrapper retained across `unbind()`) — no DOM to
        // parse into. Return null (silent no-op policy for unbound
        // DOM-touching natives).
        return Ok(JsValue::Null);
    };

    // Wrap the Document entity + install the per-entity Document method
    // suite (idempotent, entity-keyed) so the returned object exposes
    // querySelector / querySelectorAll / getElementById / body / head /
    // documentElement.
    let wrapper = ctx.vm.create_element_wrapper(doc_entity);
    ctx.vm
        .install_document_methods_for_entity(doc_entity, wrapper);
    Ok(JsValue::Object(wrapper))
}

// ---------------------------------------------------------------------------
// XMLSerializer
// ---------------------------------------------------------------------------

fn native_xml_serializer_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };
    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::XmlSerializer;
    Ok(JsValue::Object(inst_id))
}

/// `XMLSerializer.prototype.serializeToString(node)` (HTML §8.5.8).
///
/// D4 — reuses the engine-indep
/// [`elidex_dom_api::serialize_node_to_string`], which dispatches by
/// `NodeKind`: an Element serializes via its outer markup; a Document /
/// DocumentFragment as the markup of its children (the canonical
/// `serializeToString(parseFromString(...))` round-trip); a Comment as
/// `<!--data-->`; character data as the escaped text serialization. This
/// is HTML serialization (boa parity); true XML serialization
/// (self-closing void elements, namespace prefixes) is deferred → slot
/// `#11-domparser-xml-real-parsing`.
fn native_xml_serializer_serialize_to_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_xml_serializer_this(ctx, this, "serializeToString")?;

    // WebIDL `serializeToString(Node root)` — the argument is required.
    // boa threw a TypeError when the arg was absent / not a node; match
    // that by requiring a HostObject node wrapper.
    let node_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let entity = match node_arg {
        JsValue::Object(id) => match ctx.vm.get_object(id).kind {
            ObjectKind::HostObject { entity_bits } => elidex_ecs::Entity::from_bits(entity_bits),
            _ => None,
        },
        _ => None,
    };
    let Some(entity) = entity else {
        return Err(VmError::type_error(
            "Failed to execute 'serializeToString' on 'XMLSerializer': \
             parameter 1 is not of type 'Node'.",
        ));
    };

    let Some((dom, strings)) = ctx.dom_and_strings_if_bound() else {
        // Unbound — no DOM to read. Return empty string (silent no-op
        // policy for unbound DOM-touching natives).
        let empty = ctx.vm.strings.intern("");
        return Ok(JsValue::String(empty));
    };
    // HTML fragment serialization of an ARBITRARY node, dispatched by
    // NodeKind in the engine-indep serializer (layering mandate — no tag /
    // comment / doctype strings hand-built in vm/host/): an Element
    // serializes as its outer markup; a Document / DocumentFragment as the
    // markup of its children (so `serializeToString(parseFromString(...))`
    // round-trips to real markup, not concatenated text); a Comment as
    // `<!--data-->`; character data as the escaped text serialization.
    let serialized = elidex_dom_api::serialize_node_to_string(entity, dom);
    let sid = strings.intern(&serialized);
    Ok(JsValue::String(sid))
}
