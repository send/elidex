# C1 `EH-attrs-at-creation-C1` — template contents are owned by the inert template contents owner document (per-PR plan-memo)

Nested PR **C1** of the standalone program `EH-attrs-at-creation` (umbrella
`docs/plans/2026-10-event-handler-attrs-at-creation.md`, approved by #551 `b9122157` at rev 8, carried to rev 8.3 in
place by #570; "the umbrella"). It closes slot `#11-template-contents-owner-document` (carved from #380, `c801ef6c`),
closes T3's residual **R-A** (§9 item 9), and is slice b's precondition x5 in **branch A** (umbrella §1.3 x5, §1.6).
Siblings cited by ID:
- slice a (`docs/plans/2026-10-eh-attrs-slice-a.md`, landed by #563 `3cd84a04`; "the slice-a plan");
- IBP-sandbox N1 (`docs/plans/2026-10-ibp-sandbox-handler-gates.md`, rev 4.5, merged as a plan by #570; implementation
  **#580, merged** as `27b50dfe` on 2026-11-01; "the N1 memo");
- **C0**, a program of three PRs **C0a → C0b → C0c** (PM decision 244), **all landed**: C0a #595 `4893df0f`, C0b #608
  `062337aa`, C0c #611 `0a5a77ba` (with the D-14 splits (a) #593 `620758a9`, (b) #599 `e8b6a24d`, (c) #600
  `efe8d05b`). Program memo `docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md` (on main, rev 3.9.1; "the C0
  draft", its decision IDs D-n carry their owning PR) and per-PR memos
  `docs/plans/2026-11-eh-attrs-c0a-replace-record-order.md`, `2026-11-eh-attrs-c0b-node-document-fact.md`,
  `2026-11-eh-attrs-c0c-adopt-notification-adoptnode.md` ("the C0b memo" etc.), each landed with its PR. "C0 rev 1" /
  "C0 rev 2" mark a statement of that earlier rev;
- T3 (`docs/plans/2026-10-eh-attrs-t3-target-determination.md`, rev 4.4 on local branch
  `layout-eh-target-determination-plan` `d15a141c`, plan-review closed, not on main; "the T3 draft"; rev ≤ 3.1 cited
  its rev 3 at `14ff3dc2`, §0.3 P-C25);
- L3's fragment plan (`docs/plans/2026-10-fragment-dsd-not-on-context.md`, #574 `6aea28da`; PR 0a #576 `cc6a6665` and
  PR 1 #577 `ee6c7a1a` landed);
- L3's Range contents program (`docs/plans/2026-11-range-contents-program.md`, landed by #583 `12ea5dce`; RC-0a
  landed by #587 `fab44a1d`; **RC-2a, which carries D14, landed by #596 `17b2a84d`** (plan
  `docs/plans/2026-11-range-rc2a-clone-created.md`); "the Range program"): its Appendix A site IDs A1–A12 and its RC-2
  item D14, read on `origin/main`;
- **C1-pre (Document wrapper)**, landed by #616 `c7cce8c8` (plan `docs/plans/2026-11-eh-attrs-c1-pre-document-wrapper.md`,
  "the C1-pre memo"; owner Layout, PM 328 / 335 / 339). It discharges slot `#11-bridge-document-post-install`: every
  Document wrapper gets one `Document.prototype` at `create_element_wrapper` (C1-pre D-1). C1 relies on its result only
  (§4.1, edge c-pre); its record R-2 to this memo is applied in rev 3.7.
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
  212") lands before C1 and rewrites the same child-clone loop; C1 rebases on it. Done: D14 landed in RC-2a (#596).
- **PM decision 607** (relayed by the driver, 2026-11-10): PM decision 145 (a pre-implementation measurement step) is
  **withdrawn**, superseded by the later order of grounds (the user, 2026-11-06: the spec, then elidex's principles,
  and only if still undecided a measurement as evidence) and the PM 422 / 424 precedents. C1's acceptance criterion
  is the spec value of each cell (§6); every outcome C1 changes is spec-determined (§10.5), so no measurement step
  remains.
- The option texts of decisions 144, 159, 199, 207, 215 and 607 were not shown to this memo's author; they are cited
  as relayed in the driver's briefs, not quoted.

**Prerequisites (at C1's base).** N1's implementation (#580), **C0a, C0b and C0c** landed; L3 RC-0a (#587) and RC-2a
(which carries D14) landed (§4.4); **C1-pre** landed (edge c-pre). Order: N1 → C0a → C0b → C0c → C1-pre → C1 → T3 →
slice b (umbrella §1.2 *Order*, as edited by C1-pre's PR). **All of them are on `origin/main` at `c7cce8c8`** (rev 3.7,
`git log --oneline 593aa314..origin/main`: #580 `27b50dfe`, #587 `fab44a1d`, #595 `4893df0f`, #596 `17b2a84d`, #608
`062337aa`, #611 `0a5a77ba`, #616 `c7cce8c8`), so `c7cce8c8` is C1's base until main moves (§0.1). The C0 dependency is
semantic, not cosmetic: §1's table shows that C1 on a base without insertion adoption makes the most common template
pattern worse than HEAD (PM decision 143); with C0 on the base it does not (§1, K-0 lens).

**Scope.** One rule computes a template's contents' node document — WHATWG HTML §4.12.3's *appropriate template contents
owner document* — and every site that establishes, clones, parses into or adopts template contents goes through it,
reading node documents only through C0's `EcsDom::node_document` (landed by C0b):
1. the inert document itself, as an ECS entity and two components (D-C1, D-C2);
2. the establishing steps at every template creation (both parser tiers, `createElement`, the clone post-pass), through
   C0's hook point, the second argument of `attach_template_contents` (D-C3);
3. the value of C0's new-node entry `EcsDom::node_document_for_new_nodes` (landed by C0b, C0 draft D-10) inside
   template contents, as INV-C (§4.6 item 1) states it, with no template branch in the entry (D-C4);
   L3's A11 / A12 read the same entry (§4.5);
4. the template *adopting steps*: the document C0's one adopt walk gives each contents segment it does not skip, which
   is also the `new` of C0's per-segment (old, new) pair that `adoptedCallback` receives (D-C5);
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

**Changelog.** *Rev 3.10.2* (2026-11-11, PM 647): erratum of observations found at implementation — T13 and T25 gain
the legs that tell I from `document` (and I_dp from `dp`); U12 reads the walk counter; U5 states its precondition;
§5.1 P-4 / P-5's base check excludes test code (§10.10).
*Rev 3.10.1* (2026-11-10, PM 621): final clerical fixes — §2.1 A2 / A5 reference INV-C (b); INV-C (a)
scoped to an F with a node document; R-2's quote; T29's path; U12 / U14 homes; H-6 to T3 (§10.9).
*Rev 3.10* (2026-11-10, PM 619 / 618): plan-review R3 fixes — D-C6's per-pair document is guarded by
a new unit cell U14 (no adopt during a clone; T10 / T24 cannot see it, base observed 0 / 0, §10.8); DOMParser red cell
T29 and R-C2 scoped to the bound document; INV-C (b) covers the ownerless template; every §6.1 cell reaches its
elements without bare ids; §8's landing-condition reading (PM 618 (2)), record R-14 (PM 618 (1)), Re-eval-dates
(PM 618 (3)); citations (§3, §4.5, §4.9, §5.3, §11); §10.8.
*Rev 3.9* (2026-11-10, PM 612 / 613): plan-review R2 fixes; root fix — the contents-document invariant
is stated once, as INV-C (§4.6 item 1), and every other site cites it; T20 / T9 get a `<base href>` fixture, T20's
base value observed (§10.7); C0c U21 moves to "changes"; §8's records carry PM 613's dispositions (R-8 a facet of
`#11-browsing-context-state-ecs-components`, R-10 declined, R-13 added); §10.7.
*Rev 3.8.1* (2026-11-10): Q-S13 closed — §13.2.6.1 step 10 unreachable (§3, §11, §10.6).
*Rev 3.8* (2026-11-10, PM 607): plan-review R1 fixes — the pre-implementation measurement step is
gone (PM 145 withdrawn by PM 607; §1.1, D-C11 and their dependants removed, T20 asserts the spec value); every
§ ↔ title pair and step re-looked-up (four drifted); the contents-document invariant is conditional on the contents
fragment's node document (T23 route (v)); records R-2, R-4, R-8 to R-10 restated, R-12 added; §10.5.
*Rev 3.7.1* (2026-11-10, PM 604): Q-5 decided (a): T1 has three legs; T1's old legs 4–5 and M3's
`URL` / `documentURI` legs are `#11-document-base-url`'s cells (record R-11); the umbrella changelog lines for C1-pre's
and C1's §1.2 rows are a task of C1's PR (§9 item 11); §5.2's per-fn classification goes in C1's PR body; §10.4.
*Rev 3.7* (2026-11-10): C1-pre's record R-2 applied (T1, M3, edge c-pre, §4.1, §4.8, §5.3, R-C5,
Q-S10, H-C10, R-1); frame moved to `c7cce8c8`, where every prerequisite has landed, and every coordinate, population and
"on the base" premise re-measured there; premise errors P-C17 to P-C27 (§0.3); open question Q-5 (§11); §10.3.
*Rev 3.6.1* (2026-11-04, PM 337): T1's fixture sets the page URL, base column `[false, true, false, false, false]` (IMP-A); D-C3's clone post-pass → §4.4 `pair_owner` (MIN-B); A3×A6, D-C5's opening and P-C14's correction point to D-C5 (MIN-C); M3 / edge c-pre: I's URL is C1-pre's default, C1's `f` has no URL duty (MIN-D).
*Rev 3.6* (2026-11-04, focused re-check after R1; PM 335): `inert_template_document_of` deleted (D-C2,
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

**Rev 3.7 frame.** Every `file:line` without a commit is a **`c7cce8c8`** coordinate (`origin/main` on 2026-11-10,
#616; `git merge-base HEAD origin/main` → `c7cce8c8`; this branch's head `33b9d5b5` is rev 3.6.1 merged with it, and
`git diff --stat c7cce8c8 HEAD -- crates` is empty, so the code read is `c7cce8c8`'s). Every prerequisite of C1 has
landed there (header *Prerequisites*), so `c7cce8c8` **is** C1's base until main moves: the "on the base" premises of
rev ≤ 3.6.1 are measured, not inferred, in this rev (§0.2), and where one was wrong it is in §0.3. Since `593aa314`
(`git diff --stat 593aa314 c7cce8c8 -- crates | tail -1`: 558 files, verified 2026-11-10) C0b moved the node-document reader and the new-node entry
to `crates/core/elidex-ecs/src/dom/node_document.rs`, replaced `adopt_subtree` by `EcsDom::adopt`'s private walk in
`dom/tree/mutation.rs` and made every creator stamp at spawn; RC-2a rewrote the clone post-pass's pass 4 (D14); C1-pre
gave every Document wrapper `Document.prototype`. A cite marked `593aa314` or `27b50dfe` is history (the code it names
moved or is gone); the rev 3 / 3.2 frame checks are in `git show a711c4fb:<memo>` §0.1. Where a symbol exists, it is
cited by symbol and the line is a convenience: re-find with `/usr/bin/grep -n '<symbol>' <file>`. If main moves before
C1's implementation branch is cut, **every coordinate is re-measured at the cut** (§9 item 1). Code references come
from `git grep -n` / `/usr/bin/grep -n` / `sed -n` over the `c7cce8c8` tree, run 2026-11-10; populations are the
commands of §5.1 with their output at `c7cce8c8`. Every § number ↔ title pair and every algorithm step comes from
`.claude/tools/webref` (`heading`, `dfn`, `body`). **Every one was re-run on 2026-11-10 (rev 3.8)**; the commands are
in §3. Earlier revs ran them on 2026-11-01 / 2026-11-04 and rev 3.7 re-read only the cites it touched, so rev 3.7's
"every pair unchanged" (§10.3) was not measured: four had drifted (§10.5).

### §0.2 Premises re-measured

At `c7cce8c8` (= C1's base, §0.1). Rev ≤ 3.6.1's "on the base" clauses were the C0 / D14 / C1-pre drafts' statements;
each is now checked against the landed code, and the rows say where it differs (§0.3). `prod` is §5.1's production
filter (path filter plus inline `#[cfg(test)] mod … {` blocks).

| id | claim | evidence |
|---|---|---|
| H-C1 | One establishing helper exists; its owner parameter is the C2 approximation | `EcsDom::attach_template_contents(template, owner_doc: NewNodeOwner)` `crates/core/elidex-ecs/src/dom/mod.rs:614`; doc `:594-613` ("this ships the **C2 approximation** `owner_doc = template's own node document`", `:608`; slot `:610`); creates the fragment with `create_document_fragment_with_owner(owner_doc)` (`:628`) and links `TemplateContents { fragment }` (`crates/core/elidex-ecs/src/components.rs:366`). The doc's caller list (both tiers, `createElement`, the clone post-pass, each passing `node_document_for_new_nodes(template)` or the clone operation's document) is current |
| H-C2 | Four production callers of the helper | `git grep -nF 'attach_template_contents(' -- 'crates/*.rs' \| prod` → definition `mod.rs:614` + clone post-pass `crates/dom/elidex-dom-api/src/node_methods/clone.rs:427` (`clone_doc`, the operation's document), `createElement` `crates/dom/elidex-dom-api/src/document.rs:270` (`node_document_for_new_nodes(entity)`), strict `attach_ordinary_template_contents` `crates/dom/elidex-html-parser-strict/src/tree_builder/shadow.rs:122-124` (`node_document_for_new_nodes(template)`), tolerant `convert_node` `crates/dom/elidex-html-parser/src/convert.rs:291-292` (`node_document_for_new_nodes(entity)`). The path filter alone also admits two inline-test hits (`elidex-custom-elements/src/upgrade.rs:469`, `elidex-form/src/clone.rs:268`; §0.3 P-C21) |
| H-C3 | One new-node owner entry, in `elidex-ecs` (C0b) | `EcsDom::node_document_for_new_nodes(source) -> NewNodeOwner` `crates/core/elidex-ecs/src/dom/node_document.rs:66`: `node_document(source)`, except a `source` carrying `FragmentParseRoot { redirect_document }` (`components.rs:600`, inserted on the §13.4 root by `begin_detached_fragment`, `crates/core/elidex-ecs/src/dom/tree/teardown.rs:301-304`) answers that document (`:67-73`). `NewNodeOwner` (`:23`) has no public constructor outside `cfg(any(test, feature = "fixtures"))` (`:26-38`). The strict tree builder's method is gone (`git grep -n 'fn node_document_for_new_nodes' -- crates` → `node_document.rs:66` only) |
| H-C4 | Every parser creation takes its owner from the entry, given the target parent (HTML §13.2.6.1 *create an element for a token* step 6: "targetParent's node document", targetParent from the *adjusted insertion location*; text / comment insertion the same) | strict `crates/dom/elidex-html-parser-strict/src/tree_builder/insert.rs:64`, `:113`, `:140`, `:150`, `:167` (the Document), `:227`; `tree_builder/shadow.rs:123`, `:154` (DSD template, `fallback_parent`); tolerant `crates/dom/elidex-html-parser/src/convert.rs:65`, `:100`, `:291`. `git grep -nw 'owner_document' -- crates/dom/elidex-html-parser crates/dom/elidex-html-parser-strict` → 0: the tolerant threaded owner and `lib.rs:158`'s choice (both `593aa314`) are gone (C0 draft D-20) |
| H-C5 | One node-document reader, no positional fallback | `EcsDom::node_document` `node_document.rs:89` (a Document → itself; else its `AssociatedDocument` if a live Document, else `None`); `EcsDom::owner_document` `mod.rs:871-876` = `None` for a Document, else `node_document` (C0 draft D-1). Every insertion adopts before linking (`adopt_for_insert` `crates/core/elidex-ecs/src/dom/tree/mutation.rs:271`, called at `:50`, `:127`, `:180`; target `node_document_for_new_nodes(parent)`) |
| H-C6 | One adopt walk; it re-homes template contents to the **same** document as the template | `EcsDom::adopt` `mutation.rs:252` and `adopt_for_insert` (`:271`) both run `adopt_into` (`:288`), which walks with the private `adopt_walk` (`:322`): `for_each_uncapped_shadow_inclusive(root, TemplateContentsWalk::Include, …)` collects every node (`:333-336`); contents fragments are found through their templates (`:339-346`); each node is stamped with the one `document` (`stamp_node_and_attrs`, `:352`, `:395`); a contents fragment is skipped when `old == document` (`:371-373`) and records `AdoptedContents { fragment, documents: (old, document) }` (`:380-390`; type `crates/core/elidex-ecs/src/dom/mutation_event.rs:80`). One `MutationEvent::Adopt` per adopt call (`:298-304`). No function `adopt_subtree` exists (`git grep -n adopt_subtree -- crates` → 0; §0.3 P-C20) |
| H-C7 | A clone stamps its document at spawn; the post-pass threads one operation document | `clone_subtree` `crates/core/elidex-ecs/src/dom/tree_clone.rs:179` and `clone_node_shallow` `:226` pass the stamp into the spawn bundle (`clone_node_shallow_unchecked` `:253`, `AssociatedDocument` added `:367-368`, spawn `:370`). The post-pass `clone_node_with_shadow_honor` `clone.rs:235` reads **one** `document` (`:240`, re-read from the clone `:248`) and threads it into every pair's template pass (`replicate_template_contents(s, d, dom, subtree, document, …)` `:272`, def `:412`, fragment `:427`, children `:432`) and shadow pass (shadow children `clone_recording(dom, child, true, document, …)` `:295`, D14's subtree true at every pair); the host and shadow root carry no hand stamp (the shadow root takes its host's document at attach, `crates/core/elidex-ecs/src/dom/shadow.rs:194`) |
| H-C8 | The `ownerDocument` getter and URL resolution read the node document | dom-api getter `crates/dom/elidex-dom-api/src/node_methods/core.rs:264` (`owner_document(this)`, no `document_root()` fallback); href base `crates/dom/elidex-dom-api/src/element/href_accessor.rs:51` → `node_document_base_url` `crates/dom/elidex-dom-api/src/element/document_base.rs:68-71` (`node_document(node)`, `about:blank` if none) |
| H-C9 | The handler gates derive the node document through one rule | `scripting_disabled_for_platform_object` `crates/script/elidex-script-session/src/scripting.rs:73`, clause (b) `:83-84` (`node_document(target) != bound_document`, C0 draft D-7 (a)). Its doc names the template false negative as this slot's (`:64-66`). T3 (after C1) reads `node_document` too (C0 draft H-T) |
| H-C10 | A Document entity is spawned at **two** sites (C1-pre §0.3 P-7; §0.3 P-C19) | `EcsDom::create_document_node` `mod.rs:474` (spawn `:475-479`: `TreeRelation`, `NodeKind::Document`, `DocumentBaseUrl(about_blank_url())`), production callers `git grep -n 'create_document_node()' -- 'crates/*.rs' \| prod` → `mod.rs:459` (`create_document_root`), `teardown.rs:295` (fragment throwaway), `crates/dom/elidex-form/src/inert_document.rs:119` (DOMParser); and, for a Document source, the cloner's `clone_node_shallow_unchecked` `tree_clone.rs:253` (spawn `:370`), whose copy carries no `DocumentBaseUrl` (`tree_clone.rs:31`, out-of-domain) and no `AssociatedDocument` (a Document is its own node document, `:188-202`) |
| H-C11 | `root_entities` excludes only `DocumentFragment` | `EcsDom::root_entities` `crates/core/elidex-ecs/src/dom/tree/navigation.rs:362-374`; consumers `git grep -n 'root_entities()' -- 'crates/*.rs' \| prod` → `crates/core/elidex-render/src/builder/mod.rs:479`, `:995`, `crates/layout/elidex-layout/src/layout/mod.rs:371`, `crates/layout/elidex-layout/src/hit_test.rs:86` (4; the `lifecycle.rs:300` hit is `clear_root_entities`, unrelated) |
| H-C12 | Two production paths destroy a Document; DOMParser documents are never destroyed | `finish_detached_fragment`'s `destroy_entity(document)` `teardown.rs:334` and the strict rollback `despawn_subtree(document)` `crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs:234` (`git grep -n 'destroy_entity(\|despawn_subtree(\|\.despawn(' -- 'crates/*.rs' \| prod`: no other hit takes a Document); DOMParser documents (`inert_document.rs:119`) are never destroyed. `destroy_entity` (`teardown.rs:52`) cleans shadow links and focus, no Document cascade |
| H-C13 | elidex has no XML document | `git grep -n 'is_html_document\|XmlDocument' -- 'crates/*.rs'` → only the forward-looking note `mod.rs:793`; DOMParser serves `text/html` only (umbrella §3 row §8.5.1) |
| H-C14 | The clone-policy table enumerates every component | `tree_clone.rs:20-32` (module doc): `TemplateContents` row `:27`, "deliberate non-copy" row `:30`, out-of-domain `:31` |
| H-C15 | The bind pass excludes template contents | slice a's bind walk `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:178` (`TemplateContentsWalk::Exclude`); umbrella D-11, K-5, K-6 |
| H-C16 | Every Document wrapper is a Document (C1-pre, landed) | `create_element_wrapper` maps `PrototypeKind::Document` (`crates/script/elidex-js/src/vm/host_data/engine_feature/entity_kind.rs:188`) to `document_prototype` (`crates/script/elidex-js/src/vm/host/elements.rs:181`), registered once by `register_document_prototype` (`crates/script/elidex-js/src/vm/host/document.rs:295`); the path-specific installs are gone (`git grep -n 'install_document_methods\|document_methods_installed' -- crates` → 0). So `tpl.content.ownerDocument` (I) gets the Document members by construction. Members read `this` (C1-pre D-4 to D-8) except the URL pair: `document_url_value` `crates/script/elidex-js/src/vm/host/document_accessors.rs:157-169` brand-checks `this` and returns `navigation.current_url` (`:166`) for every Document (C1-pre R-10, `#11-document-base-url`; §0.3 P-C17; not asserted by C1, record R-11). No DOM interface object exists (`git grep -n '=> "\(Document\|Node\|Element\|DocumentFragment\|ShadowRoot\|HTMLDocument\)"' -- crates/script/elidex-js/src` → 0; C1-pre H-9) |
| H-C17 | No contents-fragment host link | scout Q-S7: only `ShadowRoot.host` (`components.rs:284`); no slot |

### §0.3 Premise errors (reported)

Rev 1's, with their status:
- **P-C1 (rev-1 brief: "the #380 plan").** Unchanged: no #380 plan is in the repository; the slot's "plan §5 Decision C /
  plan-review §12 ratified C2" is reachable only through the ledger and PR #380's body.
- **P-C2 (order).** Resolved by PM decisions 143 and 153 (C0 first; N1 → C0 → C1 → T3 → slice b).
- **P-C3 (umbrella §1.2 has no C1 row).** Still true at `c7cce8c8` (rows: slice a, C0, C1-pre, T3, slice b); R-1 (§8)
  adds it.
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
  C18 (the base's target rule ignores the document) and T3 rewrites it (T3 draft rev 4.4 §5.2, row C18; rev 3's P-T9 is
  gone, P-C25).
- **P-C12 (driver brief, input 6).** "D14: shadow children clone passes the operation's subtree flag". The code
  **already** passes the operation's flag (`clone_recording(dom, child, deep, shadow_doc, &mut pairs)`, `clone.rs:294` at `593aa314`),
  and that is the defect: DOM §4.4 *clone a node* step 6.7 clones shadow children "with … subtree set to true" (webref
  `body dom concept-node-clone`); the ledger text of `#11-clone-shadow-children-subtree` says the same. C1 assumes D14's
  form is **subtree = true** for shadow children (§4.4). **Confirmed (rev 3.7):** RC-2a (#596) passes `true` at every
  pair (`clone_recording(dom, child, true, document, &mut pairs)`, `clone.rs:295`; RC-2a plan D-5 (i)).
- **P-C13 (driver brief, input 4).** "`node_document_for_new_nodes` … THE single entry that decides the owner document of
  newly created nodes": at `593aa314` it is a `pub(super)` method of the strict tree builder (H-C3). The tolerant tier
  (`lib.rs:158`) and every dom-api creation site decide their owner elsewhere, and L3's A11 / A12 (in `elidex-dom-api`)
  cannot call it. A single entry therefore requires promoting it to `EcsDom`; who promotes it was rev 2's O-2.
  **Resolved (rev 3)** by PM decisions 223 O-2 / 227: C0 promotes it and migrates every site (C0 draft D-10, D-11).
- **P-C14 (C0 rev 1, A3×A4).** "Elements in contents can be custom only if upgraded there; C1 makes that impossible by
  giving contents the inert document." A custom element created in the document and appended into `tpl.content` stays
  custom in the inert document (adopt does not un-upgrade), so custom elements in contents exist on C1's head, and a
  later adoption of their template must give them `adoptedCallback` with their contents' pair (see D-C5). Hand-off H-2.
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

New in rev 3.7 (re-measured at `c7cce8c8`; each with its evidence):
- **P-C17 (rev 3.5–3.6.1, T1 legs 4–5, M3, edge c-pre: "I gets C1-pre's default about:blank at creation").** Wrong:
  C1-pre writes and reads no document URL (C1-pre memo PM 339, R-10). At `c7cce8c8` `document.URL` / `documentURI`
  (`document_url_value` `crates/script/elidex-js/src/vm/host/document_accessors.rs:157-169`) brand-check `this` and
  return the VM's `navigation.current_url` (`:166`) for **every** Document, so on C1's head `I.URL` and
  `I.documentURI` read the page URL until `#11-document-base-url` lands. C1-pre's record R-2 withdraws the edge c-pre
  text (applied: edge c-pre, M3, T1). What T1's legs 4–5 and M3's URL legs then assert on C1's head was **Q-5 (§11)**;
  **decided by PM 604 (rev 3.7.1): option (a)** — C1 does not assert them; T1 has three legs, M3 has no URL legs, and
  the URL legs are `#11-document-base-url`'s cells (§8 R-11). (Rev 3.8: §1.1, with M3, is removed, PM 607; R-11
  hands over the two VM cells only.)
- **P-C18 (rev 3.5–3.6.1, T1 leg 2 `I instanceof Document`).** Throws `ReferenceError` on any head: the VM has no
  DOM interface object (H-C16; C1-pre H-9, P-5). Replaced by C1-pre R-2's
  `Object.getPrototypeOf(I) === Object.getPrototypeOf(document)` (T1).
- **P-C19 (H-C10 "a Document entity is spawned in one place").** Two sites: `create_document_node` and, for a Document
  source, the cloner's spawn (C1-pre §0.3 P-7; H-C10). No consequence for D-C1 (I is spawned by
  `create_document_node`) or D-C8 (the clone-policy rows cover the cloner's copy).
- **P-C20 (H-C6, §3, §4.7, §5.1 P-2, D-C5: "C0 moves `adopt_subtree` to `mutation.rs` and keeps one contents edge
  whose target C1 replaces").** At `c7cce8c8` no `adopt_subtree` exists (0 hits); the walk is the private
  `adopt_walk` (`mutation.rs:322`), entered through `adopt_into` (`:288`) from `adopt` (`:252`) and every insertion
  (`adopt_for_insert`, `:271`). It has no separate contents edge: it collects every node of the `Include` walk and
  stamps each with the one `document` (`:352`, `:395`); a contents fragment is a segment boundary found through its
  template, whose skip test (`:371-373`) and pair (`:380`) also use `document`. Wording consequence (D-C5, §3, §4.7):
  C1's change is that, for every node whose innermost contents segment is live, the skip test, the segment's `new`
  and the stamp of the node (and its Attr entities) use `f(document)` instead of `document`; the root's own segment
  keeps `document`. D-C5's decision (contents → `f(target)`, skip on the mapped `new`, `f` at most once per adopt and
  only when a contents fragment is reached) is unchanged.
- **P-C21 (H-C2, §5.1 P-1 and every census: the filter `'/tests/\|tests\.rs\|/tests_'`).** It does not exclude inline
  `#[cfg(test)] mod tests { … }` blocks. At `c7cce8c8` it admits `elidex-custom-elements/src/upgrade.rs:469` and
  `elidex-form/src/clone.rs:268` (inside inline test modules: `upgrade.rs:369-370` `mod clone_creation_pass_tests`, `clone.rs:174-175` `mod tests`) into
  P-1, which then reads 6 callers, not 4 (verified 2026-11-10). Every census of rev 3.7 pipes through §5.1's `prod`, defined by the property
  (production code), not by the path. Its negative control: `crates/core/elidex-render/src/builder/mod.rs` declares
  `#[cfg(test)] mod tests;` (`:22-23`, an out-of-line module) above its `root_entities()` hits, which must stay (H-C11).
- **P-C22 (§5.1 P-3 "clone stamps" by `set_associated_document(`).** At `c7cce8c8` the stamp is part of every spawn
  (`spawn_node` `node_document.rs:109-114`; the cloner `tree_clone.rs:367-368`), and `set_associated_document` is
  `pub(crate)` (`mod.rs:851`) with the adopt walk as its only caller (`mutation.rs:403`, `:410`). P-3 is redefined by
  the property (where a clone's node document is decided).
- **P-C23 (§4.5 "the dom-api creation sites that decide an owner from `owner_document(this)` today").** Migrated by
  C0b: they read the entry, `node_document_for_new_nodes(..)` at `element/tree.rs:435`, `node_methods/text_content.rs:119`,
  `char_data/split_text.rs:89`, `:144`, `cssom_sheet.rs:213`, `element/select.rs:326`; the production `owner_document(`
  callers are `node_methods/core.rs:264` (the getter) and `selection.rs:617` only (§5.1 P-6).
- **P-C24 (C2 statements, §5.1 P-7's pattern).** `'template-contents-owner-document\|C2 approximation'` matches the
  spec anchor `#appropriate-template-contents-owner-document` (`mutation_event.rs:68`, not a C2 statement) and misses
  `clone.rs:425`'s "C2 owner-doc approximation" and `clone.rs:404-405` (split across lines). P-7 is redefined (§5.1).
- **P-C25 (T3 draft cites).** The T3 draft is rev 4.4 on `layout-eh-target-determination-plan` `d15a141c` (worktree
  `wt-t3-plan`), not rev 3 on `layout-eh-attrs-t3-plan` `14ff3dc2` (no ref of that name exists; `14ff3dc2` is not an
  ancestor of `d15a141c`). Its P-T9 is gone (C18's rewrite is its §5.2 row C18); its R-A text (§1.2) now reads "Every
  node in template contents or in a clone of them has I as its node document (H-T21)"; its §8.2 hand-off says T3
  relies on D-C2 to **D-C7** (edge c4, §9 item 9 updated).
- **P-C26 (H-C16, Q-S10).** The per-wrapper Document installs (`install_document_methods_if_cloned_doc`, DOMParser's
  `dom_parser.rs:305` at `593aa314`) are deleted by C1-pre; every Document wrapper has `Document.prototype` (H-C16). R-C5's
  precedent text follows C1-pre §4.3.
- **P-C27 (§5.3 `tests_html_t2d_protos.rs` "not touched": `/usr/bin/grep -n 'owner' <file>` → `output_form_owner`
  only).** At `c7cce8c8` the same command also hits `create_element_ownerless_fixture` (`:37`, `:38`, …); restated by
  the property (no node-document read: `/usr/bin/grep -c 'ownerDocument\|owner_document(\|node_document(' <file>` → 0).

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
  HTML document, mark newDocument as an HTML document also") always applies: every elidex Document is an HTML document
  (H-C13), so I is one too, by construction (elidex has no document-type field to set). The doc comment says so, and
  that a future XML Document kind (`#11-domparser-xml-real-parsing`) makes this step a copy of the flag. The
  rule is idempotent over the pair: `f(D) = f(I) = I`, so a caller may pass either and gets the same answer. A predicate
  `EcsDom::is_inert_template_document(&self, document: Entity) -> bool` (NEW) reads the marker; the predicate is the
  per-node "template origin" fact slice b's S-2 reads through the node document (§4.6).
- **D-C3 — the establishing steps call the rule once, inside the helper.** `attach_template_contents(template,
  owner)` keeps its name and its single-attach contract; its second argument, a `NewNodeOwner` (C0 draft D-17; H-C1),
  carries **the template's node document** and the helper applies `f` to `owner.document()`. `None` keeps
  today's orphan fragment (a documentless context or an orphan clone). No caller computes I itself. This is C0b's hook
  point (C0 draft D-10, "C1's hook point"): at `c7cce8c8` the strict tier, the tolerant tier and `createElement` pass
  `node_document_for_new_nodes(template)` (H-C2), the template's node document with the §13.4 redirection; the clone
  post-pass passes the operation's document today and each pair's `pair_owner` on C1's head (§4.4). C1 changes no argument of the other three callers; it changes what the
  helper does with it (applies `f`). Through idempotence a template nested in contents gets `f` of its own node document
  (INV-C (a)) whether that is D or I, so the same I.
- **D-C4 — the new-node entry is C0's; C1 guarantees its value inside contents (PM decisions 199 / 207, 223 O-2 /
  227).** C0b added `EcsDom::node_document_for_new_nodes(source)` (H-C3) — `node_document(source)`, except
  that the §13.4 fragment-parse root answers its `FragmentParseRoot`'s `redirect_document` — deleted the strict tree
  builder's method, and migrated every production creation site, both parser tiers included, onto it (C0 draft D-10,
  D-11, D-20, §4.4; H-C4). C1 adds no entry, no entry branch and no creation-site change. C1's obligation is the entry's
  **value** inside template contents:
  - inside template contents `source` is the contents fragment or one of its descendants. The mechanism: D-C3 makes
    the fragment's stamp `f(..)`, C0's link-time adopt (C0 draft D-4, invariant I-U) carries the fragment's stamp to
    every linked descendant, and shadow roots inside contents take their host's node document (C0 draft D-11; DOM §4.9
    *attach a shadow root* step 5), so the entry returns the contents fragment's node document there with no
    template-specific branch, for nested contents too (`f(I) = I`); that value is INV-C's;
  - in document parsing the creation stamp and C0's link-time stamp therefore agree inside contents: no `Adopt` from D
    to I for a parsed node (cell U12, both tiers);
  - in a fragment build no node except the root holds the throwaway and no build link adopts (C0 draft D-8, D-10), so
    a template of the build has the context's node document (inside contents, per INV-C) and `attach_template_contents`
    computes `f(context's document)`; `f` is never computed for the throwaway (D-C7, U4).
  Rev 2's D-C4, §4.3's parser-site list and Q-1 moved to C0 (§10).
- **D-C5 — the template adopting steps inside C0's adopt.** DOM §4.5 *adopt* step 3 walks node's **shadow-including
  inclusive descendants**, which do not include template contents; step 3.4 runs each descendant's *adopting steps*,
  and a template's (HTML §4.12.3) adopt its contents into `f(node's new node document)`. C1 replaces C0's identity edge
  with that: the walk re-homes the root's shadow-including tree to `document` and, for each template it reaches, adopts
  the template's contents into `f(document)`; nested templates resolve to the same I (idempotence). The walk's shape is
  C0's (H-C6): the private `adopt_walk` (`mutation.rs:322`) collects the uncapped `TemplateContentsWalk::Include` walk,
  treats each contents fragment it reaches as a segment boundary, and records a segment `{ fragment, documents:
  Option<(old, new)> }` (`AdoptedContents`) per contents fragment it does not skip (C0 draft D-3 (c), C0c); rev 2's Q-2
  is answered there, and C1 adds no walker variant (C0c's D-19 changes only the `Include` visiting order).
  At `c7cce8c8` every node, contents included, is stamped with the one `document` (C2; §0.3 P-C20). C1's whole change
  is the document of the contents segments: for every node whose innermost contents segment is live, the stamp (with
  its Attr entities), the segment's `new` and the skip test use `f(document)` instead of `document`; the root's own
  segment keeps `document`. `f(document)` comes from the `&mut` rule `f` (creating I on first use), computed on
  `adopt`'s `&mut` path at most once per adopt call and only when the walk reaches a contents fragment (D-C7's "iff").
  **Implementation obligation (per-node segment bookkeeping).** At `c7cce8c8` a segment carries no document
  (`Segment::Live { pair }`, `mutation.rs:688-694`) and the walk determines a node's segment only where it needs the
  skip: a template's segment through the fast path (`:366-370`; a `lookup` only after a skip or a `None` pair) and a
  non-template node's through `lookup` (`:392`) only after some segment was skipped. Every other node is stamped with
  `document` with no segment determination. On C1's head a node's stamp depends on its innermost segment, so C1 must
  determine it for **every** node of a walk that reaches a contents fragment: either the segment records its document
  (root: `document`; a live contents segment: `f(document)`) and each node reads it through `lookup`
  (the `lookup` method of `AdoptSegments`, `:719-724`, → `find_tree_root_composed`, `walkers.rs:432`: O(depth) per node, O(n · depth)
  per walk), or an equivalent that carries the segment down the walk. Which is implementation input; the walk that
  reaches no contents fragment keeps its no-bookkeeping path (`:350`) and its cost.
  C0c's skip test (`mutation.rs:371-373`, C0c memo D-3 (c), "`old` equals `new`") therefore compares the contents' old
  document with the **mapped** `new` = `f(document)`, never with the unmapped target (T27, T28).
  **Notification (owner split).** C0c owns the consumer change: one `MutationEvent::Adopt` per adopt call carries every
  segment's pair, and the custom-element consumer visits contents right after their template (D-19) and gives each
  element its own segment's pair (C0 draft D-6; C0c's cells T18 and U13 pin per-node pairs and order under C2). C1 owns only the
  value of the contents pair (the contents segments' `new` above), with no consumer change (cell T25). Rev 2's Q-3 (consumer mechanism) is answered by C0 draft D-6; C1 does not touch the consumer.
- **D-C6 — the template cloning steps, and one document per clone pair.** The copy's contents are established at the
  copy's creation through D-C3 with the pair's document, and the source contents' children are cloned with **document
  = the copy's contents' node document** (HTML §4.12.3 *cloning steps* step 2), not the operation's. Every later pass
  for a pair inside those children (a nested template's contents; a clonable shadow host's shadow children) uses
  **that pair's document**, not the operation-wide document (`clone.rs:248`); the host and its shadow root follow from the host's
  creation (§4.4 step 3). The worklist
  therefore carries a document per pair; its representation (a third tuple field, or a read of the stamp the cloner
  wrote) is input (§4.4), written on D14's form (shadow children cloned with subtree true).
  *What the per-pair document changes, observably (rev 3.10).* Not the node documents a script reads: each copied
  contents child and each shadow child is linked by `append_child` (`clone.rs:433`, `:298`), which adopts into the
  parent's node document first (`mutation.rs:50` → `adopt_for_insert` `:271` → `adopt_into` `:288`), so a child cloned
  with the operation-wide document D ends in the copy's contents' node document anyway (T10, T24 read the same values). What it changes is
  that such a child is created in D and then adopted into I: one adopt walk and one `MutationEvent::Adopt` (D, I) per
  child, which the spec's clone, creating every node in its document (DOM §4.4 *clone a node* steps 2, 5, 6.7; HTML
  §4.12.3 *cloning steps* step 2), never does. Cell U14 is the guard.
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
  at HEAD; that wider defect is not C1's (§7, scout Q-S6). The exclusion is a list (DocumentFragment, I), so the next
  root kind with no browsing context is admitted by default; the complement form (the root set is the Documents that
  have a browsing context) is the ideal, and a facet of `#11-browsing-context-state-ecs-components` (the positive fact's
  home), appended by record R-8 (PM 613), because that fact is not an ECS fact yet. C1 extends the list by one, so R-8's
  append is a landing condition of C1 (§9 item 10).
- **D-C10 — C0 first (decided: PM decisions 143, 153).** C1's base contains C0. C1 adds no second node-document reader
  and no `owner_document` read in new code: every read in C1's new or changed code is `node_document` (C0) or the
  new-node entry (D-C4); `owner_document` stays the `ownerDocument` getter's "null for a Document" wrapper (C0 draft
  D-1). The count of `git grep -cwP 'owner_document(?=\()' -- <file>` over every file of §5.3 does not grow from
  C1's base to its head (`-w` keeps `validate_owner_document(` out; DoD item 2).

## §1. Shape and verdict

**Axes (§2): 9** — A1 the rule (D-C2), A2 creation paths and the new-node entry (D-C3, D-C4), A3 nested templates, A4
shadow roots in template contents, A5 clone (D-C6), A6 adopt (D-C5, on C0), A7 lifetime (D-C7, D-C8), A8 node-document
consumers (H-C8, H-C9, D-C9), A9 the EH program's contract (K-*, X-W, S-1/S-2, T3's R-A). ≥ 3 intersecting axes, so this
is edge-dense: the PM's classification holds and plan-review is mandatory (CLAUDE.md *Edge-dense work* (b)). It is a
terminal single PR once its plan-review passes (CLAUDE.md *Edge-dense work* (c)) because PM decision 123 makes it a
per-PR plan under the approved umbrella and it changes one fact, the contents' node document (*Split decision* below).

**Why C0 must come first (PM decision 143; the evidence).** At HEAD the C2 approximation and the missing insertion
adoption cancel for the common template patterns; C1 without C0 would break the cancellation:

| pattern | HEAD (`593aa314`: C2, no insertion adoption) | C1 without C0 (rejected) | C1 on C0 (this PR) = spec |
|---|---|---|---|
| `c = tpl.content.cloneNode(true); document.body.append(c)`; a moved node's `ownerDocument` | `document` (the clone stamped `document`, H-C7) | I (the clone stamps I; nothing re-homes it at the append) | `document` (DOM *insert* step 7.1 adopts) |
| same, in a document with `<base href="http://example.com/dir/">`; `<a href="x">` from it, `a.href` | absolute against the document's `<base href>` | `"x"`: the URL does not parse against I's base `about:blank` | absolute against the document's `<base href>` |
| `tpl.innerHTML = '<b>'`; `b.ownerDocument` | `document` | `document` (the fragment parse adopts into the context's document, H-C6; the placement does not adopt) | I |
| `tpl.content.append(document.createElement('div'))`; its `ownerDocument` | `document` | `document` (explicit owner, no adoption) | I |
| `document.body.append(tpl.content)` with a parser-built `<svg>` child; its `ownerDocument` | `document` | I | `document` |

Without C0, rows 1, 2 and 5 are **worse than HEAD** on the most common template idiom (row 1 is §4.12.3's own example),
and rows 3 and 4 leave two answers to one node-document question depending on how the node got there (the decision
surface One-issue-one-way forbids). With C0 on the base every row reaches the spec's value: rows 1, 2, 5 by C0's
adoption at the append, rows 3 and 4 by C0's adoption at the placement into a contents fragment whose node document is I
(C0 draft §4.6, I-U). §6 T9 is the K-0 guard for rows 1–2.

**K-0 lens (umbrella K-0, S-3: no step worse than HEAD).** On C1's base (N1, C0, D14, C1-pre), C1's observable changes are
§5.4's R-C1 to R-C8. Each moves an outcome from C2's to the spec's; none moves it to a third value. Each spec value
is derived from cited steps (§5.4, §6) and is the cell's expected value (PM 607).

**Split decision: one PR.** The rule, the inert document, its lifetime, the establishing, parsing, cloning and adopting
sites form one fact (the contents' node document); landing a subset would leave two answers to it (rows 3 and 4 above,
in reverse). The prerequisite 1000-line split is none (§5.3).

## §2. Coupled invariants

### §2.1 Invariant per axis

| axis | invariant |
|---|---|
| A1 rule | For every Document D, `f(D)` is one Document I carrying `InertTemplateDocument`, the same entity on every call; `f(I) = I`. No site computes a template contents' node document except through `f` (D-C2, D-C3) |
| A2 creation | Template creation establishes the contents fragment (its node document: INV-C (b)). Every node a parser creates takes its owner at creation from C0's new-node entry (C0 draft D-10, D-11), whose value inside template contents is INV-C's, in both tiers, in document, fragment and DOMParser parses (D-C3, D-C4) |
| A3 nesting | A template at any depth inside contents has the node document INV-C gives it, and its own contents `f` of that (`f(I) = I`); there is one I per non-inert Document, never an I of I |
| A4 shadow | A shadow root inside template contents (declarative or `attachShadow`) and its content are in F's tree and report the contents' node document (INV-C (a); C0's host-link stamp, A2's entry); the adopt and clone walks reach them under the contents' document |
| A5 clone | The copy's contents are established as at creation (their node document: INV-C (b)); their children, and every pass for a pair among them, use `f(pair document)`; a clone of contents is in the document it is cloned with (INV-C); a clone of D or of I is a non-inert Document with no link |
| A6 adopt | Adopting a template into X (C0's insertion adopt or `adoptNode`) re-homes the template and its shadow-including descendants to X; contents: see D-C5. Adopting a contents fragment itself (`X.adoptNode(tpl.content)`) re-homes it and its descendants to X, not `f(X)`: it is the walk's root, not a segment (INV-C (c)) |
| A7 lifetime | I is created on first need and destroyed exactly when D is; no fragment parse, strict parse error or clone creates an I for a throwaway document |
| A8 consumers | Every reader of a node's document reads, for a node in template contents, the value INV-C gives it, through `node_document` (the `ownerDocument` getter, the base URL, N1's compile gate, T3's step 3 after C1); no render / layout / hit-test root discovery visits I |
| A9 EH contract | K-0 and S-3 hold; the bind pass still walks no template contents (K-5, K-6, U8 (ii)); S-1's template-contents extension and S-2's branch-A read-null are satisfiable on C1's head (§4.6); T3's R-A premises hold at C1's head in INV-C's conditional form (§9 item 9) |

### §2.2 All pairs (36), with intersection

| pair | intersect? | intersection |
|---|---|---|
| A1×A2 | yes | Four establishing callers pass a node document, the helper maps it (D-C3). A caller that passes the template's own document unmapped is C2 (T1, T3) |
| A1×A3 | yes | Idempotence: a nested template passes D or I and gets I (T4, T5). A rule without the marker check creates I-of-I (U2) |
| A1×A4 | yes | A shadow root in contents takes its host's node document; the rule is never applied to shadow roots (only templates have contents) (T12) |
| A1×A5 | yes | The copy's contents come from `f(pair document)`; for a pair inside copied contents the pair document is the copy's contents' document, an I (INV-C (b)), and `f(I) = I` (T10, T24; U14: no adopt during the clone) |
| A1×A6 | yes | Adopt maps each template's contents to `f(target)`, once per template reached (D-C5; T13, U9) |
| A1×A7 | yes | Lazy creation inside `f`; destruction cascades from D, so `f` never returns a destroyed I while D lives (U3, U4, U7) |
| A1×A8 | yes | Consumers read the result through `node_document`; no consumer calls `f`. One node-document reader (C0), one new-node entry (D-C4), one contents rule (D-C2): three functions for three facts, not three for one |
| A1×A9 | yes | `is_inert_template_document(node_document(n))` is the branch-A template-origin fact (S-2), derived from the node document, so it tracks moves (a node moved out loses it through C0's adoption, and so does every node of a contents fragment adopted out whole, INV-C (c)) |
| A2×A3 | yes | Both tiers call C0's entry with the target parent per node (HTML §13.2.6.1 *create an element for a token* step 6; C0 draft D-11, D-20), whose node document inside outer contents is INV-C's; the inner template's contents get `f` of it, the same I (T4, both tiers) |
| A2×A4 | yes | Declarative shadow content inside contents: the strict DSD path (`attach_declarative_shadow`, `shadow.rs:135`) and the tolerant DSD recursion (`try_attach_declarative_shadow`, `convert.rs:119`) create the shadow content with the entry's value for the shadow root, which C0 stamped with the host's node document (INV-C) (T12; Q-S2). A tolerant **document** parse attaches no declarative shadow root (`#11-tolerant-document-declarative-shadow-roots`), so the tolerant half holds for fragment parses only |
| A2×A5 | yes | The clone post-pass is an establishing caller (D-C3) and site A10 of the new-node census: like every creation path it calls the entry, but on a Document (the operation's, or for a pair inside copied contents the pair's, D-C6; a Document → itself, C0b U11), never on a source node, so a clone's document is the one it is cloned with (DOM §4.4 *clone a single node* "given document"), not its source's (§4.4) |
| A2×A6 | yes | Fragment build: every node under the fragment-parse root holds the context's document from creation (C0 draft D-10), so no build link adopts and no finishing walk runs (D-8); contents fragments are stamped `f(context doc)` at attach (T14 parity, both tiers) |
| A2×A7 | yes | No `f(throwaway)` is ever computed (D-C4), so neither the success path nor the strict rollback (`despawn_subtree(document)`, which reaches contents with `Include`) creates or leaks an I (U4, U5) |
| A2×A8 | yes | DOMParser: `dp`'s templates' contents are in `f(dp)`, a third document, read null (N1) and, after T3, target null, exactly as `dp`'s own nodes (T6) |
| A2×A9 | yes | The pre-bind parse establishes contents (INV-C (b)) before bind; slice a's bind pass still excludes them (U8 (ii) unchanged). Slice b's S-1 extension processes them at creation-equivalent time; C1 gives them a non-bound node document so their raw values read null (§4.6) |
| A3×A4 | yes | A template inside a shadow tree inside contents, and a shadow host inside nested contents: all report the document INV-C gives them (T12 variant) |
| A3×A5 | yes | Deep clone of an outer template clones the inner template inside the copy's contents with the copy's contents' document as pair document (an I, INV-C (b)); the inner copy's contents are `f(I) = I` (T10; U14) |
| A3×A6 | yes | Adopting an outer template into X: contents' pairs and skips per D-C5; the target switches once, at the first contents edge (U9) |
| A3×A7 | no | Nesting creates no further document (A1×A3) |
| A3×A8 | yes | Nested contents read null and (after T3) target null like outer contents, under INV-C (same I) |
| A3×A9 | no | The EH contract does not distinguish depth inside contents |
| A4×A5 | yes | A clonable shadow host inside copied contents: its host, shadow root and shadow children are in the pair's document (the copy's contents' I, INV-C (b)), not the operation's (§4.4 step 3) (T24 reads the values; U14 discriminates the per-pair document, D-C6). D14's subtree-true form is assumed (§4.4) |
| A4×A6 | yes | Adopt's walk is shadow-including under each target: a shadow root inside contents goes to `f(X)`, a shadow root of the template's light descendants to X (U9) |
| A4×A7 | no | Shadow roots create no document |
| A4×A8 | yes | `shadowRoot.ownerDocument` in contents reads INV-C's value; a handler on a shadow-root-direct child in contents reads null under N1 (no Insert fires there, H2, so it is processed only by slice b's creation path) |
| A4×A9 | yes | Slice a's G2 window (shadow-root-direct elements not processed) is unchanged by C1; in contents, the spec read is null either way |
| A5×A6 | yes | `tpl.content.cloneNode(true)` is in the contents' node document (INV-C); appending it into the document adopts it (C0) to D (§1 row 1, T9) |
| A5×A7 | yes | Cloning D creates no I until a template in the copy needs one; the link is not copied (D-C8; T11, U8) |
| A5×A8 | yes | A detached contents clone reads null for raw handlers under INV-C (N1: node document I ≠ bound) and the spec function after the append (adopted to D) |
| A5×A9 | yes | S-1 processes clones at creation (T3 draft D-T10 moves the clone path into T3, after C1); a clone of contents is in the contents' node document (INV-C), so under INV-C its raw values read null until inserted, which is S-2's branch-A outcome. The umbrella's §9 N-2 problem (no per-node fact) does not arise: the fact is the node document |
| A6×A7 | yes | Adopting all templates out of D's contents leaves I alive (D lives); an adopt never targets a throwaway's I (none exists, A2×A7) |
| A6×A8 | yes | After adoption every consumer reads the new document through `node_document` (C0) |
| A6×A9 | yes | C0's adoption precedes the Insert event (C0 draft D-4), so slice a's Insert arm (T-a2) and N1's gate see the adopted document; a node placed into contents is processed at T-a2 with the contents' node document and, under INV-C, reads null (W-K5's template half closes; R-3) |
| A7×A8 | yes | A node outliving its I reads `ownerDocument` null through `node_document`'s dangling guard; I is never a render / layout / hit-test root (D-C9; U6) |
| A7×A9 | no | The EH contract never reads I's lifetime |
| A8×A9 | yes | N1's node clause reads INV-C's value for contents: umbrella C14 inverts (no run, `null`; T17). The base's target rule ignores the document, so C18 is unchanged at C1 (T18) and T3 rewrites it; T3's R-A target cells (T-17, EW-13, EW-14, XW-8) rest on C1's facts (§9 item 9) |

Count: 36 pairs; 32 intersect, 4 do not (A3×A7, A3×A9, A4×A7, A7×A9). Rev 1's fifth non-intersecting pair, A2×A5,
intersects in rev 2 (the clone post-pass is a creation site of the new-node census). Re-count:
`awk '/^### §2.2/{f=1} /^## §3/{f=0} f && /^\| A[0-9]×A[0-9]/' <memo> | awk -F'|' '{print $3}' | sort | uniq -c`.

## §3. Spec coverage map

Lookups, **all re-run 2026-11-10 (rev 3.8)** with `.claude/tools/webref`; a pair or step that changed since its earlier
lookup is marked "(was …)" and listed in §10.5. Headings: `heading html 4.12.3` (The template element), `heading html
2.4.3` (Document base URLs), `heading html 2.6.1` (Reflecting content attributes in IDL attributes), `heading html
4.6.3` (API for hyperlink elements), `heading html 13.2.6.1` (Creating and inserting nodes), `heading html 13.2.6.4.4`
(The "in head" insertion mode), `heading html 13.4` (Parsing HTML fragments), `heading html 8.5.1` (The DOMParser
interface), `heading html 8.5.5` (The innerHTML property; was 8.5.4, now HTML serialization methods), `heading html
8.5.8` (The createContextualFragment() method; was 8.5.7, now The insertAdjacentHTML() method), `heading html 8.1.8.1`
(Event handlers), `heading dom 4.2.3` (Mutation algorithms), `heading dom 4.4` (Interface Node), `heading dom 4.5`
(Interface Document), `heading dom 4.8` (Interface ShadowRoot), `heading dom 4.9` (Interface Element), `heading url
4.4` (URL parsing). Dfns: `dfn html` 'appropriate template contents owner document', 'associated inert template
document', 'template contents' (§4.12.3); 'document base URL', 'fallback base URL' (§2.4.3); 'reinitialize url' (§4.6.3); 'reflect'
(§2.6.1); 'appropriate place for inserting a node', 'adjusted insertion location', 'create an element for a token'
(§13.2.6.1); 'HTML fragment parsing algorithm', 'initialize an HTML fragment parser' (§13.4); 'insertion target
redirection map', 'fragment parser abort requested', 'parser sanitizer configuration' (§13.2.4.5 Other parsing state
flags); 'createContextualFragment' (§8.5.8); `dfn url 'basic URL parser'` (§4.4); `dfn dom` 'adopt' (§4.5), 'URL'
for Document (§4.5), 'clone a node', 'clone a single node', 'node document' (§4.4), 'insert', 'replace' (§4.2.3),
'attach a shadow root' (§4.9), 'shadow-including inclusive descendant' (§4.8). Steps (`body`): `html
the-template-element` (establishing steps 1–3; *appropriate template contents owner document* 1, 1.1, 1.1.1–1.1.3,
1.2, 2; adopting steps 1–2; cloning steps 1–2); `dom concept-node-adopt` (steps 1–3; 3.1 node document; 3.2 shadow
root registry; 3.3.1 attributes' node document; 3.3.2 element registry; 3.3.3 `adoptedCallback` « oldDocument,
document »; 3.4 adopting steps, inside the per-descendant loop) and `dom dom-document-adoptnode` (steps 1–4); `dom
concept-node-clone` (steps 2, 3, 5, 6.7 "subtree set to true"); `dom clone-a-single-node` (step 5's Document branch
5.1–5.3; steps 7–8); `dom concept-node-insert` (step 7.1 "Adopt node into parent's node document"); `dom
concept-node-replace` (step 6); `dom concept-attach-a-shadow-root` (step 5); `dom concept-document-url` (a document's
URL is "about:blank" unless stated otherwise); `html create-an-element-for-the-token` (step 6 "Let document be
targetParent's node document", was step 3 "intendedParent's node document"; step 10 "If sanitizerAction is not
"Keep" or isDropped is true, then set document to document's appropriate template contents owner document", new);
`html insert-a-character` / `insert-a-comment` (a text / comment node "given targetParent's node document");
`html adjusted-insertion-location` (steps 1–7) and `html appropriate-place-for-inserting-a-node` (a template target
→ "(targetParent's template contents, null)"); `html html-fragment-parsing-algorithm` (steps 2, 6, 7; was steps 14,
15, 22 of a pre-rewrite algorithm) and `html initialize-an-html-fragment-parser` (steps 12–13 the root in the
temporary document, 15 "Set parser's insertion target redirection map[root] to insertionLocation");
`html document-base-url` (steps 1–3), `html fallback-base-url` (steps 1–2); `html dom-domparser-parsefromstring` (step 3, rev 3.10); `dfn html 'get the focusable area'` (§6.6.4 Processing model) and `body html get-the-focusable-area` (the delegates-focus shadow host branch, steps 1–3; rev 3.10); `html getting-the-current-value-of-the-event-handler` (step 3.2);
`html determining-the-target-of-an-event-handler` (step 3); `html dom-range-createcontextualfragment` (steps 1, 6);
`html dom-hyperlink-href` (step 4); `html reflect` (the URL getter, steps 3.3, 4).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.12.3 The template element | establishing the template contents, steps 1–3 | 1 document = node document's appropriate template contents owner document; 2 fragment with that node document and host = template; 3 link | `EcsDom::attach_template_contents` `crates/core/elidex-ecs/src/dom/mod.rs:614` (D-C3); step 2's *host* is §7 | ✗ (host not modelled) | yes |
| WHATWG HTML §4.12.3 The template element | *appropriate template contents owner document*, steps 1–2 | 1 not created by this algorithm → get-or-create the associated inert template document (1.1.1 create, browsing context null; 1.1.2 HTML flag; 1.1.3 link), set document to it; 2 return | `appropriate_template_contents_owner_document` (NEW) in `crates/core/elidex-ecs/src/dom/template_owner.rs` (NEW) (D-C2); 1.1.2 holds by construction (every elidex Document is HTML, H-C13) | ✓ | no |
| WHATWG HTML §4.12.3 The template element | template *adopting steps*, steps 1–2 | document = node's new node document's appropriate owner; adopt contents into it | C0's adopt walk (`adopt_walk`, private, `crates/core/elidex-ecs/src/dom/tree/mutation.rs:322`, through `adopt_into` `:288`; H-C6): the document of each live contents segment — its stamps, its `new`, its skip test (D-C5); C0's `Adopt` consumer unchanged by C1 | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | template *cloning steps* (node, copy, subtree), steps 1–2 | subtree false → return; else clone each contents child with document = copy's contents' node document, subtree true, parent = copy's contents | `replicate_template_contents` `crates/dom/elidex-dom-api/src/node_methods/clone.rs:412` (D-C6) | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | `content` getter | return the template contents | `crates/script/elidex-js/src/vm/host/html_template_proto.rs:106` (unchanged) | ✓ | no |
| WHATWG DOM §4.5 Interface Document | *adopt* steps 1–3 | 3 document ≠ oldDocument → for each shadow-including inclusive descendant: 3.1 set node document; 3.3.1 set each attribute's node document; 3.3.3 `adoptedCallback`; 3.4 adopting steps; 3.2 / 3.3.2 custom element registries (not modelled, §7) | C0b's `EcsDom::adopt` `mutation.rs:252` and its walk (C0 D-2, D-3; H-C6): 3.1 and 3.3.1 are `stamp_node_and_attrs` (`mutation.rs:402`), whose document C1 changes for nodes in live contents segments; C0c's `MutationEvent::Adopt` consumer (C0 D-6) is 3.3.3; C1 edits only the document of the live contents segments, hence their stamps and `new` (D-C5) | ✗ | yes |
| WHATWG DOM §4.2.3 Mutation algorithms | *insert* step 7.1; *replace* step 6 | adopt node into parent's node document | C0 (C0 D-4); not touched by C1 | ✗ | yes |
| WHATWG DOM §4.4 Interface Node | *clone a node* steps 2, 3, 5, 6.7 | 2 copy created in document (runs the establishing steps); 3 cloning steps; 5 children with the same document; 6.7 shadow children with the same document, subtree true | `clone_node_with_shadow_honor` `clone.rs:235` (D-C6, on D14), `clone_subtree` `tree_clone.rs:179` (unchanged by C1; its document is a `NewNodeOwner`, C0b D-11) | ✓ | yes |
| WHATWG DOM §4.4 Interface Node | *node document*; `ownerDocument` getter | a node's node document; a document's is itself; getter null for a document | C0b's `EcsDom::node_document` `crates/core/elidex-ecs/src/dom/node_document.rs:89` and `owner_document` `mod.rs:871`; getter `crates/dom/elidex-dom-api/src/node_methods/core.rs:264` (unchanged by C1; new values) | ✓ | no |
| WHATWG HTML §13.2.6.1 Creating and inserting nodes | *appropriate place for inserting a node* / *adjusted insertion location*; *create an element for a token* steps 6, 10; text and comment insertion | a template target → its template contents; 6 document = targetParent's node document; 10 sanitizer action not "Keep" or isDropped → document's appropriate template contents owner document; text / comment = targetParent's node document | C0b's `EcsDom::node_document_for_new_nodes` `node_document.rs:66` and its parser call sites (C0 draft D-10, D-11, §4.4; H-C3, H-C4); C1 changes no site, only the value inside contents (D-C4). Step 10 is not modelled because it is unreachable: no sanitizer and no fragment-parser abort flag in either parser (`git grep -n -i 'sanitiz\|abort_requested' -- crates/dom/elidex-html-parser/src crates/dom/elidex-html-parser-strict/src` → 0), and isDropped's referenceChild half is structurally false in both tiers — strict has no referenceChild (`appropriate_place` returns a parent only, `tree_builder/insert.rs:30-38`; create and append are one step, `:61-66`, `:106-117`; no foster parenting / adoption agency, `lib.rs:4-5`), tolerant creates nodes only after html5ever has finished the `RcDom` (`lib.rs:49-50`, `convert.rs:1-4`, `:244`) (Q-S13, closed rev 3.8.1) | ✗ (step 10, unreachable) | yes |
| WHATWG HTML §13.2.6.4.4 The "in head" insertion mode | start tag "template", the declarative branch ("Keep") | the declarative shadow root attaches when the parser's allow-declarative-shadow-roots flag is true, inside template contents too | strict `attach_declarative_shadow` `tree_builder/shadow.rs:135`, `attach_ordinary_template_contents` `:122` (D-C3); tolerant `try_attach_declarative_shadow` `convert.rs:119` | ✗ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | *HTML fragment parsing algorithm* steps 2, 6, 7 (sanitizerConfig null); *initialize an HTML fragment parser* steps 12–13, 15 | 2 the fragment is created in target's node document; init 12–13 the root `html` element in the temporary document; init 15 the insertion target redirection map sends insertions at the root to the fragment, so nodes are created in its node document (step 6 of *create an element for a token*) and templates parsed in the build get the context's inert document (D-C4); 6 adopt the fragment into target's node document; 7 return it | `begin_detached_fragment` `teardown.rs:286` (`FragmentParseRoot` on the root `:301-304`, C0 draft D-10), `finish_detached_fragment` `teardown.rs:331` (no re-home, D-8); the tolerant tier's own choice is gone (D-20; H-C4); C1: D-C4's value | ✗ | yes |
| WHATWG HTML §8.5.5 The innerHTML property | setter on a template: the target is its template contents | parsed nodes replace the contents' children (inserted, so adopted into the contents' node document by C0; INV-C (a)) | `crates/script/elidex-script-session/src/mutation/html_fragment.rs` (unchanged; C0 adopts) | ✗ | yes |
| WHATWG HTML §8.5.1 The DOMParser interface | `parseFromString` steps 3, 4 (`text/html`) | 3 the new document's URL is relevantDocument's URL; it is "not created by this algorithm": its templates get its own inert document | `parse_into_inert_document` `crates/dom/elidex-form/src/inert_document.rs:110` (unchanged; values change via D-C3); step 3's URL reaches `DocumentBaseUrl` through `initialize_base_url_for_document` (`inert_document.rs:200`; R-C2, T29) | ✗ | yes |
| WHATWG HTML §2.4.3 Document base URLs | *document base URL* steps 1–3; *fallback base URL* steps 1–2 | I has no `base` element with `href` (step 1) and no base URL override (step 2) → step 3, its fallback base URL: I's URL is `about:blank` and its about base URL is null → step 2, `about:blank` | `DocumentBaseUrl(about_blank_url())` from `create_document_node` `mod.rs:474`; read by `href_accessor.rs:51` through `node_document_base_url` (`document_base.rs:68`) (R-C2, T20) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *get the current value of the event handler* step 3.2; *determine the target of an event handler* step 3 | raw value in a document whose browsing context is null → null; node document not an active document → null target | N1's compile gate (C0's `node_document` form of clause (b), `crates/script/elidex-script-session/src/scripting.rs:83-84`; doc `:64-66` rewritten); T3 (after C1) | ✗ | yes |
| WHATWG DOM §4.9 Interface Element | *attach a shadow root* step 5 | the shadow root is created given the host's node document | C0b's `attach_shadow_with_init` → `node_document_for_new_nodes(host)` (`crates/core/elidex-ecs/src/dom/shadow.rs:194`; C0b D-11); C1 changes no site, only the host's value inside contents (D-C4, §4.4 step 3) | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *shadow-including inclusive descendant* | adopt's walk order (contents excluded; reached through step 3.4) | `for_each_uncapped_shadow_inclusive` `crates/core/elidex-ecs/src/dom/tree/walkers.rs:111` (C0c reorders `Include`, C0 draft D-19; C1 leaves it unchanged) | ✓ | no |

### §3.1 User-input touch audit

Rows marked *yes* carry author markup or script-constructed trees. C1 parses no new input and stores no string: the
two components hold an `Entity` and nothing. What changes is which Document a node of author content is attributed
to, and with it four reads: the `ownerDocument` getter (a new Document wrapper becomes reachable from script: I, whose
browsing context is null; R-C5, edge c-pre), URL resolution (relative URLs in template contents no longer resolve
against the document's `<base href>`, or in a DOMParser document against the page URL, R-C2, T20, T29), the handler gates (raw handlers in template contents read null and do not run under INV-C: the
spec's outcome, more restrictive than HEAD), and `adoptedCallback`'s arguments for custom elements in contents (R-C8,
T25). No path delivers author content to a document other than one it was parsed into, cloned into or adopted into.

## §4. Mechanism

### §4.1 Home / kind

- The rule, the predicate and the lifetime hook live in **`elidex-ecs`**
  (`crates/core/elidex-ecs/src/dom/template_owner.rs` (NEW), an `impl EcsDom` block): both parser crates and
  `elidex-dom-api` call them, and the parser crates cannot depend on `elidex-dom-api` (umbrella §9 N-1's crate-graph
  evidence). C0's new-node entry lives in `elidex-ecs` for the same reason (C0 draft D-10).
  `elidex-ecs` is engine-independent, so the Layering mandate holds. C1 adds no VM code: the VM wrapper work is
  C1-pre's (edge c-pre; H-C16) — I's wrapper gets `Document.prototype` at its first allocation (C1-pre D-1) and its
  members answer from I's entity (C1-pre D-4 to D-8). The URL pair is neither's: `#11-document-base-url` (§0.3 P-C17;
  Q-5 decided (a), PM 604; §8 R-11).
- Components (D-C1) in `crates/core/elidex-ecs/src/components.rs` beside `TemplateContents` (`:366`).
- Realm: §4.12.3 *appropriate template contents owner document* step 1.1.1 creates I "given document's relevant realm". elidex has one realm per `EcsDom` (T3 draft
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
entry's value inside template contents is INV-C's, because D-C3 stamps the contents fragment with `f(..)` (D-C4). Rev 3's constraint
on C0's Q-2 (a tolerant value threaded from the template's level, as `convert.rs:284-286` did at `593aa314`, would stamp
contents' children with D) is met by construction: Q-2 is decided per node and the threaded owner is deleted (C0 draft
D-20). The tolerant legs of T3, T12, T19 and U12 still kill that form (§6.3).

### §4.4 Clone (D-C6), on D14

On C1's base (`c7cce8c8`) D14 has rewritten pass 4's child loop to clone shadow children with subtree **true** at every
pair (`clone.rs:295`; DOM §4.4 *clone a node* step 6.7; P-C12), and pass 4 has no hand stamp (C0b D-18, RC-2a): the host carries the
document its pair was cloned with, and the shadow root takes its host's at attach (`shadow.rs:194`). Today one document
is threaded to every pass (`document`, `clone.rs:240`, re-read from the clone `:248`; H-C7). C1 then:
1. gives each worklist pair the `NewNodeOwner` its copy was made with (`pair_owner`): the root pair the operation's
   `document` (`clone.rs:248`), every pair produced under a pair P the owner P's children were cloned with; an
   ownerless pair carries the entry's `None` (C0b D-11);
2. in pass 3, `replicate_template_contents(s, d, …, pair_owner, …)` (`clone.rs:272`): `new_fragment =
   attach_template_contents(d, pair_owner)` (`:427`; mapped inside, D-C3), then clones each source contents child with
   `node_document_for_new_nodes(new_fragment)` (= `f` of the pair's document; `clone_subtree`'s override is a
   `NewNodeOwner`, §11 Q-4) (`:432`) and records those pairs with that owner;
3. in pass 4, clones the shadow children with `pair_owner` (`clone.rs:295`). The host and the shadow root
   need no step: the host was made with `pair_owner` by its pair's clone (step 2 inside contents), and the shadow root takes the host's document at
   attach (`attach_shadow_with_init` → `node_document_for_new_nodes(host)`, C0b D-11, H-b3).
Whether the pair document is a third tuple field or read back from the stamp the cloner wrote at spawn
(`tree_clone.rs:367-368`) is implementation input: every copy is stamped at creation at `c7cce8c8` (H-C7), and the
post-pass already reads its root's document back that way (`node_document_for_new_nodes(cloned)`, `clone.rs:248`), so
`node_document_for_new_nodes(d)` per pair is available (Q-S12's second half). Edge c-RC2a (§8) records the rebase.
§4.4's per-pair document (D-C6, cell T24) is written on RC-2a's landed shape (#596), re-read at `c7cce8c8`: the
`MutationEvent::CloneCreated { root, scope }` event (`crates/core/elidex-ecs/src/dom/mutation_event.rs:375`) fires once
after the worklist drains (`clone.rs:301-308`) and carries no document, and neither consumer's clone pass reads a node
document (`git grep -n 'node_document\|owner_document' -- crates/dom/elidex-custom-elements/src/upgrade.rs
crates/dom/elidex-form/src/clone.rs` → one hit, the inline test `upgrade.rs:469`).

### §4.5 L3's Range creation sites (A10–A12) and the rest of the census

- **A10** (template clone; `#11-template-contents-owner-document`'s scope) is §4.4.
- **A11 / A12** (L3 RC-4's body context element and result fragment of `createContextualFragment`, planned to read the
  new-node entry): the engine-independent site is `create_contextual_fragment` of `impl RangeState`
  (`crates/dom/elidex-dom-api/src/range/mutation.rs:648`, a stub returning `None` at `c7cce8c8`), where RC-4 writes the
  algorithm and calls the entry; the VM native `native_range_create_contextual_fragment`
  (`crates/script/elidex-js/src/vm/host/range_proto_mutation.rs:250`) throws `NotSupportedError` itself at `c7cce8c8`
  and does not call it (`:255-261`); in RC-4's planned shape it only marshals to it (Layering mandate):
  C1's obligation is that the entry returns the right document for a Range whose start node is in template contents:
  the start node's node document (HTML §8.5.8 *createContextualFragment()* step 6: "set element to the
  result of creating an element given node's node document, "body", and the HTML namespace", node = this's start node,
  step 1), which is INV-C's value.
  That holds by A2 / A5 / A6 with no Range-specific branch; unit cell U11 pins the entry's value; RC-4's own cells pin
  the end-to-end shape. A11 / A12 are the residual of `#11-ownerless-node-creation`, which C1 closes; they are L3
  RC-4's sites in the Range program, not a slot facet (record R-4, §9 item 4).
- **A1–A9** and the dom-api creation sites (`element/tree.rs:435` `insertAdjacentText`,
  `node_methods/text_content.rs:119`, `char_data/split_text.rs:89`, `:144`, `cssom_sheet.rs:213`,
  `element/select.rs:326`; `createElement`'s own element) were `#11-ownerless-node-creation`'s census (C0 / L3).
  C0b migrated them: they read the entry, `node_document_for_new_nodes(..)` (§0.3 P-C23; ledger landing note of #608),
  so no live defect is left there; for a receiver in contents their values are INV-C's, through it; C1 changes none of
  them. C1 lands the slot's last facet (template contents) and closes it (§9 item 4).

### §4.6 What C1 provides to slice b (S-1's template-contents extension, S-2 branch A) and to T3

1. **INV-C — the contents-document invariant (rev 3.9: the one statement; every other site of this memo cites it by
   this ID and does not restate it).** Let F be a template's contents fragment and "F's tree" its descendants with their
   shadow-including descendants (the contents of a nested template are that template's own F).
   (a) For an F that has a node document, every node of F's tree has F's node document: created there through the
   new-node entry (D-C4), placed there by C0's insertion adoption (DOM §4.2.3 *insert* step 7.1), or cloned there with
   the document of its pair (D-C6). (rev 3.10.1: an F with no node document, (b)'s ownerless case, is outside (a):
   `adopt_for_insert` gets target `None` and `adopt_into` returns without a walk,
   `crates/core/elidex-ecs/src/dom/tree/mutation.rs:271-274`, `:288-292` at `c7cce8c8`, so a node appended into that F
   keeps its stamp — elidex-only state with no spec counterpart, reachable only from fixtures or a despawned
   fragment-parse context.)
   (b) F's node document is `f(the template's node document)` from F's establishment (D-C3) and after every adopt that
   reaches F as a segment of its template (D-C5), so it is an inert template document. A template with no node
   document (D-C3's `None`: a documentless context or an orphan clone) gets an orphan F with no node document, and `f`
   is not applied (rev 3.10).
   (c) It is another document X only after a script adopts F itself into X (`X.adoptNode(tpl.content)`: F is the
   walk's root, not a segment; spec-correct; T23 route (v), T28, C0c U21), and stays X until an adopt of the template
   into a document other than its own reaches F as a segment again (D-C5).
   Hence: **a node in template contents has node document I = `f(D)` while its contents fragment's node document is
   I** — conditional, not universal. A clone of contents takes the document it is cloned with (DOM §4.4 *clone a node*),
   so `tpl.content.cloneNode(true)` is in F's node document, I under INV-C. The condition holds from establishment
   until a script adopts F out, so it holds before any script can observe a node. Slice b needs nothing else to make
   its creation-time raw values read null under INV-C (N1's node clause: `node_document(n) = I ≠ bound`), and T3 needs
   nothing else to make Window-delegating pairs target null there (step 3: I is not the active document). For a
   contents fragment adopted into the bound document, the spec outcome is the bound one (raw values compile, T23 (v));
   slice b's and T3's plans read the node document and cite INV-C, not "contents means I" (edges c4, c5).
2. **The template-origin fact.** `is_inert_template_document(node_document(n))` (NEW). Carried by a clone and a clone of
   a clone (A5), lost by a node moved out of contents (C0 adopts it): S-2's "tracks the node's spec node document, not
   its history".
3. **Pre-bind template contents.** S-1 extends to them in branch A; the walk that reaches them is the existing uncapped
   walker with template contents included (H-C15's `Exclude` stays for slice a's K-5 / K-6). C1 adds no walk.
4. **No insertion fallback.** C1 adds no processing at insertion (slice a's Insert arm is unchanged), so S-4 is not
   weakened.

### §4.7 What C1 takes from C0 (C0 draft §4.6; all landed, checked at `c7cce8c8`)

| C1 needs | C0 draft's answer (PR) | C1's use |
|---|---|---|
| one reader of a node's node document | `EcsDom::node_document` (C0b, H-C5) (D-1); `owner_document` = getter wrapper | every read in C1's code (D-C10) |
| every insertion adopts, before `Insert` | D-4 at the three primitives; D-5 for replace-with-fragment (C0b) | §1 rows 1–5; T7, T8, T9, T13 |
| a contents fragment's node document is the parent document for nodes placed into it | `node_document(fragment)` = its stamp (C0b) | T7, T8 |
| the one adopt walk to edit | D-3: `adopt` / `adopt_into` and the private `adopt_walk` in `mutation.rs` (C0b; no `adopt_subtree`, §0.3 P-C20), `Include` walk, contents fragments as segment boundaries; segments (D-3 (c), C0c) | see D-C5 |
| one adoption notification | D-6, D-19 (C0c): one `MutationEvent::Adopt` per adopt call with per-segment (old, new) pairs; consumer in spec order | D-C5 sets the segment's `new`; no consumer change (T25) |
| one new-node owner entry, every creation site on it | D-10, D-11, D-17, D-20 (C0b); hook = `attach_template_contents`'s second argument (`NewNodeOwner`) | D-C3 applies `f` in the helper; D-C4's value inside contents |
| no `f(throwaway)` | D-8, D-10 (C0b): no build node but the root holds the throwaway; no build link adopts | A2×A6, A2×A7 |

### §4.8 ECS-native and Layering checks

- Representation: two components, no side-store (D-C1). Lifetime by despawn cascade, not refcount or sweep (D-C7).
- One rule (D-C2), one establishing helper (D-C3), one new-node entry (C0's, D-C4), one adopt walk (D-C5, C0's), one
  reader (C0's `node_document`): §5.1's populations converge on them.
- VM: C1 adds no VM code; the VM wrapper work is C1-pre's (§4.1, edge c-pre); `html_template_proto.rs` keeps
  marshalling the link (`template_get_content`, `:88`, reads it at `:106`).

### §4.9 Docstring citation table (Axis 4)

| symbol | citation |
|---|---|
| `appropriate_template_contents_owner_document` (NEW) | HTML §4.12.3 *appropriate template contents owner document*, steps 1–2 (1.1.2 holds by construction: every elidex Document is HTML, H-C13) |
| `AssociatedInertTemplateDocument` (NEW) | HTML §4.12.3 *associated inert template document* |
| `InertTemplateDocument` (NEW) | HTML §4.12.3 ("a Document created by this algorithm") |
| `is_inert_template_document` (NEW) | HTML §4.12.3 (the same fact) |
| `attach_template_contents` (doc rewrite) | HTML §4.12.3 establishing steps 1–3 (step 2's host not modelled, §7) |
| C0's adopt walk (doc rewrite of the contents edge) | DOM §4.5 *adopt* steps 3.1, 3.3.1, 3.3.3, 3.4 (3.2 / 3.3.2 custom element registries not modelled); HTML §4.12.3 template *adopting steps* |
| `replicate_template_contents` (doc rewrite) | HTML §4.12.3 template *cloning steps* step 2 |
| `clone_node_with_shadow_honor` (pass 4's per-pair owner, D-C6; doc rewrite) | DOM §4.4 *clone a node* step 6.7 (shadow children cloned with the same document, subtree true); HTML §4.12.3 template *cloning steps* step 2 (a pair inside copied contents takes the copy's contents' node document) |
| `TemplateContents` doc (`crates/core/elidex-ecs/src/components.rs:356-361`, its adopt sentence) | DOM §4.5 *adopt* step 3.4 (adopting steps); HTML §4.12.3 template *adopting steps* steps 1–2 (contents go to `f(new document)`, not "with the template") |
| `AdoptedContents` doc (`crates/core/elidex-ecs/src/dom/mutation_event.rs:60-79`; its `new` becomes `f(document)`) | HTML §4.12.3 template *adopting steps* steps 1–2; DOM §4.5 *adopt* step 3.3.3 (« oldDocument, document ») |
| N1 gate doc `crates/script/elidex-script-session/src/scripting.rs:64-66` (the template caveat removed, R-5) | HTML §8.1.8.1 *get the current value of the event handler* step 3.2; HTML §4.12.3 *appropriate template contents owner document* (contents' node document I under INV-C) |
| `destroy_entity`'s Document cascade (D-C7) | no spec counterpart (the spec's lifetime is GC) |
| clone-policy rows of both components (D-C8) | DOM §4.4 *clone a single node* step 5's Document branch, 5.1–5.3 (the copied fields do not include the associated inert template document) |
| `create_document_node` doc (`crates/core/elidex-ecs/src/dom/mod.rs:464-473`; its inert-template-document sentence, §5.3, rev 3.10) | HTML §2.4.3 *document base URL* steps 1–3 → *fallback base URL* steps 1–2 (I's URL is DOM §4.5's default "about:blank" and its about base URL is null, so `about:blank` while I has no `<base href>`; a script-appended `<base>` changes it, step 1) |
| `create_document_node` doc, the throwaway document (`mod.rs:469-470`, "(step 4)" → step 1; §5.3 *Stale cites*, rev 3.10) | HTML §13.4 *HTML fragment parsing algorithm* step 1 ("Let document be a Document node whose type is "html"") |
| `root_entities`' exclusion (D-C9) | no spec counterpart (root discovery is elidex's); its reason is HTML §4.12.3's note "so that they aren't in a browsing context and thus remain inert" |

## §5. Populations, touched files, named risks

### §5.1 Populations (re-runnable)

Production filter for every census (rev 3.7, by the property "compiled outside tests"; §0.3 P-C21): the path filter
`NT='/tests/\|tests\.rs\|/tests_\|test_helpers'` plus every hit at or after its file's first inline test module (a
`#[cfg(test)]` line directly followed by `mod <name> {`; an out-of-line `mod tests;` does not cut):
```sh
NT='/tests/\|tests\.rs\|/tests_\|test_helpers'
prod() { /usr/bin/grep -v "$NT" | while IFS=: read -r f l rest; do
  c=$(awk '/^#\[cfg\(test\)\]/{t=NR; next} t && NR==t+1 && /^mod [A-Za-z0-9_]+ *\{/{print t; exit} {t=0}' "$f")
  if [ -z "$c" ] || [ "$l" -lt "$c" ]; then echo "$f:$l:$rest"; fi; done; }
```
Complement: a hit inside an inline test module whose `#[cfg(test)]` is not on the line directly above `mod … {` is
kept (over-inclusion, visible in the output). Controls at `c7cce8c8`: P-1 drops `upgrade.rs:469` and `clone.rs:268`
(elidex-form); H-C11 keeps `elidex-render/src/builder/mod.rs:479`, `:995` (below an out-of-line `mod tests;`).
All outputs below are at `c7cce8c8`, run 2026-11-10.
- **P-1 establishing sites** (H-C2): `git grep -nF 'attach_template_contents(' -- 'crates/*.rs' | prod` → definition
  `mod.rs:614` + 4 callers (`crates/dom/elidex-dom-api/src/document.rs:270`, `node_methods/clone.rs:427`, strict `shadow.rs:124`,
  tolerant `convert.rs:292`). All four keep their call and pass the template's node document (D-C3; the clone
  post-pass its `pair_owner`, §4.4).
- **P-2 adopt** (property: an entry into the one adopt walk): `git grep -n 'adopt_walk(\|adopt_into(\|adopt_for_insert(\|\.adopt(' -- 'crates/*.rs' | prod`
  → 11: `mutation.rs:50`, `:127`, `:180` (the insertion primitives → `adopt_for_insert`), `:261` (`adopt` →
  `adopt_into`), `:271` / `:273` (`adopt_for_insert`'s definition and its `adopt_into` call), `:288` (`adopt_into`),
  `:297` (its `adopt_walk` call), `:322` (`adopt_walk`); `elidex-script-session/src/mutation/child_list.rs:394`
  (`adoptNode`'s `apply_adopt_node`), `:455` (*replace* with a fragment). One walk; C1 edits it only (D-C5).
- **P-3 where a node's document is written** (property: every write of `AssociatedDocument`; §0.3 P-C22):
  `git grep -n 'set_associated_document(\|validate_owner_document(' -- 'crates/*.rs' | prod` → spawn-time
  `node_document.rs:110` (every creator through `spawn_node`) and `tree_clone.rs:367` (the cloner); the adopt walk
  `mutation.rs:403`, `:410`; definitions `mod.rs:820`, `:851`. `AssociatedDocument` is `pub(crate)` (`components.rs:585`),
  so no other crate writes it. C1 adds no writer: the clone's per-pair document is the `NewNodeOwner` it passes (D-C6),
  the adopt walk's is D-C5's `f(document)`.
- **P-4 parser owner decisions** and **P-5 parser creation sites**: **moved to C0 (rev 3; verified 2026-11-10)**. They are C0's creation
  census and its ratchet (C0 draft D-11, §4.4; C0b memo §5.1 P-3, P-8; PM decisions 223 O-2 / 227). C1 neither adds nor
  changes a parser creation site. Base checks at `c7cce8c8` (verified 2026-11-10): no `owner_document` in either parser crate's
  production code (rev 3.10.2, run at `8344af2b` 2026-11-11, whose code equals `c7cce8c8`'s: `git grep -nw 'owner_document' 8344af2b --
  crates/dom/elidex-html-parser crates/dom/elidex-html-parser-strict ':(exclude,glob)**/tests/**' ':(exclude,glob)**/tests*.rs'
  ':(exclude,glob)**/test_helpers*.rs'` → 0; control, the same command without the three excludes → 7, all in test files:
  `elidex-html-parser/src/tests.rs:559`, `:564`, `:604`, `:614`, `:800` and
  `elidex-html-parser-strict/src/tree_builder/tests_fragment.rs:413`, `:425`. Rev ≤ 3.10.1 printed the unfiltered command
  with "→ 0"); no
  production ownerless creation (by construction: `NewNodeOwner`'s only constructors outside the entry are
  `cfg(any(test, feature = "fixtures"))`, `node_document.rs:26-38`). Re-run at the cut if main moves (§9 item 1).
- **P-6 node-document consumers** (values change for nodes in contents, code unchanged by C1; property: a call that
  reads a node's node document): `git grep -nwP '(node_document|owner_document)(?=\()' -- 'crates/*.rs' | prod | /usr/bin/grep -v ':[0-9]*:\s*//'`
  → 16 lines: definitions `node_document.rs:89`, `mod.rs:871` and `owner_document`'s delegation `mod.rs:875`; the
  entry `node_document.rs:75`; the adopt walk `mutation.rs:296`, `:371`; the fragment root `teardown.rs:301`;
  `document_base.rs:69` (href / base URL, H-C8), `:301`, `:345`, `:378` (`BaseUrlMaintainer` arms); `focus/delegate.rs:86`;
  the getter `node_methods/core.rs:264`; `selection.rs:617`; `child_list.rs:454`; N1's gate `scripting.rs:83`. The
  production `owner_document(` callers are the getter and `selection.rs:617` only (C0b's P-3 result, re-found).
  Scout Q-S4's classification (2026-10-31) found no reader whose new value for a node in contents is not the spec's
  except `documentURL` on I (now `#11-document-base-url`'s, §0.3 P-C17; its cells are §8 R-11's, not C1's), href paths' `document_root()` fallback on I
  (gone: `node_document_base_url` answers `about:blank` for a node without a node document, `document_base.rs:68-71`)
  and `focus/delegate.rs:86` (Q-S11, closed rev 3.8: the outcome is HTML §6.6.4 *get the focusable area*'s delegates-focus shadow host branch, step 3, on base and head); N1's gate is C0's (D-7 (a)).
  Q-S4's population was `593aa314`'s; four of the 16 lines are not in it, classified in rev 3.9 (read at `c7cce8c8`):
  `document_base.rs:301`, `:345`, `:378` (`BaseUrlMaintainer`'s Insert / Remove / AttributeChange arms) are each guarded
  by `in_document_light_tree` (`:226-231`: the node's — for Remove, the former `parent`'s — tree root is a Document)
  before the read, and the Remove arm also by `removed_a_qualifying_base` (`:324-340`: a removed `<base>` carrying
  `BaseFrozenUrl`, which `attach_frozen_urls_in_subtree` never gives a `<base>` under a template, `:82-96`). A node in
  template contents has the contents fragment as its tree root, so all three return before reading on base and head
  alike: no change. The only new value is for a node in I's own tree (a script's `I.append(..)`; I exists only from
  C1): the arms then maintain I's base from its own `<base href>`, the spec's (HTML §2.4.3 *document base URL* step 1).
  `child_list.rs:454` (*replace* with a fragment: adopt the fragment into `node_document(parent)`, DOM §4.2.3 *replace*
  step 6): for a parent in template contents the value becomes INV-C's, the spec's "parent's node document"; no change
  elsewhere.
- **P-7 C2 statements** (property: a statement that template contents take the template's own document, or a pointer
  to this slot; §0.3 P-C24): `git grep -n 'C2 approximation\|C2 owner-doc\|#11-template-contents-owner-document' -- crates`
  → `mod.rs:608`, `:610`, `crates/dom/elidex-dom-api/src/document.rs:267`, `:268`, `node_methods/clone.rs:425`,
  `elidex-html-parser/src/tests.rs:566`, `scripting.rs:66`; plus `clone.rs:404-405` (the phrase split across a line
  break, which the command cannot see) and the strict tier's "(HTML §4.12.3 *template contents*, C2)"
  (`shadow.rs:118-119`). All rewritten (§5.2).
- **P-8 Document destruction**: H-C12 (2 production paths, verified 2026-11-10); U7 pins the cascade.
- **P-9 render / layout / hit-test roots**: H-C11 (4 consumers, verified 2026-11-10); D-C9 changes the source only.

### §5.2 Existing tests and docstrings whose expectation changes (red at C1 by design, rewritten in the same commit)

Tests (scout Q-S8, re-located at `c7cce8c8` with `/usr/bin/grep -n` on the function names):
`crates/dom/elidex-html-parser/src/tests.rs` `template_content_descendant_owner_document` (`:542`, both assertions:
contents and their `<p>` → I, not the document) and `fragment_parsed_template_content_owner_is_context_document`
(`:571`: the fragment and its `<p>` → the context document's I; the test is renamed to say so);
`crates/dom/elidex-dom-api/src/node_methods/tests/clone.rs` `clone_node_template_shallow_preserves_owner_document`
(`:216`: the shallow copy's `content` → the document's I); `dom/tests/traversal.rs:18`, `:366`, `:412`
(`attach_template_contents(template, dom.node_document_for_new_nodes(doc))`: still compiles, the stamp becomes
`f(doc)`, unasserted; `associated_document.rs` no longer calls the helper); slice a's VM tests holding umbrella cell
C14 (`crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs_once.rs`, inverted, T17); and C0's tests whose
expected value reads a template contents' (or a contents descendant's) node document, or the pair of an adopt segment
of contents (below). C18's test (`tests_event_handler_attrs_window_delegating.rs`) is **unchanged** at C1 (T18).

*Code-side population (rev 3.7, by property).* Every `#[test]` fn that, directly or through a test-side helper fn of
its file (three levels), touches template contents (`attach_template_contents`, `template_contents_fragment`,
`.content`, `<template`) **and** reads a node document or an adopt pair (`node_document(`, `owner_document(`,
`ownerDocument`, `get_associated_document`, `AdoptedContents`, `Adopted {`, `documents: Some`):
```sh
git ls-files 'crates/*.rs' | xargs /usr/bin/grep -l '#\[test\]' | while read -r f; do
  awk -v F="$f" '
    BEGIN { tf = (F ~ /\/tests\/|tests\.rs|\/tests_|test_helpers/) }
    /^#\[cfg\(test\)\]/ { cfg = NR }
    cfg && NR == cfg + 1 && /^mod [A-Za-z0-9_]+ *\{/ { inmod = 1 }
    /^[[:space:]]*(pub[^ ]* )?fn [A-Za-z0-9_]+/ { fn = $0; sub(/^.*fn /, "", fn); sub(/[^A-Za-z0-9_].*/, "", fn); start[fn] = NR; side[fn] = tf || inmod; if (pt) test[fn] = 1; order[++n] = fn }
    { pt = ($0 ~ /^[[:space:]]*#\[test\]/) ? 1 : (pt && $0 ~ /^[[:space:]]*#\[/) }
    fn != "" { body[fn] = body[fn] "\n" $0 }
    /attach_template_contents|template_contents_fragment|\.content[^A-Za-z_]|<template/ { if (fn != "") T[fn] = 1 }
    /node_document\(|owner_document\(|ownerDocument|get_associated_document|AdoptedContents|Adopted *\{|documents: *Some/ { if (fn != "") D[fn] = 1 }
    END {
      for (it = 0; it < 3; it++) for (i = 1; i <= n; i++) { a = order[i]; for (j = 1; j <= n; j++) { b = order[j]; if (a != b && side[b] && body[a] ~ ("(^|[^A-Za-z0-9_])" b "\\(")) { if (T[b]) T[a] = 1; if (D[b]) D[a] = 1 } } }
      for (i = 1; i <= n; i++) { a = order[i]; if (test[a] && T[a] && D[a]) printf "%s:%d:%s\n", F, start[a], a }
    }' "$f"; done
```
→ **36 fns at `c7cce8c8`**: `elidex-ecs`
`dom/tests/adopt_on_insert.rs` 13, `dom/tests/adopt_notification.rs` 4 (the U19 trio, U21), `dom/tests/destroy.rs` 1
(`adopt_rehomes_template_content_fragment`, `:310`); `elidex-custom-elements/src/tests.rs` 5 (U13, the U19 consumer
trio, U20's control); `elidex-dom-api` `node_methods/tests/clone.rs` 1 (above); `elidex-html-parser` `tests.rs` 3 (the
two above and `fragment_template_content_documentless_context_is_unstamped_in_both_backends`, `:621`, unchanged: no
document, no `f`), `tests_node_document.rs` 6 (U8 ×4, U9 ×2); `elidex-js` `vm/tests/tests_insertion_adoption.rs` 3 (T4's
control, T5, T18's control). Complement: a fixture built by a helper in another file, or a test that reaches contents
by another spelling, is not counted. Each fn is classified (changes / value unchanged / not dependent) by the rule of
the memo-cell list below, at C1's red-first run (§9 item 1), and the result goes in C1's PR body (PM 604); the memo cells named there are the ones this list's fn
names carry (`t5_…`, `u1_…`, `u8_…`, `u11_…`, `u13_…`, `u19_…`, `u21_…`).

*C0 cells C1 changes (by property, not by a "C1 turns" phrase).* Population: every §6.1 / §6.2 cell of the C0a, C0b and
C0c memos whose script, shape or expected value involves template contents — `for m in <C0a> <C0b> <C0c>; do awk
'/^## §6/{f=1} /^### §6\.5/||/^## §7/{f=0} f && /^\| *[TU][0-9]+/' "$m" | /usr/bin/grep -iE 'template|content'; done`
(the program memo has no cells, its §1.1; §6.5 base formulations run at C0's bases, not C1's) — then each is classified
by whether its expected value depends on the contents' node document or a contents segment's pair. On the landed memos
at `c7cce8c8` the population is C0b T5, T23, U1, U8, U11, U12 and C0c T18, U13, U18, U19, **U21** (rev ≤ 3.6.1, at
the frozen C0 frame `a93142c4` / `05142d8c`: the same without U21, which C0c added later):
- **changes (rewritten in C1's commit)**: C0b **T5** (both legs → the inert document, C1's T13 value); C0b **U1** (the
  template-with-contents variant: contents stamped `f(D)`; the "append into a contents fragment" variant's fixture, if
  built through `attach_template_contents`, reads `f(D)`); C0b **U8** (×2; "every node except the Document reads `D`" →
  nodes in contents read `f(D)`; the walk-counter leg is unchanged, C1's U12); C0b **U11** (the top-level-template and
  nested-template legs → `f(context's document)`); C0c **U13** (C's segment pair → « `f(old)`, `f(new)` »); C0c **U19**
  (first leg → `Adopted(X, f(target))`; second leg: the outer contents in the target are no longer skipped, `target ≠
  f(target)`, so the leg is re-set with the outer contents in `f(target)` to keep constructing the skip; third leg:
  stamped `f(target)`, its condition `X ≠ f(target)`); C0c **U21** (rev 3.9; `crates/core/elidex-ecs/src/dom/tests/adopt_notification.rs:305-327`,
  `u21_contents_cycle_root_is_not_a_segment`): its fixture establishes `f` with
  `attach_template_contents(tpl, node_document_for_new_nodes(docs.x))`, which on C1's head stamps `f` with
  `I_X = f(docs.x)` (D-C3), so `append_child(f, tpl)` adopts `tpl` into `I_X` (before the log is installed) and
  `adopt(f, docs.d)` reads `old = I_X`: the logged root record becomes `documents: Some((I_X, docs.d))`, not
  `(docs.x, docs.d)`. Its other legs are unchanged: no segment for the root (the walk does not treat its own root as a
  segment, `mutation.rs:339-346`; INV-C (c)) and `[f, tpl]` both read `docs.d`;
- **value unchanged**: C0c **T18** (its segment's `new` becomes `f(dp2)`, but the logged `n === dp` is `false` either
  way; its "C1 turns" note is a comment; C1's T25 discriminates); C0c **U18** (`dom/tests/traversal.rs:404-462`): its
  fixture establishes contents with `node_document_for_new_nodes(doc)`, so on C1's head the fragment and the `<i>`
  appended into it are stamped `f(doc)`, but both legs assert the `Include` walk's visiting order, which reads no
  document;
- **not dependent**: C0b **T23** (`tests_insertion_adoption.rs:331`, `:341`; a Range `extractContents` /
  `cloneContents` on a DOMParser document with no template — the population's grep matches "Contents") and C0b
  **U12** (`elidex-dom-api/src/creation_owner_tests.rs:53`, `:145`; DOMParser creation sites, no template — it matches
  "textContent"): neither establishes template contents nor reads an adopt pair. C0a: no cell in the population.
Re-run at the cut if main moves (§9 item 1).
Docstrings: P-7's population, plus `components.rs:356-361` (`TemplateContents`' adopt sentence, "DOM §4.5 *adopt*
re-homes the contents with the template"), `mutation.rs:234-235` (`adopt`'s "into the same document here") and the
`adopt_walk` doc's "`new` is `document` (C2)" (`mutation.rs:314-315`), `mutation_event.rs:60-79` (`AdoptedContents`:
its `new` becomes `f(document)`), `replicate_template_contents`'s "the C2 owner-doc approximation" (`clone.rs:403-405`)
and its body comment (`clone.rs:422-426`), `scripting.rs:64-66`.

### §5.3 Touched files

Line counts: `wc -l` at `c7cce8c8` (verified 2026-11-10; the `593aa314` column is history); re-measured at the cut if main
moves (§9 item 1). C1 touches no VM host file: the VM wrapper work for I is C1-pre's (edge c-pre, §4.1).

| file | lines at `c7cce8c8` (at `593aa314`) | change | 1000-line decision |
|---|---|---|---|
| `crates/core/elidex-ecs/src/dom/template_owner.rs` (NEW) | — | D-C2 rule, predicate, D-C7 hook | new file, by design: keeps `mod.rs` under the limit |
| `crates/core/elidex-ecs/src/dom/mod.rs` | 949 (926) | `mod` line; `attach_template_contents` doc and one call; two stale comments (rev 3.9): `create_document_root`'s doc `:453-455` names the CLOSED slot `#11-document-url-real-navigation` (subsumed by `#11-document-base-url`) → `#11-document-base-url` for a browsing-context Document, and `create_document_node`'s doc says that for the inert template document `about:blank` is the spec's document base URL, not a pending one: HTML §2.4.3 *document base URL* steps 1–3 → *fallback base URL* steps 1–2 (I is a new Document, HTML §4.12.3 *appropriate template contents owner document* step 1.1.1, so its URL is DOM §4.5's default "about:blank" and its about base URL is null), while I has no `<base href>` (rev 3.10: not "final", a script-appended `<base>` changes it; §4.9); `:469` "§13.4 fragment parsing algorithm's throwaway document (step 4)" → step 1 (*Stale cites* below) | net ≤ +10 lines → ≤ 959, stays < 1000; below the ~990 trigger of O-3 (decided, PM 146 / PM 223 O-3), so no prereq split unless main grows it before the cut |
| `crates/core/elidex-ecs/src/components.rs` | 831 (822) | two components, `TemplateContents` doc | stays < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/mutation.rs` (the adopt walk, H-C6) | 725 (467) | D-C5: the document of live contents segments (stamps, `new`, skip test), the per-node segment bookkeeping (D-C5's implementation obligation) and the `adopt` / `adopt_walk` docs | < 1000 (275 lines of headroom; re-checked at the cut) |
| `crates/core/elidex-ecs/src/dom/mutation_event.rs` | 439 | `AdoptedContents` doc (§5.2) | < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/teardown.rs` | 337 (344) | Document cascade in `destroy_entity` (D-C7) | < 1000 |
| `crates/core/elidex-ecs/src/dom/tree/navigation.rs` | 375 (375) | `root_entities` exclusion (D-C9) | < 1000 |
| `crates/core/elidex-ecs/src/dom/tree_clone.rs` | 414 (425) | clone-policy rows (D-C8) | < 1000 |
| `crates/dom/elidex-dom-api/src/node_methods/clone.rs` | 463 (458) | D-C6 on D14 | < 1000 |
| `crates/dom/elidex-dom-api/src/document.rs` | 728 (665) | comment only (P-7) | < 1000 |
| `crates/dom/elidex-html-parser-strict/src/tree_builder/shadow.rs` | 222 | comment only (P-7, `:118-119`) | < 1000 |
| `crates/script/elidex-script-session/src/scripting.rs` | 244 (279 at `27b50dfe`) | doc `:64-66` | < 1000 |
| `crates/dom/elidex-html-parser/src/tests.rs` | 884 (875) | two expectations (§5.2) | < 1000 |
| `crates/dom/elidex-dom-api/src/node_methods/tests/clone.rs` | 731 (992; RC-0a #587 split it, adding `tests/clone_shadow.rs`) | one expectation rewritten in place (`:216`); **no new tests** (they go to a new file) | < 1000 |
| the §5.2 population's files (C0's tests) | 183–721 each (`wc -l` of the six files) | expectations rewritten in place per §5.2's classification | re-checked at the cut |
| `crates/core/elidex-ecs/src/dom/tests/template_owner.rs` (NEW) | — | §6.2's unit cells except those the §6.2 heading places elsewhere (the heading is the single statement of each cell's home; rev 3.10.1, why U12's crate: it is the lowest crate that reaches both tiers — it depends on `elidex-html-parser-strict` — and `elidex-ecs`'s `MutationDispatcher`) | new file |
| `crates/dom/elidex-dom-api/src/node_methods/tests/clone_template_owner.rs` (NEW, rev 3.10) | — | unit cell U14 (§6.2; the clone post-pass is `elidex-dom-api`'s, which `elidex-ecs`'s tests cannot reach) | new file |
| `crates/dom/elidex-dom-api/src/node_methods/tests/mod.rs` | 29 | `mod clone_template_owner;` | < 1000 |
| `crates/script/elidex-js/src/vm/tests/tests_template_owner_document.rs` (NEW) | — | VM cells T-* (§6) | new file |
| `crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs_once.rs` | 369 | umbrella cell C14's test (`c14_template_contents_handler_runs`, `:315`) inverted (T17, §5.2) | < 1000 |
| `crates/script/elidex-js/src/vm/tests/mod.rs` | 209 | `mod tests_template_owner_document;` | < 1000 |
| `crates/core/elidex-ecs/src/dom/tests/mod.rs` | 31 | `mod template_owner;` | < 1000 |
| `crates/script/elidex-js/src/vm/tests/tests_html_t2d_protos.rs` | 1207 (1207) | **not touched** (no node-document read: `/usr/bin/grep -c 'ownerDocument\|owner_document(\|node_document(' <file>` → 0; §0.3 P-C27) | not triggered |

**Stale cites (PM 607; widened by property in rev 3.9).** The obligation is every cite, in a file C1 touches (this
table and §5.2's population), whose § or step moved per §10.5 F2 — HTML §8.5's renumbering (innerHTML §8.5.5,
createContextualFragment() §8.5.8), *create an element for a token* (step 3 → 6), the rewritten *HTML fragment parsing
algorithm* (old steps → steps 1–7 and *initialize an HTML fragment parser*), *clone a single node*'s Document branch
(5.1–5.3) — not only `§8\.5`. Seed at `c7cce8c8` (run 2026-11-10; widened in rev 3.10 to the spec's own dfn wording
and anchor): `git grep -n -E '§ ?8\.5\.[0-9]|§ ?13\.4|§ ?13\.2\.6\.1|create an element for a token|create an element
for the token|create-an-element-for-the-token|fragment parsing algorithm|clone a single node' c7cce8c8 -- <files>` →
21 hits over the 16 files of this table that exist at `c7cce8c8` and that C1 touches (verified 2026-11-10) (every row but the NEW files, the
§5.2 population row, `node_methods/tests/mod.rs` (0 hits) and the untouched `tests_html_t2d_protos.rs`), plus 1 over
§5.2's six test files (`adopt_on_insert.rs`, `adopt_notification.rs`, `destroy.rs`, `elidex-custom-elements/src/tests.rs`,
`tests_node_document.rs`, `tests_insertion_adoption.rs`): `tests_node_document.rs:3`. The widening adds no hit (its two
matches, `shadow.rs:152` and `tests_node_document.rs:3`, already match `§ ?13\.2\.6\.1`). Each hit is checked
against §3's lookups; one is stale: `crates/core/elidex-ecs/src/dom/mod.rs:469` (the throwaway document is step 1 of
the rewritten algorithm, not step 4); the step-numbered others are current (`navigation.rs:279` step 7, `shadow.rs:152`
step 6, `tree_clone.rs:23`, `:295` *clone a single node* step 2.4). C1 fixes `:469`, and its new and rewritten text cites
the current numbers. Every stale HTML cite outside the touched files goes to the PM's existing HTML renumber sweep, not
to C1 — among them the excluded `tests_html_t2d_protos.rs:700` (`HTML §8.5.4 "The innerHTML property"`, now §8.5.5;
an F2-class stale cite in a file C1 does not touch) and `range_proto_mutation.rs:248` (`§8.5.7 step 7`, now §8.5.8).
The DOM part of F2 (*clone a single node*'s renumbered Document branch) has no stale cite outside the touched files
at `c7cce8c8`: `git grep -n -i -E 'clone a single node|clone-a-single-node' c7cce8c8 -- crates | /usr/bin/grep -i
step` → 3 hits, all in `tree_clone.rs` (touched; `:23` and `:295` step 2.4, `:30` step 2.5, each current), so 0
outside; the HTML sweep, which carries no DOM cites, needs no DOM share.

**Removed in rev 3 (moved to C0 rev 2).** Rev 2 listed seven files (verified 2026-11-10) that C0 now owns: the custom-element consumer
`crates/dom/elidex-custom-elements/src/consumer.rs` (the `Adopt` arm and per-node pairs, C0 draft D-6); the parser
creation sites and the entry's strict method, `crates/dom/elidex-html-parser/src/convert.rs`,
`crates/dom/elidex-html-parser/src/lib.rs`, `crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs`,
`tree_builder/insert.rs`, `tree_builder/shadow.rs` (C0 draft D-10, D-11, §4.4); and
`crates/core/elidex-ecs/src/dom/tree/walkers.rs` (rev 2's Q-2, answered by C0 draft D-3 (c); C0c reorders `Include`,
D-19). C1's diff touches none of them except the comment-only rewrite of the strict tier's C2 sentence
(`tree_builder/shadow.rs:118-119`, §5.1 P-7; rev 3.7).

### §5.4 Named risks (and the observable change each is)

- **R-C1 `ownerDocument` of template contents** → INV-C's value, I for a fragment not adopted out (spec). A script that
  compared it to `document` sees `false`.
- **R-C2 URL resolution in template contents** → under INV-C, against `about:blank`. Spec derivation: HTML §2.4.3
  *document base URL* of I: step 1, I has no `base` element with an `href` (unless a script adds one); step 2, I has no
  base URL override; step 3, I's *fallback base URL*: step 1 does not apply (I's URL is "about:blank", DOM §4.5's
  default, but its about base URL is null), step 2 returns I's URL, `about:blank`. `x.html` fails URL §4.4's *basic URL
  parser* against it, so the `href` getter's url is null and HTML §4.6.3's step 4 returns "this's href content
  attribute's value": `"x.html"` (cell T20; read through `href_accessor.rs:51`, H-C8). On C1's base the same node's
  document is `document`, whose document base URL is its first `<base href>` (step 1). **For the bound document,
  elidex never writes the page URL into `DocumentBaseUrl`** at `c7cce8c8`: it is `about:blank` from
  `create_document_node` (`mod.rs:474-480`) and only `<base href>` changes it (`BaseUrlMaintainer`,
  `document_base.rs:270`, `:284`, `:346`, `:365`; the VM test
  `crates/script/elidex-js/src/vm/tests/tests_html_t2a_protos.rs:186-199`, and `:201-211` for the `href` getter's raw
  value, pin that a relative href does not resolve), so in a bound document with no `<base href>` a template-contents
  `<a href=x.html>` gives `"x.html"` on base and head alike. **The one writer of the page URL is DOMParser** (rev
  3.10): `parse_into_inert_document` calls `initialize_base_url_for_document(dom, doc, caller_base_url)`
  (`crates/dom/elidex-form/src/inert_document.rs:200`; `document_base.rs:187-190` → `recompute_document_base`, whose
  no-`<base>` branch writes the fallback, `:147-154`), the VM passing `ctx.vm.navigation.current_url`
  (`crates/script/elidex-js/src/vm/host/dom_parser.rs:281`, `:295`): HTML §8.5.1 `parseFromString()` step 3 (the new
  Document's URL is relevantDocument's URL). So in a `<base>`-less DOMParser document `dp` an `<a href=x.html>` in a
  template's contents resolves against the page URL on the base (its node document is `dp`) and gives `"x.html"` on
  C1's head (its node document is INV-C's, `f(dp)`, whose document base URL is `about:blank` as above): an observable change,
  the spec's (cell T29). T20 and T9 therefore carry a `<base href>` fixture, and a clone of contents
  appended to the document resolves against the document's `<base href>` again (T9). That the page URL is not the
  fallback is `#11-document-base-url`'s defect, not C1's. `img.src` is not a change: HTML §2.6.1's URL getter (steps
  3.3, 4) would resolve it, but elidex's `src` getter returns the attribute without resolving on base and head alike
  (`reflect_getter!(image_get_src, …)` `crates/script/elidex-js/src/vm/host/html_image_proto.rs:258`, macro
  `:207-228`), the defect of `#11-url-reflecting-attribute-getters-unresolved` (PM 610, owner L3), so C1 asserts
  nothing about it. C1 adds no reader of the document base URL, only nodes whose base is I's fallback;
  the readers are `#11-document-base-url`'s population.
- **R-C3 handlers in template contents** → under INV-C, raw values read null and do not run (N1 + C0 + C1; C14 inverts,
  T17).
- **R-C4 `<body>` / `<frameset>` in template contents** → unchanged at C1 (the base's target rule ignores the
  document, T18); T3 (after C1) gives them the null target.
- **R-C5 a new Document wrapper (I) is reachable from script.** Every Document wrapper has `Document.prototype`
  (C1-pre D-1), so I's does, with no C1 code; its per-entity facts (no browsing context: `defaultView` `null`,
  `hidden` `true`, …) are C1-pre's D-4 to D-8 (edge c-pre). Spec values: `defaultView` null (I's browsing context is
  null, HTML §4.12.3 *appropriate template contents owner document* step 1.1.1), `baseURI` "about:blank" (R-C2), `body` null (I has no children unless a script adds
  them), `ownerDocument` null (a Document, DOM §4.4). C1 asserts `defaultView` only (T1's third leg, C1-pre's
  value); the others are C1-pre's surface. Its `URL` / `documentURI` read the page URL until `#11-document-base-url`
  lands (§0.3 P-C17); C1 asserts neither (PM 604, Q-5 (a)); the defect and its cells are record R-11 (§8).
- **R-C6 I as a root.** Excluded (D-C9); cell U6.
- **R-C7 lifetime.** No I for a throwaway (U4, U5); I per DOMParser document lives with it (D-C7, §7).
- **R-C8 `adoptedCallback` arguments for custom elements in contents** → see D-C5 (T25): DOM *insert* step 7.1
  adopts into the contents' node document and *adopt* step 3.3.3 enqueues « oldDocument, document »; a custom element
  stays custom across an adopt, so the pair is `(document, I)` for one appended into contents.

## §6. Test plan

Columns: *HEAD* = `593aa314` (history); *base* = `c7cce8c8` (N1 implementation, C0a–C0c, RC-2a with D14 and C1-pre
landed; still C2); *C1 = spec*. **Red-first**: every
*red* cell is run on C1's base before the fix with its control green, and the failure is recorded in the PR description.
Cross-tier: cells marked (×2) run under both parser tiers. Every *C1 = spec* value is the acceptance criterion
(PM 607); none waits on a measurement.

**Element access in §6.1's scripts (rev 3.10).** The VM has no window named access (no bare-id globals), and a node in
template contents is not in any document tree, so `document.getElementById` does not reach it, and the VM's
DocumentFragment has no `getElementById` (pre-existing slot `#11-nonelementparentnode-on-fragments`; record R-14).
So every script reaches a document-tree element through `document.getElementById(..)` (or `dp.getElementById(..)`)
and a node in contents through `content.firstChild` / `.lastChild` / `.nextSibling` (or `content.querySelector`,
from the ParentNode mixin, `document_fragment_proto.rs:7-9`); an `id` in markup is for that lookup only. `tpl` is
`document.createElement('template')` unless the cell defines it; cells that use `document.head` / `document.body`
run on a fixture that has them (as `tests_base_href.rs`'s `build_fixture`). Cells T17 / T18 run the umbrella's
existing scripts unchanged.

### §6.1 VM integration (`tests_template_owner_document.rs`, NEW)

| cell | kind | script | base | C1 = spec |
|---|---|---|---|---|
| T1 | red | `t = document.createElement('template'); I = t.content.ownerDocument; r = [I !== document, Object.getPrototypeOf(I) === Object.getPrototypeOf(document), I.defaultView === null]` (second leg: C1-pre R-2; no `Document` interface object exists, H-C16). Three legs (PM 604, Q-5 (a)): rev ≤ 3.7's legs 4–5 (`I.URL` / `I.documentURI === 'about:blank'`) are `#11-document-base-url`'s cells `DBU-I-URL` / `DBU-I-documentURI` (§8 R-11), not C1's, and rev 3.6.1's page-URL fixture (IMP-A), which only those legs read, goes with them | `[false, true, false]` | `[true, true, true]`: the second and third elements rest on C1-pre (edge c-pre) |
| T2 | guard | two `createElement('template')`: `t1.content.ownerDocument === t2.content.ownerDocument` (with T1's `!== document`) | `true` (both `document`) | `true` (same I) |
| T3 | red (×2) | initial HTML `<template id=a><b></b></template>`; `a = document.getElementById('a')`: `a.content.ownerDocument !== document`, `a.content.firstChild.ownerDocument === a.content.ownerDocument` (rev 3.9: not `content.getElementById`, which the VM's DocumentFragment lacks at `c7cce8c8` — `TypeError: not a function`, observed in the T20 check, §10.7; pre-existing slot `#11-nonelementparentnode-on-fragments`, record R-14) | `[false, true]` | `[true, true]` |
| T4 | red (×2) | `<template id=o><template><p></p></template></template>`; `o = document.getElementById('o'); i = o.content.firstChild`: `i.ownerDocument === o.content.ownerDocument`, `i.content.ownerDocument === o.content.ownerDocument`, `o.content.ownerDocument !== document` | `[true, true, false]` | `[true, true, true]` |
| T5 | guard (with U2; `I.createElement` rests on C1-pre, edge c-pre) | `I = tpl.content.ownerDocument; t = I.createElement('template'); t.content.ownerDocument === I` | `true` (`I` is `document`) | `true`, and U2 shows no I-of-I entity |
| T6 | red | `dp = new DOMParser().parseFromString('<template id=t><i></i></template>','text/html'); c = dp.getElementById('t').content; [c.ownerDocument !== dp, c.ownerDocument !== document, c.ownerDocument !== tpl.content.ownerDocument, c.firstChild.ownerDocument === c.ownerDocument]` | `[false, true, true, true]` | `[true, true, true, true]` |
| T7 | red | `tpl.innerHTML = '<b></b>'; [tpl.content.firstChild.ownerDocument === tpl.content.ownerDocument, tpl.content.firstChild.ownerDocument !== document]` | `[true, false]` | `[true, true]` |
| T8 | red | `d = document.createElement('div'); tpl.content.append(d); [d.ownerDocument === tpl.content.ownerDocument, d.ownerDocument !== document]` | `[true, false]` | `[true, true]` |
| T9 | guard (K-0; §1 rows 1–2) | T20's `<base href>` fixture; `tpl.innerHTML = '<a href=x.html></a>'; c = tpl.content.cloneNode(true); a = c.firstChild; r1 = a.ownerDocument !== document; document.body.append(c); [a.ownerDocument === document, a.href === 'http://example.com/dir/x.html']` (the second leg resolves against the document's `<base href>`, R-C2) | `false`, then `[true, true]` (observed, §10.7) | `true`, then `[true, true]` (C1 without C0: `[false, false]`) |
| T10 | red | `tpl.innerHTML = '<template><p></p></template>'; t2 = tpl.cloneNode(true); n2 = t2.content.firstChild; s2 = tpl.cloneNode(false)`: `t2.content !== tpl.content`, `t2.content.ownerDocument === tpl.content.ownerDocument`, `n2.content.ownerDocument === tpl.content.ownerDocument`, `t2.content.ownerDocument !== document`; `s2.content.firstChild === null`, `s2.content.ownerDocument !== document` | `!== document` legs false | all true |
| T11 | red | initial HTML with one `<template>`; `tpl = document.querySelector('template'); dc = document.cloneNode(true)`; `x = dc.querySelector('template').content.ownerDocument; [x !== dc, x !== tpl.content.ownerDocument]` | `[false, true]` | `[true, true]` |
| T12 | red (DSD legs strict only; variant ×2) | `<template id=t><div><template shadowrootmode=open><span></span></template></div></template>`; `t = document.getElementById('t'); h = t.content.firstChild; span = h.shadowRoot.firstChild`: `[h.shadowRoot.ownerDocument === t.content.ownerDocument, span.ownerDocument === t.content.ownerDocument, h.shadowRoot.ownerDocument !== document]`; variant: a parser-built `<div>` in contents (`<template id=v><div></div></template>`, `d = document.getElementById('v').content.firstChild`), `d.attachShadow({mode:'open'}).ownerDocument !== document`. A tolerant document parse attaches no declarative shadow root (`#11-tolerant-document-declarative-shadow-roots`; host choice `#11-tolerant-dsd-host-from-tree-builder`), so the DSD legs have no tolerant run until that slot lands; their tolerant runs are that slot's cells (record R-13, §8) | the `!== document` legs false | all true |
| T13 | red | `dp = new DOMParser().parseFromString('<template id=t><template><p></p></template><div></div></template>','text/html'); t = dp.getElementById('t'); h = t.content.lastChild; h.attachShadow({mode:'open'}).innerHTML = '<i></i>'; document.body.append(t)`: `t.ownerDocument === document`, `t.content.ownerDocument === tpl.content.ownerDocument`, `t.content.firstChild.content.ownerDocument === tpl.content.ownerDocument`, `h.shadowRoot.firstChild.ownerDocument === tpl.content.ownerDocument`, `t.content.ownerDocument !== document` (rev 3.10.2: the `=== tpl.content.ownerDocument` legs alone are true on the base too, where `tpl.content.ownerDocument` is `document`; this leg tells I from `document`: I is the Document that HTML §4.12.3 *appropriate template contents owner document* step 1.1.1 creates and step 1.1.3 links to `document`, returned by steps 1.2 and 2, reached here through the template *adopting steps* steps 1–2) | `[true, true, true, true, false]` (every node `document`; `tpl.content.ownerDocument` is `document`) | `[true, true, true, true, true]` (`=== tpl.content.ownerDocument`, I of `document`, for all) |
| T14 | red (×2) | `el = document.body; el.innerHTML = '<template><p></p></template>'; f = el.firstChild`: `[f.content.ownerDocument === tpl.content.ownerDocument, f.content.firstChild.ownerDocument === tpl.content.ownerDocument, f.content.ownerDocument !== document]` | `[true, true, false]` | `[true, true, true]` |
| T17 | red (inverts umbrella C14; needs N1, C0) | umbrella C14's script | runs, `window.hit === 1` | does not run; `x.onclick === null` |
| T18 | guard (umbrella C18 unchanged at C1) | umbrella C18's script | C18's "as HEAD" column | the same (the base's target rule ignores the document); T3 rewrites C18 after C1 (T3 draft P-T9) |
| T19 | red (×2) | `<template id=s><svg></svg><b></b></template>`; `s = document.getElementById('s'); v = s.content.firstChild; w = s.content.lastChild`: inside contents `[v.ownerDocument !== document, v.ownerDocument === s.content.ownerDocument]`; `document.body.append(s.content)`; `[v.ownerDocument === document, w.ownerDocument === document]` | inside `[false, true]`; after `[true, true]` | inside `[true, true]`; after `[true, true]` |
| T20 | red (R-C2) | fixture shaped like `crates/script/elidex-js/src/vm/tests/tests_base_href.rs`'s `build_fixture` + `bind_vm`; script: `b = document.createElement('base'); b.setAttribute('href', 'http://example.com/dir/'); document.head.appendChild(b); tpl = document.createElement('template'); tpl.innerHTML = '<a id=a href=x.html></a>'; tpl.content.firstChild.href`. Without the `<base href>` the cell cannot go red (elidex writes no page URL into the bound document's `DocumentBaseUrl`, R-C2; the DOMParser case is T29) | `"http://example.com/dir/x.html"` (observed at `c7cce8c8`, §10.7: the contents node's document is `document`, whose document base URL is its `<base href>`) | `"x.html"` (R-C2: I has no `<base>`, its document base URL is its fallback `about:blank`, HTML §2.4.3 *document base URL* steps 1–3 → *fallback base URL* steps 1–2) |
| T22 | red | `tpl.innerHTML = '<b onclick="1">'; x = tpl.content.firstChild`; read `x.onclick` | a function (`scripting.rs:64-66`'s false negative) | `null` |
| T23 | red (**R-A closure**, §9 item 9; needs N1, C0) | for each route by which a `<b onfocus="window.hit=1">` reaches template contents — (i) parsed (×2): initial HTML `<template id=p><b onfocus="window.hit=1"></b></template>`, `tpl = document.getElementById('p'); b = tpl.content.firstChild`; (ii) `tpl.content.append(b)` of a `createElement`'d `b`; (iii) `tpl.innerHTML = '<b onfocus="window.hit=1"></b>'; c = tpl.content.cloneNode(true); b = c.firstChild`; (iv) the same `tpl`, `c = tpl.cloneNode(true); b = c.content.firstChild` — read `[b.ownerDocument === tpl.content.ownerDocument, b.ownerDocument !== document]`; for route (ii) also `b.onfocus === null` (the only route on which the base writes a raw value: slice a's Insert arm, W-K5; routes (i), (iii), (iv) are unprocessed at both base and C1 — K-5 and the G5 / K3 window — so their handler leg is slice b's, not this cell's). **Route (v), control** (INV-C (c), §4.6 item 1): `document.adoptNode(tpl.content)`, then `tpl.content.append(b)` of a `createElement`'d `b`; read `[b.ownerDocument === document, typeof b.onfocus === 'function']` (spec: the fragment, adopted whole, has node document `document`, so `b` is in a bound document and its raw value compiles) | routes (i)–(iv) `[true, false]`; route (ii) `b.onfocus === null` → `false`; route (v) `[true, true]` | routes (i)–(iv) `[true, true]`; route (ii) → `true`; route (v) `[true, true]` (unchanged: a C1 that mapped an adopted contents root to `f(document)` gives `[false, false]`) |
| T24 | red | clonable shadow host in contents: `tpl.innerHTML = '<div id=h></div>'`; `h = tpl.content.firstChild; h.attachShadow({mode:'open', clonable:true}).innerHTML = '<i></i>'`; `c = tpl.cloneNode(true); ch = c.content.firstChild; [ch.ownerDocument !== document, ch.shadowRoot.ownerDocument !== document, ch.shadowRoot.firstChild.ownerDocument !== document]` | `[false, false, false]` | `[true, true, true]` (C1 without D-C6's per-pair document: also `[true, true, true]` — each node cloned with `document` is adopted into the copy's I by its link, `clone.rs:433`, `:298`, D-C6; that mutant is U14's, rev 3.10) |
| T25 | red (R-C8; needs C0c's `Adopt`; C1's half only: the pair **values**; C0c's T18 / U13 own per-node pairs and order) | `I = tpl.content.ownerDocument`; `x-a` with an `adoptedCallback(o, n)` logging the pair; `dp = new DOMParser().parseFromString('<template id=t></template>','text/html'); I_dp = dp.getElementById('t').content.ownerDocument`; `e = document.createElement('x-a'); dp.getElementById('t').content.append(e)`; `document.body.append(dp.getElementById('t'))`; read `[log.length, log[0][0] === document, log[0][1] === I_dp, log[0][1] !== dp, log[1][0] === I_dp, log[1][1] === I, log[1][1] !== document]` (rev 3.10.2: on the base `I_dp` is `dp` and `I` is `document`, so the `=== I_dp` / `=== I` legs alone are true on the base; the two `!==` legs tell I_dp from `dp` and I from `document`: each is the Document that HTML §4.12.3 *appropriate template contents owner document* step 1.1.1 creates for its document and step 1.1.3 links, returned by steps 1.2 and 2 — for the second callback through the template *adopting steps* steps 1–2) | first callback `(document, dp)` (C2: `I_dp` is `dp`); second `(dp, document)`: `[2, true, true, false, true, true, false]` | first `(document, I_dp)`; second `(I_dp, I)`, in that order: `[2, true, true, true, true, true, true]` |
| T26 | red (D-C5's `f(target)` on first use) | `dp = new DOMParser().parseFromString('<p></p>','text/html')` (no template, so no I yet); `tpl.content.append(document.createElement('i')); dp.body.append(tpl); x = tpl.content.ownerDocument; [x !== dp, x !== document, tpl.content.firstChild.ownerDocument === x, dp.createElement('template').content.ownerDocument === x]` | `[false, true, true, true]` (`x` is `dp`) | `[true, true, true, true]` (`x` = `dp`'s new I, created by the adopt) |
| T27 | red (C0c skip on the mapped `new`, forward) | `x-a` as T25; `tpl.innerHTML = '<template id=n></template>'; n = tpl.content.firstChild; e = document.createElement('x-a'); n.content.append(e)` ‖ `log = []; document.body.append(n)` ‖ `[log, e.ownerDocument !== document, n.ownerDocument === document]` | `[[], false, true]` (same-document move) | `[[], true, true]`: `n.content`'s old `I` equals `f(document) = I`, so the segment is skipped and no `adoptedCallback(I, I)` fires |
| T28 | red (C0c skip on the mapped `new`, reverse) | `x-a` as T25; `dp = new DOMParser().parseFromString('<template id=t></template>','text/html'); I_dp = dp.getElementById('t').content.ownerDocument`; `t = document.createElement('template'); e = document.createElement('x-a'); t.content.append(e); dp.adoptNode(t.content)` ‖ `log = []; dp.body.append(t)` ‖ `[log, t.content.ownerDocument !== dp, e.ownerDocument === t.content.ownerDocument]` | `[[], false, true]` (contents' old `dp` equals the target: skipped) | `[[(dp, I_dp)], true, true]`: old `dp` ≠ `f(dp)`, so the contents and `e` go to `I_dp` |
| T29 | red (R-C2, DOMParser; rev 3.10) | fixture: page URL `http://example.com/dir/page` (`vm.inner.navigation.current_url = url::Url::parse("http://example.com/dir/page").unwrap()`, as `tests_forbidden_headers.rs:228`; without it the VM's `current_url` is `about:blank`, `crates/script/elidex-js/src/vm/host/navigation.rs:201`, and the cell cannot go red). Script: `dp = new DOMParser().parseFromString('<a id=b href=x.html></a><template id=t><a href=x.html></a></template>','text/html'); [dp.getElementById('b').href, dp.getElementById('t').content.firstChild.href]` (the first leg is the control: an `<a>` in `dp`'s own tree) | `["http://example.com/dir/x.html", "http://example.com/dir/x.html"]` (the contents `<a>`'s node document is `dp`, whose `DocumentBaseUrl` is the page URL, R-C2) | `["http://example.com/dir/x.html", "x.html"]`: `dp`'s URL is the page URL (HTML §8.5.1 `parseFromString()` step 3) and it has no `<base href>`, so its document base URL is its fallback base URL, its URL (HTML §2.4.3 *document base URL* step 3, *fallback base URL* step 2); the contents `<a>`'s node document is INV-C's, `f(dp)`, whose document base URL is `about:blank` (R-C2), so `x.html` does not parse and the getter returns the attribute (HTML §4.6.3 step 4) |

### §6.2 Unit (`crates/core/elidex-ecs/src/dom/tests/template_owner.rs`, NEW; except U12, in `crates/dom/elidex-html-parser` (file chosen at implementation); U3 / U4 / U5, whose home is chosen at implementation as for U12 (they depend on a parser, so they cannot live in `elidex-ecs`); and U14, in `crates/dom/elidex-dom-api/src/node_methods/tests/clone_template_owner.rs` (NEW); §5.3)

| cell | kind | shape | expected |
|---|---|---|---|
| U1 | red | `f(D)` twice → same entity, marked; D linked | equal, marked |
| U2 | red | `f(f(D)) == f(D)`; count of marked entities is 1 | 1 |
| U3 | red | D without any template: no marked entity after parse and `createElement('div')` | 0 (lazy) |
| U4 | red | 100 fragment parses with a template (both tiers), context in D: marked-entity count stays 1 (D's I); the throwaway carries no `AssociatedInertTemplateDocument` (read as a component) before its destruction | 1; absent |
| U5 | red | precondition: D's I is created before the parse (`f(D)` called once on D; rev 3.10.2, so the marked count before the parse is 1 and both counts below are exact); then a strict fragment parse with a template and a parse error, context in D: the marked-entity count stays 1 (D's I) and the world's entity count is unchanged (no entity of the throwaway survives) | as stated |
| U6 | red | `I` with an element child: `root_entities()` excludes I | excluded |
| U7 | red | `destroy_entity(D)` destroys `f(D)`; a contents node outliving it reads `node_document` → `None` | destroyed, `None` |
| U8 | red | clone policy: `clone_subtree(D)` copy has no link; `clone_subtree(I)` copy has no marker | absent |
| U9 | red | C0's adopt of a template into X, with nested contents and a shadow host in contents: shadow-including descendants → X, contents and nested contents and their shadow trees → `f(X)` | as stated |
| U10 | guard | the adopt walk past the depth and breadth caps (as `despawn_subtree_tears_down_past_the_traversal_depth_cap`, `dom/tests/destroy.rs`) | every node re-homed |
| U11 | red | C0's new-node entry for a `source` in D's template contents (the fragment, a descendant, nested contents, a shadow root in contents) → `f(D)`; and for a template parsed in a fragment build with context in D, its contents fragment's stamp → `f(D)` (never `f(throwaway)`). The entry's own contract (D → D; the fragment-parse root → the context's document) is C0b's cells (its U11), not C1's | as stated |
| U12 | red (×2) | document parse of `<template><svg></svg><b>t</b><!--c--></template>` into D: after the parse the walk counter reads 0 (`adopt_walk_count()`, `mutation.rs:419`, `fixtures`; the parse builds into a fresh `EcsDom`, whose counter starts at 0), the contents hold 5 nodes (fragment, `svg`, `b`, text, comment) and every one carries `AssociatedDocument == f(D)`. A walk would mean a node was created with another document and re-homed at its link. This is C0b's U8 form (C0b memo §6.2 U8; `crates/dom/elidex-html-parser/src/tests_node_document.rs:56-66` at `8344af2b`). Rev 3.10.2: the counter replaces a recording `MutationDispatcher`, which cannot be installed before a document parse (both tiers build a fresh `EcsDom`: tolerant `parse_html`, `crates/dom/elidex-html-parser/src/lib.rs:48-50`, through `convert_document`; strict `crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs:101`). The two are equivalent for this cell because an `Adopt` event is emitted only from the adopt walk: the one construction of `MutationEvent::Adopt` is `mutation.rs:299`, in `adopt_into`, after its `adopt_walk` call at `:297` with no return between, so 0 walks means 0 `Adopt` events (the converse does not hold — a walk from an ownerless root fires none — which makes the counter the stronger check). Commands, at `8344af2b`, run 2026-11-11: (1) every `Adopt {` token, construction or pattern: `git grep -nP '\bAdopt\s*\{' 8344af2b -- 'crates/*.rs'` → 11 hits, none in a comment: the variant's definition `crates/core/elidex-ecs/src/dom/mutation_event.rs:392`; the one construction `crates/core/elidex-ecs/src/dom/tree/mutation.rs:299`; match arms `crates/dom/elidex-custom-elements/src/consumer.rs:119`, `crates/dom/elidex-dom-api/src/range/live.rs:161`, and in `crates/core/elidex-ecs/src/dom/tests/` `adopt_notification.rs:32`, `adopt_on_insert.rs:251`, `mutation_hook.rs:149`; a test-local `Rec::Adopt` (`adopt_on_insert.rs:213`, `:252`, `:360`, `:376`). No hit outside the walk constructs the event; the command's control is its own hit on the known site `mutation.rs:299`. (2) no import that could build the variant without the `MutationEvent::` path: `git grep -nP -e 'use .*MutationEvent::[*{A]' -e 'MutationEvent as ' 8344af2b -- 'crates/*.rs'` → 0; control: `git grep -nP -e 'use .*MutationEvent' 8344af2b -- 'crates/*.rs'` hits (e.g. `crates/core/elidex-ecs/src/dom/mod.rs:33`), so the 0 is not a silent one | as stated |
| U13 | red | `adopt(template, X)` with X carrying no `AssociatedInertTemplateDocument`: afterwards X is linked to one marked I and the contents are stamped I; `adopt(div_subtree, Y)` (no template) into Y with no link: Y stays unlinked | as stated (lazy, D-C7's "iff") |
| U14 | guard (D-C6's per-pair document; rev 3.10; in `crates/dom/elidex-dom-api/src/node_methods/tests/clone_template_owner.rs`, NEW, §5.3) | template `tpl` in D whose contents hold a nested template (with a child in its contents) and a clonable shadow host (with a shadow child), built before the recorder is installed; then a recording `MutationDispatcher` (counting `MutationEvent::Adopt`) is installed and `adopt_walk_count()` (`mutation.rs:419`, `fixtures`) read; deep clone of `tpl` (`CloneNode` / `clone_node_with_shadow_honor`); the copy's nested contents and shadow tree are checked present. Positive control: afterwards, appending a node of another document into the copy's contents records 1 `Adopt` and 1 walk | 0 `Adopt` events and 0 adopt walks during the clone, on base and C1 (observed on the base, §10.8); with D-C6's per-pair document dropped (the operation's `document` in pass 3's contents-child clone or pass 4's shadow-child clone) one walk and one `Adopt` (D, I) per such child (D-C6) |

### §6.3 Mutation guards

| mutation | killed by |
|---|---|
| a new I per template (no link) | T2, U1 |
| no marker check (I of I) | T4, T5, U2 |
| caller passes the template's document unmapped (C2) | T1, T3, T6 |
| adopt re-homes contents to the target (C0's identity edge kept) | T13, U9 |
| contents children cloned with the operation's document | U14 (T10 cannot see it: each child is adopted into the copy's contents' node document at its link, D-C6) |
| shadow pass uses the operation-wide document inside copied contents | U14 (T24 cannot see it, likewise) |
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
| a contents fragment adopted as the walk's root is mapped to `f(target)` (INV-C (c) dropped: the invariant made universal) | T23 route (v), C0c U21 |

U14 is green on the base and on C1's head, so its kill of the two clone mutants above cannot be shown before C1
exists: it is confirmed red-first at C1's implementation, by running U14 against each mutant (the per-pair document
replaced by the operation's `document` in pass 3, then in pass 4) and recording both red runs in the PR description
(§9 item 5). At plan stage only its premise is verified (§10.8): the recording mechanism exists and the base's deep
clone of such a template records 0 `Adopt` events and 0 walks.

### §6.4 Kept green

Slice a's U8 (ii) (the bind pass does not walk template contents), E1–E8, C18 (T18), and the #380 template suite
(`tests_html_t2d_protos.rs` `template_*`, `dom/tests/destroy.rs`, `dom/tests/traversal.rs`), C0a–C0c's cells except
those §5.2's population marks *changes* (C0c U13's order leg stays green), D14's cells. Commands: `cargo test -p elidex-ecs --all-features`, `-p elidex-dom-api`, `-p elidex-html-parser`,
`-p elidex-html-parser-strict`, `-p elidex-custom-elements`, `-p elidex-script-session`, `-p elidex-js`, then `mise run ci`.

## §7. Out of scope and slots

| item | where | defect (stated, not the remedy) |
|---|---|---|
| insertion adoption; the node-document reader | C0, `#11-cross-document-adopt-on-insert` (prerequisite) | landed (C0b #608, C0c #611); no defect left for C1 |
| node creation outside the new-node entry (L3 Range sites A1–A9, the dom-api sites of §4.5) | `#11-ownerless-node-creation` (owner Layout C0/C1): **closed by C1** (§9 item 4, record R-4); residual A11 / A12 → L3 Range program RC-4 | none live: C0b migrated every production creation site onto the entry (§0.3 P-C23; ledger landing note of #608), C1 adds the template-contents facet, and A11 / A12 are not written yet (the method is a stub, §4.5) and are planned on the entry |
| DocumentFragment *host* of template contents (§4.12.3 establishing step 2) | `#11-template-contents-host-link` (registered, PM 505; C0c U21 pins its cycle case); §8 R-6 appends C1's line (PM 613) | the contents fragment has no host link, so host-including-ancestor checks cannot see the template |
| template `for` attribute, content patching (§4.12.3) | `#11-template-for-content-patching` (owner L3; registered by §8 R-7, PM 613) | elidex implements neither the attribute nor *prepare content patching* |
| DOM *adopt* steps 3.2 / 3.3.2 (custom element registries; 3.3.1, the attributes' node document, is modelled by C0's walk, §3) | `#11-shadow-scoped-custom-element-registry` (C0 draft §7); step 3.3.2's null-registry half `#11-document-null-registry-adopt-reset` (edge c-NR) | not modelled |
| render / layout / hit-test roots for DOMParser documents and detached subtrees with children | a facet of `#11-browsing-context-state-ecs-components` (§8 R-8, PM 613; no new slot; a landing condition of C1) | `root_entities` excludes by list, so it returns them and H-C11's consumers keep any with a child; every next kind of Document without a browsing context is a root by default |
| DOMParser documents are never destroyed | no slot found (scout Q-S1); §8 R-9 | every `parseFromString` leaves a Document (and, from C1, its I if it has a template) in the page `EcsDom` for the World's lifetime |
| template-inclusive walkers (`collect_template_inclusive_descendants`, `derive_element_components`) | umbrella §9 N-5 (slice b) | three walkers for one concept |
| multi-realm creation of I | B1 (§8 H-3, record R-12) | I is created in the single realm |

Own deferrals: **1** — C1 widens the DOMParser-leak row by one I per DOMParser document that creates a template (D-C7;
R-9). **Landing conditions: 2** — R-9 (that widening) and R-8 (rev 3.9, PM 613). Counting rule: a row is an own
deferral when C1 widens the defect's population; R-8's row is not widened (I is excluded, not admitted as a root), but
C1 extends the exclusion list that is the defect's mechanism by one, so its facet is part of C1's landing (§8 *Records*' reading, PM 618 (2)). The other
rows are pre-existing or another PR's scope.

## §8. Edges, hand-offs and records

**Edges.**
- c1: N1 implementation → C1 (T17, T22, T23's read-null legs need N1's compile gate). N1 landed (#580, `27b50dfe`).
- c3b: **C0b → C1** (semantic + code, PM decisions 143 / 153 / 244; C0 draft edge c0b; landed #608): the reader,
  insertion adoption, the new-node entry and its creation census, the hook (`attach_template_contents`'s
  `NewNodeOwner`) and the adopt walk — I's value (D-C3, D-C4, D-C5's contents segments).
- c3c: **C0c → C1** (semantic + code; C0 draft edge c0b, C0c memo edge c0b; landed #611): the segments and the
  `Adopt` consumer — the per-segment (old, new) pairs whose `new` D-C5 sets (T25).
- c-RC2a: **L3 RC-2a → C1** (code predecessor on `clone.rs`'s child-clone loop: D14 and RC-2a's cloneNode migration both
  rewrite it; PM decisions 215, 241, 249; C1 rebases on it). Landed #596 `17b2a84d`; C1's base has it (§4.4).
- c-pre: **C1-pre → C1** (semantic, PM 328; landed #616 `c7cce8c8`): C1's T5 and T1's second and third legs
  need C1-pre's D-1 (every Document wrapper has `Document.prototype`) and D-4 to D-8 (members answer from
  `this`'s entity) (header; §4.1; R-C5). The `URL` / `documentURI` legs that rev ≤ 3.7 put in T1 (legs 4–5) and in the
  since-removed §1.1 (M3) are not C1's: they rested on `#11-document-base-url`, not C1-pre (C1-pre R-2, R-10; §0.3 P-C17), and PM 604 (Q-5
  (a)) hands them to that slot as its cells (R-11); C1's `f` has no URL duty. C1's PR writes
  this edge into the umbrella's §1.3 and lists C1-pre as its §1.2 row's prerequisite (R-1; C1-pre R-2, R-5).
- c-E: **C0b** introduces the new-node entry (PM decisions 223 O-2 / 227; C0 draft D-10, §4.5 A11 / A12); L3 RC-4 (A11 / A12)
  reads it. RC-4 lands **after** C1: the Range program's definition of done ("RC-4 after N9, C0b and C1") and its §5
  RC-4 text ("PM 216 rules out the first while C1 is a dependency", the first being this memo's old "RC-4 before C1"
  branch), so RC-4's cells assert I. Whether C1 stays a dependency of RC-4 is that program's Q-Z1 ("Does C1 stay a
  dependency of RC-4?"), the PM's.
- c-NR: **C1 → L3 `#11-document-null-registry-adopt-reset`** (PM 328; the ledger trigger moved from C0c's landing to
  C1's): its scope (4) names the inert template document, which C1 creates. L3 picks the edge up from that ledger row.
- c4: **C1 → T3** (umbrella x8; T3 draft rev 4.4 §8.2: T3 relies on D-C2 to D-C7; R-A closed by C1; T3 rewrites C18).
  C1's fact is INV-C (§4.6 item 1), which is conditional. T3's plan reads the node document and cites INV-C; its R-A
  text states the fact universally and is corrected by §9 item 9.
- c5: C1 → slice b (umbrella x5; §4.6 is the provision). The same note: slice b's plan cites INV-C, not a universal
  form; for a contents fragment adopted into the bound document the spec outcome is the bound one (T23 route (v)).

**Hand-offs.**
- H-1 to slice b: §4.6 items 1–4; the umbrella's §9 N-2 / N-3 (branch B's per-node fact) do not apply in branch A.
- H-2 to C0: closed. P-C14 is folded (C0c), P-C15 is moot (C0 D-8, D-10), §4.3's constraint is met (D-20).
- H-3 to B1: create I in D's relevant realm (record R-12).
- H-4 to T3 (rev 3): T3's downstream list for this memo's owner (T3 draft §8.2 "C1 (lands before T3)") is applied by
  rev 2 (P-C11): base "N1 and C0", edge c4 (C1 → T3) replaces rev 1's c2, T18 is a guard, R-C4 unchanged at C1. T3's
  R-1 note ("done by this PR (umbrella rev 8.5)") conflicts with PM decisions 144 / 159; resolved by O-1 (a): T3 drops its C0 / C1 rows (T3 rev 4's job, not this memo's).
- H-5 to L3 (RC-2a incl. D14, RC-4): §4.4's per-pair document and §4.5's A11 / A12 contract.
- H-6 to T3 (rev 3.10.1, PM 621): at T3's next revision, T3's author replaces the universal wording in the T3 draft
  (`docs/plans/2026-10-eh-attrs-t3-target-determination.md` at `d15a141c`: R-A `:400`, A3×A11 `:446`, class I `:463`)
  with a reference to this memo's INV-C (§4.6 item 1). Wording only: T3's mechanism reads the node document (`:217`).

**Records (for the PM; exact texts).** Who applies each, and when (§9 item 10 is the same list as a DoD item):
- **by C1's PR, in place, with history** (PM decisions 144 / 159): R-1 and R-3 (umbrella), and the umbrella changelog
  lines for the C1-pre and C1 rows of §1.2 (§9 item 11; PM 604); the umbrella revision number is assigned at landing
  (PM 223 O-1 / 232). This plan PR still does **not** edit the umbrella file;
- **by C1's code**: R-5 (no ledger text);
- **by pmo at C1's landing** (ledger texts that describe the landing): R-2, R-4, R-11, R-13, R-14;
- **by pmo, landing conditions of C1** (always in pmo's application right after C1's merge; reading below): R-8 (append a facet,
  PM 613) and R-9 (register, PM 607: C1 widens it, D-C7);
- **by pmo, no ordering constraint** (PM 613): R-6 (append), R-7 (register), R-12 (append to B1's row);
- **declined**: R-10 (PM 613; pmo does not apply it).
Every record has a PM disposition (PM 604, 607, 612, 613, 618; R-1 / R-3 PM 144 / 159). pmo copies each text from this memo
as it stands on C1's merge commit (PM 613).
*Reading (PM 618 (2)).* "C1's merge commit" is the merge of C1's implementation PR; a "landing condition" is a record
that pmo's application right after that merge always includes — no registration is required before the merge.
- **R-1 umbrella §1.2** — add the row (after the C1-pre row, which C1-pre's PR added; at `c7cce8c8` §1.2's rows are
  slice a, C0, C1-pre, T3, slice b):
  "| **C1** `EH-attrs-at-creation-C1` (owner: Layout lane, this program; PM decision 123) | slot
  `#11-template-contents-owner-document`: the inert template contents owner document (HTML §4.12.3; branch A, §1.6);
  closes T3's residual R-A | IBP-sandbox N1's implementation and C0 landed; L3 RC-2a landed; C1-pre landed |".
  §1.3 gains, at C1's PR (the PR at each edge's head adds it: umbrella §1.3's closing paragraph; C1-pre R-2, R-5):
  "| x7 | C0 → C1 | semantic precondition (PM decisions 143, 153) | without insertion adoption C1 makes the most common
  template idiom worse than its base (C1 memo §1) |" and an edge row for C1-pre → C1 (id assigned at landing: the
  T3 draft rev 4.4 already uses x11 for RC-2a → T3): "| x<n> | C1-pre → C1 | semantic precondition (PM 328) | I is
  reachable from script through `ownerDocument`; its wrapper is a Document only through C1-pre's `Document.prototype`
  (C1 memo edge c-pre) |".
  §1.6's heading: "C1's handling (decided by the PM, not here)" → "C1's handling (decided: branch A, PM decision
  123)", keeping the old wording in the changelog entry (the lead already carries rev 8.4's "decided, branch A"). §8.1's `#11-template-contents-owner-document` bullet: "Owner
  TBD …" → "Owner: Layout lane (PM decision 123); C1, after C0 (PM decisions 143, 153); plan
  `docs/plans/2026-10-eh-attrs-c1-template-owner-document.md`".
- **R-2 ledger `#11-template-contents-owner-document`** (pmo, at C1's landing): "C1 plan memo as merged.
  Order (PM decisions 153, 244, 328): N1 implementation → C0a → C0b → C0c → C1-pre → C1 → T3 → slice b (supersedes this
  slot's 'N1 implementation → T3 → C1 → slice b'). C1's prerequisites: N1's implementation, C0a–C0c, L3 RC-2a, which
  carries D14 (PM decisions 215, 241), and C1-pre (PM 328). **Closed by C1** (#<n>, `<sha>`); with it T3's R-A, in the
  conditional form INV-C (C1 memo §4.6 item 1; §9 item 9)."
- **R-3 umbrella in-place corrections** (at C1's PR): cell C14's expected column "(spec: does not run; slice b inverts it
  under S-2)" → "(spec: does not run; C1 inverts it, with N1's compile gate and C0's adoption at the placement)"; K-5's
  "From N1, the DOMParser half reads null." → "From N1, the DOMParser half reads null; from C1, the template-contents half
  reads null under the C1 memo's INV-C (§4.6 item 1: conditional on the contents fragment's node document; a contents
  fragment adopted into the bound document is bound, the spec's outcome)."; §8.2 defect (5)'s "its template-contents half, and the first clause for both, remain until slice b" →
  "its template-contents half until C1 lands, and the first clause for both until slice b"; §1.4's interim line gains a
  fourth window "(iv) W-K5's template-contents half reads null from C1, the spec's outcome (K-5)"; §1.5's W-K5 bullet
  gains "(from C1, its template-contents half too)"; C18's *Spec* note "reached after T3 and slice b" is unchanged.
- **R-4 ledger `#11-ownerless-node-creation`** (pmo, at C1's landing; the close-at-C1 plan is PM 472's, "Re-eval = C1
  landing"; PM 489 gave the row its **Re-eval-date** field, 2027-01-31, as the backstop; PM 607): "**Closed
  by C1** (#<n>, `<sha>`). The `EcsDom` entry and every production creation site, both parser tiers included, landed
  with C0b (#608; PM decisions 223 O-2 / 227); C1 landed the last facet, template contents' value of the entry (the
  inert template document, through `attach_template_contents`'s second argument) and A10 (template clone). Residual:
  L3 RC-4's planned sites A11 / A12 (`createContextualFragment`'s body context element and result fragment; the Range
  program `docs/plans/2026-11-range-contents-program.md` Appendix A), which are written on the entry by construction
  and are RC-4's (L3 Range program), not a facet of this slot." (Supersedes rev 3's R-4 text, which kept the slot open
  for C1's facets.)
- **R-5 slice a's docstring** `scripting.rs:64-66` (`:73-74` at `27b50dfe`) is removed by C1 (§5.2); no ledger text.
- **R-6 to R-9: §7's known defects with no C1 slot** (PM 613 for R-6 to R-8, PM 607 for R-9; dispositions in the
  *Records* lead). Shared **Why deferred** unless a record states its own: not the contents' node document, C1's one
  fact (§1 *Split decision*). Each record's date is the fixed field (PM 489).
  - R-6 — **Apply (PM 613): append to `#11-template-contents-host-link`** (PM 505; no new slot; the proposed
    `#11-template-contents-host` is dropped). Line to append: "**Facet (C1 plan memo R-6)**: HTML §4.12.3 establishing
    step 2 creates the contents fragment with host = the template; elidex's contents fragment has no host link, so
    host-including-ancestor checks cannot see the template (pre-existing; C0c U21 pins the cycle case). **Trigger**: a
    PR that implements host-including inclusive ancestors or a fragment's host. **Re-eval-date**: 2026-12-31" (date
    approved, PM 618 (3)).
  - R-7 — **Register (PM 613)** `#11-template-for-content-patching`, owner **L3**. Text: "elidex implements neither
    the template `for` attribute (HTML §4.12.3, `#attr-template-for`) nor *prepare content patching* (HTML §4.12.3,
    `#prepare-content-patching`) (both anchors from `.claude/tools/webref dfn html for` / `dfn html 'prepare content
    patching'`, 2026-11-10). Pre-existing. **Why deferred**: neither is the contents' node document, the one fact C1
    changes (C1 plan memo §1 *Split decision*); no supported surface uses them. **Trigger**: a PR that implements
    streaming or out-of-order content patching. **Re-eval-date**: 2026-12-31".
  - R-8 — **Apply (PM 613): append a facet to `#11-browsing-context-state-ecs-components`** (owner stays that
    slot's; no new slot — the proposed `#11-root-entities-non-rendered-documents` is dropped). **A landing condition of
    C1** (C1 extends the list by one, D-C9). Reason for no separate slot: the ideal home is that slot, and a separate
    slot would make two decision surfaces for one fact (One issue, one way). Line to append: "**Facet (C1 plan memo
    R-8)**: **Defect**: `EcsDom::root_entities` (`crates/core/elidex-ecs/src/dom/tree/navigation.rs:362-374`) excludes
    roots by list (DocumentFragment; from C1 also the inert template document), so the next kind of Document without a
    browsing context is a render / layout / hit-test root by default; at `c7cce8c8` DOMParser documents and detached
    subtrees with children already are (its four consumers lay them out, paint and hit-test them). **Ideal**: the
    positive fact — the root set is the Documents that have a browsing context — which is this slot's. **Trigger**:
    B1's landing (this slot is folded into B1, `docs/plans/2026-06-agent-scoped-ecsdom-world.md` §5 req 5), or a PR
    that adds another kind of Document with no browsing context. **Re-eval-date**: 2026-12-31" (date approved, PM 618
    (3)).
  - R-9 `#11-domparser-document-lifetime` — **Register (PM 607); a landing condition of C1.** Text: "DOMParser
    documents are never destroyed, so each `parseFromString` leaves a Document in the page `EcsDom` for the World's
    lifetime; **widened by C1** with one I per such document that creates a template (C1 memo D-C7). **Why deferred**
    (its own): destroying a DOMParser document is a lifetime decision that needs a fact C1 neither has nor changes:
    whether script can still reach the Document (wrappers, ScriptSession GC coordination). C1's widening is bounded:
    at most one I per such document. **Trigger**: a PR that adds a path destroying DOMParser documents or Documents
    unreachable from script; it, and any caller of D-C7's `destroy_entity(I)`, re-examines C1 memo §4.2's live-link
    `debug_assert!`. **Re-eval-date**: 2026-12-31".
- **R-10 ledger `#11-cross-document-adopt-on-insert`** — **DECLINED (PM 613)**; pmo does not apply it. (It proposed
  replacing the closed slot's stale pointer "C1 plan memo §4.6 C0-1..C0-4 (branch `layout-eh-c1-template-owner-plan`
  @ `bc52d1b9`, not yet on main)" by "Requirements: the C0 program memo and its per-PR memos; C1 plan memo §4.7.")
- **R-11 ledger `#11-document-base-url` (Apply, at C1's landing; PM 604, Q-5 (a))** — pmo copies the quoted text into
  that slot's ledger row when C1 lands; the row's owner stays. Its fixed field (PM 489) **`**Re-eval-date**: 2027-01-31`**, proposed
  in rev 3.9, was set by PM 618 (3) and is already in the ledger in place of the struck-through ~~2026-11-01~~, so that
  field needs nothing from pmo at landing (rev 3.10). Text: "**Defect (C1, PM 604)**:
  `document_url_value` returns `navigation.current_url` for every Document
  (`crates/script/elidex-js/src/vm/host/document_accessors.rs:157-169` @ `c7cce8c8`) — an about:blank Document's
  `URL` / `documentURI` read the page URL. From C1's landing the inert template document I (HTML §4.12.3
  *appropriate template contents owner document*: a new Document, whose URL is DOM §4.5's default "about:blank") is
  reachable as `tpl.content.ownerDocument`, so the defect is observable there. **Cells this program receives** (from
  the C1 plan memo, rev ≤ 3.7's T1 legs 4–5; C1 does not assert them):
  (1) `DBU-I-URL` (VM, red until this program lands): page URL `http://example.com/page`
  (`vm.inner.navigation.current_url = url::Url::parse("http://example.com/page").unwrap()`, as
  `crates/script/elidex-js/src/vm/tests/tests_forbidden_headers.rs:228` @ `c7cce8c8`); `I = document.createElement('template').content.ownerDocument; I.URL ===
  'about:blank'` → `true` (on C1's head `false`: `I.URL` is `"http://example.com/page"`);
  (2) `DBU-I-documentURI` (VM): the same fixture, `I.documentURI === 'about:blank'` → `true` (on C1's head `false`).
  Both expected values are DOM §4.5's (a document's URL is "about:blank" unless stated otherwise)."
- **R-12 B1 (the agent-scoped `EcsDom` World, `docs/plans/2026-06-agent-scoped-ecsdom-world.md`)** — **Apply (PM 613)**:
  pmo appends one line to B1's ledger row (H-3): "HTML §4.12.3 *appropriate template contents owner document* step 1.1.1
  creates the inert template document 'given document's relevant realm'. From C1 (#<n>) elidex creates it in its
  `EcsDom`'s single realm (`appropriate_template_contents_owner_document`,
  `crates/core/elidex-ecs/src/dom/template_owner.rs`). When B1 gives one Vm several Window realms, the inert template
  document of a Document D, and its wrapper, must be created in D's realm (C1 memo §4.1, H-3)."
- **R-13 ledger `#11-tolerant-document-declarative-shadow-roots`** (Apply, at C1's landing; PM 612) — pmo appends:
  "**Cells this slot receives (C1 plan memo T12, tolerant runs; C1 asserts the strict runs only)**: (1) `TDSD-T12`
  (VM, tolerant document parse, red until this slot lands): initial HTML `<template id=t><div><template
  shadowrootmode=open><span></span></template></div></template>`; `t = document.getElementById('t'); h =
  t.content.firstChild`; `[h.shadowRoot.ownerDocument === t.content.ownerDocument,
  h.shadowRoot.firstChild.ownerDocument === t.content.ownerDocument, h.shadowRoot.ownerDocument !== document]`
  → `[true, true, true]` (today `h.shadowRoot` is `null` — no declarative shadow root is attached — so the first leg throws `TypeError`). Expected values: HTML
  §13.2.6.4.4 ("in head", start tag template, the declarative branch) attaches the shadow root, DOM §4.9 *attach a
  shadow root* step 5 gives it the host's node document, and the host is in template contents (C1 memo INV-C, §4.6
  item 1)."
- **R-14 ledger `#11-nonelementparentnode-on-fragments`** (Apply, at C1's landing; PM 618 (1); rev 3.10) — no new
  slot: the VM DocumentFragment's missing `getElementById` that rev 3.9 hit is this pre-existing slot (ledger: "Seen
  twice: EH slice a pin E7 … and scaffold cell C14"). pmo appends the facet phrase: "C1 rev 3.9 の T3 / T20 で再確認、
  content.firstChild に回避"

## §9. Definition of done

1. Coordinates re-measured at C1's base (N1 implementation, C0a–C0c, RC-2a with D14, C1-pre landed): done at `c7cce8c8`
   by rev 3.7; if main has moved when the implementation branch is cut, §5.1's and §5.2's commands are re-run there and
   any change recorded in the PR; §5.1 P-4 / P-5's base checks re-run; §5.2's population classified per test at the
   red-first run, the classification of each of the 36 fns recorded in C1's PR body (PM 604).
2. `appropriate_template_contents_owner_document`, `is_inert_template_document` and both
   components exist in `elidex-ecs` with §4.9's citations; no other site computes a template contents' node document
   (P-1 re-run); C1's diff adds no `owner_document(` call (D-C10). C1's diff adds no new-node entry, no entry branch
   and no creation-site change, and touches none of §5.3's "removed in rev 3" files beyond the comment-only rewrite of
   `tree_builder/shadow.rs:118-119` (§5.1 P-7; within this item, PM 604) (`git diff <base>..HEAD` on them
   shows comment lines only);
   C0's P-3 and P-8 ratchets (C0b memo §5.1) still hold at C1's head.
3. D-C3 to D-C9 implemented; the clone-policy table has both rows.
4. **`#11-ownerless-node-creation` closes at C1's landing** (the close-at-C1 plan is PM 472's, "Re-eval = C1 landing";
   PM 489 is the source of the row's **Re-eval-date** field, the backstop; PM 607): C1 lands its last facet
   (template contents, A10); the residual A11 / A12 are L3 RC-4's sites in the Range program (§4.5, §7); pmo applies
   record R-4's close text.
5. §6.1 and §6.2 cells present; every red cell run red on the base with its control green, recorded in the PR
   description; T9 green at the head. U14 (a guard, green on base and head) run red against each of §6.3's two clone
   mutants (pass 3, pass 4) at C1's implementation, recorded in the PR description (§6.3; rev 3.10).
6. §5.2 rewrites done; P-7's grep returns no hit in `crates/`. No stale cite (by §5.3 *Stale cites*' property: a § or
   step that moved per §10.5 F2) in the files C1 touches: `dom/mod.rs:469` fixed (the one at `c7cce8c8`, verified
   2026-11-10), and `create_document_root`'s closed-slot pointer (`dom/mod.rs:453-455`) re-pointed (§5.3); the rest of
   `crates/` is the HTML renumber sweep's, PM 607.
7. `cargo test` for the seven crates of §6.4 and `mise run ci` green.
8. `python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-10-eh-attrs-c1-template-owner-document.md`
   exits 0 (run; do not trust a stored verdict).
9. **T3's residual R-A is closed by C1 (PM decision 153), in its conditional form.** R-A (T3 draft rev 4.4 §1.2):
   "Every node in template contents or in a clone of them has I as its node document (H-T21), so step 3 returns null
   there". Stated universally it is false on the spec (INV-C (c)). C1 closes it in the form of **INV-C (§4.6 item 1)**;
   T3's R-A text is to cite INV-C rather than restate it (edge c4). Its rev 3 listed the ways a node gets there — parsed (C1 D-C3, D-C4), appended into `tpl.content` (C0's adoption), cloned from
   contents (C1 D-C6), or cloned with a template. Cell
   **T23** pins all four routes at C1's head — node document I, not `document`; on the appended route also a raw handler
   reading `null` (N1) — and route (v), the control for the condition (contents adopted into `document`: node document
   `document`, the handler compiles) —
   with T19 / T12 (parsed, both tiers, foreign and shadow content), T8 (appended), T9 / T10 (cloned) and T24 (cloned with
   a shadow host) as the per-route mutation guards, and U14 for the clone routes' per-pair document (D-C6), which
   T10 / T24 cannot see. T3's target cells for R-A (T-17, EW-13, EW-14, XW-8) are T3's to
   assert on top of these facts (T3 lands after C1).
10. Records, per §8 *Records*' list, every one with a PM disposition (PM 604 / 607 / 612 / 613 / 618), texts copied by pmo
    from this memo as it stands on C1's merge commit: R-1 and R-3 applied by C1's PR (with item 11); R-2, R-4, R-11
    and R-13 applied by pmo at C1's landing; **R-8 (facet appended to `#11-browsing-context-state-ecs-components`,
    PM 613) and R-9 (registered, PM 607) always in pmo's application right after C1's merge** (landing conditions, in
    §8 *Records*' reading, PM 618 (2)); R-14 applied by pmo at C1's landing (PM 618 (1)); R-6, R-7 and R-12
    applied by pmo (PM 613); R-10 declined (PM 613). Decided: O-1 ((a), PM 223 O-1 / 232), O-2 (PM 223 O-2
    / 227), O-3 (PM 146 / PM 223 O-3), Q-5 ((a), PM 604).
11. C1's PR adds the umbrella's changelog lines for §1.2's C1-pre row (missing since C1-pre's PR) and C1 row, one line
    each, with R-1 (PM 604).

## §10. Revision history / dispositions

The rev 3 table (each rev 2 item moved to C0 rev 2, and what C1 kept) and the rev 2 table (driver brief inputs 1–9
and their dispositions) are history: `git show <sha>:docs/plans/2026-10-eh-attrs-c1-template-owner-document.md`
§10.0 / §10.1, with `<sha>` = `git log --format=%h -1 --grep='C1 plan-memo rev 3.1' -- <memo>` on this branch (after the
squash merge, from the plan PR's head ref). Rev 3.2's alignments are the header changelog's.

### §10.2 Rev 1 (2026-10-31)

First draft, pre-plan-review. Its O-1 (C0 first) is decided by PM decisions 143 / 153; its O-2 (umbrella vehicle) by
PM decisions 144 / 159 (sequencing: O-1 below, decided rev 3.1: (a), PM 223 O-1 / 232); its O-3 (URL resolution) became §1.1 (removed in rev 3.8: the spec value is the expectation, R-C2, T20); its O-4 (split) is O-3
below; its Q-1 (shared node-document helper) is resolved by C0's `node_document` and D-C4's entry; its Q-2 is Q-2 below;
its Q-3 (T18) is resolved (T18 is a guard at C1).

### §10.3 Rev 3.7 (2026-11-10)

Why: (1) the C1-pre memo's record R-2 to this memo (C1-pre §8; it follows PM 339's URL ruling);
(2) every prerequisite has landed (#580, #587, #595, #596, #599, #600, #608, #611, #616), so the frame moves from
`593aa314` to `c7cce8c8` = C1's base and the drafts' "on the base" statements are checked against landed code.

- **R-2 (C1-pre):** T1's second leg is `Object.getPrototypeOf(I) === Object.getPrototypeOf(document)` (P-C18); T1's
  fourth / fifth legs and M3's `URL` / `documentURI` legs rest on `#11-document-base-url` (P-C17; Q-5); edge c-pre's
  "I gets C1-pre's default about:blank at creation" withdrawn and the edge restated; §4.1, §4.8, §5.3 say the VM
  wrapper work is C1-pre's; R-C5 per C1-pre §4.3; Q-S10 answered by C1-pre §5.1 P-4 / P-11; H-C10 lists both Document
  spawn sites (P-C19); R-1 adds C1-pre to C1's §1.2 prerequisites and the C1-pre → C1 edge (with x7) to §1.3.
- **Re-measure:** header sibling statuses and *Prerequisites*; §0.1 frame; §0.2 rewritten at `c7cce8c8` (every
  H-C* row); §0.3 P-C3 / P-C11 / P-C12 status and P-C17 to P-C27; D-C3, D-C4, D-C5 (the adopt walk's shape, P-C20),
  D-C10 (the `-w` count); §1.1 lead and M3; §3 code column; §4.3, §4.4 (D14 and the hand-stamp removal landed),
  §4.5 (P-C23), §4.7; §5.1 (the `prod` filter, P-C21; P-2, P-3, P-6, P-7 redefined by property, with outputs);
  §5.2 (code-side population, 36 fns; C0c U21 added to the memo-cell population, "not dependent"); §5.3 line counts;
  §6 columns, T1, T22; §7 rows (insertion adoption landed; the host row's slot `#11-template-contents-host-link`);
  §8 edges c3b, c3c, c-RC2a, c-pre, c4 and R-1, R-5, R-6; §9 items 1, 2, 9, 10; §11 O-3, Q-5, Q-S10, Q-S11.
- **Not changed:** every D-C* decision, every §2 pair, every §6 cell's expected value except T1's second leg's
  expression (the expected values of T1's legs 4–5 are Q-5's), every spec citation's § ↔ title pair (the ones the
  rev touches — DOM §4.5 *adopt* steps 3, 3.3.3, 3.4 and `adoptNode()` steps 1–4, HTML §4.12.3 template *adopting
  steps* 1 — re-read with `.claude/tools/webref body dom concept-node-adopt`, `body dom dom-document-adoptnode`,
  `body html the-template-element` and `heading dom 4.5`, 2026-11-10). **Erratum (rev 3.8):** only those pairs were
  re-read; "every pair unchanged" did not hold for the others (§10.5).

### §10.4 Rev 3.7.1 (2026-11-10, PM 604)

Why: PM 604 answers Q-5 with (a) and approves rev 3.7's two designer choices.
- **Q-5 (a):** T1 loses legs 4–5 and the page-URL fixture only they read (script, base `[false, true, false]`, C1
  `[true, true, true]`); M3 loses its `URL` / `documentURI` legs; the four legs become `#11-document-base-url`'s
  cells through record R-11 (§8, Apply at C1's landing). Swept: header changelog; H-C16; §0.3 P-C17; §1.1 M3; §4.1;
  §5.1 P-6; §5.4 R-C5; §6.1 T1; §8 edge c-pre; §9 item 10; §11 Q-5. §6.3's guards name T1 only for "caller passes the
  template's document unmapped", which leg 1 still kills; no guard named legs 4–5. §1.1 M3's "reading" note (after the
  table) concerns its non-URL members and is unchanged.
- **Approved choices:** the comment-only edit to `tree_builder/shadow.rs:118-119` is within §9 item 2 (stated there);
  §5.2's 36 fns are classified at C1's red-first run and the result goes in C1's PR body (§5.2, §9 item 1).
- **Umbrella changelog:** the line for C1-pre's §1.2 row was missing; C1's PR adds it with C1's own (§8 *Records*
  lead, §9 item 11). The umbrella is not edited by this plan PR.

### §10.5 Rev 3.8 (2026-11-10, PM 607)

Why: plan-review R1 at rev 3.7.1 (findings F1–F16); PM 607 rules F1 and approves the other dispositions. PM 145
withdrawn by PM 607.
- **F1 (PM 607):** the measurement step is gone. §1.1 is removed entirely, not kept as a spec table: its rows
  duplicated §6's cells (M1 ↔ T1, M2 ↔ T2, M6 ↔ T9, M7 ↔ T10, M8 ↔ T6, M9 ↔ T11, M10 ↔ T4, M11 ↔ T22 / T17, M12 ↔
  T12, M13 ↔ T25), and the derivations other sections cited moved to where they are cited (M3 → R-C5, M4 / M5 → R-C2
  and T20, M13 → R-C8). No row was spec-silent: every value was derived from cited steps (step (1) of the order of
  grounds decided each), so none needed step (2). M5's `img.src` leg could not discriminate (elidex's `src` getter does
  not resolve, R-C2; rev 3.9: `#11-url-reflecting-attribute-getters-unresolved`, PM 610), and is not carried. Swept: header *Placement* (PM 145 bullet → PM 607) and changelog; D-C11
  deleted; §1 K-0 lens; §3 row §2.4.3; §3.1; §5.4 R-C2, R-C5, R-C8; §6 lead; T1, T20 (expected `"x.html"`, kind red),
  T25; §8 R-11 (cells (3) / (4), the measurement form of M3's URL legs, dropped; the two VM cells stay); §9 item 4
  (replaced, F7); §10.2; §11 Q-5; §0.3 P-C17 (note). History lines (changelog, §10.3, §10.4) keep their M-row
  mentions as history.
- **F2:** every § ↔ title pair and step re-looked-up (§3 lists the commands). Changed: HTML §8.5.4 → §8.5.5 (The
  innerHTML property); §8.5.7 → §8.5.8 (The createContextualFragment() method; steps 1, 6 unchanged); *create an
  element for a token* step 3 "intendedParent's node document" → step 6 "targetParent's node document" (H-C4, §2.2
  A2×A3, §3); the *HTML fragment parsing algorithm* is rewritten upstream: old steps 14, 15, 22 → steps 2, 6, 7 plus
  *initialize an HTML fragment parser* steps 12–13, 15 (the insertion target redirection map; §3 row §13.4); DOM
  *adopt* step 3.3.1 (attributes' node document) is modelled by C0's `stamp_node_and_attrs` and changed by C1 (§3,
  §4.9, §7); *clone a single node*'s Document branch is 5.1–5.3 (§4.9). Unchanged on re-lookup: every other heading,
  dfn and step in §3's list. §0.1 and §10.3's "unchanged" claim corrected.
- **F3:** §3 row §13.2.6.1 gains step 10 (sanitizer / isDropped), not modelled, Full enum ✗; isDropped's
  reachability is Q-S13.
- **F4:** step 1.1.2 always applies and holds by construction (D-C2, §3, §4.9).
- **F5:** D-C9 kept (exclusion form named as interim); R-8's defect, ideal home, reason and trigger restated.
- **F6:** the contents-document fact is conditional (header *Scope* item 3; D-C4; §2.1 A3, A4, A6, A8, A9; §2.2 A1×A9; §4.5; §4.6 item 1; §8 R-3; §9 item
  9); T23 route (v) added as the control, with a §6.3 guard; edges c4 / c5 carry the no-universality note.
- **F7:** `#11-ownerless-node-creation` closed by C1, residual A11 / A12 to L3 RC-4 (§4.5, §7 row 2, R-4, §9 item 4).
- **F8:** R-9's trigger narrowed, own Why-deferred, registration a landing condition (§8, §9 item 10).
- **F9:** Q-S11 closed (§5.1 P-6, §11). **F10:** R-12 (B1). **F11:** D-C5's per-node segment bookkeeping is an
  implementation obligation with its cost; §5.3's `mutation.rs` row. **F12:** O-3 decided (PM 146 / PM 223 O-3; §5.3,
  §9 item 10, §11). **F13:** §8 *Records* lead lists who applies each record; §9 item 10 matches. **F14:** §5.3 rows
  for `tests_event_handler_attrs_once.rs` and both test `mod.rs` registrations. **F15:** R-2 restated. **F16:** R-10
  narrowed to the stale pointer.
- **PM 607 (stale §8.5 cites):** §5.3 note and §9 item 6: C1 fixes the ones in its touched files (0 at `c7cce8c8`);
  the rest are the HTML renumber sweep's.
- **Reviewer coordinates corrected:** the non-template `lookup` call is `mutation.rs:392` (not `:391`); `lookup`
  itself is `:719-724`.

### §10.6 Rev 3.8.1 (2026-11-10)

- Q-S13 closed (PM): isDropped is structurally false in both tiers and there is no sanitizer, so step 10 is
  unreachable; §3 row §13.2.6.1 keeps ✗ with that reason, no slot (§11 Q-S13 has the commands).

### §10.7 Rev 3.9 (2026-11-10, PM 612 / 613)

Why: plan-review R2 at rev 3.8.1 stopped on a generator: the contents-document fact was restated at many sites, and
each round found a restatement the previous sweep missed (R1 F6 made some conditional; R2 found §2.1 A2, A5, five
§2.2 pairs and §4.3 still universal, and under T23 route (v) A2 and A5 are spec-false). PM 612 (i) rules the root fix;
PM 613 rules §8's register-or-decline records.
- **Root fix (PM 612 (i)).** INV-C is stated once, at §4.6 item 1, with its condition derived as (a) a node of a contents
  fragment F's tree has F's node document, (b) F's is `f(template's)` from establishment and after every adopt that
  reaches it as a segment, (c) it differs only after a script adopts F itself. Sites found by concept and converted to
  references (each keeps its own mechanism text): header *Scope* item 3; D-C3 (nested template); D-C4 bullets 1 and 3;
  §2.1 A2, A3, A4, A5, A6, A8, A9; §2.2 A1×A5, A1×A9, A2×A3, A2×A4, A2×A9, A3×A4, A3×A5, A3×A8, A4×A5, A4×A8, A5×A6,
  A5×A8, A5×A9, A6×A9, A8×A9; §3 row §8.5.5; §3.1; §4.3; §4.5 (A11 / A12, A1–A9); §5.4 R-C1, R-C2, R-C3; §6.1 T23's
  route (v) note; §6.3's last guard; §8 c4, c5, R-2, R-3 (K-5's text); §9 item 9; §11 Q-S11. Left as written: §0.3
  P-C25 and §9 item 9's quote of T3's R-A (quotations of another memo), §6 cell expectations (fresh templates, where
  INV-C's condition holds by construction), §10 history.
- **T20 / T9 (Ax2).** elidex never writes the page URL into `DocumentBaseUrl` (R-C2), so neither cell could go red;
  both now carry a `<base href>` fixture (T20 shaped like `tests_base_href.rs`). **Red-first check of T20's base
  column**, run at `c7cce8c8` in this worktree with a throwaway test appended to
  `crates/script/elidex-js/src/vm/tests/tests_base_href.rs` (`build_fixture` + `bind_vm`; `<base
  href="http://example.com/dir/">` appended by script; `tpl.innerHTML = '<a id=a href=x.html></a>'`) and then removed
  (not committed): `unset RUSTUP_TOOLCHAIN; rustc --version` → `rustc 1.99.1`; `cargo test -p elidex-js --all-features
  zz_throwaway_c1_t20_base_side -- --nocapture` → `document.baseURI` = `http://example.com/dir/`, T20's read =
  `"http://example.com/dir/x.html"`, T9's detached clone and after-append reads = `"http://example.com/dir/x.html"`,
  `a.ownerDocument === document` = `true`. The same run showed `tpl.content.getElementById` throws `TypeError: not a
  function` (the VM's DocumentFragment has no `getElementById`), so T3's and T20's scripts use `content.firstChild`.
  (Rev 3.10: the pre-existing slot `#11-nonelementparentnode-on-fragments`, PM 618 (1); facet record R-14.)
  R-C2 re-derived from HTML §2.4.3 *document base URL* steps 1–3 → *fallback base URL* steps 1–2; §1 row 2's premise
  and §3 row §2.4.3 follow.
- **§5.2 (Ax2).** C0c U21 → "changes" (root pair `(f(docs.x), docs.d)`); C0c U18 → "value unchanged" (establishing
  stamp changes, the walk order does not); C0b T23 and U12 stay "not dependent" with the reason (no template; the
  population's grep matches "Contents" / "textContent").
- **MINs.** Ax1: §4.5 names `RangeState`'s `create_contextual_fragment` as the engine-independent site. Ax3: R-C2 / §10.5
  F1 cite `#11-url-reflecting-attribute-getters-unresolved`; T12's tolerant legs → R-13; R-8 a landing condition
  (§7 counting rule); `dom/mod.rs:453-455`'s closed-slot pointer and `:469`'s "step 4" are C1's fixes (§5.3, §9 item
  6); §7's adopt row cites `#11-shadow-scoped-custom-element-registry`. Ax4: §4.9 rows for
  `clone_node_with_shadow_honor`, `TemplateContents`, `AdoptedContents`, `scripting.rs:64-66`; the stale-cite
  obligation is by property (§5.3 *Stale cites*); §4.4 lead. Ax5: §5.1 P-6's four lines outside Q-S4's population
  classified; R-4 / §9 item 4 cite PM 472 (plan) and PM 489 (field); R-6 to R-9 and R-11 use `**Re-eval-date**`;
  R-11 proposes 2027-01-31 for `#11-document-base-url`.
- **PM 613.** R-6 appends to `#11-template-contents-host-link`; R-7 registers `#11-template-for-content-patching`
  (owner L3; `#attr-template-for`, `#prepare-content-patching`); R-8 appends a facet to
  `#11-browsing-context-state-ecs-components` (the proposed ID dropped; a landing condition; D-C9, §7, §9 item 10);
  R-10 declined; R-12 applied as proposed; §8 *Records* lead and §9 item 10 list every disposition.
- **Reviewer coordinates corrected:** the AttributeChange arm's `about_blank_url()` is `document_base.rs:365` (not
  `:366`); the relative-href pin is `tests_html_t2a_protos.rs:186-199` (not `:186-197`).

### §10.8 Rev 3.10 (2026-11-10, PM 619 / 618)

Why: plan-review R3 at rev 3.9 (G1–G17: IMP 2, MIN 15; no INV-C restatement found); PM 619 rules the dispositions,
PM 618 three ledger points. Every reviewer coordinate was re-read at `c7cce8c8` before use.
- **G2 (IMP; D-C6, §6.1 T24, §6.2 U14, §6.3, §9 items 5 and 9, §2.2 A1×A5 / A3×A5 / A4×A5, §5.3).** The clone links
  each copied child by `append_child`, which adopts into the parent's node document first, so a clone that drops
  D-C6's per-pair document reads the same node documents (T24 under that mutant is `[true, true, true]`, not
  `[false, false, false]`; T10 likewise) and differs only by an adopt walk and an `Adopt` (D, I) per child. New guard
  cell **U14**; the two §6.3 rows point to it. **Premise check at `c7cce8c8`** with a throwaway test appended to
  `crates/dom/elidex-dom-api/src/node_methods/tests/clone_shadow.rs` (a template in D whose contents hold a nested
  template with a `<p>` and a clonable shadow host with an `<i>` shadow child; a `MutationDispatcher` counting `Adopt`
  installed after the build; `adopt_walk_count()` around a deep `CloneNode`; then a cross-document append into the
  copy's contents as positive control), removed afterwards (`git status` clean): `unset RUSTUP_TOOLCHAIN; rustc
  --version` → `rustc 1.99.1 (58cf8f9c3 2026-10-13)`; `cargo test -p elidex-dom-api --all-features
  zz_throwaway_c1_g2 -- --nocapture` → `adopt_events=0 adopt_walks=0 nested_contents_children=Some(1)
  shadow_children=Some(1); control append: events=1 walks=1` (1 passed). The mutant-kill is confirmed red-first at
  C1's implementation (§6.3, §9 item 5).
- **G1 (IMP; §7, §8 *Records* lead, §9 item 10).** PM 618 (2)'s reading stated once in §8: a landing condition is in
  pmo's application right after the merge of C1's implementation PR.
- **G3** §4.5 (the VM native throws itself today; RC-4's planned shape marshals). **G4** R-C2 scoped to the bound
  document, DOMParser named as the page-URL writer (HTML §8.5.1 step 3), §3 row §8.5.1, §3.1, T20's note, new red cell
  **T29** (it needs a page-URL fixture: the VM's default `current_url` is `about:blank`). **G5** INV-C (b) covers the
  ownerless template (the only INV-C edit; no restatement elsewhere). **G6** §6.1 lead states the element-access
  rule; scripts changed: T3, T4, T7 (unused id dropped), T10, T11, T12 (and its variant), T13, T14, T19, T22, T23
  (routes (i), (iii), (iv)), and record R-13's copy of T12. **G7** T3, §6.1 lead, §10.7 cite the slot; record R-14.
  **G8** R-7's quote has its Why deferred. **G9** D-C2. **G10 / G14** §5.3 *Stale cites*: the widened seed adds no
  hit (21 + 1), the excluded files are named, `tests_html_t2d_protos.rs:700` and `range_proto_mutation.rs:248` go to
  the HTML sweep, and the DOM part outside touched files is 0 (command there). **G11** §1. **G12** §5.1 P-6, §11
  Q-S11. **G13** §5.3 `dom/mod.rs` row, §4.9 two rows. **G15** §4.1, §5.3, R-C5. **G16** R-11, §11 Q-5. **G17** R-8's
  trigger is B1's landing.
- **PM 618.** (1) = G7 (R-14); (2) = G1; (3) R-11's date already applied (G16), R-6 / R-8 dates marked approved.
- **Reviewer claims corrected:** G4's "no fixture needed" — T29 needs the page URL set (`crates/script/elidex-js/src/vm/host/navigation.rs:201` defaults
  `current_url` to `about:blank`, under which base and C1 both give `"x.html"`). Every other G1–G17 coordinate held
  at `c7cce8c8`.

### §10.9 Rev 3.10.1 (2026-11-10, PM 621)

Clerical, PM 621: (IMP) §2.1 rows A2 and A5 no longer copy INV-C (b)'s establishment clause; they reference it.
(MIN 1) §4.6 item 1 INV-C (a) is scoped to an F that has a node document (coordinates re-read at `c7cce8c8`).
(MIN 2) §8 R-2's quote names "the C1 plan memo as merged", not a revision. (MIN 3) §6.1 T29 and §10.8: full path
`crates/script/elidex-js/src/vm/host/navigation.rs:201` (`current_url: parse_about_blank()` at `c7cce8c8`).
(MIN 4) §5.3 and §6.2 name where U12 and U14 live. (MIN 5) §8 H-6 to T3 (coordinates at `d15a141c`).

### §10.10 Rev 3.10.2 (2026-11-11, PM 647)

Erratum of observations, PM 647: the C1 implementation found five cells or checks whose stated observation could not
show what the memo claims. The mechanism, INV-C (§4.6 item 1) and every expected C1 value are unchanged; only what is
observed changes. A focused reviewer re-check of these five follows.
- **§6.1 T13.** On the base `tpl.content.ownerDocument` is `document`, so every `=== tpl.content.ownerDocument` leg was
  true on the base and the cell was not red. New leg `t.content.ownerDocument !== document` (base false, C1 true),
  grounded in HTML §4.12.3 *appropriate template contents owner document* steps 1.1.1, 1.1.3, 1.2, 2 and the template
  *adopting steps* steps 1–2; base and C1 columns now give the leg vector. Matches
  `crates/script/elidex-js/src/vm/tests/tests_template_owner_document.rs` `t13_adopted_template_contents_go_to_the_inert_document`.
- **§6.1 T25.** Same shape: on the base `I_dp` is `dp` and `I` is `document`, so the identity legs held on the base.
  New legs on the logged values, `log[0][1] !== dp` and `log[1][1] !== document` (base false, C1 true), same grounding;
  the read vector is the test's (`t25_adopted_callback_pairs_in_contents`).
- **§6.2 U12.** A recording `MutationDispatcher` cannot be installed before a document parse (each tier builds a fresh
  `EcsDom`). U12 reads C0b U8's walk counter instead; the equivalence ("an `Adopt` event is emitted only from the adopt
  walk") is stated in the cell with its `git grep` population and controls at `8344af2b`. Matches
  `crates/dom/elidex-html-parser/src/tests_template_owner.rs` `u12_document_parse_creates_contents_in_the_inert_document`.
- **§6.2 U5.** "created lazily or already there" read two ways; the cell now states the precondition (D's I created
  before the parse) and the two exact counts, as `u5_strict_rollback_with_a_template_leaves_the_dom_as_it_was`.
- **§5.1 P-4 / P-5.** The base check's `git grep` returned 7 (all in test files), not 0; the command now excludes test
  paths (→ 0 at `8344af2b`, 2026-11-11) and keeps the unfiltered 7 as its control.
- Not changed: §5.3's C14 test name (the implementation's rename is reverted in code).

## §11. Open questions

**For the PM / user.**
- **O-1 Umbrella edit sequencing (P-C16).** **Decided rev 3.1: (a)**, by PM decision 223 O-1, confirmed for C1 by PM
  decision 232: one owner per §1.2 row; each PR adds only its own, in landing order; the umbrella revision number is
  assigned at landing; it covers C1's row (R-1) and R-3's texts; the T3 draft drops its C0 / C1 rows (T3 rev 4's job).
  The options as asked are in the rev 3.1 text (§10).
- **O-2 Which PR adds the `EcsDom` new-node entry (P-C13).** **Decided (rev 3): C0** (now C0b), by PM decisions 223 O-2
  / 227; C1 supplies only the template-contents value through `attach_template_contents`'s second argument (D-C3, D-C4).
- **O-3 Split.** **Decided** by PM 146 (a touch-time, move-only split of `dom/mod.rs` if it exceeds ~990 lines) and
  PM 223 O-3 (relayed): the template-contents helpers then move to `template_owner.rs` as a standalone pure move
  first. At `c7cce8c8` it is 949 (§5.3), so the rule fires only if main grows it by ~41 lines before the cut.
- **Q-5 What T1's legs 4–5 and M3's `URL` / `documentURI` legs assert on C1's head (rev 3.7; §0.3 P-C17).**
  **Answered by PM 604 (rev 3.7.1): (a).** T1 has three legs (`I !== document`, the prototype leg, `defaultView`);
  T1's old legs 4–5 and M3's `URL` / `documentURI` legs are `#11-document-base-url`'s cells `DBU-I-URL`,
  `DBU-I-documentURI` (rev 3.8: the two measurement cells `DBU-I-URL-M` / `DBU-I-documentURI-M` are dropped with PM
  145, PM 607), handed over by record R-11 (§8), which states the defect; the slot's owner stays, and its Re-eval-date is 2027-01-31 (PM 618 (3), already in the ledger; R-11). Rejected: **(b)** (keep the legs asserting the page URL as a
  known-defect pin) fixes a spec-wrong value as a test expectation, a control blessing the defect; **(c)** (the URL
  facet as C1's prerequisite) binds C1, T3 and slice b to a program with no plan. The options as asked are in the
  rev 3.7 text (`git show a36571e9:<memo>` §11).

**For plan-review.**
- **Q-1** (the entry's signature), **Q-2** (the adopt walk's shape) and **Q-3** (`adoptedCallback` arguments in
  contents) are answered by C0: Q-1 by D-10 / D-17 (one `source`, a `NewNodeOwner`, the redirection by
  `FragmentParseRoot` on the fragment-parse root); Q-2 by D-3 (c) (the `Include` walk with one contents edge; no walker
  variant; D-19 reorders it); Q-3 by D-6 (C1 sets the segment's `new`). §4.3's constraint is met by D-20.
- **Q-4 Decided (plan-review R1):** `clone_subtree`'s override is a `NewNodeOwner`. It is C0b's surface, so it is a C0b
  erratum made in the C0b memo, not here; §4.4 steps 2–3 pass `node_document_for_new_nodes(..)`. §4.4's representation
  choice (tuple field or read-back) stays open.

**Scout questions** (facts this memo needs; not read here).
- **Q-S10** Closed: a non-bound Document wrapper's surface is C1-pre's (edge c-pre), answered by the C1-pre memo §5.1
  P-4 (every Document member by fact source) and P-11 (every per-entity Document fact a member reads, with its writer,
  reader or slot).
- **Q-S11** Closed (rev 3.8): `crates/dom/elidex-dom-api/src/focus/delegate.rs:86`'s node-document read does not
  change the outcome for a delegates-focus host in template contents. On C1's head `node_document(target)` is INV-C's
  value (I for a fragment not adopted out; for one adopted out the base's reasoning below applies unchanged) and
  `current_focus(dom, I)` is `None` (`focus/sot.rs:52-54`: the focused element is not in I, `is_in_document`); on the
  base it is the document, but a target in contents cannot be a host-including inclusive ancestor of the focused
  element (contents are not connected and the walk does not climb from a contents fragment). Both reach HTML §6.6.4 *get the
  focusable area*'s delegates-focus shadow host branch, step 3 (the focus delegate). (The "6.3" of rev ≤ 3.9 is
  elidex's own branch numbering, `focus/delegate.rs:74-93`; the spec's list of branches is unnumbered.)
- **Q-S13** Closed (rev 3.8.1, PM): not reachable, no slot. The question (rev 3.8, F3) was whether HTML §13.2.6.1
  *create an element for a token* step 3's isDropped ("true if the parser's fragment parser abort requested is true,
  or if referenceChild is non-null and referenceChild's parent is not targetParent") can be true, making step 10 put
  the element in the inert template document. Both halves are false in both tiers at `c7cce8c8`: no abort flag
  (`git grep -n -iE 'fragment.*abort|abort.*fragment' -- crates` → 0); strict computes no referenceChild
  (`crates/dom/elidex-html-parser-strict/src/tree_builder/insert.rs:30-38` returns a parent only), creates and appends
  in one step (`insert.rs:61-66`, `:106-117`), has no foster parenting or adoption agency (`src/lib.rs:4-5`,
  `tree_builder/mod.rs:12-13`, `modes/in_body.rs:5-8`) and runs no script during the parse; tolerant converts only a
  finished html5ever `RcDom` (`crates/dom/elidex-html-parser/src/lib.rs:49-50`, `convert.rs:1-4`, `convert_node`
  `:244`), so nothing moves between placement and creation at elidex's layer. The step-10 sanitizer clause has no
  input either (`git grep -n -i sanitiz -- crates/dom/elidex-html-parser-strict crates/dom/elidex-html-parser` → 0).
  §3 row §13.2.6.1 states the reason.
- **Q-S12** Closed. First half: a tolerant **document** parse attaches no declarative shadow root
  (`#11-tolerant-document-declarative-shadow-roots`; the fragment path passes the flag), so T12's DSD legs are strict
  only. Second half: answered in rev 3 (§4.4).
