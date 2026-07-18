# VM Symbol-operand coercion — re-diagnosis of `#11-vm-symbol-operand-coercion-throws`

**Status**: plan-memo, ships with the impl PR. Branch `vm-symbol-operand-coercion`
(worktree `elidex-wt-symbol-coerce`), base `origin/main` `3e797897` (#472; includes
#474 `3892f2b2`).
**Lane**: core VM (post-#474 cluster). **This PR carries no code fix** — see §0.

---

## 0. TL;DR

The #467 Date converge carved `#11-vm-symbol-operand-coercion-throws` with the
premise *"`coerce.rs` `to_number`/`to_string` throw for a primitive Symbol, but
the **opcode path bypasses them** so `` `${Symbol()}` `` / `Number(Symbol())` /
`Symbol()*1` / `[Symbol()].join()` return a string instead of TypeError."*
**That premise is wrong** — same shape as the `#11-vm-native-fn-generic-invocation`
re-diagnosis that #474 corrected.

A live-VM probe of 36 spellings (§1, `Vm::eval`) shows **every ES coercion of a
Symbol — primitive AND boxed — already throws the spec-mandated `TypeError`**, and
every *non-coercion* Symbol use (`String(Symbol())`, `typeof`, `==`, valid property
key, `JSON.stringify`) is spec-correct. The opcode/builtin coercion dispatch was
never bypassing `coerce::to_number`/`to_string`; it routes through them by
construction (single chokepoint, §4).

So this PR **adds the JS-observable regression tests the slot itself deferred**
("pinned at the coerce layer since the JS spelling is blocked by
`#11-vm-native-fn-generic-invocation`" — now unblocked by #474) and **closes the
slot**. No behavior changes.

---

## 1. Re-diagnosis: evidence (live-VM probe, `Vm::eval`, engine feature)

The probe evaluated each expression top-level (no `try`/`catch`) and recorded the
outcome. All results are spec-correct:

**Coercion → must throw `TypeError` (§7.1.4 ToNumber / §7.1.18 ToString / §13.15.3
ApplyStringOrNumericBinaryOperator) — ALL THROW:**

| spelling | result | spelling | result |
|---|---|---|---|
| `` `${Symbol()}` `` | throw (string) | `+Symbol()` `-Symbol()` `~Symbol()` | throw (number) |
| `` `x${Symbol('y')}z` `` | throw | `Symbol()\|0` `Symbol()&1` `Symbol()>>1` | throw |
| `Number(Symbol())` | throw (number) | `Symbol()**2` `2**Symbol()` `Symbol()%2` | throw |
| `Symbol()*1` `Symbol()-1` | throw | `Math.abs(Symbol())` | throw |
| `Symbol()+1` | throw (number) | `parseInt(Symbol())` `'x'.repeat(Symbol())` | throw |
| `Symbol()+''` `''+Symbol()` | throw (string) | `Symbol()<1` | throw (number) |
| `[Symbol()].join()` `.toString()` | throw | `[Symbol()]+''` `` `${[Symbol()]}` `` | throw |
| `new String(Symbol())` | throw | — | — |

**Boxed Symbol — newly JS-reachable because #474 made `Object()` callable — ALL correct:**

| spelling | result | note |
|---|---|---|
| `typeof Object(Symbol())` | `"object"` | SymbolWrapper |
| `` `${Object(Symbol())}` `` `Object(Symbol())+1` | throw | §7.1.1.1 OrdinaryToPrimitive → §20.4.3.4/.5 → Symbol → throw (the #467 R9 fix, now JS-observable) |
| `Number(Object(Symbol()))` `Object(Symbol())*2` | throw | — |
| `String(Object(Symbol()))` | **throw** | value is an Object, not a primitive Symbol → §22.1.1.1 step 2.a inapplicable → step 2.b ToString → throw (contrast `String(Symbol())` below) |

**Non-coercion → must NOT throw — ALL correct:**

| spelling | result | spec |
|---|---|---|
| `String(Symbol())` / `String(Symbol('foo'))` | `"Symbol()"` / `"Symbol(foo)"` | §22.1.1.1 step 2.a SymbolDescriptiveString |
| `Symbol().toString()` | `"Symbol()"` | §20.4.3.3 explicit method |
| `Symbol().description` / `typeof Symbol()` | `undefined` / `"symbol"` | — |
| `Boolean(Symbol())` / `Symbol()?1:0` | `true` / `1` | §7.1.2 ToBoolean never throws |
| `Symbol()==1` / `Symbol()===1` | `false` | §7.2.13 IsLooselyEqual / §7.2.14 IsStrictlyEqual (no Symbol coercion clause) |
| `JSON.stringify(Symbol())` | `undefined` | §25.5.4.2 SerializeJSONProperty |
| `({})[Symbol()]` | `undefined` | Symbol is a valid property key (no coercion) |

## 2. Why the slot premise was wrong

The #467 converge R9 fixed a **real** bug: after R7 removed a `to_primitive`
wrapper fast-path, a **boxed** Symbol (`SymbolWrapper` from `ToObject`) reaching
`OrdinaryToPrimitive` fell through to `Object.prototype.toString` → `"Symbol(x)"`
(silent stringify) because `Symbol.prototype.valueOf`(§20.4.3.4) /
`@@toPrimitive`(§20.4.3.5) were not installed. #467 installed them
(`natives_symbol.rs`; regression `tests_coerce.rs::boxed_symbol_to_primitive_yields_the_symbol`).

The slot then generalized that boxed-symbol hole to *"opcode-level coercion
dispatch spans all operators × Symbol"* and assumed the **primitive**-Symbol
opcode path was also broken — but pinned it at the coerce-unit layer because the
JS spellings could not be observed (`Object.prototype.toString.call(x)` and generic
native-fn invocation were broken — the `#11-vm-native-fn-generic-invocation`
defect #474 re-diagnosed & closed). A **primitive** Symbol never took the
`OrdinaryToPrimitive` detour: it hits the `JsValue::Symbol(_) => Err(type_error…)`
arm in `coerce::to_number`/`to_string` directly. The generalization was never
verified against a running VM; §1 now does, and falsifies it.

Both #474's slot and this one share the failure mode: **a coerce-layer assumption
frozen into a defer slot while JS-observability was blocked, disproved once #474
unblocked it.**

## 3. Deliverable

`crates/script/elidex-js/src/vm/tests/tests_symbol_coercion.rs` (new) — JS-observable
regression tests locking the §1 behavior end-to-end (parser → compiler → opcode
dispatch → coerce), which the existing **unit**-level `to_number(Symbol)` /
`to_string(Symbol)` tests do not exercise:

1. Every coercion spelling in §1 throws a `TypeError` (`matches!(err.kind,
   VmErrorKind::TypeError)` — the spec-mandated error *type*; the impl-defined
   message is deliberately not asserted).
2. The boxed-Symbol coercions throw (locks the #467 R9 fix at the JS level for the
   first time — reachable only after #474's callable `Object()`), including the
   `String(Object(Symbol()))` throw-vs-`String(Symbol())`-descriptive distinction.
3. The non-coercion boundary cases return their spec values (guards against an
   over-eager "throw on any Symbol" regression).

## 4. Why no code fix (correct by construction)

The correctness is structural, not incidental: `coerce::to_number` (§7.1.4, Symbol
arm) and `coerce::to_string` (§7.1.18, Symbol arm) are the **single** coercion
chokepoint; every arithmetic/bitwise/relational opcode (`dispatch.rs` →
`ops.rs`/`coerce_ops.rs`) and every builtin (`NativeContext::to_number` /
`to_string_val`) routes through them, and their Object arm delegates to
`to_primitive`. `to_display_string` (`coerce.rs:254`, never-throws, formats
`"Symbol(x)"`) is a **separate** display-only path with exactly two call sites —
`format_value_for_console` and the unhandled-promise-rejection `eprintln!` — neither
an ES coercion. There is no bypass to fix and nothing to unify (One-issue-one-way is
already satisfied). The one legitimate non-throwing coercion, `String()` call-form,
is already special-cased at `natives_string.rs:749` (§22.1.1.1 step 2.a →
`symbol_to_descriptive_string`).

## 5. Deferred-slot reconciliation (at landing, PM)

- **CLOSE** `#11-vm-symbol-operand-coercion-throws` — re-diagnosed; the "opcode
  bypass" premise is retired (probe-falsified, §1). Real R9 bug (boxed symbol) was
  fixed in #467; this PR adds the JS-observable regression coverage the slot
  deferred and locks both the primitive and boxed paths.
- Record the re-diagnosis so the misleading premise does not resurface (mirrors the
  #474 `#11-vm-native-fn-generic-invocation` closure note).
- No new candidate slots. `#11-vm-wrapper-coercion-override-bypass` (PR 3, the
  wrapper-arm `valueOf`/`toString` override bypass at `coerce.rs:229-238`) remains a
  **real** open bug — orthogonal to this (a different `coerce.rs` seam) and still
  requires `/elidex-plan-review` (edge-dense).

## 6. Layering / ECS / edge-dense self-assessment

- **Layering**: test-only + doc. No `vm/host/` touch, no engine-independent
  algorithm. Compliant.
- **ECS-native**: N/A (pure VM coercion semantics, no per-entity state).
- **Edge-dense?** No — a single named invariant (Symbol coerces → `TypeError`)
  verified across N spellings; no intersecting invariant axes. No plan-review gate
  (contrast PR 3). This memo is written because the re-diagnosis overturns a carved
  slot's premise (cheapest-stage record), not because the work is edge-dense.
- **1000-line touch**: new test file (~1 screen); `tests_coerce.rs` untouched
  (probe reverted). No split trigger.
