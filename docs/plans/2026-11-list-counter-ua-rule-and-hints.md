# `#11-list-counter-baking-ua-rule-and-hints` — list numbering through the cascade and CSS Lists 3 counter processing (umbrella plan-memo)

This memo is the **umbrella** for slot `#11-list-counter-baking-ua-rule-and-hints` (owner Layout, PM 985; registered
PM 629; slot text = `docs/plans/2026-11-pr-a2-elidex-style-parallel.md` §7.1, between its BEGIN / END markers). The
slot is edge-dense by its own text (cascade origin × presentational hints × counter processing × render × engine
modes), so by CLAUDE.md *Edge-dense work* it is a multi-PR program: this memo fixes the end state, the slices and
their order, and each slice then gets its own plan-memo and `/elidex-plan-review` before code. Rev 1, 2026-11-23.
Branch `layout-list-counter-hints-plan`, worktree `wt-lch-plan`, base `bf1cae35`. **Docs only.**

**What this memo decides** (§0.4): the homes of the facts (who writes the computed `counter-*` values, where the
implicit `list-item` increment and the reversed initial value live, the UA rule's sheet), the slice set and order
(§5), and the program-level invariants (§2). **What it proposes and does not decide** (§1, §8): the engine modes in
which the list hints apply. That conclusion narrows a ratified decision (E0), so it is written as proposal **P-1**
with both sides' precedents for the PM to route to E0's ratifier (PM 620). This memo edits neither
`docs/plans/2026-06-e0-shell-style-compat-mode.md` nor `docs/plans/2026-09-inline-box-predicate.md`.

**Approval scope**: a plan-review of this memo ratifies §0.4's decisions, §2's invariants, §5's slice set / order /
gates and §9's ledger records. §4's mechanism sketches, §6's cell expectations and the per-slice obligations are
**input to each slice's plan-review**, which re-derives them and may change them.

---

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

- Every file / line reference is at `bf1cae35` (`git rev-parse --short HEAD` in the worktree). Each slice's plan
  re-greps at its own base.
- Spec text is quoted from `.claude/tools/webref body <spec> <anchor>`, with its `# source:` URL (§3.2).
- "Markers" below are the values of the `ListItemMarker` component (`crates/core/elidex-ecs/src/components.rs:199`),
  which is the text render paints for a list item (read at `crates/core/elidex-render/src/builder/walk.rs:760`).
  "Pseudo" is the `TextContent` of a `::before` entity after the pre-layout pass. Both are written by
  `resolve_generated_content` (`crates/css/elidex-style/src/generated_content.rs:42`).
- Ground order for every decision (CLAUDE.md 至上命題): (1) spec → (2) where the spec is silent, elidex's principles →
  (3) only then measured engine behaviour, as evidence. No decision in this memo reaches rung (3); the memo cites no
  other engine's behaviour.

### §0.2 Probes executed at `bf1cae35` (PM 620: confirm by rendering and by `getComputedStyle`, not by reading)

**Harness.** A throwaway test module in `elidex-shell` (not committed; deleted after the run) drove the production
pipeline: `build_pipeline_interactive(html, css)` → `re_render` (first with the default `EngineMode::BrowserCompat`,
then with `engine_mode = BrowserCore` and a second `re_render`), then dumped, per element with an `id`: the three
`ComputedStyle` counter fields, `ListItemMarker`, the `::before` text, and `data-*` attributes written by an inline
`<script>` that calls `getComputedStyle(el).getPropertyValue(p)` (styles are resolved before the first eval,
`crates/shell/elidex-shell/src/pipeline.rs:210`). Three runs, each:

```
cd <worktree> && unset RUSTUP_TOOLCHAIN; rustc --version; git rev-parse --short HEAD; \
  cargo test -p elidex-shell --all-features probe_lch -- --nocapture      # also probe_lch2, probe_lch3
→ rustc 1.99.1 (58cf8f9c3 2026-10-13) / bf1cae35 / test result: ok. 1 passed (each run)
```

**Every case printed the same values under `BrowserCompat` and `BrowserCore`, except P16 and R2** (noted in their
rows). "+c" marks a variant whose author rule also declares `content: normal` on the same element; N-1 below is why
that matters. "Spec" is this memo's derivation from the cited text (§3.2), not a measurement.

| ID | Markup / author CSS | elidex at `bf1cae35` | Spec value | Slot case → verdict |
|---|---|---|---|---|
| P1 | `<ol><li><li><li></ol>` | markers 1, 2, 3; `li` computed `counter_increment = [("list-item", 1)]`; `ol` `counter_reset = [list-item 0]` | markers 1, 2, 3; `li` computed `counter-increment` is `none` (CSS Lists 3 §4.6) | bake writes the implicit increment into the computed value → **confirmed** (component level; not script-visible, P10) |
| P2a | `li value=5` in the middle | 1, 5, 6 | 1, 5, 6 | control |
| P2b | P2a + `li { counter-set: foo 1 }` | 1, 5, 6; `li` `counter_set = [("list-item", 5)]` — the author declaration is **absent** | 1, 2, 3 (the author declaration replaces the hint's `counter-set`) | "author `counter-set` does not override" → **changed**: as written the author value never reaches the computed style (N-1) |
| Q2b | P2b +c | 1, 5, 6; `counter_set = [("foo", 1), ("list-item", 5)]` | 1, 2, 3 | → **confirmed** once N-1 is out of the way |
| P3a | `<ol start=5>` + `ol { counter-reset: foo }` | 5, 6; `ol` `counter_reset = [list-item 4]` (author value absent) | 1, 2 | **changed** (N-1), as P2b |
| Q3a | P3a +c | 5, 6; `counter_reset = [foo 0, list-item 4]` | 1, 2 | "author `ol { counter-reset: foo }` keeps `list-item`" → **confirmed** |
| Q3c | `<ol start=5>` + `ol { counter-reset: list-item 9; content: normal }` | 10, 11 | 10, 11 | control (author names `list-item`: the bake's guard happens to agree with the cascade) |
| P4a | `<ol reversed>` × 3 | 3, 2, 1; `ol` `counter_reset = [reversed list-item 4]` | 3, 2, 1; computed `counter-reset` is `reversed(list-item)` with no integer | "bakes `li_count + 1`" → **confirmed** (component level) |
| P4b | `<ol reversed>`, second `li` `display: none` | 3, –, 2 | 2, –, 1 (§4.4.2 counts the elements that increment; §4.5 excludes the boxless one) | reversed initial value is not computed at layout time → **confirmed**, observable |
| P4c | `<ol reversed>` + `li { counter-increment: list-item -1 }` | 3, 2, 1 (author value absent, N-1) | 3, 2, 1 | right by accident |
| Q4c | P4c +c | 5, 6, 7 | 3, 2, 1 | "negates explicit increments" → **confirmed** |
| Q4f | `<ol reversed>` + `li { counter-increment: list-item 1; content: normal }` | 3, 2, 1 | −3, −2, −1 | same defect, other sign → **confirmed** |
| P4e | `#o { counter-reset: reversed(foo) } p { counter-increment: foo -1 } p::before { content: counter(foo) }`, three `p` | pseudo 0, 0, 0; both author values absent (N-1) | 3, 2, 1 | **changed** (N-1) |
| Q4e | P4e +c | pseudo 1, 2, 3; `#o` `counter_reset = [reversed foo 0]` | 3, 2, 1 | "`counter-reset` without an integer cannot be represented; fills 0" → **confirmed** |
| Q4g | `#o { counter-reset: reversed(foo); content: normal } p::before { counter-increment: foo; content: counter(foo) }` | pseudo −1, −2, −3 | −3, −2, −1 | negation + missing §4.4.2 value → **confirmed**; reachable in production (the increment is on a pseudo that declares `content`) |
| P5 | `<ol><li>a<menu><li>b<li>c</menu><li>d</ol>` + `menu { list-style-type: decimal }` | 1, 2, 3, 4; `menu` `counter_reset = []` | 1, 1, 2, 2 | "`menu` is missing" → **confirmed** |
| P6 | `#x { counter-set: c 5; counter-increment: c } #x::before { content: counter(c) }` | pseudo 0 (author values absent, N-1) | 5 | **changed** (N-1) |
| Q6 | P6 +c | pseudo 6 | 5 | "reset → set → increment order" → **confirmed** |
| Q6b | `#x::before { counter-set: c 5; counter-increment: c; content: counter(c) }` | pseudo 6 | 5 | **confirmed**, reachable in production |
| P8a | `<ol>`, second `li` `display: block` | 1, –, 3 | 1, –, 2 | "counted by tag" → **confirmed** |
| P8b | two `div` with `display: list-item; list-style-type: decimal` | "0", "0" | 1, 2 | "a non-`li` list item does not increment" → **confirmed** |
| P10 | `getComputedStyle` of `counter-reset` / `-increment` / `-set` on `ol start=3`, `li value=7`, `li`, and a `div` with all three author values (+c in Q10, so the fields are populated: `[foo 2, bar 0]`, `[("foo", 3)]`, `[("bar", 9)]`) | `"initial"` for every read, and for the IDL attribute `counterReset` | the computed value (CSSOM 1 §9) | facet → **confirmed** |
| — | PM 637 duplicate bake | 3 call sites of `apply_implicit_list_counters_from_dom` (`git grep -n 'apply_implicit_list_counters_from_dom(' -- crates`, less the definition `counter.rs:378`: `crates/css/elidex-style/src/walk.rs:194`, `crates/css/elidex-style/src/generated_content.rs:144`, `crates/core/elidex-render/src/builder/walk.rs:134`); P1 shows one entry per field, so the second and third calls change nothing | — | structural → **confirmed** by grep; no behaviour to observe |

**Findings the probes added** (not in the slot text; §0.3 routes each):

| ID | Markup / author CSS | elidex at `bf1cae35` | Spec value | Finding |
|---|---|---|---|---|
| N-1 | every row above marked "author value absent"; also Q23: `<div style="counter-reset: foo 3">` + `p::before { content: counter(foo) }` → pseudo 0, `counter_reset = []` | an author `counter-reset` / `-increment` / `-set` is dropped unless the **same element also declares `content`** | the properties apply to all elements (`webref css css-lists-3 counter-reset`: appliesTo "all elements") | `resolve_content` returns at "not declared → default (Normal)" (`crates/css/elidex-style/src/resolve/box_model/mod.rs:399`) before it reaches the three counter resolves (`:448`–`:455`) |
| N-2 | Q11a: `h1 { counter-reset: sub } p { counter-increment: sub } p::before { content: counter(sub) }` (+c), `h1` and two `p` siblings | pseudo 1, 1 | 1, 2 (§4.3: scope includes "its following siblings with their descendants") | a counter lives only until its creator's subtree ends: `resolve_tree` calls `push_scope` before `process_element` (`generated_content.rs:122`, `:145`) and `pop_scope` drops it (`counter.rs:48`) |
| N-2 | P11b: `<div><li><li></div>`, `li { list-style-type: decimal }` | 1, 1 | 1, 2 | same; reachable in production with no author counter property |
| N-3 | Q13: `<li value=5>` + `#b { counter-increment: list-item 2; content: normal }` | 1, 3, 4 | 1, 5, 6 (increment, then the hint's `counter-set`) | the bake's `li` arm skips the `value` attribute whenever the computed `counter-increment` names `list-item` (`counter.rs:344`–`:352`) |
| N-4 | P14: `<ol start=" 5">`, `<ol start="5abc">`, `<li value="7abc">` | 1; 1; 1 (`start="+5"` → 5) | 5; 5; 7 (HTML §2.3.4.1 rules for parsing integers: skip ASCII whitespace, collect the digit run) | attributes are parsed with `str::parse::<i32>` (`counter.rs:317`, `:327`, `:349`) |
| N-5 | P17: `<ol type=a>`; P18 / R1: `<ul>` in `<ul>`, `<ul>` in `<ol>`; R3: `<menu>`; R4: nested `<ul>` | P17 marker "1"; P18 inner marker "•"; R1 inner `list-style-type` is `Decimal`, marker "1"; R3 `menu` is `display: inline`, no margin / padding; R4 inner `ul` `margin-top` 16px | "a"; circle; circle; block with `margin-block: 1em`, `padding-inline-start: 40px`; 0 | HTML §15.3.7's `type` hints and most of its non-counter rules are absent (census: §4.3) |
| N-6 | P16: `<img width=100 height=50>`, `<iframe width=600>` | `BrowserCompat`: computed `width` 100px / 600px. `BrowserCore`: `auto` / `auto` | the hint applies wherever the UA honours hints | conforming-attribute hints are compat-gated today (the premise of IBP Revision 38 item (1)); the list attributes are **not** — the bake applies them in every mode (every P-row is mode-independent) |
| N-7 | R2: `<div dir=rtl>` | `BrowserCompat`: `direction: rtl`. `BrowserCore`: `ltr` | `rtl`: HTML §15.3.5 gives `[dir]:dir(rtl) { direction: rtl; }` as a UA rule, not a hint | `dir` is generated as a presentational hint (`push_dir_attr`, `crates/dom/elidex-dom-compat/src/presentational.rs:430`) and so falls under the compat gate |

Logs and the probe sources are in the session scratchpad (`lch/lch-probe{1,2,3}.log`, `lch/probe_lch{2,3}_tests.rs`);
they are not durable. The tables above carry every value a slice's plan needs; each plan re-runs its own cells at
its base.

### §0.3 Premise errors and their routing

- **PE-1 (slot text).** The author-override cases ("an author `counter-set: foo 1` does not override it", "an author
  `ol { counter-reset: foo }` keeps `list-item`", the reversed-negation and reset → set → increment cases) are
  **unreachable as written** at `bf1cae35`: an author `counter-*` declaration on an element that does not also
  declare `content` never reaches `ComputedStyle` (N-1). So the slot's "the bake is the only source of the reset and
  set values" understates it: for such elements the bake is the only source of all three fields, author values
  included. The cases hold once N-1 is removed (the Q-rows), and three of them are reachable today through a
  pseudo-element that declares `content` (Q4g, Q6b) or with no author property (P4b, P5, P8a, P8b, P11b).
- **PE-2 (slot text).** "Owed by PM/pmo: a back-pointer from master plan §10 (c)" is already discharged: the item
  carries "Back-pointer (PM 629, 2026-11-11)" (`memory/render-layout-inline-convergence-plan.md`, §10).
- **PE-3 (the slot's "Hint placement" paragraph and this plan's brief).** The two records are not of equal status.
  E0's "no hints in `BrowserCore` / `App`" is a ratified, landed invariant (E0 §5 item 4; #406). IBP's "core-path
  dimension hints" is, by IBP's own approval-scope paragraph, **input**: what #526 ratified is that the exposure is
  placed "by the cell test" and owned by `IBP-layout`; the repair's shape (Revision 38 item (1), the §2 pair
  `IBP-classify × IBP-layout`, Appendix D) is for `IBP-layout`'s plan-review. So the conflict is between one ratified
  decision and one unreviewed repair shape, plus this slot (§1.5).
- **N-1 … N-5** are defects of the same machine and the same spec section the slot names; this memo folds them
  into the program (§5) and asks the PM to approve that growth (OQ-3). **N-6, N-7** are outside the program (§7).

### §0.4 Decisions

Each line names the ground rung that decides it.

- **D-1 — one writer.** The `counter_reset` / `counter_increment` / `counter_set` fields of `ComputedStyle` are
  written by the cascade's resolve step and by nothing else. The post-cascade bake
  (`apply_implicit_list_counters_from_dom`, `apply_implicit_list_counters`, `count_li_children`) is deleted, not
  reshaped. *Rung 1*: CSS Lists 3 §4.6 ("This does not affect the specified or computed values of
  counter-increment"); HTML §15.3.7 puts the reset and the set in the cascade (a UA rule and two hints).
- **D-2 — the implicit increment is counter processing's.** A box whose computed `display` is `list-item` increments
  `list-item` by 1 (by −1 when the innermost `list-item` counter on it is reversed) at the increment step, unless
  its computed `counter-increment` names `list-item`. Tag names play no part. *Rung 1*: §4.6.
- **D-3 — counter processing is CSS Lists 3 §4's.** Steps 1–5 in the spec's order (inherit, reset, increment, set,
  use); the counters-set model of §4.4.1 / §4.4.2 (a counter's scope is its creator's descendants **and following
  siblings with their descendants**; instantiating replaces a same-name counter that originates from the element
  itself or a previous sibling); §4.5 (a boxless element does nothing). *Rung 1*.
- **D-4 — reversed counters.** An explicit increment applies as written; only the implicit one is −1. A reversed
  counter instantiated without an integer gets §4.4.2's value, computed by the counter system when it runs (after
  the cascade, over the boxes that exist), never stored in the computed value. `CounterResetEntry` gains an
  absent-integer state for the reversed form. *Rung 1*: §4.4.2, §4.6.
- **D-5 — `counter-*` resolve independently of `content`** (N-1). *Rung 1*: appliesTo "all elements".
- **D-6 — the UA rule is in the core UA sheet.** `ol, ul, menu { counter-reset: list-item; }` goes into `UA_CSS`
  (`crates/css/elidex-style/src/ua.rs`). *Rung 1* for the rule (HTML §15.3.7); its sheet follows the criterion #408
  landed for UA rules (conforming elements → the core sheet, applied in every mode; `ol`, `ul`, `menu` are
  conforming, HTML §4.4.5 – §4.4.7). This does not depend on P-1.
- **D-7 — the list attributes are presentational hints, at the hint position.** `ol[start]` / `[reversed]` →
  `counter-reset` and `li[value]` → `counter-set` enter the cascade through the existing hint seam
  (`extra_declarations` of `collect_and_cascade`, `crates/css/elidex-style/src/cascade/mod.rs:175`), so an author
  declaration of the same property replaces them and the UA rule sits below them. *Rung 1*: HTML §15.3.7, §15.2.
  **In which engine modes they are generated is P-1 (§1), not decided here.**
- **D-8 — attribute values are parsed by HTML's rules for parsing integers** (§2.3.4.1), through one shared parser,
  not a fifth copy (§7 SQ-1). *Rung 1*.
- **D-9 — one counter step, any number of drivers.** The pre-layout pass and render's paged walk drive the same
  step and add no list logic of their own; after D-1 neither calls a bake. Whether the paged driver should exist at
  all is the master plan's settled question (`generated_content.rs` module doc: per-page values need post-pagination
  assignment) and is not reopened. *Rung 2*: One issue, one way.
- **D-10 — the resolved value comes through the property-handler seam.** `getComputedStyle` serves `counter-*` from
  `CssPropertyHandler::get_computed` like every other property; no special case in `get_computed_with_registry`.
  It lands after D-1, so the first value script sees is the cascade's. *Rung 1*: CSSOM 1 §9; *Rung 2*: Plugin-first.
- **D-11 — HTML §15.3.7 is taken as a whole section.** §4.3 lists every normative unit of the section with its
  disposition; none is left unnamed.
- **D-12 — six slices after a split prerequisite, in the order of §5**, chosen so that no slice turns a
  spec-correct observable wrong on the way (§5.2).

### §0.5 OO idioms translated (ECS-native first)

| Shape in the code today | ECS-native shape this program lands |
|---|---|
| A post-cascade fix-up mutates the style object by element class (`match tag { "ol" … "li" … }`), re-run by three callers "to be safe" | The style component has one writer (the cascade system). Markup facts reach it as cascade inputs: a UA rule and a pure function `(TagType, Attributes) → declarations` |
| The numbering state is smuggled through the style object (a `reversed` flag and a pre-counted integer baked into `counter_reset`; the increment's sign fixed up inside the machine) | Counter values are **derived** by a system that reads `ComputedStyle` in tree order and writes `ListItemMarker` / pseudo `TextContent`; nothing derived is written back into `ComputedStyle` |
| "Is this a list item?" asked of the element's class (tag) | Asked of the component the cascade wrote (`display`) |
| Per-element counters set (the spec's model is per element) | Walk-local state of the counter system, not a component and not an entity-keyed side store: at this base the only readers are the system's own two drivers (`git grep -n 'CounterState' -- crates`, non-test: `generated_content.rs`, `elidex-render/src/builder/mod.rs` and `walk.rs`, and a doc comment in `builder/paint/mod.rs`). A reader outside the system (accessibility ordinals, say) is the trigger to make it a component |

---

## §1. Hint placement: derivation, and proposal P-1

### §1.1 Rung 1 — what the spec fixes

- **One cascade position, no sub-classes.** HTML §15.2: "The CSS rules given in these subsections are, except where
  otherwise specified, expected to be used as part of the user-agent level style sheet defaults … Some rules are
  intended for the author-level zero-specificity presentational hints part of the CSS cascade; these are explicitly
  called out as presentational hints." CSS Cascade 5 §6.5 gives the position ("a special-purpose author
  presentational hint origin between the regular user origin and the author origin").
- **Honouring hints is the UA's choice.** CSS Cascade 5 §6.5: "The UA may choose to honor presentational hints in a
  source document's markup". HTML §15: "User agents are not required to present HTML documents in any particular
  way"; "expected" binds "user agents designated as supporting the suggested default rendering".
- **Which attributes are conforming.** `reversed`, `start`, `type` on `ol` (HTML §4.4.5) and `value` on `li`
  (§4.4.8) are content attributes of the current language, with normative meaning (the list's *starting value*, the
  item's *ordinal value*). HTML §16.2 lists the attributes that "are obsolete … and must not be used by authors";
  among them `type on li elements`, `type on ul elements`, and the `bgcolor` / `align` / table-dimension family.
- **The counter is meant to carry the host language's numbering.** CSS Lists 3 §4.6: "UAs and host languages should
  ensure that the list-item counter values by default reflect the underlying numeric value dictated by host language
  semantics when setting up list item styling in their UA style sheet and presentational hint style mappings."

The spec has no core / compat axis. It fixes the cascade position (D-7) and which attributes are conforming; it is
**silent** on which elidex engine mode generates which hint. The question falls to rung 2.

### §1.2 Rung 2 — elidex's principles

Lens block (one line per principle: which option it selects; options in §1.4).

- *Whole-engine core / compat / deprecated consistency* ("Legacy / non-standard … を core に混ぜない。Core に置くなら
  「modern standard baseline として必要」な理由"): selects **B**. A hint of an obsolete attribute is legacy → compat
  (rejects C). A hint of a conforming attribute is the rendering of current-language markup; `<ol start=5>` numbered
  from 1 in `BrowserCore` / `App` is a wrong rendering of modern markup, and CSS Lists 3 §4.6's "should" is the
  stated reason it belongs to the baseline (rejects A).
- *One issue, one way*: selects **B**. The §15.2 surface has two halves, the UA rules and the hints. The rule half
  is already split by spec status (#408: conforming → core sheet, §16.2-obsolete → the compat sheet). B gives the
  hint half the same criterion; A keeps two criteria for one surface (status for rules, "is a hint" for hints).
- *Ideal over pragmatic*: selects **B** (spec-faithful numbering in every mode).
- *ECS-native first*: indifferent — under every option the hint source is a pure function of two components.
- *Supported-surface testing*: indifferent between A and B; under A the core arm needs cells that pin the dropped
  numbering.

Every principle that discriminates selects B. Rung 3 is not reached.

### §1.3 Conclusion — proposal P-1 (for the PM to route to E0's ratifier; not decided here)

> **P-1.** A presentational hint's layer follows the conformance status of the attribute it maps: a hint of a
> conforming attribute is **core** and is generated in every engine mode; a hint of an attribute HTML §16.2 lists as
> obsolete is **compat** and stays behind `presentational_compat()` of `StyleCompatPolicy`. For this program:
> `ol[start]`, `ol[reversed]`, `li[value]` (and `ol[type]`, LC-5) are core; `li[type]`, `ul[type]` are compat.

**Does it overturn E0?** It leaves E0's mechanism as it is: one `EngineMode`, `StyleCompatPolicy` derived in
parallel, one gate over "the legacy UA sheet + the compat hint generator", `elidex-style` mode-agnostic. It
**narrows what the gate's hint half covers**, and that contradicts E0's ratified text in three places: §1a's doc
("HTML presentational-hint declarations … `BrowserCore` / `App` → false"), §3's second row ("presentational-hint
declarations — BrowserCore+App: withheld") and §5 item 4 ("no legacy UA, no hints"), with the matching rustdoc on
`StyleCompatPolicy` (`crates/core/elidex-plugin/src/spec_level.rs:217`). So it is an amendment of a ratified
decision and is the ratifier's to accept.

### §1.4 The options, and what each keeps and overturns

| | **A** — every hint is compat (E0 as written) | **B** — by attribute status (P-1) | **C** — every hint is core |
|---|---|---|---|
| Keeps | E0 §1a / §3 / §5.4 and the `StyleCompatPolicy` rustdoc verbatim; design doc §15.3's listing of width / height hints under the compat layer | E0's mechanism and its one-bool policy; #408's criterion; CLAUDE.md three-layer rule; `engine_mode_gates_presentational_hints` (it pins `<font color>`, an obsolete attribute); today's mode-independent list numbering | today's list numbering |
| Overturns, or costs | no ratified record; it leaves two criteria on the one §15.2 surface (#408's spec status for rules, "is a hint" for hints) and does not meet CSS Lists 3 §4.6's "should" in core modes; today's behaviour: `start` / `value` / `reversed` apply in every mode (§0.2), and under A, LC-4 **removes** them from `BrowserCore` / `App`; IBP Revision 38's repair shape (dimension hints stay compat) | E0's three sentences above (they become "legacy presentational hints"); design doc §15.3's placement of width / height hints, once `IBP-layout` applies the same criterion | E0's gate for hints entirely; the three-layer rule (`bgcolor` in core); `engine_mode_gates_presentational_hints` |
| This program under it | LC-4 puts the list hints in the compat generator; LC-4's plan adds core-arm cells pinning "numbered from 1"; the exposure is recorded for the PM (the core arm is test-only today, E0 §2 Axis 3) | LC-4 adds a core hint source (OQ-2) | as B, plus moving every existing hint — outside this slot |

### §1.5 Precedents on both sides

Population: `git grep -l -i presentational -- docs .claude CLAUDE.md | wc -l` → 20. Read at their hits: the plans, the
audit and the `docs/design/ja` files. Not read separately: the `docs/design/en` translations. Incidental hits (a
mention of `presentational.rs` or of the word, no placement statement): the line-box, tree-model (two), AbortSignal
and EH-attrs plans, the PR-A2 memo (the slot text itself), `docs/design/en/25-accessibility.md`. The audit and the
philosophy umbrella restate E0's finding F6 and add no record. What the sweep cannot see: a decision recorded only
in the PM's portfolio or in memory under other words — the PM adds those when routing.

| Record | Status | Says | Side |
|---|---|---|---|
| E0 plan `docs/plans/2026-06-e0-shell-style-compat-mode.md` §1a, §1c, §3, §5.4; landed #406 (`f39f16d2`) | ratified, landed | the core arm gets "no legacy UA sheet, no presentational hints" | A |
| `StyleCompatPolicy` rustdoc, `spec_level.rs:217`–`:231` | landed with #406 | one field gates "legacy UA stylesheet + HTML presentational hints — WHATWG HTML §15.2" | A |
| Anchor design `docs/plans/2026-06-web-api-compat-split-design.md` §3.2b ("style layer derives: style-compat policy (UA sheet + presentational hints on/off)") | ratified (A0) | hints are one on / off unit | A |
| Design docs `docs/design/ja/15-rendering-pipeline.md` §15.3 互換レイヤー統合 (survey note) and `29-survey-analysis.md` (the compatibility-rule priorities, P0（必須）) | design SSoT | width / height presentational hints are a P0 item of the compat layer / of the compatibility rules | A |
| Design docs `docs/design/ja/12-dom-cssom.md` §12.2.2 (survey note) and `29-survey-analysis.md` §29.7 | design SSoT | supporting width / height hints is a P0 requirement of style resolution; no layer named | neither (requirement only) |
| #408 (`75c327c1`), E0's prerequisite; comments in `ua.rs` (`:68`–`:73`, `:111`–`:116`) and `crates/dom/elidex-dom-compat/src/legacy_ua.rs` header | landed | "Reclassify UA-stylesheet rules by spec status": conforming → core sheet "applied in every engine mode"; only §16.2-obsolete → compat. Its stated cause: E0's core arm had dropped "the only source of standard §15.3 rendering" | B (the criterion, applied to the rule half) |
| CLAUDE.md *Whole-engine core/compat/deprecated consistency* | standing rule | legacy stays out of core; core needs a "modern standard baseline" reason | B |
| IBP `docs/plans/2026-09-inline-box-predicate.md` Revision 38 item (1); §2 pair `IBP-classify × IBP-layout`; Appendix D ("a rendering-section hint, not a legacy presentational attribute") | **input**, not ratified (IBP's approval-scope paragraph); ratified part: placement by the cell test, owner `IBP-layout` | the repair is "the dimension-attribute hints on the core path" | B (unreviewed) |
| Shell test `engine_mode_gates_presentational_hints` (`crates/shell/elidex-shell/src/tests.rs:925`) | landed | `BrowserCore` drops `<font color>` | A and B (obsolete attribute) |
| The bake (`counter.rs:298`), probes §0.2 | behaviour, never ratified | `start` / `value` / `reversed` apply in every mode | B in effect |
| PM 611 "hint source is core" | **withdrawn** (PM 620) | — | neither |

### §1.6 What this memo does not touch

The existing conforming-attribute hints that are compat-gated today (the width / height family, N-6) stay where
they are; moving them is `IBP-layout`'s repair, under whatever the ratifier decides for P-1. `dir` (N-7) is a UA rule
mis-homed as a hint and is routed in §7.

---

## §2. Coupled invariants

The program holds these at once. Each slice's plan re-derives the pairs it touches.

- **I1 — single writer**: only the cascade's resolve step writes the computed `counter-*` fields (D-1, D-5).
- **I2 — cascade position**: UA rule < hint < any author declaration, per property (D-6, D-7).
- **I3 — step order**: inherit → reset → increment (explicit, then the implicit one) → set → use (D-3).
- **I4 — scope**: a counter is visible to its creator's descendants and following siblings with their descendants;
  a new same-name counter replaces one from the same element or a previous sibling and nests inside one from an
  ancestor (D-3).
- **I5 — implicit increment**: keyed on computed `display: list-item` and on the computed `counter-increment` not
  naming `list-item`; its sign comes from the innermost `list-item` counter (D-2).
- **I6 — reversed initial value**: computed by the counter system over the boxes that exist, from the increments and
  sets in the counter's scope (D-4).
- **I7 — boxless elements do nothing**: `display: none` and `display: contents` (CSS Lists 3 §4.5), as today.
- **I8 — engine modes**: the core UA sheet applies in every mode; the hint population per mode is P-1's outcome.
- **I9 — one step, two drivers**: the pre-layout pass and the paged walk (continuation suppression, `flatten`
  across pages, `set_counter` for `page` / `pages`) see the same step (D-9).
- **I10 — resolved value = computed value** for `counter-*` (D-10); it is the script-visible face of I1.
- **I11 — marker reconcile**: `ListItemMarker` is inserted or removed on every pass and on every exit path
  (`generated_content.rs`, the reconcile block before the `display: none` return). Unchanged.

Pair intersections (pairs not listed do not meet):

| Pair | Where they meet |
|---|---|
| I1 × I5 | the implicit increment may never appear in a field: P1's `li` must read `counter-increment: none` |
| I1 × I6 | the reversed initial value may never appear in a field: `ol[reversed]` computes to `reversed(list-item)` with the absent-integer state, in the cascade and in `getComputedStyle` |
| I1 × I10 | the resolved value shows whatever writes the fields; D-10 lands after the bake is gone, or script would see the bake |
| I2 × I1 | an author `counter-reset: foo` on `ol[start]` removes both the hint's and the UA rule's `list-item` (Q3a); the bake's `already_has` guards approximated this per counter name and are deleted with it |
| I2 × I8 | the same markup yields different cascade inputs per mode only through the hint population; the UA rule is mode-independent, so `menu` and nested numbering never vary by mode |
| I2 × I5 | `li[value]`'s `counter-set` and an author `counter-increment: list-item 2` are different properties: both apply (Q13); nothing may key one on the other |
| I3 × I5 | the implicit increment runs in the increment step, before set: `li[value]` ends at the set value (P2a), not one above |
| I3 × I4 | `counter-increment` / `counter-set` on a name with no counter instantiate one at this element (value 0) before acting; its scope is I4's, so the next sibling continues it (P11b) |
| I4 × I5 | a list item with no list ancestor instantiates `list-item` on itself; its following siblings continue it (P11b); a `ul` between resets it (the UA rule) |
| I4 × I6 | the §4.4.2 walk visits "each element or pseudo-element that increments or sets the same counter in the same scope": the scope is I4's, and a nested same-name counter hides its own scope from the count |
| I4 × I9 | the paged driver flattens the state between pages; the sibling-scope model must survive `flatten` (a counter created on a previous page's sibling is still in scope) |
| I5 × I6 | the implicit −1 of each list item in scope is an increment the §4.4.2 walk counts; `display: none` items (I7) are not |
| I5 × I7 | a `display: none` `li` neither increments nor is counted (P4b) |
| I6 × I9 | both drivers need the same initial value for the same counter; it is computed from the tree, not carried in the style, so neither driver can supply its own |
| I7 × I11 | the marker is removed before the boxless early returns (today's discipline); the counter change must keep that order |
| I8 × I10 | `getComputedStyle(ol).counterReset` differs by mode exactly where the hint population differs |
| I9 × I1 | the paged driver clones the style and applies the bake today (`builder/walk.rs:134`); after D-1 it reads the component as written |

---

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §15.3.7 Lists | UA rule `ol, ul, menu { counter-reset: list-item; }` | three selectors | `UA_CSS` in `ua.rs` (LC-4) | ✓ | no |
| WHATWG HTML §15.3.7 Lists | `ol[start]` / `[reversed]` → `counter-reset`, steps 1–4 | 3.1 reversed + integer (`reversed(list-item)` value + 1); 3.2 reversed, no integer; 4.1 integer (`list-item` value − 1); 4.2 no hint; parse error = no integer | the list hint source (NEW, LC-4) | ✓ | yes |
| WHATWG HTML §15.3.7 Lists | `li[value]` → `counter-set: list-item value` | parses / parse error (no hint) | the list hint source (NEW, LC-4) | ✓ | yes |
| WHATWG HTML §15.3.7 Lists | `type` hints; the non-counter UA rules; quirks rules; the non-CSS sentence | per unit, §4.3 | `ua.rs`, the hint sources (LC-5); two units routed out, two excluded | ✓ (§4.3) | yes (`type`) |
| WHATWG HTML §15.2 The CSS user agent style sheet and presentational hints | UA-level rules vs "author-level zero-specificity presentational hints" | rule / hint | `collect_and_cascade`'s hint source (unchanged seam) | ✓ | no |
| WHATWG HTML §2.3.4.1 Signed integers | rules for parsing integers, steps 1–9 | whitespace skip; `-`; `+`; non-digit error; trailing characters ignored | one shared parser (LC-4; SQ-1) | ✓ | yes |
| WHATWG HTML §4.4.5 The ol element | `reversed`, `start`, `type` are conforming content attributes | — | classification input to P-1 | ✗ (the *starting value* algorithm is not implemented as such: CSS UAs go through §15.3.7) | yes |
| WHATWG HTML §4.4.8 The li element | `value` is a conforming content attribute; *list owner*, *ordinal value* | — | classification input to P-1; no reader of ordinal values at this base (§7) | ✗ | yes |
| WHATWG HTML §16.2 Non-conforming features | obsolete attributes: `type on li elements`, `type on ul elements` | — | classification input to P-1 (LC-5) | ✗ (the two list rows only) | yes |
| CSS Lists 3 §4 Automatic Numbering With Counters | steps 1–5 | the order | `CounterState::process_element` (LC-1) | ✓ | yes |
| CSS Lists 3 §4.1 Creating Counters: the counter-reset property | value grammar | `none`; name; name + integer; `reversed()`; `reversed()` + integer; several entries | `parse_counter_list`, `resolve_counter_reset`, `CounterResetEntry` (LC-3) | ✓ | yes |
| CSS Lists 3 §4.2 Manipulating Counter Values: the counter-increment and counter-set properties | instantiate-if-absent; several entries of one name | increments compound; last set wins | `CounterState::process_element` (LC-1) | ✓ | yes |
| CSS Lists 3 §4.3 Nested Counters and Scope | scope | descendants; following siblings; obscured by a later sibling's reset | the scope model (LC-1) | ✓ | yes |
| CSS Lists 3 §4.4.1 Inheriting Counters | steps 1–5 | root; counter source = previous sibling or parent; value source = previous element in tree order | the scope model (LC-1) | ✓ | no |
| CSS Lists 3 §4.4.2 Instantiating Counters | instantiate steps 1–3; the initial-value algorithm steps 1–5 | replace (same element / previous sibling); nest (ancestor); loop break at a set; zero increments | LC-1 (instantiate); LC-3 (initial value) | ✓ | yes |
| CSS Lists 3 §4.5 Counters in elements that do not generate boxes | boxless elements | `display: none`; `content: none` pseudo; "replaced element's descendants … undefined" | `resolve_tree` (unchanged; the pseudo and replaced branches are LC-1's plan's to audit) | ✗ | yes |
| CSS Lists 3 §4.6 The Implicit list-item Counter | implicit increment | list item; reversed (−1); `counter-increment` names `list-item`; "does not affect the specified or computed values" | LC-2 | ✓ | yes |
| CSS Lists 3 §4.7 Outputting Counters: the counter() and counters() functions | read | unchanged | `evaluate_counter`, `evaluate_counters` | ✗ (not touched) | no |
| CSS Cascade 5 §6.5 Precedence of Non-CSS Presentational Hints | origin of hints | UA-origin or the author presentational hint origin; `revert` | the hint seam (unchanged; §7 F-3) | ✗ | no |
| CSSOM 1 §9 Resolved Values | default case: the resolved value is the computed value | `counter-*` are not in the special-case list | LC-6 | ✓ | no |

**Breadth**: K = 4 specs (HTML, CSS Lists 3, CSS Cascade 5, CSSOM 1), M = 20 (the rows of the table above) → at the
split-recommended threshold. **Split decision**: split — this memo is the umbrella, and §5's slices each carry a subset (no slice
exceeds two specs).

Section numbers and titles above were each looked up: `webref heading html {15.2, 15.3.7, 2.3.4.1, 4.4.5, 4.4.8,
16.2}`; `webref heading css-lists-3 {4.1, 4.2, 4.3, 4.4.1, 4.4.2, 4.5, 4.6, 4.7}` and `webref body css-lists-3
auto-numbering` (§4); `webref heading css-cascade-5 6.5`; `webref heading cssom-1 9`. `preflight.py` verifies the
HTML rows only (its label table has no CSS-module labels, `#11-preflight-css-module-labels`).

### §3.1 User-input touch audit

- Author-controlled inputs: the `start`, `reversed`, `value`, `type` attributes and the `counter-*` declarations
  (stylesheets and `style=""`). All reach integers: values are `i32`, and arithmetic stays saturating as today
  (`saturating_add` in `process_element`; `start + 1` / `start − 1` in the hint). CSS Lists 3 §4 allows clamping
  ("the value must be clamped to that range").
- New reach: after LC-3, author `counter-*` on any element takes effect (N-1), so `process_element` runs for many
  more elements, and LC-3's §4.4.2 walk scans a counter's scope. That walk must be bounded (a scope is at most the
  document; nested reversed counters must not make it quadratic without a bound): LC-3's plan states the bound.
- Adjacent pre-existing code: the depth cap `MAX_ANCESTOR_DEPTH` in `resolve_tree` is unchanged.

### §3.2 Normative text quoted (each from `webref body`, with its source)

- `# source: https://html.spec.whatwg.org/multipage/rendering.html#lists` — "ol, ul, menu { counter-reset:
  list-item; }"; "When an li element has a value attribute, and parsing that attribute's value using the rules for
  parsing integers doesn't generate an error, the user agent is expected to use the parsed value value as a
  presentational hint for the 'counter-set' property of the form list-item value."; "When an ol element has a start
  attribute or a reversed attribute, or both, the user agent is expected to use the following steps to treat the
  attributes as a presentational hint for the 'counter-reset' property: 1. Let value be null. 2. If the element has a
  start attribute, then set value to the result of parsing the attribute's value using the rules for parsing
  integers. 3. If the element has a reversed attribute: 1. If value is an integer, then increment value by 1 and
  return reversed(list-item) value. 2. Otherwise, return reversed(list-item). … 4. Otherwise: 1. If value is an
  integer, then decrement value by 1 and return list-item value. 2. Otherwise, there is no presentational hint."
- `# source: https://html.spec.whatwg.org/multipage/rendering.html#the-css-user-agent-style-sheet-and-presentational-hints`
  — quoted in §1.1.
- `# source: https://drafts.csswg.org/css-lists-3/#auto-numbering` — "1. Existing counters are inherited from
  previous elements. 2. New counters are instantiated (counter-reset). 3. Counter values are incremented
  (counter-increment). 4. Counter values are explicitly set (counter-set). 5. Counter values are used
  (counter()/counters())."
- `# source: https://drafts.csswg.org/css-lists-3/#nested-counters` — "The scope of a counter therefore starts at the
  first element in the document that instantiates that counter and includes the element's descendants and its
  following siblings with their descendants. However, it does not include any elements in the scope of a counter
  with the same name created by a counter-reset on a later sibling of the element".
- `# source: https://drafts.csswg.org/css-lists-3/#instantiating-counters` — "2. Let innermost counter be the last
  counter in counters with the name name. If innermost counter's originating element is element or a previous sibling
  of element, remove innermost counter from counters."; "When a counter is instantiated without an initial value, the
  user agent must dynamically calculate the initial value at layout-time to be the value returned by the following
  algorithm: 1. Let num be 0. 2. Let lastNonZeroIncrementNegated be 0. 3. For each element or pseudo-element el that
  increments or sets the same counter in the same scope: 1. Let incrementNegated be el's counter-increment integer
  value for this counter, multiplied by -1. 2. If incrementNegated is not zero, then set lastNonZeroIncrementNegated
  to incrementNegated. 3. If el sets this counter with counter-set, then add that integer value to num and break this
  loop. 4. Add incrementNegated to num. 4. Add lastNonZeroIncrementNegated to num. 5. Return num."; "Note: Only
  reversed counters can be instantiated without an initial value."
- `# source: https://drafts.csswg.org/css-lists-3/#list-item-counter` — "unless the counter-increment property
  explicitly specifies a different increment for the list-item counter, it must be incremented by 1 on every list
  item, or if the counter is reversed, it must be incremented by -1 on every list item instead, at the same time that
  counters are normally incremented … This does not affect the specified or computed values of counter-increment."
- `# source: https://drafts.csswg.org/css-lists-3/#counters-without-boxes` — "An element that does not generate a box
  (for example, an element with display set to none, or a pseudo-element with content set to none) cannot set, reset,
  or increment a counter."
- `# source: https://drafts.csswg.org/css-cascade-5/#preshint` — quoted in §1.1.
- `# source: https://html.spec.whatwg.org/multipage/common-microsyntaxes.html#signed-integers` — "4. Skip ASCII
  whitespace within input given position. … 8. Collect a sequence of code points that are ASCII digits from input
  given position, and interpret the resulting sequence as a base-ten integer."

---

## §4. The end state

### §4.1 Facts and their homes

| Fact | Home at `bf1cae35` | Home at the end | Slice |
|---|---|---|---|
| `ol` / `ul` / `menu` reset `list-item` | the bake's `"ol"` / `"ul"` arms (`counter.rs:305`, `:335`); `menu` nowhere | a UA rule in `UA_CSS` | LC-4 |
| `start` / `reversed` → `counter-reset`; `value` → `counter-set` | the bake (`counter.rs:305`–`:352`), after the cascade | hint declarations, a pure function of `TagType` + `Attributes`, entering at the hint seam | LC-4 |
| The implicit increment | a baked `("list-item", 1)` in the computed field (`counter.rs:363`), keyed on the tag `li`; the pre-layout pass gated by tag (`generated_content.rs:124`, `:141`) | the counter step, keyed on computed `display` | LC-2 |
| The implicit increment's sign | `process_element` negates **every** increment of a reversed counter (`counter.rs:117`) | the step applies −1 for the implicit one only | LC-2 |
| A reversed counter's initial value with no integer | a baked `li_count + 1` (`counter.rs:313`–`:321`); `0` for an author `reversed(foo)` (`unwrap_or(0)`, `box_model/mod.rs:512`; the parser already stores the default as a number) | absent in the parsed and computed value; computed by the counter system per §4.4.2 | LC-3 |
| Author `counter-*` on an element | resolved only when the element declares `content` (`box_model/mod.rs:399`) | resolved for every element | LC-3 |
| Step order | reset → set → increment (`counter.rs:84`–`:128`) | reset → increment → set | LC-1 |
| Counter scope | the creator's subtree (`push_scope` before the step, `pop_scope` after the children) | §4.3 / §4.4: subtree and following siblings; replace / nest on instantiate | LC-1 |
| Resolved value of `counter-*` | none: `get_computed_with_registry` falls to `get_initial_value` (`resolve/mod.rs:73`) | a registered handler's `get_computed` | LC-6 |
| `ol[type]`, `ul` / `menu` marker styles, nested styles and margins, `menu` box rules | absent (N-5) | UA rules and hints per §4.3 | LC-5 |

### §4.2 The counter step (input to LC-1 … LC-3; mechanism is theirs)

One step per box-generating element or pseudo-element, in tree order, taking the element's `ComputedStyle` and
`is_continuation`: (1) inherit per §4.4.1; (2) each `counter-reset` entry instantiates per §4.4.2 (initial value:
the entry's integer, else for a reversed entry the §4.4.2 value); (3) each `counter-increment` entry increments,
instantiating first if absent; then, if `display` is `list-item` and no entry named `list-item`, the implicit ±1;
skipped for a continuation as today; (4) each `counter-set` entry sets, instantiating first if absent; (5) readers
(`evaluate_counter`, `evaluate_counters`) are unchanged. The step is the only place any of this lives; the drivers
only decide which entities get a step (I7) and in what order (I9).

### §4.3 HTML §15.3.7, unit by unit (D-11)

| # | Unit (as `webref body html lists` gives it) | At `bf1cae35` | Disposition |
|---|---|---|---|
| U1 | `dir, dd, dl, dt, menu, ol, ul { display: block; }` | `ul, ol, dl, dt, dd` present (`ua.rs:16`); `menu` computes to `inline` (R3) | `display` rules are `IBP-ua-display`'s population (IBP §4, Appendix G); this memo hands it `menu` (§7 F-5). `dir` is §16.2-obsolete → compat sheet, same owner |
| U2 | `li { display: list-item; text-align: match-parent; }` | `display` present (`ua.rs:59`); `match-parent` has no hit in `crates` (`git grep -n -E 'match-parent\|MatchParent' -- crates` → 0) | `text-align: match-parent` needs a value the text module does not have → routed out (§7 F-6) |
| U3 | `dir, dl, menu, ol, ul { margin-block: 1em; }` | `ul, ol`, `dl` present as physical `margin-top` / `-bottom` (`ua.rs:53`–`:63`); `menu` absent | `menu`: LC-5. Physical-vs-logical is the sheet's existing convention, not this program's |
| U4 | `:is(dir, dl, menu, ol, ul) :is(dir, dl, menu, ol, ul) { margin-block: 0; }` | absent (R4: 16px) | LC-5 |
| U5 | `dd { margin-inline-start: 40px; }` | present, physical (`ua.rs:64`) | none |
| U6 | `dir, menu, ol, ul { padding-inline-start: 40px; }` | `ul, ol` present, physical; `menu` absent (R3) | `menu`: LC-5 |
| U7 | `ol, ul, menu { counter-reset: list-item; }` | absent; the bake stands in for `ol`, `ul` | LC-4 |
| U8 | `ol { list-style-type: decimal; }` | present (`ua.rs:61`) | none |
| U9 | `dir, menu, ul { list-style-type: disc; }` | absent: a `ul` inside an `ol` inherits `decimal` (R1) | LC-5 |
| U10 | `:is(dir, menu, ol, ul) :is(dir, menu, ul) { list-style-type: circle; }` | absent (P18, R1) | LC-5 |
| U11 | three-deep → `square` | absent | LC-5 |
| U12 | `type` hints: `ol[type=…]`, `li[type=…]` (five marker styles); `ul[type=…]`, `li[type=…]` (four) | absent (P17) | LC-5: `ol[type]` by P-1's criterion is core; `li[type]`, `ul[type]` are §16.2-obsolete → the compat generator |
| U13 | "In quirks mode, the following rules are also expected to apply" (three `list-style-position` rules) | absent | excluded: elidex never enters quirks mode (IBP §0.4's ground). No slot |
| U14 | "non-CSS user agents are expected to use the ordinal value" | n/a | excluded: elidex is a CSS user agent |
| U15 | `li[value]` → `counter-set` | the bake | LC-4 |
| U16 | `ol[start]` / `[reversed]` → `counter-reset`, steps 1–4 | the bake | LC-4 |

---

## §5. Slices

### §5.1 The set

Each slice is a per-PR unit with its own plan-memo and `/elidex-plan-review` (CLAUDE.md *Edge-dense work*; terminal
by the base case once its plan is reviewed). LC-0 is a mechanical prerequisite.

| Slice | What it lands | Decisions | Fixes (cells of §6) | Gated on |
|---|---|---|---|---|
| **LC-0** | Touch-time split: `counter.rs` is 800 lines, 386 of them the `tests` module (`:415`–). Move the tests to `counter/tests.rs`. Standalone PR, no behaviour | — | — | nothing |
| **LC-1** | The counter step's explicit semantics: §4 step order; §4.3 / §4.4.1 / §4.4.2 scope and instantiate. Both drivers, `flatten` and `set_counter` audited against the new model | D-3, D-9 | Q6b, P11b (and Q6, Q11a at the step's level) | LC-0 |
| **LC-2** | The implicit increment moves into the step, keyed on `display`; explicit increments are no longer negated. The bake's `li` increment and the tag gate (`is_list_tag` for `li`) go | D-2, D-4 (sign) | P1 (increment), P8a, P8b, Q20's field; Q4c at the step's level | LC-1 |
| **LC-3** | Author `counter-*` resolve for every element (N-1); the absent-integer state through parser, resolve and `CounterResetEntry`; the §4.4.2 initial value in the step | D-4, D-5 | P19, P20, P6 / Q6, P11a / Q11a, P4e / Q4e, Q4g, Q23, P4c stays right | LC-2 |
| **LC-4** | The UA rule; the list hints at the hint seam with HTML integer parsing; the bake, `count_li_children` and all three call sites deleted | D-1, D-6, D-7, D-8, D-9 | P2b / Q2b, P3a / Q3a, P4a (field), P4b, P5, Q13, P14, PM 637 | LC-3; **OQ-1 (P-1)**, OQ-2 |
| **LC-5** | HTML §15.3.7's remaining units (§4.3: U3, U4, U6, U9 – U12) | D-11 | P17, P18, R1, R3 (margins / padding), R4 | LC-4 for the hint source; **OQ-1** for U12's placement |
| **LC-6** | `counter-*` resolved values through the property-handler seam | D-10 | P10 / Q10 | LC-4 |

Order: LC-0 → LC-1 → LC-2 → LC-3 → LC-4 → { LC-5, LC-6 } (the last two are independent of each other).
LC-1 … LC-3 do not depend on any open question and can start as soon as this memo is plan-reviewed.

### §5.2 Why this order: no spec-correct observable turns wrong on the way

- **The step before the gate.** N-1 hides the step's defects for element-level author properties. Opening it first
  (LC-3 before LC-1 / LC-2) would turn P4c from right (3, 2, 1, by accident) to wrong (5, 6, 7, the negation), and
  would expose the order and scope defects to every author sheet at once. With LC-1 and LC-2 landed, the same author
  rule gives 3, 2, 1 through the step.
- **The implicit increment with the sign, not before or after it.** If explicit increments stopped being negated
  while the implicit one was still a baked explicit `+1`, every reversed list would count up. So the move of the
  implicit increment and the end of the negation are one slice (LC-2).
- **The absent integer with its consumer.** The §4.4.2 value needs a producer of "no integer"; the parser and
  resolve step become that producer in the same slice (LC-3). Until LC-4 the bake still writes `li_count + 1` for
  `ol[reversed]`, which is its own consistent (wrong for P4b) answer; LC-4 replaces it with the hint's
  `reversed(list-item)`.
- **The bake goes in one piece.** Its reset and set arms can only be replaced when the UA rule and both hints exist;
  a half-deleted bake would be a second writer with a shrinking job (the strangler state *One issue, one way*
  forbids). After LC-2 it writes two fields instead of three, by deletion of an arm, not by a new seam.
- **Resolved values last.** Before LC-2 a handler would show script the baked increment; before LC-4 it would show
  `reversed(list-item) 4` where the spec's computed value has no integer.

Each slice's plan re-checks this with its own cells at its base (the cells of §6 that are right today and that the
slice touches must stay right).

### §5.3 Obligations per slice (requirements; mechanism is each plan's)

- **LC-1**: state the counters-set model it implements and how `flatten` and the paged continuation path keep I4
  and I9; audit §4.5's two branches this memo marks ✗ (a `content: none` pseudo; descendants of replaced elements);
  keep I11.
- **LC-2**: the implicit increment is derived from the style the step is given (no DOM read, so both drivers get it);
  `process_element`'s caller gate becomes "has a counter property or is a list item"; rewrite the tests that
  hand-build `counter_increment: vec![("list-item", 1)]` to stand in for the bake (`git grep -n -c 'counter_increment:
  vec' -- crates`).
- **LC-3**: decide the parsed encoding of an omitted integer (today `parse_counter_list` stores the default as a
  number, so `reversed(foo)` and `reversed(foo) 0` are indistinguishable); state the §4.4.2 walk's bound (§3.1) and
  how the paged driver obtains the same value (I6 × I9); cover `style=""` (Q23).
- **LC-4**: implement the OQ-1 / OQ-2 outcome; reuse one integer parser (SQ-1); delete the bake and its three call
  sites; under option A add the core-arm cells of §1.4.
- **LC-5**: one PR for the rule units and the `type` hints, or two if its plan finds the hint half gated longer than
  the rule half; it states which.
- **LC-6**: the handler's home crate (SQ-2) and the serialization (`none`; name–integer pairs; `reversed(name)` with
  and without an integer), with `#11-cssvalue-list-separator-fidelity` and `#11-computed-style-enumeration` as
  neighbours it must not duplicate.
- **Every slice**: remove the comment that points at this slot for the defect it closes (`git grep -n
  'list-counter-baking' -- crates` → `counter.rs:81` and `generated_content.rs:131` are the step order's, LC-1;
  `generated_content.rs:137` is the bake's, LC-4).
- **Every slice**: name the tests that guard its surface (engine-level cells from §6, and the counter subset of the
  WPT `css/css-lists` directory if the runner can take it — SQ-3), per *Supported-surface testing*; new shell-level
  cells go in a new test file, since `crates/shell/elidex-shell/src/tests.rs` is 1081 lines.

---

## §6. Edge matrix (seed cells; each slice's plan owns its final list)

"First green" is the slice after which the cell holds the spec value in **both** engine modes, except rows marked
(P-1), whose core-mode value follows OQ-1.

| Cell | Spec value | Today | First green |
|---|---|---|---|
| P1 markers / `li` field | 1, 2, 3 / `none` | 1, 2, 3 / `list-item 1` | — / LC-2 |
| P2a | 1, 5, 6 | same | stays (P-1 in core after LC-4) |
| P2b, Q2b | 1, 2, 3 | 1, 5, 6 | LC-4 |
| P3a, Q3a | 1, 2 | 5, 6 | LC-4 |
| Q3c | 10, 11 | same | stays |
| P4a markers / `ol` field | 3, 2, 1 / `reversed(list-item)` | 3, 2, 1 / `reversed(list-item) 4` | stays / LC-4 (P-1) |
| P4b | 2, –, 1 | 3, –, 2 | LC-4 (P-1) |
| P4c | 3, 2, 1 | 3, 2, 1 (accident) | stays through every slice |
| Q4c | 3, 2, 1 | 5, 6, 7 | LC-2 |
| Q4f | −3, −2, −1 | 3, 2, 1 | LC-4 (needs the absent integer on `ol[reversed]`) |
| P4e, Q4e | 3, 2, 1 | 0, 0, 0 / 1, 2, 3 | LC-3 |
| Q4g | −3, −2, −1 | −1, −2, −3 | LC-3 |
| P5 | 1, 1, 2, 2 | 1, 2, 3, 4 | LC-4 |
| P6, Q6, Q6b | 5 | 0 / 6 / 6 | LC-3 / LC-1 / LC-1 |
| P8a | 1, –, 2 | 1, –, 3 | LC-2 |
| P8b | 1, 2 | 0, 0 | LC-2 (with LC-1's scope) |
| P11a, Q11a | 1, 2 | 0, 0 / 1, 1 | LC-3 / LC-1 |
| P11b | 1, 2 | 1, 1 | LC-1 |
| P12 (`counters()` across sibling `ol`) | 1 / 1, 2 | same | stays (LC-1 must keep it: the replace rule) |
| Q13 | 1, 5, 6 | 1, 3, 4 | LC-4 |
| P14a, P14b, P14d | 5; 5; 7 | 1; 1; 1 | LC-4 |
| P17 | "a" | "1" | LC-5 (P-1) |
| P18, R1 | circle | disc / decimal | LC-5 |
| R3 margins, padding; R4 | 1em, 40px; 0 | 0, 0; 16px | LC-5 |
| P19, P20, Q19 – Q21 | 2, 4; 1, 2; … | wrong only without `content` | LC-3 |
| Q23 | 3 | 0 | LC-3 |
| P10, Q10 | the computed values | `"initial"` | LC-6 |

Derivations of the less obvious spec values: P4b — two elements increment (the boxless one cannot), so num = 1 + 1,
plus the last non-zero negated increment 1 → 3, then 2, 1. Q4f — three explicit `+1`: num = −3, plus −1 → −4, then
−3, −2, −1. Q13 — the author's `counter-increment` and the hint's `counter-set` are different properties; increment
then set gives 5. P5 — `menu` is a child of the first `li`; its UA reset nests a counter, and `d` continues the
`ol`'s.

---

## §7. Out of scope, findings for the PM, and scout questions

**Findings (each needs an owner or a slot; this memo registers nothing):**

- **F-1 (N-6)** — conforming-attribute dimension hints are compat-gated. Already `IBP-layout`'s by IBP §2; P-1's
  outcome is its input. No new record.
- **F-2 (N-7)** — `dir` → `direction` is a UA rule in HTML §15.3.5, generated as a hint in `elidex-dom-compat`, so
  `BrowserCore` / `App` ignore `dir` (R2). No owner found. Proposed: a slot, owner Layout or L1, to move the rule
  to the core UA sheet (its plan checks what `:dir()` support the selector engine has).
- **F-3** — hints enter as `Origin::Author`, specificity 0, order 0 (`cascade/mod.rs:175`–`:182`); CSS Cascade 5
  §6.5 makes them an independent origin between user and author (it differs for `revert` and for layered author
  rules). Not touched here; the program adds hints at the existing seam, whatever its origin model.
- **F-4** — `HtmlElementHandler` (`crates/core/elidex-plugin/src/traits.rs:214`) offers `default_style` and
  `spec_level` per tag and has no consumer outside `elidex-plugin`
  (`git grep -n -E 'create_html_element_registry|HtmlElementHandler' -- crates`, hits only under
  `crates/core/elidex-plugin`). It is the Plugin-first home OQ-2 weighs, and "dead code は接続するか削除" applies
  to it either way.
- **F-5** — `menu { display: block }` is missing (R3) → input to `IBP-ua-display`.
- **F-6** — `li { text-align: match-parent }` needs a `text-align` value elidex does not parse. Proposed: a slot
  (owner L1 or Layout).
- **F-7** — HTML integer / dimension microsyntax parsers exist in at least four crates (`git grep -n -i -c 'rules for
  parsing' -- crates` lists 12 files; the parser definitions among them: `elidex-api-canvas/src/component.rs:256`, `elidex-dom-api/src/element/numeric_reflect.rs:65`,
  `elidex-dom-compat/src/presentational.rs:238`, `elidex-form-core/src/reflect.rs:34`). LC-4 must not add another
  (D-8); converging them is not this program's.
- **F-8** — no reader of HTML §4.4.8's *ordinal value* exists (`git grep -n -i -E 'ordinal|posinset' --
  crates/dom/elidex-a11y` → 0). When accessibility needs it, the counters set becomes a component (§0.5).

**Scout questions (facts the slices' plans need; not blocking this memo):**

- **SQ-1**: `elidex-dom-api` depends on `elidex-style`, so `elidex-style` cannot call `parse_integer`
  (`crates/dom/elidex-dom-api/src/element/numeric_reflect.rs:88`). Which crate below both can hold the one HTML integer
  parser, and who else calls the four copies of F-7?
- **SQ-2**: which handler serves `list-style-type` to `getComputedStyle` today (P10 reads `"decimal"`), and is that
  crate the home for `counter-*` handlers?
- **SQ-3**: can `elidex-wpt` run a directory subset such as `css/css-lists`, and which counter tests does it load?
- **SQ-4**: every test that builds `ComputedStyle` counter fields by hand to stand in for the bake
  (`git grep -n -c -E 'CounterResetEntry::|counter_increment: vec|counter_reset: vec|counter_set: vec' -- crates`:
  `elidex-render/src/builder/tests/counter.rs` 20, `tests/display_types.rs` 4, `counter.rs` 20,
  `generated_content.rs` 5) — which assert the bake's shape rather than numbering?

---

## §8. Open questions for the PM

- **OQ-1 — P-1 (hint placement).** Options A / B / C of §1.4. **Recommendation: B.** Decided at rung 2 (three-layer
  consistency; One issue, one way with #408), after rung 1 was found silent on the layer. It amends E0 §1a / §3 /
  §5.4 and the `StyleCompatPolicy` rustdoc → E0's ratifier. Blocks LC-4 and LC-5's U12 only.
- **OQ-2 — the core hint source's home** (only if B or C). (X) a module in `elidex-style` beside `ua.rs`, with
  `resolve_styles_with_compat`'s `hint_generator` becoming the compat extra, as `extra_ua_sheets` is for rules; or
  (Y) a hints method on `HtmlElementHandler`, classified by its `HtmlSpecLevel`. The principles split: *Plugin-first*
  selects Y; *One issue, one way* selects X within this program's reach (UA rules and hints are one kind of thing and
  today have one form; Y for the list hints alone is a second form, and Y for everything is the whole UA-defaults
  migration, F-4). **Recommendation: X**, with F-4 recorded so the migration, if wanted, moves rules and hints
  together.
- **OQ-3 — scope growth.** The slot text does not list N-1 … N-5 or §15.3.7's other units. This memo folds them in
  (LC-1's scope model, LC-3's N-1, LC-4's N-3 / N-4, LC-5), because they are the same step, the same resolve
  function and the same spec section, and the slot's exit criterion (no silent-wrong-in-production slot, PM 620)
  covers them. **Recommendation: approve the fold**; the alternative is separate slots for N-1 / N-2 (which LC-3 and
  LC-1 would then have to precede anyway, §5.2) and for LC-5.
- **OQ-4 — owners for F-2 and F-6** (two proposed slots), and whether F-4 and F-7 get records.
- **OQ-5 — under option A only**: LC-4 would make `BrowserCore` / `App` stop honouring `start` / `value` /
  `reversed`, which they honour today. Is that an accepted, cell-pinned divergence (no production session selects
  those modes, E0 §2 Axis 3), or does it need its own record?

---

## §9. Ledger records (applied by the PM / pmo; nothing here is applied by this PR)

- Slot `#11-list-counter-baking-ua-rule-and-hints`: plan = this memo; first task (PM 620) done, results §0.2; the
  slot's "Owed by PM/pmo" back-pointer is already present (PE-2); PE-1 and PE-3 recorded against the slot text.
- Master plan §10 (c) (`memory/render-layout-inline-convergence-plan.md`): closes with LC-2.
- `IBP-layout` input lines: P-1's outcome (F-1); `IBP-ua-display` input lines: `menu` (F-5).
- Proposed, pending OQ-4: slots for F-2 and F-6.

## §10. Definition of done (the program)

- `git grep -n -E 'apply_implicit_list_counters|count_li_children' -- crates` → 0 hits.
- Every cell of §6 holds its spec value in the mode(s) OQ-1's outcome names; each slice's plan carries its cells as
  tests.
- `git grep -n 'list-counter-baking' -- crates` → 0 hits (the pointer comments are gone with their defects).
- `getComputedStyle` returns the computed `counter-*` values (P10 / Q10).
- The slot row is closed by the PM with the last slice's merge; §9's records are applied.

## Changelog

- **Rev 1 (2026-11-23)** — first draft: probes at `bf1cae35` (§0.2), decisions D-1 … D-12, proposal P-1 with its
  precedent table, six slices after a split prerequisite, five open questions.
