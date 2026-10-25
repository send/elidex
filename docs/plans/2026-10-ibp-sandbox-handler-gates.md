# N1 `IBP-sandbox-handler-gates` — the event-handler scripting gates follow the current spec (per-PR plan-memo)

Nested PR **N1** of node `IBP-sandbox` (`docs/plans/2026-09-ibp-sandbox.md`, approved by #532, `6270e219`; "the node
memo"). The node memo's approval scope (`:6-14`) binds: its **decided** lines (D2 `:172`; D8's compile-gate reading and
its N1 paragraph `:268-272`; §1's "N1 narrow" verdict `:362-364`; J13's N1–N4 qualifier `:407`; §5.1's N1 row `:529` and
the *Closed* entries for D2 × D8 and C8b `:572-580`; §5.2 edge n3 `:588`; §3's *Branch* cell for §8.1.8.1 `:445`; the §8
records row `:773`) are inherited as decisions; its **input** lines (§4 F10's "Invoke-time gate deleted (N1)" and the
node-document line `:498-502`; §5.3 population (a) `:605`; §5.4 N1's named risks `:610-612`; input file
`docs/plans/2026-09-ibp-sandbox-input.md` §B (a) `:42`, §C `:59-61`, §D `:102`, `:105`, §E C8, C8b, C28b, C30, C31) are
re-derived here. Between its approval and this PR the node memo did not change (`git log --oneline 6270e219..fd6955e8
-- docs/plans/2026-09-ibp-sandbox.md docs/plans/2026-09-ibp-sandbox-input.md` → empty); **this PR** corrects it (the
erratum, §0.3 P-N1; PM decision 16 — the PM's ruling that N1's PR corrects the node memo's §8.1.8.1 citation in place),
which inserts 15 lines after its `:24` and rewrites lines in place, so every node-memo coordinate above `:24` is the
approved one and every one below is the approved one + 15 (`git diff 6270e219 -- docs/plans/2026-09-ibp-sandbox.md`).
The input file is not edited.

**PM decisions cited here** (both 2026-10-25). **Decision 16**: N1's PR corrects the node memo's §8.1.8.1 citation in
place (the erratum); D-N3's reconcile order is the means of the node memo's D2 × D8 closure (O2 (a)); O5's home is the
slot `#11-event-handler-target-determination`, which the PM registers. **Decision 17**: (i) the event-handler consumer's
Insert arm, which turns every `on*` attribute raw again on every insertion including a move (P-N5), is **not** fixed in
N1 but by the standalone prerequisite PR `EH-insert-once` (`docs/plans/2026-10-event-handler-insert-once.md`; target
semantics: a baked attribute is turned into a raw value once per element, never on a later move), which N1 depends on
(§8, §9); (ii) the erratum's D2 wording is neutral — the two candidate step-3.2 readings give the same outcome at N1 and
which one is implemented is N4's (edge n3; O1); (iii) the slot `#11-unhandledrejection-dispatch-target` is registered
(owner core VM, reachability unconfirmed, re-eval 2026-11-30); (iv) the node memo's "§13.4 step 10" is N9's to correct
(§0.3).

**Rev 3** (2026-10-25) — folds plan-review R1 (IMP 4, MIN 17) and PM decision 17. (1) F1: the dependency on
`EH-insert-once` (§0.3 P-N5, H-P16, §2 H3 × H4, §8, §9); C8b(iv), C8c, C8d and arm 3's "source kept" oracle are
re-derived under its semantics (§6), and arm 3's source-kept claim gets discriminating oracles (C8's source oracle, C8c's
second half, C8f). (2) F2: the node memo's D2 and n3 are neutral (erratum revised); O1 is decided by PM decision 17.
(3) F3: NR5 and §7 cite `#11-unhandledrejection-dispatch-target`. (4) F4 (adopted): D-N3's arm order moves into
`elidex_script_session::reconcile_event_handler` (NEW), returning `HandlerReconcile` (NEW); `vm/host/` runs the VM-store
action for the returned case; C8f pins the order without a VM (H6). (5) MINs: the D-N2 unbound branch is located and
given its form (H-P18 backs "every caller is bound"); O4 decided by the dead-code rule (narrow the composition's
`target` and `bound_document` to `Entity`); §1 says why Plumbing is not a third axis; §2 adds H2 × H4, H3 × H5 and H6's
pairs; §4.4 says "vacuous"; G6's pattern and hit list agree (run); §5.2 adds `engine.rs:220-230`; the "without"
pseudo-quote is gone; the erratum's step-1 polarity is the spec's text, its pointer carries no rev pin, and it records
its exception to the node memo's `:12-14` / A114 idiom; §8 records the post-approval slots; the test file's split at
`:538` is decided (§5.3); §0.3 records the §13.4 step number. Disposition of every R1 finding: §10.1. *Rev 2*
(2026-10-25) — folded PM decision 16 and the scout's answers to rev 1's three questions (P-N1 corrected in this PR; P-N2
"closed at N1" by D-N3; O5's home; Q-N1–Q-N3 → H-P13–H-P15; C31 via `dispatchEvent(new MouseEvent('click'))`). *Rev 1*
(2026-10-25) — first draft.

No item is left open for plan-review (§10): O1 and O2 are decided by the PM, O3, O4, O6, O7 by the arguments in §10, O5
by the PM.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every code `file:line` is a **`fd6955e8`** coordinate (origin/main at the branch cut, N2a #544; this PR changes no code).
The node memo's code frame is `f1cf5d67`; the N1 gate sites have the same coordinates in both (`git show <c>:<file> |
/usr/bin/grep -n` over the four gate files for `c ∈ {f1cf5d67, fd6955e8}`: `engine.rs:211`/`:215`, `dispatch_target.rs:202`/`:211`,
`natives_promise.rs:540`, `event_handler_attrs.rs:553`/`:593` in both, verified 2026-10-25), and the two test files differ
only in the one-line N2a re-type each (`git diff f1cf5d67 fd6955e8 -- <the two files>`). Node-memo line numbers are
this PR's head (header paragraph). Spec text is `.claude/tools/webref body` output of 2026-10-25 (the cached
`webappapis.html` carries `Last-Modified: Thu, 22 Oct 2026`); every `§X.Y = name` pair is `webref heading` / `dfn`
output. `grep` in prose commands is `git grep` at `fd6955e8` or `/usr/bin/grep` on the worktree (never the default ugrep).

Symbols written `Type::method` (`VmInner::ensure_event_handler_current`, `DispatchTarget::resolve_callable`,
`DispatchTarget::reconcile_handler`, `HostData::scripts_allowed`, `ElidexJsEngine::scripts_allowed`,
`PendingListener::is_handler`, `EventListeners::uncompiled_source`) name existing methods or fields inside `impl` /
`struct` blocks, which preflight's literal symbol grep does not find (its soft warnings).

Paths are abbreviated after first use: `elidex-js/src/…` = `crates/script/elidex-js/src/…`,
`elidex-script-session/src/…` = `crates/script/elidex-script-session/src/…`.

### §0.2 Premises re-measured

| # | Claim | Command | Result |
|---|---|---|---|
| H-P1 | The compile gate is `ensure_event_handler_current`'s early return on the HostData flag | `sed -n 511,575p crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` | holds: doc `:511-520`, rationale comment `:526-549`, gate `:550-556` (`.is_some_and(\|hd\| !hd.scripts_allowed())` at `:553`); the gate returns **before** both the pending-source `take` (`:567`) and the `cleared` drop (`:578-579`) |
| H-P2 | Three invoke-time gates exist, one per dispatch path | `git grep -n 'scripting_disabled_for_platform_object' fd6955e8 -- crates/script/elidex-js` | holds, three callers: UA dispatch `crates/script/elidex-js/src/engine.rs:206-218` (`ScriptEngine::call_listener`), the VM walk `crates/script/elidex-js/src/vm/host/dispatch_target.rs:202-213` (`DispatchTarget::resolve_callable`), the rejection dispatch `crates/script/elidex-js/src/vm/natives_promise.rs:534-542`; each precedes that path's reconcile call |
| H-P3 | The host wrapper's only callers are the three invoke gates (input §C's condition for deleting it in N1) | same command; `/usr/bin/grep -n 'scripting_disabled_for_platform_object' crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` | **holds**: wrapper `event_handler_attrs.rs:584-620` (doc + fn); its call sites are H-P2's three (verified 2026-10-25); the compile gate does not call it (it reads `HostData::scripts_allowed`, H-P1) |
| H-P4 | HEAD's composition is in the session crate, settings clause first | `sed -n 83,114p crates/script/elidex-script-session/src/scripting.rs` | holds: `scripting_disabled_for_platform_object(dom, target: Option<Entity>, bound_document: Option<Entity>, sandbox_flags)`; `:89-91` settings clause; `:92-94` `None` target or unbound → `false`; Document target → `entity != document`; a Node → composed tree root if a Document, else `owner_document`, unresolvable → `false`; non-Node (Window) → `false` |
| H-P5 | The composition's docstring names the invoke gate as its purpose | `sed -n 12,82p crates/script/elidex-script-session/src/scripting.rs` | holds: `:12-16` "the canonical predicate behind *the event handler processing algorithm* step 1" — false after N1, and not a step of the current spec (§0.3 P-N1) |
| H-P6 | Raw (uncompiled) handlers exist on elements and on the Window entity only | `git grep -n 'set_uncompiled(' fd6955e8 -- crates ':!*tests*'` → one production writer `crates/script/elidex-script-session/src/event_handler_consumer.rs:474` (`set_inline_handler`); its target `resolve_handler_target` (`:328-334`), called from the AttributeChange arm (`:409`) and the Insert arm (`:461`) | holds: the origin element, or `dom.window_entity()` for a `WindowEventHandlers` attribute on `<body>`; the Document entity and worker globals never carry a raw handler (no content attribute reaches them) |
| H-P7 | The handler-state model: an `EventListeners` entry `{uncompiled, cleared}` plus the engine-side callable | `sed -n 165,290p crates/script/elidex-script-session/src/event_listener.rs`; `sed -n 40,62p crates/script/elidex-js/src/vm/inner.rs` | holds: `set_uncompiled` (`:207`) sets the source and resets `cleared`; `mark_cleared` (`:255`, content-attribute removal) drops the source and sets `cleared`; the IDL setter's `clear_uncompiled` (`:238`) drops the source and resets `cleared`; the callable lives in the listener store, dropped by `VmInner::remove_listener_and_prune_back_ref` (`inner.rs:49`) without touching the entry. A re-set attribute leaves the previous callable in the store until the next reconcile overwrites it |
| H-P8 | `is_handler` readers | `git grep -n -w 'is_handler' fd6955e8 -- crates` | 18 lines (verified 2026-10-25); readers: `engine.rs:211` (gate only), `dispatch_target.rs:202` (gate **and** the reconcile guard `:214`), `natives_promise.rs:540` (gate only; field `:451`, set `:491`); the plan field `ListenerPlanEntry::is_handler` (`elidex-script-session/src/event_dispatch/mod.rs:242`, documented as the reconcile skip `:234-241`) and the trait parameter (`elidex-script-session/src/engine.rs:83`, documented as the gate `:71-75`); 8 test call sites in `tests_call_listener.rs` |
| H-P9 | Population (a) (input §B) = 2 files | input §B's command at `fd6955e8` (both halves, `git grep -l … \| xargs /usr/bin/grep -l -E …`) | **holds**: `vm/tests/tests_event_handler_attrs.rs`, `tests_call_listener.rs` (verified 2026-10-25) — a vocabulary count; §5.1 defines the population by behaviour |
| H-P10 | The node memo's test pointers | `/usr/bin/grep -n '^fn ' crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs.rs crates/script/elidex-js/src/tests_call_listener.rs` | the pointers are **function** lines: `:567` `compiled_handler_on_null_bc_document_node_suppressed_listener_still_runs`, `:597` `suppressed_target_dispatch_does_not_compile_raw_inline_handler`, `:691` `appended_then_removed_domparser_node_suppressed_again`, `:719` `bound_created_node_appended_into_domparser_doc_suppressed`; `tests_call_listener.rs:188` `call_listener_skips_inline_handler_when_scripting_disabled` |
| H-P11 | Files N1 touches and the 1000-line rule | `wc -l` on §5.3's files | none >1000; `crates/script/elidex-js/src/vm/natives_promise.rs` (965; N1 deletes lines); `vm/tests/tests_event_handler_attrs.rs` (735, inside the touch-time band; §5.3 decides its split) |
| H-P12 | The eval gate is untouched by N1 | `sed -n 128,140p crates/script/elidex-js/src/engine.rs`; `:168-180` | holds: `ElidexJsEngine::scripts_allowed` (`:137-139`) feeds `eval` (`:174`) only; `HostData::scripts_allowed` (`vm/host_data/mod.rs:1107`) keeps that caller, and its field doc (`:197`) already names it "the eval gate" — N1 does not touch `host_data/mod.rs` |
| H-P13 | (rev 1's Q-N1) A DOMParser document's parser-baked handler attributes reach the content-attribute consumer | `/usr/bin/grep -n 'parse_into_inert_document' crates/script/elidex-js/src/vm/host/dom_parser.rs`; `sed -n 105,120p` and `sed -n 192,202p crates/dom/elidex-form/src/inert_document.rs` | **no**: `dom_parser.rs:296` calls `elidex_form::parse_into_inert_document`, which takes the live mutation dispatcher before the build (`inert_document.rs:112`) and restores it after (`:198-199`), so no mutation consumer runs on the throwaway document. A parser-baked `<div onclick>` in a DOMParser document carries **no** `EventListeners` entry (the spec gives it a raw value whose getter reads null — the same observable result); inserting that node into the live document fires Insert, and `handle_insert` (`event_handler_consumer.rs:427`) installs the raw handler then. The existing DOMParser handler tests (`tests_event_handler_attrs.rs` `:567`, `:597`, `:670`, `:691`, `:719`; their scripts at `:576`, `:601`, `:674`, `:695`, `:723`) all parse markup **without** handler attributes and write the handler after the parse (IDL setter or `setAttribute`), so none depends on this; no N1 cell has a parser-baked arm |
| H-P14 | (rev 1's Q-N2) the VM exposes `HTMLElement.prototype.click()` | `sed -n 140,153p crates/script/elidex-js/src/vm/host/html_element_proto.rs`; `/usr/bin/grep -rn 'mouse_event_global' crates/script/elidex-js/src` | **no**: `install_html_element_methods` (`html_element_proto.rs:146-152`) installs `focus` / `blur` only; `:144-145` and `:258-259` defer `click()` to `#11-vm-host-synthetic-dom-event-dispatch`. `dispatchEvent` is installed (`event_target.rs:68`), and the `MouseEvent` constructor is a registered global (`register_mouse_event_global`, `events_ui.rs:424`, called at `globals.rs:677` under `feature = "engine"`; exercised by `vm/tests/tests_ui_event_constructor.rs`). C31 uses `dispatchEvent(new MouseEvent('click'))` |
| H-P15 | (rev 1's Q-N3) worker / service-worker handlers are dispatched through `DispatchTarget::VmObject` | `/usr/bin/grep -rn 'resolve_callable' crates/script/elidex-js/src`; `/usr/bin/grep -n 'dispatch_script_event(&mut ctx' crates/script/elidex-js/src/vm/host/worker_scope.rs crates/script/elidex-js/src/vm/host/service_worker/event.rs`; `/usr/bin/grep -n 'DispatchTarget::' crates/script/elidex-js/src/vm/host/event_target_dispatch.rs crates/script/elidex-js/src/vm/host/event_target_dispatch_vm.rs` | **false (rev 1's claim)**: `resolve_callable` (`dispatch_target.rs:197`) has one non-test caller, `invoke_listeners_shared` (`event_target_dispatch.rs:470`). The worker path (`vm/host/worker_scope.rs:333`) and the SW path (`vm/host/service_worker/event.rs:192-211`, the global a `HostObject` entity) call `dispatch_script_event`, whose listener home is `DispatchTarget::Node` (`event_target_dispatch.rs:343`); `VmObject` homes are built only by the VM-object walk (`event_target_dispatch_vm.rs:245`). So the worker realms' §8.1.3.4 reader at HEAD is the **`Node` arm** of the deleted VM-walk gate, plus the compile gate's early return (H-P1), which runs for every handler reconcile. Neither fires in production: the only production sandbox-flag writer is the document pipeline (`git grep -n 'set_sandbox_flags' fd6955e8 -- crates ':!*tests*'` → `crates/shell/elidex-shell/src/pipeline.rs:281`, plus the trait and impl definitions), so a worker `HostData` keeps the default set |
| H-P16 | (R1 F1) A move does not make a handler raw again | `sed -n 282,299p crates/core/elidex-ecs/src/dom/tree/mutation.rs`; `/usr/bin/grep -n 'MutationEvent::Insert' crates/script/elidex-script-session/src/event_handler_consumer.rs`; `sed -n 423,463p` of that file | **false at `fd6955e8`**: `fire_after_insert` (`mutation.rs:282-299`) emits `MutationEvent::Insert` for every insertion, a move included; the consumer routes every Insert to `handle_insert` (`event_handler_consumer.rs:381`), which walks the inserted subtree and calls `set_inline_handler` → `set_uncompiled` for **every** `on*` content attribute (`:427-463`). So after any move the pending source is set again, whatever the value was (a callback compiled earlier, or one set through IDL while the attribute exists). Fixed by `EH-insert-once` (PM decision 17 (i)), not by N1 — P-N5 |
| H-P17 | (R1 A1) The `EventListeners` API the four arms read | `/usr/bin/grep -n 'pub fn ' crates/script/elidex-script-session/src/event_listener.rs` | holds: `is_handler_cleared` (`:271`, `&self`), `uncompiled_source` (`:282`, `&self`, does **not** drain — "without draining", its doc), `take_uncompiled` (`:225`, `&mut self`, drains); HEAD's reconcile calls `take_uncompiled` and `is_handler_cleared` (`event_handler_attrs.rs:567-568`, the only non-test readers in `elidex-js`: `git grep -n 'take_uncompiled\|is_handler_cleared\|uncompiled_source' fd6955e8 -- crates/script/elidex-js/src ':!*tests*'` → those two lines) |
| H-P18 | (R1 A5) Every reconcile caller runs on a bound VM (D-N2's unbound branch is unreachable) | `/usr/bin/grep -rn 'ensure_event_handler_current(' crates/script/elidex-js/src \| /usr/bin/grep -v 'fn ensure_event_handler_current\|tests'`; `sed -n 60,80p` and `sed -n 100,160p crates/script/elidex-js/src/vm/host/dispatch_target.rs`; `sed -n 455,462p crates/script/elidex-js/src/vm/natives_promise.rs`; `sed -n 24,35p crates/script/elidex-js/src/engine.rs`; `sed -n 1270,1325p crates/script/elidex-js/src/vm/host_data/mod.rs` | **holds**: three call sites. (1) `dispatch_target.rs:183` — `DispatchTarget::reconcile_handler` (`:181-185`, `Node` arm only), reached from the VM walk (`resolve_callable`, `:214`) and the IDL getter (`read_event_handler`, `event_handler_attrs.rs:649`); a `Node` target is built only while bound (`target_from_this`, `dispatch_target.rs:70-77`), and every `Node` listener-home access goes through `ctx.host_if_bound()?` (`:106`, `:125`, `:146`), so the getter returns null before the reconcile when unbound (`read_event_handler`, `:643-648`). (2) `engine.rs:233` — `call_listener`, under the BATCH-BIND "assume bound" contract (`engine.rs:27-34`); HEAD's reconcile already reaches `HostData::dom()`, which `assert!`s bound (`host_data/mod.rs:1320-1321`), on every unsandboxed call, so an unbound call panics at HEAD today. (3) `natives_promise.rs:548` — after `Some(host) if host.is_bound()` (`:459`). While bound the document entity is always `Some` (`bind` sets it, `host_data/mod.rs:1277`; `unbind` clears it, `:1283`) |

### §0.3 Premise errors

- **P-N1 — the node memo's §8.1.8.1 citation; corrected in this PR (PM decision 16: the node memo is corrected in place
  in N1's PR).** Rev 1 recorded this under the A114 idiom ("reported, not edited"); the PM decided to correct the node
  memo in N1's PR. The node memo's erratum records that this in-place edit is an exception, by that decision, to its
  approval scope's `:12-14` (a finding changes the memo only through a decision) and to its §0.3 A114 idiom. Current
  webref reads:
  - `.claude/tools/webref body html getting-the-current-value-of-the-event-handler` — *get the current value of the
    event handler* step 3.2: "If document's browsing context is null, or document's active sandboxing flag set has its
    sandboxed scripts browsing context flag set, then return null."
  - `.claude/tools/webref body html the-event-handler-processing-algorithm` — *the event handler processing algorithm*
    steps 1–4: 1 "If the result of WebDriver BiDi scripting is enabled with eventTarget's relevant settings object is
    false, then return"; 2 callback = *get the current value*; 3 "If callback is null, then return"; 4 "If callback's
    associated realm's global object is a Window object whose associated Document is not fully active, then return".
  - `.claude/tools/webref body html enabling-and-disabling-scripting` — §8.1.3.4 lists four settings-object conditions,
    the fourth "The result of WebDriver BiDi scripting is enabled with settings is true" (`webref dfn webdriver-bidi
    'WebDriver BiDi scripting is enabled'` → WebDriver BiDi §7.4.2.10 The emulation.setScriptingEnabled Command; `webref
    body webdriver-bidi webdriver-bidi-scripting-is-enabled`: with no override in the scripting enabled overrides map,
    step 6 "Return true").

  The node memo (rev ≤ 12) quoted step 3.2 as "If scripting is disabled for document, then return null" and numbered
  the processing algorithm 1–3 with "not fully active" at step 3. This is recorded as a premise error, not classified:
  webref keeps no spec history, so whether the spec changed or the earlier output was misread is not decided.
  **No decision changes at N1**, by this argument: elidex has no WebDriver BiDi and no user scripting switch, so
  §8.1.3.4's settings clause reduces to the sandboxed scripts flag of the settings object's Window's associated
  Document; in HEAD's single realm that flag store is the one `HostData` set, which is the bound document's; a document
  other than the bound document has a null browsing context (HEAD's single-browsing-context model, H-P4's docstring),
  so step 3.2's first disjunct returns null for it whatever its flags; and for the bound document the second disjunct
  reads the same set the settings clause reads. So "step 3.2 as written today" and "scripting is disabled for *el's
  node document*" (D8's reading) give the same result for every document N1 can see. N1 cites the current text and
  states this equivalence (§4.4); which of the two readings N4's `NodeDocumentOf` implements is N4's, recorded on edge
  n3 (PM decision 17 (ii): D2's wording is neutral; O1).

  **Sweep** (three surfaces: the sections that state the old reading, the definition-of-done rows, the downstream
  consequences). Commands: `/usr/bin/grep -n -i -E 'scripting is disabled|step 3|steps 1.3|1–3|1-3|A16|A17|not fully
  active|processing.algorithm|processing step|invoke.time|invoke gate|step 1|step-1|get(ting)? the current value|3\.1.3\.2|3\.2'`
  over the node memo, its input file and this memo; then the concept-level readers (every *Node(el)* /
  *NodeDocumentOf* / *Settings* object form, J5's worker note, §8 slot texts) by reading. Coordinates are the node
  memo's **approved** lines (`6270e219`); each disposition:

  | Hit (node memo, approved line) | Disposition |
  |---|---|
  | A16 `:111` | **fixed** — current text, provenance command, erratum marker |
  | A17 `:112` | **fixed** — current steps 1–4, provenance command |
  | P4 `:127` (parent F10r gate list: A16) | **fixed (annotated)** — P4's ground was A16's old quote; the current step 3.2 reads the document's flag set (F10r-shaped) and its browsing context, so whether the compile gate is an F10 or an F10r-plus-browsing-context reader is re-opened for N4 (n3), not decided |
  | P7 `:131` (S5-4's processing-algorithm gate: A17) | unaffected — S5-4's §8.1.3.4 invoke gate is no step of the current text either; A17's corrected result is still "false" |
  | D2 `:157-158` | **fixed** (rev 3 wording, PM decision 17 (ii)) — names step 3.2 and the two candidate readings (current text; D8's "scripting is disabled for *el's node document*"), states that they give the same outcome at N1 and that which one is implemented is N4's (edge n3); the invoke-gate removal reads "§8.1.3.4 gate" |
  | D8 `NodeDocumentOf` `:191-192` (compile gate = "scripting is disabled for *el's node document*") | **fixed (annotated)** — the variant stays (the fragment derivation still reads that form, §13.4 "contextDocument's scripting is disabled"); its compile-gate attribution carries the erratum and the n3 question. The same line's "§13.4 step 10" is not §8.1.8.1's and is left as it is (N9's; the record below) |
  | D8 `Node(el)` `:188` (§4.12.1.1 step 18 "If scripting is disabled for *el*") | unaffected — `webref body html prepare-the-script-element` step 18 reads "If scripting is disabled for el, then return." |
  | D8 `Settings` `:198` (§8.1.4.4 step 2) | unaffected — `webref body html check-if-we-can-run-script` step 2 reads "If scripting is disabled for settings, then return "do not run"." |
  | §1 N1 verdict `:347-349` | unaffected — names the invoke-time gate's removal, true under the current text |
  | J5 `:372` ("No production worker-realm reader exists after N1") | unaffected — re-derived in H-P15 and §0.4 D-N1: holds |
  | J13 `:392` | unaffected — names no step |
  | §3 §8.1.3.4 row `:428` | unaffected — lists the object forms, all still read by some algorithm; the settings conditions (now four) are not enumerated |
  | §3 §8.1.8.1 row `:430` | **fixed** — processing algorithm 1–4; step 4 → B1 |
  | §4 F10 `:482-483` (readers; "Invoke-time gate deleted (N1)") | **fixed (annotated)** — the compile reader points to D8's erratum note; the deletion is unaffected |
  | §5.1 `:528-529`, `:564-565` (D2's relaxation, "the spec's outcome (A17)") | unaffected — the corrected A17 still has no §8.1.3.4 invoke check, so C8b remains the spec's outcome |
  | §5.1 *Closed* D2 × D8 `:557` | **fixed (clerical)** — names the means (D-N3; C8c/C8d), PM decision 16 |
  | §5.2 n3 `:573` | **fixed** (rev 3 wording) — N4 decides which of the two readings the compile gate implements; same outcome at N1 |
  | §7 cells `:655-658` (C28b, C30, C31 named) | unaffected — the outcomes are the same under both texts (the equivalence above) |
  | §8 B1 append (2) `:708` | **fixed** — "step 4 (not fully active)" |
  | §8 records row `:758` (`#11-scripting-disabled-eventhandler-processing-step1`) | unaffected — a slot name (a label); the closure ground N1 restates is §8 item 1 below, which cites the current text |
  | §11 DoD (`:786-808`) | unaffected — no DoD row or checker cites §8.1.8.1 steps or the old quote |
  | input §B (a) `:42`, §C `:60` | unaffected — name the invoke gate, not a step |
  | input §D `:102` (step 3.1), `:105` (steps 3.1–3.2) | unaffected — step 3.1's text and the 3.1–3.2 numbering are unchanged |
  | input §E C7 | unaffected — §8.1.3.4's "global object is not a Window" condition is unchanged; the BiDi condition is vacuous |
  | input §E C8, C8b, C28b, C30 | unaffected — same outcomes under both texts |
  | input §E C31 (`click()`) | affected by H-P14, not by P-N1 — re-derived here (§6), the input file is not edited (approval scope `:10-14`) |
  | this memo (rev 1: §0.3, D-N1, D-N5, §3, §4.4, §7, §8, O1) | **fixed** in rev 2 — every hit cites the current text; §8 item 4 withdrawn |

  **§13.4 step number (recorded; PM decision 17 (iv): deferred to N9).** The node memo cites *the HTML fragment parsing
  algorithm*'s scripting-mode step as "§13.4 step 10" at approved `:192` (the D8 `NodeDocumentOf` line this PR touches),
  `:340` and `:436`, and the input file at `:104` (and "§13.4 steps 5, 10" at `:66`); `.claude/tools/webref body html
  parsing-html-fragments` numbers it step 8 ("If contextDocument's scripting is disabled, then set scriptingMode to
  Disabled"); step 5 (contextDocument = context's node document) is unchanged. N9's per-PR plan corrects the umbrella.
- **P-N2 — D2 × D8 is closed at N1 by D-N3 (PM decision 16: D-N3 is the closure's means).** The node memo's *Closed*
  entry (`:572-575`) and C30 say the reverse case "stays suppressed at every landing". That is true of a **raw** value.
  It is not true of HEAD's early-return *shape* once it carries D2 × D8's predicate and the invoke gate is gone (H-P1,
  H-P7): a node whose handler was compiled while live, moved into a DOMParser document and then given a new `onclick`
  attribute keeps the old callable in the store, and HEAD's gate returns before the overwrite, so dispatch runs the
  **old** function; a `removeAttribute('onclick')` on such a node returns before the `cleared` drop, so the removed
  handler runs. At HEAD both are suppressed by the invoke gate; in the spec the first reads null (step 3.2, the value is
  raw) and the second is null (*deactivate an event handler*). Removing the invoke gate without reordering would open a
  window **less restrictive than HEAD and than the spec**, which §5.1's interim-window claim (`:540-541`) forbids. The
  PM's decision: the closure stands, its means is D-N3 with C8c/C8d, and the node memo now names it (`:572-573`).
- **P-N3 — pointers.** The test pointers are function lines (H-P10), not assertion lines; and §5.3's "population (a)"
  (`:605`) only names the owner — the population is defined in input §B `:42`.
- **P-N4 — rev 1's own (Q-N3's premise).** Rev 1 asked whether "the deleted `VmObject` arm was [the worker realms']
  only §8.1.3.4 reader". It was not: worker and SW handlers dispatch through the `Node` arm (H-P15). The node memo's J5
  note (`:387`) is unaffected; its ground is restated in D-N1.
- **P-N5 — rev 2's own (H3 × H4's "the pending-source bit is exact"); R1 F1.** Rev 2 separated arm 2 (callback) from
  arm 3 (raw) by the pending-source bit and called the bit exact. At `fd6955e8` it is not: every insertion, a move
  included, sets it again from the content attribute (H-P16). Under D-N3 a handler compiled while live (or set through
  IDL while an `on*` attribute exists) and then moved into a DOMParser document would read as raw, and arm 3 would drop
  its callable — contradicting C8b(iv) and NR1. The defect is pre-existing (at HEAD a live → live move already replaces
  an IDL-set function by the attribute's source and a new function identity) and lies in the content-attribute consumer,
  not in N1's two gates; PM decision 17 (i) fixes it in the standalone prerequisite PR `EH-insert-once`, whose landing
  N1's implementation PR requires (§9). Everything in this memo that depends on "a move does not re-raw" says so (§2
  H3 × H4, §6 C8b(iv), C8c).

### §0.4 Decisions (this PR)

Inherited (node memo, decided): **D2** — the compile gate is *get the current value of the event handler* step 3.2,
whose two candidate readings (the current webref text; D8's "scripting is disabled for *el's node document*") give the
same outcome at N1, the choice between them being N4's (P-N1; PM decision 17 (ii)), and the invoke-time §8.1.3.4 gate
for event handlers is removed; **D8** at N1 — the compile gate reaches the node document "through HEAD's composition
(`elidex_script_session::scripting_disabled_for_platform_object`, given the element: settings clause first, then the
effective node document; HEAD's single realm makes it identical)" (`:268-271`), never through the stored owner alone;
**J13**'s N1–N4 qualifier; **§1 "N1 narrow"** — two axes, the invoke gate's removal and the compile gate's node
document. **D7** (E26 gates the node's first nested landing) was met before N2a (`d306af5a`, #541). PM decision 16
(the node memo corrected in place; D-N3 the closure's means; O5's home): P-N2, §7. PM decision 17 (`EH-insert-once`
first; D2 neutral; the unhandledrejection slot; §13.4 to N9): P-N5, §5.4 NR5, §7, §8, §9.

- **D-N1 — no invoke-time gate.** The three invoke gates (H-P2) are deleted, with their comments. Nothing replaces them:
  the processing algorithm has no §8.1.3.4 check (P-N1). Its step 1 (WebDriver BiDi) is vacuous in elidex (no BiDi
  surface, so the result is always true; §7) and step 4 (not fully active) stays unreachable until B1, as the node memo
  records (`:723`). The host wrapper `VmInner::scripting_disabled_for_platform_object` (`event_handler_attrs.rs:584-620`)
  loses its last callers (H-P3) and is deleted — input §C's first arm — together with its unbound fail-open arm
  (`:597-601`) and its `HostData`-present-but-unbound settings-only arm (`:603-612`). So no unbound fail-open path of the
  invoke family survives into N4 (input §C "No second unbound fail-open path survives N4"). **Worker realms**: the
  deleted gate's `Node` arm was their invoke-time reader (H-P15), and it never fired in production (no worker `HostData`
  receives flags); after N1 the only remaining §8.1.3.4 reader on a handler path is the compile gate, which D-N3
  evaluates only for a raw value (arm 3), and no worker global carries one (H-P6). So the node memo's J5 note "No
  production worker-realm reader exists after N1" (`:387`) holds, and in the spec a worker global's settings object is
  never scripting-disabled by the sandbox condition (§8.1.3.4's third condition) — no outcome changes for workers.
- **D-N2 — the compile gate reads HEAD's composition, given the element.** The gate is the closure
  `ensure_event_handler_current` passes to the session decision (D-N3):
  `|dom| elidex_script_session::scripting_disabled_for_platform_object(dom, entity, document, flags)`, with
  `document = HostData::document()` and `flags = HostData::sandbox_flags()` read before the call — marshalling only
  (Layering mandate). The session function evaluates the settings clause first (C28b), then derives the effective node
  document (the parent's interim; unresolvable → the node clause is false). `entity` is the handler's target: an
  element (step 3.1, document = its node document) or the Window entity (step 3.1, document = its associated Document;
  H-P6), for which the composition takes the settings clause alone — the Window clause cannot fire while bound (single
  realm), as at HEAD. **Signature** (O4, decided): `target` and `bound_document` narrow from `Option<Entity>` to
  `Entity`, deleting both `None` arms (`scripting.rs:92-94`) — after D-N1 neither has a caller (H-P18: every caller is
  bound, and a bound `HostData` always has its document; the only `None` target came from the deleted VM-walk gate's
  `VmObject` homes, `event_target_dispatch_vm.rs:245`), and CLAUDE.md's rule is that dead code is connected or deleted;
  N4 replaces the signature with `EcsDom::scripting_disabled_for` (N4 surface) over `ScriptingSubject` (N4 surface) (node
  memo §4 F10), whose `OtherPlatformObject` reading N4 writes for its own reader. **Unbound branch** — location: the
  head of `ensure_event_handler_current`, before any `EventListeners` read; form: with no `HostData` the reconcile
  returns (HEAD's no-op); with `HostData` present but unbound it is `debug_assert!(host.is_bound(), …)` and, in release,
  returns without reading, compiling or dropping — **fail-closed** (nothing compiles without the gate; the
  `EventListeners` component lives in the unbound DOM, so nothing else can be done). It is unreachable (H-P18), and
  nothing in N1 makes it reachable; the release no-op replaces HEAD's `HostData::dom()` panic on that state, which no
  caller produces.
- **D-N3 — the reconcile implements step 3 whole, in step order, decided engine-independently (PM decision 16: the means
  of D2 × D8's closure; R1 F4: the decision's home).** The arm order lives in a new session-crate function (Layering
  mandate: the HTML state machine is engine-independent and testable without a VM):

  ```rust
  // crates/script/elidex-script-session/src/event_handler_reconcile.rs (NEW)
  #[derive(Debug, Clone, PartialEq, Eq)]
  #[must_use]
  pub enum HandlerReconcile { // (NEW)
      Deactivated,     // arm 1
      Unchanged,       // arm 2
      Suppressed,      // arm 3
      Compile(String), // arm 4: the taken source
  }

  pub fn reconcile_event_handler( // (NEW)
      dom: &mut EcsDom,
      entity: Entity,
      id: ListenerId,
      scripting_disabled: impl FnOnce(&EcsDom) -> bool,
  ) -> HandlerReconcile;
  ```

  It reads `entity`'s `EventListeners` component (absent → `Unchanged`) and decides, in this order:
  1. **cleared** — `EventListeners::is_handler_cleared(id)` (content attribute removed; *deactivate an event handler*,
     which the spec runs at attribute-change time, independent of scripting) → `Deactivated`. The gate is not called.
  2. **no pending source** — `EventListeners::uncompiled_source(id)` is `None` (the value is a callback object, or null)
     → `Unchanged`: step 3 does not apply, and a callback value persists through every gate (Codex #327 R9's invariant,
     HEAD's comment `:536-541`). The gate is not called.
  3. **pending source** (the value is an *internal raw uncompiled handler*; read by the non-draining
     `uncompiled_source`, `event_listener.rs:282`) and `scripting_disabled(&dom)` is true → `Suppressed`; the source is
     **not** taken, so the value stays raw and a later read in a document where the gate does not fire compiles it
     (step 3.2 returns null and leaves the event handler's value as it is — C8's source oracle, C8c's second half, C8f).
  4. **pending source**, gate false → `EventListeners::take_uncompiled(id)` → `Compile(source)`.

  `vm/host/` only executes the VM-store action for the returned case: `Deactivated` and `Suppressed` →
  `remove_listener_and_prune_back_ref(id)` (for `Suppressed` this drops a **stale** callable — it is not the handler's
  value, the value is raw — and is a no-op when none is stored); `Unchanged` → nothing; `Compile(source)` →
  `lazy_compile_handler(&source)`, then `set_handler_callable` or, on a parse failure, `remove_listener_and_prune_back_ref`
  (HEAD's arm unchanged, steps 3.3–3.12). `set_uncompiled`, `mark_cleared` and `clear_uncompiled` keep pending source and
  `cleared` mutually exclusive (H-P7), so the four arms are total. Arms 1, 2 and 4 are HEAD's (HEAD reaches 1 and 2 only
  when the gate does not fire; with the gate firing they were unreachable at HEAD, because a no-`allow-scripts` realm
  never compiles anything, and reachable only through the invoke gate's removal plus the node clause, P-N2). Arm 3's
  drop is new and only drops a callable that step 3.2 says is not the value. The gate is a closure so that it is
  evaluated only in arm 3's position (H2 × H3) and so that N4 re-points it (n3) without touching the decision. Arm 2's
  "the value is not raw" is exact only once `EH-insert-once` has landed (P-N5).
- **D-N4 — `is_handler` stays and has one meaning: "reconcile this entry" (decided; O3).** After D-N1 the trait
  parameter `ScriptEngine::call_listener(.., is_handler, ..)` and `PendingListener::is_handler` lose their only reader
  (H-P8). Their plan-level source, `ListenerPlanEntry::is_handler`, is documented as the reconcile skip
  (`event_dispatch/mod.rs:234-241`) and is so used by the VM walk (`dispatch_target.rs:214`). N1 makes the other two
  paths read it the same way — `if is_handler { reconcile }` in `call_listener` and in the rejection dispatch — so the
  three paths have one form, and rewrites the trait doc (`elidex-script-session/src/engine.rs:71-75`) and the field doc
  (`natives_promise.rs:449-450`) to that meaning. Behaviour is unchanged: the reconcile is a no-op for a `Normal`
  listener.
- **D-N5 — docstrings and comments name the current algorithm.** Every comment or docstring that names the invoke gate
  or step 1 as a scripting gate is rewritten or deleted with its code (§5.2's list; G2). The composition's docstring
  (`scripting.rs:12-82`) is re-pointed from "the event handler processing algorithm step 1" to the compile gate —
  *get the current value of the event handler* step 3.2, quoted as it reads today, with P-N1's equivalence — and keeps
  the effective-node-document paragraph (the parent's interim, now read by the compile gate only); its "fails OPEN"
  sentence is narrowed to "the node clause is false; the settings clause has already been evaluated" (node memo D8), and
  its `None`-target / unbound sentences go with those arms (D-N2).
- **D-N6 — the migration population is behavioural (§5.1).** The tests whose outcome depends on the invoke gate are
  found by running the suites at two stages, not by vocabulary; each failure is classified by a fixed rule.

## §1. Shape and verdict

| Axis | Own invariant |
|---|---|
| Invoke | no §8.1.3.4 check at invocation on any of the three dispatch paths; one wrapper fewer (D-N1); `is_handler` = "reconcile this entry" on every path (D-N4, the deletion's completion) |
| Compile | step 3 in order, decided in the session crate: cleared → callback/null untouched → raw: gate (settings clause first, then the effective node document) → null with the source kept, else compile (D-N2, D-N3) |

**Plumbing (D-N4) is not a third axis.** It changes no outcome (the reconcile is a no-op for a `Normal` listener, and
every `is_handler` entry was already reconciled after the gate) and carries no invariant of its own: the parameter
loses its only reader when D-N1 deletes the gate, and D-N4 connects it to the reconcile guard instead of leaving it
unread (the dead-code rule). It is the Invoke axis's completion, so it sits in that row. The session-crate decision
(D-N3) is likewise the Compile axis's home, not an axis.

**Edge density: not edge-dense — single PR** (*verdict*). The node memo decided "N1 narrow — the two handler gates over
HEAD's store and composition, on two axes" (§1). Re-measured: one canonical algorithm (§8.1.8.1, the getter and the
processing algorithm); one new engine-independent function and its return enum (D-N3's decision moved out of
`vm/host/`), no new store, component, reader or crate edge (`elidex-js` already depends on `elidex-script-session`); the
reorder (D-N3) is step 3 of the same algorithm the compile gate implements, not a third axis; of §2's pairs the
intersecting ones all meet inside one decision, `reconcile_event_handler`, and its one host caller. It is a
narrowly-scoped slice under an approved node, the rule's base case (c); the plan-review is still mandatory (rule (b):
the node is edge-dense). The node-memo erratum this PR carries is a citation correction with no decision change (P-N1).
N1's implementation PR has one prerequisite outside the program, `EH-insert-once` (P-N5, §9). Breadth: K = 1 spec, M = 7
rows (§3); `webref coverage-map html 8.1.8.1 html 8.1.3.4` → "ok → single PR scope".

## §2. Coupled invariants

- **H1 No invocation gate** — the three dispatch paths invoke whatever *get the current value* returns; nothing between
  the reconcile and the call consults §8.1.3.4 (D-N1).
- **H2 Compile gate = HEAD's composition given the target** — settings clause first, then the parent's interim node
  clause (D-N2); identical to D8's `NodeDocumentOf(el)` / `Window(w)` in HEAD's single realm (node memo `:268-271`).
- **H3 Step order** — cleared before the gate; a callback value is never touched by the gate; a raw value under the
  gate reads null with no stale callable left reachable and its source kept (D-N3).
- **H4 Callback persistence** — a callback value (IDL-set, or compiled while the gate did not fire) survives any later
  gate firing, and runs (C8b) — the spec outcome D2 accepts as a relaxation (node memo `:543-544`).
- **H5 One composition** — the compile gate is the only N1 reader of §8.1.3.4 for handlers, through the one session
  function; `vm/host/` marshals (Layering mandate).
- **H6 Engine-independent decision** (R1 F4) — the arm order is decided by `reconcile_event_handler` (NEW) in
  `elidex-script-session` over the `EventListeners` component and the gate closure; `vm/host/` reads no arm (no
  `take_uncompiled` / `is_handler_cleared` / `uncompiled_source` call, G7) and only executes the VM-store action for the
  returned `HandlerReconcile` (NEW); the order is pinned by a VM-free unit test (C8f).

Pairs:
- **H1 × H3** — H1 removes the only check that used to sit after the reconcile, so every outcome the reconcile leaves
  becomes observable: the early return's skipped arms (P-N2) are exactly what H3 fixes. H1 without H3 runs a replaced or
  removed handler (C8c, C8d).
- **H1 × H4** — what H1 makes newly observable is H4's spec outcome (C8b) and nothing else; a test asserting otherwise
  is a flip (§5.1 class F).
- **H2 × H3** — the gate is evaluated only in arm 3's position (pending source), so the node clause never drops a
  callback value and never consults a document for a cleared handler; the gate's own clause order (settings first) is
  H2's, inside the session function.
- **H2 × H4** — H2 widens the gate to the node clause, so a callback value on a DOMParser node meets a gate that fires;
  H4 holds because that gate is never consulted for a non-raw value (arm 2 returns before the closure is called). H2
  can therefore change only raw outcomes (C8, C30), never C8b's.
- **H2 × H5** — the compile gate passes the target entity and nothing else to the composition; it derives no node
  document and, after D-N2's narrowing, has no `None` to branch on (J13's N1–N4 qualifier).
- **H3 × H4** — arm 3 drops a callable only when the value is raw; arm 2 is the callback case. The two are separated by
  the pending-source bit, which the attribute write (`set_uncompiled`), the compile (`take_uncompiled`) and the IDL
  setter (`clear_uncompiled`, `event_listener.rs:238`) keep exact (H-P7) **once `EH-insert-once` has landed** — at
  `fd6955e8` every insertion sets it again (H-P16, P-N5), so N1's implementation PR is based on that PR (§9).
- **H3 × H5** — arm 3 is the only position in which the composition is consulted, so "the compile gate is the only
  scripting-disabled reader on handler paths" (H5) and "the gate runs at most once per reconcile, for a raw value only" (H3) are
  the same placement; a second consultation (e.g. for a cleared handler) would be both an H3 and an H5 violation.
- **H6 × H1** — none beyond the shared call: H1 deletes gates around the call; H6 is the call's content.
- **H6 × H2** — H6 decides *when* the gate runs (closure called in arm 3 only), H2 *what* it computes; the closure is
  the seam, so N4 re-points H2 (n3) without touching H6.
- **H6 × H3** — H3 is implemented by H6: the step order lives only in `reconcile_event_handler`, and C8f's arm-order
  cells (gate spy; source after `Suppressed`) are H3's VM-free pin.
- **H6 × H4** — arm 2's "gate not called" is observable in C8f (the spy), so H4's protection is pinned below the VM as
  well as by C8b.
- **H6 × H5** — H6 evaluates no clause itself; the only §8.1.3.4 code on the path is the composition reached through the
  closure, so H5's single composition and H6's single decision are separate functions in the same crate, and `vm/host/`
  marshals into both.
- **H4 × H5**, **H1 × H2**, **H1 × H5** — none beyond the shared reconcile call.

Node-level pairs this PR touches: **J13** (the compile gate's node document, through HEAD's composition until N4,
`:407`); **D2 × D8** closed at N1 (`:572-575`) by H3 (D-N3; P-N2).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.8.1 Event handlers | *get the current value of the event handler* step 3.1 | element target → document = its node document (effective node document, the parent's interim); Window target → its associated Document (settings clause only while bound, H-P6) | `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` `ensure_event_handler_current` (the gate closure, D-N2) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *get the current value of the event handler* step 3.2 | browsing context null (node clause) ∨ sandboxed scripts flag (settings clause, evaluated first) → null, the raw value kept, a stale callable dropped (D-N3 arm 3) | `crates/script/elidex-script-session/src/event_handler_reconcile.rs` (NEW) `reconcile_event_handler` (the decision); `ensure_event_handler_current` (the drop) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *get the current value of the event handler* steps 3 (value not raw) and 4 | callback object → returned unchanged (arm 2); null → null; steps 3.3–3.12 compile (arm 4, HEAD's `lazy_compile_handler`) | the decision as above; `event_handler_attrs.rs:718` `lazy_compile_handler` (unchanged) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *deactivate an event handler* (from the attribute change steps, step 4) | content attribute removed → value null, not gated (D-N3 arm 1); the listener entry is kept flagged (HEAD's representation) | the decision (arm order); `crates/script/elidex-script-session/src/event_handler_consumer.rs` `mark_cleared` path (unchanged) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *the event handler processing algorithm* steps 1–4 | step 1 WebDriver BiDi → vacuous (no BiDi, the result is always true, §7); step 2 → the rows above; step 3 null → return; step 4 not fully active → B1 (node memo `:723`); **no §8.1.3.4 check** (D-N1) | deleted gates `crates/script/elidex-js/src/engine.rs:206-218`, `crates/script/elidex-js/src/vm/host/dispatch_target.rs:202-213`, `crates/script/elidex-js/src/vm/natives_promise.rs:534-542` | ✗ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *determine the target of an event handler* steps 3–4 | step 3 "node document is not an active document → null", step 4 the node document's relevant global — HEAD's `resolve_handler_target` has neither (pre-existing, A96); slot `#11-event-handler-target-determination` (§7); N1 does not touch it | `crates/script/elidex-script-session/src/event_handler_consumer.rs:328` (unchanged) | ✗ | yes |
| WHATWG HTML §8.1.3.4 Enabling and disabling scripting | platform-object predicate: settings clause; Node clause; Window clause; settings conditions 1–4 | settings clause = the sandboxed scripts flag only (UA support constant; no user switch; BiDi vacuous); Node clause = the parent's interim, unresolvable → false; Window clause never fires while bound; composition unchanged but for its two dead `None` arms (D-N2), docstring re-pointed (D-N5) | `crates/script/elidex-script-session/src/scripting.rs:83` | ✗ | no |

Numbers and titles: `webref heading html 8.1.8.1` (Event handlers, `#event-handler-attributes`), `8.1.3.4` (Enabling and
disabling scripting), `7.3.3` (Fully active documents); `webref dfn html` → §8.1.8.1 for `'get the current value of the
event handler'` (`#getting-the-current-value-of-the-event-handler`), `'The event handler processing algorithm'`
(`#the-event-handler-processing-algorithm`), `'internal raw uncompiled handler'`, `'deactivate an event handler'`,
`'activate an event handler'`, `'determine the target of an event handler'`
(`#determining-the-target-of-an-event-handler`); `'Scripting is disabled'` → §8.1.3.4 (`#concept-n-noscript`,
`#concept-environment-noscript`); `webref dfn webdriver-bidi 'WebDriver BiDi scripting is enabled'` → WebDriver BiDi
§7.4.2.10. Step texts: `.claude/tools/webref body html getting-the-current-value-of-the-event-handler`,
`the-event-handler-processing-algorithm`, `determining-the-target-of-an-event-handler`, `enabling-and-disabling-scripting`;
`.claude/tools/webref body webdriver-bidi webdriver-bidi-scripting-is-enabled` (2026-10-25). Breadth: K = 1, M = 7 rows.

### §3.1 User-input touch audit

Author-controlled inputs reaching the changed code: event-handler content attributes (`setAttribute('on…')`, parser-baked
attributes — in a DOMParser document these create no entry until the node is inserted into the live document, H-P13) —
the raw value, gated by D-N2 and ordered by D-N3; IDL handler assignment (`el.onclick = fn`) — a callback value, never
gated (H4); tree moves (`append`, `removeChild`) that change the effective node document between set and dispatch —
read at each reconcile (C30, C31), and, once `EH-insert-once` has landed, no longer turning the value raw again (P-N5);
`DOMParser` documents (the node clause). Every mismatch leaves the gate on the restrictive side except H4's callback
case, which is the spec's outcome. Adjacent pre-existing surface (slot `#11-event-handler-target-determination`, §7): a
`WindowEventHandlers` content attribute (`onload`, …) on a `<body>` whose node document is not active is installed on the
live Window entity at HEAD — not reached while a DOMParser document is built (H-P13); a post-build `setAttribute` on
such a body likely reaches it (the AttributeChange arm, `event_handler_consumer.rs:409`, calls `resolve_handler_target`
unconditionally), not demonstrated. N1 neither widens nor narrows it (HEAD's invoke gate gave the Window target the
settings clause alone, as N1's compile gate does).

## §4. Mechanism

### §4.1 Home / kind

| Fact | Home / kind | Status |
|---|---|---|
| reconcile decision (step 3's arm order) | `elidex_script_session::reconcile_event_handler` (NEW) → `HandlerReconcile` (NEW), `crates/script/elidex-script-session/src/event_handler_reconcile.rs` (NEW); a free function over the entity's `EventListeners` component and a gate closure; arms read `is_handler_cleared` / `uncompiled_source` / `take_uncompiled` (H-P17) | D-N3 (R1 F4) |
| VM-store action | `VmInner::ensure_event_handler_current`, `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` — marshals the DOM, the document and the flag set into the decision (the gate closure) and executes the returned case on the listener store | D-N2, D-N3 |
| compile gate | the closure over `elidex_script_session::scripting_disabled_for_platform_object` (node memo D8 at N1) | D-N2 |
| §8.1.3.4 composition | `elidex-script-session/src/scripting.rs` — `target` / `bound_document` narrowed to `Entity` (dead `None` arms deleted), docstring re-pointed | D-N2 (O4), D-N5; N4 moves it into `EcsDom` |
| handler value state | `EventListeners` `{uncompiled, cleared}` + the listener store (unchanged representation) | D-N3 reorders the reads |
| invoke gates, host wrapper | deleted | D-N1 |
| `is_handler` | "reconcile this entry" on all three paths | D-N4 (O3) |

One new function and one new enum, both engine-independent; no new store, component, trait or crate dependency.

### §4.2 Reconcile — HEAD → N1

| Case (value at the read) | HEAD (`event_handler_attrs.rs:521-581`) | N1 (`HandlerReconcile` → host action) |
|---|---|---|
| gate does not fire, raw | take, compile, store / drop on failure | `Compile(source)` → same (arm 4) |
| gate does not fire, cleared | drop callable | `Deactivated` → same (arm 1) |
| gate does not fire, callback | no-op | `Unchanged` → same (arm 2) |
| gate fires, raw, no stale callable | return (null) | `Suppressed` → the drop is a no-op (arm 3) |
| gate fires, raw, stale callable | return — the stale callable stays and is invoked, unless the invoke gate (HEAD) suppresses it | `Suppressed` → drop the stale callable, the source kept (arm 3) — C8c |
| gate fires, cleared | return — the callable stays, unless the invoke gate suppresses it | `Deactivated` → drop callable (arm 1) — C8d |
| gate fires, callback | return; callable stays, invoke gate suppresses it | `Unchanged`; it runs (arm 2; H4) — C8b |

"Gate fires" at HEAD = the HostData flag lacks `allow-scripts`; at N1 = the composition (that flag, then the node clause).
Under HEAD's gate the two stale rows were unreachable (no-`allow-scripts` realms never compile), so every N1 row with a
HEAD-reachable predecessor keeps HEAD's outcome except where D2 relaxes it (the callback row) or the spec narrows it
(the raw rows, now also under the node clause: C8, C30). The "callback" rows for a moved node hold once `EH-insert-once`
has landed (P-N5); on `fd6955e8` a move turns such a value raw first.

### §4.3 ECS-native and Layering checks

No per-entity state is added or moved; the handler value stays the `EventListeners` component plus the engine's
listener store, and D-N3 changes only the order of reads within one reconcile. The decision is a free function over the
component (the ECS-native form: a system-like read of one entity's component, returning a value the engine applies),
not a method on an engine object. OO → ECS: none new. Layering: the state machine (which arm) is in
`elidex-script-session`; `vm/host/event_handler_attrs.rs` marshals the bound DOM, the bound document and the flag set into
the decision's gate closure and executes the returned case on the VM's listener store (the engine-bound part, Layering
mandate); no clause and no arm is evaluated host-side (G7); the deleted wrapper was the one host-side clause fork (its
unbound settings-only arm). No `EcsDom::*` call is added in `vm/host/`.

### §4.4 Docstring citation table (Axis 4)

| Item | Cites |
|---|---|
| `reconcile_event_handler` (NEW), `HandlerReconcile` (NEW) | HTML §8.1.8.1 *get the current value of the event handler* step 3 (3.1, 3.2 quoted as current, with P-N1's equivalence to "scripting is disabled for *el's node document*"), 3.3–3.12 (the `Compile` arm), step 4; *deactivate an event handler* (the `Deactivated` arm) |
| `ensure_event_handler_current` | the same algorithm by name, pointing to `reconcile_event_handler` for the steps; §8.1.3.4 via the composition (the gate closure) |
| `scripting_disabled_for_platform_object` (session) | HTML §8.1.3.4 (platform-object predicate; settings clause first); used by §8.1.8.1 *get the current value* step 3.2 (P-N1's equivalence, one paragraph); the effective-node-document rule as the parent's interim (`#11-cross-document-adopt-on-insert`) |
| `ScriptEngine::call_listener` (`is_handler`) | HTML §8.1.8.1 *the event handler processing algorithm* (no scripting check at invocation; step 1 WebDriver BiDi vacuous in elidex, no BiDi surface); `is_handler` = run the *get the current value* reconcile |
| `DispatchTarget::resolve_callable`, the rejection dispatch | same as the row above |

## §5. Populations and named risks

### §5.1 Migration population (behavioural)

**(a′) := the tests whose outcome changes across two implementation stages**, recorded before any test is edited, on a
base that contains `EH-insert-once` (§9):

- **S1** — delete the three invoke gates and the host wrapper only (D-N1); leave the compile gate as at HEAD.
- **S2** — on top of S1, apply D-N2 and D-N3.

At each stage run `cargo test -p elidex-js --all-features`, `cargo test -p elidex-script-session --all-features` and
`cargo test -p elidex-shell --all-features`, and record every failing test. Each failure is classified:

- **F (flip)** — it asserts invoke-time suppression of a callback value, which H4 makes run: rewritten to C8b's
  expectation (the handler runs).
- **R (re-grounded)** — it fails at S1 and passes again at S2, because the compile gate's node clause now holds what the
  invoke gate held (a raw value): kept, comments re-grounded on C8.
- **N (new outcome)** — it fails at S2 only: a raw handler on a node whose effective node document is not the bound
  document now reads null from the getter (C8) where HEAD compiled it. Rewritten to C8 only if it is that case.
- **anything else → stop and record** in the PR description before changing it.

The implementation PR description records, per stage, the commands, the failing tests, and each one's class. Expected
from the vocabulary cross-check (input §B (a), H-P9, H-P10) — not the gate:

| Test (`vm/tests/tests_event_handler_attrs.rs` unless noted) | Expected |
|---|---|
| `:567` `compiled_handler_on_null_bc_document_node_suppressed_listener_still_runs` | F at S1 (`:584`): IDL-set handler on a DOMParser node runs → C8b |
| `:597` `suppressed_target_dispatch_does_not_compile_raw_inline_handler` | R: fails at S1 (`:607`, HEAD's flag-only compile gate compiles it), passes at S2; its store oracle (`:620-628`) checks only that **no callable** is stored, which a take-the-source arm 3 also satisfies, so N1 adds the source oracle (C8) |
| `:691` `appended_then_removed_domparser_node_suppressed_again` | F at S1 (`:706`, `"1"` → `"2"`) → C8b |
| `:719` `bound_created_node_appended_into_domparser_doc_suppressed` | F at S1 (`:733`) → C8b; C30 is the raw-form reverse case that stays suppressed |
| `:547` `sandboxed_no_allow_scripts_compile_gate_nulls_getter_and_dispatch` | passes at both (settings clause) |
| `:635` `step1_gate_does_not_suppress_bound_document_targets`, `:670` `appended_domparser_node_handler_runs_adopt_equivalent` | pass at both, but their IDL-set handlers no longer reach any gate, so they stop testing one: rewritten to raw form (O6, decided) |
| `tests_call_listener.rs:188` `call_listener_skips_inline_handler_when_scripting_disabled` | passes at both: the target is `dom.create_element` (owner `None`, detached) in a no-`allow-scripts` realm, so at S2 the settings clause refuses before the unresolvable node clause — **C28b** through the UA path |
| `tests_call_listener.rs:120` `call_listener_lazy_compiles_inline_handler` | passes at both: the same owner-`None` detached target, unsandboxed — the unresolvable node clause is false and the handler compiles (C28's shape; C28 itself is N4's cell) |
| `scripting.rs` tests `:135-153`, `:155-172` (session) | not a stage failure but a compile edit (D-N2's narrowing): the second `None`-target assertion (`:147-152`) and `none_target_or_unbound_document_fails_open` (`:155-172`) are deleted with the arms they test; the other cells drop `Some(..)` |

Every DOMParser test above writes its handler after the parse (H-P13), so the table's expectations do not depend on
parser-baked attributes. A test in this table that does not fail where expected, or a failure outside it, is recorded
with its class; the table does not limit the population.

### §5.2 Comment and docstring sites (D-N5)

`git grep -n -i -E 'event handler processing|processing step|step-1 (invoke |invocation )?gate|invoke gate|invocation gate|VmInner::scripting_disabled_for_platform_object|Self::scripting_disabled_for_platform_object'
fd6955e8 -- crates/script` → 20 lines (verified 2026-10-25), two of them unrelated (`vm/host/body_mixin.rs:205` "post-processing
step"; `vm/host_data/mod.rs:1138` "*cannot show simple dialogs* step-1 gate"). The other 18 are N1's: `engine.rs:206-207`;
`dispatch_target.rs:189`, `:203-204`; `event_handler_attrs.rs:539`, `:546` (inside the rationale block `:526-549`,
rewritten whole for D-N3); `natives_promise.rs:450`, `:534-535`; `elidex-script-session/src/engine.rs:74`;
`scripting.rs:14`; `tests_event_handler_attrs.rs:541`, `:564`, `:573`, `:593`, `:609`, `:632`. Outside the pattern but
in the same family: the wrapper's doc `event_handler_attrs.rs:584-592` (deleted with it); the reconcile's own doc
`event_handler_attrs.rs:511-520` (re-pointed to the session decision); the UA path's reconcile comment
`engine.rs:220-230`, which calls the reconcile "the scripting-disabled COMPILE chokepoint (… step 3.2): when scripting
is disabled it does NOT compile" — the old flag-only reading, rewritten to the current step 3.2 and the session
decision; the test header block `tests_event_handler_attrs.rs:538-543`; and `tests_call_listener.rs:179-185`, whose
step 3.2 paraphrase ("returns null when the sandboxed scripts flag is set") is re-cited as current (P-N1).
`event_target.rs:513` ("the internal event-handler processing algorithm") is the listener-identity rule, still correct,
not matched, not touched. G2 (§9) is the post-state check.

### §5.3 Touched files

Line counts verified 2026-10-25 (`wc -l` at the base): `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs`
(726; the wrapper and most of the rationale block go, ≈ 660 after), `crates/script/elidex-js/src/engine.rs` (831),
`crates/script/elidex-js/src/vm/host/dispatch_target.rs` (221), `crates/script/elidex-js/src/vm/natives_promise.rs`
(965), `crates/script/elidex-script-session/src/scripting.rs` (293; docstring, the two `None` arms and their tests),
`crates/script/elidex-script-session/src/engine.rs` (721, trait doc), `crates/script/elidex-script-session/src/lib.rs`
(62; `pub mod event_handler_reconcile` and its re-export),
`crates/script/elidex-script-session/src/event_handler_reconcile.rs` (NEW; ≈ 60 lines of code and ≈ 90 of C8f tests),
`crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs.rs` (735), `crates/script/elidex-js/src/tests_call_listener.rs`
(459; C8e's UA half, ≈ 485 after). Under O3's other option, `event_dispatch/mod.rs` (656) and the eight
`tests_call_listener.rs` call sites would join. This plan PR also edits `docs/plans/2026-09-ibp-sandbox.md` (the
erratum, P-N1; the `EH-insert-once` reference line, PM decision 17).

**`tests_event_handler_attrs.rs` — split at `:538`, decided** (touch-time rule: a file in the ~700–800-line band with a
real seam is split before the feature, as a standalone prerequisite commit). Projection without a split: 735 + N1's
cells (C8's source oracle ≈ 8, C8b(iv) ≈ 28, C8c ≈ 38, C8d ≈ 28, C30 ≈ 20, C8e's VM half ≈ 30, the `:635` / `:670`
raw-form rewrites ≈ 14) − the deleted invoke-gate comments ≈ 10 ⇒ **≈ 890 lines**, past the band. The seam is real: `:1-537`
are the IDL / content-attribute handler basics (F-1 … F-24); `:538-735` is the S5-4a scripting-gates section (its own
header block `:538-543`), exactly N1's subject. So the **first commit** of N1's implementation PR, before any behaviour
edit, moves `:538-735` verbatim into `crates/script/elidex-js/src/vm/tests/tests_event_handler_scripting_gates.rs` (NEW) (registered beside `tests_event_handler_attrs` in `vm/tests/mod.rs:67`), with the shared helpers `with_el`, `eval_bool`
and `entity_of` (`:38-68`) made `pub(super)` in the source file and imported by the new one: ≈ 537 and ≈ 220 lines after
the split, the new file ≈ 375 after N1. Every `tests_event_handler_attrs.rs:<line>` pointer in this memo at or after
`:538` is a `fd6955e8` coordinate of a function that the split moves into the new file; N1's cells are written there.
G4 checks both files.

### §5.4 Named risks

- **NR1 stale callable** — P-N2; mitigated by D-N3 and pinned by C8c and C8d. A reviewer reading HEAD's comment
  ("early-return WITHOUT deleting any stored callable", `:536-541`) may read arm 3's drop as its reversal: it is not —
  that comment protects a **callback** value (arm 2), and arm 3 drops only a callable that is not the value. That the
  pending-source bit identifies "not the value" exactly depends on `EH-insert-once` (P-N5).
- **NR2 behaviour population by vocabulary** — the node memo's (a) counts files by spelling; a test that asserts
  suppression through a helper or a shell fixture (e.g. a sandboxed-iframe dispatch in `elidex-shell`) is outside it.
  Mitigation: D-N6's two-stage run over three crates.
- **NR3 the getter on a DOMParser node changes** — HEAD compiles a raw handler set on a DOMParser node and its IDL getter
  returns a function (the invoke gate only stopped the call); from N1 the getter is null (C8). This is the spec's
  outcome (step 3.2), more restrictive than HEAD, and within §5.1's interim claim.
- **NR4 `natives_promise.rs` (965)** — N1 deletes lines; no split is triggered (G4).
- **NR5 the unhandledrejection target** — HEAD dispatches `unhandledrejection` at the Document entity
  (`natives_promise.rs:516-523`), not the Window; the compile gate then sees a Document target, which carries no raw
  handler (H-P6), so arm 3 never fires on that path. A HEAD fidelity gap outside N1's axes, recorded as slot
  `#11-unhandledrejection-dispatch-target` (owner core VM; reachability unconfirmed; re-eval 2026-11-30; registered by
  the PM, PM decision 17 (iii); §7).
- **NR6 worker realms** — the deleted VM-walk gate's `Node` arm also served worker and SW globals (H-P15). Its removal
  changes no worker outcome (D-N1); the S1 stage run (§5.1) includes `elidex-js`'s worker and SW suites, so a worker
  test that relied on it would appear there.
- **NR7 landing order** (R1 F1) — on a base without `EH-insert-once`, C8b(iv) fails (the move turns the compiled value
  raw and arm 3 drops it) and C8c's second half stops discriminating a take-the-source arm 3 (the re-insertion turns the
  attribute raw again). Mitigation: §9's prerequisite; the implementation PR's base contains that PR, and the (a′) run
  is made on it (§5.1).

## §6. Test and coverage plan

Declared surface: no WPT subset is newly declared (the node's `html/browsers/sandboxing/` subset is exercised from N5);
engine-independent equivalents in `elidex-js`'s VM suite, written in production form (script through `vm.eval`, the
compile gate reached by dispatch and by the IDL getter), as the node memo requires of C30 and C31 (`:611`), plus a
VM-free unit test of the decision in `elidex-script-session` (C8f). Dispatch in the VM cells is
`dispatchEvent(new Event('click'))` or `dispatchEvent(new MouseEvent('click'))`; the VM has no `click()` (H-P14). The
cells assume the post-`EH-insert-once` semantics: a move does not turn a handler raw again (P-N5).

**Source oracle** (used by C8 and C8c; R1 A2): the test reads the target's `EventListeners` component through
`vm.host_data().dom()` and asserts `EventListeners::uncompiled_source(id)` (non-draining, H-P17) equals the attribute's
source. Unlike the store oracle `:620-628` (no callable stored), it fails under an arm 3 that takes the source.

| # | Cell | Where |
|---|---|---|
| C8 | compile gate on an element of a DOMParser document: `d = dp.getElementById('d'); d.setAttribute('onclick', …)` → `d.onclick === null`, and dispatch runs nothing; no callable stored (store oracle as `:620-628`) **and the source still pending** (source oracle) — read after both the dispatch and the getter, so arm 3 is idempotent | `tests_event_handler_scripting_gates.rs` (re-grounds `:597`) |
| C8b | the handler runs (H4): (i) IDL-set on a DOMParser node (rewrites `:567`, whose `listenerRan` and `typeof … === 'function'` arms stay); (ii) IDL-set, appended into the live document, removed, dispatched twice → `"2"` (rewrites `:691`); (iii) a `createElement` node with an IDL-set handler appended into a DOMParser document (rewrites `:719`); (iv) (ADDED) `setAttribute('onclick', A)` on a live element, dispatched once (arm 4 compiles `A`; the source is taken), moved into a DOMParser document (the move leaves the value the callback `A`, P-N5), dispatched again → `A` runs (arm 2), and `typeof el.onclick === 'function'` | `tests_event_handler_scripting_gates.rs` |
| C8c (NEW) | stale callable after a re-set (D-N3 arm 3): a live element's raw handler `A` compiled by a dispatch, the element moved into a DOMParser document (still the callback `A`), `setAttribute('onclick', B)` (the value is raw `B`; the store still holds `A`) → dispatch runs neither `A` nor `B`, getter null, the source oracle reads `B`; then appended into the live document (the move leaves the value raw `B`; it does not re-read the attribute, P-N5) → dispatch runs `B` (arm 4). The second half discriminates a take-the-source arm 3: with the source taken nothing would be pending after the move and `B` would not run | `tests_event_handler_scripting_gates.rs` |
| C8d (NEW) | deactivation under the gate (D-N3 arm 1): as C8c to the move (the value is the callback `A`), then `removeAttribute('onclick')` → dispatch runs nothing, getter null. Holds whether or not a move re-raws (either way `mark_cleared` sets `cleared`) | `tests_event_handler_scripting_gates.rs` |
| C8f (NEW) | the decision without a VM (H6): an `EcsDom` entity with an `EventListeners` component and a gate spy (a closure that records its calls and returns a fixed verdict): (i) `set_uncompiled` then `mark_cleared`, spy → true: `Deactivated`, spy not called; (ii) a fresh `add_event_handler` entry (no source), spy → true: `Unchanged`, spy not called; (iii) `set_uncompiled(src)`, spy → true: `Suppressed`, spy called once, `uncompiled_source(id) == Some(src)` after, and a second call returns `Suppressed` again; (iv) `set_uncompiled(src)`, spy → false: `Compile(src)`, `uncompiled_source(id) == None` after; (v) an entity without `EventListeners`: `Unchanged`, spy not called | `elidex-script-session/src/event_handler_reconcile.rs` `#[cfg(test)]` |
| C28b | C28's sandboxed twin: a detached owner-`None` element with a raw handler in a no-`allow-scripts` realm → never compiled, never runs (the settings clause first) | `tests_call_listener.rs:188` (survives; renamed or annotated as C28b) |
| C30 | reverse case, production form: `b = document.createElement('button'); b.setAttribute('onclick', …); dp.body.append(b); b.dispatchEvent(new Event('click'))` → getter null, handler does not run | `tests_event_handler_scripting_gates.rs` (ADDED) |
| C31 | mirror case, production form: a DOMParser node appended into the live document, then `setAttribute('onclick', …)` and `dispatchEvent(new MouseEvent('click'))` → the handler runs (C29's oracle at N1; `:670` rewritten to it under O6). Re-derived from input §E C31, which names `click()` (H-P14) | `tests_event_handler_scripting_gates.rs` |
| C8e (NEW) | one form on every path (H1, D-N4): the same raw handler on a DOMParser node read through (i) the VM dispatch walk, (ii) `ScriptEngine::call_listener`, (iii) the IDL getter → null on all three; and an IDL-set handler there runs on (i) and (ii) | `tests_event_handler_scripting_gates.rs`, `tests_call_listener.rs` |
| — | `:547` (settings clause, sandboxed) and `tests_call_listener.rs:120` unchanged and green; `:635` rewritten to raw form under O6 (the detached-element and Window-`<body onpopstate>` arms compile and run; the Document arm stays IDL-set, there being no raw producer for a Document, H-P6) | per file |

**Mutation guards for D-N3** (each mutant of `reconcile_event_handler` and the cells it fails): restoring HEAD's single
early return (gate before every arm) fails C8c's first half (the stale `A` runs), C8d, and C8f (i) and (ii) (the spy is
called before any arm is read, and (i) does not return `Deactivated`); moving the cleared arm after the gate fails C8d and C8f (i); an arm 3 that takes the
source fails C8's source oracle, C8c's second half and C8f (iii); an arm 3 that does not drop the stale callable (the
host maps `Suppressed` to nothing) fails C8c's first half; consulting the gate for a callback value (arm 2 after the
gate) fails C8b (i)–(iv) and C8f (ii). C8e guards D-N1: re-adding an invoke gate on any path fails its IDL-set arm.
Commands: `cargo test -p elidex-js --all-features`, `cargo test -p elidex-script-session --all-features`, `cargo test -p
elidex-shell --all-features`; `mise run ci` before push.

## §7. Out of scope and slots

| Gap | Why not here | Owner | Re-eval |
|---|---|---|---|
| processing algorithm step 1 (WebDriver BiDi) and §8.1.3.4's fourth settings condition | elidex has no WebDriver BiDi; with no override the result is always true (`webref body webdriver-bidi webdriver-bidi-scripting-is-enabled`, step 6), so the step is vacuous | whoever adds a WebDriver BiDi surface, both together (no slot; O7, decided) | — |
| processing algorithm step 4 (not fully active) | unreachable in the single-browsing-context model | B1 append (2) (node memo `:723`) | B1's |
| *determine the target of an event handler* — step 3 (inactive node document → null), step 4 (the node document's relevant global vs HEAD's single `window_entity()`), and `<frameset>` (HEAD resolves only `<body>`, `is_body_element`) | a different algorithm (`event_handler_consumer`), pre-existing (A96), not widened by N1. Reachability: not reached while a DOMParser document is built (H-P13); a post-build `setAttribute` on such a body likely reaches it (AttributeChange arm `event_handler_consumer.rs:409`; the Insert arm `:461` also calls `resolve_handler_target`, `:328`, unconditionally), not demonstrated | slot `#11-event-handler-target-determination` (one slot for the three deviations; registered by the PM, PM decision 16) | the slot's |
| `unhandledrejection` dispatched at the Document, not the Window (NR5) | a HEAD fidelity gap of the rejection dispatch (`natives_promise.rs:516-523`), outside N1's axes; arm 3 cannot fire on that path (H-P6) | slot `#11-unhandledrejection-dispatch-target` (owner core VM; reachability unconfirmed; registered by the PM, PM decision 17 (iii)) | 2026-11-30 |
| the Insert arm turns handlers raw on every move (P-N5) | the content-attribute consumer, not N1's gates; pre-existing | the prerequisite PR `EH-insert-once` (PM decision 17 (i); lands before N1, §9) | — |
| the composition's move into `EcsDom` over `ScriptingSubject`; `NodeDocumentOf` / `Window` subjects; which step-3.2 reading the compile gate implements | N4 (edge n3) | N4 | N4's plan |
| the node clause's final representation | `#11-domparser-full-document-parse-fidelity` (the parent's) | that slot | its |
| insertion adoption (the effective-node-document proxy) | `#11-cross-document-adopt-on-insert` | that slot | 2026-10-31 (ledger) |

Own deferrals: **0** (node memo §5.2). N1 registers no slot; the PM registered `#11-event-handler-target-determination`
and `#11-unhandledrejection-dispatch-target` for pre-existing gaps (A96).

## §8. Edges, hand-offs and records

- **Prerequisite (outside the program's DAG)**: N1's implementation PR depends on `EH-insert-once`
  (`docs/plans/2026-10-event-handler-insert-once.md`, branch `layout-eh-insert-once`) landing first — PM decision 17 (i);
  H3 × H4's exact pending-source bit, C8b(iv) and C8c's second half hold only after it (P-N5, NR7). It is no node or
  edge of the node memo's DAG (the node memo carries one clerical reference line, no §5.2 change).
- **n3** (N1 → N4): N4 routes the compile gate, now the only handler reader, through `settings_route.rs` with
  `NodeDocumentOf(element)` / `Window(w)`, and promotes the effective-node-document rule into
  `EcsDom::effective_node_document` (N4 surface). N4 receives: D-N3's decision unchanged (the gate is the closure called
  in arm 3's position, which N4 re-points); the composition's signature already narrowed to `Entity` (D-N2; N4's
  `OtherPlatformObject` reading is N4's own); the choice between the two step-3.2 readings — the current text (the
  document's own flag set and browsing context) or D8's "scripting is disabled for *el's node document*" — for the text
  `NodeDocumentOf` implements, which give the same outcome at N1 (PM decision 17 (ii)), and with it P4's re-opened
  ground (node memo §5.2 n3 and D8's erratum note); and C28 / C29 over the gate (C31 is C29's oracle at N1).
- **Records — what N1's landing must record** (node memo §8 row `:773`; the ledgers are PM-owned, so N1's PR states the
  text and the PM applies it):
  1. `project_open-defer-slots.md`, `#11-scripting-disabled-eventhandler-processing-step1` (closed by S5-4a #444
     `24c9b0d4`): the closure ground is restated — the slot stays closed, but its gate is **removed** by N1, not kept:
     the current processing algorithm has no §8.1.3.4 check (its step 1 is WebDriver BiDi only, P-N1), so the closure is
     "no such step", ratified by D2; the compile gate (*get the current value* step 3.2) carries the scripting check.
  2. `project_open-defer-slots.md`, `#11-cross-document-adopt-on-insert`: its "§8.1.3.4 clause-(b) scripting-disabled
     gate" is now the **compile** gate only (the event-dispatch invocation gate is deleted, N1); the facets it lists
     (adopt-then-remove over-suppression, a live node appended into a detached foreign subtree, and their reverses)
     apply to whether a **raw** handler compiles, not to invoking a callback value, which runs wherever it moves (C8b).
  3. `project_s5-4-sandbox-kickoff.md:35` (S5-4a's "CLOSES `#11-scripting-disabled-eventhandler-processing-step1`"):
     append that the invoke-time gate S5-4a shipped is removed by N1 under D2 (node memo `:172`).
  4. *(withdrawn at rev 2)* — rev 1 left the node memo's wording to a later PM amendment; this PR carries the
     correction (P-N1).
  5. **Post-approval additions to the node memo's §8 slot records** (found after #532's approval; R1 A5). The node
     memo's §8 lists the slots it registers or appends to; two slots on N1's surface were registered **by the PM** after
     approval and are not in it: `#11-event-handler-target-determination` (PM decision 16; §7, A96) and
     `#11-unhandledrejection-dispatch-target` (PM decision 17 (iii); owner core VM; reachability unconfirmed; re-eval
     2026-11-30; NR5). Timing: both are ledger state already (registered at the PM's decision, before N1's landing), so
     under the node memo's rule "records that describe landed code or memory state ship with the nested PR whose
     landing makes them true" (`:18-19`, above `:24`, so the approved coordinate) nothing waits for N1's landing; this item is their record in
     the program's plan chain, and the node memo's §8 table is not edited (the A114 idiom; the erratum's exception is
     limited to the §8.1.8.1 citation).

## §9. Definition of done

**Prerequisites**: **`EH-insert-once` landed** (PM decision 17 (i); P-N5) — N1's implementation PR is based on a
`main` that contains it, and its description names that PR's merge commit; n3's other source (N2a) has landed
(`fd6955e8`); D7 met; N1 touches neither `dom/mod.rs` (E1) nor `elidex-html-parser/src/lib.rs` (E2).

Mechanical greps (each at the PR head):

- **G1** `git grep -n 'scripting_disabled_for_platform_object' -- crates ':!crates/script/elidex-script-session'` →
  exactly one line, the compile gate's closure in `event_handler_attrs.rs` (at the base: 10 lines, all in `elidex-js`, verified 2026-10-25 via this command; H-P2/H-P3).
- **G2** §5.2's command at the head (`-- crates/script`) → the two unrelated lines only (`body_mixin.rs`,
  `host_data/mod.rs`), plus any line that names processing step 1 as the WebDriver BiDi step (reviewer check over the
  hit list).
- **G3** `git grep -n 'scripts_allowed' -- crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` → 0 (the compile
  gate reads the composition, not the flag).
- **G4** `wc -l crates/script/elidex-js/src/vm/natives_promise.rs crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs.rs crates/script/elidex-js/src/vm/tests/tests_event_handler_scripting_gates.rs` (NEW) → each < 1000, and the split commit (§5.3) is the PR's first commit and moves lines only (`git show --stat <it>`; `git diff <it>~1 <it> --color-moved` shows the `:538-735` block moved).
- **G5** (O3) `git grep -n -w 'is_handler' -- crates/script ':!*tests*'` → every reader guards a reconcile call (reviewer
  check over the hit list); none guards a scripting check.
- **G6** `/usr/bin/grep -n -E 'scripting is disabled for document|steps 1–3|1–3; step 3|step 3 \(not fully|numbered it step 3' docs/plans/2026-09-ibp-sandbox.md | cut -d: -f1 | tr '\n' ' '`
  → `29 126 127 172 445 723` (the erratum note, A16, A17, D2, §3 row, B1 append (2); run on this PR's head, 2026-10-25),
  and the same command with `| /usr/bin/grep -c -v -E 'erratum|rev ≤ 12|Rev ≤ 12'` in place of the `cut | tr` → `0`
  (every hit carries the erratum marker on its own line).
- **G7** (H6) `git grep -n -E 'take_uncompiled|is_handler_cleared|uncompiled_source' -- crates/script/elidex-js/src ':!*tests*'`
  → 0 (the host reads no arm; at the base: 2 lines, `event_handler_attrs.rs:567-568`, H-P17).

Behavioural: the (a′) record (§5.1) is in the PR description; C8, C8b (i)–(iv), C8c, C8d, C8e, C8f, C28b, C30, C31
present and green; the three `cargo test -p` commands and `mise run ci` pass. Preflight: `python3
.claude/skills/elidex-plan-review/preflight.py docs/plans/2026-10-ibp-sandbox-handler-gates.md` exits 0.

## §10. Items formerly open

- *O1 — decided (PM decision 17 (ii)).* N1 cites the current step 3.2 verbatim and states its equivalence to D8's
  "scripting is disabled for *el's node document*" for every document N1 can see (single realm, no BiDi, no user switch);
  which reading `NodeDocumentOf` implements is N4's, on edge n3. The two differ in two places elidex cannot observe
  today — the flag set read is the document's own vs its realm's Window's (equal while the document has a browsing
  context, except the initial-`about:blank` Window reuse elidex does not model), and §8.1.3.4's user/BiDi conditions.
- *O2 — decided (PM decision 16, option (a)): D-N3's reorder is the means of D2 × D8's closure (P-N2).*
- *O3 — decided: (a) keep `is_handler`, meaning "reconcile this entry", on all three paths (D-N4).* (b) — delete the
  trait parameter and `PendingListener::is_handler` and reconcile unconditionally on those two paths — either leaves the
  VM walk's guard as a second form or, removing that too, leaves `ListenerPlanEntry::is_handler` without a reader (dead,
  to be deleted in a widening N1 does not need). (a) is the one form (One issue, one way) and keeps the plan field's
  documented purpose.
- *O4 — decided: narrow the composition's `target` and `bound_document` to `Entity` in N1, deleting both `None` arms
  (D-N2).* After D-N1 no caller passes `None` for either (H-P18); CLAUDE.md's rule is that dead code is connected or
  deleted, and there is nothing to connect it to: the VM-object homes it served reconcile nothing (`reconcile_handler`
  is `Node`-only, `dispatch_target.rs:181-185`), and N4's `OtherPlatformObject` reading is a new subject of a new
  signature, not this arm. Rev 2's "keep, because N4 undoes the narrowing" was a churn argument and is withdrawn.
- *O5 — decided (PM decision 16): home `#11-event-handler-target-determination` (§7).*
- *O6 — decided: (a) rewrite `:635` and `:670` into raw form so they pin the compile gate (`:670` becomes C31).* (b) —
  keep them as IDL-form regression pins of H1 — duplicates C8e's IDL-set arm, which pins H1 on all three paths, while the
  tests would pin no gate at all.
- *O7 — decided: (a) no slot for processing step 1 (WebDriver BiDi).* A slot records a gap in a surface elidex
  supports or plans; WebDriver BiDi is neither, and without it the step's result is always true (§7), so there is no
  gap to track; the step and §8.1.3.4's fourth condition belong to whoever adds a BiDi surface.

### §10.1 R1 review disposition

| Finding | Disposition |
|---|---|
| F1 (A2) Insert arm re-raws on every move | **fixed by dependency** — PM decision 17 (i): `EH-insert-once` first; P-N5, H-P16, H3 × H4, NR7, §8, §9; C8b(iv), C8c, C8d re-derived (§6) |
| F2 (A5) erratum's D2 wording settles O1 | **fixed** — node memo D2, erratum and n3 neutral; O1 decided by PM decision 17 (ii) |
| F3 (A3) unhandledrejection target, no slot | **fixed** — NR5 and §7 cite `#11-unhandledrejection-dispatch-target` |
| F4 (A1) arm order only in `vm/host/` | **fixed** — `reconcile_event_handler` / `HandlerReconcile` (NEW) in the session crate; H6; C8f; G7 |
| A1 MIN map arms to `EventListeners` APIs | **fixed** — D-N3 arms 1–4, H-P17 |
| A2 MIN arm-3 "source kept" has no discriminating oracle | **fixed** — C8's source oracle, C8c's second half (post-`EH-insert-once`), C8f (iii); §5.1 `:597` row corrected |
| A3 MIN §13.4 step 10 on the touched D8 line | **recorded** — §0.3 (PM decision 17 (iv): N9 corrects) |
| A3 MIN D-N2 unbound branch location | **fixed** — D-N2 (head of the reconcile; `debug_assert!` + fail-closed no-op), H-P18 |
| A3 MIN O4 "churn" argument | **fixed** — O4 decided on the dead-code rule (narrow) |
| A3 MIN §1 Plumbing as a third axis | **fixed** — §1 (the Invoke axis's completion) |
| A3 MIN §2 pairs H2 × H4, H3 × H5 missing | **fixed** — §2, plus H6's pairs |
| A3 MIN §4.4 BiDi "not implemented" vs "vacuous" | **fixed** — "vacuous" |
| A4 MIN G6 hits 2, not 6 | **fixed** — new pattern, run: 6 hits listed; unmarked count 0 |
| A4 MIN `engine.rs` old-reading comment not in §5.2 | **fixed** — `engine.rs:220-230` (the brief's `:219` is the blank line before it) |
| A4 MIN "without" quoted as spec | **fixed** — paraphrase, no quotation marks |
| A4 MIN erratum step-1 polarity | **fixed** — the erratum quotes step 1's text ("… is false, then return") |
| A5 MIN erratum in place vs `:12-14` / A114 | **fixed** — the erratum and P-N1 record the exception (PM decision 16) |
| A5 MIN new slot not in node memo §8 / timing rule | **fixed** — §8 records item 5 (post-approval additions; timing) |
| A5 MIN "PM decision 16" bare number | **fixed** — decisions 16 and 17 inlined (header; each citation glossed) |
| A5 MIN erratum pins "rev 2" | **fixed** — the erratum points to "N1 memo §0.3 P-N1" |
| A5 MIN `tests_event_handler_attrs.rs` projected 880–950 lines | **fixed** — projected ≈ 890; split at `:538` decided as the PR's first commit (§5.3, G4) |
| A5 MIN D-N2 "every caller" claim needs a command | **fixed** — H-P18 |
