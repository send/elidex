# VM P4 — Slice 1a + 1b detail: call-argument spread (§6, carved out)

**Carve-out of** `docs/plans/2026-07-vm-p4-es-language-completeness.md` — the VM P4 ES-language-completeness umbrella — whose **§6** this
file *is*. Moved **verbatim** by that document's touch-time prereq split (CLAUDE.md *1000-line debt =
touch-time split*; the seam derivation and its measurements are in the umbrella's §5). Not a rewrite:
no line below was reworded, renumbered or re-anchored, so **every line anchor, figure and revision
reference in it is stated about the pre-split umbrella** and is read that way.

**Scheduling stays in the umbrella.** §5's slice table, §8's slot ledger and §9's open decisions are
the SSoT for what is scheduled; this file is the design detail one slice pair's rows point at.

**Reading the §-numbers.** The headings below keep their umbrella numbers (`§6.1`, `§6.2`, `§6.2a`,
`§6.2a-2`, `§6.2a-3`, `§6.3`, `§6.4`, `§6.5`) — a reference to any of them, from anywhere, still
resolves here. Any other unqualified `§N` in this file refers to the **umbrella**, except: `§§10-15`
are in `docs/plans/2026-07-vm-p4-umbrella-review-rounds.md` and `§§16-18` are in
`docs/plans/2026-07-vm-p4-slice-0a-landing-record.md`, the umbrella's two other carve-outs.

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
   unless that `return()` call is removed.
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
| **Inline re-implementation** in `op_array_spread` | 1 | n/a | **not P's** — dec. 13a assigns its removal to the child of umbrella 1a that removes `op_array_spread`'s `return()`, via that umbrella's derivation, which mints it |

So the P family sweeps **12** — 10 Rust callers plus the 2 statement-lowering emits — of which only the 10 are
reachable by the signature change the charter used to describe as the whole remedy. The 2 `yield*`
sites leave P's scope entirely and the 1 inline site belongs to the child of umbrella 1a that removes `op_array_spread`'s `return()`, via that umbrella's derivation, which mints it. **Pb's plan-memo must settle the bytecode transport before it counts as
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
`grep -rn "well_known.return_str" crates/script/elidex-js/src/vm/` returns exactly **2** implementation sites (`dispatch_iter.rs:57`, `:359`; negative control `well_known.return_strz` → **0**) — the bare `return_str` returns **3**, the third being `well_known.rs:113`, the macro entry that defines the field
(`:57` inline, `:359` inside `iter_close`), so there is exactly one such duplicate. Consequences:
(a) fixing only `op_array_spread` does not advance P's sweep at all — the site is not in either
grep, and §6.2a-3 excludes it by name; (b) the P family
is sequenced before the child of umbrella 0bc that consumes the `Pa` edge, so this site keeps the
inverted convention until dec. 13a lands with the child of umbrella 1a that removes `op_array_spread`'s `return()`, via that umbrella's derivation, which mints it — and if 13a were rescoped the site stays inverted
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
termination and abrupt pattern evaluation. The child of umbrella 0bc that owns the **§8.6.2** array-binding `IteratorClose` obligation must pin both directions: `[a] = it` closes an
unfinished iterator, `[...r] = it` does **not** close an exhausted one.

**Why this lands on the critical path**: §8's `#11-vm-assignment-target-completeness` cell declares
that its work item "owns the `IteratorClose` obligation umbrella **1b**'s charter does not cover".
⚠ *This sentence quoted that cell as "the 0b family \u2026 obligation **1b** does not", and both halves had
gone stale in the cell without the quotation moving: the owner is now Slices **0ba**/**0bb** and the
children **0bc**'s derivation mints, and the obligation is stated against 1b's **charter**. A quotation
is a second copy of a cell, and this is the second one in this program to rot in the very commit that
repaired the original.* ⚠ **That cell read "Slice 1" and this sentence quoted it as "1a/1b"** — a misquote, and the
claim it carried is false in the 1a half: dec. 13a gives an [C36] obligation — to *remove* a non-spec close, not to make one — to the child of umbrella 1a that removes
`op_array_spread`'s `return()` an `IteratorClose` obligation, which is mechanism (b) of that
umbrella's own derivation. Both sites are corrected to **1b**, which genuinely carries none. The child of umbrella 0bc that its derivation mints for the [C39]→[C36] `IteratorClose` conformance will call `iter_close` and thereby **inherit the inverted contract**,
making its [C39]→[C36] conformance claim false. And because the sites span `compiler/`, core `vm/`
and `vm/host/`, fixing only `op_array_spread` leaves every one of P's sites inverted — including the
`for-of` catch handler (`stmt_loop.rs:147` at `658cc302`), which is the path most user code actually
hits.

**This is the same failure mode twice**: round 3 caught me propagating the IteratorClose *mandate* to
four sites but not the fifth; round 4 caught the *precedence* concept having its own un-swept
siblings. Both are [[feedback_semantic-sibling-selfseed-and-regate-breadth]] — the lesson is to grep
the **concept**, and a concept discovered mid-paragraph needs its own sweep, not an inherited scope.

⚠ **Once the child of umbrella 1a that removes `op_array_spread`'s `return()`, via that umbrella's derivation, which mints it lands, `op_array_spread` is [C19]/[C22]-only.** ⚠ *A clause here read "§5 sequences the 0b family before 1a"; it is deleted rather than re-pointed — no `Deps` cell in §5 carries that edge, and ordering has one structured home (`#11-plan-memo-spec-field-single-home-check`, §8).* The child of umbrella 0bc that repairs and consumes `Op::IteratorRest` owns
[C39], whose rest form (`[a, ...rest] = it`) needs a drain-into-array — and ⚠ **the drain it needs
already exists**: `compiler/stmt_destructure.rs:70` emits `Op::IteratorRest` and
`vm/dispatch_iter.rs:295-332` implements it as a drain-into-array, so that child repairs and consumes that
rather than `op_array_spread`/`spread_iter_loop`, which is the *spread* drain. ⚠ **This paragraph said the
rest form's spec *requires* `IteratorClose` and that 0bc must therefore add an explicit `iter_close`
site; both are withdrawn, and they contradicted the paragraph above.** **§13.15.5.5**
IteratorDestructuringAssignmentEvaluation's `AssignmentRestElement` production repeats *while*
`iteratorRecord.[[Done]] is false` (step 4) and performs no close, so a rest element that exhausts its
iterator normally must **not** see `.return()` — the regression the paragraph above already pins. What
that child owes here is therefore the **drain**, not a close, with the conditional close staying where
§13.15.5.2 puts it — around early termination and abrupt pattern evaluation. ⚠ **And "its own
drain-into-array" is withdrawn: the rest opcode already exists.** `compiler/stmt_destructure.rs:70`
emits `Op::IteratorRest` for an array rest element and `vm/dispatch_iter.rs:295-332` implements it as
a drain-into-array (its own comment: *"collect remaining iterator elements into a new array"*, with
the collected elements rooted on the stack). So that child **repairs and consumes `IteratorRest`**, factoring
out whatever it shares with `ArraySpread` — a second drain beside it is the N-mechanism outcome
CLAUDE.md *One issue, one way* forbids, and it would also leave the real rest path's defects
untouched, which is the opposite of what this paragraph exists to prevent. §7.2 pins all three, since an acceptance naming only the no-`return()` direction lets the child of umbrella 0bc that owns the **§8.6.2** array-binding `IteratorClose` obligation land with the close it does owe still missing: the drain that child consumes, the no-`return()` case on the exhausted-rest path, and the `[[Done]] is false` conditional close — `[a] = it` and `const [x] = it` over an iterator whose `return()` records the call (**§13.15.5.2**; **§8.6.2** BindingInitialization, production `BindingPattern : ArrayBindingPattern`, step 3, for the declaration form), which §5's 0bc row already carries as its required regression (§-numbers and
steps from `webref aoid ecma262 IteratorDestructuringAssignmentEvaluation` and
`webref body ecma262 sec-runtime-semantics-iteratordestructuringassignmentevaluation`). (Executing dec. 13a's propagation instruction here, in §6.2a, where a
0bc implementer reads it.)

**Decision** (§9 decision 13, restated): the **`op_array_spread` `return()` removal** belongs to **the child of umbrella 1a that removes `op_array_spread`'s `return()`** (it is the reuse precondition), placed there by umbrella 1a's derivation, which mints it — ⚠ *"the drain fix" stood here and is withdrawn: the rest drain is `Op::IteratorRest`, whose repair and consumption is **0bc's**, and what this child owns is specifically removing `op_array_spread`'s `return()`* —
but the **precedence sweep is its own unit** — a cross-cutting site set (§6.2a-3 has the
figure and its derivation), a signature change on the shared helper, and
a completion-kind distinction. Carve `#11-vm-iteratorclose-precedence-convention` and sequence it
**before the child of umbrella 0bc that its derivation mints for the [C39]→[C36] `IteratorClose` conformance** (whose conformance claim depends on it), not inside 1a.

### §6.3 Design — split across Slices 1a and 1b

**Slice allocation** (the user-adopted **1a/1b** split, §9 dec. 6 — not to be confused with the
removed `dispatch.rs` file split). Everything below is tagged:

- **[1a]** = VM infrastructure, and ⚠ **1a is an UMBRELLA from this revision (§5), so this tag names a
  half rather than a PR** — the three mechanisms below are the ones its cell measures as failing
  independently, and which child carries which is that umbrella's derivation to place, not this list:
  `lay_out_call_args`, `Empty` normalisation, `op_super_call_spread` conversion and its docstring
  correction (the `expr_class.rs:145-152` producer is spec-required, NOT folded), and the IC
  `Option<usize>` refactor **(a)**; `op_array_spread`'s `return()` removal **(b)**, decs. 13a; GC
  rooting of the 4 unrooted arg windows **(c)**, dec. 10 — each with its own edge row, see §6.4. No
  opcode is added and no emit path changes, so **the (a) child is observably a no-op for every call
  shape** — its only live consumer is `op_super_call_spread`, which keeps its current semantics, and
  that is what makes it a legal standalone PR under I-4. ⚠ **The no-op claim is scoped to (a), and read
  over the whole 1a half it is false** — which is what it said for several revisions: (b) removes a
  user-observable `return()` from live array-literal spread and (c) changes the outcome of calls in the
  four already-reachable argument windows whenever a collection lands there. Edges **21** and **27/28**
  exist precisely to detect those two, so a child plan reading the half as behaviour-preserving would
  contradict its own acceptance.
- **[1b]** = the compiler helper, `emit_call` aggregation, `CallMethodSpread`, the three handlers,
  and arity-based form selection — i.e. everything that changes observable behaviour.

⚠ **An umbrella states no acceptance condition (§5), so what stood here as "1a's acceptance test" is
evidence for its derivation to place on children, not a condition 1a retires against.** The reasoning
is kept because it is what makes the placement non-optional: *"the full existing suite passes
unchanged"* is **vacuous** for the two *semantic* mechanisms (`op_array_spread`'s `return()` removal,
dec. 13a; the arg-window rooting, dec. 10) — the existing suite covers neither, so an implementation
could omit both and still pass. Edges **21**, **27/28** and **32** are therefore required tests on the
specific children §5's 1a row names, edge 21 on the child of umbrella 1a that removes `op_array_spread`'s `return()`, edges 27/28 on the child of umbrella 1a that roots the four call-entry argument windows, edge 32 on
the child of umbrella 1a that owns the argument-layout and inline-cache contract. The edges umbrella 1b's derivation must place on the children it mints are §6.4's.

**Compiler** — one helper (I-3), replacing `compile_arguments`:

```rust
/// [C19] ECMA-262 §13.3.8.1 ArgumentListEvaluation.
pub(super) enum ArgsForm { Flat(u8), Array }
fn compile_call_arguments(…) -> Result<ArgsForm, CompileError>  // (NEW)
```

**The I-3 tagged-template input contract is a deliverable of the child of umbrella 1b that owns the `compile_call_arguments` helper, not a note.** The signature above
is the whole specification today, and `ArgsForm { Flat(u8), Array }` cannot express GetTemplateObject's
`« siteObj »` prefix followed by the substitutions — §3's TemplateLiteral rows state that constraint
and hand it to the child of umbrella 4a that owns the tagged-call lowering. §11 assigned the question to 1b, but an assignment in a round record owns
nothing, so as written every child of umbrella 1b can pass its charter and its tests and still leave the child of umbrella 4a that owns the tagged-call lowering to either change
this helper or emit its arguments some other way. That second mechanism is exactly what I-3 exists to
prevent. **So the child of umbrella 1b that owns the `compile_call_arguments` helper must define the helper's input contract — a caller-supplied fixed prefix plus an item
sequence — and test it.** Whether the prefix is a count or an item iterator is that child's own plan-review to
settle; that the contract exists, is documented at the signature and is exercised by a test is not.

**Acceptance condition the child of umbrella 4a that owns the tagged-call lowering consumes**: a test written by the child of umbrella 1b that owns the `compile_call_arguments` helper — that
test drives `compile_call_arguments` with a
non-empty fixed prefix plus *n* substitutions and asserts the emitted layout and the returned
`ArgsForm` for *n* = 0 and *n* > 0, so that child reaches its tagged-template lowering by *calling* this
helper — no signature change, no second argument-emission path. That child's mandatory plan-review
checks that it did. ⚠ **The consumer is named as a child rather than as `4a` because 4a is an umbrella
from this revision** (§5): an umbrella ships no PR, so naming it as the party that consumes an
acceptance condition names nobody.

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
no-regression guard, and ⚠ **No child of umbrella 3 may simply land `GetSuperProp` on the generic `CallMethodSpread` path (Codex R3)** — ECMA-262 **§13.3.7.3** MakeSuperPropertyReference step 4 takes the Reference's `[[Base]]` from **§9.1.1.3.5** `GetSuperBase(envRecord)`, which returns `home.[[GetPrototypeOf]]()` — the **`[[HomeObject]]`'s prototype, resolved at call time**, not the lexically named superclass — while step 5 keeps the current **`this`** (`actualThis`) as `[[ThisValue]]`, which §13.3.6.2 EvaluateCall then passes as the receiver. *(Both §-numbers were already right; the gloss said "superclass" and that is what was wrong.)* The generic member path emits `compile_expr(object); Dup; property-load`, while `Op::GetSuperProp` is declared `[-- value]`, so following the generic contract leaves a stray operand or passes the super base as `this`. The child of umbrella 3 that owns the super-property emit shape needs a **super-specific `PushThis; GetSuperProp` / `GetSuperElem` lowering**, covering named *and* computed spread calls; that child's own mandatory plan-review settles the stack shape. Superseded framing: Slice 3 lands `GetSuperProp` on top of the `CallMethodSpread` emit Slice 1
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

**Therefore**: this is an explicit **I-3 carve-out**, not a debt. The only obligation here belongs to
the child of umbrella 1a that owns the argument-layout and inline-cache contract, via that umbrella's derivation, which mints it, and it is
to **correct `op_super_call_spread`'s docstring**, whose claimed invariant ("the compiler emits
`CreateArray; ArraySpread x; SuperCallSpread`") is genuinely falsified by a second *legitimate*
producer — that part is behaviour-preserving. §6.4 gains edge 32 asserting
`%Array.prototype%[Symbol.iterator]` is **not** called for `class B extends A {}`, since no existing
test covers iterator non-observability and the "existing suite passes unchanged" reading cannot detect it.

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
guard, and the acceptance of the child of umbrella 1a that roots the four call-entry argument windows leans on exactly these.
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
[C37 step 3], **over an iterator whose `next()` / `return()` record the call**, as edges 21 and 24
already say: spelled with a plain array the non-evaluation observes nothing and the edge cannot
fail · 13. `new C(...a)` + `new.target` [C34 step 1] · 14. `super(...a)` no-regression ·
15. IC bookkeeping · 16. **pop → re-push holds no allocation — a structural assertion, not a runtime GC edge**: §2.5 C×D and §6.3 both derive that `lay_out_call_args` runs pop → `elements.clone()` → push with no JS allocation, and the only placement affordance arms at the next `alloc_object` (`vm/inner.rs:94-96`), so no collection can be placed in this window and a runtime "GC across pop→re-push" edge cannot fail however the helper is written. What 16 owes is a debug assertion that no allocation **event** occurred across the window — a monotonic allocation/collection epoch, or that an armed next-allocation tripwire is still unconsumed on the far side. ⚠ **Not a live-object count**: an allocation that triggers a collection sweeping an unreachable object can take the swept slot, leaving the population identical while the forbidden allocation — and the collection it may run while operands are held only in Rust locals — did happen. The live GC coverage is edges 27/28 · 17. `f.bind(x)(...a)` ·
18. native/host callees (`Math.max(...a)`) · 19. left-to-right evaluation order ·
20. `f(...[1,,3])` — assert **argument values** `[1, undefined, 3]`, not just arity (covers the
`Empty` boundary) · 21. iterator throws mid-drain → stack/GC unwind, and assert `return()` is
**NOT** called (§2.5 C×F — [C19] has no `IteratorClose`) · 22. iterator mutates callee/receiver
mid-drain · 23. **`super.m(...a)` no-regression** (stays Slice-3-broken, not differently broken) ·
24. **non-callable callee + observable iterator**: `const f = 1; f(...it)` must drain `it` fully **before** throwing — a Number callee throws at [C20] **step 4** (`const f = {}` reaches step 5); **declared**, because a bare `f=1` in this strict-only core is a write to an undeclared global and `Op::SetGlobal` raises a ReferenceError before the call is evaluated at all (`vm/dispatch.rs:195-222` at `d49465d0`), so the witness is never drained and the edge cannot fail on [C20]'s ordering · 25. same for `const NotACtor = {}; new NotACtor(...it)` [C33 steps 4.a→5] — **declared**, for edge 24's reason on the other side of the callee/argument order: [C33] steps 1-2 evaluate the callee *before* step 4.a's ArgumentListEvaluation and the compiler emits in that order (`compiler/expr.rs:88-92`), so an undeclared `NotACtor` raises a ReferenceError out of `Op::GetGlobal` before `...it` is touched · 26. `new BoundCtor(...a)` — bound-prefix splice
(`vm/ops.rs:696-700`) composed with the spread layout, asserting `boundArgs ++ spreadArgs` order ·
27. **generator callee — resumed, both vectors object-valued and read**: `function* g(p){ return [p.v, arguments[1].v] }` called as `g(...arr)` with `globalThis.arr = [{v:1},{v:2}]` built in *setup* — **and `globalThis.arr = null` before the collection, so nothing but the suspended frame's own vectors reaches those two objects**; then the collection, then `it.next()` must give `[1,2]`. ⚠ **The first spelling of this replacement left `globalThis.arr` bound across the collection, which is the sole-root failure this same invariant states one section up: the global object keeps both values marked and the assertion passes with both tracing loops omitted.** Writing the rule and breaking it in the edit that adds it is the shape to check for, not a slip peculiar to this edge — `p` witnessing `stack_slice`, `arguments[1]` witnessing `actual_args`. ⚠ **Both halves are load-bearing and both were wrong as spelled**: `actual_args` is `Some` only when the body names `arguments` (`vm/interpreter.rs:812`, `if needs_arguments`), and a generator that is never resumed never reads `stack_slice` at all (`:914` drains it into the `SuspendedFrame`), so `function* g(a){arguments}; g(...arrOf1000)` passed with all four rooting fixes omitted — §6.3 diagnosed this and §6.4 was not updated with it · 28. **async callee — awaited, same shape**: `async function a(p){ return [p.v, arguments[1].v] }`, asserting `[1,2]` after the returned Promise settles (`:867` drains, `:893` allocates); `async function a(){}` declares no parameter and names no `arguments`, so it built neither vector · ⚠⚠ **Neither edge can be written as a JS program, and two revisions spent trying is the finding.** The window they guard — drain into the Rust-local vector, then allocate — closes *before* the suspended frame is reachable from JS: by resume time the frame sits inside an `ObjectKind::Generator` whose existing arm (`vm/gc/trace.rs:345-370`) marks both vectors, so a collection placed at resume passes with the temporary rooting omitted; and an async function with no `await` is driven through its reads before the call returns (`make_async_coroutine_and_drive`, `natives_generator.rs:569`), so the async spelling must suspend before it reads. Nor can the one-shot be armed from JS: `f(...arr)` allocates its args array before `push_js_call_frame`, consuming an armed `force_gc_before_next_alloc` there. **So these two are stated over the window and executed from Rust**, through a hook that places a collection between the drain and the insertion while the source values are sole-rooted by the Rust-local vectors — the same placement affordance `#11-vm-operand-rooting-by-construction` owns (`vm/tests/tests_member_compound_assign_gc.rs:38-41`), which they name as their prerequisite. Written as JS against a resumed generator they cannot fail, whatever values the arguments carry · 29. **stack-depth bound, asserted at the bound** — the input derived from the constant the slice adopts, never written as a literal: `f(...arrOf(N))` at `N = bound` must succeed and at `N = bound + 1` must raise the bound's error, with the expected value stated (§9 dec. 12). A fixed `arrOf100k` cannot fail for any bound above 100 000 — including the crate's current *absence* of one (`grep -rn 'MAX_STACK\|stack_limit\|call_depth\|MAX_FRAMES' crates/script/elidex-js/src/vm/` → **0**), which is the state this edge exists to detect ·
30. **flat arity >255 without spread** `f(a1…a300)` — must compile and run, selecting the `Array`
form (§6.3); the pre-Slice-1 behaviour is a process abort · 31. `(o.m)(...a)` — parenthesized callee
must bind `this` to `o` (§9 dec. 14) · **32. [the child of umbrella 1a that owns the argument-layout and inline-cache contract] a synthesized default derived constructor must NOT
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
(GC window) and **32** fall inside the **1a** half — and ⚠ **1a is an umbrella from this revision, which
carries no acceptance condition (§5), so these three are stated here as what its derivation must place
and on which child**: edge **21** on the child of umbrella 1a that removes `op_array_spread`'s `return()`, edges
**27/28** on the child of umbrella 1a that roots the four call-entry argument windows, edge **32** on
the child of umbrella 1a that owns the argument-layout and inline-cache contract. That mapping is not a
convenience: it is the same independence its cell measures, one edge per mechanism.
Edge **35** belongs to **0bb**, not inside 1a or to 1b: it has no
spread and no call-shape change, and 0bb owns the optional-member receiver contract (§2.2, §5).
Everything else is for umbrella **1b**'s derivation to place on the children it mints. Round 5 flagged that calling the 1a half
"behaviour-preserving" while its only acceptance test was "the existing suite passes unchanged" left
both semantic fixes unverified: no in-tree test asserts `.return()` behaviour on array-literal spread
(`tests_generator.rs:714/736` assert the *opposite*, for `yield*`/`for-of`), and none covers iterator
non-observability. **The 1a half is therefore restated as: no call-shape change, plus two named semantic fixes
carrying their own edge rows** — and that restatement is what the terminality re-derivation then read
as three independently-failing mechanisms rather than one PR.

### §6.5 Non-goals

Tagged templates, class fields, super-property, private names — each its own slice.

---

