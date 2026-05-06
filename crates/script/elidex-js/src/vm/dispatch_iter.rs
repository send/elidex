//! Iterator-protocol opcode handlers extracted from the main dispatch loop.

use super::coerce::get_property;
use super::ops::DENSE_ARRAY_LEN_LIMIT;

/// Prototype-chain depth cap for `for-in` key collection.  Matches the
/// cap used by `coerce::find_inherited_property` and bind-chain traversal
/// (10_000); prevents attacker-built deep chains from driving unbounded
/// iteration.
const PROTO_CHAIN_LIMIT: usize = 10_000;
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

    /// `Op::ArraySpread` — spread an iterable into the array on the stack top.
    pub(super) fn op_array_spread(&mut self) -> Result<(), VmError> {
        let source = self.pop()?;
        let arr_val = self.peek()?;
        let iterator = match self.resolve_iterator(source)? {
            Some(iter @ JsValue::Object(_)) => iter,
            Some(_) => return Err(VmError::type_error("@@iterator must return an object")),
            None => return Err(VmError::type_error("value is not iterable")),
        };
        let result = self.spread_iter_loop(iterator, arr_val);
        if result.is_err() {
            // IteratorClose (§7.4.6): if .return() also throws, its error
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
            if matches!(
                self.objects[obj_id.0 as usize].as_ref().map(|o| &o.kind),
                Some(ObjectKind::DOMStringMap { .. })
            ) {
                if let Some(Ok(keys)) = super::host::dataset::collect_keys(self, obj_id) {
                    let iter_obj = self.alloc_object(Object {
                        kind: ObjectKind::ForInIterator(ForInState { keys, index: 0 }),
                        storage: PropertyStorage::shaped(super::shape::ROOT_SHAPE),
                        prototype: None,
                        extensible: true,
                    });
                    self.stack.push(JsValue::Object(iter_obj));
                    return Ok(());
                }
            }
            let mut keys = Vec::new();
            let mut seen = std::collections::HashSet::new();
            let mut current = Some(obj_id);
            // ES §13.7.5.15: integer indices in ascending numeric order first,
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

    /// `Op::IteratorRest` — collect remaining iterator elements into a new array.
    pub(super) fn op_iterator_rest(&mut self) -> Result<(), VmError> {
        let iter_val = self.pop()?;
        // Root collected elements on the stack so GC (triggered by
        // alloc_object) can see them.
        let stack_root_base = self.stack.len();
        loop {
            match self.iter_next(iter_val) {
                Ok(Some(value)) => {
                    if self.stack.len() - stack_root_base >= DENSE_ARRAY_LEN_LIMIT {
                        // §7.4.6: close iterator on abrupt completion;
                        // if `.return()` throws, that takes precedence
                        // over the range-error.
                        self.stack.truncate(stack_root_base);
                        let close_result = self.iter_close(iter_val);
                        return Err(close_result
                            .err()
                            .unwrap_or_else(|| VmError::range_error("Array allocation failed")));
                    }
                    self.stack.push(value);
                }
                Ok(None) => break,
                Err(e) => {
                    // `.next()` threw — iterator abandoned, no close.
                    self.stack.truncate(stack_root_base);
                    return Err(e);
                }
            }
        }
        // Copy elements (keeping originals on stack as GC roots during alloc).
        let elements: Vec<JsValue> = self.stack[stack_root_base..].to_vec();
        // create_array_object may trigger GC — elements are rooted on the stack.
        let arr = self.create_array_object(elements);
        // Now safe to remove the temporary roots.
        self.stack.truncate(stack_root_base);
        self.stack.push(JsValue::Object(arr));
        Ok(())
    }

    /// `Op::IteratorClose` — call `iterator.return()` if present.
    pub(super) fn op_iterator_close(&mut self) -> Result<(), VmError> {
        let iter_val = self.pop()?;
        self.iter_close(iter_val)
    }

    /// IteratorClose (§7.4.6) on an iterator value already held by the
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
