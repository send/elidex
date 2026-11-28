//! `binaryType` getter / setter (IMP-2).

use crate::vm::tests::assert_eval_string;

use super::with_ws_vm;

// ---------------------------------------------------------------------------
// Phase 2: binaryType getter + setter (IMP-2 TypeError on non-enum)
// ---------------------------------------------------------------------------

#[test]
fn binary_type_default_is_blob() {
    with_ws_vm(false, |vm| {
        vm.eval("globalThis.s = new WebSocket('ws://example.com/socket');")
            .unwrap();
        assert_eval_string(vm, "s.binaryType", "blob");
    });
}

#[test]
fn binary_type_setter_accepts_arraybuffer() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis.s = new WebSocket('ws://example.com/socket'); \
             s.binaryType = 'arraybuffer';",
        )
        .unwrap();
        assert_eval_string(vm, "s.binaryType", "arraybuffer");
    });
}

#[test]
fn binary_type_setter_accepts_blob_round_trip() {
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis.s = new WebSocket('ws://example.com/socket'); \
             s.binaryType = 'arraybuffer'; \
             s.binaryType = 'blob';",
        )
        .unwrap();
        assert_eval_string(vm, "s.binaryType", "blob");
    });
}

#[test]
fn binary_type_setter_rejects_other_string_with_type_error() {
    // WebIDL §3.7.6 ignores an unknown enum attribute value. This test
    // records the VM exception (#11-vm-webidl-enum-attribute-invalid-assignment).
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis.s = new WebSocket('ws://example.com/socket'); \
             globalThis.r = null; \
             try { s.binaryType = 'foo'; } catch (e) { globalThis.r = e.name; }",
        )
        .unwrap();
        assert_eval_string(vm, "r", "TypeError");
        // The slot must NOT have changed.
        assert_eval_string(vm, "s.binaryType", "blob");
    });
}

#[test]
fn binary_type_setter_rejects_number_with_type_error() {
    // `42` ToString → "42" → enum mismatch → TypeError.
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis.s = new WebSocket('ws://example.com/socket'); \
             globalThis.r = null; \
             try { s.binaryType = 42; } catch (e) { globalThis.r = e.name; }",
        )
        .unwrap();
        assert_eval_string(vm, "r", "TypeError");
        assert_eval_string(vm, "s.binaryType", "blob");
    });
}

#[test]
fn binary_type_setter_with_undefined_throws_type_error() {
    // `undefined` ToString → "undefined" → enum mismatch → TypeError.
    with_ws_vm(false, |vm| {
        vm.eval(
            "globalThis.s = new WebSocket('ws://example.com/socket'); \
             globalThis.r = null; \
             try { s.binaryType = undefined; } catch (e) { globalThis.r = e.name; }",
        )
        .unwrap();
        assert_eval_string(vm, "r", "TypeError");
    });
}
