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
