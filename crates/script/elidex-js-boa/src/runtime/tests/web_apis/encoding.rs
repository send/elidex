use super::*;

// ---------------------------------------------------------------------------
// atob / btoa (WHATWG HTML §8.3)
// ---------------------------------------------------------------------------

#[test]
fn btoa_atob_roundtrip() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        "console.log(atob(btoa('Hello, World!')) === 'Hello, World!')",
    );
}

// ---------------------------------------------------------------------------
// crypto (W3C WebCrypto)
// ---------------------------------------------------------------------------

#[test]
fn crypto_random_uuid() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var uuid = crypto.randomUUID();
        console.log(uuid.length === 36 && uuid[8] === '-' && uuid[13] === '-');
    ",
    );
}

// ---------------------------------------------------------------------------
// TextEncoder / TextDecoder (WHATWG Encoding §8)
// ---------------------------------------------------------------------------

#[test]
fn text_encoder_decode_roundtrip() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var enc = new TextEncoder();
        var dec = new TextDecoder();
        var bytes = enc.encode('hello');
        console.log(dec.decode(bytes) === 'hello');
    ",
    );
}

// ---------------------------------------------------------------------------
// queueMicrotask
// ---------------------------------------------------------------------------

#[test]
fn queue_microtask_executes() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var executed = false;
        queueMicrotask(function() { executed = true; });
        console.log(executed);
    ",
    );
}
