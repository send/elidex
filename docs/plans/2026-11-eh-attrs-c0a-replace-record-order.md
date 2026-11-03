# C0a `EH-attrs-at-creation-C0a` — the *replace* mutation records follow the current step order (per-PR plan memo)

Per-PR slice **C0a** of the C0 program (program memo `docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md`, rev 3.1;
"the program memo"), under the umbrella `docs/plans/2026-10-event-handler-attrs-at-creation.md`. It carries program decision
**D-12** (PM decision 160 (a); "in the C0 program", PM 244) after its prerequisite pure-move split **D-14 (a)**. It is
the first program PR to land and adds the umbrella's C0 program row and rewrites its §1.2 order line (program D-9,
record R-3; PM 227 O-1). Siblings and IDs: the
program memo's header; program IDs (H-*, P-*, D-*, A*, R-*) are cited as the program memo's.

**Placement.** PM 160 (a): the replace record order is fixed in the C0 program; PM 160 (b): `mutation/mod.rs` is split
first; PM 244: C0a = D-12 alone, after D-14 (a); C0a may land any time before C0b (program §1).

**Scope.** `apply_replace_child`'s non-fragment arm captures the source-removal record's siblings with `old_child` still
in place (DOM *replace* step 6 adopts node — removing it from its old parent — before step 7 removes child); every
*replace* step comment in the function is renumbered to the current steps 1–11, except the fragment arm's step 6, whose
code arrives with C0b (program D-5).

**Not in scope.** The fragment arm's adoption (C0b, D-5); insertion adoption itself (C0b); `replaceChild(X, X)`'s
records (`#11-replace-child-self-records`, registered PM 227); `adoptNode`'s removal record (C0c, D-13).

**Changelog.** *Rev 3.1* (2026-11-03, with the program memo's rev 3.1; plan-review R2 of the program): the umbrella
order-line rewrite (PM 227 O-1) added to this PR's records (§8, DoD 5); `#11-replace-child-self-records`'s measurement
procedure restored here (§7; R2 Ax5 IMP-4). *Rev 1* (2026-11-03): first draft, carved from the C0 memo rev 2.1 (`d262ca8c`) D-12 / D-14 (a) / T17 /
U14 / P-9 per plan-review R1's SPLIT and PM 244.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Coordinates are `27b50dfe` (program §0.1). C0a's base is `origin/main` with D-14 (a) landed, which moves the child-list
family of `crates/script/elidex-script-session/src/mutation/mod.rs` (`:296-895`) into `mutation/child_list.rs` (NEW);
every `mutation/mod.rs` coordinate below is re-measured there at C0a's base (§9 item 1) by symbol (`apply_replace_child`)
and by the quoted comment text, which a pure move keeps.

### §0.2 Premises

| id | claim | evidence (`27b50dfe`) |
|---|---|---|
| H-7 | The record layer models adopt step 2 | `capture_move_source` / `source_removal_record` / `move_record_list` (`crates/script/elidex-script-session/src/mutation/mod.rs:304-384`) |
| H-10 | The primitive detaches `new_child` before `old_child` | `replace_child` (`crates/core/elidex-ecs/src/dom/tree/mutation.rs:138`): `detach_with_hook(new_child)` `:164`, `detach(old_child)` `:186`, `link_node` `:197` |
| H-a1 | The non-fragment arm steps over `old_child` when capturing the source siblings | `mutation/mod.rs:762-774`: "Source siblings = new_child's exposed prev/next, **stepping over old_child** (spec removes old_child at step 11, before new_child's adopt at step 13 …)" |
| H-a2 | The function's step comments use the old numbering (7–14 with adopt inside insert) | `mutation/mod.rs:664-670` (docstring: steps 14, 13, 7–9), `:690` ("steps 7-14"), `:691` ("step 2", validity), `:700` ("step 3", validity), `:704` ("step 7", "Step 8's"), `:710` ("step 9"), `:712` ("step 11"), `:716-718` ("steps 12-13", "Step 13's"), `:723-726` ("step 14", "step 11"), `:736` ("step-13's", "step-14"), `:743-751` ("steps 7–9", "step 9", "step 7", "step-8", "Step 8"), `:762-764` ("step 11", "step 13"), `:790` ("step 13's", "step 14"); `:775` cites *remove* step 12 (`isParentConnected`), which is correct (`webref body dom concept-node-remove`) |
| H-a3 | The VM's `replaceChild` reaches this function | dom-api `ReplaceChild` handler `crates/dom/elidex-dom-api/src/element/tree.rs:285` → `apply_replace_child`; `git grep -n 'apply_replace_child' -- crates` |

### §0.3 Premise errors

- **P-9 (program; restated).** Current *replace* (`webref body dom concept-node-replace`, run 2026-11-03): 1 ensure
  pre-insert validity; 2–3 referenceChild; 4 previousSibling; 5 removedNodes; **6 adopt node into parent's node
  document**; 7 if child's parent is non-null, set removedNodes and remove child (suppressed); 8 nodes; 9 insert
  (suppressed); 10 queue the record; 11 return child. Adopt step 2 therefore removes `node` while `child` is still in
  place: for `P = [A, B]; P.replaceChild(B, A)` the source-removal record has `previousSibling = A`; elidex reports
  `null`.

### §0.4 Decisions (this PR)

- **D-12 (program) — applied here.** In the non-fragment arm the source context's `prev` / `next` are
  `prev_exposed_sibling(new_child)` / `next_exposed_sibling(new_child)` read pre-write, with no `old_child` step-over
  (the two `match` blocks at `:767-774` collapse to the plain reads). The coalesced record's `previousSibling` (step 4)
  and `nextSibling` (steps 2–3) are unchanged. Comment renumbering (H-a2), old → current: validity "step 2" / "step 3"
  → step 1 (*ensure pre-insert validity*); 7 → 2; 8 → 3; 9 → 4; 11 → 7; 12–13 → 8–9; "new_child's adopt at step 13" →
  step 6; 14 → 10. The fragment arm gets no step-6 comment (its adoption is C0b's D-5, which adds the call and its
  comment). Records produced: count and order unchanged; only the source record's siblings change.
- **D-14 (a) (program) — prerequisite.** The child-list family moves to `mutation/child_list.rs` (NEW) as its own PR;
  `mod.rs` re-exports the moved `pub` items. Verified as a pure move (`git diff --stat`, `cargo test -p
  elidex-script-session --all-features` green).

## §1. Shape and verdict

**Axes: 2** — A2 the insertion position (when the record layer reads siblings relative to the primitive) and A9
mutation records. Not edge-dense on its own; it is a per-PR slice of an edge-dense program, so its plan-review is
mandatory (CLAUDE.md *Edge-dense work* (b)) and, once passed, it is a terminal unit (base case (c)). One PR, after
D-14 (a). Separable from C0b because H-10 already gives the spec's DOM order; only the record capture is wrong.

**K-0 lens.** One observable change (R-6 of the program, records half): the source record's siblings move from HEAD's
to the spec's.

## §2. Coupled invariants

| pair (program §2.2) | intersection in C0a |
|---|---|
| A2×A9 | The record layer captures the source record before calling the primitive (H-7); D-12 changes which siblings it reads (with `child` in place), not when |
| A9 (axis) | Replace's source-removal siblings are captured with `child` in place; the coalesced record keeps steps 2–4's captures |

No other program pair is touched (program §2.2 "PR" column).

## §3. Spec coverage map

Lookups (2026-11-03): `heading dom 4.2.3` → Mutation algorithms; `heading dom 4.3.2` → Queuing a mutation record;
`body dom concept-node-replace` (1–11), `concept-node-adopt` (2), `concept-node-remove` (12).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §4.2.3 Mutation algorithms | *replace* 1–11 | step 6 adopt (its removal) before step 7; record siblings per step 2–4 and adopt's *remove* | `apply_replace_child` `crates/script/elidex-script-session/src/mutation/mod.rs:672` (at C0a's base: `mutation/child_list.rs`, NEW) | ✓ | yes |
| WHATWG DOM §4.3.2 Queuing a mutation record | source record before destination record | adopt's *remove* record siblings | `source_removal_record_from` `mutation/mod.rs:319`, `move_record_list` `:357` (unchanged) | ✓ | yes |

## §4. Mechanism

| site | HEAD | C0a |
|---|---|---|
| source siblings `mutation/mod.rs:767-774` | step over `old_child` | plain pre-write reads |
| step comments (H-a2) | old numbering | current 1–11 (step 6 in the fragment arm: C0b) |

Docstring citation: `apply_replace_child` — DOM §4.2.3 *replace* steps 1–11 (`#concept-node-replace`); the source
record — DOM §4.5 *adopt* step 2 → *remove* (`#concept-node-remove`).

## §5. Touched files

| file | lines at `27b50dfe` | change | 1000-line decision |
|---|---|---|---|
| `crates/script/elidex-script-session/src/mutation/mod.rs` | 994 | split first (D-14 (a)); the edit lands in `mutation/child_list.rs` (NEW) | prereq PR |
| `crates/script/elidex-script-session/src/mutation/tests.rs` (U14's home) | 866 | U14, U14b | re-measure; ≥ 1000 → split first |
| `crates/script/elidex-js/src/vm/tests/tests_mutation_observer/direct_tree_ops.rs` (T17's home) | 777 | T17 | re-measure |
| `docs/plans/2026-10-event-handler-attrs-at-creation.md` | — | the C0 program row (program R-3) | docs |

Existing tests whose expectation changes: any session or VM test that pins the stepped-over siblings; found by the
pre-implementation run (§9 item 2).

## §6. Test plan

**Red-first**: each red cell is run on the base before the fix with its control green; the failure is recorded in the
PR description.

| cell | kind | shape | base | C0a = spec |
|---|---|---|---|---|
| T17 | red (VM) | `P = document.createElement('div'); A = document.createElement('a'); B = document.createElement('b'); P.append(A, B); mo = new MutationObserver(() => {}); mo.observe(P, {childList: true}); P.replaceChild(B, A); r = mo.takeRecords()` | `r[0].previousSibling === null` | `r[0]` (adopt's removal of `B`): `removedNodes [B]`, `previousSibling === A`, `nextSibling === null`; `r[1]`: `addedNodes [B]`, `removedNodes [A]`, `previousSibling === null`, `nextSibling === null` |
| U14 | red (session unit) | `apply_replace_child` with `new_child` adjacent to `old_child`, both orders (`[A, B]` replace A by B; `[B, A]` replace A by B) | source siblings skip `old_child` | source siblings include `old_child` (`prev = A` resp. `next = A`) |
| U14b | guard | `new_child` not adjacent (`[A, X, B]`, replace A by B); `new_child` from another parent | source siblings `X` / the other parent's | unchanged |

**Mutation guards.** Step-over kept → T17, U14. Step-over applied to the coalesced record → U14b, T17's `r[1]`.
**Kept green.** `cargo test -p elidex-script-session --all-features`, `-p elidex-dom-api`, `-p elidex-js`; `mise run ci`.

## §7. Out of scope, and the `replaceChild(X, X)` measurement procedure

`replaceChild(X, X)` (`#11-replace-child-self-records`, ledger `:687`; its re-eval "after C0 lands" reads "after C0a
lands", program R-6 (e)). Its fix is not this PR's; the slot requires this procedure to run **before** any fix, because
HEAD's no-op is commented as "browser-parity" (`mutation/mod.rs:757-760` at `27b50dfe`) and the parity is unmeasured.

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
   `document.body.append(P)`); X a defined custom element (run before step 1's `createElement`: `log = []; customElements.define('x-x', class extends HTMLElement {
   connectedCallback() { log.push('c') } disconnectedCallback() { log.push('d') } })`, recording `log`: the spec's
   remove-then-insert gives `['d', 'c']` when P is connected).
3. Record every variant's records and `log` in the slot (ledger `:687`) with the Chromium version and the date.
4. Decision input for the slot's owner: Chromium = spec → the fix follows the spec and the "browser-parity" comment is
   withdrawn; Chromium = no records → a spec-vs-browser divergence, taken to the PM before any change (the fix is not
   chosen here).

## §8. Edges and records

| # | edge | kind | note |
|---|---|---|---|
| c0-pre-a | D-14 (a) → C0a | code predecessor | pure move |
| c0-ab | C0a → C0b | code predecessor (soft) | both edit `apply_replace_child`; C0b adds the fragment arm's step-6 call and comment on C0a's numbering |

Records carried by C0a's PR: the umbrella §1.2 C0 program row, **the §1.2 order line rewritten to PM 153's order with
C0 as the program** (PM 227 O-1), and one changelog line for both (program R-3 holds the exact texts: the row, the
line it replaces — "approval PR → slice a → { IBP-sandbox N1, T3 } → C1's handling decided → slice b …" — and its
replacement "approval PR → slice a → IBP-sandbox N1 → **C0** (program: C0a → C0b → C0c) → C1 → T3 → slice b";
revision number per program D-9). For the PM: program R-6 (e) re-eval wording; the ledger `:687` pointer to this memo's
§7.

## §9. Definition of done

1. D-14 (a) landed; coordinates re-measured in `mutation/child_list.rs`; the pre-implementation run of item 2 recorded.
2. With the step-over removed alone, the red tests recorded with their reasons; T17 and U14 red on the base with their
   controls green, recorded in the PR.
3. D-12 implemented; every H-a2 comment renumbered per D-12's map; `/usr/bin/grep -n 'step 1[1-4]' <child_list.rs>`
   returns no *replace* step (the function's numbering stops at 11).
4. T17, U14, U14b green; §6's crates and `mise run ci` green.
5. The umbrella row and the rewritten order line (program R-3) in the PR, revision number per D-9 at rebase.
6. `python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-11-eh-attrs-c0a-replace-record-order.md`
   exits 0 (run).
