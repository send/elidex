# C1 `EH-attrs-at-creation-C1` — template contents are owned by the inert template contents owner document (per-PR plan-memo)

Nested PR **C1** of the standalone program `EH-attrs-at-creation` (umbrella
`docs/plans/2026-10-event-handler-attrs-at-creation.md`, approved by #551 `b9122157` at rev 8, carried to rev 8.3 in
place by #570; "the umbrella"). It closes slot `#11-template-contents-owner-document` (carved from #380, `c801ef6c`) and
is slice b's precondition x5 in **branch A** (umbrella §1.3 x5, §1.6). Siblings cited by ID: slice a
(`docs/plans/2026-10-eh-attrs-slice-a.md`, landed by #563 `3cd84a04`; "the slice-a plan"), IBP-sandbox N1
(`docs/plans/2026-10-ibp-sandbox-handler-gates.md`, rev 4.5, merged as a plan by #570, implementation in flight; "the N1
memo"), T3 (`docs/plans/2026-10-eh-attrs-t3-target-determination.md`, rev 1 draft on branch
`layout-eh-attrs-t3-plan`, `e288fe0b`; "the T3 draft"; not yet on main, so its IDs are cited as draft IDs).

**Placement (evidence).**
- PM decision 123 (2026-10-31, ledger `project_open-defer-slots.md`, slot `#11-template-contents-owner-document`, its
  sub-bullet "Trigger completed 2026-10-31"): "branch A — C1 lands, before slice b; owner: Layout lane (EH-attrs program
  member, umbrella §1.2/§1.6)". The same sub-bullet: "a per-PR plan under the EH-attrs umbrella with plan-review is
  mandatory".
- The umbrella names C1 in x5, §1.6, §8.1 and S-2, but its §1.2 nested-PR table has **no C1 row** (rows: slice a, T3,
  slice b; `sed -n 419,429p` of the umbrella). §8 records item R-1 asks the PM to add it.

**Prerequisites.** N1's implementation and T3 landed (PM decision 123's order: N1 implementation → T3 → C1 → slice b).
**Plus one this memo adds and the PM must ratify (O-1): the insertion-adoption PR `C0`** (slot
`#11-cross-document-adopt-on-insert`). §1 shows that C1 without C0 makes the most common template pattern worse than
HEAD (cell T9: a node cloned from template contents and appended into the document reports the inert document as its
`ownerDocument` and resolves its URLs against `about:blank`). §4.6 states what C1 requires of C0.

**Scope.** One rule computes a template's contents' node document — WHATWG HTML §4.12.3's *appropriate template contents
owner document* — and every site that establishes, clones, parses into or adopts template contents calls it:
1. the inert document itself, as an ECS entity and two components (D-C1, D-C2);
2. the establishing steps at every template creation (both parser tiers, `createElement`, the clone post-pass) (D-C3);
3. parser-created nodes inside template contents take the contents' node document wherever they stamp an owner (D-C4);
4. the template *adopting steps* inside the one adopt function (D-C5);
5. the template *cloning steps* (D-C6);
6. the inert document's lifetime: created lazily, destroyed with its document, never copied by a clone (D-C7, D-C8);
7. the inert document is not a render/layout root (D-C9);
8. the C2 docstrings and the tests that pin C2 are rewritten (§5.2).

**Not in scope.** Insertion adoption itself (C0, §4.6); creation-time handler processing (slice b, S-1/S-2); the
template element's `for` attribute and content patching (a newer §4.12.3 surface; §7); the DocumentFragment *host*
of template contents (§7, scout Q-S7); multi-realm (B1; "given document's relevant realm", §4.1).

**Changelog.** *Rev 1* (2026-10-31): first draft, pre-plan-review.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` is a **`74e15df1`** coordinate (the branch cut, `git rev-parse HEAD` in this worktree). `origin/main`
has since moved to `1ed126b4` (#568); `git diff --stat 74e15df1 1ed126b4 -- crates` touches
`crates/core/elidex-ecs/src/dom/tree/teardown.rs` (doc and body of `begin_detached_fragment`, so `adopt_subtree`'s
neighbours below it shift), `crates/core/elidex-ecs/src/dom/tests/destroy.rs` and the strict `tree_builder/mod.rs`. C1's implementation is cut after N1, T3 and C0 land, so **every coordinate is re-measured at that
base** (§9 item 1). Code references come from `/usr/bin/grep -n` / `sed -n` over the `74e15df1` tree, run 2026-10-31.
Every § number ↔ title pair and every algorithm step comes from `.claude/tools/webref` (`heading`, `dfn`, `body`), run
2026-10-31; the commands are in §3.

### §0.2 Premises re-measured

| id | claim | evidence (`74e15df1`) |
|---|---|---|
| H-C1 | One establishing helper exists; its owner parameter is the C2 approximation | `EcsDom::attach_template_contents` `crates/core/elidex-ecs/src/dom/mod.rs:561`; doc `:551-560` ("ships the **C2 approximation** `owner_doc = template's own node document`"); creates the fragment with `create_document_fragment_with_owner(owner_doc)` `:575` and links `TemplateContents { fragment }` (`crates/core/elidex-ecs/src/components.rs:366`) |
| H-C2 | Four production callers of the helper | `/usr/bin/grep -rnF 'attach_template_contents(' crates --include='*.rs' \| /usr/bin/grep -v '/tests/\|tests\.rs\|/tests_'` → clone post-pass `crates/dom/elidex-dom-api/src/node_methods/clone.rs:422`, `createElement` `crates/dom/elidex-dom-api/src/document.rs:264`, tolerant `crates/dom/elidex-html-parser/src/convert.rs:288`, strict `crates/dom/elidex-html-parser-strict/src/tree_builder/shadow.rs:137`, plus the definition |
| H-C3 | Template-content descendants resolve their owner through the fragment | `EcsDom::owner_document` `mod.rs:821`: explicit `AssociatedDocument` first (`:828-841`), else the tree root; a `DocumentFragment` root yields the fragment's `AssociatedDocument` (`:852-856`). Parser elements carry no explicit owner (strict `insert_element_named` `crates/dom/elidex-html-parser-strict/src/tree_builder/insert.rs:55` uses `create_element`; tolerant `convert_node` `convert.rs:258` passes `None`) |
| H-C4 | Two parser sites stamp an explicit owner that is not the contents' node document | strict foreign elements: `insert_foreign_element` takes `owner_document(context)` in a fragment parse, else `self.document` (`insert.rs:108-117`); tolerant threads the template's `owner_document` into the contents' children (`convert.rs:289-290`) and into declarative shadow content (`convert.rs:206`). Which tolerant creations stamp it: scout Q-S3 |
| H-C5 | Adoption re-homes template contents to the **same** document as the template | `EcsDom::adopt_subtree` `crates/core/elidex-ecs/src/dom/tree/teardown.rs:251` walks `for_each_uncapped_shadow_inclusive(root, TemplateContentsWalk::Include, …)` (`:253`) and sets every visited node's `AssociatedDocument` to `document` (`:256-258`). Its one caller is `finish_detached_fragment` (`:328`) |
| H-C6 | No insertion adopts | `#11-cross-document-adopt-on-insert` (ledger, re-eval 2026-10-31: "`EcsDom::adopt_subtree` (`teardown.rs:251`) has one caller, the fragment parse at `:328`, and none on the insertion path"). Re-run: `/usr/bin/grep -rnF 'adopt_subtree(' crates --include='*.rs'` → `teardown.rs:251` (def), `:328` |
| H-C7 | A clone stamps an explicit owner on every cloned node | `clone_subtree`'s document choice `crates/core/elidex-ecs/src/dom/tree_clone.rs:206-210` (`doc_override.or_else(\|\| self.owner_document(src))`), root stamp `:221`, descendant stamp `:417`. The clone post-pass threads `shadow_doc` (`clone.rs:239`) into `replicate_template_contents` (`clone.rs:262`, `:405`), which creates the copy's fragment with it (`:422`) and clones the source contents' children with it (`:426-427`) |
| H-C8 | The ownerDocument getter and URL resolution read `owner_document` | getter `crates/dom/elidex-dom-api/src/node_methods/core.rs:279` (`owner_document(this).or_else(\|\| dom.document_root())`); href base `crates/dom/elidex-dom-api/src/element/href_accessor.rs:50`. Consumer census: §5.1 P-5 |
| H-C9 | The handler gates derive the node document through one rule | `scripting_disabled_for_platform_object` `crates/script/elidex-script-session/src/scripting.rs:83`: a connected node's composed root Document, else `owner_document` (`:104-110`). Its doc names the template false negative as this slot's (`:59-60`). T3 draft D-T4 extracts the rule as `effective_node_document` (NEW in T3) and uses it for step 3 |
| H-C10 | A Document entity is spawned in one place | `EcsDom::create_document_node` `mod.rs:414` (`TreeRelation`, `NodeKind::Document`, `DocumentBaseUrl(about_blank_url())`). Non-test callers: `teardown.rs:294` (fragment throwaway), `mod.rs:399` (`create_document_root`), `crates/dom/elidex-form/src/inert_document.rs:119` (DOMParser) |
| H-C11 | `root_entities` excludes only `DocumentFragment` | `crates/core/elidex-ecs/src/dom/tree/navigation.rs:362-374`; consumers keep entries that have a `LayoutBox` / `ComputedStyle` or a first child: `crates/core/elidex-render/src/builder/mod.rs:479`, `:995`, `crates/layout/elidex-layout/src/layout/mod.rs:371` |
| H-C12 | Teardown reaches contents; a Document's destruction does not cascade | `despawn_subtree` `teardown.rs:190` walks with `Include` (`:197`); `destroy_entity` `teardown.rs:51` cleans shadow links only. The fragment throwaway Document is destroyed by `finish_detached_fragment` (`:331`); at `1ed126b4` (#568) the strict error path is `despawn_subtree(document)` |
| H-C13 | elidex has no XML document | no `is_html_document` / XML-document component (`/usr/bin/grep -rn 'is_html_document\|XmlDocument' crates --include='*.rs'` → only the forward-looking note `mod.rs:734`); DOMParser serves `text/html` only (umbrella §3 row §8.5.1) |
| H-C14 | The clone-policy table enumerates every component | `tree_clone.rs:15-34` (module doc), "deliberate non-copy" row `:30`, `TemplateContents` row `:27` |
| H-C15 | The bind pass excludes template contents | slice a's bind walk `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:178` (`TemplateContentsWalk::Exclude`); umbrella D-11, K-5, K-6 |

### §0.3 Premise errors (reported)

- **P-C1 (brief).** "The #380 plan it cites (find it under docs/plans/ by grepping for the slot name)": no #380 plan is
  in the repository. `/usr/bin/grep -rln 'template-contents-owner-document' docs/` lists six memos, all later
  citers (`2026-10-eh-attrs-slice-a.md`, `2026-09-ibp-sandbox.md`, `2026-07-s5-4-sandbox-enforcement.md`, the umbrella,
  the N1 memo, `2026-09-inline-box-predicate.md`); `git show c801ef6c --stat` adds no `docs/plans/` file. The slot's
  "plan §5 Decision C / plan-review §12 ratified C2" is reachable only through the ledger text and PR #380's body
  (`gh pr view 380 --json body`). This memo relies on those two.
- **P-C2 (order).** "N1 impl → T3 → C1 → slice b" omits a prerequisite: insertion adoption (§1, O-1).
- **P-C3 (umbrella §1.2).** The umbrella describes itself as having C1 in its nested PR table; it does not (header).
- **P-C4 (umbrella, cell C14 and §8.2 defect (5)).** The umbrella says slice b inverts C14 under S-2, and that defect
  (5)'s template-contents half "remain[s] until slice b". Under N1's compile gate, the read of a raw value in template
  contents turns null as soon as the contents' node document is not the bound document, which is C1's landing (with
  C0 for the `innerHTML` shape C14 uses). So C1 inverts C14's getter and dispatch, and slice b only moves the
  processing point. §8 records R-3 states the exact replacement texts.
- **P-C5 (ledger).** The slot text's "the adopt-special-casing is that balloon" is right but incomplete: the larger
  balloon is that elidex has no insertion adoption at all (H-C6), and the C2 approximation is what hides that for
  template clones (H-C7: the clone stamps the main document, which is also what an adoption at the append would write).

### §0.4 Decisions (this PR)

- **D-C1 — the inert document is an entity.** The *associated inert template document* of a Document D is a Document
  entity I, spawned by `create_document_node` (H-C10), so it has the Document shape (and `DocumentBaseUrl` =
  `about:blank`, which is the spec's fallback base URL for it, §3 row §2.4.3). Two components (NEW), both in
  `crates/core/elidex-ecs/src/components.rs`:
  - `AssociatedInertTemplateDocument(Entity)` (NEW) on D: the forward link D → I (*associated inert template document*).
  - `InertTemplateDocument` (NEW), a unit marker on I: "a Document created by this algorithm".
  No back-link I → D: the spec defines no query from I to D, and the one lifetime need (D-C7) runs from D.
  *Side-store rule* (CLAUDE.md): the fact is per-entity, `Send + Sync`, and not a per-VM identity handle (it holds an
  `Entity`, not an `ObjectId`), so it is a component; a `HashMap<Document, Document>` on `EcsDom` or in `HostData` is
  rejected. *Idiom table:* OO engines' owning pointer from a document to its template document plus a back pointer
  → forward-link component on D + marker on I; owner-held lifetime → despawn cascade (D-C7); "is template document"
  boolean → marker query.
- **D-C2 — one rule.** `EcsDom::appropriate_template_contents_owner_document(&mut self, document: Entity) -> Entity`
  (NEW) is §4.12.3's algorithm: if `document` carries `InertTemplateDocument`, return it; else return its linked I,
  creating and linking it on first use (lazy, as the spec's "if document does not yet have"). Step 1.1.2 ("if document
  is an HTML document, mark newDocument as an HTML document") is vacuous (H-C13) and is stated so in the doc comment,
  not modelled. The rule is idempotent over the pair: `f(D) = f(I) = I`. That property is what lets every caller pass
  either D or I (a template's node document is one of them) and get the same answer. A predicate
  `EcsDom::is_inert_template_document(&self, document: Entity) -> bool` (NEW) reads the marker; it is the per-node
  "template origin" fact slice b's S-2 asks for, read through the node document (§4.5).
- **D-C3 — the establishing steps call the rule once, inside the helper.** `attach_template_contents(template,
  node_document)` keeps its name and its single-attach contract; its second parameter becomes **the template's node
  document** (what every caller already knows at creation, H-C2), and the helper applies D-C2 to it. `None` keeps
  today's orphan fragment (an orphan clone, H-C7). No caller computes the inert document itself. The four callers pass:
  `createElement` the receiver document; the strict tier the parse document (`self.document`); the tolerant tier the
  threaded document; the clone post-pass the operation's document. Because of D-C2's idempotence, a template nested in
  template contents gets I whether its caller passes D or I.
- **D-C4 — parser-created nodes in template contents carry the contents' node document.** WHATWG HTML §13.2.6.1
  *create an element for a token* step 3: "Let document be intendedParent's node document", and the appropriate place
  inside a template is its template contents. Every parser site that **stamps** an explicit owner stamps the intended
  parent's node document: (a) strict `insert_foreign_element` (H-C4) and (b) the tolerant tier's contents recursion
  and declarative-shadow recursion (H-C4; exact stamp sites Q-S3). Sites that stamp nothing are already right through
  H-C3's fragment fallback once the fragment's owner is I (D-C3). The "intended parent's node document" for a
  `Document` parent is the Document itself, for any other parent `owner_document(parent)` — one private helper per
  parser crate is input to the implementation; a shared node-document helper on `EcsDom` is Q-1 (it would be a third
  node-document function beside `owner_document` and T3's `effective_node_document`; §2 A1×A8).
- **D-C5 — the template adopting steps live in the one adopt function.** `adopt_subtree(root, document)` re-homes
  `root`'s shadow-including inclusive descendants to `document` (DOM §4.5 *adopt* step 3.1) and, for each template
  among them, adopts its contents into `appropriate_template_contents_owner_document(document)` (HTML §4.12.3
  *adopting steps*; DOM *adopt* step 3.4 runs them). Applied recursively, a template inside contents re-homes its own
  contents to the same I (idempotence). The walk keeps H10a's shape (uncapped, iterative, visited set) and carries
  the target document per stacked node. `TemplateContentsWalk::Include` keeps its teardown caller (`despawn_subtree`);
  whether the adopt walk is a third `TemplateContentsWalk` variant or a sibling walker is input (§4.3). C0 calls this
  same function at insertion (§4.6).
- **D-C6 — the template cloning steps.** The copy's contents are established at the copy's creation through D-C3 with
  the operation's document, and the source contents' children are cloned with **document = the copy's contents' node
  document** (HTML §4.12.3 *cloning steps* step 2), not the operation's document (H-C7: today both are `clone_doc`).
- **D-C7 — lifetime.** I exists iff D has needed it (lazy, D-C2). Destroying D destroys its linked I
  (`destroy_entity` on a Document follows `AssociatedInertTemplateDocument`). I has no tree children unless a script
  appends some; in that case `destroy_entity(I)` behaves as for any Document (its subtree's fate is the caller's, as
  for the DOMParser and throwaway documents today). A node whose node document is I and that outlives D resolves
  `owner_document` to `None` through the existing dangling guard (`mod.rs:832-841`, `:852-856`), the same outcome as a
  node left on a destroyed throwaway document today. Consequence: a fragment parse that builds a template creates an I
  for its throwaway document (spec-exact: the template is created in that document), the finishing adopt re-homes the
  contents to the context's I (D-C5), and the throwaway's destruction destroys its I. No per-parse leak; cells U4, U5.
- **D-C8 — clone policy.** Both components are rows of the clone-policy table (H-C14), deliberate non-copy: a cloned
  Document is "not created by this algorithm" and gets its own I on first need; a clone of I is an ordinary Document.
- **D-C9 — I is not a render or layout root.** `root_entities` excludes a Document that carries
  `InertTemplateDocument`, at the same source and for the same reason as #380's `DocumentFragment` exclusion (template
  inertness by construction): a script can give I a child (`tpl.content.ownerDocument.append(…)`), and H-C11's
  consumers would then lay out and paint it. DOMParser documents and detached subtrees reach the same consumers at
  HEAD; that wider defect is not C1's (§7, scout Q-S6).
- **D-C10 — C0 first (proposed; O-1).** C1 lands after insertion adoption. §1 gives the evidence.

## §1. Shape and verdict

**Axes (§2): 9** — A1 the rule (D-C2), A2 creation paths (D-C3, D-C4), A3 nested templates, A4 shadow roots in template
contents, A5 clone (D-C6), A6 adopt (D-C5, C0), A7 lifetime (D-C7, D-C8), A8 node-document consumers (H-C8, H-C9,
D-C9), A9 the EH program's contract (K-*, X-W, S-1/S-2). ≥ 3 intersecting axes, so this is edge-dense: the PM's
classification holds and plan-review is mandatory (CLAUDE.md *Edge-dense work* (b)). As a per-PR slice of the approved
umbrella it is a terminal unit (base case (c)) once its plan-review passes.

**Why C0 must come first (the verdict that changes the order).** Today the C2 approximation and the missing insertion
adoption cancel for the common template patterns:

| pattern | HEAD (`74e15df1`, C2, no insertion adoption) | C1 without C0 | spec |
|---|---|---|---|
| `c = tpl.content.cloneNode(true); document.body.append(c)`; a moved node's `ownerDocument` | `document` (the clone stamped `document`, H-C7) | the inert document I (the clone stamps I; nothing re-homes it at the append) | `document` (DOM *insert* step 7.1 adopts) |
| same; `<a href="x">` from it, `a.href` | absolute against the page base | `"x"`: I's base is `about:blank` (D-C1), so the URL does not parse | absolute against the page base |
| `tpl.innerHTML = '<b>'`; `b.ownerDocument` | `document` | `document`: the fragment parse adopts into the template's document (H-C5), and the placement into contents does not adopt | I |
| `tpl.content.append(document.createElement('div'))`; its `ownerDocument` | `document` | `document` (explicit owner, no adoption) | I |
| `document.body.append(tpl.content)` with a parser-built `<svg>` child; its `ownerDocument` | `document` (strict stamps the parse document, H-C4) | I (D-C4 stamps I; no adoption at the move) | `document` |

Rows 1, 2 and 5 are **worse than HEAD** on the most common template idiom (row 1 is the example in §4.12.3 itself).
Rows 3 and 4 leave two answers to "what is the node document of a node in template contents", depending on how the
node got there, which is the decision surface One-issue-one-way forbids. No representation trick inside C1 removes
both (stamping no owner on clones makes a detached clone's `ownerDocument` null; preferring the tree root over the
stamp is N1's effective-node-document proxy, whose docstring records that "no single tree / owner-document rule
resolves every facet", `scripting.rs:68-71`). The spec's mechanism is adoption at insertion, so C1 requires it.

**Split decision.** Two units, each its own PR with its own plan-review:
- **C0** (`#11-cross-document-adopt-on-insert`, insertion adoption): every insertion adopts the node into the parent's
  node document before the `MutationEvent::Insert` for it, through the one adopt function. Under C2 its template step is
  the identity (contents go to the same document, as `adopt_subtree` does today), so C0 is HEAD-faithful for templates
  and fixes the DOMParser facets the slot lists. C0's plan owns its consumer re-listing (selection, base URL,
  `ownerDocument`, the effective-node-document proxy's collapse).
- **C1** (this memo): the rule, the inert document and every site in Scope, on a base where insertion adopts.
Folding C0 into C1 is the alternative (O-1 (b)); it adds C0's consumer axis to A6×A8 and the proxy collapse to A8×A9.
The memo is written so that only §4.6 and §5.3 grow if the PM chooses it.

**K-0 lens (umbrella K-0, S-3: no step worse than HEAD).** On C1's base (N1, T3, C0 landed), C1's observable changes are
listed in §5.4 (R-C1 to R-C7). Every one moves an outcome from C2's to the spec's; none moves it to a third value.
Cell T20 (URL resolution in template contents) is the one with a web-compatibility question (O-3).

## §2. Coupled invariants

### §2.1 Invariant per axis

| axis | invariant |
|---|---|
| A1 rule | For every Document D, `f(D)` is one Document I carrying `InertTemplateDocument`, the same entity on every call; `f(I) = I`. No site computes a template contents' node document except through `f` (D-C2, D-C3) |
| A2 creation | At every template creation the contents fragment's node document is `f(template's node document)`; every parser-created node inside contents resolves `owner_document` to the same I, in both tiers, in document, fragment and DOMParser parses (D-C3, D-C4) |
| A3 nesting | A template at any depth inside contents of D's templates has node document I and contents node document I; there is one I per non-inert Document, never an I of I |
| A4 shadow | A shadow root inside template contents (declarative or `attachShadow`) and its content report I; the shadow-including walks (adopt, teardown) reach them under the contents' target |
| A5 clone | The copy's contents are established in `f(copy's node document)`; their children are cloned into that document; a clone of contents is in I; a clone of D or of I is a non-inert Document with no link |
| A6 adopt | Adopting a template into document X re-homes the template and its light/shadow descendants to X and its contents (and nested contents) to `f(X)`; with C0 every insertion runs it |
| A7 lifetime | I is created on first need and destroyed exactly when D is; no fragment parse, strict parse error or clone leaves an I without its D |
| A8 consumers | Every reader of a node's document reads I for a node in template contents: `ownerDocument`, the base URL, N1's compile gate (null read), T3's step 3 (null target), and no render/layout root discovery visits I |
| A9 EH contract | K-0 and S-3 hold; the bind pass still walks no template contents (K-5, K-6, U8 (ii)); S-1's template-contents extension and S-2's branch-A read-null are satisfiable on C1's head (§4.5) |

### §2.2 All pairs (36), with intersection

| pair | intersect? | intersection |
|---|---|---|
| A1×A2 | yes | Four establishing callers pass a node document, the helper maps it (D-C3). A caller that passes the template's own document unmapped is C2 (T1, T3) |
| A1×A3 | yes | Idempotence: a nested template passes D or I and gets I (T4, T5). A rule without the marker check creates I-of-I (T5) |
| A1×A4 | yes | A shadow root in contents takes its host's node document; the rule is never applied to shadow roots (only templates have contents) (T12) |
| A1×A5 | yes | The copy's contents come from `f(operation document)`; for a contents clone the operation document is I and `f(I) = I` (T9, T10) |
| A1×A6 | yes | Adopt calls `f(target)` for contents, once per template reached (D-C5; T13, T14) |
| A1×A7 | yes | Lazy creation inside `f`; destruction cascades from D, so `f` never returns a destroyed I while D lives (U3, U4, U5, U7) |
| A1×A8 | yes | Consumers read the result through `owner_document`; no consumer calls `f`. One node-document answer per node (Q-1: whether a shared `node_document` helper joins `owner_document` and T3's `effective_node_document`) |
| A1×A9 | yes | `is_inert_template_document` is the branch-A template-origin fact (S-2), derived from the node document, so it tracks moves (a node moved out loses it through C0's adoption) |
| A2×A3 | yes | Strict: `self.document` passed at every depth, mapped to I (D-C3). Tolerant: the threaded document for an inner template is the outer contents' I after D-C4; either maps to I (T4, both tiers) |
| A2×A4 | yes | Declarative shadow content inside contents: the strict DSD path and the tolerant DSD recursion (`convert.rs:206`) must stamp the host's node document I, not the parse document (D-C4 (b); T12; Q-S2, Q-S3) |
| A2×A5 | no | Parser creation and cloning share only the helper (A1×A2, A1×A5) |
| A2×A6 | yes | Fragment parse: nodes are created in the throwaway document T, its templates' contents in `f(T)`; the finishing adopt moves them to the context's document and contents to `f(context doc)` (D-C5, D-C7; T14). Tolerant fragment parse threads the context's document directly (`crates/dom/elidex-html-parser/src/lib.rs:161`), so its contents land in `f(context doc)` without the throwaway I; both tiers end equal (T14 parity) |
| A2×A7 | yes | A strict parse error tears down the throwaway subtree (`despawn_subtree`, contents included) and its document, whose I is destroyed by the cascade (U5) |
| A2×A8 | yes | DOMParser: `dp`'s templates' contents are in `f(dp)`, a third document, read null (N1) and target null (T3) exactly as `dp`'s own nodes (T6) |
| A2×A9 | yes | The pre-bind parse puts contents in I before bind; slice a's bind pass still excludes them (U8 (ii) unchanged). Slice b's S-1 extension processes them at creation-equivalent time; C1 gives them a non-bound node document so their raw values read null (§4.5) |
| A3×A4 | yes | A template inside a shadow tree inside contents, and a shadow host inside nested contents: all report I (T12 variant) |
| A3×A5 | yes | Deep clone of an outer template clones the inner template inside the copy's contents; the inner copy's contents are established with `f(I) = I` (T10) |
| A3×A6 | yes | Adopting an outer template into X moves outer contents and every nested template and nested contents to `f(X)`; one walk, target switched once at the first contents edge (T13 nested variant) |
| A3×A7 | no | Nesting creates no further document (A1×A3) |
| A3×A8 | yes | Nested contents read null and target null like outer contents (same I) |
| A3×A9 | no | The EH contract does not distinguish depth inside contents |
| A4×A5 | yes | Clone of a clonable shadow host inside contents threads the operation's document (I) into the shadow copy (`clone.rs:282-284`); unchanged code, new value (T12 clone variant) |
| A4×A6 | yes | Adopt's walk is shadow-including under each target: a shadow root inside contents goes to `f(X)`, a shadow root of the template's light descendants to X (T13 shadow variant) |
| A4×A7 | no | Shadow roots create no document |
| A4×A8 | yes | `shadowRoot.ownerDocument` in contents reads I; a handler on a shadow-root-direct child in contents reads null under N1 (no Insert fires there, H2, so it is processed only by slice b's creation path) |
| A4×A9 | yes | Slice a's G2 window (shadow-root-direct elements not processed) is unchanged by C1; in contents, the spec read is null either way |
| A5×A6 | yes | `tpl.content.cloneNode(true)` is in I; appending it into the document adopts it (C0) to D. Without C0 it stays I (§1 row 1, cell T9) |
| A5×A7 | yes | Cloning D creates no I until a template in the copy needs one; the link is not copied (D-C8; T11, U8) |
| A5×A8 | yes | A detached contents clone reads null for raw handlers (N1: owner I ≠ bound) and the spec function after the append (adopted to D) |
| A5×A9 | yes | S-1 processes clones at creation; a clone of contents is in I, so slice b's raw values read null until inserted, which is S-2's branch-A outcome. The umbrella's §9 N-2 problem (no per-node fact) does not arise: the fact is the node document |
| A6×A7 | yes | Adopting all templates out of D's contents leaves I alive (D lives); adopting a template from a destroyed-later document (the throwaway) moves its contents off that I before the cascade (T14, U4) |
| A6×A8 | yes | After adoption every consumer reads the new document; with C0 the effective-node-document proxy and `owner_document` agree for connected nodes (C0's plan decides the collapse) |
| A6×A9 | yes | C0's adoption precedes the Insert event, so slice a's Insert arm (T-a2) and N1's gate see the adopted document; a node placed into contents is processed at T-a2 with node document I and reads null (W-K5's template half closes, §8 R-3) |
| A7×A8 | yes | A node outliving its I reads `ownerDocument` null through the dangling guard; I is never a render root (D-C9; U6) |
| A7×A9 | no | The EH contract never reads I's lifetime |
| A8×A9 | yes | N1's node clause and T3's step 3 read I for contents: C14 inverts (no run, `null`), C18 changes to T3's W2 shape (§6 T17, T18) |

Count: 36 pairs; 31 intersect, 5 do not (A2×A5, A3×A7, A3×A9, A4×A7, A7×A9). Re-count:
`awk '/^### §2.2/{f=1} /^## §3/{f=0} f && /^\| A[0-9]×A[0-9]/' <memo> | awk -F'|' '{print $3}' | sort | uniq -c`.

## §3. Spec coverage map

Lookups (2026-10-31): `.claude/tools/webref coverage-map html 4.12.3 dom 4.5 dom 4.4 dom 4.2.3 html 13.2.6.1 html 13.4
html 8.5.4 html 13.2.6.4.4 html 8.5.1 html 8.1.8.1 dom 4.8`, then `heading html 2.4.3`; `dfn html 'appropriate template
contents owner document'` → §4.12.3 `#appropriate-template-contents-owner-document`; `dfn html 'associated inert template
document'` → §4.12.3; `dfn dom 'adopt'` → §4.5 `#concept-node-adopt`; `dfn dom 'adopting steps'` → §4.5; `dfn dom 'clone
a node'` / `'cloning steps'` → §4.4; `dfn dom 'insert'` → §4.2.3 `#concept-node-insert`; `dfn html 'create an element for
a token'` and `'appropriate place for inserting a node'` → §13.2.6.1; `dfn html 'fallback base URL'` → §2.4.3; step text
from `body html the-template-element`, `body dom concept-node-adopt`, `body dom concept-node-clone`, `body dom
concept-node-insert`, `body html parsing-main-inhead`, `body html create-an-element-for-the-token`, `body html
fallback-base-url`.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.12.3 The template element | establishing the template contents, steps 1–3 | 1 document = node document's appropriate template contents owner document; 2 fragment with that node document and host = template; 3 link | `EcsDom::attach_template_contents` `crates/core/elidex-ecs/src/dom/mod.rs:561` (D-C3); step 2's *host* is §7 | ✗ (host not modelled) | yes |
| WHATWG HTML §4.12.3 The template element | *appropriate template contents owner document*, steps 1–2 | 1 not created by this algorithm → get-or-create the associated inert template document (1.1.1 create with browsing context null; 1.1.2 HTML flag; 1.1.3 link), set document to it; 2 return | `appropriate_template_contents_owner_document` (NEW) in `crates/core/elidex-ecs/src/dom/template_owner.rs` (NEW) (D-C2); 1.1.2 vacuous (H-C13) | ✓ | no |
| WHATWG HTML §4.12.3 The template element | template *adopting steps*, steps 1–2 | document = node's (new) node document's appropriate owner; adopt contents into it | `EcsDom::adopt_subtree` `crates/core/elidex-ecs/src/dom/tree/teardown.rs:251` (D-C5) | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | template *cloning steps* (node, copy, subtree), steps 1–2 | subtree false → return; else clone each contents child with document = copy's contents' node document, parent = copy's contents | `replicate_template_contents` `crates/dom/elidex-dom-api/src/node_methods/clone.rs:405` (D-C6) | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | `content` getter | return the template contents | `crates/script/elidex-js/src/vm/host/html_template_proto.rs:106` (unchanged) | ✓ | no |
| WHATWG DOM §4.5 Interface Document | *adopt* steps 1–3 | 3 document ≠ oldDocument → for each shadow-including inclusive descendant: 3.1 set node document; 3.2 / 3.3 registries and `adoptedCallback` (not C1's); 3.4 run the adopting steps | `adopt_subtree` (D-C5); 3.2 / 3.3 are C0's or later (§7) | ✗ | yes |
| WHATWG DOM §4.2.3 Mutation algorithms | *insert* step 7.1 | adopt node into parent's node document | C0 (§4.6); not touched by C1 | ✗ | yes |
| WHATWG DOM §4.4 Interface Node | *clone a node* steps 2, 3, 5, 6.7 | 2 copy created in document (runs the establishing steps); 3 cloning steps; 5 children with the same document; 6.7 shadow children with the same document | `clone_node_with_shadow_honor` `clone.rs:227`, `clone_subtree` `crates/core/elidex-ecs/src/dom/tree_clone.rs:206-210` (unchanged except D-C6) | ✓ | yes |
| WHATWG DOM §4.4 Interface Node | *node document*; `ownerDocument` getter | a node's node document; null for a Document | `EcsDom::owner_document` `mod.rs:821` (unchanged logic; new values) | ✓ | no |
| WHATWG HTML §13.2.6.1 Creating and inserting nodes | *appropriate place for inserting a node*; *create an element for a token* step 3 | a template target parent → its template contents; document = intendedParent's node document | strict `insert_foreign_element` `crates/dom/elidex-html-parser-strict/src/tree_builder/insert.rs:108-117`; tolerant `convert.rs:289-290`, `:206` (D-C4) | ✗ (only stamping sites change) | yes |
| WHATWG HTML §13.2.6.4.4 The "in head" insertion mode | start tag "template", steps 8–13 ("Keep": declarative shadow root) | the declarative shadow root attaches when the *parser's* allow-declarative-shadow-roots flag is true, so it attaches inside template contents too | strict `tree_builder/shadow.rs:137` (D-C3), DSD content owner (Q-S2); tolerant `convert.rs:206` | ✗ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | the throwaway Document and the returned nodes | templates parsed into the throwaway document get its inert document; the caller's insertion adopts | `begin_detached_fragment` `teardown.rs:286`, `finish_detached_fragment` `teardown.rs:321` (D-C5, D-C7) | ✗ | yes |
| WHATWG HTML §8.5.4 The innerHTML property | setter on a template: the target is its template contents | parsed nodes replace the contents' children (inserted, so adopted into I under C0) | `crates/script/elidex-script-session/src/mutation/html_fragment.rs:93` (unchanged; C0 adopts) | ✗ | yes |
| WHATWG HTML §8.5.1 The DOMParser interface | `parseFromString` `text/html` | the new document is "not created by this algorithm": its templates get its own inert document | `parse_into_inert_document` `crates/dom/elidex-form/src/inert_document.rs:110` (unchanged; values change via D-C3) | ✗ | yes |
| WHATWG HTML §2.4.3 Document base URLs | *fallback base URL* steps 1–2 | I's URL is `about:blank` and its about base URL is null → base `about:blank` | `DocumentBaseUrl(about_blank_url())` from `create_document_node` `mod.rs:414`; read by `href_accessor.rs:50` (O-3) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *get the current value of the event handler* step 3.2; *determine the target of an event handler* step 3 | raw value in a document whose browsing context is null → null; node document not an active document → null target | N1's compile gate and T3's step 3 read I through the node-document rule `crates/script/elidex-script-session/src/scripting.rs:104-110` (unchanged; doc `:59-60` rewritten) | ✗ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *shadow-including inclusive descendant* | adopt's walk order | `for_each_uncapped_shadow_inclusive` `crates/core/elidex-ecs/src/dom/tree/walkers.rs:99` (D-C5 shape is input, §4.3) | ✓ | no |

### §3.1 User-input touch audit

Rows marked *yes* carry author markup or script-constructed trees. C1 parses no new input and stores no string: the
two components hold an `Entity` and nothing. What changes is which Document a node of author content is attributed
to, and with it three reads: the `ownerDocument` getter (a new Document wrapper becomes reachable from script: I,
whose browsing context is null; Q-S5 asks which Document methods on a non-bound Document fall back to the page),
URL resolution (relative URLs in template contents no longer resolve against the page, §3 row §2.4.3, O-3), and the
handler gates (raw handlers in template contents read null and do not run: the spec's outcome, more restrictive than
HEAD). No path delivers author content to a document other than one it was parsed into, cloned into or adopted into.

## §4. Mechanism

### §4.1 Home / kind

- The rule, the predicate and the lifetime hook live in **`elidex-ecs`** (`crates/core/elidex-ecs/src/dom/template_owner.rs`
  (NEW), an `impl EcsDom` block): both parser crates and `elidex-dom-api` call the helper, and the parser crates cannot
  depend on `elidex-dom-api` (umbrella §9 N-1's crate-graph evidence). `elidex-ecs` is engine-independent, so the
  Layering mandate holds; the VM touches none of it (the `content` getter and `ownerDocument` getter marshal only).
- Components (D-C1) in `crates/core/elidex-ecs/src/components.rs` beside `TemplateContents` (`:366`).
- Realm: §4.12.3 step 1.1.1 creates I "given document's relevant realm". elidex has one realm per `EcsDom` (T3 draft
  H-T6); B1's multi-realm World must create I in D's realm (§8 hand-off H-3).

### §4.2 The rule and the establishing helper (D-C2, D-C3)

`appropriate_template_contents_owner_document(D)`: marker on D → D; link on D whose target `contains` and is a
Document → that; else spawn with `create_document_node`, insert `InertTemplateDocument`, insert
`AssociatedInertTemplateDocument(I)` on D, return I. A stale link (target destroyed or recycled) is replaced, never
returned (D-C7 makes it unreachable; the check is defensive, as `owner_document`'s guard is). `attach_template_contents`
calls it on `Some(node_document)`, then creates the fragment with that owner, as today.

### §4.3 Adopt (D-C5)

One walk from `root` with a work list of `(node, target)`; `root` starts with `document`; a template's content fragment
is pushed with `f(target)`; light children and the shadow root inherit their parent's target. Each visited node gets
`AssociatedDocument(target)`. Choice between a new `TemplateContentsWalk` variant that yields the edge kind and a
sibling walker is input; "one issue, one way" weighs toward extending the existing walker so that teardown, adopt and
the bind pass keep one uncapped walker (umbrella §9 N-5 lists `collect_template_inclusive_descendants`,
`crates/dom/elidex-form/src/init.rs:111`, and `derive_element_components`,
`crates/dom/elidex-html-parser/src/element_init.rs:60`, as further template-inclusive walkers; reconciling them is not
C1's, §7).

### §4.4 Clone (D-C6) and parser stamps (D-C4)

`replicate_template_contents(src, dst, …, clone_doc, …)`: `new_fragment = attach_template_contents(dst, clone_doc)`
(mapped inside), then clone each source contents child with `owner_document(new_fragment)` instead of `clone_doc`.
Parser: strict `insert_foreign_element` stamps the intended parent's node document (`parent` from
`appropriate_place()`; a Document parent is itself); tolerant passes `owner_document(fragment)` into the contents
recursion and the host's node document into the declarative-shadow recursion.

### §4.5 What C1 provides to slice b (S-1's template-contents extension, S-2 branch A)

1. **Node document, at creation.** On C1's head, every node in template contents (every creation path of A2, every
   clone of A5, every node placed there by C0's adoption) resolves `owner_document` to an I, before any script can
   observe it. Slice b needs nothing else to make its creation-time raw values read null (N1's node clause: the node
   is detached, so the effective node document is `owner_document` = I ≠ bound) and to make Window-delegating pairs
   target null (T3 step 3: I is not the active document).
2. **The template-origin fact.** `is_inert_template_document(owner_document(n))` (NEW). It is carried by a clone and a
   clone of a clone (A5: a contents clone is in I), and lost by a node moved out of contents (C0 adopts it), which is
   exactly S-2's "tracks the node's spec node document, not its history".
3. **Pre-bind template contents.** S-1 extends to them in branch A; the walk that reaches them is the existing
   uncapped walker with template contents included (H-C15's `Exclude` stays for slice a's K-5 / K-6). C1 adds no walk;
   it makes the contents' node document I, so slice b's processing there writes values that read null.
4. **No insertion fallback.** C1 adds no processing at insertion (slice a's Insert arm is unchanged), so S-4 is not
   weakened.

### §4.6 What C1 requires of C0 (insertion adoption), if O-1 (a)

- **C0-1.** Every path the DOM *insert* algorithm covers (and the paths that delegate to it: `replaceChild`, the
  ParentNode / ChildNode mixins, `innerHTML` / `outerHTML` / `setHTMLUnsafe` placement, `Range` insertion) adopts each
  inserted node into the parent's node document when they differ, through `adopt_subtree` (D-C5).
- **C0-2.** The parent's node document of a template contents fragment is the fragment's node document.
- **C0-3.** Adoption happens before the node's `MutationEvent::Insert` is dispatched (so slice a's T-a2 and later reads
  see the adopted document).
- **C0-4.** C0's plan decides the effective-node-document proxy's collapse (`scripting.rs:51-53`: "when it lands
  `AssociatedDocument` becomes reliable for connected nodes and this rule collapses") and its interaction with T3's
  `effective_node_document` and N4's planned promotion of that rule into `EcsDom` (N1 memo §8, edge n3). C1 neither depends on nor changes
  that choice.

### §4.7 ECS-native and Layering checks

- Representation: two components, no side-store (D-C1). Lifetime by despawn cascade, not refcount or sweep (D-C7).
- One rule (D-C2), one establishing helper (D-C3), one adopt function (D-C5): §5.1's populations converge on them.
- VM: no new host code; `html_template_proto.rs` keeps marshalling the link (`:106`).

### §4.8 Docstring citation table (Axis 4)

| symbol | citation |
|---|---|
| `appropriate_template_contents_owner_document` (NEW) | HTML §4.12.3 *appropriate template contents owner document*, steps 1–2 (1.1.2 vacuous: no XML documents) |
| `AssociatedInertTemplateDocument` (NEW) | HTML §4.12.3 *associated inert template document* |
| `InertTemplateDocument` (NEW) | HTML §4.12.3 ("a Document created by this algorithm") |
| `is_inert_template_document` (NEW) | HTML §4.12.3 (the same fact) |
| `attach_template_contents` (doc rewrite) | HTML §4.12.3 establishing steps 1–3 (step 2's host not modelled, §7) |
| `adopt_subtree` (doc rewrite) | DOM §4.5 *adopt* steps 3.1 and 3.4; HTML §4.12.3 template *adopting steps* |
| `replicate_template_contents` (doc rewrite) | HTML §4.12.3 template *cloning steps* step 2 |

## §5. Populations, touched files, named risks

### §5.1 Populations (re-runnable)

- **P-1 establishing sites** (H-C2): `/usr/bin/grep -rnF 'attach_template_contents(' crates --include='*.rs' |
  /usr/bin/grep -v '/tests/\|tests\.rs\|/tests_'` → 4 callers + definition. All four keep their call and pass the
  template's node document (D-C3).
- **P-2 adopt**: `/usr/bin/grep -rnF 'adopt_subtree(' crates --include='*.rs'` → definition + 1 caller at `74e15df1`;
  C0 adds its insertion callers.
- **P-3 explicit owner stamps on clones**: `/usr/bin/grep -rnF 'set_associated_document(' crates --include='*.rs' |
  /usr/bin/grep -v '/tests/\|tests\.rs\|/tests_'` → `tree_clone.rs:221`, `:230`, `:417`, `clone.rs:283`, `:284`, `:299`,
  `teardown.rs:257`, definition `mod.rs:793`. Only the template cloning step's document changes (D-C6).
- **P-4 parser owner stamps**: `/usr/bin/grep -rn '_with_owner(\|owner_document' crates/dom/elidex-html-parser-strict/src
  crates/dom/elidex-html-parser/src --include='*.rs' | /usr/bin/grep -v tests` → strict `insert.rs:114`; tolerant
  `convert.rs:63-290` (the threaded parameter) and `lib.rs:161`. D-C4 changes the strict foreign stamp and the two
  tolerant recursions; Q-S3 confirms which tolerant creations stamp.
- **P-5 node-document consumers** (behaviour changes for nodes in template contents, no code change):
  `/usr/bin/grep -rn 'owner_document(' crates --include='*.rs' | /usr/bin/grep -v '/tests/\|tests\.rs\|/tests_\|test_helpers'
  | awk -F: '{print $1}' | sort | uniq -c | wc -l` → 20 files at `74e15df1` (verified 2026-10-31). Q-S4 classifies each (unchanged / spec outcome /
  needs a change).
- **P-6 C2 statements**: `/usr/bin/grep -rn 'template-contents-owner-document' crates` → `mod.rs:556`,
  `scripting.rs:60`, `document.rs:262`; all three docstrings are rewritten (§5.2).
- **P-7 Document destruction**: `destroy_entity` on a Document (`teardown.rs:331` and, at `1ed126b4`, the strict
  error-path `despawn_subtree(document)`); the full set, including any DOMParser / iframe document teardown, is Q-S1.
- **P-8 render / layout roots**: `/usr/bin/grep -rn 'root_entities()' crates --include='*.rs' | /usr/bin/grep -v test`
  → 3 consumers (H-C11; verified 2026-10-31); D-C9 changes the source only.

### §5.2 Existing tests and docstrings whose expectation changes (red at C1 by design, rewritten in the same commit)

Candidates from `/usr/bin/grep -rln template crates --include='*.rs' | xargs /usr/bin/grep -ln 'owner_document\|ownerDocument'`
(17 files) narrowed by reading: `crates/dom/elidex-html-parser/src/tests.rs:427-428` (contents' first child's owner is
"the document"), `crates/dom/elidex-dom-api/src/node_methods/tests/clone.rs:218` (shallow template clone's
`content.ownerDocument`), `crates/core/elidex-ecs/src/dom/tests/associated_document.rs`, `dom/tests/traversal.rs:18`
(`attach_template_contents(template, Some(doc))`: still compiles, value changes), umbrella cells C14 and C18 in slice
a's VM tests (`crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs_once.rs`,
`tests_event_handler_attrs_window_delegating.rs`). The complete list at C1's base is Q-S8. Docstrings: P-6, plus
`components.rs:356-364` (`TemplateContents`' adopt sentence), `walkers.rs:19-27` (`TemplateContentsWalk::Include`'s
"re-home" use), `mod.rs:842-851` (the fragment-fallback comment), `scripting.rs:59-60`.

### §5.3 Touched files

| file | lines at `74e15df1` | change | 1000-line decision |
|---|---|---|---|
| `crates/core/elidex-ecs/src/dom/template_owner.rs` (NEW) | — | D-C2 rule, predicate, D-C7 hook | new file, by design: keeps `mod.rs` under the limit |
| `crates/core/elidex-ecs/src/dom/mod.rs` | 930 | `mod` line; `attach_template_contents` doc and one call; fragment-fallback comment | net ≤ +10 lines, stays < 1000; re-measure at the base. If the base is ≥ 990, the template-contents helpers (`:540-591`) move to `template_owner.rs` as a standalone pure-move prereq first (O-4) |
| `crates/core/elidex-ecs/src/components.rs` | 822 | two components, `TemplateContents` doc | stays < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/teardown.rs` | 335 | `adopt_subtree` (D-C5); Document cascade in `destroy_entity` (D-C7) | < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/walkers.rs` | 392 | per §4.3 | < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/navigation.rs` | — | `root_entities` exclusion (D-C9) | re-measure (Q-S8 also counts it) |
| `crates/core/elidex-ecs/src/dom/tree_clone.rs` | 425 | clone-policy rows (D-C8) | < 1000 |
| `crates/dom/elidex-dom-api/src/node_methods/clone.rs` | 458 | D-C6 | < 1000 |
| `crates/dom/elidex-dom-api/src/document.rs` | 665 | doc only (P-6) | < 1000 |
| `crates/dom/elidex-html-parser/src/convert.rs` | 682 | D-C4 (b), doc | < 1000 |
| `crates/dom/elidex-html-parser-strict/src/tree_builder/insert.rs` | 388 | D-C4 (a) | < 1000 |
| `crates/dom/elidex-html-parser-strict/src/tree_builder/shadow.rs` | 225 | doc (D-C3); DSD owner if Q-S2 finds a stamp | < 1000 |
| `crates/script/elidex-script-session/src/scripting.rs` | 293 | doc `:59-60` | < 1000 |
| `crates/dom/elidex-dom-api/src/node_methods/tests/clone.rs` | 992 | one expectation rewritten in place; **no new tests** (they go to a new file) | stays < 1000 |
| `crates/core/elidex-ecs/src/dom/tests/template_owner.rs` (NEW) | — | unit cells U-* (§6) | new file |
| `crates/script/elidex-js/src/vm/tests/tests_template_owner_document.rs` (NEW) | — | VM cells T-* (§6) | new file |
| `crates/script/elidex-js/src/vm/tests/tests_html_t2d_protos.rs` | 1207 | **not touched** (no owner assertion in it: `/usr/bin/grep -n 'owner' <file>` → `output_form_owner` only) | not triggered; if Q-S8 finds an assertion to rewrite, split it first as a standalone prereq |

### §5.4 Named risks (and the observable change each is)

- **R-C1 `ownerDocument` of template contents** → I (spec). A script that compared it to `document` sees `false`.
- **R-C2 URL resolution in template contents** → against `about:blank` (spec, §2.4.3), so `a.href` / `img.src` IDL
  reads of relative URLs return the attribute value unchanged while inside contents. Web-compat question O-3; cell T20.
- **R-C3 handlers in template contents** → raw values read null and do not run (N1 + C1; C14 inverts). This is the
  spec's outcome and the umbrella's S-2 end state reached early for the read; slice a's processing point is unchanged.
- **R-C4 `<body>` / `<frameset>` in template contents** → null target (T3 + C1; C18 changes to T3's W2 shape).
- **R-C5 a new Document wrapper (I) is reachable from script.** Document methods on it must not route to the page
  (`core.rs:279`'s `document_root()` fallback is for orphans, not for I; Q-S5). DOMParser documents are the precedent.
- **R-C6 I as a root.** Excluded (D-C9); cell U6.
- **R-C7 lifetime.** A leaked I per fragment parse or per strict parse error would grow the World; cells U4, U5.
- **R-C8 C0 absent.** Rows 1, 2, 5 of §1's table; cell T9 is the guard. If O-1 is decided against C0, T9 is red at
  C1's head and this memo is not implementable as K-0-safe.

## §6. Test plan

Columns: *HEAD/base* = `74e15df1` and C1's base (N1, T3, C0 landed; still C2), equal unless stated; *C1 = spec*.
**Red-first**: every *red* cell is run on C1's base before the fix with its control green, and the failure is recorded in
the PR description. Cross-tier: cells marked (×2) run under both parser tiers.

### §6.1 VM integration (`tests_template_owner_document.rs`, NEW)

| cell | kind | script | HEAD/base | C1 = spec |
|---|---|---|---|---|
| T1 | red | `t = document.createElement('template'); I = t.content.ownerDocument; r = [I !== document, I instanceof Document, I.defaultView === null]` | `[false, true, false]` | `[true, true, true]` |
| T2 | guard | two `createElement('template')`: `t1.content.ownerDocument === t2.content.ownerDocument` (with T1's `!== document`) | `true` (both `document`) | `true` (same I) |
| T3 | red (×2) | initial HTML `<template id=a><b id=b></b></template>`: `a.content.ownerDocument !== document`, `a.content.getElementById('b').ownerDocument === a.content.ownerDocument` | `[false, true]` | `[true, true]` |
| T4 | red (×2) | `<template id=o><template id=i><p></p></template></template>`: `i.ownerDocument === o.content.ownerDocument`, `i.content.ownerDocument === o.content.ownerDocument`, `o.content.ownerDocument !== document` | `[true, true, false]` (all three are `document`; the third element discriminates) | `[true, true, true]` |
| T5 | guard (with U2) | `I = tpl.content.ownerDocument; t = I.createElement('template'); t.content.ownerDocument === I` | base: `I` is `document`, so `true` | `true`, and U2 shows no I-of-I entity was created |
| T6 | red | `dp = new DOMParser().parseFromString('<template id=t><i></i></template>','text/html'); c = dp.getElementById('t').content; [c.ownerDocument !== dp, c.ownerDocument !== document, c.ownerDocument !== tpl.content.ownerDocument]` | `[false, true, true]` | `[true, true, true]` |
| T7 | red (needs C0) | `tpl.innerHTML = '<b id=x></b>'; tpl.content.firstChild.ownerDocument === tpl.content.ownerDocument && tpl.content.firstChild.ownerDocument !== document` | `false` | `true` |
| T8 | red (needs C0) | `d = document.createElement('div'); tpl.content.append(d); d.ownerDocument === tpl.content.ownerDocument` | `false` | `true` |
| T9 | guard (K-0; the C0 dependency) | `c = tpl.content.cloneNode(true)` with `<a href="x.html">`; `[c.ownerDocument === tpl.content.ownerDocument]`; `document.body.append(c)`; `[a.ownerDocument === document, a.href.endsWith('/x.html')]` | `[false]`, then `[true, true]` | `[true]`, then `[true, true]` (C1 without C0: `[false, false]` after the append) |
| T10 | red | deep and shallow `tpl.cloneNode()` of a template with a nested template: `t2.content !== tpl.content`, `t2.content.ownerDocument === tpl.content.ownerDocument`, the nested copy's `content.ownerDocument` is the same I, shallow copy's contents empty and in I | contents in `document` | all in I |
| T11 | red | `dc = document.cloneNode(true)`; `x = dc.querySelector('template').content.ownerDocument; [x !== dc, x !== tpl.content.ownerDocument]` (support of `Document.cloneNode` in the VM: Q-S9) | `[false, true]` | `[true, true]` |
| T12 | red (×2) | `<template id=t><div id=h><template shadowrootmode=open><span></span></template></div></template>`: `h.shadowRoot.ownerDocument === t.content.ownerDocument`, and the span's; variant: a parser-built `<div>` in contents, `attachShadow({mode:'open'}).ownerDocument`; variant: `h.cloneNode(true)` with `shadowrootclonable` | `document` | I (whether DSD attaches inside contents: Q-S2) |
| T13 | red (needs C0) | `dp`'s template (with a nested template and a shadow host in its contents) appended into `document.body`: `t.ownerDocument === document`, `t.content.ownerDocument === tpl.content.ownerDocument`, nested contents and shadow content likewise | base with C0, C2: `t.content.ownerDocument === document` | I of `document` |
| T14 | red (×2) | `el.innerHTML = '<template id=f><p></p></template>'` (`el` in `document`): `f.content.ownerDocument === tpl.content.ownerDocument` | `document` | I of `document` |
| T17 | red (inverts umbrella C14; needs N1, C0) | umbrella C14's script | runs, `window.hit === 1` | does not run; `x.onclick === null` |
| T18 | rewrite (umbrella C18; needs T3) | umbrella C18's script | T3's outcome on C1's base | T3 draft's W2 shape for an element whose attribute was set inside contents; exact expectation re-derived at the base (§10 Q-3) |
| T19 | red (×2; needs C0) | `<template id=s><svg id=v></svg><b id=w></b></template>`: inside contents `v.ownerDocument === s.content.ownerDocument`; `document.body.append(s.content)`; `[v.ownerDocument === document, w.ownerDocument === document]` | inside: `false` (strict stamps `document`); after: `[true, true]` | inside: `true`; after: `[true, true]` |
| T20 | spec pin (O-3) | `<template id=h><a id=a href="x.html"></a></template>`: `h.content.getElementById('a').href` | absolute against the page base | `"x.html"` per §2.4.3 (browser measurement first, O-3) |
| T22 | red | `tpl.innerHTML = '<b id=x onclick="1">'`; read `x.onclick` | base (N1): a function (owner `document`, the `scripting.rs:59-60` false negative) | `null` |

### §6.2 Unit (`crates/core/elidex-ecs/src/dom/tests/template_owner.rs`, NEW)

| cell | kind | shape | expected |
|---|---|---|---|
| U1 | red | `f(D)` twice → same entity, marked; D linked | equal, marked |
| U2 | red | `f(f(D)) == f(D)`; count of marked entities is 1 | 1 |
| U3 | red | D without any template: no marked entity after parse and `createElement('div')` | 0 (lazy) |
| U4 | red | 100 fragment parses with a template (`begin_detached_fragment` / `finish_detached_fragment`, both tiers): marked-entity count stays 1 (the context's I) | 1 |
| U5 | red | strict parse error with a template in the fragment: no marked entity survives | 0 extra |
| U6 | red | `I` with an element child: `root_entities()` excludes I | excluded |
| U7 | red | `destroy_entity(D)` destroys `f(D)`; a contents node outliving it resolves `owner_document` → `None` | destroyed, `None` |
| U8 | red | clone policy: `clone_subtree(D)` copy has no link; `clone_subtree(I)` copy has no marker | absent |
| U9 | red | `adopt_subtree(t, X)` on a template with nested contents and a shadow host in contents: light/shadow descendants → X, contents and nested contents and their shadow trees → `f(X)` | as stated |
| U10 | guard | `adopt_subtree` past the depth and breadth caps (as `despawn_subtree_tears_down_past_the_traversal_depth_cap`, `dom/tests/destroy.rs:381`) | every node re-homed |

### §6.3 Mutation guards

| mutation | killed by |
|---|---|
| a new I per template (no link) | T2, U1 |
| no marker check (I of I) | T4, T5, U2 |
| caller passes the template's document unmapped (C2) | T1, T3, T6 |
| adopt re-homes contents to `document` (HEAD) | T13, T14, U9 |
| contents children cloned with the operation's document | T10 |
| no destroy cascade | U4, U5, U7 |
| link or marker copied by clone | T11, U8 |
| `root_entities` not excluding I | U6 |
| strict foreign stamp unchanged | T19 (strict leg) |
| tolerant recursion threads the template's document | T3, T12 (tolerant legs) |

### §6.4 Kept green

Slice a's U8 (ii) (the bind pass does not walk template contents), E1–E8, and the #380 template suite
(`tests_html_t2d_protos.rs` `template_*`, `dom/tests/destroy.rs:283`, `dom/tests/traversal.rs:8`). Commands: `cargo test
-p elidex-ecs --all-features`, `-p elidex-dom-api`, `-p elidex-html-parser`, `-p elidex-html-parser-strict`,
`-p elidex-script-session`, `-p elidex-js`, then `mise run ci`.

## §7. Out of scope and slots

| item | where | defect (stated, not the remedy) |
|---|---|---|
| insertion adoption | C0, `#11-cross-document-adopt-on-insert` | no insertion adopts (H-C6); §1's rows 1, 2, 5 regress without it |
| DocumentFragment *host* of template contents (§4.12.3 establishing step 2) | scout Q-S7; slot to be named by the PM if none exists | the contents fragment has no host link, so host-including-ancestor checks cannot see the template |
| template `for` attribute, content patching (§4.12.3) | not tracked here | elidex implements neither the attribute nor *prepare content patching* |
| DOM *adopt* steps 3.2 / 3.3 (registries, `adoptedCallback`) | C0's plan | not modelled |
| render/layout roots for DOMParser documents and detached subtrees with children | Q-S6 | `root_entities` returns them and H-C11's consumers keep any with a child |
| template-inclusive walkers (`collect_template_inclusive_descendants`, `derive_element_components`) | umbrella §9 N-5 (slice b) | three walkers for one concept |
| multi-realm creation of I | B1 (§8 H-3) | I is created in the single realm |

## §8. Edges, hand-offs and records

**Edges.** c1: N1 → C1 (C1's read-null cells T17, T22 assume N1's compile gate). c2: T3 → C1 (T18 assumes T3's step 3).
c3: C0 → C1 (semantic precondition, §1; O-1). c4: C1 → slice b (umbrella x5; §4.5 is the provision). T3 and C0 are
independent of each other unless C0's plan collapses T3's `effective_node_document` (C0-4).

**Hand-offs.**
- H-1 to slice b: §4.5 items 1–4; the umbrella's §9 N-2 / N-3 (branch B's per-node fact) do not apply in branch A.
- H-2 to C0: §4.6 C0-1 to C0-4; D-C5's adopt function is C0's to call.
- H-3 to B1: create I in D's relevant realm.
- H-4 to T3 (draft): its A3×A8 note ("a `body` in template contents reports the bound document … C1's, not T3's") and
  its §5.2 row for C18 stay as they are; C1 rewrites C18 (T18).

**Records (for the PM; exact texts).**
- **R-1 umbrella §1.2** (in place, if the PM accepts the vehicle of O-2): add the row
  "| **C1** `EH-attrs-at-creation-C1` (owner: Layout lane, this program; PM decision 123) | slot
  `#11-template-contents-owner-document`: the inert template contents owner document (branch A, §1.6) | N1 and T3
  landed; C0 landed (O-1) |" and, if O-1 (a), a C0 row; the order line becomes "approval PR → slice a → { IBP-sandbox
  N1, T3, C0 } → C1 → slice b".
- **R-2 ledger `#11-template-contents-owner-document`**: "C1 plan-memo `docs/plans/2026-10-eh-attrs-c1-template-owner-document.md`
  rev 1 (2026-10-31). Finding: C1 needs insertion adoption first (`#11-cross-document-adopt-on-insert`); without it a
  node cloned from template contents and appended into the document keeps the inert document as its node document
  (memo §1). PM decision requested (memo O-1). Closes at C1's landing."
- **R-3 umbrella in-place corrections** (rev 8.4, at C1's PR if O-2 says so): cell C14's expected column "(spec: does not
  run; slice b inverts it under S-2)" → "(spec: does not run; C1 inverts it, with N1's compile gate and C0's adoption
  at the placement)"; K-5's "From N1, the DOMParser half reads null." → "From N1, the DOMParser half reads null; from
  C1, the template-contents half reads null."; §8.2 defect (5)'s "its template-contents half, and the first clause for
  both, remain until slice b" → "its template-contents half until C1 lands, and the first clause for both until slice
  b"; §1.4's interim line gains the same C1 qualifier as N1's (ii).
- **R-4 ledger `#11-cross-document-adopt-on-insert`**: "Trigger hit 2026-10-31: EH-attrs C1 requires it (C1 memo §1,
  §4.6). PM decision requested on owner and placement (C1 memo O-1)."
- **R-5 slice a's docstring** `scripting.rs:59-60` ("the `<template>` contents false-negative rides
  `#11-template-contents-owner-document`") is removed by C1 (§5.2); no ledger text.

## §9. Definition of done

1. Coordinates re-measured at C1's base (N1, T3, C0 landed); §5.1 populations re-run and any change recorded in the PR.
2. `appropriate_template_contents_owner_document`, `is_inert_template_document` and both components exist in
   `elidex-ecs`, with §4.8's citations; no other site computes a template contents' node document (P-1 re-run).
3. D-C3 to D-C9 implemented; the clone-policy table has both rows.
4. §6.1 and §6.2 cells present; every red cell run red on the base with its control green, recorded in the PR
   description; T9 green at the head.
5. §5.2 rewrites done; P-6's grep returns no hit in `crates/`.
6. `cargo test` for the six crates of §6.4 and `mise run ci` green.
7. `python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-10-eh-attrs-c1-template-owner-document.md`
   exits 0 (run; do not trust a stored verdict).
8. The PM has applied R-1 to R-4 or recorded why not.

## §10. Open questions

**For the PM / user.**
- **O-1 C0.** (a) Insertion adoption lands as its own PR before C1 (recommended; §1); (b) C1 absorbs it (one PR, A6×A8
  widens to every node-document consumer); (c) C1 lands without it (rejected: §1 rows 1, 2, 5 are worse than HEAD). If
  (a): who owns C0 and where it sits (this program, or its own), and whether T3 or C0 goes first (C0-4).
- **O-2 Vehicle for the umbrella edits.** C1's PR carries R-1 / R-3 in place (N1's precedent, PM decision 93), or the PM
  records them.
- **O-3 URL resolution in template contents.** The spec is explicit (§2.4.3: base `about:blank`), but HEAD resolves
  against the page and the change is visible to scripts that read `href` / `src` inside contents. Measure in the built-in
  Chromium (and one other engine) before implementation: `t = document.createElement('template'); t.innerHTML = '<a
  href="x.html">'; t.content.firstChild.href`. If engines agree with the spec, T20 stands; if not, the user decides.
- **O-4 Split.** If `mod.rs` is ≥ 990 lines at the base, the template-contents helpers move to `template_owner.rs` as a
  standalone pure-move PR first (the #537 precedent) or as a standalone first commit of C1's PR.

**For plan-review.**
- **Q-1** Whether a shared node-document helper on `EcsDom` (a Document is its own node document) replaces the per-parser helper
  of D-C4, given T3's `effective_node_document` and N4's planned promotion (three functions for one concept).
- **Q-2** The adopt walk's shape (§4.3): a `TemplateContentsWalk` variant that reports the contents edge, or a sibling.
- **Q-3** T18's exact expectation, after T3's plan-review fixes W2.

**Scout questions** (facts this memo needs; not read here).
- **Q-S1** Every path that destroys a Document entity: DOMParser documents (are they ever destroyed?), iframe documents
  at navigation, `Vm::unbind`, the fragment throwaway (`teardown.rs:331`), the strict error path at `1ed126b4`.
- **Q-S2** Does `attach_shadow_with_init` stamp `AssociatedDocument` on the shadow root? What owner does the strict DSD
  path (`tree_builder/shadow.rs:150-170`) give the shadow root and its content? Does a declarative shadow root attach
  inside template contents in each tier today?
- **Q-S3** In the tolerant tier, which node creations stamp the threaded `owner_document` explicitly (text, comment,
  shadow root, DSD content)?
- **Q-S4** For each of P-5's 20 files (verified 2026-10-31 by P-5's command), does the reader's outcome change for a node in template contents, and is the new
  outcome the spec's? In particular: the custom-element reaction consumer, the form-control reconciler and
  `collect_template_inclusive_descendants`, `IframeData` (an iframe in contents must not load), `<img>` / `<link>`
  fetches, `<script>` preparation, `cssom_sheet.rs:213`, `selection.rs`, `range/live.rs`, `focus/delegate.rs`.
- **Q-S5** Which `Document` methods, called on a non-bound Document wrapper, fall back to `document_root()` or the bound
  document (`getElementById`, `body`, `defaultView`, `URL`, `cookie`, `createElement`'s receiver)?
- **Q-S6** Is there a slot for `root_entities` returning DOMParser documents and detached subtrees with children?
- **Q-S7** Does elidex model a DocumentFragment's *host* (template contents' host = the template) or the
  host-including inclusive ancestor in pre-insert validity? Is there a slot?
- **Q-S8** At C1's base, the full list of tests and docstrings that assert the C2 owner (template contents owned by the
  template's document), and `navigation.rs`'s line count.
- **Q-S9** Does the VM expose `Document.prototype.cloneNode` for the page document (T11)?
