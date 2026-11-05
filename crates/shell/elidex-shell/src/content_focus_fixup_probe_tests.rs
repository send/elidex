//! Probe: the "update the rendering" step-17 focus fixup (`reconcile_focus`) in
//! `crate::re_render` vs a VM-native `hidden = true` on an ancestor of the focused
//! element. `re_render` gates the fixup on `!mutation_records.is_empty()` (the
//! `session.flush` result), which is usually EMPTY under the VM.

use super::test_support::build_test_content_state;
use elidex_ecs::Entity;

fn by_id(pipeline: &crate::PipelineResult, tag: &str, id: &str) -> Entity {
    pipeline
        .dom
        .query_by_tag(tag)
        .into_iter()
        .find(|&e| {
            pipeline
                .dom
                .world()
                .get::<&elidex_ecs::Attributes>(e)
                .is_ok_and(|a| a.get("id") == Some(id))
        })
        .expect("element with the id exists")
}

fn focused(pipeline: &crate::PipelineResult) -> Option<Entity> {
    elidex_dom_api::focus::current_focus(&pipeline.dom, pipeline.document)
}

/// Control: the same scenario, but the `hidden` write goes through the session
/// mutation buffer (non-empty `flush` records), so the gated fixup runs and
/// focus is dropped. Shows the assertion itself is satisfiable.
#[test]
fn control_session_buffered_hidden_runs_focus_fixup() {
    let (mut state, _browser) =
        build_test_content_state("<div id=d><button id=b>x</button></div>", "");
    let p = &mut state.pipeline;
    let b = by_id(p, "button", "b");
    let d = by_id(p, "div", "d");
    let _ = p.eval_script("document.getElementById('b').focus();");
    crate::re_render(p);
    assert_eq!(focused(p), Some(b), "precondition: b is focused");

    p.session
        .record_mutation(elidex_script_session::Mutation::SetAttribute {
            entity: d,
            name: "hidden".into(),
            value: String::new(),
        });
    crate::re_render(p);
    assert_ne!(
        focused(p),
        Some(b),
        "control: non-empty flush runs the fixup"
    );
}

/// Probe: VM-native `hidden = true`, then one frame with no other mutation.
#[test]
#[ignore = "probe: VM-native hidden=true yields an empty session.flush, so re_render skips reconcile_focus (lib.rs mutation_records gate)"]
fn probe_vm_native_hidden_ancestor_runs_focus_fixup() {
    let (mut state, _browser) =
        build_test_content_state("<div id=d><button id=b>x</button></div>", "");
    let p = &mut state.pipeline;
    let b = by_id(p, "button", "b");
    let _ = p.eval_script("document.getElementById('b').focus();");
    crate::re_render(p);
    assert_eq!(focused(p), Some(b), "precondition: b is focused");

    let _ = p.eval_script("document.getElementById('d').hidden = true;");
    assert!(
        p.dom.has_attribute(by_id(p, "div", "d"), "hidden"),
        "precondition: the VM write landed in the DOM"
    );
    crate::re_render(p);
    assert_ne!(
        focused(p),
        Some(b),
        "HTML update-the-rendering step 17: focus fixup must drop focus from an element in a hidden subtree"
    );
}

// ---- Event-observing cells (plan docs/plans/2026-11-focus-fixup-canonical-steps.md §6) ----

/// Logs `change` / `blur` / `focusout` at `#b` into `<body data-log>`, read
/// back Rust-side with `probe_body_attr` (no extra script bracket).
/// (`listen` takes `t` as a parameter so each listener closes over its own
/// type: the VM's `for (const …)` loop shares one binding across iterations,
/// `#11-vm-per-iteration-loop-environment`.)
const LOGGER: &str = "globalThis.__log = []; \
    function listen(el, t) { \
      el.addEventListener(t, (e) => { \
        __log.push(t); \
        document.body.setAttribute('data-log', __log.join(',')); \
      }); \
    } \
    const __b = document.getElementById('b'); \
    for (const t of ['change', 'blur', 'focusout']) { listen(__b, t); }";

/// `eval_script`, asserting the script completed.
fn eval(p: &mut crate::PipelineResult, src: &str) {
    let r = p.eval_script(src);
    assert!(r.success, "eval failed: {src}: {:?}", r.error);
}

/// The logged events so far, comma-joined (`""` when none fired).
fn event_log(p: &crate::PipelineResult) -> String {
    super::test_support::probe_body_attr(p, "data-log").unwrap_or_default()
}

/// C19 (pin, PM 260): a UA-focused text `<input>` (snapshot seeded) whose value
/// only a script changed, then made non-focusable by a VM-native write: the
/// rendering update's fixup fires `blur` + `focusout` and no `change` (the
/// provenance PR's predicate, reached through the sink).
#[test]
fn focus_fixup_no_change_from_script_alone() {
    let (mut state, _browser) =
        build_test_content_state("<!DOCTYPE html><html><body><input id=b></body></html>", "");
    let p = &mut state.pipeline;
    let b = p.dom.find_by_id(p.document, "b").expect("#b");
    eval(p, LOGGER);
    super::focus::set_focus(p, b);
    crate::re_render(p);
    assert_eq!(focused(p), Some(b), "precondition: b is focused");
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
