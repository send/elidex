//! RegExpBuiltinExec step 13.a: `lastIndex > length` fails the match and, for
//! global/sticky regexps, resets `lastIndex` to 0. Before the fix the stored
//! value was clamped to the subject length, so a zero-width match at the end
//! of a global regexp re-matched forever (script-reachable hang). Expected
//! values are derived from the spec steps, not from any engine.
//!
//! Cells that used to hang run on a spawned thread joined with a deadline so a
//! red run fails instead of hanging CI.

use std::sync::mpsc;
use std::time::Duration;

use super::super::value::JsValue;
use super::super::Vm;

/// Evaluate `src` (which must yield a String) on a worker thread; fail the
/// test if it does not finish within the deadline.
fn eval_str_guarded(src: &'static str) -> String {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut vm = Vm::new();
        let out = match vm.eval(src) {
            Ok(JsValue::String(id)) => vm.get_string(id),
            other => format!("<unexpected: {other:?}>"),
        };
        let _ = tx.send(out);
    });
    rx.recv_timeout(Duration::from_secs(10))
        .unwrap_or_else(|_| panic!("timed out (hang) evaluating: {src}"))
}

#[test]
fn global_replace_zero_width_at_end_terminates() {
    // exec at 1 matches "" (lastIndex 1); empty match advances to 2 > length
    // -> null. Result: "a!".
    assert_eq!(eval_str_guarded("'a'.replace(/$/g, '!')"), "a!");
}

#[test]
fn global_match_empty_pattern_terminates() {
    // Empty matches at 0, 1, 2 (2 == length is allowed), then 3 > length.
    assert_eq!(
        eval_str_guarded("var m = 'ab'.match(/(?:)/g); m.length + ':' + m.join('|')"),
        "3:||"
    );
}

#[test]
fn trim_idiom_global_terminates() {
    // Match "  " at 2..4, then empty at 4 (== length), then 5 > length.
    assert_eq!(eval_str_guarded("'ab  '.replace(/\\s*$/g, '')"), "ab");
}

#[test]
fn exec_global_last_index_beyond_length_returns_null_and_resets() {
    assert_eq!(
        eval_str_guarded(
            "var r = /$/g; r.lastIndex = 5; var m = r.exec('a'); String(m) + ':' + r.lastIndex"
        ),
        "null:0"
    );
}

#[test]
fn exec_sticky_last_index_beyond_length_returns_null_and_resets() {
    assert_eq!(
        eval_str_guarded(
            "var r = /a/y; r.lastIndex = 2; var m = r.exec('a'); String(m) + ':' + r.lastIndex"
        ),
        "null:0"
    );
}

#[test]
fn exec_global_last_index_equal_length_still_matches_empty_at_end() {
    // Step 13.a uses `>` not `>=`: lastIndex == length may match "" at end.
    assert_eq!(
        eval_str_guarded("var r = /$/g; r.lastIndex = 1; var m = r.exec('a'); m.index + ':' + m[0].length + ':' + r.lastIndex"),
        "1:0:1"
    );
}

#[test]
fn exec_non_global_ignores_large_last_index_and_does_not_reset() {
    // Step 7 sets the local lastIndex to 0 for non-g/non-y; step 13.a.i only
    // resets the property for g/y, and step 16 does not write it.
    assert_eq!(
        eval_str_guarded(
            "var r = /a/; r.lastIndex = 99; var m = r.exec('a'); m.index + ':' + r.lastIndex"
        ),
        "0:99"
    );
}

#[test]
fn ordinary_global_replace_still_works() {
    assert_eq!(eval_str_guarded("'aaa'.replace(/a/g, 'b')"), "bbb");
}
