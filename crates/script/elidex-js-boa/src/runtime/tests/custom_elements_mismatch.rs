//! boa mismatched customized-built-in upgrade tests — split from
//! `custom_elements.rs` (1000-line file rule; new coverage goes here
//! instead of growing the monolith).

use super::*;

#[test]
fn already_defined_mismatched_builtin_never_upgrades() {
    // Codex PR331 R15 lineage: `define('x-foo', {extends: 'div'})`
    // followed by `createElement('button', {is: 'x-foo'})` fails the
    // local-name match (`upgrade_matches_local_name`) — the element
    // stays `Undefined` forever: define() can never run again for an
    // already-defined name, so no later world walk ever picks it up.
    // Observable: the constructor never runs.
    let (mut runtime, mut session, mut dom, doc) = setup();
    let result = runtime.eval(
        r"
        globalThis.__count = 0;
        customElements.define('x-foo',
            class { constructor() { globalThis.__count++; } },
            { extends: 'div' });
        var a = document.createElement('button', {is: 'x-foo'});
        var b = document.createElement('button', {is: 'x-foo'});
        ",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "eval should succeed: {:?}", result.error);
    let result = runtime.eval(
        r"console.log('count=' + globalThis.__count);",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(
        result.success,
        "second eval should succeed: {:?}",
        result.error
    );
    let output = runtime.console_output().messages();
    assert!(
        output.iter().any(|m| m.1.contains("count=0")),
        "mismatched built-ins must not upgrade, got: {output:?}"
    );
}
