use super::*;

// ---------------------------------------------------------------------------
// Blob / File (WHATWG File API §4-5)
// ---------------------------------------------------------------------------

#[test]
fn blob_constructor_size_type() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var b = new Blob(['hello'], { type: 'text/plain' });
        console.log(b.size === 5 && b.type === 'text/plain');
    ",
    );
}

#[test]
fn blob_slice() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var b = new Blob(['hello world']);
        var sliced = b.slice(0, 5);
        console.log(sliced.size === 5);
    ",
    );
}

#[test]
fn file_constructor() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var f = new File(['content'], 'test.txt', { type: 'text/plain' });
        console.log(f.name === 'test.txt' && f.size === 7 && f.type === 'text/plain' && f.lastModified > 0);
    ",
    );
}
