//! Focus management: focusability checks, focus/blur event dispatch,
//! and change-on-blur for text controls.
//!
//! Focus *state* is the canonical `ElementState::FOCUS` ECS component — read
//! via [`elidex_dom_api::focus::current_focus`] (with its connectedness
//! filter), written via [`elidex_dom_api::focus::set_focus_bit`] (clear-all
//! then set, so single-focus holds by construction). This module runs the
//! UA-input focusing / unfocusing steps (WHATWG HTML §6.6.4) — the event
//! dispatch the engine-independent helpers leave to the caller. The same
//! reconciler serves every UA input path (content thread, the legacy
//! single-thread `App`, and in-process iframes) by operating on a
//! `&mut PipelineResult` (each owns its own document `EcsDom`).

use elidex_dom_api::focus::{
    current_focus, focusing_steps, get_the_focusable_area, is_focusable_area, unfocusing_steps,
    FocusEventKind, FocusEventSink, FocusTrigger,
};
use elidex_ecs::Entity;
use elidex_form::{record_focus_snapshot, take_focus_snapshot, FormControlState};
use elidex_plugin::{EventPayload, FocusEventInit};
use elidex_script_session::DispatchEvent;

use crate::PipelineResult;

/// The WHATWG HTML §6.6.4 focusing-steps step-1 resolution of a pointer click on
/// raw hit `hit` — the focusable area that should receive focus, or `None` when
/// the hit is **not** a focusable area and has no delegate (so focus must not move
/// to it; the caller blurs, per elidex's "click a non-focusable area → blur" rule):
///
/// - hit is not a focusable area but a `delegatesFocus` shadow host (or otherwise
///   retargetable) → its [`get_the_focusable_area`] delegate;
/// - hit **is** itself a §6.6.2 focusable area ([`is_focusable_area`], criterion-2
///   aware) → `hit`;
/// - otherwise — a non-focusable element, OR a `delegatesFocus` host with no
///   focusable delegate (which §6.6.2 criterion 2 says is *not* a focusable area)
///   → `None`: the host must not receive focus itself.
///
/// The gate is [`is_focusable_area`] (criterion-2 aware), **not** the C2-blind
/// [`is_focusable`] — so a `delegatesFocus` host with an empty/non-focusable shadow
/// tree is correctly treated as a non-focusable area and blurs, rather than being
/// focused via the C2-omitting predicate. The retarget itself is still called
/// unconditionally (a `delegatesFocus` host *with* a `tabindex` is not pre-gated
/// out of delegation — PR-A1 plan-review F1). (The editing-host ancestor fallback —
/// focusing-steps step 2 — is PR-A2.)
///
/// Wired here at the content-thread `handle_click` only. The other pointer-click
/// → content-focus entries (in-process iframe content, OOP-iframe content, the
/// legacy single-thread `App`) still focus their raw hit; they converge in PR-A2,
/// when [`set_focus`] becomes a thin caller of the canonical §6.6.4 focus update
/// steps and the retarget moves to that shared seam's head — so the fix is one
/// seam, not a `focus_target_for_click` call sprinkled at every click site
/// (One-issue-one-way: no strangler middle state of N hand-wired call sites).
pub(crate) fn focus_target_for_click(dom: &elidex_ecs::EcsDom, hit: Entity) -> Option<Entity> {
    if let Some(area) = get_the_focusable_area(dom, hit, FocusTrigger::Click) {
        Some(area)
    } else if is_focusable_area(dom, hit) {
        Some(hit)
    } else {
        None
    }
}

/// Move focus to the given entity, clearing focus from the previous target.
///
/// Dispatches the WHATWG HTML §6.6.4 focus update steps' events in spec order:
/// losing side `change` → `blur` → `focusout`, then (after the `FOCUS` bit
/// moves to the new area) gaining side `focus` → `focusin` (UI Events §3.3.2
/// "Focus Event Order"; focusout follows blur per §3.3.4.4). Only focusable
/// elements receive focus (form controls, links with href, elements with
/// tabindex / contenteditable).
pub(crate) fn set_focus(pipeline: &mut PipelineResult, entity: Entity) {
    // N5: only focusable elements receive focus; a non-focusable target blurs the
    // current focus. (The click→nearest-focusable-ancestor climb and the spec's
    // leave-on-no-candidate are PR-A2b; A2a preserves this gate.) The shell
    // overlay [`is_focusable`] adds the form-subsystem fieldset-disabled check the
    // engine-independent predicate cannot see.
    if !is_focusable(&pipeline.dom, entity) {
        blur_current(pipeline);
        return;
    }
    // The canonical WHATWG HTML §6.6.4 focusing steps. The shell sink supplies the
    // engine-bound 3-phase event dispatch + the change-on-blur snapshot; the
    // transition (SoT-last designation, reentrancy, event order) is engine-indep.
    focusing_steps(
        &mut ShellFocusSink { pipeline },
        entity,
        None,
        FocusTrigger::Other,
    );
}

/// Remove focus from the current target without setting a new one.
pub(crate) fn blur_current(pipeline: &mut PipelineResult) {
    // The current holder is connected by construction (gated at focus, cleared at
    // removal via `EcsDom::fire_after_remove`, WHATWG HTML §2.1.4 removing steps),
    // so `current_focus` never misses a stale-detached holder.
    let Some(old) = current_focus(&pipeline.dom, pipeline.document) else {
        return;
    };
    unfocusing_steps(&mut ShellFocusSink { pipeline }, old);
}

/// The shell's [`FocusEventSink`] — adapts the engine-independent §6.6.4
/// transition to the content pipeline: DOM access plus the engine-bound 3-phase
/// event dispatch (`blur`/`focus` + the bubbling `focusout`/`focusin`, UI Events
/// §3.3.2 "Focus Event Order") and the `elidex-form` change-on-blur snapshot.
struct ShellFocusSink<'a> {
    pipeline: &'a mut PipelineResult,
}

impl FocusEventSink for ShellFocusSink<'_> {
    fn dom(&mut self) -> &mut elidex_ecs::EcsDom {
        &mut self.pipeline.dom
    }

    fn document(&self) -> Entity {
        self.pipeline.document
    }

    fn commit_change_on_blur(&mut self, old: Entity) {
        // §6.6.4 step 2.1 — fire `change` if the value was user-edited since
        // focus, consuming the snapshot (a no-op sink — the VM, PR-A2c — leaves it
        // intact). `change` fires while the FOCUS bit is already cleared, so it
        // runs before the new area is designated, per the losing-side ordering.
        dispatch_change_on_blur(self.pipeline, old);
    }

    fn seed_focus_snapshot(&mut self, new: Entity) {
        // §6.6.4 step 4.1 — record the focus-time value for change-on-blur (the
        // canonical transition owns the seed timing; the `elidex-form` call lives
        // here because the engine-independent crate has no form dependency).
        record_focus_snapshot(&mut self.pipeline.dom, new);
    }

    fn fire_focus_event(&mut self, target: Entity, kind: FocusEventKind, related: Option<Entity>) {
        // §6.6.4 fires `blur`/`focus`; the shell adds the bubbling `focusout`
        // (UI Events §3.3.4.4) after `blur` and `focusin` (§3.3.4.3) after
        // `focus`, per §3.3.2 order. The FOCUS bit has already moved (cleared
        // before the losing side, designated before the gaining side), so a
        // `focusin` listener sees `activeElement` at the new element and a
        // `focusout` listener sees `<body>`. `relatedTarget` is the element on the
        // other side of the transition.
        match kind {
            FocusEventKind::Blur => {
                dispatch_focus_event_with_related(self.pipeline, "blur", target, false, related);
                dispatch_focus_event_with_related(self.pipeline, "focusout", target, true, related);
            }
            FocusEventKind::Focus => {
                dispatch_focus_event_with_related(self.pipeline, "focus", target, false, related);
                dispatch_focus_event_with_related(self.pipeline, "focusin", target, true, related);
            }
        }
    }
}

/// Dispatch a focus event with optional related target.
fn dispatch_focus_event_with_related(
    pipeline: &mut PipelineResult,
    event_type: &str,
    target: Entity,
    bubbles: bool,
    related_target: Option<Entity>,
) {
    let mut event = DispatchEvent::new_composed(event_type, target);
    event.cancelable = false;
    event.bubbles = bubbles;
    event.payload = EventPayload::Focus(FocusEventInit {
        related_target: related_target.map(|e| e.to_bits().get()),
    });
    pipeline.dispatch_event(&mut event);
}

/// Dispatch "change" on text-control blur when the value differs from the
/// snapshot taken at focus (HTML §4.10.5.5). Consumes (reads + removes) the
/// `FocusValueSnapshot`; absence ⇒ not a tracked text control ⇒ no change event.
fn dispatch_change_on_blur(pipeline: &mut PipelineResult, entity: Entity) {
    let Some(initial) = take_focus_snapshot(&mut pipeline.dom, entity) else {
        return;
    };
    let changed = pipeline
        .dom
        .world()
        .get::<&FormControlState>(entity)
        .ok()
        .is_some_and(|fcs| fcs.value() != initial);
    if changed {
        // "change" does NOT compose (does not cross shadow boundaries).
        let mut event = DispatchEvent::new("change", entity);
        event.cancelable = false;
        pipeline.dispatch_event(&mut event);
    }
}

/// Check if an element is focusable per HTML §6.6.2.
///
/// **One** focusable-area predicate: the engine-independent
/// [`elidex_dom_api::focus::is_focusable`] (§6.6.2 connectedness + being-rendered
/// [hidden input / hidden subtree] + the tabindex / intrinsic / contenteditable
/// criteria), so the shell UA-input path and the VM `HTMLElement.focus()` writer
/// never diverge. A form control ADDS only the form-subsystem overlay the dom-api
/// layer cannot see — `FormControlState.disabled`, which captures
/// fieldset-inherited disabled (slot `#11-focusable-area-fieldset-inherited-disabled`
/// tracks bringing that to the engine-indep predicate for the VM path).
pub(crate) fn is_focusable(dom: &elidex_ecs::EcsDom, entity: Entity) -> bool {
    if !elidex_dom_api::focus::is_focusable(dom, entity) {
        return false;
    }
    // Form-subsystem overlay: also reject a control disabled via fieldset
    // inheritance (the attribute-only `disabled` is already handled by the
    // dom-api predicate above).
    match dom.world().get::<&FormControlState>(entity) {
        Ok(fcs) => !fcs.disabled,
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom};

    fn text_input(dom: &mut EcsDom, doc: Entity) -> Entity {
        let input = dom.create_element("input", Attributes::default());
        let _ = dom.append_child(doc, input);
        // A default `<input>` becomes a text control via the form subsystem's own
        // constructor (`FormControlState`'s fields are private to `elidex-form`).
        assert!(elidex_form::create_form_control_state(dom, input));
        input
    }

    #[test]
    fn is_focusable_rejects_hidden_form_control() {
        // Codex R7 F2: the form-control branch must honour the dom-api
        // hidden-subtree gate (§6.6.2 being-rendered), so the shell UA-input path
        // and the VM `focus()` path agree on hidden controls instead of diverging.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let input = text_input(&mut dom, doc);
        assert!(
            is_focusable(&dom, input),
            "a connected text input is focusable"
        );
        dom.set_attribute(input, "hidden", "");
        assert!(
            !is_focusable(&dom, input),
            "a hidden form control is not focusable (matches the VM path)"
        );
    }

    #[test]
    fn is_focusable_honours_form_control_disabled_overlay() {
        // The form-subsystem overlay (`FormControlState.disabled`, which captures
        // fieldset-inherited disabled the dom-api layer can't see) still rejects.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let input = text_input(&mut dom, doc);
        if let Ok(mut fcs) = dom.world_mut().get::<&mut FormControlState>(input) {
            fcs.disabled = true;
        }
        assert!(
            !is_focusable(&dom, input),
            "a disabled form control is not focusable"
        );
    }

    #[test]
    fn focus_target_for_click_retargets_through_delegates_focus_host() {
        // PR-A1: the shell click→focus entry runs the §6.6.4 focusing-steps step-1
        // resolution. A click on a `delegatesFocus` shadow host focuses its delegate;
        // a plain focusable element is its own target (no retarget).
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();

        let host = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(doc, host);
        let sr = dom
            .attach_shadow_with_init(
                host,
                elidex_ecs::ShadowInit {
                    mode: elidex_ecs::ShadowRootMode::Open,
                    delegates_focus: true,
                    ..Default::default()
                },
            )
            .expect("attach_shadow on <div>");
        let mut delegate_attrs = Attributes::default();
        delegate_attrs.set("tabindex".to_string(), "0".to_string());
        let delegate = dom.create_element("div", delegate_attrs);
        let _ = dom.append_child(sr, delegate);
        assert_eq!(
            focus_target_for_click(&dom, host),
            Some(delegate),
            "a click on a delegatesFocus host retargets to its shadow delegate"
        );

        let mut plain_attrs = Attributes::default();
        plain_attrs.set("tabindex".to_string(), "0".to_string());
        let plain = dom.create_element("div", plain_attrs);
        let _ = dom.append_child(doc, plain);
        assert_eq!(
            focus_target_for_click(&dom, plain),
            Some(plain),
            "a plain focusable element is its own focus target (no retarget)"
        );
    }

    #[test]
    fn focus_target_for_click_delegates_focus_host_without_delegate_is_not_focusable() {
        // Codex R1 (§6.6.2 criterion 2): a `delegatesFocus` host carrying `tabindex`
        // but an empty/non-focusable shadow tree has no delegate, so it is NOT a
        // focusable area — the click must yield `None` (→ blur), not focus the host.
        // A `is_focusable`-gated fallback (`unwrap_or(hit)`) would wrongly focus it,
        // since `elidex_dom_api::focus::is_focusable` omits criterion 2.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();

        let mut host_attrs = Attributes::default();
        host_attrs.set("tabindex".to_string(), "0".to_string());
        let host = dom.create_element("div", host_attrs);
        let _ = dom.append_child(doc, host);
        let _sr = dom
            .attach_shadow_with_init(
                host,
                elidex_ecs::ShadowInit {
                    mode: elidex_ecs::ShadowRootMode::Open,
                    delegates_focus: true,
                    ..Default::default()
                },
            )
            .expect("attach_shadow on <div tabindex>");
        // Sanity: the host *is* `is_focusable` (tabindex, C2 omitted) — the trap.
        assert!(
            elidex_dom_api::focus::is_focusable(&dom, host),
            "the host is is_focusable via tabindex (C2-blind) — the masked case"
        );
        assert_eq!(
            focus_target_for_click(&dom, host),
            None,
            "a delegatesFocus host with no delegate is not a focusable area → None (blur), not focused"
        );
    }
}
