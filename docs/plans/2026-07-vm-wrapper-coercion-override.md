# VM wrapper-coercion override bypass — `#11-vm-wrapper-coercion-override-bypass`

**Status**: plan-memo, pre-implementation. Branch `vm-wrapper-coercion-override`
(worktree `elidex-wt-wrapper-coerce`), base `origin/main` `bfc6e068`. Ships with the impl PR.
**Lane**: core VM (last item of the post-#467 cluster; keystone #474 + PR2 #478 landed).
**Plan-review**: MANDATORY (slot-designated edge-dense) — this memo is its input.

---

## 0. TL;DR

`coerce::to_number` / `to_string` and `JSON.stringify` take a **wrapper fast-path**
that reads a primitive wrapper's internal slot (`[[NumberData]]`/`[[StringData]]`/…)
**directly**, bypassing a user-overridden `valueOf` / `toString` / `@@toPrimitive`.
This is a **real, probe-confirmed bug** (§1) — unlike the PR2 slot, which was a false
premise. The bug spans three seams: coerce.rs (8 arms), JSON.stringify (6 arms), and
String.prototype method `this`-coercion (`coerce_this_string`).

The fix is **not** the edge-dense protector-cell machinery the slot's "4 axes +
hot-path perf" framing implies. The winning design is the **simplest**: **remove the
wrapper fast-path and let coercion flow through `ToPrimitive`, which already honors
overrides** (the #467 R7 fix). Two clinching facts:

1. The fast-path only fires on wrapper **objects** (`JsValue::Object`) — primitive
   `+5` / `"a"+b` (the real hot path) never enter it. It optimizes a **cold** path
   (explicit `new Number()` boxing), so removing it costs a method call on rare code,
   not throughput.
2. `ops.rs:266-275` **already documents** (a "#467 R7" note) that
   `ordinary_to_primitive` deliberately has **no** wrapper shortcut *because it would
   bypass a user override*. The coerce.rs fast-path **contradicts a decision already
   written into the same VM** — removing it makes the VM self-consistent
   (*One-issue-one-way*).

**Recommended: Option C (fast-path removal).** Options A (global protector flag) and
B (per-wrapper pristine check) are documented + rejected in §5.

---

## 1. Bug evidence (live-VM probe, `Vm::eval`, engine feature)

Each case runs in a fresh VM; the override is isolated. **Spec = the override is honored.**

### 1a. coerce.rs + JSON (the slot-enumerated seams)

| spelling | got | spec | seam |
|---|---|---|---|
| `Number.prototype.valueOf=()=>42; +new Number(5)` | `5` | `42` | to_number fast-path |
| `…; new Number(5) * 2` | `10` | `84` | to_number |
| `Number.prototype[Symbol.toPrimitive]=()=>99; +new Number(5)` | `5` | `99` | to_number (bypasses even @@toPrimitive) |
| `String.prototype.toString=()=>'x'; \`${new String('a')}\`` | `"a"` | `"x"` | to_string fast-path |
| `…; new String('a') + ''` | `"a"` | `"x"` | to_string |
| `Number.prototype.toString=()=>'z'; \`${new Number(5)}\`` | `"5"` | `"z"` | to_string on NumberWrapper |
| `Number.prototype.valueOf=()=>42; JSON.stringify(new Number(5))` | `"5"` | `"42"` | JSON |
| `String.prototype.toString=()=>'x'; JSON.stringify(new String('a'))` | `"\"a\""` | `"\"x\""` | JSON |
| **control** `new Number(5)==42` (via `ops.rs` to_primitive) | `true` | `true` | ✅ already correct (#467 R7) |
| **control** `+new Number(5)` (no override) | `5` | `5` | ✅ |

### 1b. String.prototype method `this`-coercion (`coerce_this_string`, third seam)

| spelling | got | spec | 
|---|---|---|
| `String.prototype.toString=()=>'xyz'; String.prototype.charAt.call(new String('abc'),0)` | `"a"` | `"x"` |
| `…; String.prototype.slice.call(new String('abc'),0,2)` | `"ab"` | `"xy"` |
| `…; String.prototype.indexOf.call(new String('abc'),'y')` | `-1` | `1` |
| **control** `String.prototype.charAt.call(new String('abc'),0)` (no override) | `"a"` | `"a"` ✅ |

§22.1.3 String.prototype methods do `RequireObjectCoercible(this)` + **`ToString(this)`**
(not `thisStringValue`; the latter is only `String.prototype.toString`/`valueOf`
themselves, `natives_string.rs:713`), so a String-wrapper `this` with overridden
`toString` must use the override. (§22.1.3 is the **current** ECMA-262 number, Axis-4-verified;
the surrounding VM code still cites the **stale** §21.1.3 for String.prototype — a pre-existing
drift, **out of this PR's scope**, candidate for an ES citation sweep. This PR uses the current
number rather than propagating the stale one — Axis-5 MIN.)

---

## 2. Spec basis (webref-verified `ecma262`; re-verify §-number/title at impl)

- **§7.1.4 ToNumber** step 8 (Object case = steps 7-10): `? ToPrimitive(argument, number)` then step 10 `? ToNumber(primitiveValue)`.
- **§7.1.18 ToString** step 10 (Object case = steps 9-12; step 9 asserts Object): `? ToPrimitive(argument, string)` then step 12 `? ToString(primitiveValue)`.
- **§7.1.1 ToPrimitive** — for an Object (step 1), dispatches an exotic `@@toPrimitive` (**step 1.b**) if present, else **step 1.d → §7.1.1.1 OrdinaryToPrimitive** (calls `valueOf`/`toString` per hint order). This is the sole spec path; there is no "read the wrapper's internal slot" shortcut.
- **§25.5.4.2 SerializeJSONProperty** step 4 (webref-verified prose): 4.b `[[NumberData]]` → **`? ToNumber(value)`**; 4.c `[[StringData]]` → **`? ToString(value)`**; 4.d `[[BooleanData]]` → `value.[[BooleanData]]` **(direct — no override)**; 4.e `[[BigIntData]]` → `value.[[BigIntData]]` **(direct)**. So JSON's Number/String wrapper unwrap must honor overrides; Boolean/BigInt must **not**.
- **§25.5.4 JSON.stringify** — the replacer-array PropertyList element coercion (**step 5.b.ii.4.f.i**: `[[StringData]]` **or** `[[NumberData]]` → `? ToString(propertyValue)` — **BOTH → ToString**, not ToNumber) and the `space` argument coercion (**step 6.a** `[[NumberData]]`→`? ToNumber` / **step 6.b** `[[StringData]]`→`? ToString`) are override-honoring (webref-verified, Axis-4 confirmed).

**Asymmetry that any fix must preserve — it is *per-site*, not one rule** (Axis-2 MIN / Axis-5 IMP):
- **coerce.rs `to_number`/`to_string`**: **all four** wrapper kinds (incl. Boolean/BigInt) route through `ToPrimitive` and **honor the override** — §7.1.4/§7.1.18 draw no per-kind distinction (BigInt-wrapper `to_number` still throws §7.1.4, but only *after* an overridden `valueOf` runs; e.g. `BigInt.prototype.valueOf=()=>42; +Object(1n)` → 42). Removing all four arms is correct; **the fix is symmetric on the coerce side** (§6 must test Boolean/BigInt coerce-override, not only Number/String).
- **JSON has three distinct Boolean/BigInt dispositions**: SerializeJSONProperty 4.d/4.e read `[[BooleanData]]`/`[[BigIntData]]` **direct** (KEEP); the replacer PropertyList (step 5.b.ii.4) **skips** non-String/Number wrappers; the `space` arg (step 6) leaves a non-Number/String Object as-is (no indent). Only JSON Number/String unwrap coerces.
A naive "route every wrapper through ToPrimitive everywhere" over-corrects the JSON Boolean/BigInt sites; the fix is **per-arm per the §3 map**, not one blanket rule.

---

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §7.1.4 ToNumber | step 8 | value is Object | `coerce::to_number` Object arm — remove wrapper fast-path → fallthrough `to_primitive(val,"number")` (`coerce.rs:61-62`) | ✓ | yes (`+wrapper`, `wrapper*n`, unary/bitwise/relational) |
| ECMA-262 §7.1.18 ToString | step 10 | value is Object | `coerce::to_string` Object arm — remove fast-path → fallthrough `to_primitive(val,"string")` (`coerce.rs:245-246`) | ✓ | yes (template, `+''`, wrapper→string) |
| ECMA-262 §7.1.1 ToPrimitive | step 1.b | exotic `@@toPrimitive` present | `ops.rs::to_primitive` (UNCHANGED — the defer target) | ✓ | yes |
| ECMA-262 §7.1.1.1 OrdinaryToPrimitive | steps 1-3 | `@@toPrimitive` absent → `valueOf`/`toString` by hint | `ops.rs::ordinary_to_primitive` (UNCHANGED) | ✓ | yes |
| ECMA-262 §25.5.4.2 SerializeJSONProperty | step 4.b | `[[NumberData]]` | `stringify.rs:91` → `ToNumber(value)` | ✓ | yes (`JSON.stringify(new Number(…))`) |
| ECMA-262 §25.5.4.2 | step 4.c | `[[StringData]]` | `stringify.rs:92` → `ToString(value)` | ✓ | yes |
| ECMA-262 §25.5.4.2 | step 4.d | `[[BooleanData]]` | `stringify.rs:93` — **KEEP** direct `value.[[BooleanData]]` | ✓ | no (no override path) |
| ECMA-262 §25.5.4.2 | step 4.e | `[[BigIntData]]` | `stringify.rs:94` — **KEEP** direct `value.[[BigIntData]]` | ✓ | no |
| ECMA-262 §25.5.4 JSON.stringify | step 5.b.ii.4.f.i | replacer-array element has `[[StringData]]` **or** `[[NumberData]]` | `stringify.rs:452-453` → `ToString(propertyValue)` (BOTH → ToString, not ToNumber) | ✓ | yes (replacer array element) |
| ECMA-262 §25.5.4 JSON.stringify | step 6.a | `space` has `[[NumberData]]` | `stringify.rs:512` → `ToNumber(space)` | ✓ | yes (space arg) |
| ECMA-262 §25.5.4 JSON.stringify | step 6.b | `space` has `[[StringData]]` | `stringify.rs:513` → `ToString(space)` | ✓ | yes (space arg) |
| ECMA-262 §22.1.3 String.prototype methods | RequireObjectCoercible + `ToString(this)` | String-wrapper `this` | `natives_string.rs:113` `coerce_this_string` → route through `ToString(this)` | ✓ | yes (charAt/slice/indexOf/… on a wrapper receiver) |

**Breadth**: K=3 spec areas (core coercion §7 / JSON §25.5.4 / String.prototype §22.1.3),
M=12 rows. All rows are the *same* transformation — *wrapper→primitive via the spec AO,
never a slot shortcut* — applied per-arm. The sole asymmetry is JSON step-4
Boolean/BigInt (direct, KEEP) vs Number/String (coerce). Not an intersecting-invariant
matrix → single-PR scope under Option C (§5).

**Non-touched adjacent**: the (b) legitimate slot-reads (§4) — `thisXValue`, builtinTag
§20.1.3.6, String-exotic index §10.4.3, StructuredSerialize, GC — are NOT coercions and
stay direct; a regression guard pins them (§6).

### §3.1 User-input touch audit

- `to_number`/`to_string` `val` (Object wrapper) — user-controllable (`+new Number(x)`,
  `` `${new String(x)}` ``). Post-removal flows to `to_primitive` → `ordinary_to_primitive`
  → user `valueOf`/`toString`/`@@toPrimitive` (honored), then the recursive
  `to_number`/`to_string` sees the returned primitive (or throws if the override yields a
  Symbol / non-coercible — spec-correct, identical to any `ToPrimitive`). Full enum ✓.
- `JSON.stringify` `value` / `replacer`-array element / `space` — all user-controllable;
  each routes to the spec AO (ToNumber/ToString) honoring the override. Full enum ✓.
- `coerce_this_string` `this` — user-controllable (`String.prototype.m.call(wrapper)`);
  routes to `ToString(this)`. Full enum ✓.
- **Adversarial**: an override that throws or returns a non-primitive propagates as a
  normal `ToPrimitive` throw (spec-correct) — **no new panic surface** (the removed arms
  never panicked; `to_primitive` is already the live path for every non-wrapper Object).
  Full enum ✓.

## 4. Site map (from a read-only investigation; classify a=bug / b=legitimate)

### (a) BUG — direct slot read bypassing a spec-mandated AO → **FIX**

| file:line | site | fix |
|---|---|---|
| `coerce.rs:47-57` | `to_number` Object arms (Number/Boolean/String/BigInt) | delete arms → fallthrough `to_primitive(val,"number")` (already at `:61-62`) |
| `coerce.rs:230-237` | `to_string` Object arms (all 4) | delete arms → fallthrough `to_primitive(val,"string")` (already at `:245-246`) |
| `natives_json/stringify.rs:91` | SerializeJSONProperty NumberWrapper | → `ToNumber` (honor override) |
| `natives_json/stringify.rs:92` | SerializeJSONProperty StringWrapper | → `ToString` |
| `natives_json/stringify.rs:452` | replacer PropertyList NumberWrapper | → `ToString(propertyValue)` |
| `natives_json/stringify.rs:453` | replacer PropertyList StringWrapper | → `ToString` |
| `natives_json/stringify.rs:512` | `space` arg NumberWrapper | → `ToNumber` **(fallible ⚠)** |
| `natives_json/stringify.rs:513` | `space` arg StringWrapper | → `ToString` **(fallible ⚠)** |
| `natives_string.rs:113` | `coerce_this_string` (String.proto method `this`) | route through `ToString(this)` (§1b confirmed bug; One-issue-one-way) |

**KEEP (correct by spec, do NOT touch)**: `stringify.rs:93` (Boolean, §25.5.4.2 4.d
direct) and `:94` (BigInt, 4.e direct).

**⚠ `space` is NOT a pure in-place arm swap — `compute_gap` fallibility (Axis-2 IMP)**: the
`space` arms (`:512-513`) live inside `fn compute_gap(...) -> String` (`stringify.rs:508`),
called **without `?`** at `:467`. Routing through the *fallible*, JS-re-entrant
`ToNumber`/`ToString` (§25.5.4 step 6 uses `?`) requires `compute_gap` →
`Result<String, VmError>` + `?` at the call site + a borrow-release for the `&mut ctx`
coercion — unlike the infallible `coerce.rs` deletions. A naive `unwrap_or` swallow would
silently drop a throwing `space.valueOf`'s abrupt completion, which a happy-path-only test
would miss (§6 item 3 pins the throw). The step-4 (`:88-97`) and replacer (`:450-453`) sites
already sit in `Result` contexts, so this is `space`-specific.

### (b) Legitimate — direct slot read is spec-correct → **MUST STAY**

`natives_symbol.rs:285-287` (Object.prototype.toString **builtinTag** §20.1.3.6 —
branches on slot *presence*, not a coercion) / `natives_number.rs:11`
`this_number_value`, `natives_boolean.rs:11`, `natives_bigint.rs:101/296`,
`natives_string.rs:713` `native_string_value_of` (**thisXValue** — read slot by
definition) / `ops_element.rs:213/398`, `coerce_format.rs:57`,
`natives_object/prototype.rs:152` (String-exotic **index/length** own props, §10.4.3) /
`inner.rs:190-257`, `coerce.rs:735` (wrapper **construction**) /
`host/structured_clone.rs:149-247` (StructuredSerialize reads slots per spec) /
`gc/trace.rs:559-564` (GC). **A regression guard test must pin the (b) sites** (esp.
`Object.prototype.toString.call(new Number(5))` builtinTag, `thisXValue`, and String
index access) so the fix does not over-reach.

### The unifying principle (One-issue-one-way)

All (a) sites share one root: **a coercion shortcut that reads a wrapper's slot
instead of invoking the spec AO**. The fix converges them onto the single canonical
rule — *wrapper→primitive coercion goes through `ToPrimitive`/`ToNumber`/`ToString`,
never a slot shortcut* — the exact rule `ops.rs:266-275` already applies at the
`ordinary_to_primitive` layer. No new mechanism; a **removal** that ends a
strangler (fast-path shortcut coexisting with the correct to_primitive path).

---

## 5. Design options (materials + decision)

**A. Global protector / dirty-flag** (V8-style; fast-path stays until an intrinsic is poisoned).
- *Cost*: highest. New `VmInner` state + hooks at **~5 mutation sinks** (`set_property_val`,
  `Object.defineProperty`, `delete`, `setPrototypeOf`, Reflect) — **miss one → silent
  wrong result**. First global-invalidation mechanism in a VM whose inline caches
  deliberately use per-access shape guards, **no global version counter** to mirror.
- *Perf*: preserves a single-slot-read — but only on the **cold** wrapper-object path.
- *Philosophy*: adds cross-cutting mutable invalidation state (tension with *design by
  structure*); a "flag + slow path" duality is itself a strangler. **Rejected.**

**B. Per-wrapper pristine check** (before fast-path, verify proto is intrinsic +
valueOf/toString/@@toPrimitive un-overridden).
- *Cost*: medium — cached intrinsic-method ObjectIds + a 3-key identity check + proto-pointer check.
- *Perf*: the check ≈ the 2-3 `get_property` chain walks `to_primitive` **already does**,
  so it mostly **erases** the fast-path's advantage. Net: complexity for ≈no perf gain.
- *Philosophy*: no global state, but a bespoke "is it still default?" gadget where a
  plain removal suffices. **Rejected** (worse-than-C on every axis).

**C. Fast-path removal (RECOMMENDED)** — delete the (a) coerce.rs arms, fix the 6 JSON
arms per §2 asymmetry, route `coerce_this_string` through ToString; keep (b) intact.
- *Cost*: lowest — deletions + reliance on **existing** `to_primitive` fallthrough
  (`coerce.rs:61-62`/`:245-246`); `to_primitive`/`ordinary_to_primitive` unchanged.
- *Perf*: wrapper-object coercion pays 2 chain walks + 1 native call instead of 1 slot
  read — **only for boxed wrappers, never for primitives** (the hot path is untouched).
- *Correctness*: directly spec-conformant; converges on the path `ops.rs:266-275`
  already argues for; no completeness obligation, no new state.
- *Philosophy*: strongest *Ideal over pragmatic* + *One-issue-one-way* fit — removes a
  documented-as-wrong shortcut at a cold-path cost.

**Perf due-diligence for plan-review**: the claim "wrapper coercion is cold" rests on
the fast-path living inside the `JsValue::Object` arm (primitives hit `JsValue::Number`/
`String` arms directly). If a plausible hot workload boxes primitives in a tight
coercion loop, that is the one scenario to weigh — but auto-boxing (`(5).toFixed()`) is
a *method call on* a transient wrapper, not a *coercion of* a persisted wrapper, so it
does not hit these arms. The reasoning generalizes to the *other* removed-arm entry points
plan-review named — `JSON.stringify` of a boxed-wrapper **value** (`stringify.rs` arms) and
WebIDL/native coercion of a boxed-wrapper **argument** (`api(new String(x))`): each still
requires an explicitly-boxed wrapper to reach the arm, which is rare across **all** coercion
entry points, not only `coerce.rs`. General thesis: **boxed primitive wrappers are uncommon
operands everywhere.** No micro-bench is planned unless plan-review identifies a real hot
wrapper-coercion path.

---

## 6. Test plan (`tests_*` under `vm/tests`, engine feature)

New `tests_wrapper_coercion.rs` (JS-observable):
1. **Override honored — coerce side, ALL FOUR wrapper kinds** (Axis-5 IMP: §4(a) removes all
   4 arms, so Boolean/BigInt must be tested too, not only Number/String): for `to_number` AND
   `to_string`, an overridden `valueOf`/`toString`/`@@toPrimitive` is honored —
   `Number.prototype.valueOf=()=>42; +new Number(5)`→`42`;
   `String.prototype.toString=()=>'x'; \`${new String('a')}\``→`'x'`;
   **Boolean** `Boolean.prototype.valueOf=()=>42; +new Boolean(true)`→`42` (was `1`);
   **BigInt** `BigInt.prototype.valueOf=()=>42; +Object(1n)`→`42` (was a throw); +/*/-/unary/
   bitwise/relational, template, concat spellings.
2. **Override honored — JSON, with concrete expected values** (Axis-5 IMP): value
   `Number.prototype.valueOf=()=>42; JSON.stringify(new Number(5))`→`'42'`;
   `String.prototype.toString=()=>'x'; JSON.stringify(new String('a'))`→`'"x"'`;
   **replacer array** element `JSON.stringify(obj,[new Number(0)])` with overridden
   `Number.prototype.toString` → the property key derived from the wrapper honors the override;
   **space** `JSON.stringify({a:1},null,new Number(2))` with overridden `valueOf` → indent
   width taken from the override.
3. **Throwing override propagates (abrupt completion)** — `Number.prototype.valueOf` that
   throws makes `+new Number(5)` throw; **and the `space` case** — a boxed `space` whose
   `valueOf`/`toString` throws must PROPAGATE through `JSON.stringify` (guards the
   `compute_gap` fallibility ⚠ §4; a naive `unwrap_or` swallow passes items 1-2 but fails here).
4. **Boolean/BigInt JSON stays direct (asymmetry preserved)** — `Boolean.prototype.valueOf`/
   `toString` override does NOT change `JSON.stringify(new Boolean(true))` (`true`); BigInt
   likewise (§25.5.4.2 4.d/4.e); replacer/space Boolean/BigInt dispositions (skip / as-Object)
   unchanged.
5. **(b) non-regression guards** — `Object.prototype.toString.call(new Number(5))` ===
   `"[object Number]"` (builtinTag unaffected); `new Number(5).valueOf()` === `5`
   (thisNumberValue); `new String('abc')[1]` === `"b"` and `.length` === `3` (String
   exotic index); no override → all coercions unchanged (control rows).
6. **Regression sweep**: `cargo test -p elidex-js --all-features` (coercion is
   foundational; run the whole crate before push).

---

## 7. Layering / ECS / edge-dense self-assessment

- **Layering**: `coerce.rs` / `natives_json` / `natives_string` = VM-core ES semantics,
  engine-bound. No `EcsDom` touch, no engine-independent algorithm elsewhere. Compliant.
- **ECS-native**: N/A (pure VM coercion; no per-entity state). Notably Option A would
  have *added* cross-cutting VM state — Option C keeps the VM stateless here.
- **Edge-dense?** The slot designated it edge-dense (4 axes + perf). The investigation
  **re-scopes** it: with Option C the "4 axes" collapse to **one principle applied at N
  sites** (route through the AO), not N intersecting invariants, and the "hot-path perf"
  axis dissolves (cold path). The residual real subtlety is the **JSON Number/String vs
  Boolean/BigInt asymmetry** (§2) — a per-arm spec detail, not an intersecting-invariant
  matrix. Plan-review is still run (slot-mandated + `coerce`/`JSON`/`String-method` is a
  wide touch with a (b)-site over-reach risk), but the fix is a **simplification**, not
  an edifice — the same "slot over-estimated the complexity" shape as #474 / PR2.
- **1000-line touch**: `coerce.rs` is ~1017 lines. Option C **removes** ~18 lines
  (net negative); no new cohesion seam introduced. Not a split trigger.

---

## 8. Scope boundaries

- **IN**: the 9 (a) sites (coerce.rs 8 arms + `coerce_this_string`) + 6 JSON arms;
  keep JSON Boolean/BigInt (`:93/:94`) direct; preserve all (b) sites.
- **OUT**: no new slots anticipated. If plan-review judges `coerce_this_string`
  (String-method seam) a distinct-enough blast-radius to carve, it splits to a
  follow-up `#11-string-method-this-coercion-override` — but the default is *include*,
  and the reason is stronger than "same fix shape" (Axis-3 MIN): `coerce_this_string`'s
  non-wrapper branch **already routes through `ctx.to_string_val` → `coerce::to_string`**
  (`natives_string.rs:121-128`), so the String-method fix is **dependent on** the
  `to_string` fix — carving would leave String-method `this`-coercion bypassing the
  override until the follow-up lands, and even that follow-up only works *after* `coerce.rs`
  ships (a probe-confirmed half-fix strangler). Include is the coherent unit.

## 9. Deferred-slot reconciliation (at landing, PM)

- **CLOSE** `#11-vm-wrapper-coercion-override-bypass` — fixed (Option C). This is the
  **last item of the core VM lane cluster** (#474 keystone + #478 PR2 + this).
- No new slots (unless the §8 `coerce_this_string` carve fires at plan-review).
- Record that the slot's "edge-dense 4-axis + hot-path" framing was re-scoped by the
  investigation to a One-issue-one-way removal (cold-path perf; VM self-consistency with
  `ops.rs:266-275`).
