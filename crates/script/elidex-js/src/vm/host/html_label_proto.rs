//! `HTMLLabelElement.prototype` intrinsic — per-tag prototype layer
//! for `<label>` wrappers (HTML §4.10.4).
//!
//! Chain (slot #11-tags-T1):
//!
//! ```text
//! label wrapper
//!   → HTMLLabelElement.prototype    (this intrinsic)
//!     → HTMLElement.prototype
//!       → Element.prototype
//!         → Node.prototype
//!           → EventTarget.prototype
//!             → Object.prototype
//! ```
//!
//! Members installed here:
//!
//! - **`htmlFor`** — DOMString reflect of the `for` content attribute
//!   (HTML §4.10.4 — IDL property is `htmlFor`, content attribute is
//!   the unprefixed `"for"` keyword).  Reads return the attribute
//!   value as-is or `""` when absent; writes coerce to string.
//! - **`control`** getter — returns the labelable element this label
//!   refers to, resolved per HTML §4.10.4 step 2: when `htmlFor` is
//!   set, look up the IDREF in the document; otherwise walk the
//!   label's own descendants for the first labelable element.
//!   Returns `null` when no labelable target is found.
//! - **`form`** getter — returns the label's associated form, derived
//!   from `control.form` (HTML §4.10.4: a label's form association
//!   is transitive through its control's form association).  Returns
//!   `null` when there is no resolved control or no enclosing form.
//!
//! Slot #11-tags-T1 (`m4-12-pr-html-form-control-prototypes-plan.md`)
//! Phase 1 — small triplet warm-up alongside HTMLOptGroupElement +
//! HTMLLegendElement.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;

use elidex_ecs::{Entity, NodeKind};

const INTERFACE: &str = "HTMLLabelElement";

impl VmInner {
    /// Allocate `HTMLLabelElement.prototype` with
    /// `HTMLElement.prototype` as its parent so
    /// `lbl instanceof HTMLElement === true` (HTML §3.2.8).
    /// Must run after `register_html_element_prototype`.
    pub(in crate::vm) fn register_html_label_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_label_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_label_prototype = Some(proto_id);

        // `htmlFor` — DOMString reflect of the `"for"` content
        // attribute.  Read/write pair following the same shape as
        // every WHATWG "DOMString reflect" attribute.
        self.install_accessor_pair(
            proto_id,
            self.well_known.html_for,
            native_label_get_html_for,
            Some(native_label_set_html_for),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // `control` getter — derived (no setter per spec).  Walks DOM
        // to resolve the labeled control either via the `for=` IDREF
        // or via descendant search.
        self.install_accessor_pair(
            proto_id,
            self.well_known.control,
            native_label_get_control,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // `form` getter — derived through `control.form`.
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_attr,
            native_label_get_form,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

/// Brand check for `<label>` receivers — rejects non-Element and
/// non-label tag entities with "Illegal invocation".
///
/// Mirrors `require_iframe_receiver` (vm/host/html_iframe_proto.rs);
/// the small-triplet phase intentionally inlines a per-element brand
/// check rather than factoring a generic helper, since a single
/// generic `require_tag_receiver(&[&str], iface)` would force every
/// call site through a slice + iter + ascii-case loop on the hot
/// accessor path.  The 1-tag specialised form folds to a single
/// `tag_matches_ascii_case` call and inlines through the call site
/// without any extra work for the common 1-tag receivers.
fn require_label_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) = super::event_target::require_receiver(ctx, this, INTERFACE, method, |k| {
        k == NodeKind::Element
    })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "label") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on '{INTERFACE}': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

/// `htmlFor` getter — reflects `for` content attribute.
fn native_label_get_html_for(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_label_receiver(ctx, this, "htmlFor")? else {
        return Ok(JsValue::String(empty));
    };
    let sid = match ctx.dom_and_strings_if_bound() {
        Some((dom, strings)) => {
            dom.with_attribute(entity, "for", |v| v.map_or(empty, |s| strings.intern(s)))
        }
        None => empty,
    };
    Ok(JsValue::String(sid))
}

/// `htmlFor` setter — coerces the argument to string and writes the
/// `for` content attribute.
fn native_label_set_html_for(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_label_receiver(ctx, this, "htmlFor")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    ctx.host().dom().set_attribute(entity, "for", s);
    Ok(JsValue::Undefined)
}

/// `control` getter — resolves the labeled control per HTML §4.10.4.
///
/// Resolution order:
/// 1. If the label's `for` attribute is set, the result is the first
///    labelable element with that ID **inside the same tree** (HTML
///    §4.10.4 step 2).  When no element matches the IDREF, the
///    result is `null`.
/// 2. Otherwise, the result is the **first descendant** that is a
///    labelable element (HTML §4.10.4 step 3 — pre-order traversal,
///    skipping nested labels per the same step's invariant).
///
/// "Labelable" elements per HTML §4.10.2: button, input (except
/// type=hidden), meter, output, progress, select, textarea.
fn native_label_get_control(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_label_receiver(ctx, this, "control")? else {
        return Ok(JsValue::Null);
    };
    let target = resolve_label_control(ctx, entity);
    match target {
        Some(t) => Ok(JsValue::Object(ctx.vm.create_element_wrapper(t))),
        None => Ok(JsValue::Null),
    }
}

/// `form` getter — derived through `control.form` (HTML §4.10.4
/// step 4).  Returns `null` when there is no resolved control or
/// when the resolved control has no enclosing form.
fn native_label_get_form(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_label_receiver(ctx, this, "form")? else {
        return Ok(JsValue::Null);
    };
    let Some(control) = resolve_label_control(ctx, entity) else {
        return Ok(JsValue::Null);
    };
    match find_form_ancestor_dom(ctx, control) {
        Some(form) => Ok(JsValue::Object(ctx.vm.create_element_wrapper(form))),
        None => Ok(JsValue::Null),
    }
}

/// Walk the label's `for=` IDREF first and then its own descendants,
/// returning the first labelable element encountered.
fn resolve_label_control(ctx: &mut NativeContext<'_>, label: Entity) -> Option<Entity> {
    // Step 1 — `for=` IDREF lookup.  Read the attribute as a borrowed
    // owned `String` (no intern needed because the lookup is by the
    // raw id text).
    let id_target = ctx.host().dom().with_attribute(label, "for", |v| {
        v.filter(|s| !s.is_empty()).map(String::from)
    });
    if let Some(id) = id_target {
        // Tree-scoped id lookup: per HTML §4.10.4 step 2, the search
        // is rooted at the label's tree (the document if attached,
        // otherwise the disconnected subtree's root).  Use
        // `owner_document` first; fall back to walking up to the
        // tree root for detached labels so a disconnected
        // `<form><label for=x>` still resolves.
        let dom = ctx.host().dom();
        let root = dom.owner_document(label).unwrap_or_else(|| {
            // Climb to the topmost ancestor.  Bounded by the same
            // 1024-step depth guard used elsewhere; pathological
            // depths are already a bug in the producer.
            let mut cur = label;
            let mut depth: u32 = 0;
            while let Some(p) = dom.get_parent(cur) {
                if depth > 1024 {
                    break;
                }
                cur = p;
                depth += 1;
            }
            cur
        });
        if let Some(target) = dom.find_by_id(root, &id) {
            if is_labelable(ctx, target) {
                return Some(target);
            }
        }
        // IDREF that fails to match a labelable element returns
        // `null` per spec — do NOT fall through to descendant
        // search, that path is only used when `for=` is absent.
        return None;
    }

    // Step 2 — descendant search.  Pre-order traversal, first
    // labelable wins.  Skip nested labels (the spec's "scope" rule
    // would prevent a labelable inside a nested label from being
    // associated with the outer label).
    first_labelable_descendant(ctx, label)
}

/// Pre-order DFS for the first labelable descendant of `root`.
/// Skips traversing into nested `<label>` subtrees so they do not
/// "claim" elements past the inner label boundary.
fn first_labelable_descendant(ctx: &mut NativeContext<'_>, root: Entity) -> Option<Entity> {
    // Iterative stack-based traversal — recursion across the borrow-
    // checker boundary requires re-entering `ctx.host().dom()` per
    // step anyway, and a small explicit stack keeps the DOM borrow
    // scoped tight to single-step accesses.
    let mut stack: Vec<Entity> = Vec::new();
    let mut child = ctx.host().dom().get_first_child(root);
    while let Some(c) = child {
        stack.push(c);
        child = ctx.host().dom().get_next_sibling(c);
    }
    // Reverse so the leftmost child is on top — pop yields pre-order.
    stack.reverse();
    while let Some(node) = stack.pop() {
        if is_labelable(ctx, node) {
            return Some(node);
        }
        // Skip nested <label> subtrees (HTML §4.10.4 — labels are
        // categorised non-recursively when computing the implicit
        // associated control).
        if ctx.host().tag_matches_ascii_case(node, "label") {
            continue;
        }
        // Push children (rightmost first so leftmost pops first).
        let mut child_stack: Vec<Entity> = Vec::new();
        let mut c = ctx.host().dom().get_first_child(node);
        while let Some(ch) = c {
            child_stack.push(ch);
            c = ctx.host().dom().get_next_sibling(ch);
        }
        for ch in child_stack.into_iter().rev() {
            stack.push(ch);
        }
    }
    None
}

/// HTML §4.10.2 labelable element categorisation.  `<input
/// type=hidden>` is intentionally excluded.
fn is_labelable(ctx: &mut NativeContext<'_>, entity: Entity) -> bool {
    // Tag-name check first; for `<input>`, additionally exclude
    // `type=hidden` per spec.
    let host = ctx.host();
    let dom = host.dom();
    let tag = match dom.get_tag_name(entity) {
        Some(t) => t,
        None => return false,
    };
    let lower = tag.to_ascii_lowercase();
    match lower.as_str() {
        "button" | "meter" | "output" | "progress" | "select" | "textarea" => true,
        "input" => {
            let ty = dom.with_attribute(entity, "type", |v| {
                v.map(|s| s.to_ascii_lowercase()).unwrap_or_default()
            });
            ty != "hidden"
        }
        _ => false,
    }
}

/// Walk `entity`'s ancestor chain for the nearest `<form>` element.
/// Mirrors `elidex_form::submit::find_form_ancestor` semantics but
/// avoids pulling that crate in for Phase 1 — Phase 4 (HTMLFormElement)
/// will switch to the elidex-form helper once the dep lands.
fn find_form_ancestor_dom(ctx: &mut NativeContext<'_>, entity: Entity) -> Option<Entity> {
    let dom = ctx.host().dom();
    let mut cur = dom.get_parent(entity);
    let mut depth: u32 = 0;
    while let Some(p) = cur {
        if depth > 1024 {
            // Defensive bound — runaway parents indicate a cycle or
            // pathological tree depth that is already a bug elsewhere.
            return None;
        }
        if dom.has_tag(p, "form") {
            return Some(p);
        }
        cur = dom.get_parent(p);
        depth += 1;
    }
    None
}
