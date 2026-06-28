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
//! [`elidex_ecs::NodeKind::Document`] entity. The whole inert-document
//! BUILD algorithm lives engine-indep in
//! [`elidex_form::parse_into_inert_document`] (CLAUDE.md Layering mandate
//! — `vm/host/` is marshalling-only; the build + the decision of which
//! structural-fact reconcilers to re-run are DOM semantics, not VM
//! marshalling). The native here only:
//!
//! 1. brand-checks the receiver + ToString-coerces / MIME-validates the args,
//! 2. resolves the caller document's URL (HTML §8.5.1 step 2 base fallback),
//! 3. calls [`elidex_form::parse_into_inert_document`] inside
//!    `with_session_and_dom` to get the throwaway Document entity, and
//! 4. wraps it with `create_element_wrapper` +
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
//! - True §13.2 full-document parse — `parseFromString(text/html)` routes
//!   via HTML §8.5.1 through the DOCUMENT parser in §13.2 (doctype + exact
//!   html/head/body construction from arbitrary markup) — slot
//!   `#11-domparser-full-document-parse-fidelity`. The fragment-parse
//!   approach in [`elidex_form::parse_into_inert_document`] is
//!   boa-parity-bounded; cross-`EcsDom` adoption of a true `parse_html`
//!   document tree is out of scope for this narrow-additive PR.
//! - Real XML parsing + XML serialization (self-closing void elements,
//!   namespace prefixes) — slot `#11-domparser-xml-real-parsing`. All
//!   accepted MIME types are HTML-parsed (boa parity).

#![cfg(feature = "engine")]

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

    // Unbound VM (wrapper retained across `Vm::unbind()`): there is no DOM
    // to parse into. Follow the silent-detached policy the DOM-touching
    // native family uses (`class_list` / `css_style_declaration` check
    // `host_if_bound()` first), returning the no-op value BEFORE coercing
    // the args or validating the MIME — otherwise a retained
    // `parser.parseFromString(Symbol(), 'text/html')` (ToString-throws) or
    // an unsupported MIME type would still throw a TypeError after unbind
    // instead of no-op'ing like the rest of the family (Codex R4).
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }

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
    // HTML §8.5.1 step 2: the new Document's URL is the URL of the CALLER
    // document, used as the base-URL fallback when the markup has no
    // `<base href>`. Read the bound document's current URL from the VM
    // navigation state (the same accessor `document.URL` / fetch Referer
    // resolution use). Cloned out before the `&mut EcsDom` borrow below.
    let caller_url = ctx.vm.navigation.current_url.clone();

    // Engine-indep inert-document BUILD (CLAUDE.md Layering mandate — the
    // build algorithm + the structural-fact reconciler-set decision live
    // in `elidex_form`, not `vm/host/`). The native is marshalling-only:
    // it just hands the resolved markup + caller URL to the primitive and
    // wraps the returned entity.
    // Boundness was checked at entry (the unbound no-op returns null above)
    // and a synchronous native cannot be re-entered to unbind mid-call, so
    // the host is present by construction here.
    let doc_entity = ctx
        .host_if_bound()
        .expect("DOMParser bound: host_if_bound() checked at native entry")
        .with_session_and_dom(|_session, dom| {
            elidex_form::parse_into_inert_document(dom, &markup, &caller_url)
        });

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

    // Unbound VM (wrapper retained across `Vm::unbind()`): no DOM to read.
    // Return the no-op empty string BEFORE the Node-arg validation below,
    // so a retained `serializer.serializeToString(<non-Node>)` no-op's like
    // the rest of the DOM-touching native family instead of throwing a
    // TypeError from the arg gate after unbind (Codex R4, sibling of the
    // DOMParser `parseFromString` fix above).
    if ctx.host_if_bound().is_none() {
        let empty = ctx.vm.strings.intern("");
        return Ok(JsValue::String(empty));
    }

    // WebIDL `serializeToString(Node root)` — the argument is required.
    // boa threw a TypeError when the arg was absent / not a node; match
    // that by requiring a HostObject node wrapper.
    let node_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let entity = match node_arg {
        JsValue::Object(id) => {
            let host_entity = match ctx.vm.get_object(id).kind {
                ObjectKind::HostObject { entity_bits } => {
                    elidex_ecs::Entity::from_bits(entity_bits)
                }
                _ => None,
            };
            // Reverse half of the canvas-2D-context bidirectional brand: a
            // `CanvasRenderingContext2D` wrapper deliberately shares its
            // `<canvas>` entity (which IS a Node), so the `is_node()` check
            // below would wrongly accept it and serialize the backing
            // `<canvas>` instead of throwing. Reject it as a non-Node here,
            // mirroring `node_proto::require_node_arg` (Codex R5).
            host_entity.filter(|&e| !super::canvas::is_canvas_2d_context_wrapper(ctx.vm, id, e))
        }
        _ => None,
    };
    let Some(entity) = entity else {
        return Err(VmError::type_error(
            "Failed to execute 'serializeToString' on 'XMLSerializer': \
             parameter 1 is not of type 'Node'.",
        ));
    };

    // Bound (checked at entry): borrow the DOM + string pool for the read.
    let (dom, strings) = ctx
        .dom_and_strings_if_bound()
        .expect("XMLSerializer bound: host_if_bound() checked at native entry");
    // WebIDL `serializeToString(Node root)`: the HostObject extraction above
    // accepts ANY entity-backed host object, but `globalThis` / `window` is a
    // HostObject over `NodeKind::Window` (an EventTarget, NOT a Node). Reject
    // non-Node kinds so `serializeToString(window)` throws a TypeError rather
    // than serializing "". `NodeKind::is_node()` is false for Window / Worker /
    // OffscreenCanvas — mirrors `node_proto::require_node_arg` /
    // `dom_bridge.rs` Node-argument coercion.
    if !matches!(dom.node_kind_inferred(entity), Some(k) if k.is_node()) {
        return Err(VmError::type_error(
            "Failed to execute 'serializeToString' on 'XMLSerializer': \
             parameter 1 is not of type 'Node'.",
        ));
    }
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
