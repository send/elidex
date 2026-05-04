//! `HTMLOptionElement.prototype` intrinsic — per-tag prototype layer
//! for `<option>` wrappers (HTML §4.10.10).
//!
//! Chain (slot #11-tags-T1 Phase 2):
//!
//! ```text
//! option wrapper
//!   → HTMLOptionElement.prototype  (this intrinsic)
//!     → HTMLElement.prototype
//!       → Element.prototype
//!         → Node.prototype
//!           → EventTarget.prototype
//!             → Object.prototype
//! ```
//!
//! Members installed here:
//!
//! - **`disabled`** — boolean reflect of the `disabled` content
//!   attribute.
//! - **`label`** — DOMString reflect of the `label` content
//!   attribute (note: per HTML §4.10.10 the IDL `label` getter falls
//!   back to `text` when the content attribute is absent — that
//!   fallback is implemented inline below rather than via the
//!   generic `string_reflect_*` helper).
//! - **`value`** — DOMString reflect of the `value` content
//!   attribute, falling back to `text` when the content attribute is
//!   absent (HTML §4.10.10).
//! - **`defaultSelected`** — boolean reflect of the `selected`
//!   content attribute.
//! - **`selected`** — current selectedness state.  Phase 2 reflects
//!   the `selected` content attribute as an approximation; the
//!   spec's separate "dirtiness" flag (which decouples runtime
//!   selectedness from the content attribute once the user has
//!   interacted with the parent select) lands with full
//!   HTMLSelectElement integration in Phase 7.
//! - **`text`** getter / setter — alias of `textContent` per HTML
//!   §4.10.10 (the getter additionally strips and collapses ASCII
//!   whitespace).
//! - **`index`** getter — the option's position within the parent
//!   `<select>`'s `options` list, walking option / optgroup
//!   descendants.  Returns 0 when there is no parent select (per
//!   HTML §4.10.10 — the option is treated as the first and only
//!   element of an implicit list of itself).
//! - **`form`** getter — the form association of the option's
//!   parent `<select>`, transitive through that select's form
//!   ancestor walk.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;

use elidex_ecs::{Entity, NodeKind};

const INTERFACE: &str = "HTMLOptionElement";

impl VmInner {
    /// Allocate `HTMLOptionElement.prototype` with
    /// `HTMLElement.prototype` as its parent.  Must run after
    /// `register_html_element_prototype`.
    pub(in crate::vm) fn register_html_option_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_option_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_option_prototype = Some(proto_id);

        // disabled: boolean reflect.
        self.install_accessor_pair(
            proto_id,
            self.well_known.disabled,
            native_option_get_disabled,
            Some(native_option_set_disabled),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // label: DOMString reflect with text fallback (no content
        // attr → returns option.text).
        self.install_accessor_pair(
            proto_id,
            self.well_known.label_attr,
            native_option_get_label,
            Some(native_option_set_label),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // value: DOMString reflect with text fallback.
        self.install_accessor_pair(
            proto_id,
            self.well_known.value,
            native_option_get_value,
            Some(native_option_set_value),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // defaultSelected: boolean reflect of `selected` content attr.
        self.install_accessor_pair(
            proto_id,
            self.well_known.default_selected,
            native_option_get_default_selected,
            Some(native_option_set_default_selected),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // selected: runtime selectedness (approximation reflects the
        // content attribute pending Phase 7 dirty-flag landing).
        self.install_accessor_pair(
            proto_id,
            self.well_known.selected,
            native_option_get_selected,
            Some(native_option_set_selected),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // text: textContent alias with whitespace normalisation on
        // read.
        self.install_accessor_pair(
            proto_id,
            self.well_known.text_attr,
            native_option_get_text,
            Some(native_option_set_text),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // index: read-only.
        self.install_accessor_pair(
            proto_id,
            self.well_known.index_attr,
            native_option_get_index,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // form: read-only, derived through parent select.
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_attr,
            native_option_get_form,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

/// Brand check for `<option>` receivers.
fn require_option_receiver(
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
    if !ctx.host().tag_matches_ascii_case(entity, "option") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on '{INTERFACE}': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// ---------------------------------------------------------------------------
// disabled
// ---------------------------------------------------------------------------

fn native_option_get_disabled(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_option_receiver(ctx, this, "disabled")? else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(
        ctx.host().dom().has_attribute(entity, "disabled"),
    ))
}

fn native_option_set_disabled(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_option_receiver(ctx, this, "disabled")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let flag = super::super::coerce::to_boolean(ctx.vm, val);
    if flag {
        ctx.host()
            .dom()
            .set_attribute(entity, "disabled", String::new());
    } else {
        super::element_attrs::attr_remove(ctx, entity, "disabled");
    }
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// label  (with `text` fallback per HTML §4.10.10)
// ---------------------------------------------------------------------------

fn native_option_get_label(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_option_receiver(ctx, this, "label")? else {
        return Ok(JsValue::String(empty));
    };
    // Fast path: explicit attribute present (including `""`).
    let attr = ctx
        .host()
        .dom()
        .with_attribute(entity, "label", |v| v.map(String::from));
    if let Some(s) = attr {
        let sid = if s.is_empty() {
            empty
        } else {
            ctx.vm.strings.intern(&s)
        };
        return Ok(JsValue::String(sid));
    }
    // Fallback: option.text — collected and whitespace-collapsed.
    let text = collect_option_text(ctx, entity);
    let sid = if text.is_empty() {
        empty
    } else {
        ctx.vm.strings.intern(&text)
    };
    Ok(JsValue::String(sid))
}

fn native_option_set_label(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_option_receiver(ctx, this, "label")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    ctx.host().dom().set_attribute(entity, "label", s);
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// value  (with `text` fallback per HTML §4.10.10)
// ---------------------------------------------------------------------------

fn native_option_get_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_option_receiver(ctx, this, "value")? else {
        return Ok(JsValue::String(empty));
    };
    let attr = ctx
        .host()
        .dom()
        .with_attribute(entity, "value", |v| v.map(String::from));
    if let Some(s) = attr {
        let sid = if s.is_empty() {
            empty
        } else {
            ctx.vm.strings.intern(&s)
        };
        return Ok(JsValue::String(sid));
    }
    let text = collect_option_text(ctx, entity);
    let sid = if text.is_empty() {
        empty
    } else {
        ctx.vm.strings.intern(&text)
    };
    Ok(JsValue::String(sid))
}

fn native_option_set_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_option_receiver(ctx, this, "value")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    ctx.host().dom().set_attribute(entity, "value", s);
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// defaultSelected  (boolean reflect of `selected` content attribute)
// ---------------------------------------------------------------------------

fn native_option_get_default_selected(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_option_receiver(ctx, this, "defaultSelected")? else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(
        ctx.host().dom().has_attribute(entity, "selected"),
    ))
}

fn native_option_set_default_selected(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_option_receiver(ctx, this, "defaultSelected")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let flag = super::super::coerce::to_boolean(ctx.vm, val);
    if flag {
        ctx.host()
            .dom()
            .set_attribute(entity, "selected", String::new());
    } else {
        super::element_attrs::attr_remove(ctx, entity, "selected");
    }
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// selected  (runtime selectedness — Phase 2 approximation; Phase 7
// adds the dirty-flag separation per HTML §4.10.10 selectedness algo).
// ---------------------------------------------------------------------------

fn native_option_get_selected(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_option_receiver(ctx, this, "selected")? else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(
        ctx.host().dom().has_attribute(entity, "selected"),
    ))
}

fn native_option_set_selected(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_option_receiver(ctx, this, "selected")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let flag = super::super::coerce::to_boolean(ctx.vm, val);
    // Phase 2 approximation: also writes the content attribute, which
    // a strict spec implementation would not do (the `selectedness`
    // and `dirty` flags are separate from content).  Phase 7 wires
    // FormControlState integration so this becomes a state-only
    // mutation on the parent select with the content attribute
    // reflecting only `defaultSelected`.
    if flag {
        ctx.host()
            .dom()
            .set_attribute(entity, "selected", String::new());
    } else {
        super::element_attrs::attr_remove(ctx, entity, "selected");
    }
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// text  (textContent alias)
// ---------------------------------------------------------------------------

fn native_option_get_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_option_receiver(ctx, this, "text")? else {
        return Ok(JsValue::String(empty));
    };
    let collected = collect_option_text(ctx, entity);
    if collected.is_empty() {
        return Ok(JsValue::String(empty));
    }
    let sid = ctx.vm.strings.intern(&collected);
    Ok(JsValue::String(sid))
}

fn native_option_set_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_option_receiver(ctx, this, "text")? else {
        return Ok(JsValue::Undefined);
    };
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let data: String = match arg {
        JsValue::Null => String::new(),
        other => {
            let sid = super::super::coerce::to_string(ctx.vm, other)?;
            ctx.vm.strings.get_utf8(sid)
        }
    };
    let dom = ctx.host().dom();
    // Replace all children with a single Text node carrying `data`.
    // Mirrors `native_node_set_text_content`'s Element branch.
    let existing: Vec<Entity> = dom.children_iter(entity).collect();
    for child in existing {
        let _ = dom.remove_child(entity, child);
    }
    if !data.is_empty() {
        let text_entity = dom.create_text(data);
        let _ = dom.append_child(entity, text_entity);
    }
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// index
// ---------------------------------------------------------------------------

/// HTML §4.10.10 `option.index` — the option's position in its
/// parent `<select>`'s options list (which flattens
/// `<option>` and `<optgroup><option>` descendants in document
/// order).  When the option has no parent select, the IDL specifies
/// the "list of an implicit list of itself" — i.e. index 0.
fn native_option_get_index(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_option_receiver(ctx, this, "index")? else {
        return Ok(JsValue::Number(0.0));
    };
    let select = enclosing_select(ctx, entity);
    let Some(sel) = select else {
        return Ok(JsValue::Number(0.0));
    };
    let mut idx: i32 = 0;
    let mut found: Option<i32> = None;
    visit_select_options(ctx, sel, |opt| {
        if opt == entity {
            found = Some(idx);
            return false;
        }
        idx += 1;
        true
    });
    Ok(JsValue::Number(f64::from(found.unwrap_or(0))))
}

// ---------------------------------------------------------------------------
// form
// ---------------------------------------------------------------------------

fn native_option_get_form(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_option_receiver(ctx, this, "form")? else {
        return Ok(JsValue::Null);
    };
    let Some(select) = enclosing_select(ctx, entity) else {
        return Ok(JsValue::Null);
    };
    // Resolve select's form association via the shared HTML
    // §4.10.18.3 walker.
    match super::form_assoc::resolve_form_association(ctx, select) {
        Some(f) => Ok(JsValue::Object(ctx.vm.create_element_wrapper(f))),
        None => Ok(JsValue::Null),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Walk up from `entity` looking for the nearest `<select>` ancestor
/// — used by both `index` and `form` getters.
fn enclosing_select(ctx: &mut NativeContext<'_>, entity: Entity) -> Option<Entity> {
    let dom = ctx.host().dom();
    let mut cur = dom.get_parent(entity);
    let mut depth: u32 = 0;
    while let Some(p) = cur {
        if depth > 1024 {
            return None;
        }
        if dom.has_tag(p, "select") {
            return Some(p);
        }
        cur = dom.get_parent(p);
        depth += 1;
    }
    None
}

/// Pre-order visitor over `<select>`'s options list.  Visits direct
/// `<option>` children plus `<option>` children of `<optgroup>`
/// children — mirroring HTMLOptionsCollection's resolve fn that
/// lands in Phase 7.  `visit` returns `false` to short-circuit.
fn visit_select_options(
    ctx: &mut NativeContext<'_>,
    select: Entity,
    mut visit: impl FnMut(Entity) -> bool,
) {
    let dom = ctx.host().dom();
    let mut child = dom.get_first_child(select);
    while let Some(c) = child {
        if dom.has_tag(c, "option") {
            if !visit(c) {
                return;
            }
        } else if dom.has_tag(c, "optgroup") {
            let mut og_child = dom.get_first_child(c);
            while let Some(oc) = og_child {
                if dom.has_tag(oc, "option") {
                    if !visit(oc) {
                        return;
                    }
                }
                og_child = dom.get_next_sibling(oc);
            }
        }
        child = dom.get_next_sibling(c);
    }
}

/// Collect the option's text per HTML §4.10.10.  Walks every Text
/// descendant in document order, concatenates their data, then
/// strips and collapses ASCII whitespace into single SPACEs.
fn collect_option_text(ctx: &mut NativeContext<'_>, option: Entity) -> String {
    let dom = ctx.host().dom();
    let mut buf = String::new();
    dom.traverse_descendants(option, |e| {
        if let Ok(text) = dom.world().get::<&elidex_ecs::TextContent>(e) {
            buf.push_str(&text.0);
        }
        true
    });
    strip_and_collapse_ascii_ws(&buf)
}

/// HTML "strip and collapse ASCII whitespace" per WHATWG Infra:
/// trim leading/trailing ASCII whitespace, then replace runs of
/// internal ASCII whitespace with a single SPACE.
fn strip_and_collapse_ascii_ws(s: &str) -> String {
    let trimmed = s.trim_matches(is_ascii_ws);
    let mut out = String::with_capacity(trimmed.len());
    let mut last_was_ws = false;
    for ch in trimmed.chars() {
        if is_ascii_ws(ch) {
            if !last_was_ws {
                out.push(' ');
                last_was_ws = true;
            }
        } else {
            out.push(ch);
            last_was_ws = false;
        }
    }
    out
}

#[inline]
fn is_ascii_ws(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\x0C' | '\r')
}
