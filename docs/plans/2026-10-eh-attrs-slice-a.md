# EH-attrs-at-creation slice a: per-PR plan (rev 3)

Label **`EH-attrs-at-creation-a`**. Program and contract: the approved umbrella
`docs/plans/2026-10-event-handler-attrs-at-creation.md` (rev 8, landed by PR #551 at `b9122157`). Branch
`layout-eh-attrs-slice-a`, cut from `origin/main` `b9122157`. Owner: Layout lane.

This plan **derives the implementation** of the decided contract. It re-decides no clause the umbrella decided (its
"Approval scope" paragraph); the one umbrella line this PR corrects is by PM decision 35 (§0.5, "marker home").
Where the umbrella left something to "slice a's plan" or "slice a's plan-review", this
plan decides it and says so (§0.5). Where the contract text and the code disagree, §0.6 records it. Changes since rev 1:
§10.

## §0. Scope

### §0.1 What this PR implements (umbrella anchors)

| umbrella clause | what slice a builds | this plan |
|---|---|---|
| D-1, §4 Home/kind | `HandlerAttrsProcessed` (NEW) unit marker on the origin element | §1.2, §1.4 |
| D-2 | one engine-independent processing function, ordinary pairs once-gated, Window-delegating pairs per X-W | §1.4 (a) |
| D-3, K-0 timing table | exactly T-a1 (bind), T-a2 (Insert), T-a3 (first handler write) | §1.4 |
| D-4, D-6, D-8 | write-side entry, called by the AttributeChange arm and both VM element setters, with the observable restriction | §1.4 (b), §1.5 |
| D-5, D-11, K-4, K-6 | bind pass keyed on the bound document, `PreBindHandlerAttrsProcessed` (NEW) on the Document entity, uncapped iterative shadow-including walk excluding template contents | §1.4 (c), §1.6 |
| D-10 | Insert arm keeps HEAD's walk set, adds the once check | §1.4 (d) |
| X-W | the (tag, name) classifier and its use at every timing | §1.4 (e) |
| K-5 | bind pass reaches no DOMParser document and no template contents; W-K5 kept | §1.6, cells U8 |
| §4's rewrite-site list, G6 | docstrings, module docs, clone-policy rows, `vm_api.rs` comment | §1.7 |
| §6.1, §6.2, §6.3 | cells C1–C9, C11–C20 (C19b, C19c, C20 (i)/(ii)), U1–U12, pins E1–E8, mutation lines | §4 |
| §10.1 | DoD | §7 |

### §0.2 Out of scope

- **T3** (`#11-event-handler-target-determination` facets 1, 3, 4; facet 2): no change to `resolve_handler_target`'s
  target logic. Step 3 stays unmodelled (X-W keeps HEAD's path).
- **Slice b** (S-1 to S-7): no creation-time call sites, no detector, no removal of the D-8 restriction or X-W.
- **IBP-sandbox N1**: no change to either scripting gate (`ensure_event_handler_current`, `scripting.rs`).
- **C1** (`#11-template-contents-owner-document`): template contents keep the bound document as owner (W-K5).
- E-G1 (i)–(iii) exposures: registered elsewhere (umbrella §8.1); not fixed here.
- G7 (`#11-unbound-attribute-writes-bypass-consumers`), `#11-cross-document-adopt-on-insert`: untouched.
- Workers: `Vm::bind_worker` (`crates/script/elidex-js/src/vm/vm_api.rs:382`) installs no dispatcher and no
  consumer, so it has no bind pass. A worker scope has no elements.
- **Bind-pass reach set.** The bind pass runs inside `Vm::bind`, so it runs exactly where `Vm::bind` runs. Its only
  production caller is `ElidexJsEngine`'s `HostDriver::bind` (`crates/script/elidex-js/src/engine.rs:378`, the call at
  `:397`), and that has two callers: `teardown_document` (`engine.rs:412`, bind at `:419`) and `with_bound`
  (`engine.rs:425`, bind at `:440`). Enumeration:
  `/usr/bin/grep -rnE --include='*.rs' '\.bind\(\s*($|&|std::ptr::|ctx\)|session)' crates` (38 hits at `534ca697`).
  Production hits that reach `Vm::bind` are exactly `engine.rs:397`, `:419`, `:440`. The rest are:
  `HostData::bind` inside `Vm::bind` / `Vm::bind_worker` (`vm/vm_api.rs:217`, `:393`) and in `host_data/mod.rs:1937`
  (`#[cfg(test)]`); the Wasm runtime's own state (`crates/script/elidex-wasm-runtime/src/host/state.rs:125`, `:167`,
  `:183`; not a `Vm`); `#[cfg(test)]` modules (`tests_call_listener.rs:45`, `:140`, `:214`, `:439`,
  `tests_dispatch_integration.rs:49`, `tests_engine_s1a.rs:42`, `s1b.rs:41`, `s1c.rs:36`, `s1d.rs:43`, `s2.rs:33`,
  `s6a.rs:36`, and under `vm/tests/`: `tests_add_event_listener.rs:215`, `:266`, `tests_visual_viewport.rs:35`,
  `:467`, `:487`,
  `tests_match_media.rs:395`, `:801`, `:814`, `:857`, `:872`, `tests_match_media_keepalive.rs:39`, `:186`, `:198`, `:230`,
  `:251`); the benches `crates/script/elidex-js/benches/event_dispatch.rs:55` and
  `dom_collection.rs:114`; and `bind_vm` (`crates/script/elidex-js/src/vm/test_helpers.rs:49-54`, the call at `:54`).
  `bind_vm` is compiled into the library (`#[cfg(feature = "engine")] #[doc(hidden)] pub mod test_helpers`,
  `vm/mod.rs:82-84`) so the bench crate can use it, but every caller is a test or a bench
  (`/usr/bin/grep -rln 'bind_vm(' crates --include='*.rs' | /usr/bin/grep -vE '/tests/|/tests_|benches/|test_helpers\.rs'`
  → no output). So the bind pass runs in production only under `with_bound` and `teardown_document`.
  `Drop` (`engine.rs:828`) calls `Vm::teardown_document` without binding,
  so it runs no bind pass. **Teardown-first bind**: `teardown_document` binds before tearing down, so if a document's
  first bind were its teardown, the pass would process a document that is being destroyed. Production never does
  this: the only production `ElidexJsEngine::new()` is `run_scripts_and_finalize` (`crates/shell/elidex-shell/src/pipeline.rs:226`),
  which opens a `with_bound` bracket unconditionally (`pipeline.rs:349-362`, no early return between `:226` and it) before it returns the engine to any
  `PipelineResult`, so every `teardown_document` (`lib.rs:475` and its shell callers) follows a first bind. If a test
  tears down first, the bind pass runs and its writes stay: `Vm::teardown_document`
  (`crates/script/elidex-js/src/vm/lifecycle.rs:536`) closes realtime connections, terminates workers, releases
  document-lifetime VM state and unbinds; it does not despawn the DOM. The document and its elements stay in `EcsDom`,
  so the writes (handler state in `EventListeners`, the element and document markers) are observable through ECS. A
  later bind of the same document does not reprocess: the document marker makes its bind pass a no-op. No code handles
  that case specially.

### §0.3 Coordinate frame

Every `path:line` below is at `b9122157`. The umbrella's coordinates are at `fd6955e8`;
`git diff --stat fd6955e8 b9122157 -- crates` is empty (run 2026-10-26), so every umbrella coordinate holds at this
plan's base. All greps use `/usr/bin/grep`. Every § number ↔ title pair and step number comes from
`.claude/tools/webref` (`coverage-map`, `dfn`, `body`, `idl`), run 2026-10-26.

### §0.4 Premises re-verified at `b9122157`

| id | claim | evidence |
|---|---|---|
| A-1 | `Vm::bind` resolves or creates the Window **before** building the dispatcher, then calls `initialize_consumers`, then installs | `vm_api.rs:224-230` (Window), `:292-297` (`ConsumerDispatcher::new`), `:313` (`initialize_consumers`), `:314` (install) |
| A-2 | The bound document is in scope at the init call | `document` parameter of `Vm::bind` (`vm_api.rs:199`), passed to `hd.bind` at `:217` |
| A-3 | `initialize_consumers` has exactly one caller | `/usr/bin/grep -rn --include='*.rs' -E 'initialize_consumers' crates` → `consumer_dispatcher.rs:135` (def), `vm_api.rs:313` (call), plus comments `vm_api.rs:312`, `tests_base_href.rs:504` |
| A-4 | `BaseUrlMaintainer`'s `initialize_from_tree` has exactly one caller and keys on `document_root()`; its Insert / Remove / AttributeChange arms key on `owner_doc(node)` | `consumer_dispatcher.rs:136`; `document_base.rs:276-286` (`:277` `document_root()`); `:350-365` (arms via `owner_doc`) |
| A-5 | The VM element setter resolves its receiver, then activates; the body-delegation setter brand-checks `<body>`, returns early without a Window, then activates on the Window | `native_event_handler_set` `event_handler_attrs.rs:344-356` (receiver `:350`, activate `:354`); `native_body_weh_set` `:459-479` (brand check `:465`, early return `:468-470`, activate `:472-477`) |
| A-6 | The VM holds the policy the consumer is built with | `VmInner::spec_level_policy` `crates/script/elidex-js/src/vm/mod.rs:2715`; passed at `vm_api.rs:296` |
| A-7 | The uncapped shadow-inclusive walker is private, visits each entity once (visited set), pushes the shadow-root entity and the template-contents fragment, and has two callers that snapshot before acting | `teardown.rs:243-277`; callers `despawn_subtree` `:197`, `adopt_subtree` `:287` (`/usr/bin/grep -rn --include='*.rs' for_each_uncapped_shadow_inclusive crates` → 3 hits: def + these 2) |
| A-8 | The shell runs the elidex-js VM, so every shell pin reaches `Vm::bind` | `crates/shell/elidex-shell/Cargo.toml:31` (`elidex-js` with `engine`); `pipeline.rs:14` (`ElidexJsEngine`) |
| A-9 | Both URL-loaded iframe entries call the same builder | `load.rs:221`, `load.rs:327` → `crate::build_pipeline_from_loaded` (`/usr/bin/grep -rn --include='*.rs' 'build_pipeline_from_loaded(' crates/shell` → `load.rs:221`, `:327`, plus `app/navigation.rs:323`, `content/mod.rs:673`, `content/navigation.rs:276`, `pipeline.rs:960` and the def `:839`) |
| A-10 | The 18 `HandlerScope::Window` rows equal the WindowEventHandlers IDL members | `.claude/tools/webref idl html WindowEventHandlers` member names, sorted, equal `awk '/HandlerScope::Window\)/' event_handler_consumer.rs` names, sorted (18 = 18, verified 2026-10-26) |
| A-11 | The Window-reflecting body element event handler set is `onblur onerror onfocus onload onresize onscroll` | `.claude/tools/webref dfn html 'Window-reflecting body element event handler set'` → §8.1.8.2 `#window-reflecting-body-element-event-handler-set`; the table's first column read with `webref body` (verified 2026-10-26) |
| A-12 | `apply_set_attribute` goes through the `set_attribute` chokepoint (so the G6 claim is stale) | `crates/script/elidex-script-session/src/mutation/mod.rs:897-908` (`:903`) |

### §0.5 Items the umbrella left to this plan (decided here)

| item | source | decision |
|---|---|---|
| A2-MIN-1 / A3-MIN-2 (D-1's sparse sentence vs Window-delegating-only elements) | review record r5, umbrella §13.3 | The element marker is inserted iff E carries **at least one handler content attribute of an ordinary pair** (policy-excluded or not, I1×I6). An element whose only handler content attributes are Window-delegating pairs is never marked: D-1 "neither gates nor records a Window-delegating pair". Unobservable; pinned white-box by plan cell P1 (§4.5) |
| A2-MIN-2 (rev 4's three-line write shape is not ordinary-aware) | r5, §13.3; umbrella §4 | The write-side entry is the four-check shape of §1.4 (b): marked → return; written pair Window-delegating → return; E's deliverable **ordinary** set ≠ {written name} → return; else process with `ProcessingTiming::HandlerWrite` (NEW) |
| A2-MIN-3 (X-W's name set has no SoT; complement falls to the once rule) | r5, §13.3 | The WindowEventHandlers half is **derived** from `EVENT_HANDLER_ATTRS` rows with `HandlerScope::Window` (a new WEH row needs no classifier edit; P2's literal list is updated by hand, and P2 is red until it is: intended, F12). The reflecting half is a named const `WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET` (NEW) citing the webref anchor (A-11). Plan cell P2 pins: X-W ⊇ HEAD's delegation predicate (`scope == Window && is_body_element`), the const equals the spec's six, every const member is a table row, and a fixed ratchet of literal name lists (18 WEH + 6 reflecting) that does not use the derivation (§4.5). The complement is the once rule, i.e. the side the record calls leaking; the only way to grow it silently is a spec change to the reflecting set, which webref drift tooling surfaces (§1.4 (e)) |
| A3-MIN-1 (X-W's "HEAD outcome at every step" vs the bind pass) | r5, §13.3 | Umbrella wording only (its X-W already says "except the bind pass"). This plan states the realisation exactly: at T-a1 a Window-delegating pair gets the spec's or E-G1's outcome (HEAD processes nothing there); at every other step HEAD's (§1.4 (e)). No umbrella edit |
| A3-MIN-3 (§8.2 slot text "deliverable" → "deliverable ordinary") | r5, §13.3 | Slot text is the PM's ledger. Handed to the PM (§9) |
| rev-8 MIN: C20 (ii) needs a fresh VM after (i) | r5 rev-8 attestation | C20 (i) and C20 (ii) are **separate `#[test]` functions**, each with its own VM, session and DOM (§4.2). (i) leaves `fn` on `window.onhashchange`, which (ii) must not inherit |
| rev-8 MIN (uncertain): no cell for a WindowEventHandlers name on `<frameset>` | r5 rev-8 attestation | Plan cell **P3** (red; §4.5): a clone-sourced `<frameset onclick onhashchange>` where only the right classification passes D-8 (a). It fails an impl whose X-W misses `(frameset, WEH name)` |
| rev8:533 (§3 row 2, "step 3 is T3, the reflecting set facet 4") | r5 rev-8 attestation | Umbrella wording. Noted for the PM (§9); not edited |
| umbrella §6.4 "takes C1–C17" (stale range) and placement | umbrella §6.4 decided obligation | Replaced by §5: the cells go to **new** files; `tests_event_handler_attrs.rs` is not touched. The C set is C1–C9, C11–C20 (there is no C10 in umbrella §6.1) |
| Q7 / E8 replacement shape | umbrella §11 Q7 | E8 is written as decided. Fallback 1 (umbrella's): `x` from a DOMParser document. Fallback 2 (decided here): if `attachShadow` cannot run in an initial script, take `sr` from a declarative shadow root in the initial markup (`<div id=h><template shadowrootmode=open></template></div>`, as E7). Selection is by E8's pre-fix run (it is a guard: it must pass on the base) (§4.4). **Terminal, as for E7**: if the decided shape and both fallbacks fail at the scaffold run, implementation stops and this plan returns to plan-review. E8 is not reshaped or dropped during implementation |
| marker home (file level) | umbrella §4 Home/kind; PM decision 35 | The home of both markers and the processing function moves from `event_handler_consumer.rs` to the new module `event_handler_processing/` (§1.1, §1.3), under PM decision 35. Same crate (`elidex-script-session`), same re-exports from `lib.rs`, same component kinds (unit, `Send + Sync`, on the origin element / on the Document). Reason: the 1000-line touch-time rule. Kept in the consumer file, the mechanism would bring it to ≈ 900 (590 + ≈ 400 for the mechanism − ≈ 74 for the six helpers it absorbs, `:313-341` and `:466-498`, and `handle_insert`'s walk), so the next touch would force a split. On umbrella §6.4's "standalone prerequisite split": this PR creates the new module from the start and moves the helpers out of the consumer file, so no existing file crosses 1000 (§5: the consumer file 590 → ≈ 500, every other touched file below 1000) and no prerequisite split of an existing file arises. The umbrella's §4 Home/kind line is corrected in this PR (§1.3, §1.7): before "Both are homed in `crates/script/elidex-script-session/src/event_handler_consumer.rs`, beside the consumer."; after "Both are homed in `crates/script/elidex-script-session/src/event_handler_processing/`, a module of `elidex-script-session` beside the consumer." |
| Q5 / E5b | umbrella §6.2, §11 Q5 | **E5b is not written.** No hermetic same-origin URL loader exists (the in-process "URL-load" security test reaches only the `blank_entry` fallback, `content_iframe_security_tests.rs:278`). `load.rs:221` and `:327` call the same builder (A-9), and the processing point is inside `Vm::bind`, beneath every builder. E5 pins `:327` |
| O4 (writer completeness) | umbrella §11 O4: closes at this plan-review | Closed: §6 re-runs the §5.0 census (unchanged) and enumerates every production writer of handler state in `EventListeners`. No VM path writes handler state outside the IDL activation path and the lazy-compile reconcile |
| two init keys in one hook | umbrella §4 input, R3-A5-MIN-5 | **One key.** `initialize_consumers(dom, document)` passes the bound document to both delegates. `BaseUrlMaintainer`'s `initialize_from_tree` takes it in place of `document_root()`, which aligns its init with its own arms (A-4). In production the two keys are the same entity: (1) every production `Vm::bind` passes `ctx.document` (`engine.rs:397`, reached from `teardown_document` `:419` and `with_bound` `:440`; §0.2 reach set); (2) every production `ScriptContext` that reaches those is built from a pipeline's `document` (`crates/shell/elidex-shell/src/lib.rs:294`–`:526`, `self.document`; `pipeline.rs:128`, `:159`, `:350`, the `document` parameter); (3) that `document` is the parse result's (`pipeline.rs:572`, `:661`, `:752`; `build_pipeline_from_loaded` `pipeline.rs:850-852` takes `LoadedDocument.document`, set from the parse result at `crates/shell/elidex-navigation/src/loader.rs:214`); (4) both document parsers create it with `create_document_root()` on a fresh `EcsDom` (strict `crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs:101-102`, tolerant `crates/dom/elidex-html-parser/src/convert.rs:17-18`), which sets `document_root` (`crates/core/elidex-ecs/src/dom/mod.rs:397-399`). No production caller binds another document. The keys differ only for a test that binds a non-root document (scout Q-S3, §8) |
| U8 / U3 kinds vs the pre-fix base | umbrella §6.1 | See §0.6 C-1 |

### §0.6 Contract clauses found problematic (reported, not changed)

- **C-1 (minor, cell kinds).** Umbrella §6 defines *guard* as "passes at both". U8 (iii) ("a shadow host with
  `<button onclick>` in its shadow root … delivered") cannot pass on the pre-fix base, which has no bind pass. U3 (i)
  ("the div is marked after the first append") and U5 (iii) assert the new marker, which the base never inserts. This
  plan runs each such oracle in two parts: the HEAD-equality part is a guard (passes at both); the new-state part
  (marker present, U8 (iii) delivered) is red-first like any red cell. No cell or oracle changes.
- **C-2 (none blocking).** Every other decided clause was traced to code and found implementable: K-0's timing table,
  X-W at each timing (§1.4 (e)), D-8 (a) with post-write `Attributes` (§1.4 (b) note), D-5 with the bound document
  (A-1, A-2), D-11 over the generalised walker (§1.6), K-5 (the DOMParser document is a separate Document entity, never a
  shadow-including descendant of the bound document).

## §1. Implementation

### §1.1 Crate ownership (Layering mandate)

| algorithm / fact | crate | why |
|---|---|---|
| attribute change steps realisation, once rule, X-W classifier, D-8 restriction, bind pass | `elidex-script-session` (new module `event_handler_processing` (NEW)) | engine-independent; `EventListeners` lives here (write-locality, `event_handler_consumer.rs:29-33`) |
| uncapped shadow-including walk with a template-contents policy | `elidex-ecs` (`dom/tree/walkers.rs`) | pure tree structure; one walker for teardown, adopt and the bind pass |
| consumer composition, the init call with the bound document | `elidex-js` `vm/consumer_dispatcher.rs`, `vm/vm_api.rs` | binding-layer wiring only |
| VM setters | `elidex-js` `vm/host/event_handler_attrs.rs` | marshalling only: resolve receiver, call `elidex_script_session::process_before_handler_write` (NEW), then activate. No marker access, no once logic (I1×I9) |

### §1.2 ECS-native shape

| OO idiom | ECS realisation |
|---|---|
| `element.attributesProcessed` flag | `HandlerAttrsProcessed` (NEW): unit `Send + Sync` component on the origin element, sparse |
| `document.handlersInitialized` flag | `PreBindHandlerAttrsProcessed` (NEW): unit component on the bound Document entity |
| "on attach, initialise the document" | a system over the bound document's shadow-including tree, run at the dispatcher's install point (`initialize_consumers`), gated by the document marker |
| "writers notify the element" | the AttributeChange arm and both VM setters call one engine-independent function |

Neither marker is a per-VM identity handle, both are per-entity, so CLAUDE.md's side-store→component rule makes them
components; neither exception (a) nor (b) applies. No `HashMap<Entity, _>`, no registry, no consumer-struct state (the
consumer is rebuilt at every bind, umbrella H13). Despawn removes both markers; `adopt_subtree` leaves them; clone copies
neither (clone-policy row, §1.7).

### §1.3 Change list by file

Line counts at `b9122157` (`wc -l`). Projection = estimated after-change size. 1000-line analysis is §5.

| file | lines | change |
|---|---|---|
| `crates/script/elidex-script-session/src/event_handler_processing/mod.rs` (NEW) | — | markers, classifier, `ProcessingTiming` (NEW), D-2 function, write-side entry, bind pass body; receives `event_handler_attr_lookup`, `resolve_handler_target`, `is_body_element`, `set_inline_handler`, `clear_inline_handler`, `ensure_event_listeners` moved from the consumer file (they are the attribute-change-steps realisation) |
| `crates/script/elidex-script-session/src/event_handler_processing/tests_insertion_and_write.rs` (NEW) | — | U1–U6, U9, P1, P2 |
| `crates/script/elidex-script-session/src/event_handler_processing/tests_bind.rs` (NEW) | — | U7, U8, U10, U11, U12 |
| `crates/script/elidex-script-session/src/event_handler_consumer.rs` | 590 | arms delegate (§1.4 (d)); helpers move out; `initialize_from_tree` (NEW) method; module doc `:1-33` rewritten; stale sentence in `web_storage_spec_level`'s doc (`:235-242`) fixed (§1.7) |
| `docs/plans/2026-10-event-handler-attrs-at-creation.md` | — | umbrella §4 Home/kind — one-line correction (PM decision 35; ratified surface, shown to this plan-review): `:563-564` file name `event_handler_consumer.rs` → `event_handler_processing/` (exact text, §0.5 "marker home") |
| `crates/script/elidex-script-session/src/lib.rs` | 62 | `mod event_handler_processing;` + re-exports of the markers and `process_before_handler_write` |
| `crates/script/elidex-js/src/vm/consumer_dispatcher.rs` | 153 | `initialize_consumers(&mut self, dom, document)`; doc `:113-134`; field doc `:58-67` |
| `crates/script/elidex-js/src/vm/vm_api.rs` | 762 | `:313` passes `document`; comment `:298-312` rewritten |
| `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` | 726 | two calls (D-6) in `native_event_handler_set` and `native_body_weh_set` |
| `crates/script/elidex-js/src/vm/test_helpers.rs` | 213 | page harness with a connected `el` (§4.1) |
| `crates/script/elidex-js/src/vm/tests/mod.rs` | 197 | two `mod` lines |
| `crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs_once.rs` (NEW) | — | C1–C7, C9, C11–C16 |
| `crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs_window_delegating.rs` (NEW) | — | C8, C17, C18, C19, C19b, C19c, C20 (i), C20 (ii), P3 |
| `crates/core/elidex-ecs/src/dom/tree/walkers.rs` | 321 | the walker moves here, `pub`, with `TemplateContentsWalk` (NEW) and shadow-including tree order |
| `crates/core/elidex-ecs/src/dom/tree/teardown.rs` | 367 | private walker removed; `:197`, `:287` call the public one with `Include` |
| `crates/core/elidex-ecs/src/dom/tests/traversal.rs` | 267 | walker tests W1–W4 (§4.3) |
| `crates/core/elidex-ecs/src/dom/tree_clone.rs` | 425 | clone-policy row `:30` gains both markers (doc only) |
| `crates/core/elidex-ecs/src/dom/attribute_reconcile.rs` | 66 | G6: stale claims at `:21-23` and `:30-32` |
| `crates/dom/elidex-dom-api/src/element/document_base.rs` | 555 | `initialize_from_tree(&mut self, dom, document)`; doc `:258-275` |
| `crates/shell/elidex-shell/src/content_eh_attrs_bind_tests.rs` (NEW) | — | E1–E8 (minus E5b) |
| `crates/shell/elidex-shell/src/content_test_support.rs` | 288 | lifted helpers + `probe_body_attr` (NEW), `build_main_document` (NEW) |
| `crates/shell/elidex-shell/src/content_iframe_security_tests.rs` | 667 | three private helpers move to test support (`:83`, `:399`, `:423`) |
| `crates/shell/elidex-shell/src/content/mod.rs` | 858 | one `#[path]` test-module line beside `:788-821` |

### §1.4 The algorithm

#### (a) D-2: one processing function

```rust
/// elidex-script-session/src/event_handler_processing/mod.rs
pub(crate) enum ProcessingTiming { Bind, Insertion, HandlerWrite }   // T-a1, T-a2, T-a3

pub(crate) fn process_handler_content_attributes(
    dom: &mut EcsDom, element: Entity, policy: SpecLevelPolicy, timing: ProcessingTiming,
) {
    // Snapshot E's handler content attributes in `Attributes` order: (name, event_type, scope, value).
    // No `Attributes` → return (Document, Window, text nodes, shadow-root entities).
    let marked = has::<HandlerAttrsProcessed>(element);
    let mut carries_ordinary = false;
    let mut writes = Vec::new();
    for (name, event_type, scope, value) in snapshot {
        let window_delegating = is_window_delegating_pair(dom, element, name);   // X-W, tag + name only
        if !window_delegating { carries_ordinary = true; }                     // counted before the policy filter (I1×I6)
        if !policy.installs(event_handler_attr_spec_level(name)) { continue; } // I6, same filter as HEAD :405 / :447
        let deliver = match (window_delegating, timing) {
            (true, Bind | Insertion) => true,     // X-W: ungated, HEAD's path; T-a1 is the spec's or E-G1's outcome
            (true, HandlerWrite)     => false,    // X-W: not processed at T-a3 (C20)
            (false, _)               => !marked,  // K-1 once
        };
        if deliver { writes.push((scope, event_type, value)); }
    }
    for (scope, event_type, value) in writes {
        let target = resolve_handler_target(element, scope, dom);  // HEAD's target, unchanged (no step 3: T3)
        set_inline_handler(target, event_type, value, dom);         // HTML §8.1.8.1 attribute change steps 5.5–5.6
    }
    if carries_ordinary && !marked { insert HandlerAttrsProcessed on element }   // D-1, §0.5 A2-MIN-1
}
```

The once check lives here and nowhere else (I1×I9). The write order inside one element is `Attributes` order, as at
HEAD's `handle_insert` (`event_handler_consumer.rs:441-462`).

#### (b) D-4 / D-8: the write-side entry

```rust
/// Called before a handler write: by the AttributeChange arm after its lookup and policy check,
/// and by both VM element setters after receiver resolution (D-6).
pub fn process_before_handler_write(dom: &mut EcsDom, element: Entity, name: &str, policy: SpecLevelPolicy) {
    if has::<HandlerAttrsProcessed>(element) { return; }               // already processed
    if is_window_delegating_pair(dom, element, name) { return; }        // D-8 (b) = X-W
    // D-8 (a): `name` is E's only deliverable ordinary handler content attribute
    // (named in EVENT_HANDLER_ATTRS, installed by `policy`, ordinary for E).
    if deliverable_ordinary_names(dom, element, policy) != [name] { return; }
    process_handler_content_attributes(dom, element, policy, ProcessingTiming::HandlerWrite);
}
```

Its doc names `#11-eh-attrs-creation-time-call-sites` as the record of the restriction (umbrella §10.1).

*Post-write `Attributes`.* The AttributeChange arm runs after the `Attributes` write (umbrella H20), so the entry
reads E's post-write attributes. Every case is HEAD-equal or the umbrella's spec outcome:
- `setAttribute(name, v)` on an unmarked E whose other attributes are not deliverable ordinary: passes; processing raws
  `v`, which the arm then raws again. The state after the write equals HEAD's; E is marked (T-a3 "passes").
- `removeAttribute(name)`: `name` is gone, so the set is `[]` or holds another name, and the entry returns: HEAD's path.
  The later insertion processes E's remaining attributes as HEAD does.
- A VM setter does not touch `Attributes`. Passing case: processing raws the attribute's value, then the setter's
  activation overwrites it (`clear_uncompiled` + callable, or removal for `null`). The state right after the write is
  HEAD's (umbrella K-0 T-a3).

The predicate is `== [name]`, the umbrella's D-8 (a) wording. Rev 4's shape ("E carries no deliverable attribute
other than `name`") would also pass the empty set; the difference is only whether an element with no deliverable
ordinary attribute gets marked, which is unobservable. The literal wording is kept.

#### (c) D-5 / D-11: the bind pass

```rust
impl EventHandlerAttributeConsumer {
    /// Called by `ConsumerDispatcher::initialize_consumers` before the dispatcher is installed.
    pub fn initialize_from_tree(&mut self, dom: &mut EcsDom, document: Entity) {   // (NEW)
        if !dom.contains(document) || has::<PreBindHandlerAttrsProcessed>(document) { return; }  // later binds: nothing
        let mut nodes = Vec::new();
        dom.for_each_uncapped_shadow_inclusive(document, TemplateContentsWalk::Exclude, &mut |e| nodes.push(e));
        for node in nodes { process_handler_content_attributes(dom, node, self.policy, ProcessingTiming::Bind); }
        insert PreBindHandlerAttrsProcessed on document;
    }
}
```

The document marker is inserted unconditionally (a document with no handler attribute is still "processed").
Re-entry: none. The pass runs before `set_mutation_dispatcher` (`vm_api.rs:313-314`) and writes only components, so
no `MutationEvent` fires (I5×I9).

#### (d) Arms of the consumer's `handle`

- AttributeChange (`event_handler_consumer.rs:389-421`): unchanged up to and including the policy check (`:405`).
  Then `process_before_handler_write(dom, node, name, policy)`. Then HEAD's write (`:408-420`) unchanged.
- Insert (`:427-464`): the walk set stays `node` + `traverse_descendants(node)` (D-10), then
  `process_handler_content_attributes(dom, e, policy, ProcessingTiming::Insertion)` per entity, in walk order. HEAD
  collected every pending write before writing. Per-entity read-then-write is equivalent: a write touches only
  `EventListeners` and E's own marker, and no later read in the arm reads either.

#### (e) X-W classifier

```rust
/// WHATWG HTML §8.1.8.2 Event handlers on elements, Document objects, and Window objects
/// (#window-reflecting-body-element-event-handler-set).
pub const WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET: [&str; 6] =
    ["onblur", "onerror", "onfocus", "onload", "onresize", "onscroll"];          // (NEW)

/// Steps 1–2 of determining the target of an event handler (WHATWG HTML §8.1.8.1 Event handlers):
/// true iff those steps do not return the element. Reads the tag and the name only (X-W).
pub(crate) fn is_window_delegating_pair(dom: &EcsDom, element: Entity, name: &str) -> bool {   // (NEW)
    is_body_or_frameset(dom, element)   // `TagType` compared ASCII-case-insensitively, as HEAD's `is_body_element`
        && (matches!(event_handler_attr_lookup(name), Some((_, HandlerScope::Window)))
            || WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET.contains(&name))
}
```

`resolve_handler_target` and `is_body_element` move with their code unchanged; `is_body_or_frameset` (NEW) sits
beside them. `resolve_handler_target`'s doc (`event_handler_consumer.rs:323-327`) cites "WHATWG HTML §8.1.8.2" for the
`<body>` delegation; on the move its citation becomes WHATWG HTML §8.1.8.1 Event handlers, *determining the target of
an event handler* (`#determining-the-target-of-an-event-handler`) steps 1–2, the algorithm it realises, with step 3
named as unmodelled (T3). The WindowEventHandlers set itself stays cited at §8.1.8.2. The
classifier decides *whether* a timing processes a pair; the *target* stays HEAD's (T3 owns step 3, `<frameset>`
delegation and the reflecting set's target).

Realisation of X-W per timing (A3-MIN-1): T-a1 delivers Window-delegating pairs; HEAD processes nothing at that point,
so the outcome there is the spec's (a `<body>` WEH row on the active bound document's Window) or E-G1 (i) (the
reflecting six and `<frameset>` pairs land on the element). T-a2 re-raws them at every insertion of HEAD's walk set,
marked or not: HEAD's outcome. T-a3 never processes them: the AttributeChange arm and the VM setters then write them
exactly as HEAD does.

### §1.5 VM surface (D-6)

- `native_event_handler_set` (`event_handler_attrs.rs:344-356`): after `require_handler_receiver` (`:350`), before
  `activate_event_handler` (`:354`): `elidex_script_session::process_before_handler_write(dom, entity,
  &format!("on{event_type}"), ctx.vm.spec_level_policy)`. A Document, Window or Worker receiver has no `Attributes`,
  so the entry returns.
- `native_body_weh_set` (`:459-479`): bind the `<body>` entity that `require_body_receiver` returns (today discarded
  at `:465`). After the no-Window early return (`:468-470`), call the same entry with that entity. Every name this
  setter serves is a `HandlerScope::Window` row, hence a Window-delegating pair on `<body>`, so the entry returns
  (X-W). The call site is kept for slice b (S-6), and C17 / C18 fail any entry that processes there.
- The DOM borrow is the one the file already uses for `EventListeners` access. No other VM code names either marker
  outside tests (§7 check).

### §1.6 The walker (D-11)

`for_each_uncapped_shadow_inclusive` moves from `teardown.rs:243-277` to `walkers.rs` as a `pub` method with a
`TemplateContentsWalk` (NEW) parameter (`Include` for teardown and adopt, `Exclude` for the bind pass). Changes:
- **Visit order = shadow-including tree order** (WHATWG DOM §4.8 Interface ShadowRoot, *shadow-including tree order*:
  "shadow-including preorder, depth-first traversal"): push the light children in reverse, then the shadow-root entity,
  so the host's shadow tree is visited right after the host and before its light children. In `Include` mode the
  template-contents fragment is pushed first (visited after the template's subtree; it is outside tree order).
  Order is observable for slice a only when two origins deliver the same event type to one target, i.e. two `<body>`
  elements with the same WEH row in one bound tree; the pre-bind parse yields at most one `<body>`. Tree order is
  chosen as the spec-defined order, not for a measured outcome.
- Termination on a malformed tree stays the `visited` set (K7); breadth uses `child_list_uncapped`
  (`crates/core/elidex-ecs/src/dom/tree/navigation.rs:287`).
- `despawn_subtree` and `adopt_subtree` snapshot the list before acting. Despawn destroys deepest-first by reversing a
  list in which every node precedes its descendants; that holds for the old order and for tree order alike. Their
  tests (`dom/tests/destroy.rs:381`, `:419`) stay green.

The shadow-root entity is visited; it has no `Attributes`, so processing returns. The walker name stays as is, so no
caller outside `elidex-ecs` breaks.

**Out of scope: `collect_template_inclusive_descendants` (`elidex-form`, `init` module)** (`crates/dom/elidex-form/src/init.rs:111`,
`pub(crate)`, capped through `for_each_shadow_inclusive_descendant`; callers `inert_document.rs:168`, form
`clone.rs:49`). It is a second template-inclusive walker for the same concept as `TemplateContentsWalk::Include`.
Unifying it onto the new walker is not slice a's: umbrella §9 N-5 routes that reconciliation to slice b's input, its
callers are slice b's creation-time call sites (inert-document parse, clone), and slice a changes neither. It is handed
to slice b's input by name (§9). I10 (§2) covers only the three callers it names.

### §1.7 Docs, comments, clone policy (umbrella §4's rewrite-site list)

- Umbrella `docs/plans/2026-10-event-handler-attrs-at-creation.md` §4 Home/kind (`:563-564`) — one-line correction
  (PM decision 35; ratified surface, shown to this plan-review). Before: "Both are homed in
  `crates/script/elidex-script-session/src/event_handler_consumer.rs`, beside the consumer." After: "Both are homed in
  `crates/script/elidex-script-session/src/event_handler_processing/`, a module of `elidex-script-session` beside the
  consumer." It lands with the implementation; no other umbrella text changes.
- `event_handler_consumer.rs:1-33` module doc: three timings (bind, Insert, first handler write), the interim Insert arm
  naming `#11-eh-attrs-creation-time-call-sites`, X-W. `handle_insert`'s doc cites WHATWG DOM §4.2.3 Mutation
  algorithms (no event-handler insertion steps) and names the slot.
- New items cite as the table below. "none" means no spec step defines the item: it is an elidex mechanism, and the
  doc says so and names the slot that records it.

  | NEW item | doc citation |
  |---|---|
  | `HandlerAttrsProcessed` | WHATWG DOM §4.9 Interface Element, *append an attribute* step 4 (each creation-time attribute runs the attribute change steps once); WHATWG HTML §8.1.8.1 Event handlers, *attribute change steps* |
  | `PreBindHandlerAttrsProcessed` | WHATWG HTML §13.2.6.1 Creating and inserting nodes, *create an element for a token* step 17.2 (the parser appended the attributes before any bind). The once-per-document bind point itself: none — elidex mechanism (D-5) |
  | `ProcessingTiming` | none — elidex mechanism (the K-0 timings T-a1 / T-a2 / T-a3), slot `#11-eh-attrs-creation-time-call-sites` |
  | `process_handler_content_attributes` | WHATWG HTML §8.1.8.1 Event handlers, *attribute change steps* 1–5.6, per attribute; WHATWG DOM §4.9 Interface Element, *append an attribute* step 4 |
  | `process_before_handler_write` | none — elidex mechanism (D-8), slot `#11-eh-attrs-creation-time-call-sites` |
  | `EventHandlerAttributeConsumer::initialize_from_tree` | WHATWG HTML §13.2.6.1 Creating and inserting nodes, *create an element for a token* step 17.2; WHATWG HTML §8.1.8.1 Event handlers, *attribute change steps*. Running them at the first bind instead of at creation: none — elidex mechanism (D-5) |
  | `WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET` | WHATWG HTML §8.1.8.2 Event handlers on elements, Document objects, and Window objects (`#window-reflecting-body-element-event-handler-set`) |
  | `is_window_delegating_pair`, `is_body_or_frameset` | WHATWG HTML §8.1.8.1 Event handlers, *determining the target of an event handler* steps 1–2 |
  | `TemplateContentsWalk` | WHATWG HTML §4.12.3 The template element (*template contents* are not descendants) |
  | the moved walker (`pub`, `walkers.rs`) | WHATWG DOM §4.8 Interface ShadowRoot (*shadow-including descendant*, *shadow-including tree order*) |
  | test helpers `setup_page`, `build_main_document`, `probe_body_attr` | none (test support) |
- `consumer_dispatcher.rs:58-67` field doc ("Dual-arm") and `:113-134` method doc ("Add a delegate call only when a
  future consumer derives state…") rewritten: two delegates, both keyed on the bound document.
- `vm_api.rs:298-312` comment: "Other consumers … are no-ops here" becomes the two-delegate statement.
- `document_base.rs:258-275`: keyed on the bound document, matching its arms.
- `event_handler_consumer.rs:235-242`, `web_storage_spec_level`'s doc: the clause "(it still runs the boa engine, not
  this VM)" is false (A-8). The paragraph is rewritten to the ledger row of `#11-storage-event-mode-aware-delivery`
  (PM re-homing, 2026-10-26): suppressing delivery for a Web-Storage-excluded session needs per-session mode plumbing
  the shell does not have yet; it is not reachable today, because no production session selects an excluded mode
  (`BrowserCompat` is the only one); so it is deferred to slot `#11-storage-event-mode-aware-delivery`. The slot name
  stays. The sentence about the `window.onstorage` handler-attribute install being gated stays.
- `tree_clone.rs:30` (deliberate non-copy): add `HandlerAttrsProcessed` and `PreBindHandlerAttrsProcessed`
  (elidex-script-session) with the reason: a clone is a new origin, unprocessed (I4); a cloned Document has had no first
  bind (`clone_subtree`'s Document branch, `tree_clone.rs:163-230`). WHATWG DOM §4.4 Interface Node, *clone a single
  node* step 2.5 appends the attributes at the clone's own creation.
- G6, `attribute_reconcile.rs`: `:21-23` says the deferred flush `apply_mutation` "writes `Attributes` without entering
  the chokepoints"; `:30-32` lists "the deferred flush" among the event-less callers. Both are false (A-12). The callers
  of `reconcile_attribute_derived_components` are `set_attribute` (`attribute.rs:227`), `remove_attribute`
  (`attribute.rs:308`) and the shell iframe lifecycle (`content/iframe/lifecycle.rs:365`, `:526`)
  (`/usr/bin/grep -rn --include='*.rs' reconcile_attribute_derived_components crates`). The umbrella names `:21-23`
  only; `:30-32` is the same claim and is fixed with it.
  Two nearby texts were checked and are **not** part of the fix: `crates/core/elidex-ecs/src/components.rs:731-733`
  (`IframeData::from_attributes`'s doc) and `crates/core/elidex-ecs/src/dom/tree_clone.rs:24` (the clone-policy
  "derived re-derive" row) both say the attribute→component direction is reconciled "at the `EcsDom::set_attribute` /
  mutation-flush (reconcile) seam". Neither says the flush bypasses the chokepoint or reconciles without an event. The
  flush reaches the reconcile through `set_attribute` (A-12), so "reconciled at the mutation flush" is true as written.
  G6 is the false bypass claim, and these two texts do not make it.

### §1.8 Commit sequence (red-first, D-9)

1. **Scaffold** (never lands alone; squash merge): every new public name with a HEAD-equivalent body (both markers
   declared and never inserted; `initialize_from_tree` and `process_before_handler_write` no-ops; the walker not yet
   moved), plus all cells, pins and plan cells. Run on this commit: every red cell and red pin fails, every guard and
   window pin passes, every pin's control is green. Record the output in the PR description. The no-op bodies are
   HEAD's behaviour (HEAD processes nothing at bind or at a write), so the run measures HEAD.
2. **Fix**: §1.4–§1.6.
3. **Docs**: §1.7.

## §2. Coupled invariants

Axes I1–I9 are the umbrella's (§1.1). **I10** is added by this plan: *one uncapped walker serves teardown, adopt and the
bind pass, in shadow-including tree order*, introduced by D-11's generalisation (§1.6). I10 is about those three
callers only; `collect_template_inclusive_descendants` is outside it (§1.6, slice b input).

Each line: pair — intersects? — slice-a mechanism (code) — cell. "b" = slice b owns it (no slice-a code).

- I1×I2 — yes — marker checked first in `process_before_handler_write`; a marked E processes nothing at a write — C1, C4, U5 (iii)
- I1×I3 — yes — marker on the origin; Window-delegating pairs bypass the once gate (`ProcessingTiming` match) — C8, C19, U2, P1
- I1×I4 — yes — clone copies no marker (`tree_clone.rs:30` row) — C9, U6
- I1×I5 — yes — every timing calls `process_handler_content_attributes`; the bind pass is gated by the document marker — E3, E8, U7, U9
- I1×I6 — yes — `carries_ordinary` counted before the policy filter, so an excluded-only element is marked — U3 (i)
- I1×I7 — yes — K-1 identity into a DOMParser document (marked element skipped at Insert) — C2, C4, C5
- I1×I8 — yes — b (S-1 with T3)
- I1×I9 — yes — the once check exists only inside the D-2 function; VM calls the entry and holds no marker logic — §7 grep check
- I1×I10 — yes — the walker visits each entity once (`visited`), so the bind pass processes each element at most once per pass — U11
- I2×I3 — yes — `native_body_weh_set` passes the `<body>` receiver; the entry returns on X-W — C17, C18
- I2×I4 — yes — a write on the source never marks the clone (marker per entity) — C9
- I2×I5 — yes — D-8 restriction in the entry; neither a later bind nor the light Insert walk reaches a shadow-root-direct element — C4, C5, C11, U5, U9, E8
- I2×I6 — yes — the AttributeChange arm calls the entry only after the policy check (`:405`); an excluded IDL setter is not installed — U3
- I2×I7 — yes — D-8 (a) blocks multi-attribute write-time processing of DOMParser nodes — C16
- I2×I8 — yes — no Window-delegating pair processed at a write (entry X-W check, `HandlerWrite` arm) or skipped at Insert — C17, C18, C19, C20, P3
- I2×I9 — yes — both VM setters call the one engine-independent entry — C11, C17
- I2×I10 — no
- I3×I4 — yes — a clone of a `<body>` is a new unmarked origin; its Window-delegating pairs follow X-W — C9, P3
- I3×I5 — yes — the bind pass delivers `<body>` WEH rows to the Window, which exists before init (A-1) — E4
- I3×I6 — yes — the policy filter applies to Window-delegating deliveries too — U3 (ii)
- I3×I7 — yes — b (S-1)
- I3×I8 — yes — target stays `resolve_handler_target` (HEAD); the bind pass walks only the active bound document — E4, C17
- I3×I9 — yes — origin and classification in `elidex-script-session`; the VM passes the receiver — C19b, C19c
- I3×I10 — yes — tree order decides which origin's same-type Window-delegating write lands last on the shared Window (reachable only with two `<body>` in one bound tree) — W2
- I4×I5 — yes — fragment-parsed nodes and clones are processed at first insertion (unmarked) — C5 (iii), C6, C9
- I4×I6 — no
- I4×I7 — yes — b (S-1, S-2)
- I4×I8 — yes — b (S-1 with T3)
- I4×I9 — yes — the non-copy is a doc row in `elidex-ecs`'s explicit copy-set table; no code — U6
- I4×I10 — no
- I5×I6 — yes — the same `policy.installs` filter at every timing (inside the D-2 function) — U3
- I5×I7 — yes — the bind pass walks only the bound document with `TemplateContentsWalk::Exclude` (NEW); Insert keeps W-K5 — U8, C14, C15
- I5×I8 — yes — b (S-1)
- I5×I9 — yes — the bind pass runs before the dispatcher install, so no `MutationEvent` re-enters (`vm_api.rs:313-314`) — U7
- I5×I10 — yes — coverage is the walker's: uncapped depth (explicit stack) and width (`child_list_uncapped`) — U10, U11, W1
- I6×I7 — no
- I6×I8 — no
- I6×I9 — yes — the VM passes `VmInner::spec_level_policy`, the value the consumer is built with (A-6) — U3, C-cells under `BrowserCompat`
- I6×I10 — no
- I7×I8 — yes — b
- I7×I9 — yes — b
- I7×I10 — yes — `Exclude` is K-5's bind half; teardown and adopt keep `Include` — U8 (ii), W3, `dom/tests/destroy.rs` kept green
- I8×I9 — no
- I8×I10 — no
- I9×I10 — yes — the walker is structure in `elidex-ecs`; the bind algorithm is in `elidex-script-session`; the VM only calls `initialize_consumers` — W1–W4, U7

Count: 45 pairs (36 umbrella + 9 with I10). Intersect: 37 (umbrella's 32 + I1, I3, I5, I7, I9 × I10). Do not
intersect: 8 (umbrella's 4 + I2, I4, I6, I8 × I10). Of the 37, b-only (no slice-a code): 7 (I1×I8, I3×I7, I4×I7,
I4×I8, I5×I8, I7×I8, I7×I9). The umbrella assigns two more to b, I2×I7 and I3×I8; their slice-a side is a guard
(D-8 (a); HEAD's target), so they carry slice-a mechanisms and cells here. Re-count:
`awk '/^## §2\./{f=1} /^## §3\./{f=0} f && /^- I[0-9]+×I[0-9]+ — /' docs/plans/2026-10-eh-attrs-slice-a.md | awk -F' — ' '{print $2}' | sort | uniq -c`
(37 yes, 8 no); b-only: the same filter with `/ — b( \(|$)/`.

## §3. Spec coverage map

Skeleton generated with `.claude/tools/webref coverage-map html 8.1.8.1 html 8.1.8.2 dom 4.9 html 13.2.6.1 dom 4.8 html 4.12.3 dom 4.4 dom 4.2.3 dom 4.5`
(verdict "ok → single PR scope", spec=2, step=9, 2026-10-26).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.8.1 Event handlers | *attribute change steps* 1–5.6 | 1 non-handler name → return; 2 determine the target; 3 null target → return; 4 null value → deactivate; 5.1 CSP (not modelled, E-G1 (iii)); 5.5 raw uncompiled; 5.6 activate | `process_handler_content_attributes` (NEW) for creation-time attributes; AttributeChange arm `event_handler_consumer.rs:389-421` for writes (entry first) | ✓ (5.1 listed, not modelled) | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *determine the target of an event handler* 1–4 | 1 not body/frameset → self; 2 not WEH and not reflecting → self; 3 inactive node document → null; 4 relevant global | steps 1–2: `is_window_delegating_pair` (NEW), the X-W test; target: `resolve_handler_target` (moved, unchanged: body + WEH only, no step 3; T3) | ✗ | no |
| WHATWG HTML §8.1.8.1 Event handlers | event handler IDL attribute setter 1–4 | target null → return; value null → deactivate; else set + activate | `native_event_handler_set` `event_handler_attrs.rs:344`, `native_body_weh_set` `:459` → `process_before_handler_write` (NEW) first | ✓ | yes |
| WHATWG HTML §8.1.8.2 Event handlers on elements, Document objects, and Window objects | WindowEventHandlers members; Window-reflecting body element event handler set | 18 WEH rows (A-10); 6 reflecting names (A-11) | `EVENT_HANDLER_ATTRS` (`HandlerScope::Window` rows) + `WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET` (NEW) | ✓ | no |
| WHATWG DOM §4.9 Interface Element | *append an attribute* step 4; *handle attribute changes* | every attribute present at creation runs the attribute change steps once; later changes run them per change | D-2 function per creation-time attribute; AttributeChange arm per change | ✓ | yes |
| WHATWG HTML §13.2.6.1 Creating and inserting nodes | *create an element for a token* step 17.2 | parser path appends every token attribute at creation | pre-bind parse: bind pass (`initialize_from_tree` (NEW)); post-bind parse (fragment, DOMParser): first insertion | ✓ | yes |
| WHATWG HTML §13.2.6.4.4 The "in head" insertion mode | start tag "template", step 13 "Keep", sub-step 1 (declarative shadow root; `body html parsing-main-inhead`) | the shadow root is attached while parsing; its children are parsed into it | strict `tree_builder/shadow.rs:137`, `:162` (umbrella §3 row); slice a: a pre-bind declarative shadow root is reached by the bind pass (E7, Q-S1); a fragment-parsed one is not processed at host insertion (D-10, HEAD-equal); slice b: S-1 | ✗ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *shadow-including descendant*; *shadow-including tree order* | shadow trees are included; host's shadow tree right after the host | `for_each_uncapped_shadow_inclusive` (moved to `walkers.rs`, `TemplateContentsWalk::Exclude`) | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | *template contents* | contents are not descendants | bind walk excludes them; Insert keeps W-K5 | ✗ | yes |
| WHATWG DOM §4.4 Interface Node | *clone a single node* step 2.5 | clone's attributes appended at its own creation | clone-policy row `tree_clone.rs:30` (non-copy of both markers) | ✓ | yes |
| WHATWG DOM §4.2.3 Mutation algorithms | *insert*; *insertion steps* | HTML defines no event-handler insertion steps | Insert arm `event_handler_consumer.rs:427`, once over HEAD's walk set (interim, slot named) | ✓ | yes |
| WHATWG DOM §4.5 Interface Document | *adopt* | node document changes; no event-handler step | `adopt_subtree` `teardown.rs:285` (walker call only; marker survives) | ✓ | no |

### §3.1 User-input touch audit

Author-controlled values in the *yes* rows: markup attribute values (handler sources) and script arguments to
`setAttribute` / IDL setters. The mechanism stores no user string of its own: both markers are unit components, and the
sources it writes are the `Attributes` values HEAD already writes. Names are matched against `EVENT_HANDLER_ATTRS` and
the reflecting const, both static. No new parse of user input. Newly reachable by the compiler: initial-HTML handler
sources (G1), compiled through HEAD's compile gate and global-only scope (E-G1 (ii)), with no CSP inline check (E-G1
(iii), as for every inline handler at HEAD). The walk is uncapped, so its cost is O(nodes) with a `HashSet` of visited
entities: a hostile document of n nodes costs one O(n) pass at its first bind (umbrella K4).

## §4. Test plan

### §4.1 Harness shapes

- **Unit (session)**: `EcsDom` with `EventHandlerAttributeConsumer` wrapped in a test-local `MutationDispatcher` and
  installed with `set_mutation_dispatcher`; real `append_child` / `set_attribute` calls; Window via
  `create_window_root`. **Pre-bind shape** = the tree is built with **no** dispatcher installed, then
  `initialize_from_tree` is called on a fresh consumer (and the dispatcher installed afterwards when the cell continues).
  Every second `initialize_from_tree` uses a **fresh consumer** (umbrella H13).
- **ECS walker**: `EcsDom` only, as `dom/tests/destroy.rs:381` / `:419` build deep and wide trees.
- **VM**: a page harness `setup_page` (NEW) in `test_helpers.rs`: Document → `html` → `body` → `div` (global `el`),
  built pre-bind with no handler attributes, then `bind_vm`. `el` is connected (umbrella §6.1 requires it; the existing
  `setup_with_element`, `test_helpers.rs:80`, leaves `el` detached). RAII unbind guard as `tests_event_handler_attrs.rs`
  does. One VM per `#[test]`. `DOMParser` is available as in `tests_dom_parser.rs`. Events by `dispatchEvent(new
  Event(…))`. Marker reads (C9) are white-box reads of `HandlerAttrsProcessed` via `entity_of`.
- **Shell**: `build_main_document(html)` (NEW) synthesises a `LoadedDocument` (the lifted `synth_cross_origin_loaded`,
  `content_iframe_security_tests.rs:399`, generalised to `synth_loaded(html, url)`) and calls
  `crate::build_pipeline_from_loaded` (`pipeline.rs:839`) with `test_network()` (`content_test_support.rs:27`). Reads
  of `data-*` on `<body>` use `probe_body_attr` (NEW), a Rust-side read with no new bracket. E3 and E8 use
  `PipelineResult::eval_script` (`crates/shell/elidex-shell/src/lib.rs:307`), E2 `PipelineResult::dispatch_event`
  (`:293`), because those pins need real brackets. OOP: `make_out_of_process_entry` + the lifted
  `run_oop_and_collect_posts` (`:423`). srcdoc: `build_test_content_state` (`content_test_support.rs:117`) + the lifted
  `in_process_entry` (`:83`).

### §4.2 VM cells (`tests_event_handler_attrs_once.rs`, `tests_event_handler_attrs_window_delegating.rs`)

Kind and oracle are the umbrella's (§6.1). `dp`, `dp2` = `new DOMParser().parseFromString('<body></body>', 'text/html')`
unless the cell gives markup; `doc2` in C2 is such a `dp`.

| cell | kind | file | setup beyond the page harness | scaffold run |
|---|---|---|---|---|
| C1 | red | once | `el.innerHTML` | fails (`t.onclick !== fn`) |
| C2 | red | once | `doc2` | fails |
| C3 | red | once | — | fails |
| C4 | red | once | `dp` | fails |
| C5 (i)–(iii) | red | once | `dp`, `dp2`; three `#[test]`s | fails |
| C6 | guard | once | `dp` | passes |
| C7 | guard | once | — | passes |
| C8 | window pin | window_delegating | — | passes |
| C9 | guard | once | white-box marker read | passes (marker never inserted at base) |
| C11 | red | once | — | fails |
| C12 | guard | once | — | passes |
| C13 | guard | once | strict string and a foster-parenting `<table>` string (tolerant tier) | passes |
| C14 | window pin | once | `<template>` in `el` | passes |
| C15 | window pin | once | `dp`, `dp2` | passes |
| C16 | guard | once | `dp` | passes |
| C17 | guard / window pin | window_delegating | `dp` with `<body onhashchange>` | passes |
| C18 | guard / window pin | window_delegating | `<template>` | passes |
| C19, C19b, C19c | guard / window pins | window_delegating | `dp` | pass |
| C20 (i) | guard | window_delegating | own `#[test]` | passes |
| C20 (ii) | guard | window_delegating | own `#[test]`, fresh VM (§0.5) | passes |
| P3 | red | window_delegating | §4.5 | fails |

### §4.3 Unit and walker cells

| cell | kind | file | setup | scaffold run |
|---|---|---|---|---|
| U1 | red | `tests_insertion_and_write.rs` | dispatcher installed | fails |
| U2 | window pin | same | Window; `<body onhashchange>` | passes |
| U3 (i), (ii) | guard (HEAD part) + new state (marker, C-1) | same | consumer with `EngineMode::BrowserCore` policy | HEAD part passes; marker part fails |
| U4 | guard | same | — | passes |
| U5 (i) | red | same | — | fails |
| U5 (ii) | guard | same | never inserted | passes |
| U5 (iii) | new state (C-1) | same | entry called on a marked element | fails (no marker at base) |
| U6 | guard | same | `clone_node_shallow` | passes |
| U9 | guard | same | pre-bind shape, host + `attach_shadow`, fresh consumer for the second init | passes |
| U7 | red | `tests_bind.rs` | pre-bind shape; two fresh consumers | fails |
| U8 (i), (ii) | guard | same | `create_document_node`; `attach_template_contents` | pass |
| U8 (iii) | red-first (C-1) | same | shadow host in the bound document | fails |
| U10 | red | same | chain > `MAX_ANCESTOR_DEPTH` (10 000), built bottom-up | fails |
| U11 | red | same | one parent, > 10 000 children | fails |
| U12 | red | same | bound `d2 = create_document_node()` ≠ `document_root()` | fails |
| P1, P2 | white-box guards | `tests_insertion_and_write.rs` | §4.5 | P1 (i) passes (no marker at base); P1 (ii) fails (new state, C-1). P2 tests a pure new function written in the scaffold, so it has no base counterpart |
| W1 | red | `crates/core/elidex-ecs/src/dom/tests/traversal.rs` | `Exclude` over depth > 10 000 and width > 10 000 | n/a (new API; written with the fix commit) |
| W2 | guard | same | order: host with shadow root and light children → host, shadow root, shadow children, light children | — |
| W3 | guard | same | `Exclude` skips template contents; `Include` reaches them | — |
| W4 | guard | same | cyclic sibling chain terminates (visited set) | — |

W1–W4 test a new public API, so they have no pre-fix run; U10 and U11 carry the red-first obligation for the walk.

### §4.4 Shell pins (`content_eh_attrs_bind_tests.rs`)

| pin | kind | harness | oracle read | note |
|---|---|---|---|---|
| E1 | red | `build_main_document(M)` | `probe_body_attr("data-hit")`, control `data-ran` | — |
| E2 | red | E1 + `dispatch_event` click at `#b` | `data-hit == "2"` | — |
| E3 | red | E1 + two `eval_script`, one `dispatch_event` between | strings `"function"`, `"true"` | rebind invariant |
| E4 | red | `build_main_document(<body onhashchange…>)` | `data-h` | — |
| E5 | red | `make_out_of_process_entry` + `run_oop_and_collect_posts` | posts contain `"ran"` and `"eh-hit"` | covers `load.rs:327`; `:221` by A-9 |
| E6 | red | `build_test_content_state` with `<iframe srcdoc='…'>` | child `probe_body_attr("data-hit")` | M's `'b'` written as `&#39;b&#39;` inside `srcdoc='…'` |
| E7 | red | `build_main_document` with a declarative shadow root | `data-hit` | depends on Q-S1 (§8) |
| E8 | guard | three brackets (`build_main_document`, two `eval_script`) | bracket 2 `true`; bracket 3 `"1,"` | Q7: §0.5 fallbacks, chosen by its scaffold run; if all fail, stop and return to plan-review (§0.5) |

Every red pin's control (`data-ran == "1"`, or the `"ran"` post) must be green on the scaffold run, or the pin does
not count as red (umbrella §6.2).

### §4.5 Plan-added cells

- **P1** (white-box guard, A2-MIN-1). Dispatcher installed, Window present. (i) A `<body onhashchange=X>` appended: not
  marked, and the Window entry is raw `X`. (ii) A `<body onclick=A onhashchange=X>` appended: marked.
- **P2** (white-box guard, A2-MIN-3). For every `EVENT_HANDLER_ATTRS` row with `HandlerScope::Window`:
  `is_window_delegating_pair` is true on a `body` and on a `frameset`. For each name in the reflecting const: true on
  both, and the name is a table row. For `onclick` on a `body`, and every name on a `div`: false. HEAD's predicate
  (`scope == Window && is_body_element`) implies the classifier for every row. The const equals the six names of A-11.
  **Fixed ratchet**, independent of the table derivation: the test holds two literal lists, the 18 WindowEventHandlers
  member names from `.claude/tools/webref idl html WindowEventHandlers` (`onafterprint onbeforeprint onbeforeunload
  onhashchange onlanguagechange onmessage onmessageerror onoffline ononline onpagehide onpagereveal onpageshow
  onpageswap onpopstate onrejectionhandled onstorage onunhandledrejection onunload`) and the 6 reflecting names
  (`onblur onerror onfocus onload onresize onscroll`). It asserts that the set of `HandlerScope::Window` rows equals the
  first list and the const equals the second, and that `is_window_delegating_pair` is true on `body` and `frameset`
  for all 24 literal names. Demoting a row from `Window` to `Global`, or editing the const, fails the ratchet even
  though the derived checks above would shrink with it. A spec change updates the literal lists by hand, from webref.
- **P3** (red, VM; rev-8 MIN). `src = document.createElement('frameset')`; `src.setAttribute('onclick', 'A')`;
  `src.setAttribute('onhashchange', 'window.h=1')`; `f = src.cloneNode()` (unmarked, both attributes, no entries);
  `f.setAttribute('onclick', 'B')`; `f.onclick = fn`; `el.append(f)`; then `f.onclick === fn`, and
  `f.dispatchEvent(new Event('hashchange'))` sets `window.h === 1`.
  - Fix: `onclick` is `f`'s only deliverable ordinary attribute, so the write passes D-8, marks `f` and processes only
    `onclick`. The append skips `onclick` (`fn` survives: spec) and re-raws `onhashchange` on `f` (HEAD's target;
    window pin).
  - HEAD: the append re-raws `B` over `fn`, so `f.onclick !== fn`. Red.
  - An impl that leaves `(frameset, onhashchange)` out of X-W counts two deliverable ordinary attributes. The write
    then processes nothing, and the append re-raws `B` over `fn`: fails.
  - Spec: `fn` (insertion runs no handler steps); the `hashchange` target is the Window (T3 facet 3), so the second
    assertion is a window pin.

### §4.6 Mutation checks (umbrella §6.3; run once at the final head)

Each mutant is applied, the named cells are run and must fail, and the mutant is reverted. Results go in the PR
description. At most two mutants run in parallel.

| mutant | must fail |
|---|---|
| drop the marker check in the D-2 function | C1, C2, C3, U1, E3, U7 |
| make `initialize_from_tree` a no-op (delete the bind delegate) | E1, E2, E4, E5, E6, E7, U7 |
| drop the document-marker check, or hold the once-state on the consumer struct | U9, E8 |
| bind walk over the light tree only | E7, U8 (iii) |
| bind walk capped (`children_iter` / `for_each_shadow_inclusive_descendant`) | U10 or U11 |
| bind walk recursive | U10 |
| Insert walk shadow-including | U9, E8 |
| write-side entry that never processes | C2, C4, C5, C11, U5 (i) |
| write-side entry without the D-8 (a) count | C16 |
| entry that processes a Window-delegating written pair | C17, C18 |
| `HandlerWrite` timing that processes Window-delegating pairs | C20 (i), C20 (ii) |
| skip at insertion covering Window-delegating pairs | C19 |
| classifier = HEAD's predicate (`scope == Window && is_body_element`) | C19b, C19c |
| classifier missing `frameset` for WEH rows | P3 |
| classifier keyed on the node document or the bound document | C8 or C18 |
| bind pass walks another document, or `TemplateContentsWalk::Include` (NEW) | U8 (i) / (ii) |
| bind pass keyed on `document_root()` | U12 |
| marker inserted on a Window-delegating-only element | P1 (i) |

WPT: no subset is claimed (umbrella §6.3).

### §4.7 Kept green

`tests_event_handler_attrs.rs` (all), the consumer's `spec_level_tests` (`event_handler_consumer.rs:500-590`; its
fabricated AttributeChange event carries no real attribute, so the entry returns and the arm writes as before),
`dom/tests/destroy.rs` (walker order change), `tests_base_href.rs` and `document_base.rs` tests (init key),
`tests_dom_parser.rs`, `content_iframe_security_tests.rs` (helpers moved). Existing tests that build pre-bind DOM with
`on*` attributes now get those handlers delivered at bind. A test whose oracle assumed G1 (inert initial handlers)
would change. None is known (scout Q-S4); `cargo test -p` on every touched crate is the backstop.

**Disposition of an existing test that flips.** A flip has exactly these allowed causes:
- the G1 inversion (initial handlers now delivered at bind; G1 / D-5);
- the base-URL init key change (§0.5, one key);
- the K-0 spec clauses at T-a2 (a marked element is skipped at insertion) and at T-a3 (the first handler write
  processes the element). Example: an existing test that expects HEAD's re-raw after
  `setAttribute('onclick','A'); el.onclick=fn; parent.append(el)` flips to `fn`.

For such a test:
- its oracle is rewritten to the spec or K-0 value the umbrella cells derive for that case (C2, C4, C5, C11, U1),
  never to whatever the new code happens to produce;
- the PR description lists it with the clause that flips it (G1 / D-5, the §0.5 one-key row, or K-0 T-a2 / T-a3).

Exception: `tests_event_handler_attrs.rs` is kept green (umbrella §6.3 "Kept green", umbrella `:758`). A flip there
is not rewritten; implementation stops.

**Terminal**: a flip that none of these causes explains is a bug in the implementation, not an oracle to update.
Implementation stops and this plan returns to plan-review. The test is not reshaped or dropped during implementation.

## §5. 1000-line touch-time check

| file | now | projected | decision |
|---|---|---|---|
| `event_handler_consumer.rs` | 590 | ≈ 500 (helpers move out) | no split |
| `event_handler_processing/mod.rs` (NEW) | — | ≈ 400 | new file |
| `event_handler_processing/tests_insertion_and_write.rs` (NEW) | — | ≈ 650 | new file |
| `event_handler_processing/tests_bind.rs` (NEW) | — | ≈ 450 | new file |
| `lib.rs` (session) | 62 | ≈ 66 | — |
| `consumer_dispatcher.rs` | 153 | ≈ 165 | — |
| `vm_api.rs` | 762 | ≈ 762 | — |
| `event_handler_attrs.rs` | 726 | ≈ 745 | no split |
| `test_helpers.rs` | 213 | ≈ 260 | — |
| `vm/tests/mod.rs` | 197 | 199 | — |
| `tests_event_handler_attrs_once.rs` (NEW) | — | ≈ 650 | new file |
| `tests_event_handler_attrs_window_delegating.rs` (NEW) | — | ≈ 550 | new file |
| `tests_event_handler_attrs.rs` | 735 | 735 | not touched |
| `walkers.rs` | 321 | ≈ 390 | — |
| `teardown.rs` | 367 | ≈ 335 | — |
| `dom/tests/traversal.rs` | 267 | ≈ 380 | — |
| `tree_clone.rs` | 425 | 425 | — |
| `attribute_reconcile.rs` | 66 | ≈ 66 | — |
| `document_base.rs` | 555 | ≈ 558 | — |
| `content_eh_attrs_bind_tests.rs` (NEW) | — | ≈ 450 | new file |
| `content_test_support.rs` | 288 | ≈ 390 | — |
| `content_iframe_security_tests.rs` | 667 | ≈ 590 | — |
| `content/mod.rs` | 858 | ≈ 861 | no split |

No touched file is over 1000 lines, and no projection crosses 1000, so **no prerequisite split PR**. The umbrella's
§6.4 risk (the consumer file and `tests_event_handler_attrs.rs` crossing 1000) is avoided by placement: the mechanism
goes to a new module, and the cells go to new files split by scenario (once and writes / Window-delegating; insertion
and writes / bind). Files over 1000 lines nearby and **not touched**: `crates/shell/elidex-shell/src/tests.rs` (1082),
`viewport_tests.rs` (1172), `crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs` (1143). Line counts:
`wc -l` at `b9122157`, verified 2026-10-26.

## §6. Verification artifacts

- **§5.0 census (umbrella)**: unchanged, because `git diff --stat fd6955e8 b9122157 -- crates` is empty (verified
  2026-10-26). The implementer re-runs the umbrella's `c()` function on the PR head before review.
- **O4, writers of handler state in `EventListeners`** (production hits, umbrella's prod/test classifier, verified 2026-10-26):
  - `/usr/bin/grep -rnE --include='*.rs' '\.set_uncompiled\(' crates` → prod 1: `event_handler_consumer.rs:474` (moves
    into the new module).
  - `'\.add_event_handler\('` → prod 2: `event_handler_consumer.rs:473`, `event_handler_attrs.rs:674` (activation).
  - `'\.set_handler_callable\('` → prod 2: `event_handler_attrs.rs:574` (lazy compile), `:681` (activation).
  - `'\.mark_cleared\('` → prod 1: `event_handler_consumer.rs:487`.
  - `'\.clear_uncompiled\('` → prod 2: `event_handler_attrs.rs:677`, `:687` (activation).
  - `'get::<&mut (elidex_script_session::)?EventListeners>'` → prod 7. Beyond the sites above they are
    `event_dispatch/mod.rs:500` and `natives_promise.rs:567` (`{once}` removal of Normal listeners) and
    `dispatch_target.rs:126`, `:155` (the activation adapter).
  - `'remove_listener_and_prune_back_ref\('` → prod 7: the lazy-compile drops (`event_handler_attrs.rs:576`, `:579`),
    activation (`:692`), `removeEventListener` (`event_target.rs:619`, which ignores handler listeners per
    `tests_event_handler_attrs.rs:321`), `{once}` (`event_target_dispatch.rs:509`), the `ScriptEngine` impl's `remove_listener`
    (`crates/script/elidex-js/src/engine.rs:303`), and the def (`inner.rs:49`). Rev 1 missed `engine.rs:303`. It writes
    no handler state: its only caller is the dispatch loop's `{once}` cleanup (`event_dispatch/mod.rs:519`, under
    `if entry.once`), handler listeners are created with `once: false` (`event_listener.rs:178`,
    `add_event_handler`), and it drops only the engine-side function store (`inner.rs:54`, `host.remove_listener`),
    not `EventListeners` (the `{once}` `EventListeners` removal is `event_dispatch/mod.rs:500`, listed above).
  - Verdict: no VM path writes handler state outside the IDL activation path and the lazy-compile reconcile. O4 is
    closed with no new K-0 case and no slot.
- **No VM code names the markers** (DoD check, at the head):
  `/usr/bin/grep -rnE --include='*.rs' 'HandlerAttrsProcessed|PreBindHandlerAttrsProcessed' crates/script/elidex-js/src | /usr/bin/grep -v '/tests/'`
  must print nothing.

## §7. Definition of done (umbrella §10.1, realised)

- `HandlerAttrsProcessed`, `PreBindHandlerAttrsProcessed`, `process_before_handler_write` and
  `EventHandlerAttributeConsumer::initialize_from_tree` (NEW) exist in `elidex-script-session`; the markers and the entry are
  re-exported from `lib.rs`.
- The Insert arm walks `node` + `traverse_descendants(node)`. The bind walk is the moved uncapped walker with
  `TemplateContentsWalk::Exclude`. It is keyed on, and marks, the bound document; no once-state on the consumer struct.
- The write-side entry implements §1.4 (b); its doc and `handle_insert`'s doc name `#11-eh-attrs-creation-time-call-sites`.
- `initialize_consumers(dom, document)` delegates to both consumers; its doc, the field doc and the `vm_api.rs` comment
  are rewritten (§1.7).
- Both VM setters call the entry as §1.5; §6's marker grep prints nothing.
- Clone-policy row, module docs and G6 (both stale claims) updated.
- Cells C1–C9, C11–C20 (C19b, C19c, C20 (i), C20 (ii)), U1–U12, P1–P3, W1–W4 and pins E1–E8 except E5b (§0.5) are
  present. The scaffold run (§1.8) is recorded in the PR description, with every red cell and pin failing and every
  control green. All pass at the head. Mutation checks (§4.6) are recorded.
- `cargo test -p elidex-script-session --all-features`, `-p elidex-js`, `-p elidex-shell`, `-p elidex-ecs`,
  `-p elidex-dom-api`, and `mise run ci` are green.
- The PR cites `#11-eh-attrs-creation-time-call-sites` (registered at the approval PR's landing; the PM confirms).
- The PR description lists every flipped existing test with the clause that flips it (§4.7).
- `python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-10-eh-attrs-slice-a.md` exits 0.

## §8. Questions for a scout (before implementation)

- **Q-S1 (E7, Q7 fallback 2).** Does `parse_progressive_str` (`crates/dom/elidex-html-parser/src/lib.rs:194`) attach a
  declarative shadow root for `<div id=h><template shadowrootmode=open>…</template></div>` in a **document** parse, on
  the strict tier? If the strict tier fails on E7's markup, does the tolerant fallback attach it? If neither does, E7
  needs a different shape and returns to plan-review.
- **Q-S2 (Q7).** Do E8's bracket-1 calls (`attachShadow` on a `div`, `cloneNode` of a two-attribute `div`) run in an
  initial script through `build_pipeline_from_loaded`? The natives exist (`vm/host/element_shadow.rs`); the scaffold run
  settles it (§0.5). If neither they nor both fallbacks work, E8 returns to plan-review, as E7 does.
- **Q-S3 (init key).** Which tests bind a document that is not `document_root()` (`bind_vm` callers with
  `create_document_node`) and assert base-URL state? Those are the only tests the one-key decision can change.
- **Q-S4 (kept green).** Which existing tests (VM, shell, form, dom-api) build a pre-bind DOM or initial HTML with `on*`
  attributes and assert that the handler is absent or inert? They encode G1 and would flip.

## §9. Hand-offs to the PM (no edit here)

- **A3-MIN-3**: `#11-eh-attrs-creation-time-call-sites` defect (1) and its "What it records" sentence say "deliverable
  handler content attribute(s)". The restriction counts **ordinary** pairs only (D-8 (a)), so both should read
  "deliverable ordinary". Ledger text; the PM's edit.
- **rev8:533**: umbrella §3 row 2's Touch cell reads "step 3 is T3, the reflecting set facet 4". T3 now carries facets 1,
  3 and 4. The cell would read better as "step 3, `<frameset>` and the reflecting set are T3 (facets 1, 3, 4)". Umbrella
  wording only.
- **`#11-storage-event-mode-aware-delivery`**: the stale docstring it is cited from (`event_handler_consumer.rs:235-242`)
  is fixed in this PR (§1.7), to the wording of the slot's ledger row (re-homed by the PM, 2026-10-26). No ledger edit.
- **Slice b input, N-5**: `collect_template_inclusive_descendants` (`elidex-form`, `init` module)
  (`crates/dom/elidex-form/src/init.rs:111`) is not unified onto the new walker here (§1.6). Its reconciliation with
  `TemplateContentsWalk::Include` goes to slice b's input (umbrella §9 N-5).
- **Slots whose trigger fires at slice a's landing** (ledger `project_open-defer-slots.md`, found by slot ID; read
  2026-10-26):
  - `#11-event-handler-target-determination`: facet (4) (the reflecting set) has "slice a's landing is an added
    trigger for this slot", because initial-HTML `<body onfocus>` etc. start landing on the body element (G1); facet (1)
    (step 3, T3) has "Added trigger: slice a's landing" (PM decision 25: T3 starts after slice a lands).
  - `#11-event-handler-attribute-edge-cases`: "Trigger: slice a's landing, …" (initial-HTML handlers start compiling,
    so facet (1), the basic-scope compile, becomes reachable on ordinary pages). Exposure stated here: E-G1 (ii), §3.1.
  - `#11-csp-inline-handler-enforcement`: "Trigger: … slice a's landing; …" (initial-HTML inline handlers join the set
    that runs with no `script-src` check). Exposure stated here: E-G1 (iii), §3.1.
  - `#11-template-contents-owner-document` (C1): "Added trigger: those two landings" (EH slice a and IBP-sandbox N1;
    the PM assigns its owner at them, PM decision 21).
  - `#11-eh-attrs-creation-time-call-sites` is not triggered; its closing path starts at slice a's landing (T3, then
    C1's handling, then slice b).

## §10. Changelog

- **rev 3** (rev-2 focused re-check: CRIT 0 / IMP 1 / MIN 4, all fixed):
  - N1: PM decision 35 — module kept; umbrella §4 line corrected in this PR; general licence removed: header paragraph,
    §0.5 "marker home" row (reason: 1000-line touch-time rule; umbrella §6.4 prerequisite split does not arise), §1.3
    umbrella row, §1.7 first bullet (before/after text).
  - N2 flip disposition: allowed causes add the K-0 clauses at T-a2 / T-a3 (oracle → the value cells C2, C4, C5, C11,
    U1 derive); `tests_event_handler_attrs.rs` excepted (kept green, umbrella §6.3); terminal (unexplained flip → stop,
    return to plan-review): §4.7; PR-description list of flipped tests: §7 DoD.
  - N3 teardown-first bind: `Vm::teardown_document` (`vm/lifecycle.rs:536`) does not despawn the DOM; the bind-pass
    writes stay observable through ECS, and a rebind does not reprocess (document marker): §0.2.
  - N4 reach set adds `bind_vm` (`vm/test_helpers.rs:49-54`, compiled, test/bench callers only); the enumeration grep
    is replaced by one that reproduces it (38 hits, classified): §0.2.
  - N5 A2-MIN-3: a new WEH row needs no classifier edit; P2's literal list is updated by hand (P2 red until then,
    intended, F12): §0.5.
- **rev 2** (plan-review R1: CRIT 0 / IMP 0 / 15 distinct MIN, all fixed):
  - F1 E8 terminal (all fallbacks fail → stop, return to plan-review): §0.5 Q7 row, §4.4 E8 note, §8 Q-S2.
  - F2 disposition of existing tests that flip (G1 inversion / init key; oracle → derived outcome, listed in the PR;
    any other flip stops implementation): §4.7.
  - F3 stale `web_storage_spec_level` docstring fixed in this PR, to the re-homed ledger row's wording: §1.3 consumer
    row, §1.7; §9 hand-off replaced by a citation.
  - F4 marker home moved to `event_handler_processing/` (same crate, re-exports, kinds; reason §5 / umbrella §6.4):
    §0.5 "marker home" row; header paragraph (file placement inside a decided crate is this plan's).
  - F5 `collect_template_inclusive_descendants` out of scope, handed to slice b's input (umbrella §9 N-5); I10 limited
    to its three callers: §1.6, §2, §9.
  - F6 HTML §13.2.6.4.4 "in head" template step 13 "Keep" sub-step 1 row (E7): §3.
  - F7 comment "HTML §8.1.8.1 attribute change steps 5.5–5.6": §1.4 (a).
  - F8 `resolve_handler_target`'s moved doc cites §8.1.8.1 *determining the target of an event handler* steps 1–2:
    §1.4 (e).
  - F9 per-NEW-item citation table, "none — elidex mechanism" where no spec step exists: §1.7.
  - F10 slots triggered by slice a's landing, by slot ID: §9.
  - F11 O4 census adds `engine.rs:303` (prod 7) with why it writes no handler state: §6.
  - F12 P2 fixed ratchet (18 WEH + 6 reflecting names as literal lists): §4.5 P2, §0.5 A2-MIN-3 row.
  - F13 every production `Vm::bind` caller and the document it passes: §0.5 "two init keys" row.
  - F14 `components.rs:731-733` and `tree_clone.rs:24` checked, not part of the G6 fix (no bypass claim): §1.7 G6.
  - F15 bind-pass reach set (`with_bound`, `teardown_document`) and teardown-first bind: §0.2.
