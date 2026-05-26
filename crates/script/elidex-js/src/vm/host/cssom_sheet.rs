//! `CSSStyleSheet` / `CSSRuleList` / `CSSStyleRule` / `StyleSheetList`
//! prototype install + native method bodies (CSSOM §6.2 / §6.3 / §6.6 / §6.8).
//!
//! Thin bindings to `elidex_dom_api::cssom_sheet` per the CLAUDE.md Layering
//! mandate — most bodies are a single [`invoke_dom_api`] dispatch or a direct
//! allocator helper. The `StyleSheetList` natives (`length` / `item` /
//! indexed exotic) and `<style>.sheet` getter additionally call the
//! engine-independent walker [`elidex_dom_api::collect_stylesheet_owners`]
//! (and `EcsDom::has_tag` for the per-element brand) directly — same shape
//! as PR-A's `getComputedStyle` host binding, which reads `ComputedStyle` /
//! tag info via dom-api helpers without an `invoke_dom_api` round-trip.
//!
//! ## Wrapper allocators
//!
//! - [`VmInner::alloc_or_cached_stylesheet`] — `<style>.sheet` identity
//!   ([`SameObject`]) per `<style>` Entity.
//! - [`VmInner::alloc_css_rule_list`] — fresh-alloc per `cssRules` access
//!   (matches WPT identity rules; the rule list is rarely retained).
//! - [`VmInner::alloc_or_cached_css_style_rule`] — `(<style> Entity, rule_id)`
//!   identity so `sheet.cssRules[i] === sheet.cssRules[i]`.
//! - [`VmInner::alloc_or_cached_rule_style`] — `(<style> Entity, rule_id)`
//!   identity for `r.style` (CSSStyleDeclaration source=Rule).
//! - [`VmInner::alloc_style_sheet_list`] — fresh-alloc per
//!   `document.styleSheets` access.
//!
//! ## Brand-check sharing with `CSSStyleDeclaration`
//!
//! `CSSRuleStyleDeclaration` chains to the same `CSSStyleDeclaration.prototype`
//! installed in PR-A so `r.style.getPropertyValue(...)` resolves through the
//! shared method table.  The native bodies in `css_style_declaration.rs` are
//! extended to accept both `ObjectKind::CSSStyleDeclaration` and
//! `ObjectKind::CSSRuleStyleDeclaration` receivers; the kind tag distinguishes
//! the backing store at dispatch time.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};
use super::super::wrapper_intern::{WrapperKey, WrapperKind};
use super::super::{NativeFn, VmInner};
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api};

use elidex_ecs::Entity;

/// Encode a CSSOM rule_id as a JS `Number` argument for engine-agnostic
/// dom-api dispatch. rule_ids transit through `JsValue::Number` so the
/// dom-api layer stays JS-engine-independent (it has no concept of a
/// branded `RuleId` type). The `f64` cast is precision-lossy beyond
/// `2^53` rule_ids; in practice a single sheet would need 9×10¹⁵
/// `insertRule` calls to reach that bound, so the lossy cast is
/// accepted. Localising the `#[allow]` here keeps the call sites quiet.
#[allow(clippy::cast_precision_loss)]
pub(super) fn rule_id_to_js(rule_id: u64) -> JsValue {
    JsValue::Number(rule_id as f64)
}

// ---------------------------------------------------------------------------
// Prototype install + wrapper allocators
// ---------------------------------------------------------------------------

impl VmInner {
    /// Install the `CSSStyleSheet` / `CSSRuleList` / `CSSStyleRule` /
    /// `StyleSheetList` prototypes.  Must run after
    /// `register_object_prototype` and `register_css_style_declaration_prototype`
    /// (CSSStyleRule.style returns a wrapper whose prototype is the
    /// shared CSSStyleDeclaration.prototype).
    pub(in crate::vm) fn register_cssom_sheet_prototypes(&mut self) {
        self.install_css_stylesheet_prototype();
        self.install_css_rule_list_prototype();
        self.install_css_style_rule_prototype();
        self.install_style_sheet_list_prototype();
    }

    /// Allocate an `Ordinary` prototype chained to `Object.prototype`,
    /// install the listed accessor pairs (RO WebIDL semantics) and
    /// methods, and return its `ObjectId`. Centralises the four
    /// CSSOM-prototype shells (CSSStyleSheet / CSSRuleList /
    /// CSSStyleRule / StyleSheetList) which differ only in their
    /// property tables.
    fn alloc_simple_prototype(
        &mut self,
        accessors: &[(super::super::value::StringId, NativeFn, Option<NativeFn>)],
        methods: &[(super::super::value::StringId, NativeFn)],
    ) -> ObjectId {
        let obj_proto = self.object_prototype;
        let proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: obj_proto,
            extensible: true,
        });
        let ro = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;
        for &(sid, getter, setter) in accessors {
            self.install_accessor_pair(proto, sid, getter, setter, ro);
        }
        for &(name_sid, func) in methods {
            self.install_native_method(proto, name_sid, func, shape::PropertyAttrs::METHOD);
        }
        proto
    }

    fn install_css_stylesheet_prototype(&mut self) {
        let proto = self.alloc_simple_prototype(
            &[
                (self.well_known.css_rules, native_sheet_css_rules_get, None),
                (
                    self.well_known.owner_node,
                    native_sheet_owner_node_get,
                    None,
                ),
                (self.well_known.r#type, native_sheet_type_get, None),
                (
                    self.well_known.disabled,
                    native_sheet_disabled_get,
                    Some(native_sheet_disabled_set),
                ),
                (self.well_known.href, native_sheet_href_get, None),
            ],
            &[
                (self.well_known.insert_rule, native_sheet_insert_rule),
                (self.well_known.delete_rule, native_sheet_delete_rule),
            ],
        );
        self.css_stylesheet_prototype = Some(proto);
    }

    fn install_css_rule_list_prototype(&mut self) {
        let proto = self.alloc_simple_prototype(
            &[(self.well_known.length, native_rule_list_length_get, None)],
            &[(self.well_known.item, native_rule_list_item)],
        );
        self.css_rule_list_prototype = Some(proto);
    }

    fn install_css_style_rule_prototype(&mut self) {
        let proto = self.alloc_simple_prototype(
            &[
                (
                    self.well_known.css_text,
                    native_style_rule_css_text_get,
                    None,
                ),
                (
                    self.well_known.selector_text,
                    native_style_rule_selector_text_get,
                    None,
                ),
                (self.well_known.style, native_style_rule_style_get, None),
                (
                    self.well_known.parent_style_sheet,
                    native_style_rule_parent_style_sheet_get,
                    None,
                ),
            ],
            &[],
        );
        self.css_style_rule_prototype = Some(proto);
    }

    fn install_style_sheet_list_prototype(&mut self) {
        let proto = self.alloc_simple_prototype(
            &[(
                self.well_known.length,
                native_style_sheet_list_length_get,
                None,
            )],
            &[(self.well_known.item, native_style_sheet_list_item)],
        );
        self.style_sheet_list_prototype = Some(proto);
    }

    /// Allocate (or return cached) `CSSStyleSheet` wrapper for `<style>`
    /// `owner`.  CSSOM §6.2 `[SameObject]` for `HTMLStyleElement.sheet`.
    pub(crate) fn alloc_or_cached_stylesheet(&mut self, owner: Entity) -> ObjectId {
        self.intern_wrapper(WrapperKey::entity(owner, WrapperKind::StyleSheet), |vm| {
            let proto = vm
                .css_stylesheet_prototype
                .expect("alloc_or_cached_stylesheet before register_cssom_sheet_prototypes");
            vm.alloc_object(Object {
                kind: ObjectKind::CSSStyleSheet {
                    entity_bits: owner.to_bits().get(),
                },
                storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                prototype: Some(proto),
                extensible: false,
            })
        })
    }

    /// Allocate a fresh `CSSRuleList` wrapper.  Not cached (matches WPT
    /// identity).
    pub(crate) fn alloc_css_rule_list(&mut self, owner: Entity) -> ObjectId {
        let proto = self
            .css_rule_list_prototype
            .expect("alloc_css_rule_list before register_cssom_sheet_prototypes");
        self.alloc_object(Object {
            kind: ObjectKind::CSSRuleList {
                sheet_entity_bits: owner.to_bits().get(),
            },
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: false,
        })
    }

    /// Allocate (or return cached) `CSSStyleRule` wrapper for
    /// `(sheet, rule_id)`.
    pub(crate) fn alloc_or_cached_css_style_rule(
        &mut self,
        sheet: Entity,
        rule_id: u64,
    ) -> ObjectId {
        self.intern_wrapper(
            WrapperKey::entity_rule(sheet, WrapperKind::CssStyleRule, rule_id),
            |vm| {
                let proto = vm.css_style_rule_prototype.expect(
                    "alloc_or_cached_css_style_rule before register_cssom_sheet_prototypes",
                );
                vm.alloc_object(Object {
                    kind: ObjectKind::CSSStyleRule {
                        sheet_entity_bits: sheet.to_bits().get(),
                        rule_id,
                    },
                    storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                    prototype: Some(proto),
                    extensible: false,
                })
            },
        )
    }

    /// Allocate (or return cached) Rule-source `CSSStyleDeclaration` wrapper
    /// for `(sheet, rule_id)`.  Chains to the shared
    /// `CSSStyleDeclaration.prototype` from PR-A.
    pub(crate) fn alloc_or_cached_rule_style(&mut self, sheet: Entity, rule_id: u64) -> ObjectId {
        self.intern_wrapper(
            WrapperKey::entity_rule(sheet, WrapperKind::RuleStyle, rule_id),
            |vm| {
                let proto = vm.css_style_declaration_prototype.expect(
                    "alloc_or_cached_rule_style before register_css_style_declaration_prototype",
                );
                vm.alloc_object(Object {
                    kind: ObjectKind::CSSRuleStyleDeclaration {
                        sheet_entity_bits: sheet.to_bits().get(),
                        rule_id,
                    },
                    storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                    prototype: Some(proto),
                    extensible: false,
                })
            },
        )
    }

    /// Allocate a fresh `StyleSheetList` wrapper.  Not cached.
    pub(crate) fn alloc_style_sheet_list(&mut self, document: Entity) -> ObjectId {
        let proto = self
            .style_sheet_list_prototype
            .expect("alloc_style_sheet_list before register_cssom_sheet_prototypes");
        self.alloc_object(Object {
            kind: ObjectKind::StyleSheetList {
                document_entity_bits: document.to_bits().get(),
            },
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: false,
        })
    }
}

// ---------------------------------------------------------------------------
// Brand-check helpers
// ---------------------------------------------------------------------------

/// Brand-check a JS receiver and decode its branded payload via
/// `decode`. `Illegal invocation` covers both "wrong receiver type" and
/// "stale entity bits" — distinguishing the two is not observable
/// (entity-bit decode failure only happens after a cross-EcsDom rebind
/// where the wrapper itself is also unusable). Returning a single
/// TypeError matches the WebIDL §3.10 brand-check shape.
fn require_branded<T>(
    ctx: &NativeContext<'_>,
    this: JsValue,
    class: &'static str,
    method: &'static str,
    decode: impl FnOnce(&ObjectKind) -> Option<T>,
) -> Result<T, VmError> {
    let illegal = || {
        VmError::type_error(format!(
            "Failed to execute '{method}' on '{class}': Illegal invocation"
        ))
    };
    let JsValue::Object(id) = this else {
        return Err(illegal());
    };
    decode(&ctx.vm.get_object(id).kind).ok_or_else(illegal)
}

fn require_sheet_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<Entity, VmError> {
    require_branded(ctx, this, "CSSStyleSheet", method, |kind| {
        if let ObjectKind::CSSStyleSheet { entity_bits } = kind {
            Entity::from_bits(*entity_bits)
        } else {
            None
        }
    })
}

fn require_rule_list_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<Entity, VmError> {
    require_branded(ctx, this, "CSSRuleList", method, |kind| {
        if let ObjectKind::CSSRuleList { sheet_entity_bits } = kind {
            Entity::from_bits(*sheet_entity_bits)
        } else {
            None
        }
    })
}

fn require_style_rule_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<(Entity, u64), VmError> {
    require_branded(ctx, this, "CSSStyleRule", method, |kind| {
        if let ObjectKind::CSSStyleRule {
            sheet_entity_bits,
            rule_id,
        } = kind
        {
            Entity::from_bits(*sheet_entity_bits).map(|e| (e, *rule_id))
        } else {
            None
        }
    })
}

fn require_style_sheet_list_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<Entity, VmError> {
    require_branded(ctx, this, "StyleSheetList", method, |kind| {
        if let ObjectKind::StyleSheetList {
            document_entity_bits,
        } = kind
        {
            Entity::from_bits(*document_entity_bits)
        } else {
            None
        }
    })
}

// ---------------------------------------------------------------------------
// CSSStyleSheet natives
// ---------------------------------------------------------------------------

fn native_sheet_css_rules_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_sheet_receiver(ctx, this, "cssRules")?;
    let id = ctx.vm.alloc_css_rule_list(entity);
    Ok(JsValue::Object(id))
}

fn native_sheet_owner_node_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_sheet_receiver(ctx, this, "ownerNode")?;
    let Some(hd) = ctx.host_if_bound() else {
        return Ok(JsValue::Null);
    };
    if hd.dom().node_kind(entity).is_none() {
        return Ok(JsValue::Null);
    }
    let id = ctx.vm.create_element_wrapper(entity);
    Ok(JsValue::Object(id))
}

fn native_sheet_type_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_sheet_receiver(ctx, this, "type")?;
    let sid = ctx.vm.strings.intern("text/css");
    Ok(JsValue::String(sid))
}

fn native_sheet_disabled_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // PR-B does not propagate `disabled` to the cascade — the bit is stored
    // as a property on the wrapper (extensibility=false prevents that on
    // instances) so we always return `false`.  Surfacing a real disabled
    // bit lands in slot `#11-stylesheet-disabled` alongside cascade
    // invalidation plumbing.
    let _ = require_sheet_receiver(ctx, this, "disabled")?;
    Ok(JsValue::Boolean(false))
}

fn native_sheet_disabled_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Silent no-op as far as cascade plumbing — see
    // `native_sheet_disabled_get`.  ToBoolean is infallible (Symbol
    // and every other type coerce to a boolean per ECMA-262 §7.1.2); we
    // still call it for shape consistency with the would-be
    // observable spec algorithm, even though no value is recorded.
    // A real WebIDL strict-boolean conversion path lands with the
    // cascade-disabled wiring (slot `#11-stylesheet-disabled`).
    let _ = require_sheet_receiver(ctx, this, "disabled")?;
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let _ = super::super::coerce::to_boolean(ctx.vm, arg);
    Ok(JsValue::Undefined)
}

fn native_sheet_href_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // CSSOM §6.2 `StyleSheet.href`: the location of the style sheet.
    // `<link>`-loaded sheets carry the resolved absolute URL (read from
    // the engine-independent `LinkStylesheet` component via dom-api);
    // `<style>` sheets have no href and return null.
    let entity = require_sheet_receiver(ctx, this, "href")?;
    let Some(hd) = ctx.host_if_bound() else {
        return Ok(JsValue::Null);
    };
    let Some(href) = elidex_dom_api::link_sheet_href(entity, hd.dom()) else {
        return Ok(JsValue::Null);
    };
    let sid = ctx.vm.strings.intern(&href);
    Ok(JsValue::String(sid))
}

fn native_sheet_insert_rule(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_sheet_receiver(ctx, this, "insertRule")?;
    let rule_sid = coerce_first_arg_to_string_id(ctx, args)?;
    // CSSOM IDL: `insertRule(CSSOMString rule, optional unsigned long index = 0)`.
    // Missing arg defaults to 0; otherwise apply WebIDL `unsigned long` →
    // ToUint32 coercion before forwarding to the dom-api handler so callers
    // like `insertRule(rule, '1')` land at index 1, not 0.
    let index_value = match args.get(1).copied() {
        None | Some(JsValue::Undefined) => JsValue::Number(0.0),
        Some(v) => JsValue::Number(f64::from(super::super::coerce::to_uint32(ctx.vm, v)?)),
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(0.0));
    }
    invoke_dom_api(
        ctx,
        "stylesheet.insertRule",
        entity,
        &[JsValue::String(rule_sid), index_value],
    )
}

fn native_sheet_delete_rule(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_sheet_receiver(ctx, this, "deleteRule")?;
    // CSSOM IDL: `deleteRule(unsigned long index)` — required arg.
    // Missing arg → WebIDL TypeError; provided → ToUint32 coercion.
    let Some(arg) = args.first().copied() else {
        return Err(VmError::type_error(
            "Failed to execute 'deleteRule' on 'CSSStyleSheet': 1 argument required, but only 0 present.",
        ));
    };
    let idx = super::super::coerce::to_uint32(ctx.vm, arg)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    invoke_dom_api(
        ctx,
        "stylesheet.deleteRule",
        entity,
        &[JsValue::Number(f64::from(idx))],
    )
}

// ---------------------------------------------------------------------------
// CSSRuleList natives
// ---------------------------------------------------------------------------

fn native_rule_list_length_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_rule_list_receiver(ctx, this, "length")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(0.0));
    }
    invoke_dom_api(ctx, "cssRules.length", entity, &[])
}

fn native_rule_list_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_rule_list_receiver(ctx, this, "item")?;
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let idx = super::super::coerce::to_uint32(ctx.vm, arg)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let id_value = invoke_dom_api(
        ctx,
        "cssRules.itemId",
        entity,
        &[JsValue::Number(f64::from(idx))],
    )?;
    let JsValue::Number(rule_id_f) = id_value else {
        return Ok(JsValue::Null);
    };
    if rule_id_f < 0.0 {
        return Ok(JsValue::Null);
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let rule_id = rule_id_f as u64;
    let id = ctx.vm.alloc_or_cached_css_style_rule(entity, rule_id);
    Ok(JsValue::Object(id))
}

/// Indexed-property exotic dispatch from `ops_element::get_element` — `list[i]`
/// returns the same wrapper as `list.item(i)`.  Returns `None` for non-numeric
/// keys so prototype-chain access (`length`) still resolves.
pub(crate) fn try_indexed_get_rule_list(
    vm: &mut VmInner,
    id: ObjectId,
    key: JsValue,
) -> Option<Result<JsValue, VmError>> {
    let ObjectKind::CSSRuleList { sheet_entity_bits } = vm.get_object(id).kind else {
        return None;
    };
    let entity = Entity::from_bits(sheet_entity_bits)?;
    let idx = match key {
        // ECMA-262 §7.1.22 canonical numeric-index string range is
        // `[0, 2^32-2]` — the same bound that
        // `coerce_format::parse_array_index_u32` and
        // `class_list::try_indexed_get` enforce for ES array-index
        // semantics.  Without the upper-bound guard, `list[2^32-1]`
        // would be intercepted as an index access and route through
        // `item` (returning null) instead of falling through to
        // ordinary [[Get]] / the prototype chain.
        JsValue::Number(n) if n.is_finite() && (0.0..=f64::from(u32::MAX - 1)).contains(&n) => {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let i = n as u32;
            if f64::from(i) != n {
                return None;
            }
            i
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
        return Some(Ok(JsValue::Null));
    }
    let mut ctx = NativeContext::new_call(vm);
    let id_value = match invoke_dom_api(
        &mut ctx,
        "cssRules.itemId",
        entity,
        &[JsValue::Number(f64::from(idx))],
    ) {
        Ok(v) => v,
        Err(e) => return Some(Err(e)),
    };
    let JsValue::Number(rule_id_f) = id_value else {
        return Some(Ok(JsValue::Null));
    };
    if rule_id_f < 0.0 {
        return Some(Ok(JsValue::Null));
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let rule_id = rule_id_f as u64;
    let wrapper = vm.alloc_or_cached_css_style_rule(entity, rule_id);
    Some(Ok(JsValue::Object(wrapper)))
}

// ---------------------------------------------------------------------------
// CSSStyleRule natives
// ---------------------------------------------------------------------------

fn native_style_rule_css_text_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (sheet, rule_id) = require_style_rule_receiver(ctx, this, "cssText")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    invoke_dom_api(ctx, "rule.cssText.get", sheet, &[rule_id_to_js(rule_id)])
}

fn native_style_rule_selector_text_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (sheet, rule_id) = require_style_rule_receiver(ctx, this, "selectorText")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    invoke_dom_api(
        ctx,
        "rule.selectorText.get",
        sheet,
        &[rule_id_to_js(rule_id)],
    )
}

fn native_style_rule_style_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (sheet, rule_id) = require_style_rule_receiver(ctx, this, "style")?;
    let id = ctx.vm.alloc_or_cached_rule_style(sheet, rule_id);
    Ok(JsValue::Object(id))
}

fn native_style_rule_parent_style_sheet_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (sheet, _rule_id) = require_style_rule_receiver(ctx, this, "parentStyleSheet")?;
    let id = ctx.vm.alloc_or_cached_stylesheet(sheet);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// StyleSheetList natives
// ---------------------------------------------------------------------------

fn native_style_sheet_list_length_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let document = require_style_sheet_list_receiver(ctx, this, "length")?;
    let Some(hd) = ctx.host_if_bound() else {
        return Ok(JsValue::Number(0.0));
    };
    let count = elidex_dom_api::count_stylesheet_owners(document, hd.dom());
    #[allow(clippy::cast_precision_loss)]
    Ok(JsValue::Number(count as f64))
}

fn native_style_sheet_list_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let document = require_style_sheet_list_receiver(ctx, this, "item")?;
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let idx = super::super::coerce::to_uint32(ctx.vm, arg)?;
    let Some(hd) = ctx.host_if_bound() else {
        return Ok(JsValue::Null);
    };
    let owners = elidex_dom_api::collect_stylesheet_owners(document, hd.dom());
    let Some(&owner) = owners.get(idx as usize) else {
        return Ok(JsValue::Null);
    };
    let id = ctx.vm.alloc_or_cached_stylesheet(owner);
    Ok(JsValue::Object(id))
}

/// Indexed-property exotic for `document.styleSheets[i]`.
pub(crate) fn try_indexed_get_style_sheet_list(
    vm: &mut VmInner,
    id: ObjectId,
    key: JsValue,
) -> Option<Result<JsValue, VmError>> {
    let ObjectKind::StyleSheetList {
        document_entity_bits,
    } = vm.get_object(id).kind
    else {
        return None;
    };
    let document = Entity::from_bits(document_entity_bits)?;
    let idx = match key {
        // ECMA-262 §7.1.22 canonical numeric-index string range is
        // `[0, 2^32-2]` — the same bound that
        // `coerce_format::parse_array_index_u32` and
        // `class_list::try_indexed_get` enforce for ES array-index
        // semantics.  Without the upper-bound guard, `list[2^32-1]`
        // would be intercepted as an index access and route through
        // `item` (returning null) instead of falling through to
        // ordinary [[Get]] / the prototype chain.
        JsValue::Number(n) if n.is_finite() && (0.0..=f64::from(u32::MAX - 1)).contains(&n) => {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let i = n as u32;
            if f64::from(i) != n {
                return None;
            }
            i
        }
        JsValue::String(sid) => {
            let units = vm.strings.get(sid);
            super::super::coerce_format::parse_array_index_u32(units)?
        }
        _ => return None,
    };
    let owner_opt = match vm.host_data.as_deref_mut() {
        Some(hd) if hd.is_bound() => elidex_dom_api::collect_stylesheet_owners(document, hd.dom())
            .get(idx as usize)
            .copied(),
        _ => return Some(Ok(JsValue::Null)),
    };
    let Some(owner) = owner_opt else {
        return Some(Ok(JsValue::Null));
    };
    let wrapper = vm.alloc_or_cached_stylesheet(owner);
    Some(Ok(JsValue::Object(wrapper)))
}

/// `HTMLStyleElement.prototype.sheet` getter (CSSOM §6.2). Returns the
/// `[SameObject]` `CSSStyleSheet` wrapper for `<style>`. Foreign
/// receivers (`getter.call(<div>)` etc.) throw `TypeError` per WebIDL
/// brand-check semantics — `<style>.sheet` is a HTMLStyleElement IDL
/// member, so the prototype install lives on `html_style_proto.rs`
/// (slot `#11-tags-T2b-passive`); non-`<style>` elements no longer
/// have a `sheet` accessor on their prototype chain.
pub(super) fn native_html_style_get_sheet(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity =
        super::event_target::require_receiver(ctx, this, "HTMLStyleElement", "sheet", |kind| {
            matches!(kind, elidex_ecs::NodeKind::Element)
        })?
        .ok_or_else(|| {
            VmError::type_error(
                "Failed to execute 'sheet' on 'HTMLStyleElement': Illegal invocation",
            )
        })?;
    let Some(hd) = ctx.host_if_bound() else {
        return Ok(JsValue::Null);
    };
    // ASCII case-insensitive tag match per WHATWG DOM §4.2.6.2.
    // Mirrors `first_child_with_tag`'s `eq_ignore_ascii_case` pattern
    // — accepts raw `create_element("STYLE", ...)` callers and any
    // future XML / mixed-case path.
    let is_style = hd.dom().with_tag_name(entity, |t| {
        t.is_some_and(|t| t.eq_ignore_ascii_case("style"))
    });
    if !is_style {
        return Err(VmError::type_error(
            "Failed to execute 'sheet' on 'HTMLStyleElement': Illegal invocation",
        ));
    }
    let id = ctx.vm.alloc_or_cached_stylesheet(entity);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// Document.styleSheets accessor — installed on Document.prototype.
// ---------------------------------------------------------------------------

/// `Document.prototype.styleSheets` getter (CSSOM §6.8).  Returns a
/// fresh `StyleSheetList` wrapper; the walker enumerates `<style>`
/// descendants on each access.
pub(super) fn native_document_get_style_sheets(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity =
        super::event_target::require_receiver(ctx, this, "Document", "styleSheets", |kind| {
            matches!(kind, elidex_ecs::NodeKind::Document)
        })?;
    // Mirror sibling Document accessors (`head` / `body` / etc.):
    // post-unbind / non-Document-receiver paths produce `Ok(None)`
    // here and surface a safe default rather than `TypeError`. The
    // brand-check `Err` path (a wrong-kind HostObject) still throws
    // through `require_receiver`'s own error.
    let Some(entity) = entity else {
        return Ok(JsValue::Null);
    };
    let id = ctx.vm.alloc_style_sheet_list(entity);
    Ok(JsValue::Object(id))
}
