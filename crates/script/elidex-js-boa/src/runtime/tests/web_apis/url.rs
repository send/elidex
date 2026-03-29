use super::*;

// ---------------------------------------------------------------------------
// URL / URLSearchParams (WHATWG URL §6)
// ---------------------------------------------------------------------------

#[test]
fn url_constructor_basic() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var u = new URL('https://example.com/path?q=1#frag');
        console.log(u.hostname === 'example.com' && u.pathname === '/path' && u.hash === '#frag');
    ",
    );
}

#[test]
fn url_search_params() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var u = new URLSearchParams('a=1&b=2');
        console.log(u.get('a') === '1' && u.has('b') && !u.has('c'));
    ",
    );
}
