use super::*;

// Helper: eval JS and check console output for "true".
fn eval_true(
    runtime: &mut JsRuntime,
    session: &mut SessionCore,
    dom: &mut EcsDom,
    doc: Entity,
    code: &str,
) {
    let result = runtime.eval(code, session, dom, doc);
    assert!(result.success, "JS error: {:?} in: {code}", result.error);
    let msgs = runtime.console_output().messages();
    // Messages are (level, text) tuples.
    assert!(
        msgs.last().is_some_and(|(_, text)| text == "true"),
        "Expected console output 'true', got: {msgs:?}\nCode: {code}"
    );
}

mod abort;
mod blob;
mod document;
mod dom_parser;
mod element;
mod encoding;
mod events;
mod form_data;
mod geometry;
mod performance;
mod url;
mod window;
