# C0 `EH-attrs-at-creation-C0` — every insertion adopts the node into the parent's node document (per-PR plan-memo)

Nested PR **C0** of the standalone program `EH-attrs-at-creation` (umbrella
`docs/plans/2026-10-event-handler-attrs-at-creation.md`, approved by #551 `b9122157` at rev 8, carried to rev 8.3 by
#570; "the umbrella"). It closes slot `#11-cross-document-adopt-on-insert` (carved by S5-4a #444) and, with it, slot
`#11-custom-elements-adopted-callback` (D-17 #227), whose trigger is "cross-document adoption infra landing" (H-12); it
closes the non-template, non-planned facets of slot `#11-ownerless-node-creation` (owner Layout C0 / C1; D-10, D-11).
Siblings cited by ID: IBP-sandbox N1 (`docs/plans/2026-10-ibp-sandbox-handler-gates.md`, rev 4.5, merged as a plan by
#570, implementation landed by #580 `27b50dfe`; "the N1 memo"), its node memo `docs/plans/2026-09-ibp-sandbox.md` ("the node memo";
amended in place by this PR, D-15), C1 (`docs/plans/2026-10-eh-attrs-c1-template-owner-document.md`, rev 3 on branch
`layout-eh-c1-template-owner-plan` at `cab354ba`, rebased on `27b50dfe`, unpushed, not on main; "the C1 memo"; content
that exists only in its rev 2 `040bf257` is cited as "C1 rev 2", only in its rev 1 `ee430f5e` as "C1 rev 1"), T3
(`docs/plans/2026-10-eh-attrs-t3-target-determination.md`, rev 3 on branch `layout-eh-attrs-t3-plan` at `14ff3dc2`, not
on main; "the T3 memo"), and L3's Range program (`docs/plans/2026-11-range-contents-program.md`, Rev 3 on branch
`range-full-impl-plan` at `7ac367a3`, unpushed, read with `git show 7ac367a3:<path>`; "the L3 memo"; its Appendix A site
IDs A1–A12 are cited as that memo's). Unlanded IDs are cited as those memos' IDs.

**Placement (evidence).** PM decision 143 (2026-10-31; ledger `project_open-defer-slots.md`, slot
`#11-cross-document-adopt-on-insert`, sub-bullet "Owner assigned 2026-10-31"): "Layout lane, as C0 of the EH-attrs
program. C0 is a standalone PR before C1 … Edge-dense, so plan-review is mandatory. L3's P1 umbrella treats C0 as
landing first (the insertion-steps boundary goes in C0's plan §2)". PM decision 153 (2026-10-31, user decision): **the
program order is N1 implementation → C0 → C1 → T3 → slice b**. PM decision 223 O-2 (2026-11-01): C0 owns lifting the
new-node owner-document entry into `EcsDom`. So at C0's base N1's implementation has landed and C1, T3 have not: HEAD's
`resolve_handler_target` (`crates/script/elidex-script-session/src/event_handler_processing/mod.rs`) is the target
rule, and T3's `determine_event_handler_target` does not exist (§8 H-T).

**Scope.** One fact — the node document, stored as `AssociatedDocument` — set at creation by one entry, kept true by
one mechanism (DOM *adopt*) run at the one tree-linking chokepoint, and read by one function:
1. `EcsDom::node_document` (NEW): DOM §4.4 *node document*; the sole reader. `owner_document` stays the `ownerDocument`
   getter's "null for a Document" wrapper over it; its two tree-position fallbacks and the getter's `document_root()`
   fallback are deleted (D-1).
2. `EcsDom::node_document_for_new_nodes` (NEW; new home — #577's strict-parser method of that name is deleted): the one place
   a new node's owner is decided, `node_document(source)` with the §13.4 throwaway substitution (D-10); every
   production node-creation site takes its owner from it (D-11).
3. `EcsDom::adopt` (NEW): DOM §4.5 *adopt* steps 1–3 (3.1 node document, 3.3.1 attribute nodes, 3.3.3 `adoptedCallback`
   with each node's own (old, new) pair, 3.4 the template adopting steps under C2) (D-2, D-3).
4. Insertion adoption at the three linking primitives at DOM *insert* step 7.1 / *replace* step 6, and the session's
   replace-with-fragment arm (D-4, D-5).
5. `MutationEvent::Adopt` (NEW), consumed by the custom-element consumer (`adoptedCallback`) (D-6).
6. Every node-document derivation reads `node_document`: N1's composed-root proxy, `owner_doc`, the `unwrap_or(x)`
   idioms (D-7); the node memo's `effective_node_document` becomes `node_document` in place (D-15).
7. `document.adoptNode` (D-13) and the *replace* mutation-record order (D-12).
8. Prerequisite pure-move splits, each a standalone PR before C0: the session's `mutation/mod.rs` and the VM's
   `vm/host/document.rs` (D-14).

**Not in scope.** Template contents' owner document and the template adopting steps' target `f(document)` (C1); the
Range program's planned sites A6–A12 (L3 RC-2 / RC-4 call the entry, §4.5); the Selection API's document-membership
check (§7); `Node.isConnected`'s bound-document compare (§7, D-16); custom-element registry steps 3.2 / 3.3.2 (§7);
`importNode` (§7); `replaceChild(X, X)`'s records (§7); `moveBefore` (not implemented).

**Changelog.** *Rev 2.1* (2026-11-03): re-framed onto `27b50dfe`; §3 vs D-6 wording; C1 rev 3 reference. Rebased
without conflict from `593aa314` onto `27b50dfe` (#578, #580); `scripting.rs` coordinates re-measured on N1's landed
body (§0.1 frame check); §3's *adopt* 3.3.3 row now states D-6's decision (one event per adopt call, each node's own
pair); the C1 memo is rev 3 (`cab354ba`), its scope statements re-checked (§2.1 A7, §4.6, §8 c0b / H-C1, Q-2's C1
constraint); §4.9 and §5.2 gain N1's landed test file. No decision changed. *Rev 2* (2026-11-01): rebased onto `593aa314` (#576, #577, #579, #575), every coordinate re-measured;
folds the eleven inputs of the Layout driver's rev-2 brief (§10): node memo amended in place (D-15, PM 157);
`document.adoptNode` in scope (D-13, PM 158); umbrella row = C0's own §1.2 row only, rev number assigned at landing
(D-9, PM 159 / 223 O-1); replace record order fixed and `mutation/mod.rs` split first (D-12, D-14, PM 160); `isConnected`
not covered → slot (D-16, PM 161); the new-node entry lifted into `EcsDom` and every creation site migrated (D-10, D-11,
PM 223 O-2); L3's A1–A9 mapped (§4.5); the `CloneCreated` × `Adopt` census line (§8); C1 rev 2's P-C14 / P-C15 folded
(D-6's per-node pairs, D-10's overwrite note); scout answers recorded (§11). New axis A8 (creation) and A9 (records):
9 axes. *Rev 1* (2026-10-31): first draft, pre-plan-review.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` is a **`27b50dfe`** coordinate (`origin/main` at the 2026-11-03 fetch: #578 and #580, both merged
2026-11-01, on top of rev 2's `593aa314`; this branch rebased onto it). N1's implementation (#580) is in this base, so
`scripting.rs` is cited on N1's landed body. C0's implementation is cut after the prerequisite splits (D-14), so every
coordinate in `mutation/mod.rs` and `vm/host/document.rs` is **re-measured at that base** (§9 item 1). #578 split
`crates/script/elidex-js/src/vm/host_data/mod.rs` into `vm/host_data/engine_feature/`; this memo cites no item of
`vm/host_data/` (by line or by symbol). Code references come from `/usr/bin/grep -n` / `git grep -n` / `sed -n` over the
`593aa314` tree, run 2026-11-01, and are `27b50dfe` coordinates by the frame check below. Every §
number ↔ title pair and every algorithm step comes from `.claude/tools/webref` (`heading`, `dfn`, `body`), run
2026-11-01; the commands are in §3.

*Rev 2.1 frame check (2026-11-03).* `git diff --name-only 593aa314 27b50dfe -- crates | wc -l` → 26 (verified 2026-11-03). Intersected with
the paths this memo cites (`git diff --name-only 593aa314 27b50dfe -- crates | while read f; do s=${f#*/src/};
/usr/bin/grep -qF "$s" docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md && echo "$f"; done`) it returns
`crates/script/elidex-script-session/src/scripting.rs` and `crates/script/elidex-script-session/src/lib.rs`; the second
is a false hit (this memo's `lib.rs` is `crates/dom/elidex-html-parser/src/lib.rs`, unchanged). So `scripting.rs`
(293 → 279 lines) is the one cited file that changed; every other cited file is byte-identical and keeps its numbers.
Re-measured in `scripting.rs` (`593aa314` → `27b50dfe`): the function `:83` → `:97`, the Document arm `:96` → `:107`,
the node arm `:104-110` → `:115-121` (`find_tree_root_composed` `:104` → `:115`, `owner_document` `:109` → `:120`), the
match `:95-113` → `:106-124`, the docstring slot paragraph `:35-72` → `:49-87`, its "when it lands" sentence `:51-53` →
`:65-67`, the slot-name comment `:51` → `:65` (plus `:83`); cited in H-5, D-7 (a), §3, §4.2, §4.9, §5.1 P-3 / P-4,
§5.3. #580 also renamed the function's parameters (`entity` → `target`, `document` → `bound_document`, both now
non-`Option`) and reworded the `None` case to "the node clause is false"; H-5 and D-7 (a) quote the landed text. The
§5.1 populations were re-run at `27b50dfe` (P-1 to P-8): only P-3's and P-4's `scripting.rs` lines moved; P-8's raw
output gains two lines, both inside the inline `#[cfg(test)]` module of the new
`elidex-script-session/src/event_handler_reconcile.rs` (`:106`, `:200`), so §4.4's production census is unchanged.

### §0.2 Premises re-measured

| id | claim | evidence (`27b50dfe`) |
|---|---|---|
| H-1 | No insertion adopts | `/usr/bin/grep -rnF 'adopt_subtree(' crates --include='*.rs'` → definition `crates/core/elidex-ecs/src/dom/tree/teardown.rs:251`, prod caller `finish_detached_fragment` (`:337`), test `crates/core/elidex-ecs/src/dom/tests/destroy.rs:324`. `append_child` (`crates/core/elidex-ecs/src/dom/tree/mutation.rs:22`) runs `detach_with_hook` (`:40`) → `link_node` (`:43`) → `fire_after_insert` (`:47`) and never writes `AssociatedDocument` |
| H-2 | Tree linking has one chokepoint | `/usr/bin/grep -rn 'link_node(' crates --include='*.rs'` → definition `mutation.rs:210` (`pub(super)`) and three calls `:43`, `:119`, `:197`; `/usr/bin/grep -rn 'rel.parent = ' crates --include='*.rs'` → `mutation.rs:218` (link) and `crates/core/elidex-ecs/src/dom/tree/mod.rs:85` (clear). `attach_shadow_with_init` links the shadow root through `append_child(host, sr)` (`crates/core/elidex-ecs/src/dom/shadow.rs:203`), the clone path through `append_child` (`crates/core/elidex-ecs/src/dom/tree_clone.rs:419`) |
| H-3 | `owner_document` is stamp-first with two tree-position fallbacks | `EcsDom::owner_document` `crates/core/elidex-ecs/src/dom/mod.rs:817`: Document → `None`; a live-Document stamp; else the tree root if a Document; else a `DocumentFragment` root's stamp; else `None`. The getter adds a third fallback: `owner_document(this).or_else(\|\| dom.document_root())` (`crates/dom/elidex-dom-api/src/node_methods/core.rs:279`) |
| H-4 | Which nodes carry a stamp at HEAD | stamped: `create_*_with_owner` callers (`createElement` and kin, `crates/dom/elidex-dom-api/src/document.rs:235`, `:291`), deep clones (`clone_subtree` `tree_clone.rs:186`), strict foreign elements and strict template contents (#577: `node_document_for_new_nodes`, `crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs:271`, used at `insert.rs:108` and `shadow.rs:124`), tolerant template contents (`crates/dom/elidex-html-parser/src/convert.rs:284`), and every node a fragment parse returns (`finish_detached_fragment` re-homes, `teardown.rs:336-337`). **Unstamped**: every other parser creation (strict `insert.rs:61`, `:133`, `:141`, `:155`, `:210`, `shadow.rs:153`; tolerant `convert.rs:251`, `:305`, `:309`, `:316`), shadow roots (`shadow.rs:189` spawns none), shallow clones (`clone_node_shallow` `tree_clone.rs:251`; `crates/dom/elidex-dom-api/src/node_methods/clone.rs:295` "The shallow cloner never stamps"), Attr entities (`create_attribute` `mod.rs:634`), doctypes (`create_document_type` `mod.rs:616`), and the dom-api / engine sites of §4.4 |
| H-5 | N1's proxy (clause (b)), on N1's landed body | `scripting_disabled_for_platform_object` `crates/script/elidex-script-session/src/scripting.rs:97`; Document arm (`:107`); node arm: composed tree root if a Document, else `owner_document` (`:115-121`); docstring slot paragraph (`:49-87`), "when it lands … this rule collapses to a plain `owner_document(target) != bound_document` compare" (`:65-67`) |
| H-6 | "Document → itself" idioms | `owner_doc` `crates/dom/elidex-dom-api/src/element/document_base.rs:173-178` (callers `:313`, `:365`, `:395`); `owner_document(x).unwrap_or(x)` `crates/dom/elidex-dom-api/src/element/href_accessor.rs:50`, `crates/dom/elidex-dom-api/src/char_data/document_props.rs:75` |
| H-7 | The session record layer models adopt step 2 | `capture_move_source` / `source_removal_record` / `move_record_list` (`crates/script/elidex-script-session/src/mutation/mod.rs:304-384`); `expand_fragment`'s "step 7: per-node adopt + insert" (`:441`) describes an adoption HEAD does not do |
| H-8 | Replace-with-fragment bypasses `EcsDom::replace_child` | `apply_replace_child` (`mutation/mod.rs:672`) fragment arm: `remove_child(parent, old_child)` (`:713`), then `expand_fragment` with `insert_before` / `append_child` per child (`:719-722`). The fragment is never re-homed |
| H-9 | `MutationEvent` has no adoption variant | `pub enum MutationEvent<'a>` `crates/core/elidex-ecs/src/dom/mutation_event.rs:107`, seven variants (`Insert` `:131`, `Remove` `:172`, `TextChange` `:191`, `ReplaceData` `:209`, `SplitText` `:237`, `NormalizeMerge` `:273`, `AttributeChange` `:306`); one exhaustive production `match` (`crates/dom/elidex-dom-api/src/range/live.rs:113-163`, scout Q-S1), the others have `_ => {}` |
| H-10 | Fire order inside the primitives | `append_child`: `is_connected` (`:39`), `detach_with_hook` (`:40`, def `:442`), `link_node` (`:43`), `rev_version` (`:44`), `fire_after_insert` (`:47`, def `:282`, suppressed for shadow-root node or parent `:289`). `replace_child` (`:138`): `detach_with_hook(new_child)` (`:164`), `detach(old_child)` (`:186`), `fire_after_remove` (`:189`), `link_node` (`:197`), `fire_after_insert` (`:201`) |
| H-11 | Fragment builds run with the dispatcher taken | `begin_detached_fragment` (`teardown.rs:293`) takes it, creates the throwaway (`:301`) and root (`:302`); callers: strict `tree_builder/mod.rs:180`, tolerant `crates/dom/elidex-html-parser/src/lib.rs:143`; `parse_into_inert_document` likewise (`crates/dom/elidex-form/src/inert_document.rs:112`) |
| H-12 | `adoptedCallback` has a reserved slot and no fire path | `CustomElementReaction::Adopted { entity, old_document, new_document }` (`crates/dom/elidex-custom-elements/src/reaction.rs:21-25`); flush no-op (`crates/script/elidex-js/src/vm/host/custom_elements/flush.rs:133-138`); consumer matches `Insert` / `Remove` / `AttributeChange` (`crates/dom/elidex-custom-elements/src/consumer.rs:75-96`) |
| H-13 | `EcsDom::is_connected` counts any Document root | `crates/core/elidex-ecs/src/dom/tree/walkers.rs:143-158` |
| H-14 | HEAD's target rule ignores the node document | `resolve_handler_target` `event_handler_processing/mod.rs:249-255` |
| H-15 | `adoptNode` / `importNode` do not exist | `/usr/bin/grep -rn 'adoptNode\|importNode\|adopt_node' crates --include='*.rs'` → no hit |
| H-16 | The walk's order | `for_each_uncapped_shadow_inclusive` (`walkers.rs:99`): node, its shadow tree, its light children, and — with `TemplateContentsWalk::Include` — a template's contents **after the template's whole subtree** (`:107-113`) |
| H-17 | Attr entities | `create_attribute` (`mod.rs:634`) spawns `AttrData` + `NodeKind::Attribute`, no `TreeRelation`, no stamp; callers `createAttribute` (`crates/dom/elidex-dom-api/src/char_data/attr.rs:35`) and `getAttributeNode` (`attr.rs:90`), which caches it in `AttrEntityCache` (`crates/core/elidex-ecs/src/components.rs:675`) on the element. `attr.ownerDocument` routes to the dom-api getter (`native_node_get_owner_document` → `ownerDocument.get`), so it reads `document_root()` today |
| H-18 | DOMParser documents are never destroyed | `parse_into_inert_document` (`inert_document.rs:110`) has no destroy path; `/usr/bin/grep -rn 'destroy_entity\|despawn' crates/dom/elidex-form/src/inert_document.rs` → no hit (C1 memo H-C12 measured the same) |
| H-19 | The fragment substitution lives in the strict tier only | `node_document_for_new_nodes` (strict, `pub(super)`): fragment context → `owner_document(context)`, else the parse document (`tree_builder/mod.rs:267-276`); the tolerant tier threads `owner_doc = dom.owner_document(context)` into template contents only (`lib.rs:158`); dom-api sites call `owner_document(this)` (§4.4) |
| H-20 | Touched files over 1000 lines | `wc -l` at `27b50dfe` (each unchanged from `593aa314`): `crates/script/elidex-script-session/src/mutation/mod.rs` 994, `crates/script/elidex-js/src/vm/host/document.rs` 1197, `crates/dom/elidex-dom-api/src/element/table_mutation/mod.rs` 994, `crates/dom/elidex-dom-api/src/selection.rs` 1036 (not touched, D-1) |

### §0.3 Premise errors (reported)

- **P-1 (brief, input 4 (b)).** The brief places the 994-line `mutation/mod.rs` under `elidex-ecs/src/dom/`: no such
  file at `593aa314` (`ls crates/core/elidex-ecs/src/dom/` has no `mutation/` directory). The 994-line file D-5 / D-12 edit is
  `crates/script/elidex-script-session/src/mutation/mod.rs` (`wc -l` → 994, unchanged). (A second 994-line file,
  `table_mutation/mod.rs`, is touched by D-11; §5.3.)
- **P-2 (brief, input 7 A8).** "A8 stamps only the shadow root": `clone_node_with_shadow_honor` stamps the replicated
  shadow root **and** the cloned host (`node_methods/clone.rs:283-284`) and each shallow shadow child (`:299`). The L3
  memo's Appendix A row says the same ("the replicated shadow root and its children are stamped"). D-11 makes the
  shallow cloner stamp, which subsumes all three.
- **P-3 (brief, input 8).** `CloneCreated` does not occur in the L3 memo at `7ac367a3`
  (`git show 7ac367a3:docs/plans/2026-11-range-contents-program.md | /usr/bin/grep -c CloneCreated` → 0; it speaks of
  "one VM post-clone seam"). §8 states the census line with the name as relayed.
- **P-4 (brief, input 9 (a)).** "rev 1 H-2's A3×A4 premise": the premise was rev 1's §2.2 A3×A4 row; "H-2" is the C1
  memo's hand-off ID for it (C1 memo P-C14, H-2). Content folded (D-6).
- **P-5 (brief, input 6 × C1 rev 2).** The brief says C1 routes "both parser tiers' template-contents creation" through
  the entry; C1 rev 2's D-C4 says C1 moves **every** parser creation site of both tiers onto it, and its O-2 (a)
  says "If (a) [C0 adds it], D-C4 and §4.3 shrink to 'call the entry'". With C0 owning the entry, C0 migrates every
  non-template creation site (D-11) and C1 changes `attach_template_contents` and its four callers' semantics only (§8
  H-C1). C1 rev 3 applies this (its D-C4: "C1 adds no entry, no entry branch and no creation-site change"; §10.0).
- **P-6 (rev 1, T4).** Rev 1 gave T4's HEAD as `[?, false]` assuming the tree-root walk crosses the shadow root. It does
  not (`find_tree_root` stops at the shadow root, `walkers.rs:319`; scout Q-S9): an unstamped shadow root reads `None`
  and the getter's `document_root()` fallback gives the page document — so for a host appended into the page the HEAD
  value is accidentally the spec's. T4 now appends into a second DOMParser document.
- **P-7 (rev 1, T12).** Rev 1 said the base-URL maintainer "updates the page document". The Insert arm routes the
  recompute to `owner_doc(x)` = the page (x's stamp), whose tree does not contain `x`, so neither document changes
  (scout Q-S10); `dp`'s base URL is never recomputed. T12's HEAD column is restated.
- **P-8 (code comment).** `expand_fragment`'s "step 7: per-node adopt + insert" (`mutation/mod.rs:441`) and
  `fire_after_remove`'s "DOM `insert` adopts the node" (`tree/mutation.rs:330-339`) describe an adoption HEAD does not
  perform. C0 makes them true.
- **P-9 (replace order).** `apply_replace_child`'s comments cite *replace* as "spec removes old_child at step 11, before
  new_child's adopt at step 13" and step over `old_child` when capturing the source-removal record's siblings
  (`mutation/mod.rs:761-785`, step numbers also at `:664`, `:790`); its fragment arm cites steps 7–14 (`:690-743`). Current *replace* (`webref body dom
  concept-node-replace`): 1 pre-insert validity, 2–3 referenceChild, 4 previousSibling, 5 removedNodes, **6 adopt node**,
  7 remove child (suppressed), 8 nodes, 9 insert (suppressed), 10 queue the record. So adopt's removal sees `child`
  still in place: for `P = [A, B]; P.replaceChild(B, A)` the source-removal record has `previousSibling = A`; elidex
  reports `null`. Fixed in C0 (D-12, PM 160 (a)).
- **P-10 (`owner_document`'s docstring).** "falls back to the tree-root walk so that legacy entities created without the
  component still resolve" (`mod.rs:812-815`): the unstamped population is not legacy — it is most parser output,
  every shadow root and every shallow clone (H-4). C0 stamps them at creation (D-11) and deletes the fallback (D-1).
- **P-11 (`split_text.rs` docstring).** Cites "WHATWG DOM §4.10 'split a Text node'" and "Step 4" (`split_text.rs:1-2`,
  `:140`); `webref dfn dom 'split a Text node'` → §4.11 Interface Text, and the creation is step 5. Rewritten with D-11's
  edit of that site.
- **P-12 (VM `isConnected`).** `native_node_get_is_connected` compares the composed root with the **bound** document
  (`crates/script/elidex-js/src/vm/host/node_proto.rs:441-463`); `EcsDom::is_connected` (H-13) and the spec say a node
  in a DOMParser document's tree is connected. It is a tree relation, not a node-document read (D-16).
- **P-13 (Selection).** The ledger lists `selection.rs` among the `owner_document` consumers. `same_document`
  (`crates/dom/elidex-dom-api/src/selection.rs:614-619`) is the Selection API's document-membership test; webref cannot
  render the Selection API's method anchors (`webref body selection-api dom-selection-collapse` → "anchor … not
  encountered"), so its step text is unverified here (§11 Q-S6). C0 does not touch it (D-1); after C0 its value for a
  connected node equals the spec's document membership (I-U), and for a detached node it stays HEAD's.

### §0.4 Decisions (this PR)

- **D-1 — the node document is the stored fact, read by one function.** `EcsDom::node_document(&self, node: Entity)
  -> Option<Entity>` (NEW), in `crates/core/elidex-ecs/src/dom/mod.rs` beside `owner_document`: a `NodeKind::Document`
  → `Some(node)` (DOM §4.4: "The node document of a document is that document itself"); any other node → its
  `AssociatedDocument` if that is a live Document, else `None` (the dangling guard). (a) `owner_document` keeps its name
  for the `ownerDocument` getter's contract ("return null, if this is a document; otherwise this's node document"):
  `None` for a Document, else `node_document`; **both tree-position fallbacks of H-3 are deleted**. (b) Its production
  callers become exactly two: the getter (`node_methods/core.rs:279`) and `same_document` (`selection.rs:618`, P-13,
  not touched); every other site reads `node_document` (D-7) or the entry (D-10). (c) The getter's
  `.or_else(|| dom.document_root())` is deleted: after D-11 every production creation is stamped, so the fallback
  answers only for a node whose stamp is missing, and answers with the bound document by fiat — the same kind of proxy
  C0 removes. `None` reaches the getter only for a node of a documentless fragment context or a destroyed document
  (A5×A8). *Side-store rule* (CLAUDE.md): already a component (`AssociatedDocument`, `components.rs:595`), `Send +
  Sync`, holds an `Entity`; no side-store is added. *Idiom table:* OO engines' per-node document pointer updated by
  `adoptNode` / `insertBefore` → the `AssociatedDocument` component written by creation (D-10) and by the one adopt
  function (D-2); the document's own "ownerDocument is null" special case → `owner_document` over `node_document`.
- **D-2 — `EcsDom::adopt(&mut self, node: Entity, document: Entity)` (NEW) is DOM §4.5 *adopt*.** Step 1 oldDocument =
  `node_document(node)`; step 2 if `node` has a parent, remove it through the primitives' removal path
  (`detach_with_hook`, firing `MutationEvent::Remove`); step 3 the walk of D-3, then one `MutationEvent::Adopt` (D-6)
  when any pair it collected is set. A `None` oldDocument (a node whose stamp is missing — after D-11, test fixtures and
  documentless contexts only) re-homes **silently**: no spec state corresponds to a node without a node document, so
  there is no "old document" to pass to `adoptedCallback`. Home: `crates/core/elidex-ecs/src/dom/tree/mutation.rs`
  (the removal path is private there). Callers: the three primitives (D-4), the session's fragment arm (D-5), the
  session's `apply_adopt_node` (D-13).
- **D-3 — the adopt walk (`adopt_subtree`).** `adopt_subtree` moves from `teardown.rs` to `mutation.rs` beside `adopt`
  (one home for the algorithm) and keeps its walk — `for_each_uncapped_shadow_inclusive(root,
  TemplateContentsWalk::Include, …)`, uncapped, iterative. It (a) sets every visited node's `AssociatedDocument` to
  `document` (step 3.1); (b) re-homes each visited element's cached Attr entities (`AttrEntityCache` entries) to
  `document` (step 3.3.1, H-17); (c) for each template contents fragment it reaches, records a **contents segment**
  `{ fragment, documents: Option<(old, new)> }` with `old` = the fragment's node document before the walk and `new` =
  the target the walk writes into it, `documents = None` when `old` is `None` or equals `new` (the nested adopt's step 3
  guard). Under C2 the target is `document` — HTML §4.12.3's template adopting steps ("Let document be node's node
  document's appropriate template contents owner document"; "Adopt node's template contents … into document") are the
  identity under C2; C1 replaces that one edge with `f(document)` (C1 memo D-C5). The segments carry each contents
  node's own (old, new) pair to D-6; nothing downstream assumes the root's pair for contents (P-4; C1 memo P-C14).
  Steps 3.2 and 3.3.2 (registries) are §7 rows. `finish_detached_fragment` keeps calling it (D-8).
- **D-4 — the primitives adopt at the spec's position.** Each of `append_child`, `insert_before`, `replace_child`
  computes `target = node_document(parent)` and, in place of its bare `detach_with_hook(child, parent)`, runs D-2: the
  removal (as today), then — if `target` is `Some` and differs from the child's node document — the walk and the
  `Adopt` event, then `link_node` and `fire_after_insert` as today. If `target` is `None` (an ownerless parent; A8) the
  child keeps its stamp. For `replace_child` the adopt of `new_child` precedes `old_child`'s removal (*replace* step 6
  before 7), which is the primitive's existing order (H-10). §2.0 states the position against the spec.
- **D-5 — replace with a DocumentFragment adopts the fragment.** *Replace* step 6 adopts `node` itself, the fragment
  included; *insert* step 7.1 then adopts each child (a no-op). The session's fragment arm (H-8) calls
  `EcsDom::adopt(fragment, d)` with `d = node_document(parent)` (if `Some`) before `remove_child(parent, old_child)`.
  Pre-insert of a fragment (`append` / `insertBefore`) adopts only the children (insert step 7.1; the fragment keeps
  its node document), which the per-child primitives already give.
- **D-6 — `MutationEvent::Adopt` (NEW) carries each adopted node's (old, new) pair.** Shape: `Adopt { node: Entity,
  documents: Option<(Entity, Entity)>, contents: &'a [AdoptedContents] }` with `AdoptedContents { fragment: Entity,
  documents: Option<(Entity, Entity)> }` (NEW), the segments of D-3 (c) in walk order; one event per adopt call,
  dispatched after the walk and before the root is linked (D-4), iff `documents` or any segment's `documents` is
  `Some`. Every node of the root's shadow-including subtree outside contents has the root pair; every node inside a
  contents fragment has that segment's pair (nested contents: the innermost segment's). **Consumer:** the custom-element
  consumer adds an `Adopt` arm that visits the subtree in the spec's order — shadow-including tree order, with each
  template's contents visited **immediately after the template**, before its light children (the template's adopting
  steps run at its step 3.4, inside the per-descendant loop; H-16's walker visits contents after the subtree, so the
  arm walks its own order) — and enqueues `CustomElementReaction::Adopted { entity, old_document, new_document }` for
  each element in `CEState::Custom` whose pair is `Some` (adopt step 3.3.3, « oldDocument, document »). The VM flush
  invokes `adoptedCallback` with the two Document wrappers. *Batch equivalence:* the spec interleaves "set node
  document" and "enqueue" per descendant; enqueuing runs no script (reactions run at the next `[CEReactions]`
  boundary), so re-homing first and enqueuing after, in the same order, is unobservable. Other consumers: `_ => {}`; the
  one exhaustive match (H-9, `range/live.rs`) gains an explicit no-op arm (live ranges are unaffected by adoption:
  adopt step 2's removal already fired `Remove`). *Idiom table:* OO engines' per-node-class "moved to new document"
  virtual override → one event on the mutation stream + a consumer arm per system that cares.
- **D-7 — every node-document derivation reads `node_document`.** (a) N1's clause (b) proxy is deleted: the Document
  and node arms become one, `dom.node_document(target).is_some_and(|d| d != bound_document)`; the docstring's slot
  paragraph (`scripting.rs:49-87`) is rewritten; `None` → the node clause is false, unchanged (N1's landed docstring
  `:69-70`; the node memo's direction rule). (b)
  `document_base.rs`'s `owner_doc` is deleted and its three callers read `node_document`. (c)
  `owner_document(x).unwrap_or(x)` at `href_accessor.rs:50` and `document_props.rs:75` becomes `node_document(x)`
  (a Document is its own). (d) `focus/delegate.rs:85`, `tree_clone.rs:208`, `node_methods/clone.rs:242`,
  `teardown.rs:336` read `node_document` (values change only where the stamp did, §5.1 P-3). (e) `selection.rs` is not
  changed (D-1 (b), P-13).
- **D-8 — the fragment parse keeps its early re-home.** `finish_detached_fragment` calls `adopt_subtree(root, doc)`
  with `doc = node_document(context)` (not `adopt`): it models the adoption the caller performs when it inserts the
  returned nodes, done early because the throwaway document is destroyed first (`teardown.rs:316-343`). It runs with the
  dispatcher taken (H-11), so no `Adopt` event is observed — matching the spec, where the fragment parser creates no
  custom element there (it does not run constructors: *create an element for a token* sets *will execute script* only
  when "the parser was not created as part of the HTML fragment parsing algorithm"). The caller's insertion then sees
  oldDocument = the context's document and does nothing (D-2 step 3 guard).
- **D-9 — the umbrella's §1.2 row is C0's to add; only that row (PM decisions 159, 223 O-1).** The PR that lands this
  memo on `main` carries, in place, C0's own row in umbrella §1.2 and a changelog entry for it, and no other row: C1 and
  T3 add their own rows in their PRs (one owner per row). **Revision-number rule:** the umbrella revision number is
  assigned by whichever of the C0 / C1 / T3 PRs lands first, as the next number after `main`'s current one (8.4 if
  `main` is still at 8.3), the next landing taking the next; a branch writes its umbrella edit without a number and
  fills it at its rebase before merge. This memo claims no number (rev 1's "rev 8.4" is withdrawn). The order line and
  any §1.3 edge with C0 as an endpoint are not rows of C0's; O-1 asks the PM (§11). Exact text: §8 R-3.
- **D-10 — one new-node owner entry: `EcsDom::node_document_for_new_nodes(&self, source: Entity) -> Option<Entity>`
  (NEW).** Contract: the node document a node created on behalf of `source` receives — `node_document(source)`
  (DOM's "given *X*'s node document"; for a Document source, itself) — with exactly one substitution: if that document
  carries `FragmentParseDocument { context_document: Option<Entity> }` (NEW component on the §13.4 throwaway), the
  result is `context_document` (the context's node document at build start; `None` for a documentless context).
  - *The substitution's home.* `begin_detached_fragment` gains the context: `begin_detached_fragment(&mut self, context:
    Entity)`; it creates the throwaway, creates root with `node_document_for_new_nodes(throwaway)` (= the throwaway: the
    component is inserted after root, so root is created in the throwaway as §13.4 says), appends it, then inserts
    `FragmentParseDocument { context_document: node_document(context) }` on the throwaway. The component dies with the
    throwaway (both teardowns despawn it), so no state outlives the build. *Side-store rule:* a per-entity fact about the
    throwaway, `Send + Sync`, not a per-VM handle → a component, not an `EcsDom` field. *Rejected:* an explicit
    `NewNodeContext` parameter on every creation (C1 rev 2 Q-1's first option: meaningful for the two fragment tiers,
    `None` at every dom-api caller of §4.4 — a parameter dead at all but two call sites); an `EcsDom` field holding the
    active context (state about an entity kept off the entity; the strict rollback would have to clear it).
  - *Why a substitution at all.* Linked parse nodes are re-homed by `finish_detached_fragment`, so for them the creation
    stamp is unobservable; the substitution exists for nodes **no link reaches** — template contents fragments — which
    would otherwise keep the throwaway's stamp and dangle in a documentless context (#380 Codex R2; H-19), and so that
    C1's `f` is never computed for a throwaway (C1 memo D-C4).
  - *Overwrite note (C1 memo P-C15; brief input 9 (b)).* In a fragment build a linked node is created with the context's
    document, then C0's link-time adopt (D-4) re-homes it to the throwaway (the parent's node document), then
    `finish_detached_fragment` re-homes it back. The intermediate stamp is unobservable (the dispatcher is taken, no
    script runs); the entry's docstring states it, so its "context's document" is read as the creation stamp, not the
    stamp during the build.
  - *C1's hook point (explicit).* The entry has no template branch: inside template contents `source` is the contents
    fragment or a descendant, whose stamp `attach_template_contents` wrote. At C0 every caller of
    `attach_template_contents(template, owner)` passes `node_document_for_new_nodes(template)` as the second argument
    (strict, tolerant, `createElement`; the clone post-pass passes the pair's document); C1 D-C3 redefines that argument
    as "the template's node document" and applies `f` inside the helper, so the entry returns the inert document I for
    every node created in contents with no change to the entry.
  - *Callers move to it* (D-11): #577's strict tree-builder method of the same name is deleted; every
    production creation passes `dom.node_document_for_new_nodes(source)` as its owner.
- **D-11 — every production node-creation site takes its owner from the entry.** The census (§4.4, re-runnable
  command in §5.1 P-8) is migrated in this PR: both parser tiers (elements, foreign elements, text, comments, doctypes,
  the DSD template; `source` = HTML §13.2.6.1's intended parent / target parent, or the document for a doctype), shadow
  roots (`attach_shadow_with_init` spawns with `node_document_for_new_nodes(host)`, DOM *attach a shadow root* step 5
  "given element's node document"), Attr entities (`create_attribute` gains `owner`; `createAttribute` → `this`,
  `getAttributeNode` → the element), doctypes (`create_document_type` gains `owner`), the dom-api sites (convert nodes
  into a node, `insertAdjacentText`, `textContent` / `replaceChildren` strings, `splitText`, the `<select>` length
  setter, `<style>` text, the table helpers through one `create_table_element` (NEW, HTML §4.9.1 *create a table
  element*), the custom-element constructor spawn, the inert document's `<html>`), Range A1–A4 (when present at the
  base, §4.5), the shallow cloner (`clone_node_shallow` stamps the operation's document, DOM *clone a node*'s default
  "node's node document"; P-2), and the engine-internal creations (pseudo-element text, anonymous table wrappers, form
  reset text: `source` = the element they are created for). The four ownerless shims (`create_element`, `create_text`,
  `create_comment`, `create_document_fragment`) keep their signatures for test fixtures and have **zero production
  callers** at C0's head (ratchet: §5.1 P-8). The clone paths keep their document **parameter** (DOM *clone a node*
  takes `document`, default `node_document(src)`); they are the one creation path that passes an argument rather than
  calling the entry (C1 memo D-C6, §4.4: the clone post-pass passes the pair's document).
- **D-12 — the *replace* mutation records follow the current step order (PM 160 (a)).** `apply_replace_child`'s
  non-fragment arm captures the source-removal record's siblings **without** stepping over `old_child` (adopt step 2
  runs at replace step 6, before step 7 removes `old_child`); the coalesced record's `previousSibling` (step 4) and
  `nextSibling` (steps 2–3) are unchanged. The fragment arm's comments are renumbered to steps 2–10 and its order
  becomes: step 6 adopt the fragment (D-5), step 7 remove `old_child`, steps 8–9 expand, step 10 the coalesced record.
  Records produced: unchanged count and order; only the source record's siblings change (T17).
- **D-13 — `document.adoptNode(node)` (PM 158).** DOM §4.5 steps: 1 a Document → `NotSupportedError`; 2 a ShadowRoot →
  `HierarchyRequestError`; 3 adopt node into this; 4 return node. Layering: the VM binding (a `Document.prototype`
  native, marshalling only) → dom-api handler `AdoptNode` (NEW, `crates/dom/elidex-dom-api/src/document.rs`, registered
  in `registry.rs`) → session `apply_adopt_node(dom, node, document) -> Vec<MutationRecord>` (NEW, in the session's
  child-list module after D-14) = the adopt step 2 source-removal record (`source_removal_record`, H-7) then
  `EcsDom::adopt`. `importNode` stays out (§7).
- **D-14 — prerequisite pure-move splits, each a standalone PR before C0 (PM 160 (b); CLAUDE.md touch-time split).**
  (a) `crates/script/elidex-script-session/src/mutation/mod.rs` (994): seam = the child-list family — `MoveSource`,
  `capture_move_source`, `source_removal_record_from`, `source_removal_record`, `move_record_list`,
  `convert_arg_source_records`, `expand_fragment`, `rejects_shadow_root_insertion`, `apply_append_child`,
  `apply_insert_before`, `apply_remove_child`, `apply_replace_child`, `apply_replace_all` (`:296-895`) — moves to
  `mutation/child_list.rs` (NEW); `mod.rs` keeps `Mutation`, `MutationKind`, `MutationRecord`, `apply_mutation`, the
  record constructors and the attribute / character-data appliers, and re-exports the moved `pub` items so no import
  path changes. (b) `crates/script/elidex-js/src/vm/host/document.rs` (1197; D-13 adds `adoptNode`'s native): seams =
  the node factories (`native_document_create_element` … `native_document_create_document_fragment`, `:278-422`) →
  `vm/host/document_factories.rs` (NEW; `adoptNode` joins it in C0), and the accessors (`:423-939`) →
  `vm/host/document_accessors.rs` (NEW); the method tables stay. (c) `table_mutation/mod.rs` (994): D-11's
  `create_table_element` makes the eight creations one-liners; if `wc -l` at C0's head reaches 1000, the DomApiHandler
  structs (`:697-992`) move to `table_mutation/handlers.rs` (NEW) as a standalone first commit (§5.3). Each split is
  verified as a pure move (`git diff --stat` + `cargo test -p <crate>` green, no logic diff).
- **D-15 — the node memo's `effective_node_document` becomes `node_document`, in place (PM 157).** Applied by this
  commit (§8 R-4): D8, §4 "Node document", the form-submission and F10r lines, the composition text and edge n3 name
  C0's `EcsDom::node_document` (NEW: C0), with a dated amendment line; N4 calls it, adds no function.
- **D-16 — `isConnected` is not covered by the `node_document` unification (PM 161).** Connectedness is a tree relation
  ("shadow-including root is a document"), not the node document; deleting the proxy of D-7 does not touch
  `native_node_get_is_connected`, whose defect is comparing the root with the *bound* document (P-12). After C0 a node
  in a DOMParser tree has node document `dp` and still reports `isConnected === false`. So it stays out of C0's DoD and
  goes to slot `#11-vm-isconnected-bound-document` (owner L3; text §8 R-6 (d)).

## §1. Shape and verdict

**Axes (§2): 9** — A1 the stored fact and its reader (D-1, D-7), A2 the insertion position (D-4, D-5), A3 the walk
(D-3), A4 the adoption notification (D-6), A5 fragment and DOMParser builds (D-8, D-10's substitution), A6
node-document consumers (D-7), A7 the EH program's contract (N1, slice a, C1, T3), A8 node creation (D-10, D-11), A9
mutation records (D-12, D-13). ≥ 3 intersecting axes: edge-dense (PM decision 143); plan-review is mandatory
(CLAUDE.md *Edge-dense work* (b)). As a per-PR slice of the approved umbrella it is a terminal unit once its
plan-review passes (base case (c)).

**Split decision: one feature PR, preceded by pure-move prerequisite PRs (D-14).** The feature candidates and why each
split is worse:
- *Adoption first, consumer collapse later.* Between the two PRs the stored fact would be correct and N1's proxy would
  still derive the document by position: two derivations of one fact (CLAUDE.md *One issue, one way*).
- *Consumer collapse first.* Without adoption the stored fact is wrong for every moved node (H-1), so deleting the
  proxy or the fallbacks makes N1's gate and the getter worse than HEAD.
- *Creation entry later (C0 adopts, a later PR stamps at creation).* Deleting the fallbacks requires every
  script-reachable node to be stamped; without D-11 the unstamped detached population (H-4: shallow clones, Attrs,
  table sections, `append('text')` texts of a detached parent) would read `None` → the getter would return null where
  HEAD returns the page document. Keeping the fallbacks until then leaves a position proxy beside the fact.
- *`adoptedCallback` / `adoptNode` later.* `MutationEvent::Adopt` would ship with no consumer, or adopt step 3.3.3
  stay unimplemented on the first path that reaches it; `adoptNode` is the adopt algorithm's own entry (PM 158).
- *Replace record order later.* Same algorithm, same function (`apply_replace_child`), edited by D-5 anyway (PM 160).
So fact, creation, maintenance, reader and notification land together; the moves land first and alone. Preflight's
breadth verdict (§3: M = 25 rows over K = 2 specs, SPLIT-RECOMMENDED) is answered by this decision: the rows are the
creation, maintenance and reads of one fact, and every row-cluster split above leaves two derivations of it or a
consumer-less event; the separable work (the moves) is already split out (D-14).

**K-0 lens (umbrella K-0: no step worse than HEAD).** Every observable change of §5.4 moves an outcome from HEAD's to
the spec's. Two are more restrictive than HEAD (R-3: a bound node moved into a DOMParser subtree stops compiling raw
handlers; R-1: the `ownerDocument` of a moved node changes), both the spec's.

### §1.1 Browser measurement

None is needed for C0's decisions: every outcome in §6 is defined by DOM §4.2.3 / §4.4 / §4.5 / §4.9 / §4.11, HTML
§4.9.1 / §4.12.3 / §4.13.6 / §13.2.6.1 / §13.4; the one elidex state with no spec counterpart (a node without a node
document, A8) is decided by D-2's and D-4's direction rules. The one spec-vs-browser question touched nearby
(`replaceChild(X, X)`'s records) is out of scope (§7) and carries its own measurement step there.

## §2. Coupled invariants

### §2.0 Where adopt sits inside *insert* (the boundary for L3's P1)

DOM §4.2.3 *insert* (`webref body dom concept-node-insert`), for each node in nodes, in tree order: **7.1 "Adopt node
into parent's node document"**; 7.2 / 7.3 append or insert into parent's children; 7.4–7.6 slot assignment; 7.7 for
each shadow-including inclusive descendant: 7.7.1 the insertion steps, 7.7.3 `connectedCallback` or upgrade; then **8**
queue the tree mutation record (unless suppressObservers); **9** the children changed steps; **10–12** the
post-connection steps over a static list. Before step 7: 4.1 removes a fragment's children (suppressed) and 4.2 queues
the fragment's record. DOM §4.5 *adopt* (`webref body dom concept-node-adopt`): step 2 removes node from its old parent
(*remove*, unsuppressed), step 3 per shadow-including inclusive descendant: 3.1 node document, 3.3.1 attributes, 3.3.3
`adoptedCallback`, 3.4 adopting steps. *Replace* (`webref body dom concept-node-replace`): **step 6 adopt node** before
step 7 (remove child, suppressed) and step 9 (insert, which adopts again as a no-op).

elidex after C0, per primitive (H-10 for HEAD's order):

| elidex step | spec step | event / record |
|---|---|---|
| `is_connected` capture | (input to 7.7 and the CE gates) | — |
| removal from the old parent (`detach_with_hook`) | adopt step 2 (*remove*) | `MutationEvent::Remove` (old parent); session source-removal record (H-7) |
| re-home (`adopt_subtree`): stamps, attrs, contents segments | adopt 3.1, 3.3.1, 3.4 (template adopting steps; identity target under C2) | — |
| `MutationEvent::Adopt` (NEW) | adopt 3.3.3 (`adoptedCallback` enqueue, per-node pair) and the adopting-steps hook | iff some pair is `Some` |
| `replace_child` only: `old_child` removal | replace step 7 | `Remove` (old child) |
| `link_node` | insert 7.2 / 7.3 | — |
| `rev_version` | (elidex cache invalidation) | — |
| `fire_after_insert` | insert 7.4–7.7, 9, 12 (consumers), 8 (session record layer) | `MutationEvent::Insert` |

**The contract P1 inherits:** adoption is complete (stamps written, `Adopt` dispatched) before `link_node`, and therefore
before every insertion-steps, children-changed, post-connection and connectedCallback consumer and before the
destination record; it follows the old parent's removal (`Remove`, removing steps, source record). P1 may restructure
`fire_after_insert` into explicit hook phases; it must not move the adopt step after any of them. The adopting-steps
hook is `MutationEvent::Adopt` (one per adopt call, carrying per-node pairs), not a per-descendant hook; a
per-descendant hook, if P1 wants one, replaces D-6's consumer walk, not D-4's position.

### §2.1 Invariant per axis

| axis | invariant |
|---|---|
| A1 fact | **I-U (uniformity):** for every node n with a parent or host p, if `node_document(p)` is `Some(d)` then `node_document(n) == Some(d)` (template contents are not children: their fragment's stamp is set by `attach_template_contents` and by the walk's contents edge). `node_document` reads only the stamp (or a Document's identity); no production reader derives a node document from tree position or from `document_root()` |
| A2 position | At every link the child's adoption completes after its old-parent removal and before `link_node`; in `replace_child`, before `old_child`'s removal; a replaced-in fragment is adopted itself; a pre-inserted fragment keeps its node document |
| A3 walk | Adoption re-homes the root's shadow-including inclusive descendants and each element's cached Attr entities, uncapped, and each reached template's contents to the contents target (C2: the same document), recording each contents fragment's (old, new) pair |
| A4 notification | `Adopt` fires once per adopt call iff some pair is a (live, different) document pair, before that root's `Insert`; every custom element of the adopted subtree whose pair is `Some` gets exactly one `Adopted` reaction with its own pair, in the spec's order (contents right after their template), enqueued before its `Connected` reaction |
| A5 builds | Dispatcher-suppressed builds (fragment parse, DOMParser, strict rollback) produce no observed `Adopt`; every node a build returns carries the receiving document's stamp; the `FragmentParseDocument` component exists only during a build |
| A6 consumers | Every production node-document read goes through `node_document`, or through `owner_document` at exactly two sites (D-1 (b)); no `find_tree_root_composed`-based or "Document → itself" local derivation remains |
| A7 EH contract | N1's compile gate and (after C0) T3's step 3 read the node document D-1 defines; slice a's Insert arm (T-a2) sees the adopted document; C1 edits only the walk's contents edge (each segment's `new`, C1 D-C5) and `attach_template_contents`; it does not change the `Adopt` consumer (C0's, D-6; cells T18 / U13), and its T25 pins the pairs' values only; K-0 holds |
| A8 creation | Every production creation takes its owner from `node_document_for_new_nodes(source)` (or, for a clone, the operation's document parameter); the four ownerless shims have zero production callers; a node is ownerless only if its source has no node document |
| A9 records | The session's records follow the current step order: replace's source-removal siblings are captured with `child` in place; `adoptNode` of a parented node queues exactly the *remove* record; adoption itself queues no record |

### §2.2 All pairs (36), with intersection

| pair | intersect? | intersection |
|---|---|---|
| A1×A2 | yes | I-U is established by D-4 at every link (H-2's chokepoint); a primitive that linked before adopting would let an `Insert` consumer read the old stamp (U4) |
| A1×A3 | yes | I-U across the host edge needs a shadow-including walk; the contents edge's stamp comes from the same walk (C2: same document); a light-only walk leaves shadow content on the old document (U1, T4) |
| A1×A4 | yes | Each pair's `old` is a stamp read before the walk, `new` the stamp written; the consumer reads no tree position |
| A1×A5 | yes | Build nodes are created with the substituted document (D-10), re-homed to the throwaway at link, re-homed to the context's document at finish; the strict rollback despawns, so no throwaway stamp survives except in a documentless context, where `node_document`'s dangling guard reads `None` |
| A1×A6 | yes | Deleting the fallbacks (D-1) is sound only if every script-reachable node is stamped: I-U for linked nodes, D-11 for creations; U8 and U12 scan parsed trees of both tiers, T19–T22 the detached dom-api creations |
| A1×A7 | yes | N1's node clause becomes `node_document(entity) != bound`; for a connected node this equals the proxy's composed-root answer (I-U across host edges), for a detached node it is the stamp its last adoption wrote (T8–T10) |
| A1×A8 | yes | Creation is the stamp's first writer; adoption its only later writer; `None` now means "source had no node document" (or a destroyed document) |
| A1×A9 | yes | `adoptNode`'s record is the removal's; the stamp change itself is not a mutation record (DOM queues none), so no record kind is added |
| A2×A3 | yes | The walk runs between removal and link, on a detached subtree, so it never visits the new parent's tree |
| A2×A4 | yes | `Adopt` precedes `Insert` for the same root (D-4); in `replace_child`, `Adopt(new_child)` precedes `Remove(old_child)` (step 6 before 7), so `adoptedCallback` for the new child is enqueued before `disconnectedCallback` for the old one (T7 variant) |
| A2×A5 | yes | During a build each link adopts from the substituted document to the throwaway (unobserved); the caller's later insertion is a no-op (D-8; U9) |
| A2×A6 | yes | The base-URL maintainer's `Insert` arm reads `node_document(node)` after the re-home, so a moved `<base>` updates the receiving document; its `Remove` arm reads the old parent's document before (T12) |
| A2×A7 | yes | Slice a's T-a2 processes a node's raw handlers at its insertion, after D-4's adoption, so N1's gate sees the adopted document there (T8) |
| A2×A8 | yes | A node created by the entry under `source = p` and then linked under `p` is a no-op adopt (same stamp); linking under an ownerless parent keeps the stamp (U3) |
| A2×A9 | yes | The record layer captures the source record before calling the primitive (H-7); D-12 changes which siblings it reads for replace, not when |
| A3×A4 | yes | The consumer visits the walk's node set in spec order, contents right after their template (not H-16's order), each node with its segment's pair; custom elements inside contents exist (P-4: created in a document, appended into `tpl.content`), so per-node pairs are load-bearing (T18, U13) |
| A3×A5 | yes | The finishing re-home uses the same walk (D-8), contents included; a build's template contents were created with the substituted document and are re-homed with the rest |
| A3×A6 | yes | Every consumer reads the node's own stamp (attrs included); none walks up |
| A3×A7 | yes | A handler on a shadow-tree element of a moved host reads the new document through its own stamp, not a composed root (T4 handler variant); C1's D-C5 replaces only the contents target |
| A3×A8 | yes | Attr entities are stamped at creation (D-11) and re-homed by the walk (D-3 (b)); an uncached Attr has no entity to re-home |
| A3×A9 | no | The walk queues no record; records come from the removal (A2×A9) |
| A4×A5 | yes | Builds run with the dispatcher taken, so no `Adopt` reaches a consumer and no `adoptedCallback` comes from a build (U9) |
| A4×A6 | no | `Adopt` has one consumer (custom elements); node-document consumers read the stamp, not the event |
| A4×A7 | yes | `adoptedCallback` runs script at the next `[CEReactions]` boundary, after the insertion completes; slice a's T-a2 processing happens inside the `Insert` dispatch, before it |
| A4×A8 | yes | After D-11 a script-reachable custom element always has a node document, so the silent re-home (D-2) cannot swallow an `adoptedCallback` (scout Q-S2: no ownerless custom-element path, `entity_spawn.rs:57` takes an owner) |
| A4×A9 | yes | `adoptNode` (D-13) fires `Adopt` like an insertion's adoption; its removal record and `Adopted` reaction come in spec order (remove, then 3.3.3) (T15, T16) |
| A5×A6 | yes | A DOMParser document's nodes read the DOMParser document through their stamp; after a move they read the receiving document (T1) |
| A5×A7 | yes | N1's x4 hand-off (raw values at DOMParser-node creation read null) is unchanged: those nodes stay in the DOMParser document until moved |
| A5×A8 | yes | The substitution is the entry's only special case and exists only while `FragmentParseDocument` exists; a documentless context yields `None` creations, the same as HEAD's orphan-detached result (`teardown.rs:318-322`) |
| A5×A9 | yes | `parse_into_inert_document` drops its records (throwaway, unobserved); unchanged |
| A6×A7 | yes | N1's clause (b) and the node memo's D8 rule are one function after C0 (D-15); T3 reads `node_document` directly (§8 H-T) |
| A6×A8 | yes | The getter's `document_root()` fallback is deleted (D-1 (c)) on the strength of D-11's zero-ownerless ratchet; a missed site would read null, which T19–T22 and U12 guard |
| A6×A9 | no | Node-document consumers read no records; records read no node document |
| A7×A8 | yes | Slice b (after C1) processes raw values at creation; the creation stamp D-11 writes is the node document N1's gate reads there, so a DOMParser-created node is gated by its own document from creation |
| A7×A9 | no | The EH contract reads no mutation record content |
| A8×A9 | yes | Range A1–A4 (extract) create the fragment and Text clones whose records RC-2 produces; their stamps come from the entry, their records are RC-2's (§4.5) |

Count: 36 pairs; 32 intersect, 4 do not (A3×A9, A4×A6, A6×A9, A7×A9). Re-count:
`awk '/^### §2.2/{f=1} /^## §3/{f=0} f && /^\| A[0-9]×A[0-9]/' <memo> | awk -F'|' '{print $3}' | sort | uniq -c`.

## §3. Spec coverage map

Lookups (2026-11-01): `heading dom 4.2.1 / 4.2.2 / 4.2.3 / 4.2.6 / 4.3.2 / 4.4 / 4.5 / 4.8 / 4.9` → Document tree /
Shadow tree / Mutation algorithms / Mixin ParentNode / Queuing a mutation record / Interface Node / Interface Document /
Interface ShadowRoot / Interface Element; `heading html 2.4.3 / 4.9.1 / 4.12.3 / 4.13.6 / 8.1.8.1 / 8.5.1 / 13.2.6.1 /
13.4` → Document base URLs / The table element / The template element / Custom element reactions / Event handlers / The
DOMParser interface / Creating and inserting nodes / Parsing HTML fragments. `dfn dom`: 'node document' → §4.4
`#concept-node-document`; 'adopting steps' → §4.5 `#concept-node-adopt-ext`; 'convert nodes into a node' → §4.2.6;
'string replace all' → §4.4; 'clone a node', 'clone a single node' → §4.4; 'attach a shadow root' → §4.9; 'split a
Text node' → §4.11 Interface Text; 'create an element' → §4.9. `dfn html`: 'enqueue a custom element callback reaction'
→ §4.13.6; 'appropriate template contents owner document' → §4.12.3; 'create an element for a token' → §13.2.6.1;
'parse HTML from a string' → §8.5.1. Step text: `body dom concept-node-insert` (7.1), `concept-node-adopt` (1–3, 3.3.1,
3.3.3, adoptNode 1–4), `concept-node-replace` (1–11), `concept-node-clone` (default document; 4 append), `clone-a-single-node`
(2.4 / 4 "given document"), `concept-attach-a-shadow-root` (5 "given element's node document"),
`convert-nodes-into-a-node` (1, 3), `string-replace-all` (2 "given parent's node document"), `concept-text-split` (5
"given node's node document"), `dom-element-insertadjacenttext` (1 "given this's node document"),
`dom-document-createattribute` (3 "creating an attribute given this"), `dom-parentnode-append` (1 "this's node
document"), `concept-range-extract` (1 "given range's start node's node document", 4.1 "a clone of"),
`dom-node-ownerdocument`; `body html create-an-element-for-the-token` (3 "intendedParent's node document"),
`insert-a-comment` (4 "targetParent's node document"), `insert-a-character` (text "given targetParent's node
document"), `the-template-element` (adopting steps 1–2), `the-table-element` (*create a table element* "given
tableElement's node document"), `dom-table-createthead` (2).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §4.2.3 Mutation algorithms | *insert* step 7.1 | adopt each node into parent's node document before 7.2 | `append_child` `crates/core/elidex-ecs/src/dom/tree/mutation.rs:22`, `insert_before` `:95` (D-4) | ✓ | yes |
| WHATWG DOM §4.2.3 Mutation algorithms | *insert* steps 4, 7.2–7.7, 8–12 | position of adopt relative to each (§2.0) | `fire_after_insert` `mutation.rs:282` (unchanged); session record layer (H-7) | ✓ | yes |
| WHATWG DOM §4.2.3 Mutation algorithms | *replace* steps 1–11 | 6 adopt node (a fragment included) before 7; record siblings per current order | `replace_child` `mutation.rs:138` (D-4); `apply_replace_child` `crates/script/elidex-script-session/src/mutation/mod.rs:672` (D-5, D-12) | ✓ | yes |
| WHATWG DOM §4.2.3 Mutation algorithms | *pre-insert*, *replace all* | delegate to *insert* | `apply_append_child` `mutation/mod.rs:480`, `apply_insert_before` `:545`, `apply_replace_all` `:813` (unchanged; reach D-4) | ✓ | yes |
| WHATWG DOM §4.2.6 Mixin ParentNode | *convert nodes into a node* 1, 3; `append` 1 | text and fragment given this's node document | `collect_nodes` / `convert_nodes_into_node` `crates/dom/elidex-dom-api/src/child_node/mod.rs:302`, `:328`, `:364` (D-11) | ✓ | yes |
| WHATWG DOM §4.5 Interface Document | *adopt* 1, 2, 3.1, 3.3.1, 3.4 | oldDocument; remove; per descendant: node document, attributes, adopting steps | `EcsDom::adopt` (NEW), `adopt_subtree` `crates/core/elidex-ecs/src/dom/tree/teardown.rs:251` (moves) (D-2, D-3) | ✓ | yes |
| WHATWG DOM §4.5 Interface Document | *adopt* 3.3.3 | custom → `adoptedCallback` « oldDocument, document »: one `Adopt` event per adopt call, carrying each node's own pair (the root's outside template contents, the contents segment's inside, the innermost segment's for nested contents); the consumer enqueues one reaction per custom element whose pair is set, in spec order (contents right after their template) | `MutationEvent::Adopt` (NEW); `crates/dom/elidex-custom-elements/src/consumer.rs:75`; `crates/script/elidex-js/src/vm/host/custom_elements/flush.rs:133` (D-6) | ✓ | yes |
| WHATWG DOM §4.5 Interface Document | *adopt* 3.2, 3.3.2 | shadow-root / element registries | not modelled (§7) | ✗ | yes |
| WHATWG DOM §4.5 Interface Document | `adoptNode` 1–4 | Document → NotSupportedError; ShadowRoot → HierarchyRequestError; adopt; return | dom-api `AdoptNode` (NEW), session `apply_adopt_node` (NEW), VM native (NEW) (D-13) | ✓ | yes |
| WHATWG DOM §4.4 Interface Node | *node document*; `ownerDocument` getter | every node has one, set upon creation; a document's is itself; getter null for a document | `node_document` (NEW), `owner_document` `crates/core/elidex-ecs/src/dom/mod.rs:817` (D-1); getter `crates/dom/elidex-dom-api/src/node_methods/core.rs:279` | ✓ | yes |
| WHATWG DOM §4.4 Interface Node | *clone a node* (default document), *clone a single node* 2.4 / 4 | copy created given document | `clone_subtree` `crates/core/elidex-ecs/src/dom/tree_clone.rs:186`, `clone_node_shallow` `:251`, `crates/dom/elidex-dom-api/src/node_methods/clone.rs:227` (D-7 (d), D-11) | ✓ | yes |
| WHATWG DOM §4.4 Interface Node | *string replace all* 2 | text given parent's node document | `crates/dom/elidex-dom-api/src/node_methods/text_content.rs:119` (D-11) | ✓ | yes |
| WHATWG DOM §4.9 Interface Element | *attach a shadow root* 5; `insertAdjacentText` 1; *create an element* | shadow root / text given element's node document | `attach_shadow_with_init` `crates/core/elidex-ecs/src/dom/shadow.rs:171`; `crates/dom/elidex-dom-api/src/element/tree.rs:431` (D-11) | ✓ | yes |
| WHATWG DOM §4.11 Interface Text | *split a Text node* 5 | new node given node's node document | `split_text_at_offset` `crates/dom/elidex-dom-api/src/char_data/split_text.rs:148` (D-11; Range A5) | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *shadow-including tree order* | the walk's and the consumer's order | `for_each_uncapped_shadow_inclusive` `crates/core/elidex-ecs/src/dom/tree/walkers.rs:99` (unchanged); D-6 consumer order | ✓ | no |
| WHATWG DOM §4.3.2 Queuing a mutation record | records of adopt's removal and of replace | source record precedes destination; siblings per current replace order | `source_removal_record` `mutation/mod.rs:341`; `apply_replace_child` (D-12); `apply_adopt_node` (D-13) | ✓ | yes |
| WHATWG DOM §4.2.2 Shadow tree | *connected* | shadow-including root is a document | `EcsDom::is_connected` `walkers.rs:143` (unchanged; VM getter D-16, §7) | ✗ (VM getter) | no |
| WHATWG HTML §4.12.3 The template element | *adopting steps* 1–2 | contents into the appropriate template contents owner document (C2: the template's) | walk's contents edge + segment pairs (D-3 (c)); `attach_template_contents` `mod.rs:557` second argument (D-10 hook) | ✗ (C1) | yes |
| WHATWG HTML §4.13.6 Custom element reactions | *enqueue a custom element callback reaction* | `adoptedCallback` with « oldDocument, document » | consumer arm + flush (D-6) | ✓ | yes |
| WHATWG HTML §13.2.6.1 Creating and inserting nodes | *create an element for a token* 3; *insert a comment* 4; *insert a character* | intendedParent's / targetParent's node document | strict `crates/dom/elidex-html-parser-strict/src/tree_builder/insert.rs:61`, `:111`, `:133`, `:141`, `:155`, `:210`, `shadow.rs:153`; tolerant `crates/dom/elidex-html-parser/src/convert.rs:251`, `:305`, `:309`, `:316` (D-10, D-11) | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | throwaway document; returned nodes | nodes created in the throwaway, adopted by the caller's insertion (elidex: substitution + early re-home) | `begin_detached_fragment` `teardown.rs:293`, `finish_detached_fragment` `:330` (D-8, D-10) | ✓ | yes |
| WHATWG HTML §8.5.1 The DOMParser interface | *parse HTML from a string* | the new document owns its nodes | `parse_into_inert_document` `crates/dom/elidex-form/src/inert_document.rs:110` (D-11 at `:120`) | ✓ | yes |
| WHATWG HTML §4.9.1 The table element | *create a table element*; `createTHead` 2 (and kin) | element given tableElement's node document | `create_thead` … `crates/dom/elidex-dom-api/src/element/table_mutation/mod.rs:112` → `create_table_element` (NEW) (D-11) | ✓ | yes |
| WHATWG HTML §2.4.3 Document base URLs | *document base URL* | per node document | `BaseUrlMaintainer` `crates/dom/elidex-dom-api/src/element/document_base.rs:173` (D-7 (b)); `href_accessor.rs:50`, `document_props.rs:75` (D-7 (c)) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *getting the current value of the event handler* 3.2 (N1); *determine the target of an event handler* 3 (T3, after C0) | node document of the element | clause (b) `crates/script/elidex-script-session/src/scripting.rs:97` (D-7 (a)); `resolve_handler_target` `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:249` (unchanged, H-14) | ✓ | yes |

### §3.1 User-input touch audit

Rows marked *yes* carry author markup or script-built trees. C0 parses nothing new and stores no string; the `Adopt`
event carries entities only. What changes is which Document an author node is attributed to at creation and after a
move, and with it four reads: `ownerDocument` (and the Document methods reached through it), URL resolution through
the document base URL, N1's compile gate, and the custom-element reaction arguments. Two new script entry points run
or reach author code: `adoptedCallback` (through the existing reaction flush, same error routing as
`connectedCallback`, `flush.rs:144-`) and `document.adoptNode` (marshalling into D-13's session path). No path
attributes author content to a document it was not created in or inserted into.

## §4. Mechanism

### §4.1 Home / kind

- `node_document`, `node_document_for_new_nodes`, `adopt`, `adopt_subtree`, `MutationEvent::Adopt`, `AdoptedContents`
  and `FragmentParseDocument` live in **`elidex-ecs`** (engine-independent; every crate that creates or links nodes
  depends on it). The session's fragment arm, `apply_adopt_node` (D-13) and the custom-element consumer (D-6) are
  engine-independent crates; the dom-api `AdoptNode` handler is engine-independent. The VM changes are the flush
  arm's callback invocation and `adoptNode`'s native (marshalling), within the Layering mandate.
- One component is added: `FragmentParseDocument` (transient, on the throwaway only; D-10). The event is an occurrence,
  so it is a `MutationEvent` variant, not a marker.

### §4.2 Reconcile — HEAD → C0

| site | HEAD | C0 |
|---|---|---|
| `owner_document` `mod.rs:817-856` | stamp, else tree-root Document, else fragment root's stamp | `None` for a Document, else `node_document` |
| getter `core.rs:279` | `owner_document(this).or_else(document_root)` | Document → null, else `owner_document(this)` |
| `append_child` / `insert_before` `mutation.rs:40`, `:114` | `detach_with_hook` then link | removal, walk + `Adopt` (D-4), then link |
| `replace_child` `mutation.rs:164-197` | detach `new_child`, detach `old_child`, link | removal of `new_child`, its walk + `Adopt`, `old_child`'s removal, link |
| `adopt_subtree` `teardown.rs:251-259` | walk + stamp | moved to `mutation.rs`; + attrs, + contents segments |
| `begin_detached_fragment` `teardown.rs:293` | no context | takes `context`; root via the entry; inserts `FragmentParseDocument` |
| strict `node_document_for_new_nodes` `tree_builder/mod.rs:271` | `pub(super)` method | deleted; callers call `EcsDom::node_document_for_new_nodes(intended_parent)` (NEW) |
| tolerant `lib.rs:158`, `convert.rs` | `owner_doc` threaded to contents only | creations call the entry with the intended parent; the threaded parameter is removed (Q-2) |
| session fragment replace `mutation/mod.rs:713` | `remove_child(old_child)` first | `adopt(fragment, node_document(parent))`, then `remove_child(old_child)` |
| replace source siblings `mutation/mod.rs:761-785` | step over `old_child` | read with `old_child` in place (D-12) |
| clause (b) `scripting.rs:106-124` | Document arm + composed-root proxy | one arm over `node_document` |
| `document_base.rs:173-178` | local `owner_doc` | deleted; callers use `node_document` |
| `href_accessor.rs:50`, `document_props.rs:75` | `owner_document(x).unwrap_or(x)` | `node_document(x)` |
| CE consumer `consumer.rs:75-96` | no `Adopt` arm | `Adopt` arm (D-6) |
| CE flush `flush.rs:133-138` | no-op | invoke `adoptedCallback(old, new)` |
| creation sites (§4.4) | ownerless or `owner_document(x)` | `node_document_for_new_nodes(source)` |

### §4.3 Cost

Stamping at creation (D-11) replaces the archetype move rev 1 expected at each parser append: the parser spawns with
`AssociatedDocument` in the spawn bundle where the creation function allows it (`create_*_with_owner` today spawns then
`insert_one`s; the PR may fold the component into the spawn tuple), and the link-time adopt is a no-op compare
(`node_document(parent) == node_document(child)`). The entry per creation is one component read and one marker read.
Measured in the PR (§9 item 7): `mise run bench` CSS / style / layout parse benches before and after; a regression
beyond noise goes to the PM with the profile, the fallbacks are not restored.

### §4.4 Creation census (D-11; production sites at `27b50dfe`, unchanged from `593aa314`)

Command (§5.1 P-8) lists the calls; inline `#[cfg(test)]` modules and test-only helpers are excluded by hand (e.g.
`crates/dom/elidex-a11y/src/names.rs:146-`, `crates/dom/elidex-dom-api/src/traversal.rs:1231-`, `test_util.rs`).

| site | creates | owner today | C0 `source` |
|---|---|---|---|
| strict `insert.rs:61` (`insert_element_named`), `:111` (foreign), `:133` / `:141` (comments), `:210` (text) | parser nodes | `None` / strict entry (`:108`) | intended / target parent (§13.2.6.1) |
| strict `insert.rs:155` | doctype | none (no parameter) | the parse document |
| strict `shadow.rs:124` (contents), `:153` (DSD fallback template) | contents fragment; template | strict entry; `None` | the template (C1 hook, D-10); intended parent |
| tolerant `convert.rs:251` / `:305` / `:309` / `:316` / `:284` | elements, text, comments, doctype; contents | `None`; threaded `owner_doc` | the converted parent (Q-2); the template |
| `teardown.rs:302` | fragment root `<html>` | `None` | the throwaway (D-10) |
| `shadow.rs:189` | shadow root | none | the host |
| `create_attribute` callers `char_data/attr.rs:35`, `:90` | Attr | none | `this` (Document); the element |
| `child_node/mod.rs:302`, `:328`, `:364` | `append('s')` texts; convert fragment | `None` | this's node document's holder (`this`) |
| `element/tree.rs:431` | `insertAdjacentText` text | `owner_document(this)` | `this` |
| `node_methods/text_content.rs:119` | `textContent` text | `owner_document(this)` | `this` (the parent) |
| `char_data/split_text.rs:148` | split tail (Range A5) | `owner_document(entity)` | the split node |
| `element/select.rs:326-329` | length-setter fragment and options | `owner_document(select)` | the select |
| `cssom_sheet.rs:213-214` | `<style>` text | `owner_document(sheet_entity)` | the sheet's element |
| `element/table_mutation/mod.rs:116`, `:127`, `:138`, `:148`, `:355`, `:359`, `:448`, `:512` | thead, tfoot, caption, tbody, tr, tbody, tr, td | `None` | the table / section / row via `create_table_element` (NEW) |
| `document.rs:235`, `:291`; `char_data/document_props.rs:337`, `:353`, `:377`, `:399` | createElement / createTextNode / title / fragment / comment | `Some(this)` | `this` (value unchanged) |
| `elidex-custom-elements/src/entity_spawn.rs:57` | CE constructor element | caller's owner | the caller's document |
| `elidex-form/src/inert_document.rs:120` | DOMParser `<html>` | `Some(doc)` | `doc` (value unchanged) |
| `elidex-form/src/submit.rs:526` | form reset text | `None` | the element |
| `range/mutation.rs:266`, `:303`, `:354`, `:392` (A1–A4) | extract fragment, Text clones | `None` | §4.5 |
| `tree_clone.rs:251` (`clone_node_shallow`) | shallow clone | none | the operation's document parameter (P-2) |
| `elidex-style/src/pseudo.rs:63` | pseudo-element text | `None` | the originating element |
| `elidex-layout-table/src/grid.rs:422`, `:467`; `elidex-layout/src/layout/anonymous_table.rs:159` | anonymous table wrappers | `None` | the wrapped parent |

Not creation-owner decisions: `create_document_node` / `create_document_root` (a Document is its own); the clone deep
path (`clone_subtree`, a document parameter).

### §4.5 L3's Range sites A1–A12 (L3 memo Appendix A, re-measured at `593aa314`; files unchanged at `27b50dfe`)

| id | site (at `27b50dfe`) | today | after C0 | changed by |
|---|---|---|---|---|
| A1 | `range/mutation.rs:266` extract fragment | ownerless | `node_document_for_new_nodes(range start node)` (*extract* 1) | **C0** if the site exists at C0's base; else gone (RC-2 replaced it by A6) |
| A2–A4 | `:303`, `:354`, `:392` Text clones | ownerless | entry with the cloned Text as `source` (*extract* 4.1 "a clone of": default document = its node document) | **C0** if present at C0's base; else gone (A7) |
| A5 | `:505` → `split_text.rs:148-149` | `create_text_with_owner(tail, owner_document(text))` | entry(text) | **C0**. Result change: none for a stamped Text; for a Text whose stamp was missing the fallback gave its tree root's Document (connected) or a fragment root's stamp; after C0 every Text is stamped at creation and at link, so the value is the Text's adopted node document — equal for connected Texts, the spec's for a Text detached after a cross-document move (where HEAD gave `None` → the tail read `document_root()`) |
| A6 | RC-2 contents fragment (planned) | (planned) today's call | entry(start node) | **L3 RC-2** if RC-2 lands after C0 (it must call the entry, L3 C3 "no new ownerless site"); **C0** if RC-2 lands first (C0 migrates the landed site) |
| A7 | RC-2 CharacterData shallow clones (planned) | (planned) `clone_node_shallow` → none | stamped by the shallow cloner (D-11) | **C0** (the cloner), either order |
| A8 | RC-2 partially-contained child shallow clone (planned) | (planned) none on the clone; shadow root, host and shadow children stamped (P-2) | stamped by the shallow cloner | **C0** (the cloner), either order |
| A9 | RC-2 contained child deep clone (planned) | `clone_subtree` stamps `owner_document(src)`; ownerless if src is | `node_document(src)`; ownerless only if src is, which after C0 is not script-reachable | **C0** (the reader, D-7 (d)), either order |
| A10 | template clone | `attach_template_contents(dst, clone_doc)` | unchanged at C0 | C1 (D-C6) |
| A11 / A12 | RC-4 `body` context element, result fragment (planned) | — | entry(start node) | L3 RC-4 (C1 memo §4.5: inside contents the entry returns I) |

### §4.6 What C0 provides to C1 (requirements C0-1..C0-4: C1 rev 1 §4.6; the C1 memo's §4.7 "What C1 takes from C0" reads this table; C1 memo D-C4, D-C5, D-C10)

| C1 requirement | C0's answer |
|---|---|
| **C0-1** every path *insert* covers adopts each inserted node through the adopt function | D-4 at the three primitives (H-2); mixins, `innerHTML` / `outerHTML` / `setHTMLUnsafe` / `insertAdjacentHTML` placement (scout Q-S5: `html_fragment.rs` `apply_set_inner_html`, `apply_set_outer_html`, `apply_insert_adjacent_html` all reach the primitives) and Range reach them; `adopt_subtree` is the one walk C1's D-C5 edits |
| **C0-2** the parent's node document of a contents fragment is the fragment's | `node_document(fragment)` = the fragment's stamp (D-1); a node appended into contents is adopted into it (U1 contents variant) |
| **C0-3** adoption before the node's `MutationEvent::Insert` | §2.0 / D-4; U4 |
| **C0-4** the proxy's collapse and T3 / N4 | collapsed (D-7 (a)); T3 reads `node_document` (§8 H-T); N4's function is `node_document` (D-15) |
| D-C4 the new-node entry (C1 O-2) | C0 adds it and migrates every non-template site (D-10, D-11); C1 changes `attach_template_contents` only; the entry needs no template branch |
| D-C5 contents target and `adoptedCallback` pairs (C1 Q-3) | the walk's contents edge is one place (D-3 (c)); each segment carries its own (old, new) (D-6) — C1 sets `new` = `f(document)` there; the consumer already uses the segment's pair, in spec order (C0's T18 / U13); C1 changes no consumer (C1 memo §4.7, T25 pins values only) |
| D-C10 no second reader | `owner_document` has two production callers at C0's head (D-1 (b)); C1's grep DoD holds |

### §4.7 ECS-native and Layering checks

- One fact (the component), one creation entry, one later writer (the adopt function), one reader (`node_document`),
  one notification (the event). No side-store, no observer registry; `adoptedCallback` is a consumer arm on the
  mutation stream, as `connectedCallback` already is. The fragment substitution is a component on the entity it
  describes.
- No algorithm in `vm/host/`: the VM marshals the callback's arguments and `adoptNode`'s argument / errors.

### §4.8 Docstring citation table (Axis 4)

| symbol | citation |
|---|---|
| `node_document` (NEW) | DOM §4.4 *node document* |
| `owner_document` (doc rewrite) | DOM §4.4 `ownerDocument` getter |
| `node_document_for_new_nodes` (NEW home) | DOM §4.4 *node document* ("set upon creation"); HTML §13.2.6.1 *create an element for a token* step 3; HTML §13.4 (the substitution; the overwrite note) |
| `FragmentParseDocument` (NEW) | HTML §13.4 (throwaway document) |
| `adopt` (NEW) | DOM §4.5 *adopt* steps 1–3 (3.2, 3.3.2 not modelled, named) |
| `adopt_subtree` (doc rewrite) | DOM §4.5 *adopt* 3.1, 3.3.1, 3.4; HTML §4.12.3 template *adopting steps* (C2: identity target) |
| `append_child` / `insert_before` (doc) | DOM §4.2.3 *insert* step 7.1 |
| `replace_child` (doc), `apply_replace_child` (doc) | DOM §4.2.3 *replace* steps 2–10 |
| `MutationEvent::Adopt`, `AdoptedContents` (NEW) | DOM §4.5 *adopt* 3.3.3 and the *adopting steps*; HTML §4.13.6 |
| `AdoptNode`, `apply_adopt_node` (NEW) | DOM §4.5 `adoptNode(node)` steps 1–4 |
| `create_table_element` (NEW) | HTML §4.9.1 *create a table element* |
| `split_text_at_offset` (doc fix) | DOM §4.11 *split a Text node* step 5 (P-11) |
| clause (b) docstring (rewrite) | HTML §8.1.8.1 step 3.2 node clause; DOM §4.4 *node document* |

### §4.9 Comment sites made true or false by C0

`/usr/bin/grep -rn 'cross-document-adopt-on-insert' crates` → `scripting.rs:65`, `:83` (the slot paragraph, rewritten,
D-7 (a)) and N1's landed cell comment `crates/script/elidex-js/src/vm/tests/tests_event_handler_scripting_gates.rs:149-153`
(C31: "elidex's insertion does not adopt"; false after C0, the cell's outcome unchanged) — with C8b (iii)'s "`append_child`
does not adopt" (`:192-195`, no slot name; same disposition); P-8's two
comments (become true; step numbers checked against §3); `owner_document`'s docstring `mod.rs:802-816` (P-10);
`split_text.rs:1-2`, `:140-147` (P-11, and "legacy entities relying on the tree-root Document fallback");
`adopt_subtree`'s docstring (`teardown.rs:245-250`); `begin_detached_fragment`'s "nodes the parse creates without an
explicit owner … resolve `owner_document` by walking up to this throwaway Document" (`teardown.rs:280-284`, false
after D-10); `finish_detached_fragment`'s "orphan-detached (no `AssociatedDocument`)" (`teardown.rs:318-322`: the
returned nodes carry the dangling throwaway stamp, read as `None`); `attach_template_contents`'s caller list
(`mod.rs:551-554`); `clone.rs:290-299` ("The shallow cloner never stamps"); `apply_replace_child`'s step numbers
(P-9); the custom-element flush's "v1 single-document VM never enqueues Adopted" (`flush.rs:134-136`); the getter's
fallback comment (`core.rs:273-278`).

## §5. Populations, touched files, named risks

### §5.1 Populations (re-runnable)

- **P-1 linking sites** (H-2): `/usr/bin/grep -rn 'link_node(' crates --include='*.rs'` → definition + 3 calls;
  `/usr/bin/grep -rn 'rel.parent = ' crates --include='*.rs'` → 2. Any new hit is a linking path D-4 does not cover.
- **P-2 adopt callers**: `/usr/bin/grep -rnF 'adopt_subtree(' crates --include='*.rs'` → at `27b50dfe` def + 1 prod +
  1 test (unchanged from `593aa314`); at C0's head def + `adopt` + `finish_detached_fragment` + tests. `/usr/bin/grep -rnF '.adopt(' crates
  --include='*.rs'` → the primitives' shared path, the session fragment arm, `apply_adopt_node`, tests.
- **P-3 node-document readers**: `/usr/bin/grep -rn 'owner_document(' crates --include='*.rs' | /usr/bin/grep -v
  '/tests/\|tests\.rs\|/tests_\|test_helpers\|///' | /usr/bin/grep -v 'fn owner_document\|validate_owner_document'` → at
  `27b50dfe` (re-run 2026-11-03; 21 lines, as at `593aa314`): `teardown.rs:336`, `tree_clone.rs:208`, `scripting.rs:120`, `cssom_sheet.rs:213`,
  `href_accessor.rs:50`, `element/tree.rs:431`, `select.rs:326`, `document_base.rs:177`, `document_props.rs:75`,
  `split_text.rs:148`, `focus/delegate.rs:85`, `node_methods/core.rs:279`, `text_content.rs:119`, `node_methods/clone.rs:242`,
  `selection.rs:618`, `elidex-html-parser/src/lib.rs:158`, `elidex-html-parser-strict/src/tree_builder/mod.rs:273`
  (plus the test fn name `range/live.rs:900` and three non-reader lines: the VM getter's fn name
  `vm/host/node_methods_extras.rs:41` and the `//` comments `document_base.rs:357`, `node_methods/clone.rs:418`). **At C0's head: `core.rs:279` and `selection.rs:618` only** (D-1 (b)).
- **P-4 position-based proxies**: `/usr/bin/grep -rn 'find_tree_root_composed' crates --include='*.rs' | /usr/bin/grep
  -v '/tests/\|tests\.rs\|/tests_'` → `scripting.rs:115` (deleted, D-7 (a)), `node_proto.rs:451`, `:455` (`isConnected`,
  D-16), the definition `walkers.rs:342`. `/usr/bin/grep -rn 'document_root())' crates/dom/elidex-dom-api/src/node_methods/core.rs`
  → the getter fallback (deleted, D-1 (c)).
- **P-5 "Document → itself" idioms**: `/usr/bin/grep -rn 'owner_document(.*)\.unwrap_or(' crates --include='*.rs'` → 2;
  `/usr/bin/grep -rn 'fn owner_doc\b' crates --include='*.rs'` → 1. At C0's head: 0 and 0.
- **P-6 `MutationEvent` consumers**: `/usr/bin/grep -rln 'MutationEvent::Insert' crates --include='*.rs' |
  /usr/bin/grep -v '/tests/\|tests\.rs\|/tests_'` → 11 files (verified 2026-11-01; 11 at `27b50dfe`); one exhaustive production match
  (`range/live.rs:113-163`) and one test match (`crates/core/elidex-ecs/src/dom/tests/mutation_hook.rs:68-148`) gain an
  arm; the others have `_ => {}` (scout Q-S1). Only the custom-element consumer acts on `Adopt`.
- **P-7 Document-destroying paths** (stale stamps): `finish_detached_fragment` (`teardown.rs:341`), the strict error
  path (`despawn_subtree(document)`, `tree_builder/mod.rs:234`); DOMParser documents are never destroyed (H-18).
- **P-8 creation census** (D-11): `git grep -n -E '\.create_(element|text|comment|document_fragment)\(' -- 'crates/*.rs'
  | /usr/bin/grep -v -E '/tests/|tests\.rs|/tests_|test_helpers|test_util|/benches/'`, minus inline test modules
  (by hand). At `27b50dfe` the production rows are §4.4's ownerless rows (431 raw lines, 429 at `593aa314`; the two
  added are inline-test lines of `event_handler_reconcile.rs`, §0.1). **At C0's head: 0 production hits**, and
  `git grep -n 'create_\(attribute\|document_type\)(' -- 'crates/*.rs'` production hits all pass an owner.

### §5.2 Existing tests whose expectation changes

Known at `27b50dfe`: `crates/core/elidex-ecs/src/dom/tests/associated_document.rs` —
`owner_document_resolves_through_detached_fragment_root` (`:31`), `owner_document_fallback_without_component` (`:64`),
`owner_document_skips_destroyed_associated_document` (`:172`) assert the deleted fallbacks (rewritten to the stamp
contract); `crates/dom/elidex-dom-api/src/node_methods/tests/clone.rs` (9 `owner_document(` calls; the shallow-clone
cases); `crates/dom/elidex-html-parser/src/tests.rs` (6) and `crates/dom/elidex-html-parser-strict/src/tree_builder/tests_fragment.rs`
(2) (fragment stamps); the `scripting.rs` inline tests (two-document fixture); the N1 cells that pin the proxy facets
(N1 memo C8 family, landed by #580 in `crates/script/elidex-js/src/vm/tests/tests_event_handler_scripting_gates.rs`:
its cells C8, C8b–C8e, C8g, C8h, C30, C31); the base-URL maintainer tests in `document_base.rs`;
`crates/script/elidex-js/src/vm/tests/tests_associated_document.rs`. The complete list is produced by a
pre-implementation step (§9 item 1): delete the fallbacks on C0's base with nothing else changed, run §6.4's commands,
record every red test with its reason.

### §5.3 Touched files

| file | lines at `27b50dfe` | change | 1000-line decision |
|---|---|---|---|
| `crates/core/elidex-ecs/src/dom/mod.rs` | 926 | `node_document`, entry (NEW); `owner_document` body/doc; create signatures (attr, doctype) | net ≈ −5; < 1000 |
| `crates/core/elidex-ecs/src/components.rs` | 822 | `FragmentParseDocument` (NEW) | < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/mutation.rs` | 467 | `adopt` (NEW), `adopt_subtree` (moved in), primitives | ≈ 580; < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/teardown.rs` | 344 | `adopt_subtree` out; `begin_detached_fragment(context)` | < 1000 |
| `crates/core/elidex-ecs/src/dom/mutation_event.rs` | 352 | `Adopt`, `AdoptedContents` (NEW) | < 1000 |
| `crates/core/elidex-ecs/src/dom/shadow.rs`, `tree_clone.rs` | 549, 425 | shadow-root stamp; shallow clone stamp; reader | < 1000 |
| `crates/script/elidex-script-session/src/mutation/mod.rs` | 994 | **split first (D-14 (a))**; then D-5 / D-12 / D-13 land in `child_list.rs` (NEW) | prereq PR |
| `crates/script/elidex-js/src/vm/host/document.rs` | 1197 | **split first (D-14 (b))**; `adoptNode` native in `document_factories.rs` (NEW) | prereq PR |
| `crates/dom/elidex-dom-api/src/element/table_mutation/mod.rs` | 994 | `create_table_element` (NEW), the eight §4.4 rows (`git grep -n 'create_element(' -- crates/dom/elidex-dom-api/src/element/table_mutation/mod.rs` → 8, verified 2026-11-01) | re-measure at head; ≥ 1000 → D-14 (c) |
| `crates/script/elidex-script-session/src/scripting.rs` | 279 (293 at `593aa314`) | clause (b) + docstring | < 1000 |
| `crates/dom/elidex-dom-api/src/` — `element/document_base.rs` 555, `href_accessor.rs` 567, `char_data/document_props.rs` 403, `char_data/split_text.rs` 447, `char_data/attr.rs` 386, `child_node/mod.rs` 459, `element/tree.rs` 633, `element/select.rs` 358, `node_methods/text_content.rs` 181, `node_methods/core.rs` 346, `node_methods/clone.rs` 458, `cssom_sheet.rs` 873, `focus/delegate.rs` 253, `range/mutation.rs` 640, `document.rs` 665, `registry.rs` 785 | as listed | D-7, D-11, D-13 | all < 1000 |
| parsers — strict `tree_builder/mod.rs` 493, `insert.rs` 380, `shadow.rs` 218; tolerant `lib.rs` 258, `convert.rs` 678 | as listed | D-10, D-11 | < 1000 |
| `crates/dom/elidex-custom-elements/src/consumer.rs` 329, `entity_spawn.rs`; `crates/script/elidex-js/src/vm/host/custom_elements/flush.rs` 237 | as listed | D-6, D-11 | < 1000 |
| `crates/dom/elidex-form/src/inert_document.rs`, `submit.rs` 546; `crates/css/elidex-style/src/pseudo.rs` 81; `crates/layout/elidex-layout-table/src/grid.rs` 513; `crates/layout/elidex-layout/src/layout/anonymous_table.rs` 170 | as listed | D-11 (owner argument only) | < 1000 |
| `crates/dom/elidex-dom-api/src/selection.rs` | 1036 | **not touched** (D-1 (b)) | not triggered |
| `crates/core/elidex-ecs/src/dom/tests/adopt_on_insert.rs` (NEW), `crates/script/elidex-js/src/vm/tests/tests_insertion_adoption.rs` (NEW) | — | cells | new files |
| `docs/plans/2026-09-ibp-sandbox.md` | 823 | D-15 (this commit) | docs |

### §5.4 Named risks (and the observable change each is)

- **R-1 `ownerDocument` after a cross-document move** → the receiving document (spec).
- **R-2 a removed main-document parse node** → keeps its last document (spec); a parse node removed from a DOMParser
  tree now reports the DOMParser document instead of the page (T3).
- **R-3 handlers** → N1's compile gate follows the adopted document: a page node moved into a detached DOMParser
  subtree stops compiling raw handlers (more restrictive than HEAD, the spec's); a DOMParser node moved into the page
  compiles them (the spec's; NR3's facet closes).
- **R-4 base URL** → a `<base>` moved into another document updates that document's base URL (T12).
- **R-5 `adoptedCallback` and `adoptNode` run / reach author code** where HEAD ran none (spec).
- **R-6 replace with a fragment** → the fragment's `ownerDocument` becomes the parent's document (spec; T11); replace's
  source record siblings change (T17).
- **R-7 parse cost** (§4.3).
- **R-8 the getter's `document_root()` fallback deleted** → a production creation site missed by D-11 would report
  `ownerDocument === null`; guarded by P-8's ratchet, U8, U12, T19–T22.
- **R-9 P1 collision** → §2.0 is the contract; the PM tells L3.
- **R-10 `MutationEvent` census collision with L3 RC-2** → §8 edge c0e.

## §6. Test plan

Columns: *HEAD/base* = `27b50dfe` (N1 landed) and C0's base (plus the D-14 splits), equal unless stated; *C0 = spec* (under C2 for template
rows). **Red-first**: every *red* cell is run on the base before the fix with its control green, and the failure is
recorded in the PR description. (×2) = both parser tiers. `dp` = `new DOMParser().parseFromString('<div id=d><span
id=s></span></div>', 'text/html')`; `dp2` a second such document.

### §6.1 VM integration (`tests_insertion_adoption.rs`, NEW)

| cell | kind | script | HEAD/base | C0 = spec |
|---|---|---|---|---|
| T1 | red | `d = dp.getElementById('d'); s = d.firstChild; document.body.append(d); [d.ownerDocument === document, s.ownerDocument === document]` | `[false, false]` (stamped `dp`, H-4) | `[true, true]` |
| T2 | red | `x = document.createElement('div'); dp.body.append(x); x.ownerDocument === dp` | `false` | `true` |
| T3 | red (×2) | initial HTML `<p id=p></p>`: `dp.body.append(p); a = p.ownerDocument === dp; p.remove(); [a, p.ownerDocument === dp]` | `[true, false]` (tree-root fallback while connected; getter `document_root()` after removal) | `[true, true]` |
| T4 | red | `h = dp.createElement('div'); sr = h.attachShadow({mode:'open'}); sr.innerHTML = '<b></b>'; dp2.body.append(h); [sr.ownerDocument === dp2, sr.firstChild.ownerDocument === dp2]` | `[false, false]` (unstamped sr → getter `document_root()`; `b` stamped `dp`; P-6) | `[true, true]` |
| T5 | red (C2) | `dp3 = parseFromString('<template id=t><b></b></template>','text/html'); t = dp3.getElementById('t'); document.body.append(t); [t.content.ownerDocument === document, t.content.firstChild.ownerDocument === document]` | `[false, false]` | `[true, true]` under C2; C1 turns both into the inert document |
| T6 | red | `log = []; customElements.define('x-a', class extends HTMLElement { adoptedCallback(o, n) { log.push(['a', o === document, n === dp]) } connectedCallback() { log.push(['c', this.ownerDocument === dp]) } }); e = document.createElement('x-a'); dp.body.append(e); log` | `[['c', false]]` (dp's tree is connected, H-13) | `[['a', true, true], ['c', true]]` |
| T7 | red | continue T6 with a `disconnectedCallback` pushing `'d'`: `log = []; document.body.append(e); log`; variant `document.body.replaceChild(e2, old)` with `old` custom and `e2` custom in `dp` | `[d, c]` | `[d, ['a', …], c]` (adopt precedes link; in the replace variant `Adopted(e2)` precedes `Disconnected(old)`) |
| T8 | red (needs N1) | `d2 = dp4.getElementById('q')` from `'<div id=q onclick="window.hit=1"></div>'`; `x = document.createElement('div'); x.append(d2); typeof d2.onclick` | `'object'` (null: owner `dp4`) | `'function'` |
| T9 | red (needs N1) | `document.body.append(d2); d2.remove(); typeof d2.onclick` | `'object'` | `'function'` |
| T10 | red (needs N1) | `y = document.createElement('div'); y.setAttribute('onclick','window.hit=1'); h = dp.createElement('div'); h.append(y); typeof y.onclick` | `'function'` | `'object'` (null: node document `dp`) |
| T11 | red | `f = dp.createDocumentFragment(); f.append(dp.createElement('i')); c = document.createElement('u'); document.body.append(c); document.body.replaceChild(f, c); f.ownerDocument === document`; variant `document.body.append(f2)` (pre-insert) → `f2.ownerDocument === dp` | `[false, true]` | `[true, true]` |
| T12 | red | `x = document.createElement('base'); x.setAttribute('href','https://other.test/'); dp.head.append(x); a = dp.createElement('a'); a.setAttribute('href','y'); dp.body.append(a); a.href` | not `'https://other.test/y'` (the recompute goes to the page document, whose tree lacks `x`; `dp`'s base is not recomputed, P-7; exact value recorded red-first, predicted the unresolved `'y'`) | `'https://other.test/y'` |
| T13 | guard (R-B, PM 153) | T3 memo T-18 shape: `p = document.createElement('div'); dd = parseFromString('<body></body>','text/html'); b = dd.body; p.append(b); b.ownerDocument === document`; X2: `b2 = document.createElement('body'); dd.documentElement.append(b2); b2.remove(); b2.ownerDocument === dd` | `false`, `false` | `true`, `true` — the node-document facts under which T3's step 3 gives XW-9 / XW-10 / EW-15 / EW-16's spec outcomes (§9 item 10) |
| T14 | guard (K-0) | same-document move: `q = document.createElement('x-a'); document.body.append(q); log = []; document.body.prepend(q); log` | `[d, c]` | `[d, c]` (no `Adopted`) |
| T15 | red | `n = dp.getElementById('d'); r = document.adoptNode(n); [r === n, n.ownerDocument === document, n.parentNode]`; errors: `document.adoptNode(dp)` → `NotSupportedError`; `document.adoptNode(h.shadowRoot)` → `HierarchyRequestError`; custom `x-a` adopted → `adoptedCallback(dp, document)` | `TypeError` (no `adoptNode`, H-15) | `[true, true, null]`; the two DOMExceptions; the callback |
| T16 | red | `MutationObserver` on `dp.body`'s child `d` parent: `document.adoptNode(d)` | no record (`TypeError`) | one record: `removedNodes = [d]`, target `d`'s old parent, siblings pre-removal |
| T17 | red (D-12) | `P = document.createElement('div'); A = …; B = …; P.append(A, B); mo.observe(P, {childList:true}); P.replaceChild(B, A); takeRecords()` | first record `previousSibling === null` | first record (adopt's removal of `B`) `previousSibling === A`, `nextSibling === null`; second record `addedNodes [B]`, `removedNodes [A]`, siblings `null`, `null` |
| T18 | red (per-node pairs; P-4) | `t = dp.createElement('template'); dp.body.append(t); e = document.createElement('x-a'); t.content.append(e); document.adoptNode(t.content); log = []; dp2.body.append(t); log` | `TypeError` (no `adoptNode`) | `e`'s callback has old = `document` (its contents' node document), new = `dp2` (C2), not the root pair (`dp`, `dp2`); C1 turns `new` into `dp2`'s inert document |
| T19 | red (D-11) | `a = dp.createAttribute('x'); el = dp.createElement('p'); el.setAttribute('id','q'); at = el.getAttributeNode('id'); [a.ownerDocument === dp, at.ownerDocument === dp]`; then `document.body.append(el); at.ownerDocument === document` | `[false, false]`, `false` (getter `document_root()`) | `[true, true]`, `true` (adopt 3.3.1) |
| T20 | red (D-11) | `dp.body.cloneNode(false).ownerDocument === dp` | `false` | `true` |
| T21 | red (D-11) | `t = dp.createElement('table'); [t.createTHead(), t.insertRow(), t.createCaption()].map(n => n.ownerDocument === dp)` | `[false, false, false]` | `[true, true, true]` |
| T22 | red (D-11) | `x = dp.createElement('div'); x.append('t', dp.createElement('i')); [x.firstChild.ownerDocument === dp]`; `x.insertAdjacentText('beforeend','u')` | `[false]` (ownerless text under a detached parent → getter `document_root()`) | `[true]` |

### §6.2 Unit (`crates/core/elidex-ecs/src/dom/tests/adopt_on_insert.rs`, NEW, unless stated)

| cell | kind | shape | expected |
|---|---|---|---|
| U1 | red | `append_child(D_parent, n)` with `n` stamped `X`, a child, a shadow root with content, a template with contents, a cached Attr; variants `insert_before`, `replace_child`, an append into a contents fragment stamped `D` | every visited node and the Attr stamped `D` |
| U2 | red | ownerless `n` appended under `D` | stamped `D`; recorder sees no `Adopt` |
| U3 | red | `n` stamped `D` appended under an ownerless parent | keeps `D`; no `Adopt` |
| U4 | red | event recorder (a `MutationDispatcher` double) over `append_child` of an `X`-stamped parented node, and over `replace_child` | `Remove(old parent)`, `Adopt`, `Insert`; replace: `Remove(new_child's old parent)`, `Adopt`, `Remove(old_child)`, `Insert` |
| U5 | guard | same-document move | no `Adopt`, stamps unchanged |
| U6 | guard | adoption past the depth and breadth caps (`destroy.rs:381`'s shape) | every node re-homed |
| U7 | red | `node_document(D) == Some(D)`; `owner_document(D) == None`; a node whose stamp's document was destroyed → `None`; an unstamped node in a Document's tree → `None` (no fallback) | as stated |
| U8 | guard (×2) | parse a document with templates, a declarative shadow root, foreign content and a doctype; every node except the Document carries `AssociatedDocument == D` **before** any later append (creation stamp, D-11) | all stamped |
| U9 | guard (×2) | fragment parse into a context in `D` with a dispatcher installed around it | no `Adopt` reaches the dispatcher; returned nodes stamped `D`; `FragmentParseDocument` absent afterwards (success and strict rollback) |
| U10 | red | `EcsDom::adopt(fragment, D)` on a parentless fragment with children | fragment and children stamped `D`, one `Adopt` |
| U11 | red | `node_document_for_new_nodes`: a Document → itself; a node → its stamp; in a fragment build a throwaway-parented source → the context's document; documentless context → `None`; root `<html>` created in the throwaway | as stated |
| U12 | guard | P-8's census ratchet as a test: a DOMParser document's tree after `createAttribute`, `cloneNode(false)`, table helpers, `append('s')`, `splitText`, `textContent` — every created node stamped | all stamped |
| U13 | red (order) | consumer arm (`elidex-custom-elements`): a template with a custom light child `L` and a custom element `C` in contents, adopted | reactions `C` then `L` (contents right after the template); `C` with its segment's pair |
| U14 | red (D-12, session) | `apply_replace_child` with `new_child` adjacent to `old_child`, both orders | source record siblings include `old_child` |
| U15 | red (D-1 (c), dom-api) | `ownerDocument.get` on an ownerless test-fixture element (created by the shim, never linked) with a bound document present | `JsValue::Null` (HEAD: the bound document) |

### §6.3 Mutation guards

| mutation | killed by |
|---|---|
| no adoption in the primitives (HEAD) | T1, T2, U1 |
| adopt after `link_node` | U4, T7 |
| light-tree-only walk | T4, U1 (shadow variant) |
| attrs not re-homed | T19, U1 |
| `Adopt` fired for an ownerless root | U2 |
| `Adopt` fired on a same-document move | T14, U5 |
| fallbacks kept (dead code) | U7 |
| getter fallback kept | U15 |
| a creation site left ownerless | U8, U12, T19–T22, P-8 |
| entry without the substitution | U11, U9 (documentless-context contents) |
| replace-with-fragment does not adopt the fragment / pre-insert adopts it | T11, U10 |
| CE arm missing, `Adopted` after `Connected`, or root pair used for contents | T6, T7, T18, U13 |
| contents visited after the template's subtree in the consumer | U13 |
| clause (b) keeps the proxy | T9, T10 |
| replace siblings still step over `old_child` | T17, U14 |
| `adoptNode` without the removal record | T16 |

### §6.4 Kept green

`crates/core/elidex-ecs/src/dom/tests/destroy.rs` (fragment teardown, #568), `associated_document.rs` (rewritten per
§5.2), the #380 template suite, #577's fragment-template cells, slice a's E1–E8 and U8, N1's cells not listed in §5.2.
Commands: `cargo test -p elidex-ecs --all-features`, `-p elidex-dom-api`, `-p elidex-custom-elements`, `-p
elidex-html-parser`, `-p elidex-html-parser-strict`, `-p elidex-form`, `-p elidex-script-session`, `-p elidex-js`, `-p
elidex-style`, `-p elidex-layout-table`, `-p elidex-layout`, then `mise run ci`.

## §7. Out of scope and slots

| item | where | defect (stated, not the remedy) |
|---|---|---|
| template contents' owner document; the adopting steps' target | C1 (`#11-template-contents-owner-document`) | under C2 the contents' node document is the template's (D-3 (c)) |
| Range planned sites A6, A10–A12 | L3 RC-2 / RC-4, C1 | §4.5 |
| adopt steps 3.2, 3.3.2 (registries) | `#11-shadow-scoped-custom-element-registry` family (scout Q-S3: not modelled) | an element's / shadow root's registry is not re-pointed at adoption |
| `importNode` | proposed slot (§8 R-6 (b)) | absent (H-15) |
| `replaceChild(X, X)` | proposed slot (§8 R-6 (e)) | `EcsDom::replace_child` and the record layer reject self-replace (`mutation.rs:139`, `mutation/mod.rs:757-760` "browser-parity no-op"); *replace* step 6 removes `node` (= `child`), so step 7 finds no parent and step 9 re-inserts it — two records (removal, insertion). Before any change, measure Chromium's records in the built-in browser (the parity claim is unmeasured here) |
| Selection API document-membership check | `#11-selection-document-membership` (L3 RC-1's file) | P-13; step text unverified (webref) |
| VM `isConnected` vs bound document | `#11-vm-isconnected-bound-document` (owner L3, D-16) | P-12 |
| HTML `img` adopting steps (relevant mutations) | not tracked (`/usr/bin/grep -rln 'relevant mutation\|update_image_data' crates` → no hit) | an `img` adopted into another document does not re-run image selection; `MutationEvent::Adopt` is where it would hook |

Own deferrals: **0**. Every row is pre-existing (A96 class) and not changed by C0, except that C0 adds the hook the
last row names.

## §8. Edges, hand-offs and records

**Edges.**

| # | edge | kind | note |
|---|---|---|---|
| c0-pre | D-14 (a), (b) [, (c)] → C0 | code predecessor | pure moves, each its own PR / commit |
| c0a | N1 implementation (#580, landed `27b50dfe`) → C0 | code predecessor + semantic | clause (b) is edited on N1's landed body; T8–T10 need N1's compile gate |
| c0b | C0 → C1 | semantic precondition (C1 memo §1) + code predecessor (the entry, the census, the walk's contents edge, the per-segment pairs; C1 memo edge c3) | C1 edits `attach_template_contents`, the walk's contents edge (D-3 (c)) and each segment's `new` |
| c0c | C0 → T3 | data flow | T3 reads `node_document` (H-T) |
| c0d | C0 → L3 P1 | boundary | §2.0 |
| c0e | C0 ↔ L3 RC-2 | census collision | **L3 RC-2's `CloneCreated` and C0's `MutationEvent::Adopt` touch the same enum, the same consumer dispatch and the same flush; whichever lands second re-adds the census (P-6's command and the exhaustive-match arms) and re-arms its cells.** |
| c0f | C0 ↔ L3 RC-2 / RC-4 | creation entry | RC-2 / RC-4 creation sites call `node_document_for_new_nodes` if they land after C0; C0 migrates them if they land first (§4.5) |

**Hand-offs.**
- **H-C1 to C1** — delivered: C1 rev 3 (`cab354ba`, its §10.0) applies each item. §4.6's table; C1's D-C4 shrinks to
  the entry's value inside contents (I), supplied through `attach_template_contents`'s second argument and D-C3's `f`
  — C1 adds no entry, no entry branch and no creation-site change (P-5; the parser tiers' sites are migrated by C0);
  C1's Q-1 (signature) is answered by D-10 (one `source` parameter, substitution by component); C1's Q-3 by D-6 (one
  event, per-segment pairs, spec order; the consumer is C0's, cells T18 / U13, and C1's T25 pins values only); C1's
  O-2 is decided (a) by PM 223 O-2. C1's R-1 adds only the C1 row (D-9). C1's coordinates that cited C0 draft rev 1
  (`fade8a19`) are on rev-2 IDs. **Received from C1** (C1 memo H-2): §4.3's constraint on Q-2 (§11).
- **H-T to T3** (rev 4, lands after C1): D-T4 (ii) holds — step 3 calls `EcsDom::node_document`. R-B closed by C0:
  XW-9 → the Window, XW-10 → nothing, EW-15 → the Window with the getter reading the attribute, EW-16 → nothing (all
  spec); pin T-18 is green from C0's head (T13 pins its node-document facts). T3 memo §8.3 item 3 (a consumer entry on
  this slot) is moot: the slot closes.
- **H-P1 to L3's P1 umbrella** (via the PM): §2.0's contract.
- **H-L3 to L3** (via the PM): §4.5's table; edges c0e, c0f; the L3 memo's line "C0 keeps this line's code (C0 memo
  §5.1 P-3 lists `core.rs:279` …)" no longer holds — C0 deletes the getter's `document_root()` fallback (D-1 (c)); the
  observable `r.extractContents().ownerDocument === document` stays green because A1 is stamped by C0 (or by RC-2
  through the entry). Slots `#11-selection-document-membership` and `#11-vm-isconnected-bound-document` are L3's.
- **H-N4 to N4**: node memo amended in place (D-15); N4 calls `node_document`, adds no function.

**Records (for the PM; exact texts).**
- **R-1 ledger `#11-cross-document-adopt-on-insert` — at C0's landing: close**: "Closed by C0 (#<PR>, `<squash>`):
  every insertion adopts (DOM §4.2.3 *insert* step 7.1, *replace* step 6) through `EcsDom::adopt` at the three linking
  primitives; `document.adoptNode` added; `AssociatedDocument` is the node document, set at creation by
  `EcsDom::node_document_for_new_nodes` and read only by `EcsDom::node_document`; the composed-tree-root proxy in N1's
  compile gate, `owner_document`'s tree-root fallbacks and the `ownerDocument` getter's `document_root()` fallback are
  deleted. The listed facets (adopt-then-remove, live node into a detached foreign subtree, their reverses) and the
  consumers (base URL, `ownerDocument`, href/baseURI, N1's compile gate, NR3) read the adopted document; selection.rs
  is not a node-document reader (`#11-selection-document-membership`)."
- **R-2 ledger / roadmap `#11-custom-elements-adopted-callback` — at C0's landing: close**: "Closed by C0 (#<PR>,
  `<squash>`): `MutationEvent::Adopt` → `CustomElementReaction::Adopted` → `adoptedCallback(oldDocument, document)`,
  each node with its own pair (template contents: the contents' pair)."
- **R-3 umbrella, in place (D-9), carried by the PR that lands this memo; revision number per D-9's rule**: §1.2 row
  "| **C0** `EH-attrs-at-creation-C0` (owner: Layout lane, this program; PM decision 143) | slot
  `#11-cross-document-adopt-on-insert`: insertion adoption (DOM §4.2.3 *insert* step 7.1, §4.5 *adopt*) and
  `document.adoptNode`; the node document is the stored fact, set at creation by one entry and read by
  `EcsDom::node_document`; closes T3's residual R-B | IBP-sandbox N1's implementation landed; the D-14 pure-move splits
  landed |"; changelog: "*Rev 8.<n>* (<date>; carried by C0's PR): §1.2 gains the C0 row (PM decisions 159, 223 O-1)."
- **R-4 node memo, in place (D-15; applied by this commit)**: see the node memo's "Amendment 2026-11-01" paragraph.
- **R-5 N1 memo**: none in place (landed); NR3's facet closes with R-1.
- **R-6 slot texts (defects only; PM registers)**: (a) `#11-ownerless-node-creation` — append at C0's landing: "C0
  (#<PR>): the `EcsDom` entry `node_document_for_new_nodes` exists and every production creation site at C0's head
  takes its owner from it (census: C0 memo §4.4, §5.1 P-8). Remaining facets: template contents (C1), L3 RC-2 / RC-4's
  planned sites (they call the entry)." (b) `#11-import-node` — "`document.importNode` is absent (TypeError); spec: DOM
  §4.5 `importNode(node, options)` clones into this." (c) `#11-selection-document-membership` — "Selection methods accept
  a detached node whose node document is the selection's document (`selection.rs:614-619`); the Selection API's check is
  unverified against its step text (webref cannot render the anchors)." (d) `#11-vm-isconnected-bound-document` (owner
  L3) — "`Node.isConnected` compares the composed tree root with the bound document (`vm/host/node_proto.rs:441-463`),
  so a node in a DOMParser document's tree reports `false`; spec: connected iff the shadow-including root is a document
  (DOM §4.2.2), which `EcsDom::is_connected` implements." (e) `#11-replace-child-self-records` — "`replaceChild(X, X)` is
  a record-free no-op (`mutation/mod.rs:757-760`); *replace* step 6 removes `node` and step 9 re-inserts it (two
  records). Chromium's behaviour is unmeasured."

## §9. Definition of done

1. Coordinates re-measured at C0's base (N1 landed, D-14 splits landed); §5.1 populations re-run, changes recorded in
   the PR; the §5.2 pre-implementation run (fallbacks deleted alone) recorded.
2. `node_document`, `node_document_for_new_nodes`, `adopt`, `MutationEvent::Adopt`, `AdoptedContents`,
   `FragmentParseDocument`, `AdoptNode`, `apply_adopt_node`, `create_table_element` exist with §4.8's citations; the
   strict tree builder's method is gone.
3. P-3 returns `core.rs:279` and `selection.rs:618` only; P-4's first command returns the definition and
   `node_proto.rs` only, its second returns 0; P-5 returns 0 and 0; P-8 returns 0 production hits.
4. D-4 to D-8 and D-10 to D-13 implemented; P-1 unchanged (no new linking path).
5. §6.1 and §6.2 cells present; every red cell run red on the base with its control green, recorded in the PR.
6. §5.2 rewrites done; §4.9's comment sites rewritten.
7. `cargo test` for §6.4's crates and `mise run ci` green; parse cost measured (§4.3) and recorded.
8. `python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md` and
   `… docs/plans/2026-09-ibp-sandbox.md` exit 0 (run; do not trust a stored verdict).
9. The umbrella §1.2 row (R-3) is in the PR with the revision number D-9's rule gives at its rebase.
10. **T3's residual R-B is closed by C0** (PM decision 153): its cause — a stale node document after a cross-document
    move followed by a detached state — no longer exists; **cell T13** pins the node-document facts of both X1 and X2
    shapes at C0's head (green), and H-T hands T3 the re-derived verdicts of XW-9, XW-10, EW-15, EW-16 and pin T-18. (At
    C0's head HEAD's `resolve_handler_target` does not read the node document, H-14, so no target cell flips here.)
11. The PM has applied R-1 to R-3 and registered or declined R-6 (b)–(e) and appended R-6 (a).

## §10. Rev-2 disposition (the Layout driver's brief, inputs 1–11)

| # | input | disposition | where |
|---|---|---|---|
| 1 | PM 157: node memo D8 / edge n3 `effective_node_document` → `node_document`, in place | applied in this commit, every non-historical occurrence (D8, F10 home/kind, form submission, F10r, composition, §3 row, §5.3, edge n3), with an amendment paragraph; preflight on the node memo run | D-15, §8 R-4, node memo |
| 2 | PM 158: `document.adoptNode` in C0 | in scope: dom-api `AdoptNode` → session `apply_adopt_node` → `EcsDom::adopt`; VM native in the split-off factory file | D-13, T15, T16 |
| 3 | PM 159 / 223 O-1: only C0's own §1.2 row; no other row; no rev number | applied; rev 1's §1.3 x7 row, order line and "rev 8.4" claims withdrawn; numbering rule stated; order line / C0 edges asked | D-9, §8 R-3, O-1 |
| 4 | PM 160: (a) replace record order in C0; (b) split `mutation/mod.rs` first, name seams | (a) D-12 (T17, U14); (b) path corrected (P-1: it is the session crate's, 994), seam = the child-list family → `child_list.rs`; also the VM `document.rs` (1197) split for D-13, and a conditional `table_mutation` split | D-12, D-14, P-1 |
| 5 | PM 161: `isConnected` → slot unless the unification covers it | not covered (tree relation vs node document); slot text `#11-vm-isconnected-bound-document`, owner L3 | D-16, §8 R-6 (d) |
| 6 | PM 223 O-2: lift the new-node entry into `EcsDom`, migrate the strict method, enumerate replaced decisions, leave C1's hook explicit | `node_document_for_new_nodes(source)` with the `FragmentParseDocument` substitution; strict method deleted; census of every production site migrated; hook = `attach_template_contents`'s second argument, no template branch in the entry. Contradiction with C1 rev 2's D-C4 reported | D-10, D-11, §4.4, P-5 |
| 7 | L3 A1–A12 | mapped; A1–A5, A7–A9 changed by C0 (A1–A4 if present at C0's base), A6 by whichever lands second, A10–A12 C1 / L3; A5's value change stated; A8 premise corrected | §4.5, P-2 |
| 8 | `CloneCreated` × `Adopt` census line | edge c0e (name unverified in the L3 memo, P-3) | §8 |
| 9 | C1 rev 2 findings: (a) per-node pairs for contents; (b) overwrite note | (a) D-3 (c) segments + D-6 consumer order and pairs (T18, U13); (b) D-10's overwrite note | D-3, D-6, D-10 |
| 10 | scout answers; open ones answered from code | Q-S1, Q-S3, Q-S5, Q-S9 used as given; Q-S2, Q-S4, Q-S7, Q-S10 answered from code; Q-S8 partly (known list) + a pre-implementation run; Q-S6 not answerable via webref | §11 |
| 11 | siblings C1 rev 2 / T3 rev 3; R-B in DoD with a cell; order | R-B = DoD item 10, cell T13; order N1 → C0 → C1 → T3 → slice b throughout; C1 / T3 hand-offs restated on rev-2 IDs | §8, §9 |

## §11. Open questions

**For the PM / user.**
- **O-1 The umbrella order line and C0's edges (D-9).** (a) C0's PR, landing first, also rewrites the order line to PM
  153's order and adds no edge row (edges into a later member are added by that member) — recommended: the order line
  is a PM decision's text, not a member's row, and must not be stale once a PM-153-ordered member lands; (b) the line
  is left to the PM's own docs PR; (c) each member inserts itself into the line.
- **O-2 Getter fallback deletion vs L3's stated premise (D-1 (c), H-L3).** The L3 memo assumes C0 keeps `core.rs:279`'s
  code. Recommended: delete (it is a proxy that the census makes dead); the observable L3 cell stays green. Alternative:
  keep it until L3 RC-2 lands and delete it there (leaves a proxy across PRs).

**For plan-review.**
- **Q-1** `MutationEvent::Adopt` per adopt call with per-segment pairs (D-6) versus one event per contents segment
  (spec structure, but the consumer would then emit contents callbacks after every outer one, out of spec order).
- **Q-2** The tolerant converter: pass the intended parent to the entry per node (D-11) or keep a threaded cached value
  computed once by the entry (C1 rev 2 Q-1's second half). *Constraint from C1 (C1 memo §4.3, rev 3 `cab354ba`; its
  H-2), not a decision of Q-2:* a threaded value must be recomputed from the entry at each contents fragment
  (`node_document_for_new_nodes(fragment)`), not carried from the template's level as today's
  `convert_children(&contents, fragment, dom, owner_document, opts)` (`convert.rs:284-286`) does; carried, it would
  stamp contents' children with D on C1's head (C1's tolerant legs of T3, T12, T19, U12 kill that form).
- **Q-3** Whether the ownerless shims should become test-only by construction (a `test-util` feature) instead of a
  grep ratchet (P-8).
- **Q-4** Whether `adopt_subtree` stays a separate `pub` function once `adopt` exists (D-3 / D-8 keep it for the
  fragment parse's early re-home).

**Scout questions — status (rev 1's list; scout file of 2026-11-01, partial).**
- **Q-S1** exhaustive `MutationEvent` matches — answered (scout): `range/live.rs:113-163` (prod), `mutation_hook.rs:68-148`
  (test). Used in D-6, P-6.
- **Q-S2** script-reachable ownerless creations — answered from code: §4.4's table (the scout's 42-line list, minus
  test modules); no ownerless custom-element path (`entity_spawn.rs:57` takes an owner). Used in D-11.
- **Q-S3** registries — answered (scout): not modelled (`ShadowRoot.null_registry` only). §7.
- **Q-S4** Attr — answered from code: H-17 (created unstamped; `attr.ownerDocument` → getter → `document_root()`;
  cached in `AttrEntityCache`). Used in D-3 (b), D-11, T19.
- **Q-S5** `outerHTML` / `setHTMLUnsafe` / `insertAdjacentHTML` — answered (scout): all through the primitives. §4.6.
- **Q-S6** Selection API step text — **open**: webref cannot render the Selection API anchors (P-13); not C0's scope.
- **Q-S7** DOMParser document lifetime — answered from code: never destroyed (H-18).
- **Q-S8** tests asserting non-adoption — partly answered (§5.2's known list); the complete list is the pre-implementation
  run of §9 item 1.
- **Q-S9** shadow root's HEAD `owner_document` — answered (scout): `None`, the getter gives `document_root()` (P-6).
- **Q-S10** T12's HEAD value — answered from code (P-7); exact string recorded red-first.
