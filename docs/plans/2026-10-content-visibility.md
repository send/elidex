# `content-visibility` skip-contents program (umbrella plan-memo)

The `content-visibility` program of the inline-box predicate prereq program's §0.6 item 11
(`docs/plans/2026-09-inline-box-predicate.md`, approved by #526 at rev 73, `f1cf5d67`; "the parent"). Home slot
`#11-content-visibility-skip-contents`; owner the Layout lane. The parent's decisions bind here (its approval scope, `:3-9`);
item 11's content and item 12's ordering are **USER DECIDED 2026-09-28**. This memo changes that content only where the
user decided so on 2026-10-20 (U1: the reader set widened; U2: CV-details builds the shadow surfaces it consumes), recorded
as amendments of item 11 under the A114 idiom ("recorded, not edited", §0.5, §8 records) and ratified by the approval PR.

**Approval scope (the #526/#532 idiom, parent `:3-9`).** This memo **decides**: the nested-PR set, their order and edges
(§5.1–§5.2), and how the parent edges E20, E22 and E26 bind to them; each fact's home and representation kind (§4's
*Home/kind* lines); the program-level invariants K1–K17 with their landing qualifiers (§2); the D-items (§0.4); the user decisions U1–U2 and
the decisions O1–O2 (§0.5); the
spec-semantics decisions (which algorithm, which steps, which outcome — §0.4, §3's *Branch* column); **which nested PR wires
skipped contents into each reader** (§5.1's reader table — the flip event every parent pin names); and the slots and records
(§8). §4's mechanism prose, §5.3's named risks, §6's tables, §7's cells, §3's *Touch* and *Full enum?* columns and the
per-PR edge-density verdicts marked *input* in §1 are **input to each nested PR's plan-review**, which re-derives them. A
review finding against that input changes this memo only when it changes a decision; otherwise it is recorded as a named
risk in the owning nested PR's input lines (§5.3).

**Approval vehicle.** A docs-only approval PR (the #526 / #532 vehicle). The user's merge approval ratifies the
approval-scope paragraph, the D-items, U1–U2 (the amendments of the parent's item 11) and O1–O2. **E26**: this memo's approval precedes
IBP-sandbox's first nested landing (parent `:1415`; IBP-sandbox memo D7, `docs/plans/2026-09-ibp-sandbox.md:180`); the
approval PR itself is a docs artefact gated by nothing. No umbrella A-row is drafted: nothing here amends a surface the
umbrella (`docs/plans/2026-08-line-box-decorated-inline-content.md`) ratified; additions to the parent are recorded here
under the A114 idiom ("recorded, not edited", §8 records).

**Revision 2** (2026-10-20) — folds the decisions on rev 1's four open questions (§0.5): **U1** (user) widens the reader
set — new nested PR CV-readers, and a standing obligation for the readers elidex lacks (K16, K17); **U2** (user) moves the
non-stale shadow surfaces CV-details needs into this program, with a per-slot record (§8); **O1** registers
`#11-details-content-pseudo`; **O2** keeps D5 and D9. Measured while folding: `#11-shadow-composed-event-retargeting` is
already **closed** (implemented, `elidex-script-session/src/event_dispatch/mod.rs:627` `apply_retarget`), so U2 moves nothing
from it (§0.3 P6).

*Rev 1* (2026-10-20) — first draft.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` is an **`a9866498`** coordinate; `git diff --stat e2d62b9e a9866498 -- crates` is empty, so the parent's
`e2d62b9e` coordinates still hold. Spec text is `.claude/tools/webref body` output (2026-10-20); every `§X.Y = name` pair is
`webref heading`/`dfn` output. `grep` = `/usr/bin/grep`; `git grep` for tree-wide searches. Memory paths are under
`~/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory/`.

### §0.2 Premises re-measured

| # | Claim (item 11 / brief) | Command | Result |
|---|---|---|---|
| B1 | elidex has no `content-visibility` | `git grep -n -i -E 'content.visibility\|ContentVisibility' -- crates` | holds: 0 hits |
| B2 | applies-to = size containment's exclusions | `webref css css-contain-2 content-visibility`; `webref body css-contain-2 containment-size` | holds: "elements for which size containment can apply"; four bullets (no principal box; inner `table`; internal table box; internal ruby box **or a non-atomic inline-level box**) |
| B3 | "non-atomic inline-level box" = F1 | `webref body css-display-3 atomic-inline` | **partial**: an atomic inline "cannot split across lines (as inline boxes **and ruby containers** can)" — the non-atomic inline-level boxes are F1 ∪ inline-level ruby containers; elidex has no `ruby` `Display` variant (`sed -n 5,29p crates/core/elidex-plugin/src/computed_style/display.rs`), so F1 alone is exact at HEAD (P2) |
| B4 | the UA rule `[hidden=until-found i]:not(embed) { content-visibility: hidden; }` (§15.3.1) | `webref body html hidden-elements` | holds verbatim |
| B5 | skipped contents read by paint, hit testing, a11y, focus, selection; computed `visibility` untouched | `webref body css-contain-2 content-visibility` | holds ("not painted (as if they had visibility: hidden)", "do not respond to hit-testing (as if they had pointer-events: none)"; `hidden`: "must not be accessible to user-agent features … nor be selectable or focusable"; §4.6 a11y omission). **Incomplete**: §4.5 names further readers — IntersectionObserver (1), ResizeObserver (2), scrolling operations (5), `innerText` (8), animations/transitions (9–10), top layer (11), media audio (12) — and CSSOM View's `scrollIntoView` step 7 and `checkVisibility` step 2 read it (Q1 → U1) |
| B6 | "contents" | same | "the **flat tree** descendants of the element, including both text and elements, **or the replaced content of a replaced element**" — the element's own box is not contents |
| B7 | `details`' UA shadow tree (HTML §15.5.5) | `webref body html the-details-and-summary-elements` | **incomplete**: **three** children — a slot taking the first `summary` child, holding a *default summary* (fallback content); a second slot taking the rest, styled `display: block; content-visibility: hidden;` when `open` is absent, `display: block;` when present, **expected to match `::details-content`** (css-pseudo-4 §5.2); a style child for `:host summary` (P3, Q2 → O1) |
| B8 | elidex has shadow/slot infrastructure, `elidex-ecs/src/dom/shadow.rs` | `wc -l crates/core/elidex-ecs/src/dom/shadow.rs`; `git grep -n 'distribute_slots' -- crates/css/elidex-style/src/walk.rs` | holds (550 lines; `attach_shadow_with_init` `:171`, manual `slot_assign` `:235`, `assigned_nodes` `:394`, `composed_children` `:529`); named distribution runs in the style walk (`walk.rs:459`, `:582`, `slot.rs:21`). **No flat-tree parent primitive** (`git grep -n 'fn flat_tree_parent\|fn assigned_slot' -- crates` = 0). Registered slots claim gaps in the surfaces a rendered UA tree consumes (Q3 → U2, P6) |
| B9 | "the first UA shadow tree" | `git grep -n -i 'user.agent.shadow\|UaShadow' -- crates` | holds: 0 hits |
| B10 | ancestor revealing on fragment navigation — "elidex has it, `elidex-shell/src/app/navigation.rs:13`" | `sed -n 9,25p crates/shell/elidex-shell/src/app/navigation.rs`; `git grep -n -i 'beforematch\|reveal' -- crates` | **partial (P1)**: `:13` is the app-mode `AppSameDocStep::FragmentNav` variant — fragment navigation exists, in **two** shells (`app/navigation.rs:199` and `content/navigation.rs:462`, both calling `content/scroll.rs:210` `scroll_offset_for_fragment`); the **ancestor revealing algorithm does not exist** (only the handler name `onbeforematch`, `elidex-script-session/src/event_handler_consumer.rs:66`) |
| B11 | (new) HEAD's focus predicate already reads `hidden=until-found` | `sed -n 190,230p;300,320p crates/dom/elidex-dom-api/src/focus/predicate.rs` | `is_focusable` (`:205`) excludes self-or-ancestor `hidden` in **both** states (`is_in_hidden_subtree`, `:215`, `:303`); right for an until-found element's descendants, **wrong for the element itself** (it generates a box, so is being rendered, HTML §15.1; only its contents are skipped) |
| B12 | (new) `details` toggle / exclusivity | `sed -n 97,140p crates/script/elidex-js/src/vm/host/html_details_proto.rs` | live only in the VM IDL setter `details_set_open` (`:97`), not in attribute change steps: `setAttribute('open', '')` and the parser fire no `toggle` and close no group member (A96; also a Layering-mandate placement) |
| B13 | (new) `summary` activation behaviour (HTML §4.11.2) | `git grep -n -i summary -- crates/shell` | absent: clicking a `summary` does nothing at HEAD |
| B14 | (new) paint and hit test share one participant walk | `grep -n collect_sc_participants crates/core/elidex-render/src/builder/walk.rs crates/layout/elidex-layout/src/hit_test.rs` | `walk.rs:554`, `hit_test.rs:208`, both `elidex-layout-block/src/paint_order.rs:54`; hit test reads neither `visibility` nor `pointer-events` (elidex has no `pointer-events`: `git grep -n -i pointer.events -- crates` = 0) |
| B15 | (new) a11y walks the composed tree | `grep -n composed_children crates/dom/elidex-a11y/src/tree.rs` | `:49`, `:133` |
| B16 | (new) find-in-page, top layer, `innerText`, `checkVisibility` | `git grep -n -i -E 'find.in.page\|top_layer\|TopLayer\|innerText\|checkVisibility' -- crates` | none exist |

### §0.3 Premise errors (reported, not edited — the A114 idiom)

- **P1** (item 11's "elidex has it, `navigation.rs:13`"): B10 — fragment navigation exists in both shells; the ancestor
  revealing algorithm and `beforematch` dispatch do not. The program **builds** the algorithm (CV-reveal); the decided
  content ("ancestor revealing on fragment navigation") is unchanged.
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
  found it implemented (`html-side-defer-sweep-2026-06-14.md:51`; `m4-12-defer-sweep-2026-06-19.md:38`; the open-slot ledger's
  close list, `project_open-defer-slots.md:103`), and `apply_retarget` is at `event_dispatch/mod.rs:627`. CV-details consumes
  it as HEAD's; the roadmap row that still lists it (`m4-12-platform-gap-roadmap.md:1363`) is stale. The other three slots
  are not in the open-slot ledger at all: they are roadmap rows (`m4-12-platform-gap-roadmap.md:1365-1367`, `:1470-1476`).

### §0.4 Decisions

**D1 — Two facts, two compositions, no stored copy.** *Skips its contents* (css-contain-2 §4 `#skips-its-contents`) is
S1(e) = computed `content-visibility` of e is `hidden` ∧ e is an element for which size containment can apply (K3). *In
skipped contents* is S2(n) = some **flat-tree ancestor** of n satisfies S1 (K4). Both are live queries over components that
already hold the inputs (`ComputedStyle`, F1, the tree) — the parent's rule "one representation per spec fact; a component
only for a fact no other component holds" (parent §1 Home grounds). No marker is written on contents: it would be a second
copy of the cascade's output with an invalidation obligation on every style or tree change. Writers: none (query), as F1.
`auto` adds S1's second arm later (`#11-content-visibility-auto`).

**D2 — Applies-to is used-time, inside S1 only.** The computed value is "as specified" (B2), so CSSOM reads `hidden` on an
element the property does not apply to; only S1 consults applies-to. Computed `visibility` and `pointer-events` are never
written (B5).

**D3 — Readers read S2 on the entity that owns the item** (a painted display item, a hit candidate, an a11y node, a focus
candidate), and S1 on a replaced or widget element for its own replaced / widget content (K5). A reader may prune a subtree
at an S1 element only where its traversal **is** the flat tree (K6).

**D4 — Paint and hit testing are wired in one nested PR** (CV-render): the spec states them together, they share one
participant walk (B14), and no landing state shows skipped contents visible-but-unhittable or invisible-but-hittable (K7).

**D5 — Reveal before skip** (the parent's cell test, §0.6 item 7, applied inside this program). After CV-core and CV-render,
`<div hidden="until-found"><p id="t">x</p></div>` navigated to `#t` paints nothing where HEAD paints `x` and the spec (reveal,
then scroll) paints `x` — so CV-reveal precedes CV-render (§5.2 c2). Likewise a closed `details` whose contents become skipped
needs `summary` activation (B13: HEAD paints the contents after a click on `summary` — vacuously, as it never hides them; the
spec paints them because the click opens it) and the reveal algorithm's `details` arm; both are built by CV-details
**before or with** the nested PR that styles the second slot `content-visibility: hidden` (K9). These are bounded repairs
the program's own PRs make necessary — derived by the parent's ownership rule, not a widening of item 11 (O2, §0.5).

**D6 — Attribute writers go through attribute change steps.** The reveal algorithm (remove `hidden`; set `open`) and
`summary` activation (toggle `open`) mutate content attributes through the engine-independent attribute mutation path, so
the element's attribute change steps run; `details`' steps (toggle event task with its tracker, name-group exclusivity —
HTML §4.11.1) move from the VM IDL setter (B12) into `elidex-dom-api`, where the setter, `setAttribute`, the parser,
activation and reveal all reach them (K10). One home; the setter becomes marshalling (Layering mandate).

**D7 — The UA shadow tree is a creation fact of the element kind.** Written once when a `details` element is created (every
creation path), by the element kind's creation hook on the `HtmlElementHandler` seam (`elidex-plugin/src/traits.rs:214`,
connected by IBP-classify, parent §1 Home grounds) — one mechanism for every later UA shadow tree (form controls, media
controls). The shadow root carries a `UserAgentShadowRoot` (NEW) marker; DOM's `ShadowRootMode` stays the spec's two values
(a UA root is never script-observable, so its mode is not a third value) (K12).

**D8 — Flat-tree parent is one primitive** — `EcsDom::flat_tree_parent` (NEW) in `elidex-ecs/src/dom/shadow.rs`, read by
S2 and by the reveal algorithm's walk ("parent node within the flat tree", HTML §6.1 step 3). It is built by its first
consumer, CV-reveal (one builder). Not in `dom/mod.rs` (1075 lines; IBP-split-ecs, parent E1).

**D9 — The parent's E20/E22 bind at nested granularity** (parent §4: "binds §4's edges to whichever nested PR supplies or
consumes the edge's fact"): E22 (the `i` flag) → **CV-core**; E20 (F1) → **CV-render** (S1's applies-to). CV-reveal consumes
neither. The program "follows its inputs" (item 11) PR by PR: CV-core and CV-reveal may land before IBP-predicate (O2).

**D10 — Selection** is read by every user-agent-initiated selection producer. HEAD has none for document content (pending
scout Q-s5; HEAD's `Selection` is script-driven, `elidex-dom-api/src/selection.rs`, and a
script `Range` over skipped nodes is legal). CV-focus owns the obligation: it wires each producer its plan measures, or, if
there is none, records the contract (S2 gates any later producer) on the slot that owns user selection; a form control inside
skipped contents cannot be focused (K15), so its text cannot be user-selected.

### §0.5 Decisions on rev 1's open questions

- **U1 — USER DECIDED 2026-10-20 (Q1 option (a), widen)** — **amendment of the parent's §0.6 item 11 decided content**
  (recorded, not edited; the A114 idiom; ratified by this memo's approval PR). Item 11's reader list ("paint, hit testing,
  accessibility, focus and selection") becomes: **every skipped-contents reader elidex has reads the one query (K1, K16)**.
  Added readers present at `a9866498`, each measured:
  - scrolling operations (css-contain-2 §4.5 item 5; CSSOM View §6.1 `scrollIntoView` step 7): the `ScrollIntoView`
    handler (`elidex-dom-api/src/element/layout_query.rs:257`) and the fragment scroll (`content/scroll.rs:210`);
  - IntersectionObserver (§4.5 item 1): `elidex-api-observers/src/intersection/mod.rs` (583 lines), VM-wired
    (`elidex-js/src/vm/host/intersection_observer.rs`);
  - ResizeObserver (§4.5 item 2): `elidex-api-observers/src/resize.rs` (620 lines), VM-wired
    (`elidex-js/src/vm/host/resize_observer.rs`) — the parent's F13 (IBP-observer) also moves it, hence the edge
    IBP-observer → CV-readers;
  - CSS animations and transitions (§4.5 items 9–10): `elidex-css-anim` (`engine.rs:147` `tick`, `engine.rs:91`
    `add_transition`, `detection.rs`).
  They are wired by the new nested PR **CV-readers**, after CV-render. Readers elidex lacks — `innerText` (§4.5 item 8),
  `checkVisibility` (CSSOM View §6 step 2, which reads an ancestor's **computed** `content-visibility: hidden`, not S1), the
  top layer (§4.5 item 11), media audio suppression (item 12) and find-in-page (HTML §6.9.2) — carry a **standing
  obligation** (K17): whoever builds one reads the same query; recorded on the slots of §8 in the Gap/Why/Trigger idiom.
  Commands: `/usr/bin/grep -rln 'scrollIntoView\|scroll_into_view' crates --include='*.rs'`; `/usr/bin/grep -rln
  'IntersectionObserverRegistry\|ResizeObserverRegistry' crates --include='*.rs'`; B16 for the absent ones.
- **U2 — USER DECIDED 2026-10-20 (Q3)** — CV-details **builds** the shadow surfaces it needs as their first production
  consumer; the non-stale parts move into this program (**the Layout lane owns them from now**), and each slot is closed or
  narrowed at CV-details' landing (§8's per-slot table: current text, what moves, what remains). Which parts are non-stale is
  **Q-s2**, answered by CV-details' own plan-review (P6 already settles `#11-shadow-composed-event-retargeting`: closed,
  nothing moves). Recorded as an amendment of item 11 (the `details` UA shadow tree's scope now includes these surfaces),
  ratified by the approval PR.
- **O1 — decision (lens converged)**: `::details-content` → slot `#11-details-content-pseudo` (NEW; §8): selector machinery
  (a pseudo-element mapped onto a UA shadow element, like `::part`) that elidex lacks, not part of skipping; A96.
- **O2 — decision (lens converged)**: D5 (summary activation and the `details` attribute change steps inside the program, by
  the parent's cell test) and D9 (E20/E22 bound per nested PR) stand as written.

## §1. Program shape and verdicts

| Axis | Own invariant | Intersects |
|---|---|---|
| CSS property | grammar, cascade, computed as specified, discrete animation with the `hidden` mapping (§4.1), UA rule with `i` | applies-to, CSSOM |
| Used-time skip | S1 over the principal box via F1; S2 over the flat tree | every reader; containment out |
| Paint + hit test | contents unpainted / pass-through; owner-entity gate; out-of-flow descendants | stacking contexts, PR-1c's inline hits, IBP-layout's F4 sites |
| A11y | omission (§4.6) | composed-tree walk |
| Focus / selection | being rendered vs skipped; render-time fixup | `reconcile_focus`, the canonical-focus lane |
| Navigation + script | reveal fires `beforematch` (script) and mutates before the scroll; two shells | attribute change steps, layout freshness |
| UA shadow tree | creation-time, never script-observable, UA-styled, `details` slot assignment; the shadow surfaces it needs (U2) | flat tree, events, serialization, clone, the shadow-lane slots |
| Further readers (U1) | scrolling operations skip; observers see no intersection / no size change; animations frozen, no transition starts | IBP-observer (F13), `elidex-css-anim`'s tick, both shells' scroll |

**Program: edge-dense — divided** (*decision*; CLAUDE.md *Edge-dense work* (a)). Per nested PR (*input*; each plan-review
applies the rule to itself): **CV-readers** may be edge-dense (four reader families in four crates × the animation
freeze/resume semantics; its plan may divide it per family); **CV-core** likely narrow (one property, one UA rule); **CV-reveal** may be edge-dense (script
event × mutation × two shells × layout freshness); **CV-render** may be edge-dense (owner-entity gating × out-of-flow ×
replaced/widget content × pseudo-elements); **CV-a11y** narrow; **CV-focus** narrow-to-moderate; **CV-details** is a
**node** — likely edge-dense (UA shadow mechanism × slot assignment × activation × attribute change steps × reveal arm ×
style) — its plan divides it (seeds: UA-shadow mechanism; tree + assignment; activation + steps + reveal arm; the slot style
last, K9).

## §2. Coupled invariants (decisions, with landing qualifiers)

- **K1 One answer** — S1 and S2 are the only answers to *skips its contents* / *in skipped contents*; no reader derives them
  from the `hidden` attribute or reads `content_visibility` directly. From **CV-render** (paint, hit), **CV-a11y**,
  **CV-focus** (which deletes `is_in_hidden_subtree`'s until-found arm, B11; its Hidden arm stays — a `display: none` proxy,
  IBP-ua-display's rule and `#11-focusable-area-being-rendered`).
- **K2 Computed ≠ used** — from **CV-core**: CSSOM serializes the computed value whatever applies-to says; `visibility` and
  `pointer-events` untouched.
- **K3 Applies-to over the principal box** — from **CV-render**: S1 requires a principal box (`display` not `none` /
  `contents`), inner display not `table` (`table`, `inline-table`), not an internal table box (css-display-3 §2.4; caption
  excluded), not an internal ruby box, not a non-atomic inline-level box (F1; plus an inline ruby container, P2). Matched
  exhaustively over `Display`, so a new variant is a compile error until classified (fail-safe: unknown never silently
  applies or not). Blockified display is F1's (the parent's IBP-predicate moves blockification to style resolution, E20).
- **K4 Flat tree** — from **CV-reveal** (primitive) and **CV-render** (S2): `flat_tree_parent` maps a slotted node to its
  assigned slot, a shadow root's child to the host, an unassigned light child of a host to *none* (not in the flat tree),
  else the DOM parent. Text nodes are contents (B6).
- **K5 What "contents" is** — flat-tree descendants ∪ the element's replaced content (F4, the parent's IBP-classify) ∪ its
  widget content (elidex paints controls without UA shadow trees, `elidex-render/src/builder/form.rs`); the element's own
  box (background, border, outline, its box's hit area) is not. Pseudo-elements of an S1 element (`::before`, `::after`,
  `::marker`) — **spec-silent edge**: CV-render's plan measures Chromium before deciding (default candidate: skipped, as size
  containment treats them as content, B2).
- **K6 Traversal independence** — a reader whose traversal is not the flat tree reads S2 per owning entity. Stacking-context
  participants, out-of-flow and positioned descendants (their containing block and stacking context lie outside the S1
  element while `#11-css-containment` is out) are skipped by flat-tree ancestry, not by where the walk meets them.
- **K7 Paint × hit test** — landed together at **CV-render**. Hit testing passes a skipped candidate over and continues to
  what lies beneath ("as if `pointer-events: none`"), so the S1 element's own box can be hit (every parent pin's conformant
  value). The gate sits where every hit candidate passes, so a later candidate producer (PR-1c's inline-box hits, umbrella
  `:1810-1811` as the parent cites it at `:1015-1018`) cannot bypass it.
- **K8 Freshness** — S2 reads the last style pass. Render-time readers (paint, hit, a11y, `reconcile_focus`,
  `focus/sot.rs:178`) run after it; script-time readers (`focus()`) read what every elidex script-time style read reads at
  HEAD (scout Q-s3). A missing `ComputedStyle` reads S1 false; a node newly inserted under an S1 element and focused in the
  same turn is blurred by the render-time fixup (K15).
- **K9 Reveal before skip** — no landing state makes an until-found descendant unpainted while fragment navigation cannot
  reveal it (**c2**), nor a closed `details`' contents unpainted while `summary` activation or the reveal `details` arm is
  missing (CV-details' internal order: the second slot's `content-visibility: hidden` lands last).
- **K10 Writes through attribute change steps** — from **CV-reveal** (`hidden` removal) and **CV-details** (`open`, and
  `details`' steps moved engine-side, D6). No writer calls a VM IDL setter.
- **K11 Reveal × script × geometry** — from **CV-reveal**: `beforematch` runs script; the algorithm's re-checks (step 4.1
  connected, 4.2.1/4.2.4 still until-found) are kept; the scroll offset is computed after the reveal from layout that
  reflects any mutation (both shells, one function).
- **K12 UA shadow tree** — from **CV-details**: created at every creation of a `details` (parser — both backends —,
  `createElement`, `cloneNode` — which never copies it but re-creates it —, fragment parsing); `UserAgentShadowRoot` marks it;
  never exposed (`shadowRoot`, `composedPath`, serialization, slot accessors) and styled by the UA sheet only.
- **K13 `details` slot assignment** — from **CV-details**: first `summary` child → slot 1 (none → the default summary),
  every other child node → slot 2; re-derived on child-list changes; one assignment writer (the style walk's named
  distribution must not overwrite it, Q-s1). The second slot's style follows `open` through K10's steps.
- **K14 Pins** — every parent pin (§5.1) flips at **CV-render**, conditional on firing at its exposer's actual base; CV-render
  flips each landed pin's assertion.
- **K16 Every present reader reads the one query** (U1) — from **CV-readers**: scrolling operations do not scroll to a node
  in skipped contents ("as if it did not have a layout box", §4.5 item 5); IntersectionObserver reports such a target as never
  intersecting, even when root and target are both skipped (item 1); ResizeObserver reports no size change for it and
  delivers on un-skip only if the size differs from the last reported (item 2); animations and transitions on a skipped
  element neither advance nor end, no new animation is created and no transition starts while skipped or on un-skip (items
  9–10). Each reads S2 (or S1 for the element itself where the item says "the element").
- **K17 Standing obligation for absent readers** (U1) — `innerText`, `checkVisibility`, the top layer, media audio and
  find-in-page read the same query when built (`checkVisibility` its computed-value form, §0.5 U1); recorded on the slots
  of §8, not as code here.
- **K15 Focus** — from **CV-focus**: a node in skipped contents is not a focusable area; the S1 element itself is (it is
  being rendered, HTML §15.1); a focused node that becomes skipped is blurred by HEAD's render-time `reconcile_focus`, which
  reads the predicate.

Pairs: K1×K6 (one answer, read per owner); K2×K3 (applies-to never feeds the computed value); K3×E20 (F1's blockified
display at style time, so S1 agrees before and after layout); K4×K12 (UA slots are flat-tree parents; the reveal walk
finds the `details` arm through them); K4×K13 (assignment decides `flat_tree_parent`); K5×F4/F7 (replaced and widget content
are the parent's classifications); K6×`#11-css-containment` (when containment lands, the ancestry rule still holds); K7×PR-1c
(new candidates pass the gate); K8×K11 (reveal's scroll needs fresh layout); K8×K15 (stale script-time reads, render-time
fixup); K9×K10 (activation and reveal write `open` through the steps that restyle the slot); K10×B12 (the IDL setter becomes
marshalling over the same steps); K14×§5.2 (CV-core precedes CV-render, so the hit-test reader is the flip, P5); K16×F13
(ResizeObserver's observed box is the parent's IBP-observer's; CV-readers gates its observation after it); K16×K8 (observer
and animation readers run in the rendering update, after the style pass); K16×K11 (the fragment scroll is both a reveal
caller and a scrolling operation: reveal first, then the skip check — a target still skipped after reveal, under an author
`content-visibility: hidden`, is not scrolled to); K17×K1 (no second query is built for a later reader).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSS CONTAIN 2 §4 Suppressing An Element’s Contents Entirely: the content-visibility property | property definition | `visible`, `hidden`; `auto` → `#11-content-visibility-auto` | CV-core | ✗ | yes |
| CSS CONTAIN 2 §4 Suppressing An Element’s Contents Entirely: the content-visibility property | *skips its contents*: contents unpainted, not hit-tested | used `contain` → `#11-css-containment`; layout-skipping SHOULD → out (§8) | CV-render | ✗ | yes |
| CSS CONTAIN 2 §4 Suppressing An Element’s Contents Entirely: the content-visibility property | `hidden`: not accessible to UA features, not selectable, not focusable | focus, selection: CV-focus; find-in-page → `#11-find-in-page` | CV-focus | ✗ | yes |
| CSS CONTAIN 2 §4.1 Animating and Interpolating content-visibility | discrete; `hidden` ↔ other maps (0,1) to the non-hidden value | | CV-core | ✓ | yes |
| CSS CONTAIN 2 §3.1 Size Containment | the four "no effect" bullets (applies-to) | ruby arm vacuous at HEAD (P2) | CV-render | ✓ | yes |
| CSS CONTAIN 2 §4.5 Restrictions and Clarifications | items 1–12 | 1, 2, 5, 9, 10: CV-readers (U1); 8, 11, 12 → K17's slots (absent in elidex); 3, 4, 6 `auto`-only → `#11-content-visibility-auto`; 7 (iframe SHOULD) → out | CV-readers | ✗ | yes |
| CSS CONTAIN 2 §4.6 Accessibility Implications | `hidden`'s skipped contents omitted | `auto` paragraph → `#11-content-visibility-auto` | CV-a11y | ✓ | yes |
| CSS DISPLAY 3 §2.4 Layout-Internal Display Types: the table-* and ruby-* keywords | *internal table box*, *internal ruby box* | | CV-render | ✓ | yes |
| WHATWG HTML §15.3.1 Hidden elements | `[hidden=until-found i]:not(embed)` | the section's other rules → IBP-ua-display / IBP-layout (parent) | CV-core | ✗ | yes |
| WHATWG HTML §6.1 The hidden attribute | *ancestor revealing algorithm* steps 1–4 | 3.1 / 4.2 (until-found): CV-reveal; 3.2 / 4.3 (`details`): CV-details | CV-reveal, CV-details | ✓ | yes |
| WHATWG HTML §7.4.6.4 Scrolling to a fragment | *scroll to the fragment* 3.4 (reveal) before 3.5 (scroll) | 3.6 / 3.7 focusing — deferred at HEAD (`content/scroll.rs` doc, §10-D2), not this program; the initial-load path per Q-s6 | CV-reveal | ✗ | yes |
| WHATWG HTML §6.9.2 Interaction with details and hidden=until-found | find-in-page | → `#11-find-in-page` (B16) | — | ✗ | yes |
| WHATWG HTML §15.5.5 The details and summary elements | the internal shadow tree, three children; the second slot's style by `open` | `::details-content` → `#11-details-content-pseudo` (O1) | CV-details | ✗ | yes |
| WHATWG HTML §4.11.1 The details element | attribute change steps (toggle task + tracker; exclusivity); insertion steps | | CV-details | ✓ | yes |
| WHATWG HTML §4.11.2 The summary element | activation behavior steps 1–3 | | CV-details | ✓ | yes |
| WHATWG HTML §6.6.2 Data model | focusable-area criteria (being rendered) | the Hidden arm → IBP-ua-display / `#11-focusable-area-being-rendered` | CV-focus | ✗ | yes |
| WHATWG HTML §15.1 Introduction | *being rendered* | the S1 element itself is | CV-focus | ✓ | no |
| CSS PSEUDO 4 §5.2 Expandable contents of details element: the ::details-content pseudo-element | | → `#11-details-content-pseudo` (O1) | — | ✗ | yes |
| CSSOM VIEW 1 §6.1 Element Scrolling Members | `scrollIntoView` step 7 (*not available to user-agent features*) | | CV-readers | ✓ | yes |
| CSSOM VIEW 1 §6 Extensions to the Element Interface | `checkVisibility` step 2 | absent (B16) → `#11-content-visibility-absent-readers` (K17) | — | ✗ | yes |

**Breadth**: K=5 (css-contain-2, css-display-3, css-pseudo-4, cssom-view-1, html), M=20 → split recommended; the split is
§5. The CSS-module rows are `webref heading` output; preflight's §3 hard-gate does not map CSS labels
(`#11-preflight-css-module-labels`), so those rows were checked by hand.

### §3.1 User-input touch audit

The `content-visibility` declaration (author CSS, new parse surface); the `hidden` attribute's value (case-insensitive
`until-found` through the `i` flag); the fragment of a navigated URL (selects the reveal target); `details`' `open` and
`name` attributes and its child list (assignment); a click or key activation on `summary`. New behaviour only hides or
reveals rendering; no new network, storage or permission surface. `beforematch` is a new script entry point fired by
navigation: its re-checks (K11) bound what a handler can make the algorithm do. Adjacent pre-existing: `details_set_open`'s
synchronous `toggle` dispatch (B12) — exposure unchanged until CV-details moves it to a queued task.

## §4. Facts → mechanisms (Home/kind lines decided; the rest input)

- **Computed value** (CV-core) — *Home/kind*: `ComputedStyle::content_visibility: ContentVisibility` (NEW; `Visible |
  Hidden`, `Auto` added by `#11-content-visibility-auto`) in `elidex-plugin/src/computed_style/`; parsed and cascaded by a new
  CSS plugin crate `elidex-css-contain` (NEW) — the css-contain-2 module's home, where `#11-css-containment`'s `contain` and
  `contain-intrinsic-size` later land (*Plugin-first*). Input: registration through the existing property-handler seam
  (precedent `elidex-css-multicol`); CSSOM serialization; §4.1's interpolation in `elidex-css-anim/src/interpolate.rs` beside
  `visibility`'s special case (`:24`).
- **UA rule** (CV-core) — *Home/kind*: the core UA sheet, `elidex-style/src/ua.rs` (436 lines), one rule. Input: consumes
  IBP-css-machinery's `i` flag (E22) and the existing simple `:not(embed)`.
- **S1** (CV-render) — *Home/kind*: `EcsDom::skips_contents(element)` (NEW), `elidex-ecs`, a query (no writer): K3 over
  `ComputedStyle` and F1 (`EcsDom::is_inline_box`, the parent's IBP-predicate); a new file `elidex-ecs/src/dom/content_visibility.rs`
  (NEW) rather than `dom/mod.rs`. Input: the internal-table classification is its own (css-display-3 §2.4), not
  `Display`'s `is_table_internal` (`display.rs:35`, which serves `visibility: collapse`).
- **S2** (CV-render) — *Home/kind*: `EcsDom::in_skipped_contents(node)` (NEW), same file, a query over `flat_tree_parent`
  and S1. Input: bounded by `MAX_ANCESTOR_DEPTH` like HEAD's walks; per-frame memoisation only if a reader's plan measures
  the need, and then as a reader-local cache dropped with its pass (never a component).
- **Flat-tree parent** (CV-reveal) — *Home/kind*: `EcsDom::flat_tree_parent(node)` (NEW) in `elidex-ecs/src/dom/shadow.rs`
  (D8), the inverse of `composed_children` (`:529`), reading the one `SlotAssignment` writer K13 settles.
- **Ancestor revealing** (CV-reveal; `details` arm CV-details) — *Home/kind*: one engine-independent function,
  `elidex_dom_api::reveal::run_ancestor_revealing` (NEW), over `flat_tree_parent`, attribute mutation (K10) and an event-fire
  callback the caller supplies (the shells dispatch `beforematch` through the session boundary). Callers: both shells'
  fragment navigation before `scroll_offset_for_fragment`; later find-in-page (`#11-find-in-page`).
- **Paint reader** (CV-render) — *Home/kind*: the gate at the display-list builder's per-owner emission
  (`elidex-render/src/builder`), reading S2 for an item's owning entity and S1 for an element's replaced/widget content (D3,
  K5). Input: which of `walk.rs` / `inline.rs` / `inline_flow.rs` / `form.rs` emit per owner (Q-s9).
- **Hit-test reader** (CV-render) — *Home/kind*: the candidate test in `elidex-layout/src/hit_test.rs` (pass over, continue,
  K7). Input: whether the gate belongs in `collect_sc_participants` (shared with paint, `paint_order.rs:54`) or in each
  consumer — a participant list filtered for both readers would also drop the S1 element's skipped out-of-flow
  descendants from painting, which K6 requires anyway.
- **A11y reader** (CV-a11y) — *Home/kind*: `elidex-a11y/src/tree.rs`'s composed walk (`:49`, `:133`), omitting nodes with S2
  (pruning at S1 is valid: the walk is the composed tree, D3). Input: a `details`' default summary appears as its summary.
- **Focus reader** (CV-focus) — *Home/kind*: `is_focusable` (`elidex-dom-api/src/focus/predicate.rs:205`) reads S2 in place of
  `is_in_hidden_subtree`'s until-found arm (K1, K15). Input: the second predicate `is_focusable_area` (`delegate.rs:133`)
  and the sequential-navigation order builder (Q-s8); `focus/tests.rs` is 999 lines — a touch-time split precedes any added
  cell (standalone prereq, CLAUDE.md).
- **Selection** (CV-focus) — D10.
- **`details` attribute change steps** (CV-details) — *Home/kind*: `elidex-dom-api` (beside `element/details_exclusion.rs`),
  run by the attribute mutation path for every writer (D6); the toggle event is **queued** as an element task with the
  tracker (HTML §4.11.1), the VM keeps only dispatch marshalling.
- **`summary` activation** (CV-details) — *Home/kind*: the activation-behaviour dispatch of the shell click/key path
  (`content/event_handlers.rs`, where checkbox/radio/label activation live), calling the `elidex-dom-api` function that
  toggles `open` (K10). Input: the default summary in the UA tree activates its host (event retargeting — HEAD's
  `apply_retarget`, P6).
- **Scrolling-operation reader** (CV-readers) — *Home/kind*: the scroll-into-view resolution shared by `ScrollIntoView`
  (`layout_query.rs:257`) and the fragment scroll (`content/scroll.rs:210`) reads S2 for the target (K16). Input: whether the
  two share one "scroll a target into view" function or the PR makes them share it (*One issue, one way*), and the other
  scroll-into-view callers (focus) its plan measures.
- **IntersectionObserver reader** (CV-readers) — *Home/kind*: the intersection computation in
  `elidex-api-observers/src/intersection/mod.rs` treats a target with S2 as non-intersecting (K16).
- **ResizeObserver reader** (CV-readers) — *Home/kind*: the observation-gathering step in `elidex-api-observers/src/resize.rs`
  skips targets with S2 and keeps their last reported size (K16), over IBP-observer's F13 computation.
- **Animation reader** (CV-readers) — *Home/kind*: `elidex-css-anim`'s per-entity tick and transition detection read S1/S2
  through a caller-supplied predicate — the crate does not depend on `elidex-ecs` (its `[dependencies]`: `elidex-plugin`,
  `elidex-css`, `cssparser`), so the engine's caller evaluates S1/S2; `engine.rs` is 809 lines — 1000-line watch.
- **Absent readers** (K17) — no code; the obligation lives on §8's slots.
- **UA shadow tree** (CV-details) — *Home/kind*: creation hook on `HtmlElementHandler` (D7); `UserAgentShadowRoot` (NEW)
  marker in `elidex-ecs/src/components.rs` (822 lines); the second slot's style as an inline declaration the steps rewrite on
  `open` change (HTML's own description: "its style attribute set to"), not author-visible (K12).

## §5. Nested PRs

### §5.1 List and reader wiring (decided)

| Nested PR | Scope |
|---|---|
| **CV-core** | the property (D2, §4 computed value), §4.1 interpolation, the `[hidden=until-found i]:not(embed)` UA rule |
| **CV-reveal** | `flat_tree_parent` (D8); the ancestor revealing algorithm's until-found arm, `beforematch`, K10/K11; wired into both shells' fragment navigation |
| **CV-render** | S1 (K3, F1), S2 (K4); paint and hit-test readers (D4, K5–K7); flips every landed parent pin (K14) |
| **CV-a11y** | a11y omission (§4.6) |
| **CV-focus** | the focus reader (K15) and the selection obligation (D10) |
| **CV-details** (node) | the shadow surfaces it consumes (U2; §8's per-slot table); the UA shadow mechanism (D7, K12); `details`' tree and assignment (K13); attribute change steps (D6); `summary` activation and the reveal `details` arm (D5); the second slot's style, last (K9) |
| **CV-readers** | U1: scrolling operations, IntersectionObserver, ResizeObserver, animations/transitions (K16) |

**Reader → wiring PR** (the flip event the parent's pins name): **paint — CV-render; hit testing — CV-render; accessibility
— CV-a11y; focus — CV-focus; selection — CV-focus** (D10); **scrolling operations, IntersectionObserver, ResizeObserver,
animations/transitions — CV-readers** (U1); `innerText`, `checkVisibility`, top layer, media audio, find-in-page — whoever
builds them (K17).

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
| E22 | IBP-css-machinery → **CV-core** | data flow (the `i` flag) — the parent's edge, bound per D9 |
| E20 | IBP-predicate → **CV-render** | data flow (F1, S1's applies-to) — the parent's edge, bound per D9 |
| E26 | this memo's approval → IBP-sandbox's first nested landing | the parent's edge — a docs artefact |
| c1 | CV-core → CV-render | data flow (the computed value S1 reads); also makes CV-render the flip of the `until-found` pins (P5) |
| c2 | CV-reveal → CV-render | A93 (D5's fragment cell) and data flow (`flat_tree_parent`) |
| c3 | CV-render → CV-a11y, CV-focus | data flow (S1, S2) |
| c4 | CV-reveal → CV-details; CV-core → CV-details | data flow (the reveal function its `details` arm extends; the property its slot style uses) |
| c5 | IBP-classify → CV-details | data flow (the `HtmlElementHandler` seam IBP-classify connects, D7) — **an addition to the parent's edge list** (§8 records) |
| c6 | CV-render → CV-readers | data flow (S1, S2) |
| c7 | IBP-observer → CV-readers | data flow (F13, the observed box the ResizeObserver gate reads) — **an addition to the parent's edge list** (§8 records); U1 |
| c8 | CV-reveal → CV-readers | data flow (K16×K11: the fragment scroll's skip check sits after the reveal call CV-reveal places) |

Acyclic: the program precedes nothing (item 12); its inputs (IBP-css-machinery, IBP-predicate, IBP-classify, IBP-observer)
have no predecessor in it; E26 is a docs artefact. CV-details and CV-render are unordered (K9 holds inside CV-details either
way: if CV-details lands first, its slot style takes visible effect at CV-render, by which time activation and the reveal arm
exist). CV-focus after CV-core transitively (c1, c3), so its until-found descendants stay excluded at its landing (B11's
right cells).

### §5.3 Named risks per nested PR (input lines)

- **CV-core**: CSSOM `getComputedStyle(el).contentVisibility` is the only observable change; `:not(embed)` against IBP-ua-display's `[hidden]:not([hidden=until-found i]):not(embed)` (the two rules partition `[hidden]`).
- **CV-reveal**: event dispatch from both shells mid-navigation (Q-s7); the initial-load fragment path (Q-s6); scroll
  geometry after mutation (K11); `dom/mod.rs` untouched (D8).
- **CV-render**: per-owner gating sites (Q-s9); pseudo-elements (K5, Chromium probe); out-of-flow (K6); replaced content
  vs the element's box for `<img hidden="until-found" src=a.png>` (the parent's no-pin note, `:1024-1025`); `builder/walk.rs`
  836 and `hit_test.rs` 764 lines — watch the 1000-line rule; the pin flips.
- **CV-a11y**: the walk's handling of unassigned light children (K4).
- **CV-focus**: two predicates (Q-s8); `focus/tests.rs` 999 lines (split first); live canonical-focus lane (§9).
- **CV-readers**: the animation freeze/resume and "no transition on un-skip" semantics (§4.5 items 9–10) against
  `elidex-css-anim`'s tick model; ResizeObserver's "deliver on un-skip only if the size differs" against F13 (IBP-observer);
  IntersectionObserver's both-skipped case; the scroll callers' population; may divide per family.
- **CV-details**: assignment vs the style walk's named distribution (Q-s1); the shadow surfaces it builds (U2, Q-s2); every creation path
  (Q-s10); `toggle` moves from synchronous to queued (an author-visible timing change, toward the spec); `focus/predicate.rs`'s
  `is_first_summary_of_details` still holds (the summary stays a DOM child); `details_set_open` becomes marshalling.

## §6. ECS-native and Layering checks (seed; input)

**OO → ECS**: Blink's `DisplayLockContext` per element (a registry with lock state and update flags) → no state: S1/S2
queries over `ComputedStyle` and the tree (D1); "contents are locked" flags propagated by a lifecycle → flat-tree ancestry
read on demand (D3); Blink's details-element hook run when its UA shadow root is added (a virtual) → a creation hook on the element-kind
plugin seam + a `UserAgentShadowRoot` marker (D7); the details' toggle task tracker held on the element object → an
element-task tracker in the `elidex-dom-api` steps (input: a component on the `details` entity, `Send + Sync`, per the
side-store rule).

Layering: queries and `flat_tree_parent` in `elidex-ecs`; reveal, `details` steps and activation logic in `elidex-dom-api`;
readers in their crates; the VM marshals (`details_set_open`, `beforematch` handler compile only); shells call one reveal
function and dispatch through the session.

## §7. Test and coverage plan (input)

**Declared surface**: css-contain-2 `content-visibility` `visible | hidden` (not `auto`), §4.6; HTML §15.3.1's until-found
rule, §6.1's revealing algorithm (fragment navigation), §15.5.5, §4.11.1, §4.11.2. WPT seeds (each PR's plan picks):
`css/css-contain/content-visibility/` (`content-visibility-0xx` hidden cases, `content-visibility-computed`/`-valid`/
`-invalid`), `html/editing/the-hidden-attribute/` (`beforematch`), `html/semantics/interactive-elements/the-details-element/`.
Cells (owner): C1 computed value + serialization (CV-core); C2 the until-found rule incl. `HIDDEN=UNTIL-FOUND` (CV-core);
C3 fragment reveal: `beforematch` fired, attribute removed, scroll after (both shells; CV-reveal); C4 a handler that
disconnects or re-hides (CV-reveal); C5 paint: contents unpainted, the element's border painted, `<img hidden=until-found>`'s
image unpainted (CV-render); C6 abspos/z-index descendant skipped (CV-render); C7 hit passes through to the S1 element
(CV-render); C8 applies-to: `display: inline`/`contents`/`table`/`table-cell` no effect, `inline-block`, `table-caption`,
an inline `img` applies (CV-render); C9 the parent pins flipped (CV-render); C10 a11y omission (CV-a11y); C11
`<button hidden=until-found tabindex=0>` focusable, its descendants not, render-time blur (CV-focus); C12 `details`: closed
contents unpainted, click opens, `toggle` queued once, name-group exclusivity on `setAttribute`, fragment into closed
`details` opens it, no `shadowRoot` exposed, clone re-creates (CV-details); C13 `scrollIntoView()` on a skipped node and a
fragment into author `content-visibility: hidden` do not scroll; C14 IntersectionObserver reports a skipped target
non-intersecting; C15 ResizeObserver silent while skipped, delivers on un-skip only on a size change; C16 a transition does
not start while skipped nor on un-skip, a running animation does not advance (CV-readers). WPT seeds:
`content-visibility-vs-scrollIntoView-00x`, `content-visibility-resize-observer-no-error`, the hidden-subtree animation
tests.

## §8. Out of scope, dispositions, records

**Out** (item 11): layout skipping (the spec's SHOULD); container-side containments (`#11-css-containment`); `auto`
(`#11-content-visibility-auto`); css-contain-2 §4.5 item 7 (an iframe's update-the-rendering skip, a SHOULD).

**Registration timing** (the IBP-sandbox §8 rule): a gap that exists at HEAD (A96) is registered **at approval**
(re-eval = approval merge + 1 month); a gap created by a nested PR's code at that landing (+1 month).

**Slots**:
- `#11-content-visibility-skip-contents` (home, opened 2026-09-30): its trigger ("the approval PR lands — authoring of its
  umbrella memo starts") has fired; restate at approval: *Trigger* = each nested landing in §5.2's order; discharged by the
  last; re-eval per the timing rule. This memo is its umbrella.
- `#11-css-containment` — append (at approval): when it lands, S1's element gets the used `contain` value (size, layout,
  style, paint); K6's ancestry rule stays the readers' rule.
- `#11-content-visibility-auto` — append: S1's `auto` arm (not relevant to the user); relevance reads focus, selection and
  the top layer (`#11-dialog-top-layer`); §4.5 items 3, 4, 6.
- `#11-find-in-page` (NEW, at approval): *Gap*: elidex has no find-in-page (B16); *obligations*: HTML §6.9.2's
  temporary un-skipping of until-found and closed-`details` contents during the search, and its reveal call through the one
  reveal function; *Trigger*: a find-in-page feature or `window.find`; A96.
- `#11-details-content-pseudo` (NEW, **at approval**, O1): *Gap*: the `::details-content` pseudo-element (css-pseudo-4 §5.2)
  — HTML §15.5.5 expects `details`' second slot to match it; elidex parses and matches no such pseudo-element. *Why
  deferred*: it is selector machinery elidex lacks (a pseudo-element mapped onto an element of a UA shadow tree, the `::part`
  shape), not part of skipping; A96 — HEAD matches nothing and CV-details leaves that unchanged, so no cell fires. *Trigger*:
  CV-details has landed **and** (a css-pseudo-4 `::details-content` WPT subset is declared supported, or a pseudo-element
  onto a UA shadow element is built for another element). *Re-eval*: the approval PR's merge date + 1 month (the timing rule
  above; the gap exists at HEAD).
- `#11-content-visibility-absent-readers` (NEW, **at approval**, U1/K17): *Gap*: `innerText` (css-contain-2 §4.5 item 8:
  skipped contents do not contribute) and `checkVisibility()` (CSSOM View §6 step 2: false when a flat-tree ancestor has
  **computed** `content-visibility: hidden`) do not exist in elidex (B16). *Why deferred*: no reader to wire; the obligation is
  that the API, when built, reads the program's one query (S2 for `innerText`; for `checkVisibility` the computed value over
  `flat_tree_parent` — not S1, whose applies-to gate the step does not have) — never a second derivation. *Trigger*: either
  API lands. *Re-eval*: approval merge + 1 month.
- Appends of the K17 obligation (at approval, text = K17): `#11-dialog-top-layer` (§4.5 item 11: an element with a
  `content-visibility: hidden` ancestor placed in the top layer generates no boxes); the roadmap's HTMLMediaElement row
  (`m4-12-platform-gap-roadmap.md:243`, `M4-13:#tags-T3-media`; §4.5 item 12: audio suppressed, events held while skipped);
  `#11-find-in-page` (above).
- `#11-focusable-area-being-rendered` — cross-referenced, not changed (cited at `focus/predicate.rs:202`, `:300`; ledger home
  per Q-s4): the Hidden arm and the CSS residue stay there.

**Shadow-lane slots under U2** (current text read 2026-10-20, read-only; each is closed or narrowed **at CV-details' landing**,
the moved part owned by the Layout lane from approval; which facets are real is Q-s2, answered by CV-details' plan-review):

| Slot | Current text (home) | Moves into this program | Remains |
|---|---|---|---|
| `#11-shadow-light-tree-distribution-rendering` | "light-tree → flat-tree distribution for paint (slotted nodes が slot position で render)", trigger "paint pipeline milestone", **M4-13 entry** (`m4-12-platform-gap-roadmap.md:1366`, `:1471`; not in the open-slot ledger) | slotted nodes rendered at their slot for **every** shadow tree — the flat tree is one structure (K4), so the part CV-details needs is the whole facet, including the one assignment writer (K13, Q-s1) | nothing if Q-s2 finds the facet real and CV-details builds it → **closed**; if Q-s2 finds it stale (HEAD's `composed_children` already renders assigned nodes, `shadow.rs:529-549`) → closed as stale at approval |
| `#11-slot-fallback-content-rendering` | "`<slot>` fallback content の rendering pipeline 専門処理 (paint coupling)", trigger "paint pipeline milestone", **M4-13 entry** (`:1365`, `:1470`) | fallback content rendered when a slot has no assigned nodes (the default summary) — again the whole facet | as above: **closed** at CV-details, or closed as stale |
| `#11-shadow-tree-style-isolation` | "`:host` / `:host-context()` / `::slotted()` selectors + cascade isolation", trigger "CSS milestone OR framework demand", **M4-13 entry** (`:1367`, `:1476`) | UA-only styling of a UA shadow tree and `:host` / `:host([open])` matching for the default-summary rules (HTML §15.5.5's third child) — or, if CV-details expresses those rules without `:host` (the spec allows "a mechanism that is not an element"), only the UA-sheet isolation | **narrowed** to `:host-context()`, `::slotted()` and author-shadow cascade isolation (and `:host` if not built), trigger unchanged |
| `#11-shadow-composed-event-retargeting` | **closed** — implemented, `apply_retarget` (`event_dispatch/mod.rs:627`; `m4-12-defer-sweep-2026-06-19.md:38`); the roadmap row `:1363` is stale | nothing (P6); CV-details consumes HEAD's retargeting for a click on the default summary | — (the stale roadmap row is struck at approval) |

**Records**:

| Record | Ships with |
|---|---|
| **Amendments of the parent's §0.6 item 11 decided content** (recorded, not edited; USER DECIDED 2026-10-20): U1 — the reader list becomes every reader elidex has (K16) plus the standing obligation for absent readers (K17); U2 — the `details` UA shadow tree's scope includes the shadow surfaces it consumes (the per-slot table above). Parent premise errors P1–P6 (recorded, not edited); the parent's edge list gains c5 (IBP-classify → CV-details) and c7 (IBP-observer → CV-readers); E20/E22's nested binding (D9, O2); the reader→PR table naming the flip for every parent pin (§5.1); the parent's §6 Open row for the home slot read with U1/U2 | **the approval PR** |
| Memory: the home slot's restated trigger and scope (U1, U2; owner of the moved shadow facets = the Layout lane); `#11-find-in-page`, `#11-details-content-pseudo`, `#11-content-visibility-absent-readers` registered; the appends to `#11-css-containment`, `#11-content-visibility-auto`, `#11-dialog-top-layer` and the roadmap's HTMLMediaElement row; the shadow-lane rows marked "moved to `#11-content-visibility-skip-contents` (CV-details)"; the stale `#11-shadow-composed-event-retargeting` roadmap row struck | **the approval PR** |
| The parent pins flipped; the home slot's paint/hit facets | CV-render |
| The home slot's further-reader facets (K16) | CV-readers |
| The shadow-lane slots closed or narrowed per the table above (Q-s2's answer recorded) | CV-details |
| The home slot closed | the last nested landing |

## §9. Prerequisites

- **E22** before CV-core; **E20** before CV-render; **c5** (IBP-classify) before CV-details; **c7** (IBP-observer) before
  CV-readers; **E26** (this memo's approval) before IBP-sandbox's first nested landing.
- **Touch-time splits**: `focus/tests.rs` (999) before CV-focus adds a cell; `elidex-ecs/src/dom/mod.rs` (1075) is not
  touched (D8; IBP-split-ecs, E1, may land first anyway).
- **Live lanes**: the canonical-focus lane (`#11-canonical-focus-update-steps`, `project_canonical-focus-update-steps.md`)
  edits `elidex-dom-api/src/focus/` — CV-focus's plan checks hunks at its cut; IBP-sandbox's nested PRs edit the shells'
  navigation files (`content/navigation.rs`, `app/navigation.rs`) — CV-reveal's plan checks at its cut; IBP-observer edits
  `elidex-api-observers/src/resize.rs` before CV-readers (c7).

## §10. Questions for the scout

- **Q-s1** Slot assignment writers: does the style walk's `distribute_slots` (`walk.rs:459`, `:582`) respect a `Manual`
  root, and which writer's `SlotAssignment` does `composed_children` read when both ran?
- **Q-s2** (answered by **CV-details' own plan-review**, U2) At its base, which facets of
  `#11-shadow-light-tree-distribution-rendering`, `#11-slot-fallback-content-rendering` and `#11-shadow-tree-style-isolation`
  are real (a slotted child painted at its slot; fallback children painted; `:host` matching in a shadow sheet)?
  (`#11-shadow-composed-event-retargeting` is settled: closed, P6.)
- **Q-s12** Every scroll-into-view caller (CSSOM View *scroll a target into view*): `ScrollIntoView`, the fragment scroll,
  focus-driven scrolling — one function or several?
- **Q-s3** Does any script-time style or layout read force a style/layout update (getComputedStyle, layout queries,
  `focus()`), and is the absence registered as a slot?
- **Q-s4** Where is `#11-focusable-area-being-rendered` registered (code cites it; the open-slot ledger has no entry)?
- **Q-s5** Any user-agent selection producer for document content (pointer drag, multi-click, select-all) outside form
  controls?
- **Q-s6** The initial-load fragment scroll (HTML §7.4.6.2 *try to scroll to the fragment*): where, and does it share
  `scroll_offset_for_fragment`?
- **Q-s7** How each shell dispatches a DOM event synchronously during a same-document fragment step, and whether style and
  layout can be re-run before the scroll offset is computed.
- **Q-s8** `is_focusable` (`predicate.rs:205`) vs `is_focusable_area` (`delegate.rs:133`): which do `focus()`, sequential
  navigation and `reconcile_focus` call?
- **Q-s9** The display-list builder's emission sites and the owning entity each item carries (text runs, replaced images,
  form-control content, list markers, pseudo-elements).
- **Q-s10** Every production creation path of an HTML element (both parsers, `createElement`, `cloneNode`, fragment
  parsing, `importNode`/adopt) and whether one hook point sees them all.
- **Q-s11** `generate_pseudo_entity`'s parent link (`elidex-style/src/pseudo.rs:42-78`): is the originating element the
  pseudo entity's tree parent?

## §11. Definition of done (program)

- Every nested PR in §5.1 (CV-readers included, U1) has landed in an order consistent with §5.2, each with its own reviewed
  plan-memo; CV-details' and CV-readers' division, if any, decided by their own plans.
- The shadow-lane slots are closed or narrowed as §8's U2 table records; K17's slots carry the obligation text.
- K1–K17 hold at the last landing; each nested plan-memo states its DoD greps — at least: `git grep -n -i
  'until-found' -- crates/dom/elidex-dom-api/src/focus` shows no attribute arm for the until-found state (K1);
  `git grep -n 'content_visibility' -- crates` lists only `elidex-plugin`, `elidex-css-contain`, `elidex-css-anim`,
  `elidex-style` (UA sheet tests) and the two queries' file (K1: no reader reads the field); `/usr/bin/grep -rn
  'in_skipped_contents\|skips_contents' crates` lists a site in each of paint, hit test, a11y, focus and the four CV-readers
  families (K16).
- Every parent pin found at its exposer's base is flipped (K14); the §8 records have landed.
- **Checkers** (run; do not trust a stored verdict): `python3 .claude/skills/elidex-plan-review/preflight.py
  docs/plans/2026-10-content-visibility.md`.
