//! Precomputed shape table for [`ObjectKind::Event`] objects.
//!
//! A single event dispatch builds one JS event object per listener
//! invocation (see `events.rs` module doc for the per-listener rebuild
//! rationale).  Without a precomputed shape, each build walks the
//! property transition table 9 + N times — one hashmap lookup per
//! core property, one per payload property — so a mouse event with 5
//! listeners costs ~85 shape-transition hashmap probes + 40
//! `strings.intern` calls just to produce the event objects.
//!
//! Since the final property layout of an event object is fixed by its
//! payload variant (the set of payload keys never changes at runtime),
//! we can build one terminal [`ShapeId`] per variant at VM init time
//! and then allocate event objects directly at that terminal shape.
//! `create_event_object` becomes a single `define_with_precomputed_shape`
//! call with the slot values arranged in shape order.
//!
//! The transition chain is shared — all variants extend the common
//! **core-9** shape (type, bubbles, cancelable, eventPhase, target,
//! currentTarget, timeStamp, composed, isTrusted), so
//! `shape_add_transition`'s built-in deduplication reuses the same
//! prefix ShapeIds across every variant.
//!
//! Variants with no payload keys (`EventPayload::None`, `Scroll`)
//! share the `core` terminal shape directly.  Any non-exhaustive
//! variant the VM doesn't yet recognise also falls through to `core`
//! — those payloads install no extra properties (see the `_`
//! fallthrough in `events::set_payload_properties`).

#![cfg(feature = "engine")]

use super::super::shape::{PropertyAttrs, ShapeId, ROOT_SHAPE};
use super::super::value::{PropertyKey, StringId};
use super::super::VmInner;
use elidex_plugin::EventPayload;

/// Terminal `ShapeId`s for every `EventPayload` variant.
///
/// Built once during `register_globals` (after the payload-key
/// `WellKnownStrings` are interned) and consulted by
/// `create_event_object` on every dispatch.
//
// `#[allow(dead_code)]` on individual fields + `shape_for` until C4
// lands (`create_event_object` rewrite).  Removed there.
#[allow(dead_code)]
pub(crate) struct PrecomputedEventShapes {
    /// Terminal shape for core-9 properties only.  Used for payload
    /// variants that install no extra properties (`None`, `Scroll`)
    /// and as the parent of every other variant's terminal shape.
    pub(crate) core: ShapeId,
    pub(crate) mouse: ShapeId,
    pub(crate) keyboard: ShapeId,
    pub(crate) transition: ShapeId,
    pub(crate) animation: ShapeId,
    pub(crate) input: ShapeId,
    pub(crate) clipboard: ShapeId,
    pub(crate) composition: ShapeId,
    pub(crate) focus: ShapeId,
    pub(crate) wheel: ShapeId,
    pub(crate) message: ShapeId,
    pub(crate) close_event: ShapeId,
    pub(crate) hash_change: ShapeId,
    pub(crate) page_transition: ShapeId,
    pub(crate) storage: ShapeId,
}

#[allow(dead_code)]
impl PrecomputedEventShapes {
    /// Return the terminal shape for `payload`.
    ///
    /// The lookup's match arms mirror `events::set_payload_properties`
    /// 1-to-1 — keeping them in sync is a structural invariant (each
    /// variant's slot order is determined by this shape's
    /// `ordered_entries`, which the set_payload_properties slot-fill
    /// must respect).
    // `EventPayload` is `#[non_exhaustive]` (wildcard required) and
    // the `Scroll | None` arm happens to share the `core` shape with
    // the wildcard — clippy flags them as duplicate, but collapsing
    // them into one arm would lose the explicit enumeration that
    // signals "these are known no-payload variants" vs "unknown
    // future variant".  Keep them separate for documentation.
    #[inline]
    #[allow(clippy::match_same_arms)]
    pub(crate) fn shape_for(&self, payload: &EventPayload) -> ShapeId {
        match payload {
            EventPayload::Mouse(_) => self.mouse,
            EventPayload::Keyboard(_) => self.keyboard,
            EventPayload::Transition(_) => self.transition,
            EventPayload::Animation(_) => self.animation,
            EventPayload::Input(_) => self.input,
            EventPayload::Clipboard(_) => self.clipboard,
            EventPayload::Composition(_) => self.composition,
            EventPayload::Focus(_) => self.focus,
            EventPayload::Wheel(_) => self.wheel,
            EventPayload::Message { .. } => self.message,
            EventPayload::CloseEvent(_) => self.close_event,
            EventPayload::HashChange(_) => self.hash_change,
            EventPayload::PageTransition(_) => self.page_transition,
            EventPayload::Storage { .. } => self.storage,
            EventPayload::Scroll | EventPayload::None => self.core,
            // Unrecognised upstream non-exhaustive variant installs no
            // payload properties (see `events::set_payload_properties`
            // `_` arm), so core-9 alone is the correct terminal shape.
            _ => self.core,
        }
    }
}

impl VmInner {
    /// Walk shape-add transitions for the core 9 event properties
    /// followed by each `EventPayload` variant's payload keys, caching
    /// the terminal `ShapeId` per variant.
    ///
    /// Called exactly once from `register_globals` after the payload
    /// `WellKnownStrings` are interned.  Every `shape_add_transition`
    /// call permanently adds a Shape to `VmInner.shapes` but the cost
    /// is paid once at VM creation (~30 shapes × negligible per-shape
    /// memory) in exchange for eliminating ~17 transition lookups and
    /// ~8 intern calls **per dispatched event** at runtime.
    #[allow(clippy::too_many_lines)]
    pub(in crate::vm) fn build_precomputed_event_shapes(&mut self) -> PrecomputedEventShapes {
        // Core-9 properties installed on every event object.  Order
        // matches `events::create_event_object` → the slot Vec handed
        // to `define_with_precomputed_shape` at runtime must follow
        // the same sequence.
        let core_keys = [
            self.well_known.event_type,
            self.well_known.bubbles,
            self.well_known.cancelable,
            self.well_known.event_phase,
            self.well_known.target,
            self.well_known.current_target,
            self.well_known.time_stamp,
            self.well_known.composed,
            self.well_known.is_trusted,
        ];
        let core = extend(self, ROOT_SHAPE, &core_keys);

        // Payload-specific keys per variant.  Order matches
        // `events::set_payload_properties` — if the payload installer
        // is reordered the matching payload_slots() in events.rs must
        // be updated in lockstep (or the slot values end up in the
        // wrong positions).
        let mouse = extend(
            self,
            core,
            &[
                self.well_known.client_x,
                self.well_known.client_y,
                self.well_known.button,
                self.well_known.buttons,
                self.well_known.alt_key,
                self.well_known.ctrl_key,
                self.well_known.meta_key,
                self.well_known.shift_key,
            ],
        );
        let keyboard = extend(
            self,
            core,
            &[
                self.well_known.key,
                self.well_known.code,
                self.well_known.alt_key,
                self.well_known.ctrl_key,
                self.well_known.meta_key,
                self.well_known.shift_key,
                self.well_known.repeat,
            ],
        );
        let transition = extend(
            self,
            core,
            &[
                self.well_known.property_name,
                self.well_known.elapsed_time,
                self.well_known.pseudo_element,
            ],
        );
        let animation = extend(
            self,
            core,
            &[
                self.well_known.animation_name,
                self.well_known.elapsed_time,
                self.well_known.pseudo_element,
            ],
        );
        let input = extend(
            self,
            core,
            &[
                self.well_known.input_type,
                self.well_known.data,
                self.well_known.is_composing,
            ],
        );
        let clipboard = extend(
            self,
            core,
            &[self.well_known.data_type, self.well_known.data],
        );
        let composition = extend(self, core, &[self.well_known.data]);
        let focus = extend(self, core, &[self.well_known.related_target]);
        let wheel = extend(
            self,
            core,
            &[
                self.well_known.delta_x,
                self.well_known.delta_y,
                self.well_known.delta_mode,
            ],
        );
        let message = extend(
            self,
            core,
            &[
                self.well_known.data,
                self.well_known.origin,
                self.well_known.last_event_id,
            ],
        );
        let close_event = extend(
            self,
            core,
            &[
                self.well_known.close_event_code,
                self.well_known.reason,
                self.well_known.was_clean,
            ],
        );
        let hash_change = extend(
            self,
            core,
            &[self.well_known.old_url, self.well_known.new_url],
        );
        let page_transition = extend(self, core, &[self.well_known.persisted]);
        let storage = extend(
            self,
            core,
            &[
                self.well_known.key,
                self.well_known.old_value,
                self.well_known.new_value,
                self.well_known.url,
            ],
        );

        PrecomputedEventShapes {
            core,
            mouse,
            keyboard,
            transition,
            animation,
            input,
            clipboard,
            composition,
            focus,
            wheel,
            message,
            close_event,
            hash_change,
            page_transition,
            storage,
        }
    }
}

/// Walk `shape_add_transition` for each key under WEBIDL_RO attrs,
/// returning the terminal ShapeId.  Free function (not a closure)
/// because the borrow checker rejects reusing an `&mut self`-capturing
/// closure across sibling calls.
fn extend(vm: &mut VmInner, parent: ShapeId, keys: &[StringId]) -> ShapeId {
    let attrs = PropertyAttrs::WEBIDL_RO;
    let mut s = parent;
    for &k in keys {
        s = vm.shape_add_transition(s, PropertyKey::String(k), attrs);
    }
    s
}
