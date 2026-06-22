//! `HTMLSelectElement` / `HTMLOptionsCollection` option tree-mutation handlers.
//!
//! HTML §2.6.4.3 `HTMLOptionsCollection` `add(element, before)` /
//! remove-an-option(index) / `length` setter (`#dom-htmloptionscollection-add` /
//! `#dom-htmloptionscollection-remove` / `#dom-htmloptionscollection-length`).
//! `HTMLSelectElement.add(element, before?)` / `remove(index)` (HTML §4.10.7
//! `#dom-select-add` / `#dom-select-remove`) "act like" the options-collection
//! namesakes, so ONE handler serves both receivers — the VM/boa receiver
//! brand-check resolves both to the same `<select>` entity (the collection root),
//! which is `this` here.
//!
//! # Convergence (B1.2b-2-select)
//!
//! These handlers are the single algorithm home for ALL runtimes (boa/wasm and,
//! post-S5, the VM): they run the §2.6.4.3 add/remove/length steps and mutate
//! through the record-producing `apply_insert_before` / `apply_append_child` /
//! `apply_remove_child` primitives so an option insert/move/remove produces the
//! same `MutationRecord`s as `appendChild`/`insertBefore`/`removeChild` (a move
//! yields the §4.5-adopt source-removal record alongside the destination, NOT
//! suppressed). The VM natives (`vm/host/html_select_proto.rs` +
//! `html_options_collection.rs`) are now thin dispatchers that brand-check the
//! receiver, resolve the WebIDL unions (the `(HTMLElement or long)?` `before`
//! discrimination + `ToInt32`/`ToUint32`), then route here; the prior VM
//! re-implementation of the algorithm is gone (One-issue-one-way).
//!
//! The `before` `(HTMLElement or long)?` discrimination is engine-bound
//! marshalling (it inspects `JsValue`/`ObjectKind` identity to decide the
//! element-vs-long arm) and stays VM-side, mirroring
//! `vm/host/element_insert_adjacent.rs::require_element_arg`. The VM hands this
//! handler EITHER an element `ObjectRef`, a `Number` (the `ToInt32` index), or
//! `Null` — never the raw union. The `element` union (option | optgroup) brand is
//! likewise checked VM-side for the detached-vs-wrong-type message; the
//! `require_option_or_optgroup` tag-guard here is the engine-independent half
//! (sole protection for boa/wasm + defense-in-depth on the VM path).

use elidex_ecs::{Attributes, EcsDom, Entity, TagType};
use elidex_plugin::JsValue;
use elidex_script_session::{
    apply_append_child, apply_insert_before, apply_remove_child, DomApiError, DomApiErrorKind,
    DomApiHandler, JsObjectRef, SessionCore,
};

use crate::util::{not_found_error, require_object_ref_arg};
use crate::{CollectionFilter, CollectionKind, LiveCollection};

/// Engine cap on the option count addressable through the `Options` live
/// collection.
///
/// HTML §2.6.4.3 length-setter step 2.1 caps growth at 100,000; elidex caps at
/// [`elidex_ecs::MAX_ANCESTOR_DEPTH`] (10,000) because the `Options`
/// `LiveCollection` walks children via `children_iter`, which truncates at
/// `MAX_ANCESTOR_DEPTH` — options grown past that point would be unaddressable by
/// `options.item` / `.length` anyway (a deeper incoherence than a lower cap). So
/// the cap is an engine constraint coupled to the child-walk limit, NOT an
/// arbitrary deviation. The spec's "over-cap → return" *shape* is preserved: a
/// target length above the cap is a silent no-op (mirroring the spec's
/// over-100,000 → return), never a clamp. Raising both together is a separate
/// cross-cutting effort (it must lift `MAX_ANCESTOR_DEPTH`-coupled walk limits in
/// lockstep) if a real over-10,000-option need arises.
const MAX_OPTIONS: usize = elidex_ecs::MAX_ANCESTOR_DEPTH;

/// Build a fresh `Options` live collection rooted at `select`.
fn options_collection(select: Entity) -> LiveCollection {
    LiveCollection::new(
        select,
        CollectionFilter::Options,
        CollectionKind::HtmlCollection,
    )
}

/// WebIDL `(HTMLOptionElement or HTMLOptGroupElement)` union: the resolved
/// `element` entity must be an `<option>` or `<optgroup>`. Anything else is a
/// union-conversion failure → **TypeError** (NOT HierarchyRequestError — the
/// pre-convergence VM mis-mapped this). Engine-independent guard: the sole
/// protection on the boa/wasm path (where a `<div>` resolved through the identity
/// map would otherwise be relinked into the select) and defense-in-depth on the
/// VM path, which additionally brand-checks the wrapper to distinguish a detached
/// handle (marshalling the dom-api layer cannot perform).
fn require_option_or_optgroup(dom: &EcsDom, element: Entity) -> Result<(), DomApiError> {
    let is_member = dom
        .world()
        .get::<&TagType>(element)
        .is_ok_and(|t| t.0.eq_ignore_ascii_case("option") || t.0.eq_ignore_ascii_case("optgroup"));
    if is_member {
        Ok(())
    } else {
        Err(DomApiError {
            kind: DomApiErrorKind::TypeError,
            message: "Failed to execute 'add' on 'HTMLOptionsCollection': The element provided \
                      is not an HTMLOptionElement or HTMLOptGroupElement."
                .into(),
        })
    }
}

/// Convert an already-`ToInt32`-coerced WebIDL `long` index `f64` to a
/// non-negative `usize`; negative / non-finite → `None` (= append for `add`,
/// no-op for `remove`). The VM coerces the index through `ToInt32` before
/// dispatch, so the value is an integral `f64` in `i32` range — the cast is exact
/// after the `>= 0` guard (a `boa`/`wasm` caller that forwards a raw number
/// relies on this same conversion).
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn long_to_index(index: f64) -> Option<usize> {
    if !index.is_finite() || index < 0.0 {
        return None;
    }
    Some(index as usize)
}

/// Resolve the `index`th option in `select`'s `Options` live collection
/// (HTML §2.6.4.3 — "the beforeth node in this" / "the indexth element in
/// collection"). A negative index has no node (`add` → append; `remove` → no-op).
/// Single index→node home, folding the deleted VM `resolve_options_index`.
fn options_index_to_node(select: Entity, index: f64, dom: &EcsDom) -> Option<Entity> {
    options_collection(select).item(long_to_index(index)?, dom)
}

/// `HierarchyRequestError` for an option insert that the `apply_*` primitive
/// rejected (cycle / invalid insertion point).
fn hierarchy_error(method: &str) -> DomApiError {
    DomApiError {
        kind: DomApiErrorKind::HierarchyRequestError,
        message: format!("{method}: hierarchy request error (cycle or invalid insertion point)"),
    }
}

// ---------------------------------------------------------------------------
// options.add (HTML §2.6.4.3 `#dom-htmloptionscollection-add`)
// ---------------------------------------------------------------------------

/// `HTMLOptionsCollection.add` / `HTMLSelectElement.add` — `this` is the
/// `<select>` element (the collection root, resolved VM/boa-side).
pub struct OptionsAdd;

impl DomApiHandler for OptionsAdd {
    fn method_name(&self) -> &str {
        "options.add"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let select = this;
        let elem_ref = require_object_ref_arg(args, 0)?;
        let (element, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(elem_ref))
            .ok_or_else(|| not_found_error("element not found"))?;

        // WebIDL union `element` type: option | optgroup, else TypeError.
        require_option_or_optgroup(dom, element)?;

        // step 1: element is an ancestor of select → HierarchyRequestError.
        // (`is_ancestor_or_self` covers the impossible element == select degenerate
        // case as a hierarchy error too — element is an option/optgroup, never the
        // select itself.)
        if dom.is_ancestor_or_self(element, select) {
            return Err(DomApiError {
                kind: DomApiErrorKind::HierarchyRequestError,
                message: "add: element is an ancestor of this select".into(),
            });
        }

        // Resolve `before` per the VM's `(HTMLElement or long)?` marshalling:
        // ObjectRef (an element node, spec step 5 "before is a node"), Number (the
        // `ToInt32` index, step 5 "beforeth node in this"), or Null/absent (append).
        let reference: Option<Entity> = match args.get(1) {
            None | Some(JsValue::Null | JsValue::Undefined) => None,
            Some(JsValue::ObjectRef(r)) => {
                let (before, _) = session
                    .identity_map()
                    .get(JsObjectRef::from_raw(*r))
                    .ok_or_else(|| not_found_error("before not found"))?;
                // step 2: before is an element not a descendant of select → NotFound.
                if !(dom.is_ancestor_or_self(select, before) && before != select) {
                    return Err(not_found_error(
                        "add: before is not a descendant of this select",
                    ));
                }
                // step 3: element == before → return (no-op).
                if element == before {
                    return Ok(JsValue::Undefined);
                }
                Some(before)
            }
            // step 5: integer `before` → beforeth node (None ⇒ append on out-of-range).
            Some(JsValue::Number(n)) => options_index_to_node(select, *n, dom),
            Some(_) => {
                // The VM marshals `before` to ObjectRef | Number | Null exclusively;
                // any other shape means a caller bypassed marshalling.
                return Err(DomApiError::type_error(
                    "add: before must be an Element, integer, or null",
                ));
            }
        };

        // steps 6-7: parent = reference's parent (or select); pre-insert element.
        let records = match reference {
            Some(ref_child) => {
                let parent = dom.get_parent(ref_child).unwrap_or(select);
                apply_insert_before(dom, parent, element, ref_child)
            }
            None => apply_append_child(dom, select, element),
        };
        if records.is_empty() {
            return Err(hierarchy_error("add"));
        }
        for record in records {
            session.push_notify_record(record);
        }
        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// options.remove (HTML §2.6.4.3 remove-an-option / `#dom-htmloptionscollection-remove`)
// ---------------------------------------------------------------------------

/// `HTMLOptionsCollection.remove(index)` / `HTMLSelectElement.remove(index)` —
/// `this` is the `<select>`. Out-of-range / empty is a silent no-op (no record).
pub struct OptionsRemove;

impl DomApiHandler for OptionsRemove {
    fn method_name(&self) -> &str {
        "options.remove"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let select = this;
        let Some(JsValue::Number(index)) = args.first() else {
            // The VM marshals the index through `ToInt32` → Number before dispatch.
            return Err(DomApiError::type_error("remove: index must be an integer"));
        };
        let index = *index;

        // "remove an option from collection given index":
        let mut opts = options_collection(select);
        let count = opts.length(dom);
        // step 1: empty collection → return.
        if count == 0 {
            return Ok(JsValue::Undefined);
        }
        // step 2: index not in [0, count) → return.
        let Some(idx) = long_to_index(index) else {
            return Ok(JsValue::Undefined);
        };
        if idx >= count {
            return Ok(JsValue::Undefined);
        }
        // step 3-4: remove the indexth element from its parent.
        let Some(target) = opts.item(idx, dom) else {
            return Ok(JsValue::Undefined);
        };
        if let Some(parent) = dom.get_parent(target) {
            if let Some(record) = apply_remove_child(dom, parent, target) {
                session.push_notify_record(record);
            }
        }
        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// options.length.set (HTML §2.6.4.3 length setter / `#dom-htmloptionscollection-length`)
// ---------------------------------------------------------------------------

/// `HTMLOptionsCollection.length` setter — `this` is the `<select>`. Grows by
/// appending bare `<option>`s (spec's fragment-append sub-algorithm → ONE
/// coalesced record) or truncates the last *n* options (per-node → *n* records).
pub struct OptionsSetLength;

impl DomApiHandler for OptionsSetLength {
    fn method_name(&self) -> &str {
        "options.length.set"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let select = this;
        let Some(JsValue::Number(value)) = args.first() else {
            // The VM marshals the value through `ToUint32` → Number before dispatch.
            return Err(DomApiError::type_error("length: value must be an integer"));
        };
        // `value` is a non-negative integral f64 in u32 range (VM `ToUint32`);
        // `webidl_unsigned_long` re-applies the `unsigned long` conversion so a
        // boa/wasm caller forwarding a raw number is handled identically.
        let target = crate::util::webidl_unsigned_long(*value);

        // step 2.1: over the engine cap → silent no-op (preserves the spec's
        // >100,000 → return shape; see `MAX_OPTIONS`). Checked BEFORE any
        // allocation so an out-of-range request never creates options.
        if target > MAX_OPTIONS {
            return Ok(JsValue::Undefined);
        }

        // step 1: current = number of options.
        let current = options_collection(select).length(dom);

        if target > current {
            // step 2: append (target − current) new option elements to select.
            // §2.6.4.3 "append new option elements to a select element select given
            // count": fragment whose node document is select's, append count options
            // to it, append the fragment to select. The fragment-append is ONE
            // §4.2.3 childList insertion → ONE coalesced record on select (addedNodes
            // = all the new options), matching the spec algorithm — NOT one record
            // per option.
            let n = target - current;
            let owner = dom.owner_document(select);
            let fragment = dom.create_document_fragment_with_owner(owner);
            for _ in 0..n {
                let opt = dom.create_element_with_owner("option", Attributes::default(), owner);
                // Raw append to the (unobserved) fragment — no record needed; the
                // single observed record is produced by the fragment→select append.
                let _ = dom.append_child(fragment, opt);
            }
            let records = apply_append_child(dom, select, fragment);
            for record in records {
                session.push_notify_record(record);
            }
            // Free the now-empty transient fragment (B1.2b leak discipline — the
            // fragment-expand leaves it childless; never destroy one still holding
            // nodes, e.g. if the append failed).
            if dom.children_iter(fragment).next().is_none() {
                let _ = dom.destroy_entity(fragment);
            }
        } else if target < current {
            // step 3: remove the last (current − target) nodes from their parents —
            // per-node removal (the spec loops), so each is a distinct record.
            let snapshot: Vec<Entity> = options_collection(select).snapshot(dom).to_vec();
            for &opt in snapshot[target..].iter().rev() {
                if let Some(parent) = dom.get_parent(opt) {
                    if let Some(record) = apply_remove_child(dom, parent, opt) {
                        session.push_notify_record(record);
                    }
                }
            }
        }
        Ok(JsValue::Undefined)
    }
}
