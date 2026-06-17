//! §6.6.4 canonical focus transition — the **focusing steps**
//! (`#focusing-steps`), **focus update steps** (`#focus-update-steps`) and
//! **unfocusing steps** (`#unfocusing-steps`) of WHATWG HTML §6.6.4, driven by
//! every focus entry point through an injected [`FocusEventSink`].
//!
//! This is the single canonical replacement for the previously hand-rolled
//! per-writer transitions (the shell `set_focus`/`blur_current` + the VM
//! `HTMLElement.focus()`/`blur()`). The transition logic — who designates the
//! [`super::set_focus_bit`] source-of-truth, in what order, and the reentrancy
//! discipline — is engine-independent and lives here; the engine-bound
//! side-effects (3-phase event dispatch + the `elidex-form` change-on-blur
//! snapshot, neither reachable from this crate) are injected via the sink.
//!
//! ## Single-element model
//!
//! elidex designates a single focusable element per document (the canonical
//! [`super::ElementState::FOCUS`] bit). The spec's multi-entry **focus chain**
//! and its step-1 common-ancestor pop are a cross-frame concern (a document's
//! focus chain spans nested navigables); A2a specialises the algorithm to the
//! chain *leaf* — `old`/`new` focusable elements — and defers the chain + pop to
//! PR-A3. For a single document this is exact: the shell fires `blur`/`focus`
//! only at the elements (never the `Document`/viewport), which is what an
//! `[element, Document]` chain with the shared-`Document` tail popped produces.
//!
//! ## Reentrancy (reentrant-wins)
//!
//! A `change`/`blur` listener fired mid-transition may call `focus()`/`blur()`,
//! writing the same FOCUS bit. The losing-side clear runs first, then step 4
//! re-reads [`super::current_focus`]: if a reentrant focus already designated a
//! new area, it **wins** (the outer transition does not clobber it). "Designate
//! the new area last" alone is insufficient — it defends the old clear-then-set
//! race but not the outer-clobbers-inner case.

use elidex_ecs::{EcsDom, Entity};

use super::{
    current_focus, get_the_focusable_area, is_focusable_area, set_focus_bit, FocusTrigger,
};

/// The §6.6.4 focus events the canonical transition fires through the sink. Only
/// `blur`/`focus` are §6.6.4 events; the bubbling `focusin` (UI Events §3.3.4.3)
/// / `focusout` (§3.3.4.4) are the shell sink's engine-bound derivation, ordered
/// per UI Events §3.3.2 "Focus Event Order".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FocusEventKind {
    /// §6.6.4 focus-update step 2.4 — the element losing focus.
    Blur,
    /// §6.6.4 focus-update step 4.4 — the element gaining focus.
    Focus,
}

/// The engine-bound seam of the canonical focus transition: DOM access plus the
/// engine-bound side-effects the engine-independent steps cannot perform
/// themselves — 3-phase event dispatch and the `elidex-form` change-on-blur
/// snapshot (this crate has no `elidex-form` dependency).
///
/// The shell impl (live) wraps the content pipeline. The VM impl (PR-A2c) wraps
/// the host context and may no-op [`fire_focus_event`](Self::fire_focus_event) /
/// [`commit_change_on_blur`](Self::commit_change_on_blur) until the synthetic
/// event-dispatch primitive lands (`#11-vm-host-synthetic-dom-event-dispatch`) —
/// but [`seed_focus_snapshot`](Self::seed_focus_snapshot) and the C2-correct SoT
/// designation run on every engine. This is a single injected dispatcher per
/// engine (cf. `MutationDispatcher`), not an observer registry.
pub trait FocusEventSink {
    /// The bound document's DOM — the transition's single mutable resource.
    fn dom(&mut self) -> &mut EcsDom;

    /// The bound (active) document.
    fn document(&self) -> Entity;

    /// §6.6.4 focus-update step 2.1 — fire `change` if a focused text control's
    /// value was user-edited since focus. **Owns the change-on-blur snapshot
    /// consume**: the shell takes the snapshot, compares, and dispatches; a
    /// no-op impl (VM, until its dispatch primitive lands) leaves the snapshot
    /// intact so the later dispatch can still observe the focus-time value.
    fn commit_change_on_blur(&mut self, old: Entity);

    /// §6.6.4 focus-update step 4.1 — seed the change-on-blur snapshot for the
    /// element gaining focus. Runs on **every** engine (the snapshot is
    /// `elidex-form` state, not event dispatch), so the canonical transition
    /// owns the seed *timing* while the form-layer call lives behind the sink.
    fn seed_focus_snapshot(&mut self, new: Entity);

    /// §6.6.4 focus-update steps 2.2-2.4 / 4.2-4.4 — fire a `blur`/`focus` event
    /// at `target`, with `related` the element on the other side of the
    /// transition. (The shell additionally fires the bubbling `focusin`/
    /// `focusout`; the VM no-ops until its dispatch primitive lands.)
    fn fire_focus_event(&mut self, target: Entity, kind: FocusEventKind, related: Option<Entity>);
}

/// §6.6.4 **focusing steps** (`#focusing-steps`) — move focus to `new_target`,
/// resolving it to a focusable area first.
///
/// `fallback` is the spec's optional fallback target (dialog / autofocus
/// callers); the shell UA path passes `None` — its candidate is pre-resolved and
/// the click→ancestor climb is PR-A2b. `trigger` selects the
/// get-the-focusable-area `"click"` vs `"other"` behaviour.
pub fn focusing_steps(
    sink: &mut dyn FocusEventSink,
    new_target: Entity,
    fallback: Option<Entity>,
    trigger: FocusTrigger,
) {
    // Step 1 — resolve a non-focusable-area target to its focusable area.
    let target = if is_focusable_area(sink.dom(), new_target) {
        new_target
    } else {
        match get_the_focusable_area(sink.dom(), new_target, trigger) {
            Some(area) => area,
            // Step 2 — null candidate: use the fallback target, else leave focus
            // unchanged (no fallback ⇒ return).
            None => match fallback {
                Some(fallback_target) => fallback_target,
                None => return,
            },
        }
    };
    // Step 3 (navigable container → content document) — unmodelled (iframe focus
    // = PR-A2b / `#11-oop-iframe-focus-lifecycle`). Step 4 (inert) — `inert`
    // unmodelled. Both no-op.
    let doc = sink.document();
    let old = current_focus(sink.dom(), doc);
    // Step 5 — already the focused area: nothing to do. (Also preserves the
    // change-on-blur reseed suppression — a re-`focus()` after a user edit must
    // not refresh the snapshot baseline, returning here before step 4's seed.)
    if old == Some(target) {
        return;
    }
    // Steps 6-8 — the transition (single-element model; chain + pop = PR-A3).
    focus_update_steps(sink, old, Some(target));
}

/// §6.6.4 **unfocusing steps** (`#unfocusing-steps`) — the blur path for
/// `old_target`.
///
/// A2a scope: the shell `blur_current` passes the *current* focused element,
/// which is always a focusable area on the focus chain, so step 1 (a
/// delegatesFocus host → focused-area retarget — needs a host `old_target`, only
/// reachable via VM `element.blur()`, PR-A2c), step 3 (area/scrollable retarget
/// — unmodelled) and steps 5-6 (old ∉ chain / not a focusable area — always
/// satisfied here) reduce to the steps 7-8 transition with an empty new chain.
pub fn unfocusing_steps(sink: &mut dyn FocusEventSink, old_target: Entity) {
    focus_update_steps(sink, Some(old_target), None);
}

/// §6.6.4 **focus update steps** (`#focus-update-steps`), specialised to the
/// single-element model: `old`/`new` are the focusable elements (chain leaves).
fn focus_update_steps(sink: &mut dyn FocusEventSink, old: Option<Entity>, new: Option<Entity>) {
    // Step 2 — losing side. elidex browser-parity divergence: clear the FOCUS
    // bit *before* the losing events so `document.activeElement` / `hasFocus`
    // report `<body>` during `change`/`blur`/`focusout` (matches real browsers +
    // the pre-A2a shell; the literal spec keeps `old` designated until step 4).
    // It also makes the step-4 reentrancy test an exact `current_focus.is_some()`.
    set_focus_bit(sink.dom(), None);
    if let Some(old) = old {
        sink.commit_change_on_blur(old); // step 2.1 — change (snapshot consume)
        sink.fire_focus_event(old, FocusEventKind::Blur, new); // steps 2.2-2.4
    }
    // Step 3 — platform conventions: no-op.
    // Step 4 — reentrancy gate (reentrant-wins): a `change`/`blur` listener may
    // have called `focus()`/`blur()`, designating a new focused area after the
    // step-2 clear. If focus already moved, that reentrant focus wins.
    let doc = sink.document();
    if current_focus(sink.dom(), doc).is_some() {
        return;
    }
    if let Some(new) = new {
        set_focus_bit(sink.dom(), Some(new)); // step 4.1 — designate (SoT last)
        sink.seed_focus_snapshot(new); // step 4.1 — seed the change-on-blur snapshot
        sink.fire_focus_event(new, FocusEventKind::Focus, old); // steps 4.2-4.4
    }
}

#[cfg(test)]
mod tests {
    use super::{focusing_steps, unfocusing_steps, FocusEventKind, FocusEventSink};
    use crate::focus::{current_focus, set_focus_bit, FocusTrigger};
    use elidex_ecs::{Attributes, EcsDom, Entity};

    fn focusable_div(dom: &mut EcsDom) -> Entity {
        let mut attrs = Attributes::default();
        attrs.set("tabindex".to_string(), "0".to_string());
        dom.create_element("div", attrs)
    }

    #[derive(Default)]
    struct Recorded {
        events: Vec<(Entity, FocusEventKind, Option<Entity>)>,
        changes: Vec<Entity>,
        seeds: Vec<Entity>,
    }

    struct TestSink<'a> {
        dom: &'a mut EcsDom,
        document: Entity,
        rec: Recorded,
        /// On the first `blur`, set focus to this entity — simulating a reentrant
        /// `focus()` from a `blur`/`change` listener (exercises the reentrant-wins
        /// gate without re-entering the public API).
        reentrant_on_blur: Option<Entity>,
    }

    impl<'a> TestSink<'a> {
        fn new(dom: &'a mut EcsDom, document: Entity) -> Self {
            Self {
                dom,
                document,
                rec: Recorded::default(),
                reentrant_on_blur: None,
            }
        }
    }

    impl FocusEventSink for TestSink<'_> {
        fn dom(&mut self) -> &mut EcsDom {
            self.dom
        }
        fn document(&self) -> Entity {
            self.document
        }
        fn commit_change_on_blur(&mut self, old: Entity) {
            self.rec.changes.push(old);
        }
        fn seed_focus_snapshot(&mut self, new: Entity) {
            self.rec.seeds.push(new);
        }
        fn fire_focus_event(
            &mut self,
            target: Entity,
            kind: FocusEventKind,
            related: Option<Entity>,
        ) {
            self.rec.events.push((target, kind, related));
            if kind == FocusEventKind::Blur {
                if let Some(reentrant) = self.reentrant_on_blur.take() {
                    set_focus_bit(self.dom, Some(reentrant));
                }
            }
        }
    }

    fn doc_with_two(dom: &mut EcsDom) -> (Entity, Entity, Entity) {
        let doc = dom.create_document_root();
        let a = focusable_div(dom);
        let b = focusable_div(dom);
        let _ = dom.append_child(doc, a);
        let _ = dom.append_child(doc, b);
        (doc, a, b)
    }

    #[test]
    fn focusing_steps_designates_last_and_fires_in_spec_order() {
        let mut dom = EcsDom::new();
        let (doc, a, b) = doc_with_two(&mut dom);
        set_focus_bit(&mut dom, Some(a));

        let rec = {
            let mut sink = TestSink::new(&mut dom, doc);
            focusing_steps(&mut sink, b, None, FocusTrigger::Other);
            sink.rec
        };

        // change(a) → blur(a, related=b) → [designate b] → focus(b, related=a).
        assert_eq!(rec.changes, vec![a]);
        assert_eq!(
            rec.events,
            vec![
                (a, FocusEventKind::Blur, Some(b)),
                (b, FocusEventKind::Focus, Some(a)),
            ]
        );
        assert_eq!(rec.seeds, vec![b], "the gaining element seeds the snapshot");
        assert_eq!(current_focus(&dom, doc), Some(b));
    }

    #[test]
    fn focusing_steps_already_focused_is_a_no_op() {
        let mut dom = EcsDom::new();
        let (doc, a, _b) = doc_with_two(&mut dom);
        set_focus_bit(&mut dom, Some(a));

        let rec = {
            let mut sink = TestSink::new(&mut dom, doc);
            focusing_steps(&mut sink, a, None, FocusTrigger::Other);
            sink.rec
        };

        // Step 5 early-out: no events, and crucially NO reseed (a re-focus after a
        // user edit must not refresh the change-on-blur baseline).
        assert!(rec.events.is_empty());
        assert!(rec.seeds.is_empty());
        assert!(rec.changes.is_empty());
        assert_eq!(current_focus(&dom, doc), Some(a));
    }

    #[test]
    fn focusing_steps_non_focusable_no_fallback_leaves_focus() {
        let mut dom = EcsDom::new();
        let (doc, a, _b) = doc_with_two(&mut dom);
        let plain = dom.create_element("span", Attributes::default()); // not a focusable area
        let _ = dom.append_child(doc, plain);
        set_focus_bit(&mut dom, Some(a));

        let rec = {
            let mut sink = TestSink::new(&mut dom, doc);
            focusing_steps(&mut sink, plain, None, FocusTrigger::Other);
            sink.rec
        };

        // Step 2 (null candidate, no fallback) → return: focus is LEFT unchanged.
        assert!(rec.events.is_empty());
        assert_eq!(current_focus(&dom, doc), Some(a));
    }

    #[test]
    fn unfocusing_steps_blurs_and_clears() {
        let mut dom = EcsDom::new();
        let (doc, a, _b) = doc_with_two(&mut dom);
        set_focus_bit(&mut dom, Some(a));

        let rec = {
            let mut sink = TestSink::new(&mut dom, doc);
            unfocusing_steps(&mut sink, a);
            sink.rec
        };

        // change(a) → blur(a, related=None); empty new chain → no designation.
        assert_eq!(rec.changes, vec![a]);
        assert_eq!(rec.events, vec![(a, FocusEventKind::Blur, None)]);
        assert!(rec.seeds.is_empty());
        assert_eq!(current_focus(&dom, doc), None);
    }

    #[test]
    fn reentrant_focus_during_blur_wins() {
        let mut dom = EcsDom::new();
        let (doc, a, b) = doc_with_two(&mut dom);
        let c = focusable_div(&mut dom);
        let _ = dom.append_child(doc, c);
        set_focus_bit(&mut dom, Some(a));

        let rec = {
            let mut sink = TestSink::new(&mut dom, doc);
            // A `blur` listener on `a` calls `c.focus()` mid-transition.
            sink.reentrant_on_blur = Some(c);
            focusing_steps(&mut sink, b, None, FocusTrigger::Other);
            sink.rec
        };

        // The reentrant focus on `c` wins: `b` is NEVER designated, no focus(b)
        // event fires, and the outer transition does not clobber `c`.
        assert_eq!(current_focus(&dom, doc), Some(c));
        assert_eq!(rec.events, vec![(a, FocusEventKind::Blur, Some(b))]);
        assert!(rec.seeds.is_empty(), "the outer transition seeds nothing");
    }
}
