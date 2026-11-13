# T3 `EH-attrs-at-creation-T3` — *determine the target of an event handler* follows the spec (per-PR plan-memo)

Nested PR **T3** of the standalone program `EH-attrs-at-creation` (umbrella
`docs/plans/2026-10-event-handler-attrs-at-creation.md`, approved by #551 `b9122157` at rev 8, carried in place to rev 8.7 on main (#570, then C0a #595, C0b #608, C0c #611
and C1 #626, each by its own entry); this PR's in-place edits are its *Rev 8.<t>*, PM decisions 129, 131, 153, 168, 699, 705, 711, 713, 714; "the umbrella"). It closes slot
`#11-event-handler-target-determination` (all four facets) and facets (1) and (3) of
`#11-event-handler-content-attribute-applicability` (PM decision 168). By PM decision 129 it also takes from slice b's
S-5 the part that ends X-W for the pairs HEAD targeted at the element, and by PM decision 153 it takes from slice b's
S-1 the clone path (D-T10), by PM decisions 699, 705 and 711 the fragment path (D-T12; DOMParser's build, declarative
shadow content and template contents a fragment parse creates included), both through one creation event (§4.5). It
is the prerequisite x3 of the umbrella's slice b, and lands after L3's ecs-entity-lifetime PR (io) (umbrella x12; PM
decisions 711, 714). Siblings cited by ID: slice a
(`docs/plans/2026-10-eh-attrs-slice-a.md`, landed by #563 `3cd84a04`; "the slice-a plan"), IBP-sandbox N1
(`docs/plans/2026-10-ibp-sandbox-handler-gates.md`, rev 4.5; implementation landed by #580 `27b50dfe`; "the N1 memo"),
the IBP-sandbox node memo (`docs/plans/2026-09-ibp-sandbox.md`; "the node memo"), and four sibling plans, all landed
on main (rev 4.5; rev 4.4 read them on their plan branches), cited by their own IDs:
- **C0**, a program of three PRs C0a → C0b → C0c (slot `#11-cross-document-adopt-on-insert`; PM decisions 143, 244):
  program memo `docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md` ("the C0 memo"; its decisions D-n name their PR,
  e.g. "C0b D-1") and per-PR memos `docs/plans/2026-11-eh-attrs-c0{a,b,c}-*.md`; landed: C0a #595 `4893df0f` (with
  the program memo), C0b #608 `062337aa`, C0c #611 `0a5a77ba`, after the C0 memo's D-14 (a)–(c) splits (#593, #599,
  #600) and the CI/tools default-features check #585 (`115a667c`);
- **C1** (slot `#11-template-contents-owner-document`, branch A, PM decision 123): plan
  `docs/plans/2026-10-eh-attrs-c1-template-owner-document.md` ("the C1 memo"; plan #620 `8344af2b`, implementation
  #626 `cee74989`); its prerequisites are C0, L3's RC-2a (`docs/plans/2026-11-range-contents-program.md` row RC-2a; "the
  Range program"; per-PR plan `docs/plans/2026-11-range-rc2a-clone-created.md`, on main by #589 `4fb8c195`, "the RC-2a
  plan", cited by section; implementation landed by #596 `17b2a84d`) and C1-pre;
- **C1-pre** (Document wrapper, slot `#11-bridge-document-post-install`, PM decision 328): plan
  `docs/plans/2026-11-eh-attrs-c1-pre-document-wrapper.md` ("the C1-pre memo"; landed with its implementation, #616
  `c7cce8c8`), after its prerequisite **C1-pre0** (the `vm/globals.rs` pure-move split, C1-pre memo D-11; branch
  `vm-globals-split-c1-pre0`; landed by #592 `f9cf8c40`).

**Placement (evidence).** T3 sits under the **EH-attrs umbrella**, not under IBP-sandbox:
- umbrella §1.2 nested-PR table, row **T3** `EH-attrs-at-creation-T3` ("owner: Layout lane, this program; PM decision
  25"); edges x3 (T3 → slice b, semantic precondition), x6 (slice a → T3, scheduling) and x11 (RC-2a → T3, semantic
  precondition, rev 8.<t>: D-T10) in §1.3; §8.1's first bullet;
- the ledger row (`project_open-defer-slots.md`, section "Event handlers — pre-existing slot", the sub-bullet **"Owner
  re-routed 2026-10-25 (PM decision 25)"**): "the owner is now the EH-attrs-at-creation program (Layout lane). T3 is a
  standalone prerequisite PR of slice b, outside the IBP-sandbox DAG". The row's first paragraph still reads "Owner:
  the IBP-sandbox program"; the sub-bullet supersedes it (§8.3 item 1 folds it at T3's landing);
- the N1 memo §7 row "*determine the target of an event handler*": "reassigned by PM decision 25 to the EH-attrs
  program, whose T3 PR carries facets 1, 3 and 4".

**Scheduling (PM decision 153, user, 2026-10-31).** Order: **N1's implementation → C0 → C1 → T3 → slice b**; inside C0,
C0a → C0b → C0c (PM decision 244). The option text the user saw: 「順番を N1 → C0 → C1 → T3 → slice b にして、R-A と R-B は T3 の前に閉じます。R-C（clone）は slice b の S-1 の該当部分を T3 に移して閉じ、R-D（SVG の body）も T3 の中で閉じるかを designer に検討させます。閉じられないものが残ったら、改めて伺います。T3 は大きくなり、入るのも遅くなります。」
**T3's base** ("the base") is main when T3's implementation branch is cut, after C1 lands: `4fd85f4e` (N1's
implementation and #585 landed) plus C0a, C0b and C0c with their D-14 splits, RC-2a (also a direct predecessor of T3,
edge x11, PM decision 359: D-T10), C1-pre0, C1-pre and C1. All of them are on main at `cee74989` (#626, C1), the
first main commit with the base's content. K-0's
"HEAD" for T3 is the base (§1.2). This memo's code coordinates are `cee74989`'s (rev 4.5; rev 4.4's were `4fd85f4e`'s)
and are re-run at the commit the branch is actually cut from (§9 item 1).

**Scope.**
- One function realises *determine the target of an event handler* (WHATWG HTML §8.1.8.1 Event handlers), steps 1–4,
  and every caller the spec gives it calls that function: the attribute change steps (content attributes, at every
  processing timing), the event handler IDL attribute getter and setter (both VM accessor pairs). Facets: (1) step 3,
  a node document that is not an active document → null; (2) step 4, the node document's relevant global object; (3)
  step 1's `frameset`; (4) step 2's Window-reflecting body element event handler set. Step 1's class test is
  HTML-namespaced (D-T5).
- **PM decision 129 (user, 2026-10-31; O-1 of rev 1):** X-W, slice a's rule that a Window-delegating pair is re-raw'd
  at every insertion, ends in T3 for the pairs HEAD targeted at the element: `body` × the six Window-reflecting names,
  and `frameset` × all 24 Window-delegating names. Those pairs are handed to the spec target at the spec's processing
  points (D-T7). The option text the user saw: 「HEAD より悪い状態が main に載らない。T3 は大きくなり、slice b の設計を少し先取りする。plan-review で確かめる」.
  X-W keeps only the pairs HEAD already targeted at the Window (`body` × the 18 WindowEventHandlers names) until slice
  b's S-5. The umbrella's slice-b scope is reduced to match, in place.
- **PM decision 153 (R-C):** the **clone path of S-1** — an element created by *clone a node* has its handler content
  attributes processed at its creation, every pair class, shadow-including and template contents included (D-T10).
- **PM decision 153 (R-D):** an HTML-interface choice for `body` that reads the namespace (D-T11), so a
  foreign-namespace `body` no longer carries `HTMLBodyElement.prototype`'s WindowEventHandlers accessors.
- **PM decision 168 (O-7 (a)):** the local-name half of the attribute change steps' step 1 ("localName is not the name
  of an event handler content attribute on element"), i.e. applicability facets (1) and (3) (D-T8). Facet (2), the
  attribute-namespace half, stays with slice b.
- **PM decision 699 (O-10 (a)):** the **fragment parse's creation path** for an element whose node document at its
  creation is the active document: a fragment parse creates its elements in the context's node document (P-T17), and
  *determine the target of an event handler* steps 3–4 ask only whether that document is active, not whether the
  element is connected, so a fragment-parsed `body` / `frameset` in an active-document context delivers its
  Window-delegating pairs to the Window at its creation (D-T7).
- **PM decision 705 (O-11 → S-i × C-ii):** the **whole fragment path of S-1** — every element a fragment parse
  creates has its handler content attributes processed at its creation, every pair class, through one consumer arm on
  one creation event: `MutationEvent::CloneCreated` generalised to `MutationEvent::SubtreeCreated { root, cause }` (no
  new variant beside it; all consumers migrate). D-T12; the event: §4.5. (Until rev 4.8: "every element a fragment
  parse creates, in the insertion-time walk set".)
- **PM decision 711 (overrides rev 4.7's calls 1 and 3; PM decision 713 settles its mechanism).** (1) A creation event
  of a fragment parse that runs while the dispatcher is taken (DOMParser's build) is **queued and dispatched at the
  outermost dispatcher-take window's close**, not dropped (§4.5 item 4), so DOMParser's build is processed at
  creation too and leaves slice b. (3) The creation-time walk set is **every node the parse created**: shadow-including
  (declarative shadow content) and template contents included, expressed by L3's `WalkScope::WithTemplateContents`
  (§4.5 item 6; PR (io)). (4) O-12 closes as an L3-owned slot (§10). (5) the umbrella §3 step number. (6) H-T31's census.
- **PM decision 713 (L3's review of §4.5, with PM rulings):** the queue lives in `DispatcherSlot::Taken` (World-level
  state), the outermost close restores then flushes in order skipping dead roots, an unwind discards it (§4.5 item 4);
  the nested flush follows the placement (§4.5 item 3); the fragment walk no longer shares `handle_insert`'s collector
  (D-T12); flushed creation events set raw uncompiled handlers on elements of documents without a browsing context,
  and compilation is deferred (D-T13, with `inert_document.rs`'s rustdoc corrected); the O-12 slot names the
  registry-less side (§8.1 item 5).
- **PM decision 714:** L3's PR (i) is split into PR (ig) (the dispatcher-take guard and the four windows) and PR (io)
  (`OwnedEntity`, `WalkScope`, `owned_entities`), in the order is → ig → io → ii. T3 depends on ig's outermost close
  (the flush point) and on io's `WalkScope`, so T3 lands after io (umbrella x12).

**Not in scope.** X-W for `body` × WindowEventHandlers names, and the D-8 restriction (both slice b's S-5); the other
creation paths of S-1 (declarative shadow content and template contents created by the pre-bind document parse, whose
creation-equivalent point is the first bind, umbrella K-6, S-2; slice b; until rev 4.8 the list also held the DOMParser
build, and declarative shadow content and template contents created by a fragment parse); the custom-element
consumer's fragment cause (§8.1 item 5, L3); insertion adoption (C0) and template contents' owner document (C1), both landed before T3; `document.body`'s class
test (PM decision 133, D-T5); the attribute change steps' step 1 namespace test (applicability facet (2), slice b,
D-T8); the `HTMLFrameSetElement` interface (`#11-htmlframesetelement-interface`, compat, PM decision 130); the
foreign-element interfaces other than `body`'s (§8.1 item 4); multi-realm (B1; §8.2 hand-off).

**Changelog.** *Rev 4.8.1* (2026-11-13): re-anchor only, on L3's split (PM decision 714) and PM 718's citation rule.
L3's names are cited by symbol and section from the PR (ig) plan, the PR (io) plan and the L3 umbrella, read at one
pre-ratification snapshot (§0.1, H-T32 rewritten); premise added: every L3 citation is re-pinned to L3's ratified
sha before T3 is ratified (R7, §9 item 1). P-T20 answered: ig has a consuming `DispatcherTake::close` and guarantees
(a) and (b) (its §8 *T3 intersection*), so §4.5 item 4 relies on them plus T3's change to `close` instead of stating
a requirement on ig; one residual stays open, the shape of the flush against ig's `Drop` as the one restore site
(P-T20 *Residual*, for the PM with L3). §4.5 item 4 uses no `is_outermost`. R7's stop condition and §9 item 1 are
narrowed to what remains; the umbrella's x12 row cites the split plans.
*Rev 4.8* (2026-11-13): PM decisions 711, 713, 714. (711 (1), 713 (2)–(3)) A `FragmentParse`
creation event fired while the dispatcher is taken is queued on `DispatcherSlot::Taken { pending }` and dispatched at
the outermost window's close (restore first, in order, dead roots skipped; discarded on unwind), so DOMParser's build
is processed at creation and leaves slice b; the nested flush follows the placement (§4.5 items 3–4; the Clone-cause
D-4 assertion stays). (711 (3), 713 (4)) The creation walk is every node the parse created, L3's
`WalkScope::WithTemplateContents` (declarative shadow content and template contents included, uncapped); it no
longer shares `handle_insert`'s collector (D-T12, §4.5 item 6); slice b's S-1 keeps only the pre-bind parse's
declarative shadow content and template contents. (713 (5)) New D-T13: a creation in a document without a browsing
context sets the raw handler and defers compilation (N1's gate); `inert_document.rs`'s rustdoc is corrected (P-T19,
H-T33, §5.3). (713 (1), 714) §4.5 item 5's anchors are a premise re-pulled at ig's head; T3 lands after io (umbrella
x12); L3's names from PR (i) plan @ 7ab6f87d (§0.1, H-T32); P-T20 (no consuming close in that snapshot). (711 (4),
713 (6)) O-12 closed as an L3-owned slot (§8.1 item 5; draft handed to the PM). (711 (6)) H-T31's census states what it
counts. Rows XW-12, EW-10, OR-1 (timing), new OR-4 to OR-6; cells T-22c and U-T9 to U-T11 rewritten, new T-26 to
T-28; §6.4's mutants for the queue and the walk; §1.1, §2, §3, §4.1–§4.4, §5, §7, §8.2, §8.3 item 11, §9 items 1, 3, 4.
Umbrella (*Rev 8.<t>*): §3's *create an element for a token* step (17.2 → 21; PM decision 711 (5)), slice b's scope
back to the whole fragment path (§0.6 S-1, §1.2, §5.1, §10.2, §11's O9 line), x4 and new x12.
*Rev 4.7* (2026-11-13): PM decision 705. O-11 → S-i × C-ii (§10): the whole fragment path is T3's
(new D-T12), processed at creation through one consumer arm on one creation event; `CloneCreated` is generalised to
`SubtreeCreated { root, cause }` with every consumer migrated, stated once in the new §4.5 (variant, fire points,
causes, walk sets, consumer migration, the D-4 assertion while the dispatcher is taken, the take / restore sites relied
on), which the other sites reference (§0.1's naming note, D-T7, D-T10, §3, §4.1–§4.4, §5.3, §8, DoD 4). Rev 4.6's
premise that the fragment build's dispatcher take keeps an event from reaching the consumer is corrected (P-T18: the
build restores it before its caller places the nodes; only DOMParser's enclosing window keeps it taken). Rows XW-15,
OR-1, new OR-3; cells T-25, U-T9 to U-T11; T-22c's DOMParser / inert-template reading; §6.4's O-10 (b) row (no killer
exists: no production fragment parse creates without placing). Rejected and recorded: C-i, S-ii, S-iii, C-iii. New
open item O-12 (the custom-element consumer and the fragment cause; not decided). Umbrella
(*Rev 8.<t>*): the §3 row "§13.4 step 4 … throwaway document", edge x3's and §9 N-4 (b)'s "fragment-throwaway
`<body>`", slice b's scope (§0.6 S-1, §1.2, §5.1, §10.2, §11's O9 line).
*Rev 4.6* (2026-11-13): PM decision 699. O-10 → (a): T3 processes a fragment-parsed element at its
creation when its node document is active (Scope; D-T7; rows EW-20, XW-15, EW-21; cells T-22, T-22b, T-22c, T-23, T-24;
§1.1, §1.2's R-E, §2, §6.4, §7, §8.2, §9 item 3); (b), (c), (d) recorded as rejected (§10). P-T17 resolved; the rev
4.5 "premise broken" flags in D-T7, §1.2, §2.1 and §7 are removed, and PM decision 129's premise ("0 rows worse than
the base") and the umbrella's G3 line hold with (a) included. New open item O-11 (which part of the fragment path,
and its mechanism). The umbrella revision number is assigned at landing (§8.4 now points at the umbrella's rule).
*Rev 4.5* (2026-11-13): rebased onto `cee74989` (C0a–C0c, RC-2a, C1-pre and C1 landed); every code
coordinate re-measured there (§0.1's frame, §0.2's commands and results; landed sibling symbols lose "NEW"); D-T3,
§2 A4×A5, §3, §4.1, §4.2, §5.3, §8.2 and DoD items 1 and 4 follow PM 441 and PM 455 (`HostData::active_document()` is
`ActiveDocument`'s one constructor, in either landing order with the event-path PR-B, and `Vm::bind` and the VM
accessors' helper both go through it); DoD 4's `window_entity` trait item is `HostDriver`'s, not `ScriptEngine`'s;
D-T4's quoted ground follows C0b's landed `scripting.rs`; new premise error P-T17 (D-T7's fragment-parse premise)
with open item O-10, flagged in place, D-T7 unchanged.
*Rev 4.4* (2026-11-05): re-check N1–N7 (PM 380); plan-review closed. T-16c's mutant, cell T-16d,
U-T6's marked-Creation row, §6.5's run-through; H-T27 confirmed at runtime.
*Rev 4.3* (2026-11-05): plan-review R1 fixes F1–F16 (PM 377). F1: the `CloneCreated` arm is the
canonical writer of a clone's handler state; a clone-internal `Insert` writes a subset of it with equal values (D-T10
*Canonical writer*, row CI-1, cell T-16c).
*Rev 4.2* (2026-11-05): O-9 closed per PM 367 (wording).
*Rev 4.1* (2026-11-05): PM decision 359. O-8: D-T10's placement changes from a VM call beside the
`cloneNode` post-pass to a `CloneCreated` arm of `EventHandlerAttributeConsumer` (RC-2a's event; edge x11), and the
sites that rested on the old placement follow (H-T24, H-T25, P-T15, D-T3, §2 A4×A5, §4, §5.3, §5.4, §6, §8, §9).
Q-15: the clone walk is RC-2a's, in *clone a node* order; cell T-16b pins it. O-9 closed (PM 367, §10).
*Rev 4* (2026-11-04): PM decision 168 (O-7 = (a)): T3 takes applicability facets (1) and (3) (D-T8,
`is_event_handler_content_attribute_of`); T-21 and U-T8 are red cells; §2.1 has 0 rows worse than the base; residual
R-D2 and §2.2 are gone. Scout answers Q-7, Q-8, Q-10 to Q-13 and the driver's Q-14 are folded in (H-T7, H-T8, H-T16,
H-T18, H-T21, H-T24, H-T25, H-T27, H-T28, D-T4, D-T11): D-T4 is (ii), C0b's `node_document` (its (i) branch and U-T5 are
deleted); C19b, C19c, P3 and T-6b carry "static; confirmed red-first at implementation". The C0 IDs of rev 2
("C0-1".."C0-4") become the C0 memo's (C0b D-1, D-4, D-5, D-7 (a), D-10, D-11, D-20); the base adds C0a–C0c, RC-2a, C1-pre0
and C1-pre; the frame moves to `4fd85f4e` (re-measured: H-T6, H-T7, H-T10, H-T15, H-T16, H-T18, H-T21, H-T25, §5.2,
§5.3). New premise errors P-T14 to P-T16; new open items O-8 and Q-15 (§10). The umbrella edits drop the C0 and C1 rows,
the order line and the order-consequence edits (C0a's and C1's, PM decision 223 O-1 as amended by 271; §8.4) and carry
one revision label, 8.<t>. History and dispositions of rev 1 to rev 3 are not restated:
`git show c8446b09:docs/plans/2026-10-eh-attrs-t3-target-determination.md` (rev 3; rev 2 `43aa3fd2`, rev 1 `ad29b9ef`).

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every code `file:line` is a **`cee74989`** coordinate (origin/main, #626; rev 4.5 — rev 4.4's frame was `4fd85f4e`,
#588), measured on the worktree with
`git grep -n -E '<pattern>' cee74989 -- <paths>`, `git show cee74989:<path> | sed -n <range>p`, or `/usr/bin/grep -n`
(never the default ugrep). Spec text is `.claude/tools/webref body` output of 2026-10-31 (MathML Core and SVG2 rows
re-read 2026-11-04); every `§X.Y = title` pair is `webref heading` / `dfn` output (§3's footnote). Paths are abbreviated
after first use: `session/…` = `crates/script/elidex-script-session/src/…`, `elidex-js/…` =
`crates/script/elidex-js/src/…`, `ecs/…` = `crates/core/elidex-ecs/src/…`. C0, RC-2a, C1-pre0, C1-pre and C1 are in
the frame (landed); a commit landing between `cee74989` and the cut can move lines, so every coordinate is re-run at
T3's base (§9 item 1).

**Base outcomes.** Where a *Base* cell of §2.1 or §6 differs from `4fd85f4e`'s outcome, the difference is C0's or C1's
own change, owned by that PR's plan (its K-0 lens), and is marked † with its source. T3's verdict compares T3 with the
base and with the spec only.

Symbols written as `Type` + method (`EcsDom::window_entity`, `HostData::window_entity`, `HostData::dom_shared`,
`test_support::install`, `EventHandlerAttributeConsumer::handle`) name existing methods or functions inside `impl` /
`mod` blocks, which preflight's literal symbol grep does not find (its soft warnings). `EcsDom::node_document` (C0b
D-1), `EcsDom::node_document_for_new_nodes` (C0b D-10), `EcsDom::appropriate_template_contents_owner_document`
(C1) and `MutationEvent::CloneCreated`, `CloneScope` (with `CloneScope::Subtree`, `CloneScope::SelfOnly`) and
`EcsDom::for_each_clone_created` (RC-2a) are the sibling plans' names, landed (on `cee74989`; rev 4.4 marked them
NEW). **Naming (rev 4.7, PM decision 705):** T3 replaces `MutationEvent::CloneCreated { root, scope }` by
`MutationEvent::SubtreeCreated` (NEW) `{ root, cause }` with `cause = CreationCause::Clone(scope)` for a clone and
`CreationCause::FragmentParse` (NEW) for a fragment parse, fired by `EcsDom::fire_subtree_created` (NEW) (§4.5); where
this memo names `CloneCreated` or "the `CloneCreated` arm" for T3's own code it means that cause of `SubtreeCreated`.
`CloneScope` and `for_each_clone_created` keep their names.
**L3's names (rev 4.8, PM decisions 711, 713, 714).** L3's ecs-entity-lifetime PR (i) is split into PR (is), PR (ig)
and PR (io), landing is → ig → io → ii (PM decision 714; umbrella §5 *Order*); T3 lands after io. Their names are
cited by symbol and section from L3's split plans, "the PR (ig) plan"
(`docs/plans/2026-11-ecs-entity-lifetime-pr-ig-dispatcher-take-guard.md`), "the PR (io) plan"
(`docs/plans/2026-11-ecs-entity-lifetime-pr-io-owned-entities-walk-scope.md`) and "the L3 umbrella"
(`docs/plans/2026-11-ecs-entity-lifetime-umbrella.md`), as read at the snapshot H-T32 names (pre-ratification; every
citation is re-pinned to L3's ratified sha before T3 is ratified, R7): from **ig**, `DispatcherTake` (PR (ig) plan
§4.1; the one way to take the dispatcher, held by all four take windows) and its consuming `DispatcherTake::close`
(§4.1, the explicit restore point on a returning path; its `Drop` the unwind backstop and the one site of the
restore); from **io**, `WalkScope` (`ShadowIncludingTree`, `WithTemplateContents`, `LifetimeUnit`), `OwnedEntity::in_scope`
(NEW: io; its exhaustive match) and `for_each_uncapped_shadow_inclusive(&self, root, scope: WalkScope, visit)`, which
replaces `TemplateContentsWalk` (deleted there) (PR (io) plan §4.1, §4.2). Where this memo writes
`TemplateContentsWalk::Include` for the base, T3's code passes `WalkScope::WithTemplateContents` (NEW: io) (the base's
`Include`, PR (io) plan §3.1 and §4.2). "ig's close" is `DispatcherTake::close` of the guard that took the dispatcher
(PR (ig) plan §4.1, *The restore point*); T3 relies on ig's two guarantees stated in the PR (ig) plan §8 *T3
intersection*, and adds the flush to that close (§4.5 item 4; P-T20). (Until rev 4.8.1: the names were cited from the
pre-split PR (i) plan.)

### §0.2 Premises re-measured

| ID | Premise | Command | Result at `cee74989` (rev 4.5; where it differs, the `4fd85f4e` state of rev 4.4 is kept in parentheses) |
|---|---|---|---|
| H-T1 | One target function, two content-attribute callers | `git grep -n 'resolve_handler_target(' cee74989 -- crates` | defined `session/event_handler_processing/mod.rs:249`; called by `process_handler_content_attributes` `mod.rs:130` (all three timings: Bind, Insertion, HandlerWrite) and by the consumer's AttributeChange arm `session/event_handler_consumer.rs:420`. Nothing else |
| H-T2 | HEAD's target rule | `sed -n 244,262p session/event_handler_processing/mod.rs` | `scope == HandlerScope::Window && is_body_element(origin)` → `dom.window_entity().unwrap_or(origin)`, else `origin` (`:249-255`); `is_body_element` is a tag-only, ASCII-case-insensitive `body` test (`:258-262`). So: no step 3; step 4 = the World's one `NodeKind::Window` entity (`ecs/dom/mod.rs:515`, a linear scan); no `frameset`; the reflecting six (`HandlerScope::Global` rows) stay on the element. C0 and C1 did not touch it (`git diff 4fd85f4e cee74989 -- crates/script/elidex-script-session/src/event_handler_processing/mod.rs`: one fixture rename, `:336`) |
| H-T3 | The IDL path determines its target separately | `sed -n 323,358p` and `sed -n 418,483p elidex-js/vm/host/event_handler_attrs.rs` | normal pair (`native_event_handler_get` `:323`, `native_event_handler_set` `:344`): target = the receiver, always. Body pair (`native_body_weh_get` `:439`, `native_body_weh_set` `:461`), installed on `HTMLBodyElement.prototype` for `HandlerScope::Window` rows only (`install_body_weh_delegation` `:207-214`, called once, `elidex-js/vm/host/html_passive_protos.rs:66`): target = `ctx.host().window_entity()` (`:448`, `:471`), no step 3; receiver check `require_body_receiver` (`:418-432`) is `tag_matches_ascii_case(entity, "body")`, tag-only. The reflecting six on a `<body>` use `HTMLElement.prototype`'s normal pair (`elidex-js/vm/host/html_element_proto.rs:134`) |
| H-T4 | Steps 1–2 already have one predicate, used by timings only | `sed -n 213,232p session/event_handler_processing/mod.rs` | `is_window_delegating_pair` (`:216`): `is_body_or_frameset` (`:228`, tag-only) ∧ (`HandlerScope::Window` row ∨ `WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET` `:42`). Read by `process_handler_content_attributes` (`:111`), `process_before_handler_write` (`:156`) and `is_only_deliverable_ordinary_name` (`:208`), **not** by `resolve_handler_target`. At HEAD the X-W classifier and the target rule disagree on the reflecting six and on `frameset` |
| H-T5 | The spec's three callers | `.claude/tools/webref body html event-handler-attributes` | the getter steps 1–2 (determine; null → return null), the setter steps 1–2 (determine; null → return), the attribute change steps steps 2–3 (determine; null → return). Step 1 of the attribute change steps: "If namespace is not null, or localName is not the name of an event handler content attribute on element, then return." Step 3's note: a `body` created in an active document but not connected still targets the Window |
| H-T6 | "Active document" in HEAD's model | `sed -n 12,40p session/scripting.rs`; `docs/plans/2026-06-agent-scoped-ecsdom-world.md` §5 req 4; H-T15 | `scripting_disabled_for_platform_object`'s docstring (`:35`): `bound_document` "is the active document of the engine's (single) browsing context". With H-T15: in one `EcsDom` the bound document is the only document with a browsing context, and the one Window is its relevant global object (`webref body html navigables`: a navigable's active window "will always equal the navigable's active document's relevant global object") |
| H-T7 | (Q-10) The node-document derivation is C0b's one reader | `sed -n 40,86p session/scripting.rs`; `git show cee74989:crates/core/elidex-ecs/src/dom/node_document.rs \| sed -n 80,101p`; the C0 memo D-1, D-7 (a), D-15 | C0b landed (#608): `EcsDom::node_document` (`ecs/dom/node_document.rs:89`: a Document → itself; any other node → its stored `AssociatedDocument` if that is a live Document, else `None`; "never derived from the node's position", so both tree-position fallbacks are gone, C0b D-1), and clause (b) of N1's compile gate is `dom.node_document(target).is_some_and(\|node_doc\| node_doc != bound_document)` (`scripting.rs:84-85`, docstring `:49-62`; C0b D-7 (a)). (At `4fd85f4e` clause (b) derived an effective node document inline: the composed tree root if a Document, else `owner_document`.) D-15 renames the node memo's planned `effective_node_document` to `node_document`, so N4 has no second home for it. The fact is kept true at creation (C0b D-10, D-11, D-20: every production creator takes a `NewNodeOwner`, `node_document.rs:12-23`) and at insertion (H-T8) |
| H-T8 | Insertion adopts | `webref body dom mutation-algorithms` (insert step "Adopt node into parent's node document"); `git show cee74989:crates/core/elidex-ecs/src/dom/tree/mutation.rs \| sed -n 20,60p` and `sed -n 266,276p`; the C0 memo D-4, D-5, §2.0 | C0b landed (#608): each of `append_child`, `insert_before`, `replace_child` adopts the child (`adopt_for_insert`, `ecs/dom/tree/mutation.rs:50`, `:127`, `:180`) into `node_document_for_new_nodes(parent)` (`:272-275`) before `link_node` (`:53`, `:132`, `:214`) and the `MutationEvent::Insert` (C0b D-4; a replace with a DocumentFragment adopts the fragment, D-5), and for a template contents fragment as parent that is the fragment's node document, I (C0b D-1; H-T21). (At `4fd85f4e` `append_child` relinked without re-homing `AssociatedDocument`.) The tree-linking chokepoint is `link_node` (`tree/mutation.rs:449`, three calls); no ECS-level insertion bypasses it (Q-10: C0 memo H-2). So a node's node document follows its last insertion or `document.adoptNode` (C0c landed, #611: `EcsDom::adopt` `tree/mutation.rs:253`, C0c D-13; `MutationEvent::Adopt`, `ecs/dom/mutation_event.rs:394`, C0c D-6); removal keeps it (as in the spec) |
| H-T9 | DOMParser documents are in the page `EcsDom`, built with the dispatcher detached, and reach the consumer after the build | `git show cee74989:crates/dom/elidex-form/src/inert_document.rs \| sed -n 108,204p` | `parse_into_inert_document` takes the dispatcher (`:112`), builds on `create_document_node()` (`:119`), restores it (`:202-203`); later `setAttribute` and inserts on its nodes fire the consumer (N1 memo H-P13) |
| H-T10 | The consumer has no bound document today | `sed -n 101,150p elidex-js/vm/consumer_dispatcher.rs`; `sed -n 224,325p elidex-js/vm/vm_api.rs` | `ConsumerDispatcher::new` builds `EventHandlerAttributeConsumer::with_policy(policy)` (`:115`); the bound `document` reaches the consumer only as `initialize_consumers(dom, document)` (`:146`, called `vm_api.rs:324`). `HostData::bind` records the document (`vm_api.rs:227`) and the Window entity is resolved (`vm_api.rs:233-239`) before the dispatcher is built (`:301`), so both halves of `HostData::active_document()` (D-T3) are `Some` there |
| H-T11 | `EcsDom::window_entity`'s production readers | `git grep -n -E '\.window_entity\(\)' cee74989 -- crates` minus `HostData` / `runtime` receivers | one: `resolve_handler_target` (`mod.rs:251`). Tests: `session/event_handler_processing/tests_insertion_and_write.rs:60`, `:101`, `:237` |
| H-T12 | The body-or-frameset class test is written three times | `git grep -n 'eq_ignore_ascii_case("body")' cee74989 -- crates` | `mod.rs:231` (`is_body_or_frameset`), `mod.rs:261` (`is_body_element`), `crates/dom/elidex-dom-api/src/char_data/document_props.rs:171` (`first_body_or_frameset_child`, the `document.body` getter). None checks the HTML namespace; the precedent `EcsDom::is_base_element` (`ecs/dom/classify.rs:39-46`) does, through `EcsDom::is_html_namespace` (`classify.rs:136`) |
| H-T13 | Slice a's window pins that name T3 | `git grep -n 'T3' cee74989 -- crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs_window_delegating.rs` | C17 (`:39`), C18 (`:64`), C19b (`:122`), C19c (`:132`), P3 (`:172`) say "reached after T3 and slice b" or "T3 facet 3" |
| H-T14 | `<body onload>` never runs at HEAD | `git show cee74989:crates/shell/elidex-shell/src/pipeline.rs \| sed -n 462,474p` | the document `load` event is dispatched at the Window entity (`:470`), non-bubbling; `<body onload>` lands on the body element (H-T2), so it never runs. Same for an initial `<body onerror\|onfocus\|onblur\|onresize\|onscroll>` against Window-targeted events |
| H-T15 | One bound document and one Window per `EcsDom`; other Documents have no Window | `git grep -n 'create_window_root()' cee74989 -- crates`; `git grep -n 'create_document_node()' cee74989 -- crates`; `git show cee74989:crates/script/elidex-js/src/vm/host_data/engine_feature/bind.rs \| sed -n 492,510p` | the Window root is created at one site, `vm_api.rs:236`, only when `HostData::window_entity` is `None`; `HostData::set_window_entity` asserts on a second set (`bind.rs:504-510`). Every iframe, navigation and worker gets a new `EcsDom` and `Vm` (`elidex-shell/src/pipeline.rs:226`, `:733`; `content/iframe/load.rs:470-489`; `content/navigation.rs:276-290`; `app/navigation.rs:323`; workers use `create_worker_global_scope_root`). Production `create_document_node()` callers besides `create_document_root` (`ecs/dom/mod.rs:460`): `inert_document.rs:119` (DOMParser: a **second, unbound, windowless Document in the page `EcsDom`**, persistent), `ecs/dom/tree/teardown.rs:309` (the fragment parser's throwaway document, destroyed at `finish_detached_fragment`, `:345-350`; since C0b only the parser's root `html` element is in it, the parsed nodes being created in the context's node document, `teardown.rs:329-334`, see P-T17) and, since C1 (landed), `ecs/dom/template_owner.rs:55`, the inert template document (C1 memo D-C1), also windowless. **Caveat (a stated defect, not T3's):** `HostData::window_entity` is initialised `None` once (`host_data/mod.rs:600`) and is **not cleared on unbind** (`git grep -n -E 'window_entity\s*=\s*None' cee74989 -- crates/script/elidex-js/src` → none), so a `Vm` re-bound to a different `EcsDom` would keep a Window entity of the old World. "One World per `Vm`" is a caller contract (doc comments), not enforced; no production path rebinds across `EcsDom`s. Unverified: whether `elidex-wasm-runtime`'s bind (`host/state.rs:50`, `:125`) shares the JS `EcsDom`; no Wasm path reaches the handler consumer (`git grep -n 'EventHandlerAttributeConsumer' cee74989 -- crates/script/elidex-wasm-runtime` → none) |
| H-T16 | Which nodes have no resolvable node document | `git show cee74989:crates/core/elidex-ecs/src/dom/node_document.rs \| sed -n 12,45p`; `git show cee74989:crates/core/elidex-ecs/src/dom/mod.rs \| sed -n 392,447p` and `sed -n 873,887p`; `git grep -n -E 'create_element_ns\(\|create_element_with_owner\(' cee74989 -- crates`; the C0 memo D-8, D-17 | C0b landed: every production creator takes a `NewNodeOwner`, which production code gets only from `node_document_for_new_nodes(source)`, so a node is ownerless only when its source has none (`node_document.rs:12-23`); the argumentless ownerless creators are fixtures (`create_element_ownerless_fixture`, `ecs/dom/mod.rs:398`, `cfg(any(test, feature = "fixtures"))`; C0b D-17); `owner_document` reads `node_document` (`mod.rs:881-886`). The landed docstrings (`mod.rs:874-879`, `scripting.rs:59-62`) give three `None` classes: a node whose document entity is gone (C1 memo D-C7's dangling guard), a node created ownerless (a fragment parse whose context has none; creation on behalf of a non-node source such as a Window / Worker), and a test-built node. The production creators of an element with a caller-chosen local name, so of a `body` / `frameset` (the only classes step 3 is reached for, D-T2), all take a node source: the dom-api `createElement` (the receiver document, `elidex-dom-api/src/document.rs:237-241`, so `dp.createElement('body')` is owned by `dp`), the parsers (the parent's, or the fragment context's, node document, C0b D-8, D-10), the cloner (the source's) and the custom-element spawn (`elidex-custom-elements` `entity_spawn.rs:58-59`). So for those classes `None` is still dangling or test-built only. (At `4fd85f4e`: `EcsDom::create_element` passed no owner and `owner_document` fell back to the tree root.) Cells XW-13, EW-18 and U-T1's unresolvable row build it with the test-only creator |
| H-T17 | A `frameset` wrapper's prototype | `git show cee74989:crates/script/elidex-js/src/vm/host/elements.rs \| sed -n 119,135p` and `sed -n 216,305p` | no `frameset` arm in `tag_specific_html_prototype` / `tag_specific_t2b_prototype`, so `HTMLElement.prototype` (`:131-133`). T-7's `f.onload` reaches `HTMLElement.prototype`'s normal pair; a WindowEventHandlers name on a `frameset` has no accessor (an ordinary property; `#11-htmlframesetelement-interface`) |
| H-T18 | Foreign elements take HTML prototypes | `git show cee74989:crates/script/elidex-js/src/vm/host_data/engine_feature/entity_kind.rs \| sed -n 124,198p` | `prototype_kind_for` (`:150`) returns `PrototypeKind::Element` for every `TagType` entity (`:164-165`); `tag_matches_ascii_case` (`:124-133`) reads no namespace. Foreign elements carry `TagType` (`create_element_ns` `ecs/dom/mod.rs:435` → `create_element_with_owner` `:408`). So an SVG or MathML `body` gets `HTMLBodyElement.prototype` with the body pair (the `body` arm, `elements.rs:294-296`), engine-wide for every tag-specific prototype. `host_data/` is split since #578 (`mod.rs` 712 lines) |
| H-T19 | The AttributeChange event carries no namespace | `git show cee74989:crates/core/elidex-ecs/src/dom/mutation_event.rs \| sed -n 366,371p` | `AttributeChange { node, name, old_value, new_value }`; emitters `ecs/dom/attribute.rs:135`, `:321`. The attribute change steps' step 1 "namespace is not null" cannot be tested at the consumer (D-T8) |
| H-T20 | The shell harness runs the `load` dispatch | `git show cee74989:crates/shell/elidex-shell/src/content_test_support.rs \| sed -n 377,399p`; `pipeline.rs:374`, `:432-478` | `build_main_document` → `build_pipeline_from_loaded` → `run_scripts_and_finalize` (`pipeline.rs:374`) → `dispatch_lifecycle_events` (`:432-478`), which dispatches `load` at the Window (H-T14). T-E1 observes it (scout, confidence high-medium; T-E1's red run is the check) |
| H-T21 | Template contents are owned by the inert document | `git show cee74989:crates/core/elidex-ecs/src/dom/template_owner.rs \| sed -n 18,62p`; `git show cee74989:crates/core/elidex-ecs/src/dom/mod.rs \| sed -n 611,647p`; the C1 memo D-C2 to D-C7; `webref body html the-template-element` | C1 landed (#626): the contents' node document is `appropriate_template_contents_owner_document(D)` = an inert document I (`ecs/dom/template_owner.rs:37`, created at `:55` and marked `InertTemplateDocument`; `attach_template_contents` maps the template's node document through it, `ecs/dom/mod.rs:620`; C1 D-C2, D-C3), parser-created contents carry it (D-C4), a clone of contents is in I (D-C6; the cloner's contents owner, `elidex-dom-api` `node_methods/clone.rs:439`), and a node appended into `tpl.content` is adopted into I (C0b D-4 with D-1, H-T8). I may also have tree children of its own (Q-13: a script can append to `tpl.content.ownerDocument`, C1 D-C7); their node document is I as well (C0b D-4). I's "browsing context is null" (spec), and I is never the bound document, so step 3 returns null for every node whose node document is I. (At `4fd85f4e`, the C2 approximation, the contents reported the template's own node document.) |
| H-T22 | SVG elements accept every event attribute | `.claude/tools/webref body SVG2 EventAttributes` | SVG2 §15.8 Event attributes: "For every event type that the user agent supports, SVG supports that as an event attribute … The event attributes are available on all SVG elements". So `onhashchange` on an SVG `body` is an event attribute of that element (target: the element, step 1) |
| H-T23 | MathML elements' event attributes and interface | `.claude/tools/webref body mathml-core attributes-common-to-html-and-mathml-elements`; `webref body mathml-core dom-and-javascript`; `webref body mathml-core mathml-elements-and-attributes`; `webref body html event-handlers-on-elements,-document-objects,-and-window-objects` | MathML Core §2.1: a `body` in the MathML namespace is an *Unknown MathML element*. §2.2.3 DOM and JavaScript: every MathML element implements `MathMLElement` (`MathMLElement : Element`, `includes GlobalEventHandlers`, `includes HTMLOrSVGOrMathMLElement`), so no WindowEventHandlers IDL attribute. §2.1.4 Attributes common to HTML and MathML elements: "All event handler content attributes noted by HTML as being supported by all HTMLElements are supported by all MathML elements as well, as defined in the MathMLElement IDL." HTML §8.1.8.2 supports the WindowEventHandlers content attributes on "body and frameset elements" only. So a WindowEventHandlers name is **not** an event handler content attribute of any MathML element, and a MathML element named `body` / `frameset` never gets HTML element semantics. An element in any other namespace has none: the attribute change steps' step 1 returns unless the local name is an event handler content attribute on the element, and HTML §8.1.8.2, SVG2 §15.8 and MathML Core §2.1.4 define them for their own namespaces only |
| H-T24 | The clone path: one production entry, one clone event | `git grep -n -E 'clone_node_with_shadow_honor\(\|"cloneNode"' cee74989 -- crates`; `sed -n 165,215p elidex-js/vm/host/node_methods_extras.rs`; `sed -n 235,313p crates/dom/elidex-dom-api/src/node_methods/clone.rs`; `sed -n 462,530p elidex-js/vm/host/dom_bridge.rs` | the only production element-cloning operation is `Node.prototype.cloneNode`: `native_node_clone_node` (`node_methods_extras.rs:203`) → `invoke_dom_api(ctx, "cloneNode", …)` (`:213`) → the dom-api `CloneNode` handler (`crates/dom/elidex-dom-api/src/node_methods/clone.rs:157`) → `clone_node_with_shadow_honor` (`clone.rs:235`; template contents and clonable shadow trees replicated), which fires `MutationEvent::CloneCreated { root, scope }` once, after the whole copy exists and before it returns (`clone.rs:305-311`; `EcsDom::fire_clone_created`, `ecs/dom/mod.rs:327-342`, debug-asserts that the dispatcher is not taken out). RC-2a landed (#596): the custom-element upgrade enqueue and the form-state pass are `CloneCreated` arms (`elidex-custom-elements` `consumer.rs:116`, `elidex-form` `reconciler.rs:53`) and the VM post-pass is deleted: the shim returns the dom-api result (`node_methods_extras.rs:213`; the RC-2a plan §4.4–§4.6). (At `4fd85f4e` the shim ran the two seams itself after the call.) **No script runs** inside the dom-api call after the handler (Q-11): `invoke_dom_api`'s phase after the handler only drains notify records into the microtask-bound record queue (`dom_bridge.rs:32-42`). T3 consumes the same event (D-T10) |
| H-T25 | The VM already marshals the policy into a session processing entry; the uncapped walker reaches every cloned node | `sed -n 603,612p elidex-js/vm/host/event_handler_attrs.rs`; `sed -n 50,57p session/event_handler_processing/mod.rs`; `git show cee74989:crates/core/elidex-ecs/src/dom/tree/walkers.rs \| sed -n 95,143p`; `sed -n 422,446p crates/dom/elidex-dom-api/src/node_methods/clone.rs` | `marshal_before_handler_write` reads `ctx.vm.spec_level_policy` and calls `elidex_script_session::process_before_handler_write(host.dom(), element, name, policy)` (`:608-610`). `ProcessingTiming` has `Bind`, `Insertion`, `HandlerWrite` (`mod.rs:50-57`). `EcsDom::for_each_uncapped_shadow_inclusive` (`walkers.rs:111`) takes a `TemplateContentsWalk` (`:19`); the bind pass passes `Exclude` (`mod.rs:178`). (Q-12) With `Include` it is uncapped (explicit stack + visited set) and pushes, per node, the template contents fragment, the light children and the shadow root, so it reaches a clone's replicated template contents (`replicate_template_contents`, `clone.rs:422-`, nested templates re-replicated) and clonable shadow trees at any depth. Its visit order since C0c D-19 (landed): a node, then its template contents, then its shadow tree, then its light children (`walkers.rs:123-141`; at `4fd85f4e` the contents came after the template's subtree). The clone path walks the copy with RC-2a's walk and only each first-level template's contents with this one (D-T10) |
| H-T26 | No other element-cloning path exists | `git grep -n -i 'importNode' cee74989 -- crates` → none; `sed -n 249,263p` and `sed -n 600,617p crates/dom/elidex-dom-api/src/range/mutation.rs` | `importNode` is not implemented. `Range.cloneContents` is a stub (`clone_contents` returns `None`, slot `#11-range-full-impl`); `extractContents` clones boundary Text only, its partially-contained element clone is deferred (B1.2d-ii plan §11) |
| H-T27 | (Q-7) A non-bubbling event dispatched at an element runs no handler on the Window | `sed -n 203,221p session/event_dispatch/mod.rs`; `git show cee74989:crates/core/elidex-ecs/src/dom/mod.rs \| sed -n 494,504p` | `build_propagation_path` follows `get_parent` only, and `create_window_root` spawns the Window with no `TreeRelation` ("does **not** participate in the DOM tree"), so the Window is on no element's propagation path (`dispatch_script_event` uses the same path builder). So a non-bubbling `focus` / `hashchange` at an element never reaches the Window's handler. Static evidence only (no dynamic run): the cells resting on it say so (§5.2, T-6b). The bubbling case is `#11-event-path-window-missing`, confirmed at runtime 2026-11-05 (`f9cf8c40`); no T3 cell depends on it (§9 item 1) |
| H-T28 | (Q-8) A DOMParser frameset document's `body` is the `frameset` | `sed -n 22,29p elidex-js/vm/host/dom_parser.rs`; `crates/dom/elidex-html-parser-strict/tests/data/html5lib/tree-construction/tests2.dat:63-80`; `elidex-js/vm/tests/tests_document_methods.rs:505-` | DOMParser builds through `parse_into_inert_document`; html5lib's expectation for `<!DOCTYPE html><frameset>` puts `frameset` under `html` with no `body`; the body getter returns a `frameset` child (`document_body_returns_frameset_when_no_body_present`). Static evidence only: T-6b says so |
| H-T29 | (rev 4.7) The fragment build takes the dispatcher only during the build; one function returns every fragment parse | `git show cee74989:crates/dom/elidex-html-parser/src/lib.rs \| sed -n 140,160p` and `sed -n 207,247p`; `git show cee74989:crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs \| sed -n 165,245p`; `git grep -n 'parse_fragment_progressive(' cee74989 -- crates` | tolerant: `begin_detached_fragment` takes (`lib.rs:143`), `finish_detached_fragment` (`:155`), restore (`:156-158`); strict: take (`tree_builder/mod.rs:180`), restore on both result arms (`:238-241`, "so the caller's placement of the returned detached nodes fires events"). `parse_fragment_progressive` (`lib.rs:223`) is the one dispatch site of both backends (strict; on reject, tolerant over the pristine dom, `:229-246`) and is called only from `session/mutation/html_fragment.rs` (`:93`, `:174`, `:209`, `:218`, `:235`, `:245`), which places the returned nodes after it returns. So the placement's `Insert` reaches the consumer on the base (the base delivers a fragment-parsed `body`'s Window-delegating pairs there: `ProcessingTiming::Bind \| Insertion`, `session/event_handler_processing/mod.rs:121`), and an event fired after the backend returns and before the placement reaches it too. Entry points: the `innerHTML` setter on Element / ShadowRoot / template (`elidex-js/vm/host/dom_inner_html.rs:146`), `setHTMLUnsafe` (`:419`, `:474`), `outerHTML` (`:361` → `apply_set_outer_html` `html_fragment.rs:143`), `insertAdjacentHTML` (`apply_insert_adjacent_html` `:193`), the queued `Mutation::SetInnerHtml` (`session/mutation/mod.rs:225-226`), DOMParser (H-T30). `Range.createContextualFragment` is a stub that parses nothing (`crates/dom/elidex-dom-api/src/range/mutation.rs:648`); no `document.write`, no `parseHTMLUnsafe` |
| H-T30 | (rev 4.7) DOMParser's window encloses a fragment parse | `git show cee74989:crates/dom/elidex-form/src/inert_document.rs \| sed -n 108,205p`; `git show cee74989:crates/core/elidex-ecs/src/dom/mod.rs \| sed -n 210,232p` | `parse_into_inert_document` takes the dispatcher (`:112`), calls `apply_set_inner_html` (`:142`, so `parse_fragment_progressive`), and restores it (`:202-203`) after its own structural re-runs. Inside, `begin_detached_fragment`'s take finds the slot `Taken` and returns `None` without opening a window (`take_mutation_dispatcher`, `ecs/dom/mod.rs:221-232`), so no restore follows and the slot is still `Taken` when `parse_fragment_progressive` returns there |
| H-T31 | (rev 4.7; census restated in rev 4.8, PM decision 711 (6)) The `CloneCreated` population | `git grep -n -e CloneCreated -e fire_clone_created cee74989 -- crates \| wc -l` and `git grep -l -e CloneCreated -e fire_clone_created cee74989 -- crates \| wc -l`; `git grep -n CloneCreated cee74989 -- crates \| wc -l` and `git grep -l CloneCreated cee74989 -- crates \| wc -l` | **73 lines in 19 files** (verified 2026-11-13) naming the variant (substring `CloneCreated`, so `CloneCreatedProbe` too) **or** its fire function `fire_clone_created`; of them **50 lines in 18 files** (verified 2026-11-13) contain `CloneCreated` (the other 23 lines name only `fire_clone_created`, and the 19th file, `ecs/dom/tests/dispatch_guard.rs`, names only the fire function). Rev 4.7 wrote "73 lines in 19 files" without saying it counted both names. Match arms: `elidex-custom-elements/src/consumer.rs:116` (`CustomElementReactionConsumer`), `elidex-form/src/reconciler.rs:53` (`FormControlReconciler`), `elidex-dom-api/src/test_util.rs:53` (test probe `CloneCreatedProbe`); ignored-variant list: `elidex-dom-api/src/range/live.rs:160` (`LiveRangeBridge`); fire: definition `ecs/dom/mod.rs:333`, one production call `elidex-dom-api/src/node_methods/clone.rs:305`; the rest are docs and tests. `EventHandlerAttributeConsumer::handle` matches `AttributeChange` and `Insert` only (`session/event_handler_consumer.rs:381-391`). The other three consumers of `ConsumerDispatcher` (`node_iter`, `base_url`, `canvas`; `elidex-js/vm/consumer_dispatcher.rs:157-163`) do not name it |
| H-T32 | (rev 4.8.1) L3's guard and walk scopes, as planned (pre-ratification snapshot of L3's split plans; PM decisions 711, 713, 714) | `S=6d31ced1; P=docs/plans/2026-11-ecs-entity-lifetime` (the one snapshot read: L3's local plan branch, read only through `git show`), then `git show $S:$P-pr-ig-dispatcher-take-guard.md \| grep -n -e 'pub fn close' -e 'one site of the restore' -e 'does nothing that can panic' -e 'window.close();' -e 'Drop restores' -e '^\*\*T3 intersection' -e 'This PR guarantees only'`; `git show $S:$P-pr-io-owned-entities-walk-scope.md \| grep -n -e 'pub enum WalkScope' -e 'const fn in_scope' -e 'scope: WalkScope, visit' -e 'enum OwnedEntity'`; `git show $S:$P-umbrella.md \| grep -n -e '^\*\*Order\.\*\*' -e 'Layout T3 lands after PR (io)'` | **PR (ig) plan** §4.1: `DispatcherTake<'a> { dom: &'a mut EcsDom, taken: Option<Box<…>> }`, "the one way to take the mutation dispatcher out of an `EcsDom`", held by all four take windows (`despawn_subtree` through `TeardownWindow`, both fragment parses through `begin_detached_fragment`, `parse_into_inert_document`); `DispatcherTake::close(self)`, "the explicit restore point on a returning path (PM 705). It consumes the guard, so its `Drop` runs here; `Drop` stays the unwind backstop and the one site of the restore" (its body is `drop(self)`); `impl Drop` restores the box when the guard holds one, a nested guard holding nothing (*Why it is unwind-safe* item 3), and `Drop` "does nothing that can panic" (item 2); *The restore point*: the outermost guard's close or drop; the two fragment windows and `parse_into_inert_document` call `window.close()` on their returning path (no early return in between), while `despawn_subtree`'s `TeardownWindow` restores by its drop at the end of its block (§4.2, "Drop restores"). §8 *T3 intersection*: T3 adds the queue in the `DispatcherSlot::Taken` state and the flush at the outermost guard's `close` (restore the box, then dispatch in order, skipping non-live roots; on unwind, restore only and discard; no consumer code during unwind); ig guarantees only (a) the restore point is the outermost guard's `close` and (b) the slot state is the single home of take/restore state (no second flag or depth counter in any window). **PR (io) plan** §4.1: `pub enum WalkScope { ShadowIncludingTree, WithTemplateContents, LifetimeUnit }` with `OwnedEntity::in_scope(self, scope)`, an exhaustive match: `ShadowRoot` ∈ all three, `TemplateContents` ∈ `WithTemplateContents` and `LifetimeUnit`, `InertTemplateDocument` ∈ `LifetimeUnit` only; `WalkScope` is `pub`, `OwnedEntity` / `in_scope` are `pub(in crate::dom)`; §4.2: the walker becomes `for_each_uncapped_shadow_inclusive(&self, root, scope: WalkScope, visit)`, `TemplateContentsWalk` is deleted, `despawn_subtree` passes `WithTemplateContents`, the base's `Include`. **L3 umbrella** §5 *Order*: is → ig → io → ii; "Layout T3 lands after PR (io)" (its `WalkScope`, ig's outermost-close flush point; the §8 Layout T3 hand-off says the same). Announced after the snapshot (PM 718; not in it, so not relied on): `TeardownWindow` gains a consuming close, `is_outermost` is removed, the walker passes an `OwnedEntity` to `visit` and the owned side gains back-links; T3 relies on none of these (§4.5 item 4 does not name `is_outermost`); the type of `visit` is re-pinned with every other citation (R7). Until rev 4.8.1: cited from the pre-split PR (i) plan |
| H-T33 | (rev 4.8) The spec's attribute change steps do not consult scripting; compilation does | `.claude/tools/webref body html event-handler-attributes`; `webref dfn html 'get the current value of the event handler'`; `webref heading html 8.1.3.4`; `git show cee74989:crates/dom/elidex-form/src/inert_document.rs \| sed -n 64,90p`; `git show cee74989:crates/script/elidex-script-session/src/scripting.rs \| sed -n 40,90p` | HTML §8.1.8.1 Event handlers, the attribute change steps: step 1 (namespace / name test), 2 (determine the target), 3 (null → return), 5.1 (CSP), 5.5 "Set eventHandler's value to the internal raw uncompiled handler value/location", 5.6 "Activate an event handler": no step reads scripting or the browsing context. *Get the current value of the event handler* (§8.1.8.1, `#getting-the-current-value-of-the-event-handler`) step 3.2: "If document's browsing context is null, or document's active sandboxing flag set has its sandboxed scripts browsing context flag set, then return null" (document = the element's node document, step 3.1). §8.1.3.4 is *Enabling and disabling scripting*. `inert_document.rs:78-84` says the `EventHandlerAttributeConsumer` must stay off for a DOMParser document because "HTML §8.1.3.4 does NOT process event-handler attrs when scripting is disabled": wrong (P-T19). N1's compile gate (landed, #580; clause (b) read through `EcsDom::node_document` since C0b) realises step 3.2 for a node whose node document is not the bound document (`scripting_disabled_for_platform_object`, `scripting.rs:40-90`), consulted by `ensure_event_handler_current` on every read and dispatch (`elidex-js/vm/host/event_handler_attrs.rs:515-560`) |

### §0.3 Premise errors (reported)

Rev 1 to rev 3's P-T1 to P-T13 are kept in rev 3 (header pointer); the two still read by a record:
- **P-T1.** The ledger row's coordinate "`resolve_handler_target` (`…/event_handler_processing/mod.rs:249`) … code as of
  `fd6955e8`" mixes frames: at `fd6955e8` the function is `session/event_handler_consumer.rs:328`; `mod.rs:249` is the
  position since #563. §8.3 item 1 carries the correction (a landing-time record).
- **P-T2.** The slot records **four** facets, not three (facet (4), the reflecting set, was added 2026-10-25), and its
  owner is the EH-attrs program (re-routed sub-bullet), although its first paragraph still says IBP-sandbox.

New in rev 4:
- **P-T14.** Rev 3's FN-1 gave the spec outcome "element" for a `body` / `frameset` in a namespace other than HTML, SVG
  and MathML with a reflecting name. H-T23 says such an element has no event handler content attributes: the spec
  outcome is "nothing" (row FN-1b). Rev 3 marked the row "both"; it was base-equal, not the spec's. Under D-T8 T3 gives the spec's.
- **P-T15.** Rev 3's base omitted RC-2a, which C1 requires (C1 memo Prerequisites; Range program row RC-2a). On the base
  `cloneNode` fires `MutationEvent::CloneCreated { root, scope }` from the canonical cloner entry, its CE and form passes
  run in dispatch-time consumers, and the VM post-pass is deleted (the RC-2a plan §2 A, §4.6). So rev 4's D-T10 placement,
  beside that post-pass, does not exist on the base; rev 4.1 moves it onto the event (D-T10, PM decision 359).
- **P-T16.** Rev 3 Q-13's premise "I has no children unless a script appends some" is the C1 memo's statement (D-C7),
  and a script can append them; no rev 3 outcome relied on it (H-T21: step 3 is null for every node in I, connected or
  not), and C0b D-7 removes the composed-root branch the question was about.

New in rev 4.5 (re-measure at `cee74989`):
- **P-T17 (resolved: PM decision 699, O-10 (a); D-T7).** D-T7 and §2.1's closing note say a fragment parse with an `html` context creates its
  `body` / `frameset` in the parser's throwaway document, so the spec delivers nothing for it (EW-10's case). The
  spec does not: *HTML fragment parsing algorithm* step 2 creates the fragment in target's node document, and *create
  an element for the token* step 6 uses targetParent's node document after the insertion target redirection map
  ("elements redirected out of the parser's root html element … are created directly in the target's node document";
  `webref body html html-fragment-parsing-algorithm`, `webref body html creating-and-inserting-nodes`, 2026-11-13).
  C0b implements that (D-8, D-10: `node_document_for_new_nodes` redirects the parser root, `ecs/dom/node_document.rs:66-77`;
  `teardown.rs:329-334`). So for a context in the active document (e.g. `document.documentElement.innerHTML =
  '<body onresize="…">'`) the `body` is created in the active document, and the spec runs the attribute change steps
  at its creation with step 3 passing: EW goes to the Window at creation. Base: slice a delivers EW at the insertion
  onto the element (X-W, HEAD's target rule). T3: the fragment path has no Creation processing (slice b's S-1) and
  EW has no Insertion delivery (D-T7), so nothing. T3 equals neither the base nor the spec there, which §1.2's
  criterion counts as worse than the base; "Rows worse than the base: 0" and §1.2's "0 rows" do not hold as written.
  XW on the same elements is unaffected (delivered to the Window at the insertion, as at the base: XW-6's class).
  A context outside the active document (a DOMParser document, I) still gives the spec's null, as rev 4.4 says.
  Rev 4.6: T3 processes the fragment path at creation (D-T7, PM decision 699), so the row is EW-20, on the spec.

New in rev 4.7:
- **P-T18 (rev 4.6's own; corrected by H-T29, H-T30).** Rev 4.6's O-11 said D-T10's `CloneCreated` arm does not reach
  a fragment parse because "the parser's fragment build takes the dispatcher (`begin_detached_fragment`, umbrella H6
  / N-1)". The build takes it only during the build and restores it before returning, so the caller's placement
  fires events (H-T29), and an event fired after the build and before the placement reaches the consumer. Only a
  window that encloses the build keeps it taken, which is DOMParser's (H-T30). Umbrella N-1's "a `MutationEvent` fired
  inside the fragment build reaches no consumer" is true of *inside* and does not bar a fire after the restore. The
  base's delivery of a fragment-parsed `<body onresize>` at the placement comes from `Insertion` timing
  (`event_handler_processing/mod.rs:121`); T3's "nothing" without D-T12 came from T3 removing EW's Insertion delivery
  (D-T7), not from an unreachable consumer.

New in rev 4.8:
- **P-T19 (rev 4.7's own, and the base's rustdoc; PM decisions 711, 713).** Rev 4.7 §4.5 item 4 dropped the fragment
  event inside DOMParser's window because "dropping is that build's intended outcome, since it suppresses the
  script-activation consumers", resting on `inert_document.rs:78-84` ("HTML §8.1.3.4 does NOT process event-handler
  attrs when scripting is disabled … Leaving both off is the CORRECT inert behavior"). The spec's attribute change
  steps set a raw uncompiled handler and activate it without consulting scripting; the null for a document without a
  browsing context is at compile time, *get the current value of the event handler* step 3.2 (H-T33). So a
  DOMParser document's elements carry their handlers from creation, and the base's (and rev 4.7's) "nothing" was a
  defect, not the inert outcome. T3 queues the event (§4.5 item 4), sets the raw handlers (D-T13) and corrects the
  rustdoc (§5.3).
- **P-T20 (PM decision 711's premise; answered by PM decision 713 and the split plans, rev 4.8.1; one residual
  open).** PM decision 711 names the flush point "PR (i)'s consuming close". The pre-split PR (i) plan had no
  consuming close (rev 4.8's finding). The PR (ig) plan has one: `DispatcherTake::close` consumes the guard and is
  the explicit restore point on a returning path, and its §8 *T3 intersection* guarantees (a) the restore point is the
  outermost guard's `close` and (b) the slot state is the single home of take/restore state, with the flush in that
  `close` assigned to T3 (H-T32). So §4.5 item 4 relies on ig's guarantees and T3's change to `close`, and no longer
  states a requirement on ig. Every window whose `pending` can be non-empty (DOMParser's, `parse_into_inert_document`)
  calls `close` on its returning path with no early return before it (H-T32); `despawn_subtree`'s `TeardownWindow`
  restores by its drop at the end of its block in the snapshot, and nothing fires inside it, so T3 does not rely on
  it.
  **Residual (open, for the PM with L3): where the flush sits against ig's one restore site.** The PR (ig) plan §4.1
  makes `Drop` "the one site of the restore" and gives `close` the body `drop(self)`; the guard owns the
  `&'a mut EcsDom` borrow. Once the restore has run in that `Drop`, `close` no longer holds the dom, so "restore in
  `Drop`, then dispatch from `close`" cannot be written in `close` as planned. Whichever shape T3's change takes must
  move one of three things: (α) the restore into `close` too (a second restore site, against ig §4.1's "one site");
  (β) the dispatch into `Drop`, run only after a `close` has handed it the pending list, so the unwind path's `Drop`
  is unchanged but the close path's `Drop` runs consumer code (ig's *Why it is unwind-safe* item 2, "`Drop` does
  nothing that can panic", then holds on the unwind path only); (γ) the dispatch out to each window's caller, which
  regains its `&mut EcsDom` after `close` returns the list (one flush per window site instead of one). §4.5 item 4
  fixes the observable order (restore before the first dispatch, in fire order, dead roots skipped, nothing on
  unwind) and leaves the shape to this question; T3 does not choose it here (designer stop, rev 4.8.1). None of
  (α)–(γ) needs `is_outermost`: the guard knows whether it took from its own `taken`.

### §0.4 Decisions (this PR)

Pair classes used below (all on HTML-namespace elements unless stated): **XW** = `body` × the 18 WindowEventHandlers
names, the pairs HEAD already targets at the Window; **EW** = `body` × the 6 Window-reflecting names and `frameset` × all
24 Window-delegating names, the pairs HEAD targets at the element (PM decision 129's population); **FN** = a
foreign-namespace `body` / `frameset` with any of those 24 names; **OR** = every other pair.

- **D-T1 — one function, every spec caller.** `determine_event_handler_target(dom, target, name, active) ->
  Option<Entity>` (NEW), in `session/event_handler_processing/mod.rs`, `pub`, is the only realisation of the algorithm.
  `None` is the spec's null. Callers: the attribute change steps at `process_handler_content_attributes` (every timing,
  the clone path's Creation included, D-T10) and the AttributeChange arm; the VM getter and setter, both pairs.
  `resolve_handler_target` and `is_body_element` are deleted. The body pair keeps its own receiver brand check
  (`require_body_receiver`; WebIDL `HTMLBodyElement`, namespaced by D-T11) but no longer chooses a target. The VmObject
  handler backend (`native_vm_event_handler_get` / `_set`) does not call it: its receivers are never `body` / `frameset`
  elements, so step 1 returns the receiver by type (stated in its docstring).
- **D-T2 — steps 1–2 are `is_window_delegating_pair`.** The function calls slice a's classifier for steps 1–2 and returns
  `Some(target)` when it is false. The X-W classifier and the target rule then agree by construction (H-T4's
  disagreement is the defect of facets (3) and (4)). Step 2's name test is by name: the `scope` parameter leaves target
  determination (`HandlerScope` stays the IDL-install key).
- **D-T3 — steps 3–4 over a marshalled `ActiveDocument` (NEW); PM decision 132.** `pub struct ActiveDocument { pub
  document: Entity, pub window: Entity }` (fields `pub`: its one constructor is in `elidex-js`, below): the active
  document of the engine's single browsing context and its relevant global object (H-T6, H-T15). Step 3: the target's
  node document (D-T4) equals `active.document`, else `None`. Step 4: `active.window`. **One constructor (PM 441):**
  `HostData::active_document(&self) -> Option<ActiveDocument>` (NEW unless the event-path PR-B landed it first;
  `host_data/engine_feature/bind.rs`, beside `document_entity_opt` `:474`) = `document_entity_opt()` zipped with
  `window_entity()` (`:492`), `None` when unbound; it is the only construction of `ActiveDocument` outside tests, in
  either landing order with the event-path plan's PR-B (`docs/plans/2026-11-event-path-window.md` D-2: whichever of T3
  and PR-B lands first adds the type and this method, the other reuses them). **Both marshalling points go through it
  (PM 455):** `Vm::bind` calls it when it builds the consumer (`Some` there, H-T10), and the VM accessors' ctx-taking
  host helper delegates to it (no second constructor; §4.2). (Until rev 4.5: "The consumer receives it at construction
  from `Vm::bind` (`document` and the resolved Window, H-T10); the VM accessors build it from `HostData`
  (`document_entity_opt()`, `window_entity()`).") `active.document` is the one source C1-pre's D-4 predicate
  `document_has_browsing_context` reads (the bound document): step 3 asks D-4's question in the session, and
  `HostData::active_document()` is host marshalling beside D-4, in the same layer (until rev 4.5: "the VM helper
  building `ActiveDocument`").
  `EcsDom::window_entity` loses its last production reader and is deleted (H-T11; dead-code rule). **PM decision 132:**
  marshalling is accepted; the active document and its Window are browsing-context state, not a per-element fact, so the
  side-store→component rule does not apply, and their move into ECS components is
  `#11-browsing-context-state-ecs-components`, decided with B1. *Rejected:* (A) a component relating the Document and
  its Window, written at bind (it duplicates `HostData`'s `document_entity` / `window_entity` pair, which B1 §5 req 5
  migrates as one cluster); (D) `document_root()` keying (slice a's D-5 / cell U12; the node memo's "No reader keys on
  `document_root()`"). *Relied-on caveat* (H-T15): `ActiveDocument.window` comes from `HostData::window_entity`, which
  unbind does not clear; T3 is correct only under the caller contract "one `EcsDom` per `Vm`", which every production
  bind path keeps. B1 makes it structural (§8.2).
- **D-T4 — one node-document derivation: C0b's `node_document` (decided by Q-10).** Step 3 reads the target's node
  document through `EcsDom::node_document` (C0b D-1, landed), the reader N1's compile-gate clause (b) uses on the base
  (C0b D-7 (a); H-T7). T3 adds no derivation and no function. Unresolvable (`None`) → treated as `active.document`, the
  direction clause (b) takes for the same `None` ("the node clause is false", `scripting.rs:59-62`; rev 4.4 quoted
  `4fd85f4e`'s ground "such nodes belong to the bound document's realm by construction", which C0b removed); for a
  `body` / `frameset`, the only classes step 3 is reached for, `None` is test-built or dangling only (H-T16; cells
  XW-13, EW-18).
  The one dangling class, a node whose node document I was destroyed with D (C1 D-C7), gets the Window where the spec
  gives null; accepted as-is, since no production path destroys a Document that has an I (C1 H-C12).
  With C0b's adoption at every insertion (H-T8) and C1's inert document (H-T21), `node_document` gives the spec's node
  document for every node that reached its place by insertion, by parsing or by cloning. *Rejected:* a derivation
  private to step 3 (two answers to one question, A3×A9).
- **D-T5 — step 1's class test is HTML-namespaced; `document.body` is not touched (PM decision 133; its reading
  confirmed by the PM, rev 2 O-5).** The session's `is_body_or_frameset` (`mod.rs:228`) gains `EcsDom::is_html_namespace`
  (the `is_base_element` precedent) and stays session-private; `is_body_element` is deleted (D-T1); no `EcsDom` symbol is
  added. PM decision 133 unifies with `document.body`'s predicate only if T3's step 3 or 4 uses it; neither does (step 3
  reads the node document, step 4 the global object; "the body element" of HTML §3.1.7 DOM tree accessors is not read).
  So `first_body_or_frameset_child` (`document_props.rs:167-171`) keeps its tag-only test and the `document.body` defect
  is recorded for the PM (§8.1 item 3). The namespace stays in step 1 because T3 itself needs it: under a tag-only test
  an SVG `body`'s `onfocus` would move from the element (HEAD's and the spec's target) to the Window (cell T-9b). The
  VM's interface choice for `body` is a separate test (D-T11).
- **D-T6 — order inside the IDL setter.** receiver brand check → determine (D-T1) → `None`: return → slice a's
  `marshal_before_handler_write` (D-6) → activate. Determination precedes D-6 per the setter's steps 1–2; the reorder
  changes nothing observable, because `None` occurs only for Window-delegating pairs, for which
  `process_before_handler_write` returns at once (`mod.rs:156`). The getter: brand check → determine → `None`: null →
  read (N1's reconcile, unchanged).
- **D-T7 — X-W ends for EW; XW keeps it until S-5 (PM decision 129); the clone path processes every class (PM decision
  153); a fragment parse's elements are processed at their creation, every class (PM decisions 699, 705; D-T12).**
  `process_handler_content_attributes` delivers a pair by class and timing, after D-T8's applicability test:

  | class | Bind (T-a1) | Creation, clone path (T-c, D-T10) | Creation, fragment path (D-T12) | clone-internal Insertion, before `CloneCreated` (D-T10) | Insertion (T-a2) | HandlerWrite (T-a3) | attribute change steps (AttributeChange arm), IDL setter |
  |---|---|---|---|---|---|---|---|
  | XW | delivered (X-W) | delivered | delivered (the placement's Insertion re-raws it, X-W) | as Insertion; Creation rewrites it | delivered at every insertion (X-W, until S-5) | not processed | T3's target |
  | EW | delivered, once per document (the bind pass's own once rule, D-5) | delivered | **delivered** | not delivered | **not delivered** | not processed | T3's target |
  | OR | once (K-1) | delivered, and marked (K-1) | delivered, and marked (K-1; the placement's Insertion finds it marked) | as Insertion; Creation rewrites it | once, if unmarked (K-1) | D-8 | element |

  Every delivery goes to `determine_event_handler_target`'s result (`None` → nothing). The X-W population is named by
  `is_x_w_pair(dom, element, name)` (NEW, session-private): an HTML `body` and a `HandlerScope::Window` name. Its
  docstring names S-5 as its remover. EW's creation-time attributes on an element created after bind by a **DOMParser
  build** are processed at its creation, after the build's window closes (§4.5 item 4; PM decision 711), and deliver
  nothing: the element's node document at creation is the DOMParser document, which is not active (cells EW-10,
  T-15b, T-22c); its OR attributes are set raw on the element (D-T13). (Until rev 4.8: "are therefore not processed
  until slice b's S-1; that is the spec's outcome …".)
  **Fragment parse (PM decision 699; P-T17).** A fragment parse creates its elements in the context's node document
  (*HTML fragment parsing algorithm* step 2; *create an element for a token* step 6, through the insertion target
  redirection for the parser's root `html` element; C0b D-8, D-10), so a fragment parse that creates a `body` /
  `frameset` (e.g. with an `html` context, `document.documentElement.innerHTML = '<body onresize=…>'`; which contexts
  create one does not matter to the mechanism, since every fragment-parsed element is processed at creation, D-T12) creates it in the context's node document, and the spec runs the attribute change steps at their creation with
  that document. *Determine the target of an event handler* (HTML §8.1.8.1) step 3 asks only whether eventTarget's
  node document is an active document, and step 4 returns that document's relevant global object; the step's note
  says that a `body` created in an active document but not connected also targets the Window. So in an active-document
  context EW is delivered to the Window **at the element's creation** (row EW-20; cells T-22, T-22b), and in a
  non-active one (a DOMParser document, I) step 3 returns null and nothing is delivered (row EW-21). T3 processes
  the fragment path at Creation for every class (the column above; PM decision 705, D-T12; the event and its walk
  set: §4.5). EW thus satisfies K-1 to K-3 unconditionally from T3: nothing derived from an EW attribute is written after the
  bind pass except at a clone's or a fragment-parsed element's creation, by the attribute change steps and by the IDL
  setter, which are the spec's writers.
  *Rejected:* (α) EW joining the ordinary pairs' once rule (processed at the first insertion if unmarked). The first
  insertion reads the target at the insertion, not at the element's creation, so a DOMParser `body`'s
  attribute reaches the page Window at its first insertion into the active document (W2, cell T-15b's mutant), and an
  unmarked element's attribute overwrites a `window.on<T>` written before that insertion (W1 on the D-8-failing path,
  U-T6). (β) A timing-dependent target (two target rules). Delivery at a clone's or a fragment-parsed element's
  **creation** is not α: it is the spec's own processing point (D-T10; PM decision 699), so an overwrite of
  `window.on<T>` there is the spec's outcome, not W1. For the fragment path, O-10's (b) (a once-only EW delivery at
  the first insertion), (c) (EW keeps X-W's re-raw at every insertion) and (d) (a stated residual) are rejected by PM
  decision 699 (§10).
- **D-T8 — applicability: T3 takes the local-name half of the attribute change steps' step 1 (PM decision 168, O-7
  (a)); the namespace half stays with slice b.** One predicate `is_event_handler_content_attribute_of(dom, element,
  name)` (NEW, session-private, `session/event_handler_processing/mod.rs`) is tested first by
  `process_handler_content_attributes` (every timing, Creation included) and by the AttributeChange arm; false → return
  (no set, no clear). By the element's namespace: an HTML element accepts the GlobalEventHandlers names, plus the
  WindowEventHandlers names on `body` / `frameset` only (`is_body_or_frameset`, D-T5; HTML §8.1.8.2), and no
  Document-only name (`HandlerScope::DocumentOnly`); an SVG element accepts every name (H-T22); a MathML element
  accepts the GlobalEventHandlers names (MathML Core §2.1.4 "as defined in the MathMLElement IDL", which includes
  GlobalEventHandlers; H-T23); any other namespace accepts none (H-T23's ground). This closes
  applicability facets (1) and (3) (§8.3 item 8). Facet (2), a non-null attribute namespace, stays with slice b: it needs
  `MutationEvent::AttributeChange` to carry a namespace (H-T19) and the silent writers' namespaces, which slice b's S-7
  census re-runs. The IDL accessors are not affected (their surface is the prototype's, D-T11). Every outcome the
  predicate changes is toward the spec (rows FN-1b, FN-4, FN-5, OR-1; cells T-21, U-T8).
- **D-T9 — `frameset`'s interface is compat (PM decision 130).** `#11-htmlframesetelement-interface` is registered at
  landing (§8.3 item 9), in the compat layer: `frameset` is an obsolete feature (HTML §16.3.2 Frames, under §16.3 Requirements for
  implementations), so its interface belongs to a compat plugin, not core (CLAUDE.md core/compat/deprecated). Owner: none
  until the compat plugin work starts. T3 changes only target determination for a `frameset`; its interface is not
  added.
- **D-T10 — the clone path of S-1 (PM decision 153).** WHATWG DOM §4.4 *clone a single node* step 2.5 appends each
  attribute to the copy (§4.9 *append an attribute*), whose step 4 runs *handle attribute changes*, whose step 3 runs
  the attribute change steps: a clone's handler content attributes are processed **at its creation**, with the copy's
  node document. T3 realises that for every clone made by the canonical cloner (H-T24, H-T26):
  - **Placement — a changed decision (rev 4.1, PM decision 359; O-8).** *Old (rev 2 to rev 4):* the VM's
    `native_node_clone_node` called the entry point after the dom-api call, beside the shim's CE and form post-pass.
    *New:* a `MutationEvent::CloneCreated { root, scope }` arm of `EventHandlerAttributeConsumer::handle` calls
    `process_created_subtree(dom, root, scope, policy, active)` with the consumer's own policy and `ActiveDocument`
    (D-T3). (Rev 4.7: the arm is the `SubtreeCreated` arm and the call passes the event's `cause`, §4.5, §4.2.) *Why:* on the base the canonical cloner entry fires `CloneCreated` once per clone call, after the whole copy
    exists and before the entry returns, and RC-2a deletes the post-pass (the RC-2a plan §2 A, §4.1, §4.6). It is the
    one clone signal, so T3 consumes it and adds no second hook (one issue, one way). The consumers run in field order
    (`form_control`, `event_handler_attrs`, …, `custom_elements`; `elidex-js/vm/consumer_dispatcher.rs:157-163`), and
    none reads what another writes (this consumer: `EventListeners` and the marker, field doc `:61-72`; the RC-2a plan
    §2 C × D), so their order is unobservable. With no dispatcher installed nothing is processed at a clone (the RC-2a
    plan §2 F × I), as for every unbound write (`#11-unbound-attribute-writes-bypass-consumers`, §8.3 item 10). *Rejected:* rev 4's VM call (the post-pass it sat beside is gone, and a
    call beside the event would be a second clone hook); a call inside the dom-api `CloneNode` handler (it holds neither
    the policy nor the active document, umbrella §9 N-6). Rev 4's ground for rejecting an event (umbrella §9 N-9's exhaustive-match question) is
    answered by RC-2a, which adds the variant and its match arms (the RC-2a plan §5.2); its other ground (the fragment
    and DOMParser builds detach the dispatcher, N-1) does not hold: the fragment build restores the dispatcher before
    its caller places the nodes (P-T18), DOMParser's window queues the event and dispatches it at its close (rev 4.8,
    PM decisions 711, 713; §4.5 item 4), and from rev 4.7 the fragment path consumes the same event, generalised (D-T12;
    §4.5; until rev 4.7: "is about slice b's paths, which call the entry point directly"; rev 4.7: "holds for the
    DOMParser build only").
  - **Entry point.** `process_created_subtree(dom, root, cause, policy, active)` (NEW), `pub`, in
    `session/event_handler_processing/mod.rs` (rev 4.7: `cause: CreationCause`; the fragment cause's walk is D-T12's;
    until rev 4.7: `scope`). For `CreationCause::Clone(scope)` it collects the elements of RC-2a's walk
    `for_each_clone_created(root, scope, …)` (the RC-2a plan §4.3: the scope's visit set, in *clone a node*'s creation
    order), then every element of the contents of each `<template>` in that set, first level only (the
    `WithTemplateContents` walk reaches nested contents itself) (`for_each_uncapped_shadow_inclusive(fragment,
    WalkScope::WithTemplateContents, …)`, io's form of the base's `TemplateContentsWalk::Include`, §0.1, H-T25, H-T32;
    the form arm's frontier shape, the RC-2a plan §4.5); for each it runs `process_handler_content_attributes`
    with `ProcessingTiming::Creation` (NEW variant) and marks the element (D-1). D-T7's Creation column applies: every
    class delivered to T3's target. Both walks are uncapped (O-9, PM 367) and enter `#11-capped-walker-deep-element-processing`'s
    census (§8.3 item 11).
  - **Canonical writer (PM 377).** The cloner appends each copied child as it goes (`tree_clone.rs:409`; `clone.rs:302`,
    `:442`; rev 4.4 read them on the `rc2a-clone-created` branch), and `fire_after_insert` skips only a shadow-root node or parent
    (`tree/mutation.rs:528-529`), so `handle_insert` processes **every clone element except the root and a shadow root's
    direct children** (this population: "CI-1's elements") at `ProcessingTiming::Insertion` before `CloneCreated` (the RC-2a plan §2 A × B, C × F; T3 changes neither).
    For both pieces of handler state, the `EventListeners` entry on D-T1's target and the `HandlerAttrsProcessed` mark,
    the canonical writer is the `CloneCreated` arm at Creation (the spec's attribute change steps). Creation bypasses
    the marked shortcut (`mod.rs:85`, kept for Insertion and HandlerWrite), so it delivers every applicable pair
    whatever an internal `Insert` marked. The internal `Insert` is an idempotent no-op on that state, per class: OR (and
    FN, ordinary at T3 where applicable) writes the attribute's source into the element's entry and sets the mark,
    which Creation writes again with equal values; XW writes the same target (the node document is the copy's at both,
    C1 H-C7 with C0b's adoption into the same document), which Creation writes again; EW is not delivered (D-T7).
    `set_inline_handler` reuses an existing entry (`mod.rs:266-274`), and no script or other reader of this state runs
    between the two (H-T24; *Placement*). *Not chosen:* keeping the shortcut at Creation and showing per class that the
    outcome is unchanged; that makes the internal `Insert` the OR writer, a second semantic writer. Cells T-16c (EW, OR)
    and T-16d (XW) run both.
  - **Order (Q-15).** Two Window writes collide only when two elements of one clone deliver the same Window-delegating
    name with the node document active. The walk visits a node, then its light children (DOM §4.4 *clone a node* step
    5), then its shadow children (step 6.7), so the later writer is the spec's (cell T-16b). Contents come last instead
    of at step 3, which no outcome shows: their node document is I, so step 3 of the target algorithm is null for every
    Window-delegating pair there (H-T21), and an element target is written by its own element only.
  - **What slice b inherits.** The entry point is the one slice b's other creation paths call (S-6 extended). The
    umbrella's O9 (call-site shape) is decided for the clone path and, by PM decision 705, the fragment path (D-T12);
    slice b decides it for its paths. Since
    Creation bypasses the marked shortcut, "once" holds only if each caller runs Creation once per fresh element.
  - **Population closed.** Rows EW-12, XW-11, EW-11, EW-19, OR-2 (umbrella §1.5 G5 / K3, and K2's EW half; §8.2 defect
    (2), the clone half of defect (1), (3)'s EW half, (7)'s clone clause). XW's re-raw at the clone's insertion stays
    (X-W, S-5).
  - **Fallback.** Slice a's insertion processing still runs for an unmarked element, so a future clone path that
    bypasses the canonical cloner entry is processed at its first insertion for OR and XW (HEAD-equal; umbrella §5.2
    K1); EW has no Insertion delivery (D-T7), so it would stay unprocessed (latent: no such path, H-T26). Range clones
    reach that entry from RC-2b (the RC-2a plan §3.1), so they fire `CloneCreated`. S-4's detector is slice b's, for its
    end state.
- **D-T11 — `body`'s HTML interface is chosen for an HTML `body` only (R-D1).** The VM gives a `TagType` entity a
  tag-specific HTML prototype by local name alone (H-T18). T3 makes the `body` arm of `tag_specific_t2b_prototype`
  (`elements.rs:294`) and `require_body_receiver` (`event_handler_attrs.rs:427`) also require the HTML namespace, read
  as `host.dom_shared().is_html_namespace(entity)` (`HostData::dom_shared`, `host_data/engine_feature/bind.rs:242`, the
  existing `&HostData → &EcsDom` path, precedents in `vm/host/custom_elements/{define,flush,lookup}.rs`; Q-14): a
  marshalling read; D-T11 adds no `HostData` method and edits no `host_data/` file (T3's one `host_data/` edit is D-T3's
  constructor, PM 441). A foreign `body` then takes
  `HTMLElement.prototype`, the fallback every other foreign element already takes (H-T18). Every member that loses is one
  the spec does not give that element: `HTMLBodyElement`'s own attributes and its 18 WindowEventHandlers accessors are
  on neither `SVGElement` (SVG2) nor `MathMLElement` (H-T23), and `HTMLElement.prototype` keeps the GlobalEventHandlers
  accessors both foreign interfaces include. So the change is toward the spec at every observation (row FN-3; cell
  T-19).
  *Why not every arm (the general fix):* for a tag that is also an SVG or MathML element name (`a`, `script`, `style`,
  `title`, …) the fallback `HTMLElement.prototype` would remove members the foreign interface has (e.g. `type` on
  `SVGScriptElement`), which the HTML interface supplies today: worse than the base. The general fix needs the SVG and
  MathML interface families (the master roadmap's `M4-13:#svg` row, "SVG 全 element family + SVGSVGElement", absent);
  the ledger has no slot for foreign-element interface selection (checked 2026-10-31). §8.1 item 4 is its defect text.
  *Not chosen:* every arm without a foreign namesake (`div`, `iframe`, …), toward the spec too but not reached by T3's
  change; §8.1 item 4 lists it as the first step of the fix.
- **D-T12 — the fragment path of S-1 (PM decision 705; O-11 → S-i × C-ii).** A fragment parse's elements have their
  handler content attributes processed at their creation, every pair class, by one consumer arm on one creation
  event. Grounds, in order:
  - *[spec]* HTML §13.2.6.1 *create an element for a token* step 21 appends each token attribute to the new element
    ("For each attribute of attributes, append attribute to element"), and DOM §4.9 *append an attribute* step 4 runs
    *handle attribute changes*, whose step 3 runs the attribute change steps: at the element's creation, as for a
    clone (DOM §4.4 *clone a single node* step 2.5, D-T10). A fragment parse (HTML §13.4 *HTML fragment parsing
    algorithm*) creates its elements through that algorithm, with the context's node document (*create an element for a token*
    step 6; D-T7, P-T17).
  - *One issue, one way:* one processing point for the fragment path (its creation), and one creation event for the
    clone and fragment paths, consumed by one arm calling one entry point (`process_created_subtree`).
  - *ECS-native:* the creation is an event on the existing mutation-event queue, consumed by a system (the consumer
    arm) over the created entities; no hook, no registry.
  - *Layering:* the policy and `ActiveDocument` stay in the consumer (D-T3); the parser fires a policy-free ECS
    event, and `elidex-script-session`'s mutation layer carries neither.

  *Scope (S-i; PM decisions 705, 711).* The fragment path leaves slice b's S-1 whole: **every node the parse
  created** is processed at its creation, declarative shadow content (`setHTMLUnsafe`, HTML §13.2.6.4.4 The "in head"
  insertion mode: a `template` start tag with `shadowrootmode` attaches the shadow root and parses its children into
  it) and template contents created by the parse included, and DOMParser's build included (its event is queued and
  dispatched at its window's close, §4.5 item 4). Each of those elements is created by *create an element for a
  token* and gets its attributes by its step 21, so the spec runs the attribute change steps at its creation, with its
  node document: the context's for the light and shadow content, the inert template document I for template contents
  (H-T21), the DOMParser document for its build. At the fragment path's Creation every class is delivered to D-T1's
  target and OR is marked (D-T7's column); where a placement `Insert` then reaches an element (the light tree under a
  non-shadow-root parent), it finds OR marked and re-raws XW (X-W until S-5); EW has no Insertion delivery. The walk is
  `for_each_uncapped_shadow_inclusive(root, WalkScope::WithTemplateContents, …)` (io; §0.1, H-T32; §4.5 item 6): the
  scope L3 declares as "the shadow-including tree and each template's contents", uncapped, so no element of a wide
  parent is skipped. `LifetimeUnit` is not used (PM decision 713): from a fragment root it reaches the same set (only a
  Document carries the third owned link, and no fragment root or descendant is a Document, H-T32), but its meaning is
  lifetime, not creation. The walk no longer shares `handle_insert`'s collector (PM decision 713; until rev 4.8: "the
  walk set is the placement's own insertion-time set … declarative shadow content and template contents created by
  the parse are outside it and stay as on the base, with slice b's S-1 …; elements past `MAX_ANCESTOR_DEPTH` children of
  one parent are skipped at Creation as at the placement"); `handle_insert` keeps its set. Elements placed directly
  under a shadow root (`ShadowRoot.innerHTML`, `setHTMLUnsafe` on a shadow root), which the base does not process at
  the placement (it fires no `Insert` under a shadow-root parent, `tree/mutation.rs:528-529`), are processed at
  creation (row OR-3, cell T-25); so are declarative shadow content (row OR-5, cell T-27) and template contents (row
  OR-6, cell T-28), which no placement `Insert` reaches either.
  *Call site (C-ii).* `MutationEvent::SubtreeCreated { root, cause: CreationCause::FragmentParse }`, fired by
  `parse_fragment_progressive` once per returned root after the build's dispatcher window has closed; with no
  enclosing window it is dispatched at once, before the caller's placement; inside an enclosing window (DOMParser's) it
  is queued on the slot and dispatched at that window's close (§4.5 items 3–4). The consumer's `SubtreeCreated` arm
  calls `process_created_subtree(dom, root, cause, policy, active)`. The event, its fire points, causes, walk sets,
  the queue, the consumers' migration and what T3 requires of ig and io are stated once, in §4.5.
  *Rejected (PM decision 705):* (C-i) a direct call from `elidex-script-session` `mutation/html_fragment.rs`:
  `apply_set_inner_html(dom, entity, html, opts)` (`html_fragment.rs:70-74`) receives neither the policy nor a
  document, and the policy lives in `EventHandlerAttributeConsumer { policy }` (`event_handler_consumer.rs:351-353`),
  out of reach from `EcsDom`, so C-i would route the policy and `ActiveDocument` into the mutation layer (Layering
  mandate) and add a second call site beside the clone arm; (S-ii) EW only at Creation, XW and OR at the placement's
  Insertion: two processing points on one creation path; (S-iii) every class but only for the entry points whose
  context can be active: a scope decided per entry point, a decision surface per caller; (C-iii) the call site left to
  slice b: T-22 / T-22b stay red at T3.
- **D-T13 — creation in a document without a browsing context: the raw handler is set, compilation is deferred (PM
  decision 713 (5); P-T19).** For a creation event whose elements' node document has no browsing context (a DOMParser
  document, the inert template document I; in the single-browsing-context model every document but the bound one,
  H-T15), the arm runs the same Creation processing as anywhere: the attribute change steps (HTML §8.1.8.1) do not
  consult scripting (H-T33), so for an element target (OR, FN) step 5.5 sets the **internal raw uncompiled handler** on
  the element's own handler and step 5.6 activates it; for a Window-delegating pair step 3 returns null (D-T1, the node
  document is not active) and nothing is delivered. Compilation is deferred to *get the current value of the event
  handler*, whose step 3.2 returns null while the node document's browsing context is null: N1's compile gate already
  realises that (clause (b), H-T33), so the getter reads null and a dispatch runs nothing there, and after the element
  is moved into the active document (C0's adoption) the same raw value compiles. T3 adds no code for the deferral; it
  adds the cells that pin both halves (T-26 for a DOMParser document, T-28 for template contents) and corrects
  `inert_document.rs`'s rustdoc, which says the consumer must stay off (§5.3). The element is marked at Creation
  (K-1), so its first insertion into the active document does not re-raw it over a value script wrote in between (row
  OR-4: the base does, and the spec does not).

## §1. Shape and verdict

| Axis | Own invariant |
|---|---|
| A1 class (step 1) | `body` / `frameset`, HTML namespace (D-T5) |
| A2 names (step 2) | WindowEventHandlers members ∪ the reflecting six = `is_window_delegating_pair` (D-T2) |
| A3 activeness (step 3) | `node_document` (D-T4) == `active.document`, else null |
| A4 global (step 4) | `active.window` (D-T3) |
| A5 callers | attribute change steps at every timing + AttributeChange arm; getter; setter, both pairs (D-T1, D-T6) |
| A6 processing timing of Window-delegating pairs | XW: X-W until S-5; EW: bind pass, clone creation, fragment-parse creation (PM decisions 699, 705) and the spec's writers only (D-T7) |
| A7 creation by clone or fragment parse | every element of a clone (D-T10) or every node a fragment parse created, shadow-including and template contents included (D-T12; `WalkScope::WithTemplateContents`), every class, at creation, through one event (§4.5), queued while a dispatcher window is open and dispatched at its close (§4.5 item 4) |
| A8 `body`'s interface | HTML namespace (D-T11) |
| A12 applicability (attribute change steps' step 1, local name) | `is_event_handler_content_attribute_of` by namespace (D-T8) |

Coupled from outside: A9 N1's compile-gate node clause (same reader, D-T4); A10 C0's insertion adoption and A11 C1's
inert template document, both **landed on the base** (H-T8, H-T21).

### §1.1 Outcome changes at T3 (toward the spec)

- WindowEventHandlers name on a `body` whose node document is not active (a DOMParser document, an inert template
  document, a cloned Document): base → page Window; T3 → null at every caller (facet 1; XW-3 to XW-5, XW-7, XW-8,
  XW-10).
- EW in the active document: base → the element; T3 → the Window (facets 3, 4; EW-1 to EW-4, EW-15). In a non-active
  document: base → the element; T3 → null (EW-5, EW-6, EW-13, EW-14, EW-16).
- EW is never re-raw'd at insertion (PM decision 129): a `window.on<T>` write survives a later insertion of the element
  (cell T-14), and an attribute set while the element was outside an active document is not delivered at its insertion
  (T-15; T-15b for a DOMParser-built attribute; T-17 for template contents).
- A clone's handler content attributes are processed at its creation (D-T10): an ordinary clone reads its function
  before any insertion (OR-2, T-20); a `body` / `frameset` cloned in the active document writes the Window at creation
  (EW-12, XW-11; T-16, T-20b).
- A fragment-parsed `body` / `frameset` whose node document at creation is the active document (e.g.
  `document.documentElement.innerHTML = '<body onfocus=…>'`) delivers EW to the Window at its creation (PM decision
  699; EW-20; T-22, T-22b); base → the element at the placement's insertion. Every fragment-parsed element is
  processed at its creation (D-T12), including one placed directly under a shadow root, which the base does not process at
  the placement (OR-3, T-25), declarative shadow content (OR-5, T-27) and template contents (OR-6, T-28).
- A DOMParser document's elements are processed at their creation, after the build's window closes (PM decisions 711,
  713; §4.5 item 4; D-T13): an element target gets its raw handler then (its getter reads null until the element is in
  the active document, N1's compile gate), and its first insertion into the active document no longer re-raws it over
  a value script wrote meanwhile (OR-4, T-26); Window-delegating pairs deliver nothing (step 3; EW-10, XW-12, T-22c).
- An initial `<body onload>` (and the other five, and an initial `<frameset>`'s 24) runs on the Window (H-T14; closes
  umbrella K-0's E-G1 (i)).
- A foreign-namespace `body` is not step 1's class (FN-1, FN-2) and does not get `HTMLBodyElement.prototype` (FN-3).
- A content attribute that is not an event handler content attribute of its element installs nothing (D-T8):
  `div.setAttribute('onhashchange', X)`, a Document-only name (`onreadystatechange`, `onvisibilitychange`) on any
  element, a WindowEventHandlers name on a MathML element, any name on an element outside the HTML, SVG and MathML
  namespaces (base: a handler on the element, run by a dispatch at it; FN-1b, FN-4, FN-5, OR-1).

### §1.2 Is T3 no worse than its base? (PM decision 129's premise)

Criterion: the umbrella's K-0, with T3's base as "HEAD" — at every observable step (an IDL getter read, a dispatch, a
read of `window.on<T>`), T3's outcome equals the base's, or equals the spec's where the base's does not. The base is the
state main is in just before T3 lands, which is what PM decision 129's option text compares against ("HEAD より悪い状態が
main に載らない"). §2.1 enumerates every pair class × timing × document kind. Rev 2's four residual classes, on the base:

- **R-A — template contents: closed by C1 (on C0).** Every node in template contents or in a clone of them has I as
  its node document (H-T21), so step 3 returns null there: EW-13, EW-14, XW-8 are on the spec. Cells T-17 (red), C18
  (first step rewritten); C1's own cell T23 pins the node-document facts (C1 memo §9 item 9).
- **R-B — a cross-document move then a detached state: closed by C0.** Insertion adopts (H-T8), so rev 2's kinds X1 and
  X2 are A and P: XW-9 "both", XW-10, EW-15, EW-16 "spec". Cell T-18 (guard).
- **R-C — a clone created in the active document: closed in T3** (D-T10; PM decision 153). EW-12. Cell T-16 (red).
- **R-D1 — a foreign `body`'s HTML interface: closed in T3** (D-T11). FN-3; cell T-19 (red).
- **R-D2 — a `body` outside the HTML and SVG namespaces with a WindowEventHandlers content attribute: closed in T3**
  (D-T8; PM decision 168). FN-4; cell T-21 (red).
- **R-E — a fragment-parsed `body` / `frameset` in an active-document context: closed in T3** (D-T7, D-T12; PM
  decisions 699, 705; P-T17). Without it T3 would deliver EW nowhere (no Creation on the fragment path, no EW Insertion), equal to neither
  the base (the element) nor the spec (the Window at creation). EW-20; cells T-22, T-22b (red).

So **PM decision 129's premise holds for T3: 0 rows worse than the base** (§2.1), with R-E's fragment path inside
T3 (PM decision 699).

### §1.3 Edge density

**Edge-dense** (*verdict*). Nine own axes meet in one algorithm with three spec callers and a creation path, and three
external couplings (A9–A11) change outcomes at the intersections (§2). So CLAUDE.md "Edge-dense work" applies:
plan-review is mandatory (rule (b)). **Split: none** — T3 is a narrowly-scoped per-PR slice under the approved EH-attrs
umbrella (§1.2 row T3, as edited by PM decisions 129, 153 and 168), the rule's base case (c). Splitting by facet or by
caller would leave two target rules live (content attribute vs IDL, or body vs frameset); splitting D-T7 off would land
the target change without the timing change (rev 1's W1 / W2, which PM decision 129 rejects); splitting D-T10 off
would land R-C, D-T11 off R-D1, D-T8 off R-D2, the fragment path off R-E — each worse than the base, and each placed
in T3 by PM decision 153, 168 or 699. Breadth: `webref coverage-map html 8.1.8.1 html 8.1.8.2 html 7.3.2 html 8.1.3.3.4 dom 4.4 dom 4.9 dom 4.5 html
16.3.2 SVG2 15.8 mathml-core 2.1.4` → K = 4 specs, "K≥4 or M≥20 → 分割推奨" (rev 3, without D-T8's two branch specs:
K = 2, "ok"). The re-evaluation the rule asks for is the paragraph above: the SVG2 and MathML Core rows are branches of
one predicate (D-T8), not separate mechanisms, and separating them re-opens R-D2.

## §2. Coupled invariants

- **A1×A2** — steps 1–2 are one conjunction (`is_window_delegating_pair`); a frameset passes A1 for every A2 name.
- **A1×A5** — every caller reaches A1 through D-T1 only; the VM brand check and interface choice are A8 (D-T11).
- **A1×A6** — `is_x_w_pair` reads the same namespaced class (`body` half), so X-W's population is a subset of A1×A2.
- **A1×A8** — both test the HTML namespace for `body`; they are different questions (step 1's class vs the WebIDL
  interface) and stay two predicates over one component read (`EcsDom::is_html_namespace`). A foreign `body` is neither
  step 1's class nor an `HTMLBodyElement`, so they agree on every element.
- **A1×A12** — the WindowEventHandlers half of the applicability test uses the same namespaced `is_body_or_frameset`,
  so a pair is Window-delegating only if it is applicable.
- **A2×A5** — the IDL normal pair now asks A2 too: `body.onfocus` (HTMLElement accessor) → Window (cell T-5).
- **A2×A6** — every Window-delegating pair targets the Window or null; XW is re-raw'd at insertion, EW is not.
- **A3×A5** — null at each caller: content attribute → no write and no clear; getter → null; setter → return.
- **A3×A6** — an XW re-raw at an insertion into a non-active document delivers nothing (T-3); into the active one, the
  Window (C8). EW has no insertion delivery.
- **A3×A6 (fragment creation)** — a fragment parse's elements take the context's node document (C0b D-8, D-10), so
  an active-document context delivers EW to the Window at creation (EW-20) and a non-active one nothing (EW-21);
  step 3 does not ask whether the element is connected (HTML §8.1.8.1 step 3's note; cells T-23, T-24).
- **A3×A7** — a clone's node document is its source's (DOM *clone a node*, `document` = the node's node document; the
  cloner stamps it, C1 memo H-C7), so a clone of a DOMParser `body` delivers nothing at creation (EW-11), a clone of a
  bound one the Window (EW-12), a clone of template contents nothing (EW-19).
- **A3×A9** — one reader (D-T4): N1's compile gate and step 3 cannot disagree on a node's document. A raw value set at
  creation in a document without a browsing context (D-T13) is read as null by that gate until the element's node
  document is the bound one (cells T-26, T-28).
- **A3×A10** — the node document follows the last insertion (C0): rev 2's X1 / X2 collapse to A / P (R-B closed).
- **A3×A11** — template contents and their clones are in I (C1): step 3 returns null there (R-A closed).
- **A4×A5** — `ActiveDocument` has two marshalling points (consumer construction, which the clone and fragment paths read too,
  D-T10, D-T12; VM accessors) and one constructor for both, `HostData::active_document()` (D-T3; PM 441, 455; until rev 4.5:
  "one source each (`Vm::bind`'s `document` and Window; `HostData`)").
- **A5×A7** — the clone path is a caller of D-2's function, not a fourth target rule: it reaches step 1–4 through D-T1.
- **A5×A12** — the applicability test is on the content-attribute callers only (process function and AttributeChange
  arm); the IDL getter and setter exist only where the prototype has the accessor (A8).
- **A6×A7** — at creation every class is delivered; afterwards XW alone is re-raw'd at insertion (X-W), and the marker
  set at creation keeps OR from being re-processed at the first insertion (K-1); a clone-internal Insertion before
  creation is a no-op on the same state (D-T10 *Canonical writer*). On the fragment path no Insertion reaches a
  consumer before Creation (the build's window, H-T29; inside DOMParser's window the placement's `Insert`s reach none,
  §4.5 item 3), and Creation's walk (every node the parse created) contains every element the placement's Insertion
  visits, so that Insertion finds every OR element it visits marked (D-T12; §4.5 item 6; until rev 4.8: "Creation walks
  the placement's own insertion-time set").
- **A7×A12** — a clone's attributes pass the same applicability test at creation (D-T8 inside the process function).
- **A2×A9** — N1's C8h (`dd.body.setAttribute('onfocus', 'A')`) keeps its outcome (getter null, nothing runs) but by
  step 3 now: no entry is written (cell U-T3).
- **A5×A9** — the getter determines before N1's reconcile (getter step 1 before step 3).

### §2.1 Enumeration: pair class × timing × document kind (Base / T3 / spec)

Document kinds (the node document of the element at the step; an insertion or C0c's `adoptNode` changes it, H-T8): **A** the bound, active document; **P** a DOMParser
document; **I** an inert template contents owner document (template contents and clones of them, C1); **U** no
resolvable node document (H-T16). Rev 2's kinds **X1** (a DOMParser node in a detached subtree of the bound document)
and **X2** (a bound node moved into a DOMParser document and removed again) are kept as rows to show R-B's closure and
are A and P respectively on the base. Timings: **bind** (T-a1); **AC** (`setAttribute` / `removeAttribute`, the
attribute change steps); **set** / **get** (IDL); **ins→A** / **ins→P** (an insertion into that kind, first or later);
**parse** (creation by the DOMParser build); **clone** (creation by `cloneNode`, T-c); **T-a3** (slice a's first-write
processing). Verdict: **both** (T3 = base = spec), **spec** (T3 = spec ≠ base), **base** (T3 = base ≠ spec), **✗**
(neither). † = the base differs from `4fd85f4e` (§0.1).

| Row | Class | Timing | Doc | Base | T3 | Spec | Verdict | Cells |
|---|---|---|---|---|---|---|---|---|
| XW-1 | XW | bind | A | Window | Window | Window | both | E4 (slice a) |
| XW-2 | XW | AC, set, get | A | Window | Window | Window | both | C20 (i), T-8a |
| XW-3 | XW | AC | P | page Window | nothing | nothing | spec | T-2 |
| XW-4 | XW | set | P | page Window | return | return | spec | T-1, C17 |
| XW-5 | XW | get | P | the Window's value | null | null | spec | T-1 |
| XW-6 | XW | ins→A | any | re-raw onto the Window over any write | same (X-W) | nothing | base | C8, C17, U2 |
| XW-7 | XW | ins→P | any | re-raw onto the page Window | nothing | nothing | spec | T-3, C19 |
| XW-8 | XW | AC, set, get | I | Window (†: I by C1; the target rule ignores it) | nothing / return / null | nothing / return / null | spec (R-A) | C18 (step 1) |
| XW-9 | XW | AC, set | X1 ≡ A | Window | Window | Window | both (R-B) | T-18 |
| XW-10 | XW | AC, set | X2 ≡ P | Window | nothing | nothing | spec | U-T1 |
| XW-11 | XW | clone, then ins→A | A | nothing at creation; re-raw at the insertion | Window at creation; re-raw at the insertion (X-W) | Window at creation; nothing at the insertion | creation: spec; insertion: base (XW-6) | T-20b |
| XW-12 | XW | parse, then ins→A | P → A | nothing at creation; re-raw onto the Window at the insertion (†: adopted by C0) | nothing at creation (processed after the window's close, null target, §4.5 item 4); re-raw at the insertion | nothing | creation: both; insertion: base (XW-6) | C17, T-22c |
| XW-13 | XW | AC, set | U | Window | Window | Window | both | U-T1 |
| XW-15 | XW | fragment parse (creation), then the placement's insertion | A | Window at the placement's insertion (X-W) | Window at creation (D-T12); re-raw at the placement's insertion (X-W), with the same source | Window at creation; nothing at the insertion | both (no script runs between creation and placement in one setter call: the Window's value after the call is the spec's) | T-22 |
| XW-14 | XW | clone, then ins→A | P → A | nothing at creation; re-raw at the insertion | nothing at creation (null target); re-raw at the insertion | nothing | creation: both; insertion: base (XW-6) | U-T7 |
| EW-1 | EW | bind | A | element | Window | Window | spec | T-E1, U-T2 |
| EW-2 | EW | AC | A | element | Window | Window | spec | T-4, T-6, T-8b, C19b, C19c, P3 |
| EW-3 | EW | set (where an accessor exists: the reflecting six) | A | element | Window | Window | spec | T-5, T-7 |
| EW-4 | EW | get | A | the element's value | the Window's value | the Window's value | spec | T-4, T-5 |
| EW-5 | EW | AC, set | P | element | nothing / return | nothing / return | spec | T-6b, U-T3, C19b |
| EW-6 | EW | get | P | the element's value (null for a raw value, N1) | null | null | spec | N1's C8h |
| EW-7 | EW | ins→A after an AC / set | any | re-raw onto the element over any write | nothing | nothing | spec | T-14 |
| EW-8 | EW | ins→A after an AC while in P | P → A | re-raw onto the element | nothing | nothing | spec | T-15 |
| EW-9 | EW | ins→P | any | re-raw onto the element | nothing | nothing | spec | C19b |
| EW-10 | EW | parse, then ins→A | P → A | element at the first insertion | nothing (null at creation, processed after the window's close, §4.5 item 4; no EW Insertion delivery) | nothing (null at creation) | spec | T-15b, T-22c |
| EW-11 | EW | clone, then ins→A | P → A | element at the first insertion | nothing (null at creation) | nothing | spec | U-T6, U-T7 |
| EW-12 | EW | clone | A | Window: untouched. Clone's getter: null, then the attribute's function after its first insertion | Window := the attribute at creation. Clone's getter: the Window's value | Window := the attribute at creation | spec (R-C) | T-16 |
| EW-13 | EW | AC, set | I | element (†: in I by C1 / C0) | nothing / return | nothing / return | spec (R-A) | T-17 |
| EW-14 | EW | get; ins→A afterwards | I → A | the element's value (null for a raw value, N1); at the move out, re-raw onto the element | null; nothing at the move out | null; nothing at the move out | spec (R-A) | T-17 |
| EW-15 | EW | AC, set | X1 ≡ A | element | Window | Window | spec (R-B) | T-18 |
| EW-16 | EW | AC, set | X2 ≡ P | element | nothing | nothing | spec (R-B) | U-T1 |
| EW-17 | EW | T-a3 | any | not processed | not processed | — | both (D-8) | C20 (i), U-T6 |
| EW-18 | EW | AC, set | U | element | Window | Window | spec | U-T1 |
| EW-19 | EW | clone (of contents, or of a template), then ins→A | I → A | element at the clone-internal Insertion (CI-1; the base has no Creation processing); re-raw onto the element at ins→A | nothing at creation (null); nothing at the insertion | nothing | spec | U-T7 |
| EW-20 | EW | fragment parse (creation), then the placement's insertion | A | element at the placement's insertion | Window at creation; nothing at the insertion | Window at creation; nothing at the insertion | spec (R-E, PM decision 699) | T-22, T-22b |
| EW-21 | EW | fragment parse (creation) in a context whose node document is P or I, then the placement's insertion | P, I | element at the placement's insertion | nothing (null at creation; no EW Insertion delivery) | nothing (null at creation) | spec | T-22c |
| FN-1 | FN: SVG `body` × the reflecting six; SVG `frameset` × any of the 24; MathML `body` / `frameset` × the reflecting six | AC, set | any | element | element | element (H-T22; `MathMLElement includes GlobalEventHandlers`, H-T23) | both | T-9b |
| FN-1b | FN: `body` / `frameset` outside the HTML, SVG and MathML namespaces × the reflecting six | AC | any | element | nothing (D-T8) | nothing (H-T23) | spec (P-T14) | U-T8 |
| FN-2 | FN: SVG `body` × a WindowEventHandlers name | AC | any | Window (tag-only) | element | element (H-T22) | spec | T-9 |
| FN-3 | FN: SVG, MathML or other non-HTML `body` × a WindowEventHandlers name | set, get (IDL) | any | Window (the body pair, H-T18) | an ordinary property (D-T11) | an ordinary property (no such attribute on `SVGElement` / `MathMLElement`) | spec (R-D1) | T-19 |
| FN-4 | FN: `body` outside the HTML and SVG namespaces (MathML, null, any other) × a WindowEventHandlers name | AC; ins (first, unmarked) | any | Window (tag-only); re-raw onto the Window at every insertion | nothing (D-T8) | nothing (H-T23) | spec (R-D2) | T-21, U-T8 |
| FN-5 | FN: `frameset` outside the HTML and SVG namespaces × a WindowEventHandlers name | AC | any | element | nothing (D-T8) | nothing (H-T23) | spec | U-T8 |
| FN-6 | FN pairs of FN-1, FN-2 (SVG, and the reflecting six on MathML) | ins→A, ins→P | any | re-raw at every insertion (tag-only X-W; onto the Window for FN-2's pairs) | the ordinary once rule (not Window-delegating at T3) | nothing at insertion | as OR-1 (FN-2's pairs leave the Window: spec there) | U-T6 |
| OR-1 | OR | every timing except clone, fragment parse and parse | any | element | element where applicable; nothing for a name that is not an event handler content attribute of the element (D-T8) | element, or nothing where step 1 returns | spec for the local-name test; base for a non-null attribute namespace (facet (2), slice b) | U-T1, U-T8 |
| OR-2 | OR | clone | any | nothing at creation; processed at the first insertion (once) | processed at creation (element; marked); not at the insertion | processed at creation | spec (umbrella G5 / K3; defect (2)) | T-20, C9, C5 (iii), C12 |
| OR-3 | OR | fragment parse (creation), then the placement's insertion | any | processed at the placement's insertion (once); not processed there when placed directly under a shadow root (no `Insert`, `tree/mutation.rs:528-529`) | processed at creation (element; marked); not at the placement (D-T12) | processed at creation | both (no script runs between creation and placement); spec under a shadow root | T-25, U-T9 |
| OR-4 | OR | parse (creation, DOMParser build), IDL write in P, then ins→A (rev 4.8, PM decisions 711, 713) | P → A | nothing at creation (the window drops every event); getter in P null (no entry); an IDL write on an element with two handler attributes does not process it (D-8); processed at the first insertion (unmarked), its raw value overwriting the IDL write | processed at creation after the window's close (raw on the element, marked; D-T13); getter in P null (N1's compile gate, step 3.2); the IDL write stays; nothing at the insertion | raw at creation; null in P; the IDL write stays | spec | T-26 |
| OR-5 | OR | fragment parse creating declarative shadow content (`setHTMLUnsafe`), A (rev 4.8, PM decision 711) | A | not processed (no placement `Insert` enters a shadow tree; umbrella §5.1 "not processed in a fragment parse") | processed at creation (element; marked; D-T12) | processed at creation | spec | T-27, U-T9 |
| OR-6 | OR | fragment parse creating template contents, IDL write in I, then the contents moved into A (rev 4.8, PM decision 711) | I → A | nothing at creation (no placement `Insert` enters template contents); getter in I null; processed at the first insertion into A (unmarked), its raw value overwriting the IDL write | processed at creation (raw on the element, marked; D-T12, D-T13); getter in I null (N1's compile gate); the IDL write stays; nothing at the insertion | raw at creation; null in I; the IDL write stays | spec | T-28, U-T9 |
| CI-1 | XW, EW, OR | clone-internal Insertion (CI-1's elements, D-T10 *Canonical writer*) | the copy's | XW: the page Window (the target rule ignores the document); EW: element; OR: element, marked | XW, OR: Creation's write, made first here (no-op, D-T10); EW: nothing | no step (Creation's outcome) | both for OR and for XW in A; spec otherwise | T-16c, T-16d, T-16b |

The *Base* cells of XW-11, XW-14, EW-11, EW-12 and OR-2 are those of clone elements outside CI-1's elements (D-T10);
the rest (EW-19's included) are first processed at CI-1. Rows not listed are covered by a listed one: `removeAttribute` follows its row's AC column (clear instead of set, on
the same target); a later bind processes nothing for every class (D-5); XW and EW in a non-active document at the bind
pass do not occur (the bind pass walks only the bound document, umbrella K-5); a fragment-parsed `body` / `frameset`
is created in the context's node document (P-T17): rows EW-20 and XW-15 (A) and EW-21 (P, I), and XW in a non-active
context is XW-7's case (until rev 4.6 the memo said the throwaway document, which the spec does not use); a cloned Document's
`body` is kind P's case with a cloned Document in place of the DOMParser document (not active; spec null).

**Rows worse than the base: 0** (PM decisions 168, 699, 705, 711; EW-20 is on the spec because T3 processes the fragment path at
creation, OR-3 is base-equal or on the spec, and OR-4 to OR-6 are on the spec). OR-1's namespaced-attribute half is base-equal (slice b).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.8.1 Event handlers | *determine the target of an event handler* steps 1–4 | 1 not an HTML body/frameset → self; 2 not WEH and not reflecting → self; 3 node document ≠ active document → null; 4 → active Window | `determine_event_handler_target` (NEW) in `crates/script/elidex-script-session/src/event_handler_processing/mod.rs`, replacing `resolve_handler_target` (`:249`) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | event handler IDL attribute getter steps 1–3 | null target → null; else get the current value (N1's reconcile) | `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` `native_event_handler_get` `:323`, `native_body_weh_get` `:439` | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | event handler IDL attribute setter steps 1–4 | null target → return; null value → deactivate; else activate | `native_event_handler_set` `:344`, `native_body_weh_set` `:461` | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | attribute change steps steps 1–5 | step 1 local name not an event handler content attribute of the element → return (D-T8); null target → return (no set, no clear); step 1's namespace test and step 5.1 not touched (§7) | `crates/script/elidex-script-session/src/event_handler_consumer.rs:420`; `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:120-132` (D-T7's delivery by class and timing; `is_x_w_pair` (NEW); `is_event_handler_content_attribute_of` (NEW)) | ✗ | yes |
| WHATWG HTML §8.1.8.2 Event handlers on elements, Document objects, and Window objects | the WEH table and the Window-reflecting body element event handler set | content and IDL attributes on body and frameset act on the Window; WEH content attributes on body and frameset only | `is_window_delegating_pair` `mod.rs:216`; `WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET` `mod.rs:42` (unchanged); the HTML branch of `is_event_handler_content_attribute_of` (NEW) | ✓ | yes |
| SVG2 §15.8 Event attributes | every supported event type is an event attribute of every SVG element | SVG element → every name applicable | the SVG branch of `is_event_handler_content_attribute_of` (NEW) | ✓ | yes |
| MathML Core §2.1.4 Attributes common to HTML and MathML elements | the event handler content attributes HTML supports on all HTML elements | MathML element → GlobalEventHandlers names only | the MathML branch of `is_event_handler_content_attribute_of` (NEW) | ✓ | yes |
| WHATWG HTML §7.3.2 Browsing contexts | *active document* | in HEAD's model: the bound document (H-T6, H-T15) | `ActiveDocument` (NEW), constructed only by `HostData::active_document()` (`crates/script/elidex-js/src/vm/host_data/engine_feature/bind.rs`, PM 441) and read at `crates/script/elidex-js/src/vm/vm_api.rs:301` and in `event_handler_attrs.rs` (PM 455) | ✗ | no |
| WHATWG HTML §8.1.3.3.4 Relevant | *relevant global object* of the node document | active document → its Window (navigable's active window, §7.3.1) | `ActiveDocument::window` (NEW) | ✗ | no |
| WHATWG DOM §4.4 Interface Node | *node document*; *clone a single node* step 2.5 (attributes appended to the copy); *clone a node* steps 5 and 6.7 (light children, then shadow children) | the base's reader (D-T4); a clone's attributes are processed at its creation with the copy's node document, in creation order | `EcsDom::node_document` (C0b D-1, landed); `process_created_subtree` (NEW) from the consumer's `SubtreeCreated` arm (cause `Clone`, §4.5), over `EcsDom::for_each_clone_created` (RC-2a) | ✗ | yes |
| WHATWG HTML §13.2.6.1 Creating and inserting nodes | *create an element for a token* step 6 (document = targetParent's node document, after the insertion-location redirection of step 2), step 21 (append each attribute) | a fragment parse's elements are created in the context's node document, their attributes appended at creation | `parse_fragment_progressive` (`crates/dom/elidex-html-parser/src/lib.rs:223`) fires `SubtreeCreated` (cause `FragmentParse`) per returned root (§4.5); `process_created_subtree` at Creation from the same arm (D-T12) | ✗ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | *HTML fragment parsing algorithm* steps 1–2 (the parser's own new `Document`; the fragment in target's node document) | the elements are not created in step 1's Document (P-T17) | C0b's `node_document_for_new_nodes` redirection (landed); T3 reads it through D-T4 | ✗ | yes |
| WHATWG HTML §13.2.6.4.4 The "in head" insertion mode | a `template` start tag with `shadowrootmode` (declarative shadow root) | the shadow root's children are created by *create an element for a token* inside the fragment parse (rev 4.8, PM decision 711) | `process_created_subtree`'s `FragmentParse` walk, `WalkScope::WithTemplateContents` (io; D-T12) | ✗ | yes |
| WHATWG HTML §8.5.1 The DOMParser interface | `parseFromString` steps 3–4 (a new Document; "Since document does not have a browsing context, scripting is disabled") | the build's elements are created in that Document; their creation event is queued in the build's window and dispatched at its close (rev 4.8, PM decisions 711, 713) | `crates/dom/elidex-form/src/inert_document.rs` (`parse_into_inert_document`, under ig's guard); `EcsDom::fire_subtree_created`'s queue arm (§4.5 item 4) | ✗ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | attribute change steps 5.5–5.6 (raw uncompiled value, activate); *get the current value of the event handler* step 3.2 (null while the node document's browsing context is null) | a creation in a document without a browsing context sets the raw handler; compilation reads null there (D-T13) | the Creation timing of `process_handler_content_attributes`; N1's compile gate (`scripting_disabled_for_platform_object`, `crates/script/elidex-script-session/src/scripting.rs`, unchanged) | ✗ | yes |
| WHATWG DOM §4.9 Interface Element | *append an attribute* step 4; *handle attribute changes* step 3 | the attribute change steps run for each appended attribute | `ProcessingTiming::Creation` (NEW) in `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:50` | ✗ | yes |
| WHATWG DOM §4.5 Interface Document | *element interface* (`Element` unless a spec says otherwise) | HTML interfaces for HTML-namespace elements only; `body`'s arm (D-T11) | `crates/script/elidex-js/src/vm/host/elements.rs:294`; `require_body_receiver` `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs:418` | ✗ | yes |
| WHATWG HTML §16.3.2 Frames | `frameset` element; `HTMLFrameSetElement includes WindowEventHandlers` | content attributes and inherited GlobalEventHandlers accessors → step 1; the interface itself absent (compat, D-T9) | `is_body_or_frameset` `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:228` (namespaced, D-T5) | ✗ | yes |

Numbers and titles (2026-10-31): `webref heading html 8.1.8.1` (Event handlers, `#event-handler-attributes`),
`8.1.8.2` (Event handlers on elements, Document objects, and Window objects), `8.1.8.2.1` (IDL definitions), `7.3.2`
(Browsing contexts, `#windows`), `7.3.1` (Navigables), `8.1.3.3.4` (Relevant), `16.3` (Requirements for
implementations), `16.3.2` (Frames), `3.1.7` (DOM tree accessors), `4.12.3` (The template element), `7.2.2` (The Window
object), `8.1.3.4` (Enabling and disabling scripting); `webref heading dom
4.4` (Interface Node), `4.2.3` (Mutation algorithms), `4.5` (Interface Document), `4.9` (Interface Element); `webref
heading SVG2 15.8` (Event attributes, `#EventAttributes`); `webref heading mathml-core 2.1` (Elements and attributes),
`2.1.3` (Global Attributes), `2.1.4` (Attributes common to HTML and MathML elements,
`#attributes-common-to-html-and-mathml-elements`), `2.2.3` (DOM and JavaScript, `#dom-and-javascript`). `webref dfn
html`: 'determine the target of an event handler' → §8.1.8.1 `#determining-the-target-of-an-event-handler`;
'Window-reflecting body element event handler set' → §8.1.8.2; 'active document' → §7.3.2 `#active-document` (and
for=navigable §7.3.1 `#nav-document`); 'relevant global object' → §8.1.3.3.4 `#concept-relevant-global`; 'associated
Document' → §7.2.2 `#concept-document-window`; 'appropriate template contents owner document' → §4.12.3
`#appropriate-template-contents-owner-document`; 'WindowEventHandlers' → §8.1.8.2.1; 'frameset' → §16.3.2 `#frameset`;
`webref dfn dom`: 'node document' → §4.4 `#concept-node-document`, 'clone a node' → §4.4 `#concept-node-clone`, 'clone a
single node' → §4.4 `#clone-a-single-node`, 'append an attribute' → §4.9 `#concept-element-attributes-append`, 'handle
attribute changes' → §4.9 `#handle-attribute-changes`, 'attribute change steps' → §4.9
`#concept-element-attributes-change-ext`, 'element interface' → §4.5 `#concept-element-interface`, 'adopt' → §4.5
`#concept-node-adopt`; `webref dfn mathml-core`: 'MathMLElement' → §2.2.3 `#dom-mathmlelement`, 'Unknown MathML element'
→ §2.1 `#dfn-unknown-mathml-element`. Step texts: `webref body html event-handler-attributes`,
`event-handlers-on-elements,-document-objects,-and-window-objects`, `frames`, `navigables`, `relevant`,
`the-template-element`; `webref body dom mutation-algorithms`, `concept-node-clone` (steps 3, 5, 6.7 re-read
2026-11-05), `clone-a-single-node` (step 2.5 re-read 2026-11-05),
`concept-element-attributes-append`, `handle-attribute-changes`, `concept-element-interface`; `webref body SVG2
EventAttributes`; `webref body mathml-core global-attributes`, `attributes-common-to-html-and-mathml-elements`,
`dom-and-javascript`, `mathml-elements-and-attributes`; `webref idl html HTMLFrameSetElement`, `webref idl mathml-core
MathMLElement`, `webref idl SVG2 SVGElement`. Rev 4.7 (2026-11-13): `webref heading html 13.2.6.1` (Creating and
inserting nodes, `#creating-and-inserting-nodes`), `13.4` (Parsing HTML fragments, `#parsing-html-fragments`); `webref
dfn html`: 'create an element for a token' → §13.2.6.1 `#create-an-element-for-the-token`, 'HTML fragment parsing
algorithm' → §13.4 `#html-fragment-parsing-algorithm`; `webref dfn dom`: 'create an element' → §4.9
`#concept-create-element`; step texts `webref body html creating-and-inserting-nodes` (steps 2, 6, 17, 21),
`html-fragment-parsing-algorithm` (steps 1, 2), `webref body dom concept-element-attributes-append` (step 4),
`handle-attribute-changes` (step 3), `clone-a-single-node` (step 2.5), `concept-create-element` (steps 4.4, 5.2.2).
Rev 4.8 (2026-11-13): `webref heading html 13.2.6.4.4` (The "in head" insertion mode, `#parsing-main-inhead`), `8.5.1`
(The DOMParser interface, `#the-domparser-interface`), `8.1.3.4` (Enabling and disabling scripting,
`#enabling-and-disabling-scripting`); `webref dfn html`: 'get the current value of the event handler' → §8.1.8.1
`#getting-the-current-value-of-the-event-handler`, 'internal raw uncompiled handler' → §8.1.8.1
`#internal-raw-uncompiled-handler`, 'activate an event handler' → §8.1.8.1 `#activate-an-event-handler`; step texts
`webref body html event-handler-attributes` (attribute change steps 1–5.6; *get the current value* steps 3.1–3.2),
`dom-domparser-parsefromstring` (steps 3–4), `creating-and-inserting-nodes` (step 21 re-read).

### §3.1 User-input touch audit

Author-controlled inputs reaching the changed code: handler content attributes on any element (parser-baked and
`setAttribute`; in a DOMParser document they reach the consumer only after the build, H-T9); IDL handler writes on
`body` / `frameset` / any element (`el.on<T> = …`); `DOMParser` documents; tree moves between documents (adopted by C0,
re-read at each determination); template contents (in I, C1); `createElementNS` with any namespace and local name;
`cloneNode` (shallow and deep, with clonable shadow roots and templates); the order of `window.on<T>` writes, clones and
insertions. Every change either narrows where a handler lands (null instead of the page Window for a non-active
document; nothing for an attribute that is not an event handler content attribute of its element), moves it to the
spec's target in the same realm (the active document's own Window), processes a clone's attributes at its creation as
the spec does, or removes an HTML interface from a foreign `body`. No case delivers a handler to a Window other than the
bound document's (H-T15; the unbind caveat is a caller contract that every production bind keeps). The clone path adds
no new delivery target: each delivery is D-T1's. Security boundaries (CSP, sandbox) are not changed; CSP stays
`#11-csp-inline-handler-enforcement`.

## §4. Mechanism

### §4.1 Home / kind

| Fact | Home / kind | Status |
|---|---|---|
| target determination | `determine_event_handler_target` (NEW), free fn, `session/event_handler_processing/mod.rs` | D-T1 |
| active-document context | `ActiveDocument` (NEW), `Copy` struct of two `pub` entities, same module; constructed only by `HostData::active_document()` (NEW, `host_data/engine_feature/bind.rs`; PM 441; both NEW unless the event-path PR-B landed first); held by `EventHandlerAttributeConsumer` (constructor `EventHandlerAttributeConsumer::new(policy, active)`, replacing `with_policy`; `Default` derive removed), which `Vm::bind` builds from it; read per call by the VM accessors through it (PM 455) | D-T3 |
| node document | `EcsDom::node_document` (C0b D-1, landed), read only | D-T4 |
| step-1 class | `is_body_or_frameset` (existing, session-private, gains the HTML namespace) | D-T5 |
| steps 1–2 | `is_window_delegating_pair` (kept) | D-T2 |
| applicability (step 1 of the attribute change steps, local name) | `is_event_handler_content_attribute_of` (NEW), session-private, same module | D-T8 |
| X-W's remaining population | `is_x_w_pair` (NEW), session-private, removed by S-5 | D-T7 |
| creation processing (clone and fragment paths) | `process_created_subtree` (NEW), `pub`, same module, taking the event's `CreationCause`; `ProcessingTiming::Creation` (NEW); the element marker (D-1, existing); the fragment cause's walk is io's `for_each_uncapped_shadow_inclusive(root, WalkScope::WithTemplateContents, …)` (until rev 4.8: `handle_insert`'s collector) | D-T10, D-T12, D-T13 |
| the creation event | `MutationEvent::SubtreeCreated { root, cause }` and `CreationCause` (NEW in `elidex-ecs`, replacing `MutationEvent::CloneCreated`); `EcsDom::fire_subtree_created` (replacing `EcsDom::fire_clone_created`); fired by the dom-api cloner and by `parse_fragment_progressive` | §4.5 |
| the window's pending creation events | `DispatcherSlot::Taken { pending: Vec<Entity> }` (the slot's existing private enum, `ecs/dom/mod.rs:55-59`, gains the payload; World-level state, no component: per-window, not per-entity, as ig's guard, H-T32); pushed by `fire_subtree_created`, flushed by ig's outermost close (T3 adds the flush there) | §4.5 item 4 |
| `body`'s interface | the `body` arm of `tag_specific_t2b_prototype` and `require_body_receiver`, each adding `EcsDom::is_html_namespace` through `HostData::dom_shared` | D-T11 |
| deleted | `resolve_handler_target`, `is_body_element`, `EcsDom::window_entity` | D-T1, D-T3, D-T5 |

No component, store, trait or crate edge is added (`elidex-js` already depends on `elidex-script-session`;
`elidex-html-parser` already depends on `elidex-ecs`). One event variant is replaced, not added, and one existing
enum variant gains a payload (§4.5).

### §4.2 Shape

```rust
// session/event_handler_processing/mod.rs
pub fn determine_event_handler_target(          // (NEW)
    dom: &EcsDom, target: Entity, name: &str, active: ActiveDocument,
) -> Option<Entity> {
    if !is_window_delegating_pair(dom, target, name) {   // steps 1–2
        return Some(target);
    }
    let node_document = dom.node_document(target).unwrap_or(active.document); // D-T4 (C0b D-1)
    (node_document == active.document).then_some(active.window)   // steps 3–4
}

pub fn process_created_subtree(                 // (NEW), D-T10, D-T12; called by the SubtreeCreated arm (§4.5)
    dom: &mut EcsDom, root: Entity, cause: CreationCause, policy: SpecLevelPolicy, active: ActiveDocument,
) { /* Clone(scope): for_each_clone_created (RC-2a), then each template's contents;
       FragmentParse: root + for_each_uncapped_shadow_inclusive(root, WalkScope::WithTemplateContents) (io; §4.5 item 6);
       each element: process at Creation, mark */ }

// session/event_handler_consumer.rs, EventHandlerAttributeConsumer::handle (NEW arm):
MutationEvent::SubtreeCreated { root, cause } =>
    process_created_subtree(dom, root, cause, self.policy, self.active),

// in process_handler_content_attributes (D-T7):
if marked && timing != ProcessingTiming::Creation && !is_body_or_frameset(dom, element) { return; } // D-T10
// per handler content attribute:
if !is_event_handler_content_attribute_of(dom, element, &name) { continue; } // D-T8 (NEW)
let deliver = match (is_window_delegating_pair(dom, element, &name), timing) {
    (_, ProcessingTiming::Creation) => true,                          // every class (NEW)
    (false, _) => !marked,                                            // OR: once
    (true, ProcessingTiming::Bind) => true,                           // XW, EW
    (true, ProcessingTiming::Insertion) => is_x_w_pair(dom, element, &name), // XW only (NEW)
    (true, ProcessingTiming::HandlerWrite) => false,
};
// then: target = determine_event_handler_target(...); None → skip
```

Call sites of `determine_event_handler_target` after T3 (re-runnable: `/usr/bin/grep -rn
'determine_event_handler_target(' crates` → exactly the five below plus the definition):
1. `process_handler_content_attributes` — `None` → `continue` (no write); signature gains `active` (so do
   `process_bound_document` and `process_before_handler_write`; the VM callers marshal it).
2. `handle_attribute_change` — after the applicability test (D-T8); `None` → return before set / clear.
3. `native_event_handler_get` / 4. `native_event_handler_set` — the normal pair, with `ActiveDocument` from `HostData`.
5. the body pair's shared core (`native_body_weh_get` / `_set` call one helper after `require_body_receiver`).

`is_event_handler_content_attribute_of(` → the definition and two calls (items 1 and 2). `process_created_subtree(` →
the definition and one call (the consumer's `SubtreeCreated` arm, both causes; rev 4.7). The VM accessors read `ActiveDocument` through one host helper
that delegates to `HostData::active_document()` (D-T3; PM 441: no second constructor; `None` = unbound → the accessor's
existing silent no-op, G7; until rev 4.5: "`ActiveDocument` from `host.document_entity_opt()` and
`host.window_entity()`"). It reads `HostData` only (marshalling,
Layering mandate); the `EcsDom::*` calls added in `vm/host/` are the namespace read of D-T11 (a component read for
interface selection, marshalling) and none other. The marked shortcut (`mod.rs:85`) no longer applies at Creation (D-T10
*Canonical writer*); at the other timings its "only `body` / `frameset` have Window-delegating pairs" stays a sound
over-approximation.

### §4.3 ECS-native and Layering checks

The algorithm is a free function over the ECS DOM (a system-like read: tag/namespace components, tree relation,
`AssociatedDocument` through `node_document`), returning an entity the caller writes `EventListeners` on, the existing
component. The applicability test is a pure read of the same tag/namespace components. The clone path is one more
system run over a freshly created subtree, writing the same component and the existing marker. OO → ECS idiom: the
"override the accessor on the subclass prototype" pattern (`HTMLBodyElement.prototype`'s delegation pair) is replaced,
for the target choice, by one query on the receiver entity; the prototype install remains only as the WebIDL surface,
now chosen by namespace as well as tag for `body`. "Run the attribute change steps when the clone appends an attribute"
(an observer on the attribute list in OO engines) → one more consumer arm on the clone's creation event (RC-2a's
`CloneCreated`), a system over the created entities. "Each element class declares its supported handler attributes"
(per-class tables in OO engines) → one predicate over the namespace and class components. The fragment path is the same
system on the same event with another cause (D-T12, §4.5): "the parser runs the attribute change steps as it appends
each attribute" (a per-element callback inside the parser in OO engines) → one event per created subtree after the
build, consumed by the same arm. `ActiveDocument` is a
per-call value, not a store (D-T3, PM
decision 132). Layering: steps 1–4, D-T7's timing, D-T8's test and both creation walks live in
`elidex-script-session`; the parser and the cloner only fire the policy-free `elidex-ecs` event; `vm/host/` brand-checks, chooses the prototype, marshals `HostData` and the policy, and executes
the result on the listener store.

### §4.4 Docstring citation table (Axis 4)

| Item | Cites |
|---|---|
| `determine_event_handler_target` (NEW) | HTML §8.1.8.1 *determine the target of an event handler* steps 1–4; step 3 = "not an active document" in the single-browsing-context model (H-T6); B1 hand-off; the node document is `EcsDom::node_document` (C0b) |
| `ActiveDocument` (NEW) | HTML §7.3.2 *active document*; §8.1.3.3.4 *relevant global object*; §7.3.1 (active window = active document's relevant global); `#11-browsing-context-state-ecs-components` (PM decision 132); the unbind caveat (H-T15) |
| `is_body_or_frameset` | HTML §8.1.8.1 step 1 ("a body element or a frameset element", HTML namespace); §16.3.2 `frameset`; the clause "as [`is_body_element`] compares it" (`mod.rs:224-225`) is removed with that function (D-T1) |
| `is_window_delegating_pair` | "Reads the tag and the name only" (`mod.rs:213-215`) → the tag, the namespace (D-T5) and the name |
| `is_event_handler_content_attribute_of` (NEW) | HTML §8.1.8.1 attribute change steps step 1 (local-name half); §8.1.8.2 (WEH content attributes on body and frameset); SVG2 §15.8; MathML Core §2.1.4 (the `MathMLElement` IDL: GlobalEventHandlers); the namespace half → `#11-event-handler-content-attribute-applicability` facet (2), slice b |
| `is_x_w_pair` (NEW) | umbrella §0.5 X-W as narrowed by PM decision 129; removed by slice b's S-5; slot `#11-eh-attrs-creation-time-call-sites` defect (7) |
| `process_created_subtree` (NEW), `ProcessingTiming::Creation` (NEW) | DOM §4.4 *clone a single node* step 2.5; HTML §13.2.6.1 *create an element for a token* step 21; DOM §4.9 *append an attribute* step 4, *handle attribute changes* step 3; umbrella S-1 (clone path, PM decision 153; fragment path, PM decision 705); the entry point slice b's other paths call (S-6) |
| `native_event_handler_get` / `_set`, body pair | §8.1.8.1 getter steps 1–3 / setter steps 1–4 |
| the `body` arm, `require_body_receiver` | DOM §4.5 *element interface*; HTML `HTMLBodyElement`; the foreign-interface defect (§8.1 item 4) |
| `handle_attribute_change`, `process_handler_content_attributes` | §8.1.8.1 attribute change steps 1–3 (the docstring's "Not realised: step 3" at `mod.rs:64-68` is removed; the X-W paragraph `:70-75` is rewritten for D-T7) |
| `EventHandlerAttributeConsumer::handle`, the `SubtreeCreated` arm | §4.5 (the event); DOM §4.4 *clone a node* steps 5 and 6.7 (the clone walk's order, D-T10); the fragment walk = every node the parse created, `WalkScope::WithTemplateContents` (D-T12); HTML §8.1.8.1 attribute change steps 5.5–5.6 and *get the current value of the event handler* step 3.2 (D-T13) |
| `MutationEvent::SubtreeCreated`, `CreationCause`, `EcsDom::fire_subtree_created` (NEW, replacing `CloneCreated` / `fire_clone_created`) | §4.5 items 1–4: DOM §4.4 *clone a node* (steps 2, 5, 6.7 define the clone's created set; RC-2a's text kept for the `Clone` cause); HTML §13.4 *HTML fragment parsing algorithm* and §13.2.6.1 *create an element for a token* step 21 (the `FragmentParse` cause); the D-4 assertion for the `Clone` cause and the `FragmentParse` cause's queue while `Taken` (§4.5 item 4; until rev 4.8: "drop") |
| `parse_fragment_progressive`'s fire | §4.5 item 3: once per returned root, after both backends' restore; before the caller's placement, or queued until the enclosing window's close |
| `DispatcherSlot::Taken`'s `pending`, ig's close (the flush) | §4.5 item 4 (PM decisions 711, 713): restore, then dispatch in order skipping dead roots; discarded on unwind |
| `parse_into_inert_document`'s rustdoc (`inert_document.rs:64-90`, corrected) | HTML §8.1.8.1 attribute change steps (no scripting test) and *get the current value of the event handler* step 3.2; §8.5.1 `parseFromString` step 4's note (scripting disabled because no browsing context); D-T13, P-T19 |

### §4.5 The creation event `SubtreeCreated` (PM decisions 705, 711, 713, 714; for L3's review)

The one statement of the event's shape; every other site of this memo refers here. Coordinates are `cee74989`'s
(H-T29 to H-T31 carry the commands); L3's names are the PR (ig) and PR (io) plans', cited by symbol and section at
the snapshot H-T32 names (§0.1; re-pinned to L3's ratified sha before T3 is ratified, R7). `MutationEvent::CloneCreated` is RC-2a's (L3's) ratified surface; T3 is the
event's first new consumer, so the generalisation lands in T3's PR (PM decision 705). Sibling plans that name
`CloneCreated` (the RC-2a plan, the Range program, the C0 and C1 memos) keep it as the name at their time. **T3 lands
after io, so after ig** (umbrella x12; PM decisions 711, 714): the queue's flush point is ig's outermost close (item
4) and the fragment walk's scope is io's `WalkScope` (item 6).

1. **Variant.** `MutationEvent::CloneCreated { root, scope }` is replaced, with no coexistence, by
   ```rust
   // crates/core/elidex-ecs/src/dom/mutation_event.rs (re-exported from elidex-ecs's lib.rs, as CloneScope is)
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum CreationCause {
       Clone(CloneScope),   // DOM §4.4 "clone a node"; CloneScope unchanged (RC-2a)
       FragmentParse,       // HTML §13.4 "HTML fragment parsing algorithm"
   }
   // in enum MutationEvent<'a>:
   SubtreeCreated { root: Entity, cause: CreationCause },
   ```
   `root` and every node in its subtree were created by one operation of the cause, and none of them has been
   observed by script yet. `CloneScope` and `EcsDom::for_each_clone_created(root, scope, …)` stay as they are: the
   clone's visit set (the RC-2a plan §4.3), which the clone consumers read from the `Clone` cause's scope.
   `EcsDom::fire_clone_created(root, scope)` is replaced by `EcsDom::fire_subtree_created(root, cause)`.
2. **Spec ground (both causes).** Each is element creation whose attributes are appended at creation: DOM §4.4 *clone
   a single node* step 2.5, HTML §13.2.6.1 *create an element for a token* step 21; DOM §4.9 *append an attribute*
   step 4 → *handle attribute changes* step 3 → the attribute change steps. elidex batches those per-element runs into
   one event per created subtree, dispatched before script can observe the subtree (RC-2a's precedent for the clone).
   The attribute change steps run for every created element, also in a document without a browsing context: they
   do not consult scripting; the null for such a document is at compile time (*get the current value of the event
   handler* step 3.2; H-T33, D-T13).
3. **Fire points (two production sites).**
   - `Clone(scope)`: `clone_node_with_shadow_honor` (`crates/dom/elidex-dom-api/src/node_methods/clone.rs:305-311`),
     unchanged: once, after the whole copy exists, before the cloning entry returns; scope `Subtree` / `SelfOnly` as
     on the base.
   - `FragmentParse`: `elidex_html_parser::parse_fragment_progressive` (`crates/dom/elidex-html-parser/src/lib.rs:223-247`),
     after its backend match: once per returned root, in the returned order (a Text root too; consumers ignore what
     they do not walk). One site serves both backends: each backend's own window is closed when it returns (P1, item
     5), the strict one after `derive_element_components` (`lib.rs:235-237`), the tolerant one direct or as the
     fallback after a strict reject. A strict reject's partial subtree was despawned (`tree_builder/mod.rs:214-236`)
     and fires nothing. Every production fragment parse goes through this function (H-T29), so every entry point gets
     the event: `innerHTML` on Element / ShadowRoot / template, `setHTMLUnsafe`, `outerHTML`, `insertAdjacentHTML`,
     the queued `Mutation::SetInnerHtml`, and DOMParser's build. `Range.createContextualFragment` is a stub that
     parses nothing; no `document.write` or `parseHTMLUnsafe` exists (H-T29).
   - **Order against the placement.** With no enclosing window (every entry point but DOMParser's) the event is
     dispatched at the fire, and each caller places the returned nodes after the return, so the event precedes every
     `Insert` of the placement. Inside DOMParser's window (the one production window that encloses a fragment parse,
     H-T30) the event is queued (item 4) and dispatched at that window's close, so it **follows** the placement under
     the synthesized `html` element (`inert_document.rs:142`; its `Insert`s reach no consumer, as on the base) and the
     two structural reconcilers (form-control state `:172-174`, base URL `:200`); no script runs inside the window, so "before script can observe the
     subtree" still holds (PM decision 713 (3); until rev 4.8: "Each caller places the returned nodes after the
     return, so the event precedes every `Insert` of the placement", without the nested case).
   - Not a fire point: the document parse (pre-bind; its creation-equivalent point is the first bind, umbrella K-6).
4. **While the dispatcher is `Taken`: queued, dispatched at the outermost close (PM decisions 711 (1), 713 (2)).**
   The window's state has one home, the slot (World-level state; ECS-native, one issue one way): T3 gives
   `DispatcherSlot::Taken` the window's pending creation events.
   ```rust
   // crates/core/elidex-ecs/src/dom/mod.rs (T3; on ig's head)
   enum DispatcherSlot {
       Absent,
       Installed(Box<dyn MutationDispatcher + Send + Sync>),
       /// A take window is open (ig's `DispatcherTake`). `pending`: the roots of the `FragmentParse` creation
       /// events fired inside it, in fire order, dispatched at the close of the guard that took the dispatcher.
       Taken { pending: Vec<Entity> },
   }
   ```
   - `fire_subtree_created(root, cause)`: slot `Installed` → dispatched at once (`dispatch_event`); `Absent` (unbound)
     → nothing, both causes, as on the base; `Taken` with `FragmentParse` → `pending.push(root)`, no assertion;
     `Taken` with `Clone(_)` → D-4's `debug_assert!` (the RC-2a plan §4.7: no clone inside a window), kept as an
     internal invariant shown unreachable (PM 584; PM decision 711 (1)), and in release the event is dropped as on the
     base. Every other event in a window is dropped as today (`dispatch_event`'s `Absent` / `Taken` arm).
   - **The outermost close** (`DispatcherTake::close` of the guard that took the box, PR (ig) plan §4.1; T3's change
     to it): `pending` leaves the slot in the same replacement that writes `Installed(box)` back, **then**
     `SubtreeCreated { root, cause: FragmentParse }` is dispatched for each root in order, skipping a root that is no
     longer alive (`EcsDom::contains`). The restore comes first, so the flush dispatches through the installed
     dispatcher, and a consumer panic during the flush leaves the slot `Installed` (`dispatch_event`'s own guard).
     This is the order the PR (ig) plan §8 *T3 intersection* assigns to T3. Which code holds the restore and which
     the dispatch, given ig's `Drop` as the one site of the restore and `close`'s body `drop(self)`, is open (P-T20
     *Residual*); the order above holds whichever shape is chosen. (Until rev 4.8.1: "it writes `Installed(box)`
     back … then dispatches", a restore written in the close.)
   - **Unwind** (ig's `Drop` reached without `close`): restores the box and **discards** `pending`; no consumer runs
     during an unwind (and ig's `Drop` keeps "does nothing that can panic" on that path, PR (ig) plan §4.1 *Why it is
     unwind-safe* item 2, H-T32).
   - A nested guard (its take found the slot `Taken` and holds nothing): its close and its drop leave the slot, and
     `pending`, untouched. `clear_mutation_dispatcher` inside a window (unbind) writes `Absent`: `pending` is
     discarded with the window, since no consumer is left to receive it. `take_mutation_dispatcher` writes `Taken {
     pending: Vec::new() }`.
   - **What T3 relies on: ig's guarantees (a) and (b), plus T3's change to `close` (PM decision 713; the PR (ig) plan
     §8 *T3 intersection*, H-T32).** (a) The restore point is the outermost guard's `close` (`DispatcherTake::close`,
     consuming, the explicit restore point on a returning path; ig's `Drop` stays the unwind backstop), so on a
     returning path the flush point exists and is distinct from the unwind `Drop`; (b) the slot state is the single
     home of take/restore state (no window keeps a second flag or depth counter; the guard holds only the box it took),
     so `pending` has one home. T3 identifies "the guard that took" by what ig's guard already holds (the box), not by
     `is_outermost` (P-T20 *Residual*; that method is not used). T3's PR adds `pending`, the push and the flush in
     `close`; ig adds neither. The windows whose `pending` can be non-empty are those enclosing a fragment parse,
     i.e. DOMParser's (H-T30), whose window calls `close` on its returning path (H-T32); the fragment backends' own
     windows close before the fire (P1), and nothing fires inside `despawn_subtree`'s (whose window restores by its
     drop in the snapshot, so it is not relied on). Open: the shape of T3's change to `close` against ig's one restore
     site (P-T20 *Residual*). (Until rev 4.8.1: "What T3 requires of ig: (a) on every return path of a take window
     the restore is the outermost guard's consuming close, distinct from the unwind `Drop` … (b) the slot state is the
     window's only storage".)
   - Consequence: DOMParser's build is processed at creation (D-T12, D-T13), so it leaves slice b (PM decision 711;
     until rev 4.8: "For `cause = FragmentParse` it does not assert, and the event is dropped as every other event in
     the window is … dropping is that build's intended outcome … the DOMParser build's creation processing stays slice
     b's", rested on the rustdoc claim P-T19 corrects). *Not chosen (rev 4.7):* firing inside each backend after its
     own restore, only when its take returned `Some` (two fire sites).
5. **Take / restore sites relied on (premise, re-pulled at ig's head; PM decisions 713 (1), 714).** At `cee74989`:
   - (a) `EcsDom::begin_detached_fragment` takes (`crates/core/elidex-ecs/src/dom/tree/teardown.rs:308`); the tolerant
     backend restores at `crates/dom/elidex-html-parser/src/lib.rs:156-158`, the strict backend at
     `crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs:238-241` (both result arms), each before its function
     returns to `parse_fragment_progressive`;
   - (b) `parse_into_inert_document` takes at `crates/dom/elidex-form/src/inert_document.rs:112` and restores at
     `:202-203`, enclosing its `apply_set_inner_html` call (`:142`);
   - (c) `despawn_subtree` (`teardown.rs:225`, restore `:256`): not relied on (nothing fires inside it; the strict
     reject path calls it inside (a)'s window).

   These anchors change shape in L3's PR (is) and PR (ig): the strict backend's ownership swap disappears and its
   restore at `tree_builder/mod.rs:238-241` becomes the guard's close (L3, PM decision 713 (1); in the PR (ig) plan
   §4.1 *The four windows* the strict window opens its guard on the caller's dom in `build_fragment` at PR (is)'s
   head, the builder borrowing through it, and calls `window.close()` after the teardown), and every take becomes a
   `DispatcherTake` (H-T32). **T3 re-pulls (a)–(c) at ig's head before implementation (§9 item 1); the properties
   below are unchanged by is / ig** (L3's review, PM decision 713 (1)). Properties relied on: **P1** when the backend
   call inside `parse_fragment_progressive` returns, the build's own window is closed (the slot is what it was at the
   build's start); **P2** a window that encloses a fragment parse is still open at the fire point (the slot reads
   `Taken { .. }`); **P3** a take inside an open window opens no window and is followed by no restore (its guard holds
   nothing). With item 4's (a) and (b) they make the fire at item 3's site either a dispatch (P1, no enclosing window)
   or a push onto the enclosing window's `pending` (P2, P3), flushed exactly once, at that window's close.
6. **Walk set and migration, per consumer** (H-T31's census; at T3's head `git grep -n -e CloneCreated -e
   fire_clone_created -- crates` → 0):
   - `EventHandlerAttributeConsumer` (T3; new arm, `session/event_handler_consumer.rs`): `SubtreeCreated { root,
     cause }` → `process_created_subtree(dom, root, cause, policy, active)` at `ProcessingTiming::Creation`.
     `Clone(scope)` → D-T10's walk (`for_each_clone_created(root, scope, …)`, then the first-level template contents
     through `for_each_uncapped_shadow_inclusive(…, WalkScope::WithTemplateContents, …)`, the base's `Include`).
     `FragmentParse` → **every node the parse created** (PM decision 711 (3)): `root` and every node
     `EcsDom::for_each_uncapped_shadow_inclusive(root, WalkScope::WithTemplateContents, …)` visits (io: the `pub`
     walker taking a `WalkScope`; `OwnedEntity::in_scope`'s exhaustive match puts `ShadowRoot` and `TemplateContents`
     in that scope and the inert template document out of it, H-T32): the light tree, declarative shadow content and
     template contents at any depth, uncapped. Every visited element is processed and marked. `LifetimeUnit` is not
     used (D-T12). `handle_insert` keeps its current set (root + `traverse_descendants`, capped); the two no longer
     share a collector (PM decision 713 (4); until rev 4.8: "the insertion-time set … one collector shared with
     `handle_insert` … not a copy"). The visit order is the walker's (H-T25); it is observable only where two
     created elements write one Window handler (two `body` / `frameset` elements of one parse in the active
     document), and no cell pins it.
   - `CustomElementReactionConsumer` (`crates/dom/elidex-custom-elements/src/consumer.rs:116`): `Clone(scope)` →
     `handle_clone_created(root, scope, dom)`, unchanged; `FragmentParse` → no-op, behaviour unchanged (PM decision
     711 (4): T3 does not change custom-element behaviour). The spec's upgrade enqueue at a fragment-parsed element's
     creation is the defect of the L3-owned slot of §8.1 item 5, closed by a later L3 PR that adds the `FragmentParse`
     arm (O-12, closed).
   - `FormControlReconciler` (`crates/dom/elidex-form/src/reconciler.rs:53`): `Clone(scope)` → `apply_clone_form_state`,
     unchanged; `FragmentParse` → no-op (the *cloning steps* are a clone's only; DOMParser's build re-runs it itself).
   - `LiveRangeBridge` (`crates/dom/elidex-dom-api/src/range/live.rs:160`, in its list of ignored variants): the listed
     name changes; still ignored.
   - `node_iter`, `base_url`, `canvas` (`consumer_dispatcher.rs:157-163`): their catch-all arm; unchanged.
   - The dom-api test probe `CloneCreatedProbe` (`test_util.rs:43-53`) is renamed `SubtreeCreatedProbe` and records the
     `Clone` cause (the grep in this item's heading would count its old name); docs and tests that
     name the old variant or fire function (`ecs/dom/mod.rs`, `walkers.rs`, `tree_clone.rs`, `upgrade.rs`, form
     `clone.rs`, `consumer_dispatcher.rs`, `node_methods_extras.rs`, and the test files of §5.3) are renamed in place.

   No consumer reads what another writes on this event (only the event-handler arm acts on `FragmentParse`; for
   `Clone` the RC-2a plan §2 C × D), and no script runs between the dispatch and the placement, or, inside
   DOMParser's window, between the fire and the flush (the setter call and `parseFromString` are synchronous;
   custom-element reactions are only enqueued), so the consumers' field order is unobservable.

## §5. Populations, touched files, named risks

### §5.1 Behavioural population (re-runnable)

The pairs whose target changes are exactly those `is_window_delegating_pair` accepts and HEAD's rule did not, plus the
non-active ones: `EVENT_HANDLER_ATTRS` rows with `HandlerScope::Window` (18; the slice-a ratchet test
`tests_insertion_and_write.rs:320-330` equates them with `webref idl html WindowEventHandlers`) and the six of
`WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET` (ratchet `:333-344`), on `body` and `frameset`. EW = 6 + 24 = 30
pairs; XW = 18. The applicability population (D-T8) is every `EVENT_HANDLER_ATTRS` row × element class {HTML `body` /
`frameset`, other HTML, SVG, MathML, other namespace}, by `HandlerScope`. Unit cells U-T1, U-T6 and U-T8 derive their
tables from those lists (not a sample). The clone path's population is every element of a clone (D-T10); its
production entry population is one (`cloneNode`, H-T24, H-T26, by their `git grep` commands). The fragment path's
population is every element each parse created: each returned root's `WalkScope::WithTemplateContents` walk, light,
declarative shadow and template contents, uncapped (D-T12; until rev 4.8: "each returned root's insertion-time walk
set"); its production entry population is one function, `parse_fragment_progressive`, with six calls (H-T29),
DOMParser's among them (queued and flushed at its window's close, §4.5 item 4; until rev 4.8: "dropped").

### §5.2 Existing tests whose expectation changes (red at T3 by design, rewritten in the same commit)

Computed from `tests_event_handler_attrs_window_delegating.rs` and `tests_event_handler_attrs_once.rs` at `cee74989`
(re-runnable: `git show cee74989:crates/script/elidex-js/src/vm/tests/<file>`; rev 4.4 read them at `4fd85f4e`: the
first is unchanged since, the second differs only in C14, inverted by C1), with the base's outcome derived from C0
and C1 (re-run at the base, §9 item 1). "Static; confirmed red-first at implementation" marks an expectation that rests
on H-T27 or H-T28 (static evidence).

| Test | Base expectation | T3 expectation | Why |
|---|---|---|---|
| C17 `c17_domparser_body_weh_idl_write_then_append` (`:39`) | `true,true,true,true,true,true` | `true,false,true,true,true,true` | XW: the setter returns (null target, XW-4); the append still re-raws (XW-6, X-W until S-5). Stays a window pin for its last four values |
| C18 (`:64`) | after the setter `window.onhashchange === fn`; after the append a function `!== fn`; `window.hit === 1` | after the setter `window.onhashchange === null`; after the append a function `!== fn`; `window.hit === 1` | `b` is adopted into I (H-T21), so `setAttribute` and the setter return (XW-8); the append re-raws (XW-6). Its first step becomes spec; it stays a window pin for the rest. Under PM decision 153's order C1 alone keeps C18 (the base's target rule ignores the document); T3 rewrites its first step |
| C19b `c19b_body_reflecting_pair_reraw_at_insertion` (`:122`) | `true,true,true,true` | `true,true,false,true` | EW: `setAttribute('onfocus')` on a detached bound `b` writes the Window (EW-2); no re-raw at either append (EW-9, EW-7); the setter returns while `b` is in `dp` (EW-5); `b.onfocus` reads the Window's function; a non-bubbling `focus` at `b` runs nothing (H-T27; static; confirmed red-first at implementation). Equals the spec → becomes a guard (renamed: no re-raw) |
| C19c `c19c_frameset_reflecting_pair_reraw_at_insertion` (`:132`) | same as C19b | same as C19b | facets 3 + 4, same steps; spec-equal → guard (H-T27; static; confirmed red-first at implementation) |
| P3 `p3_frameset_clone_classified_by_tag_and_name` (`:172`) | `true,true` | `true,false` | `onhashchange` on a `frameset` targets the Window (EW-2); the clone's copy is delivered to the Window at creation (EW-12) and is not re-raw'd at the append; `hashchange` at `f` runs nothing (H-T27; static; confirmed red-first at implementation). Spec-equal |
| C9 `c9_clone_is_unprocessed_origin` (`tests_event_handler_attrs_once.rs:216`) | before insert: `c` unmarked; after: `c.onclick` is compiled A | before insert: `c` **marked**, `c.onclick` is compiled A; after: the same | the clone is processed at creation (OR-2, D-T10); the clone-policy non-copy (U6) still holds, the marker comes from the processing. Renamed (processed at creation) |
| C5 (iii), C12; C8, C19, C20 (i), C20 (ii); session U2, U3 (ii), U6, P1 (i), P1 (ii) | — | unchanged | C5 (iii): the clone in `dp` is processed and marked at creation, `c.onclick = fn` wins, the append does not re-raw: `true,true,true` as before. C12: the removal clears what creation wrote. XW pairs (C8, C19, U2, U3 (ii), P1), the T-a3 rule (C20), the ECS clone policy (U6) |

Kept green and checked: `tests_event_handler_attrs.rs` F-12 (`:235`), F-12b (`:446`), F-14 (`:263`); N1's C8h (outcome
unchanged, mechanism now step 3); `tests_call_listener.rs:97`; the shell E1–E8 (`content_eh_attrs_bind_tests.rs`); C1's
cells (the C1 memo §6). Any existing test that installs a handler through a non-applicable attribute (a WEH name on a
non-`body` HTML element, a Document-only name on an element, any name on a null-namespace element) changes with D-T8:
re-run at the base and listed in the PR (§9 item 1). The session tests at `tests_insertion_and_write.rs:60`, `:101`,
`:237` read `dom.window_entity()` and switch to the harness's Window (`test_support::install` / `init_fresh` take
`ActiveDocument`).

### §5.3 Touched files

Sizes and paths re-verified 2026-11-13 against `cee74989` (`git show cee74989:<path> | wc -l` on each file; rev 4.4:
2026-11-04, `4fd85f4e`).

- `crates/script/elidex-script-session/src/event_handler_processing/mod.rs` — D-T1/D-T2/D-T3/D-T5/D-T7/D-T8/D-T10;
  deletions; harness (`test_support::install` / `init_fresh` take `ActiveDocument`). 410 lines + T3's (< 1000; if it
  nears 1000 at the base, the clone-path walk is the seam to split out first, as a standalone prereq).
- `crates/script/elidex-script-session/src/event_handler_consumer.rs` — constructor, both arms, the new `SubtreeCreated`
  arm (D-T10, D-T12, D-T13); `handle_insert` unchanged (its set is not shared with the fragment walk since rev 4.8)
  (545 lines).
- `crates/script/elidex-script-session/src/lib.rs` — exports.
- `crates/core/elidex-ecs/src/dom/mod.rs` — delete `EcsDom::window_entity` (a deletion; 959 lines at `cee74989`, with
  C0b and C1; 926 at `4fd85f4e`. Under 1000, so no prereq split; if a later landing takes it over 1000 at the base,
  the touch-time split check runs first).
- `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` — accessors, `require_body_receiver` (683 lines).
- `crates/script/elidex-js/src/vm/host/elements.rs` — the `body` arm (455 lines).
- `crates/script/elidex-js/src/vm/consumer_dispatcher.rs`, `crates/script/elidex-js/src/vm/vm_api.rs` — marshal
  `ActiveDocument` at bind through `HostData::active_document()` (PM 455); the `event_handler_attrs` field doc names the
  `CloneCreated` arm.
- `crates/script/elidex-js/src/vm/host_data/engine_feature/bind.rs` — `HostData::active_document()` (NEW, beside
  `document_entity_opt` `:474`; D-T3, PM 441), unless the event-path PR-B landed it first (573 lines).
- tests: `session/event_handler_processing/tests_target.rs` (NEW), `tests_bind.rs`, `tests_insertion_and_write.rs`
  (harness); `elidex-js/vm/tests/tests_event_handler_target.rs` (NEW), `tests_event_handler_attrs_window_delegating.rs`
  and `tests_event_handler_attrs_once.rs` (§5.2); `crates/shell/elidex-shell/src/content_eh_attrs_bind_tests.rs` (cell
  T-E1; 390 lines).
- the creation event (§4.5; PM decision 705), sizes at `cee74989`: `crates/core/elidex-ecs/src/dom/mutation_event.rs`
  (441; the variant, `CreationCause`), `crates/core/elidex-ecs/src/dom/mod.rs` (959; `fire_subtree_created` replaces
  `fire_clone_created` in place, net a few lines; with T3's deletion of `EcsDom::window_entity` it stays under 1000;
  if a later landing takes it over 1000 at the base, the touch-time split check runs first),
  `crates/core/elidex-ecs/src/lib.rs` (51; re-export), docs in `ecs/dom/tree/walkers.rs` (482) and
  `ecs/dom/tree_clone.rs` (415); `crates/dom/elidex-html-parser/src/lib.rs` (259; the fragment fire);
  `crates/dom/elidex-dom-api/src/node_methods/clone.rs` (472; the clone fire), `range/live.rs` (506),
  `test_util.rs` (78); `crates/dom/elidex-custom-elements/src/consumer.rs` (480), `upgrade.rs` (609);
  `crates/dom/elidex-form/src/reconciler.rs` (340), `clone.rs` (399); docs in
  `crates/script/elidex-js/src/vm/host/node_methods_extras.rs` (242). Tests renamed or extended:
  `ecs/dom/tests/dispatch_guard.rs` (194; U-T11), `mutation_hook.rs` (919), `mutation_hook_fire_helpers.rs` (112),
  `traversal.rs` (611); `elidex-custom-elements/src/tests.rs` (731); `elidex-dom-api/src/node_methods/tests/clone_shadow.rs`
  (440); `elidex-js/vm/tests/tests_clone_created.rs` (186), `tests_clone_form_state.rs` (177); a fragment-fire test
  module of `elidex-html-parser` (U-T10; the crate's `tests.rs` or a sibling, chosen by size at implementation).
- the window's queue (§4.5 item 4; PM decisions 711, 713), on ig's head (re-measured there, §9 item 1):
  `crates/core/elidex-ecs/src/dom/mod.rs` (`DispatcherSlot::Taken { pending }`, `take_mutation_dispatcher`,
  `clear_mutation_dispatcher`, `fire_subtree_created`'s `Taken` arm; same file as above);
  `crates/core/elidex-ecs/src/dom/dispatcher_take.rs` (NEW: ig's file; T3 adds the flush in the outermost close, the discard in
  `Drop`); `crates/dom/elidex-form/src/inert_document.rs` (291 at `cee74989`; the rustdoc of `parse_into_inert_document`,
  `:64-90`: the build's creation event is queued and dispatched at the window's close, and the
  `EventHandlerAttributeConsumer` then sets raw handlers, compilation being null without a browsing context (HTML
  §8.1.8.1 *get the current value of the event handler* step 3.2, N1's gate), replacing "HTML §8.1.3.4 does NOT
  process event-handler attrs when scripting is disabled … Leaving both off is the CORRECT inert behavior" (P-T19,
  D-T13); `CustomElementReactionConsumer` still ignores the event (§8.1 item 5)). Tests: U-T11 in ig's guard test
  module or `ecs/dom/tests/dispatch_guard.rs` (whichever holds the window cells at ig's head); T-26 to T-28 in
  `tests_event_handler_target.rs` (above).
- docs: the umbrella (§8.4, in this PR).

Not touched: `crates/script/elidex-script-session/src/mutation/html_fragment.rs` (858; C-i rejected, D-T12; its
`set_inner_html_fires_insert_at_caller_placement` probe matches `Insert` only, `:668-700`, re-checked at
implementation); `crates/script/elidex-script-session/src/scripting.rs` (D-T4: C0b rewrote clause (b));
`crates/core/elidex-ecs/src/dom/classify.rs` and `crates/dom/elidex-dom-api/src/char_data/document_props.rs` (D-T5, PM
decision 133); `crates/script/elidex-js/src/vm/host_data/` other than `engine_feature/bind.rs`'s constructor (D-T11; Q-14).
`crates/dom/elidex-dom-api/src/node_methods/clone.rs` (D-T10's rejected site) and
`crates/script/elidex-js/src/vm/host/node_methods_extras.rs` (rev 4's site, D-T10) get no processing call; since rev
4.7 they are touched by §4.5's rename only (the fire call, docs; until rev 4.7 both were listed as not touched).

### §5.4 Named risks

- **R2 — merge contact with C0, RC-2a, C1-pre and C1** in `event_handler_attrs.rs`, `elements.rs`, `entity_kind.rs`
  (C1-pre's `PrototypeKind::Document`) and the clone code (C1 D-C6 changes `replicate_template_contents`; RC-2a adds the
  `CloneCreated` arms T3's arm sits beside); T3 is written on their landed base (§9 item 1).
- **R3 — `ActiveDocument` staleness.** The consumer's copy is fixed at construction; it is rebuilt at every bind (slice a
  H13), and each production `Vm` binds one `EcsDom`'s one bound document (H-T15). The unbind caveat (H-T15) is the
  relied-on contract; a cross-`EcsDom` rebind would make step 4 return the old World's Window (B1 removes the
  possibility).
- **R4 — `onerror` on `<body>`** now lands on the Window and compiles with the one-argument form, where the spec gives a
  Window `onerror` five parameters: `#11-event-handler-attribute-edge-cases` facet 2 becomes reachable through
  `<body onerror>` (§8.3 item 4).
- **R5 — clone-path cost.** A deep clone of n elements adds one walk of n and one attribute scan per element (the walk
  is RC-2a's, which the form and CE arms already run once each per clone, the RC-2a plan §4.3).
- **R6 — a future clone path that bypasses the canonical cloner entry**: D-T10 *Fallback*.
- **R7 — the creation event and its queue are on L3's surface.** T3's PR changes RC-2a's `CloneCreated` into
  `SubtreeCreated`, gives ig's slot state the pending list and adds the flush to ig's close (§4.5); L3 reviewed rev 4.7's
  §4.5 (PM decision 713) and reviews rev 4.8's through the PM. **Premises (rev 4.8.1):** (1) L3's plans are cited at a
  pre-ratification snapshot (H-T32) by symbol and section; **before T3 is ratified, every L3 citation in this memo
  and in the umbrella's x12 row is re-pinned to L3's ratified sha** (the PR (ig) plan, the PR (io) plan, the L3
  umbrella), and a citation whose symbol or section is gone there is re-derived, not carried (PM 718 already
  announces changes past the snapshot, H-T32: none is relied on). (2) T3 lands after io (umbrella x12), so ig's and
  io's landed code is T3's base: §4.5 item 5's anchors are re-pulled at ig's head and the walker's names at io's (§9
  item 1). (3) P-T20 *Residual* (where the flush sits against ig's one restore site) is decided with L3 before T3's
  implementation starts. **Stop condition (what remains after rev 4.8.1):** T3 stops for the PM if ig is ratified or
  lands **without a consuming `close`** as the restore point on the returning path of a window that can enclose a
  fragment parse (guarantee (a)), or **with a second home of take/restore state** beside the slot (a flag or depth
  counter in a window or guard; guarantee (b)), or if the shape P-T20 settles cannot be built on ig's landed `close`
  and `Drop` (until rev 4.8.1: "If ig's landed guard does not have item 4's (a) (a consuming close on every return
  path, distinct from the unwind `Drop`) or (b) …, item 4 cannot be built as written and T3 stops"; until rev 4.8:
  "If (i) lands first and changes a take / restore site or property of §4.5 item 5, item 4's rule is re-derived on
  (i)'s guard").
- **R8 — fragment-path cost.** One uncapped walk per returned root at creation over every node the parse created
  (light, declarative shadow, template contents), a superset of the set the placement's `Insert` walks; at the
  placement each OR element then returns at the marked shortcut (`mod.rs:85`). DOMParser's build adds the same walk
  at its window's close, which the base does not run at all.

## §6. Test plan

Red-first: each **red** cell is committed failing on T3's base before the fix commit. Every VM cell uses
`with_page` / `run` (`elidex-js/vm/test_helpers.rs:168`, `:193`; `dp` is a DOMParser document with an empty body, `el` a
connected element, `fn` counts its calls in `fnRan`). *Base* values are the base's (§0.1).

### §6.1 VM integration (`tests_event_handler_target.rs`, NEW)

| Cell | Kind | Script | Base | T3 (= spec unless marked) |
|---|---|---|---|---|
| T-1 | red (facet 1, IDL; XW-4, XW-5) | `dd = new DOMParser().parseFromString('<body></body>','text/html'); dd.body.onhashchange = fn; r.push(window.onhashchange === null, dd.body.onhashchange === null)` | `false,false` (`fn` on the Window) | `true,true` |
| T-2 | red (facet 1, attribute change steps; XW-3) | `dd.body.setAttribute('onhashchange','window.hit = 1'); window.dispatchEvent(new Event('hashchange')); r.push(window.onhashchange === null, window.hit === undefined)` | `false,false` | `true,true` |
| T-3 | red (facet 1, XW re-raw into a non-active document; XW-7) | `b = document.createElement('body'); b.setAttribute('onhashchange','window.hit = 1'); window.onhashchange = fn; dp.body.append(b); r.push(window.onhashchange === fn)` | `false` (re-raw) | `true` |
| T-4 | red (facet 4, content attribute; EW-2, EW-4) | `document.body.setAttribute('onload','window.hit = 1'); r.push(typeof window.onload === 'function', document.body.onload === window.onload); window.dispatchEvent(new Event('load')); r.push(window.hit === 1)` | `false,false,false` | `true,true,true` |
| T-5 | red (facet 4, IDL through `HTMLElement.prototype`; EW-3) | `document.body.onfocus = fn; r.push(window.onfocus === fn, document.body.onfocus === fn); window.dispatchEvent(new Event('focus')); r.push(fnRan === 1)` | `false,true,false` | `true,true,true` |
| T-6 | red (facet 3, content attribute; EW-2) | `f = document.createElement('frameset'); f.setAttribute('onhashchange','window.hit = 1'); window.dispatchEvent(new Event('hashchange')); r.push(window.hit === 1)` | `false` | `true` |
| T-6b | guard (facet 3, non-active; EW-5; H-T28: static; confirmed red-first at implementation) | `dd = parseFromString('<frameset></frameset>','text/html'); dd.body.setAttribute('onhashchange','window.hit = 1'); r.push(window.onhashchange === null, dd.body.tagName === 'FRAMESET')` | `true,true` | `true,true` |
| T-7 | red (facets 3 + 4, IDL; EW-3, H-T17) | `f = document.createElement('frameset'); f.onload = fn; r.push(window.onload === fn, f.onload === fn)` | `false,true` | `true,true` |
| T-8a | guard (step 3 note; XW-2) | `b = document.createElement('body'); b.onhashchange = fn; r.push(window.onhashchange === fn)` | `true` | `true` |
| T-8b | red (step 3 note, reflecting; EW-2) | `b = document.createElement('body'); b.setAttribute('onresize','window.hit = 1'); r.push(typeof window.onresize === 'function')` | `false` | `true` |
| T-9 | red (step 1 namespace, WEH; FN-2) | `s = document.createElementNS('http://www.w3.org/2000/svg','body'); s.setAttribute('onhashchange','window.hit = 1'); r.push(window.onhashchange === null); s.dispatchEvent(new Event('hashchange')); r.push(window.hit === 1)` | `false,false` (tag-only: on the Window) | `true,true` |
| T-9b | guard (step 1 namespace, reflecting; FN-1; kills a tag-only class test) | `s = document.createElementNS('http://www.w3.org/2000/svg','body'); s.setAttribute('onfocus','window.hit = 1'); r.push(window.onfocus === null)` | `true` | `true` (tag-only mutant: `false`) |
| T-14 | red + guard (EW-2, EW-7; rev 1's W1) | `document.body.setAttribute('onresize','window.hit = 1'); r.push(typeof window.onresize === 'function'); window.onresize = fn; document.documentElement.append(document.body); r.push(window.onresize === fn)` | `false,true` | `true,true` (re-raw: `true,false`) |
| T-15 | red + guard (EW-5, EW-6, EW-8; rev 1's W2) | `dd = parseFromString('<body></body>','text/html'); b = dd.body; b.setAttribute('onscroll','window.hit = 1'); r.push(window.onscroll === null); document.documentElement.append(b); r.push(window.onscroll === null, b.onscroll === null)` | `true,true,false` (the element's re-raw compiles) | `true,true,true` (re-raw onto the Window: `true,false,false`) |
| T-15b | red + guard (EW-10; kills α) | `dd = parseFromString('<body onscroll="window.hit = 1"></body>','text/html'); b = dd.body; document.documentElement.append(b); r.push(window.onscroll === null, b.onscroll === null)` | `true,false` | `true,true` (α: `false,false`) |
| T-16 | **red (R-C; EW-12)** | `b = document.createElement('body'); b.setAttribute('onfocus','window.hit = 1'); window.onfocus = fn; c = b.cloneNode(); r.push(window.onfocus === fn, c.onfocus === window.onfocus); el.append(c); r.push(window.onfocus === fn)` | `true,false,true` | `false,true,false` |
| T-16b | red (Q-15; the walk's order, D-T10) | `h = document.createElement('div'); h.attachShadow({mode: 'open', clonable: true}).append(document.createElement('body')); h.append(document.createElement('body')); h.shadowRoot.firstChild.setAttribute('onfocus','window.w = "shadow"'); h.firstChild.setAttribute('onfocus','window.w = "light"'); window.onfocus = fn; h.cloneNode(true); window.onfocus(); r.push(window.w)` | `undefined` (`fn` ran) | `"shadow"` (shadow-before-light walk: `"light"`) |
| T-16c | red (CI-1; D-T10 *Canonical writer*: both writers run) | `p = document.createElement('div'); b = document.createElement('body'); b.setAttribute('onfocus','window.hit = 1'); s = document.createElement('span'); s.setAttribute('onclick','window.n = (window.n \|\| 0) + 1'); p.append(b, s); window.onfocus = fn; c = p.cloneNode(true); r.push(window.onfocus !== fn && typeof window.onfocus === 'function'); c.firstChild.dispatchEvent(new Event('focus')); r.push(window.hit === undefined); c.lastChild.dispatchEvent(new Event('click')); r.push(window.n === 1)` | `false,false,true` (EW on the element at the internal Insertion) | `true,true,true` (Creation appends a second entry instead of reusing: `false,true,false`, §6.5) |
| T-16d | guard (CI-1, XW; D-T10 *Canonical writer*: both writers write the Window, PM 377) | `p = document.createElement('div'); q = document.createElement('div'); b = document.createElement('body'); b.setAttribute('onhashchange','window.n = (window.n \|\| 0) + 1'); q.append(b); p.append(q); window.onhashchange = fn; c = p.cloneNode(true); r.push(window.onhashchange !== fn && typeof window.onhashchange === 'function'); window.dispatchEvent(new Event('hashchange')); r.push(window.n === 1)` | `true,true` (the internal Insertion writes the Window) | `true,true` (Creation appends a second entry instead of reusing: `true,false`, §6.5) |
| T-17 | **red (R-A; EW-13, EW-14)** | `b = document.createElement('body'); tpl = document.createElement('template'); tpl.content.append(b); b.setAttribute('onfocus','window.hit = 1'); r.push(window.onfocus === null); document.body.append(b); r.push(b.onfocus === null, window.onfocus === null)` | `true,false,true` (raw on `b` in I reads null by N1; the move out re-raws onto `b`, which compiles) | `true,true,true` |
| T-18 | guard (R-B; XW-9) | `p = document.createElement('div'); dd = parseFromString('<body></body>','text/html'); b = dd.body; p.append(b); b.setAttribute('onhashchange','window.hit = 1'); r.push(typeof window.onhashchange === 'function')` | `true` | `true`; kills a derivation that ignores C0's adoption |
| T-19 | **red (R-D1; FN-3; D-T11)** | `s = document.createElementNS('http://www.w3.org/2000/svg','body'); s.onhashchange = fn; s.dispatchEvent(new Event('hashchange')); r.push(fnRan === 0, window.onhashchange === null, s instanceof HTMLBodyElement); m = document.createElementNS('http://www.w3.org/1998/Math/MathML','body'); m.onhashchange = fn; r.push(window.onhashchange === null); try { Object.getOwnPropertyDescriptor(HTMLBodyElement.prototype,'onhashchange').get.call(s); r.push(false) } catch (e) { r.push(e instanceof TypeError) }` | `true,false,true,false,false` | `true,true,false,true,true` |
| T-20 | red (OR-2; umbrella G5 / K3) | `t = document.createElement('div'); t.setAttribute('onclick','globalThis.a = 1'); c = t.cloneNode(); r.push(typeof c.onclick === 'function'); c.dispatchEvent(new Event('click')); r.push(globalThis.a === 1)` | `false,false` | `true,true` |
| T-20b | red (XW-11) | `b = document.createElement('body'); b.setAttribute('onhashchange','window.hit = 1'); window.onhashchange = fn; c = b.cloneNode(); r.push(window.onhashchange === fn)` | `true` | `false` (the clone's attribute is written to the Window at creation) |
| T-21 | **red (R-D2; FN-4; D-T8)** | `m = document.createElementNS('http://www.w3.org/1998/Math/MathML','body'); m.setAttribute('onhashchange','window.hit = 1'); m.dispatchEvent(new Event('hashchange')); r.push(window.hit === undefined, window.onhashchange === null)` | `true,false` | `true,true` (without D-T8: `false,true`) |
| T-22 | **red (R-E; EW-20, XW-15; PM decision 699)** — fragment creation, both name classes | `window.onfocus = fn; window.onhashchange = fn; document.documentElement.innerHTML = '<body onfocus="window.a = 1" onhashchange="window.b = 1">'; r.push(typeof window.onfocus === 'function' && window.onfocus !== fn, typeof window.onhashchange === 'function' && window.onhashchange !== fn, document.body.onfocus === window.onfocus)` | `false,true,false` (EW raw on the element at the placement's insertion; XW re-raw onto the Window there) | `true,true,true` (fragment path without Creation, rev 4.5's premise: `false,true,true`) |
| T-22b | **red (R-E; EW-20; PM decision 699)** — fragment creation, `frameset`, both name classes | `window.onfocus = fn; document.documentElement.innerHTML = '<frameset onfocus="window.a = 1" onhashchange="window.b = 1"></frameset>'; r.push(typeof window.onfocus === 'function' && window.onfocus !== fn, typeof window.onhashchange === 'function')` | `false,false` (both on the element) | `true,true` (fragment path without Creation: `false,false`) |
| T-22c | red (EW-10, EW-21, XW-7, XW-12; step 3 on the fragment path, both in DOMParser's window and outside it; rewritten in rev 4.8, PM decision 711) | `window.onfocus = fn; dd = new DOMParser().parseFromString('<body onfocus="window.a = 1" onhashchange="window.b = 1"></body>','text/html'); r.push(window.onfocus === fn, window.onhashchange === null); dd.documentElement.innerHTML = '<body onfocus="window.a = 1" onhashchange="window.b = 1">'; r.push(window.onfocus === fn, window.onhashchange === null)` | `true,true,true,false` (the build processes nothing; XW re-raw onto the page Window at the insertion into P, XW-7) | `true,true,true,true` (Creation targeting the bound document's Window regardless of the node document: `false,false,false,false`) |
| T-23 | red (step 3 note: created by `createElement`, never connected; EW-2, XW-2) | `b = document.createElement('body'); b.setAttribute('onfocus','1'); b.setAttribute('onhashchange','1'); f = document.createElement('frameset'); f.setAttribute('onresize','1'); f.setAttribute('onpopstate','1'); r.push(typeof window.onfocus === 'function', typeof window.onhashchange === 'function', typeof window.onresize === 'function', typeof window.onpopstate === 'function')` | `false,true,false,false` (EW on the elements; XW on the Window) | `true,true,true,true` (step 3 asking whether the element is connected: `false,false,false,false`) |
| T-24 | red (step 3 note: a disconnected `body`; EW-2, XW-2) | `b = document.body; b.remove(); b.setAttribute('onfocus','window.a = 1'); b.setAttribute('onhashchange','window.b = 1'); r.push(typeof window.onfocus === 'function', typeof window.onhashchange === 'function', b.onfocus === window.onfocus)` | `false,true,false` | `true,true,true` (step 3 asking whether the element is connected: `false,false,true`) |
| T-25 | red (OR-3; D-T12: placement under a shadow root; static; confirmed red-first at implementation) | `h = document.createElement('div'); el.append(h); s = h.attachShadow({mode: 'open'}); s.innerHTML = '<p onclick="window.c = 1"></p>'; s.firstChild.dispatchEvent(new Event('click')); r.push(window.c === 1)` | `false` (no `Insert` under a shadow-root parent, so the attribute is never processed) | `true` (processed at creation) |
| T-26 | **red (OR-4; D-T13; PM decisions 711, 713: DOMParser's build processed at its window's close)** | `dd = new DOMParser().parseFromString('<p onclick="window.c = 1" onfocus="window.d = 1"></p>','text/html'); p = dd.querySelector('p'); r.push(p.onclick === null); p.dispatchEvent(new Event('click')); r.push(window.c === undefined); p.onclick = fn; document.body.append(p); p.dispatchEvent(new Event('click')); r.push(fnRan === 1, window.c === undefined)` | `true,true,false,false` (nothing at creation; the IDL write does not process an element with two handler attributes, D-8; the first insertion processes the unmarked `p` and its raw value replaces `fn`) | `true,true,true,true` (raw at creation and marked; null in P by N1's compile gate; the insertion processes nothing). Mutants: the event dropped in the window (rev 4.7), or flushed before the restore: `true,true,false,false` |
| T-27 | **red (OR-5; D-T12: declarative shadow content of a fragment parse; static; confirmed red-first at implementation)** | `h = document.createElement('div'); el.append(h); h.setHTMLUnsafe('<div><template shadowrootmode="open"><p onclick="window.c = 1"></p></template></div>'); h.firstChild.shadowRoot.firstChild.dispatchEvent(new Event('click')); r.push(window.c === 1)` | `false` (no placement `Insert` enters a shadow tree; never processed) | `true` (processed at creation). Mutant: the insertion-time walk set (rev 4.7): `false` |
| T-28 | **red (OR-6; D-T12, D-T13: template contents of a fragment parse)** | `el.innerHTML = '<template><p onclick="window.c = 1" onfocus="window.d = 1"></p></template>'; p = el.firstChild.content.firstChild; r.push(p.onclick === null); p.onclick = fn; document.body.append(p); p.dispatchEvent(new Event('click')); r.push(fnRan === 1, window.c === undefined)` | `true,false,false` (contents unprocessed at the placement; the first insertion into A re-raws over `fn`) | `true,true,true` (raw at creation in I, null there by N1's compile gate; marked, so the insertion processes nothing). Mutant: the insertion-time walk set (rev 4.7): `true,false,false` |


**T-22c under §4.5 (rev 4.8; PM decisions 711, 713).** `parseFromString` runs inside DOMParser's window: its
fragment build's event is queued and dispatched at the window's close (§4.5 item 4), the arm processes the `body` at
Creation, and step 3 returns null for both names (its node document is `dd`, not active), so nothing is delivered;
the base processes nothing there, with the same outcome. The later `dd.documentElement.innerHTML = …` runs outside
any window: the event is dispatched at once, Creation again finds step 3 null, and the placement's Insertion re-raws
XW through D-T1, null again, where the base re-raws onto the page Window (XW-7). EW-21's I half (an `html` context
inside template contents) takes the same path with I; U-T9 (iv) covers it. (Until rev 4.8 the cell parsed `<p></p>`
and the note said the build's event "is dropped … and the parsed document gets no creation processing (slice b)".)

### §6.2 Unit (session, `tests_target.rs`, NEW)

| Cell | Kind | Shape | Oracle |
|---|---|---|---|
| U-T1 | red (full enum) | for every `EVENT_HANDLER_ATTRS` name and every reflecting name × tag {HTML `body`, HTML `frameset`, `div`, SVG `body`} × node-document class {the active document (connected; detached owned), a non-active Document (a `create_document_node()` document: connected; detached owned), the inert document I (a node in a template's contents, C1), unresolvable (test-only creator, H-T16)}: `determine_event_handler_target` | the spec table: not Window-delegating → `Some(target)`; Window-delegating and active (or unresolvable) → `Some(window)`; else `None`. The base's rule fails every `frameset`, reflecting, foreign-namespace and non-active WEH row |
| U-T2 | red (bind pass, facet 4; EW-1) | pre-bind page with `<body onload=A>`; `init_fresh` | the Window's `load` entry is raw `A`; the body has no `load` entry. Base: the reverse |
| U-T3 | red (C8h's store; EW-5) | foreign document's `body`, `setAttribute('onfocus', 'A')` through the consumer | no `focus` entry on the body and none on the Window. Base: raw `A` on the body |
| U-T4 | guard (D-5 keying) | U12's shape: bound document `d2 = create_document_node()` ≠ `document_root()`; a `body` in `d2` with `onhashchange=A`; `active = {d2, w}` | delivered to `w`; a `document_root()`-keyed step 3 returns `None` |
| U-T6 | red (D-T7 delivery matrix) | for class ∈ {XW, EW, OR, FN (SVG `body` × a WEH name)} (derived from the two lists and a `div` name) × timing ∈ {Bind, Creation, first Insertion of an unmarked element, later Insertion of a marked element, HandlerWrite, Creation of an element marked by a first Insertion}: an element carrying one such attribute plus one ordinary attribute, its target's entry taken (`take_uncompiled`) after the first delivery | XW: delivered at Bind, Creation and every Insertion, never at HandlerWrite. EW: delivered at Bind and Creation only. OR and FN: Creation, or once. Creation on an already-marked element still delivers: the entry is raw again (§6.5). Base: EW and FN delivered at every Insertion |
| U-T7 | red (D-T10 coverage) | built without a dispatcher (as umbrella U7 / U12), so no `Insert` marks it: `process_created_subtree` with `CloneScope::Subtree` over a clone root whose subtree has (i) a parent wider than `MAX_ANCESTOR_DEPTH` (the U11 precedent; independent of depth; the deeper-than-cap chain is RC-2a's deep-tree cell, PM 367), (ii) a clonable shadow root with `<button onclick=A>`, (iii) a `<template>` whose contents hold `<div onclick=A>` and a `<body onfocus=B>`, built with each owner (active, a non-active Document, I); then (ii)'s host with a light child, with `CloneScope::SelfOnly` | every child of (i)'s wide parent and every element of (ii)–(iii) is marked; under `SelfOnly` the shadow tree is marked and the light child is not; ordinary entries raw on their elements; the contents' `<body>` delivers nothing (I, EW-19); a non-active owner's `<body>` delivers nothing (EW-11, XW-14). The production shape (clone-internal `Insert`s, CI-1) is the VM cells': root T-16, T-20, T-20b; shadow-root children T-16b; both writers T-16c, T-16d |
| U-T8 | red (D-T8; FN-1b, FN-4, FN-5, OR-1) | `is_event_handler_content_attribute_of` over every `EVENT_HANDLER_ATTRS` name × element {HTML `body`, HTML `frameset`, HTML `div`, SVG `body`, SVG `rect`, MathML `body`, MathML `mi`, null-namespace `body`}, through the AttributeChange arm and at Creation | HTML: GlobalEventHandlers names everywhere, WindowEventHandlers names on `body` / `frameset` only, Document-only names nowhere; SVG: every name; MathML: GlobalEventHandlers names only; null namespace: none. The base: every name installs |
| U-T9 | red (D-T12 coverage; OR-3, OR-5, OR-6, EW-20, EW-21, XW-15; rewritten in rev 4.8, PM decision 711 (3)) | built without a dispatcher, so no `Insert` marks it: a fragment from `parse_fragment_progressive` whose returned root holds (i) a parent wider than `MAX_ANCESTOR_DEPTH`, (ii) a `<template>` whose contents hold `<div onclick=A>` and a nested `<template>` whose contents hold `<span onclick=A>`, (iii) (`allow_declarative_shadow`) a nested `<template shadowrootmode=open>` host whose shadow tree holds `<button onclick=A>` and a `<template>` with `<i onclick=A>` in its contents, then `process_created_subtree` with `CreationCause::FragmentParse` on each root; (iv) `<body onfocus=B onhashchange=C>` parsed with an `html` context owned by the active document, a non-active Document and I | the marked set equals every element `for_each_uncapped_shadow_inclusive(root, WalkScope::WithTemplateContents, …)` visits from the roots: every child of (i)'s wide parent, (ii)'s contents at both levels, (iii)'s shadow tree and the contents inside it; ordinary entries raw on their elements (I for contents); (iv): `focus` and `hashchange` raw on the Window for the active owner, nothing anywhere for the other two. Base: no `FragmentParse` cause. Mutants: the insertion-time set (rev 4.7), or `ShadowIncludingTree`: some of those unmarked |
| U-T10 | red (§4.5 items 3–4; `elidex-html-parser`) | a probe dispatcher recording every event; `parse_fragment_progressive` with two top-level elements, on input the strict backend accepts and on input it rejects (tolerant fallback); then the same inside an outer `DispatcherTake` (ig) window, closed after the call | exactly two `SubtreeCreated { cause: FragmentParse }`, roots in the returned order, no other event before them, none naming a despawned entity; inside the outer window none reaches the probe during the call, and the same two, in the same order, reach it at the outer close, after the restore. Base: no event |
| U-T11 | red + guard (§4.5 item 4; ig's guard test module or `ecs/dom/tests/dispatch_guard.rs`, §5.3; rewritten in rev 4.8, PM decisions 711, 713) | a probe dispatcher installed; an outer `DispatcherTake`: (a) `fire_subtree_created(r1, FragmentParse)`, `fire_subtree_created(r2, FragmentParse)`, `fire_subtree_created(r3, FragmentParse)`, `despawn_subtree(r2)` inside the window, then a nested `DispatcherTake` opened and closed, then the outer close; (b) as (a) up to the nested window, then a panic inside the outer window, caught outside it (`catch_unwind`); (c) one push, then `clear_mutation_dispatcher` inside the window, then the close; (d) `fire_subtree_created(root, Clone(CloneScope::Subtree))` while `Taken` | (a) nothing reaches the probe before the outer close (the nested close flushes nothing); at the close the slot is `Installed` before the first dispatch (the probe reads it), and exactly `r1`, `r3` arrive, in that order; (b) the slot holds the dispatcher and the probe received nothing; (c) nothing is received; (d) debug-asserts (D-4, the existing test renamed). Base: (a)–(c) receive nothing (the event is not queued) |

### §6.3 End to end (shell)

| Cell | Kind | Markup | Base | T3 |
|---|---|---|---|---|
| T-E1 | red (H-T14, EW-1) | `build_main_document("<body onload=\"document.body.dataset.l=1\">…")`, then `probe_body_attr(&pipeline, "data-l")` (the E4 pattern, `content_eh_attrs_bind_tests.rs:165-181`) | `None` | `Some("1")` (H-T20) |

### §6.4 Mutation guards

| Mutant | Killed by |
|---|---|
| step 3 dropped | T-1, T-2, T-3, T-15, T-17, U-T1, U-T3 |
| step 3 keyed on `document_root()` | U-T4 |
| step 3 reading a derivation that ignores the inert document or C0's adoption | T-17, T-18, U-T1 (I rows) |
| reflecting set dropped from step 2 | T-4, T-5, T-8b, T-14, U-T1, U-T2 |
| `frameset` dropped from step 1 | T-6, T-7, P3, U-T1 |
| class test without namespace | T-9, T-9b, U-T1 |
| an IDL accessor bypassing D-T1 | T-1 (body pair), T-5 / T-7 (normal pair) |
| unresolvable → `None` | U-T1 (unresolvable row) |
| X-W not narrowed (EW re-raw'd at insertion, rev 1's form) | T-14, T-15, T-17, U-T6 |
| Creation appends a second entry instead of reusing (`set_inline_handler`'s `find_event_handler`, `mod.rs:266-274`) | T-16c, T-16d |
| §4.2's `timing != Creation` dropped (the marked shortcut applies at Creation) | U-T6 (Creation of a marked element, OR) |
| EW under the ordinary once rule (α) | T-15b, U-T6 (first-insertion row) |
| X-W narrowed too far (XW not re-raw'd) | C8, U2, U-T6 |
| clone path absent, or skipping a class | T-16, T-20, T-20b, U-T6 (Creation row), U-T7 |
| fragment path without Creation for EW (rev 4.5's premise), or Creation reading the parser's throwaway document | T-22, T-22b |
| fragment path's Creation not asking step 3 (the bound document's Window for every fragment) | T-22c |
| step 3 asking whether the element is connected (not its node document's activeness) | T-8a, T-8b, T-23, T-24 |
| fragment path delivering EW once at the first insertion instead of at creation (O-10 (b), rejected) | T-15b for DOMParser's build (its first insertion that reaches a consumer is the move into A, where the mutant delivers to the page Window); for the other entry points none: each is placed by its caller in the same call with no script between (H-T29; `createContextualFragment` is a stub), so the outcome is equal at T3's base; a `createContextualFragment` implementation (`#11-range-full-impl`) would add a killer (until rev 4.8: "DOMParser's build drops the event, H-T30") |
| fragment event fired inside the build (before the restore), so no consumer sees it | T-22, T-22b, U-T10 |
| fragment event for the first returned root only | U-T10 |
| the `FragmentParse` cause asserting while `Taken` (D-4 made uniform) | U-T11 (a) (debug panic at the first push); T-15b, T-22c and T-26 (debug panic inside `parseFromString`) |
| the `FragmentParse` event dropped while `Taken` (rev 4.7's shape) | T-26, U-T10, U-T11 (a) |
| the flush before the restore (dispatched into the `Taken` slot, so lost), or at a nested close | T-26, U-T11 (a) |
| the queue flushed on unwind, or not discarded at `clear_mutation_dispatcher` | U-T11 (b), (c) |
| a dead queued root dispatched (no `contains` check) | U-T11 (a) |
| fragment walk = the insertion-time set (rev 4.7: no shadow trees, no template contents, capped), or `ShadowIncludingTree` | U-T9, T-27, T-28 |
| fragment walk on `LifetimeUnit` | none: an equivalent mutant from a fragment root (only a Document carries the inert-document link, H-T32); excluded by D-T12's wording, not by a cell |
| fragment Creation skipping a class, or not marking OR | U-T9, T-25, T-26, T-28 |
| clone path walking light tree only | T-16b, U-T7 |
| clone path excluding template contents, or ignoring the scope | U-T7 only (with a dispatcher both are outcome-equivalent: CI-1 processes those elements) |
| clone walk in shadow-including tree order (shadow tree before light children) | T-16b |
| clone path not marking (re-processed at insertion) | C9, C5 (iii), U-T7 |
| `body` arm or `require_body_receiver` tag-only | T-19 |
| applicability test dropped (every name installs) | T-21, U-T8 |
| MathML treated as SVG (every name), or WEH names accepted on every HTML element | T-21, U-T8 (MathML `body`, HTML `div` rows) |
| applicability test skipped at Creation or in the AttributeChange arm | U-T8 (both paths) |

### §6.5 Run-through of the PM 377 cells (PM 380)

A written trace: the session code at `f9cf8c40` (unchanged by RC-2a), the cloner at `rc2a-clone-created`, plus §4.2
(rev 4.5: the cited lines re-measured at `cee74989`, where the session lines are unchanged and the cloner is RC-2a's
as landed). Shared steps: each copied child is appended as a childless shallow copy (`tree_clone.rs:405-409`), so its `Insert`
(`tree/mutation.rs:528`) makes `handle_insert` (`event_handler_consumer.rs:441-455`) process that element alone. The
IDL setter reuses the entry and clears its source (`event_handler_attrs.rs:628-634`). The getter reads the first
handler entry (`event_listener.rs:194`, `event_handler_attrs.rs:580-596`). Dispatch runs every handler entry, each
compiled on its own (`event_dispatch/mod.rs:317`, `engine.rs:234`).

| Cell ↔ mutant | Intended (T3) | Mutant | Base (`f9cf8c40` + RC-2a) |
|---|---|---|---|
| T-16c ↔ Creation appends a second entry | `true,true,true`: b′'s Insertion skips EW (D-T7), Creation makes the Window's entry raw; b′ has no entry; s′'s Insertion and Creation share one entry (`mod.rs:266-274`), so `n` = 1 | `false,true,false`: the getter reads the first Window entry, which still holds `fn`; s′ has two entries, so `n` = 2 | `false,false,true`: `onfocus` is not Window-scoped (`resolve_handler_target`), so b′ is raw on itself and the Window keeps `fn` |
| T-16d ↔ Creation appends a second entry | `true,true`: b′'s Insertion (XW, active node document) makes the Window's entry raw, and Creation rewrites it with the same source; one entry, `n` = 1 | `true,false`: the Insertion's reused entry compiles; the appended one also runs, so `n` = 2 | `true,true`: the Insertion is the only writer (guard) |
| U-T6 marked-Creation row ↔ `timing != Creation` dropped | OR (`div`): `raw(A)`: the shortcut is skipped and `set_uncompiled` runs (`event_listener.rs:207`) | `NoSource`: `mod.rs:85` returns for a marked `div`; for the `body` classes the shortcut never applies (`is_body_or_frameset`, tag-only), so only the OR row turns red | none: no `Creation` timing |

## §7. Out of scope and slots

| Item | Why not T3 | Home |
|---|---|---|
| X-W for XW (`body` × WEH re-raw at insertion); the D-8 restriction | slice b's S-5 (PM decision 129 moved only EW's part) | slice b; `#11-eh-attrs-creation-time-call-sites` defect (7) (§8.3 item 2) |
| creation-time processing on S-1's other paths (declarative shadow content and template contents created by the pre-bind document parse) | slice b's S-1 (PM decision 153 moved the clone path; PM decisions 699, 705 and 711 the whole fragment path, DOMParser's build, declarative shadow content and template contents it creates included, D-T12; until rev 4.8 those three stayed here) | slice b S-1 |
| the custom-element consumer's `FragmentParse` arm (the upgrade enqueue at a fragment-parsed element's creation) | T3 does not change custom-element behaviour (PM decision 711 (4)) | L3-owned slot `#11-custom-element-upgrade-at-fragment-creation` (proposed id; §8.1 item 5), closed by a later L3 PR after T3 |
| foreign elements' HTML interfaces other than `body`'s | the general fix needs the SVG / MathML interface families (D-T11) | proposed defect text (§8.1 item 4; PM to route; `M4-13:#svg` row as candidate home) |
| **the `HTMLFrameSetElement` interface**: no `HTMLFrameSetElement.prototype`, so its 18 WindowEventHandlers IDL attributes and `cols` / `rows` read `undefined` | an interface surface; compat (PM decision 130, D-T9) | `#11-htmlframesetelement-interface` (§8.3 item 9) |
| **attribute change steps step 1, namespace half** (a non-null attribute namespace) | needs `MutationEvent::AttributeChange` to carry a namespace (H-T19; D-T8) | `#11-event-handler-content-attribute-applicability` facet (2), closed by slice b (§8.3 item 8) |
| `document.body`'s tag-only class test (H-T12) | PM decision 133: T3's steps 3–4 do not use "the body element" | proposed defect text (§8.1 item 3; PM to route) |
| attribute change steps step 5.1 (CSP) | not modelled | `#11-csp-inline-handler-enforcement` |
| multi-realm step 4; the unbind caveat | B1 | hand-off (§8.2) |
| Range element clones (`cloneContents`, `extractContents`' partial element clone) | not implemented (H-T26); through the canonical cloner from RC-2b they fire `CloneCreated` (D-T10) | `#11-range-full-impl` |
| the ASCII-case-insensitive tag compare (an XML-document `createElementNS(html, 'BODY')` counts as `body`) | engine-wide tag-predicate convention (`is_base_element` precedent) | not changed; noted |
| a bubbling event at an element not reaching the Window (H-T27) | not T3's mechanism; no T3 cell depends on it | `#11-event-path-window-missing` |

Own deferrals: **0**. Residuals: **0** (PM decisions 168, 699, 705, 711). The custom-element row is L3's surface,
recorded as a slot by PM decision 711 (4), not a T3 deferral.

## §8. Edges, hand-offs, records

### §8.1 Slot texts (for the PM; defects only)

1. (moved to §8.3 item 9: registered at landing, PM 377; number kept.)
2. (moved to §8.3 item 8: registered at landing, PM 377; number kept.)
3. **`document.body`'s class test** (pre-existing; PM to route; PM decision 133 keeps it out of T3). Text: "HTML §3.1.7
   DOM tree accessors: *the body element* is the first child of the `html` element that is either a body element or a
   frameset element, both HTML-namespace elements. `first_body_or_frameset_child`
   (`crates/dom/elidex-dom-api/src/char_data/document_props.rs:167-171` at `cee74989`; the `document.body` getter and the
   VM focus fallback) compares the tag only, so an SVG `body` child of `<html>` placed before the HTML body is returned
   as `document.body`. Since T3 the event-handler step-1 test is namespaced and this one is not: two predicates for one
   class test. **Trigger:** the next PR touching `document.body` or `elidex-dom-api` tree accessors. **Re-eval:**
   2026-12-31. ⚠ No remedy reviewed."
4. **Foreign elements take HTML interfaces** (pre-existing, engine-wide; PM to route; the ledger has no row for it,
   checked 2026-10-31; candidate home: the master roadmap's `M4-13:#svg` row). Text: "The VM chooses an element
   wrapper's prototype by tag name alone: `prototype_kind_for` returns `PrototypeKind::Element` for every `TagType`
   entity (`crates/script/elidex-js/src/vm/host_data/engine_feature/entity_kind.rs:164-165` at `cee74989`), and the
   per-tag selection (`crates/script/elidex-js/src/vm/host/elements.rs:216-`) uses `tag_matches_ascii_case`
   (`entity_kind.rs:124-133`), which reads no namespace, as do the per-interface receiver checks that call it
   (e.g. `html_style_proto.rs:90`). So an SVG or MathML element whose local name is an HTML tag (`a`, `script`,
   `style`, `title`, `div`, …) gets that HTML interface's prototype, and every other foreign element gets
   `HTMLElement.prototype`; no `SVGElement` / `MathMLElement` interface is exposed (WHATWG DOM §4.5 *element interface*;
   MathML Core §2.2.3; SVG2). Since T3 (#<PR>, `<squash>`) the `body` arm and `HTMLBodyElement`'s receiver check also
   require the HTML namespace (T3 memo D-T11), so a foreign `body` takes `HTMLElement.prototype`. The rest is ordered by
   what it needs: (i) arms whose local name is not an SVG or MathML element name can require the HTML namespace with
   no new interface (toward the spec at every member); (ii) arms whose local name is also a foreign element name
   (`a`, `script`, `style`, `title`, …) need the foreign interfaces first, since `HTMLElement.prototype` lacks members
   the foreign interface has (e.g. `SVGScriptElement`'s `type`). **Trigger:** an SVG / MathML DOM interface PR, the
   `M4-13:#svg` program's plan, or a WPT `svg/` / `mathml/` subset. **Re-eval:** 2026-12-31. ⚠ No remedy reviewed."
5. **The custom-element upgrade at a fragment-parsed element's creation** (rev 4.8; O-12 closed by PM decision 711
   (4) as (c)+(a); owner L3; proposed id `#11-custom-element-upgrade-at-fragment-creation`; registered at T3's
   landing; closed by a later L3 PR adding the `SubtreeCreated { cause: FragmentParse }` arm to
   `CustomElementReactionConsumer` after T3). Text: the draft handed to the PM with this rev (defect, spec steps, the
   registry-less side from L3's review, PM decision 713 (6), where the arm goes, closing condition); its one home is
   the ledger once registered, so it is not restated here. In short: a defined custom element created by a fragment
   parse is upgraded only at a connected `Insert`
   (`crates/dom/elidex-custom-elements/src/consumer.rs:128-155` at `cee74989`), where HTML §13.2.6.1 *create an
   element for a token* step 17 (willExecuteScript false in a fragment parse) and DOM §4.9 *create an element* steps
   4.4 / 5.2.2 enqueue the upgrade at creation, connected or not; an element whose looked-up registry is null (a
   DOMParser document's) stays not upgraded (HTML §4.13.3 *look up a custom element definition* step 1).

### §8.2 Hand-offs

- **B1** (`docs/plans/2026-06-agent-scoped-ecsdom-world.md` §5 req 5 / req 7): `ActiveDocument` is the single-browsing-
  context form of steps 3–4. Under a multi-realm World, "active" and "relevant global" become per document; B1's plan
  replaces `ActiveDocument`'s one constructor, `HostData::active_document()`, which both marshalling points read (the
  consumer's construction in `Vm::bind`, the VM accessor helper; PM 441, 455), and C1-pre's
  `document_has_browsing_context` (D-4; same source, D-T3) with a per-document resolution, and `determine_event_handler_target`'s body; no caller
  changes. BFCache (a document kept non-active with its Window) is the case that makes this a real distinction. The
  unbind caveat (H-T15) becomes structurally impossible under "1 agent = 1 World = 1 Vm". The ECS home of the pair is
  `#11-browsing-context-state-ecs-components` (PM decision 132).
- **N4** (node memo §0.4 D8): none from T3. C0 D-15 re-states N4's planned `effective_node_document` as C0b's
  `node_document`, which T3 only reads (D-T4).
- **Slice b**: S-5 removes X-W for XW and the D-8 restriction; it inverts C8, U2, C17's last four values and C18's
  later steps. S-1's remaining paths (declarative shadow content and template contents created by the pre-bind
  document parse, whose creation-equivalent point is the first bind, K-6) call T3's entry point
  `process_created_subtree` (or extend it), reading `ActiveDocument` at each creation (S-6 extended); the fragment
  path, DOMParser's build and the clone path are T3's, through `SubtreeCreated` (§4.5; PM decisions 705, 711), so a
  slice-b path that reaches a consumer through that event adds a cause there (until rev 4.8 the list of remaining paths also
  held the DOMParser build and the declarative shadow content and template contents a fragment parse creates). `#11-event-handler-content-attribute-applicability`: facets (1) and (3) closed by T3; slice b closes
  facet (2) (the attribute namespace, H-T19), extending D-T8's predicate to the namespace test.
- **C0** (landed: #595, #608, #611): T3 relies on C0b D-4 / D-5 (every insertion adopts, before `link_node` and the Insert event,
  so X-W's re-raw at T-a2 reads the adopted document), D-1 (`node_document`, including a contents fragment's), D-7 (a)
  (clause (b) reads the same function) and D-10, D-11, D-20 (every production creation stamps its node document). If
  C0's landed code changes any of them, D-T4 and rows XW-9/10, EW-15/16, XW-12 are re-derived (§9 item 1). Rev 4.5,
  at `cee74989`: D-1, D-4 / D-5 and D-7 (a) hold as relied on (H-T7, H-T8); D-8 / D-10 give a fragment parse's nodes
  the context's node document, which broke rev 4.5's D-T7 fragment-parse premise (P-T17); D-T7 now relies on them
  (PM decision 699: the fragment path is processed at creation with that document; PM decision 705: through §4.5's
  event; PM decision 711: DOMParser's build too, with the DOMParser document as the node document). The C0
  memo's H-T (the umbrella-edit split, PM decision 223 O-1 as amended by 271) is applied by this rev (§8.4).
- **C1** (landed: #626): T3 relies on D-C2 to D-C7 (every node in template contents, in a clone of them or under I
  resolves to I; held at `cee74989`, H-T21). The C1 memo has applied PM decision 153's order (its P-C11; edge c4 = umbrella x8).
- **`#11-capped-walker-deep-element-processing`** (L3, after RC-2a; PM 369): its one bounding rule covers D-T10's two
  walks (§8.3 item 11); if its PR lands first, they take its rule (§9 item 1).
- **L3's ecs-entity-lifetime PRs ig and io** (PM decisions 711, 713, 714; umbrella x12: T3 after io): T3 relies on
  ig's outermost consuming close and its slot-only window state (§4.5 item 4 (a), (b)) and on io's `pub` walker taking
  `WalkScope::WithTemplateContents` (§4.5 item 6); names cited by symbol and section from the PR (ig) / PR (io)
  plans and the L3 umbrella at the snapshot H-T32 names, re-pinned to L3's ratified sha before T3 is ratified and
  re-pulled at their landed heads (R7, §9 item 1). T3's PR adds the queue (`DispatcherSlot::Taken { pending }`) and
  the flush in ig's close (its shape open, P-T20 *Residual*); ig and io add neither.
- **RC-2a** (landed: #596; edge x11): T3 relies on the RC-2a plan §2 A (one emitter, after the whole copy
  exists), §4.3 (the walk's set and order) and §4.7 (no clone inside a dispatcher window). Rev 4.7 (PM decision 705):
  T3 generalises RC-2a's event (§4.5) and keeps all three for the `Clone` cause; L3 reviews §4.5 against its
  ecs-entity-lifetime PRs ig and io (R7; rev 4.8: §4.7's "no clone inside a dispatcher window" stays D-4's
  assertion, while a `FragmentParse` event inside one is queued, PM decision 711). If RC-2a's landed code
  changes any of them, D-T10 is re-derived (§9 item 1). Rev 4.5, at `cee74989`: all three hold (H-T24;
  `walkers.rs:145-199`; the `fire_clone_created` debug-assert, `ecs/dom/mod.rs:334-340`).

### §8.3 Records for existing slots (for the PM, exact texts; all at T3's landing unless stated)

1. `#11-event-handler-target-determination` — **close**, with this record: "Closed by T3 (#<PR>, `<squash>`): facets
   (1)–(4) realised by `determine_event_handler_target`, the only caller of steps 1–4 (attribute change steps, IDL getter
   and setter, a clone's creation); facet (2) in HEAD's single-browsing-context model (B1 replaces the source of
   `ActiveDocument`). T3 landed after C0 (`#11-cross-document-adopt-on-insert`) and C1
   (`#11-template-contents-owner-document`), so step 3 reads the spec's node document for template contents and after
   cross-document moves. Related defects homed elsewhere: the `frameset` interface → `#11-htmlframesetelement-interface`;
   foreign-element interfaces other than `body`'s → <the PM's home for §8.1 item 4>; the attribute change steps' step 1
   namespace test → `#11-event-handler-content-attribute-applicability` facet (2)." In the same edit, the row's first
   paragraph: "(`…/event_handler_processing/mod.rs:249`) … code as of `fd6955e8`" →
   "(`crates/script/elidex-script-session/src/event_handler_consumer.rs:328` at `fd6955e8`;
   `event_handler_processing/mod.rs:249` since #563)", "in three ways" → "in four ways (facet (4) below)", and
   "**Owner**: the IBP-sandbox program (Layout lane), since its N-series rewrites the handler path." → "**Owner**: the
   EH-attrs-at-creation program (Layout lane), T3 (PM decision 25; sub-bullet below)." (P-T1, P-T2.)
2. `#11-eh-attrs-creation-time-call-sites` — (a) defect (7): replace "(7) a `<body>` / `<frameset>` handler that
   delegates to the Window (X-W) is re-raw'd at every insertion over any write, and a `<body>` outside an active document
   delivers it to the page Window." by "(7) a `<body>` handler for a WindowEventHandlers name (X-W, narrowed by T3) is
   re-raw'd onto the Window at every insertion of the element into the active document, over any write (since T3 the
   Window-reflecting names on `<body>` and every Window-delegating name on `<frameset>` are not re-raw'd, PM decision
   129, and an insertion into a non-active document delivers nothing)." (b) Defect (2) "a never-inserted clone of an
   element with a handler content attribute reads `null` from `on<T>`" → closed by T3 (struck, with "closed by T3, the
   clone path of S-1, PM decision 153"). (c) Defect (3) → "after `c = document.body.cloneNode()`, a `window.on<T>`
   assignment for a WindowEventHandlers name is overwritten by `c`'s attribute at each insertion of `c` (X-W; since T3
   the attribute is also written at `c`'s creation, as the spec does, and the Window-reflecting names are not
   overwritten)". (d) Defect (1): append "(since T3 not for a clone: a clone is processed and marked at its creation)".
   (e) The classification sentence: drop "and (7)'s pairs that HEAD targets at the element (both to the spec's
   `null`)"; "three" becomes "two". (f) **Closes**: "which removes the restriction, X-W and processing at insertion"
   stays; the closing path reads "slice a lands → N1 → C0 → C1 → T3 → slice b" (PM decision 153). The umbrella §8.2
   carries the same text (defects in its *Rev 8.<t>*; the closing path by C0a, the C0 memo R-3 E12).
3. ~~Order lines of `#11-cross-document-adopt-on-insert` and `#11-template-contents-owner-document`~~ — withdrawn (rev 4):
   the C0 memo R-8 and the C1 memo R-2 / R-10 carry them.
4. `#11-event-handler-attribute-edge-cases` — append to facet (2): "Reachable through `<body onerror>` since T3 (the
   reflecting set now targets the Window)."
5. (withdrawn in rev 3; number kept.)
6. Umbrella K-0's E-G1 (i) and the ledger row's facet (4) bullet: closed by T3 (the record of item 1 covers both).
7. (withdrawn in rev 4.1: a Range clone through the canonical cloner fires `CloneCreated`, D-T10; number kept.)
8. `#11-event-handler-content-attribute-applicability` — **register** with rev 3's §8.1 item 2 text (header pointer),
   then append: "Facets (1) and (3) closed by T3 (#<PR>, `<squash>`,
   `is_event_handler_content_attribute_of`; PM decision 168): an HTML element accepts the GlobalEventHandlers names and,
   on `body` / `frameset` only, the WindowEventHandlers names (HTML §8.1.8.2); an SVG element every name (SVG2 §15.8);
   a MathML element the names HTML supports on all HTML elements (MathML Core §2.1.4, as the `MathMLElement` IDL
   defines: GlobalEventHandlers); any other namespace none (the attribute change steps' step 1; the three define them for their own namespaces only). Facet (2) (a non-null
   attribute namespace) remains, closed by slice b."
9. `#11-htmlframesetelement-interface` — **register** (PM decision 130, compat layer) with rev 3's §8.1 item 1 text
   (header pointer), unchanged.
10. `#11-unbound-attribute-writes-bypass-consumers` — append: "Since T3 (#<PR>, `<squash>`) an element cloned while
    unbound gets no `CloneCreated` processing; at a later insertion its ordinary and XW pairs are processed (slice a's
    fallback), its EW pairs (`body` × the Window-reflecting names, `frameset` × every Window-delegating name) never
    (T3 memo D-T7, D-T10)."
11. `#11-capped-walker-deep-element-processing` — append to the caller census: "T3 (#<PR>, `<squash>`) adds two
    uncapped callers in `process_created_subtree` (`elidex-script-session` `event_handler_processing/mod.rs`):
    `for_each_clone_created` and, per first-level template, `for_each_uncapped_shadow_inclusive(…,
    WalkScope::WithTemplateContents, …)` (T3 memo D-T10); and a third uncapped caller, the fragment path's creation
    walk, `for_each_uncapped_shadow_inclusive(root, WalkScope::WithTemplateContents, …)` over every node the parse
    created (T3 memo D-T12)." (Until rev 4.8 the third was "one capped caller … (`traverse_descendants`, through
    `children_iter_rev`'s `MAX_ANCESTOR_DEPTH` per-parent cap), which shares `handle_insert`'s collector".)
12. `#11-browsing-context-state-ecs-components` — append to C1-pre's R-11 defect: "T3 (#<PR>, `<squash>`) adds readers
    of the same association: `ActiveDocument`, constructed by `HostData::active_document()` and read at bind for the
    handler consumer and by the VM handler accessors (T3 memo D-T3)."

### §8.4 Umbrella in-place edits carried by this PR (PM decisions 129, 131, 153, 168, 699, 705, 711, 713, 714)

The edits and their sites are listed in the umbrella's changelog entry *Rev 8.<t>*; each edited clause keeps what it
said before (PM decision 131). `<t>` follows the umbrella's numbering rule (its *Rev 8.<t>* entry). By PM decision 223 O-1 as amended by 271 each row and
edge is added by its own PR: the C0 program row, the order line, x9, x10 and the order-consequence edits E1–E13 by C0a
(the C0 memo D-9, R-3); the C1 row and x7 by C1 (the C1 memo R-1). T3 keeps x8 (C1 → T3) and its own scope edits; it
does not restate C0's or C1's content.

## §9. Definition of done

1. T3's branch is cut from main after C0a–C0c, RC-2a, C1-pre0, C1-pre and C1 land (PM decision 153; all on main at
   `cee74989`); every `cee74989` coordinate in §0.2 is re-measured there by its command; the event-path PR-B's state is
   read there (`git grep -n 'fn active_document' -- crates`): if it landed first, T3 reuses its `ActiveDocument` and
   `HostData::active_document()` and adds neither (D-T3, PM 441); D-T4 is re-checked on C0b's landed `node_document`, H-T24 and
   D-T10 on RC-2a's landed clone path and on `#11-capped-walker-deep-element-processing`'s rule if landed (§8.2), H-T27 on `#11-event-path-window-missing` if
   landed (L3 may land it before T3; T3's element dispatches are non-bubbling with non-capture handlers, so a Window on
   the path changes none of them), and the memo's coordinates and *Base* columns are updated before
   implementation starts; existing tests changed by D-T8 are listed (§5.2). Rev 4.8 (PM decisions 711, 713, 714): the
   branch is cut after L3's PR (io) lands (so after ig; umbrella x12); §4.5 item 5's take / restore anchors are
   re-pulled at ig's head and the walker's names (`WalkScope`, `for_each_uncapped_shadow_inclusive`'s signature) at
   io's, both against H-T32. Rev 4.8.1: before T3 is ratified, every L3 citation is re-pinned from the H-T32 snapshot
   to L3's ratified sha (R7 premise 1); before implementation, P-T20 *Residual* is decided (R7 premise 3); ig's
   landed guard is checked for §4.5 item 4's guarantees (a) and (b), and T3 stops for the PM on R7's stop condition
   (no consuming `close` on such a window's returning path, a second home of take/restore state, or the settled
   flush shape not buildable on the landed guard).
2. `/elidex-plan-review` passes on this memo (mandatory, §1.3).
3. Red-first: T-1–T-9, T-14, T-15, T-15b, T-16, T-16b, T-16c, T-17, T-19, T-20, T-20b, T-21, T-22, T-22b, T-22c, T-23, T-24,
   T-25, T-26, T-27, T-28, U-T1–U-T3, U-T6, U-T7, U-T8, U-T9, U-T10, U-T11, T-E1
   committed failing; the fix makes them and §5.2's rewritten cells green; the guards (T-6b, T-8a, T-9b, T-16d, T-18, U-T4)
   are green at the head; the static-marked expectations (§5.2, T-6b) are confirmed by their runs.
4. `/usr/bin/grep -rn 'resolve_handler_target\|is_body_element' crates` and `/usr/bin/grep -rn 'fn window_entity'
   crates/core/elidex-ecs` → 0 hits (`HostData::window_entity`, `bind.rs:492`, and the `HostDriver` trait item
   `window_entity`, `session/engine.rs:565`, implemented at `elidex-js/src/engine.rs:694`, stay; rev 4.4 called the
   latter "the `ScriptEngine` trait method": it is a `HostDriver` item, and `ScriptEngine`, `engine.rs:57`, is a separate
   trait); with `<filter>` = `grep -v -E ':[[:space:]]*//'` (drops doc lines): `git grep -n 'fn active_document' --
   crates | <filter>` → exactly one, `HostData::active_document` (`host_data/engine_feature/bind.rs`), if the
   event-path PR-B has not landed, else three (that one, the `HostDriver` item, its `ElidexJsEngine` impl), T3 adding
   none (PM 441); `git grep -n -E 'ActiveDocument \{' -- crates ':!*/tests/*' ':!*tests*.rs' ':!*test_*.rs' | <filter>`
   → the definition line and exactly one construction, inside `HostData::active_document`, in either order (PM 441);
   `git grep -n -P '^\s*impl\b[^{;]*\bActiveDocument\b' -- crates` → 0 (`-P`: under `-E` a `\b` matches nothing);
   `git grep -n 'active_document()' -- crates/script/elidex-js/src/vm/vm_api.rs` → at least one, the `Vm::bind`
   consumer construction (PM 455);
   `determine_event_handler_target(` → the definition and the five call sites of §4.2;
   `is_event_handler_content_attribute_of(` → the definition and two calls; `is_x_w_pair(` → the definition and one
   call; `process_created_subtree(` → the definition and one call (the `SubtreeCreated` arm, both causes; PM decision
   705); `git grep -n -e CloneCreated -e fire_clone_created -- crates` → 0; `git grep -n 'fire_subtree_created(' --
   crates ':!*tests*'` → the definition and two calls (the dom-api cloner, `parse_fragment_progressive`; §4.5 item 3);
   `git grep -n -w 'Taken' -- crates/core/elidex-ecs/src/dom/mod.rs` → every construction is `Taken { pending: … }`
   (§4.5 item 4); `git grep -n 'LifetimeUnit' -- crates/script/elidex-script-session` → 0 (D-T12);
   `/usr/bin/grep -n
   'tag_matches_ascii_case(entity, "body")' crates/script/elidex-js/src/vm/host/elements.rs
   crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` → each hit paired with the namespace read (D-T11).
5. `cargo test -p elidex-script-session -p elidex-js -p elidex-ecs -p elidex-shell -p elidex-html-parser -p
   elidex-dom-api -p elidex-custom-elements -p elidex-form --all-features` green (the last four: §4.5's migration); `mise run ci`
   before push.
6. §8.1 / §8.3 texts handed to the PM in the PR description.
7. **Checker** (run; do not trust a stored verdict): `python3 .claude/skills/elidex-plan-review/preflight.py
   docs/plans/2026-10-eh-attrs-t3-target-determination.md` and the same on the umbrella exit 0.

## §10. Open questions

**Decided:** O-1 → PM decision 129 (D-T7); O-2 → PM decision 130 (D-T9); O-3 → PM decision 131 (§8.4); O-4 → PM
decision 132 (D-T3); O-5 → the PM's reading of PM decision 133 (D-T5); O-6 → PM decision 153 (order; R-A, R-B by C0 /
C1; R-C, R-D1 in T3); **O-7 → (a), PM decision 168** (D-T8: T3 takes applicability facets (1) and (3); facet (2) stays
with slice b; §2.1 has 0 rows worse than the base); **O-8 → PM decision 359** (D-T10: the clone path consumes RC-2a's
`CloneCreated`; edge x11); **O-10 → (a), PM decision 699** (D-T7: a fragment-parsed element is processed at its
creation; its node document is the context's, and steps 3–4 of *determine the target of an event handler* (HTML
§8.1.8.1) ask whether that document is active, not whether the element is connected, the step-3 note's disconnected
`body` included; rows EW-20, EW-21, XW-15; cells T-22 to T-24; §1.2's R-E; PM decision 129's premise holds with it).
Rejected by PM decision 699: (b) an EW delivery once at the first insertion and (c) EW keeping X-W's re-raw at every
insertion, since both deliver at a time the spec does not ((c) also leaves the older form beside the new one, a
strangler intermediate); (d) a stated residual, since the spec decides the outcome. Rev 4.5's text of O-10:
`git show 662e0377:docs/plans/2026-10-eh-attrs-t3-target-determination.md` (§10).
**O-11 → S-i × C-ii, PM decision 705** (D-T12; the event: §4.5): the whole fragment path is T3's, processed at
creation, every class, over the insertion-time walk set, by the consumer's one `SubtreeCreated` arm; `CloneCreated` is
generalised (no new variant beside it) and every consumer migrates. Rejected by PM decision 705, with D-T12's
reasons: C-i (the policy routed into the mutation layer, Layering), S-ii (two processing points), S-iii (a scope per
entry point), C-iii (T-22 / T-22b red at T3). Rev 4.6's facts F-1 to F-4 are answered by H-T29 to H-T31 (F-1, F-3),
D-T12 *Rejected* (F-2) and §4.5 item 6 (F-4); rev 4.6's premise that the build's take keeps the event from the
consumer is P-T18. Rev 4.6's text of O-11: `git show 0e1ef07e:docs/plans/2026-10-eh-attrs-t3-target-determination.md`
(§10). Rev 4.8: PM decision 711 overrides rev 4.7's walk set ("over the insertion-time walk set" → every node the parse
created, `WalkScope::WithTemplateContents`) and its drop inside a dispatcher window (→ queued, flushed at the outermost
close); PM decision 713 settles the queue's home and the flush (§4.5 item 4) and the raw-handler requirement (D-T13);
PM decision 714 assigns the guard to L3's ig and the walk scope to io (T3 after io, umbrella x12).
**O-12 → (c)+(a), PM decision 711 (4)** (§4.5 item 6, §7, §8.1 item 5): T3 does not change custom-element behaviour;
the defect is recorded as an L3-owned slot (proposed id `#11-custom-element-upgrade-at-fragment-creation`), closed by
a later L3 PR adding the `SubtreeCreated(FragmentParse)` arm after T3; L3's registry-less point (PM decision 713 (6))
is in its text. Rejected: (b) T3 adds the arm. Rev 4.7's text of O-12:
`git show 83075ea0:docs/plans/2026-10-eh-attrs-t3-target-determination.md` (§10).

**Answered (folded):** Q-7 → H-T27; Q-8 → H-T28; Q-9 → D-T11, §8.1 item 4; Q-10 → H-T7, D-T4 ((ii)); Q-11 → H-T24;
Q-12 → H-T25; Q-13 → H-T21, P-T16; Q-14 → D-T11 (moot: `host_data/` split by #578, `HostData::dom_shared`); Q-15 → D-T10's *Order* (RC-2a's walk, the
RC-2a plan §4.3; PM decision 359 folded it into the plan-review), cell T-16b. Rev 1's
Q-1 to Q-6 → H-T15 to H-T20.

**For the PM (designer's findings, not decided here):**
- **P-T20 *Residual* — open (rev 4.8.1), for the PM with L3.** Where T3's flush sits against the PR (ig) plan's
  `Drop` as the one site of the restore (`close` = `drop(self)`; after that drop `close` holds no dom): (α) restore
  in `close` too, (β) dispatch in `Drop` after a `close` hand-off, (γ) dispatch by each window's caller. §0.3 P-T20
  states the constraint each one moves; T3 does not choose (R7 premise 3).
- **O-9 — closed (PM 367): (b), owner L3.** RC-2a's implementation PR makes `for_each_clone_created` uncapped (iterative, creation order) with its own deep-tree cell; T3's deep-element processing rests on it (edge x11).

No browser measurement is needed: every outcome in §6 is spec-defined, including FN-4's (MathML Core §2.1.4, H-T23).
