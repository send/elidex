use super::*;

// ---------------------------------------------------------------------------
// window properties
// ---------------------------------------------------------------------------

#[test]
fn window_self_reference() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        "console.log(window === self && window === globalThis)",
    );
}

#[test]
fn window_closed_is_false() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        "console.log(closed === false)",
    );
}

#[test]
fn window_is_secure_context() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        "console.log(typeof window.isSecureContext === 'boolean')",
    );
}

#[test]
fn image_constructor_creates_img() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var img = new Image();
        console.log(typeof img === 'object' && typeof img.setAttribute === 'function');
    ",
    );
}

// ---------------------------------------------------------------------------
// localStorage / sessionStorage
// ---------------------------------------------------------------------------

#[test]
fn session_storage_crud() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        sessionStorage.setItem('k', 'v');
        var got = sessionStorage.getItem('k');
        sessionStorage.removeItem('k');
        console.log(got === 'v' && sessionStorage.getItem('k') === null);
    ",
    );
}

#[test]
fn local_storage_crud() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        localStorage.setItem('k', 'v');
        console.log(localStorage.getItem('k') === 'v' && localStorage.length === 1);
        localStorage.clear();
        console.log(localStorage.length === 0);
    ",
    );
}

// ---------------------------------------------------------------------------
// navigator
// ---------------------------------------------------------------------------

#[test]
fn navigator_properties() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        console.log(
            typeof navigator.userAgent === 'string' &&
            typeof navigator.platform === 'string' &&
            navigator.onLine === true &&
            navigator.cookieEnabled === true
        );
    ",
    );
}

// ---------------------------------------------------------------------------
// console extensions
// ---------------------------------------------------------------------------

#[test]
fn console_time_end() {
    let (mut rt, mut s, mut d, doc) = setup();
    let result = rt.eval(
        r"
        console.time('test');
        console.timeEnd('test');
    ",
        &mut s,
        &mut d,
        doc,
    );
    assert!(
        result.success,
        "console.time/timeEnd error: {:?}",
        result.error
    );
}
