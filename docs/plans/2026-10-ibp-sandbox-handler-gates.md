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
erratum, §0.3 P-N1; PM decision 16), which inserts 15 lines after its `:24` and rewrites lines in place, so every
node-memo coordinate above `:24` is the approved one and every one below is the approved one + 15 (`git diff 6270e219 --
docs/plans/2026-09-ibp-sandbox.md`). The input file is not edited.

**Rev 2** (2026-10-25) — folds PM decision 16 and the scout's answers to rev 1's three questions. (1) P-N1: the node
memo's citation of §8.1.8.1 is corrected **in this PR** (A16, A17, P4, D2, D8's `NodeDocumentOf` line, §3 row, §4 F10
readers, §5.2 n3, §8 B1 append (2), plus an erratum note); §0.3 records the sweep and each hit's disposition; §8 records
item 4 is withdrawn. (2) P-N2: the node memo's "closed at N1" stands, its means being D-N3 with C8c/C8d; O2 is decided
(option (a)) and the node memo names the means. (3) O5 is no longer open: its home is the slot
`#11-event-handler-target-determination` (§7), which the PM registers. (4) Q-N1–Q-N3 become premises H-P13–H-P15;
H-P15 corrects rev 1's claim that the deleted `VmObject` arm was the worker realms' §8.1.3.4 reader (it was the `Node`
arm); the node memo's J5 note `:387` still holds. (5) C31 uses `dispatchEvent(new MouseEvent('click'))` (no `click()`,
H-P14). *Rev 1* (2026-10-25) — first draft.

Five items are left **open for plan-review** (§10: O1, O3, O4, O6, O7); the body is written to the recommended option of
each and says so where it does.

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
`HostData::scripts_allowed`, `ElidexJsEngine::scripts_allowed`, `PendingListener::is_handler`) name existing methods or
fields inside `impl` / `struct` blocks, which preflight's literal symbol grep does not find (its soft warnings).

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
| H-P7 | The handler-state model: an `EventListeners` entry `{uncompiled, cleared}` plus the engine-side callable | `sed -n 165,290p crates/script/elidex-script-session/src/event_listener.rs`; `sed -n 40,62p crates/script/elidex-js/src/vm/inner.rs` | holds: `set_uncompiled` (`:207`) sets the source and resets `cleared`; `mark_cleared` (`:255`, content-attribute removal) drops the source and sets `cleared`; the callable lives in the listener store, dropped by `VmInner::remove_listener_and_prune_back_ref` (`inner.rs:49`) without touching the entry. A re-set attribute leaves the previous callable in the store until the next reconcile overwrites it |
| H-P8 | `is_handler` readers | `git grep -n -w 'is_handler' fd6955e8 -- crates` | 18 lines (verified 2026-10-25); readers: `engine.rs:211` (gate only), `dispatch_target.rs:202` (gate **and** the reconcile guard `:214`), `natives_promise.rs:540` (gate only; field `:451`, set `:491`); the plan field `ListenerPlanEntry::is_handler` (`elidex-script-session/src/event_dispatch/mod.rs:242`, documented as the reconcile skip `:234-241`) and the trait parameter (`elidex-script-session/src/engine.rs:83`, documented as the gate `:71-75`); 8 test call sites in `tests_call_listener.rs` |
| H-P9 | Population (a) (input §B) = 2 files | input §B's command at `fd6955e8` (both halves, `git grep -l … \| xargs /usr/bin/grep -l -E …`) | **holds**: `vm/tests/tests_event_handler_attrs.rs`, `tests_call_listener.rs` (verified 2026-10-25) — a vocabulary count; §5.1 defines the population by behaviour |
| H-P10 | The node memo's test pointers | `/usr/bin/grep -n '^fn ' crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs.rs crates/script/elidex-js/src/tests_call_listener.rs` | the pointers are **function** lines: `:567` `compiled_handler_on_null_bc_document_node_suppressed_listener_still_runs`, `:597` `suppressed_target_dispatch_does_not_compile_raw_inline_handler`, `:691` `appended_then_removed_domparser_node_suppressed_again`, `:719` `bound_created_node_appended_into_domparser_doc_suppressed`; `tests_call_listener.rs:188` `call_listener_skips_inline_handler_when_scripting_disabled` |
| H-P11 | Files N1 touches and the 1000-line rule | `wc -l` on §5.3's files | none >1000; watch `crates/script/elidex-js/src/vm/natives_promise.rs` (965; N1 deletes lines) |
| H-P12 | The eval gate is untouched by N1 | `sed -n 128,140p crates/script/elidex-js/src/engine.rs`; `:168-180` | holds: `ElidexJsEngine::scripts_allowed` (`:137-139`) feeds `eval` (`:174`) only; `HostData::scripts_allowed` (`vm/host_data/mod.rs:1107`) keeps that caller, and its field doc (`:197`) already names it "the eval gate" — N1 does not touch `host_data/mod.rs` |
| H-P13 | (rev 1's Q-N1) A DOMParser document's parser-baked handler attributes reach the content-attribute consumer | `/usr/bin/grep -n 'parse_into_inert_document' crates/script/elidex-js/src/vm/host/dom_parser.rs`; `sed -n 105,120p` and `sed -n 192,202p crates/dom/elidex-form/src/inert_document.rs` | **no**: `dom_parser.rs:296` calls `elidex_form::parse_into_inert_document`, which takes the live mutation dispatcher before the build (`inert_document.rs:112`) and restores it after (`:198-199`), so no mutation consumer runs on the throwaway document. A parser-baked `<div onclick>` in a DOMParser document carries **no** `EventListeners` entry (the spec gives it a raw value whose getter reads null — the same observable result); inserting that node into the live document fires Insert, and `handle_insert` (`event_handler_consumer.rs:427`) installs the raw handler then. The existing DOMParser handler tests (`tests_event_handler_attrs.rs` `:567`, `:597`, `:670`, `:691`, `:719`; their scripts at `:576`, `:601`, `:674`, `:695`, `:723`) all parse markup **without** handler attributes and write the handler after the parse (IDL setter or `setAttribute`), so none depends on this; no N1 cell has a parser-baked arm |
| H-P14 | (rev 1's Q-N2) the VM exposes `HTMLElement.prototype.click()` | `sed -n 140,153p crates/script/elidex-js/src/vm/host/html_element_proto.rs`; `/usr/bin/grep -rn 'mouse_event_global' crates/script/elidex-js/src` | **no**: `install_html_element_methods` (`html_element_proto.rs:146-152`) installs `focus` / `blur` only; `:144-145` and `:258-259` defer `click()` to `#11-vm-host-synthetic-dom-event-dispatch`. `dispatchEvent` is installed (`event_target.rs:68`), and the `MouseEvent` constructor is a registered global (`register_mouse_event_global`, `events_ui.rs:424`, called at `globals.rs:677` under `feature = "engine"`; exercised by `vm/tests/tests_ui_event_constructor.rs`). C31 uses `dispatchEvent(new MouseEvent('click'))` |
| H-P15 | (rev 1's Q-N3) worker / service-worker handlers are dispatched through `DispatchTarget::VmObject` | `/usr/bin/grep -rn 'resolve_callable' crates/script/elidex-js/src`; `/usr/bin/grep -n 'dispatch_script_event(&mut ctx' crates/script/elidex-js/src/vm/host/worker_scope.rs crates/script/elidex-js/src/vm/host/service_worker/event.rs`; `/usr/bin/grep -n 'DispatchTarget::' crates/script/elidex-js/src/vm/host/event_target_dispatch.rs crates/script/elidex-js/src/vm/host/event_target_dispatch_vm.rs` | **false (rev 1's claim)**: `resolve_callable` (`dispatch_target.rs:197`) has one non-test caller, `invoke_listeners_shared` (`event_target_dispatch.rs:470`). The worker path (`vm/host/worker_scope.rs:333`) and the SW path (`vm/host/service_worker/event.rs:192-211`, the global a `HostObject` entity) call `dispatch_script_event`, whose listener home is `DispatchTarget::Node` (`event_target_dispatch.rs:343`); `VmObject` homes are built only by the VM-object walk (`event_target_dispatch_vm.rs:245`). So the worker realms' §8.1.3.4 reader at HEAD is the **`Node` arm** of the deleted VM-walk gate, plus the compile gate's early return (H-P1), which runs for every handler reconcile. Neither fires in production: the only production sandbox-flag writer is the document pipeline (`git grep -n 'set_sandbox_flags' fd6955e8 -- crates ':!*tests*'` → `crates/shell/elidex-shell/src/pipeline.rs:281`, plus the trait and impl definitions), so a worker `HostData` keeps the default set |

### §0.3 Premise errors

- **P-N1 — the node memo's §8.1.8.1 citation; corrected in this PR (PM decision 16).** Rev 1 recorded this under the
  A114 idiom ("reported, not edited"); the PM decided to correct the node memo in N1's PR. Current webref reads:
  - `.claude/tools/webref body html getting-the-current-value-of-the-event-handler` — *get the current value of the
    event handler* step 3.2: "If document's browsing context is null, or document's active sandboxing flag set has its
    sandboxed scripts browsing context flag set, then return null."
  - `.claude/tools/webref body html the-event-handler-processing-algorithm` — *the event handler processing algorithm*
    steps 1–4: 1 "If the result of WebDriver BiDi scripting is enabled with eventTarget's relevant settings object is
    false, then return"; 2 callback = *get the current value*; 3 "If callback is null, then return"; 4 "If callback's
    associated realm's global object is a Window object whose associated Document is not fully active, then return".
  - `.claude/tools/webref body html enabling-and-disabling-scripting` — §8.1.3.4 lists four settings-object conditions,
    the fourth "The result of WebDriver BiDi scripting is enabled with settings is true" (`webref dfn webdriver-bidi
    'WebDriver BiDi scripting is enabled'` → WebDriver BiDi §7.4.2.10 The emulation.setScriptingEnabled Command).

  The node memo (rev ≤ 12) quoted step 3.2 as "If scripting is disabled for document, then return null" and numbered
  the processing algorithm 1–3 with "not fully active" at step 3. This is recorded as a premise error, not classified:
  webref keeps no spec history, so whether the spec changed or the earlier output was misread is not decided.
  **No decision changes at N1**, by this argument: elidex has no WebDriver BiDi and no user scripting switch, so
  §8.1.3.4's settings clause reduces to the sandboxed scripts flag of the settings object's Window's associated
  Document; in HEAD's single realm that flag store is the one `HostData` set, which is the bound document's; a document
  other than the bound document has a null browsing context (HEAD's single-browsing-context model, H-P4's docstring),
  so step 3.2's first disjunct returns null for it whatever its flags; and for the bound document the second disjunct
  reads the same set the settings clause reads. So "step 3.2 as written today" and "scripting is disabled for document"
  give the same result for every document N1 can see. N1 cites the current text and states this equivalence (§4.4);
  which text N4's `NodeDocumentOf` implements is open item **O1** (§10), recorded on edge n3.

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
  | D2 `:157-158` | **fixed** — quotes the current step 3.2; the invoke-gate removal reads "§8.1.3.4 gate"; points here for the equivalence |
  | D8 `NodeDocumentOf` `:191-192` (compile gate = "scripting is disabled for *el's node document*") | **fixed (annotated)** — the variant stays (the fragment derivation still reads that form, §13.4 "contextDocument's scripting is disabled"); its compile-gate attribution carries the erratum and the n3 question |
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
  | §5.2 n3 `:573` | **fixed** — n3 carries the O1 question |
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

  Outside the brief (not fixed; reported to the PM): the node memo cites *the HTML fragment parsing algorithm*'s
  scripting-mode step as "§13.4 step 10" (approved `:192`, `:340`, `:436`; input §D `:104`), but `webref body html
  parsing-html-fragments` numbers it step 8 ("If contextDocument's scripting is disabled, then set scriptingMode to
  Disabled"); step 5 (contextDocument = context's node document) is unchanged. It is N9's citation, not §8.1.8.1's.
- **P-N2 — D2 × D8 is closed at N1 by D-N3 (PM decision 16).** The node memo's *Closed* entry (`:572-575`) and C30 say
  the reverse case "stays suppressed at every landing". That is true of a **raw** value. It is not true of HEAD's
  early-return *shape* once it carries D2 × D8's predicate and the invoke gate is gone (H-P1, H-P7): a node whose handler
  was compiled while live, moved into a DOMParser document and then given a new `onclick` attribute keeps the old
  callable in the store, and HEAD's gate returns before the overwrite, so dispatch runs the **old** function; a
  `removeAttribute('onclick')` on such a node returns before the `cleared` drop, so the removed handler runs. At HEAD
  both are suppressed by the invoke gate; in the spec the first reads null (step 3.2, the value is raw) and the second
  is null (*deactivate an event handler*). Removing the invoke gate without reordering would open a window **less
  restrictive than HEAD and than the spec**, which §5.1's interim-window claim (`:540-541`) forbids. The PM's decision:
  the closure stands, its means is D-N3 with C8c/C8d, and the node memo now names it (`:572-573`).
- **P-N3 — pointers.** The test pointers are function lines (H-P10), not assertion lines; and §5.3's "population (a)"
  (`:605`) only names the owner — the population is defined in input §B `:42`.
- **P-N4 — rev 1's own (Q-N3's premise).** Rev 1 asked whether "the deleted `VmObject` arm was [the worker realms']
  only §8.1.3.4 reader". It was not: worker and SW handlers dispatch through the `Node` arm (H-P15). The node memo's J5
  note (`:387`) is unaffected; its ground is restated in D-N1.

### §0.4 Decisions (this PR)

Inherited (node memo, decided): **D2** — the compile gate follows the current spec (step 3.2 as current webref reads
it, P-N1), and the invoke-time §8.1.3.4 gate for event handlers is removed; **D8** at N1 — the compile gate reaches the
node document "through HEAD's composition (`elidex_script_session::scripting_disabled_for_platform_object`, given the
element: settings clause first, then the effective node document; HEAD's single realm makes it identical)"
(`:268-271`), never through the stored owner alone; **J13**'s N1–N4 qualifier; **§1 "N1 narrow"** — two axes, the
invoke gate's removal and the compile gate's node document. **D7** (E26 gates the node's first nested landing) was met
before N2a (`d306af5a`, #541). PM decision 16: D-N3 is the means of D2 × D8's closure (P-N2); O5's home is
`#11-event-handler-target-determination` (§7).

- **D-N1 — no invoke-time gate.** The three invoke gates (H-P2) are deleted, with their comments. Nothing replaces them:
  the processing algorithm has no §8.1.3.4 check (P-N1). Its step 1 (WebDriver BiDi) is vacuous in elidex (no BiDi
  surface; §7) and step 4 (not fully active) stays unreachable until B1, as the node memo records (`:723`). The host
  wrapper `VmInner::scripting_disabled_for_platform_object` (`event_handler_attrs.rs:584-620`) loses its last callers
  (H-P3) and is deleted — input §C's first arm — together with its unbound fail-open arm (`:597-601`) and its
  `HostData`-present-but-unbound settings-only arm (`:603-612`). So no unbound fail-open path of the invoke family
  survives into N4 (input §C "No second unbound fail-open path survives N4"). **Worker realms**: the deleted gate's
  `Node` arm was their invoke-time reader (H-P15), and it never fired in production (no worker `HostData` receives
  flags); after N1 the only remaining §8.1.3.4 reader on a handler path is the compile gate, which D-N3 evaluates only
  for a raw value (arm 3), and no worker global carries one (H-P6). So the node memo's J5 note "No production
  worker-realm reader exists after N1" (`:387`) holds, and in the spec a worker global's settings object is never
  scripting-disabled by the sandbox condition (§8.1.3.4's third condition) — no outcome changes for workers.
- **D-N2 — the compile gate reads HEAD's composition, given the element.** In `ensure_event_handler_current` the flag
  read `hd.scripts_allowed()` (`:550-556`) is replaced by one call
  `elidex_script_session::scripting_disabled_for_platform_object(dom, Some(entity), document, flags)` with the bound
  DOM, `HostData::document_entity_opt()` and `HostData::sandbox_flags()` — marshalling only (Layering mandate). The
  session function evaluates the settings clause first (C28b), then derives the effective node document (the parent's
  interim; unresolvable → the node clause is false). `entity` is the handler's target: an element (step 3.1, document
  = its node document) or the Window entity (step 3.1, document = its associated Document; H-P6), for which the
  composition takes the settings clause alone — the Window clause cannot fire while bound (single realm), as at HEAD.
  The session function keeps its signature (`target: Option<Entity>`, `bound_document: Option<Entity>`): N4 replaces it
  with `EcsDom::scripting_disabled_for` (N4 surface) over `ScriptingSubject` (N4 surface) (node memo §4 F10). Its
  `None`-target arm has no production caller after D-N1 (it served the VM walk's `DispatchTarget::VmObject` homes —
  VM-object event targets, `event_target_dispatch_vm.rs:245`, not worker globals, H-P15); it is the settings-only
  reading N4's `OtherPlatformObject` keeps, so it stays (open item **O4**). An unbound reconcile compiles nothing (there
  is no DOM to read); it is not reached today (every caller dispatches on a bound VM), and nothing in N1 makes it
  reachable.
- **D-N3 — the reconcile implements step 3 whole, in step order (decided: PM decision 16; the means of D2 × D8's
  closure).** `ensure_event_handler_current` becomes, for an event-handler listener `id` on `entity`:
  1. **cleared** (content attribute removed; *deactivate an event handler*, which the spec runs at attribute-change
     time, independent of scripting) → drop the stored callable; the value is null. Not gated.
  2. **no pending source** (the value is a callback object, or null) → no-op: step 3 does not apply, and a callback
     value persists through every gate (Codex #327 R9's invariant, HEAD's comment `:536-541`).
  3. **pending source** (the value is an *internal raw uncompiled handler*) and the D-N2 gate fires → drop any
     **stale** stored callable (it is not the handler's value — the value is the raw handler), **keep** the source
     (step 3.2 returns null "without" changing the value, so a later read in a document where the gate does not fire
     compiles it — C8c's second half, C31's shape), return.
  4. **pending source**, gate does not fire → take, compile (steps 3.3–3.12), store or drop on parse failure — HEAD's
     arm unchanged.

  `set_uncompiled` and `mark_cleared` keep pending source and `cleared` mutually exclusive (H-P7), so the four arms are
  total. Arms 1, 2 and 4 are HEAD's (HEAD reaches 1 and 2 only when the gate does not fire; with the gate firing they
  were unreachable at HEAD, because a no-`allow-scripts` realm never compiles anything, and reachable only through the
  invoke gate's removal plus the node clause, P-N2). Arm 3's drop is new and only drops a callable that step 3.2 says is
  not the value.
- **D-N4 — `is_handler` stays and has one meaning: "reconcile this entry" (recommended; O3).** After D-N1 the trait
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
  sentence is narrowed to "the node clause is false; the settings clause has already been evaluated" (node memo D8).
- **D-N6 — the migration population is behavioural (§5.1).** The tests whose outcome depends on the invoke gate are
  found by running the suites at two stages, not by vocabulary; each failure is classified by a fixed rule.

## §1. Shape and verdict

| Axis | Own invariant |
|---|---|
| Invoke | no §8.1.3.4 check at invocation on any of the three dispatch paths; one wrapper fewer (D-N1) |
| Compile | step 3 in order: cleared → callback/null untouched → raw: gate (settings clause first, then the effective node document) → null with the source kept, else compile (D-N2, D-N3) |
| Plumbing | `is_handler` = "reconcile this entry" on every path (D-N4) |

**Edge density: not edge-dense — single PR** (*verdict*). The node memo decided "N1 narrow — the two handler gates over
HEAD's store and composition, on two axes" (§1). Re-measured: one canonical algorithm (§8.1.8.1, the getter and the
processing algorithm), no new type, store, reader or crate edge; the reorder (D-N3) is step 3 of the same algorithm the
compile gate implements, not a third axis; of §2's pairs the intersecting ones all meet inside one function,
`ensure_event_handler_current`. It is a narrowly-scoped slice under an approved node, the rule's base case (c); the
plan-review is still mandatory (rule (b): the node is edge-dense). The node-memo erratum this PR carries is a citation
correction with no decision change (P-N1). Breadth: K = 1 spec, M = 7 rows (§3); `webref coverage-map html 8.1.8.1
html 8.1.3.4` → "ok → single PR scope".

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

Pairs:
- **H1 × H3** — H1 removes the only check that used to sit after the reconcile, so every outcome the reconcile leaves
  becomes observable: the early return's skipped arms (P-N2) are exactly what H3 fixes. H1 without H3 runs a replaced or
  removed handler (C8c, C8d).
- **H1 × H4** — what H1 makes newly observable is H4's spec outcome (C8b) and nothing else; a test asserting otherwise
  is a flip (§5.1 class F).
- **H2 × H3** — the gate is evaluated only in arm 3's position (pending source), so the node clause never drops a
  callback value and never consults a document for a cleared handler; the gate's own clause order (settings first) is
  H2's, inside the session function.
- **H2 × H5** — the compile gate passes the target entity and nothing else to the composition; it derives no node
  document and branches on no `None` (J13's N1–N4 qualifier).
- **H3 × H4** — arm 3 drops a callable only when the value is raw; arm 2 is the callback case. The two are separated by
  the pending-source bit, which `set_uncompiled` / the IDL setter (`clear_uncompiled`, `event_listener.rs:238`) keep
  exact (H-P7).
- **H4 × H5**, **H1 × H2**, **H1 × H5** — none beyond the shared reconcile call.

Node-level pairs this PR touches: **J13** (the compile gate's node document, through HEAD's composition until N4,
`:407`); **D2 × D8** closed at N1 (`:572-575`) by H3 (D-N3; P-N2).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.8.1 Event handlers | *get the current value of the event handler* step 3.1 | element target → document = its node document (effective node document, the parent's interim); Window target → its associated Document (settings clause only while bound, H-P6) | `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` `ensure_event_handler_current` (the gate, D-N2) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *get the current value of the event handler* step 3.2 | browsing context null (node clause) ∨ sandboxed scripts flag (settings clause, evaluated first) → null, the raw value kept, a stale callable dropped (D-N3 arm 3) | same | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *get the current value of the event handler* steps 3 (value not raw) and 4 | callback object → returned unchanged (arm 2); null → null; steps 3.3–3.12 compile (arm 4, HEAD's `lazy_compile_handler`) | same; `event_handler_attrs.rs:718` `lazy_compile_handler` (unchanged) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *deactivate an event handler* (from the attribute change steps, step 4) | content attribute removed → value null, not gated (D-N3 arm 1); the listener entry is kept flagged (HEAD's representation) | `ensure_event_handler_current` (arm order); `crates/script/elidex-script-session/src/event_handler_consumer.rs` `mark_cleared` path (unchanged) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *the event handler processing algorithm* steps 1–4 | step 1 WebDriver BiDi → vacuous (no BiDi, §7); step 2 → the rows above; step 3 null → return; step 4 not fully active → B1 (node memo `:723`); **no §8.1.3.4 check** (D-N1) | deleted gates `crates/script/elidex-js/src/engine.rs:206-218`, `crates/script/elidex-js/src/vm/host/dispatch_target.rs:202-213`, `crates/script/elidex-js/src/vm/natives_promise.rs:534-542` | ✗ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *determine the target of an event handler* steps 3–4 | step 3 "node document is not an active document → null", step 4 the node document's relevant global — HEAD's `resolve_handler_target` has neither (pre-existing, A96); slot `#11-event-handler-target-determination` (§7); N1 does not touch it | `crates/script/elidex-script-session/src/event_handler_consumer.rs:328` (unchanged) | ✗ | yes |
| WHATWG HTML §8.1.3.4 Enabling and disabling scripting | platform-object predicate: settings clause; Node clause; Window clause; settings conditions 1–4 | settings clause = the sandboxed scripts flag only (UA support constant; no user switch; no BiDi); Node clause = the parent's interim, unresolvable → false; Window clause never fires while bound; composition unchanged, docstring re-pointed (D-N5) | `crates/script/elidex-script-session/src/scripting.rs:83` | ✗ | no |

Numbers and titles: `webref heading html 8.1.8.1` (Event handlers, `#event-handler-attributes`), `8.1.3.4` (Enabling and
disabling scripting), `7.3.3` (Fully active documents); `webref dfn html` → §8.1.8.1 for `'get the current value of the
event handler'` (`#getting-the-current-value-of-the-event-handler`), `'The event handler processing algorithm'`
(`#the-event-handler-processing-algorithm`), `'internal raw uncompiled handler'`, `'deactivate an event handler'`,
`'activate an event handler'`, `'determine the target of an event handler'`
(`#determining-the-target-of-an-event-handler`); `'Scripting is disabled'` → §8.1.3.4 (`#concept-n-noscript`,
`#concept-environment-noscript`); `webref dfn webdriver-bidi 'WebDriver BiDi scripting is enabled'` → WebDriver BiDi
§7.4.2.10. Step texts: `.claude/tools/webref body html getting-the-current-value-of-the-event-handler`,
`the-event-handler-processing-algorithm`, `determining-the-target-of-an-event-handler`, `enabling-and-disabling-scripting`
(2026-10-25). Breadth: K = 1, M = 7 rows.

### §3.1 User-input touch audit

Author-controlled inputs reaching the changed code: event-handler content attributes (`setAttribute('on…')`, parser-baked
attributes — in a DOMParser document these create no entry until the node is inserted into the live document, H-P13) —
the raw value, gated by D-N2 and ordered by D-N3; IDL handler assignment (`el.onclick = fn`) — a callback value, never
gated (H4); tree moves (`append`, `removeChild`) that change the effective node document between set and dispatch —
read at each reconcile (C30, C31); `DOMParser` documents (the node clause). Every mismatch leaves the gate on the
restrictive side except H4's callback case, which is the spec's outcome. Adjacent pre-existing surface (slot
`#11-event-handler-target-determination`, §7): a `WindowEventHandlers` content attribute (`onload`, …) on a `<body>`
whose node document is not active is installed on the live Window entity at HEAD — not reached while a DOMParser
document is built (H-P13); a post-build `setAttribute` on such a body likely reaches it (the AttributeChange arm,
`event_handler_consumer.rs:409`, calls `resolve_handler_target` unconditionally), not demonstrated. N1 neither widens
nor narrows it (HEAD's invoke gate gave the Window target the settings clause alone, as N1's compile gate does).

## §4. Mechanism

### §4.1 Home / kind

| Fact | Home / kind | Status |
|---|---|---|
| compile gate | `VmInner::ensure_event_handler_current`, `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` — marshals into `elidex_script_session::scripting_disabled_for_platform_object` | D-N2 (node memo D8 at N1) |
| §8.1.3.4 composition | `elidex-script-session/src/scripting.rs` — unchanged code, docstring re-pointed | D-N5; N4 moves it into `EcsDom` |
| handler value state | `EventListeners` `{uncompiled, cleared}` + the listener store (unchanged representation) | D-N3 reorders the reads |
| invoke gates, host wrapper | deleted | D-N1 |
| `is_handler` | "reconcile this entry" on all three paths | D-N4 (O3) |

No new type, store, component, trait or crate dependency.

### §4.2 Reconcile — HEAD → N1

| Case (value at the read) | HEAD (`event_handler_attrs.rs:521-581`) | N1 |
|---|---|---|
| gate does not fire, raw | take, compile, store / drop on failure | same (arm 4) |
| gate does not fire, cleared | drop callable | same (arm 1) |
| gate does not fire, callback | no-op | same (arm 2) |
| gate fires, raw, no stale callable | return (null) | same (arm 3; nothing to drop) |
| gate fires, raw, stale callable | return — the stale callable stays and is invoked, unless the invoke gate (HEAD) suppresses it | drop the stale callable, keep the source (arm 3) — C8c |
| gate fires, cleared | return — the callable stays, unless the invoke gate suppresses it | drop callable (arm 1) — C8d |
| gate fires, callback | return; callable stays, invoke gate suppresses it | no-op; it runs (arm 2; H4) — C8b |

"Gate fires" at HEAD = the HostData flag lacks `allow-scripts`; at N1 = the composition (that flag, then the node clause).
Under HEAD's gate the two stale rows were unreachable (no-`allow-scripts` realms never compile), so every N1 row with a
HEAD-reachable predecessor keeps HEAD's outcome except where D2 relaxes it (the callback row) or the spec narrows it
(the raw rows, now also under the node clause: C8, C30).

### §4.3 ECS-native and Layering checks

No per-entity state is added or moved; the handler value stays the `EventListeners` component plus the engine's
listener store, and D-N3 changes only the order of reads within one reconcile. OO → ECS: none new. Layering:
`vm/host/event_handler_attrs.rs` marshals the bound DOM, the bound document and the flag set into the session function;
no clause is evaluated host-side; the deleted wrapper was the one host-side clause fork (its unbound settings-only arm).
No `EcsDom::*` call is added in `vm/host/`.

### §4.4 Docstring citation table (Axis 4)

| Item | Cites |
|---|---|
| `ensure_event_handler_current` | HTML §8.1.8.1 *get the current value of the event handler* step 3 (3.1, 3.2 quoted as current), 3.3–3.12 (compile), step 4; *deactivate an event handler* (the cleared arm); §8.1.3.4 via the composition |
| `scripting_disabled_for_platform_object` (session) | HTML §8.1.3.4 (platform-object predicate; settings clause first); used by §8.1.8.1 *get the current value* step 3.2 (P-N1's equivalence, one paragraph); the effective-node-document rule as the parent's interim (`#11-cross-document-adopt-on-insert`) |
| `ScriptEngine::call_listener` (`is_handler`) | HTML §8.1.8.1 *the event handler processing algorithm* (no scripting check at invocation; step 1 WebDriver BiDi not implemented); `is_handler` = run the *get the current value* reconcile |
| `DispatchTarget::resolve_callable`, the rejection dispatch | same as the row above |

## §5. Populations and named risks

### §5.1 Migration population (behavioural)

**(a′) := the tests whose outcome changes across two implementation stages**, recorded before any test is edited:

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
| `:597` `suppressed_target_dispatch_does_not_compile_raw_inline_handler` | R: fails at S1 (`:607`, HEAD's flag-only compile gate compiles it), passes at S2; its store oracle (`:620-628`) still holds because arm 3 never takes the source |
| `:691` `appended_then_removed_domparser_node_suppressed_again` | F at S1 (`:706`, `"1"` → `"2"`) → C8b |
| `:719` `bound_created_node_appended_into_domparser_doc_suppressed` | F at S1 (`:733`) → C8b; C30 is the raw-form reverse case that stays suppressed |
| `:547` `sandboxed_no_allow_scripts_compile_gate_nulls_getter_and_dispatch` | passes at both (settings clause) |
| `:635` `step1_gate_does_not_suppress_bound_document_targets`, `:670` `appended_domparser_node_handler_runs_adopt_equivalent` | pass at both, but their IDL-set handlers no longer reach any gate, so they stop testing one: rewritten to raw form (open item **O6**) |
| `tests_call_listener.rs:188` `call_listener_skips_inline_handler_when_scripting_disabled` | passes at both: the target is `dom.create_element` (owner `None`, detached) in a no-`allow-scripts` realm, so at S2 the settings clause refuses before the unresolvable node clause — **C28b** through the UA path |
| `tests_call_listener.rs:120` `call_listener_lazy_compiles_inline_handler` | passes at both: the same owner-`None` detached target, unsandboxed — the unresolvable node clause is false and the handler compiles (C28's shape; C28 itself is N4's cell) |

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
in the same family: the wrapper's doc `event_handler_attrs.rs:584-592` (deleted with it), the test header block
`tests_event_handler_attrs.rs:538-543`, and `tests_call_listener.rs:179-185`, whose step 3.2 paraphrase ("returns null
when the sandboxed scripts flag is set") is re-cited as current (P-N1). `event_target.rs:513` ("the internal
event-handler processing algorithm") is the listener-identity rule, still correct, not matched, not touched. G2 (§9) is the
post-state check.

### §5.3 Touched files

Line counts verified 2026-10-25 (`wc -l` at the base): `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` (726), `crates/script/elidex-js/src/engine.rs` (831),
`crates/script/elidex-js/src/vm/host/dispatch_target.rs` (221), `crates/script/elidex-js/src/vm/natives_promise.rs`
(965), `crates/script/elidex-script-session/src/scripting.rs` (293, docstring),
`crates/script/elidex-script-session/src/engine.rs` (721, trait doc), `crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs.rs`
(735; the new cells add tests, expected to stay < 1000), `crates/script/elidex-js/src/tests_call_listener.rs` (459). Under O3's other option, `event_dispatch/mod.rs` (656) and the eight
`tests_call_listener.rs` call sites would join. This plan PR also edits `docs/plans/2026-09-ibp-sandbox.md` (the
erratum, P-N1).

### §5.4 Named risks

- **NR1 stale callable** — P-N2; mitigated by D-N3 and pinned by C8c and C8d. A reviewer reading HEAD's comment
  ("early-return WITHOUT deleting any stored callable", `:536-541`) may read arm 3's drop as its reversal: it is not —
  that comment protects a **callback** value (arm 2), and arm 3 drops only a callable that is not the value.
- **NR2 behaviour population by vocabulary** — the node memo's (a) counts files by spelling; a test that asserts
  suppression through a helper or a shell fixture (e.g. a sandboxed-iframe dispatch in `elidex-shell`) is outside it.
  Mitigation: D-N6's two-stage run over three crates.
- **NR3 the getter on a DOMParser node changes** — HEAD compiles a raw handler set on a DOMParser node and its IDL getter
  returns a function (the invoke gate only stopped the call); from N1 the getter is null (C8). This is the spec's
  outcome (step 3.2), more restrictive than HEAD, and within §5.1's interim claim.
- **NR4 `natives_promise.rs` (965)** — N1 deletes lines; no split is triggered (G4).
- **NR5 the unhandledrejection target** — HEAD dispatches `unhandledrejection` at the Document entity
  (`natives_promise.rs:516-523`), not the Window; the compile gate then sees a Document target, which carries no raw
  handler (H-P6), so arm 3 never fires on that path. A HEAD fidelity gap outside N1's axes; not recorded as N1's.
- **NR6 worker realms** — the deleted VM-walk gate's `Node` arm also served worker and SW globals (H-P15). Its removal
  changes no worker outcome (D-N1); the S1 stage run (§5.1) includes `elidex-js`'s worker and SW suites, so a worker
  test that relied on it would appear there.

## §6. Test and coverage plan

Declared surface: no WPT subset is newly declared (the node's `html/browsers/sandboxing/` subset is exercised from N5);
engine-independent equivalents in `elidex-js`'s VM suite, written in production form (script through `vm.eval`, the
compile gate reached by dispatch and by the IDL getter), as the node memo requires of C30 and C31 (`:611`). Dispatch in
the cells is `dispatchEvent(new Event('click'))` or `dispatchEvent(new MouseEvent('click'))`; the VM has no `click()`
(H-P14).

| # | Cell | Where |
|---|---|---|
| C8 | compile gate on an element of a DOMParser document: `d = dp.getElementById('d'); d.setAttribute('onclick', …)` → `d.onclick === null`, and dispatch runs nothing; the source is still pending (store oracle as `:620-628`) | `tests_event_handler_attrs.rs` (re-grounds `:597`) |
| C8b | the handler runs (H4): (i) IDL-set on a DOMParser node (rewrites `:567`, whose `listenerRan` and `typeof … === 'function'` arms stay); (ii) IDL-set, appended into the live document, removed, dispatched twice → `"2"` (rewrites `:691`); (iii) a `createElement` node with an IDL-set handler appended into a DOMParser document (rewrites `:719`); (iv) (ADDED) `setAttribute('onclick', …)` on a live element, dispatched once (compiles), moved into a DOMParser document, dispatched again → runs | `tests_event_handler_attrs.rs` |
| C8c (NEW) | stale callable after a re-set (D-N3 arm 3): a live element's raw handler `A` compiled by a dispatch, the element moved into a DOMParser document, `setAttribute('onclick', B)` → dispatch runs neither `A` nor `B`, getter null; then appended into the live document → dispatch runs `B` | `tests_event_handler_attrs.rs` |
| C8d (NEW) | deactivation under the gate (D-N3 arm 1): as C8c to the move, then `removeAttribute('onclick')` → dispatch runs nothing, getter null | `tests_event_handler_attrs.rs` |
| C28b | C28's sandboxed twin: a detached owner-`None` element with a raw handler in a no-`allow-scripts` realm → never compiled, never runs (the settings clause first) | `tests_call_listener.rs:188` (survives; renamed or annotated as C28b) |
| C30 | reverse case, production form: `b = document.createElement('button'); b.setAttribute('onclick', …); dp.body.append(b); b.dispatchEvent(new Event('click'))` → getter null, handler does not run | `tests_event_handler_attrs.rs` (ADDED) |
| C31 | mirror case, production form: a DOMParser node appended into the live document, then `setAttribute('onclick', …)` and `dispatchEvent(new MouseEvent('click'))` → the handler runs (C29's oracle at N1; `:670` rewritten to it under O6). Re-derived from input §E C31, which names `click()` (H-P14) | `tests_event_handler_attrs.rs` |
| C8e (NEW) | one form on every path (H1, D-N4): the same raw handler on a DOMParser node read through (i) the VM dispatch walk, (ii) `ScriptEngine::call_listener`, (iii) the IDL getter → null on all three; and an IDL-set handler there runs on (i) and (ii) | `tests_event_handler_attrs.rs`, `tests_call_listener.rs` |
| — | `:547` (settings clause, sandboxed) and `tests_call_listener.rs:120` unchanged and green; `:635` rewritten to raw form under O6 (the detached-element and Window-`<body onpopstate>` arms compile and run; the Document arm stays IDL-set, there being no raw producer for a Document, H-P6) | per file |

C8c and C8d are the mutation guards for D-N3: restoring HEAD's single early return fails both; moving the cleared arm
after the gate fails C8d only; taking the source on a firing gate fails C8c's second half. C8e guards D-N1: re-adding an
invoke gate on any path fails its IDL-set arm. Commands: `cargo test -p elidex-js --all-features`, `cargo test -p
elidex-script-session --all-features`, `cargo test -p elidex-shell --all-features`; `mise run ci` before push.

## §7. Out of scope and slots

| Gap | Why not here | Owner | Re-eval |
|---|---|---|---|
| processing algorithm step 1 (WebDriver BiDi) and §8.1.3.4's fourth settings condition | elidex has no WebDriver BiDi; the step is vacuous | whoever adds a WebDriver BiDi surface, both together (no slot; O7) | — |
| processing algorithm step 4 (not fully active) | unreachable in the single-browsing-context model | B1 append (2) (node memo `:723`) | B1's |
| *determine the target of an event handler* — step 3 (inactive node document → null), step 4 (the node document's relevant global vs HEAD's single `window_entity()`), and `<frameset>` (HEAD resolves only `<body>`, `is_body_element`) | a different algorithm (`event_handler_consumer`), pre-existing (A96), not widened by N1. Reachability: not reached while a DOMParser document is built (H-P13); a post-build `setAttribute` on such a body likely reaches it (AttributeChange arm `event_handler_consumer.rs:409`; the Insert arm `:461` also calls `resolve_handler_target`, `:328`, unconditionally), not demonstrated | slot `#11-event-handler-target-determination` (one slot for the three deviations; registered by the PM, PM decision 16) | the slot's |
| the composition's move into `EcsDom` over `ScriptingSubject`; `NodeDocumentOf` / `Window` subjects | N4 (edge n3) | N4 | N4's plan |
| the node clause's final representation | `#11-domparser-full-document-parse-fidelity` (the parent's) | that slot | its |
| insertion adoption (the effective-node-document proxy) | `#11-cross-document-adopt-on-insert` | that slot | 2026-10-31 (ledger) |

Own deferrals: **0** (node memo §5.2). N1 registers no slot; the PM registers `#11-event-handler-target-determination`
for a pre-existing gap (A96).

## §8. Edges, hand-offs and records

- **n3** (N1 → N4): N4 routes the compile gate, now the only handler reader, through `settings_route.rs` with
  `NodeDocumentOf(element)` / `Window(w)`, and promotes the effective-node-document rule into
  `EcsDom::effective_node_document` (N4 surface). N4 receives: D-N3's arm order unchanged (the gate is one call in arm 3's
  position); the session function's `None`-target arm (O4); O1 — whether the current step 3.2 (the document's own flag
  set and browsing context) or D8's "scripting is disabled for *el's node document*" is the text `NodeDocumentOf`
  implements for the compile gate, and with it P4's re-opened ground (node memo §5.2 n3 and D8's erratum note); and C28
  / C29 over the gate (C31 is C29's oracle at N1).
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

## §9. Definition of done

**Prerequisites**: none open — n3's other source (N2a) has landed (`fd6955e8`); D7 met; N1 touches neither
`dom/mod.rs` (E1) nor `elidex-html-parser/src/lib.rs` (E2).

Mechanical greps (each at the PR head):

- **G1** `git grep -n 'scripting_disabled_for_platform_object' -- crates ':!crates/script/elidex-script-session'` →
  exactly one line, the compile gate's call in `event_handler_attrs.rs` (at the base: 10 lines, all in `elidex-js`, verified 2026-10-25 via this command; H-P2/H-P3).
- **G2** §5.2's command at the head (`-- crates/script`) → the two unrelated lines only (`body_mixin.rs`,
  `host_data/mod.rs`), plus any line that names processing step 1 as the WebDriver BiDi step (reviewer check over the
  hit list).
- **G3** `git grep -n 'scripts_allowed' -- crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` → 0 (the compile
  gate reads the composition, not the flag).
- **G4** `wc -l crates/script/elidex-js/src/vm/natives_promise.rs crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs.rs` → each < 1000.
- **G5** (O3 as recommended) `git grep -n -w 'is_handler' -- crates/script ':!*tests*'` → every reader guards a
  reconcile call (reviewer check over the hit list); none guards a scripting check.
- **G6** `/usr/bin/grep -n -E 'scripting is disabled for document, then|processing algorithm 1–3|step 3 \(not fully' docs/plans/2026-09-ibp-sandbox.md`
  → only the erratum-marked lines (A16, A17, D2, §3 row, B1 append (2), the erratum note), each beside the current text
  (reviewer check over the hit list).

Behavioural: the (a′) record (§5.1) is in the PR description; C8, C8b (i)–(iv), C8c, C8d, C8e, C28b, C30, C31 present
and green; the three `cargo test -p` commands and `mise run ci` pass. Preflight: `python3
.claude/skills/elidex-plan-review/preflight.py docs/plans/2026-10-ibp-sandbox-handler-gates.md` exits 0.

## §10. Open for plan-review

- **O1 — which text the compile gate implements (P-N1).** (a) Cite the current step 3.2 verbatim and argue its
  equivalence to D8's "scripting is disabled for *el's node document*" for every document N1 can see (single realm, no
  BiDi, no user switch) — **recommended** for N1, which changes no outcome either way; (b) stop N1 until N4's reading is
  settled. Either way N4's `NodeDocumentOf` reading (settings clause over the document's relevant realm's associated
  Document, then the node clause) differs from the current text in two places that cannot be observed in elidex today —
  the flag set read is the document's own vs its realm's Window's (equal while the document has a browsing context,
  except the initial-`about:blank` Window reuse elidex does not model), and §8.1.3.4's user/BiDi conditions — so the
  question is N4's to settle, recorded on edge n3 (node memo `:588`).
- *O2 — decided (PM decision 16, option (a)): D-N3's reorder is the means of D2 × D8's closure (P-N2).*
- **O3 — `is_handler` after the gates go (D-N4).** (a) Keep it, meaning "reconcile this entry", on all three paths —
  **recommended** (one form, matching `ListenerPlanEntry::is_handler`'s documented purpose and the VM walk); (b) delete
  the trait parameter and `PendingListener::is_handler` and reconcile unconditionally on those two paths — a smaller
  surface, but it leaves the VM walk's guard as a second form (or removes it too, making the plan field dead).
- **O4 — the composition's `None` target arm (D-N2).** After D-N1 no production caller passes `None`. (a) Keep it until
  N4 replaces the signature (it is the settings-only reading N4's `OtherPlatformObject` keeps) — **recommended**;
  (b) narrow to `Entity` in N1 — churn N4 undoes.
- *O5 — decided (PM decision 16): home `#11-event-handler-target-determination` (§7).*
- **O6 — the two tests that stop testing a gate (`:635`, `:670`).** (a) Rewrite into raw form so they pin the compile
  gate (`:670` becomes C31) — **recommended**; (b) keep them as IDL-form regression pins of H1.
- **O7 — processing step 1 (WebDriver BiDi).** (a) No slot: no BiDi surface exists or is planned, and the step is
  vacuous — **recommended**; (b) register a slot now.
