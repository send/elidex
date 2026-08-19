# VM P4 umbrella — round-2 … round-7 review dispositions and the convergence call (§§10-15, carved out)

**Carve-out of** `docs/plans/2026-07-vm-p4-es-language-completeness.md` — the VM P4 ES-language-completeness umbrella — whose **§§10-15**
this file *is*. Moved **verbatim** by that document's touch-time prereq split (CLAUDE.md *1000-line
debt = touch-time split*; the seam derivation and its measurements are in the umbrella's §5). Not a
rewrite: no line below was reworded, renumbered or re-anchored, so **every finding, round number,
figure and line anchor in it is stated about the pre-split umbrella** and is read that way.

**An append-only review archive, not a plan.** Nothing here is scheduled and nothing here owns work —
a round record cannot own work, which is a rule the records themselves state. The sections keep their
original numbers *and their original order*, in which **§13 precedes §12**; that out-of-order run is
what append-only looks like, and it is preserved rather than tidied.

**Reading the §-numbers.** `§10` (round 2), `§11` (round 3), `§13` (round 5), `§12` (round 4 +
convergence state), `§14` (rounds 6-7 + the convergence series) and `§15` (the convergence call) are
here. Any other unqualified `§N` refers to the **umbrella**, except: `§6` is in
`docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md` and `§§16-18` are in
`docs/plans/2026-07-vm-p4-slice-0a-landing-record.md`, the umbrella's two other carve-outs.

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
| `natives_string.rs` §21.1.3→§22.1.3 citation-drift unit | **the child of umbrella 9b that its derivation places the file-wide retag on** (§5 — whichever child it mints lands first with `vm/natives_string.rs` in its touch set), slot `#11-vm-builtin-prototype-static-sweep` — the file is under 9b, and Slice P does not reach it. *(It was homed on `#11-vm-webidl-section-3-10-retag` as a second citation-drift class; that slot now owns the WebIDL class only.)* |
| `vm/value.rs:1049-1053`'s eval-contract retag | **the child of umbrella 3 that owns the frame-state axis** (§5), via that umbrella's derivation, which mints it — the umbrella's module column already names `vm/value.rs` for that axis |

⚠ **The phase4-plan P4 disposition list above had silently become a to-do registry.** This section is a
*round record*; a round record cannot own work, because nothing re-reads it at re-eval time. Owner of
each of its five items, as assigned at `39bbdb1b`:

| Item | Owner now |
|---|---|
| TypedArray/DataView | §8 row `#11-vm-typed-array-family-layering-and-gate` |
| RegExp named-groups / lookbehind | named-groups → **Slice 8c** (§5 — that row states it is the owner); lookbehind → the child of umbrella 8 that its `webref heading ecma262 22.2` derivation mints for it. Slot `#11-vm-regexp-constructor-and-flags` |
| `replaceAll` non-global | **the child of umbrella 9b that its §22.1.3 derivation mints for `replaceAll`** (§5), slot `#11-vm-builtin-prototype-static-sweep` |
| `Object.fromEntries` iterator protocol | **was unowned** → the child of umbrella **9c** that its §20.1.2 derivation mints for the iterator drain, same slot |
| `String.matchAll` double-homing | **was unowned** — the round-2 text itself said "double-homing between `#11-vm-regexp-constructor-and-flags` and Slice 9", i.e. an explicitly unresolved owner → **the child of umbrella 9b that its §22.1.3 derivation mints for `matchAll`** (§5), same slot |

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
  (fixed-prefix count or item iterator) so Slice 4a has a live consumer path. ⚠ **Owner assigned:
  Slice 1b**, as an explicit deliverable of `compile_call_arguments` / `ArgsForm`, with Slice **4a**'s
  mandatory plan-review as the check (Slice 4 is an umbrella this document itself records as wrongly
  judged terminal; §6.3 already writes 4a). §3 already records the binding constraint (`ArgsForm` as
  specified cannot express `« siteObj »` ++ substitutions) and §5 gave Slice **4a** `Deps: 1b (I-3
  helper)` (this sentence said "Slice 4", whose own cell is `—`) — so the precondition was real and scheduled, but lived only in this round record, which
  owns nothing. ⚠ **The `4a` half of this bullet is superseded and the Deps reading with it**: the
  systematic terminality re-derivation makes `4a` an umbrella too, so its cell is `—` and both the
  edge and the check now belong to the child its derivation mints for the tagged-call lowering. Read
  §5, not this line.
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
  `#11-vm-iteratorclose-precedence-convention`, sequenced **before 0bc** (the child carrying the `Pa`
  edge — §5's Deps column).
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
`vm/dispatch.rs` prereq split; that split was removed — §5.)* **Slice 0bc** is gated on **Slice Pa** (its [C39]→[C36] conformance claim
inherits the inverted contract otherwise) plus three couplings the 0b family's plan-reviews must close (the `peel_paren` chokepoint **0bb** shares with dec. 14; the family's dependence on the sub-arm sweep that
**0ca** owns but is sequenced *after* it; and whether **0bc** connects the four destructuring opcodes or
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
that re-derive them*. 0bc is gated on Pa; 0ca on the I-1 carve-out (applied); 1a on dec. 6's contract
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

**Ship order — read it from the `Deps` cells; there is no sequence here.** A chain stood at this
position and was **withdrawn**: it read `0a → Pb → the layering/gating decision → Pa → 0b → 0c → 1a →
1b`, which named **three umbrellas (`0b`, `0c` and — from this revision — `1a`) as schedulable positions** — under §5's row-kind rule an
umbrella never lands, so no event advances the order past either — and thereby blocked three terminal
children whose `Deps` cells are empty (`0ba`, `0bb`, `0ca`; only **`0bc`** consumes Pa). It was also a
second home for the ordering, which §5 gives to the `Deps` column alone. What survives is the *reason*
an edge exists, which is all this section may carry:
⚠ **The layering/gating decision is a prerequisite of Pa, not a parallel concern** (§8's row; §5's Pa
`Deps` cell). Pa must edit `vm/host/typed_array_static.rs:798`, an `iter_close` caller, so without it
Pa either changes pure ECMA-262 behaviour inside a directory whose own mandate forbids that, or omits
a required site and ships an incomplete convention sweep that `0bc` then depends on. It is a
*decision*, so it need not be its own PR — but it must be settled before Pa starts, which is why it
sits in Pa's `Deps` cell rather than being described as a parallel concern here. Pb carries no edge to
the gate: its touch set reaches no `vm/host/` file, so nothing orders the two. *(The `vm/dispatch.rs` prereq split that rounds 2-9 mandated was **removed at implementation time** — §5's 1000-line check, whose figures were then re-derived at PR-B and largely withdrawn: the **match** is not relocated because CLAUDE.md exempts a flat case table; the **file's** size debt is discharged continuously by the arm-body extraction rule. Nine review rounds carried the standalone-split mandate.)*
Each carries its own `/elidex-plan-review`, at which point that slice's residual §9 decisions,
module columns and edge rows are settled against code rather than against this document.


