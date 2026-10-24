# `content-visibility` skip-contents program (umbrella plan-memo)

The `content-visibility` program of the inline-box predicate prereq program's §0.6 item 11
(`docs/plans/2026-09-inline-box-predicate.md`, approved by #526 at rev 73, `f1cf5d67`; "the parent"). Home slot
`#11-content-visibility-skip-contents`; owner the Layout lane. The parent's decisions bind here (its approval scope, `:3-9`);
item 11's content and item 12's ordering are **USER DECIDED 2026-09-28**. This memo changes that content only where the
user decided so on 2026-10-20 (U1: the reader set widened; U2: CV-details builds the shadow surfaces it consumes), recorded
as amendments of item 11 under the A114 idiom ("recorded, not edited", §0.5, §8 records) and ratified by the approval PR.

**Approval scope (the #526/#532 idiom, parent `:3-9`).** This memo **decides**: the nested-PR set, their order and edges
(§5.1–§5.2), and how the parent edges E20, E22 and E26 bind to them; each fact's home and representation kind (§4's
*Home/kind* lines); the program-level invariants K1–K17 with their landing qualifiers (§2); the D-items (§0.4); the user
decisions U1–U2 and the decisions O1–O2 (§0.5); the spec-semantics decisions (which algorithm, which steps, which outcome —
§0.4, §3's *Branch* column); **which nested PR wires skipped contents into each reader** (§5.1's reader table — the flip
event every parent pin names); and the slots and records (§8). §4's mechanism prose, §5.3's named risks, §6's tables, §7's
cells, §3's *Touch* and *Full enum?* columns, the per-PR edge-density verdicts marked *input* in §1, and every item marked
*input* are **input to each nested PR's plan-review**, which re-derives them. A review finding against that input changes
this memo only when it changes a decision; otherwise it is recorded as a named risk in the owning nested PR's input lines
(§5.3).

**Approval vehicle.** A docs-only approval PR (the #526 / #532 vehicle). The user's merge approval ratifies the
approval-scope paragraph, the D-items, U1–U2 (the amendments of the parent's item 11) and O1–O2. **E26**: this memo's
approval precedes IBP-sandbox's first nested landing (parent `:1415`; IBP-sandbox memo D7,
`docs/plans/2026-09-ibp-sandbox.md:180`); the approval PR itself is a docs artefact gated by nothing. No umbrella A-row is
drafted: nothing here amends a surface the umbrella (`docs/plans/2026-08-line-box-decorated-inline-content.md`) ratified;
additions to the parent are recorded here under the A114 idiom ("recorded, not edited", §8 records).

**Revision 3** (2026-10-24) — folds plan-review R1 (F1–F30). Structural: two new nested PRs before CV-reveal —
**CV-flat-tree** (D11: one mutation-time slot-assignment writer, a node-side `AssignedSlot` back-link unifying
`SlottedMarker`, and `flat_tree_parent`, moved from CV-reveal; F2, F3, F11) and **CV-flush** (D12: a script-time forced
style(+layout) update point, first read by reveal's post-mutation scroll and `focus()`; discharges
`#11-forced-style-layout-flush-on-script-read`; F1). Rewritten: D7 (the element kind *declares* a UA shadow template, one
engine-independent instantiation function; the second slot's style is a cascade-derived `:host(:not([open]))` UA rule, no
stored inline declaration; F4, F5), D6's seam (the `elidex-ecs` attribute chokepoint family, not the VM dispatcher; F5), D9
(item 11's literal: every nested PR follows E20 and E22; F15), D10 (selection producers → the absent-readers slot; F10, F13),
K1 (the spec's distinctions: S1/S2, their hidden arm, the computed-ancestor query C; F10, F13, F28), K5 (pseudo-elements are
contents per css-contain-2 §3.1, no probe; F8), K8, K11 (re-checks 4.1, 4.2.1, 4.2.3, 4.2.4; F14), K16 (items 9–10 exactly,
un-skip set in the caller; F7, F18), K17 (per reader). Added: CV-reveal builds the initial-load *try to scroll to the
fragment* (F9); `summary` activation in one engine-independent home both shells reach, the default summary by §15.5.5's
rendering expectation (F6); edges c9–c11 (F2, F11); §3 rows (F6, F8, F9, F12, F19, F23, DOM slot assignment); §6's fact →
API table (F17); the §0.1 origin/main note and 1000-line watch (F24, F25); §8 fixes (F20–F22, F26, F27); F16, F29, F30
clerical. §10's answered questions are marked with their answers; the rest moved to their owning nested PRs.

**Revision 2** (2026-10-20) — folds the decisions on rev 1's four open questions (§0.5): **U1** (user) widens the reader
set — new nested PR CV-readers, and a standing obligation for the readers elidex lacks (K16, K17); **U2** (user) moves the
non-stale shadow surfaces CV-details needs into this program, with a per-slot record (§8); **O1** registers
`#11-details-content-pseudo`; **O2** keeps D5 and D9 (rev 3: D9 rewritten, F15). Measured while folding:
`#11-shadow-composed-event-retargeting` is already **closed** (implemented, `elidex-script-session/src/event_dispatch/mod.rs:627`
`apply_retarget`), so U2 moves nothing from it (§0.3 P6).

*Rev 1* (2026-10-20) — first draft.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` is an **`a9866498`** coordinate; `git diff --stat e2d62b9e a9866498 -- crates` is empty, so the parent's
`e2d62b9e` coordinates still hold. origin/main at rev 3 (`e2122e23`: E1 #537 `cc05381e`, E2 #538 `811f523e`, #539) differs
from `a9866498` in 9 crate files (`git diff --stat a9866498 origin/main -- crates | tail -1` = "9 files changed", verified
2026-10-24); of those, this memo cites
`elidex-ecs/src/dom/attribute.rs` and `attribute_reconcile.rs` (each diff a one-line doc-comment edit at `:3`; cited lines
unchanged) and `dom/mod.rs` (its line count only). E1 moved the
node-classification queries into `elidex-ecs/src/dom/classify.rs` (its boundary doc); `dom/mod.rs` is 1075 lines at
`a9866498`, 929 at origin/main. Line counts in §9's watch list are origin/main measurements (`git show origin/main:<f> | wc
-l`). Spec text is `.claude/tools/webref body` output (2026-10-20, re-run 2026-10-24 for rev 3's rows); every `§X.Y = name`
pair is `webref heading`/`dfn` output. `grep` = `/usr/bin/grep`; `git grep` for tree-wide searches. Memory paths are under
`~/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory/`.

### §0.2 Premises re-measured

| # | Claim (item 11 / brief) | Command | Result |
|---|---|---|---|
| B1 | elidex has no `content-visibility` | `git grep -n -i -E 'content.visibility\|ContentVisibility' -- crates` | holds: 0 hits |
| B2 | applies-to = size containment's exclusions | `webref css css-contain-2 content-visibility`; `webref body css-contain-2 containment-size` | holds: "elements for which size containment can apply"; four bullets (no principal box; inner `table`; internal table box; internal ruby box **or a non-atomic inline-level box**) |
| B3 | "non-atomic inline-level box" = F1 | `webref body css-display-3 atomic-inline` | **partial**: an atomic inline "cannot split across lines (as inline boxes **and ruby containers** can)" — the non-atomic inline-level boxes are F1 ∪ inline-level ruby containers; elidex has no `ruby` `Display` variant (`sed -n 5,29p crates/core/elidex-plugin/src/computed_style/display.rs`), so F1 alone is exact at HEAD (P2) |
| B4 | the UA rule `[hidden=until-found i]:not(embed) { content-visibility: hidden; }` (§15.3.1) | `webref body html hidden-elements` | holds verbatim |
| B5 | skipped contents read by paint, hit testing, a11y, focus, selection; computed `visibility` untouched | `webref body css-contain-2 content-visibility` | holds ("not painted (as if they had visibility: hidden)", "do not respond to hit-testing (as if they had pointer-events: none)"; `hidden`: "must not be accessible to user-agent features … nor be selectable or focusable"; §4.6 a11y omission). **Incomplete**: §4.5 names further readers — IntersectionObserver (1), ResizeObserver (2), scrolling operations (5), `innerText` (8), animations/transitions (9–10), top layer (11), media audio (12) — and CSSOM View §6's `scrollIntoView` step 7 and `checkVisibility` step 2 read it (Q1 → U1) |
| B6 | "contents" | same | "the **flat tree** descendants of the element, including both text and elements, **or the replaced content of a replaced element**" — the element's own box is not contents |
| B7 | `details`' UA shadow tree (HTML §15.5.5) | `webref body html the-details-and-summary-elements` | **incomplete**: **three** children — a slot taking the first `summary` child, holding a *default summary* (fallback content) that "is expected to allow the user to request the details be shown or hidden"; a second slot taking the rest, styled `display: block; content-visibility: hidden;` when `open` is absent, `display: block;` when present ("not directly visible to author code. Its impacts, however, are visible"), **expected to match `::details-content`** (css-pseudo-4 §5.2); a style child for `:host summary` (P3, Q2 → O1) |
| B8 | elidex has shadow/slot infrastructure, `elidex-ecs/src/dom/shadow.rs` | `wc -l crates/core/elidex-ecs/src/dom/shadow.rs`; `git grep -n 'distribute_slots' -- crates/css/elidex-style/src/walk.rs` | holds (550 lines; `attach_shadow_with_init` `:171`, manual `slot_assign` `:235`, `assigned_nodes` `:394`, `composed_children` `:529`); named distribution runs in the style walk (`walk.rs:459`, `:582`, `slot.rs:21`). **No flat-tree parent primitive** (`git grep -n 'fn flat_tree_parent\|fn assigned_slot' -- crates` = 0). Registered slots claim gaps in the surfaces a rendered UA tree consumes (Q3 → U2, P6) |
| B9 | "the first UA shadow tree" | `git grep -n -i 'user.agent.shadow\|UaShadow' -- crates` | holds: 0 hits |
| B10 | ancestor revealing on fragment navigation — "elidex has it, `elidex-shell/src/app/navigation.rs:13`" | `sed -n 9,25p crates/shell/elidex-shell/src/app/navigation.rs`; `git grep -n -i 'beforematch\|reveal' -- crates` | **partial (P1)**: `:13` is the app-mode `AppSameDocStep::FragmentNav` variant — same-document fragment navigation exists, in **two** shells (`app/navigation.rs:199` and `content/navigation.rs:462`, both calling `content/scroll.rs:210` `scroll_offset_for_fragment`); the **ancestor revealing algorithm does not exist** (only the handler name `onbeforematch`, `elidex-script-session/src/event_handler_consumer.rs:66`) |
| B11 | (new) HEAD's focus predicate already reads `hidden=until-found` | `sed -n 190,230p;300,320p crates/dom/elidex-dom-api/src/focus/predicate.rs` | `is_focusable` (`:205`) excludes self-or-ancestor `hidden` through `is_in_hidden_subtree` (`:303-317`) — **one** `has_attribute(c, "hidden")` test, no state split; right for an until-found element's descendants, **wrong for the element itself** (it generates a box, so is being rendered, HTML §15.1; only its contents are skipped). Pinned by `focus/tests.rs:451-456` and the doc at `predicate.rs:197-198` |
| B12 | (new) `details` toggle / exclusivity | `sed -n 97,140p crates/script/elidex-js/src/vm/host/html_details_proto.rs` | live only in the VM IDL setter `details_set_open` (`:97`), not in attribute change steps: `setAttribute('open', '')` and the parser fire no `toggle` and close no group member (A96; also a Layering-mandate placement) |
| B13 | (new) `summary` activation behaviour (HTML §4.11.2) | `git grep -n -i summary -- crates/shell` | absent: clicking a `summary` does nothing at HEAD. Activation behaviour of any kind (checkbox, radio, label) lives in the content shell only (`content/event_handlers.rs:115-168` over `content/form_input.rs`); `app/events.rs:17` `handle_click` has none |
| B14 | (new) paint and hit test share one participant walk | `grep -n collect_sc_participants crates/core/elidex-render/src/builder/walk.rs crates/layout/elidex-layout/src/hit_test.rs` | `walk.rs:554`, `hit_test.rs:208`, both `elidex-layout-block/src/paint_order.rs:54`; hit test reads neither `visibility` nor `pointer-events` (elidex has no `pointer-events`: `git grep -n -i pointer.events -- crates` = 0) |
| B15 | (new) a11y walks the composed tree | `grep -n composed_children crates/dom/elidex-a11y/src/tree.rs` | `:49`, `:133` |
| B16 | (new) find-in-page, top layer, `innerText`, `checkVisibility` | `git grep -n -i -E 'find.in.page\|top_layer\|TopLayer\|innerText\|checkVisibility' -- crates` | none exist |
| B17 | (rev 3) slot assignment writers and representation | `sed -n 21,86p crates/css/elidex-style/src/slot.rs`; `sed -n 235,330p crates/core/elidex-ecs/src/dom/shadow.rs`; `git grep -n SlottedMarker -- crates` | **two writers of `SlotAssignment`**: `distribute_slots` (style walk only; no `SlotAssignmentMode` branch — always named/default, so it overwrites a manual assignment) and `slot_assign` (manual; writes `SlotAssignment`, never `SlottedMarker`). The node side is `SlottedMarker` (unit, `components.rs:335`; read by `::slotted` matching, `elidex-css/src/selector/matching.rs:435`) — **no node→slot back-link** (Q-s1 answered) |
| B18 | (rev 3) attribute change steps seam | `sed -n 284,320p crates/script/elidex-js/src/vm/vm_api.rs`; `git grep -n reconcile_attribute_derived_components -- crates/core/elidex-ecs` | the VM's `ConsumerDispatcher` (`consumer_dispatcher.rs:38`, a `MutationDispatcher`, `elidex-ecs/src/dom/mutation_event.rs:333`) is installed at `Vm::bind` (`vm_api.rs:314`) — parser-created nodes before bind reach no consumer. The `elidex-ecs` attribute-write chokepoint calls `reconcile_attribute_derived_components` (`attribute_reconcile.rs:50`) at `attribute.rs:227`, `:308`. HTML §4.11.1: "these attribute change and insertion steps also run when an attribute or element is inserted via the parser" |
| B19 | (rev 3) pseudo-element representation | `sed -n 38,82p crates/css/elidex-style/src/pseudo.rs`; `sed -n 190,200p crates/core/elidex-ecs/src/components.rs` | `::before`/`::after` are DOM child entities of the originating element (`pseudo.rs:63-80`; Q-s11 answered: the tree parent is the originating element); `::marker` is `ListItemMarker` on the `li` itself (`components.rs:199`) |
| B20 | (rev 3) initial-load fragment scroll | `git grep -n 'scroll_offset_for_fragment\|\.fragment()' -- crates/shell` | **absent**: the only callers are the two same-document steps (`app/navigation.rs:199`, `content/navigation.rs:462`); HTML §7.4.6.2 *try to scroll to the fragment* is not implemented (Q-s6 answered) |
| B21 | (rev 3) script-time style/layout freshness | `project_open-defer-slots.md:410` | slot `#11-forced-style-layout-flush-on-script-read` (registered at #388, 2026-06-21; *Trigger* "a handler-stale-geometry test OR a forced-flush program") — open; no forced flush exists |
| B22 | (rev 3) shadow selector matching | `sed -n 65,120p crates/css/elidex-css/src/selector/matching.rs` | `:host` / `:host()` (`:71`, `:116`) and `::slotted()` (`:76`) are parsed and matched for author shadow sheets; what a UA root lacks is a UA-origin shadow sheet applied to it (Q-s2's input) |

### §0.3 Premise errors (reported, not edited — the A114 idiom)

- **P1** (item 11's "elidex has it, `navigation.rs:13`"): B10 — same-document fragment navigation exists in both shells;
  the ancestor revealing algorithm, `beforematch` dispatch and the initial-load path (B20) do not. The program **builds**
  them (CV-reveal); the decided content ("ancestor revealing on fragment navigation") is unchanged.
- **P2** (item 11's applies-to "non-atomic inline-level box … consuming F1"): B3 — exact at HEAD, F1 plus a ruby-container
  arm once a `ruby` display lands; K3 makes that fail-safe (an exhaustive `Display` match).
- **P3** (item 11's "`details`' UA shadow tree"): B7 — the tree has three children and its second slot carries the
  `::details-content` pseudo-element; the pseudo-element is a CSS selector surface item 11 does not name (Q2 → O1).
- **P4** (item 11's reader list): B5 — css-contain-2 §4.5 and CSSOM View name further readers; several exist in elidex
  (Q1). Resolved by U1 (§0.5): item 11's five readers are kept and widened.
- **P5** (the parent's pins, "flip = the content-visibility PR that wires skipped contents into hit testing"): every
  witness sits inside `<div hidden="until-found">`, so it flips only once the UA rule (CV-core) **and** the hit-test reader
  (CV-render) have landed; §5.2's order puts CV-core before CV-render, so the flip event is CV-render, as the pins name it.
- **P6** (U2's slot list, as put to the user): `#11-shadow-composed-event-retargeting` is not open — the 2026-06-14 sweep
  found it implemented (`html-side-defer-sweep-2026-06-14.md:51`; `m4-12-defer-sweep-2026-06-19.md:38`; the open-slot
  ledger's close list, `project_open-defer-slots.md:103`), and `apply_retarget` is at `event_dispatch/mod.rs:627`.
  CV-details consumes it as HEAD's; the roadmap rows that still list it as open (`m4-12-platform-gap-roadmap.md:768`,
  `:1363`) are stale. The other three slots are not in the open-slot ledger at all: they are roadmap rows
  (`m4-12-platform-gap-roadmap.md:1365-1367`, `:1470-1476`; also named at `:1376`).

### §0.4 Decisions

**D1 — Two facts, two compositions, no stored copy.** *Skips its contents* (css-contain-2 §4 `#skips-its-contents`) is
S1(e) = computed `content-visibility` of e is `hidden` ∧ e is an element for which size containment can apply (K3). *In
skipped contents* is S2(n) = some **flat-tree ancestor** of n satisfies S1 (K4). Both are live queries over components that
already hold the inputs (`ComputedStyle`, F1, the tree) — the parent's rule: "One representation per spec fact; one
composition per spec predicate; each representation's writer set closed" (parent `:1167`), components only where "no
other component holds those facts" (`:1197`). No marker is written on contents: it would be a second copy of the cascade's
output with an invalidation obligation on every style or tree change. Writers: none (query), as F1. `auto` adds S1's
second arm later (`#11-content-visibility-auto`). K1 adds the further queries the spec's wording distinguishes.

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
then scroll) paints `x` — so CV-reveal precedes CV-render (§5.2 c2). The same cell fires for a document **loaded** with
`#t`: HEAD paints `x` (no initial-load scroll, B20), the spec's *try to scroll to the fragment* (HTML §7.4.6.2) reaches
*scroll to the fragment* and its reveal step — so CV-reveal builds the initial-load path too. Likewise a closed `details`
whose contents become skipped needs `summary` activation (B13: HEAD paints the contents after a click on `summary` —
vacuously, as it never hides them; the spec paints them because the click opens it), the default summary's activation (a
`details` with no `summary` child: §15.5.5's rendering expectation, K13) and the reveal algorithm's `details` arm; all are
built by CV-details **before or with** the rule that styles the second slot `content-visibility: hidden` (K9). These are
bounded repairs the program's own PRs make necessary — derived by the parent's ownership rule, not a widening of item 11
(O2, §0.5).

**D6 — Attribute writers go through attribute change steps.** The reveal algorithm (remove `hidden`; set `open`) and
`summary` activation (toggle `open`) mutate content attributes through the engine-independent attribute mutation path, so
the element's attribute change steps run; `details`' steps (toggle event task with its tracker, name-group exclusivity —
HTML §4.11.1) move from the VM IDL setter (B12) into `elidex-dom-api`, where every writer reaches them — the setter,
`setAttribute`, activation, reveal **and parser creation** (DOM *append an attribute* → *handle attribute changes*; HTML
§4.11.1's parser sentence, B18), including nodes created before `Vm::bind`. The seam is the `elidex-ecs` attribute
chokepoint family (`reconcile_attribute_derived_components`, `attribute.rs:227`, `:308`) or the mutation flush — CV-details'
plan measures which covers the parser (*input*); it is **not** the VM's `ConsumerDispatcher` (installed at `Vm::bind`, B18).
One home; the setter becomes marshalling (Layering mandate) (K10).

**D7 — The UA shadow tree is a declared creation fact of the element kind.** `HtmlElementHandler`
(`elidex-plugin/src/traits.rs:214`, connected by IBP-classify, parent §1 Home grounds; `elidex-plugin` has no `elidex-ecs`
dependency, its `Cargo.toml` `[dependencies]`) **declares** a UA shadow template — data: the tree's shape and its UA shadow
sheet — and builds nothing. One engine-independent instantiation function, run at the element-creation chokepoint,
instantiates the declared template on every creation path (both parsers, `createElement`, fragment parsing, clone — which
re-creates from the template, never copies); its crate is CV-details' plan's after Q-s10 (*input*), constrained to be
engine-independent and reachable from every creation path. One mechanism for every later UA shadow tree (form controls,
media controls). The shadow root carries a `UserAgentShadowRoot` (NEW) marker; DOM's `ShadowRootMode` stays the spec's two
values (a UA root is never script-observable, so its mode is not a third value) (K12). The second slot's
`content-visibility: hidden` is **not** a stored inline declaration: a rule of the UA shadow sheet keyed on
`:host(:not([open]))` derives it by cascade (D1: no stored copy, no rewrite on `open` change; the sheet needs `:host`
anyway for §15.5.5's third child, `:host summary`). HTML words it "its style attribute set to"; the UA tree is not
script-observable and the spec itself says that attribute is "not directly visible to author code", so the cascade-derived
value has the same impact — a statement of equivalence, not a deviation.

**D8 — Flat-tree parent is one primitive** — `EcsDom::flat_tree_parent` (NEW) in `elidex-ecs/src/dom/shadow.rs`, beside the
shadow/slot primitives it inverts (`composed_children` `:529`, `assigned_nodes` `:394`), read by S2 and by the reveal
algorithm's walk ("parent node within the flat tree", HTML §6.1 step 3). It reads D11's `AssignedSlot`; it is built by
CV-flat-tree (one builder).

**D9 — E20 and E22 bind every nested PR** (item 11's literal: "it follows its inputs, `IBP-predicate` (E20) and
`IBP-css-machinery` (E22)", parent `:997-998`; the ledger's "its implementation PRs follow `IBP-predicate` (E20) and
`IBP-css-machinery` (E22)", `project_open-defer-slots.md:367`) — the literal over the node paragraph's general rule
(`:1381-1382`), as IBP-sandbox's D7 chose for E26 (`ibp-sandbox.md:180`). Data flow, for information: E22's `i` flag is
consumed by CV-core's UA rule; E20's F1 by CV-render's S1.

**D10 — Selection** is read by every user-agent-initiated selection producer (drag, multi-click, select-all). HEAD has none
for document content (Q-s5 answered: the shells' selection code is form-control text editing only,
`content/event_handlers.rs:535-544`; HEAD's `Selection` is script-driven, `elidex-dom-api/src/selection.rs`, and a script
`Range` over skipped nodes is legal). The obligation (producers read S2) is recorded on
`#11-content-visibility-absent-readers` (K17, §8); a form control inside skipped contents cannot be focused (K15), so its
text cannot be user-selected.

**D11 — One slot-assignment writer, at mutation time** (CV-flat-tree). DOM's *assign slottables* (§4.2.2.4) runs at
insertion, at a `slot` attribute change and at manual `assign()`; elidex's assignment is written by one mutation-time
writer in `elidex-ecs` that handles both modes (*find a slot*, §4.2.2.3: manual reads the slot's **manually assigned
nodes**, an input distinct from its computed assigned nodes; named reads names). The style walk's `distribute_slots` (B17)
becomes a reader of that assignment or is deleted (*input*). The node side is `AssignedSlot` (NEW, a component on the
slotted node: DOM's "set slottable's assigned slot to slot", §4.2.2.4 step 4), written atomically with the slot's
`SlotAssignment` by the same writer; it replaces `SlottedMarker` (one representation: `::slotted` matching reads
`AssignedSlot`). `flat_tree_parent` (D8) and every flat-tree reader read it. The assignment-writer part of
`#11-shadow-light-tree-distribution-rendering` moves here from CV-details (§8).

**D12 — Script-time readers read through one forced update point** (CV-flush). A script-time forced style (+layout) update
— one point both shells' pipelines provide — is what a script-time reader of S1/S2 or of geometry calls before reading.
First consumers: the reveal's post-mutation scroll offset (K11) and `focus()` (K15). Whether `getComputedStyle` and the
layout queries join it is CV-flush's own plan (*input*, with Q-s3, Q-s7 and the crate). Discharges
`#11-forced-style-layout-flush-on-script-read` (B21). Rejected: deriving until-found from the attribute (a second
derivation, K1; wrong under author CSS); accepting a stale read plus a render-time blur (a deviation); an external prereq
program (D5's ownership rule: the program's own CV-reveal needs it).

### §0.5 Decisions on rev 1's open questions

- **U1 — USER DECIDED 2026-10-20 (Q1 option (a), widen)** — **amendment of the parent's §0.6 item 11 decided content**
  (recorded, not edited; the A114 idiom; ratified by this memo's approval PR). Item 11's reader list ("paint, hit testing,
  accessibility, focus and selection") becomes: **every skipped-contents reader elidex has reads the one query (K1, K16)**
  (rev 3: "the one query" is K1's set — S1/S2, their hidden arm and C, the distinctions the spec's wording makes, in one
  home; never a second derivation). Added readers present at `a9866498`, each measured:
  - scrolling operations (css-contain-2 §4.5 item 5; CSSOM View §6 `scrollIntoView` step 7): the `ScrollIntoView`
    handler (`elidex-dom-api/src/element/layout_query.rs:257`) and the fragment scroll (`content/scroll.rs:210`);
  - IntersectionObserver (§4.5 item 1): `elidex-api-observers/src/intersection/mod.rs` (583 lines), VM-wired
    (`elidex-js/src/vm/host/intersection_observer.rs`);
  - ResizeObserver (§4.5 item 2): `elidex-api-observers/src/resize.rs` (620 lines), VM-wired
    (`elidex-js/src/vm/host/resize_observer.rs`) — the parent's F13 (IBP-observer) also moves it, hence the edge
    IBP-observer → CV-readers;
  - CSS animations and transitions (§4.5 items 9–10): `elidex-css-anim` (`engine.rs:147` `tick`, `engine.rs:91`
    `add_transition`) and the shell's detection (`elidex-shell/src/animation.rs:87`).
  They are wired by the new nested PR **CV-readers**, after CV-render. Readers elidex lacks — `innerText` (§4.5 item 8),
  `checkVisibility` (CSSOM View §6 step 2), the top layer (§4.5 item 11), media audio suppression (item 12), find-in-page
  (HTML §6.9.2) and (rev 3, D10) user-agent selection producers — carry a **standing obligation** (K17): whoever builds
  one reads K1's query for it; recorded on the slots of §8 in the Gap/Why/Trigger idiom. Commands: `/usr/bin/grep -rln
  'scrollIntoView\|scroll_into_view' crates --include='*.rs'`; `/usr/bin/grep -rln
  'IntersectionObserverRegistry\|ResizeObserverRegistry' crates --include='*.rs'`; B16 for the absent ones.
- **U2 — USER DECIDED 2026-10-20 (Q3)** — CV-details **builds** the shadow surfaces it needs as their first production
  consumer; the non-stale parts move into this program (**the Layout lane owns them from now**), and each slot is closed or
  narrowed at its owning landing (§8's per-slot table: current text, what moves, what remains; rev 3: the assignment
  writer is CV-flat-tree's, D11). Which parts are non-stale is **Q-s2**, answered by CV-details' own plan-review (P6
  settles `#11-shadow-composed-event-retargeting`: closed, nothing moves). Recorded as an amendment of item 11 (the
  `details` UA shadow tree's scope now includes these surfaces), ratified by the approval PR.
- **O1 — decision (lens converged)**: `::details-content` → slot `#11-details-content-pseudo` (NEW; §8): selector machinery
  (a pseudo-element mapped onto a UA shadow element, like `::part`) that elidex lacks, not part of skipping; A96.
- **O2 — decision (lens converged; rev 3)**: D5 (summary activation, the default summary, the `details` attribute change
  steps and the initial-load fragment path inside the program, by the parent's cell test) stands. D9 is rewritten to item
  11's literal (R1 F15): every nested PR follows E20 and E22; the rev-2 per-PR binding ("CV-core and CV-reveal may land
  before IBP-predicate") is withdrawn.

## §1. Program shape and verdicts

| Axis | Own invariant | Intersects |
|---|---|---|
| CSS property | grammar, cascade, computed as specified, discrete animation with the `hidden` mapping (§4.1), UA rule with `i` | applies-to, CSSOM |
| Used-time skip | S1 over the principal box via F1; S2 over the flat tree; the hidden arm and C (K1) | every reader; containment out |
| Flat tree | one mutation-time assignment writer, both modes; `AssignedSlot` back-link; `flat_tree_parent` | the style walk's `distribute_slots`, `::slotted` matching, clone/destroy, `slotchange` |
| Script-time freshness | one forced style(+layout) update point read before a script-time S1/S2 or geometry read | both shells, the style/layout phases, `focus()`, the reveal scroll, `getComputedStyle` (CV-flush's plan) |
| Paint + hit test | contents unpainted / pass-through; owner-entity gate; out-of-flow descendants; pseudo-elements | stacking contexts, PR-1c's inline hits, IBP-layout's F4 sites, U2's slot/fallback emissions |
| A11y | omission (§4.6) | composed-tree walk |
| Focus | being rendered vs skipped; Hidden / Hidden Until Found split; render-time fixup | `reconcile_focus`, the canonical-focus lane (driver-less) |
| Navigation + script | reveal fires `beforematch` (script) and mutates before the scroll; same-document and initial-load; two shells | attribute change steps, CV-flush |
| UA shadow tree | declared template, one instantiation function, never script-observable, UA-styled by cascade; the shadow surfaces it needs (U2) | flat tree, events, serialization, clone, the shadow-lane slots |
| Further readers (U1) | scrolling operations skip; observers see no intersection / no size change; animations frozen, transitions not started | IBP-observer (F13), `elidex-css-anim`'s tick, the shell's transition detection, both shells' scroll |

**Program: edge-dense — divided** (*decision*; CLAUDE.md *Edge-dense work* (a)). Per nested PR (*input*; each plan-review
applies the rule to itself): **CV-flat-tree** may be edge-dense (mutation-time writer × two modes × the style walk's reader
× `SlottedMarker`'s readers × clone/destroy × `slotchange`); **CV-flush** likely edge-dense (engine-wide script-read
freshness × two shells × the style and layout phases × re-entrancy inside event dispatch); **CV-core** likely narrow (one
property, one UA rule); **CV-reveal** may be edge-dense (script event × mutation × two shells × same-document and
initial-load paths); **CV-render** may be edge-dense (owner-entity gating × out-of-flow × replaced/widget content ×
pseudo-elements); **CV-a11y** narrow; **CV-focus** narrow-to-moderate; **CV-readers** may be edge-dense (four reader
families in four crates × the animation semantics; its plan may divide it per family); **CV-details** is a **node** —
likely edge-dense (UA shadow template and instantiation × fallback rendering × slotted rendering × `:host` UA-sheet
matching × activation × attribute change steps × reveal arm × style) — its plan divides it (seeds: UA-shadow mechanism;
the U2 facets — fallback rendering and `:host` UA-sheet matching; activation + steps + reveal arm; the slot style rule
last, K9).

## §2. Coupled invariants (decisions, with landing qualifiers)

- **K1 One home, the spec's distinctions** — the program provides exactly the queries the spec's wording distinguishes, in
  one file (§4): S1/S2; their **hidden arm** S1ʰ/S2ʰ (identical to S1/S2 until `auto` lands); and **C(n)** = n or a
  flat-tree ancestor of n has computed `content-visibility: hidden` (no applies-to), its *ancestor form* C over
  `flat_tree_parent(n)`. C is built by its first consumer, in that file, never inline. No reader derives any of them from
  the `hidden` attribute or reads `content_visibility` directly. From **CV-render** (paint, hit), **CV-a11y**, **CV-focus**
  — which **adds** a state split to `is_in_hidden_subtree` (B11: one attribute test at HEAD): the Hidden state stays the
  attribute test (a `display: none` proxy, IBP-ua-display's rule and `#11-focusable-area-being-rendered`), the Hidden
  Until Found state is read through S2; it flips `focus/tests.rs:451-456` and the doc at `predicate.rs:197-198`.
- **K2 Computed ≠ used** — from **CV-core**: CSSOM serializes the computed value whatever applies-to says; `visibility` and
  `pointer-events` untouched.
- **K3 Applies-to over the principal box** — from **CV-render**: S1 requires a principal box (`display` not `none` /
  `contents`), inner display not `table` (`table`, `inline-table`), not an internal table box (css-display-3 §2.4; caption
  excluded), not an internal ruby box, not a non-atomic inline-level box (F1; plus an inline ruby container, P2). Matched
  exhaustively over `Display`, so a new variant is a compile error until classified (fail-safe: unknown never silently
  applies or not). Blockified display is F1's (the parent's IBP-predicate moves blockification to style resolution, E20).
- **K4 Flat tree** — from **CV-flat-tree**: `flat_tree_parent` maps a slotted node to its `AssignedSlot`, a shadow root's
  child to the host, an unassigned light child of a host to *none* (not in the flat tree), else the DOM parent. The
  assignment is written at mutation time by the one writer (D11), so it is current whenever any reader runs. Text nodes are
  contents (B6).
- **K5 What "contents" is** — flat-tree descendants (including `::before`/`::after`, DOM child entities of the originating
  element, B19) ∪ the element's replaced content (F4, the parent's IBP-classify) ∪ its widget content (elidex paints
  controls without UA shadow trees, `elidex-render/src/builder/form.rs`) ∪ its `::marker` (`ListItemMarker` on the element,
  B19; D3's S1 arm). css-contain-2 §3.1: "treated as having no content—not even through pseudo-elements such as ::before,
  ::after, or ::marker" — decided, no Chromium probe. The element's own box (background, border, outline, its box's hit
  area) is not contents.
- **K6 Traversal independence** — a reader whose traversal is not the flat tree reads S2 per owning entity. Stacking-context
  participants, out-of-flow and positioned descendants (their containing block and stacking context lie outside the S1
  element while `#11-css-containment` is out) are skipped by flat-tree ancestry, not by where the walk meets them.
- **K7 Paint × hit test** — landed together at **CV-render**. Hit testing passes a skipped candidate over and continues to
  what lies beneath ("as if `pointer-events: none`"), so the S1 element's own box can be hit (every parent pin's conformant
  value). The gate sits where every hit candidate and display item passes, so a later producer (PR-1c's inline-box hits,
  umbrella `:1810-1811` as the parent cites it at `:1015-1018`; CV-details' slot and fallback emissions) cannot bypass it.
- **K8 Freshness** — render-time readers (paint, hit, a11y, `reconcile_focus`, `focus/sot.rs:178`) read the last style
  pass, which they run after. Script-time readers (`focus()`, the reveal's post-mutation scroll offset) read through
  CV-flush's update point (D12), so `setAttribute('hidden', 'until-found')` then `inner.focus()` in one task is refused, as
  at HEAD, and `details.open = true` then `inner.focus()` succeeds. A focused node that becomes skipped later is blurred by
  the render-time fixup (K15).
- **K9 Reveal before skip** — no landing state makes an until-found descendant unpainted while fragment navigation
  (same-document or initial-load) cannot reveal it (**c2**), nor a closed `details`' contents unpainted while `summary`
  activation, the default summary's activation or the reveal `details` arm is missing (CV-details' internal order: the
  second slot's `content-visibility: hidden` rule lands last).
- **K10 Writes through attribute change steps** — from **CV-reveal** (`hidden` removal) and **CV-details** (`open`, and
  `details`' steps moved engine-side, D6, reached by every writer including the parser). No writer calls a VM IDL setter.
- **K11 Reveal × script × geometry** — from **CV-reveal**: `beforematch` runs script; the algorithm's re-checks are kept —
  step 4.1 (connected), 4.2.1 (still until-found), 4.2.3 (connected after `beforematch`), 4.2.4 (still until-found after
  it); the scroll offset is computed after the reveal through CV-flush's point, from layout that reflects any mutation (both
  shells, one function; same-document and initial-load).
- **K12 UA shadow tree** — from **CV-details**: instantiated from the declared template at every creation of a `details`
  (parser — both backends —, `createElement`, `cloneNode` — which never copies it but re-creates it —, fragment parsing);
  `UserAgentShadowRoot` marks it; never exposed (`shadowRoot`, `composedPath`, serialization, slot accessors) and styled by
  the UA shadow sheet only.
- **K13 `details` slot assignment** — from **CV-details**, through CV-flat-tree's writer: first `summary` child → slot 1
  (none → the default summary, its fallback content), every other child node → slot 2; current on child-list changes
  because the writer runs at mutation time. How the `details` rule feeds the writer (manual assignment driven by its
  child-list steps, or a UA slot rule the writer reads) is CV-details' plan (*input*). The second slot's style follows
  `open` by cascade (D7). The default summary activates its host by HTML §15.5.5's rendering expectation — not §4.11.2,
  whose step 1 returns because its parent is the slot, not a `details`.
- **K14 Pins** — every parent pin (§5.1) flips at **CV-render**, conditional on firing at its exposer's actual base; CV-render
  flips each landed pin's assertion.
- **K15 Focus** — from **CV-focus**: a node in skipped contents is not a focusable area; the S1 element itself is (it is
  being rendered, HTML §15.1); `focus()` reads through CV-flush's point (K8); a focused node that becomes skipped is blurred
  by HEAD's render-time `reconcile_focus`, which reads the predicate.
- **K16 Every present reader reads K1's query** (U1) — from **CV-readers**: scrolling operations do not scroll to a node in
  skipped contents ("as if it did not have a layout box", §4.5 item 5); IntersectionObserver reports such a target as never
  intersecting, even when root and target are both skipped (item 1); ResizeObserver reports no size change for it and
  delivers on un-skip only if the size differs from the last reported (item 2). Animations, exactly per items 9–10: while
  an element is skipped no new animation is created, existing ones do not advance and running ones do not end; a script
  style query samples them; on un-skip they are sampled and resume; no transition starts while skipped, nor from the
  un-skip style change event. Un-skip detection: the caller (elidex-shell — `animation.rs:87`
  `detect_and_start_transitions`, called at `lib.rs:692` after the style pass with pre-pass styles, and the event-loop
  tick, `content/event_loop.rs:256`) evaluates S2 **before** the style pass into a reader-local per-pass set, dropped with
  the pass (never a component); freeze = the tick does not advance skipped entities (no stored freeze state).
- **K17 Standing obligation for absent readers** (U1) — each, when built, reads K1's form the spec names: `innerText` → S2;
  `checkVisibility` (CSSOM View §6 step 2) → C's ancestor form; the top layer (§4.5 item 11, whose Note excludes `auto`) →
  C's ancestor form; media audio (item 12) → self computed `hidden` or S2ʰ (`auto` excluded); find-in-page → S2 plus the
  reveal; user-agent selection producers (D10) → S2. Recorded on the slots of §8, not as code here.

Pairs: K1×K6 (one answer, read per owner); K2×K3 (applies-to never feeds the computed value); K3×E20 (F1's blockified
display at style time, so S1 agrees before and after layout); K4×K8 (the assignment is written at mutation time, so a
script-time reader of `flat_tree_parent` needs no style pass); K4×K11 (the reveal walk runs at navigation time over a
current assignment); K4×K12 (UA slots are flat-tree parents; the reveal walk finds the `details` arm through them); K4×K13
(assignment decides `flat_tree_parent`; one writer); K5×F4/F7 (replaced and widget content are the parent's
classifications); K6×`#11-css-containment` (when containment lands, the ancestry rule still holds); K6/K7×U2 (slotted nodes
painted at their slot and fallback content pass the same owner-entity gate — hence CV-render → CV-details); K7×PR-1c (new
candidates pass the gate); K8×K11 (reveal's scroll reads through CV-flush); K8×K15 (script-time `focus()` reads through
CV-flush; render-time fixup for later changes); K9×K10 (activation and reveal write `open` through the steps; the slot
style follows by cascade); K10×B12 (the IDL setter becomes marshalling over the same steps); K14×§5.2 (CV-core precedes
CV-render, so the hit-test reader is the flip, P5); K16×F13 (ResizeObserver's observed box is the parent's IBP-observer's;
CV-readers gates its observation after it); K16×K8 (observer and animation readers run in the rendering update, after the
style pass; the un-skip set is taken before it); K16×K11 (the fragment scroll is both a reveal caller and a scrolling
operation: reveal first, then the skip check — a target still skipped after reveal, under an author
`content-visibility: hidden`, is not scrolled to); K17×K1 (the absent readers use K1's queries; C lives in K1's file).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSS CONTAIN 2 §4 Suppressing An Element’s Contents Entirely: the content-visibility property | property definition | `visible`, `hidden`; `auto` → `#11-content-visibility-auto` | CV-core | ✗ | yes |
| CSS CONTAIN 2 §4 Suppressing An Element’s Contents Entirely: the content-visibility property | *skips its contents*: contents unpainted, not hit-tested | used `contain` → `#11-css-containment`; layout-skipping SHOULD → out (§8) | CV-render | ✗ | yes |
| CSS CONTAIN 2 §4 Suppressing An Element’s Contents Entirely: the content-visibility property | `hidden`: not accessible to UA features, not selectable, not focusable | focus: CV-focus; selection producers, find-in-page → K17's slots | CV-focus | ✗ | yes |
| CSS CONTAIN 2 §4.1 Animating and Interpolating content-visibility | discrete; `hidden` ↔ other maps (0,1) to the non-hidden value | | CV-core | ✓ | yes |
| CSS CONTAIN 2 §3.1 Size Containment | the four "no effect" bullets (applies-to) | ruby arm vacuous at HEAD (P2) | CV-render | ✓ | yes |
| CSS CONTAIN 2 §3.1 Size Containment | "not even through pseudo-elements such as ::before, ::after, or ::marker" | all three are contents (K5) | CV-render | ✓ | yes |
| CSS CONTAIN 2 §4.5 Restrictions and Clarifications | items 1–12 | 1, 2, 5, 9, 10: CV-readers (U1); 8, 11, 12 → K17's slots (absent in elidex); 3, 4, 6 `auto`-only (3 in effect: *update content relevancy for a document* step 2 iterates only elements with `auto` used value) → `#11-content-visibility-auto`; 7 (iframe SHOULD) → out | CV-readers | ✗ | yes |
| CSS CONTAIN 2 §4.6 Accessibility Implications | `hidden`'s skipped contents omitted | `auto` paragraph → `#11-content-visibility-auto` | CV-a11y | ✓ | yes |
| CSS DISPLAY 3 §2.4 Layout-Internal Display Types: the table-* and ruby-* keywords | *internal table box*, *internal ruby box* | | CV-render | ✓ | yes |
| WHATWG DOM §4.2.2.3 Finding slots and slottables | *find a slot* (manual vs named), *find slottables* | both modes in one writer (D11) | CV-flat-tree | ✓ | yes |
| WHATWG DOM §4.2.2.4 Assigning slottables and slots | *assign slottables* (step 4: the slottable's assigned slot), *assign slottables for a tree*, *assign a slot* | `AssignedSlot` (D11) | CV-flat-tree | ✓ | yes |
| WHATWG HTML §15.3.1 Hidden elements | `[hidden=until-found i]:not(embed)` | the section's other rules → IBP-ua-display / IBP-layout (parent) | CV-core | ✗ | yes |
| WHATWG HTML §6.1 The hidden attribute | *ancestor revealing algorithm* steps 1–4 | 3.1 / 4.2 (until-found): CV-reveal; 3.2 / 4.3 (`details`): CV-details | CV-reveal, CV-details | ✓ | yes |
| WHATWG HTML §7.4.6.4 Scrolling to a fragment | *scroll to the fragment* 3.4 (reveal) before 3.5 (scroll) | 3.6 / 3.7 focusing → `#11-fragment-navigation-focusing-step`, not this program | CV-reveal | ✗ | yes |
| WHATWG HTML §7.4.6.2 Updating the document | *try to scroll to the fragment* (initial load) | absent at HEAD (B20); built with its reveal (D5) | CV-reveal | ✓ | yes |
| WHATWG HTML §6.9.2 Interaction with details and hidden=until-found | find-in-page | → `#11-find-in-page` (B16) | — | ✗ | yes |
| WHATWG HTML §15.5.5 The details and summary elements | the internal shadow tree, three children; the second slot's style by `open` | cascade-derived (D7); `::details-content` → `#11-details-content-pseudo` (O1) | CV-details | ✗ | yes |
| WHATWG HTML §15.5.5 The details and summary elements | the default summary "is expected to allow the user to request the details be shown or hidden" | the host's UA activation (K13) | CV-details | ✓ | yes |
| WHATWG HTML §4.11.1 The details element | attribute change steps (toggle task + tracker; exclusivity); insertion steps; run for parser insertion | | CV-details | ✓ | yes |
| WHATWG HTML §4.11.2 The summary element | activation behavior steps 1–3 | step 1 excludes the default summary (K13) | CV-details | ✓ | yes |
| WHATWG HTML §6.6.2 Data model | focusable-area criteria (being rendered) | the Hidden arm → IBP-ua-display / `#11-focusable-area-being-rendered` | CV-focus | ✗ | yes |
| WHATWG HTML §15.1 Introduction | *being rendered* | the S1 element itself is | CV-focus | ✓ | no |
| CSS PSEUDO 4 §5.2 Expandable contents of details element: the ::details-content pseudo-element | | → `#11-details-content-pseudo` (O1) | — | ✗ | yes |
| CSSOM VIEW 1 §6 Extensions to the Element Interface | `scrollIntoView` step 7 (*not available to user-agent features*) | | CV-readers | ✓ | yes |
| CSSOM VIEW 1 §6 Extensions to the Element Interface | `checkVisibility` step 2 | absent (B16) → `#11-content-visibility-absent-readers` (K17: C's ancestor form) | — | ✗ | yes |

**Breadth**: K=6 (css-contain-2, css-display-3, css-pseudo-4, cssom-view-1, dom, html), M=25 → split by default (preflight
SPLIT-DEFAULT); the split is §5 (nine nested PRs, CV-details a node). The CSS-module rows are `webref heading` output;
preflight's §3 hard-gate does not map CSS labels (`#11-preflight-css-module-labels`), so those rows were checked by hand.

### §3.1 User-input touch audit

The `content-visibility` declaration (author CSS, new parse surface); the `hidden` attribute's value (case-insensitive
`until-found` through the `i` flag); the fragment of a navigated or loaded URL (selects the reveal target); `details`'
`open` and `name` attributes and its child list (assignment); a light child's `slot` attribute and `slot.assign()` (the
one writer, D11); a click or key activation on `summary` or the default summary. New behaviour only hides or reveals
rendering; no new network, storage or permission surface. `beforematch` is a new script entry point fired by navigation:
its re-checks (K11) bound what a handler can make the algorithm do. Adjacent pre-existing: `details_set_open`'s synchronous
`toggle` dispatch (B12) — exposure unchanged until CV-details moves it to a queued task.

## §4. Facts → mechanisms (Home/kind lines decided; the rest input)

- **Computed value** (CV-core) — *Home/kind*: `ComputedStyle::content_visibility: ContentVisibility` (NEW; `Visible |
  Hidden`, `Auto` added by `#11-content-visibility-auto`) in `elidex-plugin/src/computed_style/`; parsed and cascaded by a new
  CSS plugin crate `elidex-css-contain` (NEW) — the css-contain-2 module's home, where `#11-css-containment`'s `contain` and
  `contain-intrinsic-size` later land (*Plugin-first*). Input: registration through the existing property-handler seam
  (precedent `elidex-css-multicol`); CSSOM serialization; §4.1's interpolation in `elidex-css-anim/src/interpolate.rs` beside
  `visibility`'s special case (`:24`).
- **UA rule** (CV-core) — *Home/kind*: the core UA sheet, `elidex-style/src/ua.rs` (436 lines), one rule. Input: consumes
  IBP-css-machinery's `i` flag (E22) and the existing simple `:not(embed)`.
- **Slot assignment** (CV-flat-tree) — *Home/kind*: one mutation-time writer in `elidex-ecs` (D11), run at insertion, `slot`
  attribute change and `slot_assign`; `SlotAssignment` on the slot and `AssignedSlot` (NEW) on the slotted node, written
  together; `SlottedMarker` deleted. Input: the writer's file (beside `shadow.rs`, 550 lines); whether `distribute_slots`
  becomes a reader or is deleted; `slotchange` signalling (DOM §4.2.2.5) for named mode.
- **Flat-tree parent** (CV-flat-tree) — *Home/kind*: `EcsDom::flat_tree_parent(node)` (NEW) in `elidex-ecs/src/dom/shadow.rs`
  (D8), reading `AssignedSlot`.
- **Script-time update point** (CV-flush) — *Home/kind*: one forced style(+layout) update point both shells' pipelines
  provide to script-time readers (D12) — one function, not a per-reader flush. Input: its crate and signature, which
  readers join (`getComputedStyle`, layout queries), re-entrancy during event dispatch (Q-s3, Q-s7).
- **S1, S2, S1ʰ/S2ʰ, C** (CV-render; C by its first consumer) — *Home/kind*: `EcsDom::skips_contents(element)` and
  `EcsDom::in_skipped_contents(node)` (NEW), with the hidden-arm forms and C, queries (no writer), all in one new file
  `elidex-ecs/src/dom/content_visibility.rs` (NEW): K3 over `ComputedStyle` and F1 (`EcsDom::is_inline_box`, the parent's
  IBP-predicate); S2 over `flat_tree_parent` and S1. Input: the internal-table classification is its own (css-display-3
  §2.4), not `Display`'s `is_table_internal` (`display.rs:35`, which serves `visibility: collapse`); bounded by
  `MAX_ANCESTOR_DEPTH`; per-frame memoisation only if a reader's plan measures the need, and then reader-local, dropped with
  its pass (never a component).
- **Ancestor revealing** (CV-reveal; `details` arm CV-details) — *Home/kind*: one engine-independent function,
  `elidex_dom_api::reveal::run_ancestor_revealing` (NEW), over `flat_tree_parent`, attribute mutation (K10) and an event-fire
  callback the caller supplies (the shells dispatch `beforematch` through the session boundary). Callers: both shells'
  same-document fragment navigation and the initial-load *try to scroll to the fragment* (NEW, both shells), before the
  scroll; later find-in-page (`#11-find-in-page`). Input: the Hidden Until Found state classifier's engine-independent home
  (at HEAD only the VM's `HiddenKind`, `html_element_proto.rs:679`).
- **Paint reader** (CV-render) — *Home/kind*: the gate at the display-list builder's per-owner emission
  (`elidex-render/src/builder`), reading S2 for an item's owning entity and S1 for an element's replaced/widget content and
  `::marker` (D3, K5). Input: which of `walk.rs` / `inline.rs` / `inline_flow.rs` / `form.rs` emit per owner (Q-s9).
- **Hit-test reader** (CV-render) — *Home/kind*: the candidate test in `elidex-layout/src/hit_test.rs` (pass over, continue,
  K7). Input: whether the gate belongs in `collect_sc_participants` (shared with paint, `paint_order.rs:54`) or in each
  consumer — a participant list filtered for both readers would also drop the S1 element's skipped out-of-flow
  descendants from painting, which K6 requires anyway.
- **A11y reader** (CV-a11y) — *Home/kind*: `elidex-a11y/src/tree.rs`'s composed walk (`:49`, `:133`), omitting nodes with S2
  (pruning at S1 is valid: the walk is the composed tree, D3). Input: a `details`' default summary appears as its summary.
- **Focus reader** (CV-focus) — *Home/kind*: `is_focusable` (`elidex-dom-api/src/focus/predicate.rs:205`) with
  `is_in_hidden_subtree` split by state, the Hidden Until Found state read through S2 (K1, K15), `focus()` through CV-flush.
  Input: the second predicate `is_focusable_area` (`delegate.rs:133`) and the sequential-navigation order builder (Q-s8);
  `focus/tests.rs` is 999 lines — a touch-time split precedes any added cell (standalone prereq, CLAUDE.md).
- **Selection** — D10 (K17's slot; no code).
- **`details` attribute change steps** (CV-details) — *Home/kind*: `elidex-dom-api` (beside `element/details_exclusion.rs`),
  run for every writer through the `elidex-ecs` attribute chokepoint family or the mutation flush (D6); the toggle event is
  **queued** as an element task with the tracker (HTML §4.11.1), the VM keeps only dispatch marshalling.
- **`summary` activation** (CV-details) — *Home/kind*: one engine-independent activation home that both shells' click/key
  paths reach (not `content/event_handlers.rs` alone — `app/events.rs` has no activation, B13), calling the `elidex-dom-api`
  function that toggles `open` (K10); the default summary's activation (K13) is its host's UA behaviour there. Input: the
  home's crate; whether the checkbox/radio/label activation joins it (*One issue, one way*); how a click on the default
  summary reaches it (event retargeting — HEAD's `apply_retarget`, P6).
- **Scrolling-operation reader** (CV-readers) — *Home/kind*: one `elidex-dom-api` "scroll a target into view" resolution,
  called by `ScrollIntoView` (`layout_query.rs:257`) and by the shell fragment scroll (`content/scroll.rs:210`), reading S2
  for the target (K16). Input: the other scroll-into-view callers (focus) its plan measures (Q-s12).
- **IntersectionObserver reader** (CV-readers) — *Home/kind*: the intersection computation in
  `elidex-api-observers/src/intersection/mod.rs` (`gather_observations` `:270`) treats a target with S2 as non-intersecting.
- **ResizeObserver reader** (CV-readers) — *Home/kind*: the observation-gathering step in `elidex-api-observers/src/resize.rs`
  (`gather_observations` `:235`) skips targets with S2 and keeps their last reported size (K16), over IBP-observer's F13.
- **Animation reader** (CV-readers) — *Home/kind*: the caller in elidex-shell (K16: `animation.rs:87`, `lib.rs:692`, the tick)
  evaluates S2 — `elidex-css-anim` does not depend on `elidex-ecs` (its `[dependencies]`: `elidex-plugin`, `elidex-css`,
  `cssparser`) — passing the engine a skipped set per pass; the un-skip set is reader-local and per pass. Input: the
  engine's API shape; `engine.rs` is 809 lines — 1000-line watch.
- **Absent readers** (K17) — no code; the obligation lives on §8's slots.
- **UA shadow tree** (CV-details) — *Home/kind*: a template declared on `HtmlElementHandler` (data, D7); one engine-independent
  instantiation function at the creation chokepoint (crate *input*, Q-s10); `UserAgentShadowRoot` (NEW) marker in
  `elidex-ecs/src/components.rs` (822 lines); the second slot's style a `:host(:not([open]))` rule of the UA shadow sheet,
  derived by cascade (K12).

## §5. Nested PRs

### §5.1 List and reader wiring (decided)

| Nested PR | Scope |
|---|---|
| **CV-flat-tree** | D11: the one mutation-time slot-assignment writer (both modes), `AssignedSlot` replacing `SlottedMarker`, `distribute_slots` a reader or deleted; `flat_tree_parent` (D8, K4) |
| **CV-flush** | D12: the script-time forced style(+layout) update point; first consumers K11's reveal scroll (CV-reveal) and K15's `focus()` (which PR wires the call is *input*); discharges `#11-forced-style-layout-flush-on-script-read` |
| **CV-core** | the property (D2, §4 computed value), §4.1 interpolation, the `[hidden=until-found i]:not(embed)` UA rule |
| **CV-reveal** | the ancestor revealing algorithm's until-found arm, `beforematch`, K10/K11; wired into both shells' same-document fragment navigation and the initial-load *try to scroll to the fragment* (built, D5) |
| **CV-render** | S1 (K3, F1), S2 (K4); paint and hit-test readers (D4, K5–K7); flips every landed parent pin (K14) |
| **CV-a11y** | a11y omission (§4.6) |
| **CV-focus** | the focus reader (K1's state split, K15) |
| **CV-details** (node) | the shadow surfaces it consumes (U2; §8's per-slot table); the UA shadow template and instantiation (D7, K12); `details`' assignment rule (K13); attribute change steps (D6); `summary` and default-summary activation and the reveal `details` arm (D5); the second slot's style rule, last (K9) |
| **CV-readers** | U1: scrolling operations, IntersectionObserver, ResizeObserver, animations/transitions (K16) |

**Reader → wiring PR** (the flip event the parent's pins name): **paint — CV-render; hit testing — CV-render; accessibility
— CV-a11y; focus — CV-focus; selection — whoever builds a user-agent selection producer (K17; D10, none at HEAD)**;
**scrolling operations, IntersectionObserver, ResizeObserver, animations/transitions — CV-readers** (U1); `innerText`,
`checkVisibility`, top layer, media audio, find-in-page — whoever builds them (K17).

**The parent's pinned cells** (parent `:1001-1023`; each recorded by its exposer only if it fires at the exposer's actual
base): IBP-sandbox (`<noscript>` witness; its node pins it at N9, IBP-sandbox §5.2), IBP-css-machinery (`[data-k=V i]`),
IBP-layout (`button` overflow), PR-1b, PR-1c, PR-1d — **all hit-test cells; flip = CV-render**. IBP-sandbox,
IBP-css-machinery and IBP-layout precede CV-render (E20's chain: IBP-sandbox → … → IBP-layout → IBP-predicate → CV-render),
so their pins fire whenever their witnesses do; PR-1b–PR-1d are unordered with CV-render, so each pins only if CV-render has
not landed at its base. CV-render's plan enumerates the landed pins by a recorded marker (§7) and flips each assertion to
the conformant value (the `div`). Ruled out by the parent: PR-1a, IBP-ua-display.

**Own deferrals per nested PR** (cap 3): every nested PR — 0 expected: the slots each registers or appends to (§8) are
pre-existing class (A96); a cell one of them fires inside this program is repaired inside it (D5).

### §5.2 Edges (decided; the parent's one rule)

| # | Edge | Kind |
|---|---|---|
| E22 | IBP-css-machinery → **every nested PR** | item 11's literal (D9); data flow into CV-core (the `i` flag) |
| E20 | IBP-predicate → **every nested PR** | item 11's literal (D9); data flow into CV-render (F1, S1's applies-to) |
| E26 | this memo's approval → IBP-sandbox's first nested landing | the parent's edge — a docs artefact |
| c1 | CV-core → CV-render | data flow (the computed value S1 reads); also makes CV-render the flip of the `until-found` pins (P5) |
| c2 | CV-reveal → CV-render | A93 (D5's fragment cells, same-document and initial-load) |
| c3 | CV-render → CV-a11y, CV-focus | data flow (S1, S2) |
| c4 | CV-reveal → CV-details; CV-core → CV-details | data flow (the reveal function its `details` arm extends; the property its slot style uses) |
| c5 | IBP-classify → CV-details | data flow (the `HtmlElementHandler` seam IBP-classify connects, D7) — **an addition to the parent's edge list** (§8 records) |
| c6 | CV-render → CV-readers | data flow (S1, S2) |
| c7 | IBP-observer → CV-readers | data flow (F13, the observed box the ResizeObserver gate reads) — **an addition to the parent's edge list** (§8 records); U1 |
| c8 | CV-reveal → CV-readers | data flow (K16×K11: the fragment scroll's skip check sits after the reveal call CV-reveal places) |
| c9 | CV-flat-tree → CV-reveal, CV-render, CV-details | data flow (`flat_tree_parent`, `AssignedSlot`, the one writer K13 feeds) |
| c10 | CV-flush → CV-reveal, CV-focus | data flow (the update point K11's scroll and K15's `focus()` read through) |
| c11 | CV-render → CV-details | data flow (U2's slot and fallback paint emissions pass K7's gate; S1, S2) |

Acyclic: the program precedes nothing (item 12); its inputs (IBP-css-machinery, IBP-predicate, IBP-classify, IBP-observer)
have no predecessor in it; E26 is a docs artefact. Inside, one topological order is CV-flat-tree, CV-flush, CV-core,
CV-reveal, CV-render, CV-a11y, CV-focus, CV-readers, CV-details: every edge c1–c11 points forward in it (CV-flat-tree and
CV-flush have no in-program predecessor; CV-details follows CV-render by c11, so K9 holds inside CV-details — its slot
style rule takes visible effect at its own landing and lands last). CV-focus after CV-core transitively (c1, c3), so its
until-found descendants stay excluded at its landing (B11's right cells).

### §5.3 Named risks per nested PR (input lines)

- **CV-flat-tree**: `SlottedMarker`'s readers (`::slotted` matching, `matching.rs:435`; retargeting per its doc) move to
  `AssignedSlot`; `tree_clone.rs:30` lists `SlotAssignment` and `SlottedMarker` as never copied — `AssignedSlot` joins;
  destroy and re-parent clear the back-link atomically; `slotchange` for named mode may be newly observable; the manual
  list vs computed assigned nodes (D11); `distribute_slots`' two walk call sites (`walk.rs:459`, `:582`).
- **CV-flush**: both shells' pipelines; style-only vs style+layout per reader; re-entrancy (a flush inside a `beforematch`
  handler or event dispatch); which readers join (Q-s3); Q-s7; cost of a forced pass; `#11-forced-style-layout-flush-on-script-read`'s
  trigger text ("a handler-stale-geometry test") as a DoD cell.
- **CV-core**: CSSOM `getComputedStyle(el).contentVisibility` is the only observable change; `:not(embed)` against IBP-ua-display's `[hidden]:not([hidden=until-found i]):not(embed)` (the two rules partition `[hidden]`); `computed_style/tests.rs` 947 lines.
- **CV-reveal**: event dispatch from both shells mid-navigation; the initial-load path's task model (§7.4.6.2's queued
  global task and its retry); scroll geometry after mutation through CV-flush (K11); the Hidden Until Found classifier's
  home (§4).
- **CV-render**: per-owner gating sites (Q-s9); pseudo-elements (K5: `::marker` through S1, `::before`/`::after` through S2);
  out-of-flow (K6); replaced content vs the element's box for `<img hidden="until-found" src=a.png>` (the parent's no-pin
  note, `:1024-1025`); `builder/mod.rs` is 1002 lines (split before touching), `builder/walk.rs` 836 and `hit_test.rs` 764;
  the pin flips.
- **CV-a11y**: the walk's handling of unassigned light children (K4).
- **CV-focus**: two predicates (Q-s8); `focus/tests.rs` 999 lines (split first); the canonical-focus lane is driver-less
  (§9); the flipped cells (K1).
- **CV-readers**: the animation semantics (items 9–10) against `elidex-css-anim`'s tick model and the shell caller; the app
  shell's tick site; ResizeObserver's "deliver on un-skip only if the size differs" against F13 (IBP-observer);
  IntersectionObserver's both-skipped case; the scroll callers' population (Q-s12); may divide per family.
- **CV-details**: the template / instantiation crate (Q-s10); the shadow surfaces it builds (U2, Q-s2; B22: `:host` matching
  exists for author shadow sheets); the steps' seam and parser coverage (D6); `toggle` moves from synchronous to queued (an
  author-visible timing change, toward the spec); the activation home both shells reach (`content/event_handlers.rs` 966
  lines); `focus/predicate.rs`'s `is_first_summary_of_details` (`:122`) still holds (the summary stays a DOM child);
  `details_set_open` becomes marshalling.

## §6. ECS-native and Layering checks (seed; input)

**OO → ECS**: Blink's `DisplayLockContext` per element (a registry with lock state and update flags) → no state: S1/S2
queries over `ComputedStyle` and the tree (D1); "contents are locked" flags propagated by a lifecycle → flat-tree ancestry
read on demand (D3); Blink's details-element hook run when its UA shadow root is added (a virtual) → a template declared on
the element-kind plugin seam, instantiated by one function, + a `UserAgentShadowRoot` marker (D7); a slot's per-node
"assigned slot" pointer kept by the slot object → `AssignedSlot` component on the node, one writer (D11); the per-element
"style attribute set to" rewrite → a cascade-derived `:host(:not([open]))` rule (D7); animation "paused while locked"
state → the tick not advancing skipped entities, a per-pass un-skip set (K16); the details' toggle task tracker held on
the element object → an element-task tracker in the `elidex-dom-api` steps (input: a component on the `details` entity,
`Send + Sync`, per the side-store rule).

**Fact → existing API → crate** (a9866498; reuse before adding):

| Fact | Existing API | Crate |
|---|---|---|
| mutation notification | `MutationDispatcher` (`dom/mutation_event.rs:333`); the VM's `ConsumerDispatcher` (`vm/consumer_dispatcher.rs:38`, installed at `Vm::bind`, `vm_api.rs:314`) | elidex-ecs; elidex-js |
| attribute-write chokepoint | `reconcile_attribute_derived_components` (`dom/attribute_reconcile.rs:50`; called `dom/attribute.rs:227`, `:308`) | elidex-ecs |
| shadow attach / manual assign | `attach_shadow_with_init` (`dom/shadow.rs:171`), `slot_assign` (`:235`) | elidex-ecs |
| named distribution | `distribute_slots` (`slot.rs:21`) | elidex-style |
| slotted-ness | `SlottedMarker` (`components.rs:335`), read at `selector/matching.rs:435` | elidex-ecs; elidex-css |
| `details` name group | `collect_open_details_by_name` (`element/details_exclusion.rs:61`) | elidex-dom-api |
| form activation | `toggle_checkbox_if_needed`, `handle_label_click` (`content/form_input.rs:18`, `:48`); `toggle_radio` (`radio.rs:59`) | elidex-shell (content only); elidex-form |
| observer gathering | `gather_observations` (`intersection/mod.rs:270`, `resize.rs:235`) | elidex-api-observers |
| transitions / ticking | `detect_and_start_transitions` (`animation.rs:87`, called `lib.rs:692`); `AnimationEngine`'s `tick` (`engine.rs:147`, called `content/event_loop.rs:256`) | elidex-shell; elidex-css-anim |
| focus | `is_focusable` (`focus/predicate.rs:205`), `reconcile_focus` (`focus/sot.rs:178`) | elidex-dom-api |
| fragment scroll | `scroll_offset_for_fragment` (`content/scroll.rs:210`) | elidex-shell |

Layering: queries, the assignment writer and `flat_tree_parent` in `elidex-ecs`; reveal, `details` steps, activation logic
and the scroll-into-view resolution in `elidex-dom-api`; the UA template declared in `elidex-plugin`, instantiated
engine-independently; readers in their crates; the VM marshals (`details_set_open`, `beforematch` handler compile only);
shells call one reveal function, one activation home and CV-flush's point, and dispatch through the session.

## §7. Test and coverage plan (input)

**Declared surface**: css-contain-2 `content-visibility` `visible | hidden` (not `auto`), §4.6; HTML §15.3.1's until-found
rule, §6.1's revealing algorithm (same-document and initial-load fragment navigation), §15.5.5, §4.11.1, §4.11.2; DOM
§4.2.2.3–§4.2.2.4 slot assignment. WPT seeds (each PR's plan picks): `css/css-contain/content-visibility/`
(`content-visibility-0xx` hidden cases, `content-visibility-computed`/`-valid`/`-invalid`),
`html/editing/the-hidden-attribute/` (`beforematch`), `html/semantics/interactive-elements/the-details-element/`,
`shadow-dom/` slotting (CV-flat-tree). Cells (owner): C1 computed value + serialization (CV-core); C2 the until-found rule
incl. `HIDDEN=UNTIL-FOUND` (CV-core); C3 fragment reveal: `beforematch` fired, attribute removed, scroll after — same-document
and initial load, both shells (CV-reveal); C4 a handler that disconnects or re-hides (K11's four re-checks; CV-reveal); C5
paint: contents unpainted, the element's border painted, `<img hidden=until-found>`'s image unpainted, `::before`/`::after`
and an `li`'s `::marker` unpainted (CV-render); C6 abspos/z-index descendant skipped (CV-render); C7 hit passes through to
the S1 element (CV-render); C8 applies-to: `display: inline`/`contents`/`table`/`table-cell` no effect, `inline-block`,
`table-caption`, an inline `img` applies (CV-render); C9 the parent pins flipped (CV-render); C10 a11y omission (CV-a11y);
C11 `<button hidden=until-found tabindex=0>` focusable, its descendants not, render-time blur (CV-focus); C12 `details`:
closed contents unpainted, click opens, a summary-less `details` opens by clicking its default summary, `toggle` queued
once, name-group exclusivity on `setAttribute` and on a parsed `<details open name>`, fragment into closed `details` opens
it, no `shadowRoot` exposed, clone re-creates (CV-details); C13 `scrollIntoView()` on a skipped node and a fragment into
author `content-visibility: hidden` do not scroll; C14 IntersectionObserver reports a skipped target non-intersecting; C15
ResizeObserver silent while skipped, delivers on un-skip only on a size change; C16 while skipped a running animation does
not advance or end and no animation or transition starts; on un-skip the animation resumes and no transition starts
(CV-readers); C17 named assignment current after insertion and after a `slot` attribute change without a style pass, a
manual root's assignment survives the style walk, `::slotted` matches through `AssignedSlot` (CV-flat-tree); C18
`setAttribute('hidden','until-found')` then `inner.focus()` in one task refused; `details.open = true` then `inner.focus()`
succeeds; a `beforematch` handler's mutation is reflected in the scroll offset (CV-flush with its first readers). WPT seeds:
`content-visibility-vs-scrollIntoView-00x`, `content-visibility-resize-observer-no-error`, the hidden-subtree animation tests.

## §8. Out of scope, dispositions, records

**Out** (item 11): layout skipping (the spec's SHOULD); container-side containments (`#11-css-containment`); `auto`
(`#11-content-visibility-auto`); css-contain-2 §4.5 item 7 (an iframe's update-the-rendering skip, a SHOULD).

**Registration timing** (the IBP-sandbox §8 rule): a gap that exists at HEAD (A96) is registered **at approval**
(re-eval = approval merge + 1 month); a gap created by a nested PR's code at that landing (+1 month).

**Slots**:
- `#11-content-visibility-skip-contents` (home, opened 2026-09-30, `project_open-defer-slots.md:367`): its trigger ("the
  approval PR lands — authoring of its umbrella memo starts") has fired; restate at approval: *Trigger* = each nested
  landing in §5.2's order; discharged by the last; re-eval per the timing rule. This memo is its umbrella.
- `#11-forced-style-layout-flush-on-script-read` (`project_open-defer-slots.md:410`, #388) — restate at approval: owned by
  this program's CV-flush (D12), *Trigger* = CV-flush's landing; **discharged by CV-flush** (its readers beyond the first
  two, if CV-flush's plan leaves any, are recorded at that landing).
- `#11-css-containment` — append (at approval): when it lands, S1's element gets the used `contain` value (size, layout,
  style, paint); K6's ancestry rule stays the readers' rule.
- `#11-content-visibility-auto` — append: S1's `auto` arm (not relevant to the user); relevance reads focus, selection and
  the top layer (`#11-dialog-top-layer`); §4.5 items 3, 4, 6; (rev 3) readers that are hidden-arm-only — the top layer
  (item 11) and media (item 12) — must use K1's hidden-arm form, not S1/S2, once `auto` lands.
- `#11-find-in-page` (NEW, at approval): *Gap*: elidex has no find-in-page (B16); *obligations*: HTML §6.9.2's temporary
  un-skipping of until-found and closed-`details` contents during the search, and its reveal call through the one reveal
  function; reads S2 (K17). *Why deferred*: no find-in-page feature exists to wire; A96. *Trigger*: a find-in-page feature
  or `window.find`. *Re-eval*: approval merge + 1 month.
- `#11-details-content-pseudo` (NEW, **at approval**, O1): *Gap*: the `::details-content` pseudo-element (css-pseudo-4 §5.2)
  — HTML §15.5.5 expects `details`' second slot to match it; elidex parses and matches no such pseudo-element. *Why
  deferred*: it is selector machinery elidex lacks (a pseudo-element mapped onto an element of a UA shadow tree, the `::part`
  shape), not part of skipping; A96 — HEAD matches nothing and CV-details leaves that unchanged, so no cell fires. *Trigger*:
  CV-details has landed **and** (a css-pseudo-4 `::details-content` WPT subset is declared supported, or a pseudo-element
  onto a UA shadow element is built for another element). *Re-eval*: the approval PR's merge date + 1 month (the timing rule
  above; the gap exists at HEAD).
- `#11-content-visibility-absent-readers` (NEW, **at approval**, U1/K17/D10): *Gap*: `innerText` (css-contain-2 §4.5 item 8:
  skipped contents do not contribute), `checkVisibility()` (CSSOM View §6 step 2: false when a flat-tree ancestor has
  computed `content-visibility: hidden`) and user-agent selection producers for document content (drag, multi-click,
  select-all; D10) do not exist in elidex (B16, Q-s5). *Why deferred*: no reader to wire; the obligation is that each, when
  built, reads K1's query — `innerText` and selection producers S2, `checkVisibility` C's ancestor form (no applies-to) —
  never a second derivation. *Trigger*: any of them lands. *Re-eval*: approval merge + 1 month.
- Appends of the K17 obligation (at approval, text = K17's entry): `#11-dialog-top-layer` (home
  `m4-12-pr-tags-t2d-interactive-landing.md:81`; roadmap `m4-12-platform-gap-roadmap.md:1290`, `:1493`; §4.5 item 11: an
  element with a `content-visibility: hidden` ancestor placed in the top layer generates no boxes — C's ancestor form); the
  roadmap's HTMLMediaElement row (`m4-12-platform-gap-roadmap.md:243`, `M4-13:#tags-T3-media`; §4.5 item 12: audio
  suppressed, events held while skipped — self computed `hidden` or S2ʰ); `#11-find-in-page` (above).
- `#11-focusable-area-being-rendered` (home `boa-vm-cutover-s2-r3-handoff.md:775`, Q-s4; cited at `focus/predicate.rs:202`,
  `:300`) — append (at approval): "S2 and CV-flush remove the 'needs computed style' blocker; CV-focus's plan decides
  discharge". The Hidden arm and the CSS residue stay there until then.

**Shadow-lane slots under U2** (current text read 2026-10-20, read-only; each is closed or narrowed **at its owning
landing**, the moved part owned by the Layout lane from approval; which facets are real is Q-s2, answered by CV-details'
plan-review):

| Slot | Current text (home) | Moves into this program | Remains |
|---|---|---|---|
| `#11-shadow-light-tree-distribution-rendering` | "light-tree → flat-tree distribution for paint (slotted nodes が slot position で render)", trigger "paint pipeline milestone", **M4-13 entry** (`m4-12-platform-gap-roadmap.md:1366`, `:1471`; not in the open-slot ledger) | the one assignment writer → **CV-flat-tree** (D11); slotted nodes rendered at their slot for **every** shadow tree → CV-details (the flat tree is one structure, K4) | nothing once both have landed → **closed** at CV-details (writer part recorded at CV-flat-tree); if Q-s2 finds the rendering facet stale (HEAD's `composed_children` already renders assigned nodes, `shadow.rs:529-549`) → that facet closed as stale |
| `#11-slot-fallback-content-rendering` | "`<slot>` fallback content の rendering pipeline 専門処理 (paint coupling)", trigger "paint pipeline milestone", **M4-13 entry** (`:1365`, `:1470`) | fallback content rendered when a slot has no assigned nodes (the default summary) — the whole facet | **closed** at CV-details, or closed as stale |
| `#11-shadow-tree-style-isolation` | "`:host` / `:host-context()` / `::slotted()` selectors + cascade isolation", trigger "CSS milestone OR framework demand", **M4-13 entry** (`:1367`, `:1476`) | a UA-origin shadow sheet applied to a UA root, with `:host` matching in it (`:host summary`, `:host([open]) summary`, D7's `:host(:not([open]))` rule) — **built by CV-details** (B22: `:host`/`::slotted()` already match in author shadow sheets) | **narrowed** at CV-details to what Q-s2 finds missing of `:host-context()`, `::slotted()` and author-shadow cascade isolation; trigger unchanged; *Re-eval*: CV-details' landing + 1 month |
| `#11-shadow-composed-event-retargeting` | **closed** — implemented, `apply_retarget` (`event_dispatch/mod.rs:627`; `m4-12-defer-sweep-2026-06-19.md:38`); the roadmap rows `:768`, `:1363` are stale | nothing (P6); CV-details consumes HEAD's retargeting for a click on the default summary | — (the stale roadmap rows are struck at approval) |

**Records**:

| Record | Ships with |
|---|---|
| **Amendments of the parent's §0.6 item 11 decided content** (recorded, not edited; USER DECIDED 2026-10-20): U1 — the reader list becomes every reader elidex has (K16) plus the standing obligation for absent readers (K17); U2 — the `details` UA shadow tree's scope includes the shadow surfaces it consumes (the per-slot table above). Parent premise errors P1–P6 (recorded, not edited); the parent's edge list gains c5 (IBP-classify → CV-details) and c7 (IBP-observer → CV-readers); E20/E22 bind every nested PR (D9, item 11's literal); the reader→PR table naming the flip for every parent pin (§5.1); the parent's §6 Open row for the home slot read with U1/U2 | **the approval PR** |
| Memory: the home slot's restated trigger and scope (U1, U2; owner of the moved shadow facets = the Layout lane); `#11-forced-style-layout-flush-on-script-read` restated (owner CV-flush); `#11-find-in-page`, `#11-details-content-pseudo`, `#11-content-visibility-absent-readers` registered; the appends to `#11-css-containment`, `#11-content-visibility-auto`, `#11-dialog-top-layer` (its T2d home and roadmap rows), the roadmap's HTMLMediaElement row and `#11-focusable-area-being-rendered`; the shadow-lane rows marked "moved to `#11-content-visibility-skip-contents` (CV-flat-tree / CV-details)", including the `:1376` mention; the stale `#11-shadow-composed-event-retargeting` roadmap rows (`:768`, `:1363`) struck | **the approval PR** |
| `#11-shadow-light-tree-distribution-rendering`'s writer part recorded as moved-and-built | CV-flat-tree |
| `#11-forced-style-layout-flush-on-script-read` discharged | CV-flush |
| The parent pins flipped; the home slot's paint/hit facets | CV-render |
| The home slot's further-reader facets (K16) | CV-readers |
| The shadow-lane slots closed or narrowed per the table above (Q-s2's answer recorded) | CV-details |
| The home slot closed | the last nested landing |

## §9. Prerequisites

- **E20** and **E22** before **every** nested PR (D9); **c5** (IBP-classify) before CV-details; **c7** (IBP-observer) before
  CV-readers; **E26** (this memo's approval) before IBP-sandbox's first nested landing.
- **Touch-time splits** (origin/main counts, §0.1): `elidex-render/src/builder/mod.rs` (1002) and
  `elidex-dom-api/src/selection.rs` (1036) are over 1000 — a standalone split precedes any nested PR that touches them;
  `focus/tests.rs` (999) before CV-focus adds a cell. Watch: `content/event_handlers.rs` 966 (CV-details' activation home),
  `elidex-plugin/src/computed_style/tests.rs` 947 (CV-core), `elidex-css-anim/src/engine.rs` 809, `builder/walk.rs` 836,
  `hit_test.rs` 764. `elidex-ecs/src/dom/mod.rs` (929 at origin/main after E1) is not touched (D8, §4's new files).
- **Lanes**: the canonical-focus lane (`#11-canonical-focus-update-steps`, `project_canonical-focus-update-steps.md`) is
  driver-less and parked on `elidex-dom-api/src/focus/` — CV-focus's plan checks its branch at its cut; IBP-sandbox's nested
  PRs edit the shells' navigation files (`content/navigation.rs`, `app/navigation.rs`) — CV-reveal's and CV-flush's plans
  check at their cuts; IBP-observer edits `elidex-api-observers/src/resize.rs` before CV-readers (c7).

## §10. Questions for the scout

Answered (rev 3):
- **Q-s1** (slot assignment writers) — answered by B17: `distribute_slots` has no mode branch and overwrites a manual
  root's assignment; `composed_children` reads whichever wrote last. Resolved by D11 (CV-flat-tree).
- **Q-s4** (`#11-focusable-area-being-rendered`'s home) — `boa-vm-cutover-s2-r3-handoff.md:775` (§8).
- **Q-s5** (user-agent selection producers) — none for document content (D10).
- **Q-s6** (initial-load fragment scroll) — absent (B20); CV-reveal builds it (D5).
- **Q-s11** (`generate_pseudo_entity`'s parent link) — the originating element (B19, K5).

Moved to the owning nested PR's plan-review:
- **CV-flush** — **Q-s3** Which script-time style or layout reads (`getComputedStyle`, layout queries, `focus()`) join the
  update point. **Q-s7** How each shell dispatches a DOM event synchronously during a same-document fragment step, and how
  style and layout are re-run before the scroll offset is computed.
- **CV-details** — **Q-s2** At its base, which facets of `#11-shadow-light-tree-distribution-rendering`,
  `#11-slot-fallback-content-rendering` and `#11-shadow-tree-style-isolation` are real (a slotted child painted at its
  slot; fallback children painted; a UA shadow sheet applied with `:host`)? **Q-s10** Every production creation path of an
  HTML element (both parsers, `createElement`, `cloneNode`, fragment parsing, `importNode`/adopt) and the one chokepoint
  that sees them all (D7's instantiation).
- **CV-focus** — **Q-s8** `is_focusable` (`predicate.rs:205`) vs `is_focusable_area` (`delegate.rs:133`): which do
  `focus()`, sequential navigation and `reconcile_focus` call?
- **CV-render** — **Q-s9** The display-list builder's emission sites and the owning entity each item carries (text runs,
  replaced images, form-control content, list markers, pseudo-elements).
- **CV-readers** — **Q-s12** Every scroll-into-view caller (CSSOM View *scroll a target into view*): `ScrollIntoView`, the
  fragment scroll, focus-driven scrolling — routed through the one `elidex-dom-api` resolution (§4).

## §11. Definition of done (program)

- Every nested PR in §5.1 (CV-flat-tree, CV-flush and CV-readers included) has landed in an order consistent with §5.2,
  each with its own reviewed plan-memo; CV-details' and CV-readers' division, if any, decided by their own plans.
- The shadow-lane slots are closed or narrowed as §8's U2 table records; K17's slots carry the obligation text;
  `#11-forced-style-layout-flush-on-script-read` is discharged.
- K1–K17 hold at the last landing; each nested plan-memo states its DoD greps — at least (each run at the landing head):
  - K1 (focus): `git grep -n -F '!is_focusable(&dom, uf)' -- crates/dom/elidex-dom-api/src/focus` = 0 hits (the
    until-found cell flipped) and `git grep -n 'in_skipped_contents' -- crates/dom/elidex-dom-api/src/focus` ≥ 1;
  - K1 (no direct reader): `git grep -l 'content_visibility' -- crates` lists only files under `elidex-plugin`,
    `elidex-css-contain`, `elidex-css-anim`, `elidex-style` and `elidex-ecs/src/dom/content_visibility.rs`;
  - K16: `/usr/bin/grep -rlE 'in_skipped_contents|skips_contents' crates` includes a file in each of paint
    (`elidex-render`), hit test (`elidex-layout`), a11y (`elidex-a11y`), focus (`elidex-dom-api/src/focus`) and the four
    CV-readers families (the scroll-into-view resolution, `elidex-api-observers/src/intersection`, `…/resize.rs`,
    `elidex-shell`'s animation caller);
  - D11: `git grep -n 'SlottedMarker' -- crates` = 0 hits; `git grep -n -E 'SlotAssignment *\{' -- crates
    ':(exclude)**/tests/**' ':(exclude)**/tests*.rs'` lists only `components.rs` (the definition) and the one writer's file;
  - D6: `git grep -n -F 'collect_open_details_by_name(' -- crates/script` = 0 hits (no VM call; the setter is marshalling);
  - D7: `git grep -l -E '"[^"]*content-visibility:' -- crates ':(exclude)**/tests/**' ':(exclude)**/tests*.rs'` (string
    literals declaring the property) lists only `elidex-style/src/ua.rs` and the UA shadow sheet's file;
  - D12: CV-flush's plan names the update point's function `P`; `git grep -n P -- crates/dom/elidex-dom-api/src/focus`
    and the reveal callers' files each ≥ 1.
- Every parent pin found at its exposer's base is flipped (K14); the §8 records have landed.
- **Checkers** (run; do not trust a stored verdict): `python3 .claude/skills/elidex-plan-review/preflight.py
  docs/plans/2026-10-content-visibility.md`.
