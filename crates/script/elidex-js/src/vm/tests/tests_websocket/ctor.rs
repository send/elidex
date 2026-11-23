//! Constructor — URL validation + `protocols` argument.

use crate::vm::tests::{assert_eval_bool, assert_eval_number, assert_eval_string};
use crate::vm::value::JsValue;

use super::with_ws_vm;

// ---------------------------------------------------------------------------
// Constructor: success path + URL validation
// ---------------------------------------------------------------------------

#[test]
fn ctor_accepts_ws_url() {
    with_ws_vm(false, |vm| {
        vm.eval("globalThis.s = new WebSocket('ws://example.com/socket');")
            .expect("ctor succeeds");
        assert_eval_number(vm, "s.readyState", 0.0);
    });
}

#[test]
fn ctor_accepts_wss_url() {
    with_ws_vm(true, |vm| {
        vm.eval("globalThis.s = new WebSocket('wss://example.com/socket');")
            .expect("wss should succeed on https page");
    });
}

#[test]
fn ctor_promotes_http_to_ws() {
    // WHATWG §9.3.1 step 6: `http://` → `ws://`, `https://` → `wss://`.
    with_ws_vm(false, |vm| {
        vm.eval("globalThis.s = new WebSocket('http://example.com/socket');")
            .expect("http promotes to ws");
        match vm.eval("s.url").unwrap() {
            JsValue::String(id) => {
                let s = vm.get_string(id);
                assert!(s.starts_with("ws://"), "expected ws:// prefix, got: {s}");
            }
            other => panic!("url should be String, got {other:?}"),
        }
    });
}

#[test]
fn ctor_promotes_https_to_wss() {
    with_ws_vm(true, |vm| {
        vm.eval("globalThis.s = new WebSocket('https://example.com/socket');")
            .expect("https promotes to wss");
        match vm.eval("s.url").unwrap() {
            JsValue::String(id) => {
                let s = vm.get_string(id);
                assert!(s.starts_with("wss://"), "expected wss:// prefix, got: {s}");
            }
            other => panic!("url should be String, got {other:?}"),
        }
    });
}

#[test]
fn ctor_rejects_unsupported_scheme() {
    with_ws_vm(false, |vm| {
        let err = vm
            .eval("new WebSocket('ftp://example.com/socket');")
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("Syntax") && msg.contains("scheme"),
            "expected SyntaxError mentioning scheme, got: {msg}"
        );
    });
}

#[test]
fn ctor_rejects_fragment() {
    // WHATWG §9.3.1 step 7: SyntaxError on fragment.
    with_ws_vm(false, |vm| {
        let err = vm
            .eval("new WebSocket('ws://example.com/socket#frag');")
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("Syntax") && msg.contains("fragment"),
            "expected SyntaxError mentioning fragment, got: {msg}"
        );
    });
}

#[test]
fn ctor_rejects_garbage_url() {
    with_ws_vm(false, |vm| {
        let err = vm.eval("new WebSocket('not a url at all');").unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("Syntax"), "expected SyntaxError, got: {msg}");
    });
}

#[test]
fn ctor_rejects_missing_url_arg() {
    with_ws_vm(false, |vm| {
        let err = vm.eval("new WebSocket();").unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("argument") || msg.contains("required"),
            "expected arity error, got: {msg}"
        );
    });
}

#[test]
fn ctor_rejects_mixed_content() {
    // https page + ws:// URL → SecurityError (DOMException).
    with_ws_vm(true, |vm| {
        let err = vm
            .eval("new WebSocket('ws://example.com/socket');")
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("Security") || msg.contains("insecure"),
            "expected SecurityError, got: {msg}"
        );
    });
}

#[test]
fn ctor_requires_new_operator() {
    super::super::assert_ctor_requires_new("WebSocket('ws://example.com/socket')", "WebSocket");
}

// ---------------------------------------------------------------------------
// Constructor: protocols arg
// ---------------------------------------------------------------------------

#[test]
fn ctor_accepts_single_string_protocol() {
    with_ws_vm(false, |vm| {
        vm.eval("new WebSocket('ws://example.com/socket', 'chat');")
            .expect("single string protocol ok");
    });
}

#[test]
fn ctor_accepts_array_of_protocols() {
    with_ws_vm(false, |vm| {
        vm.eval("new WebSocket('ws://example.com/socket', ['chat', 'superchat']);")
            .expect("array of distinct protocols ok");
    });
}

#[test]
fn ctor_rejects_empty_protocol() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "try { new WebSocket('ws://example.com/socket', '') } catch (e) { globalThis.r = e.name + '|' + e.message }",
        )
        .expect("eval ok");
        assert_eval_string(
            vm,
            "r",
            "SyntaxError|Failed to construct 'WebSocket': empty sub-protocol is not allowed",
        );
    });
}

#[test]
fn ctor_rejects_protocol_with_separator() {
    // RFC 9110 §5.6.2 `token`: a comma is a delimiter, not a tchar.
    with_ws_vm(false, |vm| {
        vm.eval(
            "try { new WebSocket('ws://example.com/socket', 'bad,protocol') } catch (e) { globalThis.r = e.name + '|' + e.message }",
        )
        .expect("eval ok");
        assert_eval_string(
            vm,
            "r",
            "SyntaxError|Failed to construct 'WebSocket': sub-protocol 'bad,protocol' contains a character not allowed by RFC 7230 token ABNF",
        );
    });
}

#[test]
fn ctor_rejects_protocol_with_space() {
    // Space (0x20) is below the 0x21..=0x7E token range.
    with_ws_vm(false, |vm| {
        vm.eval(
            "try { new WebSocket('ws://example.com/socket', 'bad protocol') } catch (e) { globalThis.r = e.name + '|' + e.message }",
        )
        .expect("eval ok");
        assert_eval_string(
            vm,
            "r",
            "SyntaxError|Failed to construct 'WebSocket': sub-protocol 'bad protocol' contains a character not allowed by RFC 7230 token ABNF",
        );
    });
}

#[test]
fn ctor_rejects_duplicate_protocols() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "try { new WebSocket('ws://example.com/socket', ['chat', 'chat']) } catch (e) { globalThis.r = e.name + '|' + e.message }",
        )
        .expect("eval ok");
        assert_eval_string(
            vm,
            "r",
            "SyntaxError|Failed to construct 'WebSocket': duplicate sub-protocol 'chat'",
        );
    });
}

#[test]
fn ctor_protocol_collect_precedes_validation() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "try { new WebSocket('ws://example.com/socket', ['', {toString() { throw new TypeError('t') }}]) } catch (e) { globalThis.r = e.name + '|' + e.message }",
        )
        .expect("eval ok");
        assert_eval_string(vm, "r", "TypeError|t");
    });
}

#[test]
fn ctor_protocol_error_is_ecmascript_syntaxerror() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "try { new WebSocket('ws://example.com/socket', '') } catch (e) { globalThis.r = e instanceof SyntaxError }",
        )
        .expect("eval ok");
        assert_eval_bool(vm, "r", true);
    });
}

#[test]
fn ctor_protocol_duplicate_comparison_is_exact() {
    with_ws_vm(false, |vm| {
        vm.eval("new WebSocket('ws://example.com/socket', ['chat', 'Chat']);")
            .expect("case-distinct protocols are not duplicates");
    });
}

#[test]
fn ctor_protocol_validation_order() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "try { new WebSocket('ws://example.com/socket', ['chat', '', 'chat']) } catch (e) { globalThis.r = e.name + '|' + e.message }",
        )
        .expect("eval ok");
        assert_eval_string(
            vm,
            "r",
            "SyntaxError|Failed to construct 'WebSocket': empty sub-protocol is not allowed",
        );
    });
}
