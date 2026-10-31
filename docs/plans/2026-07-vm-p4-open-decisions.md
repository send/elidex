# VM P4 umbrella — Open decisions for plan-review (§9, carved out)

**Carve-out of** `docs/plans/2026-07-vm-p4-es-language-completeness.md` — the VM P4 ES-language-completeness umbrella — whose
**§9** this file *is*. Moved **verbatim** at `39cc365a` by the umbrella's second touch-time
prereq split; the seam derivation, the ranges and the content-free proof are in `docs/plans/2026-10-vm-p4-memo-seam-split.md`
(not linked, so it stays outside the checker's population). Not a rewrite: every figure, line
anchor and revision reference below is stated about the umbrella at `39cc365a`; an unlabelled code line anchor is relative to the entry's **Baseline** `f7d9b5ce` unless marked "at HEAD" or with its own revision — a revision label on the anchor itself, on its paragraph, on its table row, or on its section counts as its own revision.

**Reading this file.** Section numbers are the umbrella's; an unqualified section number names
that section of the family, located by the entry's carved-out stubs. `<this file>` in a command
means `git show 39cc365a:docs/plans/2026-07-vm-p4-es-language-completeness.md`. "above" / "below" read in family section order.

## §9. Open decisions for plan-review

1. **Slice 0a/0b/0c admission.** All three surfaced during this re-probe / round-1 review and are
   not in the registered slot set. Recommendation: admit — 0a is a process abort, 0b a silent no-op at the baseline
   on ubiquitous syntax, 0c the I-1 discharge that makes the rest of the program honest.
2. **Emit-site aggregation** (round 2 split this from decision 2b — they are orthogonal, and R2
   round 1 conflated them). I-3 says "no per-call-site spread branch", but `ArgsForm` unifies only
   the *decision*: the emit sites that [§6.3](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md) tables still each hand-write
   `match form { Flat => emit(op, argc, ic), Array => emit(spread_op) }` — the very per-site emit
   branching whose divergence caused this bug. Options: (a) one `emit_call(fc, shape, args_form, ic)`
   owning op selection, so no site names an opcode; (b) keep the branches that [§6.3](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md) tables. **Recommendation: (a)**
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
   **emit-site selection** change ([§6.3](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md)'s table), not an opcode. Edges 33/34 guard it.
3. **Array-literal path sharing.** `expr.rs:105-119` emits the same sequence but handles elisions
   ([C22] ≠ [C19]). Recommendation: keep separate.
4. ~~Call-IC on spread calls~~ — **resolved** in [§6.3](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md) (monotonic counter; skip allocation).
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
   behaviour; the child of umbrella 2 that owns class-member key lowering owns making it correct — the owner both structured homes name for that row's two *reachable* arms (§2.2's `compiler/expr_class.rs` row's Slice column, and `#11-vm-property-key-lowering-unification`'s own ownership clause, which takes from that row only the dead `if computed` guard) ⚠ **this read `#11-vm-property-key-lowering-unification`, a surviving copy of the over-read that slot's cell already records withdrawing**.
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
   `expr_class.rs:145-152` producer is **spec-required** and is NOT folded — [§6.3](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md) / I-3 carve-out) +
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

   What holds regardless: splitting **by call shape** is forbidden (I-3), and **the 1a/1b boundary this
   decision adopted stands** — nothing below reopens where the seam falls.

   ⚠ **AMENDED: both halves are themselves UMBRELLAs (§5's rows); neither 1a nor 1b is a terminal unit.** This decision
   settled the **breadth** axis — VM infrastructure against compiler + opcodes — and settled it
   correctly; the terminality criterion applied to **each** half runs on a **different axis**, over the
   mechanisms that half contains, and returns umbrella for both. §5's 1a row carries one derivation: an
   argument-layout and inline-cache contract, an *observable* iterator-closing correction (dec. 13a),
   and GC root lifetime across four call-entry windows (dec. 10), each with an assertion the other two
   pass — edge 32, edge 21, edges 27/28 respectively. §5's 1b row carries the other: five roots that
   intersect at one argument-emit helper and share no chokepoint below it, so following this decision
   as it stood would let all five ship as one PR. So the sentence this paragraph used to end on —
   *"each of 1a/1b remains a terminal unit under an approved umbrella (per-PR slices touching one
   subsystem do not re-trigger splitting, else infinite regress)"* — is corrected in **both** halves.
   ⚠ **The parenthetical it quoted from CLAUDE.md clause (c) is NOT weakened**: a slice does not become
   an umbrella by touching one subsystem, and a reading that made every per-PR slice re-trigger the
   split was implemented in this program, falsified against that very parenthetical, and reverted. What
   moves each half is the mechanism measurement on that half — three on 1a, five on 1b — and
   neither reaches any other row.
7. **`Function`/`eval` gate.** I-6 supplies **half** of it — ⚠ *this entry read "I-6 does not supply it
   (strict `eval` is core)", which is I-6's note with the word* direct *dropped, the same undivided
   reading §5's pointer row carried*: strict **direct** `eval` is core, while the **sloppy half** of each of the three dynamic-code forms — direct `eval`, indirect `eval` and the global `Function` constructor — is I-6's to keep out of core, the strict half of each being core: I-6 draws the line on the strictness of the **evaluation**, not on the spelling and — ⚠ **on the direct path** — not on the strictness of the parsed source either, which is what this entry said for one revision: **§19.2.1.1** PerformEval **step 12** makes a strict caller's `eval("var x = 1")` a strict eval and so core's however the source reads, while **step 1**'s assert leaves indirect `eval` and `Function` with no `strictCaller`, so on those two alone the criterion reduces to source strictness. So "indirect `eval` and `Function` are sloppy by construction" is superseded there and here. What
   I-6 does not supply is the remaining question, a security/CSP policy decision — ⚠ **and that is one of two questions, not the whole of it**: the
   ECMA-262 **§19.2.1** / **§20.2.1.1** algorithms are code, they are joined to this decision only at
   **§19.2.1.2**'s host hook, and §5's `Function`/`eval` pointer row carries that split and the security
   layer's home for the policy half. Recommendation: keep sequenced last; decide the policy in its
   own memo. R1 marked the row "decision first" but gave the decision no owner.
8. **`collect_array_like` sharing.** `vm/natives_function.rs:272` already unpacks an array to call
   args with the correct `Empty` normalisation, and is the natural home for the child umbrella 10a mints for
   `Reflect.apply`. Semantics differ (spread's array is compiler-guaranteed dense; `apply` is
   generic array-like per CreateListFromArrayLike). Share the dense fast path or keep separate with
   a stated reason? **Recommendation (added round 8 — this was the only §9 entry with none, and it
   sits inside the 1a half's deliverable — specifically the child of umbrella 1a that owns the argument-layout and inline-cache contract): share — for that child only.** `collect_array_like` already has the correct
   `.or_undefined()` normalisation that child needs, and
   `call_internal` — one of dec. 10's four unrooted windows — is already reached at N≫1 through it,
   so sharing puts that on one audited path. ⚠ *This entry once said `Reflect.apply` "will home there" too; that is withdrawn — **dec. 17** resolves the generic path the other way (it must NOT reuse the dense fast path), and dec. 17 is the home of that question.* The array-like/dense distinction stays a
   *caller-side* precondition, not a second helper.
9. **Slice 0ca sweep method — RESOLVED (executed, not stipulated).** The three-pass sweep is now
   documented and **run** at the head of §2.2, and §2.2 is its output. It earned its keep
   immediately: it found 3 defects (`(x)++`, `(x)+=1`, `(a[0])++`) that the probe, R1, R2 and
   round-2 review had all missed, and it corrected a round-2 claim (`delete x` *is* parser-gated).
   **0ca's acceptance criteria**: (a) re-run **all three passes** — pass 3 at **both** variant and sub-arm-body granularity — and classify every hit, so the arm list is reproducible rather than inherited; (b) §7.2 carries **one row per §2.2 defect row** as its
   row-derivation rule — the table's completeness claim is then anchored to the sweep rather than to
   authoring instinct.
10. **Generator/async callee GC window — root it in the child of umbrella 1a that roots the four call-entry argument windows, via that umbrella's derivation, which mints it; carve withdrawn. ⚠ REMEDY CORRECTED
    (round 8): `gc_enabled` bracketing is WRONG for the async path.** Round 4 chose bracketing as
    "the only remedy-complete-by-construction option". Verified in round 8 that it cannot be applied
    at the site [§6.3](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md) named: `interpreter.rs:893` is not an allocation but
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
    (uncapped for a real Array) — the very function dec. 8 proposes to share and the child umbrella 10a mints for
    `Reflect.apply` will home there. dec. 10's own justification (a documented-contract violation
    does not qualify for deferral) applies verbatim. Enumerate by command
    (`grep -n "drain(base\.\.)"` / `SuspendedFrame {`) and root all four windows in one unit.

11. **IC entry point** ([§6.3](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md) IC): `Option<usize>` refactor of `ic_call`/`ic_call_method`
    (recommended, one SoT) vs. duplicating the three-way body. ⚠ **Round-8 caveat**: both callers
    (`dispatch.rs:705/716`, `ic_call` / `ic_call_method` — ⚠ *at HEAD c032379d; this read `:719/730`, the entry's Baseline `f7d9b5ce` values; `dispatch.rs` is unchanged since `658cc302`*) pass `Some`, so the `None` arm has **zero live producers until the child umbrella 1b mints for `CallSpread`** — a
    dead branch wherever the refactor lands inside the 1a half, which is the same I-4 "third state"
    argument dec. 6 used to reject VM-first,
    one layer down. Either move dec. 11 to the child umbrella **1b** mints for `CallSpread` (its producer), or pair it with a
    direct unit test on `ic_call(.., None)` so the branch is exercised where it lands.
    **RESOLVED (Codex R1 re-raised it): the second option.** Moving dec. 11 to that child is not free —
    §5's 1a row records `dispatch.rs`'s two callers as the *only* callers of `ic_call`/`ic_call_method`,
    and re-signaturing them is what lets the layout work compile at all. So the refactor stays with
    the child of umbrella 1a that owns the argument-layout and inline-cache contract, and **a direct
    unit test on `ic_call(.., None)` is a required acceptance item of that child**, not an optional one;
    without it that child ships exactly the dead third state I-4 rejects. ⚠ **The owner is the child and
    not `1a`, which is an umbrella from this revision and names nobody as an owner (§5)** — placing this
    item is a mandatory output of its derivation. Ratified rather than left open, because an
    unratified either/or in §9 is invisible to the slice author reading §5.
12. **Stack-depth guard — RESOLVED (a limit is required, and it belongs to the child umbrella **1b**'s derivation mints for the argument-emit helper).** Verified 2026-07-26:
    `grep -rn "MAX_STACK\|stack_limit\|call_depth\|MAX_FRAMES" crates/script/elidex-js/src/vm/` →
    **0 hits** — the crate has *no* stack or frame bound at all. `spread_iter_loop` caps the
    *array* at `DENSE_ARRAY_LEN_LIMIT` (1<<27, `vm/ops.rs:24`) but nothing caps the `vm.stack`
    push side, so once [§6.3](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md) routes >255 args through the Array path, `f(...arrOf10M)` becomes an
    unbounded user-driven push — **reintroducing a T0 in the fix for a T0**. The child umbrella 1b's derivation mints for the
    argument-emit helper must add a bound (and edge 29 must assert it, with an expected value), and
    placing both on that child is a mandatory output of the derivation — ⚠ *this read "Slice 1b must
    add a bound … a **1b blocker**", and an umbrella carries no obligation and blocks nothing.*
13. **[§6.2a](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md) — the reused drain's `IteratorClose` divergence — RESOLVED as two units.** ⚠ **The "Round-8
    sequencing hazard" is withdrawn with the premise it rested on**: the child umbrella 0bc mints for the binding-pattern rest path owns [C39], whose rest form
    (`[a, ...rest] = it`) needs a drain-into-array and whose spec **requires no `IteratorClose` on
    that path** — **§13.15.5.5** IteratorDestructuringAssignmentEvaluation, production
    `AssignmentRestElement`, step 4, repeats *while* `iteratorRecord.[[Done]] is false` and performs
    no close — so the child of umbrella 1a that removes `op_array_spread`'s `return()` **deletes** a
    call that child's rest path must not make either, and there is no tension between them to sequence
    around. ⚠ **This clause read "`op_array_spread`/`spread_iter_loop` is the only in-tree
    drain-into-array"; that is false and the instruction built on it is withdrawn.**
    `compiler/stmt_destructure.rs:70` emits `Op::IteratorRest` for an array rest element and
    `vm/dispatch_iter.rs:295-332` implements it as a drain-into-array. So that child **repairs and consumes
    `IteratorRest`** rather than hand-rolling a second drain or adding an explicit `iter_close` to the
    rest path — **§13.15.5.5** `AssignmentRestElement` repeats only while `[[Done]] is false` and
    performs no close, so a `return()` after normal rest exhaustion is a user-visible divergence and
    the no-`return()` regression in [§6.2a](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md) is what pins it. Whatever `IteratorRest` shares with
    `ArraySpread` is factored out rather than duplicated (*One issue, one way*). ⚠ *An either/or stood here — "0bc routes through it … or 0bc hand-rolls a second
    drain" — and is withdrawn: it survived the correction one sentence above it and re-permitted
    exactly the second drain that correction forbids.* There is no second branch. That child repairs and
    consumes `IteratorRest`, and the 1a child's `return()` removal does not falsify that child's [C39]
    claim: [C39]'s close is the `[[Done]] is false` conditional around early termination and abrupt
    pattern evaluation (**§13.15.5.2**), which the exhausted-rest path never reaches. ⚠ *This clause opened "§5 lands 0bc before 1a"; the
    ordering half is deleted rather than re-pointed — no `Deps` cell carries that edge and ordering has
    one structured home (§5, `#11-plan-memo-spec-field-single-home-check`) — while the hazard it names
    is real and stated over the artifact instead.* State in §5/[§6.2a](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md) that once that child lands, `op_array_spread` is
    **[C19]/[C22]-only**, have that child **repair and consume `Op::IteratorRest`** — the rest drain that
    already exists (`compiler/stmt_destructure.rs:70`, `vm/dispatch_iter.rs:295-332`) — **with no
    `iter_close` on the exhausted-rest path**, and pin both with a §7.2 row. ⚠ *This clause read
    "give 0bc's rest path its own explicit `iter_close` site"; it is withdrawn twice over — a second
    drain beside `IteratorRest`, and a `return()` after normal exhaustion that **§13.15.5.5** forbids.* (a) Removing
    `op_array_spread`'s `return()` call belongs to **the child of umbrella 1a that removes `op_array_spread`'s `return()`**, via that umbrella's derivation, which mints it — it is the reuse precondition and edge 21
    fails without it. (b) The **error-precedence inversion is a separate, crate-wide unit** ([§6.2a-2](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md):
    a cross-cutting site set — [§6.2a-3](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md) is the authoritative figure and carries its
    derivation — incl. the canonical `iter_close` helper, whose docstring states the wrong rule as its
    contract; the correct behaviour is completion-kind-dependent so the signature must change) →
    carve **`#11-vm-iteratorclose-precedence-convention`**, sequenced **before the children umbrella 0bc mints**, whose
    [C39]→[C36] conformance claim otherwise inherits the inverted contract.
14. **`(o.m)(...)` `this`-loss — owner needed.** `compile_call_expr` (`expr_member.rs:92`) matches
    `ExprKind::Member` on the **raw** callee, and `ExprKind::Paren` is a real AST node
    (`ast.rs:299`), so `(o.m)()` takes the plain-call branch and loses `this` — a live T1
    silent-wrong sharing the non-normalised-Paren root, inside the very function 1b rewrites.
    Coupling: §5 puts Paren normalisation in **0bb's** scope, so if 0bb normalises
    globally this is fixed as an **unrecorded side effect** of a slice scoped to assign/update
    targets; if 0bb scopes narrowly, the children umbrella 1b mints ship the bug intact. Recommendation: one `peel_paren`
    chokepoint owned by **0bb**, with `(o.m)(...)` an explicit 0bb deliverable + §2.2 row, and
    edge 31 as the regression guard of the child umbrella 1b mints for the call-emit shape. ⚠ **Made operative (Codex R1)**: as written, edge 31 was
    allocated to 1b alone, so 0bb could land the receiver-binding fix it owns with **no test asserting
    `this === o`**, and the regression would surface only in a later slice that rewrites the same call
    compiler. `(o.m)()` asserting `this === o` is therefore **part of 0bb's own acceptance suite**; that child
    keeps edge 31 as its regression guard.
15. **`expr.rs:186-189` prefix `Spread` — layer mismatch.** §2.2 originally assigned it to the I-1 discharge, whose rule (§9
    dec. 5) is *runtime throw*; but `var y = ...x` is an **early SyntaxError** per spec, so a runtime
    throw lets preceding side effects run and lets `if (false) { var y = ...x }` execute the whole
    script. Root is parser-layer: `parser/expr.rs:256-263`'s `Ellipsis` arm is **ungated** (its own
    comment says "in arguments/array context" but no context check exists). Recommendation: reassign
    to **0ba** (which owns `parser/expr.rs`) as a parse-time rejection, not 0ca.
16. **§7.2's conformance-row outcome type — RESOLVED (crash-aware outcome), owner Slice 0cb.** The
    table's outcome type expresses `Panics` / `Throws` alongside a value, so `f(a×256)` — a process
    abort owned by the child umbrella 1b mints for the argument-emit helper — has a writable row before that child lands. The alternative (sequence the arity fix
    ahead of the table) is excluded by this document's own ordering rather than by taste: the fix is
    the responsibility of the children umbrella **1b** mints, and §5 puts the edge on the child umbrella 1b mints, consuming the child of umbrella 1a that produces
    `lay_out_call_args`, so hoisting it inverts that edge. ⚠ **This sentence also cited "1a `Deps: 0cb`" until PR-B; that edge was deleted
    at `b3eb07c1` under §5's rule** — the children umbrella 1a mints consume no artifact 0cb delivers, and the reason offered for
    it ("every later slice inherits a baseline") is one **no** Deps cell in §5's table carries:
    `0cb` occurs in no cell of that column (control, same splitter, same run: `0ca` occurs in one and
    `1a` in one). The decision is unaffected, and ⚠ **the "0cb-before-1a is a ship-order fact, not a Deps
    edge" framing is withdrawn with it** — it reinstated the deleted edge in prose. Ordering has **one**
    structured home, the `Deps` column (`#11-plan-memo-spec-field-single-home-check`, §8), and the
    sequencing paragraph in §5 defers to it by name rather than carrying a second copy, so an ordering
    assertion outside the column is neither enforceable nor readable: nothing consults it. And none is
    needed here. **The children umbrella 1a mints landing first breaks nothing**: their acceptance is stated over their own artifacts —
    the existing suite plus edges 21/27/28/32, which umbrella 1a's derivation places one per child —
    and turns true whether or not the conformance table
    exists, which is the same measurement that deleted the edge. What 0cb buys later slices is a
    baseline they are *measured against once it exists*, not a precondition they wait on. Was an unowned either/or in §7.2; ratified here so 0cb's author
    reads a decision, not a fork.
17. **Dense vs generic argument unpacking — RESOLVED (keep them separate), owners the child of umbrella 1a that owns the argument-layout and inline-cache contract and the child umbrella 10a mints for `Reflect.apply` (§28.1.1).**
    **Umbrella 1a's child** shares `collect_array_like`'s **dense** fast path (dec. 8), and may, because the arrays it
    unpacks are compiler-emitted and dense by construction. **The child umbrella 10a mints for `Reflect.apply` must NOT reuse that fast path**: ECMA-262 **§28.1.1** `Reflect.apply` delegates the argument list to
    **§7.3.19** `CreateListFromArrayLike`, which performs a real `Get` per index, so a getter on
    `Array.prototype[0]` must run for `Reflect.apply(f, null, new Array(1))` while the dense path
    reads `elements` directly and maps `Empty` to `undefined`. Making the generic path do property
    lookup is therefore **that child's deliverable**, settled at that child's own mandatory plan-review; the
    array-like-vs-dense distinction stays a **caller-side precondition** (dec. 8), not a second
    helper. §-numbers verified via `webref heading ecma262 28.1.1` /
    `webref aoid ecma262 CreateListFromArrayLike`. Was an unowned either/or inside dec. 6.

---

