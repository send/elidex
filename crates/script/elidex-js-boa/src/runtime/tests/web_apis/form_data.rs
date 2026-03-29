use super::*;

// ---------------------------------------------------------------------------
// FormData (WHATWG XHR §4.3)
// ---------------------------------------------------------------------------

#[test]
fn form_data_crud() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var fd = new FormData();
        fd.append('key', 'value');
        console.log(fd.has('key') && fd.get('key') === 'value' && !fd.has('missing'));
    ",
    );
}

#[test]
fn form_data_set_replaces() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var fd = new FormData();
        fd.append('k', 'a');
        fd.append('k', 'b');
        fd.set('k', 'c');
        console.log(fd.getAll('k').length === 1 && fd.get('k') === 'c');
    ",
    );
}

#[test]
fn form_data_delete() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var fd = new FormData();
        fd.append('k', 'v');
        fd.delete('k');
        console.log(!fd.has('k'));
    ",
    );
}

#[test]
fn form_data_foreach() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var fd = new FormData();
        fd.append('a', '1');
        fd.append('b', '2');
        var keys = [];
        fd.forEach(function(v, k) { keys.push(k); });
        console.log(keys.length === 2 && keys[0] === 'a' && keys[1] === 'b');
    ",
    );
}
