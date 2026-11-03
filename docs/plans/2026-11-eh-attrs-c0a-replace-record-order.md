# C0a `EH-attrs-at-creation-C0a` — the *replace* mutation records follow the current step order (per-PR plan memo)

Per-PR slice **C0a** of the C0 program (program memo `docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md`, rev 3.2;
"the program memo"), under the umbrella `docs/plans/2026-10-event-handler-attrs-at-creation.md`. It carries program
decision **D-12** (PM decision 160 (a); "in the C0 program", PM 244) after its prerequisite pure-move split **D-14 (a)**. It is
the first program PR to land and adds the umbrella's C0 program row, rewrites its §1.2 order line (program D-9,
record R-3; PM 227 O-1) and writes that rewrite's immediate consequences in the umbrella (PM 271, amending PM 223 O-1;
§8). Siblings and IDs: the program memo's header; program IDs (H-*, P-*, D-*, A*, R-*) are cited as the program memo's.

**Placement.** PM 160 (a): the replace record order is fixed in the C0 program; PM 160 (b): `mutation/mod.rs` is split
first; PM 244: C0a = D-12 alone, after D-14 (a); C0a may land any time before C0b (program §1).

**Scope.** `apply_replace_child`'s non-fragment arm captures the source-removal record's siblings with `old_child` still
in place (DOM *replace* step 6 adopts node — removing it from its old parent — before step 7 removes child); every
*replace* step comment in the function, in the dom-api `ReplaceChild` handler and in the replace tests is renumbered to
the current steps 1–11, except the fragment arm's step 6, whose code arrives with C0b (program D-5).

**Not in scope.** The fragment arm's adoption (C0b, D-5); insertion adoption itself (C0b); `replaceChild(X, X)`'s
records (`#11-replace-child-self-records`, registered PM 227); `adoptNode`'s removal record (C0c, D-13).

**Changelog.** *Rev 3.2* (2026-11-03, with the program memo's rev 3.2; plan-review of C0a at `4c79d9c8`, 0 CRIT / 3 IMP /
15 MIN, and PM 271 / 273): the two existing tests that pin the stepped-over siblings are U14, rewritten to the spec's
values (Ax3 IMP-1); D-12's renumbering is a per-line table (Ax4 IMP-1, MIN-1, MIN-2); the source capture becomes
`capture_move_source` (Ax2 cross-axis note); T17b (`replaceWith`) and U14c (deferred flush) added (Ax2 MIN-1); the
reversed B1.2a decision cited (Ax5 MIN-4); the umbrella consistency edits of PM 271 (§8; Ax5 IMP-1, MIN-1–3); the
dispositions: §10. *Rev 3.1* (2026-11-03, with the program memo's rev 3.1; plan-review R2 of the program): the umbrella
order-line rewrite (PM 227 O-1) added to this PR's records (§8, DoD 5); `#11-replace-child-self-records`'s measurement
procedure restored here (§7; R2 Ax5 IMP-4). *Rev 1* (2026-11-03): first draft, carved from the C0 memo rev 2.1 (`d262ca8c`) D-12 / D-14 (a) / T17 /
U14 / P-9 per plan-review R1's SPLIT and PM 244.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Coordinates are `27b50dfe` (program §0.1). C0a's base is `origin/main` with D-14 (a) landed, which moves the child-list
family of `crates/script/elidex-script-session/src/mutation/mod.rs` (`:296-895`) into `mutation/child_list.rs` (NEW);
every `mutation/mod.rs` coordinate at `:296-895` below is re-measured there at C0a's base (§9 item 1) by symbol
(`apply_replace_child`) and by the quoted comment text, which a pure move keeps. `apply_mutation` (`:188`) stays in
`mod.rs`.

### §0.2 Premises

| id | claim | evidence (`27b50dfe`) |
|---|---|---|
| H-7 | The record layer models adopt step 2 | `capture_move_source` / `source_removal_record` / `move_record_list` (`crates/script/elidex-script-session/src/mutation/mod.rs:304-384`) |
| H-10 | The primitive detaches `new_child` before `old_child` | `replace_child` (`crates/core/elidex-ecs/src/dom/tree/mutation.rs:138`): `detach_with_hook(new_child)` `:164`, `detach(old_child)` `:186`, `link_node` `:197` |
| H-a1 | The non-fragment arm steps over `old_child` when capturing the source siblings | `mutation/mod.rs:761-778`: "Source siblings = new_child's exposed prev/next, **stepping over old_child** (spec removes old_child at step 11, before new_child's adopt at step 13 …)"; without the step-over the closure is `capture_move_source(dom, new_child)` (`:304-313`: old parent, exposed prev / next, the old parent's connectedness) |
| H-a2 | The replace step comments use the old numbering (validity 1–6, then 7–14 with adopt inside insert) | `apply_replace_child`: D-12's table (`mutation/mod.rs:664-790`); the dom-api `ReplaceChild` handler `crates/dom/elidex-dom-api/src/element/tree.rs:215-230`, `:256-271`; the tests `crates/script/elidex-script-session/src/mutation/tests.rs:231`, `:244`, `:249`, `:260-261`, `:287-288`, `crates/script/elidex-js/src/vm/tests/tests_mutation_observer/integration.rs:164`, `:194` |
| H-a3 | Three paths reach this function | the VM's `replaceChild` (dom-api `ReplaceChild` `crates/dom/elidex-dom-api/src/element/tree.rs:285`); `ChildNode.replaceWith` when `this` is still under its parent (`crates/dom/elidex-dom-api/src/child_node/mutations.rs:235`; one argument is passed as the node itself, `child_node/mod.rs:361-362`); the deferred flush `Mutation::ReplaceChild` (`crates/script/elidex-script-session/src/mutation/mod.rs:199-203`). `git grep -n 'apply_replace_child' -- crates` |
| H-a4 | Two existing session tests pin the stepped-over siblings | `mutation/tests.rs:231` `apply_replace_child_move_step8_referencechild_adjustment` (`[A, B, C].replaceChild(C, B)`, `records[0].previous_sibling == A` at `:247`) and `:259` `apply_replace_child_move_prev_may_equal_new_child` (`[A, B, C].replaceChild(A, B)`, `records[0].next_sibling == C` at `:276`) |
| H-a5 | Two comments call the self-replace no-op "browser parity" without a measurement | `mutation/mod.rs:756-760` ("matching the VM handler's browser-parity no-op short-circuit"), `element/tree.rs:267-271` ("a no-op per browser parity (Chrome / Firefox / WebKit)"), `mutation/tests.rs:287-288` ("pre-existing browser-parity no-op") |

### §0.3 Premise errors

- **P-9 (program; restated).** Current *replace* (`webref body dom concept-node-replace`, run 2026-11-03): 1 ensure
  pre-insert validity; 2–3 referenceChild; 4 previousSibling; 5 removedNodes; **6 adopt node into parent's node
  document**; 7 if child's parent is non-null, set removedNodes and remove child (suppressed); 8 nodes; 9 insert
  (suppressed); 10 queue the record; 11 return child. Adopt step 2 therefore removes `node` while `child` is still in
  place: for `P = [A, B]; P.replaceChild(B, A)` the source-removal record has `previousSibling = A`; elidex reports
  `null`.
- **P-20 (B1.2a's decision, reversed here; Ax5 MIN-4).** B1.2a (#384 `c3bdb6f9`) decided the step-over in
  `docs/plans/2026-06-b1.2a-move-record-plan.md:210-217` ("Per spec, `oldChild` is removed at **step 11**, *before*
  `newChild`'s adopt at **step 13**") and its capture table `:234` ("old_child-exclusion = spec step-11-before-13
  ordering"). That was the spec's old numbering; the current *replace* adopts at step 6, before the removal at step 7
  (P-9), so the decision's premise no longer holds and D-12 reverses it. The B1.2a plan is a landed record and is not
  edited; this memo is the citation of the reversal.

### §0.4 Decisions (this PR)

- **D-12 (program) — applied here.** In the non-fragment arm the source context is `capture_move_source(dom,
  new_child)` (H-7, H-a1), read pre-write, with no `old_child` step-over: the closure at `:761-778` and its two `match`
  blocks are deleted, so the move-source shape has one builder for append, insert and replace (one-issue-one-way). The
  coalesced record's `previousSibling` (step 4) and `nextSibling` (steps 2–3) are unchanged. Records produced: count and
  order unchanged; only the source record's siblings change. Comment renumbering (H-a2), per line (`27b50dfe`
  coordinates; Ax4's table):

  | line | text at `27b50dfe` | C0a |
  |---|---|---|
  | `:664` | "the coalesced record (step 14)" | step 10 |
  | `:665-666` | "from `new_child`'s adopt (step 13's insert calls adopt → remove …)" | "from `new_child`'s adopt (step 6: adopt step 2 → remove, with `suppressObservers` left at its default …)" |
  | `:670` | "(steps 7–9)" | steps 2–4 |
  | `:690` | "steps 7-14 with expansion" | steps 2–10 (the fragment arm's step 6 comes with C0b's call, D-5) |
  | `:691` | "§4.2.3 replace step 2" (cycle check) | "replace step 1 (ensure pre-insert validity step 2)" |
  | `:700` | "(replace step 3)" (old child's parent) | "replace step 1 (ensure pre-insert validity step 3)" |
  | `:704`, `:705` | "step 7: referenceChild", "Step 8's … adjustment" | step 2, step 3 |
  | `:710` | "step 9: previousSibling" | step 4 |
  | `:712` | "step 11: remove oldChild" | step 7 |
  | `:716`, `:717` | "steps 12-13", "Step 13's insert" | steps 8–9, step 9 |
  | `:718` | "its step-4.2 fragment record" | unchanged (*insert* step 4.2) |
  | `:723`, `:726` | "step 14", "removed at step 11" | step 10, step 7 |
  | `:736` | "step-13's fragment record (4.2) THEN step-14 coalesced" | step 9's (*insert* 4.2), step 10 |
  | `:738` | "nested insert returned at step 3" | unchanged (*insert* step 3) |
  | `:743`, `:744` | "(steps 7–9)", "step 9" | steps 2–4, step 4 |
  | `:748`, `:750` | "step 7 … step-8 adjustment", "Step 8" | step 2, step 3 |
  | `:756-760` | "matching the VM handler's browser-parity no-op short-circuit" | rewritten: "elidex queues no record here; the spec queues two (`#11-replace-child-self-records`: Chromium unmeasured, C0a memo §7)" |
  | `:762-766` | the step-over rationale ("removes old_child at step 11, before new_child's adopt at step 13 …") | deleted with the closure (not renumbered: under the current order it would read "step 7 before step 6") |
  | `:790` | "source-removal (from step 13's adopt) THEN coalesced (step 14)" | step 6's adopt, step 10 |
  | `:684`, `:775` | *remove* step 12 `isParentConnected` | `:684` unchanged; `:775` deleted with the closure (`capture_move_source`'s docstring `:302` carries it) |

  Outside the function (the same numbering, H-a2): `element/tree.rs:217` "§4.2.3 step 3" → "replace step 1 (ensure
  pre-insert validity step 3)"; `:220` "§4.2.3 steps 1-2, 4" → "ensure pre-insert validity steps 1–2, 4–11"; `:225`
  "step 14" → step 10; `:256` "steps 1-6" → "step 1, ensure pre-insert validity with « oldChild » as
  childrenToExclude"; `:259` "step 6 counts EXCLUDING the replaced oldChild" → "ensure pre-insert validity steps 8–9
  exclude « oldChild »"; `:262-263` "step-3 NotFoundError (the spec runs validity steps 1-6 before the step-7..11
  self-replace collapse)" → "ensure pre-insert validity step 3's NotFoundError (replace step 1 runs before steps 2–10)";
  `:267-271` "a no-op per browser parity (Chrome / Firefox / WebKit) — the §4.2.3 step-8 reference-child adjustment
  makes the insert+remove sequence collapse to nothing" → "elidex queues no record for a self-replace; the spec
  queues two (`#11-replace-child-self-records`, C0a memo §7; Chromium unmeasured)" (the code is unchanged: the slot
  owns the fix); `integration.rs:164`, `:194` "step 14" → step 10. Tests: §6.
- **D-14 (a) (program) — prerequisite.** The child-list family moves to `mutation/child_list.rs` (NEW) as its own PR;
  `mod.rs` re-exports the moved `pub` items. *Why first (Ax3 MIN-1):* `mutation/mod.rs` is 994 lines and three program
  PRs edit it (C0a, C0b's fragment-arm adoption, C0c's `apply_adopt_node`), so the touch-time split comes before the
  first of them (program D-14 (a); PM 160 (b)). *Pure-move proof:* `git diff --color-moved=plain
  --color-moved-ws=no <base> -- crates/script/elidex-script-session/src/mutation/` shows every removed line of
  `mod.rs:296-895` as moved; the only non-moved lines are `child_list.rs`'s `use` header, `mod.rs`'s `mod child_list;`
  and `pub use` lines, and visibility qualifiers (`pub(super)`) on items `mod.rs` still calls, each listed in the PR
  body; `diff <(git show <base>:crates/script/elidex-script-session/src/mutation/mod.rs | sed -n '296,895p')
  <(sed -n '<k>,$p' <child_list.rs>)` (`<child_list.rs>` = `mutation/child_list.rs`, NEW; `<k>` = the first line
  after its header) prints only those qualifier lines; `cargo test -p elidex-script-session --all-features` green.

## §1. Shape and verdict

**Axes: 2** — A2 the insertion position (when the record layer reads siblings relative to the primitive) and A9
mutation records. Not edge-dense on its own; it is a per-PR slice of an edge-dense program, so its plan-review is
mandatory (CLAUDE.md *Edge-dense work* (b)) and, once passed, it is a terminal unit (base case (c)). One PR, after
D-14 (a). Separable from C0b because H-10 already gives the spec's DOM order; only the record capture is wrong.

**K-0 lens.** One observable change (R-6 of the program, records half): the source record's siblings move from HEAD's
to the spec's, on all three paths of H-a3.

**ECS-native check (Ax2 MIN-2).** No component, side-store, query or event is added or changed; the change is a
pre-write read in the record layer.

## §2. Coupled invariants

| pair (program §2.2) | intersection in C0a |
|---|---|
| A2×A9 | The record layer captures the source record before calling the primitive (H-7); D-12 changes which siblings it reads (with `child` in place), not when |
| A9 (axis) | Replace's source-removal siblings are captured with `child` in place; the coalesced record keeps steps 2–4's captures |

No other program pair is touched (program §2.2 "PR" column).

## §3. Spec coverage map

Lookups (2026-11-03): `heading dom 4.2.3` → Mutation algorithms; `body dom concept-node-replace` (1–11),
`concept-node-adopt` (2), `concept-node-remove` (5–6, 12, 16), `concept-node-insert` (3, 4.2),
`concept-node-ensure-pre-insertion-validity` (1–11).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §4.2.3 Mutation algorithms | *replace* 1–11 | step 6 adopt (its removal) before step 7; the coalesced record's siblings per steps 2–4 | `apply_replace_child` `crates/script/elidex-script-session/src/mutation/mod.rs:672` (at C0a's base: `mutation/child_list.rs`, NEW) | ✓ | yes |
| WHATWG DOM §4.2.3 Mutation algorithms | *remove* 5–6, 16 (inside *adopt* 2) | the source record's siblings are node's siblings before its removal, with `child` in place; it is queued (step 16, not suppressed) before *replace* step 10's record | `capture_move_source` `mutation/mod.rs:304`, `source_removal_record_from` `:319`, `move_record_list` `:357` (unchanged) | ✓ | yes |

## §4. Mechanism

| site | HEAD | C0a |
|---|---|---|
| source context `mutation/mod.rs:761-778` | inline closure, step over `old_child` | `capture_move_source(dom, new_child)` |
| step comments (H-a2) | old numbering | current 1–11 per D-12's table (step 6 in the fragment arm: C0b) |
| "browser-parity" comments (H-a5) | stated as fact | stated as unmeasured, citing `#11-replace-child-self-records` |

Docstring citation: `apply_replace_child` — DOM §4.2.3 *replace* steps 1–11 (`#concept-node-replace`); the source
record — DOM §4.5 *adopt* step 2 → *remove* steps 5–6, 16 (`#concept-node-remove`); the order of the two records —
*replace* step 6 before step 10.

## §5. Touched files

| file | lines at `27b50dfe` | change | 1000-line decision |
|---|---|---|---|
| `crates/script/elidex-script-session/src/mutation/mod.rs` | 994 | split first (D-14 (a)); the edit lands in `mutation/child_list.rs` (NEW) | prereq PR |
| `crates/script/elidex-script-session/src/mutation/tests.rs` (U14's home) | 866 | U14 (two tests rewritten and renamed), U14b, U14c; `:287-288` comment | ≈ 900; re-measure, ≥ 1000 → split first |
| `crates/script/elidex-js/src/vm/tests/tests_mutation_observer/integration.rs` (T17's home) | 648 | T17; `:164`, `:194` renumbered | ≈ 700 |
| `crates/script/elidex-js/src/vm/tests/tests_mutation_observer/direct_tree_ops.rs` (T17b's home) | 777 | T17b | ≈ 820; re-measure |
| `crates/dom/elidex-dom-api/src/element/tree.rs` | — | comments only (D-12, outside the function) | comment-only |
| `docs/plans/2026-10-event-handler-attrs-at-creation.md` | — | the C0 program row, the order line and the PM 271 edits (§8) | docs |

T17's home is `integration.rs`: `direct_tree_ops.rs:1-11` declares that the `replaceChild` coverage lives in
`super::integration` and that `direct_tree_ops.rs` covers the ChildNode / ParentNode methods, `replaceWith` among them
(`:3`), so T17b lives there (Ax3 MIN-6).

## §6. Test plan

**Red-first**: each red cell is run on the base before the fix with its named control green; the failure is recorded in
the PR description.

| cell | kind | shape | base | C0a = spec | control (green on the base and after) |
|---|---|---|---|---|---|
| T17 | red (VM) | `P = document.createElement('div'); A = document.createElement('a'); B = document.createElement('b'); P.append(A, B); mo = new MutationObserver(() => {}); mo.observe(P, {childList: true}); P.replaceChild(B, A); r = mo.takeRecords()` | `r[0].previousSibling === null` | `r[0]` (adopt's removal of `B`): `removedNodes [B]`, `previousSibling === A`, `nextSibling === null`; `r[1]`: `addedNodes [B]`, `removedNodes [A]`, `previousSibling === null`, `nextSibling === null` | `real_replace_child_move_delivers_source_removal_and_coalesced` (`integration.rs:396`; cross-parent, no step-over) |
| T17b | red (VM, `replaceWith`) | T17's setup with `A.replaceWith(B)` in place of `P.replaceChild(B, A)` (H-a3: `this` under `P`, the single argument passed as `B`) | as T17 | as T17 (*replaceWith* replaces `A` with `B` within `P`) | `real_replace_with_same_parent_delivers_one_coalesced_record` (`direct_tree_ops.rs:201`) |
| U14 | red (session unit; H-a4's two tests, rewritten) | (1) `[A, B, C]`, `apply_replace_child(parent, C, B)`; (2) `[A, B, C]`, `apply_replace_child(parent, A, B)` | (1) `records[0].previous_sibling == A`; (2) `records[0].next_sibling == C` | (1) `records[0]` `{removed [C], prev B, next None}`, `records[1]` `{added [C], removed [B], prev A, next None}`, children `[A, C]`; (2) `records[0]` `{removed [A], prev None, next B}`, `records[1]` `{added [A], removed [B], prev A, next C}`, children `[A, C]` | `apply_replace_child_cross_parent_move_source_plus_coalesced` (`tests.rs:204`) |
| U14b | guard | `new_child` not adjacent: `[A, X, B]`, `apply_replace_child(parent, B, A)` | `records[0]` `{removed [B], prev X, next None}` | unchanged | — |
| U14c | red (deferred flush) | U14 (1) through `apply_mutation(&Mutation::ReplaceChild { parent, new_child: C, old_child: B })` (H-a3's third path) | as U14 (1) | as U14 (1) | `apply_replace_child` (`tests.rs:395`, the fresh-node flush) |

**U14's spec values, derived by hand** (`concept-node-replace`, `concept-node-adopt` 2, `concept-node-remove` 5–6, 16):
(1) node = C, child = B. Step 2 referenceChild = B's next = C; step 3 it is node, so C's next = null; step 4
previousSibling = B's prev = A; step 6 adopts C, whose *remove* reads oldPreviousSibling = B, oldNextSibling = null
(tree `[A, B, C]`) and queues `{removed [C], prev B, next null}`; step 7 removes B (suppressed, tree `[A]`); step 9
inserts C before null (`[A, C]`); step 10 queues `{added [C], removed [B], prev A, next null}`. (2) node = A, child = B.
Step 2 referenceChild = C; step 3 does not apply; step 4 previousSibling = A; step 6's *remove* of A reads prev null,
next B and queues `{removed [A], prev null, next B}`; step 7 removes B (`[C]`); step 9 inserts A before C (`[A, C]`);
step 10 queues `{added [A], removed [B], prev A, next C}`. HEAD's values differ only in `records[0]`: (1) prev A,
(2) next C (H-a4).

**U14's renaming and comments (Ax3 MIN-3).** `tests.rs:231` `apply_replace_child_move_step8_referencechild_adjustment`
→ `apply_replace_child_move_step3_referencechild_adjustment`; `:244` "C's siblings step over B (removed first) -> prev
= A" → "adopt (step 6) removes C with B in place -> prev = B"; `:249` "step-8 next" → "step-3 next"; `:259`
`apply_replace_child_move_prev_may_equal_new_child` → `apply_replace_child_move_step4_prev_may_equal_new_child`;
`:260-261` "replace step 9 has no adjustment" → "replace step 4 has no adjustment"; `:273` "A's siblings step over B ->
next = C" → "adopt (step 6) removes A with B in place -> next = B". `:286-288` `apply_replace_child_self_replace_no_records`
keeps its assertion (the slot owns the fix) and its comment becomes "elidex queues no record; the spec queues two
(`#11-replace-child-self-records`, Chromium unmeasured)". There is one test family for these shapes, not two.

**Mutation guards.** Step-over kept → T17, T17b, U14, U14c. Step-over applied to the coalesced record → U14b, T17's
`r[1]`, U14's `records[1]`.
**Kept green.** `cargo test -p elidex-script-session --all-features`, `-p elidex-dom-api`, `-p elidex-js`; `mise run ci`.

## §7. Out of scope, and the `replaceChild(X, X)` measurement procedure

`replaceChild(X, X)` (`#11-replace-child-self-records`, ledger `:688`; its re-eval "after C0 lands" reads "after C0a
lands", program R-6 (e)). Its fix is not this PR's. **Owner:** Layout (the slot's owner lane). **Backstop:**
2026-12-15 (program R-6 (e)). **Runner:** the Layout driver, before any fix of the slot (Ax3 MIN-2). The slot requires
this procedure first because HEAD's no-op is commented as "browser parity" (H-a5; C0a rewrites those comments as
unmeasured) and the parity is unmeasured.

**Spec outcome** (`webref body dom concept-node-replace`, run 2026-11-03): for `P = [A, X, B]`, `P.replaceChild(X, X)`:
step 2 referenceChild = B; step 4 previousSibling = A; step 6 adopts X, whose *remove* (not suppressed) queues record 1
= `{removedNodes [X], previousSibling A, nextSibling B}`; step 7 is skipped (child's parent is null: "can only be false
if child is node"); step 9 re-inserts X before B, suppressed; step 10 queues record 2 = `{addedNodes [X], removedNodes
[], previousSibling A, nextSibling B}`. elidex at HEAD: no records.

**Procedure** (in the built-in Chromium of the desktop app, on `about:blank`; record `navigator.userAgent`):
1. `P = document.createElement('div'); document.body.append(P); A = document.createElement('a'); X =
   document.createElement('x-x'); B = document.createElement('b'); P.append(A, X, B); mo = new MutationObserver(() =>
   {}); mo.observe(P, {childList: true}); P.replaceChild(X, X); r = mo.takeRecords();` then serialise each record as
   `{type, added: [...addedNodes].map(n => n.localName), removed: [...removedNodes].map(n => n.localName), prev:
   previousSibling?.localName ?? null, next: nextSibling?.localName ?? null}`.
2. Variants: X the only child (`P.append(X)` alone); X first (`[X, B]`) and last (`[A, X]`); P detached (no
   `document.body.append(P)`); `X.replaceWith(X)` in place of `P.replaceChild(X, X)` (H-a3: the same *replace X with X
   within P*; elidex at HEAD queues no record on this path either); X a defined custom element (run before step 1's
   `createElement`: `log = []; customElements.define('x-x', class extends HTMLElement {
   connectedCallback() { log.push('c') } disconnectedCallback() { log.push('d') } })`, recording `log`: the spec's
   remove-then-insert gives `['d', 'c']` when P is connected).
3. Record every variant's records and `log` in the slot (ledger `:688`) with the Chromium version and the date.
4. Decision input for the slot's owner: Chromium = spec → the fix follows the spec and the "unmeasured" comments are
   replaced by the spec citation; Chromium = no records → a spec-vs-browser divergence, taken to the PM before any
   change (the fix is not chosen here).

## §8. Edges and records

| # | edge | kind | note |
|---|---|---|---|
| c0-pre-a | D-14 (a) → C0a | code predecessor | pure move |
| c0-ab | C0a → C0b | code predecessor (soft) | both edit `apply_replace_child`; C0b adds the fragment arm's step-6 call and comment on C0a's numbering |

Records carried by C0a's PR, in the umbrella (exact texts in program R-3; revision number per program D-9):
1. the §1.2 C0 program row;
2. the §1.2 order line rewritten to PM 153's order with C0 as the program (PM 227 O-1): the line "approval PR → slice
   a → { IBP-sandbox N1, T3 } → C1's handling decided → slice b …" is replaced by "approval PR → slice a → IBP-sandbox
   N1 → **C0** (program: C0a → C0b → C0c) → C1 → T3 → slice b";
3. **consistency edits under PM 271, amending PM 223 O-1**: the umbrella sentences that the new order line or an
   already-made decision (PM 123 / 153 / 227 / 244) contradicts, before → after:

| # | umbrella site (`27b50dfe`) | before | after |
|---|---|---|---|
| E1 | header `:5-6` | "it also owns **T3**, slice b's independent prerequisite PR: slot `#11-event-handler-target-determination` was reassigned to this program by PM decision 25 (§1.2, §8.1)." | "it also owns **T3**, slice b's prerequisite PR: slot `#11-event-handler-target-determination` was reassigned to this program by PM decision 25 (§1.2, §8.1). From rev 8.<n> it also owns **C0** (`#11-cross-document-adopt-on-insert`, PM decision 143; a program of three PRs, PM decision 244) and **C1** (`#11-template-contents-owner-document`, branch A, PM decision 123), and T3 lands after them (PM decision 153; until rev 8.<n> T3 was "slice b's independent prerequisite PR")." |
| E2 | §1.2 T3 row, prerequisites `:424` | "slice a landed" | "C1 landed (PM decision 153; slice a, N1's implementation and C0 precede it in the order line; until rev 8.<n>: "slice a landed")" |
| E3 | §1.2 slice b row, prerequisites `:425` | "T3, N1, C1's handling decided" | "T3, N1, C0 and C1 landed (PM decisions 123, 153; until rev 8.<n>: "T3, N1, C1's handling decided")" |
| E4 | §1.3 x6 `:439`, kind and reason | "scheduling precondition (PM decision 25) \| T3 is started once slice a lands. No code or semantic dependency is claimed." | "scheduling precondition (PM decisions 25, 153) \| T3 is started after C1 lands (PM decision 153; until rev 8.<n>: "once slice a lands"), so x6 holds transitively (slice a → N1 → C0 → C1 → T3). No code or semantic dependency is claimed." (the rest of the cell unchanged) |
| E5 | §1.3, new row after x6 | — | "\| x9 \| C0 → T3 (rev 8.<n>) \| semantic precondition (PM decision 153) \| without insertion adoption a node moved into another document keeps its old node document, which T3's step 3 reads; C0b makes the node document a stored fact, kept true at every insertion and read by `EcsDom::node_document` \|" |
| E6 | §1.3, new row after x9 | — | "\| x10 \| N1 → C0 (rev 8.<n>) \| code predecessor (PM decision 153) \| C0b rewrites N1's compile-gate clause (b) to read `EcsDom::node_document` (C0 program memo D-7 (a)), so N1's implementation (#580) lands first \|" |
| E7 | §1.3 `:441` | "Acyclic. The order satisfies x4 transitively." | "Acyclic. The order satisfies x4 transitively. x7 (C0 → C1) and x8 (C1 → T3) are added by the PR at each edge's head (C0 program memo D-9)." |
| E8 | §1.4 closing path `:467-468` | "slice a lands → T3 (this program's PR, slot `#11-event-handler-target-determination`; N1 lands independently) → C1's handling decided (PM) → slice b → the PM closes the slot above (§1.3 x6, x3–x5, x2)." | "slice a lands → N1 → C0 → C1 → T3 (this program's PR, slot `#11-event-handler-target-determination`) → slice b → the PM closes the slot above (§1.3 x6, x3–x5, x2, x9, x10; rev 8.<n>, PM decision 153; until rev 8.<n>: "slice a lands → T3 (…; N1 lands independently) → C1's handling decided (PM) → slice b")." |
| E9 | §1.6 `:504-505`, after "(§8.1)." | — | "Rev 8.<n>: decided, branch A (PM decision 123); C1 is this program's nested PR, after C0 and before T3 (§1.2)." |
| E10 | §7 G3 `:820-821` | "It is T3, slice b's independent prerequisite PR, owned by this program (PM decision 25; §1.2)." | "It is T3, slice b's prerequisite PR, owned by this program (PM decision 25; §1.2), after C1 (PM decision 153)." |
| E11 | §8.1 `:842` | "slice b's independent prerequisite PR (x3), started once slice a lands (x6)," | "slice b's prerequisite PR (x3), started after C1 lands (x6; PM decision 153)," |
| E12 | §8.2 closing path and trigger `:898-899` | "**Closing path:** slice a lands → T3 (this program) → C1's handling decided → slice b. **Trigger:** T3 and N1 landed, and C1's handling decided (the ledger" | "**Closing path:** slice a lands → N1 → C0 → C1 → T3 (this program) → slice b (PM decision 153). **Trigger:** N1, C0, C1 and T3 landed (the ledger" |
| E13 | §8.2 confidence `:901-902` | "The order is fixed (PM decisions 21, 23, 25) and T3 has an owner, but T3 starts only after slice a lands, and slice b also waits on C1's handling (owner TBD)." | "The order is fixed (PM decisions 21, 23, 25, 153) and T3 has an owner; T3 starts after C1 lands, and C1's handling is decided (branch A, PM decision 123)." |

E1–E7 and E9 are the items PM 271 lists; E8 and E10–E13 are the other umbrella sentences that state the replaced order
(found by `/usr/bin/grep -n -i 'independent prerequisite\|once slice a\|lands independently\|handling decided' <umbrella>`
at `27b50dfe`: hits `:5`, `:59`, `:425`, `:427-428`, `:438`, `:439`, `:468`, `:821`, `:842`, `:899`), edited under the same
rule. Not edited: `:59` (rev 6's changelog entry, history); x5 `:438` (its condition, "C1's handling decided", is met by
E9; its branch-A clause stays true). No design content of T3 is brought in (T3's facets, D-T*, rev 8.4 / 8.5 scope
stay on T3's branch).

4. one umbrella changelog line for 1–3 (program R-3).

T3's branch drops E1–E13 and its own C0 / C1 rows and order-line edits when it rebases onto C0a, keeps its x8, and
renumbers its rev headings after C0a / C0b / C0c take the next numbers (program H-T; Ax5 MIN-2).

For the PM: program R-8 (the ledger order line); program R-6 (e) re-eval wording; the ledger `:688` pointer to this memo's §7; memory
`project_b1-mutationobserver-next-task.md` `:42` lesson line (and `:44`): it must say that the source-removal siblings
are read with `child` in place (current *replace* step 6 before step 7) and that C0a removed B1.2a's step-over (P-20) —
read 2026-11-03, both lines already carry that correction ("[corrected 2026-11-03 …]"), so the record is a check at
C0a's landing, not an edit.

## §9. Definition of done

1. D-14 (a) landed with its pure-move proof (D-14 (a)); coordinates re-measured in `mutation/child_list.rs`; the
   pre-implementation run of item 2 recorded.
2. With the step-over removed alone, the red tests recorded with their reasons (expected: U14 (1), (2) and nothing
   else outside §6); T17, T17b, U14, U14c red on the base with their named controls green, recorded in the PR.
3. D-12 implemented; every row of D-12's table and its "outside the function" list applied; `awk '/^\/\/\/ Replace
   .old_child. with .new_child./,/^}$/' <child_list.rs> | /usr/bin/grep -n -E '[Ss]teps?[ -]1[1-4]'` (as in D-14 (a))
   prints exactly one line, the *remove* step 12 `isParentConnected` comment (HEAD `:684`; at `27b50dfe` the same
   command over `mod.rs` prints 12 lines, run 2026-11-03);
   `git grep -n -i 'browser.parity' -- crates/script/elidex-script-session/src/mutation crates/dom/elidex-dom-api/src/element/tree.rs`
   → no hit.
4. T17, T17b, U14, U14b, U14c green; §6's crates and `mise run ci` green.
5. The umbrella row, the rewritten order line and the **consistency edits under PM 271, amending PM 223 O-1** (E1–E13,
   §8) in the PR, revision number per D-9 at rebase; the PR body lists them under a section with that title, each by
   site, and states that T3 drops them when it rebases (program H-T).
6. If this is the C0 program PR that lands after #583: program §9 item 6.
7. `python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-11-eh-attrs-c0a-replace-record-order.md`
   exits 0 (run).

## §10. Review dispositions (plan-review of C0a at `4c79d9c8`, 2026-11-03)

| finding | disposition | where |
|---|---|---|
| Ax2 MIN-1 (`replaceWith` and the deferred flush reach the function) | H-a3; T17b; U14c (session unit, reachable through `apply_mutation`); `X.replaceWith(X)` variant | §0.2, §6, §7 |
| Ax2 MIN-2 (no ECS-native line) | added | §1 |
| Ax2 note (the capture equals `capture_move_source`) | D-12 calls it | D-12, §4 |
| Ax3 IMP-1 (two existing tests flip) | they are U14, rewritten to the hand-derived values and renamed | H-a4, §6 |
| Ax3 MIN-1 (D-14 (a) reason; weak proof) | reason cited; `--color-moved` + range `diff` | D-14 (a) |
| Ax3 MIN-2 (§7 owner, backstop, runner) | stated | §7 |
| Ax3 MIN-3 (tests' numbering) | renamed and renumbered | §6 |
| Ax3 MIN-4 (DoD 3 grep) | scoped to the function, expected output stated | §9 |
| Ax3 MIN-5 ("browser-parity" unmeasured) | rewritten as unmeasured with the slot (also `tree.rs:267-271`, found by the same grep). The reviewer's `tests.rs:282` is `:287-288` at `27b50dfe` | H-a5, D-12, §6 |
| Ax3 MIN-6 (T17's home; controls) | `integration.rs` per `direct_tree_ops.rs:1-11`; T17b in `direct_tree_ops.rs`; a control per red cell | §5, §6 |
| Ax4 IMP-1, MIN-1, MIN-2 (renumber table) | per-line table; `:762-766` deleted, not renumbered; validity cites | D-12 |
| Ax4 MIN-3 (§3 basis) | *remove* 5–6, 16; order = *replace* 6 before 10 | §3, §4 |
| Ax5 IMP-1, MIN-1, MIN-3 (umbrella consistency; C0 edges; R-B) | PM 271: E1–E13; R-B stated as text (program R-3) | §8; program D-9, R-3, H-T |
| Ax5 MIN-2 (T3's rev numbering) | T3 renumbers at rebase | §8; program H-T |
| Ax5 MIN-4 (B1.2a reversed uncited) | P-20; the PM's memory check | §0.3, §8 |
| Ax5 note (ledger `:688`) | updated | §7, §8 |
