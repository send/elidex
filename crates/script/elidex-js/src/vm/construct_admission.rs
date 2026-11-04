//! Construct admission: the IsConstructor abstract operation
//! (ECMA-262 §7.2.4) and its application at every `[[Construct]]`
//! entry.
//!
//! Holds three items, one IsConstructor unit:
//! - [`classify_construct_target`], the classifier: whether a non-bound
//!   object has a `[[Construct]]` internal method, and if not, which
//!   refusal it gets ([`ConstructAdmission`]);
//! - [`is_constructor`], the predicate (the class heritage check,
//!   `customElements.define`, the `%TypedArray%` statics);
//! - [`VmInner::admit_construct`], the admission helper both construct
//!   entries call before any callee effect — `do_new` (`new`) and
//!   `construct_synchronous` (`super(...)`, written or in the default
//!   derived constructor, and the custom-element upgrade).
//!
//! The bound-chain walk is one depth loop in `vm/` core, the private
//! `walk_bound_chain` in `vm/dispatch_class.rs`.
//! `unwrap_bound_function_chain` (collects the bound arguments; the
//! admission uses it) and `bound_chain_target` (the target only; the
//! predicate uses it) are its two forms.  The TypedArray resolver's
//! walker (`unwrap_bound_chain` in `vm/host/typed_array_static.rs`) is
//! `#11-vm-typed-array-create-construct`'s.

use super::value::{CallShape, JsValue, ObjectId, ObjectKind, StringId, VmError};
use super::VmInner;

/// The classifier's answer for a non-bound object: has it a
/// `[[Construct]]` internal method, and if not, which refusal applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConstructAdmission {
    /// Has `[[Construct]]`.
    Construct,
    /// A JS function MakeConstructor never applied to, or any object
    /// that is not a function at all.
    NotConstructor,
    /// A built-in without `[[Construct]]` (`CallShape::CallableOnly`);
    /// carries the function's name for the message.
    CallableOnlyNative(StringId),
    /// A WebIDL interface object with no constructor operation
    /// (`CallShape::IllegalConstructor`); carries the interface name.
    IllegalConstructorNative(StringId),
}

/// Which `[[Construct]]` entry is admitting the callee.  Only the
/// non-Object refusal (IsConstructor step 1) is keyed by it, so each
/// entry keeps its message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConstructEntry {
    /// EvaluateNew (§13.3.5.1.1), and the custom-element upgrade, whose
    /// IsConstructor comes from WebIDL "construct a callback function"
    /// step 3 (§3.12; HTML "upgrade an element" §4.13.5 step 10.3,
    /// "constructing C").  The upgrade always passes an Object, so the
    /// key is only read by the non-Object arm.
    New,
    /// SuperCall Evaluation (§13.3.7.1): the callee is
    /// GetSuperConstructor's result, an Object or `null`.
    Super,
}

/// Classify a **non-bound** object for IsConstructor (§7.2.4).
///
/// - JS function: reads `CompiledFunction::has_construct` and nothing
///   else — the compiler sets it when MakeConstructor (§10.2.5) applies
///   to the creation path.
/// - Native function: a built-in has `[[Construct]]` "if and only if"
///   it is described as a constructor or an algorithm sets it (§10.3
///   Built-in Function Objects), which the VM records as its
///   `CallShape`.  Whether a WebIDL interface object without a
///   constructor operation is a constructor is
///   `#11-webidl-interface-object-isconstructor`'s question; the split
///   below is unchanged until it is settled.
/// - Every other kind (a `BoundFunction` included — callers unwrap
///   first) is `NotConstructor`, so a kind nobody classified is refused.
pub(crate) fn classify_construct_target(vm: &VmInner, id: ObjectId) -> ConstructAdmission {
    match &vm.get_object(id).kind {
        ObjectKind::Function(fo) => {
            if vm.get_compiled(fo.func_id).has_construct {
                ConstructAdmission::Construct
            } else {
                ConstructAdmission::NotConstructor
            }
        }
        ObjectKind::NativeFunction(nf) => match nf.shape {
            CallShape::Ordinary | CallShape::ConstructorOnly => ConstructAdmission::Construct,
            CallShape::CallableOnly => ConstructAdmission::CallableOnlyNative(nf.name),
            CallShape::IllegalConstructor => ConstructAdmission::IllegalConstructorNative(nf.name),
        },
        _ => ConstructAdmission::NotConstructor,
    }
}

/// `IsConstructor(value)` (ECMA-262 §7.2.4) for an Object: true when it
/// has a `[[Construct]]` internal method.
///
/// A bound function has one iff its target does (BoundFunctionCreate
/// §10.4.1.3 step 6), so the chain is unwrapped with
/// `bound_chain_target` — the target-only form of the walk the
/// construct entries share (`unwrap_bound_function_chain`) — and the
/// target classified by [`classify_construct_target`].  A chain past
/// [`crate::vm::MAX_BIND_CHAIN_DEPTH`] answers `false`: a predicate
/// answers rather than throws, and its callers (ClassDefinitionEvaluation
/// step 8.g, `customElements.define`, the `%TypedArray%` statics) raise
/// their own TypeError.
pub(crate) fn is_constructor(vm: &VmInner, id: ObjectId) -> bool {
    vm.bound_chain_target(id)
        .is_ok_and(|target| classify_construct_target(vm, target) == ConstructAdmission::Construct)
}

impl VmInner {
    /// Admit `callee` at a `[[Construct]]` entry, or refuse it: the
    /// IsConstructor test of EvaluateNew (§13.3.5.1.1) step 5, SuperCall
    /// Evaluation (§13.3.7.1) step 5 and ClassDefinitionEvaluation
    /// (§15.7.14) step 14.a.iv.3 (the default derived constructor's
    /// `super(...args)`), so that Construct (§7.3.14), whose caller
    /// guarantees a constructor, is only reached with one.
    ///
    /// Owns IsConstructor step 1 — a non-Object is refused with the
    /// `entry`'s message — then unwraps the bound chain (keeping
    /// `unwrap_bound_function_chain`'s depth RangeError) and classifies
    /// the target; checking the target is IsConstructor of the wrapper
    /// by BoundFunctionCreate (§10.4.1.3) step 6.  Returns the target
    /// and the bound arguments to prepend.
    ///
    /// Both entries call it after the arguments are evaluated and before
    /// any allocation, frame push, stack splice or callee body.
    pub(crate) fn admit_construct(
        &self,
        callee: JsValue,
        entry: ConstructEntry,
    ) -> Result<(ObjectId, Vec<JsValue>), VmError> {
        let JsValue::Object(id) = callee else {
            return Err(VmError::type_error(match entry {
                ConstructEntry::New => "not a constructor",
                ConstructEntry::Super => "Super constructor null is not a constructor",
            }));
        };
        let (target, prepended) = self.unwrap_bound_function_chain(id)?;
        match classify_construct_target(self, target) {
            ConstructAdmission::Construct => Ok((target, prepended)),
            ConstructAdmission::NotConstructor => Err(VmError::type_error("not a constructor")),
            ConstructAdmission::CallableOnlyNative(name) => Err(VmError::type_error(format!(
                "{} is not a constructor",
                self.strings.get_utf8(name)
            ))),
            ConstructAdmission::IllegalConstructorNative(name) => {
                Err(VmError::illegal_constructor(&self.strings.get_utf8(name)))
            }
        }
    }
}
