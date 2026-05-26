//! `CSSStyleDeclaration.prototype` intrinsic + `Element.style` accessor +
//! `window.getComputedStyle` + `CSS` namespace plumbing (CSSOM §6.6 / §6.7 /
//! §7.2).
//!
//! Thin binding to the engine-independent `elidex_dom_api::style` and
//! `elidex_dom_api::computed_style` and `elidex_dom_api::css_namespace`
//! handlers.  Every native body in this file is a single
//! [`invoke_dom_api`] dispatch — no DOM mutation algorithms, declaration
//! parsing, or CSS-OM serialisation lives here, per the CLAUDE.md
//! Layering mandate.
//!
//! ## Backing state
//!
//! [`ObjectKind::CSSStyleDeclaration`] carries `(source, key_bits)` inline:
//! - `source = 0` (Inline): `key_bits` = owner Entity bits, mutable, interned
//!   per Entity under `WrapperKind::InlineStyle` so
//!   `el.style === el.style` (CSSOM §6.6 `[SameObject]`).
//! - `source = 1` (Computed): `key_bits` = owner Entity bits, read-only,
//!   freshly allocated on each `getComputedStyle` call (matches WPT — the
//!   resolved-value declaration block does NOT preserve identity across
//!   calls).
//!
//! ## Source-aware dispatch
//!
//! The accessor / method natives consult `source` on the receiver kind:
//! - Inline `getPropertyValue` → `style.getPropertyValue` handler
//! - Computed `getPropertyValue` → `getComputedStyle` handler (resolved value)
//! - Inline `setProperty` / `removeProperty` / `cssText.set` → mutating
//!   handlers
//! - Computed mutators are silent no-ops in PR-A (read-only declaration
//!   block; observable strict-mode TypeError on mutation deferred to slot
//!   `#11-style-readonly-strict-throw`)
//! - Inline `length` / `item` / `cssText.get` → `style.*` handlers
//! - Computed `length` / `item` / `cssText.get` return `0` / `""` / `""` —
//!   per-property enumeration of `ComputedStyle` is deferred to slot
//!   `#11-computed-style-enumeration`; PR-A only exposes `getPropertyValue`
//!   for the Computed source which covers the dominant framework usage
//!   (`getComputedStyle(el).color`).
//!
//! ## Indexed-property exotic
//!
//! `style[i]` (CSSOM §6.6.1 indexed getter) returns the `i`th declared
//! property name for Inline source; empty string for Computed (per the
//! deferred-enumeration disclaimer above).  Implemented in
//! [`try_indexed_get`].
//!
//! ## Named-property exotic
//!
//! `style.color = "red"` ↔ `style.setProperty("color", "red")` and
//! `style.color` ↔ `style.getPropertyValue("color")` (CSSOM §6.6.1 named
//! getter / setter).  Per design review IMP-1/2: named-exotic [[Set]]
//! routes to the raw `setProperty` handler bypassing
//! `parse_declaration_block`, so unknown property names like
//! `style.foo = "bar"` write verbatim (matches Chrome).

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};
use super::super::wrapper_intern::{WrapperKey, WrapperKind};
use super::super::{NativeFn, VmInner};
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api};
use super::named_property_exotic::{coerce_key_or_none, is_bound, key_on_prototype_chain};

use elidex_ecs::Entity;

/// Inline source — mutable, backed by `InlineStyle` ECS, identity-cached.
const SOURCE_INLINE: u8 = 0;
/// Computed source — read-only, backed by `ComputedStyle` ECS, fresh-alloc.
const SOURCE_COMPUTED: u8 = 1;
/// Decoded receiver — covers both `ObjectKind::CSSStyleDeclaration`
/// (source 0/1) and `ObjectKind::CSSRuleStyleDeclaration` (Rule source,
/// keyed by `(sheet, rule_id)` instead of a single `Entity`).  Rule
/// source is read-only in PR-B; mutators are silent no-ops (deferred
/// to slot `#11-css-rule-style-mutation` — write-back requires
/// Selector + Declaration serialisers to round-trip the rule's
/// `source_text`).
#[derive(Clone, Copy)]
pub(super) enum StyleReceiver {
    Inline(Entity),
    Computed(Entity),
    Rule { sheet: Entity, rule_id: u64 },
}

impl VmInner {
    /// Allocate `CSSStyleDeclaration.prototype` chained to
    /// `Object.prototype`.  Must run after `register_object_prototype`.
    /// Carries `length` / `cssText` / `parentRule` accessors and the
    /// `item` / `getPropertyValue` / `getPropertyPriority` /
    /// `setProperty` / `removeProperty` methods.
    pub(in crate::vm) fn register_css_style_declaration_prototype(&mut self) {
        let obj_proto = self.object_prototype;
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: obj_proto,
            extensible: true,
        });
        self.css_style_declaration_prototype = Some(proto_id);

        // `length` (RO) / `cssText` (RW) / `parentRule` (RO) accessors.
        self.install_accessor_pair(
            proto_id,
            self.well_known.length,
            native_style_length_get,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.css_text,
            native_style_css_text_get,
            Some(native_style_css_text_set),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.parent_rule,
            native_style_parent_rule_get,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // Methods.
        for (name_sid, func) in [
            (self.well_known.item, native_style_item as NativeFn),
            (
                self.well_known.get_property_value,
                native_style_get_property_value,
            ),
            (
                self.well_known.get_property_priority,
                native_style_get_property_priority,
            ),
            (self.well_known.set_property, native_style_set_property),
            (
                self.well_known.remove_property,
                native_style_remove_property,
            ),
        ] {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }
    }

    /// Allocate (or return cached) Inline `CSSStyleDeclaration` wrapper for
    /// `owner`.  CSSOM §6.6 `[SameObject]`: `el.style === el.style`.
    pub(crate) fn alloc_or_cached_style(&mut self, owner: Entity) -> ObjectId {
        self.intern_wrapper(WrapperKey::entity(owner, WrapperKind::InlineStyle), |vm| {
            let proto = vm
                .css_style_declaration_prototype
                .expect("alloc_or_cached_style before register_css_style_declaration_prototype");
            vm.alloc_object(Object {
                kind: ObjectKind::CSSStyleDeclaration {
                    source: SOURCE_INLINE,
                    key_bits: owner.to_bits().get(),
                },
                storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                prototype: Some(proto),
                extensible: false,
            })
        })
    }

    /// Allocate a *fresh* read-only `CSSStyleDeclaration` wrapper for
    /// `owner` (Computed source).  Not cached — CSSOM §7.2 / WPT specify
    /// that each `getComputedStyle` call returns a new declaration block
    /// (identity is NOT preserved across reads).
    pub(crate) fn alloc_computed_style(&mut self, owner: Entity) -> ObjectId {
        let proto = self
            .css_style_declaration_prototype
            .expect("alloc_computed_style before register_css_style_declaration_prototype");
        self.alloc_object(Object {
            kind: ObjectKind::CSSStyleDeclaration {
                source: SOURCE_COMPUTED,
                key_bits: owner.to_bits().get(),
            },
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: false,
        })
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

/// Brand-check the receiver and recover the decoded
/// [`StyleReceiver`] — accepts both the PR-A unified
/// `CSSStyleDeclaration` (Inline / Computed sources) and the PR-B
/// `CSSRuleStyleDeclaration` (Rule source) variants.  Throws
/// `TypeError` on any other receiver (`Illegal invocation`) and on
/// stale entity bits (the wrapper survived a re-bind into a different
/// `EcsDom` world).  Post-unbind safe-defaults are checked separately
/// by each native via `ctx.host_if_bound()` after this brand check.
fn require_style_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<StyleReceiver, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'CSSStyleDeclaration': Illegal invocation"
        )));
    };
    match &ctx.vm.get_object(id).kind {
        ObjectKind::CSSStyleDeclaration { source, key_bits } => {
            let entity = Entity::from_bits(*key_bits).ok_or_else(|| {
                VmError::type_error(format!(
                    "Failed to execute '{method}' on 'CSSStyleDeclaration': stale entity"
                ))
            })?;
            match *source {
                SOURCE_INLINE => Ok(StyleReceiver::Inline(entity)),
                SOURCE_COMPUTED => Ok(StyleReceiver::Computed(entity)),
                _ => Err(VmError::type_error(format!(
                    "Failed to execute '{method}' on 'CSSStyleDeclaration': unknown source"
                ))),
            }
        }
        ObjectKind::CSSRuleStyleDeclaration {
            sheet_entity_bits,
            rule_id,
        } => {
            let sheet = Entity::from_bits(*sheet_entity_bits).ok_or_else(|| {
                VmError::type_error(format!(
                    "Failed to execute '{method}' on 'CSSStyleDeclaration': stale entity"
                ))
            })?;
            Ok(StyleReceiver::Rule {
                sheet,
                rule_id: *rule_id,
            })
        }
        _ => Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'CSSStyleDeclaration': Illegal invocation"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Method natives
// ---------------------------------------------------------------------------

fn native_style_length_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let recv = require_style_receiver(ctx, this, "length")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(0.0));
    }
    match recv {
        StyleReceiver::Inline(entity) => invoke_dom_api(ctx, "style.length", entity, &[]),
        StyleReceiver::Computed(_) => Ok(JsValue::Number(0.0)),
        StyleReceiver::Rule { sheet, rule_id } => invoke_dom_api(
            ctx,
            "rule.style.length",
            sheet,
            &[super::cssom_sheet::rule_id_to_js(rule_id)],
        ),
    }
}

fn native_style_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let recv = require_style_receiver(ctx, this, "item")?;
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    // WebIDL §3.10.13 indexed getter (`unsigned long`) → ToUint32.
    let idx = super::super::coerce::to_uint32(ctx.vm, arg)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    match recv {
        StyleReceiver::Inline(entity) => invoke_dom_api(
            ctx,
            "style.item",
            entity,
            &[JsValue::Number(f64::from(idx))],
        ),
        StyleReceiver::Computed(_) => Ok(JsValue::String(ctx.vm.well_known.empty)),
        StyleReceiver::Rule { sheet, rule_id } => invoke_dom_api(
            ctx,
            "rule.style.item",
            sheet,
            &[
                super::cssom_sheet::rule_id_to_js(rule_id),
                JsValue::Number(f64::from(idx)),
            ],
        ),
    }
}

fn native_style_css_text_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let recv = require_style_receiver(ctx, this, "cssText")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    match recv {
        StyleReceiver::Inline(entity) => invoke_dom_api(ctx, "style.cssText.get", entity, &[]),
        StyleReceiver::Computed(_) => Ok(JsValue::String(ctx.vm.well_known.empty)),
        StyleReceiver::Rule { sheet, rule_id } => invoke_dom_api(
            ctx,
            "rule.style.cssText.get",
            sheet,
            &[super::cssom_sheet::rule_id_to_js(rule_id)],
        ),
    }
}

fn native_style_css_text_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let recv = require_style_receiver(ctx, this, "cssText")?;
    let sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    match recv {
        StyleReceiver::Inline(entity) => {
            invoke_dom_api(ctx, "style.cssText.set", entity, &[JsValue::String(sid)])
        }
        // Computed / Rule sources: silent no-op (read-only declaration
        // blocks).  Strict-mode TypeError surfacing deferred — slot
        // `#11-style-readonly-strict-throw` (Computed) /
        // `#11-css-rule-style-mutation` (Rule).
        StyleReceiver::Computed(_) | StyleReceiver::Rule { .. } => Ok(JsValue::Undefined),
    }
}

fn native_style_parent_rule_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // `parentRule` returns the owning `CSSStyleRule` for Rule-source
    // declarations (CSSOM §6.6.5) and `null` for Inline / Computed
    // declaration blocks.  Brand-check the receiver so
    // `parentRule.call({})` throws the same `Illegal invocation`
    // TypeError as the other accessors / methods.
    let recv = require_style_receiver(ctx, this, "parentRule")?;
    match recv {
        StyleReceiver::Inline(_) | StyleReceiver::Computed(_) => Ok(JsValue::Null),
        StyleReceiver::Rule { sheet, rule_id } => {
            let id = ctx.vm.alloc_or_cached_css_style_rule(sheet, rule_id);
            Ok(JsValue::Object(id))
        }
    }
}

fn native_style_get_property_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let recv = require_style_receiver(ctx, this, "getPropertyValue")?;
    let sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    match recv {
        StyleReceiver::Inline(entity) => invoke_dom_api(
            ctx,
            "style.getPropertyValue",
            entity,
            &[JsValue::String(sid)],
        ),
        StyleReceiver::Computed(entity) => {
            invoke_dom_api(ctx, "getComputedStyle", entity, &[JsValue::String(sid)])
        }
        StyleReceiver::Rule { sheet, rule_id } => invoke_dom_api(
            ctx,
            "rule.style.getPropertyValue",
            sheet,
            &[
                super::cssom_sheet::rule_id_to_js(rule_id),
                JsValue::String(sid),
            ],
        ),
    }
}

fn native_style_get_property_priority(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // `getPropertyPriority` requires per-property `!important` tracking
    // on `InlineStyle`.  Deferred to slot `#11-style-important-priority`
    // (see plan §A-1).  PR-A returns the empty string for every property
    // (CSSOM §6.6.1 — empty string ⇒ "not !important"), which keeps the
    // method shape callable for framework feature-detect.
    let _ = require_style_receiver(ctx, this, "getPropertyPriority")?;
    Ok(JsValue::String(ctx.vm.well_known.empty))
}

fn native_style_set_property(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let recv = require_style_receiver(ctx, this, "setProperty")?;
    let prop_sid = coerce_first_arg_to_string_id(ctx, args)?;
    let val_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let val_sid = super::super::coerce::to_string(ctx.vm, val_arg)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    match recv {
        StyleReceiver::Inline(entity) => invoke_dom_api(
            ctx,
            "style.setProperty",
            entity,
            &[JsValue::String(prop_sid), JsValue::String(val_sid)],
        ),
        // Computed / Rule sources: silent no-op (read-only).
        StyleReceiver::Computed(_) | StyleReceiver::Rule { .. } => Ok(JsValue::Undefined),
    }
}

fn native_style_remove_property(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let recv = require_style_receiver(ctx, this, "removeProperty")?;
    let sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    match recv {
        StyleReceiver::Inline(entity) => {
            invoke_dom_api(ctx, "style.removeProperty", entity, &[JsValue::String(sid)])
        }
        StyleReceiver::Computed(_) | StyleReceiver::Rule { .. } => {
            Ok(JsValue::String(ctx.vm.well_known.empty))
        }
    }
}

// ---------------------------------------------------------------------------
// Indexed-property exotic — dispatched from `ops_element::get_element`.
// ---------------------------------------------------------------------------

/// `style[i]` (CSSOM §6.6.1 indexed getter) — returns the `i`th declared
/// property name for Inline source, empty string for Computed (per the
/// deferred-enumeration disclaimer in module docs).  Returns `None` for
/// non-numeric / non-canonical-string keys so `.length` / `.cssText`
/// resolve via the prototype chain.
pub(crate) fn try_indexed_get(
    vm: &mut VmInner,
    id: ObjectId,
    key: JsValue,
) -> Option<Result<JsValue, VmError>> {
    let recv = decode_receiver(vm, id)?;
    let idx_u32 = match key {
        JsValue::Number(n) if n.is_finite() => {
            // Exact-integer round-trip per ECMA-262 §7.1.22
            // canonical-numeric-index-string — same gate as
            // `class_list::try_indexed_get`.
            if !(n >= 0.0 && n <= f64::from(u32::MAX - 1)) {
                return None;
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let idx = n as u32;
            if f64::from(idx) != n {
                return None;
            }
            idx
        }
        JsValue::String(sid) => {
            let units = vm.strings.get(sid);
            super::super::coerce_format::parse_array_index_u32(units)?
        }
        _ => return None,
    };
    if !vm
        .host_data
        .as_deref()
        .is_some_and(super::super::host_data::HostData::is_bound)
    {
        // Post-unbind: indexed access on a retained wrapper returns the
        // empty string (out-of-range token) — matches the empty-block
        // behaviour for any index.  All sources share this default.
        return Some(Ok(JsValue::String(vm.well_known.empty)));
    }
    let mut ctx = NativeContext::new_call(vm);
    let result = match recv {
        StyleReceiver::Inline(entity) => invoke_dom_api(
            &mut ctx,
            "style.item",
            entity,
            &[JsValue::Number(f64::from(idx_u32))],
        ),
        StyleReceiver::Computed(_) => Ok(JsValue::String(ctx.vm.well_known.empty)),
        StyleReceiver::Rule { sheet, rule_id } => invoke_dom_api(
            &mut ctx,
            "rule.style.item",
            sheet,
            &[
                super::cssom_sheet::rule_id_to_js(rule_id),
                JsValue::Number(f64::from(idx_u32)),
            ],
        ),
    };
    Some(result)
}

// ---------------------------------------------------------------------------
// Named-property exotic — dispatched from `ops_element::get_element`,
// `ops_property::set_property_val`, `ops_property::try_delete_property`.
// ---------------------------------------------------------------------------

/// Decode a `[[Get]]` / `[[Set]]` / `[[Delete]]` receiver — accepts
/// both `CSSStyleDeclaration` (Inline / Computed) and
/// `CSSRuleStyleDeclaration` (Rule) variants.  Returns `None` when the
/// receiver is neither (so the named-property exotic falls through to
/// ordinary [[Get]]).
fn decode_receiver(vm: &VmInner, id: ObjectId) -> Option<StyleReceiver> {
    match vm.get_object(id).kind {
        ObjectKind::CSSStyleDeclaration { source, key_bits } => {
            let entity = Entity::from_bits(key_bits)?;
            match source {
                SOURCE_INLINE => Some(StyleReceiver::Inline(entity)),
                SOURCE_COMPUTED => Some(StyleReceiver::Computed(entity)),
                _ => None,
            }
        }
        ObjectKind::CSSRuleStyleDeclaration {
            sheet_entity_bits,
            rule_id,
        } => {
            let sheet = Entity::from_bits(sheet_entity_bits)?;
            Some(StyleReceiver::Rule { sheet, rule_id })
        }
        _ => None,
    }
}

/// Whether `sid` is a canonical numeric-index string per ECMA-262 §7.1.22
/// (an integer in `[0, 2^32-2]` whose `ToString` round-trips).  Used
/// to peel off `style[0]` / `style["0"]` shaped writes / deletes from
/// the named-property exotic so they fall through to the indexed-
/// property path (which is read-only on a non-extensible legacy
/// platform object).
fn is_canonical_numeric_index_key(vm: &VmInner, sid: super::super::value::StringId) -> bool {
    super::super::coerce_format::parse_array_index_u32(vm.strings.get(sid)).is_some()
}

/// `[[Get]]` trap (CSSOM §6.6.1 named getter).  `style.color` resolves
/// to the value of the `color` property.  Returns `None` for prototype-
/// chain hits (so `style.length` / `style.cssText` / `style.setProperty`
/// resolve through the accessor / method chain) and Symbol keys.
pub(crate) fn try_get(
    vm: &mut VmInner,
    id: ObjectId,
    key: JsValue,
) -> Option<Result<JsValue, VmError>> {
    let recv = decode_receiver(vm, id)?;
    let sid = match coerce_key_or_none(vm, key)? {
        Ok(sid) => sid,
        Err(e) => return Some(Err(e)),
    };
    if key_on_prototype_chain(vm, id, sid) {
        return None;
    }
    if !is_bound(vm) {
        // Post-unbind: CSSOM §6.6.1 named getter is spec-shaped to
        // always return a string (empty for absent), so a retained
        // `el.style.color` after `Vm::unbind()` must still surface `""`
        // — falling through to ordinary [[Get]] would resolve to
        // `undefined` because the wrapper carries no own `color` data
        // property.  This differs from `dataset.try_get`'s post-unbind
        // fall-through (DOMStringMap is allowed to fall through to the
        // prototype chain since `dataset.foo` is not spec-shaped to a
        // string).
        return Some(Ok(JsValue::String(vm.well_known.empty)));
    }
    let mut ctx = NativeContext::new_call(vm);
    let result = match recv {
        StyleReceiver::Inline(entity) => invoke_dom_api(
            &mut ctx,
            "style.getPropertyValue",
            entity,
            &[JsValue::String(sid)],
        ),
        StyleReceiver::Computed(entity) => invoke_dom_api(
            &mut ctx,
            "getComputedStyle",
            entity,
            &[JsValue::String(sid)],
        ),
        StyleReceiver::Rule { sheet, rule_id } => invoke_dom_api(
            &mut ctx,
            "rule.style.getPropertyValue",
            sheet,
            &[
                super::cssom_sheet::rule_id_to_js(rule_id),
                JsValue::String(sid),
            ],
        ),
    };
    Some(result)
}

/// `[[Set]]` trap (CSSOM §6.6.1 named setter).  Symbol keys fall through;
/// string / numeric keys route to `setProperty`.  Per design review IMP-1/2:
/// uses the **raw** `setProperty` handler — does NOT route through
/// `parse_declaration_block`, so `style.foo = "bar"` writes verbatim
/// (matches Chrome).  Computed source is a silent no-op (read-only block).
pub(crate) fn try_set(
    vm: &mut VmInner,
    id: ObjectId,
    key: JsValue,
    value: JsValue,
) -> Option<Result<(), VmError>> {
    let recv = decode_receiver(vm, id)?;
    let key_sid = match coerce_key_or_none(vm, key)? {
        Ok(sid) => sid,
        Err(e) => return Some(Err(e)),
    };
    // CSSOM §6.6.1 indexed properties are read-only — `style[0] = "x"`
    // must NOT be redirected to `setProperty("0", "x")` (which would
    // create a CSS property named "0").  Fall through to ordinary
    // [[Set]]; the wrapper is non-extensible, so the ordinary path
    // rejects the write at the spec-correct layer.
    if is_canonical_numeric_index_key(vm, key_sid) {
        return None;
    }
    if key_on_prototype_chain(vm, id, key_sid) {
        return None;
    }
    let val_sid = match super::super::coerce::to_string(vm, value) {
        Ok(sid) => sid,
        Err(e) => return Some(Err(e)),
    };
    if !is_bound(vm) {
        return Some(Ok(()));
    }
    let mut ctx = NativeContext::new_call(vm);
    let result = match recv {
        StyleReceiver::Inline(entity) => invoke_dom_api(
            &mut ctx,
            "style.setProperty",
            entity,
            &[JsValue::String(key_sid), JsValue::String(val_sid)],
        ),
        // Computed / Rule sources: silent no-op (read-only block).
        StyleReceiver::Computed(_) | StyleReceiver::Rule { .. } => Ok(JsValue::Undefined),
    };
    Some(result.map(|_| ()))
}

/// `[[Delete]]` trap.  String / numeric keys route to `removeProperty`;
/// Symbol keys / non-string-coercible keys fall through.  Computed and
/// Rule sources are silent no-ops (mirror `try_set`).
pub(crate) fn try_delete(
    vm: &mut VmInner,
    id: ObjectId,
    key: JsValue,
) -> Option<Result<bool, VmError>> {
    let recv = decode_receiver(vm, id)?;
    let key_sid = match coerce_key_or_none(vm, key)? {
        Ok(sid) => sid,
        Err(e) => return Some(Err(e)),
    };
    // CSSOM §6.6.1 indexed properties are not deletable.
    if is_canonical_numeric_index_key(vm, key_sid) {
        return None;
    }
    if key_on_prototype_chain(vm, id, key_sid) {
        return None;
    }
    if !is_bound(vm) {
        return Some(Ok(true));
    }
    let mut ctx = NativeContext::new_call(vm);
    let result = match recv {
        StyleReceiver::Inline(entity) => invoke_dom_api(
            &mut ctx,
            "style.removeProperty",
            entity,
            &[JsValue::String(key_sid)],
        ),
        StyleReceiver::Computed(_) | StyleReceiver::Rule { .. } => Ok(JsValue::Undefined),
    };
    Some(result.map(|_| true))
}

// ---------------------------------------------------------------------------
// HTMLElement.prototype.style accessor
// ---------------------------------------------------------------------------

/// `HTMLElement.prototype.style` getter — return an identity-cached
/// Inline `CSSStyleDeclaration` wrapper backed by the element's
/// `InlineStyle` ECS component (CSSOM §6.6).  Repeated reads return the
/// same `ObjectId` via [`VmInner::alloc_or_cached_style`].
pub(super) fn native_html_element_get_style(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity =
        super::event_target::require_receiver(ctx, this, "HTMLElement", "style", |kind| {
            matches!(kind, elidex_ecs::NodeKind::Element)
        })?
        .ok_or_else(|| {
            VmError::type_error("Failed to execute 'style' on 'HTMLElement': Illegal invocation")
        })?;
    let id = ctx.vm.alloc_or_cached_style(entity);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// window.getComputedStyle — global function on Window.prototype
// ---------------------------------------------------------------------------

/// `window.getComputedStyle(element[, pseudoElt])` (CSSOM §7.2).  Returns
/// a fresh read-only `CSSStyleDeclaration` (Computed source) wrapper for
/// `element`.  The `pseudoElt` arg is accepted but ignored — pseudo-element
/// computed-value resolution is deferred (no current consumer queries
/// pseudo styles via this API).
pub(super) fn native_window_get_computed_style(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(arg_id) = arg else {
        return Err(VmError::type_error(
            "Failed to execute 'getComputedStyle' on 'Window': parameter 1 is not an Element",
        ));
    };
    let entity = match ctx.vm.get_object(arg_id).kind {
        ObjectKind::HostObject { entity_bits } => {
            Entity::from_bits(entity_bits).ok_or_else(|| {
                VmError::type_error(
                    "Failed to execute 'getComputedStyle' on 'Window': stale element",
                )
            })?
        }
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'getComputedStyle' on 'Window': parameter 1 is not an Element",
            ));
        }
    };
    // WebIDL signature `getComputedStyle(Element elt, ...)` — `HostObject`
    // is shared with Text / Comment / Document / Window wrappers, so a
    // brand-only check would let `getComputedStyle(textNode)` pass with a
    // misleading "not an Element" message after the fact.  Reject any
    // non-Element NodeKind here while the bound DOM is still in scope;
    // post-unbind the entity has no observable kind so we let it through
    // (the wrapper is stale anyway and the resulting CSSStyleDeclaration
    // is read-only).
    if let Some(hd) = ctx.host_if_bound() {
        if !matches!(
            hd.dom().node_kind(entity),
            Some(elidex_ecs::NodeKind::Element)
        ) {
            return Err(VmError::type_error(
                "Failed to execute 'getComputedStyle' on 'Window': parameter 1 is not an Element",
            ));
        }
    }
    let id = ctx.vm.alloc_computed_style(entity);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// CSS namespace global — CSS.escape / CSS.supports
// ---------------------------------------------------------------------------

impl VmInner {
    /// Install the `CSS` namespace global with `escape` / `supports` static
    /// methods (CSSOM §6.7).  Called from `register_globals` after
    /// `register_object_prototype` (`CSS` chains to `Object.prototype`).
    pub(in crate::vm) fn register_css_namespace_global(&mut self) {
        let obj_proto = self.object_prototype;
        let ns_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: obj_proto,
            extensible: true,
        });
        for (name_sid, func) in [
            (self.well_known.escape, native_css_escape as NativeFn),
            (self.well_known.supports, native_css_supports),
        ] {
            self.install_native_method(ns_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }
        let key = self.well_known.css_namespace;
        self.globals.insert(key, JsValue::Object(ns_id));
    }
}

/// `CSS.escape(ident)` (CSSOM §6.7.2) — pure string transformation, no
/// DOM context needed.  Calls [`elidex_css::escape_ident`] directly per
/// CLAUDE.md Layering mandate ("engine-independent crate" — direct
/// dispatch is preferred to a registry round-trip when no `this` /
/// `dom` participates).
fn native_css_escape(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let sid = coerce_first_arg_to_string_id(ctx, args)?;
    let input = ctx.vm.strings.get_utf8(sid);
    let escaped = elidex_css::escape_ident(&input);
    let out_sid = ctx.vm.strings.intern(&escaped);
    Ok(JsValue::String(out_sid))
}

/// `CSS.supports(property, value)` 2-arg form (CSSOM §6.7.1) — feature
/// query against the declaration parser.  `CSS.supports(condition)` 1-arg
/// form is deferred (slot `#11-css-supports-condition`); PR-A returns
/// `false` for any 1-arg call so framework feature-detect calls fail
/// closed rather than throwing.
fn native_css_supports(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if args.is_empty() {
        // `CSS.supports()` with no args throws TypeError per WebIDL.
        return Err(VmError::type_error(
            "Failed to execute 'supports' on 'CSS': 1 argument required, but 0 present.",
        ));
    }
    if args.len() < 2 {
        // 1-arg `<supports-condition>` form deferred.  Force-coerce arg
        // so a Symbol still throws (matches WebIDL coercion order).
        let _ = super::super::coerce::to_string(ctx.vm, args[0])?;
        return Ok(JsValue::Boolean(false));
    }
    let prop_sid = super::super::coerce::to_string(ctx.vm, args[0])?;
    let val_sid = super::super::coerce::to_string(ctx.vm, args[1])?;
    let property = ctx.vm.strings.get_utf8(prop_sid);
    let value = ctx.vm.strings.get_utf8(val_sid);
    let css = format!("{property}: {value};");
    let supported = !elidex_css::parse_declaration_block(&css).is_empty();
    Ok(JsValue::Boolean(supported))
}
