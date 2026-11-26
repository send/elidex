# `#11-list-counter-baking-ua-rule-and-hints` — list numbering through the cascade and CSS Lists 3 counter processing (umbrella plan-memo)

This memo is the **umbrella** for slot `#11-list-counter-baking-ua-rule-and-hints` (owner Layout, PM 985; registered
PM 629; slot text = `docs/plans/2026-11-pr-a2-elidex-style-parallel.md` §7.1, between its BEGIN / END markers). The
slot is edge-dense by its own text (cascade origin × presentational hints × counter processing × render × engine
modes), so by CLAUDE.md *Edge-dense work* it is a multi-PR program: this memo fixes the end state, the slices and
their order, and each slice then gets its own plan-memo and `/elidex-plan-review` before code. Rev 3.2, 2026-11-26
(plan-review rounds 1 – 3 and the Step 4.5 re-review of rev 3.1 folded; changelog at the end). Branch `layout-list-counter-hints-plan`, worktree
`wt-lch-plan`, base `bf1cae35`; `file:line` refs into code are anchored at main `01426656` (§0.2's probes ran at
`bf1cae35`; `git diff --stat 01426656 aeecf9b0 -- crates` is empty, so they hold at rev 2.2's head too). **Docs
only.**

**What this memo decides** (§0.4): the homes of the facts (who writes the computed `counter-*` values, where the
implicit `list-item` increment and the reversed initial value live, the UA rule's sheet), the slice set and order
(§5) with the window table that grounds the order (§5.2), and the program-level invariants (§2). **What it proposes
and does not decide** (§1, §8): the engine modes in which the list hints apply, and the reading of HTML §2.1.8's
*designation* that the answer rests on. Every answer but option A changes a ratified decision (E0), so the
conclusion is written as proposal **P-1** with every reading and both sides' precedents, for the PM to route to E0's
ratifier (PM 620, `PORTFOLIO-archive-2026-W46.md`).
This memo edits neither `docs/plans/2026-06-e0-shell-style-compat-mode.md` nor
`docs/plans/2026-09-inline-box-predicate.md`.

**Approval scope**: a plan-review of this memo ratifies §0.4's decisions, §2's invariants, §5's slice set / order /
gates (the constraints of §5.2 included) and §9's ledger records. §4's mechanism sketches, §6's cell expectations
and the per-slice obligations are **input to each slice's plan-review**, which re-derives them and may change them.

---

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

- The probes ran at `bf1cae35` (the worktree's `65a38919` is `bf1cae35` plus this memo: `git diff --stat bf1cae35
  65a38919 -- crates` is empty); `file:line` references are at main `01426656` (rev 2.1). "At `bf1cae35`" below names
  the behaviour measured there. Each slice's plan re-greps at its own base.
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
- A value marked "by reading" or "derived" was not run: revs 3 and 3.1 add cells without a new probe run (neither
  revision builds anything), and the slice that owns each such cell runs it at its base before relying on it.

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
| P19, P20, Q19 – Q21 | `<ol><li>x<li>x</ol>` + P19 `li { counter-increment: list-item 2 }`; P20 `li { counter-increment: other }`; Q19, Q20 = P19, P20 +c; Q21 `li { counter-increment: none; content: normal }` | P19, P20: 1, 2, each `li` `counter_increment = [("list-item", 1)]` (author value absent, N-1). Q19: 2, 4, `[("list-item", 2)]`. Q20: 1, 2, `[("other", 1), ("list-item", 1)]`. Q21: 1, 2, `[("list-item", 1)]` (`lch-probe1.log`, `lch-probe2.log`) | P19 / Q19: 2, 4; P20 / Q20 / Q21: 1, 2 (the implicit increment applies: the author value does not name `list-item`); the `li` field holds the author's value only (`other 1`; `none`) | P19 **changed** (N-1); Q19 control (the bake's guard sees the author's `list-item`); Q20, Q21 markers right, fields carry the baked increment (I1 × I5) |
| — | PM 637 duplicate bake | 3 call sites of `apply_implicit_list_counters_from_dom` (`git grep -n 'apply_implicit_list_counters_from_dom(' -- crates`, less the definition `counter.rs:377`: `crates/css/elidex-style/src/walk.rs:168`, `crates/css/elidex-style/src/generated_content.rs:144`, `crates/core/elidex-render/src/builder/walk.rs:134`); P1 shows one entry per field, so the second and third calls change nothing | — | structural → **confirmed** by grep; no behaviour to observe |

**Findings the probes added** (not in the slot text; §0.3 routes each):

| ID | Markup / author CSS | elidex at `bf1cae35` | Spec value | Finding |
|---|---|---|---|---|
| N-1 | every row above marked "author value absent"; also Q23: `<div style="counter-reset: foo 3">` + `p::before { content: counter(foo) }` → pseudo 0, `counter_reset = []` | an author `counter-reset` / `-increment` / `-set` is dropped unless the **same element also declares `content`** | the properties apply to all elements (`webref css css-lists-3 counter-reset`: appliesTo "all elements") | `resolve_content` returns at "not declared → default (Normal)" (`crates/css/elidex-style/src/resolve/box_model/mod.rs:416`) before it reaches the three counter resolves (`:465`–`:472`) |
| N-2 | Q11a: `h1 { counter-reset: sub } p { counter-increment: sub } p::before { content: counter(sub) }` (+c), `h1` and two `p` siblings | pseudo 1, 1 | 1, 2 (§4.3: scope includes "its following siblings with their descendants") | a counter lives only until its creator's subtree ends: `resolve_tree` calls `push_scope` before `process_element` (`generated_content.rs:122`, `:145`) and `pop_scope` drops it (`counter.rs:48`) |
| N-2 | P11b: `<div><li><li></div>`, `li { list-style-type: decimal }` | 1, 1 | 1, 2 | same; reachable in production with no author counter property |
| N-3 | Q13: `<li value=5>` + `#b { counter-increment: list-item 2; content: normal }` | 1, 3, 4 | 1, 5, 6 (increment, then the hint's `counter-set`) | the bake's `li` arm skips the `value` attribute whenever the computed `counter-increment` names `list-item` (`counter.rs:343`–`:351`) |
| N-4 | P14: `<ol start=" 5">`, `<ol start="5abc">`, `<li value="7abc">` | 1; 1; 1 (`start="+5"` → 5) | 5; 5; 7 (HTML §2.3.4.1 rules for parsing integers: skip ASCII whitespace, collect the digit run) | attributes are parsed with `str::parse::<i32>` (`counter.rs:316`, `:326`, `:348`) |
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
| W-H5 | `<div><ol><li>a</ol><dir><li>b</dir></div>` | b = 1 (`dir` computes `display: inline`; no rule in either sheet) | 1, in every mode: `dir` is rendered as `ul` (HTML §16.3.3, §4.3), so it resets `list-item` | §5.2 c10 |
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
  `<iframe width=600>` is a cell at all is `IBP-layout`'s to derive. The premise's reach **is** stated, by
  application: IBP's ratified §0.3, §0.4 and §0.6 item 3 apply the "must" to mode-independent core facts
  (replacedness, button layout) and carve the quirks disjunct out to compat (§1.2, reading DR-4). What IBP does not
  state is an engine mode, or that reach for a **hint**. So the conflict is between one ratified decision (E0: no
  hints in core), one ratified premise applied to core facts but never to hints, and one unreviewed repair shape —
  plus this slot.
- **N-1 … N-5** are defects of the same machine and the same spec section the slot names; this memo folds them
  into the program (§5) and asks the PM to approve that growth (OQ-3). **N-6, N-7** are outside the program (§7).

### §0.4 Decisions

Each line names the ground rung that decides it.

- **D-1 — one writer.** At the end state the `counter_reset` / `counter_increment` / `counter_set` fields of
  `ComputedStyle` are written by the cascade's resolve step and by nothing else: the post-cascade bake
  (`apply_implicit_list_counters_from_dom`, `apply_implicit_list_counters`, `count_li_children`) is deleted (LC-3).
  **Until LC-3 there are two writers, as at `bf1cae35`**: in every earlier window the bake still appends to the
  fields the cascade wrote, for `ol` / `ul` / `li` — at the cascade walk's call site
  (`crates/css/elidex-style/src/walk.rs:168`) for every such element, and at the two driver sites
  (`generated_content.rs:144`, `builder/walk.rs:134`) from LC-1 on only for the elements LC-1's one gate admits. In
  production that still includes every `ol` / `ul`: the walk's call has already written their reset, so "has a
  counter property" holds for them. A caller that attaches a hand-built `ComputedStyle` and relies on a driver site
  to bake an `ol` / `ul` reset loses it from W1 on, **permanently**: from LC-3 the reset is the UA rule's, written by
the cascade, which a hand-built style never passes through. Such callers are tests, and LC-1 updates them (§5.3). What
  D-1 fixes for those windows is the direction: the bake only
  **loses** work whose producer has moved (LC-1: the increment arm with its guard, and the `li` count); it never
  gains an input, a guard or a second caller. *Rung 1*: CSS Lists 3 §4.6 ("This does not affect the specified or
  computed values of counter-increment"); HTML §15.3.7 puts the reset and the set in the cascade (a UA rule and two
  hints).
- **D-2 — the implicit increment is counter processing's.** A box whose computed `display` is `list-item` increments
  `list-item` by 1 (by −1 when the innermost `list-item` counter on it is reversed) at the increment step, unless
  its computed `counter-increment` names `list-item`. Tag names play no part at the end state; between LC-1 and
  LC-2k the step's list-item predicate still reads the tag `li`, because the key cannot change while N-1 hides
  author `counter-increment` (§5.2 c11). *Rung 1*: §4.6.
- **D-3 — counter processing is CSS Lists 3 §4's.** Steps 1–5 in the spec's order (inherit, reset, increment, set,
  use); the counters-set model of §4.4.1 / §4.4.2 (a counter's scope is its creator's descendants **and following
  siblings with their descendants**; instantiating replaces a same-name counter that originates from the element
  itself or a previous sibling); §4.5 (a boxless element does nothing). *Rung 1*.
- **D-4 — reversed counters.** An explicit increment applies as written; only the implicit one is −1 (*rung 1*:
  §4.6). A reversed counter instantiated without an integer gets §4.4.2's value, computed by the counter step over
  the boxes that exist. That value is **never part of the computed value**: `CounterResetEntry` gains an
  absent-integer state for the reversed form, and the cascade writes that state and nothing derived from other
  elements. *Rung 1 is in conflict on this point* (§3.2, F7 note); *rung 2* decides it: ECS-native first — the style
  component has one writer, the cascade system, and a value the counter system derives from other entities'
  components is that system's output, not the cascade's (D-1; §0.5 row 2).
  **Where the derived value lives is LC-1's to decide, under one named criterion** — CLAUDE.md's *Side-store →
  component* rule, read at its precondition: it governs per-entity state **held** in an entity-keyed store beyond
  one computation. Its grounds (SameObject as a component get, GC as one query, despawn as automatic cleanup) are
  about state that outlives the computation that made it, and the trap it names as typical is the literal port of
the boa `HostBridge` side-store to the VM. ECS first
  principles say the same: a component is world data that other systems read. The value is a per-entity fact (one
  per reversed reset entry of an element), `Send + Sync`, and no per-VM handle. So: it is a **component** on the
  resetting entity when it **crosses systems** (computed by the pre-layout pass, read by the paged walk) or
  **outlives the run** that computed it. Otherwise it is **run-local scratch** — dropped when the walk ends, under
  any key, an entity-keyed precompute map that bounds the walk included — or it lives only in the counter instance
  the step creates (as every counter value does today: `CounterState` is keyed by counter name and scope depth,
  `counter.rs:24`–`:28`, not by entity). Neither of those is a side-store, so the rule is not engaged. (Rev 3 forbade
  an entity-keyed map "walk-local or not"; that read the rule past its precondition and closed the linear-time
  precompute route, plan-review R3 B2-I1.) **If LC-1 picks the component**, it carries I11's discipline: on every
  pass the pre-layout pass inserts or removes it on every exit path — the gate-out, `display: none`
  (`generated_content.rs:105`), `display: contents` (`:114`) and the depth cap (`:58`) — so no value from an
  earlier pass survives on an element that no longer resets a reversed counter. Under every shape the value has
  **one definition** and both drivers obtain the same value (§2, I6 × I9; §0.5 row 4 draws the system boundary).
- **D-5 — `counter-*` resolve independently of `content`** (N-1). *Rung 1*: appliesTo "all elements".
- **D-6 — the UA rule is in the core UA sheet.** `ol, ul, menu { counter-reset: list-item; }` goes into `UA_CSS`
  (`crates/css/elidex-style/src/ua.rs`). *Rung 1* gives the rule's text (HTML §15.3.7) and, in every engine mode,
  CSS Lists 3 §4.6's "should" (the UA sheet reflects the host language's numbering). *Rung 2* gives the sheet:
  #408's criterion for UA rules (rules of conforming elements → the core sheet, applied in every mode; `ol`, `ul`,
  `menu` are conforming, HTML §4.4.5 – §4.4.7). Whether rung 1 makes the rule a "must" in a given mode is the
  designation reading of §1.2; the decision is the same under every reading and every option of §1.6.
  **`dir` resets with them, in the same sheet** (`ol, ul, menu, dir`): *rung 1*, HTML §16.3.3's "must" — a
  requirement outside §15, so no designation reading touches it — gives `dir` every rendering rule `ul` has, in
  the sheet and the modes in which `ul` has it (§4.3). For #408's status criterion that is a second exception
  beside `tt`, on a rung-1 ground (`tt`'s is reach).
- **D-7 — the list attributes are presentational hints, at the hint position.** `ol[start]` / `[reversed]` →
  `counter-reset` and `li[value]` → `counter-set` enter the cascade through the existing hint seam
  (`extra_declarations` of `collect_and_cascade`, `crates/css/elidex-style/src/cascade/mod.rs:231`), so an author
  declaration of the same property replaces them and the UA rule sits below them. *Rung 1*: HTML §15.3.7, §15.2.
  **In which engine modes they are generated is P-1 (§1), not decided here.** The hint output is a function of the
  element's `TagType`, its `Namespace` and its `start` / `reversed` / `value` (and, in LC-5, `type`) attributes: a
  change of any of them is a style-input change of that element (§2, I2). The namespace is an input because HTML
  §15.3.7's rules are scoped to HTML elements (its sheets open with `@namespace "http://www.w3.org/1999/xhtml"`);
  the bake and the compat generator read none today (§7 F-14).
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
  (`crates/core/elidex-plugin/src/traits.rs:111`, `:121`): for `counter-*` the single parse path is the declaration
  parser in `elidex-css` and the single resolve path is the cascade's resolve step, and the handler's two
  trait-forced entries call those implementations — they hold no copy (§5.3 LC-6). It lands after D-1, so the first
  value script sees is the cascade's. **Registering the handler also changes cascade inputs**: `inherit`, `unset`
  and `initial` on `counter-*` resolve through the handler's `get_computed`, `is_inherited` and `initial_value`
  (`resolve_keyword`, `crates/css/elidex-style/src/resolve/helpers.rs:19`–`:35`; `inherit.rs:15`–`:36`), so the
  handler's output re-enters the cascade's resolve and must have the shape that resolve reads (§5.2, window W6;
  §5.3 LC-6). *Rung 1*: CSSOM 1 §9. *Rung 2*: Plugin-first (the seam); One issue, one way (one implementation).
- **D-11 — HTML §15.3.7 is taken as a whole section.** §4.3 lists every normative unit of the section with its
  disposition; none is left unnamed, and a unit this program does not repair is routed with an owner or a proposed
  slot (§7).
- **D-12 — seven slices after a split prerequisite, in the order §5.2 derives.** The order satisfies the
  constraints c1 – c13 of §5.2, each of which is a cell that is right at some point of the program and would turn
  wrong under another order. §5.2's two tables — the units that exist at `bf1cae35`, and the units each slice
  **adds** — show no cell turning wrong in any window of the chosen order, apart from option A's hint population
  (which is a decision, OQ-5).

### §0.5 OO idioms translated (ECS-native first)

| Shape in the code today | ECS-native shape this program lands |
|---|---|
| A post-cascade fix-up mutates the style object by element class (`match tag { "ol" … "li" … }`), re-run by three callers "to be safe" | The style component has one writer (the cascade system). Markup facts reach it as cascade inputs: a UA rule and a pure function `(TagType, Namespace, Attributes) → declarations` |
| The numbering state is smuggled through the style object (a `reversed` flag and a pre-counted integer baked into `counter_reset`; the increment's sign fixed up inside the machine) | Counter values are **derived** by a system that reads `ComputedStyle` in tree order and writes `ListItemMarker` / pseudo `TextContent`; nothing derived is written back into `ComputedStyle` |
| "Is this a list item?" asked of the element's class (tag) | Asked of the component the cascade wrote (`display`) |
| Per-element counters set, and a reversed counter's initial value (the spec's model is per element) | **The boundary.** The two drivers are two systems — the pre-layout pass (`elidex-style`, before layout) and the paged walk (`elidex-render`, at display-list build) — that share one step function; the paged walk is not "inside" the pre-layout pass. Readers of the counter state at this base are those two only (`git grep -n 'CounterState' -- crates`, non-test: `generated_content.rs`, `elidex-render/src/builder/mod.rs`, `paged.rs` and `walk.rs`, and a doc comment in `builder/paint/mod.rs`). **The counters set** stays what it is today: state of one run of one driver, keyed by counter name and scope depth (not by entity), recomputed from components (`ComputedStyle` in tree order) on every run; that the paged walk re-derives it rather than reads it is the master plan's settled choice for per-page values (D-9), not reopened. **The reversed initial value** follows D-4's criterion (CLAUDE.md *Side-store → component*, at its precondition: state held beyond one computation): crossing from the pre-layout pass to the paged walk, or outliving the run → a component written by the pre-layout pass, which the paged walk then reads, inserted or removed on every exit path (I11's discipline); computed and dropped inside one run (scratch, under any key) or kept only in the counter instance → no store. LC-1 picks, and its plan shows that the pre-layout pass has run before the paged walk reads. **Promotion of the counters set itself**: a reader outside these two systems (accessibility ordinals, say, §7 F-8) makes it a component written by the pre-layout pass |

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

No elidex record names an engine mode as "designated": `git grep -n -i -E 'suggested default rendering|designated
as supporting|conformance class' -- docs CLAUDE.md crates .claude/skills` at `aeecf9b0` has one hit outside this
memo, IBP's quotation of the §15 prelude (`docs/plans/2026-09-inline-box-predicate.md:666`); the same pattern over
the memory directory and the PM portfolio files has none (control: `grep -rl -i presentational <memory dir> | wc -l`
→ 5, verified 2026-11-26). The spec permits each of DR-1 – DR-3 ("may", "possibly as a user option", "This is not
required"), so **choosing among the readings is not rung 1**: it is elidex's choice, and rung 1's requirements
then follow from it. DR-4 is not a fourth thing the spec offers; it is the reach IBP's ratified passages **apply**
(rev 3; rev 2's table lacked it).

| | **DR-1** — every engine mode is designated, for the whole section | **DR-2** — the designation is the engine mode: `BrowserCompat` is designated, `BrowserCore` / `App` are not | **DR-3** — no mode is designated | **DR-4** — every mode is held to §15, less a compat carve-out (quirks; legacy / obsolete features) |
|---|---|---|---|---|
| HTML §15 in `BrowserCompat` | every rule and every hint is a "must" | every rule and every hint is a "must" | advisory | every rule and every hint is a "must" (nothing is carved there) |
| HTML §15 in `BrowserCore` / `App` | every rule and every hint is a "must", obsolete ones included | advisory: HTML requires neither rules nor hints there | advisory | a "must" for every unit the three-layer rule does not place in compat. Carved: the quirks rules, the rules of §16.2-obsolete elements (#408) less `dir`'s, which HTML §16.3.3 equates with `ul` (D-6 — §16.3.3's "must" is outside §15, so no carve reaches it; the other four equivalences of §16.3.3 are not decided here: they are `IBP-ua-display`'s rung-1 input, §7 F-12), and whichever hints the option of §1.6 carves |
| Where the core-mode "must" gets its force | HTML §2.1.8 | — | — | HTML §2.1.8 knows no partial designation. So DR-4 is either a designation with a recorded departure (the carve) — **rung 1, with a recorded nonconformance** — or no designation plus a ratified elidex rule (IBP §0.2 as applied) — **rung 2**; the ratifier names which. The reach is the same. Every later "under DR-4" ground in this memo carries these two forms ("DR-4's force") |
| IBP §0.2 "HTML §15's *expected* is read as **must**" (`:664`–`:666`; a §0 premise, ratified by #526; stated without a mode; the cell test evaluates under the current, compat-included cascade, `:886`–`:887`) | consistent | consistent, as a `BrowserCompat`-only "must" | **contradicts** | consistent |
| IBP's **applications** of that premise, all ratified (§0 premises and §0.6): §0.3 *Why* (`:706`–`:708`: rule (3) "is a must under the §15 normativity above", its missing alt-text rendering "a separate core gap"); §0.4 (`:717`–`:723`: the quirks disjunct "is compat and excluded from the core predicate"); §0.6 item 3 (`:803`–`:813`: §15.5.3's "expected" decides the core predicate). The facts they decide are mode-independent | consistent (the quirks disjunct never fires: elidex never enters quirks mode) | **strained**: the "must" these passages lean on would bind `BrowserCompat` only, and the one mode-independent predicate would follow it in core modes by rung 2 — a step IBP nowhere states, and "excluded from the **core** predicate" reads against | **contradicts** | consistent: this is the reading they apply |
| E0 (core arm: "no legacy UA, no hints", §5 item 4) | **contradicts** (both halves of the gate) | consistent — E0's gate would be the designation switch | consistent | sheet half consistent (after #408 the gated sheet holds obsolete-element rules only). Hint half consistent **only if every hint is carved** (option A); with a carve by status it is amended for current-feature hints (the passages of §1.7) |
| #408 (conforming-element rules in the core sheet "applied in every engine mode"; obsolete-element rules compat-gated) | **contradicts** its compat half (those rules are "must" in core modes too) | consistent; the core half is a rung-2 selection from an advisory section, not a §15 "must" | consistent, likewise | consistent, both halves, with `tt`'s reach exception and `dir` (HTML §16.3.3's `dir` ≡ `ul`) outside the compat half (the other four equivalences are §7 F-12's input to `IBP-ua-display`); the core half is a "must" (DR-4's force) |
| CLAUDE.md *Whole-engine core / compat / deprecated consistency* | **contradicts** (legacy rendering in core) | consistent | consistent | consistent: it is the carve criterion |
| D-6's UA rule in core modes | "must" | CSS Lists 3 §4.6 "should" + rung 2 | the same | "must", in DR-4's force: rung 1 with a recorded nonconformance, or rung 2 |

**What the table shows.** No reading is consistent with every ratified record as written. DR-2 fits E0, #408 and
CLAUDE.md, and leaves IBP's applied "must" on core facts without its stated ground in core modes. DR-4 fits IBP,
#408 and CLAUDE.md, and fits E0's hint half only if every hint is carved; otherwise E0's hint gate and IBP's reach
conflict independently of this slot — to the extent that reach covers hints, which IBP's ratified text does not
say (§1.3's caveat) and its input assumes; PM 985 (`PORTFOLIO-archive-2026-W48.md`) names the two as in
conflict. DR-1 is the only reading under which rung 1 by itself decides the hint population (every hint, every mode). Rev 2 called
DR-2 "the only reading consistent with all three ratified records"; that rested on IBP's premise sentence alone and
is withdrawn (plan-review R2, A5-I1).

Two supporting records. (i) "Production is `BrowserCompat`" is recorded in the open-slot registry (memory
`project_open-defer-slots.md`, slot `#11-enginemode-full-session-threading`: "production is uniformly
`BrowserCompat`", the iframe builders and the navigation rebuild hard-code it; `BrowserCore` / `App` are not
production-selectable until `#11-async-core-storage-cookiestore`), and in E0's own text (§6 Q2,
`docs/plans/2026-06-e0-shell-style-compat-mode.md:204`–`:207`: "production is uniformly `BrowserCompat`") and the
philosophy-alignment umbrella §2.6 (`docs/plans/2026-06-elidex-philosophy-alignment-umbrella.md:266`: "Production
stays `BrowserCompat` (byte-identical)"). (ii) The same registry's
`#11-form-control-ua-rendering-fidelity` records §15 UA-rule defects that #408 "moved **verbatim** into the core
sheet" as defects that "now also [affect] `resolve_styles`/`BrowserCore`": a landed record that treats §15's rules
as owed in core modes (DR-1's or DR-4's side, for the rule half; IBP's ratified §6 enriches that slot, `:1688`).

One limit on IBP's reading, for completeness: where §15 and another normative text disagree (§15.5.3's button
layout against css-ui-4 §7.2), IBP decides by a probe, not by §15's letter (`:1515`–`:1524`). No such disagreement
exists for the list hints: CSS Lists 3 §4.6 asks for the same mappings §15.3.7 defines.

**Uncertain, and for the ratifier:** which reading elidex holds. This memo does not choose one; OQ-1 (a) carries
all four with the precedents on each side (§1.8). HTML §16.3's requirements for obsolete features do **not** hang
on this question: §16.3.3's "must" is outside §15 (D-6, §4.3).

### §1.3 What the rungs decide under each reading

- **DR-1** → every hint in every mode, and the legacy UA sheet in every mode: option **C** (§1.6). Rung 2 is not
  consulted; E0's gate has nothing left to gate.
- **DR-4** → in `BrowserCore` / `App` §15 binds for every unit not carved, in DR-4's force (§1.2): at rung 1 with a
  recorded nonconformance if the ratifier takes DR-4 as a designation with a recorded departure, at rung 2 if as no
  designation plus IBP's applied rule. The UA rule (D-6) is a "must" there in that sense.
  For hints the question becomes the carve: which hints the three-layer rule sends to compat. That rule (rung 2)
  carves "legacy" and asks a "modern standard baseline" reason for what stays in core, and HTML's own attribute
  classes (§1.1) put `start`, `reversed`, `value` and `ol[type]` in the current language and `li[type]`, `ul[type]`
  in §16.2. So a carve **by status** gives option **B**: its core half is the reading's reach applied to hints,
  its compat half is the carve. Option **A** needs every hint classed as legacy, which those classes do not support
  for the four current-language attributes; option **B0** follows from no carve criterion. One caveat: IBP's
  ratified applications are all of UA-level expected behaviour; that the reach covers **hints** is what §15.2
  implies (one section, no hint exception, §1.1) and what IBP's unreviewed input assumes — it is not itself
  ratified.
- **DR-2, DR-3** → in `BrowserCore` / `App` HTML requires nothing. CSS Lists 3 §4.6 still asks, at "should"
  strength and whatever the designation, that numbering reflect the host language's values: the UA rule (D-6) and
  the `start` / `reversed` / `value` mappings. **Rung 1 therefore sets a "should" against option A for these three
  hints; it does not reject A by itself** — a "should" may be set aside for a stated reason. **Rung 2 finds no such
  reason**: these are conforming attributes with normative meaning (the list's *starting value*, the item's
  *ordinal value*), and the core sheet already numbers lists (`li { display: list-item }`, `ol { list-style-type:
  decimal }`). The one candidate reason is E0's own ratified sentence, "no hints in core". This memo does not find
  a reason in it: E0 is the decision under review, not a principle; its plan gates the hint generator wholesale
  (its §3.1 names "`bgcolor`/`width`/`align`/…" and no list attribute — the list attributes are not generated as
  hints at all, the bake applies them in every mode, §0.2, so E0's gate has never covered them); it gives no ground
  specific to a conforming attribute; and the ground it does give, the three-layer rule, is the one that asks for a
  "modern standard baseline" reason — which these attributes have. If the ratifier nevertheless holds E0's sentence
  as the reason, the answer is A, as a recorded decision to set a normative "should" aside (OQ-1 (b), OQ-5). For
  every other hint — `ol[type]`, the width / height family, §15.5.9's color well — rung 1 says nothing under these
  readings; they fall to rung 2.

### §1.4 Rung 2 — elidex's principles, for what rung 1 leaves

What is left: under DR-2 / DR-3, the placement of every hint (for the three list hints, whether any ground sets
§4.6's "should" aside); under DR-4, the carve criterion. Under DR-1 nothing is left. Lens block (one line per
principle: which option it selects; options in §1.6).

- *Whole-engine core / compat / deprecated consistency* ("Legacy / non-standard … を core に混ぜない。Core に置くなら
  「modern standard baseline として必要」な理由"): selects **B** over B0 and A — a hint that maps a current-language
  feature is the rendering of modern markup; rejects **C** (`bgcolor` in core).
- *One issue, one way*: selects **B**. The §15.2 surface has two halves, UA rules and hints. The rule half is
  split by spec status (#408). B gives the hint half the same criterion. A keeps two criteria for one surface
  (status for rules, "is a hint" for hints); **B0** keeps A's two criteria and adds an exception list ("unless
  another spec asks for the mapping"), which every later hint must be checked against.
- *Ideal over pragmatic*: selects **B** over B0 (B0 answers this slot's hints and leaves the criterion undecided).
- *ECS-native first*: indifferent — under every option the hint source is a pure function of three components
  (`TagType`, `Namespace`, `Attributes`).
- *Supported-surface testing*: indifferent between B0 and B; under A the two core arms and every hint-less cascade
  entry need cells that pin the dropped numbering (§5.3, the entry-point table).

Every principle that discriminates selects B. Rung 3 is not reached.

### §1.5 Conclusion — proposal P-1 (for the PM to route to E0's ratifier; not decided here)

> **P-1a.** The list-numbering mappings — the UA rule `ol, ul, menu { counter-reset: list-item; }` and the hints of
> `ol[start]`, `ol[reversed]`, `li[value]` — apply in every engine mode. *Ground: under every reading, rung 1 sets
> CSS Lists 3 §4.6's "should" against withholding them. Under DR-2 and DR-3 rung 2 then finds no ground to set that
> "should" aside (§1.3). Under DR-4, HTML §15 requires them in core modes as well, in DR-4's force (§1.2: rung 1
> with a recorded nonconformance, or rung 2 by IBP's applied rule). Under DR-1 it is part of C.*
>
> **P-1b.** In general, a presentational hint's layer follows the status of the markup feature it maps (a content
> attribute, or element state such as §15.5.9's color-well value): a current-language feature → **core**, generated
> in every engine mode; a feature HTML §16 lists as obsolete — §16.2 non-conforming, and §16.1 obsolete but
> conforming (the spec's own name for it is obsolete, and it gives the CSS replacement) — → **compat**, behind
> `presentational_compat()` of `StyleCompatPolicy`. For this program: `ol[type]` is core; `li[type]`, `ul[type]`
> are compat. *Ground: rung 2 under DR-2 and DR-3 (§1.4). Under DR-4 the core half is the reading's reach applied
> to hints, in DR-4's force (rung 1 with a recorded nonconformance, or rung 2), and the compat half is the carve,
> whose criterion is rung 2 (the three-layer rule; #408).*

**Recommendation: B = P-1a + P-1b, under DR-2, DR-3 or DR-4; C under DR-1.** Against rev 2 ("B on reading
DR-2"): the option is unchanged, its ground is not. Rev 2 held DR-2 to be the only reading consistent with the
ratified records and wrote "rung 1 rejects A". Both were over-claims (plan-review R2, A5-I1 and A4-I1). With IBP's
applied passages tabled, no reading fits every record (§1.2), and B is what three of the four readings give, each
on its own ground:

| Reading | P-1a stands on | P-1b stands on |
|---|---|---|
| DR-2, DR-3 | rung 1 "should" (CSS Lists 3 §4.6) + rung 2 (no ground to set it aside) | rung 2 (three-layer consistency; One issue, one way with #408) |
| DR-4 | the same "should", and HTML §15 as IBP applies it, in DR-4's force (rung 1 with a recorded nonconformance, or rung 2) | core half: that reach, extended to hints (not itself ratified, §1.3), in the same force; compat half: the rung-2 carve |
| DR-1 | — (C: every hint, every mode, at rung 1; E0, #408's compat half and the three-layer rule for styles are overturned) | — |

This memo does not choose the reading. What the choice changes: (i) the rung P-1 stands on; (ii) whether E0 and IBP
conflict independently of this slot (DR-4: yes, once its reach is taken to cover hints; DR-2: no — only this
proposal amends E0); (iii) what `IBP-layout` owes for
the dimension hints (DR-4: by the reading; DR-2 / DR-3: by P-1b, at rung 2). So the reading is put to the ratifier
with the option (OQ-1). If the ratifier accepts only P-1a (option B0), LC-3 is unblocked and LC-5's `ol[type]`
stays compat.

**Does it overturn E0?** Under B and B0 E0's mechanism stands: one `EngineMode`, `StyleCompatPolicy` derived in
parallel, one boolean gating "the legacy UA sheet + the compat hint generator", `elidex-style` mode-agnostic. What
changes is **what the gate's hint half covers**, which contradicts E0's ratified text in the places §1.7 lists. So
it is an amendment of a ratified decision and is the ratifier's to accept. Under C E0's gate is removed.

### §1.6 The options, and what each keeps and overturns

| | **A** — every HTML hint is compat (E0 as written) | **B0** — A, except the list-numbering hints (P-1a only) | **B** — by the mapped feature's status (P-1a + P-1b) | **C** — every hint, and every §15 rule, in every mode |
|---|---|---|---|---|
| Reading it rests on | DR-2 or DR-3 with §4.6's "should" set aside (the reason offered would be E0's own sentence, §1.3); or DR-4 with every hint classed as legacy | DR-2 or DR-3; under DR-4 no carve criterion yields it | DR-2, DR-3 or DR-4 | DR-1 |
| Rung | none found: rung 1 sets a "should" against it for three hints under every reading (and, under DR-4, §15's "must" for every current-feature hint, in DR-4's force: rung 1 with a recorded nonconformance, or rung 2); rung 2 finds no ground to set them aside | rung 1 "should" + rung 2 (no ground to set it aside) | P-1a as B0; P-1b at rung 2 (DR-2 / DR-3), or DR-4's reach (in DR-4's force) with a rung-2 carve | rung 1 |
| Keeps | every passage of §1.7 verbatim; design doc §15.3 / §29.5's listing of width / height hints under the compat layer | E0's mechanism; all of §1.7 except its "every hint" wording; `engine_mode_gates_presentational_hints`; today's mode-independent list numbering | E0's mechanism and its one-boolean policy; #408's criterion; the three-layer rule; `engine_mode_gates_presentational_hints` (it pins `<font color>`, an obsolete attribute); today's mode-independent list numbering | today's list numbering |
| Overturns, or costs | no ratified record. Today's behaviour: `start` / `value` / `reversed` apply in every mode and at every cascade entry (§0.2); under A, LC-3 **removes** them from `BrowserCore` / `App` and from every hint-less entry, the WPT harness included (§5.3); and once a core session exists, a list in it is numbered differently from the same list in its own iframe, whose builders hard-code `BrowserCompat` (`#11-enginemode-full-session-threading`). IBP Revision 38's repair shape. And §15.5.9's color-well hint, which IBP's ratified §2 I1 states (`docs/plans/2026-09-inline-box-predicate.md:1206`, the hint at `:1222`–`:1224`), is withheld in core modes (the last row) | E0's "every hint" sentences (§1.7), for three hints. Leaves two criteria plus an exception on the §15.2 surface; `ol[type]` and `IBP-layout`'s question stay as A | E0's sentences of §1.7 (they become "legacy presentational hints"); design doc §15.3 / §29.5's placement of width / height hints, once `IBP-layout` applies the criterion | E0's gate entirely, with its plan; #408's compat sheet (the obsolete-element rules become core); the three-layer rule for styles; `engine_mode_gates_presentational_hints` |
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
| E0 §2 Axis 1 (`:100`–`:107`) | "`elidex-style` stays mode-agnostic"; the shell "selects which existing elidex-style entry to call"; `get_presentational_hints` remains in `elidex-dom-compat`; "E0 moves **no algorithm**" | kept: the core hint source applies in every mode, so `elidex-style` still reads no `EngineMode` (OQ-2: applied by the cascade itself) | void |
| E0 §6 Q1 (`:200`–`:203`) | open question: one boolean, or "legacy UA sheet" and "presentational hints" separately queryable — "grow when a mode splits them" | kept: no mode splits the two legacy halves | void |
| E0 §5 item 5 (`:195`) | "Compat algorithm stays in `elidex-dom-compat`" | kept for this program (the list hint source is new code); under B a conforming-attribute hint is not compat, so `IBP-layout`'s move takes that conversion out of `elidex-dom-compat` | void |
| `StyleCompatPolicy` rustdoc (`crates/core/elidex-plugin/src/spec_level.rs:203`–`:204`, `:206`–`:209`, `:218`–`:219`, `:224`–`:226`) | the same sentences as E0 §1a | amends; LC-3 edits them | void |
| `EngineMode::style_compat_policy` rustdoc (`spec_level.rs:126`–`:127`) | "`BrowserCompat` applies the legacy presentational surface; `BrowserCore` / `App` resolve against the modern UA baseline only" | kept: it is exact under B0 / B (the baseline includes the core hints) | void |
| `elidex-dom-compat` crate doc (`crates/dom/elidex-dom-compat/src/lib.rs:1`–`:6`, `:16`–`:18`) and package description (`Cargo.toml:3`) | the crate normalises "legacy HTML and CSS patterns" and is linked by the shell, not by core crates; its hints convert "HTML attributes (`bgcolor`, `width`, `border`, `color`, etc.)"; "legacy tags, presentational hints, vendor prefixes" | A: LC-3 adds the list hints to this generator and to these lists. B0 / B: kept as the description of the legacy generator (LC-3 words the hint lines "legacy"); `width` leaves it only when `IBP-layout` applies P-1b | void |
| Parser test comments (`crates/dom/elidex-html-parser/src/convert.rs:461`, `:467`; an `<img width height>` test) | "Presentational hints are now handled by elidex-dom-compat"; "generated by compat layer during cascade" | B0: kept. B: amends, when `IBP-layout` applies P-1b; not this program's edit | amends |
| Shell comments landed with #406 (`crates/shell/elidex-shell/src/lib.rs:138`–`:145`, `:160`–`:161`) and the `hint_generator` doc (`crates/css/elidex-style/src/lib.rs:333`–`:335`) | core arm: "no legacy UA sheet, no presentational hints" | amends; LC-3 edits them (not ratified text) | void |
| A0 anchor design §3.2b (`docs/plans/2026-06-web-api-compat-split-design.md:516`) | "style layer derives: style-compat policy (UA sheet + presentational hints on/off)" | amends (one on/off unit → the legacy unit) | void |
| A0 §5, E0 row (`:750`–`:755`) | the policy derives from the engine-wide `EngineMode`, parallel to `SpecLevelPolicy` | kept | void (nothing to derive) |
| Design docs `docs/design/ja/15-rendering-pipeline.md` §15.3 (`:66`–`:67`), `29-survey-analysis.md` §29.5 (`:165`–`:169`) | width / height hints are a P0 item "of the compat layer" | B0: kept. B: amends, when `IBP-layout` applies P-1b | amends |
| #408's criterion (`crates/css/elidex-style/src/ua.rs:111`–`:116`; `crates/dom/elidex-dom-compat/src/legacy_ua.rs` header) | conforming-element rules → core; §16.2-obsolete → compat, with `tt` as the one stated exception (`ua.rs:68`–`:72`; the `legacy_ua.rs` header's parenthetical, `:4`–`:6`) | kept as a criterion, with a **second exception** under every option, A included: `dir`, on HTML §16.3.3's rung-1 ground (D-6). Both statements name `tt` alone, so LC-3, which lands `dir`'s first core rule, adds `dir` to them (§5.3). Extended to hints by B | amends (no compat sheet) |

### §1.8 Precedents on both sides

**Search.** By concept, in English and Japanese, over the repo's docs (at `aeecf9b0`), the memory directory and the
PM portfolio (both outside git; counts as of 2026-11-26):

```
P='presentational|presentation[ -]attribute|プレゼンテーション|プレゼンテーショナル|表示属性|属性ヒント|スタイルヒント|presentation hint|bgcolor'
git grep -l -i -E "$P" -- docs CLAUDE.md .claude            # 26 files (this memo among them)
git grep -n -i -E 'ヒント|[^a-z]hints?[^a-z]' -- docs/design/ja CLAUDE.md   # second sweep, read at every hit
grep -rl -i -E "$P|ヒント|StyleCompatPolicy|style[- ]compat|#406|#408|legacy UA" <memory dir>                # 8 files
grep -n  -i -E "<the same>" <PM dir>/PORTFOLIO-2026-10-26.md PORTFOLIO-archive-*.md PORTFOLIO-index.md   # 5 lines
```

Control (a known hit the first pattern must find, which rev 1's missed): `docs/design/ja/19-svg-rendering.md:92`,
`:94`, `:99`, `:103` — found. Read at their hits: every file under `docs/design/ja`, the plans, the audit, the
memory files, the portfolio lines and the PM decisions they cite. PM decisions are cited by number and archive
file, since their lines move: PM 611, 620, 629, 637 in `PORTFOLIO-archive-2026-W46.md`; PM 985 in
`PORTFOLIO-archive-2026-W48.md`; PM 470 in `PORTFOLIO-archive-2026-W45.md`. Not read separately: the
`docs/design/en` translations. Incidental hits (the word, or `presentational.rs`, with no placement statement):
the line-box, tree-model (two), AbortSignal, EH-attrs and Flex-shorthand-parser plans, the PR-A2 memo (the slot
text itself), `06-thread-model.md:600` (surface presentation), `25-accessibility.md:78` (ARIA "presentational"),
`11-parser-design.md:155` (a usage figure); memory `project_html-section-renumber-sweep.md`,
`project_pr396-c1-viewport-structural-pivot.md`, `project_slice1-elementstate-cache-deletion-state.md` (a
legacy-colour slot order), `m4-12-landings-ledger.md` (E0's done-ify line: #406 + #408, no placement),
`m4-12-pr-tags-t2c-table-landing.md` (deprecated table attributes as IDL reflection, not as hints); three of the
five portfolio lines (the CS lane's legacy-colour parsing in `presentational.rs`, PM 1135 among them). What the
sweep cannot see: a decision recorded under none of these words — the PM adds those when routing.

| Record | Status | Says | Side |
|---|---|---|---|
| E0 plan, the passages of §1.7; landed #406 (`f39f16d2`) | ratified, landed | the core arm gets "no legacy UA sheet, no presentational hints" | A |
| `StyleCompatPolicy` rustdoc (§1.7) | landed with #406 | one field gates "legacy UA stylesheet + HTML presentational hints — WHATWG HTML §15.2" | A |
| Anchor design (A0) §3.2b, `:516` | ratified | hints are one on / off unit | A |
| PM 985 (`PORTFOLIO-archive-2026-W48.md`); PM 620 (`PORTFOLIO-archive-2026-W46.md`); memory `project_line-box-decorated-inline-content.md` (its "11-10 PM note (IBP hints)") | PM decisions | describe E0 as "presentational hint はすべて compat" and route the conflict to this slot's plan-review; a conclusion that overturns E0 goes to E0's ratifier | A as a description of E0; no new placement |
| PM 611 (`PORTFOLIO-archive-2026-W46.md`): conforming attributes → "hint の源は core、compat は廃止属性の hint だけ" | **withdrawn** by PM 620 ("2 つの ratify 済み面を読まずに") | P-1b's outcome | neither (withdrawn) |
| Design docs `15-rendering-pipeline.md` §15.3 (`:66`–`:67`), `29-survey-analysis.md` §29.5 (`:165`–`:169`) | design SSoT | width / height hints are a P0 item of the compat layer ("compat 層実装優先度") / of the "Compatルール優先度" | A |
| Design doc `29-survey-analysis.md` §29.2 (`:53`–`:59`) | design SSoT | counts `width` / `height` under the heading 非推奨属性, and in the same paragraph calls them size hints that prevent layout shift, a pattern recommended today | both |
| Design docs `12-dom-cssom.md` §12.2.2 (`:144`), `29-survey-analysis.md` §29.7 (`:210`) | design SSoT | "presentational hints対応はP0要件。StyleSystemはスタイル解決時にこれらをCSS初期値として扱う必要がある"; "Phase 1のCSSパーサーでpresentational hintsをサポートする必要がある" — the style system and Phase 1 are named, no compat layer | lean B |
| Design doc `19-svg-rendering.md` §19.2.2 (`:92`–`:103`) | design SSoT | SVG presentation attributes enter the cascade as author-level, specificity-0 declarations, handled by the StyleSystem, "特別なパスは不要", no compat layer | B, by analogy: another current-language attribute → property mapping (SVG, not HTML; E0's text is about HTML hints) |
| — the design SSoT taken together | | split: §15.3 / §29.5 place the HTML hints in the compat layer; §12.2.2 / §29.7 / §19.2.2 place attribute mappings in the style system | split |
| #408 (`75c327c1`), E0's prerequisite; `ua.rs:111`–`:116`, `legacy_ua.rs` header | landed | "Reclassify UA-stylesheet rules by spec status": conforming → core sheet "applied in every engine mode"; only §16.2-obsolete → compat. Cause: E0's core arm had dropped "the only source of standard §15.3 rendering" | B (the criterion, applied to the rule half) |
| #408, the same PR | landed | (i) its criterion has an exception: `<tt>` is §16.2-obsolete and stays in the core sheet, for reach into shadow trees (`ua.rs:68`–`:73`); (ii) its PR body calls the hints "legacy/presentational compat" and the compat cascade "(`ua` + `legacy` + hints)" (`gh pr view 408 --json body`, body lines 6 and 33) | (i) placement was not decided by status alone: the rule half's criterion is status **plus a reach exception**, which IBP keeps as a live placement input for `IBP-ua-display` ("compat-sheet placement for shadow trees decided with the `tt` precedent", IBP `:1585`–`:1588`, `:1607`). The exception does not arise for hints: the hint generator already reaches shadow trees (§5.3's walk row), unlike the compat sheet. (ii) A for hints |
| Design doc `07-plugin-system.md` §7.6 (`:176`–`:185`); `01-executive-summary.md` §1.2 (`:12`–`:14`); anchor design (A0) `:517`–`:521` | design SSoT / ratified | `HtmlSpecLevel` classifies per tag by status (`Html5` = Living Standard, `Legacy` = 非推奨タグ); "HTML5のみのコア … レガシータグ処理、Quirksモード、エラー回復はすべて … コア外"; `BrowserCore` → "install Modern only" | B (the status criterion, stated for tags and APIs; silent on attributes); also the classification OQ-2's option Y would reuse |
| Audit `docs/audits/2026-06-elidex-philosophy-implementation-audit.md` (`:243`, `:300`) and the philosophy-alignment umbrella §2.6 (`docs/plans/2026-06-elidex-philosophy-alignment-umbrella.md:248`) — E0's origin (F6) | audit / landed plan | the compat path is "legacy UA styles plus presentational hints"; the audit asks whether core paths "can observe legacy UA stylesheet / presentational hints" | A as a description (hints named beside the legacy sheet as one compat unit); neither classifies a hint by status. The umbrella's resolution paragraph (`:262`–`:266`) does classify the **rules**: #408 "reclassified the UA stylesheets by spec status (§15.3 conforming → core `ua_stylesheet`, §16.2 obsolete → compat `legacy_ua`)" — B for the rule half, as #408's own row |
| IBP §0.6 item 15 (`docs/plans/2026-09-inline-box-predicate.md:1065`, the passage at `:1074`–`:1076`) | ratified (§0.6, USER DECIDED 2026-09-29) | withdraws S5-4's compat label for CSP `sandbox`: "a modern standard, not a legacy or quirks surface, so CLAUDE.md's core/compat discipline places it in core" | **B** (the status criterion, applied by a ratified record to a surface outside the UA sheets) |
| Memory `project_open-defer-slots.md`: `#11-form-control-ua-rendering-fidelity`; `#11-enginemode-full-session-threading` | open-slot registry | §15 UA-rule defects "moved **verbatim** into the core sheet" by #408 "now also [affect] `resolve_styles`/`BrowserCore`"; production is uniformly `BrowserCompat` | bears on the reading (§1.2): §15's rules treated as owed in core modes |
| CLAUDE.md *Whole-engine core/compat/deprecated consistency* | standing rule | legacy stays out of core; core needs a "modern standard baseline" reason | B (rejects C) |
| Memory `policy-deprecated-and-annex-b-out-of-scope.md:19` | policy memo (its attribute list is its own: `git grep -n -i bgcolor -- docs/design/ja` → 0) | deprecated HTML attributes, "`width`/`height` (presentational on most elements)" and "`type` (UL)" among them, are outside core scope | A, and B's compat half; silent on conforming attributes |
| IBP §0.2 (`:664`–`:666`) | ratified premise (#526) | HTML §15's "expected" is read as "must" | bears on the reading (§1.2), not on a layer |
| IBP §0.3 *Why* (`:706`–`:708`); §0.4 (`:717`–`:723`); §0.6 item 3 (`:803`–`:813`) | ratified (§0 premises; §0.6) | the premise **applied**: §15.4.2 rule (3) "is a must" for the core replacedness predicate, and its missing rendering is "a separate core gap"; rule (2)'s quirks disjunct "is compat and excluded from the core predicate"; §15.5.3's "expected" decides the core inline-box predicate | **B's side**: §15 binds mode-independent core facts, and what leaves core is carved by kind (quirks) — reading DR-4. For UA-level behaviour only; no hint is named |
| IBP `:1500`–`:1501` (`IBP-classify`: F4 per §15.4 / §15.4.2, F7 per each §15.5.x); `:1546` (`IBP-layout` lays out §15.5.3's button layout) | **input** (per-sub-PR obligations) | the same reach carried into the sub-PRs: §15.x decides core classification and layout | B's side, as input |
| IBP `:1688` (the `#11-form-control-ua-rendering-fidelity` row of IBP §6) | **ratified**: §6 *Slots and ledger* is in IBP's approval scope (`:3`–`:9`) | enriches the form-control slot, whose own text treats §15's UA rules as owed in `BrowserCore` (§1.2 (ii)) | bears on the reading (§1.2), DR-1's or DR-4's side for the rule half |
| IBP §0.6 item 7, the cell test (`:886`–`:892`) | ratified rule | M is markup "under elidex's **current** cascade (UA and compat sheets included)"; ownership of an exposure is the exposer's | neither: whether core-mode iframe sizing is a cell is `IBP-layout`'s to derive |
| IBP Revision 38 item (1) (`:279`–`:282`); §2 pair `IBP-classify × IBP-layout` (`:1269`–`:1275`); Appendix D (`:1859`–`:1863`); §3's §15.3.1 row (`:1312`) | **input**, not ratified (IBP's approval-scope paragraph, `:3`–`:9`) | the repair is "the dimension-attribute hints on the core path"; criterion: "a rendering-section hint, not a legacy presentational attribute" | B's outcome for those hints, by a **different criterion**: IBP's separates hints by where the spec defines them and does not say which hints are legacy; P-1b separates by the mapped feature's status. For §15.5.9 (the hint maps the element's value, not a content attribute) IBP's wording says core; P-1b says core only because it is worded over "markup feature", not "attribute" |
| Shell test `engine_mode_gates_presentational_hints` (`crates/shell/elidex-shell/src/tests.rs:925`) | landed | `BrowserCore` drops `<font color>` | A and B (obsolete attribute) |
| The bake (`counter.rs:299`), probes §0.2 | behaviour, never ratified | `start` / `value` / `reversed` apply in every mode and at every cascade entry | B0 / B in effect |

### §1.9 What this memo does not touch

The existing conforming-attribute hints that are compat-gated today (the width / height family, N-6) stay where
they are; moving them is `IBP-layout`'s repair, under whatever the ratifier decides for P-1b. `dir` the attribute
(N-7) is a UA rule mis-homed as a hint and is routed in §7; `dir` the element is §4.3's.

---

## §2. Coupled invariants

The program holds these at once. Each slice's plan re-derives the pairs it touches.

- **I1 — single writer**: only the cascade's resolve step writes the computed `counter-*` fields (D-1, D-5; from
  LC-3 on — before it the bake is a second writer that only shrinks). What that writer **reads**: the cascade
  winners (UA rule, hints, author declarations), the parent's computed style for `inherit`, and — once a handler is
  registered (LC-6) — the handler's `get_computed`, `initial_value` and `is_inherited` for the CSS-wide keywords
  (D-10).
- **I2 — cascade position and inputs**: UA rule < hint < any author declaration, per property (D-6, D-7). The hint
  is a function of the element's tag, its namespace and its `start` / `reversed` / `value` / `type` attributes, so
  those attributes are **style inputs** (a namespace does not change after creation): today every attribute write
  bumps the tree version
  (`crates/core/elidex-ecs/src/dom/attribute.rs:124`, `:291`; `:227`), the event loop re-renders on a changed
  `inclusive_descendants_version` (`crates/shell/elidex-shell/src/content/event_loop.rs:440`–`:447`) and the restyle
  is whole-tree, so the dependency holds structurally. An incremental restyle must honour it: a change of one of
  these attributes restyles that element.
- **I3 — step order**: inherit → reset → increment (explicit, then the implicit one) → set → use (D-3).
- **I4 — scope**: a counter is visible to its creator's descendants and following siblings with their descendants;
  a new same-name counter replaces one from the same element or a previous sibling and nests inside one from an
  ancestor (D-3). Counter inheritance runs on the flattened element tree (CSS Lists 3 §4.4.1, its note).
- **I5 — implicit increment**: keyed on computed `display: list-item` and on the computed `counter-increment` not
  naming `list-item`; its sign comes from the innermost `list-item` counter (D-2). "Is a list item" is **one
  predicate with one definition**, read by the step, by the caller gate and by the §4.4.2 walk; LC-1 defines it
  over the tag `li` (the bake's key, moved) and LC-2k changes its body to `display` (§5.2 c5, c11). A fourth reader
  tests the same fact inline today: the marker reconcile (`generated_content.rs:97`–`:98`, `display ==
  Display::ListItem && list_style_type != ListStyleType::None`). It joins the predicate at LC-2k, not before: while
  the body is the tag, reading it would give a `display: block` `li` a marker (P8a). LC-2k replaces its `display`
  half by the predicate and keeps the `list-style-type` half, which is the marker's own fact (I5 × I11).
- **I6 — reversed initial value**: computed by the counter step over the boxes that exist, from the increments and
  sets in the counter's scope; never in the computed value; its storage shape is LC-1's under D-4's criterion (a
  component only if it crosses from the pre-layout pass to the paged walk or outlives the run). It
  depends on the set of elements in the scope and on their `display`, increments and sets, so a counter pass that
  is ever made incremental re-runs a scope when any of those changes (and, if LC-1 makes it a component, rewrites
  the component then).
- **I7 — boxless elements do nothing**: `display: none` and `display: contents` (CSS Lists 3 §4.5). "Is boxless" is
  **one predicate**, read by the caller gate of both drivers and by the §4.4.2 walk (I6 × I7).
- **I8 — engine modes**: the core UA sheet applies in every mode; the hint population per mode is P-1's outcome.
- **I9 — one step, two drivers**: the pre-layout pass and the paged walk (continuation suppression, `flatten`
  across pages, `set_counter` for `page` / `pages`) see the same step (D-9).
- **I10 — resolved value = computed value** for `counter-*` (D-10); it is the script-visible face of I1.
- **I11 — marker reconcile**: `ListItemMarker` is inserted or removed on every pass and on every exit path
  (`generated_content.rs`, the reconcile block before the `display: none` return). Unchanged.

**Pairs.** All 55 pairs were checked; the first table gives those that meet (32), the second the rest (23) with
the reason.
"By reading" marks a statement about today's code that no probe ran.

| Pair | Where they meet |
|---|---|
| I1 × I2 | an author `counter-reset: foo` on `ol[start]` removes both the hint's and the UA rule's `list-item` (Q3a); the bake's `already_has` guards approximated this per counter name and are deleted with it |
| I1 × I3 | today the bake compensates for the step's order: for `li[value]` it writes the set and **withholds** the increment (`counter.rs:348`–`:364`). With I1 the fields hold only what the cascade resolved, and I3 alone orders increment before set (P2a) |
| I1 × I5 | the implicit increment may never appear in a field: P1's `li` must read `counter-increment: none` |
| I1 × I6 | the reversed initial value may never appear in a field: `ol[reversed]` computes to `reversed(list-item)` with the absent-integer state, in the cascade and in `getComputedStyle` (§3.2, F7 note) |
| I1 × I7 | a boxless element still has its computed `counter-*` (the cascade writes them whatever `display` is; `getComputedStyle` shows them); only the step ignores them (§4.5: "still valid on such an element, but they must have no effect") |
| I1 × I9 | the paged driver clones the style and applies the bake today (`builder/walk.rs:134`); after D-1 it reads the component as written |
| I1 × I10 | the resolved value shows whatever writes the fields; D-10 lands after the bake is gone, or script would see the bake. The handler's `resolve` entry is not a second writer: it calls the cascade's implementation and no production path reaches it (§5.3 LC-6). The meeting runs both ways: the handler's `get_computed` output is also an **input** of the writer (`inherit`; `unset` / `initial` through `is_inherited` and `initial_value`), so it must re-enter `resolve_counter_reset` / `resolve_counter_property` in the shape they read — a value they do not recognise falls to `_ => {}` and resolves to the empty list (`box_model/mod.rs:538`, `:574`) (W-K1 – W-K3) |
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
| I5 × I11 | "is a list item" has one key for the marker and for the increment. Today the marker is keyed on `display` and the increment on the tag, so a `display: list-item` `div` shows a marker reading "0" (P8b). In W1 – W2 the two keys stay apart (the predicate's body is the tag; the marker keeps its inline `display` test); from LC-2k the marker reads the predicate (I5) |
| I6 × I7 | the §4.4.2 walk looks **ahead** of the element being stepped, at elements no driver has yet gated, so the caller gate does not reach it: the walk itself must skip the boxless set (`display: none`, `display: contents`, a `content: none` pseudo), through the same predicate the gate uses. Cell W-BX: `reversed(foo)`, three `p` incrementing `foo` by −1, the second `display: none` → 2, –, 1; an ungated walk gives 3, –, 2 (P4b is its `list-item` form) |
| I6 × I9 | both drivers need the same initial value for the same counter, from the whole scope and not the page being painted: one definition computes it — the step, in each driver's run, or the pre-layout pass once, as a component the paged walk reads (D-4; LC-1 picks) |
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
| I7 × I8, I8 × I9, I8 × I11, I9 × I10 | no shared input: modes act on the cascade, drivers on the walk |

---

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §2.1.8 Conformance classes | "may be designated … as supporting the suggested default rendering"; "must, while so designated, implement the rules the Rendering section defines" | designated / not; "possibly as a user option" | none — the reading input of OQ-1 (§1.2) | ✓ (the spec's three readings; DR-4 is IBP's applied reach) | no |
| WHATWG HTML §15.3.7 Lists | UA rule `ol, ul, menu { counter-reset: list-item; }` | three selectors; `dir` joins them by §16.3.3 (D-6) | `UA_CSS` in `ua.rs` (LC-3) | ✓ | no |
| WHATWG HTML §15.3.7 Lists | `ol[start]` / `[reversed]` → `counter-reset`, steps 1–4 | 3.1 reversed + integer (`reversed(list-item)` value + 1); 3.2 reversed, no integer; 4.1 integer (`list-item` value − 1); 4.2 no hint; parse error = no integer | the list hint source (NEW, LC-3) | ✓ | yes |
| WHATWG HTML §15.3.7 Lists | `li[value]` → `counter-set: list-item value` | parses / parse error (no hint) | the list hint source (NEW, LC-3) | ✓ | yes |
| WHATWG HTML §15.3.7 Lists | `type` hints; the non-counter UA rules; quirks rules; the non-CSS sentence | per unit, §4.3 | `ua.rs`, the hint sources (LC-5); units routed out or excluded per §4.3 | ✓ (§4.3) | yes (`type`) |
| WHATWG HTML §15.2 The CSS user agent style sheet and presentational hints | UA-level rules vs "author-level zero-specificity presentational hints" | rule / hint | `collect_and_cascade`'s hint source (unchanged seam) | ✓ | no |
| WHATWG HTML §2.3.4.1 Signed integers | rules for parsing integers, steps 1–9 | whitespace skip; `-`; `+`; non-digit error; trailing characters ignored | one shared parser (LC-3; SQ-1) | ✓ | yes |
| WHATWG HTML §4.4.5 The ol element | `reversed`, `start`, `type` are conforming content attributes | — | classification input to P-1 | ✗ (the *starting value* algorithm is not implemented as such: CSS UAs go through §15.3.7) | yes |
| WHATWG HTML §4.4.8 The li element | `value` is a conforming content attribute; *list owner*, *ordinal value* | — | classification input to P-1; no reader of ordinal values at this base (§7) | ✗ | yes |
| WHATWG HTML §16.1 Obsolete but conforming features | "Authors should not specify … CSS should be used instead" | the features with a §15 mapping (`border` on `img`) | classification input to P-1b: compat | ✗ (no list attribute is in §16.1) | yes |
| WHATWG HTML §16.2 Non-conforming features | obsolete attributes: `type on li elements`, `type on ul elements`; the `dir` element | — | classification input to P-1 (LC-5) | ✗ (the list rows only) | yes |
| WHATWG HTML §16.3.3 Other elements, attributes and APIs | "User agents must treat dir elements in a manner equivalent to ul elements in terms of semantics and for purposes of rendering" | the `dir` sentence | §4.3: `dir` gets every rule `ul` has, in `ul`'s sheet and in every mode — its `counter-reset` in LC-3 (D-6, U7), the other non-`display` units in LC-5; `display` is `IBP-ua-display`'s (§7 F-12, with the section's sibling sentences) | ✗ (that sentence only) | no |
| WHATWG HTML §15.5.5 The details and summary elements | `details > summary:first-of-type { display: list-item; counter-increment: list-item 0; … }` and, in `details`' UA shadow tree, `:host summary { display: list-item; counter-increment: list-item 0; … }` | document form / shadow form (the default summary) | none in this program: the document form's `display` is the member of IBP's unregistered `#11-ua-sheet-html-rendering-display-fidelity`, the shadow form is CV-details'; the `counter-increment` half is in no plan; ordered against LC-2k in §5.2 (cross-lane) and §7 F-15 | ✗ (those two rules only) | no |
| CSS Lists 3 §4 Automatic Numbering With Counters | steps 1–5; the clamp to the UA's value range | the order; out-of-range reset / set / increment | `CounterState::process_element` (LC-1); the hint's integers (LC-3) (§3.1) | ✓ | yes |
| CSS Lists 3 §4.1 Creating Counters: the counter-reset property | value grammar; *Computed value* line | `none`; name; name + integer; `reversed()`; `reversed()` + integer; several entries (the last of a name is honoured) | `parse_counter_list`, `resolve_counter_reset`, `CounterResetEntry` (LC-1) | ✓ | yes |
| CSS Lists 3 §4.2 Manipulating Counter Values: the counter-increment and counter-set properties | instantiate-if-absent; several entries of one name; the *Computed value* lines ("the keyword none or a list, each item an identifier paired with an integer") and the *Initial* (`none`) / *Inherited* (`no`) lines | increments compound; last set wins; `none` / list | `CounterState::process_element` (LC-1); the resolve and the handler's `get_computed`, `initial_value`, `is_inherited` (LC-6) | ✓ | yes |
| CSS Lists 3 §4.3 Nested Counters and Scope | scope | descendants; following siblings; obscured by a later sibling's reset | the scope model (LC-4) | ✓ | yes |
| CSS Lists 3 §4.4.1 Inheriting Counters | steps 1–5 | root; counter source = preceding sibling or parent; value source = preceding element in tree order; the flattened element tree | the scope model (LC-4) | ✓ | no |
| CSS Lists 3 §4.4.2 Instantiating Counters | instantiate steps 1–3; the initial-value algorithm steps 1–5 | replace (same element / previous sibling); nest (ancestor); loop break at a set; zero increments | LC-4 (instantiate); LC-1 (initial value) | ✓ | yes |
| CSS Lists 3 §4.5 Counters in elements that do not generate boxes | boxless elements | `display: none`; `content: none` pseudo; "replaced element's descendants … undefined" | one boxless predicate, read by the caller gate of both drivers and by the §4.4.2 walk (LC-1; I6 × I7; the pseudo and replaced branches are its plan's to audit) | ✗ | yes |
| CSS Lists 3 §4.6 The Implicit list-item Counter | implicit increment; the "should" on UA sheets and hint mappings | list item; reversed (−1); `counter-increment` names `list-item`; "does not affect the specified or computed values" | LC-1 (the increment in the step), LC-2k (its key); P-1a (§1) | ✓ | yes |
| CSS Lists 3 §4.7 Outputting Counters: the counter() and counters() functions | read | unchanged | `evaluate_counter`, `evaluate_counters` | ✗ (not touched) | no |
| CSS Cascade 5 §4.4 Computed Values | "the result of resolving the specified value as defined in the "Computed Value" line" (normative); the two Notes on implied values and on not laying out the document | — | one side of the conflict over `reversed()` (§3.2, F7 note; LC-1, LC-6) | ✗ (that definition only) | no |
| CSS Cascade 5 §6.5 Precedence of Non-CSS Presentational Hints | origin of hints | UA-origin or the author presentational hint origin; "a document language may define" which; `revert` | the hint seam (unchanged; §7 F-3) | ✗ | no |
| CSSOM 1 §9 Resolved Values | default case: the resolved value is the computed value | `counter-*` are not in the special-case list | LC-6 | ✓ | no |

**Breadth**: K = 4 specs (HTML, CSS Lists 3, CSS Cascade 5, CSSOM 1), M = 25 (the rows of the table above) → past the
split-recommended threshold. **Split decision**: split — this memo is the umbrella, and §5's slices each carry a
subset (no slice exceeds three specs; LC-6 spans three: CSSOM 1 §9, CSS Lists 3 §4.1 / §4.2, CSS Cascade 5 §4.4; LC-3 touches
Cascade 5 §6.5 only as the unchanged seam).

Section numbers and titles above were each looked up: `webref heading html {2.1.8, 15.2, 15.3.7, 15.5.5, 2.3.4.1,
4.4.5, 4.4.8, 16.1, 16.2, 16.3.3}`; `webref heading css-lists-3 {4.1, 4.2, 4.3, 4.4.1, 4.4.2, 4.5, 4.6, 4.7}` and `webref
body css-lists-3 auto-numbering` (§4); `webref heading css-cascade-5 {4.4, 4.5, 6.5}`; `webref heading cssom-1 9`. The
step numbers are those `webref body` prints: `html lists` (1–4, 3.1–3.2, 4.1–4.2), `html signed-integers` (1–9),
`css-lists-3 inheriting-counters` (1–5), `css-lists-3 instantiating-counters` (1–3 and 1–5). `preflight.py` verifies
the HTML rows only (its label table has no CSS-module labels, `#11-preflight-css-module-labels`).

### §3.1 User-input touch audit

- Author-controlled inputs: the `start`, `reversed`, `value`, `type` attributes and the `counter-*` declarations
  (stylesheets and `style=""`). All reach integers; values are `i32`. **Arithmetic is not uniformly saturating
  today**: the step adds with `saturating_add` (`counter.rs:118`), but negates an increment on a reversed counter
  unchecked (`-(*value)`, `:117`); the bake's `start + 1` saturates (`:318`) and its `start − 1` does not (`s - 1`,
  `:327`). By reading, not run: `counter-increment: c -2147483648` on a reversed counter, and
  `<ol start="-2147483648">`, overflow — a panic where overflow checks are on, a wrapped value where they are off.
  CSS Lists 3 §4 fixes the behaviour: "If a counter reset, set, or increment would push the value outside of that
  range, the value must be clamped to that range." Obligations, per operation:
  - LC-1: the step's increment and set clamp; so does each operation of §4.4.2's algorithm (the `× −1` of step 3.1
    and the three additions); the unchecked negation at `:117` goes with the sign change.
  - LC-3: the hint's `start − 1` and `start + 1` clamp. The integer parser: HTML's rules for parsing integers put no
    bound on the digit run, so a value outside `i32` is not a parse error and "no hint" would be wrong; it clamps
    where the integer enters (the same §4 rule). Cells W-OV (§6).
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
  (counter()/counters())."; "UAs may have implementation-specific limits on the maximum or minimum value of a
  counter. If a counter reset, set, or increment would push the value outside of that range, the value must be
  clamped to that range."
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
- `# source: https://drafts.csswg.org/css-cascade-5/#computed` — normative: "The computed value is the result of
  resolving the specified value as defined in the "Computed Value" line of the property definition table, generally
  absolutizing it in preparation for inheritance." Two **Notes** of the same section (non-normative): "Note: … the
  computed value is an abstract data representation … For example, serialization rules often allow omitting certain
  values which are implied during parsing; but those values are nonetheless part of the computed value."; "Note: In
  general, the computed value resolves the specified value as far as possible without laying out the document or
  performing other expensive or hard-to-parallelize operations, such as resolving network requests or retrieving
  values other than from the element and its parent."
- `# source: https://drafts.csswg.org/css-cascade-5/#used` — "The used value is the result of taking the computed
  value and completing any remaining calculations to make it the absolute theoretical value used in the formatting
  of the document."
- `# source: https://drafts.csswg.org/css-cascade-5/#preshint` — quoted in §1.1; also "For the purpose of cascading
  this author presentational hint origin is treated as an independent origin; however for the purpose of the revert
  keyword (but not for the revert-layer keyword) it is considered part of the author origin."; and its second
  paragraph: "A document language may define whether such a presentational hint enters the cascade as UA-origin or
  author-origin; if so, the UA must behave accordingly. For example, [SVG11] maps its presentation attributes into
  the author origin."
- `# source: https://html.spec.whatwg.org/multipage/rendering.html#the-details-and-summary-elements` — "details >
  summary:first-of-type { display: list-item; counter-increment: list-item 0; list-style: disclosure-closed inside;
  }"; and, for `details`' UA shadow tree, "The third child element is either a link or style element with the
  following styles for the default summary: :host summary { display: list-item; counter-increment: list-item 0;
  list-style: disclosure-closed inside; }"
- `# source: https://html.spec.whatwg.org/multipage/common-microsyntaxes.html#signed-integers` — "4. Skip ASCII
  whitespace within input given position. … 8. Collect a sequence of code points that are ASCII digits from input
  given position, and interpret the resulting sequence as a base-ten integer."

**F7 note — the computed value of `reversed(name)` without an integer.** The texts conflict, and rung 1 does not
decide between them. On one side: §4.1's *Computed value* line reads "each item an identifier or a reversed()
function **paired with an integer**", and Cascade 5 §4.4's normative sentence defines the computed value as what
that line says. On the other: §4.1's value prose gives a reversed counter "no starting value if not given", and
§4.4.2 says the UA "must dynamically calculate the initial value at layout-time". The sentence rev 2 leaned on —
omitted values "are nonetheless part of the computed value" — is a **Note** of Cascade 5 §4.4, and so is that
section's remark that a computed value is resolved without "retrieving values other than from the element and its
parent"; they describe the tension and settle nothing. Cascade 5 §4.5's definition of the used value (quoted
above: "completing any remaining calculations to make it the absolute theoretical value used in the formatting of
the document") describes what §4.4.2's layout-time value is like, but §4.5 names no property, so it does not decide
the conflict either; it is quoted for the comparison only. For a plain name there is no tension (the omitted integer
defaults to 0). For a reversed name the line's letter asks for an integer that only §4.4.2's layout-time walk over
other elements can supply. **Decided at rung 2**: the computed value of `reversed(name)` is the `reversed()` item
with no integer; of `reversed(name) 5`, the item paired with 5. Ground: *ECS-native first* — `ComputedStyle` has
one writer, the cascade system, which reads the element and its parent; a value derived from following siblings'
components is the counter system's output (D-1, D-4, §0.5 row 2), and writing it back would give the style
component a second writer that depends on a later phase (*Concurrency by ownership and phases*). The summary
line's wording is a candidate spec issue (§7 F-13 (a); not filed by us). LC-1 (the representation) and LC-6 (the
serialization, and its round trip through `inherit`) carry the decision and re-check the texts at their base.

### §3.3 Docstring citations for the new functions

Every function a slice adds or rewrites cites, in its doc comment, the spec step it implements; each slice's plan
carries this table for its own functions, with the names it settles on and each citation looked up with `webref`
at its base. Names below are placeholders.

| Function (slice) | Its doc cites |
|---|---|
| the counter step (LC-1) | CSS Lists 3 §4 Automatic Numbering With Counters, steps 2–4, in that order; §4.2 for instantiate-if-absent |
| the implicit increment (LC-1) | CSS Lists 3 §4.6 The Implicit list-item Counter — "incremented by 1 on every list item, or if the counter is reversed … by -1" and "unless the counter-increment property explicitly specifies" |
| the list-item predicate (LC-1; body changed by LC-2k) | CSS Lists 3 §4.6, "on every list item". While its body reads the tag `li` (LC-1 – LC-2), its doc names that as a defect against the spec's key (`display: list-item`) and LC-2k as the repair |
| the reversed initial value (LC-1) | CSS Lists 3 §4.4.2 Instantiating Counters, the initial-value algorithm steps 1–5 |
| the boxless predicate, read by the caller gate and by the §4.4.2 walk (LC-1) | CSS Lists 3 §4.5 Counters in elements that do not generate boxes |
| the `counter-reset` parse and resolve, the absent-integer state (LC-1) | CSS Lists 3 §4.1 Creating Counters: the counter-reset property, the `<reversed-counter-name> <integer>?` value definition |
| the list hint source: `ol` (LC-3) | HTML §15.3.7 Lists, the `start` / `reversed` steps 1–4 |
| the list hint source: `li` (LC-3) | HTML §15.3.7 Lists, the `value` mapping |
| the integer parser (LC-3) | HTML §2.3.4.1 Signed integers, the rules for parsing integers, steps 1–9 |
| the UA rule's comment in `UA_CSS` (LC-3) | HTML §15.3.7 Lists; CSS Lists 3 §4.6 (the "should"); HTML §16.3.3 for `dir` in the selector list |
| inherit counters (LC-4) | CSS Lists 3 §4.4.1 Inheriting Counters, steps 1–5 |
| instantiate a counter (LC-4) | CSS Lists 3 §4.4.2 Instantiating Counters, steps 1–3; §4.3 Nested Counters and Scope |
| the `type` hint source and the added UA rules (LC-5) | HTML §15.3.7 Lists, each rule as §4.3 numbers it; §16.3.3 for `dir` |
| the handler's `get_computed` (LC-6) | CSSOM 1 §9 Resolved Values; CSS Lists 3 §4.1 / §4.2, the *Computed value* lines |
| the handler's `initial_value` (LC-6) | CSS Lists 3 §4.1 / §4.2, the *Initial* lines (`none` for all three properties) |
| the handler's `is_inherited` (LC-6) | CSS Lists 3 §4.1 / §4.2, the *Inherited* lines (`no` for all three properties) |

---

## §4. The end state

### §4.1 Facts and their homes

| Fact | Home at `bf1cae35` | Home at the end | Slice |
|---|---|---|---|
| `ol` / `ul` / `menu` / `dir` reset `list-item` | the bake's `"ol"` / `"ul"` arms (`counter.rs:306`, `:334`); `menu` and `dir` nowhere | a UA rule in `UA_CSS` (`ol, ul, menu, dir`) | LC-3 |
| `start` / `reversed` → `counter-reset`; `value` → `counter-set` | the bake (`counter.rs:306`–`:351`), after the cascade, for any element with that tag name whatever its namespace | hint declarations, a pure function of `TagType` + `Namespace` + `Attributes`, entering at the hint seam | LC-3 |
| The implicit increment | a baked `("list-item", 1)` in the computed field (`counter.rs:364`), keyed on the tag `li`; the pre-layout pass gated by tag (`generated_content.rs:124`, `:141`) | the counter step, keyed on computed `display` | LC-1 (into the step, key unchanged), LC-2k (the key) |
| The implicit increment's sign | `process_element` negates **every** increment of a reversed counter (`counter.rs:117`) | the step applies −1 for the implicit one only | LC-1 |
| A reversed counter's initial value with no integer | a baked `li_count + 1` (`counter.rs:312`–`:320`); `0` for an author `reversed(foo)` (`unwrap_or(0)`, `box_model/mod.rs:529`; the parser already stores the default as a number) | absent in the parsed and computed value; computed by the counter step per §4.4.2; held where D-4's criterion puts it (a component, or the counter instance — LC-1's) | LC-1 |
| Step order | reset → set → increment (`counter.rs:84`–`:128`) | reset → increment → set | LC-1 |
| Author `counter-*` on an element | resolved only when the element declares `content` (`box_model/mod.rs:416`) | resolved for every element | LC-2 |
| Counter scope | the creator's subtree (`push_scope` before the step, `pop_scope` after the children) | §4.3 / §4.4: subtree and following siblings; replace / nest on instantiate | LC-4 |
| Resolved value of `counter-*` | none: `get_computed_with_registry` falls to `get_initial_value` (`resolve/mod.rs:92`) | a registered handler's `get_computed` | LC-6 |
| `ol[type]`, `ul` / `menu` / `dir` marker styles, nested styles and margins, `menu` box rules | absent (N-5) | UA rules and hints per §4.3 | LC-5 |

### §4.2 The counter step (input to LC-1 and LC-4; mechanism is theirs)

One step per box-generating element or pseudo-element, in tree order, taking the element's `ComputedStyle` and
`is_continuation`: (1) inherit per §4.4.1; (2) each `counter-reset` entry instantiates per §4.4.2 (initial value:
the entry's integer, else for a reversed entry the §4.4.2 value); (3) each `counter-increment` entry increments,
instantiating first if absent; then, if the element is a list item (I5's one predicate: computed `display` at the
end state) and no entry named `list-item`, the implicit ±1; skipped for a continuation as today; (4) each
`counter-set` entry sets, instantiating first if absent; (5) readers
(`evaluate_counter`, `evaluate_counters`) are unchanged. The step is the only place any of this lives; the drivers
only decide which entities get a step (I7, one gate for both) and in what order (I9). The §4.4.2 value needs the
elements of the counter's scope ahead of the current one, so its input is the tree, not only the current style,
and it applies the boxless predicate itself to the elements it visits (I6 × I7); how the step receives the value,
and where the value is held, are LC-1's (D-4's criterion).

### §4.3 HTML §15.3.7, unit by unit (D-11)

**`dir`, once for every row below.** `dir` is an obsolete element (§16.2), and HTML §16.3.3 says "User agents must
treat dir elements in a manner equivalent to ul elements in terms of semantics and for purposes of rendering". That
is a plain "must" on user agents, in a section outside §15: **rung 1, and independent of the designation reading**
(rev 2 tied the sheet to the reading; plan-review R2, A4-I3, showed that under DR-2 / DR-3 it would have given `dir`
a compat-only reset beside `ul`'s core one, breaking the sentence). So: (1) `dir` gets **all** of `ul`'s rendering —
every rule §15.3.7 lists it in, and also `ul`'s `counter-reset` (U7), whose selector omits `dir`: a tension inside
HTML that §16.3.3's "must" decides (candidate spec issue, §7 F-13); never a subset. (2) **In the same sheet and
the same modes as `ul`**: each `dir` rule lands beside the `ul` rule it mirrors, which for every unit below is the
core UA sheet, applied in every engine mode. #408's criterion would put an obsolete element's rules in the compat
sheet; that criterion is rung 2 and yields here to rung 1, as a second exception beside `tt`. At `bf1cae35`
neither sheet has a `dir` rule (`/usr/bin/grep -n -w dir crates/css/elidex-style/src/ua.rs crates/dom/elidex-dom-compat/src/legacy_ua.rs` → only `bdo[dir=…]`, `ua.rs:106`–`:107`),
so nothing regresses. LC-3 lands `dir`'s `counter-reset` (it must precede LC-4, §5.2 c10); LC-5 the other
non-`display` units; `display` is `IBP-ua-display`'s, with the same rung-1 input (§7 F-12).

| # | Unit (as `webref body html lists` gives it) | At `bf1cae35` | Disposition |
|---|---|---|---|
| U1 | `dir, dd, dl, dt, menu, ol, ul { display: block; }` | `ul, ol, dl, dt, dd` present (`ua.rs:16`); `menu` and `dir` compute to `inline` (R3, W-H5) | `display` rules are `IBP-ua-display`'s population (IBP §4, Appendix G); this memo hands it `menu` (§7 F-5) and, for `dir`, §16.3.3 (§7 F-12: IBP's Appendix G seeds `dir` as a compat-sheet rule, `:1955`, where `ul`'s `display: block` is in the core sheet) |
| U2 | `li { display: list-item; text-align: match-parent; }` | `display` present (`ua.rs:59`); `match-parent` has no hit in `crates` (`git grep -n -F -e 'match-parent' -e 'MatchParent' -- crates` → 0; control `"text-align"` → 32 lines) | `text-align: match-parent` needs a value the text module does not have → routed out (§7 F-6) |
| U3 | `dir, dl, menu, ol, ul { margin-block: 1em; }` | `ul, ol`, `dl` present as physical `margin-top` / `-bottom` (`ua.rs:53`–`:57`, `:63`); `menu`, `dir` absent | `menu`: LC-5; `dir`: LC-5, per the paragraph above. The physical longhands where the spec writes flow-relative ones: routed finding §7 F-9 |
| U4 | `:is(dir, dl, menu, ol, ul) :is(dir, dl, menu, ol, ul) { margin-block: 0; }` | absent (R4: 16px) | LC-5 (`dir` as above) |
| U5 | `dd { margin-inline-start: 40px; }` | present as physical `margin-left` (`ua.rs:64`) | not this program's to rewrite: routed finding §7 F-9 |
| U6 | `dir, menu, ol, ul { padding-inline-start: 40px; }` | `ul, ol` present as physical `padding-left`; `menu`, `dir` absent (R3) | `menu`: LC-5; `dir`: LC-5 as above; physical vs flow-relative: §7 F-9 |
| U7 | `ol, ul, menu { counter-reset: list-item; }` | absent; the bake stands in for `ol`, `ul` | LC-3, in the core sheet; `dir` in the same rule, by §16.3.3 (D-6) |
| U8 | `ol { list-style-type: decimal; }` | present (`ua.rs:61`) | none |
| U9 | `dir, menu, ul { list-style-type: disc; }` | absent: a `ul` inside an `ol` inherits `decimal` (R1) | LC-5 (`dir` as above) |
| U10 | `:is(dir, menu, ol, ul) :is(dir, menu, ul) { list-style-type: circle; }` | absent (P18, R1) | LC-5 (`dir` as above) |
| U11 | three-deep → `square` | absent | LC-5 (`dir` as above) |
| U12 | `type` hints: `ol[type=…]`, `li[type=…]` (five marker styles); `ul[type=…]`, `li[type=…]` (four) | absent (P17) | LC-5, placed by OQ-1's outcome: under B `ol[type]` is core and `li[type]`, `ul[type]` (§16.2) go to the compat generator; under A and B0 all are compat; under C all are core. **`dir[type]`** gets `ul[type]`'s four hints, in `ul[type]`'s layer under each option (the paragraph above: all of `ul`'s rendering; rung 1, HTML §16.3.3); the spec's selectors omit `dir`, the same tension as U7 (§7 F-13 (b)). The `ol[type]` keyword set is shared with the reflection's existing table, not copied (§5.3 LC-5) |
| U13 | "In quirks mode, the following rules are also expected to apply" (three `list-style-position` rules) | absent | excluded while elidex has no quirks mode (IBP §0.4's ground). Not slot-less: the document's mode is unstored in slot `#11-document-mode` (owner Layout, PM 576 in `PORTFOLIO-archive-2026-W45.md`; ledger `project_open-defer-slots.md:1476`), whose trigger includes quirks-mode layout; these three rules are an input line for it (OQ-4), compat under every option (they are the quirks rules DR-4 carves) |
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
| **LC-0** | Touch-time split: `counter.rs` is 799 lines, 386 of them the `tests` module (`:414`–). Move the tests to `counter/tests.rs`. Standalone PR, no behaviour | — | — | nothing |
| **LC-1** | The per-element step: §4's order (reset → increment → set); the implicit increment moved into the step **under the bake's key, the tag `li`**, read through one list-item predicate; explicit increments no longer negated; the absent-integer state through parser, resolve and `CounterResetEntry`; §4.4.2's initial value in the step; one boxless predicate for both drivers' gate and for the §4.4.2 walk. The bake loses its `li` increment arm with that arm's guard and `count_li_children` (its `ol[reversed]` arm passes the absent integer); the pre-layout gate's `is_list_tag` is replaced by the predicate | D-2 (the step), D-3 (order), D-4, D-9 | P1 (field), P4a (field), P4b, Q4c, Q4e, Q4f, Q4g, Q6, Q6b, Q13, Q20's and Q21's fields, W-O1, W-RV, W-BX; W-ML stays right | LC-0 |
| **LC-2** | Author `counter-*` resolve for every element (N-1) | D-5 | P6, P4e, P19, P20, Q23; P4c, W-F2, W-F4 stay right | LC-1 |
| **LC-2k** | The list-item predicate's body changes from the tag `li` to computed `display: list-item`: the increment, the caller gate and the §4.4.2 walk change key together, and the marker reconcile's inline `display` test becomes a read of the predicate (I5). **Conditionally**, one UA-sheet edit: if a form of §15.5.5's `summary` rule carrying `display: list-item` without `counter-increment: list-item 0` is in a UA sheet at LC-2k's base, LC-2k adds that declaration to the rule (§7 F-15 (b)); for the shadow form that sheet is CV-details', another program's file (OQ-6) | D-2 (the key) | P8a; P8b reaches 1, 1; W-F3, W-F4, W-SUM, W-SUM-D stay right | LC-2 (c11) |
| **LC-3** | The UA rule (`ol, ul, menu, dir`); the list hints at the hint seam with HTML integer parsing; the bake, and all three call sites, deleted | D-1, D-6, D-7, D-8 | P2b / Q2b, P3a / Q3a, P5, P14, W-N1 / W-N2, W-NS, W-OV, PM 637 | LC-2 (c6), LC-1 (c7); LC-2k by choice (§5.2); **OQ-1 (P-1)**, OQ-2 |
| **LC-4** | The scope model: §4.3 / §4.4.1 inheritance and §4.4.2's instantiate (replace / nest). Both drivers, `flatten` and `set_counter` audited against it | D-3 (scope), D-9 | P8b, P11a / Q11a, P11b, W-S1, W-S2, W-RS | LC-3 |
| **LC-5** | HTML §15.3.7's remaining units (§4.3: U3, U4, U6, U9 – U12) | D-11 | P17, P18, R1, R3 (margins / padding), R4 | LC-3 for the hint source; **OQ-1** for U12's placement |
| **LC-6** | `counter-*` resolved values through the property-handler seam; with it, `inherit` on `counter-*` | D-10 | P10 / Q10, W-K1, W-K2; W-K3 stays right | LC-3 |

Order: LC-0 → LC-1 → LC-2 → LC-2k → LC-3 → { LC-4, LC-5, LC-6 } (the last three are independent of one another).
LC-1, LC-2 and LC-2k depend on no open question and can start as soon as this memo is plan-reviewed. Against rev
2: the key change tag → `display` leaves LC-1 and becomes LC-2k, after N-1's removal (c11); the slice names LC-3 –
LC-6 are unchanged. (Against rev 1: rev 1's LC-1 (order + scope) is split between LC-1 and LC-4; rev 1's LC-2 and
the reversed half of its LC-3 are in LC-1; rev 1's LC-3 keeps N-1 only and is LC-2; rev 1's LC-4 is LC-3.)

### §5.2 Why this order: no spec-correct observable turns wrong on the way

**Method.** Two populations of units, both checked in every window against every input that can reach them.
(1) **The units that exist at `bf1cae35`**: every guard, count and arm of the bake, every defect of the step and of
the resolve path, and both drivers' gates. (2) **The units each slice adds**: every new read a slice introduces —
a key, a suppression test, a look-ahead walk, a rule, a hint source, a handler entry — because a new read can meet
a defect that an earlier slice has not yet removed (rev 2 listed population (1) only, and missed c11). Every
slice's newly opened author-visible input is listed. A hazard is a cell that is right at some point of the program
and wrong in a later window. Each hazard found is a **constraint** on the order ("X lands no later than Y").

**Constraints, each with the cell that proves it.**

| # | Constraint | The cell that turns wrong otherwise | Found |
|---|---|---|---|
| c1 | the end of the negation ≼ N-1's removal | P4c: 3, 2, 1 → 5, 6, 7 (the author's explicit −1 becomes visible and is negated) | rev 1 |
| c2 | N-1's removal ≼ sibling scope | W-H4, W-H4b: b = 1 → 2 / 3 — under sibling scope the `ol`'s counter reaches the later `div`, whose author `counter-reset` / `counter-set` is still dropped | rev 2 |
| c3 | `menu`'s reset ≼ sibling scope | W-H3, W-H3b: b = 1 → 2 — the previous sibling's counter reaches `menu`'s items while `menu` has no reset | rev 2 |
| c4 | the bake's `li` guard on the computed increment goes ≼ N-1's removal | W-F2: 1, 5, 6 → 1, 3, 4 — the guard (`counter.rs:343`–`:347`) wraps the `value` → set push as well as the increment, so a visible author `counter-increment: list-item 2` drops the hint (R1 finding F2) | R1 |
| c5 | §4.4.2's count is in the step ≼ the list-item key changes from tag to `display`; and the count and the increment read **one** predicate | W-F3: y = 1 → 2 — `count_li_children` counts `li` tags whatever their `display` (R1 finding F3). It cannot be re-keyed in place: the bake runs for the `ol` in the cascade walk before its children are styled (`crates/css/elidex-style/src/walk.rs:168`), so the count moves to §4.4.2 in the step, where it shares the increment's predicate | R1 |
| c6 | N-1's removal ≼ the UA rule and the hints | a `counter-reset` on an `ol` that declares no `content` would be dropped by `resolve_content` (`box_model/mod.rs:416`): every list would lose its reset | rev 1 (implicit) |
| c7 | the absent integer and §4.4.2 ≼ the `reversed` hint | step 3.2's `reversed(list-item)` has no integer to give | rev 1 |
| c8 | the order fix ≼ the implicit increment's move into the step | P2a: 1, 5, 6 → 1, 6, 7 — under set → increment the step would add the implicit 1 after `li[value]`'s set (today the bake withholds the increment instead) | rev 2 |
| c9 | the implicit increment's move into the step ≼ the end of the `li` arm's increment guard and push ≼ the sign change | the guard and the push gone before the move: P1 1, 2, 3 → 0, 0, 0 (nothing increments); the guard gone, the push kept: Q19 2, 4 → 3, 6 (the bake pushes its `list-item 1` beside the author's `list-item 2`). The sign change before the push is gone: P4a 3, 2, 1 → 5, 6, 7 (the bake's `+1` is an explicit increment, now applied as written). While the push stands it names `list-item` in the `li`'s computed increment, so D-2's suppression test withholds the moved increment whatever its sign: the move is free of the sign change. The end of the push may land before the sign change only if the moved increment is `+1` and today's negation (`counter.rs:117`) flips it; if LC-1's mechanism gives the moved increment its own sign and that increment still passes through today's negation, **the end of the guard and push and the sign change co-land** (P4a → 5, 6, 7 otherwise: from the push's end the moved −1 is the item's only increment, and it is negated) | rev 1; the guard half rev 3; restated as an order rev 3.1 (R3 B3-M1); co-landing pair corrected rev 3.2 (C-M1) |
| c10 | `dir`'s `counter-reset` (LC-3, D-6) ≼ sibling scope | W-H5: b = 1 → 2 — as c3, for `dir` | rev 2 |
| c11 | N-1's removal ≼ the list-item key's change to `display` | W-F4: 1, 1, 2 → 1, 2, 3 — a non-`li` element with `display: list-item` becomes a list item while its author `counter-increment: list-item 0` is still dropped (it declares no `content`), so the step's suppression test reads nothing and adds the implicit 1. (It returns to 1, 1, 2 when N-1 goes: a cell wrong only in the window between) | R2 finding A3-I1 |
| c12 | the sign change ≼ §4.4.2's value | P4a: 3, 2, 1 → −5, −6, −7 — §4.4.2 reads each element's increment for the counter; while the implicit increment is a baked `list-item 1` whose sign the step fixes up, the walk reads +1 where the step applies −1 | rev 3 |
| c13 | §4.4.2's value in the step ≼ the bake's `ol[reversed]` arm passing the absent integer | P4a: 3, 2, 1 → −1, −2, −3 — an absent integer with no §4.4.2 value instantiates at 0 — 0 is how an absent integer is encoded today (`resolve_counter_reset`'s `unwrap_or(0)`, `box_model/mod.rs:529`, for an author `reversed(name)` with no integer), and a step that has neither an integer nor a §4.4.2 value has only that to read — then each item applies −1 | rev 3.1 (R3 B3-M2) |

**What the constraints force** (R2 finding A3-M6 asked for this explicitly).

- *Co-landing* — none is forced by a cell. c9 is an order: rev 3 wrote it as one change of three parts, which its
  witness (Q19) did not prove (R3 B3-M1). The end of the guard and push and the sign change co-land only under the
  mechanism c9 names (rev 3.1 named the move: while the push stands the moved increment is suppressed, so the move
  never meets the sign change; C-M1).
- *Order*: order fix ≼ move ≼ the end of the guard and push ≼ sign change ≼ §4.4.2 ≼ the `ol[reversed]` arm's
  absent integer (c8, c9, c12, c13); §4.4.2 ≼ the key change (c5); the end of the guard and push, and the sign
  change, ≼ N-1's removal ≼ the key change (c4, c1, c11); N-1's removal and §4.4.2 ≼ the cascade inputs (c6, c7);
  N-1's removal, `menu`'s and `dir`'s resets ≼ sibling scope (c2, c3, c10).
- *The consequence rev 2 missed*: the implicit increment must be in the step **before** N-1 goes (c9 with c1, c4) and
  may take the `display` key only **after** (c11). Between the two it lives in the step under the tag key.

Three ways to satisfy that were weighed, under the rule "no spec-correct observable turns wrong in any window, and
the bake never becomes a second writer of anything new":

- **(α) the key change is its own slice, LC-2k, after LC-2 — chosen.** The interim tag key adds no behaviour: it
  is the bake's key, moved together with the arm it keyed, and read through the one predicate whose body LC-2k then
  changes. c5 holds by construction (the count and the increment share the predicate).
- **(β) one slice for LC-1, LC-2 and the key change**, so that c1, c4 and c11 hold by equality — rejected. It
  satisfies the constraints only as a single change; it joins the step rewrite to N-1's removal, which opens the
  program's largest new input (author `counter-*` on every element, §3.1); and its own plan would have to divide it
  along (α)'s lines, at nested-PR granularity, to be reviewable.
- **(γ) N-1's removal first** — impossible: c1 and c4 (P4c and W-F2 turn wrong).

**Must LC-1 divide?** No. What the constraints force is that the key change is **not in** LC-1. LC-1's remaining
changes may land as one PR; if its plan divides it, the nested order is: the order fix (c8) → the move → the end
of the guard and push → the sign change (c9; the end of the guard and push and the sign change in one nested PR if
the moved increment carries its own sign and passes through today's negation) → §4.4.2 (c12) → the `ol[reversed]` arm's absent integer (c13). Adjacent steps may share a
nested PR. The boxless predicate goes with the first nested PR that reads it.

c6 puts LC-2 before the cascade inputs (LC-3). c2, c3 and c10 put the scope model after both — LC-4 — where
`menu`'s and `dir`'s resets are the cascade's and no author reset is hidden. No constraint orders LC-2k against
LC-3 – LC-6, nor LC-4, LC-5 and LC-6 among themselves. **LC-2k is placed before LC-3 by choice, not by a
constraint**: it is unblocked while LC-3 waits on OQ-1, and a fixed position lets every later window be checked
against the final key only.

**Cross-lane: HTML §15.5.5's `summary` rule, in its two forms.** HTML §15.5.5 gives `display: list-item` and
`counter-increment: list-item 0` in **one rule**, for the document (`details > summary:first-of-type { … }`) and for
the default summary in `details`' UA shadow tree (`:host summary { … }`) (§3.2). Neither form is in a plan whole:
IBP's slot `#11-ua-sheet-html-rendering-display-fidelity` holds the document form's `display` declaration alone
(IBP `:1964`), and CV-details builds "only §15.5.5's shadow `:host summary` (the default summary's display)"
(`docs/plans/2026-10-content-visibility.md:880`); §7 F-15 has the facts. Each form does the two things c11 is
about: it makes a non-`li` element a list item, and it relies on a `counter-*` declaration on an element that
declares no `content`. Today `summary` computes `display: block` (`ua.rs:19`), no `details` UA shadow tree is built
(CV `:96`, B7), and the only `display: list-item` rule in either sheet is `li`'s (`/usr/bin/grep -n 'list-item'
crates/css/elidex-style/src/ua.rs crates/dom/elidex-dom-compat/src/legacy_ua.rs` → `ua.rs:59` and its test, `:289`,
`:292`, at `7fcaffed`). Cells: W-SUM, `<ol><li><details><summary>s</summary></details><li>b</ol>`, spec 1, 2 (the
`summary` adds 0); W-SUM-D, `<ol><li><details>x</details><li>b</ol>`, spec 1, 2 (the default summary adds 0; it is
in the flattened element tree, so it takes the `ol`'s counter, I4 × I7).

| The rule (either form) lands… | W-SUM / W-SUM-D, rule whole | the same, `display` alone |
|---|---|---|
| before LC-2 | stays 1, 2 — `summary` is not an `li`, so the tag-keyed predicate does not increment it; N-1 drops its `counter-increment`, harmlessly | stays 1, 2, until LC-2k |
| between LC-2 and LC-2k | stays 1, 2 — tag key; the declaration is now resolved and adds 0 | stays 1, 2, until LC-2k |
| after LC-2k | stays 1, 2 — `display` makes it a list item, and the visible `list-item 0` suppresses the implicit increment | **1, 3** — the implicit increment applies |

So **under this order either form is order-free against this program, on one condition: it lands whole** (rung 1:
§15.5.5 states the two declarations as one rule). A `display: list-item` without its `counter-increment:
list-item 0` turns W-SUM or W-SUM-D wrong at LC-2k, or at once if LC-2k has landed. Excluding `summary` from the
list-item predicate instead would be a second key against CSS Lists 3 §4.6's "every list item" — rejected at rung
1, and by One issue, one way. Under rev 2's order — the `display` key in LC-1, before N-1's removal — the whole
rule landing between LC-1 and LC-2 would have turned the cells wrong: c11's hazard, reached through another lane.
LC-2k's plan greps every UA sheet for `list-item` at its base — `crates/css/elidex-style/src/ua.rs`,
`crates/dom/elidex-dom-compat/src/legacy_ua.rs`, and the `details` UA shadow sheet once CV-details has landed it
(its file is CV-details' plan's to name: CV `:986`'s placeholder `<the UA shadow sheet's file>`) — and, for each
form present, pins its cell; a form present without its `counter-increment` gets the declaration added in LC-2k's
change (§7 F-15; in CV-details' sheet that is a cross-program edit, OQ-6).

**The exhaustive tables under that order.** Windows: W1 = after LC-1, W2 = after LC-2, W2k = after LC-2k, W3 =
after LC-3, W4 = after LC-4, W6 = after LC-6 (LC-4, LC-5 and LC-6 are unordered; no unit of one is an input of
another, so each is checked against W3). Inputs each slice opens — LC-1: (i) set and increment on one element in
spec order, (iii) explicit increments on a reversed counter as written, (iv) `reversed(name)` without an integer —
all three only where `counter-*` reach the computed style (pseudo-elements, elements declaring `content`, and the
bake's own output); LC-2: (v) author `counter-*` on every element; LC-2k: (ii) list items by `display`; LC-3: (vi)
per-property author override of the rule and the hints, (vii) `menu` and `dir`, (viii) HTML integer forms, (ix) the
hint population by mode; LC-4: (x) sibling scope and replace / nest; LC-6: (xi) `inherit` / `unset` / `initial` on
`counter-*` through the handler. "✓" = no cell turns wrong, with the witness; "gone" = the unit no longer exists;
"—" = no new input reaches the unit in that window. W6 is given in the second table only: no unit of the first
table survives to read (xi) except the resolve path, whose row says so.

*Table 1 — the units at `bf1cae35`.*

| Unit at `bf1cae35` | W1 (i, iii, iv) | W2 (v) | W2k (ii) | W3 (vi – ix) | W4 (x) |
|---|---|---|---|---|---|
| Bake tag gate `ol \| ul \| li` (`counter.rs:392`) | stays, for the arms that remain | — | — ((ii) does not reach it: what remains in the bake are tag facts, `ol` / `ul` resets and `li[value]`, which hold whatever the `display`) | gone | gone |
| `ol` arm guard `already_has` (`:307`) | — | ✓ author names `list-item` → the author's value (Q3c: 10, 11 before and after); names another counter → the reset is kept, wrong before and after (P3a, W-N2) | — | gone; Q3a, W-N1 / W-N2 turn right | gone |
| `ol` arm `start` parse (`str::parse`, `:316`, `:326`) and ±1 | — | — | — | gone; P14 and W-OV turn right | gone |
| `ol[reversed]` value `li_count + 1` (`:312`–`:320`) and `count_li_children` (`:277`) | gone in LC-1: the arm passes the absent integer (no earlier than §4.4.2 is in the step, c13) and §4.4.2 counts in the step, by the same predicate as the increment (tag); ✓ W-F3 stays 1, P4a stays 3, 2, 1, P4b and W-RV turn right | ✓ (v): author increments and sets in the scope are now visible to the walk — P4c stays (c1) | ✓ W-F3 stays 1: the `display: block` `li` leaves the count and the increment together (c5) | the arm is gone; the hint passes the same absent integer | gone |
| `ul` arm and its guard (`:334`–`:336`); `menu`, `dir` absent | — | ✓ as the `ol` guard | — | gone; P5 turns right; ✓ W-H3, W-H5 stay 1 (subtree scope) | ✓ W-H3, W-H3b, W-H5 stay 1: `menu` and `dir` reset by the rule (c3, c10) |
| `li` arm guard on the computed increment (`:343`–`:347`) | gone in LC-1 with the push (c9, c4); ✓ P1's markers and Q19 stay; Q13 turns right | (the guard is gone) ✓ W-F2 stays 1, 5, 6 | — | gone | gone |
| `li` arm `value` → set, guard `already_set`, parse (`:348`–`:351`); no increment when `value` parses | ✓ (i): the step now adds the implicit increment before the set — P2a stays 1, 5, 6 (c8) | ✓ author `counter-set` names `list-item` → the author's value, which is the spec's; names another counter → the hint is kept, wrong before and after (P2b) | ✓ a `li` with `display: block` and `value` still sets (the hint applies to the element whatever its `display`) and no longer increments | gone; P2b / Q2b turn right | gone |
| `li` arm increment push `("list-item", 1)` (`:364`) | gone in LC-1; ✓ P1 markers stay (the step adds it for `li`); P1's field turns right | gone | gone; P8a turns right here | gone | gone |
| Step order reset → set → increment (`:84`–`:128`) | fixed in LC-1; ✓ no element holds both a set and an increment of one counter today except through `content`-declaring author rules, which turn right (Q6, Q6b, W-O1) | ✓ P6 turns right | — | — | — |
| Negation of every increment on a reversed counter (`:117`) | fixed in LC-1 after the implicit increment's move and the end of the bake's push (c9); ✓ P4a stays; Q4c, Q4f, Q4g turn right | ✓ P4c stays 3, 2, 1 (c1): the explicit −1 applies as written and §4.4.2 gives 4 | — | — | — |
| Instantiate: reset and instantiate-if-absent push at the current depth, no replace (`:86`–`:125`) | — | ✓ (v): author counters under subtree scope — P11a 0, 0 → 1, 1, wrong before and after; W-H4, W-H4b stay 1 (the `div`'s own reset / set is now visible and the `ol`'s counter does not reach it) | ✓ (ii): a non-`li` list item instantiates under subtree scope — P8b "0", "0" → "1", "1", wrong before and after | ✓ the rule's resets nest and pop per subtree; sibling lists each start at 1 | replaced in LC-4; ✓ W-H4, W-H4b stay 1 (c2); P8b, P11a / Q11a, P11b, W-S1, W-S2 turn right; P12 must stay |
| Scope `push_scope` / `pop_scope` (`:42`–`:58`), `flatten` (`:165`), `set_counter` (`:64`) | — (the §4.4.2 walk takes its scope from the same model) | — | — | — | replaced in LC-4; the paged driver's `flatten` and `page` / `pages` re-audited (I4 × I9) |
| `is_continuation` skips increments only (`:113`) | ✓ the implicit increment is skipped with them (I5 × I9); by reading, a continuation re-runs reset and set — LC-1 states which steps a continuation runs (I3 × I9) | — | — | — | re-audited with `flatten` |
| Caller gates: pre-layout `is_list_tag \|\| has_counter_props` (`generated_content.rs:141`), returning for `none` and `contents` (`:105`, `:114`); paged walk: every styled element except `display: none` (`builder/walk.rs:131`) | one gate in LC-1: "has a counter property, or is a list item (the predicate)", minus the boxless set. By reading, the paged walk steps a `display: contents` element today (I7 × I9); not a right-today cell of the pre-layout pass. The gate no longer admits an `ol` / `ul` by tag: production ones pass by their reset (D-1); hand-built-style test fixtures that relied on the tag are updated by LC-1 (§5.3), not readmitted | ✓ (v): many more elements pass the gate (§3.1's bound) | the predicate's body changes; the gate follows it | — | — |
| N-1: `resolve_content` returns before the counter resolves (`box_model/mod.rs:416`) | hides (iii) and (iv) on elements that declare no `content`; ✓ P4c stays (implicit −1). It also hides author `counter-increment` from the step's new suppression test — harmless under the tag key (Table 2, row 2) | gone in LC-2 | — (c11: the key change relies on it) | — (c6: the rule and the hints rely on it) | — (c2: the scope model relies on it) |
| `reversed(name)` without an integer resolves to 0 (`box_model/mod.rs:529`; parser default, `crates/css/elidex-css/src/declaration.rs:653`) | gone in LC-1 (the absent-integer state); ✓ Q4e, Q4g turn right | ✓ P4e turns right | — | (c7: the hint relies on it) | — |
| `counter-*: inherit` resolves to the empty list (no handler: `get_computed` falls to `CssValue::Initial`, which `resolve_counter_reset` / `resolve_counter_property` drop at `_ => {}`, `box_model/mod.rs:538`, `:574`) | — | (v) makes it reachable on every element; wrong before and after wherever the parent's value is not `none` | — | — | — (W6: Table 2, last row) |
| The three call sites (`crates/css/elidex-style/src/walk.rs:168`, `generated_content.rs:144`, `builder/walk.rs:134`) | stay; each still runs the shrunken bake | stay | stay | gone | gone |
| Hint population by mode (no unit today: the bake applies the attributes in every mode and at every cascade entry) | — | — | — | (ix): under B0 / B / C ✓ every mode and entry keeps `start` / `value` / `reversed`; **under A, P2a and P4a turn wrong in `BrowserCore` / `App` and at every hint-less entry (§5.3)** — the one right-to-wrong change the program can ship, and only by decision (OQ-5) | — |

*Table 2 — the units each slice adds*, from the window in which each first exists.

| Unit added (by) | Reads | The windows it lives through |
|---|---|---|
| The list-item predicate, body = tag `li` (LC-1) | `TagType` | W1: — it is the key of the bake arm it replaces, so no input is new to it. W2 ✓ (v): W-F4 stays 1, 1, 2 — the `div` is not an `li`, and its now-visible `counter-increment: list-item 0` adds 0. W2k: the body becomes computed `display` (c11); ✓ W-F4 stays 1, 1, 2 (the explicit 0 suppresses the implicit increment); P8a turns right; P8b "0", "0" → "1", "1"; the marker reconcile now reads the predicate, and P8b's markers keep existing (both keys are `display` from here). ✓ W-SUM, W-SUM-D, with the condition above. W3, W4: — |
| The step's suppression test: computed `counter-increment` names `list-item` (LC-1) | the computed field | W1: it reads what the bake's guard read (c4's guard, moved here for its increment half): ✓ Q19 – Q21 stay; an `li` whose author `counter-increment: list-item N` is hidden by N-1 gets the implicit 1, as the bake gave it — wrong before and after. W2 ✓ P19 turns right (2, 4), P20 stays. W2k ✓ W-F4 (c11) |
| The step's sign test: the innermost `list-item` counter is reversed (LC-1) | the counter instance | W1 ✓ P4a stays (the instance comes from the bake's `ol[reversed]` arm). W2 ✓ (v): an author `counter-reset: reversed(list-item)` on an element with no `content` becomes visible — wrong → right. W3 ✓ `ol[reversed]` with an author `counter-reset: list-item 9`: the author's entry replaces the hint and decides the sign — counts up from 10, as today (Q3c's form) |
| §4.4.2's look-ahead walk (LC-1) | the scope's elements: computed `counter-increment` / `counter-set`, the list-item predicate, the boxless predicate, the scope model | W1 ✓ P4a, W-F3 stay; P4b, W-RV, W-BX turn right (it applies the boxless predicate itself, I6 × I7); it needs the sign change first (c12). W2 ✓ P4c stays (c1); P4e turns right. W2k ✓ W-F3 (c5). W4: its scope becomes LC-4's (I4 × I6) — a reversed counter's count then includes following siblings that increment it (W-RS: wrong → right by the letter of §4.4.2; LC-4's plan derives it) |
| The absent-integer state of a reversed reset entry (LC-1) | — (a representation) | W1 – W3: written by the parser / resolve for author `reversed(name)` and by the bake's `ol[reversed]` arm, then (W3) by the hint. W6: must survive `get_computed` → `inherit` → resolve unchanged (W-K2) |
| One caller gate with one boxless predicate (LC-1) | computed `display`; the list-item predicate | W1: the paged walk stops stepping `display: contents` elements (W-PG1). W2 ✓. W2k: follows the predicate |
| — (LC-2 adds no unit: it removes N-1) | | |
| — (LC-2k adds no unit: it changes the predicate's body) | | |
| The UA rule `ol, ul, menu, dir { counter-reset: list-item }` (LC-3) | the cascade (needs N-1 gone, c6) | W3 ✓ every list keeps its reset (the bake's `ol` / `ul` arms go in the same change); P5 turns right; an author `counter-reset` replaces it (W-N1, W-N2 turn right). W4 ✓ W-H3, W-H3b, W-H5 (c3, c10) |
| The list hint source (LC-3) | `TagType`, `Namespace`, `Attributes`; the shared integer parser; the mode (ix) | W3 ✓ P2a, P4a stay under B0 / B / C; (vi) P2b, P3a turn right; (viii) P14, W-OV turn right; a non-HTML element named `ol` / `li` stops getting the hint (W-NS: wrong → right; the bake was namespace-blind). Under A: (ix), Table 1's last row |
| The scope model: inherit, instantiate with replace / nest (LC-4) | the flattened element tree; the counters set | W4 (x): Table 1's rows for instantiate and scope; ✓ every "stays" cell of §6 pinned by LC-4 |
| §15.3.7's other rules and the `type` hints (LC-5) | the cascade; `list-style-type`, margins, padding | no counter unit reads any of them. Its own cells are wrong today (P17, P18, R1, R3, R4); `menu`'s and `dir`'s box rules sit on an inline box until `IBP-ua-display` gives them `display: block`, wrong before and after |
| The `counter-*` handler: `get_computed`, `initial_value`, `is_inherited`, and the trait-forced `parse` / `resolve` entries (LC-6) | the computed fields; **and it is read by the cascade** for the CSS-wide keywords (I1) | W6 (xi). `getComputedStyle`: the string `"initial"`, no property's computed value here, becomes the fields the cascade wrote (P10 / Q10). `inherit`: the empty list becomes the parent's computed list — wrong → right, **if** `get_computed`'s output has the shape `resolve_counter_reset` / `resolve_counter_property` read; a shape they do not recognise resolves to the empty list again, silently (W-K1; W-K2 for `reversed(name)` without an integer). `unset` / `initial`: right before LC-6 (they resolve to the empty list) and they stay right only if `is_inherited` is false and `initial_value` resolves to the empty list (`webref css css-lists-3 counter-reset`: initial `none`, inherited `no`) — W-K3, the one way LC-6 could turn a right cell wrong |

**LC-6 does open a window** (rev 2 said it did not; R2 finding A2-I1): registering a handler changes what
`inherit`, `unset` and `initial` resolve to in the cascade, not only what script reads. LC-5 opens none against
any counter unit.

Each slice's plan re-checks its window with its own cells at its base (the cells of §6 that are right in the
window before it and that the slice touches must stay right), re-derives both tables for any nested division, and
adds to Table 2 every unit its own mechanism introduces.

### §5.3 Obligations per slice (requirements; mechanism is each plan's)

- **LC-1**: the implicit increment is derived in the step from the list-item predicate — **one definition**, its
  body the tag `li` in this slice, read by the step, by the caller gate and by the §4.4.2 walk (I5; LC-2k changes
  the body and points the marker reconcile at it; its one other edit is §15.5.5's conditional declaration, §5.3
  LC-2k) — so both drivers get it; the caller gate becomes "has a counter property or is a list
  item", one gate for both drivers; the boxless set is **one predicate**, read by that gate and by the §4.4.2 walk
  (I7 × I9, I6 × I7); state which of steps 2 – 4 a continuation fragment runs (I3 × I9); decide the parsed encoding
  of an omitted integer (today `parse_counter_list` stores the default as a number, so `reversed(foo)` and
  `reversed(foo) 0` are indistinguishable) under §3.2's F7 decision, as **one `CssValue` shape** shared by the
  parser's output, the resolve's input and, later, the handler's `get_computed` output, so that LC-6's round trip
  holds (W-K2). **Every reader of that shape** is in the plan, and at `7fcaffed` they include two that the encoding
  can break: the serializer `CssValue::to_css_string` (`crates/core/elidex-plugin/src/values.rs:583`; it prints the
  `Keyword("reversed:a")` item verbatim, which is not re-parseable CSS) and the two resolve loops
  (`crates/css/elidex-style/src/resolve/box_model/mod.rs:519`–`:536`, `:559`–`:572`), which take the item after a
  name as its integer whatever it is — so an encoding that omits the number item makes `counter-reset: reversed(a)
  b 2` lose `b` (cell W-ML: `#o { counter-reset: reversed(a) b 2 } #o::before { content: counter(b) }` → 2;
  `getComputedStyle(#o).counterReset` is `reversed(a) b 2` from LC-6). Decide where the §4.4.2 value is held under
  D-4's criterion: a component on the resetting entity if it crosses from the pre-layout pass to the paged walk or
  outlives the run, inserted or removed on every exit path of the pre-layout pass (I11's discipline, D-4); otherwise
  run-local scratch under any key, or the counter instance. If a component, show that the pre-layout pass has run
  before the paged walk reads it; state the walk's bound (§3.1), that it takes its scope from the same definition
  the step uses (I4 × I6), and that both drivers get the same value from the whole scope (I6 × I9); clamp per
  §3.1's list; honour "only the last one is honored" for repeated names (§4.1); rewrite the tests that hand-build
  `counter_increment: vec![("list-item", 1)]` to stand in for the bake (`git grep -n -c 'counter_increment: vec' --
  crates` at `aeecf9b0`: 14 lines in three test files). **Update the fixtures the new gate stops baking** (D-1): tests
  that attach a hand-built `ComputedStyle` to an `ol` / `ul` with no `counter_reset` and rely on the pre-layout
  call to bake it, which that count and SQ-4's grep do not find. Named: `ol_with_items`
  (`crates/css/elidex-style/src/generated_content.rs:221`) and its users — `list_item_markers_number_in_document_order`
  (`:258`) reads 1, 1, 1 once the gate stops admitting the `ol` — and every such `ol` / `ul` among the
  `resolve_generated_content(&mut` callers (`git grep -c -F 'resolve_generated_content(&mut' 7fcaffed -- crates` →
  `generated_content.rs` 10, `crates/core/elidex-render/src/builder/tests/counter.rs` 7,
  `crates/core/elidex-render/src/builder/tests/display_types.rs` 6; the `counter.rs` ones already hand-build a
  `counter_reset`). Each gets the field the cascade writes (`counter_reset: [list-item 0]`, the bake's output until
  LC-3 and the UA rule's after it). Re-adding `ol` / `ul` to the gate would be a second key and is not allowed; audit §4.5's two branches this memo marks ✗ (a `content:
  none` pseudo; descendants of replaced elements); keep I11. The comment sweep (below) includes
  `crates/script/elidex-js/src/vm/host/html_olist_proto.rs:38`–`:41` ("(default = list length) live in cascade-time
  evaluation"): false once the count is §4.4.2's, in the counter step. **L1's files** (§7 F-16): LC-1 changes
  the paged gate and the bake's call in `crates/core/elidex-render/src/builder/walk.rs` (`:131`, `:134`) and the
  step the paged driver `crates/core/elidex-render/src/builder/paged.rs` runs, and L1's page children edit both
  files (#785 among them); the plan re-greps both at its base, names the L1 work landed or in flight there, and
  states any change it makes to what `set_counter("page" | "pages")` and `flatten` give the margin boxes, with the
  cells L1's 3b.3 / 3b.4 would see (W-PG1, W-PG2) — F-16 (a) and (b). If its plan divides it, the nested order is
  §5.2's (c8, c9, c12, c13: order fix → move → guard and push end → sign change → §4.4.2 → the arm's absent
  integer).
- **LC-2**: move the three counter resolves out of `resolve_content`'s early return; cover `style=""` (Q23); pin
  P4c, W-F2, W-F4, W-H4 and W-H4b as cells that stay right.
- **LC-2k**: change the predicate's body from the tag to computed `display` — the increment, the gate and the
  §4.4.2 walk follow through the one definition (if they do not, LC-1 left a second key: a finding against LC-1,
  not a second edit here) — and make the marker reconcile read the predicate in place of its inline `display` test
  (`generated_content.rs:97`–`:98`; I5), keeping its `list-style-type` half; nothing else in code, and in the UA
  sheets only the conditional declaration below. Pin W-F3 and W-F4 as
  cells that stay right, P8a as the cell that turns right and P8b at 1, 1. Grep every UA sheet for `list-item` at
  its base (§5.2 cross-lane names the files, the `details` UA shadow sheet included once CV-details has landed it);
  for each form of §15.5.5's `summary` rule present, pin W-SUM (document form) or W-SUM-D (shadow form), and if a
  form carries `display: list-item` without `counter-increment: list-item 0`, add that declaration to the rule in
  this change rather than exclude `summary` from the predicate (§7 F-15). **Cross-program touch**: for the shadow
  form the sheet is CV-details' UA shadow sheet (`docs/plans/2026-10-content-visibility.md`, CV `:986`'s
  placeholder), a file another program owns; who makes that edit is OQ-6, and the plan follows its outcome. Drop
  the predicate's defect note (§3.3).
- **LC-3**: implement the OQ-1 / OQ-2 outcome; reuse one integer parser (SQ-1); delete the bake and its three call
  sites; put `dir` in the UA rule (D-6); read the element's `Namespace` in the hint source, so that only HTML
  elements get the hints (W-NS); clamp the hint's integers and the parser's result per §3.1 (W-OV); edit the rustdoc
  and comments §1.7 lists — among them #408's statements of its criterion, which stop being complete when `dir`'s
  reset lands in the core sheet: the criterion at `crates/css/elidex-style/src/ua.rs:111`–`:116`, and its one named
  exception, `tt`, at `ua.rs:68`–`:72` and in the `crates/dom/elidex-dom-compat/src/legacy_ua.rs` header
  (`:1`–`:11`); LC-3 records `dir` there as the second exception, on HTML §16.3.3's ground (D-6); under option A add the
  core-mode and hint-less-entry cells of §1.6. Re-read the print entry and the paged driver at its base: L1 lands
  page-side children there (§7 F-16), and IBP-sandbox deletes `build_paged_pipeline` as a touch-time cleanup
  (`docs/plans/2026-09-inline-box-predicate.md:1488`–`:1497`), after which rows 4 – 5 of the entry table below keep
  only `prepare_print_style`'s other callers. **Every cascade entry point gets the hint population the outcome
  names** — the entries, grep-verified at `01426656`. `git grep -n -E 'resolve_styles(_with_compat)?\(' -- crates`
  → 84 lines; it does not match the print entry, so the census is `git grep -n -E
  '(resolve_styles(_with_compat)?|prepare_print_style)\(' -- crates` → 92 lines: 4 production calls (shell
  `lib.rs:128`, `:146`; `pipeline.rs:413`, `:421`), 1 tool (`crates/tools/elidex-wpt/src/harness.rs:41`), 3 benches,
  1 fixture (`page.rs:253`), 75 test lines, 3 definitions (`lib.rs:311`, `:337`; `page.rs:207`), 1 delegation
  (`lib.rs:319`), 4 doc / comment lines. The hint side: `git grep -n -E
  'hint_generator|no_hints|get_presentational_hints' -- crates` → 53 lines, the walk's at `walk.rs:36`, `:161`,
  `:210`, `:219`:

  | Cascade entry | Hint argument today | `start` / `value` / `reversed` today | After LC-3 under A | under B0 / B | under C |
  |---|---|---|---|---|---|
  | `elidex_style::resolve_styles` (`crates/css/elidex-style/src/lib.rs:311`), which passes `no_hints` (`:301`, `:323`); callers: `crates/tools/elidex-wpt/src/harness.rs:41`, `crates/css/elidex-style/benches/style_resolve.rs` (3), the `elidex-style` tests, and the `cfg(test)` callers `crates/dom/elidex-dom-api/src/computed_style.rs:227`, `:253`, `crates/layout/elidex-layout/src/layout/tests/basic.rs:906`, `crates/layout/elidex-layout-flex/src/tests/grow_shrink.rs:493` | none | honoured (the bake) | **lost**: no generator reaches this entry, so the WPT `css/css-lists` subset (SQ-3) cannot see the attributes | honoured: the core source is inside the cascade | honoured only if every hint source moves inside |
  | shell `resolve_with_mode`, compat arm (`crates/shell/elidex-shell/src/lib.rs:128`) | `get_presentational_hints` | honoured | honoured (the compat generator) | honoured | honoured |
  | shell `resolve_with_mode`, core arm (`:146`; `no_hints`, `:162`) | none | honoured | **lost** | honoured | honoured |
  | `elidex_style::page::prepare_print_style` (`crates/css/elidex-style/src/page.rs:207`): `walk::preliminary_root` (`walk.rs:252`), then `apply_document_style` (`lib.rs:372`), whose walk runs the bake; shell caller `build_paged_pipeline` (`crates/shell/elidex-shell/src/pipeline.rs:401`, `#[allow(dead_code)]`, reached from its tests), compat arm (`:413`) | `get_presentational_hints` | honoured (the bake, in `apply_document_style`'s walk) | honoured (the compat generator) | honoured | honoured |
  | `prepare_print_style`, core arm (`pipeline.rs:421`; `no_hints`); also the fixture (`page.rs:253`, feature `fixtures`) and the tests (`page/tests.rs` (3), `resolve/var_resolution/page_receivers.rs:45`), which pass an empty closure | none | honoured | **lost** | honoured | honoured |
  | inside the walk: one flat-tree `walk_node` (`crates/css/elidex-style/src/walk.rs:308`) resolves every node — light tree, shadow-tree children (through `walk_shadow_tree`) and slotted nodes — at its one `resolve_and_attach_style` site (`:321`); that calls `compute_element_style`, which calls `hint_generator` once (`:219`), and then runs the bake (`:168`). `preliminary_root` (`:252`, the print entry's first step) also calls `compute_element_style` with the hints but does not run the bake | — | — | the one site and `preliminary_root` keep the caller's generator: hints already reach shadow trees, unlike the compat *sheet* | the core source is applied inside `compute_element_style`, so the walk's site and `preliminary_root` both get it | the same |

  Under B0 / B the core hint source is applied **by the cascade itself**, not passed by its callers, so no entry
  point can omit it (OQ-2).
- **LC-4**: state the counters-set model it implements, the tree it walks (the flattened element tree, I4 × I7),
  and how `flatten` and the paged continuation path keep I4 and I9; the §4.4.2 walk follows the new scope through
  the shared definition (I4 × I6), and its plan derives W-RS (a reversed counter's count under sibling scope); P12
  stays; pin W-H3, W-H3b, W-H4, W-H4b and W-H5 as cells that stay right, in every mode. `flatten` and
  `set_counter` sit in `crates/core/elidex-render/src/builder/paged.rs`, which L1's page children edit: the plan
  re-greps that file at its base and names the L1 work landed or in flight there (§7 F-16).
- **LC-5**: one PR for the rule units and the `type` hints, or two if its plan finds the hint half gated longer
  than the rule half; it states which. Each `dir` rule goes into the rule `ul` has, in `ul`'s sheet (§4.3: one
  selector list, no second copy of the rule text), `dir[type]` with `ul[type]` (U12). The `ol[type]` hint's keyword
  set is the one `OL_TYPE_VALUES` already holds (`crates/dom/elidex-dom-api/src/element/enumerated_reflect.rs:155`,
  `["1", "a", "A", "i", "I"]`, read by the VM's `ol.type` reflection at
  `crates/script/elidex-js/src/vm/host/html_olist_proto.rs:213`). `elidex-style` cannot depend on `elidex-dom-api`
  (the dependency runs the other way: `crates/dom/elidex-dom-api/Cargo.toml:19`), so the hint source and the
  reflection share one table in a crate below both — the question SQ-1 asks for the integer parser — not a second
  copy (One issue, one way); if the plan finds the two sets must differ, it says why. The `ul[type]` / `li[type]`
  set (`none`, `disc`, `circle`, `square`, ASCII case-insensitive) has no HTML-attribute table today; the
  `list-style-type` keyword lists (`crates/css/elidex-css/src/declaration.rs:471` and two more) are the property's
  grammar, not the attribute's.
- **LC-6**: the handler's home crate (SQ-2) and the serialization (`none`; name–integer pairs; `reversed(name)`
  with and without an integer, §3.2's F7 reading), with `#11-cssvalue-list-separator-fidelity` and
  `#11-computed-style-enumeration` as neighbours it must not duplicate. **The handler is a cascade input (D-10,
  I1 × I10).** (a) Round trip: for every computed value `v` of each of the three properties, resolving
  `get_computed(v)` again yields `v` — including `none`, repeated names, and `reversed(name)` with and without an
  integer; LC-6 tests it as a property of the pair (handler, resolve), not only through cells W-K1 / W-K2. (b)
  `initial_value` resolves to the empty list and `is_inherited` is false for all three (W-K3). (c) Its plan lists
  every other reader of `get_computed` / `initial_value` / `is_inherited` for these names at its base. **One parse,
  one resolve (D-10).** The trait
  brings `parse` and `resolve` with `get_computed`. For `counter-*` the single parse path is the declaration parser
  (`crates/css/elidex-css/src/declaration.rs:653`–`:654`): it is the only one a registry-less parse reaches, and the
  UA sheet that will carry `counter-reset` is parsed that way (`crates/css/elidex-style/src/ua.rs:199`), as is the
  WPT harness's author CSS (`crates/tools/elidex-wpt/src/harness.rs:40`). The single resolve path is the cascade's
  resolve step. The handler's `parse` and `resolve` entries call those implementations (the handler crates already
  depend on `elidex-css`, e.g. `crates/css/elidex-css-box/Cargo.toml:15`; the resolve functions move to where the
  handler can call them) and hold no copy; its plan states where they live and shows that I1 holds — the cascade's
  resolve step stays the only caller that writes a live component. This is **not** how the other handlers are built:
  for most properties two parse and two resolve implementations exist side by side — e.g. `list-style-type` is
  parsed at `declaration.rs:467` and in `crates/css/elidex-css-text/src/lib.rs:94`, resolved in
  `crates/css/elidex-style/src/resolve/font.rs:81` and at `elidex-css-text/src/lib.rs:174`, and served to
  `getComputedStyle` from `:287`; the handler's `parse` runs only for names the built-in match does not know
  (`PropertyGrammar::select`, `declaration.rs:258`–`:262`), and only the transform family resolves through its handler
  (`crates/css/elidex-style/src/resolve/mod.rs:354`). That project-wide split is a finding for the PM (§7 F-10), not
  this program's to repair; LC-6 does not add another copy to it. L1's typed-shorthand-handlers plan
  (`docs/plans/2026-11-typed-shorthand-handlers.md`, §1: "Change CssPropertyHandler, all implementations,
  coordinator and serialize_omit_initial in one change") changes the same trait; LC-6's plan re-reads the trait at
  its base and names the order against that program's landing.
- **Every slice**: carry §3.3's docstring-citation table for its own functions.
- **Every slice**: remove the comment that points at this slot for the defect it closes (`git grep -n
  'list-counter-baking' -- crates` → `counter.rs:81` and `generated_content.rs:131` are the step order's, LC-1;
  `generated_content.rs:137` is the bake's, LC-3). And sweep **by concept**, since a comment can describe the bake
  without naming the slot or a function: `git grep -n -i -E 'cascade-time|implicit (list|counter)|implicit_list|
  li_count|list length|baked the implicit' -- crates` (pattern on one line) → 27 lines in 6 files at `aeecf9b0`
  (`counter.rs` 13, `generated_content.rs` 7, `builder/walk.rs` 3, style `walk.rs` 2, `html_olist_proto.rs` 1; the
  one in `url_search_params.rs` is unrelated). Each slice rewrites the lines its change makes false; the DoD greps
  (§10) do not see these.
- **Every slice**: name the tests that guard its surface (engine-level cells from §6, and the counter subset of the
  WPT `css/css-lists` directory if the runner can take it — SQ-3), per *Supported-surface testing*; new shell-level
  cells go in a new test file, since `crates/shell/elidex-shell/src/tests.rs` is 1081 lines.

---

## §6. Edge matrix (seed cells; each slice's plan owns its final list)

"First green" is the slice after which the cell holds the spec value in **both** engine modes, except rows marked
(P-1), whose core-mode value follows OQ-1. "Stays" cells are right today and are pinned by the slices named, as the
windows of §5.2 require. Rows from W-F4 down were added in rev 3 and rev 3.1 without a probe run: their "Today" column is by
reading (§0.1), and the owning slice runs each at its base.

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
| P8a | 1, –, 2 | 1, –, 3 | LC-2k |
| P8b | 1, 2 | 0, 0 | LC-4 (1, 1 from LC-2k) |
| P11a, Q11a | 1, 2 | 0, 0 / 1, 1 | LC-4 |
| P11b | 1, 2 | 1, 1 | LC-4 |
| P12 (`counters()` across sibling `ol`) | 1 / 1, 2 | same | stays (LC-4 must keep it: the replace rule) |
| Q13 | 1, 5, 6 | 1, 3, 4 | LC-1 |
| P14a, P14b, P14d | 5; 5; 7 | 1; 1; 1 | LC-3 |
| P17 | "a" | "1" | LC-5 (P-1) |
| P18, R1 | circle | disc / decimal | LC-5 |
| R3 margins, padding; R4 | 1em, 40px; 0 | 0, 0; 16px | LC-5 |
| P19, P20 | 2, 4; 1, 2 | 1, 2; 1, 2 (the author value absent) | LC-2 / stays |
| Q19 – Q21 | 2, 4; 1, 2; 1, 2 | same (Q20's and Q21's fields carry the baked increment; markup in §0.2) | stay; Q20's and Q21's fields LC-1 |
| Q23 | 3 | 0 | LC-2 |
| P10, Q10 | the computed values | `"initial"` | LC-6 |
| W-F2 | 1, 5, 6 | same | stays: LC-2 (c4) |
| W-F3 | y = 1 | same | stays: LC-1, LC-2k (c5) |
| W-H3, W-H3b | b = 1 | same | stay: LC-4 (c3) |
| W-H4, W-H4b | b = 1 | same | stay: LC-2, LC-4 (c2) |
| W-H5 | b = 1, in every mode | same | stays: LC-3 (the rule covers `dir`), LC-4 (c10) |
| W-RV | 11, 10, 9 | 3, 10, 9 | LC-1; (P-1) after LC-3 |
| W-S1 | 1 | 0 | LC-4 |
| W-S2 | 2 | 1 | LC-4 |
| W-O1 | 7, 7 | 8, 8 | LC-1 |
| W-N1, W-N2 | 2 | 1 | LC-3 |
| W-PG1 (paged): a `display: contents` element with `counter-increment: c`, read in a margin box | the element does not increment | by reading: the paged walk increments (`builder/walk.rs:131`); not run | LC-1 |
| W-PG2 (paged): an element with `counter-reset: c` continued on a second page, `c` read in the second page's margin box | LC-1's plan derives it (I3 × I9) | by reading: the continuation resets again; not run | LC-1 |
| W-F4: `<ol><li>a<div style="display:list-item;counter-increment:list-item 0">x</div><li>b</ol>` | 1, 1, 2 | 1, 1, 2 (the `div` is not an `li`; its `counter-increment` is dropped, N-1) | stays: LC-2, LC-2k (c11) |
| W-SUM: `<ol><li><details><summary>s</summary></details><li>b</ol>` | 1, 2 (with HTML §15.5.5's rule, the `summary` adds 0) | 1, 2 (no such rule; `summary` is `display: block`) | stays: LC-2k, when the rule is in the sheet at its base (§5.2, cross-lane) |
| W-SUM-D: `<ol><li><details>x</details><li>b</ol>` | 1, 2 (the default summary, in `details`' UA shadow tree, is a list item that adds 0) | 1, 2 (no UA shadow tree is built) | stays: LC-2k, when CV-details' shadow rule is in its sheet at LC-2k's base; otherwise CV-details pins it (§7 F-15). With `display` alone after LC-2k: 1, 3 |
| W-ML: `#o { counter-reset: reversed(a) b 2 } #o::before { content: counter(b) }` | 2; `getComputedStyle(#o).counterReset` is `reversed(a) b 2` | 0 (N-1); with `content: normal` on `#o`: 2 (the parser stores a number after every name) | stays: LC-1 (the absent-integer encoding must not make the resolve take `b` as `a`'s integer), LC-2 (the no-`content` form); the serialization LC-6 |
| W-BX: `#o { counter-reset: reversed(foo) } p { counter-increment: foo -1 } p::before { content: counter(foo) }`, three `p`, the second `display: none` | 2, –, 1 | 0, –, 0 (N-1); with `content: normal` on `#o` and `p`: 1, –, 2 | LC-1 (the `content`-declaring form) / LC-2 |
| W-K1: `#o { counter-increment: c 2 } #i { counter-increment: inherit } #i::before { content: counter(c) }`, `#i` a child of `#o` | 4 | 0 (N-1); 2 from LC-2 (the `inherit` resolves to the empty list) | LC-6 |
| W-K2: `#o { counter-reset: reversed(c) } #i { counter-reset: inherit } p { counter-increment: c -1 } p::before { content: counters(c, ".") }`, `#i` a child of `#o` holding two `p` | "0.2", "0.1"; `getComputedStyle(#i).counterReset` is `reversed(c)` | from LC-2: "2", "1" | LC-6 (with the absent integer lost in the round trip: "0.-1", "0.-2") |
| W-K3: `#o { counter-increment: c 2 } #i { counter-increment: unset } #i::before { content: counter(c) }`; the same with `initial` | 2 | 0 (N-1); 2 from LC-2 | LC-2; stays: LC-6 |
| W-NS: an element whose local name is `ol` in a non-HTML namespace, `start="5"`, holding two HTML `li` (reachable only through `createElementNS`: the HTML parser breaks `ol` / `ul` / `li` / `menu` out of foreign content; LC-3's plan checks that the VM exposes it at its base) | no hint: the first item is not 5. Whether the element resets at all follows the UA sheet's namespace matching (§7 F-14); LC-3's plan derives the values | 5, 6 (the bake reads the tag name only) | LC-3, for the hint |
| W-OV: `<ol start="2147483648">`, `<ol start="-2147483648">`, `<ol reversed start="2147483647">`, `<li value="99999999999">` | each clamps to the `i32` range, where the integer enters and at the ±1 (§3.1); LC-3's plan derives the four values | `start="2147483648"` → 1 (`str::parse` fails); `start="-2147483648"` → overflow at `s - 1` | LC-3 |
| W-RS: `<div><ol reversed><li>a<li>b</ol><p style="display:list-item">z</p></div>` | 3, 2, 1 by the letter of §4.3 / §4.4.2 (`z` is in the counter's scope and increments it); LC-4's plan derives it | 2, 1, and `z` shows 0 (1 from LC-2k, its own counter) | LC-4 |

Derivations of the less obvious spec values: P4b — two elements increment (the boxless one cannot), so num = 1 + 1,
plus the last non-zero negated increment 1 → 3, then 2, 1. Q4f — three explicit `+1`: num = −3, plus −1 → −4, then
−3, −2, −1. Q13 — the author's `counter-increment` and the hint's `counter-set` are different properties; increment
then set gives 5. P5 — `menu` is a child of the first `li`; its UA reset nests a counter, and `d` continues the
`ol`'s. W-RV — the second item increments (its negated increment 1 becomes the last non-zero one) and sets: num =
1 + 10, the loop breaks, + 1 → 12; then 11, 10 (set), 9. W-S2 — the inner `div` inherits the `ul`'s counter from its
preceding sibling (§4.4.1 step 3) with the value 1 from the element before it in tree order (step 4), and b
increments it. P19 — `li { counter-increment: list-item 2 }`. P20 — `li { counter-increment: other }`: the implicit
increment still applies. W-F4 — the `div` is a list item with an explicit `list-item 0`, so §4.6's implicit
increment does not apply and it shows the `ol`'s counter unchanged. W-BX — two `p` increment (the boxless one
cannot): num = 2, plus 1 → 3; then 2, 1. W-K1 — `#o` instantiates `c` and raises it to 2; `#i` inherits the
declaration `c 2` and raises the same counter to 4. W-K2 — `#i`'s inherited `reversed(c)` instantiates a nested
counter whose scope holds both `p`: num = 2, plus 1 → 3; then 2, 1; `#o`'s counter has no element left that
increments it, so it stays 0 and `counters()` prints it first.

---

## §7. Out of scope, findings for the PM, and scout questions

**Findings (each needs an owner or a slot; this memo registers nothing).** Every proposed slot below is drafted
with name, *Gap*, *Why not this program's*, *Trigger*, *Owner* and *Re-eval-date*; names, owners and dates are
proposals for the PM (OQ-4). Existing slots were searched first, in memory `project_open-defer-slots.md` on
2026-11-26: `/usr/bin/grep -n -o -i -E` for slot ids containing any of `direction`, `dir-`, `match-parent`,
`text-align`, `logical`, `handler`, `layer`, `revert`, `namespace` (control: the bare slot-id pattern finds 1031
ids) — the hits used are named where they apply. The drafts are prose by choice, not rows of the slot-table
schema: `python3 .claude/tools/plan-memo-umbrella-check.py docs/plans/2026-11-list-counter-ua-rule-and-hints.md`
exits 2 at `7fcaffed` (no table matches its `citation` / `stub` / `slice` / `slot` schemas) — unscanned, not clean
(CLAUDE.md, *Plan-memo umbrella checker*). CLAUDE.md asks an exit 0 of the VM-P4 family's entry memo only, and this
memo is not in that family; the PM transcribes the drafts into the ledger (§9).

- **F-1 (N-6)** — conforming-attribute dimension hints are compat-gated. Already `IBP-layout`'s by IBP §2; P-1b's
  outcome is its input. No new record.
- **F-2 (N-7)** — `dir` → `direction` is a UA rule in HTML §15.3.5, generated as a hint in `elidex-dom-compat`
  (`push_dir_attr`, `presentational.rs:430`), so `BrowserCore` / `App` ignore the `dir` attribute (R2). No slot
  found (the search's one near hit, `#11-bidi-full-uba-fidelity`, is the bidi algorithm). **Proposed slot**:
  `#11-dir-attribute-ua-rule-in-core` — *Gap*: §15.3.5's `[dir]` rules are UA rules; elidex produces them as
  compat-gated hints. *Why not this program's*: no counter is involved; the repair moves a rule family to the core
  UA sheet and its plan must first check what `:dir()` and `dir=auto` support the selector engine has. *Trigger*: a
  core mode becoming production-selectable (`#11-async-core-storage-cookiestore`), or the next change to
  `push_dir_attr`. *Owner* proposed: Layout (or L1). *Re-eval-date* proposed: 2026-12-31.
- **F-3** — hints enter as `Origin::Author`, specificity 0, order 0 (`cascade/mod.rs:231`–`:238`). CSS Cascade 5
  §6.5's first paragraph puts document-language styling "in either the UA-origin or a special-purpose author
  presentational hint origin between the regular user origin and the author origin", independent for cascading and
  part of the author origin for `revert` only. **Whether elidex's placement is a defect rests on a reading.** The
  section's second paragraph lets the document language decide ("A document language may define whether such a
  presentational hint enters the cascade as UA-origin or author-origin; if so, the UA must behave accordingly"),
  and HTML §15.2 does speak to it: "the author-level zero-specificity presentational hints part of the CSS
  cascade". If that phrase names §6.5's hint origin — the only author-side origin its first paragraph offers — then
  elidex's regular author origin differs from it; if it names the regular author origin at zero specificity, elidex
  does what HTML defines. This memo does not settle it; rev 2 stated the first reading as fact. The difference is
  **not observable today** under either: `revert` is not a CSS-wide keyword elidex parses (`parse_global_keyword`,
  `crates/css/elidex-css/src/values.rs:5`, accepts `initial` / `inherit` / `unset` only), there are no cascade
  layers (`git grep -n -i -F '@layer' -- crates/css` → 0; control `@media` → 36 lines in `parser.rs`) and no user
  origin (`git grep -n -w 'Origin::User' -- crates` → 0; control `Origin::UserAgent` → 12). It becomes observable
  with any of the three: with layers, a hint at author order 0 would beat a layered author rule that a hint in its
  own origin loses to. The program adds its hints at the existing seam. **Record proposed: a facet of an existing
  slot, not a new one** (rev 2 proposed an owner-less `#11-presentational-hint-cascade-origin`):
  `#11-css-wide-revert-keyword` — owner L1 (PM 470), backstop 2026-12-15 — already holds "revert support [needs a
  `CssValue` variant + cascade origin rollback]", so the origin model is that slot's work. Facet: *Gap*: hints are
  regular author declarations, not a separate origin (subject to the reading above, which that slot's plan
  settles). *Why not this program's*: no reader can tell today. *Trigger*: the slot's own, or `@layer` / a
  user-origin sheet entering the cascade, whichever comes first. *Owner*, *Re-eval-date*: the slot's.
- **F-4** — `HtmlElementHandler` (`crates/core/elidex-plugin/src/traits.rs:222`) offers `default_style` and
  `spec_level` per tag and has no consumer outside `elidex-plugin`
  (`git grep -n -E 'create_html_element_registry|HtmlElementHandler' -- crates`, hits only under
  `crates/core/elidex-plugin`). It is the Plugin-first home OQ-2 weighs, and "dead code は接続するか削除" applies
  to it either way. (IBP assigns its connection to `IBP-classify`, F4 / F7's home.)
- **F-5** — `menu { display: block }` is missing (R3) → input to `IBP-ua-display`.
- **F-6** — `li { text-align: match-parent }` (U2) needs a `text-align` value elidex does not parse (`webref css
  css-text-3 text-align` lists `match-parent`; `git grep -n -F -e 'match-parent' -e 'MatchParent' -- crates` → 0,
  control `"text-align"` → 32 lines). No slot found. **Proposed slot**: `#11-text-align-match-parent` — *Gap*: the
  value, and the UA rule that uses it. *Why not this program's*: it is a text-module value with its own
  computed-value rule (against the parent's `direction`); no counter reads it. *Trigger*: the next change to
  `text-align` parsing or resolution, or right-to-left list rendering work (with F-9). *Owner* proposed: L1 (or
  Layout). *Re-eval-date* proposed: 2026-12-31.
- **F-7** — HTML integer / dimension microsyntax parsers exist in at least four crates (`git grep -n -i -c 'rules for
  parsing' 7fcaffed -- crates` lists 12 files; the parser definitions among them:
  `crates/api/elidex-api-canvas/src/component.rs:170` `parse_canvas_dim`,
  `crates/dom/elidex-dom-api/src/element/numeric_reflect.rs:88` `parse_integer` (`:30` `parse_unsigned_long`),
  `crates/dom/elidex-dom-compat/src/presentational.rs:240` `parse_dimension_value`,
  `crates/dom/elidex-form-core/src/reflect.rs:48` `parse_positive_with_fallback`). LC-3 must not add another (D-8).
  **Record proposed: a facet of an existing slot.** `#11-form-numeric-attr-html-integer-parsing`
  (`project_open-defer-slots.md:345`) already names the repair, "a shared `parse_html_integer` primitive +
  per-rule wrappers across ~8 sites" (the row's words, verified 2026-11-26). Facet: *Gap*: the four parsers above, beside the form sites that slot lists.
  *Why not this program's*: LC-3 needs one signed-integer parser (§2.3.4.1), which SQ-1 places; the non-negative,
  dimension and per-reflection wrappers are reflection and form semantics, not counters. *Trigger*: the slot's own,
  or LC-3's landing of the shared signed-integer parser, whichever comes first — that is where the others can start
  delegating. *Owner*, *Re-eval-date*: the slot's — its row names no owner and a Re-eval of 2026-11-30, so the
  PM assigns the owner and a date (OQ-4).
- **F-8** — no reader of HTML §4.4.8's *ordinal value* exists (`git grep -n -i -E 'ordinal|posinset' 7fcaffed --
  crates/dom/elidex-a11y` → 0; control `git grep -c -i -w listitem 7fcaffed -- crates/dom/elidex-a11y` →
  `roles.rs` 3). **No record proposed**, and this defers no planned work: no reader is wrong today, because none
  exists. *Trigger* (the promotion rule, §0.5 row 4): the first reader of list ordinals outside the two counter
  drivers — an accessibility list-position mapping, say — makes the counters set a component written by the
  pre-layout pass, in that reader's plan. *Receiver*: §0.5 row 4 of this memo; the PM decides in OQ-4 whether a
  slot should hold it.
- **F-9** — the UA sheet writes physical box longhands where HTML §15.3.7 writes flow-relative ones (U3
  `margin-block`, U5 `margin-inline-start`, U6 `padding-inline-start`; `ua.rs:53`–`:57`, `:63`, `:64`), and no
  flow-relative box property is parsed at all (`/usr/bin/grep -rn -E '(margin|padding)-(inline|block)|inset-(inline|block)'
  crates --include='*.rs'` → 0 at `aeecf9b0`; control `(margin|padding)-(top|left)` → 256 lines). No slot found
  (the open-slot registry
  and the M4-12 roadmap searched for `logical`, `inline-start`, `block-start`; the only hits are `logical_px`). By
  reading, not rendered here: in a right-to-left list the 40px sits on the left where the spec's inline-start is the
  right. **Proposed slot**: `#11-ua-sheet-flow-relative-box-properties` — *Gap*: as above, for every §15.3 rule
  that uses a flow-relative property, not the list rules alone. *Why not this program's*: the repair is
  css-logical-1 property support in the property system, which the list rules only consume; LC-5 adds `menu`'s and
  `dir`'s rules in the sheet's existing physical form and names them in the slot. *Trigger*: flow-relative box
  properties entering the property registry, or UA-sheet fidelity work for `direction` / vertical writing modes.
  *Owner* proposed: L1 or Layout. *Re-eval-date* proposed: 2026-12-31.
- **F-10** — the property-handler split (§5.3 LC-6): for most properties the built-in declaration parser and the
  cascade's resolve functions are one implementation and the registered handler's `parse` / `resolve` a second;
  `docs/design/ja/11-parser-design.md:19` says property parsing is delegated to the handlers. Extent not measured
  here (seen for `list-style-type` and `content`: `crates/css/elidex-css-box/src/lib.rs:162`, `:303` beside
  `declaration.rs:650`). Constraint any repair meets: registry-less parses (the UA sheet, the WPT harness) reach
  only the built-in parser, and the handler crates depend on `elidex-css`. No slot found in the open-slot registry
  under `CssPropertyHandler`, `property-handler`, `handler` or 二重実装. **Proposed slot**:
  `#11-css-property-handler-dual-implementation` — *Gap*: two parse and two resolve implementations per property,
  one of them reached only for names the built-in table does not know. *Why not this program's*: it spans every
  handler crate; LC-6 only refrains from adding a copy (D-10). *Trigger*: L1's typed-shorthand-handlers program
  changing `CssPropertyHandler` and all its implementations in one change
  (`docs/plans/2026-11-typed-shorthand-handlers.md`, §1) — the natural point to decide which implementation is the
  one — or the next handler registered. *Owner* proposed: L1. *Re-eval-date* proposed: 2026-12-31.
- **F-11** — a citation drift outside this memo: `crates/script/elidex-js/src/vm/host/html_olist_proto.rs:8` cites
  "HTML §2.4.4.1 ("rules for parsing integers")"; `webref heading html 2.4.4.1` finds no such section, and the
  rules are §2.3.4.1 Signed integers (`webref heading html 2.3.4.1`). For the cite-sweep lane; not edited here.
- **F-12** — obsolete elements that §16.3.3 equates with a conforming one. The section has five sentences of one
  form, "User agents must treat X elements in a manner equivalent to Y elements in terms of semantics and for
  purposes of rendering": `acronym` ≡ `abbr`, `dir` ≡ `ul`, `listing` ≡ `pre`, `plaintext` ≡ `pre`, `xmp` ≡ `pre`
  (`webref body html other-elements,-attributes-and-apis`). Each is a "must" outside §15, so X's rules belong where
  Y's are (§4.3) — for `ul` and `pre`, the core sheet (`ua.rs:16`, `:66`, `:73`, `:82`). This memo decides that for
  `dir`'s list rules only (D-6). IBP's Appendix G seeds `dir, listing, plaintext, xmp { display: block }` as a
  **compat-sheet** rule (IBP `:1955`); neither sheet names any of the four today (`/usr/bin/grep -n -w -E
  'acronym|listing|plaintext|xmp' crates/css/elidex-style/src/ua.rs
  crates/dom/elidex-dom-compat/src/legacy_ua.rs` → 0; control `-w pre` → the four `ua.rs` lines above and a
  test). Proposed: an input line for `IBP-ua-display` — §16.3.3's five sentences as rung-1 input to its placement,
  beside the `tt` precedent.
- **F-13** — two candidate spec issues, noted and **not filed by us**: (a) CSS Lists 3 §4.1's *Computed value* line
  says a `reversed()` item is "paired with an integer" while the value definition gives it "no starting value if
  not given" (§3.2, F7 note); (b) HTML §15.3.7's `ol, ul, menu { counter-reset: list-item; }` omits `dir`, which
  §16.3.3 requires to be rendered as `ul` (§4.3), and so do its `ul[type=…]` hint selectors (U12).

- **F-14** — namespace blindness of the list markup facts. HTML §15.3.7's rules and hints are scoped to HTML
  elements (`@namespace "http://www.w3.org/1999/xhtml"`). The bake keys on the tag name alone (`counter.rs:299`,
  its `match tag`), and so does the compat hint generator (`get_presentational_hints`, `presentational.rs:47`):
  `git grep -n -w 'Namespace' -- crates/dom/elidex-dom-compat/src/presentational.rs
  crates/css/elidex-style/src/counter.rs crates/css/elidex-style/src/walk.rs` → 0 (control: the component is
  `crates/core/elidex-ecs/src/components.rs:62`). UA-sheet rules cannot be namespace-scoped either: the CSS parser
  has no `@namespace` (`git grep -n -i -c -F '@namespace' -- crates` → one line, in `elidex-css/src/serialize.rs`).
  This program's part: the list hint source reads `Namespace` (D-7, LC-3, W-NS), and the bake goes. The rest
  belongs to an existing slot's class — `#11-html-element-type-tests-without-namespace` (owner L3, PM 780 in
  `PORTFOLIO-archive-2026-W47.md`; ledger `project_open-defer-slots.md:1702`: "elidex decides that an element is a
  given HTML element type by its local name alone"). **Record proposed: a facet of that slot**, not a new one
  (neighbour: `#11-selector-foreign-name-case`). Facet: *Gap*: two more members — `get_presentational_hints`
  (`presentational.rs:47`) and the UA sheets' type selectors (no `@namespace` in the CSS parser) — give an SVG or
  MathML element with an HTML local name HTML's hints and UA rules. *Why not this program's*: the list hint source
  reads `Namespace` (LC-3) and the bake goes, so this program adds no member; the other hints and the sheets'
  selectors are every element's, and the slot's census already holds the class (110 functions). *Trigger*: the
  slot's own (the PR wiring `#11-vm-create-element-ns-missing`, or the first SVG / MathML DOM interface PR), or
  `@namespace` support entering the CSS parser. *Owner*: L3 (the slot's). *Re-eval-date*: 2026-12-31 (the
  slot's).
- **F-15 — cross-lane, for the PM: HTML §15.5.5's `summary` rule against LC-2k** (§5.2 cross-lane). The facts, at
  `7fcaffed`:
  (1) HTML §15.5.5 states `display: list-item` and `counter-increment: list-item 0` as **one rule**, in two forms:
  `details > summary:first-of-type { … }` for the document, and `:host summary { … }` for the default summary in
  `details`' UA shadow tree (§3.2).
  (2) IBP's slot `#11-ua-sheet-html-rendering-display-fidelity` holds the document form's **`display` declaration
  alone**: its member is "`details > summary:first-of-type { display: list-item }`" (IBP `:1964`; rev 10, `:596`:
  "the slot keeps `details > summary:first-of-type` alone"). Its repair needs `:first-of-type`, which elidex lacks
  (IBP `:1967`; `git grep -c -F 'first-of-type' 7fcaffed -- crates/css/elidex-css/src/selector` → 0, control
  `first-child` → 11 lines in 6 files, verified 2026-11-26). The slot is **not registered** (ledger `project_open-defer-slots.md:704`;
  IBP `:1695`: `IBP-ua-display` registers it only for the remainder its plan derives), so the member's lander is
  whoever registers and repairs that slot after `:first-of-type` exists — not `IBP-ua-display`'s own PR.
  (3) CV-details builds the shadow form: "CV-details builds only §15.5.5's shadow `:host summary` (the default
  summary's display)" (`docs/plans/2026-10-content-visibility.md:880`), in a UA shadow sheet whose file CV-details'
  plan names (CV `:986`, the placeholder `<the UA shadow sheet's file>`).
  (4) The `counter-increment: list-item 0` half is in **no plan**: `git grep -l 'counter-increment: list-item 0'
  7fcaffed -- docs` → this memo only; 0 lines in the open-slot ledger (`/usr/bin/grep -c`).
  **Constraint, for the PM** (decided at rung 1: §15.5.5 states the two declarations as one rule, and CSS Lists 3
  §4.6 keys the implicit increment on every list item, so excluding `summary` from the predicate would be a second
  key against the spec — and against One issue, one way): (a) whoever lands either form lands
  `counter-increment: list-item 0` in the same rule; (b) if a form carrying `display: list-item` without it is in a
  UA sheet at LC-2k's base, LC-2k adds the declaration to that rule in its own change (for the shadow form a
  cross-program edit of CV-details' sheet: who makes it is OQ-6); (c) whoever lands second pins
  the form's cell — W-SUM for the document form, W-SUM-D for the shadow form (§6): LC-2k if the rule is in its sheet
  at LC-2k's base, otherwise the rule's lander; (d) before LC-2 the `counter-increment` half is dropped by N-1 (it
  declares no `content`) and is harmless only because the list-item key is still the tag — so the rule must not be
  used as a reason to move the key earlier. To carry to: CV-details (the shadow form, now), and the eventual lander
  of IBP's slot member, through that slot's registration (`IBP-ua-display`'s plan) (OQ-4).
- **F-16 — cross-lane, for the PM: L1's G′a-geo page children and L1's P1 (#785) against LC-1, LC-3 and LC-4.** PM 1008
  (`PORTFOLIO-archive-2026-W48.md`; recorded on this slot's ledger row, `project_open-defer-slots.md:1482`; the
  boundary text is `handoff/codex-lanes/l1/gageo-recut-pm1007.md`, "L2 と共有する範囲") draws the L1 ↔ L2 line:
  whether a line exists, decoration, text transforms, and **the list UA rule and counter order** are L2's (this
  slot); source-position preservation, page events and geometry replay are L1's, and for 3b.3 / 3b.4 "ページ側は既存
  counter 入力を消費する" (page count and continuation are observed through the counters). What the two lanes share,
  at `7fcaffed`:
  (1) the paged driver — `crates/core/elidex-render/src/builder/paged.rs` (`CounterState::new` `:47`, `:185`;
  `set_counter("page" | "pages")` `:50`–`:53`, `:215`–`:216`; `flatten` `:270`) and
  `crates/core/elidex-render/src/builder/walk.rs` (the bake's call `:134`, the paged gate `:131`). LC-1 changes
  that gate and step (one gate and one step for both drivers, I9) and LC-4 re-audits `flatten` and `set_counter`.
  L1's open PR #785 (`layout-block-fragment-contributions`, open on 2026-11-26) edits both files. #785 is not one
  of PM 1008's page children: it is **L1's P1**, the shared Block break token's coverage, which PM 1113 made a
  standalone prerequisite PR before 3b.2f / 3b.2g (its plan delta ruled by PM 1114; the branch is named at W48's
  #782 landing line), and PM 1123 makes P1 + P2 one landing unit (`PORTFOLIO-archive-2026-W48.md`; under PM 1113
  the PM checks #785 against L2's `elidex-layout-block` branch before the merge go). PM 1008's boundary is F-16's other half: it is what 3b.3's
  page plan ("page counter と entry 数が一致") and 3b.4, which read `page` / `pages`, rest on;
  (2) the print entry — `prepare_print_style` (`crates/css/elidex-style/src/page.rs:207`) and the shell's
  `build_paged_pipeline` (`crates/shell/elidex-shell/src/pipeline.rs:401`), which rows 4 – 5 of LC-3's entry table
  cover and which L1's 3b.1 reshaped (#773).
  The branches plan-review R3 named as unmerged L1 work (`css-impl-prepared-page-snapshot`,
  `css-impl-print-style-preparation`, `layout-impl-page-source-cursor`) are **merged** (#773, #772, #774; each merge
  commit is an ancestor of `01426656`: `git merge-base --is-ancestor`), so this memo's anchors already include
  them; their remote branches remain. **Obligations** (no order is derived here — no cell of §5.2 is an L1 cell, and
  the PM sequences the lanes): (a) LC-1's and LC-4's plans re-grep `builder/paged.rs` and `builder/walk.rs` at their
  base and name the L1 children landed or in flight there; (b) each states any change it makes to what
  `set_counter("page" | "pages")` and `flatten` give the margin boxes, with the cells L1's 3b.3 / 3b.4 would see
  (W-PG1, W-PG2 are this program's); (c) LC-3's plan re-reads the print entry at its base (§5.3 LC-3).

**Scout questions (facts the slices' plans need; not blocking this memo):**

- **SQ-1**: `elidex-dom-api` depends on `elidex-style`, so `elidex-style` cannot call `parse_integer`
  (`crates/dom/elidex-dom-api/src/element/numeric_reflect.rs:88`). Which crate below both can hold the one HTML integer
  parser, and who else calls the four copies of F-7? The same crate question holds for `OL_TYPE_VALUES`
  (`crates/dom/elidex-dom-api/src/element/enumerated_reflect.rs:155`), which LC-5's `ol[type]` hint shares (§5.3
  LC-5).
- **SQ-2**: `list-style-type` is served to `getComputedStyle` by `elidex-css-text`'s handler (`lib.rs:287`) and
  `content` by `elidex-css-box`'s (`lib.rs:491`). Which handler crate is the home for `counter-*`, and where do the
  shared resolve functions live so that both the cascade and the handler call them (D-10)?
- **SQ-3**: can `elidex-wpt` run a directory subset such as `css/css-lists`, and which counter tests does it load?
  (Its harness resolves through the hint-less entry, §5.3.)
- **SQ-4**: every test that builds `ComputedStyle` counter fields by hand to stand in for the bake
  (`git grep -n -c -E 'CounterResetEntry::|counter_increment: vec|counter_reset: vec|counter_set: vec' -- crates`
  at `aeecf9b0`: `elidex-render/src/builder/tests/counter.rs` 20, `tests/display_types.rs` 4, `tests/paged.rs` 1,
  `counter.rs` 16, `generated_content.rs` 1, `resolve/box_model/mod.rs` 2) — which assert the bake's shape rather
  than numbering?

---

## §8. Open questions for the PM

- **OQ-1 — P-1 (hint placement), with the designation reading it rests on.** Two linked questions for E0's
  ratifier; the proposal also amends the anchor design (A0) §3.2b's "UA sheet + presentational hints on/off" and
  touches IBP's ratified premise and its applications (IBP §0.2, §0.3, §0.4, §0.6 item 3), so the routing names all
  three records.
  (a) *The reading* (§1.2). Which does elidex hold? This memo does not choose; each reading with the precedents
  that keep it and the ones it overturns (§1.8 has the full list):

  | Reading | Precedents for it | Precedents it overturns or strains |
  |---|---|---|
  | **DR-1** every mode designated, whole section | IBP §0.2 and its applications; the form-control slot's "now also affects `BrowserCore`" | E0 (both halves of its gate); #408's compat half; CLAUDE.md three-layer rule for styles; design `01` §1.2 |
  | **DR-2** the designation is the engine mode | E0 (its gate as the designation switch); #408; CLAUDE.md three-layer; A0 §3.2b; design `01` §1.2 and `07` §7.6 (status classification, which the core / compat split follows) | IBP §0.3 / §0.4 / §0.6 item 3 (their "must" would not reach the core modes whose predicate they decide) |
  | **DR-3** no mode designated | E0; #408; CLAUDE.md three-layer; design `01` §1.2 and `07` §7.6 | IBP §0.2 and all its applications |
  | **DR-4** every mode held to §15, less a compat carve-out | IBP §0.2 and its applications (it is the reading they apply); #408, both halves; CLAUDE.md three-layer (the carve criterion); design `01` §1.2 and `07` §7.6 (status classification); the form-control slot's "now also affects `BrowserCore`" (§1.2 (ii): DR-1's or DR-4's side) | E0's hint half, for current-feature hints — unless every hint is carved (option A) |

  (b) *The option* (§1.6): A / B0 / B / C. **Recommendation: B, under DR-2, DR-3 or DR-4; C under DR-1.** Ground,
  stated per reading (§1.5): under DR-2 / DR-3, P-1a stands on rung 1's "should" (CSS Lists 3 §4.6) with rung 2
  finding no ground to set it aside, and P-1b on rung 2 (three-layer consistency; One issue, one way with #408);
  under DR-4, P-1a and P-1b's core half follow from the reading's reach applied to hints, in DR-4's force (§1.2: at
  rung 1 with a recorded nonconformance if the ratifier takes DR-4 as a designation with a recorded departure, at
  rung 2 if as no designation plus IBP's applied rule), and P-1b's compat half is the rung-2 carve. Under DR-1 the answer is C and E0 is withdrawn rather than amended. B and B0 amend the E0,
  rustdoc and A0 passages of §1.7. **A is not rejected by rung 1 alone**: it is what remains if the ratifier holds
  E0's "no hints in core" itself as the reason to set §4.6's "should" aside — a reason this memo does not find in
  E0 (§1.3) — and under DR-4 it further needs every hint classed as legacy.
  Blocks LC-3, and through it LC-4, LC-5 and LC-6; LC-5's U12 placement directly. LC-1, LC-2 and LC-2k are not
  blocked.
- **OQ-2 — the core hint source's home** (only if B0, B or C). (X) a module in `elidex-style` beside `ua.rs`, with
  `resolve_styles_with_compat`'s `hint_generator` becoming the compat extra, as `extra_ua_sheets` is for rules; or
  (Y) a hints method on `HtmlElementHandler`, classified by its `HtmlSpecLevel`. Fixed under both: the cascade
  applies the core source itself, at every entry point of §5.3 — it is not a caller's argument. The principles
  split: *Plugin-first* selects Y; *One issue, one way* selects X within this program's reach (UA rules and hints
  are one kind of thing and today have one form; Y for the list hints alone is a second form, and Y for everything
  is the whole UA-defaults migration, F-4). **Recommendation: X**, with F-4 recorded so the migration, if wanted,
  moves rules and hints together.
- **OQ-3 — scope growth.** The slot text does not list N-1 … N-5 or §15.3.7's other units. This memo folds them in
  (LC-1's order fix, LC-2's N-1, LC-3's N-4, LC-4's scope model, LC-5; N-3 falls out of LC-1; rev 3 adds `inherit`
  on `counter-*` to LC-6 and the namespace input to LC-3), because they are the
  same step, the same resolve function and the same spec section, the slot's exit criterion (no
  silent-wrong-in-production slot, PM 620) covers them, and §5.2's constraints tie their order to the slot's own
  work (c1, c2, c4, c6, c11). **Recommendation: approve the fold**; the alternative is separate slots for N-1 / N-2
  and
  for LC-5, which §5.2 would still order between this slot's slices.
- **OQ-4 — owners and records for the findings.** New slots proposed, each drafted in §7 with its fields: F-2
  (`#11-dir-attribute-ua-rule-in-core`), F-6 (`#11-text-align-match-parent`), F-9
  (`#11-ua-sheet-flow-relative-box-properties`), F-10 (`#11-css-property-handler-dual-implementation`). Facets of
  existing slots instead of new ones: F-3 → `#11-css-wide-revert-keyword` (L1); F-7 →
  `#11-form-numeric-attr-html-integer-parsing` (no owner on its row: the PM assigns one); F-14 →
  `#11-html-element-type-tests-without-namespace` (L3). Input lines: F-5 and F-12 → `IBP-ua-display`; F-15 →
  CV-details (the shadow form) and `IBP-ua-display`'s plan, for the registration of its slot's `summary` member;
  §4.3 U13's three quirks rules → `#11-document-mode` (Layout); F-16 → the L1 lane and the PM's sequencing of L1's
  page children against LC-1 / LC-3 / LC-4; F-11 → the cite-sweep lane. Whether F-4 gets a record, and whether F-8
  (no defect today; its trigger is §0.5 row 4's promotion rule) should. And where §9 item 4's amendment (or
  divergence) record goes when this slot closes: proposed, the closing PR's row of `memory/m4-12-landings-ledger.md`,
  with E0's back-pointer re-pointed (§9 item 4 (v)).
- **OQ-5 — under option A only**: LC-3 would make `BrowserCore` / `App`, and every hint-less cascade entry (the
  WPT harness, the benches, the print entry's fixture, the tests that call `resolve_styles` or
  `prepare_print_style`), stop honouring `start` / `value` / `reversed`, which they honour today — the one
  right-to-wrong change in §5.2's tables. Is that an accepted, cell-pinned divergence, or does it need its own
  record? Two existing slots bound the answer: `#11-async-core-storage-cookiestore` (no production session selects
  those modes until it lands, E0 §2 Axis 3), and `#11-enginemode-full-session-threading` (iframe builders and the
  navigation rebuild hard-code `BrowserCompat`, so once a core session exists the same list is numbered from 1 in
  the top-level document and from its `start` in an iframe until that slot lands). Under A the divergence would be
  recorded against both.
- **OQ-6 — LC-2k's conditional edit to another program's UA sheet.** §7 F-15 (b) has LC-2k add `counter-increment:
  list-item 0` to a §15.5.5 `summary` rule that is in a UA sheet at LC-2k's base with `display: list-item` alone.
  For the document form the sheet is one this program already edits (`ua.rs` / `legacy_ua.rs`); for the shadow form
  it is CV-details' UA shadow sheet (`docs/plans/2026-10-content-visibility.md`; its file is CV `:986`'s placeholder
  `<the UA shadow sheet's file>`), which CV-details' plan names and owns — so the edit is a cross-program touch.
  What the rule must say is fixed at rung 1 under every option (§15.5.5 states the two declarations as one rule;
  excluding `summary` from the predicate is rejected, F-15); what is open is who edits the file. Cell: W-SUM-D
  (`<ol><li><details>x</details><li>b</ol>`; spec 1, 2; 1, 3 with `display` alone once LC-2k has landed).
  (a) *LC-2k edits CV-details' sheet* in its own change. Rung 1: the rule is whole from LC-2k's change on, so
  W-SUM-D is 1, 2 in every window whichever lands first. Cost: one program edits a file another program owns, and
  CV-details' plan must take in a declaration it did not land.
  (b) *CV-details lands the full rule* (`display: list-item` and `counter-increment: list-item 0` together), as
  F-15 (a) and §9's CV-details input line ask. Rung 1, with rung 2's One issue, one way (one lander for one rule);
  LC-2k's conditional edit then never fires for the shadow form, and W-SUM-D is 1, 2 in every window (§5.2's
  cross-lane table, "rule whole"). Cost: CV-details takes a declaration beyond the scope its plan states (CV
  `:880`: it builds "only" the default summary's display).
  (c) *The PM sequences it*: the PM orders CV-details' shadow sheet against LC-2k and names, at that point, who adds
  the declaration. No rung decides it: it is a lane-sequencing decision, F-16's kind. W-SUM-D is 1, 2 if the rule is
  whole once both have landed; if the `display` declaration is in the sheet alone after LC-2k, W-SUM-D reads 1, 3
  until the declaration lands (§5.2's cross-lane table, "`display` alone").
  This memo does not choose. It blocks no slice's start; LC-2k's plan needs the outcome only if the shadow sheet is
  in the tree at its base without the declaration.

---

## §9. Ledger records (applied by the PM / pmo; nothing here is applied by this PR)

- Slot `#11-list-counter-baking-ua-rule-and-hints`: plan = this memo; first task (PM 620) done, results §0.2; the
  slot's "Owed by PM/pmo" back-pointer is already present (PE-2); PE-1 and PE-3 recorded against the slot text.
- The CS cite-sweep lane's semantics rows 2720 and 2721 (`counter-increment` vs `counter-set` order,
  `elidex-style/src/counter.rs`) are attached to this slot with no new routing
  (`handoff/codex-lanes/cs/routing-2026-11-24.md`; `PORTFOLIO-archive-2026-W48.md`): **discharged by LC-1's order
  fix** (D-3, c8), which also removes the two pointer comments (`counter.rs:81`, `generated_content.rs:131`).
- Master plan §10 (c) (`memory/render-layout-inline-convergence-plan.md`): closes with **LC-2k**. The item is "implicit
  `list-item` counter keys on the `<li>` tag, not `display:list-item`" — the key, which LC-1 moves unchanged and
  LC-2k changes (rev 3 wrote LC-1, from before the key left LC-1; R3 B5-I2).
- **Conditional on OQ-1's outcome — if B0, B or C is ratified**: an amendment record for E0
  (`docs/plans/2026-06-e0-shell-style-compat-mode.md`) and for the anchor design A0 §3.2b
  (`docs/plans/2026-06-web-api-compat-split-design.md:516`), in IBP's idiom "recorded, not edited" (IBP §0.3's
  heading; A114): one record naming the ratifier's decision, the option and the reading it was taken under, and
  the passages of §1.7 that no longer hold as written. **Where** (neither plan has a ledger of its own, so the
  record goes where this program's decisions already are, and the plans' readers are pointed at it — One issue,
  one way: one record, the rest pointers; the spec is silent here): (i) the record is a paragraph of this slot's
  row in the open-slot ledger (`project_open-defer-slots.md:1482`), headed with both plan paths so that a search
  for either file name finds it; (ii) E0's back-pointer goes on its landing line in the landings ledger
  (`m4-12-landings-ledger.md:224`, the #410 line that done-ifies E0), naming that paragraph; (iii) A0 has no
  ledger or landing line (`/usr/bin/grep -c web-api-compat-split-design <memory dir>/m4-12-landings-ledger.md` → 0 on 2026-11-26;
  control `/usr/bin/grep -c -F '#406'` on the same file → 1),
  so (i)'s heading is its pointer; (iv) the ratifier's decision itself is the PM's decision line in the PORTFOLIO,
  cited by number in (i); (v) **the record outlives the slot**: (i) sits on an open-slot row, and a closed slot's
  row leaves that ledger (the ledger's header banners record each closure as an entry removed), so the paragraph
  must not go with it — when the PM closes the slot, the paragraph moves unchanged to the landings-ledger row of
  the closing PR (`memory/m4-12-landings-ledger.md`), and E0's back-pointer of (ii) is re-pointed to it there. This
  destination is **pending the PM's ruling** (sent with §8's open questions, OQ-4). The two plan files are not
  edited; the rustdoc and comments of §1.7 are edited by LC-3.
  If A is ratified: no amendment record; OQ-5's divergence record instead, in the same place (i). Whatever the
  option, the reading the ratifier names is recorded once, in (i), against IBP's premise (§0.2), and goes to
  `IBP-layout` and `IBP-ua-display` as input lines (below), since they consume it.
- `IBP-layout` input lines: P-1's outcome and the designation reading (F-1); `IBP-ua-display` input lines: `menu`
  (F-5); HTML §16.3.3's five equivalences, `dir` among them (F-12); the `summary` rule's unit and pin (F-15), for
  the registration of its slot's member; CV-details input line: the shadow form's unit and pin W-SUM-D (F-15).
- `#11-document-mode` input line: §15.3.7's three quirks-mode list rules (§4.3 U13).
- F-16: no ledger write — PM 1008's boundary is already on this slot's row (`project_open-defer-slots.md:1482`);
  the PM sequences L1's page children against LC-1, LC-3 and LC-4 when their plans name them.
- Cite-sweep lane input: F-11.
- Proposed, pending OQ-4: new slots for F-2, F-6, F-9, F-10; facets of `#11-css-wide-revert-keyword` (F-3),
  `#11-form-numeric-attr-html-integer-parsing` (F-7) and `#11-html-element-type-tests-without-namespace` (F-14).

## §10. Definition of done (the program)

- `git grep -n -E 'apply_implicit_list_counters|count_li_children' -- crates` → 0 hits.
- Every cell of §6 holds its spec value in the mode(s) OQ-1's outcome names; each slice's plan carries its cells as
  tests, the "stays" cells of its window included.
- The list-item predicate has one definition and its body reads computed `display` (LC-2k); no reader of "is a
  list item" for counters keys on a tag.
- OQ-1's outcome is recorded (§9, item 4): under B0 / B / C the E0 / A0 amendment paragraph exists on this slot's
  ledger row, headed with both plan paths, with E0's back-pointer on its landings-ledger line; under A the
  divergence record of OQ-5 exists there instead; the reading is recorded there against IBP's premise. The record
  outlives the slot: when the row is closed, the paragraph is on the closing PR's row of
  `memory/m4-12-landings-ledger.md` and E0's back-pointer names it there (§9 item 4 (v); destination pending the
  PM's ruling).
- `git grep -n 'list-counter-baking' -- crates` → 0 hits (the pointer comments are gone with their defects), and
  §5.3's concept sweep leaves no comment that describes the bake or a cascade-time count.
- `getComputedStyle` returns the computed `counter-*` values (P10 / Q10), and `counter-*` have one parse and one
  resolve implementation (D-10).
- The slot row is closed by the PM with the last slice's merge; §9's records are applied.

## Changelog

- **Rev 3.2 (2026-11-26)** — folds the Step 4.5 focused re-review of rev 3.1 (`4fd8752a`; CRIT 0 / IMP 0 / MIN 7,
  C-M1 – C-M7, plus two clerical notes). Each finding was checked against its evidence before it was applied; none
  was wrong. No slice reordered, no section restructured. Code refs re-verified at `4fd8752a` (`git diff --stat
  01426656 4fd8752a -- crates` is empty).
  - **C-M1** — §5.2 c9, "What the constraints force", "Must LC-1 divide?": the pair that co-lands under the
    own-sign mechanism is {the end of the guard and push, the sign change}, not {the move, the sign change}. Re-derived
    from `counter.rs` (the bake's `li` arm pushes `list-item 1` into the computed increment, `:364`; the step negates
    every increment on a reversed counter, `:117`) and Table 2's P4a / P1 / Q19 rows: while the push stands, D-2's
    suppression test withholds the moved increment, so the move never meets the sign change. Every order c9 states
    stays an order.
  - **C-M2** — §5.3 LC-1: F-16's obligations (a) and (b), for `builder/walk.rs` (`:131`, `:134`) and
    `builder/paged.rs`, as LC-3 and LC-4 carry theirs.
  - **C-M3** — §7 F-16: #785 cited by its own PM decisions — L1's P1, PM 1113 (standalone prerequisite; the PM's
    check against L2's `elidex-layout-block` branch), PM 1114 (its plan delta), PM 1123 (P1 + P2 one landing unit),
    from `PORTFOLIO-archive-2026-W48.md`; PM 1008's boundary kept as F-16's other half; heading names #785.
  - **C-M4** — §9 item 4 (v), §10, OQ-4: the amendment (or divergence) record outlives the slot; at closure it moves
    to the closing PR's row of `memory/m4-12-landings-ledger.md`, with E0's back-pointer re-pointed; destination
    pending the PM's ruling.
  - **C-M5** — §5.1 LC-2k's "What it lands", §5.3 LC-1 and LC-2k ("nothing else" qualified), §5.2's cross-lane
    paragraph, §7 F-15 (b): LC-2k's conditional UA-sheet edit stated, and flagged as a cross-program touch of
    CV-details' UA shadow sheet; new OQ-6 with options (a) / (b) / (c), each with its rung and W-SUM-D; not chosen.
  - **C-M6** — D-1: a hand-built-style caller loses the reset from W1 on, permanently (from LC-3 the reset is the
    UA rule's, which a hand-built style bypasses); rev 3.1's "in W1 – W2k" implied it returned at W3.
  - **C-M7** — §1.2's DR-4 carve cell and its #408 row narrowed to `dir`; `acronym`, `listing`, `plaintext` and
    `xmp` stay `IBP-ua-display`'s rung-1 input, as F-12 says.
  - Clerical — D-4: CLAUDE.md names the literal port of the boa `HostBridge` side-store as the rule's typical trap,
    not its origin. §5.2 c13: the cite of `box_model/mod.rs:529` states its condition (`resolve_counter_reset`
    encodes an absent integer of an author `reversed(name)` as 0).
- **Rev 3.1 (2026-11-26)** — folds `/elidex-plan-review` round 3 on rev 3 (`7fcaffed`; CRIT 0 / IMP 5 / MIN 23, 21
  unique; change type DESIGN-AFFECTING, localized: no slice reordered, no section restructured). R3 finding IDs are
  the reviewers' (B*n*-I*n* / B*n*-M*n*). Every finding was checked against its evidence; two were partly wrong on a
  fact, noted at B5-I3 and B5-M1. Code refs re-verified at `7fcaffed` (`git diff --stat 01426656 7fcaffed --
  crates` is empty). No probe was run: the new cells are by reading (§0.1, §6); Q19 – Q21's values are from probe
  2's log.
  - **B2-I1** (IMP) — D-4, §0.5 row 4, I6, §5.3 LC-1: the *Side-store → component* rule is read at its precondition
    (state held beyond one computation): a component when the value crosses from the pre-layout pass to the paged
    walk or outlives the run; otherwise run-local scratch under any key, or the counter instance. Rev 3's "not
    allowed, walk-local or not" withdrawn. **B2-M4** — the same places: a component carries I11's insert-or-remove
    on every exit path (gate-out, `:105`, `:114`, `:58`).
  - **B4-I1** (IMP) — §1.2's force row names DR-4's two forms (rung 1 with a recorded nonconformance; rung 2), and
    every "under DR-4" ground carries them: §1.2's D-6 and #408 rows, §1.3, P-1a, P-1b, §1.5's table, §1.6's Rung
    row, OQ-1 (b).
  - **B5-I1** (IMP) — §7 F-15 rewritten to the facts: §15.5.5's two forms (document, `:host summary`) each one rule;
    IBP's unregistered slot holds the `display` declaration alone and needs `:first-of-type`; CV-details builds the
    shadow form's display in a UA shadow sheet; the `counter-increment` half is in no plan. Constraint decided at
    rung 1 (one rule in §15.5.5; excluding `summary` would be a second key): land whole; LC-2k adds a missing
    declaration rather than exclude. §5.2's cross-lane paragraph and table, §3's §15.5.5 row, §3.2's quote (shadow
    form), §5.1 / §5.3 LC-2k (every UA sheet, the shadow sheet named), §6 W-SUM-D, OQ-4, §9.
  - **B5-I2** (IMP) — §9: master plan §10 (c) closes with LC-2k.
  - **B5-I3** (IMP) — new §7 F-16: PM 1008's L1 ↔ L2 boundary, the shared files (`builder/paged.rs`,
    `builder/walk.rs`, the print entry) and obligations on LC-1 / LC-3 / LC-4; §5.3 LC-3 and LC-4; OQ-4; §9. **Fact
    corrected**: the three L1 branches the finding called unmerged are merged (#772 – #774, ancestors of
    `01426656`); what is in flight is L1's later children (open PR #785).
  - **B1-M1** — §5.3 LC-5, §4.3 U12, SQ-1: `OL_TYPE_VALUES` shared, home below both crates. Side note: F-7's cite
    `:65` → `:88` (and the canvas cite `:256`, a test, → `:170`).
  - **B2-M1** — §5.3 LC-1: the serializer `to_css_string` and the two resolve loops as readers of the encoding; cell
    W-ML (§6, §5.1). **B2-M2** — I5, I5 × I11, §5.1 / §5.3 LC-2k, Table 2: the marker reconcile reads the predicate
    from LC-2k. **B2-M3 / B3-M3** — D-1's per-window sentence, Table 1's gate row, §5.3 LC-1: the fixtures that rely
    on the tag gate are named and updated; no second key.
  - **B3-M1** — c9 restated as an order (move ≼ guard and push end ≼ sign change), with the mechanism under which
    the move and the sign co-land; "What the constraints force" and "Must LC-1 divide?" follow. **B3-M2** — c13
    (§4.4.2 ≼ the `ol[reversed]` arm's absent integer; P4a); D-12. **B3-M4** — §0.2: P19, P20, Q19 – Q21 with
    markup and measured values; §6 / §5.1 add Q21's field. **B3-M5** — U13 → `#11-document-mode`. **B3-M6** — F-7 →
    facet of `#11-form-numeric-attr-html-integer-parsing`; F-8: no record, trigger and receiver stated; both in
    OQ-4. **B3-M7** — F-14's facet fields. **B3-M8** — §7's lead: prose by choice; checker exit 2 = unscanned.
  - **B4-M1** — §1.2's DR-4 carve excludes §16.3.3's equivalences. **B4-M2** — U12: `dir[type]` mirrors `ul[type]`;
    F-13 (b). **B4-M3** — §3's breadth line: at most three specs (LC-6). **B4-M4** — §3's §4.2 row, §3.3's
    `initial_value` / `is_inherited` rows. **B4-M5** — the §4.5 quote anchored in the F7 note; §3's heading lookups.
  - **B5-M1** — §1.7's #408 row ("kept, with a second exception") and §5.3 LC-3 (the comments it edits). **Fact
    corrected**: the comments do not become untrue — they become incomplete (they name `tt` as the one exception),
    and the finding's `ua.rs:110`–`:113` is `:111`–`:116`. **B5-M2** — §1.8: IBP §0.6 item 15; the umbrella's
    `:262`–`:266`; IBP `:1688` as ratified; §1.6 A's cost (IBP §2 I1's color well); §1.2 (i) cites E0 §6 Q2.
    **B5-M3** — OQ-1 (a)'s table. **B5-M4** — §9 item 4 and §10: the record's place (this slot's ledger row; E0's
    landings line; A0 has none). **B5-M5** — §5.3 LC-3: IBP-sandbox deletes `build_paged_pipeline`.
  - Provenance side note — full paths in the UA-sheet greps (§4.3; §5.2; F-12).
- **Rev 3 (2026-11-26)** — folds `/elidex-plan-review` round 2 on rev 2.2 (`aeecf9b0`; CRIT 0 / IMP 9 / MIN 21;
  change type STRUCTURAL). Round-2 finding IDs are the reviewers' (A*n*-I*n* / A*n*-M*n*; A5s = the successor
  reviewer of axis 5). Every finding was checked against its evidence before use; none was a false positive. Two
  citations inside findings were replaced by the record itself (noted at A5-M4, A5-M5). No probe was run for this
  revision: cells it adds are marked "by reading" (§0.1, §6).
  - **What changed in substance.** (1) **Order**: the list-item key's change from tag to `display` leaves LC-1 and
    becomes its own slice, **LC-2k**, after N-1's removal — seven slices; constraints c11 and c12 are new, c5 and c9
    restated; §5.2 now tables the units each slice *adds* as well as the units at `bf1cae35`. (2) **§1**: a fourth
    reading, DR-4, is tabled — the reach IBP's ratified passages apply. The recommendation stays **B** but is no
    longer "B on DR-2": it is B under DR-2, DR-3 or DR-4 (C under DR-1), with the ground rung stated per reading.
    "DR-2 is the only reading consistent with the ratified records" and "rung 1 rejects A" are withdrawn: no
    reading fits every record, and against A rung 1 sets a "should" which rung 2 finds no ground to set aside.
    (3) **`dir`** resets with `ul` in the core sheet in every mode (rung 1, HTML §16.3.3), under every reading.
    (4) **LC-6 opens a window** (`inherit` / `unset` / `initial` resolve through the handler).
  - **A3-I1** (IMP) — §5.2: method restated over two populations; c11 with cell W-F4; the three ways to satisfy it
    weighed, (α) chosen; LC-2k added to §5.1, §5.3, §6, §3.3, §4.1, D-2, D-12, I5; IBP's
    `#11-ua-sheet-html-rendering-display-fidelity` ordered against the windows (§5.2 cross-lane table, cell W-SUM)
    and routed to the PM (§7 F-15, §9). **A3-M6** — §5.2 "What the constraints force" and "Must LC-1 divide?".
  - **A5-I1** (IMP) — §1.2 re-derived with IBP §0.3 / §0.4 / §0.6 item 3 (and `:1500`–`:1501`, `:1546`, `:1688` in
    §1.8): reading DR-4, a "force of the must" row, the IBP-applications row; PE-3, §1.3, §1.5, §1.6 and OQ-1 (a)
    restated; OQ-1 (a) carries all four readings with the precedents for and against each. **A5s-M1** — §1.8's `tt`
    row: IBP keeps the reach exception as a placement input; it does not arise for hints. **A5s-M2** — §1.2 and
    §1.8 cite `#11-form-control-ua-rendering-fidelity` and `#11-enginemode-full-session-threading`. **A5s-M3** —
    §1.2: IBP decides §15-versus-css-ui-4 conflicts by probe; none arises here.
  - **A4-I1** (IMP) — §1.3, §1.5 (P-1a's ground), §1.6 (A's "Rung"), OQ-1 (b): "sets a should against A; rung 2
    finds no ground to set it aside"; E0's "no hints in core" examined as the candidate reason (§1.3).
  - **A4-I2** (IMP) — §3.2: the "nonetheless part of the computed value" text is marked as a Note and quoted
    apart from the normative sentence; the F7 note states the conflict and decides at rung 2 (ECS-native first:
    one writer of the style component); D-4 follows. F-13 (a) kept.
  - **A4-I3** (IMP), **A3-M5** — D-6, §4.3's `dir` paragraph and U7, c10, W-H5 (§0.2, §6), §5.3 LC-3 / LC-4 /
    LC-5: `dir` gets `ul`'s rules in `ul`'s sheet in every mode; the reading-dependent placement is removed, and
    with it the unlisted right-to-wrong change. F-12 widened to §16.3.3's five equivalences.
  - **A2-I1** (IMP) — D-10, I1, I1 × I10, §5.2 (Table 1's `inherit` row, Table 2's handler row, "LC-6 does open a
    window"), §5.3 LC-6 (round trip; `initial_value` / `is_inherited`) and LC-1 (one encoding shape), §6 W-K1 –
    W-K3.
  - **A2-I2** (IMP) — D-4 and §0.5 row 4: the criterion is CLAUDE.md's *Side-store → component* rule; the two
    drivers are named as two systems; the storage shape is LC-1's under that criterion; an entity-keyed precompute
    that is not a component is not allowed. I6, I6 × I9 and §4.1 follow.
  - **A5-I2** (IMP) — §9: the conditional E0 / A0 amendment record with back-pointers ("recorded, not edited");
    §10: its DoD line.
  - **A5-I3** (IMP), **A4-M1** — §7 F-3: proposed as a facet of `#11-css-wide-revert-keyword` (owner L1, PM 470)
    instead of a new owner-less slot; Cascade 5 §6.5's second paragraph quoted (§3.2) and F-3's premise restated as
    resting on a reading of HTML §15.2's phrase.
  - **A1-M1** — §5.3: `html_olist_proto.rs:38`–`:41` in LC-1's sweep; a concept grep for every slice; §10.
    **A2-M1** — D-7, I2, §0.5 row 1, §4.1: the hint input is (`TagType`, `Namespace`, `Attributes`); W-NS; §7 F-14
    routes the existing blindness to `#11-html-element-type-tests-without-namespace`. **A3-M1** — I6 × I7 moved to
    the meeting pairs (32 / 23); one boxless predicate read by the gate and by the §4.4.2 walk; W-BX. **A3-M2,
    A5-M5** — every proposed slot in §7 carries name, Gap, Why, Trigger, Owner, Re-eval-date; F-10 and LC-6 cite
    L1's typed-shorthand-handlers plan (the plan file itself, `docs/plans/2026-11-typed-shorthand-handlers.md` §1;
    the line the finding gave, in the Flex-parser prerequisite memo, does not state the trait change). **A3-M3** —
    §3.1: "saturating as today" replaced by the four operations as they are, the §4 clamp quoted, obligations per
    operation on LC-1 and LC-3; W-OV. **A3-M4, A2-M2** — D-1 restated per window (two writers until LC-3).
    **A4-M2** — the "user option" gloss removed from DR-2. **A5-M1** — §1.8: `07` §7.6, `01` §1.2, A0 `:517`–`:521`.
    **A5-M2** — §1.8: the audit and the philosophy-alignment umbrella. **A5-M3** — §1.7: five rows (E0 Axis 1, E0
    §6 Q1, `style_compat_policy` rustdoc, the `elidex-dom-compat` crate doc and description, the parser test
    comments). **A5-M4** — PM decisions cited by number and archive file; sweep counts re-measured (§1.8's search
    block), new hits classified; the memory memo is cited by its heading (the line the
    finding gave has moved again). **A5-M6** — §9: CS rows 2720 / 2721 discharged by LC-1. **A5-M7** — OQ-5 and
    §1.6 name `#11-async-core-storage-cookiestore` and `#11-enginemode-full-session-threading`.
  - Also re-measured at `aeecf9b0`: F-9's control (256 lines; rev 2's 211 was stale), SQ-4's counts, LC-0's line
    numbers (`counter.rs` 799 lines, tests from `:414`).
- **Rev 2.2 (2026-11-26)** — §5.3 LC-3's entry table corrected to main `01426656` (no change to its conclusion or to
  the recommendation): the walk row restated for the flat-tree walk (one `resolve_and_attach_style` site, one
  `hint_generator` call in `compute_element_style`, `preliminary_root` with hints and no bake); two rows for the
  print entry `prepare_print_style` (compat arm honoured, core arm **lost** under A); the `cfg(test)` callers of
  `resolve_styles` in `elidex-dom-api` and the layout crates added; the census grep re-run and widened to the print
  entry (84 → 92 lines, classified). §1.4's testing line ("the two core arms") and OQ-5's list of hint-less entries
  follow.
- **Rev 2.1 (2026-11-26)** — re-anchors the `file:line` refs into code to main `01426656` (merged at `2e922681`);
  no design change. Moves only: `counter.rs` (+1 for `:277`–`:307`, −1 from `:312`), style `walk.rs` (the bake's call site `:168`),
  `box_model/mod.rs`, `cascade/mod.rs` (hint source `:231`), `traits.rs`, style `lib.rs`, `resolve/mod.rs`,
  `declaration.rs` (the built-in table is now `builtin_parser`; registry fallback `PropertyGrammar::select`),
  `values.rs` (`parse_global_keyword` `:5`); the `CounterState` reader list gains `builder/paged.rs` (split out of
  `builder/mod.rs`); F-3's two control counts re-measured (`@media` 36, `Origin::UserAgent` 12). The `li` arm's
  increment push is cited at its own line (`:364`; rev 2's `:363` sat two lines above it). Not re-anchored, pending
  the PM: §5.3 LC-3's walk row (the walk now has one `resolve_and_attach_style` site and one `hint_generator` call
  in `compute_element_style`) and the entry table's coverage of the print entry `elidex_style::page::prepare_print_style`.
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
