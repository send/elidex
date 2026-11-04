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

**Rev 2** (2026-11-04) — R1 dispositions (F1-F13). **F1** (structural): the source-grep trip-wire is dropped; D-4 is a
runtime property check in `elidex-ecs` (a `Taken` state on the dispatcher slot plus a `debug_assert!` in
`fire_clone_created`) — §0.2 census (one place, with its command and output), §0.4 D-4, §2 I / A × I / I × J, §4.1,
§4.7, §5.1, §5.3 (removed), §6, §9 edges / hand-offs / DoD 8, §10 Q-A. **F2** `tests_custom_elements.rs` (1320) is not
touched; the end-to-end cases go to a new VM test file — §5.1 (every file re-measured), §6, §7. **F3** walk order =
"clone a node" creation order, visit set = "shadow-including descendant" — §3, §4.3. **F4** the two pass functions
become `pub(crate)`, re-exports removed — §4.4, §4.5, §5.1, DoD 2. **F5** #466 reversal: I3's precondition, F × I,
"marker over pairs" re-grounded, residual — §2, §4.2, §4.8. **F6** D-7. **F7** stale-prose census — §0.2, §4.9, §6.
**F8** C × F, A × B. **F9** §4.5. **F10** §4.1 docstring, §3 steps, `:150`, §4.9 sweep counts and drifts. **F11**
slot statuses (§8), C0c overlap (§9), D-1, D-5. **F12** §1 count, §6 WPT, D-1 / §7 cohesion. **F13** §4.10 table.

**Rev 3** (2026-11-04) — R2 dispositions (G1-G14; one IMP, G1, in Rev 2's own additions). **G1** the slot census is
defined by the property (every access to the private `dispatcher` field), not by the slot's method names; the reader
`teardown.rs:80` enters §5.1 and every reader's `Taken` mapping is stated — §0.2 "Slot access census", §2 I, §4.7
"The readers", §4.10, §5.1. **G2** take-without-restore: the owed restore or `clear`, `clear` from `Taken`, the two
existing tests, the bounded blind spot — §4.7. **G3** generic assertion message — §4.7, §6. **G4** §2 C × I. **G5**
§7's population is the files this PR edits. **G6** DoD 8 is decidable. **G7** the recorder tests join the `fire_*`
group; the measured count (about 1004) crosses 1000, so a standalone pure-move split (RC-2a-pre) comes first — D-1,
D-2, §1, §5.1, §6, §7, §9. **G8** D-6 slot statuses. **G9** C0b's edge — D-2, §9. **G10** H9 by reference only —
§2 H, §4.2. **G11** §4.8 residual: clones below the cap, `handle_insert`'s cap. **G12** §4.10 table. **G13**
citations — §0.2 (cloner callers), §0.3 item 5, §3, §4.1, §4.3, §4.5, §4.9. **G14** `define.rs:204` / `:208` —
§0.2. Every census and universal claim kept or added in this revision names its population by the property checked,
the command run at `fab44a1d` with its output, and the complement (what the command would miss and why that is
empty or excluded).

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
  `dispatcher` slot `:58-66` (`Option<Box<dyn MutationDispatcher + Send + Sync>>`, `:66`) and `dispatch_depth: u8`
  (`:67-73`); `RestoreDispatcher` (doc `:99-114`, struct `:115-118`, `Drop` `:120-130`: `debug_assert!` that the slot is
  `None` `:122-126`, the direct write `self.dom.dispatcher = self.pending.take()` `:127`, the depth decrement `:128`);
  `new()` `:134-144` (`dispatcher: None` `:138`); `set_mutation_dispatcher` `:167-172` (`replace`, returns the
  displaced box), `clear_mutation_dispatcher` `:175-177`, `take_mutation_dispatcher` `:181-185` (`take`, returns
  `None` when nothing is installed); the private `fn dispatch_event` `:199-220` (`debug_assert_eq!(dispatch_depth, 0)`
  `:200-206`, its own `self.dispatcher.take()` `:207`, not the public method); `fire_split_text` `:231-247` (calls
  `dispatch_event` at `:246`); `fire_normalize_merge` `:258-274`. `crates/core/elidex-ecs/src/lib.rs:44-47` re-exports
  `MutationDispatcher` / `MutationEvent`. Every window caller restores with `if let Some(d) = saved {
  set_mutation_dispatcher(d) }` (`teardown.rs:239-241`; `tree_builder/mod.rs:240-242`; `elidex-html-parser/src/lib.rs:161-163`;
  `inert_document.rs:198-200`): a take that found nothing is never followed by a restore call (read from the code; the
  fact D-4's state design rests on). Between each take and its restore there is no early return
  (`tree_builder/mod.rs:180-242`: the `drive()` result is matched and both arms fall through to the restore;
  `elidex-html-parser/src/lib.rs:143-163`: straight-line; `inert_document.rs:112-199`: no `return` and no `?`), so only
  a panic skips a restore (read from the code; the fact §4.7's "a restore is owed" rests on).
- **Slot access census (Rev 3, R2 G1; defined by the property).** The population is *every access to `EcsDom`'s
  private field `dispatcher`* — reads included, because the enum of D-4 changes what every access means, and the Rev 2
  census (the slot's method names plus the field writes) was a vocabulary that missed a reader. The field is private,
  so the compiler bounds the set to the module `dom` and its descendants (`crates/core/elidex-ecs/src/dom/**`), and the
  enum change makes the compiler enumerate every site (each `Option` idiom on the field stops compiling). Command (run
  2026-11-04 at `fab44a1d`): `git grep -n -w dispatcher -- crates/core/elidex-ecs/src` → **67** lines. The **11**
  field accesses among them: `dom/mod.rs:66` (the declaration), `:123` (read: `RestoreDispatcher::drop`'s
  `is_none()` assertion), `:127` (write: its restore), `:138` (`new()`), `:171` (`replace`), `:176` (`= None`),
  `:184` (`take`), `:207` (`take`, `dispatch_event`); **`tree/teardown.rs:80`** (read: `destroy_entity`'s
  `is_some()`); `tests/dispatch_guard.rs:44`, `:66` (reads). The other 56: 52 doc / comment / string lines, and 4 code
  lines where the token is the parameter, a local or an argument (`dom/mod.rs:169`, `:213`; `teardown.rs:239`,
  `:240`). The narrower form `git grep -n -P '\.dispatcher\b|^\s*dispatcher:' -- crates/core/elidex-ecs/src` prints
  the 11 plus `:100` (doc) and `:169` (parameter). Complement: an access that does not spell the token is impossible —
  no destructuring pattern of `EcsDom` exists (`git grep -n -P 'EcsDom\s*\{' -- crates` prints only the `struct` and
  `impl` lines), the crate's one `macro_rules!` (`components.rs:11`) is outside `dom` and takes no field of `EcsDom`,
  and any site outside `dom/**` is refused by the compiler. The workspace-wide `git grep -n -P '\.dispatcher\b' --
  crates` adds one line, `crates/net/elidex-net/src/lib.rs:551`, excluded by path: it is the `dispatcher:
  SchemeDispatcher` field of another struct (`:343`), outside the crate the private field is bounded to. The `Taken`
  mapping of every reader is in §4.7 "The readers"; the invariant it gives is §2 I.
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
  `vm/host/custom_elements/flush.rs:39` and four callers: `engine.rs:165`, `vm/host/custom_elements/define.rs:204`
  (`:208` on `origin/main` `4fd85f4e`, 2026-11-04; this memo is anchored at `fab44a1d` and cites `:204`),
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
  `tests_clone_form_state.rs` (11 tests, `:48-180`; the I3 case `clone_input_copies_live_value_detached` `:48-56`);
  `tests_inner_html_mixin.rs:620-635` `clone_node_shallow_clones_clonable_shadow_tree_shallowly`, whose name and
  comment (`:621-625`: "only the shadow children's own clone depth follows the flag (step 6.7), so the `<p>` appears but
  its descendants would not") encode the rule D14 reverses, while its assertion (`:626-634`: the shadow root exists,
  is not the source's, and its first child is a `<p>`) holds under both rules. Sizes in §5.1.
- **Exhaustive `MutationEvent` matches (census, umbrella §4.3 condition 1).** `git grep -l 'MutationEvent::' -- crates ':!*tests*'`
  → 31 files at `fab44a1d` (the umbrella's count holds). Of them, the `match` sites: `range/live.rs:113-163`
  (**exhaustive**, no wildcard; the `AttributeChange` arm `:159-163`); `consumer.rs:76-95` (`_ => {}` `:94`);
  `reconciler.rs:43-52` (`_` `:51`); `crates/dom/elidex-dom-api/src/element/document_base.rs:292-399` (`_` `:399`);
  `crates/script/elidex-script-session/src/event_handler_consumer.rs:382-390` (`_` `:390`);
  `crates/api/elidex-api-canvas/src/component.rs:202` and `crates/dom/elidex-dom-api/src/traversal.rs:1194` (`let … else` /
  `if let`, not exhaustive); `crates/dom/elidex-dom-api/src/test_util.rs:32` (`matches!`); the rest are fire sites or
  doc mentions. So one production exhaustive match gains an arm (`range/live.rs`), plus the test recorder
  (`mutation_hook.rs:68-148`), plus the two consumers' real arms.
- **No-dispatcher windows (umbrella §0.3 item 34, re-run; the one census in this memo, informational).** The
  umbrella's `-P` command at `fab44a1d` gives **25** lines (27 at `27b50dfe`): RC-0a moved `range/live.rs`'s inline
  tests to `range/live_tests.rs`, which the `':!*tests*'` exclusion drops, so the two test hits `live.rs:588` / `:868`
  are gone. That pattern misses one direct write of the field, `RestoreDispatcher::drop`'s restore (`dom/mod.rs:127`,
  R1 I1), so this memo's command adds the field-write form (run 2026-11-04 at `fab44a1d`, **26** lines):
  `git grep -n -P '\b(take_mutation_dispatcher|clear_mutation_dispatcher|set_mutation_dispatcher|begin_detached_fragment)\(|\bself\.(dom\.)?dispatcher\s*=|\bdispatcher\.(take|replace)\(' -- crates ':!*tests*'`
  → `dom/mod.rs:127`, `:167`, `:171`, `:175`, `:176`, `:181`, `:184`, `:207`; `tree/teardown.rs:209`, `:240`, `:293`,
  `:300`; `char_data/split_text.rs:369`, `:395`; `elidex-form/src/inert_document.rs:112`, `:199`;
  `elidex-form/src/value_mode.rs:130`; `elidex-html-parser-strict/src/tree_builder/mod.rs:180`, `:241`;
  `elidex-html-parser/src/lib.rs:143`, `:162`; `vm/lifecycle.rs:34`, `:251`; `vm/vm_api.rs:324`;
  `event_handler_processing/mod.rs:317`; `mutation/html_fragment.rs:696`. The **20** production lines: the slot's
  own methods and guard (`dom/mod.rs:127`, `:167`, `:171`, `:175`, `:176`, `:181`, `:184`) and `dispatch_event`'s take
  (`:207`); the three **take-and-restore windows** — `despawn_subtree` (`teardown.rs:209` take, `:240` restore),
  `begin_detached_fragment` (`teardown.rs:293` definition, `:300` take) with its two callers (`tree_builder/mod.rs:180`,
  restore `:241`; `elidex-html-parser/src/lib.rs:143`, restore `:162`), and `parse_into_inert_document`
  (`inert_document.rs:112`, restore `:199`); bind (`vm_api.rs:324`) and unbind (`lifecycle.rs:34`). The 5 inline-test
  lines (`split_text.rs:369`, `:395`; `value_mode.rs:130`; `event_handler_processing/mod.rs:317`; `html_fragment.rs:696`)
  and the comment (`lifecycle.rs:251`) are the rest. No window calls the cloner entry today: the parser crates create
  nodes through `EcsDom` constructors and `elidex-html-parser-strict` does not depend on `elidex-dom-api`;
  `despawn_subtree` creates nothing; `parse_into_inert_document` parses and attaches components (`:112-199`). This
  list is not a check and ratchets nothing; the check is D-4's assertion (§4.7).
- **Production cloner callers.** `git grep -n 'clone_node_with_shadow_honor\|"cloneNode"' -- crates` (run
  2026-11-04 at `fab44a1d`, **13** lines) → the entry (`clone.rs:227`), its one production caller, `CloneNode`'s
  `invoke` (`clone.rs:154`), the handler's `method_name` string (`clone.rs:88`), its doc mentions (`clone.rs:2`,
  `:329`, `:368`, `:389`; `node_methods_extras.rs:175`), the handler registration (`crates/dom/elidex-dom-api/src/registry.rs:394`,
  `:607`), the VM native's `invoke_dom_api` string (`node_methods_extras.rs:212`), and the well-known name
  (`crates/script/elidex-js/src/vm/well_known.rs:341`). One entry, one production caller, as the umbrella says.
  Complement: a caller that reaches the entry without naming it or the handler's method name would have to call the
  handler through the registry by another string, and `registry.rs:394` registers it under `"cloneNode"` only.
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
   the production file), and 26 with the field-write form this memo adds (`dom/mod.rs:127`). The production sites
   are the same; the census is informational now (D-4).
4. The umbrella's `tests/clone.rs:792-827` / `:902` cites are `clone_shadow.rs:76-112` / `:186-214` after RC-0a
   (the brief's item 1; this memo cites the post-RC-0a locations only).
5. The umbrella's `is_value` grep pattern (`git grep -n is_value -- …`) also matches `this_value`: 19 lines at
   `fab44a1d` (18 `this_value` + 1 `is_value`; Rev 2's "17" was miscounted, R2 G13); with `-w` it gives the one line
   the umbrella meant. The fact stands.
6. The umbrella's D14 structural row says "the subtree flag is per pair … every pair it produces keeps true for passes
   3 and 4". Checked against the spec: the only pair whose subtree flag can be false is the **root** pair. "Clone a
   node" step 5 recurses with its own `subtree` (so only when it is true), step 6.7 passes true, and the template
   cloning steps pass true (step 2) or return (step 1) (`webref body dom concept-node-clone`; `webref body html
   the-template-element`). So "per pair" reduces to one rule: the operation's flag for the root pair, true for every
   other pair (§4.2). This is the umbrella's row made exact, not a change to it.
7. No premise in the brief was found wrong against the code or the umbrella.

### §0.4 Decisions (this PR)

- **D-1 (recorder visibility; brief item 2; Rev 3, R2 G7).** `MockEvent` gains a `CloneCreated { root, scope }`
  variant and `MockHook`'s exhaustive `match` gains its arm in `mutation_hook.rs` (the compiler forces it). RC-2a's
  recorder tests are `fire_*` helper tests — the scenario of `fire_split_text_helper_routes_to_hook` /
  `fire_normalize_merge_helper_routes_to_hook` / `fire_helpers_silent_when_no_hook_installed`
  (`mutation_hook.rs:886-949`) — so they go into that group, not into a per-variant file (one scenario, one file;
  Rev 2's `mutation_hook_clone_created.rs` sibling is withdrawn). Measured against the file's own shapes (§7), the
  group with the new test and the arm puts the file at about 1004 lines, over 1000, so the `fire_*` group is split
  out first as a **standalone pure-move prerequisite** (RC-2a-pre, the shape of RC-0a #587; CLAUDE.md touch-time, not
  a deferral): `mutation_hook.rs:886-949` (64 lines) moves to
  `crates/core/elidex-ecs/src/dom/tests/mutation_hook_fire_helpers.rs` (NEW), `MockEvent` / `MockHook` /
  `install_mock` become `pub(super)` (visibility only — one recorder for the whole stream; a second `MockEvent` beside
  the first would be a duplicated decision surface), and `tests/mod.rs` declares the module. RC-2a then adds the
  variant and the arm to `mutation_hook.rs` (about 907 → about 915) and its tests to the sibling (about 70 → about
  95). The nested-fire assertion test and D-4's window tests go to `dispatch_guard.rs`, the slot's own test file.
- **D-2 (§7 re-measure; brief item 3).** Of the files RC-2a edits (§5.1, with the `wc -l` command), one would cross
  1000 with RC-2a's addition — `mutation_hook.rs` (971 + about 33, §7) — and is split first (D-1, RC-2a-pre); every
  other stays under (`dom/mod.rs` is 926 and gains about 35: the `fire_*` helper with its assertion, the slot enum, a
  re-export; §4.7). `tests_custom_elements.rs` is 1320 and is therefore not touched (F2). Neither C0b nor C0c has
  landed. If either lands before RC-2a's implementation starts (both edit `dom/mod.rs`: C0b adds the entry
  `node_document_for_new_nodes` / `NewNodeOwner` and a test-only walk-counter field on `EcsDom` and in `new()`, and
  changes `begin_detached_fragment(context)` / `finish_detached_fragment` in `teardown.rs` and at both parser callers;
  C0c also `walkers.rs`, `lib.rs`; §9), the files are re-measured at that base and a standalone split comes first if
  any would reach 1000 with RC-2a's addition (umbrella §7 row; the seam for `dom/mod.rs` is in §7). In the C0b-first
  state the two parser call sites of `begin_detached_fragment` are re-read for the restore form D-4 rests on (`if let
  Some(d) = saved { set_mutation_dispatcher(d) }` with no early return between take and restore, §0.2), since C0b
  rewrites those call sites (R2 G9).
- **D-3 (where the form pass's first-level walk calls `for_each_clone_created`).** In `elidex-form/src/clone.rs`,
  not in `init.rs`: `apply_clone_form_state` takes the first level from the shared scope-gated walk and then runs the
  existing `collect_template_inclusive_descendants` on each `<template>` content fragment it found (the fixpoint stays
  where it is, unchanged). `init.rs` is not edited; `parse_into_inert_document`, the other caller of
  `collect_template_inclusive_descendants`, is untouched (§4.5).
- **D-4 (the no-dispatcher census becomes a standing check; brief item 4; Rev 2, R1 F1).** Yes, as a **runtime
  property check inside `elidex-ecs`**, not a source grep. The property is "no `CloneCreated` is fired while the
  dispatcher has been taken out for a window" — a take-and-restore span (`despawn_subtree`,
  `begin_detached_fragment`, `parse_into_inert_document`; §0.2) during which the clone's two passes would be dropped
  without error. The slot gains a third state, **`Taken`**, which distinguishes "taken for a window, a restore is
  pending" from "never installed / cleared at unbind"; `fire_clone_created` `debug_assert!`s that the slot is not
  `Taken` (§4.7). An unbound `EcsDom` (no dispatcher ever, or cleared) stays a legitimate silent no-op, in debug and
  release alike; release keeps today's silent no-op for the window case too. Rev 1's source-grep wire (a population
  census with an allowlist) is withdrawn and not re-introduced in any form, nor is any `REQUIRED_WIRES` change: R1
  found the census defined by the identifier `dispatcher` rather than by the property (its pattern matched
  `dom/mod.rs:127`, `vm_api.rs:300` and four `SchemeDispatcher` lines in `elidex-net`, none of them in its own
  table), its test-line exclusion failing open (`tree_builder/mod.rs:34` `#[cfg(test)]` sits above the
  production windows `:180` / `:241`), and its path + content key collapsing identical takes (`teardown.rs:209` /
  `:300`). Rev 1's reason for rejecting a loud `fire_clone_created` — that `EcsDom` cannot tell a window from "never
  installed" — was true of the `Option` slot and is exactly what the third state supplies; it is one discriminant, not
  a guard type. The honest blind spot of the check is in §4.7.
- **D-5 (D14's rule).** The subtree flag is the operation's for the root pair and true for every other pair (§0.3
  item 6). Implemented as one local in the worklist loop, `let subtree = deep || d != cloned;`, used by pass 3
  (`replicate_template_contents`) and pass 4 (the shadow-child `clone_recording`). The `!deep` hand stamp
  `:294-301` is removed because `clone_subtree` with `doc_override = shadow_doc` stamps the child (and its subtree)
  itself (`tree_clone.rs:208-224`). Which hand stamp each side removes in each landing order is H9's rule (umbrella
  Appendix H), applied as written; the PR body records which state its base was in.
- **D-6 (Attribute stub; umbrella §5.2a).** RC-2a's diff does not touch `CloneNode`'s `invoke` (`clone.rs:91-167`): the
  emitter is inside `clone_node_with_shadow_honor` and the D14 edit is in its worklist loop. So the Attribute clone
  is not folded in; `#11-attr-clone-node` (registered 2026-11-03, PM 316; §8) stays open (owner L3; fold candidate
  `#11-dom-api-attr-entity-model-unification`). The ProcessingInstruction stub stays with
  `#11-processing-instruction-clone` (registered 2026-11-03, PM 316; §8).
- **D-7 (scope in the event, as a contract guard).** `CloneScope` travels in the event (umbrella D-a″). At dispatch
  a `SelfOnly` root has no light children, so a consumer walking the tree would see the same set either way; the
  scope is carried so that the visit rule lives in the walk and not in the caller's timing (umbrella §4.3 condition
  4): the created set is a fact of the clone call, and the walk must not depend on whether a caller (RC-2b, which
  appends after the entry returns) has already attached children by the time a consumer looks.
- **D-8 (the census arm in `range/live.rs`).** RC-2a adds the `CloneCreated` no-op arm there (one production
  exhaustive match, §0.2). RC-1 also edits `range/live.rs` (`LiveRangeRef`); RC-2a lands first (umbrella §9), so RC-1
  rebases. If C0c lands first, RC-2a re-runs the census at its head and adds its arm beside C0c's `Adopt` arm (H6).

## §1. Shape and verdict

A terminal unit under an approved umbrella (CLAUDE.md "Edge-dense work", base case): one PR, this per-PR plan through
`/elidex-plan-review` before implementation. It is still edge-dense in itself (§2 enumerates ten invariant axes, A-J), which
is why the plan-review is mandatory and not a judgment. Coverage breadth K = 2 specs, M = 12 rows (§3): single-PR
scope.

Not in this PR: the pure-move split RC-2a-pre (D-1, §7; its own PR, landed first); any Range file; the contents
algorithm (RC-2b); the node document of clones (C0b / C1, umbrella §2 C3); the registry association of clones in a
null-registry document (`#11-document-null-registry-adopt-reset`, umbrella §9); the reaction queue's element-queue
grouping and the per-method drain (`#11-ce-element-queue-grouping`, `#11-ce-reactions-method-scope-drain`, umbrella
E2 / §9).

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
  6.7, template step 2); the hand stamps follow H9 (D-5).
- **I (windows)**: no cloner-entry call runs while the dispatcher is taken out for a window (the parser builds, the
  inert parse, teardown); `fire_clone_created` asserts it in debug builds (D-4, §4.7). Inside a window every read of
  the slot behaves as "no dispatcher" — `destroy_entity` takes its no-dispatcher branch (`teardown.rs:80`),
  `dispatch_event` returns, `set` returns `None` — and only `fire_clone_created` tells `Taken` from `Absent` (§0.2
  "Slot access census", §4.7 "The readers"). An absent dispatcher (unbound) is not a window: the fire is a silent
  no-op there, as every other fire is.
- **J (census)**: every exhaustive `match` on `MutationEvent` gains an arm (`range/live.rs`, the test recorder); the
  consumers gain real arms; C0c's `Adopt` meets this on the same enum and `match`es (H6).

Pairs (one line each; pairs not listed do not interact):

| Pair | Intersection |
|---|---|
| A × B | the event fires after the whole worklist, so for `Subtree` the full subtree (shadow trees and template contents included) exists at dispatch; for `SelfOnly` the root's shadow tree exists and its light children do not. D14 makes a `SelfOnly` clone emit internal `Insert` events it did not emit before: the shadow children's own descendants are appended under non-ShadowRoot parents during the entry call (`clone_children_recursive`, §0.2 "Pair order"), as a deep clone's are today; `CloneCreated` still comes last |
| A × E | one fire per entry call means one CE pass per root, at the clone's position; RC-2b's per-root calls put each root's upgrades between the `Disconnected` / `Connected` of the surrounding mutations (umbrella M × C) |
| A × I | the entry fires through `dispatch_event`; inside a window the slot is `Taken` and the event would be dropped — `fire_clone_created`'s assertion makes that loud in debug (§4.7); no window reaches the entry today (§0.2) |
| F × I | the detached read (F) holds only when a dispatcher is installed at the clone: an unbound `EcsDom` (this crate's unit tests, Rust-only users) gets the marker and no copy, as before #466's shim it also got none (the shim ran only `host_if_bound()`); inside a window the assertion fires instead of silently losing the copy |
| B × H | after D14 a `SelfOnly` root's shadow tree is a complete deep copy, so the `SelfOnly` walk over it reaches the grandchild custom elements D14 creates; before D14 those did not exist |
| B × F | the form arm's visit set is the same scope-gated first level plus the template frontier; a `SelfOnly` root cannot carry a `<template>` with contents (template step 1), but a `<template>` inside its shadow tree can (cloned with true after D14), and the frontier reaches it |
| C × D | the lock guards are dropped before any component write that the other consumer could read; neither consumer reads the other's components, so the form-before-CE dispatch order (field order) is unobservable |
| C × I | inside a dispatch the slot is `Absent` and `dispatch_depth > 0`: a `fire_clone_created` from a consumer is quiet on the window assertion and trips the depth assertion (the §6 nested-fire test expects the re-entry message); a `take_mutation_dispatcher` inside a dispatch finds `Absent`, returns `None` and leaves it, so the slot is never `Taken` when `RestoreDispatcher::drop` runs, and its `debug_assert!` (the slot is not `Installed`, §4.7) has only today's installed-inside-a-dispatch case to report |
| C × F | `create_form_control_state` at dispatch is the precedent `handle_insert` already sets; the clone's own internal `Insert` events (deep clone, template contents, and after D14 the shadow children's descendants of a shallow clone, A × B) run that same function before the sweep, so a cloned control under a cloned shadow child first gets a default `FormControlState` from `handle_insert`, which the arm then overwrites — today's deep-clone behaviour. The other consumers (`event_handler_attrs`, `base_url`) see those `Insert`s as they see a deep clone's today |
| E × C | the CE arm pushes during dispatch, not after `invoke_dom_api`; between those two points today nothing touched the queue (§0.2), so the position is unchanged and the earlier `is_value` clear is unobservable |
| G × F | the form copy still cannot be done by `elidex-dom-api` (it cannot see `FormControlState`); the event carries only `root` and `scope`, never pairs, so the marker remains the linkage — grounded in §4.8 "marker over pairs", not in #466's I1 × I2 row |
| H × J | D14 and the census touch different files (`node_methods/clone.rs`; `mutation_event.rs` and the match sites); the only shared file with another lane is `node_methods/clone.rs` with C0b (stamps, H9) and C1 (rebases, umbrella §9) |
| I × J | a window that takes the dispatcher also silences every other variant, by design (the windows exist to suppress `Insert` / `Remove` during a throwaway build or teardown); only `CloneCreated` asserts against it, because a clone inside a window is a defect while a suppressed insert is the window's purpose. C0c's `Adopt` decides for itself whether it asserts; nothing here is shared with it |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §4.4 Interface Node | clone a node step 3 (run the cloning steps with node, copy, subtree) | input / textarea (form), template (pass 3) | `clone_node_with_shadow_honor` worklist (`node_methods/clone.rs:247-304`); the form arm at dispatch | ✓ | yes |
| WHATWG DOM §4.4 Interface Node | clone a node step 5 (children, subtree set to subtree) | subtree true / false | `clone_recording` for the root (`clone.rs:229`, `:313-327`) | ✓ | yes |
| WHATWG DOM §4.4 Interface Node | clone a node steps 6.4 and 6.7 (attach a shadow root with clonable true; clone each shadow child with subtree set to **true**) | clonable host / non-clonable host; root pair / nested pair | pass 4 (`clone.rs:266-303`); D14: the child clone passes true, pass 3 and pass 4 of every non-root pair pass true (D-5) | ✓ | yes |
| WHATWG DOM §4.4 Interface Node | clone a single node step 2.4 (create an element with the source's is value, synchronous false, and registry) | custom-element identity (pass 1): defined / undefined / null registry association | `propagate_ce_identity` (identity, `clone.rs:344-366`) then the CE arm at dispatch (registry half); registry steps 2.1-2.3 unchanged (slot, umbrella §9) | ✓ (steps 2.1-2.3 ✗, slot `#11-document-null-registry-adopt-reset`) | yes |
| WHATWG DOM §4.9 Interface Element | create an element steps 4.2 / 4.4 (customized built-in: create with state "undefined" and the is value, enqueue an upgrade reaction) and 5.2.1 / 5.2.2 (autonomous, synchronous false: create with state "undefined" and a null is value, enqueue an upgrade reaction) | customized built-in / autonomous / no definition / `is` mismatch | `apply_clone_creation_ce_semantics(dom, registry, root, scope)` (`elidex-custom-elements/src/upgrade.rs:125-156`), called by the CE arm | ✓ | no |
| WHATWG HTML §4.13.6 Custom element reactions | enqueue a custom element upgrade reaction steps 1-2 | — | the CE arm pushes `CustomElementReaction::Upgrade` per candidate at dispatch (`consumer.rs`, NEW arm) | ✗ (element-queue grouping and the drain point are the registered / proposed engine-wide slots, umbrella E2) | no |
| WHATWG HTML §4.10.5 The input element | cloning steps (propagate value, dirty value flag, checkedness, dirty checkedness flag, indeterminateness) | each field; dirty checkedness unmodeled | the form arm → `apply_clone_form_state(dom, root, scope)` → `copy_form_state` (`elidex-form/src/clone.rs:74-152`) | ✗ (dirty checkedness: `#11-input-dirty-checkedness-flag`, pre-existing, `clone.rs:122-126`) | yes |
| WHATWG HTML §4.10.11 The textarea element | cloning steps (propagate raw value and dirty value flag) | both fields | same | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | cloning steps 1-2 (subtree false → return; else clone the contents' children with subtree true into copy's contents) | root template, shallow / deep; template inside a cloned shadow tree | `replicate_template_contents` (`clone.rs:405-431`) with the per-pair flag (D-5) | ✓ | yes |
| WHATWG HTML §4.13.3 Core concepts | look up a custom element definition step 1 (null registry → null) | template contents (null-registry document) | the walk shape (today's `for_each_shadow_inclusive_descendant` does not enter template contents); interim of umbrella §4.3 condition 6, folded into `#11-document-null-registry-adopt-reset` | ✗ (slot) | no |
| WHATWG DOM §4.5 Interface Document | a document's custom element registry (null unless stated otherwise) | the inert template document | same interim; not modelled | ✗ (slot) | no |
| WHATWG DOM §4.8 Interface ShadowRoot | shadow-including descendant (the visit **set**: `Subtree` = the root and its shadow-including descendants minus the ShadowRoot nodes themselves; `SelfOnly` = the root, its ShadowRoot entity and that root's shadow-including descendants minus nested ShadowRoot nodes — §4.3; the walk's **order** is DOM §4.4 "clone a node" creation order, steps 2 → 5 → 6.7: the node, its light children, then its shadow children — not §4.8 "shadow-including tree order", which would take the shadow tree just after the host) | `Subtree` / `SelfOnly` | `EcsDom::for_each_clone_created` (NEW, `walkers.rs`), on `for_each_shadow_inclusive_descendant` (`walkers.rs:54-80`) | ✓ | no |

Numbers and titles: `webref heading dom 4.4` (Interface Node), `dom 4.9` (Interface Element), `dom 4.5` (Interface
Document), `dom 4.8` (Interface ShadowRoot); `webref heading html 4.13.6` (Custom element reactions), `html 4.10.5`
(The input element), `html 4.10.11` (The textarea element), `html 4.12.3` (The template element), `html 4.13.3` (Core
concepts); also looked up and not cited by number: `html 4.13.5` = Upgrades, `html 4.13.4` = The CustomElementRegistry
interface. Dfns: `webref dfn dom 'cloning steps'` (§4.4, `#concept-node-clone-ext`), `'shadow-including descendant'`
(§4.8), `'shadow-including tree order'` (§4.8, `#concept-shadow-including-tree-order`; looked up to state what the
walk's order is *not*), `'custom element registry'` (for=Document, §4.5); `webref dfn html 'look up a custom element
definition'` (§4.13.3). Prose: `webref body dom concept-node-clone` (steps 2, 3, 5, 6.4, 6.7), `clone-a-single-node`
(steps 2.1-2.4, 7, 8), `concept-create-element` (steps 4.2, 4.4, 5.2.1, 5.2.2), `concept-shadow-including-tree-order`
(the shadow root is traversed "just after" its host); `webref body html the-template-element` (cloning steps 1-2),
`the-input-element` and `the-textarea-element` (cloning steps), `enqueue-a-custom-element-upgrade-reaction` (steps
1-2), `look-up-a-custom-element-definition` (step 1). All 2026-11-04. `webref coverage-map dom 4.4 dom 4.9 html 4.13.6
html 4.10.5 html 4.10.11 html 4.12.3 html 4.13.3 dom 4.5 dom 4.8` (every section of the table, R2 G13; run
2026-11-04) → spec = 2, step = 9 entries (verified 2026-11-04 against the table's nine distinct sections), "ok →
single PR scope". **Breadth: K = 2, M = 12**; single PR.

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
/// created by the same DOM §4.4 "clone a node" call. `Subtree`: step 5 ran
/// (subtree true), so the light children and, under step 6.7, the shadow
/// children of a clonable shadow host. `SelfOnly`: step 5 did not run; the
/// shadow children were still created when the root is a clonable shadow
/// host (step 6 is not gated on subtree), each with subtree true (step
/// 6.7), so their descendants exist too.
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
`dispatch_event`'s take-and-restore and its `dispatch_depth` assertion, and adds D-4's own: before dispatching it
`debug_assert!`s that the slot is not `Taken` (§4.7). The module doc's variant list (`:45-106`),
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
  document into its subtree (`tree_clone.rs:199-224`); when `shadow_doc` is `None` neither path stamps, as today.
  Which other hand stamps this PR keeps or removes is H9's rule, applied as D-5 says (not restated here). The
  pass-4 doc (`:208-216`) and the step 6.7 comment (`:288-289`) are rewritten to the spec's rule.
- The `ClonedFrom` doc (`:24-75`) is rewritten where it names the VM shim: `:33-35` ("from the VM `cloneNode`
  marshalling shim, beside the CE clone-upgrade seam") → the consumer is `FormControlReconciler`'s `CloneCreated`
  arm, run at dispatch before the entry returns; `:48-57` ("Why a persisted marker, not a returned list") → the
  pairs worklist is not in the consumers' visit order (umbrella §0.3 item 14: siblings are appended as a group, so
  a pairs list would run the passes in a different order from the one walk both arms share, umbrella §4.3 condition
  4), so the event names only the root and the scope and the walk defines the set and order; the per-entity src
  link that the walk cannot recover is the marker (§4.8 "marker over pairs"); `:58-64` ("The sole production caller
  is the VM `cloneNode` shim") → a clone made with no
  dispatcher installed (this crate's unit tests, an unbound `EcsDom`) leaves the marker, which stays harmless
  (umbrella §6 row). `propagate_ce_identity`'s doc (`:334-340`, "The engine bindings run the registry-dependent half")
  names the CE arm instead.

Everything else in the file is unchanged: `CloneNode`'s `invoke` (so the Attribute / PI stubs stay, D-6), pass 1, pass
2's tag gate, `read_clonable_shadow_init`.

### §4.3 The shared walk (`elidex-ecs`)

`EcsDom::for_each_clone_created(&self, root: Entity, scope: CloneScope, visit: &mut F)` (NEW) in
`crates/core/elidex-ecs/src/dom/tree/walkers.rs`, beside `for_each_shadow_inclusive_descendant` (`:54`):

- `Subtree` → `self.for_each_shadow_inclusive_descendant(root, visit)`: today's order and cap, unchanged. The visit
  set is DOM §4.8 "shadow-including descendant" (plus the root); the order is the clone's **creation order** — DOM
  §4.4 "clone a node" step 2 (the node), step 5 (its light children, recursively), step 6.7 (its shadow children) —
  which is the order the upgrade reactions are enqueued in today (umbrella §0.3 item 14). It is not §4.8
  "shadow-including tree order" (that traverses a host's shadow tree just after the host, before its light children);
  the §3 row and `for_each_clone_created`'s doc say so.
- `SelfOnly` → `visit(root)`; then, if `self.get_shadow_root(root)` is `Some(sr)`,
  `self.for_each_shadow_inclusive_descendant(sr, visit)`. That visits the ShadowRoot entity first (today's walk from
  the host does not, `walkers.rs:69-73`), then the same elements in the same order at the same depths; neither
  consumer acts on a ShadowRoot (umbrella §4.3 condition 1). The root's light children are never visited. The visit
  set, stated for both scopes (R2 G13): `Subtree` = the root plus its shadow-including descendants (DOM §4.8) minus
  the ShadowRoot nodes themselves (today's walk visits a shadow root's children, never the root, `walkers.rs:69-73`);
  `SelfOnly` = the root, its ShadowRoot entity, and that shadow root's shadow-including descendants minus nested
  ShadowRoot nodes.

It lives in `elidex-ecs` because that is the one crate both consumers depend on. One walk, two consumers (umbrella
§4.3 condition 4; one issue, one way). The depth cap applies per root from depth 0, as `cloneNode`'s does today.

### §4.4 The CE arm (`elidex-custom-elements`)

- `apply_clone_creation_ce_semantics(dom, registry, clone_root, scope: CloneScope)` gains the `scope` argument and
  calls `dom.for_each_clone_created(clone_root, scope, …)` in place of its walk (`upgrade.rs:131`). Its contract
  doc (`:121-124`) becomes "on the root of a `CloneCreated` event, with that event's scope, during its dispatch". It
  becomes **`pub(crate)`** and its re-export is removed from `crates/dom/elidex-custom-elements/src/lib.rs:29-33`
  (the `apply_clone_creation_ce_semantics` item at `:30`): after the VM deletion (§4.6) its only callers are the arm
  and the four inline tests, all in-crate (`git grep -n 'apply_clone_creation_ce_semantics' -- crates` at `fab44a1d`,
  **9** lines: the definition, its four inline-test calls, the re-export, the VM call
  `vm/host/custom_elements/upgrade.rs:142`, and doc mentions in `node_methods/clone.rs:181`, `:336` — the VM call
  goes, the doc mentions are re-pointed, §4.9).
  The four inline tests (`:391`, `:416`, `:427`, `:452`) pass `CloneScope::Subtree` (their trees are deep clones of
  a single element; one gains a `SelfOnly` sibling case, §6).
- `CustomElementReactionConsumer::handle` (`consumer.rs:76-95`) gains, before the wildcard,
  `MutationEvent::CloneCreated { root, scope } => self.handle_clone_created(root, scope, dom)`.
  `handle_clone_created(&mut self, root, scope, dom: &mut EcsDom)` (NEW): lock the registry; `let candidates =
  apply_clone_creation_ce_semantics(dom, &registry, root, scope);` drop the guard; if non-empty, lock the queue and
  push one `CustomElementReaction::Upgrade(entity)` per candidate in order. This is `handle_insert`'s shape
  (`:98-123`) with `&mut EcsDom`, which `handle` already receives. The poison policy is the consumer's `expect`
  (`:116`, `:120`), not the shim's `unwrap_or_else` (`vm/host/custom_elements/upgrade.rs:138`, `:150`): every
  insertion already takes the `expect` path, so this adds no failure mode (umbrella §0.3 item 28). The module doc
  (`:72-74`) and the gating-rules section name the new arm.

### §4.5 The form arm (`elidex-form`)

- `apply_clone_form_state(dom, clone_root, scope: CloneScope)` (`clone.rs:44-65`) gains `scope`. Its collection
  step (`:49`) becomes (D-3): first level = `dom.for_each_clone_created(clone_root, scope, |e| found.push(e))`;
  then, for each `e` in `found` with `dom.template_contents_fragment(e) == Some(fragment)`, extend `found` with
  `crate::init::collect_template_inclusive_descendants(dom, fragment)` (the existing fixpoint, `init.rs:111-125`,
  which already handles nested templates). That loop iterates the **first-level set only** (the entities the shared
  walk produced); the entities the fixpoint appends are not iterated again, because the fixpoint has already
  followed every nested `<template>` under them. It becomes **`pub(crate)`** and its re-export is removed from
  `crates/dom/elidex-form/src/lib.rs:46`: after the VM deletion its callers are the arm and the six inline tests
  (`git grep -n 'apply_clone_form_state' -- crates` at `fab44a1d`, **16** lines: the definition and its six
  inline-test calls, the re-export, the VM call `node_methods_extras.rs:230`, and doc mentions in `tree_clone.rs:28`,
  `:29`, `node_methods/clone.rs:33`, `:60`, `:196`, `:254`, `node_methods_extras.rs:190`, `tests_clone_form_state.rs:8`
  — the VM call and its doc go, the rest are re-pointed, §4.9). The marker sweep and `copy_form_state` are
  unchanged. A `SelfOnly` `<template>` root has an empty fresh fragment (template cloning steps step 1;
  `replicate_template_contents` returns at `:423-425`), so the frontier finds nothing under it; a `<template>` inside the root's shadow tree has
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

### §4.7 The no-dispatcher windows and the window assertion (D-4)

**The windows at `fab44a1d`** are the three take-and-restore spans of §0.2's census; no clone happens inside any of
them today (§0.2). The entry is reached only from `CloneNode`'s `invoke` (the caller census in §0.2), which runs from
a VM native with a bound VM (dispatcher installed) or from a Rust caller of the dom-api handler.

**The state** (`crates/core/elidex-ecs/src/dom/mod.rs`). The slot `dispatcher: Option<Box<dyn MutationDispatcher +
Send + Sync>>` (`:66`) becomes a private three-state enum

```rust
/// The mutation-dispatcher slot. `Taken` is a dispatcher handed out by
/// `take_mutation_dispatcher` for a window (despawn / throwaway parse); the
/// caller's restore is pending. `Absent` is "never installed", "cleared at
/// unbind", or the slot during a dispatch (`dispatch_depth > 0` says which).
enum DispatcherSlot { Absent, Installed(Box<dyn MutationDispatcher + Send + Sync>), Taken }
```

so there is one field and no two-field invariant to keep. The transitions, each the existing method with the
state made explicit:

- `set_mutation_dispatcher(d)`: `Installed(d)`; returns the displaced box when the slot was `Installed` (the `Vm::bind`
  `debug_assert!` on `displaced`, `vm_api.rs:325-329`, is unchanged), `None` when it was `Absent` or `Taken` (a
  window's restore).
- `clear_mutation_dispatcher()`: `Absent`, from every state, `Taken` included (R2 G2): unbind is legitimate at any
  time, and a window whose dispatcher was cleared under it restores into `Absent` as a plain install, which is what
  today's `Option` does too.
- `take_mutation_dispatcher()`: `Installed(d)` → `Taken`, returns `Some(d)`; `Absent` or `Taken` → unchanged, returns
  `None`. **A take that finds nothing opens no window**: every window caller restores only `if let Some(d) = saved`
  (§0.2), so a window opened on an empty take would never close, and an unbound `EcsDom` running the fragment parser
  (every parser unit test) would trip the assertion on its next clone. Nested windows (the strict parse-error
  rollback runs `despawn_subtree` inside `begin_detached_fragment`'s window, `tree_builder/mod.rs:180` / `:241`) are
  right by the same rule: the inner take returns `None` and leaves `Taken`, the outer restore closes it. **The
  contract its doc states** (R2 G2): a take that returned `Some` owes a restore (`set_mutation_dispatcher` with the
  box) or a `clear_mutation_dispatcher`; a take followed by neither leaves `Taken`, and the next `fire_clone_created`
  on that `EcsDom` asserts in debug. In production every take is restored (§0.2: no early return between a take and
  its restore). The two existing tests that take without restoring — `tests/destroy.rs:393` (asserts the restored box
  is `Some`) and `mutation_hook.rs:478-480` (`set_mutation_dispatcher_returns_previous_hook`: two takes, the second
  `None`) — stay green: no clone follows the take in either, and the second take finds `Taken` and returns `None`, as
  it found `None` today.
- `dispatch_event`: `Installed(d)` → `Absent` (today's `take()` at `:207`, with the state named), then the existing
  depth increment and `RestoreDispatcher`; `Absent` / `Taken` → return (today's "no dispatcher" path). A dispatch in
  progress is **not** a window: it is already represented by `dispatch_depth`, and a fire inside it is the re-entry
  assertion's case (`:200-206`), so it does not use `Taken`. `RestoreDispatcher::drop` (`:120-130`) writes
  `Installed(pending)` and its `debug_assert!` becomes "the slot is not `Installed`" (a dispatcher installed inside
  a dispatch would be overwritten, as today); the doc at `:99-114` is rewritten for the enum.

**The assertion.** `fire_clone_created` (§4.1) runs, before `dispatch_event`,

```rust
debug_assert!(
    !matches!(self.dispatcher, DispatcherSlot::Taken),
    "fire_clone_created while the mutation dispatcher is taken out \
     (take_mutation_dispatcher without its restore or clear): the CloneCreated \
     event would be dropped and the clone's custom-element upgrade and form-state \
     passes lost.  Restore the dispatcher before cloning."
);
```

on the `dispatch_depth` re-entry precedent (`:200-206`): the message names the contract (`take_mutation_dispatcher`'s
doc, above) and the loss, not the windows — the windows are a census (§0.2), and a message that listed them would be
a second copy of it to keep in step (R2 G3). Inside a dispatch the slot is `Absent`, so this assertion is quiet there
and `dispatch_event`'s depth assertion fires — the two do not overlap (§2 C × I; the §6 nested-fire test expects the
re-entry message). Release builds compile the assertion out and keep today's silent no-op; an `Absent` slot is silent
in every build.

**The readers** (R2 G1; the population is §0.2 "Slot access census", bounded by the compiler). Every access to the
field and what it does on each state after D-4:

| Site | Access | `Installed(d)` | `Absent` | `Taken` |
|---|---|---|---|---|
| `set_mutation_dispatcher` (`dom/mod.rs:171`) | write | `Installed(new)`, returns `Some(d)` | `Installed(new)`, `None` | `Installed(new)`, `None` (a window's restore) |
| `clear_mutation_dispatcher` (`:176`) | write | `Absent` | `Absent` | `Absent` |
| `take_mutation_dispatcher` (`:184`) | read + write | `Taken`, returns `Some(d)` | unchanged, `None` | unchanged, `None` |
| `dispatch_event` (`:207`) | read + write | `Absent` for the dispatch, then the guard restores `Installed(d)` | return (no dispatcher) | return (no dispatcher) |
| `RestoreDispatcher::drop` (`:123`, `:127`) | read + write | `debug_assert!` fails (a dispatcher installed inside a dispatch; today's `is_none()` check with the state named) | writes `Installed(pending)` | unreachable (§2 C × I); the assertion passes it |
| `new()` (`:138`) | write | — | `Absent` | — |
| **`destroy_entity` (`tree/teardown.rs:80`)** | read | fires (`index_in_parent` + `is_connected`) | no-dispatcher branch | no-dispatcher branch |
| `fire_clone_created` (NEW, §4.1) | read | dispatches | silent no-op | `debug_assert!` fails |
| `dispatch_guard.rs:44`, `:66` | read (tests) | `matches!(…, Installed(_))` holds | fails | fails |

The one reader outside `dom/mod.rs` is `destroy_entity`'s `self.dispatcher.is_some()`, which becomes
`matches!(self.dispatcher, DispatcherSlot::Installed(_))`: `Taken` and `Absent` both read as "no dispatcher" there,
and that is what
`despawn_subtree`'s window needs. It relies on that branch to skip `index_in_parent` and `is_connected` per node
(`teardown.rs:73-78`: both O(ancestor depth), so O(n²) over a maliciously deep teardown — the rollback path the
primitive exists for) and the per-node focused-area scan (`:216-219`: O(n·world), reset once after the walk instead);
the window is its own `take` (`:209`), so inside it the slot is `Taken`, and a `Taken` that read as "installed" would
restore the O(n²) work the window was built to avoid. `fire_clone_created` is the only reader that tells `Taken`
from `Absent`; every other reader treats them alike (§2 I).

**Tests** (`dispatch_guard.rs`, the slot's file; §6): (w1) a dispatcher installed, `take_mutation_dispatcher()`, then
`fire_clone_created` panics under `#[cfg(debug_assertions)]` with `#[should_panic(expected = "dispatcher is taken out")]`;
(w2) `EcsDom::new()`, `take_mutation_dispatcher()` (returns `None`), then `fire_clone_created` does not panic — the
empty take opened no window; (w3) take, `set_mutation_dispatcher` back, then `fire_clone_created` dispatches
normally (the hook counts one event). The unbound no-op is (w2)'s first half and the recorder test in §6. The two
`dom.dispatcher.is_some()` reads in the existing tests (`dispatch_guard.rs:44`, `:66`) become `matches!(…,
DispatcherSlot::Installed(_))`.

**What the check sees and what it does not** (R1 asked for this plainly). It checks the property itself — a clone
fired while a window is open — at the one place that knows whether a window is open, the slot; a new window opened
by `take_mutation_dispatcher` anywhere in the tree, or a new caller of the entry (RC-2b), is covered with nothing to
register. Bounded (R2 G2): a window built another way — `set_mutation_dispatcher` with a stand-in dispatcher and a
later restore, the replace-then-retrieve pattern its doc allows (`dom/mod.rs:163-166`) — leaves the slot `Installed`
and is invisible to the check. No such window exists at `fab44a1d`: `git grep -n 'set_mutation_dispatcher(' -- crates`
(run 2026-11-04) gives **11** lines — the definition (`dom/mod.rs:167`), the four window restores (`teardown.rs:240`,
`tree_builder/mod.rs:241`, `elidex-html-parser/src/lib.rs:162`, `inert_document.rs:199`; each passes back the box its
own take returned, §0.2), `Vm::bind`'s install (`vm_api.rs:324`), and five inline-test installs (`split_text.rs:369`,
`:395`; `value_mode.rs:130`; `event_handler_processing/mod.rs:317`; `html_fragment.rs:696`), none of them a stand-in
with a later restore. The complement is a future caller; the take-and-restore idiom is the one the four windows use
and the one the doc names, so a stand-in window would be a new idiom, reviewed as such. It sees only what runs:
a clone inside a window on a path no test exercises stays silent, and every path is silent in release. Against
that, Rev 1's wire saw every production write of the slot but, by its own header, not whether a clone ran inside
one — the classification was a judgment recorded in a row; its population was defined by an identifier and matched
non-windows (D-4); its test exclusion failed open; its key collapsed identical lines. A list that bounds the wrong
population with a judgment per row is weaker than an assertion on the right property that runs on every debug test
of every crate that clones (`cargo nextest` under the dev profile, which `mise run ci` uses, has `debug_assertions`
on). The residual — untested paths — is the general residual of every `debug_assert!` in `elidex-ecs`, including the
re-entry one this follows.

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
#466's shim (§2 F) — with one precondition I3 did not have to state: **a dispatcher is installed at the clone**. #466's
shim ran under `host_if_bound()`, so a clone on an unbound `EcsDom` got no copy then either; what is new is the window
case, where a dispatcher exists but is taken out, and that is the F × I pair (§2), closed by D-4's assertion rather
than by prose. I4 and I5 hold unchanged: the same function creates the state at clone time, over the same tree
(§2 B × F; `cloneNode(true)` is `Subtree`, today's walk; the template frontier stays).

**Marker over pairs** (why the event carries `root` + `scope` and not the `(src, dst)` list). Not #466's I1 × I2 row
(it rests on I2, which no longer binds). The independent reason: the pairs worklist is not in the consumers' visit
order — `clone_children_recursive` appends each parent's children as a group, so for `<div><x-a><x-b/></x-a><x-c/></div>`
the pairs run div, x-a, x-c, x-b while creation order is div, x-a, x-b, x-c (umbrella §0.3 item 14, measured there) —
and both arms take their set and order from the one scope-gated walk (umbrella §4.3 condition 4). A pairs list in
the event would be a second definition of the visit set, in a different order. The marker is the per-entity src link
the walk cannot recover; it stays. **Residual** (R2 G11, a pre-existing defect, stated, not fixed here): the walk is
capped at `MAX_ANCESTOR_DEPTH` (§0.2 "The walk") but the cloner is not — `clone_children_recursive`
(`tree_clone.rs:390-424`) is an explicit stack with no depth check (`git grep -n MAX_ANCESTOR_DEPTH --
crates/core/elidex-ecs/src/dom/tree_clone.rs` → its doc at `:384` only) — so clones below the cap are created and are
invisible to both arms: no upgrade reaction is enqueued for a custom element there, and no form state is copied to a
control there (its `ClonedFrom` marker is never swept; harmless by `tree_clone.rs:29`'s rule — a stale marker
resolves to a miss and despawns with its entity). Today's post-pass has the same hole (the same walk, `upgrade.rs:131`
and `init.rs:116`), and so does `handle_insert`: `classify_connected_subtree` (`consumer.rs:193-217`) walks
`for_each_shadow_inclusive_descendant` from the inserted root, so an element inserted deeper than the cap gets no
`Connected` / `Upgrade` reaction either (uncertain in R2; read from the code here). The cap is the deliberate DoS
bound of the walker (`walkers.rs:40-47`); lifting it for the two arms alone would make clones and inserts disagree,
so it is left as it is and recorded.

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
(the test file header states the wrong step 6.7 rule) and `propagate_ce_identity`'s doc (`:334-340`). Rev 2 adds
(R1 F7): `node_methods/clone.rs:176-185` (pass 1's doc: "enqueued by the engine-bound seam
`elidex_custom_elements::apply_clone_creation_ce_semantics` (Codex PR331 R13/R14)" → the CE arm, in-crate), `:196`
and `:254` (the `elidex_form::apply_clone_form_state` path, no longer public → the form arm); `tree_clone.rs:29`
("recording the src→dst link across the handler-dispatch boundary" → across the clone, to the `CloneCreated` arm);
`tests_inner_html_mixin.rs:620-635`, whose name and comment encode the reversed rule (§0.2): renamed
`clone_node_shallow_replicates_clonable_shadow_tree`, comment rewritten to the spec's rule (step 6 is not gated on
subtree; step 6.7 clones the shadow children with subtree true), assertion unchanged (§6).

Citation rule (umbrella §6): on the files this PR edits, every `§` number ↔ title pair and step number is verified
with the umbrella's patterns and the step grep, and the PR body records each pattern's hit count and disposition.
**Pre-sweep at `fab44a1d`** (2026-11-04). Population: the files of §5.1 that RC-2a edits and that exist at
`fab44a1d` — 25 files (counted against §5.1's table, verified 2026-11-04), written out (R2 G13): `<files>` =
`crates/dom/elidex-dom-api/src/node_methods/clone.rs crates/core/elidex-ecs/src/dom/mutation_event.rs
crates/core/elidex-ecs/src/dom/mod.rs crates/core/elidex-ecs/src/lib.rs
crates/core/elidex-ecs/src/dom/tree/walkers.rs crates/core/elidex-ecs/src/dom/tree_clone.rs
crates/core/elidex-ecs/src/dom/tree/teardown.rs crates/dom/elidex-custom-elements/src/consumer.rs
crates/dom/elidex-custom-elements/src/upgrade.rs crates/dom/elidex-custom-elements/src/lib.rs
crates/dom/elidex-form/src/reconciler.rs crates/dom/elidex-form/src/clone.rs crates/dom/elidex-form/src/lib.rs
crates/script/elidex-js/src/vm/consumer_dispatcher.rs crates/script/elidex-js/src/vm/host/node_methods_extras.rs
crates/script/elidex-js/src/vm/host/custom_elements/upgrade.rs crates/dom/elidex-dom-api/src/range/live.rs
crates/dom/elidex-dom-api/src/node_methods/tests/clone_shadow.rs crates/dom/elidex-dom-api/src/test_util.rs
crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs crates/core/elidex-ecs/src/dom/tests/dispatch_guard.rs
crates/core/elidex-ecs/src/dom/tests/traversal.rs crates/script/elidex-js/src/vm/tests/tests_clone_form_state.rs
crates/script/elidex-js/src/vm/tests/tests_inner_html_mixin.rs crates/script/elidex-js/src/vm/tests/mod.rs` (the two
NEW files and the file RC-2a-pre creates have no text at `fab44a1d`; `elidex-ecs`'s `tests/mod.rs` is edited by
RC-2a-pre, not RC-2a, and carries no `§` or step text: `git grep -n -i -P '§|(?<![a-z])steps?[\s_-]+\d' --
crates/core/elidex-ecs/src/dom/tests/mod.rs` → 0). Counts: prefixed `git grep -n -P '(DOM|HTML|CSSOM|Selection API)
§\d' -- <files>` → **155** lines; bare `git grep -n -P '(?<!DOM |HTML |CSSOM |API |262 |IDL )§\d' -- <files>` →
**92**; synonym / context (umbrella pattern 4) → **89**; the non-existent-section probe (pattern 5) → **95**
distinct forms (the 95 include non-spec `§` such as design-doc `§15.4.1` and plan `§3.2.3`, which the sweep
classifies, not verifies); step grep → **90** lines. The same five over Rev 2's list (R2's check: `elidex-ecs`'s
`tests/mod.rs` in, `teardown.rs` out) give 149 / 87 / 83 / **94** / 89 — pattern 5 is 94 there, not Rev 2's 95 (R2
G13); `teardown.rs` adds 6 / 5 / 6 / 1 / 1. The per-form `webref heading` verification of all 95 is the PR's sweep
(DoD 11).
**Drifts found so far**, all on files this PR edits, fixed in it, each looked up 2026-11-04: `walkers.rs:254`
"WHATWG §4.4.2" for `Node.prototype.contains` — DOM has no §4.4.2 (`webref heading dom 4.4.2` → none);
`contains(other)` is §4.4 Interface Node (`webref dfn dom 'contains'`) → "DOM §4.4 Interface Node `contains(other)`";
`reconciler.rs:256` "HTML §2.5.2" for boolean attributes — §2.5.2 = Determining the type of a resource; boolean
attributes are §2.3.2 (`webref heading html 2.3.2`; `webref dfn html 'boolean attribute'`) → "HTML §2.3.2";
`dom/mod.rs:426` "HTML §7.2" for the Window object — §7.2 = APIs related to navigation and session history; the
Window object is §7.2.2 (`webref heading html 7.2.2`) → "HTML §7.2.2". Rev 3 adds (R2 G13 and the `teardown.rs`
rows): `walkers.rs:249` "WHATWG 'descendant' semantics (§4.2.1)" — DOM §4.2.1 = Document tree (`webref heading dom
4.2.1`); "descendant" is DOM §1.1 Trees (`webref dfn dom descendant` → §1.1 `#concept-tree-descendant`) → "DOM §1.1
Trees"; `walkers.rs:49` "`customElements.upgrade(root)` (HTML §4.13.4 step 8)" — §4.13.4 = The CustomElementRegistry
interface holds (`webref heading html 4.13.4`), but the `upgrade(root)` method steps are one step with three
substeps (`webref body html dom-customelementregistry-upgrade`): no step 8 → "HTML §4.13.4 `upgrade(root)` step 1";
`tests_inner_html_mixin.rs:3` "`innerHTML`, `outerHTML`, `setHTMLUnsafe`, `getHTML` (WHATWG HTML §4.4.5 / §4.4.6 /
§4.4.7)" — those are The ol / ul / menu element (`webref heading html 4.4.5`, `4.4.6`, `4.4.7`); the four are HTML
§8.5 DOM parsing and serialization APIs — `setHTMLUnsafe` in §8.5.2 (HTML parsing and insertion methods),
`getHTML` in §8.5.3 (HTML serialization methods), `innerHTML` in §8.5.4, `outerHTML` in §8.5.5 (`webref dfn html`
for each) → cite those four;
`:325` "shadow content is skipped per §4.4.6" (`getHTML()` with no options) → "HTML §8.5.3"; `mutation_hook.rs:279`
"WHATWG DOM §5.5 'remove a node' step 4" — DOM §5.5 = Interface Range; "remove" is DOM §4.2.3 Mutation algorithms
(`webref heading dom 4.2.3`; `webref body dom concept-node-remove`), whose step 3 runs the live range pre-remove
steps, where step 3 is "Let index be node's index" (`webref body dom live-range-pre-remove-steps`) → "DOM §4.2.3
'remove' step 3 (live range pre-remove steps, step 3)"; `teardown.rs:102` "WHATWG §5.5 remove step 4" — the same
drift, the same fix; `teardown.rs:47` "shadow-tree boundaries are the consumer's responsibility per WHATWG §5.5" —
resolves to DOM §5.5 Interface Range (the consumer is the live-range registry; `webref heading dom 5.5`) and holds;
the spec word is added ("DOM §5.5 Interface Range"). Verified and left as they are in `teardown.rs`: the bare
`§2.1.4` at `:110`, `:135`, `:215` resolve to HTML §2.1.4 DOM trees, the "HTML element removing steps" step 2
(`webref dfn html 'removing steps'` → §2.1.4 `#html-element-removing-steps`; `mutation.rs:315` names it in full);
`:7`, `:245`, `:317` DOM §4.5 Interface Document (`adopt`; `webref heading dom 4.5`); `:248`, `:261`, `:309` HTML
§13.4 Parsing HTML fragments (`webref heading html 13.4`). Those this memo depends on are verified in §3; a number
webref cannot confirm is replaced by the name.

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
- **Piece → crate → API** (R1 F13):

| Piece | Crate | Existing API reused / NEW |
|---|---|---|
| `MutationEvent::CloneCreated`, `CloneScope` | `elidex-ecs` | NEW variant on the existing enum; NEW enum |
| `EcsDom::fire_clone_created` (NEW) | `elidex-ecs` | NEW, one call to the existing private `dispatch_event` (`fire_split_text` shape) |
| `DispatcherSlot` (D-4) | `elidex-ecs` | NEW private enum replacing the `Option` slot; the three public slot methods (`set` `:167` / `clear` `:175` / `take` `:181`) keep their signatures; the private `dispatch_event` (`:199`) and `RestoreDispatcher` follow the enum |
| `destroy_entity`'s slot read (`tree/teardown.rs:80`) | `elidex-ecs` | existing read follows the enum (`Installed(_)`; §4.7 "The readers") |
| `EcsDom::for_each_clone_created` (NEW) | `elidex-ecs` | NEW, over the existing `for_each_shadow_inclusive_descendant` and `get_shadow_root` |
| census no-op arm | `elidex-dom-api` `range/live.rs` | existing exhaustive `match` gains an arm (D-8, §5.2) |
| the fire and D14 | `elidex-dom-api` | existing `clone_node_with_shadow_honor`; existing `clone_recording` / `replicate_template_contents` with the per-pair flag |
| CE arm | `elidex-custom-elements` | existing `CustomElementReactionConsumer::handle` gains an arm; NEW `handle_clone_created` (the `handle_insert` shape); existing `apply_clone_creation_ce_semantics` (+ `scope`, `pub(crate)`) |
| form arm | `elidex-form` | existing `FormControlReconciler::handle` gains an arm; NEW free fn `handle_clone_created` beside `handle_insert`; existing `apply_clone_form_state` (+ `scope`, `pub(crate)`), `collect_template_inclusive_descendants` |
| `cloneNode` native | `elidex-js` `vm/host/` | existing `native_node_clone_node`, post-pass deleted; `enqueue_subtree_upgrade_reactions` deleted |
| test probe | `elidex-dom-api` `test_util.rs` | NEW `CloneCreatedProbe` (the `AttrChangeCounter` shape) |

## §5. Populations

### §5.1 What the PR edits and adds (`wc -l` at `fab44a1d`)

`wc -l crates/dom/elidex-dom-api/src/node_methods/clone.rs crates/core/elidex-ecs/src/dom/mutation_event.rs crates/core/elidex-ecs/src/dom/mod.rs crates/core/elidex-ecs/src/lib.rs crates/core/elidex-ecs/src/dom/tree/walkers.rs crates/core/elidex-ecs/src/dom/tree_clone.rs crates/core/elidex-ecs/src/dom/tree/teardown.rs crates/dom/elidex-custom-elements/src/consumer.rs crates/dom/elidex-custom-elements/src/upgrade.rs crates/dom/elidex-custom-elements/src/lib.rs crates/dom/elidex-form/src/reconciler.rs crates/dom/elidex-form/src/clone.rs crates/dom/elidex-form/src/lib.rs crates/script/elidex-js/src/vm/consumer_dispatcher.rs crates/script/elidex-js/src/vm/host/node_methods_extras.rs crates/script/elidex-js/src/vm/host/custom_elements/upgrade.rs crates/dom/elidex-dom-api/src/range/live.rs crates/dom/elidex-dom-api/src/node_methods/tests/clone_shadow.rs crates/dom/elidex-dom-api/src/test_util.rs crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs crates/core/elidex-ecs/src/dom/tests/dispatch_guard.rs crates/core/elidex-ecs/src/dom/tests/traversal.rs crates/core/elidex-ecs/src/dom/tests/mod.rs crates/script/elidex-js/src/vm/tests/tests_clone_form_state.rs crates/script/elidex-js/src/vm/tests/tests_inner_html_mixin.rs crates/script/elidex-js/src/vm/tests/mod.rs crates/script/elidex-js/src/vm/tests/tests_custom_elements.rs crates/script/elidex-js/src/vm/tests/tests_create_element_options.rs crates/script/elidex-js/src/vm/tests/tests_clone_node.rs`
(run 2026-11-04 in the worktree at `f95deec4`, whose tree differs from `fab44a1d` only in `docs/plans/` — `git diff
--stat fab44a1d HEAD` → one file; every file in the table is therefore at its `fab44a1d` count; the command also
measures the four files named below as not touched by RC-2a):

| File | Lines | Change |
|---|---|---|
| `crates/dom/elidex-dom-api/src/node_methods/clone.rs` | 458 | the fire; D14 (per-pair flag, `:294-301` deleted); docs (§4.2, §4.9) |
| `crates/core/elidex-ecs/src/dom/mutation_event.rs` | 352 | `CloneCreated`, `CloneScope`; docs |
| `crates/core/elidex-ecs/src/dom/mod.rs` | 926 | `fire_clone_created` with D-4's assertion; `DispatcherSlot` (the slot, `new()`, the four methods, `dispatch_event`, `RestoreDispatcher`); re-export; slot doc; `:426` citation |
| `crates/core/elidex-ecs/src/lib.rs` | 51 | re-export |
| `crates/core/elidex-ecs/src/dom/tree/walkers.rs` | 392 | `for_each_clone_created`; `:254` citation |
| `crates/core/elidex-ecs/src/dom/tree_clone.rs` | 425 | doc rows `:28-29` only |
| `crates/core/elidex-ecs/src/dom/tree/teardown.rs` | 344 | `destroy_entity`'s slot read `:80` follows the enum (`Installed(_)`, §4.7 "The readers"); `:47`, `:102` citations (§4.9) |
| `crates/dom/elidex-custom-elements/src/consumer.rs` | 329 | the arm; docs |
| `crates/dom/elidex-custom-elements/src/upgrade.rs` | 526 | `scope` argument on the shared walk; `pub(crate)`; contract doc; inline tests |
| `crates/dom/elidex-custom-elements/src/lib.rs` | 37 | re-export removed (`:30`) |
| `crates/dom/elidex-form/src/reconciler.rs` | 333 | the arm; module doc; `:256` citation |
| `crates/dom/elidex-form/src/clone.rs` | 304 | `scope` argument, first level on the shared walk (D-3); `pub(crate)`; module doc; inline tests |
| `crates/dom/elidex-form/src/lib.rs` | 69 | re-export removed (`:46`) |
| `crates/script/elidex-js/src/vm/consumer_dispatcher.rs` | 160 | field docs |
| `crates/script/elidex-js/src/vm/host/node_methods_extras.rs` | 293 | post-pass deleted; doc |
| `crates/script/elidex-js/src/vm/host/custom_elements/upgrade.rs` | 487 | `enqueue_subtree_upgrade_reactions` deleted |
| `crates/dom/elidex-dom-api/src/range/live.rs` | 500 | the census no-op arm |
| `crates/dom/elidex-dom-api/src/node_methods/tests/clone_shadow.rs` | 277 | D14 inversion; stamp-guard extension; event-count and scope tests; header |
| `crates/dom/elidex-dom-api/src/test_util.rs` | 49 | `CloneCreatedProbe` (NEW) test dispatcher recording `(root, scope)` |
| `crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs` | 971 (about 907 after RC-2a-pre) | `MockEvent` variant + recorder arm (D-1); the recorder is `pub(super)` since RC-2a-pre |
| `crates/core/elidex-ecs/src/dom/tests/mutation_hook_fire_helpers.rs` (NEW in RC-2a-pre, about 70) | — at `fab44a1d` | `fire_clone_created_helper_routes_to_hook`; one line in `fire_helpers_silent_when_no_hook_installed` (D-1, §6) |
| `crates/core/elidex-ecs/src/dom/tests/dispatch_guard.rs` | 69 | the nested-fire assertion test; D-4's three window tests; the two `is_some()` reads follow the enum (§4.7) |
| `crates/core/elidex-ecs/src/dom/tests/traversal.rs` | 413 | `for_each_clone_created` unit tests |
| `crates/script/elidex-js/src/vm/tests/tests_clone_form_state.rs` | 176 | module doc `:7-9` only (§4.8) |
| `crates/script/elidex-js/src/vm/tests/tests_inner_html_mixin.rs` | 749 | `:620-635` rename + comment (§4.9, §6); value unchanged |
| `crates/script/elidex-js/src/vm/tests/tests_clone_created.rs` (NEW) | — | the end-to-end cases (e)-(h) of §6 (R1 F2) |
| `crates/script/elidex-js/src/vm/tests/mod.rs` | 202 | `mod tests_clone_created;` |

Not touched, with the reason: `crates/script/elidex-js/src/vm/tests/tests_custom_elements.rs` (**1320**: over the
1000-line threshold; RC-2a adds nothing to it and edits no doc in it, R1 F2 — its `:567-587` clone test stays as a
"stay green" guard, §6); `tests_create_element_options.rs` (376: its `:219-237` case stays as a guard; the `true`
twin of §6 (f) goes to the new file); `tests_clone_node.rs` (296, `cloneNode` basics; untouched);
`crates/core/elidex-ecs/src/dom/tests/mod.rs` (28: RC-2a-pre adds `mod mutation_hook_fire_helpers;`, RC-2a adds
nothing). RC-2a-pre's own edit set is `mutation_hook.rs` (the move and the three visibility changes),
`mutation_hook_fire_helpers.rs` (NEW) and that `tests/mod.rs`, nothing else (D-1, §7). Stated because
the umbrella listed them as possible: `crates/dom/elidex-form/src/init.rs` (D-3), `CloneNode`'s `invoke` (D-6), every
Range file, `reaction.rs`, `custom_elements/flush.rs`. No file under `scripts/` or `.claude/tools/` (D-4, Rev 2).

### §5.2 Exhaustive `MutationEvent` matches gaining an arm

From §0.2 (command: `git grep -l 'MutationEvent::' -- crates ':!*tests*'`, 31 files, each `match` inspected for a
wildcard): `crates/dom/elidex-dom-api/src/range/live.rs:113-163` (no-op arm, the bridge ignores the event, umbrella
L × C); the test recorder `crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs:68-148` (records it). The two
consumer matches gain real arms (§4.4, §4.5). All other matches have a wildcard or are `if let` / `let … else`.
Re-run at the implementation head, and again if C0c lands first (H6).

(Rev 1's §5.3, the window census with per-row classifications, is withdrawn with the wire: the census lives once, in
§0.2, as information; the check is §4.7.)

## §6. Test and coverage plan

**Flips (expected value changes).** One: `clone_node_shallow_replicates_own_clonable_shadow_shallowly`
(`clone_shadow.rs:76-112`) is inverted and renamed `clone_node_shallow_replicates_own_clonable_shadow_deeply`: the
shadow child's own descendants **are** cloned (step 6.7, subtree true). Its red run at the pre-fix head is D14's
negative control (umbrella §0.4: measured, no green branch). The header comment `:9-14` is rewritten. The second test
that encodes the reversed rule, `tests_inner_html_mixin.rs:620-635` (§0.2), flips no value: its assertion is true
under both rules, so it is renamed and its comment rewritten (§4.9) and it stays green throughout.

**Signature-only changes.** `apply_clone_creation_ce_semantics` tests (`upgrade.rs:391`, `:416`, `:427`, `:452`) and
`apply_clone_form_state` tests (`clone.rs:195`, `:215`, `:247`, `:269`, `:285`, `:297`) pass `CloneScope::Subtree`.

**Stay green, unchanged** (the "cloneNode as today" guard, umbrella §5.2a): `tests_clone_form_state.rs` (11),
`tests_custom_elements.rs:567-587`, `tests_create_element_options.rs:219-` (the three clone cases), every test in
`node_methods/tests/clone.rs` (720 lines; among them the `ClonedFrom` marker tests `:72-134`, the template tests
`:135-245`, the CE identity tests `:471-595`, the null-registry tests `:618-687`), the rest of `clone_shadow.rs`
(the stamp guards `:186-214` and `:244-277` among them).

**New tests.**

- `elidex-ecs`, `tests/mutation_hook_fire_helpers.rs` (the `fire_*` group, moved there by RC-2a-pre; D-1):
  `fire_clone_created_helper_routes_to_hook` — `fire_clone_created` records `MockEvent::CloneCreated { root, scope }`
  once per call, for both scopes (the `fire_split_text_helper_routes_to_hook` shape); and one line in the existing
  `fire_helpers_silent_when_no_hook_installed`: with no dispatcher installed `fire_clone_created` is a no-op (no
  panic).
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
- `elidex-js` end-to-end, all in `vm/tests/tests_clone_created.rs` (NEW; R1 F2 — `tests_custom_elements.rs` is 1320
  lines and is not touched; the file's name is the mechanism's, since every case below is observable only through
  the event's arms): (e) `host.cloneNode(false)` with a defined custom element at grandchild depth inside a
  clonable shadow tree runs its constructor (the CE check on the D14 control; a `SelfOnly` root whose shadow tree
  the walk visits); (f) `cloneNode(true)` and `cloneNode(false)` of a defined custom element each enqueue one upgrade
  (count = 2 with the original, as `tests_create_element_options.rs:219-237` already pins for `false`; the `true`
  twin is added); (g) a cloned `<template>` whose contents hold a defined custom element enqueues no upgrade for the
  content clone; (h) queue position (E2): `x.cloneNode(true)` followed by `document.body.append(y)` in one script
  runs x's clone constructor before y's `connectedCallback` — an order equal under both queue models (umbrella E2
  rule), read at the eval checkpoint; (i) the I3 case stays as is (`tests_clone_form_state.rs:48-56`).
- `elidex-ecs`, `tests/dispatch_guard.rs` (D-4, §4.7): (w1) a clone fired inside a take-window panics in debug
  (`#[cfg(debug_assertions)]`, `#[should_panic(expected = "dispatcher is taken out")]`); (w2) a take on an unbound
  `EcsDom` opens no window, and the fire stays silent; (w3) a restored dispatcher closes the window and receives
  the event. (w1) is the check's negative control: it is red (no panic, test fails) at any head where the assertion is
  missing.

**Coverage statement (CLAUDE.md "Supported-surface testing").** The `cloneNode` surface is guarded by the VM
end-to-end tests above plus the engine-independent unit tests in the three crates; D14 by its negative control; the
mechanism by the recorder and probe tests, D-4 by its three window tests. No WPT subset is claimed (umbrella §5
preamble: the program's coverage is engine-independent unit tests plus VM tests); none is added here.

## §7. 1000-line audit (CLAUDE.md "1000-line debt")

Population (R2 G5): the files this PR edits — §5.1's table, measured there with `wc -l` at `fab44a1d` (not "the
files the plan names"; `teardown.rs` joined the edit set in Rev 3, G1, and is in the command and the table). Among
the files the umbrella listed as possible, one is over 1000, `crates/script/elidex-js/src/vm/tests/tests_custom_elements.rs`
(1320), and RC-2a does not touch it (R1 F2; §5.1). Of the edit set, one crosses 1000 with RC-2a's addition:
`crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs`, 971 at `fab44a1d`, plus the `MockEvent` variant (4 lines,
the file's one-field-per-line variant style, `:17-52`), the `MockHook` arm (4), the routing test
`fire_clone_created_helper_routes_to_hook` (about 24: the `fire_split_text_helper_routes_to_hook` shape, `:886-909`,
with a two-element expected `vec!`) and one line in `fire_helpers_silent_when_no_hook_installed` → about **1004**
(R2 G7; counted against the file's own shapes, not Rev 2's "about 10"). The next largest, `dom/mod.rs`, is 926 and
gains about 35 (D-2).

**The split (RC-2a-pre; cohesion, not timing).** The seam is real and named: the `fire_*` helper group
(`mutation_hook.rs:886-949`, 64 lines — `fire_split_text_helper_routes_to_hook`,
`fire_normalize_merge_helper_routes_to_hook`, `fire_helpers_silent_when_no_hook_installed`) is the one scenario in
the file that tests the `EcsDom::fire_*` helpers other crates call, rather than the mutation primitives, and RC-2a's
recorder tests are of that scenario
(D-1). So the group moves, as a standalone pure-move prerequisite before RC-2a's implementation (RC-0a's shape,
#587): `:886-949` to `tests/mutation_hook_fire_helpers.rs` (NEW, about 70 with its module doc and `use` lines),
`MockEvent` / `MockHook` / `install_mock` to `pub(super)` (`:17`, `:55`, `:152`; visibility only — the recorder stays
the one fixture of the whole stream), `mod mutation_hook_fire_helpers;` in `tests/mod.rs`. After it `mutation_hook.rs`
is about 907; RC-2a's variant and arm put it at about 915 and the sibling at about 95. The other seams in the file
stay for whoever adds a scenario there next: the slot-method tests `:434-485`, the shadow-root-silence group
`:671-788`, the `replace_text_data` group `:788-885`. C0c's `Adopt` arm in the recorder (its memo at wt-c0-plan
`05142d8c`, `:270`) is a few lines on the fixture, not a scenario, and fits at about 915; C0c is told in §9. For
`dom/mod.rs` the seam is its mutation-dispatcher section (`:160-274`, plus the slot and guard `:58-129` after D-4) to
`dom/dispatch.rs` (NEW), to land as a standalone pure-move split first if the re-measure at the actual base (D-2:
C0b or C0c landed) would put the file at 1000 with RC-2a's addition. `range/live.rs` is 500 after RC-0a. The
umbrella's §7 rows for RC-2a are thereby discharged at this base.

## §8. Out of scope, and slots

- `#11-attr-clone-node` (registered 2026-11-03 at #583, PM 316): not folded in (D-6).
- `#11-processing-instruction-clone` (registered 2026-11-03 at #583, PM 316): the stub stays.
- `#11-document-null-registry-adopt-reset` (open, registered 2026-11-03 at #583, PM 316; re-sequenced after C1's
  landing, PM 328): the walk-shape interim is kept unchanged; no new divergence (§4.10).
- `#11-ce-element-queue-grouping` (registered) and `#11-ce-reactions-method-scope-drain` (registered 2026-11-03 at
  #583, PM 316): the queue model and the drain point are untouched; the new end-to-end order test asserts only an
  order equal under both models.
- `#11-input-dirty-checkedness-flag` (pre-existing, `elidex-form/src/clone.rs:122-126`): untouched.
- `#11-clone-cloning-steps-event` (closed by #466): stays closed; the PM may annotate it (umbrella H5).
- The node document of clones and the `:282-285` stamps: C0b's (H9); C1 rebases onto this PR (umbrella §9).
- No new slot is proposed by this memo.

## §9. Edges, hand-offs, definition of done

**Edges.**

- **RC-0a (landed, `fab44a1d`)**: this memo cites the split files.
- **RC-2a-pre (this memo, D-1, §7)**: the standalone pure-move split of `mutation_hook.rs`'s `fire_*` group, landed
  before RC-2a's implementation starts (RC-0a's shape, #587). Its edit set is `mutation_hook.rs`,
  `tests/mutation_hook_fire_helpers.rs` (NEW) and `tests/mod.rs` of `elidex-ecs` (§5.1); no other lane's plan edits
  them. If C0c lands first with its `Adopt` arm in the recorder, the move is unchanged: the arm is in the fixture,
  which stays.
- **C0b (Layout, unlanded)** on `node_methods/clone.rs` and on `elidex-ecs` (R2 G9): H9 decides which stamps each
  side removes; at `fab44a1d` the RC-2a-first state applies (D-5), and the PR body records the state of its base.
  C0b also changes `begin_detached_fragment` to take `context` and `finish_detached_fragment` to drop it
  (`teardown.rs` and both parser callers) and adds a test-only walk-counter field to `EcsDom` and `new()` in
  `dom/mod.rs` (its memo at wt-c0-plan `a50b6d12`, `docs/plans/2026-11-eh-attrs-c0b-node-document-fact.md:127-137`,
  `:326-327`, `:487`, `:545`) — the struct and constructor D-4's enum also edits, and the window callers whose
  restore form D-4 rests on. If C0b lands first: re-measure `dom/mod.rs` and `teardown.rs` at that base (D-2) and
  re-read the two parser call sites for the restore form (`if let Some(d) = saved { set_mutation_dispatcher(d) }`,
  no early return between take and restore; §0.2).
- **C0c (Layout, unlanded)** on `MutationEvent`: H6; whichever lands second re-runs §5.2 and adds its arm beside
  the other's. C0c also edits `walkers.rs` and the re-exports `dom/mod.rs:32` / `lib.rs:44-47` (its memo at wt-c0-plan
  `05142d8c`, `:53-55`, `:213`, `:267`, `:366`), the files of §4.1 / §4.3 here. If C0c lands first, RC-2a re-measures
  `dom/mod.rs`, `walkers.rs` and `lib.rs` at that base (D-2) and places `for_each_clone_created` and the re-export
  beside C0c's additions. If C0c adds a window (a new take-and-restore span), D-4's assertion covers it with nothing
  to register (§2 I × J).
- **C1 (Layout)**: depends on all of RC-2a and rebases onto it (umbrella §9). Layout is told at landing (umbrella H2).
- **RC-1**: rebases its `range/live.rs` edit onto the census arm (D-8).
- **RC-2b**: consumes the event; calls the entry once per clone root from the Range natives (a bound VM, no window);
  its clones run under D-4's assertion like any other, so its plan states no reachability claim of its own.

**Hand-offs** (for the PM; the memo edits neither the ledger nor Layout's files): close
`#11-clone-shadow-children-subtree` at landing with the PR number and squash SHA; optionally annotate the closed
`#11-clone-cloning-steps-event` (§4.8); tell Layout that RC-2a has landed (H2), that `MutationEvent` has its
`CloneCreated` arm in the sites of §5.2 (H6), and — for C0c's recorder arm — that since RC-2a-pre the recorder
(`MockEvent` / `MockHook` / `install_mock`) is `pub(super)` with the `fire_*` group in
`tests/mutation_hook_fire_helpers.rs`, and `mutation_hook.rs` is at about 915 after RC-2a (D-1, §7).

**Definition of done** (every command re-runnable at the PR head):

1. D14 negative control: `cargo test -p elidex-dom-api --all-features clone_node_shallow_replicates_own_clonable_shadow`
   is red at the pre-fix head with the inverted assertion and green at the PR head (the PR body records both runs).
2. One clone path, and no public seam: `git grep -n 'enqueue_subtree_upgrade_reactions\|apply_clone_creation_ce_semantics\|apply_clone_form_state' -- crates`
   shows, outside `crates/dom/elidex-custom-elements/src/upgrade.rs` and `crates/dom/elidex-form/src/clone.rs`
   (each: the `pub(crate)` definition, its in-crate arm call and inline tests), only doc prose that names the arms;
   no `pub use` of either and no call from any other crate (today: the VM's 5 hits and the two re-exports, §0.2,
   §4.4, §4.5).
3. One emitter: `git grep -n 'fire_clone_created(' -- crates ':!*tests*'` shows the definition in `dom/mod.rs` and one
   call, in `clone_node_with_shadow_honor`; the definition's body calls `dispatch_event`.
4. The variant's sites: `git grep -n 'MutationEvent::CloneCreated' -- crates ':!*tests*'` shows the helper, the CE arm,
   the form arm and the `range/live.rs` no-op arm, nothing else (plus the recorder under `tests/`).
5. One walk: `git grep -n 'for_each_clone_created' -- crates ':!*tests*'` shows the definition and its calls in
   exactly two files, `elidex-custom-elements/src/upgrade.rs` and `elidex-form/src/clone.rs`, one call each (verified at the head by this grep); neither file calls
   `for_each_shadow_inclusive_descendant` on the clone root (`git grep -n 'for_each_shadow_inclusive_descendant' --
   crates/dom/elidex-custom-elements/src/upgrade.rs crates/dom/elidex-form/src/clone.rs` → nothing).
6. Cloner census: `git grep -n 'clone_node_with_shadow_honor\|"cloneNode"' -- crates` matches §0.2's 13-line list
   (one entry, one production caller; the handler's `method_name` string `clone.rs:88` among them).
7. Census of exhaustive matches: §5.2 re-run at the head (`cargo build --workspace --all-features` and
   `cargo test -p elidex-ecs` compile every match).
8. The slot (D-4; R2 G6 — each item is a command with its exact expected output, re-runnable at the head):
   (a) `git grep -n -P '\.dispatcher\.(is_some|is_none|take|replace)\(|\.dispatcher\s*=\s*None\b' --
   crates/core/elidex-ecs/src` → **no output** (every `Option` idiom on the field is gone; the enum refuses them at
   compile time, and the grep is the re-runnable record of that);
   (b) `git grep -n -P '\.dispatcher\b|^\s*dispatcher:' -- crates/core/elidex-ecs/src` → lines in exactly three files,
   `dom/mod.rs`, `dom/tree/teardown.rs`, `dom/tests/dispatch_guard.rs` — the file set of §0.2's slot access census at
   `fab44a1d` (no reader was added elsewhere);
   (c) `git grep -n 'Option<Box<dyn MutationDispatcher' -- crates/core/elidex-ecs/src/dom/mod.rs` → **exactly three
   lines**: the return type of `set_mutation_dispatcher`, the return type of `take_mutation_dispatcher`, and
   `RestoreDispatcher`'s `pending` field (the `EcsDom` field is `dispatcher: DispatcherSlot`);
   (d) `cargo test -p elidex-ecs --all-features dispatch_guard` → `test result: ok`, with the three window tests (w1-w3)
   and the nested-fire test among the names passed (w1 is `should_panic`);
   (e) `git diff --stat origin/main...HEAD -- scripts .claude/tools` → **no output**.
9. `cargo test -p elidex-ecs -p elidex-dom-api -p elidex-custom-elements -p elidex-form -p elidex-js --all-features`
   green; the new tests of §6 present; `mise run ci` green before push.
10. The hand-stamp state of the base recorded in the PR body (H9); the stamp guards green.
11. The citation sweep (umbrella §6) run on the files of §5.1, hit counts and dispositions in the PR body.
12. The #466 reversal reviewed in this plan's `/elidex-plan-review` and the outcome recorded in §10 of this memo
    (umbrella R6-3).

## §10. Open questions

- Rev 1's Q-A (whether the trip-wire belonged in this PR or in a CI/tools PR) is moot: D-4 is a check inside
  `elidex-ecs` (Rev 2), with no tooling and no `REQUIRED_WIRES` change, so it lands in this PR as code and tests.
- **The #466 reversal (DoD 12).** R1 (2026-11-04) reviewed §4.8 and asked for its wording to be exact (F5: I3's
  precondition, the F × I pair, "marker over pairs" on its own reason, the depth-cap residual), not for the
  reversal to be withdrawn; applied in Rev 2. R2 (2026-11-04, full 5-axis after F1): no finding on §4.8 beyond the
  residual's exactness (G11, applied in Rev 3); the reversal stands as reviewed. R2's one IMP (G1) and 24 MIN
  (G2-G14) are applied in Rev 3; Step 5: a focused re-review of the Rev 3 delta.
- No PM question. Every item the umbrella left to this plan is decided in §0.4.
