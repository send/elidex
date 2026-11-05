//! The "update the rendering" step-17 focus fixup in `crate::re_render`: it
//! runs every rendering update (ungated, VM-native mutations included) and
//! moves focus through the canonical focusing steps for the viewport
//! (`elidex_dom_api::focus::focus_fixup_steps` via `content::focus::focus_fixup`),
//! so it fires `change` (user-edited text control), `blur` and `focusout`.
//!
//! Plan: `docs/plans/2026-11-focus-fixup-canonical-steps.md` §6 (cells C1–C19,
//! plus C24, the PM 379 extra). Events are observed by listeners that append to
//! a log mirrored into a `<body>` attribute, read back Rust-side with
//! `probe_body_attr` (no extra script bracket).

use super::test_support::{
    build_test_content_state, by_id, eval, event_log, log_listener_js, probe_body_attr, user_key,
    Browser,
};
use super::ContentState;
use elidex_ecs::{ElementState, Entity};

fn focused(pipeline: &crate::PipelineResult) -> Option<Entity> {
    elidex_dom_api::focus::current_focus(&pipeline.dom, pipeline.document)
}

/// Loads `html`, installs `logger` (if any), focuses `#b` via script, runs one
/// frame, and asserts the precondition that `#b` holds focus.
fn focused_on_b_with(html: &str, logger: Option<&str>) -> (ContentState, Browser, Entity) {
    let (mut state, browser) = build_test_content_state(html, "");
    let p = &mut state.pipeline;
    let b = by_id(p, "b");
    if let Some(logger) = logger {
        eval(p, logger);
    }
    eval(p, "document.getElementById('b').focus();");
    crate::re_render(p);
    assert_eq!(focused(p), Some(b), "precondition: b is focused");
    (state, browser, b)
}

fn focused_on_b(html: &str) -> (ContentState, Browser, Entity) {
    focused_on_b_with(html, None)
}

/// UA focus (`content::focus::set_focus`, the shell sink's canonical
/// transition, which seeds the change-on-blur snapshot) on `#b`, then one frame.
fn ua_focused_on_b(html: &str, logger: &str) -> (ContentState, Browser, Entity) {
    let (mut state, browser) = build_test_content_state(html, "");
    let p = &mut state.pipeline;
    let b = by_id(p, "b");
    eval(p, logger);
    super::focus::set_focus(p, b);
    crate::re_render(p);
    assert_eq!(focused(p), Some(b), "precondition: b is focused");
    (state, browser, b)
}

const HIDDEN_FIXTURE: &str = "<div id=d><button id=b>x</button></div>";

/// Logs `change` / `blur` / `focusout` at `#b` into `<body data-log>`.
const LOGGER: &str = concat!(
    log_listener_js!(),
    "const __b = document.getElementById('b'); \
    for (const t of ['change', 'blur', 'focusout']) { listen(__b, t); }"
);

/// As [`LOGGER`], recording `type:related:active` — `related` is `null` when
/// the event's `relatedTarget` is null (or absent, for `change`), and `active`
/// names `document.activeElement` (`body` / `b` / `other`).
const DETAIL_LOGGER: &str = concat!(
    log_listener_js!(),
    "const __b = document.getElementById('b'); \
    function who() { \
      const a = document.activeElement; \
      return a === document.body ? 'body' : a === __b ? 'b' : 'other'; \
    } \
    const fmt = (t, e) => t + ':' + (e.relatedTarget == null ? 'null' : 'set') + ':' + who(); \
    for (const t of ['change', 'blur', 'focusout']) { listen(__b, t, fmt); }"
);

// ---- C1–C4: the fixup runs, ungated ----------------------------------------

/// C3 (control): the `hidden` write goes through the session mutation buffer
/// (non-empty `flush` records); focus is dropped.
#[test]
fn control_session_buffered_hidden_runs_focus_fixup() {
    let (mut state, _browser, _b) = focused_on_b(HIDDEN_FIXTURE);
    let p = &mut state.pipeline;
    let d = by_id(p, "d");
    p.session
        .record_mutation(elidex_script_session::Mutation::SetAttribute {
            entity: d,
            name: "hidden".into(),
            value: String::new(),
        });
    crate::re_render(p);
    assert_eq!(
        focused(p),
        None,
        "control: session-buffered hidden drops focus"
    );
}

/// C1: VM-native `hidden = true` on an ancestor, then one frame with no other
/// mutation.
#[test]
fn vm_native_hidden_ancestor_runs_focus_fixup() {
    let (mut state, _browser, _b) = focused_on_b(HIDDEN_FIXTURE);
    let p = &mut state.pipeline;
    let d = by_id(p, "d");
    eval(p, "document.getElementById('d').hidden = true;");
    assert!(
        p.dom.has_attribute(d, "hidden"),
        "precondition: the VM write landed in the DOM"
    );
    crate::re_render(p);
    assert_eq!(
        focused(p),
        None,
        "HTML update-the-rendering step 17: focus fixup must drop focus from an element in a hidden subtree"
    );
}

/// C4 (negative): a focusable focused element stays focused across rendering
/// updates with no mutation, and nothing fires.
#[test]
fn focusable_focus_survives_idle_rendering_updates() {
    let (mut state, _browser, b) = focused_on_b_with(HIDDEN_FIXTURE, Some(LOGGER));
    let p = &mut state.pipeline;
    crate::re_render(p);
    crate::re_render(p);
    assert_eq!(focused(p), Some(b), "idle updates must not reset focus");
    assert_eq!(event_log(p), "", "idle updates fire nothing");
}

/// C2: second attribute path of the same predicate — VM-native
/// `disabled = true` on the focused `<button>`.
#[test]
fn vm_native_disabled_focused_button_runs_focus_fixup() {
    let (mut state, _browser, b) = focused_on_b("<button id=b>x</button>");
    let p = &mut state.pipeline;
    eval(p, "document.getElementById('b').disabled = true;");
    assert!(
        p.dom.has_attribute(b, "disabled"),
        "precondition: the VM write landed in the DOM"
    );
    crate::re_render(p);
    assert_eq!(focused(p), None, "disabled button must lose focus");
}

// ---- C5–C9: the canonical transition's events and re-entry ----------------

/// C5: `blur` then `focusout` at the holder, related target null, with
/// `activeElement` already `<body>` in both.
#[test]
fn focus_fixup_fires_blur_then_focusout_at_holder() {
    let (mut state, _browser, _b) = focused_on_b_with(HIDDEN_FIXTURE, Some(DETAIL_LOGGER));
    let p = &mut state.pipeline;
    eval(p, "document.getElementById('d').hidden = true;");
    crate::re_render(p);
    assert_eq!(event_log(p), "blur:null:body,focusout:null:body");
    assert_eq!(focused(p), None);
}

/// C6: a user-edited text control made non-focusable by script gets `change`
/// (while still the active element), `blur`, `focusout`; the snapshot is
/// consumed.
#[test]
fn focus_fixup_fires_change_for_edited_text_control() {
    let (mut state, _browser, b) = ua_focused_on_b("<input id=b>", DETAIL_LOGGER);
    user_key(&mut state, "a");
    let p = &mut state.pipeline;
    eval(p, "document.getElementById('b').disabled = true;");
    crate::re_render(p);
    assert_eq!(
        event_log(p),
        "change:null:b,blur:null:body,focusout:null:body"
    );
    assert!(
        p.dom
            .world()
            .get::<&elidex_form::FocusValueSnapshot>(b)
            .is_err(),
        "the change-on-blur snapshot is consumed"
    );
}

/// C7: a `blur` listener's DOM write is laid out by the same `re_render`.
#[test]
fn focus_fixup_listener_mutation_is_rendered_this_frame() {
    let (mut state, _browser, _b) = focused_on_b(HIDDEN_FIXTURE);
    let p = &mut state.pipeline;
    eval(
        p,
        "document.getElementById('b').addEventListener('blur', () => { \
           const q = document.createElement('p'); q.id = 'late'; q.textContent = 'x'; \
           document.body.appendChild(q); });",
    );
    eval(p, "document.getElementById('d').hidden = true;");
    crate::re_render(p);
    let late = p
        .dom
        .find_by_id(p.document, "late")
        .expect("the blur listener ran and appended #late");
    assert!(
        p.dom.world().get::<&elidex_plugin::LayoutBox>(late).is_ok(),
        "the listener's element is laid out this frame"
    );
}

/// C8: a `MutationObserver` sees the `blur` listener's write once, at the
/// listener's checkpoint, with the post-fixup `activeElement`.
#[test]
fn focus_fixup_listener_mutation_is_observed() {
    let (mut state, _browser, _b) = focused_on_b(HIDDEN_FIXTURE);
    let p = &mut state.pipeline;
    eval(
        p,
        "globalThis.__mo = 0; \
         new MutationObserver(() => { \
           __mo += 1; \
           document.body.setAttribute('data-mo', __mo + ':' + \
             (document.activeElement === document.body)); \
         }).observe(document.body, { childList: true }); \
         document.getElementById('b').addEventListener('blur', () => { \
           document.body.appendChild(document.createElement('p')); });",
    );
    eval(p, "document.getElementById('d').hidden = true;");
    crate::re_render(p);
    assert_eq!(probe_body_attr(p, "data-mo").as_deref(), Some("1:true"));
}

/// C9: a `blur` listener's `c.focus()` wins; `b` saw one `blur` and the outer
/// transition designates nothing after it.
#[test]
fn focus_fixup_reentrant_focus_wins() {
    let (mut state, _browser, _b) = focused_on_b_with(
        "<div id=d><button id=b>x</button></div><button id=c>y</button>",
        Some(LOGGER),
    );
    let p = &mut state.pipeline;
    let c = by_id(p, "c");
    eval(
        p,
        "document.getElementById('b').addEventListener('blur', () => \
           document.getElementById('c').focus());",
    );
    eval(p, "document.getElementById('d').hidden = true;");
    crate::re_render(p);
    assert_eq!(focused(p), Some(c), "the reentrant focus() wins");
    assert_eq!(event_log(p), "blur,focusout", "b saw exactly one blur");
}

// ---- C11, C16: iframes ------------------------------------------------------

/// C11: parent VM `iframe.hidden = true` on the focused in-process `<iframe>`:
/// the parent fixup blurs the `<iframe>` element (with `blur`), and pass 2
/// blurs the child's focused `<input>`.
#[test]
fn vm_native_hidden_focused_iframe_blurs_parent_and_child() {
    let (mut state, _browser) =
        build_test_content_state(r#"<iframe id=b srcdoc="<input id=i>"></iframe>"#, "");
    let iframe = by_id(&state.pipeline, "b");
    eval(&mut state.pipeline, LOGGER);
    super::focus::set_focus(&mut state.pipeline, iframe);
    state.focused_iframe = Some(iframe);
    {
        let entry = state.iframes.get_mut(iframe).expect("srcdoc iframe loaded");
        let super::iframe::IframeHandle::InProcess(ip) = &mut entry.handle else {
            panic!("a same-origin srcdoc iframe is in-process");
        };
        let i = by_id(&ip.pipeline, "i");
        super::focus::set_focus(&mut ip.pipeline, i);
        assert_eq!(focused(&ip.pipeline), Some(i), "precondition: child focus");
    }
    assert_eq!(focused(&state.pipeline), Some(iframe), "precondition");

    eval(
        &mut state.pipeline,
        "document.getElementById('b').hidden = true;",
    );
    state.re_render();

    assert_eq!(focused(&state.pipeline), None, "parent focus dropped");
    assert_eq!(state.focused_iframe, None, "focused_iframe cleared");
    let ip = super::test_support::in_process_entry(&state);
    assert_eq!(focused(&ip.pipeline), None, "child focus dropped");
    assert_eq!(
        event_log(&state.pipeline),
        "blur,focusout",
        "the parent fixup fired blur at the <iframe> element"
    );
}

/// C16: an OOP child's own per-frame `crate::re_render` runs the fixup: its
/// `blur` listener posts what it saw.
#[test]
fn oop_iframe_vm_native_hidden_runs_focus_fixup() {
    let loaded = super::test_support::synth_loaded(
        "<div id=d><input id=b></div><script>\
           const b = document.getElementById('b'); \
           b.addEventListener('blur', () => \
             postMessage('blur:' + (document.activeElement === document.body), '*')); \
           b.focus(); \
           document.getElementById('d').hidden = true; \
           postMessage('armed', '*');\
         </script>",
        "https://other.example/",
    );
    let entry = super::iframe::make_out_of_process_entry(
        loaded,
        crate::PreEvalFrameState {
            origin: elidex_plugin::SecurityOrigin::opaque(),
            sandbox_flags: elidex_plugin::parse_sandboxing_directive("allow-scripts"),
            iframe_depth: 1,
            credentialless: false,
            referrer: None,
        },
        crate::ipc::DeviceFacts::default(),
    );
    let posts: Vec<String> = super::test_support::run_oop_and_collect_posts(entry)
        .into_iter()
        .map(|(data, _)| data)
        .collect();
    assert!(posts.iter().any(|d| d == "armed"), "posts: {posts:?}");
    assert_eq!(
        posts
            .iter()
            .filter(|d| d.starts_with("blur:"))
            .collect::<Vec<_>>(),
        vec!["blur:true"],
        "exactly one fixup blur, seen with activeElement === body: {posts:?}"
    );
}

// ---- C17–C19, C24: style, predicate, provenance ----------------------------

/// The resolved `color` of `e`, as a stable string.
fn computed_color(p: &crate::PipelineResult, e: Entity) -> String {
    let style = p
        .dom
        .world()
        .get::<&elidex_plugin::ComputedStyle>(e)
        .expect("ComputedStyle");
    format!("{:?}", style.color)
}

/// C17: the unfocus is styled by the same `re_render` that ran the fixup — an
/// author `:focus` rule no longer applies.
#[test]
fn focus_fixup_unfocus_is_styled_this_frame() {
    let (mut state, _browser) = build_test_content_state(
        "<style>#b:focus{color:rgb(255,0,0)}</style>\
         <button id=b>x</button><button id=r style=\"color:rgb(255,0,0)\">r</button>",
        "",
    );
    let p = &mut state.pipeline;
    let b = by_id(p, "b");
    let red = computed_color(p, by_id(p, "r"));
    super::focus::set_focus(p, b);
    crate::re_render(p);
    assert_eq!(
        computed_color(p, b),
        red,
        "pin: :focus applies while focused"
    );
    eval(p, "document.getElementById('b').disabled = true;");
    crate::re_render(p);
    assert_ne!(computed_color(p, b), red, ":focus no longer applies");
    assert!(
        !p.dom
            .world()
            .get::<&ElementState>(b)
            .is_ok_and(|s| s.contains(ElementState::FOCUS)),
        "the FOCUS bit is cleared"
    );
}

/// C18 (pin, 248 (6)): a `delegatesFocus` host focused by VM `focus()` is not
/// reset — the fixup's predicate is the VM writer's (C2-blind) `is_focusable`.
#[test]
fn delegates_focus_host_focused_by_vm_survives_fixup() {
    let (mut state, _browser) = build_test_content_state("<div id=b tabindex=0></div>", "");
    let p = &mut state.pipeline;
    let h = by_id(p, "b");
    let sr = p
        .dom
        .attach_shadow_with_init(
            h,
            elidex_ecs::ShadowInit {
                mode: elidex_ecs::ShadowRootMode::Open,
                delegates_focus: true,
                ..Default::default()
            },
        )
        .expect("attach shadow");
    let delegate = p
        .dom
        .create_element("input", elidex_ecs::Attributes::default());
    let _ = p.dom.append_child(sr, delegate);
    eval(p, LOGGER);
    eval(p, "document.getElementById('b').focus();");
    assert_eq!(
        focused(p),
        Some(h),
        "precondition: VM focus() kept the host"
    );
    crate::re_render(p);
    crate::re_render(p);
    assert_eq!(focused(p), Some(h));
    assert_eq!(event_log(p), "", "no blur");
}

/// C19 (pin, PM 260): a UA-focused text `<input>` (snapshot seeded) whose value
/// only a script changed, then made non-focusable by a VM-native write: the
/// rendering update's fixup fires `blur` + `focusout` and no `change` (the
/// provenance PR's predicate, reached through the sink).
#[test]
fn focus_fixup_no_change_from_script_alone() {
    let (mut state, _browser, _b) = ua_focused_on_b(
        "<!DOCTYPE html><html><body><input id=b></body></html>",
        LOGGER,
    );
    let p = &mut state.pipeline;
    eval(
        p,
        "document.getElementById('b').value = 'x'; document.getElementById('b').disabled = true;",
    );
    crate::re_render(p);
    assert_eq!(
        event_log(p),
        "blur,focusout",
        "the fixup blurs the holder; a script-only value change fires no change"
    );
    assert_eq!(focused(p), None);
}

/// C24 (extra, PM 379): the regression of the superseded silent un-gating. A
/// user-edited text `<input>` whose ancestor gets a VM-native `hidden`: the
/// next rendering update's fixup runs the canonical focusing steps, firing
/// `change` for the user's value and `blur` — not a silent reset (which fires
/// nothing and leaves the snapshot) nor no fixup at all (the gated base).
#[test]
fn focus_fixup_after_user_edit_fires_change_and_blur_not_a_silent_reset() {
    let (mut state, _browser, b) = ua_focused_on_b("<div id=d><input id=b></div>", LOGGER);
    user_key(&mut state, "a");
    let p = &mut state.pipeline;
    eval(p, "document.getElementById('d').hidden = true;");
    crate::re_render(p);
    assert_eq!(event_log(p), "change,blur,focusout");
    assert_eq!(focused(p), None);
    assert!(
        p.dom
            .world()
            .get::<&elidex_form::FocusValueSnapshot>(b)
            .is_err(),
        "the snapshot is consumed by the transition"
    );
}
