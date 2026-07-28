//! Iterator-protocol opcode handlers extracted from the main dispatch loop.

use super::coerce::{get_property, PROTO_CHAIN_LIMIT};
use super::ops::DENSE_ARRAY_LEN_LIMIT;
use super::value::{
    ForInState, JsValue, Object, ObjectKind, PropertyKey, PropertyStorage, VmError,
};
use super::VmInner;

/// Format a `usize` into a stack-allocated buffer, returning a `&str`.
/// Avoids heap allocation from `i.to_string()`.
fn format_usize(n: usize, buf: &mut [u8; 20]) -> &str {
    use std::io::Write;
    let mut cursor = std::io::Cursor::new(&mut buf[..]);
    write!(cursor, "{n}").unwrap();
    let len = cursor.position() as usize;
    std::str::from_utf8(&buf[..len]).unwrap()
}

impl VmInner {
    /// Resolve the `@@iterator` for a value and call it to get an iterator object.
    ///
    /// Returns `Ok(Some(iterator))` on success, `Ok(None)` when the value has no
    /// iterator protocol (e.g. numbers, booleans), or propagates call errors.
    pub(crate) fn resolve_iterator(&mut self, val: JsValue) -> Result<Option<JsValue>, VmError> {
        let lookup_id = match val {
            JsValue::Object(id) => Some(id),
            JsValue::String(_) => self.string_prototype,
            _ => None,
        };
        let Some(obj_id) = lookup_id else {
            return Ok(None);
        };
        let iter_key = PropertyKey::Symbol(self.well_known_symbols.iterator);
        let Some(iter_result) = get_property(self, obj_id, iter_key) else {
            return Ok(None);
        };
        let iter_fn = self.resolve_property(iter_result, val)?;
        let result = self.call_value(iter_fn, val, &[])?;
        Ok(Some(result))
    }

    /// `Op::ArraySpread` — `[array source -- array]`, spread an iterable into
    /// the array beneath it.
    ///
    /// The **iterator** is the operand that needs rooting here, and it is not
    /// one the compiler put on the stack: §7.4.4 `GetIterator` calls
    /// `@@iterator` with the *iterable* as receiver, so what comes back is a
    /// fresh object nothing references. Each §7.4.10 `IteratorStepValue` then
    /// calls `next()`, and the loop dereferences the iterator again on the turn
    /// after — plus once more on the §7.4.11 `IteratorClose` path. A `next` that
    /// is a method makes the iterator its own receiver and hides the exposure;
    /// an arrow or bound `next` does not, and an iterator held only in a Rust
    /// local is then collected mid-iteration. It is written back into the
    /// source's slot (which the source no longer needs — `resolve_iterator` is
    /// the last read of it), so the arm's stack effect is unchanged.
    pub(super) fn op_array_spread(&mut self) -> Result<(), VmError> {
        let len = self.stack.len();
        if len < 2 {
            return Err(VmError::internal("stack underflow on ArraySpread"));
        }
        let arr_val = self.stack[len - 2];
        let source = self.stack[len - 1];
        let outcome = self.array_spread_from(source, arr_val);
        // Consume the source slot (now holding the iterator) on every path.
        self.stack.truncate(len - 1);
        outcome
    }

    /// Body of [`Self::op_array_spread`], split out so the caller drops the
    /// rooted slot on exactly one path. `iter_slot` is that slot: it holds the
    /// source on entry and the iterator for the rest of the call.
    fn array_spread_from(&mut self, source: JsValue, arr_val: JsValue) -> Result<(), VmError> {
        let iter_slot = self.stack.len() - 1;
        let iterator = match self.resolve_iterator(source)? {
            Some(iter @ JsValue::Object(_)) => iter,
            Some(_) => return Err(VmError::type_error("@@iterator must return an object")),
            None => return Err(VmError::type_error("value is not iterable")),
        };
        self.stack[iter_slot] = iterator;
        let result = self.spread_iter_loop(iterator, arr_val);
        if result.is_err() {
            // IteratorClose (§7.4.11): if .return() also throws, its error
            // takes precedence over the original iteration error.
            if let JsValue::Object(iter_id) = iterator {
                let return_key = PropertyKey::String(self.well_known.return_str);
                if let Some(return_result) = get_property(self, iter_id, return_key) {
                    let return_fn = self.resolve_property(return_result, iterator)?;
                    self.call_value(return_fn, iterator, &[])?;
                }
            }
            return result;
        }
        Ok(())
    }

    /// Inner loop for [`op_array_spread`] — extracted so iteration errors can
    /// be caught and `IteratorClose` called before propagating.
    fn spread_iter_loop(&mut self, iterator: JsValue, arr_val: JsValue) -> Result<(), VmError> {
        while let Some(value) = self.iter_next(iterator)? {
            if let JsValue::Object(arr_id) = arr_val {
                let arr = self.get_object_mut(arr_id);
                if let ObjectKind::Array { ref mut elements } = arr.kind {
                    if elements.len() >= DENSE_ARRAY_LEN_LIMIT {
                        return Err(VmError::range_error("Array allocation failed"));
                    }
                    elements.push(value);
                }
            }
        }
        Ok(())
    }

    /// `Op::GetIterator` — call `[Symbol.iterator]()` on the top-of-stack value.
    ///
    /// For objects, looks up `@@iterator` on the object itself (+ prototype chain).
    /// For strings, looks up `@@iterator` on `String.prototype`.
    pub(super) fn op_get_iterator(&mut self) -> Result<(), VmError> {
        let val = self.pop()?;
        if let Some(iter) = self.resolve_iterator(val)? {
            if matches!(iter, JsValue::Object(_)) {
                self.stack.push(iter);
            } else {
                return Err(VmError::type_error("@@iterator must return an object"));
            }
        } else {
            return Err(VmError::type_error("value is not iterable"));
        }
        Ok(())
    }

    /// WebIDL §3.10 named-property exotic for-in collection.
    /// Returns `Some(keys)` when `obj_id` is a `DOMStringMap` or
    /// `Storage` wrapper (sealed wrappers whose enumerable keys
    /// come from the supported-property-names hook, not the
    /// ordinary storage walk).  Returns `None` so the ordinary
    /// for-in path runs for everything else.
    ///
    /// Errors from the underlying handler propagate (a stale
    /// `Entity` or bridge failure on `dataset.keys` / a storage
    /// backend error on the Storage path surfaces as a thrown
    /// exception instead of a silent "no own keys").
    #[cfg(feature = "engine")]
    fn try_named_property_exotic_keys(
        &mut self,
        obj_id: super::value::ObjectId,
    ) -> Result<Option<Vec<super::value::StringId>>, VmError> {
        let kind = self.objects[obj_id.0 as usize].as_ref().map(|o| &o.kind);
        if matches!(kind, Some(ObjectKind::DOMStringMap { .. })) {
            if let Some(result) = super::host::dataset::collect_keys(self, obj_id) {
                return Ok(Some(result?));
            }
        }
        // Storage [[OwnPropertyKeys]] — gated with the `Legacy` Web Storage
        // surface (A2); absent in `App`-profile builds.
        #[cfg(all(feature = "engine", feature = "compat-webapi"))]
        if matches!(
            self.objects[obj_id.0 as usize].as_ref().map(|o| &o.kind),
            Some(ObjectKind::Storage { .. })
        ) {
            if let Some(result) = super::host::storage::collect_keys(self, obj_id) {
                return Ok(Some(result?));
            }
        }
        Ok(None)
    }

    /// `Op::ForInIterator` — collect enumerable string keys from the object
    /// and its prototype chain into a `ForInIterator` object.
    pub(super) fn op_for_in_iterator(&mut self) -> Result<(), VmError> {
        let obj = self.pop()?;
        // Collect enumerable string keys from the object and its
        // prototype chain, skipping shadowed properties.
        let keys = if let JsValue::Object(obj_id) = obj {
            // DOMStringMap (HTMLElement.dataset) for-in: yield only
            // the supported property names (camelCase keys backing
            // each `data-*` attribute) per WebIDL §3.10.  No
            // ordinary own keys are visible (the wrapper is sealed
            // with `extensible: false`), and prototype enumeration
            // skips because `Object.prototype` has no enumerable
            // properties.
            #[cfg(feature = "engine")]
            if let Some(keys) = self.try_named_property_exotic_keys(obj_id)? {
                let iter_obj = self.alloc_object(Object {
                    kind: ObjectKind::ForInIterator(ForInState { keys, index: 0 }),
                    storage: PropertyStorage::shaped(super::shape::ROOT_SHAPE),
                    prototype: None,
                    extensible: true,
                });
                self.stack.push(JsValue::Object(iter_obj));
                return Ok(());
            }
            let mut keys = Vec::new();
            let mut seen = std::collections::HashSet::new();
            let mut current = Some(obj_id);
            // ECMA-262 §10.1.11.1 OrdinaryOwnPropertyKeys: integer indices in ascending numeric order first,
            // then string keys in insertion order.
            if let Some(obj_ref) = self.objects[obj_id.0 as usize].as_ref() {
                // Collect integer indices from elements (non-Empty) + storage.
                let mut index_keys: Vec<(usize, super::value::StringId)> = Vec::new();
                let mut non_index_keys: Vec<super::value::StringId> = Vec::new();
                if let ObjectKind::Array { ref elements } = obj_ref.kind {
                    let mut buf = [0u8; 20];
                    for (i, elem) in elements.iter().enumerate() {
                        if !elem.is_empty() {
                            let s = format_usize(i, &mut buf);
                            let idx_str = self.strings.intern(s);
                            if seen.insert(idx_str) {
                                index_keys.push((i, idx_str));
                            }
                        }
                    }
                }
                // Own storage properties on the first object.
                for (key, attrs) in obj_ref.storage.iter_keys(&self.shapes) {
                    if let PropertyKey::String(sid) = key {
                        if attrs.enumerable && seen.insert(sid) {
                            let units = self.strings.get(sid);
                            if let Some(idx) = super::ops::parse_array_index_u16(units) {
                                index_keys.push((idx, sid));
                            } else {
                                non_index_keys.push(sid);
                            }
                        }
                    }
                }
                index_keys.sort_unstable_by_key(|(idx, _)| *idx);
                keys.extend(index_keys.into_iter().map(|(_, sid)| sid));
                keys.extend(non_index_keys);
                // Continue with prototype chain (skip obj_id, already processed).
                current = obj_ref.prototype;
            }
            // Prototype-chain cap matches `find_inherited_property` /
            // bind depth: prevents attacker-built deep chains from causing
            // unbounded iteration in `for (k in obj)`.
            let mut hops = 0usize;
            while let Some(id) = current {
                if hops >= PROTO_CHAIN_LIMIT {
                    return Err(VmError::range_error(
                        "Prototype chain depth exceeded in for-in iteration",
                    ));
                }
                hops += 1;
                let obj_ref = self.objects[id.0 as usize]
                    .as_ref()
                    .ok_or_else(|| VmError::type_error("cannot iterate freed object"))?;
                for (key, attrs) in obj_ref.storage.iter_keys(&self.shapes) {
                    if let PropertyKey::String(sid) = key {
                        if attrs.enumerable && seen.insert(sid) {
                            keys.push(sid);
                        }
                    }
                }
                current = obj_ref.prototype;
            }
            keys
        } else {
            Vec::new()
        };
        let iter_obj = self.alloc_object(Object {
            kind: ObjectKind::ForInIterator(ForInState { keys, index: 0 }),
            storage: PropertyStorage::shaped(super::shape::ROOT_SHAPE),
            prototype: None,
            extensible: true,
        });
        self.stack.push(JsValue::Object(iter_obj));
        Ok(())
    }

    /// `Op::ForInNext` — advance the for-in iterator, pushing the next key
    /// and a done flag.
    pub(super) fn op_for_in_next(&mut self) -> Result<(), VmError> {
        // Stack: [iterator] → [iterator key done]
        let iter_val = *self
            .stack
            .last()
            .ok_or_else(|| VmError::internal("empty stack on ForInNext"))?;
        if let JsValue::Object(iter_id) = iter_val {
            let iter_obj = self.objects[iter_id.0 as usize]
                .as_mut()
                .ok_or_else(|| VmError::internal("freed for-in iterator"))?;
            if let ObjectKind::ForInIterator(state) = &mut iter_obj.kind {
                if state.index < state.keys.len() {
                    let key_sid = state.keys[state.index];
                    state.index += 1;
                    let key_val = JsValue::String(key_sid);
                    self.stack.push(key_val);
                    self.stack.push(JsValue::Boolean(false)); // not done
                } else {
                    self.stack.push(JsValue::Undefined);
                    self.stack.push(JsValue::Boolean(true)); // done
                }
            } else {
                self.stack.push(JsValue::Undefined);
                self.stack.push(JsValue::Boolean(true));
            }
        } else {
            self.stack.push(JsValue::Undefined);
            self.stack.push(JsValue::Boolean(true));
        }
        Ok(())
    }

    /// `Op::IteratorNext` — call `iterator.next()` and push `value` + `done`.
    pub(super) fn op_iterator_next(&mut self) -> Result<(), VmError> {
        let iter_val = *self
            .stack
            .last()
            .ok_or_else(|| VmError::internal("empty stack on IteratorNext"))?;
        match self.iter_next(iter_val) {
            Ok(Some(value)) => {
                self.stack.push(value);
                self.stack.push(JsValue::Boolean(false));
            }
            Ok(None) => {
                self.stack.push(JsValue::Undefined);
                self.stack.push(JsValue::Boolean(true));
            }
            Err(e) => return Err(e),
        }
        Ok(())
    }

    /// `Op::IteratorRest` — `[iterator -- array]`, collect the remaining
    /// iterator elements into a new array.
    ///
    /// Leaves the iterator **in its stack slot** for the whole drain rather than
    /// popping it, which is what [`Self::collect_iterator`] already does for the
    /// identical loop (it pushes the iterator itself, since its caller holds one
    /// in a Rust local). The collected values were already rooted on the stack;
    /// the iterator was not, and it is re-read on every turn — see
    /// [`Self::op_array_spread`] for why `next()` being a method is not enough
    /// to root it.
    pub(super) fn op_iterator_rest(&mut self) -> Result<(), VmError> {
        let len = self.stack.len();
        if len < 1 {
            return Err(VmError::internal("stack underflow on IteratorRest"));
        }
        let iter_slot = len - 1;
        // Elements collect above the iterator; both are stack roots.
        let stack_root_base = len;
        loop {
            // Re-read by index: the slot, not this copy, is what roots it.
            let iter_val = self.stack[iter_slot];
            match self.iter_next(iter_val) {
                Ok(Some(value)) => {
                    if self.stack.len() - stack_root_base >= DENSE_ARRAY_LEN_LIMIT {
                        // §7.4.11: close iterator on abrupt completion;
                        // if `.return()` throws, that takes precedence
                        // over the range-error.
                        self.stack.truncate(stack_root_base);
                        let close_result = self.iter_close(iter_val);
                        self.stack.truncate(iter_slot);
                        return Err(close_result
                            .err()
                            .unwrap_or_else(|| VmError::range_error("Array allocation failed")));
                    }
                    self.stack.push(value);
                }
                Ok(None) => break,
                Err(e) => {
                    // `.next()` threw — iterator abandoned, no close.
                    self.stack.truncate(iter_slot);
                    return Err(e);
                }
            }
        }
        // Copy elements (keeping originals on stack as GC roots during alloc).
        let elements: Vec<JsValue> = self.stack[stack_root_base..].to_vec();
        // create_array_object may trigger GC — elements are rooted on the stack.
        let arr = self.create_array_object(elements);
        // Now safe to remove the temporary roots and the iterator.
        self.stack.truncate(iter_slot);
        self.stack.push(JsValue::Object(arr));
        Ok(())
    }

    /// `Op::IteratorClose` — call `iterator.return()` if present.
    pub(super) fn op_iterator_close(&mut self) -> Result<(), VmError> {
        let iter_val = self.pop()?;
        self.iter_close(iter_val)
    }

    /// IteratorClose (§7.4.11) on an iterator value already held by the
    /// caller — does not pop from the stack.  Invokes `iterator.return()`
    /// if present; no-op for non-object iterators.  Used by abrupt
    /// completion paths (e.g. `collect_iterator` / `op_iterator_rest`
    /// aborting on `DENSE_ARRAY_LEN_LIMIT`) where the spec requires
    /// closing the iterator and, if `.return()` itself throws, having
    /// that new throw take precedence over the triggering abrupt
    /// completion.
    ///
    /// `iter_val` is rooted on `self.stack` for the duration of the
    /// `.return()` call — without this the iterator would only be
    /// held in a Rust local, and a user-defined `.return()` that
    /// triggers GC could collect it mid-call.  Callers therefore do
    /// not need to root the iterator themselves.
    pub(crate) fn iter_close(&mut self, iter_val: JsValue) -> Result<(), VmError> {
        let JsValue::Object(iter_id) = iter_val else {
            return Ok(());
        };
        self.stack.push(iter_val);
        let return_key = PropertyKey::String(self.well_known.return_str);
        let result = match get_property(self, iter_id, return_key) {
            Some(return_result) => match self.resolve_property(return_result, iter_val) {
                Ok(return_fn) => self.call_value(return_fn, iter_val, &[]).map(|_| ()),
                Err(e) => Err(e),
            },
            None => Ok(()),
        };
        self.stack.pop();
        result
    }
}
