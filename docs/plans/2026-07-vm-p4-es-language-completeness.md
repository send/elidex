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

All rows verified via `.claude/tools/webref` against `ecma262` — the original 25 on 2026-07-26,
[C43] on 2026-08-03.

⚠ **This table is not the document's citation chokepoint, and reading it as one is how two
fabricated anchors survived under a blanket header before (§10).** It covers the ECMA-262 sections
carrying an ID; the document also cites ECMA-262 sections inline without IDs, and cites **WebIDL**
and **WHATWG HTML** — specs this header has never
covered. Verify those at their use sites with `webref heading webidl <n>` / `webref dfn html <t>`.

| ID | Citation | Anchor | Used by |
|---|---|---|---|
| [C19] | ECMA-262 §13.3.8.1 Runtime Semantics: ArgumentListEvaluation | `#sec-runtime-semantics-argumentlistevaluation` | Slice 1b, 4b |
| [C20] | ECMA-262 §13.3.6.2 EvaluateCall | `#sec-evaluatecall` | Slice 1b, 4b |
| [C33] | ECMA-262 §13.3.5.1.1 EvaluateNew | `#sec-evaluatenew` | Slice 1b |
| [C34] | ECMA-262 §7.3.14 Construct | `#sec-construct` | Slice 1b |
| [C35] | ECMA-262 §7.4.10 IteratorStepValue | `#sec-iteratorstepvalue` | Slice 1a |
| [C36] | ECMA-262 §7.4.11 IteratorClose | `#sec-iteratorclose` | Slice 0bc ([C39] conformance) / Slice 1a (dec. 13a) / `#11-vm-iteratorclose-precedence-convention` (dec. 13b) — **not** Slice 1b (§2.5 C×F) |
| [C37] | ECMA-262 §13.3.9.1 Runtime Semantics: Evaluation (optional chain) | `#sec-optional-chaining-evaluation` | Slice 1b |
| [C38] | ECMA-262 §13.3.9.2 Runtime Semantics: ChainEvaluation | `#sec-optional-chaining-chain-evaluation` | Slice 1b |
| [C13] | ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation (`super`) | `#sec-super-keyword-runtime-semantics-evaluation` | Slice 1a handler + 1b emit, 3 |
| [C21] | ECMA-262 §7.4.4 GetIterator ( obj, kind ) | `#sec-getiterator` | Slice 1a, 6 |
| [C22] | ECMA-262 §13.2.4.1 Runtime Semantics: ArrayAccumulation | `#sec-runtime-semantics-arrayaccumulation` | Slice 1a (`SpreadElement` drain) + contrast for dec. 3 |
| [C39] | ECMA-262 §13.15.5.2 Runtime Semantics: DestructuringAssignmentEvaluation | `#sec-runtime-semantics-destructuringassignmentevaluation` | **Slice 0bc** |
| [C23] | ECMA-262 §13.2.8.4 GetTemplateObject | `#sec-gettemplateobject` | Slice 4a |
| [C40] | ECMA-262 §13.3.11.1 Runtime Semantics: Evaluation (tagged template) | `#sec-tagged-templates-runtime-semantics-evaluation` | Slice 4b |
| [C44] | ECMA-262 §22.1.2.4 `String.raw ( template, ...substitutions )` | `#sec-string.raw` | Slice 4c |
| [C24] | ECMA-262 §13.3.7.3 MakeSuperPropertyReference | `#sec-makesuperpropertyreference` | Slice 3 |
| [C25] | ECMA-262 §9.1.1.3.5 GetSuperBase | `#sec-getsuperbase` | Slice 3 |
| [C26] | ECMA-262 §7.3.32 DefineField | `#sec-definefield` | Slices 2ab, 2ac |
| [C27] | ECMA-262 §7.3.33 InitializeInstanceElements | `#sec-initializeinstanceelements` | Slice 2ab |
| [C28] | ECMA-262 §15.7.10 ClassFieldDefinitionEvaluation | `#sec-runtime-semantics-classfielddefinitionevaluation` | Slice 2aa |
| [C29] | ECMA-262 §7.3.26 PrivateElementFind | `#sec-privateelementfind` | Slice 5 |
| [C30] | ECMA-262 §7.3.28 PrivateMethodOrAccessorAdd | `#sec-privatemethodoraccessoradd` | Slice 5 |
| [C31] | ECMA-262 §7.3.30 PrivateGet | `#sec-privateget` | Slice 5 |
| [C41] | ECMA-262 §7.3.31 PrivateSet | `#sec-privateset` | Slice 5 |
| [C42] | ECMA-262 §7.3.27 PrivateFieldAdd | `#sec-privatefieldadd` | Slice 5 |
| [C43] | ECMA-262 §13.15.2 Runtime Semantics: Evaluation (assignment operators) | `#sec-assignment-operators-runtime-semantics-evaluation` | Slice 0a (§16) |
| [C32] | ECMA-262 §27.9.3.2 AsyncGeneratorStart | `#sec-asyncgeneratorstart` | Slice 6a |

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

### §1.0 What this umbrella owns

The unit is the **stubbed-emit class**: a compiler arm that lowers a user-writable production to a
substitute — emitting nothing, an `Op::PushUndefined`, or a wrong constant — so the program runs and
answers wrongly without raising. §2.2's three passes **are** that class's derivation, and that
section's scope statement says, as a property of the commands, what they read.

**§5 sequences the work that discharges that class. A row enters §5 when the derivation produced the
defect.** A conformance defect measured some other way is not thereby unowned, and is never argued
away on that ground: it is recorded where it was measured — §2.2's per-slice-sweep rule is what puts
it there — and it is given an owner of its own, an umbrella or a §8 slot carrying the required
triple. That is already the disposition of **M**, **R**, `#11-vm-atomics-global` and
`#11-vm-typed-array-family-layering-and-gate`; until now it was reached once per defect instead of
stated once, which is why §5 grew a row every round while the class it was derived for did not.
`#11-vm-object-spread-source-coercion` and `#11-vm-arrow-lexical-new-target` already have such a
slot, so their §5 rows are rewritten below as pointers to it and nothing they measured is lost.

The document keeps its name. The scope statement above is what bounds the program; a title never
did.

### §1.1 Corrections to the prior memo

The re-probe **contradicted** `project_vm-p4-es-language-gaps.md` §2's scope-precision note and
found five gaps it never listed.

| Prior claim | Re-probe result |
|---|---|
| "`new X(...args)` … **works**" | **FALSE — broken.** `new C(...[1,2])` → `a=[1,2]`, `b=undefined`; `new C(...[1,2,3]).n` → `arguments.length === 1` |
| (not listed) | **`super.x` / `super.m()` / `super[k]` / `super.x=` all throw** `TypeError: Cannot convert undefined or null to object` — total loss of super-property access |
| (not listed) | **public class fields `class A{x=1}` → `undefined`** (silent). `static x=1` *does* work |
| (not listed) | **`obj[k] += v` PANICS the process** (`assert!`, `compiler/expr_assign.rs:170`) |
| (not listed) | **destructuring *assignment* is a silent no-op** at the baseline — see §1.2; 0a made it a scoped throw |
| (not listed) | **async generators non-functional** — `ag().next` is `undefined`; `for await…of ag()` raises an unhandled `TypeError: value is not iterable` |
| `super(...)` grouped with broken | **`super(...args)` works in the directly executed constructor spelling the probe ran** — the one correct spread path, and 1b's reference implementation. ⚠ **Narrowed at PR-B** — a `super` spelling reached through an arrow resolves against the arrow's own frame and throws; see §2.2's `parser/arrow.rs:104-119` row |
| "flag accessors missing" | Precise: `.flags` / `.lastIndex` / `.exec` **work**; `.global`/`.ignoreCase`/`.multiline`/`.sticky` and `@@match`/`@@replace` absent |
| `new.target` "probe mis-designed" | Working **in the direct-constructor spelling the probe ran**. ⚠ **Narrowed at PR-B** — lost through an arrow call; see §2.2's `vm/dispatch_class.rs:29-37` row |

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
constructs that variant: all producers use `Simple`.) This is registered as **Slice 0bc** (§5).

Also newly confirmed silent-wrong, previously mis-tiered or unlisted:
- **`import('x')` → `undefined`** (not a Promise). `compiler/expr.rs:200` compiles
  `ExprKind::DynamicImport` to `PushUndefined`, and the parser applies **no module gate** to
  `import(...)` (`parser/primary.rs:382`) — so this is reachable in ordinary script context. **T1,
  not T3.** (`import.meta` *is* correctly module-gated at `parser/primary.rs:390`.)
- **`{1n: 'x'}` → key is the empty string** `{"":"x"}` (`compiler/expr_object.rs:117-121`
  "conservative fallback").
- **`obj.#x++` silently keeps the old value** (`compiler/expr_ops.rs:249`).
- ⚠ **RETRACTED at PR-B — "Computed class *accessor* keys hard-error (`compiler/expr_class.rs:588-590`)"
  was false, at `39bbdb1b` and at baseline `f7d9b5ce` alike.** Measured: `let k='p'; class C { get
  [k]() { return 7 } }; (new C).p` → `7`, and `let n=0; class C { get [++n]() { return 7 } }; n` → `1`.
  Computed class members take an earlier path (`expr_class.rs:523`, identical at `f7d9b5ce:523`) and
  never reach the cited guard, which is unreachable dead code — see the corrected §2.2 row. R1's
  "computed class keys correct" needed no narrowing after all.
- **Object-literal accessor keys with a non-Identifier / non-String-literal key are a silent no-op**
  (`compiler/expr_object.rs:26-39`) — the defect the retracted item was pointing at, one construct
  over. `let k='p'; let o={ get [k]() { return 7 } }; o.p` → `undefined`, and the key expression never
  runs: `let n=0; ({ get [++n]() {} }); n` → `0`. Numeric literal keys too: `({ get 1(){ return 7 } })`
  defines nothing. See §2.2.
- **Object-pattern destructuring with a non-Identifier / non-String-literal / non-computed key is a
  silent no-op** (`compiler/stmt_destructure.rs:117-119` at baseline, `:118-120` at head):
  `const {1: a} = {1: 'x'}; a` → `undefined`. See §2.2.

Newly confirmed absent: `Array.prototype.at` / `String.prototype.at` / `findLast` /
`findLastIndex` / `toSorted` / `Object.hasOwn` / `Object.groupBy` / `String.matchAll`.
⚠ **This is a probe sample, not the sweep boundary (Codex R5).** `Array.prototype.toReversed`,
`toSpliced` and `with` — ECMA-262 §23.1.3, alongside the `toSorted` listed here — appear nowhere in
the repo or this plan (`grep -rl` → 0 for each, and 0 for `toSorted` too). Since §7.2 derives its
permanent rows from the *enumerated* absences plus one row per later slot, **the Slice-9 umbrella
could satisfy this plan with those still missing**. Each Slice-9 sub-slice must derive, at its own
start, the whole property surface ECMA-262's main body defines on the object it is named for —
**edition-agnostic, minus Annex B (I-6)**, per its §5 charter — and treat this list as the seed that
found the class, not as the boundary. Which sub-slice each name falls to follows from the sub-slice
charters in §5.
Confirmed **correct**, no action: destructuring **declarations** (incl. nested/default/rest, and
parameter destructuring) — ⚠ **narrowed at PR-B to Identifier, string-literal and computed keys**;
other key kinds are a silent no-op (see the retraction bullets above and §2.2) — **and narrowed
again here on a second axis: the entry holds only for a pattern that drains its source.** An array
binding pattern that stops early pops its iterator instead of closing it, so `const [x] = it` never
calls `it.return()`, and the arm is shared by declarations, parameter patterns and for-in/of heads
alike (§2.2's `stmt_destructure.rs:36-81` row) —
`IncElem`/`DecElem` (`a[0]++`), `await` microtask ordering, static
fields/methods — ⚠ **narrowed at PR-B to the *definition*, which is all the probe `static x = 1`
exercises**; the initializer runs with the enclosing function's `this`, so
`class C { static x = this }` gives `C.x === C` → `false` (§2.2's `expr_class.rs:402-427` row) —
computed **class** keys (methods *and* accessors — the retraction above),
object-literal accessors with **Identifier or string-literal** keys only (⚠ likewise narrowed at
PR-B), `flatMap`, `Promise.prototype.finally`. The class and object-literal accessor paths differ in
**failure mode**, not only in coverage: a key kind neither lowers is a `CompileError` in a class
(measured: `class C { get 1n() { return 7 } }`) and a silent `Op::Pop` in an object literal.

The static-field narrowing and §1.1's `new.target` and `super(...args)` narrowings share the shape
this section's absence list already carries: **a probe that exercises one spelling cannot close a
surface.** `static x = 1` cannot distinguish a correct field receiver from an incorrect one, `new F()`
read from `F`'s own body cannot distinguish a frame-local `new.target` from a lexical one, and
`super(...args)` written directly in a constructor cannot distinguish a frame-local `super` from a
lexical one.

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
all of them — **18** zero-emit opcodes in total, see §2.3). So a fix that targets the dispatch stub touches dead code and changes nothing.

Each stub **silently substitutes `undefined` / `false` / a wrong arity** for the spec value. That
is why a whole tier of these was invisible to the defer-slot ledger: a stub throws no error, fails
no test, and registers no TODO.

**P4 = discharging that inventory**, at the emit layer, with connect-or-delete applied to the dead
opcodes behind it. Directly mandated by CLAUDE.md:

- **「TODO 先送り禁止」** — exactly the deferred-implementation pattern the rule forbids; they never
  got the "理由 + 対処時期を明示して確認を取る" step.
- **「dead code は接続するか削除」** — **18** opcodes with documented stack effects and zero emit sites (§2.3).
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
grep -rn "Op::PushUndefined" *.rs        # class 1  → 24 hits. The hit count is the only
                                         # figure this block carries: which hits are defects
                                         # is a read-every-hit classification step, and a
                                         # defect count written here goes stale silently
                                         # (pass 2 misclassified one — see below).
# Pass 3 — STRUCTURAL production coverage (added R2 round 4; passes 1-2 are
#          comment-keyed and cannot see an arm that skips silently *without* a marker)
awk '/^pub enum ExprKind/,/^}/' ../ast.rs | grep -oE '^    [A-Z][A-Za-z]*'   # 30 variants
awk '/^pub enum StmtKind/,/^}/' ../ast.rs | grep -oE '^    [A-Z][A-Za-z]*'   # 24 variants
#   → map every variant to its compiler arm; flag any arm that neither emits an op
#     nor returns CompileError for a user-writable production.
```

**Pass 3 result (run 2026-07-26; the two counts re-derived at `658cc302` by the same two `awk`
commands above, still **30** and **24**).** All **30** `ExprKind` and all **24** `StmtKind` arms
exist in their compilers, so no *variant* is unhandled — the class-1/2/3 defects are in arm
*content*, tabled below. (Not "no production is unhandled": pass 3's input is the variant list, so a
production with no variant is not among the 54 it checks. That is the scope statement below, not a
separate caveat.)
`compile_stmt` groups **6** `StmtKind` variants into one no-op arm (`stmt.rs:31-39`):
`Empty | Error | Debugger | ImportDeclaration | ExportDeclaration | FunctionDeclaration`. Of these
`Empty`/`Error`/`Debugger` are legitimate. ⚠ **`FunctionDeclaration` was classified
"legitimate-and-documented (hoisted at function/script level)"; that holds only at the top level and
the classification is corrected here.** The arm's own comment is true of the two hoist loops that
exist and of nothing else — `compiler/mod.rs:82` iterates `program.body` and
`compiler/expr_function.rs:167` iterates `func.body`, both immediate-child lists, and those are the
only other sites (`for f in $(git ls-tree -r --name-only 658cc302 crates/script/elidex-js/src/compiler/); do git show 658cc302:$f | grep -n 'StmtKind::FunctionDeclaration'; done`
→ `expr_function.rs:167`, `mod.rs:82`, `stmt.rs:37`). `StmtKind::Block` (`stmt.rs:134-144`) only
recurses back into `compile_stmt`, so a block-nested declaration reaches the no-op arm and is never
instantiated — tabled below. **`ImportDeclaration` / `ExportDeclaration` are silent no-ops** —
and both passes 1 and 2 missed them, because the arm emits nothing (pass 2 blind) and its comment
says "stubs", a word not in pass 1's pattern list.

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
its operand; every such node is an early-SyntaxError position per spec). 0ca must run pass 3 at
**both** granularities — variant-level, then sub-arm-level over the enum-shaped inner matches, which
is where every class-2 defect found so far actually lives. **That inner-enum set is derived, not
listed.** AST-side candidate set:
`grep -oE '^pub enum [A-Za-z]+' crates/script/elidex-js/src/ast.rs` → **26** at `658cc302` and at
HEAD `d49465d0` (`git diff 658cc302 HEAD -- crates/script/elidex-js/src/ast.rs` is empty), plus the
compiler-internal enums the same arms match on (`VarLocation`, `compiler/resolve.rs:13`). An enum is
**in scope** when a compiler arm matches on one of its variants. Swept so far: `ClassMemberKind`,
`MemberProp`, `AssignTarget` and `PropertyKey` (four of the 26) plus `VarLocation`. **`PatternKind`
is one of the 26 and was never in that set**, and the destructured-catch-parameter row below is what
that omission hid. ⚠ **`PropertyKey` was swept and the
sweep still missed four of its arms** (tabled below — three rows added at PR-B, one corrected in
place) — because the sub-arm pass is
still *per file*, and `PropertyKey` lowering is open-coded across **3** of them
(`grep -rln 'PropertyKey::' crates/script/elidex-js/src/compiler/` at `39bbdb1b`:
`expr_class.rs`, `expr_object.rs`, `stmt_destructure.rs`) with catch-alls of four different shapes —
`Op::Pop`, `Op::PushUndefined`, `CompileError` and a bare `_ => {}` — and the `CompileError` one is
invisible to any substitution-class grep. Slot
`#11-vm-property-key-lowering-unification` (§8) owns the collapse; 0ca's sub-arm pass
should be run **per inner enum across all files**, not per file.

Pass 1 returns 31 hits, pass 2's `PushUndefined` arm returns 24; **the large majority are
legitimate** (`yield` with no argument, optional-chain null path, parameter/field defaults, empty
returns, `ExprKind::Error` after a parse error already reported). Classification is therefore a
read-every-arm step, and it is what makes the count trustworthy rather than the grep itself.

**The sweep found 3 defects no prior inventory (probe, R1, R2, or round-2 review) contained** —
parenthesized assignment/update targets, all probe-confirmed silent no-ops **at `f7d9b5ce`** (0a
made all three loud — see §2.2's ⚠):
`(x)++` → x unchanged · `(x)+=1` → x unchanged · `(a[0])++` → unchanged. Root:
`parser/expr.rs:531-536` `is_valid_assign_target` unwraps `ExprKind::Paren` to *validate* but never
normalises it, and neither `expr_ops.rs` nor `expr_assign.rs` unwraps, so a parenthesized target
falls to the catch-all arms. This is direct evidence that decision 9's concern was correct and that
a stipulated inventory would have shipped incomplete.

Emit-site counts grep-verified 2026-07-26 (command in §2.1).

⚠ **This is what the documented derivation found at `f7d9b5ce`, not the defect surface** — the §1.2
treatment, applied to §2.2 itself. **Scope, stated as what the commands look at**: pass 1 matches
comment text in the Rust sources under `crates/script/elidex-js/src/compiler/`; pass 2 counts
`Op::PushUndefined`
occurrences in those same files; pass 3 walks the `ExprKind` / `StmtKind` variant lists in `ast.rs`
and maps each variant to its arm in those files. So the derivation sees compiler arms, their marker
comments, and productions that have an AST variant. Anything else is beyond its reach **by
construction** — that is a property of the commands, not a list of known holes, and this section
does not carry such a list.

**§2.2 is therefore a seed, not a boundary** — the framing §1.2 already applies to its own probe. It
found the class and justified the slice plan; nothing is out of scope by being absent from it, and
no row may be argued away on that ground.

**One mechanism discovers defects, and it is the per-slice sweep.** §5 already mandates that 0ca
re-run all three passes against **0ca's own parent HEAD**, and §5's `Primary module(s)` demotion
already puts touch-set derivation against a slice's own parent HEAD inside every slice's mandatory
`/elidex-plan-review`. The rule those two share is the canonical one: **the slice that owns a file
reads that file against its own parent HEAD and tables what it finds**, whatever directory the file
sits in. A second mechanism — a section-level inventory that cannot be complete — would be the
duplicated decision surface CLAUDE.md *One issue, one way* forbids, so there is not one. Two
obligations follow, and they are the whole of what this paragraph asks for:

- **0ca reads pass 3 for arm *content*, not only arm presence.** An arm that exists and does the
  wrong thing is invisible to a presence walk. State that in 0ca's memo.
- **A slice whose touch set reaches a file this derivation never reads** — anything under `vm/`,
  `parser/` or `scope/` — reads it against its own parent HEAD and tables what it finds there,
  rather than inheriting §2.3's dead-opcode partition as the dispatch-layer inventory.

Rows below whose Site column names a file outside `compiler/` arrived exactly that way. They are
evidence the mechanism works, not exceptions to a rule.

**Consequence for §7.2.** §9 dec. 9(b) derives the permanent conformance table as one row per §2.2
defect row, plus the §1.1/§1.2 absences — so the conformance table inherits this section's scope.
Round 8 found that on the T3-absence axis and patched it with a second source (§7.2); the general
form is the same, and §7.2 states it: rows for what this derivation does not read arrive from the
slice that owns the code, derived against its own parent HEAD.

⚠ **This table is the probe at baseline `f7d9b5ce`; its Site / Emits / Observable columns are frozen
there.** Read them as "what the sweep found", never as current state. Slice 0a then landed
(`658cc302`) and, beyond its three T0 rows, converted **nine** further rows from a silent no-op into a
*scoped* `Op::ThrowUnsupported` — marked **0a ✅ loud** in the Slice column. The construct is still
unimplemented and the named slice still owns it; what changed is the **failure mode**, which is the
very axis 0ca is scoped by, so 0ca's charter is narrowed accordingly below. **Nine is a count of marked
rows in this table, not of call sites** — `grep -rn 'emit_unsupported(fc' compiler/ | grep -v 'fn '`
gives **10** at `658cc302`, and the two do not correspond: two call sites guard constructs with no row
here, while one row's rejection is reached from two of them. One of the nine (`AssignTarget::Pattern`)
is a **dead** arm the parser never constructs, so eight are observable conversions.

Site line numbers are `f7d9b5ce`-relative and several have moved: `compiler/stmt.rs` went **964→712**
when 0a split out `compiler/stmt_loop.rs`, so the `:877-878` and `:882` rows now live in that new
file. (§18.2's R1 row says 1001→712, which is right at *its* anchor `c1791ed0` — mid-PR, after
earlier 0a commits had grown the file. Do not carry that figure back to this baseline.)

⚠ **Three rows were added at PR-B and a fourth was corrected in place, after the freeze, and the
freeze still holds for all four.** Added: the object-literal accessor key row, the object-pattern
destructuring key row, the object-pattern rest-exclusion row. Corrected in place: the
computed-class-accessor row, which *replaces* the row asserting a `CompileError` — measured,
`git show 2a51106c -- docs/plans/2026-07-vm-p4-es-language-completeness.md` deletes that one row and
adds four, so net table growth is **+3**. The arms they cite are unchanged between `f7d9b5ce` and
`39bbdb1b`, so their Observable columns are baseline facts as much as the rest of the table.
*(Inside the table cells, "base" means `f7d9b5ce` and "head" means `39bbdb1b`. Both SHAs are fixed
anchors; the bare word "head" is not, which is why every claim **outside** the table now names the
SHA.)* The
files are not all byte-identical, and the rows say where their line numbers moved:
`git diff f7d9b5ce 39bbdb1b -- crates/script/elidex-js/src/compiler/{expr_object,stmt_destructure,expr_class}.rs`
is empty for `expr_object.rs`; for `stmt_destructure.rs` it is one `use` line plus an
`emit_unsupported` in the `VarLocation::Module` **store** arm (+1 line shift), and for
`expr_class.rs` one comment expansion at `:542` (+2 line shift). Neither touches a `PropertyKey`
arm. **Pass 2 missed one of these four**: the destructuring-key row is a class-1
`Op::PushUndefined` substitution, and pass 2 claims to have read and classified all 24 hits.
`git grep -c 'Op::PushUndefined' f7d9b5ce -- 'crates/script/elidex-js/src/compiler/*.rs'` sums to
**24** and puts **3** of them in `stmt_destructure.rs` — `:51`, `:118`, `:170` — so `:118`, the
catch-all this revision tables as a defect, was inside the set pass 2 read and was classified
legitimate. The count was right; the classification was not — which is why the sweep block above
carries the hit count and no defect count.

| Site | Syntax | Emits | Observable | Tier | Slice |
|---|---|---|---|---|---|
| `compiler/expr_assign.rs:170` | `obj[k] += v` | **`assert!` → panic** | process abort | **T0** | 0a ✅ |
| `compiler/expr_ops.rs:29` | **`obj.p \|\|= v` / `&&=` / `??=`** (named member) | **`unreachable!` → panic** — short-circuit was implemented only for the *identifier* target, so every member logical assignment reached `compound_op_to_opcode` | process abort | **T0** | 0a ✅ |
| `compiler/expr_assign.rs:170` + `expr_ops.rs:29` | **`obj[k] \|\|= v`** (computed logical) | **panic** (both of the above) | process abort | **T0** | 0a ✅ |
| `compiler/expr_assign.rs:210-212` | `[a,b]=…`, `({x}=…)` | RHS only, no store | silent no-op | T1 | 0bc — **0a ✅ loud** |
| `compiler/expr_member.rs:70-76` | `f(...a)` | spread operand as one arg | silent wrong arity | T1 | 1b |
| `compiler/expr_class.rs:428` | `class A{x=1}` | **skipped entirely** | field `undefined` | T1 | 2 |
| `compiler/expr.rs:200-207` | `super.x`, `super[k]` | `PushUndefined` | TypeError at use | T2 | 3 |
| `compiler/expr.rs:237-239` | `` t`a${1}` `` | `PushUndefined` | tag never called | T1 | **4b** |
| `compiler/expr_member.rs:44`, `expr.rs:230` | `#x` get / `#x in o` | emits `GetPrivate`/`PrivateIn` → dispatch stub | `undefined` / `false` | T1 | 5 |
| `compiler/expr_assign.rs:202-206` | `o.#x = v` | `Op::Pop` (the `_ =>` arm); **no `SetPrivate` emit site exists at all** | write lost **and** the assignment evaluates to the *object*: `x = (o.#p = 5)` ⇒ `x === o` | T1 | 5 — **0a ✅ loud** |
| `compiler/expr.rs:200-207` | `import('x')` | `PushUndefined` | not a Promise | **T1** | *(see §5)* |
| `compiler/expr_object.rs:117-121` | `{1n: 'x'}` (**literal** key only — `{[1n]:…}` computed is correct, probe-verified). The cited arm is `PropertyKey::Literal(Literal::BigInt(_) \| Literal::RegExp { .. })` **inside** the `PropertyKind::Init` match whose head is `expr_object.rs:83` (arm head `:65`) — the same match the slot below names as the exhaustive copy the canonical lowering collapses the others into; a BigInt key is a key kind that lowering must answer | empty-string key → `{"":"x"}` | wrong key | T1 | `#11-vm-property-key-lowering-unification` |
| `compiler/expr_ops.rs:249` | `obj.#x++` | emits nothing, old value retained | silent no-op | T1 | 5 — **0a ✅ loud** |
| **`compiler/expr_ops.rs:261`** | **`(x)++`, `(a[0])++`** — parenthesized update target | operand evaluated only | **silent no-op** | T1 | 0bb — **0a ✅ loud** |
| **`compiler/expr_assign.rs:210-212`** | **`(x)+=1`** — parenthesized assign target (same catch-all as destructuring) | RHS only | **silent no-op** | T1 | 0bb — **0a ✅ loud** |
| `compiler/expr_ops.rs:147-149` + **`parser/`** | `delete this.#x` | `Pop; PushTrue` → `true` | wrong constant; ECMA-262 §13.5.1.1 makes it an **early SyntaxError** ⇒ **parse-time** rejection, so 0c's runtime-throw regime is wrong for it (same layer argument as §9 dec. 15). The sibling half of the *same* spec bullet (`delete <identifier>`) is **already** parser-gated, so the two halves must not land in two layers | T1 | **0ba** |
| `compiler/expr_ops.rs:133-163` (at `658cc302`) | **`delete (o.x)`**, **`delete o?.x`** — the arm tests the **raw** AST for `ExprKind::Member` (`:135`), so an `ExprKind::Paren` wrapper (`ast.rs:299`) or an `ExprKind::OptionalChain` takes the `else` at `:156-161` | `compile_expr`; `Op::Pop`; `Op::PushTrue` — the property **value** is evaluated, discarded, and `true` pushed | **silent success**: `{ let o={x:1}; delete (o.x); return 'x' in o }` → `true`. ECMA-262 **§13.5.1.2** step 4 deletes whenever Evaluation of the UnaryExpression yields a property Reference, and **§13.2.9.2** returns the ParenthesizedExpression's Reference unchanged | T1 | **0bb** |
| `compiler/expr_ops.rs:114-132` (at `658cc302`) | **`typeof (missingName)`** — the unresolvable-reference case runs only when the *immediate* argument is `ExprKind::Identifier` (`:117`), so a `Paren` wrapper defeats it | falls past `:132` to the generic `compile_expr` + `unary_op_to_opcode` (`:164-165`), whose `Op::GetGlobal` miss path raises `ReferenceError` (`vm/dispatch.rs:187-192`) | **throws** instead of returning `"undefined"`. ECMA-262 **§13.5.3.1** step 2.a | T1 | **0bb** |
| `compiler/expr_ops.rs:226` | module-binding **update** (`importedBinding++`) — *not* `delete`; falls to the `:261` catch-all, leaving the current value (the in-code "fall through to push undefined" comment is itself stale) | operand only | silent no-op | T1 | M — **0a ✅ loud** |
| `compiler/stmt.rs:877-878` | module-binding `for-in` target | `Pop` | silent no-op | T1 | M — **0a ✅ loud** |
| `compiler/stmt.rs:31-39` | `import`/`export` **declarations**, grouped into the `Empty`/`Debugger` no-op arm (found by the pass-3 structural sweep) | nothing | silent no-op — **latent**: unreachable until `parse_module` gains a production caller | T1 | M (precondition) |
| `compiler/stmt.rs:515-630` (at `658cc302`) | **`switch` does not enter its CaseBlock lexical scope** — `StmtKind::Block` (`:134-144`) and the `try` block (`:313-317`) both save `fc.current_scope_idx`, call `find_child_block_scope` and restore; the `StmtKind::Switch` arm does neither (`git show 658cc302:…/compiler/stmt.rs \| awk 'NR>=515 && NR<=632' \| grep -cE 'current_scope_idx\|find_child_block_scope'` → **0**, against **4** for the Block arm), while scope analysis *does* push a child `ScopeKind::Block` for the case block (`scope/visitor.rs:126-141`, `push_scope(ScopeKind::Block, …)` at `:131`) | ops emitted against the **enclosing** scope index, so a case-body declaration resolves to an outer binding | silent wrong **value**. Probe-measured at `658cc302` (`crates/` is byte-identical at `42c80199`): `let x=1; switch(0){case 0: let x=2;} x` → **`2`** where ECMA-262 gives `1`; control `let x=1; { let x=2; } x` → **`1`**; same shape inside a function → **`2`**. ECMA-262 **§14.12.4** Runtime Semantics: Evaluation steps 3-6 take `blockEnv = NewDeclarativeEnvironment(oldEnv)`, run `BlockDeclarationInstantiation(CaseBlock, blockEnv)` and set the running LexicalEnvironment to it, restoring at step 9. ⚠ **Arm-content defect** — the arm exists and emits, so a presence-only walk marks it handled | T1 | **B** |
| `compiler/stmt.rs:369-371`, `:380-382`, `:476-478`, `:501-503`, and `emit_pending_finally_bodies` (`:678-696`) (at `658cc302`) | **a `finally` body does not enter its own block scope** — `let x=1; try {} finally { let x=2 } x`. Every site that compiles the finalizer's statements compiles them against the enclosing `fc.current_scope_idx`, while scope analysis pushes a child `ScopeKind::Block` for the finalizer at `scope/visitor.rs:166`. Region probe, the same command over sibling regions of the one `Try` arm: `git show 658cc302:…/compiler/stmt.rs \| sed -n '<range>p' \| grep -cE 'find_child_block_scope\|current_scope_idx'` → **0** over `365,383` (no-catch finalizer) and **0** over `466,505` (shared + catch-rethrow finalizer), against **3** over `310,320` (the try block) | the finalizer's statements emit against the enclosing scope index, so a declaration in the finalizer resolves to an outer binding | silent wrong **value**. Probe-measured at `0c10870b` (`git diff --stat 658cc302 0c10870b -- crates/` is empty): `let x=1; try {} finally { let x=2 } x` → **`2`** where ECMA-262 gives `1`, and the `catch`+`finally` spelling `let x=1; try {} catch(e) {} finally { let x=2 } x` → **`2`** as well, so both emission branches carry it. Controls, same harness: `let x=1; { let x=2 } x` → **`1`**, `let x=1; try { let x=2 } finally {} x` → **`1`**, `let x=1; try { throw 0 } catch(e) { let x=2 } x` → **`1`** — the sibling scopes of the same statement are entered. Governing: ECMA-262 **§14.2.3** BlockDeclarationInstantiation (`webref aoid ecma262 BlockDeclarationInstantiation`), invoked by **§14.2.2** Block Runtime Semantics: Evaluation. **A call at the finalizer sites is not the fix.** `scope/visitor.rs:148` and `:166` both push with the `Try` statement's own `span`, so the try and finalizer scopes are siblings under one parent with the same kind *and* the same span, and `find_child_block_scope` (`stmt.rs:639-652`) returns the first child matching kind+span — always the try block. So the lookup key has to distinguish same-span siblings before any entry site can be added | T1 | **B** |
| `compiler/stmt.rs:31-39` + `compiler/mod.rs:82` + `compiler/expr_function.rs:167` (at `658cc302`) | **block-scoped function declarations are never instantiated** — `{ f(); function f() {} }`. Both hoist loops iterate an immediate-child list (`program.body` / `func.body`) and `StmtKind::Block` only recurses, so the declaration reaches the no-op arm. Scope analysis binds the name in the child block scope (`scope/visitor.rs:189` → `bind_and_visit_function` at `:422`, whose `state.add_binding(prog, state.current_scope(), …)` uses the innermost scope), so the slot exists and nothing ever writes it | nothing emitted at the declaration site, and no hoist reaches it | silent no-op → TypeError at call. Probe-measured at `658cc302`: `(function(){ { try { return 'called:'+f() } catch(e) { return 'ERR:'+e } function f(){ return 7 } } })()` → **`"ERR:TypeError: not a function"`**; `(function(){ { return typeof f; function f(){} } })()` → **`"undefined"`**; and `(function(){ { function f(){return 7} } return typeof f })()` → **`"undefined"`** as well. Control, the function body itself: `(function(){ return typeof g; function g(){} })()` → **`"function"`**. ECMA-262 **§14.2.3** BlockDeclarationInstantiation must instantiate and initialize it on block entry (AO number from `webref aoid ecma262 BlockDeclarationInstantiation`); §14.12.4 step 5 is the same AO for the CaseBlock, which is why the row above shares this row's owner | T1 | **B** |
| `compiler/expr_class.rs:430-447` | `class A{#x=1}` — `PrivateField` compiled only under `if *is_static`, no else | nothing | silent no-op | T1 | 5 |
| `compiler/expr.rs:186-189` + **`parser/expr.rs:256-263`** | `ExprKind::Spread` in prefix position (`var y = ...x`) — an early-SyntaxError position per spec; the parser's `Ellipsis` arm is **ungated** | operand only | silent no-op | T1 | **0ba** (§9 dec. 15) |
| `compiler/expr_object.rs:26-39` (identical at head) | **object-literal accessor keys** — `compile_accessor` matches only `PropertyKey::Identifier` (`:27`) and `PropertyKey::Literal(Literal::String)` (`:32`); every other key kind falls to `_ => { fc.emit(Op::Pop); }` (`:37-39`). The key expression of a computed accessor is **never compiled at all** | `Op::Pop` — no property defined, and for a computed key **the key expression never runs** | silent no-op (class 2). Probe-measured at `39bbdb1b`: `let n=0; ({ get [++n]() { return 7 } }); n` → **`0`** · `let k='p'; let o={ get [k]() { return 7 } }; o.p` → **`undefined`** · `let n=0; ({ set [++n](v) {} }); n` → **`0`** (setter half) · `let o={ get 1() { return 7 } }; o[1]` → **`undefined`**, so this is **not limited to computed keys** — every non-Identifier / non-String-literal key is swallowed. (`{get true(){}}` **works**: `true` in key position parses as `PropertyKey::Identifier`, not `Literal::Boolean` — do not claim boolean keys are broken.) Contrast, same file: the sibling `PropertyKind::Init` arm (match at `:83`) **is** exhaustive over `PropertyKey` — `let o={1:'x'}; o[1]` → `'x'` (exhaustive is not the same as correct: its BigInt/RegExp arm is the `:117-121` row above). ECMA-262 §13.2.5.6 PropertyDefinitionEvaluation | T1 | `#11-vm-property-key-lowering-unification` |
| `compiler/stmt_destructure.rs:91` match, `:117-119` catch-all (`:92` / `:118-120` at head) | **object-pattern destructuring keys** — matches `PropertyKey::Identifier` (`:92` base / `:93` head), `Literal::String` (`:98`/`:99`) and `Computed` (`:104`/`:105`); every other key kind falls to `_ => { fc.emit(Op::PushUndefined); }` | `Op::PushUndefined` instead of a `GetProp`/`GetElem` | silent no-op (class 1 — **missed by pass 2**). Probe-measured at `39bbdb1b`: `const {1: a} = {1: 'x'}; a` → **`undefined`** (spec `'x'`). Identifier, string-literal and computed keys all work (`{p: a}` / `{'s': a}` / `{[k]: a}` → `'x'`), so this narrows — not deletes — §1.2's "destructuring **declarations** confirmed correct". Governing: ECMA-262 **§14.3.3.1** Runtime Semantics: PropertyBindingInitialization, whose step 1 is `Let propertyKey be ? Evaluation of PropertyName` = **§13.2.5.5**, where `LiteralPropertyName : NumericLiteral` is `Let number be the NumericValue…; Return ! ToString(number)` (the assignment-form counterpart on 0bc's path is **§13.15.5.3** PropertyDestructuringAssignmentEvaluation) | T1 | `#11-vm-property-key-lowering-unification` |
| `compiler/stmt_destructure.rs:134` match, `:149-150` catch-all (`:135` / `:150-151` at head) | **object-pattern REST exclusion** — the second `PropertyKey` match in the same function, deleting already-destructured keys from the rest object. Matches `Identifier` (`:135` base / `:136` head) and `Literal::String` (`:142`/`:143`); `_ => {}` under a comment reading "Computed/other keys: handled below via temp locals" — which is true of `Computed` (the `computed_key_slots` loop below) and false of every other kind | nothing emitted, so the key is never deleted from the rest object | silent wrong **value**, not a no-op: the destructured key survives into the rest binding. Probe-measured at `39bbdb1b`: `const {1: a, ...r} = {1:'x', z:'y'}` leaves `r` as **`{"1":"x","z":"y"}`**; the Identifier and computed spellings both give `{"z":"y"}`. Governing: ECMA-262 **§14.3.3.2** Runtime Semantics: RestBindingInitialization, which *receives* `excludedNames (a List of property keys)` as an argument — the set is built by **§14.3.3.1**, which returns « propertyKey » (the assignment-form counterpart on 0bc's path is **§13.15.5.4** RestDestructuringAssignmentEvaluation) | T1 | `#11-vm-property-key-lowering-unification` |
| `parser/object.rs:200-203` + `compiler/expr_object.rs:84-87` (at `658cc302`) | **`#x` is accepted as an object property key** — `({#x: 1})` and `const {#x: y} = o`. `parse_property_key` (`parser/object.rs:176`) has a `TokenKind::PrivateIdentifier` arm returning `PropertyKey::PrivateIdentifier`, and the arm reads only the token: nothing in it looks at which of the helper's three callers is asking (`git grep -n 'parse_property_key' 658cc302 -- crates/script/elidex-js/src` → the class-member path `parser/function.rs:518`, the object literal `parser/object.rs:51`, the binding pattern `parser/pattern.rs:123`, plus the definition) | object literal: `expr_object.rs:84` groups `PrivateIdentifier` with `Identifier` and emits `Op::DefineProperty`, so an ordinary property is defined; binding pattern: the key kind reaches the `stmt_destructure.rs` catch-all of the row two above | silent wrong **program** — the spelling has no meaning for the lowering to be wrong about. Probe-measured at `0c10870b`: `Object.keys({#x: 1})` → **`"x"`** (the `#` is dropped), `({#x: 1})['x']` → **`1`**, `({#x: 1})['#x']` → **`undefined`**; and `const o = {'#x': 7, x: 9}; const {#x: y} = o; y` → **`undefined`**. Governing grammar: **§13.2.5** Object Initializer derives a key only through `PropertyName : LiteralPropertyName \| ComputedPropertyName` with `LiteralPropertyName : IdentifierName \| StringLiteral \| NumericLiteral`, and **§14.3.3** Destructuring Binding Patterns reaches the same nonterminal through `BindingProperty : PropertyName : BindingElement` (`webref body ecma262 sec-object-initializer` / `sec-destructuring-binding-patterns`) — so neither context can derive `#x` and both are a Syntax Error. `PrivateIdentifier` is an alternative of `ClassElementName`, which is why the arm stays and gains a condition on its calling context rather than being deleted | T1 | **0ba** |
| `compiler/expr_class.rs:588-590` (`:590-592` head) + `:568`, `:621` (`:570`, `:623` head) | ⚠ **the previous row here was FALSE at head *and* at baseline** and is corrected. It claimed `class{get [k](){}}` is a `CompileError` — a "loud reject — not a defect". Measured at `39bbdb1b`: `let k='p'; class C { get [k]() { return 7 } }; (new C).p` → **`7`** · `let n=0; class C { get [++n]() { return 7 } }; n` → **`1`** (the key side effect runs) · `class C { get 1() { return 7 } }; (new C)[1]` → **`7`**. Computed class members never reach the cited guard: they are handled by a different, earlier path — `expr_class.rs:523` `if computed { … MethodKind::Get => Op::DefineComputedGetter, Set => Op::DefineComputedSetter, Method \| Constructor => Op::DefineComputedMethod … }` — which exists identically at `f7d9b5ce:523` | the `if computed { return Err(…) }` guard at `:588-590` is **unreachable dead code**: `grep -rn 'emit_class_member_name_op' crates/script/elidex-js/src/compiler/` gives both call sites passing `computed: false` (`:423` and `:575` at baseline, `:423` and `:577` at head). The two `_ =>` arms at `:568` / `:621` **are** reachable, but only for key kinds neither preceding arm covers — measured, `class C { get 1n() { return 7 } }` → `CompileError "unsupported class member key type"` while `class C { get 1() {…} }` succeeds | the guard is an **I-4 connect-or-delete**; the two `_ =>` arms are a loud **BigInt-literal-key** reject (a RegExp key in class position does not reach them — it is a *parse* error, measured) | I-4 / T2 | `#11-vm-property-key-lowering-unification` (guard) · 2 |
| `compiler/stmt.rs:882` | `for (obj.prop in …)` | `Pop` | silent no-op | T1 | 0bc — **0a ✅ loud** |
| `compiler/expr_member.rs:92` | **`(o.m)()`** — `compile_call_expr` matches `ExprKind::Member` on the **raw** callee, so a parenthesized callee takes the plain-call branch | `Op::Call` (no receiver pushed) | **`this` is `undefined`** instead of `o` ([C20] step 1.a.i) | T1 | **0bb** (the shared `peel_paren` chokepoint — §9 dec. 14) |
| `compiler/expr_member.rs:180-187` (at `658cc302`) | **`o.m?.(…)`** — `prev_is_member` is `i > 0 && matches!(chain[i - 1], OptionalChainPart::Member { .. })`, so an optional call that is the chain's *first* part takes the `else` branch | `Op::Call` at `:186` (no receiver pushed); `o?.m(…)` takes `Op::CallMethod` at `:184` | **`this` is `undefined`** instead of `o` ([C20] step 1.a.i) | T1 | **0bb** — sibling of the `expr_member.rs:92` row above: same observable (a callee that is a member Reference loses its receiver), **two mechanisms, not one chokepoint**. `(o.m)()` is `compile_call_expr`'s `if let ExprKind::Member` test on the raw callee, defeated by the `ExprKind::Paren` wrapper (`ast.rs:299` at `658cc302` and at `6edda6f2`); this row is `compile_optional_chain_expr`'s `prev_is_member`, which reads `chain[i-1]` and never consults the chain's *base*. `peel_paren` does not reach this one — 0bb owns both because the receiver contract is one, and fixing either alone leaves the other |
| `compiler/stmt.rs:99-103` | **`for await (x of it)`** — the `is_await: _` discard | compiles identically to sync `for-of` | silent **wrong protocol** (sync iterator used for an async iterable) | T1 | 6 |
| `compiler/expr_member.rs:60-64` | `f(a×256)` | **`assert!` → panic** | process abort | T0 | 1b |
| `compiler/stmt.rs:424-439` — **not** an `f7d9b5ce` row: derived at `658cc302` and re-derived at HEAD `d49465d0`, where `git show <rev>:crates/script/elidex-js/src/compiler/stmt.rs \| grep -n 'if let Some(param_id) = catch.param'` gives **424** at both, and `'Op::Pop); // pop exception'` gives **439** at both | **destructured catch parameter** — `catch ({x})`, `catch ([a])` | the parameter is stored only under `if let PatternKind::Identifier(atom) = &pattern.kind` (`:426`); no other `PatternKind` has an arm, so nothing is emitted for the pattern's bound names, and the arm then runs `fc.emit(Op::Pop)` (`:439`) **unconditionally**, discarding the exception either way | silent no-op: `try { throw {x: 1} } catch ({x}) { … }` never initializes `x`. ECMA-262 **§14.15.2** Runtime Semantics: CatchClauseEvaluation **step 5** — `Let status be Completion(BindingInitialization of CatchParameter with arguments thrownValue and catchEnv)` (§-number from `webref aoid ecma262 CatchClauseEvaluation`; step text from `webref body ecma262 sec-runtime-semantics-catchclauseevaluation`) | T1 | **0bc** (gained here — see §5) |
| `compiler/expr_assign.rs:215-220` | `AssignTarget::Pattern` | `Op::Pop`, claims to "fail explicitly" but does not | **dead** — parser never constructs this variant | I-4 | 0bc — **0a ✅ loud** |
| `compiler/expr_class.rs:402-427` (at `658cc302`) | **`class C { static x = this }`** — the static initializer is compiled with the **enclosing** `FunctionCompiler` (`compile_expr(fc, …)` at `:411` computed-key branch, `:418` otherwise), so `ExprKind::This` inside it reads the surrounding function's receiver. Contrast the static **block** arm at `:448-471`, which builds a child compiler and invokes it with the ctor as `this` (`:454` `Op::Dup`, `:470` `Op::CallMethod`) | the initializer's value evaluated against the wrong receiver | `C.x === C` → **`false`**. ECMA-262 **§15.7.10** ClassFieldDefinitionEvaluation and **§7.3.32** DefineField call the initializer with the field receiver. ⚠ Narrows §1.2's "static fields/methods, no action" and §8's treatment of that facet of `#11-step9-class-extras` as discharged — the probe `static x = 1` cannot distinguish the receivers | T1 | **2ac** |
| `vm/dispatch_objects.rs:135-186` (at `658cc302`) | **`{...'ab'}`**, **`const {...r} = 'ab'`** — `op_spread_object` copies only when source **and** destination are both `JsValue::Object` (`:137`); any other source falls out of the `if let` and the handler returns `Ok(())` | nothing copied | **empty object** instead of the enumerable index properties. ECMA-262 **§7.3.25** CopyDataProperties returns early only for `undefined`/`null` (step 1) and otherwise applies `ToObject` (step 2). ⚠ **Layer-B live defect in a connected opcode** — outside the scope stated above (the passes read `compiler/`) and outside §2.3's zero-emit partition; it is tabled here because the handler was read directly, which is the mechanism that section names | T1 | **O** |
| `vm/dispatch_class.rs:29-37` + `vm/value.rs:588-599` (at `658cc302`) | **`new.target` read inside an arrow** — `op_new_target` reads only the current frame's `CallMode` (`:32-33`), and `FunctionObject` carries `captured_this` (`value.rs:598`) with no lexical `new.target` counterpart; invoking an arrow pushes a `CallMode::Call` frame (`vm/interpreter.rs:648`, `:683`, `:732`) | `JsValue::Undefined` | `function F(){ this.ok = (() => new.target)() === F } new F().ok` → **`false`**. ECMA-262 **§9.4.5** GetNewTarget resolves the surrounding function environment, which for an arrow is the enclosing function's. ⚠ Narrows §1.1's `new.target` row | T1 | **N** |
| `compiler/stmt_destructure.rs:36-81` (at `658cc302`) | **an array binding pattern never closes its iterator** — `const [x] = it`. The `PatternKind::Array` arm emits `Op::GetIterator` (`:38`), takes one value per element, and where the pattern has no rest element ends at `fc.emit(Op::Pop); // pop iterator` (`:80`). No `Op::IteratorClose` is emitted from this file at all: `git grep -l 'Op::IteratorClose' 658cc302 -- crates/script/elidex-js/src/compiler/` returns `expr_yield_star.rs`, `stmt.rs` and `stmt_loop.rs`, and not this one | `Op::Pop` on an unexhausted iterator | `it.return()` is never called on the **normal early exit**. ECMA-262 **§8.6.2** BindingInitialization, production `BindingPattern : ArrayBindingPattern`, **step 3**: "If iteratorRecord.[[Done]] is false, return ? IteratorClose(iteratorRecord, result)" (AO number from `webref aoid ecma262 BindingInitialization`, step text from `webref body ecma262 sec-runtime-semantics-bindinginitialization`). The arm is `compile_destructure_pattern`, reached from parameter patterns (`expr_function.rs:55`), `var`/`let`/`const` declarations (`stmt.rs:121`), the `for` head (`stmt_loop.rs:251`) and the for-in/of head (`stmt_loop.rs:321`) — `git grep -n compile_destructure_pattern 658cc302 -- crates/script/elidex-js/src`. **Derived from the arm; not probe-measured** (this PR edits no `.rs` file, and the crate has no runner binary) | T1 | **0bc** |
| `compiler/stmt.rs:284-291` + `:224-256` (at `658cc302`) | **`outer: { break outer; }` does not compile** | the `Labeled` arm records the label as `fc.loop_stack.len()` (`:287-288`) and emits no patch point for the end of an ordinary labelled statement; the `Break` arm reads that index back and rejects it when no loop is open (`:244-249`, message `"label '{label_name}' is not associated with a loop or switch"`) | a labelled statement outside any loop records depth `0` while `fc.loop_stack.len()` is `0`, so the guard `loop_idx >= fc.loop_stack.len()` holds and **valid code is rejected at compile time**; with a loop nested inside the labelled block the same index resolves to `fc.loop_stack[0]` (`:251`), so the patch attaches to the loop rather than to the block. ECMA-262 **§14.13.4** LabelledEvaluation, production `LabelledStatement : LabelIdentifier : LabelledItem` **step 4** — a break completion whose `[[Target]]` is the label is caught at the labelled statement itself, so a labelled statement is a break target whether or not it is a `BreakableStatement` (§-number from `webref aoid ecma262 LabelledEvaluation`, step text from `webref body ecma262 sec-runtime-semantics-labelledevaluation`). **Derived from the arms; not probe-measured** | T2 | **L** |
| `scope/visitor.rs:79-124` + `compiler/stmt_loop.rs` (at `658cc302`) | **no per-iteration lexical environment for a loop's `let` head** — `let fs=[]; for (let i=0;i<2;i++) fs.push(()=>i)` and `for (let x of [1,2])` alike | `visit_stmt` pushes exactly one `ScopeKind::Block` for the whole `for` (`:85`) and one for the whole `for-in`/`for-of` (`:114`), and the compiler stores the head binding into that one scope's local — `compile_for`'s init at `stmt_loop.rs:245-260`, the for-in/of head at `:321` — with no environment clone between iterations. `git grep -c CreatePerIterationEnvironment 658cc302 -- crates/script/elidex-js/src` exits **1** (no match) | one binding shared by every iteration, so every closure created in the body observes the last value. ECMA-262 **§14.7.4.4** CreatePerIterationEnvironment, reached from **§14.7.4.2** ForLoopEvaluation, builds a fresh `NewDeclarativeEnvironment` per iteration and copies the previous iteration's value in (steps 1.d, 1.e.ii-iii); **§14.7.5.7** ForIn/OfBodyEvaluation **step 8.h.iii** takes `iterationEnv = NewDeclarativeEnvironment(oldEnv)` per iteration for a lexical head (both read via `webref body ecma262`). **Derived from the scope pass and the arms; not probe-measured** | T1 | **E** |
| `vm/dispatch_iter.rs:141-239` + `:243-273` (at `658cc302`) | **`for-in` enumerates a front-loaded snapshot** | `op_for_in_iterator` collects keys once into `ForInState { keys, index: 0 }` (`:231-236`) and inserts a name into `seen` only inside `if attrs.enumerable && seen.insert(sid)` (`:188` own storage, `:220` prototype chain), so a **non-enumerable own key never marks the name visited**; `op_for_in_next` (`:243`) hands back `state.keys[state.index]` (`:255`) with no re-check of existence or enumerability | an inherited enumerable property shadowed by a non-enumerable own property of the same name is still enumerated, and a key deleted before its turn is still yielded. ECMA-262 **§14.7.5.9** EnumerateObjectProperties: "The values of [[Enumerable]] attributes are not considered when determining if a property of a prototype object has already been processed", and "A property that is deleted before it is processed by the iterator's next method is ignored" — and its informative generator adds the key to `visited` **before** testing `desc.enumerable` (`webref body ecma262 sec-enumerate-object-properties`). A live defect in a **connected** handler, so §1.0's derivation did not produce it: it is recorded here under this section's per-slice-sweep rule and owned by its own §8 slot rather than by a §5 slice. **Derived from the handlers; not probe-measured** | T1 | `#11-vm-for-in-enumeration-semantics` |
| `parser/arrow.rs:104-119` + `vm/value.rs:588-599` + `vm/dispatch_class.rs:115-119` (at `658cc302`) | **`super` reached through an arrow** — `(() => super())()` in a derived constructor, `(() => super.x)()` in a method | the parser deliberately admits it: the arrow parser saves `in_method` / `in_constructor` / `in_static_block` / `in_derived_constructor` and restores them inside `with_function_context`, under the comment "Arrow functions inherit super context". Nothing carries that inheritance past the parser: `git grep -n 'in_method\|in_derived_constructor\|in_static_block' 658cc302 -- crates/script/elidex-js/src \| grep -v '/parser/'` returns only `install_body_mixin_methods` substring hits in `vm/globals.rs` and `vm/host/body_mixin.rs`, while the same pattern inside `parser/` matches across six files, so it discriminates. `FunctionObject` (`value.rs:588-599`) has `captured_this` and no home-object counterpart — `git show 658cc302:…/vm/value.rs \| sed -n '588,599p' \| grep -ci home` → **0**, against **3** for `captured` over the same range and **4** for `home` over the whole file, so the pattern discriminates — and a frame's `home_class` is `Some` only when `compiled.is_class_ctor` (`vm/interpreter.rs:802`) or hard-coded `None` on the `call_internal` entry (`:589`, whose own comment says so) | `dispatch_super_inner` (`vm/dispatch_class.rs:115`) reads the **current** frame's `home_class` and raises `VmError::syntax_error("'super' keyword unexpected here")` when it is `None` (`:117-119`); an invoked arrow's frame is not a class-ctor frame, so that is the branch taken. ECMA-262 **§13.3.7.2** GetSuperConstructor step 1 and **§13.3.7.3** MakeSuperPropertyReference step 1 both begin at **§9.4.3** GetThisEnvironment, whose step 2 walks outward until a record's `HasThisBinding()` is true — an arrow's is not — so both spellings resolve against the enclosing method or constructor (§-numbers from `webref aoid ecma262 GetSuperConstructor` / `MakeSuperPropertyReference` / `GetThisEnvironment`; steps from `webref body ecma262 sec-getsuperconstructor` / `sec-makesuperpropertyreference` / `sec-getthisenvironment`). ⚠ Narrows §1.1's `super(...args)` row, and Slice 3's frame-state audit is stated over the direct-method entry points, so it does not reach this. **Derived from the arms; not probe-measured** | T2 | **S** |
| `compiler/stmt.rs:203-205` (at `658cc302`; the same pair at `:227` / `:238` in the `Break` arm, under the same comment) | **a `for-of` left by `return` or `break` from inside an enclosing `try`/`finally`** | `emit_pending_finally_bodies` first, `emit_iter_close_range` second, so the finally bodies' ops precede the `Op::IteratorClose` ops in one instruction sequence (`fn emit_iter_close_range` is `stmt.rs:657`; its `fc.emit(Op::IteratorClose)` at `:661` is one of the two statement-lowering emits §6.2a-3 partitions, and neither of that section's greps sees this defect — they find the emit, not its position relative to the finally bodies: `git grep -n 'emit(Op::IteratorClose)' 658cc302 -- crates/script/elidex-js/src/compiler` returns `:661` and never `:203-205`, while `git grep -n 'emit_iter_close_range(\|emit_pending_finally_bodies(' 658cc302 -- crates/script/elidex-js/src/compiler` returns both call sites, so a discriminating probe exists) | the close and the enclosing `finally` run in the **wrong order**, and because the ops are sequential a throw from the finally body transfers control before the close ops execute at all, so `.return()` is never called. ECMA-262 **§14.7.5.7** ForIn/OfBodyEvaluation performs the close **inside the loop's own evaluation** — step 8.9.6 on an abrupt lhs completion and step 8.13.5 when `LoopContinues(result, labelSet)` is false, each `Return ? IteratorClose(iteratorRecord, status)` — so the completion reaches an enclosing `finally` only after the iterator is closed. The order is **nesting-dependent**: for a `try`/`finally` *inside* the loop body the finally is part of step 8.10's "Evaluation of stmt" and therefore precedes the same close (§-number from `webref aoid ecma262 ForIn/OfBodyEvaluation`, steps from `webref body ecma262 sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`). ⚠ The in-tree comment above the pair states one nesting's order as unconditional and attributes it to the spec — a code-side observation for the owning slice, not a claim this plan carries. **Derived from the arms; not probe-measured** | T1 | **Pd** |
| `compiler/expr_function.rs:31-66` + `scope/visitor.rs:442-486` (at `658cc302`; `git diff --stat 658cc302 909a77cb -- crates/` is empty, so both readings hold at this document's HEAD) | **a parameter binding is never uninitialized, and the body shares the parameters' environment** — `function f(a=b,b=1){}` and `function f(a=x){var x}` | `compile_param_prologue` lowers each default as read-the-slot / compare-to-`undefined` / conditionally store (`Op::GetLocal` `:41`, `Op::PushUndefined` `:42`, `Op::StrictEq` `:43`, `Op::JumpIfFalse` `:44`, `Op::SetLocal` `:46`), and the caller has already filled every positional slot before the prologue runs, so a default initializer reads a later parameter's slot as an ordinary value; the arm's own comment at `:52` states it — "`Var` kind selects no `InitLocal` — param bindings carry no TDZ". `visit_function` pushes exactly one `ScopeKind::Function` (`:456`) and runs `visit_params` (`:479`) and the body statements (`:481-483`) inside it, so a body `var` and a parameter default resolve against the same scope | a default initializer reads a later parameter's value instead of raising, and reads a body-local `var` instead of the outer binding of that name. ECMA-262 **§10.2.11** FunctionDeclarationInstantiation orders it otherwise: step 21.3.a creates each parameter binding with "Perform ! envRecord.CreateMutableBinding(paramName, false)" and initializes it only under step 21.3.b's `hasDuplicates` case, so the bindings are uninitialized while the defaults run; step 28 performs "IteratorBindingInitialization of formals", which is what initializes them, left to right; and step 30.2 takes "variableEnv be NewDeclarativeEnvironment(envRecord)" for the body whenever `hasParamExprs` (step 8, ContainsExpression of formals) is true, under step 30.1's note that a separate record "ensure[s] that closures created by expressions in the formal parameter list do not have visibility of declarations in the function body" (§-number from `webref aoid ecma262 FunctionDeclarationInstantiation`, steps from `webref body ecma262 sec-functiondeclarationinstantiation`). A live defect in fully emitted code, so §1.0's derivation did not produce it: recorded here under this section's per-slice-sweep rule and owned by its own §8 slot rather than by a §5 slice — 0bc's parameter coverage is binding-pattern storage and iterator closing, a different mechanism. **Derived from the arms and the scope pass; not probe-measured** | T1 | `#11-vm-parameter-environment-instantiation` |
| `vm/dispatch_objects.rs:352-384` (at `658cc302`) | **`instanceof` never tests its right operand for callability** — `({}) instanceof {prototype:{}}` | `op_instanceof` rejects a non-Object right operand (`:353-357`), looks `@@hasInstance` up on it (`:360-361`) and, if that lookup returns anything at all, resolves and calls it (`:362-364`); otherwise it falls straight through to the prototype-chain walk (`:367-383`) with no callability test in between. `coerce::get_property` (`vm/coerce.rs:907`) returns `Some(PropertyResult::Data(v))` for a **present** property whatever its value, `resolve_property` (`vm/ops_property.rs:38-41`) passes a data slot through unchanged, and `call_value` (`vm/ops.rs:665-674`) raises `TypeError "not a function"` for a non-Object callee | a plain object carrying a `prototype` property answers `instanceof` with `false` where the AO throws, and an `@@hasInstance` present with the value `null` throws where the AO ignores it. ECMA-262 **§13.10.2** InstanceofOperator orders the two: step 2 is "Let instOfHandler be ? GetMethod(target, %Symbol.hasInstance%)" — GetMethod yields **undefined** for a `null` or `undefined` value, so step 3's "is not undefined" test routes those to the default path — step 4 is "If IsCallable(target) is false, throw a TypeError exception", and only step 5 reaches **§7.3.21** OrdinaryHasInstance, whose step 1 returns false for a non-callable and whose step 5 throws when `Get(ctor, "prototype")` is not an Object, where `:373`'s `if let` takes the `Ok(false)` path instead (§-numbers from `webref aoid ecma262 InstanceofOperator` / `OrdinaryHasInstance`, steps from `webref body ecma262 sec-instanceofoperator` / `sec-ordinaryhasinstance`). A live defect in a **connected** handler, so §1.0's derivation did not produce it: recorded here under this section's per-slice-sweep rule and owned by its own §8 slot rather than by a §5 slice — it is neither a compiler-arm stub nor a builtin-property absence. Read from the handler, not by grep: `is_callable` returns **0** hits over the whole of `vm/dispatch_objects.rs`, which cannot distinguish "the check is missing" from "the check is spelled otherwise", while `grep -rln 'is_callable' crates/script/elidex-js/src/vm/` reaches `vm/ops.rs` and `vm/coerce.rs` among other siblings, so the name is in use in the tree. **Derived from the handler; not probe-measured** | T1 | `#11-vm-instanceof-callable-check` |
| `compiler/expr_assign.rs:80-88` (at `2039766a`; `git diff --stat 658cc302 2039766a -- crates/` is empty, so the reading holds at `658cc302` too) | **a write to a `const` binding is rejected at compile time** — `try { const x = 1; x = 2 } catch (e) { 42 }` | `compile_identifier_store`'s `VarLocation::Local` arm returns `CompileError { message: "Assignment to constant variable '…'" }` (`:81-88`) before emitting anything; its sibling check two lines below takes the other route for the same lookup, emitting `Op::CheckTdz` (`:90-91`) rather than rejecting | the whole script yields no bytecode, so every unrelated statement is lost and the `catch` is never reached. ECMA-262 puts the failure at run time: **§13.15.2** Runtime Semantics: Evaluation reaches **§6.2.5.6** PutValue, whose Declarative Environment Record path is **§9.1.1.1.5** SetMutableBinding, step 5 — "Assert: This is an attempt to change the value of an immutable binding", then 5.b "If strict is true, throw a TypeError exception" (§-numbers from `webref aoid ecma262 PutValue` / `SetMutableBinding` + `webref heading ecma262 9.1.1.1`, steps from `webref body ecma262 sec-declarative-environment-records-setmutablebinding-n-v-s`). This is the shape §17's private-store CRIT had and §9 dec. 5 resolved against measured evidence — loud but **not scoped** — so the disposition is dec. 5's, not a fresh one: a runtime throw through the `Op::ThrowUnsupported` mechanism (an engine-constructed error raised through `throw_error`, catchable and local), and `CompileError` reserved for what the spec itself rejects before execution. §1.0's derivation does not produce this row — §2.2's pass 3 flags an arm that neither emits an op **nor** returns `CompileError`, and this arm returns one — so it is recorded here under this section's per-slice-sweep rule and owned by its own §8 slot. **Derived from the arm; not probe-measured** | T2 | `#11-vm-const-assignment-runtime-throw` |
| `compiler/stmt.rs:262-282` (at `2039766a`; contrast `:203-205` in the `Return` arm and `:227` / `:238` in the `Break` arm) | **an inner `for-of` left by a labelled `continue` is never closed** — `outer: for (;;) { for (const x of it) { continue outer } }` | the `Continue` arm emits the pending finally bodies (`:263`) and the jump (`:264`) and pushes the patch onto the targeted loop's `continue_patches` (`:273`); it never calls `emit_iter_close_range`, which the `Return` arm calls over `0..loop_stack.len()` (`:205`) and the `Break` arm over `target..loop_stack.len()` (`:238`), and which is what emits `Op::IteratorClose` for the for-of frames in its range (`:657-664`, the emit at `:661`). The absence is probed, not assumed: `grep -rn 'emit_iter_close_range' crates/script/elidex-js/src --include='*.rs'` returns the definition and exactly those two call sites (plus one test comment naming it), so the pattern reaches the helper wherever it is used and reaches nothing in `:262-282`. Nor is the close recovered downstream — the jump is patched onto the **targeted** loop's `continue_patches` and lands at its `loop_start` (`compiler/stmt_loop.rs:123`), bypassing the inner `for-of`'s exit path (`:125-128`, whose `:128` comment reads "normal exhaustion: discard iterator without calling .return()") and its catch handler, the only other for-of `Op::IteratorClose` emit (`:143-147`), which runs on a throw | `it.return()` is never called on the way out. ECMA-262 **§14.7.5.7** ForIn/OfBodyEvaluation step 8.13 tests `LoopContinues(result, labelSet)` and on false reaches step 8.13.5, `Return ? IteratorClose(iteratorRecord, status)`; **§14.7.1.1** LoopContinues returns true for a continue completion only when step 3's `[[Target]]` is empty or step 4's labelSet contains it, so the loop the label names continues while the loops inside it close (§-numbers from `webref aoid ecma262 'ForIn/OfBodyEvaluation'` / `LoopContinues`, steps from `webref body ecma262 sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset` / `sec-loopcontinues`). The range this needs is therefore not the `Break` arm's: `break` leaves the targeted frame and `continue` re-enters it. **Derived from the arms; not probe-measured** | T1 | **Pd** |
| `compiler/stmt.rs:227` and `:263` (at `b5356098`; `git diff --stat 658cc302 b5356098 -- crates/` is empty, so the reading holds at `658cc302` too) | **a `break` or `continue` whose target lies inside the enclosing `try` still runs the finalizer** — `try { while (true) { break } log.push('after') } finally { log.push('fin') }` | the `Break` arm's first statement is `emit_pending_finally_bodies(fc, prog, analysis, func_scopes)?` (`:227`) and the `Continue` arm's is the same call (`:263`); neither is guarded by any test of where the jump lands, and the helper (`:678-696`) takes no target — it walks the whole of `fc.finally_stack`, `for i in (0..saved.len()).rev()` (`:686`) — so every enclosing finalizer is emitted whatever the target. The arms do compute a target for the *other* obligation on the same path (`target_idx` at `:231-236`, the `emit_iter_close_range` lower bound at `:238`), so the information a guard needs is already in scope at the site that lacks it. Contrast the sibling `Return` arm (`:203`), where emitting all of them is correct because a return does leave every enclosing `try` | the finalizer's ops run at the jump **and** again when the try block completes, so `fin` is logged before `after` and once more after it — silent wrong **order**, plus a duplicated finalizer run. ECMA-262 **§14.15.3** Runtime Semantics: Evaluation, production `TryStatement : try Block Finally`: step 1 is "Let blockResult be Completion(Evaluation of Block)" and step 2 "Let finallyResult be Completion(Evaluation of Finally)"; the `try Block Catch Finally` production orders the same two at steps 1 and 4. So the Finalizer is evaluated only after the Block's evaluation **completes**, and a jump whose target is inside the Block does not complete it (§-number from `webref heading ecma262 14.15.3`, steps from `webref body ecma262 sec-try-statement-runtime-semantics-evaluation`). **Derived from the arms; not probe-measured** | T1 | **Pd** |
| `compiler/expr_assign.rs:45` + `vm/dispatch.rs:155-160` + `vm/value.rs:952-957` (at `2039766a`) | **a TDZ check is lost through a captured binding** — `let f = () => x; f(); let x = 1` | `compile_identifier_load`'s `VarLocation::Local` arm consults `needs_tdz` and emits `Op::CheckTdz` before `Op::GetLocal` (`:34-42`), while its `VarLocation::Upvalue` arm is a bare `fc.emit_u16(Op::GetUpvalue, idx)` (`:45`). Both `Op::CheckTdz` emit sites in the crate are inside that Local arm (`grep -rn 'Op::CheckTdz' crates/script/elidex-js/src --include='*.rs'` → two emit sites, `compiler/expr_assign.rs:42` and `:91`, both inside that Local arm; every remaining hit is the opcode's own definition, its documentation, its dispatch arm or a test, so the pattern reaches the name's other uses and still finds no emit outside the Local arm). The runtime half cannot recover it: the `Op::GetUpvalue` arm reads `upvalue_ids[idx]` and pushes `read_upvalue` (`vm/dispatch.rs:155-160`), `read_upvalue` returns the stack slot or the closed value (`vm/ops.rs:597-602`), `UpvalueState` carries exactly `Open { frame_base, slot }` and `Closed(JsValue)` (`vm/value.rs:952-957`) with no uninitialized state, and the TDZ bits live on `CallFrame` (`is_in_tdz`, `vm/value.rs:1106`), which an upvalue read never consults | a captured binding read before its declaration yields `undefined` instead of raising. ECMA-262 **§9.1.1.1.6** GetBindingValue step 2 — "If the binding for name in envRecord is an uninitialized binding, throw a ReferenceError exception" — under a preamble that adds "regardless of the value of strict" (§-number from `webref heading ecma262 9.1.1.1`, steps from `webref body ecma262 sec-declarative-environment-records-getbindingvalue-n-s`). Cross-layer (compiler emit + upvalue representation + dispatch), so §1.0's derivation does not produce it: §2.2's sweep is over `compiler/` arms and this arm emits an op. Recorded here under the per-slice-sweep rule and owned by its own §8 slot. **Derived from the arms and the runtime reads; not probe-measured** | T1 | `#11-vm-upvalue-tdz-check` |
| `scope/visitor.rs:458-463` + `compiler/expr.rs:210-214` + `compiler/expr_function.rs:116-133` (at `2039766a`) | **a named function expression has no binding to its own name** — `let f = function g(){ return g }; f() === f` | `visit_function` adds the name as an inner `BindingKind::Function` binding for the expression form (`:458-463`, under the `:439` comment "E13: `is_expression` controls inner name binding"), and `compile_nested_function` then initializes every `BindingKind::Var \| BindingKind::Function` local above the parameter count to `undefined` (`:120` filter; `Op::PushUndefined` / `Op::SetLocal` / `Op::Pop` at `:130-132`). Nothing stores the closure into that slot: the `ExprKind::Function` arm is `compile_nested_function` → `add_constant` → `fc.emit_u16(Op::Closure, idx)` and ends (`compiler/expr.rs:210-214`). Counting `Op::Closure` emits proves nothing — several files emit it — so the probe is what **follows** each: `grep -n -A 3 'emit_u16(Op::Closure, idx)' crates/script/elidex-js/src/compiler/expr.rs crates/script/elidex-js/src/compiler/mod.rs crates/script/elidex-js/src/compiler/expr_function.rs` returns the two *declaration* sites followed by `resolve_identifier` and a store (`compiler/mod.rs:93-96`, `compiler/expr_function.rs:180-183`) and the two *expression* sites followed by the arm's closing brace (`compiler/expr.rs:213-214`, `:219-220`), so it separates the paths rather than counting the opcode | the self-name reads `undefined`, so recursion through it fails and the identity test is false. ECMA-262 **§15.2.5** InstantiateOrdinaryFunctionExpression, production `FunctionExpression : function BindingIdentifier ( FormalParameters ) { FunctionBody }`: step 4 "Let funcEnv be NewDeclarativeEnvironment(outerEnv)", step 5 "Perform ! funcEnv.CreateImmutableBinding(name, false)", step 8 creates the closure against `funcEnv`, and step 11 "Perform ! funcEnv.InitializeBinding(name, closure)" — an **immutable** binding in an environment of the expression's own, against a mutable one in the function's own scope here (§-number from `webref aoid ecma262 InstantiateOrdinaryFunctionExpression`, steps from `webref body ecma262 sec-runtime-semantics-instantiateordinaryfunctionexpression`). The arm emits an op, so §1.0's derivation does not produce it: recorded here under the per-slice-sweep rule and owned by its own §8 slot. **Derived from the arms and the scope pass; not probe-measured** | T1 | `#11-vm-named-function-expression-binding` |
| `parser/primary.rs:200-203` + `parser/function.rs:281-292`, `:370-371` + `scope/visitor.rs` (at `b5356098`; `git diff --stat 658cc302 b5356098 -- crates/` is empty, so the readings hold at `658cc302` too) | **a private reference is never validated against a declared private name** — `({}).#x`, and `class C { m() { return this.#missing } }` | `parse_member_prop`'s `TokenKind::PrivateIdentifier` arm advances and returns `MemberProp::PrivateIdentifier(name)` (`primary.rs:200-203`) without consulting anything about which private names are in scope. The one private-name set the parser builds is for **declarations**, not references: `parse_class_body` allocates `private_names: [HashSet<Atom>; 3]` (`function.rs:370-371`) and fills it through `check_private_name_dup` (`:281-292`), whose `match` inserts only from `ClassMemberKind::PrivateField` and `PrivateMethod` and `return`s for every other member kind; the set is a local of that loop and is dropped at `:390`, and no reference site reads it. Scope analysis carries no declared-private-name environment either: `git grep -in 'private' b5356098 -- crates/script/elidex-js/src/scope/` returns exactly three lines — `visitor.rs:278`, where `ExprKind::PrivateIn` is grouped with the other single-argument expressions and its `right` visited, and `:546` / `:550`, where `PrivateMethod` / `PrivateField` are traversed for their function or initializer — all child traversals, none of which binds a name. Negative control: the same case-insensitive pattern over `crates/script/elidex-js/src/` reaches **48** files (`git grep -iln 'private' b5356098 -- crates/script/elidex-js/src/ \| wc -l`), so it discriminates rather than matching nothing. The reference reaches lowering — `compiler/expr_member.rs:44` emits `Op::GetPrivate` and `compiler/expr.rs:230` `Op::PrivateIn` at `b5356098` (`git grep -n 'Op::GetPrivate\|Op::PrivateIn' b5356098 -- crates/script/elidex-js/src/compiler/`), the two emit sites the rows above already table | **no SyntaxError anywhere**. What happens next is Slice 5's business and changes with it — today the dispatch stub answers `undefined` / `false` (the `expr_member.rs:44` row above) — but the reference is never *rejected*, so a `#missing` in a branch that never executes is never diagnosed at all, where the spec requires an early error. ECMA-262 **§15.7.7** Static Semantics: AllPrivateIdentifiersValid returns false for `MemberExpression : MemberExpression . PrivateIdentifier` when `names` does not contain the StringValue of the PrivateIdentifier, and likewise for the `CallExpression . PrivateIdentifier`, both `OptionalChain` and `RelationalExpression : PrivateIdentifier in ShiftExpression` productions; its `ClassBody : ClassElementList` production is what supplies the names — "Let newNames be the list-concatenation of names and the PrivateBoundIdentifiers of ClassBody". The rules that **check** it are **§16.1.1** Static Semantics: Early Errors, `ScriptBody : StatementList` ("It is a Syntax Error if AllPrivateIdentifiersValid of StatementList with argument « » is false unless the source text containing ScriptBody is eval code that is being processed by a direct eval") and **§16.2.1.1** Static Semantics: Early Errors, `ModuleBody : ModuleItemList` (the same rule with no eval carve-out). ⚠ **Not §15.7.1** — the class-body early errors do not mention the SDO at all (`webref body ecma262 sec-class-definitions-static-semantics-early-errors \| grep -c 'AllPrivateIdentifiersValid'` → **0**); §15.7.7's own `ClassBody` production is where a class body enters the algorithm. §-numbers from `webref aoid ecma262 AllPrivateIdentifiersValid` and `webref heading ecma262 16.1.1` / `16.2.1.1`; step text from `webref body ecma262 sec-static-semantics-allprivateidentifiersvalid` / `sec-scripts-static-semantics-early-errors` / `sec-module-semantics-static-semantics-early-errors`. Cross-layer (parser + scope analysis) and outside `compiler/`, so §1.0's derivation does not produce it: recorded here under this section's per-slice-sweep rule and owned by its own §8 slot. **Derived from the arms and the scope pass; not probe-measured** | T1 | `#11-vm-private-name-early-errors` |

**Verified NOT defects** (checked during the sweep, no action): `delete x` **is** correctly gated by
the parser ("Cannot delete an unqualified identifier in strict mode") — so `expr_ops.rs:156-159` is
unreachable for identifiers, contrary to a round-2 review claim; `with` → `CompileError` (arm head `stmt.rs:190` at `f7d9b5ce`, `:88` at `658cc302` — the file was
split; the `:192` this line used to carry was the message-string line, `:90` at `658cc302`)
is correct per I-6/ADR #2; `{[1n]:…}` and `{[/a/]:…}` computed keys are correct.

### §2.3 Stub inventory — Layer B (dispatch; dead code behind Layer A)

These are I-4 connect-or-delete items, **not** live defects. Each is discharged by the slice that
lands its Layer-A emit (connect), or deleted by the dead-opcode sweep (Slice D, adopting
`#11-dead-opcode-removal`).

⚠ **The membership test here is "zero compiler emit sites", so this section is not the dispatch-layer
defect inventory.** A handler that *is* emitted to and is wrong belongs in §2.2 — `op_spread_object`
is tabled there — and is found by reading handlers, not by the `compiler/` sweep. §2.2 states the
obligation that puts: the slice that owns a handler reads it against its own parent HEAD and tables
what it finds.

**⚠ Corrected R2 round 3 — the earlier hand-curated "nine" was wrong in both directions.** Enumerated
mechanically over all 125 `Op` variants in `bytecode/opcode.rs` (2026-07-26): **18** have zero
compiler emit sites, and **all 18 are actionable**. ⚠ **The `17 actionable + 1 reserved` partition
that stood here is withdrawn** — it exempted `Wide` (see the bullet below); the count is flat again,
and every live site states it that way.

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
- `Wide` raises a loud `VmError` at **`vm/dispatch.rs:1074`** — re-derived with
  `git grep -n 'Op::Wide' -- crates/`, which returns that line and `bytecode/opcode.rs:562` and
  nothing else; the `:1083` this bullet used to cite is wrong at `658cc302` and at HEAD alike, and
  §6.3 carried the same wrong offset. ⚠ **It is swept like the other 17, and the "reserved"
  exemption is withdrawn**: it was stated in prose with no slot id, no owner and no expiry, which
  §8's Forward rule forbids, and it is the opposite of what §2.3 did for `ImportMeta` /
  `DynamicImport` in the identical situation — they got a slot id. CLAUDE.md is
  *「dead code は接続するか削除」* and *「後方互換性は維持しない」*, and nothing pins the discriminant
  (`bytecode/opcode.rs` derives only `Debug, Clone, Copy, PartialEq, Eq` under `#[repr(u8)]`, and
  `git grep -ln 'serde' -- crates/script/elidex-js/src/bytecode/` is empty, so there is no
  serialized bytecode format). The option taken is **delete**, in Slice D, whose row carries the
  mechanical consequences.

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
| C×F | [C20] step 3 runs ArgumentListEvaluation **before** the step 4/5 callability checks ⇒ a non-callable callee must still drain the iterator to completion before throwing. **[C19] does NOT call `IteratorClose`** — verified 2026-07-26, `body ecma262 sec-runtime-semantics-argumentlistevaluation \| grep -ci iteratorclose` → **0**; `?` propagates the abrupt completion directly, and the drain's own AO already sets `[[Done]]` on throw: it calls **§7.4.10 IteratorStepValue** [C35], whose **step 4.a** sets it on an `IteratorValue` throw, and whose step 1 delegates to §7.4.9 IteratorStep — which sets it at **step 3.a** for an `IteratorComplete` throw, its own step 1 delegating further to §7.4.6 IteratorNext for the `next()` throw. *(An earlier draft credited §7.4.9 alone, which is not the AO invoked here.)* Calling `return()` here would be an *observable divergence*. (Contrast [C39] DestructuringAssignmentEvaluation → 6 `IteratorClose` call sites, which is why [C36] is **Slice 0bc's**, not Slice 1's.) |
| D×E | (none — IC slots are compile-time indices, not heap refs.) |
| D×F | The unwind path must not leave the args Array reachable only from a dropped Rust local. |

≥3 intersecting axes ⇒ **edge-dense ⇒ per-slice plan-review mandatory** — *and* ⇒ the row is an
umbrella under §5's terminality criterion, since the rule quoted there says the plan-review does not
discharge the split. Applied to the rows this section had already flagged (0b/2/3/5/6, plus 0c for
the three substitution classes × 6 compiler files × a deliberate user-visible behaviour change per
dec. 5, plus **P** for a convention sweep spanning `compiler/`, core `vm/` and `vm/host/`): §5 now
mints sub-slices for 0b, 0c, P, 2, 6, 7, 8 and 10, and each sub-slice carries its own
coupled-invariant enumeration in its own memo. 3 and 5 stay terminal and carry theirs.
0a and D are narrow enough
to skip. ⚠ **Withdrawn for both (§18).** 0a's is moot — it shipped, and the measured axis count was
**6**. **D is re-adjudicated here rather than left between two contradicting sites: it needs its own
plan-review.** Its charter is to *delete* opcodes on the strength of a re-derived §2.3 set, and §2.3
records that the previous enumeration was wrong in both directions — it would have deleted a live
opcode (`Op::GetModuleVar`). A deletion slice whose input is an enumeration with a known failure
history is not narrow.

---

## §3. Spec coverage map

Scope = **the 1a+1b pair** (per-PR split tagged below). [C19] is an SDO defined piecewise over **8 productions** (prose read via
`.claude/tools/webref body ecma262 sec-runtime-semantics-argumentlistevaluation`, 2026-07-26); 3 are
TemplateLiteral productions that route the *same* SDO into tagged templates and are therefore
listed here as explicit out-of-scope rows with a Slice-4b hand-off (I-3 requires Slice 4b reuse this
slice's helper, and `ArgsForm` as specified cannot express `« siteObj »` ++ substitutions — that
constraint is recorded now rather than discovered in Slice 4b).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | production | `Arguments : ( )` — empty list | `compile_call_arguments` (NEW) → `Flat(0)` | ✓ | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | steps 1-3 | `ArgumentList : AssignmentExpression` | `Flat(n)` path | ✓ | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | steps 1-5 | `ArgumentList : ... AssignmentExpression` — leading spread | `Array` path → `Op::CreateArray` + `Op::ArraySpread` | ✓ | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | steps 1-4 | `ArgumentList : ArgumentList , AssignmentExpression` | `Op::ArrayPush` / `Flat` | ✓ | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | steps 1-4 | `ArgumentList : ArgumentList , ... AssignmentExpression` — trailing/multiple spread | `Op::ArraySpread` after prior pushes | ✓ | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | production | `TemplateLiteral : NoSubstitutionTemplate` | — | n/a (out of scope → Slice 4b) | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | production | `TemplateLiteral : SubstitutionTemplate` | — | n/a (out of scope → Slice 4b) | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | production | `SubstitutionTemplate : TemplateHead Expression TemplateSpans` | — | n/a (out of scope → Slice 4b) | yes |
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
| ECMA-262 §7.4.11 IteratorClose | steps 1-8 | abrupt completion mid-drain | — | n/a (**not reached from [C19]** → Slice 0bc) | yes |
| ECMA-262 §13.3.9.1 Evaluation (optional chain) | step 3 | nullish → return `undefined` **before** ChainEvaluation | optional-call short-circuit (edge 12) | ✓ | yes |
| ECMA-262 §13.3.9.2 ChainEvaluation | step 3 | `OptionalChain : ?. Arguments` — optional **call** `f?.(...a)` → EvaluateCall. ⚠ **Not universal (Codex R2)**: for `o.m?.(...a)` the callee is still a *property Reference*, so §13.3.6.2 EvaluateCall step 1.a.i must pass `o` as `this`. `compile_optional_chain_expr` (`expr_member.rs:146`) compiles the `Member` base **as a value** and selects `CallMethod` only when a preceding member sits inside `chain`, so this spelling has **already lost its receiver at `658cc302`** — lowering it to `CallSpread` preserves the silent-wrong result. Needs a member-reference-base call shape plus edges for `o.m?.(...a)` and `o?.m?.(...a)` | `Op::CallSpread` (**+ a receiver-preserving form**) | ✓ | yes |
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
   call shapes. **7 rows are explicit `n/a` out-of-scope hand-offs** (3 TemplateLiteral → Slice 4b,
   [C20] steps 1.b.iii and 6, [C36] → Slice 0bc, [C22] contrast) — i.e. rows that document what this
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

1. **`PushUndefined`** for unimplemented syntax (`expr.rs:200-207`, `:237-239`, and
   **`stmt_destructure.rs:117-119`** — the object-pattern key catch-all, added at PR-B);
2. **silent no-op** — emitting nothing, or only the operand's side effects
   (`expr_assign.rs:210-212`, **`expr_assign.rs:202-206`**, `expr_ops.rs:261`, `expr_ops.rs:226`,
   `expr_ops.rs:249`, `expr_class.rs:428`, `expr_class.rs:430-447`, `expr.rs:186-189`,
   **`stmt.rs:99-103`**, `stmt.rs:877-878`, `stmt.rs:882`, the
   `ImportDeclaration`/`ExportDeclaration` half of `stmt.rs:31-39`, and — both added at PR-B —
   **`expr_object.rs:26-39`** (the accessor-key `_ => { fc.emit(Op::Pop); }`) and
   **`stmt_destructure.rs:149-150`** (the rest-exclusion `_ => {}`, whose *observable* is a wrong
   **value** rather than a missing effect — the destructured key survives into the rest binding.
   Listed here anyway: the banned thing is the emit shape, and typing it by its observable is how it
   dropped out of this list in the first place);
3. **wrong constant** — a plausible-looking substitute value (`expr_object.rs:117-121` empty-string
   key; `expr_ops.rs:147` `Pop; PushTrue` for `delete this.#x`);
4. **wrong arity** — the right *kind* of value in the wrong *count* (`expr_member.rs:70-75`, the
   call-spread defect itself). Named for completeness; owned by Slice 1b, not 0ca, since making it
   loud would break every `f(...a)` site that a working implementation is about to fix.

The class-1 and class-2 lists above are the §2.2 table's class-1 / class-2 rows, kept in sync
deliberately — R2 round 3 caught them as a strict subset, and PR-B's four new rows reproduced
exactly that, which is how an arm silently drops out of 0ca's remit.

**Slice 0ca discharges all three program-wide up front** (§5) — with **three** carve-outs, not one:

- **the two early-SyntaxError sites**, which §2.2 assigns to **0ba** because a runtime throw
  is the wrong remedy for a parse-time rejection (§9 dec. 15): `expr.rs:186-189` prefix `Spread`
  (class 2) and `expr_ops.rs:147` `delete this.#x` (class 3);
- **the four `PropertyKey` rows PR-B added or corrected** (`expr_object.rs:26-39`,
  `stmt_destructure.rs:117-119`, `stmt_destructure.rs:149-150` and the `expr_class.rs` dead
  `if computed` guard), which §2.2 routes to **`#11-vm-property-key-lowering-unification`** — and the
  `expr_class.rs` `_ =>` arms to **Slice 2** — because making four divergent catch-alls individually
  loud entrenches the duplication the collapse exists to remove (CLAUDE.md *One issue, one way*);
- **class 4 (wrong arity)**, carved in item 4 above (Slice 1b's, not 0ca's).

Its scope is derived by a documented
sweep, not by inspection — see §5 Slice 0ca and §9 decision 9. §9 decision 5 records the
throw-vs-`CompileError` choice.

**I-2 · `assert!` is not rejection** — *for the refusal-of-user-constructs class only*. A compiler
arm that rejects a **user-writable construct** must never `assert!`/`panic!`/`unreachable!`
(`expr_assign.rs:170`, `expr_member.rs:60`). **The remedy is `Op::ThrowUnsupported`, a scoped
runtime throw — NOT `CompileError`** (dec. 5, ratified by evidence in §17/§18): I-1 asks for loud
*and scoped*, and `CompileError` yields no bytecode for the whole script, so it fails every
unrelated statement too. **That reason is not bound to this invariant's class**: it holds for any arm
whose `CompileError` stands in for a failure ECMA-262 evaluates at run time, whether or not the arm
ever asserted — §2.2's `compiler/expr_assign.rs:80-88` row is the instance, and it is owned by a slot
rather than by I-2. `CompileError` stays correct only where the compiler *already* rejects, and
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
flat-vs-array; Slice 4b's tagged-template arg list routes through the same helper. **Carve-out
(spec-mandated, R2 round 5)**: the synthesized default derived constructor
(`compiler/expr_class.rs:145-152`) must NOT be folded — ECMA-262 §15.7.14 step 14.a.iv.1 NOTE requires it
not to observably call `%Array.prototype%[@@iterator]`, which the shared path would. See §6.3. The direct lesson
of the present bug: `super` got a correct hand-written spread branch while the other four
`compile_arguments` callers silently shared a broken one.

**I-4 · Connect or delete.** All **18** zero-emit opcodes (§2.3 — rebuilt mechanically in
round 3; the earlier "nine" was wrong in both directions) are each either connected by the
slice that lands their Layer-A emit, or deleted by the dead-opcode sweep. §5 names an owner for
every one — no unowned violations, and **no named exception**: the `Op::Wide` carve-out that stood
here had no slot id, no owner and no expiry, which §8's Forward rule forbids, so `Wide` is in Slice
D's delete scope like the rest (§2.3).

**I-5 · Layering.** Two directions, and the boundary is the **`engine` feature gate + host-binding
dependency**, not directory name (R2 round-2 correction — `vm/gc/keepalive.rs`, `vm/host_data/`,
`vm/sw_thread.rs`, `vm/worker_thread.rs`, `vm/webidl_sequence.rs`, `vm/wasm_payload.rs` all sit
outside `vm/host/` yet are engine-bound):

- **Outbound**: no slice may add *language semantics* to engine-bound code (`vm/host/` and the
  `#[cfg(feature = "engine")]` surface above).
- **Inbound**: no slice may pull *engine-bound responsibility* (network / HTML fetch / DOM binding)
  into core.
- ⚠ **The inbound rule has no membership test, and core dispatch already carries host knowledge.**
  Measured at `658cc302`: `vm/dispatch_objects.rs:404`/`:415` (the `in` operator special-casing
  `DOMStringMap` / `Storage`) and `vm/dispatch_iter.rs:121`/`:132` (for-in named-property exotic
  keys) call `host::dataset::*` / `host::storage::*` from core. I-5's consequence #2 then directs
  Slice 10b to resolve `Proxy`/`Reflect` × `ObjectKind::HostObject` *inside* core dispatch, i.e. to
  add more. Whether those are admissible marshalling or inbound violations is undecided here; the
  outbound rule got a Converse precisely because a literal reading stops at the wrong place, and
  the same is available in this direction. **Slice 10b's mandatory plan-review must settle it, and
  audit these four sites under whatever test it adopts.**
- ⚠ **The Converse below is contradicted by `vm/host/mod.rs:11-13`'s own layering mandate**, which restricts everything under that directory to engine-bound responsibilities. Carved as `#11-vm-typed-array-family-layering-and-gate` (§8) rather than resolved here — it is a relocation/gating decision, not a wording fix.
- **Converse (added R2 round 5)**: engine-**gated** is not the same as engine-**bound**. Files under
  `vm/host/` that implement pure ECMA-262 language surface (`vm/host/typed_array_*.rs` = ECMA-262
  **§23.2 TypedArray Objects**, the *family* — **not** §23.2.2 "Properties of the %TypedArray%
  Intrinsic Object", the static-method subsection alone, which is what this sentence used to say;
  §-number verified via `webref heading ecma262 23.2`, and §23.2.2.1 `%TypedArray%.from` is the one
  member this clause actually reasons about, at `typed_array_static.rs:798`, an `iter_close` caller;
  and the
  iterator-protocol call sites in `vm/host/{url_search_params,headers/parse_init}.rs` — ⚠ **round-8
  correction: these two are NOT "pure ECMA-262"**; with `vm/webidl_sequence.rs` they implement
  **WebIDL §3.2.21.1** "Creating a sequence from an iterable", which has *no* `IteratorClose` step,
  so they take the same two-layer disposition as their shared helper) are **language-semantics sites the core
  convention owns** — a core-wide convention sweep (e.g. Slice P) *must* reach them, and doing so is
  not an outbound violation. Without this clause an implementer reading the outbound rule literally
  would stop at the core sites and ship the split-contract One-issue-one-way failure the sweep
  exists to remove. ⚠ **Slice 0a already spent part of this disposition** — it touched
  `vm/host/{headers/parse_init,structured_clone,url_search_params}.rs`, re-attributed their governing
  sections (WebIDL §3.2.27 → §3.2.21 / §3.2.21.1, §3.10.21 → §3.2.15) and registered
  `#11-webidl-sequence-dense-array-fast-path` at three in-code sites. P inherits a partly-made
  decision, not a blank one — see §16.
  ⚠ *A correction bullet used to sit above this Converse re-pairing each `typed_array_*` file with
  its own §-number. It is **dropped, not re-derived**: `ls vm/host/ | grep typed_array` gives seven
  files, the bullet paired three, and one of the three was wrong —* `typed_array_parts.rs`*'s only
  §23 citation is `:84` and is **§23.2.4.4 ValidateTypedArray**, not the §23.2.3 it was paired with.
  An annotation layered over a normative sentence is the pattern this document is removing; the
  sentence itself now says §23.2.*
  **Two-layer cases needing explicit disposition in Pa's and Pc's memos**: `vm/webidl_sequence.rs`
  (governed by WebIDL §3.2.21) and **`vm/host/structured_clone.rs`** (WHATWG HTML **§2.7.4** StructuredSerialize
  (§2.7.7 StructuredSerializeWithTransfer for the transfer-list path where the `iter_close` at
  `:1062` sat — `:1064` at `658cc302`. ⚠ **Codex R1 corrected the attribution of that site**: it is
  inside `ensure_empty_transfer_list`, whose own docstring calls it a WebIDL `sequence<object>`
  conversion throwing per **WebIDL §3.2.21 step 3**, and its `iter_close` runs while consuming the
  iterable — *before* HTML StructuredSerializeWithTransfer ever receives the list. So the **site
  stays in the WebIDL half of §6.2a-3's 7/5 partition**, and it is the *outer native* that is separately
  HTML §2.7.4 / §2.7.7. Calling the site HTML's own loop would select the wrong `.return()` and
  error-precedence remedy in the P family. The in-code docstring's "§2.9" is drifted; **§5's Slice-Pc
  row owns the retag as a named deliverable** — this clause does not) — ⚠ **the sentence that used to follow here is withdrawn (Codex R3)**: it called the `DataCloneError` an abrupt completion of *HTML's own loop*, contradicting the correction directly above and leaving §6.2a-2 enumerating only four non-ECMA sites. `ensure_empty_transfer_list` is the **WebIDL** conversion; only the outer native is HTML §§2.7.4/2.7.7, and the WebIDL count must follow from that. Superseded text: its abrupt was described as a `DataCloneError` thrown by HTML's own loop body, so a
  step-5 flip changes an HTML-defined error surface, not an ECMA-262 one; round 6 corrected an
  earlier mis-classification of this file as pure ECMA-262).

  ⚠ This clause is currently an **allowlist, not a predicate** — state the membership test
  ("which spec governs the algorithm whose completion is being closed?") and **re-derive the file
  list from it at implementation time**, per §13's enumerate-by-command rule. This binds **Slice M** — module-script *fetching* is an ECMA-262 host hook and must go
  through a host seam, not the core VM; the renderer holds no direct network access (CLAUDE.md
  "Security by structure"). Slice M's memo must draw that line before implementation.

Two named consequences: **Slice 7b**'s WeakMap/WeakSet ephemeron work lands in `vm/gc/trace.rs`
alongside engine-gated wrapper-store rooting and the keepalive predicate
(`vm/gc/collect.rs:1149→1235→1341`), and must define identical semantics for non-`engine` builds;
**Slice 10b** must resolve `Proxy`/`Reflect` × `ObjectKind::HostObject` (`vm/object_kind.rs:272`)
inside core dispatch — a host-side special case would violate the outbound rule.

**I-6 · No sloppy/Annex B surface.** No slice introduces sloppy-mode or Annex B behaviour
([[reference_elidex-js-core-strict-only]]). **Note**: this does *not* gate `Function`/`eval` — per
`docs/design/ja/14-script-engines-webapi.md` §14.1.1, strict-mode `eval` is **core**; only sloppy
`eval`'s caller-scope injection is LegacySemantics. That slice's real gate is a security/CSP policy
decision (§9 decision 7).

---

## §5. Slice plan

Each slice = own PR + own `/elidex-plan-review`.

**Terminality criterion — the rule, quoted from CLAUDE.md § *Design discipline*, "Edge-dense work":**

> ≥3 intersecting invariant axes を束ねる、または正準アルゴリズムが無い subsystem を触る work は
> (a) **単一 PR に束ねない** — umbrella plan + PR ごとの plan に分割し各 PR を個別に full review、
> (b) **各 PR は実装前に `/elidex-plan-review` 必須** (judgment でなく rule)、(c) **base case
> (再帰の終端)** = 承認済 umbrella 配下で plan-review を通った narrowly-scoped per-PR slice は
> terminal 単位 = 許容される単一 PR

Read (c) in the direction it is written: approval makes a **narrowly-scoped** slice terminal. It does
not make a row narrowly scoped, and clause (b) says the mandatory plan-review does not discharge (a).
So the test a row must pass, and the test to apply to any row added later **before** a reviewer
applies it: *does this row bundle ≥3 intersecting invariant axes, or span more than one governing
algorithm across layers?* If it does, the row is an umbrella; it states its charter, mints terminal
sub-slices, and each sub-slice is its own PR under its own plan-review. Sub-slices take the parent's
id plus a trailing letter, as `9a`-`9d` already do.

This umbrella is the approved parent for every row below that passes that test.

**A derivation or an acceptance condition is stated over the algorithms the spec defines for a
surface, not over the members that surface has.** §2.2's accessor row already carries the
distinction in the form that matters here — *exhaustive is not the same as correct*: the sibling
`PropertyKind::Init` match is exhaustive over `PropertyKey` and still lowers a BigInt key to the
empty-string key. A rule written over members inherits that failure and adds a worse one: a member
can be present while the algorithm governing it is unbuilt, so the rule reads satisfied, the slice
retires, and the row that would have caught it is the row that declared itself done. So a slice's
derivation names the spec clauses that define the algorithms for its surface and is run against
those, and its acceptance condition asserts what those algorithms require of an observable
result — never that a name resolves.

**A derivation's output is a grouping, not a slice.** What a derivation returns is what the clauses it
names cover, and nothing about that shape makes it one PR's worth of work. So a group is an
**intermediate** result: the terminality criterion above is applied to each group the derivation
returned, and a group yields as many terminal slices as that criterion requires. A rule that mints one
slice per group defeats the criterion precisely where the derivation *succeeded*, because a group is
well-formed as a spec unit and can still hold mechanisms with no shared lowering chokepoint. `Object`
is the demonstration: **§20.1.2.7** `Object.fromEntries` step 6 performs **§24.1.1.2**
AddEntriesFromIterable, whose step 1 is `GetIterator(iterable, sync)`, and **§20.1.2.13**
`Object.groupBy` step 1 performs **§7.3.35** GroupBy, whose step 4 is the same `GetIterator` — while
**§20.1.2.14** `Object.hasOwn` is `ToObject` → `ToPropertyKey` → `HasOwnProperty` and consumes no
iterator (numbers from `webref aoid ecma262 AddEntriesFromIterable` / `GroupBy` and
`webref heading ecma262 20.1.2`; steps from `webref body ecma262 sec-object.fromentries` /
`sec-add-entries-from-iterable` / `sec-object.groupby` / `sec-groupby` / `sec-object.hasown`). One PR
for "the Object family" therefore bundles an iterator-protocol contract with a member that has none.
This is stated once for the whole table and governs every derivation this document writes — the
builtin one, Slice 7's, Slice 8's, Slice M's, and any later one; no row restates it, and a row that
reads as one slice per group is repointed here.

⚠ **The `Primary module(s)` column is a non-authoritative HINT, not a touch set.** It was written
against one tree; the slices land months apart against moving code, so a column that reads as a
specification is wrong by construction — and it duplicates a decision each slice's own **mandatory
`/elidex-plan-review` already owns**: deriving the touch set against that slice's parent HEAD. Four
consecutive Codex rounds each checked one more column against current code and found it incomplete
(the 0b and P rows missing `stmt_loop.rs`; Slice 6 missing `stmt_loop.rs` *and* `vm/interpreter.rs`; Slice 8
missing the well-known-symbol table and string-method dispatch) — hopping corner to corner, which is
the signature of a whole layer generating findings rather than a list needing one more entry.
**So: derive the touch set at slice time; treat the column as a starting point.** The gaps already
found are kept below as *evidence about the code*, because they were expensive to find — not as a
claim that the columns are now complete.

⚠ **The evidence block is the single home for those gaps; the cells carry none of them.** The first
pass at this demotion moved Slice 8's gap into the evidence block and left the others written into
their cells, so the column and the evidence contradicted each other inside one document — a
strangler state, and exactly the decision-surface duplication the demotion was for. A gap detail
appears **once**, below.

**Load-bearing exceptions that survive as specification**, because they are *ordering* facts rather
than file lists: the Deps column, and the prerequisites called out per row.

**What puts an edge in the Deps column — stated once for the whole table, and nowhere else.** An
ordering edge exists **only where the later row names the artifact it consumes** from the earlier
one: an opcode, a data structure, a helper signature, a spec-algorithm output. A shared file, a
shared arm, a shared family and a shared spec area are **not** edges. Where two rows must each carry
the other's contract but neither's deliverable is an input to the other, that is **coordination** —
recorded in the rows, never in the Deps column. Two rows used to restate this locally, having each
been corrected into it a round at a time; both restatements are deleted in favour of this one.

**Evidence found by those rounds, kept because it was expensive** (each verified against
`658cc302`; re-verify at slice time rather than reading forward):

- **Slices 6a/6b** cannot work from `stmt.rs` alone. `compile_for_of` (`stmt_loop.rs:62`) takes no
  `is_await` and owns every `GetIterator` / `IteratorNext` / body / close path. Worse, `call_internal`
  (`interpreter.rs:629-664`) and `push_js_call_frame` (`:862-912`) both test `is_async` **before**
  `is_generator`, so an async-generator function — whose compiled flags are *both* true — takes the
  ordinary async path and returns a Promise before generator construction is considered. Adding an
  `ObjectKind` and generator natives changes neither entry point, so `ag().next` stays non-functional:
  6a needs a distinct async-generator construction path in **both** callers.
- **Slice 8b** cannot complete the `@@match` / `@@replace` surface §1.1 assigns it from
  `natives_regexp.rs` + `globals_primitives.rs`: `well_known.rs:1552-1585` has no `match` / `replace`
  symbol ids, and `natives_string.rs:468` / `:549` special-case `ObjectKind::RegExp` **directly**
  instead of looking up the argument's symbol method — so methods can be installed but never
  dispatched to, and user overrides never honoured.
- **Slice 0bc** cannot work from `stmt.rs` alone either. `compile_forin_left_binding` is at
  `stmt_loop.rs:308` at `658cc302` (`git grep -n 'fn compile_forin_left_binding' 658cc302`) and is
  where `[a,b]` for-of heads and `for (obj.p in …)` currently reach `emit_unsupported`; `stmt.rs`
  only delegates `ForIn`/`ForOf` there. A touch set naming only `stmt.rs` leaves 0bc's headline cases
  unimplemented.
- **Slice Pb** likewise: `compiler/stmt_loop.rs` did not exist when the columns were first written —
  0a created it by splitting `compiler/stmt.rs`, and it took the `for-of` catch handler's
  `Op::IteratorClose` emit with it (§6.2a-2).
- **Slice 1a**: `ic_call` / `ic_call_method`'s only callers are in `vm/dispatch.rs`, and their line
  numbers have already moved once — `:719`/`:730` at `f7d9b5ce`, `:705`/`:716` at `658cc302`
  (`git grep -n 'ic_call\b\|ic_call_method' <rev> -- vm/dispatch.rs`). Re-derive by grep; do not read
  either pair forward. Without re-signaturing both, 1a does not compile (dec. 11).

**Acceptance conditions are demoted the same way, and the reason is the one stated just above.** Every
acceptance condition this document writes was derived against one tree at one time; the slices land
months apart against moving code, and a finite sentence cannot bound the spec surface it is written
over — so a condition that reads as *the specification of when a slice may retire* is wrong by
construction, and it duplicates a decision each slice's own **mandatory `/elidex-plan-review` already
owns**: deriving the slice's obligation from the spec clauses governing the surface it touches,
against that slice's own parent HEAD. The plan-review round that produced the evidence below hit one
row after another with a sibling requirement in the same spec surface that the row's stated condition
does not detect — hopping from row to row, which is the signature of a whole layer generating findings
rather than a list needing one more entry.
**So: derive the obligation at slice time; treat what is written here as a starting point.** The
requirements already found are kept below as *evidence about the code*, because they were expensive to
find — not as a claim that the conditions are now complete. The load-bearing exception is the one the
passage above already states once, and it holds here unchanged: the Deps column and the prerequisites
called out per row are *ordering* facts and survive as specification. This is the same rule as the
members-vs-semantics rule at the head of this section, applied to the other half of a charter — and,
like the terminality criterion, it is stated once for the whole table.

**A terminality verdict recorded in this table is the same class of claim, and falls under this same
demotion.** It too was reached against one tree at one time, by reading a row's own charter sentence
rather than the algorithms the surface needs — so a row that reads **terminal** is a starting point,
and the criterion at the head of this section is re-applied at slice time by the slice's own
mandatory plan-review, against its own parent HEAD. Slice 4 is the instance: it was judged terminal
when the criterion was applied to the whole table, and that verdict was wrong — it is an umbrella
below, split into 4a-4c.

**Evidence found by that round, kept because it was expensive** (each verified against `658cc302`;
`git diff --stat 658cc302 ddd2b931 -- crates/` is empty, so these are current — re-verify at slice
time rather than reading forward):

- **Slice 0bc** — a destructuring **assignment** must evaluate to its RHS, and the storage helper it
  will reuse is specified to consume it. `compile_destructure_pattern`
  (`compiler/stmt_destructure.rs:23`) documents "Assumes the value to destructure is on top of the
  stack. After compilation, the value is consumed (popped)." (`:20-21`), and every store path in
  `compile_pattern_store` (`:189`) emits `Op::Pop` (`:211`, `:215`, `:221`, `:227`). Correct for the
  declaration form; wrong for the expression form. ECMA-262 **§13.15.2** Runtime Semantics: Evaluation,
  production `AssignmentExpression : LeftHandSideExpression = AssignmentExpression`, reaches step 5
  "Perform ? DestructuringAssignmentEvaluation of assignmentPattern with argument rightValue"
  (**§13.15.5.2**, which itself returns *unused*) and then step 6 "Return rightValue"
  (`webref body ecma262 sec-assignment-operators-runtime-semantics-evaluation`). So 0bc can satisfy
  every named-storage and `IteratorClose` regression it carries while leaving nothing on the stack.
  Identity probe: `const rhs=[1]; let out=([a]=rhs); out===rhs`.
- **Slice L** — a label is recorded as iterative only when a loop keyword follows it *immediately*, so
  chained labels lose the outer one. `parser/stmt.rs:113-117` computes
  `is_iteration = matches!(self.at(), TokenKind::Keyword(Keyword::For | Keyword::While | Keyword::Do))`
  from the token after the colon and pushes it with the label; `parser/control_flow.rs:444-454` then
  raises "'continue' label '…' does not refer to an iteration statement" whenever
  `!is_break && !is_iteration`. So `outer: inner: while (cond) { continue outer; }` is rejected though
  it is valid. The SDO that decides a labelled `continue`'s target is ECMA-262 **§8.3.3**
  ContainsUndefinedContinueTarget: its `LabelledStatement : LabelIdentifier : LabelledItem` production
  is "Let newLabelSet be the list-concatenation of labelSet and « label »" and recurses with it, its
  `BreakableStatement : IterationStatement` production is "Let newIterationSet be the
  list-concatenation of iterationSet and labelSet", and its `ContinueStatement : continue
  LabelIdentifier ;` production returns false when iterationSet contains the label — so a chain
  accumulates and every label in it names the loop. It is invoked as an early error from **§16.1.1**
  Static Semantics: Early Errors ("It is a Syntax Error if ContainsUndefinedContinueTarget of
  StatementList with arguments « » and « » is true"). **§14.8.1** is the separate rule — "It is a
  Syntax Error if this ContinueStatement is not nested … within an IterationStatement" — and decides
  nesting, not label targets.
- **Slice 2aa** — a computed field name is evaluated **once, when the class is defined**, and the key is
  retained for later instances. The baseline emits nothing at all for a non-static field:
  `ClassMemberKind::Property` (`compiler/expr_class.rs:393`) is wholly inside `if *is_static`
  (`:402`), and `:428` reads "Non-static properties: skip (would need field initializer injection)" —
  so the computed key is skipped with the rest. ECMA-262 **§15.7.14** ClassDefinitionEvaluation step 26
  runs ClassElementEvaluation over every class element at definition time; **§15.7.10**
  ClassFieldDefinitionEvaluation step 1 is "Let name be ? Evaluation of ClassElementName" and step 4
  returns a ClassFieldDefinition Record carrying it as `[[Name]]`; §15.7.14 step 30 then sets
  `ctorFunc.[[Fields]]` to that list, so construction reuses the key rather than recomputing it. An
  implementation that injects initializers into the constructor passes the `x = 1` and receiver tests
  while evaluating the key per construction. Probe:
  `let n=0; class C { [++n]=1 } new C; new C; n===1`.
- **Shared prerequisite of Slices 2ac and 2b** (an ordering fact, so it is stated once here rather than
  on either row — and a shared prerequisite is not a member of either slice's charter) — the
  class-name binding is initialized **before** static elements run.
  `compile_class` (`compiler/expr_class.rs:99`) emits every member in its body loop, the
  `ClassMemberKind::StaticBlock` arm (`:448`) included, and only afterwards initializes the inner
  class-name binding (`:479-500`; `Op::InitLocal` at `:498`, `Op::SetLocal` at `:499`). ECMA-262
  **§15.7.14** orders it the other way: step 28 is "Perform ! classEnv.InitializeBinding(classBinding,
  ctorFunc)", and step 32 is the loop over staticElements that performs `DefineField` for a static
  field and "Call(elementRecord.[[BodyFunction]], ctorFunc)" for a static block. So
  `class C { static self = C; static { if (C !== this) throw 0 } }` reads an uninitialized binding
  today, and whichever of 2ac/2b lands first is the one that moves the initialization.
- **Shared prerequisite of the 2a family** (an ordering fact, so it is stated once here rather than on
  a row — and a shared prerequisite is not a member of either slice's charter) — `arguments` is
  accepted inside a class field initializer, and the 2a family is what turns that from inert into
  observable behaviour. The `arguments` arm of `visit_expr` (`scope/visitor.rs:241-263` at
  `658cc302`) walks the scope stack outwards and raises "'arguments' is not allowed in class static
  blocks" only on reaching a `ScopeKind::StaticBlock` (`:246-252`); every other kind falls to
  `_ => {}` (`:260`) until the first non-arrow `ScopeKind::Function`, which it marks
  `uses_arguments = true` (`:256-258`). `visit_class_body` (`:542-564`) pushes that scope for a
  `ClassMemberKind::StaticBlock` (`:555`) and for nothing else — a `Property` / `PrivateField`
  initializer is visited with a bare `visit_expr(prog, state, *v)` (`:549-552`) — so
  `function f(){ class C { x = arguments } }` reaches the enclosing function's arm and is accepted.
  ECMA-262 **§15.7.1** Static Semantics: Early Errors makes it a Syntax Error at the field
  production: under `FieldDefinition : ClassElementName Initializer[opt]`, "It is a Syntax Error if
  Initializer is present and ContainsArguments of Initializer is true", where **§15.7.9**
  ContainsArguments returns true for an `IdentifierReference : Identifier` whose StringValue is
  "arguments" and otherwise recurses into child nodes (§-numbers from
  `webref aoid ecma262 ContainsArguments` and `webref heading ecma262 15.7.1`, step text from
  `webref body ecma262 sec-class-definitions-static-semantics-early-errors` /
  `sec-static-semantics-containsarguments`). It is inert today only because nothing emits a
  non-static field initializer at all (`compiler/expr_class.rs:428`, "Non-static properties: skip
  (would need field initializer injection)"), so whichever of 2aa/2ab first connects the initializer
  is the one that must carry the rejection — otherwise the family's first landing is an initializer
  capturing the enclosing function's `arguments`. **Derived from the arms and the scope pass; not
  probe-measured.**
- **Shared prerequisite of Slices 2 and 3** (an ordering fact, so it is stated once here rather than on
  either row — and a shared prerequisite is not a member of either slice's charter) — **a class field
  initializer and a class static block each need a home object of their own, and neither producer has
  one.** Slice 3 makes `super.x` work for the producers that already reach lowering with class frame
  state; these two do not reach it at all, so 3 can retire with both still broken and 2 can connect
  both while `super` inside them stays unavailable. Two producers, two layers, both measured at
  `b5356098` (`git diff --stat 658cc302 b5356098 -- crates/` is empty, so both readings hold at
  `658cc302`): (i) a **field initializer** is parsed by `parse_optional_initializer`
  (`parser/mod.rs:324-331`), which changes no parse context at all, and all three field sites call it
  that way (`parser/function.rs:416` the `static`-named field, `:482` the private field, `:579` the
  ordinary field), so the initializer inherits the class body's ambient context — while the `super`
  gate at `parser/primary.rs:417-428` errors with "'super' keyword unexpected here" unless
  `in_method || in_constructor || in_static_block`, none of which a class body sets. So
  `class B extends A { x = super.x }` is rejected outright at parse time, and what decides the verdict
  is where the class is *written* rather than what a field initializer is: the same text inside a
  method inherits that method's `in_method` and passes the same gate. (ii) a **static block** parses
  with `in_static_block = true`
  (`parser/function.rs:432-447`) and so passes the gate, but the compiler's arm builds an ordinary
  child closure — `FunctionCompiler::new(fc.func_scope_idx, fc.current_scope_idx, true)`
  (`compiler/expr_class.rs:456-457`), whose third argument is `is_strict`
  (`compiler/function.rs:81`) — and `is_class_ctor` stays at its `false` default
  (`compiler/function.rs:95`; the only two writes are `compiler/expr_class.rs:82` and `:125`, both the
  constructor). A frame's `home_class` is `Some` only when `compiled.is_class_ctor`
  (`vm/interpreter.rs:802-806`), and `dispatch_super_inner` raises
  `VmError::syntax_error("'super' keyword unexpected here")` when it is `None`
  (`vm/dispatch_class.rs:115-119`), so `class B extends A { static { this.y = super.x } }` cannot
  resolve the superclass. ECMA-262 gives both producers the class as an explicit argument:
  **§15.7.10** ClassFieldDefinitionEvaluation takes `homeObj` and, for
  `FieldDefinition : ClassElementName Initializer[opt]`, creates the initializer function at step 2.5
  (`OrdinaryFunctionCreate(%Function.prototype%, sourceText, formalParamList, Initializer,`
  `non-lexical-this, envRecord, privateEnv)`) and performs "MakeMethod(initializer, homeObj)" at step
  2.6; **§15.7.11** ClassStaticBlockDefinitionEvaluation takes the same argument and does the same at
  its steps 5 and 6 for `bodyFunc`; **§10.2.7** MakeMethod ( func, homeObj ) is what sets
  `[[HomeObject]]` (§-numbers from `webref aoid ecma262 ClassFieldDefinitionEvaluation` /
  `ClassStaticBlockDefinitionEvaluation` / `MakeMethod`, steps from `webref body ecma262`
  `sec-runtime-semantics-classfielddefinitionevaluation` /
  `sec-runtime-semantics-classstaticblockdefinitionevaluation`). Regressions, one per producer and one
  per receiver: instance-field `class B extends A { x = super.x }`, static-field
  `class B extends A { static x = super.x }` and static-block
  `class B extends A { static { this.y = super.x } }` must each read `A`'s property. **Derived from the
  arms; not probe-measured.**
- **Shared prerequisite of Slices 2b and M** (an ordering fact, so it is stated once here rather than on
  either row — and a shared prerequisite is not a member of either slice's charter) — **`await` is
  accepted inside a class static block in module mode.** `parse_class_member`'s static-block arm
  (`parser/function.rs:432-447` at `b5356098`) saves the context and clears `in_function`, `in_async`,
  `in_generator`, `in_loop` and `in_switch` while setting `in_static_block = true` (`:434-439`); it does
  not touch `is_module`. `await_is_keyword` (`parser/mod.rs:61-63`) is
  `self.in_async || self.in_async_params || (self.is_module && !self.in_function)`, so clearing
  `in_function` inside a module *satisfies* it, and the `Await` arm (`parser/expr.rs:208-226`) errors
  only for `in_async_params` (`:213-218`) before allocating `ExprKind::Await` at `:222-225`. So the
  moment Slice M gives `parse_module` a production caller, `class C { static { await 0 } }` parses, and
  Slice 2b's generic static-block lowering compiles it into the synchronous child closure of
  `compiler/expr_class.rs:456-466` — the silent-wrong class this umbrella exists to remove, arriving
  through a parser gate rather than a compiler arm. ECMA-262 forbids it as an early error:
  **§15.7.1** Static Semantics: Early Errors, production
  `ClassStaticBlockBody : ClassStaticBlockStatementList` — "It is a Syntax Error if
  ClassStaticBlockStatementList Contains await is true" (§-number from
  `webref heading ecma262 15.7.1`, rule text from
  `webref body ecma262 sec-class-definitions-static-semantics-early-errors`). ⚠ **The rule is the
  `Contains` SDO applied to the terminal `await`, not an AO named `ContainsAwait`**:
  `webref aoid ecma262 ContainsAwait` returns "no AO with aoid='ContainsAwait' in ecma262", so do not
  cite one. The fix is the parser early error, and it must land with whichever of 2b and M comes first —
  before M enables module parsing and before 2b lowers a body that can contain the token. Regression in
  **module mode**: `class C { static { await 0 } }` parsed through `parse_module` is a SyntaxError. One
  spelling is enough because the arm's other clears close the other routes into
  `await_is_keyword` — it sets `in_async = false` (`:435`), so a static block nested in an `async`
  function does not reach the first disjunct, and `in_async_params` is not a class-body state; the
  module disjunct is the only one the arm leaves live. **Derived from the arms; not probe-measured.**
- **Slice 3** — an object-literal method gets `[[HomeObject]]` too, and the frame state the slice
  threads is class-only. The parser already enables `super` there: `parse_method_function`
  (`parser/object.rs:334`) sets `this.context.in_method = true` (`:345`) under the comment "S4: methods
  have [[HomeObject]] — super.prop is valid" (`:344`), and an object-literal method is mapped
  `MethodKind::Method | MethodKind::Constructor => PropertyKind::Init` (`:60`). The compiler then
  installs it through the ordinary paths with no home-object state: `compile_object_expr`
  (`compiler/expr_object.rs:46`) compiles the `PropertyKind::Init` value with `compile_expr` and
  defines it with `Op::DefineProperty` / `Op::DefineComputedProperty`, and the accessor kinds go
  through `compile_accessor`. `git grep -n 'home_class' 658cc302 -- crates/script/elidex-js/src`
  reaches `compiler/expr_class.rs:60`, `compiler/function.rs:40`, `compiler/expr_member.rs:111`,
  `bytecode/` and `vm/` but no line in `compiler/expr_object.rs`, and the frame field is derived from
  `compiled.is_class_ctor` (`vm/interpreter.rs:802`). ECMA-262 routes both spellings to the same
  operation: **§13.2.5.6** PropertyDefinitionEvaluation, production
  `PropertyDefinition : MethodDefinition`, step 1 "Let result be ? MethodDefinitionEvaluation of
  MethodDefinition with arguments obj and true"; **§15.4.5** MethodDefinitionEvaluation step 1 "Let
  methodDef be ? DefineMethod of MethodDefinition with argument obj"; **§15.4.4** DefineMethod step 7
  "Perform MakeMethod(closure, obj)"; **§10.2.7** MakeMethod step 2 "Set func.[[HomeObject]] to
  homeObj" — and §15.4.5's `get` / `set` productions perform MakeMethod at their own step 7. Slice 3's
  audit is stated over `home_class` and a `new B().m` regression, so every class probe can pass while
  `const o={m(){return super.x}}; Object.setPrototypeOf(o,{x:1}); o.m()` stays broken.
- **Slice M** — the parser already accepts top-level `await`, and the VM asserts it cannot happen.
  `parse_module` (`lib.rs:94`) builds the parser with `ProgramKind::Module`, which sets `is_module`
  (`parser/mod.rs:104`); `await_is_keyword` on `ParseContext` (`parser/mod.rs:61-63`) is
  `self.in_async || self.in_async_params || (self.is_module && !self.in_function)`; and
  `parser/expr.rs:208-226` allocates `ExprKind::Await` under it, its comment reading "Await — B21: also
  valid at top level in modules (not inside sync functions)". On the VM side `call_internal`
  debug-asserts `matches!(kind, FrameKind::Function) || !(is_async || is_generator)`
  (`vm/interpreter.rs:575-579`) and the comment above it (`:566-574`) says top-level bodies are never
  async or generator, "top-level await deferred". M's row derives over the **grammar** — ECMA-262
  §16.2.2 Imports and §16.2.3 Exports — and that derivation cannot reach this: module *execution* is
  **§16.2.1** Module Semantics. **§16.2.1.7.3.2** ExecuteModule takes the synchronous branch at step 3
  only "If module.[[HasTLA]] is false" and otherwise asserts a PromiseCapability Record and performs
  `AsyncBlockStart` at step 4; **§16.2.1.6.1.3.1** InnerModuleEvaluation step 12 routes a module whose
  `[[HasTLA]]` is true to **§16.2.1.6.1.3.2** ExecuteAsyncModule (whose step 2 asserts `[[HasTLA]]` is
  true), against step 13's plain "Perform ? module.ExecuteModule()". So M's derivation must reach
  §16.2.1 and not only the §16.2.2/§16.2.3 productions — the members-vs-semantics distinction at the
  head of this section, in its module spelling.
- **Slice B** — three consecutive rounds each produced one more construct whose block scope is never
  entered, one per round, and that set is derivable in two commands. Run at `658cc302`:
  `git grep -n 'ScopeKind::Block' 658cc302 -- crates/script/elidex-js/src/scope` returns **7** push
  sites once `scope/tests.rs` is excluded (`scope/visitor.rs:72`, `:85`, `:114`, `:131`, `:148`,
  `:166`, `:533`), and `git grep -n 'find_child_block_scope' 658cc302 -- crates/script/elidex-js/src`
  returns **5** entry sites (`compiler/stmt.rs:136`, `:315`, `compiler/stmt_loop.rs:37`, `:73`,
  `:228`) besides the `fn` definition at `stmt.rs:639` and the `use` at `stmt_loop.rs:20`. Reading
  each push site's arm maps them: `:72` is `StmtKind::Block` → `stmt.rs:136`; `:85` is
  `StmtKind::For` → `stmt_loop.rs:228`; `:114` is the `ForIn`/`ForOf` arm → `stmt_loop.rs:37` and
  `:73`; `:148` is the `try` block → `stmt.rs:315`. Three push sites have no entry: `:131`
  (`StmtKind::Switch`), `:166` (the `Try` arm's finalizer) and `:533` (`visit_class`). The first two
  are §2.2 rows and are B's; the third is not a defect — the class scope holds only the class-name
  binding, which `compiler/expr_class.rs:483-489` reaches by an open-coded scan for
  `kind == ScopeKind::Block && span == class.span` without entering the scope — but it is a **second
  copy of the lookup B is about to change**, so B collapses it rather than leaving a diverged one
  (CLAUDE.md *One issue, one way*). **Per-iteration loop environments are not in this set and B
  does not own them**: `:85` and `:114` *are* entered, and Slice E's defect is that one environment
  serves every iteration. Correcting a "found one per round" pattern by listing the three found so
  far would have carried that misattribution in.

| # | Slice | Primary module(s) | Slot | Tier | Deps |
|---|---|---|---|---|---|
| **0a — MERGED `658cc302`** | Compound **and logical** assignment to member targets — killed **3** panic classes (the plan had recorded 1; the other two were found while implementing and land together, same concept + same files). NB only `Dup`/`Swap` exist, so preserving `[obj key]` across the load needs a **new stack-shuffle opcode** ⇒ handler only (**`bytecode/disasm.rs` needs no arm** — it dispatches generically on `op.operand_size()`; this corrects a cost model that also mis-stated Slices 1b/6/D) | ⚠ **charter, not outcome — the landing was 36 files** (`git show --stat 658cc302`), incl. `compiler/stmt.rs` + new `stmt_loop.rs`, `vm/object_kind.rs`, `vm/interpreter.rs`, `vm/value.rs` and four `vm/host/` files; §16 has the record and §8's cold gate reasons over this column, so read §16 first. Charter was: `compiler/expr_assign.rs`, `bytecode/opcode.rs`, `vm/dispatch.rs`, `vm/tests/{mod,tests_member_compound_assign}.rs` | **new** `#11-vm-computed-compound-assignment` | T0 | — |
| **0b** | **Assignment/update target completeness — UMBRELLA, not a terminal unit.** The charter as written spanned binding-pattern storage, `IteratorClose` conformance ([C39]→[C36]), optional-call receiver preservation, parser-level early errors and catch-parameter binding initialization — and the row itself recorded that `peel_paren` does **not** reach the optional-call case, so there was no shared lowering chokepoint holding them together. Split below along the mechanism boundaries the charter already named; the sub-slices are ordered and dependent | — | **new** `#11-vm-assignment-target-completeness` (umbrella) | T1 | — |
| **0ba** | **Parser-level rejections.** One contract: an arm that accepts a token the grammar cannot derive in the context the arm was reached from must reject it there, at parse time. What each of 0ba's rows has in common is that shape — an **ungated arm**, not a shared spec clause — so the rows are §2.2's and the slice reads them there rather than restating a list here: prefix `ExprKind::Spread` (the `Ellipsis` arm at `parser/expr.rs:256-263` is ungated), `delete this.#x` (ECMA-262 §13.5.1.1), and `#x` in an object property key, whose `parse_property_key` arm reads the token without reading its caller. **Why that third row lands here rather than minting a slice**: it is the same contract in the same layer, its fix is a condition on an existing arm, and the two rows it might otherwise attach to own something else — `#11-vm-property-key-lowering-unification` owns how a *valid* `PropertyKey` lowers, and Slice 5 owns class-private *operations* and can complete without touching a non-class spelling. Adding it therefore does not bundle a second governing algorithm, and the criterion above leaves the slice terminal | `parser/expr.rs`, `parser/object.rs`, `parser/pattern.rs` | `#11-vm-assignment-target-completeness` (0ba) | T1 | — |
| **0bb** | **References through a wrapper** — the arms that test the **raw** AST and are defeated by an `ExprKind::Paren` (`ast.rs:299`) or by an optional chain, losing the operand's Reference: the parenthesized callee `(o.m)()` (`expr_member.rs:92`) and the parenthesized assign/update targets `(x)++` / `(x)+=1` / `(a[0])++` — the `peel_paren` chokepoint of §9 dec. 14 — plus `delete (o.x)` / `delete o?.x` and `typeof (missingName)` (`expr_ops.rs:114-163`), plus the flat optional member call `o.m?.()` (`expr_member.rs:180-187`), which `peel_paren` does **not** reach: `compile_optional_chain_expr`'s `prev_is_member` reads `chain[i-1]` and never consults the chain's base. One contract — an operand that evaluates to a Reference must arrive at the arm that consumes it — over two mechanisms; §6.4 edge 35 | `compiler/expr_member.rs`, `compiler/expr_ops.rs`, `compiler/expr_assign.rs`, `compiler/expr.rs` | `#11-vm-assignment-target-completeness` (0bb) | T1 | — |
| **0bc** | **Binding-pattern storage** — destructuring assignment (`[a,b]=…`, `({x}=…)`, for-of pattern heads), `for (obj.p in …)`, the dead `AssignTarget::Pattern` arm, and the destructured catch parameter `catch ({x})` / `catch ([a])` (§2.2's `stmt.rs:424-439` row; ECMA-262 §14.15.2 step 5). 0bc **gains** the catch parameter rather than finding it already in charter: it is the only slice lowering a `PatternKind` reached from a statement head (`compile_forin_left_binding`, evidence block above), matched only for `Identifier`. Carries the [C39]→[C36] `IteratorClose` conformance, which is why it waits on **Pa** — re-pointed at PR-B with the Pa/Pb inversion: what 0bc would otherwise inherit is the *inverted precedence contract*, and that contract is Pa's deliverable, not Pb's transport. Depending on Pa reaches Pb through Pa's own Deps. **Charter widened at PR-B: the `IteratorClose` obligation is over every array binding-pattern consumer, not over destructuring assignment alone.** As written it named [C39] DestructuringAssignmentEvaluation, which is the *assignment* form; the declaration form is governed by **§8.6.2** BindingInitialization step 3 and goes through the same `compile_destructure_pattern` arm, which pops an unexhausted iterator (§2.2's `stmt_destructure.rs:36-81` row) and is reached from parameter patterns, `var`/`let`/`const` declarations and both loop heads. So 0bc retiring on the assignment form alone would leave the shared arm wrong. Required regression is a **normal early exit** — `const [x] = it` over an iterator whose `return()` records the call — not only an abrupt-completion one, since [C36] is reached from step 3 on the normal path | `compiler/expr_assign.rs`, `compiler/stmt.rs`, `compiler/stmt_loop.rs`, `compiler/stmt_destructure.rs`, `compiler/expr_function.rs` | `#11-vm-assignment-target-completeness` (0bc) | T1 | **Pa** |
| **P** | **`IteratorClose` precedence convention — UMBRELLA, not a terminal unit.** The charter carried three independently actionable units across three layers with different governing algorithms: the Rust `iter_close` completion/result semantics (core `vm/`), the compiler→bytecode completion transport (`compiler/` + `bytecode/`, which §6.2a-3 records as **design work, not a sweep**, governed by the still-open §9 dec. 2b), and the WebIDL-governed conversions plus the WHATWG HTML citation retag (`vm/host/`). §6.2a-3 already names both axes — transport first, then governing algorithm; this is that partition applied to the slice table. **Pd is minted from a measurement rather than from that partition**: the order in which a `for-of`'s close and an enclosing `finally` run is ECMA-262 §14.7.5.7's, not §7.4.11's, and it is a precedence question — when the close runs relative to the rest of completion processing — so it lands under this umbrella rather than beside it. ⚠ **§6.2a-3 and its Slice-P prose stay the authoritative home for the site partition and the two commands that re-derive it** — the sub-slice rows name owners, not counts | — | **new** `#11-vm-iteratorclose-precedence-convention` (umbrella) | T1 | — |
| **Pa** | **ECMA-262 §7.4.11 `iter_close` semantics** — the completion-kind parameter on the Rust signature, the per-caller audit of the ECMA-262-governed callers, **and §7.4.11 step 7** ("If innerResult.[[Value]] is not an Object, throw a TypeError exception", `webref body ecma262 sec-iteratorclose`), which `iter_close` does not implement: at `658cc302`, `dispatch_iter.rs:362` ends the `.return()` call with `.map(\|_\| ())` and discards the result, so `{ return() { return 1 } }` is accepted. Deliverables: signature + result-object validation + a **primitive-return regression** (an iterator whose `return()` yields a non-Object must make an otherwise-normal close throw a TypeError). **Pa consumes Pb, not the reverse** (corrected at PR-B; the cells were pointed the other way): one of the ECMA-262-governed callers §6.2a-3 assigns to Pa is `dispatch_iter.rs:337`, which is the body of `op_iterator_close` (`:335`) — the `Op::IteratorClose` handler — and the opcode carries no operand, so there is no completion kind in scope for that call to pass. Pa ahead of Pb therefore hard-codes one kind at that handler and leaves the other wrong, or carries a second `iter_close` entry point until Pb replaces it: the strangler intermediate CLAUDE.md *One issue, one way* forbids. **Ordering inside §7.4.11**: step 7 sits after step 5 (a throw completion returns the *original* completion) and step 6 (an `innerResult` throw returns that), so validation must not fire on either path — read the numbered steps in the `body` output before wiring it | `vm/dispatch_iter.rs`, `vm/ops.rs`, `vm/natives_array_hof.rs`, `vm/host/typed_array_static.rs` | `#11-vm-iteratorclose-precedence-convention` (Pa) | T1 | **Pb**, **`#11-vm-typed-array-family-layering-and-gate`** (§8 — Pa edits `vm/host/typed_array_static.rs:798`, so the layering/gating decision must settle first) |
| **Pb** | **Compiler→bytecode completion transport** — `Op::IteratorClose` is operandless (`bytecode/opcode.rs:284` at `658cc302`, stack effect `[iterator -- ]`) and is emitted from a throw handler *and* the non-throw Return path, so the handler cannot tell them apart. Deliverable is the transport itself (an operand, or separate opcodes) plus the statement-lowering emit sites, with the throw and Return paths tested **separately** — a single fixed argument in the handler gets §7.4.11 step 5 right and steps 6-7 wrong, or the reverse. Governed by **§9 dec. 2b**, which is open, so Pb's plan-review settles it. **Pb is the family's first PR and is behaviour-preserving**: it widens the transport and hands the handler a completion kind it did not have, and Pa is what then makes the two kinds behave differently — so Pb ships with the handler still doing what it does today on both routes, and its own tests assert that the throw and Return routes arrive distinguishable, not that they diverge yet. That is why it depends on nothing: §6.2a-3's axis order (transport first, then governing algorithm) is a dependency, not just a reading order. ⚠ Excluded, with owners: the `expr_yield_star.rs` emits (delegation, not §7.4.11 — `#11-vm-yield-delegation-lowering`, §6.2a-3) and the inline re-implementation in `op_array_spread` (dec. 13a assigns its *removal* to Slice 1a; 1a's edge 21 verifies it) | `compiler/stmt.rs`, `compiler/stmt_loop.rs`, `bytecode/opcode.rs`, `vm/dispatch_iter.rs` | `#11-vm-iteratorclose-precedence-convention` (Pb) | T1 | — |
| **Pc** | **WebIDL-governed conversions — the deliverable is *removal*, not adaptation.** The governing algorithm is WebIDL **§3.2.21.1** "Creating a sequence from an iterable", whose entire body is `GetIteratorFromMethod`, then a repeat of `IteratorStepValue` / return-on-done / "Initialize S\_i to the result of converting next to an IDL value of type T" — **no `IteratorClose` step anywhere** (`.claude/tools/webref body webidl create-sequence-from-iterable \| grep -ci iteratorclose` → **0**; positive control `body ecma262 sec-array.from` → **1**, so the grep discriminates; and `webref dfn webidl "IteratorClose"` answers *no dfn matching 'IteratorClose' in webidl*). **§6.2a-3's "5 of the sites, across 4 files, are not governed by ECMA-262 at all" paragraph is the authoritative home of this partition and already says so**, so "what completion kind does a WebIDL caller pass" is a settled non-question and **Pc consumes nothing Pa produces**. The five sites fall in two classes. **One closes on a step that genuinely exists** — `vm/webidl_sequence.rs:149`, where the element `validator` fails: that is step 3.3's conversion to type T. **Four close on an early exit the governing algorithm does not have**: `vm/webidl_sequence.rs:141`, an **engine cap** (`index >= cap`, `cap_exceeded_msg`) with no counterpart in §3.2.21.1; `vm/host/url_search_params.rs:318` and `vm/host/headers/parse_init.rs:208`, **arity checks the two consuming specs perform on the already-converted sequence** — URL **§6.2**'s `URLSearchParams` constructor step 1 is "If init is a sequence, then for each innerSequence of init: If innerSequence's size is not 2, then throw a TypeError" and Fetch **§5.1**'s *fill a Headers object* step 1.1 is "If header's size is not 2, then throw a TypeError", each over the converted value rather than mid-drain (`webref body url interface-urlsearchparams` / `webref body fetch concept-headers-fill`; §-numbers from `webref dfn url "URLSearchParams"` / `webref dfn fetch "fill"`); and `vm/host/structured_clone.rs:1064`, an **unsupported-transferable bail-out** ("Transferable objects are not yet supported"), an engine limitation and not a spec step. All four lose the call. The residue — those four still stop the drain early, so their `next()` call count differs from a conversion that drains to done and checks afterwards — is deferred with its own §8 row, `#11-vm-webidl-sequence-early-exit-drain`. **Also a named deliverable**: retag `vm/host/structured_clone.rs`'s drifted in-code `§2.9` to WHATWG HTML §2.7.4 / §2.7.7 — `git show 658cc302:crates/script/elidex-js/src/vm/host/structured_clone.rs \| grep -c '§2\.9'` → **8**, and 0a touched none | `vm/webidl_sequence.rs`, `vm/host/url_search_params.rs`, `vm/host/headers/parse_init.rs`, `vm/host/structured_clone.rs` | `#11-vm-iteratorclose-precedence-convention` (Pc) | T1 | — |
| **Pd** | **Abrupt-exit sequencing at a `for-of` body** — the close and an enclosing `finally` run in the wrong order, and because the two are emitted as one sequence a throwing finally body suppresses the close entirely (§2.2's `compiler/stmt.rs:203-205` row). Governed by **§14.7.5.7** ForIn/OfBodyEvaluation, whose steps 8.9.6 and 8.13.5 perform the close inside the loop's own evaluation, so a completion reaches an enclosing `finally` already closed — while a `try`/`finally` nested inside the body runs first, as part of step 8.10's "Evaluation of stmt". Deliverable is nesting-aware sequencing at the arms that pre-emit an abrupt exit, with the two nestings asserted **separately** — an iterator whose `return()` records the call and a `finally` that records its own run, ordered one way for the outer nesting and the other for the inner — plus a throwing finally that still closes. **The `Continue` arm is Pd's too, and does not make Pd an umbrella** (§2.2's `compiler/stmt.rs:262-282` row): it is one of the arms that pre-emit an abrupt exit, and the only one that calls no `emit_iter_close_range` at all, so an inner `for-of` left by `continue outer` is never closed. Same governing AO, same arm family, same helper — the criterion above asks whether a row bundles ≥3 intersecting invariant axes or spans more than one governing algorithm across layers, and this adds neither. What it does add is the arm's **exit range**, which is a design question of its own rather than a copy of the `Break` arm's: **§14.7.1.1** LoopContinues returns true for the loop the label names (step 4) and false for the loops inside it, so the targeted frame is re-entered and must not close while the frames strictly inside it must — where `break` leaves the targeted frame and closes from it. Regression: a labelled `continue` out of an inner `for-of`, over iterators whose `return()` records the call at both depths, asserting the inner closed and the targeted loop's own did not. **The finalizer half of those same two arms is Pd's too, and likewise does not make Pd an umbrella** (§2.2's `compiler/stmt.rs:227` / `:263` row): both arms open with an unconditional `emit_pending_finally_bodies`, whose helper takes no target and walks the whole of `fc.finally_stack`, so a jump whose target is **inside** the enclosing `try` runs the finalizer at the jump and again when the try block completes — against ECMA-262 **§14.15.3** Runtime Semantics: Evaluation, whose `TryStatement : try Block Finally` production evaluates the Finalizer at step 2 only after step 1's evaluation of the Block completes (§-number from `webref heading ecma262 14.15.3`). That is this row's own question — *which enclosing regions does this abrupt exit actually leave* — asked of the other pre-emitted sequence, over the same arms and against the same target the close range already computes (`:231-236`), so it is the charter rather than a second governing algorithm. Deliverable is target-region-aware finalizer emission at both arms, with a regression that **stays inside the `try`** — `try { while (true) { break } log.push('after') } finally { log.push('fin') }` logging `after` before a single `fin` — against a control whose target is outside the `try`, which must still run it. **Shares a contract with Slice L in both directions, which is coordination rather than ordering**: the close range for a labelled `break` is `emit_iter_close_range(fc, target, fc.loop_stack.len())` (`:238`), whose lower bound is the label's `fc.loop_stack` index — the representation L replaces. So whichever of the two lands second carries the other's contract: L by retaining a label target's enclosing loop depth (L's row states it), Pd by deriving its sequencing against whatever representation exists at Pd's own parent HEAD rather than against the one written here. **Minted rather than folded into Pb**, per the terminality criterion above: Pb is behaviour-preserving by charter and its governing algorithm is §7.4.11's completion transport, while this is §14.7.5.7's ordering and a deliberate behaviour change. **Not ordered after Pb, though the Deps cell said so until PR-B**: "Pb rewrites the same emit sites" is a shared-arm argument, and Pd's deliverable is §14.7.5.7 sequencing, which consumes nothing Pb's transport produces | `compiler/stmt.rs`, `compiler/stmt_loop.rs` | `#11-vm-iteratorclose-precedence-convention` (Pd) | T1 | — |
| **0c** | **I-1 discharge + the permanent conformance table — UMBRELLA, not a terminal unit.** §2.5 already classifies this work as edge-dense (substitution classes × compiler files × a deliberate user-visible behaviour change per dec. 5 — the program's widest blast radius; the figures are §2.5's), and the quoted rule says the mandatory plan-review does not discharge the split. The behaviour change and the permanent test artifact are separate deliverables with separate design questions | — | (invariant, no slot) | — | — |
| **0ca** | **I-1 discharge** — re-run the §2.2 three-pass sweep against **0ca's own parent HEAD** (not `658cc302`: the ship order puts the P family and the 0b family ahead of it, so a sweep pinned to `658cc302` reconstructs a pre-0b inventory and makes 0ca double-own cases 0bb/0bc have already fixed), then convert the residue of all three substitution classes to a loud throw per dec. 5. ⚠ **Narrowed by 0a's landing** — the 9 rows marked *0a ✅ loud* in §2.2 are discharged; `compiler/expr_assign.rs` has no residue at all. ⚠ **Pass 3 must be read for arm *content*, not only arm presence** — the obligation §2.2's scope statement puts on 0ca by name. The module list opposite is the pre-0a one and is **not** the charter | sweep-derived (§9 dec. 9) — pre-0a list, **stale**: `compiler/{expr,expr_object,expr_ops,expr_class,expr_assign,stmt}.rs` **+ `vm/dispatch.rs`** (the reachable Layer-B arms: `GetPrivate`/`PrivateIn`) | (invariant, no slot) | — | — |
| **0cb** | **The permanent ES-language conformance table** (§7.2) — `vm/tests/tests_es_language_surface.rs`, row-derivation per §9 dec. 9(b) **plus** the second source §7.2 adds (§1.1/§1.2 absences, one row per Slice 7-10 slot) **plus** the rows later slices contribute under §2.2's per-slice-sweep rule, and the crash-aware outcome type of §9 dec. 16. Lands after 0ca so its `KNOWN-DIVERGENCE` rows record post-discharge behaviour | `vm/tests/` | (invariant, no slot) | — | **0ca** |
| **1a** | **Call-spread VM infrastructure** (user-adopted split, §9 dec. 6 — **no call-shape change, plus two named semantic fixes**: decs. 13a + 10; NOT unqualified "behaviour-preserving", see §6.4): `lay_out_call_args` stack-layout helper + `Empty` normalisation + convert `op_super_call_spread` to consume it + correct `op_super_call_spread`'s falsified docstring (the `expr_class.rs:145-152` producer is **spec-required** and is NOT folded — §6.3 / I-3 carve-out) + `ic_call`/`ic_call_method` → `call_ic_idx: Option<usize>` (dec. 11) + remove `op_array_spread`'s `return()` (dec. 13a) + **rooting** the 4 unrooted arg windows (dec. 10 — ⚠ *not* `gc_enabled` bracketing; that was overturned in round 8 because `:893`/`:658` hand off to `make_async_coroutine_and_drive`, which drives the async body) | **`vm/dispatch_helpers.rs`** (home of `lay_out_call_args` — the proven cohesion seam, §5 1000-line note), `vm/dispatch_class.rs`, `vm/dispatch_iter.rs`, `vm/dispatch_ic.rs`, `vm/dispatch.rs`, `vm/interpreter.rs` — **no `compiler/` file** (the fold is withdrawn; edge 32 is a test) | `#11-vm-call-spread-arguments` (shared with 1b) | T1 | — |
| **1b** | **Call-argument spread — compiler + opcodes**: `compile_call_arguments`/`ArgsForm` + `emit_call` aggregation (dec. 2) + `CallMethodSpread` (dec. 2b) + the 3 handlers + arity-based form selection **+ the I-3 tagged-template input contract, as a deliverable rather than a carry-over note**. §3's TemplateLiteral rows record that `ArgsForm` as specified cannot express GetTemplateObject's `« siteObj »` prefix followed by the substitutions, and §11 assigned the question to 1b — but §6.3 still specifies only `compile_call_arguments(…) -> ArgsForm`, so 1b could satisfy its charter and its tests without producing the path I-3 requires, leaving Slice 4 to change the helper or add a second argument-emission mechanism (the strangler shape I-3 exists to prevent). 1b **defines and tests** the fixed-prefix / item-iterator input contract; its *shape* is 1b's own plan-review to settle. Acceptance condition Slice 4 consumes: §6.3 | `compiler/expr_member.rs`, `expr.rs`, `bytecode/opcode.rs`, `bytecode/disasm.rs`, `vm/dispatch.rs`, **`vm/ops.rs`** (`do_new` §3 rows 17/18, the bound-prefix splice `:696-700` for edge 26, and dec. 12's stack bound — which has no implementation today) | `#11-vm-call-spread-arguments` | T1 | **1a** (`lay_out_call_args`) |
| **2** | **Class field initializers + static-block lowering — UMBRELLA, not a terminal unit.** Assigning generic static-block lowering here (§8) put two mechanisms in one row: field initialization turns on base/derived construction timing and custom-element receiver identity, while a static block is a separate lexical function-scope boundary with its own binding isolation and capture. Different governing algorithms — ECMA-262 §15.7.10 / §7.3.32 against §15.7.11 — so they are minted separately | — | **adopt** `#11-step9-class-extras` (umbrella) | T1 | — |
| **2a** | **Class field definition and initialization — UMBRELLA, not a terminal unit.** The charter as written spanned what a field record *is* and when its name is computed, *when* instance elements are initialized (base construction against derived `super()`), the receiver a static initializer runs against, and which receiver a custom-element upgrade substitutes — across `compiler/expr_class.rs`, core `vm/dispatch_class.rs` and `vm/host/custom_elements/`. More than one governing algorithm across layers, so the criterion above makes it an umbrella; its sub-slices are minted from the clause boundaries rather than one per spelling, per the grouping rule above. The clauses that draw those boundaries: **§15.7.14** ClassDefinitionEvaluation and **§15.7.10** ClassFieldDefinitionEvaluation decide the record and its definition-time name, **§10.2.2** `[[Construct]]` and **§7.3.33** InitializeInstanceElements decide when instance elements run, §15.7.14 step 32 with **§7.3.32** DefineField decides the static receiver, and the custom-element receiver is a host-layer identity question `construct_synchronous` decides (§-numbers from `webref aoid ecma262 ClassDefinitionEvaluation` / `ClassFieldDefinitionEvaluation` / `InitializeInstanceElements` / `DefineField` and `webref heading ecma262 10.2.2`) | — | `#11-step9-class-extras` (2a, umbrella) | T1 | — |
| **2aa** | **The field record.** A class field's name is computed **once, at class-definition time**, and retained for later instances; the baseline emits nothing at all for a non-static field — `ClassMemberKind::Property` (`compiler/expr_class.rs:393`) sits wholly inside `if *is_static` (`:402`) and `:428` reads "Non-static properties: skip (would need field initializer injection)". ECMA-262 **§15.7.14** step 26 runs ClassElementEvaluation over every class element at definition time; **§15.7.10** step 1 is "Let name be ? Evaluation of ClassElementName" and step 4 returns a ClassFieldDefinition Record carrying it as `[[Name]]`; §15.7.14 step 30 sets `ctorFunc.[[Fields]]` to that list. Deliverable is that record and its place on the constructor — an implementation that injects initializers into the constructor body passes a field-*value* test while recomputing the key per construction (evidence block above). Regression: `let n=0; class C { [++n]=1 } new C; new C; n===1`. First in the family, because nothing below can run a field list that does not exist | `compiler/expr_class.rs`, `bytecode/`, `vm/dispatch_class.rs` | `#11-step9-class-extras` (2aa) | T1 | — |
| **2ab** | **When instance elements are initialized** — base against derived. ECMA-262 **§10.2.2** `[[Construct]]` initializes a base class's instance elements **before** the constructor body and performs the explicit-Object-return substitution **afterwards**; a derived class initializes when `super()` establishes `this`, via **§7.3.33** InitializeInstanceElements. So `class A { x = 1; constructor() { return {} } }` must not add `x` to the returned object, a base constructor body must observe `this.x`, and a derived constructor that returns an object without calling `super()` must not acquire fields — three regressions, one per shape. ⚠ The paragraph below this table headed *field-initialisation ordering* is this row's, and the reading it corrects — apply fields to `construct_synchronous`'s final, post-substitution value — is what this row must not ship. **The custom-element upgrade path is a no-regression test inside this row, not a slice of its own**: on that path the receiver and the pre-allocated instance are the same object (the paragraph below this table headed *custom-element upgrade receiver* is the measurement), so a host-layer slice would restate this row's receiver decision across a layer boundary with no artifact passing between the two. ⚠ **§10.2.2's this-binding *status* is not this row's.** All three regressions above pass while `this` stays readable before `super()`, because `do_new` preallocates the receiver for a derived constructor too — so retiring 2ab does not close it. `#11-vm-derived-constructor-this-binding` (§8) owns that contract and carries the carve reason | `vm/dispatch_class.rs`, `vm/interpreter.rs`, `vm/dispatch.rs`, `vm/host/custom_elements/` | `#11-step9-class-extras` (2ab) | T1 | **2aa** |
| **2ac** | **The static-element receiver.** The static initializer is compiled with the **enclosing** `FunctionCompiler` (`compiler/expr_class.rs:411` computed-key branch, `:418` otherwise, at `658cc302`), so `ExprKind::This` inside it reads the surrounding function's receiver and `class C { static x = this }` gives `C.x === C` → `false` (§2.2's `expr_class.rs:402-427` row). ECMA-262 **§15.7.14** step 32 performs DefineField on `ctorFunc` for each static field and **§7.3.32** DefineField calls the initializer with the field receiver. The mechanism already exists one arm over: the static **block** arm at `:448-471` builds a child compiler and invokes it with the ctor as `this` (`:454` `Op::Dup`, `:470` `Op::CallMethod`), and the static-field path does not use it. **The receiver contract covers the `PrivateField` static arm as well as the public `Property` one.** `:438` compiles a `static #x = …` initializer with the same enclosing `fc` (`git show 658cc302:crates/script/elidex-js/src/compiler/expr_class.rs \| sed -n '430,447p'`), and §7.3.32 DefineField reaches the initializer **before** it branches on the name — step 3.a is "Let initValue be ? Call(initializer, receiver)", the Private Name test is step 5 (`webref body ecma262 DefineField`) — so one receiver decision governs both arms. §2.2's `expr_class.rs:430-447` row covers that arm for a **different** defect (the missing non-static branch) and belongs to Slice 5, so without this sentence no row owns the private static receiver; it is unobservable today only because the private read path is the stub Slice 5 implements. **No dependency on 2aa**, though the Deps cell named it until PR-B: §15.7.14 step 30 sets `ctorFunc.[[Fields]]` to **instanceFields** only, static elements going to `staticElements` for step 32 (`webref body ecma262 ClassDefinitionEvaluation`), so 2aa's field record is not on this row's path. The class-name-ordering prerequisite the evidence block above states is **shared with Slice 2b and a member of neither charter** — that block says so in as many words — and it is owned by `#11-vm-class-name-binding-initialization-order` (§8) rather than by this row | `compiler/expr_class.rs` | `#11-step9-class-extras` (2ac) | T1 | — |
| **2b** | **Generic class static-block lowering** — `expr_class.rs:457` gives the block's child `FunctionCompiler` the *enclosing* `func_scope_idx`/`current_scope_idx` although `scope/visitor.rs:555` allocated the block its own function boundary, so block-local `let`/`var` resolve against the wrong scope (ECMA-262 **§15.7.11** step 5). Binding isolation and capture are the tests that retire the slot's static-block facet | `compiler/expr_class.rs` | `#11-step9-class-extras` (2b) | T1 | — |
| **3** | Super property references | `compiler/expr.rs`, `expr_member.rs`, `vm/dispatch.rs`, **+ the frame-state axis: `vm/interpreter.rs:795-806`, `vm/value.rs:1038-1046`, `bytecode/compiled.rs:74`** | **adopt** `#11-step9-class-extras` | T2 | — |
| **4** | **Tagged templates + `String.raw` — UMBRELLA, not a terminal unit.** The charter bundled three mechanisms that share no lowering chokepoint: the template object [C23] (a per-realm, per-call-site cached Array, frozen, carrying a frozen `raw`), the tagged-call lowering [C40] (which reaches [C20] EvaluateCall with the tag's **Reference**, so a property-reference tag keeps its receiver), and `String.raw` [C44] (an ordinary builtin, installed and coerced like any other). They sit in the compiler, in per-realm runtime object state, and in builtin installation respectively. So the charter can pass a basic tag-call test while the template object is freshly built on every evaluation, or mutable, or while ``obj.tag`x` `` calls the tag with the wrong receiver — three ways to be wrong that no one regression sees. More than one governing algorithm across layers ⇒ umbrella under the criterion above; the split is by governing algorithm, and the sub-slices are ordered | — | `#11-vm-tagged-template-literals` (umbrella) | T1 | — |
| **4a** | **The template object** — [C23] **§13.2.8.4** GetTemplateObject. Its step 3 returns the already-registered Array when `[[Site]]` is the same Parse Node and step 16 appends the Record to `realm.[[TemplateMap]]`, so the object is keyed by call site *and* realm and is the **same object** on every evaluation of that site; steps 12.c/12.e define the cooked and raw elements non-writable and non-configurable, step 13 freezes `rawObj`, step 14 defines `"raw"` non-enumerable and non-writable, step 15 freezes `template`. Carries the per-realm registry the family needs, which is why 4b depends on it rather than building a second one (CLAUDE.md *One issue, one way*). The acceptance shape those steps require is an **identity** assertion across two evaluations of one site and a **frozen** assertion — never that a tag received an array | `vm/`, `compiler/expr.rs` | `#11-vm-tagged-template-literals` (4a) | T1 | — |
| **4b** | **The tagged-call lowering** — [C40] **§13.3.11.1**, whose `MemberExpression TemplateLiteral` and `CallExpression TemplateLiteral` productions both evaluate the tag to a **Reference** at step 1 and pass that reference to `EvaluateCall(tagFunc, tagRef, TemplateLiteral, tailCall)` at step 5, so [C20] **§13.3.6.2** step 1.a.i `GetThisValue` is what supplies the receiver. The `1b` edge is on 1b's **I-3 fixed-prefix / item-iterator input contract** for `compile_call_arguments` / `ArgsForm`, which 1b's row carries as a named deliverable: I-3 routes 4b's tagged-template argument list through that one helper, so 4b consumes the input shape 1b defines and tests. The baseline emits `Op::PushUndefined` for the whole node (`compiler/expr.rs:237-239` — §2.2), so the tag is never called at all. Consumes 4a's object as the first argument, substitutions after it | `compiler/expr.rs`, `vm/dispatch.rs` | `#11-vm-tagged-template-literals` (4b) | T1 | **4a**, 1b (I-3 helper) |
| **4c** | **`String.raw`** — [C44] **§22.1.2.4**, which is **not** on the tag path: it is an ordinary builtin whose steps 2-4 are `ToObject(template)`, `ToObject(Get(cooked, "raw"))` and `LengthOfArrayLike(literals)`, so it operates on any object carrying a `raw` property and needs its own installation and its own coercion behaviour, not 4b's lowering. Absent at `658cc302`: `git grep -n '"raw"' 658cc302 -- crates/script/elidex-js/src/vm` reaches only an unrelated SubtleCrypto key-format arm (`vm/host/subtle_crypto/marshal/key.rs:89`), while the same grep shape for a sibling String static (`'"fromCharCode"'`) reaches `vm/globals_primitives.rs`. **Independent of 4a**, though the Deps cell named it until PR-B: the algorithm reaches its argument only through `ToObject(template)` (step 2), `ToObject(? Get(cooked, "raw"))` (step 3) and `LengthOfArrayLike(literals)` (step 4), i.e. ordinary property access on any object carrying `raw`, and the only mention of tagged templates in the clause is its closing **Note**, which is non-normative (`webref body ecma262 sec-string.raw`) | `vm/globals_primitives.rs` | `#11-vm-tagged-template-literals` (4c) | T1 | — |
| **5** | Private names complete. **The `2ab` edge names an artifact** (it did not before; the row's only justification was §5 prose): a private *instance* field `#x = 1` is initialized by ECMA-262 **§7.3.33** InitializeInstanceElements, whose steps 3-4 walk `ctor.[[Fields]]` and perform `DefineField(obj, fieldRecord)`, and **§7.3.32** DefineField step 5.a is `PrivateFieldAdd(receiver, fieldName, initValue)` — so the invocation point 2ab decides is on this slice's path (`webref body ecma262 InitializeInstanceElements` / `DefineField`). **And a `2ac` edge**: `class C { static x = this }`'s wrong receiver is present on the `PrivateField` static arm too (2ac's row), and it is **this slice that unmasks it**, because the private read path is the stub below | `compiler/expr_class.rs`, `expr_member.rs`, `expr_ops.rs`, `expr_assign.rs:202-206`, **`vm/dispatch.rs:1011-1026`** (the `GetPrivate`/`SetPrivate`/`PrivateIn` stub — the start line was cited as `:1020`, which is a `self.pop()?;` inside the `SetPrivate` branch; re-derived with `git show 658cc302:crates/script/elidex-js/src/vm/dispatch.rs \| grep -n 'Op::GetPrivate'` → **1011**, same at HEAD. 0c only makes the reachable arms loud; the implementation is this slice's, and `SetPrivate` is additionally an I-4 connect) | `#11-vm-class-private-fields` + `#11-step9-class-extras` | T1 | **2ab**, **2ac** |
| **6** | **Async generators + async iteration — UMBRELLA, not a terminal unit.** Three algorithm families intersect here: async-generator objects (ECMA-262 **§27.9** *AsyncGenerator Objects*), the sync→async iterator adapter (**§27.1.5.1** CreateAsyncFromSyncIterator) and the async form of ForIn/OfBodyEvaluation — ⚠ **both numbers were drifted and are re-derived here**: `webref heading ecma262 27.6` is *GeneratorFunction Objects* and `27.1.4` is *The %AsyncIteratorPrototype% Object*, while `webref heading ecma262 27.9` gives *AsyncGenerator Objects* and `webref aoid ecma262 CreateAsyncFromSyncIterator` gives §27.1.5.1 (consistent with 6a's own §27.9.x citations) — and the evidence block above adds a further axis inside the first, two call entry points rather than one | — | **new** `#11-vm-async-generators` (umbrella) | T2 | — |
| **6a** | **Async-generator objects** — the distinct construction path in **both** call entry points (evidence block above), the `ObjectKind`, and the natives. **The acceptance condition is the request protocol, not a callable member** (rewritten at PR-B; "so `ag().next` becomes callable" is a member test, and routing async generators through the existing synchronous generator path satisfies it). ECMA-262 **§27.9.1.2** `%AsyncGeneratorPrototype%.next` creates a promise capability at step 2, performs **§27.9.3.4** `AsyncGeneratorEnqueue` at step 8, resumes at step 9 *only* when the state is suspended-start or suspended-yield — step 10 asserting the alternative is executing or draining-queue — and returns `promiseCapability.[[Promise]]` at step 11; so a call arriving while an earlier one is in flight is queued rather than run, and `.return` (**§27.9.1.3**) and `.throw` (**§27.9.1.4**) enqueue against that same queue. In-tree at `658cc302` there is no queue to enqueue against and no promise to return: `grep -rl 'AsyncGeneratorEnqueue' crates/script/elidex-js/src` → **0** files, and `native_generator_next` (`vm/natives_generator.rs:356`) resumes at `:374` and returns `create_iter_result` directly at `:375-376` — a synchronous resume handing back an iterator-result object, which is exactly the shape a member test accepts. Acceptance: Promise-returning `next` / `return` / `throw`, a request queue they enqueue on, and settlement driven from yield / await / return. Regressions: `ag().next() instanceof Promise`, and two `.next()` calls issued with no `await` between them settling in call order | `vm/natives_generator.rs`, `vm/object_kind.rs`, `vm/interpreter.rs`, `bytecode/opcode.rs` | `#11-vm-async-generators` (6a) | T2 | — |
| **6b** | **`for await (x of …)` lowering** — the `is_await: _` discard and the `for-of` lowering that takes no `is_await` (evidence block above). **Does not depend on 6a**, though the Deps cell named it until PR-B, and it consumes nothing 6a produces: ECMA-262 **§7.4.4** `GetIterator(obj, async)` step 1.b routes an object with no `%Symbol.asyncIterator%` through `GetIteratorFromMethod(obj, syncMethod)` (step 1.b.iii) and `CreateAsyncFromSyncIterator(syncIteratorRecord)` (step 1.b.iv) — `webref body ecma262 GetIterator` — so `for await` over a **sync** iterable is **§27.1.5.1**'s adapter and constructs no async-generator object at all. **Coordination with 6a, not ordering**: whichever lands second must not re-derive the other's `is_await` lowering | `compiler/stmt.rs`, `compiler/stmt_loop.rs`, `vm/dispatch_iter.rs` | `#11-vm-async-generators` (6b) | T2 | — |
| **O** | **Object-spread source coercion — SCHEDULED FROM ITS OWN SLOT, outside this plan's slice sequence** (§1.0). `op_spread_object` copies only when source **and** destination are both `JsValue::Object` (`vm/dispatch_objects.rs:137` at `658cc302`), so `{...'ab'}` and `const {...r} = 'ab'` produce empty objects, against ECMA-262 **§7.3.25** CopyDataProperties. A live defect in a **connected** handler: §1.0's derivation did not produce it, §2.2 records the measurement and the §8 row carries the triple, so this row is a pointer rather than a slice | — | `#11-vm-object-spread-source-coercion` | T1 | — |
| **B** | **Block-entry lexical instantiation.** One governing AO: ECMA-262 **§14.2.3** BlockDeclarationInstantiation (`webref aoid ecma262 BlockDeclarationInstantiation`), invoked by **§14.2.2** for a Block and by **§14.12.4** step 5 for the CaseBlock. **B's inventory is derived, not listed**, and the derivation follows the AO's own two obligations. *Entering the environment*: the sites where scope analysis pushes a `ScopeKind::Block` (`git grep -n 'ScopeKind::Block' <parent> -- crates/script/elidex-js/src/scope`, less `scope/tests.rs`) against the sites where lowering enters one (`git grep -n 'find_child_block_scope' <parent> -- crates/script/elidex-js/src`, less the `fn` definition and the `use` line); a push site with no entry is B's, and each is read to say which construct's arm it belongs to. *Instantiating into it*: §14.2.3 steps 3.a and 3.b bind **and** initialize every LexicallyScopedDeclaration of the block, function declarations included at step 3.b.iv — so an entered scope whose declarations nothing writes is B's too. Both are run at **B's own parent HEAD**; what they produced at `658cc302` is in the evidence block above, and the rows they produced are §2.2's. ⚠ **A third obligation, and the two derivations above cannot produce it, because both are static.** They ask which scopes lowering *enters* and which declarations it *writes* — questions about compile-time state — while §14.2.2's `Block : { StatementList }` production makes step 2's "Let blockEnv be NewDeclarativeEnvironment(oldEnv)" part of the Block's **evaluation**, so a block that is evaluated twice must have two environments (steps from `webref body ecma262 sec-block-runtime-semantics-evaluation`; the AO number from `webref aoid ecma262 BlockDeclarationInstantiation`). In-tree there is exactly one, and the allocation is compile-time throughout, measured at `b5356098` (`git diff --stat 658cc302 b5356098 -- crates/` is empty): `StmtKind::Block` (`compiler/stmt.rs:134-144`) saves `fc.current_scope_idx`, sets it to `find_child_block_scope(…)` and restores it — a compiler variable, with no op emitted at entry or exit; `FunctionScope::locals` is a `HashMap<(usize, Atom), LocalInfo>` (`compiler/resolve.rs:31`) and `add_local` (`:95-110`) assigns one slot per `(scope_idx, name)` from `build_function_scopes`, guarded against re-entry by the `contains_key` tests at `:222` and `:238`; and `add_upvalue` (`:148-157`) deduplicates captures by that same `(scope_idx, name)` key through `upvalue_map`. So `for (let i=0;i<2;i++) { let x=i; fs.push(() => x) }` gives both closures the one slot and the one upvalue, and every closure reads the last value. **This is B's**, on the same contract — what must happen when a block scope is entered — with *entered* read as the spec's per-evaluation event rather than as a compile-time position, and on the same production's evaluation: `BlockDeclarationInstantiation ( code, envRecord )` takes as `envRecord` the environment §14.2.2 step 2 creates; step 3 instantiates into it, step 4 installs it as the running LexicalEnvironment, and step 5 evaluates the StatementList; the deliverable is a per-entry environment for a block's lexical bindings that closure capture observes, and the regression is that program, asserting the two closures return `0` and `1`. Note the failure honestly: **the two derivations above are static, and that is exactly why they returned an inventory with this absent** — every push site can have an entry site and every declaration can be written, and the result still shares one binding across entries. ⚠ **Not Slice E, and not a re-run of the note in the evidence block above**: E owns a loop *head*'s bindings under §14.7.4.4 and §14.7.5.7 step 8.h.iii, and the evidence block's "per-iteration loop environments are not in this set" stands — this is a **body-local** binding in a block, whose environment is §14.2.2's, so Ea and Eb can both retire while the program above still observes the last value. **Terminal under the criterion above**: one contract (what must happen when a block scope is entered) governed by one AO, in one layer. **Minted rather than added to an existing row**: no other row's charter is lexical-environment entry — 0bc is binding-pattern *storage* and 2b is the static-block *function* boundary — and appending a second governing algorithm to either makes it non-terminal, which the criterion above forbids | `compiler/stmt.rs`, `compiler/mod.rs`, `compiler/expr_function.rs`, `compiler/expr_class.rs`, `scope/visitor.rs` | **new** `#11-vm-block-declaration-instantiation` | T1 | — |
| **L** | **Labelled-statement break targets** — `outer: { break outer; }` is rejected at compile time, and where the labelled block contains a loop the same `break` patches the loop instead (§2.2's `compiler/stmt.rs:284-291` row). The representation is the defect: a label is stored as an index into `fc.loop_stack`, which has no entry for a statement that is neither a loop nor a switch, so the `Labeled` arm has nothing to patch and the `Break` arm has nothing valid to find. Deliverable is a label→patch-list representation that does not presuppose a loop, plus regressions for the bare labelled block, the labelled block wrapping a loop (the `break` must leave the block, not the loop), and `continue` on a non-loop label, which ECMA-262 **§14.8.1** makes an early SyntaxError. ⚠ **A label target must also retain its enclosing loop depth, and none of those three regressions detects the loss.** Today the depth *is* the representation: the `Labeled` arm records `fc.loop_stack.len()` (`compiler/stmt.rs:287-288` at `b5356098`), and the `Break` arm reads it back at `:231-236` and uses it as the lower bound of `emit_iter_close_range(fc, target, fc.loop_stack.len())` at `:238` — before `fc.emit_jump(Op::Jump)` at `:240`, so the close range is computed from the label, not from the patch. A label→patch-list representation that carries only patch points therefore leaves `outer: { for (const x of it) { break outer } }` with no way to say how many iterators the `break` exits, and all three regressions above pass without it: none constructs an iterator. So the deliverable is the patch list **plus** the depth the exit range is derived from, with a fourth regression — a `break` to a **non-loop** outer label out of a `for-of`, over an iterator whose `return()` records the call, asserting the call happened. ECMA-262 **§14.7.5.7** ForIn/OfBodyEvaluation step 8.13.5 is what requires it (`Return ? IteratorClose(iteratorRecord, status)` when `LoopContinues` is false), and **§14.13.4** LabelledEvaluation is what makes the labelled block the completion's target. **This contract runs in both directions with Slice Pd, which is coordination rather than ordering**: Pd's sequencing consumes that exit range, its row says so, and whichever of the two lands second must not silently lose the other's contract — L by writing the retention into the new representation, Pd by re-deriving the range against its own parent HEAD. **Terminal under the criterion above**: one contract (a label names a statement and `break` exits *that* statement), one governing SDO (**§14.13.4** LabelledEvaluation), one layer, and the state it changes (`label_map` / `loop_stack`, `compiler/function.rs`) is read from `compiler/stmt.rs` only — no second invariant axis is bundled | `compiler/stmt.rs`, `compiler/function.rs` | **new** `#11-vm-labelled-statement-break-target` | T2 | — |
| **E** | **Per-iteration loop environments — UMBRELLA, not a terminal unit.** A `let`/`const` loop head gets one binding for the whole loop (§2.2's `scope/visitor.rs:79-124` row), so every closure made in the body observes the last value. **Two governing algorithms decide it and they differ in substance**: **§14.7.4.4** CreatePerIterationEnvironment, reached from **§14.7.4.2** ForLoopEvaluation, makes a fresh environment per iteration and **copies the previous iteration's value forward** so the update expression can act on it; **§14.7.5.7** ForIn/OfBodyEvaluation step 8.h.iii makes a fresh environment per iteration and **instantiates the head from that iteration's value**, copying nothing. Both intersect scope analysis (one scope per loop today), the compiler's local allocation, and closure capture. More than one governing algorithm across layers ⇒ umbrella under the criterion above, and the split is by **head form**, which is exactly where the two algorithms part, rather than by loop keyword | — | **new** `#11-vm-per-iteration-loop-environment` (umbrella) | T1 | — |
| **Ea** | **The `for` head** — §14.7.4.2 / §14.7.4.4: a fresh environment per iteration for the head's lexical bindings, with the previous iteration's value copied in before the update runs. Carries the representation the family needs — a per-iteration binding that closure capture observes — which is why **Eb** depends on it rather than building a second one (CLAUDE.md *One issue, one way*). Regression: a closure captured in iteration *n* still reads iteration *n*'s value after the loop ends | `scope/visitor.rs`, `compiler/stmt_loop.rs`, `compiler/resolve.rs` | `#11-vm-per-iteration-loop-environment` (Ea) | T1 | — |
| **Eb** | **The `for-in` / `for-of` heads** — §14.7.5.7 step 8.h.iii: a fresh environment per iteration into which the head is instantiated from that iteration's value, with no copy-forward. Consumes Ea's representation. Regression: the same closure-capture assertion over `for (let k in o)` and `for (let x of it)`, so both loop forms are covered by the family | `scope/visitor.rs`, `compiler/stmt_loop.rs` | `#11-vm-per-iteration-loop-environment` (Eb) | T1 | **Ea** |
| **R** | **Explicit Resource Management — PROMOTED TO ITS OWN UMBRELLA, outside this plan's slice sequence** (the Slice M treatment). `using` / `await using` have **no AST production at all**, so the syntax is a parse error and the object surface is absent: measured at `658cc302`, `grep -rl <name> crates/script/elidex-js/src` returns **0** files for each of `UsingDeclaration`, `await using`, `DisposableStack`, `AsyncDisposableStack`, `SuppressedError`, and `grep -niE 'using\|dispose' <ast.rs>` returns nothing. Probe-measured: `using x = {…};` → `CompileError` "Expected ';'", `async function a(){ await using x = {}; }` likewise, and `typeof DisposableStack` / `AsyncDisposableStack` / `SuppressedError` / `Symbol.dispose` → `"undefined"`. Spec surface: **§14.3.1** *Let, Const, Using, and Await Using Declarations* (the declaration forms), **§27.2** *Resource Management* (the Disposable / AsyncDisposable interfaces, §27.2.1.1 / §27.2.1.2), **§27.3** *DisposableStack Objects*, **§27.4** *AsyncDisposableStack Objects*, **§20.5.8** *SuppressedError Objects* — every number from `webref heading ecma262`. **§27.2 is Resource Management, not Promise Objects (§27.5)**; Slice 9d's §8 row records that correction and this row is its first consumer, so do not "fix" §27.2 back to §27.5 here. Axes: lexer/parser productions, scope analysis (a new binding kind with a disposal list), and abrupt-completion disposal semantics that intersect `try`/`finally`, `return` and `IteratorClose` ordering ⇒ ≥3 intersecting invariant axes, so the CLAUDE.md edge-dense rule forbids one PR. **No sub-slices are minted here** — the umbrella derives them from those sections at its own start | — | **new** `#11-vm-explicit-resource-management` | T2 | — |
| **N** | **Arrow-function lexical `new.target` — SCHEDULED FROM ITS OWN SLOT, outside this plan's slice sequence** (§1.0). `op_new_target` (`vm/dispatch_class.rs:29-37`) reads only the current frame's `CallMode` and `FunctionObject` has no lexical `new.target` beside `captured_this` (`vm/value.rs:598`), against ECMA-262 **§9.4.5** GetNewTarget. Same routing as **O**: a connected handler, measured in §2.2, triple in §8. It shares the closure-capture seam (`FunctionObject`, `vm/value.rs`) and both call entry points with **Slice 3**, which threads `[[HomeObject]]` through the same two — different AO, same seam, so the two coordinate rather than depend | — | `#11-vm-arrow-lexical-new-target` | T1 | — |
| **S** | **Lexical `super` through an arrow** — `(() => super())()` in a derived constructor and `(() => super.x)()` in a method (§2.2's `parser/arrow.rs:104-119` row). One contract with one governing AO: **§9.4.3** GetThisEnvironment, the outward walk that skips a record with no `this` binding, which **§13.3.7.2** GetSuperConstructor step 1 and **§13.3.7.3** MakeSuperPropertyReference step 1 both start from — so an arrow's `super` resolves against the enclosing method or constructor. Deliverable is that link carried on the closure rather than read off the current frame, threaded through both call entry points, with a regression for each spelling. **Minted rather than folded into Slice 3**, per the terminality criterion above: Slice 3 owns super-*property* emit and dispatch ([C13], [C24], [C25]) and the paragraph below this table already adds a frame-state axis to it, so adding the super-*call* clause and the environment walk gives it a second governing algorithm across a further layer — an umbrella's business to mint, not a row's to absorb. Shares the closure-capture seam (`FunctionObject`, `vm/value.rs`) and both call entry points with Slices **3** and **N**; that is coordination, not ordering. Depends on **3** because the `super.x` spelling has no implementation to make lexical until 3 lands | `vm/value.rs`, `vm/interpreter.rs`, `vm/dispatch_class.rs`, `compiler/expr_class.rs` | **new** `#11-vm-arrow-lexical-super` | T2 | **3** |
| **7** | **`Map`/`Set`/`WeakMap`/`WeakSet` — UMBRELLA, not a terminal unit.** The weak half intersects a different subsystem: entry liveness is a GC-tracing invariant, and §5's 1000-line note puts `vm/gc/collect.rs` (2074) and `vm/gc/trace.rs` (1255) in this row's reach. Keyed-collection semantics and GC liveness are separate invariant axes. **It had no derivation at all until PR-B** — it named two sub-slices and let their charter sentences stand as the scope, which is the member-shaped failure §5's head now rules out. **Its scope is a derivation rule, and running that derivation is its first step**: *the property surface and the abstract operations ECMA-262 **§24** *Keyed Collections* defines — the constructor clauses and the operations they invoke as much as the prototype clauses* (`webref heading ecma262 24`), run against the slice's own parent HEAD at the slice's start. That derivation is what mints the sub-slices; the 7a/7b rows below are the sample the umbrella was written from, so a member or an algorithm absent from them is *in scope and un-minted*, never out of scope | — | **new** `#11-vm-map-set-collections` (umbrella) | T3 | — |
| **7a** | **`Map` / `Set`** — SameValueZero keying, insertion-order iteration, and the entries/keys/values iterator surface; that clause is the **seed**, not 7a's charter, which comes from the umbrella's §24 derivation. No GC-tracing change | `vm/natives_*`, `vm/object_kind.rs` | `#11-vm-map-set-collections` (7a) | T3 | — |
| **7b** | **`WeakMap` / `WeakSet`** — weakly-held keys, so the deliverable is the GC-tracing side: entries must not keep their keys alive, and collection must remove them. **Does not depend on 7a**, though the Deps cell named it until PR-B — the same shape as the already-withdrawn `10b → 7b`: 7a's row ends "No GC-tracing change", so it produces nothing this row's deliverable consumes | `vm/natives_*`, `vm/object_kind.rs`, `vm/gc/` | `#11-vm-map-set-collections` (7b) | T3 | — |
| **8** | **RegExp completion — UMBRELLA, not a terminal unit.** The evidence block above shows the charter spanning two mechanisms in different files: the RegExp instance/constructor surface, and the well-known-symbol **dispatch** path inside the String methods. The second is not a RegExp deliverable at all — it is how `String.prototype.match`/`replace` find a method — so one PR would either skip it or rewrite string dispatch under a RegExp charter. **Its scope is a derivation rule, and running that derivation is its first step** — not the sub-slice rows below, which are what the §1.1 probe seeded. The rule, given Slice 9 explicitly excludes surfaces another slot owns (so 8a/8b can retire while a protocol stays missing and no row notices): *ECMA-262 **§22.2.6** Properties of the RegExp Prototype Object — accessors, methods and the `%Symbol.*%` protocol entries alike — plus, for each `%Symbol.*%` entry there, the **§22.1.3** String-prototype method that dispatches to it, **plus the ECMA-262 §22.2.7 *Abstract Operations for RegExp Matching* those members delegate to**, each read for what it computes and returns*, run against the slice's own parent HEAD at the slice's start. That derivation is what mints the sub-slices; a member or an algorithm absent from the rows below is *in scope and un-minted*, never out of scope. **The §22.2.7 clause is what makes the rule bite and it was absent until PR-B** — §5's head now states the general form. Read over §22.2.6's members alone, the rule is discharged by `RegExp.prototype.exec` being present; but **§22.2.6.2** `exec` is four steps ending in `Return ? RegExpBuiltinExec(regexp, string)` (`webref body ecma262 sec-regexp.prototype.exec`), so what the member *does* is **§22.2.7.2**, and that is where 8a's named-capture deliverable was hiding. ⚠ **The seed is not the boundary**: the probe found `.global`/`.ignoreCase`/`.multiline`/`.sticky` and `@@match`/`@@replace`, and re-running the greps at `658cc302` finds more of the same class — `grep -rl <name> crates/script/elidex-js/src` returns **0** files for each of `dotAll`, `hasIndices`, `Symbol.search`, `Symbol.split` and `Symbol.match`/`Symbol.replace`/`Symbol.matchAll`, and probe-measured `typeof /a/s.dotAll`, `typeof /a/d.hasIndices`, `typeof /a/v.unicodeSets`, `typeof /a/g.global` and `typeof Symbol.search` / `Symbol.split` / `Symbol.matchAll` are all `"undefined"`. *(`unicodeSets` returns **4** files, but reading them shows all four are v-flag **engine** internals — `regexp/class_set.rs:1`, `regexp/unicode_property.rs:134`, `regexp/tests.rs:768`, `parser/expr_tests_special.rs:210` — not the §22.2.6.19 prototype accessor, which is absent like the rest. A file count is not an accessor.)* | — | `#11-vm-regexp-constructor-and-flags` (umbrella) | T3 | — |
| **8a** | **RegExp constructor + flag accessors** — derives its inventory from ECMA-262 **§22.2.6** at its start (the accessor half: `§22.2.6.3` `dotAll` … `§22.2.6.19` `unicodeSets`), rather than from the probe list `.global` / `.ignoreCase` / `.multiline` / `.sticky` that seeded it; plus the constructor surface. What `exec` *returns* is **not** 8a's — that is §22.2.7.2's algorithm and it is **8c**, so 8a retiring says nothing about it | `vm/natives_regexp.rs`, `vm/globals_primitives.rs` | `#11-vm-regexp-constructor-and-flags` (8a) | T3 | — |
| **8b** | **Well-known-symbol dispatch for the String↔RegExp protocols** — the missing symbol ids and the direct `ObjectKind::RegExp` special-casing inside the String methods. Measured at `658cc302`: `WellKnownSymbols` (`vm/well_known.rs:1552-1585`) allocates **7** symbols — `iterator`, `asyncIterator`, `hasInstance`, `toPrimitive`, `toStringTag`, `species`, `isConcatSpreadable` — so `@@match`/`@@replace`/`@@search`/`@@split`/`@@matchAll` cannot even be *installed*, let alone dispatched; and the consumers bypass the protocol structurally: `native_string_search` (`natives_string.rs:633`) tests `ObjectKind::RegExp` directly at `:665` and otherwise `ToString`-coerces, while `native_string_split` (`:350`) `ToString`-coerces the separator at `:356` before any lookup. ECMA-262 **§22.1.3.21** `String.prototype.search` step 3.a is `GetMethod(regexpOrPattern, %Symbol.search%)` with the `ToString` only at step 4, and step 6 is `Invoke(regexp, %Symbol.search%, « string »)`; **§22.1.3.23** `split` is the same shape (`GetMethod(separator, %Symbol.split%)` at step 3.a, `ToString(separator)` at step 6); **§22.1.3.14** `matchAll` at step 3.c. The RegExp-side implementations are §22.2.6.8 / .9 / .11 / .12 / .14. Deliverable is `GetMethod`-based dispatch and the symbol ids it needs, with the RegExp-side methods it then reaches — inventory derived from §22.2.6's `%Symbol.*%` entries at the slice's start. **The `8a` edge names an artifact** (it did not before): three of those RegExp-side methods read the `flags` accessor **§22.2.6.4** `get RegExp.prototype.flags`, which is in 8a's accessor half — **§22.2.6.8** `%Symbol.match%` step 4, **§22.2.6.11** `%Symbol.replace%` step 7 and **§22.2.6.14** `%Symbol.split%` step 5 are each `Let flags be ? ToString(? Get(regexp, "flags"))` (`webref body ecma262` over the three anchors; the accessor's number from `webref heading ecma262 22.2.6`). **And an `8c` edge, previously undrawn because it crosses a family line**: §22.2.6.11 step 10 is `Let namedCaptures be ? Get(result, "groups")`, and `groups` is 8c's sole deliverable — so without it 8b lands and retires with named-capture substitution silently dead | `vm/well_known.rs`, `vm/natives_string.rs`, `vm/natives_regexp.rs` | `#11-vm-regexp-constructor-and-flags` (8b) | T3 | **8a**, **8c** |
| **8c** | **Named-capture result construction** — the first output of the §22.2.7 clause the umbrella's rule gained at PR-B. ECMA-262 **§22.2.7.2** RegExpBuiltinExec step 30 creates the `groups` object when the pattern contains a `GroupName` (step 31 leaves it `undefined` otherwise), step 32 defines it on the result array, and step 34.5 defines each matched `GroupName` on it. At `658cc302` the result array stops before that: `native_regexp_exec` (`vm/natives_regexp.rs:93`) defines `index` at `:128` and `input` at `:136` through `ctx.vm.well_known.*` and returns at `:144`, while `git show 658cc302:crates/script/elidex-js/src/vm/natives_regexp.rs \| grep -c '"groups"'` → **0** and `grep -c 'groups' crates/script/elidex-js/src/vm/well_known.rs` → **0** — neither of the two spellings the file uses for the other properties defines this one, so a named-capture read off the result resolves to nothing. **Minted rather than folded into 8a, per §5's terminality criterion**: the name→index pairing step 34.5 consumes does not exist below the JS surface either — `grep -rn 'GroupKind::Named' crates/script/elidex-js/src` returns **2** lines, `regexp/parser.rs:450` producing the variant and `regexp/tests.rs:110` asserting it, and `NamedBackreference` reaches only its declaration (`regexp/mod.rs:46`) and its producer (`regexp/escape.rs:80`), so the name does not leave the parser. That makes this a second governing algorithm over a second layer against 8a's accessor charter, which the criterion says is an umbrella's business to mint, not a row's to absorb. Deliverables: the name→capture-index pairing carried through to exec, the `groups` object per those steps, and a regression asserting a named capture is readable off the result of a matching `exec`. §10's disposition table routes *RegExp named-groups* to "Slice 8", but a round record cannot own work, because nothing re-reads it at re-eval time (§10's own preamble) — this row is the owner | `vm/natives_regexp.rs`, `regexp/` | `#11-vm-regexp-constructor-and-flags` (8c) | T3 | — |
| **9** | **Builtin prototype/static surface — UMBRELLA, not a terminal unit.** The charter as written spanned the Array / String / Object prototype-and-static surface, plus iterator-protocol semantics (`Object.fromEntries`), plus RegExp-argument dispatch inside string methods (`matchAll` / `replaceAll`) — intersecting invariant axes that the CLAUDE.md *Edge-dense work* rule forbids in one PR, and that rule states requiring another plan-review does not discharge the split. **Its scope is a derivation rule, and running that derivation is its first step** — not the sub-slice rows below. The rule, **widened again here because two successive wordings were each under-scoped**, and it is now a **user-reachable** rule rather than a global-rooted one: *for every intrinsic object a program can obtain — by reading a global property, by evaluating syntax that produces it, or by reading a property of a value it already holds — the whole property surface ECMA-262's main body defines on it* — **minus Annex B (I-6)**, **minus any surface another slot already owns**. What puts a surface inside the rule is the spec clause that defines properties on such an object, so the rule reads the clause and never the object's shape, nor the route by which a program reached it. Each wording failed on the axis it did not name: "constructor / prototype surface" could not reach an object that is neither, which is how `Math` sat unowned through ten rounds (measurement **(4)** in the §8 row), and "reachable from the global object" could not reach an intrinsic that is never a global property, which is measurement **(5)**. The umbrella runs it; what the rule returns is a set of **groupings**, and terminal sub-slices are minted from them by applying the terminality criterion at the head of this section — not one per family, for the reason and with the `Object` demonstration stated there. Each minted sub-slice is its own PR under its own mandatory `/elidex-plan-review`; this umbrella is the approved parent that makes each of them a base case. The sub-slice rows below are **what the derivation has produced so far** and are groupings themselves, so a family absent from them is *in scope and un-minted*, never out of scope — the `Promise` row was minted exactly that way after the enumeration read as the boundary. ⚠ **Edition-agnostic charter.** The unit is the *property surface ECMA-262's main body defines on a builtin object*, not an ECMAScript edition: the ES2021-2024 probe list is the **seed that found the class, not the boundary**. ⚠ **Minus Annex B (I-6)**, program-wide across every sub-slice: "derive a complete builtin inventory from the spec" executed literally pulls in §B.2.1 Additional Properties of the Global Object and §B.2.2 Additional Properties of the String.prototype Object (`webref heading ecma262 B.2.1` / `B.2.2`), plus `Date.prototype.getYear`/`setYear` and `RegExp.prototype.compile` — LegacySemantics compat territory, which §4 I-6 forbids program-wide. The inventory is the **main-body** surface. ⚠ **The `{1n:…}` key fix is NOT here.** It is not builtin work: the arm sits inside `expr_object.rs`'s `PropertyKind::Init` key match, so it belongs to `#11-vm-property-key-lowering-unification` (§2.2, §8). ⚠ **An ordinary function's own `name` and `length` are NOT here either.** The rule above reads the clauses that define properties *on an intrinsic object*; **§10.2.9** SetFunctionName and **§10.2.10** SetFunctionLength define these on every function syntax creates, reached from **§10.2.3** OrdinaryFunctionCreate step 22 and **§15.2.4** / **§15.2.5** — outside the rule's reach by construction, so this umbrella can retire with its inventory complete and `f.name` still absent. `#11-vm-ordinary-function-name-length` (§8) owns them | — | **new** `#11-vm-builtin-prototype-static-sweep` (umbrella) | T3 | — |
| **9a** | **`Array` prototype/static surface.** Seeded by the §1.2 absence probe (`at` / `findLast` / `findLastIndex` / `toSorted` / `toReversed` / `toSpliced` / `with`); derives its own inventory at its start from ECMA-262 **§23.1.2** (Properties of the Array Constructor = the statics) and **§23.1.3** (Properties of the Array Prototype Object), rather than from that probe | `vm/natives_array.rs` | `#11-vm-builtin-prototype-static-sweep` (9a) | T3 | — |
| **9b** | **`String` prototype surface.** Derives its inventory from ECMA-262 **§22.1.3** at its start. Named deliverables: `String.prototype.matchAll` (absent) and `String.prototype.replaceAll`'s RegExp behaviour — measurements in the §8 slot row, which is their permanent home — **plus the `natives_string.rs` §21.1.3 → §22.1.3 citation retag** (§21.1.3 is *Properties of the Number Prototype Object*; the String prototype is §22.1.3 — both verified with `webref heading ecma262`), which lands here because Slice P does not touch this file. **Ordered after 8b**: `matchAll` is ECMA-262 §22.1.3.14, whose step 3.c is `GetMethod(regexpOrPattern, %Symbol.matchAll%)`, and `replaceAll`'s RegExp behaviour is §22.1.3.20 — both consume the well-known-symbol dispatch 8b owns, so implementing them first would either re-open-code the `ObjectKind::RegExp` special-casing 8b exists to remove or ship a `matchAll` that ignores a user `@@matchAll` | `vm/natives_string.rs`, `vm/natives_string_ext.rs` | `#11-vm-builtin-prototype-static-sweep` (9b) | T3 | **8b** |
| **9c** | **`Object` statics.** Named deliverables: `Object.fromEntries`'s iterator protocol (ECMA-262 §20.1.2.7 step 6 → `AddEntriesFromIterable` §24.1.1.2), `Object.hasOwn`, `Object.groupBy` — measurements in the §8 slot row. **Depends on Pa, for the reason 0bc's cell already carries**: §24.1.1.2 step 2.c.ii is `Return ? IteratorClose(iteratorRecord, error)` with `error` a ThrowCompletion, and steps 2.e / 2.g / 2.i are `IfAbruptCloseIterator` (`webref body ecma262 AddEntriesFromIterable`) — every one of them a §7.4.11 close on an **abrupt** completion, which is exactly the inverted-precedence contract Pa's completion-kind parameter delivers. Writing the drain against today's `iter_close` bakes the inversion into a second surface | `vm/natives_object/` | `#11-vm-builtin-prototype-static-sweep` (9c) | T3 | **Pa** |
| **9d** | **`Promise` constructor statics.** Minted by the derivation above rather than by a probe list — an output the rows 9a-9c did not carry. Measured: `crates/script/elidex-js/src/vm/globals_async.rs:58-66` registers exactly `resolve` / `reject` / `all` / `allSettled` / `race` / `any` (identical at `658cc302` and HEAD `d49465d0` — `git diff 658cc302 HEAD -- crates/script/elidex-js/src/vm/globals_async.rs` is empty), while ECMA-262 **§27.5.4** Properties of the Promise Constructor also carries **§27.5.4.8** `Promise.try` and **§27.5.4.9** `Promise.withResolvers` — main body, not Annex B (`webref heading ecma262 27.5.4`). Both absent in-tree: `grep -rl 'withResolvers' crates/script/elidex-js/src` → **0** files; `grep -c '"try"' crates/script/elidex-js/src/vm/globals_async.rs` → **0**. Promise Objects is **§27.5**, not §27.2 — `webref heading ecma262 27.2` returns *Resource Management*, so re-derive the number rather than recalling it. ⚠ **A complete inventory is not a correct `Promise`, and this row is where §5's members-vs-semantics rule at the head of that section bites**: the rule says a derivation is stated over the algorithms the spec defines for a surface, not over the members it has, and resolution is the algorithm here. Measured at `658cc302`: `settle_promise` (`vm/natives_promise.rs:123`) adopts a resolution only when it is already an `ObjectKind::Promise` (`:155`), and the function's own doc comment states the gap — "Arbitrary thenables are not yet assimilated" (`:121`). ECMA-262 **§27.5.1.3** CreateResolvingFunctions is the governing algorithm: its resolve closure fulfils directly when "resolution is not an Object" (step 2.5), retrieves `then` as "Let then be Completion(Get(resolution, \"then\"))" (step 2.6) and **rejects with the getter's own throw value** when that completion is abrupt (step 2.7), fulfils with the resolution itself when `IsCallable(thenAction)` is false (step 2.9), and otherwise builds and enqueues **§27.5.2.2** NewPromiseResolveThenableJob (steps 2.10-2.12) — whose job calls `then` with a fresh resolve/reject pair from a second CreateResolvingFunctions (step 1.1-1.2) and routes an abrupt call result to that reject (step 1.3.1). That chain is not separable from this row's own new members: **§27.5.4.8** `Promise.try` step 6.1 and **§27.5.4.9** `Promise.withResolvers` step 5 both hand out the `[[Resolve]]` of **§27.5.1.5** NewPromiseCapability, and for `%Promise%` that function is the one **§27.5.3.1** `Promise ( executor )` step 9 obtains from CreateResolvingFunctions — so registering either member on today's `settle_promise` re-ships the same defect under a new name, which is why the extension belongs to this row rather than to a slot beside it. §-numbers from `webref aoid ecma262 CreateResolvingFunctions` / `NewPromiseResolveThenableJob` / `NewPromiseCapability`; step text from `webref body ecma262 sec-createresolvingfunctions` / `sec-newpromiseresolvethenablejob` / `sec-promise.try` / `sec-promise.withResolvers` / `sec-promise-executor`. Regressions, one per branch of step 2: `Promise.resolve({then(r){r(7)}}).then(v => globalThis.x = v)` must set `x` to `7`; a `then` **getter** that throws must reject with the thrown value; a non-callable `then` must fulfil with the object itself. The retag comes with it — `git show 658cc302:crates/script/elidex-js/src/vm/natives_promise.rs \| grep -c '§27\.2'` gives **16** lines still citing the withdrawn `§27.2` numbering. Derives its own inventory from §27.5.4 and **§27.5.5** (Properties of the Promise Prototype Object), and its algorithms from **§27.5.1** Promise Abstract Operations, at its start | `vm/globals_async.rs`, `vm/natives_promise.rs` | `#11-vm-builtin-prototype-static-sweep` (9d) | T3 | — |
| **10** | **`Proxy`/`Reflect` — UMBRELLA, not a terminal unit.** `Reflect` is a static-method surface (ECMA-262 §28.1) that ships and tests on its own; `Proxy` is an exotic-object algorithm (§10.5) whose traps intersect every essential internal method plus their invariant checks. Different governing algorithms, and the second reaches every property operation | — | **new** `#11-vm-proxy-reflect` (umbrella) | T3 | — |
| **10a** | **`Reflect` statics** (ECMA-262 §28.1) — each mirrors an essential internal method and needs no exotic object | `vm/ops_property.rs` | `#11-vm-proxy-reflect` (10a) | T3 | — |
| **10b** | **`Proxy` exotic object** (ECMA-262 §10.5) — the trap table and the invariant checks each internal method must enforce. **Does not depend on 7b**, though the Deps cell named it until PR-B: the two rows share `vm/object_kind.rs`. 7b's deliverable is weak-key GC tracing; 10b holds a Proxy's target and handler strongly and consumes nothing 7b produces. **The `10a` edge went the same way at PR-B and for the same reason**, having survived while the row withdrew its neighbour at length: `Reflect`'s statics each mirror an essential internal method and are not what a trap's default behaviour calls, so this row consumes nothing 10a produces either | `vm/object_kind.rs`, `vm/ops_property.rs` | `#11-vm-proxy-reflect` (10b) | T3 | — |
| **D** | **Dead-opcode sweep** — mechanically re-derive the §2.3 set and delete what no slice connected. ⚠ **Slice M is outside this sequence** (promoted to its own umbrella), and §2.3 assigns `Op::ImportMeta` / `Op::DynamicImport` to M's connect work — so at the documented "after 1b-5" point D would have to either delete two opcodes M owns or leave them dead and fail its own connect-or-delete criterion (Codex R1). **M's connects are a prerequisite for D.** That prerequisite now has a **§8 row with a slot id** (`#11-vm-module-opcode-connects`) instead of a by-name prose carve-out: an unschedulable dependency whose escape hatch is a permanent third state is precisely what I-4 forbids, and a carve-out stated only in prose is unretirable (§8 Forward rule). ⚠ **`Op::Wide` is in this slice's scope, not exempt** (§2.3, §4 I-4): it has zero emit sites like the rest — `git grep -n 'Op::Wide' -- crates/` returns exactly `bytecode/opcode.rs:562` and `vm/dispatch.rs:1074`, neither of them a `compiler/` emit, against the negative control `git grep -n 'Op::Pop' -- crates/script/elidex-js/src/compiler`, which returns emits in ten files — and nothing pins its discriminant. Mechanical consequences to carry, each from `git grep -n 'Wide' -- crates/script/elidex-js/src` at HEAD: `Wide` is the **last** enum discriminant (`bytecode/opcode.rs:388`, the variant immediately before the closing brace), so `from_byte`'s bound `if byte <= Self::Wide as u8` (`:396`) and the `roundtrip` test's range `for byte in 0..=Op::Wide as u8` (`:562`) both retarget to the new last variant; the `operand_size` arm `\| Self::Wide => 0` (`:494`) goes; the dispatch arm at `vm/dispatch.rs:1074-1075` goes; and the comments at `opcode.rs:13`, `:387` and `:397` go with it, while `:411`'s "excluding Wide prefix" is reworded. ⚠ **Deps re-stated as a criterion rather than a row-order range**: `after 1b-5` named no artifact and "1b-5" is a position in this table, not a set of slice ids | `bytecode/opcode.rs`, `vm/dispatch.rs` | **adopt** `#11-dead-opcode-removal` | — | every slice that connects a §2.3 opcode **and** `#11-vm-module-opcode-connects` |
| **M** | **ES modules — PROMOTED TO ITS OWN UMBRELLA, outside this plan's slice sequence** (R2 round 5). It carries the `stmt.rs:31-39` precondition, 3 §2.2 rows (⚠ two of them — the module-binding update and for-in head — are **0a ✅ loud**, so M inherits their *implementation*, not their conversion; I-2's `expr_assign.rs:102` `unreachable!` is likewise already converted), I-5's host-fetch-seam boundary, and the `ImportMeta`/`DynamicImport` connects = ≥3 intersecting invariant axes, so the CLAUDE.md edge-dense rule forbids one PR. Only the **`#11-vm-dynamic-import`** T1 carve stays homed here (0c makes `import()` loud; the Promise-returning impl belongs to the module umbrella). **Its preconditions are a derivation, and the list just given is that derivation's seed, not its output** (PR-B — Slice 9's treatment, applied here for the same reason: a fixed list of preconditions cannot say what it is missing). **The rule**: at the module umbrella's own start, walk the ES-modules grammar of ECMA-262 **§16.2** *Modules* — the productions of **§16.2.2** *Imports* and **§16.2.3** *Exports* — against `parser/module.rs` and `ast.rs` at that umbrella's parent HEAD, and table every production the parser does not accept or the AST does not represent. A precondition absent from the seed is *un-derived, never out of scope*. **Measured instance the derivation must produce**, at `658cc302`: `WithClause` is in both productions — `ImportDeclaration : import ImportClause FromClause WithClause[opt] ;` and `ExportDeclaration : export ExportFromClause FromClause WithClause[opt] ;` (`webref body ecma262 sec-imports` / `sec-exports`) — and is absent in both layers. `grep -rl 'ImportAttributes' crates/` and `grep -rl 'WithClause' crates/` each return **0** files; `ast.rs:695-699` `ImportDecl` carries only `specifiers` / `source` / `span`; `parser/module.rs:22-24` and `:86-96` call `expect_semicolon()` immediately after the module specifier. So `import data from './data.json' with { type: 'json' }` is a parse error, and the re-export form with it — a defect no fixed precondition list held, because it is a grammar production rather than a compiler arm | — | `#11-vm-dynamic-import` | T1 | — |
| **—** | `Function`/`eval` — ⚠ **had no slice, no deps and no owner (Codex R5)**; §9 dec. 7 recorded the policy decision as unowned, so I-6's core strict-mode `eval` surface could stay unimplemented forever inside a *completeness* program. Whichever trigger fires, that work opens its own umbrella (it is a policy call, not a language slice). ⚠ **The Why/Trigger/Re-eval triple now lives in §8's table**, which is this document's registry — a full deferral triple carried in a slice-table row is invisible to §8's derive recipe (Forward rule). This row is a pointer | — | `#11-vm-function-constructor-global` (§8) | policy | §9 dec. 7 |

**Ordering rationale.** 0a first on severity (process abort). *(Rounds 2-9 put a `vm/dispatch.rs`
prereq split ahead of it; removed — see the 1000-line check below.)* The 0b family next — at the baseline a silent no-op (0a has since made it a scoped throw; the *implementation* is still the family's) on the
ubiquitous swap/destructure idiom, and it shares `expr_assign.rs` with 0a; within it the three rows are
**independent of one another** — the `0bb → 0ba` and `0bc → 0bb` edges were shared-arm arguments and
were deleted at PR-B — while 0bc still waits on **Pa**, per its own cell. 0ca discharges I-1
program-wide and 0cb lands the conformance table (§7.2) so every later slice inherits a baseline.
Inside the P family the only chain is **Pb → Pa**, and it was inverted here until PR-B: **Pb** first
because it is behaviour-preserving and because Pa's own handler caller has no completion kind to pass
until Pb's transport exists (Pa's row), then **Pa**. **Pc** and **Pd** sit outside that chain, per
their rows: Pc's deliverable is the *removal* of the `iter_close` calls WebIDL §3.2.21.1 never asks
for, and Pd's is §14.7.5.7 sequencing — neither consumes what Pa or Pb produces. **Pa** must
land **before 0bc** (0bc's [C39]→[C36] conformance claim inherits the inverted contract otherwise), and
**`#11-vm-typed-array-family-layering-and-gate`** (§8) must be settled before **Pa** — before Pa rather
than before the family, since Pb's touch set reaches no `vm/host/` file at all. **1a then 1b** proceed per the registered slot ordering and standing directive. 2a/2b/3 follow
(new machinery), the 2a family ordered by its Deps column (`2aa` first, then `2ab`; `2ac` is
independent of `2aa` since PR-B). 5 depends on **2ab** and on **2ac**, for the artifacts its own
row now names. **S** follows **3**
for the reason its row states. 4b depends on **1b** (I-3
requires reusing the arg helper); Slice 4's own Deps cell is `—`. D runs after the connecting slices so it deletes only what
remains. **B**, **L** and the **E** family sit outside that chain — no row's deliverable is an input
to any of them, and inside E the Deps column orders `Ea → Eb`. **O**, **N**, **R** and **M** are
outside the sequence entirely: each is scheduled from its own owner per §1.0 and its row here is a
pointer. N's row records the seam it shares with Slice 3, which is a coordination note, not an
ordering.

**`import()` note**: tiered **T1** (silent `undefined` in ordinary script context), so it is *not*
deferred with the rest of ES modules as R1 implied. Interim: 0ca makes it throw loudly; the real
implementation lands with Slice M.

**Slice M preconditions — the rule is in M's row above; this paragraph records the one instance the
§2.2 pass-3 structural sweep produced.** `StmtKind::ImportDeclaration` and
`StmtKind::ExportDeclaration` are already grouped into `compile_stmt`'s **no-op arm**
(`stmt.rs:31-39`), while the parser and scope analysis otherwise support modules. Enabling
`parse_module` therefore makes `export const x = 1` compile to nothing **silently**. Slice M must
fix that arm **before** wiring module parsing, and its plan-memo must state so — otherwise the
module program's first milestone ships the exact silent-wrong class this umbrella exists to remove.
This instance came from a sweep over `compiler/` arms, which is why it says nothing about the module
**grammar**; the grammar walk M's row mandates is what produces the rest, and the import-attributes
measurement there is the first thing it returns.

**Slices 7-10 coupled-invariant duty**: each needs its own §2.5-style enumeration (see I-5 for the
Slice 7 / Slice 10 boundary obligations).

**Slice 3 frame-state axis** (R2 round 2): `super.x` in an *ordinary method* needs frame state that
does not exist. `vm/interpreter.rs:795-806` sets `home_class` **only on class-ctor frames**, and
`vm/value.rs:1041-1046` documents this as "fail-closed-by-construction: a non-ctor method frame has
`home_class = None`, so any future super-property reader trips a SyntaxError fallback". So Slice 3 is
⚠ **and the frame-state audit named only one of the two call entry points (Codex R5).**
`call_internal` **independently hardcodes** `home_class = None` (`vm/interpreter.rs:589`, whose own
comment says "always `None` on the `call_internal` entry") and is the `NativeContext` path — so
`Function.prototype.call` / `.apply` lose `[[HomeObject]]` even after direct method dispatch is
fixed, and a method containing `super` breaks when invoked as an extracted function. The home object
belongs on the **closure**, threaded through *both* entries (§§13.3.7.3, 9.1.1.3.5), with a
regression like `const m = new B().m; m.call(receiver)`. Original finding: Slice 3 is
**not** emit+dispatch only — it must add `[[HomeObject]]`-equivalent frame state ([C24]/[C25] both key
off it). The §2.2/§2.3 Layer-A/Layer-B decomposition has no row for this third (frame-state)
dimension; Slice 3's memo must add one.

⚠ **Slice 2ab field-initialisation ordering — the contract below is wrong as stated (Codex R2, P1).**
Applying fields to `construct_synchronous`'s **final, post-substitution** value is not what the spec
does. ECMA-262 **§10.2.2** `[[Construct]]` initialises a *base* class's instance elements **before**
the constructor body and performs the explicit-Object-return substitution **afterwards**; a *derived*
class initialises when `super()` establishes `this`, via **§7.3.33** `InitializeInstanceElements`.
Under the post-substitution reading, `class A { x = 1; constructor() { return {}; } }` wrongly adds
`x` to the returned object, a base constructor body cannot observe `this.x`, and a derived
constructor that returns an object without calling `super()` acquires fields it should not have.
**Initialise at base frame entry, or immediately after a successful `super()`, using the receiver
established there** — not the value `construct_synchronous` finally returns. The custom-element
concern below is real and unchanged; it constrains *which* receiver, not *when*.

**Slice 2ab custom-element upgrade receiver** — the measurement that folded the separate host-layer
row back into 2ab (this paragraph carried that row's contract until PR-B). The upgrade caller passes
one object in **both** receiver positions: `invoke_upgrade` calls
`ctx.vm.construct_synchronous(constructor, JsValue::Object(wrapper_id), &[], CallMode::Construct { new_target: constructor }, Some(wrapper_id))`
(`vm/host/custom_elements/upgrade.rs:315-323` at `658cc302`), so the receiver argument and the
pre-allocated instance are the same `wrapper_id`. The `HTMLElement` constructor's upgrade branch
hands back that same object rather than allocating one: it reads the construction-stack entry's
entity, looks the wrapper up with `ctx.host().get_cached_wrapper(entity)` (`html_element.rs:199`) and
returns `Ok(JsValue::Object(wrapper_id))` (`:219`). And `dispatch_super_inner` writes the constructed
object back onto the derived frame before anything the constructor body does can observe it —
`self.frames[frame_idx].this_value = result` and `.new_instance = Some(id)`
(`vm/dispatch_class.rs:150-154`). So the receiver 2ab initialises against, whether at base frame entry
or immediately after a successful `super()`, is already the wrapper the upgrade published; the only
way to reach a different object is an explicit `return otherObj`, which `upgrade.rs:350-358` already
rejects with a `NotSupportedError` (`matches!(value, JsValue::Object(id) if id != wrapper_id)`)
rather than initialising anything.
A host-layer slice would therefore decide the same receiver question 2ab decides, one layer down,
with nothing passing between them; the deliverable that remains is 2ab's no-regression test on the
upgrade path.

**Slot triggers this program fires**: `#11-compiler-class-emit-readability` and
`#11-reflect-apply-ce-test`. Both now have **§8 rows** — a trigger stated only here fires into a
paragraph nothing re-reads at re-eval time (Forward rule), and both slugs return **0** against the
live ledger, so neither is registered anywhere else either.

**1000-line touch-time check — the standalone prereq split is withdrawn (2026-07-27), on a narrower
ground than the first statement of this section claimed.** Rounds 2-9 all carried a mandated
"standalone prereq split branch" for `vm/dispatch.rs`.

⚠ **The 2026-07-27 reversal shipped after the §15 convergence call, ungated, and its measurements did
not survive re-derivation** — caught by this document's own plan-review at PR-B (`#506`). Its
originals were 1036/1113 (93%), 68 arms ≤8, 13 arms >20, a `run()`-residue enumeration that omitted
two top-level items, and — the figure the argument turned on — "29 arms using inline loop control
flow (16 `continue`, 13 `return`)", which summed two occurrence counts as though the sets were
disjoint.

**Every arm-classification figure has been removed rather than corrected.** Two independent passes
agreed exactly on every number taken straight off `wc -l` or `grep -c`, and disagreed on every number
requiring an arm-boundary convention (arm span, size buckets, control-flow arms) — the convention is
a choice, so those figures were argument dressed as measurement. What remains is re-derivable in one
command each:

| Measure | `f7d9b5ce` (rounds 2-9 basis) | `658cc302` (0a landed) |
|---|---|---|
| `wc -l vm/dispatch.rs` | 1112 | **1103** |
| `wc -l vm/dispatch_{class,helpers,ic,iter,objects}.rs` | 1725 across 5 files | **1888** |

*(An arm **count** is deliberately absent. `grep -c '^ *Op::.*=>'` returns 113 / 117 — it counts
pattern lines, and multi-pattern arms like `Op::IncElem | Op::DecElem =>` are one arm. Any figure
that reconciles the two encodes an arm-boundary convention, which is the class of number this
section got wrong. The dispatch match is order-100 arms; nothing below needs it sharper.)*

Two conclusions, and they are **not** the same conclusion:

1. **The match is not split.** CLAUDE.md's discipline is cohesion judgment, not line-count mechanics,
   and names the exemption: *「一枚岩の cohesive unit・巨大 generated table・**flat な case table** は
   対象外」*. A ~100-arm opcode dispatch table is that case, and that clause is the whole of the
   argument. *(The reversal also argued the match "cannot be split without a behavioural rewrite"
   because N arms `continue`/`return` the loop and read loop-local state. Removed: Slice 0a itself
   relocated `Op::IncElem | Op::DecElem` — an arm carrying two inline `continue`s — into
   `dispatch_helpers.rs::op_inc_dec_elem`, by returning `Result` and calling `throw_error` at the call
   site. No control-flow enum, no semantic rewrite. The premise was false.)*

2. **The file is NOT exempt from reduction, and the earlier text claimed it was.** "The match cannot
   be split without a behavioural rewrite" was read as covering the file; it does not follow, and per
   conclusion 1 it was not even true of the match. The `dispatch_*.rs` family is the in-tree seam that
   absorbs arm bodies, and Slice 0a moved the numbers in the right direction while implementing an
   unrelated feature: `dispatch_helpers.rs` 237→391, `dispatch_objects.rs` 429→438, `dispatch.rs`
   1112→1103.

**Decision: no standalone prereq-split PR — because the debt is discharged continuously, not because
the file cannot be reduced.** The forward rule for every slice that touches this file: extract the
*arm body* into the existing `dispatch_*.rs` family, never grow an arm in place — **choosing the
destination by I-5's predicate (the `engine` feature gate + host-binding dependency), not by the
file-family name.** Measured at `658cc302`: `dispatch_{class,helpers,ic}.rs` have zero
`feature = "engine"` occurrences; `dispatch_iter.rs` has 3 (`:114`, `:127`, `:153`) and
`dispatch_objects.rs` 2. ⚠ **The predicate is the *region*, not the file** — those three gates are
narrow bands **inside a 370-line file**, and everything outside them, including `iter_close` at
`:354` (which §6.2a-2 calls the canonical §7.4.11 implementation), is core-only. *(370 is the
**whole** `dispatch_iter.rs`, not a remainder: `git show <rev>:…/vm/dispatch_iter.rs | wc -l` gives
370 at `f7d9b5ce`, `658cc302` and HEAD alike. An earlier reading of this sentence as "the rest of
the file, 370 lines" double-counted.)* A file-level reading would forbid
Slices **1a** and **Pa** from the file both their module columns name, and Pa's whole deliverable is
re-signaturing that function. Read the predicate in **both directions**: do not place a core-only
body *inside* an engine-gated region, **and** do not extract an engine-bound arm body into a
currently core-only file by wrapping it in a fresh `#[cfg(feature = "engine")]`. That rule is the
discharge mechanism, so a slice that adds an arm and leaves the file larger has not complied.
(The withdrawn text also argued the reversal "takes a PR off the critical path and unblocks the T0 fix
immediately" — schedule is judgment-supporting information, not a design constraint (CLAUDE.md
*Ideal over pragmatic*), so it is removed rather than restated.)

**Files the watch list still owns** (`#11-d17b-dispatch-expr-file-growth`): `vm/interpreter.rs` 1366
(1a + Slice 3), `vm/value.rs` 1187 (Slice 3). **Not on any watch list and >1000**:
`vm/object_kind.rs` 1700 (Slices 6/7/10 — and see the cross-lane note in §8),
`vm/gc/collect.rs` 2074, `vm/gc/trace.rs` 1255 (Slice 7b). Each needs a cohesion verdict in its own
slice's memo — and this reversal is the precedent: **measure the file's shape before assuming a
split**.

Globals this program does not implement are registered in §8's table, like every other deferral.
`Intl`, `Atomics` and `WeakRef`/`FinalizationRegistry` moved there; this paragraph used to carry
their dispositions in prose, inside a section about `vm/dispatch.rs` line counts, which is not a
place a deferral can be retired from.

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
round 6 to "14". ⚠ **All four of those figures are withdrawn, and so is the flat "15" that followed
them** — §6.2a-3 shows why: they flattened three different transport mechanisms into one count, so a
remedy stated at one of them silently failed to reach the others. **§6.2a-3 is the
authoritative home** for what P sweeps (12 sites, partitioned 7 ECMA-262 / 5 WebIDL) and carries the
two commands that re-derive it. Five successive enumerations, each wrong; what this section is for
is the *method*, not the number. The raw greps:

```
grep -rn "iter_close("        crates/script/elidex-js/src/ | grep -v "fn iter_close"  # 10 call sites
grep -rn "Op::IteratorClose"  crates/script/elidex-js/src/compiler/                   #  5 hits → 4 emits
#   the 5th is a COMMENT: `stmt.rs:169` at `f7d9b5ce`, `stmt_loop.rs:134` at `658cc302` and at HEAD
grep -rn "takes precedence\|step 6-7" crates/script/elidex-js/src/                    # 17 (classify every hit)
```

### §6.2a-3 Slice P is partitioned by TRANSPORT first, then by governing algorithm — §5 mints Pa/Pb/Pc from it

⚠ **Added after Codex R1+R2 both raised findings against P's charter (the loop's ≥2-round
root-check).** The root was not either finding: it was that **"15 sites" flattened three different
transport mechanisms into one count**, so a remedy stated at one of them ("change `iter_close`'s
signature") silently failed to reach the others, and one site turned out to have two owners. The
existing ECMA-262-vs-WebIDL split is a *governing-algorithm* axis and is **orthogonal** to this one;
both are needed, and this one comes first because it decides what the deliverable even is.

| Transport class | Sites | Can a Rust signature change reach it? | P's deliverable there |
|---|---|---|---|
| **Rust callers** of `iter_close` | 10 | **Yes** — the completion kind is a value in scope at the call | completion-kind parameter + per-caller audit |
| **Bytecode emits** of `Op::IteratorClose` — *statement lowering* | 2 (`stmt_loop.rs:147`, `stmt.rs:661`) | **No** — the opcode is operandless (`bytecode/opcode.rs:284`) and is emitted from a throw handler *and* the non-throw Return path, so the handler cannot tell them apart | define the transport: an operand, or separate opcodes. **This is design work, not a sweep** — the opcode is operandless and both routes leave the same stack. Governed by **§9 dec. 2b**, which is open |
| **Bytecode emits** in `expr_yield_star.rs` (`:146`, `:157`) | 2 | **Not P's problem at all** | ⚠ **Reclassified out of P (Codex R3).** These are not completion-kind variants of §7.4.11: on an outer `.throw(e)` / `.return(v)` during delegation, **ECMA-262 §15.5.5 steps 8.b/8.c** invoke the *inner* iterator's corresponding method **with the injected value**, validate the result, and **continue delegating when `done` is false**. The compiler's own docstring says it reduces both protocols to a plain `IteratorClose`. No operand or split-opcode transport can carry an injected value or resume delegation, so this needs a **yield-delegation lowering**, tracked separately from P |
| **Inline re-implementation** in `op_array_spread` | 1 | n/a | **not P's** — dec. 13a assigns its removal to Slice 1a |

So the P family sweeps **12** — 10 Rust callers plus the 2 statement-lowering emits — of which only the 10 are
reachable by the signature change the charter used to describe as the whole remedy. The 2 `yield*`
sites leave P's scope entirely and the 1 inline site is 1a's. **Pb's plan-memo must settle the bytecode transport before it counts as
planned**, and must test the throw and Return paths separately — a single fixed argument in the
handler gets ECMA-262 §7.4.11 step 5 right and steps 6-7 wrong, or the reverse.

⚠ **This section is the authoritative home for the site partition** (it moved here when §5 split the
Slice-P row into Pa/Pb/Pc; the sub-slice cells name owners, not counts). Derived, not recalled, by
these two commands — re-run them rather than reading the figures forward:

```
git grep -n "iter_close(" 658cc302 -- crates/script/elidex-js/src | grep -v "fn iter_close"   # 10 callers
git grep -n "emit(Op::IteratorClose)" 658cc302 -- crates/script/elidex-js/src/compiler        #  4 emits
```

Of the 10 callers, 5 are ECMA-262-governed (`dispatch_iter.rs:309`, `:337`, `natives_array_hof.rs:485`,
`ops.rs:60`, `host/typed_array_static.rs:798`) and 5 WebIDL-governed (`webidl_sequence.rs:141`, `:149`,
`host/url_search_params.rs:318`, `host/headers/parse_init.rs:208`, `host/structured_clone.rs:1064`); of
the 4 emits, the 2 statement-lowering ones (`stmt.rs:661`, `stmt_loop.rs:147`) are ECMA-262-governed —
giving **7 / 5**. *(The earlier "10 ECMA / 5 WebIDL" was residue of the withdrawn flat 15 and was a
**subset written as a partition** — the 5 WebIDL sites are 5 of the 10 callers, not a disjoint second
group.)* The 7 are Pa's and Pb's, the 5 are Pc's.

*Method note, recorded because this program keeps paying for it: the flat count read as rigour —
it was mechanically derived and it was correct — while hiding that its members did not share a
remedy. **A count is only a plan when every member admits the same fix.***

**10 `iter_close(` call sites** (verified 2026-07-26): `vm/dispatch_iter.rs:309`, `:337` ·
`vm/natives_array_hof.rs:485` · `vm/webidl_sequence.rs:141`, `:149` · `vm/ops.rs:60` ·
`vm/host/url_search_params.rs:315` · `vm/host/structured_clone.rs:1062` ·
`vm/host/typed_array_static.rs:798` · `vm/host/headers/parse_init.rs:207`.
⚠ Three of the `vm/host/` offsets drifted under 0a: at `658cc302` they are `url_search_params.rs:318`,
`structured_clone.rs:1064`, `headers/parse_init.rs:208` (`typed_array_static.rs:798` unmoved). The
**count of 10 still holds at HEAD** — re-run the grep above rather than reading either set of offsets.

**4 compiler `Op::IteratorClose` emit sites.** The count has held, the anchors have not — Slice 0a
split `compiler/stmt.rs` (964→712) into it plus the new `compiler/stmt_loop.rs`, which took the
`for-of` handler with it. **Re-run the grep above at implementation time; do not read either column
forward.**

| | `f7d9b5ce` (as first enumerated) | `658cc302` |
|---|---|---|
| the `for-of` catch handler — **the most reachable IteratorClose path in the language** | `compiler/stmt.rs:175` | **`compiler/stmt_loop.rs:147`** (comment at `:134`) |
| the other statement-level emit | `stmt.rs:913` (out of range at HEAD) | `compiler/stmt.rs:661` |
| the `yield*` **throw** route | `compiler/expr_yield_star.rs:146` | `:146` |
| the finally route | `:157` | `:157` |

⚠ **`compiler/stmt_loop.rs` is therefore in Slice Pb's touch set** — the file did not exist when §5's
Slice-P module column was written. That gap is recorded once, in §5's **evidence block**; the module
column is a non-authoritative hint and this sentence must not re-assert the authority §5's demotion
removed by claiming the column "has been updated".

**SoT**: `iter_close` (`vm/dispatch_iter.rs:354`, docstring `:340-353`) — the canonical §7.4.11 implementation, whose
*docstring states the inverted rule as its contract*: "if `.return()` itself throws, having that new
throw take precedence over the triggering abrupt completion". Exactly four sites additionally cite
"§7.4.11 step 6-7" in support of the inverted rule.

**⚠ A further site the two greps structurally cannot see** (round 7): `op_array_spread`
(`vm/dispatch_iter.rs:53-63`) **re-implements IteratorClose inline** — it looks up
`well_known.return_str` and calls the method directly, never calling `iter_close`. Verified:
`grep -rn "return_str" crates/script/elidex-js/src/vm/` returns exactly **2** implementation sites
(`:57` inline, `:359` inside `iter_close`), so there is exactly one such duplicate. Consequences:
(a) fixing only `op_array_spread` does not advance P's sweep at all — the site is not in either
grep, and §6.2a-3 excludes it by name; (b) the P family
is sequenced before 0bc and hence before 1a, so this site keeps the
inverted convention until dec. 13a lands — and if 13a were rescoped the site stays inverted
invisibly. **The concept grep *was* run** (`takes precedence\|step 6-7` → 17 hits) **but its hits
were dismissed as "incl. unrelated" and never classified — which is exactly how this one dropped
out.** Rule: classify every hit; never filter by expectation.

**⚠ 5 of the sites, across 4 files, are not governed by ECMA-262 at all** (round 7, corrected here):
`vm/webidl_sequence.rs:141`, `:149`, `vm/host/url_search_params.rs:318`,
`vm/host/headers/parse_init.rs:208` **and `vm/host/structured_clone.rs:1064`** (offsets at
`658cc302`; the fifth was moved into this half by §4 I-5's own Codex-R1 correction —
`ensure_empty_transfer_list` is the WebIDL conversion, not HTML's loop — and the count here had not
followed) all implement
**WebIDL §3.2.21.1 "Creating a sequence from an iterable"**, which has **zero** `IteratorClose`
steps (verified `body webidl create-sequence-from-iterable | grep -ci iteratorclose` → **0**;
control `body ecma262 sec-array.from` → 1, so the grep discriminates). For those sites the question
is **not** precedence but whether `return()` should be called *at all* — the same prior question
§2.5 C×F and §6.2a already answered ("remove it") for [C19]/[C22], never propagated to this list.
**§6.2a-3 carries that split (7 ECMA-262 / 5 WebIDL) with its derivation**, and I-5's
"pure ECMA-262 language surface" label is wrong for all four of these files (their shared helper
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

⚠ **But the converse over-reads it (Codex R5): "always close" is equally wrong.** ECMA-262
**§13.15.5.2** closes only while the iterator's `[[Done]]` is still **false**, and a rest element
drains through `IteratorStepValue` *until* `[[Done]]` becomes true — so mandating a dedicated
`iter_close` for `[...rest] = it` makes a custom iterator observe `.return()` after **normal
exhaustion**, a user-visible divergence. The obligation is a **conditional** close around early
termination and abrupt pattern evaluation. Slice 0bc must pin both directions: `[a] = it` closes an
unfinished iterator, `[...r] = it` does **not** close an exhausted one.

**Why this lands on the critical path**: §8 declares the 0b family "owns the `IteratorClose` obligation
1a/1b do not". Slice 0bc will call `iter_close` and thereby **inherit the inverted contract**,
making its [C39]→[C36] conformance claim false. And because the sites span `compiler/`, core `vm/`
and `vm/host/`, fixing only `op_array_spread` leaves every one of P's sites inverted — including the
`for-of` catch handler (`stmt_loop.rs:147` at `658cc302`), which is the path most user code actually
hits.

**This is the same failure mode twice**: round 3 caught me propagating the IteratorClose *mandate* to
four sites but not the fifth; round 4 caught the *precedence* concept having its own un-swept
siblings. Both are [[feedback_semantic-sibling-selfseed-and-regate-breadth]] — the lesson is to grep
the **concept**, and a concept discovered mid-paragraph needs its own sweep, not an inherited scope.

⚠ **After 1a, `op_array_spread` is [C19]/[C22]-only.** §5 sequences **the 0b family before 1a**, and 0bc owns
[C39], whose rest form (`[a, ...rest] = it`) needs a drain-into-array and whose spec *requires*
`IteratorClose` — while `op_array_spread`/`spread_iter_loop` is the only in-tree drain-into-array.
**0bc must therefore give its rest path an explicit `iter_close` site of its own**, not reuse this
drain, and §7.2 must pin it. (Executing dec. 13a's propagation instruction here, in §6.2a, where a
0bc implementer reads it.)

**Decision** (§9 decision 13, restated): the drain fix is **1a's** (it is the reuse precondition),
but the **precedence sweep is its own unit** — a cross-cutting site set (§6.2a-3 has the
figure and its derivation), a signature change on the shared helper, and
a completion-kind distinction. Carve `#11-vm-iteratorclose-precedence-convention` and sequence it
**before Slice 0bc** (whose conformance claim depends on it), not inside 1a.

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

1a's acceptance test is *"the full existing suite passes unchanged"* **plus edges 21/27/28/32 as
required tests** — §15 restated it and this sentence had not been propagated. The bare form is
vacuous for 1a's two *semantic* fixes (`op_array_spread`'s `return()` removal, dec. 13a; the arg-window
rooting, dec. 10): the existing suite covers neither, so an implementation could omit both and still
pass. 1b's is §6.4.

**Compiler** — one helper (I-3), replacing `compile_arguments`:

```rust
/// [C19] ECMA-262 §13.3.8.1 ArgumentListEvaluation.
pub(super) enum ArgsForm { Flat(u8), Array }
fn compile_call_arguments(…) -> Result<ArgsForm, CompileError>  // (NEW)
```

**The I-3 tagged-template input contract is a Slice-1b deliverable, not a note.** The signature above
is the whole specification today, and `ArgsForm { Flat(u8), Array }` cannot express GetTemplateObject's
`« siteObj »` prefix followed by the substitutions — §3's TemplateLiteral rows state that constraint
and hand it to Slice 4b. §11 assigned the question to 1b, but an assignment in a round record owns
nothing, so as written 1b can pass its charter and its tests and still leave Slice 4b to either change
this helper or emit its arguments some other way. That second mechanism is exactly what I-3 exists to
prevent. **So 1b must define the helper's input contract — a caller-supplied fixed prefix plus an item
sequence — and test it.** Whether the prefix is a count or an item iterator is 1b's own plan-review to
settle; that the contract exists, is documented at the signature and is exercised by a test is not.

**Acceptance condition Slice 4b consumes**: a 1b test drives `compile_call_arguments` with a
non-empty fixed prefix plus *n* substitutions and asserts the emitted layout and the returned
`ArgsForm` for *n* = 0 and *n* > 0, so Slice 4b reaches its tagged-template lowering by *calling* this
helper — no signature change, no second argument-emission path. Slice 4b's mandatory plan-review
checks that it did.

`compile_arguments` has **5 call sites** (verified 2026-07-26 via
`grep -rn 'compile_arguments(' crates/script/elidex-js/src/`: `expr_member.rs` lines 102/131/137/176
+ `expr.rs` line 90; line 53 is the definition), feeding the user-facing emit sites tabled below.
*(No row count is stated: `expr_member.rs:105` and `:186` each serve two call shapes, so any total
encodes a per-line-vs-per-shape convention — the class of figure this document has repeatedly got
wrong. Count rows if you need one.)*

| Emit site | Shape | Flat op | Spread op |
|---|---|---|---|
| `expr_member.rs:105` | method call `o.m(…)` | `CallMethod` | **`CallMethodSpread` (NEW)** |
| `expr_member.rs:105` | **`super.m(…)`** — `Member{object: Super}` takes this branch | `CallMethod` | `CallMethodSpread` |
| `expr_member.rs:129/133` | `super(…)` | `SuperCall` | `SuperCallSpread` — fold onto helper |
| `expr_member.rs:140` | plain call | `Call` | `CallSpread` (exists, stub) |
| `expr_member.rs:184` | optional method | `CallMethod` | `CallMethodSpread` |
| `expr_member.rs:186` | optional call, **non-Reference callee** (`f?.(…)`) | `Call` | `CallSpread` |
| `expr_member.rs:186` | optional call, **member-Reference callee** (`o.m?.(…)`, `o?.m?.(…)`) — the `else` branch is taken because the `Member` base is outside `chain`, so `prev_is_member` is false and `this` is already lost at `658cc302` (§3 ChainEvaluation step 3) | **`CallMethod`** (receiver-preserving; today wrongly `Call`) | **`CallMethodSpread`** |
| `expr.rs:92` | `new` | `New` | `NewSpread` (exists, stub) |

**`super.m(...)` cross-slice contract**: `compile_call_expr` matches `ExprKind::Member` **first**
(`expr_member.rs:92`), so `super.m(...)` is a method call whose receiver comes from
`compile_expr(ExprKind::Super)` → `PushUndefined` (`expr.rs:200-207`, Slice 3's stub). Slice 1
therefore **does** change this shape's argument emission while its receiver stays broken. Slice 1
must not claim to fix it and must not make it *differently* broken; §6.4 edge 23 is the
no-regression guard, and ⚠ **Slice 3 must NOT simply land `GetSuperProp` on the generic `CallMethodSpread` path (Codex R3)** — ECMA-262 **§13.3.7.3** MakeSuperPropertyReference step 4 takes the Reference's `[[Base]]` from **§9.1.1.3.5** `GetSuperBase(envRecord)`, which returns `home.[[GetPrototypeOf]]()` — the **`[[HomeObject]]`'s prototype, resolved at call time**, not the lexically named superclass — while step 5 keeps the current **`this`** (`actualThis`) as `[[ThisValue]]`, which §13.3.6.2 EvaluateCall then passes as the receiver. *(Both §-numbers were already right; the gloss said "superclass" and that is what was wrong.)* The generic member path emits `compile_expr(object); Dup; property-load`, while `Op::GetSuperProp` is declared `[-- value]`, so following the generic contract leaves a stray operand or passes the super base as `this`. Slice 3 needs a **super-specific `PushThis; GetSuperProp` / `GetSuperElem` lowering**, covering named *and* computed spread calls; its own mandatory plan-review settles the stack shape. Superseded framing: Slice 3 lands `GetSuperProp` on top of the `CallMethodSpread` emit Slice 1
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
`dispatch.rs:1074` — the `:1083` cited here until PR-B is wrong at `658cc302` and at HEAD — with no
emit site, and Slice D deletes it; `Call`/`CallMethod` are `emit_u8_u16`, `New`/`SuperCall`
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
⚠ **As spelled, those edges cannot fail (Codex R3).** `async function a(){}` declares no parameters
and never touches `arguments`, so `push_js_call_frame` builds no `actual_args` and truncates the
passed values before the `stack_slice` is taken — the test passes with all four rooting fixes
omitted, even under forced collection. The generator case likewise never resumes, so it never
observes its saved arguments. **Both edges must retain object-valued arguments in both vectors and
verify them after `.next()` or after the Promise settles**; a rooting edge that cannot fail is not a
guard, and 1a's acceptance leans on exactly these.
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
must bind `this` to `o` (§9 dec. 14) · **32. [1a] a synthesized default derived constructor must NOT
observably call `%Array.prototype%[Symbol.iterator]`** (ECMA-262 §15.7.14 step 14.a.iv.1 NOTE — the
I-3 carve-out's regression guard). ⚠ **Merely *defining* `class B extends A {}` never executes its
synthesized constructor, so an edge that only evaluates that spelling passes even when construction
wrongly routes through `ArraySpread`.** The edge is therefore: override
`%Array.prototype%[Symbol.iterator]` with an observable stand-in, **execute `new B(1, 2)`**, and
assert the override was **not** called *and* that the arguments still reached `A`'s constructor —
**plus a positive control in the same edge**, the hand-written spelling
`class C extends A { constructor(...args) { super(...args) } }`, where `new C(1, 2)` **does**
observably call it. Without the control the assertion is satisfiable by an implementation that never
calls the iterator at all, which would not distinguish the carve-out from a broken spread path. ·
**33. `o.m?.(...a)` — optional call on a member-Reference callee**: `this` must be `o`
([C38] ChainEvaluation step 3 → [C20] EvaluateCall step 1.a.i). It is `undefined` at `658cc302`
because `compile_optional_chain_expr`'s `prev_is_member` test is false when the `Member` sits in the
chain's *base* rather than inside `chain`, so `expr_member.rs:186` emits `Op::Call` (verified at
`f7d9b5ce` and `658cc302` — the branch is byte-identical at both). Assert the receiver **and** the
spread arity; a receiver-only assertion passes on the flat path. · **34. `o?.m?.(...a)`** — the same
with the base itself optional: nullish `o` short-circuits without evaluating or draining the operand
(edge 12's rule), non-nullish `o` binds `this === o`. · **35. `o.m?.()` — the no-spread spelling of
edge 33**: assert `this === o`. Edges 33/34 both carry `...a`, so a fix that selects a spread call
form for the optional-member callee satisfies them while the flat spelling keeps emitting `Op::Call`
at `expr_member.rs:186` (`658cc302`) and still passes `undefined` as the receiver.

**Slice allocation of this matrix** (added R2 round 5): edges **21** (`return()` not called), **27/28**
(GC window) and **32** belong to **1a** — they are the assertions for its two semantic fixes (decs.
13a and 10) and its docstring carve-out. Edge **35** belongs to **0bb**, not to 1a or 1b: it has no
spread and no call-shape change, and 0bb owns the optional-member receiver contract (§2.2, §5).
Everything else is **1b**'s. Round 5 flagged that calling 1a
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
the fixing slice — the `vm/tests/tests_dataset.rs:283` pattern. **Deliverable of Slice 0cb**, so
every later slice inherits the baseline.

⚠ **The row shape needs a crash-aware outcome (Codex R2) — ratified as §9 decision 16.**
"One row per §2.2 row" includes
`f(a×256)`, whose defect is a **process abort** — `assert!(arguments.len() <= 255, …)` in
`compiler/expr_member.rs` — and is owned by Slice **1b**, which lands after 0cb. No `(source,
expected)` *value* comparison can be written for an input that panics the compiler, so 0cb's
permanent suite cannot pass as specified. **Resolution: the outcome type expresses
`Panics`/`Throws` alongside a value** (§9 dec. 16). The alternative — sequencing the arity fix ahead
of the table — is excluded by this document's own ordering, not by preference: the fix is 1b's, 1b
depends on 1a (§5 Deps), and 1a depends on 0cb, so moving it ahead of the table inverts two declared
dependencies and §15's ship order.

⚠ **The row-derivation rule needs a second source** (round 8). §9 dec. 9(b) sets "one row per §2.2
defect row", but §2.2 is **Layer-A compiler-emit only** — so the rule structurally produces *no* row
for the T3 absence surface (`Map`/`Set`/`WeakMap`/`WeakSet`, `Proxy`/`Reflect`, the `RegExp` ctor,
`Array.prototype.at`/`findLast`/`Object.hasOwn`/`String.matchAll`), which is roughly half of what
§1.1/§1.2 found. As written, 0cb ships the program's safety net with a hole exactly where the probe
found most gaps. Second source: one row per **§1.1/§1.2 absence** finding + per Slice 7-10 slot. Note the I-1
discharge reduces the number of divergence rows up front by converting silent stubs to loud throws.

⚠ **A third source, for the same reason on a different axis**: §2.2's derivation is scoped to what
its three passes read (`compiler/` arms, their marker comments, and productions that have an AST
variant — see that section's scope statement), so "one row per §2.2 defect row" inherits that scope.
Rows for what lies outside it arrive the way §2.2's own non-`compiler/` rows did — from the slice
that owns the code, derived against its own parent HEAD, at the point that slice lands. 0cb
therefore ships the table it can derive, and later slices add rows; the table is not asserted
complete at 0cb.

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
  bodies / computed-name methods / static blocks / `Op::GetSuperProp` + `Op::SetSuperProp`".
  **Already in the SoT ledger** (adopted at #489's landing; the earlier claim
  that it lived only in `m4-12-pr-d17b-html-element-constructor-base-vm-landing.md` is superseded).
  Partially discharged already (computed method keys verified working, §1.2). ⚠ **Static
  fields are NOT among the discharged** — narrowed at PR-B to their *definition*; the initializer's
  receiver is wrong (§2.2's `expr_class.rs:402-427` row), owned by Slice **2ac**, and the probe that
  established the discharge (`static x = 1`) could not see a receiver. **The claim that this scope is "exactly Slices 2/3/5" is deleted, not amended.** Those three
  rows own class instance fields, super property references and private names; the scope string also
  names **class static blocks**, which none of them covers, and reading the scope as exhausted by the
  slice list is what hid the following. Measured at `658cc302` and re-derived at HEAD `d49465d0`
  (`git grep -n 'FunctionCompiler::new' <rev> -- crates/script/elidex-js/src/compiler/`; both revs
  give the same line numbers): scope analysis allocates a static block its **own function boundary** —
  `scope/visitor.rs:555` pushes `ScopeKind::StaticBlock`, and `compiler/resolve.rs:433` is
  `matches!(kind, ScopeKind::Function | ScopeKind::StaticBlock)` — while `expr_class.rs:457` builds
  the block's child compiler as `FunctionCompiler::new(fc.func_scope_idx, fc.current_scope_idx, true)`
  and `:464` finishes it against `func_scopes[fc.func_scope_idx]`: the **enclosing** function scope
  both times. Contrast `expr_function.rs:111` / `:243`, which pass the child's own `child_func_idx`.
  So a static block's block-local `let`/`var` bindings compile against the wrong scope. ECMA-262
  **§15.7.11** ClassStaticBlockDefinitionEvaluation step 5 creates the body with
  `OrdinaryFunctionCreate(%Function.prototype%, sourceText, formalParams, ClassStaticBlockBody,`
  `non-lexical-this, lexicalEnv, privateEnv)` — its own function environment (§-number from
  `webref aoid ecma262 ClassStaticBlockDefinitionEvaluation`, step text from
  `webref body ecma262 ClassStaticBlockDefinitionEvaluation`). **Generic static-block lowering is
  Slice 2b's**: the Slice 2 family already owns class-body emit in `compiler/expr_class.rs` and the construction
  path in `vm/dispatch_class.rs`, and no other slice touches the static-block arm. This slot cannot
  retire until Slice 2b tests **block-local binding isolation** (a `let` in a static block does not
  resolve to an enclosing binding of the same name) **and capture** (a closure created inside the
  block still reads the block's own bindings after it returns). **The 2a family, 2b, 3 and 5 retag every in-code citation `grep -rn 'step9-class-extras' crates/` reports
  at retag time** — 6 at `658cc302` (`compiler/expr.rs`, `compiler/expr_assign.rs`,
  `compiler/stmt_loop.rs`, `vm/interpreter.rs`, `vm/value.rs`, `bytecode/compiled.rs`), where this
  section originally froze a 4-site list measured 2026-07-26; 0a added two. Retagging a frozen list
  is what leaves the dangle the sentence exists to prevent.
- `#11-dead-opcode-removal` — "`Op::CreateClass` verifiably dead; bundle with D-26 Op-enum
  re-baseline"; trigger **already fired** at #458. Becomes Slice D, broadened to the §2.3 set.
  **Still absent from the SoT ledger** (0 hits, present only in the `m4-12-pr-d17b-*` landing memos),
  as is `#11-d17b-dispatch-expr-file-growth` below — so both adoptions are genuinely outstanding,
  unlike `#11-step9-class-extras`.

**Also adopt** `#11-d17b-dispatch-expr-file-growth` (uncounted watch slot, D-17b r1/r2 landings;
homes the `vm/dispatch.rs` + `compiler/expr_class.rs` + `vm/interpreter.rs` + `vm/value.rs`
1000-line debt). ⚠ **The `dispatch.rs` facet is NOT discharged, and it is not a
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
the 0b family, the 0c family, 1a or 1b.

**Forward rule.** Every surface this program identifies and does not implement gets a **row in the
table below with a slot id** — a deferral stated only in prose elsewhere in this document is
structurally unretirable, because (as this section already says) landing this document registers
nothing and the ledger is the SoT, so nothing outside this table is ever re-read at re-eval time.

**Slots this umbrella owns** — the table below, each with the required triple.

⚠ **The frozen registration partition that stood here ("12 rows = 8 already in the ledger + 3 still to
register + 1 to retire", measured at `658cc302`) is withdrawn.** It was already wrong before this
revision added rows: `#11-vm-yield-delegation-lowering` **is** in the ledger and the list of eight
omitted it. A hand count of this table goes stale every time the table grows — the same failure this
section's opening ⚠ warns about. **Derive it instead**, one command per row against the live ledger
(the memory-dir `project_open-defer-slots.md`, outside this repo):
`grep -c -- '#11-<slug>' <ledger>`, run over **every `#11-*` slug in the table — not only the
`#11-vm-*` ones**. ⚠ **The recipe used to sweep `#11-vm-*` alone**, which made every adopted
non-`vm` slot (`#11-step9-class-extras`, `#11-dead-opcode-removal`,
`#11-d17b-dispatch-expr-file-growth`, `#11-compiler-class-emit-readability`,
`#11-reflect-apply-ce-test`, `#11-webidl-sequence-dense-array-fast-path`) invisible to it **in
principle**, however many rows the table gained. Widened here.

⚠ **The derivation is anchored to the ledger, not to a repo commit** — the ledger lives outside this
repo, so a repo SHA cannot date its contents. Run against `project_open-defer-slots.md` as of
**2026-08-08** (`stat -f %Sm <ledger>`), it returns **nonzero** for
`#11-vm-iteratorclose-precedence-convention`,
`#11-vm-assignment-target-completeness`, `#11-vm-topropertykey-symbol-from-toprimitive`,
`#11-vm-operand-rooting-by-construction`, `#11-vm-internal-error-hard-exit`,
`#11-vm-delete-elem-raw-key-array-fast-path`, `#11-vm-statement-completion-updateempty`,
`#11-vm-typed-array-family-layering-and-gate`, `#11-vm-yield-delegation-lowering`,
`#11-vm-function-constructor-global`, `#11-step9-class-extras` and
`#11-webidl-sequence-dense-array-fast-path`; and **zero** for
`#11-vm-async-generators`, `#11-vm-builtin-prototype-static-sweep`, `#11-vm-dynamic-import`,
`#11-vm-property-key-lowering-unification`, `#11-vm-atomics-global`,
`#11-vm-weakref-finalization-registry`, `#11-vm-module-opcode-connects`,
`#11-vm-webidl-section-3-10-retag`, `#11-dead-opcode-removal`,
`#11-d17b-dispatch-expr-file-growth`, `#11-compiler-class-emit-readability` and
`#11-reflect-apply-ce-test` — those are what is still to register. The slots later revisions add also
return **zero** against the same ledger (`stat -f %Sm` = 2026-08-08 11:51) and join that set:
`#11-vm-block-declaration-instantiation` and `#11-vm-explicit-resource-management`, plus
`#11-vm-labelled-statement-break-target`, `#11-vm-per-iteration-loop-environment`,
`#11-vm-for-in-enumeration-semantics` and `#11-vm-arrow-lexical-super`
(`grep -c -- '<slug>' <ledger>` → **0** for each of the last
four, run 2026-08-09), and `#11-vm-parameter-environment-instantiation` and
`#11-vm-instanceof-callable-check` (same command, same ledger mtime, → **0** for both; control, run
in the same invocation, `#11-step9-class-extras` → **1** and
`#11-vm-typed-array-family-layering-and-gate` → **1**, so the pattern discriminates rather than
returning zero for everything), and `#11-vm-const-assignment-runtime-throw`,
`#11-vm-upvalue-tdz-check` and `#11-vm-named-function-expression-binding` (same command, same ledger
mtime, → **0** for each; the same two controls, run in the same invocation, → **1** each), and
`#11-vm-derived-constructor-this-binding`, `#11-vm-ordinary-function-name-length` and
`#11-vm-object-literal-proto-setter` (same command, same ledger mtime, → **0** for each; the same two
controls, run in the same invocation, → **1** each), and `#11-vm-private-name-early-errors` (same
command, same ledger mtime `stat -f %Sm` = 2026-08-08 11:51, → **0**; the same two controls, run in
the same invocation, → **1** each), and `#11-vm-webidl-sequence-early-exit-drain` and
`#11-vm-class-name-binding-initialization-order` (same command, same ledger mtime, → **0** for both;
the same two controls, run in the same invocation, → **1** each).
`#11-vm-computed-compound-assignment`
also returns zero, but it is the
to-retire row below, not a registration gap; and the `Intl` row mints no slot at all, since it points
at an externally owned standing decision. Re-run rather than read forward.

**Ledger reconciliation owed at landing, not by this PR** (the ledger is outside this repo):
`project_open-defer-slots.md:217` lists `Intl` / `Atomics` / `WeakRef`+`FinalizationRegistry` /
`String.matchAll` under "Deliberately NOT slotted", and its registered trigger for
`#11-vm-function-constructor-global` ("a dynamic-`Function` need, OR the LegacySemantics/core-strict
compat work") is not the one this document states — so `#11-vm-function-constructor-global`,
`#11-vm-atomics-global`, `#11-vm-weakref-finalization-registry` and the `Intl` row are reconciled on
the ledger side at landing, and the divergence must not be closed by quietly overwriting the
registered text.

**To retire, not register**:
`computed-compound-assignment` — its stated purpose was "a ledger home until it lands", and it landed
as `658cc302` while never reaching the ledger, so the row below is the only place it has ever existed.

| Slot | Why deferred | Trigger | Re-eval |
|---|---|---|---|
| `#11-vm-iteratorclose-precedence-convention` | **(carved R2 round 4)** the §7.4.11 error-precedence inversion spans `compiler/`, core `vm/` and `vm/host/`, and the correct behaviour is completion-kind-dependent ⇒ `iter_close`'s signature must change. A cross-cutting convention sweep, not a slice deliverable. ⚠ **The site count and its ECMA-262/WebIDL partition are NOT restated here** — §6.2a-3 is their single home and carries the two commands that re-derive them; five successive figures in this row's lineage (14 lines → 5 → ~15 → 14 → 15) were each withdrawn, and the last of them survived in this row after §6.2a-3 had been corrected | **now** — Pb gates Slice 0bc, whose [C39]→[C36] conformance claim inherits the inverted contract. **Pa blocked by `#11-vm-typed-array-family-layering-and-gate`** (§5 Deps) | 2026-09-30 |
| `#11-vm-computed-compound-assignment` | Slice 0a work item, not a defer — registered so the T0 crash has a ledger home until it lands | now (Slice 0a) | 2026-09-30 |
| `#11-vm-assignment-target-completeness` | The 0b family's work item; distinct SDO ([C39]) + owns the `IteratorClose` obligation Slice 1 does not, and the Paren-normalisation fix shares the same catch-all arms | now (Slices 0ba/0bb/0bc) | 2026-09-30 |
| `#11-vm-object-spread-source-coercion` | **(carved at PR-B)** Slice **O**. `op_spread_object` copies only between two `JsValue::Object`s, so `{...'ab'}` and `const {...r} = 'ab'` give empty objects where ECMA-262 §7.3.25 applies `ToObject` to every non-nullish source. Carved rather than folded into an existing slice because §2.2 derives from `compiler/` and §2.3's membership test is zero emit sites, so a live defect in a **connected** dispatch handler had no owner at all | now (Slice O) | 2026-09-30 |
| `#11-vm-arrow-lexical-new-target` | **(carved at PR-B)** Slice **N**. `Op::NewTarget` reads the current frame's `CallMode` and `FunctionObject` has no lexical `new.target` beside `captured_this`, so `new.target` is `undefined` inside an arrow where ECMA-262 §9.4.5 resolves the surrounding function environment. Separate from Slice 3 because the AO differs, though both add lexical state to the same closure/frame seam | now (Slice N) | 2026-09-30 |
| `#11-vm-arrow-lexical-super` | **(carved at PR-B)** Slice **S**. The parser deliberately lets an arrow inherit `super` context (`parser/arrow.rs:104-119` at `658cc302`), but `FunctionObject` carries no home-object field beside `captured_this` (`vm/value.rs:588-599`) and `dispatch_super_inner` (`vm/dispatch_class.rs:115-119`) resolves against the **current** frame's `home_class`, which is `Some` only on a class-ctor frame (`vm/interpreter.rs:802`) and hard-coded `None` on the `call_internal` entry (`:589`) — so `super` reached through an arrow raises "'super' keyword unexpected here". ECMA-262 **§13.3.7.2** GetSuperConstructor step 1 and **§13.3.7.3** MakeSuperPropertyReference step 1 both begin at **§9.4.3** GetThisEnvironment, whose outward walk skips an arrow's environment. Carved rather than folded into Slice 3, whose charter is super-*property* emit and dispatch: adding the super-*call* clause and the environment walk gives that row a second governing algorithm across a further layer, which §5's criterion forbids. Same closure/frame seam as **N** and **3**, different AO | now (Slice S, after Slice 3) | 2026-09-30 |
| `#11-vm-block-declaration-instantiation` | **(carved at PR-B)** Slice **B**. elidex has no ECMA-262 **§14.2.3** BlockDeclarationInstantiation. The slot's inventory is **derived, not listed** — §5's Slice B row states the derivation, and it is re-run at the slice's own parent HEAD; §5's evidence block records what they produced at `658cc302`, and the rows are §2.2's. Probe-measured consequences already in hand: `let x=1; switch(0){case 0: let x=2;} x` → `2` (spec `1`), `let x=1; try {} finally { let x=2 } x` → `2` (spec `1`), and `{ f(); function f(){} }` inside a function → `TypeError: not a function`. Carved as its own slice rather than folded in because no existing row's charter is lexical-environment entry, and adding a second governing algorithm to 0bc (binding-pattern storage) or 2b (the static-block function boundary) makes either non-terminal under §5's criterion. Everything the derivation returns shares the AO, so the deliverable is one block-entry instantiation, not a patch per construct (CLAUDE.md *One issue, one way*) | now (Slice B) | 2026-09-30 |
| `#11-vm-labelled-statement-break-target` | **(carved at PR-B)** Slice **L**. A label is recorded as an index into `fc.loop_stack` (`compiler/stmt.rs:287-288` at `658cc302`), which has no entry for a statement that is neither a loop nor a switch, so `outer: { break outer; }` is rejected at compile time by the `Break` arm's guard at `:244-249` and a labelled block wrapping a loop patches the loop instead (`:251`). ECMA-262 **§14.13.4** LabelledEvaluation step 4 of `LabelledStatement : LabelIdentifier : LabelledItem` catches a break completion whose `[[Target]]` is the label at the labelled statement itself, so a labelled statement is a break target regardless of whether it is a `BreakableStatement`. Deliverable is the representation — a label→patch-list that does not presuppose a loop — plus the `continue`-on-a-non-loop-label early error (**§14.8.1**). Terminal under §5's criterion: one contract, one SDO, one layer | now (Slice L) | 2026-09-30 |
| `#11-vm-per-iteration-loop-environment` | **(carved at PR-B)** Slice **E**, an umbrella with sub-slices **Ea** (`for` head) and **Eb** (`for-in`/`for-of` heads). `scope/visitor.rs` pushes one `ScopeKind::Block` for the whole loop (`:85` for `for`, `:114` for `for-in`/`for-of` at `658cc302`) and the compiler stores the head binding into that one scope's local, so every closure created in the body shares one binding; `git grep -c CreatePerIterationEnvironment 658cc302 -- crates/script/elidex-js/src` exits **1** (no match). ECMA-262 **§14.7.4.4** CreatePerIterationEnvironment (from **§14.7.4.2** ForLoopEvaluation) copies the previous iteration's value into a fresh environment each iteration; **§14.7.5.7** ForIn/OfBodyEvaluation step 8.h.iii instantiates a fresh environment from the iteration value with no copy-forward. Two governing algorithms across scope analysis, local allocation and closure capture ⇒ umbrella under §5's criterion, split by head form because that is where the algorithms differ; **plan-review MANDATORY** per the CLAUDE.md edge-dense rule. Carved rather than folded into **B**, whose contract is block *entry*, or into 0bc, whose contract is binding-pattern storage | now (Slices Ea/Eb) | 2026-09-30 |
| `#11-vm-for-in-enumeration-semantics` | **(carved at PR-B)** `op_for_in_iterator` (`vm/dispatch_iter.rs:141` at `658cc302`) snapshots the key list once into `ForInState { keys, index: 0 }` (`:231-236`) and records a name in `seen` only inside `if attrs.enumerable && seen.insert(sid)` (`:188`, `:220`), and `op_for_in_next` (`:243`) replays that snapshot with no re-check. ECMA-262 **§14.7.5.9** EnumerateObjectProperties requires the opposite on both counts — "The values of [[Enumerable]] attributes are not considered when determining if a property of a prototype object has already been processed", so a non-enumerable own key must shadow an inherited enumerable one of the same name; and "A property that is deleted before it is processed by the iterator's next method is ignored", so existence must be re-checked as iteration advances. Deliverable is the enumeration state machine (**§14.7.5.10.1** CreateForInIterator and **§14.7.5.10.2.1** `%ForInIteratorPrototype%.next`, whose incremental shape is what the spec's carve-out is written against), with regressions for shadowing and for mid-iteration deletion. **Owned by a slot rather than a §5 slice** because it is a live defect in a connected dispatch handler, which §1.0's derivation does not produce; 0bc owns only the store into the left-hand side | the next VM dispatch-loop PR, or with `#11-vm-operand-rooting-by-construction` | 2026-09-30 |
| `#11-vm-parameter-environment-instantiation` | **(carved at PR-B)** `compile_param_prologue` (`compiler/expr_function.rs:31-66` at `658cc302`) lowers a parameter default as read-the-slot / compare-to-`undefined` / conditionally store over positional slots the caller has already filled, and the arm's own comment records the consequence — "param bindings carry no TDZ" (`:52`); `visit_function` (`scope/visitor.rs:442-486`) pushes a single `ScopeKind::Function` (`:456`) and visits both the parameters (`:479`) and the body statements (`:481-483`) into it. ECMA-262 **§10.2.11** FunctionDeclarationInstantiation requires the opposite on both counts: step 21.3.a creates each parameter binding with `CreateMutableBinding(paramName, false)` and step 28's IteratorBindingInitialization is what initializes them, so a default initializer that reads a later parameter must raise rather than read a filled slot; and step 30.2 gives the body its own `NewDeclarativeEnvironment(envRecord)` whenever step 8's `hasParamExprs` (ContainsExpression of formals) holds, so a body `var` must not be visible to a default initializer. **A third obligation sits in the same AO and the same emit order**: step 22.5 "Perform ! envRecord.InitializeBinding(\"arguments\", argumentsObj)" precedes the step-28 IteratorBindingInitialization already cited, so a default initializer may read the arguments object (`webref body ecma262 sec-functiondeclarationinstantiation`). elidex emits the two in the opposite order — `compile_nested_function` calls `compile_param_prologue` at `compiler/expr_function.rs:136` and reaches the `CreateArguments` / `SetLocal` / `InitLocal` sequence only afterwards (`:156-158`, under the `uses_arguments` guard at `:139`). The guard is not what suppresses it: `visit_params` visits a parameter default through `visit_expr` (`scope/visitor.rs:513-514`, reached from `:479`), so `uses_arguments` is set and the sequence *is* emitted — after the prologue that reads its slot. Deliverable is that environment shape — uninitialized parameter bindings initialized left to right, the arguments binding established before any of them, and a body environment distinct from the parameter one under the step-8 condition — with a regression for each; for the third, `function f(a = arguments.length, b) { return a } f(undefined, 2)` → `2`. Stated here rather than as a second slot because all three are steps of one AO and one emit sequence. **Owned by a slot rather than a §5 slice** because it is a live defect in fully emitted code, which §1.0's derivation does not produce; 0bc's parameter coverage is binding-pattern storage and iterator closing, a different mechanism | the next `compiler/expr_function.rs` + `scope/` PR, or with Slice **B** | 2026-09-30 |
| `#11-vm-derived-constructor-this-binding` | **(carved this round)** a derived constructor's `this` is never uninitialized, so a pre-`super()` read succeeds where it must throw. `do_new` (`vm/ops.rs:681` at `658cc302`) allocates the instance unconditionally (`:766-771`) and passes it as the new frame's `this` (`:784-793`, `push_js_call_frame(callee, JsValue::Object(instance), …, Some(instance), CallMode::Construct { … })`); `Op::PushThis` (`vm/dispatch.rs:738-741`) pushes `frames[frame_idx].this_value` with no status check; and `dispatch_super_inner` (`vm/dispatch_class.rs:115`) reads that same `this_value` as the receiver it forwards (`:136`) and then *substitutes* it with what `construct_synchronous` returned (`:152`) — a swap, not the establishing of a binding. ECMA-262 **§10.2.2** `[[Construct]]` creates `thisArg` only "If kind is base" (step 3) and binds it only under the matching step 6, so a derived constructor's `this` is unbound until `super()`; **§9.1.2.4** NewFunctionEnvironment step 4 is "Else, set envRecord.[[ThisBindingStatus]] to uninitialized", **§9.1.1.3.1** BindThisValue step 2 throws a ReferenceError when the status is already initialized and step 4 sets it, and **§9.1.1.3.3** GetThisBinding step 2 throws when it is uninitialized — which §10.2.2 step 15 reaches for a derived constructor that completes without one. §-numbers from `webref heading ecma262 10.2.2` / `9.1.1.3.3` and `webref aoid ecma262 NewFunctionEnvironment` / `BindThisValue`; **§9.1.1.3.3 is GetThisBinding and §9.1.1.3.4 is HasSuperBinding**, so derive rather than recall. Deliverable is that status on the call frame, with one regression per throw site: `class B extends A { constructor() { this.x = 1; super() } }`, `class B extends A { constructor() { super(); super() } }`, and `class B extends A { constructor() {} }` must each throw. **The return-substitution branch of the same clause is this slot's too — a fourth throw site, not a second mechanism.** `complete_inline_frame` (`vm/dispatch.rs:1086` at `b5356098`; `git diff --stat 658cc302 b5356098 -- crates/` is empty, so the reading holds at `658cc302`) substitutes for **every** non-Object return whenever the field is present — `let final_val = if let Some(instance_id) = frame.new_instance { if matches!(return_value, JsValue::Object(_)) { return_value } else { JsValue::Object(instance_id) } }` (`:1089-1094`) — with no test of base against derived and none of `undefined` against another primitive, while `do_new` sets `new_instance` for a derived constructor as well (this row's own measurement above). So `class B extends A { constructor() { super(); return 1 } }` succeeds and evaluates to the instance. ECMA-262 **§10.2.2** separates the three cases in consecutive steps: step 12 "If result.[[Value]] is an Object, return result.[[Value]]"; step 13 "If kind is base, return thisArg" — the substitution the handler performs unconditionally, and the only case in which it is correct; step 14 "If result.[[Value]] is not undefined, throw a TypeError exception" — and `undefined` is permitted by falling past it to step 15's "Let thisBinding be ? ctorEnv.GetThisBinding()" and step 17's return of that binding (steps from `webref body ecma262 sec-ecmascript-function-objects-construct-argumentslist-newtarget`). **Extended rather than carved**: step 15 is the same GetThisBinding the third regression above already reaches, and steps 13/14 turn on the same base-against-derived `kind` this deliverable puts on the frame, so the handler needs no state this slot is not already adding — a separate slot would restate one clause's branch table across two ids. Fourth regression: `class B extends A { constructor() { super(); return 1 } }` must throw a TypeError, against `return undefined`, which must still yield the instance. **Carved rather than folded into Slice 2ab**, whose charter is *when* instance elements are initialized under §10.2.2 and §7.3.33: this adds the Function Environment Record this-binding AOs as a second governing algorithm plus a call-frame representation change reaching `vm/ops.rs`, `vm/interpreter.rs`, `vm/dispatch.rs` and `vm/dispatch_class.rs`, which §5's terminality criterion forbids in one row — the same reason `#11-vm-arrow-lexical-super` was carved off Slice 3. 2ab satisfies all three of its field-placement regressions without it, which is why the split has to be stated rather than left to be noticed at 2ab's retirement. **Owned by a slot rather than a §5 slice** because it is a live defect in fully emitted code, which §1.0's derivation does not produce | with Slice **2ab**, or the next `vm/interpreter.rs` call-frame PR | 2026-09-30 |
| `#11-vm-ordinary-function-name-length` | **(carved this round)** an ordinary function gets no own `name` and no own `length`. `create_closure` (`vm/ops.rs:820` at `658cc302`) allocates the function object root-shaped (`:882-891`) and defines exactly one own property on it — `.prototype`, non-arrow only (`:918`). Negative control for the absence: the same `define_shaped_property(…, well_known.name, …)` shape *is* present elsewhere — `vm/shape_ops.rs:513` for built-in functions and `vm/ops.rs:211` for Error instances — so the pattern discriminates; and `git grep -n 'well_known.length' 658cc302 -- crates/script/elidex-js/src/vm/ops.rs` returns nothing while the identical shape for `well_known.prototype` over that same file returns `:755` and `:918`. The only compensation is bind-local: `target_function_length_name` (`vm/natives_function.rs:151`, called once, from `:100`) falls back to `internal_function_length` (`:205`) and `internal_function_name_u16` (`:212`) when the property is absent, and that length source is `CompiledFunction.param_count`, assigned `func.params.len()` (`compiler/expr_function.rs:220`; arrows `:287`). ECMA-262 **§10.2.3** OrdinaryFunctionCreate step 21 is "Let length be the ExpectedArgumentCount of paramList" and step 22 performs **§10.2.10** SetFunctionLength, whose step 2 defines `length` with `[[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: true`; **§15.1.5** ExpectedArgumentCount returns 0 for a `FormalParameterList : FormalParameter` whose HasInitializer is true and returns the count to the left of the rest parameter, so `params.len()` is the wrong number as well as the wrong place. **§15.2.4** InstantiateOrdinaryFunctionObject step 4 and **§15.2.5** InstantiateOrdinaryFunctionExpression perform **§10.2.9** SetFunctionName, whose step 6 defines `name` with those same attributes. §-numbers from `webref aoid ecma262 OrdinaryFunctionCreate` / `SetFunctionLength` / `SetFunctionName` / `ExpectedArgumentCount` / `InstantiateOrdinaryFunctionObject` / `InstantiateOrdinaryFunctionExpression`. Deliverable is both own properties at closure creation, with a default-parameter and a rest-parameter regression: `function f(a, b = 0, ...c) {}` must give `f.length === 1` and `f.name === "f"`. **Not Slice 9's**: §5's Slice 9 row scopes its derivation to the property surface ECMA-262's main body defines *on an intrinsic object*, and these are the own properties syntax gives every ordinary function, so 9 can retire with its inventory complete and these still absent. **Owned by a slot rather than a §5 slice** because `create_closure` emits and connects correctly — a live defect in fully emitted code, which §1.0's derivation does not produce | the next `vm/ops.rs` closure-creation or `Function.prototype` PR | 2026-09-30 |
| `#11-vm-object-literal-proto-setter` | **(carved this round)** `{__proto__: p}` defines an own property instead of setting the prototype. The parser already isolates the production and then discards the distinction: `parse_object_literal` tracks `has_proto` (`parser/object.rs:21`) for a non-computed `__proto__` identifier or string key (`:86-102` at `658cc302`) solely to raise the duplicate-field SyntaxError. `compile_object_expr`'s `PropertyKind::Init` arm (`compiler/expr_object.rs:46`) has no `__proto__` branch — an `Identifier` key (`:84`) or a `Literal::String` key (`:89`) lowers to `Op::DefineProperty` like any other, so `const p={x:1}; const o={__proto__:p}` leaves `o`'s prototype unchanged and gives it an own `__proto__` data property. ECMA-262 **§13.2.5.6** PropertyDefinitionEvaluation, production `PropertyDefinition : PropertyName : AssignmentExpression`, step 3 sets `isProtoSetter` when "propertyKey is \"__proto__\" and IsComputedPropertyKey of PropertyName is false"; step 7.1 then performs `obj.[[SetPrototypeOf]](propertyValue)` when "propertyValue is an Object or propertyValue is null" and step 7.2 returns — so for any other value the production defines nothing at all — and step 5 suppresses NamedEvaluation while `isProtoSetter` is true (`webref aoid ecma262 PropertyDefinitionEvaluation`, `webref body ecma262 sec-runtime-semantics-propertydefinitionevaluation`). Deliverable is that branch, with one regression per value class in step 7: an object, `null`, and a primitive that must leave the literal with neither the property nor a changed prototype. **Core, not Annex B**: `webref body ecma262 sec-runtime-semantics-propertydefinitionevaluation` resolves to `multipage/ecmascript-language-expressions.html`, so the initializer form is main-body normative; the separate `__proto__` **accessor** is §20.1.3.8 (`webref heading ecma262 20.1.3`). `crates/script/elidex-js/src/lib.rs:18` lists "`__proto__` semantics" inside its "No Annex B" policy paragraph; that sentence reaches the accessor and not §13.2.5.6, and the slice that lands this narrows it. **Not `#11-vm-property-key-lowering-unification`**: that slot's deliverable is collapsing the open-coded `PropertyKey` → static-name lowerings into one, and every one of them would still emit a define for this key; the mechanism here replaces the define with `[[SetPrototypeOf]]` on a production-level test, a different governing algorithm. Both land in `compile_object_expr`'s `Init` arm, so whichever goes second inherits the other's shape — an ordering fact, not a shared charter. **Owned by a slot rather than a §5 slice** because none of §2.2's three keys is on this lowering: it is a marker-free `Op::DefineProperty` emit inside an arm that exists, so a marker sweep, a `PushUndefined` census and a variant-presence walk each pass over it — the §2.2 property that section states as a property of its commands | the next `compiler/expr_object.rs` PR, or with `#11-vm-property-key-lowering-unification` | 2026-09-30 |
| `#11-vm-instanceof-callable-check` | **(carved at PR-B)** `op_instanceof` (`vm/dispatch_objects.rs:352-384` at `658cc302`) rejects a non-Object right operand (`:353-357`), then either calls whatever the `@@hasInstance` lookup returned (`:361-364`) or walks the left operand's prototype chain against `rhs.prototype` (`:367-383`) — with no callability test between the two, so `({}) instanceof {prototype:{}}` answers `false`. ECMA-262 **§13.10.2** InstanceofOperator puts one there: step 2's `GetMethod` treats a `null` or `undefined` `@@hasInstance` as absent and step 3 therefore routes it to the default path, step 4 is "If IsCallable(target) is false, throw a TypeError exception", and only step 5 reaches **§7.3.21** OrdinaryHasInstance — whose step 5 additionally throws when the `"prototype"` read is not an Object, where the handler answers `false`. Deliverable is that ordering, with regressions for a non-callable right operand, a `null`-valued `@@hasInstance`, and a callable whose `prototype` is a primitive. **Owned by a slot rather than a §5 slice** because it is a live defect in a connected dispatch handler, which §1.0's derivation does not produce — the same routing as `#11-vm-for-in-enumeration-semantics` and `#11-vm-object-spread-source-coercion` | the next VM dispatch-loop PR, or with `#11-vm-proxy-reflect` | 2026-09-30 |
| `#11-vm-const-assignment-runtime-throw` | **(carved at PR-B)** `compile_identifier_store` returns `CompileError { message: "Assignment to constant variable '…'" }` for a write to a `BindingKind::Const` local (`compiler/expr_assign.rs:81-88` at `2039766a`), so a syntactically valid assignment yields no bytecode for the whole script and `try { const x = 1; x = 2 } catch (e) { 42 }` never reaches its `catch`. ECMA-262 puts the failure at run time: **§13.15.2** Runtime Semantics: Evaluation reaches **§6.2.5.6** PutValue, whose Declarative Environment Record path is **§9.1.1.1.5** SetMutableBinding step 5.b, "If strict is true, throw a TypeError exception". **The remedy is not re-derived here**: §17's private-store CRIT had this exact shape — loud but not scoped — and §9 dec. 5 resolved it against measured evidence to a runtime throw carried by `Op::ThrowUnsupported` (an engine-constructed error raised through `throw_error`, catchable and local), with `CompileError` reserved for what the spec rejects before execution. Deliverable is that conversion at this arm, with a regression that the `catch` runs and the statements after it still execute — the observable §18 measured as the cost of the `CompileError` form. **Owned by a slot rather than a §5 slice** because §2.2's pass 3 flags an arm that neither emits an op nor returns `CompileError`, and this arm returns one, so §1.0's derivation does not produce it | the next `compiler/expr_assign.rs` PR — Slices **0bb** and **0bc** both name that file | 2026-09-30 |
| `#11-vm-upvalue-tdz-check` | **(carved at PR-B)** `compile_identifier_load` emits `Op::CheckTdz` before `Op::GetLocal` for a `VarLocation::Local` (`compiler/expr_assign.rs:34-42` at `2039766a`) and a bare `Op::GetUpvalue` for a `VarLocation::Upvalue` (`:45`), and the runtime half cannot recover the check: the `Op::GetUpvalue` arm pushes `read_upvalue` (`vm/dispatch.rs:155-160`), which returns the stack slot or the closed value (`vm/ops.rs:597-602`); `UpvalueState` carries exactly `Open { frame_base, slot }` and `Closed(JsValue)` (`vm/value.rs:952-957`), with no uninitialized state; and the TDZ bits sit on `CallFrame` (`is_in_tdz`, `vm/value.rs:1106`), which an upvalue read never consults. So `let f = () => x; f(); let x = 1` reads `undefined` where **§9.1.1.1.6** GetBindingValue step 2 throws a ReferenceError for an uninitialized binding, "regardless of the value of strict". Deliverable spans the three layers together — an uninitialized state the upvalue representation can carry, the emit that checks it, and the dispatch arm that raises — with a regression reading a captured binding before its declaration. **Owned by a slot rather than a §5 slice** because §2.2's sweep is over `compiler/` arms and this arm emits an op, so §1.0's derivation does not produce it; the defect is only visible across the compiler/runtime seam | the next `vm/value.rs` + `compiler/` PR, or with Slice **B** (`#11-vm-block-declaration-instantiation`), which owns when a block's bindings become initialized | 2026-09-30 |
| `#11-vm-named-function-expression-binding` | **(carved at PR-B)** `visit_function` adds a function **expression**'s own name as a `BindingKind::Function` binding inside the function's scope (`scope/visitor.rs:458-463` at `2039766a`), `compile_nested_function` initializes every such local above the parameter count to `undefined` (`compiler/expr_function.rs:120`, `:130-132`), and the `ExprKind::Function` arm emits only `Op::Closure` (`compiler/expr.rs:210-214`) — the two *declaration* paths are the ones that follow `Op::Closure` with a `resolve_identifier` and a store (`compiler/mod.rs:93-96`, `compiler/expr_function.rs:180-183`), which is the probe that separates them rather than counting the opcode. So `let f = function g(){ return g }; f() === f` is false and recursion through `g` fails. ECMA-262 **§15.2.5** InstantiateOrdinaryFunctionExpression's `FunctionExpression : function BindingIdentifier ( FormalParameters ) { FunctionBody }` production requires a different shape: step 4 `NewDeclarativeEnvironment(outerEnv)`, step 5 `CreateImmutableBinding(name, false)`, step 8 creating the closure against that environment, step 11 `InitializeBinding(name, closure)`. Deliverable is that environment and its initialization — immutable, and reachable only from inside the body — with regressions for recursion through the self-name and for its identity with the bound value. **Owned by a slot rather than a §5 slice** because the arm emits an op, so §1.0's derivation does not produce it, and no §5 row owns function-expression naming (`git show 2039766a:docs/plans/2026-07-vm-p4-es-language-completeness.md \| grep -ci 'function expression'` → **0**, against **8** for `'static block'` and **38** for `'arrow'` by the same command on the same blob, so the probe discriminates) | the next `compiler/expr_function.rs` + `scope/visitor.rs` PR, or with Slice **B** | 2026-09-30 |
| `#11-vm-private-name-early-errors` | **(carved this round)** a private reference is never validated against a declared private name, so `({}).#x` and `class C { m() { return this.#missing } }` reach lowering where ECMA-262 requires an early SyntaxError — including where the reference sits in code that never runs, which is the case no runtime diagnostic can reach. Three layers, each measured at `b5356098` (§2.2's `parser/primary.rs:200-203` row): `parse_member_prop`'s `PrivateIdentifier` arm returns `MemberProp::PrivateIdentifier` without consulting which private names are in scope; the parser's only private-name set is a `parse_class_body` local (`function.rs:370-371`) that `check_private_name_dup` (`:281-292`) fills from **declarations** for duplicate detection and that dies with the loop; and scope analysis carries no declared-private-name environment at all — `git grep -in 'private' b5356098 -- crates/script/elidex-js/src/scope/` returns three lines, all child traversals (`visitor.rs:278`, `:546`, `:550`), against **48** files for the same case-insensitive pattern over `crates/script/elidex-js/src/`. Governing: **§15.7.7** Static Semantics: AllPrivateIdentifiersValid, whose reference productions return false when `names` does not contain the StringValue and whose `ClassBody : ClassElementList` production supplies those names, checked as an early error by **§16.1.1** (`ScriptBody : StatementList`) and **§16.2.1.1** (`ModuleBody : ModuleItemList`) — **not** by §15.7.1, which does not mention the SDO. Deliverable is that environment — the private names each enclosing class body declares, carried to every reference site — plus the two early errors written over it, with regressions for a reference with no enclosing class, a reference to an undeclared name inside a class, and a reference in **unreached** code, the last being what separates an early error from a runtime throw. **Carved rather than routed to Slice 0ba**: 0ba's contract is an arm that accepts a token the grammar cannot derive *in the context the arm was reached from*, and each of its fixes is a condition on an existing arm over state the arm already has — the `#x`-as-property-key row is answered by which of `parse_property_key`'s three callers is asking. A private *reference* is grammatical wherever it is written; what decides it is a set of names accumulated by enclosing class bodies, so the fix is a new carrier threaded through the parser or the scope pass, which is a representation change rather than a condition. **Nor to Slice 5**, whose charter is the compiler emit and VM dispatch of the private *operations* (`compiler/expr_class.rs`, `expr_member.rs`, `expr_ops.rs`, `expr_assign.rs:202-206`, `vm/dispatch.rs:1020-1026`): all of those can land while `({}).#x` still compiles, because nothing on that path asks whether the name was declared. **Owned by a slot rather than a §5 slice** because the defect is in `parser/` and `scope/`, which §1.0's derivation does not read | the next `parser/` + `scope/` PR, or with Slice **5** | 2026-09-30 |
| `#11-vm-explicit-resource-management` | **(carved at PR-B)** Slice **R**, promoted to its own umbrella like Slice M. `using` / `await using` have no AST production, so they are parse errors, and the whole object surface is absent: `grep -rl <name> crates/script/elidex-js/src` → **0** files for each of `UsingDeclaration`, `await using`, `DisposableStack`, `AsyncDisposableStack`, `SuppressedError`; `typeof` on each of the last three, and on `Symbol.dispose`, → `"undefined"`. Spec surface, all numbers from `webref heading ecma262`: **§14.3.1** *Let, Const, Using, and Await Using Declarations* (declaration forms — there is **no** `#sec-using-declarations` anchor; `webref` rejects it), **§27.2** *Resource Management* (§27.2.1.1 Disposable / §27.2.1.2 AsyncDisposable interfaces), **§27.3** *DisposableStack Objects*, **§27.4** *AsyncDisposableStack Objects*, **§20.5.8** *SuppressedError Objects*. ⚠ §27.2 is Resource Management; **Promise Objects is §27.5** (the correction the `#11-vm-builtin-prototype-static-sweep` row records) — and `DisposableStack` / `AsyncDisposableStack` are §27.3 / §27.4, *not* under §27.2, so do not cite §27.2 for them. Edge-dense — parser productions × a scope-analysis binding kind carrying a disposal list × abrupt-completion disposal ordering against `try`/`finally`, `return` and `IteratorClose` ⇒ **plan-review MANDATORY** (CLAUDE.md rule, not judgment), and the umbrella derives its own sub-slices from those sections rather than inheriting a list from here. Invisible to §2.2 by that section's stated scope: pass 3 walks `ExprKind`/`StmtKind` variants, and this syntax has none | a WPT/site depending on it, or the next `parser/` + `scope/` language-feature program | 2026-12-31 |
| `#11-vm-async-generators` | needs a new async-iterator opcode + `compiler/stmt.rs` await-flag plumbing (the `is_await: _` discard — `:83` at `658cc302` and at HEAD, `git grep -n 'is_await: _'`; the `:103` this row used to carry predates 0a's `stmt.rs` split and was never re-derived) = own slice | now (Slice 6) | 2026-10-31 |
| `#11-vm-builtin-prototype-static-sweep` | **Umbrella slot whose scope is a derivation rule, not a list of sub-slices** (§5), and the rule is **user-reachable** rather than global-rooted: for every intrinsic object a program can obtain — by reading a global property, by evaluating syntax that produces it, or by reading a property of a value it already holds — the whole property surface ECMA-262's main body defines on it, minus Annex B (I-6), minus any surface another slot already owns. The rule reads the spec clause, not the object's shape and not the route — see §5 for why the "constructor / prototype surface" and "reachable from the global object" wordings were each replaced. Running that derivation is the umbrella's first step; its output is a set of groupings from which terminal sub-slices are minted by §5's terminality criterion — **not one per family**, for the reason §5's head states — each its own PR under its own mandatory `/elidex-plan-review`. The sub-slice rows in §5 are the derivation's output so far and are groupings themselves — a family absent from them is un-minted, not out of scope. *(The earlier rationale "batched to avoid N micro-PRs" is withdrawn: batching those families into one PR is what the CLAUDE.md* Edge-dense work *rule forbids, and that rule says a plan-review does not substitute for the split.)* It retires only once the derivation has been run and every sub-slice it mints has landed; a partial landing is recorded against the sub-slice, not against this id. Pure builtin surface, no language-layer dependency. Charter is **edition-agnostic** — the unit is the *property surface ECMA-262's main body defines on a builtin object*, which is why the id's "prototype-static" wording is a name and not the rule — **minus Annex B** (§B.2.1 / §B.2.2, `Date.prototype.getYear`/`setYear`, `RegExp.prototype.compile`), which §4 I-6 forbids program-wide. *(Renamed from `#11-vm-es2021-2024-prototype-sweep`. The old id occurred **only in this document** — repo-wide `grep -rl 'es2021-2024'` returns this file alone, live ledger returns 0 — so the rename cost nothing, and it removes two ⚠ annotations that existed solely to tell readers the name lied about its own scope.)* **This row is the permanent home for the measurements below** (§10's round record found the first three, and a round record owns nothing), each tagged with the sub-slice that discharges it: **(1) → 9b** `String.prototype.replaceAll` exists — `vm/natives_string_ext.rs:205` doc comment "ES2021 String.prototype.replaceAll(searchValue, replaceValue)", registered at `vm/globals_primitives.rs:62` `("replaceAll", native_string_replace_all)` — so this is a behavioural fix, not an absence: `'aabb'.replaceAll(/b/,'x')` → `'aabb'` where ECMA-262 **§22.1.3.20** step 3.b.iii throws a **TypeError**, and `'aabb'.replaceAll(/b/g,'x')` → `'aabb'` too, so the `g` path is silently wrong as well. **(2) → 9c** `Object.fromEntries` — `native_object_from_entries` at `vm/natives_object/iteration.rs:157` requires `ObjectKind::Array` and otherwise raises `"Object.fromEntries requires an iterable"` (`:165` / `:181`), then index-walks the dense array; **§20.1.2.7** step 6 instead runs `AddEntriesFromIterable` (**§24.1.1.2**), the iterator protocol. Measured: `function* g(){ yield ['a',1] }; Object.fromEntries(g())` → `TypeError`. **(3) → 9b** `String.prototype.matchAll` genuinely absent — `typeof 'ab'.matchAll` → `undefined`, and `grep -rn 'matchAll' crates/script/elidex-js/src` → **23** hits, every one ServiceWorker `clients.matchAll` / Cache `matchAll`. A parallel ES2019/ES2020 slot is **not** to be minted — that is the duplicated mechanism CLAUDE.md *One issue, one way* forbids. **(4) → an un-minted sub-slice, and the reason §5's rule was re-derived at PR-B**: `Math` is neither a constructor nor a prototype, so the previous wording could not produce it and it had no owner. Measured at `658cc302` (`crates/` is byte-identical at `b478b2b6` — `git diff --stat 658cc302 b478b2b6 -- crates/` is empty): `register_math_global` (`vm/globals.rs:1466`) installs **27** methods (`awk 'NR>=1468 && NR<=1494' crates/script/elidex-js/src/vm/globals.rs \| grep -c '^\s*("'`) while ECMA-262 **§21.3.2** *Function Properties of the Math Object* carries **37** (`.claude/tools/webref heading ecma262 21.3.2 \| grep -c 'Math\.'`); every installed name is a §21.3.2 entry, and `comm -23` over the two sorted name lists leaves **10** absent — `acosh` `asinh` `atanh` `cosh` `expm1` `f16round` `log1p` `sinh` `sumPrecise` `tanh`, each returning **0** files from `grep -rl '"<name>"' crates/script/elidex-js/src`. `JSON` (`register_json_global`, `vm/globals.rs:1520`) is the same shape and is likewise produced by the re-derived rule. The sub-slice is minted by running the derivation, not by this row. **(5) → an un-minted sub-slice, and the reason §5's rule was widened a second time**: the shared generator prototype is never a global property, so a rule rooted at the global object could not produce it either. Measured at `658cc302` (`git diff --stat 658cc302 909a77cb -- crates/` is empty, so the reading holds at this document's HEAD as well): `register_generator_prototype` (`vm/globals_async.rs:84-105`) builds the object with `create_object_with_methods(&[("next", …), ("return", …), ("throw", …)])` (`:90-94`) and one `[Symbol.iterator]` (`:96-104`), and defines nothing further — `create_object_with_methods` (`vm/globals.rs:1025-1037`) allocates an `Ordinary` object and calls `install_methods` with exactly the pairs it is given, so no `constructor` and no `%Symbol.toStringTag%` reach it. The sibling `Promise` registration in the same file **does** define `constructor` (`globals_async.rs:50-56`), so the absence is a property of this function and not of the file. ECMA-262 **§27.8.1** *The %GeneratorPrototype% Object* defines both beside the three methods — **§27.8.1.1** `%GeneratorPrototype%.constructor` and **§27.8.1.5** `%GeneratorPrototype% [ %Symbol.toStringTag% ]` (`webref body ecma262 sec-properties-of-generator-prototype`, `webref heading ecma262 27.8`). A program obtains the object only through syntax — `Object.getPrototypeOf((function*(){})())` — since there is no `GeneratorFunction` surface to read it off: `grep -rn 'GeneratorFunction' crates/script/elidex-js/src` returns exactly one line, a comment at `vm/tests/tests_typed_array_static.rs:344`, so the pattern does match in-tree text and still reaches no installation site. The function's own citations drifted with it: `:81` reads "Generator.prototype (§25.4.1)" and `:95` "spec §25.4.1.5", while `webref heading ecma262 25.4.1` is *Waiter Record*; the retag belongs to whichever sub-slice the derivation mints for this object. **Also owned here, by 9b**: the `vm/natives_string.rs` §21.1.3 → §22.1.3 citation retag, moved off `#11-vm-webidl-section-3-10-retag` because that slot's WebIDL class and this ECMA one fire independently and one retirable id cannot record half a discharge. `git show 658cc302:crates/script/elidex-js/src/vm/natives_string.rs \| grep -c '§21\.1\.3'` → **8**, and **8** at HEAD `6edda6f2`; `webref heading ecma262 21.1.3` is "Properties of the Number Prototype Object" and `22.1.3` is "Properties of the String Prototype Object". Re-derive by grep at slice time rather than reading the figure forward | any sub-slice the derivation mints (§5 carries them), or a WPT/site needing `at`/`findLast`/`hasOwn`/`fromEntries` on a non-Array iterable/`matchAll`/`withResolvers` | 2026-10-31 |
| `#11-vm-dynamic-import` | the T1 carve out of the ES-modules program: 0ca makes `import()` loud immediately, the Promise-returning implementation needs the module loader | Slice M, or the ES-modules umbrella | 2026-10-31 |
| `#11-vm-topropertykey-symbol-from-toprimitive` | **(carved by #489's converge)** §7.1.20 tests the *argument* for Symbol-ness, not the ToPrimitive *result*, so an `@@toPrimitive`-returns-Symbol key throws. Open-coded **8 times** (2 named helpers + 6 inline) and `get_element`/`set_element` are **not** `make_property_key` callers ⇒ the unit is "collapse the 8, then fix", not a helper patch | the next property-key coercion PR, or with `#11-vm-proxy-reflect` | 2026-09-30 |
| `#11-vm-operand-rooting-by-construction` | **(carved by #489's converge; SUPERSEDES `#11-vm-element-access-base-rooting`, which must not be recorded as closed)** ~20 dispatch arms pop an operand into a Rust local and then run user JS before reading or storing through it; `gc/roots.rs` walks the VM stack but not Rust locals. Beyond the 5 element opcodes: `GetProp`/`SetProp`, `IncProp`/`DecProp`, `In`, `Add`, `Instanceof`, `TemplateConcat`, `ops.rs`'s three operator helpers, the three computed-key definition bodies, `SpreadObject`, `ArraySpread`, `IteratorRest`, the unary arms and `op_get_iterator` — two of them *panic*, two *store* the collected id. **Deliverable is an invariant making an unrooted hold unrepresentable, NOT a sixth sweep**: five successive audits each drew the boundary differently and each was falsified by the next round. `Op::GetElemRef`, the one arm #489 introduces, is rooted by construction and pinned. Implementation + a 14-arm test module preserved on branch `vm-p4-rooting-carved` | **now** — the next VM dispatch-loop PR, or Slice P. Edge-dense ⇒ plan-review MANDATORY | 2026-09-30 |
| `#11-vm-internal-error-hard-exit` | **(carved by #489's converge)** every extracted `op_*` helper's dispatch arm routes `VmErrorKind::InternalError` through `throw_error`, so a broken VM invariant becomes a catchable JS `Error` and user `try`/`catch` can swallow it; inline arms (`Op::Swap` / `Op::Pop` / `Op::PopUnder`) propagate with `?` instead. The disposition must key on the error's *kind*, not on whether the body was extracted. `Op::ThrowUnsupported` must stay catchable — it reports an unimplemented construct, not a broken invariant | with `#11-vm-operand-rooting-by-construction`, or the next dispatch-loop PR | 2026-09-30 |
| `#11-vm-delete-elem-raw-key-array-fast-path` | **(carved by #489's converge)** `Op::DeleteElem` derives its array-index fast path from the **raw** operand, so an object key that stringifies to an index skips it while the generic `try_delete_property` path never consults dense array storage: `var a=[1,2,3]; delete a[{toString(){return '0'}}]` reports `true` and leaves `a[0]` as `1`. Same root as `#11-vm-topropertykey-symbol-from-toprimitive` — a fast path keyed on the raw value rather than the `ToPropertyKey` result | with that slot | 2026-09-30 |
| `#11-vm-yield-delegation-lowering` | **(carved at PR-B, Codex R3/R4)** `expr_yield_star.rs:146`/`:157` emit a plain `Op::IteratorClose` for outer `.throw(e)` / `.return(v)` injected during `yield*` delegation, and the compiler's own docstring admits it reduces both protocols to that. **ECMA-262 §15.5.5 steps 8.b/8.c** instead invoke the *inner* iterator's corresponding method **with the injected value**, validate the result, and **continue delegating when `done` is false** — which no `IteratorClose` operand or split opcode can express. Reclassified out of Slice P (§6.2a-3), so without this row the two known-broken generator paths would be permanently unowned. Deliverable is a yield-delegation lowering, not a precedence sweep — a **new lowering** for §15.5.5's injected-value resume protocol, spanning the compiler emit, the opcode/transport, the generator-resume path and both injection protocols ⇒ **edge-dense, plan-review MANDATORY** (CLAUDE.md makes this a rule, not a judgment) | with Slice 6 (async generators — same `expr_yield_star` / generator-resume surface), or the next generator PR | 2026-10-31 |
| `#11-vm-typed-array-family-layering-and-gate` | **(carved at PR-B by this document's own plan-review)** two normative sources disagree about the same files. `vm/host/mod.rs:11-13` states that everything under `vm/host/` is restricted to **engine-bound** responsibilities (prototype install / brand check / marshalling); §4 I-5's Converse declares `vm/host/typed_array_*.rs` — ECMA-262 §23.2, a pure language built-in — to be a **language-semantics site the core convention owns**. Both are normative and they cannot both hold, and there is no relocation slot. The measurable consequence the umbrella never inventoried: the whole family is `#[cfg(feature = "engine")]` (`vm/host/mod.rs:339-352` for the seven `typed_array*` modules, plus `array_buffer` `:75` and `data_view` `:102`), so a **non-`engine` core build has no `ArrayBuffer` / `TypedArray` / `DataView` constructors** — and the gating is not uniform: `ObjectKind::ArrayBuffer` is gated while `ObjectKind::TypedArray` and `ObjectKind::DataView` are **not**, so such a build carries a `TypedArray { buffer_id, … }` variant pointing at one that does not exist in that configuration. An ES-completeness hole absent from §2.2, §2.3, §3 and the §5 slice table, in an umbrella named for ES language completeness. Deliverable is a decision (relocate out of `vm/host/`, or ungate, or write the exemption into `vm/host/mod.rs` itself), not a doc edit | ⚠ **prerequisite of Slice Pa (Codex R4; narrowed from "Slice P" at PR-B)** — Pa must edit `vm/host/typed_array_static.rs:798`, an `iter_close` caller, so under the documented ship order Pa would have to either change pure ECMA-262 behaviour inside a directory whose mandate forbids it, or omit a required site and ship an incomplete convention sweep that Slice 0b then depends on. **Pa, not the P family's first PR**: §5 re-points the family to Pb → Pa → Pc, and Pb's touch set reaches no `vm/host/` file, so the gate blocks Pa alone and Pb can proceed while it is open. My earlier trigger ("Slice 7, or the next `vm/host/` layering pass") put the decision *after* the slice that needs it. Settle relocation / ungating / an explicit written exemption **before Pa**; Slice 7 remains the fallback owner if the P family is resequenced | 2026-10-31 |
| `#11-vm-statement-completion-updateempty` | **(carved by #489's converge)** the half of the completion-ownership bug 0a did not fix: no `UpdateEmpty` (AO §6.2.4.4) equivalent, so `42; if (false) {}` yields `42` where §14.6.2 says `undefined` — for *that* example at **step 3** (`IfStatement : if ( Expression ) Statement`, which returns before ever reaching step 5's `UpdateEmpty`); step 5 is the right cite for a truthy test or the else-bearing production. Preserving the already-correct forms while adding the resets is ≥3 axes ⇒ edge-dense, plan-review MANDATORY | the next VM statement-lowering PR | 2026-09-30 |
| `#11-vm-property-key-lowering-unification` | **(carved at PR-B)** `PropertyKey` → static-name lowering is open-coded at **6** match sites in `crates/script/elidex-js/src/compiler/` (`grep -rn 'match &prop.key\|match &property.key\|match key {\|let name_idx = match key' compiler/` at `39bbdb1b`: `expr_object.rs:26`, `expr_object.rs:83`, `stmt_destructure.rs:92`, `stmt_destructure.rs:135`, `expr_class.rs:550`, `expr_class.rs:596`). Five of them answer the same question and give **4 different** answers to a key kind they do not list: `expr_object.rs:26` `compile_accessor` emits `Op::Pop` (silent), `expr_object.rs:83` `PropertyKind::Init` is exhaustive, `stmt_destructure.rs:92` emits `Op::PushUndefined` (silent), and both `expr_class.rs` sites return `CompileError`. The sixth (`stmt_destructure.rs:135`, rest-key exclusion) answers a *different* question — which already-destructured keys to delete — and its `_ => {}` is paired with the computed-key temp-local loop below it; measured, it still drops non-computed non-Identifier/String keys: `const {1: a, ...r} = {1:'x', z:'y'}` leaves `r` as `{"1":"x","z":"y"}`. **Deliverable is one canonical `PropertyKey` lowering plus its consumers** (CLAUDE.md *One issue, one way*), not four catch-all patches — the point is that a key kind cannot be silently absent from one site. The five sites answering the same question **collapse** into it; the sixth (`stmt_destructure.rs:135`) answers a different question and is instead **converted to consume** the canonical lowering, since "collapse the partial copies" does not literally apply to it. **Ownership, scoped to match §2.2's own Slice column**: this slot owns the `expr_object.rs:26-39`, `expr_object.rs:117-121` (the `{1n:'x'}` empty-string key — **routed here off Slice 9**, because the arm is inside the `:83` `PropertyKind::Init` key match itself, not builtin surface), `stmt_destructure.rs:117-119` and `stmt_destructure.rs:149-150` rows outright, and of the `expr_class.rs` row **only the dead `if computed` guard** (an I-4 connect-or-delete) — the two *reachable* `_ =>` arms are **Slice 2's**, which is what that row's Slice column already says (`… (guard) · 2`). An earlier wording claimed all four rows plus the guard and over-read one of them. **Edge-density**: 6 sites × 3 files × key kind × syntactic position (object literal / class body / binding pattern / rest exclusion) × failure-mode discipline (four different current answers across I-1's substitution classes) = ≥3 intersecting invariant axes ⇒ **plan-review MANDATORY** (CLAUDE.md rule, not judgment). **Why the sweeps missed it**: §2.2's passes are file- and marker-oriented (`grep` per file, comment keywords, `Op::PushUndefined` occurrences), while this concept spans **3** files (`grep -rln 'PropertyKey::' crates/script/elidex-js/src/compiler/`) with catch-alls of four different shapes, one of them a `CompileError` return that no substitution-class grep sees | **now** — with Slice 0c's sweep, or whichever of Slices 2/5 first touches `expr_class.rs` or `expr_object.rs` (Slice 9 is no longer a trigger: `compiler/expr_object.rs` left its module column with the `{1n:…}` row) | 2026-09-30 |
| `#11-vm-atomics-global` | `typeof Atomics` is `undefined` (probe at `39bbdb1b`) and `grep -rn 'Atomics' crates/script/elidex-js/src` → 0. ⚠ **The earlier rationale — "meaningless without SharedArrayBuffer, which is itself COEP/COOP-gated, so the feature has no reachable surface today" — is spec-wrong for almost the whole global** and is withdrawn. Verified by `webref body ecma262`: **§25.4.8** `Atomics.isLockFree ( size )` takes only a byte size (`ToIntegerOrInfinity(size)`, then the surrounding agent's Agent Record) and touches no buffer at all; **§25.4.3.1** `ValidateIntegerTypedArray ( ta, waitable )` has **no SharedArrayBuffer requirement**, only `IsNoTearConfiguration` and, at **step 5**, "If waitable is true and elementType is neither int32 nor bigint64, throw a TypeError exception". ⚠ **Two routes into it, and an earlier version of this row collapsed them into one**: the **RMW / `load` / `store`** family reaches it via **§25.4.3.3** `ValidateAtomicAccessOnIntegerTypedArray`, whose step 1 passes `waitable` **false**; but **`notify` calls §25.4.3.1 directly with `waitable` true, and `wait` / `waitAsync` reach it one step in through `DoWait`** — `sec-atomics.notify` step 1 is `Let taRecord be ? ValidateIntegerTypedArray(ta, true)`, and `sec-dowait` step 1 is identical. Routing `notify` through the waitable-false gate would let `Atomics.notify(new Uint8Array(8), 0)` succeed where the spec throws. All four anchors re-verified with `.claude/tools/webref body ecma262 <anchor>`. **§25.4.10** `Atomics.notify` step 7 says "If IsSharedArrayBuffer(buffer) is false, return +0𝔽" — defined behaviour, not a throw. The **only** SAB gate is **§25.4.3.14** `DoWait` step 3 ("If IsSharedArrayBuffer(buffer) is false, throw a TypeError exception"), whose only callers are **§25.4.15** `Atomics.wait` and **§25.4.16** `Atomics.waitAsync` (each one step: `Return ? DoWait(…)`). So the read-modify-write / `load` / `store` / `isLockFree` / `notify` surface of §25.4 is reachable on ordinary `ArrayBuffer`-backed integer TypedArrays, which already exist in-tree (`grep -rn 'Int32Array' crates/script/elidex-js/src` → **17**, `typeof Int32Array` → `function`). ⚠ **"Implementable today" holds for `engine` builds only** — the whole backing surface is `#[cfg(feature = "engine")]`: `vm/host/mod.rs` gates `array_buffer` at `:75`, `data_view` at `:102` and the seven `typed_array*` modules at `:340`-`:352`; `Int32Array` is registered at `vm/host/typed_array_install.rs:463`; and `ObjectKind::ArrayBuffer` is gated at `vm/object_kind.rs:420-421` while `TypedArray`/`DataView` are not. That backing surface is what this slot **depends on `#11-vm-typed-array-family-layering-and-gate`** for — the same decision Slice P reaches from the other side. ⚠ **The earlier routing of this work into Slice 9's widened charter is withdrawn**: §25.4 operates on TypedArrays under `vm/host/`, behind that gate, while the Slice-9 umbrella's units are the `Array` / `String` / `Object` prototype-and-static surface in core `vm/natives_*`. This slot gets its own slice. **Split**: the non-`wait` surface is an ordinary unimplemented builtin; only `wait`/`waitAsync` are SAB-gated | non-`wait` surface: **after `#11-vm-typed-array-family-layering-and-gate`** — its own slice, not a Slice-9 sub-slice. `wait`/`waitAsync`: SharedArrayBuffer + COEP/COOP | non-`wait`: **2026-10-31** (matching the other builtin slots — the `2027-01-31` this row carried belonged to the withdrawn "no reachable surface today" rationale and did not follow it out); `wait`/`waitAsync`: 2027-01-31 |
| `#11-vm-weakref-finalization-registry` | ECMA-262 **§26.1 WeakRef Objects** / **§26.2 FinalizationRegistry Objects** (verified `webref heading ecma262 26.1` / `26.2`; both under §26 Managing Memory). Needs a GC-observability design of its own (exposing collection timing to script). Absent in-tree: `grep -rn 'WeakRef' crates/script/elidex-js/src` → 0, `FinalizationRegistry` → 0, and `typeof` on each is `undefined` (probe at `39bbdb1b`) | a WPT/site depending on them, or the GC-observability pass | 2027-01-31 |
| `#11-vm-function-constructor-global` | **(pre-existing, registered 2026-07-18 at #474's landing; ledger count 3)** global `Function` is absent and §9 dec. 7 recorded the policy decision as unowned, so I-6's core strict-mode `eval` surface could stay unimplemented forever inside a *completeness* program. `Function`'s constructor is §20.2.1 = dynamic creation via §20.2.1.1.1 CreateDynamicFunction, entangled with the core-strict-only stance. It opens its own umbrella when triggered — a policy call, not a language slice. **§5's slice table carries a pointer row only**: a full triple living in a slice-table cell is invisible to this section's derive recipe | **the ledger's registered trigger** (SoT): "a dynamic-`Function` need, OR the LegacySemantics/core-strict compat work". ⚠ **Additionally**, per this plan: the CSP / sandbox-policy decision, or the first WPT/site needing strict-mode `eval`. Neither trigger subsumes the other; **reconciling the two is owed on the ledger side at landing** and is not done by this PR | 2026-10-31 |
| `#11-vm-module-opcode-connects` | **(carved here)** Slice D's connect-or-delete criterion cannot be satisfied while `Op::ImportMeta` / `Op::DynamicImport` are assigned to the ES-modules umbrella's connect work (§2.3, §5 Slice D): D would have to delete two opcodes M owns, or leave them dead and fail its own criterion. Carving the two connects by name rather than by prose gives D's Deps something **retirable** — an escape hatch that is a permanent third state is exactly what I-4 forbids, and a carve-out recorded only in prose is unretirable (Forward rule). Distinct from `#11-vm-dynamic-import`, which owns `import()`'s Promise-returning *implementation*; this owns the two opcode connects D is blocked on | the ES-modules umbrella (Slice M), or whichever PR first enables `parse_module` | 2026-12-31 |
| `#11-vm-webidl-section-3-10-retag` | **(carved at PR-B by this document's own plan-review)** §16 asserts that "§3.10.16 and §3.10.21 do not exist in Web IDL at all" and 0a retagged **one file** (`vm/host/events_modern/touch.rs`). Verified with `webref heading webidl 3.10`: §3.10 *does* exist — "Observable array exotic objects" — but stops at **§3.10.9**, so every `§3.10.N` with `N > 9` is a dangling citation. Measured at HEAD: `grep -rnE '3\.10\.(1[0-9]\|[2-9][0-9])' crates/` → **58** sites over **10** distinct nonexistent numbers (`3.10.10` ×9, `.13` ×2, `.14` ×6, `.16` ×11, `.18` ×9, `.20` ×3, `.21` ×10, `.23` ×4, `.25` ×2, `.34` ×2) — and `touch.rs:253`, the very file 0a retagged, still carries §3.10.18. **This is an identified-and-unfixed class, so the Forward rule requires a row**; deliverable is the `§3.10.x → §3.2.x` retag derived by that grep, not by this list. **No `.rs` file was edited by this PR**. ⚠ **This slot owns the WebIDL class only.** The `vm/natives_string.rs` §21.1.3 → §22.1.3 ECMA-262 drift was folded in here and is **moved out to Slice 9b** (`#11-vm-builtin-prototype-static-sweep`): Slice P reaches the WebIDL callers but not `natives_string.rs`, so a single id could be neither retired after P nor marked half-discharged | with Slice P (which already re-attributes WebIDL sections in three of these files), or the next `vm/host/` citation sweep | 2026-10-31 |
| `#11-step9-class-extras` | **ADOPTED, not minted** — pre-existing and **already in the live ledger**. Scope "static members / private fields / getters & setters in class bodies / computed-name methods / static blocks / `Op::GetSuperProp` + `Op::SetSuperProp`". Slices 2/3/5 do **not** exhaust it — class static blocks are in the scope string and outside all three, and generic static-block lowering is assigned to Slice 2 above, with the measured scope-index defect and the retirement conditions. The adoption terms and the in-code retag obligation are stated above this table | Slices 2/3/5, plus Slice 2's static-block lowering | per the ledger entry |
| `#11-dead-opcode-removal` | **ADOPTED, not minted** — terms stated above this table; **absent from the live ledger** (0 hits), so the adoption is genuinely outstanding. Becomes Slice D, broadened to the §2.3 set — which now includes `Op::Wide`, whose prose-only "reserved" exemption was withdrawn at PR-B | Slice D (after every slice that connects a §2.3 opcode **and** `#11-vm-module-opcode-connects`) | 2026-10-31 |
| `#11-d17b-dispatch-expr-file-growth` | **ADOPTED, not minted** — terms stated above this table; **absent from the live ledger** (0 hits). Homes the `vm/dispatch.rs` + `compiler/expr_class.rs` + `vm/interpreter.rs` + `vm/value.rs` 1000-line debt | each slice that touches one of the four files | ⚠ **the carried re-eval `2026-08-08` has expired.** Re-set to **2026-09-30**, aligning it with the P / 0b / 0c cluster that touches these files first |
| `#11-compiler-class-emit-readability` | **(trigger this program fires)** Slices 2 and 5 both touch `compiler/expr_class.rs`. Registered here because a trigger stated only in §5 prose fires into a paragraph nothing re-reads at re-eval time; ledger count **0** | whichever of Slices 2 / 5 lands first | ⚠ **the carried re-eval `2026-08-08` has expired.** Re-set to **2026-09-30** |
| `#11-reflect-apply-ce-test` | **(trigger this program fires)** paired with Slice 10a, whose `Reflect.apply` deliverable §9 dec. 17 now scopes. Registered here for the same reason as the row above; ledger count **0** | Slice 10 | 2026-10-31 |
| `#11-webidl-sequence-dense-array-fast-path` | **ADOPTED, not minted** — registered by Slice 0a while it was in `vm/host/`, cited at three in-code sites and **already in the live ledger**. An Array fast path that skips WebIDL §3.2.21 step 2's `GetMethod`, so an overridden `@@iterator` is ignored. Belongs to Slice P's WebIDL half (§16, §4 I-5); it was adopted in prose above this table before PR-B, which made it invisible to the derive recipe | Slice P | per the ledger entry |
| `#11-vm-webidl-sequence-early-exit-drain` | **(carved at PR-B, by Slice Pc's re-derivation)** Pc deletes the `iter_close` call at four sites whose triggering early exit WebIDL **§3.2.21.1** does not have (Pc's §5 row enumerates them: an engine cap, two arity checks, an unsupported-transferable bail-out). Deleting the call does not make those four spec-faithful — they still **stop the drain early**, whereas §3.2.21.1 repeats `IteratorStepValue` until done and the consuming specs then check the *converted* sequence (URL §6.2 step 1, Fetch §5.1 step 1.1), so the observable `next()` call count still differs on a `sequence<sequence<…>>` longer than the check allows. Draining to done changes the cap's memory contract and the transfer-list bail-out's cost, which is a design question rather than a deletion, so it is not Pc's | with Slice Pc's own plan-review, or the first WPT/site that observes the `next()` count on an over-long sequence init | 2026-12-31 |
| `#11-vm-class-name-binding-initialization-order` | **(carved at PR-B)** `compile_class` (`compiler/expr_class.rs:99`) emits every class member — the `ClassMemberKind::StaticBlock` arm at `:448` included — and only afterwards initializes the inner class-name binding (`:479-500`), while ECMA-262 **§15.7.14** step 28 is `classEnv.InitializeBinding(classBinding, ctorFunc)` and step 32 is the loop over `staticElements`, so `class C { static self = C; static { if (C !== this) throw 0 } }` reads an uninitialized binding. **Slot-owned rather than row-owned** because the evidence block in §5 states it as a prerequisite **shared by Slices 2ac and 2b and a member of neither charter**, and does not choose between them — so neither Deps cell can carry it and a Deps edge would be the wrong shape. Whichever of 2ac/2b lands first moves the initialization and retires this slot; the other consumes it | Slice 2ac or Slice 2b, whichever starts first | 2026-10-31 |
| `Intl` → owned externally by [[intl-icu-deferral]] (**no slot minted here**) | governed by **ECMA-402**, not ECMA-262 — which is why it is out of this umbrella's spec scope as well as its implementation scope. Standing project decision: no ICU dependency. `typeof Intl` → `undefined` (probe at `39bbdb1b`). The row exists so the deferral is *visible in the registry* rather than only in prose | per that standing decision | per that standing decision |

**Cap accounting — by classification, not by a total.** The ratified policy counts a PR's *own*
deferrals (≤3) and separates two other categories out of that count. **Every row above carries
exactly one of the classifications below, and the classification is what the cap reads** — no row-total
is stated, because a hand total of a growing table is the figure this section already withdrew once:

- **Relocation of a disposition that already existed elsewhere in this document** — `Atomics`,
  `WeakRef`/`FinalizationRegistry` and `Intl`. Derivable: `git show 2a51106c` deletes the "**Not
  slotted**, each with its disposition: `Intl` → … `Atomics` → … `WeakRef`/`FinalizationRegistry` →
  …" paragraph from §5 and adds these three rows. They are not new deferrals; they moved out of a
  paragraph about `vm/dispatch.rs` line counts, which is not a place a deferral can be retired from.
- **Adoption of a pre-existing slot** — `#11-step9-class-extras`, `#11-dead-opcode-removal`,
  `#11-d17b-dispatch-expr-file-growth`, `#11-webidl-sequence-dense-array-fast-path`. Minted by
  earlier work, homed here. Not deferrals of any PR in this program.
- **Gate-found pre-existing defect** — everything carved by a review gate against code this program
  did not write, identified **by marker, not by a written list or count**: run
  `grep -n '^| .*carved by #489' <this file>` and `grep -n '^| .*carved at PR-B' <this file>`; the
  rows they return *are* the classification. ⚠ **A count is deliberately absent, and the reason is
  measured**: the previous wording asserted `carved at PR-B` → **4** and named four rows, while the
  same command returned **6** at `42c80199` — `#11-vm-object-spread-source-coercion` and
  `#11-vm-arrow-lexical-new-target` were added under the marker without the list being re-run. A
  figure that must be hand-maintained alongside a growing table is the class of number this section
  already withdrew once. Plus `#11-vm-module-opcode-connects`, carved here against pre-existing
  code, and `#11-vm-function-constructor-global`, registered 2026-07-18 at #474's landing. *(The
  `^| ` anchor is load-bearing: an unanchored `grep` matches the mentions in this very bullet — see
  [[feedback_document-landing-invalidates-its-own-measurements]].)*
- **Own deferral** — the remaining rows are **slice work items**, not deferrals
  at all (`computed-compound-assignment`, `assignment-target-completeness`,
  `iteratorclose-precedence-convention`, `async-generators`, `builtin-prototype-static-sweep`,
  `dynamic-import`, `compiler-class-emit-readability`, `reflect-apply-ce-test`).

**PR-B's own count is therefore 0**, by that rule: PR-B is a plan document; it writes no
implementation surface, so it can defer none. **#489's own count is 2** — the operand-rooting and
internal-error slots are its own *carves*, since it deliberately reverted work it had already
implemented (§18.1); ≤3 holds. Whatever the row total, the rows register as **discovery carves of
this umbrella** (the treatment the 2026-07-18 P4 registration used,
`project_open-defer-slots.md` §"VM P4 ES-language + builtin gaps",
"**Origin**: not a carve — a **discovery**"), not slice-introduced defers. Net ledger delta is
smaller than the table looks: the adoptions mint nothing, and
`#11-vm-class-instance-field-init` / `#11-vm-super-property-reference` from R1 are **withdrawn** in
favour of `#11-step9-class-extras`.

**Memo corrections at landing** — `project_vm-p4-es-language-gaps.md`:
- §2 table: add super-property total loss, public instance fields → `undefined`, `obj[k] += v`
  panic, destructuring assignment no-op, `import()` → `undefined`.
- §2 scope note: retract the `new X(...args)` "works" claim.
- §4: async generators resolved **broken**; `new.target` resolved **working in the
  direct-constructor spelling only** — the arrow case is Slice N (§1.1, §2.2).
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
   not in the registered slot set. Recommendation: admit — 0a is a process abort, 0b a silent no-op at the baseline
   on ubiquitous syntax, 0c the I-1 discharge that makes the rest of the program honest.
2. **Emit-site aggregation** (round 2 split this from decision 2b — they are orthogonal, and R2
   round 1 conflated them). I-3 says "no per-call-site spread branch", but `ArgsForm` unifies only
   the *decision*: the emit sites that §6.3 tables still each hand-write
   `match form { Flat => emit(op, argc, ic), Array => emit(spread_op) }` — the very per-site emit
   branching whose divergence caused this bug. Options: (a) one `emit_call(fc, shape, args_form, ic)`
   owning op selection, so no site names an opcode; (b) keep the branches that §6.3 tables. **Recommendation: (a)**
   — it is fully compatible with keeping `CallMethodSpread`, and it is what actually discharges I-3.
2b. **Opcode family shape.** Given (a), does the spread family stay four variants
   (`CallSpread`/`CallMethodSpread`/`NewSpread`/`SuperCallSpread`) or collapse to a sentinel `argc`
   on the existing opcodes? Round-2 review notes the variants' only difference is how many slots
   below the args array they read (0/1/2) — data the emit site knows statically — which is the
   lesson-#276 variant-vs-data shape, and that the sentinel is structurally safe (both IC accesses
   are bounds-checked, so an out-of-range sentinel reliably misses). Recommendation: **keep the four
   variants** (dispatch handlers genuinely differ in stack shape; a sentinel overloads a
   width-constrained operand with a mode bit) — but the burden of proof is now on the family, so
   ratify explicitly. ⚠ **The enumeration is of *opcodes*, not of *call shapes*.** §3's
   ChainEvaluation step-3 row adds a fifth shape — the optional call whose callee is a member
   Reference (`o.m?.(…)`, `o?.m?.(…)`) — which this decision's own criterion assigns to the existing
   **`CallMethodSpread`** rather than to a fifth variant: its stack shape is
   `[receiver callee args_array]`, identical to `o.m(...a)`, and "dispatch handlers genuinely differ
   in stack shape" is the whole justification for having variants at all. What the shape needs is an
   **emit-site selection** change (§6.3's table), not an opcode. Edges 33/34 guard it.
3. **Array-literal path sharing.** `expr.rs:105-119` emits the same sequence but handles elisions
   ([C22] ≠ [C19]). Recommendation: keep separate.
4. ~~Call-IC on spread calls~~ — **resolved** in §6.3 (monotonic counter; skip allocation).
5. **0ca: throw vs `CompileError` — RESOLVED (runtime throw), ratified by evidence in Slice 0a.**
   A `CompileError` fails the whole script; a thrown `TypeError` scopes the failure to the
   expression, matching how the feature would fail if half-implemented. Slice 0a shipped the
   `CompileError` form first and the post-push gate showed the cost concretely: one
   `this.#x = 1` anywhere took **every unrelated statement in the file** down with it, making it
   *worse* than the silent-no-op it replaced for `=` and `+=`. So: **runtime throw** for
   unimplemented *expressions*, `CompileError` reserved for what the compiler already rejects
   and for parse-time rejections (dec. 15). ⚠ **The illustration this decision used to carry —
   "e.g. computed accessor keys" — was falsified by PR-B and is replaced.** Computed class accessor
   keys *work* (`let k='p'; class C { get [k]() { return 7 } }; (new C).p` → `7`); the cited guard is
   unreachable dead code (§2.2). The surviving measured reject is a **BigInt literal key**:
   `class C { get 1n() { return 7 } }` → `CompileError "unsupported class member key type"`, while
   `class C { get 1() {…} }` succeeds. Note that this reject is **itself a divergence** — ECMA-262
   **§13.2.5.5** evaluates `LiteralPropertyName : NumericLiteral` as `ToString(NumericValue)`,
   yielding the String `"1"` — so it illustrates the
   *mechanism* (`CompileError` is for what the compiler already rejects), not an endorsement of the
   behaviour; `#11-vm-property-key-lowering-unification` owns making it correct.
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

   ⚠ **And the generic path must not be collapsed onto the dense one (Codex R5).** Routing
   `Reflect.apply` through `collect_array_like`'s dense fast path is observably wrong for a sparse
   Array: that path reads `elements` directly and maps `Empty` to `undefined`, so it skips own indexed
   accessors and inherited numeric getters. ECMA-262 **§28.1.1** delegates the argument list to
   **§7.3.19 `CreateListFromArrayLike`**, which performs a real `Get` per index — a getter on
   `Array.prototype[0]` must run for `Reflect.apply(f, null, new Array(1))`. **Ratified as §9
   decision 17** (owners named there) rather than left as an either/or — an unratified either/or in
   §9 is invisible to the slice author reading §5, which is the reason dec. 11 was closed.

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
9. **Slice 0ca sweep method — RESOLVED (executed, not stipulated).** The three-pass sweep is now
   documented and **run** at the head of §2.2, and §2.2 is its output. It earned its keep
   immediately: it found 3 defects (`(x)++`, `(x)+=1`, `(a[0])++`) that the probe, R1, R2 and
   round-2 review had all missed, and it corrected a round-2 claim (`delete x` *is* parser-gated).
   **0ca's acceptance criteria**: (a) re-run **all three passes** — pass 3 at **both** variant and sub-arm-body granularity — and classify every hit, so the arm list is reproducible rather than inherited; (b) §7.2 carries **one row per §2.2 defect row** as its
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
    **RESOLVED (Codex R1 re-raised it): the second option.** Moving dec. 11 to 1b is not free —
    §5's 1a row records `dispatch.rs`'s two callers as the *only* callers of `ic_call`/`ic_call_method`,
    and re-signaturing them is what lets 1a compile at all. So 1a keeps the refactor and **a direct
    unit test on `ic_call(.., None)` is a required 1a acceptance item**, not an optional one; without
    it 1a ships exactly the dead third state I-4 rejects. Ratified rather than left open, because an
    unratified either/or in §9 is invisible to the slice author reading §5.
12. **Stack-depth guard — RESOLVED (a limit is required, and it is 1b's).** Verified 2026-07-26:
    `grep -rn "MAX_STACK\|stack_limit\|call_depth\|MAX_FRAMES" crates/script/elidex-js/src/vm/` →
    **0 hits** — the crate has *no* stack or frame bound at all. `spread_iter_loop` caps the
    *array* at `DENSE_ARRAY_LEN_LIMIT` (1<<27, `vm/ops.rs:24`) but nothing caps the `vm.stack`
    push side, so once §6.3 routes >255 args through the Array path, `f(...arrOf10M)` becomes an
    unbounded user-driven push — **reintroducing a T0 in the fix for a T0**. Slice 1b must add a
    bound (and edge 29 must assert it, with an expected value). This is a **1b blocker**, not a
    carry-over.
13. **§6.2a — the reused drain's `IteratorClose` divergence — RESOLVED as two units.** ⚠ **Round-8
    sequencing hazard**: §5 lands **0bc before 1a**, and 0bc owns [C39], whose rest form
    (`[a, ...rest] = it`) needs a drain-into-array and whose spec *requires* `IteratorClose` —
    while `op_array_spread`/`spread_iter_loop` is the only in-tree drain-into-array and 1a **deletes**
    its `return()`. So either 0bc routes through it (and 1a silently falsifies 0bc's [C39] claim) or 0bc
    hand-rolls a second drain (the I-3 failure). State in §5/§6.2a that after 1a `op_array_spread` is
    **[C19]/[C22]-only**, give 0bc's rest path its own explicit `iter_close` site, and pin it with a
    §7.2 row. (a) Removing
    `op_array_spread`'s `return()` call is **1a's** — it is the reuse precondition and edge 21
    fails without it. (b) The **error-precedence inversion is a separate, crate-wide unit** (§6.2a-2:
    a cross-cutting site set — §6.2a-3 is the authoritative figure and carries its
    derivation — incl. the canonical `iter_close` helper, whose docstring states the wrong rule as its
    contract; the correct behaviour is completion-kind-dependent so the signature must change) →
    carve **`#11-vm-iteratorclose-precedence-convention`**, sequenced **before Slice 0bc**, whose
    [C39]→[C36] conformance claim otherwise inherits the inverted contract.
14. **`(o.m)(...)` `this`-loss — owner needed.** `compile_call_expr` (`expr_member.rs:92`) matches
    `ExprKind::Member` on the **raw** callee, and `ExprKind::Paren` is a real AST node
    (`ast.rs:299`), so `(o.m)()` takes the plain-call branch and loses `this` — a live T1
    silent-wrong sharing the non-normalised-Paren root, inside the very function 1b rewrites.
    Coupling: §5 puts Paren normalisation in **0bb's** scope, so if 0bb normalises
    globally this is fixed as an **unrecorded side effect** of a slice scoped to assign/update
    targets; if 0bb scopes narrowly, 1b ships the bug intact. Recommendation: one `peel_paren`
    chokepoint owned by **0bb**, with `(o.m)(...)` an explicit 0bb deliverable + §2.2 row, and
    edge 31 as 1b's regression guard. ⚠ **Made operative (Codex R1)**: as written, edge 31 was
    allocated to 1b alone, so 0bb could land the receiver-binding fix it owns with **no test asserting
    `this === o`**, and the regression would surface only in a later slice that rewrites the same call
    compiler. `(o.m)()` asserting `this === o` is therefore **part of 0bb's own acceptance suite**; 1b
    keeps edge 31 as its regression guard.
15. **`expr.rs:186-189` prefix `Spread` — layer mismatch.** §2.2 originally assigned it to the I-1 discharge, whose rule (§9
    dec. 5) is *runtime throw*; but `var y = ...x` is an **early SyntaxError** per spec, so a runtime
    throw lets preceding side effects run and lets `if (false) { var y = ...x }` execute the whole
    script. Root is parser-layer: `parser/expr.rs:256-263`'s `Ellipsis` arm is **ungated** (its own
    comment says "in arguments/array context" but no context check exists). Recommendation: reassign
    to **0ba** (which owns `parser/expr.rs`) as a parse-time rejection, not 0ca.
16. **§7.2's conformance-row outcome type — RESOLVED (crash-aware outcome), owner Slice 0cb.** The
    table's outcome type expresses `Panics` / `Throws` alongside a value, so `f(a×256)` — a process
    abort owned by 1b — has a writable row before 1b lands. The alternative (sequence the arity fix
    ahead of the table) is excluded by this document's own ordering rather than by taste: the fix is
    **1b**'s, §5 gives 1b `Deps: 1a` and 1a `Deps: 0cb`, so hoisting it inverts two declared
    dependencies and §15's ship order. Was an unowned either/or in §7.2; ratified here so 0cb's author
    reads a decision, not a fork.
17. **Dense vs generic argument unpacking — RESOLVED (keep them separate), owners 1a and Slice 10a.**
    **1a** shares `collect_array_like`'s **dense** fast path (dec. 8), and may, because the arrays it
    unpacks are compiler-emitted and dense by construction. **Slice 10a must NOT reuse that fast path
    for `Reflect.apply`**: ECMA-262 **§28.1.1** `Reflect.apply` delegates the argument list to
    **§7.3.19** `CreateListFromArrayLike`, which performs a real `Get` per index, so a getter on
    `Array.prototype[0]` must run for `Reflect.apply(f, null, new Array(1))` while the dense path
    reads `elements` directly and maps `Empty` to `undefined`. Making the generic path do property
    lookup is therefore **Slice 10a's deliverable**, settled at its mandatory plan-review; the
    array-like-vs-dense distinction stays a **caller-side precondition** (dec. 8), not a second
    helper. §-numbers verified via `webref heading ecma262 28.1.1` /
    `webref aoid ecma262 CreateListFromArrayLike`. Was an unowned either/or inside dec. 6.

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
disposition table (five items — **owners recorded below, this section no longer carries them**), per-file 1000-line verdicts for
`vm/object_kind.rs` (1700) / `vm/gc/collect.rs` (2074) / `vm/gc/trace.rs` (1255) /
`vm/interpreter.rs` (1366) / `vm/value.rs` (1187), Slice M's promotion to its own umbrella, Slice 9
× the open `natives_string.rs` §21.1.3→§22.1.3 citation-drift unit, the `delete` arms
(`expr_ops.rs:145-160` — ECMA-262 §13.5.1.1 makes both an early SyntaxError in strict code),
`vm/value.rs:1049-1053`'s eval contract retag, and assorted line-cite off-by-ones
(`expr_assign.rs` :211-213 / :216-221; `#x = v` does have an emit arm at :203-207).

⚠ **Carry-overs in that sentence sat alongside the five phase4-plan items below and were
left unowned when those five were given owners. Same treatment — a round record cannot own work:**

| Carry-over | Owner now |
|---|---|
| `natives_string.rs` §21.1.3→§22.1.3 citation-drift unit | **Slice 9b** (§5), slot `#11-vm-builtin-prototype-static-sweep` — the file is 9b's, and Slice P does not reach it. *(It was homed on `#11-vm-webidl-section-3-10-retag` as a second citation-drift class; that slot now owns the WebIDL class only.)* |
| `vm/value.rs:1049-1053`'s eval-contract retag | **Slice 3** (§5), whose module column already names `vm/value.rs` for the frame-state axis |

⚠ **The phase4-plan P4 disposition list above had silently become a to-do registry.** This section is a
*round record*; a round record cannot own work, because nothing re-reads it at re-eval time. Owner of
each of its five items, as assigned at `39bbdb1b`:

| Item | Owner now |
|---|---|
| TypedArray/DataView | §8 row `#11-vm-typed-array-family-layering-and-gate` |
| RegExp named-groups / lookbehind | Slice 8 (§5), slot `#11-vm-regexp-constructor-and-flags` |
| `replaceAll` non-global | **Slice 9b**, slot `#11-vm-builtin-prototype-static-sweep` |
| `Object.fromEntries` iterator protocol | **was unowned** → **Slice 9c**, same slot |
| `String.matchAll` double-homing | **was unowned** — the round-2 text itself said "double-homing between `#11-vm-regexp-constructor-and-flags` and Slice 9", i.e. an explicitly unresolved owner → **Slice 9b**, same slot |

⚠ **The measurements that stood in these cells have moved to the §8 slot row.** This section had the
only copy of them while §5's Slice-9 row and §8's slot row both pointed *here* — putting load-bearing
detail in a round record while the same preamble says nothing re-reads one. The three measurements
(`replaceAll` present-but-wrong, `fromEntries` array-only, `matchAll` absent) now live in
`#11-vm-builtin-prototype-static-sweep`'s row, which is where they are re-read at re-eval time; this
table keeps the historical record of what round 2 *decided*, which is what a round record is for.

The last two are **not** given new slots. Minting a parallel slot for ES2019/ES2020 builtins beside
`#11-vm-builtin-prototype-static-sweep` would duplicate the mechanism CLAUDE.md *One issue, one way*
forbids; Slice 9's charter is widened instead (§5, and the slot's own row in §8).

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
- **I-3 tagged-template clause recorded, not discharged** — decide the helper's *input* shape
  (fixed-prefix count or item iterator) so Slice 4 has a live consumer path. ⚠ **Owner assigned:
  Slice 1b**, as an explicit deliverable of `compile_call_arguments` / `ArgsForm`, with Slice 4's
  mandatory plan-review as the check. §3 already records the binding constraint (`ArgsForm` as
  specified cannot express `« siteObj »` ++ substitutions) and §5 gives Slice **4b** `Deps: 1b (I-3
  helper)` (this sentence said "Slice 4", whose own cell is `—`) — so the precondition was real and scheduled, but lived only in this round record, which
  owns nothing.
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
  core `vm/` and `vm/host/`, and including the `for-of` catch handler (`compiler/stmt.rs:175` when enumerated, `compiler/stmt_loop.rs:147` at `658cc302`) — the
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

**Implementation-ready now** *(round-4 record; 0a has since MERGED as `658cc302` — §16)*: **Slice 0a** (single verified root site `expr_assign.rs:170`, no open
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

**Readiness**: **Slice 0a** is ready (modulo dec. 1's formal admission) *— superseded: 0a MERGED as `658cc302`, and dec. 1's admission was overtaken by the measured 6-axis count (§18)*. *(The `vm/dispatch.rs`
prereq split is removed — §5.)* **P** needed its own SoT count fixed and must split its sites by
governing algorithm *— superseded: the "15" this sentence carried is withdrawn, and **§5's Slice-P
cell** is now the single home for both the site set and its ECMA-262/WebIDL split, with the commands
that re-derive them*. 0b is gated on P; 0c on the I-1 carve-out (applied); 1a on dec. 6's contract
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

**The cost of continuing was concrete** *(written at the convergence call; the abort has since been
fixed on `main` by `658cc302` — `grep -c 'assert!' compiler/expr_assign.rs` → 0)*: the T0 process
abort (`obj[k] += v`, `compiler/expr_assign.rs:170`) had been declared implementation-ready since
round 4 and stayed live on `main` for five further rounds while the umbrella was polished. That inverts §2.4's own
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

**Ship order** *(re-pointed at PR-B, when §5 inverted Pa and Pb and then deleted the `Pc → Pa` and
`Pd → Pb` edges; the only chain inside the P family is Pb → Pa)*:
**Slice 0a** → **Pb** → **`#11-vm-typed-array-family-layering-and-gate`** → **Pa** → 0b → 0c → 1a → 1b,
with **Pc** and **Pd** schedulable at any point — neither consumes what Pa or Pb produces (their §5 rows).
⚠ **That layering/gating decision is a prerequisite of Pa, not a parallel concern** (§8's row; §5's Pa
`Deps` cell). Pa must edit `vm/host/typed_array_static.rs:798`, an `iter_close` caller, so without it
Pa either changes pure ECMA-262 behaviour inside a directory whose own mandate forbids that, or omits
a required site and ships an incomplete convention sweep that 0b then depends on. It is a *decision*,
so it need not be its own PR — but it must be settled before Pa starts, and this line is an ordering
site (with §5's Deps column and §5's ordering rationale) that an implementer actually reads. Pb
precedes the gate because its touch set reaches no `vm/host/` file. *(The `vm/dispatch.rs` prereq split that rounds 2-9 mandated was **removed at implementation time** — §5's 1000-line check, whose figures were then re-derived at PR-B and largely withdrawn: the **match** is not relocated because CLAUDE.md exempts a flat case table; the **file's** size debt is discharged continuously by the arm-body extraction rule. Nine review rounds carried the standalone-split mandate.)*
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

**Scope beyond the compiler, which §5's module column does not carry.** `git show --stat 658cc302`
is **36 files**, and four are under `vm/host/`: `events_modern/touch.rs`, `headers/parse_init.rs`,
`structured_clone.rs`, `url_search_params.rs`. Those edits are not marshalling — they re-attribute
the governing algorithm (WebIDL §3.2.27 → **§3.2.21** / **§3.2.21.1** "Creating a sequence from an
iterable"; §3.10.21 → **§3.2.15**; §3.10.16 → **§3.2.21** in `events_modern/touch.rs` — §3.10.16 and
§3.10.21 do not exist in Web IDL at all — ⚠ **but 0a retagged only this one file, and the class is
crate-wide**: §3.10 exists and stops at §3.10.9, so every §3.10.N with N > 9 is dangling, and
`touch.rs` itself still carries §3.10.18. Owned by **`#11-vm-webidl-section-3-10-retag`** (§8), which
carries the measurement) and register a new divergence slot,
**`#11-webidl-sequence-dense-array-fast-path`**, at three in-code sites: an Array fast path that skips
§3.2.21 step 2's `GetMethod`, so an overridden `@@iterator` is ignored. Three of the four files are
named in §4 I-5 and §5's Slice-P module column as WebIDL two-layer cases *reserved for P's memo*, so
**Slice P inherits a partly-made decision** and must not re-derive it from scratch. Not done, and
still owed by P: `structured_clone.rs`'s in-code `§2.9` tag is drifted (HTML §2.7.4 / §2.7.7) and
survives at 8 occurrences — 0a's diff touches none of them.

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

**And another mis-attributed §-number, shipped by the carve commit itself.** §15.7.14
ClassDefinitionEvaluation does **not** pass `enumerable: false` — it calls `ClassElementEvaluation`
with one argument; **§15.7.13 ClassElementEvaluation** is what passes `false` to §15.4.5
MethodDefinitionEvaluation. The carve introduced this at two sites *while correcting two others*. Known members of the class in this PR — `§6.2.4.5` (RequireObjectCoercible → §7.2.1), `§12.5.3.2`
(delete → §13.5.1.2), `§12.10.4` (instanceof → §13.10.1/§13.10.2), `§12.2.6.8` (→ §7.3.25),
`§14.3.8` (→ §15.4.5 + §15.7.13), `§14.3.8`'s own replacement, plus the three recorded in §18's
"Spec citations" subsection (`§6.2.4.8` ×2 → §6.2.5.6 step 3.a; `§6.2.4.1` → §6.2.5.5 step 3.d; `§9.4.3` → §10.1.8.1
step 7). **The ordinal has been dropped rather than recomputed**: it was written by recall three
times running, and a tally of this document's own recall failures is the last place to keep one.
PR-B's Axis-4 pass re-derived every §-number here via webref and found the numbering clean. **The rule §13 states for counts
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

Two facts the rows do not show, both measured. **Every formal review body *by the reviewer* on this PR is 621 bytes —
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
