# `content-visibility` skip-contents program (umbrella plan-memo)

The `content-visibility` program of the inline-box predicate prereq program's §0.6 item 11
(`docs/plans/2026-09-inline-box-predicate.md`, approved by #526 at rev 73, `f1cf5d67`; "the parent"). Home slot
`#11-content-visibility-skip-contents`; owner the Layout lane. The parent's decisions bind here (its approval scope, `:3-9`);
item 11's content and item 12's ordering are **USER DECIDED 2026-09-28**. This memo changes that content only where the
user decided so on 2026-10-20 (U1: the reader set widened; U2: CV-details builds the shadow surfaces it consumes), recorded
as amendments of item 11 under the A114 idiom ("recorded, not edited", §0.5, §8 records) and ratified by the approval PR.

**Approval scope (the #526/#532 idiom, parent `:3-9`).** This memo **decides**: the nested-PR set, their order and edges
(§5.1–§5.2), and how the parent edges E20, E22 and E26 bind to them; the **observable invariants** this program needs from
its sibling programs — FU, forced style/layout update (D12); P1, element lifecycle hooks (D6); P2, shadow slot assignment
and flat tree (D8, D11) — and the canonical-focus gate CF-1 (CF-2 is CV-details' own), each with a named cell (§7), and their edges into it (a
sibling's mechanism and §5.3's risks for it are its own plan-review's input); each fact's home and representation kind for this program's own nested PRs (§4's *Home/kind* lines); the
program-level invariants K1–K17 with their landing qualifiers (§2); the D-items (§0.4); the user decisions U1–U2 and the
decisions O1–O2 (§0.5); the spec-semantics decisions (which algorithm, which steps, which outcome — §0.4, §3's *Branch*
column); **which nested PR wires skipped contents into each reader** (§5.1's reader table — the flip event every parent pin
names); and the slots and records (§8). §4's mechanism prose, §5.3's named risks, §6's tables, §7's cells, §3's *Touch* and
*Full enum?* columns, the per-PR edge-density verdicts marked *input* in §1, and every item marked *input* are **input to
each nested PR's plan-review**, which re-derives them; a sibling program's mechanism is its own umbrella's. A review finding
against that input changes this memo only when it changes a decision; otherwise it is recorded as a named risk in the
owning nested PR's input lines (§5.3) or as input to the owning sibling program.

**Approval vehicle.** A docs-only approval PR (the #526 / #532 vehicle). The user's merge approval ratifies the
approval-scope paragraph, the D-items, U1–U2 (the amendments of the parent's item 11) and O1–O2. **E26**: this memo's
approval precedes IBP-sandbox's first nested landing (parent `:1415`; IBP-sandbox memo D7,
`docs/plans/2026-09-ibp-sandbox.md:180`); the approval PR itself is a docs artefact gated by nothing. No umbrella A-row is
drafted: nothing here amends a surface the umbrella (`docs/plans/2026-08-line-box-decorated-inline-content.md`) ratified;
additions to the parent are recorded here under the A114 idiom ("recorded, not edited", §8 records).

**Revision 8** (2026-10-24) — final clerical pass; **USER DECIDED 2026-10-24: no review round after rev 8** — residual
risks ride the sibling and nested plan-reviews. Folds the terminal re-check of rev 7 (IMP-1, IMP-2, MIN-1–5) by lowering
altitude, with the user's adjustments (via PM): FU keeps only what its own landing can check — **FU-6** (cell C30, a test
consumer) and FU-3 in generic form; FU-1/FU-2 withdrawn, their cells C16, C20, C22 now CV-readers' own (K16); §4's
animation-reader line (a shell caller passing a skipped set) deleted and its siblings lowered (caller location → FU-R3);
**owners USER DECIDED 2026-10-24 (via PM)**: P1 the L3 DOM/form lane, P2 the Layout lane (§8 triggers); C12, C24, C26
widened to their invariants; C29's greps; CF-2 CV-details' own (C25, CF-R1); P1-R4 a cross-reference; P2-2 generic (C31).

**Earlier revisions.** Rev 7 (PM approved): sibling contracts restated as observable invariants with cells, every sibling
"how" withdrawn to §5.3's named risks (R3–R5). Rev 6 folded R4 (the external edge **A2c → CV-focus**; K16's marker **CV-readers'**; DOM §2.9's
*activationTarget* steps renumbered). Rev 5 folded R3 (IMP 19 → 20 → 17 on the same mechanisms: a mis-drawn slice,
`feedback_defer-accumulation-signals-mis-drawn-slice`) — **re-slice (PM decided, 2026-10-24)**: P1 and P2 (ex
CV-flat-tree) left as **sibling programs** like FU; R2's H2, H3, H4's `summary` member, H5's marker and D12 (b)'s VM-entry
call site superseded. Rev 4 folded R2 (H1–H8). Rev 3 folded R1 (F1–F30). Rev 2: U1, U2, O1, O2, PE6. Rev 1: first draft.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` is an **`a9866498`** coordinate; `git diff --stat e2d62b9e a9866498 -- crates` is empty, so the parent's
`e2d62b9e` coordinates still hold. origin/main at rev 5 (`3fb34355`: E1 #537 `cc05381e`, E2 #538 `811f523e`, #539, #540)
differs from `a9866498` in 9 crate files (`git diff --stat a9866498 origin/main -- crates | tail -1` = "9 files changed",
verified 2026-10-24 for rev 5); of those, this memo cites `elidex-ecs/src/dom/attribute.rs` and `attribute_reconcile.rs`
(each diff a one-line doc-comment edit at `:3`; cited lines unchanged) and `dom/mod.rs` — B18's `create_element*` at
`:372`, `:382` (origin/main `:334`, `:344`), its dispatcher lines `:193-206` (origin/main `:194-207`) and its line count (1075 at `a9866498`, 929 at
origin/main; E1 moved the node-classification queries into `elidex-ecs/src/dom/classify.rs`). Line counts in §9's watch
list are origin/main measurements (`git show origin/main:<f> | wc -l`). Spec text is `.claude/tools/webref body` output
(2026-10-20, re-run 2026-10-24 for revs 3–5); every `§X.Y = name` pair is `webref heading`/`dfn` output. `grep` =
`/usr/bin/grep`; `git grep` for tree-wide searches. Memory paths are under
`~/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory/`; live ledgers (the open-slot ledger, the roadmap) are cited
by slot or row name, not line.

### §0.2 Premises re-measured

| # | Claim (item 11 / brief) | Command | Result |
|---|---|---|---|
| B1 | elidex has no `content-visibility` | `git grep -n -i -E 'content.visibility\|ContentVisibility' -- crates` | holds: 0 hits |
| B2 | applies-to = size containment's exclusions | `webref css css-contain-2 content-visibility`; `webref body css-contain-2 containment-size` | holds: "elements for which size containment can apply"; four bullets (no principal box; inner `table`; internal table box; internal ruby box **or a non-atomic inline-level box**) |
| B3 | "non-atomic inline-level box" = F1 | `webref body css-display-3 atomic-inline` | **partial**: an atomic inline "cannot split across lines (as inline boxes **and ruby containers** can)"; elidex has no `ruby` `Display` variant (`sed -n 5,29p crates/core/elidex-plugin/src/computed_style/display.rs`), so F1 alone is exact at HEAD (PE2) |
| B4 | the UA rule `[hidden=until-found i]:not(embed) { content-visibility: hidden; }` (§15.3.1) | `webref body html hidden-elements` | holds verbatim; the section's other `[hidden]` rules are `[hidden]:not([hidden=until-found i]):not(embed)` and `embed[hidden]` |
| B5 | skipped contents read by paint, hit testing, a11y, focus, selection; computed `visibility` untouched | `webref body css-contain-2 content-visibility` | holds ("not painted (as if they had visibility: hidden)", "do not respond to hit-testing (as if they had pointer-events: none)"; `hidden`: "must not be accessible to user-agent features … nor be selectable or focusable"; §4.6). **Incomplete**: §4.5 names IntersectionObserver (1), ResizeObserver (2), scrolling operations (5), `innerText` (8), animations/transitions (9–10), top layer (11), media audio (12); CSSOM View §6's `scrollIntoView` step 7 and `checkVisibility` step 2 read it (Q1 → U1) |
| B6 | "contents" | same | "the **flat tree** descendants of the element, including both text and elements, **or the replaced content of a replaced element**" — the element's own box is not contents |
| B7 | `details`' UA shadow tree (HTML §15.5.5) | `webref body html the-details-and-summary-elements` | **incomplete**: **three** children — a slot taking the first `summary` child, holding a *default summary* (fallback) "expected to allow the user to request the details be shown or hidden"; a second slot taking the rest, styled `display: block; content-visibility: hidden;` when `open` is absent ("not directly visible to author code. Its impacts, however, are visible"), expected to match `::details-content` (css-pseudo-4 §5.2); a style child for `:host summary` (PE3, Q2 → O1) |
| B8 | elidex has shadow/slot infrastructure | `wc -l crates/core/elidex-ecs/src/dom/shadow.rs`; `git grep -n 'fn flat_tree_parent\|fn assigned_slot' -- crates` | holds (550 lines; `attach_shadow_with_init` `:171`, `slot_assign` `:235`, `assigned_nodes` `:394`, `composed_children` `:529`); named distribution in the style walk (`walk.rs:459`, `:582`, `slot.rs:21`); **no flat-tree parent primitive** (0 hits) — P2's (D8) |
| B9 | "the first UA shadow tree" | `git grep -n -i 'user.agent.shadow\|UaShadow' -- crates` | holds: 0 hits |
| B10 | ancestor revealing on fragment navigation — "elidex has it, `app/navigation.rs:13`" | `sed -n 9,25p crates/shell/elidex-shell/src/app/navigation.rs`; `git grep -n -i 'beforematch\|reveal' -- crates` | **partial (PE1)**: `:13` is the app-mode `AppSameDocStep::FragmentNav` variant; same-document fragment navigation exists in **two** shells (`app/navigation.rs:199`, `content/navigation.rs:462`, both calling `content/scroll.rs:210`); the **ancestor revealing algorithm does not exist** (only the handler name `onbeforematch`, `event_handler_consumer.rs:66`) |
| B11 | HEAD's focus predicate reads `hidden=until-found` | `sed -n 190,230p;300,320p crates/dom/elidex-dom-api/src/focus/predicate.rs` | `is_focusable` (`:205`) excludes self-or-ancestor `hidden` through `is_in_hidden_subtree` (`:303-317`) — one `has_attribute(c, "hidden")` test; right for an until-found element's descendants, **wrong for the element itself** (it is being rendered, HTML §15.1). Pinned by `focus/tests.rs:451-456`, the doc at `predicate.rs:197-198` |
| B12 | `details` toggle / exclusivity | `sed -n 97,140p crates/script/elidex-js/src/vm/host/html_details_proto.rs` | live only in the VM IDL setter `details_set_open` (`:97`): `setAttribute('open', '')` and the parser fire no `toggle` and close no group member (A96; a Layering-mandate placement) |
| B13 | `summary` activation (HTML §4.11.2) | `git grep -n -i summary -- crates/shell` | absent. Activation of any kind (checkbox, radio, label) lives in the content shell only (`content/event_handlers.rs:115-168` over `content/form_input.rs`); `app/events.rs:17` `handle_click` has none |
| B14 | paint and hit test share one participant walk | `grep -n collect_sc_participants crates/core/elidex-render/src/builder/walk.rs crates/layout/elidex-layout/src/hit_test.rs` | `walk.rs:554`, `hit_test.rs:208`, both `paint_order.rs:54`; hit test reads neither `visibility` nor `pointer-events` (`git grep -n -i pointer.events -- crates` = 0) |
| B15 | a11y walks the composed tree | `grep -n composed_children crates/dom/elidex-a11y/src/tree.rs` | `:49`, `:133` |
| B16 | find-in-page, top layer, `innerText`, `checkVisibility` | `git grep -n -i -E 'find.in.page\|top_layer\|TopLayer\|innerText\|checkVisibility' -- crates` | none exist |
| B17 | (P2 context) slot assignment at HEAD | `sed -n 21,86p crates/css/elidex-style/src/slot.rs`; `sed -n 235,330p crates/core/elidex-ecs/src/dom/shadow.rs`; `git grep -n SlottedMarker -- crates` | two writers of `SlotAssignment`: `distribute_slots` (style walk; always named mode, so it overwrites a manual assignment) and `slot_assign` (manual; never writes `SlottedMarker`); `assigned_nodes` (`shadow.rs:394`) recomputes named mode live (`:409-440`); `distribute_slots`' light-child loop (`slot.rs:59-61`) has no `PseudoElementMarker` filter (a live bug). `SlottedMarker` (`components.rs:335`) is read by `::slotted` matching (`elidex-css/src/selector/matching.rs:435`) and by `apply_retarget`'s slotted exemption (`event_dispatch/mod.rs:635`); no node→slot back-link |
| B18 | (rev 5; P1 context) element-step and activation seams | `sed -n 290,316p crates/script/elidex-js/src/vm/vm_api.rs`; `sed -n 28,36p …/vm/lifecycle.rs`; `sed -n 375,441p crates/script/elidex-js/src/engine.rs`; `sed -n 186,206p crates/core/elidex-ecs/src/dom/mod.rs` | the VM's `ConsumerDispatcher` is installed at `Vm::bind` (`vm_api.rs:313-314`, after `initialize_consumers`) and removed at unbind (`lifecycle.rs:34`); binding is per batch bracket (`engine.rs:375-441`, `with_bound` `:425-443`), so shell writers outside a bracket (`content/event_handlers.rs:115-168`) and documents with no VM reach no consumer; `EcsDom::dispatch_event` no-ops without a dispatcher (`dom/mod.rs:206`) and forbids mutation from inside dispatch (`:193-205`, a `debug_assert`). The `elidex-ecs` chokepoint (`attribute_reconcile.rs:50`, called `attribute.rs:227`, `:308`) is for core invariants only (its doc, `:25-33`); parser `create_element*` (`dom/mod.rs:372`, `:382`) enters none. Script `dispatchEvent` walks in the VM (`event_target_dispatch.rs:42`, `:272`; `event_target_dispatch_vm.rs:271`), apart from the engine-independent loop (`elidex-script-session/src/event_dispatch/mod.rs`); no DOM §2.9 activation step exists anywhere. HTML §4.11.1: "these attribute change and insertion steps also run when an attribute or element is inserted via the parser" |
| B19 | pseudo-element representation | `sed -n 38,82p crates/css/elidex-style/src/pseudo.rs`; `sed -n 190,200p crates/core/elidex-ecs/src/components.rs` | `::before`/`::after` are DOM child entities of the originating element (`pseudo.rs:63-80`; tree parent = originating element); `::marker` is `ListItemMarker` on the `li` (`components.rs:199`) |
| B20 | initial-load fragment scroll | `git grep -n 'scroll_offset_for_fragment\|\.fragment()' -- crates/shell` | **absent**: only the two same-document steps call it; HTML §7.4.6.2 *try to scroll to the fragment* is not implemented. An OOP iframe's navigation rebuilds its pipeline (`content/iframe/thread.rs:199` `handle_navigate`) |
| B21 | script-time style/layout freshness | the open-slot ledger, slot `#11-forced-style-layout-flush-on-script-read` | registered at #388 as "a general engine concern" (*Trigger* "a handler-stale-geometry test OR a forced-flush program"); no forced flush exists; the PM re-pointed it 2026-10-24 (home = the FU umbrella; owner the Layout lane) |
| B22 | shadow selector matching | `sed -n 25,125p crates/css/elidex-css/src/selector/matching.rs` | `:host` / `:host()` match only as the **subject** compound (`:71-74`, `:116-120`): the descendant and child combinators stop at a `ShadowRoot` (`:33-35`, `:48-50`), so `:host X` never matches; `:not` inside `:host()` reaches `_ => false` (`:122`, via `:348-349`); no `:host-context()` (0 hits). For **every** shadow sheet. `::slotted()` matches through `SlottedMarker` (`:76-80`) |
| B23 | (rev 5 corrected) the focusing steps | `sed -n 269,312p crates/script/elidex-js/src/vm/host/html_element_proto.rs`; `sed -n 104,114p crates/dom/elidex-dom-api/src/focus/update_steps.rs`; `sed -n 178,186p …/focus/sot.rs` | the focusing steps **are** in `elidex-dom-api`: `focusing_steps` (`update_steps.rs:106`), which reads `is_focusable_area` (`:113`), called by the shell (`content/focus.rs:85`). The VM host `native_html_element_focus` (`html_element_proto.rs:269`) bypasses it, calling `is_focusable` (`:302`); making the VM a thin caller is canonical-focus A2c (slot `#11-vm-host-synthetic-dom-event-dispatch`). `reconcile_focus` (`sot.rs:178`), run at render time, calls `is_focusable` (`:185`). The VM's Hidden classification is `HiddenKind` (`html_element_proto.rs:679`) |
| B24 | the frame style pass's animation phases | `sed -n 632,700p crates/shell/elidex-shell/src/lib.rs` | inside `re_render` (`:559`): old-style snapshot (`:636-640`), style resolution (`:681-688`), transition detection, CSS-animation sync and `apply_active_animations` (`:690-700`); the session flush is `SessionCore::flush` (`elidex-script-session/src/session.rs:114`; `SessionCore` is CLAUDE.md's ScriptSession) |
| B25 | (rev 5 corrected; P2 context) `slotchange` | `sed -n 10,32p;58,65p;389,410p crates/script/elidex-js/src/vm/host/html_slot_proto.rs`; `sed -n 386,389p crates/core/elidex-ecs/src/dom/shadow.rs` | fires only on explicit `slot.assign()`; HEAD **has** an ordered VM queue, `pending_slot_change_signals` (`signal_slot_change` `:61-63`, drained `:391-408` into the NotifyMutationObservers microtask); named-mode detection is deferred to `#11-shadow-slotchange-named-mode-detection` (`:25-28`, cited in code, not in the ledger); `assignedNodes({flatten: true})` to `#11-shadow-slot-flatten` (`:13`, `:30`; `shadow.rs:389`; unledgered, recorded by the PM) |
| B26 | K5's pseudo-element outcome | Chrome 156.0.8078.48 headless, 2026-10-24 (driver probe; screenshot) | a `content-visibility: hidden` `div` with `::before`/`::after` paints neither; an `li` (marker outside and inside) paints no marker; each element's own border paints; the controls paint all. One engine: evidence, not spec ground |

### §0.3 Premise errors (reported, not edited — the A114 idiom)

- **PE1** (item 11's "elidex has it, `navigation.rs:13`"): B10 — same-document fragment navigation exists in both shells;
  the ancestor revealing algorithm, `beforematch` and the initial-load path (B20) do not. CV-reveal **builds** them; the
  decided content is unchanged.
- **PE2** (item 11's applies-to "non-atomic inline-level box … consuming F1"): B3 — exact at HEAD, F1 plus a ruby-container
  arm once a `ruby` display lands; K3 makes that fail-safe.
- **PE3** (item 11's "`details`' UA shadow tree"): B7 — three children; the second slot carries `::details-content`, a
  selector surface item 11 does not name (Q2 → O1).
- **PE4** (item 11's reader list): B5 — further readers exist (Q1); resolved by U1.
- **PE5** (the parent's pins, "flip = the content-visibility PR that wires skipped contents into hit testing"): every
  witness sits inside `<div hidden="until-found">`, so it flips only once CV-core **and** CV-render have landed; §5.2 puts
  CV-core first, so the flip event is CV-render, as the pins name it.
- **PE6** (U2's slot list): `#11-shadow-composed-event-retargeting` is not open — the 2026-06-14 sweep found it implemented
  (`html-side-defer-sweep-2026-06-14.md:51`; `m4-12-defer-sweep-2026-06-19.md:38`; the open-slot ledger's close list), and
  `apply_retarget` is at `event_dispatch/mod.rs:627`. The roadmap rows that still list it as open are stale. The other
  three U2 slots are roadmap rows, not open-slot ledger entries.

### §0.4 Decisions

**D1 — Two facts, two compositions, no stored copy.** *Skips its contents* (css-contain-2 §4 `#skips-its-contents`) is
S1(e) = computed `content-visibility` of e is `hidden` ∧ e is an element for which size containment can apply (K3). *In
skipped contents* is S2(n) = some **flat-tree ancestor** of n satisfies S1 (K4). Both are live queries over components that
already hold the inputs (`ComputedStyle`, F1, the tree) — the parent's rule: "One representation per spec fact; one
composition per spec predicate; each representation's writer set closed" (parent `:1167`), components only where "no
other component holds those facts" (`:1197`). No marker copying the **current** skipped state is written (a second copy
of the cascade's output); K16's marker is a historical fact (the value at the previous style change event), not a copy.
Writers: none (query), as F1. `auto` adds S1's second arm later (`#11-content-visibility-auto`). K1 adds the further
queries the spec's wording distinguishes.

**D2 — Applies-to is used-time, inside S1 only.** The computed value is "as specified" (B2), so CSSOM reads `hidden` on an
element the property does not apply to; only S1 consults applies-to. Computed `visibility` and `pointer-events` are never
written (B5).

**D3 — Readers read S2 on the entity that owns the item** (a painted display item, a hit candidate, an a11y node, a focus
candidate), and S1 on an element for content it owns without a child entity: its replaced / widget content and its
`::marker` (K5). A reader may prune a subtree at an S1 element only where its traversal **is** the flat tree (K6).

**D4 — Paint and hit testing are wired in one nested PR** (CV-render): the spec states them together, they share one
participant walk (B14), and no landing state shows skipped contents visible-but-unhittable or invisible-but-hittable (K7).

**D5 — Reveal before skip** (the parent's cell test, §0.6 item 7, applied inside this program). After CV-core and CV-render,
`<div hidden="until-found"><p id="t">x</p></div>` navigated to `#t` paints nothing where HEAD paints `x` and the spec (reveal,
then scroll) paints `x` — so CV-reveal precedes CV-render (§5.2 c2). The same cell fires, **conditionally**, for a
document **loaded** with `#t`: HEAD paints `x` (B20); the spec's *try to scroll to the fragment* (HTML §7.4.6.2, called
from *update document for history step application* step 8.3) reaches *scroll to the fragment* and its reveal step unless
its queued task aborts (step 2.1: no parser, or the parser has stopped) — so CV-reveal builds the initial-load path too,
and which load timings reach the reveal is its input. Every document load path, iframe documents included, reaches the
one *try to scroll* function. Likewise a closed `details` whose contents become skipped needs `summary` activation (B13),
the default summary's activation (K13) and the reveal algorithm's `details` arm; all are built by CV-details **before or
with** the rule that styles the second slot `content-visibility: hidden` (K9). These are bounded repairs the program's own
PRs make necessary — by the parent's ownership rule, not a widening of item 11 (O2).

**D6 — Contract on P1 (element lifecycle hooks), as observable invariants.** Not a nested PR: the sibling program **P1**
(planned umbrella `docs/plans/2026-10-element-lifecycle-hooks.md`, own plan-review; slot `#11-element-lifecycle-hooks`,
NEW, registered at the approval PR; owner the **L3 DOM/form lane**, USER DECIDED 2026-10-24 (via PM), starting when L3
returns to DOM/form after #527/#501 land). Edges **P1 → CV-reveal,
CV-details**. P1 owns how element steps and DOM §2.9's activation step run; this program registers behaviours and builds
no step-running or activation path of its own (*One issue, one way*). Invariants (cells, §7):
- **P1-1 every writer runs the element's steps** (K10) — attribute change, insertion and removing steps run for VM IDL
  setters, `setAttribute`, the parser (a parsed `<details open>` queues `toggle`; HTML §4.11.1, B18), shell-side writers
  outside a VM bind bracket (the reveal), activation, and a document with no VM (C12, C24);
- **P1-2 a step may mutate and queue** (K10) — exclusivity closes another group member; `toggle` is queued once, with its
  tracker (C12, C24);
- **P1-3 every writer's mutation is observable** (K10) — MutationObserver sees it as it sees a script's, in one record
  stream (CLAUDE.md: ScriptSession the sole Script↔ECS boundary) (C24);
- **P1-4 activation in every dispatch path** (K13) — DOM §2.9's activation step (step 6.5 selects *activationTarget*,
  ancestors 6.9.6.1 / 6.9.8.2; step 12.1 runs it) for a shell click, a script `dispatchEvent` click and keyboard
  activation, on a UA shadow tree's element too (the default summary) (C12).
The MO slots `#11-tree-mutation-record-pipeline`, `#11-method-driven-attribute-records`,
`#11-ce-reaction-mutation-observer-ordering` overlap P1-3 and stay open; whether P1 absorbs any is decided by P1's
plan-review alone (PM ledger, 2026-10-24). How P1 meets P1-1–4 is P1's (§5.3's P1 risks as input).
What this program supplies on P1's mechanisms (CV-details, CV-reveal): `details`' attribute change and insertion steps
(HTML §4.11.1: the toggle task with its tracker; *ensure details exclusivity*), `summary`'s activation behavior (§4.11.2),
the default summary's UA activation behavior and keyboard subwidget (K13), and the reveal's attribute writes — as
`elidex-dom-api` functions; the VM setter becomes marshalling (Layering mandate) (K10).

**D7 — The UA shadow tree is a declared creation fact of the element kind.** `HtmlElementHandler`
(`elidex-plugin/src/traits.rs:214`, connected by IBP-classify, parent §1 Home grounds; `elidex-plugin` has no `elidex-ecs`
dependency) **declares** a UA shadow template — data: the tree's shape and its UA shadow sheet — and builds nothing. One
engine-independent instantiation function instantiates it on every creation path (both parsers, `createElement`, fragment
parsing, clone — which re-creates from the template, never copies); its crate and how it reaches every path are CV-details'
input (Q-s10). One mechanism for every later UA shadow tree (form controls, media controls). The shadow root carries a
`UserAgentShadowRoot` (NEW) marker; DOM's `ShadowRootMode` stays the spec's two values. **Never exposed** is one
engine-independent query, "script-visible shadow root" (NEW; a `UserAgentShadowRoot` root ⇒ none), read by every exposure
site — the VM `shadowRoot` getter (`native_element_get_shadow_root`, `element_shadow.rs:86`, mode test `:112-126`),
`composedPath`, serialization, the slot accessors — never per-site checks (K12). The second slot's `content-visibility` is
**not** a stored inline declaration: a **positive pair** of UA shadow-sheet rules derives it by cascade — a default rule
giving the second slot `display: block; content-visibility: hidden` and a `:host([open])` rule giving `visible` — so no
`:not` inside `:host()` is needed (D1: no stored copy). This pair and §15.5.5's third child (`:host summary`) need `:host`
as a **non-subject** compound (css-shadow-1 §3.2.2–§3.2.3), which HEAD lacks for every shadow sheet (B22): CV-details builds
host-as-ancestor matching **once**, in the shared combinator walk for all origins, closing that author facet too (§8).
HTML words it "its style attribute set to"; the UA tree is not script-observable and the spec says that attribute is "not
directly visible to author code", so the cascade-derived value has the same impact — equivalence, not a deviation.

**D8 — `flat_tree_parent` is P2's primitive** — one builder (P2), read by S2 and by the reveal algorithm's walk ("parent
node within the flat tree", HTML §6.1 step 3); its cases are K4's (P2-1).

**D9 — E20 and E22 bind every nested PR** (item 11's literal: "it follows its inputs, `IBP-predicate` (E20) and
`IBP-css-machinery` (E22)", parent `:997-998`, and the home slot's ledger entry) — the literal over the node paragraph's
general rule (`:1381-1382`), as IBP-sandbox's D7 chose for E26 (`ibp-sandbox.md:180`). Data flow, for information: E22's `i`
flag is consumed by CV-core's UA rule; E20's F1 by CV-render's S1. The sibling programs are not nested PRs; D9 does not
bind them.

**D10 — Selection** is read by every user-agent-initiated selection producer (drag, multi-click, select-all). HEAD has none
for document content (Q-s5: the shells' selection code is form-control text editing only,
`content/event_handlers.rs:535-544`; HEAD's `Selection` is script-driven, `elidex-dom-api/src/selection.rs`, and a script
`Range` over skipped nodes is legal). The obligation (producers read S2) is recorded on
`#11-content-visibility-absent-readers` (K17, §8); a form control inside skipped contents cannot be focused (K15).

**D11 — Contract on P2 (shadow slot assignment and flat tree), as observable invariants.** Not a nested PR: the sibling
program **P2** (ex CV-flat-tree; planned umbrella `docs/plans/2026-10-shadow-slot-assignment.md`, own plan-review; slot
`#11-shadow-slot-assignment-flat-tree`, NEW, registered at the approval PR; owner the **Layout lane**, USER DECIDED
2026-10-24 (via PM)). Edges **P2 → CV-reveal** (the reveal walk), **CV-render** (S2), **CV-details**. P2 owns slot assignment (one
writer) and the flat tree of css-shadow-1 §4.1 (*find slottables*); this program reads it. Invariants (cells, §7):
- **P2-1 K4's cases, current** (K4, K5, K11) — pseudo-element entities never slottables, their flat-tree parent the
  originating element; current with no style pass after DOM *insert*, *remove*, *move*, a slot's `name` or a slottable's
  `slot` change, `HTMLSlotElement.assign()`, entity destroy, and `moveBefore` when built (C26);
- **P2-2 a UA element kind's assignment, current** (K13) — any declared UA element-kind assignment rule is current after
  every child-list change, with no style pass between and no VM bound (C31, a test rule); `details`' rule (HTML §15.5.5;
  first `summary` child → slot 1, none → the default summary; every other child node → slot 2; CV-details' fact) is its
  acceptance case (C27);
- **P2-3 one relation** (K5, K6, D3) — `flat_tree_parent` and `composed_children` describe one flat tree, a host's
  `::before`/`::after` included (C26);
- **P2-4 S2's home can read it** (K1, K4) — S2's readers query the flat tree from `elidex-ecs`, not `elidex-dom-api` (C28).
`slotchange`, manual vs computed assignment, the *assigned slot*'s clearing and the event path's *get the parent* /
`composedPath` are P2's own surfaces; how P2 meets P2-1–4 is P2's (§5.3's P2 risks as input); rev 4's C17 is P2's.
P2 absorbs `#11-shadow-slotchange-named-mode-detection`, `#11-shadow-slot-flatten` and the assignment-writer part of
`#11-shadow-light-tree-distribution-rendering` (§8); the code-cited slot names (`html_slot_proto.rs:13`, `:28`, `:30`;
`shadow.rs:389`) are retargeted at P2's landing (P2's record).

**D12 — Contract on FU (forced style/layout update), as observable invariants** (rev 4 H1, PM decided 2026-10-24). Not a
nested PR: the sibling program **FU** (planned umbrella `docs/plans/2026-10-forced-style-layout-update.md`, own
plan-review; slot `#11-forced-style-layout-flush-on-script-read`, B21, id kept — not an item-11 amendment; owner the
Layout lane; other consumers, e.g. `getBoundingClientRect`, FU's to register). Edges **FU → CV-reveal, CV-focus,
CV-readers**. FU owns the one style-phase system (the frame's style pass with its animation phases, B24), invoked frame
and forced, and the script-time update point; this program calls the point and owns K16's marker (CV-readers). Spec
semantics decided here: every invocation, frame or forced, is one **style change event** (css-transitions-1 §3,
`#style-change-event`, leaves its timing to the implementation: elidex's decision). Invariants (cells, §7):
(FU-1 and FU-2 are withdrawn by rev 8: their outcomes are K16's, checked by CV-readers' own cells C16, C20, C22; FU's
part is FU-6.)
- **FU-3 one computed style** (K3, K16) — forced and frame invocations produce identical computed style for any property,
  animated or not (C21, its CV-render acceptance cell: S1 agrees under an animated `content-visibility`);
- **FU-4 script-time reads are fresh** (K8, K11, K15, K16) — style **and layout** reflect the task's earlier mutations:
  `focus()`, the reveal's scroll offset after a `beforematch` mutation (C18), `getComputedStyle`'s sampling (C23);
- **FU-5 reachable from engine-independent callers, never from a predicate** (K11, K15) — from `elidex-dom-api` (the
  reveal, `focusing_steps`' path) and the VM, which cannot depend on a shell; no focusability predicate and not
  `reconcile_focus` (it would recurse, B23) (C29);
- **FU-6 consumer state carries across invocations** (K16) — a consumer's per-entity state written at the end of
  style-phase invocation N−1 (over N−1's after-change styles) is visible to invocation N's transition-start decision, for
  frame and forced invocations alike, two forced invocations in one task being two (C30, a test consumer).
The reveal's scroll (K11), `focus()` (K15) and `getComputedStyle` (K16 item 9) are **decided** consumers and the slot's
own subject: FU discharges the slot. How FU meets FU-3–6 is FU's (§5.3's FU risks as input). Rejected (rev 3, standing):
deriving until-found from the attribute (a second derivation, K1); a stale read plus a render-time blur (a deviation).

**CF — canonical-focus** (the canonical-focus lane, driver-less). **CF-1** (an external gate; K8, K15; A2c,
`#11-vm-host-synthetic-dom-event-dispatch`) — the VM's `focus()` runs `focusing_steps` (B23), reaching FU's point and the
predicates CV-focus wires (C18, through the VM). **CF-2** (K13, K15) — **CV-details' own invariant**, no edge: the
default summary's keyboard subwidget is a focusable area that sequential navigation reaches and the render-time fixup does
not blur (C25); coordination with A3's unified predicate is the named risk CF-R1.

### §0.5 Decisions on rev 1's open questions

- **U1 — USER DECIDED 2026-10-20 (Q1 option (a), widen)** — **amendment of the parent's §0.6 item 11 decided content**
  (recorded, not edited; the A114 idiom; ratified by the approval PR). Item 11's reader list ("paint, hit testing,
  accessibility, focus and selection") becomes: **every skipped-contents reader elidex has reads the one query (K1, K16)**
  ("the one query" is K1's set — S1/S2 and C, in one home, with §2's reader-arm table). Added readers present at
  `a9866498`: scrolling operations (§4.5 item 5; CSSOM View §6 `scrollIntoView` step 7) — the `ScrollIntoView` handler
  (`elidex-dom-api/src/element/layout_query.rs:257`) and the fragment scroll (`content/scroll.rs:210`); IntersectionObserver
  (item 1) — `elidex-api-observers/src/intersection/mod.rs`, VM-wired (`vm/host/intersection_observer.rs`); ResizeObserver
  (item 2) — `elidex-api-observers/src/resize.rs`, VM-wired (`vm/host/resize_observer.rs`), also moved by the parent's F13
  (IBP-observer → CV-readers); CSS animations and transitions (items 9–10) — `elidex-css-anim` (`engine.rs:147` `tick`,
  `:91` `add_transition`) and the shell's detection (`animation.rs:87`). They are wired by the nested PR **CV-readers**.
  Readers elidex lacks — `innerText`, `checkVisibility`, the top layer, media audio, find-in-page and user-agent selection
  producers (D10) — carry a **standing obligation** (K17), recorded on §8's slots. Commands: `/usr/bin/grep -rln
  'scrollIntoView\|scroll_into_view' crates --include='*.rs'`; `/usr/bin/grep -rln
  'IntersectionObserverRegistry\|ResizeObserverRegistry' crates --include='*.rs'`; B16 for the absent ones.
- **U2 — USER DECIDED 2026-10-20 (Q3)** — CV-details **builds** the shadow surfaces it needs as their first production
  consumer; the non-stale parts move into this program (**the Layout lane owns them from now**), each slot closed or
  narrowed at its owning landing (§8's per-slot table; rev 5: the assignment writer is P2's, D11). Which parts are
  non-stale is **Q-s2**, answered by CV-details' plan-review (PE6 settles `#11-shadow-composed-event-retargeting`: closed).
  Recorded as an amendment of item 11, ratified by the approval PR.
- **O1 — decision (lens converged)**: `::details-content` → slot `#11-details-content-pseudo` (NEW; §8): selector machinery
  (a pseudo-element mapped onto a UA shadow element, like `::part`) that elidex lacks, not part of skipping; A96.
- **O2 — decision (lens converged; rev 3)**: D5 (summary activation, the default summary, the `details` steps and the
  initial-load fragment path inside the program, by the parent's cell test) stands — rev 5: their *mechanisms* are P1's
  (D6), the behaviours this program's. D9 follows item 11's literal (R1 F15).

## §1. Program shape and verdicts

| Axis | Own invariant | Intersects |
|---|---|---|
| CSS property | grammar, cascade, computed as specified, discrete animation with the `hidden` mapping (§4.1), UA rule with `i` | applies-to, CSSOM |
| Used-time skip | S1 over the principal box via F1; S2 over the flat tree; C; each reader's arm classified (K1) | every reader; containment out |
| Flat tree; element lifecycle; script-time freshness | consumed from P2 (D11), P1 (D6), FU (D12) as invariants, not built here | S2, the reveal walk and writes, K13's assignment, `details`' steps and activation, `focus()`, the reveal scroll, `getComputedStyle` |
| Paint + hit test | contents unpainted / pass-through; owner-entity gate; out-of-flow descendants; pseudo-elements | stacking contexts, PR-1c's inline hits, IBP-layout's F4 sites, U2's slot/fallback emissions |
| A11y | omission (§4.6) | composed-tree walk |
| Focus | being rendered vs skipped; Hidden / Hidden Until Found split; render-time fixup | `focusing_steps`, `reconcile_focus`, the canonical-focus lane (driver-less) |
| Navigation + script | reveal fires `beforematch` (script) and mutates before the scroll; same-document and initial-load; two shells | P1, FU |
| UA shadow tree | declared template, one instantiation function, never script-observable, UA-styled by cascade; the shadow surfaces it needs (U2) | P2's flat tree, P1's activation, serialization, clone, the shadow-lane slots |
| Further readers (U1) | scrolling operations skip; observers see no intersection / no size change; animations frozen, transitions not started | IBP-observer (F13), `elidex-css-anim`'s tick, transition detection, both shells' scroll |

**Program: edge-dense — divided** (*decision*; CLAUDE.md *Edge-dense work* (a)). FU, P1 and P2 are sibling programs, not
counted here (each its own umbrella and plan-review). Per nested PR (*input*; each plan-review applies the rule to itself):
**CV-core** likely narrow (one property, one UA rule); **CV-reveal** may be edge-dense (script event × mutation × two
shells × same-document and initial-load paths); **CV-render** may be edge-dense (owner-entity gating × out-of-flow ×
replaced/widget content × pseudo-elements); **CV-a11y** narrow; **CV-focus** narrow-to-moderate; **CV-readers** may be
edge-dense (four reader families in four crates × the animation semantics; its plan may divide it per family);
**CV-details** is a **node** — likely edge-dense (UA shadow template and instantiation × fallback rendering × slotted
rendering × host-as-ancestor matching × the behaviours run by P1 × reveal arm × style) — its plan divides it (seeds:
UA-shadow mechanism; the U2 facets — fallback rendering and host-as-ancestor matching; behaviours + reveal arm; the slot
style rule last, K9).

## §2. Coupled invariants (decisions, with landing qualifiers)

- **K1 One home, the spec's distinctions** — the program provides exactly the queries the spec's wording distinguishes, in
  one file (§4): S1/S2, and **C(n)** = n or a flat-tree ancestor of n has computed `content-visibility: hidden` (no
  applies-to), its *ancestor form* C over `flat_tree_parent(n)`. C is built by its first consumer, in that file, never
  inline. S2 is built once; no hidden-arm alias is built before `auto` (it would be dead) — each reader's arm is
  classified **now** in the table below, which `#11-content-visibility-auto` consumes. No reader derives any of them from
  the `hidden` attribute or reads `content_visibility` directly. From **CV-render** (paint, hit), **CV-a11y**, **CV-focus**
  — which **adds** a state split to `is_in_hidden_subtree` (B11): the Hidden state stays the attribute test (a
  `display: none` proxy, IBP-ua-display's rule and `#11-focusable-area-being-rendered`), the Hidden Until Found state is
  read through S2, classified by the one Hidden Until Found classifier (§4); it flips `focus/tests.rs:451-456` and the doc
  at `predicate.rs:197-198`.

  | Reader | Arm | Spec ground | At HEAD |
  |---|---|---|---|
  | paint, hit testing | any skip | css-contain-2 §4 `#skips-its-contents` | present |
  | IntersectionObserver, ResizeObserver | any skip | §4.5 items 1, 2 | present |
  | animations, transitions | any skip | §4.5 items 9, 10 | present |
  | `innerText` | any skip | §4.5 item 8 | absent |
  | focus, selection producers, find-in-page | hidden arm only | §4 `hidden` ("must not be accessible to user-agent features … nor be selectable or focusable") vs `auto` ("must still be available as normal … focusable and selectable as normal") | focus present; others absent |
  | scrolling operations | hidden arm only | §4.5 item 5 ("not available to user-agent features"); CSSOM View §6 `scrollIntoView` step 7 | present |
  | accessibility | hidden arm; `auto` conditional | §4.6 (¶1 `hidden`: omitted; ¶2 `auto`: must not reveal a11y use through timing) | present |
  | top layer | C ancestor form | §4.5 item 11 and its Note (`auto` excluded); "ancestor" read as flat-tree ancestor (spec-silent; consistent with `checkVisibility` step 2) | absent |
  | media audio | self computed `hidden` or S2 (hidden arm); an iframe so placed extends it to its document's media | §4.5 item 12, its iframe clause and Note (`auto` excluded) | absent |
  | `checkVisibility` | C ancestor form | CSSOM View §6 step 2 | absent |
- **K2 Computed ≠ used** — from **CV-core**: CSSOM serializes the computed value whatever applies-to says; `visibility` and
  `pointer-events` untouched.
- **K3 Applies-to over the principal box** — from **CV-render**: S1 requires a principal box (`display` not `none` /
  `contents`), inner display not `table` (`table`, `inline-table`), not an internal table box (css-display-3 §2.4; caption
  excluded), not an internal ruby box, not a non-atomic inline-level box (F1; plus an inline ruby container, PE2). Matched
  exhaustively over `Display`, so a new variant is a compile error until classified (fail-safe). Blockified display is
  F1's (the parent's IBP-predicate moves blockification to style resolution, E20).
- **K4 Flat tree** — consumed from **P2's landing** (D8, D11): `flat_tree_parent` maps a slotted node to its slot, a shadow
  root's child to the host, a pseudo-element entity to its originating element, an unassigned light child of a host to
  *none*, a child of a slot whose assignment as the flat tree reads it (P2's: named, manual, or a UA element kind's rule
  such as `details`', P2-2) is non-empty to *none* (fallback is not in the flat tree), else the DOM parent; current after
  every trigger (P2-1), so whenever any reader runs. Text nodes are contents.
- **K5 What "contents" is** — the spec's *contents* is the flat-tree descendants plus the replaced content (B6);
  pseudo-elements are not flat-tree descendants, so whether they are contents is a spec-silent edge, **decided by
  measurement** (B26; `feedback_measure-browser-before-deciding-spec-silent-outcomes`): `::before`, `::after` and
  `::marker` (outside and inside) are **contents** (skipped), consistent with css-contain-2 §3.1's sizing sentence ("not
  even through pseudo-elements such as ::before, ::after, or ::marker"). Classified once, in S1/S2's definition:
  `::before`/`::after` are child entities (B19) whose flat-tree parent is the originating element (K4), hence S2;
  `::marker` (`ListItemMarker`, B19) through S1 (D3). Also contents: replaced content (F4, the parent's IBP-classify) and
  widget content (elidex paints controls without UA shadow trees, `elidex-render/src/builder/form.rs`). The element's own
  box (background, border, outline, its hit area) is not contents.
- **K6 Traversal independence** — a reader whose traversal is not the flat tree reads S2 per owning entity. Stacking-context
  participants, out-of-flow and positioned descendants (their containing block and stacking context lie outside the S1
  element while `#11-css-containment` is out) are skipped by flat-tree ancestry, not by where the walk meets them.
- **K7 Paint × hit test** — landed together at **CV-render**. Hit testing passes a skipped candidate over and continues to
  what lies beneath ("as if `pointer-events: none`"), so the S1 element's own box can be hit (every parent pin's conformant
  value). The gate sits where every hit candidate and display item passes, so a later producer (PR-1c's inline-box hits,
  umbrella `:1810-1811` as the parent cites it at `:1015-1018`; CV-details' slot and fallback emissions) cannot bypass it.
- **K8 Freshness** — render-time readers (paint, hit, a11y, `reconcile_focus`) read the last style pass, which they run
  after. Script-time readers (`focus()`, the reveal's post-mutation scroll offset, `getComputedStyle`'s animation sampling)
  read through FU's point (FU-4) — `focus()` once A2c has put the VM on `focusing_steps`' path (CF-1, §5.2) — so
  `setAttribute('hidden', 'until-found')` then `inner.focus()` in one task is refused, as at HEAD, and `details.open = true` then `inner.focus()` succeeds. A focused node that becomes skipped later is blurred by
  the render-time fixup (K15).
- **K9 Reveal before skip** — no landing state makes an until-found descendant unpainted while fragment navigation
  (same-document or initial-load) cannot reveal it (**c2**), nor a closed `details`' contents unpainted while `summary`
  activation, the default summary's activation or the reveal `details` arm is missing (CV-details' internal order: the
  second slot's `content-visibility: hidden` rule lands last).
- **K10 Writes run the element's steps** — from **CV-reveal** (`hidden` removal) and **CV-details** (`open`; `details`' steps
  run by P1, D6), so every writer reaches them, the parser and shell-side writers included (P1-1–P1-3). No writer calls a
  VM IDL setter.
- **K11 Reveal × script × geometry** — from **CV-reveal**: `beforematch` runs script; the algorithm's re-checks are kept —
  step 4.1 (connected), 4.2.1 (still until-found), 4.2.3 (connected after `beforematch`), 4.2.4 (still until-found after
  it); the scroll offset is computed after the reveal through FU's point, from layout that reflects any mutation (FU-4;
  both shells, one function; same-document and initial-load).
- **K12 UA shadow tree** — from **CV-details**: instantiated from the declared template at every creation of a `details`
  (both parsers, `createElement`, `cloneNode` — which re-creates it —, fragment parsing); `UserAgentShadowRoot` marks it;
  never exposed — every exposure site reads the one "script-visible shadow root" query (D7) — and styled by the UA shadow
  sheet only.
- **K13 `details` slot assignment and the default summary** — from **CV-details**, through P2's assignment: first `summary`
  child → slot 1 (none → the default summary, its fallback content), every other child node → slot 2; current after every
  child-list change, from every writer (P2-2). The second slot's style follows `open` by cascade (D7). §15.5.5's
  expectation ("The summary element that this slot represents is expected to allow the user to request the details be
  shown or hidden") has as subject the summary the first slot represents — the author's or the default. For the **default
  summary** the affordance is a UA-specific **activation behavior of the default summary itself**: DOM §2.9 step 6.5
  selects it as *activationTarget* (a host-level behavior would toggle on a click anywhere in the contents), plus a
  **keyboard** subwidget focusable area (HTML §6.6.2: "the part of a details element's rendering that enables the element
  to be opened or closed using keyboard input"; CF-2). Activation dispatch is P1's (P1-4), the behaviours CV-details'. Not
  §4.11.2, whose step 1 returns (its parent is the slot, not a `details`).
- **K14 Pins** — every parent pin (§5.1) flips at **CV-render**, conditional on firing at its exposer's actual base; CV-render
  flips each landed pin's assertion.
- **K15 Focus** — from **CV-focus**: a node in skipped contents is not a focusable area; the S1 element itself is (it is
  being rendered, HTML §15.1). At CV-focus's base (after A2c) `focusing_steps` — the VM's `focus()` included (CF-1) —
  reads `is_focusable_area` (`update_steps.rs:113`) and `reconcile_focus` reads `is_focusable` (B23); CV-focus wires the
  predicates they read (Q-s8). `focus()` reads through FU's point (FU-4, FU-5), so CV-focus lands **after A2c** (§5.2): at
  HEAD the VM entry calls `is_focusable` directly (B23) and would read a stale S2 — a regression against HEAD's attribute
  test. A focused node that becomes skipped is blurred at render time: HTML §8.1.7.3 *update the rendering* step 17 (the
  focused area is not a focusable area → the focusing steps for the viewport), HEAD's `reconcile_focus`.
- **K16 Every present reader reads K1's query** (U1) — from **CV-readers**: scrolling operations do not scroll to a node in
  skipped contents ("as if it did not have a layout box", §4.5 item 5); IntersectionObserver reports such a target as never
  intersecting, even when root and target are both skipped (item 1); ResizeObserver reports no size change for it and
  delivers on un-skip only if the size differs from the last reported (item 2). Animations, exactly per items 9–10: while
  an element is skipped no new animation is created, existing ones do not advance and running ones do not end; a script
  style query samples them (through FU's point, FU-4); on un-skip they are sampled and resume; no transition starts
  while skipped, nor from the un-skip style change event (C16, C22). *Skipped at the last style change* is a **historical**
  fact (not a stored copy, D1): a per-entity marker component **defined by CV-readers** (home `elidex-ecs`, beside the
  S1/S2 file), holding S2 as of the previous style change event; read by transition detection and un-skip handling. The
  boundary is the **style change event** (item 10; every invocation, frame or forced, D12), not the rendering update: two
  forced passes in one task are two, so the second starts a real transition (C20; FU's side is FU-6). Not a reader-local per-pass set: P2's
  tree is current at mutation time (P2-1), so a pre-pass S2 reads the new tree with old styles — moving `p` out of a
  hidden ancestor must not start a transition (C22). How the marker is written at every invocation is FU × CV-readers
  input (FU-R1). Freeze = the tick does not advance skipped entities (no stored freeze state). HEAD's callers (shell
  `animation.rs:87` `detect_and_start_transitions`, `lib.rs:692`; the tick, `content/event_loop.rs:256`) are input (FU-R3).
- **K17 Standing obligation for absent readers** (U1) — each, when built, reads K1's form per the arm table: `innerText` →
  S2; `checkVisibility` → C's ancestor form; the top layer → C's ancestor form; media → §8's media obligation;
  find-in-page → S2 plus the reveal; user-agent selection producers (D10) → S2. Recorded on §8's slots, not as code here.

Pairs: K1×K6 (one answer, read per owner); K2×K3 (applies-to never feeds the computed value); K3×E20 (F1's blockified
display at style time, so S1 agrees before and after layout); K4×K8 (P2's assignment is current at mutation time, so a
script-time reader of `flat_tree_parent` needs no style pass); K4×K11 (the reveal walk runs over a current assignment);
K4×K12 (UA slots are flat-tree parents; the reveal walk finds the `details` arm through them); K4×K13 (P2's assignment
decides `flat_tree_parent`; the `details` rule only feeds it); K5×F4/F7 (replaced and widget content are the parent's
classifications); K6×`#11-css-containment` (the ancestry rule still holds when containment lands); K6/K7×U2 (slotted nodes
painted at their slot and fallback content pass the same owner-entity gate — hence CV-render → CV-details); K7×PR-1c (new
candidates pass the gate); K8×K11, K8×K15, K8×K16 (the reveal's scroll, `focus()` and script sampling read through FU);
K10×P1/P2 (`details`' steps run by P1, P1-1; its assignment P2's, P2-2 — different owners); K12×K13 (the UA tree is
instantiated before the `details`' children are assigned — in parser order and in clone order); K9×K10 (activation and reveal write `open` through the
steps; the slot style follows by cascade); K10×B12 (the IDL setter becomes marshalling); K14×§5.2 (CV-core precedes
CV-render, so the hit-test reader is the flip, PE5); K16×F13 (ResizeObserver's observed box is IBP-observer's; CV-readers
gates after it); K16×FU-6 (the marker's boundary is every style change event, so forced and frame passes share one
baseline); K16×K11 (the fragment scroll is both a reveal caller and a scrolling operation: reveal first,
then the skip check — a target still skipped under an author `content-visibility: hidden` is not scrolled to); K17×K1 (C
lives in K1's file).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSS CONTAIN 2 §4 Suppressing An Element’s Contents Entirely: the content-visibility property | property definition | `visible`, `hidden`; `auto` → `#11-content-visibility-auto` | CV-core | ✗ | yes |
| CSS CONTAIN 2 §4 Suppressing An Element’s Contents Entirely: the content-visibility property | *skips its contents*: contents unpainted, not hit-tested | used `contain` → `#11-css-containment`; layout-skipping SHOULD → out (§8) | CV-render | ✗ | yes |
| CSS CONTAIN 2 §4 Suppressing An Element’s Contents Entirely: the content-visibility property | `hidden`: not accessible to UA features, not selectable, not focusable | focus: CV-focus; selection producers, find-in-page → K17's slots | CV-focus | ✗ | yes |
| CSS CONTAIN 2 §4.1 Animating and Interpolating content-visibility | discrete; `hidden` ↔ other maps (0,1) to the non-hidden value | | CV-core | ✓ | yes |
| CSS CONTAIN 2 §3.1 Size Containment | the four "no effect" bullets (applies-to) | ruby arm vacuous at HEAD (PE2) | CV-render | ✓ | yes |
| CSS CONTAIN 2 §3.1 Size Containment | "not even through pseudo-elements such as ::before, ::after, or ::marker" | the sizing rule only; K5 decided by measurement (B26), consistent with it | CV-render | ✗ | yes |
| CSS CONTAIN 2 §4.5 Restrictions and Clarifications | items 1–12 (each reader's arm: §2's table); item 10's style change event (K16's marker boundary) | 1, 2, 5, 9, 10: CV-readers (U1); 8, 11, 12 (incl. its iframe clause) → K17's slots; 3, 4, 6 `auto`-only (3 in effect: *update content relevancy for a document* step 2 iterates only `auto` elements) → `#11-content-visibility-auto`; 7 (iframe SHOULD) → out | CV-readers | ✗ | yes |
| CSS TRANSITIONS 1 §3 Starting of transitions | *style change event* (`#style-change-event`; timing not defined by the spec) | every style-phase invocation, frame and forced, is one — elidex's decision (D12, K16) | CV-readers | ✗ | yes |
| CSS CONTAIN 2 §4.6 Accessibility Implications | `hidden`'s skipped contents omitted | `auto` paragraph → `#11-content-visibility-auto` | CV-a11y | ✓ | yes |
| CSS DISPLAY 3 §2.4 Layout-Internal Display Types: the table-* and ruby-* keywords | *internal table box*, *internal ruby box* | | CV-render | ✓ | yes |
| CSS SHADOW 1 §4.1 Flattening the DOM into an Element Tree | the flat tree (*find slottables*) | consumed from P2 (P2-1, P2-3); S2 and the reveal walk read it | P2 (sibling) | ✗ | yes |
| CSS SHADOW 1 §3.2.2 Selecting Shadow Hosts from within a Shadow Tree | the host matched in shadow-tree context | host-as-ancestor (`:host summary`, the `:host([open])` pair) built once for all origins (D7) | CV-details | ✗ | yes |
| CSS SHADOW 1 §3.2.3 Selecting Into the Light: the :host, :host(), and :host-context() pseudo-classes | `:host`, `:host()` | `:host-context()`, `:not` inside `:host()` → `#11-shadow-tree-style-isolation` (§8) | CV-details | ✗ | yes |
| WHATWG HTML §15.3.1 Hidden elements | `[hidden=until-found i]:not(embed)` | `[hidden]:not([hidden=until-found i]):not(embed)` and `embed[hidden]` → IBP-ua-display / IBP-layout (parent) | CV-core | ✗ | yes |
| WHATWG HTML §6.1 The hidden attribute | *ancestor revealing algorithm* steps 1–4 | 3.1 / 4.2 (until-found): CV-reveal; 3.2 / 4.3 (`details`): CV-details | CV-reveal, CV-details | ✓ | yes |
| WHATWG HTML §7.4.6.4 Scrolling to a fragment | *scroll to the fragment* 3.4 (reveal) before 3.5 (scroll) | 3.6 / 3.7 focusing → `#11-fragment-navigation-focusing-step`, not this program | CV-reveal | ✗ | yes |
| WHATWG HTML §7.4.6.2 Updating the document | *try to scroll to the fragment* (*update document for history step application* step 8.3) | absent at HEAD (B20); built with its reveal (D5); step 2.1's abort makes the cell conditional; every load path incl. iframe documents | CV-reveal | ✓ | yes |
| WHATWG HTML §6.9.2 Interaction with details and hidden=until-found | find-in-page | → `#11-find-in-page` (B16) | — | ✗ | yes |
| WHATWG HTML §15.5.5 The details and summary elements | the internal shadow tree, three children; the second slot's style by `open` | cascade-derived (D7); `::details-content` → `#11-details-content-pseudo` (O1) | CV-details | ✗ | yes |
| WHATWG HTML §15.5.5 The details and summary elements | the default summary "is expected to allow the user to request the details be shown or hidden" | the default summary's own UA activation behavior plus keyboard subwidget (K13) | CV-details | ✓ | yes |
| WHATWG HTML §4.11.1 The details element | attribute change steps (toggle task + tracker; exclusivity); insertion steps (*ensure details exclusivity*); run for the parser | run by P1 for every writer (P1-1, P1-2) | CV-details | ✓ | yes |
| WHATWG HTML §4.11.2 The summary element | activation behavior steps 1–3 | step 1 excludes the default summary (K13) | CV-details | ✓ | yes |
| WHATWG DOM §2.9 Dispatching events | step 6.5 (*activationTarget*; ancestors 6.9.6.1, 6.9.8.2), step 12.1 (activation behavior) | P1's activation step in every dispatch path (P1-4); `summary` and default-summary behaviours registered on it | CV-details | ✗ | yes |
| WHATWG HTML §6.6.2 Data model | focusable-area criteria (being rendered); the `details` keyboard subwidget | the Hidden arm → IBP-ua-display / `#11-focusable-area-being-rendered`; the subwidget: K13 (CV-details) | CV-focus, CV-details | ✗ | yes |
| WHATWG HTML §8.1.7.3 Processing model | *update the rendering* step 17 (focused area not a focusable area → focusing steps for the viewport) | K15's render-time blur (`reconcile_focus`) | CV-focus | ✓ | no |
| WHATWG HTML §15.1 Introduction | *being rendered* | the S1 element itself is | CV-focus | ✓ | no |
| CSS PSEUDO 4 §5.2 Expandable contents of details element: the ::details-content pseudo-element | | → `#11-details-content-pseudo` (O1) | — | ✗ | yes |
| CSSOM VIEW 1 §6 Extensions to the Element Interface | `scrollIntoView` step 7 (*not available to user-agent features*) | | CV-readers | ✓ | yes |
| CSSOM VIEW 1 §6 Extensions to the Element Interface | `checkVisibility` step 2 | absent (B16) → `#11-content-visibility-absent-readers` (K17: C's ancestor form) | — | ✗ | yes |

**Breadth**: K=8 (css-contain-2, css-display-3, css-pseudo-4, css-shadow-1, css-transitions-1, cssom-view-1, dom, html), M=29 → split by
default (preflight SPLIT-DEFAULT); the split is §5 (seven nested PRs, CV-details a node) plus the sibling programs. The
CSS-module rows are `webref heading` output; preflight's §3 hard-gate does not map CSS labels
(`#11-preflight-css-module-labels`), so those rows were checked by hand. DOM §4.2.2–§4.2.3 and HTML §4.12.4 (slot
assignment) are P2's map, Node's *get the parent* (DOM §4.4 Interface Node) P2's (§5.3).

### §3.1 User-input touch audit

The `content-visibility` declaration (author CSS, new parse surface); the `hidden` attribute's value (case-insensitive
`until-found` through the `i` flag); the fragment of a navigated or loaded URL (selects the reveal target); `details`'
`open` and `name` attributes and its child list (assignment, through P2); a click or key activation on `summary` or the
default summary. New behaviour only hides or reveals rendering; no new network, storage or permission surface.
`beforematch` is a new script entry point fired by navigation: its re-checks (K11) bound what a handler can make the
algorithm do. Adjacent pre-existing: `details_set_open`'s synchronous `toggle` dispatch (B12) — exposure unchanged until
CV-details moves it to a queued task.

## §4. Facts → mechanisms (Home/kind lines decided; the rest input)

- **Computed value** (CV-core) — *Home/kind*: `ComputedStyle::content_visibility: ContentVisibility` (NEW; `Visible |
  Hidden`, `Auto` added by `#11-content-visibility-auto`) in `elidex-plugin/src/computed_style/`; parsed and cascaded by a new
  CSS plugin crate `elidex-css-contain` (NEW) — the css-contain-2 module's home, where `#11-css-containment`'s `contain` and
  `contain-intrinsic-size` later land (*Plugin-first*). Input: registration through the existing property-handler seam
  (precedent `elidex-css-multicol`); CSSOM serialization; §4.1's interpolation in `elidex-css-anim/src/interpolate.rs` beside
  `visibility`'s special case (`:24`).
- **UA rule** (CV-core) — *Home/kind*: the core UA sheet, `elidex-style/src/ua.rs` (436 lines), one rule. Input: consumes
  IBP-css-machinery's `i` flag (E22) and the existing simple `:not(embed)`.
- **Not this program's**: slot assignment and `flat_tree_parent` (P2, D8, D11; read here); element steps and activation
  dispatch (P1, D6; behaviours registered here); the script-time update point (FU, D12; called here).
- **S1, S2, C** (CV-render; C by its first consumer) — *Home/kind*: `EcsDom::skips_contents(element)` and
  `EcsDom::in_skipped_contents(node)` (NEW), with C, queries (no writer), all in one new file
  `elidex-ecs/src/dom/content_visibility.rs` (NEW): K3 over `ComputedStyle` and F1 (`EcsDom::is_inline_box`, the parent's
  IBP-predicate); S2 over `flat_tree_parent` (P2) and S1. Input: the internal-table classification is its own (css-display-3
  §2.4), not `Display`'s `is_table_internal` (`display.rs:35`, which serves `visibility: collapse`); bounded by
  `MAX_ANCESTOR_DEPTH`; per-frame memoisation only if a reader's plan measures the need, and then reader-local, dropped with
  its pass (never a component).
- **Ancestor revealing** (CV-reveal; `details` arm CV-details) — *Home/kind*: one engine-independent function,
  `elidex_dom_api::reveal::run_ancestor_revealing` (NEW), over `flat_tree_parent`, attribute mutation run through P1 (K10, P1-1)
  and an event-fire callback the caller supplies (the shells dispatch `beforematch` through the session boundary).
  Callers: both shells' same-document fragment navigation and the initial-load *try to scroll to the fragment* (NEW, both
  shells), before the scroll; later find-in-page. The **Hidden Until Found classifier** has one engine-independent home,
  built by CV-reveal and consumed by CV-focus, absorbing the VM's `HiddenKind` (B23) and `is_in_hidden_subtree`'s test
  (B11). Input: its crate; the callback's shape against the bind bracket (§5.3).
- **Paint reader** (CV-render) — *Home/kind*: the gate at the display-list builder's per-owner emission
  (`elidex-render/src/builder`), reading S2 for an item's owning entity and S1 for an element's replaced/widget content and
  `::marker` (D3, K5). Input: which of `walk.rs` / `inline.rs` / `inline_flow.rs` / `form.rs` emit per owner (Q-s9).
- **Hit-test reader** (CV-render) — *Home/kind*: the candidate test in `elidex-layout/src/hit_test.rs` (pass over, continue,
  K7). Input: whether the gate belongs in `collect_sc_participants` (shared with paint, `paint_order.rs:54`) or in each
  consumer — a participant list filtered for both readers would also drop the S1 element's skipped out-of-flow
  descendants from painting, which K6 requires anyway.
- **A11y reader** (CV-a11y) — *Home/kind*: `elidex-a11y/src/tree.rs`'s composed walk (`:49`, `:133`), omitting nodes with S2
  (pruning at S1 is valid: the walk is the composed tree, D3).
- **Focus reader** (CV-focus) — *Home/kind*: the focusability predicates (`is_focusable`, `focus/predicate.rs:205`;
  `is_focusable_area`, `delegate.rs:133`, read by `focusing_steps`) with `is_in_hidden_subtree` split by state, the Hidden
  Until Found state read through S2 (K1, K15); `focus()` through FU's point (FU-4, FU-5; CF-1), never from
  a predicate. Input: Q-s8; `focus/tests.rs` is 999 lines — a touch-time split precedes any added cell.
- **Selection** — D10 (K17's slot; no code).
- **`details` steps and activation behaviours** (CV-details) — *Home/kind*: `elidex-dom-api` functions (beside
  `element/details_exclusion.rs`) run by P1 (D6): the attribute change and insertion steps, the toggle event
  **queued** as an element task with the tracker (HTML §4.11.1) — the tracker a component on the `details` entity
  (`Send + Sync`; the side-store rule); `summary`'s activation behavior; the default summary's UA activation behavior; its
  keyboard subwidget (K13, CF-2; its representation input, §5.3). The VM keeps only dispatch marshalling.
- **Scrolling-operation reader** (CV-readers) — *Home/kind*: one `elidex-dom-api` "scroll a target into view" resolution,
  called by `ScrollIntoView` (`layout_query.rs:257`) and by the shell fragment scroll (`content/scroll.rs:210`), reading S2
  for the target (K16). Input: the other scroll-into-view callers (focus) its plan measures (Q-s12).
- **IntersectionObserver reader** (CV-readers) — *Home/kind*: the intersection computation in
  `elidex-api-observers/src/intersection/mod.rs` (`gather_observations` `:270`) treats a target with S2 as non-intersecting.
- **ResizeObserver reader** (CV-readers) — *Home/kind*: the observation-gathering step in `elidex-api-observers/src/resize.rs`
  (`gather_observations` `:235`) skips targets with S2 and keeps their last reported size (K16), over IBP-observer's F13.
- **Absent readers** (K17) — no code; the obligation lives on §8's slots.
- **UA shadow tree** (CV-details) — *Home/kind*: a template declared on `HtmlElementHandler` (data, D7); one engine-independent
  instantiation function (crate *input*, Q-s10); `UserAgentShadowRoot` (NEW) marker in `elidex-ecs/src/components.rs` (822
  lines); the "script-visible shadow root" query (NEW, engine-independent; crate *input*); the second slot's style a
  positive `:host([open])` pair of UA shadow-sheet rules, derived by cascade over host-as-ancestor matching built once in
  the shared combinator walk (`elidex-css/src/selector/matching.rs`, D7, K12).

## §5. Nested PRs

### §5.1 List and reader wiring (decided)

| Nested PR | Scope |
|---|---|
| **CV-core** | the property (D2, §4 computed value), §4.1 interpolation, the `[hidden=until-found i]:not(embed)` UA rule |
| **CV-reveal** | the ancestor revealing algorithm's until-found arm, `beforematch`, K10/K11 (writes run through P1; scroll through FU's point); the Hidden Until Found classifier (§4); wired into both shells' same-document fragment navigation and the initial-load *try to scroll to the fragment* (built, every load path, D5) |
| **CV-render** | S1 (K3, F1), S2 (K4, over P2's `flat_tree_parent`); paint and hit-test readers (D4, K5–K7); flips every landed parent pin (K14) |
| **CV-a11y** | a11y omission (§4.6) |
| **CV-focus** | the focus reader (K1's state split, K15; `focus()` through FU's point); after A2c (§5.2) |
| **CV-readers** | U1: scrolling operations, IntersectionObserver, ResizeObserver, animations/transitions (K16; the skipped-at-last-style-change marker, defined here; script sampling through FU's point) |
| **CV-details** (node) | the shadow surfaces it consumes (U2; §8's per-slot table); the UA shadow template and instantiation (D7, K12); `details`' assignment rule over P2 (K13); `details`' steps, `summary` and default-summary activation behaviours and keyboard subwidget on P1's mechanisms (D6, K13); the reveal `details` arm (D5); the second slot's style rule, last (K9) |

Not in this list: the sibling programs FU (D12), P1 (D6) and P2 (D8, D11), whose edges into this program are §5.2's.

**Reader → wiring PR** (the flip event the parent's pins name): **paint — CV-render; hit testing — CV-render; accessibility
— CV-a11y; focus — CV-focus; selection — whoever builds a user-agent selection producer (K17; D10, none at HEAD)**;
**scrolling operations, IntersectionObserver, ResizeObserver, animations/transitions — CV-readers** (U1); `innerText`,
`checkVisibility`, top layer, media audio, find-in-page — whoever builds them (K17).

**The parent's pinned cells** (parent `:1001-1023`; each recorded by its exposer only if it fires at the exposer's actual
base): IBP-sandbox (`<noscript>` witness; its node pins it at N9, IBP-sandbox §5.2), IBP-css-machinery (`[data-k=V i]`),
IBP-layout (`button` overflow), PR-1b, PR-1c, PR-1d — **all hit-test cells; flip = CV-render**. IBP-sandbox,
IBP-css-machinery and IBP-layout precede CV-render (E20's chain), so their pins fire whenever their witnesses do; PR-1b–PR-1d
are unordered with CV-render, so each pins only if CV-render has not landed at its base. CV-render's plan enumerates the
landed pins by a recorded marker (§7) and flips each assertion to the conformant value (the `div`). Ruled out by the
parent: PR-1a, IBP-ua-display.

**Own deferrals per nested PR** (cap 3): every nested PR — 0 expected: the slots each registers or appends to (§8) are
pre-existing class (A96); a cell one of them fires inside this program is repaired inside it (D5).

### §5.2 Edges (decided; the parent's one rule)

| # | Edge | Kind |
|---|---|---|
| E22 | IBP-css-machinery → **every nested PR** | item 11's literal (D9); data flow into CV-core (the `i` flag) |
| E20 | IBP-predicate → **every nested PR** | item 11's literal (D9); data flow into CV-render (F1, S1's applies-to) |
| E26 | this memo's approval → IBP-sandbox's first nested landing | the parent's edge — a docs artefact |
| c1 | CV-core → CV-render | data flow (the computed value S1 reads); also makes CV-render the flip of the `until-found` pins (PE5) |
| c2 | CV-reveal → CV-render | A93 (D5's fragment cells, same-document and initial-load) |
| c3 | CV-render → CV-a11y, CV-focus | data flow (S1, S2) |
| c4 | CV-reveal → CV-details; CV-core → CV-details | data flow (the reveal function its `details` arm extends; the property its slot style uses) |
| c5 | IBP-classify → CV-details | data flow (the `HtmlElementHandler` seam, D7) — **an addition to the parent's edge list** (§8 records) |
| c6 | CV-render → CV-readers | data flow (S1, S2) |
| c7 | IBP-observer → CV-readers | data flow (F13, the observed box the ResizeObserver gate reads) — **an addition to the parent's edge list** (§8 records); U1 |
| c8 | CV-reveal → CV-readers | data flow (K16×K11: the fragment scroll's skip check sits after the reveal call CV-reveal places) |
| c11 | CV-render → CV-details | data flow (U2's slot and fallback paint emissions pass K7's gate; S1, S2) |
| FU | FU (sibling; its consumer landing) → CV-reveal, CV-focus, CV-readers | data flow (FU-3–FU-6: the update point K11's scroll, K15's `focus()` and K16's sampling read through; D12) — **external**, like E20/E22 |
| P1 | P1 (sibling) → CV-reveal, CV-details | data flow (P1-1–P1-4: steps for every writer — the reveal's `hidden` removal and `open` set from the shell; `details`' steps; the activation step; D6) — **external** |
| A2c | canonical-focus A2c (`#11-vm-host-synthetic-dom-event-dispatch`; after S5) → CV-focus | data flow (CF-1: the VM's `focus()` on `focusing_steps`' path, so it reaches FU's point and the S2-wired predicates; else a stale-S2 regression, K15) — **external**; A2c's ownership unchanged (the canonical-focus lane, driver-less) |
| P2 | P2 (sibling) → CV-reveal, CV-render, CV-details | data flow (P2-1–P2-4: `flat_tree_parent` for the reveal walk and S2; `details`' assignment, K13; D11) — **external** |

Acyclic: the program precedes nothing (item 12); its inputs (IBP-css-machinery, IBP-predicate, IBP-classify, IBP-observer,
FU, P1, P2, A2c) have no predecessor in it; E26 is a docs artefact. Inside, one topological order is CV-core, CV-reveal,
CV-render, CV-a11y, CV-focus, CV-readers, CV-details: every edge c1–c8, c11 points forward in it (c9 and c10 are withdrawn
with CV-flat-tree and CV-flush); CV-details follows CV-render by c11, so K9 holds inside CV-details. CV-focus follows
CV-core transitively (c1, c3), so its until-found descendants stay excluded at its landing (B11's right cells).

**Ordering cost** (stated for the approval PR body, honestly): every nested PR waits on the IBP chain (E20, E22; D9).
Beyond it, only CV-core waits on no sibling: CV-reveal waits on FU, P1 and P2, so CV-render (the parent pins' flip, c2)
and everything after it — CV-a11y, CV-focus, CV-readers, CV-details — sit behind all three sibling programs (owners: P1 the L3 DOM/form lane, paused since 07-27, starting after #527/#501
land, and P2 the Layout lane — USER DECIDED 2026-10-24 (via PM); FU the Layout lane, D12). CV-focus
also waits on A2c, which is driver-less and follows S5.

### §5.3 Named risks per sibling and per nested PR (input lines)

**Sibling risks** (input to each sibling's plan-review with its invariants; withdrawn from decided text by rev 7):
- **FU-R1** (R5-1; FU-6) hook vs transition detection: the marker is S2 at the end of invocation N over N's after-change styles;
  detection reads the previous marker first; the old-style baseline feeds detection, not the marker (else pre-pass S2).
- **FU-R2** (R2/R3) a forced pass's phase and aliasing: order against `SessionCore::flush` (B24), the phase window, a bind
  bracket's `*mut EcsDom` (`#11-bound-safe-dispatch-dom-aliasing`). **FU-R3** FU-5's boundary (revs 4–6: an injected
  trait); where the animation engine's S2 caller sits if FU moves the style phase out of the shell (`elidex-css-anim`
  does not depend on `elidex-ecs`; K16's HEAD callers). **FU-R4** both shells; style vs style+layout per reader; re-entrancy in `beforematch` / dispatch; cost; Q-s3, Q-s7.
- **P1-R1** (R5-3) MO records for writers with no VM bound: `commit_notify_records` (`vm/host/dom_bridge.rs:57`) drops
  them when unbound; reuse the record-producing primitives (`apply_set_attribute` / `attribute_record`); the drain.
- **P1-R2** (R3) dispatcher lifetime per bind bracket (B18); the mutation-in-dispatch ban (`dom/mod.rs:193-205`) vs P1-2;
  event-loop access for `toggle`; B1's install point (`docs/plans/2026-06-b1-mutationobserver-plan.md:142`) moves.
- **P1-R3** (R3) dispatch paths: both shells, script `dispatchEvent` (a VM walk, B18), `DispatchTarget::VmObject` (#264);
  no `elidex-script-session` → `elidex-dom-api` cycle; whether to unify the two walks; the form-activation migration (B13).
- **P1-R4** cross-reference only: D7's instantiation at every creation path is CV-details' input (D7, Q-s10). **P1-R5** the three MO slots (D6).
- **P2-R1** (R5-4a) `details`' rule reachable from the `elidex-ecs` writer, destroy included, with no registry lookup
  (candidate: a component). **P2-R2** (R3/R4) stored *assigned nodes* vs *find slottables* on a manual re-insertion
  (*insert* step 7.4 is named-mode only). **P2-R3** (R3) the *assigned slot* DOM never clears (Node's *get the parent*,
  DOM §4.4) — a deviation to record; the event path's *get the parent* / `composedPath`.
- **P2-R4** (R3/R4) `slotchange` order and one channel from every source (*assign slottables* 2; *insert* 7.5; *remove* 9;
  *move* 15, 23), out-of-bracket writers included; reuse of `pending_slot_change_signals` (B25). **P2-R5** (R4) host
  pseudo-element entities (`composed_children(host)` omits them, `shadow.rs:529-537`); whether they render → Chromium.
- **P2-R6** a UA rule's re-derivation point vs *insert*'s insertion steps (7.7.1) and children changed steps (9), a P1 × P2
  pair; `moveBefore`; rev 4's list (`tree_clone.rs:30`, destroy, `walk.rs:459`, `:582`, `docs/architecture/core.md:22`).
- **CF-R1** (R5-2) the subwidget read by the base predicate `is_focusable` (`reconcile_focus`'s, B23), not only
  `is_focusable_area`; anchor the `details` element (HTML §6.6.2); A3's unification and sequential navigation.

**Nested-PR risks**:
- **CV-core**: CSSOM `getComputedStyle(el).contentVisibility` is the only observable change; the until-found rule against
  §15.3.1's other two rules (B4) — the two `:not(embed)` rules partition `[hidden]:not(embed)`, and `embed[hidden]` is the
  third rule; `computed_style/tests.rs` 947 lines.
- **CV-reveal**: event dispatch from both shells mid-navigation; the initial-load path's task model (§7.4.6.2's queued
  global task, step 2.1's abort and its retry); iframe documents' load path — at HEAD a same-document iframe fragment
  navigation rebuilds through a load (`content/iframe/thread.rs:199`, B20), so it reaches the initial-load *try to scroll*,
  overlapping the OOP-iframe re-tag of `#11-iframe-fragment-navigation`; the callback shape against the bind bracket (the
  reveal holds `&mut EcsDom` across the `beforematch` dispatch, which needs the pipeline mutably) — P1-1 covers the
  shell-side writers; scroll geometry after mutation through FU's point (K11, FU-4); the classifier's crate.
- **CV-render**: per-owner gating sites (Q-s9); pseudo-elements (K5); out-of-flow (K6); replaced content vs the element's box
  for `<img hidden="until-found" src=a.png>` (the parent's no-pin note, `:1024-1025`); `builder/mod.rs` is 1002 lines
  (split before touching), `builder/walk.rs` 836 and `hit_test.rs` 764; the pin flips; FU's S1-agreement cell (C21).
- **CV-a11y**: the walk's handling of unassigned light children (K4); `a11y/tree.rs` is a `LayoutBox` reader on
  `.claude/tools/layout-box-reader-allowlist.tsv` (C-3c) — the allowlist and its trip-wire are updated with it.
- **CV-focus**: the predicates (Q-s8) — the canonical-focus A3 unified predicate overlaps it (§9), and A2c moves the VM onto
  `focusing_steps` (CF-1); CF-R1; `focus/tests.rs` 999 lines (split first); the flipped cells (K1).
- **CV-readers**: the animation semantics (items 9–10) against `elidex-css-anim`'s tick model, the caller's location (FU-R3)
  and the marker's writing at every invocation (K16, FU-R1); script sampling through FU's point; the app shell's tick site;
  ResizeObserver's "deliver on un-skip only if the size differs" against F13; IntersectionObserver's both-skipped case; the
  scroll callers' population (Q-s12); may divide per family; its cells C16, C20, C22; the scrolling reader's files are
  on `layout-box-reader-allowlist.tsv` (`layout_query.rs` C-3b, `content/scroll.rs` C-3d) — updated with it.
- **CV-details**: the template / instantiation crate and how it reaches every creation path (Q-s10; R5-4b:
  `EcsDom::create_element*` holds no handler registry — candidate: components on the UA root / slots — nor reaches the CSS parser); the "script-visible shadow root" query's crate — a home in `elidex-dom-api` cycles with `composed_path_for_js` in
  `elidex-script-session` (`event_dispatch/mod.rs:156`), so at or below `elidex-script-session`; the shadow surfaces it
  builds (U2, Q-s2; B22); how P2's writer obtains `details`' rule (P2-R1); the keyboard subwidget (K13) across
  K12 × K13 × K15 — its exposure (`activeElement` and the like through the one script-visible-root query),
  `reconcile_focus` (CF-R1), sequential navigation (Q-s8); `toggle` from synchronous to queued (an author-visible timing
  change, toward the spec); the exposure sites moved onto the one query (K12); `focus/predicate.rs`'s
  `is_first_summary_of_details` (`:122`) still holds; `details_set_open` becomes marshalling.

## §6. ECS-native and Layering checks (seed; input)

**OO → ECS**: Blink's `DisplayLockContext` per element (a registry with lock state and update flags) → no state: S1/S2
queries over `ComputedStyle` and the tree (D1); "contents are locked" flags propagated by a lifecycle → flat-tree ancestry
read on demand (D3); Blink's details-element hook run when its UA shadow root is added (a virtual) → a template declared on
the element-kind plugin seam, instantiated by one function, + a `UserAgentShadowRoot` marker (D7); a per-site "is this root
closed/UA" check → one "script-visible shadow root" query (D7); the per-element "style attribute set to" rewrite → a
cascade-derived positive `:host([open])` pair (D7); animation "paused while locked" state → the tick not advancing skipped
entities, plus a per-entity *skipped at the last style change* marker component (K16); the details' toggle task tracker
held on the element object → a tracker component on the `details` entity (`Send + Sync`, the side-store rule; §4); a
per-element-kind slot-assignment override (a virtual on the element class) → a per-element-kind assignment rule P2's one
writer reads (*input*: its representation, P2-R1). (The slot-object idioms are P2's; the per-element virtual steps and
activation are P1's.)

**Fact → existing API → crate** (a9866498; reuse before adding):

| Fact | Existing API | Crate |
|---|---|---|
| shadow children / assigned nodes (P2 extends) | `composed_children` (`dom/shadow.rs:529`), `assigned_nodes` (`:394`) | elidex-ecs |
| `details` name group | `collect_open_details_by_name` (`element/details_exclusion.rs:61`) | elidex-dom-api |
| VM shadow accessors | `native_element_get_shadow_root` (`vm/host/element_shadow.rs:86`) | elidex-js |
| focusing steps / `focus()` entry | `focusing_steps` (`focus/update_steps.rs:106`); `native_html_element_focus` (`vm/host/html_element_proto.rs:269`) | elidex-dom-api; elidex-js |
| focus predicates, fixup | `is_focusable` (`focus/predicate.rs:205`), `is_focusable_area` (`focus/delegate.rs:133`), `reconcile_focus` (`focus/sot.rs:178`) | elidex-dom-api |
| session flush | `SessionCore::flush` (`session.rs:114`) | elidex-script-session |
| observer gathering | `gather_observations` (`intersection/mod.rs:270`, `resize.rs:235`) | elidex-api-observers |
| transitions / ticking | `detect_and_start_transitions` (`animation.rs:87`, called `lib.rs:692`); `AnimationEngine`'s `tick` (`engine.rs:147`, called `content/event_loop.rs:256`) | elidex-shell; elidex-css-anim |
| fragment scroll | `scroll_offset_for_fragment` (`content/scroll.rs:210`) | elidex-shell |

Layering: the S1/S2/C queries in `elidex-ecs`; reveal, `details` behaviours, the Hidden Until Found classifier and the
scroll-into-view resolution in `elidex-dom-api`; the script-visible-root query engine-independent (§5.3); the UA template
declared in `elidex-plugin`, instantiated engine-independently; readers in their crates; the VM marshals
(`details_set_open`, `beforematch` handler compile only); shells call one reveal function and dispatch
through the session. P1's, P2's and FU's mechanisms and their crates are their programs' layering.

## §7. Test and coverage plan (input)

**Declared surface**: css-contain-2 `content-visibility` `visible | hidden` (not `auto`), §4.6; HTML §15.3.1's until-found
rule, §6.1's revealing algorithm (same-document and initial-load fragment navigation), §15.5.5, §4.11.1, §4.11.2 (the
assignment surface — DOM §4.2.2 — is P2's). WPT seeds (each PR's plan picks): `css/css-contain/content-visibility/`
(`content-visibility-0xx` hidden cases, `content-visibility-computed`/`-valid`/`-invalid`,
`content-visibility-vs-scrollIntoView-00x`, `content-visibility-resize-observer-no-error`, the hidden-subtree animation
tests), `html/editing/the-hidden-attribute/` (`beforematch`), `html/semantics/interactive-elements/the-details-element/`.
Cells (owner):
- C1 computed value + serialization (CV-core); C2 the until-found rule incl. `HIDDEN=UNTIL-FOUND` (CV-core).
- C3 fragment reveal: `beforematch` fired, attribute removed, scroll after — same-document and initial load (conditional on
  §7.4.6.2 step 2.1: a load whose task runs after the parser stops does not reveal), both shells, an iframe document's load
  (CV-reveal); C4 a handler that disconnects or re-hides (K11's four re-checks; CV-reveal).
- C5 paint: contents unpainted, the element's border painted, `<img hidden=until-found>`'s image unpainted,
  `::before`/`::after` and an `li`'s outside and inside `::marker` unpainted (K5, B26); C6 abspos/z-index descendant
  skipped; C7 hit passes through to the S1 element; C8 applies-to: `display: inline`/`contents`/`table`/`table-cell` no
  effect, `inline-block`, `table-caption`, an inline `img` applies; C9 the parent pins flipped (all CV-render).
- C10 a11y omission (CV-a11y); C11 `<button hidden=until-found tabindex=0>` focusable, its descendants not, render-time blur
  (CV-focus).
- C12 `details` (CV-details; P1-1, P1-2, P1-4): closed contents unpainted; a shell click and a script-dispatched click
  on `summary`, a click on a descendant of an author `summary` and keyboard activation of an author `summary` open it;
  a summary-less `details` opens by clicking its default summary, and a click elsewhere in its
  contents does not toggle it; keyboard activation of the default summary's subwidget toggles it; the default summary is
  exposed as its summary in the a11y tree; `toggle` queued once; name-group exclusivity on `setAttribute`, on insertion and
  on a parsed `<details open name>`, and a parsed `<details open>` queues `toggle` once; fragment into a closed `details`
  opens it; no `shadowRoot` exposed; clone re-creates; `:host`-as-ancestor rules match in author shadow sheets too.
- C13 `scrollIntoView()` on a skipped node and a fragment into author `content-visibility: hidden` do not scroll; C14
  IntersectionObserver reports a skipped target non-intersecting; C15 ResizeObserver silent while skipped, delivers on
  un-skip only on a size change; C16 (K16) while skipped a running animation does not advance or end and no animation or
  transition starts; on un-skip the animation resumes and no transition starts; C22 (K16) moving `p` out of a
  `content-visibility: hidden` ancestor while changing its `color` in the task starts no transition, under the frame pass
  and under a forced pass; C23 (FU-4) a `getComputedStyle` read of a skipped element in the task that changed its
  animation style samples per the styles at that style change event (§4.5 item 9) (all CV-readers).
- C18 (FU-4, CF-1) through the VM, `setAttribute('hidden','until-found')` then `inner.focus()` in one task refused,
  `details.open = true` then `inner.focus()` succeeds; a `beforematch` handler's mutation is reflected in the scroll offset
  (CV-reveal, CV-focus).
- C19 a parsed and a cloned `details` get the UA tree before their children are assigned (K12×K13; CV-details).
- C20 (K16, CV-readers' marker): two forced passes in one task — the element un-skipped by the first, its color changed
  before the second — the second style change starts its transition; C21 (FU-3's CV-render acceptance cell): under an animated
  `content-visibility`, a forced pass and the next frame pass agree on S1. C30 (FU-6, FU; a test consumer): state the
  consumer writes at the end of invocation N−1 is read by invocation N's transition-start decision — frame and forced alike.
- C24 (P1-1–P1-3; CV-reveal, CV-details): a shell-side fragment reveal (no VM bound during the write) removes `hidden` and
  opens a closed `details` — `toggle` queued once, exclusivity run, one page MutationObserver given the attribute records,
  interleaved with script records in mutation order;
  in a document with no VM, a parsed `<details name=g open>` pair runs exclusivity. C25 (CF-2, CV-details): sequential
  navigation focuses the default summary's keyboard subwidget and the next rendering update does not blur it.
- C26 (P2-1, P2-3; CV-render): after *insert*, a slottable's `slot` change, a slot's `name` change, `assign()`, *remove*,
  *move* and entity destroy (`moveBefore` when built), with no style pass between, `flat_tree_parent`
  and S2 reflect the new tree; a host's `::before` entity is reached alike upward and downward. C27 (P2-2's `details` acceptance cell, CV-details):
  `d.append('x'); d.prepend(summary)` with no style pass between puts `summary` in slot 1, `x` in slot 2; likewise a
  parser-built `details` and a shell-side writer's change. C28 (P2-4, CV-render): `cargo tree -p elidex-ecs -e normal`
  lists no `elidex-dom-api` while `dom/content_visibility.rs` reads the flat tree. C29 (FU-5): §11's D12 greps. C31 (P2-2, P2):
  a test-declared UA assignment rule is current after `append`, `prepend`, `remove` and a parser insertion, with no style
  pass and no VM bound.
(C17 — slot assignment — is P2's, D11.)

## §8. Out of scope, dispositions, records

**Out** (item 11): layout skipping (the spec's SHOULD); container-side containments (`#11-css-containment`); `auto`
(`#11-content-visibility-auto`); css-contain-2 §4.5 item 7 (an iframe's update-the-rendering skip, a SHOULD). The event
path's *get the parent* / `composedPath` (P2's, D11).

**Registration timing** (the IBP-sandbox §8 rule): a gap that exists at HEAD (A96) is registered **at approval**
(re-eval = approval merge + 1 month); a gap created by a nested PR's code at that landing (+1 month).

**Slots** (registration and appends at approval unless the row says otherwise):

| Slot | Disposition |
|---|---|
| `#11-content-visibility-skip-contents` (home, opened 2026-09-30) | its trigger ("the approval PR lands — authoring of its umbrella memo starts") has fired; restate at approval: *Trigger* = each nested landing in §5.2's order; discharged by the last; re-eval per the timing rule. This memo is its umbrella |
| `#11-element-lifecycle-hooks` (NEW, P1); `#11-shadow-slot-assignment-flat-tree` (NEW, P2) | **registered at the E26 approval PR**; owners USER DECIDED 2026-10-24 (via PM): P1 the L3 DOM/form lane, P2 the Layout lane; homes = their planned umbrellas, which inherit D6's / D11's invariants and §5.3's risks for them; this memo records only their consumer edges. *Trigger*: the owner's start — P1: the L3 lane's return to DOM/form after #527/#501 land; P2: the Layout lane's scheduling after E26's approval. *Re-eval*: approval merge + 1 month. P1's MO overlap (P1-3): `#11-tree-mutation-record-pipeline`, `#11-method-driven-attribute-records`, `#11-ce-reaction-mutation-observer-ordering` stay open; whether P1 absorbs any is decided by P1's plan-review alone (PM ledger, 2026-10-24). P2 absorbs `#11-shadow-slotchange-named-mode-detection` (cited in code, not in the ledger, B25), `#11-shadow-slot-flatten` (unledgered; recorded by the PM) and the writer part of `#11-shadow-light-tree-distribution-rendering` |
| `#11-forced-style-layout-flush-on-script-read` (#388) | **not this program's**: the PM re-pointed it (2026-10-24; home = the FU umbrella; owner the Layout lane); id kept, not an item-11 amendment; this memo records only FU's consumer edges and D12's contract; FU discharges it |
| `#11-ua-sheet-html-rendering-display-fidelity` | **not registered** at HEAD (registered by IBP-ua-display only if a remainder exists, parent `:587`, `:1693`). Its `summary` member is the document-level `details > summary:first-of-type { display: list-item }` (light DOM; needs `:first-of-type`, parent `:1961-1965`), which **stays** there. CV-details builds only §15.5.5's shadow `:host summary` (the default summary's display): it touches only a default-summary facet, and only if that slot exists |
| `#11-iframe-fragment-navigation` | cross-reference: CV-reveal's initial-load path covers iframe documents' loads (D5), and at HEAD a same-document iframe fragment navigation reaches it through a load (§5.3) — overlap with the slot's OOP-iframe re-tag, recorded for its owner |
| `#11-css-containment` | append: when it lands, S1's element gets the used `contain` value (size, layout, style, paint); K6's ancestry rule stays the readers' rule |
| `#11-content-visibility-auto` | append: S1's `auto` arm; relevance reads focus, selection and the top layer (`#11-dialog-top-layer`); §4.5 items 3, 4, 6; **K1's reader-arm table** — when `auto` lands, every reader classified hidden-arm-only gets the hidden-arm form; the any-skip readers keep S1/S2 |
| `#11-find-in-page` (NEW) | *Gap*: elidex has no find-in-page (B16); *obligations*: HTML §6.9.2's temporary un-skipping of until-found and closed-`details` contents during the search, its reveal call through the one reveal function; reads S2 (K17). *Why deferred*: no find-in-page feature exists to wire; A96. *Trigger*: a find-in-page feature or `window.find`. *Re-eval*: approval merge + 1 month |
| `#11-details-content-pseudo` (NEW, O1) | *Gap*: the `::details-content` pseudo-element (css-pseudo-4 §5.2) — HTML §15.5.5 expects `details`' second slot to match it. *Why deferred*: selector machinery elidex lacks (a pseudo-element onto a UA shadow element, the `::part` shape), not skipping; A96. *Trigger*: CV-details has landed **and** (a css-pseudo-4 `::details-content` WPT subset is declared supported, or a pseudo-element onto a UA shadow element is built for another element). *Re-eval*: approval merge + 1 month |
| `#11-content-visibility-absent-readers` (NEW, U1/K17/D10) | *Gap*: `innerText` (§4.5 item 8), `checkVisibility()` (CSSOM View §6 step 2), user-agent selection producers for document content (D10) and media audio suppression (§4.5 item 12) do not exist (B16, Q-s5). *Obligations*: each, when built, reads K1's query per the arm table — `innerText` and selection producers S2, `checkVisibility` C's ancestor form — never a second derivation; **media**: an element whose own computed value is `hidden` or that is in skipped contents (hidden arm) is not paused on account of being skipped, its audio suppressed without changing `muted` / `volume`; the events it queues are held until it is un-skipped, its state queries stay current; audio need not be suppressed when presented outside the skipped contents (e.g. Picture-in-Picture); an iframe so placed extends this to its document's media. *Why deferred*: no reader to wire. *Trigger*: any of them lands. *Re-eval*: approval merge + 1 month |
| `#11-dialog-top-layer` (home `m4-12-pr-tags-t2d-interactive-landing.md:81`, and its roadmap rows) | append K17's entry: §4.5 item 11 — an element with a `content-visibility: hidden` flat-tree ancestor placed in the top layer generates no boxes (C's ancestor form). The roadmap's HTMLMediaElement row (`M4-13:#tags-T3-media`) gets a pointer to `#11-content-visibility-absent-readers` only |
| `#11-focusable-area-being-rendered` (home `boa-vm-cutover-s2-r3-handoff.md:775`, Q-s4; cited at `focus/predicate.rs:202`, `:300`) | append: "S2 and FU's update point remove the 'needs computed style' blocker; CV-focus's plan decides discharge"; the Hidden arm and the CSS residue stay there until then |

**Shadow-lane slots under U2** (current text read 2026-10-20, read-only; each closed or narrowed **at its owning landing**,
each moved part owned by the program that receives it — P2 for the assignment writer, the Layout lane for CV-details' facets; which facets are real is Q-s2, answered by CV-details' plan-review):

| Slot | Current text (home) | Moves | Remains |
|---|---|---|---|
| `#11-shadow-light-tree-distribution-rendering` | "light-tree → flat-tree distribution for paint (slotted nodes が slot position で render)", trigger "paint pipeline milestone", **M4-13 roadmap rows** (not in the open-slot ledger) | the one assignment writer → **P2** (D11); slotted nodes rendered at their slot for **every** shadow tree → CV-details (the flat tree is one structure, K4) | nothing once both have landed → **closed** at CV-details (writer part recorded at P2's landing); if Q-s2 finds the rendering facet stale (HEAD's `composed_children` renders assigned nodes, `shadow.rs:529-549`) → that facet closed as stale |
| `#11-slot-fallback-content-rendering` | "`<slot>` fallback content の rendering pipeline 専門処理 (paint coupling)", trigger "paint pipeline milestone", **M4-13 roadmap rows** | fallback content rendered when a slot has no assigned nodes (the default summary) — the whole facet → CV-details | **closed** at CV-details, or closed as stale |
| `#11-shadow-tree-style-isolation` | "`:host` / `:host-context()` / `::slotted()` selectors + cascade isolation", trigger "CSS milestone OR framework demand", **M4-13 roadmap rows** | a UA-origin shadow sheet applied to a UA root, and `:host` as a non-subject compound in the shared combinator walk for **every** origin (css-shadow-1 §3.2.2–§3.2.3; D7) — **built by CV-details** (B22) | **narrowed** at CV-details to `:host-context()`, `:not` inside `:host()` (unless built), author-shadow cascade isolation, and what else Q-s2 re-measures; trigger unchanged; *Re-eval*: CV-details' landing + 1 month |
| `#11-shadow-composed-event-retargeting` | **closed** — implemented, `apply_retarget` (`event_dispatch/mod.rs:627`; `m4-12-defer-sweep-2026-06-19.md:38`); its open roadmap rows are stale | nothing (PE6); CV-details consumes HEAD's retargeting | — (the stale roadmap rows are struck at approval) |

**Records**:

| Record | Ships with |
|---|---|
| **Amendments of the parent's §0.6 item 11 decided content** (recorded, not edited; USER DECIDED 2026-10-20): U1 — the reader list becomes every reader elidex has (K16) plus the standing obligation for absent readers (K17); U2 — the `details` UA shadow tree's scope includes the shadow surfaces it consumes (the per-slot table above). Parent premise errors PE1–PE6 (recorded, not edited); the parent's edge list gains c5 (IBP-classify → CV-details) and c7 (IBP-observer → CV-readers); E20/E22 bind every nested PR (D9); the sibling programs FU, P1, P2 and their edges into this program; the reader→PR table naming the flip for every parent pin (§5.1); the parent's §6 Open row for the home slot read with U1/U2 | **the approval PR** |
| Memory — every shared-ledger write (the open-slot ledger, the roadmap, the landings ledger), at approval **and at each nested landing** (slot closures and narrowings), is **transcribed by the PM** from the driver's report (`project_pm-and-lane-driver-structure.md`). At approval: the P1 and P2 slots registered (owners and triggers as above) with P2's absorptions; the home slot's restated trigger and scope (U1, U2); `#11-find-in-page`, `#11-details-content-pseudo` and `#11-content-visibility-absent-readers` (media included) registered; the appends to `#11-css-containment`, `#11-content-visibility-auto` (the arm table), `#11-dialog-top-layer` and `#11-focusable-area-being-rendered`; the HTMLMediaElement row's pointer; the shadow-lane roadmap rows marked "moved to `#11-content-visibility-skip-contents` (CV-details) / P2"; the stale `#11-shadow-composed-event-retargeting` roadmap rows struck. FU's slot re-point is the PM's (already recorded) — FU's consumer edges and its invariants FU-3–FU-6 (rev 8; FU-1/FU-2 withdrawn) are reported for it. The earlier ledger promise "the slotchange slot registered at approval, CV-flat-tree closes it" is superseded by P2's absorption | **the approval PR** |
| The parent pins flipped; the home slot's paint/hit facets | CV-render |
| The home slot's further-reader facets (K16) | CV-readers |
| The shadow-lane slots closed or narrowed per the table above (Q-s2's answer recorded); the default-summary facet of `#11-ua-sheet-html-rendering-display-fidelity`, if that slot exists | CV-details |
| The home slot closed | the last nested landing |

## §9. Prerequisites

- **E20** and **E22** before **every** nested PR (D9); **c5** (IBP-classify) before CV-details; **c7** (IBP-observer) before
  CV-readers; the sibling programs — **FU** before CV-reveal, CV-focus, CV-readers; **P1** before CV-reveal, CV-details;
  **P2** before CV-reveal, CV-render, CV-details (§5.2's cost); **A2c** (canonical-focus, external) before CV-focus;
  **E26** (this memo's approval) before IBP-sandbox's first nested landing.
- **Touch-time splits** (origin/main counts, §0.1): `elidex-render/src/builder/mod.rs` (1002) and
  `elidex-dom-api/src/selection.rs` (1036) are over 1000 — a standalone split precedes any nested PR that touches them;
  `focus/tests.rs` (999) before CV-focus adds a cell. Watch: `content/event_handlers.rs` 966 (P1's activation migration),
  `computed_style/tests.rs` 947 (CV-core), `html_element_proto.rs` 909 (CV-reveal, CV-focus), `app/mod.rs` 897 and
  `content/mod.rs` 858 (CV-reveal's load paths), `builder/inline.rs` 860 and `builder/walk.rs` 836 (CV-render),
  `elidex-css-anim/src/engine.rs` 809, `hit_test.rs` 764. `elidex-ecs/src/dom/mod.rs` (929 after E1) is not touched (§4's
  new file).
- **Lanes**: the canonical-focus lane (`#11-canonical-focus-update-steps`) is driver-less and has no branch; its planned A2c
  (CF-1; an edge into CV-focus, §5.2) and A3 (the unified focusability predicate plus sequential navigation, its memory
  file's A3 section; CF-R1) overlap CV-focus (Q-s8): CV-focus coordinates with them at its cut. IBP-sandbox's nested PRs
  edit the shells' navigation files (`content/navigation.rs`, `app/navigation.rs`) — CV-reveal's plan checks at its cut;
  IBP-observer edits `elidex-api-observers/src/resize.rs` before CV-readers (c7). FU may move the frame pass (`lib.rs`'s
  `re_render`; FU-R1, FU-R3); P1 may edit the content shell's activation (`content/event_handlers.rs`; P1-R3).

## §10. Questions for the scout

Answered: **Q-s1** (slot assignment writers) — B17, now P2's input; **Q-s4** (`#11-focusable-area-being-rendered`'s home)
— `boa-vm-cutover-s2-r3-handoff.md:775` (§8); **Q-s5** (user-agent selection producers) — none for document content (D10);
**Q-s6** (initial-load fragment scroll) — absent (B20), built by CV-reveal (D5); **Q-s11** (`generate_pseudo_entity`'s
parent link) — the originating element (B19, K5).

Moved to the owning nested PR's plan-review, or to a sibling umbrella:
- **FU umbrella** — **Q-s3** Which script-time reads join the update point beyond D12's decided consumers (layout
  queries, `getBoundingClientRect`). **Q-s7** How each shell dispatches a DOM event synchronously during a same-document
  fragment step, and how style and layout are re-run before the scroll offset is computed.
- **CV-details** — **Q-s2** At its base, which facets of `#11-shadow-light-tree-distribution-rendering`,
  `#11-slot-fallback-content-rendering` and `#11-shadow-tree-style-isolation` are real? **Q-s10** Every production creation
  path of an HTML element (both parsers, `createElement`, `cloneNode`, fragment parsing, `importNode`/adopt) and how D7's
  instantiation reaches them all.
- **CV-focus** — **Q-s8** At CV-focus's base (after A2c, CF-1) `focusing_steps`, the VM's `focus()` included, reads
  `is_focusable_area` (`update_steps.rs:113`); `reconcile_focus` reads `is_focusable` (`predicate.rs:205`); sequential
  navigation's order builder reads which? CV-focus
  wires S2 into each predicate these reach, coordinated with A3's unification.
- **CV-render** — **Q-s9** The display-list builder's emission sites and the owning entity each item carries (text runs,
  replaced images, form-control content, list markers, pseudo-elements).
- **CV-readers** — **Q-s12** Every scroll-into-view caller (CSSOM View *scroll a target into view*): `ScrollIntoView`, the
  fragment scroll, focus-driven scrolling — routed through the one `elidex-dom-api` resolution (§4).

## §11. Definition of done (program)

- Every nested PR in §5.1 (seven) has landed in an order consistent with §5.2 (FU's point before CV-reveal, CV-focus,
  CV-readers; P1 before CV-reveal, CV-details; P2 before CV-reveal, CV-render, CV-details; A2c before CV-focus), each
  with its own reviewed plan-memo. A nested PR whose verdict is "may be edge-dense" (CV-reveal, CV-render, CV-readers,
  CV-details) may divide by its own plan; CV-render's division keeps D4 / K7 (paint and hit test together) and K14's
  flip in one child.
- The shadow-lane slots are closed or narrowed as §8's U2 table records; K17's slots carry the obligation text. (The FU,
  P1 and P2 slots are their programs' to discharge, not this program's.)
- K1–K17 hold at the last landing; each nested plan-memo states its DoD greps — at least (each run at the landing head):
  - K1 (focus): `git grep -n -F '!is_focusable(&dom, uf)' -- crates/dom/elidex-dom-api/src/focus` = 0 hits (the
    until-found cell flipped) and `git grep -n 'in_skipped_contents' -- crates/dom/elidex-dom-api/src/focus` ≥ 1;
  - K1 (no direct reader): `git grep -l 'content_visibility' -- crates` lists only files under `elidex-plugin`,
    `elidex-css-contain`, `elidex-css-anim`, `elidex-style` and `elidex-ecs/src/dom/content_visibility.rs`;
  - K16: `/usr/bin/grep -rlE 'in_skipped_contents|skips_contents' crates` includes a file in each of paint
    (`elidex-render`), hit test (`elidex-layout`), a11y (`elidex-a11y`), focus (`elidex-dom-api/src/focus`) and the four
    CV-readers families (the scroll-into-view resolution, `elidex-api-observers/src/intersection`, `…/resize.rs`,
    the animation reader's S2 call, in the file CV-readers' plan names);
  - D6 (consumer side): `git grep -n -F 'collect_open_details_by_name(' -- crates/script` = 0 hits (no VM call; the setter
    is marshalling);
  - D7: `git grep -l -E '"[^"]*content-visibility:' -- crates ':(exclude)**/tests/**' ':(exclude)**/tests*.rs'` (string
    literals declaring the property) lists only `elidex-style/src/ua.rs` and the UA shadow sheet's file;
  - D12 (consumer side): the FU umbrella names the update point's entry `P`; `P` is reachable (a `git grep -n P` hit in
    the file of each call on the path, named by FU's plan) from the `elidex-dom-api` reveal function
    (`run_ancestor_revealing`) and from `focus()`'s path into `focusing_steps`, each ≥ 1; `git grep -n P --
    crates/dom/elidex-dom-api/src/focus/predicate.rs crates/dom/elidex-dom-api/src/focus/delegate.rs
    crates/dom/elidex-dom-api/src/focus/sot.rs` = 0 (neither a predicate nor `reconcile_focus` calls it).
- Every parent pin found at its exposer's base is flipped (K14); the §8 records have landed.
- **Checkers** (run; do not trust a stored verdict): `python3 .claude/skills/elidex-plan-review/preflight.py
  docs/plans/2026-10-content-visibility.md`.
