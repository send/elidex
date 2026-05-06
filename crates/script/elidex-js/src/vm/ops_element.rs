//! Element access operations: bracket-indexed `obj[key]` reads and
//! writes, including the TypedArray integer-indexed exotic dispatch
//! and Array / Arguments / StringWrapper dense-storage fast paths.
//!
//! Split from [`super::ops_property`] to keep both files below the
//! 1000-line convention (cleanup tranche 2).  The moved entry points
//! ([`super::VmInner::get_element`] / [`super::VmInner::set_element`])
//! delegate to the same property-access primitives
//! ([`super::VmInner::resolve_property`] /
//! [`super::VmInner::set_property_val`] /
//! [`super::VmInner::ordinary_set`] /
//! [`super::VmInner::lookup_on_proto`]) that named-property reads and
//! writes use, so the precedence ordering is unchanged: TypedArray
//! integer-indexed exotic → Array / Arguments dense fast path →
//! StringWrapper / String index → live DOM collection / NamedNodeMap →
//! ordinary property + prototype chain.
//!
//! ## TypedArray key classification
//!
//! ES §10.4.5 + §7.1.16.1 (`CanonicalNumericIndexString`) splits
//! property keys against a TypedArray receiver into three buckets:
//!
//! - **`IntegerIndex(u32)`** — non-negative integer in `[0, u32::MAX]`.
//!   In-range writes go through
//!   [`super::host::typed_array::write_element_raw`]; reads through
//!   [`super::host::typed_array::read_element_raw`].  Out-of-range
//!   reads return `undefined`; out-of-range writes are silent no-ops
//!   (§10.4.5.16 step 1).
//! - **`CanonicalNonInteger`** — canonical numeric string that ISN'T a
//!   valid integer index (`"-0"` / `"NaN"` / `"Infinity"` /
//!   `"-Infinity"` / negative integer / fractional / exponential with
//!   canonical round-trip).  Reads return `undefined`; writes are
//!   silent no-ops; neither path creates an ordinary own property.
//! - **`NotNumeric`** — falls through to ordinary property storage.
//!
//! The classification helpers ([`classify_typed_array_number_key`] /
//! [`classify_typed_array_string_key`]) are private to this module
//! since they're only consumed by the element-access methods.

use super::coerce::{get_property, to_string};
#[cfg(feature = "engine")]
use super::value::StringId;
use super::value::{JsValue, ObjectKind, PropertyKey, VmError};
use super::VmInner;

use super::ops::{parse_array_index_u16, try_as_array_index, DENSE_ARRAY_LEN_LIMIT};

/// Classification of a WTF-16 string key against the TypedArray
/// integer-indexed exotic object contract (ES §10.4.5 + §7.1.16.1
/// `CanonicalNumericIndexString`).
#[cfg(feature = "engine")]
enum TypedArrayStringKey {
    /// Non-negative integer in [0, u32::MAX].  Dispatches to
    /// `read_element_raw` / `write_element_raw`.  If the caller's
    /// length check fails, Get returns `undefined` and Set is a
    /// silent no-op.
    IntegerIndex(u32),
    /// Canonical numeric string that is NOT a valid integer index
    /// (`"-0"`, `"Infinity"`, `"-Infinity"`, `"NaN"`, negative
    /// integer, fractional, exponential with canonical round-trip).
    /// TypedArray Get returns `undefined`; Set is a silent no-op;
    /// neither creates an ordinary property (§10.4.5.15 step 3 /
    /// §10.4.5.16 step 1).
    CanonicalNonInteger,
    /// Not a canonical numeric string — falls through to ordinary
    /// property storage.
    NotNumeric,
}

/// Classify a Number key against the TypedArray integer-indexed
/// exotic contract.  A Number key `n` is treated as if ToString'd —
/// non-negative integers up to `u32::MAX` map to their index; all
/// other numeric forms (`NaN`, ±`Infinity`, negative, fractional,
/// out-of-u32-range) are canonical-numeric-but-not-integer, which
/// §10.4.5.15/16 short-circuit to `undefined` / silent no-op.
#[cfg(feature = "engine")]
fn classify_typed_array_number_key(n: f64) -> TypedArrayStringKey {
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    {
        if n.is_finite() && n >= 0.0 && n <= f64::from(u32::MAX) {
            let as_u32 = n as u32;
            if f64::from(as_u32) == n {
                return TypedArrayStringKey::IntegerIndex(as_u32);
            }
        }
    }
    TypedArrayStringKey::CanonicalNonInteger
}

/// Parse a WTF-16 string as a TypedArray integer index (0..=u32::MAX).
/// Distinct from `parse_array_index_u16` (capped at 2^32−2 per the
/// Array `[[HasOwnProperty]]` contract); TypedArray permits the full
/// u32 range.
#[cfg(feature = "engine")]
fn parse_typed_array_index_u32(units: &[u16]) -> Option<u32> {
    if units.is_empty() {
        return None;
    }
    if units.len() > 1 && units[0] == u16::from(b'0') {
        return None;
    }
    let mut n: u64 = 0;
    for &u in units {
        let digit = u.wrapping_sub(u16::from(b'0'));
        if digit > 9 {
            return None;
        }
        n = n.checked_mul(10)?.checked_add(u64::from(digit))?;
    }
    u32::try_from(n).ok()
}

#[cfg(feature = "engine")]
fn classify_typed_array_string_key(vm: &mut VmInner, sid: StringId) -> TypedArrayStringKey {
    {
        let units = vm.strings.get(sid);
        if let Some(idx) = parse_typed_array_index_u32(units) {
            return TypedArrayStringKey::IntegerIndex(idx);
        }
        // ES §7.1.16.1 step 1 hard-codes `"-0"` as canonical numeric
        // (returns -0) even though ToString(-0) = "0" — the round-trip
        // check below would otherwise miss it.
        if units == [u16::from(b'-'), u16::from(b'0')] {
            return TypedArrayStringKey::CanonicalNonInteger;
        }
    }
    // Slow path: round-trip via ES Number::toString.  If
    // ToString(ToNumber(key)) == key, the key is canonical numeric
    // (the fast path above already handles non-negative integers, so
    // any remaining canonical form — `"Infinity"`, `"NaN"`, negative
    // integer, fractional — is non-integer-valid).  `to_number` takes
    // `&mut vm` for the §7.1.4 step 4 Object path, so the early borrow
    // above is dropped before this call and re-acquired below.
    let n = super::coerce::to_number(vm, JsValue::String(sid)).unwrap_or(f64::NAN);
    let mut roundtrip = String::new();
    if n.is_nan() {
        roundtrip.push_str("NaN");
    } else if n.is_infinite() {
        roundtrip.push_str(if n > 0.0 { "Infinity" } else { "-Infinity" });
    } else {
        super::coerce_format::write_number_es(n, &mut roundtrip);
    }
    let units = vm.strings.get(sid);
    if units.len() == roundtrip.len()
        && units
            .iter()
            .zip(roundtrip.as_bytes())
            .all(|(&u, &b)| u == u16::from(b))
    {
        TypedArrayStringKey::CanonicalNonInteger
    } else {
        TypedArrayStringKey::NotNumeric
    }
}

impl VmInner {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn get_element(&mut self, obj: JsValue, key: JsValue) -> Result<JsValue, VmError> {
        // §6.2.4.5 RequireObjectCoercible: `null[key]` / `undefined[key]` throw.
        super::coerce::require_object_coercible(obj)?;
        if let JsValue::Object(id) = obj {
            // TypedArray integer-indexed get (ES §10.4.5.15).  Must
            // run ahead of the generic numeric fast path so any
            // CanonicalNumericIndexString Number key — including
            // `NaN` / ±`Infinity` / negative / fractional /
            // out-of-u32-range values rejected by `try_as_array_index`
            // — short-circuits to `undefined` without consulting
            // ordinary properties or the prototype chain.
            #[cfg(feature = "engine")]
            if let JsValue::Number(n) = key {
                if let ObjectKind::TypedArray {
                    buffer_id,
                    byte_offset,
                    byte_length,
                    element_kind,
                } = self.get_object(id).kind
                {
                    match classify_typed_array_number_key(n) {
                        TypedArrayStringKey::IntegerIndex(i) => {
                            let len_elem =
                                byte_length / u32::from(element_kind.bytes_per_element());
                            if i < len_elem {
                                return Ok(super::host::typed_array::read_element_raw(
                                    self,
                                    buffer_id,
                                    byte_offset,
                                    i,
                                    element_kind,
                                ));
                            }
                            return Ok(JsValue::Undefined);
                        }
                        TypedArrayStringKey::CanonicalNonInteger => return Ok(JsValue::Undefined),
                        TypedArrayStringKey::NotNumeric => {}
                    }
                }
            }
            // Numeric index for arrays / Arguments / StringWrapper.
            if let JsValue::Number(n) = key {
                if let Some(idx) = try_as_array_index(n) {
                    let obj_ref = self.get_object(id);
                    match &obj_ref.kind {
                        ObjectKind::Array { ref elements } => {
                            let elem = elements.get(idx).copied().unwrap_or(JsValue::Empty);
                            if !elem.is_empty() {
                                return Ok(elem);
                            }
                            // Hole or out-of-range: fall through to property/prototype lookup.
                        }
                        ObjectKind::Arguments { ref values } if idx < values.len() => {
                            return Ok(values[idx]);
                        }
                        ObjectKind::StringWrapper(sid) => {
                            if let Some(&unit) = self.strings.get(*sid).get(idx) {
                                let ch_id = self.strings.intern_utf16(&[unit]);
                                return Ok(JsValue::String(ch_id));
                            }
                        }
                        _ => {}
                    }
                }
            }

            // PR5b §C3: HTMLCollection / NodeList indexed + legacy
            // named property access.  Delegates to a shared helper
            // that re-traverses the backing filter and resolves
            // both numeric indices and (HTMLCollection-only)
            // `id` / tag-allowlisted `name` lookups.  Falls through
            // to the standard property / prototype lookup on miss
            // so that `.length` / `.item` still see the prototype
            // accessor / method.
            #[cfg(feature = "engine")]
            {
                let kind_snapshot = &self.get_object(id).kind;
                let is_live_collection = matches!(
                    kind_snapshot,
                    ObjectKind::HtmlCollection | ObjectKind::NodeList
                );
                let is_named_node_map = matches!(kind_snapshot, ObjectKind::NamedNodeMap);
                if is_live_collection || is_named_node_map {
                    // Two-phase lookup:
                    //
                    //   1. With a shared `&EcsDom` borrow (obtained
                    //      through a raw-pointer detach from
                    //      `HostData::dom_shared`), call the typed
                    //      helper to produce an `Entity` (for live
                    //      collections) or `(owner, qname_sid)`
                    //      (for NamedNodeMap).  The helper must not
                    //      itself allocate wrappers — doing so
                    //      would mutably reborrow `host_data`
                    //      (`wrapper_cache` / `attr_states`) while
                    //      the `&EcsDom` derived from the same
                    //      `HostData` reborrow chain is still live,
                    //      a Stacked Borrows violation.
                    //   2. Drop the `&EcsDom` borrow, then allocate
                    //      the wrapper on the clean `&mut VmInner`.
                    //
                    // `dom_shared()` panics when `HostData` is
                    // unbound.  Collection wrappers can outlive
                    // `Vm::unbind()` when they remain reachable
                    // from ordinary JS roots (e.g. `globalThis.hc
                    // = ...`); the side tables
                    // (`live_collection_states` /
                    // `named_node_map_states`) are NOT GC roots —
                    // they are pruned after the mark phase based on
                    // whether the key `ObjectId` was itself marked.
                    // Post-unbind indexed access on a retained
                    // wrapper therefore falls through to normal
                    // prototype lookup rather than panicking.
                    let entity_hit: Option<elidex_ecs::Entity>;
                    let nnm_hit: Option<(elidex_ecs::Entity, super::value::StringId)>;
                    if let Some(hd) = self.host_data.as_deref().filter(|h| h.is_bound()) {
                        #[allow(unsafe_code)]
                        let dom_ptr: *const elidex_ecs::EcsDom = hd.dom_shared();
                        #[allow(unsafe_code)]
                        let dom = unsafe { &*dom_ptr };
                        if is_live_collection {
                            entity_hit =
                                super::host::dom_collection::try_indexed_get(self, dom, id, key);
                            nnm_hit = None;
                        } else {
                            entity_hit = None;
                            nnm_hit =
                                super::host::named_node_map::try_indexed_get(self, dom, id, key);
                        }
                        // `dom` / `dom_ptr` fall out of scope here
                        // — subsequent wrapper allocation runs with
                        // no outstanding DOM borrow aliasing
                        // `host_data`.
                    } else {
                        entity_hit = None;
                        nnm_hit = None;
                    }
                    if let Some(e) = entity_hit {
                        return Ok(JsValue::Object(self.create_element_wrapper(e)));
                    }
                    if let Some((owner, qname_sid)) = nnm_hit {
                        let attr_id = self.cached_or_alloc_attr_live(owner, qname_sid);
                        return Ok(JsValue::Object(attr_id));
                    }
                }
            }

            // DOMTokenList (Element.classList) indexed-property
            // exotic — `tokens[i]` returns the i-th token string,
            // out-of-range and non-canonical keys fall through.
            #[cfg(feature = "engine")]
            if matches!(self.get_object(id).kind, ObjectKind::DOMTokenList { .. }) {
                if let Some(result) = super::host::class_list::try_indexed_get(self, id, key) {
                    return result;
                }
            }
            // DOMStringMap (HTMLElement.dataset) named-property
            // exotic [[Get]] — `dataset.fooBar` reads through to the
            // backing `data-foo-bar` attribute via the
            // `dataset.get` handler.  Symbol keys fall through to
            // the prototype chain (so `Symbol.iterator` / etc.
            // resolve via `Object.prototype`).
            #[cfg(feature = "engine")]
            if matches!(self.get_object(id).kind, ObjectKind::DOMStringMap { .. }) {
                if let Some(result) = super::host::dataset::try_get(self, id, key) {
                    return result;
                }
            }
            // Symbol key -> direct property lookup.
            if let JsValue::Symbol(sid) = key {
                let pk = PropertyKey::Symbol(sid);
                return match get_property(self, id, pk) {
                    Some(result) => self.resolve_property(result, obj),
                    None => Ok(JsValue::Undefined),
                };
            }
            // Fall back to string key property lookup.
            let key_id = to_string(self, key)?;
            // StringWrapper: index access and length
            if let ObjectKind::StringWrapper(sid) = self.get_object(id).kind {
                if key_id == self.well_known.length {
                    #[allow(clippy::cast_precision_loss)]
                    let len = self.strings.get(sid).len() as f64;
                    return Ok(JsValue::Number(len));
                }
                let key_units = self.strings.get(key_id);
                if let Some(idx) = parse_array_index_u16(key_units) {
                    if let Some(&unit) = self.strings.get(sid).get(idx) {
                        let ch_id = self.strings.intern_utf16(&[unit]);
                        return Ok(JsValue::String(ch_id));
                    }
                }
            }
            // String numeric key on TypedArray — ES §10.4.5 integer-
            // indexed exotic dispatch.  Any CanonicalNumericIndexString
            // (§7.1.16.1) — including `"-0"` / `"Infinity"` / `"NaN"` /
            // negative integer / fractional — short-circuits to
            // `undefined` rather than falling through to ordinary
            // property access.
            #[cfg(feature = "engine")]
            {
                if let ObjectKind::TypedArray {
                    buffer_id,
                    byte_offset,
                    byte_length,
                    element_kind,
                } = self.get_object(id).kind
                {
                    match classify_typed_array_string_key(self, key_id) {
                        TypedArrayStringKey::IntegerIndex(i) => {
                            let len_elem =
                                byte_length / u32::from(element_kind.bytes_per_element());
                            if i < len_elem {
                                return Ok(super::host::typed_array::read_element_raw(
                                    self,
                                    buffer_id,
                                    byte_offset,
                                    i,
                                    element_kind,
                                ));
                            }
                            return Ok(JsValue::Undefined);
                        }
                        TypedArrayStringKey::CanonicalNonInteger => {
                            return Ok(JsValue::Undefined);
                        }
                        TypedArrayStringKey::NotNumeric => {}
                    }
                }
            }
            // String key that parses as array index → check elements first.
            if matches!(self.get_object(id).kind, ObjectKind::Array { .. }) {
                let key_units = self.strings.get(key_id);
                if let Some(idx) = parse_array_index_u16(key_units) {
                    let elem = {
                        let obj_ref = self.get_object(id);
                        if let ObjectKind::Array { ref elements } = obj_ref.kind {
                            elements.get(idx).copied().unwrap_or(JsValue::Empty)
                        } else {
                            JsValue::Empty
                        }
                    };
                    if !elem.is_empty() {
                        return Ok(elem);
                    }
                    // Hole: fall through to property/prototype lookup.
                }
            }
            let pk = PropertyKey::String(key_id);
            match get_property(self, id, pk) {
                Some(result) => self.resolve_property(result, obj),
                None => Ok(JsValue::Undefined),
            }
        } else if let JsValue::String(sid) = obj {
            // String bracket access: str[index] returns a single UTF-16 code unit.
            if let JsValue::Number(n) = key {
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let idx = n as usize;
                #[allow(clippy::cast_precision_loss)]
                if n >= 0.0 && (idx as f64) == n {
                    let unit = self.strings.get(sid).get(idx).copied();
                    if let Some(u) = unit {
                        let id = self.strings.intern_utf16(&[u]);
                        return Ok(JsValue::String(id));
                    }
                }
            } else if let JsValue::String(key_sid) = key {
                let unit = {
                    let key_units = self.strings.get(key_sid);
                    parse_array_index_u16(key_units)
                        .and_then(|idx| self.strings.get(sid).get(idx).copied())
                };
                if let Some(u) = unit {
                    let ch_id = self.strings.intern_utf16(&[u]);
                    return Ok(JsValue::String(ch_id));
                }
            }
            let pk = match key {
                JsValue::Symbol(sym) => PropertyKey::Symbol(sym),
                other => PropertyKey::String(to_string(self, other)?),
            };
            if pk == PropertyKey::String(self.well_known.length) {
                #[allow(clippy::cast_precision_loss)]
                let len = self.strings.get(sid).len() as f64;
                return Ok(JsValue::Number(len));
            }
            if let Some(proto_id) = self.string_prototype {
                match get_property(self, proto_id, pk) {
                    Some(result) => self.resolve_property(result, obj),
                    None => Ok(JsValue::Undefined),
                }
            } else {
                Ok(JsValue::Undefined)
            }
        } else if matches!(
            obj,
            JsValue::Number(_) | JsValue::Boolean(_) | JsValue::BigInt(_)
        ) {
            let proto = match obj {
                JsValue::Number(_) => self.number_prototype,
                JsValue::BigInt(_) => self.bigint_prototype,
                _ => self.boolean_prototype,
            };
            let pk = match key {
                JsValue::Symbol(sym) => PropertyKey::Symbol(sym),
                other => PropertyKey::String(to_string(self, other)?),
            };
            self.lookup_on_proto(proto, pk, obj)
        } else {
            Ok(JsValue::Undefined)
        }
    }

    /// Check whether an array element write at `idx` should be rejected
    /// due to non-extensible / frozen constraints. Returns `Some(result)`
    /// to early-return from the caller, or `None` to proceed.
    fn check_array_element_write(
        &self,
        obj_id: super::value::ObjectId,
        idx: usize,
    ) -> Option<Result<(), VmError>> {
        let obj = self.get_object(obj_id);
        if !matches!(obj.kind, ObjectKind::Array { .. }) || obj.extensible {
            return None;
        }
        let is_new = match &obj.kind {
            ObjectKind::Array { elements } => {
                idx >= elements.len() || elements.get(idx).is_some_and(|v| v.is_empty())
            }
            _ => false,
        };
        // Frozen = non-extensible + all named properties are non-writable+non-configurable.
        // Requires at least one named property to distinguish from preventExtensions.
        let mut has_named_props = false;
        let is_frozen = !is_new
            && obj.storage.iter_keys(&self.shapes).all(|(_, attrs)| {
                has_named_props = true;
                !attrs.configurable && (attrs.is_accessor || !attrs.writable)
            })
            && has_named_props;
        if is_new || is_frozen {
            return Some(Err(VmError::type_error(
                "Cannot assign to read only property",
            )));
        }
        None
    }

    /// TypedArray integer-indexed-write fast path (ES §10.4.5.16
    /// `IntegerIndexedElementSet`).  Returns `Some(Ok(()))` when the
    /// receiver is a TypedArray and `key` resolves to a canonical
    /// integer index (in-range write or silent out-of-range no-op);
    /// `Some(Err(…))` on coercion failure; `None` to defer to the
    /// ordinary property path.  Keeps `set_element` under the
    /// 100-line clippy limit while preserving the required
    /// precedence ahead of Array / Arguments dispatch.
    #[cfg(feature = "engine")]
    fn try_typed_array_element_set(
        &mut self,
        id: super::value::ObjectId,
        key: JsValue,
        val: JsValue,
    ) -> Option<Result<(), VmError>> {
        let ObjectKind::TypedArray {
            buffer_id,
            byte_offset,
            byte_length,
            element_kind,
        } = self.get_object(id).kind
        else {
            return None;
        };
        // Resolve a canonical integer index.  Non-canonical strings
        // (`"01"`, `"1.5e2"`) fall through to ordinary property
        // storage; canonical forms that are NOT valid integer
        // indices — `NaN` / ±`Infinity` / negative / fractional /
        // out-of-u32-range — are silent no-ops per §10.4.5.16 step 1
        // and must NOT surface as ordinary own properties.  Objects
        // with a custom `toString` routing to a canonical numeric
        // index string flow through the generic `ToString` branch
        // below; Symbols bypass the TypedArray exotic path and land
        // on ordinary own properties (§10.4.5 only specialises
        // Strings).
        let idx: u32 = match key {
            JsValue::Number(n) => match classify_typed_array_number_key(n) {
                TypedArrayStringKey::IntegerIndex(i) => i,
                TypedArrayStringKey::CanonicalNonInteger => return Some(Ok(())),
                TypedArrayStringKey::NotNumeric => return None,
            },
            JsValue::String(sid) => match classify_typed_array_string_key(self, sid) {
                TypedArrayStringKey::IntegerIndex(i) => i,
                TypedArrayStringKey::CanonicalNonInteger => return Some(Ok(())),
                TypedArrayStringKey::NotNumeric => return None,
            },
            JsValue::Symbol(_) => return None,
            other => {
                let sid = match to_string(self, other) {
                    Ok(sid) => sid,
                    Err(err) => return Some(Err(err)),
                };
                match classify_typed_array_string_key(self, sid) {
                    TypedArrayStringKey::IntegerIndex(i) => i,
                    TypedArrayStringKey::CanonicalNonInteger => return Some(Ok(())),
                    TypedArrayStringKey::NotNumeric => return None,
                }
            }
        };
        let len_elem = byte_length / u32::from(element_kind.bytes_per_element());
        if idx >= len_elem {
            // Canonical integer but out-of-range → silent no-op
            // (§10.4.5.16 step 1).  Does NOT create an own
            // ordinary property.
            return Some(Ok(()));
        }
        // In-range: coerce through `write_element_raw` (handles
        // ToBigInt / ToInt* / float encoding per `element_kind`).
        let mut ctx = super::value::NativeContext { vm: self };
        Some(super::host::typed_array::write_element_raw(
            &mut ctx,
            buffer_id,
            byte_offset,
            idx,
            element_kind,
            val,
        ))
    }

    pub(crate) fn set_element(
        &mut self,
        obj: JsValue,
        key: JsValue,
        val: JsValue,
    ) -> Result<(), VmError> {
        // §6.2.4.5 RequireObjectCoercible: `null[k] = v` / `undefined[k] = v` throw.
        super::coerce::require_object_coercible(obj)?;
        if let JsValue::Object(id) = obj {
            // TypedArray integer-indexed write dispatches ahead of
            // the Array / Arguments fast path — see
            // `try_typed_array_element_set` for rationale.
            #[cfg(feature = "engine")]
            if let Some(result) = self.try_typed_array_element_set(id, key, val) {
                return result;
            }
            // Numeric key → Array/Arguments dense-storage fast path.
            if let JsValue::Number(n) = key {
                if let Some(idx) = try_as_array_index(n) {
                    // Check extensible/frozen before taking mutable borrow.
                    if let Some(reject) = self.check_array_element_write(id, idx) {
                        return reject;
                    }
                    let obj_ref = self.get_object_mut(id);
                    match &mut obj_ref.kind {
                        ObjectKind::Array { ref mut elements } => {
                            if idx >= elements.len() {
                                if idx >= DENSE_ARRAY_LEN_LIMIT {
                                    return Err(VmError::range_error("Array allocation failed"));
                                }
                                let new_len = idx + 1;
                                elements
                                    .try_reserve(new_len - elements.len())
                                    .map_err(|_| VmError::range_error("Array allocation failed"))?;
                                elements.resize(new_len, JsValue::Empty);
                            }
                            elements[idx] = val;
                            return Ok(());
                        }
                        ObjectKind::Arguments { ref mut values } if idx < values.len() => {
                            values[idx] = val;
                            return Ok(());
                        }
                        _ => {}
                    }
                }
            }
            // Symbol key → §9.1.9 OrdinarySet directly (no string conversion).
            if let JsValue::Symbol(sid) = key {
                let pk = PropertyKey::Symbol(sid);
                self.ordinary_set(id, pk, val, obj)?;
                return Ok(());
            }
            // DOMStringMap (HTMLElement.dataset) named-property
            // exotic [[Set]] — `dataset.fooBar = "x"` writes the
            // backing `data-foo-bar` attribute via the
            // `dataset.set` handler (WHATWG HTML §3.2.6 named
            // setter).  String-coerced keys only; Symbol keys
            // already short-circuited above.
            #[cfg(feature = "engine")]
            if matches!(self.get_object(id).kind, ObjectKind::DOMStringMap { .. }) {
                if let Some(result) = super::host::dataset::try_set(self, id, key, val) {
                    return result;
                }
            }
            let key_id = to_string(self, key)?;
            // Numeric-string key on Array → dense-storage fast path.
            if matches!(self.get_object(id).kind, ObjectKind::Array { .. }) {
                let key_units = self.strings.get(key_id);
                if let Some(idx) = parse_array_index_u16(key_units) {
                    if let Some(reject) = self.check_array_element_write(id, idx) {
                        return reject;
                    }
                    let obj_ref = self.get_object_mut(id);
                    if let ObjectKind::Array { ref mut elements } = obj_ref.kind {
                        if idx >= elements.len() {
                            if idx >= DENSE_ARRAY_LEN_LIMIT {
                                return Err(VmError::range_error("Array allocation failed"));
                            }
                            let new_len = idx + 1;
                            elements
                                .try_reserve(new_len - elements.len())
                                .map_err(|_| VmError::range_error("Array allocation failed"))?;
                            elements.resize(new_len, JsValue::Empty);
                        }
                        elements[idx] = val;
                        return Ok(());
                    }
                }
            }
            return self.set_property_val(obj, key_id, val);
        }

        // Primitive base (after RequireObjectCoercible): box for descriptor
        // lookup per §6.2.4.8 PutValue step 5.a, keeping the original base
        // as Receiver so `ordinary_set` rejects data writes via §9.1.9.2
        // step 2.b.  (Array-style fast paths don't apply: primitive
        // wrappers are never `ObjectKind::Array`.)
        if let JsValue::Symbol(sid) = key {
            let target = super::coerce::to_object(self, obj)?;
            self.ordinary_set(target, PropertyKey::Symbol(sid), val, obj)?;
            return Ok(());
        }
        let key_id = to_string(self, key)?;
        self.set_property_val(obj, key_id, val)
    }
}
