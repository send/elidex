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

impl VmInner {
    /// Begin a stack-scope: snapshot `stack.len()` and return an
    /// RAII guard that truncates back on drop.
    ///
    /// Companion to [`Self::push_temp_root`].  Use a stack scope
    /// when the rooted region pushes an arbitrary number of values
    /// (e.g. draining an iterator into the stack) — the
    /// single-value identity check on [`VmTempRoot`] doesn't fit
    /// that shape, but the same panic-safe restore semantics are
    /// still required to avoid leaking GC roots through a
    /// `catch_unwind` boundary upstream.
    ///
    /// The guard derefs to `&mut VmInner` so the rooted region is
    /// written as method calls / field accesses on the guard:
    ///
    /// ```rust,ignore
    /// let mut frame = vm.push_stack_scope();
    /// frame.stack.push(JsValue::Object(iter));
    /// while let Some(v) = frame.iter_next(...)? { frame.stack.push(v); }
    /// // … consume `stack[saved_len + 1 ..]` …
    /// // guard drops here; stack restored to pre-snapshot length
    /// ```
    ///
    /// Unlike [`VmTempRoot`] there is no value-identity check —
    /// the guard only enforces the `len` invariant.  Code that
    /// roots a single known value should prefer
    /// [`Self::push_temp_root`] for the stronger contract.
    pub(crate) fn push_stack_scope(&mut self) -> VmStackScope<'_> {
        let saved_len = self.stack.len();
        VmStackScope {
            vm: self,
            saved_len,
        }
    }
}

/// RAII guard for a stack-scope region created via
/// [`VmInner::push_stack_scope`].
///
/// Truncates the VM stack back to the saved length on drop —
/// **including during panic unwinding** — so any rooted values
/// pushed during the scope are released regardless of exit path.
/// Unlike [`VmTempRoot`] no value-identity is asserted on clean
/// drop; the guard is only useful when the rooted region pushes
/// an arbitrary (data-dependent) number of values.
pub(crate) struct VmStackScope<'a> {
    vm: &'a mut VmInner,
    saved_len: usize,
}

impl VmStackScope<'_> {
    /// The stack length captured at scope entry.  Useful when the
    /// scope body needs to compute index offsets relative to the
    /// snapshot point.
    #[inline]
    pub(crate) fn saved_len(&self) -> usize {
        self.saved_len
    }
}

impl std::ops::Deref for VmStackScope<'_> {
    type Target = VmInner;
    #[inline]
    fn deref(&self) -> &VmInner {
        self.vm
    }
}

impl std::ops::DerefMut for VmStackScope<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut VmInner {
        self.vm
    }
}

impl Drop for VmStackScope<'_> {
    fn drop(&mut self) {
        // Truncate unconditionally on every exit (clean return,
        // `?` propagation, panic unwinding alike).  No length
        // assertion: the scope's contract is "len at exit ≤ len at
        // entry doesn't matter, just restore" — over-popping
        // beyond `saved_len` is the only mistake possible, and a
        // bare `truncate` simply caps at the existing length when
        // the stack is shorter, which is the correct no-op.
        self.vm.stack.truncate(self.saved_len);
    }
}
