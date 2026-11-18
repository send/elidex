# T3-pre `EH-attrs-at-creation-T3-pre` — one creation event `SubtreeCreated`, fired by the cloner and the fragment parse, queued in a dispatcher-take window (per-PR plan-memo)

Nested PR **T3-pre** of the standalone program `EH-attrs-at-creation` (umbrella
`docs/plans/2026-10-event-handler-attrs-at-creation.md`, "the umbrella"; its in-place edits for this PR are carried by
the T3 memo's *Rev 8.<t>* entry, since the program's plan memos land in one plan PR (PM decision 722)). It is the prerequisite of
**T3** (`docs/plans/2026-10-eh-attrs-t3-target-determination.md`, "the T3 memo", rev 6.2), split out of T3 rev 4.8.2 by
**PM decision 722** (plan-review R1 F1 on T3 rev 4.8.2 `571cd9c8`: the queue / window cluster is edge-dense on its own,
§1.3). It carries the creation event and its queue; T3 carries the event-handler consumer of it.

**Owner: Layout lane (L2)** writes and implements T3-pre (PM decision 722). **The surfaces are L3's**: the dispatcher
slot (`DispatcherSlot`, `EcsDom::set_mutation_dispatcher` and its siblings), RC-2a's `MutationEvent::CloneCreated` and
its fire function, L3's dispatcher-take guard (PR (ig)), and the custom-element and form consumers. **T3-pre's
plan-review requires L3's confirmation, through the PM, of the sections marked [L3 surface]** (as PM decision 711 did
for T3 rev 4.7's §4.5): §2's queue/window pairs and I-RC1 to I-RC3, §4.2 to §4.6, §6.1's U-T11, §8.2's ig and RC-2b
entries. L3 confirmed rev 1's (2026-11-15; folded in rev 2 with PM decisions 740 and 741).

**Placement and order (PM decisions 722, 723, 740, 741).** Row **T3-pre** of the umbrella's §1.2 (owner Layout; added
by this plan PR). Edges (umbrella §1.3): **x13** L3 PR (ig) → T3-pre (T3-pre changes ig's `DispatcherTake::restore`
and `DispatcherTake::close`, so ig's guard must exist); **x11** L3 RC-2a → T3-pre (the event T3-pre generalises;
landed); **x14** T3-pre → T3 (T3's consumer arm matches `SubtreeCreated`). **No edge with L3's PR
(io) or with L3's tree-model program** (PM 730 placed that program between ig and io: L3's order is is → ig →
tree-model program → io → ii): T3-pre uses neither `WalkScope` nor the tree model's links, so it lands after ig, in
parallel with both (merge contact only, §8.1). **The order is unchanged by PM decisions 740 (1) and 741 (1).** T3 lands
after both T3-pre and io (umbrella x12, x14), so the sequence is ig → T3-pre → tree-model program → io → T3; the
`FragmentParse` cause reaches no acting consumer until the first of T3's landing and the fixes of the two slots that
fill its other arms (DoD item 7, the one statement of that window, with T3-a's and T3-b's condition on it). L3's Range **RC-2b** lands
after T3-pre and is written against `SubtreeCreated { cause: Clone(scope) }` (PM decision 723; §8.2, I-RC1 to I-RC3).

**Base.** main when T3-pre's implementation branch is cut, after L3's PR (is) and PR (ig) land: `cee74989` (#626, the
T3 memo's frame) + is + ig, plus whatever of L3's tree-model program and io landed first (merge contacts, §8.1). Every
coordinate below is a `cee74989` coordinate and is re-run at the base (§9 item 1). main is past `cee74989` (`c0578223`
on 2026-11-15); of the files this memo cites, one differs there, `crates/script/elidex-js/src/vm/lifecycle.rs` (the
unbind clear, `:34` → `:37`), and none moved (`git diff --name-only cee74989 c0578223 -- <the crates of §5.3 and
H-P1>`).

**Scope.**
- `MutationEvent::CloneCreated { root, scope }` is replaced, with no coexistence, by `MutationEvent::SubtreeCreated
  { root, cause }` (NEW) with `CreationCause` (NEW) = `Clone(CloneScope)` | `FragmentParse` (NEW), fired by
  `EcsDom::fire_subtree_created` (NEW), which replaces `EcsDom::fire_clone_created` (D-P1, §4.2; PM decision 705).
- Every consumer migrates (the custom-element, form and live-range consumers, the dom-api test probe, docs and tests;
  §4.6). The event-handler consumer's arm is **not** here: it is T3's (T3 memo D-T12; PM decision 722).
- The `FragmentParse` fire in `elidex_html_parser::parse_fragment_progressive` (D-P2, §4.3; PM decision 705).
- The window's queue: `DispatcherSlot::Taken { pending }`, the push, the `Vec<Entity>` return value of ig's restore
  function, the flush after it in the outermost `close`, the discard on unwind (D-P3, §4.4; PM decisions 711 (1),
  713 (2), 720), and a debug assertion in `Drop` against a silent discard on a returning path (plan-review R1 M3).
  Every writer of the slot is defined per state (§4.4's table; R1 M2); of the two changes R1 M2 asked for, the restore
  function's direct slot write and `set_mutation_dispatcher`'s assertion while `Taken`, both are **ig's** (PM
  decisions 740 (2), 741 (2)), and T3-pre relies on them (§4.4).
- The backend fragment entries, which fire nothing: `parse_html_fragment` becomes `pub(crate)`, and
  `parse_fragment_strict`'s rustdoc records it (§4.3).
- One sentence of `parse_into_inert_document`'s rustdoc, which T3-pre would otherwise make false (§4.6).
- Cells U-T10 and U-T11 (ids kept from the T3 memo rev 4.8.2) and new U-T12, U-T13 (§6).

**Not in scope.** What the event-handler consumer does with the event (T3: the `SubtreeCreated` arm,
`process_created_subtree`, the walks D-T10 / D-T12, D-T13, the rewrite of `inert_document.rs`'s rustdoc); the
restore function's direct slot write and `set_mutation_dispatcher`'s `Taken` assertion (L3's PR (ig), PM decision
740 (2)); the custom-element
upgrade at a fragment-parsed element's creation (`#11-custom-element-upgrade-at-fragment-creation`, L3, registered by
PM 719; §7); RC-2b's use of the event (L3, PM decision 723).

**Changelog.** *Rev 2.4* (2026-11-18; PM decision 774, after a re-check of rev 2.3 `10af92ee`). **Generator removed
(PM 774):** twice in a row the remedy §8.3 item 2 prescribed for `#11-form-control-state-template-contents-fragment-parse`
produced a new IMP in its own added text (N1: "every control" regressed fieldset disabledness; F1: rev 2.3's
re-derive-at-placement is the propagation that L3's `docs/plans/2026-07-form-derived-state-reconciliation.md` removes;
F2: "template contents only", with DOMParser's loop folded in, drops `FormControlState` from every DOMParser control
outside a template). Item 2 now states the defect only, in the ledger's shape (defect / spec / where / related root /
closing cells (a)–(d) / one hand-off sentence); the constraint, its two options and the dispatcher dependence as a
mechanism are deleted (the dependence stays as a fact the slot's plan must consider); the T3-pre sentences the fix
makes false move to an append after the row. §4.6's form arm (its heading line, its comment, its rev-2.3 note), §4.8's
form row and §10 name the slot and the arm as the fix's location, with no mechanism or constraint (F5). **F6:** DoD 7
check (b) gains a positive control of its own expression on a known hit (this memo). **F3:** the umbrella's x14 row
says T3's landing ends the window unless a slot's fix ended it first, and points to DoD 7 for the ends, sequence and
bound and to §8.3 item 3 for the escalation (umbrella rev-list note).
*Rev 2.3* (2026-11-18; Step 4.5 focused re-check on rev 2.2 `9f5010d9`: IMP 2 / clerical 6). **N1
(IMP):** §8.3 item 2's *Where the fix goes* no longer hands the slot a mechanism ("attach `FormControlState` to every
form control of the created set, absence-guarded"), which would leave a control parsed into a `<fieldset disabled>` by
`innerHTML` enabled (a serial parse's event precedes placement, U-T12; `is_fieldset_disabled` climbs the parent); it
hands a constraint (no ancestor-dependent field fixed before placement) with the two ways to meet it, chosen by the
slot's plan; the location (the arm, PM 767) and the goal (DOMParser's loop folded into it) are kept; a fieldset cell
joins *Closes when*; the dispatcher dependence of DOMParser's `FormControlState` after the fold is named; the form arm's
comment follows (§4.6). **N2 (IMP):** the window ends at the **first** of T3's landing and the two slots' fixes (either
slot can land before T3), stated once in DoD 7; §1 Q10, §1.1 (1)–(2), §4.6 (the common part, now carried by both arms
and naming no unit's landing; the `inert_document.rs` sentence; *The pointers' rewrite* replaced by *No pointer is
rewritten*), §4.8, R-P4, §8.2's T3 landing list, §8.3 item 3, D-P4 and §10 point to it; each slot row lists the T3-pre
sentences its fix makes false (§8.3 item 2, new item 4 for the custom-element slot); DoD 7 gains the grep that the
shipped comments name no window end; the T3 memo (rev 6.2) drops its landing item 13 and its grep. **N4:** §8.3 item
3 is the one statement of the escalation (triggers, raiser, backstop, cadence, what the raise asks); DoD 7 points to it.
**N5:** trigger t2 is "io's implementation PR is opened". **N6:** the resettable elements per HTML §4.10.2
(`category-reset`, includes `output`), read as the controls elidex gives `FormControlState`. **N7, N8:**
`node_methods/clone.rs`'s 2 census lines are roles I and D, which stay (no slice edits it), and `tree_clone.rs` has 0
census hits: both are contacts to re-check at the base (§8.1, §5.3).
*Rev 2.2* (2026-11-18; plan-review R3 wave 1 on rev 2.1 `0f8113eb`: CRIT 0 / IMP 1 / MIN 14 / FP 7;
PM decision 767). **W1-5 (IMP; PM 767 (2)):** the form `FragmentParse` arm is no longer "a permanent no-op": it is a
no-op in T3-pre and the creation-time write path that `#11-form-control-state-template-contents-fragment-parse` (L3,
registered by PM 767) fills, for the base defect that a form control in template contents created by a post-bind
fragment parse never gets `FormControlState` (HTML §13.2.6.1 step 24); its pointer names the slot, not T3 (§4.6,
§4.8, §7); §8.3 item 2 hands the PM the slot row's text. **PM 767 (1):** T3-ab is split into T3-a (D-T8) and T3-b
(D-T11); DoD 7's condition is "T3-a and T3-b land before io lands". **DoD 7 (Axis 3):** "at risk" gets observable
triggers, a named raiser (the Layout lane) and a record that outlives T3-pre's landing (§8.3 item 3, on
`#11-eh-attrs-creation-time-call-sites`'s row); the working bound is that slot's Re-eval date, its ledger *Trigger*
text being pending T3's landing list; L3's own escalation (tree-model umbrella §7.2) is cross-referenced. **Axis 5:**
the CE arm's "until T3 lands" pointer gets a rewrite obligation at T3's landing (§4.6); §8.1's tree-model contact is
re-pulled from that umbrella's draft 2 (`1593a5ad`: slices tp → ta → tc → tr → tf → tm → te → ts), with how U-T13's
parent assertion survives `ta`'s deletion of `get_parent`; §8.2's walker slice 1 is placed after `ta` (PM 761 (1));
§5.3's `dom/mod.rs` bullet no longer puts io before T3-pre's base. **Axis 2:** the `FragmentParse` created set is
defined at the fire, with the window invariant that makes a dispatch-time walk see the same set (§4.2). **Axis 4:**
the `Taken` rustdoc cites "T3-pre memo §4.4"; the comment on `CreationCause::FragmentParse` (NEW) cites HTML §8.5.1
`parseFromString` step 4; `fire_subtree_created`'s row cites its spec ground; the CE arm's anchor is scoped to §13.4
fragment parses (DOMParser's document has a null registry, so §13.2.6.1 step 16 gives no definition); the form arm
cites §13.2.6.1 step 24; §8.5.1's "scripting is disabled" is cited as step 4's note (§3, §4.2, §4.4, §4.6, §4.8).
*Rev 2.1* (2026-11-18; PM decision 756, pre-R3): DoD 7 states T3-ab's condition on the window (T3-ab
lands before io; raised to the PM when at risk of missing it), replacing rev 2's "judged when that slice is planned"
sentence; the header cites the T3 memo at rev 6.
*Rev 2* (2026-11-15): plan-review R2 on rev 1 (`38451533`; CRIT 0 / IMP 2 / MIN 14), L3's
confirmation of the [L3 surface] sections (CRIT 0 / IMP 4 / MIN 7) and PM decisions 740, 741. **P1:** §3's §13.4 row
cites steps 5 and 7. **P2 (= L3 IMP-1):** L3's plans are re-read at `23aa745a`, where ig already has the private
restore function and a `close` that calls it and checks a postcondition; so T3-pre *changes* ig's restore (it gains
the `Vec<Entity>` return value) and ig's `close` (it gains the flush after the restore), and introduces neither (H-P5
re-pulled; P-P1 closed; §4.4). **PM 740 (2), 741 (2):** the restore's direct slot write and `set_mutation_dispatcher`'s
assertion while `Taken` are ig's; T3-pre relies on them (§4.4, R-P3, DoD 1) and keeps only the `Drop` assertion of
rev 1's three; rev 1's U-T11 (f) and its mutant are ig's (§6, §8.2). **L3 IMP-2:** U-T11 (a) no longer reads the slot
from the probe. **L3 IMP-3:** the mutant "liveness checked once before the loop" is equivalent under the re-entry
contract and leaves the kill table; Q3×Q8 reworded. **PM 740 (1), 741 (1) (L3 IMP-4):** order unchanged; the no-op
window of the `FragmentParse` arms is "until T3 lands", stated once with its sequence, bound and escalation (DoD 7)
and carried by each arm's pointer (§4.6); the tree-model program (PM 730) is a merge contact (Base, §8.1). **MINs:**
the backend entries (§4.3, DoD 4); the restore row per state and Q3×Q7's ground (§4.4, §2); the `FragmentParse`
created set (§4.2); `inert_document.rs`'s rustdoc sentence (§4.6); `Taken { .. }` in ig's postcondition (§4.4); DoD 4
re-scoped to T3-pre's own surface and its script-session grep narrowed; the `TeardownWindow::close` path (§4.4); the
L3-side sentences handed off (§8.2); ig's M-3 (§6.2); §2's unlisted pairs; U-T13's roots; the no-op arms' spec anchors
and the cite chain (§4.8); §8.5.1 not read as governed by §13.4 (§3, §4.2); H-P1's population defined by property.
*Rev 1* (2026-11-13): new memo, PM decision 722. Moved from the T3 memo rev 4.8.2 without restating
there: §4.5 items 1–6 except the event-handler arm (here §4.2–§4.6), P-T20 (here P-P1), H-T29 to H-T31 and H-T32's
PR (ig) half (here H-P2 to H-P5), cells U-T10 / U-T11 and their mutants, DoD 4's `CloneCreated` /
`fire_subtree_created` / `Taken` greps. New against rev 4.8.2's text: every slot writer per state, with
`set_mutation_dispatcher` on `Taken` asserting and the restore function not going through it (D-P3, P-P2; R1 M2); the
non-panicking `Drop` assertion (R1 M3); §2's queue/window pairs (R1 F2); the fragment-path fallback record (R1 M8); the
three properties RC-2b relies on (PM decision 723); cells U-T11 (e)–(f), U-T12, U-T13.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every code `file:line` is a **`cee74989`** coordinate (origin/main, #626), measured with `git grep -n -E '<pattern>'
cee74989 -- <paths>`, `git show cee74989:<path> | sed -n <range>p` or `/usr/bin/grep -n` (never the default ugrep).
Paths are abbreviated after first use: `ecs/…` = `crates/core/elidex-ecs/src/…`. Spec text is `.claude/tools/webref
body` output of 2026-11-13 (rev 2's additions: 2026-11-15); every `§X.Y = title` pair is `webref heading` / `dfn`
output (§3's footnote).
**L3's names** are cited by symbol and section from L3's split plans at one pre-ratification snapshot, **`23aa745a`**
(the tip of L3's local plan branch on 2026-11-15, read only through `git show`; this is the one place the sha is
recorded, and "the snapshot" below means it; rev 1 read `6d31ced1`): "the PR (ig) plan"
(`docs/plans/2026-11-ecs-entity-lifetime-pr-ig-dispatcher-take-guard.md`) and "the L3 umbrella"
(`docs/plans/2026-11-ecs-entity-lifetime-umbrella.md`). From ig: `DispatcherTake` (NEW: ig; PR (ig) plan §4.1, "the
one way to take the mutation dispatcher out of an `EcsDom`"), its consuming `DispatcherTake::close` (NEW: ig), its
private `DispatcherTake::restore` (NEW: ig), its `Drop`, and `TeardownWindow` (NEW: ig) with its own
`TeardownWindow::close`. Every L3 citation is re-pinned to L3's ratified sha before T3-pre is ratified (R-P3). "The RC-2a plan" is
`docs/plans/2026-11-range-rc2a-clone-created.md` (on main, #589; implementation #596), cited by section.

### §0.2 Premises re-measured

| ID | Premise | Command | Result at `cee74989` |
|---|---|---|---|
| H-P1 | (R1 M2) The slot and every writer of it. **Population, by property:** a writer is any code that changes the value of an `EcsDom`'s `dispatcher` field: (i) a direct write of the field (it is private, so every one is inside `crates/core/elidex-ecs/src/dom`), (ii) a call of a function that writes it, (iii) a replacement of the whole `EcsDom` value in a place, which replaces its slot with it | (i) `git grep -n -e 'self.dispatcher' -e 'dom.dispatcher' -e 'dispatcher = ' cee74989 -- crates/core/elidex-ecs/src ':!*tests*'`; (ii) `git grep -n -e 'set_mutation_dispatcher(' -e 'take_mutation_dispatcher(' -e 'clear_mutation_dispatcher(' cee74989 -- crates ':!*tests*'`; (iii) `git grep -n -e 'mem::replace(dom' -e 'mem::take(dom' -e 'mem::swap(dom' -e '\*dom = ' cee74989 -- crates ':!*tests*'`; `git show cee74989:crates/core/elidex-ecs/src/dom/mod.rs \| sed -n 51,59p`, `sed -n 117,156p`, `sed -n 190,270p` | `enum DispatcherSlot { Absent, Installed(Box<…>), Taken }` (`ecs/dom/mod.rs:55-59`, private, in the private field `dispatcher`). Writers: `set_mutation_dispatcher` (`:196`, `pub`; `mem::replace` with `Installed(d)`, then `Absent \| Taken => None`, `:200-203`), `clear_mutation_dispatcher` (`:208`, `pub`; `Absent`), `take_mutation_dispatcher` (`:221`, `pub`; `Installed` → `Taken`, else unchanged), `dispatch_event` (`:247`, private; `Installed` → `Absent` for the dispatch, `Absent` / `Taken` left as they are, `:253-259`), `RestoreDispatcher::drop` (`:140-155`; writes `Installed(d)` back, debug-asserting `!Installed \|\| std::thread::panicking()` on what it displaced). Production restores of a take window call `set_mutation_dispatcher`: `ecs/dom/tree/teardown.rs:256`, `crates/dom/elidex-html-parser/src/lib.rs:157`, `crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs:241`, `crates/dom/elidex-form/src/inert_document.rs:203`; the one production install is `Vm::bind` (`crates/script/elidex-js/src/vm/vm_api.rs:325`, debug-asserting nothing was displaced), the one production clear is unbind (`crates/script/elidex-js/src/vm/lifecycle.rs:34`). Reader only: `teardown.rs:91`. (iii) → two lines, one site: the strict fragment build moves the caller's whole `EcsDom` out and back (`crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs:156`, `:158`), so between them the caller's place holds a fresh `EcsDom` whose slot is `Absent`; L3's PR (is) removes that shuffle (the builder borrows the caller's dom), so it is not a writer at T3-pre's base, and the grep is re-run there expecting 0 (§9 item 1) |
| H-P2 | The clone fire: one production site, once per clone call | `git grep -n -e 'fire_clone_created(' cee74989 -- crates ':!*tests*'`; `git show cee74989:crates/dom/elidex-dom-api/src/node_methods/clone.rs \| sed -n 235,312p`; `sed -n 325,342p` of `ecs/dom/mod.rs` | `clone_node_with_shadow_honor` (`crates/dom/elidex-dom-api/src/node_methods/clone.rs:235`) fires `MutationEvent::CloneCreated { root, scope }` once, after the whole copy exists (template contents and clonable shadow trees replicated) and before it returns (`clone.rs:305-311`); `EcsDom::fire_clone_created` (`ecs/dom/mod.rs:333`) debug-asserts that the slot is not `Taken` (`:334-340`; the RC-2a plan §4.7, D-4) and dispatches through `dispatch_event` |
| H-P3 | (T3 rev 4.7's H-T31, census restated) The `CloneCreated` population | `git grep -n -e CloneCreated -e fire_clone_created cee74989 -- crates \| wc -l` and `git grep -l -e CloneCreated -e fire_clone_created cee74989 -- crates \| wc -l`; `git grep -n CloneCreated cee74989 -- crates \| wc -l` and `git grep -l CloneCreated cee74989 -- crates \| wc -l` | **73 lines in 19 files** naming the variant (substring `CloneCreated`, so `CloneCreatedProbe` too) **or** `fire_clone_created`; of them **50 lines in 18 files** contain `CloneCreated` (the 19th file, `ecs/dom/tests/dispatch_guard.rs`, names only the fire function) (verified 2026-11-13). Match arms: `crates/dom/elidex-custom-elements/src/consumer.rs:116` (`CustomElementReactionConsumer`), `crates/dom/elidex-form/src/reconciler.rs:53` (`FormControlReconciler`), `crates/dom/elidex-dom-api/src/test_util.rs:53` (test probe `CloneCreatedProbe`); ignored-variant list `crates/dom/elidex-dom-api/src/range/live.rs:160` (`LiveRangeBridge`); fire: definition `ecs/dom/mod.rs:333`, one production call `clone.rs:305`. `EventHandlerAttributeConsumer::handle` matches `AttributeChange` and `Insert` and has a catch-all (`crates/script/elidex-script-session/src/event_handler_consumer.rs:381-391`); so do `node_iter`, `base_url`, `canvas` (`crates/script/elidex-js/src/vm/consumer_dispatcher.rs:157-163`) |
| H-P4 | (T3 rev 4.7's H-T29 / H-T30) The fragment build's own window; one function returns every fragment parse; DOMParser's window encloses one | `git show cee74989:crates/dom/elidex-html-parser/src/lib.rs \| sed -n 140,160p` and `sed -n 207,247p`; `git show cee74989:crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs \| sed -n 165,245p`; `git grep -n 'parse_fragment_progressive(' cee74989 -- crates`; `git show cee74989:crates/dom/elidex-form/src/inert_document.rs \| sed -n 108,205p`; `sed -n 221,232p` of `ecs/dom/mod.rs` | tolerant: `begin_detached_fragment` takes (`lib.rs:143`; `ecs/dom/tree/teardown.rs:308`), restore `lib.rs:156-158`; strict: take `tree_builder/mod.rs:180`, restore on both result arms `:238-241`. `parse_fragment_progressive` (`lib.rs:223`) is the one dispatch site of both backends (strict; on reject, tolerant over the pristine dom, `:229-246`), called only from `crates/script/elidex-script-session/src/mutation/html_fragment.rs` (`:93`, `:174`, `:209`, `:218`, `:235`, `:245`), which places the returned nodes after it returns. Entry points: `innerHTML` on Element / ShadowRoot / template, `setHTMLUnsafe`, `outerHTML`, `insertAdjacentHTML`, the queued `Mutation::SetInnerHtml`, DOMParser. `parse_into_inert_document` takes (`inert_document.rs:112`), calls `apply_set_inner_html` (`:142`), runs the form-control (`:172-174`) and base-URL (`:200`) reconcilers, restores (`:202-203`); inside it `begin_detached_fragment`'s take finds `Taken` and opens nothing (`ecs/dom/mod.rs:224-229`), so the slot is still `Taken` when `parse_fragment_progressive` returns there |
| H-P5 | (T3 rev 4.8.1's H-T32, its PR (ig) half; **re-pulled in rev 2**) L3's guard, as planned at the snapshot (§0.1) | with `S` = the snapshot and `P=docs/plans/2026-11-ecs-entity-lifetime`: `git show "${S}:${P}-pr-ig-dispatcher-take-guard.md" \| /usr/bin/grep -n -e 'pub fn close(mut self)' -e 'fn restore(&mut self)' -e 'self.restore();' -e 'DispatcherSlot::Taken));' -e 'set_mutation_dispatcher(dispatcher)' -e 'delegates the dispatcher half' -e 'through `set_mutation_dispatcher`' -e 'No new assertion is added' -e 'Layout T3 adds' -e 'This PR guarantees only' -e 'T3 lands after PR (io)'` → 13 lines (`:200`, `:202`, `:204`, `:206`, `:211`, `:213`, `:224`, `:282`, `:372`, `:399`, `:574`, `:579`, `:589`); `git show "${S}:${P}-umbrella.md" \| /usr/bin/grep -n -e '^\*\*Order\.\*\*' -e 'Layout T3 lands after PR (io)'` → `:347`, `:353` | PR (ig) plan §4.1: `DispatcherTake<'a> { dom: &'a mut EcsDom, taken: Option<Box<…>> }`, held by all four take windows; `open` calls `take_mutation_dispatcher` (made `pub(in crate::dom)`). **The restore function exists**: private `fn restore(&mut self)`, "the one restore code (PM 720)": `if let Some(dispatcher) = self.taken.take() { let _ = self.dom.set_mutation_dispatcher(dispatcher); }`, returning `()`. **`close(mut self)`** is `let took = self.taken.is_some(); self.restore(); debug_assert!(!took \|\| !matches!(self.dom.dispatcher, DispatcherSlot::Taken));` (the postcondition: a guard that took the box has put it back); `Drop` calls `self.restore()` and dispatches nothing. `TeardownWindow::close` writes its flags back and "delegates the dispatcher half to `DispatcherTake::close`". *Why it is unwind-safe* item 2: "`Drop` does nothing that can panic: the restore fn (one `mem::replace` through `set_mutation_dispatcher`)"; item 3: a nested guard holds nothing. *The restore point*: the outermost guard's close or drop; every window calls `close` on its returning path. "No new assertion is added; the slot behaviour on the return path is today's". §8 *T3 intersection*: "Layout T3 adds" the queue in `Taken` and the flush inside `DispatcherTake::close` after the restore fn (dispatch in order, skipping roots no longer live; on unwind the restore fn only, the queue discarded); ig guarantees only (a) the restore point is the outermost guard's `close`, its restore fn running there while the guard holds the dom, exactly one of `close` / `Drop` running it, and (b) whether a take is open has one home, the slot's `Taken`. L3 umbrella §5 *Order*: is → ig → io → ii, "Layout T3 lands after PR (io)". **Not in the snapshot, decided after it:** PM 730 (the tree-model program between ig and io); PM decisions 740 (2) and 741 (2) (ig's restore writes the slot directly and `set_mutation_dispatcher` debug-asserts while `Taken`; P-P2). The snapshot's "Layout T3" in these sentences is T3-pre since PM decision 722 (§8.2 lists them for L3) |
| H-P6 | (R1 M8) No other production fragment-creating path | `git show cee74989:crates/dom/elidex-dom-api/src/range/mutation.rs \| sed -n 640,660p`; `git grep -n -i -e 'parseHTMLUnsafe' -e 'document_write' -e 'fn write(' cee74989 -- crates/dom crates/script/elidex-js/src/vm/host` | `Range.createContextualFragment` is a stub that parses nothing (`crates/dom/elidex-dom-api/src/range/mutation.rs:648`); its rustdoc names the parser boundary it is to call ("through `elidex-script-session::mutation` (the canonical parser boundary)", `:640-641`) and the slot `#11-range-create-contextual-fragment`, which the ledger closed into `#11-range-full-impl` (2026-05-15 sweep; home: the Range program's RC-4). The grep → no hits: no `document.write`, no `parseHTMLUnsafe` |
| H-P7 | (PM decision 723) What RC-2b relies on | `git show origin/main:docs/plans/2026-11-range-contents-program.md \| /usr/bin/grep -n -e '^\| M × C' -e '^\| L × C' -e '^\| RC-2b'` | the Range program (on main, unlanded slice RC-2b): every clone root fires the clone-creation event from the canonical cloner entry, "between the mutations, in step order" (§2 *M × C*); "the bridge ignores `CloneCreated`" (§2 *L × C*); RC-2b clones through the canonical entry, one event per clone root (§1 row RC-2b) |

### §0.3 Premise errors (reported)

- **P-P1 (T3 rev 4.8.1's P-T20, moved; decided by PM 720; closed in rev 2).** PM decision 711 named the flush point
  "PR (i)'s consuming close"; the plan rev 1 read (`6d31ced1`) made `Drop` "the one site of the restore" with `close` =
  `drop(self)`, so a flush after the restore could not be written, and PM 720 chose (δ): one private restore function,
  called by `close` and by `Drop`. **At the snapshot the PR (ig) plan carries (δ) itself** (H-P5: the private
  `restore`, `close` calling it while the guard still holds the dom, the rejected forms (α), (β), (γ) recorded in its
  §4.1 *Why one restore fn*), so the premise error is gone and T3-pre introduces no restore function: it changes ig's
  (§4.4).
- **P-P2 (R1 M2; resolved by PM decisions 740 (2), 741 (2)).** The snapshot's restore function restores **through
  `set_mutation_dispatcher`** (H-P5), whose `mem::replace` returns the replaced `Taken` and drops it (`Absent | Taken
  => None`, H-P1); with `Taken { pending }` that call would drop the queue. And `set_mutation_dispatcher`, `pub`, can
  meet `Taken` from any caller. Rev 1 put both remedies in T3-pre. **PM 740 (2) puts them in ig** (the PR that creates
  the function owns its invariant): ig's restore writes the slot directly, in one `mem::replace`, and
  `set_mutation_dispatcher` debug-asserts while the slot is `Taken`; PM 741 (2) gives the reason they go together (with
  ig's own assertion forbidding `set_mutation_dispatcher` while `Taken`, ig's restore must write the slot directly in
  the same PR, or it would hit that assertion). The snapshot predates both decisions; T3-pre relies on ig landing with
  them (§4.4; R-P3's stop condition; DoD 1).

### §0.4 Decisions (this PR)

- **D-P1 — one creation event, generalised (PM decision 705).** `MutationEvent::CloneCreated` becomes
  `MutationEvent::SubtreeCreated { root, cause }`; no second variant beside it (one issue, one way). §4.2.
- **D-P2 — two fire points, the cloner's unchanged (PM decisions 705, 723).** `Clone(scope)` at the cloner's existing
  site; `FragmentParse` once per returned root in `parse_fragment_progressive`, after the backend's window has closed.
  §4.3.
- **D-P3 — the window's queue lives in the slot; ig's one restore function returns it; one flush point (PM decisions
  711 (1), 713 (2), 720, 740 (2); R1 M2, M3).** §4.4 is the one statement of the queue and of the flush; every other
  site of this memo, of the T3 memo and of the umbrella refers to it.
- **D-P4 — every consumer migrates; the `FragmentParse` arms that do nothing name what fills them (their own slot)
  and point to the window in which the cause is dead code (PM decision 722's dead-code condition, as reworded by PM 740
  (1); rev 2.3).** §4.6; DoD item 7.
- **D-P5 — the `Clone` cause keeps RC-2a's contract (PM decision 723).** I-RC1 to I-RC3 (§2), which RC-2b relies on.

## §1. Shape and verdict

| Axis | Own invariant |
|---|---|
| Q1 event shape | one variant, two causes (D-P1) |
| Q2 fire sites | the cloner (unchanged) and `parse_fragment_progressive` (D-P2) |
| Q3 window lifecycle | the slot's `Taken { pending }` is the window's only state; ig's one restore function returns `pending`; flush at the outermost `close` (D-P3) |
| Q4 placement order | serial: the event precedes the caller's placement; inside DOMParser's window: it follows the placement and the structural reconcilers (D-P2) |
| Q5 unwind | `Drop` without `close` restores and discards; no consumer code during unwind (D-P3) |
| Q6 nesting | only the guard that took restores and flushes (D-P3; ig's guarantee (a)) |
| Q7 unbind clear | `clear_mutation_dispatcher` discards `pending` with the window (D-P3) |
| Q8 root liveness | evaluated per root, at dispatch time, after the restore (D-P3) |
| Q9 slot writers | every writer defined per state (D-P3, H-P1); the restore's direct write and `set_mutation_dispatcher`'s `Taken` assertion are ig's (PM 740 (2)) |
| Q10 consumers | every consumer migrated; no arm acts on `FragmentParse` at T3-pre's head; the first acting arm (T3's, or the custom-element or form slot's fix) ends the window (D-P4; DoD item 7) |

### §1.1 Outcome at T3-pre's landing

Behaviour-neutral. The `Clone` cause is RC-2a's event renamed: same site, same scope, same consumers' bodies
(I-RC1 to I-RC3). The `FragmentParse` cause reaches every consumer, and none acts on it at T3-pre's head: the
event-handler consumer's arm is T3's (its catch-all at T3-pre's head), the custom-element and form consumers' arms are
no-ops (§4.6). So no observable step changes: no script-visible outcome, no `Insert` moved. What is new and observable
only to a probe dispatcher is that the event arrives (U-T10, U-T12, U-T13).

**The dead-code condition (PM decision 722, its condition (2) reworded by PM 740 (1); all three hold).** (1) Each
`FragmentParse` no-op arm carries, at its position, the pointer to what fills it, naming only its own slot: the
custom-element arm, `#11-custom-element-upgrade-at-fragment-creation`; the form arm,
`#11-form-control-state-template-contents-fragment-parse`, whose fix writes that arm (rev 2.2, PM 767 (2)); both point
to DoD item 7 for the window (§4.6; until rev 2.3 the custom-element arm named T3 and the window's sequence). (2)
The cause reaches no acting consumer **until the window ends, at the first of T3's landing and the two slots' fixes**;
DoD item 7 is the one statement of that window (its ends, sequence, why it is approved, its bound), and §8.3 item 3 of
its escalation. (3) Probe cells show the
event arrives (U-T10 in the parser, U-T12 through the session's `innerHTML` path, U-T13 through DOMParser's window).

### §1.2 Is T3-pre no worse than its base?

Yes: §1.1 (0 observable outcomes change). Release-build behaviour on the asserted paths: a `Clone` fire while `Taken`
is dropped as on the base (I-RC3); `set_mutation_dispatcher` while `Taken` (ig's assertion in a debug build, PM
740 (2)) installs as on the base, and the replaced window state takes its queue with it (a path no production code
takes, §4.4); a guard dropped without `close` on a returning path discards the queue (no consumer acts on it at
T3-pre's head).

### §1.3 Edge density

**Edge-dense** (*verdict*). Five intersecting axes of the window (Q3 lifecycle, Q4 placement order, Q5 unwind, Q6
nesting, Q7 unbind clear, with Q8 liveness evaluated inside them), on a subsystem without a canonical algorithm (the
spec has no dispatcher window; it runs the attribute change steps synchronously at creation, §3), so CLAUDE.md
"Edge-dense work" applies: plan-review is mandatory (rule (b)). **Split: none** — T3-pre is a narrowly-scoped per-PR
slice under the approved EH-attrs umbrella, carved by PM decision 722 (the rule's base case (c)). Splitting the event
from the queue would land a `FragmentParse` fire that DOMParser's window drops (rev 4.7's shape, overridden by PM
decision 711 (1)) or a queue with nothing to queue; splitting the consumer migration off would leave `CloneCreated`
and `SubtreeCreated` coexisting (one issue, one way). Breadth: `webref coverage-map dom 4.4 dom 4.9 html 13.2.6.1 html
13.4 html 8.5.1` → K = 2, M = 5, "ok → single PR scope" (2026-11-13). External couplings: L3's ig (the guard), RC-2a
(the event, landed), RC-2b (PM decision 723, I-RC1 to I-RC3); decisions 705, 711, 713, 714, 720, 722, 723, 730, 740,
741.

## §2. Coupled invariants

Queue / window pairs [L3 surface] (R1 F2; each named in §4.4, pinned in §6):
- **Q3×Q5 (× unwind)** — `close` restores, then flushes; `Drop` reached without `close` restores and discards. A panic
  in a consumer during the flush unwinds with the slot already `Installed` (`dispatch_event`'s own guard restores it),
  and the guard's `Drop` then finds `taken` `None`: no second restore, no second flush (U-T11 (b)).
- **Q3×Q6 (× nested guard)** — a nested guard (its take found `Taken`) holds nothing: its `close` and `Drop` neither
  restore nor flush, and `pending` stays in the slot for the outermost close (ig *Why it is unwind-safe* item 3; U-T11
  (a)).
- **Q3×Q7 (× unbind clear)** — `clear_mutation_dispatcher` inside a window writes `Absent`. `pending` exists only in
  `Taken`, so it goes with the state that held it; the outermost restore then writes `Installed(box)` back as on the
  base (ig §4.1: "the slot behaviour on the return path is today's") and its returned list is empty (U-T11 (c)). The
  pair is defined for the `pub` API, not reached in production: the one production clear is unbind (H-P1), and no
  production path unbinds inside a window, since a window's body is a synchronous `elidex-ecs` / parser /
  `elidex-form` function that runs no script and returns to no event loop before its close (H-P4, §4.5).
- **Q3×Q8 (× root liveness)** — liveness is evaluated **per root, at dispatch time, after the restore**: the flush
  checks `EcsDom::contains(root)` for each queued root immediately before dispatching it, so a root despawned inside the
  window is skipped (U-T11 (a)). Nothing despawns a root between two dispatches of one flush: a dispatcher's `dispatch`
  must not call a mutation primitive on the same `EcsDom` (the re-entry contract: `dispatch_event`'s rustdoc and its
  `dispatch_depth` assertion, `ecs/dom/mod.rs:232-252`), so a consumer of an earlier queued event cannot.
- **Q3×Q4 (× placement order)** — serial (no enclosing window: every entry point but DOMParser's): the event is
  dispatched at the fire, **before** the caller's placement, so it precedes every `Insert` of the placement (U-T12).
  Inside DOMParser's window: the event is queued and dispatched at the window's close, **after** the placement under
  the synthesized `html` element (whose `Insert`s reach no consumer, as on the base) and after the two structural
  reconcilers (form-control state, base URL; H-P4) (U-T13). No script runs inside the window, so "before script can
  observe the subtree" (D-P1's ground) holds in both.
- **Q3×Q9 (× slot writers)** — `pending` exists only in `Taken`; each writer's effect on it is defined (§4.4's table); no
  writer outside the restore function and the push changes it without either asserting (ig's assertion in
  `set_mutation_dispatcher`) or being the unbind discard.
- **Q1×Q3** — only the `FragmentParse` cause is ever queued (the `Clone` cause asserts instead, I-RC3), so the queue
  holds roots, not events, and the flush rebuilds each event with that one cause (§4.4).
- **Q2×Q3** — the fragment fire happens after the backend's own window closed (P1), so it is either a dispatch (no
  enclosing window) or a push (P2, P3); the clone fire never meets `Taken` in production (I-RC3).
- **Q1×Q10** — no consumer reads what another writes on this event (the `Clone` arms: the RC-2a plan §2 C × D; the
  `FragmentParse` arms: none acts at T3-pre's head), so the consumers' field order is unobservable at T3-pre's head.
  The arms that act later (the event-handler arm from T3; the form and custom-element arms from their slots' fixes,
  §7) each state this pair in the plan that adds them.

**The pairs not listed** (R2). Q4 to Q9 are facets of the window, each defined against Q3 above; a pair of two of them
either restates a listed pair or cannot occur: Q5×Q6 (an unwind through nested guards restores once, at the guard that
took: ig *Why it is unwind-safe* item 3; U-T11 (b) panics after a nested window), Q6×Q7 and Q7×Q9 (a clear is a row of
§4.4's table whichever guard is innermost), Q4×Q6 (the build's own window is nested in DOMParser's and closed before
the fire: Q2×Q3), Q4×Q8 and Q6×Q8 (the flush's order and its liveness check are one loop, §4.4); and with Q5, vacuous:
an unwind dispatches nothing, so it has no order (Q4×Q5), evaluates no liveness (Q5×Q8), and meets a clear or another
writer only as §4.4's restore row states (Q5×Q7, Q5×Q9). Q1 and Q2 meet the window only through Q1×Q3 and Q2×Q3
(Q2×Q4 is Q3×Q4; one cause per fire site is D-P2, Q1×Q2). Q10 against Q2 to Q9: a consumer receives the same event
whether it was dispatched at the fire or at the flush, and at T3-pre's head no arm acts on the queued cause, so these
are vacuous here; from T3 they are the T3 memo's (its §2).

Properties RC-2b relies on, kept by T3-pre for the `Clone` cause [L3 surface] (PM decision 723; L3 reads the T3 memo rev
4.8.2's §4.5 items 3, 4 and 6 as already satisfying them; verified here against that text):
- **I-RC1** — the `Clone`-cause fire stays inside the same cloner as today (`clone_node_with_shadow_honor`, H-P2), once
  per clone root, at the same step (after the whole copy exists, before the entry returns), so a caller that clones
  several roots between mutations gets one event per root in its own step order. (Rev 4.8.2 §4.5 item 3: "`Clone(scope)`:
  `clone_node_with_shadow_honor` …, unchanged: once, after the whole copy exists, before the cloning entry returns".)
  Here: §4.3. Pinned by the RC-2a cells renamed in place (`tests_clone_created.rs`, §5.3) and DoD 4's
  `fire_subtree_created(` count.
- **I-RC2** — `LiveRangeBridge` ignores `SubtreeCreated` (both causes). (Rev 4.8.2 §4.5 item 6: "the listed name
  changes; still ignored".) Here: §4.6; DoD 4's grep.
- **I-RC3** — for the `Clone` cause, D-4's `debug_assert!` (slot not `Taken`; the RC-2a plan §4.7) is kept. (Rev 4.8.2
  §4.5 item 4: "`Taken` with `Clone(_)` → D-4's `debug_assert!` … kept".) Here: §4.4; U-T11 (d).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §4.4 Interface Node | *clone a node* steps 2, 5, 6.7 (the copy, its light children, its shadow children); *clone a single node* step 2.5 (attributes appended to the copy) | the `Clone` cause: one event per clone root, after the whole copy exists | `clone_node_with_shadow_honor` (`crates/dom/elidex-dom-api/src/node_methods/clone.rs:305`) calls `EcsDom::fire_subtree_created` (NEW) with `CreationCause::Clone(scope)` (NEW) | ✓ | yes |
| WHATWG DOM §4.9 Interface Element | *append an attribute* step 4 → *handle attribute changes* step 3 | the attribute change steps run per appended attribute at creation: the event's ground (both causes) | `MutationEvent::SubtreeCreated` (NEW) in `crates/core/elidex-ecs/src/dom/mutation_event.rs` | ✗ | yes |
| WHATWG HTML §13.2.6.1 Creating and inserting nodes | *create an element for a token* step 21 (append each attribute) | a parser-created element gets its attributes at creation | the `FragmentParse` cause (NEW), fired by `parse_fragment_progressive` (`crates/dom/elidex-html-parser/src/lib.rs:223`) once per returned root | ✗ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | *HTML fragment parsing algorithm* step 5 (the parser runs: either backend) and step 7 ("Return fragment.") | every production fragment parse runs its parser and returns its nodes through one function, which fires after the return | `parse_fragment_progressive`, after both backends' windows close (H-P4) | ✓ | yes |
| WHATWG HTML §8.5.1 The DOMParser interface | `parseFromString` steps 3–4 (step 3: a new Document; step 4, `text/html`: *parse HTML from a string*; step 4's note: scripting is disabled because the document has no browsing context) | the spec parses a document here, not a fragment (§13.4 does not govern it); elidex builds that document by a fragment parse into a synthesized `html` element (`inert_document.rs:114-143`; slot `#11-domparser-full-document-parse-fidelity`), so the build's elements arrive by the `FragmentParse` cause, fired inside DOMParser's take window: queued and dispatched at the window's close | `crates/dom/elidex-form/src/inert_document.rs` `parse_into_inert_document` (under ig's `DispatcherTake`); `DispatcherSlot::Taken { pending }`, `DispatcherTake::close` (§4.4) | ✗ | yes |

Numbers and titles (2026-11-13): `webref heading dom 4.4` (Interface Node), `4.9` (Interface Element); `webref heading
html 13.2.6.1` (Creating and inserting nodes, `#creating-and-inserting-nodes`), `13.4` (Parsing HTML fragments,
`#parsing-html-fragments`), `8.5.1` (The DOMParser interface, `#the-domparser-interface`). `webref dfn dom`: 'clone a
node' → §4.4 `#concept-node-clone`, 'clone a single node' → §4.4 `#clone-a-single-node`, 'append an attribute' → §4.9
`#concept-element-attributes-append`, 'handle attribute changes' → §4.9 `#handle-attribute-changes`; `webref dfn
html`: 'create an element for a token' → §13.2.6.1 `#create-an-element-for-the-token`, 'HTML fragment parsing
algorithm' → §13.4 `#html-fragment-parsing-algorithm`. Step texts: `webref body dom concept-node-clone` (steps 2, 5,
6.7), `clone-a-single-node` (step 2.5), `concept-element-attributes-append` (step 4), `handle-attribute-changes` (step
3); `webref body html creating-and-inserting-nodes` (step 21), `html-fragment-parsing-algorithm` (rev 2, 2026-11-15: step 5
"Start parser and let it run until it has consumed all the characters just inserted into the input stream", step 7
"Return fragment."; rev 1 cited steps 1–2, the temporary Document and the fragment, which do not carry the row's
branch), `dom-domparser-parsefromstring` (steps 3–4; step 4's `text/html` case is "Parse HTML from a string given
document and compliantString", `webref dfn html` 'parse HTML from a string' → §8.5.1 `#parse-html-from-a-string`).
Rev 2's anchors outside this table (§4.2, §4.8), 2026-11-15: `webref heading html 4.10.5` (The input element,
`#the-input-element`), `4.10.11` (The textarea element, `#the-textarea-element`), `4.13.6` (Custom element reactions,
`#custom-element-reactions`), `4.12.3` (The template element); `webref dfn dom` 'cloning steps' → §4.4
`#concept-node-clone-ext`, 'create an element' → §4.9 `#concept-create-element`, 'shadow-including inclusive
descendant' → §4.8 `#concept-shadow-including-inclusive-descendant`; `webref dfn html` 'enqueue a custom element
upgrade reaction' → §4.13.6 `#enqueue-a-custom-element-upgrade-reaction`, 'template contents' → §4.12.3
`#template-contents`. Step texts: `webref body dom concept-node-clone` (step 3 "Run any cloning steps defined for node
in other applicable specifications"), `concept-create-element` (steps 4.4 and 5.2.2, "enqueue a custom element upgrade
reaction given result and definition", when *synchronousCustomElements* is false), `webref body html
create-an-element-for-the-token` (step 17: *willExecuteScript* is false when the parser's fragment context element is
non-null; step 19 passes it to *create an element*), `the-input-element` and `the-textarea-element` ("The cloning
steps for input elements …", "The cloning steps for textarea elements …").
Rev 2.2's anchors (2026-11-18, all re-run that day): `webref body html create-an-element-for-the-token` → step 5
(*registry* by *look up a custom element registry*), step 16 ("Let definition be null if creationRegistry is null"),
step 17, step 19, step 21 (append each attribute), step 24 ("If element is a resettable element and not a
form-associated custom element, then invoke its reset algorithm. (This initializes the element's value and
checkedness based on the element's attributes.)"); `webref dfn html` 'look up a custom element registry' → §4.13.4
(The CustomElementRegistry interface) `#look-up-a-custom-element-registry` (a Document's lookup returns the
document's registry), 'Resettable elements' → §4.10.2 (Categories) `#category-reset`, 'reset algorithm' → §4.10.23
(Resetting a form) `#concept-form-reset-control`; `webref body dom document-custom-element-registry` (DOM §4.5
Interface Document: "custom element registry is null" unless stated otherwise; `parseFromString` step 3 states none);
`webref body dom concept-create-element` (steps 4.4 and 5.2.2 are reached only when *definition* is non-null);
`webref body html dom-domparser-parsefromstring` (step 4, `text/html`, item 1 "Parse HTML from a string given
document and compliantString"; "Since document does not have a browsing context, scripting is disabled." is the note
after it, not a step).

### §3.1 User-input touch audit

Author-controlled inputs reaching the changed code: `cloneNode` (shallow / deep, with templates and clonable shadow
roots), every fragment-parsing entry point (`innerHTML` / `outerHTML` / `insertAdjacentHTML` / `setHTMLUnsafe` on any
context, the queued `Mutation::SetInnerHtml`) and `DOMParser.parseFromString`. T3-pre changes which event they fire and
when it is dispatched; at its head no consumer acts on the new cause (§1.1). A queue entry is an `Entity`, checked
alive before dispatch; the queue is bounded by the number of roots one parse returns inside one window, and is
discarded on unbind and on unwind. No security boundary changes.

## §4. Mechanism

### §4.1 Home / kind

| Fact | Home / kind | Status |
|---|---|---|
| the creation event | `MutationEvent::SubtreeCreated { root, cause }`, `CreationCause` (NEW, `elidex-ecs`, re-exported as `CloneScope` is), replacing `MutationEvent::CloneCreated` | D-P1 |
| its fire function | `EcsDom::fire_subtree_created(root, cause)` (NEW), replacing `EcsDom::fire_clone_created` | D-P1, D-P3 |
| the window's pending creation events | `DispatcherSlot::Taken { pending: Vec<Entity> }` (the slot's private enum gains the payload; World-level state, no component: per-window, not per-entity, as ig's guard) | D-P3 |
| the restore | ig's private `DispatcherTake::restore` (NEW: ig; the one restore function); T3-pre adds its `Vec<Entity>` return value | D-P3 |
| the flush | ig's `DispatcherTake::close`; T3-pre adds the flush after the restore | D-P3 |

No component, store, trait or crate edge is added (`elidex-html-parser` already depends on `elidex-ecs`). One event
variant is replaced, not added; one enum variant gains a payload.

### §4.2 The event [L3 surface]

```rust
// crates/core/elidex-ecs/src/dom/mutation_event.rs (re-exported from elidex-ecs's lib.rs, as CloneScope is)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreationCause {
    Clone(CloneScope),   // DOM §4.4 "clone a node"; CloneScope unchanged (RC-2a)
    FragmentParse,       // one elidex fragment parse: HTML §13.4 "HTML fragment parsing algorithm", and
                         // DOMParser's build (HTML §8.5.1 parseFromString step 4, a document parse by the
                         // spec), which elidex runs as a fragment parse
}
// in enum MutationEvent<'a>, replacing CloneCreated { root, scope }:
SubtreeCreated { root: Entity, cause: CreationCause },
```

**The created set.** The event carries the root only; the set it stands for is defined per cause, and none of its
nodes has been observed by script yet. `Clone(scope)`: the nodes `EcsDom::for_each_clone_created(root, scope, …)`
visits (the clone's visit set, the RC-2a plan §4.3; `CloneScope` and that function stay as they are). `FragmentParse`:
`root`'s shadow-including inclusive descendants (DOM §4.8; a declarative shadow root the parse attached, and its
tree, included) together with the template contents (HTML §4.12.3) of every `template` among them, recursively: every
node the parse created that hangs from that returned root. A consumer picks the subset it acts on and the walk that
reaches it (T3's: T3 memo D-T12); T3-pre fixes the set, not a walk.

**When the set is evaluated (R3 Axis 2).** The `FragmentParse` set is defined **at the fire**: the nodes hanging from
`root` when `parse_fragment_progressive` returns it. A consumer walks it **at dispatch**, so the two must agree, and
they do by this invariant of the window: *between a root's push and the flush that dispatches it, nothing changes the
nodes hanging from that root; the only tree change inside the window is the placement of the returned roots, which
gives each root a parent and leaves what hangs from it as it was.* Serial (no enclosing window): the dispatch is at
the fire, so there is no interval. Inside DOMParser's window (H-P4, `inert_document.rs:112-203`) the interval runs
the rest of `apply_set_inner_html` (the placement of the returned roots under the synthesized `html` element), the
form-control reconciler and the base-URL reconciler (component writes, no tree change), and the restore; no script
runs (Q3×Q4). A root despawned in the interval is skipped at the flush (Q3×Q8), not walked. *Why at the fire, not
"at dispatch":* the set is a creation fact ("what this parse created"); a dispatch-time definition gives the same set
today, but would silently take in nodes that a later step inside a grown window attaches under a root, which no parse
created. With the fire-time definition such a step breaks a stated invariant of an enumerated window body, which the
window's re-pull at ig's head checks (§9 item 1). Each acting arm's plan relies on this invariant, not on its own
re-derivation. **Spec ground (both causes):** each is
element creation whose attributes are appended at creation, DOM §4.4 *clone a single node* step 2.5 and HTML §13.2.6.1
*create an element for a token* step 21, each running DOM §4.9 *append an attribute* step 4 → *handle attribute
changes* step 3 ("Run the attribute change steps with element, …"). elidex batches those per-element runs into one
event per created subtree, dispatched before script can observe the subtree (RC-2a's precedent for the clone). Sibling plans that name
`CloneCreated` (the RC-2a plan, the Range program, the C0 and C1 memos) keep it as the name at their time.

### §4.3 Fire points [L3 surface]

- **`Clone(scope)`** — `clone_node_with_shadow_honor` (`clone.rs:305-311`), unchanged in place and step (I-RC1): once,
  after the whole copy exists, before the cloning entry returns; scope `Subtree` / `SelfOnly` as on the base. The call
  becomes `dom.fire_subtree_created(cloned, CreationCause::Clone(scope))`.
- **`FragmentParse`** — `parse_fragment_progressive` (`lib.rs:223-247`), after its backend match: once per returned
  root, in the returned order (a Text root too; consumers ignore what they do not walk). One site serves both backends:
  each backend's own window is closed when it returns (P1, §4.5), the strict one after `derive_element_components`
  (`lib.rs:235-237`), the tolerant one direct or as the fallback after a strict reject. A strict reject's partial
  subtree was despawned (`tree_builder/mod.rs:214-236`) and fires nothing. Every production fragment parse goes
  through this function (H-P4), so every entry point gets the event, DOMParser's build included. *Not chosen (T3 rev
  4.7):* firing inside each backend after its own restore, only when its take returned `Some` (two fire sites).
- **The backend entries fire nothing (R2; which way an unknown caller falls).** `parse_html_fragment`
  (`crates/dom/elidex-html-parser/src/lib.rs:88`) and `parse_fragment_strict`
  (`crates/dom/elidex-html-parser-strict/src/lib.rs:93`) are both `pub` and return detached roots without the event.
  Their only non-test callers are the two calls in `parse_fragment_progressive` (`lib.rs:229`, `:245`; `git grep -n -e
  'parse_html_fragment(' -e 'parse_fragment_strict(' cee74989 -- crates ':!*tests*'` → 4 lines, the two definitions and
  those two calls), but a new caller of either would create elements with no creation event, silently: the unknown
  falls on the unsafe side. A second fire site inside the backends is the form already not chosen (above). So the
  population is closed by structure where the compiler can hold it, and recorded where it cannot:
  `parse_html_fragment` becomes `pub(crate)` (no other crate names it: `git grep -n parse_html_fragment cee74989 -- .
  ':!crates/dom/elidex-html-parser/src' ':!docs/plans'` → 0; its test callers are the crate's own modules);
  `parse_fragment_strict` must stay `pub`, since `parse_fragment_progressive` calls it across the crate boundary
  (`elidex-html-parser` is the only crate that depends on `elidex-html-parser-strict`, `git grep -n
  'elidex-html-parser-strict' cee74989 -- '*Cargo.toml'`, so a second caller needs a new dependency edge, a reviewed
  change), and its rustdoc records: "Fires no creation event. Production code parses fragments through
  `elidex_html_parser::parse_fragment_progressive`, which fires `MutationEvent::SubtreeCreated` for the returned roots
  (T3-pre memo §4.3)". `parse_fragment_progressive`'s rustdoc names the tolerant backend as an intra-doc link
  (`lib.rs:213`); to a `pub(crate)` item that link is `rustdoc::private_intra_doc_links`, an error under the `doc`
  task's `-D warnings`, so it becomes plain code text. The caller population is re-run at T3-pre's head (§9 item 4).
- **Order against the placement** — Q3×Q4 (§2): before the placement when no window encloses the parse; inside
  DOMParser's window, at its close, after the placement and the two structural reconcilers.
- **Not a fire point:** the document parse (pre-bind; its creation-equivalent point is the first bind, umbrella K-6).
- **Record: the fragment path's fallback (R1 M8; the fragment analogue of T3 memo D-T10 *Fallback* / R6).** A future
  production path that creates a fragment by parsing must go through `parse_fragment_progressive`, or it fires no
  `FragmentParse` event and its elements get no creation processing (from T3: OR and XW processed at the first
  insertion by slice a's fallback, EW never, T3 memo D-T7). `Range.createContextualFragment` is the known future path
  (a stub today, H-P6; slot `#11-range-full-impl`, the Range program's RC-4); its implementation calls
  `parse_fragment_progressive` (or the session's `apply_*` wrappers that call it, the boundary its rustdoc already
  names). Record for the slot: §8.3 item 1.

### §4.4 The window's queue and the restore function (the one statement) [L3 surface]

The window's state has one home, the slot (World-level state; ig's guarantee (b)). This is the only statement of the
queue and of the flush (PM 720's shape (δ), which the PR (ig) plan §4.1 carries at the snapshot, H-P5); the T3 memo and
the umbrella refer here.

```rust
// crates/core/elidex-ecs/src/dom/mod.rs (T3-pre; on ig's head)
enum DispatcherSlot {
    Absent,
    Installed(Box<dyn MutationDispatcher + Send + Sync>),
    /// A take window is open (ig's `DispatcherTake`). `pending`: the roots of the `FragmentParse` creation events
    /// fired inside it, in fire order, dispatched at the close of the guard that took the dispatcher (T3-pre memo
    /// §4.4).
    Taken { pending: Vec<Entity> },
}
```

**Every writer of the slot, per state** (R1 M2; at T3-pre's head; H-P1 re-pulled at ig's head, §9 item 1). The
*Owner* column says whose change the row's behaviour is: "base" (unchanged), "ig" (L3's PR (ig), relied on), or
"T3-pre".

| Writer | Owner | `Absent` | `Installed(d)` | `Taken { pending }` |
|---|---|---|---|---|
| `fire_subtree_created(root, cause)` (NEW) | T3-pre | nothing, both causes (unbound, as on the base) | dispatch at once (`dispatch_event`) | `FragmentParse`: `pending.push(root)`, no assertion. `Clone(_)`: D-4's `debug_assert!` (I-RC3); release: dropped, as on the base |
| `dispatch_event` (every other event) | base | left as is | `Absent` for the dispatch, `Installed` again by `RestoreDispatcher::drop` | left as is: the event is dropped, as today; `pending` untouched |
| `take_mutation_dispatcher` (`pub(in crate::dom)` from ig; its one caller `DispatcherTake::open` (NEW: ig)) | ig; T3-pre writes the payload | left as is; the guard holds nothing | → `Taken { pending: Vec::new() }`; the guard holds `d` | left as is; the guard holds nothing (a nested window) |
| `DispatcherTake::restore` (private; called by `close` and `Drop` only), **when the guard holds a box** | ig: the write, one direct `mem::replace` of the slot with `Installed(box)` (PM 740 (2)). T3-pre: the return value, the replaced state's `pending` | reachable (the slot was cleared inside the window, Q3×Q7): → `Installed(box)`, as on the base; returns an empty list | reachable in a release build only (an install inside the window; ig's assertion fires in a debug build): → `Installed(box)`, the displaced dispatcher dropped, as on the base; returns an empty list | the window's ordinary close: → `Installed(box)`; returns `pending` |
| `DispatcherTake::restore`, when the guard holds nothing (a nested window, or a take that found `Absent`) | ig | no write; returns an empty list | no write; returns an empty list | no write; returns an empty list (`pending` stays in the slot for the guard that took) |
| `set_mutation_dispatcher` (`pub`) | ig: the `Taken` assertion (PM 740 (2)) | → `Installed` | → `Installed`, returns the previous | **`debug_assert!`** against an install inside an open take window; release: → `Installed`, the window's state replaced as by a clear, and with T3-pre's payload its `pending` with it |
| `clear_mutation_dispatcher` (`pub`; unbind) | base | → `Absent` | → `Absent` | → `Absent`; `pending` goes with the state that held it (Q3×Q7) |
| `RestoreDispatcher::drop` | base | → `Installed(d)` | — | — (it runs only after `dispatch_event` emptied an `Installed` slot; no consumer can take inside a dispatch, the re-entry contract) |

`set_mutation_dispatcher` keeps its base contract for `Absent` / `Installed` (`Vm::bind` and the test installs). Its
production callers on `Taken` are the base's four window restores (H-P1), which ig moves into the guard's restore
function; with ig's direct write no production caller meets `Taken`, and ig's assertion makes a future one visible
instead of dropping the window's state.

**What T3-pre relies on from ig** [L3 surface]. From the snapshot (the PR (ig) plan §4.1, §8 *T3 intersection*; H-P5):
(a) the restore point is the outermost guard's `close`, the explicit restore point on a returning path, distinct from
the unwind `Drop`; (b) whether a take is open has one home, the slot's `Taken` (no window keeps a second flag or depth
counter; the guard holds only the box it took), so `pending` has one home; (c) one private restore function, run by
exactly one of `close` and `Drop` for a guard that took the box, with `close` running it while it still holds the
dom; (d) `close`'s postcondition `debug_assert!` after the restore. **Decided after the snapshot (PM decisions 740 (2),
741 (2)) and not yet in it:** (e) the restore function writes the slot directly (one `mem::replace`, no
`set_mutation_dispatcher` call); (f) `set_mutation_dispatcher` debug-asserts when it meets `Taken`. (e) and (f) go
together in ig (P-P2). "The guard that took" is identified by what it holds (the box); nothing asks whether a guard is
the outermost (ig §4.1 *The restore point*). T3-pre stops for the PM if ig lands without any of (a) to (f) (R-P3, DoD
1); it does not add (e) or (f) itself.

**What T3-pre changes** (its whole edit to ig's guard and to the slot):
1. `DispatcherSlot::Taken` gains the payload, `Taken { pending: Vec<Entity> }`; `take_mutation_dispatcher` writes
   `Taken { pending: Vec::new() }`. Every pattern on the variant follows, ig's among them: `close`'s postcondition
   reads `!matches!(self.dom.dispatcher, DispatcherSlot::Taken { .. })` (the snapshot's `DispatcherSlot::Taken` no
   longer compiles).
2. The push: `fire_subtree_created`'s `Taken` arm for `FragmentParse` (the table's first row).
3. **ig's restore function gets a return value.** `fn restore(&mut self)` → `fn restore(&mut self) -> Vec<Entity>`: it
   is already a direct slot write at ig's head ((e)), and T3-pre returns from that same replacement the replaced
   state's `pending` (an empty list when the replaced state was not `Taken`, or when the guard holds nothing and
   nothing is written). Still one restore function, one slot state, called by `close` and `Drop` only.
4. **ig's `close` gains the flush after the restore.** `let took = …; let pending = self.restore();` ig's
   postcondition assertion; then, through `self.dom` (still held), for each root of `pending` in fire order: if
   `self.dom.contains(root)`, dispatch `SubtreeCreated { root, cause: FragmentParse }` (through `dispatch_event`, as
   `fire_subtree_created`'s `Installed` arm). Then `self` drops, and since `taken` is already `None` its `Drop`
   restores nothing and gets an empty list. The restore comes first, so the flush dispatches through the installed
   dispatcher. Liveness is evaluated per root at its dispatch (Q3×Q8). A consumer panic during the flush unwinds with
   the slot `Installed` (`dispatch_event`'s own guard) and the remaining roots undispatched; the guard's `Drop` then
   finds `taken` `None` (Q3×Q5).
5. **`Drop` discards, and asserts it discards only on unwind (R1 M3; PM 740 (2): this assertion is T3-pre's).** `Drop`
   calls the same function and discards the returned list; no consumer runs during an unwind, so ig's "`Drop` does
   nothing that can panic" still holds (PR (ig) plan §4.1 *Why it is unwind-safe* item 2). ig's windows call `close`
   on every returning path (H-P5), so a non-empty list reaching `Drop` on a returning path is a bug, and `Drop` says
   so: `debug_assert!(pending.is_empty() || std::thread::panicking(), "a take window holding queued creation events
   was dropped without close on a returning path")`. It fires only when not already unwinding, so it cannot turn an
   unwind into an abort (the precedent is `RestoreDispatcher::drop`'s `|| std::thread::panicking()`, H-P1). A guard
   that took nothing, or whose `pending` is empty, is unaffected. Cell U-T11 (e).

**The teardown window takes the same path.** `despawn_subtree`'s `TeardownWindow` closes through
`TeardownWindow::close`, which writes its flags back and then calls `DispatcherTake::close` (H-P5); on unwind its
`DispatcherTake` field is dropped. So there is one flush site and one discard site for all four windows, and the
teardown window needs no code of its own: when it is the outermost window its list is empty (nothing fires
`FragmentParse` inside a teardown), and inside a parse window (the strict rollback) its guard holds nothing.

### §4.5 Take / restore sites relied on (premise, re-pulled at ig's head) [L3 surface]

At `cee74989` (H-P4): (a) `begin_detached_fragment` takes (`teardown.rs:308`); the tolerant backend restores at
`lib.rs:156-158`, the strict one at `tree_builder/mod.rs:238-241` (both result arms), each before returning to
`parse_fragment_progressive`; (b) `parse_into_inert_document` takes at `inert_document.rs:112` and restores at
`:202-203`, enclosing `apply_set_inner_html` (`:142`); (c) `despawn_subtree` (`teardown.rs:225`, restore `:256`): not
relied on (nothing fires inside it; the strict reject path calls it inside (a)'s window). These anchors change shape in
L3's PR (is) and PR (ig) (every take becomes a `DispatcherTake`; the strict window opens its guard on the caller's dom,
PR (ig) plan §4.1 *The four windows*; the strict build's whole-`EcsDom` shuffle, H-P1 (iii), goes with PR (is)); T3-pre
re-pulls (a)–(c) at ig's head (§9 item 1). Properties relied on,
unchanged by is / ig (L3's review, PM decision 713 (1)): **P1** when the backend call inside
`parse_fragment_progressive` returns, the build's own window is closed; **P2** a window that encloses a fragment parse is
still open at the fire point (the slot reads `Taken { .. }`); **P3** a take inside an open window opens no window and is
followed by no restore (its guard holds nothing). With §4.4's (a) to (c) they make the fragment fire either a dispatch
(P1, no enclosing window) or a push onto the enclosing window's `pending` (P2, P3), flushed exactly once, at that
window's close.

### §4.6 Consumer migration [L3 surface]

At T3-pre's head `git grep -n -e CloneCreated -e fire_clone_created -- crates` → 0 (H-P3's census).
Each `FragmentParse` no-op arm carries a pointer at the arm to what fills it, with its spec anchor (§4.8).
**The common part** — "The window in which no consumer acts on this cause, and what ends it: T3-pre memo DoD item
7." — is carried by both arms. Each arm's comment names only its own slot and no other unit's landing, so it stays true
whichever of T3, the custom-element slot's fix and the form slot's fix lands first (rev 2.3; until rev 2.3 the
custom-element arm carried "No consumer acts on this cause until T3 lands …" with the window's sequence, which either
slot's fix landing before T3 would have made false).
- `CustomElementReactionConsumer` (`consumer.rs:116`): `SubtreeCreated { root, cause: CreationCause::Clone(scope) }` →
  `handle_clone_created(root, scope, dom)`, unchanged; `SubtreeCreated { cause: CreationCause::FragmentParse, .. }` →
  no-op, with the common part and: "in an HTML fragment parse (HTML §13.4) whose context's registry has a definition
  for the element, its creation enqueues an upgrade reaction (HTML §13.2.6.1 *create an element for a token* steps
  16, 17 and 19 → DOM §4.9 *create an element* steps 4.4 and 5.2.2 → HTML §4.13.6 *enqueue a custom element upgrade
  reaction*); that is `#11-custom-element-upgrade-at-fragment-creation` (L3). DOMParser's build enqueues none: its
  document's registry is null (DOM §4.5; HTML §8.5.1 `parseFromString` step 3), so step 16 gives no definition".
  Behaviour unchanged (PM decision 711 (4)).
- `FormControlReconciler` (`reconciler.rs:53`): `Clone(scope)` → `apply_clone_form_state`, unchanged;
  `FragmentParse` → **a no-op in T3-pre, and the arm where the fix of
  `#11-form-control-state-template-contents-fragment-parse` (L3; registered by PM 767) goes** (rev 2.2, R3 W1-5;
  worded as a location in rev 2.4, PM 774).
  The arm's comment: "No-op in T3-pre. A parser-created resettable element is reset at its creation (HTML §13.2.6.1
  *create an element for a token* step 24), which elidex does by attaching `FormControlState`. A post-bind fragment
  parse gets it today from the `Insert` arm, whose walk misses template contents (HTML §4.12.3), so a control created
  there gets none: `#11-form-control-state-template-contents-fragment-parse` is that defect, and its fix goes in this
  arm (T3-pre memo §8.3 item 2). The form *cloning steps* (HTML §4.10.5 for
  `input`, §4.10.11 for `textarea`; DOM §4.4 *clone a node* step 3) are a clone's, in the `Clone` arm", followed by
  the common part. *Base defect, not T3-pre's:* `handle_insert` walks only `for_each_shadow_inclusive_descendant`
  (`crates/dom/elidex-form/src/reconciler.rs:89-99` @ `cee74989`), which misses template contents, while DOMParser's
  build covers them with its own loop over `collect_template_inclusive_descendants`
  (`crates/dom/elidex-form/src/inert_document.rs:162-174`); without `FormControlState` the `value` getter returns
  the empty string (`crates/script/elidex-js/src/vm/host/html_input_value.rs:62-65`). T3-pre stays behaviour-neutral
  (§1.1); the slot's row text is §8.3 item 2. This pointer does not name T3, so T3's landing leaves it true. The
  comment names the slot and the location of its fix only; how the fix acts is the slot's own plan's (rev 2.4, PM 774;
  until rev 2.4 it also stated a condition on the fix, and until rev 2.3 a mechanism).
- **No pointer is rewritten at another unit's landing (rev 2.3; until rev 2.3 *The pointers' rewrite at T3's
  landing*, R3 Axis 5).** Neither arm's comment nor `inert_document.rs`'s added sentence (below) names T3 or a window
  end, so T3's landing leaves them true and T3's landing list carries no comment rewrite (the T3 memo rev 6.2 deletes
  its §8.3 item 13). Each slot's fix replaces its own arm and that arm's comment; the T3-pre sentences that a slot's
  fix makes false are listed in that slot's row (§8.3 items 2 and 4). DoD item 7 has the grep.
- `LiveRangeBridge` (`range/live.rs:160`, its ignored-variant list): the listed name changes to `SubtreeCreated { .. }`;
  still ignored (I-RC2).
- `EventHandlerAttributeConsumer`, `node_iter`, `base_url`, `canvas`: their catch-all arm, unchanged at T3-pre's head.
  T3 adds the event-handler consumer's `SubtreeCreated` arm (T3 memo D-T12).
- The dom-api test probe `CloneCreatedProbe` (`test_util.rs:43-53`) is renamed `SubtreeCreatedProbe` and records the
  cause; docs and tests that name the old variant or fire function (`ecs/dom/mod.rs`, `ecs/dom/tree/walkers.rs`,
  `ecs/dom/tree_clone.rs`, `elidex-custom-elements` `upgrade.rs`, `elidex-form` `clone.rs`,
  `elidex-js` `consumer_dispatcher.rs` and `node_methods_extras.rs`, and the test files of §5.3) are renamed in place.
- **`parse_into_inert_document`'s rustdoc (R2).** At `cee74989` it says the build "must fire NO mutation reactions"
  and that the suppression "silences ALL seven mutation consumers" (`crates/dom/elidex-form/src/inert_document.rs:52-67`).
  From T3-pre the second is false as written: the build's creation event is queued and reaches every consumer at the
  window's close. Left to T3 (which rewrites this rustdoc, T3 memo D-T13), the sentence would be false for the whole
  window of DoD item 7, so T3-pre adds one sentence after it and changes nothing else there: "One event is not
  silenced but deferred: the build's `MutationEvent::SubtreeCreated { cause: FragmentParse }` is queued in the window
  and dispatched at its close (T3-pre memo §4.4). At T3-pre's head no consumer acts on it (the window, and what ends
  it: T3-pre memo DoD item 7), so the build still runs no reaction." The rest of the rustdoc (its script-activation
  class) is T3's. The sentence names no later unit (rev 2.3). The form slot's fix, which makes the form arm act on the
  build, rewrites it (§8.3 item 2); T3 rewrites the whole rustdoc (T3 memo D-T13); the custom-element slot's fix
  leaves it true, since DOMParser's build enqueues no upgrade (its document's registry is null).

### §4.7 ECS-native and layering

The creation is an event on the existing mutation-event queue, consumed by systems (consumer arms) over the created
entities; no hook, no registry. The window's pending list is World-level state in the slot that already says whether a
window is open (one home; no per-entity component, since it is per window). OO → ECS idiom: "the parser calls back into
each element as it appends its attributes" (a per-element callback inside the parser in OO engines [contrast]) → one
event per created subtree after the build, consumed by the systems that need it. Layering: the parser and the cloner
fire a policy-free `elidex-ecs` event; no policy or document reaches `elidex-ecs` or `elidex-html-parser`.

### §4.8 Docstring citation table (Axis 4)

| Item | Cites |
|---|---|
| `MutationEvent::SubtreeCreated`, `CreationCause` (NEW) | the ground, whole chain: DOM §4.9 *append an attribute* step 4 → *handle attribute changes* step 3 (the attribute change steps, run per attribute at creation). `Clone`: DOM §4.4 *clone a node* steps 2, 5, 6.7 and *clone a single node* step 2.5 (RC-2a's text kept). `FragmentParse`: HTML §13.2.6.1 *create an element for a token* step 21 (the ground); HTML §13.4 *HTML fragment parsing algorithm* steps 5 and 7 (the fragment entry points); HTML §8.5.1 `parseFromString` step 4 (DOMParser's build: a document parse by the spec, which elidex runs as a fragment parse, so §13.4 is not cited for it). The created set: DOM §4.8 *shadow-including inclusive descendant*, HTML §4.12.3 *template contents* (T3-pre memo §4.2) |
| `EcsDom::fire_subtree_created` (NEW) | the spec ground by pointer: "see `MutationEvent::SubtreeCreated`" (DOM §4.4 *clone a node* for `Clone`; HTML §13.2.6.1 *create an element for a token* step 21 for `FragmentParse`; the whole chain is that item's row); T3-pre memo §4.4's writer table (the `Taken` arms: queue for `FragmentParse`, D-4 assertion for `Clone`) |
| `parse_fragment_progressive`'s fire | HTML §13.4 *HTML fragment parsing algorithm* step 7 ("Return fragment."); T3-pre memo §4.3: once per returned root, after both backends' restore; T3-pre memo §4.3's fallback record |
| `parse_fragment_strict`'s rustdoc (the added sentence); `parse_html_fragment` (`pub(crate)`) | T3-pre memo §4.3: fires no creation event; production code goes through `parse_fragment_progressive` |
| `DispatcherSlot::Taken { pending }`, `DispatcherTake::restore`'s return value, `DispatcherTake::close`'s flush, `Drop`'s assertion | T3-pre memo §4.4 (PM decisions 711, 713, 720, 740): restore, then dispatch in order, each root checked alive at its dispatch; discarded on unwind and at unbind. (The restore's direct write and `set_mutation_dispatcher`'s `Taken` assertion carry ig's citations, not T3-pre's) |
| the `FragmentParse` no-op arm of `CustomElementReactionConsumer` | the window (T3-pre memo DoD item 7, the common part; until rev 2.3 also T3 memo D-T12); **for HTML §13.4 fragment parses:** DOM §4.9 *create an element* steps 4.4 and 5.2.2 → HTML §4.13.6 *enqueue a custom element upgrade reaction* (HTML §13.2.6.1 *create an element for a token* steps 16, 17 and 19: with a definition and a non-null fragment context element, the element is created without running its constructor, so the upgrade is enqueued); **for DOMParser's build:** none (DOM §4.5: the document's registry is null, HTML §8.5.1 `parseFromString` step 3 setting none; so §13.2.6.1 step 16 gives definition null); slot `#11-custom-element-upgrade-at-fragment-creation` |
| the `FragmentParse` no-op arm of `FormControlReconciler` | slot `#11-form-control-state-template-contents-fragment-parse` (its fix goes in this arm; §8.3 item 2) and the window (T3-pre memo DoD item 7, the common part); HTML §13.2.6.1 *create an element for a token* step 24 (a resettable element, HTML §4.10.2, is reset at creation, HTML §4.10.23 *reset algorithm*); HTML §4.12.3 *template contents* (what `handle_insert`'s walk misses); HTML §4.10.5 (`input`) and §4.10.11 (`textarea`) *cloning steps*, run from DOM §4.4 *clone a node* step 3: the `Clone` arm's |
| `parse_into_inert_document`'s rustdoc (the added sentence, §4.6) | HTML §8.5.1 `parseFromString` step 4 (note: "Since document does not have a browsing context, scripting is disabled"); T3-pre memo §4.4 and DoD item 7 |

## §5. Populations, touched files, named risks

### §5.1 Populations (re-runnable)

The event's population: H-P3's census (73 lines in 19 files at `cee74989`, verified 2026-11-13) → 0 at T3-pre's head (DoD 4). Fire sites:
one per cause (H-P2; H-P4: `parse_fragment_progressive`, six callers). Slot writers: H-P1's population (by property), each in §4.4's
table. The backend fragment entries and their non-test callers: §4.3 (4 lines at `cee74989`).
Windows whose `pending` can be non-empty: those enclosing a fragment parse, i.e. DOMParser's (H-P4).

### §5.2 Existing tests whose expectation changes

None changes outcome (§1.1). Renamed in place: the `CloneCreated` probes and cells (§5.3); `fire_clone_created`'s
`Taken` assertion test in `ecs/dom/tests/dispatch_guard.rs` becomes U-T11 (d).

### §5.3 Touched files

Sizes at `cee74989` (`git show cee74989:<path> | wc -l`, 2026-11-13); every size is re-measured at the base T3-pre is
cut from (§9 item 1).
- `crates/core/elidex-ecs/src/dom/mod.rs` (959; the slot's payload, `take_mutation_dispatcher`'s `Taken { pending:
  Vec::new() }`, every `Taken` pattern, `fire_subtree_created` replacing `fire_clone_created`;
  `set_mutation_dispatcher`'s assertion is ig's edit, not T3-pre's).
  **R1 M19:** L3's PR (is) and PR (ig) also edit this file before T3-pre's base; L3's tree-model slices `ta`, `tc`,
  `te` and `ts` and PR (io) edit it too, after T3-pre in the planned order (ig → T3-pre → tree-model program → io) or
  before it if they land first (merge contacts, §8.1); its size is re-measured at the
  base, and if it is over 1000 there, the touch-time split check (CLAUDE.md "1000-line debt") runs first, as a
  standalone prerequisite split before T3-pre's feature commits (the dispatcher section, `:190-345` at `cee74989`, is
  the candidate seam).
- `crates/core/elidex-ecs/src/dom/dispatcher_take.rs` (NEW: ig): `restore`'s return value, the flush in `close`, the
  `Taken { .. }` pattern of `close`'s postcondition, `Drop`'s assertion (§4.4 *What T3-pre changes*).
- `crates/core/elidex-ecs/src/dom/mutation_event.rs` (441; the variant, `CreationCause`); `crates/core/elidex-ecs/src/lib.rs`
  (51; re-export); docs in `ecs/dom/tree/walkers.rs` (482; also edited by io, by L3's tree-model slices `ta` and `ts`
  and by the DOM-walker program's slice 1: merge contacts, §8.1, §8.2) and `ecs/dom/tree_clone.rs` (415; 0 census hits, a contact to
  re-check at the base, §8.1).
- `crates/dom/elidex-html-parser/src/lib.rs` (259; the fragment fire; `parse_html_fragment` → `pub(crate)` and the
  intra-doc link to it, §4.3); `crates/dom/elidex-html-parser-strict/src/lib.rs` (183; one rustdoc sentence, §4.3);
  `crates/dom/elidex-form/src/inert_document.rs` (291; one rustdoc sentence, §4.6; ig edits this file too);
  `crates/dom/elidex-dom-api/src/node_methods/clone.rs` (472; the clone fire; no tree-model slice edits it, a contact
  to re-check at the base, §8.1),
  `range/live.rs` (506), `test_util.rs` (78); `crates/dom/elidex-custom-elements/src/consumer.rs`
  (480), `upgrade.rs` (609); `crates/dom/elidex-form/src/reconciler.rs` (340), `clone.rs` (399); docs in
  `crates/script/elidex-js/src/vm/host/node_methods_extras.rs` (242) and `vm/consumer_dispatcher.rs`.
- Tests: `ecs/dom/tests/dispatch_guard.rs` (194; U-T11, or ig's guard test module, whichever holds the window cells at
  ig's head), `mutation_hook.rs` (919), `mutation_hook_fire_helpers.rs` (112), `traversal.rs` (611);
  `elidex-custom-elements/src/tests.rs` (731); `elidex-dom-api/src/node_methods/tests/clone_shadow.rs` (440);
  `elidex-js/vm/tests/tests_clone_created.rs` (186), `tests_clone_form_state.rs` (177); a fragment-fire test module of
  `elidex-html-parser` (U-T10; its `tests.rs` or a sibling, chosen by size); `crates/script/elidex-script-session/src/mutation/html_fragment.rs`
  (858; U-T12 beside `set_inner_html_fires_insert_at_caller_placement`, `:668-700`); `elidex-form`'s `inert_document`
  test module (U-T13).

Not touched: `crates/script/elidex-script-session/src/event_handler_consumer.rs` and
`event_handler_processing/mod.rs` (T3's); `inert_document.rs`'s code and the rest of its rustdoc (T3's, T3 memo
D-T13).

### §5.4 Named risks

- **R-P1 — the asserted paths in release.** §1.2: release keeps the base's outcome on each.
- **R-P2 — a consumer panic during the flush** leaves later queued roots undispatched (Q3×Q5); a consumer panic is
  already a bug that unwinds the whole mutation (as at any dispatch).
- **R-P3 — L3's surface, cited at a snapshot.** Every L3 citation (H-P5, §4.4, §4.5) is re-pinned to L3's ratified sha
  before T3-pre is ratified; a citation whose symbol or section is gone there is re-derived, not carried. The re-pin
  also checks that the PR (ig) plan then carries §4.4's (e) and (f) (PM 740 (2)), which the snapshot predates, and
  the reworded sentences of §8.2. **Stop condition:** T3-pre stops for the PM if ig is ratified or lands without any
  of §4.4's (a) to (f): no consuming `close` as the restore point on the returning path of a window that can enclose
  a fragment parse (a); a second home of take/restore state (b); no single private restore function that `close`
  runs while it still holds `&mut EcsDom` and that `Drop` shares (c); a restore that goes through
  `set_mutation_dispatcher` instead of writing the slot directly (e); or `set_mutation_dispatcher` not asserting
  while `Taken` (f). T3-pre does not add (e) or (f) to make up for it: they are ig's by PM 740 (2).
- **R-P4 — the no-op window (PM decisions 722, 740 (1), 741 (1), 767 (1)).** From T3-pre's landing until the window
  ends the `FragmentParse` event reaches no acting consumer. Its ends (the first of T3's landing and the two slots'
  fixes), sequence, bound and T3-a's and T3-b's condition are DoD item 7's; the escalation to the PM is §8.3 item 3's.

## §6. Test plan

Red-first: each **red** cell is committed failing on T3-pre's base before the fix commit.

### §6.1 Unit

| Cell | Kind | Shape | Oracle |
|---|---|---|---|
| U-T10 | red (§4.3; `elidex-html-parser`) | a probe dispatcher recording every event; `parse_fragment_progressive` with two top-level elements, on input the strict backend accepts and on input it rejects (tolerant fallback); then the same inside an outer `DispatcherTake` (ig) window, closed after the call | exactly two `SubtreeCreated { cause: FragmentParse }`, roots in the returned order, no other event before them, none naming a despawned entity; inside the outer window none reaches the probe during the call, and the same two, in the same order, reach it at the outer close, after the restore. Base: no event |
| U-T11 [L3 surface] | red + guard (§4.4; ig's guard test module or `ecs/dom/tests/dispatch_guard.rs`) | a probe dispatcher installed; an outer `DispatcherTake`: (a) `fire_subtree_created(r1, FragmentParse)`, `(r2, …)`, `(r3, …)`, `despawn_subtree(r2)` inside the window, then a nested `DispatcherTake` opened and closed, then the outer close; (b) as (a) up to the nested window, then a panic inside the outer window, caught outside it (`catch_unwind`); (c) one push, then `clear_mutation_dispatcher` inside the window, then the close; (d) `fire_subtree_created(root, Clone(CloneScope::Subtree))` while `Taken`; (e) one push, then the outer guard dropped without `close` on a returning path. (Rev 1's (f), an install inside the window, is ig's cell since PM 740 (2): the assertion it pins is ig's, §8.2; the letter is not reused) | (a) nothing reaches the probe before the outer close (the nested close flushes nothing); at the close exactly `r1`, `r3` arrive, in that order (their arrival is what pins "restore, then flush": a flush into a slot still `Taken` delivers nothing; the probe does not read the slot, which `dispatch_event` has emptied to `Absent` for the dispatch); (b) the slot holds the dispatcher and the probe received nothing; (c) nothing is received; (d) debug-asserts (D-4, the existing test renamed; I-RC3); (e) debug-asserts in `Drop` (R1 M3). Base: (a)–(c) receive nothing (no queue), (e) does not assert |
| U-T12 | red (Q3×Q4, serial; end to end through the session) | beside `set_inner_html_fires_insert_at_caller_placement` (`html_fragment.rs:668-700`): a probe dispatcher recording every event; `apply_set_inner_html` on an empty connected element with `<p></p><i></i>` | the log is `SubtreeCreated { p, FragmentParse }`, `SubtreeCreated { i, FragmentParse }`, then the placement's `Insert`s. Base: the `Insert`s only |
| U-T13 | red (Q3×Q4 nested; end to end through DOMParser's window) | a probe dispatcher; `parse_into_inert_document` with `<body><p></p></body>` | no event reaches the probe during the build except at its close; at the close, after the restore, exactly two events, `SubtreeCreated { cause: FragmentParse }` for the `head` element and then for the `body` element (the two roots the build's fragment parse returns for this markup under the synthesized `html`; asserted by tag, and each root's parent is the `html` element when its event arrives: the placement came first; the parent is read as the tree parent, §8.1 for L3's `ta`). No `Insert` is in the log (the placement's reached no consumer). Base: nothing |

Kept green, renamed in place: the RC-2a cells (`tests_clone_created.rs`, `tests_clone_form_state.rs`, the CE and
dom-api clone tests): every `Clone` outcome unchanged (I-RC1).

### §6.2 Mutation guards

| Mutant | Killed by |
|---|---|
| fragment event fired inside the build (before the backend's restore), so no consumer sees it | U-T10, U-T12 (and T3's T-22, T-22b) |
| fragment event for the first returned root only | U-T10, U-T12 |
| the `FragmentParse` cause asserting while `Taken` (D-4 made uniform) | U-T11 (a) (debug panic at the first push), U-T13 (and T3's T-15b, T-22c, T-26) |
| the `Clone` cause queued instead of asserted | U-T11 (d) |
| the `FragmentParse` event dropped while `Taken` (T3 rev 4.7's shape) | U-T10, U-T11 (a), U-T13 (and T3's T-26) |
| the restore function returning an empty list (the replaced `Taken`'s `pending` dropped at the replacement) | U-T10 (its outer-window half), U-T11 (a), U-T13 |
| the flush before the restore (dispatched into the `Taken` slot, so lost), or at a nested close | U-T11 (a), U-T13 |
| `close` not running the restore function, the restore left to the `Drop` at its end (ig's M-3), so `close` has no list to flush | U-T11 (a), U-T10 (its outer-window half), U-T13, in any build: nothing arrives (a debug build stops earlier, at ig's postcondition assertion) |
| the queue flushed on unwind, or not discarded at `clear_mutation_dispatcher` | U-T11 (b), (c) |
| a dead queued root dispatched (no `contains` check) | U-T11 (a) |
| `Drop` discarding silently on a returning path | U-T11 (e) |
| the serial fire after the placement | U-T12 |
| `LiveRangeBridge` acting on `SubtreeCreated` | none by outcome at T3-pre's head (no live range refers to a fresh subtree); held by DoD 4's grep (I-RC2) |

**Not claimed (R2).** (1) Rev 1's mutant "liveness checked once before the loop" is **equivalent under the
`MutationDispatcher` re-entry contract**: the only input that tells it from the per-root check is a consumer of an
earlier queued event despawning a later queued root, which the contract forbids (Q3×Q8), so no conforming cell
kills it and none is claimed. (2) Rev 1's mutants "the restore through `set_mutation_dispatcher`" and
"`set_mutation_dispatcher` on `Taken` without the assertion" mutate ig's code since PM 740 (2); they are ig's rows
(§8.2).

## §7. Out of scope and slots

| Item | Why not T3-pre | Home |
|---|---|---|
| the event-handler consumer's `SubtreeCreated` arm, its walks, D-T13 | PM decision 722 | T3 (T3 memo D-T10, D-T12, D-T13) |
| the custom-element upgrade at a fragment-parsed element's creation | L3's consumer; PM decision 711 (4) | `#11-custom-element-upgrade-at-fragment-creation` (L3; **registered**, PM 719) |
| RC-2b's clones on the event | L3's slice (PM decision 723) | the Range program, RC-2b (§8.2) |
| the restore function's direct slot write; `set_mutation_dispatcher`'s assertion while `Taken`, its cell and mutant | the PR that creates the restore function owns its invariant (PM decisions 740 (2), 741 (2)) | L3's PR (ig) (§8.2) |
| `Range.createContextualFragment` | not implemented (H-P6) | `#11-range-full-impl` (§4.3's record) |
| `FormControlState` at creation for a form control in template contents created by a post-bind fragment parse (filling the form `FragmentParse` arm; folding DOMParser's template-inclusive loop into it) | a base defect T3-pre neither causes nor changes (T3-pre is behaviour-neutral, §1.1); the arm it fills exists from T3-pre (§4.6); PM decision 767 (2) | `#11-form-control-state-template-contents-fragment-parse` (L3; **registered**, PM 767; Re-eval 2026-12-31; row text: §8.3 item 2) |

Own deferrals: **0** (the form-control slot is a base defect registered by the PM with owner L3, not a deferral of
T3-pre's scope).

## §8. Edges, hand-offs, records

### §8.1 Edges (umbrella §1.3, edited by this plan PR)

- **x13** L3 PR (ig) → T3-pre: code predecessor (T3-pre changes ig's `DispatcherTake::restore` and
  `DispatcherTake::close`, relying on §4.4's (a) to (f); PM decisions 713, 714, 722, 740). Since L3's order is is →
  ig, T3-pre also follows is.
- **x11** L3 RC-2a → T3-pre (was RC-2a → T3 until rev 8.<t>'s T3 memo rev 5): semantic precondition (the event and
  its `Clone` cause are RC-2a's; landed by #596).
- **x14** T3-pre → T3: code predecessor + semantic precondition (T3's arm matches `SubtreeCreated`; DOMParser's build
  reaches it only through the queue).
- **No edge** with L3's PR (io): merge contact in `ecs/dom/mod.rs` and `walkers.rs` docs only; whichever lands second
  re-pulls the other's coordinates.
- **No edge** with L3's tree-model program (PM 730: the shadow root becomes a tree root linked to its host by a pair
  of link components; owner L3; between ig and io). **Re-pulled in rev 2.2** from that program's umbrella, draft 2
  (`git show "1593a5ad:docs/plans/2026-11-tree-model-shadow-root-is-a-root-umbrella.md"`, §5 table, §5.1, §7.2; its
  census `2026-11-tree-model-census.md` at the same sha). Eight slices, in the order **tp → ta → tc → tr → tf → tm →
  te → ts**, then PR (io) rewritten on that base (its §5.1). Merge contacts with the files T3-pre edits (§5.3), by
  slice:
  - `crates/core/elidex-ecs/src/dom/mod.rs` (T3-pre: the slot, the fire function): `ta`, `tc`, `te`, `ts` (each
    lists `dom/mod.rs` as a primary module).
  - `ecs/dom/tree/walkers.rs` (T3-pre: docs only): `ta` and `ts` (`dom/tree/*.rs`); the census names `walkers.rs` 13
    times (e.g. `find_tree_root_composed`, `is_ancestor_or_self`, renamed or re-chosen in `ta`).
  - `crates/dom/elidex-dom-api/src/range/live.rs` (T3-pre: one name in the ignored-variant list): `tr` (`range/`);
    `ts` deletes the dead node half of the fire guards, whose parent half `LiveRangeRegistry` relies on (census role E,
    `live.rs:38-54`), not the line T3-pre edits.
  - `crates/dom/elidex-custom-elements/src/consumer.rs` (T3-pre: the arm at `:116`): `ta` (census: the
    `find_tree_root_composed` caller at `:408`).
  - `crates/script/elidex-script-session/src/mutation/html_fragment.rs` (T3-pre: U-T12 in its tests): `ta` (census:
    5 lines of role T1).
  - `crates/dom/elidex-dom-api/src/node_methods/clone.rs` (T3-pre: the clone fire's call, one line): the census's
    2 lines are `:115` (role I) and `:453` (role D), roles the census keeps (its C-5: "G, Ls, I and D stay"), so no
    slice edits it; a contact to re-check at the base (until rev 2.3 attributed to `ta`).
  - `ecs/dom/tree_clone.rs`: not named by the census (0 hits); left as a contact to re-check at the base.
  Command for the census side: for each file `f` of §5.3, `git show "1593a5ad:docs/plans/2026-11-tree-model-census.md"
  | /usr/bin/grep -c -F "$f"` (rev 2.2 counts: `dom/mod.rs` 11, `walkers.rs` 13, `node_methods/clone.rs` 2,
  `range/live.rs` 1, `consumer.rs` 1, `html_fragment.rs` 5, `tree_clone.rs` 0; re-run at L3's ratified sha, R-P3).
  T3-pre reads no parent link in production code and walks nothing, and §4.2's created set is stated by the DOM's
  terms, not by how elidex links a shadow root to its host, so the program changes no T3-pre statement; whichever lands
  second re-pulls the other's coordinates. (PM decisions 740 (1), 741 (1): T3-pre is not moved behind it.)
  **U-T13's parent assertion and `ta`.** `ta` deletes `get_parent`, "tests included" (its §5 row), replacing every
  caller by a relation chosen per its §1.1 (the tree parent, the shadow-including parent, or the transitional
  accessor `parent_crossing_pending_flip` where the base behaviour crosses, §1.4). U-T13 asserts that each returned
  root's parent is the synthesized `html` element: an element's child, never a shadow root, so the tree parent and
  the shadow-including parent are the same node and the assertion means the DOM parent. It is written against the
  relation, not the storage: if T3-pre lands first, U-T13 reads the parent through the base's `get_parent` and `ta`'s
  census of `get_parent` callers takes it, re-typed to the tree parent (no transitional accessor: the assertion does not
  cross, so it adds no line to `ta`'s pending-name trip-wire); if `ta` lands first, T3-pre's U-T13 uses `ta`'s
  tree-parent accessor directly. Either way the one who lands second re-pulls (the merge-contact rule above); the
  item is on §8.2's L3 list so `ta`'s census sees it. **L3's own escalation:** the tree-model umbrella §7.2 (*Layout
  T3 / T3-pre*) says L3 escalates to the PM if that program's landing would push T3 past the cap 2026-12-31; DoD item
  7 cross-references it and does not duplicate it.

### §8.2 Hand-offs

- **T3** (after T3-pre, io, T3-a and T3-b; DoD item 7): consumes `SubtreeCreated` in `EventHandlerAttributeConsumer`
  (T3 memo D-T10, D-T12).
- **The umbrella, through the T3 memo's Rev 8.<t>** (PM 767 (1); this plan PR): the umbrella's edge from T3-ab to T3
  becomes one edge from T3-a and one from T3-b, and where the umbrella states the order against io it refers to DoD
  item 7 for T3-a's and T3-b's condition (stated once here, not restated).
- **L3 PR (ig)** [L3 surface] (PM decisions 740 (2), 741 (2); R2). ig takes, in its own PR: the restore function's
  direct slot write (§4.4 (e)); `set_mutation_dispatcher`'s `debug_assert!` while `Taken` (§4.4 (f)); the cell for it
  (rev 1's U-T11 (f): an install inside an open window asserts) and its mutant rows (the restore through
  `set_mutation_dispatcher`; the assertion removed). Sentences of L3's plans at the snapshot that these decisions and
  PM 722 make stale, for L3 to reword (T3-pre edits none of L3's plans); each is re-pinned before T3-pre is ratified
  (R-P3):
  1. PR (ig) plan §4.1, the `restore` body (`let _ = self.dom.set_mutation_dispatcher(dispatcher);`) → the direct
     write.
  2. PR (ig) plan §4.1, "No new assertion is added; the slot behaviour on the return path is today's
     (`set_mutation_dispatcher` replaces whatever the slot holds, …)" → one assertion is added, and the restore does
     not go through `set_mutation_dispatcher`.
  3. PR (ig) plan §4.1 *Why it is unwind-safe* item 2, "the restore fn (one `mem::replace` through
     `set_mutation_dispatcher`)" → one `mem::replace` in the restore fn.
  4. "Layout T3" as the unit that adds the queue and the flush → T3-pre (PM 722): PR (ig) plan §4.1 `close`'s rustdoc
     ("Layout T3 adds, after that call, the dispatch of the queue …"), its comment "The postcondition T3's dispatch
     relies on", *Why one restore fn* ("T3 needs to restore the box and then dispatch its queue", "T3's dispatch after
     it"); §8 *T3 intersection* ("Layout T3 adds", "T3's insertion point", "the queue T3 adds", "Until T3 lands").
  5. PR (ig) plan §8 "T3 lands after PR (io) (umbrella §5 Order)" and the L3 umbrella §5 / §8 "Layout T3 lands after
     PR (io), since it depends on PR (io)'s `WalkScope` and on PR (ig)'s outermost-close flush point" → the flush's
     unit is T3-pre, which lands after ig and is not ordered against io; T3 lands after io for `WalkScope` (and after
     T3-pre).
  6. PR (ig) plan §7 M-3 ("once T3 dispatches inside `close` after the restore fn, T3's flush cell kills it in any
     build") → T3-pre's U-T11 (a) (§6.2's row for it).
  Not L3's to reword: `close`'s postcondition pattern `DispatcherSlot::Taken` → `Taken { .. }` and the restore's
  return type are T3-pre's own edits to ig's landed code (§4.4 *What T3-pre changes*).
- **L3 RC-2b** [L3 surface] (PM decision 723; R1 M17): RC-2b (`docs/plans/2026-11-range-contents-program.md` row RC-2b,
  on main, unlanded) was written against `CloneCreated` / `fire_clone_created`, which T3-pre replaces. It lands after
  T3-pre, on `SubtreeCreated { cause: Clone(scope) }` (PM decision 723), and relies on I-RC1 to I-RC3 (§2), which
  T3-pre keeps; T3-pre's migration set does not grow (RC-2b's own clones are its own edit).
- **L3's DOM-walker program, slice 1** (`#11-dom-walkers-iterative-over-ecs-links`; touches `walkers.rs`) (R1 M18;
  re-placed in rev 2.2): **after `ta`** (PM 761 (1): is → ig → tp → ta → walker slice 1, in parallel with `tc` onward;
  PM 694's "on PR (i), using `owned_entities`" kept), no longer "stacks on io". No ordering edge with T3-pre (T3-pre
  edits only `walkers.rs` docs); merge contact. Its edge with T3's creation walk is the T3 memo's (§8.2).
- **L3's tree-model slice `ta`** (R3 Axis 5): U-T13 (§6.1) reads a returned root's parent; if T3-pre lands before
  `ta`, that test line is a `get_parent` caller in `ta`'s census, re-typed to the tree parent (§8.1). For L3's
  information; no rewording of L3's plans.
- **T3's landing list** (R3 Axis 5; for the T3 memo): `#11-eh-attrs-creation-time-call-sites`'s ledger
  *Trigger* text, which at T3-pre's writing still reads "T3 and N1 landed, and C1's handling decided" while the
  umbrella §8.2 reads "N1, C0, C1, T3-pre and T3 landed", is brought to the umbrella's text (DoD item 7's bound).
  (Until rev 2.3 the list also carried the custom-element arm's comment rewrite; §4.6 *No pointer is rewritten*
  removes it.)
- **`#11-custom-element-upgrade-at-fragment-creation`** (L3, registered): the arm it adds matches T3-pre's
  `SubtreeCreated { cause: FragmentParse }`. Its ledger *Trigger* is T3-pre's landing (PM 732); the event exists from T3-pre's
  (§10 Q-P1, closed). The text appended to its row: §8.3 item 4.

### §8.3 Records for existing slots (for the PM, exact texts; at T3-pre's landing)

1. `#11-range-full-impl` — append: "Since T3-pre (#<PR>, `<squash>`), every production fragment parse fires
   `MutationEvent::SubtreeCreated { cause: FragmentParse }` from `elidex_html_parser::parse_fragment_progressive`, the
   one function both parser backends return through. `Range.createContextualFragment`'s implementation must parse
   through it (or the session's `apply_*` wrappers that call it), or its elements get no creation processing (T3-pre
   memo §4.3)."
2. `#11-form-control-state-template-contents-fragment-parse` (registered by PM 767; owner L3; Re-eval-date
   2026-12-31) — the row's text, in the shape defect / spec / where / related root / closing cells (PM 774; rev 2.4),
   replacing its body after the registration header:
   "**Defect.** A form control that elidex gives `FormControlState` (among HTML §4.10.2's *resettable elements*:
   `input`, `output`, `select`, `textarea`, form-associated custom elements, the last excluded by step 24) inside the
   template contents (HTML §4.12.3) of a `template` that a post-bind fragment parse creates (`innerHTML`, `outerHTML`,
   `insertAdjacentHTML`, `setHTMLUnsafe`, the queued `Mutation::SetInnerHtml`) never receives `FormControlState`, so
   its value and checkedness are not initialised from its attributes: `FormControlReconciler::handle_insert` walks only
   `for_each_shadow_inclusive_descendant` (`crates/dom/elidex-form/src/reconciler.rs:89-99` @ `cee74989`), which does
   not enter template contents, and no other path reaches them. The same element built by `DOMParser.parseFromString`
   does get it, from a separate loop over `collect_template_inclusive_descendants`
   (`crates/dom/elidex-form/src/inert_document.rs:162-174`): two paths, two answers. **Spec.** HTML §13.2.6.1 *create
   an element for a token* step 24: a resettable element that is not a form-associated custom element has its reset
   algorithm (HTML §4.10.23) invoked at creation, which initialises its value and checkedness from its attributes; this
   holds for elements the parser creates in template contents too. For clones the form *cloning steps* (HTML §4.10.5
   `input`, §4.10.11 `textarea`, run from DOM §4.4 *clone a node* step 3) already apply through the `Clone(scope)`
   arm. **Where the fix goes.** The `FragmentParse` arm of
   `FormControlReconciler` (a no-op since T3-pre, #<PR> `<squash>`), with DOMParser's separate template-inclusive loop
   collapsed into it (PM 767; One issue, one way). It comes after T3-pre and does not need T3. **Related root.** L3's
   form-derived-state reconciliation (`docs/plans/2026-07-form-derived-state-reconciliation.md`): while
   `FormControlState` caches effective disabledness, a write at creation could freeze state that depends on the
   element's ancestors; the reconciliation's `is_effectively_disabled` pull removes that question. A fact the fix's
   plan must consider: DOMParser's build reaches the arm only through the window's queue (T3-pre memo §4.4), which is
   dispatched only when a mutation dispatcher was installed as the window opened. **Closes when** these cells pass on
   the bound VM: (a) `d.innerHTML = '<template><input value=x></template>'; d.firstChild.content.firstChild.value ===
   "x"` (at `cee74989` the `value` getter returns `""` without `FormControlState`,
   `crates/script/elidex-js/src/vm/host/html_input_value.rs:62-65`); (b) with `f` a connected `<fieldset disabled>`,
   `f.innerHTML = '<input>'; f.firstChild.matches(':disabled')` stays `true`; (c) `new
   DOMParser().parseFromString('<input value=x>', 'text/html').body.firstChild.value === "x"` (a control outside any
   template keeps the base behaviour); (d) the same through `DOMParser` inside a `template` stays `"x"`. The mechanism,
   and its order against the reconciliation's slices, is chosen in this slot's own plan under its own plan-review."
   Appended to the same row (T3-pre's record, not the defect; same shape as item 4's): "**T3-pre sentences the fix
   makes false** (rewritten in the fix's PR): the form arm's comment (replaced with the arm); the sentence T3-pre adds
   to `parse_into_inert_document`'s rustdoc (`crates/dom/elidex-form/src/inert_document.rs`, "At T3-pre's head no
   consumer acts on it …"), since the form arm then acts on the build. If the window is still open when the fix
   lands, the fix ends it (T3-pre memo DoD item 7), and the window condition on
   `#11-eh-attrs-creation-time-call-sites`'s row is cleared."
3. `#11-eh-attrs-creation-time-call-sites` (the umbrella's slot; owner Layout) — append the window condition and its
   escalation, so they outlive T3-pre's landing (R3 Axis 3). This text is the one statement of the escalation (rev
   2.3; DoD item 7 points here): "**Window condition (T3-pre memo DoD item 7; PM 756, 767 (1)).** From T3-pre's
   landing (#<PR>, `<squash>`) the `FragmentParse` creation event reaches no acting consumer until the window ends, at
   the first of T3's landing, the fix of `#11-custom-element-upgrade-at-fragment-creation` and the fix of
   `#11-form-control-state-template-contents-fragment-parse` (T3-pre memo DoD item 7). In the planned order T3 ends
   it, and it keeps its planned length only if **T3-a and T3-b land before io lands**. **Raise to the PM** (raiser:
   the Layout lane, which owns T3-a, T3-b and T3; backstop: the PM pass, checking the same against L3's lane SSoT) at
   the first of these while T3-a or T3-b is not merged: (t1) io's per-PR plan enters `/elidex-plan-review` and T3-a's
   or T3-b's plan is not ratified; (t2) io's implementation PR is opened; (t3) a 'merge at head' is requested for io.
   Each is observable in L3's lane SSoT, the PORTFOLIO or `gh pr view`; the Layout lane reads L3's lane SSoT for io's
   state at each of its own session starts and at each T3-a / T3-b plan-review or landing step. Also raise if the
   window grows otherwise (a unit other than T3-a / T3-b placed before T3, or T3 re-sliced so that it no longer lands
   right after io) or the window is still open at this row's Re-eval date. The raise asks the PM to hold io's merge
   until T3-a and T3-b land, to re-prioritise them in the Layout lane, or to accept a longer window with a new bound.
   L3's own duty for its tree-model program is that program's umbrella §7.2. Cleared when the window ends."
4. `#11-custom-element-upgrade-at-fragment-creation` (L3; registered by PM 719, *Trigger* T3-pre's landing by PM 732)
   — append (rev 2.3): "Since T3-pre (#<PR>, `<squash>`), the arm this fix fills is `CustomElementReactionConsumer`'s
   `SubtreeCreated { cause: FragmentParse }` arm, a no-op whose comment names only this slot and the window (T3-pre
   memo §4.6). **T3-pre sentences the fix makes false** (rewritten in the fix's PR): the arm's comment (replaced with
   the arm). If the window is still open when the fix lands, the fix ends it (T3-pre memo DoD item 7), and the window
   condition on `#11-eh-attrs-creation-time-call-sites`'s row is cleared. `parse_into_inert_document`'s rustdoc
   sentence stays true: DOMParser's build enqueues no upgrade (its document's registry is null)."

## §9. Definition of done

1. T3-pre's branch is cut from main after L3's PR (is) and PR (ig) land (x13); every `cee74989` coordinate in §0.2 is
   re-measured there by its command (H-P1, all three parts of its population, (iii) expecting 0; §4.5's anchors at
   ig's head, with DOMParser's window body read against §4.2's invariant (between the fire and the close, no step
   changes what hangs from a returned root); sizes of §5.3); every L3 citation is re-pinned to L3's ratified sha (R-P3), §8.2's reworded sentences
   included. **What ig provides is checked on ig's landed code before T3-pre's first commit** (these are ig's, so
   they are checks here, not T3-pre's greps): `git grep -n 'set_mutation_dispatcher(' --
   crates/core/elidex-ecs/src/dom/dispatcher_take.rs` → 0 (§4.4 (e): the restore writes the slot directly);
   `set_mutation_dispatcher`'s body debug-asserts on `Taken` and a cell of ig's pins it (§4.4 (f)); `close` runs the
   private restore function and then its postcondition assertion (§4.4 (c), (d)). T3-pre stops for the PM on R-P3's
   stop condition and adds none of these itself.
2. `/elidex-plan-review` passes on this memo (mandatory, §1.3), with L3's confirmation of the [L3 surface] sections
   through the PM (PM decision 722).
3. Red-first: U-T10, U-T11 (a)–(c), (e), U-T12, U-T13 committed failing; the fix makes them green; U-T11 (d) and
   the renamed RC-2a cells green at the head.
4. Structural greps, **each over a surface T3-pre owns** (what ig provides is item 1's):
   - the event: `git grep -n -e CloneCreated -e fire_clone_created -- crates` → 0; `git grep -n
     'fire_subtree_created(' -- crates ':!*tests*'` → the definition and two calls (the dom-api cloner,
     `parse_fragment_progressive`; I-RC1);
   - the payload: `git grep -n -w 'Taken' -- crates/core/elidex-ecs/src/dom ':!*tests*'` → every construction is
     `Taken { pending: … }` and every pattern `Taken { .. }` or one that binds `pending` (the compiler enforces it;
     the grep shows the list);
   - the restore's return value and the flush: `git grep -n -e 'fn restore' -e 'self.restore()' --
     crates/core/elidex-ecs/src/dom/dispatcher_take.rs` → the definition returns `Vec<Entity>`, and its two calls are
     in `close` (the list flushed) and in `Drop` (the list asserted empty unless panicking);
   - the backend entries (§4.3): `git grep -n -e 'parse_html_fragment(' -e 'parse_fragment_strict(' -- crates
     ':!*tests*'` → 4 lines, the two definitions (`parse_html_fragment` now `pub(crate)`) and the two calls in
     `parse_fragment_progressive`;
   - the consumers: `git grep -n 'SubtreeCreated' -- crates/dom/elidex-dom-api/src/range/live.rs` → its
     ignored-variant list only (I-RC2); `git grep -n 'SubtreeCreated' --
     crates/script/elidex-script-session/src/event_handler_consumer.rs
     crates/script/elidex-script-session/src/event_handler_processing` → 0 (the arm is T3's; the session's
     `mutation/html_fragment.rs` names the event in U-T12, so the crate as a whole is not 0).
5. `cargo test -p elidex-ecs -p elidex-html-parser -p elidex-dom-api -p elidex-custom-elements -p elidex-form -p
   elidex-script-session -p elidex-js --all-features` green; `mise run ci` before push.
6. §8.3's records (items 1 to 4) handed to the PM in the PR description.
7. **The no-op window of the `FragmentParse` arms (the one statement of the window; PM decision 722's dead-code
   condition (2), read by PM 740 (1) as "until T3 lands" and restated in rev 2.3 by its first end; PM 741 (1)).** From
   T3-pre's landing the `FragmentParse` cause reaches no acting consumer until the window ends, at the **first** of:
   (w1) T3's landing (the event-handler consumer's arm, T3 memo D-T12); (w2) the fix of
   `#11-custom-element-upgrade-at-fragment-creation` (the custom-element arm); (w3) the fix of
   `#11-form-control-state-template-contents-fragment-parse` (the form arm). Each gives the cause an acting arm. (w2)
   and (w3) are not ordered against T3 (the custom-element slot's *Trigger* is T3-pre's landing, PM 732; the form slot
   does not need T3, §8.3 item 2), so they can only shorten the window. **Sequence (the planned order, in which T3
   ends it):** ig → T3-pre → L3's tree-model program (PM 730) → io → T3; T3's
   implementation is cut once T3-pre and io have landed (umbrella x12, x14). **Why it is approved (PM 740 (1)):**
   T3-pre has live consumers from its own landing (the generalised event with every consumer migrated, and RC-2b's
   `Clone` cause, PM decision 723), and the window changes no behaviour (§1.1); delaying T3-pre to just before io
   would stop those too (not chosen, PM 740 (1), 741 (1)). **Upper bound (PM 741 (1)):** T3's, since T3 ends the window
   in the planned order; the working bound is the
   Re-eval date **2026-12-31** of the umbrella's slot `#11-eh-attrs-creation-time-call-sites`. (Its *Trigger* in the
   umbrella §8.2 is "N1, C0, C1, T3-pre and T3 landed"; the ledger row still reads "T3 and N1 landed, and C1's
   handling decided", and bringing it to the umbrella's text is an item of T3's landing list, §8.2; so the date, which
   both texts share, is the bound cited here.) **T3-a and T3-b (PM decisions 756, 767 (1)).** T3-ab, the prerequisite
   slice that PM decision 741 (3) placed before T3, is split into **T3-a** (D-T8, the applicability predicate,
   script-session) and **T3-b** (D-T11, the body prototype, VM host) (R3 Axis 3; PM 767 (1)); each is a predecessor of
   T3 in the umbrella §1.3, depends on nothing unlanded and is not in the sequence. The window keeps its length on the
   condition that **T3-a and T3-b land before io lands**; if either is unlanded at io's landing, T3's landing, and so
   the window's close, is delayed. **Escalation (raised before the condition is missed, not after):** stated once, in
   §8.3 item 3's ledger text (its triggers t1–t3, its raiser and backstop, the cadence, what the raise asks the PM,
   and the cross-reference to L3's own duty in the tree-model umbrella §7.2, draft 2, `1593a5ad`), which outlives
   T3-pre's landing; this item does not restate it. The umbrella's own record of the condition is the T3 memo's edit
   (handed off, §8.2). **Checked at T3-pre's head:** (a) §8.3 item 3's text is handed to the PM (item 6); (b) the shipped
   comments name no window end: `for f in crates/dom/elidex-custom-elements/src/consumer.rs
   crates/dom/elidex-form/src/reconciler.rs crates/dom/elidex-form/src/inert_document.rs; do /usr/bin/grep -c -e
   'until T3' -e 'T3 lands' $f; done` → 0, 0, 0 (0, 0, 0 at `cee74989` too). Positive control of the same expression
   on a known hit: `/usr/bin/grep -c -e 'until T3' -e 'T3 lands'
   docs/plans/2026-11-eh-attrs-t3-pre-subtree-created-event.md` → at least 1 (12 at `10af92ee`, 13 at rev 2.4; rev
   2.4, F6). Control that the pointer is shipped: `/usr/bin/grep -c -F 'T3-pre memo DoD item 7'` over the same three
   files → at least 1 each (the common part, and the rustdoc sentence; the pointer is written on one source line).
   (Until rev 2.3: the window ran "until T3's landing", and the triggers, raiser, cadence and the raise's ask were
   stated here in full as well as in §8.3 item 3.)
   (Until rev 2.2: "T3-ab lands before io lands", with "at risk" undefined; until rev 2.1: "PM 741 (3) puts a
   prerequisite slice of T3 before T3; whether it lengthens the window is judged against this condition when that
   slice is planned, not here.") Each `FragmentParse` arm's comment names its own slot and points here (§4.6).
8. **Checker** (run; do not trust a stored verdict): `python3 .claude/skills/elidex-plan-review/preflight.py
   docs/plans/2026-11-eh-attrs-t3-pre-subtree-created-event.md` exit 0.

## §10. Open questions

**Decided:** the split (PM decision 722); the flush shape (δ) (PM 720, P-P1); RC-2b's order and the three properties
(PM decision 723); the order, unchanged, and the no-op window (until the first of T3's landing and the two slots' fixes, rev 2.3), with its bound and escalation, and T3-a's
and T3-b's condition on it (PM decisions 740 (1), 741 (1), 756, 767 (1); DoD item 7); the form `FragmentParse` arm as
the location of the fix of `#11-form-control-state-template-contents-fragment-parse`, its mechanism left to that slot's plan (PM 767 (2), 774; §4.6, §8.3 item 2); the owner of R1 M2's two remedies, the restore function's direct slot write
and `set_mutation_dispatcher`'s assertion while `Taken`: ig (PM decisions 740 (2), 741 (2); P-P2). T3-pre's own, within
PM 713 / 720 / 740 (not new decisions): the restore function returns `pending` from its one replacement; `close`
flushes after it; `Drop` asserts an empty list unless unwinding (§4.4). L3 confirmed the [L3 surface] sections of rev 1
(11 items without finding; 4 IMP and 7 MIN folded in rev 2).

**For the PM:**
- **Q-P1 (closed).** `#11-custom-element-upgrade-at-fragment-creation`'s ledger *Trigger* reads "T3 の着地"; the event its arm
  needs exists from T3-pre's landing. Whether the trigger moves to T3-pre's landing is the PM's and L3's call (no
  T3-pre text depends on it). **Answered by PM 732:** the trigger = T3-pre's landing (ledger updated by pmo).

No browser measurement is needed: no outcome changes (§1.1).
