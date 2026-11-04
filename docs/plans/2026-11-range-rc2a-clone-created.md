# RC-2a — the clone-creation event, its two consumers, `cloneNode` on it, D14, the #466 reversal (per-PR plan-memo)

Slice **RC-2a** of the Range contents program (umbrella `docs/plans/2026-11-range-contents-program.md`, landed by #583
`12ea5dce`; "the umbrella"; its line cites are anchored at `ee6c7a1a` per its header, and this memo cites it by section
and item, not by line, except where a line is the only handle). Slot: `#11-clone-shadow-children-subtree` (registered,
PM 212; closed by this PR). Base: `origin/main` `fab44a1d` (RC-0a #587 merged). Every code fact below is re-measured at
`fab44a1d` unless marked; line numbers move, the commands do not. Owner lane: L3 DOM/form.

**What binds this memo.** The umbrella's §5 row for RC-2a, its §5.2a scope / files / tests / DoD, its §4.3 conditions
1-6 and driver decision D-g′, its §2 invariants C2 / E2 / L4 / X1 / X2 (and the A6 rule, canonical in §2 C3, which this
slice does not touch), its §0.4 D14 rows, its §9 order (RC-0a → **RC-2a** → RC-1 → RC-2b → RC-3; RC-2a before C1), and
its Appendix H (H6 census collision with C0c, **H9 = the canonical hand-stamp rule**). Those are inherited as decisions
and referenced, never restated. What the umbrella left to this memo (its "For the per-PR plans" list) is decided in
§0.4. This plan-review is also a **review venue for the #466 I2 / I3 reversal** (umbrella R6-3; §4.8 here).

**Rev 1** (2026-11-04) — first draft.

## §0. Frame, premises, decisions

### §0.1 Frame

RC-2a is the engine-wide mechanism, not a Range change: clone creation becomes a `MutationEvent` fired once by the
canonical cloner's entry, and the two post-clone passes (custom-element upgrade enqueue; form-control cloning steps)
run **at dispatch time** inside the clone, in the consumers of their own crates. `cloneNode` moves onto it and the VM
post-pass is deleted. D14 (shadow children cloned with subtree true) lands in the same cloner. RC-2b later calls the
same entry once per clone root and gets both passes for free (umbrella §4.3 condition 1, "Why the entry"). No Range
file is edited.

### §0.2 Premises re-measured at `fab44a1d`

- **The cloner.** `crates/dom/elidex-dom-api/src/node_methods/clone.rs` (458 lines): `CloneNode`'s `invoke` (`impl DomApiHandler for CloneNode`) `:91-167`
  (Attribute stub `:120-125`, ProcessingInstruction stub `:126-131`, the entry call `:154`);
  `clone_node_with_shadow_honor` `:227-306` (doc `:169-226`; pass-4 doc `:208-216` says step 6.7 uses "the *same*
  `subtree` flag as the overall operation"; `shadow_doc` `:239-243`; worklist `:247-304`; pass 3 call with `deep`
  `:262`; host / shadow-root hand stamp `:282-285`; step 6.7 comment `:288-289`; the shadow-child clone with `deep`
  `:290`; the `!deep` hand stamp `:294-301`; `append_child(cloned_shadow, …)` `:302`; `Some(cloned)` `:305`);
  `clone_recording` `:313-327`; `propagate_ce_identity` `:344-366` (its doc `:329-343` names the VM seam);
  `mark_cloned_form_control` `:381-387`; `replicate_template_contents` `:405-431` (gated on `deep` at `:423`). The
  `ClonedFrom` doc `:24-75`: "Why a persisted marker, not a returned `(src, dst)` list" `:48-57`, "The sole production
  caller is the VM `cloneNode` shim" `:58-64`.
- **Pair order.** `EcsDom::clone_subtree` pushes the root pair first (`crates/core/elidex-ecs/src/dom/tree_clone.rs:217`)
  and `clone_children_recursive` links each child with `append_child` (`:419`) then pushes its pair (`:420`). So a deep
  clone fires one `Insert` per cloned child **during** the entry call, with a detached parent; `fire_after_insert`
  skips only ShadowRoot nodes and parents (`crates/core/elidex-ecs/src/dom/tree/mutation.rs:282-292`). Read from the
  code, not run. Consequence: the clone's own appends already put events on the stream today; `CloneCreated` is the
  **last** event of the entry call. (The CE consumer's `handle_insert` returns at once for a disconnected parent,
  `crates/dom/elidex-custom-elements/src/consumer.rs:109-111`; the form reconciler's `handle_insert` attaches a
  default `FormControlState` to any cloned control it reaches, `crates/dom/elidex-form/src/reconciler.rs:84-95`,
  which the sweep then overwrites through `create_form_control_state`, `crates/dom/elidex-form/src/clone.rs:114`.
  Pre-existing, unchanged by this PR.)
- **The event stream.** `crates/core/elidex-ecs/src/dom/mutation_event.rs` (352 lines): module doc `:1-39`
  (re-entry contract `:26-32`); seven variants `:107-312`; `MutationDispatcher` doc `:314-332` (its "Production impl"
  list `:319-322` names four consumers; there are seven today, `crates/script/elidex-js/src/vm/consumer_dispatcher.rs:152-158`);
  `dispatch` `:351`. `crates/core/elidex-ecs/src/dom/mod.rs` (926 lines, `wc -l`): re-export `:32`; the private
  `dispatcher` slot `:58-66`; `set_mutation_dispatcher` `:167-172`, `clear_mutation_dispatcher` `:175-177`,
  `take_mutation_dispatcher` `:181-185`; the private `fn dispatch_event` `:199-220` (`debug_assert_eq!(dispatch_depth, 0)`
  `:200-206`, take `:207`); `fire_split_text` `:231-247` (calls `dispatch_event` at `:246`); `fire_normalize_merge`
  `:258-274`. `crates/core/elidex-ecs/src/lib.rs:44-47` re-exports `MutationDispatcher` / `MutationEvent`.
- **The walk.** `EcsDom::for_each_shadow_inclusive_descendant` (`crates/core/elidex-ecs/src/dom/tree/walkers.rs:54-80`):
  visits the node (`:63`), then its light children, then its shadow root's children (`:69-73`, the ShadowRoot entity
  itself is not visited); depth cap `MAX_ANCESTOR_DEPTH` (`:60-62`; `dom/mod.rs:46` = 10,000).
- **The CE side.** `CustomElementReactionConsumer::handle` (`consumer.rs:75-96`) matches `Insert` / `Remove` /
  `AttributeChange` and drops the rest at `:94`; `handle_insert` `:98-123` takes `&EcsDom`, locks the registry
  (`:116`), classifies, drops the guard, then locks the queue and `extend`s (`:120-121`); `handle_attribute_change`
  `:138-159` has the same lock-disjoint shape. `apply_clone_creation_ce_semantics`
  (`crates/dom/elidex-custom-elements/src/upgrade.rs:125-156`): one `for_each_shadow_inclusive_descendant` walk
  (`:131`), `prepare_upgrade` per entity (`:55-99`, a pure read; `Null` association skipped `:74-76`), then the
  `is_value` clear through `world_mut()` (`:150-151`); its contract "MUST only be called on a freshly cloned subtree"
  `:121-124`. Its inline tests call it at `:391`, `:416`, `:427`, `:452`. Re-exported from
  `crates/dom/elidex-custom-elements/src/lib.rs:29-33`.
- **The form side.** `FormControlReconciler::handle` (`reconciler.rs:42-53`) matches `Insert` / `AttributeChange`,
  `_ => {}` at `:51`; module doc `:4-8`. `apply_clone_form_state(dom, clone_root)` (`crates/dom/elidex-form/src/clone.rs:44-65`)
  collects `collect_template_inclusive_descendants` (`:49`; `crates/dom/elidex-form/src/init.rs:111-125`, a
  fixpoint over `<template>` content fragments on top of the plain walk) and, per `ClonedFrom` marker, removes the
  marker (`:62`) and calls `copy_form_state` (`:74-152`), which calls `create_form_control_state` (`:114`;
  `init.rs:63-110`, with `finalize_control` `:128` and `apply_element_state_flags` `:150`: component reads and writes
  only — the three `append_child` calls in `init.rs` (`:211`, `:226`, `:237`) are in its inline test module). Module
  doc `:16-18` says "invoked synchronously at clone time from the VM `cloneNode` shim". Its inline tests call
  `apply_clone_form_state(&mut dom, dst)` at `:195`, `:215`, `:247`, `:269`, `:285`, `:297`.
- **The VM post-pass (deleted by this PR).** `crates/script/elidex-js/src/vm/host/node_methods_extras.rs:201-236`
  `native_node_clone_node`: `invoke_dom_api(ctx, "cloneNode", …)` `:211-212`, `install_document_methods_if_cloned_doc`
  `:213` (stays), then the post-pass `:214-234` (CE `:221`, form `:229-231` behind `host_if_bound()`); doc `:187-190`.
  `crates/script/elidex-js/src/vm/host/custom_elements/upgrade.rs:128-156` `enqueue_subtree_upgrade_reactions` (doc
  `:117-127`): registry lock `:135-138` (poison-tolerant), the engine pass `:142`, queue lock `:147-150`, pushes
  `:151-155`. The umbrella's §5.2a DoD grep gives the same five hits at `fab44a1d`:
  `git grep -n 'enqueue_subtree_upgrade_reactions\|apply_clone_creation_ce_semantics\|apply_clone_form_state' -- crates/script/elidex-js/src ':!*tests*'`
  → `custom_elements/upgrade.rs:128`, `:142`; `node_methods_extras.rs:190`, `:221`, `:230`.
- **Dispatch order and docs.** `consumer_dispatcher.rs` (160 lines): `form_control` fourth, `custom_elements`
  seventh and last (`:152-158`); field docs `:50-57` (`form_control`) and `:78-87` (`custom_elements`:
  "Order-independent — only pushes to the side-band reaction queue").
- **Bind / unbind.** `Vm::bind` builds the `ConsumerDispatcher` (`crates/script/elidex-js/src/vm/vm_api.rs:300-305`)
  and installs it (`:324`, `debug_assert` that nothing was displaced `:325-329`; the umbrella's `:316` has moved);
  `Vm::unbind` clears it (`crates/script/elidex-js/src/vm/lifecycle.rs:34`).
- **`is_value` writers in the VM.** `git grep -n -w 'is_value' -- crates/script/elidex-js/src ':!*tests*'` →
  `vm/host/custom_elements/creation.rs:233` only (the umbrella's §0.3 item 29 fact holds; a `-w` grep, because the
  umbrella's bare pattern also matches `this_value`).
- **Drain points.** `git grep -n 'flush_ce_reactions(' -- crates/script/elidex-js/src ':!*tests*'` → the definition
  `vm/host/custom_elements/flush.rs:39` and four callers: `engine.rs:165`, `vm/host/custom_elements/define.rs:204`,
  `vm/interpreter.rs:54`, `vm/natives_timer.rs:281` (umbrella E2, unchanged).
- **Tests after RC-0a.** `crates/dom/elidex-dom-api/src/node_methods/tests/clone.rs` is 720 lines and
  `crates/dom/elidex-dom-api/src/node_methods/tests/clone_shadow.rs` (RC-0a) 277. The D14 test is now
  `clone_node_shallow_replicates_own_clonable_shadow_shallowly` (`clone_shadow.rs:76-112`; the wrong assertion at
  `:108-111`), the file header `:9-14` states the wrong rule ("the shadow children's own clone depth follows the
  flag, step 6.7"); the stamp guard is `clone_node_shallow_shadow_children_carry_owner_document` (`:186-214`), and
  `clone_node_shallow_shadow_root_carries_owner_document` (`:244-277`) pins the `:282-285` stamps. The `elidex-ecs`
  recorder: `crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs` (971 lines) holds the private `MockEvent`
  (`:16-52`), `MockHook` with its exhaustive `match` (`:54-150`, the match `:68-148`) and `install_mock` (`:152-157`);
  `crates/core/elidex-ecs/src/dom/tests/mutation_hook_specialised.rs` (179) carries its own hooks;
  `crates/core/elidex-ecs/src/dom/tests/dispatch_guard.rs` (69) tests the take-and-restore guard;
  `crates/core/elidex-ecs/src/dom/tests/traversal.rs` (413) tests the walkers. VM: `tests_custom_elements.rs:567-587`
  (`clone_node_attaches_ce_state_so_upgrade_fires`), `tests_create_element_options.rs:219-237`
  (`clone_of_defined_custom_element_upgrades_via_clone_reaction`, a never-inserted clone upgraded at the eval
  checkpoint), `:239-` (`clone_of_null_registry_element_not_upgraded`, `clone_after_define_drops_retained_autonomous_is_value`);
  `tests_clone_form_state.rs` (11 tests, `:48-180`; the I3 case `clone_input_copies_live_value_detached` `:48-56`).
- **Exhaustive `MutationEvent` matches (census, umbrella §4.3 condition 1).** `git grep -l 'MutationEvent::' -- crates ':!*tests*'`
  → 31 files at `fab44a1d` (the umbrella's count holds). Of them, the `match` sites: `range/live.rs:113-163`
  (**exhaustive**, no wildcard; the `AttributeChange` arm `:159-163`); `consumer.rs:76-95` (`_ => {}` `:94`);
  `reconciler.rs:43-52` (`_` `:51`); `crates/dom/elidex-dom-api/src/element/document_base.rs:292-399` (`_` `:399`);
  `crates/script/elidex-script-session/src/event_handler_consumer.rs:382-390` (`_` `:390`);
  `crates/api/elidex-api-canvas/src/component.rs:202` and `crates/dom/elidex-dom-api/src/traversal.rs:1194` (`let … else` /
  `if let`, not exhaustive); `crates/dom/elidex-dom-api/src/test_util.rs:32` (`matches!`); the rest are fire sites or
  doc mentions. So one production exhaustive match gains an arm (`range/live.rs`), plus the test recorder
  (`mutation_hook.rs:68-148`), plus the two consumers' real arms.
- **No-dispatcher windows (umbrella §0.3 item 34, re-run).** The same `-P` command at `fab44a1d` gives **25** lines
  (27 at `27b50dfe`): RC-0a moved `range/live.rs`'s inline tests to `range/live_tests.rs`, which the `':!*tests*'`
  exclusion drops, so the two test hits `live.rs:588` / `:868` are gone. The 19 production lines are unchanged in
  substance: the slot's own methods (`dom/mod.rs:167`, `:171`, `:175`, `:176`, `:181`, `:184`) and `dispatch_event`'s
  take (`:207`); `despawn_subtree` (`tree/teardown.rs:209` take, `:240` restore); `begin_detached_fragment`
  (`teardown.rs:293` definition, `:300` take) and its two callers (verified 2026-11-04 by the same command: `crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs:180`,
  restore `:241`; `crates/dom/elidex-html-parser/src/lib.rs:143`, restore `:162`); `parse_into_inert_document`
  (`crates/dom/elidex-form/src/inert_document.rs:112`, restore `:199`); bind (`vm_api.rs:324`) and unbind
  (`lifecycle.rs:34`). The 5 inline-test lines (`char_data/split_text.rs:369`, `:395`; `elidex-form/src/value_mode.rs:130`;
  `event_handler_processing/mod.rs:317`; `mutation/html_fragment.rs:696`) and the comment (`lifecycle.rs:251`) are the
  rest. Full list with dispositions in §5.3.
- **Production cloner callers.** `git grep -n 'clone_node_with_shadow_honor\|"cloneNode"' -- crates` → the entry
  (`clone.rs:227`), its one production caller, `CloneNode`'s `invoke` (`clone.rs:154`), its doc mentions (`clone.rs:2`,
  `:329`, `:368`, `:389`; `node_methods_extras.rs:175`), the handler registration (`crates/dom/elidex-dom-api/src/registry.rs:394`,
  `:607`), the VM native's `invoke_dom_api` string (`node_methods_extras.rs:212`), and the well-known name
  (`crates/script/elidex-js/src/vm/well_known.rs:341`). One entry, one production caller, as the umbrella says.
- **Crate edges.** `elidex-custom-elements` depends on `elidex-ecs` only (`crates/dom/elidex-custom-elements/Cargo.toml:13-14`);
  `elidex-form` on `elidex-ecs` and `elidex-dom-api` (`crates/dom/elidex-form/Cargo.toml:15`, `:17`); `elidex-dom-api`
  on `elidex-ecs` and `elidex-custom-elements` (`crates/dom/elidex-dom-api/Cargo.toml:15`, `:18`), never on
  `elidex-form`. `#466`'s I1 holds and no edge is added.
- **#466.** `docs/plans/2026-07-clone-cloning-steps-fcs.md`: I1 `:52`, I2 `:53`, I3 `:54`, I4 `:55`, I5 `:56`; the
  pairs `:60-70`; the conclusion rejecting the #331 sketch's `MutationEvent::Clone { pairs }` `:73-79`. Landed as
  `7bafcb9f` (#466), which closed `#11-clone-cloning-steps-event` (ledger entry, read 2026-11-04).

### §0.3 Premise checks against the brief and the umbrella

1. The brief's "the `27b50dfe` re-run check (umbrella ~:712)" is the umbrella's §0.3 item 34, the no-dispatcher
   window census (its text starts at the umbrella's `:681`); decided in §0.4 D-4.
2. The umbrella's `vm_api.rs:316` (the dispatcher install) is `:324` at `fab44a1d`; `:292-316` for `Vm::bind`'s
   dispatcher construction is `:300-329`. Substance unchanged.
3. The umbrella's census count of 27 lines is 25 at `fab44a1d` for the reason in §0.2 (two test hits moved out of
   the production file). The 19 production lines are the same sites.
4. The umbrella's `tests/clone.rs:792-827` / `:902` cites are `clone_shadow.rs:76-112` / `:186-214` after RC-0a
   (the brief's item 1; this memo cites the post-RC-0a locations only).
5. The umbrella's `is_value` grep pattern (`git grep -n is_value -- …`) also matches `this_value` (17 lines at
   `fab44a1d`); with `-w` it gives the one line the umbrella meant. The fact stands.
6. The umbrella's D14 structural row says "the subtree flag is per pair … every pair it produces keeps true for passes
   3 and 4". Checked against the spec: the only pair whose subtree flag can be false is the **root** pair. "Clone a
   node" step 5 recurses with its own `subtree` (so only when it is true), step 6.7 passes true, and the template
   cloning steps pass true (step 2) or return (step 1) (`webref body dom concept-node-clone`; `webref body html
   the-template-element`). So "per pair" reduces to one rule: the operation's flag for the root pair, true for every
   other pair (§4.2). This is the umbrella's row made exact, not a change to it.
7. No premise in the brief was found wrong against the code or the umbrella.

### §0.4 Decisions (this PR)

- **D-1 (recorder visibility; brief item 2).** `MockEvent` gains a `CloneCreated { root, scope }` variant and
  `MockHook`'s exhaustive `match` gains its arm in `mutation_hook.rs` (the compiler forces it; about 10 lines, so the
  file stays under 1000). `MockEvent`, `MockHook` and `install_mock` become **`pub(super)`**, and RC-2a's recorder
  tests go into a **new sibling file** `crates/core/elidex-ecs/src/dom/tests/mutation_hook_clone_created.rs` (NEW).
  Reasons: one recorder for the whole stream (a second `MockEvent` beside the first would be a duplicated decision
  surface); the file was split by RC-0a four days ago and is a flat list of per-primitive tests with no further
  cohesion seam, so a re-split now would be churn without a seam; a per-variant sibling is the natural unit, which
  C0c mirrors for `Adopt` (its own sibling, the same `pub(super)` recorder). The nested-fire assertion test goes to
  `dispatch_guard.rs`, the guard's own file.
- **D-2 (§7 re-measure; brief item 3).** Every file RC-2a touches is under 1000 lines at `fab44a1d` (§7, with the
  `wc -l` command). `dom/mod.rs` is 926; RC-2a adds about 20 lines (one `fire_*` helper and a re-export); C0b has not
  landed. No prerequisite split. If C0b lands before RC-2a's implementation starts, `dom/mod.rs` is re-measured at
  that base and a standalone split comes first if the two together reach 1000 (umbrella §7 row).
- **D-3 (where the form pass's first-level walk calls `for_each_clone_created`).** In `elidex-form/src/clone.rs`,
  not in `init.rs`: `apply_clone_form_state` takes the first level from the shared scope-gated walk and then runs the
  existing `collect_template_inclusive_descendants` on each `<template>` content fragment it found (the fixpoint stays
  where it is, unchanged). `init.rs` is not edited; `parse_into_inert_document`, the other caller of
  `collect_template_inclusive_descendants`, is untouched (§4.5).
- **D-4 (the no-dispatcher census becomes a standing check; brief item 4).** Yes, as a layering trip-wire with an
  allowlist, `.claude/tools/dispatcher-window-trip-wire.sh` (NEW) and `.claude/tools/dispatcher-window-allowlist.tsv`
  (NEW), registered in `scripts/trip-wires.sh` `REQUIRED_WIRES`, landing in this PR (§4.7). What it ratchets is the
  **population** of production sites that remove or replace `EcsDom`'s dispatcher, each with a one-word
  classification; a new site reds the PR until it is classified. What it cannot see (declared): whether a cloner entry
  is reachable inside a window — that is a judgment the classification records, re-made whenever the list changes.
  The reasons for a standing check rather than a one-shot: the population is derived from the code and shrinks or
  grows in silence; the hazard is fail-open (a `CloneCreated` fired in a window is dropped and both passes are lost,
  with no error); and RC-2b adds a second production caller of the entry, so the question is asked again at least
  once more. A loud `fire_clone_created` (an assertion that a dispatcher is installed) is rejected: an unbound
  `EcsDom` is a legitimate state (this crate's unit tests, Rust-only users), and `take_mutation_dispatcher` hands the
  box to the caller, so `EcsDom` cannot tell a window from "never installed" without a new guard type, which would
  be mechanism for a check a grep already gives.
- **D-5 (D14's rule).** The subtree flag is the operation's for the root pair and true for every other pair (§0.3
  item 6). Implemented as one local in the worklist loop, `let subtree = deep || d != cloned;`, used by pass 3
  (`replicate_template_contents`) and pass 4 (the shadow-child `clone_recording`). The `!deep` hand stamp
  `:294-301` is removed because `clone_subtree` with `doc_override = shadow_doc` stamps the child (and its subtree)
  itself (`tree_clone.rs:208-224`); the `:282-285` stamp stays (H9, RC-2a-first state). If C0b has landed at
  implementation time, both are already gone and RC-2a writes none (H9, C0b-first state); the PR body records which
  state its base was in.
- **D-6 (Attribute stub; umbrella §5.2a).** RC-2a's diff does not touch `CloneNode`'s `invoke` (`clone.rs:91-167`): the
  emitter is inside `clone_node_with_shadow_honor` and the D14 edit is in its worklist loop. So the Attribute clone
  is not folded in; `#11-attr-clone-node` stays as proposed in the umbrella's §8 (owner L3; fold candidate
  `#11-dom-api-attr-entity-model-unification`). The ProcessingInstruction stub stays with the proposed
  `#11-processing-instruction-clone`.
- **D-7 (scope in the event, not derived at dispatch).** `CloneScope` travels in the event (umbrella D-a″). A
  consumer cannot derive it: a `SelfOnly` root and a `Subtree` root with no light children look the same at dispatch
  time, and the difference decides whether the walk may enter light children that a later caller (RC-2b) appends.
- **D-8 (the census arm in `range/live.rs`).** RC-2a adds the `CloneCreated` no-op arm there (one production
  exhaustive match, §0.2). RC-1 also edits `range/live.rs` (`LiveRangeRef`); RC-2a lands first (umbrella §9), so RC-1
  rebases. If C0c lands first, RC-2a re-runs the census at its head and adds its arm beside C0c's `Adopt` arm (H6).

## §1. Shape and verdict

A terminal unit under an approved umbrella (CLAUDE.md "Edge-dense work", base case): one PR, this per-PR plan through
`/elidex-plan-review` before implementation. It is still edge-dense in itself (§2 enumerates nine invariants), which
is why the plan-review is mandatory and not a judgment. Coverage breadth K = 2 specs, M = 12 rows (§3): single-PR
scope.

Not in this PR: any Range file; the contents algorithm (RC-2b); the node document of clones (C0b / C1, umbrella §2
C3); the registry association of clones in a null-registry document (`#11-document-null-registry-adopt-reset`,
umbrella §9); the reaction queue's element-queue grouping and the per-method drain (`#11-ce-element-queue-grouping`,
`#11-ce-reactions-method-scope-drain`, umbrella E2 / §9).

## §2. Coupled invariants

Axes: **A** one emitter, **B** scope = created set, **C** re-entry contract, **D** lock discipline, **E** queue
position (umbrella E2), **F** detached read (#466 I3), **G** crate boundary (#466 I1), **H** D14 subtree per pair,
**I** no-dispatcher windows, **J** the variant census (H6).

- **A (one emitter, exactly once)**: every production clone goes through `clone_node_with_shadow_honor`, which fires
  `CloneCreated { root: cloned, scope }` once, after its worklist converges and before `Some(cloned)`. Its helpers
  (`clone_recording`, `replicate_template_contents`, the shadow loop) never fire. A caller cannot forget the event.
- **B (the visit set at dispatch is the created set)**: `Subtree` = today's walk from the root; `SelfOnly` = the root,
  then today's walk over its shadow root if any; never the root's light children. At the event a `SelfOnly` root has
  no light children (umbrella §0.3 item 26), so the rule and the tree agree; the rule stays in the walk so it does
  not depend on the caller's timing (RC-2b appends after the entry returns).
- **C (re-entry)**: both arms read the tree and write components only (`CustomElementState.is_value`;
  `FormControlState`, `ElementState`, `ClonedFrom` removal); no `EcsDom` mutation primitive runs inside dispatch
  (umbrella §0.3 item 27, re-checked in §0.2 "The form side"). A nested fire trips `dispatch_event`'s
  `debug_assert_eq!(dispatch_depth, 0)`.
- **D (locks)**: the CE arm takes the registry lock, runs the pass, drops it, then takes the queue lock; the form arm
  takes none. The same two locks in the same order as `handle_insert`; no new lock (umbrella §0.3 item 28).
- **E (queue position)**: `cloneNode`'s upgrades land at the same position in the flat FIFO as today: nothing between
  the entry's return and the old post-pass pushed to `ce_reaction_queue` or read `is_value` (umbrella §0.3 item 29;
  §0.2 `is_value` and `ce_reaction_queue` facts). The drain point is unchanged (checkpoints, E2).
- **F (detached read)**: the clone's `FormControlState` exists before the entry returns (dispatch is synchronous,
  inside the clone), which is earlier than #466's shim. `detachedInput.cloneNode().value` reads the copy.
- **G (crate boundary)**: the variant and `CloneScope` are in `elidex-ecs`; `elidex-dom-api` fires through an
  `EcsDom` helper; the copy stays in `elidex-form`, reached through the dispatcher; the linkage stays the dom-api
  `ClonedFrom` marker. No `elidex-dom-api` → `elidex-form` edge; `elidex-custom-elements` still depends on
  `elidex-ecs` only.
- **H (D14)**: the root pair takes the operation's `subtree`; every other pair is cloned with true (step 5, step
  6.7, template step 2); the `!deep` stamp is dead and removed (H9).
- **I (windows)**: no cloner-entry call is reachable while the dispatcher is taken or absent (the parser builds, the
  inert parse, teardown, unbound); the population is ratcheted by the trip-wire (D-4).
- **J (census)**: every exhaustive `match` on `MutationEvent` gains an arm (`range/live.rs`, the test recorder); the
  consumers gain real arms; C0c's `Adopt` meets this on the same enum and `match`es (H6).

Pairs (one line each; pairs not listed do not interact):

| Pair | Intersection |
|---|---|
| A × B | the event fires after the whole worklist, so for `Subtree` the full subtree (shadow trees and template contents included) exists at dispatch; for `SelfOnly` the root's shadow tree exists and its light children do not |
| A × E | one fire per entry call means one CE pass per root, at the clone's position; RC-2b's per-root calls put each root's upgrades between the `Disconnected` / `Connected` of the surrounding mutations (umbrella M × C) |
| A × I | the entry fires through `dispatch_event`, which takes the dispatcher; inside a window there is none and the event is dropped — the trip-wire ratchets the window population, and no window runs script or reaches the entry today (§5.3) |
| B × H | after D14 a `SelfOnly` root's shadow tree is a complete deep copy, so the `SelfOnly` walk over it reaches the grandchild custom elements D14 creates; before D14 those did not exist |
| B × F | the form arm's visit set is the same scope-gated first level plus the template frontier; a `SelfOnly` root cannot carry a `<template>` with contents (template step 1), but a `<template>` inside its shadow tree can (cloned with true after D14), and the frontier reaches it |
| C × D | the lock guards are dropped before any component write that the other consumer could read; neither consumer reads the other's components, so the form-before-CE dispatch order (field order) is unobservable |
| C × F | `create_form_control_state` at dispatch is the precedent `handle_insert` already sets; the clone's own internal `Insert` events (deep clone, template contents) run that same function before the sweep, and the sweep's overwrite is today's behaviour |
| E × C | the CE arm pushes during dispatch, not after `invoke_dom_api`; between those two points today nothing touched the queue (§0.2), so the position is unchanged and the earlier `is_value` clear is unobservable |
| G × F | the form copy still cannot be done by `elidex-dom-api` (it cannot see `FormControlState`); the event carries only `root` and `scope`, never pairs, so the marker remains the linkage (#466's I1 × I2 row) |
| H × J | D14 and the census touch different files (`node_methods/clone.rs`; `mutation_event.rs` and the match sites); the only shared file with another lane is `node_methods/clone.rs` with C0b (stamps, H9) and C1 (rebases, umbrella §9) |
| I × J | a window that takes the dispatcher also silences every other variant; the trip-wire's allowlist therefore classifies each window once for all variants, and C0c's `Adopt` inherits the same list |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §4.4 Interface Node | clone a node step 3 (run the cloning steps with node, copy, subtree) | input / textarea (form), template (pass 3), custom-element identity (pass 1) | `clone_node_with_shadow_honor` worklist (`node_methods/clone.rs:247-304`); the form arm at dispatch | ✓ | yes |
| WHATWG DOM §4.4 Interface Node | clone a node step 5 (children, subtree set to subtree) | subtree true / false | `clone_recording` for the root (`clone.rs:229`, `:313-327`) | ✓ | yes |
| WHATWG DOM §4.4 Interface Node | clone a node steps 6.4 and 6.7 (attach a shadow root with clonable true; clone each shadow child with subtree set to **true**) | clonable host / non-clonable host; root pair / nested pair | pass 4 (`clone.rs:266-303`); D14: the child clone passes true, pass 3 and pass 4 of every non-root pair pass true (D-5) | ✓ | yes |
| WHATWG DOM §4.4 Interface Node | clone a single node step 2.4 (create an element with the source's is value, synchronous false, and registry) | defined / undefined / null registry association | `propagate_ce_identity` (identity, `clone.rs:344-366`) then the CE arm at dispatch (registry half); registry steps 2.1-2.3 unchanged (slot, umbrella §9) | ✓ (steps 2.1-2.3 ✗, slot `#11-document-null-registry-adopt-reset`) | yes |
| WHATWG DOM §4.9 Interface Element | create an element steps 4.4, 5.2.1, 5.2.2 (customized built-in: keep is, enqueue upgrade; autonomous: null is value, enqueue upgrade) | customized built-in / autonomous / no definition / `is` mismatch | `apply_clone_creation_ce_semantics(dom, registry, root, scope)` (`elidex-custom-elements/src/upgrade.rs:125-156`), called by the CE arm | ✓ | no |
| WHATWG HTML §4.13.6 Custom element reactions | enqueue a custom element upgrade reaction steps 1-2 | — | the CE arm pushes `CustomElementReaction::Upgrade` per candidate at dispatch (`consumer.rs`, NEW arm) | ✗ (element-queue grouping and the drain point are the registered / proposed engine-wide slots, umbrella E2) | no |
| WHATWG HTML §4.10.5 The input element | cloning steps (propagate value, dirty value flag, checkedness, dirty checkedness flag, indeterminateness) | each field; dirty checkedness unmodeled | the form arm → `apply_clone_form_state(dom, root, scope)` → `copy_form_state` (`elidex-form/src/clone.rs:74-152`) | ✗ (dirty checkedness: `#11-input-dirty-checkedness-flag`, pre-existing, `clone.rs:122-126`) | yes |
| WHATWG HTML §4.10.11 The textarea element | cloning steps (propagate raw value and dirty value flag) | both fields | same | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | cloning steps 1-2 (subtree false → return; else clone the contents' children with subtree true into copy's contents) | root template, shallow / deep; template inside a cloned shadow tree | `replicate_template_contents` (`clone.rs:405-431`) with the per-pair flag (D-5) | ✓ | yes |
| WHATWG HTML §4.13.3 Core concepts | look up a custom element definition step 1 (null registry → null) | template contents (null-registry document) | the walk shape (today's `for_each_shadow_inclusive_descendant` does not enter template contents); interim of umbrella §4.3 condition 6, folded into `#11-document-null-registry-adopt-reset` | ✗ (slot) | no |
| WHATWG DOM §4.5 Interface Document | a document's custom element registry (null unless stated otherwise) | the inert template document | same interim; not modelled | ✗ (slot) | no |
| WHATWG DOM §4.8 Interface ShadowRoot | shadow-including descendant (the walk's order: node, light subtree, then shadow children) | `Subtree` / `SelfOnly` | `EcsDom::for_each_clone_created` (NEW, `walkers.rs`), on `for_each_shadow_inclusive_descendant` (`walkers.rs:54-80`) | ✓ | no |

Numbers and titles: `webref heading dom 4.4` (Interface Node), `dom 4.9` (Interface Element), `dom 4.5` (Interface
Document), `dom 4.8` (Interface ShadowRoot); `webref heading html 4.13.6` (Custom element reactions), `html 4.10.5`
(The input element), `html 4.10.11` (The textarea element), `html 4.12.3` (The template element), `html 4.13.3` (Core
concepts); also looked up and not cited by number: `html 4.13.5` = Upgrades, `html 4.13.4` = The CustomElementRegistry
interface. Dfns: `webref dfn dom 'cloning steps'` (§4.4, `#concept-node-clone-ext`), `'shadow-including descendant'`
(§4.8), `'custom element registry'` (for=Document, §4.5); `webref dfn html 'look up a custom element definition'`
(§4.13.3). Prose: `webref body dom concept-node-clone` (steps 3, 5, 6.4, 6.7), `clone-a-single-node` (steps 2.1-2.4,
7, 8), `concept-create-element` (steps 4.4, 5.2.1, 5.2.2); `webref body html the-template-element` (cloning steps 1-2),
`the-input-element` and `the-textarea-element` (cloning steps), `enqueue-a-custom-element-upgrade-reaction` (steps
1-2), `look-up-a-custom-element-definition` (step 1). All 2026-11-04. `webref coverage-map dom 4.4 dom 4.9 html 4.13.6
html 4.10.5 html 4.10.11 html 4.12.3 html 4.13.3 dom 4.5` → spec = 2, "ok → single PR scope". **Breadth: K = 2, M =
12**; single PR.

Step numbers are cited where `webref body` shows them today. Code comments cite the clone-a-node steps by number
(they have been stable: step 3 cloning steps, step 5 children, step 6.7 shadow children) and "create an element"'s
by name where the number is deep (the autonomous null-is rule).

### §3.1 User-input touch audit

- `Node.prototype.cloneNode` (`native_node_clone_node`, `node_methods_extras.rs:201-236`): user script reaches the
  entry with user-chosen `deep`; the form cloning steps propagate a user-typed value / toggled checkedness /
  indeterminateness onto the clone. Unchanged surface; the propagation now runs at dispatch instead of post-dispatch,
  with the same function.
- No other native is edited. The Range natives reach the entry only after RC-2b.

## §4. Mechanism

### §4.1 The event (`elidex-ecs`)

`crates/core/elidex-ecs/src/dom/mutation_event.rs` gains

```rust
/// Scope of a `CloneCreated` event: which of the root's descendants were
/// created by the same "clone a node" call (DOM §4.4 step 5: light children
/// only when subtree is true; step 6.7: shadow children always).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneScope { Subtree, SelfOnly }

/// A "clone a node" call completed: `root` and its `scope` were created.
/// Fired by `EcsDom::fire_clone_created` from the canonical cloner's entry,
/// after the whole copy exists and before the entry returns.
CloneCreated { root: Entity, scope: CloneScope },
```

`MutationEvent::CloneCreated` (NEW) and `CloneScope` (NEW) are re-exported beside `MutationEvent` (`dom/mod.rs:32`,
`lib.rs:44-47`). The variant borrows nothing, so the lifetime contract (`mutation_event.rs:34-39`) is untouched.
`EcsDom::fire_clone_created(&mut self, root: Entity, scope: CloneScope)` (NEW) sits beside `fire_split_text`
(`dom/mod.rs:231`) and does one thing: `self.dispatch_event(&MutationEvent::CloneCreated { root, scope })`. It inherits
`dispatch_event`'s take-and-restore and its `dispatch_depth` assertion. The module doc's variant list (`:45-106`),
the `Production impl` list (`:319-322`, which also gains the three consumers it omits today) and the slot doc
(`dom/mod.rs:58-66`, the fire-site list) name the new variant and its emitter (§4.9).

### §4.2 The emitter and D14 (`elidex-dom-api`)

In `clone_node_with_shadow_honor` (`clone.rs:227-306`):

- After the worklist loop and before `Some(cloned)` (`:305`):
  `dom.fire_clone_created(cloned, if deep { CloneScope::Subtree } else { CloneScope::SelfOnly });` (`CloneScope::Subtree` / `CloneScope::SelfOnly` (NEW), §4.1)
  One call, in the entry only; the helpers are unchanged in this respect.
- **D14 (D-5).** In the loop, `let subtree = deep || d != cloned;` replaces `deep` at the two pass sites: pass 3's
  `replicate_template_contents(s, d, dom, subtree, shadow_doc, &mut pairs)` (`:262`) and pass 4's shadow-child
  `clone_recording(dom, child, subtree, shadow_doc, &mut pairs)` (`:290`). Since `d != cloned` holds for every
  non-root pair, `subtree` is true for every shadow child (step 6.7) and for every template or host inside a shadow
  tree or inside template contents; for the root pair it is the operation's flag (step 5, template step 1). The
  `if !deep { … set_associated_document(child_clone, doc) }` block (`:294-301`) is deleted: `clone_recording` with
  `subtree = true` goes through `clone_subtree`, whose `doc_override = shadow_doc` stamps the child and threads the
  document into its subtree (`tree_clone.rs:199-224`); when `shadow_doc` is `None` neither path stamps, as today. The
  `:282-285` stamps stay (H9). The pass-4 doc (`:208-216`) and the step 6.7 comment (`:288-289`) are rewritten to
  the spec's rule.
- The `ClonedFrom` doc (`:24-75`) is rewritten where it names the VM shim: `:33-35` ("from the VM `cloneNode`
  marshalling shim, beside the CE clone-upgrade seam") → the consumer is `FormControlReconciler`'s `CloneCreated`
  arm, run at dispatch before the entry returns; `:48-57` ("Why a persisted marker, not a returned list") → the
  event carries only the root and the scope, so the src→dst link still has to survive from pass 2 to the arm, which
  is the marker's job; `:58-64` ("The sole production caller is the VM `cloneNode` shim") → a clone made with no
  dispatcher installed (this crate's unit tests, an unbound `EcsDom`) leaves the marker, which stays harmless
  (umbrella §6 row). `propagate_ce_identity`'s doc (`:334-340`, "The engine bindings run the registry-dependent half")
  names the CE arm instead.

Everything else in the file is unchanged: `CloneNode`'s `invoke` (so the Attribute / PI stubs stay, D-6), pass 1, pass
2's tag gate, `read_clonable_shadow_init`.

### §4.3 The shared walk (`elidex-ecs`)

`EcsDom::for_each_clone_created(&self, root: Entity, scope: CloneScope, visit: &mut F)` (NEW) in
`crates/core/elidex-ecs/src/dom/tree/walkers.rs`, beside `for_each_shadow_inclusive_descendant` (`:54`):

- `Subtree` → `self.for_each_shadow_inclusive_descendant(root, visit)`: today's order and cap, unchanged.
- `SelfOnly` → `visit(root)`; then, if `self.get_shadow_root(root)` is `Some(sr)`,
  `self.for_each_shadow_inclusive_descendant(sr, visit)`. That visits the ShadowRoot entity first (today's walk from
  the host does not, `walkers.rs:69-73`), then the same elements in the same order at the same depths; neither
  consumer acts on a ShadowRoot (umbrella §4.3 condition 1). The root's light children are never visited.

It lives in `elidex-ecs` because that is the one crate both consumers depend on. One walk, two consumers (umbrella
§4.3 condition 4; one issue, one way). The depth cap applies per root from depth 0, as `cloneNode`'s does today.

### §4.4 The CE arm (`elidex-custom-elements`)

- `apply_clone_creation_ce_semantics(dom, registry, clone_root, scope: CloneScope)` gains the `scope` argument and
  calls `dom.for_each_clone_created(clone_root, scope, …)` in place of its walk (`upgrade.rs:131`). Its contract
  doc (`:121-124`) becomes "on the root of a `CloneCreated` event, with that event's scope, during its dispatch". The
  four inline tests (`:391`, `:416`, `:427`, `:452`) pass `CloneScope::Subtree` (their trees are deep clones of a
  single element; one gains a `SelfOnly` sibling case, §6).
- `CustomElementReactionConsumer::handle` (`consumer.rs:76-95`) gains, before the wildcard,
  `MutationEvent::CloneCreated { root, scope } => self.handle_clone_created(root, scope, dom)`.
  `handle_clone_created(&mut self, root, scope, dom: &mut EcsDom)` (NEW): lock the registry; `let candidates =
  apply_clone_creation_ce_semantics(dom, &registry, root, scope);` drop the guard; if non-empty, lock the queue and
  push one `CustomElementReaction::Upgrade(entity)` per candidate in order. This is `handle_insert`'s shape
  (`:98-123`) with `&mut EcsDom`, which `handle` already receives. The poison policy is the consumer's `expect`
  (`:116`, `:120`), not the shim's `unwrap_or_else` (`vm/host/custom_elements/upgrade.rs:138`, `:149`): every
  insertion already takes the `expect` path, so this adds no failure mode (umbrella §0.3 item 28). The module doc
  (`:72-74`) and the gating-rules section name the new arm.

### §4.5 The form arm (`elidex-form`)

- `apply_clone_form_state(dom, clone_root, scope: CloneScope)` (`clone.rs:44-65`) gains `scope`. Its collection
  step (`:49`) becomes (D-3): first level = `dom.for_each_clone_created(clone_root, scope, |e| found.push(e))`;
  then, for each `e` in `found` with `dom.template_contents_fragment(e) == Some(fragment)`, extend `found` with
  `crate::init::collect_template_inclusive_descendants(dom, fragment)` (the existing fixpoint, `init.rs:111-125`,
  which already handles nested templates). The marker sweep and `copy_form_state` are unchanged. A `SelfOnly`
  `<template>` root has an empty fresh fragment (template cloning steps step 1; `replicate_template_contents`
  returns at `:423-425`), so the frontier finds nothing under it; a `<template>` inside the root's shadow tree has
  contents after D14 and the frontier reaches them. The module doc (`:16-18`) names the arm.
- `FormControlReconciler::handle` (`reconciler.rs:43-52`) gains, before the wildcard,
  `MutationEvent::CloneCreated { root, scope } => handle_clone_created(root, scope, dom)`, a free fn
  `handle_clone_created` (NEW) beside `handle_insert` that calls `crate::clone::apply_clone_form_state(dom, root, scope)`.
  The module doc (`:4-8`) names it. The six inline tests in `clone.rs` (`:195` … `:297`) pass `CloneScope::Subtree`;
  one gains a `SelfOnly` case (§6).
- Order between the arms: `form_control` is dispatched before `custom_elements` (`consumer_dispatcher.rs:155`, `:158`),
  the reverse of the shim; the two write disjoint components and neither reads the other's (§2 C × D).

### §4.6 The VM (`elidex-js`): deletion only

- `native_node_clone_node` (`node_methods_extras.rs:201-236`) loses `:214-234`; what remains is entity extraction,
  `deep` coercion, `invoke_dom_api`, `install_document_methods_if_cloned_doc`, return. Its doc (`:187-190`) names
  the consumer arms instead of the post-pass.
- `enqueue_subtree_upgrade_reactions` (`vm/host/custom_elements/upgrade.rs:128-156`) and its doc (`:117-127`) are
  deleted. Nothing else references it (§0.2 grep).
- `consumer_dispatcher.rs` field docs: `form_control` (`:50-57`) also runs the cloning steps on `CloneCreated`;
  `custom_elements` (`:78-87`) also writes `CustomElementState.is_value` on `CloneCreated`, which no other consumer
  reads, so both stay order-independent. The bind comment (`vm_api.rs:296-299`) is unchanged (it lists field order).

`vm/host/` ends with no clone step at all (umbrella X1). The `cloneNode` native is marshalling only.

### §4.7 The no-dispatcher windows and the trip-wire (D-4)

**The population at `fab44a1d`** (§0.2, 19 production lines; §5.3 lists each with its disposition). No window runs
script, and the entry is reached only from `CloneNode`'s `invoke` (the caller census in §0.2), which runs only from a VM native with a
bound VM (dispatcher installed) or from a Rust caller of the dom-api handler. The parser crates create nodes through
`EcsDom` constructors and never call the entry; `despawn_subtree` creates nothing; `parse_into_inert_document` parses
and attaches components only (`inert_document.rs:112-199`). So today no clone happens inside any window.

**The wire** `.claude/tools/dispatcher-window-trip-wire.sh` (NEW), registered in `scripts/trip-wires.sh`
`REQUIRED_WIRES` (`:85-91`), with `.claude/tools/dispatcher-window-allowlist.tsv` (NEW), on the
`layout-box-reader-trip-wire.sh` pattern (its allowlist is `.claude/tools/layout-box-reader-allowlist.tsv`):

- Population command: the umbrella's §0.3 item 34 `git grep -n -P` over `crates ':!*tests*'`, extended to every write
  of the private field (`\bdispatcher\s*=\s*|\bdispatcher\.(take|replace|insert|get_or_insert)\(`), so a new writer
  inside `elidex-ecs` and a new caller outside it both land in the population. Inline `#[cfg(test)]` modules are
  excluded by a second grep on the file's `#[cfg(test)]` line position (a hit below that line is a test hit), so the
  list holds production lines only.
- Allowlist row: `<classification>\t<path>\t<content>`; classification ∈ {`slot-def` (the four methods and the
  constructor / `dispatch_event` take), `bind` (install / clear at `Vm::bind` / `Vm::unbind`), `window:<reason>`
  (a take-and-restore span; the reason names why no cloner entry is reachable inside it: `parse`, `teardown`,
  `inert-parse`)}.
- Fails on an added row and on a removed row alike (the allowlist cannot rot), on an unknown classification, and
  runs `--regenerate` to rewrite the allowlist keeping classifications by path + content. Its header states the
  blind spot: it bounds the population of windows, not the reachability of a cloner call inside one; that is the
  classification's claim, re-made when the row is added.
- Self-test: the script's `--self-test` builds a fixture with one extra `take_mutation_dispatcher()` line and
  asserts red, and one with a row removed and asserts red, so an emptied allowlist cannot pass green.

Why not a test: a Rust test cannot see call sites; the property is a source population. Why not nothing: D-4.

### §4.8 The #466 I2 / I3 reversal (review venue, umbrella R6-3)

**What #466 did** (`docs/plans/2026-07-clone-cloning-steps-fcs.md`, landed `7bafcb9f`). It implemented the HTML
cloning steps of `<input>` / `<textarea>` under five invariants (I1-I5, `:52-56`): the copy runs in `elidex-form`
(I1); it **must not** be a `MutationEvent`, because `crates/core/elidex-ecs/src/dom/` was off-limits to the lane (I2,
`:53`); it must be synchronous at clone time, "not a deferred event consumed on insertion" (I3, `:54`); it creates the
clone's `FormControlState` so the insert-time absence guard skips it (I4); and it covers the shadow-inclusive,
template-inclusive clone (I5). The mechanism: the dom-api cloner attaches a `ClonedFrom` marker per `<input>` /
`<textarea>` clone, and the VM `cloneNode` shim calls `elidex_form::apply_clone_form_state` after `invoke_dom_api`
returns (`node_methods_extras.rs:229-231`), beside the #331 CE seam (`:221`). Its conclusion (`:73-79`) rejected the
#331 §6.1 sketch's `MutationEvent::Clone { pairs }` because "I2 rules out the event; I3 rules out 'deferred'".

**What this PR does instead.** The same copy, the same marker, the same function, run by `FormControlReconciler`'s
`CloneCreated` arm at dispatch, inside the cloner, before the entry returns (§4.5). The CE seam takes the same path.

**Why I2 and I3 do not bind** (the umbrella's reasons, §4.3 "#466 reversal", are the authority; this is the slice
view): I2 was a lane boundary of its time, not an engine invariant — the enum's own design invites a variant
(`mutation_event.rs:8-12`), Layout's C0c adds `Adopt` to it, and what I2 protected in substance (I1, the crate
boundary) still holds (§2 G). I3 excluded a *deferred* consumer; this consumer is synchronous and runs earlier than
#466's shim (§2 F). I4 and I5 hold unchanged: the same function creates the state at clone time, over the same tree
(§2 B × F; `cloneNode(true)` is `Subtree`, today's walk; the template frontier stays).

**What flips.** No #466 test changes its expected value: the eleven VM tests in `tests_clone_form_state.rs` (`:48-180`,
the I3 case among them) and the six `elidex-form` unit tests (`clone.rs:184-304`) stay green; the unit tests gain a
`scope` argument (signature only). The `elidex-form` module doc, the `tree_clone.rs:28` row ("there is no
`MutationEvent::Clone`") and `tests_clone_form_state.rs`'s module doc (`:7-9`, "through the real VM `cloneNode`
marshalling shim") are rewritten to name the arm. The only test whose expected value flips in this PR is D14's
(§6). `#11-clone-cloning-steps-event` stays closed: the defect it fixed stays fixed, and the PM may note on the closed
entry that the mechanism is now the event (umbrella H5).

**What the reviewer is asked to judge here**: that §2 F and G hold on the slice's code (not only in the umbrella's
prose), that the queue position (§2 E) is unchanged, and that nothing of #466's five invariants is weakened by moving
the call from the shim to the arm.

### §4.9 Hygiene (umbrella §6 rows owned by RC-2a) and citations

The umbrella's §6 RC-2a rows, applied here: `node_methods/clone.rs:58-64` and `:33-35`, `:48-57` (§4.2);
`tree_clone.rs:28` (the `FormControlState` row: the cloning steps run from `FormControlReconciler`'s
`MutationEvent::CloneCreated` arm, dispatched before the cloner entry returns; `#11-clone-cloning-steps-event` stays
closed) and `:29` ("within the *same* synchronous clone operation" stays true); `mutation_event.rs:1`, `:45-106`,
`:319-322`; `consumer.rs:72-74`; `reconciler.rs:4-8`; `consumer_dispatcher.rs:50-57`, `:78-87`;
`custom_elements/upgrade.rs:117-127` and `node_methods_extras.rs:187-190`, `:214-234`, `elidex-form/src/clone.rs:16-18`
(deleted or re-pointed); `node_methods/clone.rs:208-216`, `:288-289` (the D14 rule); plus `clone_shadow.rs:9-14`
(the test file header states the wrong step 6.7 rule) and `propagate_ce_identity`'s doc (`:334-340`).

Citation rule (umbrella §6): on the files this PR edits, every `§` number ↔ title pair and step number is verified
with the umbrella's five patterns and the step grep, and the PR body records each pattern's hit count and
disposition. Known at `fab44a1d`: the files cite DOM §4.4, §4.8, §4.9, §4.2.3, §4.10, §4.11, §4.3.2, §4.3.3, §5.5,
§6.1 and HTML §4.13.6, §4.13.5, §4.13.4, §4.10.5, §4.10.11, §4.12.3, §4.10.18.3, §2.4.3, §4.2.3, §8.1.8.1, §4.12.5;
those this memo depends on are verified in §3; the rest are verified in the PR's sweep (truth first, then the
patterns), and a number webref cannot confirm is replaced by the name.

### §4.10 ECS-native and layering checks

- **ECS-native (umbrella X2, §4.4).** One event variant on the existing stream; no component, no system, no
  entity-keyed side-store, no reaction variant, no subscriber list. The consumers write existing components. The
  OO notion "run the cloning steps / create an element for each copy" maps to an event-as-data consumer pass (the
  `handle_insert` idiom); `CloneScope` is event data, not state.
- **Layering (umbrella X1; CLAUDE.md mandate).** The algorithm pieces live in `elidex-ecs` (variant, helper, walk),
  `elidex-dom-api` (emitter, D14), `elidex-custom-elements` and `elidex-form` (the passes). `vm/host/` loses code
  and gains none.
- **One issue, one way.** One clone path for `cloneNode` and the Range natives; one walk for both arms; one
  recorder in the `elidex-ecs` tests; the subtree flag by one rule. The separate post-clone seam is retired, not
  kept beside the event.
- **Null-registry documents (umbrella §5 preamble, R6-1).** The diff touches neither `propagate_ce_identity`'s
  registry copy (`clone.rs:359-363`) nor `prepare_upgrade`'s `Null` skip (`upgrade.rs:74-76`); the early upgrade
  inside `document.cloneNode(true)` stays the pre-existing defect of `#11-document-null-registry-adopt-reset` and is
  not tested as a target.

## §5. Populations

### §5.1 What the PR edits and adds (`wc -l` at `fab44a1d`)

`wc -l crates/dom/elidex-dom-api/src/node_methods/clone.rs crates/core/elidex-ecs/src/dom/mutation_event.rs crates/core/elidex-ecs/src/dom/mod.rs crates/core/elidex-ecs/src/lib.rs crates/core/elidex-ecs/src/dom/tree/walkers.rs crates/core/elidex-ecs/src/dom/tree_clone.rs crates/dom/elidex-custom-elements/src/consumer.rs crates/dom/elidex-custom-elements/src/upgrade.rs crates/dom/elidex-form/src/reconciler.rs crates/dom/elidex-form/src/clone.rs crates/script/elidex-js/src/vm/consumer_dispatcher.rs crates/script/elidex-js/src/vm/host/node_methods_extras.rs crates/script/elidex-js/src/vm/host/custom_elements/upgrade.rs crates/dom/elidex-dom-api/src/range/live.rs crates/dom/elidex-dom-api/src/node_methods/tests/clone_shadow.rs crates/dom/elidex-dom-api/src/test_util.rs crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs crates/core/elidex-ecs/src/dom/tests/dispatch_guard.rs crates/core/elidex-ecs/src/dom/tests/traversal.rs`
(run 2026-11-04):

| File | Lines | Change |
|---|---|---|
| `crates/dom/elidex-dom-api/src/node_methods/clone.rs` | 458 | the fire; D14 (per-pair flag, `:294-301` deleted); docs |
| `crates/core/elidex-ecs/src/dom/mutation_event.rs` | 352 | `CloneCreated`, `CloneScope`; docs |
| `crates/core/elidex-ecs/src/dom/mod.rs` | 926 | `fire_clone_created`; re-export; slot doc |
| `crates/core/elidex-ecs/src/lib.rs` | 51 | re-export |
| `crates/core/elidex-ecs/src/dom/tree/walkers.rs` | 392 | `for_each_clone_created` |
| `crates/core/elidex-ecs/src/dom/tree_clone.rs` | 425 | doc rows `:28-29` only |
| `crates/dom/elidex-custom-elements/src/consumer.rs` | 329 | the arm; docs |
| `crates/dom/elidex-custom-elements/src/upgrade.rs` | 526 | `scope` argument on the shared walk; contract doc; inline tests |
| `crates/dom/elidex-form/src/reconciler.rs` | 333 | the arm; module doc |
| `crates/dom/elidex-form/src/clone.rs` | 304 | `scope` argument, first level on the shared walk (D-3); module doc; inline tests |
| `crates/script/elidex-js/src/vm/consumer_dispatcher.rs` | 160 | field docs |
| `crates/script/elidex-js/src/vm/host/node_methods_extras.rs` | 293 | post-pass deleted; doc |
| `crates/script/elidex-js/src/vm/host/custom_elements/upgrade.rs` | 487 | `enqueue_subtree_upgrade_reactions` deleted |
| `crates/dom/elidex-dom-api/src/range/live.rs` | 500 | the census no-op arm |
| `crates/dom/elidex-dom-api/src/node_methods/tests/clone_shadow.rs` | 277 | D14 inversion; stamp-guard extension; event-count and scope tests; header |
| `crates/dom/elidex-dom-api/src/test_util.rs` | 49 | `CloneCreatedProbe` (NEW) test dispatcher recording `(root, scope)` |
| `crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs` | 971 | `MockEvent` variant + recorder arm; `pub(super)` on the recorder (D-1) |
| `crates/core/elidex-ecs/src/dom/tests/mutation_hook_clone_created.rs` (NEW) | — | recorder tests for `fire_clone_created` |
| `crates/core/elidex-ecs/src/dom/tests/dispatch_guard.rs` | 69 | the nested-fire assertion test |
| `crates/core/elidex-ecs/src/dom/tests/traversal.rs` | 413 | `for_each_clone_created` unit tests |
| `crates/core/elidex-ecs/src/dom/tests/mod.rs` | 28 | `mod mutation_hook_clone_created;` |
| `crates/script/elidex-js/src/vm/tests/tests_clone_form_state.rs`, `tests_custom_elements.rs`, `tests_create_element_options.rs` | — | module doc; new end-to-end cases (§6) |
| `scripts/trip-wires.sh`; `.claude/tools/dispatcher-window-trip-wire.sh` (NEW); `.claude/tools/dispatcher-window-allowlist.tsv` (NEW) | — | D-4 |

Files not touched, stated because the umbrella listed them as possible: `crates/dom/elidex-form/src/init.rs` (D-3),
`CloneNode`'s `invoke` (D-6), every Range file, `reaction.rs`, `custom_elements/flush.rs`.

### §5.2 Exhaustive `MutationEvent` matches gaining an arm

From §0.2 (command: `git grep -l 'MutationEvent::' -- crates ':!*tests*'`, 31 files, each `match` inspected for a
wildcard): `crates/dom/elidex-dom-api/src/range/live.rs:113-163` (no-op arm, the bridge ignores the event, umbrella
L × C); the test recorder `crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs:68-148` (records it). The two
consumer matches gain real arms (§4.4, §4.5). All other matches have a wildcard or are `if let` / `let … else`.
Re-run at the implementation head, and again if C0c lands first (H6).

### §5.3 No-dispatcher windows: the 19 production lines and their dispositions

| Site | Lines | Classification | Why no cloner entry is reachable inside |
|---|---|---|---|
| `crates/core/elidex-ecs/src/dom/mod.rs` | `:167`, `:171`, `:175`, `:176`, `:181`, `:184` | `slot-def` | the slot's own methods (definitions) |
| `crates/core/elidex-ecs/src/dom/mod.rs` | `:207` | `slot-def` | `dispatch_event`'s take-and-restore; a fire inside it is a nested dispatch, caught by `:200-206` |
| `crates/core/elidex-ecs/src/dom/tree/teardown.rs` | `:209` take, `:240` restore | `window:teardown` | `despawn_subtree`: despawns, creates nothing |
| `crates/core/elidex-ecs/src/dom/tree/teardown.rs` | `:293` definition, `:300` take | `window:parse` | `begin_detached_fragment`: definition and its take; the parse runs in the callers below |
| `crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs` | `:180` begin, `:241` restore | `window:parse` | the strict fragment build creates nodes through `EcsDom` constructors; `elidex-html-parser-strict` does not depend on `elidex-dom-api` |
| `crates/dom/elidex-html-parser/src/lib.rs` | `:143` begin, `:162` restore | `window:parse` | the html5ever fragment build, same |
| `crates/dom/elidex-form/src/inert_document.rs` | `:112` take, `:199` restore | `window:inert-parse` | DOMParser's parse plus component attach; no clone call (`:112-199`) |
| `crates/script/elidex-js/src/vm/vm_api.rs` | `:324` | `bind` | install at bind |
| `crates/script/elidex-js/src/vm/lifecycle.rs` | `:34` | `bind` | clear at unbind; after it the VM is unbound and runs no native |

Line counts: 7 + 2 + 2 + 2 + 2 + 2 + 1 + 1 = 19. The remaining 6 of the command's 25 lines are the 5 inline-test
hits and the comment listed in §0.2.

## §6. Test and coverage plan

**Flips (expected value changes).** One: `clone_node_shallow_replicates_own_clonable_shadow_shallowly`
(`clone_shadow.rs:76-112`) is inverted and renamed `clone_node_shallow_replicates_own_clonable_shadow_deeply`: the
shadow child's own descendants **are** cloned (step 6.7, subtree true). Its red run at the pre-fix head is D14's
negative control (umbrella §0.4: measured, no green branch). The header comment `:9-14` is rewritten.

**Signature-only changes.** `apply_clone_creation_ce_semantics` tests (`upgrade.rs:391`, `:416`, `:427`, `:452`) and
`apply_clone_form_state` tests (`clone.rs:195`, `:215`, `:247`, `:269`, `:285`, `:297`) pass `CloneScope::Subtree`.

**Stay green, unchanged** (the "cloneNode as today" guard, umbrella §5.2a): `tests_clone_form_state.rs` (11),
`tests_custom_elements.rs:567-587`, `tests_create_element_options.rs:219-` (the three clone cases), every test in
`node_methods/tests/clone.rs` (720 lines; among them the `ClonedFrom` marker tests `:72-134`, the template tests
`:135-245`, the CE identity tests `:471-595`, the null-registry tests `:618-687`), the rest of `clone_shadow.rs`
(the stamp guards `:186-214` and `:244-277` among them).

**New tests.**

- `elidex-ecs`, `tests/mutation_hook_clone_created.rs` (NEW): `fire_clone_created` records
  `MockEvent::CloneCreated { root, scope }` once, for both scopes; with no dispatcher installed it is a no-op (no
  panic, nothing recorded afterwards).
- `elidex-ecs`, `tests/dispatch_guard.rs`: a hook that calls `dom.fire_clone_created(…)` from inside `dispatch`
  panics under `#[cfg(debug_assertions)]` (`#[should_panic(expected = "re-entry contract")]`, matching the message at
  `dom/mod.rs:200-206`).
- `elidex-ecs`, `tests/traversal.rs`: `for_each_clone_created` with `SelfOnly` on a root that has light children and a
  clonable shadow root visits the root, then the ShadowRoot entity and its subtree, never the light children; with
  `Subtree` it equals `for_each_shadow_inclusive_descendant`; with `SelfOnly` on a root without a shadow root it
  visits the root only.
- `elidex-dom-api`, `test_util.rs`: `CloneCreatedProbe` (NEW), a `MutationDispatcher` recording `(root, scope)`
  (the `AttrChangeCounter` shape, `test_util.rs:21-34`).
- `elidex-dom-api`, `tests/clone_shadow.rs`: (a) one event per entry call: `cloneNode(true)` of a host with a
  clonable shadow root holding a nested clonable host and a `<template>` records exactly one `CloneCreated`, with
  `root == cloned` and `Subtree`; `cloneNode(false)` records one with `SelfOnly`; (b) **D14 extended**: a
  `<template>` with contents inside the shadow tree of a `cloneNode(false)` root has its contents cloned (per-pair
  flag), and a nested clonable host inside that shadow tree has its own shadow children deep-cloned; (c) the stamp
  guard `clone_node_shallow_shadow_children_carry_owner_document` is extended to the shadow child's grandchild
  (`owner_document == Some(doc)`), and stays green in the C0b-first state too (H9), because it reads
  `owner_document` and expects the source's document; (d) the event fires after the whole copy exists: the probe
  reads, at dispatch, that the recorded root already has its shadow root and (for `Subtree`) its light children.
- `elidex-custom-elements`, `upgrade.rs` inline tests: `apply_clone_creation_ce_semantics` with `SelfOnly` on a
  root with a light child custom element and a shadow child custom element returns the shadow one only; the
  template-contents rule: a `<template>` root with a defined custom element in its contents yields no candidate
  (umbrella §4.3 condition 6 pin).
- `elidex-form`, `clone.rs` inline tests: `apply_clone_form_state` with `SelfOnly` copies the shadow-encapsulated
  `<input>`'s marker and not a light-child `<input>`'s (the light child's marker is left, as a later root's or a
  moved original's would be).
- `elidex-js` end-to-end: (e) `host.cloneNode(false)` with a defined custom element at grandchild depth inside a
  clonable shadow tree runs its constructor (the CE check on the D14 control; a `SelfOnly` root whose shadow tree
  the walk visits); (f) `cloneNode(true)` and `cloneNode(false)` of a defined custom element each enqueue one upgrade
  (count = 2 with the original, as `tests_create_element_options.rs:219-237` already pins for `false`; the `true`
  twin is added); (g) a cloned `<template>` whose contents hold a defined custom element enqueues no upgrade for the
  content clone; (h) queue position (E2): `x.cloneNode(true)` followed by `document.body.append(y)` in one script
  runs x's clone constructor before y's `connectedCallback` — an order equal under both queue models (umbrella E2
  rule), read at the eval checkpoint; (i) the I3 case stays as is (`tests_clone_form_state.rs:48-56`).
- The trip-wire's `--self-test` (two red fixtures) runs under `mise run trip-wires` with the other wires.

**Coverage statement (CLAUDE.md "Supported-surface testing").** The `cloneNode` surface is guarded by the VM
end-to-end tests above plus the engine-independent unit tests in the three crates; D14 by its negative control; the
mechanism by the recorder and probe tests. No WPT subset is taken on by this PR (the Range program's WPT choice is
RC-2b's).

## §7. 1000-line audit (CLAUDE.md "1000-line debt")

Measured with `wc -l` at `fab44a1d` (§5.1 command, 2026-11-04). No file RC-2a touches is at or over 1000 lines:
the largest are `crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs` (971, test; gains about 10 lines for the
recorder arm and variant, and nothing else — RC-2a's recorder tests go to the new sibling, D-1) and
`crates/core/elidex-ecs/src/dom/mod.rs` (926; gains about 20). No prerequisite split. Re-measure rule: at
implementation start, `wc -l` the two files above at the actual base; if C0b has landed and `dom/mod.rs` with RC-2a's
addition would reach 1000, a standalone pure-move split (its mutation-dispatcher section, `:160-274`, to
`dom/dispatch.rs` (NEW), is the cohesion seam) lands first. `range/live.rs` is 500 after RC-0a. The umbrella's §7
rows for RC-2a are thereby discharged at this base.

## §8. Out of scope, and slots

- `#11-attr-clone-node` (proposed, umbrella §8): not folded in (D-6).
- `#11-processing-instruction-clone` (proposed, umbrella §8): the stub stays.
- `#11-document-null-registry-adopt-reset` (proposed, umbrella §9): the walk-shape interim is kept unchanged; no new
  divergence (§4.10).
- `#11-ce-element-queue-grouping` (registered) and `#11-ce-reactions-method-scope-drain` (proposed): the queue model
  and the drain point are untouched; the new end-to-end order test asserts only an order equal under both models.
- `#11-input-dirty-checkedness-flag` (pre-existing, `elidex-form/src/clone.rs:122-126`): untouched.
- `#11-clone-cloning-steps-event` (closed by #466): stays closed; the PM may annotate it (umbrella H5).
- The node document of clones and the `:282-285` stamps: C0b's (H9); C1 rebases onto this PR (umbrella §9).
- No new slot is proposed by this memo.

## §9. Edges, hand-offs, definition of done

**Edges.**

- **RC-0a (landed, `fab44a1d`)**: this memo cites the split files.
- **C0b (Layout, unlanded)** on `node_methods/clone.rs`: H9 decides which stamps each side removes; at `fab44a1d`
  the RC-2a-first state applies (D-5). The PR body records the state of its base.
- **C0c (Layout, unlanded)** on `MutationEvent`: H6; whichever lands second re-runs §5.2 and adds its arm beside
  the other's. The trip-wire's allowlist is shared (§2 I × J): if C0c adds a window it adds a row.
- **C1 (Layout)**: depends on all of RC-2a and rebases onto it (umbrella §9). Layout is told at landing (umbrella H2).
- **RC-1**: rebases its `range/live.rs` edit onto the census arm (D-8).
- **RC-2b**: consumes the event; calls the entry once per clone root; the trip-wire's population gains no row from it
  (it adds a caller of the entry, not a window), and its per-PR plan re-states the reachability claim for the new
  caller.

**Hand-offs** (for the PM; the memo edits neither the ledger nor Layout's files): close
`#11-clone-shadow-children-subtree` at landing with the PR number and squash SHA; optionally annotate the closed
`#11-clone-cloning-steps-event` (§4.8); tell Layout that RC-2a has landed (H2) and that the `dispatcher-window`
allowlist exists for C0c (H6).

**Definition of done** (every command re-runnable at the PR head):

1. D14 negative control: `cargo test -p elidex-dom-api --all-features clone_node_shallow_replicates_own_clonable_shadow`
   is red at the pre-fix head with the inverted assertion and green at the PR head (the PR body records both runs).
2. One clone path: `git grep -n 'enqueue_subtree_upgrade_reactions\|apply_clone_creation_ce_semantics\|apply_clone_form_state' -- crates/script/elidex-js/src ':!*tests*'`
   returns nothing (today 5 hits, §0.2).
3. One emitter: `git grep -n 'fire_clone_created(' -- crates ':!*tests*'` shows the definition in `dom/mod.rs` and one
   call, in `clone_node_with_shadow_honor`; the definition's body calls `dispatch_event`.
4. The variant's sites: `git grep -n 'MutationEvent::CloneCreated' -- crates ':!*tests*'` shows the helper, the CE arm,
   the form arm and the `range/live.rs` no-op arm, nothing else (plus the recorder under `tests/`).
5. One walk: `git grep -n 'for_each_clone_created' -- crates ':!*tests*'` shows the definition and its calls in
   exactly two files, `elidex-custom-elements/src/upgrade.rs` and `elidex-form/src/clone.rs`, one call each (verified at the head by this grep); neither file calls
   `for_each_shadow_inclusive_descendant` on the clone root (`git grep -n 'for_each_shadow_inclusive_descendant' --
   crates/dom/elidex-custom-elements/src/upgrade.rs crates/dom/elidex-form/src/clone.rs` → nothing).
6. Cloner census: `git grep -n 'clone_node_with_shadow_honor\|"cloneNode"' -- crates` matches §0.2's list (one entry,
   one production caller).
7. Census of exhaustive matches: §5.2 re-run at the head (`cargo build --workspace --all-features` and
   `cargo test -p elidex-ecs` compile every match).
8. Windows: `bash scripts/trip-wires.sh` green, with `dispatcher-window-trip-wire.sh` in `REQUIRED_WIRES`, its
   allowlist holding exactly the §5.3 rows (plus any row a landed C0 part added, classified), and its `--self-test`
   red on both fixtures.
9. `cargo test -p elidex-ecs -p elidex-dom-api -p elidex-custom-elements -p elidex-form -p elidex-js --all-features`
   green; the new tests of §6 present; `mise run ci` green before push.
10. The hand-stamp state of the base recorded in the PR body (H9); the stamp guards green.
11. The citation sweep (umbrella §6) run on the files of §5.1, hit counts and dispositions in the PR body.
12. The #466 reversal reviewed in this plan's `/elidex-plan-review` and the outcome recorded in §10 of this memo
    (umbrella R6-3).

## §10. Open questions

- **Q-A (for the plan-review, not the PM).** Whether the trip-wire (D-4) belongs in this PR or in a standalone
  CI/tools PR before it. Recommendation: this PR — it is RC-2a's own DoD check, it is small, and the trip-wires job
  needs no setup step (CLAUDE.md "CI"). If the review prefers the CI/tools lane to own every wire, it becomes a
  prerequisite PR and RC-2a's DoD item 8 waits for it.
- No PM question. Every item the umbrella left to this plan is decided in §0.4.
