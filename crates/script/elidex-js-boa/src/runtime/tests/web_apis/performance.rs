use super::*;

// ---------------------------------------------------------------------------
// Performance Timeline (W3C User Timing §3-4)
// ---------------------------------------------------------------------------

#[test]
fn performance_now_returns_number() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        "console.log(typeof performance.now() === 'number')",
    );
}

#[test]
fn performance_time_origin_is_positive() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        "console.log(performance.timeOrigin > 0)",
    );
}

#[test]
fn performance_mark_and_measure() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        performance.mark('start');
        performance.mark('end');
        var m = performance.measure('test', 'start', 'end');
        console.log(m.entryType === 'measure' && m.name === 'test' && m.duration >= 0);
    ",
    );
}

#[test]
fn performance_get_entries_by_type() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        performance.mark('a');
        performance.mark('b');
        performance.measure('m', 'a', 'b');
        var marks = performance.getEntriesByType('mark');
        var measures = performance.getEntriesByType('measure');
        console.log(marks.length >= 2 && measures.length >= 1);
    ",
    );
}

#[test]
fn performance_clear_marks() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        performance.mark('x');
        performance.clearMarks('x');
        console.log(performance.getEntriesByName('x').length === 0);
    ",
    );
}
