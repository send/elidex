# VM P4 Slice 0a — landing record and its two review gates (§§16-18, carved out)

**Carve-out of** `docs/plans/2026-07-vm-p4-es-language-completeness.md` — the VM P4 ES-language-completeness umbrella — whose **§§16-18**
this file *is*. Moved **verbatim** by that document's touch-time prereq split (CLAUDE.md *1000-line
debt = touch-time split*; the seam derivation and its measurements are in the umbrella's §5). Not a
rewrite: no line below was reworded, renumbered or re-anchored, so **every figure, commit anchor and
line anchor in it is stated about the pre-split umbrella and about `658cc302`** and is read that way.

**A record of a landed PR, not a forward plan.** Slice 0a merged as `658cc302`; §5's three
specification fields are read from none of this, which is the asymmetry that made it a seam.

**Reading the §-numbers.** `§16` (implemented and merged), `§17` (pre-push `/elidex-review`, 2 CRIT)
and `§18` (post-push `/external-converge` and the design re-gate it triggered, with `§18.1` the carve
and `§18.2` the converge loop on the carved head) are here. Any other unqualified `§N` refers to the
**umbrella**, except: `§6` is in `docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md` and
`§§10-15` are in `docs/plans/2026-07-vm-p4-umbrella-review-rounds.md`, the
umbrella's two other carve-outs.

---

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
named in §4 I-5 and §5's Slice-P family rows as WebIDL two-layer cases *reserved for Pa's and Pc's
memos*, and all three sit in **Pc**'s touch set (§8's layering-gate row), so
**Slice Pc inherits a partly-made decision** and must not re-derive it from scratch. Not done, and
still owed by Pc: `structured_clone.rs`'s in-code `§2.9` tag is drifted (HTML §2.7.3 / §2.7.4 / §2.7.6 / §2.7.7 — ⚠ **§2.7.3 StructuredSerializeInternal was missing from this list while Pc's row carries it**, and it is the clause the memory-map identity citations at `:194-195` and `tests_typed_array_extras.rs:235` belong to, so an implementation inheriting *this* list retags to §2.7.4/§2.7.6/§2.7.7 and leaves those two drifted; the authoritative statement of the retag target is Slice Pc's §5 row, which F's classification correction reaches, and this parenthetical only restates it) and
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
