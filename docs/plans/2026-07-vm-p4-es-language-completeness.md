# VM P4 — ES language completeness: discharging the stubbed-emit inventory

**Status**: umbrella plan (multi-PR program). Per-slice `/elidex-plan-review` mandatory.
**Baseline**: `f7d9b5ce` (= `origin/main` at authoring time, commit date 2026-07-26).
**Slots**: `project_open-defer-slots.md` §"VM P4 ES-language + builtin gaps"; **plus** the two
pre-existing slots this umbrella adopts ([§8](2026-07-vm-p4-slot-ledger.md)).
**Corrects**: `project_vm-p4-es-language-gaps.md` §2/§4 (see §1.1).
**Revision**: R2 — restructured after `/elidex-plan-review` round 1 (2 CRIT / 11 IMP), which
established that the defect class sits in the **compiler emit layer**, not the dispatch layer.

---

## §0.5 Spec citation table

All rows verified via `.claude/tools/webref` against `ecma262` — the original 25 on 2026-07-26,
[C43] on 2026-08-03.

⚠ **This table is not the document's citation chokepoint, and reading it as one is how two
fabricated anchors survived under a blanket header before ([§10](2026-07-vm-p4-umbrella-review-rounds.md)).** It covers the ECMA-262 sections
carrying an ID; the document also cites ECMA-262 sections inline without IDs, and cites **WebIDL**
and **WHATWG HTML** — specs this header has never
covered. Verify those at their use sites with `webref heading webidl <n>` / `webref dfn html <t>`.
⚠ **And a *planned* function has no use site yet, which is the half of that obligation nothing in
this document owned.** Measured over this table's rows rather than over the file — the rows are the
lines this table starts, and the read is pipe-free for the reason [§5](2026-07-vm-p4-slice-plan.md)'s census is:
`python3 -c "import sys;C=chr(124);R=[l for l in open(sys.argv[1]) if l.startswith(C+' [C')];print(len(R),*[sum(t in l for l in R) for t in ('Reflect','Promise','RegExp','asyncIterator','AsyncIterator','String.raw')])" <this file>`
prints `29 0 0 0 0 0 1` at this revision — 29 rows, none of them naming `Reflect`, `Promise`,
`RegExp` or either spelling of the async-iterator symbol, against the control `String.raw` → **1**,
so the read discriminates rather than returning zero for everything — while [§5](2026-07-vm-p4-slice-plan.md) assigns the
thirteen `Reflect` statics, the `Promise` combinators, the `RegExp` prototype members and the
async-iterator methods to children **no derivation has minted yet**. A terminal child can therefore
implement its assigned function and cite nothing, and the umbrella's inline evidence for the
governing clause sits somewhere else entirely. The use-site rule above is **extended, not
duplicated**: where the function does not exist yet, the use site is the child's own plan-memo, and
**the derivation that mints a child records there, per function or handler it assigns to that child,
the verified section and the `webref` invocation that returned it** — a mandatory output of the
derivation alongside the obligations, edges and acceptance conditions [§5](2026-07-vm-p4-slice-plan.md) already places on it, read
at that child's own mandatory `/elidex-plan-review`. Expanding *this* table instead would put the
contract in a header whose first sentence says it is not the chokepoint, and would restate at the
umbrella a rule the child is the one able to discharge. No row and no derivation restates this
rule.

| ID | Citation | Anchor | Used by |
|---|---|---|---|
| [C19] | ECMA-262 §13.3.8.1 Runtime Semantics: ArgumentListEvaluation | `#sec-runtime-semantics-argumentlistevaluation` | the child umbrella **1b**'s derivation mints for the argument-emit helper · the child of umbrella 4a that owns the tagged-call lowering |
| [C20] | ECMA-262 §13.3.6.2 EvaluateCall | `#sec-evaluatecall` | the child umbrella **1b**'s derivation mints for the argument-emit helper · the child of umbrella 4a that owns the tagged-call lowering |
| [C33] | ECMA-262 §13.3.5.1.1 EvaluateNew | `#sec-evaluatenew` | the child umbrella **1b**'s derivation mints for the argument-emit helper |
| [C34] | ECMA-262 §7.3.14 Construct | `#sec-construct` | the child umbrella **1b**'s derivation mints for the argument-emit helper |
| [C35] | ECMA-262 §7.4.10 IteratorStepValue | `#sec-iteratorstepvalue` | the child of umbrella 1a that removes `op_array_spread`'s `return()`, via that umbrella's derivation, which mints it |
| [C45] | ECMA-262 §13.15.5.5 Runtime Semantics: IteratorDestructuringAssignmentEvaluation | `#sec-runtime-semantics-iteratordestructuringassignmentevaluation` | the child of umbrella 0bc that owns §13.15.5.5 IteratorDestructuringAssignmentEvaluation (the `AssignmentRestElement` no-close rule — step 4 repeats only while `[[Done]] is false`) |
| [C36] | ECMA-262 §7.4.11 IteratorClose | `#sec-iteratorclose` | the child of umbrella 0bc that owns the [C39] `IteratorClose` conformance / the child of umbrella 1a that removes `op_array_spread`'s `return()` (dec. 13a), via that umbrella's derivation, which mints it / `#11-vm-iteratorclose-precedence-convention` (dec. 13b) — **not** the argument-emit helper the child umbrella **1b**'s derivation mints (§2.5 C×F) |
| [C37] | ECMA-262 §13.3.9.1 Runtime Semantics: Evaluation (optional chain) | `#sec-optional-chaining-evaluation` | the child umbrella **1b**'s derivation mints for the argument-emit helper |
| [C38] | ECMA-262 §13.3.9.2 Runtime Semantics: ChainEvaluation | `#sec-optional-chaining-chain-evaluation` | the child umbrella **1b**'s derivation mints for the argument-emit helper |
| [C11] | ECMA-262 §10.2.2 `[[Construct]] ( argList, newTarget )` | `#sec-ecmascript-function-objects-construct-argumentslist-newtarget` | the child of umbrella 2ab that owns the base-path field-initialisation timing · the child of umbrella 3 that owns both call entry points |
| [C13] | ECMA-262 §13.3.7.1 Runtime Semantics: Evaluation (`super`) | `#sec-super-keyword-runtime-semantics-evaluation` | the child of umbrella 1a that owns the argument-layout and inline-cache contract, via that umbrella's derivation, which mints it (the `op_super_call_spread` handler) + the emit the child umbrella 1b's derivation mints, the child of umbrella 3 that owns the super-property emit shape |
| [C21] | ECMA-262 §7.4.4 GetIterator ( obj, kind ) | `#sec-getiterator` | the child of umbrella 1a that removes `op_array_spread`'s `return()`, via that umbrella's derivation, which mints it, 6b |
| [C22] | ECMA-262 §13.2.4.1 Runtime Semantics: ArrayAccumulation | `#sec-runtime-semantics-arrayaccumulation` | the child of umbrella 1a that removes `op_array_spread`'s `return()`, via that umbrella's derivation, which mints it (the `SpreadElement` drain) + contrast for dec. 3 |
| [C39] | ECMA-262 §13.15.5.2 Runtime Semantics: DestructuringAssignmentEvaluation | `#sec-runtime-semantics-destructuringassignmentevaluation` | the child of umbrella 0bc that owns §13.15.5.2 DestructuringAssignmentEvaluation |
| [C23] | ECMA-262 §13.2.8.4 GetTemplateObject | `#sec-gettemplateobject` | the child of umbrella 4a that owns GetTemplateObject |
| [C40] | ECMA-262 §13.3.11.1 Runtime Semantics: Evaluation (tagged template) | `#sec-tagged-templates-runtime-semantics-evaluation` | the child of umbrella 4a that owns the tagged-call lowering |
| [C44] | ECMA-262 §22.1.2.4 `String.raw ( template, ...substitutions )` | `#sec-string.raw` | Slice 4c |
| [C24] | ECMA-262 §13.3.7.3 MakeSuperPropertyReference | `#sec-makesuperpropertyreference` | the child of umbrella 3 that owns §13.3.7.3 MakeSuperPropertyReference |
| [C25] | ECMA-262 §9.1.1.3.5 GetSuperBase | `#sec-getsuperbase` | the child of umbrella 3 that owns §9.1.1.3.5 GetSuperBase |
| [C26] | ECMA-262 §7.3.32 DefineField | `#sec-definefield` | the child of umbrella 2ab that owns §7.3.32 DefineField · Slice 2ac |
| [C27] | ECMA-262 §7.3.33 InitializeInstanceElements | `#sec-initializeinstanceelements` | the child of umbrella 2ab that owns §7.3.33 InitializeInstanceElements' intra-initialization ordering |
| [C28] | ECMA-262 §15.7.10 ClassFieldDefinitionEvaluation | `#sec-runtime-semantics-classfielddefinitionevaluation` | Slice 2aa |
| [C29] | ECMA-262 §7.3.26 PrivateElementFind | `#sec-privateelementfind` | the child of umbrella 5 that owns Private Name identity + the `GetPrivate`/`SetPrivate`/`PrivateIn` dispatch |
| [C30] | ECMA-262 §7.3.28 PrivateMethodOrAccessorAdd | `#sec-privatemethodoraccessoradd` | the child of umbrella 5 that mints the method/accessor closure |
| [C31] | ECMA-262 §7.3.30 PrivateGet | `#sec-privateget` | the child of umbrella 5 that owns Private Name identity + the `GetPrivate`/`SetPrivate`/`PrivateIn` dispatch |
| [C41] | ECMA-262 §7.3.31 PrivateSet | `#sec-privateset` | the child of umbrella 5 that owns Private Name identity + the `GetPrivate`/`SetPrivate`/`PrivateIn` dispatch |
| [C42] | ECMA-262 §7.3.27 PrivateFieldAdd | `#sec-privatefieldadd` | the child of umbrella 5 that mints the field record |
| [C43] | ECMA-262 §13.15.2 Runtime Semantics: Evaluation (assignment operators) | `#sec-assignment-operators-runtime-semantics-evaluation` | Slice 0a ([§16](2026-07-vm-p4-slice-0a-landing-record.md)) |
| [C32] | ECMA-262 §27.9.3.2 AsyncGeneratorStart | `#sec-asyncgeneratorstart` | umbrella 6a's children |

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

**[§5](2026-07-vm-p4-slice-plan.md) sequences the work that discharges that class. A row enters [§5](2026-07-vm-p4-slice-plan.md) when the derivation produced the
defect.** A conformance defect measured some other way is not thereby unowned, and is never argued
away on that ground: it is recorded where it was measured — §2.2's per-slice-sweep rule is what puts
it there — and it is given an owner of its own, a terminal row, a [§8](2026-07-vm-p4-slot-ledger.md) slot, or the child an umbrella's derivation mints for it, carrying the required
triple. That is already the disposition of the children the derivation of umbrella **M** mints, those the derivation of umbrella **R** mints, those `#11-vm-atomics-global`'s mints and
those `#11-vm-typed-array-family-layering-and-gate`'s mints; until now it was reached once per defect instead of
stated once, which is why [§5](2026-07-vm-p4-slice-plan.md) grew a row every round while the class it was derived for did not.
`#11-vm-object-spread-source-coercion` and `#11-vm-arrow-lexical-new-target` already have such a
slot, so their [§5](2026-07-vm-p4-slice-plan.md) rows are rewritten below as pointers to it and nothing they measured is lost.

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
| `super(...)` grouped with broken | **`super(...args)` works in the directly executed constructor spelling the probe ran** — the one correct spread path, and the reference implementation for the argument-emit helper the child umbrella **1b**'s derivation mints. ⚠ **Narrowed at PR-B** — a `super` spelling reached through an arrow resolves against the arrow's own frame and throws; see §2.2's `parser/arrow.rs:104-119` row |
| "flag accessors missing" | Precise: `.flags` / `.lastIndex` / `.exec` **work**; `.global`/`.ignoreCase`/`.multiline`/`.sticky` and `@@match`/`@@replace` absent |
| `new.target` "probe mis-designed" | Working **in the direct-constructor spelling the probe ran**. ⚠ **Narrowed at PR-B** — lost through an arrow call; see §2.2's `vm/dispatch_class.rs:29-37` row |

### §1.2 ⚠ Retraction — this plan's own round-1 over-claim

Plan revision R1 asserted "Confirmed correct, no action: **all destructuring forms**". That was
generalised from a probe of destructuring **declarations** only. Round-1 review (Axis 3 CRIT)
caught it; the round-1 re-probe read destructuring **assignment** as a **silent no-op**. ⚠ **That reading, and the `Actual` column below, are stale at `658cc302`: all eight rows are now loud.** Re-measured row by row against the pinned tree, every one raises a `TypeError` — `update of this target is not yet supported` for the two `(x)++` / `(a[0])++` spellings, `assignment to this target is not yet supported` for the five plain-assignment spellings, and `this for-in/of assignment target is not yet supported` for the `for ([a,b] of …)` spelling — while the `Spec` column is unchanged and verified against Node row by row. §2.2 already annotates this transition on its own rows; this table never did:

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

Root, re-derived at `658cc302` rather than carried forward with the `Actual` column: the
`AssignTarget::Simple` arm (`compiler/expr_assign.rs:213`) still lowers only `ExprKind::Identifier`
and `ExprKind::Member`, but **every other LHS now reaches a scoped gate** —
`emit_unsupported(fc, "assignment to this target is not yet supported")` at `:266-268`, and
`AssignTarget::Pattern` `emit_unsupported(fc, "destructuring assignment is not yet supported")` at
`:271-280`, both lowering to `Op::ThrowUnsupported` (`:368`). ⚠ **The arms this paragraph described
for several revisions — `_ => { compile_expr(right)?; }` compiling the RHS with no store, and a
`Pattern` arm emitting `Op::Pop` while its comment claimed to "fail explicitly" — do not exist at the
pinned tree: Slice 0a removed them, which is the same transition the sentence above records when it
says all eight rows are now loud.** They are baseline-only evidence. **So what the child umbrella **0bc**'s derivation mints for the assignment-form binding-pattern lowering must replace is
those two `ThrowUnsupported` gates, not a silent fall-through** — a redesign written against the old
arms would re-remove work already landed. (The `Pattern` gate stays unreachable:
`git grep -n 'AssignTarget::Pattern' 658cc302 -- crates/script/elidex-js/src/parser` returns nothing,
rc=1 — all producers use `Simple`.) This is registered as **Slice 0bc** ([§5](2026-07-vm-p4-slice-plan.md)).

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
the repo or this plan (the **install site** is what is enumerated, never the bare name: `git grep -c '"<name>"' 658cc302 -- crates/script/elidex-js/src/vm/globals.rs` → **0** for each of `toReversed`, `toSpliced`, `with` and `toSorted`, against the positive controls `'"lastIndexOf"'` → **1** and `'"sort"'` → **1** on the same file, so the pattern reaches an installed name where one exists. ⚠ A bare `grep -rl <name>` decides none of this: `with` matches **1209** files under `crates/` and its three `elidex-js` hits are all the `with` *statement* keyword, while the other three names match this plan file itself). Since §7.2 derives its
permanent rows from the *enumerated* absences plus one row per later slot, **the children the Slice-9 umbrella's derivation mints
could satisfy this plan with those still missing**. Which sub-slice each name falls to follows from
the sub-slice charters in [§5](2026-07-vm-p4-slice-plan.md).
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
`Op::DestructureElem` **all have zero compiler emit sites** (re-derived at `658cc302`, where
`git diff --stat 658cc302 HEAD -- crates/` is empty, with the **comment-stripped** form — a bare
`grep -rn "Op::$op" … | wc -l` counts a mention inside a comment as an emit site and therefore
returns **17**, silently keeping `SetPrivate`. ⚠ **The loop's input is the enum, never a hand-supplied
list** — a placeholder `for op in …` reproduces the very under-enumeration this form exists to prevent,
since an opcode added after the list was written is skipped while the output still reads consistent.
It therefore feeds from **§2.3's extraction, which is the one place variant names are derived**
(CLAUDE.md *One issue, one way*):
`for op in $(git show <rev>:crates/script/elidex-js/src/bytecode/opcode.rs | awk '/^pub enum Op \{/{f=1;next} f&&/^\}/{f=0} f' | grep -oE '^    [A-Z][A-Za-z0-9]*'); do git grep -h "Op::$op" <rev> -- crates/script/elidex-js/src/compiler/ | grep -vE '^[[:space:]]*(//|\*)' | grep -cE "Op::$op([^A-Za-z0-9_]|$)"; done`
→ run whole at `658cc302` it walks **129** variants and returns 0 for **18** of them, the §2.3 set).
So a fix that targets the dispatch stub touches dead code and changes nothing.

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
(`grep -rlw 'parse_module' crates/script/elidex-js/src` → **8** files and **55** non-definition sites, every one inside a `#[cfg(test)]` module or a doc comment — `scope/tests.rs`, `parser/{expr_tests_validation,expr_tests_special,function_tests,stmt_tests,module}.rs`, `compiler/mod.rs`; negative control `grep -rlw 'parse_modulez' crates/script/elidex-js/src` → **0**), so the VM never parses as a module. But the **parser and scope
analysis already support modules** (`ProgramKind::Module`, `ScopeKind::Module`, import/export
parsing, `scope/tests.rs` exercises all of it). So this is a **latent trap for Slice M**: the moment
module parsing is enabled, `export const x = 1` and `import {x} from 'm'` compile to **nothing**,
silently, with the front end reporting success. The child of umbrella M that enables `parse_module`
must fix this arm *before* enabling it, not after. Recorded in [§5](2026-07-vm-p4-slice-plan.md) as a precondition on the child of umbrella M that enables `parse_module`.

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
`#11-vm-property-key-lowering-unification` has no derivation and mints no child — its kind is UNDETERMINED ([§8](2026-07-vm-p4-slot-ledger.md)), so the collapse has no owner yet; 0ca's sub-arm pass
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

**One mechanism discovers defects, and it is the per-slice sweep.** [§5](2026-07-vm-p4-slice-plan.md) already mandates that 0ca
re-run all three passes against **0ca's own parent HEAD**, and [§5](2026-07-vm-p4-slice-plan.md)'s `Primary module(s)` demotion
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

**Consequence for §7.2.** [§9](2026-07-vm-p4-open-decisions.md) dec. 9(b) derives the permanent conformance table as one row per §2.2
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
file. ([§18.2](2026-07-vm-p4-slice-0a-landing-record.md)'s R1 row says 1001→712, which is right at *its own* anchor — a mid-PR commit on the
`vm-p4-slice0a` branch, after earlier 0a commits had grown the file. That commit is reachable from neither HEAD nor `origin/main`, so
it is not cited as a sha here: by the rule above the two figures are themselves the record, and their durable half is checkable at
`658cc302`, where `compiler/stmt.rs` is **712** lines. Do not carry the 1001 back to this baseline.)

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
| `compiler/expr_assign.rs:210-212` | `[a,b]=…`, `({x}=…)` | RHS only, no store | silent no-op | T1 | the child of umbrella 0bc that owns the assignment-form binding-pattern storage — **0a ✅ loud** |
| `compiler/expr_member.rs:70-76` | `f(...a)` | spread operand as one arg | silent wrong arity | T1 | the child umbrella 1b's derivation mints for the argument-emit helper |
| `compiler/expr_class.rs:428` | `class A{x=1}` | **skipped entirely** | field `undefined` | T1 | **2aa** |
| `compiler/expr.rs:200-207` | `super.x`, `super[k]` | `PushUndefined` | TypeError at use | T2 | the child of umbrella 3 that owns the super-property emit shape |
| `compiler/expr.rs:237-239` | `` t`a${1}` `` | `PushUndefined` | tag never called | T1 | the child of umbrella **4a** that owns the tagged-template emit |
| `compiler/expr_member.rs:44`, `expr.rs:230` | `#x` get / `#x in o` | emits `GetPrivate`/`PrivateIn` → dispatch stub | `undefined` / `false` | T1 | the child of umbrella 5 that owns Private Name identity + the `GetPrivate`/`SetPrivate`/`PrivateIn` dispatch |
| `compiler/expr_assign.rs:202-206` | `o.#x = v` | `Op::Pop` (the `_ =>` arm); **no `SetPrivate` emit site exists at all** | write lost **and** the assignment evaluates to the *object*: `x = (o.#p = 5)` ⇒ `x === o` | T1 | the child of umbrella 5 that owns Private Name identity + the `GetPrivate`/`SetPrivate`/`PrivateIn` dispatch — **0a ✅ loud** |
| `compiler/expr.rs:200-207` | `import('x')` | `PushUndefined` | not a Promise | **T1** | *(see [§5](2026-07-vm-p4-slice-plan.md))* |
| `compiler/expr_object.rs:117-121` | `{1n: 'x'}` (**literal** key only — `{[1n]:…}` computed is correct, probe-verified). The cited arm is `PropertyKey::Literal(Literal::BigInt(_) \| Literal::RegExp { .. })` **inside** the `PropertyKind::Init` match whose head is `expr_object.rs:83` (arm head `:65`) — the same match the slot below names as the exhaustive copy the canonical lowering collapses the others into; a BigInt key is a key kind that lowering must answer | empty-string key → `{"":"x"}` | wrong key | T1 | **owner undetermined** — `#11-vm-property-key-lowering-unification`'s kind is UNDETERMINED ([§8](2026-07-vm-p4-slot-ledger.md)) and it mints no child |
| `compiler/expr_ops.rs:249` | `obj.#x++` | emits nothing, old value retained | silent no-op | T1 | the child of umbrella 5 that owns Private Name identity + the `GetPrivate`/`SetPrivate`/`PrivateIn` dispatch — **0a ✅ loud** |
| **`compiler/expr_ops.rs:261`** | **`(x)++`, `(a[0])++`** — parenthesized update target | operand evaluated only | **silent no-op** | T1 | 0bb — **0a ✅ loud** |
| **`compiler/expr_assign.rs:210-212`** | **`(x)+=1`** — parenthesized assign target (same catch-all as destructuring) | RHS only | **silent no-op** | T1 | 0bb — **0a ✅ loud** |
| `compiler/expr_ops.rs:147-149` + **`parser/`** | `delete this.#x` | `Pop; PushTrue` → `true` | wrong constant; ECMA-262 §13.5.1.1 makes it an **early SyntaxError** ⇒ **parse-time** rejection, so 0c's runtime-throw regime is wrong for it (same layer argument as [§9](2026-07-vm-p4-open-decisions.md) dec. 15). The sibling half of the *same* spec bullet (`delete <identifier>`) is **already** parser-gated, so the two halves must not land in two layers | T1 | **0ba** |
| `compiler/expr_ops.rs:133-163` (at `658cc302`) | **`delete (o.x)`**, **`delete o?.x`** — the arm tests the **raw** AST for `ExprKind::Member` (`:135`), so an `ExprKind::Paren` wrapper (`ast.rs:299`) or an `ExprKind::OptionalChain` takes the `else` at `:156-161` | `compile_expr`; `Op::Pop`; `Op::PushTrue` — the property **value** is evaluated, discarded, and `true` pushed | **silent success**: `{ let o={x:1}; delete (o.x); return 'x' in o }` → `true`. ECMA-262 **§13.5.1.2** step 4 deletes whenever Evaluation of the UnaryExpression yields a property Reference, and **§13.2.9.2** returns the ParenthesizedExpression's Reference unchanged | T1 | **0bb** |
| `compiler/expr_ops.rs:114-132` (at `658cc302`) | **`typeof (missingName)`** — the unresolvable-reference case runs only when the *immediate* argument is `ExprKind::Identifier` (`:117`), so a `Paren` wrapper defeats it | falls past `:132` to the generic `compile_expr` + `unary_op_to_opcode` (`:164-165`), whose `Op::GetGlobal` miss path raises `ReferenceError` (`vm/dispatch.rs:187-192`) | **throws** instead of returning `"undefined"`. ECMA-262 **§13.5.3.1** step 2.a | T1 | **0bb** |
| `compiler/expr_ops.rs:226` | module-binding **update** (`importedBinding++`) — *not* `delete`; falls to the `:261` catch-all, leaving the current value (the in-code "fall through to push undefined" comment is itself stale) | operand only | silent no-op | T1 | the child of umbrella M that implements module bindings — **0a ✅ loud** |
| `compiler/stmt.rs:877-878` | module-binding `for-in` target | `Pop` | silent no-op | T1 | the child of umbrella M that implements module bindings — **0a ✅ loud** |
| `compiler/stmt.rs:31-39` | `import`/`export` **declarations**, grouped into the `Empty`/`Debugger` no-op arm (found by the pass-3 structural sweep) | nothing | silent no-op — **latent**: unreachable until `parse_module` gains a production caller | T1 | the child of umbrella M that enables `parse_module` (precondition — this arm is fixed before that child enables it) |
| `compiler/stmt.rs:515-630` (at `658cc302`) | **`switch` does not enter its CaseBlock lexical scope** — `StmtKind::Block` (`:134-144`) and the `try` block (`:313-317`) both save `fc.current_scope_idx`, call `find_child_block_scope` and restore; the `StmtKind::Switch` arm does neither (`git show 658cc302:…/compiler/stmt.rs \| awk 'NR>=515 && NR<=632' \| grep -c -E -e current_scope_idx -e find_child_block_scope` → **0**, against **4** for the Block arm), while scope analysis *does* push a child `ScopeKind::Block` for the case block (`scope/visitor.rs:126-141`, `push_scope(ScopeKind::Block, …)` at `:131`) | ops emitted against the **enclosing** scope index, so a case-body declaration resolves to an outer binding | silent wrong **value**. Probe-measured at `658cc302` (`crates/` is byte-identical at `42c80199`): `let x=1; switch(0){case 0: let x=2;} x` → **`2`** where ECMA-262 gives `1`; control `let x=1; { let x=2; } x` → **`1`**; same shape inside a function → **`2`**. ECMA-262 **§14.12.4** Runtime Semantics: Evaluation steps 3-6 take `blockEnv = NewDeclarativeEnvironment(oldEnv)`, run `BlockDeclarationInstantiation(CaseBlock, blockEnv)` and set the running LexicalEnvironment to it, restoring at step 9. ⚠ **Arm-content defect** — the arm exists and emits, so a presence-only walk marks it handled | T1 | the child of umbrella **B** that owns block-scope entry |
| `compiler/stmt.rs:369-371`, `:380-382`, `:476-478`, `:501-503`, and `emit_pending_finally_bodies` (`:678-696`) (at `658cc302`) | **a `finally` body does not enter its own block scope** — `let x=1; try {} finally { let x=2 } x`. Every site that compiles the finalizer's statements compiles them against the enclosing `fc.current_scope_idx`, while scope analysis pushes a child `ScopeKind::Block` for the finalizer at `scope/visitor.rs:166`. Region probe, the same command over sibling regions of the one `Try` arm: `git show 658cc302:…/compiler/stmt.rs \| sed -n '<range>p' \| grep -c -E -e find_child_block_scope -e current_scope_idx` → **0** over `365,383` (no-catch finalizer) and **0** over `466,505` (shared + catch-rethrow finalizer), against **3** over `310,320` (the try block) | the finalizer's statements emit against the enclosing scope index, so a declaration in the finalizer resolves to an outer binding | silent wrong **value**. Probe-measured at `0c10870b` (`git diff --stat 658cc302 0c10870b -- crates/` is empty): `let x=1; try {} finally { let x=2 } x` → **`2`** where ECMA-262 gives `1`, and the `catch`+`finally` spelling `let x=1; try {} catch(e) {} finally { let x=2 } x` → **`2`** as well, so both emission branches carry it. Controls, same harness: `let x=1; { let x=2 } x` → **`1`**, `let x=1; try { let x=2 } finally {} x` → **`1`**, `let x=1; try { throw 0 } catch(e) { let x=2 } x` → **`1`** — the sibling scopes of the same statement are entered. Governing: ECMA-262 **§14.2.3** BlockDeclarationInstantiation (`webref aoid ecma262 BlockDeclarationInstantiation`), invoked by **§14.2.2** Block Runtime Semantics: Evaluation. **A call at the finalizer sites is not the fix.** `scope/visitor.rs:148` and `:166` both push with the `Try` statement's own `span`, so the try and finalizer scopes are siblings under one parent with the same kind *and* the same span, and `find_child_block_scope` (`stmt.rs:639-652`) returns the first child matching kind+span — always the try block. So the lookup key has to distinguish same-span siblings before any entry site can be added | T1 | the child of umbrella **B** that owns block-scope entry |
| `compiler/stmt.rs:31-39` + `compiler/mod.rs:82` + `compiler/expr_function.rs:167` (at `658cc302`) | **block-scoped function declarations are never instantiated** — `{ f(); function f() {} }`. Both hoist loops iterate an immediate-child list (`program.body` / `func.body`) and `StmtKind::Block` only recurses, so the declaration reaches the no-op arm. Scope analysis binds the name in the child block scope (`scope/visitor.rs:189` → `bind_and_visit_function` at `:422`, whose `state.add_binding(prog, state.current_scope(), …)` uses the innermost scope), so the slot exists and nothing ever writes it | nothing emitted at the declaration site, and no hoist reaches it | silent no-op → TypeError at call. Probe-measured at `658cc302`: `(function(){ { try { return 'called:'+f() } catch(e) { return 'ERR:'+e } function f(){ return 7 } } })()` → **`"ERR:TypeError: not a function"`**; `(function(){ { return typeof f; function f(){} } })()` → **`"undefined"`**. ⚠ **A third spelling is a *control*, not a symptom**: `(function(){ { function f(){return 7} } return typeof f })()` → **`"undefined"`** is what a strict-only core must keep — making it `"function"` is §10.2.11 FunctionDeclarationInstantiation step 32's *Normative Optional* web-legacy arm, and step 31 takes the other branch whenever `strict` is true (`webref aoid ecma262 FunctionDeclarationInstantiation`; steps from `webref body ecma262 sec-functiondeclarationinstantiation`), so CLAUDE.md's core-vs-compat split keeps it out of this slice. Control, the function body itself: `(function(){ return typeof g; function g(){} })()` → **`"function"`**. ECMA-262 **§14.2.3** BlockDeclarationInstantiation must instantiate and initialize it on block entry (AO number from `webref aoid ecma262 BlockDeclarationInstantiation`); §14.12.4 step 5 is the same AO for the CaseBlock, which is why the row above shares this row's owner | T1 | the child of umbrella **B** that owns §14.2.3 declaration instantiation |
| `compiler/expr_class.rs:430-447` | `class A{#x=1}` — `PrivateField` compiled only under `if *is_static`, no else | nothing | silent no-op | T1 | the child of umbrella 5 that mints the field record |
| `compiler/expr.rs:186-189` + **`parser/expr.rs:256-263`** | `ExprKind::Spread` in prefix position (`var y = ...x`) — an early-SyntaxError position per spec; the parser's `Ellipsis` arm is **ungated** | operand only | silent no-op | T1 | **0ba** ([§9](2026-07-vm-p4-open-decisions.md) dec. 15) |
| `compiler/expr_object.rs:26-39` (identical at head) | **object-literal accessor keys** — `compile_accessor` matches only `PropertyKey::Identifier` (`:27`) and `PropertyKey::Literal(Literal::String)` (`:32`); every other key kind falls to `_ => { fc.emit(Op::Pop); }` (`:37-39`). The key expression of a computed accessor is **never compiled at all** | `Op::Pop` — no property defined, and for a computed key **the key expression never runs** | silent no-op (class 2). Probe-measured at `39bbdb1b`: `let n=0; ({ get [++n]() { return 7 } }); n` → **`0`** · `let k='p'; let o={ get [k]() { return 7 } }; o.p` → **`undefined`** · `let n=0; ({ set [++n](v) {} }); n` → **`0`** (setter half) · `let o={ get 1() { return 7 } }; o[1]` → **`undefined`**, so this is **not limited to computed keys** — every non-Identifier / non-String-literal key is swallowed. (`{get true(){}}` **works**: `true` in key position parses as `PropertyKey::Identifier`, not `Literal::Boolean` — do not claim boolean keys are broken.) Contrast, same file: the sibling `PropertyKind::Init` arm (match at `:83`) **is** exhaustive over `PropertyKey` — `let o={1:'x'}; o[1]` → `'x'` (exhaustive is not the same as correct: its BigInt/RegExp arm is the `:117-121` row above). ECMA-262 §13.2.5.6 PropertyDefinitionEvaluation | T1 | **owner undetermined** — `#11-vm-property-key-lowering-unification`'s kind is UNDETERMINED ([§8](2026-07-vm-p4-slot-ledger.md)) and it mints no child |
| `compiler/stmt_destructure.rs:91` match, `:117-119` catch-all (`:92` / `:118-120` at head) | **object-pattern destructuring keys** — matches `PropertyKey::Identifier` (`:92` base / `:93` head), `Literal::String` (`:98`/`:99`) and `Computed` (`:104`/`:105`); every other key kind falls to `_ => { fc.emit(Op::PushUndefined); }` | `Op::PushUndefined` instead of a `GetProp`/`GetElem` | silent no-op (class 1 — **missed by pass 2**). Probe-measured at `39bbdb1b`: `const {1: a} = {1: 'x'}; a` → **`undefined`** (spec `'x'`). Identifier, string-literal and computed keys all work (`{p: a}` / `{'s': a}` / `{[k]: a}` → `'x'`), so this narrows — not deletes — §1.2's "destructuring **declarations** confirmed correct". Governing: ECMA-262 **§14.3.3.1** Runtime Semantics: PropertyBindingInitialization, whose step 1 is `Let propertyKey be ? Evaluation of PropertyName` = **§13.2.5.5**, where `LiteralPropertyName : NumericLiteral` is `Let number be the NumericValue…; Return ! ToString(number)` (the assignment-form counterpart on 0bc's path is **§13.15.5.3** PropertyDestructuringAssignmentEvaluation) | T1 | **owner undetermined** — `#11-vm-property-key-lowering-unification`'s kind is UNDETERMINED ([§8](2026-07-vm-p4-slot-ledger.md)) and it mints no child |
| `compiler/stmt_destructure.rs:134` match, `:149-150` catch-all (`:135` / `:150-151` at head) | **object-pattern REST exclusion** — the second `PropertyKey` match in the same function, deleting already-destructured keys from the rest object. Matches `Identifier` (`:135` base / `:136` head) and `Literal::String` (`:142`/`:143`); `_ => {}` under a comment reading "Computed/other keys: handled below via temp locals" — which is true of `Computed` (the `computed_key_slots` loop below) and false of every other kind | nothing emitted, so the key is never deleted from the rest object | silent wrong **value**, not a no-op: the destructured key survives into the rest binding. Probe-measured at `39bbdb1b`: `const {1: a, ...r} = {1:'x', z:'y'}` leaves `r` as **`{"1":"x","z":"y"}`**; the Identifier and computed spellings both give `{"z":"y"}`. Governing: ECMA-262 **§14.3.3.2** Runtime Semantics: RestBindingInitialization, which *receives* `excludedNames (a List of property keys)` as an argument — the set is built by **§14.3.3.1**, which returns « propertyKey » (the assignment-form counterpart on 0bc's path is **§13.15.5.4** RestDestructuringAssignmentEvaluation) | T1 | **owner undetermined** — `#11-vm-property-key-lowering-unification`'s kind is UNDETERMINED ([§8](2026-07-vm-p4-slot-ledger.md)) and it mints no child |
| `parser/object.rs:200-203` + `compiler/expr_object.rs:84-87` (at `658cc302`) | **`#x` is accepted as an object property key** — `({#x: 1})` and `const {#x: y} = o`. `parse_property_key` (`parser/object.rs:176`) has a `TokenKind::PrivateIdentifier` arm returning `PropertyKey::PrivateIdentifier`, and the arm reads only the token: nothing in it looks at which of the helper's three callers is asking (`git grep -n 'parse_property_key' 658cc302 -- crates/script/elidex-js/src` → the class-member path `parser/function.rs:518`, the object literal `parser/object.rs:51`, the binding pattern `parser/pattern.rs:123`, plus the definition) | object literal: `expr_object.rs:84` groups `PrivateIdentifier` with `Identifier` and emits `Op::DefineProperty`, so an ordinary property is defined; binding pattern: the key kind reaches the `stmt_destructure.rs` catch-all of the row two above | silent wrong **program** — the spelling has no meaning for the lowering to be wrong about. Probe-measured at `0c10870b`: `Object.keys({#x: 1})` → **`"x"`** (the `#` is dropped), `({#x: 1})['x']` → **`1`**, `({#x: 1})['#x']` → **`undefined`**; and `const o = {'#x': 7, x: 9}; const {#x: y} = o; y` → **`undefined`**. Governing grammar: **§13.2.5** Object Initializer derives a key only through `PropertyName : LiteralPropertyName \| ComputedPropertyName` with `LiteralPropertyName : IdentifierName \| StringLiteral \| NumericLiteral`, and **§14.3.3** Destructuring Binding Patterns reaches the same nonterminal through `BindingProperty : PropertyName : BindingElement` (`webref body ecma262 sec-object-initializer` / `sec-destructuring-binding-patterns`) — so neither context can derive `#x` and both are a Syntax Error. `PrivateIdentifier` is an alternative of `ClassElementName`, which is why the arm stays and gains a condition on its calling context rather than being deleted | T1 | **0ba** |
| `compiler/expr_class.rs:588-590` (`:590-592` head) + `:568`, `:621` (`:570`, `:623` head) | ⚠ **the previous row here was FALSE at head *and* at baseline** and is corrected. It claimed `class{get [k](){}}` is a `CompileError` — a "loud reject — not a defect". Measured at `39bbdb1b`: `let k='p'; class C { get [k]() { return 7 } }; (new C).p` → **`7`** · `let n=0; class C { get [++n]() { return 7 } }; n` → **`1`** (the key side effect runs) · `class C { get 1() { return 7 } }; (new C)[1]` → **`7`**. Computed class members never reach the cited guard: they are handled by a different, earlier path — `expr_class.rs:523` `if computed { … MethodKind::Get => Op::DefineComputedGetter, Set => Op::DefineComputedSetter, Method \| Constructor => Op::DefineComputedMethod … }` — which exists identically at `f7d9b5ce:523` | the `if computed { return Err(…) }` guard at `:588-590` is **unreachable dead code**: `grep -rn 'emit_class_member_name_op' crates/script/elidex-js/src/compiler/` gives both call sites passing `computed: false` (`:423` and `:575` at baseline, `:423` and `:577` at head). The two `_ =>` arms at `:568` / `:621` **are** reachable, but only for key kinds neither preceding arm covers — measured, `class C { get 1n() { return 7 } }` → `CompileError "unsupported class member key type"` while `class C { get 1() {…} }` succeeds | the guard is an **I-4 connect-or-delete**; the two `_ =>` arms are a loud **BigInt-literal-key** reject (a RegExp key in class position does not reach them — it is a *parse* error, measured) | I-4 / T2 | **owner undetermined** for the guard — `#11-vm-property-key-lowering-unification`'s kind is UNDETERMINED ([§8](2026-07-vm-p4-slot-ledger.md)) and it mints no child · the child of umbrella 2 that owns class-member key lowering (the two reachable `_ =>` arms) |
| `compiler/stmt.rs:882` | `for (obj.prop in …)` | `Pop` | silent no-op | T1 | the child of umbrella 0bc that owns the for-in/of head assignment-target store — **0a ✅ loud** |
| `compiler/expr_member.rs:92` | **`(o.m)()`** — `compile_call_expr` matches `ExprKind::Member` on the **raw** callee, so a parenthesized callee takes the plain-call branch | `Op::Call` (no receiver pushed) | **`this` is `undefined`** instead of `o` ([C20] step 1.a.i) | T1 | **0bb** (the shared `peel_paren` chokepoint — [§9](2026-07-vm-p4-open-decisions.md) dec. 14) |
| `compiler/expr_member.rs:180-187` (at `658cc302`) | **`o.m?.(…)`** — `prev_is_member` is `i > 0 && matches!(chain[i - 1], OptionalChainPart::Member { .. })`, so an optional call that is the chain's *first* part takes the `else` branch | `Op::Call` at `:186` (no receiver pushed); `o?.m(…)` takes `Op::CallMethod` at `:184` | **`this` is `undefined`** instead of `o` ([C20] step 1.a.i) | T1 | **0bb** — sibling of the `expr_member.rs:92` row above: same observable (a callee that is a member Reference loses its receiver), **two mechanisms, not one chokepoint**. `(o.m)()` is `compile_call_expr`'s `if let ExprKind::Member` test on the raw callee, defeated by the `ExprKind::Paren` wrapper (`ast.rs:299` at `658cc302` and at `6edda6f2`); this row is `compile_optional_chain_expr`'s `prev_is_member`, which reads `chain[i-1]` and never consults the chain's *base*. `peel_paren` does not reach this one — 0bb owns both because the receiver contract is one, and fixing either alone leaves the other |
| `compiler/stmt.rs:79-86` (at `658cc302`; the `is_await: _` discard is `:83`) | **`for await (x of it)`** — the `is_await: _` discard, which Slice 0a moved out of `:99-103` when it split the loop bodies into `stmt_loop.rs` | compiles identically to sync `for-of` | silent **wrong protocol** (sync iterator used for an async iterable) | T1 | **6b** |
| `compiler/expr_member.rs:60-64` | `f(a×256)` | **`assert!` → panic** | process abort | T0 | the child umbrella 1b's derivation mints for the argument-emit helper |
| `compiler/stmt.rs:424-439` — **not** an `f7d9b5ce` row: derived at `658cc302` and re-derived at HEAD `d49465d0`, where `git show <rev>:crates/script/elidex-js/src/compiler/stmt.rs \| grep -n 'if let Some(param_id) = catch.param'` gives **424** at both, and `'Op::Pop); // pop exception'` gives **439** at both | **destructured catch parameter** — `catch ({x})`, `catch ([a])` | the parameter is stored only under `if let PatternKind::Identifier(atom) = &pattern.kind` (`:426`); no other `PatternKind` has an arm, so nothing is emitted for the pattern's bound names, and the arm then runs `fc.emit(Op::Pop)` (`:439`) **unconditionally**, discarding the exception either way | silent no-op: `try { throw {x: 1} } catch ({x}) { … }` never initializes `x`. ECMA-262 **§14.15.2** Runtime Semantics: CatchClauseEvaluation **step 5** — `Let status be Completion(BindingInitialization of CatchParameter with arguments thrownValue and catchEnv)` (§-number from `webref aoid ecma262 CatchClauseEvaluation`; step text from `webref body ecma262 sec-runtime-semantics-catchclauseevaluation`) | T1 | the child of umbrella 0bc that owns the destructured catch parameter (gained here — see [§5](2026-07-vm-p4-slice-plan.md)) |
| `compiler/expr_assign.rs:215-220` | `AssignTarget::Pattern` | `Op::Pop`, claims to "fail explicitly" but does not | **dead** — parser never constructs this variant | I-4 | the child of umbrella 0bc that owns the `AssignTarget::Pattern` connect-or-delete — **0a ✅ loud** |
| `compiler/expr_class.rs:402-427` (at `658cc302`) | **`class C { static x = this }`** — the static initializer is compiled with the **enclosing** `FunctionCompiler` (`compile_expr(fc, …)` at `:411` computed-key branch, `:418` otherwise), so `ExprKind::This` inside it reads the surrounding function's receiver. Contrast the static **block** arm at `:448-471`, which builds a child compiler and invokes it with the ctor as `this` (`:454` `Op::Dup`, `:470` `Op::CallMethod`) | the initializer's value evaluated against the wrong receiver | `C.x === C` → **`false`**. ECMA-262 **§15.7.10** ClassFieldDefinitionEvaluation and **§7.3.32** DefineField call the initializer with the field receiver. ⚠ Narrows §1.2's "static fields/methods, no action" and [§8](2026-07-vm-p4-slot-ledger.md)'s treatment of that facet of `#11-step9-class-extras` as discharged — the probe `static x = 1` cannot distinguish the receivers | T1 | **2ac** |
| `vm/dispatch_objects.rs:135-186` (at `658cc302`) | **`{...'ab'}`**, **`const {...r} = 'ab'`** — `op_spread_object` copies only when source **and** destination are both `JsValue::Object` (`:137`); any other source falls out of the `if let` and the handler returns `Ok(())` | nothing copied | **empty object** instead of the enumerable index properties. ECMA-262 **§7.3.25** CopyDataProperties returns early only for `undefined`/`null` (step 1) and otherwise applies `ToObject` (step 2). ⚠ **Layer-B live defect in a connected opcode** — outside the scope stated above (the passes read `compiler/`) and outside §2.3's zero-emit partition; it is tabled here because the handler was read directly, which is the mechanism that section names | T1 | **O** |
| `vm/dispatch_class.rs:29-37` + `vm/value.rs:588-599` (at `658cc302`) | **`new.target` read inside an arrow** — `op_new_target` reads only the current frame's `CallMode` (`:32-33`), and `FunctionObject` carries `captured_this` (`value.rs:598`) with no lexical `new.target` counterpart; invoking an arrow pushes a `CallMode::Call` frame (`vm/interpreter.rs:648`, `:683`, `:732`) | `JsValue::Undefined` | `function F(){ this.ok = (() => new.target)() === F } new F().ok` → **`false`**. ECMA-262 **§9.4.5** GetNewTarget resolves the surrounding function environment, which for an arrow is the enclosing function's. ⚠ Narrows §1.1's `new.target` row | T1 | **N** |
| `compiler/stmt_destructure.rs:36-81` (at `658cc302`) | **an array binding pattern never closes its iterator** — `const [x] = it`. The `PatternKind::Array` arm emits `Op::GetIterator` (`:38`), takes one value per element, and where the pattern has no rest element ends at `fc.emit(Op::Pop); // pop iterator` (`:80`). No `Op::IteratorClose` is emitted from this file at all: `git grep -l 'Op::IteratorClose' 658cc302 -- crates/script/elidex-js/src/compiler/` returns `expr_yield_star.rs`, `stmt.rs` and `stmt_loop.rs`, and not this one | `Op::Pop` on an unexhausted iterator | `it.return()` is never called on the **normal early exit**. ECMA-262 **§8.6.2** BindingInitialization, production `BindingPattern : ArrayBindingPattern`, **step 3**: "If iteratorRecord.[[Done]] is false, return ? IteratorClose(iteratorRecord, result)" (AO number from `webref aoid ecma262 BindingInitialization`, step text from `webref body ecma262 sec-runtime-semantics-bindinginitialization`). The arm is `compile_destructure_pattern`, reached from parameter patterns (`expr_function.rs:55`), `var`/`let`/`const` declarations (`stmt.rs:121`), the `for` head (`stmt_loop.rs:251`) and the for-in/of head (`stmt_loop.rs:321`) — `git grep -n compile_destructure_pattern 658cc302 -- crates/script/elidex-js/src`. **Derived from the arm; not probe-measured** (this PR edits no `.rs` file, and the crate has no runner binary) | T1 | the child of umbrella 0bc that owns §8.6.2 step 3's `IteratorClose` across the array-binding consumers |
| `compiler/stmt.rs:284-291` + `:224-256` (at `658cc302`) | **`outer: { break outer; }` does not compile** | the `Labeled` arm records the label as `fc.loop_stack.len()` (`:287-288`) and emits no patch point for the end of an ordinary labelled statement; the `Break` arm reads that index back and rejects it when no loop is open (`:244-249`, message `"label '{label_name}' is not associated with a loop or switch"`) | a labelled statement outside any loop records depth `0` while `fc.loop_stack.len()` is `0`, so the guard `loop_idx >= fc.loop_stack.len()` holds and **valid code is rejected at compile time**; with a loop nested inside the labelled block the same index resolves to `fc.loop_stack[0]` (`:251`), so the patch attaches to the loop rather than to the block. ECMA-262 **§14.13.4** LabelledEvaluation, production `LabelledStatement : LabelIdentifier : LabelledItem` **step 4** — a break completion whose `[[Target]]` is the label is caught at the labelled statement itself, so a labelled statement is a break target whether or not it is a `BreakableStatement` (§-number from `webref aoid ecma262 LabelledEvaluation`, step text from `webref body ecma262 sec-runtime-semantics-labelledevaluation`). **Derived from the arms; not probe-measured** | T2 | the child of umbrella **L** that owns the label→patch-list representation |
| `scope/visitor.rs:79-124` + `compiler/stmt_loop.rs` (at `658cc302`) | **no per-iteration lexical environment for a loop's `let` head** — `let fs=[]; for (let i=0;i<2;i++) fs.push(()=>i)` and `for (let x of [1,2])` alike | `visit_stmt` pushes exactly one `ScopeKind::Block` for the whole `for` (`:85`) and one for the whole `for-in`/`for-of` (`:114`), and the compiler stores the head binding into that one scope's local — `compile_for`'s init at `stmt_loop.rs:245-260`, the for-in/of head at `:321` — with no environment clone between iterations. `git grep -c CreatePerIterationEnvironment 658cc302 -- crates/script/elidex-js/src` exits **1** (no match) | one binding shared by every iteration, so every closure created in the body observes the last value. ECMA-262 **§14.7.4.4** CreatePerIterationEnvironment, reached from **§14.7.4.2** ForLoopEvaluation, builds a fresh `NewDeclarativeEnvironment` per iteration and copies the previous iteration's value in (steps 1.d, 1.e.ii-iii); **§14.7.5.7** ForIn/OfBodyEvaluation **step 8.h.iii** takes `iterationEnv = NewDeclarativeEnvironment(oldEnv)` per iteration for a lexical head (both read via `webref body ecma262`). **Derived from the scope pass and the arms; not probe-measured** | T1 | **row E** (kind UNDETERMINED — no owner until its measurement is made) |
| `vm/dispatch_iter.rs:141-239` + `:243-273` (at `658cc302`) | **`for-in` enumerates a front-loaded snapshot** | `op_for_in_iterator` collects keys once into `ForInState { keys, index: 0 }` (`:231-236`) and inserts a name into `seen` only inside `if attrs.enumerable && seen.insert(sid)` (`:188` own storage, `:220` prototype chain), so a **non-enumerable own key never marks the name visited**; `op_for_in_next` (`:243`) hands back `state.keys[state.index]` (`:255`) with no re-check of existence or enumerability | an inherited enumerable property shadowed by a non-enumerable own property of the same name is still enumerated, and a key deleted before its turn is still yielded. ECMA-262 **§14.7.5.9** EnumerateObjectProperties: "The values of [[Enumerable]] attributes are not considered when determining if a property of a prototype object has already been processed", and "A property that is deleted before it is processed by the iterator's next method is ignored" — and its informative generator adds the key to `visited` **before** testing `desc.enumerable` (`webref body ecma262 sec-enumerate-object-properties`). A live defect in a **connected** handler, so §1.0's derivation did not produce it: it is recorded here under this section's per-slice-sweep rule and owned by its own [§8](2026-07-vm-p4-slot-ledger.md) slot rather than by a [§5](2026-07-vm-p4-slice-plan.md) slice. **Derived from the handlers; not probe-measured** | T1 | `#11-vm-for-in-enumeration-semantics` |
| `parser/arrow.rs:104-119` + `vm/value.rs:588-599` + `vm/dispatch_class.rs:115-119` (at `658cc302`) | **`super` reached through an arrow** — `(() => super())()` in a derived constructor, `(() => super.x)()` in a method — ⚠ **only the first spelling reaches the observable below**: measured at `658cc302`, `(() => super.x)()` and its non-arrow control `super.x` fail *identically* with `TypeError: Cannot convert undefined or null to object`, because `ExprKind::Super` lowers to `Op::PushUndefined` (`compiler/expr.rs:200-207`) and the crate emits no `Op::GetSuperProp` / `Op::SetSuperProp` anywhere under `compiler/` (`git grep -n -e GetSuperProp -e SetSuperProp 658cc302 -- crates/script/elidex-js/src/compiler` returns two comment lines and no emit), so `dispatch_super_inner` is never reached and the arrow contributes nothing. The `super()` half does discriminate: `class A {} class B extends A { constructor(){ (() => super())(); this.z=1 } }` raises exactly `SyntaxError: 'super' keyword unexpected here` while the non-arrow control answers `1` | the parser deliberately admits it: the arrow parser saves `in_method` / `in_constructor` / `in_static_block` / `in_derived_constructor` and restores them inside `with_function_context`, under the comment "Arrow functions inherit super context". Nothing carries that inheritance past the parser: `git grep -n -e 'in_method' -e 'in_derived_constructor' -e 'in_static_block' 658cc302 -- crates/script/elidex-js/src \| grep -v '/parser/'` returns only `install_body_mixin_methods` substring hits in `vm/globals.rs` and `vm/host/body_mixin.rs`, while the same pattern inside `parser/` matches across six files, so it discriminates. `FunctionObject` (`value.rs:588-599`) has `captured_this` and no home-object counterpart — `git show 658cc302:…/vm/value.rs \| sed -n '588,599p' \| grep -ci home` → **0**, against **3** for `captured` over the same range and **4** for `home` over the whole file, so the pattern discriminates — and a frame's `home_class` is `Some` only when `compiled.is_class_ctor` (`vm/interpreter.rs:802`) or hard-coded `None` on the `call_internal` entry (`:589`, whose own comment says so) | `dispatch_super_inner` (`vm/dispatch_class.rs:115`) reads the **current** frame's `home_class` and raises `VmError::syntax_error("'super' keyword unexpected here")` when it is `None` (`:117-119`); an invoked arrow's frame is not a class-ctor frame, so that is the branch taken. ECMA-262 **§13.3.7.2** GetSuperConstructor step 1 and **§13.3.7.3** MakeSuperPropertyReference step 1 both begin at **§9.4.3** GetThisEnvironment, whose step 2 walks outward until a record's `HasThisBinding()` is true — an arrow's is not — so both spellings resolve against the enclosing method or constructor (§-numbers from `webref aoid ecma262 GetSuperConstructor` / `MakeSuperPropertyReference` / `GetThisEnvironment`; steps from `webref body ecma262 sec-getsuperconstructor` / `sec-makesuperpropertyreference` / `sec-getthisenvironment`). ⚠ Narrows §1.1's `super(...args)` row, and the frame-state audit umbrella **3**'s derivation places on its children is stated over the direct-method entry points, so it does not reach this. **Derived from the arms; not probe-measured** | T2 | **S** |
| `compiler/stmt.rs:203-205` (at `658cc302`; the same pair at `:227` / `:238` in the `Break` arm, under the same comment) | **a `for-of` left by `return` or `break` from inside an enclosing `try`/`finally`** | `emit_pending_finally_bodies` first, `emit_iter_close_range` second, so the finally bodies' ops precede the `Op::IteratorClose` ops in one instruction sequence (`fn emit_iter_close_range` is `stmt.rs:657`; its `fc.emit(Op::IteratorClose)` at `:661` is one of the two statement-lowering emits [§6.2a-3](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md) partitions, and neither of that section's greps sees this defect — they find the emit, not its position relative to the finally bodies: `git grep -n 'emit(Op::IteratorClose)' 658cc302 -- crates/script/elidex-js/src/compiler` returns four hits (`expr_yield_star.rs:146`/`:157`, `stmt.rs:661`, `stmt_loop.rs:147`), none in `:203-205`, while `git grep -n -e 'emit_iter_close_range(' -e 'emit_pending_finally_bodies(' 658cc302 -- crates/script/elidex-js/src/compiler` returns both call sites, so a discriminating probe exists) | the close and the enclosing `finally` run in the **wrong order**, and because the ops are sequential a throw from the finally body transfers control before the close ops execute at all, so `.return()` is never called. ECMA-262 **§14.7.5.7** ForIn/OfBodyEvaluation performs the close **inside the loop's own evaluation** — step 8.i.vi on an abrupt lhs completion and step 8.m.v when `LoopContinues(result, labelSet)` is false, each `Return ? IteratorClose(iteratorRecord, status)` — so the completion reaches an enclosing `finally` only after the iterator is closed. The order is **nesting-dependent**: for a `try`/`finally` *inside* the loop body the finally is part of step 8.10's "Evaluation of stmt" and therefore precedes the same close (§-number from `webref aoid ecma262 ForIn/OfBodyEvaluation`, steps from `webref body ecma262 sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`). ⚠ The in-tree comment above the pair states one nesting's order as unconditional and attributes it to the spec — a code-side observation for the owning slice, not a claim this plan carries. **Derived from the arms; not probe-measured** | T1 | the child of umbrella **Pd** that owns the close-versus-enclosing-finalizer ordering |
| `compiler/expr_function.rs:31-66` + `scope/visitor.rs:442-486` (at `658cc302`; `git diff --stat 658cc302 909a77cb -- crates/` is empty, so both readings hold at this document's HEAD) | **a parameter binding is never uninitialized, and the body shares the parameters' environment** — `function f(a=b,b=1){}` and `function f(a=x){var x}` | `compile_param_prologue` lowers each default as read-the-slot / compare-to-`undefined` / conditionally store (`Op::GetLocal` `:41`, `Op::PushUndefined` `:42`, `Op::StrictEq` `:43`, `Op::JumpIfFalse` `:44`, `Op::SetLocal` `:46`), and the caller has already filled every positional slot before the prologue runs, so a default initializer reads a later parameter's slot as an ordinary value; the arm's own comment at `:52` states it — "`Var` kind selects no `InitLocal` — param bindings carry no TDZ". `visit_function` pushes exactly one `ScopeKind::Function` (`:456`) and runs `visit_params` (`:479`) and the body statements (`:481-483`) inside it, so a body `var` and a parameter default resolve against the same scope | a default initializer reads a later parameter's value instead of raising, and reads a body-local `var` instead of the outer binding of that name. ECMA-262 **§10.2.11** FunctionDeclarationInstantiation orders it otherwise: step 21.3.a creates each parameter binding with "Perform ! envRecord.CreateMutableBinding(paramName, false)" and initializes it only under step 21.3.b's `hasDuplicates` case, so the bindings are uninitialized while the defaults run; step 28 performs "IteratorBindingInitialization of formals", which is what initializes them, left to right; and step 30.2 takes "variableEnv be NewDeclarativeEnvironment(envRecord)" for the body whenever `hasParamExprs` (step 8, ContainsExpression of formals) is true, under step 30.1's note that a separate record "ensure[s] that closures created by expressions in the formal parameter list do not have visibility of declarations in the function body" (§-number from `webref aoid ecma262 FunctionDeclarationInstantiation`, steps from `webref body ecma262 sec-functiondeclarationinstantiation`). A live defect in fully emitted code, so §1.0's derivation did not produce it: recorded here under this section's per-slice-sweep rule and owned by its own [§8](2026-07-vm-p4-slot-ledger.md) slot rather than by a [§5](2026-07-vm-p4-slice-plan.md) slice — 0bc's parameter coverage is binding-pattern storage and iterator closing, a different mechanism. **Derived from the arms and the scope pass; not probe-measured** | T1 | the child of umbrella `#11-vm-parameter-environment-instantiation` that owns the parameter/body environment separation |
| `vm/dispatch_objects.rs:352-384` (at `658cc302`) | **`instanceof` never tests its right operand for callability** — `({}) instanceof {prototype:{}}` | `op_instanceof` rejects a non-Object right operand (`:353-357`), looks `@@hasInstance` up on it (`:360-361`) and, if that lookup returns anything at all, resolves and calls it (`:362-364`); otherwise it falls straight through to the prototype-chain walk (`:367-383`) with no callability test in between. `coerce::get_property` (`vm/coerce.rs:907`) returns `Some(PropertyResult::Data(v))` for a **present** property whatever its value, `resolve_property` (`vm/ops_property.rs:38-41`) passes a data slot through unchanged, and `call_value` (`vm/ops.rs:665-674`) raises `TypeError "not a function"` for a non-Object callee | a plain object carrying a `prototype` property answers `instanceof` with `false` where the AO throws, and an `@@hasInstance` present with the value `null` throws where the AO ignores it. ECMA-262 **§13.10.2** InstanceofOperator orders the two: step 2 is "Let instOfHandler be ? GetMethod(target, %Symbol.hasInstance%)" — GetMethod yields **undefined** for a `null` or `undefined` value, so step 3's "is not undefined" test routes those to the default path — step 4 is "If IsCallable(target) is false, throw a TypeError exception", and only step 5 reaches **§7.3.21** OrdinaryHasInstance, whose step 1 returns false for a non-callable and whose step 5 throws when `Get(ctor, "prototype")` is not an Object, where `:373`'s `if let` takes the `Ok(false)` path instead (§-numbers from `webref aoid ecma262 InstanceofOperator` / `OrdinaryHasInstance`, steps from `webref body ecma262 sec-instanceofoperator` / `sec-ordinaryhasinstance`). A live defect in a **connected** handler, so §1.0's derivation did not produce it: recorded here under this section's per-slice-sweep rule and owned by its own [§8](2026-07-vm-p4-slot-ledger.md) slot rather than by a [§5](2026-07-vm-p4-slice-plan.md) slice — it is neither a compiler-arm stub nor a builtin-property absence. Read from the handler, not by grep: `is_callable` returns **0** hits over the whole of `vm/dispatch_objects.rs`, which cannot distinguish "the check is missing" from "the check is spelled otherwise", while `grep -rln 'is_callable' crates/script/elidex-js/src/vm/` reaches `vm/ops.rs` and `vm/coerce.rs` among other siblings, so the name is in use in the tree. **Derived from the handler; not probe-measured** | T1 | `#11-vm-instanceof-callable-check` |
| `compiler/expr_assign.rs:80-88` (at `2039766a`; `git diff --stat 658cc302 2039766a -- crates/` is empty, so the reading holds at `658cc302` too) | **a write to a `const` binding is rejected at compile time** — `try { const x = 1; x = 2 } catch (e) { 42 }` | `compile_identifier_store`'s `VarLocation::Local` arm returns `CompileError { message: "Assignment to constant variable '…'" }` (`:81-88`) before emitting anything; its sibling check two lines below takes the other route for the same lookup, emitting `Op::CheckTdz` (`:90-91`) rather than rejecting | the whole script yields no bytecode, so every unrelated statement is lost and the `catch` is never reached. ECMA-262 puts the failure at run time: **§13.15.2** Runtime Semantics: Evaluation reaches **§6.2.5.6** PutValue, whose Declarative Environment Record path is **§9.1.1.1.5** SetMutableBinding, step 5 — "Assert: This is an attempt to change the value of an immutable binding", then 5.b "If strict is true, throw a TypeError exception" (§-numbers from `webref aoid ecma262 PutValue` / `SetMutableBinding` + `webref heading ecma262 9.1.1.1`, steps from `webref body ecma262 sec-declarative-environment-records-setmutablebinding-n-v-s`). This is the shape [§17](2026-07-vm-p4-slice-0a-landing-record.md)'s private-store CRIT had and [§9](2026-07-vm-p4-open-decisions.md) dec. 5 resolved against measured evidence — loud but **not scoped** — so the disposition is dec. 5's, not a fresh one: a runtime throw through the `Op::ThrowUnsupported` mechanism (an engine-constructed error raised through `throw_error`, catchable and local), and `CompileError` reserved for what the spec itself rejects before execution. §1.0's derivation does not produce this row — §2.2's pass 3 flags an arm that neither emits an op **nor** returns `CompileError`, and this arm returns one — so it is recorded here under this section's per-slice-sweep rule and owned by its own [§8](2026-07-vm-p4-slot-ledger.md) slot. **Derived from the arm; not probe-measured** | T2 | `#11-vm-const-assignment-runtime-throw` |
| `compiler/stmt.rs:262-282` (at `2039766a`; contrast `:203-205` in the `Return` arm and `:227` / `:238` in the `Break` arm) | **an inner `for-of` left by a labelled `continue` is never closed** — `outer: for (;;) { for (const x of it) { continue outer } }` | the `Continue` arm emits the pending finally bodies (`:263`) and the jump (`:264`) and pushes the patch onto the targeted loop's `continue_patches` (`:273`); it never calls `emit_iter_close_range`, which the `Return` arm calls over `0..loop_stack.len()` (`:205`) and the `Break` arm over `target..loop_stack.len()` (`:238`), and which is what emits `Op::IteratorClose` for the for-of frames in its range (`:657-664`, the emit at `:661`). The absence is probed, not assumed: `grep -rn 'emit_iter_close_range' crates/script/elidex-js/src --include='*.rs'` returns the definition and exactly those two call sites (plus one test comment naming it), so the pattern reaches the helper wherever it is used and reaches nothing in `:262-282`. Nor is the close recovered downstream — the jump is patched onto the **targeted** loop's `continue_patches` and lands at its `loop_start` (`compiler/stmt_loop.rs:123`), bypassing the inner `for-of`'s exit path (`:125-128`, whose `:128` comment reads "normal exhaustion: discard iterator without calling .return()") and its catch handler, the only other for-of `Op::IteratorClose` emit (`:143-147`), which runs on a throw | `it.return()` is never called on the way out. ECMA-262 **§14.7.5.7** ForIn/OfBodyEvaluation step 8.m tests `LoopContinues(result, labelSet)` and on false reaches step 8.m.v, `Return ? IteratorClose(iteratorRecord, status)`; **§14.7.1.1** LoopContinues returns true for a continue completion only when step 3's `[[Target]]` is empty or step 4's labelSet contains it, so the loop the label names continues while the loops inside it close (§-numbers from `webref aoid ecma262 'ForIn/OfBodyEvaluation'` / `LoopContinues`, steps from `webref body ecma262 sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset` / `sec-loopcontinues`). The range this needs is therefore not the `Break` arm's: `break` leaves the targeted frame and `continue` re-enters it. **Derived from the arms; not probe-measured** | T1 | the child of umbrella **Pd** that owns the labelled-`continue` exit range |
| `compiler/stmt.rs:227` and `:263` (at `b5356098`; `git diff --stat 658cc302 b5356098 -- crates/` is empty, so the reading holds at `658cc302` too) | **a `break` or `continue` whose target lies inside the enclosing `try` still runs the finalizer** — `try { while (true) { break } log.push('after') } finally { log.push('fin') }` | the `Break` arm's first statement is `emit_pending_finally_bodies(fc, prog, analysis, func_scopes)?` (`:227`) and the `Continue` arm's is the same call (`:263`); neither is guarded by any test of where the jump lands, and the helper (`:678-696`) takes no target — it walks the whole of `fc.finally_stack`, `for i in (0..saved.len()).rev()` (`:686`) — so every enclosing finalizer is emitted whatever the target. The arms do compute a target for the *other* obligation on the same path (`target_idx` at `:231-236`, the `emit_iter_close_range` lower bound at `:238`), so the information a guard needs is already in scope at the site that lacks it. Contrast the sibling `Return` arm (`:203`), where emitting all of them is correct because a return does leave every enclosing `try` | the finalizer's ops run at the jump **and** again when the try block completes, so `fin` is logged before `after` and once more after it — silent wrong **order**, plus a duplicated finalizer run. ECMA-262 **§14.15.3** Runtime Semantics: Evaluation, production `TryStatement : try Block Finally`: step 1 is "Let blockResult be Completion(Evaluation of Block)" and step 2 "Let finallyResult be Completion(Evaluation of Finally)"; the `try Block Catch Finally` production orders the same two at steps 1 and 4. So the Finalizer is evaluated only after the Block's evaluation **completes**, and a jump whose target is inside the Block does not complete it (§-number from `webref heading ecma262 14.15.3`, steps from `webref body ecma262 sec-try-statement-runtime-semantics-evaluation`). **Derived from the arms; not probe-measured** | T1 | the child of umbrella **Pd** that owns target-region-aware finalizer emission |
| `compiler/expr_assign.rs:45` + `vm/dispatch.rs:155-160` + `vm/value.rs:952-957` (at `2039766a`) | **a TDZ check is lost through a captured binding** — `let f = () => x; f(); let x = 1` | `compile_identifier_load`'s `VarLocation::Local` arm consults `needs_tdz` and emits `Op::CheckTdz` before `Op::GetLocal` (`:34-42`), while its `VarLocation::Upvalue` arm is a bare `fc.emit_u16(Op::GetUpvalue, idx)` (`:47`; `:45` is the `Op::GetLocal` emit of the Local arm above it). Both `Op::CheckTdz` emit sites in the crate are inside that Local arm (`grep -rn 'Op::CheckTdz' crates/script/elidex-js/src --include='*.rs'` → two emit sites, `compiler/expr_assign.rs:42` and `:91`, both inside that Local arm; every remaining hit is the opcode's own definition, its documentation, its dispatch arm or a test, so the pattern reaches the name's other uses and still finds no emit outside the Local arm). The runtime half cannot recover it: the `Op::GetUpvalue` arm reads `upvalue_ids[idx]` and pushes `read_upvalue` (`vm/dispatch.rs:155-160`), `read_upvalue` returns the stack slot or the closed value (`vm/ops.rs:597-602`), `UpvalueState` carries exactly `Open { frame_base, slot }` and `Closed(JsValue)` (`vm/value.rs:952-957`) with no uninitialized state, and the TDZ bits live on `CallFrame` (`is_in_tdz`, `vm/value.rs:1106`), which an upvalue read never consults | a captured binding read before its declaration yields `undefined` instead of raising. ECMA-262 **§9.1.1.1.6** GetBindingValue step 2 — "If the binding for name in envRecord is an uninitialized binding, throw a ReferenceError exception" — under a preamble that adds "regardless of the value of strict" (§-number from `webref heading ecma262 9.1.1.1`, steps from `webref body ecma262 sec-declarative-environment-records-getbindingvalue-n-s`). Cross-layer (compiler emit + upvalue representation + dispatch), so §1.0's derivation does not produce it: §2.2's sweep is over `compiler/` arms and this arm emits an op. Recorded here under the per-slice-sweep rule and owned by its own [§8](2026-07-vm-p4-slot-ledger.md) slot. **Derived from the arms and the runtime reads; not probe-measured** | T1 | `#11-vm-upvalue-tdz-check` |
| `scope/visitor.rs:458-463` + `compiler/expr.rs:210-214` + `compiler/expr_function.rs:116-133` (at `2039766a`) | **a named function expression has no binding to its own name** — `let f = function g(){ return g }; f() === f` | `visit_function` adds the name as an inner `BindingKind::Function` binding for the expression form (`:458-463`, under the `:439` comment "E13: `is_expression` controls inner name binding"), and `compile_nested_function` then initializes every `BindingKind::Var \| BindingKind::Function` local above the parameter count to `undefined` (`:120` filter; `Op::PushUndefined` / `Op::SetLocal` / `Op::Pop` at `:130-132`). Nothing stores the closure into that slot: the `ExprKind::Function` arm is `compile_nested_function` → `add_constant` → `fc.emit_u16(Op::Closure, idx)` and ends (`compiler/expr.rs:210-214`). Counting `Op::Closure` emits proves nothing — several files emit it — so the probe is what **follows** each: `grep -n -A 3 'emit_u16(Op::Closure, idx)' crates/script/elidex-js/src/compiler/expr.rs crates/script/elidex-js/src/compiler/mod.rs crates/script/elidex-js/src/compiler/expr_function.rs` returns the two *declaration* sites followed by `resolve_identifier` and a store (`compiler/mod.rs:93-96`, `compiler/expr_function.rs:180-183`) and the two *expression* sites followed by the arm's closing brace (`compiler/expr.rs:213-214`, `:219-220`), so it separates the paths rather than counting the opcode | the self-name reads `undefined`, so recursion through it fails and the identity test is false. ⚠ **Both of those pass on a *mutable* binding in the function's own scope, which is not what the AO says** — **§15.2.5** step 5 is `CreateImmutableBinding(name, false)` in a `funcEnv` of the expression's own, so a third regression is required: in strict mode, assigning to the self-name inside the body throws a TypeError (**§9.1.1.1.5** SetMutableBinding step 5.b for an immutable binding under `S`). Without it the slot retires with the binding present and its immutability absent. ECMA-262 **§15.2.5** InstantiateOrdinaryFunctionExpression, production `FunctionExpression : function BindingIdentifier ( FormalParameters ) { FunctionBody }`: step 4 "Let funcEnv be NewDeclarativeEnvironment(outerEnv)", step 5 "Perform ! funcEnv.CreateImmutableBinding(name, false)", step 8 creates the closure against `funcEnv`, and step 11 "Perform ! funcEnv.InitializeBinding(name, closure)" — an **immutable** binding in an environment of the expression's own, against a mutable one in the function's own scope here (§-number from `webref aoid ecma262 InstantiateOrdinaryFunctionExpression`, steps from `webref body ecma262 sec-runtime-semantics-instantiateordinaryfunctionexpression`). The arm emits an op, so §1.0's derivation does not produce it: recorded here under the per-slice-sweep rule and owned by its own [§8](2026-07-vm-p4-slot-ledger.md) slot. **Derived from the arms and the scope pass; not probe-measured** | T1 | `#11-vm-named-function-expression-binding` |
| `parser/primary.rs:200-203` + `parser/function.rs:281-292`, `:370-371` + `scope/visitor.rs` (at `b5356098`; `git diff --stat 658cc302 b5356098 -- crates/` is empty, so the readings hold at `658cc302` too) | **a private reference is never validated against a declared private name** — `({}).#x`, and `class C { m() { return this.#missing } }` | `parse_member_prop`'s `TokenKind::PrivateIdentifier` arm advances and returns `MemberProp::PrivateIdentifier(name)` (`primary.rs:200-203`) without consulting anything about which private names are in scope. The one private-name set the parser builds is for **declarations**, not references: `parse_class_body` allocates `private_names: [HashSet<Atom>; 3]` (`function.rs:370-371`) and fills it through `check_private_name_dup` (`:281-292`), whose `match` inserts only from `ClassMemberKind::PrivateField` and `PrivateMethod` and `return`s for every other member kind; the set is a local of that loop and is dropped at `:390`, and no reference site reads it. Scope analysis carries no declared-private-name environment either: `git grep -in 'private' b5356098 -- crates/script/elidex-js/src/scope/` returns exactly three lines — `visitor.rs:278`, where `ExprKind::PrivateIn` is grouped with the other single-argument expressions and its `right` visited, and `:546` / `:550`, where `PrivateMethod` / `PrivateField` are traversed for their function or initializer — all child traversals, none of which binds a name. Negative control: the same case-insensitive pattern over `crates/script/elidex-js/src/` reaches **48** files (`git grep -iln 'private' b5356098 -- crates/script/elidex-js/src/ \| wc -l`), so it discriminates rather than matching nothing. The reference reaches lowering — `compiler/expr_member.rs:44` emits `Op::GetPrivate` and `compiler/expr.rs:230` `Op::PrivateIn` at `b5356098` (`git grep -n -e 'Op::GetPrivate' -e 'Op::PrivateIn' b5356098 -- crates/script/elidex-js/src/compiler/`), the two emit sites the rows above already table | **no SyntaxError anywhere**. What happens next is Slice 5's business and changes with it — today the dispatch stub answers `undefined` / `false` (the `expr_member.rs:44` row above) — but the reference is never *rejected*, so a `#missing` in a branch that never executes is never diagnosed at all, where the spec requires an early error. ECMA-262 **§15.7.7** Static Semantics: AllPrivateIdentifiersValid returns false for `MemberExpression : MemberExpression . PrivateIdentifier` when `names` does not contain the StringValue of the PrivateIdentifier, and likewise for the `CallExpression . PrivateIdentifier`, both `OptionalChain` and `RelationalExpression : PrivateIdentifier in ShiftExpression` productions; its `ClassBody : ClassElementList` production is what supplies the names — "Let newNames be the list-concatenation of names and the PrivateBoundIdentifiers of ClassBody". The rules that **check** it are **§16.1.1** Static Semantics: Early Errors, `ScriptBody : StatementList` ("It is a Syntax Error if AllPrivateIdentifiersValid of StatementList with argument « » is false unless the source text containing ScriptBody is eval code that is being processed by a direct eval") and **§16.2.1.1** Static Semantics: Early Errors, `ModuleBody : ModuleItemList` (the same rule with no eval carve-out). ⚠ **Not §15.7.1** — the class-body early errors do not mention the SDO at all (`webref body ecma262 sec-class-definitions-static-semantics-early-errors \| grep -c 'AllPrivateIdentifiersValid'` → **0**); §15.7.7's own `ClassBody` production is where a class body enters the algorithm. §-numbers from `webref aoid ecma262 AllPrivateIdentifiersValid` and `webref heading ecma262 16.1.1` / `16.2.1.1`; step text from `webref body ecma262 sec-static-semantics-allprivateidentifiersvalid` / `sec-scripts-static-semantics-early-errors` / `sec-module-semantics-static-semantics-early-errors`. Cross-layer (parser + scope analysis) and outside `compiler/`, so §1.0's derivation does not produce it: recorded here under this section's per-slice-sweep rule and owned by its own [§8](2026-07-vm-p4-slot-ledger.md) slot. **Derived from the arms and the scope pass; not probe-measured** | T1 | `#11-vm-private-name-early-errors` |

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
mechanically over every `Op` variant in `bytecode/opcode.rs` — **129** at `658cc302`
(`git show <rev>:crates/script/elidex-js/src/bytecode/opcode.rs | awk '/^pub enum Op \{/{f=1;next} f&&/^\}/{f=0} f' | grep -cE '^    [A-Z][A-Za-z0-9]*( = [0-9]+)?,$'`;
the **125** that stood here was last true at `f627a3f6`, i.e. already stale at this document's own
baseline, which is why the figure is anchored to a revision and not to a date): **18** have zero
compiler emit sites, and **all 18 are actionable**. ⚠ **The `17 actionable + 1 reserved` partition
that stood here is withdrawn** — it exempted `Wide` (see the bullet below); the count is flat again,
and every live site states it that way.

- **Previously listed, confirmed dead (9)**: `CallSpread`, `NewSpread` (`dispatch.rs:888`),
  `TaggedTemplate` (`:896`), `DefineField` (`:942`), `GetSuperProp`, `SetSuperProp`, `GetSuperElem`
  (`:1036`), `CreateClass` (`:931`), `DestructureElem` (`:923`).
- **WRONGLY listed as dead — `GetModuleVar` IS emitted** at `compiler/expr_assign.rs:52`
  (`VarLocation::Module(idx) => fc.emit_u16(Op::GetModuleVar, idx)`). Removing it under Slice D would
  have broken module-binding reads. Also `ImportMeta`/`DynamicImport` are dead, but for the *reason*
  in §2.2 (the compiler folds them into `PushUndefined`), so they are connects owned by the child of umbrella M that implements module bindings, not deletes.
- **MISSING from the earlier list (5)**: `DestructureProp`, `ObjectRest`, `DefaultIfUndefined` — all
  three share the same no-op dispatch arms as the listed `DestructureElem` (`dispatch.rs:925-928`) —
  plus `Debugger` and `SwitchJump`.
- `GetPrivate`/`PrivateIn` **do** have emit sites (1 each) so are *not* dead; `SetPrivate` is — and
  it is why §2.1's derivation strips comments before counting. `SetPrivate`'s **only** occurrence
  under `compiler/` is the comment at `expr_assign.rs:330`, so the count-based form classifies it
  live and yields 17, under-deleting by one; `GetPrivate` (`expr_member.rs:44`) and `PrivateIn`
  (`expr.rs:230`) are the controls — real `fc.emit_u16` sites, which the comment-stripped form
  still keeps out of the set. **Re-derive with §2.1's command**, not with a bare `grep … | wc -l`.
- `Wide` raises a loud `VmError` at **`vm/dispatch.rs:1074`** — re-derived with
  `git grep -n 'Op::Wide' -- crates/`, which returns that line and `bytecode/opcode.rs:562` and
  nothing else; the `:1083` this bullet used to cite is wrong at `658cc302` and at HEAD alike, and
  [§6.3](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md) carried the same wrong offset. ⚠ **It is swept like the other 17, and the "reserved"
  exemption is withdrawn**: it was stated in prose with no slot id, no owner and no expiry, which
  [§8](2026-07-vm-p4-slot-ledger.md)'s Forward rule forbids, and it is the opposite of what §2.3 did for `ImportMeta` /
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
| A×B | Each call shape needs both forms ⇒ one spread opcode per shape. `CallMethodSpread` does not exist — the gap forcing a new opcode ([§9](2026-07-vm-p4-open-decisions.md) decision 2). |
| A×C | The Array form *is* where the iterator drain happens. Flat must never run an iterator; Array always must, even for `f(...[])`. |
| A×E | A site switching to the Array form must not allocate an unused IC slot. **Resolved** ([§6.3](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md)): `alloc_call_ic_slot` is a monotonic counter and the table is indexed by the bytecode operand — desync is structurally impossible; skip allocation on the Array path. |
| A×F | **Corrected R2r2**: once `lay_out_call_args` succeeds, the Array path leaves the *same* `argc` slots as Flat; the 0-or-1-slot asymmetry exists only for a failure *inside* the helper. The three new handlers must still pick one of the two in-tree error-exit disciplines ([§6.3](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md)). |
| B×C | Optional-call short-circuit: `nullish?.m(...a)` must **not** evaluate or drain the operand ([C37] step 3 returns before [C38] ChainEvaluation). Iterator side effects are observable. |
| B×D | **Corrected R2r2**: under `lay_out_call_args` the array is popped *before* `do_new` allocates the instance (`vm/ops.rs:766`), so the two are never simultaneously live-and-needed. The real B×D case is the generator/async callee window ([§6.3](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md) GC). |
| B×E | `New` has no IC dimension; only `Call`/`CallMethod` do. |
| C×D | **The drain is rooted by construction** (`op_array_spread` `peek`s, so the array stays on `vm.stack`, itself a GC root). The **unrooted window is pop → re-push**: no JS allocation may occur inside it. |
| C×F | [C20] step 3 runs ArgumentListEvaluation **before** the step 4/5 callability checks ⇒ a non-callable callee must still drain the iterator to completion before throwing. **[C19] does NOT call `IteratorClose`** — verified 2026-07-26, `body ecma262 sec-runtime-semantics-argumentlistevaluation \| grep -ci iteratorclose` → **0**; `?` propagates the abrupt completion directly, and the drain's own AO already sets `[[Done]]` on throw: it calls **§7.4.10 IteratorStepValue** [C35], whose **step 4.a** sets it on an `IteratorValue` throw, and whose step 1 delegates to §7.4.9 IteratorStep — which sets it at **step 3.a** for an `IteratorComplete` throw, its own step 1 delegating further to §7.4.6 IteratorNext for the `next()` throw. *(An earlier draft credited §7.4.9 alone, which is not the AO invoked here.)* Calling `return()` here would be an *observable divergence*. (Contrast [C39] DestructuringAssignmentEvaluation → 6 `IteratorClose` call sites, which is why [C36] falls to the child of umbrella 0bc that owns the destructuring-assignment `IteratorClose` conformance, not to Slice 1.) |
| D×E | (none — IC slots are compile-time indices, not heap refs.) |
| D×F | The unwind path must not leave the args Array reachable only from a dropped Rust local. |

≥3 intersecting axes ⇒ **edge-dense**, which is the criterion's own antecedent and the only half this
section states; what follows from it is [§5](2026-07-vm-p4-slice-plan.md)'s, at the head of that section and on the row. Applied to
the rows this section had already flagged (0b/2/3/5/6, plus 0c for
the three substitution classes × 6 compiler files × a deliberate user-visible behaviour change per
dec. 5, plus **P** for a convention sweep spanning `compiler/`, core `vm/` and `vm/host/`): each sub-slice [§5](2026-07-vm-p4-slice-plan.md)
mints carries its own coupled-invariant enumeration in its own memo. ⚠ **Which rows are terminal and
which are umbrellas is not restated here** —
[§5](2026-07-vm-p4-slice-plan.md)'s rows are that field's single home, and the list that stood in this sentence went stale twice
(first naming 5 terminal after its row became an umbrella, then agreeing with the table in a copy that
would rot on the next re-split). 0a and D are narrow enough
to skip. ⚠ **Withdrawn for both ([§18](2026-07-vm-p4-slice-0a-landing-record.md)).** 0a's is moot — it shipped, and the measured axis count was
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
listed here as explicit out-of-scope rows with a hand-off to the child of umbrella 4a that owns the tagged-call lowering (I-3 requires the child of umbrella 4a that owns the tagged-call lowering reuse this
slice's helper, and `ArgsForm` as specified cannot express `« siteObj »` ++ substitutions — that
constraint is recorded now rather than discovered in Slice 4a).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | production | `Arguments : ( )` — empty list | `compile_call_arguments` (NEW) → `Flat(0)` | ✓ | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | steps 1-3 | `ArgumentList : AssignmentExpression` | `Flat(n)` path | ✓ | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | steps 1-5 | `ArgumentList : ... AssignmentExpression` — leading spread | `Array` path → `Op::CreateArray` + `Op::ArraySpread` | ✓ | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | steps 1-4 | `ArgumentList : ArgumentList , AssignmentExpression` | `Op::ArrayPush` / `Flat` | ✓ | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | steps 1-4 | `ArgumentList : ArgumentList , ... AssignmentExpression` — trailing/multiple spread | `Op::ArraySpread` after prior pushes | ✓ | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | production | `TemplateLiteral : NoSubstitutionTemplate` | — | n/a (out of scope → the child of umbrella 4a that owns the tagged-call lowering) | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | production | `TemplateLiteral : SubstitutionTemplate` | — | n/a (out of scope → the child of umbrella 4a that owns the tagged-call lowering) | yes |
| ECMA-262 §13.3.8.1 ArgumentListEvaluation | production | `SubstitutionTemplate : TemplateHead Expression TemplateSpans` | — | n/a (out of scope → the child of umbrella 4a that owns the tagged-call lowering) | yes |
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
| ECMA-262 §7.4.11 IteratorClose | steps 1-8 | abrupt completion mid-drain | — | n/a (**not reached from [C19]** → the child of umbrella 0bc that owns the destructuring-assignment `IteratorClose` conformance) | yes |
| ECMA-262 §13.3.9.1 Evaluation (optional chain) | step 3 | nullish → return `undefined` **before** ChainEvaluation | optional-call short-circuit (edge 12) | ✓ | yes |
| ECMA-262 §13.3.9.2 ChainEvaluation | step 3 | `OptionalChain : ?. Arguments` — optional **call** `f?.(...a)` → EvaluateCall. ⚠ **Not universal (Codex R2)**: for `o.m?.(...a)` the callee is still a *property Reference*, so §13.3.6.2 EvaluateCall step 1.a.i must pass `o` as `this`. `compile_optional_chain_expr` (`expr_member.rs:146`) compiles the `Member` base **as a value** and selects `CallMethod` only when a preceding member sits inside `chain`, so this spelling has **already lost its receiver at `658cc302`** — lowering it to `CallSpread` preserves the silent-wrong result. Needs a member-reference-base call shape plus edges for `o.m?.(...a)` and `o?.m?.(...a)` | `Op::CallSpread` (**+ a receiver-preserving form**) | ✓ | yes |
| ECMA-262 §13.3.9.2 ChainEvaluation | step 6 | `OptionalChain : OptionalChain Arguments` — optional **method** call `o?.m(...a)` → EvaluateCall | `Op::CallMethodSpread` (`expr_member.rs:184`) | ✓ | yes |
| ECMA-262 §13.3.7.1 Evaluation (`super`) | `SuperCall` step 4 | ArgumentListEvaluation (spread variant) | `Op::SuperCallSpread` — already correct, folded onto helper | ✓ | yes |
| ECMA-262 §13.2.4.1 ArrayAccumulation | steps 1-4 | `SpreadElement : ... AssignmentExpression` — `GetIterator(…, sync)` + `IteratorStepValue` drain, **no `IteratorClose`** | **[the child of umbrella 1a that removes `op_array_spread`'s `return()`]** `op_array_spread` (dec. 13a removes the non-spec `return()`) | ✓ | yes |
| ECMA-262 §13.2.4.1 ArrayAccumulation | production | `Elision : ,` / `Elision : Elision ,` — array-literal **holes** | `expr.rs:105-119` — contrast only, NOT shared ([§9](2026-07-vm-p4-open-decisions.md) decision 3) | n/a (out of scope) | yes |
| ECMA-262 §13.3.6.2 EvaluateCall | step 6 | `PrepareForTailCall()` | — | n/a (no tail calls in elidex) | no |
| ECMA-262 §13.3.5.1.1 EvaluateNew | steps 1-2 | constructExpr evaluation + GetValue | `expr.rs:89` callee compile | ✓ | yes |
| ECMA-262 §13.3.5.1.1 EvaluateNew | step 3 | `argumentsNode` empty → new empty List | `ArgsForm::Flat(0)` (`new C`) | ✓ | yes |

**Breadth**: K=1 spec (ecma262), **M=32** (verified by `preflight.py` → "total entries (M): 32"). M≥30 ⇒ **⚠ SPLIT-DEFAULT** — the strongest breadth verdict.

**✅ Answered by splitting, not by override** (2026-07-26, user decision — [§9](2026-07-vm-p4-open-decisions.md) dec. 6). The work is
now the **Slice 1a** half (VM infrastructure) and the **Slice 1b** half (compiler + opcodes + handlers). This map spans
**both**, so M=32 is the *pair's* depth.

**Per-PR recompute** (R2 round 5 — the gate is a per-PR verdict, so a pair-level answer would be
the same author-altitude override dec. 6 disclaims): tagging each row by its Touch column, **1a
owns 5** ([C22] `SpreadElement`, [C21] GetIterator ×2, [C35] IteratorStepValue, [C13] SuperCall)
and **1b owns 20**, with **7** `n/a` shared. So **1b M=20 < 30 ⇒ clears SPLIT-DEFAULT on its own**, and
the 1a half is trivially clear. ⚠ **`1a` and `1b` are both umbrellas from this revision ([§5](2026-07-vm-p4-slice-plan.md)), so neither half's row count is a
per-PR verdict — `1b M=20 < 30` above is a reading of the Touch column, not a terminality finding,
the criterion having returned umbrella for that half on a different axis; 1a's 5 rows are the same
kind of reading** — and the reading is itself the mechanism
partition that umbrella's cell states: [C22], [C21]×2 and [C35] are all the `op_array_spread` drain,
while [C13] is the `op_super_call_spread` handler. Each child recomputes this gate at its own start,
against a row set that is a subset of the 5, so each clears it a fortiori; that is the only sense in
which the figure survives. The split resolves the gate on both sides, not just on the pair. The points below are retained because they explain why this map is deep rather than broad,
and why a **call-shape** split stays forbidden (I-3) — but the gate itself is satisfied structurally.

**Why the map is deep, not broad.** Three points, in order of weight:

1. **The invariant argument ([§9](2026-07-vm-p4-open-decisions.md) decision 6) is what carries this, not the ratio.** Every candidate
   seam reintroduces an umbrella invariant — VM-first ships 3 dead opcodes (I-4), compiler-first
   emits into stubs (I-1), shape-split is the strangler form that caused the bug (I-3). A breadth
   metric cannot override a structural "must change together".
2. **M here measures coverage *depth on one algorithm*, not scope breadth.** K=1: every row is
   ECMA-262, and 25 of 32 rows are the single call-argument evaluation path viewed through the five
   call shapes. **7 rows are explicit `n/a` out-of-scope hand-offs** (3 TemplateLiteral → the child of umbrella 4a that owns the tagged-call lowering,
   [C20] steps 1.b.iii and 6, [C36] → the child of umbrella 0bc that owns the destructuring-assignment `IteratorClose` conformance, [C22] contrast) — i.e. rows that document what this
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

## §4. Cross-cutting design invariants → **carved out**

Moved **verbatim** to `docs/plans/2026-07-vm-p4-design-invariants.md` ([§4](2026-07-vm-p4-design-invariants.md)); sub-headings keep their numbers there.

---

## §5. Slice plan → **carved out**

Moved **verbatim** to `docs/plans/2026-07-vm-p4-slice-plan.md` ([§5](2026-07-vm-p4-slice-plan.md)), and its 1000-line touch-time check to `docs/plans/2026-07-vm-p4-touch-time-split-record.md` ([§5](2026-07-vm-p4-touch-time-split-record.md)); sub-headings keep their numbers there.

---

## §6. Slice 1a + 1b detail — call-argument spread → **carved out**

Moved **verbatim** to `docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md` by this document's
touch-time prereq split ([§5](2026-07-vm-p4-touch-time-split-record.md)'s *"And the verdict this check has never stated…"* passage carries the
seam derivation, the measured ranges and the discharge). `§6.1`, `§6.2`, `§6.2a`, `§6.2a-2`,
`§6.2a-3`, `§6.3`, `§6.4` and `§6.5` keep their numbers there, and every `§6…` reference in this
document links to that file — so scheduling from [§5](2026-07-vm-p4-slice-plan.md), [§8](2026-07-vm-p4-slot-ledger.md) and [§9](2026-07-vm-p4-open-decisions.md) reads exactly as it did before the
carve.

⚠ **The `edge NN` namespace went with §6.4, and this sentence is its single home.** Every bare
`edge NN` / `edges NN/NN` in this document, in
[the review-rounds record](2026-07-vm-p4-umbrella-review-rounds.md) and in the detail file itself
names an entry of **§6.4's numbered edge matrix** in that file, and nothing else in this program
numbers edges. Most such references carry no pointer to §6.4, and before this sentence they resolved
against a section number this document no longer contains. The repair is one declaration and not one
annotation per site: a namespace is one of the things [§5](2026-07-vm-p4-slice-plan.md) says has exactly one structured home, and a
pointer on every reference would be a second copy of it at every reference.

Two commands, and they are the claim rather than an illustration of it. References, per file, over
the four files of this program in the order they are listed above — **29 / 21 / 2 / 0**:

    grep -o -E '\b[Ee]dges? \*{0,2}[0-9]' <file> | wc -l

⚠ *Not* `grep -c`, which counts matching **lines**: [§5](2026-07-vm-p4-slice-plan.md)'s rows are one line each and carry several
references apiece, so the line count reads 15 / 17 / 2 / 0 and disagrees with the reference count on
two of the four files. Definitions, from §6.4 — **1-35, contiguous**:

    grep -o -E '(^|· )\*{0,2}[0-9]{1,2}\. ' \
        docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md |
      grep -o '[0-9]*' | sort -n | uniq | tr '\n' ' '

A reference outside that range is dangling, and the second command is what says so rather than a
reader's memory of how far the matrix runs.

---


## §7. Verification strategy

**§7.1 Per-slice unit tests** in `crates/script/elidex-js/src/vm/tests/`, one module per slice,
covering that slice's full edge matrix.

**§7.2 Permanent ES-language conformance table** — land the probe harness as
`vm/tests/tests_es_language_surface.rs`: a declarative `(source, expected)` table asserting current
truth across the language/builtin surface. This is the artifact that would have caught every gap in
§1.1 **and** this plan's own §1.2 over-claim. Rows for not-yet-implemented slices assert their
**known divergent** value with a `KNOWN-DIVERGENCE (#11-slug)` marker, flipping to the spec value in
the fixing slice — the `vm/tests/tests_dataset.rs:283` pattern. **Owned by [§5](2026-07-vm-p4-slice-plan.md)'s row 0cb**, which is this document's single home for that assignment -- §7 states no owner of its own -- so
every later slice inherits the baseline.

⚠ **The row shape needs a crash-aware outcome (Codex R2) — ratified as [§9](2026-07-vm-p4-open-decisions.md) decision 16.**
"One row per §2.2 row" includes
`f(a×256)`, whose defect is a **process abort** — `assert!(arguments.len() <= 255, …)` in
`compiler/expr_member.rs` — and is owned by the child umbrella **1b**'s derivation mints for the argument-emit helper — ⚠ **", which lands after 0cb" is deleted here for the reason [§9](2026-07-vm-p4-open-decisions.md) dec. 16 gives in full: ordering has one structured home and `0cb` occurs in no cell of it.** No `(source,
expected)` *value* comparison can be written for an input that panics the compiler, so 0cb's
permanent suite cannot pass as specified. **Resolution: the outcome type expresses
`Panics`/`Throws` alongside a value** ([§9](2026-07-vm-p4-open-decisions.md) dec. 16, which is also the single home for why sequencing
the arity fix ahead of the table is excluded).

⚠ **The row-derivation rule needs a second source** (round 8). [§9](2026-07-vm-p4-open-decisions.md) dec. 9(b) sets "one row per §2.2
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

## §8. Slot ledger changes at landing → **carved out**

Moved **verbatim** to `docs/plans/2026-07-vm-p4-slot-ledger.md` ([§8](2026-07-vm-p4-slot-ledger.md)); sub-headings keep their numbers there.

---

## §9. Open decisions for plan-review → **carved out**

Moved **verbatim** to `docs/plans/2026-07-vm-p4-open-decisions.md` ([§9](2026-07-vm-p4-open-decisions.md)); sub-headings keep their numbers there.

---

## §§10-15. Round-2 … round-7 review dispositions + the convergence call → **carved out**

Moved **verbatim** to `docs/plans/2026-07-vm-p4-umbrella-review-rounds.md` by this document's
touch-time prereq split. `§10` (round 2), `§11` (round 3), `§13` (round 5), `§12` (round 4 +
convergence state), `§14` (rounds 6-7 + the convergence series) and `§15` (the convergence call) keep
their numbers there — including the out-of-order `§13`-before-`§12` the archive was written in — and
every reference to them in this document links to that file.

---

## §§16-18. Slice 0a — landing record and its two review gates → **carved out**

Moved **verbatim** to `docs/plans/2026-07-vm-p4-slice-0a-landing-record.md` by this document's
touch-time prereq split. `§16` (implemented and merged as `658cc302`), `§17` (the pre-push
`/elidex-review` findings) and `§18` (the post-push `/external-converge` and the design re-gate it
triggered, with `§18.1` the carve and `§18.2` the converge loop on the carved head) keep their
numbers there, and every reference to them in this document links to that file.
