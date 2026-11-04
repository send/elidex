# C1 `EH-attrs-at-creation-C1` — template contents are owned by the inert template contents owner document (per-PR plan-memo)

Nested PR **C1** of the standalone program `EH-attrs-at-creation` (umbrella
`docs/plans/2026-10-event-handler-attrs-at-creation.md`, approved by #551 `b9122157` at rev 8, carried to rev 8.3 in
place by #570; "the umbrella"). It closes slot `#11-template-contents-owner-document` (carved from #380, `c801ef6c`),
closes T3's residual **R-A** (§9 item 9), and is slice b's precondition x5 in **branch A** (umbrella §1.3 x5, §1.6).
Siblings cited by ID:
- slice a (`docs/plans/2026-10-eh-attrs-slice-a.md`, landed by #563 `3cd84a04`; "the slice-a plan");
- IBP-sandbox N1 (`docs/plans/2026-10-ibp-sandbox-handler-gates.md`, rev 4.5, merged as a plan by #570; implementation
  **#580, merged** as `27b50dfe` on 2026-11-01; "the N1 memo");
- **C0**, a program of three PRs **C0a → C0b → C0c** (PM decision 244): program memo
  `docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md` **rev 3.6** ("the C0 draft"; its decision IDs D-n carry their
  owning PR) and per-PR memos `docs/plans/2026-11-eh-attrs-c0a-replace-record-order.md`,
  `2026-11-eh-attrs-c0b-node-document-fact.md`, `2026-11-eh-attrs-c0c-adopt-notification-adoptnode.md` ("the C0b
  memo" etc.), branch `layout-dom-c0-insert-adopt-plan` `65180942`, not on main. "C0 rev 1" / "C0 rev 2" mark a
  statement of that earlier rev;
- T3 (`docs/plans/2026-10-eh-attrs-t3-target-determination.md`, rev 3 draft on branch `layout-eh-attrs-t3-plan`,
  `14ff3dc2`, not on main; "the T3 draft");
- L3's fragment plan (`docs/plans/2026-10-fragment-dsd-not-on-context.md`, #574 `6aea28da`; PR 0a #576 `cc6a6665` and
  PR 1 #577 `ee6c7a1a` landed);
- L3's Range contents program (`docs/plans/2026-11-range-contents-program.md`, landed by #583 `12ea5dce`; RC-0a
  landed by #587 `fab44a1d`; "the Range program"): its Appendix A site IDs A1–A12 and its RC-2 item D14, read on
  `origin/main`;
- **C1-pre (Document wrapper)**: a standalone prerequisite PR with its own per-PR plan and plan-review (owner Layout,
  PM 328). It discharges slot `#11-bridge-document-post-install` (C1 fires its trigger: I is a new path to a `Document`
  wrapper). Its scope is that plan's; C1 relies on its result only (§4.1, edge c-pre).
Draft IDs are cited as draft IDs.

**Placement (evidence).**
- PM decision 123 (2026-10-31, ledger `project_open-defer-slots.md`, slot `#11-template-contents-owner-document`,
  sub-bullet "Trigger completed 2026-10-31"): "branch A — C1 lands, before slice b; owner: Layout lane (EH-attrs program
  member, umbrella §1.2/§1.6)"; "a per-PR plan under the EH-attrs umbrella with plan-review is mandatory".
- PM decision 143 (2026-10-31; same ledger, slot `#11-cross-document-adopt-on-insert`, sub-bullet "Owner assigned
  2026-10-31"): "C0 is a standalone PR before C1 … Without it, C1 alone makes `document.body.append(tpl.content.cloneNode(true))`
  keep the inert owner document, so relative URLs resolve against about:blank (worse than HEAD)." Rev 1's O-1 is
  therefore decided: (a), C0 first.
- **PM decision 153** (2026-10-31, the user's decision; T3 draft header quotes the option text the user saw): **order
  N1 implementation → C0 → C1 → T3 → slice b**. The ledger sub-bullet for this slot still records the older order
  "N1 implementation → T3 → C1 → slice b (T3 and C1 independent)"; decision 153 supersedes it (§8 R-2).
- PM decisions 144 and 159 (relayed by the driver, 2026-11-01): the umbrella edits C1 needs are carried by C1's PR, in
  place, with history; C1's PR adds the C1 row of umbrella §1.2, not the C0 row (C0 draft D-9 adds that one).
- PM decisions 199 and 207 (ledger slot `#11-ownerless-node-creation`, "registered 2026-11-01, PM 207"): "One mechanism:
  every creation site takes its owner from a single entry, and `node_document` is the only way it is read (C0; PM 157
  replaces `effective_node_document`)", corrected by PM 223 (the entry is lifted into `EcsDom`). Owner: Layout (C0/C1).
- **PM decisions 223 O-2 and 227** (relayed by the driver, 2026-11-01; option texts not shown to this memo's author):
  C0 owns lifting the new-node entry into `EcsDom` and migrating every production creation site, both parser tiers
  included, onto it (C0 draft D-10, D-11, §4.4). C1's share of the entry shrinks to the template-contents hook: the
  second argument of `attach_template_contents` (C0 draft D-10, "C1's hook point"). This closes rev 2's O-2 as (a).
- PM decision 215 (relayed): L3 RC-2's D14 (slot `#11-clone-shadow-children-subtree`, ledger "registered 2026-11-01, PM
  212") lands before C1 and rewrites the same child-clone loop; C1 rebases on it.
- PM decision 145 (relayed): before implementation, measure in the built-in Chromium the outcomes C1 changes where the
  spec is silent or elidex's rule is a reading (§1.1).
- The option texts of decisions 144, 145, 159, 199, 207 and 215 were not shown to this memo's author; they are cited
  as relayed in the driver's rev-2 brief, not quoted.

**Prerequisites (at C1's base).** N1's implementation (#580), **C0a, C0b and C0c** landed; L3 RC-0a (#587) and RC-2a
(which carries D14) landed (§4.4); **C1-pre** landed (edge c-pre). Order: N1 → C0a → C0b → C0c → C1 → T3 → slice b (C0
draft §1 *Order*), with C1-pre before C1 (its order against C0 is its own plan's). Of these, N1's implementation and
RC-0a are on `origin/main`; C0, RC-2a and C1-pre are not. The C0 dependency is
semantic, not cosmetic: §1's table shows that C1 on a base without insertion adoption makes the most common template
pattern worse than HEAD (PM decision 143); with C0 on the base it does not (§1, K-0 lens).

**Scope.** One rule computes a template's contents' node document — WHATWG HTML §4.12.3's *appropriate template contents
owner document* — and every site that establishes, clones, parses into or adopts template contents goes through it,
reading node documents only through C0's `EcsDom::node_document` (NEW: C0):
1. the inert document itself, as an ECS entity and two components (D-C1, D-C2);
2. the establishing steps at every template creation (both parser tiers, `createElement`, the clone post-pass), through
   C0's hook point, the second argument of `attach_template_contents` (D-C3);
3. the guarantee that C0's new-node entry `node_document_for_new_nodes` (NEW: C0, C0 draft D-10) returns the inert
   document for every node created inside template contents, with no template branch in the entry (D-C4); L3's A11 /
   A12 read the same entry (§4.5);
4. the template *adopting steps*: the target of the contents edge of C0's one adopt walk, which is also the `new` of
   C0's per-segment (old, new) pair that `adoptedCallback` receives (D-C5);
5. the template *cloning steps*, and the per-pair document of the clone worklist (D-C6; site A10);
6. the inert document's lifetime: created lazily, destroyed with its document, never copied by a clone (D-C7, D-C8);
7. the inert document is not a render/layout root (D-C9);
8. the C2 docstrings and the tests that pin C2 are rewritten (§5.2).

**Not in scope.** Insertion adoption, the node-document reader, the new-node entry and the migration of every
production creation site onto it, both parser tiers included (C0 draft D-10, D-11, §4.4; PM decisions 223 O-2 / 227);
the custom-element `Adopt` consumer and its per-node (old, new) pairs in spec order (C0 draft D-3 (c), D-6); creation-time handler processing (slice b,
S-1/S-2); target determination (T3, after C1; it rewrites umbrella cell C18, §6 T18); L3's Range node-creation sites
A1–A9 (C0 rev 2 / L3, §7); the template element's `for` attribute and content patching (§7); the DocumentFragment
*host* of template contents (§7); multi-realm (B1; "given document's relevant realm", §4.1).

**Changelog.** *Rev 3.6* (2026-11-04, focused re-check after R1; PM 335): `inert_template_document_of` deleted (D-C2,
D-C5, §4.2, §4.9, U4, §6.3, DoD 2; IMP-2); the adopt pair's restatements become pointers to D-C5 (A6, D-C5's
Notification, R-C8, §4.7; IMP-3); T1's URL legs and M3's writer on C1-pre (edge c-pre; IMP-1); §4.2 matches its
`debug_assert!`, R-9 pointer (MIN-1); §4.4's per-pair document is a `NewNodeOwner` (MIN-2); §3 `clone_subtree` row
(MIN-3); P-6's href owner C0b D-7 (c) (MIN-5); §5.2 frame and U19 leg 3 (MIN-6); T28 self-contained (MIN-7).
*Rev 3.5* (2026-11-04, plan-review R1; findings F1–F9 and Q-4 of the R1 aggregate, PM 328): C1-pre
(Document wrapper) is a prerequisite (header, §4.1, §4.8, H-C16, T1, T5, M3, R-C5, edge c-pre; Q-S10 closed; F1/F2);
D-C5's contents edge uses `f(target)` and C0c's skip compares against it (T26–T28, U13, §6.3; F3, F5); §5.2's C0 cell
list rebuilt by property (F4); edge c-E follows #583 on main (F6); T12's DSD legs strict only, Q-S12 closed (F7); edge
c-NR (F8); §7's unslotted rows go to the PM as R-6 to R-9, own deferrals 1 (F9); Q-4 decided (`NewNodeOwner`, a C0b
erratum); R-10 (stale ledger pointer); §4.2's stale-link branch removed; citations (§3, §4.5, §4.9, M4, M5); the Range
program and RC-0a cited as landed (header, §5.3).
*Rev 3.4* (2026-11-03): §4.4 step 3, D-C6, A4×A5 and §5.1 P-3 on the base: C0b D-18 / edge c0g remove
pass 4's hand stamps; the host and shadow root get the pair's document from creation (C0b D-11); `clone_subtree`'s
override type is §11 Q-4. *Rev 3.3* (2026-11-03): the clone post-pass passes `node_document_for_new_nodes(pair_doc)`, not the
`Entity` (`NewNodeOwner` has no public constructor, C0b D-17; a Document → itself, C0b U11); the per-pair document
stays an `Entity` in either §4.4 representation; A2×A5 no longer says the clone path skips the entry (D-C3, §2.2, §4.4).
*Rev 3.2* (2026-11-03): aligned to the C0 program (C0 draft rev 3.6, `65180942`): C0 is three PRs
(C0b gives I its hook and value, C0c the per-segment pairs; edges c3b / c3c); the §13.4 redirection is
`FragmentParseRoot` on the fragment-parse root (C0 D-10), so the "re-home through the throwaway and back" text is gone
(C0 P-17); the hook's argument is a `NewNodeOwner` (D-17); §4.3's constraint is met by D-20; `walkers.rs` changes order
in C0c (D-19); §10's rev 2 / rev 3 tables moved to history pointers. Rebased onto `12ea5dce`; no cited coordinate
moved (§0.1). *Rev 3.1* (2026-11-03): clerical — C0 reference to rev 2.1 `d262ca8c`; O-1 decided (a) (PM 223 O-1 / 232); edge c-D14 → c-RC2a (PM 241). *Rev 3* (2026-11-01): aligned with C0 rev 2 (`fa68c20f`, now `42ca6a2c` after C0's rebase; PM decisions 223 O-2 / 227): the new-node
entry, its parser-tier migration and the per-node `adoptedCallback` pairs are C0's; D-C4, D-C5, §4.3, §5.1, §5.3,
§6, §8, §9 and §11 shrink to the template-contents part; N1's implementation landed (`27b50dfe`); §10.1 lists each
move. *Rev 2* (2026-11-01): driver brief inputs 1–9 (§10 lists each and its disposition); rebased on
`593aa314` and every coordinate re-measured; C0 is a landed prerequisite (decided), its `node_document` / `Adopt` are the
only node-document reader / notification C1 uses; `node_document_for_new_nodes` is the single new-node owner entry;
the order is N1 → C0 → C1 → T3 → slice b; the browser-measurement step is §1.1; R-A is in the DoD. *Rev 1*
(2026-10-31, `1a2cb286` before the rebase, `162c6acf` after): first draft, pre-plan-review.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` is a **`593aa314`** coordinate (`origin/main` on 2026-11-01, this branch's base after the rebase;
`git merge-base HEAD origin/main`). Since rev 1's frame (`74e15df1`) main gained #568 (throwaway-root append outside
`debug_assert!`), #576 (serializer / `insertAdjacentHTML` moved to `crates/dom/elidex-dom-api/src/element/markup.rs`),
#577 (`node_document_for_new_nodes`; a fragment's top-level DSD template stays plain) and #579. Where a symbol exists,
it is cited by symbol and the line is a convenience: re-find with `/usr/bin/grep -n '<symbol>' <file>`. C1's
implementation is cut after N1's implementation, C0 and D14 land, so **every coordinate is re-measured at that base**
(§9 item 1); C0 moves `adopt_subtree` from `teardown.rs` to `mutation.rs` (C0 draft D-3) and deletes `owner_document`'s
fallbacks (C0 draft D-1). Code references come from `/usr/bin/grep -n` / `sed -n` over the `593aa314` tree, run
2026-11-01. Every § number ↔ title pair and every algorithm step comes from `.claude/tools/webref` (`heading`, `dfn`,
`body`), run 2026-11-01; the commands are in §3.

*Rev 3 frame check.* This branch is rebased onto `27b50dfe` (`origin/main` on 2026-11-01: #578, #580). `git diff --stat
593aa314 27b50dfe -- crates` touches, among the files this memo cites, only
`crates/script/elidex-script-session/src/scripting.rs` (N1's implementation) and
`crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs_once.rs`; every other coordinate is byte-identical and
stays a `593aa314` coordinate. The two changed files are cited at `27b50dfe` (H-C9, §5.1 P-7, §5.2, §5.3).
*Rev 3.2:* rebased onto `12ea5dce`; of the cited files only `cssom_sheet.rs` changed (at `:584`, not the cited `:213`),
so the frame stays (`git diff --stat 27b50dfe 12ea5dce -- crates`).

### §0.2 Premises re-measured

At `593aa314` unless the row says "on the base" (= `593aa314` + N1 implementation + C0 + D14, as their drafts state).

| id | claim | evidence |
|---|---|---|
| H-C1 | One establishing helper exists; its owner parameter is the C2 approximation | `EcsDom::attach_template_contents` `crates/core/elidex-ecs/src/dom/mod.rs:557`; doc `:548-556` ("ships the **C2 approximation** `owner_doc = template's own node document`"; its "strict: post-insert `owner_document(template)`" is stale since #577, §5.2); creates the fragment with `create_document_fragment_with_owner(owner_doc)` and links `TemplateContents { fragment }` (`crates/core/elidex-ecs/src/components.rs:366`) |
| H-C2 | Four production callers of the helper | `/usr/bin/grep -rnF 'attach_template_contents(' crates --include='*.rs' \| /usr/bin/grep -v '/tests/\|tests\.rs\|/tests_'` → clone post-pass `crates/dom/elidex-dom-api/src/node_methods/clone.rs:422`, `createElement` `crates/dom/elidex-dom-api/src/document.rs:264` (`Some(this)`), tolerant `crates/dom/elidex-html-parser/src/convert.rs:284` (threaded `owner_document`), strict `attach_ordinary_template_contents` `crates/dom/elidex-html-parser-strict/src/tree_builder/shadow.rs:123-125` (`node_document_for_new_nodes()`), plus the definition (verified 2026-11-01) |
| H-C3 | The strict parser has one new-node owner function (since #577) | `node_document_for_new_nodes` (strict `TreeBuilder` method) `crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs:271` (`pub(super)`): fragment parse → `owner_document(context)`, else `Some(self.document)`. Its callers: `/usr/bin/grep -rn 'node_document_for_new_nodes' crates` → `insert.rs:108` (`insert_foreign_element`) and `shadow.rs:124` (`attach_ordinary_template_contents`) only. It takes no intended parent, so inside template contents it returns the parse (or context) document. **On the base** C0b deletes it and adds `EcsDom::node_document_for_new_nodes(source) -> NewNodeOwner` (NEW: C0b), `node_document(source)` except under the §13.4 fragment-parse root, which carries `FragmentParseRoot { redirect_document }` (NEW: C0b) and answers the context's document (C0 draft D-10, D-17) |
| H-C4 | The other parser creations stamp nothing | strict: `insert_element_named` `insert.rs:55` (`create_element`), `insert_comment` / `insert_comment_to` `:131`, `:140` (`create_comment`), `append_text` `:193` (`create_text`), the DSD template `shadow.rs:153`; tolerant: `convert_node` `convert.rs:237` creates elements with `None` (`:251`), text, comment and doctype without an owner (`:305`, `:309`, `:316`). The tolerant tier stamps **only** the contents fragment (`convert.rs:284` → the helper); the threaded `owner_document` is otherwise only an argument (`:199`, `:286`, `:289`) (scout Q-S3). The tolerant tier's fragment entry chooses its threaded document itself: `owner_document(context)` `crates/dom/elidex-html-parser/src/lib.rs:158`, a second statement of H-C3's fragment branch. **On the base** every site of this row takes its owner from C0's entry (C0 draft D-11, §4.4) and `lib.rs:158`'s choice is gone |
| H-C5 | Today descendants without a stamp resolve through the tree root; C0 removes that | `EcsDom::owner_document` `mod.rs:817`: stamp first, else the tree root's Document, else a `DocumentFragment` root's stamp (`:836-856`). **On the base** C0 deletes both fallbacks and adds `EcsDom::node_document` (NEW: C0) (Document → itself; node → its live `AssociatedDocument`, else `None`) as the sole reader (C0 draft D-1); every node linked under a parent with a node document is stamped by C0's link-time adopt (C0 draft D-4, invariant I-U) |
| H-C6 | Adoption re-homes template contents to the **same** document as the template | `EcsDom::adopt_subtree` `crates/core/elidex-ecs/src/dom/tree/teardown.rs:251` walks `for_each_uncapped_shadow_inclusive(root, TemplateContentsWalk::Include, …)` and stamps every visited node with `document`. Caller: `finish_detached_fragment` (`teardown.rs:337`); a test (`dom/tests/destroy.rs:324`). **On the base** C0b keeps this walk as `adopt`'s step 3 with the identity contents edge that C1 replaces by `f(document)` (C0 draft D-3 (a)) and deletes the `finish_detached_fragment` caller (D-8); C0c records a contents segment `{ fragment, documents: Option<(old, new)> }` per contents fragment reached (D-3 (c)), visits contents right after their template (D-19), and dispatches one `MutationEvent::Adopt` (NEW: C0c) per adopt call whose consumer gives each element its own segment's pair (D-6) |
| H-C7 | A clone stamps an explicit owner on every node the deep cloner makes | `clone_subtree`'s document choice `crates/core/elidex-ecs/src/dom/tree_clone.rs:206-210`, root stamp `:221`, descendant stamp `:417`. The post-pass `clone_node_with_shadow_honor` `clone.rs:227` computes **one** `shadow_doc` (`:239-243`) and threads it into every pair's template pass (`replicate_template_contents`, `:262`, def `:405`; fragment `:422`, children `:426`) and shadow pass (host and shadow root stamps `:283-284`, shadow children `:290-302`) |
| H-C8 | The `ownerDocument` getter and URL resolution read the node document | dom-api getter `crates/dom/elidex-dom-api/src/node_methods/core.rs:279` (`owner_document(this).or_else(\|\| dom.document_root())`); href base `crates/dom/elidex-dom-api/src/element/href_accessor.rs:50`; base URL `crates/dom/elidex-dom-api/src/element/document_base.rs:177`. On the base the last two read `node_document` (C0 draft D-7 (b), (c)) |
| H-C9 | The handler gates derive the node document through one rule | `scripting_disabled_for_platform_object` clause (b) `crates/script/elidex-script-session/src/scripting.rs:108-122` at `27b50dfe` (node arm, composed-root proxy; fn `:97`). Its doc names the template false negative as this slot's (`:73-74` at `27b50dfe`). **On the base** C0 replaces the proxy by `node_document(entity) != bound` (C0 draft D-7 (a)); T3 (after C1) reads `node_document` too (C0 draft H-T) |
| H-C10 | A Document entity is spawned in one place | `EcsDom::create_document_node` `mod.rs:410`. Production callers: `teardown.rs:301` (fragment throwaway), `mod.rs:395` (`create_document_root`), `crates/dom/elidex-form/src/inert_document.rs:119` (DOMParser) |
| H-C11 | `root_entities` excludes only `DocumentFragment` | `EcsDom::root_entities` `crates/core/elidex-ecs/src/dom/tree/navigation.rs:362-374`; consumers: `/usr/bin/grep -rn 'root_entities()' crates --include='*.rs' \| /usr/bin/grep -v '/tests/\|tests\.rs\|/tests_'` → `crates/core/elidex-render/src/builder/mod.rs:479`, `:995`, `crates/layout/elidex-layout/src/layout/mod.rs:371`, `crates/layout/elidex-layout/src/hit_test.rs:86` (4; the `lifecycle.rs:300` hit is `clear_root_entities`, unrelated) (verified 2026-11-01). Rev 1's `grep -v test` dropped `hit_test.rs` (§0.3 P-C8) |
| H-C12 | Two production paths destroy a Document; DOMParser documents are never destroyed | scout Q-S1 (29 destroy / despawn calls enumerated, 2026-10-31): `finish_detached_fragment`'s `destroy_entity(document)` `teardown.rs:341` and the strict rollback `despawn_subtree(document)` `tree_builder/mod.rs:233`; DOMParser documents (`inert_document.rs:119`) are never destroyed; iframe documents are not destroyed as ECS entities (each navigation and iframe gets a new `EcsDom`). `destroy_entity` (`teardown.rs:51`) cleans shadow links only, no Document cascade |
| H-C13 | elidex has no XML document | `/usr/bin/grep -rn 'is_html_document\|XmlDocument' crates --include='*.rs'` → only the forward-looking note `mod.rs:730`; DOMParser serves `text/html` only (umbrella §3 row §8.5.1) |
| H-C14 | The clone-policy table enumerates every component | `tree_clone.rs:15-34` (module doc), "deliberate non-copy" row `:30`, `TemplateContents` row `:27` |
| H-C15 | The bind pass excludes template contents | slice a's bind walk `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:178` (`TemplateContentsWalk::Exclude`); umbrella D-11, K-5, K-6 |
| H-C16 | The VM's `Document` methods act on the receiver entity | scout Q-S5: methods resolve the receiver (`crates/script/elidex-js/src/vm/host/document.rs:89-100`); the `document_root()` fallback appears in the `ownerDocument` getter and in the href / `documentURL` paths (sampled; the 109 `document_root()` hits are not classified). `Document.prototype.cloneNode` is reachable (`node_proto.rs:171`, `node_methods_extras.rs:201-215`, `install_document_methods_if_cloned_doc`) (scout Q-S9). The methods are installed per wrapper on three paths only (bound document, DOMParser, `cloneNode`), not on the `ownerDocument` path that reaches I (plan-review R1 F1); I's Document surface is C1-pre's (edge c-pre) |
| H-C17 | No contents-fragment host link | scout Q-S7: only `ShadowRoot.host` (`components.rs:276`); no slot |

### §0.3 Premise errors (reported)

Rev 1's, with their status:
- **P-C1 (rev-1 brief: "the #380 plan").** Unchanged: no #380 plan is in the repository; the slot's "plan §5 Decision C /
  plan-review §12 ratified C2" is reachable only through the ledger and PR #380's body.
- **P-C2 (order).** Resolved by PM decisions 143 and 153 (C0 first; N1 → C0 → C1 → T3 → slice b).
- **P-C3 (umbrella §1.2 has no C1 row).** Still true at `593aa314`; R-1 (§8) adds it.
- **P-C4 (umbrella C14 / §8.2 defect (5)).** Still true; on the decided order C1 (with N1 and C0 on its base) inverts
  C14's getter and dispatch. R-3 (§8) gives the texts.
- **P-C5 (ledger "adopt-special-casing is that balloon").** Superseded by C0's existence.

New in rev 2:
- **P-C6 (rev 1, release leak).** Rev 1's H-C12 / U5 / R-C7 said the strict error path reclaims the throwaway subtree at
  its frame `74e15df1`. At `74e15df1` `begin_detached_fragment`'s `append_child(document, root)` was inside
  `debug_assert!`, so in release builds the root was not attached and `despawn_subtree(document)` did not reach it
  (scout). The claim is true only from #568 (`1ed126b4`), which is on this rev's frame.
- **P-C7 (rev 1, D-C7: "a fragment parse that builds a template creates an I for its throwaway document").** Wrong at
  `593aa314`: since #577 both tiers attach a fragment parse's template contents with the **context's** node document
  (strict `node_document_for_new_nodes`, tolerant `lib.rs:158`), so no inert document of the throwaway is ever needed.
  On the base C0's root redirect keeps that value (C0 draft D-10); D-C7 / U4 are restated.
- **P-C8 (rev 1, P-8 / H-C11).** Rev 1's root-consumer census filtered with `grep -v test`, which also drops
  `crates/layout/elidex-layout/src/hit_test.rs` (the path contains "test"); the consumers are 4, not 3. Every census in
  this rev filters with `'/tests/\|tests\.rs\|/tests_'`.
- **P-C9 (rev 1, H-C4 / D-C4 (b)).** "The tolerant tier threads the template's `owner_document` into the contents'
  children and into declarative shadow content" as a *stamp*: it stamps only the fragment (scout Q-S3; H-C4).
- **P-C10 (rev 1, §1 table and cells T9 / T19).** Rev 1's HEAD column gave `[false]` for T9's
  `c.ownerDocument === tpl.content.ownerDocument` and `false` for T19's inside-contents comparison. Under C2 both sides
  are `document`, so both are `true` at HEAD; neither cell discriminated. T9 and T19 are re-stated with `!== document`
  discriminators (§6).
- **P-C11 (rev 1, H-C8 / §4.6 / edges).** Rev 1 assumed T3 before C1 (base "N1, T3, C0 landed", edge c2 "T3 → C1",
  cell T18 "needs T3", A8×A9 and R-C4 "C18 changes to T3's W2 shape"). Under PM decision 153, T3 is after C1: C1 keeps
  C18 (the base's target rule ignores the document) and T3 rewrites it (T3 draft P-T9).
- **P-C12 (driver brief, input 6).** "D14: shadow children clone passes the operation's subtree flag". The code
  **already** passes the operation's flag (`clone_recording(dom, child, deep, shadow_doc, &mut pairs)`, `clone.rs:294`),
  and that is the defect: DOM §4.4 *clone a node* step 6.7 clones shadow children "with … subtree set to true" (webref
  `body dom concept-node-clone`); the ledger text of `#11-clone-shadow-children-subtree` says the same. C1 assumes D14's
  form is **subtree = true** for shadow children (§4.4).
- **P-C13 (driver brief, input 4).** "`node_document_for_new_nodes` … THE single entry that decides the owner document of
  newly created nodes": at `593aa314` it is a `pub(super)` method of the strict tree builder (H-C3). The tolerant tier
  (`lib.rs:158`) and every dom-api creation site decide their owner elsewhere, and L3's A11 / A12 (in `elidex-dom-api`)
  cannot call it. A single entry therefore requires promoting it to `EcsDom`; who promotes it was rev 2's O-2.
  **Resolved (rev 3)** by PM decisions 223 O-2 / 227: C0 promotes it and migrates every site (C0 draft D-10, D-11).
- **P-C14 (C0 rev 1, A3×A4).** "Elements in contents can be custom only if upgraded there; C1 makes that impossible by
  giving contents the inert document." A custom element created in the document and appended into `tpl.content` stays
  custom in the inert document (adopt does not un-upgrade), so custom elements in contents exist on C1's head, and a
  later adoption of their template must give them `adoptedCallback(f(old), f(new))` (D-C5, cell T25). Hand-off H-2.
  **Folded by C0** (C0c: D-3 (c) per-segment pairs, D-6, D-19; C0c memo P-4, cells T18, U13).
- **P-C15 (C0 rev 1 × #577).** C0 rev 1 (frame `1ed126b4`, before #577) does not mention
  `node_document_for_new_nodes`. On C0's base its link-time adopt (D-4) re-homes a strict foreign element, created with
  the context's document by that function in a fragment parse, to the throwaway document at its append (the parent's
  node document), and `finish_detached_fragment` re-homes it back. The intermediate stamp is unobservable (the parse
  runs with the dispatcher taken), but the function's fragment branch then describes a stamp that C0 overwrites.
  **Moot since C0 rev 3:** no build node except the root holds the throwaway and no build link adopts (C0 draft D-8,
  D-10; its P-17).
- **P-C16 (T3 draft × C0 draft × this brief, umbrella rows).** The T3 draft (`14ff3dc2`) carries umbrella rev 8.4 / 8.5
  edits that add **both** C0 and C1 rows to §1.2 and edges x7–x10; the C0 draft D-9 says C0's PR adds the C0 row; the
  driver's brief (PM decisions 144 / 159) says C1's PR adds the C1 row. Three branches claim the same lines. O-1 (§11); decided rev 3.1 (PM 223 O-1 / 232).

### §0.4 Decisions (this PR)

- **D-C1 — the inert document is an entity.** The *associated inert template document* of a Document D is a Document
  entity I, spawned by `create_document_node` (H-C10), so it has the Document shape (and `DocumentBaseUrl` =
  `about:blank`, the spec's fallback base URL for it, §3 row §2.4.3). Two components (NEW), both in
  `crates/core/elidex-ecs/src/components.rs`:
  - `AssociatedInertTemplateDocument(Entity)` (NEW) on D: the forward link D → I.
  - `InertTemplateDocument` (NEW), a unit marker on I: "a Document created by this algorithm".
  No back-link I → D: the spec defines no query from I to D, and the lifetime need (D-C7) runs from D.
  *Side-store rule* (CLAUDE.md): per-entity, `Send + Sync`, not a per-VM identity handle (an `Entity`, not an
  `ObjectId`), so a component; a `HashMap<Document, Document>` on `EcsDom` or in `HostData` is rejected. *Idiom table:*
  OO engines' owning pointer from a document to its template document plus a back pointer → forward-link component on
  D + marker on I; owner-held lifetime → despawn cascade (D-C7); "is template document" boolean → marker query.
- **D-C2 — one rule.** `EcsDom::appropriate_template_contents_owner_document(&mut self, document: Entity) -> Entity`
  (NEW; "`f`") is §4.12.3's algorithm: if `document` carries `InertTemplateDocument`, return it; else return its linked
  I, creating and linking it on first use (the spec's "if document does not yet have"). Step 1.1.2 ("if document is an
  HTML document, mark newDocument as an HTML document also") is vacuous (H-C13) and stated so in the doc comment. The
  rule is idempotent over the pair: `f(D) = f(I) = I`, so a caller may pass either and gets the same answer. A predicate
  `EcsDom::is_inert_template_document(&self, document: Entity) -> bool` (NEW) reads the marker; the predicate is the
  per-node "template origin" fact slice b's S-2 reads through the node document (§4.6).
- **D-C3 — the establishing steps call the rule once, inside the helper.** `attach_template_contents(template,
  owner)` keeps its name and its single-attach contract; its second argument, a `NewNodeOwner` on the base (C0 draft
  D-17), carries **the template's node document** and the helper applies `f` to `owner.document()`. `None` keeps
  today's orphan fragment (a documentless context or an orphan clone). No caller computes I itself. This is C0b's hook
  point (C0 draft D-10, "C1's hook point"): on the base the strict tier, the tolerant tier and `createElement` already
  pass `node_document_for_new_nodes(template)` (C0 draft D-10, D-11, §4.4), the template's node document with the §13.4
  redirection; the clone post-pass passes the entry on the pair's Document (D-C6; a Document → itself, C0b U11). C1 changes no argument of the other three callers; it changes what the
  helper does with it (applies `f`). Through idempotence a template nested in contents gets I whether its caller passes
  D or I.
- **D-C4 — the new-node entry is C0's; C1 guarantees its value inside contents (PM decisions 199 / 207, 223 O-2 /
  227).** C0b adds `EcsDom::node_document_for_new_nodes(source)` (NEW: C0b, H-C3) — `node_document(source)`, except
  that the §13.4 fragment-parse root answers its `FragmentParseRoot`'s `redirect_document` — deletes the strict tree
  builder's method, and migrates every production creation site, both parser tiers included, onto it (C0 draft D-10,
  D-11, D-20, §4.4). C1 adds no entry, no entry branch and no creation-site change. C1's obligation is the entry's
  **value** inside template contents:
  - inside template contents `source` is the contents fragment or one of its descendants; D-C3 makes the fragment's
    stamp I, C0's link-time adopt (C0 draft D-4, invariant I-U) carries it to every linked descendant, and shadow roots
    inside contents take their host's node document (C0 draft D-11; DOM §4.9 *attach a shadow root* step 5), so the entry returns
    I there with no template-specific branch, for nested contents too (`f(I) = I`);
  - in document parsing the creation stamp and C0's link-time stamp therefore agree inside contents: no `Adopt` from D
    to I for a parsed node (cell U12, both tiers);
  - in a fragment build no node except the root holds the throwaway and no build link adopts (C0 draft D-8, D-10), so
    a template of the build has the context's node document (inside contents, its I) and `attach_template_contents`
    computes `f(context's document)`; `f` is never computed for the throwaway (D-C7, U4).
  Rev 2's D-C4, §4.3's parser-site list and Q-1 moved to C0 (§10).
- **D-C5 — the template adopting steps inside C0's adopt.** DOM §4.5 *adopt* step 3 walks node's **shadow-including
  inclusive descendants**, which do not include template contents; step 3.4 runs each descendant's *adopting steps*,
  and a template's (HTML §4.12.3) adopt its contents into `f(node's new node document)`. C1 replaces C0's identity edge
  with that: the walk re-homes the root's shadow-including tree to `document` and, for each template it reaches, adopts
  the template's contents (recursively the same way) into `f(document)`; nested templates resolve to the same I
  (idempotence). The walk's shape is C0's: `adopt_subtree` keeps its uncapped `TemplateContentsWalk::Include` walk and
  has one contents edge, where it records a segment `{ fragment, documents: Option<(old, new)> }` with `new` = the
  target it writes into the contents (C0 draft D-3 (c), C0c); rev 2's Q-2 is answered there, and C1 adds no walker
  variant (C0c's D-19 changes only the `Include` visiting order).
  C1's whole change is that edge's target: `new` = `f(target of the template)` instead of the template's target.
  `new` comes from the `&mut` rule `f` (creating I on first use), computed on `adopt`'s `&mut` path at most once per
  adopt call and only when the walk reaches a contents fragment (D-C7's "iff").
  C0c's skip test (C0c memo D-3 (c), "`old` equals `new`") therefore compares the contents' old document with the
  **mapped** `new` = `f(target)`, never with the unmapped target (T27, T28).
  **Notification (owner split).** C0c owns the consumer change: one `MutationEvent::Adopt` per adopt call carries every
  segment's pair, and the custom-element consumer visits contents right after their template (D-19) and gives each
  element its own segment's pair (C0 draft D-6; C0c's cells T18 and U13 pin per-node pairs and order under C2). C1 owns only the
  value of the contents pair (the edge above), with no consumer change (cell T25). Rev 2's Q-3 (consumer mechanism) is answered by C0 draft D-6; C1 does not touch the consumer.
- **D-C6 — the template cloning steps, and one document per clone pair.** The copy's contents are established at the
  copy's creation through D-C3 with the pair's document, and the source contents' children are cloned with **document
  = the copy's contents' node document** (HTML §4.12.3 *cloning steps* step 2), not the operation's. Every later pass
  for a pair inside those children (a nested template's contents; a clonable shadow host's shadow children) uses
  **that pair's document**, not the operation-wide `shadow_doc`; the host and its shadow root follow from the host's
  creation (§4.4 step 3). The worklist
  therefore carries a document per pair; its representation (a third tuple field, or a read of the stamp the cloner
  wrote) is input (§4.4), written on D14's form (shadow children cloned with subtree true).
- **D-C7 — lifetime.** I exists iff D has needed it (lazy, D-C2). Destroying D destroys its linked I
  (`destroy_entity` on a Document follows `AssociatedInertTemplateDocument`). On C1's head no production path destroys
  a Document that has an I (H-C12: only throwaway documents are destroyed, and D-C4 never computes `f` of a throwaway),
  so the cascade is the lifetime invariant for any later destroying path (and the test double's), not a hot path; it
  is not dead code (it runs on every Document destruction and is exercised by U7). I has no tree children unless a
  script appends some; `destroy_entity(I)` then behaves as for any Document. A node whose node document is I and that
  outlives D reads `None` through C0's `node_document` dangling guard. DOMParser documents are never destroyed (H-C12),
  so each DOMParser document that ever creates a template keeps one I for as long: one entity per such document on top
  of a pre-existing per-document leak (§7).
- **D-C8 — clone policy.** Both components are rows of the clone-policy table (H-C14), deliberate non-copy: a cloned
  Document is "not created by this algorithm" and gets its own I on first need; a clone of I is an ordinary Document.
- **D-C9 — I is not a render, layout or hit-test root.** `root_entities` excludes a Document carrying
  `InertTemplateDocument`, at the same source and for the same reason as #380's `DocumentFragment` exclusion (template
  inertness by construction): a script can give I a child (`tpl.content.ownerDocument.append(…)`), and H-C11's four
  consumers would then lay out, paint and hit-test it. DOMParser documents and detached subtrees reach the same consumers
  at HEAD; that wider defect is not C1's (§7, scout Q-S6).
- **D-C10 — C0 first (decided: PM decisions 143, 153).** C1's base contains C0. C1 adds no second node-document reader
  and no `owner_document` read in new code: every read in C1's new or changed code is `node_document` (C0) or the
  new-node entry (D-C4); `owner_document` stays the `ownerDocument` getter's "null for a Document" wrapper (C0 draft
  D-1). `/usr/bin/grep -n 'owner_document(' <every file in §5.3>` over C1's diff adds no hit (DoD item 2).
- **D-C11 — browser measurement before implementation (PM decision 145).** §1.1's table is filled in the built-in
  Chromium before C1's implementation branch is cut; a row whose result differs from the "C1 head" column goes back to
  the PM before implementation.

## §1. Shape and verdict

**Axes (§2): 9** — A1 the rule (D-C2), A2 creation paths and the new-node entry (D-C3, D-C4), A3 nested templates, A4
shadow roots in template contents, A5 clone (D-C6), A6 adopt (D-C5, on C0), A7 lifetime (D-C7, D-C8), A8 node-document
consumers (H-C8, H-C9, D-C9), A9 the EH program's contract (K-*, X-W, S-1/S-2, T3's R-A). ≥ 3 intersecting axes, so this
is edge-dense: the PM's classification holds and plan-review is mandatory (CLAUDE.md *Edge-dense work* (b)). As a
per-PR slice of the approved umbrella it is a terminal unit (base case (c)) once its plan-review passes.

**Why C0 must come first (PM decision 143; the evidence).** At HEAD the C2 approximation and the missing insertion
adoption cancel for the common template patterns; C1 without C0 would break the cancellation:

| pattern | HEAD (`593aa314`: C2, no insertion adoption) | C1 without C0 (rejected) | C1 on C0 (this PR) = spec |
|---|---|---|---|
| `c = tpl.content.cloneNode(true); document.body.append(c)`; a moved node's `ownerDocument` | `document` (the clone stamped `document`, H-C7) | I (the clone stamps I; nothing re-homes it at the append) | `document` (DOM *insert* step 7.1 adopts) |
| same; `<a href="x">` from it, `a.href` | absolute against the page base | `"x"`: the URL does not parse against I's base `about:blank` | absolute against the page base |
| `tpl.innerHTML = '<b>'`; `b.ownerDocument` | `document` | `document` (the fragment parse adopts into the context's document, H-C6; the placement does not adopt) | I |
| `tpl.content.append(document.createElement('div'))`; its `ownerDocument` | `document` | `document` (explicit owner, no adoption) | I |
| `document.body.append(tpl.content)` with a parser-built `<svg>` child; its `ownerDocument` | `document` | I | `document` |

Without C0, rows 1, 2 and 5 are **worse than HEAD** on the most common template idiom (row 1 is §4.12.3's own example),
and rows 3 and 4 leave two answers to one node-document question depending on how the node got there (the decision
surface One-issue-one-way forbids). With C0 on the base every row reaches the spec's value: rows 1, 2, 5 by C0's
adoption at the append, rows 3 and 4 by C0's adoption at the placement into a contents fragment whose node document is I
(C0 draft §4.6, I-U). §6 T9 is the K-0 guard for rows 1–2.

**K-0 lens (umbrella K-0, S-3: no step worse than HEAD).** On C1's base (N1, C0, D14, C1-pre), C1's observable changes are
§5.4's R-C1 to R-C8. Each moves an outcome from C2's to the spec's; none moves it to a third value. Two need a
measurement before they are asserted (R-C2 URL resolution, R-C5 Document methods on I; §1.1).

**Split decision: one PR.** The rule, the inert document, its lifetime, the establishing, parsing, cloning and adopting
sites form one fact (the contents' node document); landing a subset would leave two answers to it (rows 3 and 4 above,
in reverse). The prerequisite 1000-line split is none (§5.3).

### §1.1 Browser measurement (pre-implementation step, PM decision 145)

Run in the built-in Chromium (and, if available, one other engine) on a page served over `http://` with a known base,
before C1's implementation branch is cut; record the engine version, fill the two right-hand columns, and send any row
whose result differs from the "C1 head" column to the PM (D-C11). **No result is filled in this memo.** The spec column
is derived from the cited steps; the HEAD column from code reading at `593aa314` (not run).

| id | expression (fresh page; `tpl = document.createElement('template')` unless stated) | spec (derivation) | elidex HEAD | elidex C1 head | Chromium | other engine |
|---|---|---|---|---|---|---|
| M1 | `tpl.content.ownerDocument !== document` | `true` (§4.12.3 establishing step 1) | `false` | `true` | | |
| M2 | `tpl.content.ownerDocument === document.createElement('template').content.ownerDocument` | `true` (one associated inert template document) | `true` | `true` | | |
| M3 | `I = tpl.content.ownerDocument; [I.defaultView, I.URL, I.baseURI, I.documentURI, I.body, I.ownerDocument]` | `[null, 'about:blank', 'about:blank', 'about:blank', null, null]` (browsing context null; URL is the default; §2.4.3 fallback base URL) | n/a (I is `document`) | the spec values, given C1-pre's Document install and per-entity facts (URL: C1-pre writes and reads it; edge c-pre) | | |
| M4 | `tpl.innerHTML = '<a href="x.html"></a>'; tpl.content.firstChild.href` | the `href` content attribute value `"x.html"` (base `about:blank`, §2.4.3; `x.html` fails URL §4.4 *basic URL parser* against it, so url is null and HTML §4.6.3's `href` getter step 4 returns "this's href content attribute's value") | absolute against the page base | `"x.html"` | | |
| M5 | parser path: page markup `<template id=h><a id=a href="x.html"></a><img id=i src="y.png"></template>`; `[h.content.getElementById('a').href, h.content.getElementById('i').src]` | `["x.html", "y.png"]`-shaped (`href` as M4; `src` by HTML §2.6.1's URL-reflecting getter: step 3.3 returns urlString only if not failure, else step 4 the content attribute value) | absolute | as M4 | | |
| M6 | `c = tpl.content.cloneNode(true)` with M4's `<a>`; `[c.ownerDocument === tpl.content.ownerDocument, c.firstChild.href]`; `document.body.append(c)`; `a.href` | `[true, "x.html"]`, then absolute | `[true, absolute]`, then absolute | spec | | |
| M7 | `t2 = tpl.cloneNode(true); t2.content.ownerDocument === tpl.content.ownerDocument` | `true` (copy created in `document`; establishing step 1) | `true` (both `document`) | `true` (both I) | | |
| M8 | `dp = new DOMParser().parseFromString('<template id=t></template>','text/html'); x = dp.getElementById('t').content.ownerDocument; [x !== dp, x !== tpl.content.ownerDocument]` | `[true, true]` | `[false, true]` | `[true, true]` | | |
| M9 | `dc = document.cloneNode(true)` (page with a `<template>`); `x = dc.querySelector('template').content.ownerDocument; [x !== dc, x !== tpl.content.ownerDocument]` | `[true, true]` | `[false, true]` | `[true, true]` | | |
| M10 | nested: `<template id=o><template id=i></template></template>`; `[i.ownerDocument === o.content.ownerDocument, i.content.ownerDocument === o.content.ownerDocument]` | `[true, true]` | `[true, true]` (all `document`) | `[true, true]` (all I) | | |
| M11 | `tpl.innerHTML = '<b id=x onclick="window.hit=1"></b>'; b = tpl.content.firstChild; [b.onclick, (b.dispatchEvent(new Event('click')), window.hit)]` | `[null, undefined]` (*get the current value of the event handler*: browsing context null) | `[function, 1]` | `[null, undefined]` | | |
| M12 | `<template id=s><div id=h><template shadowrootmode=open><span></span></template></div></template>`: `[s.content.getElementById('h').shadowRoot !== null, …shadowRoot.ownerDocument === s.content.ownerDocument]` | DSD attaches inside contents when the parser's flag is set (§13.2.6.4.4); then `[true, true]` | `[true, true]` (both `document`) | `[true, true]` (both I) | | |
| M13 | custom element in contents: `customElements.define('x-a', class extends HTMLElement { adoptedCallback(o, n) { log.push([o, n]) } })`; `e = document.createElement('x-a'); tpl.content.append(e); log` | `[[document, I]]` (insert step 7.1 adopts into the contents' node document; `e` stays custom) | `[]` | `[[document, I]]` (needs C0c's `Adopt`) | | |

M3, M4–M6 and M13 are the rows where elidex's planned value is a reading (M3's unbound-Document methods, M4–M5's
unparsable relative URL, M13's custom element staying custom in I). The remaining rows confirm the spec reading.

## §2. Coupled invariants

### §2.1 Invariant per axis

| axis | invariant |
|---|---|
| A1 rule | For every Document D, `f(D)` is one Document I carrying `InertTemplateDocument`, the same entity on every call; `f(I) = I`. No site computes a template contents' node document except through `f` (D-C2, D-C3) |
| A2 creation | At every template creation the contents fragment's node document is `f(template's node document)`. Every node a parser creates takes its owner at creation from C0's new-node entry (C0 draft D-10, D-11), which inside template contents returns I, in both tiers, in document, fragment and DOMParser parses (D-C3, D-C4) |
| A3 nesting | A template at any depth inside contents of D's templates has node document I and contents node document I; there is one I per non-inert Document, never an I of I |
| A4 shadow | A shadow root inside template contents (declarative or `attachShadow`) and its content report I (C0's host-link stamp, A2's entry); the adopt and clone walks reach them under the contents' document |
| A5 clone | The copy's contents are established in `f(pair document)`; their children, and every pass for a pair among them, use `f(pair document)`; a clone of contents is in I; a clone of D or of I is a non-inert Document with no link |
| A6 adopt | Adopting a template into X (C0's insertion adopt or `adoptNode`) re-homes the template and its shadow-including descendants to X; contents: see D-C5 |
| A7 lifetime | I is created on first need and destroyed exactly when D is; no fragment parse, strict parse error or clone creates an I for a throwaway document |
| A8 consumers | Every reader of a node's document reads I for a node in template contents through `node_document` (the `ownerDocument` getter, the base URL, N1's compile gate, T3's step 3 after C1); no render / layout / hit-test root discovery visits I |
| A9 EH contract | K-0 and S-3 hold; the bind pass still walks no template contents (K-5, K-6, U8 (ii)); S-1's template-contents extension and S-2's branch-A read-null are satisfiable on C1's head (§4.6); T3's R-A premises hold at C1's head (§9 item 9) |

### §2.2 All pairs (36), with intersection

| pair | intersect? | intersection |
|---|---|---|
| A1×A2 | yes | Four establishing callers pass a node document, the helper maps it (D-C3). A caller that passes the template's own document unmapped is C2 (T1, T3) |
| A1×A3 | yes | Idempotence: a nested template passes D or I and gets I (T4, T5). A rule without the marker check creates I-of-I (U2) |
| A1×A4 | yes | A shadow root in contents takes its host's node document; the rule is never applied to shadow roots (only templates have contents) (T12) |
| A1×A5 | yes | The copy's contents come from `f(pair document)`; for a contents clone the pair document is I and `f(I) = I` (T10, T24) |
| A1×A6 | yes | Adopt maps each template's contents to `f(target)`, once per template reached (D-C5; T13, U9) |
| A1×A7 | yes | Lazy creation inside `f`; destruction cascades from D, so `f` never returns a destroyed I while D lives (U3, U4, U7) |
| A1×A8 | yes | Consumers read the result through `node_document`; no consumer calls `f`. One node-document reader (C0), one new-node entry (D-C4), one contents rule (D-C2): three functions for three facts, not three for one |
| A1×A9 | yes | `is_inert_template_document(node_document(n))` is the branch-A template-origin fact (S-2), derived from the node document, so it tracks moves (a node moved out loses it through C0's adoption) |
| A2×A3 | yes | Both tiers call C0's entry with the intended parent per node (C0 draft D-11, D-20), whose node document is I inside outer contents; the inner template's contents get `f(I) = I` (T4, both tiers) |
| A2×A4 | yes | Declarative shadow content inside contents: the strict DSD path (`attach_declarative_shadow`, `shadow.rs:136`) and the tolerant DSD recursion (`try_attach_declarative_shadow`, `convert.rs:120`) create the shadow content with the entry's value for the shadow root, which C0 stamped with the host's node document I (T12; Q-S2). A tolerant **document** parse attaches no declarative shadow root (`#11-tolerant-document-declarative-shadow-roots`), so the tolerant half holds for fragment parses only |
| A2×A5 | yes | The clone post-pass is an establishing caller (D-C3) and site A10 of the new-node census: like every creation path it calls the entry, but on a Document (the operation's, or for a pair inside copied contents the pair's, D-C6; a Document → itself, C0b U11), never on a source node, so a clone's document is the one it is cloned with (DOM §4.4 *clone a single node* "given document"), not its source's (§4.4) |
| A2×A6 | yes | Fragment build: every node under the fragment-parse root holds the context's document from creation (C0 draft D-10), so no build link adopts and no finishing walk runs (D-8); contents fragments are stamped `f(context doc)` at attach (T14 parity, both tiers) |
| A2×A7 | yes | No `f(throwaway)` is ever computed (D-C4), so neither the success path nor the strict rollback (`despawn_subtree(document)`, which reaches contents with `Include`) creates or leaks an I (U4, U5) |
| A2×A8 | yes | DOMParser: `dp`'s templates' contents are in `f(dp)`, a third document, read null (N1) and, after T3, target null, exactly as `dp`'s own nodes (T6) |
| A2×A9 | yes | The pre-bind parse puts contents in I before bind; slice a's bind pass still excludes them (U8 (ii) unchanged). Slice b's S-1 extension processes them at creation-equivalent time; C1 gives them a non-bound node document so their raw values read null (§4.6) |
| A3×A4 | yes | A template inside a shadow tree inside contents, and a shadow host inside nested contents: all report I (T12 variant) |
| A3×A5 | yes | Deep clone of an outer template clones the inner template inside the copy's contents with pair document I; the inner copy's contents are `f(I) = I` (T10) |
| A3×A6 | yes | Adopting an outer template into X moves outer contents and every nested template and nested contents to `f(X)`; the target switches once, at the first contents edge (U9) |
| A3×A7 | no | Nesting creates no further document (A1×A3) |
| A3×A8 | yes | Nested contents read null and (after T3) target null like outer contents (same I) |
| A3×A9 | no | The EH contract does not distinguish depth inside contents |
| A4×A5 | yes | A clonable shadow host inside copied contents: its host, shadow root and shadow children are in the pair's document I, not the operation's (§4.4 step 3) (T24). D14's subtree-true form is assumed (§4.4) |
| A4×A6 | yes | Adopt's walk is shadow-including under each target: a shadow root inside contents goes to `f(X)`, a shadow root of the template's light descendants to X (U9) |
| A4×A7 | no | Shadow roots create no document |
| A4×A8 | yes | `shadowRoot.ownerDocument` in contents reads I; a handler on a shadow-root-direct child in contents reads null under N1 (no Insert fires there, H2, so it is processed only by slice b's creation path) |
| A4×A9 | yes | Slice a's G2 window (shadow-root-direct elements not processed) is unchanged by C1; in contents, the spec read is null either way |
| A5×A6 | yes | `tpl.content.cloneNode(true)` is in I; appending it into the document adopts it (C0) to D (§1 row 1, T9) |
| A5×A7 | yes | Cloning D creates no I until a template in the copy needs one; the link is not copied (D-C8; T11, U8) |
| A5×A8 | yes | A detached contents clone reads null for raw handlers (N1: node document I ≠ bound) and the spec function after the append (adopted to D) |
| A5×A9 | yes | S-1 processes clones at creation (T3 draft D-T10 moves the clone path into T3, after C1); a clone of contents is in I, so its raw values read null until inserted, which is S-2's branch-A outcome. The umbrella's §9 N-2 problem (no per-node fact) does not arise: the fact is the node document |
| A6×A7 | yes | Adopting all templates out of D's contents leaves I alive (D lives); an adopt never targets a throwaway's I (none exists, A2×A7) |
| A6×A8 | yes | After adoption every consumer reads the new document through `node_document` (C0) |
| A6×A9 | yes | C0's adoption precedes the Insert event (C0 draft D-4), so slice a's Insert arm (T-a2) and N1's gate see the adopted document; a node placed into contents is processed at T-a2 with node document I and reads null (W-K5's template half closes; R-3) |
| A7×A8 | yes | A node outliving its I reads `ownerDocument` null through `node_document`'s dangling guard; I is never a render / layout / hit-test root (D-C9; U6) |
| A7×A9 | no | The EH contract never reads I's lifetime |
| A8×A9 | yes | N1's node clause reads I for contents: umbrella C14 inverts (no run, `null`; T17). The base's target rule ignores the document, so C18 is unchanged at C1 (T18) and T3 rewrites it; T3's R-A target cells (T-17, EW-13, EW-14, XW-8) rest on C1's facts (§9 item 9) |

Count: 36 pairs; 32 intersect, 4 do not (A3×A7, A3×A9, A4×A7, A7×A9). Rev 1's fifth non-intersecting pair, A2×A5,
intersects in rev 2 (the clone post-pass is a creation site of the new-node census). Re-count:
`awk '/^### §2.2/{f=1} /^## §3/{f=0} f && /^\| A[0-9]×A[0-9]/' <memo> | awk -F'|' '{print $3}' | sort | uniq -c`.

## §3. Spec coverage map

Lookups (2026-11-01): `.claude/tools/webref heading html 4.12.3` (The template element), `heading html 2.4.3` (Document
base URLs), `heading html 13.2.6.1` (Creating and inserting nodes), `heading html 13.2.6.4.4` (The "in head" insertion
mode), `heading html 13.4` (Parsing HTML fragments), `heading html 8.5.1` (The DOMParser interface), `heading html 8.5.4`
(The innerHTML property), `heading html 8.1.8.1` (Event handlers), `heading dom 4.2.3` (Mutation algorithms), `heading
dom 4.4` (Interface Node), `heading dom 4.5` (Interface Document), `heading dom 4.8` (Interface ShadowRoot); step text
from `body html the-template-element` (establishing steps 1–3, *appropriate template contents owner document* 1–2,
adopting steps 1–2, cloning steps 1–2), `body dom concept-node-adopt` (steps 1–3, 3.1–3.4: 3.3.3 enqueues
`adoptedCallback` « oldDocument, document », 3.4 runs the adopting steps inside the per-descendant loop), `body dom
concept-node-clone` (steps 2, 3, 5, 6.7 "subtree set to true"), `body dom concept-node-insert` (step 7.1 "Adopt node into
parent's node document"; replace step 6), `body html create-an-element-for-the-token` (step 3 "Let document be
intendedParent's node document"; text / comment: "targetParent's node document"), `body html fallback-base-url` (steps
1–2). Rev 3.5 (2026-11-04): `heading dom 4.9` (Interface Element), `body dom concept-attach-a-shadow-root` (step 5),
`body html getting-the-current-value-of-the-event-handler` (step 3.2), `body html parsing-html-fragments` (steps 14,
15, 22), `dfn html createContextualFragment` (§8.5.7) and `body html dom-range-createcontextualfragment` (steps 1, 6),
`body html dom-hyperlink-href` (step 4; `dfn html 'reinitialize url'` → §4.6.3), `dfn url 'basic URL parser'` (§4.4),
`body html reflect` (§2.6.1, URL getter steps 3.3, 4), `body dom clone-a-single-node` (step 5, Document 5.1),
`body dom dom-document-adoptnode` (steps 1–4).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.12.3 The template element | establishing the template contents, steps 1–3 | 1 document = node document's appropriate template contents owner document; 2 fragment with that node document and host = template; 3 link | `EcsDom::attach_template_contents` `crates/core/elidex-ecs/src/dom/mod.rs:557` (D-C3); step 2's *host* is §7 | ✗ (host not modelled) | yes |
| WHATWG HTML §4.12.3 The template element | *appropriate template contents owner document*, steps 1–2 | 1 not created by this algorithm → get-or-create the associated inert template document (1.1.1 create, browsing context null; 1.1.2 HTML flag; 1.1.3 link), set document to it; 2 return | `appropriate_template_contents_owner_document` (NEW) in `crates/core/elidex-ecs/src/dom/template_owner.rs` (NEW) (D-C2); 1.1.2 vacuous (H-C13) | ✓ | no |
| WHATWG HTML §4.12.3 The template element | template *adopting steps*, steps 1–2 | document = node's new node document's appropriate owner; adopt contents into it | C0's adopt walk (`EcsDom::adopt_subtree`, `crates/core/elidex-ecs/src/dom/tree/teardown.rs:251` at `593aa314`; moved by C0 D-3): its contents edge's target and segment `new` (D-C5); C0's `Adopt` consumer unchanged by C1 | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | template *cloning steps* (node, copy, subtree), steps 1–2 | subtree false → return; else clone each contents child with document = copy's contents' node document, subtree true, parent = copy's contents | `replicate_template_contents` `crates/dom/elidex-dom-api/src/node_methods/clone.rs:405` (D-C6) | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | `content` getter | return the template contents | `crates/script/elidex-js/src/vm/host/html_template_proto.rs:106` (unchanged) | ✓ | no |
| WHATWG DOM §4.5 Interface Document | *adopt* steps 1–3 | 3 document ≠ oldDocument → for each shadow-including inclusive descendant: 3.1 set node document; 3.2 / 3.3.1–3.3.2 registries and attributes (not C1's); 3.3.3 `adoptedCallback`; 3.4 adopting steps | C0b's `adopt` / `adopt_subtree` (C0 D-2, D-3) and C0c's `MutationEvent::Adopt` (NEW: C0c) consumer (C0 D-6); C1 edits only the contents edge's target, hence the contents segments' `new` (D-C5) | ✗ | yes |
| WHATWG DOM §4.2.3 Mutation algorithms | *insert* step 7.1; *replace* step 6 | adopt node into parent's node document | C0 (C0 D-4); not touched by C1 | ✗ | yes |
| WHATWG DOM §4.4 Interface Node | *clone a node* steps 2, 3, 5, 6.7 | 2 copy created in document (runs the establishing steps); 3 cloning steps; 5 children with the same document; 6.7 shadow children with the same document, subtree true | `clone_node_with_shadow_honor` `clone.rs:227` (D-C6, on D14), `clone_subtree` (unchanged by C1; its document is a `NewNodeOwner`, C0b D-11) | ✓ | yes |
| WHATWG DOM §4.4 Interface Node | *node document*; `ownerDocument` getter | a node's node document; a document's is itself; getter null for a document | C0's `EcsDom::node_document` (NEW: C0) and `owner_document` `mod.rs:817`; getter `crates/dom/elidex-dom-api/src/node_methods/core.rs:279` (unchanged by C1; new values) | ✓ | no |
| WHATWG HTML §13.2.6.1 Creating and inserting nodes | *appropriate place for inserting a node*; *create an element for a token* step 3; text and comment insertion | a template target parent → its template contents; document = intendedParent's node document; text / comment = targetParent's node document | C0's `EcsDom::node_document_for_new_nodes` (NEW: C0) and its migrated parser call sites (C0 draft D-10, D-11, §4.4; at `593aa314` the strict `TreeBuilder` method `crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs:271` and tolerant `convert_node` `crates/dom/elidex-html-parser/src/convert.rs:237`); C1 changes no site, only the value inside contents (D-C4) | ✓ | yes |
| WHATWG HTML §13.2.6.4.4 The "in head" insertion mode | start tag "template", the declarative branch ("Keep") | the declarative shadow root attaches when the parser's allow-declarative-shadow-roots flag is true, inside template contents too | strict `attach_declarative_shadow` `tree_builder/shadow.rs:136`, `attach_ordinary_template_contents` `:123` (D-C3); tolerant `try_attach_declarative_shadow` `convert.rs:120` | ✗ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | *HTML fragment parsing algorithm* steps 14, 15, 22 (configuration null) | templates parsed in the build get the context's inert document (C0's root redirect, D-C4); the caller's insertion adopt | `begin_detached_fragment` `teardown.rs:293` (C0b inserts `FragmentParseRoot` on the root, C0 draft D-10), `finish_detached_fragment` `teardown.rs:330` (its re-home deleted, D-8); tolerant `crates/dom/elidex-html-parser/src/lib.rs:158` (deleted, D-20); C1: D-C4's value | ✗ | yes |
| WHATWG HTML §8.5.4 The innerHTML property | setter on a template: the target is its template contents | parsed nodes replace the contents' children (inserted, so adopted into I by C0) | `crates/script/elidex-script-session/src/mutation/html_fragment.rs` (unchanged; C0 adopts) | ✗ | yes |
| WHATWG HTML §8.5.1 The DOMParser interface | `parseFromString` `text/html` | the new document is "not created by this algorithm": its templates get its own inert document | `parse_into_inert_document` `crates/dom/elidex-form/src/inert_document.rs:110` (unchanged; values change via D-C3) | ✗ | yes |
| WHATWG HTML §2.4.3 Document base URLs | *fallback base URL* steps 1–2 | I's URL is `about:blank` and its about base URL is null → base `about:blank` | `DocumentBaseUrl(about_blank_url())` from `create_document_node` `mod.rs:410`; read by `href_accessor.rs:50` (§1.1 M4–M6) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *get the current value of the event handler* step 3.2; *determine the target of an event handler* step 3 | raw value in a document whose browsing context is null → null; node document not an active document → null target | N1's compile gate (C0's `node_document` form of clause (b), `crates/script/elidex-script-session/src/scripting.rs:108-122` at `27b50dfe`; doc `:73-74` rewritten); T3 (after C1) | ✗ | yes |
| WHATWG DOM §4.9 Interface Element | *attach a shadow root* step 5 | the shadow root is created given the host's node document | C0b's `attach_shadow_with_init` → `node_document_for_new_nodes(host)` (C0b D-11); C1 changes no site, only the host's value inside contents (D-C4, §4.4 step 3) | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *shadow-including inclusive descendant* | adopt's walk order (contents excluded; reached through step 3.4) | `for_each_uncapped_shadow_inclusive` `crates/core/elidex-ecs/src/dom/tree/walkers.rs:99` (C0c reorders `Include`, C0 draft D-19; C1 leaves it unchanged) | ✓ | no |

### §3.1 User-input touch audit

Rows marked *yes* carry author markup or script-constructed trees. C1 parses no new input and stores no string: the
two components hold an `Entity` and nothing. What changes is which Document a node of author content is attributed
to, and with it four reads: the `ownerDocument` getter (a new Document wrapper becomes reachable from script: I, whose
browsing context is null; §1.1 M3, edge c-pre), URL resolution (relative URLs in template contents no longer resolve
against the page, §1.1 M4–M6), the handler gates (raw handlers in template contents read null and do not run: the
spec's outcome, more restrictive than HEAD), and `adoptedCallback`'s arguments for custom elements in contents (§1.1
M13). No path delivers author content to a document other than one it was parsed into, cloned into or adopted into.

## §4. Mechanism

### §4.1 Home / kind

- The rule, the predicate and the lifetime hook live in **`elidex-ecs`**
  (`crates/core/elidex-ecs/src/dom/template_owner.rs` (NEW), an `impl EcsDom` block): both parser crates and
  `elidex-dom-api` call them, and the parser crates cannot depend on `elidex-dom-api` (umbrella §9 N-1's crate-graph
  evidence). C0's new-node entry lives in `elidex-ecs` for the same reason (C0 draft D-10).
  `elidex-ecs` is engine-independent, so the Layering mandate holds. C1 adds no VM code; I's Document surface (the
  Document install on its wrapper and its per-entity facts) comes from C1-pre (edge c-pre).
- Components (D-C1) in `crates/core/elidex-ecs/src/components.rs` beside `TemplateContents` (`:366`).
- Realm: §4.12.3 step 1.1.1 creates I "given document's relevant realm". elidex has one realm per `EcsDom` (T3 draft
  H-T15); B1's multi-realm World must create I in D's realm (§8 H-3).

### §4.2 The rule and the establishing helper (D-C2, D-C3)

`appropriate_template_contents_owner_document(D)`: marker on D → D; link on D → its target (a `debug_assert!`
pins that it is a live marked Document); no link → spawn with `create_document_node`, insert `InertTemplateDocument`, insert
`AssociatedInertTemplateDocument(I)` on D, return I. The rule has no re-spawn branch: no production path destroys I
(H-C12), and D-C7's cascade from D takes the link with D (R-9). `attach_template_contents` calls the rule on `Some(node_document)`, then creates
the fragment with that owner, as today.

### §4.3 The new-node entry (D-C4): C0b's

The entry, its signature (one `source`; `NewNodeOwner`; the redirection by `FragmentParseRoot` on the fragment-parse
root), the deletion of the strict tree builder's method and the migration of every parser and dom-api creation site
are C0b's (C0 draft D-10, D-11, D-17, D-20, §4.4; PM decisions 223 O-2 / 227). C1 changes none of them. On C1's head the
entry returns I inside template contents because D-C3 stamps the contents fragment with I (D-C4). Rev 3's constraint
on C0's Q-2 (a tolerant value threaded from the template's level, as `convert.rs:284-286` does today, would stamp
contents' children with D) is met by construction: Q-2 is decided per node and the threaded owner is deleted (C0 draft
D-20). The tolerant legs of T3, T12, T19 and U12 still kill that form (§6.3).

### §4.4 Clone (D-C6), on D14

On C1's base D14 has rewritten pass 4's child loop (`clone.rs:290-302` at `593aa314`) to clone shadow children with
subtree **true** (DOM §4.4 step 6.7; P-C12), and pass 4 has no hand stamp: C0b removes `clone.rs:282-285` (host and
shadow root) and D14 or C0b, by landing order, `:294-301` (C0b D-18, edge c0g). C1 then:
1. gives each worklist pair the `NewNodeOwner` its copy was made with (`pair_owner`): the root pair `shadow_doc` (a
   `NewNodeOwner` on the base, C0b D-11), every pair produced under a pair P the owner P's children were cloned with; an
   ownerless pair carries the entry's `None` (C0b D-11);
2. in pass 3, `replicate_template_contents(s, d, …, pair_owner, …)`: `new_fragment = attach_template_contents(d,
   pair_owner)` (mapped inside, D-C3), then clones each source contents child with `node_document_for_new_nodes(new_fragment)` (= `f` of the pair's document; `clone_subtree`'s
   override is a `NewNodeOwner`, §11 Q-4) and records those pairs with that owner;
3. in pass 4, clones the shadow children with `pair_owner` (`clone.rs:294` at `593aa314`). The host and the shadow root
   need no step: the host was made with `pair_owner` by its pair's clone (step 2 inside contents), and the shadow root takes the host's document at
   attach (`attach_shadow_with_init` → `node_document_for_new_nodes(host)`, C0b D-11, H-b3).
Whether the pair document is a third tuple field or read back from the stamp the deep cloner wrote (`tree_clone.rs:221`,
`:417`) depends on whether the shallow root is stamped on the base (`#11-ownerless-node-creation`'s shallow-clone site,
L3's D8): if every copy is stamped at creation, reading `node_document_for_new_nodes(d)` suffices; otherwise the field. *Rev 3:* C0
draft D-11 makes `clone_node_shallow` stamp the operation's document, so on the base every copy is stamped at creation
and the read-back form is available (Q-S12's second half answered); the choice stays implementation input. Edge c-RC2a
(§8) records the rebase.
§4.4's per-pair document (D-C6, cell T24) is written on D14's form; it must be re-checked against RC-2a's final shape (the
cloneNode migration and the CloneCreated consumers, the Range program) before C1's implementation starts.

### §4.5 L3's Range creation sites (A10–A12) and the rest of the census

- **A10** (template clone; `#11-template-contents-owner-document`'s scope) is §4.4.
- **A11 / A12** (L3 RC-4's body context element and result fragment of `createContextualFragment`, planned to read the
  new-node entry; the method is a stub at `593aa314`, `crates/script/elidex-js/src/vm/host/range_proto_mutation.rs:250`):
  C1's obligation is that the entry returns the right document for a Range whose start node is in template contents,
  which is I — the start node's node document (HTML §8.5.7 *createContextualFragment()* step 6: "set element to the
  result of creating an element given node's node document, "body", and the HTML namespace", node = this's start node,
  step 1).
  That holds by A2 / A5 / A6 (every node in contents has node document I) with no Range-specific branch; unit cell U11
  pins the entry's value; RC-4's own cells pin the end-to-end shape.
- **A1–A9** and the dom-api creation sites that decide an owner from `owner_document(this)` today
  (`element/tree.rs:431` `insertAdjacentText`, `node_methods/text_content.rs:119`, `char_data/split_text.rs:148`,
  `cssom_sheet.rs:213`, `element/select.rs:326`; `createElement`'s own element) are `#11-ownerless-node-creation`'s
  census, owned by C0 rev 2 / L3 (ledger: "L3 sends the list of Range sites into C0 rev 2"). Their values become I for a
  receiver in contents through C0's `node_document`; C1 changes none of them (§7).

### §4.6 What C1 provides to slice b (S-1's template-contents extension, S-2 branch A) and to T3

1. **Node document, at creation.** On C1's head every node in template contents (every creation path of A2, every clone
   of A5, every node placed there by C0's adoption) has node document I before any script can observe it. Slice b needs
   nothing else to make its creation-time raw values read null (N1's node clause: `node_document(n) = I ≠ bound`), and
   T3 needs nothing else to make Window-delegating pairs target null (step 3: I is not the active document).
2. **The template-origin fact.** `is_inert_template_document(node_document(n))` (NEW). Carried by a clone and a clone of
   a clone (A5), lost by a node moved out of contents (C0 adopts it): S-2's "tracks the node's spec node document, not
   its history".
3. **Pre-bind template contents.** S-1 extends to them in branch A; the walk that reaches them is the existing uncapped
   walker with template contents included (H-C15's `Exclude` stays for slice a's K-5 / K-6). C1 adds no walk.
4. **No insertion fallback.** C1 adds no processing at insertion (slice a's Insert arm is unchanged), so S-4 is not
   weakened.

### §4.7 What C1 takes from C0 (C0 draft §4.6)

| C1 needs | C0 draft's answer (PR) | C1's use |
|---|---|---|
| one reader of a node's node document | `EcsDom::node_document` (NEW: C0b) (D-1); `owner_document` = getter wrapper | every read in C1's code (D-C10) |
| every insertion adopts, before `Insert` | D-4 at the three primitives; D-5 for replace-with-fragment (C0b) | §1 rows 1–5; T7, T8, T9, T13 |
| a contents fragment's node document is the parent document for nodes placed into it | `node_document(fragment)` = its stamp (C0b) | T7, T8 |
| the one adopt walk to edit | D-3: `adopt_subtree` (moved to `mutation.rs`), `Include` walk, one contents edge (C0b); segments (D-3 (c), C0c) | see D-C5 |
| one adoption notification | D-6, D-19 (C0c): one `MutationEvent::Adopt` per adopt call with per-segment (old, new) pairs; consumer in spec order | D-C5 sets the segment's `new`; no consumer change (T25) |
| one new-node owner entry, every creation site on it | D-10, D-11, D-17, D-20 (C0b); hook = `attach_template_contents`'s second argument (`NewNodeOwner`) | D-C3 applies `f` in the helper; D-C4's value inside contents |
| no `f(throwaway)` | D-8, D-10 (C0b): no build node but the root holds the throwaway; no build link adopts | A2×A6, A2×A7 |

### §4.8 ECS-native and Layering checks

- Representation: two components, no side-store (D-C1). Lifetime by despawn cascade, not refcount or sweep (D-C7).
- One rule (D-C2), one establishing helper (D-C3), one new-node entry (C0's, D-C4), one adopt walk (D-C5, C0's), one
  reader (C0's `node_document`): §5.1's populations converge on them.
- VM: C1 adds no VM code (§4.1); `html_template_proto.rs` keeps marshalling the link (`:106`).

### §4.9 Docstring citation table (Axis 4)

| symbol | citation |
|---|---|
| `appropriate_template_contents_owner_document` (NEW) | HTML §4.12.3 *appropriate template contents owner document*, steps 1–2 (1.1.2 vacuous: no XML documents) |
| `AssociatedInertTemplateDocument` (NEW) | HTML §4.12.3 *associated inert template document* |
| `InertTemplateDocument` (NEW) | HTML §4.12.3 ("a Document created by this algorithm") |
| `is_inert_template_document` (NEW) | HTML §4.12.3 (the same fact) |
| `attach_template_contents` (doc rewrite) | HTML §4.12.3 establishing steps 1–3 (step 2's host not modelled, §7) |
| C0's adopt walk (doc rewrite of the contents edge) | DOM §4.5 *adopt* steps 3.1, 3.3.3, 3.4; HTML §4.12.3 template *adopting steps* |
| `replicate_template_contents` (doc rewrite) | HTML §4.12.3 template *cloning steps* step 2 |
| `destroy_entity`'s Document cascade (D-C7) | no spec counterpart (the spec's lifetime is GC) |
| clone-policy rows of both components (D-C8) | DOM §4.4 *clone a single node* step 5, Document 5.1 (the copied fields do not include the associated inert template document) |
| `root_entities`' exclusion (D-C9) | no spec counterpart (root discovery is elidex's); its reason is HTML §4.12.3's note "so that they aren't in a browsing context and thus remain inert" |

## §5. Populations, touched files, named risks

### §5.1 Populations (re-runnable)

Filter for production code everywhere: `NT='/tests/\|tests\.rs\|/tests_\|test_helpers'`.
- **P-1 establishing sites** (H-C2): `/usr/bin/grep -rnF 'attach_template_contents(' crates --include='*.rs' |
  /usr/bin/grep -v "$NT"` → 4 callers + definition (verified 2026-11-01). All four keep their call and pass the
  template's node document (D-C3).
- **P-2 adopt**: `/usr/bin/grep -rnF 'adopt_subtree(' crates --include='*.rs'` → definition + 1 production caller + 1
  test at `593aa314`; C0 adds `adopt` and moves the function.
- **P-3 clone stamps**: `/usr/bin/grep -rnF 'set_associated_document(' crates --include='*.rs' | /usr/bin/grep -v "$NT"`
  → `tree_clone.rs:221`, `:230`, `:417`, `clone.rs:283`, `:284`, `:299`, `teardown.rs:257`, definition `mod.rs:789`
  (verified 2026-11-01). On C1's base the three `clone.rs` calls are gone (C0b D-18, edge c0g); C1 adds none (D-C6).
- **P-4 parser owner decisions** and **P-5 parser creation sites**: **moved to C0 (rev 3)**. They are C0's creation
  census and its ratchet (C0 draft D-11, §4.4; C0b memo §5.1 P-3, P-8; PM decisions 223 O-2 / 227). C1 neither adds nor
  changes a parser creation site. At C1's base C1 re-runs C0's P-8 (0 production ownerless creations) and C0's P-3 (no
  `owner_document(` in either parser crate) as base checks only (§9 item 1).
- **P-6 node-document consumers** (values change for nodes in contents, code unchanged by C1): C0's P-3
  census (C0 draft §5, list in the C0b memo §5.1; at C0b's head `node_methods/core.rs:279` and `selection.rs:618`
  only, the rest read `node_document`, C0 draft D-1, D-7). Re-run at C1's
  base: `/usr/bin/grep -rn 'node_document(\|owner_document(' crates --include='*.rs' | /usr/bin/grep -v "$NT"`.
  Scout Q-S4's classification (2026-10-31) found no reader whose new value for a node in contents is not the spec's
  except `documentURL` on I (C1-pre's, edge c-pre), href paths' `document_root()` fallback on I (C0b D-7 (c)) and `focus/delegate.rs:85`
  and `scripting.rs:109` (`:120` at `27b50dfe`), marked "needs check" — that read is C0's (D-7 (a)); `focus/delegate.rs:85` is
  Q-S11.
- **P-7 C2 statements**: `/usr/bin/grep -rn 'template-contents-owner-document\|C2 approximation' crates` →
  `mod.rs:550`, `:552`, `scripting.rs:74` (at `27b50dfe`), `document.rs:261-262`, `elidex-html-parser/src/tests.rs:566` (verified
  2026-11-01); all rewritten (§5.2).
- **P-8 Document destruction**: H-C12 (2 production paths, scout Q-S1); U7 pins the cascade.
- **P-9 render / layout / hit-test roots**: H-C11 (4 consumers); D-C9 changes the source only.

### §5.2 Existing tests and docstrings whose expectation changes (red at C1 by design, rewritten in the same commit)

Tests (scout Q-S8, re-located at `593aa314` with `/usr/bin/grep -n` on the function names):
`crates/dom/elidex-html-parser/src/tests.rs` `template_content_descendant_owner_document` (`:542`, both assertions:
contents and their `<p>` → I, not the document) and `fragment_parsed_template_content_owner_is_context_document`
(`:571`: the fragment and its `<p>` → the context document's I; the test is renamed to say so);
`crates/dom/elidex-dom-api/src/node_methods/tests/clone.rs` `clone_node_template_shallow_preserves_owner_document`
(`:214`: the shallow copy's `content` → the document's I); `crates/core/elidex-ecs/src/dom/tests/associated_document.rs`
and `dom/tests/traversal.rs:18` (`attach_template_contents(template, Some(doc))`: still compiles, value changes); slice
a's VM tests holding umbrella cell C14 (`crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs_once.rs`,
inverted, T17); and C0's cells whose expected value reads a template contents' (or a contents descendant's) node
document, or the pair of an adopt segment of contents (list below). C18's test (`tests_event_handler_attrs_window_delegating.rs`) is **unchanged** at C1 (T18). The complete
list is re-derived at C1's base (C0 and D14 change neighbouring tests).

*C0 cells C1 changes (by property, not by a "C1 turns" phrase).* Population: every §6.1 / §6.2 cell of the C0a, C0b and
C0c memos whose script, shape or expected value involves template contents — `for m in <C0a> <C0b> <C0c>; do awk
'/^## §6/{f=1} /^### §6\.5/||/^## §7/{f=0} f && /^\| *[TU][0-9]+/' "$m" | /usr/bin/grep -iE 'template|content'; done`
(the program memo has no cells, its §1.1; §6.5 base formulations run at C0's bases, not C1's) — then each is classified
by whether its expected value depends on the contents' node document or a contents segment's pair. At the frozen C0
frame (C0b / C0c memos as of `a93142c4`; the same at `05142d8c`) the population is C0b T5, T23, U1, U8, U11, U12 and C0c T18, U13, U18, U19:
- **changes (rewritten in C1's commit)**: C0b **T5** (both legs → the inert document, C1's T13 value); C0b **U1** (the
  template-with-contents variant: contents stamped `f(D)`; the "append into a contents fragment" variant's fixture, if
  built through `attach_template_contents`, reads `f(D)`); C0b **U8** (×2; "every node except the Document reads `D`" →
  nodes in contents read `f(D)`; the walk-counter leg is unchanged, C1's U12); C0b **U11** (the top-level-template and
  nested-template legs → `f(context's document)`); C0c **U13** (C's segment pair → « `f(old)`, `f(new)` »); C0c **U19**
  (first leg → `Adopted(X, f(target))`; second leg: the outer contents in the target are no longer skipped, `target ≠
  f(target)`, so the leg is re-set with the outer contents in `f(target)` to keep constructing the skip; third leg:
  stamped `f(target)`, its condition `X ≠ f(target)`);
- **value unchanged**: C0c **T18** (its segment's `new` becomes `f(dp2)`, but the logged `n === dp` is `false` either
  way; its "C1 turns" note is a comment; C1's T25 discriminates);
- **not dependent**: C0b T23 and U12 (no contents in the expected value), C0c U18 (walk order only). C0a: no cell in the
  population.
The population is re-run at C1's base against the landed C0 memos.
Docstrings: P-7, plus `components.rs:354-364` (`TemplateContents`' adopt sentence), `attach_template_contents`'s
"strict: post-insert `owner_document(template)`" (stale since #577; C0b rewrites its caller list, C0b memo §4.9), `replicate_template_contents`'s "the C2 owner-doc approximation" (`clone.rs:398-399`), `scripting.rs:73-74` (at `27b50dfe`).

### §5.3 Touched files

Line counts: `wc -l` at `593aa314` (verified 2026-11-01); re-measured at C1's base (§9 item 1).

| file | lines at `593aa314` | change | 1000-line decision |
|---|---|---|---|
| `crates/core/elidex-ecs/src/dom/template_owner.rs` (NEW) | — | D-C2 rule, predicate, D-C7 hook | new file, by design: keeps `mod.rs` under the limit |
| `crates/core/elidex-ecs/src/dom/mod.rs` | 926 | `mod` line; `attach_template_contents` doc and one call | net ≤ +10 lines, stays < 1000 (C0 nets ≈ −10); if the base is ≥ 990, the template-contents helpers (`:536-588`) move to `template_owner.rs` as a standalone pure-move prereq first (O-3) |
| `crates/core/elidex-ecs/src/components.rs` | 822 | two components, `TemplateContents` doc | stays < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/mutation.rs` (C0's home of the adopt walk) | 467 | D-C5: the contents edge's target and its segment's `new` | < 1000 (C0 adds `adopt`; re-measure) |
| `crates/core/elidex-ecs/src/dom/tree/teardown.rs` | 344 | Document cascade in `destroy_entity` (D-C7) | < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/navigation.rs` | 375 | `root_entities` exclusion (D-C9) | < 1000 |
| `crates/core/elidex-ecs/src/dom/tree_clone.rs` | 425 | clone-policy rows (D-C8) | < 1000 |
| `crates/dom/elidex-dom-api/src/node_methods/clone.rs` | 458 | D-C6 on D14 | < 1000 |
| `crates/dom/elidex-dom-api/src/document.rs` | 665 | comment only (P-7) | < 1000 |
| `crates/script/elidex-script-session/src/scripting.rs` | 279 (at `27b50dfe`) | doc `:73-74` (C0 rewrites the surrounding paragraph) | < 1000 |
| `crates/dom/elidex-html-parser/src/tests.rs` | 875 | two expectations (§5.2) | < 1000 |
| `crates/dom/elidex-dom-api/src/node_methods/tests/clone.rs` | 992 (720 at `fab44a1d`: RC-0a #587 split it, adding `tests/clone_shadow.rs`; `git show fab44a1d:<file> \| wc -l`) | one expectation rewritten in place (`:214` at `fab44a1d`); **no new tests** (they go to a new file) | < 1000 |
| `crates/core/elidex-ecs/src/dom/tests/template_owner.rs` (NEW) | — | unit cells U-* (§6) | new file |
| `crates/script/elidex-js/src/vm/tests/tests_template_owner_document.rs` (NEW) | — | VM cells T-* (§6) | new file |
| `crates/script/elidex-js/src/vm/tests/tests_html_t2d_protos.rs` | 1207 | **not touched** (`/usr/bin/grep -n 'owner' <file>` → `output_form_owner` only) | not triggered |

**Removed in rev 3 (moved to C0 rev 2).** Rev 2 listed seven files that C0 now owns: the custom-element consumer
`crates/dom/elidex-custom-elements/src/consumer.rs` (the `Adopt` arm and per-node pairs, C0 draft D-6); the parser
creation sites and the entry's strict method, `crates/dom/elidex-html-parser/src/convert.rs`,
`crates/dom/elidex-html-parser/src/lib.rs`, `crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs`,
`tree_builder/insert.rs`, `tree_builder/shadow.rs` (C0 draft D-10, D-11, §4.4); and
`crates/core/elidex-ecs/src/dom/tree/walkers.rs` (rev 2's Q-2, answered by C0 draft D-3 (c); C0c reorders `Include`,
D-19). C1's diff touches none of them.

### §5.4 Named risks (and the observable change each is)

- **R-C1 `ownerDocument` of template contents** → I (spec). A script that compared it to `document` sees `false`.
- **R-C2 URL resolution in template contents** → against `about:blank` (§2.4.3), so `a.href` / `img.src` IDL reads of
  relative URLs return the attribute value while inside contents. Gated on §1.1 M4–M6; cell T20. C1 adds no reader of
  the document base URL, only nodes whose base is I's fallback; the readers are `#11-document-base-url`'s population.
- **R-C3 handlers in template contents** → raw values read null and do not run (N1 + C0 + C1; C14 inverts, T17).
- **R-C4 `<body>` / `<frameset>` in template contents** → unchanged at C1 (the base's target rule ignores the
  document, T18); T3 (after C1) gives them the null target.
- **R-C5 a new Document wrapper (I) is reachable from script.** Its Document install and per-entity facts are C1-pre's
  (edge c-pre; §1.1 M3). DOMParser installs its documents' methods eagerly (`dom_parser.rs:305`); C1-pre makes the
  install uniform for every Document wrapper.
- **R-C6 I as a root.** Excluded (D-C9); cell U6.
- **R-C7 lifetime.** No I for a throwaway (U4, U5); I per DOMParser document lives with it (D-C7, §7).
- **R-C8 `adoptedCallback` arguments for custom elements in contents** → see D-C5 (T25, §1.1 M13).

## §6. Test plan

Columns: *HEAD* = `593aa314`; *base* = HEAD + N1 implementation + C0a–C0c + RC-2a with D14 + C1-pre (still C2); *C1 = spec*. **Red-first**: every
*red* cell is run on C1's base before the fix with its control green, and the failure is recorded in the PR description.
Cross-tier: cells marked (×2) run under both parser tiers. A cell whose spec value depends on §1.1 is asserted only
after the measurement agrees (D-C11).

### §6.1 VM integration (`tests_template_owner_document.rs`, NEW)

| cell | kind | script | base | C1 = spec |
|---|---|---|---|---|
| T1 | red | `t = document.createElement('template'); I = t.content.ownerDocument; r = [I !== document, I instanceof Document, I.defaultView === null, I.URL === 'about:blank', I.documentURI === 'about:blank']` | `[false, true, false, u, u]`, `u` = `document.URL === 'about:blank'` | `[true, true, true, true, true]` (the second to fifth elements rest on C1-pre, edge c-pre; the third to fifth after §1.1 M3) |
| T2 | guard | two `createElement('template')`: `t1.content.ownerDocument === t2.content.ownerDocument` (with T1's `!== document`) | `true` (both `document`) | `true` (same I) |
| T3 | red (×2) | initial HTML `<template id=a><b id=b></b></template>`: `a.content.ownerDocument !== document`, `a.content.getElementById('b').ownerDocument === a.content.ownerDocument` | `[false, true]` | `[true, true]` |
| T4 | red (×2) | `<template id=o><template id=i><p></p></template></template>`: `i.ownerDocument === o.content.ownerDocument`, `i.content.ownerDocument === o.content.ownerDocument`, `o.content.ownerDocument !== document` | `[true, true, false]` | `[true, true, true]` |
| T5 | guard (with U2; `I.createElement` rests on C1-pre, edge c-pre) | `I = tpl.content.ownerDocument; t = I.createElement('template'); t.content.ownerDocument === I` | `true` (`I` is `document`) | `true`, and U2 shows no I-of-I entity |
| T6 | red | `dp = new DOMParser().parseFromString('<template id=t><i></i></template>','text/html'); c = dp.getElementById('t').content; [c.ownerDocument !== dp, c.ownerDocument !== document, c.ownerDocument !== tpl.content.ownerDocument, c.firstChild.ownerDocument === c.ownerDocument]` | `[false, true, true, true]` | `[true, true, true, true]` |
| T7 | red | `tpl.innerHTML = '<b id=x></b>'; [tpl.content.firstChild.ownerDocument === tpl.content.ownerDocument, tpl.content.firstChild.ownerDocument !== document]` | `[true, false]` | `[true, true]` |
| T8 | red | `d = document.createElement('div'); tpl.content.append(d); [d.ownerDocument === tpl.content.ownerDocument, d.ownerDocument !== document]` | `[true, false]` | `[true, true]` |
| T9 | guard (K-0; §1 rows 1–2) | `c = tpl.content.cloneNode(true)` with `<a href="x.html">`; `r1 = c.firstChild.ownerDocument !== document`; `document.body.append(c)`; `[a.ownerDocument === document, a.href.endsWith('/x.html')]` | `false`, then `[true, true]` | `true`, then `[true, true]` (C1 without C0: `[false, false]`) |
| T10 | red | deep and shallow `tpl.cloneNode()` of a template with a nested template: `t2.content !== tpl.content`, `t2.content.ownerDocument === tpl.content.ownerDocument`, the nested copy's `content.ownerDocument` is the same, `!== document`; shallow copy's contents empty and `!== document` | `!== document` legs false | all true |
| T11 | red | `dc = document.cloneNode(true)`; `x = dc.querySelector('template').content.ownerDocument; [x !== dc, x !== tpl.content.ownerDocument]` | `[false, true]` | `[true, true]` |
| T12 | red (DSD legs strict only; variant ×2) | `<template id=t><div id=h><template shadowrootmode=open><span></span></template></div></template>`: `[h.shadowRoot.ownerDocument === t.content.ownerDocument, span.ownerDocument === t.content.ownerDocument, h.shadowRoot.ownerDocument !== document]`; variant: a parser-built `<div>` in contents, `attachShadow({mode:'open'}).ownerDocument !== document`. A tolerant document parse attaches no declarative shadow root (`#11-tolerant-document-declarative-shadow-roots`; host choice `#11-tolerant-dsd-host-from-tree-builder`), so the DSD legs have no tolerant run until that slot lands | the `!== document` legs false | all true |
| T13 | red | `dp`'s template (with a nested template and a shadow host in its contents) appended into `document.body`: `t.ownerDocument === document`, `t.content.ownerDocument === tpl.content.ownerDocument`, nested contents and shadow content likewise | `t.content.ownerDocument === document` | `=== tpl.content.ownerDocument` (I of `document`) for all |
| T14 | red (×2) | `el.innerHTML = '<template id=f><p></p></template>'` (`el` in `document`): `[f.content.ownerDocument === tpl.content.ownerDocument, f.content.firstChild.ownerDocument === tpl.content.ownerDocument, f.content.ownerDocument !== document]` | `[true, true, false]` | `[true, true, true]` |
| T17 | red (inverts umbrella C14; needs N1, C0) | umbrella C14's script | runs, `window.hit === 1` | does not run; `x.onclick === null` |
| T18 | guard (umbrella C18 unchanged at C1) | umbrella C18's script | C18's "as HEAD" column | the same (the base's target rule ignores the document); T3 rewrites C18 after C1 (T3 draft P-T9) |
| T19 | red (×2) | `<template id=s><svg id=v></svg><b id=w></b></template>`: inside contents `[v.ownerDocument !== document, v.ownerDocument === s.content.ownerDocument]`; `document.body.append(s.content)`; `[v.ownerDocument === document, w.ownerDocument === document]` | inside `[false, true]`; after `[true, true]` | inside `[true, true]`; after `[true, true]` |
| T20 | spec pin (§1.1 M4–M5) | `<template id=h><a id=a href="x.html"></a></template>`: `h.content.getElementById('a').href` | absolute against the page base | the measured value, if it is §1.1's spec value `"x.html"`; otherwise the PM's decision |
| T22 | red | `tpl.innerHTML = '<b id=x onclick="1">'`; read `x.onclick` | a function (`scripting.rs:73-74`'s false negative, at `27b50dfe`) | `null` |
| T23 | red (**R-A closure**, §9 item 9; needs N1, C0) | for each route by which a `<b onfocus="window.hit=1">` reaches template contents — (i) parsed (×2), (ii) `tpl.content.append(b)` of a `createElement`'d `b`, (iii) `tpl.content.cloneNode(true)` of contents holding it, (iv) `tpl.cloneNode(true)` of a template holding it — read `[b.ownerDocument === tpl.content.ownerDocument, b.ownerDocument !== document]`; for route (ii) also `b.onfocus === null` (the only route on which the base writes a raw value: slice a's Insert arm, W-K5; routes (i), (iii), (iv) are unprocessed at both base and C1 — K-5 and the G5 / K3 window — so their handler leg is slice b's, not this cell's) | `[true, false]` for every route; route (ii) `b.onfocus === null` → `false` | `[true, true]` for every route; route (ii) → `true` |
| T24 | red | clonable shadow host in contents: `tpl.innerHTML = '<div id=h></div>'`; `h = tpl.content.firstChild; h.attachShadow({mode:'open', clonable:true}).innerHTML = '<i></i>'`; `c = tpl.cloneNode(true); ch = c.content.firstChild; [ch.ownerDocument !== document, ch.shadowRoot.ownerDocument !== document, ch.shadowRoot.firstChild.ownerDocument !== document]` | `[false, false, false]` | `[true, true, true]` (C1 without D-C6's per-pair document: `[false, false, false]`, the host re-stamped with `document`) |
| T25 | red (§1.1 M13; needs C0c's `Adopt`; C1's half only: the pair **values**; C0c's T18 / U13 own per-node pairs and order) | `I = tpl.content.ownerDocument`; `x-a` with an `adoptedCallback(o, n)` logging the pair; `dp = new DOMParser().parseFromString('<template id=t></template>','text/html'); I_dp = dp.getElementById('t').content.ownerDocument`; `e = document.createElement('x-a'); dp.getElementById('t').content.append(e)`; `document.body.append(dp.getElementById('t'))` | first callback `(document, dp)` (C2: `I_dp` is `dp`); second `(dp, document)` | first `(document, I_dp)`; second `(I_dp, I)`, in that order |
| T26 | red (D-C5's `f(target)` on first use) | `dp = new DOMParser().parseFromString('<p></p>','text/html')` (no template, so no I yet); `tpl.content.append(document.createElement('i')); dp.body.append(tpl); x = tpl.content.ownerDocument; [x !== dp, x !== document, tpl.content.firstChild.ownerDocument === x, dp.createElement('template').content.ownerDocument === x]` | `[false, true, true, true]` (`x` is `dp`) | `[true, true, true, true]` (`x` = `dp`'s new I, created by the adopt) |
| T27 | red (C0c skip on the mapped `new`, forward) | `x-a` as T25; `tpl.innerHTML = '<template id=n></template>'; n = tpl.content.firstChild; e = document.createElement('x-a'); n.content.append(e)` ‖ `log = []; document.body.append(n)` ‖ `[log, e.ownerDocument !== document, n.ownerDocument === document]` | `[[], false, true]` (same-document move) | `[[], true, true]`: `n.content`'s old `I` equals `f(document) = I`, so the segment is skipped and no `adoptedCallback(I, I)` fires |
| T28 | red (C0c skip on the mapped `new`, reverse) | `x-a` as T25; `dp = new DOMParser().parseFromString('<template id=t></template>','text/html'); I_dp = dp.getElementById('t').content.ownerDocument`; `t = document.createElement('template'); e = document.createElement('x-a'); t.content.append(e); dp.adoptNode(t.content)` ‖ `log = []; dp.body.append(t)` ‖ `[log, t.content.ownerDocument !== dp, e.ownerDocument === t.content.ownerDocument]` | `[[], false, true]` (contents' old `dp` equals the target: skipped) | `[[(dp, I_dp)], true, true]`: old `dp` ≠ `f(dp)`, so the contents and `e` go to `I_dp` |

### §6.2 Unit (`crates/core/elidex-ecs/src/dom/tests/template_owner.rs`, NEW)

| cell | kind | shape | expected |
|---|---|---|---|
| U1 | red | `f(D)` twice → same entity, marked; D linked | equal, marked |
| U2 | red | `f(f(D)) == f(D)`; count of marked entities is 1 | 1 |
| U3 | red | D without any template: no marked entity after parse and `createElement('div')` | 0 (lazy) |
| U4 | red | 100 fragment parses with a template (both tiers), context in D: marked-entity count stays 1 (D's I); the throwaway carries no `AssociatedInertTemplateDocument` (read as a component) before its destruction | 1; absent |
| U5 | red | strict parse error with a template in the fragment: marked-entity count unchanged (D's I, created lazily or already there); no entity of the throwaway survives | as stated |
| U6 | red | `I` with an element child: `root_entities()` excludes I | excluded |
| U7 | red | `destroy_entity(D)` destroys `f(D)`; a contents node outliving it reads `node_document` → `None` | destroyed, `None` |
| U8 | red | clone policy: `clone_subtree(D)` copy has no link; `clone_subtree(I)` copy has no marker | absent |
| U9 | red | C0's adopt of a template into X, with nested contents and a shadow host in contents: shadow-including descendants → X, contents and nested contents and their shadow trees → `f(X)` | as stated |
| U10 | guard | the adopt walk past the depth and breadth caps (as `despawn_subtree_tears_down_past_the_traversal_depth_cap`, `dom/tests/destroy.rs`) | every node re-homed |
| U11 | red | C0's new-node entry for a `source` in D's template contents (the fragment, a descendant, nested contents, a shadow root in contents) → `f(D)`; and for a template parsed in a fragment build with context in D, its contents fragment's stamp → `f(D)` (never `f(throwaway)`). The entry's own contract (D → D; the fragment-parse root → the context's document) is C0b's cells (its U11), not C1's | as stated |
| U12 | red (×2) | document parse of `<template><svg></svg><b>t</b><!--c--></template>` into D with a recording `MutationDispatcher` installed: after the parse every node in contents carries `AssociatedDocument == f(D)`, and the recorder holds no `Adopt` event (an `Adopt` would mean a node was created with another document and re-homed at its link) | as stated |
| U13 | red | `adopt(template, X)` with X carrying no `AssociatedInertTemplateDocument`: afterwards X is linked to one marked I and the contents are stamped I; `adopt(div_subtree, Y)` (no template) into Y with no link: Y stays unlinked | as stated (lazy, D-C7's "iff") |

### §6.3 Mutation guards

| mutation | killed by |
|---|---|
| a new I per template (no link) | T2, U1 |
| no marker check (I of I) | T4, T5, U2 |
| caller passes the template's document unmapped (C2) | T1, T3, T6 |
| adopt re-homes contents to the target (C0's identity edge kept) | T13, U9 |
| contents children cloned with the operation's document | T10 |
| shadow pass uses the operation-wide document inside copied contents | T24 |
| no destroy cascade | U7 |
| `f` of the throwaway computed (root redirect dropped) | U4 |
| link or marker copied by clone | T11, U8 |
| `root_entities` not excluding I | U6 |
| inside contents the entry's value is not I (a creation stamp from the parse / context document; the defect class C0's entry would have if it ignored `source`, here observed through contents) | T19 (strict leg), U11, U12 |
| tolerant threads the template's document into contents instead of recomputing at the contents fragment (§4.3) | T3, T12 (attachShadow variant), T19, U12 (tolerant legs) |
| the contents edge's segment `new` left at the template's target (C2) | T25, U9 |
| the contents edge reads the target's existing link (none on a target with no I) instead of computing `f(target)` | T26, U13 |
| `f(target)` computed on every adopt, template or not | U13 |
| C0c's skip compares the contents' old document with the unmapped target | T27, T28 |

### §6.4 Kept green

Slice a's U8 (ii) (the bind pass does not walk template contents), E1–E8, C18 (T18), and the #380 template suite
(`tests_html_t2d_protos.rs` `template_*`, `dom/tests/destroy.rs`, `dom/tests/traversal.rs`), C0a–C0c's cells except
those §5.2's population marks *changes* (C0c U13's order leg stays green), D14's cells. Commands: `cargo test -p elidex-ecs --all-features`, `-p elidex-dom-api`, `-p elidex-html-parser`,
`-p elidex-html-parser-strict`, `-p elidex-custom-elements`, `-p elidex-script-session`, `-p elidex-js`, then `mise run ci`.

## §7. Out of scope and slots

| item | where | defect (stated, not the remedy) |
|---|---|---|
| insertion adoption; the node-document reader | C0, `#11-cross-document-adopt-on-insert` (prerequisite) | no insertion adopts at `593aa314` |
| L3 Range creation sites A1–A9; dom-api creation sites that decide an owner from `owner_document(this)` | `#11-ownerless-node-creation` (owner Layout C0/C1; C0 rev 2 receives L3's list) | creation sites leave a node ownerless or decide its owner outside the entry |
| DocumentFragment *host* of template contents (§4.12.3 establishing step 2) | no slot (scout Q-S7); §8 R-6 | the contents fragment has no host link, so host-including-ancestor checks cannot see the template |
| template `for` attribute, content patching (§4.12.3) | no slot; §8 R-7 | elidex implements neither the attribute nor *prepare content patching* |
| DOM *adopt* steps 3.2 / 3.3.1–3.3.2 (registries, attribute nodes) | C0 draft §7; step 3.3.2's null-registry half `#11-document-null-registry-adopt-reset` (edge c-NR) | not modelled |
| render / layout / hit-test roots for DOMParser documents and detached subtrees with children | no slot (scout Q-S6); §8 R-8 | `root_entities` returns them and H-C11's consumers keep any with a child |
| DOMParser documents are never destroyed | no slot found (scout Q-S1); §8 R-9 | every `parseFromString` leaves a Document (and, from C1, its I if it has a template) in the page `EcsDom` for the World's lifetime |
| template-inclusive walkers (`collect_template_inclusive_descendants`, `derive_element_components`) | umbrella §9 N-5 (slice b) | three walkers for one concept |
| multi-realm creation of I | B1 (§8 H-3) | I is created in the single realm |

Own deferrals: **1** — C1 widens the DOMParser-leak row by one I per DOMParser document that creates a template (D-C7).
C1 also edits the function of the `root_entities` row (D-C9 adds the I exclusion) without widening that row's class.
The other rows are pre-existing or another PR's scope.

## §8. Edges, hand-offs and records

**Edges.**
- c1: N1 implementation → C1 (T17, T22, T23's read-null legs need N1's compile gate). N1 landed (#580, `27b50dfe`).
- c3b: **C0b → C1** (semantic + code, PM decisions 143 / 153 / 244; C0 draft edge c0b): the reader, insertion
  adoption, the new-node entry and its creation census, the hook (`attach_template_contents`'s `NewNodeOwner`) and the
  adopt walk's contents edge — I's value (D-C3, D-C4, D-C5's target).
- c3c: **C0c → C1** (semantic + code; C0 draft edge c0b, C0c memo edge c0b): the segments and the `Adopt` consumer —
  the per-segment (old, new) pairs whose `new` D-C5 sets (T25).
- c-RC2a: **L3 RC-2a → C1** (code predecessor on `clone.rs`'s child-clone loop: D14 and RC-2a's cloneNode migration both
  rewrite it; PM decisions 215, 241, 249; C1 rebases on it). RC-2a is in either order with C0b and C0c (C0 draft edges
  c0g, c0e).
- c-pre: **C1-pre → C1** (code + semantic, PM 328): I's wrapper has the Document install and reads its own entity's
  facts, which C1-pre also writes (URL included) (header; §4.1; T1, T5, §1.1 M3, R-C5).
- c-E: **C0b** introduces the new-node entry (PM decisions 223 O-2 / 227; C0 draft D-10, §4.5 A11 / A12); L3 RC-4 (A11 / A12)
  reads it. RC-4 lands **after** C1: the Range program's definition of done ("RC-4 after N9, C0b and C1") and its §5
  RC-4 text ("PM 216 rules out the first while C1 is a dependency", the first being this memo's old "RC-4 before C1"
  branch), so RC-4's cells assert I. Whether C1 stays a dependency of RC-4 is that program's Q-Z1 ("Does C1 stay a
  dependency of RC-4?"), the PM's.
- c-NR: **C1 → L3 `#11-document-null-registry-adopt-reset`** (PM 328; the ledger trigger moved from C0c's landing to
  C1's): its scope (4) names the inert template document, which C1 creates. L3 picks the edge up from that ledger row.
- c4: **C1 → T3** (T3 draft: T3 relies on D-C2 to D-C6; R-A closed by C1; T3 rewrites C18).
- c5: C1 → slice b (umbrella x5; §4.6 is the provision).

**Hand-offs.**
- H-1 to slice b: §4.6 items 1–4; the umbrella's §9 N-2 / N-3 (branch B's per-node fact) do not apply in branch A.
- H-2 to C0: closed. P-C14 is folded (C0c), P-C15 is moot (C0 D-8, D-10), §4.3's constraint is met (D-20).
- H-3 to B1: create I in D's relevant realm.
- H-4 to T3 (rev 3): T3's downstream list for this memo's owner (T3 draft §8.2 "C1 (lands before T3)") is applied by
  rev 2 (P-C11): base "N1 and C0", edge c4 (C1 → T3) replaces rev 1's c2, T18 is a guard, R-C4 unchanged at C1. T3's
  R-1 note ("done by this PR (umbrella rev 8.5)") conflicts with PM decisions 144 / 159; resolved by O-1 (a): T3 drops its C0 / C1 rows (T3 rev 4's job, not this memo's).
- H-5 to L3 (RC-2a incl. D14, RC-4): §4.4's per-pair document and §4.5's A11 / A12 contract.

**Records (for the PM; exact texts).** Applied by C1's PR in place, with history (PM decisions 144 / 159): C1's PR
applies R-1 and R-3; the umbrella revision number is assigned at landing (PM 223 O-1 / 232). This plan PR still does
**not** edit the umbrella file.
- **R-1 umbrella §1.2** — add the row (after the C0 program's row, which C0a's PR adds, C0 draft D-9):
  "| **C1** `EH-attrs-at-creation-C1` (owner: Layout lane, this program; PM decision 123) | slot
  `#11-template-contents-owner-document`: the inert template contents owner document (HTML §4.12.3; branch A, §1.6);
  closes T3's residual R-A | IBP-sandbox N1's implementation and C0 landed; L3 RC-2a landed |".
  §1.6's heading and lead: "C1's handling (decided by the PM, not here)" → "C1's handling (decided: branch A, PM decision
  123)", keeping the old wording in the changelog entry. §8.1's `#11-template-contents-owner-document` bullet: "Owner
  TBD …" → "Owner: Layout lane (PM decision 123); C1, after C0 (PM decisions 143, 153); plan
  `docs/plans/2026-10-eh-attrs-c1-template-owner-document.md`".
- **R-2 ledger `#11-template-contents-owner-document`**: "C1 plan-memo rev 2 (2026-11-01). Order (PM decision 153): N1
  implementation → C0 → C1 → T3 → slice b (supersedes this sub-bullet's 'N1 implementation → T3 → C1 → slice b'). C1's
  base also contains L3 RC-2a, which carries D14 (PM decisions 215, 241). Closes at C1's landing; closes T3's R-A with it."
- **R-3 umbrella in-place corrections** (at C1's PR): cell C14's expected column "(spec: does not run; slice b inverts it
  under S-2)" → "(spec: does not run; C1 inverts it, with N1's compile gate and C0's adoption at the placement)"; K-5's
  "From N1, the DOMParser half reads null." → "From N1, the DOMParser half reads null; from C1, the template-contents half
  reads null."; §8.2 defect (5)'s "its template-contents half, and the first clause for both, remain until slice b" →
  "its template-contents half until C1 lands, and the first clause for both until slice b"; §1.4's interim line gains a
  fourth window "(iv) W-K5's template-contents half reads null from C1, the spec's outcome (K-5)"; §1.5's W-K5 bullet
  gains "(from C1, its template-contents half too)"; C18's *Spec* note "reached after T3 and slice b" is unchanged.
- **R-4 ledger `#11-ownerless-node-creation`**: "C1 rev 3 (2026-11-01): the `EcsDom` entry and every production
  creation site, both parser tiers included, are C0's (PM decisions 223 O-2 / 227; C0 memo D-10, D-11, §4.4). C1's facets:
  template contents' value of the entry (I, through `attach_template_contents`'s second argument), A10 (template clone)
  and the A11 / A12 contract. A1–A9 and the dom-api sites: C0 / L3." (Supersedes rev 2's R-4 text, which assigned the
  parser tiers to C1 and left the entry's PR to O-2.)
- **R-5 slice a's docstring** `scripting.rs:73-74` (at `27b50dfe`) is removed by C1 (§5.2); no ledger text.
- **R-6 to R-9: §7's unslotted known defects** (for the PM to register or decline; IDs proposed). Each: **Why deferred**
  — not the contents' node document, C1's one fact (§1 *Split decision*); **Re-eval** 2026-12-31.
  - R-6 `#11-template-contents-host` — the contents fragment has no host link (HTML §4.12.3 establishing step 2), so
    host-including-ancestor checks cannot see the template. Pre-existing. **Trigger**: a PR that implements
    host-including inclusive ancestors or a fragment's host.
  - R-7 `#11-template-for-content-patching` — elidex implements neither the template `for` attribute nor *prepare
    content patching* (HTML §4.12.3). Pre-existing. **Trigger**: a PR that implements streaming or out-of-order content
    patching.
  - R-8 `#11-root-entities-non-rendered-documents` — `root_entities` returns DOMParser documents and detached subtrees
    with children, and H-C11's four consumers lay out, paint and hit-test them. Pre-existing; C1 adds only the I
    exclusion (D-C9). **Trigger**: a PR that changes root discovery or the render / layout / hit-test root set.
  - R-9 `#11-domparser-document-lifetime` — DOMParser documents are never destroyed, so each `parseFromString` leaves a
    Document in the page `EcsDom` for the World's lifetime; **widened by C1** with one I per such document that creates
    a template (D-C7). **Trigger**: a PR that adds Document destruction or GC of unreachable Documents; it, and any caller of D-C7's
    `destroy_entity(I)`, re-examines §4.2's live-link `debug_assert!`.
- **R-10 ledger `#11-cross-document-adopt-on-insert`**, sub-bullet "Owner assigned 2026-10-31": its pointer "C1 plan memo
  §4.6 C0-1..C0-4 (branch `layout-eh-c1-template-owner-plan` @ `bc52d1b9`)" is stale (§4.6 is now what C1 provides to
  slice b; the C0 interface is §4.7, and C0's own memos carry it), and its order "N1 implementation → T3 → C0 → C1 →
  slice b" predates PM decision 153. Proposed text: "Requirements: the C0 program memo and its per-PR memos; C1 plan
  memo §4.7. Order (PM decision 153): N1 implementation → C0 → C1 → T3 → slice b."

## §9. Definition of done

1. Coordinates re-measured at C1's base (N1 implementation, C0a–C0c, RC-2a with D14, C1-pre landed); §5.1 populations re-run
   and any change recorded in the PR; the C0b memo's §5.1 P-3 and P-8 re-run as base checks (§5.1 P-4 / P-5).
2. `appropriate_template_contents_owner_document`, `is_inert_template_document` and both
   components exist in `elidex-ecs` with §4.9's citations; no other site computes a template contents' node document
   (P-1 re-run); C1's diff adds no `owner_document(` call (D-C10). C1's diff adds no new-node entry, no entry branch
   and no creation-site change, and touches none of §5.3's "removed in rev 3" files (`git diff --stat <base>..HEAD`);
   C0's P-3 and P-8 ratchets (C0b memo §5.1) still hold at C1's head.
3. D-C3 to D-C9 implemented; the clone-policy table has both rows.
4. §1.1's table filled (engine and version recorded) before the implementation branch is cut; any row differing from
   the "C1 head" column decided by the PM (D-C11).
5. §6.1 and §6.2 cells present; every red cell run red on the base with its control green, recorded in the PR
   description; T9 green at the head.
6. §5.2 rewrites done; P-7's grep returns no hit in `crates/`.
7. `cargo test` for the seven crates of §6.4 and `mise run ci` green.
8. `python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-10-eh-attrs-c1-template-owner-document.md`
   exits 0 (run; do not trust a stored verdict).
9. **T3's residual R-A is closed by C1 (PM decision 153).** R-A (T3 draft §1.2): "a node in template contents or in a
   clone of them has the inert document I as its node document, by every way it gets there: parsed (C1 D-C3, D-C4),
   appended into `tpl.content` (C0-2's adoption), cloned from contents (C1 D-C6), or cloned with a template". Cell
   **T23** pins all four routes at C1's head — node document I, not `document`; on the appended route also a raw handler
   reading `null` (N1) —
   with T19 / T12 (parsed, both tiers, foreign and shadow content), T8 (appended), T9 / T10 (cloned) and T24 (cloned with
   a shadow host) as the per-route mutation guards. T3's target cells for R-A (T-17, EW-13, EW-14, XW-8) are T3's to
   assert on top of these facts (T3 lands after C1).
10. The PM has applied R-1 to R-4 and R-6 to R-10 (or recorded why not), and decided O-3 (O-1 is decided: (a), PM 223 O-1 / 232; O-2 is decided: PM 223 O-2 / 227).

## §10. Revision history / dispositions

The rev 3 table (each rev 2 item moved to C0 rev 2, and what C1 kept) and the rev 2 table (driver brief inputs 1–9
and their dispositions) are history: `git show <sha>:docs/plans/2026-10-eh-attrs-c1-template-owner-document.md`
§10.0 / §10.1, with `<sha>` = `git log --format=%h -1 --grep='C1 plan-memo rev 3.1' -- <memo>` on this branch (after the
squash merge, from the plan PR's head ref). Rev 3.2's alignments are the header changelog's.

### §10.2 Rev 1 (2026-10-31)

First draft, pre-plan-review. Its O-1 (C0 first) is decided by PM decisions 143 / 153; its O-2 (umbrella vehicle) by
PM decisions 144 / 159 (sequencing: O-1 below, decided rev 3.1: (a), PM 223 O-1 / 232); its O-3 (URL resolution) becomes §1.1; its O-4 (split) is O-3
below; its Q-1 (shared node-document helper) is resolved by C0's `node_document` and D-C4's entry; its Q-2 is Q-2 below;
its Q-3 (T18) is resolved (T18 is a guard at C1).

## §11. Open questions

**For the PM / user.**
- **O-1 Umbrella edit sequencing (P-C16).** **Decided rev 3.1: (a)**, by PM decision 223 O-1, confirmed for C1 by PM
  decision 232: one owner per §1.2 row; each PR adds only its own, in landing order; the umbrella revision number is
  assigned at landing; it covers C1's row (R-1) and R-3's texts; the T3 draft drops its C0 / C1 rows (T3 rev 4's job).
  The options as asked are in the rev 3.1 text (§10).
- **O-2 Which PR adds the `EcsDom` new-node entry (P-C13).** **Decided (rev 3): C0** (now C0b), by PM decisions 223 O-2
  / 227; C1 supplies only the template-contents value through `attach_template_contents`'s second argument (D-C3, D-C4).
- **O-3 Split.** If `mod.rs` is ≥ 990 lines at the base, the template-contents helpers move to `template_owner.rs` as a
  standalone pure-move PR first (the #537 precedent) or as a standalone first commit of C1's PR.

**For plan-review.**
- **Q-1** (the entry's signature), **Q-2** (the adopt walk's shape) and **Q-3** (`adoptedCallback` arguments in
  contents) are answered by C0: Q-1 by D-10 / D-17 (one `source`, a `NewNodeOwner`, the redirection by
  `FragmentParseRoot` on the fragment-parse root); Q-2 by D-3 (c) (the `Include` walk with one contents edge; no walker
  variant; D-19 reorders it); Q-3 by D-6 (C1 sets the segment's `new`). §4.3's constraint is met by D-20.
- **Q-4 Decided (plan-review R1):** `clone_subtree`'s override is a `NewNodeOwner`. It is C0b's surface, so it is a C0b
  erratum made in the C0b memo, not here; §4.4 steps 2–3 pass `node_document_for_new_nodes(..)`. §4.4's representation
  choice (tuple field or read-back) stays open.

**Scout questions** (facts this memo needs; not read here).
- **Q-S10** Closed: a non-bound Document wrapper's surface is C1-pre's (edge c-pre).
- **Q-S11** `crates/dom/elidex-dom-api/src/focus/delegate.rs:85`: does its node-document read change outcome for an
  element in template contents (focus delegation into an inert document), and is the new outcome the spec's?
- **Q-S12** Closed. First half: a tolerant **document** parse attaches no declarative shadow root
  (`#11-tolerant-document-declarative-shadow-roots`; the fragment path passes the flag), so T12's DSD legs are strict
  only. Second half: answered in rev 3 (§4.4).
