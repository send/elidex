# C1 `EH-attrs-at-creation-C1` — template contents are owned by the inert template contents owner document (per-PR plan-memo)

Nested PR **C1** of the standalone program `EH-attrs-at-creation` (umbrella
`docs/plans/2026-10-event-handler-attrs-at-creation.md`, approved by #551 `b9122157` at rev 8, carried to rev 8.3 in
place by #570; "the umbrella"). It closes slot `#11-template-contents-owner-document` (carved from #380, `c801ef6c`),
closes T3's residual **R-A** (§9 item 9), and is slice b's precondition x5 in **branch A** (umbrella §1.3 x5, §1.6).
Siblings cited by ID:
- slice a (`docs/plans/2026-10-eh-attrs-slice-a.md`, landed by #563 `3cd84a04`; "the slice-a plan");
- IBP-sandbox N1 (`docs/plans/2026-10-ibp-sandbox-handler-gates.md`, rev 4.5, merged as a plan by #570; implementation
  PR **#580, open, not merged** on 2026-11-01; "the N1 memo");
- **C0** (`docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md`, rev 1 draft on branch
  `layout-dom-c0-insert-adopt-plan`, `fade8a19`, not on main; "the C0 draft"; read with `git -C <wt-c0-plan> show
  fade8a19:docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md`);
- T3 (`docs/plans/2026-10-eh-attrs-t3-target-determination.md`, rev 3 draft on branch `layout-eh-attrs-t3-plan`,
  `14ff3dc2`, not on main; "the T3 draft");
- L3's fragment plan (`docs/plans/2026-10-fragment-dsd-not-on-context.md`, #574 `6aea28da`; PR 0a #576 `cc6a6665` and
  PR 1 #577 `ee6c7a1a` landed);
- L3's Range contents program (umbrella on branch `range-full-impl-plan`, `7ac367a3`, **unpushed**: its appendix A
  site IDs A1–A12 and its RC-2 item D14 are cited as relayed by the Layout lane driver, not read here; "the L3 Range
  draft").
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
- PM decisions 199 and 207 (relayed; ledger slot `#11-ownerless-node-creation`, "registered 2026-11-01, PM 207"): "every
  creation site takes its owner from the single entry `node_document_for_new_nodes` (C1 rev 2; PM 123/199), and
  `node_document` is the only way it is read (C0 …)". Owner: Layout (C0/C1).
- PM decision 215 (relayed): L3 RC-2's D14 (slot `#11-clone-shadow-children-subtree`, ledger "registered 2026-11-01, PM
  212") lands before C1 and rewrites the same child-clone loop; C1 rebases on it.
- PM decision 145 (relayed): before implementation, measure in the built-in Chromium the outcomes C1 changes where the
  spec is silent or elidex's rule is a reading (§1.1).
- The option texts of decisions 144, 145, 159, 199, 207 and 215 were not shown to this memo's author; they are cited
  as relayed in the driver's rev-2 brief, not quoted.

**Prerequisites (at C1's base).** N1's implementation (#580) and **C0** landed; L3 RC-2's D14 landed (§4.4). T3 and
slice b are **after** C1. Nothing listed here is on `origin/main` at `593aa314` (2026-11-01). The C0 dependency is
semantic, not cosmetic: §1's table shows that C1 on a base without insertion adoption makes the most common template
pattern worse than HEAD (PM decision 143); with C0 on the base it does not (§1, K-0 lens).

**Scope.** One rule computes a template's contents' node document — WHATWG HTML §4.12.3's *appropriate template contents
owner document* — and every site that establishes, clones, parses into or adopts template contents goes through it,
reading node documents only through C0's `EcsDom::node_document` (NEW: C0):
1. the inert document itself, as an ECS entity and two components (D-C1, D-C2);
2. the establishing steps at every template creation (both parser tiers, `createElement`, the clone post-pass) (D-C3);
3. parser-created nodes take their owner at creation from the single new-node entry `node_document_for_new_nodes`,
   whose value inside template contents is the inert document (D-C4); L3's A11 / A12 read the same entry (§4.5);
4. the template *adopting steps* inside C0's one adopt walk, including the `adoptedCallback` arguments for custom
   elements inside contents (D-C5);
5. the template *cloning steps*, and the per-pair document of the clone worklist (D-C6; site A10);
6. the inert document's lifetime: created lazily, destroyed with its document, never copied by a clone (D-C7, D-C8);
7. the inert document is not a render/layout root (D-C9);
8. the C2 docstrings and the tests that pin C2 are rewritten (§5.2).

**Not in scope.** Insertion adoption and the node-document reader (C0); creation-time handler processing (slice b,
S-1/S-2); target determination (T3, after C1; it rewrites umbrella cell C18, §6 T18); L3's Range node-creation sites
A1–A9 (C0 rev 2 / L3, §7); the template element's `for` attribute and content patching (§7); the DocumentFragment
*host* of template contents (§7); multi-realm (B1; "given document's relevant realm", §4.1).

**Changelog.** *Rev 2* (2026-11-01): driver brief inputs 1–9 (§10 lists each and its disposition); rebased on
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

### §0.2 Premises re-measured

At `593aa314` unless the row says "on the base" (= `593aa314` + N1 implementation + C0 + D14, as their drafts state).

| id | claim | evidence |
|---|---|---|
| H-C1 | One establishing helper exists; its owner parameter is the C2 approximation | `EcsDom::attach_template_contents` `crates/core/elidex-ecs/src/dom/mod.rs:557`; doc `:548-556` ("ships the **C2 approximation** `owner_doc = template's own node document`"; its "strict: post-insert `owner_document(template)`" is stale since #577, §5.2); creates the fragment with `create_document_fragment_with_owner(owner_doc)` and links `TemplateContents { fragment }` (`crates/core/elidex-ecs/src/components.rs:366`) |
| H-C2 | Four production callers of the helper | `/usr/bin/grep -rnF 'attach_template_contents(' crates --include='*.rs' \| /usr/bin/grep -v '/tests/\|tests\.rs\|/tests_'` → clone post-pass `crates/dom/elidex-dom-api/src/node_methods/clone.rs:422`, `createElement` `crates/dom/elidex-dom-api/src/document.rs:264` (`Some(this)`), tolerant `crates/dom/elidex-html-parser/src/convert.rs:284` (threaded `owner_document`), strict `attach_ordinary_template_contents` `crates/dom/elidex-html-parser-strict/src/tree_builder/shadow.rs:123-125` (`node_document_for_new_nodes()`), plus the definition (verified 2026-11-01) |
| H-C3 | The strict parser has one new-node owner function (since #577) | `node_document_for_new_nodes` (strict `TreeBuilder` method) `crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs:271` (`pub(super)`): fragment parse → `owner_document(context)`, else `Some(self.document)`. Its callers: `/usr/bin/grep -rn 'node_document_for_new_nodes' crates` → `insert.rs:108` (`insert_foreign_element`) and `shadow.rs:124` (`attach_ordinary_template_contents`) only. It takes no intended parent, so inside template contents it returns the parse (or context) document |
| H-C4 | The other parser creations stamp nothing | strict: `insert_element_named` `insert.rs:55` (`create_element`), `insert_comment` / `insert_comment_to` `:131`, `:140` (`create_comment`), `append_text` `:193` (`create_text`), the DSD template `shadow.rs:153`; tolerant: `convert_node` `convert.rs:237` creates elements with `None` (`:251`), text, comment and doctype without an owner (`:305`, `:309`, `:316`). The tolerant tier stamps **only** the contents fragment (`convert.rs:284` → the helper); the threaded `owner_document` is otherwise only an argument (`:199`, `:286`, `:289`) (scout Q-S3). The tolerant tier's fragment entry chooses its threaded document itself: `owner_document(context)` `crates/dom/elidex-html-parser/src/lib.rs:158`, a second statement of H-C3's fragment branch |
| H-C5 | Today descendants without a stamp resolve through the tree root; C0 removes that | `EcsDom::owner_document` `mod.rs:817`: stamp first, else the tree root's Document, else a `DocumentFragment` root's stamp (`:836-856`). **On the base** C0 deletes both fallbacks and adds `EcsDom::node_document` (NEW: C0) (Document → itself; node → its live `AssociatedDocument`, else `None`) as the sole reader (C0 draft D-1); every node linked under a parent with a node document is stamped by C0's link-time adopt (C0 draft D-4, invariant I-U) |
| H-C6 | Adoption re-homes template contents to the **same** document as the template | `EcsDom::adopt_subtree` `crates/core/elidex-ecs/src/dom/tree/teardown.rs:251` walks `for_each_uncapped_shadow_inclusive(root, TemplateContentsWalk::Include, …)` and stamps every visited node with `document`. Caller: `finish_detached_fragment` (`teardown.rs:337`); a test (`dom/tests/destroy.rs:324`). **On the base** C0 keeps this walk as `adopt`'s step 3 (C0 draft D-3: "Under C2 the template adopting steps … are the identity … C1 replaces that one edge with `f(document)`") and dispatches `MutationEvent::Adopt` (NEW: C0) once per adopted root (C0 draft D-6) |
| H-C7 | A clone stamps an explicit owner on every node the deep cloner makes | `clone_subtree`'s document choice `crates/core/elidex-ecs/src/dom/tree_clone.rs:206-210`, root stamp `:221`, descendant stamp `:417`. The post-pass `clone_node_with_shadow_honor` `clone.rs:227` computes **one** `shadow_doc` (`:239-243`) and threads it into every pair's template pass (`replicate_template_contents`, `:262`, def `:405`; fragment `:422`, children `:426`) and shadow pass (host and shadow root stamps `:283-284`, shadow children `:290-302`) |
| H-C8 | The `ownerDocument` getter and URL resolution read the node document | dom-api getter `crates/dom/elidex-dom-api/src/node_methods/core.rs:279` (`owner_document(this).or_else(\|\| dom.document_root())`); href base `crates/dom/elidex-dom-api/src/element/href_accessor.rs:50`; base URL `crates/dom/elidex-dom-api/src/element/document_base.rs:177`. On the base the last two read `node_document` (C0 draft D-7 (b), (c)) |
| H-C9 | The handler gates derive the node document through one rule | `scripting_disabled_for_platform_object` clause (b) `crates/script/elidex-script-session/src/scripting.rs:95-113` (composed-root proxy). Its doc names the template false negative as this slot's (`:59-60`). **On the base** C0 replaces the proxy by `node_document(entity) != bound` (C0 draft D-7 (a)); T3 (after C1) reads `node_document` too (C0 draft H-T) |
| H-C10 | A Document entity is spawned in one place | `EcsDom::create_document_node` `mod.rs:410`. Production callers: `teardown.rs:301` (fragment throwaway), `mod.rs:395` (`create_document_root`), `crates/dom/elidex-form/src/inert_document.rs:119` (DOMParser) |
| H-C11 | `root_entities` excludes only `DocumentFragment` | `EcsDom::root_entities` `crates/core/elidex-ecs/src/dom/tree/navigation.rs:362-374`; consumers: `/usr/bin/grep -rn 'root_entities()' crates --include='*.rs' \| /usr/bin/grep -v '/tests/\|tests\.rs\|/tests_'` → `crates/core/elidex-render/src/builder/mod.rs:479`, `:995`, `crates/layout/elidex-layout/src/layout/mod.rs:371`, `crates/layout/elidex-layout/src/hit_test.rs:86` (4; the `lifecycle.rs:300` hit is `clear_root_entities`, unrelated) (verified 2026-11-01). Rev 1's `grep -v test` dropped `hit_test.rs` (§0.3 P-C8) |
| H-C12 | Two production paths destroy a Document; DOMParser documents are never destroyed | scout Q-S1 (29 destroy / despawn calls enumerated, 2026-10-31): `finish_detached_fragment`'s `destroy_entity(document)` `teardown.rs:341` and the strict rollback `despawn_subtree(document)` `tree_builder/mod.rs:233`; DOMParser documents (`inert_document.rs:119`) are never destroyed; iframe documents are not destroyed as ECS entities (each navigation and iframe gets a new `EcsDom`). `destroy_entity` (`teardown.rs:51`) cleans shadow links only, no Document cascade |
| H-C13 | elidex has no XML document | `/usr/bin/grep -rn 'is_html_document\|XmlDocument' crates --include='*.rs'` → only the forward-looking note `mod.rs:730`; DOMParser serves `text/html` only (umbrella §3 row §8.5.1) |
| H-C14 | The clone-policy table enumerates every component | `tree_clone.rs:15-34` (module doc), "deliberate non-copy" row `:30`, `TemplateContents` row `:27` |
| H-C15 | The bind pass excludes template contents | slice a's bind walk `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:178` (`TemplateContentsWalk::Exclude`); umbrella D-11, K-5, K-6 |
| H-C16 | The VM's `Document` methods act on the receiver entity | scout Q-S5: methods resolve the receiver (`crates/script/elidex-js/src/vm/host/document.rs:89-100`); the `document_root()` fallback appears in the `ownerDocument` getter and in the href / `documentURL` paths (sampled; the 109 `document_root()` hits are not classified). `Document.prototype.cloneNode` is reachable (`node_proto.rs:171`, `node_methods_extras.rs:201-215`, `install_document_methods_if_cloned_doc`) (scout Q-S9) |
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
  D-C4 keeps that substitution, and D-C7 / U4 are restated.
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
  cannot call it. A single entry therefore requires promoting it to `EcsDom`; who promotes it is O-2 (§11).
- **P-C14 (C0 draft, A3×A4).** "Elements in contents can be custom only if upgraded there; C1 makes that impossible by
  giving contents the inert document." A custom element created in the document and appended into `tpl.content` stays
  custom in the inert document (adopt does not un-upgrade), so custom elements in contents exist on C1's head, and a
  later adoption of their template must give them `adoptedCallback(f(old), f(new))` (D-C5, cell T25). Hand-off H-2.
- **P-C15 (C0 draft × #577).** The C0 draft (frame `1ed126b4`, before #577) does not mention
  `node_document_for_new_nodes`. On C0's base its link-time adopt (D-4) re-homes a strict foreign element, created with
  the context's document by that function in a fragment parse, to the throwaway document at its append (the parent's
  node document), and `finish_detached_fragment` re-homes it back. The intermediate stamp is unobservable (the parse
  runs with the dispatcher taken), but the function's fragment branch then describes a stamp that C0 overwrites.
  Hand-off H-2.
- **P-C16 (T3 draft × C0 draft × this brief, umbrella rows).** The T3 draft (`14ff3dc2`) carries umbrella rev 8.4 / 8.5
  edits that add **both** C0 and C1 rows to §1.2 and edges x7–x10; the C0 draft D-9 says C0's PR adds the C0 row; the
  driver's brief (PM decisions 144 / 159) says C1's PR adds the C1 row. Three branches claim the same lines. O-1 (§11).

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
  rule is idempotent over the pair: `f(D) = f(I) = I`, so a caller may pass either and gets the same answer. A
  read-only lookup `EcsDom::inert_template_document_of(&self, document: Entity) -> Option<Entity>` (NEW) returns
  `f(document)` when it exists without creating it (D-C5 needs it on a `&self` path), and a predicate
  `EcsDom::is_inert_template_document(&self, document: Entity) -> bool` (NEW) reads the marker; the predicate is the
  per-node "template origin" fact slice b's S-2 reads through the node document (§4.6).
- **D-C3 — the establishing steps call the rule once, inside the helper.** `attach_template_contents(template,
  node_document)` keeps its name and its single-attach contract; its second parameter becomes **the template's node
  document** and the helper applies `f` to it. `None` keeps today's orphan fragment (a documentless context or an
  orphan clone). No caller computes I itself. The four callers pass: `createElement` the receiver (`Some(this)`); the
  strict tier the template's node document from the new-node entry (D-C4); the tolerant tier its threaded document,
  obtained from the same entry (D-C4); the clone post-pass the pair's document (D-C6). Through idempotence a template
  nested in contents gets I whether its caller passes D or I.
- **D-C4 — one new-node owner entry; parser creations take their owner from it (PM decisions 199 / 207).** The
  contract of `node_document_for_new_nodes` becomes WHATWG HTML §13.2.6.1's: the new node's document is "intendedParent's
  node document" (*create an element for a token* step 3; text and comments: "targetParent's node document"), read
  through `node_document`, with exactly the one elidex substitution #577 introduced: in a §13.4 fragment build, the
  throwaway document is replaced by the context's node document (the throwaway is destroyed before the caller inserts
  the nodes). Consequences:
  - inside template contents the intended parent's node document is the contents fragment's, which D-C3 made I, so
    the entry returns I there with no template-specific branch; the same holds for nested contents and for shadow
    roots inside contents (C0 stamps a shadow root with its host's node document, C0 draft I-U);
  - **every** parser creation in both tiers (elements, foreign elements, text, comments, the DSD template, doctype,
    template contents) takes its owner from the entry at creation, so creation stamp and C0's link-time stamp agree in
    document parsing (no `Adopt` from D to I for a parsed node; cell U12). In a fragment build C0's link still writes
    the throwaway for linked nodes (P-C15), and `finish_detached_fragment` re-homes them; only the contents fragments,
    which are never linked, keep the substituted stamp;
  - the entry is **one** function for both tiers and for dom-api callers: an `EcsDom` method (NEW home), which the
    strict tree builder and the tolerant converter call with the intended parent and their fragment context. Its
    signature (how the fragment context is passed) is plan-review input (Q-1); whether C0 or C1 introduces it is O-2.
    Rev 1's per-parser private helper and its Q-1 ("a third node-document function") are withdrawn: C0's
    `node_document` is the reader, this entry is the one place that decides a new node's owner.
- **D-C5 — the template adopting steps inside C0's adopt.** DOM §4.5 *adopt* step 3 walks node's **shadow-including
  inclusive descendants**, which do not include template contents; step 3.4 runs each descendant's *adopting steps*,
  and a template's (HTML §4.12.3) adopt its contents into `f(node's new node document)`. C1 replaces C0's identity edge
  with that: the walk re-homes the root's shadow-including tree to `document` and, for each template it reaches, adopts
  the template's contents (recursively the same way) into `f(document)`; nested templates resolve to the same I
  (idempotence). The walk stays uncapped and iterative and carries the target per stacked node (`TemplateContentsWalk::Include`
  keeps its teardown caller; whether the adopt walk is a third variant or a sibling walker is Q-2). **Notification:**
  each custom element inside contents receives `adoptedCallback` with « its old node document, its new one » =
  « `f(old)`, `f(document)` », in the spec's order (the contents' descendants immediately after their template, step
  3.4 inside the per-descendant loop). C0's draft consumer walks the root's subtree with contents included and passes
  the event's single `new_document` to every element (C0 draft D-6, A3×A4), which under C1 is wrong for elements in
  contents (P-C14). Rev 2 decides the outcome (cell T25); the mechanism — the consumer's walk switching (old, new) to
  (`f(old)`, `f(new)`) at each contents edge from one event (recommended: spec order from one event, `f` read through
  `inert_template_document_of`), or one nested `Adopt` per contents fragment — is Q-3.
- **D-C6 — the template cloning steps, and one document per clone pair.** The copy's contents are established at the
  copy's creation through D-C3 with the pair's document, and the source contents' children are cloned with **document
  = the copy's contents' node document** (HTML §4.12.3 *cloning steps* step 2), not the operation's. Every later pass
  for a pair inside those children (a nested template's contents; a clonable shadow host's shadow root, its host stamp
  and its shadow children) uses **that pair's document**, not the operation-wide `shadow_doc` (H-C7: at HEAD the
  shadow pass stamps the operation's document onto a host inside copied contents, `clone.rs:283-284`). The worklist
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
(C0 draft §4.4, C0-2). §6 T9 is the K-0 guard for rows 1–2.

**K-0 lens (umbrella K-0, S-3: no step worse than HEAD).** On C1's base (N1, C0, D14), C1's observable changes are
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
| M3 | `I = tpl.content.ownerDocument; [I.defaultView, I.URL, I.baseURI, I.documentURI, I.body, I.ownerDocument]` | `[null, 'about:blank', 'about:blank', 'about:blank', null, null]` (browsing context null; URL is the default; §2.4.3 fallback base URL) | n/a (I is `document`) | `defaultView` / `URL` per H-C16's fallbacks: unmeasured (Q-S10) | | |
| M4 | `tpl.innerHTML = '<a href="x.html"></a>'; tpl.content.firstChild.href` | the `href` content attribute value `"x.html"` (base `about:blank`: the URL does not parse, so the getter returns the attribute) | absolute against the page base | `"x.html"` | | |
| M5 | parser path: page markup `<template id=h><a id=a href="x.html"></a><img id=i src="y.png"></template>`; `[h.content.getElementById('a').href, h.content.getElementById('i').src]` | `["x.html", "y.png"]`-shaped (as M4) | absolute | as M4 | | |
| M6 | `c = tpl.content.cloneNode(true)` with M4's `<a>`; `[c.ownerDocument === tpl.content.ownerDocument, c.firstChild.href]`; `document.body.append(c)`; `a.href` | `[true, "x.html"]`, then absolute | `[true, absolute]`, then absolute | spec | | |
| M7 | `t2 = tpl.cloneNode(true); t2.content.ownerDocument === tpl.content.ownerDocument` | `true` (copy created in `document`; establishing step 1) | `true` (both `document`) | `true` (both I) | | |
| M8 | `dp = new DOMParser().parseFromString('<template id=t></template>','text/html'); x = dp.getElementById('t').content.ownerDocument; [x !== dp, x !== tpl.content.ownerDocument]` | `[true, true]` | `[false, true]` | `[true, true]` | | |
| M9 | `dc = document.cloneNode(true)` (page with a `<template>`); `x = dc.querySelector('template').content.ownerDocument; [x !== dc, x !== tpl.content.ownerDocument]` | `[true, true]` | `[false, true]` | `[true, true]` | | |
| M10 | nested: `<template id=o><template id=i></template></template>`; `[i.ownerDocument === o.content.ownerDocument, i.content.ownerDocument === o.content.ownerDocument]` | `[true, true]` | `[true, true]` (all `document`) | `[true, true]` (all I) | | |
| M11 | `tpl.innerHTML = '<b id=x onclick="window.hit=1"></b>'; b = tpl.content.firstChild; [b.onclick, (b.dispatchEvent(new Event('click')), window.hit)]` | `[null, undefined]` (*get the current value of the event handler*: browsing context null) | `[function, 1]` | `[null, undefined]` | | |
| M12 | `<template id=s><div id=h><template shadowrootmode=open><span></span></template></div></template>`: `[s.content.getElementById('h').shadowRoot !== null, …shadowRoot.ownerDocument === s.content.ownerDocument]` | DSD attaches inside contents when the parser's flag is set (§13.2.6.4.4); then `[true, true]` | `[true, true]` (both `document`) | `[true, true]` (both I) | | |
| M13 | custom element in contents: `customElements.define('x-a', class extends HTMLElement { adoptedCallback(o, n) { log.push([o, n]) } })`; `e = document.createElement('x-a'); tpl.content.append(e); log` | `[[document, I]]` (insert step 7.1 adopts into the contents' node document; `e` stays custom) | `[]` | `[[document, I]]` (needs C0's `Adopt`) | | |

M3, M4–M6 and M13 are the rows where elidex's planned value is a reading (M3's unbound-Document methods, M4–M5's
unparsable relative URL, M13's custom element staying custom in I). The remaining rows confirm the spec reading.

## §2. Coupled invariants

### §2.1 Invariant per axis

| axis | invariant |
|---|---|
| A1 rule | For every Document D, `f(D)` is one Document I carrying `InertTemplateDocument`, the same entity on every call; `f(I) = I`. No site computes a template contents' node document except through `f` (D-C2, D-C3) |
| A2 creation | At every template creation the contents fragment's node document is `f(template's node document)`. Every node a parser creates takes its owner at creation from the one new-node entry, which returns `node_document(intended parent)` (the throwaway replaced by the context's document in a fragment build); in both tiers, in document, fragment and DOMParser parses (D-C3, D-C4) |
| A3 nesting | A template at any depth inside contents of D's templates has node document I and contents node document I; there is one I per non-inert Document, never an I of I |
| A4 shadow | A shadow root inside template contents (declarative or `attachShadow`) and its content report I (C0's host-link stamp, A2's entry); the adopt and clone walks reach them under the contents' document |
| A5 clone | The copy's contents are established in `f(pair document)`; their children, and every pass for a pair among them, use `f(pair document)`; a clone of contents is in I; a clone of D or of I is a non-inert Document with no link |
| A6 adopt | Adopting a template into X (C0's insertion adopt, or the fragment parse's early re-home) re-homes the template and its shadow-including descendants to X and its contents (and nested contents) to `f(X)`; a custom element in contents gets `adoptedCallback(f(old), f(X))` in spec order |
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
| A2×A3 | yes | Strict: the entry reads the intended parent's node document, I inside outer contents; the inner template's contents get `f(I) = I`. Tolerant: the threaded document for an inner template is the outer contents' entry value, I; either maps to I (T4, both tiers) |
| A2×A4 | yes | Declarative shadow content inside contents: the strict DSD path (`attach_declarative_shadow`, `shadow.rs:136`) and the tolerant DSD recursion (`try_attach_declarative_shadow`, `convert.rs:120`) create the shadow content with the entry's value for the shadow root, which C0 stamped with the host's node document I (T12; Q-S2) |
| A2×A5 | yes | The clone post-pass is an establishing caller (D-C3) and site A10 of the new-node census: it passes the pair's document, not the entry's (a clone's document is the operation's, DOM §4.4 *clone a single node*), so the clone path is the one creation path that does not call the entry (§4.5) |
| A2×A6 | yes | Fragment build: the entry substitutes the context's document for the throwaway; contents fragments are stamped `f(context doc)` at attach and never re-homed by C0's link (they are not linked); light nodes are re-homed to the throwaway by C0's link (P-C15) and back by `finish_detached_fragment`, whose walk (D-C5) maps contents to `f(context doc)` = their stamp already (T14 parity, both tiers) |
| A2×A7 | yes | No `f(throwaway)` is ever computed (D-C4), so neither the success path nor the strict rollback (`despawn_subtree(document)`, which reaches contents with `Include`) creates or leaks an I (U4, U5) |
| A2×A8 | yes | DOMParser: `dp`'s templates' contents are in `f(dp)`, a third document, read null (N1) and, after T3, target null, exactly as `dp`'s own nodes (T6) |
| A2×A9 | yes | The pre-bind parse puts contents in I before bind; slice a's bind pass still excludes them (U8 (ii) unchanged). Slice b's S-1 extension processes them at creation-equivalent time; C1 gives them a non-bound node document so their raw values read null (§4.6) |
| A3×A4 | yes | A template inside a shadow tree inside contents, and a shadow host inside nested contents: all report I (T12 variant) |
| A3×A5 | yes | Deep clone of an outer template clones the inner template inside the copy's contents with pair document I; the inner copy's contents are `f(I) = I` (T10) |
| A3×A6 | yes | Adopting an outer template into X moves outer contents and every nested template and nested contents to `f(X)`; the target switches once, at the first contents edge (U9) |
| A3×A7 | no | Nesting creates no further document (A1×A3) |
| A3×A8 | yes | Nested contents read null and (after T3) target null like outer contents (same I) |
| A3×A9 | no | The EH contract does not distinguish depth inside contents |
| A4×A5 | yes | A clonable shadow host inside copied contents: its shadow root, host stamp and shadow children use the pair's document I, not the operation's (`clone.rs:283-284` at HEAD stamp the operation's) (T24). D14's subtree-true form is assumed (§4.4) |
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
| A6×A9 | yes | C0's adoption precedes the Insert event (C0-3), so slice a's Insert arm (T-a2) and N1's gate see the adopted document; a node placed into contents is processed at T-a2 with node document I and reads null (W-K5's template half closes; R-3) |
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
1–2).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.12.3 The template element | establishing the template contents, steps 1–3 | 1 document = node document's appropriate template contents owner document; 2 fragment with that node document and host = template; 3 link | `EcsDom::attach_template_contents` `crates/core/elidex-ecs/src/dom/mod.rs:557` (D-C3); step 2's *host* is §7 | ✗ (host not modelled) | yes |
| WHATWG HTML §4.12.3 The template element | *appropriate template contents owner document*, steps 1–2 | 1 not created by this algorithm → get-or-create the associated inert template document (1.1.1 create, browsing context null; 1.1.2 HTML flag; 1.1.3 link), set document to it; 2 return | `appropriate_template_contents_owner_document` (NEW) in `crates/core/elidex-ecs/src/dom/template_owner.rs` (NEW) (D-C2); 1.1.2 vacuous (H-C13) | ✓ | no |
| WHATWG HTML §4.12.3 The template element | template *adopting steps*, steps 1–2 | document = node's new node document's appropriate owner; adopt contents into it | C0's adopt walk (`EcsDom::adopt_subtree`, `crates/core/elidex-ecs/src/dom/tree/teardown.rs:251` at `593aa314`; moved by C0 D-3) and C0's `Adopt` consumer (D-C5) | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | template *cloning steps* (node, copy, subtree), steps 1–2 | subtree false → return; else clone each contents child with document = copy's contents' node document, subtree true, parent = copy's contents | `replicate_template_contents` `crates/dom/elidex-dom-api/src/node_methods/clone.rs:405` (D-C6) | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | `content` getter | return the template contents | `crates/script/elidex-js/src/vm/host/html_template_proto.rs:106` (unchanged) | ✓ | no |
| WHATWG DOM §4.5 Interface Document | *adopt* steps 1–3 | 3 document ≠ oldDocument → for each shadow-including inclusive descendant: 3.1 set node document; 3.2 / 3.3.1–3.3.2 registries and attributes (not C1's); 3.3.3 `adoptedCallback`; 3.4 adopting steps | C0's `adopt` / `adopt_subtree` (C0 D-2, D-3) and `MutationEvent::Adopt` (NEW: C0) consumer (C0 D-6); C1 edits the contents edge and the contents arguments (D-C5) | ✗ | yes |
| WHATWG DOM §4.2.3 Mutation algorithms | *insert* step 7.1; *replace* step 6 | adopt node into parent's node document | C0 (C0 D-4); not touched by C1 | ✗ | yes |
| WHATWG DOM §4.4 Interface Node | *clone a node* steps 2, 3, 5, 6.7 | 2 copy created in document (runs the establishing steps); 3 cloning steps; 5 children with the same document; 6.7 shadow children with the same document, subtree true | `clone_node_with_shadow_honor` `clone.rs:227` (D-C6, on D14), `clone_subtree` `crates/core/elidex-ecs/src/dom/tree_clone.rs:206-210` (unchanged) | ✓ | yes |
| WHATWG DOM §4.4 Interface Node | *node document*; `ownerDocument` getter | a node's node document; a document's is itself; getter null for a document | C0's `EcsDom::node_document` (NEW: C0) and `owner_document` `mod.rs:817`; getter `crates/dom/elidex-dom-api/src/node_methods/core.rs:279` (unchanged by C1; new values) | ✓ | no |
| WHATWG HTML §13.2.6.1 Creating and inserting nodes | *appropriate place for inserting a node*; *create an element for a token* step 3; text and comment insertion | a template target parent → its template contents; document = intendedParent's node document; text / comment = targetParent's node document | the new-node entry (D-C4) behind `node_document_for_new_nodes` (strict `TreeBuilder` method) `crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs:271`; strict creation sites `tree_builder/insert.rs:55`, `:96`, `:131`, `:140`, `:193`; tolerant `convert_node` `crates/dom/elidex-html-parser/src/convert.rs:237` | ✓ | yes |
| WHATWG HTML §13.2.6.4.4 The "in head" insertion mode | start tag "template", the declarative branch ("Keep") | the declarative shadow root attaches when the parser's allow-declarative-shadow-roots flag is true, inside template contents too | strict `attach_declarative_shadow` `tree_builder/shadow.rs:136`, `attach_ordinary_template_contents` `:123` (D-C3); tolerant `try_attach_declarative_shadow` `convert.rs:120` | ✗ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | the throwaway Document and the returned nodes | templates parsed in the build get the context's inert document (D-C4's substitution); the finishing re-home and the caller's insertion adopt | `begin_detached_fragment` `teardown.rs:293`, `finish_detached_fragment` `teardown.rs:330`; tolerant `crates/dom/elidex-html-parser/src/lib.rs:158` (D-C4, D-C5) | ✗ | yes |
| WHATWG HTML §8.5.4 The innerHTML property | setter on a template: the target is its template contents | parsed nodes replace the contents' children (inserted, so adopted into I by C0) | `crates/script/elidex-script-session/src/mutation/html_fragment.rs` (unchanged; C0 adopts) | ✗ | yes |
| WHATWG HTML §8.5.1 The DOMParser interface | `parseFromString` `text/html` | the new document is "not created by this algorithm": its templates get its own inert document | `parse_into_inert_document` `crates/dom/elidex-form/src/inert_document.rs:110` (unchanged; values change via D-C3) | ✗ | yes |
| WHATWG HTML §2.4.3 Document base URLs | *fallback base URL* steps 1–2 | I's URL is `about:blank` and its about base URL is null → base `about:blank` | `DocumentBaseUrl(about_blank_url())` from `create_document_node` `mod.rs:410`; read by `href_accessor.rs:50` (§1.1 M4–M6) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *get the current value of the event handler* step 3; *determine the target of an event handler* step 3 | raw value in a document whose browsing context is null → null; node document not an active document → null target | N1's compile gate (C0's `node_document` form of clause (b), `crates/script/elidex-script-session/src/scripting.rs:95-113` at `593aa314`; doc `:59-60` rewritten); T3 (after C1) | ✗ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *shadow-including inclusive descendant* | adopt's walk order (contents excluded; reached through step 3.4) | `for_each_uncapped_shadow_inclusive` `crates/core/elidex-ecs/src/dom/tree/walkers.rs:99` (D-C5 shape is Q-2) | ✓ | no |

### §3.1 User-input touch audit

Rows marked *yes* carry author markup or script-constructed trees. C1 parses no new input and stores no string: the
two components hold an `Entity` and nothing. What changes is which Document a node of author content is attributed
to, and with it four reads: the `ownerDocument` getter (a new Document wrapper becomes reachable from script: I, whose
browsing context is null; §1.1 M3, Q-S10), URL resolution (relative URLs in template contents no longer resolve
against the page, §1.1 M4–M6), the handler gates (raw handlers in template contents read null and do not run: the
spec's outcome, more restrictive than HEAD), and `adoptedCallback`'s arguments for custom elements in contents (§1.1
M13). No path delivers author content to a document other than one it was parsed into, cloned into or adopted into.

## §4. Mechanism

### §4.1 Home / kind

- The rule, the lookup, the predicate and the lifetime hook live in **`elidex-ecs`**
  (`crates/core/elidex-ecs/src/dom/template_owner.rs` (NEW), an `impl EcsDom` block): both parser crates and
  `elidex-dom-api` call them, and the parser crates cannot depend on `elidex-dom-api` (umbrella §9 N-1's crate-graph
  evidence). The new-node entry (D-C4) lives in `elidex-ecs` for the same reason (O-2 decides which PR adds it).
  `elidex-ecs` is engine-independent, so the Layering mandate holds; the VM touches none of it (the `content` and
  `ownerDocument` getters marshal only).
- Components (D-C1) in `crates/core/elidex-ecs/src/components.rs` beside `TemplateContents` (`:366`).
- Realm: §4.12.3 step 1.1.1 creates I "given document's relevant realm". elidex has one realm per `EcsDom` (T3 draft
  H-T15); B1's multi-realm World must create I in D's realm (§8 H-3).

### §4.2 The rule and the establishing helper (D-C2, D-C3)

`appropriate_template_contents_owner_document(D)`: marker on D → D; link on D whose target `contains` and is a Document
→ that; else spawn with `create_document_node`, insert `InertTemplateDocument`, insert
`AssociatedInertTemplateDocument(I)` on D, return I. A stale link (target destroyed or recycled) is replaced, never
returned (D-C7 makes it unreachable; the check is defensive, as `node_document`'s guard is). `inert_template_document_of`
is the same lookup without the spawn. `attach_template_contents` calls the rule on `Some(node_document)`, then creates
the fragment with that owner, as today.

### §4.3 The new-node entry (D-C4) and the parser sites

Contract (the signature is Q-1): given the node whose node document the spec names (the intended parent / target
parent), and, in a §13.4 build, the throwaway document and the context, return `node_document(origin)`, except that the
throwaway maps to `node_document(context)`. Callers on C1's head:
- strict: `node_document_for_new_nodes` (strict `TreeBuilder` method) becomes a thin wrapper taking the intended parent (it already has
  the throwaway as `self.document` and the context as `self.state.fragment_context`); every creation site of H-C4 and
  both of H-C3 pass their `parent` (`appropriate_place()`), and `attach_ordinary_template_contents` passes the template's
  node document (the template was just inserted, so its node document is the entry's value for its parent);
- tolerant: `convert_node` and `try_attach_declarative_shadow` take the parent entity they convert into and call the
  entry; `lib.rs:158` stops choosing the document itself (the fragment branch is the entry's);
- dom-api: L3's A11 / A12 (§4.5).
The tolerant converter keeps its threaded `owner_document` argument only if Q-1 keeps it as the cached entry value; it
is never a second decision.

### §4.4 Clone (D-C6), on D14

On C1's base D14 has rewritten pass 4's child loop (`clone.rs:290-302` at `593aa314`) to clone shadow children with
subtree **true** (DOM §4.4 step 6.7; P-C12), which makes the shallow branch (`clone.rs:295-300`, the explicit stamp of a
shallow-cloned shadow child) unnecessary. C1 then:
1. gives each worklist pair the document its copy was made in: the root pair `shadow_doc` (today's value), every pair
   produced under a pair P the document P's children were cloned with;
2. in pass 3, `replicate_template_contents(s, d, …, pair_doc, …)`: `new_fragment = attach_template_contents(d, pair_doc)`
   (mapped inside), then clones each source contents child with `node_document(new_fragment)` (= `f(pair_doc)`) and
   records those pairs with that document;
3. in pass 4, the host stamp, the shadow-root stamp and the shadow children use `pair_doc` (`clone.rs:283-284`, `:294`
   at `593aa314`).
Whether the pair document is a third tuple field or read back from the stamp the deep cloner wrote (`tree_clone.rs:221`,
`:417`) depends on whether the shallow root is stamped on the base (`#11-ownerless-node-creation`'s shallow-clone site,
L3's D8): if every copy is stamped at creation, reading `node_document(d)` suffices; otherwise the field. Edge c-D14
(§8) records the rebase.

### §4.5 L3's Range creation sites (A10–A12) and the rest of the census

- **A10** (template clone; `#11-template-contents-owner-document`'s scope) is §4.4.
- **A11 / A12** (L3 RC-4's body context element and result fragment of `createContextualFragment`, planned to read the
  new-node entry; the method is a stub at `593aa314`, `crates/script/elidex-js/src/vm/host/range_proto_mutation.rs:250`):
  C1's obligation is that the entry returns the right document for a Range whose start node is in template contents,
  which is I — the start node's node document (DOM's "creating an element given this's start node's node document").
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

### §4.7 What C1 takes from C0 (C0 draft §4.4)

| C1 needs | C0 draft's answer | C1's use |
|---|---|---|
| one reader of a node's node document | `EcsDom::node_document` (NEW: C0) (D-1); `owner_document` = getter wrapper | every read in C1's code (D-C10) |
| every insertion adopts, before `Insert` | D-4 at the three primitives; D-5 for replace-with-fragment | §1 rows 1–5; T7, T8, T9, T13 |
| a contents fragment's node document is the parent document for nodes placed into it | C0-2: `node_document(fragment)` = its stamp | T7, T8 |
| the one adopt walk to edit | D-3: `adopt_subtree` (moved to `mutation.rs`), `Include` walk | D-C5 replaces the contents edge |
| one adoption notification | D-6: `MutationEvent::Adopt` (NEW: C0) per adopted root; custom-element consumer | D-C5's contents arguments (Q-3; P-C14) |
| the fragment parse's early re-home without events | D-8 | A2×A6 |

### §4.8 ECS-native and Layering checks

- Representation: two components, no side-store (D-C1). Lifetime by despawn cascade, not refcount or sweep (D-C7).
- One rule (D-C2), one establishing helper (D-C3), one new-node entry (D-C4), one adopt walk (D-C5, C0's), one reader
  (C0's `node_document`): §5.1's populations converge on them.
- VM: no new host code; `html_template_proto.rs` keeps marshalling the link (`:106`).

### §4.9 Docstring citation table (Axis 4)

| symbol | citation |
|---|---|
| `appropriate_template_contents_owner_document` (NEW) | HTML §4.12.3 *appropriate template contents owner document*, steps 1–2 (1.1.2 vacuous: no XML documents) |
| `inert_template_document_of` (NEW) | HTML §4.12.3 *associated inert template document* (lookup only) |
| `AssociatedInertTemplateDocument` (NEW) | HTML §4.12.3 *associated inert template document* |
| `InertTemplateDocument` (NEW) | HTML §4.12.3 ("a Document created by this algorithm") |
| `is_inert_template_document` (NEW) | HTML §4.12.3 (the same fact) |
| `attach_template_contents` (doc rewrite) | HTML §4.12.3 establishing steps 1–3 (step 2's host not modelled, §7) |
| the new-node entry (doc rewrite or NEW, O-2) | HTML §13.2.6.1 *create an element for a token* step 3, text / comment insertion; §13.4 (the throwaway substitution) |
| C0's adopt walk (doc rewrite of the contents edge) | DOM §4.5 *adopt* steps 3.1, 3.3.3, 3.4; HTML §4.12.3 template *adopting steps* |
| `replicate_template_contents` (doc rewrite) | HTML §4.12.3 template *cloning steps* step 2 |

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
  (verified 2026-11-01). D14 removes `clone.rs:299`; C1 changes the document of `clone.rs:283-284` (D-C6).
- **P-4 parser owner decisions**: `/usr/bin/grep -rn 'node_document_for_new_nodes\|owner_document(' crates/dom/elidex-html-parser-strict/src
  crates/dom/elidex-html-parser/src --include='*.rs' | /usr/bin/grep -v "$NT"` → strict `mod.rs:271`, `:273`,
  `insert.rs:107-108`, `shadow.rs:119`, `:124`; tolerant `lib.rs:158` (verified 2026-11-01). After C1: the entry's
  definition, its strict wrapper and its callers only; no `owner_document(` in either parser crate.
- **P-5 parser creation sites**: `/usr/bin/grep -rn 'create_\(element\|text\|comment\|document_type\)[a-z_]*(' crates/dom/elidex-html-parser-strict/src/tree_builder
  crates/dom/elidex-html-parser/src --include='*.rs' | /usr/bin/grep -v "$NT"` → strict `insert.rs:61`, `:111`, `:133`,
  `:141`, `:155`, `:210`, `shadow.rs:153`; tolerant `convert.rs:251`, `:305`, `:309`, `:316` (11, verified 2026-11-01).
  After C1 every one passes the entry's value (D-C4).
- **P-6 node-document consumers** (values change for nodes in contents, code unchanged by C1): the C0 draft's P-3
  census (17 production readers at `1ed126b4`; C0 changes four, the rest read `node_document` values). Re-run at C1's
  base: `/usr/bin/grep -rn 'node_document(\|owner_document(' crates --include='*.rs' | /usr/bin/grep -v "$NT"`.
  Scout Q-S4's classification (2026-10-31) found no reader whose new value for a node in contents is not the spec's
  except the two Q-S10 rows (`documentURL` / href paths' `document_root()` fallback on I) and `focus/delegate.rs:85`
  and `scripting.rs:109`, marked "needs check" — `scripting.rs:109` is C0's (D-7 (a)); `focus/delegate.rs:85` is
  Q-S11.
- **P-7 C2 statements**: `/usr/bin/grep -rn 'template-contents-owner-document\|C2 approximation' crates` →
  `mod.rs:550`, `:552`, `scripting.rs:60`, `document.rs:261-262`, `elidex-html-parser/src/tests.rs:566` (verified
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
inverted, T17). C18's test (`tests_event_handler_attrs_window_delegating.rs`) is **unchanged** at C1 (T18). The complete
list is re-derived at C1's base (C0 and D14 change neighbouring tests).
Docstrings: P-7, plus `components.rs:354-364` (`TemplateContents`' adopt sentence), `walkers.rs:19-27`
(`TemplateContentsWalk::Include`'s "re-home" use, if the adopt walk stops using it, Q-2), `attach_template_contents`'s
"strict: post-insert `owner_document(template)`" (stale since #577), `node_document_for_new_nodes`'s doc (strict `tree_builder/mod.rs:267-270`),
`replicate_template_contents`'s "the C2 owner-doc approximation" (`clone.rs:398-399`), `scripting.rs:59-60`.

### §5.3 Touched files

Line counts: `wc -l` at `593aa314` (verified 2026-11-01); re-measured at C1's base (§9 item 1).

| file | lines at `593aa314` | change | 1000-line decision |
|---|---|---|---|
| `crates/core/elidex-ecs/src/dom/template_owner.rs` (NEW) | — | D-C2 rule, lookup, predicate, D-C7 hook | new file, by design: keeps `mod.rs` under the limit |
| `crates/core/elidex-ecs/src/dom/mod.rs` | 926 | `mod` line; `attach_template_contents` doc and one call | net ≤ +10 lines, stays < 1000 (C0 nets ≈ −10); if the base is ≥ 990, the template-contents helpers (`:536-588`) move to `template_owner.rs` as a standalone pure-move prereq first (O-3) |
| `crates/core/elidex-ecs/src/components.rs` | 822 | two components, `TemplateContents` doc | stays < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/mutation.rs` (C0's home of the adopt walk) | 467 | D-C5 contents edge | < 1000 (C0 adds `adopt`; re-measure) |
| `crates/core/elidex-ecs/src/dom/tree/teardown.rs` | 344 | Document cascade in `destroy_entity` (D-C7) | < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/walkers.rs` | 392 | per Q-2 | < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/navigation.rs` | 375 | `root_entities` exclusion (D-C9) | < 1000 |
| `crates/core/elidex-ecs/src/dom/tree_clone.rs` | 425 | clone-policy rows (D-C8) | < 1000 |
| `crates/dom/elidex-custom-elements/src/consumer.rs` | 329 | C0's `Adopt` arm: contents arguments (D-C5, Q-3) | < 1000 |
| `crates/dom/elidex-dom-api/src/node_methods/clone.rs` | 458 | D-C6 on D14 | < 1000 |
| `crates/dom/elidex-dom-api/src/document.rs` | 665 | comment only (P-7) | < 1000 |
| `crates/dom/elidex-html-parser/src/convert.rs` | 678 | D-C4 call sites | < 1000 |
| `crates/dom/elidex-html-parser/src/lib.rs` | 258 | the fragment entry's document choice (D-C4) | re-measure |
| `crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs` | 493 | `node_document_for_new_nodes` wrapper | < 1000 |
| `crates/dom/elidex-html-parser-strict/src/tree_builder/insert.rs` | 380 | D-C4 call sites | < 1000 |
| `crates/dom/elidex-html-parser-strict/src/tree_builder/shadow.rs` | 218 | D-C4 call sites, doc | < 1000 |
| `crates/script/elidex-script-session/src/scripting.rs` | 293 | doc `:59-60` (C0 rewrites the surrounding paragraph) | < 1000 |
| `crates/dom/elidex-html-parser/src/tests.rs` | 875 | two expectations (§5.2) | < 1000 |
| `crates/dom/elidex-dom-api/src/node_methods/tests/clone.rs` | 992 | one expectation rewritten in place; **no new tests** (they go to a new file) | stays < 1000 (D14 may move it; re-measure, split first if D14 pushes it past 1000) |
| `crates/core/elidex-ecs/src/dom/tests/template_owner.rs` (NEW) | — | unit cells U-* (§6) | new file |
| `crates/script/elidex-js/src/vm/tests/tests_template_owner_document.rs` (NEW) | — | VM cells T-* (§6) | new file |
| `crates/script/elidex-js/src/vm/tests/tests_html_t2d_protos.rs` | 1207 | **not touched** (`/usr/bin/grep -n 'owner' <file>` → `output_form_owner` only) | not triggered |

### §5.4 Named risks (and the observable change each is)

- **R-C1 `ownerDocument` of template contents** → I (spec). A script that compared it to `document` sees `false`.
- **R-C2 URL resolution in template contents** → against `about:blank` (§2.4.3), so `a.href` / `img.src` IDL reads of
  relative URLs return the attribute value while inside contents. Gated on §1.1 M4–M6; cell T20.
- **R-C3 handlers in template contents** → raw values read null and do not run (N1 + C0 + C1; C14 inverts, T17).
- **R-C4 `<body>` / `<frameset>` in template contents** → unchanged at C1 (the base's target rule ignores the
  document, T18); T3 (after C1) gives them the null target.
- **R-C5 a new Document wrapper (I) is reachable from script.** Document methods on it act on I (H-C16); the
  `document_root()` fallbacks in the href / `documentURL` paths may report the page for I (Q-S10; §1.1 M3). DOMParser
  documents are the precedent.
- **R-C6 I as a root.** Excluded (D-C9); cell U6.
- **R-C7 lifetime.** No I for a throwaway (U4, U5); I per DOMParser document lives with it (D-C7, §7).
- **R-C8 `adoptedCallback` arguments for custom elements in contents** → « `f(old)`, `f(new)` » (D-C5; T25, §1.1 M13).

## §6. Test plan

Columns: *HEAD* = `593aa314`; *base* = HEAD + N1 implementation + C0 + D14 (still C2); *C1 = spec*. **Red-first**: every
*red* cell is run on C1's base before the fix with its control green, and the failure is recorded in the PR description.
Cross-tier: cells marked (×2) run under both parser tiers. A cell whose spec value depends on §1.1 is asserted only
after the measurement agrees (D-C11).

### §6.1 VM integration (`tests_template_owner_document.rs`, NEW)

| cell | kind | script | base | C1 = spec |
|---|---|---|---|---|
| T1 | red | `t = document.createElement('template'); I = t.content.ownerDocument; r = [I !== document, I instanceof Document, I.defaultView === null]` | `[false, true, false]` | `[true, true, true]` (third element after §1.1 M3) |
| T2 | guard | two `createElement('template')`: `t1.content.ownerDocument === t2.content.ownerDocument` (with T1's `!== document`) | `true` (both `document`) | `true` (same I) |
| T3 | red (×2) | initial HTML `<template id=a><b id=b></b></template>`: `a.content.ownerDocument !== document`, `a.content.getElementById('b').ownerDocument === a.content.ownerDocument` | `[false, true]` | `[true, true]` |
| T4 | red (×2) | `<template id=o><template id=i><p></p></template></template>`: `i.ownerDocument === o.content.ownerDocument`, `i.content.ownerDocument === o.content.ownerDocument`, `o.content.ownerDocument !== document` | `[true, true, false]` | `[true, true, true]` |
| T5 | guard (with U2) | `I = tpl.content.ownerDocument; t = I.createElement('template'); t.content.ownerDocument === I` | `true` (`I` is `document`) | `true`, and U2 shows no I-of-I entity |
| T6 | red | `dp = new DOMParser().parseFromString('<template id=t><i></i></template>','text/html'); c = dp.getElementById('t').content; [c.ownerDocument !== dp, c.ownerDocument !== document, c.ownerDocument !== tpl.content.ownerDocument, c.firstChild.ownerDocument === c.ownerDocument]` | `[false, true, true, true]` | `[true, true, true, true]` |
| T7 | red | `tpl.innerHTML = '<b id=x></b>'; [tpl.content.firstChild.ownerDocument === tpl.content.ownerDocument, tpl.content.firstChild.ownerDocument !== document]` | `[true, false]` | `[true, true]` |
| T8 | red | `d = document.createElement('div'); tpl.content.append(d); [d.ownerDocument === tpl.content.ownerDocument, d.ownerDocument !== document]` | `[true, false]` | `[true, true]` |
| T9 | guard (K-0; §1 rows 1–2) | `c = tpl.content.cloneNode(true)` with `<a href="x.html">`; `r1 = c.firstChild.ownerDocument !== document`; `document.body.append(c)`; `[a.ownerDocument === document, a.href.endsWith('/x.html')]` | `false`, then `[true, true]` | `true`, then `[true, true]` (C1 without C0: `[false, false]`) |
| T10 | red | deep and shallow `tpl.cloneNode()` of a template with a nested template: `t2.content !== tpl.content`, `t2.content.ownerDocument === tpl.content.ownerDocument`, the nested copy's `content.ownerDocument` is the same, `!== document`; shallow copy's contents empty and `!== document` | `!== document` legs false | all true |
| T11 | red | `dc = document.cloneNode(true)`; `x = dc.querySelector('template').content.ownerDocument; [x !== dc, x !== tpl.content.ownerDocument]` | `[false, true]` | `[true, true]` |
| T12 | red (×2) | `<template id=t><div id=h><template shadowrootmode=open><span></span></template></div></template>`: `[h.shadowRoot.ownerDocument === t.content.ownerDocument, span.ownerDocument === t.content.ownerDocument, h.shadowRoot.ownerDocument !== document]`; variant: a parser-built `<div>` in contents, `attachShadow({mode:'open'}).ownerDocument !== document` | the `!== document` legs false | all true (DSD inside contents: Q-S2 says it attaches in strict; tolerant re-checked) |
| T13 | red | `dp`'s template (with a nested template and a shadow host in its contents) appended into `document.body`: `t.ownerDocument === document`, `t.content.ownerDocument === tpl.content.ownerDocument`, nested contents and shadow content likewise | `t.content.ownerDocument === document` | `=== tpl.content.ownerDocument` (I of `document`) for all |
| T14 | red (×2) | `el.innerHTML = '<template id=f><p></p></template>'` (`el` in `document`): `[f.content.ownerDocument === tpl.content.ownerDocument, f.content.firstChild.ownerDocument === tpl.content.ownerDocument, f.content.ownerDocument !== document]` | `[true, true, false]` | `[true, true, true]` |
| T17 | red (inverts umbrella C14; needs N1, C0) | umbrella C14's script | runs, `window.hit === 1` | does not run; `x.onclick === null` |
| T18 | guard (umbrella C18 unchanged at C1) | umbrella C18's script | C18's "as HEAD" column | the same (the base's target rule ignores the document); T3 rewrites C18 after C1 (T3 draft P-T9) |
| T19 | red (×2) | `<template id=s><svg id=v></svg><b id=w></b></template>`: inside contents `[v.ownerDocument !== document, v.ownerDocument === s.content.ownerDocument]`; `document.body.append(s.content)`; `[v.ownerDocument === document, w.ownerDocument === document]` | inside `[false, true]`; after `[true, true]` | inside `[true, true]`; after `[true, true]` |
| T20 | spec pin (§1.1 M4–M5) | `<template id=h><a id=a href="x.html"></a></template>`: `h.content.getElementById('a').href` | absolute against the page base | the measured value, if it is §1.1's spec value `"x.html"`; otherwise the PM's decision |
| T22 | red | `tpl.innerHTML = '<b id=x onclick="1">'`; read `x.onclick` | a function (`scripting.rs:59-60`'s false negative) | `null` |
| T23 | red (**R-A closure**, §9 item 9; needs N1, C0) | for each route by which a `<b onfocus="window.hit=1">` reaches template contents — (i) parsed (×2), (ii) `tpl.content.append(b)` of a `createElement`'d `b`, (iii) `tpl.content.cloneNode(true)` of contents holding it, (iv) `tpl.cloneNode(true)` of a template holding it — read `[b.ownerDocument === tpl.content.ownerDocument, b.ownerDocument !== document]`; for route (ii) also `b.onfocus === null` (the only route on which the base writes a raw value: slice a's Insert arm, W-K5; routes (i), (iii), (iv) are unprocessed at both base and C1 — K-5 and the G5 / K3 window — so their handler leg is slice b's, not this cell's) | `[true, false]` for every route; route (ii) `b.onfocus === null` → `false` | `[true, true]` for every route; route (ii) → `true` |
| T24 | red | clonable shadow host in contents: `tpl.innerHTML = '<div id=h></div>'`; `h = tpl.content.firstChild; h.attachShadow({mode:'open', clonable:true}).innerHTML = '<i></i>'`; `c = tpl.cloneNode(true); ch = c.content.firstChild; [ch.ownerDocument !== document, ch.shadowRoot.ownerDocument !== document, ch.shadowRoot.firstChild.ownerDocument !== document]` | `[false, false, false]` | `[true, true, true]` (C1 without D-C6's per-pair document: `[false, false, false]`, the host re-stamped with `document`) |
| T25 | red (§1.1 M13; needs C0's `Adopt`) | `I = tpl.content.ownerDocument`; `x-a` with an `adoptedCallback(o, n)` logging the pair; `dp = new DOMParser().parseFromString('<template id=t></template>','text/html'); I_dp = dp.getElementById('t').content.ownerDocument`; `e = document.createElement('x-a'); dp.getElementById('t').content.append(e)`; `document.body.append(dp.getElementById('t'))` | first callback `(document, dp)` (C2: `I_dp` is `dp`); second `(dp, document)` | first `(document, I_dp)`; second `(I_dp, I)`, in that order |

### §6.2 Unit (`crates/core/elidex-ecs/src/dom/tests/template_owner.rs`, NEW)

| cell | kind | shape | expected |
|---|---|---|---|
| U1 | red | `f(D)` twice → same entity, marked; D linked | equal, marked |
| U2 | red | `f(f(D)) == f(D)`; count of marked entities is 1 | 1 |
| U3 | red | D without any template: no marked entity after parse and `createElement('div')` | 0 (lazy) |
| U4 | red | 100 fragment parses with a template (both tiers), context in D: marked-entity count stays 1 (D's I); `inert_template_document_of(throwaway)` is `None` before each throwaway's destruction | 1; `None` |
| U5 | red | strict parse error with a template in the fragment: marked-entity count unchanged (D's I, created lazily or already there); no entity of the throwaway survives | as stated |
| U6 | red | `I` with an element child: `root_entities()` excludes I | excluded |
| U7 | red | `destroy_entity(D)` destroys `f(D)`; a contents node outliving it reads `node_document` → `None` | destroyed, `None` |
| U8 | red | clone policy: `clone_subtree(D)` copy has no link; `clone_subtree(I)` copy has no marker | absent |
| U9 | red | C0's adopt of a template into X, with nested contents and a shadow host in contents: shadow-including descendants → X, contents and nested contents and their shadow trees → `f(X)` | as stated |
| U10 | guard | the adopt walk past the depth and breadth caps (as `despawn_subtree_tears_down_past_the_traversal_depth_cap`, `dom/tests/destroy.rs`) | every node re-homed |
| U11 | red | the new-node entry: for a node in D's template contents → `f(D)`; for D → D; in a fragment build, for the throwaway → the context's node document | as stated |
| U12 | red (×2) | document parse of `<template><svg></svg><b>t</b><!--c--></template>` into D with a recording `MutationDispatcher` installed: after the parse every node in contents carries `AssociatedDocument == f(D)`, and the recorder holds no `Adopt` event (an `Adopt` would mean a node was created with another document and re-homed at its link) | as stated |

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
| `f` of the throwaway computed (substitution dropped) | U4 |
| link or marker copied by clone | T11, U8 |
| `root_entities` not excluding I | U6 |
| the entry ignores the intended parent (returns the parse / context document inside contents) | T19 (strict leg), U11, U12 |
| tolerant threads the template's document into contents without the entry | T3, T12 (tolerant legs) |
| `adoptedCallback` for a contents element gets the outer (old, new) | T25 |

### §6.4 Kept green

Slice a's U8 (ii) (the bind pass does not walk template contents), E1–E8, C18 (T18), and the #380 template suite
(`tests_html_t2d_protos.rs` `template_*`, `dom/tests/destroy.rs`, `dom/tests/traversal.rs`), C0's cells (its §6), D14's
cells. Commands: `cargo test -p elidex-ecs --all-features`, `-p elidex-dom-api`, `-p elidex-html-parser`,
`-p elidex-html-parser-strict`, `-p elidex-custom-elements`, `-p elidex-script-session`, `-p elidex-js`, then `mise run ci`.

## §7. Out of scope and slots

| item | where | defect (stated, not the remedy) |
|---|---|---|
| insertion adoption; the node-document reader | C0, `#11-cross-document-adopt-on-insert` (prerequisite) | no insertion adopts at `593aa314` |
| L3 Range creation sites A1–A9; dom-api creation sites that decide an owner from `owner_document(this)` | `#11-ownerless-node-creation` (owner Layout C0/C1; C0 rev 2 receives L3's list) | creation sites leave a node ownerless or decide its owner outside the entry |
| DocumentFragment *host* of template contents (§4.12.3 establishing step 2) | no slot (scout Q-S7); the PM names one if accepted | the contents fragment has no host link, so host-including-ancestor checks cannot see the template |
| template `for` attribute, content patching (§4.12.3) | not tracked here | elidex implements neither the attribute nor *prepare content patching* |
| DOM *adopt* steps 3.2 / 3.3.1–3.3.2 (registries, attribute nodes) | C0 draft §7 | not modelled |
| render / layout / hit-test roots for DOMParser documents and detached subtrees with children | no slot (scout Q-S6) | `root_entities` returns them and H-C11's consumers keep any with a child |
| DOMParser documents are never destroyed | no slot found (scout Q-S1); the PM names one if accepted | every `parseFromString` leaves a Document (and, from C1, its I if it has a template) in the page `EcsDom` for the World's lifetime |
| template-inclusive walkers (`collect_template_inclusive_descendants`, `derive_element_components`) | umbrella §9 N-5 (slice b) | three walkers for one concept |
| multi-realm creation of I | B1 (§8 H-3) | I is created in the single realm |

Own deferrals: **0**. Every row is pre-existing or another PR's scope.

## §8. Edges, hand-offs and records

**Edges.**
- c1: N1 implementation → C1 (T17, T22, T23's read-null legs need N1's compile gate).
- c3: **C0 → C1** (semantic precondition, PM decisions 143 / 153; the C0 draft's x7 row).
- c-D14: **L3 RC-2 D14 → C1** (code predecessor on `clone.rs`'s child-clone loop, PM decision 215; C1 rebases on it).
- c-E: C0 or C1 introduces the new-node entry (O-2); L3 RC-4 (A11 / A12) reads it. If RC-4 lands before C1, its cells
  for a Range inside template contents assert C2's document and C1 flips them; if after, RC-4 asserts I.
- c4: **C1 → T3** (T3 draft: T3 relies on D-C2 to D-C6; R-A closed by C1; T3 rewrites C18).
- c5: C1 → slice b (umbrella x5; §4.6 is the provision).

**Hand-offs.**
- H-1 to slice b: §4.6 items 1–4; the umbrella's §9 N-2 / N-3 (branch B's per-node fact) do not apply in branch A.
- H-2 to C0 (via the PM): P-C14 (custom elements can be in contents; the `Adopt` consumer's contents arguments are C1's,
  D-C5, and C0's consumer should leave a seam for them, Q-3) and P-C15 (on C0's base its link-time adopt overwrites the
  entry's fragment-build stamp of linked nodes; unobservable, but the entry's doc must say so).
- H-3 to B1: create I in D's relevant realm.
- H-4 to T3 (rev 3): T3's downstream list for this memo's owner (T3 draft §8.2 "C1 (lands before T3)") is applied by
  rev 2 (P-C11): base "N1 and C0", edge c4 (C1 → T3) replaces rev 1's c2, T18 is a guard, R-C4 unchanged at C1. T3's
  R-1 note ("done by this PR (umbrella rev 8.5)") conflicts with PM decisions 144 / 159 (O-1).
- H-5 to L3 (RC-2 D14, RC-4): §4.4's per-pair document and §4.5's A11 / A12 contract.

**Records (for the PM; exact texts).** Applied by C1's PR in place, with history (PM decisions 144 / 159); the vehicle
and the revision number depend on O-1, so this rev does **not** edit the umbrella file.
- **R-1 umbrella §1.2** — add the row (after C0's row, which the C0 draft's D-9 adds):
  "| **C1** `EH-attrs-at-creation-C1` (owner: Layout lane, this program; PM decision 123) | slot
  `#11-template-contents-owner-document`: the inert template contents owner document (HTML §4.12.3; branch A, §1.6);
  closes T3's residual R-A | IBP-sandbox N1's implementation and C0 landed; L3 RC-2's D14 landed |".
  §1.6's heading and lead: "C1's handling (decided by the PM, not here)" → "C1's handling (decided: branch A, PM decision
  123)", keeping the old wording in the changelog entry. §8.1's `#11-template-contents-owner-document` bullet: "Owner
  TBD …" → "Owner: Layout lane (PM decision 123); C1, after C0 (PM decisions 143, 153); plan
  `docs/plans/2026-10-eh-attrs-c1-template-owner-document.md`".
- **R-2 ledger `#11-template-contents-owner-document`**: "C1 plan-memo rev 2 (2026-11-01). Order (PM decision 153): N1
  implementation → C0 → C1 → T3 → slice b (supersedes this sub-bullet's 'N1 implementation → T3 → C1 → slice b'). C1's
  base also contains L3 RC-2's D14 (PM decision 215). Closes at C1's landing; closes T3's R-A with it."
- **R-3 umbrella in-place corrections** (at C1's PR): cell C14's expected column "(spec: does not run; slice b inverts it
  under S-2)" → "(spec: does not run; C1 inverts it, with N1's compile gate and C0's adoption at the placement)"; K-5's
  "From N1, the DOMParser half reads null." → "From N1, the DOMParser half reads null; from C1, the template-contents half
  reads null."; §8.2 defect (5)'s "its template-contents half, and the first clause for both, remain until slice b" →
  "its template-contents half until C1 lands, and the first clause for both until slice b"; §1.4's interim line gains a
  fourth window "(iv) W-K5's template-contents half reads null from C1, the spec's outcome (K-5)"; §1.5's W-K5 bullet
  gains "(from C1, its template-contents half too)"; C18's *Spec* note "reached after T3 and slice b" is unchanged.
- **R-4 ledger `#11-ownerless-node-creation`**: "C1 rev 2 (2026-11-01): D-C4 states the entry's contract
  (intendedParent's node document via `node_document`; the §13.4 throwaway substitution) and moves both parser tiers'
  creation sites onto it; A10 (template clone) and the A11 / A12 contract are C1's; A1–A9 and the dom-api sites stay with
  C0 rev 2 / L3. Which PR adds the `EcsDom` entry: C1 memo O-2."
- **R-5 slice a's docstring** `scripting.rs:59-60` is removed by C1 (§5.2); no ledger text.

## §9. Definition of done

1. Coordinates re-measured at C1's base (N1 implementation, C0, D14 landed); §5.1 populations re-run and any change
   recorded in the PR.
2. `appropriate_template_contents_owner_document`, `inert_template_document_of`, `is_inert_template_document` and both
   components exist in `elidex-ecs` with §4.9's citations; no other site computes a template contents' node document
   (P-1 re-run); C1's diff adds no `owner_document(` call (D-C10) and P-4's after-C1 shape holds.
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
10. The PM has applied R-1 to R-4 (or recorded why not), and decided O-1 to O-3.

## §10. Revision history / dispositions

### §10.1 Rev 2 (2026-11-01): driver brief inputs 1–9

| input | content (as relayed) | disposition | where |
|---|---|---|---|
| Step 0 | fetch, rebase on `origin/main`, re-verify every `file:line` | done: rebased without conflict onto `593aa314` (#576, #577, #579 included); every coordinate re-measured; symbol and grep keys preferred | §0.1, §0.2, §5.1 |
| 1 | order N1 impl → C0 → C1 → T3 → slice b; fix stale order refs; claim nothing landed that is not on main | applied: header, Prerequisites, §1, §6 column header, edges c1/c3/c4, T18, A8×A9, R-C4; N1 (#580 open) and C0 (draft) stated as not landed | header, §0.3 P-C11, §1, §6, §8 |
| 2 | map every owner-document read/write onto C0's `node_document` / `Adopt`; no second reader; state the dependency (PM 143) | applied: D-C10 (no `owner_document(` in C1's new code, DoD item 2); §4.7 table; D-C5 edits C0's walk and `Adopt` consumer; the dependency table in §1 with the C1-without-C0 column marked rejected. Found P-C14 (C0's consumer premise) and P-C15 (C0 × #577) | §0.4, §1, §4.7, §8 H-2 |
| 3 | R-A closed by C1, in the DoD with a concrete cell | applied: §9 item 9 quotes T3 rev 3's R-A; new cell T23 (four routes) plus per-route guards | §6.1 T23, §9 |
| 4 | `node_document_for_new_nodes` is the single new-node owner entry; no parallel path; slot `#11-ownerless-node-creation` | applied as D-C4 (contract = intendedParent's node document with #577's substitution; every parser creation in both tiers uses it). Premise error P-C13: it is strict-only and `pub(super)` today, so a single entry needs an `EcsDom` home; who adds it is O-2 | §0.3, §0.4 D-C4, §4.3, §8 R-4, §11 O-2 |
| 5 | A10 in scope; A11 / A12 read the entry and must get the right document; A1–A9 out of scope with owner | applied: A10 = §4.4; A11 / A12 = §4.5 + U11 (no Range-specific branch needed); A1–A9 and the dom-api sites in §7 with owner C0 rev 2 / L3 | §4.5, §6.2, §7 |
| 6 | clone.rs loop rewritten by D14 first; C1 rebases; write on D14's form; record the edge | applied: §4.4 on D14's form, edge c-D14. Premise error P-C12: the brief's "passes the operation's subtree flag" is today's code and the defect; D14's form is subtree **true** (DOM step 6.7). Found a C1 defect in the same loop (pass 4 stamps the operation's document inside copied contents): D-C6 per-pair document, T24 | §0.3, §0.4 D-C6, §4.4, §6 T24, §8 |
| 7 | Chromium measurement step before implementation, table to fill, no invented results | applied: §1.1 (13 rows, Chromium / other-engine columns empty), D-C11, DoD item 4; T1 / T20 gated on it | §1.1, §0.4, §9 |
| 8 | fold scout answers Q-S1..Q-S9 | applied: Q-S1 → H-C12, D-C7, §7 (DOMParser documents never destroyed); release-leak premise → P-C6; Q-S2 → A2×A4, T12; Q-S3 → H-C4, P-C9; Q-S4 → §5.1 P-6 (residual Q-S11); Q-S5 → H-C16, R-C5 (residual Q-S10); Q-S6 → D-C9, §7, H-C11 (+ `hit_test.rs`, P-C8); Q-S7 → H-C17, §7; Q-S8 → §5.2 (lines re-located at `593aa314`); Q-S9 → H-C16, T11 | §0.2, §0.3, §5, §7 |
| 9 | umbrella edits in C1's PR, in place, with history; C1 row only | texts written (§8 R-1, R-3); **not applied in this commit**: the T3 draft (`14ff3dc2`) already carries umbrella rev 8.4 / 8.5 with both C0 and C1 rows, and the C0 draft claims rev 8.4 for its row (P-C16); O-1 asks the PM which branch carries what before any of the three edits the file | §0.3 P-C16, §8, §11 O-1 |

### §10.2 Rev 1 (2026-10-31)

First draft, pre-plan-review. Its O-1 (C0 first) is decided by PM decisions 143 / 153; its O-2 (umbrella vehicle) by
PM decisions 144 / 159 (open sequencing: O-1 below); its O-3 (URL resolution) becomes §1.1; its O-4 (split) is O-3
below; its Q-1 (shared node-document helper) is resolved by C0's `node_document` and D-C4's entry; its Q-2 is Q-2 below;
its Q-3 (T18) is resolved (T18 is a guard at C1).

## §11. Open questions

**For the PM / user.**
- **O-1 Umbrella edit sequencing (P-C16).** Three unmerged branches edit umbrella §1.2: the T3 draft (rev 8.4 / 8.5,
  both C0 and C1 rows, edges x7–x10), the C0 draft (D-9: the C0 row, x7, the order line, as rev 8.4) and this memo (R-1,
  R-3: the C1 row and the C14 / K-5 / §8.2 texts). (a) Each PR carries its own row in landing order — C0's plan PR, then
  C1's, then T3's — and the T3 draft drops its C0 / C1 rows and renumbers (recommended: matches PM decisions 144 / 159
  and C0 draft D-9; each row lands with the PR that makes it true); (b) T3's PR carries all three as drafted (contradicts
  144 / 159); (c) the PM applies all rows in one PM-owned docs PR. Until decided, this memo states the texts and edits
  nothing.
- **O-2 Which PR adds the `EcsDom` new-node entry (P-C13).** (a) C0 rev 2 adds it (it lands first and receives L3's
  Range site list, ledger `#11-ownerless-node-creation`), and C1 moves the parser tiers onto it and supplies the
  template-contents facts (recommended: the first PR that needs a single entry creates it; no interim second path);
  (b) C1 adds it and moves the parser tiers; C0's sites keep `owner_document(this)` until C0 rev 2's own migration —
  impossible in landing order (C0 is first), so (b) means C0 rev 2 does not migrate any site; (c) C1 adds it and also
  migrates C0's and L3's A1–A9 (widens C1 into C0's census). If (a), D-C4 and §4.3 shrink to "call the entry".
- **O-3 Split.** If `mod.rs` is ≥ 990 lines at the base, the template-contents helpers move to `template_owner.rs` as a
  standalone pure-move PR first (the #537 precedent) or as a standalone first commit of C1's PR.

**For plan-review.**
- **Q-1** The entry's signature: how the fragment build's (throwaway, context) pair reaches it from two tiers (a small
  `NewNodeContext` value owned by the build, or two parameters), and whether the tolerant converter keeps a threaded
  cached value.
- **Q-2** The adopt walk's shape (D-C5): a `TemplateContentsWalk` variant that reports the contents edge, or a sibling
  walker; one walk either way.
- **Q-3** `adoptedCallback` arguments for custom elements in contents (D-C5): C0's one-event consumer switching (old,
  new) to (`f(old)`, `f(new)`) at each contents edge in tree order (recommended: the spec's order), or one nested `Adopt`
  per contents fragment (spec structure, but contents callbacks then come after every outer callback).

**Scout questions** (facts this memo needs; not read here).
- **Q-S10** For a non-bound Document wrapper (I, or a DOMParser document): `defaultView`, `URL`, `documentURI`,
  `baseURI`, `body`, `cookie` — which return I's own value and which fall back to `document_root()` (H-C16's sampled
  fallbacks)? Feeds §1.1 M3 and T1's third element.
- **Q-S11** `crates/dom/elidex-dom-api/src/focus/delegate.rs:85`: does its node-document read change outcome for an
  element in template contents (focus delegation into an inert document), and is the new outcome the spec's?
- **Q-S12** At C1's base: does the tolerant tier attach a declarative shadow root inside template contents (T12's
  tolerant leg; scout Q-S2 answered for strict), and does the shallow clone stamp its root (§4.4's representation
  choice)?
