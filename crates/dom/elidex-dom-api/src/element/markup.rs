//! DOM parsing and serialization handlers: innerHTML (getter and setter),
//! insertAdjacentHTML, and the HTML fragment serializer behind innerHTML,
//! outerHTML, getHTML and XMLSerializer.
//!
//! WHATWG HTML §8.5.3 HTML serialization methods, §8.5.4 The innerHTML
//! property, §8.5.5 The outerHTML property, §8.5.6 The insertAdjacentHTML()
//! method, §8.5.8 The XMLSerializer interface, §13.3 Serializing HTML
//! fragments.

use elidex_ecs::{Attributes, EcsDom, Entity, TagType, TextContent};
use elidex_plugin::JsValue;
use elidex_script_session::{DomApiError, DomApiErrorKind, DomApiHandler, Mutation, SessionCore};

use crate::util::{escape_attr, escape_html};

// ---------------------------------------------------------------------------
// innerHTML getter
// ---------------------------------------------------------------------------

/// HTML raw text elements whose text children must NOT be escaped during
/// serialization (the content is literal, not entity-decoded by parsers).
const RAW_TEXT_ELEMENTS: &[&str] = &[
    "script", "style", "xmp", "iframe", "noembed", "noframes", "noscript",
];

/// `element.innerHTML` setter — replaces children with parsed HTML.
///
/// Records a `Mutation::SetInnerHtml` which is applied during
/// `session.flush()` so that any caller flushing through the session
/// (notably the boa runtime) keeps surfacing innerHTML changes as
/// `MutationRecord`s through the flush return value — required for
/// MutationObserver delivery and custom-element-reaction enqueueing
/// on that path. The VM-direct path in `vm/host/dom_inner_html.rs`
/// bypasses this handler and applies the algorithm synchronously
/// (calling [`elidex_script_session::apply_set_inner_html`]) before invoking
/// `Vm::deliver_mutation_records` itself, so the two paths share one
/// engine-indep algorithm without colliding on observer delivery.
pub struct SetInnerHtml;

impl DomApiHandler for SetInnerHtml {
    fn method_name(&self) -> &str {
        "innerHTML.set"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        _dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let html = match args.first() {
            Some(JsValue::String(s)) => s.clone(),
            _ => String::new(),
        };
        session.record_mutation(Mutation::SetInnerHtml { entity: this, html });
        Ok(JsValue::Undefined)
    }
}

/// `element.insertAdjacentHTML(position, text)` — parses HTML and inserts at position.
///
/// Position values: "beforebegin", "afterbegin", "beforeend", "afterend".
/// Uses the same fragment parser as innerHTML setter. `invoke` validates the
/// position and records a [`Mutation::InsertAdjacentHtml`]; the fragment parse
/// and the insertion happen when the session flushes (`session.flush()`), which
/// generates the `MutationRecord` for `MutationObserver`.
pub struct InsertAdjacentHtml;

impl DomApiHandler for InsertAdjacentHtml {
    fn method_name(&self) -> &str {
        "insertAdjacentHTML"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let position = match args.first() {
            Some(JsValue::String(s)) => s.to_ascii_lowercase(),
            _ => {
                return Err(DomApiError::syntax_error(
                    "insertAdjacentHTML requires a position string",
                ));
            }
        };
        let html = match args.get(1) {
            Some(JsValue::String(s)) => s.clone(),
            _ => String::new(),
        };

        // Validate position and parent requirement before recording.
        match position.as_str() {
            "beforebegin" | "afterend" => {
                if dom.get_parent(this).is_none() {
                    return Err(DomApiError {
                        kind: DomApiErrorKind::HierarchyRequestError,
                        message: format!(
                            "insertAdjacentHTML: element has no parent for {position}"
                        ),
                    });
                }
            }
            "afterbegin" | "beforeend" => {}
            _ => {
                return Err(DomApiError::syntax_error(
                    "Invalid position for insertAdjacentHTML",
                ));
            }
        }

        // Record mutation — applied during session.flush() with proper
        // MutationRecord generation for MutationObserver support.
        session.record_mutation(Mutation::InsertAdjacentHtml {
            entity: this,
            position,
            html,
        });

        Ok(JsValue::Undefined)
    }
}

/// `element.innerHTML` getter — serializes children to HTML.
pub struct GetInnerHtml;

impl DomApiHandler for GetInnerHtml {
    fn method_name(&self) -> &str {
        "innerHTML.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let html = serialize_inner_html(this, dom);
        Ok(JsValue::String(html))
    }
}

/// Serialize children of an entity to HTML using the default options
/// (matches plain `innerHTML` / `outerHTML` getter behaviour — shadow
/// roots are skipped to preserve encapsulation per WHATWG DOM §4.8).
pub fn serialize_inner_html(entity: Entity, dom: &EcsDom) -> String {
    serialize_inner_html_with_options(entity, dom, &SerializeOptions::default())
}

/// Serialize `entity` itself plus its descendants — opening tag with
/// attributes, recursive child serialization, closing tag. Shadow
/// roots remain skipped per default `SerializeOptions`; for `getHTML`
/// callers needing shadow visibility, the entity tag is serialized
/// here and the inner content is routed through
/// [`serialize_inner_html_with_options`] separately.
pub fn serialize_outer_html(entity: Entity, dom: &EcsDom) -> String {
    let mut html = String::new();
    serialize_node(entity, dom, &mut html, false, &SerializeOptions::default());
    html
}

/// Serialize an **arbitrary** node to its HTML-fragment markup, dispatching by
/// [`NodeKind`](elidex_ecs::NodeKind) — the engine-indep home for
/// `XMLSerializer.serializeToString(node)` (HTML §8.5.8, HTML-serialization /
/// boa parity) and any caller that must serialize a node of unknown kind:
///
/// - **`Document` / `DocumentFragment`** → the markup of its CHILDREN
///   ([`serialize_inner_html`]). These kinds have no tag of their own, so the
///   "outer" serialization is exactly their children's markup — this is what
///   makes the canonical `serializeToString(parseFromString(...))` round-trip
///   return real document markup rather than its concatenated text.
/// - **`Element`** → [`serialize_outer_html`] (the element + its subtree).
/// - **Text / `CDATASection` / Comment / `DocumentType`** → routed through
///   [`serialize_outer_html`] (whose internal node-kind dispatch emits escaped
///   text for character data, `<!--data-->` for a comment, and `<!DOCTYPE name>`
///   for a doctype). A bare entity with no DOM payload serializes to the empty
///   string.
#[must_use]
pub fn serialize_node_to_string(entity: Entity, dom: &EcsDom) -> String {
    use elidex_ecs::NodeKind;
    match dom.node_kind_inferred(entity) {
        // No tag of their own → serialize the markup of their children.
        Some(NodeKind::Document | NodeKind::DocumentFragment) => serialize_inner_html(entity, dom),
        // Element, character data, comment, doctype, or a payload-less entity:
        // `serialize_node` (reached via `serialize_outer_html`) handles each
        // kind by component dispatch.
        _ => serialize_outer_html(entity, dom),
    }
}

/// Options controlling shadow root visibility for [`serialize_inner_html_with_options`].
///
/// Per HTML §4.4.6 `Element.getHTML(options)` / `ShadowRoot.getHTML(options)`:
/// - `serializable_shadow_roots: true` emits a `<template shadowrootmode>`
///   element for any shadow root whose host is currently being serialized
///   AND whose `serializable` flag is set.
/// - `explicit_shadow_roots` is a set of shadow root entities that must
///   ALWAYS be emitted regardless of their `serializable` flag or mode
///   ("closed" included) — matches the spec's "regardless of whether or
///   not they are marked as serializable" clause.
#[derive(Default, Clone, Debug)]
pub struct SerializeOptions {
    /// When true, serialize each visited shadow root whose `serializable`
    /// flag is set as a declarative `<template shadowrootmode>` child of
    /// its host (HTML §13.3 Serializing HTML fragments, with shadow roots).
    pub serializable_shadow_roots: bool,
    /// Force-emit declarative templates for these shadow root entities
    /// regardless of their `serializable` flag. (HTML §4.4.6 places no
    /// `mode` restriction on either path — both `Open` and `Closed`
    /// shadows may be serialized when their host opted in.)
    pub explicit_shadow_roots: std::collections::HashSet<Entity>,
}

/// Serialize children of `entity` to HTML, honouring `opts` for shadow
/// root visibility. Equivalent to [`serialize_inner_html`] when `opts`
/// is the default.
///
/// Per HTML §13.3 Serializing HTML fragments, with shadow roots, when
/// `entity` is itself a shadow host whose shadow root should be
/// emitted (see [`SerializeOptions`]), the declarative
/// `<template shadowrootmode>` is written *first*, ahead of the
/// light-tree children. This ordering matches the spec's "shadow
/// roots are serialized as the first child of their host" rule and
/// lets the output round-trip when it is parsed under a parent, e.g.
/// `wrap.setHTMLUnsafe('<div>' + html + '</div>')`. `host.setHTMLUnsafe(
/// host.getHTML(opts))` itself yields a plain `<template>` child and no shadow
/// root: the output's top-level template is hostIsRoot (§13.2.6.4.4), so a
/// declarative template re-attaches only where a host element sits above it.
pub fn serialize_inner_html_with_options(
    entity: Entity,
    dom: &EcsDom,
    opts: &SerializeOptions,
) -> String {
    let mut html = String::new();
    emit_own_shadow_root_if_needed(entity, dom, &mut html, opts);
    let raw_text = dom
        .world()
        .get::<&TagType>(entity)
        .is_ok_and(|tag| RAW_TEXT_ELEMENTS.contains(&tag.0.as_str()));
    // HTML §13.3: a `<template>`'s serialized children are its *template
    // contents* (the detached fragment), not its (empty) light children.
    for child in dom.children_iter(serialization_children_root(entity, dom)) {
        serialize_node(child, dom, &mut html, raw_text, opts);
    }
    html
}

/// The node whose children the HTML fragment serialization algorithm
/// (HTML §13.3) walks for `entity`: a `<template>`'s content fragment ("let
/// the node also be current node's template contents"), else `entity` itself.
fn serialization_children_root(entity: Entity, dom: &EcsDom) -> Entity {
    dom.template_contents_fragment(entity).unwrap_or(entity)
}

/// HTML void elements that must not have a closing tag.
const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Returns `true` if the attribute name contains characters that would break
/// HTML serialization.
fn is_safe_attr_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b != b'"' && b != b'>' && b != b'<' && b != b'=' && !b.is_ascii_whitespace())
}

/// Decide whether to emit a given shadow root as a declarative template.
/// Per HTML §4.4.6 there is no mode-based filter (both `Open` and
/// `Closed` may be serialized whenever the host opted in): when the
/// caller-supplied `explicit_shadow_roots` set contains `sr` it is
/// always emitted; otherwise it is emitted iff the caller passed
/// `serializable_shadow_roots = true` AND the root carries the
/// `serializable` init flag.
fn shadow_root_should_emit(
    sr: Entity,
    sr_component: &elidex_ecs::ShadowRoot,
    opts: &SerializeOptions,
) -> bool {
    if opts.explicit_shadow_roots.contains(&sr) {
        return true;
    }
    opts.serializable_shadow_roots && sr_component.serializable
}

/// Shared helper: if `entity` is a shadow host and the host's shadow
/// root meets `opts`'s emission criteria, append a declarative
/// `<template shadowrootmode>` block describing it. Called both at
/// the [`serialize_inner_html_with_options`] entry (for the host the
/// caller asked about) and inside [`serialize_node`] (for descendant
/// shadow hosts encountered while walking the light tree).
fn emit_own_shadow_root_if_needed(
    entity: Entity,
    dom: &EcsDom,
    html: &mut String,
    opts: &SerializeOptions,
) {
    let Some(sr) = dom.get_shadow_root(entity) else {
        return;
    };
    let Ok(sr_component) = dom.world().get::<&elidex_ecs::ShadowRoot>(sr) else {
        return;
    };
    if !shadow_root_should_emit(sr, &sr_component, opts) {
        return;
    }
    let sr_copy: elidex_ecs::ShadowRoot = *sr_component;
    drop(sr_component);
    emit_shadow_root_template(sr, &sr_copy, dom, html, opts);
}

fn emit_shadow_root_template(
    sr: Entity,
    sr_component: &elidex_ecs::ShadowRoot,
    dom: &EcsDom,
    html: &mut String,
    opts: &SerializeOptions,
) {
    html.push_str("<template shadowrootmode=\"");
    html.push_str(match sr_component.mode {
        elidex_ecs::ShadowRootMode::Open => "open",
        elidex_ecs::ShadowRootMode::Closed => "closed",
    });
    html.push('"');
    if sr_component.delegates_focus {
        html.push_str(" shadowrootdelegatesfocus=\"\"");
    }
    if sr_component.clonable {
        html.push_str(" shadowrootclonable=\"\"");
    }
    if sr_component.serializable {
        html.push_str(" shadowrootserializable=\"\"");
    }
    // `shadowrootslotassignment` is an enumerated declarative attribute
    // (manual / named) that the parser hook honours; omit for the
    // default `Named` so round-tripping unchanged hosts stays terse.
    if matches!(
        sr_component.slot_assignment,
        elidex_ecs::SlotAssignmentMode::Manual
    ) {
        html.push_str(" shadowrootslotassignment=\"manual\"");
    }
    html.push('>');
    for child in dom.children_iter(sr) {
        serialize_node(child, dom, html, false, opts);
    }
    html.push_str("</template>");
}

fn serialize_node(
    entity: Entity,
    dom: &EcsDom,
    html: &mut String,
    in_raw_text: bool,
    opts: &SerializeOptions,
) {
    // `EcsDom::children_iter` already skips ShadowRoot entities, so
    // they never reach this function — shadow visibility is gated by
    // [`emit_own_shadow_root_if_needed`] on the host instead, which
    // handles both the outer entity (called from
    // [`serialize_inner_html_with_options`]) and descendant hosts
    // (called for every Element below).
    if let Ok(tc) = dom.world().get::<&TextContent>(entity) {
        if in_raw_text {
            html.push_str(&tc.0);
        } else {
            html.push_str(&escape_html(&tc.0));
        }
        return;
    }
    // HTML §13.3: a Comment node serializes as `<!--` + data + `-->`
    // (the data is emitted verbatim — the algorithm does no escaping).
    // Without this branch a comment (which carries `CommentData`, not
    // `TextContent`/`TagType`) would fall through to the child loop and
    // vanish from inner/outer serialization.
    if let Ok(comment) = dom.world().get::<&elidex_ecs::CommentData>(entity) {
        html.push_str("<!--");
        html.push_str(&comment.0);
        html.push_str("-->");
        return;
    }
    // HTML §13.3 DocumentType: `<!DOCTYPE ` + name + `>`. This is the COMPLETE
    // §13.3 output — the HTML fragment-serialization algorithm emits ONLY the
    // name (public/system identifiers are never serialized by §13.3; emitting
    // them is XML serialization, deferred to `#11-domparser-xml-real-parsing`).
    if let Ok(doctype) = dom.world().get::<&elidex_ecs::DocTypeData>(entity) {
        html.push_str("<!DOCTYPE ");
        html.push_str(&doctype.name);
        html.push('>');
        return;
    }
    if let Ok(tag) = dom.world().get::<&TagType>(entity) {
        html.push('<');
        html.push_str(&tag.0);
        // HTML §13.3 "Serializing HTML fragments": if the node's *is
        // value* is non-null and the element has no `is` attribute in
        // its attribute list, append ` is="..."` (the spec places this
        // BEFORE the attribute loop) — this is what lets a customized
        // built-in created via `createElement(tag, {is})` (which sets
        // NO `is` attribute per DOM §4.5) survive a serialize→parse
        // round-trip.  The sparse `CustomElementState` probe runs
        // first so the overwhelmingly common no-CE element pays one
        // failed lookup; the membership check is the spec condition
        // (`has_attribute`), independent of emission filtering.
        // `escape_attr` is load-bearing: the is value is an arbitrary
        // author string (DOM §4.9 step 6.3 imposes no validity), so
        // raw emission would inject markup.
        if let Ok(ce) = dom
            .world()
            .get::<&elidex_custom_elements::CustomElementState>(entity)
        {
            if let Some(is_value) = ce.is_value() {
                if !dom.has_attribute(entity, "is") {
                    html.push_str(" is=\"");
                    html.push_str(&escape_attr(is_value));
                    html.push('"');
                }
            }
        }
        if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
            let mut sorted: Vec<(&str, &str)> = attrs.iter().collect();
            sorted.sort_by_key(|(name, _)| *name);
            for (name, value) in sorted {
                if !is_safe_attr_name(name) {
                    continue;
                }
                html.push(' ');
                html.push_str(name);
                html.push_str("=\"");
                html.push_str(&escape_attr(value));
                html.push('"');
            }
        }
        html.push('>');
        if VOID_ELEMENTS.contains(&tag.0.as_str()) {
            return;
        }
        emit_own_shadow_root_if_needed(entity, dom, html, opts);
        let child_raw_text = RAW_TEXT_ELEMENTS.contains(&tag.0.as_str());
        // HTML §13.3: a `<template>` serializes its template contents (the
        // detached fragment), not its (empty) light children.
        for child in dom.children_iter(serialization_children_root(entity, dom)) {
            serialize_node(child, dom, html, child_raw_text, opts);
        }
        html.push_str("</");
        html.push_str(&tag.0);
        html.push('>');
        return;
    }
    for child in dom.children_iter(entity) {
        serialize_node(child, dom, html, false, opts);
    }
}
