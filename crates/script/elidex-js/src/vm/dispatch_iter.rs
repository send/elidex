//! Iterator-protocol opcode handlers extracted from the main dispatch loop.

use super::coerce::get_property;
use super::value::{
    ForInState, JsValue, Object, ObjectKind, PropertyKey, PropertyStorage, VmError,
};
use super::VmInner;

impl VmInner {
    /// Resolve the `@@iterator` for a value and call it to get an iterator object.
    ///
    /// Returns `Ok(Some(iterator))` on success, `Ok(None)` when the value has no
    /// iterator protocol (e.g. numbers, booleans), or propagates call errors.
    fn resolve_iterator(&mut self, val: JsValue) -> Result<Option<JsValue>, VmError> {
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
        loop {
            let Some(value) = self.iter_next(iterator)? else {
                break;
            };
            if let JsValue::Object(arr_id) = arr_val {
                let arr = self.get_object_mut(arr_id);
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
            let mut keys = Vec::new();
            let mut seen = std::collections::HashSet::new();
            let mut current = Some(obj_id);
            while let Some(id) = current {
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
        let mut elements = Vec::new();
        loop {
            match self.iter_next(iter_val) {
                Ok(Some(value)) => elements.push(value),
                Ok(None) => break,
                Err(e) => return Err(e),
            }
        }
        let proto = self.array_prototype;
        let arr = self.alloc_object(Object {
            kind: ObjectKind::Array { elements },
            storage: PropertyStorage::shaped(super::shape::ROOT_SHAPE),
            prototype: proto,
        });
        self.stack.push(JsValue::Object(arr));
        Ok(())
    }

    /// `Op::IteratorClose` — call `iterator.return()` if present.
    pub(super) fn op_iterator_close(&mut self) -> Result<(), VmError> {
        let iter_val = self.pop()?;
        if let JsValue::Object(iter_id) = iter_val {
            let return_key = PropertyKey::String(self.well_known.return_str);
            if let Some(return_result) = get_property(self, iter_id, return_key) {
                let return_fn = self.resolve_property(return_result, iter_val)?;
                self.call_value(return_fn, iter_val, &[])?;
            }
        }
        Ok(())
    }
}
