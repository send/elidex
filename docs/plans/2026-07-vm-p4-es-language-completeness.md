# VM P4 — ES language completeness: discharging the stubbed-emit inventory

**Status**: umbrella plan (multi-PR program). Per-slice `/elidex-plan-review` mandatory.
**Baseline**: `f7d9b5ce` (= `origin/main` at authoring time, commit date 2026-07-26).
**Slots**: `project_open-defer-slots.md` §"VM P4 ES-language + builtin gaps"; **plus** the two
pre-existing slots this umbrella adopts (§8).
**Corrects**: `project_vm-p4-es-language-gaps.md` §2/§4 (see §1.1).
**Revision**: R2 — restructured after `/elidex-plan-review` round 1 (2 CRIT / 11 IMP), which
established that the defect class sits in the **compiler emit layer**, not the dispatch layer.

---

## §0.5 Spec citation table

All verified via `.claude/tools/webref` against `ecma262` on 2026-07-26.

| ID | Citation | Anchor | Used by |
|---|---|---|---|
| [C19] | ECMA-262 §13.3.8.1 Runtime Semantics: ArgumentListEvaluation | `#sec-runtime-semantics-argumentlistevaluation` | Slice 1b, 4 |
| [C20] | ECMA-262 §13.3.6.2 EvaluateCall | `#sec-evaluatecall` | Slice 1b, 4 |
| [C33] | ECMA-262 §13.3.5.1.1 EvaluateNew | `#sec-evaluatenew` | Slice 1b |
| [C34] | ECMA-262 §7.3.14 Construct | `#sec-construct` | Slice 1b |
| [C35] | ECMA-262 §7.4.10 IteratorStepValue | `#sec-iteratorstepvalue` | Slice 1a |
| [C36] | ECMA-262 §7.4.11 IteratorClose | `#sec-iteratorclose` | Slice 0b ([C39] conformance) / Slice 1a (dec. 13a) / `#11-vm-iteratorclose-precedence-convention` (dec. 13b) — **not** Slice 1b (§2.5 C×F) |
| [C37] | ECMA-262 §13.3.9.1 Runtime Semantics: Evaluation (optional chain) | `#sec-optional-chaining-evaluation` | Slice 1b |
| [C38] | ECMA-262 §13.3.9.2 Runtime Semantics: ChainEvaluation | `#sec-optional-chaining-chain-evaluation` | Slice 1b |
| [C13] | ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation (`super`) | `#sec-super-keyword-runtime-semantics-evaluation` | Slice 1a handler + 1b emit, 3 |
| [C21] | ECMA-262 §7.4.4 GetIterator ( obj, kind ) | `#sec-getiterator` | Slice 1a, 6 |
| [C22] | ECMA-262 §13.2.4.1 Runtime Semantics: ArrayAccumulation | `#sec-runtime-semantics-arrayaccumulation` | Slice 1a (`SpreadElement` drain) + contrast for dec. 3 |
| [C39] | ECMA-262 §13.15.5.2 Runtime Semantics: DestructuringAssignmentEvaluation | `#sec-runtime-semantics-destructuringassignmentevaluation` | **Slice 0b** |
| [C23] | ECMA-262 §13.2.8.4 GetTemplateObject | `#sec-gettemplateobject` | Slice 4 |
| [C40] | ECMA-262 §13.3.11.1 Runtime Semantics: Evaluation (tagged template) | `#sec-tagged-templates-runtime-semantics-evaluation` | Slice 4 |
| [C24] | ECMA-262 §13.3.7.3 MakeSuperPropertyReference | `#sec-makesuperpropertyreference` | Slice 3 |
| [C25] | ECMA-262 §9.1.1.3.5 GetSuperBase | `#sec-getsuperbase` | Slice 3 |
| [C26] | ECMA-262 §7.3.32 DefineField | `#sec-definefield` | Slice 2 |
| [C27] | ECMA-262 §7.3.33 InitializeInstanceElements | `#sec-initializeinstanceelements` | Slice 2 |
| [C28] | ECMA-262 §15.7.10 ClassFieldDefinitionEvaluation | `#sec-runtime-semantics-classfielddefinitionevaluation` | Slice 2 |
| [C29] | ECMA-262 §7.3.26 PrivateElementFind | `#sec-privateelementfind` | Slice 5 |
| [C30] | ECMA-262 §7.3.28 PrivateMethodOrAccessorAdd | `#sec-privatemethodoraccessoradd` | Slice 5 |
| [C31] | ECMA-262 §7.3.30 PrivateGet | `#sec-privateget` | Slice 5 |
| [C41] | ECMA-262 §7.3.31 PrivateSet | `#sec-privateset` | Slice 5 |
| [C42] | ECMA-262 §7.3.27 PrivateFieldAdd | `#sec-privatefieldadd` | Slice 5 |
| [C32] | ECMA-262 §27.9.3.2 AsyncGeneratorStart | `#sec-asyncgeneratorstart` | Slice 6 |

Existing in-code citations `[C13] §13.3.7.1`, `[C19] §13.3.8.1`, `[C11] §10.2.2`
(`crates/script/elidex-js/src/vm/dispatch_class.rs:6`) re-verified correct — no drift.

---

## §1. Origin and evidence

P4 is the live remaining VM work: [[phase4-plan]]'s **"VM Builtins P4 — 大型 builtins: boa→VM
切替後"** gate opened at the boa deletion (#458 `315ba316`) and was only converted into slots on
2026-07-18 (post-#480).

All behavioural findings are **live-VM probe evidence** — a temporary harness under `vm/tests/`
driving `Vm::eval` at `f7d9b5ce`, run in four rounds, removed afterwards. All structural claims
are grep/read-verified against the same tree.

### §1.1 Corrections to the prior memo

The re-probe **contradicted** `project_vm-p4-es-language-gaps.md` §2's scope-precision note and
found five gaps it never listed.

| Prior claim | Re-probe result |
|---|---|
| "`new X(...args)` … **works**" | **FALSE — broken.** `new C(...[1,2])` → `a=[1,2]`, `b=undefined`; `new C(...[1,2,3]).n` → `arguments.length === 1` |
| (not listed) | **`super.x` / `super.m()` / `super[k]` / `super.x=` all throw** `TypeError: Cannot convert undefined or null to object` — total loss of super-property access |
| (not listed) | **public class fields `class A{x=1}` → `undefined`** (silent). `static x=1` *does* work |
| (not listed) | **`obj[k] += v` PANICS the process** (`assert!`, `compiler/expr_assign.rs:170`) |
| (not listed) | **destructuring *assignment* is a silent no-op** — see §1.2 |
| (not listed) | **async generators non-functional** — `ag().next` is `undefined`; `for await…of ag()` raises an unhandled `TypeError: value is not iterable` |
| `super(...)` grouped with broken | **`super(...args)` WORKS** — the one correct spread path, and 1b's reference implementation |
| "flag accessors missing" | Precise: `.flags` / `.lastIndex` / `.exec` **work**; `.global`/`.ignoreCase`/`.multiline`/`.sticky` and `@@match`/`@@replace` absent |
| `new.target` "probe mis-designed" | Confirmed **working** |

### §1.2 ⚠ Retraction — this plan's own round-1 over-claim

Plan revision R1 asserted "Confirmed correct, no action: **all destructuring forms**". That was
generalised from a probe of destructuring **declarations** only. Round-1 review (Axis 3 CRIT)
caught it; re-probe confirms destructuring **assignment** is a **silent no-op**:

| Spelling | Actual | Spec |
|---|---|---|
| `var x=1; (x)++` | **unchanged `1`** (found by the §2.2 sweep, round 3) | `2` |
| `var x=1; (x)+=1` | **unchanged `1`** (ditto) | `2` |
| `var a=[1]; (a[0])++` | **unchanged `1`** (ditto) | `2` |
| `var a,b; [a,b]=[1,2]` | `a`,`b` both **`undefined`** | `1`, `2` |
| `var x,y; ({x,y}={x:1,y:2})` | both **`undefined`** | `1`, `2` |
| `var a=1,b=2; [a,b]=[b,a]` | **unchanged `1,2`** (the swap idiom silently does nothing) | `2,1` |
| `var o={}; [o.p]=[7]` | `o.p` **`undefined`** | `7` |
| `for ([a,b] of [[1,2]]) {}` | `a`,`b` **unchanged** | `1`, `2` |

Root: `compiler/expr_assign.rs:210-212` — the `AssignTarget::Simple` arm handles only
`ExprKind::Identifier` and `ExprKind::Member`; **every other LHS falls to
`_ => { compile_expr(right)?; }`**, compiling the RHS and assigning nothing. (`AssignTarget::Pattern`
at `:215-219` claims to "fail explicitly" but emits `Op::Pop` and never fails — and the parser never
constructs that variant: all producers use `Simple`.) This is registered as **Slice 0b** (§5).

Also newly confirmed silent-wrong, previously mis-tiered or unlisted:
- **`import('x')` → `undefined`** (not a Promise). `compiler/expr.rs:200` compiles
  `ExprKind::DynamicImport` to `PushUndefined`, and the parser applies **no module gate** to
  `import(...)` (`parser/primary.rs:382`) — so this is reachable in ordinary script context. **T1,
  not T3.** (`import.meta` *is* correctly module-gated at `parser/primary.rs:390`.)
- **`{1n: 'x'}` → key is the empty string** `{"":"x"}` (`compiler/expr_object.rs:117-121`
  "conservative fallback").
- **`obj.#x++` silently keeps the old value** (`compiler/expr_ops.rs:249`).
- **Computed class *accessor* keys hard-error** (`compiler/expr_class.rs:588-590`) — R1's
  "computed class keys correct" holds only for plain computed *methods*.

Newly confirmed absent: `Array.prototype.at` / `String.prototype.at` / `findLast` /
`findLastIndex` / `toSorted` / `Object.hasOwn` / `Object.groupBy` / `String.matchAll`.
Confirmed **correct**, no action: destructuring **declarations** (incl. nested/default/rest, and
parameter destructuring), `IncElem`/`DecElem` (`a[0]++`), `await` microtask ordering, static
fields/methods, plain computed method keys, accessors, `flatMap`, `Promise.prototype.finally`.

---

## §2. Root-cause thesis + coupled-invariant enumeration

### §2.1 Thesis (R2 — corrected layer)

These are not N unrelated feature gaps. They are **one defect class**, and round-1 review
established it sits one layer higher than R1 claimed:

> **Unimplemented syntax is compiled to `Op::PushUndefined` (or to a no-op, or to an `assert!`)
> at the compiler emit layer. The dispatch handlers nominally responsible for those features are
> unreachable dead stubs behind them.**

Verified: `Op::TaggedTemplate`, `Op::DefineField`, `Op::GetSuperProp`, `Op::SetSuperProp`,
`Op::GetSuperElem`, `Op::ImportMeta`, `Op::DynamicImport`, `Op::CreateClass`,
`Op::DestructureElem` **all have zero compiler emit sites** (verified 2026-07-26 via
`for op in …; do grep -rn "Op::$op" crates/script/elidex-js/src/compiler/ | wc -l; done` → 0 for
all of them — 18 in total, see §2.3). So a fix that targets the dispatch stub touches dead code and changes nothing.

Each stub **silently substitutes `undefined` / `false` / a wrong arity** for the spec value. That
is why a whole tier of these was invisible to the defer-slot ledger: a stub throws no error, fails
no test, and registers no TODO.

**P4 = discharging that inventory**, at the emit layer, with connect-or-delete applied to the dead
opcodes behind it. Directly mandated by CLAUDE.md:

- **「TODO 先送り禁止」** — exactly the deferred-implementation pattern the rule forbids; they never
  got the "理由 + 対処時期を明示して確認を取る" step.
- **「dead code は接続するか削除」** — 18 opcodes with documented stack effects and zero emit sites (§2.3).
- **「One issue, one way」** — converge each syntax form onto one emit path.
- **「Ideal over pragmatic」** — no "minimal v1"; each slice ships the spec-complete form.

### §2.2 Stub inventory — Layer A (compiler emit; the live defects)

**Derivation (R2 round 3 — decision 9 resolved empirically, not stipulated).** The table below is
the output of a documented three-pass sweep over `crates/script/elidex-js/src/compiler/`, run
2026-07-26, **not** probe hits plus targeted greps (the method that produced the §1.2 over-claim):

```
# Pass 1 — LEXICAL marker sweep (concept, not one string)
grep -rniE "not yet|not supported|unsupported|conservative|stub|for now|simplified|\
fall(s)? through to push|just evaluate|side effects only|skip \(|would need" *.rs
# Pass 2 — LEXICAL substitution-class enumeration; every hit read and classified
grep -rn "Op::PushUndefined" *.rs        # class 1  → 24 hits, 4 defects
# Pass 3 — STRUCTURAL production coverage (added R2 round 4; passes 1-2 are
#          comment-keyed and cannot see an arm that skips silently *without* a marker)
awk '/^pub enum ExprKind/,/^}/' ../ast.rs | grep -oE '^    [A-Z][A-Za-z]*'   # 30 variants
awk '/^pub enum StmtKind/,/^}/' ../ast.rs | grep -oE '^    [A-Z][A-Za-z]*'   # 24 variants
#   → map every variant to its compiler arm; flag any arm that neither emits an op
#     nor returns CompileError for a user-writable production.
```

**Pass 3 result (run 2026-07-26).** All **30** `ExprKind` and all **24** `StmtKind` arms exist in their compilers, so no
production is unhandled — the class-1/2/3 defects are in arm *content*, already tabled below.
`compile_stmt` groups **6** `StmtKind` variants into one no-op arm (`stmt.rs:31-39`):
`Empty | Error | Debugger | ImportDeclaration | ExportDeclaration | FunctionDeclaration`. Of these
`Empty`/`Error`/`Debugger` are legitimate and `FunctionDeclaration` is legitimate-and-documented
(hoisted at function/script level). **`ImportDeclaration` / `ExportDeclaration` are silent no-ops** —
and both passes 1 and 2 missed them, because the arm emits nothing (pass 2 blind) and its comment
says "stubs", a word not in pass 1's pattern list. Exactly the blind spot round 3 predicted.

**Reachability**: not live today — `parse_module` (`lib.rs:94`) has **no production caller**
(`grep` finds only `scope/tests.rs`), so the VM never parses as a module. But the **parser and scope
analysis already support modules** (`ProgramKind::Module`, `ScopeKind::Module`, import/export
parsing, `scope/tests.rs` exercises all of it). So this is a **latent trap for Slice M**: the moment
module parsing is enabled, `export const x = 1` and `import {x} from 'm'` compile to **nothing**,
silently, with the front end reporting success. Slice M must fix this arm *before* enabling
`parse_module`, not after. Recorded as a Slice-M precondition in §5.

**Granularity caveat (honest scope of pass 3)**: this pass is at *variant* granularity. Two round-3
findings live one level deeper — inside an arm, in a sub-`match`/`if` with no else:
`expr_class.rs:430-447` (`ClassMemberKind::PrivateField` compiled only under `if *is_static` ⇒
`class A{#x=1}` emits nothing) and `expr.rs:186-189` (`ExprKind::Spread` in prefix position compiles
its operand; every such node is an early-SyntaxError position per spec). 0c must run pass 3 at
**both** granularities — variant-level, then sub-arm-level over the enum-shaped inner matches
(`ClassMemberKind`, `MemberProp`, `AssignTarget`, `VarLocation`, `PropertyKey`), which is where
every class-2 defect found so far actually lives.

Pass 1 returns 31 hits, pass 2's `PushUndefined` arm returns 24; **the large majority are
legitimate** (`yield` with no argument, optional-chain null path, parameter/field defaults, empty
returns, `ExprKind::Error` after a parse error already reported). Classification is therefore a
read-every-arm step, and it is what makes the count trustworthy rather than the grep itself.

**The sweep found 3 defects no prior inventory (probe, R1, R2, or round-2 review) contained** —
parenthesized assignment/update targets, all probe-confirmed silent no-ops:
`(x)++` → x unchanged · `(x)+=1` → x unchanged · `(a[0])++` → unchanged. Root:
`parser/expr.rs:531-536` `is_valid_assign_target` unwraps `ExprKind::Paren` to *validate* but never
normalises it, and neither `expr_ops.rs` nor `expr_assign.rs` unwraps, so a parenthesized target
falls to the catch-all arms. This is direct evidence that decision 9's concern was correct and that
a stipulated inventory would have shipped incomplete.

Emit-site counts grep-verified 2026-07-26 (command in §2.1).

⚠ **This table is the probe at baseline `f7d9b5ce`; its Site / Emits / Observable columns are frozen
there.** Read them as "what the sweep found", never as current state. Slice 0a then landed
(`658cc302`) and, beyond its three T0 rows, converted **nine** further rows from a silent no-op into a
*scoped* `Op::ThrowUnsupported` — marked **0a ✅ loud** in the Slice column. The construct is still
unimplemented and the named slice still owns it; what changed is the **failure mode**, which is the
very axis 0c is scoped by, so 0c's charter is narrowed accordingly below. Re-derive with
`grep -rn 'emit_unsupported\|unsupported_member_target' crates/script/elidex-js/src/compiler/`
(9 call sites at `658cc302`, in `expr_assign.rs`, `expr_ops.rs`, `stmt_loop.rs`,
`stmt_destructure.rs`). Site line numbers are `f7d9b5ce`-relative and several have moved:
`compiler/stmt.rs` went 1001→712 when 0a split out `compiler/stmt_loop.rs`, so the `:877-878` and
`:882` rows now live in that new file.

| Site | Syntax | Emits | Observable | Tier | Slice |
|---|---|---|---|---|---|
| `compiler/expr_assign.rs:170` | `obj[k] += v` | **`assert!` → panic** | process abort | **T0** | 0a ✅ |
| `compiler/expr_ops.rs:29` | **`obj.p \|\|= v` / `&&=` / `??=`** (named member) | **`unreachable!` → panic** — short-circuit was implemented only for the *identifier* target, so every member logical assignment reached `compound_op_to_opcode` | process abort | **T0** | 0a ✅ |
| `compiler/expr_assign.rs:170` + `expr_ops.rs:29` | **`obj[k] \|\|= v`** (computed logical) | **panic** (both of the above) | process abort | **T0** | 0a ✅ |
| `compiler/expr_assign.rs:210-212` | `[a,b]=…`, `({x}=…)` | RHS only, no store | silent no-op | T1 | 0b — **0a ✅ loud** |
| `compiler/expr_member.rs:70-76` | `f(...a)` | spread operand as one arg | silent wrong arity | T1 | 1b |
| `compiler/expr_class.rs:428` | `class A{x=1}` | **skipped entirely** | field `undefined` | T1 | 2 |
| `compiler/expr.rs:200-207` | `super.x`, `super[k]` | `PushUndefined` | TypeError at use | T2 | 3 |
| `compiler/expr.rs:237-239` | `` t`a${1}` `` | `PushUndefined` | tag never called | T1 | 4 |
| `compiler/expr_member.rs:44`, `expr.rs:230` | `#x` get / `#x in o` | emits `GetPrivate`/`PrivateIn` → dispatch stub | `undefined` / `false` | T1 | 5 |
| `compiler/expr_assign.rs:202-206` | `o.#x = v` | `Op::Pop` (the `_ =>` arm); **no `SetPrivate` emit site exists at all** | write lost **and** the assignment evaluates to the *object*: `x = (o.#p = 5)` ⇒ `x === o` | T1 | 5 — **0a ✅ loud** |
| `compiler/expr.rs:200-207` | `import('x')` | `PushUndefined` | not a Promise | **T1** | *(see §5)* |
| `compiler/expr_object.rs:117-121` | `{1n: 'x'}` (**literal** key only — `{[1n]:…}` computed is correct, probe-verified) | empty-string key → `{"":"x"}` | wrong key | T1 | 9 |
| `compiler/expr_ops.rs:249` | `obj.#x++` | emits nothing, old value retained | silent no-op | T1 | 5 — **0a ✅ loud** |
| **`compiler/expr_ops.rs:261`** | **`(x)++`, `(a[0])++`** — parenthesized update target | operand evaluated only | **silent no-op** | T1 | 0b — **0a ✅ loud** |
| **`compiler/expr_assign.rs:210-212`** | **`(x)+=1`** — parenthesized assign target (same catch-all as destructuring) | RHS only | **silent no-op** | T1 | 0b — **0a ✅ loud** |
| `compiler/expr_ops.rs:147-149` + **`parser/`** | `delete this.#x` | `Pop; PushTrue` → `true` | wrong constant; ECMA-262 §13.5.1.1 makes it an **early SyntaxError** ⇒ **parse-time** rejection, so 0c's runtime-throw regime is wrong for it (same layer argument as §9 dec. 15). The sibling half of the *same* spec bullet (`delete <identifier>`) is **already** parser-gated, so the two halves must not land in two layers | T1 | **0b** (parser) |
| `compiler/expr_ops.rs:226` | module-binding **update** (`importedBinding++`) — *not* `delete`; falls to the `:261` catch-all, leaving the current value (the in-code "fall through to push undefined" comment is itself stale) | operand only | silent no-op | T1 | M — **0a ✅ loud** |
| `compiler/stmt.rs:877-878` | module-binding `for-in` target | `Pop` | silent no-op | T1 | M — **0a ✅ loud** |
| `compiler/stmt.rs:31-39` | `import`/`export` **declarations**, grouped into the `Empty`/`Debugger` no-op arm (found by the pass-3 structural sweep) | nothing | silent no-op — **latent**: unreachable until `parse_module` gains a production caller | T1 | M (precondition) |
| `compiler/expr_class.rs:430-447` | `class A{#x=1}` — `PrivateField` compiled only under `if *is_static`, no else | nothing | silent no-op | T1 | 5 |
| `compiler/expr.rs:186-189` + **`parser/expr.rs:256-263`** | `ExprKind::Spread` in prefix position (`var y = ...x`) — an early-SyntaxError position per spec; the parser's `Ellipsis` arm is **ungated** | operand only | silent no-op | T1 | **0b** (parser — §9 dec. 15) |
| `compiler/expr_class.rs:588-590`, `:568`, `:621` | `class{get [k](){}}` + 2 sibling key arms | `CompileError` | **loud reject — not a defect**, listed for completeness | T2 | 2 |
| `compiler/stmt.rs:882` | `for (obj.prop in …)` | `Pop` | silent no-op | T1 | 0b — **0a ✅ loud** |
| `compiler/expr_member.rs:92` | **`(o.m)()`** — `compile_call_expr` matches `ExprKind::Member` on the **raw** callee, so a parenthesized callee takes the plain-call branch | `Op::Call` (no receiver pushed) | **`this` is `undefined`** instead of `o` ([C20] step 1.a.i) | T1 | **0b** (the shared `peel_paren` chokepoint — §9 dec. 14) |
| `compiler/stmt.rs:99-103` | **`for await (x of it)`** — the `is_await: _` discard | compiles identically to sync `for-of` | silent **wrong protocol** (sync iterator used for an async iterable) | T1 | 6 |
| `compiler/expr_member.rs:60-64` | `f(a×256)` | **`assert!` → panic** | process abort | T0 | 1b |
| `compiler/expr_assign.rs:215-220` | `AssignTarget::Pattern` | `Op::Pop`, claims to "fail explicitly" but does not | **dead** — parser never constructs this variant | I-4 | 0b — **0a ✅ loud** |

**Verified NOT defects** (checked during the sweep, no action): `delete x` **is** correctly gated by
the parser ("Cannot delete an unqualified identifier in strict mode") — so `expr_ops.rs:156-159` is
unreachable for identifiers, contrary to a round-2 review claim; `stmt.rs:192` `with` → `CompileError`
is correct per I-6/ADR #2; `{[1n]:…}` and `{[/a/]:…}` computed keys are correct.

### §2.3 Stub inventory — Layer B (dispatch; dead code behind Layer A)

These are I-4 connect-or-delete items, **not** live defects. Each is discharged by the slice that
lands its Layer-A emit (connect), or deleted by the dead-opcode sweep (Slice D, adopting
`#11-dead-opcode-removal`).

**⚠ Corrected R2 round 3 — the earlier hand-curated "nine" was wrong in both directions.** Enumerated
mechanically over all 125 `Op` variants in `bytecode/opcode.rs` (2026-07-26): **18** have zero
compiler emit sites.

- **Previously listed, confirmed dead (9)**: `CallSpread`, `NewSpread` (`dispatch.rs:888`),
  `TaggedTemplate` (`:896`), `DefineField` (`:942`), `GetSuperProp`, `SetSuperProp`, `GetSuperElem`
  (`:1036`), `CreateClass` (`:931`), `DestructureElem` (`:923`).
- **WRONGLY listed as dead — `GetModuleVar` IS emitted** at `compiler/expr_assign.rs:52`
  (`VarLocation::Module(idx) => fc.emit_u16(Op::GetModuleVar, idx)`). Removing it under Slice D would
  have broken module-binding reads. Also `ImportMeta`/`DynamicImport` are dead, but for the *reason*
  in §2.2 (the compiler folds them into `PushUndefined`), so they are Slice-M connects, not deletes.
- **MISSING from the earlier list (5)**: `DestructureProp`, `ObjectRest`, `DefaultIfUndefined` — all
  three share the same no-op dispatch arms as the listed `DestructureElem` (`dispatch.rs:925-928`) —
  plus `Debugger` and `SwitchJump`.
- `GetPrivate`/`PrivateIn` **do** have emit sites (1 each) so are *not* dead; `SetPrivate` is.
- `Wide` (`:1083`) raises a loud `VmError` — acceptable, not swept.

Slice D's scope is this corrected enumeration, and it must be **re-derived mechanically at
implementation time** rather than inherited from this list (opcodes gain emit sites as slices land).

Stale "stub" labels on now-real implementations (`SetPrototype`, `AssertConstructor`,
`DefineMethod`, `SuperCall`, `NewTarget`): comment-only cleanup, fold into the touching slice.

### §2.4 Severity taxonomy (drives ordering)

- **T0 CRASH** — process abort on valid JS.
- **T1 SILENT-WRONG** — wrong answer, no error. Strictly worse than a missing global, which at
  least throws `ReferenceError`.
- **T2 LOUD-BROKEN** — throws / total feature loss. Debuggable but unusable.
- **T3 ABSENT** — missing globals; loud `ReferenceError`.

### §2.5 Coupled invariants (edge-dense enumeration — the 1a+1b pair)

Six axes; each **pair's intersection** named:

- **A · Arg-emit form** — flat values + `argc`, or one args Array.
- **B · Call shape** — plain / method / `new` / optional / `super(...)` / `super.m(...)`.
- **C · Iterator protocol** — [C19] spreads via `GetIterator(spreadObj, **sync**)` [C21], drained
  by `IteratorStepValue` [C35]; a user iterator can throw, be infinite, or mutate VM state.
- **D · GC safety** — the args Array is a heap object; unpacking crosses an allocation boundary.
- **E · Inline cache** — `Call`/`CallMethod` carry a u16 call-IC slot; spread opcodes carry none;
  `New` has **no IC at all**.
- **F · Exception unwind** — stack-depth restore on a mid-drain throw.

| Pair | Intersection (a decision, not an assumption) |
|---|---|
| A×B | Each call shape needs both forms ⇒ one spread opcode per shape. `CallMethodSpread` does not exist — the gap forcing a new opcode (§9 decision 2). |
| A×C | The Array form *is* where the iterator drain happens. Flat must never run an iterator; Array always must, even for `f(...[])`. |
| A×E | A site switching to the Array form must not allocate an unused IC slot. **Resolved** (§6.3): `alloc_call_ic_slot` is a monotonic counter and the table is indexed by the bytecode operand — desync is structurally impossible; skip allocation on the Array path. |
| A×F | **Corrected R2r2**: once `lay_out_call_args` succeeds, the Array path leaves the *same* `argc` slots as Flat; the 0-or-1-slot asymmetry exists only for a failure *inside* the helper. The three new handlers must still pick one of the two in-tree error-exit disciplines (§6.3). |
| B×C | Optional-call short-circuit: `nullish?.m(...a)` must **not** evaluate or drain the operand ([C37] step 3 returns before [C38] ChainEvaluation). Iterator side effects are observable. |
| B×D | **Corrected R2r2**: under `lay_out_call_args` the array is popped *before* `do_new` allocates the instance (`vm/ops.rs:766`), so the two are never simultaneously live-and-needed. The real B×D case is the generator/async callee window (§6.3 GC). |
| B×E | `New` has no IC dimension; only `Call`/`CallMethod` do. |
| C×D | **The drain is rooted by construction** (`op_array_spread` `peek`s, so the array stays on `vm.stack`, itself a GC root). The **unrooted window is pop → re-push**: no JS allocation may occur inside it. |
| C×F | [C20] step 3 runs ArgumentListEvaluation **before** the step 4/5 callability checks ⇒ a non-callable callee must still drain the iterator to completion before throwing. **[C19] does NOT call `IteratorClose`** — verified 2026-07-26, `body ecma262 sec-runtime-semantics-argumentlistevaluation \| grep -ci iteratorclose` → **0**; `?` propagates the abrupt completion directly, and §7.4.9 IteratorStep already sets `[[Done]]` on throw. Calling `return()` here would be an *observable divergence*. (Contrast [C39] DestructuringAssignmentEvaluation → 6 `IteratorClose` call sites, which is why [C36] is **Slice 0b's**, not Slice 1's.) |
| D×E | (none — IC slots are compile-time indices, not heap refs.) |
| D×F | The unwind path must not leave the args Array reachable only from a dropped Rust local. |

≥3 intersecting axes ⇒ **edge-dense ⇒ per-slice plan-review mandatory**. Slices 0b/2/3/5/6 carry their own
enumerations in their own memos; §5 names this explicitly for slices 7-10 too. **Also required
(added R2 round 6)**: 0c (three substitution classes × 6 compiler files + `vm/dispatch.rs`, plus a
deliberate user-visible behaviour change per dec. 5 — the program's widest blast radius) and **P**
(a crate-wide convention sweep with a signature change and completion-kind semantics spanning
`compiler/`, core `vm/` and `vm/host/` — edge-dense under trigger (b)). 0a and D are narrow enough
to skip.

---

## §3. Spec coverage map

Scope = **the 1a+1b pair** (per-PR split tagged below). [C19] is an SDO defined piecewise over **8 productions** (prose read via
`.claude/tools/webref body ecma262 sec-runtime-semantics-argumentlistevaluation`, 2026-07-26); 3 are
TemplateLiteral productions that route the *same* SDO into tagged templates and are therefore
listed here as explicit out-of-scope rows with a Slice-4 hand-off (I-3 requires Slice 4 reuse this
slice's helper, and `ArgsForm` as specified cannot express `« siteObj »` ++ substitutions — that
constraint is recorded now rather than discovered in Slice 4).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | production | `Arguments : ( )` — empty list | `compile_call_arguments` (NEW) → `Flat(0)` | ✓ | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | steps 1-3 | `ArgumentList : AssignmentExpression` | `Flat(n)` path | ✓ | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | steps 1-5 | `ArgumentList : ... AssignmentExpression` — leading spread | `Array` path → `Op::CreateArray` + `Op::ArraySpread` | ✓ | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | steps 1-4 | `ArgumentList : ArgumentList , AssignmentExpression` | `Op::ArrayPush` / `Flat` | ✓ | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | steps 1-4 | `ArgumentList : ArgumentList , ... AssignmentExpression` — trailing/multiple spread | `Op::ArraySpread` after prior pushes | ✓ | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | production | `TemplateLiteral : NoSubstitutionTemplate` | — | n/a (out of scope → Slice 4) | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | production | `TemplateLiteral : SubstitutionTemplate` | — | n/a (out of scope → Slice 4) | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | production | `SubstitutionTemplate : TemplateHead Expression TemplateSpans` | — | n/a (out of scope → Slice 4) | yes |
| ECMA-262 §13.3.6.2 EvaluateCall | step 1.a.i | `GetThisValue(thisValueRef)` — property-reference receiver | `Op::CallMethodSpread` (NEW) receiver slot | ✓ | yes |
| ECMA-262 §13.3.6.2 EvaluateCall | step 2.a | not a Reference Record → `thisValue` = `undefined` | `Op::CallSpread` (plain-call shape) | ✓ | yes |
| ECMA-262 §13.3.6.2 EvaluateCall | step 3 | argList = ArgumentListEvaluation — **before** callability checks | the A×C / C×F intersections | ✓ | yes |
| ECMA-262 §13.3.6.2 EvaluateCall | step 4 | `func` not an Object → TypeError | existing `Call` dispatch | ✓ | yes |
| ECMA-262 §13.3.6.2 EvaluateCall | step 5 | `IsCallable(func)` false → TypeError | existing `Call` dispatch | ✓ | yes |
| ECMA-262 §13.3.6.2 EvaluateCall | step 7 | `Call(func, thisValue, argList)` — the invocation | all three new handlers | ✓ | yes |
| ECMA-262 §13.3.6.2 EvaluateCall | step 1.b.iii | `refEnv.WithBaseObject()` — reachable only via `with` | — | n/a (out of scope per I-6) | no |
| ECMA-262 §13.3.5.1.1 EvaluateNew | step 4.a | argList = ArgumentListEvaluation | `Op::NewSpread` | ✓ | yes |
| ECMA-262 §13.3.5.1.1 EvaluateNew | step 5 | `IsConstructor` false → TypeError (**after** step 4.a) | `do_new` | ✓ | yes |
| ECMA-262 §13.3.5.1.1 EvaluateNew | step 6 | `Construct(ctor, argList)` — newTarget omitted | `do_new` | ✓ | yes |
| ECMA-262 §7.3.14 Construct | step 1 | newTarget defaults to `ctor` | `new.target` propagation (edge 13) | ✓ | yes |
| ECMA-262 §7.4.4 GetIterator | steps 1-2 | `kind = sync` selects `GetMethod(obj, %Symbol.iterator%)` | `Op::ArraySpread` (existing) | ✓ | yes |
| ECMA-262 §7.4.4 GetIterator | step 3 | `method` undefined → TypeError (non-iterable) | `Op::ArraySpread` (existing) | ✓ | yes |
| ECMA-262 §7.4.10 IteratorStepValue | steps 1-5 | per-element drain | `spread_iter_loop` (existing) | ✓ | yes |
| ECMA-262 §7.4.11 IteratorClose | steps 1-8 | abrupt completion mid-drain | — | n/a (**not reached from [C19]** → Slice 0b) | yes |
| ECMA-262 §13.3.9.1 Evaluation (optional chain) | step 3 | nullish → return `undefined` **before** ChainEvaluation | optional-call short-circuit (edge 12) | ✓ | yes |
| ECMA-262 §13.3.9.2 ChainEvaluation | step 3 | `OptionalChain : ?. Arguments` — optional **call** `f?.(...a)` → EvaluateCall | `Op::CallSpread` | ✓ | yes |
| ECMA-262 §13.3.9.2 ChainEvaluation | step 6 | `OptionalChain : OptionalChain Arguments` — optional **method** call `o?.m(...a)` → EvaluateCall | `Op::CallMethodSpread` (`expr_member.rs:184`) | ✓ | yes |
| ECMA-262 §13.3.7.1 Evaluation (`super`) | `SuperCall` step 4 | ArgumentListEvaluation (spread variant) | `Op::SuperCallSpread` — already correct, folded onto helper | ✓ | yes |
| ECMA-262 §13.2.4.1 ArrayAccumulation | steps 1-4 | `SpreadElement : ... AssignmentExpression` — `GetIterator(…, sync)` + `IteratorStepValue` drain, **no `IteratorClose`** | **[1a]** `op_array_spread` (dec. 13a removes the non-spec `return()`) | ✓ | yes |
| ECMA-262 §13.2.4.1 ArrayAccumulation | production | `Elision : ,` / `Elision : Elision ,` — array-literal **holes** | `expr.rs:105-119` — contrast only, NOT shared (§9 decision 3) | n/a (out of scope) | yes |
| ECMA-262 §13.3.6.2 EvaluateCall | step 6 | `PrepareForTailCall()` | — | n/a (no tail calls in elidex) | no |
| ECMA-262 §13.3.5.1.1 EvaluateNew | steps 1-2 | constructExpr evaluation + GetValue | `expr.rs:89` callee compile | ✓ | yes |
| ECMA-262 §13.3.5.1.1 EvaluateNew | step 3 | `argumentsNode` empty → new empty List | `ArgsForm::Flat(0)` (`new C`) | ✓ | yes |

**Breadth**: K=1 spec (ecma262), **M=32** (verified by `preflight.py` → "total entries (M): 32"). M≥30 ⇒ **⚠ SPLIT-DEFAULT** — the strongest breadth verdict.

**✅ Answered by splitting, not by override** (2026-07-26, user decision — §9 dec. 6). The work is
now **Slice 1a** (VM infrastructure) → **Slice 1b** (compiler + opcodes + handlers). This map spans
**both**, so M=32 is the *pair's* depth.

**Per-PR recompute** (R2 round 5 — the gate is a per-PR verdict, so a pair-level answer would be
the same author-altitude override dec. 6 disclaims): tagging each row by its Touch column, **1a
owns 5** ([C22] `SpreadElement`, [C21] GetIterator ×2, [C35] IteratorStepValue, [C13] SuperCall)
and **1b owns 20**, with **7** `n/a` shared. So **1b M=20 < 30 ⇒ clears SPLIT-DEFAULT on its own**, and
1a is trivially clear. The split resolves the gate on both sides, not just on the pair. The points below are retained because they explain why this map is deep rather than broad,
and why a **call-shape** split stays forbidden (I-3) — but the gate itself is satisfied structurally.

**Why the map is deep, not broad.** Three points, in order of weight:

1. **The invariant argument (§9 decision 6) is what carries this, not the ratio.** Every candidate
   seam reintroduces an umbrella invariant — VM-first ships 3 dead opcodes (I-4), compiler-first
   emits into stubs (I-1), shape-split is the strangler form that caused the bug (I-3). A breadth
   metric cannot override a structural "must change together".
2. **M here measures coverage *depth on one algorithm*, not scope breadth.** K=1: every row is
   ECMA-262, and 25 of 32 rows are the single call-argument evaluation path viewed through the five
   call shapes. **7 rows are explicit `n/a` out-of-scope hand-offs** (3 TemplateLiteral → Slice 4,
   [C20] steps 1.b.iii and 6, [C36] → Slice 0b, [C22] contrast) — i.e. rows that document what this
   slice does *not* do.
3. **The metric moved for the right reason and should not be gamed.** M rose 23 → 27 → 31 purely
   because rounds 2-4 *added* honest step-level and `n/a` rows; the implementation scope never grew.
   Deleting the `n/a` rows would drop M below the threshold while making the plan strictly worse, so
   the count stays and the gate is answered on substance.

(R1/R2 stated "19 of 23" and R3's draft "21 of 27"; both were stale the moment rows were added —
corrected here, and the ratio is explicitly *not* the argument.)

### §3.1 User-input touch audit

- `compile_call_arguments` (NEW): argument-list shape is user-controlled (count, spread positions).
- `Op::CallSpread` / `Op::NewSpread` (existing, stub → real): args Array contents and callee are
  user-controlled; the spread operand may be any object with `@@iterator`.
- `Op::CallMethodSpread` (NEW): additionally the **receiver** is user-controlled.
- `Op::ArraySpread` (existing, unchanged): already runs a user iterator — this slice **increases its
  reachability** (previously only array literals and `super(...)`), so its throw/GC behaviour is
  newly on the plain-call path. Exposure change: **increased, deliberately**.
- Adjacent pre-existing lax surface: `compile_arguments`'s `assert!(arguments.len() <= 255)`
  (`expr_member.rs:60-64`) is an I-2 violation on user input (a 256-argument call aborts the
  process). **In scope** — this slice rewrites the function.

---

## §4. Cross-cutting design invariants

**I-1 · No silent stub.** Every unimplemented opcode or compiler emit path either is implemented or
raises a **loud, scoped runtime error**. The banned set is **all three *substitution* classes** (§2.1 also names
`assert!`, which belongs to I-2, and this list adds a fourth, *wrong arity*) — R2 round-2 review caught that scoping it to `PushUndefined` alone would have left out
exactly the class that produced the §1.2 retraction:

1. **`PushUndefined`** for unimplemented syntax (`expr.rs:200-207`, `:237-239`);
2. **silent no-op** — emitting nothing, or only the operand's side effects
   (`expr_assign.rs:210-212`, **`expr_assign.rs:202-206`**, `expr_ops.rs:261`, `expr_ops.rs:226`,
   `expr_ops.rs:249`, `expr_class.rs:428`, `expr_class.rs:430-447`, `expr.rs:186-189`,
   **`stmt.rs:99-103`**, `stmt.rs:877-878`, `stmt.rs:882`, and the
   `ImportDeclaration`/`ExportDeclaration` half of `stmt.rs:31-39`);
3. **wrong constant** — a plausible-looking substitute value (`expr_object.rs:117-121` empty-string
   key; `expr_ops.rs:147` `Pop; PushTrue` for `delete this.#x`);
4. **wrong arity** — the right *kind* of value in the wrong *count* (`expr_member.rs:70-75`, the
   call-spread defect itself). Named for completeness; owned by Slice 1b, not 0c, since making it
   loud would break every `f(...a)` site that a working implementation is about to fix.

The class-2 list is the §2.2 table's class-2 rows, kept in sync deliberately — R2 round 3 caught it
as a strict subset, which is how an arm silently drops out of 0c's remit.

**Slice 0c discharges all three program-wide up front** (§5) — **except the two early-SyntaxError
sites**, which §2.2 assigns to **0b (parser)** because a runtime throw is the wrong remedy for a
parse-time rejection (§9 dec. 15): `expr.rs:186-189` prefix `Spread` (class 2) and `expr_ops.rs:147`
`delete this.#x` (class 3). Same shape as class 4's carve-out below. Its scope is derived by a documented
sweep, not by inspection — see §5 Slice 0c and §9 decision 9. §9 decision 5 records the
throw-vs-`CompileError` choice.

**I-2 · `assert!` is not rejection** — *for the refusal-of-user-constructs class only*. A compiler
arm that rejects a **user-writable construct** must never `assert!`/`panic!`/`unreachable!`
(`expr_assign.rs:170`, `expr_member.rs:60`). **The remedy is `Op::ThrowUnsupported`, a scoped
runtime throw — NOT `CompileError`** (dec. 5, ratified by evidence in §17/§18): I-1 asks for loud
*and scoped*, and `CompileError` yields no bytecode for the whole script, so it fails every
unrelated statement too. `CompileError` stays correct only where the compiler *already* rejects, and
for parse-time rejections (dec. 15). Earlier drafts of this invariant said "must produce
`CompileError`"; that contradicted dec. 5 and Slice 0a shipped the contradiction before the gate
caught it. **Explicit carve-out**:
asserts that encode **ISA/bytecode-format invariants** (jump-offset and operand-width bounds —
`compiler/function.rs:194/222/239`, `stmt.rs:808` — all genuine i16/u16 range asserts) **stay as
`assert!`** per
[[feedback_compiler_asserts_are_isa_invariants]], which directs that "assert → Result" findings on
those sites be declined with a pointer to that memo. R2 round-2 review flagged that some of these
*are* user-JS-reachable (a >32 KB function body); that is a known, ratified position, not a defect.
⚠ *This invariant used to close by naming `expr_assign.rs:102`
(`unreachable!("assignment to import binding")`) as becoming reachable with Slice M and Slice M's to
convert. **Slice 0a already converted it**: `grep -c 'unreachable!' compiler/expr_assign.rs` → 0 at
`658cc302`, and the site is `emit_unsupported(fc, "assignment to an imported binding is not
supported")` at `:104`. Slice M inherits no conversion here — only the implementation.*

**I-3 · One argument-emit path.** All call shapes route through **one** helper that decides
flat-vs-array; Slice 4's tagged-template arg list routes through the same helper. **Carve-out
(spec-mandated, R2 round 5)**: the synthesized default derived constructor
(`compiler/expr_class.rs:145-152`) must NOT be folded — ECMA-262 §15.7.14 step 14.a.iv.1 NOTE requires it
not to observably call `%Array.prototype%[@@iterator]`, which the shared path would. See §6.3. The direct lesson
of the present bug: `super` got a correct hand-written spread branch while the other four
`compile_arguments` callers silently shared a broken one.

**I-4 · Connect or delete.** The **18** zero-emit opcodes (§2.3 — rebuilt mechanically in round 3; the earlier "nine" was wrong in both directions) are each either connected by the
slice that lands their Layer-A emit, or deleted by the dead-opcode sweep. §5 names an owner for
every one — no unowned violations.

**I-5 · Layering.** Two directions, and the boundary is the **`engine` feature gate + host-binding
dependency**, not directory name (R2 round-2 correction — `vm/gc/keepalive.rs`, `vm/host_data/`,
`vm/sw_thread.rs`, `vm/worker_thread.rs`, `vm/webidl_sequence.rs`, `vm/wasm_payload.rs` all sit
outside `vm/host/` yet are engine-bound):

- **Outbound**: no slice may add *language semantics* to engine-bound code (`vm/host/` and the
  `#[cfg(feature = "engine")]` surface above).
- **Inbound**: no slice may pull *engine-bound responsibility* (network / HTML fetch / DOM binding)
  into core.
- **Converse (added R2 round 5)**: engine-**gated** is not the same as engine-**bound**. Files under
  `vm/host/` that implement pure ECMA-262 language surface (`vm/host/typed_array_*.rs` = ECMA-262 §23.2.2, and the
  iterator-protocol call sites in `vm/host/{url_search_params,headers/parse_init}.rs` — ⚠ **round-8
  correction: these two are NOT "pure ECMA-262"**; with `vm/webidl_sequence.rs` they implement
  **WebIDL §3.2.21.1** "Creating a sequence from an iterable", which has *no* `IteratorClose` step,
  so they take the same two-layer disposition as their shared helper) are **language-semantics sites the core
  convention owns** — a core-wide convention sweep (e.g. Slice P) *must* reach them, and doing so is
  not an outbound violation. Without this clause an implementer reading the outbound rule literally
  would stop at the core sites and ship the split-contract One-issue-one-way failure the sweep
  exists to remove. **Two-layer cases needing explicit disposition in Slice P's memo**: `vm/webidl_sequence.rs`
  (governed by WebIDL §3.2.21) and **`vm/host/structured_clone.rs`** (WHATWG HTML **§2.7.4** StructuredSerialize
  (§2.7.7 StructuredSerializeWithTransfer for the transfer-list path where the `iter_close` at
  `:1062` actually sits; the in-code docstring's "§2.9" is itself drifted and must be retagged) — its abrupt is a `DataCloneError` thrown by HTML's own loop body, so a
  step-5 flip changes an HTML-defined error surface, not an ECMA-262 one; round 6 corrected an
  earlier mis-classification of this file as pure ECMA-262).

  ⚠ This clause is currently an **allowlist, not a predicate** — state the membership test
  ("which spec governs the algorithm whose completion is being closed?") and **re-derive the file
  list from it at implementation time**, per §13's enumerate-by-command rule. This binds **Slice M** — module-script *fetching* is an ECMA-262 host hook and must go
  through a host seam, not the core VM; the renderer holds no direct network access (CLAUDE.md
  "Security by structure"). Slice M's memo must draw that line before implementation.

Two named consequences: **Slice 7**'s WeakMap/WeakSet ephemeron work lands in `vm/gc/trace.rs`
alongside engine-gated wrapper-store rooting and the keepalive predicate
(`vm/gc/collect.rs:1149→1235→1341`), and must define identical semantics for non-`engine` builds;
**Slice 10** must resolve `Proxy`/`Reflect` × `ObjectKind::HostObject` (`vm/object_kind.rs:272`)
inside core dispatch — a host-side special case would violate the outbound rule.

**I-6 · No sloppy/Annex B surface.** No slice introduces sloppy-mode or Annex B behaviour
([[reference_elidex-js-core-strict-only]]). **Note**: this does *not* gate `Function`/`eval` — per
`docs/design/ja/14-script-engines-webapi.md` §14.1.1, strict-mode `eval` is **core**; only sloppy
`eval`'s caller-scope injection is LegacySemantics. That slice's real gate is a security/CSP policy
decision (§9 decision 7).

---

## §5. Slice plan

Each slice = own PR + own `/elidex-plan-review`. This umbrella is the approved parent making each
narrowly-scoped slice a **terminal unit** (edge-dense base case).

| # | Slice | Primary module(s) | Slot | Tier | Deps |
|---|---|---|---|---|---|
| **0a — implemented, in review** | Compound **and logical** assignment to member targets — killed **3** panic classes (the plan had recorded 1; the other two were found while implementing and land together, same concept + same files). NB only `Dup`/`Swap` exist, so preserving `[obj key]` across the load needs a **new stack-shuffle opcode** ⇒ handler only (**`bytecode/disasm.rs` needs no arm** — it dispatches generically on `op.operand_size()`; this corrects a cost model that also mis-stated Slices 1b/6/D) | `compiler/expr_assign.rs`, `bytecode/opcode.rs`, `vm/dispatch.rs`, `vm/tests/{mod,tests_member_compound_assign}.rs` | **new** `#11-vm-computed-compound-assignment` | T0 | — |
| **0b** | *(Deps: **P**)* **Assignment/update target completeness** — destructuring assignment (`[a,b]=…`, `({x}=…)`, for-of patterns), **parenthesized targets** (`(x)++`, `(x)+=1`, `(a[0])++`, **and the parenthesized _callee_ `(o.m)()`** — one shared `peel_paren` chokepoint, §9 dec. 14), `for(obj.p in …)`, **the two early-SyntaxError rejections** (prefix `Spread`, `delete this.#x` — §9 dec. 15) | `compiler/expr_assign.rs`, `compiler/expr_ops.rs`, `compiler/stmt.rs`, **`compiler/expr_member.rs`** (the `(o.m)()` callee match), **`compiler/expr.rs`**, `parser/expr.rs` (Paren normalisation + the ungated `Ellipsis` arm) | **new** `#11-vm-assignment-target-completeness` | T1 | — |
| **P** | **`IteratorClose` precedence convention** — completion-kind-dependent `iter_close` signature + **15** sites (10 `iter_close(` + 4 `fc.emit(Op::IteratorClose)` + 1 inline re-implementation in `op_array_spread`), **split by governing algorithm first** (**10** ECMA-262 §7.4.11 / **5** WebIDL §3.2.21.1) (§6.2a-2). Gates 0b | `vm/dispatch_iter.rs`, `vm/ops.rs`, `vm/natives_array_hof.rs`, `vm/webidl_sequence.rs`, `vm/host/{typed_array_static,url_search_params,structured_clone,headers/parse_init}.rs`, `compiler/{stmt,expr_yield_star}.rs` | **new** `#11-vm-iteratorclose-precedence-convention` | T1 | — |
| **0c** | I-1 discharge: **all three** substitution classes → loud throw; **+ §7.2 conformance table**. ⚠ **Narrowed by 0a's landing** — the 9 rows marked *0a ✅ loud* in §2.2 are already discharged (all of `expr_assign.rs`'s rows among them). 0c's first act is to **re-run the §2.1 three-pass sweep at `658cc302`** and derive its file list from the residue. The list opposite is the pre-0a one and must not be used as the charter | sweep-derived (§9 dec. 9) — pre-0a list, **stale**: `compiler/{expr,expr_object,expr_ops,expr_class,expr_assign,stmt}.rs` **+ `vm/dispatch.rs`** (the reachable Layer-B arms: `GetPrivate`/`PrivateIn`) | (invariant, no slot) | — | — |
| **1a** | **Call-spread VM infrastructure** (user-adopted split, §9 dec. 6 — **no call-shape change, plus two named semantic fixes**: decs. 13a + 10; NOT unqualified "behaviour-preserving", see §6.4): `lay_out_call_args` stack-layout helper + `Empty` normalisation + convert `op_super_call_spread` to consume it + correct `op_super_call_spread`'s falsified docstring (the `expr_class.rs:145-152` producer is **spec-required** and is NOT folded — §6.3 / I-3 carve-out) + `ic_call`/`ic_call_method` → `call_ic_idx: Option<usize>` (dec. 11) + remove `op_array_spread`'s `return()` (dec. 13a) + **rooting** the 4 unrooted arg windows (dec. 10 — ⚠ *not* `gc_enabled` bracketing; that was overturned in round 8 because `:893`/`:658` hand off to `make_async_coroutine_and_drive`, which drives the async body) | **`vm/dispatch_helpers.rs`** (home of `lay_out_call_args` — the proven cohesion seam, §5 1000-line note), `vm/dispatch_class.rs`, `vm/dispatch_iter.rs`, `vm/dispatch_ic.rs`, **`vm/dispatch.rs`** (the *only* callers of `ic_call`/`ic_call_method`, which dec. 11 re-signatures — without it 1a does not compile; ⚠ these were `:719`/`:730` at `f7d9b5ce` and are `:705`/`:716` at `658cc302`, so 1a must re-derive them by grep at implementation time rather than reading either pair forward), `vm/interpreter.rs` — **no `compiler/` file** (the fold is withdrawn; edge 32 is a test) | `#11-vm-call-spread-arguments` (shared with 1b) | T1 | 0c |
| **1b** | **Call-argument spread — compiler + opcodes**: `compile_call_arguments`/`ArgsForm` + `emit_call` aggregation (dec. 2) + `CallMethodSpread` (dec. 2b) + the 3 handlers + arity-based form selection | `compiler/expr_member.rs`, `expr.rs`, `bytecode/opcode.rs`, `bytecode/disasm.rs`, `vm/dispatch.rs`, **`vm/ops.rs`** (`do_new` §3 rows 17/18, the bound-prefix splice `:696-700` for edge 26, and dec. 12's stack bound — which has no implementation today) | `#11-vm-call-spread-arguments` | T1 | **1a** |
| **2** | Class **instance** field initializers (public) | `compiler/expr_class.rs`, `vm/dispatch.rs`, **`vm/dispatch_class.rs:232-250`** (`construct_synchronous` — the receiver substitution the contract below turns on); **+ `vm/host/custom_elements/`** (no-regression only — see below) | **adopt** `#11-step9-class-extras` | T1 | — |
| **3** | Super property references | `compiler/expr.rs`, `expr_member.rs`, `vm/dispatch.rs`, **+ the frame-state axis: `vm/interpreter.rs:795-806`, `vm/value.rs:1038-1046`, `bytecode/compiled.rs:74`** | **adopt** `#11-step9-class-extras` | T2 | — |
| **4** | Tagged templates + `String.raw` | `compiler/expr.rs`, `vm/dispatch.rs` | `#11-vm-tagged-template-literals` | T1 | 1b (I-3 helper) |
| **5** | Private names complete | `compiler/expr_class.rs`, `expr_member.rs`, `expr_ops.rs`, `expr_assign.rs:202-206`, **`vm/dispatch.rs:1020-1026`** (the `GetPrivate`/`SetPrivate`/`PrivateIn` stub — 0c only makes the reachable arms loud; the implementation is this slice's, and `SetPrivate` is additionally an I-4 connect) | `#11-vm-class-private-fields` + `#11-step9-class-extras` | T1 | 2 |
| **6** | Async generators + async `for await…of` | **`compiler/stmt.rs:103`** (the `is_await: _` discard — the Layer-A emit defect), **`bytecode/opcode.rs`** (no async-iterator opcode exists), `vm/natives_generator.rs`, `vm/object_kind.rs` | **new** `#11-vm-async-generators` | T2 | — |
| **7** | `Map`/`Set`/`WeakMap`/`WeakSet` | `vm/natives_*`, `vm/object_kind.rs`, `vm/gc/` | `#11-vm-map-set-collections` | T3 | — |
| **8** | RegExp completion | `vm/natives_regexp.rs`, `vm/globals_primitives.rs` | `#11-vm-regexp-constructor-and-flags` | T3 | — |
| **9** | Prototype micro-sweep (T3) **+ the T1 `{1n:…}` key fix** — tier-mixed; the T1 row may be pulled forward if the severity ordering is enforced strictly | `vm/natives_array.rs`, `natives_string.rs`, `natives_object/`, `compiler/expr_object.rs` | **new** `#11-vm-es2021-2024-prototype-sweep` | T3 | — |
| **10** | `Proxy`/`Reflect` | `vm/object_kind.rs`, `vm/ops_property.rs` | `#11-vm-proxy-reflect` | T3 | 7 |
| **D** | **Dead-opcode sweep** — mechanically re-derive the §2.3 set and delete what no slice connected | `bytecode/opcode.rs`, `vm/dispatch.rs` | **adopt** `#11-dead-opcode-removal` | — | after 1b-5 |
| **M** | **ES modules — PROMOTED TO ITS OWN UMBRELLA, outside this plan's slice sequence** (R2 round 5). It carries the `stmt.rs:31-39` precondition, 3 §2.2 rows (⚠ two of them — the module-binding update and for-in head — are **0a ✅ loud**, so M inherits their *implementation*, not their conversion; I-2's `expr_assign.rs:102` `unreachable!` is likewise already converted), I-5's host-fetch-seam boundary, and the `ImportMeta`/`DynamicImport` connects = ≥3 intersecting invariant axes, so the CLAUDE.md edge-dense rule forbids one PR. Only the **`#11-vm-dynamic-import`** T1 carve stays homed here (0c makes `import()` loud; the Promise-returning impl belongs to the module umbrella) | — | `#11-vm-dynamic-import` | T1 | — |
| **—** | `Function`/`eval` | — | `#11-vm-function-constructor-global` | policy | §9 dec. 7 |

**Ordering rationale.** 0a first on severity (process abort). *(Rounds 2-9 put a `vm/dispatch.rs`
prereq split ahead of it; removed — see the 1000-line check below.)* 0b next — a silent no-op on the
ubiquitous swap/destructure idiom, and it shares `expr_assign.rs` with 0a. 0c discharges I-1
program-wide and lands the conformance table (§7.2) so every later slice inherits a baseline.
**P** must land **before 0b** (0b's [C39]→[C36] conformance claim inherits the inverted contract
otherwise). **1a then 1b** proceed per the registered slot ordering and standing directive. 2/3 follow
(new machinery). 5 depends on 2 (`#x = 1` reuses the field-initializer mechanism). 4 depends on **1b** (I-3
requires reusing the arg helper). D runs after the connecting slices so it deletes only what
remains.

**`import()` note**: tiered **T1** (silent `undefined` in ordinary script context), so it is *not*
deferred with the rest of ES modules as R1 implied. Interim: 0c makes it throw loudly; the real
implementation lands with Slice M.

**Slice M precondition (found by the §2.2 pass-3 structural sweep)**: `StmtKind::ImportDeclaration`
and `StmtKind::ExportDeclaration` are already grouped into `compile_stmt`'s **no-op arm**
(`stmt.rs:31-39`), while the parser and scope analysis fully support modules. Enabling
`parse_module` therefore makes `export const x = 1` compile to nothing **silently**. Slice M must
fix that arm **before** wiring module parsing, and its plan-memo must state so — otherwise the
module program's first milestone ships the exact silent-wrong class this umbrella exists to remove.

**Slices 7-10 coupled-invariant duty**: each needs its own §2.5-style enumeration (see I-5 for the
Slice 7 / Slice 10 boundary obligations).

**Slice 3 frame-state axis** (R2 round 2): `super.x` in an *ordinary method* needs frame state that
does not exist. `vm/interpreter.rs:795-806` sets `home_class` **only on class-ctor frames**, and
`vm/value.rs:1041-1046` documents this as "fail-closed-by-construction: a non-ctor method frame has
`home_class = None`, so any future super-property reader trips a SyntaxError fallback". So Slice 3 is
**not** emit+dispatch only — it must add `[[HomeObject]]`-equivalent frame state ([C24]/[C25] both key
off it). The §2.2/§2.3 Layer-A/Layer-B decomposition has no row for this third (frame-state)
dimension; Slice 3's memo must add one.

**Slice 2 custom-element receiver contract** (R2 round 2): `construct_synchronous`
(`vm/dispatch_class.rs:232-250`) substitutes the receiver ("object-return-wins, else pre-alloc"), and
`vm/host/custom_elements/html_element.rs:201` returns a *cached wrapper* on the upgrade path
(`upgrade.rs:315` is the only host caller). If [C27] `InitializeInstanceElements` runs against the
pre-allocated instance, `class X extends HTMLElement { y = 1 }` initialises fields on a discarded
object — reproducing the exact T1 silent-wrong class this program exists to remove. Contract: **field
initialisation applies to the post-substitution receiver**, with a no-regression test on the
custom-element upgrade path.

**Slot triggers this program fires** (register at landing): `#11-compiler-class-emit-readability`
(re-eval 2026-08-08; Slices 2/5 touch `expr_class.rs`) and `#11-reflect-apply-ce-test` (paired with
Slice 10).

**1000-line touch-time check — the standalone prereq split is withdrawn (2026-07-27), on a narrower
ground than the first statement of this section claimed.** Rounds 2-9 all carried a mandated
"standalone prereq split branch" for `vm/dispatch.rs`.

⚠ **The 2026-07-27 reversal shipped after the §15 convergence call, ungated, and four of its five
measurements were wrong** — caught by this document's own plan-review at PR-B (`#506`). They are
restated below from measurement; the erroneous originals were 1036/1113 (93%), 68, 13, and — the
figure the argument turned on — "29 arms (16 `continue`, 13 `return`)", which summed two occurrence
counts as though the sets were disjoint. Re-derive with the script in this section's commit rather
than reading these forward:

| Measure | `f7d9b5ce` (rounds 2-9 basis) | `658cc302` (0a landed) |
|---|---|---|
| file (`wc -l`) | 1112 | **1103** |
| top-level items besides `run()` | `use`, `fn resolve_delete_base` (L21), `fn complete_inline_frame` | same |
| opcode match arms | **104**, spanning 1008 of 1112 lines (91%) | **108**, spanning 996 (90%) |
| arms ≤ 8 lines | 69 of 104 | 75 of 108 |
| arms > 20 lines | 12 | 11 |
| arms using inline loop control flow | **20** (13 `continue`, 13 `return`, **6 both**) | **18** |
| arms > 8 lines with **no** `continue`/`return` | 17 | **18, totalling 293 body lines** |
| `dispatch_{class,helpers,ic,iter,objects}.rs` | 1725 across 5 files | 1888 |

Two conclusions, and they are **not** the same conclusion:

1. **The match is not split.** CLAUDE.md's discipline is cohesion judgment, not line-count mechanics,
   and names the exemption: *「一枚岩の cohesive unit・巨大 generated table・**flat な case table** は
   対象外」*. A 104-arm opcode dispatch table is that case. Independently, 20 arms `continue`/`return`
   the dispatch loop directly and read loop-local state (`entry_frame_depth`, `frame_idx`, `func_id`,
   `ip`, `bytecode`), so relocating the *match* into a second file means threading a control-flow enum
   through those 20 — a semantic rewrite, the opposite of what a prereq split is for.

2. **The file is NOT exempt from reduction, and the earlier text claimed it was.** "The match cannot
   be split without a behavioural rewrite" was true of the match and was then read as covering the
   file. It does not: 18 arms exceed 8 lines while carrying **no** loop control flow at all, 293 body
   lines between them, and the `dispatch_*.rs` family is the in-tree seam that already absorbs exactly
   this shape. Slice 0a demonstrated it — `dispatch_helpers.rs` 237→391, `dispatch_objects.rs`
   429→438, `dispatch.rs` 1112→1103 — without touching one control-flow arm.

**Decision: no standalone prereq-split PR — because the debt is discharged continuously, not because
the file cannot be reduced.** The forward rule for every slice that touches this file: extract the
*arm body* into the existing `dispatch_*.rs` family, never grow an arm in place. That rule is the
discharge mechanism, so a slice that adds an arm and leaves the file larger has not complied.
(The withdrawn text also argued the reversal "takes a PR off the critical path and unblocks the T0 fix
immediately" — schedule is judgment-supporting information, not a design constraint (CLAUDE.md
*Ideal over pragmatic*), so it is removed rather than restated.)

**Files the watch list still owns** (`#11-d17b-dispatch-expr-file-growth`): `vm/interpreter.rs` 1366
(1a + Slice 3), `vm/value.rs` 1187 (Slice 3). **Not on any watch list and >1000**:
`vm/object_kind.rs` 1700 (Slices 6/7/10 — and see the cross-lane note in §8),
`vm/gc/collect.rs` 2074, `vm/gc/trace.rs` 1255 (Slice 7). Each needs a cohesion verdict in its own
slice's memo — and this reversal is the precedent: **measure the file's shape before assuming a
split**.

**Not slotted**, each with its disposition: `Intl` → [[intl-icu-deferral]] (no ICU dep; standing project decision). `Atomics` → **Why**: meaningless without SharedArrayBuffer, which is itself COEP/COOP-gated, so the
feature has no reachable surface today; **Trigger**: SAB + COEP/COOP land; **Re-eval** 2027-01-31. `WeakRef`/`FinalizationRegistry` → **Why**: needs a GC-observability design of its own (exposing collection timing to script); **Trigger**: a WPT/site depending on them, or the GC-observability pass; **Re-eval** 2027-01-31.

---

## §6. Slice 1a + 1b detail — call-argument spread

### §6.1 Root cause

`compiler/expr_member.rs:65-77` `compile_arguments` matches `Argument::Spread(e)` and compiles the
**operand as a plain argument** ("Spread arguments are not yet supported … The stack remains
balanced since the spread expression produces one value, matching the argc count"). So `f(...[1,2])`
passes the array as argument 0 — stack-balanced, silently wrong.

### §6.2 The reference implementation already in-tree

`super(...)` at `expr_member.rs:113-129`: `has_spread` check → `CreateArray`/`ArrayPush`/`ArraySpread`
→ `Op::SuperCallSpread`. `Op::ArraySpread` already runs the [C21]+[C35] drain (implemented at
`vm/dispatch_iter.rs:44`, **not** a stub), which is why `[...gen()]` and `super(...gen())` work.
Note this implements [C22]'s `SpreadElement` drain rather than [C19]'s List-append drain; the two
are step-equivalent apart from array materialisation — **except for iterator closing, §6.2a.**

### §6.2a ⚠ Pre-existing defect in the reused drain (found R2 round 3)

`op_array_spread` (`vm/dispatch_iter.rs:53-66`) **calls the iterator's `return()`** on a mid-drain
error. Neither [C19] ArgumentListEvaluation **nor** [C22] ArrayAccumulation contains an
`IteratorClose` call (both verified `grep -ci iteratorclose` → 0), so this is a **live spec
divergence today for array literals** (`[...it]`), independent of this program. Its docstring also
claims "if `.return()` also throws, its error takes precedence over the original iteration error",
which inverts [C36] §7.4.11 **step 5** ("If completion is a throw completion, return ? completion" —
the *original* wins); the code `?`-propagates the `return()` error, matching the docstring, not the
spec.

This matters because §3.1 records that **1b** **deliberately increases
`op_array_spread`'s reachability** onto the plain-call path. Two consequences:

1. §6.4 edge 21 ("assert `return()` is NOT called") **fails on day 1** against the code being reused,
   unless the drain is fixed.
2. Fixing it changes existing array-literal behaviour, so it needs an owner, not a silent edit.

### §6.2a-2 ⚠ The precedence inversion is crate-wide, not local (corrected R2 round 4)

R2 round 3 scoped this to "the same 14 lines"; round 4 corrected it to "5 sites"; round 5 to "~15";
**round 6 to the verified 14** (10 `iter_close(` callers + 4 `fc.emit(Op::IteratorClose)` — round 5's
"5 emit sites" had counted a *comment* line). Four successive enumerations, each wrong. The
list below is therefore **derived mechanically**, not by inspection (the method that worked for §2.2
and §2.3):

```
grep -rn "iter_close("        crates/script/elidex-js/src/ | grep -v "fn iter_close"  # 10 call sites
grep -rn "Op::IteratorClose"  crates/script/elidex-js/src/compiler/                   #  5 hits → 4 emits (`stmt.rs:169` is a COMMENT)
grep -rn "takes precedence\|step 6-7" crates/script/elidex-js/src/                    # 17 (incl. unrelated)
```

**10 `iter_close(` call sites** (verified 2026-07-26): `vm/dispatch_iter.rs:309`, `:337` ·
`vm/natives_array_hof.rs:485` · `vm/webidl_sequence.rs:141`, `:149` · `vm/ops.rs:60` ·
`vm/host/url_search_params.rs:315` · `vm/host/structured_clone.rs:1062` ·
`vm/host/typed_array_static.rs:798` · `vm/host/headers/parse_init.rs:207`.

**4 compiler `Op::IteratorClose` emit sites**: `compiler/stmt.rs:175` (**the `for-of` catch handler —
the most reachable IteratorClose path in the language**), `stmt.rs:913`,
`compiler/expr_yield_star.rs:146` (the `yield*` **throw** route), `:157` (the finally route).

**SoT**: `iter_close` (`vm/dispatch_iter.rs:354`, docstring `:340-353`) — the canonical §7.4.11 implementation, whose
*docstring states the inverted rule as its contract*: "if `.return()` itself throws, having that new
throw take precedence over the triggering abrupt completion". Exactly four sites additionally cite
"§7.4.11 step 6-7" in support of the inverted rule.

**⚠ A 15th site the two greps structurally cannot see** (round 7): `op_array_spread`
(`vm/dispatch_iter.rs:53-63`) **re-implements IteratorClose inline** — it looks up
`well_known.return_str` and calls the method directly, never calling `iter_close`. Verified:
`grep -rn "return_str" crates/script/elidex-js/src/vm/` returns exactly **2** implementation sites
(`:57` inline, `:359` inside `iter_close`), so there is exactly one such duplicate. Consequences:
(a) "fixing only `op_array_spread` fixes 1 of 14" is **wrong** — it is not one of the 14; (b) Slice P
is sequenced before 0b and hence before 1a, so P would convert 14 sites while the 15th keeps the
inverted convention until dec. 13a lands — and if 13a were rescoped the site stays inverted
invisibly. **The concept grep *was* run** (`takes precedence\|step 6-7` → 17 hits) **but its hits
were dismissed as "incl. unrelated" and never classified — which is exactly how this one dropped
out.** Rule: classify every hit; never filter by expectation.

**⚠ 4 of the sites are not governed by ECMA-262 at all** (round 7): `vm/webidl_sequence.rs:141`,
`:149`, `vm/host/url_search_params.rs:315` and `vm/host/headers/parse_init.rs:207` all implement
**WebIDL §3.2.21.1 "Creating a sequence from an iterable"**, which has **zero** `IteratorClose`
steps (verified `body webidl create-sequence-from-iterable | grep -ci iteratorclose` → **0**;
control `body ecma262 sec-array.from` → 1, so the grep discriminates). For those sites the question
is **not** precedence but whether `return()` should be called *at all* — the same prior question
§2.5 C×F and §6.2a already answered ("remove it") for [C19]/[C22], never propagated to this list.
**Slice P must split its 15 sites by governing algorithm before choosing a remedy**, and I-5's
"pure ECMA-262 language surface" label is wrong for these three files (their shared helper
`webidl_sequence.rs` is already flagged as a WebIDL two-layer case — same governance, inconsistent
labels).

Where the completion is a **throw**, [C36] **step 5** applies ("If completion is a throw completion,
return ? completion" — the *original* wins), so every abrupt-path site above is inverted. Step 5
tests **only** for throw completions, so **normal, return, break and continue** completions all fall
through to steps 6-7, where a `.return()` throw legitimately *does* propagate. The correct behaviour
is therefore **completion-kind-dependent**, which `iter_close`'s current signature cannot express —
a design change, not an edit.

⚠ The normal-completion half is not academic: [C39] DestructuringAssignmentEvaluation calls
`IteratorClose(iteratorRecord, NormalCompletion(unused))` unconditionally at one site and passes a
possibly-normal completion at four more — only **one** of its six call sites is abrupt-gated. So the
algorithm this unit is sequenced *before* passes normal completions at 5 of 6 sites, exactly the kind
an "abrupt-only" reading would mishandle.

**Why this lands on the critical path**: §8 declares Slice 0b "owns the `IteratorClose` obligation
1a/1b do not". Slice 0b will call `iter_close` and thereby **inherit the inverted contract**,
making its [C39]→[C36] conformance claim false. And because the sites span `compiler/`, core `vm/`
and `vm/host/`, fixing only `op_array_spread` fixes **1 of 15** — the `for-of` catch handler
(`stmt.rs:175`) would stay inverted, which is the path most user code actually hits.

**This is the same failure mode twice**: round 3 caught me propagating the IteratorClose *mandate* to
four sites but not the fifth; round 4 caught the *precedence* concept having its own un-swept
siblings. Both are [[feedback_semantic-sibling-selfseed-and-regate-breadth]] — the lesson is to grep
the **concept**, and a concept discovered mid-paragraph needs its own sweep, not an inherited scope.

⚠ **After 1a, `op_array_spread` is [C19]/[C22]-only.** §5 sequences **0b before 1a**, and 0b owns
[C39], whose rest form (`[a, ...rest] = it`) needs a drain-into-array and whose spec *requires*
`IteratorClose` — while `op_array_spread`/`spread_iter_loop` is the only in-tree drain-into-array.
**0b must therefore give its rest path an explicit `iter_close` site of its own**, not reuse this
drain, and §7.2 must pin it. (Executing dec. 13a's propagation instruction here, in §6.2a, where a
0b implementer reads it.)

**Decision** (§9 decision 13, restated): the drain fix is **1a's** (it is the reuse precondition),
but the **precedence sweep is its own unit** — **15 sites** (§6.2a-2), a signature change on the shared helper, and
a completion-kind distinction. Carve `#11-vm-iteratorclose-precedence-convention` and sequence it
**before Slice 0b** (whose conformance claim depends on it), not inside 1a.

### §6.3 Design — split across Slices 1a and 1b

**Slice allocation** (the user-adopted **1a/1b** split, §9 dec. 6 — not to be confused with the
removed `dispatch.rs` file split). Everything below is tagged:

- **[1a]** = VM infrastructure — **no call-shape change, plus two named semantic fixes** (decs. 13a
  `return()` removal and 10 GC rooting, each with its own edge row; see §6.4): `lay_out_call_args`, `Empty`
  normalisation, `op_super_call_spread` conversion, the `op_super_call_spread` docstring correction (the `expr_class.rs:145-152` producer is spec-required, NOT folded), the IC `Option<usize>` refactor, `op_array_spread`'s `return()` removal, and
  GC rooting of the 4 unrooted arg windows (dec. 10). No opcode is added and no emit path changes, so **1a is observably a
  no-op for every call shape** — its only live consumer is `op_super_call_spread`, which keeps its
  current semantics. That is what makes it a legal standalone PR under I-4.
- **[1b]** = the compiler helper, `emit_call` aggregation, `CallMethodSpread`, the three handlers,
  and arity-based form selection — i.e. everything that changes observable behaviour.

1a's acceptance test is therefore *"the full existing suite passes unchanged"*; 1b's is §6.4.

**Compiler** — one helper (I-3), replacing `compile_arguments`:

```rust
/// [C19] ECMA-262 §13.3.8.1 ArgumentListEvaluation.
pub(super) enum ArgsForm { Flat(u8), Array }
fn compile_call_arguments(…) -> Result<ArgsForm, CompileError>  // (NEW)
```

`compile_arguments` has **5 call sites** (verified 2026-07-26 via
`grep -rn 'compile_arguments(' crates/script/elidex-js/src/`: `expr_member.rs` lines 102/131/137/176
+ `expr.rs` line 90; line 53 is the definition), feeding **7 user-facing emit sites**:

| Emit site | Shape | Flat op | Spread op |
|---|---|---|---|
| `expr_member.rs:105` | method call `o.m(…)` | `CallMethod` | **`CallMethodSpread` (NEW)** |
| `expr_member.rs:105` | **`super.m(…)`** — `Member{object: Super}` takes this branch | `CallMethod` | `CallMethodSpread` |
| `expr_member.rs:129/133` | `super(…)` | `SuperCall` | `SuperCallSpread` — fold onto helper |
| `expr_member.rs:140` | plain call | `Call` | `CallSpread` (exists, stub) |
| `expr_member.rs:184` | optional method | `CallMethod` | `CallMethodSpread` |
| `expr_member.rs:186` | optional call | `Call` | `CallSpread` |
| `expr.rs:92` | `new` | `New` | `NewSpread` (exists, stub) |

**`super.m(...)` cross-slice contract**: `compile_call_expr` matches `ExprKind::Member` **first**
(`expr_member.rs:92`), so `super.m(...)` is a method call whose receiver comes from
`compile_expr(ExprKind::Super)` → `PushUndefined` (`expr.rs:200-207`, Slice 3's stub). Slice 1
therefore **does** change this shape's argument emission while its receiver stays broken. Slice 1
must not claim to fix it and must not make it *differently* broken; §6.4 edge 23 is the
no-regression guard, and Slice 3 lands `GetSuperProp` on top of the `CallMethodSpread` emit Slice 1
introduces.

**Second `SuperCallSpread` producer — ⚠ SPEC-REQUIRED, do NOT fold (corrected R2 round 5).**
`compiler/expr_class.rs:145-152` emits `SuperCallSpread` as **raw bytecode** in the synthesized
default derived constructor (`GetLocal 0; SuperCallSpread; Pop; ReturnUndefined`), with its args
array built by **rest-param packing** (`has_rest_param: true`) rather than `ArraySpread`.

Rounds 3-4 of this plan called that "the second mechanism I-3 exists to prevent" and told Slice 1 to
fold it. **That was a spec error.** ECMA-262 **§15.7.14 ClassDefinitionEvaluation step 14.a.iv.1 NOTE**
(verified `webref body ecma262 sec-runtime-semantics-classdefinitionevaluation`):

> This branch behaves similarly to `constructor(...args) { super(...args); }`. The most notable
> distinction is that while the aforementioned ECMAScript source text **observably calls the
> %Symbol.iterator% method on %Array.prototype%**, **this function does not**.

The shared path (`CreateArray; ArraySpread×n; SuperCallSpread`) runs the [C21] `GetIterator` drain, so
folding would make `%Array.prototype%[@@iterator]` observable for a bare `class B extends A {}` —
an observable conformance violation. The hand-rolled producer is **the spec-conformant
implementation**, and the in-tree code is already correct.

**Therefore**: this is an explicit **I-3 carve-out**, not a debt. Slice 1a's only obligation here is
to **correct `op_super_call_spread`'s docstring**, whose claimed invariant ("the compiler emits
`CreateArray; ArraySpread x; SuperCallSpread`") is genuinely falsified by a second *legitimate*
producer — that part is behaviour-preserving. §6.4 gains edge 32 asserting
`%Array.prototype%[Symbol.iterator]` is **not** called for `class B extends A {}`, since no existing
test covers iterator non-observability and 1a's "existing suite passes unchanged" cannot detect it.

**Arity, not just spread-presence, selects the form** (corrected R2 round 4). `ArgsForm::Flat(u8)`
caps at 255 and there is **no wide-operand escape** (`Op::Wide` is a loud `VmError` at
`dispatch.rs:1083` with no emit site; `Call`/`CallMethod` are `emit_u8_u16`, `New`/`SuperCall`
`emit_u8`). Keying the choice on spread-presence alone would convert the T0 crash at
`expr_member.rs:60-64` into a **permanent `CompileError` on legal ECMAScript** (`f(a1…a300)`) —
trading a crash for §2.4's T2 "unusable" tier, contradicting "no minimal v1". The rule is therefore:

> `Array` when **any spread is present OR the argument count exceeds 255**; `Flat(n)` otherwise.

The operand-width invariant then holds **by construction** instead of by rejecting valid programs,
and the `assert!` is **deleted outright** rather than converted to `CompileError` — nothing is left
for it to reject. The Array path is unbounded and must be: `f(...arrOf1000)` is legal.
§6.4 edge 6 covers spread-driven >255; edge 30 covers the flat-arity case.

**New opcode `CallMethodSpread`** `[receiver callee args_array -- result]` — completes the existing
one-spread-op-per-call-shape family. §9 decision 2 records the alternatives, including the
form-as-data option round-1 review raised.

**VM** — the shared helper must be a **stack-layout** operation, not a `Vec`-returning one. Verified
consumer contracts diverge: `op_super_call_spread` (`vm/dispatch_class.rs:72`) hands a **slice** to
`dispatch_super`, whereas `ic_call`/`ic_call_method` (`vm/dispatch_ic.rs:222/268`) and `do_new`
(`vm/ops.rs:681`) read args **from `self.stack`** (`args_start = self.stack.len() - argc`, callee at
`args_start - 1`, receiver at `args_start - 2`). One `Vec`-returning helper cannot serve both. The
canonical in-tree form already exists: `construct_synchronous` (`vm/dispatch_class.rs:303-306`)
pushes callee then args "matching `Op::Call`'s shape". Specify:

```
lay_out_call_args(args_array_value) -> Result<usize /*argc*/, VmError>
```

leaving `[…, arg0..argN]` on the stack, and convert `op_super_call_spread` to consume it too.

**Hole sentinel**: `Op::ArrayHole` pushes **`JsValue::Empty`** into `ObjectKind::Array { elements }`
(`vm/dispatch_objects.rs:114-121`), and indexed-write/`delete` paths also write `Empty`
(`vm/ops_element.rs:680/755`, `vm/dispatch.rs:660`). `op_super_call_spread` does a bare
`elements.clone()` — safe only by the undocumented invariant that args arrays are `Empty`-free. The
crate already has the correct answer: `collect_array_like` (`vm/natives_function.rs:272-277`, the
`Function.prototype.apply` path) normalises via `.or_undefined()`. The shared helper adopts that
normalisation and documents the invariant. §9 decision 8 records whether to also route
`collect_array_like`'s dense fast path through it.

**IC** (A×E, **resolved for desync; entry-point choice open**): `alloc_call_ic_slot`
(`compiler/function.rs:115-118`) is a monotonic counter; the table is sized
`vec![None; call_ic_slot_count]` (`function.rs:410`) and indexed by the u16 operand in the bytecode,
with bounds-safe `.get`/`.get_mut` (`dispatch_ic.rs:206/335`). Desync is structurally impossible ⇒
**skip allocation on the Array path**.

⚠ R2 round-2 review established that "spread handlers call a non-IC entry point" is
**under-specified: no such entry point exists.** `ic_call` (`vm/dispatch_ic.rs:222-263`) and
`ic_call_method` (`:268-327`) own the entire three-way body (IC hit → `push_js_call_frame` / IC miss
→ `extract_js_callee` + `populate_call_ic` + `push_js_call_frame` / native or non-callable →
`to_vec` + `truncate` + `self.call()`). The only existing non-IC entry, `Vm::call`
(`vm/interpreter.rs:76`), is **synchronous** (re-entrant `run()`), so routing spread calls there
would change re-entrancy, frame depth and exception behaviour relative to the flat path for the same
JS. The two real options are (i) duplicate ~35 lines × 2 — a second canonical call-dispatch
implementation, the One-issue-one-way failure one layer below I-3 — or (ii) make
`ic_call`/`ic_call_method` take `call_ic_idx: Option<usize>`, keeping one SoT. **Recommendation:
(ii)**; §9 decision 11 ratifies. (Note the sentinel-index option §9 dec. 2 rejected is in fact
structurally safe — both accesses are bounds-checked, so an out-of-range sentinel reliably misses
and cannot alias; the "aliases another site's slot" objection holds only for an *in-range* fixed
index.)

**GC** (C×D, **re-derived end-to-end in R2 round 2**): the drain is rooted by construction
(`op_array_spread` `peek`s; the array stays on `vm.stack`, a GC root — `vm/temp_root.rs:12-16`).
Under the `lay_out_call_args` contract the **pop → re-push window contains no JS allocation**
(pop → `elements.clone()` → push), so it is *not* the hazard; and `do_new`'s instance
`alloc_object` (`vm/ops.rs:761-771`, not `dispatch.rs:864-873` — that guard is `Op::CreateArguments`)
runs *after* re-push with every argument rooted on the stack.

The window this slice actually **widens** is downstream, inside `push_js_call_frame`: for a
**generator** callee `vm/interpreter.rs:914` drains into a Rust-local `stack_slice`, then `:946`
allocates the Generator object with no `gc_enabled` suppression; for an **async** callee `:867` →
`:893` allocates the wrapper Promise. Across that region the argument values live only in a
Rust-local, and `alloc_object` (`vm/inner.rs:78-98`) may run GC before insertion. Today `g(...arr)`
passes **one** value through it; after this slice it passes **N** user-controlled values. Slice 1
**roots** that region (§9 dec. 10) — ⚠ *not* by `gc_enabled` bracketing: `:893` is
`make_async_coroutine_and_drive`, which ends by driving the async body, so bracketing there would
disable GC across user JS. Root `stack_slice` **and** `actual_args`; edges 27/28 must assert both.
The same shape exists at `call_internal` (`:629-659`, `:664-700`) — **4 windows, one unit**.

**Unwind** (A×F): `Op::Call`/`CallMethod`/`New` hand-roll `vm_error_to_thrown` +
`handle_exception(…, entry_frame_depth)` (`dispatch.rs:716-750`); `op_super_call_spread` routes
through `throw_error(…, entry_frame_depth)` (`dispatch_class.rs:72-95`). The three new handlers must
choose one discipline explicitly. They must **NOT** call `IteratorClose` — see §2.5 C×F and §6.2a.
(R2 round 3 caught this paragraph still mandating the opposite after [C36] was moved to Slice 0b in
four other sites; a self-inflicted incomplete propagation of exactly the
[[feedback_semantic-sibling-selfseed-and-regate-breadth]] shape.)

### §6.4 Edge matrix

1. `f(...a)` · 2. `f(1,...a)` · 3. `f(...a,2)` · 4. `f(...a,...b)` · 5. `f(...[])` → argc 0 ·
6. `>255` effective args via spread (no `u8` truncation) · 7. non-iterable `f(...1)` → TypeError ·
8. `f(...'ab')` · 9. `f(...g())` · 10. `arguments.length` / `arguments[i]` · 11. `this` on
`o.m(...a)` · 12. optional-call short-circuit `nullish?.m(...a)` — operand must **not** evaluate
[C37 step 3] · 13. `new C(...a)` + `new.target` [C34 step 1] · 14. `super(...a)` no-regression ·
15. IC bookkeeping · 16. GC across pop→re-push · 17. `f.bind(x)(...a)` ·
18. native/host callees (`Math.max(...a)`) · 19. left-to-right evaluation order ·
20. `f(...[1,,3])` — assert **argument values** `[1, undefined, 3]`, not just arity (covers the
`Empty` boundary) · 21. iterator throws mid-drain → stack/GC unwind, and assert `return()` is
**NOT** called (§2.5 C×F — [C19] has no `IteratorClose`) · 22. iterator mutates callee/receiver
mid-drain · 23. **`super.m(...a)` no-regression** (stays Slice-3-broken, not differently broken) ·
24. **non-callable callee + observable iterator**: `f=1; f(...it)` must drain `it` fully **before**
throwing — a Number callee throws at [C20] **step 4** (`f={}` reaches step 5) · 25. same for
`new NotACtor(...it)` [C33 steps 4.a→5] · 26. `new BoundCtor(...a)` — bound-prefix splice
(`vm/ops.rs:696-700`) composed with the spread layout, asserting `boundArgs ++ spreadArgs` order ·
27. **generator callee** `function* g(a){arguments}; g(...arrOf1000)` — the GC window, asserting
against **`actual_args`** (the larger unrooted vector), not just `stack_slice`
(`vm/interpreter.rs:811/846/914/946`) · 28. **async callee** `async function a(){}; a(...arr)` — same
(`:867/:893`) · 29. very large spread (`f(...arrOf100k)`) — stack-depth behaviour (§9 dec. 12) ·
30. **flat arity >255 without spread** `f(a1…a300)` — must compile and run, selecting the `Array`
form (§6.3); the pre-Slice-1 behaviour is a process abort · 31. `(o.m)(...a)` — parenthesized callee
must bind `this` to `o` (§9 dec. 14) · **32. [1a] `class B extends A {}` must NOT observably call
`%Array.prototype%[Symbol.iterator]`** (ECMA-262 §15.7.14 step 14.a.iv.1 NOTE — the I-3 carve-out's
regression guard).

**Slice allocation of this matrix** (added R2 round 5): edges **21** (`return()` not called), **27/28**
(GC window) and **32** belong to **1a** — they are the assertions for its two semantic fixes (decs.
13a and 10) and its docstring carve-out. Everything else is **1b**'s. Round 5 flagged that calling 1a
"behaviour-preserving" while its only acceptance test was "the existing suite passes unchanged" left
both fixes unverified: no in-tree test asserts `.return()` behaviour on array-literal spread
(`tests_generator.rs:714/736` assert the *opposite*, for `yield*`/`for-of`), and none covers iterator
non-observability. **1a is therefore restated as: no call-shape change, plus two named semantic fixes
carrying their own edge rows.**

### §6.5 Non-goals

Tagged templates, class fields, super-property, private names — each its own slice.

---

## §7. Verification strategy

**§7.1 Per-slice unit tests** in `crates/script/elidex-js/src/vm/tests/`, one module per slice,
covering that slice's full edge matrix.

**§7.2 Permanent ES-language conformance table** — land the probe harness as
`vm/tests/tests_es_language_surface.rs`: a declarative `(source, expected)` table asserting current
truth across the language/builtin surface. This is the artifact that would have caught every gap in
§1.1 **and** this plan's own §1.2 over-claim. Rows for not-yet-implemented slices assert their
**known divergent** value with a `KNOWN-DIVERGENCE (#11-slug)` marker, flipping to the spec value in
the fixing slice — the `vm/tests/tests_dataset.rs:283` pattern. **Deliverable of Slice 0c**, so
every later slice inherits the baseline.

⚠ **The row-derivation rule needs a second source** (round 8). §9 dec. 9(b) sets "one row per §2.2
defect row", but §2.2 is **Layer-A compiler-emit only** — so the rule structurally produces *no* row
for the T3 absence surface (`Map`/`Set`/`WeakMap`/`WeakSet`, `Proxy`/`Reflect`, the `RegExp` ctor,
`Array.prototype.at`/`findLast`/`Object.hasOwn`/`String.matchAll`), which is roughly half of what
§1.1/§1.2 found. As written, 0c ships the program's safety net with a hole exactly where the probe
found most gaps. Second source: one row per **§1.1/§1.2 absence** finding + per Slice 7-10 slot. Note 0c reduces the number of divergence rows up front by
converting silent stubs to loud throws.

**§7.3 Standard gate** per slice: `cargo fmt --all` → `mise run ci` → `/pre-push` (6-stage) → push →
`/external-converge` (edge-dense ⇒ converge, per
[[feedback_gate-miss-on-edge-dense-escalate-to-converge]]).

**§7.4 No WPT dependency** — engine-independent language semantics covered by unit tests.

---

## §8. Slot ledger changes at landing

⚠ **This section was written before Slice 0a landed and is a *plan* for registration, not a record of
it. Re-derive both lists against the live ledger before acting** — at #489's landing 7 of the 11
below were already registered, so following the section as originally written would have minted
duplicates (`grep -c '#11-<slot>' <ledger>`; the ledger is the memory-dir
`project_open-defer-slots.md`, outside this repo, so **landing this document registers nothing**).

**Adopt (do not duplicate) 2 pre-existing slots** — this umbrella homes them:

- `#11-step9-class-extras` — scope "static members / private fields / getters & setters in class
  bodies / computed-name methods / static blocks / `Op::GetSuperProp` + `Op::SetSuperProp`" =
  exactly Slices 2/3/5. **Already in the SoT ledger** (adopted at #489's landing; the earlier claim
  that it lived only in `m4-12-pr-d17b-html-element-constructor-base-vm-landing.md` is superseded).
  Partially discharged already (static fields/methods and computed method keys verified working,
  §1.2). **Slices 2/3/5 retag every in-code citation `grep -rn 'step9-class-extras' crates/` reports
  at retag time** — 6 at `658cc302` (`compiler/expr.rs`, `compiler/expr_assign.rs`,
  `compiler/stmt_loop.rs`, `vm/interpreter.rs`, `vm/value.rs`, `bytecode/compiled.rs`), where this
  section originally froze a 4-site list measured 2026-07-26; 0a added two. Retagging a frozen list
  is what leaves the dangle the sentence exists to prevent.
- `#11-dead-opcode-removal` — "`Op::CreateClass` verifiably dead; bundle with D-26 Op-enum
  re-baseline"; trigger **already fired** at #458. Becomes Slice D, broadened to the §2.3 set.

**Also adopt** `#11-d17b-dispatch-expr-file-growth` (uncounted watch slot, D-17b r1/r2 landings;
homes the `vm/dispatch.rs` + `compiler/expr_class.rs` + `vm/interpreter.rs` + `vm/value.rs`
1000-line debt, re-eval 2026-08-08). ⚠ **The `dispatch.rs` facet is NOT discharged, and it is not a
no-action entry either** — §5's 1000-line check exempts the *match* from being split but explicitly
does **not** exempt the file from reduction; its discharge mechanism is the arm-body extraction rule,
so the slot's `dispatch.rs` facet is measured against `wc -l` at each slice that touches it (1103 at
`658cc302`, down from 1112). Its `interpreter.rs` / `value.rs` / `expr_class.rs` facets remain live. **Cold-gate recorded 2026-07-26** (⚠ *narrowed round 9 — the round-5 wording over-claimed; and the open-PR set below is that date's, since superseded — re-run `gh pr list` before relying on it*): **`vm/dispatch.rs` was clear** — verified no branch/worktree/PR touched it (PR #488 layout/ecs,
PR #487 shell, PR #486 dependabot manifest-only, `vm-input-value-as-date` plan-doc only, and
`domform-submittable-category` = 0 hits). But the wider claim "nothing touches
`elidex-js/{compiler,bytecode,vm}`" is **false**: the active L3 lane
(`elidex-wt-submittable`, `domform-submittable-category`) has committed changes to
`vm/object_kind.rs`, `vm/globals.rs`, `vm/mod.rs` and 6 `vm/host/` files. **`vm/object_kind.rs` is in
the module column of Slices 6/7/10** ⇒ cross-lane coordination required there, though not for 0a,
0b, 0c, 1a or 1b.

**Slots this umbrella owns** (11), each with the required triple. Registration state measured at
`658cc302` — **7 are already in the ledger**, recorded there as "Registered here at #489's landing":
`iteratorclose-precedence-convention`, `assignment-target-completeness`,
`topropertykey-symbol-from-toprimitive`, `operand-rooting-by-construction`, `internal-error-hard-exit`,
`delete-elem-raw-key-array-fast-path`, `statement-completion-updateempty`. **Still to register**:
`async-generators`, `es2021-2024-prototype-sweep`, `dynamic-import`. **To retire, not register**:
`computed-compound-assignment` — its stated purpose was "a ledger home until it lands", and it landed
as `658cc302` while never reaching the ledger, so the row below is the only place it has ever existed.

| Slot | Why deferred | Trigger | Re-eval |
|---|---|---|---|
| `#11-vm-iteratorclose-precedence-convention` | **(carved R2 round 4, enumeration corrected round 5)** the §7.4.11 error-precedence inversion spans **15** verified sites across `compiler/`, core `vm/` and `vm/host/` (**10** governed by ECMA-262 §7.4.11, **5** by WebIDL §3.2.21.1 — which has no IteratorClose step at all, so those need a different remedy), and the correct behaviour is completion-kind-dependent ⇒ `iter_close`'s signature must change. A cross-cutting convention sweep, not a slice deliverable | **now** — gates Slice 0b, whose [C39]→[C36] conformance claim inherits the inverted contract | 2026-09-30 |
| `#11-vm-computed-compound-assignment` | Slice 0a work item, not a defer — registered so the T0 crash has a ledger home until it lands | now (Slice 0a) | 2026-09-30 |
| `#11-vm-assignment-target-completeness` | Slice 0b work item; distinct SDO ([C39]) + owns the `IteratorClose` obligation Slice 1 does not, and the Paren-normalisation fix shares the same catch-all arms | now (Slice 0b) | 2026-09-30 |
| `#11-vm-async-generators` | needs a new async-iterator opcode + `compiler/stmt.rs:103` await-flag plumbing = own slice | now (Slice 6) | 2026-10-31 |
| `#11-vm-es2021-2024-prototype-sweep` | pure builtin surface, no language-layer dependency; batched to avoid N micro-PRs | Slice 9, or a WPT/site needing `at`/`findLast`/`hasOwn` | 2026-10-31 |
| `#11-vm-dynamic-import` | the T1 carve out of the ES-modules program: 0c makes `import()` loud immediately, the Promise-returning implementation needs the module loader | Slice M, or the ES-modules umbrella | 2026-10-31 |
| `#11-vm-topropertykey-symbol-from-toprimitive` | **(carved by #489's converge)** §7.1.20 tests the *argument* for Symbol-ness, not the ToPrimitive *result*, so an `@@toPrimitive`-returns-Symbol key throws. Open-coded **8 times** (2 named helpers + 6 inline) and `get_element`/`set_element` are **not** `make_property_key` callers ⇒ the unit is "collapse the 8, then fix", not a helper patch | the next property-key coercion PR, or with `#11-vm-proxy-reflect` | 2026-09-30 |
| `#11-vm-operand-rooting-by-construction` | **(carved by #489's converge; SUPERSEDES `#11-vm-element-access-base-rooting`, which must not be recorded as closed)** ~20 dispatch arms pop an operand into a Rust local and then run user JS before reading or storing through it; `gc/roots.rs` walks the VM stack but not Rust locals. Beyond the 5 element opcodes: `GetProp`/`SetProp`, `IncProp`/`DecProp`, `In`, `Add`, `Instanceof`, `TemplateConcat`, `ops.rs`'s three operator helpers, the three computed-key definition bodies, `SpreadObject`, `ArraySpread`, `IteratorRest`, the unary arms and `op_get_iterator` — two of them *panic*, two *store* the collected id. **Deliverable is an invariant making an unrooted hold unrepresentable, NOT a sixth sweep**: five successive audits each drew the boundary differently and each was falsified by the next round. `Op::GetElemRef`, the one arm #489 introduces, is rooted by construction and pinned. Implementation + a 14-arm test module preserved on branch `vm-p4-rooting-carved` | **now** — the next VM dispatch-loop PR, or Slice P. Edge-dense ⇒ plan-review MANDATORY | 2026-09-30 |
| `#11-vm-internal-error-hard-exit` | **(carved by #489's converge)** every extracted `op_*` helper's dispatch arm routes `VmErrorKind::InternalError` through `throw_error`, so a broken VM invariant becomes a catchable JS `Error` and user `try`/`catch` can swallow it; inline arms (`Op::Swap` / `Op::Pop` / `Op::PopUnder`) propagate with `?` instead. The disposition must key on the error's *kind*, not on whether the body was extracted. `Op::ThrowUnsupported` must stay catchable — it reports an unimplemented construct, not a broken invariant | with `#11-vm-operand-rooting-by-construction`, or the next dispatch-loop PR | 2026-09-30 |
| `#11-vm-delete-elem-raw-key-array-fast-path` | **(carved by #489's converge)** `Op::DeleteElem` derives its array-index fast path from the **raw** operand, so an object key that stringifies to an index skips it while the generic `try_delete_property` path never consults dense array storage: `var a=[1,2,3]; delete a[{toString(){return '0'}}]` reports `true` and leaves `a[0]` as `1`. Same root as `#11-vm-topropertykey-symbol-from-toprimitive` — a fast path keyed on the raw value rather than the `ToPropertyKey` result | with that slot | 2026-09-30 |
| `#11-vm-statement-completion-updateempty` | **(carved by #489's converge)** the half of the completion-ownership bug 0a did not fix: no `UpdateEmpty` (AO §6.2.4.4) equivalent, so `42; if (false) {}` yields `42` where §14.6.2 step 5 says `undefined`. Preserving the already-correct forms while adding the resets is ≥3 axes ⇒ edge-dense, plan-review MANDATORY | the next VM statement-lowering PR | 2026-09-30 |

Registering 11 exceeds
per-PR ≤3, so they register as **discovery carves of this umbrella** (the treatment the 2026-07-18
P4 registration used, `project_open-defer-slots.md` §"VM P4 ES-language + builtin gaps",
"**Origin**: not a carve — a **discovery**"), not slice-introduced defers. Five of them
(`#11-vm-topropertykey-symbol-from-toprimitive`, `#11-vm-statement-completion-updateempty`,
`#11-vm-delete-elem-raw-key-array-fast-path`, and the two rooting/error-routing slots' *pre-existing*
half) are additionally **gate-found pre-existing defects**, a category the ratified policy separates
from a slice's own deferrals. **#489's own count is 2**, not 0 — the operand-rooting and
internal-error slots are also its own *carves*, since it deliberately reverted work it had already
implemented (§18.1); ≤3 still holds. Net ledger delta is smaller than it looks: 2 adopted (not minted) and
`#11-vm-class-instance-field-init` / `#11-vm-super-property-reference` from R1 are **withdrawn** in
favour of `#11-step9-class-extras`.

**Memo corrections at landing** — `project_vm-p4-es-language-gaps.md`:
- §2 table: add super-property total loss, public instance fields → `undefined`, `obj[k] += v`
  panic, destructuring assignment no-op, `import()` → `undefined`.
- §2 scope note: retract the `new X(...args)` "works" claim.
- §4: async generators resolved **broken**; `new.target` resolved **working**.
- §line 113 + MEMORY.md: the 2026-07-18 probe is attributed to `f7d9b5ce`, whose commit date is
  **2026-07-26** — the baseline attribution is wrong and should name the then-current tip.
- SoT ledger `#11-vm-call-spread-arguments` reads "**First slice**", and MEMORY.md reads
  "slice 1 = call-spread". Slices 0a/0b/0c and P precede it (the standalone `dispatch.rs` split that
  an earlier round sequenced here was **withdrawn** — see §5). Both texts need "first *feature*
  slice" or an explicit reorder.
- Cite the discovery-carve precedent by **section heading** (§"VM P4 ES-language + builtin gaps"),
  not line number — the R1/R2 citation `project_open-defer-slots.md:139` is already off by 3
  (actual :142) because ledger line numbers drift with every registration.

---

## §9. Open decisions for plan-review

1. **Slice 0a/0b/0c admission.** All three surfaced during this re-probe / round-1 review and are
   not in the registered slot set. Recommendation: admit — 0a is a process abort, 0b a silent no-op
   on ubiquitous syntax, 0c the I-1 discharge that makes the rest of the program honest.
2. **Emit-site aggregation** (round 2 split this from decision 2b — they are orthogonal, and R2
   round 1 conflated them). I-3 says "no per-call-site spread branch", but `ArgsForm` unifies only
   the *decision*: the 7 emit sites still each hand-write
   `match form { Flat => emit(op, argc, ic), Array => emit(spread_op) }` — the very per-site emit
   branching whose divergence caused this bug. Options: (a) one `emit_call(fc, shape, args_form, ic)`
   owning op selection, so no site names an opcode; (b) keep the 7 branches. **Recommendation: (a)**
   — it is fully compatible with keeping `CallMethodSpread`, and it is what actually discharges I-3.
2b. **Opcode family shape.** Given (a), does the spread family stay four variants
   (`CallSpread`/`CallMethodSpread`/`NewSpread`/`SuperCallSpread`) or collapse to a sentinel `argc`
   on the existing opcodes? Round-2 review notes the variants' only difference is how many slots
   below the args array they read (0/1/2) — data the emit site knows statically — which is the
   lesson-#276 variant-vs-data shape, and that the sentinel is structurally safe (both IC accesses
   are bounds-checked, so an out-of-range sentinel reliably misses). Recommendation: **keep the four
   variants** (dispatch handlers genuinely differ in stack shape; a sentinel overloads a
   width-constrained operand with a mode bit) — but the burden of proof is now on the family, so
   ratify explicitly.
3. **Array-literal path sharing.** `expr.rs:105-119` emits the same sequence but handles elisions
   ([C22] ≠ [C19]). Recommendation: keep separate.
4. ~~Call-IC on spread calls~~ — **resolved** in §6.3 (monotonic counter; skip allocation).
5. **0c: throw vs `CompileError` — RESOLVED (runtime throw), ratified by evidence in Slice 0a.**
   A `CompileError` fails the whole script; a thrown `TypeError` scopes the failure to the
   expression, matching how the feature would fail if half-implemented. Slice 0a shipped the
   `CompileError` form first and the post-push gate showed the cost concretely: one
   `this.#x = 1` anywhere took **every unrelated statement in the file** down with it, making it
   *worse* than the silent-no-op it replaced for `=` and `+=`. So: **runtime throw** for
   unimplemented *expressions*, `CompileError` reserved for what the compiler already rejects
   (e.g. computed accessor keys) and for parse-time rejections (dec. 15).
   **Mechanism**: `Op::ThrowUnsupported` (u16 constant index → `TypeError` with that message),
   following `Op::CheckTdz`'s precedent of an engine-constructed error raised through
   `throw_error`, so the failure is catchable and local. Note this is a deliberate, user-visible
   behaviour change: code that today silently gets `undefined` will start throwing.
6. **Slice 1 breadth — RESOLVED (split adopted).** §3 is M=32 ⇒ preflight **SPLIT-DEFAULT** (escalated
   from SPLIT-RECOMMENDED as rounds 2-4 added step-level and `n/a` rows; scope unchanged), and
   round-2 review correctly noted the §3 appeal only refutes a split **by call shape**, leaving a
   **compiler/VM seam split** untested. Resolved against splitting, on the umbrella's own
   invariants:
   - **VM-seam-first (PR-A = VM, PR-B = compiler) violates I-4.** PR-A would land `CallSpread`,
     `NewSpread` and `CallMethodSpread` handlers with **zero compiler emit sites** — three dead
     opcodes, the exact "third state" I-4 forbids and the exact defect class §2.1 says this program
     exists to discharge. Converting `op_super_call_spread` to the shared helper gives *the helper* a
     live consumer but does nothing for the three handlers.
   - **Compiler-first is strictly worse** — it would emit opcodes whose handlers are still the
     `pop; pop; push undefined` stubs, i.e. ship a *new* silent-wrong path (I-1).
   - **Shape-split violates I-3** — half the call shapes migrated, half on the broken
     `compile_arguments`, which is the precise strangler shape that caused this bug (`super` correct,
     four callers broken).
   **✅ RESOLVED 2026-07-26 (user decision): adopt the 4th seam as a prereq PR.** Slice 1 splits into
   **1a (VM infrastructure — no call-shape change + two named semantic fixes, decs. 13a/10)** and **1b (compiler + opcodes + handlers)** — see
   §5. This answers SPLIT-DEFAULT by *actually splitting* rather than by overriding the gate at
   author altitude, and it halves a Slice-1 load that had grown across four review rounds. The
   history below is retained because it records why three earlier seams were rejected — those
   rejections still bind (in particular, splitting **by call shape** remains forbidden by I-3).

   ⚠ **Round 4 showed the earlier enumeration was incomplete.** Rounds 3 and 4 both identified a
   **fourth seam** that violates *none* of I-1/I-3/I-4: a **VM-infrastructure prereq PR** (no call-shape
   change; it carries two named semantic fixes, decs. 13a/10, each with its own edge row) containing `lay_out_call_args` + the `Empty` normalisation + converting
   `op_super_call_spread` to consume it + correcting its falsified docstring (the
   `expr_class.rs:145-152` producer is **spec-required** and is NOT folded — §6.3 / I-3 carve-out) +
   the dec.-11 `Option<usize>` refactor of `ic_call`/`ic_call_method` + the two semantic fixes decs. 13a and 10. It lands **zero
   opcodes** (no I-4 third state), adds **zero
   emit paths** (no I-1 silent-wrong), and leaves every call shape on the unchanged
   `compile_arguments` (no I-3 strangler). `op_super_call_spread` is a live consumer today, so the
   helper is not dead on arrival. Decision 6's earlier text conceded the seam exists and then
   dismissed it on "does nothing for the three handlers" — a *completeness* criterion, not the split
   criterion. *(This decision originally cited §5's acceptance of the `vm/dispatch.rs` prereq split as
   precedent for the same shape. §5 now withdraws that split, so the precedent is gone; the seam
   argument below stands on its own and is what the decision rests on.)*

   Aggravating: Slice 1's load has grown across four rounds to helper + `emit_call` aggregation
   (dec. 2) + new opcode + 3-4 handlers + `lay_out_call_args` + `Empty` normalisation +
   `op_super_call_spread` conversion + docstring fix + IC refactor (dec. 11) +
   `op_array_spread` `return()` removal (dec. 13a) + GC rooting (dec. 10) + stack-depth guard
   (dec. 12) + the arity-selection rework + `(o.m)()` regression guard (dec. 14).

   The lens therefore stopped converging, and per
   `.claude/skills/elidex-plan-review/SKILL.md` the SPLIT-DEFAULT verdict was escalated to the user,
   who **adopted the 1a/1b split** (recorded at the head of this decision).

   What holds regardless: splitting **by call shape** is forbidden (I-3), and each of 1a/1b remains a
   terminal unit under an approved umbrella (per-PR slices touching one subsystem do not re-trigger
   splitting, else infinite regress).
7. **`Function`/`eval` gate.** I-6 does *not* supply it (strict `eval` is core). The real question
   is a security/CSP policy decision. Recommendation: keep sequenced last; decide the policy in its
   own memo. R1 marked the row "decision first" but gave the decision no owner.
8. **`collect_array_like` sharing.** `vm/natives_function.rs:272` already unpacks an array to call
   args with the correct `Empty` normalisation, and is the natural home for Slice 10's
   `Reflect.apply`. Semantics differ (spread's array is compiler-guaranteed dense; `apply` is
   generic array-like per CreateListFromArrayLike). Share the dense fast path or keep separate with
   a stated reason? **Recommendation (added round 8 — this was the only §9 entry with none, and it
   sits inside 1a's deliverable): share.** `collect_array_like` already has the correct
   `.or_undefined()` normalisation 1a needs, `Reflect.apply` (Slice 10) will home there, and
   `call_internal` — one of dec. 10's four unrooted windows — is already reached at N≫1 through it,
   so sharing puts all of that on one audited path. The array-like/dense distinction stays a
   *caller-side* precondition, not a second helper.
9. **Slice 0c sweep method — RESOLVED (executed, not stipulated).** The three-pass sweep is now
   documented and **run** at the head of §2.2, and §2.2 is its output. It earned its keep
   immediately: it found 3 defects (`(x)++`, `(x)+=1`, `(a[0])++`) that the probe, R1, R2 and
   round-2 review had all missed, and it corrected a round-2 claim (`delete x` *is* parser-gated).
   **0c's acceptance criteria**: (a) re-run **all three passes** — pass 3 at **both** variant and sub-arm-body granularity — and classify every hit, so the arm list is reproducible rather than inherited; (b) §7.2 carries **one row per §2.2 defect row** as its
   row-derivation rule — the table's completeness claim is then anchored to the sweep rather than to
   authoring instinct.
10. **Generator/async callee GC window — root it in Slice 1a; carve withdrawn. ⚠ REMEDY CORRECTED
    (round 8): `gc_enabled` bracketing is WRONG for the async path.** Round 4 chose bracketing as
    "the only remedy-complete-by-construction option". Verified in round 8 that it cannot be applied
    at the site §6.3 named: `interpreter.rs:893` is not an allocation but
    `make_async_coroutine_and_drive` (`vm/natives_generator.rs:545-571`), which allocates the wrapper
    Promise + Generator, back-links at `:562-566`, **then calls `drive_async_coroutine` (`:569`),
    running the async body until its first `await`**. Bracketing that region would hold
    `gc_enabled = false` across arbitrary user JS on every async call — and the in-tree contract says
    so explicitly (`natives_generator.rs:489-490`: the async driver step "does not save/restore
    `gc_enabled` (user JS inside the resumed body needs GC to keep running)";
    `interpreter.rs:1246-1250` calls a persisted `false` a hazard that "would silently disable GC").
    **The real unrooted window ends at `natives_generator.rs:566`** — inside the callee, before the
    drive. So the remedy is either (a) push the bracket down into `make_async_coroutine_and_drive`
    so it closes before `:569`, or (b) **root** `stack_slice` + `actual_args` rather than suppress
    GC. (b) is preferred: it is remedy-complete for both vectors without any GC-suppression window.
    Edges 27/28 must assert **both** vectors and must not pass against a partial fix.

    **⚠ The window has 4 sibling sites, not 2** (round 8): `call_internal`
    (`vm/interpreter.rs:629-659` async, `:664-700` generator) has the identical
    `stack_slice = self.stack.drain(base..).collect()` + `actual_args: Some(args.to_vec())` shape,
    and is **already reached at N≫1 today** via `Function.prototype.apply` → `collect_array_like`
    (uncapped for a real Array) — the very function dec. 8 proposes to share and Slice 10's
    `Reflect.apply` will home there. dec. 10's own justification (a documented-contract violation
    does not qualify for deferral) applies verbatim. Enumerate by command
    (`grep -n "drain(base\.\.)"` / `SuspendedFrame {`) and root all four windows in one unit.

11. **IC entry point** (§6.3 IC): `Option<usize>` refactor of `ic_call`/`ic_call_method`
    (recommended, one SoT) vs. duplicating the three-way body. ⚠ **Round-8 caveat**: both callers
    (`dispatch.rs:719/730`) pass `Some`, so the `None` arm has **zero live producers until 1b** — a
    dead branch in 1a, which is the same I-4 "third state" argument dec. 6 used to reject VM-first,
    one layer down. Either move dec. 11 to **1b** (where `CallSpread` is its producer), or give 1a a
    direct unit test on `ic_call(.., None)` so the branch is exercised where it lands.
12. **Stack-depth guard — RESOLVED (a limit is required, and it is 1b's).** Verified 2026-07-26:
    `grep -rn "MAX_STACK\|stack_limit\|call_depth\|MAX_FRAMES" crates/script/elidex-js/src/vm/` →
    **0 hits** — the crate has *no* stack or frame bound at all. `spread_iter_loop` caps the
    *array* at `DENSE_ARRAY_LEN_LIMIT` (1<<27, `vm/ops.rs:24`) but nothing caps the `vm.stack`
    push side, so once §6.3 routes >255 args through the Array path, `f(...arrOf10M)` becomes an
    unbounded user-driven push — **reintroducing a T0 in the fix for a T0**. Slice 1b must add a
    bound (and edge 29 must assert it, with an expected value). This is a **1b blocker**, not a
    carry-over.
13. **§6.2a — the reused drain's `IteratorClose` divergence — RESOLVED as two units.** ⚠ **Round-8
    sequencing hazard**: §5 lands **0b before 1a**, and 0b owns [C39], whose rest form
    (`[a, ...rest] = it`) needs a drain-into-array and whose spec *requires* `IteratorClose` —
    while `op_array_spread`/`spread_iter_loop` is the only in-tree drain-into-array and 1a **deletes**
    its `return()`. So either 0b routes through it (and 1a silently falsifies 0b's [C39] claim) or 0b
    hand-rolls a second drain (the I-3 failure). State in §5/§6.2a that after 1a `op_array_spread` is
    **[C19]/[C22]-only**, give 0b's rest path its own explicit `iter_close` site, and pin it with a
    §7.2 row. (a) Removing
    `op_array_spread`'s `return()` call is **1a's** — it is the reuse precondition and edge 21
    fails without it. (b) The **error-precedence inversion is a separate, crate-wide unit** (§6.2a-2:
    **15** sites (§6.2a-2) incl. the canonical `iter_close` helper, whose docstring states the wrong rule as its
    contract; the correct behaviour is completion-kind-dependent so the signature must change) →
    carve **`#11-vm-iteratorclose-precedence-convention`**, sequenced **before Slice 0b**, whose
    [C39]→[C36] conformance claim otherwise inherits the inverted contract.
14. **`(o.m)(...)` `this`-loss — owner needed.** `compile_call_expr` (`expr_member.rs:92`) matches
    `ExprKind::Member` on the **raw** callee, and `ExprKind::Paren` is a real AST node
    (`ast.rs:299`), so `(o.m)()` takes the plain-call branch and loses `this` — a live T1
    silent-wrong sharing 0b's non-normalised-Paren root, inside the very function 1b rewrites.
    Coupling: §5 puts `parser/expr.rs` Paren normalisation in **0b's** scope, so if 0b normalises
    globally this is fixed as an **unrecorded side effect** of a slice scoped to assign/update
    targets; if 0b scopes narrowly, 1b ships the bug intact. Recommendation: one `peel_paren`
    chokepoint owned by **0b**, with `(o.m)(...)` added as an explicit 0b deliverable + §2.2 row, and
    edge 31 as 1b's regression guard.
15. **`expr.rs:186-189` prefix `Spread` — layer mismatch.** §2.2 assigns it to 0c, whose rule (§9
    dec. 5) is *runtime throw*; but `var y = ...x` is an **early SyntaxError** per spec, so a runtime
    throw lets preceding side effects run and lets `if (false) { var y = ...x }` execute the whole
    script. Root is parser-layer: `parser/expr.rs:256-263`'s `Ellipsis` arm is **ungated** (its own
    comment says "in arguments/array context" but no context check exists). Recommendation: reassign
    to **0b** (which already owns `parser/expr.rs`) as a parse-time rejection, not 0c.

---

## §10. Round-2 review disposition

`/elidex-plan-review` round 2 returned **2 CRIT / 18 IMP / 22 MIN** (round 1's 2 CRIT / 11 IMP all
verified resolved). Both round-2 CRITs and the structural IMPs are applied above. Independently
re-verified by the author before applying:

- `IteratorClose` count in [C19] = **0**, in [C39] = **6** ⇒ CRIT confirmed; [C36] moved to Slice 0b.
- `[C37]`/`[C38]` anchors were **fabricated** (plausible-shaped guesses) despite §0.5's blanket
  "verified" header. Real anchors are `#sec-optional-chaining-evaluation` /
  `#sec-optional-chaining-chain-evaluation`. **Note for future revisions**: `preflight.py` verifies
  §3 §-number↔title only — it does **not** check §0.5 anchors, step ranges, step attribution, or
  production completeness. Those need manual `webref body`/`heading` calls.
- ~~Nine~~ zero-emit opcodes re-confirmed by independent grep. **(Stale: round 3 re-derived this mechanically as 18 — see §2.3. Retained as the round-2 record.)**

**Not yet applied — carried to round 3** (MIN tier, plus the two genuinely open questions):
§2.2 exhaustive-derivation rewrite (decision 9 gates it), the phase4-plan P4 item-by-item
disposition table (TypedArray/DataView, RegExp named-groups/lookbehind, `Object.fromEntries`
iterator protocol, `replaceAll` non-global, and the `String.matchAll` double-homing between
`#11-vm-regexp-constructor-and-flags` and Slice 9), per-file 1000-line verdicts for
`vm/object_kind.rs` (1700) / `vm/gc/collect.rs` (2074) / `vm/gc/trace.rs` (1255) /
`vm/interpreter.rs` (1366) / `vm/value.rs` (1187), Slice M's promotion to its own umbrella, Slice 9
× the open `natives_string.rs` §21.1.3→§22.1.3 citation-drift unit, the `delete` arms
(`expr_ops.rs:145-160` — ECMA-262 §13.5.1.1 makes both an early SyntaxError in strict code),
`vm/value.rs:1049-1053`'s eval contract retag, and assorted line-cite off-by-ones
(`expr_assign.rs` :211-213 / :216-221; `#x = v` does have an emit arm at :203-207).

## §11. Round-3 review disposition

Round 3 (Axes 2/3/4) returned **2 CRIT / 9 IMP / 11 MIN**. Round-2's CRITs and most IMPs verified
resolved; agents independently reproduced the §2.2 sweep counts (31 / 24) and all three new defects.

**Both round-3 CRITs are one root, now fixed**: §6.3's Unwind paragraph still mandated
`IteratorClose` after [C36] had been moved to Slice 0b in four other sites — an incomplete
propagation of my own edit ([[feedback_semantic-sibling-selfseed-and-regate-breadth]]: change the
*concept*, grep the concept, fix all siblings in one commit). Axis 2's sharper form of it uncovered a
**genuine pre-existing bug** now written up as §6.2a (`op_array_spread` calls `return()` where no
spec algorithm does, with inverted error precedence) — a finding this program would otherwise have
inherited silently.

**Also applied**: §2.3 rebuilt mechanically (18 zero-emit, not 9; `GetModuleVar` was wrongly listed
as dead and IS emitted at `expr_assign.rs:52` — Slice D would have broken module reads).

**Round-4 carry-over (verified, not yet applied)** — none are blocking, all are recorded so nothing
is lost:
- **Sweep blind spot (IMP ×2)**: classes 2/3 are detected only via pass-1 *marker comments*, so an
  arm that silently skips **without** a comment is invisible. Two confirmed misses:
  `expr_class.rs:430-447` (`ClassMemberKind::PrivateField` compiled only under `if *is_static`, no
  else → `class A{#x=1}` emits nothing) and `expr.rs:186-189` (`ExprKind::Spread` in prefix position
  → compiles the operand; every such node is an early-SyntaxError position per spec). Fix: add a
  **structural** third pass (arms that neither emit nor return `CompileError` for a user-writable
  production) instead of a lexical one.
- **`(o.m)(...)` loses `this`** (Axis 2 IMP-2) — same non-normalised `ExprKind::Paren` root as the
  0b defects, but on the *call* path, inside the very function Slice 1 rewrites; currently ownerless.
  Suggests one `peel_paren` chokepoint so assign-target / update-target / call-callee all read the
  same normalised node.
- **`f(a1…a300)` (no spread) would be permanently rejected** — `ArgsForm::Flat(u8)` caps at 255 and
  the flat-vs-array choice keys on spread-presence, not arity. Fix: arity ≥256 selects `Array`, so
  the operand-width invariant holds by construction rather than by rejecting legal ES.
- **I-3 tagged-template clause recorded, not discharged** — decide the helper's *input* shape in
  Slice 1 (fixed-prefix count or item iterator) so Slice 4 has a live consumer path.
- **§9 dec. 10 understated** — the generator/async window is a live defect at N=1, not merely a
  widened risk; three in-tree remedies exist (`gc_enabled` bracketing, rooted-copy-then-truncate
  `inner.rs:172-180`, `push_stack_scope` `temp_root.rs:155`) and the decision should pick one.
- **Spec MIN**: §3 breadth arithmetic stale (composition sums to 23, actual M=27; [C20] is 7 rows and
  [C37]/[C38] is 3 — the appeal is "21 of 27 … 6 n/a", not "19 of 23"); GetIterator "kind = sync"
  belongs to steps 1-2 not step 3; ArrayAccumulation Step/Branch cells inverted; [C20] step 6 and
  [C33] steps 1-3 lack n/a rows.
- **Inventory MIN**: I-1's class-2 list is a strict subset of §2.2's; "wrong arity" is a fourth
  substitution class I-1 does not name; `expr_ops.rs:226` row mislabels update as `delete`;
  `expr_object.rs:123` is an internal `unreachable!`, not an ISA-bound carve-out member; line-cites
  off by one (`expr_assign.rs` :210-212 / :215-220 / :202-206; `expr_class.rs:145-152`).

## §13. Round-5 disposition

Round 5 (Axes 1/3/4, full breadth for the first time since round 2) returned **2 CRIT / 14 IMP /
19 MIN**. Applied:

- **CRIT (Axis 1) — a spec error, not a hygiene issue.** §6.3 told Slice 1a to fold the hand-rolled
  `expr_class.rs:145-152` `SuperCallSpread` producer. ECMA-262 **§15.7.14 step 14.a.iv.1 NOTE** requires
  the default derived constructor *not* to observably call `%Array.prototype%[@@iterator]`, which the
  shared `ArraySpread` path would. **The in-tree hand-rolled producer is the conformant
  implementation**; rounds 3-4 mis-diagnosed a spec-mandated divergence as an I-3 violation. Now an
  explicit I-3 carve-out; 1a keeps only the docstring correction; edge 32 guards it.
- **CRIT (Axis 4) — the precedence enumeration was wrong for the third time** (14 lines → 5 sites → ~15 →
  **14 verified in round 6**). Re-derived **mechanically** (grep patterns + counts cached inline), spanning `compiler/`,
  core `vm/` and `vm/host/`, and including `compiler/stmt.rs:175` — the `for-of` catch handler, the
  most reachable IteratorClose path in the language. Also corrected the completion-kind claim: step 5
  fires on **throw only**, so *normal* completions reach steps 6-7 — which matters because [C39]
  passes normal completions at 5 of its 6 call sites.
- **Slot P registered** — the carved unit existed in prose only (no §5 row, no §8 entry) while §12
  called 0b "near-ready" against it. Now a §5 row + §8 triple; new-slot count 5 → 6.
- **1a's acceptance restated** — "existing suite passes unchanged" was vacuous for its two semantic
  fixes; edges 21/27/28/32 are now explicitly 1a's.
- **dec. 9 / §2.2 propagated** two-pass → **three-pass, both granularities**.
- **[C36] `Used by`** corrected — decs. 13a/13b gave it two more owners after "Slice 0b only" was written.

**The pattern, stated plainly**: rounds 3, 4 and 5 each caught a *hand-curated enumeration* that was
too small — the IteratorClose mandate (4/5), the precedence rule (1/5, then 5/15), "nine"→18 (3
sites). Every enumeration I derived **mechanically** (§2.2's sweep, §2.3's 125-variant walk, this
round's grep-with-counts) has held up; every one I curated by inspection has not. That is the durable
rule, now written into §6.2a-2 and dec. 9: **enumerate by command, cache the count inline, never by
recall.**

## §12. Round-4 disposition + convergence state

Round 4 (Axes 2/3) returned **1 CRIT / 12 IMP / 8 MIN**. Applied:

- **CRIT** — §6.2a had scoped the `IteratorClose` **error-precedence** inversion to "the same 14
  lines". It is crate-wide across **5** sites, including the canonical `iter_close` helper whose
  *docstring states the inverted rule as its contract*. Slice 0b calls `iter_close`, so its
  [C39]→[C36] conformance claim would have been false. → §6.2a-2 + carved
  `#11-vm-iteratorclose-precedence-convention`, sequenced **before 0b**.
- **"nine" → 18** propagated (3 residual sites) after §2.3's round-3 rebuild; `StmtKind` 23 → 24.
- **dec. 10 resolved** — `gc_enabled` bracketing, because `actual_args` (~1000 values) is the
  *larger* unrooted vector and the other two remedies root only what they are handed. *(Round-4
  record. **The bracketing remedy was overturned in round 8** — it would disable GC across user JS on
  the async path; see §9 dec. 10.)*
- **dec. 13 split** into 13a (Slice 1a) / 13b (the crate-wide precedence unit).
- **decs. 14/15 added** — `(o.m)()` `this`-loss owner; prefix `Spread` is parser-layer so 0c's
  runtime-throw rule is wrong for it.
- **Arity now selects the form** (`Array` when spread **or** count > 255), so `f(a1…a300)` compiles
  instead of being permanently rejected; the `assert!` is deleted, not converted.
- **§9 dec. 6 resolved by the user**: adopt the 4th seam → Slices **1a** / **1b**.

**Twice in a row a concept I changed had un-swept siblings** — round 3 the IteratorClose *mandate*
(4 of 5 sites), round 4 the *precedence* rule (1 of 5) and "nine"→18 (3 sites). Both are
[[feedback_semantic-sibling-selfseed-and-regate-breadth]]. The durable lesson, now written into
§6.2a-2: a concept discovered *mid-edit* needs its own concept-grep, not the scope it was found in.

**Convergence state.** R1 2C/11I → R2 2C/18I → R3 2C/9I → R4 1C/12I. CRITs are falling and each
round's findings are more local, but round 4's IMP count did not drop — largely because it surfaced
consequences of round-3's own fixes (the precedence sweep, the layer mismatch on prefix `Spread`).
**Not converged; a round 5 is warranted**, now against a materially different plan (Slice 1 split in
two, four decisions closed). *(Round-4 record; superseded by §14.)*

**Implementation-ready now**: **Slice 0a** (single verified root site `expr_assign.rs:170`, no open
finding touches it) — modulo dec. 1's formal admission. *(Earlier rounds also gated 0a on a
`vm/dispatch.rs` prereq split; that split was removed — §5.)* **Slice 0b** is gated on **Slice P** (its [C39]→[C36] conformance claim
inherits the inverted contract otherwise) plus three couplings its own plan-review must close (the `peel_paren` chokepoint it shares with dec. 14; its dependence on the sub-arm sweep that
0c owns but is sequenced *after* it; and whether it connects the four destructuring opcodes or
leaves them to Slice D).


## §14. Round-6 + round-7 disposition, and the convergence series

**Round 6** (Axes 1/3): **0 CRIT** — the first clean round — / 7 IMP / 15 MIN. Its diagnosis was that
*every IMP but one was a **sibling site** of a round-5 fix applied only where the reviewer pointed*.
Remedy applied: seven concepts swept document-wide (precedence count, `iter_close` SoT line, breadth
numbers, 1a's contract label, prefix-`Spread` owner, stale "Slice 1", `structured_clone` layering).

**Round 7** (Axes 3/4): **0 CRIT** again / 9 IMP / 16 MIN. Four were **spec-path errors**, all now
fixed and re-verified:

| Claim | Was | Is |
|---|---|---|
| default-derived-ctor NOTE | §15.7.14 **step 4.a** (4 sites — the round-7 sweep grepped the exact string `§15.7.14 step 4.a` and so missed `§15.7.14 ClassDefinitionEvaluation step 4.a`) | **step 14.a.iv.1** (step 4 is `outerPrivateEnv`; no sub-step a) |
| StructuredSerialize | WHATWG HTML **§2.9** | **§2.7.4** (§2.7.7 for the transfer path); the *in-code* docstring is drifted too |
| precedence sites | 14 | **15** — `op_array_spread` re-implements IteratorClose **inline** and is in neither grep |
| governing algorithm | "pure ECMA-262" for all | **5 sites are WebIDL §3.2.21.1**, which has *zero* IteratorClose steps ⇒ precedence is the wrong question there |

Round 7's meta-finding is the sharper version of round 6's: the sweep was *still* site-directed —
every residual was findable by a single `grep -n` on the swept term, and **3 of 5 IMPs sat in §4
(invariants) and §9 (decisions)**, the two sections carrying the operative contracts and the two no
finding had ever pointed at. Round 8's pass therefore ran `grep -c` per term **before** editing and
swept §4/§9 wholesale.

**The compounding lesson**, now three layers deep: enumerate by command (round 5) → sweep the
concept, not the site (round 6) → **run the grep before editing, classify every hit, and never
filter by expectation** (round 7 — the 15th IteratorClose site was lost precisely because 17 concept
hits were dismissed as "incl. unrelated" without classification).

**Convergence series**: R1 2C/11I → R2 2C/18I → R3 2C/9I → R4 1C/12I → R5 2C/14I → **R6 0C/7I** →
**R7 0C/9I**. Two consecutive CRIT-free rounds, and round 7 reported that *every mechanically derived
enumeration verified clean*. What has not converged is the *editing* discipline, not the content.

**Readiness**: **Slice 0a** is ready (modulo dec. 1's formal admission). *(The `vm/dispatch.rs`
prereq split is removed — §5.)* **P** needed its own SoT count fixed (done) and must now split its 15 sites by
governing algorithm. 0b is gated on P; 0c on the I-1 carve-out (applied); 1a on dec. 6's contract
wording (applied); 1b is well-specified behind 1a with dec. 12's stack bound as its named blocker.


## §15. Convergence call — stop the umbrella loop, ship

Nine `/elidex-plan-review` rounds. The series: R1 2C/11I → R2 2C/18I → R3 2C/9I → R4 1C/12I →
R5 2C/14I → R6 0C/7I → R7 0C/9I → R8 1C/13I → R9 1C/10I.

**`.claude/skills/elidex-plan-review/SKILL.md` sets the stopping criterion**: *"convergence =
findings moving from 'open design tension' to 'fixed concrete mechanism' … further passes are
impl-detail the tests catch."* That criterion is met. Every remaining finding is one of:

1. **A sibling site of a fix already applied elsewhere** (rounds 6-9 all diagnosed this shape) —
   document hygiene, caught mechanically by the next `grep -c`, not a design question.
2. **A per-slice implementation detail** — and **per-slice `/elidex-plan-review` is mandatory**
   (§5), so every one of these is re-derived at slice time against real code rather than prose.

**The cost of continuing is now concrete**: the T0 process abort (`obj[k] += v`,
`compiler/expr_assign.rs:170`) has been declared implementation-ready since round 4 and has stayed
live on `main` for five further rounds while the umbrella was polished. That inverts §2.4's own
severity ordering — [[feedback_ship-first-over-close]] and [[feedback_cap-vs-completeness]] both cut
against it.

**What the loop actually bought** (worth recording — it was not waste): 2 spec-conformance errors
that would have shipped (the §15.7.14 default-derived-ctor iterator carve-out; the ~15-site
`IteratorClose` precedence inversion, incl. the inline 15th site), 1 GC-safety design error
overturned (`gc_enabled` bracketing would have disabled GC across user JS on every async call),
3 previously-unknown silent no-ops found by the executed sweep (`(x)++`, `(x)+=1`, `(a[0])++`),
the Slice-1 split, and a corrected dead-opcode set that would otherwise have deleted a live opcode.

**Decision**: the umbrella is converged **for its purpose** — it is a decomposition and invariant
document, not an implementation spec. Remaining work moves into the per-slice reviews.

**Ship order**: **Slice 0a** → P → 0b → 0c → 1a → 1b. *(The `vm/dispatch.rs` prereq split that rounds 2-9 mandated was **removed at implementation time** — §5's 1000-line check, whose figures were then re-derived at PR-B: the match is a cohesive flat case table and 20 arms use inline loop control flow, so the **match** is not relocated; the **file's** size debt is discharged continuously by the arm-body extraction rule instead. Nine review rounds carried the standalone-split mandate.)*
Each carries its own `/elidex-plan-review`, at which point that slice's residual §9 decisions,
module columns and edge rows are settled against code rather than against this document.


## §16. Slice 0a — implemented and merged (`658cc302`)

**Scope was 3× the plan's record.** §2.2 listed one T0 (`obj[k] += v`, the `assert!`). Implementing
it surfaced two more in the same two files, all probe-confirmed as **process aborts on valid JS**:

| Spelling | Panic site |
|---|---|
| `obj[k] += v` | `compiler/expr_assign.rs:170` `assert!` |
| `obj.p \|\|= v` · `&&=` · `??=` | `compiler/expr_ops.rs:29` `unreachable!` |
| `obj[k] \|\|= v` | both |

Short-circuit handling existed **only** for the identifier target, so every logical assignment to a
member fell through to `compound_op_to_opcode`. `opts.foo ??= default` — idiomatic modern JS —
aborted the process. Same concept, same files ⇒ landed together rather than leaving siblings, per
§14's rule.

**Design** ([C43] ECMA-262 §13.15.2): both **read-modify-write** productions evaluate the LHS
reference **once** and reuse it for `GetValue` and `PutValue` (steps 1/3/9 for the compound
production, 1/2/6 for the logical ones — the indices are per-production). Simple `=` is *not* one of
them: it evaluates `leftRef` at step 1.a and `PutValue`s at step 1.e with **no** `GetValue`, so it
needs no reference kept across a load — which is why only the other two reach `Op::GetElemRef`. One `emit_logical_assign_tail` serves both member
shapes via a `LogicalStore` enum; it `Dup`s only for the **popping** jumps
(`JumpIfFalse`/`JumpIfTrue`), not for `JumpIfNotNullish` which peeks, so both paths leave exactly one
value, and `Op::PopUnder` drops the reference slots on the short-circuit path in one instruction.

⚠ **An earlier draft of this section described `Op::Dup2` (`[a b -- a b a b]`). That opcode never
shipped** — it was the CRIT-2 defect below, not the design. The landed reference primitive is
**`Op::GetElemRef`** (`[object key -- object key' value]`), which returns the *converted* key so the
following store cannot re-run `ToPropertyKey`. The slice also split `Op::Pop` into a pure discard
plus a recording `Op::PopCompletion`, because the short-circuit cleanup exposed that `Op::Pop` was
writing the script completion value from all 88 of its emit sites; see §18.

**Verification** (measured at the merge commit `658cc302`): **30** tests across
`tests_member_compound_assign.rs` (27) and `tests_member_compound_assign_gc.rs` (3) — all three
forms, both short-circuit paths, expression values, `0`-vs-nullish for `??=`, accessor call order
(`"gs"` on assign vs `"g"` alone on short-circuit), RHS-not-evaluated-on-short-circuit,
key-evaluated-once, stack balance after every form, the admissibility gate across all **four** of its
lowerings (assignment / update / for-in/of head / imported binding), `Op::GetElemRef`'s rooting, plus
the guards and pinned divergences §18 lists. Full crate **6473 passed / 0 failed**
(`cargo nextest run -p elidex-js --all-features`), workspace **12785** (`mise run ci`); clippy + fmt
clean. Both totals are for the merge commit, so they include the `origin/main` commits taken in
before landing — re-run the two commands rather than reading either number forward.

The two files are a touch-time split, taken proactively at 730 lines rather than after a reviewer
named the number (which is how `compiler/stmt.rs` went — see §18.2). The seam is the GC-window
cluster: every test in `_gc.rs` places a collection with the `force_gc_before_next_alloc` one-shot
and asserts what survives it, and they share a `diverged_from` helper the behavioural cases do not
use.

**Method note**: the plan's inventory was Layer-A-complete but *severity*-incomplete — the sweep
found the arms, but only implementation revealed that two of them abort rather than misbehave. That
is the §15 argument restated: shipping finds what reviewing does not.


## §17. Slice 0a — pre-push `/elidex-review` findings (2 CRIT, all fixed)

The 5-agent gate caught two CRITs in my own fix, both probe-confirmed:

**CRIT 1 — the panic class was closed for one `MemberProp` variant, not the enum.** The new guard was
`if let (Some(jump_op), MemberProp::Identifier(name))`; `MemberProp::PrivateIdentifier` fell through
to `compound_op_to_opcode`'s `unreachable!`, so **`this.#x ??= 1` still aborted the process** — the
standard lazy-private-field idiom, and exactly the shape §16 claimed to have killed. Three agents
found it independently. **This is the failure mode this plan documents (§13/§14: sweep the concept,
enumerate by command) committed inside the fix for that failure mode** — §2.2's own pass-3 mandate
names `MemberProp` as an enum needing sub-arm enumeration, but assigns it to 0c, so 0a's ad-hoc
sibling check ran without it. Fixed as a loud `CompileError` (no `Op::SetPrivate` emit path exists
until Slice 5, so emitting the store would silently lose the write — banned by I-1).

⚠ **That remedy was itself overturned post-push (§18): `CompileError` is loud but NOT scoped.** It
yields no bytecode for the whole script, so one `this.#x = 1` anywhere took every unrelated
statement down with it — *worse* than the pre-slice behaviour for `=` and `+=`, which at least let
the rest of the script run. The shipped remedy is `Op::ThrowUnsupported`, per §9 decision 5.

**CRIT 2 — `Op::Dup2` prevents operand re-evaluation but not key *conversion*.** It duplicates the
**raw** key, so `GetElem` and `SetElem` each run their own `ToPropertyKey`. ECMA-262 §6.2.5.5
GetValue step 3.c *memoizes* the converted key into the Reference Record, so the spec runs user
`toString` **once**. Measured: `o[k]+=2` with a stateful key → 2 conversions (spec 1), and
`o[k]+=5` with `toString` returning `'p'+n` **reads `p1` and writes `p2`**. The docstring and §16
claimed the opposite, and the existing `computed_compound_evaluates_key_once` test only covered the
key *expression* — the half that works — so the overclaim shipped green. The docstring, §16 and the
test suite were corrected, and the divergence was initially pinned and carved as
`#11-vm-element-ref-single-key-conversion`.

**That carve is CLOSED — the divergence was FIXED at `7cca56a9`,** not deferred: Codex R1 argued the
converted-key representation was the cheaper fix, and it was. `Op::Dup2` was replaced by
`Op::GetElemRef`, which returns the converted key so `GetElem` and `SetElem` share one
`ToPropertyKey` — the §6.2.5.5 step 3.c.i memoization expressed in the opcode's own signature rather
than defended by a comment. `Op::IncElem`/`DecElem` were routed through the same helper in the same
commit, so the pre-existing half closed with it. The pinning test is now
`computed_compound_converts_key_once`, asserting **one** conversion.

**IMP — identifier `??=` leaked a stack slot.** `JumpIfNotNullish` peeks where
`JumpIfFalse`/`JumpIfTrue` pop, but the identifier path `Dup`ed unconditionally. Observable, not just
growth: `f(x ??= 1)` read the stray as the **callee** → TypeError. The new
`emit_logical_assign_tail` derived exactly this rule 100 lines below and it was not carried up.
Fixed and pinned by `identifier_nullish_assign_is_stack_balanced`.

**Also corrected**: `PutValue` is §13.15.2 **step 9**, not step 8 (step 8 is
`ApplyStringOrNumericBinaryOperator`) — replicated at 4 sites, and the pair (GetValue 3 / PutValue 8)
matches no edition, so the two numbers were not derived from one lookup.

**Method note**: every CRIT here is a *sibling-enumeration* miss, the same class as rounds 3-9. The
difference is that this time the gate caught it before push rather than a later round catching it in
prose. That is the argument for running the full 5-agent gate on even a "small" fix — the blast
radius of a compiler codegen change is not proportional to its diff size.


## §18. Slice 0a — post-push `/external-converge` + the design re-gate it triggered

PR [#489](https://github.com/send/elidex/pull/489). Codex R1 (3 findings) and R2 (4 findings), all
real, zero FP. R2 also tripped the loop's **≥2-round self-root-check**, so the cumulative fix-delta
`/elidex-review` ran *then* rather than at TERMINAL — **1 CRIT / 15 IMP / 11 MIN**.

| Round | Commit | CRIT | IMP | MIN | New-real |
|---|---|---|---|---|---|
| R1 | `b9d2f603` | 0 | 2 | 1 | 3 |
| R2 | `7cca56a9` | 0 | 3 | 1 | 4 |
| R3 | `32332ed7` | 0 | 2 | 0 | 2 (+1 FP) |
| R4 | `3129ebe9` | 0 | 1 | 0 | 1 |
| R5 / R6 | `0c4dfaeb` | 0 | 0 | 0 | 0 (dry ×2) |
| TERMINAL fix-delta `/elidex-review` | `32332ed7..0c4dfaeb` | 0 | 12 | 16 | — |
| R7 | `73d148c6` | 0 | 3 | 0 | 2 (+1 already-fixed) — the classification spans R7+R8 |
| R8 | `37a97d49` | ↑ | ↑ | ↑ | ↑ |
| **carve** | `612f167a` | — | — | — | — |
| carve-delta `/elidex-review` | `612f167a` | 0 | 8 | 7 | — |

⚠ **This table was written from the fix history, and it does not line up row-for-row with what Codex
actually posted.** Enumerated at landing, the pre-carve series is eight assessments, each carrying
its findings as inline threads: `b9d2f603` 3, `7cca56a9` 4, `32332ed7` 3, `3129ebe9` 2, `0c4dfaeb`
dry, `0c4dfaeb` dry, `73d148c6` 1, `37a97d49` 2. Two rows disagree with that. **`73d148c6` was
missing entirely** — its one finding had been folded into the row below, so the table showed seven
assessments where there were eight; the two rows are now split, with the 3-finding classification
marked as spanning both. And **`3129ebe9` shows one finding against two threads**. The second is left
as measured rather than reconciled: the classified counts are dispositions (an FP, an
already-fixed duplicate and a split finding all break the 1:1), and choosing which reading to write
down from memory is the failure this document keeps recording. Re-run the enumeration in §18.2.

Two dry rounds are visible at `0c4dfaeb`, and proposing merge off them would have shipped all 12 IMP
the TERMINAL gate then found — including a **second fabricated §-number** (`§12.5.3.2`; the `delete`
operator is §13.5.1.2, `ToObject` at step 4.c, the strict throw at step 4.f) *re-shipped by R3 while
moving the code*. Real-gap exhaustion in the reviewer is not the same as design convergence.

### The two findings that overturned an author judgment

**A carve justification that was factually false.** The GC-rooting divergence was carved as
"inherited from the read path, not introduced" — true for `Op::GetElem`/`IncElem`, and **false for
`Op::GetElemRef`**, which hands its base to the following `SetElem`. A base collected during key
conversion is therefore a **store through a dangling `ObjectId`** — a write into whatever object
recycled the slot, or `get_object_mut`'s "object already freed" panic — not the merely-wrong-value
outcome the read paths have. Fixed in place: the opcode reads the `[object key]` pair without
popping, so the GC's stack walk roots it **by construction**.

⚠ *The "read-side family stays carved" clause that stood here is FALSIFIED* — R3 showed the carve
**boundary**, not just its enumeration, was wrong: `IncElem`/`DecElem` call `set_element`, and of the
five element opcodes only `GetElem` is genuinely read-only. See §18.1.

**The CRIT was a fix from the previous round** — see §17's addendum: `CompileError` is loud but not
scoped. This is the second time in one PR that a remedy applied to a real finding was itself the
next round's defect, which is the argument for the fix-delta re-gate existing at all.

### Enumerations that were wrong because they were curated, not derived

Both carved slots' blast radii were re-derived by command and both were wrong:

- **`#11-vm-topropertykey-symbol-from-toprimitive`** said "fix the shared helper". There is no
  shared helper: §7.1.20 is **open-coded 8 times** — two named (`VmInner::make_property_key`,
  `natives_object::to_property_key`) and six inline. Critically `get_element`/`set_element` — the
  plain `o[k]` read and write — are **not** `make_property_key` callers, so "fix the helper" would
  have left them diverging while the slot read as closed. The unit is *collapse the 8, then fix*.
- **`#11-vm-element-access-base-rooting`** said "4 legacy element opcodes". It is **5**:
  `Op::DeleteElem` was omitted, and `delete o[k]` carries **both** carved defects. ⚠ *Even "5" was a
  curated count* — §18.1 records the real figure, which is roughly twenty arms across the whole
  dispatch loop, and the slot is superseded.

This is §13's rule failing again in a *new* place — the slot bodies. The rule now reads: enumerate by
command **wherever a count appears, including prose in slots and plan sections**, not only in code.

### Spec citations: I fabricated a section number

**`§6.2.4.5` does not exist.** RequireObjectCoercible is **§7.2.1**, and the ordering guarantee the
comment attached to it is §6.2.5.5 step 3.a's `ToObject`. The concept sweep found **6 pre-existing
siblings** plus 3 more of the same class (`§6.2.4.8 PutValue` → §6.2.5.6 step 3.a, ×2;
`§6.2.4.1` → §6.2.5.5 step 3.d; `§9.4.3` → §10.1.8.1 OrdinaryGet step 7). Also corrected: `o[k]++`
is **postfix §13.4.2.1 steps 1/3/6** (§13.4.4.1 is *prefix*, and its GetValue/PutValue are 3/6 not
1/5); `UpdateEmpty` is **§14.2.2 step 3** (AO §6.2.4.4), not §14.5.1.

### The edge-dense verdict, upheld

Codex R2 raised that this slice bundles ≥3 intersecting invariant axes without the mandatory
pre-implementation plan-review. **Measured from the diff: 6 axes.** The exemption in §2.5 rests on a
"narrow enough to skip" judgment made against a *one-`assert!`* charter that implementation falsified
three times over, and was never revisited — so the exemption did not hold as shipped.

**Disposition: retroactive plan-review + this record, NOT re-slicing.** Axes 3/4/6 (completion
ownership, update-expression semantics, stack discipline) are causally *downstream* of axes 1/2 —
the short-circuit cleanup is what exposed the completion-value ownership bug, and `IncElem` is the
semantic sibling of the key-conversion fix — so splitting them out would divide a root fix and ship a
knowingly-broken intermediate. **P / 0b / 0c / 1a / 1b remain plan-review-mandatory before
implementation; 0a's skip is not a precedent, and §2.5's narrowness verdict is hereby withdrawn.**

### Divergences pinned rather than fixed (all pre-existing, all gate-found)

`#11-vm-topropertykey-symbol-from-toprimitive`, `#11-vm-statement-completion-updateempty` and
`#11-vm-delete-elem-raw-key-array-fast-path` — the second being the half of the completion-ownership
bug this slice did **not** fix: the VM has no `UpdateEmpty`, so `42; if (false) {}` yields `42` where
the spec says `undefined`. Each is asserted by a `*_known_divergence` test docstring-fenced to its
slot, so none can widen unnoticed.

`#11-vm-operand-rooting-by-construction` is fenced the same way, by
`compound_assign_rhs_lost_to_gc_known_divergence`. ⚠ *An earlier revision of this paragraph claimed
it **cannot** carry such a test "because a GC race has no observable result to assert" — that was
wrong, and Codex R2 re-raising the theme a third time is what forced the measurement that disproved
it.* The `force_gc_before_next_alloc` one-shot makes the collection deterministic, so the race has a
reproducible outcome: `o[k] -= mk()` yields `-1` with no collection in the window and a
`TypeError: Cannot convert object to primitive value` with one. The pin asserts **both** that and the
identifier form `z -= mk()`, which fails byte-identically through a lowering that predates this slice
entirely — so the test doubles as the evidence that the slice adds a *spelling* reaching an
already-reachable defect rather than introducing or widening one.

`#11-vm-internal-error-hard-exit` remains docstring-anchored only; catchability of an
unreachable-by-construction invariant guard genuinely has no observable result to assert without
first synthesising malformed bytecode. Per the ratified category split the three above are **not**
the slice's own deferrals; the two carves are (own count: 2 — see §8).

### §18.1 The carve — five audits, five boundaries, and the decision to stop sweeping

Rounds 2–7 grew this slice from one `assert!` site into an engine-wide operand-rooting sweep: **15
defects beyond the original carve**, across `IncProp`/`DecProp`, `In`, `Add`, `Instanceof`,
`TemplateConcat`, `ops.rs`'s three operator helpers, the three computed-key definition bodies,
`SpreadObject`, `ArraySpread`, `IteratorRest`, `GetProp`/`SetProp` and the unary arms. Each entered
by a reviewer pointer, one round at a time.

| Pass | Sweep word used | Missed |
|---|---|---|
| R2 | "`GetElemRef`" | the other 5 element opcodes |
| R3 | "**element** access" | `IncProp`/`DecProp`, `In`, `Add`, `Instanceof`, `TemplateConcat` |
| R4 gate | "**dispatch arm**" | the 3 `ops.rs` helpers |
| final | "`&mut` callee that can reach user JS" | +7 more (computed defs ×3, `SpreadObject`, `ArraySpread`, `IteratorRest`, `SetProp`) |
| R7 | — | the **unary** arms + `op_get_iterator`, which the row above had declared *checked and clear* |

Two derived "safe" arguments were falsified, both after having been used to declare arms clean:
*"the operand is re-rooted as the receiver of the user code"* is false for an **arrow or bound**
callee (its `this` comes from the closure or the binding), which exposed the spread source and the
iterator; *"the value is rooted as the callee's argument"* is false for a **zero-parameter** callee,
since `call_internal` copies only `args[..min(argc, param_count)]` — which exposed `Op::SetProp`'s
stored value, **and the test pinned one commit earlier asserted the opposite**, i.e. a wrong verdict
shipped as a guard. The one load-bearing fact that *does* bound the set is `call_dispatch` setting
`gc_enabled = false` around native bodies.

**Root-check (SKILL Step 4, both questions).** *Abstraction-coverage*: a canonical algorithm is
missing, but it is not another helper — `binary_op_rooted` and the `op_*` family already existed.
What is missing is anything that makes an unrooted hold **unrepresentable**. *Own-ideal*: CLAUDE.md
verbatim — "One issue, one way … 単一の正準形に一括収束させる。**新 seam + N 個の legacy 実装が共存する
strangler 中間状態を残さない**". Five rounds of "convert N more sites", each declared complete, *is*
that strangler, sustained across rounds. An audit that must be re-run correctly forever is not a
canonical algorithm.

**Disposition (escalation option B, carve).** The rooting work is **not causally downstream of member
assignment** — unlike the completion-value split, which genuinely is and therefore stays. `612f167a`
reverts every one of those arms to merge-base, so the PR makes nothing worse than the branch point,
and the work moves to `#11-vm-operand-rooting-by-construction` with `#11-vm-internal-error-hard-exit`
alongside it (the `VmInner::raise` kind-keyed routing was itself a response to the ~15 new
internal-error producers the sweep had created). The implementation and its 510-line, 14-arm test
module are preserved on branch **`vm-p4-rooting-carved`** so the follow-up starts from working code
rather than re-deriving it. Diff: 25 files / +2637 −471 → 22 / +1444 −183.

**The carve commit was itself gated** (`/elidex-review`, 0 CRIT / 8 IMP / 7 MIN), and that pass is
why §16's `Op::GetElemRef` rooting is now *stated and pinned* rather than asserted: "its stack effect
requires the in-place read" is false as a forcing argument, since a pop-then-repush produces the
identical `[object key -- object key' value]`. The real reason is GC safety, and the pin is
mutation-tested — under a popping implementation it fails with `NaN` instead of `3`.

That pass also found the **fourth** site deciding admissibility inside its own lowering:
`compile_forin_left_binding` discarded the iteration value through `Op::Pop`, so
`for (o.p in obj)` / `for (this.#x of a)` / `for (super.x of a)` / `for ([a,b] of a)` all ran the
loop with the target never written. Now routed through the same `unsupported_member_target`, emitted
at the assignment site because ECMA-262 §14.7.5.7 step 8.g runs **per iteration** — an empty iterable
performs no assignment and must not throw.

**And a seventh mis-attributed §-number, shipped by the carve commit itself.** §15.7.14
ClassDefinitionEvaluation does **not** pass `enumerable: false` — it calls `ClassElementEvaluation`
with one argument; **§15.7.13 ClassElementEvaluation** is what passes `false` to §15.4.5
MethodDefinitionEvaluation. The carve introduced this at two sites *while correcting two others*. The
running total for this one PR is now: `§6.2.4.5` (RequireObjectCoercible → §7.2.1), `§12.5.3.2`
(delete → §13.5.1.2), `§12.10.4` (instanceof → §13.10.1/§13.10.2), `§12.2.6.8` (→ §7.3.25),
`§14.3.8` (→ §15.4.5 + §15.7.13), and `§14.3.8`'s own replacement. **The rule §13 states for counts
applies unchanged to §-numbers: look it up, every time, including when the commit's whole purpose is
correcting lookups.**

### §18.2 The converge loop on the carved head, and what it proved twice

Restarted from round 1 at `c1791ed0`; the pre-carve rounds are void (the head moved twice).

| Round | Commit | CRIT | IMP | MIN | New-real | Outcome |
|---|---|---|---|---|---|---|
| R1 | `c1791ed0` | 0 | 0 | 1 | 1 | `compiler/stmt.rs` 1001 → 712, new `stmt_loop.rs` |
| R2 | `fc06a02c` | 0 | 1 | 0 | 1 | operand rooting **measured**; slot held, divergence pinned |
| R3 | `a7460311` | 0 | 0 | 0 | 0 | dry (all three channels) |
| **fix-delta `/elidex-review`** | `612f167a..a7460311` | 0 | **14** | 18 | — | all applied |
| R4 | `6d759b1e` | 0 | 0 | 1 | 1 | the 7th sibling miss — in the sweep commit |
| R5 | `dcccb15d` | 0 | 0 | 0 | 0 | dry |
| R6 | `dcccb15d` | 0 | 0 | 0 | 0 | dry |
| R7 | `d44e2c58` | 0 | 0 | 0 | 0 | dry |
| R8 | `511a4ac4` | 0 | 0 | 0 | 0 | dry |
| R9 | `90d42384` | 0 | 0 | 0 | 0 | dry |
| R10 | `00e34dad` | 0 | 0 | 0 | 0 | dry |
| R11 | `3f555379` | 0 | 0 | 0 | 0 | dry |
| R12 | `2156ac76` | 0 | 0 | 0 | 0 | dry |
| R13 | `3d6dc576` | 0 | 0 | 0 | 0 | dry |
| R14 | `3d6dc576` | 0 | 0 | 0 | 0 | dry |
| R15 | `59373952` | 0 | 0 | 0 | 0 | dry |
| R16 | `59373952` | 0 | 0 | 0 | 0 | dry |

Rounds R6-R16 were absent from every record of this PR until they were enumerated from the API at
landing; the tables and the handoff memo both stopped at R5, and the memo's own summary of the tail
("12 dry across **11** heads") was off — the measured tail is **12 consecutive dry assessments
across 9 distinct heads** (`dcccb15d`, `3d6dc576` and `59373952` were each assessed twice). Recall
miscounting a tally *of this document's own recall miscounts* is §13's rule failing on itself. The
enumeration is `gh api --paginate` over `pulls/489/reviews` ∪ `issues/489/comments`, keyed on the
`Reviewed commit:` marker; re-run it rather than reading these rows forward.

Two facts the rows do not show, both measured. **Every formal review body on this PR is 621 bytes —
Codex's finding-free boilerplate.** No P-badge body ever appeared, so all 18 findings arrived as
inline threads and the review-body channel contributed nothing here; that is a fact about this PR,
not a reason to stop scanning the channel. And **the merged head was never reviewed**: `origin/main`
was taken in after R16, producing `03f3e81f`, which was squash-merged as `658cc302` under a
user-approved `# merge-stale-ok` rather than a seventeenth round. The PR-owned changeset was
verified byte-identical across that merge (`git diff origin/main...HEAD` before and after), so what
went unreviewed is the *new base*, not this slice.

**Reviewer real-gap exhaustion is not design convergence — demonstrated twice.** R5/R6 went dry
pre-carve and the TERMINAL gate then found 12 IMP (§18); R3 went dry post-carve and the fix-delta
gate found **14**. Both times, proposing merge off the dry rounds would have shipped everything the
gate caught, including a fabricated §-number in each case. The overlay's rule that the fix-delta pass
is skippable only with a per-axis yield-0 table is doing real work; on this PR it has never once been
skippable.

**What the 14 were, at root.** Nine were one failure wearing different clothes: **the docstrings had
become an R-loop changelog.** Prose asserting a fact not re-derived at write time, which the *next*
commit falsified and nothing caught — "takes fields by reference so the bodies are unchanged"
(falsified by the clippy fix one commit later), "a pop-then-repush … and every test still passes"
(contradicted six lines below by the pin in the same commit), "a slot-cited message" (the same delta
removed every slot id), "pinned by … below" (above), "the one arm this slice introduces" (four),
"1001 → 704" (712, counted by recall — against §13's own rule). The fix is not per-sentence: **code
docs state present-tense invariants, and the history lives here.**

**A correction that was itself wrong.** The delta corrected `Op::PopCompletion`'s body-kind
attribution from §15.2.3 to the generator/async AOs — and got the dispatch wrong: §10.2.1.4 has one
step (`Return ? EvaluateBody`), so the per-body-kind dispatch is **§10.2.1.3**, over eight
productions. "None surfaces a trailing expression's value" is falsified by **§15.3.3
EvaluateConciseBody** (arrow expression bodies) and by the `Initializer` production; the claim is now
scoped to the four statement-list productions the opcode can reach, with the exclusion stated.

**Sibling misses reached seven.** R4's was the sharpest: the fix-delta gate flagged the
`PutValue(lhsRef, …)` rendering at *both* of its sites and called them semantic siblings; one was
fixed and the other shipped — in the commit whose own message named un-swept siblings as the root
cause it was correcting. The rule keeps failing at the moment one believes one has just applied it,
which is the argument for *grepping the concept* rather than trusting that belief.

**Two measured pins now fence the rooting slot** (both flip to the spec answer when it lands):
`compound_assign_rhs_lost_to_gc_known_divergence` (`o[k] -= mk()` **and** the identifier
`z -= mk()`, whose byte-identical failure is the pre-existence proof) and
`inc_elem_base_lost_to_gc_known_divergence` (`mk()[k]++` → `1` with no collection in the window,
`NaN` with one). The second exists because a block comment *claimed* `Op::IncElem`/`Op::DecElem` were
covered by the first, which never emits those opcodes — and they are the arms this slice actually
rewrote. Both assert **"not the spec answer"** rather than a specific error: whether a freed slot
stays empty (`TypeError`) or is recycled (`NaN`) is `free_objects.pop()` arithmetic that one new
built-in global can flip, so pinning the message would fence the artifact instead of the divergence.
