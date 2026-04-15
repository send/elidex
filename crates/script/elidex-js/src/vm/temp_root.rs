//! RAII temporary GC root guard.
//!
//! Split out of `mod.rs` to keep that file under the project's
//! 1000-line convention.  Engine-feature gated — rooting matters only
//! when host code can produce un-rooted intermediate `JsValue`s
//! (event objects, PromiseRejection synthetic events, etc.).  Without
//! the engine feature there is no host bridge and no caller.

#![cfg(feature = "engine")]

use super::value::{same_value, JsValue};
use super::VmInner;

impl VmInner {
    /// Push `value` onto the VM stack as a temporary GC root and
    /// return an RAII guard that restores the stack on drop.
    ///
    /// See [`VmTempRoot`] for the rooting contract — the guard
    /// derefs to `&mut VmInner` so the rooted region is written as
    /// method calls on the guard:
    ///
    /// ```rust,ignore
    /// let mut g = vm.push_temp_root(JsValue::Object(id));
    /// let _ = g.call(func_id, this, &[arg]);
    /// match g.get_object(id).kind { ... }
    /// // guard drops here; stack restored to pre-push length
    /// ```
    pub(crate) fn push_temp_root(&mut self, value: JsValue) -> VmTempRoot<'_> {
        let saved_len = self.stack.len();
        self.stack.push(value);
        VmTempRoot {
            vm: self,
            saved_len,
            expected: value,
        }
    }
}

/// RAII guard for a temporary GC root pushed onto the VM stack.
///
/// Created via [`VmInner::push_temp_root`].  Restores the stack to
/// its pre-push length on drop, **including during panic unwinding**
/// — this is the key property over a bare `push` / `pop` pair, which
/// would leak the root through a `catch_unwind` boundary upstream
/// and corrupt subsequent GC cycles.
///
/// On normal (non-panic) drop, two release-enforced asserts catch
/// closure-side stack-corruption bugs:
///
/// - **Length check**: stack must end at `saved_len + 1` (no leaked
///   pushes, no over-pops).
/// - **Slot identity**: `stack[saved_len]` must still hold the
///   pushed value (defends against pop-then-push-different which
///   leaves length unchanged but unroots the original).  Comparison
///   uses `same_value` (NaN-safe) since `JsValue::Number(NaN) !=
///   JsValue::Number(NaN)` under JS strict equality.
///
/// During panic unwinding (`std::thread::panicking()`) both asserts
/// are skipped to avoid double-panic process-abort; the stack is
/// truncated unconditionally so any propagation through
/// `catch_unwind` upstream sees a clean stack.
pub(crate) struct VmTempRoot<'a> {
    vm: &'a mut VmInner,
    saved_len: usize,
    expected: JsValue,
}

impl std::ops::Deref for VmTempRoot<'_> {
    type Target = VmInner;
    #[inline]
    fn deref(&self) -> &VmInner {
        self.vm
    }
}

impl std::ops::DerefMut for VmTempRoot<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut VmInner {
        self.vm
    }
}

impl Drop for VmTempRoot<'_> {
    fn drop(&mut self) {
        let stack = &mut self.vm.stack;
        if std::thread::panicking() {
            // Avoid double-panic; just restore.  An assertion failure
            // here while unwinding would abort the process and lose the
            // original panic's diagnostic.
            stack.truncate(self.saved_len);
            return;
        }
        assert_eq!(
            stack.len(),
            self.saved_len + 1,
            "VmTempRoot: rooted region left the VM stack at {} entries, \
             expected {} — GC root corruption hazard",
            stack.len(),
            self.saved_len + 1,
        );
        assert!(
            same_value(stack[self.saved_len], self.expected),
            "VmTempRoot: stack[saved_len] no longer holds the rooted value \
             — rooted region pop'd and re-push'd the slot"
        );
        stack.truncate(self.saved_len);
    }
}
