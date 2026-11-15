# T3-pre `EH-attrs-at-creation-T3-pre` — one creation event `SubtreeCreated`, fired by the cloner and the fragment parse, queued in a dispatcher-take window (per-PR plan-memo)

Nested PR **T3-pre** of the standalone program `EH-attrs-at-creation` (umbrella
`docs/plans/2026-10-event-handler-attrs-at-creation.md`, "the umbrella"; its in-place edits for this PR are carried by
the T3 memo's *Rev 8.<t>* entry, since both memos land in one plan PR, PM decision 722). It is the prerequisite of
**T3** (`docs/plans/2026-10-eh-attrs-t3-target-determination.md`, "the T3 memo", rev 5), split out of T3 rev 4.8.2 by
**PM decision 722** (plan-review R1 F1 on T3 rev 4.8.2 `571cd9c8`: the queue / window cluster is edge-dense on its own,
§1.3). It carries the creation event and its queue; T3 carries the event-handler consumer of it.

**Owner: Layout lane (L2)** writes and implements T3-pre (PM decision 722). **The surfaces are L3's**: the dispatcher
slot (`DispatcherSlot`, `EcsDom::set_mutation_dispatcher` and its siblings), RC-2a's `MutationEvent::CloneCreated` and
its fire function, L3's dispatcher-take guard (PR (ig)), and the custom-element and form consumers. **T3-pre's
plan-review requires L3's confirmation, through the PM, of the sections marked [L3 surface]** (as PM decision 711 did
for T3 rev 4.7's §4.5): §2's queue/window pairs and I-RC1 to I-RC3, §4.2 to §4.6, §6.1's U-T11, §8.2's RC-2b entry.

**Placement and order (PM decisions 722, 723).** Row **T3-pre** of the umbrella's §1.2 (owner Layout; added by this
plan PR). Edges (umbrella §1.3): **x13** L3 PR (ig) → T3-pre (the flush is T3-pre's change to ig's
`DispatcherTake::close`, so ig's guard must exist); **x11** L3 RC-2a → T3-pre (the event T3-pre generalises; landed);
**x14** T3-pre → T3 (T3's consumer arm matches `SubtreeCreated`). **No edge with L3's PR (io)**: T3-pre does not use
`WalkScope`; the two may land in either order (merge contact only, §5.3). T3 lands after both T3-pre and io (umbrella
x12, x14). L3's Range **RC-2b** lands after T3-pre and is written against `SubtreeCreated { cause: Clone(scope) }`
(PM decision 723; §8.2, I-RC1 to I-RC3).

**Base.** main when T3-pre's implementation branch is cut, after L3's PR (is) and PR (ig) land: `cee74989` (#626, the
T3 memo's frame) + is + ig, plus io if it landed first. Every coordinate below is a `cee74989` coordinate and is re-run
at the base (§9 item 1).

**Scope.**
- `MutationEvent::CloneCreated { root, scope }` is replaced, with no coexistence, by `MutationEvent::SubtreeCreated
  { root, cause }` (NEW) with `CreationCause` (NEW) = `Clone(CloneScope)` | `FragmentParse` (NEW), fired by
  `EcsDom::fire_subtree_created` (NEW), which replaces `EcsDom::fire_clone_created` (D-P1, §4.2; PM decision 705).
- Every consumer migrates (the custom-element, form and live-range consumers, the dom-api test probe, docs and tests;
  §4.6). The event-handler consumer's arm is **not** here: it is T3's (T3 memo D-T12; PM decision 722).
- The `FragmentParse` fire in `elidex_html_parser::parse_fragment_progressive` (D-P2, §4.3; PM decision 705).
- The window's queue: `DispatcherSlot::Taken { pending }`, the one restore function of ig's guard, the flush at the
  outermost `close`, the discard on unwind (D-P3, §4.4; PM decisions 711 (1), 713 (2), 720), with every writer of the
  slot defined per state (plan-review R1 M2) and a debug assertion against a silent discard on a returning path (M3).
- Cells U-T10 and U-T11 (ids kept from the T3 memo rev 4.8.2) and new U-T12, U-T13 (§6).

**Not in scope.** What the event-handler consumer does with the event (T3: the `SubtreeCreated` arm,
`process_created_subtree`, the walks D-T10 / D-T12, D-T13, the `inert_document.rs` rustdoc fix); the custom-element
upgrade at a fragment-parsed element's creation (`#11-custom-element-upgrade-at-fragment-creation`, L3, registered by
PM 719; §7); RC-2b's use of the event (L3, PM decision 723).

**Changelog.** *Rev 1* (2026-11-13): new memo, PM decision 722. Moved from the T3 memo rev 4.8.2 without restating
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
body` output of 2026-11-13; every `§X.Y = title` pair is `webref heading` / `dfn` output (§3's footnote).
**L3's names** are cited by symbol and section from L3's split plans at the one pre-ratification snapshot H-P5 names
(`6d31ced1`, L3's local plan branch, read only through `git show`): "the PR (ig) plan"
(`docs/plans/2026-11-ecs-entity-lifetime-pr-ig-dispatcher-take-guard.md`) and "the L3 umbrella"
(`docs/plans/2026-11-ecs-entity-lifetime-umbrella.md`). From ig: `DispatcherTake` (NEW: ig; PR (ig) plan §4.1, "the
one way to take the mutation dispatcher out of an `EcsDom`"), its consuming `DispatcherTake::close` (NEW: ig) and its
`Drop`. Every L3 citation is re-pinned to L3's ratified sha before T3-pre is ratified (R-P3). "The RC-2a plan" is
`docs/plans/2026-11-range-rc2a-clone-created.md` (on main, #589; implementation #596), cited by section.

### §0.2 Premises re-measured

| ID | Premise | Command | Result at `cee74989` |
|---|---|---|---|
| H-P1 | (R1 M2) The slot and every writer of it | `git grep -n -e 'self.dispatcher' -e 'dispatcher = ' cee74989 -- crates/core/elidex-ecs/src`; `git grep -n -e 'set_mutation_dispatcher(' -e 'take_mutation_dispatcher(' -e 'clear_mutation_dispatcher(' cee74989 -- crates ':!*tests*'`; `git show cee74989:crates/core/elidex-ecs/src/dom/mod.rs \| sed -n 51,59p`, `sed -n 117,156p`, `sed -n 190,270p` | `enum DispatcherSlot { Absent, Installed(Box<…>), Taken }` (`ecs/dom/mod.rs:55-59`, private, in the private field `dispatcher`). Writers: `set_mutation_dispatcher` (`:196`, `pub`; `mem::replace` with `Installed(d)`, then `Absent \| Taken => None`, `:200-203`), `clear_mutation_dispatcher` (`:208`, `pub`; `Absent`), `take_mutation_dispatcher` (`:221`, `pub`; `Installed` → `Taken`, else unchanged), `dispatch_event` (`:247`, private; `Installed` → `Absent` for the dispatch, `Absent` / `Taken` left as they are, `:253-259`), `RestoreDispatcher::drop` (`:140-155`; writes `Installed(d)` back, debug-asserting `!Installed \|\| std::thread::panicking()` on what it displaced). Production restores of a take window call `set_mutation_dispatcher`: `ecs/dom/tree/teardown.rs:256`, `crates/dom/elidex-html-parser/src/lib.rs:157`, `crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs:241`, `crates/dom/elidex-form/src/inert_document.rs:203`; the one production install is `Vm::bind` (`crates/script/elidex-js/src/vm/vm_api.rs:325`, debug-asserting nothing was displaced), the one production clear is unbind (`crates/script/elidex-js/src/vm/lifecycle.rs:34`). Reader only: `teardown.rs:91` |
| H-P2 | The clone fire: one production site, once per clone call | `git grep -n -e 'fire_clone_created(' cee74989 -- crates ':!*tests*'`; `git show cee74989:crates/dom/elidex-dom-api/src/node_methods/clone.rs \| sed -n 235,312p`; `sed -n 325,342p` of `ecs/dom/mod.rs` | `clone_node_with_shadow_honor` (`crates/dom/elidex-dom-api/src/node_methods/clone.rs:235`) fires `MutationEvent::CloneCreated { root, scope }` once, after the whole copy exists (template contents and clonable shadow trees replicated) and before it returns (`clone.rs:305-311`); `EcsDom::fire_clone_created` (`ecs/dom/mod.rs:333`) debug-asserts that the slot is not `Taken` (`:334-340`; the RC-2a plan §4.7, D-4) and dispatches through `dispatch_event` |
| H-P3 | (T3 rev 4.7's H-T31, census restated) The `CloneCreated` population | `git grep -n -e CloneCreated -e fire_clone_created cee74989 -- crates \| wc -l` and `git grep -l -e CloneCreated -e fire_clone_created cee74989 -- crates \| wc -l`; `git grep -n CloneCreated cee74989 -- crates \| wc -l` and `git grep -l CloneCreated cee74989 -- crates \| wc -l` | **73 lines in 19 files** naming the variant (substring `CloneCreated`, so `CloneCreatedProbe` too) **or** `fire_clone_created`; of them **50 lines in 18 files** contain `CloneCreated` (the 19th file, `ecs/dom/tests/dispatch_guard.rs`, names only the fire function) (verified 2026-11-13). Match arms: `crates/dom/elidex-custom-elements/src/consumer.rs:116` (`CustomElementReactionConsumer`), `crates/dom/elidex-form/src/reconciler.rs:53` (`FormControlReconciler`), `crates/dom/elidex-dom-api/src/test_util.rs:53` (test probe `CloneCreatedProbe`); ignored-variant list `crates/dom/elidex-dom-api/src/range/live.rs:160` (`LiveRangeBridge`); fire: definition `ecs/dom/mod.rs:333`, one production call `clone.rs:305`. `EventHandlerAttributeConsumer::handle` matches `AttributeChange` and `Insert` and has a catch-all (`crates/script/elidex-script-session/src/event_handler_consumer.rs:381-391`); so do `node_iter`, `base_url`, `canvas` (`crates/script/elidex-js/src/vm/consumer_dispatcher.rs:157-163`) |
| H-P4 | (T3 rev 4.7's H-T29 / H-T30) The fragment build's own window; one function returns every fragment parse; DOMParser's window encloses one | `git show cee74989:crates/dom/elidex-html-parser/src/lib.rs \| sed -n 140,160p` and `sed -n 207,247p`; `git show cee74989:crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs \| sed -n 165,245p`; `git grep -n 'parse_fragment_progressive(' cee74989 -- crates`; `git show cee74989:crates/dom/elidex-form/src/inert_document.rs \| sed -n 108,205p`; `sed -n 221,232p` of `ecs/dom/mod.rs` | tolerant: `begin_detached_fragment` takes (`lib.rs:143`; `ecs/dom/tree/teardown.rs:308`), restore `lib.rs:156-158`; strict: take `tree_builder/mod.rs:180`, restore on both result arms `:238-241`. `parse_fragment_progressive` (`lib.rs:223`) is the one dispatch site of both backends (strict; on reject, tolerant over the pristine dom, `:229-246`), called only from `crates/script/elidex-script-session/src/mutation/html_fragment.rs` (`:93`, `:174`, `:209`, `:218`, `:235`, `:245`), which places the returned nodes after it returns. Entry points: `innerHTML` on Element / ShadowRoot / template, `setHTMLUnsafe`, `outerHTML`, `insertAdjacentHTML`, the queued `Mutation::SetInnerHtml`, DOMParser. `parse_into_inert_document` takes (`inert_document.rs:112`), calls `apply_set_inner_html` (`:142`), runs the form-control (`:172-174`) and base-URL (`:200`) reconcilers, restores (`:202-203`); inside it `begin_detached_fragment`'s take finds `Taken` and opens nothing (`ecs/dom/mod.rs:224-229`), so the slot is still `Taken` when `parse_fragment_progressive` returns there |
| H-P5 | (T3 rev 4.8.1's H-T32, its PR (ig) half) L3's guard, as planned (snapshot `6d31ced1`) | `S=6d31ced1; P=docs/plans/2026-11-ecs-entity-lifetime`; `git show $S:$P-pr-ig-dispatcher-take-guard.md \| /usr/bin/grep -n -e 'pub fn close' -e 'one site of the restore' -e 'does nothing that can panic' -e 'set_mutation_dispatcher(dispatcher)' -e 'window.close();' -e '^\*\*T3 intersection' -e 'This PR guarantees only'`; `git show $S:$P-umbrella.md \| /usr/bin/grep -n -e 'Layout T3 lands after PR (io)'` | PR (ig) plan §4.1: `DispatcherTake<'a> { dom: &'a mut EcsDom, taken: Option<Box<…>> }`, held by all four take windows; `open` calls `take_mutation_dispatcher` (made `pub(in crate::dom)`); `close(self)` is "the explicit restore point on a returning path … `Drop` stays the unwind backstop and the one site of the restore" (body `drop(self)`); `impl Drop`: `if let Some(dispatcher) = self.taken.take() { let _ = self.dom.set_mutation_dispatcher(dispatcher); }`; *Why it is unwind-safe* item 2 "`Drop` does nothing that can panic: one `mem::replace` through `set_mutation_dispatcher`", item 3 a nested guard holds nothing; *The restore point*: the outermost guard's close or drop; the parse windows and `parse_into_inert_document` call `window.close()` on their returning path. §8 *T3 intersection*: T3 (now T3-pre, PM decision 722) adds the queue in `Taken` and the flush at the outermost guard's `close` (restore, then dispatch in order skipping non-live roots; on unwind restore only and discard); ig guarantees only (a) the restore point is the outermost guard's `close` and (b) the slot state is the single home of take/restore state. L3 umbrella §5 *Order*: is → ig → io → ii. Announced after the snapshot (PM 718), not relied on: `TeardownWindow` gains a consuming close; `is_outermost` is removed |
| H-P6 | (R1 M8) No other production fragment-creating path | `git show cee74989:crates/dom/elidex-dom-api/src/range/mutation.rs \| sed -n 640,660p`; `git grep -n -i -e 'parseHTMLUnsafe' -e 'document_write' -e 'fn write(' cee74989 -- crates/dom crates/script/elidex-js/src/vm/host` | `Range.createContextualFragment` is a stub that parses nothing (`crates/dom/elidex-dom-api/src/range/mutation.rs:648`); its rustdoc names the parser boundary it is to call ("through `elidex-script-session::mutation` (the canonical parser boundary)", `:640-641`) and the slot `#11-range-create-contextual-fragment`, which the ledger closed into `#11-range-full-impl` (2026-05-15 sweep; home: the Range program's RC-4). The grep → no hits: no `document.write`, no `parseHTMLUnsafe` |
| H-P7 | (PM decision 723) What RC-2b relies on | `git show origin/main:docs/plans/2026-11-range-contents-program.md \| /usr/bin/grep -n -e '^\| M × C' -e '^\| L × C' -e '^\| RC-2b'` | the Range program (on main, unlanded slice RC-2b): every clone root fires the clone-creation event from the canonical cloner entry, "between the mutations, in step order" (§2 *M × C*); "the bridge ignores `CloneCreated`" (§2 *L × C*); RC-2b clones through the canonical entry, one event per clone root (§1 row RC-2b) |

### §0.3 Premise errors (reported)

- **P-P1 (T3 rev 4.8.1's P-T20, moved; decided by PM 720).** PM decision 711 named the flush point "PR (i)'s consuming
  close"; the pre-split PR (i) plan had none. The PR (ig) plan has one (`DispatcherTake::close`, H-P5) and assigns the
  flush in it to T3 (now T3-pre). Its §4.1 makes `Drop` "the one site of the restore" with `close` = `drop(self)`, and
  the guard owns the `&'a mut EcsDom` borrow, so "restore in `Drop`, then dispatch from `close`" cannot be written as
  planned. PM 720 chose (δ): one private restore function, called by `close` and by `Drop` (§4.4). *Not chosen:* (α) a
  restore written in `close` as well as `Drop` (two restore sites); (β) dispatch in `Drop` after a `close` hand-off
  (`Drop` would run consumer code, so "does nothing that can panic" would hold on the unwind path only); (γ) dispatch by
  each window's caller after `close` returns the list (one flush per window site). L3 rewords the PR (ig) plan §4.1
  from "`Drop` … the one site of the restore" to "the restore function is the only one" (PM 720).
- **P-P2 (new, R1 M2).** The snapshot's `Drop` restores **through `set_mutation_dispatcher`** (H-P5), whose
  `mem::replace` returns the replaced `Taken` and drops it (`Absent | Taken => None`, H-P1). With `Taken { pending }`
  that call would drop the queue, so the restore function cannot be "today's `Drop` body" verbatim (T3 rev 4.8.2's
  wording): it writes the slot itself in one replacement and returns the replaced `pending` (D-P3). And
  `set_mutation_dispatcher` itself, `pub`, can meet `Taken { pending }` from any caller; it must not drop a non-empty
  queue silently (D-P3).

### §0.4 Decisions (this PR)

- **D-P1 — one creation event, generalised (PM decision 705).** `MutationEvent::CloneCreated` becomes
  `MutationEvent::SubtreeCreated { root, cause }`; no second variant beside it (one issue, one way). §4.2.
- **D-P2 — two fire points, the cloner's unchanged (PM decisions 705, 723).** `Clone(scope)` at the cloner's existing
  site; `FragmentParse` once per returned root in `parse_fragment_progressive`, after the backend's window has closed.
  §4.3.
- **D-P3 — the window's queue lives in the slot; one restore function; one flush point (PM decisions 711 (1), 713 (2),
  720; R1 M2, M3).** §4.4 is the one statement of the queue and of (δ); every other site of this memo, of the T3 memo
  and of the umbrella refers to it.
- **D-P4 — every consumer migrates; the `FragmentParse` arms that do nothing say who acts on the cause (PM decision 722's
  dead-code condition).** §4.6.
- **D-P5 — the `Clone` cause keeps RC-2a's contract (PM decision 723).** I-RC1 to I-RC3 (§2), which RC-2b relies on.

## §1. Shape and verdict

| Axis | Own invariant |
|---|---|
| Q1 event shape | one variant, two causes (D-P1) |
| Q2 fire sites | the cloner (unchanged) and `parse_fragment_progressive` (D-P2) |
| Q3 window lifecycle | the slot's `Taken { pending }` is the window's only state; one restore function; flush at the outermost `close` (D-P3) |
| Q4 placement order | serial: the event precedes the caller's placement; inside DOMParser's window: it follows the placement and the structural reconcilers (D-P2) |
| Q5 unwind | `Drop` without `close` restores and discards; no consumer code during unwind (D-P3) |
| Q6 nesting | only the guard that took restores and flushes (D-P3; ig's guarantee (a)) |
| Q7 unbind clear | `clear_mutation_dispatcher` discards `pending` with the window (D-P3) |
| Q8 root liveness | evaluated per root, at dispatch time, after the restore (D-P3) |
| Q9 slot writers | every writer defined per state (D-P3, H-P1) |
| Q10 consumers | every consumer migrated; `FragmentParse` acted on by T3's arm only (D-P4) |

### §1.1 Outcome at T3-pre's landing

Behaviour-neutral. The `Clone` cause is RC-2a's event renamed: same site, same scope, same consumers' bodies
(I-RC1 to I-RC3). The `FragmentParse` cause reaches every consumer, and none acts on it at T3-pre's head: the
event-handler consumer's arm is T3's (its catch-all at T3-pre's head), the custom-element and form consumers' arms are
no-ops (§4.6). So no observable step changes: no script-visible outcome, no `Insert` moved. What is new and observable
only to a probe dispatcher is that the event arrives (U-T10, U-T12, U-T13).

**The dead-code condition (PM decision 722; all three hold).** (1) Each `FragmentParse` no-op arm carries, at its
position, the pointer to its consumer: T3 (T3 memo D-T12) for the cause, plus the arm's own reason (§4.6). (2) DoD item
7: T3 is Layout's next unit after T3-pre and lands immediately after it once io has landed (umbrella x12, x14), so the
window in which `FragmentParse` reaches no acting consumer is T3-pre → T3. (3) Probe cells show the event arrives
(U-T10 in the parser, U-T12 through the session's `innerHTML` path, U-T13 through DOMParser's window).

### §1.2 Is T3-pre no worse than its base?

Yes: §1.1 (0 observable outcomes change). Release-build behaviour on the asserted paths: a `Clone` fire while `Taken`
is dropped as on the base (I-RC3); `set_mutation_dispatcher` while `Taken` installs as on the base, dropping the queue
(a path no production code takes, D-P3); a guard dropped without `close` on a returning path discards the queue (no
consumer acts on it at T3-pre's head).

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
(the event, landed), RC-2b (PM decision 723, I-RC1 to I-RC3); decisions 705, 711, 713, 714, 720, 722, 723.

## §2. Coupled invariants

Queue / window pairs [L3 surface] (R1 F2; each named in §4.4, pinned in §6):
- **Q3×Q5 (× unwind)** — `close` restores, then flushes; `Drop` reached without `close` restores and discards. A panic
  in a consumer during the flush unwinds with the slot already `Installed` (`dispatch_event`'s own guard restores it),
  and the guard's `Drop` then finds `taken` `None`: no second restore, no second flush (U-T11 (b)).
- **Q3×Q6 (× nested guard)** — a nested guard (its take found `Taken`) holds nothing: its `close` and `Drop` neither
  restore nor flush, and `pending` stays in the slot for the outermost close (ig *Why it is unwind-safe* item 3; U-T11
  (a)).
- **Q3×Q7 (× unbind clear)** — `clear_mutation_dispatcher` inside a window writes `Absent`: `pending` is discarded with
  the window, since no consumer is left; the outermost restore then writes `Installed(box)` back as on the base (ig §4.1:
  "the slot behaviour on the return path is today's") and its returned list is empty (U-T11 (c)).
- **Q3×Q8 (× root liveness)** — liveness is evaluated **per root, at dispatch time, after the restore**: the flush
  checks `EcsDom::contains(root)` for each queued root immediately before dispatching it, so a root despawned inside the
  window, or by a consumer of an earlier queued event, is skipped (U-T11 (a)).
- **Q3×Q4 (× placement order)** — serial (no enclosing window: every entry point but DOMParser's): the event is
  dispatched at the fire, **before** the caller's placement, so it precedes every `Insert` of the placement (U-T12).
  Inside DOMParser's window: the event is queued and dispatched at the window's close, **after** the placement under
  the synthesized `html` element (whose `Insert`s reach no consumer, as on the base) and after the two structural
  reconcilers (form-control state, base URL; H-P4) (U-T13). No script runs inside the window, so "before script can
  observe the subtree" (D-P1's ground) holds in both.
- **Q3×Q9 (× slot writers)** — `pending` exists only in `Taken`; each writer's effect on it is defined (§4.4's table); no
  writer outside the restore function and the push changes it without either asserting or being the unbind discard.
- **Q2×Q3** — the fragment fire happens after the backend's own window closed (P1), so it is either a dispatch (no
  enclosing window) or a push (P2, P3); the clone fire never meets `Taken` in production (I-RC3).
- **Q1×Q10** — no consumer reads what another writes on this event (the `Clone` arms: the RC-2a plan §2 C × D; the
  `FragmentParse` arms: none acts at T3-pre's head, and from T3 only the event-handler arm does), so the consumers'
  field order is unobservable.

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
| WHATWG HTML §13.4 Parsing HTML fragments | *HTML fragment parsing algorithm* steps 1–2 | every production fragment parse goes through one function | `parse_fragment_progressive`, after both backends' windows close (H-P4) | ✓ | yes |
| WHATWG HTML §8.5.1 The DOMParser interface | `parseFromString` steps 3–4 (a new Document; scripting disabled because it has no browsing context) | the build's fragment parse runs inside DOMParser's take window: its event is queued and dispatched at the window's close | `crates/dom/elidex-form/src/inert_document.rs` `parse_into_inert_document` (under ig's `DispatcherTake`); `DispatcherSlot::Taken { pending }`, `DispatcherTake::close` (§4.4) | ✗ | yes |

Numbers and titles (2026-11-13): `webref heading dom 4.4` (Interface Node), `4.9` (Interface Element); `webref heading
html 13.2.6.1` (Creating and inserting nodes, `#creating-and-inserting-nodes`), `13.4` (Parsing HTML fragments,
`#parsing-html-fragments`), `8.5.1` (The DOMParser interface, `#the-domparser-interface`). `webref dfn dom`: 'clone a
node' → §4.4 `#concept-node-clone`, 'clone a single node' → §4.4 `#clone-a-single-node`, 'append an attribute' → §4.9
`#concept-element-attributes-append`, 'handle attribute changes' → §4.9 `#handle-attribute-changes`; `webref dfn
html`: 'create an element for a token' → §13.2.6.1 `#create-an-element-for-the-token`, 'HTML fragment parsing
algorithm' → §13.4 `#html-fragment-parsing-algorithm`. Step texts: `webref body dom concept-node-clone` (steps 2, 5,
6.7), `clone-a-single-node` (step 2.5), `concept-element-attributes-append` (step 4), `handle-attribute-changes` (step
3); `webref body html creating-and-inserting-nodes` (step 21), `html-fragment-parsing-algorithm` (steps 1, 2),
`dom-domparser-parsefromstring` (steps 3–4).

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
| the restore | `DispatcherTake::restore` (NEW, private; the one restore function) | D-P3 |
| the flush | ig's `DispatcherTake::close`, T3-pre's change | D-P3 |

No component, store, trait or crate edge is added (`elidex-html-parser` already depends on `elidex-ecs`). One event
variant is replaced, not added; one enum variant gains a payload.

### §4.2 The event [L3 surface]

```rust
// crates/core/elidex-ecs/src/dom/mutation_event.rs (re-exported from elidex-ecs's lib.rs, as CloneScope is)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreationCause {
    Clone(CloneScope),   // DOM §4.4 "clone a node"; CloneScope unchanged (RC-2a)
    FragmentParse,       // HTML §13.4 "HTML fragment parsing algorithm"
}
// in enum MutationEvent<'a>, replacing CloneCreated { root, scope }:
SubtreeCreated { root: Entity, cause: CreationCause },
```

`root` and every node in its subtree were created by one operation of the cause, and none of them has been observed by
script yet. `CloneScope` and `EcsDom::for_each_clone_created(root, scope, …)` stay as they are (the clone's visit set,
the RC-2a plan §4.3), read by the clone consumers from the `Clone` cause's scope. **Spec ground (both causes):** each is
element creation whose attributes are appended at creation, DOM §4.4 *clone a single node* step 2.5 and HTML §13.2.6.1
*create an element for a token* step 21, each running DOM §4.9 *append an attribute* step 4 → *handle attribute
changes* step 3 → the attribute change steps. elidex batches those per-element runs into one event per created subtree,
dispatched before script can observe the subtree (RC-2a's precedent for the clone). Sibling plans that name
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
queue and of the flush shape (δ) (PM 720); the T3 memo and the umbrella refer here.

```rust
// crates/core/elidex-ecs/src/dom/mod.rs (T3-pre; on ig's head)
enum DispatcherSlot {
    Absent,
    Installed(Box<dyn MutationDispatcher + Send + Sync>),
    /// A take window is open (ig's `DispatcherTake`). `pending`: the roots of the `FragmentParse` creation events
    /// fired inside it, in fire order, dispatched at the close of the guard that took the dispatcher (§4.4).
    Taken { pending: Vec<Entity> },
}
```

**Every writer of the slot, per state** (R1 M2; at T3-pre's head; H-P1 re-pulled at ig's head, §9 item 1):

| Writer | `Absent` | `Installed(d)` | `Taken { pending }` |
|---|---|---|---|
| `fire_subtree_created(root, cause)` (NEW) | nothing, both causes (unbound, as on the base) | dispatch at once (`dispatch_event`) | `FragmentParse`: `pending.push(root)`, no assertion. `Clone(_)`: D-4's `debug_assert!` (I-RC3); release: dropped, as on the base |
| `dispatch_event` (every other event) | left as is | `Absent` for the dispatch, `Installed` again by `RestoreDispatcher::drop` | left as is: the event is dropped, as today; `pending` untouched |
| `take_mutation_dispatcher` (`pub(in crate::dom)` from ig; its one caller `DispatcherTake::open` (NEW: ig)) | left as is; the guard holds nothing | → `Taken { pending: Vec::new() }`; the guard holds `d` | left as is; the guard holds nothing (a nested window) |
| `DispatcherTake::restore` (NEW, private; called by `close` and `Drop` only) | — (a guard holds a box only if it took one) | — | when the guard holds a box: **one `mem::replace`** of the slot with `Installed(box)`; it returns the replaced `Taken`'s `pending` (empty if the slot was `Absent`, i.e. cleared inside the window); when the guard holds nothing: no write, returns empty |
| `set_mutation_dispatcher` (`pub`) | → `Installed` | → `Installed`, returns the previous | **`debug_assert!(false, …)`** "install inside an open take window" (NEW); release: → `Installed`, the window's state, `pending` included, replaced as by a clear (not a restore: the restore function does not call it, P-P2) |
| `clear_mutation_dispatcher` (`pub`; unbind) | → `Absent` | → `Absent` | → `Absent`; `pending` discarded with the window (no consumer left) |
| `RestoreDispatcher::drop` | → `Installed(d)` | — | — (it runs only after `dispatch_event` emptied an `Installed` slot; no consumer can take inside a dispatch, the re-entry contract) |

`set_mutation_dispatcher` keeps its base contract for `Absent` / `Installed` (`Vm::bind` and the test installs). Its
production callers on `Taken` are the base's four window restores (H-P1), which ig moves into the guard; at T3-pre's head
the restore no longer goes through it, so no production caller meets `Taken`, and the assertion makes a future one
visible instead of dropping its queue (`git grep -n 'set_mutation_dispatcher(' -- crates/core/elidex-ecs/src/dom/dispatcher_take.rs` → 0, DoD 4).

**The restore function (PM 720, option (δ)).** `DispatcherTake::restore(&mut self) -> Vec<Entity>`: if
`self.taken.take()` is `Some(box)`, one `mem::replace(&mut self.dom.dispatcher, DispatcherSlot::Installed(box))`,
returning the replaced value's `pending` (or an empty `Vec`); else an empty `Vec` and no write. `close` and `Drop` both
call it and nothing else restores: one restore function, one slot state (guarantee (b)), one flush point.

**The outermost close** (`DispatcherTake::close(mut self)`, T3-pre's change to ig's body `drop(self)`): `let pending =
self.restore();` then, through `self.dom` (still held), for each root in fire order: if `self.dom.contains(root)`,
dispatch `SubtreeCreated { root, cause: FragmentParse }` (through `dispatch_event`, as `fire_subtree_created`'s
`Installed` arm); then `self` drops, and since `taken` is already `None` its `Drop` restores nothing and finds an
empty list. The restore comes first, so the flush dispatches through the installed dispatcher. Liveness is evaluated
per root at its dispatch (Q3×Q8). A consumer panic during the flush unwinds with the slot `Installed`
(`dispatch_event`'s own guard) and the remaining roots undispatched; the guard's `Drop` then finds `taken` `None`
(Q3×Q5).

**Unwind** (`Drop` reached without `close`): calls the same function and **discards** the returned list; no consumer
runs during an unwind, so ig's "`Drop` does nothing that can panic" holds on the unwind path (PR (ig) plan §4.1 *Why
it is unwind-safe* item 2).

**A returning path without `close` (R1 M3).** ig's windows that can enclose a fragment parse call `close` with no early
return before it (H-P5), so PM 720's intent is that `Drop` discards only on unwind. `Drop` states it:
`debug_assert!(pending.is_empty() || std::thread::panicking(), "a take window holding queued creation events was
dropped without close on a returning path")`. It fires only when not already unwinding, so it cannot turn an unwind
into an abort (the precedent is `RestoreDispatcher::drop`'s `|| std::thread::panicking()`, H-P1). A window whose guard
took nothing, or whose `pending` is empty (`despawn_subtree`'s `TeardownWindow`, which fires nothing inside), is
unaffected. Cell U-T11 (e).

**What T3-pre relies on from ig** (the PR (ig) plan §8 *T3 intersection*): (a) the restore point is the outermost
guard's `close`, the explicit restore point on a returning path, distinct from the unwind `Drop`; (b) the slot state is
the single home of take/restore state (no window keeps a second flag or depth counter; the guard holds only the box it
took), so `pending` has one home. "The guard that took" is identified by what it holds (the box), not by
`is_outermost` (announced removed, PM 718). T3-pre's PR adds `pending`, the push, the restore function, the flush in
`close`, the `Drop` assertion and `set_mutation_dispatcher`'s `Taken` assertion; ig adds none of them. L3 rewords the
PR (ig) plan §4.1 to "the restore function is the only one" and its *Why it is unwind-safe* item 2 to "one
`mem::replace` in the restore function" (PM 720; R1 M2), re-pinned before T3-pre is ratified (R-P3).

### §4.5 Take / restore sites relied on (premise, re-pulled at ig's head) [L3 surface]

At `cee74989` (H-P4): (a) `begin_detached_fragment` takes (`teardown.rs:308`); the tolerant backend restores at
`lib.rs:156-158`, the strict one at `tree_builder/mod.rs:238-241` (both result arms), each before returning to
`parse_fragment_progressive`; (b) `parse_into_inert_document` takes at `inert_document.rs:112` and restores at
`:202-203`, enclosing `apply_set_inner_html` (`:142`); (c) `despawn_subtree` (`teardown.rs:225`, restore `:256`): not
relied on (nothing fires inside it; the strict reject path calls it inside (a)'s window). These anchors change shape in
L3's PR (is) and PR (ig) (every take becomes a `DispatcherTake`; the strict window opens its guard on the caller's dom,
PR (ig) plan §4.1 *The four windows*); T3-pre re-pulls (a)–(c) at ig's head (§9 item 1). Properties relied on,
unchanged by is / ig (L3's review, PM decision 713 (1)): **P1** when the backend call inside
`parse_fragment_progressive` returns, the build's own window is closed; **P2** a window that encloses a fragment parse is
still open at the fire point (the slot reads `Taken { .. }`); **P3** a take inside an open window opens no window and is
followed by no restore (its guard holds nothing). With §4.4's (a) and (b) they make the fragment fire either a dispatch
(P1, no enclosing window) or a push onto the enclosing window's `pending` (P2, P3), flushed exactly once, at that
window's close.

### §4.6 Consumer migration [L3 surface]

At T3-pre's head `git grep -n -e CloneCreated -e fire_clone_created -- crates` → 0 (H-P3's census).
- `CustomElementReactionConsumer` (`consumer.rs:116`): `SubtreeCreated { root, cause: CreationCause::Clone(scope) }` →
  `handle_clone_created(root, scope, dom)`, unchanged; `SubtreeCreated { cause: CreationCause::FragmentParse, .. }` →
  no-op, with the pointer at the arm: "the cause's consumer is `EventHandlerAttributeConsumer` from T3 (T3 memo
  D-T12); the upgrade enqueue at a fragment-parsed element's creation is `#11-custom-element-upgrade-at-fragment-creation`
  (L3)". Behaviour unchanged (PM decision 711 (4)).
- `FormControlReconciler` (`reconciler.rs:53`): `Clone(scope)` → `apply_clone_form_state`, unchanged;
  `FragmentParse` → no-op, with the pointer: "the cause's consumer is `EventHandlerAttributeConsumer` from T3 (T3 memo
  D-T12); the form *cloning steps* are a clone's only, and DOMParser's build re-runs the form-control reconciler
  itself" (a permanent no-op, not dead code).
- `LiveRangeBridge` (`range/live.rs:160`, its ignored-variant list): the listed name changes to `SubtreeCreated { .. }`;
  still ignored (I-RC2).
- `EventHandlerAttributeConsumer`, `node_iter`, `base_url`, `canvas`: their catch-all arm, unchanged at T3-pre's head.
  T3 adds the event-handler consumer's `SubtreeCreated` arm (T3 memo D-T12).
- The dom-api test probe `CloneCreatedProbe` (`test_util.rs:43-53`) is renamed `SubtreeCreatedProbe` and records the
  cause; docs and tests that name the old variant or fire function (`ecs/dom/mod.rs`, `ecs/dom/tree/walkers.rs`,
  `ecs/dom/tree_clone.rs`, `elidex-custom-elements` `upgrade.rs`, `elidex-form` `clone.rs`,
  `elidex-js` `consumer_dispatcher.rs` and `node_methods_extras.rs`, and the test files of §5.3) are renamed in place.

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
| `MutationEvent::SubtreeCreated`, `CreationCause` (NEW) | DOM §4.4 *clone a node* steps 2, 5, 6.7 and *clone a single node* step 2.5 (the `Clone` cause; RC-2a's text kept); HTML §13.4 *HTML fragment parsing algorithm* and §13.2.6.1 *create an element for a token* step 21 (the `FragmentParse` cause); DOM §4.9 *append an attribute* step 4 |
| `EcsDom::fire_subtree_created` (NEW) | T3-pre memo §4.4's writer table (the `Taken` arms: queue for `FragmentParse`, D-4 assertion for `Clone`) |
| `parse_fragment_progressive`'s fire | T3-pre memo §4.3: once per returned root, after both backends' restore; T3-pre memo §4.3's fallback record |
| `DispatcherSlot::Taken { pending }`, `DispatcherTake::restore` (NEW), `DispatcherTake::close`, its `Drop` | T3-pre memo §4.4 (PM decisions 711, 713, 720): restore, then dispatch in order, each root checked alive at its dispatch; discarded on unwind and at unbind |
| `set_mutation_dispatcher` | its `Taken` assertion: T3-pre memo §4.4 (R1 M2) |
| the `FragmentParse` no-op arms (custom elements, form) | T3 memo D-T12 (the cause's consumer); the arm's own reason (T3-pre memo §4.6) |

## §5. Populations, touched files, named risks

### §5.1 Populations (re-runnable)

The event's population: H-P3's census (73 lines in 19 files at `cee74989`, verified 2026-11-13) → 0 at T3-pre's head (DoD 4). Fire sites:
one per cause (H-P2; H-P4: `parse_fragment_progressive`, six callers). Slot writers: H-P1's list, each in §4.4's table.
Windows whose `pending` can be non-empty: those enclosing a fragment parse, i.e. DOMParser's (H-P4).

### §5.2 Existing tests whose expectation changes

None changes outcome (§1.1). Renamed in place: the `CloneCreated` probes and cells (§5.3); `fire_clone_created`'s
`Taken` assertion test in `ecs/dom/tests/dispatch_guard.rs` becomes U-T11 (d).

### §5.3 Touched files

Sizes at `cee74989` (`git show cee74989:<path> | wc -l`, 2026-11-13); every size is re-measured at the base T3-pre is
cut from (§9 item 1).
- `crates/core/elidex-ecs/src/dom/mod.rs` (959; the slot's payload, `set_mutation_dispatcher`'s assertion,
  `take_mutation_dispatcher`'s `Taken { pending: Vec::new() }`, `fire_subtree_created` replacing `fire_clone_created`).
  **R1 M19:** L3's PR (is), PR (ig) and PR (io) also edit this file before T3-pre's base; its size is re-measured at the
  base, and if it is over 1000 there, the touch-time split check (CLAUDE.md "1000-line debt") runs first, as a
  standalone prerequisite split before T3-pre's feature commits (the dispatcher section, `:190-345` at `cee74989`, is
  the candidate seam).
- `crates/core/elidex-ecs/src/dom/dispatcher_take.rs` (NEW: ig): `restore`, the flush in `close`, `Drop`'s call and
  assertion.
- `crates/core/elidex-ecs/src/dom/mutation_event.rs` (441; the variant, `CreationCause`); `crates/core/elidex-ecs/src/lib.rs`
  (51; re-export); docs in `ecs/dom/tree/walkers.rs` (482; also edited by io and by the DOM-walker program's slice 1,
  merge contact, §8.2) and `ecs/dom/tree_clone.rs` (415).
- `crates/dom/elidex-html-parser/src/lib.rs` (259; the fragment fire); `crates/dom/elidex-dom-api/src/node_methods/clone.rs`
  (472; the clone fire), `range/live.rs` (506), `test_util.rs` (78); `crates/dom/elidex-custom-elements/src/consumer.rs`
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
`event_handler_processing/mod.rs` (T3's); `inert_document.rs`'s code (its rustdoc is T3's, T3 memo D-T13).

### §5.4 Named risks

- **R-P1 — the asserted paths in release.** §1.2: release keeps the base's outcome on each.
- **R-P2 — a consumer panic during the flush** leaves later queued roots undispatched (Q3×Q5); a consumer panic is
  already a bug that unwinds the whole mutation (as at any dispatch).
- **R-P3 — L3's surface, cited at a snapshot.** Every L3 citation (H-P5, §4.4, §4.5) is re-pinned to L3's ratified sha
  before T3-pre is ratified; a citation whose symbol or section is gone there is re-derived, not carried. **Stop
  condition:** T3-pre stops for the PM if ig is ratified or lands without a consuming `close` as the restore point on
  the returning path of a window that can enclose a fragment parse (guarantee (a)), with a second home of take/restore
  state (guarantee (b)), or if (δ) cannot be built on ig's landed guard (`close` does not keep `&mut EcsDom` past the
  restore, or the restore is not one body `close` and `Drop` can share).
- **R-P4 — the dead window (PM decision 722).** Between T3-pre and T3 the `FragmentParse` event reaches no acting
  consumer; bounded by DoD item 7.

## §6. Test plan

Red-first: each **red** cell is committed failing on T3-pre's base before the fix commit.

### §6.1 Unit

| Cell | Kind | Shape | Oracle |
|---|---|---|---|
| U-T10 | red (§4.3; `elidex-html-parser`) | a probe dispatcher recording every event; `parse_fragment_progressive` with two top-level elements, on input the strict backend accepts and on input it rejects (tolerant fallback); then the same inside an outer `DispatcherTake` (ig) window, closed after the call | exactly two `SubtreeCreated { cause: FragmentParse }`, roots in the returned order, no other event before them, none naming a despawned entity; inside the outer window none reaches the probe during the call, and the same two, in the same order, reach it at the outer close, after the restore. Base: no event |
| U-T11 [L3 surface] | red + guard (§4.4; ig's guard test module or `ecs/dom/tests/dispatch_guard.rs`) | a probe dispatcher installed; an outer `DispatcherTake`: (a) `fire_subtree_created(r1, FragmentParse)`, `(r2, …)`, `(r3, …)`, `despawn_subtree(r2)` inside the window, then a nested `DispatcherTake` opened and closed, then the outer close; (b) as (a) up to the nested window, then a panic inside the outer window, caught outside it (`catch_unwind`); (c) one push, then `clear_mutation_dispatcher` inside the window, then the close; (d) `fire_subtree_created(root, Clone(CloneScope::Subtree))` while `Taken`; (e) one push, then the outer guard dropped without `close` on a returning path; (f) one push, then `set_mutation_dispatcher(other)` inside the window | (a) nothing reaches the probe before the outer close (the nested close flushes nothing); at the close the slot is `Installed` before the first dispatch (the probe reads it), and exactly `r1`, `r3` arrive, in that order; (b) the slot holds the dispatcher and the probe received nothing; (c) nothing is received; (d) debug-asserts (D-4, the existing test renamed; I-RC3); (e) debug-asserts in `Drop` (R1 M3); (f) debug-asserts in `set_mutation_dispatcher` (R1 M2). Base: (a)–(c) receive nothing (no queue), (e)–(f) do not assert |
| U-T12 | red (Q3×Q4, serial; end to end through the session) | beside `set_inner_html_fires_insert_at_caller_placement` (`html_fragment.rs:668-700`): a probe dispatcher recording every event; `apply_set_inner_html` on an empty connected element with `<p></p><i></i>` | the log is `SubtreeCreated { p, FragmentParse }`, `SubtreeCreated { i, FragmentParse }`, then the placement's `Insert`s. Base: the `Insert`s only |
| U-T13 | red (Q3×Q4 nested; end to end through DOMParser's window) | a probe dispatcher; `parse_into_inert_document` with `<body><p></p></body>` | no event reaches the probe during the build except at its close; at the close, after the restore, `SubtreeCreated { cause: FragmentParse }` for each root the build's fragment parse returned, after the window's placement (no `Insert` precedes in the log: they reached no consumer). Base: nothing |

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
| the restore through `set_mutation_dispatcher` (the queue dropped at the replacement, P-P2) | U-T11 (a), U-T13 |
| the flush before the restore (dispatched into the `Taken` slot, so lost), or at a nested close | U-T11 (a), U-T13 |
| the queue flushed on unwind, or not discarded at `clear_mutation_dispatcher` | U-T11 (b), (c) |
| a dead queued root dispatched (no `contains` check), or liveness checked once before the loop | U-T11 (a) |
| `Drop` discarding silently on a returning path | U-T11 (e) |
| `set_mutation_dispatcher` on `Taken` without the assertion | U-T11 (f) |
| the serial fire after the placement | U-T12 |
| `LiveRangeBridge` acting on `SubtreeCreated` | none by outcome at T3-pre's head (no live range refers to a fresh subtree); held by DoD 4's grep (I-RC2) |

## §7. Out of scope and slots

| Item | Why not T3-pre | Home |
|---|---|---|
| the event-handler consumer's `SubtreeCreated` arm, its walks, D-T13 | PM decision 722 | T3 (T3 memo D-T10, D-T12, D-T13) |
| the custom-element upgrade at a fragment-parsed element's creation | L3's consumer; PM decision 711 (4) | `#11-custom-element-upgrade-at-fragment-creation` (L3; **registered**, PM 719) |
| RC-2b's clones on the event | L3's slice (PM decision 723) | the Range program, RC-2b (§8.2) |
| `Range.createContextualFragment` | not implemented (H-P6) | `#11-range-full-impl` (§4.3's record) |

Own deferrals: **0**.

## §8. Edges, hand-offs, records

### §8.1 Edges (umbrella §1.3, edited by this plan PR)

- **x13** L3 PR (ig) → T3-pre: code predecessor (the flush is T3-pre's change to `DispatcherTake::close`; ig's
  guarantees (a), (b); PM decisions 713, 714, 722). Since L3's order is is → ig, T3-pre also follows is.
- **x11** L3 RC-2a → T3-pre (was RC-2a → T3 until rev 8.<t>'s T3 memo rev 5): semantic precondition (the event and
  its `Clone` cause are RC-2a's; landed by #596).
- **x14** T3-pre → T3: code predecessor + semantic precondition (T3's arm matches `SubtreeCreated`; DOMParser's build
  reaches it only through the queue).
- **No edge** with L3's PR (io): merge contact in `ecs/dom/mod.rs` and `walkers.rs` docs only; whichever lands second
  re-pulls the other's coordinates.

### §8.2 Hand-offs

- **T3** (Layout's next unit): consumes `SubtreeCreated` in `EventHandlerAttributeConsumer` (T3 memo D-T10, D-T12).
- **L3 PR (ig)**: the PR (ig) plan §4.1 rewording (PM 720; §4.4's last paragraph), re-pinned before T3-pre is ratified.
- **L3 RC-2b** [L3 surface] (PM decision 723; R1 M17): RC-2b (`docs/plans/2026-11-range-contents-program.md` row RC-2b,
  on main, unlanded) was written against `CloneCreated` / `fire_clone_created`, which T3-pre replaces. It lands after
  T3-pre, on `SubtreeCreated { cause: Clone(scope) }` (PM decision 723), and relies on I-RC1 to I-RC3 (§2), which
  T3-pre keeps; T3-pre's migration set does not grow (RC-2b's own clones are its own edit).
- **L3's DOM-walker program, slice 1** (`#11-dom-walkers-iterative-over-ecs-links`; stacks on io, touches
  `walkers.rs`; L3 umbrella §8 hand-offs) (R1 M18): no ordering edge with T3-pre (T3-pre edits only `walkers.rs` docs);
  merge contact. Its edge with T3's creation walk is the T3 memo's (§8.2).
- **`#11-custom-element-upgrade-at-fragment-creation`** (L3, registered): the arm it adds matches T3-pre's
  `SubtreeCreated { cause: FragmentParse }`. Its ledger *Trigger* is T3-pre's landing (PM 732); the event exists from T3-pre's
  (§10 Q-P1, closed).

### §8.3 Records for existing slots (for the PM, exact texts; at T3-pre's landing)

1. `#11-range-full-impl` — append: "Since T3-pre (#<PR>, `<squash>`), every production fragment parse fires
   `MutationEvent::SubtreeCreated { cause: FragmentParse }` from `elidex_html_parser::parse_fragment_progressive`, the
   one function both parser backends return through. `Range.createContextualFragment`'s implementation must parse
   through it (or the session's `apply_*` wrappers that call it), or its elements get no creation processing (T3-pre
   memo §4.3)."

## §9. Definition of done

1. T3-pre's branch is cut from main after L3's PR (is) and PR (ig) land (x13); every `cee74989` coordinate in §0.2 is
   re-measured there by its command (H-P1 and §4.5's anchors at ig's head; sizes of §5.3); every L3 citation is
   re-pinned to L3's ratified sha (R-P3), the PR (ig) plan §4.1 rewording included; T3-pre stops for the PM on
   R-P3's stop condition.
2. `/elidex-plan-review` passes on this memo (mandatory, §1.3), with L3's confirmation of the [L3 surface] sections
   through the PM (PM decision 722).
3. Red-first: U-T10, U-T11 (a)–(c), (e), (f), U-T12, U-T13 committed failing; the fix makes them green; U-T11 (d) and
   the renamed RC-2a cells green at the head.
4. `git grep -n -e CloneCreated -e fire_clone_created -- crates` → 0; `git grep -n 'fire_subtree_created(' -- crates
   ':!*tests*'` → the definition and two calls (the dom-api cloner, `parse_fragment_progressive`; I-RC1);
   `git grep -n -w 'Taken' -- crates/core/elidex-ecs/src/dom/mod.rs` → every construction is `Taken { pending: … }`;
   `git grep -n 'set_mutation_dispatcher(' -- crates/core/elidex-ecs/src/dom/dispatcher_take.rs` → 0 (P-P2);
   `git grep -n 'SubtreeCreated' -- crates/dom/elidex-dom-api/src/range/live.rs` → its ignored-variant list only (I-RC2);
   `git grep -n 'SubtreeCreated' -- crates/script/elidex-script-session/src` → 0 (the arm is T3's).
5. `cargo test -p elidex-ecs -p elidex-html-parser -p elidex-dom-api -p elidex-custom-elements -p elidex-form -p
   elidex-script-session -p elidex-js --all-features` green; `mise run ci` before push.
6. §8.3's record handed to the PM in the PR description.
7. **T3 lands immediately after** (PM decision 722's dead-code condition): T3 is Layout's next unit after T3-pre; its
   implementation is cut once T3-pre and io have landed (umbrella x12, x14). Each `FragmentParse` no-op arm carries
   its pointer (§4.6).
8. **Checker** (run; do not trust a stored verdict): `python3 .claude/skills/elidex-plan-review/preflight.py
   docs/plans/2026-11-eh-attrs-t3-pre-subtree-created-event.md` exit 0.

## §10. Open questions

**Decided:** the split (PM decision 722); the flush shape (δ) (PM 720, P-P1); RC-2b's order and the three properties
(PM decision 723). Within PM 713 / 720 (R1 M2, M3; not new decisions): the restore function writes the slot itself and
returns `pending` from the same replacement; `set_mutation_dispatcher` on `Taken` debug-asserts; `Drop` asserts an
empty list unless unwinding (§4.4). L3 confirms them with the [L3 surface] sections.

**For the PM:**
- **Q-P1 (closed).** `#11-custom-element-upgrade-at-fragment-creation`'s ledger *Trigger* reads "T3 の着地"; the event its arm
  needs exists from T3-pre's landing. Whether the trigger moves to T3-pre's landing is the PM's and L3's call (no
  T3-pre text depends on it). **Answered by PM 732:** the trigger = T3-pre's landing (ledger updated by pmo).

No browser measurement is needed: no outcome changes (§1.1).
