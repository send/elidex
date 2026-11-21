# R-JS per-PR plan — plan-review disposition tables

Companion of `docs/plans/2026-11-vm-native-construct-r-js.md` (its §13). Moved out of the memo at Rev 3.2.1 to keep the memo under 1000 lines; verbatim from `b13ea4c6` lines 972-1071 (PM 899: tracked in the repo, not a local handoff file).

## §13. Plan-review R1 dispositions (at `bee45988`; CRIT 0 / IMP 5 / MIN 25 / FP 3)
Record: `/Users/kazuaki/repos/send.sh/elidex-pm-trial/handoff/ncons/r-js-plan-review-r1-bee45988.md`. Each finding
was checked against the code at `3cbd5b88` or `webref` before it was folded.

| Finding | Disposition | Where |
|---|---|---|
| A1 MIN no layering row for the host touches | fixed: the CE crate owns the state machine, host = marshalling | §4.3.4 |
| A1 FP second prototype resolver in host | FP agreed: `resolve_validated_prototype` is R-NAT's (umbrella §5.3 S-1); named in the S-6 census | §4.3 DoD, §10 |
| A1 FP no sloppy this-binding branch | FP agreed: ThisMode is {Lexical, Strict} in core | — |
| A2 IMP visitor-defined population | fixed: defined at emission, unknown → capture; C1-12 … C1-15 | §1.3.3, §4.1 |
| A2 MIN static block over-mark | fixed by the same definition (the block is a non-arrow child) | §1.3.3, C1-15 |
| A2 MIN "never a JsValue" unchecked; holder list short | fixed: `ThisEnvId` newtype, four holders, DoD on `object_id()` callers | §1.3.1, §4.1 DoD |
| A2 MIN transitional writer unnamed | fixed: `overwrite_this_for_pre_allocation`, deleted by R-JS-3 | §1.3.3, §4.3 |
| A2 FP Side-store→component rule | FP agreed (PM 880 check; `PromiseCombinatorState` precedent) | §1.3.1 |
| A2 note umbrella §1.2 anchor `:17` | fixed: `call_dispatch_rooted` `:46` at `3cbd5b88` | RS-1 |
| A3 IMP second GetPrototypeFromConstructor | fixed: R-JS-3 converges it if the generator slot lands first (trigger met, verified) | §4.3 DoD, §7.2 |
| A3 MIN `expr_class.rs` marker not retagged | fixed: R-JS-3 retags both markers to `#11-step9-class-extras` | §4.3.1 |
| A3 MIN umbrella "next revision" items without trigger | fixed: three-children made now; R-0b's has trigger + date; S-3 partly FP (#654 re-cited it) | §9, C-7 |
| A3 MIN C3-17 escape hatch | fixed: red = R-JS-3's defect to fix; slot never closed on red | §4.3.5 cells |
| A3 MIN B6×B8 marked "—" | fixed | §2.1 |
| A3 MIN bound remap mechanism unnamed | fixed: visitor signature change, one walk; RS-2 restated | §4.3.1, RS-2 |
| A3 routed / A4-4 [C11] range | fixed: "steps 2-6.a, 9-17, the step-6.b call point" in both documents | RS-3 |
| A4-1 BindThisValue vs InitializeBinding | fixed: only SetMutableBinding is the reverse | §1.3 table |
| A4-2 PerformEval caller | fixed as a note; the step is **11.a**, not 10.a (`webref body ecma262 sec-performeval`) | §1.3 |
| A4-3 default constructors are built-in | fixed: 14.b / §10.3.2 path, value 1 for both | C-4, §3 |
| A4-5 "after step 3's creation" | fixed: "§10.2.2 step 3" | §3 |
| A4-6 provenance gaps | fixed | §12 M-1 |
| A4-7 docstring rule for R-JS-3 only | fixed: every child | §1.2 |
| A5 IMP-1 (E) vs ratified "on the call frame" | fixed: D1 shape (i); errata RS-6 / RS-7 | §1.3.4, §0.3 |
| A5 IMP-2 approval scope | fixed: PM 883 D2 widens PM 880's go | §0.3, C-2 |
| A5 IMP-3 C1-9 not a sole root | fixed: arrow dropped, receiver held only by the record | §4.1 C1-9 |
| A5 MIN §8.1 stale rows | fixed (PM 883 D2) | RS-1 |
| A5 MIN S / N canonical rows | fixed: only row C's text was prescriptive; the others gain forward pointers | RS-8 |
| A5 MIN history order 1.5, 1.7, 1.6 | fixed | RS-5 |
| A5 MIN RS-3 vs umbrella §4.2 | fixed (one range in both) | RS-3 |
| A5 MIN stale base | fixed: `3cbd5b88`, counts re-measured identical | header, §12 M-2 |
| A5 MIN §8 rule (a) | fixed: 11-28 is P0a-1's implementation date read as its landing | §8 |
| A5 MIN P0a-2b D-4 names `construct_synchronous` | fixed: D-4 reads `construct`'s `Err` | §2.1, §6.2, §4.3 DoD |
| A5 MIN PM ledger upgrade wording | fixed in the landing list (roots at `:350` / `:386` at `3cbd5b88`, not `:361` / `:388`) | §9 |
| A5 MIN R-JS-3 "single-axis" undersold | fixed: R-JS-3 carries four axes; why one PR | §1.2 |

### §13.1 Re-check of the R1 fix delta (at `58b666a6`; CRIT 0 / IMP 2 / MIN 10), folded per PM 886
Record: `/Users/kazuaki/repos/send.sh/elidex-pm-trial/handoff/ncons/r-js-recheck-r1fix-58b666a6.md`. Each claim was
checked against the files at `58b666a6`, main at `3cbd5b88`, or `webref` before it was folded.

| Finding | Disposition | Where |
|---|---|---|
| A5 IMP N1 slice-plan row S "on the call frame" ×2 | fixed: pointer to §1.3 at both sites; part of one sweep | RS-9 |
| A5 IMP N2 umbrella not swept for `construct_synchronous`'s deletion; §4.2 "for `new_instance` only" | fixed: one sweep, pointer or one-line correction per site; umbrella 3's overlap reading kept; history 1.7 (6) | RS-9 |
| A5 MIN N3 two stale ledger `:293` clauses | fixed in the landing list (A1 items 6, 7) | §9 |
| A5 MIN N4 RS-3 / RS-5 "old" text not on main | fixed: net change against `3cbd5b88` stated (addition) | RS-3, RS-5 |
| A5 MIN N5 C-after-R-JS-1 ordering not ratified | fixed: removed from §7.2 and the landing list A2; §1.3.4 sets no order | §7.2, §1.3.4 |
| A5 MIN N6 "C1-1 … C1-15" range | fixed: ids listed | §1.3.4, landing list A2 |
| A5 MIN N7 "single-axis" attributed to the base case | fixed: the base case quoted as written | §1.2 |
| A2 MIN `Closure` decided outside the exhaustive match | fixed: `ThisEnvRead::ByChildConstant`, resolved in `push_op`'s one `match`, unknown → capture | §1.3.3 |
| A2 MIN super-property stubs unnamed; S-7 by dispatch arm | fixed: the three stubs are `Reads`; S-7 by §9.4.3 consumer | §1.3.3, §10 S-7 |
| A2 MIN §4.1 DoD hit lists | fixed: "creation"; hit list named; three accessors; `ThisEnvId::for_new_cell` visibility | §1.3.1, §4.1 DoD |
| A2 MIN frame-entry writers unnamed | fixed: `do_new`, `construct_synchronous`, `run_function` | §1.3.3, §4.1 |
| A2 MIN accessor for a popped frame | fixed: `this_binding(&FrameThisEnv)`, no allocation between pop and read | §1.3.3, §4.2 |

### §13.2 Re-check of the Rev 3.1 delta (at `b8029ab9`; CRIT 0 / IMP 1 / MIN 19 — A2 3, A5 7 + 1, A3 5, A4 3 — and a label note), folded per PM 892
Record: `/Users/kazuaki/repos/send.sh/elidex-pm-trial/handoff/ncons/r-js-recheck-r31-b8029ab9.md`. Each claim was
checked against `3cbd5b88`, the files at `b8029ab9`, or `webref` before it was folded (§12 M-1, M-9 Rev 3.2); none
was false. **Additions ledger** (PM 892):
`/Users/kazuaki/repos/send.sh/elidex-pm-trial/handoff/ncons/r-js-additions-ledger-rev3x.md` — 52 rows (21 for Rev
3.1's additions, 31 for Rev 3.2's) over the memo, umbrella, slice plan, entry memo and landing list; **dead additions
found 2, fixed 2**: `bind_this` (the IMP) and `ThisBinding::Uninitialized` (found by the ledger: no construction
before R-JS-3's derived push, the same clippy ground); **contradictions found 9, resolved 9** (the `FrameThisEnv`
matcher claim vs `mark_frame`; the converter site; PushThis's "arm already returns ReferenceError"; `mark_frame`'s
`Inline(Uninitialized)` arm in R-JS-1; `complete_construct`'s step 15; a §4.3.3 → §4.3.1 cross-reference; §5's file
list; §6.2's start rule; the landing list's drifted PM-ledger line numbers). Intentional, listed there: umbrella §4.6's
birth list and `:448`, PM ledger `:245` / `:270`, the §13 / §13.1 history rows, `initialize_instance_elements`'s
unread parameters.

| Finding | Disposition | Where |
|---|---|---|
| A3 IMP `bind_this` dead until R-JS-3 | fixed (PM 892): `bind_this` lands in R-JS-3 with its caller; R-JS-1 has two accessors. Ground: CLAUDE.md "dead code は接続するか削除", `mise.toml:19` clippy `-D warnings` on lib targets, `VmInner` `pub(crate)` (`vm/mod.rs:190`). The ledger found the same for `ThisBinding::Uninitialized` (fixed alike; `this_binding` returns the value or a throw, so R-JS-3 changes no caller) | §1.2, §1.3.1, §1.3.3, §3, §4.1 DoD, §4.3.3, §4.3 DoD |
| A2 MIN-1 DoD grep cannot match `fn for_new_cell(` | fixed by the A3 uncertain MIN's change: the constructor is gone; the DoD greps `\bbox_this_environment\(` | §4.1 DoD |
| A2 MIN-2 `FunctionCompiler::bytecode` is `pub`; raw-byte grep misses `to_byte()` writes | fixed: R-JS-1 makes the field private (13 outside touches re-routed through `emit_u16_u8`, `patch_jump_to`, `ends_with_return`); DoD on the field, `to_byte()` and classified `as u8` hits | §1.3.3, §4.1, §4.1 DoD, §5 |
| A2 MIN-3 S-7 cross-check vs `Closure` = `ByChildConstant` | true (`create_closure` reads `this_value`, `vm/ops.rs:878-879`): `Closure` is the cross-check's one expected non-`Reads` hit | §10 S-7 |
| A5 MIN-1 slice-plan row S "not the call frame" | fixed (PM 892): "inline on the frame until an arrow observes it, then a shared cell", as row C / RS-7 / §1.3.1 | slice plan row S, RS-9 |
| A5 MIN-2 history 1.7 (6) | fixed: fact (a)'s sites (§1.2 In, §4.2), §2.2 E-9 (was "§2.1 … E-9"), grounds PM 886 / PM 892 | umbrella `:69` |
| A5 MIN-3 RS-7 "three phrasings" | fixed: "landing list A1 items 2-8" | RS-7 |
| A5 MIN-4 §9 Made / Listed | fixed: ledger edits only under "Listed"; `:333` (A5), A1 items 6-8, A6 added | §9 |
| A5 MIN-5 PM ledger `:248` / `:254` / `:257` | fixed (PM 892): landing list C2, at R-JS-3's landing, old texts `grep -c -F` = 1 each; `:245` / `:270` judged past measurements (not re-pointed) | landing list C2, §9 |
| A5 MIN-6 A1 item 6 cites Rev 1.7 (4) | fixed both ways: (4) now records 2ab's body / R-JS's call points, and item 6 also cites RS-3 | umbrella `:69`, landing list A1.6 |
| A5 MIN-7 slice-plan "umbrella §4.2 E-10" | fixed: "umbrella §2.2 E-10, §4.2" | slice plan 2ab |
| A5 MIN (uncertain) 1a's stack-layout evidence | fixed: the form moves to `VmInner::construct`'s `[newTarget, ctor, args…]` (§4.3.2 item 1) | slice plan 1a |
| A5 label note umbrella `:133` | no change (a live §1.2 row, as the note says) | — |
| A3 MIN §4.3.1 "admit_construct, unchanged" vs RS-2 | fixed: its IsConstructor classification is unchanged; it passes newTarget through | §4.3.1 |
| A3 MIN C1-14 / call points / retag not in a `#11-step9-class-extras` landing line | fixed: landing list A6 (repo `:23`) and B2 item 4 (PM `:289`); C1-14's value is Slice 2ac's | landing list, §4.1 C1-14, §9 |
| A3 MIN (uncertain) one-time DoD greps for the record's creation / `object_id()` | fixed: creation by type (`ThisEnvId`'s field private to `vm/frame.rs`, built only by `box_this_environment` there); what Rust cannot state (`object_id()` callers ⊆ `vm/gc/`, no `ObjectKind::ThisEnvironment` construction elsewhere) is wire #5's record half, born in R-JS-2 | §1.3.1, §4.2 |
| A3 MIN §8 rule (a) ignores R-0b | fixed: the later of P0a-1's and R-0b's landings | §8, §6.2 |
| A3 MIN `initialize_instance_elements` is effectively a no-op | stated: empty body (no [[Fields]] / [[PrivateMethods]] yet), its parameters unread until 2ab, 6.c unreachable until then; the ratified seam (E-10), not an unwired stub | §4.3.1 |
| A4 MIN (uncertain) "a fourth caller, PerformEval" | fixed: SuperCall step 7 named too (`webref body`); its op is already `Reads` | §1.3 |
| A4 MIN (uncertain) [C11] omits step 6.c | fixed: "2-6.a, 6.c and 9-17" / "the step-6.b call point and its step-6.c exit" in the memo, umbrella `:457`, entry memo `:54`, landing list A1.5; §4.3.1 states the exit | RS-3, §4.3.1 |
| A4 MIN "steps 9-11's frame removal" | fixed: step 9 removes calleeContext, steps 10-11 handle the completion | umbrella `:69` (4), RS-3 |

The Rev 3.1 dispositions in §13.1 that named `ThisEnvId::for_new_cell` and "three accessors" are superseded by the
rows above (kept as the record of Rev 3.1).
