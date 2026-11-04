//! Construct admission: the `IsConstructor` abstract operation
//! (ECMA-262 §7.2.4).
//!
//! The follow-up slot `#11-vm-construct-isconstructor-check` adds the
//! constructor classifier and the shared admission helper here.

#[cfg(feature = "engine")]
use super::object_kind::ObjectKind;

/// `IsConstructor(value)` (ECMA-262 §7.2.4): true when the object has
/// a `[[Construct]]` internal slot.  Walks `BoundFunction` chains
/// up to [`crate::vm::MAX_BIND_CHAIN_DEPTH`] and inspects the
/// underlying target — a bound chain ending in an arrow function,
/// async function, generator function, or non-constructable native
/// must NOT report constructor (`do_new` in `ops.rs` does the
/// same unwrap before validating, so without this recursive check
/// the `IsConstructor` gate at `%TypedArray%.of` / `.from` could
/// be bypassed by
/// `Object.setPrototypeOf((()=>{}).bind(null), Uint8Array)` /
/// `Object.setPrototypeOf(async function(){}, Uint8Array)`).
///
/// For JS `Function` objects, this VM treats a function as
/// constructable iff it is not an arrow function
/// (`ThisMode::Lexical`) AND its compiled metadata is neither
/// `is_async` nor `is_generator`.  In other words, any non-arrow,
/// non-async, non-generator JS function is considered to have
/// `[[Construct]]` here — that includes classic `function`
/// declarations and `class` ctors alike (the latter is a
/// strict-mode `function` in our compiled representation).
///
/// Free function rather than a method on [`ObjectKind`] because
/// the chain walk needs `VmInner` access to look up each
/// `BoundFunction.target` by `ObjectId` and to fetch the compiled
/// function metadata for the async/generator check.
#[cfg(feature = "engine")]
pub(crate) fn is_constructor(vm: &super::VmInner, id: super::value::ObjectId) -> bool {
    let mut current = id;
    // `0..=MAX_BIND_CHAIN_DEPTH` (one more iteration than the
    // half-open range) so a chain of exactly `MAX_BIND_CHAIN_DEPTH`
    // `BoundFunction` wrappers can fully unwrap to its target —
    // matches `do_new`'s policy in `ops.rs` (allows MAX wrappers,
    // errors only on MAX+1).  An off-by-one half-open range
    // would exit before inspecting the final target and incorrectly
    // report non-constructor on otherwise-valid bound chains.
    for _ in 0..=crate::vm::MAX_BIND_CHAIN_DEPTH {
        match &vm.get_object(current).kind {
            ObjectKind::Function(fo) => {
                if fo.this_mode == crate::vm::value::ThisMode::Lexical {
                    return false;
                }
                let compiled = vm.get_compiled(fo.func_id);
                return !compiled.is_async && !compiled.is_generator;
            }
            ObjectKind::NativeFunction(nf) => return nf.shape.can_construct(),
            ObjectKind::BoundFunction { target, .. } => current = *target,
            _ => return false,
        }
    }
    // Chain length exceeded — reject defensively (matches
    // `do_new`'s `Maximum bind chain depth exceeded` RangeError
    // intent at the `IsConstructor`-precheck level).
    false
}
