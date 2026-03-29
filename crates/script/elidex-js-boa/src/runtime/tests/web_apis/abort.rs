use super::*;

// ---------------------------------------------------------------------------
// AbortController / AbortSignal (WHATWG DOM §3.2)
// ---------------------------------------------------------------------------

#[test]
fn abort_controller_basic() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var ac = new AbortController();
        var before = ac.signal.aborted;
        ac.abort();
        var after = ac.signal.aborted;
        console.log(!before && after);
    ",
    );
}

#[test]
fn abort_signal_abort_static() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var s = AbortSignal.abort('test reason');
        console.log(s.aborted === true);
    ",
    );
}

#[test]
fn abort_controller_onabort_callback() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var called = false;
        var ac = new AbortController();
        ac.signal.onabort = function() { called = true; };
        ac.abort();
        console.log(called);
    ",
    );
}
