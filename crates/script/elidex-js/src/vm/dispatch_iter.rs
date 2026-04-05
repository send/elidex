//! Iterator-protocol opcode handlers extracted from the main dispatch loop.

use super::coerce::get_property;
use super::value::{ForInState, JsValue, Object, ObjectKind, PropertyKey, VmError};
use super::Vm;

impl Vm {
    /// Resolve the `@@iterator` for a value and call it to get an iterator object.
    ///
    /// Returns `Ok(Some(iterator))` on success, `Ok(None)` when the value has no
    /// iterator protocol (e.g. numbers, booleans), or propagates call errors.
    fn resolve_iterator(&mut self, val: JsValue) -> Result<Option<JsValue>, VmError> {
        let lookup_id = match val {
            JsValue::Object(id) => Some(id),
            JsValue::String(_) => self.inner.string_prototype,
            _ => None,
        };
        let Some(obj_id) = lookup_id else {
            return Ok(None);
        };
        let iter_key = PropertyKey::Symbol(self.inner.well_known_symbols.iterator);
        let Some(iter_fn) = get_property(&self.inner, obj_id, iter_key) else {
            return Ok(None);
        };
        let result = self.call_value(iter_fn, val, &[])?;
        Ok(Some(result))
    }

    /// `Op::ArraySpread` — spread an iterable into the array on the stack top.
    pub(super) fn op_array_spread(&mut self) -> Result<(), VmError> {
        let source = self.pop()?;
        let arr_val = self.peek()?;
        if let Some(iterator) = self.resolve_iterator(source)? {
            if matches!(iterator, JsValue::Object(_)) {
                let result = self.spread_iter_loop(iterator, arr_val);
                if result.is_err() {
                    // Best-effort IteratorClose on error — ignore close errors.
                    if let JsValue::Object(iter_id) = iterator {
                        let return_key = PropertyKey::String(self.inner.well_known.return_str);
                        if let Some(return_fn) = get_property(&self.inner, iter_id, return_key) {
                            let _ = self.call_value(return_fn, iterator, &[]);
                        }
                    }
                }
                result?;
            }
        }
        Ok(())
    }

    /// Inner loop for [`op_array_spread`] — extracted so iteration errors can
    /// be caught and `IteratorClose` called before propagating.
    fn spread_iter_loop(&mut self, iterator: JsValue, arr_val: JsValue) -> Result<(), VmError> {
        loop {
            let Some(value) = self.iter_next(iterator)? else {
                break;
            };
            if let JsValue::Object(arr_id) = arr_val {
                let arr = self.inner.get_object_mut(arr_id);
                if let ObjectKind::Array { ref mut elements } = arr.kind {
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
    pub(super) fn op_get_iterator(&mut self, entry_frame_depth: usize) -> Result<(), VmError> {
        let val = self.pop()?;
        if let Some(iter) = self.resolve_iterator(val)? {
            self.inner.stack.push(iter);
        } else {
            let err = VmError::type_error("value is not iterable");
            self.throw_error(err, entry_frame_depth)?;
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
            let mut keys = Vec::new();
            let mut seen = std::collections::HashSet::new();
            let mut current = Some(obj_id);
            while let Some(id) = current {
                let obj_ref = self.inner.objects[id.0 as usize]
                    .as_ref()
                    .ok_or_else(|| VmError::type_error("cannot iterate freed object"))?;
                for (key, prop) in &obj_ref.properties {
                    if let PropertyKey::String(sid) = key {
                        if prop.enumerable && seen.insert(*sid) {
                            keys.push(*sid);
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
            properties: Vec::new(),
            prototype: None,
        });
        self.inner.stack.push(JsValue::Object(iter_obj));
        Ok(())
    }

    /// `Op::ForInNext` — advance the for-in iterator, pushing the next key
    /// and a done flag.
    pub(super) fn op_for_in_next(&mut self) -> Result<(), VmError> {
        // Stack: [iterator] → [iterator key done]
        let iter_val = *self
            .inner
            .stack
            .last()
            .ok_or_else(|| VmError::internal("empty stack on ForInNext"))?;
        if let JsValue::Object(iter_id) = iter_val {
            let iter_obj = self.inner.objects[iter_id.0 as usize]
                .as_mut()
                .ok_or_else(|| VmError::internal("freed for-in iterator"))?;
            if let ObjectKind::ForInIterator(state) = &mut iter_obj.kind {
                if state.index < state.keys.len() {
                    let key_sid = state.keys[state.index];
                    state.index += 1;
                    let key_val = JsValue::String(key_sid);
                    self.inner.stack.push(key_val);
                    self.inner.stack.push(JsValue::Boolean(false)); // not done
                } else {
                    self.inner.stack.push(JsValue::Undefined);
                    self.inner.stack.push(JsValue::Boolean(true)); // done
                }
            } else {
                self.inner.stack.push(JsValue::Undefined);
                self.inner.stack.push(JsValue::Boolean(true));
            }
        } else {
            self.inner.stack.push(JsValue::Undefined);
            self.inner.stack.push(JsValue::Boolean(true));
        }
        Ok(())
    }
}
