# `#11-list-counter-baking-ua-rule-and-hints` — list numbering through the cascade and CSS Lists 3 counter processing (umbrella plan-memo)

This memo is the **umbrella** for slot `#11-list-counter-baking-ua-rule-and-hints` (owner Layout, PM 985; registered
PM 629; slot text = `docs/plans/2026-11-pr-a2-elidex-style-parallel.md` §7.1, between its BEGIN / END markers). The
slot is edge-dense by its own text (cascade origin × presentational hints × counter processing × render × engine
modes), so by CLAUDE.md *Edge-dense work* it is a multi-PR program: this memo fixes the end state, the slices and
their order, and each slice then gets its own plan-memo and `/elidex-plan-review` before code. Rev 2, 2026-11-23
(plan-review round 1 folded; changelog at the end). Branch `layout-list-counter-hints-plan`, worktree `wt-lch-plan`,
base `bf1cae35`. **Docs only.**

**What this memo decides** (§0.4): the homes of the facts (who writes the computed `counter-*` values, where the
implicit `list-item` increment and the reversed initial value live, the UA rule's sheet), the slice set and order
(§5) with the window table that grounds the order (§5.2), and the program-level invariants (§2). **What it proposes
and does not decide** (§1, §8): the engine modes in which the list hints apply, and the reading of HTML §2.1.8's
*designation* that the answer rests on. Every answer but option A changes a ratified decision (E0), so the
conclusion is written as proposal **P-1** with both sides' precedents for the PM to route to E0's ratifier (PM 620).
This memo edits neither `docs/plans/2026-06-e0-shell-style-compat-mode.md` nor
`docs/plans/2026-09-inline-box-predicate.md`.

**Approval scope**: a plan-review of this memo ratifies §0.4's decisions, §2's invariants, §5's slice set / order /
gates (the constraints of §5.2 included) and §9's ledger records. §4's mechanism sketches, §6's cell expectations
and the per-slice obligations are **input to each slice's plan-review**, which re-derives them and may change them.

---

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

- Every file / line reference is at `bf1cae35` (the worktree's `65a38919` is `bf1cae35` plus this memo:
  `git diff --stat bf1cae35 65a38919 -- crates` is empty). Each slice's plan re-greps at its own base.
- Spec text is quoted from `.claude/tools/webref body <spec> <anchor>`, with its `# source:` URL (§3.2).
- "Markers" below are the values of the `ListItemMarker` component (`crates/core/elidex-ecs/src/components.rs:199`),
  which is the text render paints for a list item (read at `crates/core/elidex-render/src/builder/walk.rs:760`).
  "Pseudo" is the `TextContent` of a `::before` entity after the pre-layout pass. Both are written by
  `resolve_generated_content` (`crates/css/elidex-style/src/generated_content.rs:42`).
- Ground order for every decision (CLAUDE.md 至上命題): (1) spec → (2) where the spec is silent, elidex's principles →
  (3) only then measured engine behaviour, as evidence. No decision in this memo reaches rung (3); the memo cites no
  other engine's behaviour.
- "A cell turns wrong" (§5.2) means: an observable that holds its spec value before a slice lands and does not hold
  it after. A cell that is wrong before and after, with a different wrong value, is not a hazard.

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
| P4a | `<ol reversed>` × 3 | 3, 2, 1; `ol` `counter_reset = [reversed list-item 4]` | 3, 2, 1; computed `counter-reset` is `reversed(list-item)` with no integer (the reading of §3.2's F7 note) | "bakes `li_count + 1`" → **confirmed** (component level) |
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

**Probe 4 (rev 2).** Same harness and command with the filter `probe_lch4`; `rustc 1.99.1 (58cf8f9c3 2026-10-13)` /
`65a38919` / `test result: ok. 1 passed`. It measures today's values of the window cells §5.2 and §6 use. Every
case printed the same values under `BrowserCompat` and `BrowserCore`.

| ID | Markup / author CSS | elidex at `bf1cae35` | Spec value | Used by |
|---|---|---|---|---|
| W-F2 | `<ol><li><li value=5 id=b><li></ol>` + `#b { counter-increment: list-item 2 }` (no `content`) | 1, 5, 6 (`#b`: `counter_set = [("list-item", 5)]`; the author increment is absent, N-1) | 1, 5, 6 | §5.2 c4 |
| W-F3 | `<ol reversed><li style="display:block">x<li>y</ol>` | y = 1 (`ol`: `[reversed list-item 3]`; both `li` carry the baked increment) | y = 1 | §5.2 c5 |
| W-H3 | `<div><ol><li>a</ol><menu><li>b</menu></div>` + `menu { list-style-type: decimal }` | b = 1 | 1 | §5.2 c3 |
| W-H3b | `<div><li>a</li><menu><li>b</menu></div>` + `li { list-style-type: decimal }` | 1, 1 | 1, 1 | §5.2 c3 |
| W-H4 | `<div><ol><li>a</ol><div style="counter-reset: list-item"><li>b</div></div>` (b: `list-style-type: decimal`) | b = 1 (the reset is absent, N-1) | 1 | §5.2 c2 |
| W-H4b | `<div><ol><li><li></ol><div style="counter-set: list-item 0"><li>b</div></div>` | b = 1 (the set is absent, N-1) | 1 | §5.2 c2 |
| W-H5 | `<div><ol><li>a</ol><dir><li>b</dir></div>` | b = 1 (`dir` computes `display: inline`; no rule in either sheet) | 1 where `dir` is rendered as `ul` (HTML §16.3.3); 2 where `dir` carries no UA rule (§4.3) | §5.2 c10 |
| W-RV | `<ol reversed><li><li value=10><li></ol>` | 3, 10, 9 | 11, 10, 9 (§4.4.2's loop breaks at the set: num = 1 + 10, then + 1) | I3 × I6 |
| W-S1 | `<div><ol><li>a</ol><p id=p></div>` + `#p::before { content: counter(list-item) }` | pseudo 0 | 1 | N-2 |
| W-S2 | `<div><ul><li>a</ul><div><li>b</div></div>` | b = 1 | 2 (the `ul`'s counter is in scope for its following sibling's descendants, §4.4.1) | N-2 |
| W-O1 | `<ol><li><li></ol>` + `li { counter-set: list-item 7; content: normal }` | 8, 8 | 7, 7 | step order |
| W-N1, W-N2 | `<ol><li>a<ol id=x><li>b</ol></ol>` + `#x { counter-reset: foo }`, with / without `content: normal` | b = 1 (`#x`: `[foo 0, list-item 0]` / `[list-item 0]`) | 2 (the author value replaces the UA rule's, so `#x` instantiates no `list-item`) | I2 × I4 |

The log and source are beside the others (`lch/lch-probe4.log`, `lch/probe_lch4_tests.rs`), not durable.

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
  E0's "no hints in `BrowserCore` / `App`" is a ratified, landed invariant (E0 §5 item 4; #406). On IBP's side the
  ratified parts are a rule and a premise: the cell test with its ownership rule (IBP §0.6 item 7), and the premise
  that HTML §15's "expected" is read as "must" (IBP §0.2, `:664`–`:666`). The "core-path dimension hints" repair
  shape (Revision 38 item (1), the §2 pair `IBP-classify × IBP-layout`, Appendix D) is **input** by IBP's
  approval-scope paragraph. And the rule does not yet say the repair is owed: the cell test defines its markup M
  "under elidex's **current** cascade (UA and compat sheets included)" (`:886`–`:887`), so whether a core-mode
  `<iframe width=600>` is a cell at all is `IBP-layout`'s to derive. So the conflict is between one ratified
  decision, one ratified premise whose reach over engine modes nobody has stated (§1.2), and one unreviewed repair
  shape — plus this slot.
- **N-1 … N-5** are defects of the same machine and the same spec section the slot names; this memo folds them
  into the program (§5) and asks the PM to approve that growth (OQ-3). **N-6, N-7** are outside the program (§7).

### §0.4 Decisions

Each line names the ground rung that decides it.

- **D-1 — one writer.** The `counter_reset` / `counter_increment` / `counter_set` fields of `ComputedStyle` are
  written by the cascade's resolve step and by nothing else. The post-cascade bake
  (`apply_implicit_list_counters_from_dom`, `apply_implicit_list_counters`, `count_li_children`) is deleted. On the
  way it only **loses** work whose producer has moved (LC-1: the increment arm with its guard, and the `li` count);
  it never gains an input, a guard or a second caller, and it never shares an element's property with the cascade.
  *Rung 1*: CSS Lists 3 §4.6 ("This does not affect the specified or computed values of counter-increment"); HTML
  §15.3.7 puts the reset and the set in the cascade (a UA rule and two hints).
- **D-2 — the implicit increment is counter processing's.** A box whose computed `display` is `list-item` increments
  `list-item` by 1 (by −1 when the innermost `list-item` counter on it is reversed) at the increment step, unless
  its computed `counter-increment` names `list-item`. Tag names play no part. *Rung 1*: §4.6.
- **D-3 — counter processing is CSS Lists 3 §4's.** Steps 1–5 in the spec's order (inherit, reset, increment, set,
  use); the counters-set model of §4.4.1 / §4.4.2 (a counter's scope is its creator's descendants **and following
  siblings with their descendants**; instantiating replaces a same-name counter that originates from the element
  itself or a previous sibling); §4.5 (a boxless element does nothing). *Rung 1*.
- **D-4 — reversed counters.** An explicit increment applies as written; only the implicit one is −1. A reversed
  counter instantiated without an integer gets §4.4.2's value. That value is **state of the counter system's walk**:
  computed when the step instantiates the counter, over the boxes that exist, on every run of either driver; it is
  never stored in the computed value and it is not a component. `CounterResetEntry` gains an absent-integer state
  for the reversed form (the reading of §4.1's *Computed value* line: §3.2, F7 note). *Rung 1*: §4.4.2 ("at
  layout-time"), §4.6. *Rung 2* for "not a component": a derived value becomes a component when a system other than
  its writer reads it (§0.5 row 4's criterion); at this base no such reader exists.
- **D-5 — `counter-*` resolve independently of `content`** (N-1). *Rung 1*: appliesTo "all elements".
- **D-6 — the UA rule is in the core UA sheet.** `ol, ul, menu { counter-reset: list-item; }` goes into `UA_CSS`
  (`crates/css/elidex-style/src/ua.rs`). *Rung 1* gives the rule's text (HTML §15.3.7) and, in every engine mode,
  CSS Lists 3 §4.6's "should" (the UA sheet reflects the host language's numbering). *Rung 2* gives the sheet:
  #408's criterion for UA rules (rules of conforming elements → the core sheet, applied in every mode; `ol`, `ul`,
  `menu` are conforming, HTML §4.4.5 – §4.4.7). Whether rung 1 makes the rule a "must" in a given mode is the
  designation reading of §1.2; the decision is the same under every reading and every option of §1.6. `dir`: §4.3.
- **D-7 — the list attributes are presentational hints, at the hint position.** `ol[start]` / `[reversed]` →
  `counter-reset` and `li[value]` → `counter-set` enter the cascade through the existing hint seam
  (`extra_declarations` of `collect_and_cascade`, `crates/css/elidex-style/src/cascade/mod.rs:175`), so an author
  declaration of the same property replaces them and the UA rule sits below them. *Rung 1*: HTML §15.3.7, §15.2.
  **In which engine modes they are generated is P-1 (§1), not decided here.** The hint output is a function of the
  element's `TagType` and of its `start` / `reversed` / `value` (and, in LC-5, `type`) attributes: a change of any
  of them is a style-input change of that element (§2, I2).
- **D-8 — attribute values are parsed by HTML's rules for parsing integers** (§2.3.4.1). *Rung 1*. Through **one
  shared parser**, not a fifth copy (§7 SQ-1): *rung 2*, One issue, one way.
- **D-9 — one counter step, any number of drivers.** The pre-layout pass and render's paged walk drive the same
  step and add no list logic of their own; after D-1 neither calls a bake, and both skip the same boxless set
  (§2, I7 × I9). Whether the paged driver should exist at all is the master plan's settled question
  (`generated_content.rs` module doc: per-page values need post-pagination assignment) and is not reopened.
  *Rung 2*: One issue, one way.
- **D-10 — the resolved value comes through the property-handler seam, and `counter-*` keep one parse and one
  resolve implementation.** `getComputedStyle` serves `counter-*` from `CssPropertyHandler::get_computed` like every
  other property; no special case in `get_computed_with_registry`. The trait also carries `parse` and `resolve`
  (`crates/core/elidex-plugin/src/traits.rs:103`, `:113`): for `counter-*` the single parse path is the declaration
  parser in `elidex-css` and the single resolve path is the cascade's resolve step, and the handler's two
  trait-forced entries call those implementations — they hold no copy (§5.3 LC-6). It lands after D-1, so the first
  value script sees is the cascade's. *Rung 1*: CSSOM 1 §9. *Rung 2*: Plugin-first (the seam); One issue, one way
  (one implementation).
- **D-11 — HTML §15.3.7 is taken as a whole section.** §4.3 lists every normative unit of the section with its
  disposition; none is left unnamed, and a unit this program does not repair is routed with an owner or a proposed
  slot (§7).
- **D-12 — six slices after a split prerequisite, in the order §5.2 derives.** The order satisfies the constraints
  c1 – c10 of §5.2, each of which is a cell that is right today and would turn wrong under another order; the
  exhaustive table of §5.2 shows no cell turning wrong in any window of the chosen order.

### §0.5 OO idioms translated (ECS-native first)

| Shape in the code today | ECS-native shape this program lands |
|---|---|
| A post-cascade fix-up mutates the style object by element class (`match tag { "ol" … "li" … }`), re-run by three callers "to be safe" | The style component has one writer (the cascade system). Markup facts reach it as cascade inputs: a UA rule and a pure function `(TagType, Attributes) → declarations` |
| The numbering state is smuggled through the style object (a `reversed` flag and a pre-counted integer baked into `counter_reset`; the increment's sign fixed up inside the machine) | Counter values are **derived** by a system that reads `ComputedStyle` in tree order and writes `ListItemMarker` / pseudo `TextContent`; nothing derived is written back into `ComputedStyle` |
| "Is this a list item?" asked of the element's class (tag) | Asked of the component the cascade wrote (`display`) |
| Per-element counters set, and a reversed counter's initial value (the spec's model is per element) | Walk-local state of the counter system, not a component and not an entity-keyed side store: at this base the only readers are the system's own two drivers (`git grep -n 'CounterState' -- crates`, non-test: `generated_content.rs`, `elidex-render/src/builder/mod.rs` and `walk.rs`, and a doc comment in `builder/paint/mod.rs`), and each run recomputes it from components (`ComputedStyle` in tree order), so the two drivers cannot disagree. **Criterion for promoting it**: a reader outside the system (accessibility ordinals, say, §7 F-8) makes the counters set a component written by the pre-layout pass; the paged driver then reads it instead of recomputing |

---

## §1. Hint placement: derivation, and proposal P-1

### §1.1 Rung 1 — what the specs say

- **Following the Rendering section is a designation, and the designation is the UA's choice.** HTML §2.1.8: "User
  agents, whether interactive or not, may be designated (possibly as a user option) as supporting the suggested
  default rendering defined by this specification. This is not required. … User agents that are designated as
  supporting the suggested default rendering must, while so designated, implement the rules the Rendering section
  defines as the behavior that user agents are expected to implement." HTML §15's prelude: "User agents are not
  required to present HTML documents in any particular way. … For the purposes of conformance for user agents
  designated as supporting the suggested default rendering, the term "expected" in this section has the same
  conformance implications as "must"."
- **The designation covers the whole section.** Neither sentence separates UA rules from presentational hints, a
  conforming attribute from an obsolete one, or a conforming element from an obsolete one. For a designated UA the
  `bgcolor` hint is as much a "must" as `ol[start]`'s, and so is a rule for an obsolete element (§15.3.7 itself
  lists `dir`).
- **One cascade position, no sub-classes.** HTML §15.2: "The CSS rules given in these subsections are, except where
  otherwise specified, expected to be used as part of the user-agent level style sheet defaults … Some rules are
  intended for the author-level zero-specificity presentational hints part of the CSS cascade; these are explicitly
  called out as presentational hints." CSS Cascade 5 §6.5: "The UA may choose to honor presentational hints in a
  source document's markup … All document language-based styling must be translated to corresponding CSS rules and
  enter the cascade as rules in either the UA-origin or a special-purpose author presentational hint origin between
  the regular user origin and the author origin."
- **HTML has three attribute classes, not two.** Current-language attributes: `reversed`, `start`, `type` on `ol`
  (§4.4.5) and `value` on `li` (§4.4.8), with normative meaning (the list's *starting value*, the item's *ordinal
  value*). §16.1 *obsolete but conforming*: "Authors should not specify a border attribute on an img element. If
  the attribute is present, its value must be the string "0". CSS should be used instead." §16.2 *non-conforming*:
  attributes that "must not be used by authors", among them `type on li elements`, `type on ul elements`, and the
  `bgcolor` / `align` / table-dimension family.
- **The list-item counter carries the host language's numbering, whatever the designation.** CSS Lists 3 §4.6: "UAs
  and host languages should ensure that the list-item counter values by default reflect the underlying numeric
  value dictated by host language semantics when setting up list item styling in their UA style sheet and
  presentational hint style mappings." This is a normative "should" of a CSS module, not part of HTML §15.

With no reading added, rung 1 fixes two things: the cascade position (D-7), and a "should" that list numbering
reflects `start` / `reversed` / `value` wherever list items are numbered. Everything else it fixes only **given a
designation** — and it does not say which elidex engine mode is designated.

### §1.2 The designation readings (part of OQ-1; not chosen here)

No elidex record states which engine mode, if any, is "designated": `git grep -n -i -E 'suggested default
rendering|designated as supporting|conformance class' -- docs CLAUDE.md crates .claude/skills` has one hit outside
this memo, IBP's quotation of the §15 prelude (`docs/plans/2026-09-inline-box-predicate.md:666`); the same pattern
over the memory directory and the PM portfolio has none. The spec permits each reading below ("may", "possibly as
a user option", "This is not required"), so **choosing among them is not rung 1**: it is elidex's choice, and rung
1's requirements then follow from it.

| | **DR-1** — every engine mode is designated | **DR-2** — the designation is the engine mode: `BrowserCompat` is designated, `BrowserCore` / `App` are not (the mode is §2.1.8's "user option") | **DR-3** — no mode is designated |
|---|---|---|---|
| HTML §15 in `BrowserCompat` | every rule and every hint is a "must" | every rule and every hint is a "must" | advisory |
| HTML §15 in `BrowserCore` / `App` | every rule and every hint is a "must", obsolete ones included | advisory: HTML requires neither rules nor hints there | advisory |
| IBP §0.2 "HTML §15's *expected* is read as **must**" (`:664`–`:666`; a §0 premise, ratified by #526; stated without a mode; IBP's cell test evaluates under the current, compat-included cascade, `:886`–`:887`) | consistent | consistent (production is `BrowserCompat`) | **contradicts** |
| E0 (core arm: "no legacy UA, no hints", §5 item 4) | **contradicts** (both halves of the gate) | consistent — E0's gate is the user option | consistent |
| #408 (conforming-element rules in the core sheet "applied in every engine mode"; obsolete-element rules compat-gated) | **contradicts** its compat half (those rules are "must" in core modes too) | consistent; the core half is a rung-2 selection from an advisory section, not a §15 "must" | consistent, likewise |
| CLAUDE.md *Whole-engine core / compat / deprecated consistency* | **contradicts** (legacy rendering in core) | consistent | consistent |
| D-6's UA rule in core modes | "must" | CSS Lists 3 §4.6 "should" + rung 2 | the same |

DR-2 is the only reading consistent with all three ratified records, and DR-1 is the only one under which rung 1
by itself decides the hint population (every hint, every mode). **Uncertain, and for the ratifier:** whether elidex
reads E0's compat gate as the "user option" §2.1.8 permits (DR-2). Nothing recorded says so; IBP's premise was
written without a mode, and IBP's unreviewed input (the §2 pair, Appendix D) treats a rendering-section hint as owed
in core modes, which is DR-1's consequence for that hint, or option B's (§1.5). The same reading question governs
§16.3's requirements for obsolete features (§4.3, `dir`).

### §1.3 What rung 1 decides under each reading

- **DR-1** → every hint in every mode, and the legacy UA sheet in every mode: option **C** (§1.6). Rung 2 is not
  consulted; E0's gate has nothing left to gate.
- **DR-2, DR-3** → in `BrowserCore` / `App` HTML requires nothing. CSS Lists 3 §4.6 still asks, at "should"
  strength, that numbering reflect the host language's values: the UA rule (D-6) and the `start` / `reversed` /
  `value` mappings. A "should" may be set aside for a stated reason; elidex's principles supply none (these are
  conforming attributes with normative meaning; the core sheet already numbers lists: `li { display: list-item }`,
  `ol { list-style-type: decimal }`), so **rung 1 rejects option A for these three hints**. It decides nothing
  for any other hint — `ol[type]`, the width / height family, §15.5.9's color well. Those fall to rung 2.

### §1.4 Rung 2 — elidex's principles, for what rung 1 leaves (DR-2 / DR-3 only)

Lens block (one line per principle: which option it selects; options in §1.6).

- *Whole-engine core / compat / deprecated consistency* ("Legacy / non-standard … を core に混ぜない。Core に置くなら
  「modern standard baseline として必要」な理由"): selects **B** over B0 and A — a hint that maps a current-language
  feature is the rendering of modern markup; rejects **C** (`bgcolor` in core).
- *One issue, one way*: selects **B**. The §15.2 surface has two halves, UA rules and hints. The rule half is
  split by spec status (#408). B gives the hint half the same criterion. A keeps two criteria for one surface
  (status for rules, "is a hint" for hints); **B0** keeps A's two criteria and adds an exception list ("unless
  another spec asks for the mapping"), which every later hint must be checked against.
- *Ideal over pragmatic*: selects **B** over B0 (B0 answers this slot's hints and leaves the criterion undecided).
- *ECS-native first*: indifferent — under every option the hint source is a pure function of two components.
- *Supported-surface testing*: indifferent between B0 and B; under A the core arm and every hint-less cascade
  entry need cells that pin the dropped numbering (§5.3, the entry-point table).

Every principle that discriminates selects B. Rung 3 is not reached.

### §1.5 Conclusion — proposal P-1 (for the PM to route to E0's ratifier; not decided here)

> **P-1a.** The list-numbering mappings — the UA rule `ol, ul, menu { counter-reset: list-item; }` and the hints of
> `ol[start]`, `ol[reversed]`, `li[value]` — apply in every engine mode. *Ground: rung 1 under DR-2 and DR-3 (CSS
> Lists 3 §4.6, "should"); under DR-1 it is part of C.*
>
> **P-1b.** In general, a presentational hint's layer follows the status of the markup feature it maps (a content
> attribute, or element state such as §15.5.9's color-well value): a current-language feature → **core**, generated
> in every engine mode; a feature HTML §16 lists as obsolete — §16.2 non-conforming, and §16.1 obsolete but
> conforming (the spec's own name for it is obsolete, and it gives the CSS replacement) — → **compat**, behind
> `presentational_compat()` of `StyleCompatPolicy`. For this program: `ol[type]` is core; `li[type]`, `ul[type]`
> are compat. *Ground: rung 2 under DR-2 and DR-3.*

**Recommendation: B = P-1a + P-1b, on reading DR-2.** What changed from rev 1: the outcome for this program is the
same (rev 1's P-1 is P-1b), but rev 1 called rung 1 silent and decided everything at rung 2. Rung 1 is not silent:
(i) it decides P-1a by itself under DR-2 / DR-3; (ii) the general criterion P-1b is rung 2 and reaches hints this
slot does not own; (iii) both stand on a designation reading the spec leaves to the UA and no elidex record states —
under DR-1 the answer is C, not B. So the recommendation is **conditional on the reading**, and the reading is put
to the ratifier with it (OQ-1). If the ratifier accepts only P-1a (option B0), LC-3 is unblocked and LC-5's `ol[type]`
stays compat.

**Does it overturn E0?** Under B and B0 E0's mechanism stands: one `EngineMode`, `StyleCompatPolicy` derived in
parallel, one boolean gating "the legacy UA sheet + the compat hint generator", `elidex-style` mode-agnostic. What
changes is **what the gate's hint half covers**, which contradicts E0's ratified text in the places §1.7 lists. So
it is an amendment of a ratified decision and is the ratifier's to accept. Under C E0's gate is removed.

### §1.6 The options, and what each keeps and overturns

| | **A** — every HTML hint is compat (E0 as written) | **B0** — A, except the list-numbering hints (P-1a only) | **B** — by the mapped feature's status (P-1a + P-1b) | **C** — every hint, and every §15 rule, in every mode |
|---|---|---|---|---|
| Reading it rests on | DR-2 or DR-3, and setting §4.6's "should" aside | DR-2 or DR-3 | DR-2 or DR-3 | DR-1 |
| Rung | none: rung 1 (§4.6) rejects it for three hints | rung 1 | rung 1 (P-1a) + rung 2 (P-1b) | rung 1 |
| Keeps | every passage of §1.7 verbatim; design doc §15.3 / §29.5's listing of width / height hints under the compat layer | E0's mechanism; all of §1.7 except its "every hint" wording; `engine_mode_gates_presentational_hints`; today's mode-independent list numbering | E0's mechanism and its one-boolean policy; #408's criterion; the three-layer rule; `engine_mode_gates_presentational_hints` (it pins `<font color>`, an obsolete attribute); today's mode-independent list numbering | today's list numbering |
| Overturns, or costs | no ratified record. Today's behaviour: `start` / `value` / `reversed` apply in every mode and at every cascade entry (§0.2); under A, LC-3 **removes** them from `BrowserCore` / `App` and from every hint-less entry, the WPT harness included (§5.3). IBP Revision 38's repair shape | E0's "every hint" sentences (§1.7), for three hints. Leaves two criteria plus an exception on the §15.2 surface; `ol[type]` and `IBP-layout`'s question stay as A | E0's sentences of §1.7 (they become "legacy presentational hints"); design doc §15.3 / §29.5's placement of width / height hints, once `IBP-layout` applies the criterion | E0's gate entirely, with its plan; #408's compat sheet (the obsolete-element rules become core); the three-layer rule for styles; `engine_mode_gates_presentational_hints` |
| This program under it | LC-3 puts the list hints in the compat generator; its plan adds cells pinning "numbered from 1" in core modes and at hint-less entries (OQ-5); LC-5's `type` hints all compat | LC-3 adds a core hint source (OQ-2) for the three hints; LC-5's `type` hints all compat | LC-3 adds a core hint source (OQ-2); LC-5 puts `ol[type]` there and `li[type]` / `ul[type]` in the compat generator | as B for this program's hints, with `li[type]` / `ul[type]` core; moving every other hint and the legacy sheet is outside this slot |
| §15.5.9's color-well hint (maps the element's value; `IBP-layout`'s) | compat | compat | core (current-language state) | core |

### §1.7 The ratified and landed text each option amends

"Amends" = the passage is no longer true as written. A amends none; B0, B and C amend the same passages (B0 for
three hints, B for every current-feature hint, C for all), except where a column says otherwise.

| Passage | Says | B0 / B | C |
|---|---|---|---|
| E0 plan §0 (`docs/plans/2026-06-e0-shell-style-compat-mode.md:26`–`:30`) | the compat path "applies HTML presentational hints"; a core session resolves "against the modern UA baseline only" | amends: the baseline includes the core hints | amends |
| E0 §1a (`:43`–`:46`) | `presentational_compat()`: "Legacy UA stylesheet + HTML presentational-hint declarations … `BrowserCore`/`App` → false" | amends ("legacy presentational-hint declarations") | void |
| E0 §1a (`:59`–`:63`) | one boolean; "can grow a field if a future mode must split "legacy UA sheet" from "presentational hints"" | kept: the boolean still gates the legacy sheet and the legacy hints together; the split is inside the hint set, along #408's line, not the one E0 foresaw | void |
| E0 §1c (`:79`–`:86`) | core arm = "`resolve_styles` semantics (empty extra-UA, no hints, no registry)"; `no_hints` is "no compat data" | amends "no hints" → "no compat hints" (the core source is not the caller's argument, §5.3) | void |
| E0 note on `registry` (`:93`–`:96`) | "the *only* real compat-vs-core delta is the legacy UA sheet + presentational hints" | amends (the locus the slot text itself names) | void |
| E0 §2 Axis 3 (`:112`–`:116`) | the core arm is test-only until `#11-async-core-storage-cookiestore` | kept (a fact; it is why A's loss ships unseen) | kept |
| E0 §2 Axis 4 (`:117`–`:120`) | "The compat layer (legacy UA sheet + presentational hints) … E0 only gates *whether* it applies" | amends | void |
| E0 §3 lead and rows (`:128`–`:132`, `:136`, `:137`) | "compat = full §15.2 surface applied; core = withheld"; hints "BrowserCore+App: withheld" | amends. (The rule half of this sentence was already untrue at landing: #408, E0's prerequisite, put §15.3's conforming rules in the core sheet) | void |
| E0 §3.1 (`:147`–`:153`) | the gated conversion is of "`bgcolor`/`width`/`align`/…" — E0 gated the conforming `width` knowingly | B0: kept. B: amends for `width` / `height`, when `IBP-layout` applies P-1b | void |
| E0 §5 item 4 (`:193`–`:194`) | "`BrowserCore`/`App` path = core resolution (no legacy UA, no hints)" | amends | void |
| E0 §5 item 5 (`:195`) | "Compat algorithm stays in `elidex-dom-compat`" | kept for this program (the list hint source is new code); under B a conforming-attribute hint is not compat, so `IBP-layout`'s move takes that conversion out of `elidex-dom-compat` | void |
| `StyleCompatPolicy` rustdoc (`crates/core/elidex-plugin/src/spec_level.rs:203`–`:204`, `:206`–`:209`, `:218`–`:219`, `:224`–`:226`) | the same sentences as E0 §1a | amends; LC-3 edits them | void |
| Shell comments landed with #406 (`crates/shell/elidex-shell/src/lib.rs:138`–`:145`, `:160`–`:161`) and the `hint_generator` doc (`crates/css/elidex-style/src/lib.rs:328`–`:330`) | core arm: "no legacy UA sheet, no presentational hints" | amends; LC-3 edits them (not ratified text) | void |
| A0 anchor design §3.2b (`docs/plans/2026-06-web-api-compat-split-design.md:516`) | "style layer derives: style-compat policy (UA sheet + presentational hints on/off)" | amends (one on/off unit → the legacy unit) | void |
| A0 §5, E0 row (`:750`–`:755`) | the policy derives from the engine-wide `EngineMode`, parallel to `SpecLevelPolicy` | kept | void (nothing to derive) |
| Design docs `docs/design/ja/15-rendering-pipeline.md` §15.3 (`:66`–`:67`), `29-survey-analysis.md` §29.5 (`:165`–`:169`) | width / height hints are a P0 item "of the compat layer" | B0: kept. B: amends, when `IBP-layout` applies P-1b | amends |
| #408's criterion (`crates/css/elidex-style/src/ua.rs:111`–`:116`; `crates/dom/elidex-dom-compat/src/legacy_ua.rs` header) | conforming-element rules → core; §16.2-obsolete → compat | kept, and extended to hints by B | amends (no compat sheet) |

### §1.8 Precedents on both sides

**Search.** By concept, in English and Japanese, over the repo's docs, the memory directory and the PM portfolio:

```
P='presentational|presentation[ -]attribute|プレゼンテーション|プレゼンテーショナル|表示属性|属性ヒント|スタイルヒント|presentation hint|bgcolor'
git grep -l -i -E "$P" -- docs CLAUDE.md .claude            # 25 files (rev 1's English-only pattern: 21)
git grep -n -i -E 'ヒント|[^a-z]hints?[^a-z]' -- docs/design/ja CLAUDE.md   # second sweep, read at every hit
grep -rl -i -E "$P|ヒント|StyleCompatPolicy|style[- ]compat|#406|#408|legacy UA" <memory dir>                # 6 files
grep -n  -i -E "<the same>" <PM dir>/PORTFOLIO-2026-10-26.md PORTFOLIO-archive-*.md PORTFOLIO-index.md   # 2 lines
```

Control (a known hit the first pattern must find, which rev 1's missed): `docs/design/ja/19-svg-rendering.md:92`,
`:94`, `:99`, `:103` — found. Read at their hits: every file under `docs/design/ja`, the plans, the audit, the six
memory files, the two portfolio lines and the PM decisions they cite (PM 611, 620, 629, 637, 985). Not read
separately: the `docs/design/en` translations. Incidental hits (the word, or `presentational.rs`, with no placement
statement): the line-box, tree-model (two), AbortSignal and EH-attrs plans, the PR-A2 memo (the slot text itself),
`06-thread-model.md:600` (surface presentation), `25-accessibility.md:78` (ARIA "presentational"),
`11-parser-design.md:155` (a usage figure), memory `project_html-section-renumber-sweep.md:43` and
`project_pr396-c1-viewport-structural-pivot.md:49`. What the sweep cannot see: a decision recorded under none of
these words — the PM adds those when routing.

| Record | Status | Says | Side |
|---|---|---|---|
| E0 plan, the passages of §1.7; landed #406 (`f39f16d2`) | ratified, landed | the core arm gets "no legacy UA sheet, no presentational hints" | A |
| `StyleCompatPolicy` rustdoc (§1.7) | landed with #406 | one field gates "legacy UA stylesheet + HTML presentational hints — WHATWG HTML §15.2" | A |
| Anchor design (A0) §3.2b, `:516` | ratified | hints are one on / off unit | A |
| PM 985 (`PORTFOLIO-2026-10-26.md:438`); PM 620 (`PORTFOLIO-archive-2026-W46.md:54`); memory `project_line-box-decorated-inline-content.md:464` | PM decisions | describe E0 as "presentational hint はすべて compat" and route the conflict to this slot's plan-review; a conclusion that overturns E0 goes to E0's ratifier | A as a description of E0; no new placement |
| PM 611 (`PORTFOLIO-archive-2026-W46.md:45`): conforming attributes → "hint の源は core、compat は廃止属性の hint だけ" | **withdrawn** by PM 620 ("2 つの ratify 済み面を読まずに") | P-1b's outcome | neither (withdrawn) |
| Design docs `15-rendering-pipeline.md` §15.3 (`:66`–`:67`), `29-survey-analysis.md` §29.5 (`:165`–`:169`) | design SSoT | width / height hints are a P0 item of the compat layer ("compat 層実装優先度") / of the "Compatルール優先度" | A |
| Design doc `29-survey-analysis.md` §29.2 (`:53`–`:59`) | design SSoT | counts `width` / `height` under the heading 非推奨属性, and in the same paragraph calls them size hints that prevent layout shift, a pattern recommended today | both |
| Design docs `12-dom-cssom.md` §12.2.2 (`:144`), `29-survey-analysis.md` §29.7 (`:210`) | design SSoT | "presentational hints対応はP0要件。StyleSystemはスタイル解決時にこれらをCSS初期値として扱う必要がある"; "Phase 1のCSSパーサーでpresentational hintsをサポートする必要がある" — the style system and Phase 1 are named, no compat layer | lean B |
| Design doc `19-svg-rendering.md` §19.2.2 (`:92`–`:103`) | design SSoT | SVG presentation attributes enter the cascade as author-level, specificity-0 declarations, handled by the StyleSystem, "特別なパスは不要", no compat layer | B, by analogy: another current-language attribute → property mapping (SVG, not HTML; E0's text is about HTML hints) |
| — the design SSoT taken together | | split: §15.3 / §29.5 place the HTML hints in the compat layer; §12.2.2 / §29.7 / §19.2.2 place attribute mappings in the style system | split |
| #408 (`75c327c1`), E0's prerequisite; `ua.rs:111`–`:116`, `legacy_ua.rs` header | landed | "Reclassify UA-stylesheet rules by spec status": conforming → core sheet "applied in every engine mode"; only §16.2-obsolete → compat. Cause: E0's core arm had dropped "the only source of standard §15.3 rendering" | B (the criterion, applied to the rule half) |
| #408, the same PR | landed | (i) its criterion has an exception: `<tt>` is §16.2-obsolete and stays in the core sheet, for reach into shadow trees (`ua.rs:68`–`:73`); (ii) its PR body calls the hints "legacy/presentational compat" and the compat cascade "(`ua` + `legacy` + hints)" (`gh pr view 408 --json body`, body lines 6 and 33) | (i) placement was not decided by status alone; (ii) A for hints |
| CLAUDE.md *Whole-engine core/compat/deprecated consistency* | standing rule | legacy stays out of core; core needs a "modern standard baseline" reason | B (rejects C) |
| Memory `policy-deprecated-and-annex-b-out-of-scope.md:19` | policy memo (its attribute list is its own: `git grep -n -i bgcolor -- docs/design/ja` → 0) | deprecated HTML attributes, "`width`/`height` (presentational on most elements)" and "`type` (UL)" among them, are outside core scope | A, and B's compat half; silent on conforming attributes |
| IBP §0.2 (`:664`–`:666`) | ratified premise (#526) | HTML §15's "expected" is read as "must" | bears on the reading (§1.2), not on a layer |
| IBP §0.6 item 7, the cell test (`:886`–`:892`) | ratified rule | M is markup "under elidex's **current** cascade (UA and compat sheets included)"; ownership of an exposure is the exposer's | neither: whether core-mode iframe sizing is a cell is `IBP-layout`'s to derive |
| IBP Revision 38 item (1) (`:279`–`:282`); §2 pair `IBP-classify × IBP-layout` (`:1269`–`:1275`); Appendix D (`:1859`–`:1863`); §3's §15.3.1 row (`:1312`) | **input**, not ratified (IBP's approval-scope paragraph, `:3`–`:9`) | the repair is "the dimension-attribute hints on the core path"; criterion: "a rendering-section hint, not a legacy presentational attribute" | B's outcome for those hints, by a **different criterion**: IBP's separates hints by where the spec defines them and does not say which hints are legacy; P-1b separates by the mapped feature's status. For §15.5.9 (the hint maps the element's value, not a content attribute) IBP's wording says core; P-1b says core only because it is worded over "markup feature", not "attribute" |
| Shell test `engine_mode_gates_presentational_hints` (`crates/shell/elidex-shell/src/tests.rs:925`) | landed | `BrowserCore` drops `<font color>` | A and B (obsolete attribute) |
| The bake (`counter.rs:298`), probes §0.2 | behaviour, never ratified | `start` / `value` / `reversed` apply in every mode and at every cascade entry | B0 / B in effect |

### §1.9 What this memo does not touch

The existing conforming-attribute hints that are compat-gated today (the width / height family, N-6) stay where
they are; moving them is `IBP-layout`'s repair, under whatever the ratifier decides for P-1b. `dir` the attribute
(N-7) is a UA rule mis-homed as a hint and is routed in §7; `dir` the element is §4.3's.

---

## §2. Coupled invariants

The program holds these at once. Each slice's plan re-derives the pairs it touches.

- **I1 — single writer**: only the cascade's resolve step writes the computed `counter-*` fields (D-1, D-5).
- **I2 — cascade position and inputs**: UA rule < hint < any author declaration, per property (D-6, D-7). The hint
  is a function of the element's tag and of its `start` / `reversed` / `value` / `type` attributes, so those
  attributes are **style inputs**: today every attribute write bumps the tree version
  (`crates/core/elidex-ecs/src/dom/attribute.rs:124`, `:291`; `:227`), the event loop re-renders on a changed
  `inclusive_descendants_version` (`crates/shell/elidex-shell/src/content/event_loop.rs:440`–`:447`) and the restyle
  is whole-tree, so the dependency holds structurally. An incremental restyle must honour it: a change of one of
  these attributes restyles that element.
- **I3 — step order**: inherit → reset → increment (explicit, then the implicit one) → set → use (D-3).
- **I4 — scope**: a counter is visible to its creator's descendants and following siblings with their descendants;
  a new same-name counter replaces one from the same element or a previous sibling and nests inside one from an
  ancestor (D-3). Counter inheritance runs on the flattened element tree (CSS Lists 3 §4.4.1, its note).
- **I5 — implicit increment**: keyed on computed `display: list-item` and on the computed `counter-increment` not
  naming `list-item`; its sign comes from the innermost `list-item` counter (D-2).
- **I6 — reversed initial value**: computed by the counter system over the boxes that exist, from the increments and
  sets in the counter's scope (D-4). It depends on the set of elements in the scope and on their `display`,
  increments and sets, so a counter pass that is ever made incremental re-runs a scope when any of those changes.
- **I7 — boxless elements do nothing**: `display: none` and `display: contents` (CSS Lists 3 §4.5).
- **I8 — engine modes**: the core UA sheet applies in every mode; the hint population per mode is P-1's outcome.
- **I9 — one step, two drivers**: the pre-layout pass and the paged walk (continuation suppression, `flatten`
  across pages, `set_counter` for `page` / `pages`) see the same step (D-9).
- **I10 — resolved value = computed value** for `counter-*` (D-10); it is the script-visible face of I1.
- **I11 — marker reconcile**: `ListItemMarker` is inserted or removed on every pass and on every exit path
  (`generated_content.rs`, the reconcile block before the `display: none` return). Unchanged.

**Pairs.** All 55 pairs were checked; the first table gives those that meet, the second the rest with the reason.
"By reading" marks a statement about today's code that no probe ran.

| Pair | Where they meet |
|---|---|
| I1 × I2 | an author `counter-reset: foo` on `ol[start]` removes both the hint's and the UA rule's `list-item` (Q3a); the bake's `already_has` guards approximated this per counter name and are deleted with it |
| I1 × I3 | today the bake compensates for the step's order: for `li[value]` it writes the set and **withholds** the increment (`counter.rs:349`–`:363`). With I1 the fields hold only what the cascade resolved, and I3 alone orders increment before set (P2a) |
| I1 × I5 | the implicit increment may never appear in a field: P1's `li` must read `counter-increment: none` |
| I1 × I6 | the reversed initial value may never appear in a field: `ol[reversed]` computes to `reversed(list-item)` with the absent-integer state, in the cascade and in `getComputedStyle` (§3.2, F7 note) |
| I1 × I7 | a boxless element still has its computed `counter-*` (the cascade writes them whatever `display` is; `getComputedStyle` shows them); only the step ignores them (§4.5: "still valid on such an element, but they must have no effect") |
| I1 × I9 | the paged driver clones the style and applies the bake today (`builder/walk.rs:134`); after D-1 it reads the component as written |
| I1 × I10 | the resolved value shows whatever writes the fields; D-10 lands after the bake is gone, or script would see the bake. The handler's `resolve` entry is not a second writer: it calls the cascade's implementation and no production path reaches it (§5.3 LC-6) |
| I2 × I4 | the UA rule is what makes each list a scope boundary; an author `counter-reset` that replaces it removes the boundary, and the nested list continues its parent's numbering (W-N1, W-N2) |
| I2 × I5 | `li[value]`'s `counter-set` and an author `counter-increment: list-item 2` are different properties: both apply (Q13, W-F2); nothing may key one on the other |
| I2 × I6 | `ol[reversed]` without `start` is the only hint that yields an absent integer; with `start` the hint carries one and §4.4.2 does not run; an author `counter-reset` replaces the hint and the list counts up |
| I2 × I8 | the same markup yields different cascade inputs per mode only through the hint population; the UA rule is mode-independent, so `menu` and nested numbering never vary by mode |
| I2 × I10 | the resolved value is the hint's where the hint wins and the author's where the author wins (the Q-rows read through `getComputedStyle`) |
| I3 × I4 | `counter-increment` / `counter-set` on a name with no counter instantiate one at this element (value 0) before acting; its scope is I4's, so the next sibling continues it (P11b) |
| I3 × I5 | the implicit increment runs in the increment step, before set: `li[value]` ends at the set value (P2a), not one above. So the order fix lands no later than the implicit increment's move into the step (§5.2 c8) |
| I3 × I6 | §4.4.2's loop reads each element in step order: its increment is negated into *lastNonZeroIncrementNegated*, then, if it sets the counter, the set value is added and the loop **breaks** without adding that increment — the set overrides the increment, as in I3 (W-RV: 11, 10, 9) |
| I3 × I9 | a continuation fragment re-enters the step. Today `is_continuation` skips the increments only (`counter.rs:113`), so, by reading, a continuation runs the element's reset and set again. Which of steps 2 – 4 a continuation runs is LC-1's to state (the paged driver's counters feed margin boxes only) |
| I3 × I11 | the marker is evaluated at step 5, after the element's own set (P2a's `li[value]` shows 5) |
| I4 × I5 | a list item with no list ancestor instantiates `list-item` on itself; its following siblings continue it (P11b); a `ul` between resets it (the UA rule) |
| I4 × I6 | the §4.4.2 walk visits "each element or pseudo-element that increments or sets the same counter in the same scope": the scope is I4's, and a nested same-name counter hides its own scope from the count. The walk and the step take the scope from one definition, so LC-4's change of the scope model moves both together |
| I4 × I7 | §4.4.1's "preceding sibling" and "parent" are of the flattened element tree: a `display: contents` element does nothing itself (I7) but its children inherit through it. LC-4's plan states the tree the model walks |
| I4 × I9 | the paged driver flattens the state between pages; the sibling-scope model must survive `flatten` (a counter created on a previous page's sibling is still in scope) |
| I5 × I6 | the implicit −1 of each list item in scope is an increment the §4.4.2 walk counts; `display: none` items (I7) are not. So the count's producer changes no later than the increment's key does (§5.2 c5, W-F3) |
| I5 × I7 | a `display: none` `li` neither increments nor is counted (P4b) |
| I5 × I8 | the implicit increment and the UA rule are mode-independent, so lists are numbered in every mode; a mode without the list hints numbers them from 1 (option A's core modes) |
| I5 × I9 | the implicit increment is an increment: a continuation fragment does not apply it again, and the §4.4.2 walk counts the element once, not once per fragment |
| I5 × I11 | "is a list item" has one key for the marker and for the increment. Today the marker is keyed on `display` and the increment on the tag, so a `display: list-item` `div` shows a marker reading "0" (P8b) |
| I6 × I9 | both drivers need the same initial value for the same counter; each computes it from the tree in its own run (D-4), reading the whole scope and not the page being painted |
| I7 × I9 | both drivers must skip the same boxless set. By reading, they do not today: the pre-layout pass returns for `none` and `contents` (`generated_content.rs:105`, `:114`), the paged walk filters `none` only (`builder/walk.rs:131`). LC-1 makes the step's caller gate one |
| I7 × I11 | the marker is removed before the boxless early returns (today's discipline); the counter change must keep that order |
| I8 × I10 | `getComputedStyle(ol).counterReset` differs by mode exactly where the hint population differs |
| I9 × I11 | markers are written by the pre-layout pass only; the paged driver writes none. Unchanged, and the reason a paged continuation never renumbers a marker |

| Pairs that do not meet | Reason |
|---|---|
| I1 × I4, I1 × I11 | I1 is about one component; scope is the walk's state and the marker is another component with another writer |
| I1 × I8, I4 × I8, I6 × I8 | modes act only through the hint population; that is I2 × I8 |
| I2 × I3, I2 × I7, I2 × I9, I2 × I11 | the cascade finishes before the step starts: position is settled per property, and the step, the boxless rule and both drivers read the result (I2 × I5 and I1 × I7 carry the two cases that look like meetings) |
| I3 × I7, I3 × I8, I3 × I10 | a boxless element has no step; the order does not vary by mode; the order is not script-visible except through values (I1 × I10) |
| I4 × I10, I4 × I11, I6 × I11, I10 × I11 | scope and the initial value reach script and the marker only as counter values; no shared state |
| I5 × I10, I6 × I10, I7 × I10 | the script-visible faces of I1 × I5, I1 × I6, I1 × I7 |
| I6 × I7 | the boxless element is not counted: I5 × I7 and I5 × I6 |
| I7 × I8, I8 × I9, I8 × I11, I9 × I10 | no shared input: modes act on the cascade, drivers on the walk |

---

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §2.1.8 Conformance classes | "may be designated … as supporting the suggested default rendering"; "must, while so designated, implement the rules the Rendering section defines" | designated / not; "possibly as a user option" | none — the reading input of OQ-1 (§1.2) | ✓ (three readings) | no |
| WHATWG HTML §15.3.7 Lists | UA rule `ol, ul, menu { counter-reset: list-item; }` | three selectors | `UA_CSS` in `ua.rs` (LC-3) | ✓ | no |
| WHATWG HTML §15.3.7 Lists | `ol[start]` / `[reversed]` → `counter-reset`, steps 1–4 | 3.1 reversed + integer (`reversed(list-item)` value + 1); 3.2 reversed, no integer; 4.1 integer (`list-item` value − 1); 4.2 no hint; parse error = no integer | the list hint source (NEW, LC-3) | ✓ | yes |
| WHATWG HTML §15.3.7 Lists | `li[value]` → `counter-set: list-item value` | parses / parse error (no hint) | the list hint source (NEW, LC-3) | ✓ | yes |
| WHATWG HTML §15.3.7 Lists | `type` hints; the non-counter UA rules; quirks rules; the non-CSS sentence | per unit, §4.3 | `ua.rs`, the hint sources (LC-5); units routed out or excluded per §4.3 | ✓ (§4.3) | yes (`type`) |
| WHATWG HTML §15.2 The CSS user agent style sheet and presentational hints | UA-level rules vs "author-level zero-specificity presentational hints" | rule / hint | `collect_and_cascade`'s hint source (unchanged seam) | ✓ | no |
| WHATWG HTML §2.3.4.1 Signed integers | rules for parsing integers, steps 1–9 | whitespace skip; `-`; `+`; non-digit error; trailing characters ignored | one shared parser (LC-3; SQ-1) | ✓ | yes |
| WHATWG HTML §4.4.5 The ol element | `reversed`, `start`, `type` are conforming content attributes | — | classification input to P-1 | ✗ (the *starting value* algorithm is not implemented as such: CSS UAs go through §15.3.7) | yes |
| WHATWG HTML §4.4.8 The li element | `value` is a conforming content attribute; *list owner*, *ordinal value* | — | classification input to P-1; no reader of ordinal values at this base (§7) | ✗ | yes |
| WHATWG HTML §16.1 Obsolete but conforming features | "Authors should not specify … CSS should be used instead" | the features with a §15 mapping (`border` on `img`) | classification input to P-1b: compat | ✗ (no list attribute is in §16.1) | yes |
| WHATWG HTML §16.2 Non-conforming features | obsolete attributes: `type on li elements`, `type on ul elements`; the `dir` element | — | classification input to P-1 (LC-5) | ✗ (the list rows only) | yes |
| WHATWG HTML §16.3.3 Other elements, attributes and APIs | "User agents must treat dir elements in a manner equivalent to ul elements in terms of semantics and for purposes of rendering" | the `dir` sentence | §4.3: `dir` mirrors `ul` in LC-3 (U7) and LC-5; `display` is `IBP-ua-display`'s | ✗ (that sentence only) | no |
| CSS Lists 3 §4 Automatic Numbering With Counters | steps 1–5 | the order | `CounterState::process_element` (LC-1) | ✓ | yes |
| CSS Lists 3 §4.1 Creating Counters: the counter-reset property | value grammar; *Computed value* line | `none`; name; name + integer; `reversed()`; `reversed()` + integer; several entries (the last of a name is honoured) | `parse_counter_list`, `resolve_counter_reset`, `CounterResetEntry` (LC-1) | ✓ | yes |
| CSS Lists 3 §4.2 Manipulating Counter Values: the counter-increment and counter-set properties | instantiate-if-absent; several entries of one name | increments compound; last set wins | `CounterState::process_element` (LC-1) | ✓ | yes |
| CSS Lists 3 §4.3 Nested Counters and Scope | scope | descendants; following siblings; obscured by a later sibling's reset | the scope model (LC-4) | ✓ | yes |
| CSS Lists 3 §4.4.1 Inheriting Counters | steps 1–5 | root; counter source = preceding sibling or parent; value source = preceding element in tree order; the flattened element tree | the scope model (LC-4) | ✓ | no |
| CSS Lists 3 §4.4.2 Instantiating Counters | instantiate steps 1–3; the initial-value algorithm steps 1–5 | replace (same element / previous sibling); nest (ancestor); loop break at a set; zero increments | LC-4 (instantiate); LC-1 (initial value) | ✓ | yes |
| CSS Lists 3 §4.5 Counters in elements that do not generate boxes | boxless elements | `display: none`; `content: none` pseudo; "replaced element's descendants … undefined" | the step's caller gate, one for both drivers (LC-1; the pseudo and replaced branches are its plan's to audit) | ✗ | yes |
| CSS Lists 3 §4.6 The Implicit list-item Counter | implicit increment; the "should" on UA sheets and hint mappings | list item; reversed (−1); `counter-increment` names `list-item`; "does not affect the specified or computed values" | LC-1; P-1a (§1) | ✓ | yes |
| CSS Lists 3 §4.7 Outputting Counters: the counter() and counters() functions | read | unchanged | `evaluate_counter`, `evaluate_counters` | ✗ (not touched) | no |
| CSS Cascade 5 §4.4 Computed Values | "the result of resolving the specified value as defined in the "Computed Value" line" | — | the reading of §4.1's line for `reversed()` (§3.2, F7 note; LC-1, LC-6) | ✗ (that definition only) | no |
| CSS Cascade 5 §6.5 Precedence of Non-CSS Presentational Hints | origin of hints | UA-origin or the author presentational hint origin; `revert` | the hint seam (unchanged; §7 F-3) | ✗ | no |
| CSSOM 1 §9 Resolved Values | default case: the resolved value is the computed value | `counter-*` are not in the special-case list | LC-6 | ✓ | no |

**Breadth**: K = 4 specs (HTML, CSS Lists 3, CSS Cascade 5, CSSOM 1), M = 24 (the rows of the table above) → past the
split-recommended threshold. **Split decision**: split — this memo is the umbrella, and §5's slices each carry a
subset (no slice exceeds two specs).

Section numbers and titles above were each looked up: `webref heading html {2.1.8, 15.2, 15.3.7, 2.3.4.1, 4.4.5,
4.4.8, 16.1, 16.2, 16.3.3}`; `webref heading css-lists-3 {4.1, 4.2, 4.3, 4.4.1, 4.4.2, 4.5, 4.6, 4.7}` and `webref
body css-lists-3 auto-numbering` (§4); `webref heading css-cascade-5 {4.4, 6.5}`; `webref heading cssom-1 9`. The
step numbers are those `webref body` prints: `html lists` (1–4, 3.1–3.2, 4.1–4.2), `html signed-integers` (1–9),
`css-lists-3 inheriting-counters` (1–5), `css-lists-3 instantiating-counters` (1–3 and 1–5). `preflight.py` verifies
the HTML rows only (its label table has no CSS-module labels, `#11-preflight-css-module-labels`).

### §3.1 User-input touch audit

- Author-controlled inputs: the `start`, `reversed`, `value`, `type` attributes and the `counter-*` declarations
  (stylesheets and `style=""`). All reach integers: values are `i32`, and arithmetic stays saturating as today
  (`saturating_add` in `process_element`; `start + 1` / `start − 1` in the hint). CSS Lists 3 §4 allows clamping
  ("the value must be clamped to that range").
- New reach: LC-1's §4.4.2 walk scans a counter's scope when a reversed counter is instantiated without an integer.
  That walk must be bounded (a scope is at most the document; nested reversed counters must not make it quadratic
  without a bound): LC-1's plan states the bound. After LC-2, author `counter-*` on any element takes effect (N-1),
  so the step runs for many more elements and an author sheet can create such counters anywhere.
- Adjacent pre-existing code: the depth cap `MAX_ANCESTOR_DEPTH` in `resolve_tree` is unchanged.

### §3.2 Normative text quoted (each from `webref body`, with its source)

- `# source: https://html.spec.whatwg.org/multipage/infrastructure.html#conformance-classes` — quoted in §1.1.
- `# source: https://html.spec.whatwg.org/multipage/rendering.html#rendering` — quoted in §1.1.
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
- `# source: https://html.spec.whatwg.org/multipage/obsolete.html#obsolete-but-conforming-features` — quoted in §1.1.
- `# source: https://html.spec.whatwg.org/multipage/obsolete.html#other-elements,-attributes-and-apis` — "User agents
  must treat dir elements in a manner equivalent to ul elements in terms of semantics and for purposes of rendering."
- `# source: https://drafts.csswg.org/css-lists-3/#auto-numbering` — "1. Existing counters are inherited from
  previous elements. 2. New counters are instantiated (counter-reset). 3. Counter values are incremented
  (counter-increment). 4. Counter values are explicitly set (counter-set). 5. Counter values are used
  (counter()/counters())."
- `# source: https://drafts.csswg.org/css-lists-3/#counter-reset` — "Computed value: the keyword none or a list, each
  item an identifier or a reversed() function paired with an integer"; "`<counter-name> <integer>?` Instantiates a
  counter of the given `<counter-name>` with a starting value of the given `<integer>`, defaulting to 0.
  `<reversed-counter-name> <integer>?` Instantiates a reversed counter of the given `<counter-name>` with a starting
  value of the given `<integer>`, or no starting value if not given."; "If multiple instances of the same
  `<counter-name>` occur in the property value, only the last one is honored."
- `# source: https://drafts.csswg.org/css-lists-3/#nested-counters` — "The scope of a counter therefore starts at the
  first element in the document that instantiates that counter and includes the element's descendants and its
  following siblings with their descendants. However, it does not include any elements in the scope of a counter
  with the same name created by a counter-reset on a later sibling of the element".
- `# source: https://drafts.csswg.org/css-lists-3/#inheriting-counters` — "3. Let counter source be the CSS counters
  set of element's preceding sibling, if it has one, or else of element's parent if it does not. 4. Let value source
  be the CSS counters set of the element immediately preceding element in tree order."; "Note: Counter inheritance,
  like regular CSS inheritance, operates on the "flattened element tree" in the context of the [DOM]."
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
  counters are normally incremented … This does not affect the specified or computed values of counter-increment.";
  the "should" sentence is quoted in §1.1.
- `# source: https://drafts.csswg.org/css-lists-3/#counters-without-boxes` — "An element that does not generate a box
  (for example, an element with display set to none, or a pseudo-element with content set to none) cannot set, reset,
  or increment a counter. The counter properties are still valid on such an element, but they must have no effect."
- `# source: https://drafts.csswg.org/css-cascade-5/#computed` — "The computed value is the result of resolving the
  specified value as defined in the "Computed Value" line of the property definition table, generally absolutizing
  it in preparation for inheritance. … serialization rules often allow omitting certain values which are implied
  during parsing; but those values are nonetheless part of the computed value."
- `# source: https://drafts.csswg.org/css-cascade-5/#used` — "The used value is the result of taking the computed
  value and completing any remaining calculations to make it the absolute theoretical value used in the formatting
  of the document."
- `# source: https://drafts.csswg.org/css-cascade-5/#preshint` — quoted in §1.1; also "For the purpose of cascading
  this author presentational hint origin is treated as an independent origin; however for the purpose of the revert
  keyword (but not for the revert-layer keyword) it is considered part of the author origin."
- `# source: https://html.spec.whatwg.org/multipage/common-microsyntaxes.html#signed-integers` — "4. Skip ASCII
  whitespace within input given position. … 8. Collect a sequence of code points that are ASCII digits from input
  given position, and interpret the resulting sequence as a base-ten integer."

**F7 note — the computed value of `reversed(name)` without an integer.** The texts pull apart. §4.1's *Computed
value* line reads "each item an identifier or a reversed() function **paired with an integer**"; its value prose
gives a reversed counter "no starting value if not given", and §4.4.2 supplies the value "at layout-time". For a
plain name the pairing is unproblematic: the omitted integer defaults to 0, an implied value that Cascade 5 §4.4
says is "nonetheless part of the computed value". For a reversed name there is no implied integer to pair: the
only integer comes from §4.4.2's walk over other elements at layout time, which is what Cascade 5 §4.5 calls
completing the remaining calculations — a used-value step, and one that depends on following siblings, which no
computed value can. **Reading taken**: the computed value of `reversed(name)` is the `reversed()` item with no
integer; of `reversed(name) 5`, the item paired with 5. *Rung 1*, by the more specific normative prose (§4.1's
value definition, §4.4.2) and Cascade 5's definitions against the summary line's letter. Should a plan-review find
rung 1 undecided, *rung 2* decides the same way: nothing the counter system derives is written back into
`ComputedStyle` (D-1, §0.5 row 2). The summary line's wording is a candidate spec issue (§7 F-13; not filed by us).
LC-1 (the representation) and LC-6 (the serialization) carry the reading and re-check the texts at their base.

### §3.3 Docstring citations for the new functions

Every function a slice adds or rewrites cites, in its doc comment, the spec step it implements; each slice's plan
carries this table for its own functions, with the names it settles on and each citation looked up with `webref`
at its base. Names below are placeholders.

| Function (slice) | Its doc cites |
|---|---|
| the counter step (LC-1) | CSS Lists 3 §4 Automatic Numbering With Counters, steps 2–4, in that order; §4.2 for instantiate-if-absent |
| the implicit increment (LC-1) | CSS Lists 3 §4.6 The Implicit list-item Counter — "incremented by 1 on every list item, or if the counter is reversed … by -1" and "unless the counter-increment property explicitly specifies" |
| the reversed initial value (LC-1) | CSS Lists 3 §4.4.2 Instantiating Counters, the initial-value algorithm steps 1–5 |
| the step's caller gate (LC-1) | CSS Lists 3 §4.5 Counters in elements that do not generate boxes |
| the `counter-reset` parse and resolve, the absent-integer state (LC-1) | CSS Lists 3 §4.1 Creating Counters: the counter-reset property, the `<reversed-counter-name> <integer>?` value definition |
| the list hint source: `ol` (LC-3) | HTML §15.3.7 Lists, the `start` / `reversed` steps 1–4 |
| the list hint source: `li` (LC-3) | HTML §15.3.7 Lists, the `value` mapping |
| the integer parser (LC-3) | HTML §2.3.4.1 Signed integers, the rules for parsing integers, steps 1–9 |
| the UA rule's comment in `UA_CSS` (LC-3) | HTML §15.3.7 Lists; CSS Lists 3 §4.6 (the "should") |
| inherit counters (LC-4) | CSS Lists 3 §4.4.1 Inheriting Counters, steps 1–5 |
| instantiate a counter (LC-4) | CSS Lists 3 §4.4.2 Instantiating Counters, steps 1–3; §4.3 Nested Counters and Scope |
| the `type` hint source and the added UA rules (LC-5) | HTML §15.3.7 Lists, each rule as §4.3 numbers it; §16.3.3 for `dir` |
| the handler's `get_computed` (LC-6) | CSSOM 1 §9 Resolved Values; CSS Lists 3 §4.1 / §4.2, the *Computed value* lines |

---

## §4. The end state

### §4.1 Facts and their homes

| Fact | Home at `bf1cae35` | Home at the end | Slice |
|---|---|---|---|
| `ol` / `ul` / `menu` reset `list-item` | the bake's `"ol"` / `"ul"` arms (`counter.rs:305`, `:335`); `menu` nowhere | a UA rule in `UA_CSS` | LC-3 |
| `start` / `reversed` → `counter-reset`; `value` → `counter-set` | the bake (`counter.rs:305`–`:352`), after the cascade | hint declarations, a pure function of `TagType` + `Attributes`, entering at the hint seam | LC-3 |
| The implicit increment | a baked `("list-item", 1)` in the computed field (`counter.rs:363`), keyed on the tag `li`; the pre-layout pass gated by tag (`generated_content.rs:124`, `:141`) | the counter step, keyed on computed `display` | LC-1 |
| The implicit increment's sign | `process_element` negates **every** increment of a reversed counter (`counter.rs:117`) | the step applies −1 for the implicit one only | LC-1 |
| A reversed counter's initial value with no integer | a baked `li_count + 1` (`counter.rs:313`–`:321`); `0` for an author `reversed(foo)` (`unwrap_or(0)`, `box_model/mod.rs:512`; the parser already stores the default as a number) | absent in the parsed and computed value; computed by the counter step per §4.4.2, walk-local (D-4) | LC-1 |
| Step order | reset → set → increment (`counter.rs:84`–`:128`) | reset → increment → set | LC-1 |
| Author `counter-*` on an element | resolved only when the element declares `content` (`box_model/mod.rs:399`) | resolved for every element | LC-2 |
| Counter scope | the creator's subtree (`push_scope` before the step, `pop_scope` after the children) | §4.3 / §4.4: subtree and following siblings; replace / nest on instantiate | LC-4 |
| Resolved value of `counter-*` | none: `get_computed_with_registry` falls to `get_initial_value` (`resolve/mod.rs:73`) | a registered handler's `get_computed` | LC-6 |
| `ol[type]`, `ul` / `menu` / `dir` marker styles, nested styles and margins, `menu` box rules | absent (N-5) | UA rules and hints per §4.3 | LC-5 |

### §4.2 The counter step (input to LC-1 and LC-4; mechanism is theirs)

One step per box-generating element or pseudo-element, in tree order, taking the element's `ComputedStyle` and
`is_continuation`: (1) inherit per §4.4.1; (2) each `counter-reset` entry instantiates per §4.4.2 (initial value:
the entry's integer, else for a reversed entry the §4.4.2 value); (3) each `counter-increment` entry increments,
instantiating first if absent; then, if `display` is `list-item` and no entry named `list-item`, the implicit ±1;
skipped for a continuation as today; (4) each `counter-set` entry sets, instantiating first if absent; (5) readers
(`evaluate_counter`, `evaluate_counters`) are unchanged. The step is the only place any of this lives; the drivers
only decide which entities get a step (I7, one gate for both) and in what order (I9). The §4.4.2 value needs the
elements of the counter's scope ahead of the current one, so its input is the tree, not only the current style;
how the step receives it is LC-1's mechanism.

### §4.3 HTML §15.3.7, unit by unit (D-11)

**`dir`, once for every row below.** `dir` is an obsolete element (§16.2), and HTML §16.3.3 says "User agents must
treat dir elements in a manner equivalent to ul elements in terms of semantics and for purposes of rendering" — a
"must" outside §15, so it does not hang on the designation. Two consequences. (1) Wherever elidex renders `dir` at
all, it gets **all** of `ul`'s rendering: every rule §15.3.7 lists it in, and also `ul`'s `counter-reset` (U7),
whose selector omits `dir` — a tension inside HTML that §16.3.3's "must" decides (candidate spec issue, §7 F-13);
never a subset of the rules. (2) In which modes that is: §16.3 is HTML's own requirements for obsolete features,
and whether `BrowserCore` / `App` implement them is the same reading question as §1.2 — under DR-1 `dir` joins the
core sheet beside `ul`; under DR-2 / DR-3 the three-layer rule and #408's criterion put an obsolete element's
rules in the compat sheet, which then mirrors `ul`'s whole rule set for `dir`. Rev 1 named the compat sheet without
the "must". At `bf1cae35` neither sheet has a `dir` rule (`/usr/bin/grep -n -E '\bdir\b' ua.rs legacy_ua.rs` → only
`bdo[dir=…]`), so nothing regresses either way. LC-3 lands `dir`'s `counter-reset` (it must precede LC-4, §5.2
c10); LC-5 the other non-`display` units; `display` is `IBP-ua-display`'s (§7 F-12).

| # | Unit (as `webref body html lists` gives it) | At `bf1cae35` | Disposition |
|---|---|---|---|
| U1 | `dir, dd, dl, dt, menu, ol, ul { display: block; }` | `ul, ol, dl, dt, dd` present (`ua.rs:16`); `menu` and `dir` compute to `inline` (R3, W-H5) | `display` rules are `IBP-ua-display`'s population (IBP §4, Appendix G); this memo hands it `menu` (§7 F-5) and, for `dir`, §16.3.3 (§7 F-12: IBP's Appendix G seeds `dir` as a compat rule without it) |
| U2 | `li { display: list-item; text-align: match-parent; }` | `display` present (`ua.rs:59`); `match-parent` has no hit in `crates` (`git grep -n -F -e 'match-parent' -e 'MatchParent' -- crates` → 0; control `"text-align"` → 32 lines) | `text-align: match-parent` needs a value the text module does not have → routed out (§7 F-6) |
| U3 | `dir, dl, menu, ol, ul { margin-block: 1em; }` | `ul, ol`, `dl` present as physical `margin-top` / `-bottom` (`ua.rs:53`–`:57`, `:63`); `menu`, `dir` absent | `menu`: LC-5; `dir`: LC-5, per the paragraph above. The physical longhands where the spec writes flow-relative ones: routed finding §7 F-9 |
| U4 | `:is(dir, dl, menu, ol, ul) :is(dir, dl, menu, ol, ul) { margin-block: 0; }` | absent (R4: 16px) | LC-5 (`dir` as above) |
| U5 | `dd { margin-inline-start: 40px; }` | present as physical `margin-left` (`ua.rs:64`) | not this program's to rewrite: routed finding §7 F-9 |
| U6 | `dir, menu, ol, ul { padding-inline-start: 40px; }` | `ul, ol` present as physical `padding-left`; `menu`, `dir` absent (R3) | `menu`: LC-5; `dir`: LC-5 as above; physical vs flow-relative: §7 F-9 |
| U7 | `ol, ul, menu { counter-reset: list-item; }` | absent; the bake stands in for `ol`, `ul` | LC-3; `dir` with it, by §16.3.3, in the sheet the paragraph above selects |
| U8 | `ol { list-style-type: decimal; }` | present (`ua.rs:61`) | none |
| U9 | `dir, menu, ul { list-style-type: disc; }` | absent: a `ul` inside an `ol` inherits `decimal` (R1) | LC-5 (`dir` as above) |
| U10 | `:is(dir, menu, ol, ul) :is(dir, menu, ul) { list-style-type: circle; }` | absent (P18, R1) | LC-5 (`dir` as above) |
| U11 | three-deep → `square` | absent | LC-5 (`dir` as above) |
| U12 | `type` hints: `ol[type=…]`, `li[type=…]` (five marker styles); `ul[type=…]`, `li[type=…]` (four) | absent (P17) | LC-5, placed by OQ-1's outcome: under B `ol[type]` is core and `li[type]`, `ul[type]` (§16.2) go to the compat generator; under A and B0 all are compat; under C all are core |
| U13 | "In quirks mode, the following rules are also expected to apply" (three `list-style-position` rules) | absent | excluded: elidex never enters quirks mode (IBP §0.4's ground). No slot |
| U14 | "non-CSS user agents are expected to use the ordinal value" | n/a | excluded: elidex is a CSS user agent |
| U15 | `li[value]` → `counter-set` | the bake | LC-3 |
| U16 | `ol[start]` / `[reversed]` → `counter-reset`, steps 1–4 | the bake | LC-3 |

---

## §5. Slices

### §5.1 The set

Each slice is a **node** with its own plan-memo and `/elidex-plan-review` (CLAUDE.md *Edge-dense work*): its plan
decides whether it is a narrowly-scoped single PR or divides into nested, individually reviewed PRs, and a nested
division must satisfy §5.2's constraints at nested-PR granularity. LC-0 is a mechanical prerequisite.

| Slice | What it lands | Decisions | Fixes (cells of §6) | Gated on |
|---|---|---|---|---|
| **LC-0** | Touch-time split: `counter.rs` is 800 lines, 386 of them the `tests` module (`:415`–). Move the tests to `counter/tests.rs`. Standalone PR, no behaviour | — | — | nothing |
| **LC-1** | The per-element step: §4's order (reset → increment → set); the implicit increment moved into the step and keyed on `display`; explicit increments no longer negated; the absent-integer state through parser, resolve and `CounterResetEntry`; §4.4.2's initial value in the step; one boxless gate for both drivers. The bake loses its `li` increment arm with that arm's guard and `count_li_children` (its `ol[reversed]` arm passes the absent integer); the tag gate `is_list_tag` goes | D-2, D-3 (order), D-4, D-9 | P1 (field), P4a (field), P4b, Q4c, Q4e, Q4f, Q4g, Q6, Q6b, P8a, Q13, Q20's field, W-O1, W-RV | LC-0 |
| **LC-2** | Author `counter-*` resolve for every element (N-1) | D-5 | P6, P4e, P19, P20, Q23; P4c, W-F2 stay right | LC-1 |
| **LC-3** | The UA rule; the list hints at the hint seam with HTML integer parsing; the bake, and all three call sites, deleted | D-1, D-6, D-7, D-8 | P2b / Q2b, P3a / Q3a, P5, P14, W-N1 / W-N2, PM 637 | LC-2; **OQ-1 (P-1)**, OQ-2 |
| **LC-4** | The scope model: §4.3 / §4.4.1 inheritance and §4.4.2's instantiate (replace / nest). Both drivers, `flatten` and `set_counter` audited against it | D-3 (scope), D-9 | P8b, P11a / Q11a, P11b, W-S1, W-S2 | LC-3 |
| **LC-5** | HTML §15.3.7's remaining units (§4.3: U3, U4, U6, U9 – U12) | D-11 | P17, P18, R1, R3 (margins / padding), R4 | LC-3 for the hint source; **OQ-1** for U12's placement |
| **LC-6** | `counter-*` resolved values through the property-handler seam | D-10 | P10 / Q10 | LC-3 |

Order: LC-0 → LC-1 → LC-2 → LC-3 → { LC-4, LC-5, LC-6 } (the last three are independent of one another).
LC-1 and LC-2 depend on no open question and can start as soon as this memo is plan-reviewed. Against rev 1: rev
1's LC-1 (order + scope) is split between LC-1 (order) and LC-4 (scope); rev 1's LC-2 and the reversed half of its
LC-3 are merged into LC-1; rev 1's LC-3 keeps N-1 only and is LC-2; rev 1's LC-4 is LC-3.

### §5.2 Why this order: no spec-correct observable turns wrong on the way

**Method.** Every guard, count and arm of the bake, every defect of the step and of the resolve path, and both
drivers' gates are listed as units; every slice's newly opened author-visible input is listed; each unit is checked
in each window against each input that can reach it. A hazard is a cell that is right today (measured, §0.2) and
wrong in some window. Each hazard found is a **constraint** on the order ("X lands no later than Y").

**Constraints, each with the cell that proves it.**

| # | Constraint | The cell that turns wrong otherwise | Found |
|---|---|---|---|
| c1 | the end of the negation ≼ N-1's removal | P4c: 3, 2, 1 → 5, 6, 7 (the author's explicit −1 becomes visible and is negated) | rev 1 |
| c2 | N-1's removal ≼ sibling scope | W-H4, W-H4b: b = 1 → 2 / 3 — under sibling scope the `ol`'s counter reaches the later `div`, whose author `counter-reset` / `counter-set` is still dropped | rev 2 |
| c3 | `menu`'s reset ≼ sibling scope | W-H3, W-H3b: b = 1 → 2 — the previous sibling's counter reaches `menu`'s items while `menu` has no reset | rev 2 |
| c4 | the bake's `li` guard on the computed increment goes ≼ N-1's removal | W-F2: 1, 5, 6 → 1, 3, 4 — the guard (`counter.rs:344`–`:348`) wraps the `value` → set push as well as the increment, so a visible author `counter-increment: list-item 2` drops the hint (R1 finding F2) | R1 |
| c5 | the reversed count's producer changes ≼ the increment's key changes from tag to `display` | W-F3: y = 1 → 2 — `count_li_children` counts `li` tags whatever their `display` (R1 finding F3). It cannot be re-keyed in place: the bake runs for the `ol` in the cascade walk before its children are styled (`crates/css/elidex-style/src/walk.rs:194`), so the count moves to §4.4.2 in the step | R1 |
| c6 | N-1's removal ≼ the UA rule and the hints | a `counter-reset` on an `ol` that declares no `content` would be dropped by `resolve_content` (`box_model/mod.rs:399`): every list would lose its reset | rev 1 (implicit) |
| c7 | the absent integer and §4.4.2 ≼ the `reversed` hint | step 3.2's `reversed(list-item)` has no integer to give | rev 1 |
| c8 | the order fix ≼ the implicit increment's move into the step | P2a: 1, 5, 6 → 1, 6, 7 — under set → increment the step would add the implicit 1 after `li[value]`'s set (today the bake withholds the increment instead) | rev 2 |
| c9 | the sign change and the implicit increment's move are one change | every reversed list would count up, or every explicit increment would stay negated | rev 1 |
| c10 | `dir`'s `counter-reset` is settled ≼ sibling scope | W-H5: b = 1 → 2 where `dir` is to be rendered as `ul` (§4.3) | rev 2 |

c8, c5, c7 and c9 put the order, the implicit increment, the sign, the absent integer and §4.4.2 in one
slice (LC-1; inside it, the order fix first if its plan divides it). c1 and c4 put LC-1 before N-1's removal
(LC-2). c6 puts LC-2 before the cascade inputs (LC-3). c2, c3 and c10 put the scope model after both — LC-4 — where
`menu`'s and `dir`'s resets are the cascade's and no author reset is hidden. No constraint orders LC-4, LC-5 and
LC-6 among themselves.

**The exhaustive table under that order.** Windows: W1 = after LC-1, W2 = after LC-2, W3 = after LC-3, W4 = after
LC-4. Inputs each slice opens — LC-1: (i) set and increment on one element in spec order, (ii) list items by
`display`, (iii) explicit increments on a reversed counter as written, (iv) `reversed(name)` without an integer —
(i), (iii) and (iv) only where `counter-*` reach the computed style (pseudo-elements, elements declaring `content`,
and the bake's own output); LC-2: (v) author `counter-*` on every element; LC-3: (vi) per-property author override of the
rule and the hints, (vii) `menu`, (viii) HTML integer forms, (ix) the hint population by mode; LC-4: (x) sibling
scope and replace / nest. "✓" = no cell turns wrong, with the witness; "gone" = the unit no longer exists; "—" = no
new input reaches the unit in that window.

| Unit at `bf1cae35` | W1 (i – iv) | W2 (v) | W3 (vi – ix) | W4 (x) |
|---|---|---|---|---|
| Bake tag gate `ol \| ul \| li` (`counter.rs:393`) | stays, for the arms that remain; (ii) does not reach it — the step's own gate is `display` | — | gone | gone |
| `ol` arm guard `already_has` (`:306`) | — | ✓ author names `list-item` → the author's value (Q3c: 10, 11 before and after); names another counter → the reset is kept, wrong before and after (P3a, W-N2) | gone; Q3a, W-N1 / W-N2 turn right | gone |
| `ol` arm `start` parse (`str::parse`, `:317`, `:327`) and ±1 | — | — | gone; P14 turns right | gone |
| `ol[reversed]` value `li_count + 1` (`:313`–`:321`) and `count_li_children` (`:276`) | gone in LC-1 (c5): the arm passes the absent integer; ✓ W-F3 stays 1, P4a stays 3, 2, 1, P4b and W-RV turn right | — | the arm is gone; the hint passes the same absent integer | gone |
| `ul` arm and its guard (`:335`–`:337`); `menu` absent | — | ✓ as the `ol` guard | gone; P5 turns right; ✓ W-H3 stays 1 (subtree scope) | ✓ W-H3, W-H3b stay 1: `menu` resets by the rule (c3) |
| `li` arm guard on the computed increment (`:344`–`:348`) | gone in LC-1 (c4); ✓ P1's markers stay; Q13 turns right | (the guard is gone) ✓ W-F2 stays 1, 5, 6 | gone | gone |
| `li` arm `value` → set, guard `already_set`, parse (`:349`–`:352`); no increment when `value` parses | ✓ (i): the step now adds the implicit increment before the set — P2a stays 1, 5, 6 (c8). ✓ (ii): a `li` with `display: block` and `value` still sets (the hint applies to the element whatever its `display`) | ✓ author `counter-set` names `list-item` → the author's value, which is the spec's; names another counter → the hint is kept, wrong before and after (P2b) | gone; P2b / Q2b turn right | gone |
| `li` arm increment push `("list-item", 1)` (`:363`) | gone in LC-1; ✓ P1 markers stay, P8a and P1's field turn right | gone | gone | gone |
| Step order reset → set → increment (`:84`–`:128`) | fixed in LC-1; ✓ no element holds both a set and an increment of one counter today except through `content`-declaring author rules, which turn right (Q6, Q6b, W-O1) | ✓ P6 turns right | — | — |
| Negation of every increment on a reversed counter (`:117`) | fixed in LC-1 with the implicit increment's move; ✓ P4a stays; Q4c, Q4f, Q4g turn right | ✓ P4c stays 3, 2, 1 (c1): the explicit −1 applies as written and §4.4.2 gives 4 | — | — |
| Instantiate: reset and instantiate-if-absent push at the current depth, no replace (`:86`–`:125`) | ✓ (ii): a non-`li` list item instantiates under subtree scope — P8b "0", "0" → "1", "1", wrong before and after | ✓ (v): author counters under subtree scope — P11a 0, 0 → 1, 1, wrong before and after; W-H4, W-H4b stay 1 (the `div`'s own reset / set is now visible and the `ol`'s counter does not reach it) | ✓ the rule's resets nest and pop per subtree; sibling lists each start at 1 | replaced in LC-4; ✓ W-H4, W-H4b stay 1 (c2); P8b, P11a / Q11a, P11b, W-S1, W-S2 turn right; P12 must stay |
| Scope `push_scope` / `pop_scope` (`:42`–`:58`), `flatten` (`:165`), `set_counter` (`:64`) | — (the §4.4.2 walk takes its scope from the same model) | — | — | replaced in LC-4; the paged driver's `flatten` and `page` / `pages` re-audited (I4 × I9) |
| `is_continuation` skips increments only (`:113`) | ✓ the implicit increment is skipped with them (I5 × I9); by reading, a continuation re-runs reset and set — LC-1 states which steps a continuation runs (I3 × I9) | — | — | re-audited with `flatten` |
| Caller gates: pre-layout `is_list_tag \|\| has_counter_props` (`generated_content.rs:141`), returning for `none` and `contents` (`:105`, `:114`); paged walk: every styled element except `display: none` (`builder/walk.rs:131`) | one gate in LC-1: "has a counter property, or is a list item", minus the boxless set. By reading, the paged walk steps a `display: contents` element today (I7 × I9); not a right-today cell of the pre-layout pass | ✓ (v): many more elements pass the gate (§3.1's bound) | — | — |
| N-1: `resolve_content` returns before the counter resolves (`box_model/mod.rs:399`) | hides (iii) and (iv) on elements that declare no `content`; ✓ P4c stays (implicit −1) | gone in LC-2 | — (c6: the rule and the hints rely on it) | — (c2: the scope model relies on it) |
| `reversed(name)` without an integer resolves to 0 (`box_model/mod.rs:512`; parser default, `crates/css/elidex-css/src/declaration.rs:526`) | gone in LC-1 (the absent-integer state); ✓ Q4e, Q4g turn right | ✓ P4e turns right | (c7: the hint relies on it) | — |
| The three call sites (`crates/css/elidex-style/src/walk.rs:194`, `generated_content.rs:144`, `builder/walk.rs:134`) | stay; each still runs the shrunken bake | stay | gone | gone |
| Hint population by mode (no unit today: the bake applies the attributes in every mode and at every cascade entry) | — | — | (ix): under B0 / B / C ✓ every mode and entry keeps `start` / `value` / `reversed`; **under A, P2a and P4a turn wrong in `BrowserCore` / `App` and at every hint-less entry (§5.3)** — the one right-to-wrong change the program can ship, and only by decision (OQ-5) | — |

LC-5 and LC-6 open no window against the units above: LC-5 adds rules and hints for cells that are wrong today
(P17, P18, R1, R3, R4; `menu`'s box rules sit on an inline box until `IBP-ua-display` gives it `display: block`,
wrong before and after), and LC-6 replaces the string `"initial"`, which is no property's computed value here, by
the fields the cascade wrote.

Each slice's plan re-checks its window with its own cells at its base (the cells of §6 that are right today and
that the slice touches must stay right), and re-derives this table for any nested division.

### §5.3 Obligations per slice (requirements; mechanism is each plan's)

- **LC-1**: the implicit increment is derived from the style the step is given, so both drivers get it; the
  step's caller gate becomes "has a counter property or is a list item", one gate for both drivers with one boxless
  set (I7 × I9); state which of steps 2 – 4 a continuation fragment runs (I3 × I9); decide the parsed encoding of an
  omitted integer (today `parse_counter_list` stores the default as a number, so `reversed(foo)` and
  `reversed(foo) 0` are indistinguishable) under §3.2's F7 reading; state the §4.4.2 walk's bound (§3.1), that it
  takes its scope from the same definition the step uses (I4 × I6), and that both drivers compute the same value
  from the whole scope (I6 × I9); honour "only the last one is honored" for repeated names (§4.1); rewrite the tests
  that hand-build `counter_increment: vec![("list-item", 1)]` to stand in for the bake (`git grep -n -c
  'counter_increment: vec' -- crates`); audit §4.5's two branches this memo marks ✗ (a `content: none` pseudo;
  descendants of replaced elements); keep I11. If its plan divides it, the order fix is the first nested PR (c8)
  and the others satisfy c5 and c9.
- **LC-2**: move the three counter resolves out of `resolve_content`'s early return; cover `style=""` (Q23); pin
  P4c, W-F2, W-H4 and W-H4b as cells that stay right.
- **LC-3**: implement the OQ-1 / OQ-2 outcome; reuse one integer parser (SQ-1); delete the bake and its three call
  sites; land `dir`'s `counter-reset` per §4.3; edit the rustdoc and comments §1.7 lists; under option A add the
  core-mode and hint-less-entry cells of §1.6. **Every cascade entry point gets the hint population the outcome
  names** — the entries, grep-verified (`git grep -n -E 'resolve_styles(_with_compat)?\(' -- crates`; `git grep -n
  -E 'hint_generator|no_hints|get_presentational_hints' -- crates`):

  | Cascade entry | Hint argument today | `start` / `value` / `reversed` today | After LC-3 under A | under B0 / B | under C |
  |---|---|---|---|---|---|
  | `elidex_style::resolve_styles` (`crates/css/elidex-style/src/lib.rs:306`), which passes `no_hints` (`:296`, `:318`); callers: `crates/tools/elidex-wpt/src/harness.rs:41`, `crates/css/elidex-style/benches/style_resolve.rs` (3), the `elidex-style` tests | none | honoured (the bake) | **lost**: no generator reaches this entry, so the WPT `css/css-lists` subset (SQ-3) cannot see the attributes | honoured: the core source is inside the cascade | honoured only if every hint source moves inside |
  | shell `resolve_with_mode`, compat arm (`crates/shell/elidex-shell/src/lib.rs:128`) | `get_presentational_hints` | honoured | honoured (the compat generator) | honoured | honoured |
  | shell `resolve_with_mode`, core arm (`:146`; `no_hints`, `:162`) | none | honoured | **lost** | honoured | honoured |
  | inside the walk: `hint_generator` is called at `crates/css/elidex-style/src/walk.rs:174` and passed at all five `resolve_and_attach_style` sites — `:270`, `:281` (light tree), `:409`, `:438`, `:461` (shadow host, shadow-tree child, slotted) | — | — | all five keep it: hints already reach shadow trees, unlike the compat *sheet* | the core source is applied at the same five | the same |

  Under B0 / B the core hint source is applied **by the cascade itself**, not passed by its callers, so no entry
  point can omit it (OQ-2).
- **LC-4**: state the counters-set model it implements, the tree it walks (the flattened element tree, I4 × I7),
  and how `flatten` and the paged continuation path keep I4 and I9; the §4.4.2 walk follows the new scope through
  the shared definition (I4 × I6); P12 stays; pin W-H3, W-H3b, W-H4, W-H4b as cells that stay right, and W-H5 with
  the value LC-3's `dir` decision gives it in each mode.
- **LC-5**: one PR for the rule units and the `type` hints, or two if its plan finds the hint half gated longer
  than the rule half; it states which. `dir` mirrors `ul` per §4.3, without a second copy of `ul`'s rule text if
  the sheets allow it (its plan states how the two stay in step).
- **LC-6**: the handler's home crate (SQ-2) and the serialization (`none`; name–integer pairs; `reversed(name)`
  with and without an integer, §3.2's F7 reading), with `#11-cssvalue-list-separator-fidelity` and
  `#11-computed-style-enumeration` as neighbours it must not duplicate. **One parse, one resolve (D-10).** The trait
  brings `parse` and `resolve` with `get_computed`. For `counter-*` the single parse path is the declaration parser
  (`crates/css/elidex-css/src/declaration.rs:526`–`:527`): it is the only one a registry-less parse reaches, and the
  UA sheet that will carry `counter-reset` is parsed that way (`crates/css/elidex-style/src/ua.rs:199`), as is the
  WPT harness's author CSS (`crates/tools/elidex-wpt/src/harness.rs:40`). The single resolve path is the cascade's
  resolve step. The handler's `parse` and `resolve` entries call those implementations (the handler crates already
  depend on `elidex-css`, e.g. `crates/css/elidex-css-box/Cargo.toml:15`; the resolve functions move to where the
  handler can call them) and hold no copy; its plan states where they live and shows that I1 holds — the cascade's
  resolve step stays the only caller that writes a live component. This is **not** how the other handlers are built:
  for most properties two parse and two resolve implementations exist side by side — e.g. `list-style-type` is
  parsed at `declaration.rs:374` and in `crates/css/elidex-css-text/src/lib.rs:94`, resolved in
  `crates/css/elidex-style/src/resolve/font.rs:81` and at `elidex-css-text/src/lib.rs:174`, and served to
  `getComputedStyle` from `:287`; the handler's `parse` runs only for names the built-in match does not know
  (`declaration.rs:530`), and only the transform family resolves through its handler
  (`crates/css/elidex-style/src/resolve/mod.rs:305`). That project-wide split is a finding for the PM (§7 F-10), not
  this program's to repair; LC-6 does not add another copy to it.
- **Every slice**: carry §3.3's docstring-citation table for its own functions.
- **Every slice**: remove the comment that points at this slot for the defect it closes (`git grep -n
  'list-counter-baking' -- crates` → `counter.rs:81` and `generated_content.rs:131` are the step order's, LC-1;
  `generated_content.rs:137` is the bake's, LC-3).
- **Every slice**: name the tests that guard its surface (engine-level cells from §6, and the counter subset of the
  WPT `css/css-lists` directory if the runner can take it — SQ-3), per *Supported-surface testing*; new shell-level
  cells go in a new test file, since `crates/shell/elidex-shell/src/tests.rs` is 1081 lines.

---

## §6. Edge matrix (seed cells; each slice's plan owns its final list)

"First green" is the slice after which the cell holds the spec value in **both** engine modes, except rows marked
(P-1), whose core-mode value follows OQ-1. "Stays" cells are right today and are pinned by the slices named, as the
windows of §5.2 require.

| Cell | Spec value | Today | First green |
|---|---|---|---|
| P1 markers / `li` field | 1, 2, 3 / `none` | 1, 2, 3 / `list-item 1` | stays / LC-1 |
| P2a | 1, 5, 6 | same | stays: LC-1 (c8); (P-1) in core after LC-3 |
| P2b, Q2b | 1, 2, 3 | 1, 5, 6 | LC-3 |
| P3a, Q3a | 1, 2 | 5, 6 | LC-3 |
| Q3c | 10, 11 | same | stays |
| P4a markers / `ol` field | 3, 2, 1 / `reversed(list-item)` | 3, 2, 1 / `reversed(list-item) 4` | stays: LC-1; (P-1) after LC-3 / LC-1 |
| P4b | 2, –, 1 | 3, –, 2 | LC-1; (P-1) after LC-3 |
| P4c | 3, 2, 1 | 3, 2, 1 (accident) | stays through every slice: LC-1, LC-2 (c1) |
| Q4c | 3, 2, 1 | 5, 6, 7 | LC-1 |
| Q4f | −3, −2, −1 | 3, 2, 1 | LC-1 (the bake no longer writes a count); its no-`content` variant LC-2 |
| P4e, Q4e | 3, 2, 1 | 0, 0, 0 / 1, 2, 3 | LC-2 / LC-1 |
| Q4g | −3, −2, −1 | −1, −2, −3 | LC-1 |
| P5 | 1, 1, 2, 2 | 1, 2, 3, 4 | LC-3 |
| P6, Q6, Q6b | 5 | 0 / 6 / 6 | LC-2 / LC-1 / LC-1 |
| P8a | 1, –, 2 | 1, –, 3 | LC-1 |
| P8b | 1, 2 | 0, 0 | LC-4 (1, 1 from LC-1) |
| P11a, Q11a | 1, 2 | 0, 0 / 1, 1 | LC-4 |
| P11b | 1, 2 | 1, 1 | LC-4 |
| P12 (`counters()` across sibling `ol`) | 1 / 1, 2 | same | stays (LC-4 must keep it: the replace rule) |
| Q13 | 1, 5, 6 | 1, 3, 4 | LC-1 |
| P14a, P14b, P14d | 5; 5; 7 | 1; 1; 1 | LC-3 |
| P17 | "a" | "1" | LC-5 (P-1) |
| P18, R1 | circle | disc / decimal | LC-5 |
| R3 margins, padding; R4 | 1em, 40px; 0 | 0, 0; 16px | LC-5 |
| P19, P20 | 2, 4; 1, 2 | 1, 2; 1, 2 (the author value absent) | LC-2 / stays |
| Q19 – Q21 | 2, 4; 1, 2; 1, 2 | same (Q20's field carries the baked increment) | stay; Q20's field LC-1 |
| Q23 | 3 | 0 | LC-2 |
| P10, Q10 | the computed values | `"initial"` | LC-6 |
| W-F2 | 1, 5, 6 | same | stays: LC-2 (c4) |
| W-F3 | y = 1 | same | stays: LC-1 (c5) |
| W-H3, W-H3b | b = 1 | same | stay: LC-4 (c3) |
| W-H4, W-H4b | b = 1 | same | stay: LC-2, LC-4 (c2) |
| W-H5 | 1 where `dir` is rendered as `ul`; 2 where it carries no rule | 1 | LC-4, with the value LC-3's `dir` decision gives per mode (c10) |
| W-RV | 11, 10, 9 | 3, 10, 9 | LC-1; (P-1) after LC-3 |
| W-S1 | 1 | 0 | LC-4 |
| W-S2 | 2 | 1 | LC-4 |
| W-O1 | 7, 7 | 8, 8 | LC-1 |
| W-N1, W-N2 | 2 | 1 | LC-3 |
| W-PG1 (paged): a `display: contents` element with `counter-increment: c`, read in a margin box | the element does not increment | by reading: the paged walk increments (`builder/walk.rs:131`); not run | LC-1 |
| W-PG2 (paged): an element with `counter-reset: c` continued on a second page, `c` read in the second page's margin box | LC-1's plan derives it (I3 × I9) | by reading: the continuation resets again; not run | LC-1 |

Derivations of the less obvious spec values: P4b — two elements increment (the boxless one cannot), so num = 1 + 1,
plus the last non-zero negated increment 1 → 3, then 2, 1. Q4f — three explicit `+1`: num = −3, plus −1 → −4, then
−3, −2, −1. Q13 — the author's `counter-increment` and the hint's `counter-set` are different properties; increment
then set gives 5. P5 — `menu` is a child of the first `li`; its UA reset nests a counter, and `d` continues the
`ol`'s. W-RV — the second item increments (its negated increment 1 becomes the last non-zero one) and sets: num =
1 + 10, the loop breaks, + 1 → 12; then 11, 10 (set), 9. W-S2 — the inner `div` inherits the `ul`'s counter from its
preceding sibling (§4.4.1 step 3) with the value 1 from the element before it in tree order (step 4), and b
increments it. P19 — `li { counter-increment: list-item 2 }`. P20 — `li { counter-increment: other }`: the implicit
increment still applies.

---

## §7. Out of scope, findings for the PM, and scout questions

**Findings (each needs an owner or a slot; this memo registers nothing):**

- **F-1 (N-6)** — conforming-attribute dimension hints are compat-gated. Already `IBP-layout`'s by IBP §2; P-1b's
  outcome is its input. No new record.
- **F-2 (N-7)** — `dir` → `direction` is a UA rule in HTML §15.3.5, generated as a hint in `elidex-dom-compat`, so
  `BrowserCore` / `App` ignore the `dir` attribute (R2). No owner found. Proposed: a slot, owner Layout or L1, to
  move the rule to the core UA sheet (its plan checks what `:dir()` support the selector engine has).
- **F-3** — hints enter as `Origin::Author`, specificity 0, order 0 (`cascade/mod.rs:175`–`:182`); CSS Cascade 5
  §6.5 makes them an independent origin between the user origin and the author origin, part of the author origin
  for `revert` only. **Not observable today**: `revert` is not a CSS-wide keyword elidex parses
  (`parse_global_keyword`, `crates/css/elidex-css/src/values.rs:96`, accepts `initial` / `inherit` / `unset`
  only), there are no cascade layers (`git grep -n -i -F '@layer' -- crates/css` → 0; control `@media` → 37 lines in
  `parser.rs`) and no user origin (`git grep -n -w 'Origin::User' -- crates` → 0; control `Origin::UserAgent` → 10).
  It becomes observable with any of the three: with layers, a hint at author order 0 would beat a layered author
  rule it should lose to. The program adds its hints at the existing seam. **Proposed slot** (no owner found):
  `#11-presentational-hint-cascade-origin` — *Gap*: hints are author declarations, not Cascade 5 §6.5's own origin.
  *Why deferred*: no reader can tell today. *Trigger*: `revert`, `@layer` or a user-origin sheet entering the
  cascade. Owner proposed: the lane that lands the first of them.
- **F-4** — `HtmlElementHandler` (`crates/core/elidex-plugin/src/traits.rs:214`) offers `default_style` and
  `spec_level` per tag and has no consumer outside `elidex-plugin`
  (`git grep -n -E 'create_html_element_registry|HtmlElementHandler' -- crates`, hits only under
  `crates/core/elidex-plugin`). It is the Plugin-first home OQ-2 weighs, and "dead code は接続するか削除" applies
  to it either way. (IBP assigns its connection to `IBP-classify`, F4 / F7's home.)
- **F-5** — `menu { display: block }` is missing (R3) → input to `IBP-ua-display`.
- **F-6** — `li { text-align: match-parent }` needs a `text-align` value elidex does not parse. Proposed: a slot
  (owner L1 or Layout).
- **F-7** — HTML integer / dimension microsyntax parsers exist in at least four crates (`git grep -n -i -c 'rules for
  parsing' -- crates` lists 12 files; the parser definitions among them: `elidex-api-canvas/src/component.rs:256`,
  `elidex-dom-api/src/element/numeric_reflect.rs:65`, `elidex-dom-compat/src/presentational.rs:238`,
  `elidex-form-core/src/reflect.rs:34`). LC-3 must not add another (D-8); converging them is not this program's.
- **F-8** — no reader of HTML §4.4.8's *ordinal value* exists (`git grep -n -i -E 'ordinal|posinset' --
  crates/dom/elidex-a11y` → 0). When accessibility needs it, the counters set becomes a component (§0.5).
- **F-9** — the UA sheet writes physical box longhands where HTML §15.3.7 writes flow-relative ones (U3
  `margin-block`, U5 `margin-inline-start`, U6 `padding-inline-start`; `ua.rs:53`–`:57`, `:63`, `:64`), and no
  flow-relative box property is parsed at all (`/usr/bin/grep -rn -E '(margin|padding)-(inline|block)|inset-(inline|block)'
  crates --include='*.rs'` → 0; control `(margin|padding)-(top|left)` → 211). No slot found (the open-slot registry
  and the M4-12 roadmap searched for `logical`, `inline-start`, `block-start`; the only hits are `logical_px`). By
  reading, not rendered here: in a right-to-left list the 40px sits on the left where the spec's inline-start is the
  right. **Proposed slot**: `#11-ua-sheet-flow-relative-box-properties` — *Gap*: as above, for every §15.3 rule
  that uses a flow-relative property, not the list rules alone. *Why not this program's*: the repair is
  css-logical-1 property support in the property system, which the list rules only consume; LC-5 adds `menu`'s and
  `dir`'s rules in the sheet's existing physical form and names them in the slot. *Trigger*: flow-relative box
  properties entering the property registry, or UA-sheet fidelity work for `direction` / vertical writing modes.
  Owner proposed: L1 or Layout.
- **F-10** — the property-handler split (§5.3 LC-6): for most properties the built-in declaration parser and the
  cascade's resolve functions are one implementation and the registered handler's `parse` / `resolve` a second;
  `docs/design/ja/11-parser-design.md:19` says property parsing is delegated to the handlers. Extent not measured
  here (seen for `list-style-type` and `content`: `crates/css/elidex-css-box/src/lib.rs:162`, `:303` beside
  `declaration.rs:523`). Constraint any repair meets: registry-less parses (the UA sheet, the WPT harness) reach
  only the built-in parser, and the handler crates depend on `elidex-css`. No slot found in the open-slot registry
  under `CssPropertyHandler`, `property-handler` or 二重実装. Proposed: a slot, owner L1.
- **F-11** — a citation drift outside this memo: `crates/script/elidex-js/src/vm/host/html_olist_proto.rs:8` cites
  "HTML §2.4.4.1 ("rules for parsing integers")"; `webref heading html 2.4.4.1` finds no such section, and the
  rules are §2.3.4.1 Signed integers (`webref heading html 2.3.4.1`). For the cite-sweep lane; not edited here.
- **F-12** — `dir` the element: IBP's Appendix G seeds `dir { display: block }` as a compat-sheet rule; HTML
  §16.3.3's "must" (§4.3) is input `IBP-ua-display`'s plan does not have. Proposed: an input line for
  `IBP-ua-display`.
- **F-13** — two candidate spec issues, noted and **not filed by us**: (a) CSS Lists 3 §4.1's *Computed value* line
  says a `reversed()` item is "paired with an integer" while the value definition gives it "no starting value if
  not given" (§3.2, F7 note); (b) HTML §15.3.7's `ol, ul, menu { counter-reset: list-item; }` omits `dir`, which
  §16.3.3 requires to be rendered as `ul` (§4.3).

**Scout questions (facts the slices' plans need; not blocking this memo):**

- **SQ-1**: `elidex-dom-api` depends on `elidex-style`, so `elidex-style` cannot call `parse_integer`
  (`crates/dom/elidex-dom-api/src/element/numeric_reflect.rs:88`). Which crate below both can hold the one HTML integer
  parser, and who else calls the four copies of F-7?
- **SQ-2**: `list-style-type` is served to `getComputedStyle` by `elidex-css-text`'s handler (`lib.rs:287`) and
  `content` by `elidex-css-box`'s (`lib.rs:491`). Which handler crate is the home for `counter-*`, and where do the
  shared resolve functions live so that both the cascade and the handler call them (D-10)?
- **SQ-3**: can `elidex-wpt` run a directory subset such as `css/css-lists`, and which counter tests does it load?
  (Its harness resolves through the hint-less entry, §5.3.)
- **SQ-4**: every test that builds `ComputedStyle` counter fields by hand to stand in for the bake
  (`git grep -n -c -E 'CounterResetEntry::|counter_increment: vec|counter_reset: vec|counter_set: vec' -- crates`:
  `elidex-render/src/builder/tests/counter.rs` 20, `tests/display_types.rs` 4, `counter.rs` 20,
  `generated_content.rs` 5) — which assert the bake's shape rather than numbering?

---

## §8. Open questions for the PM

- **OQ-1 — P-1 (hint placement), with the designation reading it rests on.** Two linked questions for E0's
  ratifier; the proposal also amends the anchor design (A0) §3.2b's "UA sheet + presentational hints on/off" and
  touches IBP's ratified premise (§0.2), so the routing names all three records.
  (a) *The reading* (§1.2): is `BrowserCompat` the mode designated under HTML §2.1.8, with `BrowserCore` / `App` not
  designated (DR-2)? Or every mode (DR-1), or none (DR-3)? Nothing recorded answers it; DR-2 is the only reading
  consistent with E0, #408 and IBP §0.2 at once.
  (b) *The option* (§1.6): A / B0 / B / C. **Recommendation: B on DR-2** — P-1a at rung 1 (CSS Lists 3 §4.6's
  "should"), P-1b at rung 2 (three-layer consistency; One issue, one way with #408). Under DR-1 the answer is C
  and E0 is withdrawn rather than amended. B and B0 amend the E0, rustdoc and A0 passages of §1.7; A amends none
  and is rejected at rung 1 for the three list hints.
  Blocks LC-3, and through it LC-4, LC-5 and LC-6; LC-5's U12 placement directly. LC-1 and LC-2 are not blocked.
- **OQ-2 — the core hint source's home** (only if B0, B or C). (X) a module in `elidex-style` beside `ua.rs`, with
  `resolve_styles_with_compat`'s `hint_generator` becoming the compat extra, as `extra_ua_sheets` is for rules; or
  (Y) a hints method on `HtmlElementHandler`, classified by its `HtmlSpecLevel`. Fixed under both: the cascade
  applies the core source itself, at every entry point of §5.3 — it is not a caller's argument. The principles
  split: *Plugin-first* selects Y; *One issue, one way* selects X within this program's reach (UA rules and hints
  are one kind of thing and today have one form; Y for the list hints alone is a second form, and Y for everything
  is the whole UA-defaults migration, F-4). **Recommendation: X**, with F-4 recorded so the migration, if wanted,
  moves rules and hints together.
- **OQ-3 — scope growth.** The slot text does not list N-1 … N-5 or §15.3.7's other units. This memo folds them in
  (LC-1's order fix, LC-2's N-1, LC-3's N-4, LC-4's scope model, LC-5; N-3 falls out of LC-1), because they are the
  same step, the same resolve function and the same spec section, the slot's exit criterion (no
  silent-wrong-in-production slot, PM 620) covers them, and §5.2's constraints tie their order to the slot's own
  work (c1, c2, c4, c6). **Recommendation: approve the fold**; the alternative is separate slots for N-1 / N-2 and
  for LC-5, which §5.2 would still order between this slot's slices.
- **OQ-4 — owners and records for the findings.** Proposed slots: F-2 (`dir` attribute rule), F-3 (hint origin),
  F-6 (`match-parent`), F-9 (flow-relative box properties in the UA sheet), F-10 (the handler split). Input lines:
  F-5 and F-12 → `IBP-ua-display`; F-11 → the cite-sweep lane. Whether F-4 and F-7 get records.
- **OQ-5 — under option A only**: LC-3 would make `BrowserCore` / `App`, and every hint-less cascade entry (the
  WPT harness, the benches, `elidex-style`'s own tests), stop honouring `start` / `value` / `reversed`, which they
  honour today — the one right-to-wrong change in §5.2's table. Is that an accepted, cell-pinned divergence (no
  production session selects those modes, E0 §2 Axis 3), or does it need its own record?

---

## §9. Ledger records (applied by the PM / pmo; nothing here is applied by this PR)

- Slot `#11-list-counter-baking-ua-rule-and-hints`: plan = this memo; first task (PM 620) done, results §0.2; the
  slot's "Owed by PM/pmo" back-pointer is already present (PE-2); PE-1 and PE-3 recorded against the slot text.
- Master plan §10 (c) (`memory/render-layout-inline-convergence-plan.md`): closes with LC-1.
- `IBP-layout` input lines: P-1's outcome and the designation reading (F-1); `IBP-ua-display` input lines: `menu`
  (F-5), `dir` with HTML §16.3.3 (F-12).
- Cite-sweep lane input: F-11.
- Proposed, pending OQ-4: slots for F-2, F-3, F-6, F-9, F-10.

## §10. Definition of done (the program)

- `git grep -n -E 'apply_implicit_list_counters|count_li_children' -- crates` → 0 hits.
- Every cell of §6 holds its spec value in the mode(s) OQ-1's outcome names; each slice's plan carries its cells as
  tests, the "stays" cells of its window included.
- `git grep -n 'list-counter-baking' -- crates` → 0 hits (the pointer comments are gone with their defects).
- `getComputedStyle` returns the computed `counter-*` values (P10 / Q10), and `counter-*` have one parse and one
  resolve implementation (D-10).
- The slot row is closed by the PM with the last slice's merge; §9's records are applied.

## Changelog

- **Rev 2 (2026-11-23)** — folds `/elidex-plan-review` round 1 on rev 1 (`65a38919`; CRIT 0 / IMP 9 / MIN 12; change
  type STRUCTURAL). Round-1 finding IDs are written R1-F*n* / R1-M*n* to keep them apart from this memo's F-*n*.
  - **R1-F1** — §1 re-derived from rung 1 with HTML §2.1.8's designation: three readings (§1.2), what rung 1 decides
    under each (§1.3), rung 2 only for the remainder (§1.4). P-1 is split into P-1a (rung 1, CSS Lists 3 §4.6) and
    P-1b (rung 2); option B0 added; C restated as DR-1's consequence. The recommendation stays B but is now
    conditional on DR-2, and the reading is part of OQ-1. D-6's rungs restated.
  - **R1-F2, R1-F3** — §5 re-sliced from an exhaustive unit × window × input table (§5.2). Constraints c1 – c10; new
    hazards beyond the two reported: c2 (hidden author reset / set under sibling scope), c3 (`menu` under sibling
    scope), c8 (order before the implicit increment's move), c10 (`dir`); two driver disagreements found by reading
    (I7 × I9, I3 × I9). New order: LC-1 = order + implicit increment + sign + absent integer + §4.4.2 (the bake
    loses its `li` guard and the count there); LC-2 = N-1; LC-3 = cascade inputs, bake deleted; LC-4 = scope. Probe 4
    (§0.2) measures the window cells; §6 gains a cell per hazard.
  - **R1-F4** — D-10 and §5.3 LC-6: one parse and one resolve implementation for `counter-*`, the handler's
    trait-forced entries delegating; the existing project-wide split recorded with greps and routed as F-10; I1 × I10
    updated.
  - **R1-F5** — §5.3 LC-3: every cascade entry point with the hint population each option gives it; §1.6 and OQ-5
    carry the cost under A; OQ-2 fixes "applied by the cascade, not passed by callers".
  - **R1-F6** — §4.3 U3 / U5 / U6: physical vs flow-relative properties routed as F-9 with a proposed slot (OQ-4).
  - **R1-F7** — §3.2's F7 note: both texts quoted, the tension stated, the reading and its rung; flagged for LC-1 /
    LC-6 and as candidate spec issue F-13 (a).
  - **R1-F8** — §1.8: the precedent search re-run by concept in English and Japanese with a control; added
    `19-svg-rendering.md` §19.2.2, `29-survey-analysis.md` §29.2, PM 985 / 620 / 611, the memory policy memo.
  - **R1-F9** — §1.7: every ratified or landed passage each option amends, with lines (E0 §0, §1a ×2, §1c, the
    registry note, §2 Axis 3 / 4, §3, §3.1, §5 items 4 / 5; the rustdoc; shell comments; A0 `:516`, `:750`); OQ-1
    names A0.
  - **R1-M1** — F-3: observability checked (`revert`, layers, user origin all absent), proposed slot, OQ-4.
    **R1-M2** — §2: all 55 pairs checked; I3 × I6, I5 × I9 and eleven more added; the non-meeting pairs listed with
    reasons. **R1-M3** — §4.3: `dir` under HTML §16.3.3, one statement for U1 / U3 / U4 / U6 / U7 / U9 – U11; F-12.
    **R1-M4** — §1.1 / P-1b: three attribute classes; §16.1 → compat. **R1-M5** — §3.3 and the "every slice"
    obligation. **R1-M6** — I2 and D-7: the attributes are style inputs; I6's dependency. **R1-M7** — D-6 and D-8
    relabelled. **R1-M8** — D-4 and §0.5 row 4: the §4.4.2 value is walk-local, recomputed per driver run, with the
    promotion criterion. **R1-M9** — §1.8: the design SSoT's split recorded. **R1-M10** — §1.8: #408's `<tt>`
    exception and its PR body's wording. **R1-M11** — §1.8 IBP row and §1.6's last row: the criterion difference and
    §15.5.9. **R1-M12** — PE-3 and §1.8: the cell test's M; status restated. **R1 side note** — verified with
    `webref`; F-11.
  - Not judged wrong: none of the round-1 findings. Two were sharpened: R1-F1's "§4.6 … rejects A at rung 1 for the
    list-item hints only" holds under DR-2 / DR-3, while under DR-1 rung 1 rejects A for every hint; R1-M3's
    "`dir` → compat ignores §16.3.3" is answered by requiring `dir` to mirror `ul` wherever it is rendered, the
    sheet following the same reading question as §1.2.
- **Rev 1 (2026-11-23)** — first draft: probes at `bf1cae35` (§0.2), decisions D-1 … D-12, proposal P-1 with its
  precedent table, six slices after a split prerequisite, five open questions.
