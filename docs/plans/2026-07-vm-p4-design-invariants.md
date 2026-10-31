# VM P4 umbrella — Cross-cutting design invariants (§4, carved out)

**Carve-out of** `docs/plans/2026-07-vm-p4-es-language-completeness.md` — the VM P4 ES-language-completeness umbrella — whose
**§4** this file *is*. Moved **verbatim** at `39cc365a` by the umbrella's second touch-time
prereq split; the seam derivation, the ranges and the content-free proof are in `docs/plans/2026-10-vm-p4-memo-seam-split.md`
(not linked, so it stays outside the checker's population). Not a rewrite: every figure, line
anchor and revision reference below is stated about the umbrella at `39cc365a`.

**Reading this file.** Section numbers are the umbrella's; an unqualified section number names
that section of the family, located by the entry's carved-out stubs. `<this file>` in a command
means `git show 39cc365a:docs/plans/2026-07-vm-p4-es-language-completeness.md`. "above" / "below" read in family section order.

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
   `ImportDeclaration`/`ExportDeclaration` half of `stmt.rs:31-39`, the `FunctionDeclaration` half of that same arm (`stmt.rs:31-39` + `mod.rs:82` + `expr_function.rs:167` — a block-nested declaration reaches the no-op arm and neither hoist loop reaches it), `stmt.rs:424-439` (the destructured catch parameter — no `PatternKind` arm but `Identifier`, and an unconditional `Op::Pop` after it), and — both added at PR-B —
   **`expr_object.rs:26-39`** (the accessor-key `_ => { fc.emit(Op::Pop); }`) and
   **`stmt_destructure.rs:149-150`** (the rest-exclusion `_ => {}`, whose *observable* is a wrong
   **value** rather than a missing effect — the destructured key survives into the rest binding.
   Listed here anyway: the banned thing is the emit shape, and typing it by its observable is how it
   dropped out of this list in the first place);
3. **wrong constant** — a plausible-looking substitute value (`expr_object.rs:117-121` empty-string
   key; `expr_ops.rs:147` `Pop; PushTrue` for `delete this.#x`);
4. **wrong arity** — the right *kind* of value in the wrong *count* (`expr_member.rs:70-75`, the
   call-spread defect itself). Named for completeness; owned by the child umbrella 1b's derivation mints for the argument-emit helper, not 0ca, since making it
   loud would break every `f(...a)` site that a working implementation is about to fix.

The class-1 and class-2 lists above are the §2.2 table's class-1 / class-2 rows, kept in sync
deliberately — R2 round 3 caught them as a strict subset, and PR-B's four new rows reproduced
exactly that, which is how an arm silently drops out of 0ca's remit.

**Slice 0ca discharges all three program-wide up front** (§5) — with **three** carve-outs, not one:

- **the two early-SyntaxError sites**, which §2.2 assigns to **0ba** because a runtime throw
  is the wrong remedy for a parse-time rejection (§9 dec. 15): `expr.rs:186-189` prefix `Spread`
  (class 2) and `expr_ops.rs:147` `delete this.#x` (class 3);
- **the five `PropertyKey` rows §2.2 routes to that slot** (`expr_object.rs:26-39`, `expr_object.rs:117-121` (the class-3 `{1n:'x'}` empty-string key, whose arm sits inside the same `PropertyKind::Init` match the collapse absorbs the others into),
  `stmt_destructure.rs:117-119`, `stmt_destructure.rs:149-150` and the `expr_class.rs` dead
  `if computed` guard), which §2.2 routes to **a child `#11-vm-property-key-lowering-unification` cannot mint while its kind is UNDETERMINED (§8)** — and the
  `expr_class.rs` `_ =>` arms to **the child of umbrella 2 that owns class-member key lowering** — because making four divergent catch-alls individually
  loud entrenches the duplication the collapse exists to remove (CLAUDE.md *One issue, one way*);
- **class 4 (wrong arity)**, carved in item 4 above (the child umbrella 1b mints for it, not 0ca).

Its scope is derived by a documented
sweep, not by inspection — see §5 Slice 0ca and §9 decision 9. §9 decision 5 records the
throw-vs-`CompileError` choice.

**I-2 · `assert!` is not rejection** — *for the refusal-of-user-constructs class only*. A compiler
arm that rejects a **user-writable construct** must never `assert!`/`panic!`/`unreachable!`
(`expr_assign.rs:170`, `expr_member.rs:60`). **The remedy is `Op::ThrowUnsupported`, a scoped
runtime throw — NOT `CompileError`** (dec. 5, ratified by evidence in [§17](2026-07-vm-p4-slice-0a-landing-record.md)/[§18](2026-07-vm-p4-slice-0a-landing-record.md)): I-1 asks for loud
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
supported")` at `:104`. The child of umbrella M that owns this arm inherits no conversion here — only
the implementation.*

**I-3 · One argument-emit path.** All call shapes route through **one** helper that decides
flat-vs-array; the tagged-template arg list of the child of umbrella 4a that owns the tagged-call lowering routes through the same helper. **Carve-out
(spec-mandated, R2 round 5)**: the synthesized default derived constructor
(`compiler/expr_class.rs:145-152`) must NOT be folded — ECMA-262 §15.7.14 step 14.a.iv.1 NOTE requires it
not to observably call `%Array.prototype%[@@iterator]`, which the shared path would. See [§6.3](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md). The direct lesson
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
  the children of umbrella **10a** (its `Reflect` half) and the children of umbrella **10b** (its `Proxy` half) to
  resolve `Proxy`/`Reflect` × `ObjectKind::HostObject` *inside* core dispatch, i.e. to add more. Whether those are admissible marshalling or inbound violations is undecided here; the
  outbound rule got a Converse precisely because a literal reading stops at the wrong place, and
  the same is available in this direction. **The test is settled once, by the slot
  `#11-vm-inbound-host-call-membership-test` (§8), and both consumers inherit it**: that slot owns the
  membership test for what core `vm/` may call in `vm/host/`, and the audit of the four sites measured
  above under whatever test it adopts. ⚠ **Neither consumer can carry it in a `Deps` cell, both rows now being umbrellas and an umbrella named in a `Deps` cell naming nobody** — placing the edge on the children that resolve `ObjectKind::HostObject` inside core dispatch is a mandatory output of each umbrella's derivation, which both rows record.
  ⚠ *Settling it per surface — 10a's mandatory plan-review for the `Reflect` half, 10b's for the
  `Proxy` half, whichever ran first auditing the four sites and the second inheriting that test — is
  **withdrawn**. The two rows carry no edge between them, so their plan-reviews may run concurrently:
  "whichever runs first" orders nothing, and both could classify the same four host calls differently.
  A core/engine boundary is project-wide, not a per-surface choice.* ⚠ *Both rows becoming umbrellas strengthens that withdrawal rather than reopening it: each half is now several children with several mandatory plan-reviews, so a per-surface settlement would have to converge more reviewers on the same four sites, not two.* Assigning the whole
  question to 10b alone let 10a's children land, add `Reflect` × `ObjectKind::HostObject` special cases to core
  dispatch, and retire with the contract still unmade — the decision is not available at the time the
  work is done, so "coordinate with the other row" cannot supply it.
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
  convention owns** — a core-wide convention sweep *must* reach them, and doing so is
  not an outbound violation. ⚠ **The parenthetical here named "the child of umbrella P that owns the `vm/host/` `iter_close` call sites", and it was wrong three ways at once.** It keyed a child's charter on a **directory**, which the paragraph above this one rejects by name — the boundary is the `engine` feature gate plus host-binding dependency, *not* directory name. It resolved to no single child: §8's `#11-vm-typed-array-family-layering-and-gate` partitions these very sites by governing spec, putting `typed_array_static.rs` under **Pa** and `url_search_params.rs` / `headers/parse_init.rs` / `structured_clone.rs` under **Pc**, so the definite description spanned two already-minted children and named neither. And the sites this sentence is about are wider than any `vm/host/` charter — `vm/webidl_sequence.rs` is core `vm/`, which is why the sentence calls the sweep *core-wide*. The parties are named by the membership test this section already states — *which spec governs the algorithm whose completion is being closed?* — so ECMA-262 **§7.4.11** puts the `%TypedArray%.from` site on **Pa**, and WebIDL **§3.2.21.1**, which has no `IteratorClose` step, puts the sequence-creation sites on **Pc** together with their core helper. Without this clause an implementer reading the outbound rule literally
  would stop at the core sites and ship the split-contract One-issue-one-way failure the sweep
  exists to remove. ⚠ **Slice 0a already spent part of this disposition** — it touched
  `vm/host/{headers/parse_init,structured_clone,url_search_params}.rs`, re-attributed their governing
  sections (WebIDL §3.2.27 → §3.2.21 / §3.2.21.1, §3.10.21 → §3.2.15) and registered
  `#11-webidl-sequence-dense-array-fast-path` at three in-code sites. P inherits a partly-made
  decision, not a blank one — see [§16](2026-07-vm-p4-slice-0a-landing-record.md).
  ⚠ *A correction bullet used to sit above this Converse re-pairing each `typed_array_*` file with
  its own §-number. It is **dropped, not re-derived**: `ls vm/host/ | grep typed_array` gives seven
  files, the bullet paired three, and one of the three was wrong —* `typed_array_parts.rs`*'s only
  §23 citation is `:84` and is **§23.2.4.4 ValidateTypedArray**, not the §23.2.3 it was paired with.
  An annotation layered over a normative sentence is the pattern this document is removing; the
  sentence itself now says §23.2.*
  **Two-layer cases needing explicit disposition in Pa's and Pc's memos**: `vm/webidl_sequence.rs`
  (governed by WebIDL §3.2.21) and **`vm/host/structured_clone.rs`** (WHATWG HTML **§2.7.4** StructuredSerialize **fused with §2.7.6
  StructuredDeserialize**, per the entry point's own docstring at `:63-64` and row F's classification
  (§2.7.7 StructuredSerializeWithTransfer for the transfer-list path where the `iter_close` at
  `:1062` sat — `:1064` at `658cc302`. ⚠ **Codex R1 corrected the attribution of that site**: it is
  inside `ensure_empty_transfer_list`, whose own docstring calls it a WebIDL `sequence<object>`
  conversion throwing per **WebIDL §3.2.21 step 3**, and its `iter_close` runs while consuming the
  iterable — *before* HTML StructuredSerializeWithTransfer ever receives the list. So the **site
  stays in the WebIDL half of [§6.2a-3](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md)'s 7/5 partition**, and it is the *outer native* that is separately
  HTML §2.7.4 / §2.7.7. Calling the site HTML's own loop would select the wrong `.return()` and
  error-precedence remedy in the P family. The in-code docstring's "§2.9" is drifted; **§5's Slice-Pc
  row owns the retag as a named deliverable** — this clause does not) — ⚠ **the sentence that used to follow here is withdrawn (Codex R3)**: it called the `DataCloneError` an abrupt completion of *HTML's own loop*, contradicting the correction directly above and leaving [§6.2a-2](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md) enumerating only four non-ECMA sites. `ensure_empty_transfer_list` is the **WebIDL** conversion; only the outer native is HTML §§2.7.4/2.7.7, and the WebIDL count must follow from that. Superseded text: its abrupt was described as a `DataCloneError` thrown by HTML's own loop body, so a
  step-5 flip changes an HTML-defined error surface, not an ECMA-262 one; round 6 corrected an
  earlier mis-classification of this file as pure ECMA-262).

  ⚠ This clause is currently an **allowlist, not a predicate** — state the membership test
  ("which spec governs the algorithm whose completion is being closed?") and **re-derive the file
  list from it at implementation time**, per [§13](2026-07-vm-p4-umbrella-review-rounds.md)'s enumerate-by-command rule. This binds the children umbrella **M**'s derivation mints — module-script *fetching* is an ECMA-262 host hook and must go
  through a host seam, not the core VM; the renderer holds no direct network access (CLAUDE.md
  "Security by structure"). Slice M's memo must draw that line before implementation.

Two named consequences: the WeakMap/WeakSet ephemeron work of the child of umbrella **7b** that carries it — that row is not a landable slice, so placing this obligation on a specific child is a mandatory output of its own derivation — lands in `vm/gc/trace.rs`
alongside engine-gated wrapper-store rooting and the keepalive predicate
(`vm/gc/collect.rs:1149→1235→1341`), and must define identical semantics for non-`engine` builds;
and the `Proxy`/`Reflect` × `ObjectKind::HostObject` resolution (`vm/object_kind.rs:272`) inside core
dispatch splits by surface, because the row that used to carry both halves became an umbrella. The
`Reflect` half is **umbrella 10a**'s — `Reflect` is that row's surface, but the row became an
umbrella at R77 (thirteen §28.1 functions, independent mechanisms), so the obligation falls to
whichever child its derivation mints for the `HostObject` routing rather than being stated on the
row directly. The `Proxy` half falls to the children of umbrella **10b** that
reach that variant; an umbrella lands no PR, so placing the obligation on the specific child is a
mandatory output of its derivation, exactly as for 7b above. A host-side special case would violate
the outbound rule in either half.

**I-6 · No sloppy/Annex B surface.** No slice introduces sloppy-mode or Annex B behaviour
([[reference_elidex-js-core-strict-only]]). **Note**: this does *not* gate **strict direct `eval`** — per
`docs/design/ja/14-script-engines-webapi.md` §14.1.1 that is **core**. ⚠ **What §14.1.1 states is what
this PR amends it to state** — the reconciliation §8 records as done here rather than deferred: its
criterion was caller-scope injection ("only sloppy `eval`'s caller-scope injection is LegacySemantics",
the reading this note carried), and it is now the strictness of the **evaluation** across all three
dynamic-code forms — one criterion, though the ⚠ sentence below measures that it is *derived* from
`strictCaller` as well as from the source on the direct path and from the source alone on the other
two, so the citation above and the split below name one criterion and not two. ⚠ **But this note read "`Function`/`eval`" and so put the
whole language half in core, which introduces exactly the sloppy surface this invariant forbids.**
⚠ **Superseded — the ⚠ sentence below draws the line, and it is not this one.** Superseded text: two of the three forms were called sloppy *by construction*, not by a directive the caller might omit, on this evidence:
**§19.2.1.1** PerformEval step 1 is `Assert: If direct is false, then strictCaller is also false`, and
steps 12-13 set `strictEval` from `strictCaller` or from the script's own directive — so `(0, eval)("this")`
is **sloppy**, no matter how strict the caller; and **§20.2.1.1.1** CreateDynamicFunction parses the body
and hands it to `OrdinaryFunctionCreate` at step 28 with no step forcing strictness, so
`Function("return this")` is **non-strict** absent a directive in the body it was handed (§-numbers from
`webref aoid ecma262 PerformEval` / `CreateDynamicFunction`; steps from
`webref body ecma262 sec-performeval` / `sec-createdynamicfunction`). **`HostEnsureCanCompileStrings`
does not close this** — the host-defined **§19.2.1.2** hook (§9 decision 7's security/CSP surface)
allows or denies *compilation*; it makes neither body strict. ⚠ **But neither form is sloppy *as a form*, so the split cannot be drawn on the spelling.** **§19.2.1.1** **step 13** — `Else, let strictEval be ScriptIsStrict of script` — is reached exactly when `strictCaller` is false, i.e. on the indirect path, so `(0, eval)('"use strict"; …')` runs **strict**; and **§10.2.3** OrdinaryFunctionCreate **step 7** `Let strict be IsStrict(body)` with **step 10** setting `[[ThisMode]]` to strict makes `Function('"use strict"; return this')()` strict as well (steps from `webref body ecma262 sec-performeval` / `sec-ordinaryfunctioncreate`). Sending the whole of either form to LegacySemantics therefore leaves **core with no owner for valid strict dynamic code**, and draws a wider split than CLAUDE.md's own enumeration does — it names *sloppy direct-eval (caller-env scope injection)*, not the constructor and not the indirect form. So the split is on the **strictness of the evaluation**, not on the spelling — ⚠ **and that is not the same as the strictness of the parsed source, which is what this passage said for one revision and which is right for only two of the three forms.** **§19.2.1.1** **step 12** is `If strictCaller is true, let strictEval be true` and **step 13** — the `ScriptIsStrict of script` read — is its `Else`, so on the **direct** path the two are disjuncts and the source's own directive is consulted only when the caller is sloppy: strict core code evaluating `eval("var x = 1")` evaluates a body carrying no directive of its own **strictly**, and a rule reading source strictness alone would route that conforming case to LegacySemantics. The **indirect** path has no second disjunct — step 1's `Assert: If direct is false, then strictCaller is also false` forces `strictCaller` false there — and neither does `Function`, whose **§10.2.3** **step 7** `Let strict be IsStrict(body)` reads the handed body alone. So: **direct `eval` splits on the *effective* evaluation strictness (`strictCaller` or the source's own directive); indirect `eval` and `Function` split on the strictness of the parsed source, which is what effective strictness reduces to on those two paths; the strict half of each is core and the sloppy half of each LegacySemantics** (§-numbers from `webref aoid ecma262 PerformEval` / `OrdinaryFunctionCreate`, steps from `webref body ecma262 sec-performeval` / `sec-ordinaryfunctioncreate`). Core's dynamic-code surface is then a *subset* of the standard one and conformant on every evaluation it accepts, which is what makes it designable rather than arrived at by calling the half core. §5's `Function`/`eval`
pointer row carries that separation and the home of each part; the umbrella
`#11-vm-function-constructor-global` opens states which forms it mints.
⚠ **And what `CLAUDE.md`'s *Core vs compat split* clause states is what this PR amends it to state**
— the same form the §14.1.1 note above takes, and the reconciliation §8 records as done here rather
than deferred. That clause carried two sentences that the split above puts in disagreement: the
first is **motivation-scoped** (「その motivation が plugin 領域 (= 上記 legacy 機能) なら **core slot
化しない**」) and the second a **flat prohibition** (「`FrameKind` / opcode / dispatch path 等を
direct/indirect eval 区別 … で拡張する案は core scope 外」) — and assigning strict-source indirect
`eval` to core requires the dispatch path to tell direct from indirect, which is literally what the
second forbids. `CLAUDE.md` is the repo SSoT, so this document cannot override it; the amendment
**narrows the second sentence to the first's own motivation test** rather than weakening it. An
extension that exists to serve *legacy* semantics — var hoisting, sloppy-mode flags, sloppy
direct-eval's caller-env scope injection — is still rejected by it, and a reader must still reject a
sloppy-motivated `FrameKind`/opcode/dispatch proposal by reading it; what it no longer rejects is an
extension that exists because ECMA-262 **derives strictness differently on the two paths**
(**§19.2.1.1** step 12's `strictCaller` disjunct against step 1's assert). The reason that is a
narrowing and not a hole: when the clause was written the only reason to distinguish the two forms
was sloppy direct-eval's scope injection, and `#11-eval-direct-vs-indirect-flag` was dropped
pre-implementation on exactly that ground ("core = strict-only + new scope なので direct/indirect
区別不要", [[reference_elidex-js-core-strict-only]]). The split above establishes a second reason
that did not exist then, and the amended clause is the one that can tell the two apart.

**I-7 · A representation change is a collector change.** Where object-valued state *lives* is what
roots it, so a slice that moves such state from a frame onto a closure — or adds it to one — owes the
tracing edge and a regression that forces a collection between the store and the read — **and that scopes every other holder out, so the new edge is the object's *sole* root**. That second half is not a refinement of the first: a test that leaves the surrounding constructor, class or object bound in a global or a live local keeps the moved object marked *through that binding*, so it passes under a no-op arm and measures nothing. The shape that works is to build the holder inside an immediately invoked function and keep only the closure — an arrow, an extracted method — then collect, then read through it. Measured at
`7c7c2c61`: the `ObjectKind::Function` arm marks `fo.upvalue_ids` and `fo.captured_this` and nothing
else (`vm/gc/trace.rs:251-258`), which is complete today because those are `FunctionObject`'s only
object-valued fields — `func_id` is a `compiled_functions` index and `name` a `StringId`
(`vm/value.rs:588-599`) — and the match is *exhaustive, no wildcard* by its own statement at
`vm/gc/trace.rs:244` (`grep -c '^\s*_ =>' crates/script/elidex-js/src/vm/gc/trace.rs` → **0**, against
the positive control `grep -c 'ObjectKind::' crates/script/elidex-js/src/vm/gc/trace.rs` → **99** —
⚠ **both carried a bare `vm/gc/trace.rs` and neither reproduced from the repository root**: that path
does not exist there, so the claim side printed a no-such-file warning rather than a zero, and the
control side named no file at all and read standard input). Frames are the other marked location: `mark_roots` marks
`CallMode::Construct`'s `new_target` and `frame.home_class` (`vm/gc/roots.rs:294-297`) and the
suspended-generator arm marks the same two on `susp.frame` (`vm/gc/trace.rs:353-357`) — **4 sites, 2
fields**, the whole of `grep -rn 'home_class\|new_target' crates/script/elidex-js/src/vm/gc/` less its
two doc-comment hits. **So a move is a net *loss* of a strong edge rather than a missing addition**,
and an unmarked slot is returned to `free_objects` by `sweep_objects` (`vm/gc/sweep.rs:22-31`), which
makes the consequence a **recycled `ObjectId`** — a stale id resolving to an unrelated later object —
rather than a leak. Stated over the **edge**, not over a struct field, so it binds a hidden-property
representation (already reached by `trace_storage`) exactly as it binds a new field, and each slice's
own plan-review derives which it is. ⚠ **Invisible to the functional probes**, which is why it is an
invariant and not a per-row note: a reader that runs at *call* time passes every single-call test with
no collection in between. ⚠ **The population is defined by a **predicate**, and a grep is only its seed.**
The predicate: *does this row's deliverable put an `ObjectId` or a `JsValue` somewhere the collector
does not already walk?* Every row answering yes owes the arm and the regression, and each row answers
it at its own plan-review against its own parent HEAD — that answer, not a list here, is what binds.
⚠ **The seed grep is not the inventory, and this clause said it was for one revision.**
`grep -nE '^\| .*(GcRoots|mark_roots|gc/trace\.rs|gc/roots\.rs|gc/collect\.rs|tracing edge|trace arm|root set)' <this file>`
→ **12** rows, measured on blob `04734642` (this file at `c6b5bc31`) and **anchored to a blob because
the seed now counts this document's own prose**. Row **2aa** is the twelfth, and it matches for a
reason that is this clause's own thesis turned back on it: the note 2aa carries in order to
demonstrate that it sat *outside* the collector vocabulary quotes all eight alternatives verbatim,
which put it inside. That same note also quotes the negative control, so `GcRootsXX|mark_rootsXX`
over the same blob returns **2**, which is neither the 0 this clause first asserted nor the **1** that replaced it: the two hits are 2aa's note and the superseded sentence one screen above it, which quoted the same control literal while asserting 0. A seed that counts this document's own prose reaches the *control* exactly as it reaches the pattern, which is why the control is re-derived against the pinned blob by the same `git show 04734642` pipeline and never against the working tree. A control literal the file does
not contain, `GcRootsNONE|mark_rootsNONE`, same pipeline and same blob, returns **0**, so the pattern
still discriminates. The **11** that stood here was the count at `c26d7148`, before 2aa's note landed;
it is not a miscount but a figure whose population moved under it, which is why re-running is
`git show 04734642 | grep -cE '<the pattern above>'` and not the same pattern against `<this file>` —
the latter measures the working tree at the instant it runs, which is how this figure went stale
twice. The twelve: §5 rows **1a**, **2aa**, **3**, **4a**, **C**, **S**, **7**, **7a**, **7b** — nine —
and §8 slot rows `#11-vm-arrow-lexical-new-target`,
`#11-vm-operand-rooting-by-construction`, `#11-vm-typed-array-family-layering-and-gate` — three;
9 + 3 = 12. **It selects
rows that already use collector vocabulary, so it cannot return the rows whose omission is the very
thing this invariant exists to catch** — the out-of-vocabulary positive control proves it: the child umbrella **10b**'s derivation mints for `Proxy`
mandates a representation strongly holding `[[ProxyTarget]]` and `[[ProxyHandler]]` (**§10.5.15**
ProxyCreate) and the child umbrella **6a**'s derivation mints for async generators mandates an async-generator `ObjectKind` carrying suspended state and a
request queue; both answer the predicate yes, neither matches the grep, and neither appeared in the
twelve. **They are named here as failures of the seed, not as the completion of a list** — a list
completed by hand is the curated shape this clause replaced. ⚠ **A negative control shows a pattern is
not vacuous; only a positive control drawn from *outside the pattern's own vocabulary* shows it is not
self-selecting.** Any command offered as an inventory in this document carries both.
The two the earlier curated list omitted remain the illustration of why a list is the wrong shape:
**4a**, whose cell already owed a root field and a forced-collection regression and wrote that
regression *without* the sole-root half; and **C**, which had to write "so I-7 binds this row too"
into its own cell — the restatement this rule's last sentence forbids. For **S**, the predicate is
answered by whichever answer its plan-review gives about the environment link being object-valued;
for **3** and for **7a**, by the child that owns the representation rather than the umbrella — 7a's `[[MapData]]` / `[[SetData]]` variant and its unconditional marking arm sit on the child its derivation mints for them. ⚠ **And a forced-collection regression asserts that the collection was *placed*, not
merely requested** — arm `vm.inner.force_gc_before_next_alloc` and assert it was consumed, the in-tree
pattern at `vm/tests/tests_member_compound_assign_gc.rs:56-62` whose own docstring names the mode this
guards (a case that stops allocating at all, and so passes vacuously). A test that requests a
collection into a window holding no allocation measures nothing, however the code under it is
written. No row restates this rule; a row states only what it owes under it.

**I-8 · An acceptance condition declares its polarity.** Every acceptance condition a row states carries one of three declared readings **at the tree that row's slice is pinned to** — a **red probe**, which answers wrongly there and answers correctly only once the deliverable lands; a **green guard**, which already answers correctly and is stated so that the remedy may not break it; or a **declared control**, positive or negative, whose job is to bound an over-eager fix. The vocabulary is this document's own — "positive control" and "negative control" are already used throughout §5 and §8, and `#11-vm-named-function-expression-binding` already writes one guard out in full ("green today … a guard on the remedy rather than a red probe") — so what this invariant adds is that the reading is **stated**, not that it is invented per row. ⚠ **A condition written in red-probe voice while green at the pinned tree is worse than none**: a reader cannot tell it from a live defect, and a slice retires it without writing a line of the deliverable. Measured at `658cc302` over the ≈95 acceptance conditions this document's own R-loop delta touched, §5 rows **0bc**, **2ab** and **2ac** and §8 slots `#11-vm-object-spread-source-coercion`, `#11-vm-for-in-enumeration-semantics`, `#11-vm-parameter-environment-instantiation`, `#11-vm-arrow-lexical-new-target` and `#11-vm-object-literal-proto-setter` each state at least one condition the tree already satisfies while introducing it as a subtraction an implementation has to fix. ⚠ **That was one dry-run's output over one delta and a *seed*, not an inventory — and the population it seeded has since been measured in full.** The predicate was applied to **every** terminal row of §5 and §8 — the population being the rows the census program leaves unmarked, **75** of them at `d8e799aa`, the revision the sweep ran against (**33** §5 data rows, **42** §8 slot rows), and **61** at this revision because the same sweep converted fourteen of them: **17** state no acceptance because they are merged, pointer, adopted or spent rows, **58** are assessable, **2** pass — §5 row **6c** and §8's `#11-vm-native-function-length-on-allocator` — and **56** fail, each with a named implementation that satisfies every condition it states while leaving the deliverable wrong. Of the 56, **13** are discharged by becoming umbrella rows at this same revision, which carry no acceptance at all; the other **43** state theirs here. ⚠ **The eight rows this seed named are now four**: **0bc**, **2ab** and `#11-vm-parameter-environment-instantiation` are umbrella rows from this revision and carry no condition to mis-declare, and **2ac** discharged its flag by declaring its ordering regression a guard — while the seed *under*-read **2ab**, whose first regression was green for the same reason as its third and was declared vacuous only for the third. No row inherits its polarity from any list; each states its own, and the figures above are re-derived by re-running the sweep rather than read forward. ⚠ **Polarity is a property of the tree, not of the sentence**, so it is re-derived at each row's own mandatory `/elidex-plan-review` against that row's parent HEAD: a condition that is red today is turned green by whatever lands first, which is §5's prerequisite rule seen from the other side. **And the producer half**: a witness reads whatever it names, so where it reads an artifact no landed row mints — a global binding, a constructor, an intrinsic, a build configuration — the row records that derivation, and the child that will mint it, in its `Deps` cell rather than a blank. §5 already says this over slice *deliverables*; it is stated here over *witnesses*, because a row can carry a correct `Deps` cell for what its deliverable consumes and still spell an acceptance on a producer nobody owns — §5 row **8a** is the instance, every one of whose accessor witnesses reads a `RegExp` global that does not exist. No row restates this rule; a row declares its polarity and names the producer it waits on.

**I-9 · A new `ObjectKind` variant is admissible only against a stated criterion.** Lesson #276's
plan-memo control — *is this variant justified by a unique runtime state shape, or is it routable
through a representation that dispatch and the collector already walk?*
([[feedback_objectkind-resolution-uniformity]], PR #198) — is asked **per variant a derivation
mints**, so each umbrella's derivation answers it rather than whichever reviewer happens to notice.
⚠ **This document does reach for #276 once, and on a different question**: §9 decision 2b applies
the variant-vs-data shape to *opcodes*, where the discriminant is a stack shape the emit site knows
statically. Nothing in it reaches the object representation, and four sites here mandate a variant
with no criterion behind them — the async-generator kind the child umbrella **6a**'s derivation mints carrying suspended state and a request
queue and the representation the child umbrella **10b**'s derivation mints strongly holding `[[ProxyTarget]]`/`[[ProxyHandler]]`
(**§10.5.15** ProxyCreate, from `webref aoid ecma262 ProxyCreate`), both of which **I-7** already
names as its seed's out-of-vocabulary positives; the `[[MapData]]`/`[[SetData]]` variant on the
child umbrella **7a** mints for it; and the child `#11-vm-typed-array-family-layering-and-gate`'s derivation mints for the `TypedArray`/`DataView` variants (§8), whose
**second separating observable is stated over variant existence** — that no configuration carries a
`TypedArray` or `DataView` variant without the `ArrayBuffer` variant they point at — with nothing
behind it saying when a variant may exist at all. ⚠ **And #276's second disjunct is not available to
a core-language variant, so citing it unread would mandate an I-5 violation.** Its remedy is spelled
`HostObject` + an ECS-component brand check, which is engine-bound by construction; proposing it for
a language surface is exactly the outbound violation **I-5** forbids. For a core variant the
alternative is a representation the VM already carries — an object's hidden-property storage, an
existing kind's payload — and a row that can name none has *answered* the question rather than
dodged it. **I-7 covers the collector consequence of a representation change and explicitly not this
question**, which is what invites a reader to think it does: a row can owe a tracing arm under I-7,
write it, and still have minted a variant nothing sanctioned. No row restates this rule; a
derivation records, per variant it mints, which disjunct it answers and against what.

---

