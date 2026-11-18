# EH-attrs-at-creation — event-handler content attributes processed once, at the element's creation (umbrella memo)

Standalone program, label **`EH-attrs-at-creation`**. Owner: Layout lane. It sits **outside** the IBP-sandbox program and
is not part of that umbrella's DAG (PM decision 17 (i), restated by PM decision 21). It has two nested PRs, **slice a**
and **slice b** (§1), and it also owns **T3**, slice b's prerequisite PR: slot `#11-event-handler-target-determination` was reassigned to this program by PM decision 25 (§1.2, §8.1). From rev 8.4 it also owns **C0** (`#11-cross-document-adopt-on-insert`, PM decision 143; a program of three PRs, PM decision 244) and **C1** (`#11-template-contents-owner-document`, branch A, PM decision 123), and T3 lands after them (PM decision 153; until rev 8.4 T3 was "slice b's independent prerequisite PR"). IBP-sandbox N1 (`docs/plans/2026-10-ibp-sandbox-handler-gates.md`, rev 4.5, carried by N1's PR on branch
`layout-ibp-sandbox-handler-gates` together with this file's rev 8.1 to 8.3) depends on slice a (§1.3 edge x1). §L shows that no landing order can put N1 first.

**Approval scope (rev 5, PM decision 24: decision altitude).** Approving this umbrella **decides**:
- the landing-order verdict (§L);
- the nested-PR set, their order and edges (§1.2–§1.3);
- the classification of slice a as an **approved interim, required by dependency**, and what slice b removes (§1.4,
  PM decision 23);
- for **slice a**: the contract clauses K-0 to K-6 with their landing qualifiers and the rule X-W (§0.5), the complete list of its
  processing timings with the cell that proves each no worse than HEAD (§0.5 K-0), the D-items (§0.4), and its cells and
  pins (§6.1–§6.2, the red-first rule included);
- for **slice b**: only the observable invariants S-1 to S-7 (§0.6);
- each fact's home and representation kind (§4's *Home/kind* line);
- the spec-semantics decisions (§0.5, §3's *Step* and *Branch* columns);
- the gap dispositions (§7), the slots and records (§8), and the per-slice DoD (§10).

Everything else is **input to the owning slice's own plan-review**, which re-derives it. That covers §4's mechanism lines,
§5's populations and risks, §6's file placement, helper reuse and mutation guards (§6.3–§6.4), §3's *Touch* column, and the whole of §9 (slice b's
input notes). **Explicitly not decided here** (slice b's per-PR plan and its plan-review decide each one): the
creation-time call-site shape (O9); whether the consumer's Insert arm keeps any role in branch B (rev 4's K-B); slice b's
cells, including the detector and branch-B cells (rev 4's B7, B9); how "clones of template contents" are identified;
and whether branch B is chosen at all (the PM decides C1's handling, §1.6). A review finding against input changes this
umbrella only when it changes a decision. Otherwise it is recorded in the owning slice's plan.

**Approval vehicle (O10, decided).** A **docs-only umbrella-approval PR** that carries this file only, separate from
slice a's implementation PR (the #541 precedent). "At umbrella approval" means "when the approval PR lands". Slice a's
implementation is cut from main after that.

**Changelog.** *Rev 8.<t>* (carried by T3's plan PR, branch `layout-eh-target-determination-plan`, with the T3 memo
`docs/plans/2026-10-eh-attrs-t3-target-determination.md` rev 6.1, the T3-pre memo
`docs/plans/2026-11-eh-attrs-t3-pre-subtree-created-event.md` rev 2.4, the T3-a memo
`docs/plans/2026-11-eh-attrs-t3a-applicability.md` rev 1 and the T3-b memo `docs/plans/2026-11-eh-attrs-t3b-body-prototype.md` rev 1 (until PM decision 767 one memo, T3-ab, `docs/plans/2026-11-eh-attrs-t3-applicability-and-body-prototype.md` rev 1.1). **The PM decisions Rev 8.<t> applies, the
one list** (the four memos refer here and keep no list of their own): 129–131, 153, 168, 359, 699, 705, 711, 713,
714, 718–720, 722, 723, 730, 732, 740, 741, 756, 764, 767 (721 is another lane's; until T3 memo rev 6 this entry, the T3 memo's
header and its §8.4 heading each kept a different list). `<t>` is
assigned at landing, as for C1's *Rev 8.7* (PM 223 O-1 / 232; PM decision 699); until T3 memo rev 4.6: "the next number
after main's when T3 rebases, the C0 program memo D-9's rule"). PM decision 129 (the user's choice for T3 memo
rev 1 O-1): T3 also ends X-W for the pairs HEAD targeted at the element (a `<body>`'s six Window-reflecting names,
every Window-delegating name on a `<frameset>`) and hands them to the spec target, so the windows the T3 memo calls W1
and W2 never land on main; that part leaves slice b's S-5. PM decision 153 (the user's choice for T3 memo rev 2 O-6):
S-1's clone path moves into T3 (T3 memo D-T10), so slice b's S-1 loses it, and T3 chooses a `<body>`'s HTML interface
by namespace (D-T11). PM decision 168 (T3 memo rev 3 O-7 (a)): T3 closes facets (1) and (3) of
`#11-event-handler-content-attribute-applicability` (D-T8); facet (2) stays with slice b. PM decision 699 (T3 memo rev
4.5 O-10 (a)): T3 also processes a fragment-parsed element at its creation when its node document is active (T3 memo
D-T7). PM decision 705 (T3 memo rev 4.6 O-11 → S-i × C-ii): the whole fragment path of S-1 is T3's (T3 memo D-T12),
processed at creation through one creation event, RC-2a's `CloneCreated` generalised to `SubtreeCreated { root, cause }`
(T3 memo §4.5). PM decisions 711, 713 and 714 (T3 memo rev 4.7 → 4.8): the creation walk is every node the parse
created (declarative shadow content and template contents included, L3's `WalkScope::WithTemplateContents`), and a
creation event fired inside a dispatcher window (DOMParser's build) is queued and dispatched at the window's close, so
slice b's S-1 loses the whole fragment path and the DOMParser build (until T3 memo rev 4.8: "loses the fragment path
except the declarative shadow content and template contents a fragment parse creates (outside the insertion-time
walk set)"); T3 lands after L3's ecs-entity-lifetime PR (io) (new x12). T3 memo rev 4.8.1: x12 cites L3's split plans by symbol and section (no edge or order change). T3 memo rev 5 (PM decisions 722, 723; T3 plan-review R1 on rev 4.8.2): the creation event's generalisation, its
consumers' migration, the fragment fire and the window queue split out of T3 into the new nested PR **T3-pre** (new §1.2
row; the Order line gains "(L3's is → ig) → T3-pre ∥ L3's io → T3"; x11 re-pointed RC-2a → T3-pre; x12 narrowed to
io's `WalkScope`; new x13 L3 ig → T3-pre and x14 T3-pre → T3; this plan PR carries T3-pre's row, edges and order-line
entry because it lands T3-pre's memo, PM decision 722; with the T3-pre memo rev 2, PM decisions 730, 740, 741: the Order
line names L3's tree-model program between ig and io with T3-pre's place unchanged, x13 names what T3-pre relies on from ig
after PM 740 (2), and x14 points at the no-op window of the T3-pre memo DoD item 7 instead of "Layout's next unit"). **T3 memo rev 6
(PM decision 741 (3); T3 plan-review R2 on rev 5):** applicability facets (1) and (3) (T3 memo rev 5's D-T8) and the
HTML-interface choice for a foreign `<body>` (D-T11) are carved out of T3 into the new nested PR **T3-ab** (split into **T3-a** and **T3-b** by PM decision 767, below), a
prerequisite of T3 that needs nothing beyond main (new §1.2 row; the Order line gains it beside T3-pre; new edge x15
T3-ab → T3, whose no-op-window sentence points at the T3-pre memo DoD item 7 from the T3-pre memo rev 2.1, PM decision
756; PM decision 168 is revised: those facets close at T3-ab's landing). **PM decision 767 (plan-review R3 wave 1, Axis 3 split verdict, W1-1; PM 227):** T3-ab is two nested PRs, **T3-a** (D-T8: the applicability predicate and its three readers, in the session) and **T3-b** (D-T11: `HTMLBodyElement.prototype` for an HTML `<body>` only, in the VM host), each needing nothing beyond main and unordered against the other: §1.2's T3-ab row is two rows, the Order line names both, x15 is T3-a → T3 and the new **x16** is T3-b → T3, §4.1's tags read `[T3-a]` (D-T11 touches neither function) and its Q-A1 sentence is the answer (PM decision 756), and their condition on the no-op window is stated only in the T3-pre memo DoD item 7. Applicability facet (2) is recorded as having no case at the base (no attribute namespace is modelled; R3 wave 1 W1-3; §1.2, §8.1, §10.2), and `#11-clipboard-handlers-missing-on-window` is slotted (PM decision 764; the T3-a memo Q-A4); **new §4.1** decides once the final
shape of the two functions both PRs edit; §8.1's applicability bullet, §1.2's T3 and slice b rows and §10.2 / §10.3
follow; §1.5's G5 / K3, K2 and X-W bullets and §8.2's defects (2), (3), (7) and classification now point at T3 memo
§8.3 item 2 instead of restating it (R2 Axis 5 MIN); §1.5's heading and its K-2 / K-3, G2 rest, post-bind width cap
and W-K5 bullets and §8.2's defects (1), (4), (5), (6), closing path, trigger and confidence point at T3 memo §8.3 item
2, the one re-derivation of what T3's grown scope changes (R1 F3); §3's §13.2.6.4.4 row reads step 14 (R1 M13); §0.6
S-1, §5.1's DOMParser row, x3, §9 N-6 and §11's O9 line cite the T3-pre memo for the event. Edited in place, each with its
pre-8.<t> wording kept: X-W *Status*; K-0's X-W paragraph and "K-0's wording and X-W"; the K-1 / K-2 qualifiers; §0.6's
lead, S-1, S-5 and S-6; §1.1 I4; §1.2's T3 and slice b rows (scope); §1.3 x3, x4 (PM decision 711), x6 (closure), new x8 (C1 → T3), new x11 (RC-2a → T3, PM decision 359) and new x12 (L3 io → T3, PM decisions 711, 714); §1.4
(1) (§1.4's order sentence is C0a's, rev 8.4; T3's version of it was dropped when T3 rebased, T3 memo rev 4.5); §1.5's heading and its K-2 / K-3, G5 / K3, K2, G2 rest and X-W bullets; §2.1 I1×I4 and
I4×I5; §3's §13.4 row (PM decision 705) and §13.2.6.1 row (step number, PM decision 711); §5.1's `cloneNode`, fragment, DOMParser, declarative shadow and template contents rows; C19b's and C19c's *Spec* notes; §7 G3, G4 and G5; §8.1 (first bullet, and the two slots
T3 registers at landing: `#11-htmlframesetelement-interface`, PM decision 130, and the applicability slot); §8.2 defects (1),
(2), (3), (7) and the classification; §9 N-4 (b) and N-6; §10.2; §10.3; §11's O9 line. Not carried here (PM decision 223 O-1 as
amended by 271: each row and edge by its own PR): the C0 program row, the order line, x9, x10 and the order's other
consequences (C0a's PR, C0 program memo D-9 / R-3), the C1 row and x7 (C1's PR, C1 memo R-1), the C1-pre row and the
C1-pre → C1 edge (C1's PR). **With the T3-pre memo rev 2.4 (PM decision 774; re-check F3):** x14 says T3's landing
ends the no-op window unless the custom-element or form slot's fix has ended it first, and points to the T3-pre memo
DoD item 7 for the window's ends, sequence and bound and to its §8.3 item 3 for the escalation (the T3 memo §8.2's
T3-pre entry is the model); the Order line's "when it is raised" points to §8.3 item 3 likewise.
*C1-pre row* (carried by C1's PR; missing since C1-pre's PR, #616, PM 604): §1.2's C1-pre row landed with #616 (c7cce8c8; C1 memo §8 R-1, §9 item 11). *Rev 8.7* (carried by C1's PR; number assigned at landing, PM 223 O-1 / 232): §1.2 gains the C1 row and §1.3 gains x7 (C0 → C1) and the C1-pre → C1 edge (C1 memo R-1); §1.6's heading "C1's handling (decided by the PM, not here)" → "C1's handling (decided: branch A, PM decision 123)"; §8.1's `#11-template-contents-owner-document` "Owner TBD …" → the Layout lane's owner line; cell C14's expected column, K-5, §8.2 defect (5), §1.4's interim line (window iv), §1.5's W-K5 bullet and heading, §1.1's I7 row and §0.5's K-5 red-cell line carry C1's template-contents half (C1 memo R-3). *Rev 8.6* (2026-11-08; carried by C0c's PR): §1.2's C0 row: "C0c: planned." → "C0c: landed (#611)." (C0 program memo R-3). *Rev 8.5* (2026-11-07; carried by C0b's PR): §1.2's C0 row: "C0b: planned." → "C0b: landed (#608)." (C0 program memo R-3). *Rev 8.4* (2026-11-05; carried by C0a's PR): §1.2 gains the C0 program row and its order line follows PM 153; the header, the T3 and slice b rows, x6, the closing paths, §1.6, G3, §8.1 and §8.2 follow the new order and PM 123; edges x9 and x10 added (PM decisions 123, 153, 159, 223 O-1 as amended by 271, 227 O-1, 244, 271). *Rev 8.3* (2026-10-31; carried by N1's PR with N1 memo rev 4.5; N1 memo §10.5, Step 4.5 MIN-1 and
MIN-2): the header's N1 pointer is rev 4.5; §1.4 (i), §1.5's K-2 / K-3 bullet, x1 and §8.2's defect (1) and
classification name NR8 (a)'s population by its value, a callback at the first insertion (an IDL write, or a
`setAttribute` value compiled by a read or a dispatch before it), not by the IDL write alone; §8.4 item 4 excepts the
DOMParser-document element-targeted pairs (§1.4 (iii)).
*Rev 8.2* (2026-10-31; carried by N1's PR with N1 memo rev 4.4, PM decision 93; plan-review R2 on N1
rev 4.3, N1 memo §10.4): the header's N1 pointer is rev 4.4; §L's closing sentence and x1 mark a §L row's element at
its first processing (T-a1, T-a2 or T-a3); §1.4's interim line names the three windows whose getter N1 changes (the
K-2 / K-3 residual's IDL-written subset, away from the spec; W-K5's DOMParser half and X-W's element-targeted pairs,
to the spec), with §1.5's heading and its K-2 / K-3, X-W and W-K5 bullets qualified to match, and §1.1's I7 row
noting W-K5's N1 change; §8.2 says "ordinary" where the ledger copy does, adds NR8 (a)'s detached-owner variant to
defect (1), scopes defect (5)'s N1 qualifier to its second clause, lists the three subsets in the classification, and
points the trigger's landings to the ledger's Source line.
*Rev 8.1* (2026-10-31; carried by IBP-sandbox N1's PR, branch `layout-ibp-sandbox-handler-gates`,
PM decision 93): PM decision 90 (the user's choice B1). §L's closing sentence and x1 hold per row and name N1 memo §5.4
NR8 (a) as the mechanism-level counterexample; O5 is decided (§11); §1.4's interim line, §1.5's K-2 / K-3 residual,
§8.2's defect (1) and classification, and §8.4 items 3 and 7 carry the N1 qualifier.
*Rev 8* (2026-10-26): the rev-7 focused re-check (CRIT 0 / IMP 3; §13.3), scope limited by the user via the PM
to its three IMPs and A3-MIN-4. D-2 is aligned to X-W (Window-delegating pairs are not processed at T-a3;
cell C20). T3 must carry facets 1, 3 and 4 (X-W Status, §1.2, x3, §8.1, §10.3). New cells C19b (reflecting set on
`<body>`) and C19c (`<frameset>`). The option text of 案 X is quoted at X-W. *Rev 7* (2026-10-26): the user's decision 案 X and the rev-6 focused re-check (CRIT 0 / IMP 2 / MIN 13,
11 after dedup; §13.2). Both IMPs had one root: slice a's once rule on a `<body>` depends on step 3 of target
determination (T3), which slice a lacks. D-8 (b)'s document test is replaced by the property rule **X-W** (§0.5): a
Window-delegating pair, judged on (tag, handler name) alone, stays on HEAD's path at every slice-a timing except the
bind pass. New cells C18, C19; C8 and U2 become window pins. *Rev 6* (2026-10-25): PM decision 25 and R3 (CRIT 0 / IMP 1 / MIN 16; §13.1). The R3 IMP is fixed
(D-8 (b), K-0, I2×I8, cell C17). T3 is slice b's independent prerequisite PR, owned by this program (§1.2, §1.4,
§8.1). The CSP exposure is E-G1 (iii). New: U12, S-7. *Rev 5* (2026-10-25): PM decision 24. The R2 review (CRIT 0 / IMP 4 / MIN 14 distinct) found all four
IMPs in text that rev 4 / 4.1 added, three of them slice-b or branch-B mechanism decided below this umbrella's altitude.
Rev 5 changes the instrument: slice a is decided as invariants plus cells, and slice b as invariants only. Mechanism
detail moves to §9's input notes. The bind walk becomes uncapped (D-11; cells U10, U11). The bind pass is keyed on the
bound document (D-5). U9 gets a real two-bracket end-to-end twin (E8). O10 is decided. §13 disposes of every R2 finding.
*Rev 4 / 4.1* (`a7feb4ba`, `ab93a51a`): PM decision 23, R1 fixes, §L, K-0, D-5, D-10. *Rev 3* (`399f4534`): PM decision 21
(program, slices a / b). *Rev 2* (`00b863dd`): PM decision 18 (landing-order comparison, G1 in scope). *Rev 1*
(`28efbb67`): first draft, label `EH-insert-once` (retired, with its file name). The branch `layout-eh-insert-once`
keeps its name and carries docs commits only.

## §L. Landing-order verdict (PM decision 23)

**Question.** Is there a landing order that puts IBP-sandbox N1 before slice a and is no worse than HEAD on any
observable case? (i) N1's plan-review R1 F1 said that, under N1's D-N3 arm 3, the Insert arm's re-raw on every move makes
a compiled or IDL-set handler that is moved into a DOMParser document disappear. (ii) Rev 2 listed "T3 → N1 → C1 → the
whole work" as a viable landing.

**Columns.** *HEAD* = `fd6955e8`. HEAD has two gates. The compile gate is flag-only: `ensure_event_handler_current`
returns before any read only when the realm lacks `allow-scripts` (`event_handler_attrs.rs:550-556`). The main page and
a DOMParser document share the page realm, so this gate never fires for either. The invoke gate is the WHATWG HTML
§8.1.3.4 Enabling and disabling scripting composition with the node clause (`scripting.rs:83-113`). It is evaluated
**before** the reconcile on every dispatch path (`dispatch_target.rs:211`, `engine.rs:215`, `natives_promise.rs:540`).
The getter is `read_event_handler` (`event_handler_attrs.rs:638`). It reconciles (`:649`) and passes no invoke gate.

*N1-alone* = N1 rev 3 applied to HEAD without slice a. N1 deletes the invoke gate (D-N1) and makes the compile gate the
composition (D-N2). It also reorders the reconcile so that arm 3 drops a stale callable when the value is raw and the
gate fires, keeping the source (D-N3, N1 memo `0f99538d:261-272`). The consumer's Insert arm is HEAD's in both columns:
every insertion, a move included, calls `set_uncompiled` for every `on*` attribute (H1). `set_uncompiled` leaves the
stored callable in place (`event_listener.rs:207-219`), so after a move the callable is stale and the value is raw.

*Spec* = WHATWG HTML §8.1.8.1 Event handlers: an insertion runs no event-handler steps, so a callback value survives
the move. N1's D2 (an approved node-memo decision) says a callback value runs in a DOMParser document (N1 memo C8b).

`dp` is a DOMParser document. Its consumers are restored after the build (`inert_document.rs:198-199`), so a later
`dp.body.append(x)` fires Insert (H2). `el`, `el2` are live elements.

| # | scenario, step | HEAD | N1-alone | spec | N1-alone vs HEAD |
|---|---|---|---|---|---|
| L1 | setup: live `t`; `t.setAttribute('onclick','A')`; dispatch (compiles `fA`, runs) | `fA` runs | `fA` runs | `fA` runs | equal |
| L1.1 | `dp.body.append(t)`; getter `t.onclick` | Insert re-raws `A`. The compile gate does not fire, so the getter compiles a **new function `fA′`** | Insert re-raws `A`; arm 3 (raw, node clause fires) drops `fA`: **`null`** | `fA` (a function) | **worse**: HEAD's `typeof` is `'function'` like the spec's, and has `A`'s behaviour; N1-alone's is `null` |
| L1.2 | dispatch `click` on `t` | invoke gate suppresses: nothing runs | arm 3 → `null`: nothing runs | `fA` runs | equal |
| L1.3 | `el.append(t)`; getter; dispatch | re-raw; a new function; `A` runs | re-raw; arm 4 compiles a new function; `A` runs | `fA`; `fA` runs | equal |
| L2 | setup: `el.innerHTML='<div id=d onclick="A">'`; `d.onclick = fn` | `d.onclick === fn` | same | same | equal |
| L2.1 | `dp.body.append(d)`; getter | re-raw `A`; the getter compiles `A` over `fn`: **a function (not `fn`)** | re-raw `A`; arm 3 drops `fn`: **`null`** | `fn` | **worse**: N1-alone's value is not closer to `fn` than HEAD's, and it loses `typeof === 'function'`, which HEAD and the spec share |
| L2.2 | dispatch | nothing (invoke gate) | nothing (arm 3) | `fn` runs | equal |
| L2.3 | `el.append(d)`; getter; dispatch | re-raw; a new `A` function; `A` runs | re-raw; arm 4; `A` runs | `fn`; `fn` runs | equal |
| L3 | live → live after an IDL set: L2's setup, `el2.append(d)`; getter; dispatch | re-raw; a new `A` function; `A` runs | the same (live: neither clause fires; arm 4) | `fn`; `fn` runs | equal |
| L4 | identity live → live: `f = t.onclick` (compiled); `el2.append(t)`; `t.onclick === f` | `false` | `false` | `true` | equal |
| L5 | R-a (N1 R2): `c = createElement('div')`, never inserted; `c.setAttribute('onclick','A')`; dispatch (detached, page-owned: compiles `fA`, runs); `f = c.onclick`; `dp.body.append(c)`; getter | re-raw; a new function `fA′` | re-raw; arm 3 drops `fA`: **`null`** | `f` | **worse** (as L1.1) |
| L5.1 | then dispatch | nothing | nothing | `f` runs | equal |
| L6 | R-b (N1 R2): `dp` `<div id=d onclick="A">` (no `EventListeners` entry, H5); `d.onclick = fn`; getter; dispatch | `fn`; the invoke gate suppresses | `fn`; `fn` runs (C8b (i)) | `fn`; `fn` runs | closer |
| L6.1 | (i) `dp2.body.append(d)`; getter | re-raw `A`; compiles `A` over `fn`: a function | re-raw `A`; arm 3 drops `fn`: **`null`** | `fn` | **worse** (as L2.1) |
| L6.2 | (i) then dispatch | nothing | nothing | `fn` runs | equal |
| L6.3 | (ii) `el.append(d)`; getter; dispatch | re-raw; `A` function; `A` runs | the same | `fn`; `fn` runs | equal |
| L6.4 | (iii) `c = d.cloneNode(); c.onclick = fn; el.append(c)`; getter; dispatch | re-raw over `fn`; `A` runs | the same | `fn` runs | equal |
| L7 | shadow-root-direct child: live `x` with compiled `fA` (as L1's setup); `sr.appendChild(x)`, `sr` the shadow root of a live host; getter; dispatch | no Insert (`mutation.rs:289`); `fA`; `fA` runs | the same (arm 2) | `fA`; `fA` runs | equal |
| L7.1 | as L7, but the host is in `dp` | `fA`; the invoke gate suppresses | arm 2: `fA`; `fA` runs | `fA`; `fA` runs | closer |
| L8 | N1's C8c shape: live `A` compiled; `dp.body.append(t)`; `t.setAttribute('onclick','B')`; getter; dispatch; then `el.append(t)`; dispatch | getter compiles `B` (a function); nothing runs; then `B` runs | getter `null`; nothing runs; then `B` runs | `null`; nothing; then `B` runs | closer, then equal. Only C8c's second half stops discriminating (N1 NR7), a test-quality loss with no observable regression |

**Verdict: NO such order exists.** Rows L1.1, L2.1, L5 and L6.1 decide it. They are one mechanism: a compiled or
IDL-set callback on an element that carries an `on*` content attribute, read after a move into a DOMParser document.
HEAD returns a function there: the right source with a new identity (L1.1, L5), or the attribute's function in place of
`fn` (L2.1, L6.1). N1-alone returns `null` and drops the stored callable, where the spec returns the callback. In every
row dispatch is equal, so the regression is on the getter. It sits squarely on the population N1 exists to fix (its
D2 / H4: a callback value persists). Every order that lands N1 before slice a passes through the N1-alone state on
main. Rev 2's landing is one of those orders, so its "viable" is withdrawn. Claim (i) holds; claim (ii) is withdrawn.
After slice a these four rows give the spec's callback under N1: each row's element carries one deliverable ordinary
handler attribute, so slice a marks it before the move (at its first processing: T-a1, T-a2 or T-a3) and K-1 then
removes the re-raw. Slice a does not remove the mechanism for every element. Counterexample (N1 memo §5.4 NR8 (a)): an element given two or more deliverable
ordinary handler attributes by a silent writer, IDL-written before its first processing (D-8 (a)), then first inserted
into a DOMParser document. Its re-raw stays until slice b's S-5, so N1's getter reads null there where HEAD's returns a
function. O5 records the decision on it (PM decision 90). R2 re-derived the verdict from code and confirmed it.

PM decision 23: with the verdict "no order", slice a goes first as an approved interim, required by dependency (§1.4).

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` is a **`fd6955e8`** coordinate (the branch cut). `git diff --stat fd6955e8 a116b2c3 -- crates` (origin/main
on 2026-10-25) is empty, so the coordinates hold on main too. Every code reference comes from `/usr/bin/grep -n` or
`sed -n` over that tree, run on 2026-10-25. Every § number ↔ title pair and every algorithm step comes from
`.claude/tools/webref` (`heading`, `body`), run on 2026-10-25. N1's memo is cited at its rev 3 head `0f99538d`. The
IBP-sandbox umbrella is cited at the same commit. Both are read-only here.

### §0.2 Premises

| id | claim | evidence (`fd6955e8`) |
|---|---|---|
| H1 | The Insert arm re-delivers baked `on*` attributes on **every** `MutationEvent::Insert` | `handle` (`event_handler_consumer.rs:373`) routes `Insert` (`:381`) to `handle_insert` (`:427`). That walks `node` plus `traverse_descendants`, reads every `Attributes` entry named in `EVENT_HANDLER_ATTRS` (`:61`) and calls `set_inline_handler` → `set_uncompiled` (`:474`). There is no state check |
| H2 | `MutationEvent::Insert` fires for every insertion, a move included, except when node or parent is a shadow root | `fire_after_insert`, early return at `mutation.rs:289`. The filter serves every consumer |
| H3 | A re-raw replaces a compiled or IDL-set callable at the next read | `ensure_event_handler_current` (`event_handler_attrs.rs:521`) takes the source, compiles it, and overwrites the callable. `set_uncompiled` keeps the stored callable until then (`event_listener.rs:207-219`) |
| H5 | The DOMParser build suppresses every consumer | `take_mutation_dispatcher` at `inert_document.rs:112`, restored at `:198-199` |
| H6 | Fragment parses are built with dispatch suppressed and fire Insert at the caller's placement | `begin_detached_fragment` (`teardown.rs:318`) takes the dispatcher (`:325`) |
| H9 | Production parses the document **before** the VM binds, and nothing processes the parsed handlers until an Insert (G1) | `build_pipeline_from_loaded` (`pipeline.rs:839`) receives a parsed `dom`, then binds in the initial-scripts bracket (`:355`). `ConsumerDispatcher::initialize_consumers` (`consumer_dispatcher.rs:135-137`) calls only the base-URL init |
| H10 | The existing shadow-including walker is capped; the light walker is width-capped | `for_each_shadow_inclusive_descendant` (`walkers.rs:33`) skips nodes deeper than `MAX_ANCESTOR_DEPTH` (`:39-41`) and enumerates children with `children_iter`. `traverse_descendants` (`walkers.rs:294`) uses `children_iter_rev`. Both iterators stop after `MAX_ANCESTOR_DEPTH` (= 10 000, `dom/mod.rs:45`) siblings (`navigation.rs:308-314`, `:323-329`). Neither walker visits template contents |
| H10a | An uncapped iterative shadow-including walk exists as a precedent | `for_each_uncapped_shadow_inclusive` (`teardown.rs:251`, private; doc `:243-250`) uses an explicit stack, `child_list_uncapped` (`navigation.rs:287`) for width and a `visited` set as the termination guard. It also pushes the shadow-root entity and the template-contents fragment (`:263-272`), so it is **not** slice a's walk set as it stands. Its tests: `despawn_subtree_tears_down_past_the_traversal_depth_cap` (`dom/tests/destroy.rs:381`), `…_past_the_breadth_cap` (`:419`) |
| H12 | Bind/unbind keeps the listener store and the Window entity | first bind creates the Window (`vm_api.rs:227`); later binds reuse it |
| H13 | **Every** bind builds a fresh dispatcher and re-runs `initialize_consumers`; every unbind removes it | `ConsumerDispatcher::new` (`vm_api.rs:292`), `initialize_consumers` (`:313`), install (`:314`); `clear_mutation_dispatcher` (`lifecycle.rs:34`). Each `PipelineResult::dispatch_event` / `eval_script` is its own bracket (`lib.rs:293`, `:307`). So consumer-struct state does not survive a bind |
| H13a | The bound document is passed explicitly at bind | `Vm::bind(…, document)` (`vm_api.rs:195-199`) hands it to `HostData::bind` (`:217`). The scripting gates read it as `bound_document` (`scripting.rs:86`, `:93-111`) |
| H14 | At `initialize_consumers` the Window entity exists | it is resolved or created (`vm_api.rs:222-229`) before the dispatcher is built |
| H16/H17 | Every shell builder parses first and binds in `run_scripts_and_finalize`, including the three iframe paths | main documents: `build_pipeline_from_loaded` (`pipeline.rs:839`). URL-loaded iframes: `load.rs:221` (same origin), `:327` (out of process). srcdoc / about:blank: `build_iframe_pipeline` (`load.rs:465`) → `build_pipeline_interactive_shared` (`:471`) |
| H19 | The VM holds the policy that the dispatcher is built with | `VmInner::spec_level_policy` (`vm/mod.rs:2715`) is passed at `vm_api.rs:296` |
| H20 | `AttributeChange` fires after the `Attributes` write | `set_attribute` (`attribute.rs:125`) |
| H21 | HEAD's two gates (§L columns) | as §L |
| H22 | Placement into template contents by `innerHTML` fires Insert | `placement_target` (`html_fragment.rs:93`), `append_child` (`:102`) |
| H24 | `native_body_weh_set` returns before any write when no Window is bound | early return at `event_handler_attrs.rs:468-470` |
| H25 | Handler compile uses the global scope only | `lazy_compile_handler` (`event_handler_attrs.rs:718`) |

### §0.3 Premise errors (reported)

- **P1:** `importNode` / `adoptNode` do not exist in elidex. Insertion adoption is slot `#11-cross-document-adopt-on-insert`.
- **P2:** the Window-reflecting body set is not delegated (G4). It belongs to facet 4 of
  `#11-event-handler-target-determination`.
- **P3:** the docstring at `inert_document.rs:78-84` overstates a scripting rule. Slice b's plan fixes it if slice b
  touches that file. Slice a does not touch it.
- **P17 (rev 5):** rev 4's §1.5 said "HEAD processes no pre-bind element, so the depth cap is HEAD-equal". That is
  **false**. HEAD processes a pre-bind element whenever it is inserted or moved after bind (H1). Rev 4's R1 disposition
  A2-MIN-1 said the Insert arm keeps "HEAD's uncapped walk". That is also false: `traverse_descendants` is width-capped
  (H10). Slice a keeps that walk exactly as it is (D-10), so it is HEAD-equal. The bind walk is now uncapped (D-11).
- **P18 (rev 5):** rev 4 keyed the bind pass on the `document_root()` singleton and rejected R1 A2-MIN-2 on the ground
  that no multi-document World exists. That rejection reason ignored H13a: the bound document is passed at bind and
  already read by the gates. D-5 now keys on it.

### §0.4 Decisions (slice a unless stated)

- **D-1, once-key.** `HandlerAttrsProcessed` is a unit marker component on the **origin** element, never on the
  handler's target (the Window for `<body>` delegation). Meaning: *this element carries no creation-time handler content
  attribute of an ordinary pair that is still unprocessed* (§0.5). It neither gates nor records a Window-delegating pair
  (X-W). It is sparse: it is inserted only on an element that had a handler content
  attribute when processed. Absent means unprocessed, which fails toward HEAD's delivery, never toward loss. That holds for an **unmarked** element
only. For a **marked** element a silent `Attributes` write is never processed again, so it fails toward loss. The one
such writer is `clone_attributes` onto an existing `dst` (`tree_clone.rs:59-72`), which has no production caller
(§5.0 (a)). The case is recorded, not handled: a future silent writer onto an existing element is O4's census
(§11), and S-7 re-checks it for slice b.
- **D-2, one processing function.** A single engine-independent function in `elidex-script-session` runs the attribute
  change steps (WHATWG HTML §8.1.8.1 Event handlers) for every handler content attribute of an **ordinary pair** that E
  carries, at most once per element (D-1). It applies the session's `SpecLevelPolicy` and takes the target from
  `resolve_handler_target`. Every processing timing calls it. **Window-delegating pairs (rev 8, aligned to X-W):** the
  once gate does not apply to them. At T-a1 and T-a2 they are written ungated, on HEAD's path. At T-a3 they are
  **not processed**: a write that passes D-8 on an element that also carries a Window-delegating attribute processes
  only E's ordinary pairs (cell C20).
- **D-3, timings.** Slice a has exactly the three timings of K-0. The end state is slice b's (S-1).
- **D-4, writers.** A handler write (the AttributeChange arm and both VM element setters) first ensures that E is
  processed, under D-8's restriction, and then writes.
- **D-5, bind (rev 5: keyed on the bound document).** At bind, the consumer processes the shadow-including tree of
  the **bound document**, the `document` passed to `Vm::bind` (H13a). It does this **once per document**, at that
  document's first bind, and marks the Document entity with `PreBindHandlerAttrsProcessed`, a unit marker component.
  A later bind of the same document finds the marker and processes nothing. The once-state lives on the entity, because
  the consumer is rebuilt at every bind (H13). The elements that exist at a document's first bind are exactly those
  created while no consumer existed: no script runs between the parse and that bind (H9). Elements created while
  unbound between brackets are G7, slot `#11-unbound-attribute-writes-bypass-consumers` (§8.1). D-5 relies on that slot.
- **D-6, VM surface.** The two element setters call the D-4 entry with `VmInner::spec_level_policy` (H19) before they
  activate the handler. In `native_body_weh_set` the call comes **after** the no-Window early return (H24), so a setter
  that writes nothing also processes nothing. The `<body>` receiver is the origin (D-1). Every name that setter serves
  is a Window-delegating pair, so in slice a its call processes nothing (X-W); the call site is kept for slice b (S-6).
  The VM does marshalling only (Layering mandate).
- **D-7, program shape (PM decision 21).** Two nested PRs, slice a and slice b, as in §1.
- **D-8, writer restriction (observable form; rev 7: (b) is X-W).** In slice a, a handler write on an unprocessed
  element E processes E's ordinary pairs first **only if both** hold: (a) the written name is E's only deliverable
  handler content attribute of an ordinary pair (deliverable means named in `EVENT_HANDLER_ATTRS` and installed by the
  policy); (b) the written pair is ordinary (X-W, §0.5). Otherwise the write processes nothing, which is HEAD's outcome,
  and E is left to T-a2, HEAD's own path. In the passing case the write then overwrites the only processed value, so the
  state right after the write equals HEAD's (K-0 T-a3). Rev 6's (b) tested E's node document; the rev-6 re-check found
  that every such test leaks another path (C17's DOMParser `<body>`, then C18's template contents), so (b) is now the property rule.
  Slice b removes the restriction (S-5), after T3. The code shape is input to slice a's plan.
- **D-9, G1 pins by path, red first.** Slice a carries end-to-end pins on three paths: the main document, a URL-loaded
  iframe, and a srcdoc iframe (§6.2). Each pin runs red on the pre-fix base with a green non-vacuity control, and that is
  recorded before the fix lands.
- **D-10, Insert walk set.** In slice a the Insert arm keeps HEAD's walk set exactly: `node` plus
  `traverse_descendants(node)`. That is the light tree, with HEAD's width cap (H10). The arm adds the once check per
  element. A shadow-including Insert walk is rejected (R11): it would process shadow content at a host's insertion,
  where HEAD processes none, and so overwrite a guarded write on a shadow-root-direct element (cells U9, E8).
- **D-11, bind walk (rev 5, R2 IMP 1; PM decision 24).** The bind walk is **uncapped in depth and in width**. It
  visits every light and shadow child of every node in the bound document's tree, and excludes template contents in
  slice a. It is **iterative** (an explicit work list, never recursion), on the H10a precedent. Termination on a
  malformed tree is by a visited set, not by a cap. Cells U10 (depth over 10 000) and U11 (one parent with over 10 000
  children) pin it. Whether slice a's plan generalises the private H10a walker (which also pushes template contents) or
  writes a sibling is input. "One issue, one way" weighs toward one uncapped walker.
- **D-12, approval vehicle (O10).** As the approval-scope paragraph states.

### §0.5 Slice a's contract

Definitions. For an element E, the **creation-time attributes** are the `Attributes` written without
`MutationEvent::AttributeChange` at E's creation: the four parser bakes and the clone copy (§5.0 (a)). **Handler state**
is the `EventListeners` entry for an event type on E's resolved target, together with its stored callable.
**Processed** means the attribute change steps have run for E's creation-time handler content attributes (D-1). An
**observable step** is an IDL getter read or a dispatch. A **Window-delegating pair** is an (element, handler name)
pair for which steps 1 and 2 of *determining the target of an event handler* (WHATWG HTML §8.1.8.1 Event handlers) do
not return the element: the element is a `body` or a `frameset` (its tag compared as HEAD's `is_body_element` compares
it, `event_handler_consumer.rs:337-341`), and the name is a WindowEventHandlers member or in the Window-reflecting body
element event handler set (`onblur`, `onerror`, `onfocus`, `onload`, `onresize`, `onscroll`). Every other pair is
**ordinary**. The test reads the tag and the name only, never a document, a position or a connectedness.

- **X-W, Window-delegating pairs stay on HEAD's path (rev 7; the user's decision 案 X).** The option text the user
  chose, verbatim: 「window に付くべき <body> の handler を、例外なく今と同じ経路（挿入時）に任せる。…よく分からない経路はすべて「今と同じ」に落ちるようにする（安全な側に倒れる）。…その代わり <body onload> などの扱いは今のままで、正しくなるのは T3 か 2 本目の PR のとき」.
  X-W's extension to insertion (T-a2: a Window-delegating pair is re-raw'd at every insertion, marked or not; C19, and
  C8 / U2 as HEAD-equal window pins) follows this text: 「例外なく今と同じ経路（挿入時）」 covers the insertion re-raw
  (PM ruling 10-26). Slice a's once rule (K-1),
  its write-time processing (T-a3) and its skip at insertion (T-a2) do not apply to a Window-delegating pair. Such a
  pair is written exactly where and as HEAD writes it: by the AttributeChange arm and the VM setters, and by the Insert
  arm at **every** insertion of HEAD's walk set (H1). Slice a adds one timing for it, T-a1: the bind pass delivers the
  pairs of the bound document's tree, which is active, so step 3 does not fire there and HEAD's target for a `<body>`
  WindowEventHandlers name is the spec's (E-G1 covers the rest).
  - *Why.* HEAD's target determination has no step 3 (`resolve_handler_target`, `event_handler_consumer.rs:328-334`),
    so HEAD's setters and its AttributeChange arm write the page Window, or the element, where the spec's step 3 makes
    the target null. HEAD's re-raw at each insertion then overwrites that write with the attribute. Any once rule on
    such a pair (processing at a write, or a skip at insertion) removes the overwrite and leaves a value that is neither
    HEAD's nor the spec's (C17, C18, C19).
  - *Fail-safe direction.* The test reads no document, so no path (a DOMParser document, template contents, a node
    document left stale by an insertion without adoption, or a path not yet known) can carry a pair past it, and a pair
    it classifies as Window-delegating gets HEAD's outcome at every step, which K-0 allows.
  - *`<frameset>`.* HEAD does not delegate it: `resolve_handler_target` tests `body` only, and the VM's delegation
    accessors are installed for the `HandlerScope::Window` rows on `HTMLBodyElement.prototype` only
    (`event_handler_attrs.rs:207-213`) with a `body` brand check (`:425`). It is included because the test is the
    spec's step 1, which names `frameset`, and inclusion can only yield HEAD's outcome.
  - *The six reflecting names.* HEAD scopes them `HandlerScope::Global` (`event_handler_consumer.rs:68`, `:90`, `:91`,
    `:99`, `:125`, `:126`), so HEAD's target for them is the `<body>` itself (G4), and their IDL setter is the element's
    own. They are Window-delegating by the test all the same: the spec's step 3 makes their target null in an inactive
    document, so a once rule leaks for them as for the WindowEventHandlers rows.
  - *Status.* X-W is not an exception to K-0, which holds literally (the outcome is HEAD's at every step). It is the
    named exception to K-1 to K-3 in slice a. Until rev 8.<t> this read "T3 (facets 1, 3, 4) then slice b (S-5) remove
    it". From rev 8.<t> (PM decision 129): **T3** (`#11-event-handler-target-determination`, facet 1 for step 3, facet 3
    for `<frameset>` and facet 4 for the reflecting set) gives every Window-delegating pair the spec's target, and ends
    X-W for the pairs HEAD targeted at the element: a `<body>`'s six reflecting names and every name on a `<frameset>`
    (T3 memo D-T7: delivered by the bind pass, the attribute change steps and the IDL setter only; never at an
    insertion or at slice a's first-write processing). X-W then covers only a `<body>`'s WindowEventHandlers names, the
    pairs HEAD already targeted at the Window, and **slice b (S-5)** removes that remainder. From T3, D-2's "at T-a2 they
    are written ungated" and the T-a2 row below hold for that remainder only. The closure record is
    `#11-eh-attrs-creation-time-call-sites` (§8.2 defect (7), restated from rev 8.<t>).

- **K-0, no step worse than HEAD.** For every element and every observable step, slice a's outcome equals HEAD's at
  the same step, or equals the spec's where HEAD's does not.

  One exception class exists, **E-G1**. A pre-bind (initial-HTML) element gets exactly the outcome that HEAD gives the
  same markup inserted after bind. Where that outcome diverges from the spec, the divergence is HEAD's existing post-bind
  defect, which the G1 fix now reaches. E-G1 has three members:
  - (i) The Window-reflecting body set lands on the `<body>`. For example, an initial `<body onfocus="…">` runs on a
    focus event dispatched at the body, where the spec puts it on the Window. This is facet 4 of
    `#11-event-handler-target-determination`. Likewise every Window-delegating pair of an initial `<frameset>` lands on
    the `<frameset>` (HEAD delegates `body` only, X-W), the same slot's `<frameset>` deviation.
  - (ii) The handler scope chain is missing. *Get the current value of the event handler* step 3.9 (WHATWG HTML
    §8.1.8.1 Event handlers) builds the function with a scope of document, form owner and element. It also gives
    `onerror` on a Window its five-parameter list. elidex compiles with the global scope only (H25), so an initial
    `<select onchange="form.submit()">` throws a `ReferenceError` on `form`. This is
    `#11-event-handler-attribute-edge-cases`.

  - (iii) The CSP inline check is not made. *Attribute change steps* step 5.1 (WHATWG HTML §8.1.8.1 Event handlers)
    returns when "Should element's inline behavior be blocked by Content Security Policy?" returns "Blocked". Since
    HEAD, no inline script or inline handler passes a CSP check, because no `script-src` handling exists in `crates/`
    (§3 row 1). Slice a only adds initial-HTML handlers to that set. Inline handlers set by script, or inserted after
    bind, already run without the check at HEAD, so this is HEAD's existing post-bind defect. Slot
    `#11-csp-inline-handler-enforcement`.

  Members (i) and (ii) are registered with the trigger "slice a's landing"; (iii) under its own slot (§8.1).

  **`<body>` / `<frameset>` Window-delegating handlers (X-W).** In slice a they keep HEAD's behaviour at every step
  after the bind pass. That is a named exception to K-1 to K-3, not to K-0 and not an E-G1 member, because the outcome
  is HEAD's. T3 (`#11-event-handler-target-determination`) corrects the pairs HEAD targeted at the element, and the
  target of the rest; slice b (S-5) corrects the timing of the rest (rev 8.<t>, PM decision 129; until then: "T3 then
  slice b correct it").

  **Slice a's processing timings (complete) and the cells that prove each no worse than HEAD:**

  | timing | what is processed | outcome vs HEAD | cells |
  |---|---|---|---|
  | **T-a1**, the bound document's first bind (D-5, D-11) | every element of its shadow-including tree, uncapped, template contents excluded; Window-delegating pairs are delivered too, outside the once rule (X-W) | HEAD processes none of them at this point, and no script has run since their creation (H9), so no handler state exists to overwrite. Every outcome is the spec's, or E-G1 | E1, E2, E4, E5, E6, E7, U7 (first call), U10, U11 |
  | (T-a1, later binds) | nothing | equal to HEAD, which also processes nothing at a bind. A guarded write on an unprocessed element survives | U7 (second call), U9, E3, E8 |
  | **T-a2**, Insert (D-10) | the unmarked elements of HEAD's own walk set, at the Insert at which HEAD processes them, with HEAD's policy filter and target; and the Window-delegating pairs of every element of that set, marked or not (X-W) | an unmarked element gets HEAD's outcome. A marked element's ordinary pairs are skipped where HEAD re-raws, and the skip is the spec's outcome. Its Window-delegating pairs are re-raw'd, as at HEAD. A host's insertion does not reach its shadow content (HEAD-equal) | equal: C6, C13, C14, C15, C8, C19, C19b, C19c, U2. Spec: C1, C3, U1. Shadow: U9, E8 |
  | **T-a3**, the first handler write (D-4, D-8) | the restriction passes: E's only deliverable ordinary attribute, which the write then overwrites. The restriction returns (two reasons: two or more deliverable ordinary attributes; or the written pair is Window-delegating, X-W): nothing | passes: the state right after the write equals HEAD's, and the later effect is T-a2's skip (spec). Returns: nothing is processed, so the write and the later insertion take HEAD's own path, T-a2, and the outcome is HEAD's | passes: U5 (i), C20 (a Window-delegating pair on the same element is not processed, D-2); later effect C2, C4, C5, C11. Returns: U5 (ii), C16, C17, C18 |

  **K-0's wording and X-W.** K-0 reads "slice a's outcome equals HEAD's at the same step, or equals the spec's where
  HEAD's does not". Under X-W a Window-delegating pair's outcome at every step is HEAD's, so K-0 holds literally and
  its wording is unchanged. Rev 6 instead let a `<body>` pass T-a3 inside the bound document and skipped every marked
  `<body>` at insertion; C18 and C19 show values that are neither HEAD's nor the spec's for both. The residual
  divergence from the spec is the X-W window (§1.5), closed by T3 (the pairs HEAD targeted at the element, and every
  pair's target) and slice b's S-5 (the remaining timing) (rev 8.<t>, PM decision 129; until then: "closed by T3 then
  slice b").

  No other timing exists in slice a. Nothing is processed at a placement under a shadow root (no `MutationEvent` fires
  there, H2, and nothing is hooked), at a read, or at an internal adopt (the marker survives `adopt_subtree`).
- **K-1, once.** E's creation-time handler content attributes are processed at most once. After that, no insertion,
  move or rebind writes handler state derived from them. A compiled callable keeps its identity across moves and
  rebinds, in the live document and into a DOMParser document alike. *From slice a, for every ordinary pair of every
  element; a Window-delegating pair keeps HEAD's behaviour (X-W). From T3, also for every Window-delegating pair HEAD
  targeted at the element (rev 8.<t>, PM decision 129). From slice b, for every pair (S-5).*
- **K-2, an AttributeChange write wins.** Processing never overwrites a handler written by `setAttribute` /
  `removeAttribute` of `on<T>` on E, including a callable compiled from it. *From slice a, for every ordinary pair of
  an element with at most one deliverable ordinary handler content attribute (D-8); a Window-delegating pair keeps
  HEAD's behaviour (X-W). From T3, also for every Window-delegating pair HEAD targeted at the element, on every element
  (rev 8.<t>, PM decision 129: after the bind pass nothing but the attribute change steps and the IDL setter writes such a
  pair). From slice b, for every element and pair (S-5).*
- **K-3, an IDL write wins.** Processing never overwrites a callable or null assigned through `E.on<T>`, or through a
  `<body>`'s delegated WindowEventHandlers accessor, whatever wrote E's `Attributes` silently. *Same qualifier as K-2;
  the delegated accessor serves Window-delegating pairs only, so K-3 covers it from slice b on.*
- **K-4, timing (bind arm).** For elements created with no consumer (the pre-bind parse), processing happens at the
  first bind of their document, before that bind's first script. It writes the internal raw uncompiled handler on the
  resolved target. *From slice a.* The creation arm is S-1.
- **K-5, inert documents (slice a's part).** The bind pass writes no raw value into a DOMParser document or into
  template contents: it walks only the bound document's tree, and skips template contents (cells U8 (i), (ii)). Slice
  a's Insert arm keeps HEAD's window **W-K5**, which is HEAD-equal: an unmarked element placed into template contents
  (`tpl.innerHTML`, H22, or a script append into `tpl.content`), or into a DOMParser document after its build, gets HEAD's
  raw values. They read non-null, and in template contents they run (window pins C14, C15). From N1, the DOMParser half reads null; from C1, the template-contents half
  reads null under the C1 memo's INV-C (§4.6 item 1: conditional on the contents fragment's node document; a contents
  fragment adopted into the bound document is bound, the spec's outcome). The spec's outcome everywhere is slice b's (S-1, S-2).
- **K-6, bind completeness (G1).** At the first bind of a document, **every** element of its shadow-including tree,
  at any depth and under a parent of any width, is processed before any script of that bind runs. Template contents are
  excluded in slice a. Later binds of the same document process nothing. *From slice a.* This is why the bind pass
  stays after slice b: it is the creation-equivalent point for the pre-bind parse.

Red cells per clause (§6): K-0 guards U5 (ii), U9, E8, C6, C13, C16 (no red: K-0 is HEAD-equality or improvement). X-W:
C8, C17, C18, C19, C19b, C19c, C20, U2 (guards and window pins). K-1: C1–C3, U1, E3. K-2: C4, C11, U5 (i). K-3: C5. K-4: E1, E2, U7. K-5: U8 (C15 a window pin until N1, whose node clause inverts it; C14 a window pin until C1, which inverts it to the spec's outcome, C1 memo T17). K-6: E1, E2,
E4–E7, U7, U10, U11, U12.

### §0.6 Slice b's invariants (decided; mechanism is slice b's plan)

Slice b's base contains slice a, N1, C0, C1 (branch A) and T3 (rev 8.<t>, PM decision 153; until rev 8.<t>: "slice a, N1
and T3, and C1's handling has been decided") (§1.3). Slice b's per-PR plan must state
how it satisfies each of the following, and its plan-review checks that:

- **S-1, spec creation-time behaviour per creation path.** For every creation path that runs while a consumer
  exists, E's creation-time handler content attributes are processed at E's creation, before any script can observe E.
  This is WHATWG DOM §4.9 Interface Element, *append an attribute* step 4, realised per path. The paths are the fragment
  build, the DOMParser build, the clone and declarative shadow content, and template contents per S-2. The result is
  the spec's target (step 3 of *determining the target of an event handler*, through T3) and the spec's read (null where
  the node document's browsing context is null, through N1 and S-2). Coverage is complete over each created subtree, at
  any depth and width, because insertion no longer backs it up. For the pre-bind parse, the creation-equivalent point
  stays the first bind (K-6). In branch A it extends to pre-bind template contents. In the end state, insertion processes
  no element that S-1 covers. HTML defines no event-handler insertion steps (WHATWG DOM §4.2.3 Mutation algorithms).
  Rev 8.<t> (PM decision 153): the **clone** path is T3's (T3 memo D-T10: every element of a clone, every pair class,
  shadow-including and template contents included, processed at creation through one session entry point); slice b's
  S-1 covers its other creation paths (until rev 8.<t> the clone was slice b's). PM decisions 699, 705 and 711: the
  **fragment** path is T3's too, whole (T3 memo D-T7, D-T12: every node a fragment parse creates, declarative shadow
  content and template contents included, every pair class, processed at creation by T3's arm, T3 memo §4.5, on the
  creation event of the T3-pre memo §4.2, PM decision 722), and so is the **DOMParser** build (its event is queued in
  the build's dispatcher window and dispatched at the window's close, the T3-pre memo §4.4; raw handlers in a document without a browsing context, compilation
  null, T3 memo D-T13). Slice b's S-1 keeps the declarative shadow content and template contents of the pre-bind
  document parse. (Until T3 memo rev 4.7: "the fragment path's creation in an active-document context is T3's too (…
  how much of the fragment path and its call site: T3 memo O-11)"; T3 memo rev 4.7: "every element of a fragment's
  insertion-time walk set …; the declarative shadow content and template contents a fragment parse creates are
  outside that walk set and stay slice b's".)
- **S-2, template contents and their clones follow C1's handling.** In branch A (C1 landed), they are processed at
  creation and read null. In branch B, slice b's plan decides how they are processed, subject to S-3 and S-4, and S-3
  allows **no** exception. S-1 processes a clone at creation unconditionally, and S-3 forbids a clone of template
  contents from reading a function where HEAD and the spec read null. So branch B requires slice b to **supply a
  per-node template-origin fact** that a clone and a clone of a clone both carry. The fact tracks the node's spec node
  document (an inert template contents owner document), not its history: a node moved out of template contents into
  another document no longer carries it, nor does a clone made after that move. Where its plan cannot supply it, the
  unsupplied case is not processed at creation and stays a §1.5 window, so S-3 still holds. Input N-2 (§9) records why
  a clone of template contents carries no such fact without C1 part (a).
- **S-3, no step worse than HEAD.** K-0 holds against HEAD at `fd6955e8` and against slice b's own base, for every
  element and every observable step, with no exception class beyond E-G1.
- **S-4, omissions are detected, never masked.** A creation path that slice b fails to process is detected by tests
  that read E before any insertion, and by a `debug_assert!`. No release path exists whose effect is to process an
  element that a creation path omitted. If slice b keeps any processing at insertion (branch B), its plan must show that
  this processing cannot serve as such a path.
- **S-5, writes win unconditionally.** K-1 to K-3 hold for every element and every pair: the D-8 restriction and X-W
  are gone, and Window-delegating pairs take the spec's target through T3. Rev 8.<t> (PM decision 129): the part of S-5
  for the Window-delegating pairs HEAD targeted at the element (a `<body>`'s six reflecting names, every name on a
  `<frameset>`) is T3's, so slice b removes X-W for a `<body>`'s WindowEventHandlers names only, together with the D-8
  restriction (until rev 8.<t> S-5 removed X-W for every Window-delegating pair).
- **S-6, slice a's surface is kept.** D-1's marker and meaning, D-2's function, the bind pass (D-5, D-11) and the
  writer call sites (D-4, D-6) are kept and extended, not replaced. From rev 8.<t> also T3's creation entry point
  (`process_created_subtree`, T3 memo D-T10): slice b's other creation paths call or extend it. What slice b removes is §1.4's list.
- **S-7, census and fail direction (restored from rev 4; R3 A3 MIN).** Slice b's plan re-runs the §5.0 census at its
  own base and shows that every creation path is covered by S-1, and that every silent `on*` writer is either such a
  creation path or a post-creation writer whose disposition its plan states (processing at the write, or a slot). The
  known post-creation member is `clone_attributes` onto an existing, possibly marked `dst` (D-1). In branch A nothing is processed at insertion, so an omitted path fails toward **never
  delivered**, worse than HEAD's delivery at first insertion. S-4 and this census are then the only protection.

## §1. Program shape (decided)

### §1.1 Invariant axes

| axis | source | slice a | slice b |
|---|---|---|---|
| I1 once (per element, across moves and rebinds) | K-1, H13 | owns it: marker, once check, document marker on the bound document (D-5) | extends it to creation time (S-1, S-6) |
| I2 writer order | K-2, K-3 | restricted (D-8; X-W) | full (S-5) |
| I3 origin key | D-1 | owns it | — |
| I4 freshness (new elements start unprocessed; clone does not copy) | §5.0 | owns it (clone-policy row) | clone processed at creation (S-1; rev 8.<t>: by T3, PM decision 153) |
| I5 coverage (every creation path, the pre-bind tree, shadow trees, template contents) | K-4, K-6, G1, G2 | the pre-bind tree, shadow-including, uncapped (D-11); first insertion over HEAD's light walk (D-10) | creation-time coverage (S-1, S-2) |
| I6 policy | `event_handler_consumer.rs:405`, `:447` | owns it (one `policy.installs` filter on every path) | — |
| I7 inert-document reads | K-5 | the bind pass reaches none (U8); W-K5 kept, HEAD-equal (its DOMParser half reads null from N1, §1.4 (ii); its template-contents half from C1, §1.4 (iv)) | owns it (S-1, S-2; needs N1 and C1's handling) |
| I8 target determination (step 3) | G3 | not reached: the bind pass walks only the bound, active document, and every Window-delegating pair otherwise takes HEAD's path (X-W; C8, C17–C19) | owns it (needs T3) |
| I9 layering / re-entry | Layering mandate | owns it for the VM setters and the bind pass | owns it for the creation-time sites |

**Count: 9 (≥ 3).** The work is edge-dense, so it is a multi-PR program, and each slice gets its own
`/elidex-plan-review` before implementation (CLAUDE.md *Edge-dense work* (a), (b)). Each slice is a narrowly scoped
per-PR slice of this approved umbrella (base case (c)), and its own plan-review decides whether it recurses.

### §1.2 Nested PRs

| nested PR | scope | prerequisites |
|---|---|---|
| **slice a** `EH-attrs-at-creation-a` (branch cut from main after the approval PR lands) | D-1 to D-11; K-0 to K-6; X-W; the cells and pins of §6.1–§6.2; the clone-policy row; G1; the G6 docstring | the umbrella-approval PR |
| **C0** `EH-attrs-at-creation-C0` (owner: Layout lane, this program; PM decisions 143, 244), a program of three PRs in order **C0a** `EH-attrs-at-creation-C0a` (*replace* record order) → **C0b** `EH-attrs-at-creation-C0b` (the node document as the stored fact: creation entry, insertion adoption, one reader) → **C0c** `EH-attrs-at-creation-C0c` (`adoptedCallback` and `document.adoptNode`); program memo `docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md` | slot `#11-cross-document-adopt-on-insert` (DOM §4.2.3 *insert* step 7.1, §4.5 *adopt*); C0b ends the stale node document after a cross-document move (a node moved into another document keeps its old node document, which T3's step 3 would read; edge x9). C0a: landed (#595). C0b: landed (#608). C0c: landed (#611). | IBP-sandbox N1's implementation landed; C0a after D-14 (a); C0b after C0a, D-14 (c), the CI/tools default-features check PR (#585); C0c after C0b and D-14 (b) |
| **C1-pre** `layout-c1-pre-document-wrapper` (owner: Layout lane; PM 328 / 335 / 339; memo `docs/plans/2026-11-eh-attrs-c1-pre-document-wrapper.md`) | slot `#11-bridge-document-post-install`: every Document wrapper is a Document (one `Document.prototype`, chosen at first allocation) | C0 D-14 (b) and RC-2a landed |
| **C1** `EH-attrs-at-creation-C1` (owner: Layout lane, this program; PM decision 123) | slot `#11-template-contents-owner-document`: the inert template contents owner document (HTML §4.12.3; branch A, §1.6); closes T3's residual R-A | IBP-sandbox N1's implementation and C0 landed; L3 RC-2a landed; C1-pre landed |
| **T3-pre** `EH-attrs-at-creation-T3-pre` (owner: Layout lane, this program; PM decision 722; memo `docs/plans/2026-11-eh-attrs-t3-pre-subtree-created-event.md`) | the creation event T3 consumes: RC-2a's `MutationEvent::CloneCreated` generalised to `SubtreeCreated { root, cause }` with every consumer migrated, the `FragmentParse` fire in the fragment parse's one return point, and the queue of a dispatcher-take window, flushed at the outermost close of L3's guard (the T3-pre memo §4.2–§4.6); behaviour-neutral at its landing (its §1.1); on L3's surface, so its plan-review takes L3's confirmation through the PM. Rev 8.<t> (PM decision 722: split out of T3, plan-review R1 F1 on T3 memo rev 4.8.2) | L3's PR (is) and PR (ig) landed (x13); RC-2a landed (x11); not ordered against L3's PR (io) |
| **T3-a** `EH-attrs-at-creation-T3-a` (owner: Layout lane, this program; PM decisions 741 (3), 767; memo `docs/plans/2026-11-eh-attrs-t3a-applicability.md`) | facets (1) and (3) of `#11-event-handler-content-attribute-applicability`: one predicate for "an event handler content attribute of this element" (the local-name half of the attribute change steps' step 1), applied by every reader of an element's handler content attributes (the T3-a memo D-T8). Target determination is untouched. Rev 8.<t> (PM decision 741 (3): carved out of T3, plan-review R2 T1 on T3 memo rev 5; PM decision 767: split from T3-ab, plan-review R3 wave 1 W1-1) | C1 landed (so every prerequisite is on main); no edge with T3-b, L3's PRs (is), (ig), (io), L3's tree-model program or T3-pre (the T3-a memo §8.1) |
| **T3-b** `EH-attrs-at-creation-T3-b` (owner: Layout lane, this program; PM decisions 741 (3), 767; memo `docs/plans/2026-11-eh-attrs-t3b-body-prototype.md`) | `HTMLBodyElement.prototype` and its receiver check for an HTML-namespace `<body>` only, both reading one statement of that fact (the T3-b memo D-T11). Target determination and the two processing functions of §4.1 are untouched. Rev 8.<t> (PM decisions 741 (3), 767, as the row above) | C1 landed (so every prerequisite is on main); no edge with T3-a, L3's PRs (is), (ig), (io), L3's tree-model program or T3-pre (the T3-b memo §8.1) |
| **T3** `EH-attrs-at-creation-T3` (owner: Layout lane, this program; PM decision 25) | slot `#11-event-handler-target-determination` facet (1), step 3 of *determining the target of an event handler*, with facets (3) (`<frameset>`) and (4) (the reflecting set): slice b needs all three (x3, S-5; rev 8). Facet (2) rides with it unless its plan splits it off (the T3 memo carries it). **Rev 8.<t> (PM decision 129):** also the part of S-5 that ends X-W for the pairs HEAD targeted at the element (a `<body>`'s six reflecting names, every name on a `<frameset>`). Its own plan, plan-review and PR. **Rev 8.<t> (PM decision 153):** also S-1's clone path (T3 memo D-T10) (until T3 memo rev 6: "and the HTML-interface choice for a foreign-namespace `<body>` (T3 memo D-T11)", now **T3-b**'s, PM decisions 741 (3), 767). (Until T3 memo rev 6, by PM decision 168: "also facets (1) and (3) of `#11-event-handler-content-attribute-applicability` (T3 memo D-T8)", now **T3-a**'s, the rows above.) **Rev 8.<t> (PM decisions 699, 705, 711):** also S-1's fragment path, whole: every node a fragment parse creates (declarative shadow content and template contents included) processed at its creation, every pair class (T3 memo D-T7, D-T12), and the DOMParser build (queued in its window, dispatched at its close), through `SubtreeCreated`, RC-2a's `CloneCreated` generalised (from T3 memo rev 5 by **T3-pre**, the row above, PM decision 722; T3 keeps the event-handler consumer's arm, T3 memo §4.5); T3 lands after T3-pre and L3's ecs-entity-lifetime PR (io) (x14, x12; until T3 memo rev 4.8: "every element of a fragment's insertion-time walk set"; until rev 5 the generalisation and the queue were T3's) | C1 landed (PM decision 153; slice a, N1's implementation and C0 precede it in the order line; until rev 8.4: "slice a landed"); T3-pre and L3's PR (io) landed (x14, x12; PM decisions 714, 722); T3-a and T3-b landed (x15, x16; PM decisions 741 (3), 767) |
| **slice b** `EH-attrs-at-creation-b` | S-1 to S-7, S-5 without T3's part (rev 8.<t>), S-1 without the clone path (rev 8.<t>) and without the fragment path and the DOMParser build (rev 8.<t>, PM decisions 699, 705, 711; until T3 memo rev 4.8: "without the fragment path except the declarative shadow content and template contents it creates"); G2 rest, G5's non-clone half, K2's X-W half (§1.5); slot `#11-event-handler-content-attribute-applicability` facet (2) (rev 8.<t>; it has no case at the base, and its prerequisites, trigger and owner question are the T3-a memo §8.3 item 1 and Q-A6; facets (1) and (3) are T3-a's, the T3-a memo D-T8, PM decisions 741 (3), 767; until T3 memo rev 6: T3's, PM decision 168) | T3, N1, C0 and C1 landed (PM decisions 123, 153; until rev 8.4: "T3, N1, C1's handling decided") |

**Order (PM decisions 21, 23, 25, 123, 153, 244, 714, 722, 741, 767; §L):** approval PR → slice a → IBP-sandbox N1 → **C0** (program: C0a → C0b → C0c) → **C1-pre** → C1 → { **T3-a** ∥ **T3-b** ∥ ((L3's PR (is) → PR (ig)) → **T3-pre** ∥ (L3's tree-model program → PR (io))) } → T3 → slice b (PM 153, user decision: N1 implementation → C0 → C1 → T3 → slice b; PM 244: C0 is a program of three PRs; rev 8.<t>, PM decisions 714, 722: T3-pre after ig, in either order with io; T3 after both; until rev 8.<t>: "C1 → T3 → slice b"; PM 730 placed L3's tree-model program between ig and io, and PM decisions 740 (1) and 741 (1) keep T3-pre's place: after ig, parallel with that program and io; PM decisions 741 (3), 767: T3-a and T3-b after C1, each ordered against nothing but T3, which both precede, x15 and x16; their condition on the no-op window is the T3-pre memo DoD item 7's, and when it is raised is the T3-pre memo §8.3 item 3's; neither is restated here (until the T3-pre memo rev 2.4 both were attributed to DoD item 7)).

### §1.3 Edges

| # | edge | kind | reason |
|---|---|---|---|
| x1 | slice a → IBP-sandbox N1 (external) | semantic precondition | §L: without slice a, N1 returns null from the getter of a compiled or IDL-set handler moved into a DOMParser document, where HEAD returns a function. Slice a's K-1 removes that for each §L row (one deliverable ordinary attribute, marked before the move at its first processing: T-a1, T-a2 or T-a3), not for N1 memo §5.4 NR8 (a) (two or more silent attributes, a callback before first processing, D-8 (a)), which keeps it until S-5 (O5, PM decision 90) |
| x2 | slice a → slice b | code predecessor + data flow | S-6 |
| x3 | T3 (`#11-event-handler-target-determination` facets 1 (step 3), 3 (`<frameset>`) and 4 (the reflecting set); **owner: this program, Layout lane**, PM decision 25) → slice b | semantic precondition | creation-time processing of a `<body>` whose node document at creation is not the active document (a DOMParser document's, or a fragment's parsed in such a document's context: a fragment's elements are created in the context's node document, HTML §13.4 step 2 and §13.2.6.1 step 6, not in the parser's own Document) would otherwise deliver its WindowEventHandlers attributes to the **page** Window (§9 N-4 (b); until rev 8.<t>: "a DOMParser or fragment-throwaway `<body>`"). S-5 drops X-W, so every Window-delegating pair must then take the spec's target: facet 4 for the reflecting six, facet 3 for `<frameset>` (rev 8). Rev 8.<t> (PM decision 129): T3 itself drops X-W for those pairs (the reflecting six on `<body>`, every name on `<frameset>`); S-5 drops the rest. Rev 8.<t> (PM decision 153): T3 also processes a clone at its creation (S-1's clone path), so slice b's S-1 starts from T3's entry point. Rev 8.<t> (PM decisions 699, 705): T3 also processes a fragment parse's elements at their creation (S-1's fragment path, T3 memo D-T12, through T3-pre's creation event, the T3-pre memo §4.2, consumed by T3's arm, T3 memo §4.5; PM decision 722) |
| x4 | N1 → slice b | semantic precondition | raw values written at DOMParser-node creation read null only under N1's compile-gate node clause (§9 N-4 (a)). Rev 8.<t> (PM decision 711): T3 writes them (T3 memo D-T13), and N1 precedes T3 in the order (N1 → C0 → C1 → T3, PM decision 153), so the clause holds for T3 too |
| x5 | C1's handling decided (`#11-template-contents-owner-document`) → slice b | decision precondition; in branch A also C1's landing | S-2 (§9 N-4 (c)) |
| x6 | slice a → T3 | scheduling precondition (PM decisions 25, 153) | T3 is started after C1 lands (PM decision 153; until rev 8.4: "once slice a lands"), so x6 holds transitively (slice a → N1 → C0 → C1 → T3). No code or semantic dependency is claimed. Slice a's X-W window (§1.5) is what T3 then slice b close (rev 8.<t>: T3 closes it for the pairs HEAD targeted at the element, slice b the rest) |
| x9 | C0 → T3 (rev 8.4) | semantic precondition (PM decision 153) | without insertion adoption a node moved into another document keeps its old node document, which T3's step 3 reads; C0b makes the node document a stored fact, kept true at every insertion and read by `EcsDom::node_document` |
| x10 | N1 → C0 (rev 8.4) | code predecessor (PM decision 153) | C0b rewrites N1's compile-gate clause (b) to read `EcsDom::node_document` (C0 program memo D-7 (a)), so N1's implementation (#580) lands first |
| x7 | C0 → C1 | semantic precondition (PM decisions 143, 153) | without insertion adoption C1 makes the most common template idiom worse than its base (C1 memo §1) |
| x<n> | C1-pre → C1 | semantic precondition (PM 328) | I is reachable from script through `ownerDocument`; its wrapper is a Document only through C1-pre's `Document.prototype` (C1 memo edge c-pre) |
| x8 | C1 → T3 (rev 8.<t>) | semantic precondition (PM decision 153) | without C1 template contents report the bound document, so T3's step 3 passes there and a Window-reflecting `<body>` / `<frameset>` handler written inside template contents reaches the Window (the T3 memo rev 2's residual R-A) |
| x12 | L3 ecs-entity-lifetime PR (io) (external) → T3 (rev 8.<t>) | code predecessor + semantic precondition (PM decisions 711, 714) | T3's fragment creation walk passes io's `WalkScope::WithTemplateContents` (NEW: io) to the `pub` `for_each_uncapped_shadow_inclusive` (T3 memo D-T12, §4.5). L3's order is is → ig → io → ii (PM decision 714). L3's names are cited by symbol and section from the PR (io) plan (`docs/plans/2026-11-ecs-entity-lifetime-pr-io-owned-entities-walk-scope.md`: `WalkScope::WithTemplateContents`, `for_each_uncapped_shadow_inclusive`) and the L3 umbrella (`docs/plans/2026-11-ecs-entity-lifetime-umbrella.md` §5 *Order*), read at the pre-ratification snapshot T3 memo H-T32 names, and re-pinned to L3's ratified sha before T3 is ratified (T3 memo R7). Until T3 memo rev 5 x12 also carried the flush at ig's outermost close, now x13's (until T3 memo rev 4.8.1: names from L3's pre-split PR (i) plan) |
| x13 | L3 ecs-entity-lifetime PR (ig) (external) → T3-pre (rev 8.<t>) | code predecessor (PM decisions 713, 714, 722) | T3-pre's creation-event queue is flushed at the outermost close of ig's dispatcher-take guard, a change T3-pre makes to `DispatcherTake::close` (NEW: ig), relying on ig's guarantees (a) and (b) (the PR (ig) plan `docs/plans/2026-11-ecs-entity-lifetime-pr-ig-dispatcher-take-guard.md` §8 *T3 intersection*; the T3-pre memo §4.4, H-P5, cited at its snapshot and re-pinned, its R-P3) and, from the T3-pre memo rev 2, on ig's private restore function writing the slot directly and on `set_mutation_dispatcher`'s assertion while `Taken`, both ig's by PM decision 740 (2) (the T3-pre memo §4.4 (c) to (f)); T3-pre also changes that restore function (it gains the queue as its return value). No edge between T3-pre and L3's PR (io): T3-pre does not use `WalkScope` |
| x11 | L3 RC-2a (external) → T3-pre (rev 8.<t>; until T3 memo rev 5: → T3) | semantic precondition (PM decisions 359, 722) | T3-pre generalises RC-2a's `MutationEvent::CloneCreated` (landed by #596) to `SubtreeCreated`, keeping its `Clone` cause as RC-2a's (the T3-pre memo I-RC1 to I-RC3); T3's clone arm (T3 memo D-T10) consumes it and walks RC-2a's uncapped `for_each_clone_created` (PM 367), so RC-2a precedes T3 through x11 and x14 |
| x14 | T3-pre → T3 (rev 8.<t>) | code predecessor + semantic precondition (PM decision 722) | T3's consumer arm matches T3-pre's `SubtreeCreated` (T3 memo §4.5), and DOMParser's build reaches it only through T3-pre's queue (T3 memo D-T13); T3's landing ends the no-op window of T3-pre's `FragmentParse` arms unless the custom-element or form slot's fix has ended it first (the T3-pre memo DoD item 7: the window's ends, its sequence ig → T3-pre → tree-model program → io → T3, and its bound; its escalation: the T3-pre memo §8.3 item 3) (PM decisions 740 (1), 741 (1); until the T3-pre memo rev 2.4: "T3's landing ends the no-op window …, whose sequence …, bound and escalation are the T3-pre memo DoD item 7"; until the T3-pre memo rev 2: "T3 is Layout's next unit after T3-pre") |
| x15 | T3-a → T3 (rev 8.<t>; until PM decision 767: T3-ab → T3) | code predecessor + decision precondition (PM decisions 153, 741 (3), 767) | T3 rewrites the two functions T3-a edits (§4.1: T3-a's parts are in their final place, T3 adds around them), and T3's no-worse analysis is made on a base that has T3-a (T3 memo §2.1); PM decision 153's strict reading closes R-D2 before or in T3, and T3-a closes it before. T3-a has no other edge: it reads no creation event, dispatcher window, walker or tree link (the T3-a memo §8.1, H-A9), so it is unordered against T3-b, is, ig, T3-pre, the tree-model program and io. Its bearing on the no-op window is stated only in the T3-pre memo DoD item 7 (PM decisions 756, 767) |
| x16 | T3-b → T3 (rev 8.<t>; PM decision 767) | decision precondition (PM decisions 153, 741 (3), 767) | PM decision 153's strict reading closes R-D1 before or in T3, and T3-b closes it before; T3's no-worse analysis is made on a base that has T3-b (T3 memo §2.1, row FN-3; §1.3 seam S1). No code contact: T3 edits neither of T3-b's two sites, and T3-b touches neither function of §4.1. T3-b has no other edge (the T3-b memo §8.1), so it is unordered against T3-a, is, ig, T3-pre, the tree-model program and io. Its bearing on the no-op window is stated only in the T3-pre memo DoD item 7 (PM decisions 756, 767) |

Acyclic. The order satisfies x4 transitively. It is listed so that a re-ordering cannot drop it. x7 (C0 → C1) and x8 (C1 → T3) are added by the PR at each edge's head (C0 program memo D-9).

### §1.4 Slice a is an approved interim, required by dependency (PM decision 23)

CLAUDE.md "One issue, one way" forbids a strangler state, in which a new mechanism coexists with the one it replaces.
Slice a is such a state for **timing**: it processes post-bind creations at first insertion, where slice b's end state
processes them at creation. The D-8 restriction exists for the same reason. The PM approves slice a as an interim
because a dependency requires it. N1 requires K-1 (§L), and slice b requires T3, N1 and C1's handling (x3–x5), and T3
is started after C1 lands (x6, x8; PM decision 153). So the only order that satisfies both is slice a → N1 → C0 → C1 → T3 → slice b
(PM decision 153; until rev 8.4: "slice a → N1 (with T3) → slice b").

- **Kept by slice b:** S-6.
- **Removed by slice b (the interim's whole surface):** (1) the D-8 restriction and X-W (S-5; from rev 8.<t>, X-W for a
  `<body>`'s WindowEventHandlers names only, since T3 removes it for the pairs HEAD targeted at the element, PM
  decision 129); (2) processing at
  insertion, for every element that S-1 covers. In branch A, nothing is processed at insertion. Whether any processing at insertion
  remains in branch B is slice b's plan's decision, under S-2 to S-4.
- **Interim behaviour:** §1.5, all HEAD-equal (K-0) until N1 lands. From N1, N1's compile-gate node clause changes
  the getter in three of its windows, each where the node document N1 reads is a DOMParser document's, with dispatch
  equal to HEAD's (nothing runs) in each, and from C1 in a fourth: (i) the K-2 / K-3 residual's subset whose value is a callback at that insertion (an IDL
  write, or a `setAttribute` value compiled by a read or a dispatch before it), at a first insertion into a
  DOMParser document (or of a DOMParser document's element into a detached subtree of the bound document, which
  insertion does not adopt), reads null where HEAD compiled the attribute and the spec returns the callback,
  away from the spec (N1 memo §5.4 NR8 (a); O5, PM decision 90); (ii) W-K5's DOMParser half reads null where HEAD
  compiled the attribute, the spec's outcome (K-5); (iii) X-W's element-targeted pairs (a Window-reflecting name on a
  `<body>` / `<frameset>`, or a WindowEventHandlers name on a `<frameset>`, which HEAD targets at the element) in a
  DOMParser document read null where HEAD compiled the attribute, the spec's outcome (N1 memo §5.4 NR8 (b), pin C8h); (iv) from C1, W-K5's template-contents half, where the node document N1 reads is the contents' inert template document (C1 memo INV-C, §4.6 item 1), reads null where HEAD compiled the attribute and dispatch runs nothing where HEAD ran the handler, the spec's outcome (K-5; cell C14).
  **Closure record:** slot `#11-eh-attrs-creation-time-call-sites` (§8.2; (i) to (iv) are its defects (1), (5), (7) and
  (5)'s template-contents half respectively).
- **Closing path (visible):** slice a lands → N1 → C0 → C1 → T3 (this program's PR, slot `#11-event-handler-target-determination`) → slice b → the PM closes the slot above (§1.3 x6, x3–x5, x2, x9, x10; rev 8.4, PM decision 153; until rev 8.4: "slice a lands → T3 (…; N1 lands independently) → C1's handling decided (PM) → slice b").

### §1.5 Windows slice a leaves open (each HEAD-equal until N1 lands, three DOMParser subsets then excepted, and from C1 a fourth, W-K5's template-contents half, §1.4; slice b closes them, except X-W's part and what T3's creation processing covers, the clone, fragment-parse and DOMParser-build windows, which T3 closes; rev 8.<t>, T3 memo rev 5: what T3 changes in each bullet is re-derived once, in T3 memo §8.3 item 2, and the bullets point there; until T3 memo rev 5: "except X-W's part and the clone windows")

- **K-2 / K-3 residual.** A handler write on a post-bind element with **two or more** deliverable handler attributes,
  made before the element's first insertion, is overwritten at that insertion (D-8). HEAD overwrites it at every
  insertion. From N1, such a write whose value is a callback at that insertion (§1.4 (i)), so overwritten at a first
  insertion into a DOMParser document (or of a DOMParser document's element into a detached subtree of the bound
  document), reads null, where HEAD's getter compiles the
  attribute (N1 memo §5.4 NR8 (a); O5; §1.4 (i)). Rev 8.<t>: not for an element a clone, a fragment parse or a
  DOMParser build created, which T3 processes and marks at its creation (T3 memo §8.3 item 2 (a); until T3 memo rev 5:
  "not for a clone").
- **G5 / K3.** A never-inserted clone reads null. Rev 8.<t> (PM decision 153): closed by **T3** (T3 memo §8.3 item 2
  (b)); until rev 8.<t> slice b.
- **K2.** The body-clone cross-origin write is overwritten at the clone's first insertion. Rev 8.<t>: what T3 changes
  here is T3 memo §8.3 item 2 (c); what then remains (X-W's part) is closed by slice b's S-5.
- **G2 rest.** An element placed directly under a shadow root after bind, or inside a subtree placed there, is not
  processed. Neither is the shadow content of a host inserted after bind. Rev 8.<t>: an element a clone or a fragment
  parse created is processed at its creation by T3, a clone's replicated shadow tree, `ShadowRoot.innerHTML` and
  `setHTMLUnsafe`'s declarative shadow content included (T3 memo §8.3 item 2 (d); until T3 memo rev 5: the clone only).
- **X-W (G3, G4).** A Window-delegating pair keeps HEAD's behaviour after the bind pass: it is re-raw'd at every
  insertion, over any write, and a `<body>` outside an active document delivers it to the page Window (C8, C17–C19,
  U2). From N1, an element-targeted pair in a DOMParser document reads null, where HEAD compiled the attribute: the
  spec's outcome (§1.4 (iii)). Until rev 8.<t>: "T3 then slice b close it". From rev 8.<t> (PM decision 129): what **T3** changes
  in this window is T3 memo §8.3 item 2 (g) (its ground: T3 memo D-T7, PM decision 129). What then remains is pinned by
  C8, U2, C17's later steps and C18's later steps, and **slice b's S-5** closes it.
- **Post-bind width cap.** The Insert arm keeps HEAD's `children_iter_rev` cap (D-10, H10). That iterator starts at
  the last child, so of a parent with n > 10 000 children in a subtree inserted after bind, the **first n − 10 000**
  children (and their subtrees) are never processed in slice a (HEAD-equal). The bind
  walk is uncapped (D-11). Slice b's S-1 covers "any depth and width" at creation. Rev 8.<t>: not for a subtree a clone or
  a fragment parse created, whose creation walks are uncapped (T3 memo §8.3 item 2 (f)).
- **W-K5.** As K-5 (from N1, its DOMParser half reads null: §1.4 (ii); from C1, its template-contents half too).
  Rev 8.<t>: not for the elements of a DOMParser build or of template contents a fragment parse or a clone created,
  which T3 processes and marks at their creation (T3 memo §8.3 item 2 (e)).
- **E-G1.** As K-0.

Rev 4's "depth cap" window is deleted. The bind walk is uncapped (D-11), so no pre-bind element of the bound
document's shadow-including tree outside template contents is left for insertion to reach. That is also why deleting
processing at insertion in branch A (§1.4 (2)) leaves no such element unprocessed: U10 and U11 show that every such
element is marked at T-a1. U10 and U11 say nothing about **pre-bind template contents**, which D-11 and K-6 exclude. In
branch A they are covered by S-1's extension to pre-bind template contents, not by the bind walk. Example: an initial table with more than
10 000 `<tr onclick>` rows, re-appended by a sort script. HEAD processes each row at its own Insert. Slice a processes
each at bind and skips it at the re-append (K-1). After slice b in branch A, nothing is processed at the re-append,
and each row is already processed.

### §1.6 C1's handling (decided: branch A, PM decision 123)

The PM decides, after slice a and N1 land, whether C1 (`#11-template-contents-owner-document`) lands (branch A) or
not (branch B) (§8.1). Rev 8.4: decided, branch A (PM decision 123); C1 is this program's nested PR, after C0 and before T3 (§1.2). This umbrella decides only what each branch requires of slice b: S-2 in both branches, and S-1's
template-contents extension in branch A. Rev 4's branch-B design (the Insert arm kept as the template-contents
processing point, K-B, cell B9, and a pre-drafted slot `#11-eh-template-contents-at-creation`) is withdrawn as a
decision. It survives as input to slice b's plan (§9 N-2, N-3). If branch B is chosen and leaves a residue, slice b's
plan drafts that residue's slot text. No text exists when branch B is decided, so the PM registers the slot when slice
b's plan passes plan-review, before slice b's implementation. Until then the residue is recorded only by W-K5 (§1.5) and
by `#11-eh-attrs-creation-time-call-sites` defect (5).

## §2. Coupled invariants

### §2.1 All pairs (36), with owner

*Owner* = the slice whose plan resolves the pair; *a → b* = slice a resolves it for its scope, and slice b re-derives it
at creation time.

| pair | intersect? | owner | resolution |
|---|---|---|---|
| I1×I2 | yes | a → b | A processed element is marked, and a write on a marked element processes nothing. Cells C1, C4, U5 (iii) |
| I1×I3 | yes | a | The marker is per origin and governs ordinary pairs only. A Window-delegating pair is outside the once rule and is re-raw'd at every insertion, as at HEAD (X-W). Cells C8, C19, U2 |
| I1×I4 | yes | a → b | A clone does not copy the marker (clone-policy row). Slice b processes the clone at creation (S-1; rev 8.<t>: T3 does, PM decision 153). Cells C9, U6 |
| I1×I5 | yes | a → b | Every timing calls D-2's function. The document marker makes the bind pass run once per document across H13's rebinds (D-5). Cells E3, E8, U7, U9 |
| I1×I6 | yes | a | The marker is set when the element has any ordinary handler content attribute, excluded or not, so an excluded-only element is not rescanned. Cell U3 (i) |
| I1×I7 | yes | a, b | Slice a: a compiled callable keeps its identity when moved into a DOMParser document (C2). Slice b: S-1, S-2 |
| I1×I8 | yes | b | S-1 with T3 |
| I1×I9 | yes | a | The once check is inside D-2's function. The VM has no once logic |
| I2×I3 | yes | a | A delegated write passes the `<body>` receiver (D-6). Its pair is Window-delegating, so it processes nothing (X-W). Cells C17, C18 |
| I2×I4 | yes | a | A clone starts unmarked. A write on the source never marks the clone. Cell C9 |
| I2×I5 | yes | a → b | Slice a: D-8 (C4, C5, C11, U5). No later bind and no Insert walk reaches a shadow-root-direct element (D-5, D-10; U9, E8). Slice b: S-5 |
| I2×I6 | yes | a | An excluded name never reaches the D-4 entry from the AttributeChange arm (`:405`), and its IDL setter is not installed |
| I2×I7 | yes | b | An unrestricted write-time processing of a DOMParser node needs N1 (x4). That is why D-8 exists |
| I2×I8 | yes | a → b | The same on a DOMParser `<body>` needs T3 (x3). Slice a: no Window-delegating pair is processed at a write or skipped at an insertion (X-W; C17, C18, C19); T3 and slice b re-derive it |
| I2×I9 | yes | a | Both VM setters call the engine-independent entry (D-6) |
| I3×I4 | yes | a | A clone of a `<body>` is a new origin, unmarked; its Window-delegating pairs follow X-W. Cell C9 |
| I3×I5 | yes | a | The bind pass resolves `<body>` WindowEventHandlers rows to the Window, which exists at bind (H14); later insertions re-raw them (X-W). Cell E4 |
| I3×I6 | yes | a | An excluded WindowEventHandlers attribute (`onstorage` under `BrowserCore`) on a `<body>` writes nothing to the Window at bind or at any insertion. Cell U3 (ii) |
| I3×I7 | yes | b | S-1 |
| I3×I8 | yes | b | Slice a keeps HEAD's target: the bind pass reaches only the active bound document, where step 3 does not fire, and X-W keeps HEAD's path elsewhere. Slice b: x3 |
| I3×I9 | yes | a | The origin is chosen in engine-independent code; `native_body_weh_set` passes the `<body>` receiver (D-6) |
| I4×I5 | yes | a → b | Fragment-parsed nodes and clones: first insertion in slice a (H6), creation in slice b (S-1; rev 8.<t>: clones at T3) |
| I4×I6 | no | — | The clone's non-copy is independent of the policy, and the filter applies at processing |
| I4×I7 | yes | b | S-1, S-2; §9 N-2 |
| I4×I8 | yes | b | S-1 with T3 |
| I4×I9 | yes | a | The marker's non-copy is a row of `elidex-ecs`'s clone-policy table (`tree_clone.rs:19-34`, the "deliberate non-copy" row `:30`). `elidex-ecs` copies an explicit set, so no code changes |
| I5×I6 | yes | a | Every timing applies the same `policy.installs` filter as HEAD (`:447`). Cell U3 |
| I5×I7 | yes | a, b | Slice a's bind pass walks only the bound document's tree and skips template contents (D-5, D-11; U8). The Insert arm keeps W-K5. Slice b: S-1, S-2 |
| I5×I8 | yes | b | S-1 (step 3 must see the inactive document at creation). §9 N-1 |
| I5×I9 | yes | a → b | The bind pass runs before the dispatcher is installed, so no `MutationEvent` re-enters (`vm_api.rs:313-314`). Slice b's re-entry: its plan |
| I6×I7 | no | — | The policy filter is independent of the node document |
| I6×I8 | no | — | The policy filter runs on the name only, before target resolution |
| I6×I9 | yes | a | The VM passes `VmInner::spec_level_policy`, the value the consumer is built with (H19) |
| I7×I8 | yes | b | Both need slice b's prerequisites (x3, x4, x5) |
| I7×I9 | yes | b | Slice b's plan (§9 N-1) |
| I8×I9 | no | — | Step 3 lives in `resolve_handler_target`; no VM code is involved |

Count: 36 pairs; 32 intersect and 4 do not (I4×I6, I6×I7, I6×I8, I8×I9). Of the 32: slice a alone 14, slice b alone
9, *a → b* 7, *a, b* 2. Re-count with `awk '/^### §2.1/{f=1} /^## §3/{f=0} f && /^\| I[0-9]×I[0-9]/' <memo> | awk -F'|'
'$3 ~ /yes/ {print $4}' | sort | uniq -c`.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.8.1 Event handlers | event handler content attribute *attribute change steps* 1–5.6 | step 1 non-handler name → return; step 2 determine the target; step 3 null target → return; step 4 value null → deactivate; step 5.1 CSP inline check (not modelled: `frame-ancestors` is parsed and enforced, `crates/core/elidex-plugin/src/origin.rs:172-303`, `parse_frame_ancestors` `:202`, but no `script-src` handling exists in `crates/`, so no inline script or inline handler passes a CSP check; slot `#11-csp-inline-handler-enforcement`); step 5.5 set raw uncompiled handler; step 5.6 activate | a: D-2's function (NEW) for creation-time attributes; `handle_attribute_change` `crates/script/elidex-script-session/src/event_handler_consumer.rs:389` for writes (D-4 entry first). b: its plan | ✓ (5.1 listed, not modelled; E-G1 (iii)) | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *determining the target of an event handler* 1–4 | non-body → self; not WEH and not the reflecting set → self; step 3 inactive node document → null; else relevant global | `resolve_handler_target` `event_handler_consumer.rs:328` (unchanged; step 3 is T3, the reflecting set facet 4); a: steps 1–2 are X-W's test, on (tag, name) only | ✗ | no |
| WHATWG HTML §8.1.8.1 Event handlers | event handler IDL attribute setter 1–4 | target null → return; value null → deactivate; else set value + activate | a: `native_event_handler_set` `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs:344`, `native_body_weh_set` `:459` (after `:468-470`) → D-4 entry | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *get the current value of the event handler* steps 3.2 and 3.9 | 3.2: raw value in a null-browsing-context document → null; 3.9: function created with the document / form owner / element scope, and the `onerror` five-parameter list on a Window | `ensure_event_handler_current` `event_handler_attrs.rs:521` (unchanged; 3.2 is N1's and S-1's; 3.9 is E-G1 (ii), not modelled here) | ✗ | no |
| WHATWG HTML §8.1.8.2 Event handlers on elements, Document objects, and Window objects | WindowEventHandlers rows | WEH rows on `<body>` delegated (HEAD) | `EVENT_HANDLER_ATTRS` `event_handler_consumer.rs:61`; cells C8, E4 | ✗ | no |
| WHATWG DOM §4.9 Interface Element | *append an attribute* step 4 (*handle attribute changes*) | every attribute present at creation runs the attribute change steps once | D-2's function: the realisation of those per-attribute runs | ✓ | yes |
| WHATWG HTML §13.2.6.1 Creating and inserting nodes | *create an element for a token* step 21, "For each attribute of attributes, append attribute to element" (rev 8.<t>, PM decision 711: `webref body html creating-and-inserting-nodes`, 2026-11-13; until rev 8.<t>: "step 17.2, "Append attribute to element"") | parser path: every token attribute is appended at creation | parser bakes `Attributes` at spawn (`crates/dom/elidex-html-parser/src/convert.rs:258`; strict `crates/dom/elidex-html-parser-strict/src/tree_builder/insert.rs:61`, `:119`, `tree_builder/shadow.rs:162`); a: first bind (pre-bind) or first insertion; b: S-1 | ✓ | yes |
| WHATWG HTML §13.2.6.4.4 The "in head" insertion mode | start tag "template", step 14 "Switch on sanitizerAction", case "Keep", sub-step 1 (declarative shadow root) (rev 8.<t>: `webref body html parsing-main-inhead`, 2026-11-13; until rev 8.<t>: "step 13 "Keep"", step 13 being "If the parser's parser sanitizer configuration is not null …"; T3 memo rev 5, R1 M13) | the shadow root is attached while parsing; its children are parsed into it | strict `tree_builder/shadow.rs:137`, `:162`; a: pre-bind DSD by the bind pass (E7); fragment DSD not processed at host insertion (D-10, HEAD-equal); b: S-1 | ✗ | yes |
| WHATWG HTML §13.2.6.4.7 The "in body" insertion mode | start tag "html" step 2.2, start tag "body" step 3.2 | an attribute not already present is added to the existing element | no elidex-side writer (§5.0); not a creation-time attribute | ✗ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | *HTML fragment parsing algorithm* steps 1–2 (step 1: the parser's own new `Document`; step 2: the fragment, in target's node document) | fragment elements are created in the context's node document (through §13.2.6.1 *create an element for a token* step 6, after the redirection out of the parser's root `html` element), not in step 1's Document (rev 8.<t>, T3 memo P-T17; until rev 8.<t>: "step 4, the fragment's new `Document` \| fragment elements are created in a throwaway document") | `begin_detached_fragment` (`crates/core/elidex-ecs/src/dom/tree/teardown.rs`, unchanged in a); creation-time processing: T3 (PM decisions 699, 705; T3 memo D-T12, §4.5) | ✗ | yes |
| WHATWG DOM §4.4 Interface Node | *clone a single node* step 2.5; *clone a node* steps 5 and 6.7 | a clone's attributes are appended at its own creation; children and shadow children are cloned the same way | a: clone-policy table `crates/core/elidex-ecs/src/dom/tree_clone.rs:19-34` (row added: marker non-copy); b: S-1 | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | *template contents*; *appropriate template contents owner document*; the template *cloning steps* step 2 | contents are not descendants; their owner document has a null browsing context; a template clone's contents are cloned into the copy's contents | a: not walked by the bind pass (D-11); the Insert arm keeps W-K5 (H22); b: S-2 | ✗ | yes |
| WHATWG DOM §4.5 Interface Document | *adopt* | node document changes; no event-handler step | `EcsDom::adopt_subtree` `teardown.rs:285` (unchanged; the marker survives) | ✓ | no |
| WHATWG DOM §4.2.3 Mutation algorithms | *insert*, *insertion steps* | HTML defines no event-handler insertion steps | a: `handle_insert` `event_handler_consumer.rs:427`, once over HEAD's walk set (D-10); b: S-1 | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *shadow-including descendant* | walks include shadow trees | a: the bind pass only, uncapped (D-11) | ✓ | yes |
| WHATWG HTML §8.5.1 The DOMParser interface | `parseFromString` | `text/html` branch only | `parse_into_inert_document` `crates/dom/elidex-form/src/inert_document.rs:110` (unchanged in a; b: S-1) | ✗ | yes |
| WHATWG HTML §8.5.4 The innerHTML property | setter | fragment parse + replace children | placement fires Insert (H6; a unchanged; b: S-1) | ✗ | yes |
| WHATWG HTML §8.5.5 The outerHTML property | setter | fragment parse + replace self | as innerHTML | ✗ | yes |
| WHATWG HTML §8.5.2 HTML parsing and insertion methods | `setHTMLUnsafe` | fragment parse with declarative shadow roots allowed | as innerHTML; its DSD content is the §13.2.6.4.4 row | ✗ | yes |

### §3.1 User-input touch audit

The rows marked *yes* carry author-controlled markup or script values. The mechanism stores no user string: the
markers are unit components, and the handler sources it writes are HEAD's. Slice a's bind pass newly makes initial-HTML
handler sources reachable by the compiler, which is the spec's behaviour except step 5.1 (E-G1 (iii): no CSP check, as for
every inline handler at HEAD). They compile only through HEAD's compile gate, with HEAD's global-only scope (H25;
E-G1 (ii)). No row adds a parse of user input.

## §4. Home/kind (decided) and slice a's mechanism (input)

**Home/kind (decided).** The once-key is a unit marker **component** on the origin element. The bind-pass key is a unit
marker **component** on the bound Document entity. Both are homed in the crate `elidex-script-session`. Their file placement is not decided here: slice a's per-PR
plan §5 (`docs/plans/2026-10-eh-attrs-slice-a.md`) is canonical (PM decision 39). Each is `Send + Sync`,
per-entity, and not a per-VM identity handle, so by CLAUDE.md's side-store→component rule each is a component, not
consumer state (the consumer is rebuilt at every bind, H13). Despawn removes them, adopt keeps the element marker, and
clone does not copy it.

**Slice a's mechanism (input; slice a's plan re-derives it).**
- Items: `HandlerAttrsProcessed`, `PreBindHandlerAttrsProcessed`, a per-element processing function, a subtree form
  over D-11's walk, a write-side entry (`note_handler_write` in rev 4), and an `initialize_from_tree` on the consumer,
  which `ConsumerDispatcher::initialize_consumers` calls. The bound document must reach it (H13a). Today
  `initialize_consumers(&mut self, dom)` takes no document (`consumer_dispatcher.rs:135`).
- The D-8 restriction was rev 4's three-line shape: return if marked; return if E carries a deliverable attribute other
  than `on<event_type>`; otherwise process.
- Rewrite sites: `handle_insert` (`event_handler_consumer.rs:427-463`), `handle_attribute_change` (`:389-421`), the two
  VM setters (§3), `initialize_consumers` and its doc (`consumer_dispatcher.rs:113-137`), the clone-policy rows
  (`tree_clone.rs:30` for both markers, deliberate non-copy), the
  re-exports (`elidex-script-session` `lib.rs`), the G6 docstring (`attribute_reconcile.rs:21-23`), and the
  `Vm::bind` comment at `vm_api.rs:298-312` ("other consumers … are no-ops here; see
  `ConsumerDispatcher::initialize_consumers`"), which goes stale when the consumer gains an init pass.
- Clone policy of the Document marker: `PreBindHandlerAttrsProcessed` is a **deliberate non-copy** (`tree_clone.rs:30`;
  the table starts at `:20`). Documents are cloneable (`clone_subtree`'s Document branch, `tree_clone.rs:163-230`),
  and a cloned Document has had no first bind, so it must start unmarked. `DocumentBaseUrl`'s "out-of-domain" row
  (`:31`) is not this row's precedent.
- Two init keys in one hook (input; no decision): D-5 keys the bind pass on the bound document, while
  the base-URL maintainer's `initialize_from_tree` (`crates/dom/elidex-dom-api/src/element/document_base.rs:276-285`) keys on
  `document_root()`, in the same `initialize_consumers` call. They are equal in production, so no harm is shown. Slice
  a's plan either passes the bound document to both or states why the two keys stay.
- Docstring citations (Axis 4): every new item cites WHATWG HTML §8.1.8.1 Event handlers and WHATWG DOM §4.9 Interface
  Element (*append an attribute* step 4). The bind items cite WHATWG HTML §13.2.6.1 Creating and inserting nodes. The
  walk cites WHATWG DOM §4.8 Interface ShadowRoot and WHATWG HTML §4.12.3 The template element. `handle_insert` cites
  WHATWG DOM §4.2.3 Mutation algorithms and names `#11-eh-attrs-creation-time-call-sites`. The clone row cites WHATWG
  DOM §4.4 Interface Node. The module docs (`event_handler_consumer.rs:1-33`) and **the `ConsumerDispatcher`
  `event_handler_attrs` field doc (`consumer_dispatcher.rs:58-66`, which says "Dual-arm (AttributeChange + Insert)")**
  are rewritten to the processing rule (D-3), the interim Insert arm and the bind arm.

**Alternatives rejected (kept as decisions).** R-D3, rev 1's `Pending { superseded }` component, enlarges the interim.
R1, a new `MutationEvent` at every spawn, is rejected for slice a; whether slice b uses an event is O9. R2, keying on
the target's entry: the Window's entries are shared. R3, a connectedness filter, is not the spec's condition. R4, a
positive "unprocessed" marker at spawn: a forgotten site leaves dead handlers. R5, comparing source text: no spec basis.
R6, a session or `HostData` set, violates the side-store→component rule. R7, unrestricted write-time processing in
slice a, needs N1 and T3. R9, a dense marker, costs one archetype move per element for no information. R11, a
shadow-including Insert walk, violates K-0 (D-10). R12, marking at a placement under a shadow root: no hook (H2). R13
(rev 5), a capped bind walk: it violates K-6 and leaves branch A a hole (D-11).

**ECS-native and Layering.** OO "element.attributesProcessed flag" → a marker component on the entity. "document.initialized"
→ a marker component on the Document entity. "process the document at attach" → a system over the bound tree, run at
the dispatcher's install point and gated by the document marker. "writers notify the element" → writers calling one
engine-independent function. There is no registry, no new dispatcher variant and no side-store. `vm/host/` adds two
calls into `elidex_script_session` (marshalling). Inside `dispatch` the consumer writes only `EventListeners` and the
element marker. The document marker is written before install.

### §4.1 The two processing functions: their final shape, decided once (rev 8.<t>; PM decisions 741 (3), 756, 767; decided, not input)

Two functions are edited by both T3-a and T3 (T3-b edits neither; until PM decision 767 T3-a and T3-b were one PR, T3-ab): `process_handler_content_attributes`
(`crates/script/elidex-script-session/src/event_handler_processing/mod.rs:76` at `cee74989`) and
`handle_attribute_change` (`crates/script/elidex-script-session/src/event_handler_consumer.rs:397`). Measured
(2026-11-15): T3-a's sites are the three calls of `is_event_handler_content_attribute_of` (the T3-a memo §4.2:
`mod.rs:97`, `mod.rs:206`, `event_handler_consumer.rs:405`); T3-b's are the `body` test and its two readers in `vm/host/` (the T3-b memo §4.2); T3's are the list
of T3 memo §4.2. The intersection of T3-a's and T3's is these two functions; T3-b's meets neither. `is_only_deliverable_ordinary_name` (`mod.rs:196`), the
`body` arm of `tag_specific_t2b_prototype` and `require_body_receiver` are edited by T3-a (the first) and T3-b (the other two) only; every other function
of T3 memo §4.2 by T3 only.

This is their shape when both have landed. The tag on each part names the PR that writes it; an untagged part is
slice a's, on main.

```rust
pub(crate) fn process_handler_content_attributes(
    dom: &mut EcsDom, element: Entity, policy: SpecLevelPolicy, timing: ProcessingTiming,
    active: ActiveDocument,                                                 // [T3] the signature's one change
) {
    // 1. the marked shortcut; its clause `timing != ProcessingTiming::Creation`            [T3]
    // 2. the snapshot: the element's attributes that are its event handler content attributes
    //        .filter(|(name, _)| is_event_handler_content_attribute_of(dom, element, name)) [T3-a]
    //        .filter_map(/* event_handler_attr_lookup: event type, value */)
    // 3. per snapshot entry, in this order: window_delegating; the mark's count (carries_ordinary);
    //    the policy filter; `deliver = match (window_delegating, timing)`, whose arms
    //    `(_, Creation) => true` and `(true, Insertion) => is_x_w_pair(…)` are                [T3]
    // 4. per delivered entry: `determine_event_handler_target(dom, element, &name, active)`,
    //    `None` → skip; it replaces `resolve_handler_target`                                 [T3]
    // 5. the mark
}

fn handle_attribute_change(
    policy: SpecLevelPolicy, node: Entity, name: &str, old_value: Option<&str>, new_value: Option<&str>,
    dom: &mut EcsDom, active: ActiveDocument,                               // [T3] the signature's one change
) {
    // 1. `event_handler_attr_lookup(name)`; an unknown name → return
    // 2. `!is_event_handler_content_attribute_of(dom, node, name)` → return                 [T3-a]
    // 3. the policy filter → return
    // 4. `determine_event_handler_target(dom, node, name, active)`; `None` → return         [T3]
    //    (it replaces `resolve_handler_target` and precedes step 5, the order T3 memo D-T6 gives the IDL setter)
    // 5. `process_before_handler_write` (slice a's D-8 entry; it passes `active` on)          [T3: the argument]
    // 6. set, or clear on a genuine removal
}
```

**What is fixed here.** (i) T3-a writes its parts, step 2 of each, at these positions and in this form, on the
signatures main has; it changes no signature and adds no branch to the processing function's loop (its part is a
filter of the snapshot). (ii) T3 adds its tagged parts and does not move, re-order or rewrite T3-a's. (iii) Each
signature changes once, at T3: the `active` parameter, whose type `ActiveDocument`, constructor and two marshalling
points are T3's (T3 memo D-T3), so it cannot be written earlier without them. (iv) The predicate's own signature,
`is_event_handler_content_attribute_of(dom, element, name) -> bool`, is final at T3-a (`pub(crate)`); facet (2)'s closer (the T3-a memo §8.3 item 1) extends
its callers with the attribute's namespace, not T3. **Stop condition (PM decision 741 (3)):** if T3's implementation
finds that it must change this shape (move or rewrite a T3-a part, or change the predicate), the slice is mis-drawn
and T3 stops for the PM. **(iii) meets the PM's condition (answered, PM decision 756):** PM decision 741 (3) asks that the final shape be decided once, here, not that T3-a write the final
signature; R3 wave 1's Axis 3 confirmed that this section states T3's added `active` parameter as the final shape (the T3-a memo §10 Q-A1; until the T3-ab memo rev 1.1 the question was put to the PM). The condition holds for T3-a and T3-b alike (PM decision 767): neither memo, nor the T3 memo, restates this shape.

## §5. Populations and named risks (input)

### §5.0 `Attributes` writers: silent vs firing (census, reproducible)

A **silent writer** is a production code path that writes an element's `Attributes` without firing
`MutationEvent::AttributeChange` for that write. The population is every hit of the patterns below in `crates/`. A hit
`path:line` is **test** if `path` matches the ERE `/tests/|/tests\.rs$|/tests_[^/]*\.rs$|_tests\.rs$|/benches/|test_helpers|test_support`,
or if the file contains `#[cfg(test)]` and `line` is after its first occurrence. Otherwise it is **prod**.

```sh
c() { /usr/bin/grep -rnE --include='*.rs' "$1" crates | while IFS=: read -r f l _; do
  if echo "$f" | /usr/bin/grep -qE '/tests/|/tests\.rs$|/tests_[^/]*\.rs$|_tests\.rs$|/benches/|test_helpers|test_support'
  then echo "T $f:$l"; continue; fi
  t=$(/usr/bin/grep -n -m1 '#\[cfg(test)\]' "$f" | cut -d: -f1)
  if [ -n "$t" ] && [ "$l" -gt "$t" ]; then echo "T $f:$l"; else echo "P $f:$l"; fi; done; }
c '<pattern>' | /usr/bin/grep -c '^P'    # and '^T'; `| /usr/bin/grep '^P'` lists the prod sites
```

Run at `fd6955e8` on 2026-10-25:

| pattern (ERE) | total | test | prod | production sites → class |
|---|---|---|---|---|
| `get::<&mut (elidex_ecs::)?Attributes>` | 21 | 17 | 4 | `attribute.rs:215` (`write_attribute_no_dispatch`), `:305` (`remove_attribute`: **fires** when present), `tree_clone.rs:68` (`clone_attributes`: **silent**, no prod caller), `content/iframe/lifecycle.rs:358` (iframe `src` rewrite: **silent**, `src` only) |
| `insert_one\([^)]*(attrs\|Attributes\|attributes)` | 7 | 5 | 2 | `attribute.rs:224`, `tree_clone.rs:72` |
| `builder\.add\(attrs\)` | 1 | 0 | 1 | `tree_clone.rs:368` (the cloneNode path: **silent**, all attributes) |
| `\.create_element(_with_owner\|_ns)?\(` | 2210 | 2187 | 23 | 17 pass `Attributes::default()`; 2 are shims (`dom/mod.rs:335`, `:380`); **4 bake parser attributes (silent)**: `elidex-html-parser/src/convert.rs:258`, strict `tree_builder/insert.rs:61`, `:119`, `tree_builder/shadow.rs:162` |
| `\.set_attribute_without_dispatch\(` | 1 | 0 | 1 | `crates/dom/elidex-form/src/value_mode.rs:64`: **silent**, `value` only |
| `\.clone_attributes\(` | 1 | 1 | 0 | test only |
| `\.set_attribute\(` | 141 | 134 | 7 | all **fire** through the `attribute.rs:125` chokepoint |
| `\.remove_attribute\(` | 24 | 18 | 6 | 5 **fire** when present; 1 is a comment |

Parser attribute merging (WHATWG HTML §13.2.6.4.7 The "in body" insertion mode, `html` step 2.2 and `body` step 3.2) has
no elidex-side writer.

Result:
- (a) The silent writers that can write an `on*` attribute are exactly the creation-time bakes: the four parser sites
  and `clone_node_shallow_unchecked` (plus the prod-unreachable `clone_attributes`). They define "creation-time
  attributes" (§0.5).
- (b) The other silent writers write fixed non-handler names (`value`, `src`).
- (c) Every post-creation `on*` write fires AttributeChange while a dispatcher is installed. While **unbound**, a firing
  write is observed by no consumer. That is G7 (§8.1).

### §5.1 Processing point per creation path

| path | slice a | slice b |
|---|---|---|
| main-document parse, URL-loaded iframe, srcdoc / about:blank iframe (pre-bind; H16/H17) | **first bind** (G1) | same (K-6) |
| fragment (innerHTML / outerHTML / setHTMLUnsafe) | first insertion (placement, H6); none when placed directly under a shadow root (HEAD) | S-1 (rev 8.<t>: from T3, PM decisions 699, 705, 711; T3 memo D-T12; the declarative shadow content and template contents it creates included; until T3 memo rev 4.8: "stay S-1 / S-2") |
| DOMParser | first insertion after return (W-K5 when into a DOMParser document) | S-1 (rev 8.<t>: from T3, PM decisions 711, 713; the build's event is queued in its window and dispatched at the close, the T3-pre memo §4.4; T3 memo D-T13) |
| `createElement` + `setAttribute` | the write processes and marks (D-8); a Window-delegating pair is not processed at the write (X-W) | same (S-5) |
| `cloneNode` | first insertion | S-1 (rev 8.<t>: from T3, PM decision 153; T3 memo D-T10) |
| declarative shadow (parser) | first bind (pre-bind parse); not processed in a fragment parse (D-10) | S-1 (pre-bind parse); in a fragment parse from T3 (rev 8.<t>, PM decision 711; T3 memo D-T12) |
| template contents | placement into contents (W-K5), or when moved out (HEAD) | S-2; created by a fragment parse (DOMParser's build included): from T3 (rev 8.<t>, PM decision 711; T3 memo D-T12) |

### §5.2 Named risks (slice a)

- **K1, an unknown creation path.** In slice a, an element baked by such a path is processed at its first insertion,
  as at HEAD. For slice b this is S-4.
- **K4, walk cost.** The Insert arm keeps HEAD's walk and adds one marker read per element. The bind pass walks a
  document's tree once, at its first bind (D-5). The base-URL maintainer already walks the tree on every bind.
- **K5, K6.** E-G1 (i) and (ii); and E-G1 (iii) for the CSP exposure (§0.5; §8.1).
- **K7 (rev 5), the uncapped walk on a malformed tree.** A sibling or parent cycle must terminate. The H10a precedent
  uses a visited set, and slice a's plan states its guard.

## §6. Test and coverage plan (slice a: decided; slice b: §9)

*Red* = must fail on slice a's pre-fix base (its branch base) and pass after. The implementer runs each red cell against
that base first and records the failure in the PR description (red-first). *Guard* = passes at both, asserting
HEAD-equality. *Window pin* = passes at both, asserting HEAD's outcome on a shape where it is **not** the spec's. It names
what inverts it and is never cited as the correct answer. The "HEAD" predictions come from H1 / H3 / H9 / H21; they are
not measurements. The cells and their kinds are decided. File placement and helper reuse are input.

### §6.1 VM integration and unit cells

VM cells drive the VM through `dispatchEvent(new Event('click'))` (the VM has no `click()`), with `el` connected.

| cell | kind | shape | HEAD | oracle |
|---|---|---|---|---|
| C1 | red | `el.innerHTML = '<div id=t onclick="A">'`; `t.onclick = fn`; `el.append(t)` (move) | move re-raws A | `t.onclick === fn`; dispatch runs `fn` |
| C2 | red | `t = createElement('div')`; `el.append(t)`; `t.setAttribute('onclick', S)`; dispatch (compiles); `f = t.onclick`; `doc2.body.appendChild(t)` (§L L1) | move re-raws S | store: `click` entry `uncompiled_source == None`; `t.onclick === f` |
| C3 | red | baked A via innerHTML; `f = t.onclick`; move twice, live→live | each move recompiles | `t.onclick === f` after each |
| C4 | red | `c = createElement('div')` (never inserted); `c.setAttribute('onclick', A)`; dispatch on `c`; `f = c.onclick`; `doc2.body.append(c)` (§L L5) | first insert re-raws A | store `uncompiled_source == None`; `c.onclick === f` |
| C5 | red | DOMParser `<div id=d onclick="A">`; `d.onclick = fn`; variants (i) `dp2.body.append(d)`, (ii) `el.append(d)`, (iii) `c = d.cloneNode()`, `c.onclick = fn`, `el.append(c)` (§L L6) | first insert raws A over `fn` | `d.onclick === fn` (`c.onclick === fn`); dispatch runs `fn` in (ii), (iii) |
| C6 | guard | DOMParser `<div id=d onclick="A">` appended into `el` | delivered at insert | `typeof d.onclick === 'function'`; dispatch runs A |
| C7 | guard | delivered `t`, `f = t.onclick`; `t.setAttribute('onclick', B)` | — | `t.onclick !== f`; dispatch runs B |
| C8 | window pin (X-W; rev 7, was red) | `b = createElement('body')`; `el.append(b)`; `b.setAttribute('onhashchange', X)`; `window.onhashchange = fn`; move `b` | move re-raws X onto the Window | `typeof window.onhashchange === 'function'` and `!== fn` (HEAD's). **Spec** (not asserted): `fn`; slice b (S-5) inverts it. It discriminates X-W's test from a document-keyed one: `b` is in the live, bound document, so a test that exempts that document fails C8 |
| C9 | guard | `t` delivered and IDL-set to `fn`; `c = t.cloneNode()`; insert `c` | — | before insert: `c` unmarked; after: `c.onclick` is compiled A (not `fn`) |
| C11 | red | `c = createElement('div')`; `c.setAttribute('onclick', A)`; `c.onclick = fn`; `el.append(c)` | first insert raws A over `fn` | `c.onclick === fn` |
| C12 | guard | clone of `onclick=A`; `c.removeAttribute('onclick')`; insert | — | `c.onclick === null` |
| C13 | guard | parser tiers: `el.innerHTML = '<button onclick=…>'` (strict) and a foster-parenting `<table>` string (tolerant fallback) | delivered | dispatch runs the body on both |
| C14 | window pin (W-K5, template) | `tpl.innerHTML = '<b id=x onclick="window.hit=1">'`; `tpl.content.getElementById('x').dispatchEvent(new Event('click'))` | runs | `window.hit === 1` (spec: does not run; C1 inverts it, with N1's compile gate and C0's adoption at the placement) |
| C15 | window pin (W-K5, DOMParser) | `x` from `dp = parseFromString('<div id=x onclick="A">')` (unmarked, no entry); `x.onclick` (null); `dp2.body.append(x)`; getter | before `null`; after a function | after the append `typeof x.onclick === 'function'` (spec: null; N1's node clause inverts it) |
| C16 | guard (T-a3, restriction returns) | DOMParser `<div id=d onclick="A" onmouseover="C">`; `d.setAttribute('onclick', 'B')`; `d.onmouseover` | `null` (no `mouseover` entry, H5) | `d.onmouseover === null` (an unrestricted write would raw `C`, and the flag-only gate would compile it) |
| C17 | guard (X-W at T-a3; window pin; R3 IMP) | `dp = new DOMParser().parseFromString('<body onhashchange="window.hit=1">', 'text/html')`; `b = dp.body` (its only deliverable attribute is `onhashchange`); `window.onhashchange` is null; step 1: `b.onhashchange = fn`; step 2: `el.append(b)`; step 3: dispatch `hashchange` on the Window | step 1: `window.onhashchange === fn` (HEAD's setter has no step 3). Step 2: the Insert re-raws `hit=1` over `fn`; the getter compiles it. Step 3: runs `hit=1`, not `fn` | step 1 and step 2 as HEAD, step 3 as HEAD: `window.onhashchange === fn`, then `typeof window.onhashchange === 'function'` and `!== fn`, then `window.hit === 1` with `fn` not run. **Spec** (not asserted): step 1 the setter returns (null target), step 2 `window.onhashchange === null`, step 3 runs nothing; reached after T3 and slice b (§1.5 X-W). Rev 5 gave `window.onhashchange === fn` at step 2, which matches neither. A write-side entry that processes a Window-delegating pair fails it. K-2 and K-3 do not claim this pair (X-W), so the cell and their qualifiers agree |
| C18 | guard (X-W at T-a3; window pin; rev-6 re-check A2 IMP) | `b = createElement('body')`; `tpl.content.append(b)`; `b.setAttribute('onhashchange', 'window.hit=1')`; `b.onhashchange = fn`; `document.body.append(b)`; dispatch `hashchange` on the Window | the append into the contents fires Insert while `b` has no attribute (H22). The AttributeChange arm resolves to the Window (`event_handler_consumer.rs:389-421`, `:328-334`): raw `hit=1`. The setter writes `fn` to the Window (`event_handler_attrs.rs:459-479`). The append into the body re-raws `hit=1` over `fn` (H1); the getter compiles it; dispatch runs `hit=1` | as HEAD: after the setter `window.onhashchange === fn`; after the append `typeof window.onhashchange === 'function'` and `!== fn`; `window.hit === 1`, `fn` not run. **Spec** (not asserted): the contents' node document is inert, so step 3 makes both writes return and `window.onhashchange === null`; reached after T3 and slice b. It fails under rev 6's (b), which let `b` pass because elidex gives template contents the bound document (the C1 gap) |
| C19 | guard (X-W at T-a2) | `b = createElement('body')`; `b.setAttribute('onclick', 'A')` (T-a3 passes and marks `b`); `b.setAttribute('onhashchange', 'window.hit=1')`; `dp.body.append(b)`; `b.onhashchange = fn`; `el.append(b)`; dispatch `hashchange` on the Window | the AttributeChange arm writes raw `hit=1` to the Window; each append re-raws both attributes; the setter writes `fn` to the Window; the second append re-raws `hit=1` over `fn` | `typeof window.onhashchange === 'function'` and `!== fn`; `window.hit === 1`. HEAD and the spec agree (spec: while `b` is in `dp` the setter's target is null, so the Window keeps `hit=1`). A skip at insertion that covers Window-delegating pairs gives `=== fn`, which is neither |
| C19b | window pin (X-W at T-a2, reflecting set; rev 8, rev-7 re-check A3 IMP-2) | C19's shape with `onfocus` (a Window-reflecting name) on `b = createElement('body')`: `b.setAttribute('onclick', 'A')` (marks `b`); `b.setAttribute('onfocus', 'window.hit=1')`; `dp.body.append(b)`; `b.onfocus = fn`; `el.append(b)`; `b.dispatchEvent(new Event('focus'))` | `onfocus` is `HandlerScope::Global` (`event_handler_consumer.rs:91`), so `resolve_handler_target` (`:328-334`) keeps `b`: the AttributeChange arm writes raw `hit=1` on `b`; each append re-raws both attributes on `b` (`handle_insert`, `:427`, `:462`); the setter is `HTMLElement.prototype`'s (`html_element_proto.rs:134-139`; the `<body>` prototype delegates `HandlerScope::Window` rows only, `event_handler_attrs.rs:207-213`), `native_event_handler_set` (`event_handler_attrs.rs:344`), which writes `fn` on `b`; the second append re-raws `hit=1` over `fn`; dispatch at `b` runs `hit=1` | as HEAD: `typeof b.onfocus === 'function'` and `!== fn`; `window.hit === 1`, `fn` not run. **Spec** (not asserted): the target is the Window (the reflecting set, facet 4), and the setter returns while `b` is in `dp`; reached after T3 and slice b (rev 8.<t>: reached at T3, PM decision 129; the T3 memo turns this cell into a spec guard). A skip at insertion that classifies by HEAD's predicate (`scope == Window && is_body_element`) treats `onfocus` as ordinary, skips it, and gives `b.onfocus === fn`, which is neither |
| C19c | window pin (X-W at T-a2, `<frameset>`; rev 8, rev-7 re-check A3 IMP-2) | C19b with `f = createElement('frameset')` in place of `b` | `is_body_element` tests `body` only (`event_handler_consumer.rs:337-341`) and `onfocus` is `HandlerScope::Global`, so every write lands on `f`; `f` gets no delegating accessors (they are installed on the `<body>` prototype only, `event_handler_attrs.rs:207-213`, chosen by tag at `elements.rs:264-266`), so `f.onfocus = fn` is `HTMLElement.prototype`'s setter (`html_element_proto.rs:134-139`) and writes `fn` on `f`; the second append re-raws `hit=1` over `fn`; dispatch at `f` runs `hit=1` | as HEAD: `typeof f.onfocus === 'function'` and `!== fn`; `window.hit === 1`, `fn` not run. **Spec** (not asserted): the target is the Window (step 1's `frameset`, facet 3, with the reflecting set, facet 4); reached after T3 and slice b (rev 8.<t>: reached at T3, PM decision 129). A skip at insertion keyed on a `body` tag treats `f`'s pair as ordinary and gives `f.onfocus === fn`, which is neither |
| C20 | guard (X-W at T-a3, mixed element; rev 8, rev-7 re-check A2 IMP-1) | (i) `b = createElement('body')`; `b.setAttribute('onhashchange', 'X')`; `window.onhashchange = fn`; `b.setAttribute('onclick', 'A')` (T-a3 passes: `onclick` is `b`'s only deliverable ordinary attribute). (ii) `dp = parseFromString('<body onclick="A" onhashchange="window.hit=1">')`; `dp.body.setAttribute('onclick', 'B')`; `window.onhashchange` | (i) the AttributeChange arm writes raw `X` to the Window; the setter writes `fn` there; the `onclick` write touches `b` only: `window.onhashchange === fn`. (ii) the DOMParser build ran no consumer (H5), and the `onclick` write touches `dp.body` only: `window.onhashchange === null` | (i) `window.onhashchange === fn`; (ii) `window.onhashchange === null`. HEAD and the spec agree on both. A D-2 function that processes the Window-delegating pair at T-a3 re-raws `X` over `fn` in (i) and raws `hit=1` onto the page Window in (ii) |

Unit cells drive `EcsDom` with `EventHandlerAttributeConsumer` installed, through real `append_child` /
`set_attribute` calls. A cell that calls `initialize_from_tree` more than once uses a **fresh consumer instance per
call**, as production does (H13).

| cell | kind | shape | oracle |
|---|---|---|---|
| U1 | red | element spawned with `onclick=A`, appended; `take_uncompiled`; appended elsewhere | `uncompiled_source == None` (HEAD `Some("A")`) |
| U2 | window pin (X-W; rev 7, was red) | `<body onhashchange=X>` with a Window; append; `take_uncompiled` on the Window entry; move | the Window entry is raw `X` again (HEAD's re-raw). Spec: `None`; slice b inverts it |
| U3 | guard | `BrowserCore`: (i) `<div onstorage=…>` (an ordinary pair) appended twice; (ii) `<body onstorage=…>` appended twice | (i) no `storage` entry; the div is marked after the first append. (ii) no `storage` entry on the Window after either append |
| U4 | guard | element with no handler attributes appended | unmarked |
| U5 | red (i) / guard (ii), (iii) | (i) unmarked element with only `onclick=A`: `set_attribute(onclick, B)`; then append. (ii) unmarked, never-inserted element with `onclick=A onmouseover=C`: `set_attribute(onclick, B)`, then append. (iii) the write-side entry on a marked element | (i) marked; raw `B` (HEAD's state after the write); after `take_uncompiled` and the append, still `None`. (ii) after the append: `click` raw `B`, `mouseover` raw `C` (HEAD's, and the spec's for this shape). (iii) no change |
| U6 | guard | `clone_node_shallow` of a marked element | clone unmarked |
| U7 | red (G1) | tree built **without** a dispatcher (pre-bind shape) containing `<button onclick=A>`; `initialize_from_tree` with the bound document, twice (fresh consumer each time), `take_uncompiled` between | after the first call: entry raw A, button marked, document marked; after the second: entry still `None` |
| U8 | guard | `initialize_from_tree` with the bound document, while there also exist (i) a second document (`create_document_node`) holding `<div onclick>`, (ii) a `<template>` in the bound document whose contents hold `<div onclick>`, (iii) a shadow host with `<button onclick>` in its shadow root | (i), (ii) no entry and unmarked; (iii) delivered |
| U9 | guard (K-0; R1 A2 IMP-1) | after a first `initialize_from_tree`: a connected host `h` with shadow root `sr`; `x` spawned with `onclick=A onmouseover=C` and appended directly to `sr` (no Insert, H2); the write-side entry for `click` on `x` (the restriction returns); a `click` entry with no source on `x` (an IDL write's store state); `h` moved; `initialize_from_tree` again with a **fresh** consumer | `x` unmarked; the `click` entry still has no source (HEAD's outcome, and the spec's for that write) |
| U10 | red (K-6, D-11 depth; R2 IMP 1) | pre-bind shape: a chain of more than `MAX_ANCESTOR_DEPTH` nested elements under the bound document's body, built bottom-up (the `destroy.rs:381` precedent), the deepest carrying `onclick=A`; `initialize_from_tree` | the deepest element is marked and its `click` entry is raw A. A depth-capped walk leaves it unmarked. A recursive walk overflows the stack (the precedent comment at `dom/tests/tree_ops.rs:259-266` states that recursion overflows at about 2–3k frames) |
| U11 | red (K-6, D-11 width; R2 IMP 1) | pre-bind shape: one parent with more than `MAX_ANCESTOR_DEPTH` children, each carrying `onclick=A` (the `destroy.rs:419` precedent); `initialize_from_tree` | every child is marked (count = the number built), including the first and the last. A forward-capped walk misses the last children, and a reverse-capped walk (`children_iter_rev`) misses the first |
| U12 | red (K-6, D-5; R3 A2 MIN) | pre-bind shape (built without a dispatcher, as U7), in a World where the bound document `d2` is **not** `document_root()`: `d2 = create_document_node()` (it leaves `document_root()` unchanged), with a `<button onclick=A>` in `d2`'s tree; a second `<button onclick>` in the `document_root()` tree; `initialize_from_tree` with `d2` | `d2`'s button is delivered (raw A) and `d2` is marked. The `document_root()` tree's button is unmarked and has no entry. A `document_root()`-keyed pass fails both. U8 (i) alone cannot tell the keys apart whenever the bound document is the root |

### §6.2 End-to-end pins (G1 and the rebind invariant), by path

The pins run through the shell pipeline builders, so every bracket builds a fresh dispatcher and consumer (H13).

**Red-first rule (decided).** Each red pin is committed and run on slice a's pre-fix base before the fix commit, and
its failure output is recorded in the PR description. A pin counts as red only if its **non-vacuity control** (a value
the script itself writes) is green on that base. Unit cells (U7, U9) do not substitute for any pin.

Shared markup `M`: `<body><button id=b onclick="document.body.dataset.hit=(+document.body.dataset.hit||0)+1"></button><script>document.body.dataset.ran=1;document.getElementById('b').dispatchEvent(new MouseEvent('click'))</script></body>`.
Control: `data-ran == "1"`.

| path | pin | kind | builder | shape | base (pre-fix) | oracle |
|---|---|---|---|---|---|---|
| main document | E1 | red | `build_pipeline_from_loaded` (`pipeline.rs:839`) | `M` | `data-ran == "1"`, no `data-hit` | `data-hit == "1"` after the build |
| main document | E2 | red | same | then `PipelineResult::dispatch_event` (`lib.rs:293`) with a click at `b` | `data-hit` absent | `data-hit == "2"` |
| main document | E3 | red | same | `eval_script("window.f=document.getElementById('b').onclick; typeof window.f")`; `dispatch_event`; `eval_script("document.getElementById('b').onclick===window.f")` | first read `"object"` | `"function"`, then `true` (K-1 across rebinds) |
| main document | E4 | red | same | `<body onhashchange="document.body.dataset.h=1"><script>document.body.dataset.ran=1;window.dispatchEvent(new Event('hashchange'))</script>` | no `data-h` | `data-h == "1"` (delivered to the Window at bind) |
| main document | E7 | red | same | `<!DOCTYPE html><div id=h><template shadowrootmode=open><button id=b onclick="…as M…"></button></template></div><script>document.body.dataset.ran=1;document.getElementById('h').shadowRoot.querySelector('#b').dispatchEvent(new MouseEvent('click'))</script>` | no `data-hit` | `data-hit == "1"` (the bind pass walks shadow trees) |
| main document | **E8** (rev 5; R2 A2 MIN, A3 MIN) | guard (K-0 T-a1 later binds, T-a2 shadow) | same | bracket 1 (initial script): `<div id=src onclick="document.body.dataset.attr=1" onmouseover="0"></div><div id=h></div><script>document.body.dataset.ran=1;const sr=h.attachShadow({mode:'open'});window.x=src.cloneNode();sr.appendChild(x);x.onclick=()=>{document.body.dataset.fn=1}</script>`. Bracket 2: `eval_script("document.body.append(h); h.parentNode===document.body")` (a host move, and a new bind). Bracket 3: `eval_script("x.dispatchEvent(new MouseEvent('click')); [document.body.dataset.fn, document.body.dataset.attr].join()")` | bracket 2 `true`; bracket 3 `"1,"` | bracket 2 `true`; bracket 3 `"1,"`, so `fn` ran and the attribute did not. A bind pass at every bind, or a shadow-including Insert walk, reaches the unmarked `x` (two deliverable attributes, so the restriction returned), re-raws `onclick` over `fn`, and gives `",1"` |
| URL-loaded iframe | E5 | red | `build_pipeline_from_loaded` via the out-of-process entry (`load.rs:327`) | `<button id=b onclick="postMessage('eh-hit','*')"></button><script>postMessage('ran','*');document.getElementById('b').dispatchEvent(new MouseEvent('click'))</script>`, cross-origin, unsandboxed | posts `["ran"]` | posts contain `"ran"` and `"eh-hit"` |
| URL-loaded iframe | E5b | conditional (Q5) | the same-origin URL entry (`load.rs:221`) | E5's markup through a hermetic same-origin loader, if one exists | as E5 | as E5. Slice a's plan decides whether E5b is written; if not, it states that `:221` and `:327` call the same builder, which E5 pins |
| srcdoc iframe | E6 | red | `build_iframe_pipeline` (`load.rs:465`) → `build_pipeline_interactive_shared` (`:471`) | `<iframe srcdoc='…M…'>`; read the in-process child's body | child no `data-hit` | child `data-hit == "1"` |

### §6.3 Mutation guards and kept-green (input)

The mutation cells below are the pre-measurement prediction. The measured record is slice a's per-PR plan §4.6
(`docs/plans/2026-10-eh-attrs-slice-a.md`): its cells and notes 1–6 are canonical (PM decision 63).

Kept green: `tests_event_handler_attrs.rs` and the consumer's `spec_level_tests`. Mutation guards: dropping the marker
check fails C1–C3, U1, E3, U7. Deleting the bind delegate fails E1, E2, E4–E7, U7. Dropping the document-marker
check, or holding the once-state on the consumer struct, fails U9 and E8. A light-tree bind pass fails E7 and U8 (iii).
A capped bind walk fails U10 or U11, and a recursive one fails U10. A shadow-including Insert walk fails U9 and E8. A
write-side entry that never processes fails C2, C4, C5, C11 and U5 (i). One without the restriction fails C16. One that processes a Window-delegating pair at a write fails C17 and C18; a skip at insertion that covers Window-delegating pairs fails C19; an impl that processes a mixed element's Window-delegating pair at T-a3 fails C20; a test that omits the reflecting set or `<frameset>` (e.g. reusing `scope==Window && is_body_element`) fails C19b / C19c; a test keyed on the node document or the bound document instead of (tag, name) fails C8 or C18. A bind
pass that walks a document other than the bound one, or walks template contents, fails U8 (i) / (ii). A pass keyed on
`document_root()` instead of the bound document fails U12. WPT: no subset
is claimed.

### §6.4 Touch-time 1000-line check (input)

Slice a's file placement, the 1000-line projection at placement of every file it touches or creates, and whether
a standalone prerequisite split is needed are canonical in slice a's per-PR plan §5
(`docs/plans/2026-10-eh-attrs-slice-a.md`; PM decision 39).

## §7. Gaps: dispositions (decided)

Inclusion criterion (PM decision 18 (2)): a gap whose root cause is the delivery mechanism this program builds is in
scope; any other becomes a slot.

- **G1, initial-HTML handlers never wired** (H9). **In scope, slice a** (D-5, D-11; E1–E8, U7, U10, U11). Its
  exposures E-G1 are registered elsewhere (§8.1).
- **G2, shadow-tree elements never delivered.** **In scope.** Slice a covers pre-bind shadow trees (E7, U8 (iii)).
  Slice b covers the rest (S-1). The shadow-root filter (`mutation.rs:289`) is **not** changed: it serves every consumer.
- **G3, target step 3 not modelled.** Slot `#11-event-handler-target-determination` facet 1. It is T3, slice b's prerequisite PR, owned by this program (PM decision 25; §1.2), after C1 (PM decision 153). Slice a keeps HEAD's path for every
  Window-delegating pair (X-W; C17, C18, C19). Rev 8.<t> (PM decision 129): from T3, X-W keeps HEAD's path only for a
  `<body>`'s WindowEventHandlers names (to S-5); every other Window-delegating pair is on the spec's target and the
  spec's processing points (T3 memo D-T7). T3 has no residual where step 3 reads a pre-existing gap (template contents
  and cross-document moves are closed by C1 and C0 before it, T3 memo §1.2) or where a fragment parse creates a
  `<body>` / `<frameset>` in an active document (T3 processes it at creation, PM decision 699; T3 memo §1.2 R-E).
- **G4, the Window-reflecting body set.** Slot `#11-event-handler-target-determination` facet 4 (trigger: slice a's
  landing). Interaction: E-G1 (i); the six names are Window-delegating pairs under X-W. Rev 8.<t> (PM decision 129): T3
  closes G4 whole, target and timing; after T3 the six names are no longer under X-W (until rev 8.<t>: the timing was
  S-5's).
- **G5, reads never deliver.** **In scope, slice b** (S-1). Rev 8.<t> (PM decision 153): its clone half at **T3** (T3 memo
  D-T10).
- **G6, stale docstring** `crates/core/elidex-ecs/src/dom/attribute_reconcile.rs:21-23`. **Fixed in slice a**
  (docstring only).
- **G7, writes, creations and insertions while unbound are not observed.** Slot
  `#11-unbound-attribute-writes-bypass-consumers` (§8.1). D-5 relies on it.

## §8. Slots, records, hand-offs

**Registration timing.** A gap that exists at HEAD is registered when the PM accepts its text. A gap created by code a
slice adds is registered at that slice's landing. The PM registers every slot; this memo writes defect texts only.
`#11-eh-attrs-creation-time-call-sites` records pre-existing defects (§8.2). **Its text is sent to the PM when the
umbrella-approval PR lands, and the PM registers it then** (O10; PM decision 24). The approval PR lands before slice a's
PR is cut (§1.2), so the slot exists before slice a's PR, which cites it (§10.1).

### §8.1 Existing slots cited (ledger state 2026-10-25, after the PM's R2 corrections)

- `#11-event-handler-target-determination`: **re-routed by the PM (decision 25): the owner is this program (Layout
  lane).** Facet (1) step 3 is **T3**, slice b's prerequisite PR (x3), started after C1 lands (x6; PM decision 153),
  outside the IBP-sandbox DAG, because no IBP-sandbox N-PR touches target determination. Facets (3) and (4) are part
  of T3 as well (x3, rev 8); facet (2) rides with it unless its plan splits it off. Facet (4) is G4, with the trigger "slice a's landing" (E-G1 (i)).
  Rev 8.<t> (PM decisions 129, 131): T3's plan carries all four facets (facet (2) in HEAD's single-browsing-context
  model) and, from S-5, the end of X-W for the pairs HEAD targeted at the element; the slot closes at T3's landing
  (T3 memo §8.3 item 1). Rev 8.<t> (PM decision 153): T3 also carries S-1's clone path.
- `#11-htmlframesetelement-interface` (rev 8.<t>): **registered by the PM at T3's landing (decision 130)**, compat layer (`frameset` is an
  obsolete feature, HTML §16.3.2 Frames); owner none until the compat plugin work starts. Text: T3 memo §8.3 item 9.
  After T3 a `<frameset>`'s handler content attributes act on the Window, while its interface stays absent.
- `#11-event-handler-content-attribute-applicability` (rev 8.<t>): **registered at T3-a's landing** (PM decisions 741 (3), 767; until T3 memo rev 6: "at T3's landing"), owner this program (Layout lane);
  the attribute change steps' step 1 (a handler name that is not an event handler content attribute of that element,
  a non-null attribute namespace, MathML elements). Facets (1) and (3) (the local-name test, MathML elements) are closed
  by **T3-a** (PM decisions 168, 741 (3), 767; the T3-a memo D-T8; until T3 memo rev 6: by T3); facet (2), the attribute namespace, has no case at the base: no attribute namespace is modelled and no writer of a namespaced attribute exists, so it closes with the PR that makes one writable, not by a namespace on the event alone (R3 wave 1 W1-3; registered to this program with slice b by PM decision 168, its owner after W1-3 being the T3-a memo Q-A6; until rev 8.<t>'s PM 767 fold: "by **slice b** (it needs
  `MutationEvent::AttributeChange` to carry a namespace)"). Text: the T3-a memo §8.3 item 1 (until T3 memo rev 6: T3 memo §8.3 item 8).
- `#11-csp-inline-handler-enforcement`: **registered by the PM** (E-G1 (iii)). Root cause: `script-src` is not
  implemented at all, so this is not a handler-specific gap. Since HEAD, no inline script or inline handler passes a CSP
  check. Slice a only adds initial-HTML handlers to that set. `frame-ancestors` is the only CSP directive parsed and
  enforced (`origin.rs:202`). Trigger: IBP-sandbox N3 `IBP-sandbox-csp-list`'s per-PR plan.
- `#11-template-contents-owner-document`: **marked by the PM as EH's C1** (x5). Owner: Layout lane (PM decision 123); C1, after C0 (PM decisions 143, 153); plan `docs/plans/2026-10-eh-attrs-c1-template-owner-document.md`. The trigger includes those two landings. Re-eval 2026-11-30.
- `#11-unbound-attribute-writes-bypass-consumers` (G7): **widened by the PM.** Its population is every DOM write made
  while no dispatcher exists (creation, insertion, attributes), and it is recorded as relied on by D-5. Its trigger
  includes "an unbound path that creates or inserts elements after the bind pass has run". Owner: this program.
  Re-eval 2026-12-31.
- `#11-iframe-lifecycle-direct-attribute-writes`: **corrected by the PM.** `content/iframe/lifecycle.rs:358` writes
  `src` only, and it is the only production writer. It writes no `on*` attribute, so it does not enter "creation-time
  attributes".
- `#11-event-handler-attribute-edge-cases`: facets are the handler scope chain and the `onerror` five-parameter form
  (*get the current value of the event handler* step 3.9). Owner: core VM. Trigger: slice a's exposure (E-G1 (ii)).
  Re-eval 2026-11-30.
- `#11-cross-document-adopt-on-insert`: importNode / adoptNode and insertion adoption (P1). Not touched.

### §8.2 Proposed slot text (for the PM; defects only)

> **`#11-eh-attrs-creation-time-call-sites`** — owner: Layout lane (program `EH-attrs-at-creation`). **What it
> records:** the approved interim of slice a (PM decision 23). An element created after bind is processed at its first
> insertion, over HEAD's light-tree walk, instead of at its creation. A handler write is restricted: it processes the
> element first only when the written name is the element's only deliverable ordinary handler content attribute, and never for
> a `<body>` / `<frameset>` handler that delegates to the Window.
> **Defects** (observable both at HEAD and after slice a): (1) a handler write (`setAttribute` / `removeAttribute` of
> `on<T>`, or `E.on<T> = …`) on a never-inserted element with two or more deliverable ordinary handler content
> attributes is overwritten at the element's first insertion (at HEAD: for every element, at every insertion; once N1
> lands, such a write whose value is a callback at that insertion (an IDL write, or a `setAttribute` value compiled
> by a read or a dispatch before it), when the insertion is into a DOMParser document or puts a DOMParser document's
> element into a detached subtree of the bound document, reads `null`, where HEAD's getter compiles the attribute;
> dispatch runs nothing at both; spec: the callback runs; N1 memo NR8 (a), pin C8g, closed by S-5; rev 8.<t>: since T3
> not for an element a clone, a fragment parse or a DOMParser build created, T3 memo §8.3 item 2 (a)); (2) a
> never-inserted clone of an element with a handler content attribute reads `null` from `on<T>` (rev 8.<t>: T3 memo §8.3 item 2
> (b)); (3) after
> `c = document.body.cloneNode()`, a `window.on<T>` assignment is overwritten by `c`'s attribute at `c`'s first
> insertion (rev 8.<t>: T3 memo §8.3 item 2 (c)); (4) an element with a handler content attribute placed directly under a shadow root after bind, or in the
> shadow tree of a host inserted after bind, is never processed (rev 8.<t>: since T3 not for an element a fragment
> parse or a clone created, T3 memo §8.3 item 2 (d)); (5) DOMParser and template-content elements are
> processed at their first insertion, not at creation, and an unmarked element placed into template contents or into a
> DOMParser document reads a non-null handler (the DOMParser-document half of this second clause holds only until N1
> lands, after which such an element reads `null` there, K-5; its template-contents half until C1 lands, and the first clause for
> both until slice b; rev 8.<t>: since T3 not for the elements of a DOMParser build or of template contents a fragment
> parse or a clone created, T3 memo §8.3 item 2 (e)); (6) of a parent with n > 10 000 children inside a subtree inserted
> after bind, the first n − 10 000 children are not reached by the Insert walk (HEAD's `children_iter_rev` cap, which
> starts at the last child, D-10), so they are never processed (rev 8.<t>: since T3 not for a subtree a clone or a
> fragment parse created, T3 memo §8.3 item 2 (f)); (7) a `<body>` / `<frameset>` handler that delegates
> to the Window (X-W) is re-raw'd at every insertion over any write, and a `<body>` outside an active document
> delivers it to the page Window (rev 8.<t>, PM decision 129: **from T3** (7) is replaced by the text of T3 memo §8.3 item
> 2 (g), which T3's landing records in the ledger). **Classification:** all seven are **pre-existing**. Each exists at HEAD
> with the same outcome or a wider population, except that once N1 lands its compile gate changes the getter outcome,
> with dispatch equal, of three DOMParser subsets: (1)'s subset whose value is a callback at the insertion (away from
> the spec, PM decision 90), the DOMParser-document half of (5)'s second clause, and (7)'s pairs that HEAD targets at
> the element (both to the spec's `null`; rev 8.<t>: what T3 changes in this sentence, the third subset and the counts,
> is T3 memo §8.3 item 2 (h)); and from C1 in a fourth, (5)'s template-contents half, whose getter reads `null` where HEAD compiled the attribute and whose dispatch changes too, since HEAD ran the handler (the spec's outcome; §1.4 (iv)); slice a
> narrows (1) and makes nothing worse (K-0). **Registered** when
> the umbrella-approval PR lands. **Why deferred:** the end state processes at creation. That writes raw values on
> DOMParser nodes (null-reading only under N1), on DOMParser bodies (needs T3) and on template contents (needs C1's
> handling). N1 itself needs slice a first (§L). **Closes:** at slice b's landing, which removes the restriction, X-W and
> processing at insertion (§1.4). If §1.6 branch B leaves a template-contents residue, slice b's plan drafts its slot
> text and the PM registers it when that plan passes plan-review (§1.6). **Closing path:** slice a lands → N1 → C0 → C1 → T3-pre → T3 (this program) → slice b (PM decisions 153, 722; T3 memo §8.3 item 2 (i)). **Trigger:** N1, C0, C1, T3-pre and T3 landed (the ledger
> records each landing's merge commit in this section's Source line). **Re-eval:**
> 2026-12-31. **Confidence:** medium. The order is fixed (PM decisions 21, 23, 25, 153) and T3 has an owner; T3 starts after C1, T3-pre and L3's PR (io) land (rev 8.<t>, PM decision 722; until then "after C1 lands"), and C1's handling is decided (branch A, PM decision 123).

### §8.3 Records for existing slots (for the PM)

None are new. E-G1 (i) and (ii) are already in the ledger (`project_open-defer-slots.md` :540, :551), E-G1 (iii) is in
`#11-csp-inline-handler-enforcement` (§8.1), and rev 5's "slice b as a facet (1) dependant" is superseded by PM
decision 25 (the slot's owner is this program; §1.2). Rev 4's records for `#11-template-contents-owner-document` and
`#11-unbound-attribute-writes-bypass-consumers` were made by the PM (§8.1).

### §8.4 Hand-off: what N1's rev 4 must change (N1 is not edited here)

Coordinates are N1 memo `0f99538d` lines. Each item cites a decided clause of this umbrella, not a cell ID, because
slice a's plan re-derives the cells.

1. **Rename the prerequisite.** Every `EH-insert-once` becomes "`EH-attrs-at-creation` slice a", and the path
   `docs/plans/2026-10-event-handler-insert-once.md` becomes `docs/plans/2026-10-event-handler-attrs-at-creation.md`.
   Lines: `:21`, `:29`, `:88`, `:188`, `:202`, `:282`, `:322`, `:358`, `:407`, `:447`, `:476`, `:559`, `:575`, `:587`, `:623`,
   `:633`, `:634`, `:671`, `:728`, `:733`. `:540` is revision history; N1's idiom decides.
2. **Restate P-N5 / H-P16 against K-1:** "creation-time handler content attributes are processed at most once; no
   insertion, move or rebind writes handler state from them", not "delivered at first insertion".
3. **R2 G2 (R-a):** cite K-1 and K-2 with their slice-a qualifiers (ordinary pairs only, X-W; at most one
   deliverable ordinary attribute, D-8). The multi-attribute case and Window-delegating pairs are S-5. N1 decides
   whether this form suffices for its cells (O5; for N1's landing, O5 is decided by PM decision 90, §11).
4. **R2 G3 (R-b):** cite K-3 with the same qualifier. The multi-attribute case and Window-delegating pairs are S-5.
   For any N1 cell on a `<body>` / `<frameset>` Window-delegating pair, N1's expectation after slice a is HEAD's
   outcome (X-W), not K-1 to K-3's, except for an element-targeted pair in a DOMParser document, where it is the
   spec's `null` (§1.4 (iii), N1 cell C8h). At N1 rev 3 (`0f99538d`) no R-a / R-b or C8-family cell uses such a pair: they
   use `onclick` on a `div` or `button`. The only such shape is the Window `<body onpopstate>` arm of the `:635`
   raw-form rewrite (O6; N1 `:502`, `:604`). It is a pair on the live document's `<body>`, where X-W gives HEAD's
   outcome, so N1's expectation there must be HEAD's (the raw handler compiles and runs). Measured with
   `git -C <N1 worktree> show 0f99538d:docs/plans/2026-10-ibp-sandbox-handler-gates.md | /usr/bin/grep -nE
   '<body|onhashchange|onpopstate|WindowEventHandlers|frameset|Window-reflecting|window\.on|R-a|R-b'` (hits `:78`,
   `:410`, `:604`, `:621`; none for `R-a` / `R-b`, which are N1 R2 labels, §L L5–L6).
5. **H-P13 stays.** Cite K-5's slice-a statement (the bind pass walks only the bound document and writes nothing into a
   DOMParser document) as the evidence.
6. **NR7 (`:575`):** "on a base without slice a". Cite §L's verdict as the measurement behind it: without slice a, N1
   is worse than HEAD on the getter.
7. **The `:623` row** ("the Insert arm turns handlers raw on every move") is fixed by slice a (K-1), in its slice-a
   form; the residual (X-W, and the multi-attribute case: N1 memo §5.4 NR8 (a)) is S-5's.
8. **§9 prerequisites (`:671`):** the re-measure obligation on the post-slice-a base names slice a's landing commit.
9. **Check, not edit:** whether any N1 cell builds initial HTML through a shell pipeline. From slice a the bind pass
   delivers its inline handlers (K-6), which may change that cell's HEAD prediction.
10. **IBP-sandbox umbrella, "Prerequisite outside the DAG" paragraph** (`0f99538d:docs/plans/2026-09-ibp-sandbox.md`
    line 38; it lands with N1). Replace the label `EH-insert-once`, the retired path, and the contract text "a baked
    attribute becomes a raw value once per element, never on a later move" with "`EH-attrs-at-creation` slice a"
    (`docs/plans/2026-10-event-handler-attrs-at-creation.md`; contract K-1). Cite §L for "N1 depends on it landing
    first".

## §9. Input notes for slice b's per-PR plan (not decisions)

These are evidence that slice b's plan must account for. None of them is decided here.

- **N-1, call-site shape (O9; R2 A1 IMP).** The parsers' fragment build takes the dispatcher at
  `begin_detached_fragment` (`teardown.rs:325`) and restores it only after `finish_detached_fragment`
  (`elidex-html-parser/src/lib.rs:163-166`; strict `tree_builder/mod.rs:178` takes it, `:239` restores it). So a
  `MutationEvent` fired inside the fragment build reaches no consumer. Parser template attach runs pre-bind, inside the
  fragment build, or inside the DOMParser build (dispatcher taken at `inert_document.rs:112`). In all three places it is
  suppressed exactly where a signal would be needed. Every production fragment parse is called from
  `elidex-script-session` `mutation/html_fragment.rs` (`:87`, `:168`, `:203`, `:212`, `:229`, `:239`), so rev 4's
  "fragment build: direct-call-incapable" is wrong. The remaining constraint is timing: those calls run after the
  adopt, and step 3 of target determination matters only for `<body>` / `<frameset>`. Rev 4's other evidence (crate
  graph: the parser crates cannot depend on `elidex-script-session`, which depends on `elidex-html-parser`; a parser
  hook is an extension point that falls under *Plugin-first extensibility*) still stands.
- **N-2, clones of template contents (R2 A2 IMP).** A clone inherits `owner_document(src)` (`tree_clone.rs:204-210`),
  and the template-contents fragment's owner is the template's own document under the C2 approximation
  (`dom/mod.rs:551-557`). Without C1 part (a), a node carries no per-node "from template contents" fact. In branch B,
  processing such a clone at creation makes its getter return a function where HEAD and the spec read null (worse than
  HEAD, against S-3). And a clone of a clone cannot be told apart, so a detector cannot distinguish it (S-4).
- **N-3, rev 4's branch-B design.** The Insert arm kept as the template-contents processing point, "K-B" (it processes
  nothing else), cell B9 and the slot `#11-eh-template-contents-at-creation`. R2 A3 found that a kept arm acts as a
  release fallback for any unmarked element (against S-4). R2 A2's N-2 makes "their clones" unidentifiable.
- **N-4, R8's blockers (the reason for x3–x5).** (a) DOMParser nodes: raw entries at parse; HEAD's compile gate reads
  only the realm flag, so `d.onclick` compiles (spec: null, *get the current value* step 3.2) → N1 first. (b) A `<body>`
  with a WindowEventHandlers attribute whose node document at creation is not active (a DOMParser document's, or a
  fragment's parsed in such a document's context; until rev 8.<t>: "A DOMParser or fragment-throwaway `<body>`", the
  parser's own Document holding no fragment element, T3 memo P-T17): `resolve_handler_target` returns the page Window
  for any `<body>` → T3 first. (c) Template contents and their clones: the composition does not suppress them (the
  composed root is a fragment, which falls back to the page document) → C1's handling.
- **N-5, an existing template-inclusive walker (R2 A1 MIN).** `collect_template_inclusive_descendants` in `elidex_form::init`
  (`crates/dom/elidex-form/src/init.rs:111`, `pub(crate)`; used at `inert_document.rs:168` and form `clone.rs:49`). A
  second walker for the same concept is a "One issue, one way" risk. D-11's uncapped walker and H10a are the others to
  reconcile.
- **N-6, policy at direct-call sites (R2 A1 MIN).** A direct call needs the `SpecLevelPolicy`, which the VM holds (H19).
  The capable sites (`inert_document.rs:110`, `elidex-dom-api` `node_methods/clone.rs:227`) carry none today. Rev 8.<t>:
  the clone's site is T3's: an `EventHandlerAttributeConsumer` arm on L3 RC-2a's `MutationEvent::CloneCreated` (from
  T3-pre, `SubtreeCreated` with the `Clone` cause, PM decision 722); the consumer holds the policy (T3 memo D-T10, PM
  decision 359).
- **N-7, rev 4's slice-b cell shapes** (B1: multi-attribute DOMParser write; B2: never-inserted clone read; B3: body
  clone vs `window.on*`; B4: template contents before and after a move out; B5: DOMParser `<body onpageshow>`; B6:
  shadow-root-direct and post-bind host shadow content; B8: multi-attribute post-bind write). Each pins a §1.5 window.
  Slice b's plan re-derives them and adds its detector cells (S-4).
- **N-8, P3:** `inert_document.rs:78-84`'s docstring.
- **N-9, Q6:** whether any `MutationEvent` consumer matches exhaustively, so that a new variant would need edits there.

## §10. Definition of done

### §10.1 Slice a

- The marker components and functions of §4 exist in `elidex-script-session` and are re-exported. The Insert arm walks
  HEAD's set (D-10). The bind walk is uncapped and iterative (D-11). It is keyed on, and marks, the bound document
  (D-5), and its once-state is not on the consumer struct.
- The write-side entry has D-8's observable restriction, X-W holds at every slice-a timing, and the restriction line names
  `#11-eh-attrs-creation-time-call-sites`. `handle_insert`'s doc names the same slot as the interim's closure.
- `initialize_consumers` delegates to the consumer, and its doc and the `event_handler_attrs` field doc no longer say
  the consumer needs no init or is dual-arm only (§4).
- Both VM element setters call the write-side entry as D-6 states. No other VM code touches either marker outside
  tests.
- The clone-policy entries (element marker; Document marker, both deliberate non-copy) are added, the module and field docs
  and the `vm_api.rs:298-312` comment are updated, and the G6 docstring is corrected.
- Cells C1–C9, C11–C19, C19b, C19c, C20, U1–U12 and pins E1–E8 are present (E5b per the plan's recorded Q5 decision; E8 in the form
  Q7 settles). Every red cell
  and red pin was run red on the pre-fix base with its control green, and the failures are recorded in the PR
  description. All pass at the head.
- `cargo test -p elidex-script-session --all-features`, `-p elidex-js`, `-p elidex-shell`, `-p elidex-ecs`, and
  `mise run ci` are green.
- `#11-eh-attrs-creation-time-call-sites` was registered when the approval PR landed, and slice a's PR cites it.

### §10.2 Slice b

- Its own per-PR plan-memo has passed `/elidex-plan-review`. That plan decides O9, the branch-B treatment, the
  identification of template-content clones, and its cells.
- Its base contains T3 (landed as this program's own PR, PM decision 25) and N1, and C1's handling is decided. Rev 8.<t>:
  its base contains N1, C0, C1 and T3 (PM decision 153).
- S-1 to S-7 hold, each pinned by cells its plan names (S-5 without T3's part, S-1 without the clone path; rev 8.<t>;
  and without the fragment path and the DOMParser build, rev 8.<t>, PM decisions 699, 705, 711; until T3 memo rev 4.8:
  "without the fragment path except the declarative shadow content and template contents it creates").
- The PM closes `#11-eh-attrs-creation-time-call-sites`, and facet (2) of
  `#11-event-handler-content-attribute-applicability` (rev 8.<t>; facet (2) has no case until a namespaced attribute is writable, and whether it is still slice b's to close then is the T3-a memo Q-A6, with its prerequisites in that memo's §8.3 item 1; facets (1) and (3) close at T3-a, the T3-a memo D-T8, PM decisions 741 (3), 767; until T3 memo rev 6: at T3).

### §10.3 Program

- Both slices have landed in the §1.3 order, each with its own reviewed plan-memo.
- T3 (`#11-event-handler-target-determination` facets 1, 3 and 4; from rev 8.<t> all four facets and the end of X-W
  for the pairs HEAD targeted at the element, PM decision 129) has landed before slice b (x3). Rev 8.<t> (PM decisions 153, 168, 741 (3)): T3 also carries S-1's
  clone path, and lands after C1 (x8) and after **T3-a** (x15), which carries applicability facets (1) and (3), and **T3-b** (x16), which carries
  the `<body>` interface choice (until T3 memo rev 6 T3 carried them; PM decision 767). Each has its own plan and DoD.
- K-0 to K-6 and S-1 to S-7 hold at slice b's landing, X-W and the D-8 restriction gone (S-5). The one qualifier is
  S-2's in branch B: a case for which slice b's plan cannot supply the per-node template-origin fact is not processed at
  creation, stays HEAD-equal (S-3 has no exception), and is registered as the branch-B residue slot (§1.6).
- **Checker** (run; do not trust a stored verdict): `python3 .claude/skills/elidex-plan-review/preflight.py
  docs/plans/2026-10-event-handler-attrs-at-creation.md` exits 0.

## §11. Open for plan-review

Decided (no longer open): O1 split (PM decision 21); order (PM decision 21, §L); O2 interim (PM decision 23, §1.4); O3
walk cost (D-5); O6 (PM decision 23: no processing at insertion in the end state; branch B is slice b's, §1.6); O7 G7
(§8.1); O8 N1 edits (§8.4); **O10 approval vehicle (PM decision 24, D-12)**; **O5 N1's landing (PM decision 90,
below)**.

**O5, N1's R-a / R-b (decided, PM decision 90).** In slice a they hold only for ordinary pairs of elements with one
deliverable ordinary handler attribute (K-2, K-3 qualifiers); a Window-delegating pair gets HEAD's outcome (X-W; §8.4
item 4). Is that sufficient for N1's landing? *Decided by the user (choice B1, 2026-10-31; PM decision 90; this text
carried by N1's PR, PM decision 93).* §L's lens (L2.1 / L6.1, "worse") reaches N1 memo §5.4 NR8 (a) as a
classification; §L's verdict, which is order-only (no order puts N1 before slice a), stands. N1 lands after slice a.
NR8 (a) is a defect, recorded under `#11-eh-attrs-creation-time-call-sites` defect (1) and closed by slice b's S-5.
The criterion for N1's landing is the IBP-sandbox node memo's §5.1 claim (each interim window "equals HEAD or is more
restrictive than it; none is a restriction escape"); §L classifies NR8 (a) as worse but does not gate the landing.

Moved to slice b's plan (not open here): **O9** call-site shape (§9 N-1; rev 8.<t>: decided for the clone path by T3's
plan, T3 memo D-T10, one session entry point called from the `CloneCreated` arm; and for the fragment path by PM
decision 705, T3 memo D-T12, the same arm on `SubtreeCreated`, `CloneCreated` generalised by T3-pre (the T3-pre memo
§4.2, PM decision 722), and by PM decision 711 for the DOMParser build, the same event queued in its window, the T3-pre
memo §4.4; slice b decides it for its paths); branch-B treatment (§9 N-2, N-3); Q6 (§9 N-9).

Open:
- **O4, writer completeness.** Is §5.0 complete for handler state, in particular any VM path that writes
  `EventListeners` handler state outside the IDL activation path? **Owner: slice a's plan-review. Close point: that
  plan-review**, which runs the §5.0 census and a search of VM writers of `EventListeners` at its base and records the
  result. A path it finds is a K-0 case or a slot. Slice b re-runs it (S-7).

Questions for a scout:
- **Q3:** does elidex fetch or fire `load` / `error` for an `<img>` in template contents, or in a clone of them? (The
  size of N-4 (c)'s exposure; slice b's input.)
- **Q5:** is there a hermetic harness for the same-origin URL iframe path (`load.rs:221`)? (E5b.)
- **Q7 (rev 5):** do E8's three brackets build as stated, specifically `attachShadow` on a `div` and `cloneNode` with
  two handler attributes in an initial script, through `build_pipeline_from_loaded`? **Fallback if not:** take `x` from a
  DOMParser document (`parseFromString('<div id=x onclick=… onmouseover=…>')`: unmarked, and no Insert when placed under
  the shadow root) and keep brackets 2 and 3. If `attachShadow` cannot run there either, E8 is not dropped: its
  obligation (a real multi-bracket path showing that no later bind and no Insert reaches an unmarked
  shadow-root-direct element) stays in §10.1, and slice a's plan-review decides the replacement shape before
  implementation.

Rev 4's Q2 (is the bound document equal to `document_root()` at every bind site) is moot: D-5 keys on the bound document.

## §12. R1 disposition (summary)

R1 (on rev 3 `399f4534`; CRIT 0 / IMP 9 / MIN 18) is disposed in rev 4.1 §12 (`ab93a51a`): IMP 9 fixed, MIN 17 fixed
and 1 rejected (A2-MIN-2). Rev 5 reverses two lines: A2-MIN-1 (false premise P17) is fixed by D-11, A2-MIN-2 (wrong
rejection reason P18) by D-5.

## §13. R2 disposition (umbrella plan-review R2 on rev 4.1 `ab93a51a`)

The record lists IMP 4 and MIN 16 by axis. Two of the 16 MINs restate A3's IMP, which gives the record's
"MIN 14 distinct". Each line below was re-verified against `fd6955e8` before disposition.

| id | sev | finding (short) | disposition | reason / where |
|---|---|---|---|---|
| A3-IMP | IMP | the walker cap (depth and width) leaves pre-bind elements beyond 10 000 unprocessed; branch A ends worse than HEAD | **fixed** | D-11, K-6, U10, U11; §1.5's depth-cap window deleted |
| A1-IMP | IMP | rev 4's O9 event shape reaches no consumer; fragment build misclassified | **moved to slice b input** | O9 is not decided here; §9 N-1 |
| A5-IMP | IMP | registration timing rests on open O10 | **fixed** | O10 decided (D-12); §8, §8.2, §10.1 consistent |
| A2-IMP | IMP | "template contents and their clones" has no per-node fact without C1 | **moved to slice b input** | S-2, S-3, S-4 state the invariant; §9 N-2; K-B withdrawn |
| A3-MIN-1 | MIN | §8 hand-off timing rests on open O10 | **fixed** | as A5-IMP |
| A3-MIN-2 | MIN | the branch-B Insert arm is a release fallback; K1 analyses branch A only | **moved to slice b input** | S-4; §9 N-3 |
| A3-MIN-3 | MIN | U9 is simulated; E3 cannot catch a missing document marker | **fixed** | E8 (a real two-bracket path) |
| A1-MIN-1 | MIN | `collect_template_inclusive_descendants` not mapped | **moved to slice b input** | §9 N-5 |
| A1-MIN-2 | MIN | direct-call sites lack `SpecLevelPolicy` | **moved to slice b input** | §9 N-6 |
| A4-MIN-1 | MIN | bare "§8.1.3.4" in §L | **fixed** | §L columns |
| A4-MIN-2 | MIN | scope chain / onerror cited without a step | **fixed** | step 3.9 in K-0 E-G1 (ii), §3, §8.1 |
| A4-MIN-3 | MIN | no row for the `consumer_dispatcher.rs:58-66` field doc | **fixed** | §4 docstring line, §10.1 |
| A5-MIN-1 | MIN | the G7 slot covers attribute writes only | **fixed (by the PM's ledger update)** | cited in §8.1; §8.3 record removed |
| A5-MIN-2 | MIN | the ledger says `lifecycle.rs:358` writes src/srcdoc/loading | **fixed (by the PM's ledger update)** | cited in §8.1 |
| A5-MIN-3 | MIN | the C1 row has no trigger for the PM's decision point | **fixed (by the PM's ledger update)** | cited in §8.1 |
| A5-MIN-4 | MIN | the depth-cap open item is embedded in decided K-B (= A3-IMP) | **fixed** | K-B withdrawn; cap resolved by D-11 |
| A5-MIN-5 | MIN | §8.4 cites cell IDs that slice a's plan re-derives | **fixed** | §8.4 cites clauses |
| A2-MIN-1 | MIN | the §1.5 rationale "HEAD processes no pre-bind element" is false (= A3-IMP) | **fixed** | P17, D-11 |
| A2-MIN-2 | MIN | U9 / U7 reuse one consumer; production rebuilds per bind | **fixed** | fresh consumer per call (§6.1); E8 |
| A2-MIN-3 | MIN | the bind pass is keyed on the `document_root()` singleton, not the bound document | **fixed** | D-5, H13a, P18 |
| A1-FP | FP | (not described in the record) | no action | — |

Counts: IMP 4: 2 fixed, 2 moved to slice b input, 0 rejected. MIN 16: 13 fixed (3 of them by the PM's ledger update),
3 moved to slice b input, 0 rejected.

## §13.1 R3 disposition (umbrella plan-review R3 on rev 5 `8975d74a`, applied in rev 6)

R3: CRIT 0 / IMP 1 / MIN 16, each re-verified against `fd6955e8` before disposition. The A1 note is in neither total.

| id | sev | finding (short) | disposition | reason / where |
|---|---|---|---|---|
| R3-A3-IMP | IMP | T-a3 × DOMParser `<body onhashchange>`: rev 5 gives `window.onhashchange === fn`, matching neither HEAD nor spec | **fixed** | verified from `resolve_handler_target` (`event_handler_consumer.rs:328-334`, no step 3). D-8 (b), K-0 T-a3 row and reconciliation, I2×I8, §1.5 G3, cell C17 |
| R3-A3-MIN-1 | MIN | step 5.1 CSP not modelled; "elidex has no CSP" false | **fixed** | `frame-ancestors` is enforced (`origin.rs:202`); `/usr/bin/grep -rnE 'script-src\|script_src' crates` returns 0. §3 row 1, E-G1 (iii), §8.1 slot |
| R3-A3-MIN-2 | MIN | post-bind width-cap window unrecorded | **fixed** | §1.5, §8.2 defect (6) |
| R3-A3-MIN-3 | MIN | branch-B residue slot registration precedes slice b's plan | **fixed** | §1.6, §8.2: registered when slice b's plan passes plan-review |
| R3-A3-MIN-4 | MIN | end-state fail direction in branch A; dropped census obligation | **fixed** | S-7 |
| R3-A3-MIN-5 | MIN | E8 rests on open Q7 with no fallback | **fixed** | Q7 fallback (§11) |
| R3-A3-MIN-6 | MIN | O4 has no owner or close point | **fixed** | §11 O4: slice a's plan-review |
| R3-A3-MIN-7 | MIN | §1.5 claims U10/U11 cover template contents | **fixed** | §1.5: excluded by D-11/K-6; branch A covers them by S-1's extension |
| R3-A2-MIN-1 | MIN | D-5 key (bound document vs `document_root()`) unpinned | **fixed** | cell U12 (`create_document_node` leaves `document_root()` unchanged, `dom/mod.rs:413-419`); §6.3 |
| R3-A2-MIN-2 | MIN | no clone-policy row for the Document marker | **fixed** | §4: out-of-domain row beside `DocumentBaseUrl` (`tree_clone.rs:31`); §10.1 |
| R3-A2-MIN-3 | MIN | D-1 "never toward loss" false for a marked element | **fixed** | D-1 qualified; silent writer `clone_attributes` onto an existing `dst` (`tree_clone.rs:59-72`), no production caller; O4, S-7 |
| R3-A2-MIN-4 | MIN | S-2 vs §10.3 inconsistent on branch B | **fixed** | S-2: branch B requires slice b to supply a per-node template-origin fact; §10.3 no longer qualifies S-3 |
| R3-A5-MIN-1 | MIN | §8.3 records already in the ledger | **fixed** | §8.3 emptied; the remaining record is superseded by PM decision 25 |
| R3-A5-MIN-2 | MIN | T3 has no owner or mover; closure path not visible | **fixed** | PM decision 25: §1.2 row, x3, x6, §1.4 closing path, §8.1 |
| R3-A5-MIN-3 | MIN | slot defect (5): DOMParser half untrue after N1 | **fixed** | §8.2 defect (5) qualifier |
| R3-A5-MIN-4 | MIN | 1000-line projection only pre-touch | **fixed** | §6.4 decided obligation; `tests_event_handler_attrs.rs` is `crates/script/elidex-js/src/vm/tests/` (735) |
| R3-A5-MIN-5 | MIN | two init keys in one hook | **moved to slice a plan input** | §4 input note (`document_base.rs:276-285`); no harm shown, equal in production |
| R3-A1-note | note | `vm_api.rs:298-312` comment goes stale | **fixed** | §4 rewrite-site list; §10.1 |
| R3-A5-FP | FP | call-sites slot not yet in ledger | no action | registration timing is already committed (§8) |

Counts: IMP 1: 1 fixed. MIN 16: 15 fixed, 1 moved to slice a's plan input, 0 rejected.

## §13.2 Rev-6 re-check disposition (focused re-check on rev 6 `e7586cf3`, applied in rev 7)

The record lists CRIT 0 / IMP 2 / MIN 13. Two MINs appear under both axes (S-7, S-2's escape clause), which leaves
11 distinct MINs; the `<frameset>` MIN also sits in A3's IMP text. Each line was re-verified against
`fd6955e8` before disposition. Root pattern (the record's): slice a's once rule on a `<body>` depends on step 3 of
target determination (T3), which slice a lacks, and every narrowing by "which document" leaks another path. The
user's decision 案 X replaces the narrowing with the property rule X-W (§0.5).

| id | sev | finding (short) | disposition | reason / where |
|---|---|---|---|---|
| A3-IMP | IMP | K-2 / K-3's slice-a qualifier contradicts C17 (DOMParser `<body onhashchange>`, one attribute, is re-raw'd) | **fixed** | X-W; K-1 to K-3 qualifiers exclude Window-delegating pairs; C17's note; §8.4 items 3–4; O5 |
| A2-IMP | IMP | template-contents `<body>` passes rev 6's (b) (elidex gives the contents the bound document) and ends with `fn` on the Window | **fixed** | X-W reads no document; cell C18 (HEAD-equal; spec null, the T3 and slice b target) |
| self (rev 7) | — | found while writing X-W: a **marked** `<body>` moved into a DOMParser document, IDL-set, moved back is skipped and keeps `fn`, where HEAD and the spec both give the attribute's function. Same root, timing T-a2 | **fixed** | X-W covers the skip at insertion; cell C19. C8 and U2 become window pins. Rev 8: this extension follows the option text of 案 X quoted at X-W (PM ruling 10-26) |
| MIN-1 | MIN | `<frameset>` in rev 6's (b) has no HEAD basis | **fixed** | X-W's `<frameset>` paragraph: HEAD tests `body` only (`event_handler_consumer.rs:337-341`, VM `event_handler_attrs.rs:207-213`, `:425`); included because the test is the spec's step 1, and inclusion yields HEAD's outcome |
| MIN-2 | MIN | C17 "T3 inverts the cell" contradicts §1.5 ("T3 then slice b") | **fixed** | C17, C18: "reached after T3 and slice b"; x6 |
| MIN-3 | MIN | the width-cap population is reversed | **fixed** | `children_iter_rev` starts at the last child (`navigation.rs:323-330`), so the first n − 10 000 children are missed; §1.5, §8.2 defect (6) |
| MIN-4 | MIN | Q7's fallback drops E8 against §10.1 | **fixed** | §11 Q7: E8's obligation stays; slice a's plan-review decides the replacement; §10.1 |
| MIN-5 | MIN | T3 has no DoD; the ledger row's owner and trigger are stale | **fixed (memo half)** | §10.3 requires T3 landed before slice b and references the slot; the ledger row is the PM's to update and is not edited here |
| MIN-6 | MIN | §10.3 "no qualifier" vs S-2's escape clause | **fixed** | §10.3 names S-2's branch-B case as the one qualifier, HEAD-equal and registered (§1.6) |
| MIN-7 | MIN | S-7 is unsatisfiable for a silent writer onto a marked element (post-creation, outside S-1) | **fixed** | S-7: creation paths by S-1; post-creation silent writers by a stated disposition |
| MIN-8 | MIN | U12 cannot be built with the consumer installed | **fixed** | U12: pre-bind shape, as U7 |
| MIN-9 | MIN (uncertain) | the node document goes stale on a cross-document insert, so rev 6's (b) passes; the AttributeChange arm has no bound-document input | **FP after rev 7** | X-W reads neither the node document nor the bound document, so neither staleness nor a missing input reaches it |
| MIN-10 | MIN (uncertain) | S-2's template-origin fact as provenance over-suppresses after a move out | **fixed** | S-2: the fact tracks the spec node document, not the node's history |
| MIN-11 | MIN (uncertain) | the Document marker's "out-of-domain" clone row; Documents are cloneable | **fixed (verified)** | `clone_subtree`'s Document branch (`tree_clone.rs:163-230`); §4: deliberate non-copy (`:30`) |

Counts: IMP 2: 2 fixed. MIN 11 distinct: 10 fixed (one of them on the memo half only), 1 FP after rev 7, 0 rejected.
One self-found defect, fixed.

## §13.3 Rev-7 re-check disposition (focused re-check on rev 7 `68fb174e`, applied in rev 8)

Scope (the user's decision via the PM): the three IMPs and A3-MIN-4 only; no other change.

| id | sev | finding (short) | disposition | reason / where |
|---|---|---|---|---|
| A2-IMP-1 | IMP | D-2 processes every attribute E carries, so since rev 7's D-8 (a) a mixed `<body>` passing T-a3 re-raws its Window-delegating pair | **fixed** | D-2 aligned to X-W; cell C20 (i), (ii); T-a3 row; §6.3 |
| A3-IMP-1 | IMP | S-5 / §10.3 need every Window-delegating pair on the spec target via T3, but x3 required facet 1 only | **fixed** | X-W Status, §1.2 T3 row, x3, §8.1, §10.3 name facets 1, 3 and 4 (facet 2 may still split off) |
| A3-IMP-2 | IMP | no cell pins the reflecting six or `<frameset>` under X-W | **fixed** | cells C19b, C19c (HEAD-equal window pins); §6.3; §10.1 |
| A3-MIN-4 | MIN | 案 X cited without its option text | **fixed** | X-W quotes the option text; the extension to insertion follows it (PM ruling 10-26) |

Deferred to slice a's plan-review (input there, not decided here): A2-MIN-1, A2-MIN-2, A2-MIN-3, A3-MIN-1, A3-MIN-2,
A3-MIN-3.

Findings newly raised by the rev-8 attestation are handled in slice a's per-PR plan-review (slice a で扱う), not in
this umbrella.
