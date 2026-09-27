# Predicate prereq program: one canonical answer to css-display-3's *inline box*

**Revision 37** (2026-09-28) — folds Codex R7 on #526 (`a42ca1fe`; IMP 3). (1) F1 encodes §0.6 item 3's conclusion:
it answers false for a button-layout element (F7's class) whatever its computed `display` — §15.5.3 coerces an
inline `button` to `inline-block` and gives every button-layout element a new formatting context. (2) Resize Observer
§3.4.8's `device-pixel-content-box` branch is ✗, routed to the existing `#11-resize-observer-device-pixel-box`
(no device-pixel input). (3) F8 is always built: devolution decides native vs primitive appearance for the
rendering condition whatever the probe finds about replacedness; only F4's read of it stays probe-conditional.
Self-review (IMP 3 / MIN 5, folded): IBP-layout lays out §15.5.3's coercion and new formatting context (no code
does today) so layout agrees with F1, and owns the devolved rendering (measured for button layout); G8's
author-origin exposure is an IBP-classify obligation; `uses_button_layout` named and pinned; the device-pixel arm's
CSS-pixel fallback stated and booked in the slot (§6).

Revision 36 (2026-09-28) — folds Codex R6 on #526 (`f3d597ca`; IMP 2, MIN 1; R5 on the same head was dry).
(1) The hidden-`embed` witness drops its author sizing (self-introduced at rev 34): an author declaration or the
`width`/`height` attribute hints beat HTML §15.3.1's normal UA-origin rule, so the cell is a bare `<embed hidden>`,
✗ at IBP-layout unless the rule lands in the switch's cut — IBP-layout builds it (the rule, not a 0×0 natural size, is the repair). (2) IBP-observer owns the observed box's writer
transition: re-`observe()` replaces the observation (Resize Observer `observe()` steps 1–4), which
`ResizeObserverRegistry::observe` does not today. (3) IBP-sandbox's `host_data/mod.rs` line states the touch-time
rule's application (the touched concern is deleted whole; the full decomposition is the user's option-A slot) instead
of "only shrinks" (self-review: IMP 1 / MIN 4, folded). A >1000-line sweep of every `.rs` path the memo cites found eight; those known to be edited are the two
split prereqs' files and `host_data/mod.rs`, `element/tree.rs` is one if an F11 caller edit reaches it, and the rest
are cited as evidence only.

Revision 35 (2026-09-28) — folds Codex R4 on #526 (`ee3badbe`; IMP 2, MIN 1). (1) §3's transform rows
(css-transforms-1 §2, css-transforms-2 §8, css-will-change-1 §2) marked ✓ while the umbrella's rows for the same
sections route their transformable-element containing-block branches to `#11-transformed-block-abspos-double-layout`
/ `#11-transform-family-3d-and-containing-block` — now ✗ with those omitted branches named. Root-check (R3 and R4 both
in the transform area): the rows had summarised the umbrella's as "gated on F12" and dropped their ✗ branches; a
sweep over both populations — every ✓ row here against the umbrella's ✗ rows for its section, and every umbrella §3
row of a section predicate prereq requirement 8 relies on against a row here — found these three and seven sections with no row
(css-transforms-1 §3–§5, css-transforms-2 §5, §7, §9, §10), now added with the umbrella's dispositions (M 43 → 50);
CSSOM View §6's ✗ is the umbrella's hand-off to this program, CSS Pseudo 4 §4.1's to the umbrella's pseudo prereq
(F4's consumer), CSS Content 3 §1's a branch outside this row's step. Follow-up check (N1–N5, folded).
(2) F15 claims only the machinery `IBP-css-machinery`'s population does not hold (§0.6 item 7's late-need clause), so
no feature has two builders. (3) F12's citation names the `#transformable-element` anchor: the draft's Terminology
heading does not render and the definition falls inside §1.2 CSS Values (R3 rejected this half; re-examined).
Cumulative re-gate of revs 33–35 (IMP 4 / MIN 4, folded): the sweep's missing population above; F6's trigger
population (§4.8.4.3.2's relevant mutations) moves to IBP-classify with its writers; §4's node paragraph no longer
decides every node by its gate cell (the plan-memo decides narrow vs edge-dense, CLAUDE.md (c)/(a)); the SVG
omission no longer presumes the probe's answer; item 6 carries item 7's late-need clause; request state is read
through one composition, `image_request_state`.

Revision 34 (2026-09-28) — folds Codex R3 on #526 (`1182c143`; IMP 2, FP 1; R2 on the same head was dry).
(1) F12 carries css-transforms-1 §1.2's box-model clause whole — an inline box, a `table-column` and a
`table-column-group` box are not transformable (elidex has both `Display` variants); the SVG clause is a named
omitted branch (at `IBP-transform`'s base the outermost `svg` is replaced and its descendants get no box), so the
§3 row reads ✗. (2) `embed[hidden]`'s rule (HTML §15.3.1) had no owner — Appendix G's `display`-keyed measurement cannot see
it; HEAD draws no `embed` at all (an empty non-replaced inline), so a hidden one is right by accident until
IBP-layout's F4 switch sizes it — IBP-layout's own cell (§3). F12's widening is recorded as §0.6 item 14 with its
`colgroup` fixed-descendant seed. FP: §15.3.1's `[hidden]` rule is `display: none` without `!important` (`webref
body html hidden-elements`). Self-review (two passes: CRIT 1 / IMP 2 / MIN 4, folded): the first draft routed `embed[hidden]`
to A96 on a false premise ("full size today").

Revision 33 (2026-09-28) — folds Codex R1 on the approval PR #526 (`b59ae926`; IMP 3, FP 1). (1) F6 lands
with its whole writer set: the `src`-mutation transitions move from IBP-layout to IBP-classify (I1 holds back only
the F5 removal, F6 being read by no presence-keyed site), and F4 reads a null source (identity), then F6, before F5, so no state reachable at
IBP-classify's landing — a stale `ImageData` included — gets a wrong replacedness answer (§1 F5/F6, §2 I6 and the
classify×layout pair, §3, §4 table and obligations, Appendix B). (2) §4: each non-terminal row is a node, not a
declared single PR — the edge-dense rule applies recursively in its plan-memo, and the memo's rules then apply per
nested PR; I1 is a lower bound on `IBP-layout`'s switch, whose narrowness its own plan-review decides (an exception
goes to the user). Self-review of the delta (IMP 3 / MIN 5 over two passes, folded): F6's clone row and G5 move with it; a null
source is identity, so step 11 clears F6; the discriminating pin is step 13's; F6's fact is *broken with a non-null source*; its clone row is decided at IBP-classify. (3) IBP-classify's probe must decide replacedness both ways: css-pseudo-4 §4.1 gives replaced ⇒
suppressed only. FP: a no-`alt` broken `<img>` is §15.4.2 rule (2) by its own disjunct ("the element has no alt
attribute").

Revision 32 (2026-09-28) — the user decisions (2026-09-28): (1) a pinned `content-visibility` cell **is** the
exposing PR's own deferral (`feedback_defer_cap_policy.md:12`/`:16`; the cell test makes a right-at-base cell wrong,
which the bidi precedent `:486-490` lacked), one rule before or after the exposer lands — recorded as §0.6 **item 13**
(the umbrella's per-PR counts, conditional: PR-1b 0 → 1, PR-1c 1 → 2, PR-1d 1 → 2) with a §6 approval-PR row;
(2) the program memo goes to a docs-only approval PR with /external-converge as its terminal gate, so this revision
also carries the umbrella ledger rows the approval PR ships (A152–A156). E25's direction rests on the umbrella-
exposure rule (umbrella cells are `IBP-ua-display`'s to build), with an umbrella-side witness; "builder-eligible"
defined. MINs: §4's no-edge paragraph (ownership, not repair; E25 a builder edge); §6's recorder row.

Revision 31 (2026-09-28) — folds the re-check of rev 30 (frozen `cacda2e0`; IMP 3 / MIN 4). The one-builder
rule now counts an umbrella PR's exposure as `IBP-ua-display`'s, and E25 stands on that. A pin is conditional on
its cell firing at the exposer's actual base (the other program's repair may already be there); the frame's "D
held unrepaired" governs ordering only. **Rev 30's cap change is reverted**: a `content-visibility` pin is an
accepted divergence, not a deferral of the exposer — its repair is owned by a program with its own slot and
umbrella (item 12), the umbrella's own criterion (`:486-487`), and the deferral unit is the slot — so the
umbrella's ratified counts (`:2379`) stand and no §0.6 record is needed. MINs: stale repairer wording, "places the
cell's ownership", one cell at two unordered PRs, and the post-landing slot counting to the exposer's cap.

Revision 30 (2026-09-28) — folds the re-check of rev 29 (frozen `a714d5c4`; IMP 3 / MIN 2, all in §0.6 item 7's
meta-rules), replacing the refinements with simpler rules. HEAD for defect D and PR X is `e2d62b9e` plus X's §4
predecessors **other than D's own repair**, D held unrepaired — witnesses re-checked, and E25's ground is no longer
circular. Ownership is one rule: **while the exposer is unlanded it owns the cell** (repairs or records it, whoever
found it); **once it has landed, the defect has shipped** and the finder registers an own-class slot of this lane.
The finder clause, the umbrella-specific sentence and the "next unlanded PR" routing are removed; one builder per
repair governs building only. A `content-visibility` pin is its exposing PR's **own deferral**
(`feedback_defer_cap_policy.md:16`), counted per PR (IBP-sandbox, IBP-css-machinery, IBP-layout 1 each; PR-1b 1,
PR-1c 2, PR-1d 2 — all ≤ 3); the "not counted" claim and its bidi argument are dropped.

Revision 29 (2026-09-28) — folds the re-check of rev 28 (frozen `3ad0f579`; IMP 2 / MIN 5, both IMPs in D3).
IMP-1: the finder clause now covers a cell found after its **repairer or** its exposer has landed; an umbrella PR's
cell found after `IBP-ua-display` has landed is the still-unlanded umbrella PR's own; a cell found where the finder
has already landed goes to the next unlanded PR in §4's order, else to slot registration. IMP-2: HEAD is evaluated
at each exposer's base **as projected** (`e2d62b9e` plus its §4 predecessors, umbrella PRs in their order), and the
witnesses are restated under it (PR-1c's: its base follows PR-1b, so the padding lies left of `x`). MINs: F17's
Home / Writers columns; the acyclicity paragraph and table order for E25/E26; the bidi precedent cited at
`:486-487` with its difference and the cap rested on `feedback_defer_cap_policy.md:16`; F11 names
`elidex_plugin::sandbox::scripting_enabled`. Self-caught: rev 28 had split the F11 table row across six lines,
breaking the §1 table — rejoined.

Revision 28 (2026-09-28) — folds `/elidex-plan-review` round 4 on rev 27 (frozen `ef8c7318`; CRIT 0 / IMP 7 /
MIN 19 / FP 2; decisions D1–D6). D1: F11's document-parse derivation and F10's settings clause share one pure
function of the flag set, applied by the tolerant parser to the creation input before any `EcsDom` exists; the
invariant *parser mode == F10's settings clause* is IBP-sandbox's. D2: a defect whose repair is another program is
never `IBP-ua-display`'s — the exposing PR records it. D3: the cell test's HEAD is the exposing PR's base
(`e2d62b9e` for this memo's own evaluations), and a cell found after its exposer landed goes to the finder. D4:
`#11-content-visibility-skip-contents` is pre-existing class, not A93-ordered; pins are A102-disposed divergences,
not own deferrals. D5: owner lane = this Layout lane (default); the program's umbrella memo is approved before
`IBP-sandbox` (**E26**). D6: the seams slot's `fill.rs` disjunct fires at IBP-layout. MINs folded (PR-1c/PR-1d pins,
PR-1a ruled out, F17 `scripting` writer set, item 7's rule record at the approval PR, flip event, `auto` slot,
E22's drop-test exemption, `#11-range-create-contextual-fragment`, `noscript !important`, omitted branches, item 5's
trigger sites, item 12's differing surfaces, landing vehicle, §4.1 anchor, pseudo slot at IBP-layout).

Revision 27 (2026-09-28) — the user decision (2026-09-28, option (A), after a fourth opinion on every ratified
precedent): a new §0.6 item 12 records an **umbrella-ledger amendment to A102** (A102 standing): its separator is
the size of the repair — a whole program with its own umbrella and slot, whichever lane owns it — so a
program-scale repair is never ordered ahead of this lane; precedent table included. The `content-visibility`
program keeps option (d)'s content, loses its deadline, and merges parts 1/2 into one applies-to consuming F1
(E20); its `i` flag comes from `IBP-css-machinery` again (**E22 restored**); **E23, E24 withdrawn**. Each PR that
fires one of its cells pins it in cell 17c's idiom (item 7's new sentence; item 11 lists `IBP-sandbox`,
`IBP-css-machinery`, `IBP-layout` and PR-1b). E25 and the one-builder rule stand. §2, §3, §4 edges / acyclicity /
obligations and §6 records re-derived.

Revision 26 (2026-09-28) — folds the re-check of rev 25 (frozen `22934653`) and the coordinator's decision to
split the `content-visibility` program's applies-to by the cell rule (the rev-25b hypothesis that replaced content is
never rendered was false: css-contain-2 skips "the replaced content of a replaced element"). **Part 1** decides
applies-to from computed `display` alone; the replaced-inline gap (`<img src=a.png hidden=until-found>`) is not an
A93 cell (HEAD already paints it) → A96. **Part 2** refines by F1 after `IBP-predicate`, scheduled, no deadline (no
cell found). The cell test on the earlier sub-PRs puts part 1 **before `IBP-sandbox`** — `IBP-sandbox` (a
`noscript` parsed as elements) and `IBP-css-machinery` (author `i` / `scripting` rules newly applying) both fire
`content-visibility` cells — so part 1 builds the `i` flag itself: **E22 withdrawn** (contrary to the directive's
"after IBP-css-machinery (E22)", which the requested check refutes), **E21 withdrawn** (property form), **E20**
now part 2's input only; new **E23** part 1 → IBP-sandbox, **E24** part 1 → IBP-layout. Once part 1 lands, nothing
later fires a cell there, so rev 25's exception to "exposer repairs" is withdrawn. New **one builder per repair**
rule (§0.6 item 7): `IBP-ua-display` builds the `[hidden]` rule both it and `IBP-layout` expose (**E25**
IBP-ua-display → IBP-layout); §2's "disjoint" corrected.

Revision 25 (2026-09-28) — folds the re-check of rev 24 (frozen `1e692cd7`; IMP 1 / MIN 2). The IMP: rev 24
derived the `content-visibility` program's deadline over the umbrella PRs only, while this program's own `IBP-layout`
(a `button`'s children laid out for the first time) and likely `IBP-transform` fire its hit-test cells earlier, and
ordering it before `IBP-layout` cycles through E20. Item 11 and E21 now state the deadline as a **property** — before
the first PR of either program that fires one of its cells, derived and cycle-broken by the program's own plan —
with PR-1b, `IBP-layout` and `IBP-transform` as candidate inputs; E20/E22 are subject to that cycle-breaking; and
§0.6 item 7's ownership rule gains its **one exception** (c-v cells are the c-v program's, whoever fires them).
MINs: the home slot's trigger is the later of its two inputs; the E12 fallback introduces the `param` case as a
PR-1b cell.

Revision 24 (2026-09-28) — folds the re-check of rev 23 (frozen `621bd451`; IMP 5 / MIN 5). PR-1b joins every
list of exposing umbrella PRs (it fires line-count / position cells: `<p>a<param style="padding:0 20px">b</p>`).
The cell test puts the `content-visibility` program's deadline at **PR-1b**, not PR-1c (a hit-test cell after A80's
content move; rev 23's PR-1c came from the recommendation's text): **E21 → PR-1b**. Its UA rule
`[hidden=until-found i]` is P1's, with a new edge **E22** `IBP-css-machinery` → the program. §6 gains the umbrella
ledger record for item 11; the home slot's trigger is external (`IBP-predicate` lands). MINs: the five
size-containment exclusions; the slot-class sentence's exception; A96's HEAD reason in cell terms; the canvas ground
per F10r; §4's A93 order sentence and the 6k bullet restated by the cell test.

Revision 23 (2026-09-28) — folds the re-check of rev 22b (frozen `c93538e1`), which found the class reading
contradicts ratified precedents, and the user decision (2026-09-28, re-asked with both sides' precedents). §0.6 item
10 is **withdrawn** (one-sided framing; it contradicted A94, A95 and the min-content promotion); item 7's test is now
the **cell test** — A93 fires on a (markup, channel) cell HEAD gets right and a program PR breaks — with a
verification table over A95, A94, A80, min-content, reqs 8/9 (E13/E14), A96 and A101/A102 and a boundedness note.
Decoration pins corrected: PR-1a's channel is the item stream only (umbrella `:1564-1565`). E12's deadline is
derived (PR-1a, by the item-stream channel). `content-visibility` becomes its **own umbrella prereq program** (§0.6
item 11, option (d)), ordered before PR-1c, with edges E20 (IBP-predicate → it) and E21 (it → PR-1c), homed at
`#11-content-visibility-skip-contents`; `#11-css-containment` and `#11-content-visibility-auto` registered; rev 22's
A96-slot treatment removed. Rev 22b's machinery and late-umbrella clauses stand.

Revision 22 (2026-09-28) — folds the re-check of rev 21 (frozen `f6c56e43`; IMP 3 / MIN 2) and the user
decision (2026-09-28) to read A93/A96 at **defect-class granularity**, aligned with ratified A96's text (§0.6 item
10, new). Item 7's class test is restated accordingly; it reverses rev 5–9's instance-level placements, which
`IBP-ua-display`'s and `IBP-layout`'s plans re-derive (seeds given, not decisions). `content-visibility` becomes an
**A96** class — hidden contents render today — with a block witness (an inline `hidden=until-found` element is not
hidden by the spec), and `#11-content-visibility` is registered as a slot of pre-existing class with an independent
trigger; its A102-divergence wording is gone. The machinery clause is keyed on **order** (rev 22b: the order of derivation — §0.6 item 7). An umbrella exposure found after `IBP-ua-display` has landed is repaired by that umbrella PR itself. Residual
restatements of the rule outside item 7 (rev 21's header, §2, §4 `IBP-layout` and `IBP-css-machinery`) now refer to
it. The edge list is unchanged from rev 21.

Revision 21 (2026-09-28) — folds the re-check of rev 20 (frozen `eb074e03`; IMP 2 / MIN 4, again at the
ownership seam: two owners for late findings, and E16 left late findings with no repairer once `IBP-ua-display` had
landed) and the user decision (2026-09-28) to simplify the structure to **one rule with no seam** (stated in §0.6 item 7 only).
`IBP-css-machinery` (NEW) owns the `i` attribute-selector flag, the `scripting` media feature and any
selector-list form a repair needs; edges **E17** `IBP-sandbox` → it, **E18** it → `IBP-ua-display`, **E19** it →
`IBP-layout`; **E16 is withdrawn**. `content-visibility` is derived, not built: PR-1a newly exposes the contents of
`[hidden=until-found]` and closed `details` (witness in item 7), the repair is the whole property, so it is an A102
accepted, cell-pinned divergence recorded by `IBP-ua-display`, booked in `#11-content-visibility` (NEW). Edge list,
ordering sentences, §2 pairs, §3 owners, §4 table and obligations, §4.1, §5 and §6 re-derived.

Revision 20 (2026-09-28) — folds the re-check of rev 19 (frozen `90166bec`; IMP 3 / MIN 3). IMP-1: rev 19
narrowed §0.6 item 7's class test to the umbrella PRs and orphaned exposures by the other prereq sub-PRs; the test
again applies against every program PR except `IBP-layout`'s presence switch, an exposer that precedes
`IBP-ua-display` keeps its own exposure, and an exposure a sub-PR's plan finds later is placed with its exposer (A93
edge if the repair belongs to another sub-PR). IMP-2: the `i` attribute-selector flag, which both
`IBP-ua-display` (`[hidden=until-found i]`) and `IBP-layout` (`input[type=hidden i]`) need, gets one owner —
`IBP-ua-display` — and a new edge **E16 `IBP-ua-display` → `IBP-layout`** (data flow; acyclic, re-checked);
the cross-pair and the §3 selectors-4 §6.3 row are fixed. IMP-3: `#11-form-control-ua-rendering-fidelity`'s SoT
trigger fires at `IBP-layout`, whose plan re-audits facets (2)/(3). MINs: the facet-(1) row's owner is
`IBP-layout`, its misattributed WPT trigger removed (the SoT's stands); the §3 §15.3.10 row is placed by the
exposer and notes the existing `display: inline-block` rule.

Revision 19 (2026-09-28) — folds the re-check of rev 18 (frozen `13ef10bc`; IMP 3 / MIN 3), all at the seam
between `IBP-ua-display`, `IBP-layout`, E15 and item 7's extension (the `input` states placed both in
`IBP-ua-display` + E15 and in `IBP-layout` with no edge; two derivers for one placement; E15's premise cited only in
Appendix D). Root: the consequences of `IBP-layout`'s presence switch were split across two sub-PRs. Decision (one
owner per exposure; exposer = repairer needs no edge): `IBP-layout` owns **every** consequence of its presence
switch — derives, places (A93 / A102 with both branches / A96 "left exactly as found") and registers the A96
remainder as a new facet of `#11-form-control-ua-rendering-fidelity` — including the Hidden-state `input`'s
`display: none !important` rule and §15.3.10's resets; the body regains one premise cite (`helpers.rs:405-418`,
`block/mod.rs:224`, `:380` before `:401`). `IBP-ua-display` keeps only item 7's hiding/re-boxing class exposed by
the umbrella PRs; item 7's first-time-subtree extension is reverted into `IBP-layout`'s obligation; **E15 is
withdrawn** and every site citing it re-derived (item 7 Edges, §4 edge table, the cross-pair, IBP-classify's
Hidden-state line, IBP-ua-display's obligation, §3 owners of §15.3.1's hidden-input rule and §15.3.10, the §6
facet (1) row). MINs: the §6 facet row quotes the slot's SoT trigger/re-eval and gives the facet its own trigger;
the `IBP-layout` obligation states A96 and A102 in item 7's wording.

Revision 18 (2026-09-28) — folds the re-check of rev 17 (frozen `f04184e9`; IMP 2 / MIN 3), again in newly
written button/form-control facts: rev 17 said a `<button>`'s children are not painted today — false (the paint
walk visits them and the legacy inline path paints their text against the button's box), so only their layout is
first-time and some §15.3.10 gaps are author-visible today; and its "the rest stay A96 slots" had no slot or
registration owner. Every IMP since rev 13 came from form-control rendering facts written into this body, which are
`IBP-layout`'s and `IBP-ua-display`'s mechanism, so rev 18 hands the area over: the body keeps I1 as the property,
the spec-fixed content sources as quoted inputs (no claims about today's code), and an `IBP-layout` obligation to
derive, place (A93/A96/A102) and register every rendering change its presence switch causes, the A96 remainder as a
new facet of `#11-form-control-ua-rendering-fidelity` (§6 row shipped with `IBP-layout`). §0.6 item 7's extension
is generalised to a property with seeds, placement left to the owner plans; E15's rationale sites (item 7 Edges, §4
E15 row, the cross-pair) now name first-time layout generally. Rev 17's today-code analysis, corrected, is Appendix D
input. MINs: §3 §15.3.10 row per the column definition; the slot's *Why deferred* drops the sentence contrasting box model with cascaded
style; §3 §7.2.1 row names button layout's undefined primitive appearance and `select`'s exception.
Self-check also fixed two stale cites (`sizing.rs:41` → `:91`; `init.rs:37`/`:63` → `:51`/`:95`).

Revision 17 (2026-09-28) — folds the re-check of rev 16 (frozen `87f08d91`; IMP 2 / MIN 3), both IMPs in rev 16's
"today's paint diverges" text. Its fact (a) was wrong — `from_input_element` substitutes "Submit"/"Reset" for an
empty raw value, so the divergence is `value=""` showing "Submit", and it lives in form-core, not in the paint arm —
and its scope reason ("whose rendering I1 requires to be the spec's") over-reached: I1 requires one defined
rendering, and that reason would have pulled every value-text-painted control (Range, Date, …, File) out of A96.
The per-divergence analysis, corrected, moves to Appendix D. I1 keeps the content-source obligation (§15.5.12 now
read branch by branch, `value=""` included), gains a scope rule (pre-existing form-control divergences stay A96
except inside code `IBP-layout` rewrites for the content source; its plan names which, with a fixture per branch),
and records that `<button>` subtrees are laid out for the first time at `IBP-layout`, with §0.6 item 7's class test
applied — self-review found it **does** expose a difference: HTML §15.3.10's inherited `initial` resets are
missing from elidex's sheets, so §0.6 item 7's property is **extended** to computed-value differences on a subtree a
program PR renders first (placed in `IBP-ua-display`, already ordered by E15; new §3 row, M=43). MINs: §3's css-ui-4 §7.2 row names Color; the §7.2.1 row names its omitted branch
(primitive appearance) under the column definition.

Revision 16 (2026-09-28) — folds the re-check of rev 15 (frozen `2f40d979`; IMP 1 / MIN 3). The IMP: with the
button-layout rows probe-decided, the body said what must not be painted as `<button>`'s content but not what must,
and nothing kept an `input` button or Color state from laying out script-appended DOM children if the probe answers
non-replaced. I1 and `IBP-layout`'s obligations now fix every button-layout element's content source independent of
F4's answer (HTML §4.10.6, §15.5.12, §15.5.9; never an `input`'s DOM children), name today's two paint divergences
in the re-keyed arm as `IBP-layout`'s to repair, and §4 derives that the `input` exposure needs no E15-like
edge (exposure and repair land in `IBP-layout`), with the condition under which one appears. MINs: I1 quotes §4.10.6
directly; D1 names Color (§15.5.9); §3 defines the *Full enum?* column and marks the three button-layout rows
✓ (probe-bound), adding a §4.10.6 row (M=42 at rev 16).

Revision 15 (2026-09-28) — folds the re-check of rev 14 (frozen `19890589`; IMP 2 / MIN 2). **Correction of a
fabricated quotation**: rev 14 attributed to HTML §15.5.3 a quoted sentence about a child anonymous button content box; no such
sentence is in `webref body html button-layout` and no such dfn exists. It is removed, and with it
rev 14's §15.5.3-decided `button` row: §15.5.3 suggests non-replaced (a new formatting context; replaced-element treatment only for two
purposes) but does not state it, while css-ui-4 §7.2 makes widgets replaced — a
spec conflict, so `button` and `input` in the Submit/Reset/Button (§15.5.12) and Color (§15.5.9) states return to
**probe-decided** rows, and the "impossible by construction" claim is withdrawn. The widget-rendering key is
mechanism and moves to Appendix D; I1 keeps the decision (every site switches, together, onto the predicate matching
its purpose, each element getting exactly one defined rendering) and one obligation (`<button>` gets no
`value`-attribute label painting). Every quoted spec sentence in the body was re-checked against `webref body`
output (`plan-review-r3/rev15-selfcheck.md`).

Revision 14 (2026-09-28) — folds the re-check of rev 13 (frozen `5a931f90`; IMP 1 / MIN 3). The IMP: rev 13's
I1 switched every presence-keyed site onto F4, but those sites answer two different predicates — replacedness (F4)
and widget rendering (F7, native appearance). They now key on the predicate each stands for, both supplied by
`IBP-classify` (E4 orders both), and the one combination where the two can differ — a native-appearance widget F4
answers non-replaced — exists only for the `button` element, whose rendering HTML §15.5.3 defines, and is impossible
for every other widget by construction (§2 I1, F4/F7, §4) [withdrawn at rev 15: rested on a fabricated quotation]. The self-check worksheet this revision was built with
turned `button`'s F4 row from probe-decided into §15.5.3-decided (the probe corroborates). MINs: paint-source reads are outside I1's property unless they gate
children or chrome; the 6k F5-removal case rests on its per-entity reason; Appendix D's lead-in is reordered.

Revision 13 (2026-09-28) — folds the re-check of rev 12 (frozen `a39ecc8e`; IMP 1 / MIN 3). The IMP: I1's
presence-keyed population was scoped to layout, but paint also decides widget rendering by component presence
(`elidex-render/src/builder/walk.rs:409` dispatches `emit_form_control` for any entity with `FormControlState`). I1
is restated by the property — **every production site, in any crate, that uses component presence to stand for
replacedness or widget rendering** — owned by `IBP-layout`, measured in its own plan-memo, with layout and paint
switching together (§2, §1 F4, §4). MINs: `input[type=hidden i]` leaves `#11-form-control-ua-rendering-fidelity`
(discharged by `IBP-ua-display`, E15); item 7's reach sentence aligned, with the `IBP-sandbox` `noscript` check; the
6k bullet covers `IBP-layout`'s F5 removal.

Revision 12 (2026-09-28) — folds the final re-check of rev 11 (frozen `902b89a2`; IMP 1 / MIN 6). The IMP: §0.6
item 7's reach counted only the umbrella's PRs, but this prereq program's own sub-PRs are program PRs too —
`IBP-layout` moves the presence-keyed sites onto F4 and can newly expose a difference (a hidden `input`'s appended
children, once `FormControlState`-presence replacedness goes away). The reach is widened to both programs, and the
Hidden case gets a new edge **E15 `IBP-ua-display` → `IBP-layout`** (§4). MINs: the class test replaces "each rule
below"; §2 and §4 no longer presuppose derived members or a closed axis list; the rev-10 dispositions handed to
`IBP-ua-display`'s plan are named in the body as reopened inputs; `IBP-ua-display` records any A102 accepted
divergence (and §6 ships it); §4.1's F7 row is touch-set evidence; the changelog names the appendix additions.

Revision 11 (2026-09-28) — changes the instrument rather than the list (the rev-10 re-check found one more
hiding mechanism, `content-visibility`, as each round had found one more): the enumerations that belong to a
sub-PR's own plan-memo leave the reviewed body. `IBP-ua-display`'s population is now **derived by that sub-PR's
plan** from §0.6 item 7's property, with seeds named; rev 10's per-rule partition, witnesses, measuring command and
slot analysis move to the new **Appendix G**. F7's site list and command move to Appendix B, F11's caller list to
Appendix A; the body keeps the decisions. `input[type=hidden i]` becomes an `IBP-classify` obligation. The slot
`#11-ua-sheet-html-rendering-display-fidelity` is registered by `IBP-ua-display` only if its population leaves a
remainder.

Revision 10 (2026-09-28) — folds the Step 4.5 re-check of rev 9 (frozen `fc29a50e`; IMP 2 / MIN 10):
- **IMP-1** — F7's measuring command is widened from two read forms to every attribute-read API (the `Attributes` methods `get` /
  `contains` / `iter` and the `EcsDom` methods `get_attribute` / `with_attribute` / `has_attribute`), which finds two more mappings
  in `elidex-dom-api`'s focus predicate; each of F7's, F11's and item 7's commands now states what it cannot see.
- **IMP-2** — item 7's sort key for `display: none` rules is corrected to the property "some selected element
  computes a non-`none` display and its laid-out subtree can contain an inline box", which moves the table `form`
  rule into `IBP-ua-display`; the slot keeps `details > summary:first-of-type` alone.
- MINs: 6k's any-PR argument; `#11-media-extended-features`' trigger; the slot's population; the measuring command
  prints `@media` wrappers; F11's Inert/Fragment modes; the template-contents limit; F10's worker invariant; §2/§4
  global-kind wording.

Revision 9 (2026-09-28) — folds the Step 4.5 re-check of rev 8 (frozen `9ee95a78`; IMP 4 / MIN 10):
- **IMP-A** — A93's reach is restated as a property: **any PR of the program** after which a defect becomes newly
  reachable or newly visible. PR-1d (M6's line height and M7's baseline reach every inline box on a committed line,
  decorated or not) makes rev 8's (b) members that compute `inline` newly wrong, so they move into
  `IBP-ua-display` (A93; bounded UA rules, A102); the slot keeps only the non-inline members (§0.6 item 7).
- **IMP-B** — F7's single mapping is defined by the property "every production site that maps an `input@type`
  string to a kind or predicate", measured (five mappings at six sites), not by adding names.
- **IMP-C** — F11 is defined by the property "every production caller of a fragment parse", measured
  (`insertAdjacentHTML` added); the `scripting_disabled` option field is deleted, one source.
- **IMP-D** — the two enriched slots carry a reason-Why, an independent trigger and a re-eval date.
- MINs: F10's global-kind discriminator made single; rev 8's slot wording; `build_paged_display_lists_interleaved`;
  `#11-appearance-replacedness`'s trigger.

Revision 8 (2026-09-28) — narrows rev 7's C1 by the ratified ledger's own splitting criterion (the
orchestrator's correction: rev 7's one-class population over-reached into selector-engine work). Umbrella
**A96** (`docs/plans/2026-08-line-box-decorated-inline-content.md:795`) keeps a defect that is author-visible
today regardless of this program a slot; **A102** (`:801`) separates a bounded repair inside this program's
reach (a prerequisite) from another lane's deferred program (an accepted divergence). Applied per rule in §0.6
item 7: the rules whose spec value is `display: none` **and** whose elements compute `display: inline` in elidex
stay in `IBP-ua-display` (A93); the rest go to one newly registered slot of pre-existing class,
`#11-ua-sheet-html-rendering-display-fidelity` (A96) — narrowed again by rev 9. The machinery left in the sub-PR is the `i` attribute
flag and the `scripting` media feature (bounded, A102); `:popover-open` reduces to authoring; `:is()` and
`:first-of-type` leave the program. F15, §3, §4, §6 are made consistent.

Revision 7 (2026-09-28) — folds the Step 4.5 focused re-checks of rev 6 (frozen `9bcb7cfe`; C1 2 IMP,
C2 3 IMP, C3 0 IMP, C4 4 IMP), at decision altitude:
- **C1** — the UA-display sub-PR is renamed **`IBP-ua-display`** and its population is defined by the
  property, not a vocabulary: every HTML §15 `display` rule whose selector computes a different `display` in
  elidex's production cascade, with the measuring command; each rule needing unsupported machinery gets a
  disposition; the A93 criterion is restated with both halves and applied (§0.6 items 6–7, §4).
- **C2** — F10's global kind, F7's second mapping (`KNOWN_INPUT_TYPES`), F11's fragment home, the
  `IframeSandboxFlags` allow-token inversion, F4's read of F10 via F10r, and the scope of "no gate can fail
  open" (§1).
- **C3** — §0.6 item 3's `inline-flex`/`inline-grid` exemption, F10's Window wording, item 9's S5-4 cite,
  "S5-4 E5" naming, §0.3's verbatim quotes.
- **C4** — §0.6 records the relaxation of the umbrella's PR-1a deadline for reqs 8/9 and of A105 for the pseudo
  prereq; I1 widened to writers of presence-site inputs, closing the step-11/13 window by moving the `src`
  mutation algorithm into `IBP-layout`; the approval vehicle; `build_paged_pipeline`'s disposition; slot Why /
  Trigger; every memory surface naming the singular "predicate prereq" re-pointed (§6).

Rev 7's C1 population is superseded by rev 8 (above). Rev 6 folded round 3's clusters C1–C4; rev 5 re-sliced the memo into this program memo (R2-S); rev 4 folded
round 1; rev 3 added decision d5; rev 2 recorded the req 5 amendment. The appendices (input to the sub-PR
plan-memos, not reviewed here) are rev 6's, with the sub-PR rename applied and one routing note at the head of
Appendix B; rev 11 added Appendix G (`IBP-ua-display`) and appended F11's caller list to Appendix A and F7's site
list to Appendix B.

Prereq program of the `#11-line-box-decorated-inline-content` umbrella
(`docs/plans/2026-08-line-box-decorated-inline-content.md`, approved by #515 at rev 99, `e2d62b9e`), whose §8
*Predicate prereq PR* block states nine requirements. Under ledger **A145** those requirements and every
ordering are ratified; the mechanism is derived here and in the sub-PR memos; every departure from a ratified
surface is listed in §0.6.

## §0. Frames, premises, decisions

### §0.1 Coordinate frame

Every `file:line` is an **`e2d62b9e`** coordinate. `git diff --stat 22de3078 e2d62b9e -- crates` is empty, so
every umbrella `22de3078` coordinate holds. Against `154bac3f`, `git diff --stat=200 154bac3f e2d62b9e -- crates`
changes files only in `elidex-layout-block/src/inline/` (6), `elidex-js` (38), `elidex-script-session` (1),
`elidex-navigation` (5) and `elidex-shell` (8); a `154bac3f` coordinate in any other file holds.

### §0.2 Premises that do not hold at `e2d62b9e` (decision-relevant)

HTML §15's *expected* is read as **must**. The §15 prelude says so verbatim (`webref body html rendering`):

> For the purposes of conformance for user agents designated as supporting the suggested default rendering, the term "expected" in this section has the same conformance implications as "must".

| # | Premise (umbrella or brief) | What holds | Decision it drives |
|---|---|---|---|
| D1 | Req 3: a widget such as `input` is a "replaced form control"; complete over `get_intrinsic_size`'s domain | css-ui-4 §7.2: "The term widget … denotes replaced elements that can have native appearance" (`webref body css-ui-4 appearance-switching`), so req 3's ground holds for widgets with native appearance. Residues: elidex has no `appearance` property (`git grep -n -i '"appearance"' -- crates` → none); css-ui-4 §7.2.1 devolves a devolvable widget on an author-origin border/background value (44 properties, `webref body css-ui-4 appearance-disabling-properties`); HTML leaves the primitive appearance undefined ("Need to define the primitive appearance", `webref body html button-layout`); HTML §15.4's list of elements that "can be replaced" names `input` but not `select`/`textarea`/`meter`/`progress`/`button`; `<output>` is in `get_intrinsic_size`'s domain (`FormControlKind::Output`, `elidex-form/src/sizing.rs:91`) and is no widget ; and HTML §15.5.3's button layout — "The element is expected to establish a new formatting context for its contents", "act as if the element is a replaced element" only for absolute positioning and `align-self` — reads as non-replaced for `button`, the `input` button states (§15.5.12) and the Color state (§15.5.9), against css-ui-4 §7.2 (a conflict the probe decides) | §0.6 item 3 |
| D2 | Req 5: "a **failed or 404 image**" is replaced | HTML §15.4.2 rule (3): an `img` that "represents some text and the user agent does not expect this to change" is "a non-replaced phrasing element" (`webref body html images-3`); §4.8.3 defines when it represents text (`webref body html the-img-element`) | §0.3 |
| D3 | Req 5: an undrawn `<canvas>` is replaced | only when it "represents embedded content", i.e. "if scripting is enabled for the canvas element" (`webref body html the-canvas-element`; §15.4.1) | §0.6 item 1; F4 via F10r |
| D4 | Req 5 / brief: `ImageData` must be split, reaching both producers | `ImageData` already means "has pixels" (inserted only after decode, `elidex-navigation/src/loader.rs:286`; `elidex-api-canvas/src/component.rs:139`); elidex models no image-request outcome — `loader.rs:283-294` only logs a failure | F5, F6 |
| D5 | Req 6: the replacedness half's home is `elidex-form-core`, or `elidex-dom-api` gains a form edge | the classification reads no form component; `FormControlState` is attached by `elidex-form` (`init.rs:51`, `:95`); `elidex-dom-api → elidex-form` is a cycle (`elidex-form` depends on `elidex-dom-api`) | §1 home grounds |
| D6 | §9: the display half needs no crate work | it exists as private `is_atomic_inline` (`inline/collect.rs:14`), unreachable from `elidex-ecs` | F2 |
| D7 | Req 5 routing: `fill.rs:418` is the same monolithic test | a separate definition (`find_tallest_monolithic`, `fill.rs:407-418`, omits `has_transform`, reads `ImageData`) | `IBP-layout` obligation |
| D8 | Req 5: presence classifies `<object>` non-replaced "always" (implied: replaced) | §15.4.1: only an `object` "that represents an image, plugin, or its content navigable" is replaced; elidex loads no `object` content | §0.6 item 1; §5 |
| D9 | the 2026-06-11 ruling's ground 2: a component on an un-swapped document entity goes stale under navigation | false at HEAD — §0.5 | §0.5 |
| D10 | S5-4 E5 (the S5-4 plan's edge E5): "scripting-disabled ≠ parser `scripting_disabled` … name collision, zero shared semantics" (`docs/plans/2026-07-s5-4-sandbox-enforcement.md:943`) | HTML couples them: the parser's scripting mode "is initially set to Normal if scripting was enabled for the Document with which the parser is associated … and Disabled otherwise" (§13.2.4.5, `webref body html other-parsing-state-flags`); fragment parsing: "If contextDocument's scripting is disabled, then set scriptingMode to Disabled" (§13.4 step 10) | F11; §0.6 item 9 |

The umbrella's §9 survey (bool fns stating a `Display` categorisation, awk command in the umbrella) returns the
same seven lines at `e2d62b9e` (`columns.rs:28`, `display.rs:35`, `a11y/src/tree.rs:280`, `block/mod.rs:46`,
`block/mod.rs:694`, `inline/collect.rs:14`, `anonymous_table.rs:12`); none answers *inline box*.

### §0.3 Amendment of ratified requirement 5 (recorded, not edited — A114 idiom)

*Req 5 says* (umbrella §8, verbatim): "the predicate answers *replaced* for an `<img>` entity carrying no
`ImageData` (a failed decode, a JS-created `<img>`) and for an undrawn `<canvas>`", with the cases named as "a
**failed or 404 image**" and "a **JS-created `<img>`**", and with no or-justify branch (req 5's words, `docs/plans/2026-08-line-box-decorated-inline-content.md:5383-5384`).

*Corrected*: replacedness is keyed on what the `<img>` **represents** (§4.8.3) and §15.4.2's rules in order —
never on `ImageData` presence as such. An HTML `<img>` is **non-replaced** exactly when rule (3) is the first
applicable rule:
- **(ii)** its request is **broken** and `alt` is non-empty — the **user decision** (2026-09-27, option (b) on
  "a failed or 404 image");
- **(i)** its selected source is null (`src` absent or empty; elidex selects no `srcset`/`picture` source) and
  `alt` is non-empty — added by the **read-only audit that followed** that decision (rule (3) reached
  deterministically, with no request); it contradicts req 5's own illustration, since a JS-created
  `<img alt="x">` with no `src` is non-replaced.

Every other `<img>` is replaced (rule (1) available; rule (2) pending, or no `alt` attribute; rule (4) `alt=""`
with nothing to show).

*Why*: rule (3) is a must under the §15 normativity above; the missing alt-text rendering is a separate core gap
(`docs/design/ja/18-image-decode.md` §18.10 puts broken-image rendering in core; `01-executive-summary.md` §1.2's
エラー回復 (error recovery) is parse-error recovery), not a reason to bend classification.

*Umbrella surfaces this contradicts* (recorded, not edited): cell **6c**'s rationale holds for its fixture only
because that `<img>` has no `alt` (rule (2)); cell **6f**'s ground "answers **replaced** from element identity,
not from decode outcome" is false for an `alt`-bearing `<img>`; §9's canonical-predicate bullet ("A predicate
whose answer depends on network timing is not one css-display-3 §A describes") is false for rule (3). New cell
**6k** for PR-1a: `<p>a<img alt="x" style="padding:10px">b</p>` (no `src`) → the `<img>` is an inline box and M1
emits an adjacent marker pair. Its missing alt text is placed by the A93 criterion in §0.6 item 7.

### §0.4 Compat excluded

§15.4.2 rule (2)'s quirks disjunct ("the Document is in quirks mode, and the element already has natural
dimensions") is compat and excluded from the core predicate. elidex never enters quirks mode (strict parser,
`elidex-html-parser-strict/src/tree_builder/insert.rs:155-157`; `compatMode` fixed `"CSS1Compat"`,
`elidex-js/src/vm/host/document.rs:533`), and ADR #4's pre-core normalisation (`docs/design/ja/28-adr.md:11`)
cannot reach a disjunct keyed on runtime state. No slot.

### §0.5 Decision d5 as decided in form (iv): the flag set is stamped at Document creation

**User decision (2026-09-28)**: the user explicitly re-approved superseding their 2026-06-11 ruling ("partial
move = strangler"; recorded in the memory memo for `#11-browsing-context-state-ecs-components`) in form **(iv)**.
`ActiveSandboxingFlagSet` (NEW) is stamped **at Document creation**: the entry that creates a Window's document
root receives the flag set as a creation input and stamps it immediately after creating the root. The shape is
`DocumentBaseUrl`'s — a document-node component written as the node is spawned
(`elidex-ecs/src/dom/mod.rs:466-471`) — but restricted to **Window document roots** (F9's non-applicability,
§1), whereas `DocumentBaseUrl` is on every document node. The parser's scripting mode derives from the same
input. The rev-4 write point at `run_scripts_and_finalize` is withdrawn.

**Why this is a premise change and not a lens override** — the ruling's three grounds, each re-measured:

1. *Ground 3 ("No S1b correctness loss meanwhile") is reversed.* A sandboxed iframe's `<noscript>` parses as
   scripting-enabled today: the document parse hard-codes it (`elidex-html-parser/src/lib.rs:48-49`, html5ever
   `ParseOpts::default()`), while HTML §13.2.4.5 sets the parser's scripting mode "to Normal if scripting was
   enabled for the Document … and Disabled otherwise". And the canvas arm (D3) would, without the flag set, ship
   a wrong answer on PR-1a's item stream: with no F9, F10r reads empty flags, so scripting is enabled and a sandboxed
   document's `<canvas>` answers replaced — the marker pair its non-replaced inline box should get is missing
   (§0.6 item 7's cell test, the program's new markers counting as intended) — which ratified **A93** orders before
   PR-1a, not after.
2. *Ground 2's premise (a component on an un-swapped entity goes stale under navigation) is false at HEAD.*
   `location` setters are enqueue-only ("They do **not** mutate `current_url`",
   `elidex-js/src/vm/host/location.rs:9-15`); a cross-document navigation builds a fresh pipeline — a fresh
   `EcsDom` and document root — (`build_pipeline_from_loaded`, `elidex-shell/src/content/navigation.rs:276-291`;
   reload/traversal re-stamp the rebuilt document, `:324-343`); an iframe navigation unloads the old entry and
   loads anew (`content/iframe/lifecycle.rs:342-366`).
3. *Ground 1 ("partial move = strangler") — why this member and not the cluster.* The discriminator is: **read by
   engine code without a script runtime AND written only at Document creation**. Layout and style read the flag
   set through the replacedness query, the parser through its scripting mode. Its writers in the spec: HTML §7.1.5
   — "When the Document is created, its active sandboxing flag set must be empty. It is populated by the
   navigation algorithm" (`webref body html sandboxing`); *create and initialize a Document object* (§7.5.1) sets
   it to "navigationParams's final sandboxing flag set" (`webref body html initialise-the-document-object`);
   *create a new browsing context and document* (§7.3.2.1) sets the initial `about:blank` document's (`webref body
   html creating-a-new-browsing-context`). Both writes happen as the Document is created, and no other writer was
   found — so "immutable after creation" is **elidex's reading of the spec's writer set**. Precedent:
   `DocumentBaseUrl` is already a document-node component; `current_url` stays per-VM, mutated by same-document
   navigation. The cluster holds **two classes**, not a member excepted from one. (A reviewer's alternative —
   "VM-only consumers" — is false: the shell reads origin, `iframe_depth` and `current_url` through the runtime
   trait, `content/iframe/lifecycle.rs:215`/`:297`/`:390`, `content/event_loop.rs:324`.)

**Relation to B1** (`docs/plans/2026-06-agent-scoped-ecsdom-world.md`): §5 req 2 ("Creation-parameters-first
ordering") computes "the sandboxing flag set" among the creation parameters **before** creating the document
(`:418-427`). Stamping at creation is that requirement's own write point, so B1 later **swaps only the producer**
of the input — no second move of the write. B1 §5 req 5 (the cluster fold, `:450-455`) loses this one field. The
embedder→embeddee flag union (§7.1.5, "the flags set on embedder's node document's active sandboxing flag set")
stays B1's.

### §0.6 Ratified-surface changes (the one list)

Each item is plan-review input for the surface it changes; its record ships with the event named in §6.

1. **Umbrella req 5** — amended for the represents-text arms (§0.3), with cells 6c, 6f, §9's bullet and new cell
   6k recorded; the canvas arm becomes **scripting-dependent** (D3); `<object>` is ordinary while elidex loads no
   object content, reversing req 5's implication (D8).
2. **Umbrella req 2** — re-read as "one canonical answer to *inline box*": `is_atomic_inline` (`collect.rs:14`)
   is subsumed into the display facet; `is_block_level` (`block/mod.rs:46`) answers a different question
   (block-level) and is left standing. `IBP-layout` lands before PR-1a (§4), so the presence-keyed replacedness
   answers also converge before PR-1a, as req 2's ordering intends.
3. **Umbrella req 3** — narrowed: completeness over `get_intrinsic_size`'s domain excludes `<output>` (D1); a
   devolvable widget whose native appearance is disabled (css-ui-4 §7.2.1) is replaced or not **as the probe
   measures**; the elements using button layout are not inline boxes, on two separate grounds: a `button` whose
   computed `display` has outer type `inline` — other than `inline-flex` and `inline-grid`, which §15.5.3 keeps as
   computed ("If the computed value of 'display' is 'inline-grid', 'grid', 'inline-flex', 'flex', 'none', or
   'contents', then behave as the computed value") — is expected to "behave as 'inline-block'" (§15.5.3's first
   bullet, `button` only); and every element using button layout — `button`, `input` in the Submit/Reset/Button
   states (§15.5.12) and the Color state (§15.5.9) — "is expected to establish a new formatting context for its
   contents" (§15.5.3's second bullet), so its contents do not share its parent's inline formatting context, which
   css-display-3 §A's *inline box* requires. (`inline-flex`/`inline-grid` are not inner flow, so not inline boxes
   either.)
4. **B1 §5 req 2 and req 5** — user-approved (decision (iv), §0.5): the Window document's flag set is stamped at
   creation ahead of B1, at req 2's own write point; req 5's cluster fold loses `sandbox_flags`.
5. **Umbrella ordering** — the umbrella orders "the predicate prereq" as one PR before PR-1a
   (`docs/plans/2026-08-line-box-decorated-inline-content.md:5238`: PR-1a "branches off `main` only after the
   approval PR, the predicate prereq and the replaced-origin pseudo prereq"; reqs 8/9 at `:5503`/`:5510`; req 9's
   consequence text at `:4573`, "The predicate prereq lands before PR-1a") and the pseudo prereq after it
   (**A105**). This program splits it and applies **one rule to every edge** (§4). Two consequences **relax**
   ratified orderings and are recorded as such, not as the umbrella's own texts: (a) `IBP-transform` and
   `IBP-observer` (reqs 8/9) are ordered before **PR-1c**, not PR-1a — the relaxed deadline is PR-1c, the
   program PR after which that defect becomes reachable (item 7); (b) the pseudo prereq is ordered after
   `IBP-classify` and `IBP-layout` and is **unordered against `IBP-predicate`**, relaxing A105's "after the
   predicate prereq" to its own ground (data flow: the pseudo prereq reads F4 only).
6. **Umbrella prereq set** — a new A93-ordered prereq, `IBP-ua-display` (§4), whose population its own plan-memo
   derives by item 7's rule; the CSS machinery its repairs consume comes from `IBP-css-machinery`, inside this
   program (§4), except a need found after that plan fixes its population, which item 7's machinery clause gives to
   the repairer (F15).
7. **The A93 criterion, restated as a property and applied once.** Umbrella A93 (`:1848-1850`): "a pre-existing
   defect that **this program makes newly reachable, or newly visible to an author** is **ordered before the PR
   that makes it so**; a defect this program leaves exactly as it found it stays a **slot**." Its reach is **any
   PR of the umbrella program or of this prereq program** after which a defect becomes newly reachable or newly
   visible — not a list of PRs. Among the umbrella's, the PRs that change what an inline box does are PR-1a (M1: an element enters the item stream as an inline box —
   the level of **A95**), PR-1b (the inline-axis advance — line count, positions, the `LayoutBox` content position, A94/A80), PR-1c (the box gets a `LayoutBox` — reqs 8/9) and PR-1d (M6's height and M7's baseline:
   "M6 — **PR-1d**, for **every** inline box, decorated or not", umbrella `:1258`); among this program's, likewise any
   sub-PR after which a defect becomes newly reachable or newly visible — notably `IBP-layout`, which moves the
   presence-keyed sites (layout and paint) onto F4. `IBP-sandbox` was checked: it changes how `<noscript>` parses
   in a scripting-disabled document (F11), which needs no UA rule (the spec's `noscript { display: none !important; }`
   applies only under `@media (scripting)`) and moves the element toward the spec. **The ownership rule — one rule, stated here and referenced everywhere else.** **While the exposer is unlanded, the
   exposer owns the cell** — it repairs it, or records it when the repair is another program's — whoever found it.
   **Once the exposer has landed, the defect has shipped**: the finder registers it as an own-class slot of this
   lane (Why / trigger / re-eval) — or, where the repair is another program's, records the pin against the exposer's
   own-deferral count, with no second slot — and it is never relabelled A96. No other late-finding rule applies. **Who builds a
   repair** is a separate question and does not change who owns the cell: when the exposer is an umbrella PR
   (PR-1a, PR-1b, PR-1c, PR-1d), the repair is built by `IBP-ua-display`, ordered before that umbrella PR (A93; E12 to
   PR-1a, the earliest) — except a repair that is another program (below); and **one builder per repair**: when
   several PRs expose the same defect D whose repair is one mechanism or rule, the earliest of them in §4's order
   builds it and the others consume it; if they are unordered, an edge is added that makes one earliest —
   preferring the one whose other inputs already precede — and that one builds it. For this rule a PR is **builder-eligible** for D if it
   exposes D or is the builder of an exposer's repair, and **an umbrella PR's exposure counts as
   `IBP-ua-display`'s**, since `IBP-ua-display` builds umbrella repairs (the umbrella-exposure rule above decides this,
   not the symmetric tie-break). **One cell firing at two
   unordered PRs** is owned by the earlier to land; the later re-checks at its actual base and finds it repaired or
   pinned — no double pin. A post-landing own-class slot counts toward **the exposer's** cap: it shipped the defect. The exposer still owns its cell
   and consumes the earlier builder's repair; where no earlier builder carries it (e.g. a cell found after
   `IBP-ua-display` has landed), the unlanded exposer repairs it itself. **When the repairer is another program**
   (item 12, the A102 amendment), the exposer records the pinned divergence (cell 17c's idiom: elidex's value
   asserted, the conformant value named, the flip event named — the other program's PR that wires the fix into the
   relevant reader, as that program's umbrella names it). Such a pin is an A102-disposed accepted divergence and **the exposing PR's own deferral**
   (USER DECIDED 2026-09-28, item 13): `feedback_defer_cap_policy.md:12` counts own deferrals only, and `:16` defines
   own as a defect the PR brought in — under the cell test the PR makes a cell that is right at its base wrong. That
   is the difference from the umbrella's bidi precedent (`:486-490`, "no deferral of this one's"), where no cell
   fires. It counts whenever the cell fires at the exposer's actual base, found before or after the exposer lands
   (one rule), and stays within the per-PR cap of 3 (item 13). **A pin is conditional on the cell firing at the exposer's actual base**:
   if the other program's repair is already there, the cell does not fire — no pin, no record; the exposer's own
   plan checks this at its actual base. (The frame's "D held unrepaired" governs ordering derivations, not pins.) **Machinery, keyed on the order of derivation**:
   `IBP-css-machinery`'s population is fixed when its own plan-memo derives it, from every consumer known at that
   point, each with the data-flow edge from `IBP-css-machinery` to it; any machinery need found later — by any
   sub-PR's or umbrella PR's plan after that derivation — is built by that repairer itself (the first builder owns
   it). Nothing late routes to `IBP-css-machinery`. Repairers at or before `IBP-css-machinery` in §4's order — the
   two splits (pure moves, no exposure), and `IBP-sandbox` (E17) — build their own; `IBP-classify` is unordered with
   `IBP-css-machinery` and stays so: it is no known consumer, so it builds any machinery its own repairs need (none
   is known; an edge `IBP-css-machinery` → `IBP-classify` would lengthen the critical path for no consumer). No
   sub-PR places or derives another sub-PR's exposure: each sub-PR's plan-memo applies the cell test below to its
   own changes (population: its own new outputs × their readers), and a finding made later — in a plan or in
   review — is placed by the same rule. **A96** (`:795`)
   keeps a defect that is author-visible today regardless of the program a slot; **A102** (`:801`) orders a
   bounded repair inside the program's reach as a prerequisite, and records one whose repair is another lane's
   deferred program as an accepted, cell-pinned divergence. Applied:
   - **The cell test (USER DECIDED 2026-09-28; supersedes item 10).**
     > A93(D, X) ⇔ there is a cell (M, O): M = markup + style under elidex's **current** cascade (UA and compat
     > sheets included); O = one channel a §6 cell can assert (item stream / line count / `LayoutBox` / display
     > list / hit test / CSSOM or observer read); the spec defines O(M), HEAD returns the spec value, after program
     > PR X lands it does not, and repairing D alone restores it (the program's own new behaviour counts as
     > intended). Deadline = the first such X. A96(D) ⇔ no such cell for any X. A102 changes only the disposition
     > of a fired A93 (order vs accepted divergence), never the trigger. "Class" is the repair unit, not the
     > trigger. A66 is a user carve outside this test.

     **HEAD** in this test, for defect D and program PR X, is **`e2d62b9e` (§0.1) plus X's §4 predecessors other than
     D's own repair, with D held unrepaired** (umbrella PRs among the predecessors in their order, PR-1a < PR-1b <
     PR-1c < PR-1d). Ground: reachability is relative to what X changes on the base it is cut from, and D's repair
     landing earlier is the ordering the test decides, so it cannot be part of the frame. Every witness in items 7
     and 11 is stated under this frame; each exposer's plan re-checks at its actual base.

     A fired A93 is handled per the ownership rule above: a bounded repair — a UA rule, or an engine feature in a
     crate the builder edits — is built by the builder and lands before the exposer (A102's prerequisite side); a
     repair that is a whole program, whichever lane owns it (item 12), is an accepted, cell-pinned divergence
     (A102), recorded by the exposer. A96 → a slot,
     registered by the sub-PR whose plan found it. **Boundedness**: the population a sub-PR's plan measures is its
     own new outputs × their readers — each channel the PR changes, crossed with the markup whose value on that
     channel it changes — not the space of all markup.

     **Verification against the ratified precedents** (witnesses from the umbrella, re-read at `e2d62b9e`; umbrella
     line numbers):

     | Precedent | M | O | HEAD | after X | Test says |
     |---|---|---|---|---|---|
     | **A95** (`:794`; cell 6j `:2661`) | `img::before { content: ""; padding: 10px }` on an `<img>` | item stream | no item (the pseudo is inert: the collect branch pushes a `Text` only for non-empty content) — spec: no pseudo, so nothing ✓ | PR-1a: a marker pair ✗ | A93, deadline PR-1a ✓ |
     | **A94** (`:793`; cell 15e `:2994`) | `<p style="width:W">a<span style="padding-left:20px">bbbb</span></p>`, W in the stated interval | line count | 1 ✓ | PR-1b: 2 ✗ (the advance precedes the break test) | A93, deadline PR-1b ✓ |
     | **A80** (`:779`) | a `position:relative` background-only span after a decorated inline with content, then a restyle | display list (the span's painted background vs its glyphs) | aligned ✓ (`border_box()` = content, no edges) | PR-1b: glyphs move, the background stays at the first-layout box (`assign_inline_layout_boxes` skips an entity with a `LayoutBox`) ✗ | A93, deadline PR-1b ✓ (the umbrella pins no cell: the channel needs a restyle and a second layout) |
     | **min-content** (A47 `:746`) | an `auto`-width inline-block holding `<span style="padding:0 20px">word</span>` at a small available width | `LayoutBox` (the line fits its box) | fits ✓ (both sides edge-free) | PR-1b: the line needs word + edges, the box stays word-wide → overflow ✗ | A93, deadline PR-1b ✓ |
     | **reqs 8/9 → E13/E14** (`:5501-5516`; cells 14g `:3710`, 14c–14e, 21) | `<p><a style="transform:translateX(1000px)"><b>x</b></a></p>`; an observer on `<a><b>x</b></a>`'s `<a>` | hit test; observer read | `x` hit where painted ✓; `(0,0)` ✓ ("conformant by accident") | PR-1c: the `<a>` gets a box and an ungated transform shifts the `<b>` ✗; a non-empty `contentRect` ✗ | A93, deadline PR-1c ✓ (item 5(a)) |
     | **A96** (`:795`) | vertical-mode `rtl` padded inline | display list / `LayoutBox` edges | not the spec value: an inline's edges are zero at HEAD, so no inline-box cell's HEAD value includes them, and the edge-mapping shape is already wrong for a block box ("author-visible today with no inline box in the markup") | — | A96 ✓ (no cell has HEAD right) |
     | **A101/A102** (`:800`, `:801`; cell 12h `:2851`) | `<p>אא<span style="padding-left:10px">ב</span></p>` | display list (glyph positions) | same as without the padding — **not** the spec value (no gap at HEAD) | PR-1b: unchanged (the reorder branch drops the gap) | **no cell fires** → A96 by this test; the umbrella's A102 disposition (not ordered) gives the same outcome, and its own text says "the loss is reachable today with no marker involved, this program adding reach and not the defect" (`:489-490`) |

     **Which PR fires what** (so no site pins decoration on the wrong PR): PR-1a is behaviour-neutral in layout
     output (umbrella `:1542`, `:1564-1565`: "no layout output moves — but the item stream is deliberately not
     neutral") — its channel is the **item stream** (cell 6j); PR-1b moves the inline-axis advance (line count,
     positions); PR-1c makes the box's geometry real (`LayoutBox` edges, hit test, CSSOM/observer reads, and the
     painted rect of a `position:relative` inline only — a static inline's decoration is not painted before or
     after PR-1c, `:1797-1801`, cell 13d `:3632`, `#11-inline-decoration-paint-path`); PR-1d grants existence
     (M5's flip, M6, M7).
   - **Umbrella exposures → `IBP-ua-display` builds their repairs** (the umbrella PR owns each cell, the ownership
     rule above). Its
     plan-memo places, by the cell test, what PR-1a–PR-1d fire,
     with a measuring command and a statement of what the command cannot see (population: those PRs' new outputs ×
     readers); records any A102 divergence, cell-pinned; registers `#11-ua-sheet-html-rendering-display-fidelity`
     for the A96 remainder. **E12's deadline, derived**: the first X that fires one of its cells. An element HTML
     gives `display: none` and elidex computes `inline` fires on the item-stream channel at **PR-1a** —
     `<dialog style="padding:10px"></dialog>`: HEAD no marker (spec: no box) ✓, PR-1a a marker pair ✗ — so E12 stays
     PR-1a unless its plan's population has no item-stream cell, in which case the deadline moves to the first X
     its cells name — PR-1b, or PR-1c for the `LayoutBox`/hit-test channels. A PR-1b cell, e.g.:
     `<p>a<param style="padding:0 20px">b</p>`: no gap at
     HEAD (spec: `param` is `display: none`) ✓, a 40px gap after PR-1b, whose extent counts "every inline-axis edge
     a marker advances, whether or not the box holds text" (umbrella `:1595-1596`) ✗. **Seeds (candidates, not decisions)**: `area`,
     `base`, `param`, the non-exposing `audio`, `dialog`, `search`, `datalist`, `[hidden]`, `@media (scripting) {
     noscript { display: none !important; } }` (elidex has no `display` rule for any, `/usr/bin/grep -n` on `ua.rs`/`legacy_ua.rs`); the
     shadow-tree compat-sheet gap (`elidex-style/src/walk.rs:510-513`); the non-exposing `audio`'s forced `none`
     (F16). Its rules consume `IBP-css-machinery`'s `i` flag and `scripting` feature (E18) where they need them.
     Rev 10's worked partition (Appendix G) is input only, re-derived there.
   - **`content-visibility`** — its own program (item 11), a program-scale repair under item 12: not ordered ahead;
     each PR that fires one of its cells pins it.
   - **This program's exposures.** `IBP-layout`'s presence switch → `IBP-layout`, its population placed by the
     cell test in its own plan (seeds: the Hidden-state `input`'s `input[type=hidden i] { display: none !important
     }` (§15.3.1), §15.3.10's resets — `letter-spacing` is already painted today through the legacy paint path, so
     its cells are where HEAD is right; a layout-only effect on a subtree never laid out before). (F4 answering
     "replaced" to keep nothing exposed would key a classification on its consequence and is declined.)
     `IBP-sandbox` was checked above (`noscript`). Every other sub-PR's plan applies the cell test to its own
     changes.
   - **Cell 6k's missing alt text: no cell fires at any program PR** (the cell test). On the display-list channel the
     spec paints the alt text and HEAD does not — HEAD is already wrong, so no program PR can fire that cell (A96).
     On the item stream PR-1a's marker pair for `<img alt="x">` is the spec's output (rule (3) makes it an inline
     box) — nothing wrong is newly reachable; PR-1c's `LayoutBox` and PR-1d's M6
     height and M7 baseline for the empty non-replaced `<img alt="x">` are what an inline box gets, so they move it
     toward the spec, not away; and the alt text is absent before and after every one of them — nothing newly
     visible to an author. `IBP-layout`'s F5 removal adds one path: an `<img src=a.png alt="x">` whose `src` is then
     cleared. Today that entity renders visibly wrong — it keeps painting the stale pixels, where steps 11/13 discard
     the image data and rule (3) makes it represent its text. `IBP-layout` removes the stale pixels and makes it an
     empty non-replaced inline — toward the spec, with only the alt text still missing. Supporting that per-entity
     reason, the missing alt text is the defect the static `<img alt="x">` (no `src`) already shows an author today
     (A96). It stays a slot (§6).
   - **Reqs 8 and 9** (`IBP-transform`, `IBP-observer`): PR-1c gives inline boxes the
     `LayoutBox` that an ungated transform reader or the ResizeObserver content rect then misuses — so they are
     ordered before PR-1c (E13/E14), the relaxation item 5(a) records.
8. **S5-4 plan §4.5** (`docs/plans/2026-07-s5-4-sandbox-enforcement.md:663-677`) — "no duplicate parsed-flag
   component is introduced pre-B1 (avoids a dual-SoT)" and the component migration "B1-gated": decision (iv)
   introduces the component pre-B1 as the **single** store (`HostData.sandbox_flags` is deleted, not duplicated),
   user-approved.
9. **S5-4 plan §5.1 CORRECTION and S5-4 E5** (`:713-719`; the do-not-conflate note at `:466-468`; the edge at
   `:733` and its row at `:943`) — the platform-object composition the CORRECTION placed in
   `elidex-script-session::scripting` moves into `elidex-ecs` (F10), and S5-4 E5's "zero shared semantics" (and
   `:466-468`'s "a different concept") is superseded: F11 derives the parser's scripting mode from F10, which is
   HTML's own coupling (D10).
10. ~~A93 / A96 read at defect-class granularity~~ — **withdrawn (rev 23)**. The class reading was put to the user
   with one side's precedents only (A96's); measured against all of them it contradicts ratified **A94** (the
   item-boundary break is a class authors meet today through other breaks, yet A94 orders it), **A95** (an inert
   pseudo is reachable today, yet A95 orders it) and the min-content promotion (A47). Superseded by the per-cell
   reading, USER DECIDED 2026-09-28 (item 7's cell test), which reproduces every precedent (item 7's table). Rev
   22's class placements are void; the rev 5–9 instance placements are re-derived by the owning plans under the
   cell test.
11. **A new `content-visibility` skip-contents program** (USER DECIDED 2026-09-28: option (d) for its content;
   option (A) for its ordering, item 12): its own umbrella memo and per-PR plan-review, **not** one of this program's
   sub-PRs. **Content** (unchanged from option (d)): applies-to per css-contain-2's size-containment exclusions —
   no principal box (`display: contents` / `none`), inner display `table`, an internal table box, an internal ruby
   box, a non-atomic inline-level box — consuming F1 from the start (E20; a `display: inline` replaced element is
   atomic, so it applies); the UA rule `[hidden=until-found i]:not(embed) { content-visibility: hidden; }` (HTML
   §15.3.1), its `i` flag from `IBP-css-machinery` (E22); a used-time *skipped contents* flag read by paint, hit
   testing, accessibility, focus and selection (computed `visibility` untouched); `details`' UA shadow tree (elidex
   has shadow/slot infrastructure, `elidex-ecs/src/dom/shadow.rs`; the first UA shadow tree); ancestor revealing on
   fragment navigation (elidex has it, `elidex-shell/src/app/navigation.rs:13`). Out: layout skipping (the spec's
   SHOULD); container-side containments → `#11-css-containment` (A96: `<div hidden="until-found">tall</div>` is tall
   today); `auto` → `#11-content-visibility-auto` (declared subset). **Order**: its own plan's; with no deadline
   (item 12) it follows its inputs, `IBP-predicate` (E20) and `IBP-css-machinery` (E22). Revs 23–26's deadline
   clauses (PR-1c, PR-1b, the property form, "part 1 before `IBP-sandbox`") and the part 1 / part 2 split are
   **withdrawn**; E21, E23 and E24 are withdrawn.
   **Pinned cells** (§0.6 item 7: the exposer records them — each only if the cell fires at the exposer's actual base,
   which the exposer's plan checks; each asserting elidex's value, naming the conformant
   value and the flip — the `content-visibility` PR that wires skipped contents into the reader the cell reads,
   hit testing here, as the program's umbrella names it), the known firing PRs, all hit-tested inside
   `<div hidden="until-found" style="height:100px">`: **`IBP-sandbox`** — in a scripting-disabled document,
   `<noscript><span style="display:inline-block;width:50px;height:50px"></span></noscript>` at (10px, 40px):
   elidex after `IBP-sandbox` hits the `span`, conformant the `div`; **`IBP-css-machinery`** — an author rule
   `[data-k=V i] { display: inline-block; width: 50px; height: 50px }` on `<span data-k="v">` (before it,
   `parse_attribute_matcher` leaves the `i` unconsumed inside the `parse_nested_block` at
   `elidex-css/src/selector/parse.rs:187-199`, which cssparser 0.37 runs through `parse_entirely`, so the selector fails):
   elidex hits the `span`, conformant the `div`; **`IBP-layout`** —
   `<button style="width:10px"><span>longlabel</span></button>`, the span's overflow: elidex hits the `span`,
   conformant the `div`; **PR-1b** — `a<span style="padding-left:20px">b</span>` at the centre of `b` as PR-1b
   places it: elidex hits the `span`, conformant the `div` (the umbrella's PR-1b pins it, §6 umbrella record);
   **PR-1c** — `<a style="padding:10px"><b>x</b></a>` at a point in the `<a>`'s left padding: HEAD — PR-1c's base,
   after PR-1b, whose advance has moved `x` right by the padding — the `div` (the `<a>` owns no run and has no
   `LayoutBox`, and the point is left of `x`), after PR-1c the `<a>` (its box and real edges, `:5514-5516`,
   `:1810-1811`) — conformant the `div`; **PR-1d** — `<span style="padding:10px"></span>` at a point in its padding:
   HEAD — PR-1d's base, after PR-1c — the `div` (an empty inline still has no `LayoutBox`), after PR-1d the `span` (its first `LayoutBox`, `:1842-1843`) —
   conformant the `div`. **Ruled out: PR-1a** — skipped contents keep defined layout (css-contain-2 skips their painting,
   hit testing and user-agent features; skipping layout work is only a SHOULD, undone by any API that reads it), so the item stream, line count and `LayoutBox` channels
   PR-1a changes carry the spec's value inside a skipped subtree too; **`IBP-ua-display`** — its rules only hide,
   so it fires no cell here, and umbrella exposures whose repair is this program are not its population (item 7).
   Others found later are owned per the ownership rule (item 7). `<img src="a.png" hidden="until-found">` needs no pin: HEAD
   already paints it (HEAD ≠ spec), and the program repairs it.
   Home: slot `#11-content-visibility-skip-contents` (§6). **Owner lane: this Layout lane** (default; the user may
   assign another). Its umbrella memo is authored and approved **before the first exposer, `IBP-sandbox`, lands**
   (E26 — a docs artefact, not the implementation), so every pin names an existing program.
12. **Umbrella ledger amendment to A102 — USER DECIDED 2026-09-28 (option (A))** (A102 stands; the same idiom as
   A102 amending A93). A102's separator — "the owner and size of the repair" — is read as **the size of the repair:
   a whole program with its own umbrella and slot, whichever lane owns it**; an owner-less repair does not qualify
   (a slot and an umbrella are required). **Ratio**: a program-scale repair is never ordered ahead of this lane —
   A102's "the gate the user declined". **Precedent check**:

   | Precedent | Repair | Under the amendment |
   |---|---|---|
   | A47 (min-content), A80 (reconciler), A94 (item-boundary), A95 (replaced-origin pseudo) | bounded, in this program's own crates | still ordered as prerequisites (unchanged) |
   | A96 | none (no cell fires) | unchanged — a slot |
   | A101 / A102 (bidi) | `#11-bidi-full-uba-fidelity`'s program | the same judgment (not ordered; pinned) |
   | A93 | — | trigger unchanged; the amendment changes only the disposition of a fired A93 |
   | `content-visibility` (item 11) | a whole program with an umbrella and a slot (`#11-content-visibility-skip-contents`) | not ordered; its cells pinned by their exposers |

   The condition "slot + umbrella" is met by item 11 as recorded there (owner, slot, and the umbrella memo approved
   before `IBP-sandbox`, E26). **Umbrella surfaces this reading differs from**, left as ratified (the A114 idiom):
   §5.3's A102 paragraph, "What separates it from a promotion is the owner and the size of the repair, not the
   reach" (`:1861-1866`), and A102's row, "The separator is the owner and size of the repair" (`:801`). Its
   record ships with the approval PR (§6).
13. **Umbrella per-PR own-deferral counts — USER DECIDED 2026-09-28**: a pinned `content-visibility` cell is the
   exposing PR's own deferral (item 7). The umbrella's counts (`:2379`; statements `:1787`, `:1837`, `:1846`; check 9
   `:2415`) change **conditionally on the pin firing at that PR's actual base** (item 11); §10's `(own)`-tagged rows that check 9 reads
   (PR-1c's and PR-1d's) and the §5.3 statements are left as ratified, the umbrella ledger row being the record. The counts: **PR-1b 0 → 1, PR-1c 1 → 2,
   PR-1d 1 → 2**, each within the cap of 3. Grounds, both sides: for — `feedback_defer_cap_policy.md:12` counts own
   deferrals and `:16` defines own as a defect the PR brought in, and under the cell test the PR turns a cell that
   is right at its base wrong; against — the umbrella's bidi precedent (`:486-490`) treats a divergence whose repair
   lives in another program's slot as "no deferral of this one's"; decided for, the difference being that a cell
   fires here and none did there. Its record ships with the approval PR (§6).
14. **Umbrella req 8** — widened: its gate is the predicate's *non-replaced inline box* answer; F12 carries
   css-transforms-1 §1.2's box-model clause whole, so `table-column` and `table-column-group` boxes are also
   non-transformable (the definition names all three; F12 under that name must not answer part of it). The
   consequence is `IBP-transform`'s to derive by item 7's cell test: HEAD gives a `col`/`colgroup` no `LayoutBox`
   (`elidex-layout-table` keeps its style for widths and collapsed borders, `helpers.rs:109-157`, and lays out no box for it), but the fixed-descendant scan (`positioned/mod.rs:215`) walks DOM
   children, so a script-inserted fixed-position child of a transformed `colgroup` — dropped with it at HEAD, which
   matches CSS 2 §17.2.1's treatment of a column's non-column children — escapes to the viewport once the
   `colgroup` is non-transformable (read from the code, not run). Its record ships with `IBP-transform` (§6).

**Surfaced to the user** (information, recorded): d5 departed from a user-confirmed judgment; the grounds are
§0.5; the user re-approved in form (iv).

## §1. Facts and their homes (decisions)

One representation per spec fact; one composition per spec predicate; each representation's writer set closed.

| # | Spec fact | Representation (decision) | Home | Writers (closed set) | Owning sub-PR |
|---|---|---|---|---|---|
| F1 | css-display-3 §A *inline box* | one composition, `EcsDom::is_inline_box` (NEW): display facet ∧ ¬replaced ∧ ¬(uses button layout — F7's button-layout class: `button`, and `input` in the Submit/Reset/Button and Color states; HTML §15.5.3's inline→`inline-block` coercion and new formatting context, §0.6 item 3), pseudo arm from `content` | `elidex-ecs` | — (query) | IBP-predicate |
| F2 | outer inline / inner flow; inline-level with inner not flow | `impl Display` facets; `is_atomic_inline` subsumed | `elidex-plugin` | — | IBP-predicate |
| F3 | a pseudo's replacedness (css-content-3 §1) | a match total over `ContentValue`/`ContentItem`; elidex **adopts** the non-normative Issue #2889 reading (`<content-list>` for `::before`/`::after`) as its choice | `elidex-plugin` | — | IBP-predicate |
| F4 | *replaced element* (HTML §15.4, css-ui-4 §7.2, SVG2 §5.1.3 outermost `svg`) | one query, `EcsDom::is_replaced_element` (NEW), live over identity (namespace, local name, attributes), F5–F7, F8 only if the probe keeps it (§1 F8) (an `img`'s F5/F6 through `image_request_state`, §1 F6), F16, and F10 through F10r (never F9 directly); its consumers include, from `IBP-layout` on, every site that today uses component presence to stand for **replacedness** (I1); it answers a native-appearance widget replaced (css-ui-4 §7.2), except the rows the probe decides — `button` and `input` in the Submit/Reset/Button (§15.5.12) and Color (§15.5.9) states, where HTML's button layout (§15.5.3) suggests otherwise (D1) | `elidex-ecs` | — | IBP-classify |
| F5 | image **available** (§4.8.4.3: partially or completely available) | `ImageData` presence — no second encoding; elidex produces only complete decodes, so "partially" has no producer (`#11-img-dynamic-image-request`) | `elidex-ecs` | the loader's insert; canvas sync (existing); **removal** at *update the image data* steps 11/13 on a null or unparsable new source (new) | IBP-layout (the removal transition, I1) |
| F6 | image request **broken** with a **non-null** selected source (step 11's broken state, a null source, is identity) — read only through **one composition**, `EcsDom::image_request_state` (NEW; the spec's current-request state: null source → broken, F6 → broken, F5 → available, else pending), which F4 and any later reader (`complete`, the alt-text slot) call; no reader tests F6 or F5 alone for request state | `ImageRequestBroken` (NEW), a marker carrying only what no other component says — a failed request (the loader) or an unparsable new source (step 13); a **null** selected source (step 11) is identity (`src` absent or empty, §0.3 arm (i)), so step 11 clears F6 rather than setting it. F4 reads the current request's state in that order — null source (identity), else broken (F6), else available (F5), else pending — so a stale `ImageData` left by steps 11/13 before `IBP-layout`'s F5 removal is never read as available | `elidex-ecs` | the loader (sets); the attribute reconcile seam (sets at step 13, clears at steps 11 and 18); its clone-policy row (`elidex-ecs/src/dom/tree_clone.rs:13-39`; a step-13 F6 derives from attributes and the base URL, not from a request outcome, so the row is decided between `ImageData`'s non-copy and a re-derive on the clone — IBP-classify's input) — all IBP-classify, so the marker lands with its whole writer set | IBP-classify |
| F7 | widget; devolvable vs non-devolvable (HTML §15.5.x); the elements using button layout (§15.5.3) | `EcsDom::is_native_widget` (NEW) and `EcsDom::uses_button_layout` (NEW; the button-layout class, read by F1) — the base of the widget-rendering conditions `IBP-layout`'s plan derives for every site that today uses component presence to stand for **widget rendering** (I1); the `input@type` mapping is **one** pure mapping in `elidex-plugin`, absorbing **every production site that maps an `input@type` string to a kind or a predicate** (the site list, its measuring command and what the command cannot see are `IBP-classify`'s input, Appendix B) | `elidex-ecs`; mapping in `elidex-plugin` | — | IBP-classify |
| F8 | devolved widget (css-ui-4 §7.2.1) | a style-computed input, always built: the widget-rendering condition (native appearance = F7 ∧ ¬F8, `IBP-layout`'s) reads it whatever the probe finds; F4 reads it, in the same pass after the cascade, **only if** the probe shows devolution changes replacedness | `elidex-style` → `ComputedStyle` | the cascade | IBP-classify |
| F9 | a Window document's active sandboxing flag set (§7.1.5) | `ActiveSandboxingFlagSet(Option<IframeSandboxFlags>)` (NEW) on the Window-global EcsDom's document root. `IframeSandboxFlags` holds **allow tokens**, the inverse of the spec's restriction flags: `None` is the spec's **empty** flag set (unsandboxed); `Some(IframeSandboxFlags::empty())` is a `sandbox` attribute with no tokens — every restriction set | `elidex-ecs` | one write API, called only by the Window document-root creation entries (the parse entries and pipeline builders); **not applicable** to worker / service-worker document roots (`vm/worker_thread.rs:174`, `vm/sw_thread.rs:178`: non-Window globals) and to throwaway documents (DOMParser `elidex-form/src/inert_document.rs:119`, the fragment scratch document `elidex-ecs/src/dom/tree/teardown.rs:326`) — their scripting answer never reads their own flag set (F10) | IBP-sandbox |
| F10 | scripting is disabled for a platform object (§8.1.3.4) | **one composition**, `EcsDom::scripting_disabled_for` (NEW). *Settings clause* — its sandbox condition applies only when the relevant settings object's global is a Window (§8.1.3.4: "Either settings's global object is not a Window object, or … sandboxed scripts browsing context flag"): the settings document is the EcsDom's `document_root` (field `dom/mod.rs:55`, written only by `create_document_root`, `:450-454`); the global kind has one discriminator: **non-Window exactly when the DOM holds a worker-VM scope root and no Window root** (`worker_scope_entity().is_some() && window_entity().is_none()`), every other DOM — incl. an engine-only build with neither root — being a Window's. Neither call alone is the test: a Window DOM also holds `NodeKind::Worker` entities, one per `Worker` object (`vm/host/worker.rs:383-386`; `dom/mod.rs:532-543`), and an engine-only build has no Window root. The test rests on the invariant **a worker DOM holds no nested `Worker` entity** (the `Worker` constructor is Window-scope only, `elidex-js/src/vm/globals.rs:597-599`); if it breaks, `worker_scope_entity()` returns `None` for that DOM, the worker is classified a Window, its unstamped root reads through F10r as fully sandboxed, and scripting is disabled for it — the failure fails closed. *Node clause* — "node document's browsing context is null": its own fact; interim representation the effective-node-document rule (`elidex-script-session/src/scripting.rs:96-111`) promoted unchanged (it fails **open** when the node's document cannot be resolved; it inherits the known `<template>`-contents false negative, `elidex-script-session/src/scripting.rs:59-60`, `#11-template-contents-owner-document`); final representation the enriched `#11-domparser-full-document-parse-fidelity` marker. *Window clause* — the same fact, read for the Window's document. `scripting.rs` delegates and drops its `sandbox_flags` parameter | `elidex-ecs` | — | IBP-sandbox |
| F10r | the flag read path | **one** engine-side read path for every flag gate (scripts, forms, popups, modals, top navigation, `window.open`, and F10's settings clause): it maps a **missing** F9 on a Window settings document to `Some(IframeSandboxFlags::empty())` **before** any `elidex_plugin::sandbox` predicate runs, so **flag absence** can never fail open; the predicates keep `None` = empty flag set. (The node clause's fail-open on an unresolvable document is a different case, kept as the interim rule states.) | `elidex-ecs` | — | IBP-sandbox |
| F11 | parser scripting mode (§13.2.4.5) and fragment scripting (§13.4 step 10) | derived from F10 — for a document parse, of the Document being created, through **the existing single function** `elidex_plugin::sandbox::scripting_enabled(flags)` (`elidex-plugin/src/sandbox.rs:179`, already the settings-level rule `elidex-script-session/src/scripting.rs:89` and `elidex-js/src/vm/host/event_handler_attrs.rs:611` evaluate), after F10r's missing-flag mapping: F10 applies it to F9; a document parse applies it to the **creation input** — the same flag set F9 is stamped from — the strict parser after the root exists, the tolerant parser before any `EcsDom` exists (html5ever's `ParseOpts` are fixed at `elidex-html-parser/src/lib.rs:49`, and `convert.rs:17-18` creates the root afterwards); invariant (IBP-sandbox's): *parser mode == F10's settings clause for the created document*; for a fragment parse, of the **context element**'s document — at **every production caller of a fragment parse** (the caller list and its measuring command are `IBP-sandbox`'s input, Appendix A). Only the **Disabled** override comes from F10 (§13.4 step 10; for a document parse, §13.2.4.5's Normal/Disabled); the **Inert** and **Fragment** modes stay caller-supplied (fragment parsing's default Inert; `createContextualFragment`'s Fragment). The `scripting_disabled` field of `SetInnerHtmlOptions` (`html_fragment.rs:39`) and of `ParseFragmentOptions` (`elidex-html-parser-strict/src/result.rs:120`) is **deleted** and no caller supplies it — one source. At the DOMParser call (`elidex-form/src/inert_document.rs:142-143`, today `scripting_disabled: true`) the derivation yields Disabled: the context element's document is a throwaway one with no browsing context (F10's node clause). `createContextualFragment` (`elidex-dom-api/src/range/mutation.rs:620-628`, a forward stub; `#11-range-create-contextual-fragment`) joins the population when it parses | the parser crates (`elidex-html-parser`, `elidex-html-parser-strict`); `elidex-script-session` (fragments), fed by the setter paths (`elidex-js/src/vm/host/dom_inner_html.rs:403`/`:458`, `SetInnerHtmlOptions::default()`; `:419-421`/`:474-476`, `scripting_disabled: false`) | — | IBP-sandbox |
| F12 | *transformable element* (css-transforms-1 `#transformable-element`, the draft's Terminology section — its heading does not render, so the anchor falls inside §1.2 CSS Values: `webref dfn css-transforms-1 "transformable element"`, `webref body css-transforms-1 css-values`) | `EcsDom::is_transformable_element` (NEW), the definition's box-model clause whole: an element whose layout is governed by the CSS box model, except an inline box (F1) and a `table-column` or `table-column-group` box (both `Display` variants exist, `elidex-plugin/src/computed_style/display.rs:25-26`); the SVG clause is omitted (§3) | `elidex-ecs` | — | IBP-transform |
| F13 | ResizeObserver content rect (§3.3.1) and observed-box size (§3.4.8, isActive §3.1) | engine-side in `elidex-api-observers`; the host closure only marshals | `elidex-api-observers` | — | IBP-observer |
| F14 | natural size; monolithic set | `natural_size` (NEW) contract for every replaced element (inputs in §2); monolithic = every replaced element (**elidex's choice**; css-break-3 §4.1's "many types of replaced elements" is illustrative) | layout crates | — | IBP-layout |
| F15 | the repairs of umbrella-PR exposures §0.6 item 7's rule gives `IBP-ua-display` | UA rules in the core / compat sheets; a computed-value force for a non-exposing `audio`; the engine features the rules need that `IBP-css-machinery`'s population does not hold — needs found after that plan fixes its population, which §0.6 item 7's machinery clause gives to the repairer — where bounded (A102); the population and those features are derived by `IBP-ua-display`'s plan | `elidex-style`, `elidex-dom-compat`, `elidex-css` | the cascade | IBP-ua-display |
| F16 | a media element **exposes a user interface** (§4.8.11.13) | elidex **policy**: exposes iff `controls` is present or scripting is disabled for the element (F10) — the spec's *should*; the *may* (controls without the attribute) is not taken | `elidex-ecs` (query) | — | IBP-classify |
| F17 | the `scripting` media feature's value (mediaqueries-5 §9.1) | a `MediaEnvironment` field: `enabled` / `none` from F10's settings clause for the document whose styles are resolved; `initial-only` is not produced (a declared-subset choice) | `elidex-css` (`MediaEnvironment`, `media/types.rs:289`) | closed: the two `MediaEnvironment` producers — the cascade's (`elidex-style/src/lib.rs:366`) and `matchMedia`'s (`elidex-js/src/vm/host/media_query.rs:357`, marshalling only) — both reading F10 | IBP-css-machinery |

**Home grounds.** Every consumer (layout-block, layout, render, dom-api, api-observers, style) depends on
`elidex-ecs`. `elidex-dom-api` cannot host these facts: `elidex-style` is a consumer (the pseudo prereq) and
`elidex-dom-api` depends on `elidex-style` (`sed -n '/^\[dependencies\]/,/^\[/p' crates/dom/elidex-dom-api/Cargo.toml`
lists `elidex-style.workspace = true`), so the edge would be a cycle. `elidex-ecs` already hosts small HTML
predicates (`is_base_element`, `dom/mod.rs:235`). Identity is a live query (a marker would be a second copy of
`TagType`/`Attributes`); F6 and F9 are components because no other component holds those facts. The
`HtmlElementHandler` registry (`elidex-plugin/src/traits.rs:214`) has no production caller, so no hook is added.
The F7 mapping move edits `elidex-form-core/src/lib.rs:232-279`, hunk-disjoint from the live L3 branch's
`:173-180` and `:420-432` hunks (§4.1).

## §2. Program-level coupled invariants (pairs that cross sub-PRs)

- **I1 One answer** per spec predicate (F1, F4, F7, F10). The **presence-keyed population** is defined by the
  property: every production site, in any crate — layout, paint, hit testing, accessibility or other — that uses
  component presence (`ImageData`, `FormControlState`, `IframeData`, …) to stand for **replacedness** or for **widget
  rendering** (control content or chrome), or to gate whether an element's children are laid out or painted. Pure
  paint-source reads are outside it unless they gate children or chrome: the `ImageData` pixel read
  (`elidex-render/src/builder/walk.rs:376`) only supplies pixels and stays as it is; the `IframeDisplayList` read
  (`:393-406`, whose comment says it will "skip child painting") gates children and is inside. `IBP-layout` owns the
  population, measures it in its own plan-memo with a command and a statement of what the command cannot see, and
  switches every member **together**, each onto the predicate matching its purpose — replacedness sites onto F4;
  widget-rendering sites onto F7-based conditions its plan derives — such that **each element gets exactly one
  defined rendering** (rev 14's worked analysis is input in Appendix D). **Spec-fixed content sources — inputs to `IBP-layout`'s plan, independent of F4's answer** (the probe
  decides only replacedness): the `button` element — "The button element represents a button labeled by its
  contents." (HTML §4.10.6, `webref body html the-button-element`); an `input` in the Submit Button, Reset Button or
  Button state — "its contents are expected to be the text of the element's value attribute, if any, or text
  derived from the element's type attribute in an implementation-defined (and probably locale-specific) fashion, if
  not" (§15.5.12), read branch by branch: the `value` attribute whenever it is present, `value=""` included, and
  type-derived text only when it is absent; an `input` in the Color state — "a single block-level block container
  child box, which is expected to have a presentational hint setting the 'background-color' property to the
  element's value" (§15.5.9); and no `input` state takes DOM children as content (its content model is nothing,
  `webref body html the-input-element`). Every consequence of the presence switch is derived, placed and, where it
  stays A96, registered by `IBP-layout`'s plan (§4 obligations). Premise: today `FormControlState` presence makes
  the element replaced and its children are not laid out (`elidex-layout-block/src/helpers.rs:405-418`, read at
  `block/mod.rs:224`; the replaced branch `:380` precedes the children branch `:401`). Both predicates come from `IBP-classify`, so E4 orders them. **No F4 consumer and no writer of an input a presence-keyed site reads (F5) lands
  while the presence-keyed sites answer "replaced" differently** — i.e. before `IBP-layout`. **I2 Representation, not pixels** — F4 keys on F5/F6 and identity, never on decode timing. `#11-img-alt-text-rendering`'s
  trigger (a change to `<img>` painting at `walk.rs:376`) does not fire at `IBP-layout`: that read is outside I1's
  property and is not edited; the F5 removal changes whether `ImageData` exists, not how it is painted.
  **I3 Domain** — every `ComputedStyle` entity, incl. the pseudo. **I4 Reachability** — F1/F4/F10 in `elidex-ecs`.
  **I5 Layering** — `vm/host/` and the shell marshal only. **I6 Liveness** — replacedness changes over an entity's
  life; consumers re-read per pass; each transition has an owner (F5 removal: IBP-layout; F6's loader write and
  `src`-mutation transitions: IBP-classify — F6 is read by no presence-keyed site, so I1 does not hold them back). **I7 Stamp-at-creation** — F9 is on every Window document root
  from creation, before any cascade or parse decision reads it.

Cross-sub-PR pairs:
- **IBP-sandbox × IBP-classify** (I7×I2): F4's canvas and audio arms read F10/F16 through F10r; F9 is stamped at
  creation, so no pass reads an unstamped pipeline document. F10 is now evaluated per pass, so the global-kind
  test's two linear scans (`window_entity()`, documented "only invoked off the hot path", `dom/mod.rs:499-500`;
  `worker_scope_entity()`, `:532-543`) are an IBP-sandbox obligation.
- **IBP-classify × IBP-ua-display** (I1): F15 reads what its plan derives it needs — F16 (owned by IBP-classify)
  for a forced `none`; F10 reaches its scripting-conditioned rules through `IBP-css-machinery`'s `scripting` feature
  (E18) (no F4 read, so I1 gives no ordering beyond E7).
- **IBP-sandbox × IBP-css-machinery** (E17): the `scripting` media feature's value is F10.
- **IBP-css-machinery × IBP-ua-display / IBP-layout / the `content-visibility` program** (E18, E19, E22): the
  selectors-4 §6.3 `i` attribute-selector flag, which elidex lacks (`AttributeMatcher` carries no case flag,
  `elidex-css/src/selector/types.rs:64-77`; `match_attr` compares values as stored, `matching.rs:322-341`), the
  `scripting` feature (`IBP-ua-display`) and any selector-list form a repair needs; any other machinery follows
  §0.6 item 7.
- **IBP-ua-display × IBP-layout** (E25, one builder per repair): the missing
  `[hidden]:not([hidden=until-found i]):not(embed) { display: none }` is exposed by `IBP-layout` through
  `<button><span hidden>x</span></button>` (hit test, item 7's frame — IBP-layout's base without the `[hidden]`
  rule, D's own repair: HEAD the `button`, whose children are not laid out ✓; after
  `IBP-layout` the `span` ✗) and by PR-1a–PR-1d through their cells — e.g. `<p><span hidden style="padding:10px"></span></p>`, item
  stream: HEAD (PR-1a's base without the rule) no marker, the spec's (no box) ✓; PR-1a a marker pair ✗ — which count
  as `IBP-ua-display`'s for building (§0.6 item 7): IBP-ua-display is the builder because umbrella cells are its to
  build, not by the symmetric tie-break; `IBP-ua-display` was unordered with `IBP-layout` and its other inputs (E7, E18) already
  precede it, so E25 makes `IBP-ua-display` the earliest builder-eligible PR: it builds the rule and `IBP-layout`
  consumes it. Their other `ua.rs` rules are disjoint.
- **IBP-classify × IBP-layout** (I1): every presence-keyed site (I1's property, any crate) converges on the predicate it
  stands for — replacedness sites on F4, widget-rendering sites on F7-based conditions its plan derives — and the
  `src`-mutation F5 removal (steps 11/13) lands with them, before any other F4 consumer; F6's transitions are
  already IBP-classify's. F14's inputs per replaced
  class: available `img` (`image_request_state`) → F5's `ImageData` dims; pending `img` → none yet (the probe decides the used size);
  rule-(4) `img` → 0×0; `input` Image Button → §15.4.2's button whose content is the alt text ("about one line in
  height and whatever width is necessary to render the text on one line"), elidex loading no `input` image
  (`#11-input-image-state`); `canvas` → its bitmap dimensions from `width`/`height`, not F5; `iframe` →
  `IframeData`; widgets → `form_intrinsic_size`; `video` / `embed` / outermost `svg` / exposing `audio` → the
  contract IBP-layout's memo fixes.
- **IBP-layout × pseudo prereq** (I1, I6): generation reads F4 every style pass, after the presence sites
  converge; F8 is computed by the cascade before generation reads F4 in the same pass; F5/F6
  transitions and attribute mutations reach a pass.
- **IBP-layout × IBP-predicate** (I1×I3): F1 composes F4 unchanged, and F7's button-layout class (both IBP-classify's, reached through E4→E5); the pseudo arm (F3) is F1's.
- **IBP-predicate × PR-1a** (I1): M1's emit test is F1 and nothing else (cells 6c, 6k, 6f).
- **IBP-predicate × IBP-transform / IBP-observer** (I1×I5): both consume F1 through F12/F13, engine-side.
- **IBP-sandbox × B1** (I7): F9's write point is B1 req 2's; B1 swaps the input's producer only.

## §3. Spec coverage map (which sub-PR owns which fact)

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSS DISPLAY 3 §A Glossary | *inline box*, *atomic inline*, *replaced element* | F1–F3 | IBP-predicate | ✓ | yes |
| CSS CONTENT 3 §1 Inserting and Replacing Content: the content property | Issue #2889 (non-normative) | adopted as elidex's reading | IBP-predicate | ✓ | yes |
| WHATWG HTML §15.4 Replaced elements | element list | F4 | IBP-classify | ✓ | yes |
| WHATWG HTML §15.4.1 Embedded content | embed/iframe/video; canvas; object; audio | F4 (canvas via F10r; audio via F16; object ordinary ✗ → slot) | IBP-classify | ✗ | yes |
| WHATWG HTML §15.4.2 Images | rules (1)–(4) in order; quirks disjunct compat | F4–F6 | IBP-classify | ✓ | yes |
| WHATWG HTML §4.8.3 The img element | what an `img` represents | F4 | IBP-classify | ✓ | yes |
| WHATWG HTML §4.8.4.3.5 Updating the image data | steps 11, 13, 18 on `src` mutation | F6 transitions; F5 removal | IBP-classify (F6); IBP-layout (F5) | ✓ | yes |
| WHATWG HTML §15.5.1 Native appearance | widgets; devolvable / non-devolvable per §15.5.x | F7 | IBP-classify | ✓ | yes |
| WHATWG HTML §15.5.3 Button layout | display bullet (`button` only; `inline-flex`/`inline-grid` kept); new formatting context (every button-layout element) | F7 (the class); F4 probe-decided for button-layout elements (HTML suggests non-replaced, css-ui-4 §7.2 replaced); both bullets read by F1 (§0.6 item 3) and laid out by IBP-layout | IBP-classify (class, F4); IBP-layout (the coercion and formatting context in layout); IBP-predicate (F1) | ✓ (probe-bound) | yes |
| WHATWG HTML §15.5.9 The input element as a color well | uses button layout; content = one block-level block container child box with a `background-color` hint (never DOM children) | F7; F4 probe-decided; content source fixed independent of F4 (I1) | IBP-classify (F4/F7); IBP-layout (content source) | ✓ (probe-bound) | yes |
| WHATWG HTML §15.5.12 The input element as a button | uses button layout; contents = `value` text, else type-derived text (never DOM children) | F7; F4 probe-decided; content source fixed independent of F4 (I1) | IBP-classify (F4/F7); IBP-layout (content source) | ✓ (probe-bound) | yes |
| WHATWG HTML §4.10.6 The button element | represents a button labeled by its contents | content source = children, laid out and painted, independent of F4 (I1) | IBP-layout | ✓ | yes |
| CSS UI 4 §7.2 Switching appearance: the appearance property | widget = replaced under `auto` | `auto` assumed; conflicts with HTML §15.5.3's button layout for `button`, the `input` button states and the Color state → probe; `none` ✗ → slot | IBP-classify | ✗ | yes |
| CSS UI 4 §7.2.1 Properties Disabling Native Appearance | devolution: author-origin cascaded value after revert/revert-layer rollback, 44 properties, devolvable vs non-devolvable (host language) | F8 (F7 for devolvability; F4's read of it conditional on the probe); the devolved rendering (primitive appearance): IBP-layout's widget-rendering condition — a CSS box and its control content without native chrome — measured for button layout, where HTML leaves it undefined (§15.5.3's note "Need to define the primitive appearance"); host-language exceptions (e.g. `select`'s drop-down devolved state, §15.5.16) per F7 | IBP-classify (F8); IBP-layout (the rendering) | ✓ | yes |
| SVG2 §5.1.3 Definitions | *outermost svg element* | F4 (replaced: elidex's reading, probed) | IBP-classify | ✓ | yes |
| WHATWG HTML §4.8.11.13 User interface | "should expose a user interface" | F16 (elidex policy) | IBP-classify | ✓ | yes |
| WHATWG HTML §7.1.5 Sandboxing | active sandboxing flag set, per Document | F9 | IBP-sandbox | ✓ | yes |
| WHATWG HTML §7.5.1 Shared document creation infrastructure | *create and initialize a Document object* | F9 write point | IBP-sandbox | ✓ | no |
| WHATWG HTML §7.3.2.1 Creating browsing contexts | initial `about:blank` document | F9 write point | IBP-sandbox | ✓ | no |
| WHATWG HTML §8.1.3.4 Enabling and disabling scripting | settings clause (Window only); node and Window clauses | F10, F10r | IBP-sandbox | ✓ | yes |
| WHATWG HTML §13.2.4.5 Other parsing state flags | scripting mode (Normal/Disabled/Inert/Fragment) | F11 | IBP-sandbox | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | fragment algorithm step 10 | F11 | IBP-sandbox | ✓ | yes |
| WHATWG HTML §15.3.1 Hidden elements | `[hidden]:not([hidden=until-found i]):not(embed)`; `[hidden=until-found i]` (`content-visibility`); `input[type=hidden i]`; `@media (scripting) { noscript }` | by the cell test (§0.6 item 7), in the exposer's plan: `[hidden]`, `noscript` — IBP-ua-display's plan (candidates); `input[type=hidden i]` — IBP-layout's plan (candidate); `[hidden=until-found i]` — the `content-visibility` program builds the rule (§0.6 item 11; `i` via E22); `[hidden]` — built by IBP-ua-display, consumed by IBP-layout (E25) | IBP-ua-display; `input[type=hidden i]`: IBP-layout ; `embed[hidden] { display: inline; height: 0; width: 0; }` — IBP-layout's plan (candidate): HEAD has no `hidden` rule (`ua.rs`/`legacy_ua.rs` 0 hits) and no `embed` handling in layout or paint, so an `<embed>` is an empty non-replaced inline and draws nothing — as the spec's 0×0 replaced box for a hidden one does; IBP-layout's F4 switch makes `embed` replaced and sized, so a bare `<embed hidden src=x>` turns ✗ there — sized by its natural size or CSS 2's 300×150 fallback where the spec's UA rule computes `width`/`height` to 0 — unless that rule lands in the same cut; the rule, not F14's natural size (which also sizes a visible `embed`), is the repair, so IBP-layout builds it with the switch. The witness carries no author sizing and no `width`/`height` attributes: in the spec's cascade an author declaration, and the attributes' presentational hints (author-level, zero specificity; in elidex generated only on the compat path, `elidex-shell/src/lib.rs:126-136`, from `elidex-dom-compat/src/presentational.rs:111`), beat that normal UA-origin rule, so with either the spec's size is the author's — IBP-layout's own cell (§0.6 item 7) | ✗ | yes |
| WHATWG HTML §15.3.10 Form controls | UA rules on form controls: the inherited `initial` resets on `button` and `input`; `input, button { display: inline-block }` (a `display` rule — already present, `elidex-style/src/ua.rs:128`, which also covers `textarea`/`select`); rules with `i` (e.g. `input:is([type=reset i], [type=button i], [type=submit i])`) | by the exposer (§0.6 item 7): computed values reaching subtrees `IBP-layout`'s presence switch lays out first are IBP-layout's; A96 remainder → `#11-form-control-ua-rendering-fidelity`; `:is()` and `i` from IBP-css-machinery if a repair needs them | the exposer (IBP-layout / IBP-ua-display) | ✗ | yes |
| WHATWG HTML §15.3.3 Flow content | `display` rules for flow content, `dialog`, `[popover]` | seed | IBP-ua-display | ✗ | yes |
| WHATWG HTML §15.5.5 The details and summary elements | closed `details`' `content-visibility: hidden`; `summary` `list-item` | closed contents: the `content-visibility` program (§0.6 item 11; `details`' UA shadow tree); `summary`: slot (Appendix G) | IBP-ua-display | ✗ | yes |
| WHATWG HTML §16.2 Non-conforming features | compat-sheet placement, incl. shadow trees | seed | IBP-ua-display | ✗ | yes |
| MEDIAQUERIES 5 §9.1 Scripting Support: the scripting feature | `scripting` media feature (value from F10) | F17; machinery; consumer `@media (scripting) { noscript }` (IBP-ua-display) | IBP-css-machinery ; omitted branch: `initial-only` — not produced (F17) | ✗ | no |
| WHATWG HTML §4.16.3 Pseudo-classes | `:popover-open` | not built: no popover exists, so the `[popover]` rule is authored in reduced form (Appendix G) | IBP-ua-display | ✗ | no |
| SELECTORS 4 §6.3 Case-sensitivity | the `i` attribute flag; the `s` flag | `i`: machinery; consumers `[hidden]:not([hidden=until-found i])` (IBP-ua-display, E18), `input[type=hidden i]` and any §15.3.10 `i` rule (IBP-layout, E19), `[hidden=until-found i]` (the `content-visibility` program, E22); omitted branch: `s` — no consumer in the program | IBP-css-machinery | ✗ | yes |
| CSS CONTAIN 2 §4 Suppressing An Element’s Contents Entirely: the content-visibility property | `content-visibility: hidden` (no support in elidex) | the `content-visibility` program (§0.6 item 11), not ordered ahead (item 12's A102 amendment), applies-to consuming F1 (E20); its cells pinned by their exposers; container-side containments → `#11-css-containment`; `auto` → `#11-content-visibility-auto` | the `content-visibility` program | ✗ | yes |
| CSSOM VIEW 1 §6 Extensions to the Element Interface | `client*` step 1; `scroll*` no inline clause | consumer of F1 | IBP-predicate | ✓ | yes |
| CSS PSEUDO 4 §4.1 Generated Content Pseudo-elements: ::before and ::after | suppression on replaced origin | consumer of F4 | pseudo prereq | ✓ | yes |
| CSS TRANSFORMS 1 §1.2 CSS Values | *transformable element* (`#transformable-element`, the unrendered Terminology section inside §1.2, F12): the box-model clause (non-replaced inline, `table-column`, `table-column-group` excluded); the SVG clause | F12 | IBP-transform ; omitted branch: the SVG clause (paint servers, `clipPath`, SVG renderable elements) — elidex has no SVG layout or paint model, so the clause's categories have no elidex counterpart: SVG elements are CSS boxes today (the box-model clause answers them), and if `IBP-classify`'s probe answers the outermost `svg` replaced, its descendants get no box after `IBP-layout` (E5→E8) | ✗ | yes |
| CSS TRANSFORMS 1 §2 The Transform Rendering Model | geometry; stacking context; fixed-descendant containing block; the absolute-position containing block on a transformable element | gated on F12, except the omitted branch | IBP-transform ; omitted branch: the absolute-position half on a transformed static block, whose absolute descendants are laid out twice (by reading, not run) — pre-existing, `#11-transformed-block-abspos-double-layout` (umbrella §3's css-transforms-1 §2 rows, A150) | ✗ | yes |
| CSS TRANSFORMS 1 §3 The transform Property | *Applies to*: transformable elements | gated on F12 (req 8's core row, umbrella cell 14g) | IBP-transform | ✓ | yes |
| CSS TRANSFORMS 1 §4 The transform-origin Property | *Applies to* | gated on F12 | IBP-transform | ✓ | yes |
| CSS TRANSFORMS 1 §5 Transform reference box: the transform-box property | *Applies to* | vacuous until represented (`transform-box` has no `ComputedStyle` representation); F12 gates it when it lands | IBP-transform | ✓ | yes |
| CSS TRANSFORMS 2 §5 Individual Transform Properties: the translate, scale, and rotate properties | *Applies to* of all three; the stacking-context and containing-block sentence on a transformable element | *Applies to* vacuous until represented (no `ComputedStyle` representation); F12 gates it when it lands | IBP-transform ; omitted branch: the sentence — the three have no `ComputedStyle` representation, pre-existing → `#11-transform-family-3d-and-containing-block` (umbrella §3's css-transforms-2 §5 rows, A150) | ✗ | yes |
| CSS TRANSFORMS 2 §7 The transform-style Property | *Applies to*; the `preserve-3d` sentence and its 3D-rendering-context clause on a transformable element | *Applies to* vacuous (`transform_style` has no reader, so requirement 8 has nothing to gate) | IBP-transform ; omitted branch: the transformable-element sentence and its used-value (3D rendering context) clause — `transform_style` has no reader beyond its writer, pre-existing → `#11-transform-family-3d-and-containing-block` (umbrella §3's css-transforms-2 §7 rows, A149) | ✗ | yes |
| CSS TRANSFORMS 2 §8 The perspective Property | perspective geometry; stacking context; the containing-block sentence on a transformable element | gated on F12, except the omitted branch | IBP-transform ; omitted branch: a non-`none` `perspective` establishing a containing block for all descendants — not implemented (every containing-block decision keys on `has_transform` alone), pre-existing → `#11-transform-family-3d-and-containing-block` (umbrella §3's css-transforms-2 §8 rows, A149) | ✗ | yes |
| CSS TRANSFORMS 2 §9 The perspective-origin Property | *Applies to* | gated on F12 | IBP-transform | ✓ | yes |
| CSS TRANSFORMS 2 §10 The backface-visibility Property | *Applies to*; the stacking-context and containing-block sentence on a transformable element in a 3D rendering context | gated on F12, except the omitted branch | IBP-transform ; omitted branch: that sentence — `backface_visibility` keys no stacking-context or containing-block decision, pre-existing → `#11-transform-family-3d-and-containing-block` (umbrella §3's css-transforms-2 §10 rows, A150) | ✗ | yes |
| CSS WILL CHANGE 1 §2 Hinting at Future Behavior: the will-change property | the stacking-context conditional (family entries); the two containing-block conditionals for `will-change` naming `transform` or `perspective` | gated on F12, except the omitted branch | IBP-transform ; omitted branch: the two containing-block conditionals — not implemented (`will_change` has no containing-block reader), pre-existing → `#11-transform-family-3d-and-containing-block` (umbrella §3's css-will-change-1 §2 rows, A149) | ✗ | yes |
| RESIZE OBSERVER 1 §3.3.1 content rect | inline box → empty | F13 | IBP-observer | ✓ | yes |
| RESIZE OBSERVER 1 §3.4.8 Calculate box size, given target and observed box | per observed box | F13 | IBP-observer ; omitted branch: `device-pixel-content-box` — `SizeProvider` supplies CSS-pixel sizes only (`resize.rs:19`), so there is no device-pixel input; → `#11-resize-observer-device-pixel-box` (existing, in the M4-12 roadmap's §H-8 backlog, not the open-slot list; trigger: HiDPI / canvas content-box precision, M4-13 paint coupling) | ✗ | yes |
| RESIZE OBSERVER 1 §3.1 ResizeObservation example struct | isActive on the observed box | F13 | IBP-observer | ✓ | yes |
| CSS BREAK 3 §4.1 Possible Break Points | monolithic (illustrative list) | F14 | IBP-layout | ✓ | yes |
| CSS IMAGES 3 §4.1 Object-Sizing Terminology | default object size | F14 | IBP-layout | ✓ | yes |
| WHATWG HTML §4.12.5 The canvas element | natural dimensions = bitmap | F14 | IBP-layout | ✓ | yes |

**Full enum?** records whether this memo enumerates every branch of the row's step with an owner and an answer:
✓ = all enumerated and answered here; **✓ (probe-bound)** = all enumerated, with F4's answer bound to `IBP-classify`'s
probe (decided by measurement, not here) and everything else answered here; ✗ = a branch left out (named in the
row, with its slot or reason).

**Breadth**: K=16 specs, M=50 rows → split default; the split is §4. Section numbers were looked up
(`webref heading <spec> <n>`).

### §3.1 User-input touch audit

Author-controlled throughout: element, `type`, `src`, `alt`, `controls`, `hidden`, `popover`, `open`, author
border/background, `display`, `content`, the transform family, the embedding `<iframe sandbox>`, and (through the
network) whether an image loads. No sub-PR widens exposure: F1/F4 classify; `IBP-transform` removes an effect from
non-transformable boxes (its `table-column` arm's consequence is its own cell, §0.6 item 14); F10r fails closed on flag absence; F15 hides what HTML hides.

## §4. Sub-PRs: list, order, gates, obligations

**Naming**: `IBP-*` (inline-box predicate), so no name reads as a PR-1x letter.

| Sub-PR | Scope (decision level) | Discharges | Gate |
|---|---|---|---|
| **IBP-split-ecs** | split `elidex-ecs/src/dom/mod.rs` (1075 lines) on its classification-query seam | 1000-line rule | terminal — pure move, no behaviour |
| **IBP-split-parser** | split `elidex-html-parser/src/lib.rs` (1017 lines) on its in-file test seam (`#[cfg(test)]` modules at `:260`, `:291`) | 1000-line rule | terminal — pure move |
| **IBP-sandbox** | F9, F10, F10r, F11 | d5 (iv); parser scripting mode | **own plan-memo + review** (security gates × document creation × parser × VM/shell) |
| **IBP-classify** | F4, F6 (marker and its whole writer set: loader write, `src`-mutation transitions; `image_request_state`), F7, F8, F16 + probe | reqs 3, 4, 5 (as amended), 6 | **own plan-memo + review** (identity × image request × sandbox input × devolution × probe) |
| **IBP-layout** | F14; every presence-keyed site (I1's property — layout, paint and any other crate, switched together, each onto the predicate matching its purpose); F5 removal at steps 11/13 (§4.8.4.3.5); every consequence of its presence switch, derived, placed and repaired or registered (§0.6 item 7's one-owner rule) | req 5's routing note; req 2 for replacedness | **own plan-memo + review** (sizing × fragmentation × multicol × natural-size contract × image-data algorithm, with a probe) |
| **IBP-predicate** | F1, F2, F3; `client*` | reqs 1, 2, 7; 3/4 composed side | **own plan-memo + review** (display × content model/pseudo domain × CSSOM `client*`) |
| **IBP-transform** | F12 and its readers | req 8 | **own plan-memo + review** (stacking × hit test × paint × positioned layout) |
| **IBP-observer** | F13 | req 9 | **own plan-memo + review** (inline empty rect × observed-box activation for every element × the observed box's writer transition) |
| **IBP-css-machinery** | F17; the CSS machinery a program repair consumes that elidex lacks (population by that property; seeds: the selectors-4 §6.3 `i` flag, the mediaqueries-5 §9.1 `scripting` feature, selector-list forms — `:is()`, a multi-compound `:not()` — if a repair needs them) | §0.6 item 7's machinery clause | **own plan-memo + review** (selector parse × match × media evaluation × F10 input) |
| **IBP-ua-display** | F15 | A93 (§0.6 items 6–7) | **own plan-memo + review** (axes per its plan; seeds: `display`, compat-sheet and shadow-tree placement, computed-value forces; placements by §0.6 item 7's cell test) |

Only the two pure moves need no plan-memo of their own; every other row is a node (below).

**Each other row is a node, not a declared single PR** (CLAUDE.md *Edge-dense work*). Its plan-memo applies the
rule to itself: it decides, and its plan-review checks, whether the node is a narrowly-scoped slice — then it is the
base case (CLAUDE.md *Edge-dense work* (c)) — or edge-dense, then divided into stacked, individually plan-reviewed
PRs ((a)). The gate column names the axes that decision weighs, not a verdict; a plan-review that passes a node
without deciding it does not make the node the base case. I1 fixes a **lower
bound**, not a license: `IBP-layout`'s presence switch contains at least every member switched together, with the F5
removal, and nested PRs before it add no F4 consumer and no F5 writer. If `IBP-layout`'s plan finds that
indivisible switch itself edge-dense, it records it as an exception for the user's approval, not as the base case. Once a node is divided, this memo's rules apply at
nested-PR granularity: §4's order rule gives the edges among its nested PRs and binds §4's edges to whichever nested
PR supplies or consumes the edge's fact; in §0.6 item 7's cell test "PR X" is a nested PR whose predecessors include
its node's earlier nested PRs; ownership of an exposure (§4's no-edge paragraph) and the own-deferral counts (§6) are
the exposing nested PR's.

**Order — one rule** (§0.6 item 5): an edge is **data flow** (the later PR consumes what the earlier supplies),
**I1** (§2), **A93** (before the first program PR at which a cell of the defect fires, §0.6 item 7's cell test), or **code predecessor** (the later PR edits what a
split creates). The full edge list:

| # | Edge | Kind |
|---|---|---|
| E1 | IBP-split-ecs → IBP-sandbox | code predecessor |
| E2 | IBP-split-parser → IBP-sandbox | code predecessor |
| E3 | IBP-sandbox → IBP-classify | data flow (F10) |
| E4 | IBP-classify → IBP-layout | data flow (F4, F7) |
| E5 | IBP-layout → IBP-predicate | I1 |
| E6 | IBP-layout → replaced-origin pseudo prereq | I1 (and data flow F4, transitively) |
| E7 | IBP-classify → IBP-ua-display | data flow (F16; F10 transitively) |
| E8 | IBP-predicate → IBP-transform | data flow (F1) |
| E9 | IBP-predicate → IBP-observer | data flow (F1) |
| E10 | IBP-predicate → PR-1a | data flow (M1 reads F1) |
| E11 | replaced-origin pseudo prereq → PR-1a | A95 (PR-1a's markers) |
| E12 | IBP-ua-display → PR-1a | A93 — the repairer of umbrella exposures lands before the earliest exposer, PR-1a |
| E13 | IBP-transform → PR-1c | A93 — PR-1c (req 8; relaxation §0.6 item 5(a)) |
| E14 | IBP-observer → PR-1c | A93 — PR-1c (req 9; relaxation §0.6 item 5(a)) |
| E17 | IBP-sandbox → IBP-css-machinery | data flow (F10, the `scripting` feature's value) |
| E18 | IBP-css-machinery → IBP-ua-display | data flow (the `i` flag; the `scripting` feature) |
| E19 | IBP-css-machinery → IBP-layout | data flow (the `i` flag; selector-list forms if a repair needs them) |
| E20 | IBP-predicate → `content-visibility` program | data flow (F1, its applies-to), no deadline (§0.6 items 11, 12) |
| E21 | — | withdrawn at rev 26 (the property-form deadline); number not reused |
| E22 | IBP-css-machinery → `content-visibility` program | data flow (the `i` flag for `[hidden=until-found i]`); restored at rev 27 (withdrawn at rev 26) |
| E23 | — | withdrawn at rev 27 (no deadline, item 12); number not reused |
| E24 | — | withdrawn at rev 27 (no deadline, item 12); number not reused |
| E25 | IBP-ua-display → IBP-layout | one builder per repair (§0.6 item 7): the `[hidden]` rule both expose |
| E26 | `content-visibility` umbrella memo's approval → IBP-sandbox | the A102 amendment's condition (§0.6 items 11, 12): the program exists before its first exposer lands — a docs artefact |

**No edge for the consequences of `IBP-layout`'s presence switch (rev 19).** Every consequence — first-time
layout of the subtrees of formerly presence-replaced elements, widget content and chrome paint, and the computed
values those subtrees now expose, including the Hidden-state `input`'s `display: none !important` rule and
§15.3.10's resets — is derived, placed and owned by `IBP-layout` itself (§4 obligations); the exposure and its
ownership sit in one node — in the exposing nested PR once it is divided (the node paragraph above) — so there is no ownership edge; E25 is a builder edge (§0.6 item 7's one-builder rule).
E15 (revs 12–18, `IBP-ua-display` → `IBP-layout`) is withdrawn. An edge reappears only if `IBP-layout`'s plan
places the cell's ownership in another sub-PR; §0.6 item 7's rule forbids that, so none does. The cross-sub-PR dependencies of these
repairs are machinery (`IBP-css-machinery`, E19) and the
`[hidden]` rule `IBP-ua-display` builds (E25). E15 and E16 (rev 20, `IBP-ua-display` →
`IBP-layout` for the `i` flag) are withdrawn; their numbers are not reused.

No edge between the pseudo prereq and IBP-predicate (§0.6 item 5(b)). IBP-ua-display has no F4 read and
writes no presence-site input, so I1 gives it no edge **from** IBP-layout, and no exposure of `IBP-layout`'s is its,
so A93 gives it none **to** IBP-layout; one builder per repair gives it E25 for the `[hidden]` rule. Acyclic: `IBP-css-machinery`'s only predecessor is IBP-sandbox (E17, and
transitively the splits and E26's memo approval), and its successors are IBP-ua-display (E18) and IBP-layout (E19); IBP-ua-display's
predecessors are IBP-classify (E7) and IBP-css-machinery (E18), and its successors PR-1a (E12) and IBP-layout (E25); IBP-layout's
predecessors are IBP-classify (E4) and IBP-css-machinery (E19). Neither new predecessor depends on anything after
IBP-layout, so no loop closes. The `content-visibility` program follows IBP-predicate (E20) and IBP-css-machinery
(E22) and precedes nothing (item 12); its umbrella memo's approval precedes IBP-sandbox (E26), a docs artefact
with no program predecessor, so E26 closes no loop; IBP-css-machinery precedes IBP-predicate (E19 → E5), so E22 adds no loop. E25
(IBP-ua-display → IBP-layout): IBP-ua-display's predecessors (IBP-classify, IBP-css-machinery, IBP-sandbox) contain
nothing after IBP-layout. Every F4 consumer — IBP-predicate, the pseudo prereq,
and through F1 IBP-transform, IBP-observer and PR-1a — is downstream of IBP-layout; the only F5 writer that is new
(the step-11/13 removal) is IBP-layout itself.

### Obligations per sub-PR (requirements, not mechanism; mechanism input is in the appendices)

**IBP-split-ecs / IBP-split-parser** — bodies byte-identical; each file under 1000 lines; no public path change.

**IBP-sandbox** (appendix A):
- Pin its `content-visibility` cell (the `noscript` witness, §0.6 item 11).
- F9 through one write API, called by every Window document-root creation entry (the parse entries incl.
  `parse_progressive`, the `LoadedDocument` path, `build_pipeline_interactive_shared`, and any other); worker /
  service-worker roots and throwaway documents are not stamped (F9's non-applicability).
- F10 as one composition in `elidex-ecs` — Window-only settings clause (global kind by F10's single test), the node
  and Window clauses; `scripting.rs` delegates; `#11-cross-document-adopt-on-insert` enriched (its proxy is
  promoted). F10 is read per pass: the global-kind test's linear scans (`window_entity()` `dom/mod.rs:499-500`, "only invoked off the hot
  path"; `worker_scope_entity()`, `:532-543`) must not become a per-element cost.
- F10r: every flag gate reads through the one path; flag absence on a Window document fails closed.
- F11: §13.2.4.5's four modes; fragment parsing follows its context element's document (§13.4 step 10) at every
  production fragment-parse caller (the list and its command: Appendix A); the `scripting_disabled` option field is
  deleted.
- Every gate site re-pointed; `HostData.sandbox_flags` and its trait/impl surface deleted (no back-compat).
- Must not break: S5-4b's order-proof tests (`elidex-shell/src/content_iframe_security_tests.rs`).
- Input lines: G1 URL-loaded parse path (`load_document` → `parse_progressive`); G2 fragment setter paths
  (`vm/host/dom_inner_html.rs:403`/`:458` pass `SetInnerHtmlOptions::default()`; `:421`/`:476` pass
  `scripting_disabled: false`); G3 fixture population (`create_document_root()`: 99 files / 204 calls in
  `elidex-js/src/vm/tests`, 180 files / 510 calls workspace-wide — verified 2026-09-28 via `git grep -c
  'create_document_root()' -- <path> | wc -l` and `git grep -o … | wc -l`) and the silent-green risk of an eval
  refusal; G11 the one write API; G12 the OOP `Navigate` rebuild (`content/iframe/thread.rs:227`) needs the
  **frame's** flags, not the retired document's; the VM wrapper's callers (`elidex-js/src/engine.rs:215`,
  `vm/host/dispatch_target.rs:211`, `vm/natives_promise.rs:540`); verify the compile / modals / `window.open` sites
  are bound; rev 4 mis-cited `elidex-html-parser-strict/src/result.rs:137` (a fragment-options test); comments at
  `vm/host/storage.rs:79-80` and `event_handler_attrs.rs:586`; `content/form_input.rs` overlaps the live L3 branch;
  `vm/host_data/mod.rs` (2023 lines): touch-time rule applied — the touched concern — the `sandbox_flags` field (`:187-200`), its initializer (`:863`) and accessors (`:1095-1139`; the trait surface is in the two `engine.rs` files) — is deleted whole, with the `document_origin_override` doc that names it (`:218-226`) swept, so no seam along the touch remains to split first, and the file's reduction below 1000 is the design refactor the user placed in `#11-host-data-full-decomposition` (option A, 2026-07-11, when #455 split its touch seam); any other >1000-line file this PR's plan finds it edits (e.g. `elidex-dom-api/src/element/tree.rs`, 1076, if an F11 caller edit reaches it) gets the same check before implementation; **1000-line watch**:
  `elidex-shell/src/pipeline.rs` (973), `elidex-js/src/vm/natives_promise.rs` (965) and `elidex-script-session/src/mutation/mod.rs` (994, an F11 fragment-parse caller) must not cross 1000 at this
  PR's cut; the unbound invocation read (§7).
- **Touch-time cleanup in a file it edits** (not an F9 path — it creates no document root; it takes a built
  `EcsDom`): `build_paged_pipeline` (`elidex-shell/src/pipeline.rs:390-421`, `#[allow(dead_code)] // Exposed for
  future print/PDF integration.` at `:398`; `git grep -n build_paged_pipeline -- crates` → its definition only) is
  **deleted**. Decided on the lenses: CLAUDE.md's "dead code は接続するか削除"; connecting it needs the print/PDF
  feature, which no plan schedules; a future print path is written against the pipeline as it then is, so keeping a
  wrapper for it is the speculative abstraction the design discipline declines; and deleting 32 lines relieves the
  973-line watch item. No slot (a slot would book the print feature itself). The function it wraps,
  `elidex_render::build_paged_display_lists_interleaved` (`elidex-render/src/builder/mod.rs:296`, re-exported at
  `elidex-render/src/lib.rs:13`, exercised by `builder/tests/paged.rs:75`), stays — a tested public library API of
  the paged render path, not dead code.

**IBP-classify** (appendix B):
- F4 per §15.4 / §15.4.2 rules in order / §4.8.3 / css-ui-4 §7.2; F7 per each §15.5.x subsection (range and
  checkbox/radio non-devolvable; button-layout elements per §0.6 item 3); §15.4's replaced list vs css-ui-4
  recorded for `select`/`textarea`/`meter`/`progress`/`button`; F7's one mapping in `elidex-plugin` absorbs every site
  F7's property covers (Appendix B) and resolves their disagreements.
- F4 answers the `input` **Hidden** state explicitly: it is no §15.5 widget subsection's element (HTML gives it
  `display: none !important`, §15.3.1), and `FormControlState`-presence replacedness goes away at IBP-layout; the
  answer is `IBP-layout`'s input (E4) for the rule's placement (§4 IBP-layout).
- F6's marker with its whole writer set: the loader writer, its clone-policy row, and the `src`-mutation
  transitions — *update the image data* step 13 (unparsable new source → broken: F6 set) and step 11 (null source:
  F6 cleared, the null source being identity, §1 F6) before step 18 (non-null source: a broken current request is
  replaced — F6 cleared; an available one stays current); the attribute reconcile seam's
  contract widens from "component = f(Attributes)" to "reset on mutation" — recorded in the seam's doc. F4 reads
  a null source, then F6, before F5 (§1 F6), so its replacedness answer holds for every state reachable at this
  PR's landing, a stale `ImageData` included. Input lines: G5 null/unparsable `src` (step 13's parse is this PR's); §4.8.4.3.2's list of relevant mutations — the trigger population of *update the image data*, so F6's writer set is fixed here (IBP-layout's F5 removal reuses it). F16 as elidex's stated policy.
- The probe runs **before** the rows it decides — `button`, `input` in the Submit/Reset/Button and Color states
  (the HTML button-layout vs css-ui-4 §7.2 conflict, D1), devolution and outermost `svg`. Its observable decides
  replacedness in **both** directions: css-pseudo-4 §4.1 makes a replaced originating element suppress
  `::before`/`::after` ("suppressed when their parent, the originating element, is replaced", `webref body
  css-pseudo-4 generated-content`) and says nothing of the converse, so a suppressed pseudo alone (Appendix B's zero
  width delta) is no proof of replaced — it counts only with a second observable or a control that rules out
  element-specific suppression. For the four widgets §15.4
  does not list (`select`, `textarea`, `meter`, `progress`) it corroborates css-ui-4 §7.2's "replaced"; a
  contradicting result there is a finding recorded in IBP-classify's memo, not a silent row change; F8 is built in every case — the rendering condition needs it — and F4 reads it only if devolution changes replacedness (F8 with css-ui-4's revert/revert-layer clause and
  author-origin-only cascade).
- Own pins: an `<img>` without `ImageData` (pending) and an undrawn `<canvas>` classify replaced; `<img alt=x>`
  without `src` classifies non-replaced (6k's classification); an `<img alt=x>` holding `ImageData` whose `src` is
  then set to an unparsable URL classifies non-replaced (step 13: F6 set, the stale `ImageData` not read as
  available — the pin that fails if F5 is read before F6); `uses_button_layout` is true for `button` and `input`
  Submit/Reset/Button/Color, false for a text `input`, `select` and `output`.
- Must not: write F5 (I1 — the step-11/13 removal is IBP-layout's).
- Input lines: G6 fixtures with `ImageData` and no marker are available by F5; G7 the loader's status policy (a
  non-2xx body reaches `decode_image`, `loader.rs:370-375`); G8 — an obligation since rev 37, F8 being always built: the cascade exposes the
  author-origin cascaded value after revert/revert-layer rollback, which it does not today (the winners map loses
  `Origin`, `elidex-style/src/cascade/mod.rs:127-135`); §15.4.2's fifth rule (an `input` not
  representing an image); rev 4's D1(b) quote misattribution; SVG2 gives no "replaced" wording (spec-silent →
  probe / elidex reading).

**IBP-layout** (appendix D): measure the presence-keyed population by I1's property (a command and what it cannot
see; Appendix D's layout list is a seed) and switch every member — layout, paint and any other crate — together,
each onto the predicate matching its purpose (replacedness → F4; widget rendering → F7-based conditions it
derives), such that each element gets exactly one defined rendering, before any other F4 consumer; its plan-memo **derives every consequence of its presence switch** — first-time layout
of the subtrees of formerly presence-replaced elements, paint of widget content and chrome, and the computed values
those subtrees now expose; seeds: the `button` element's children, the `input` button and Color states under each
answer the probe could give (against I1's spec-fixed content sources), the Hidden-state `input`'s script-appended
children, HTML §15.3.10's resets — with a measuring method and a statement of what it cannot see; **places each** by §0.6 item 7's rule and cell test, consuming the `[hidden]` rule `IBP-ua-display` builds (E25)
and `IBP-css-machinery`'s `i` flag (E19) for its `input[type=hidden i]` rule; builds the `embed[hidden]` rule in its F4 switch's cut, the repair of the cell that switch fires (§3 §15.3.1 row); lays out §15.5.3's button layout — a `button` whose computed `display` has outer type `inline` (not `inline-flex`/`inline-grid`) behaves as `inline-block`, and every button-layout element establishes a new formatting context — which no style or layout code does today (`is_atomic_inline`, `inline/collect.rs:14-19`, routes by `display` alone), so layout agrees with F1's button-layout term before F1 lands (E5); defines the devolved rendering its widget-rendering condition gives an F8-devolved widget — a CSS box and its control content without native chrome (Appendix D) — measuring the button-layout case, whose primitive appearance HTML leaves undefined, like any spec-silent outcome; pins its `content-visibility` cells
(§0.6 item 11, the `button`-children cell); with a fixture per content-source branch, `value=""` included; and
**registers** every A96 remainder as a new facet of `#11-form-control-ua-rendering-fidelity` (§6; one slot for the
class, not a parallel one). That slot's SoT trigger, "a form-control rendering-fidelity pass", **fires at
`IBP-layout`** (it repairs form-control UA rendering on the same `input`/`button` selectors, `ua.rs:128-152`): its
plan re-audits facets (2) and (3) there by the landing 4-question audit — fold or keep each. Rev 14–17's form-control analysis (today's code, `<output>` included) is Appendix D
input. F14 per
§2's inputs, measured where the spec is silent; the `src`-mutation F5 removal — *update the image data* steps
11/13 (null or unparsable new source → image data discarded), on the reconcile seam IBP-classify widened; fires `#11-replaced-inline-no-atomic-layout`. Input lines: G5 and the relevant-mutation triggers (IBP-classify's) reused for the removal; reset detail
(`tree_clone.rs` table row); multicol test placement.

**IBP-predicate** (appendix C): F1–F3; the four `client*` members return 0 for an inline box while
`scrollWidth`/`scrollHeight` (same `get_padding_box`) stay unguarded; `client_top_returns_border_width` gets a
production-shaped fixture; IFC routing unchanged; owns the classification side of cells 6c/6f/6k; F1 answers false for a button-layout element whatever its computed `display` (cells: `<button style="display:inline">`, an `input type=submit` with `display:inline`). Input line:
`layout_query.rs` growth (740 lines).

**IBP-transform** (appendix E): req 8 as ratified and widened by §0.6 item 14 (F12's `table-column` / `table-column-group` arm, with that item's seed cell placed by item 7's test; both partitions; the three stacking terms together; the
fixed-descendant scan); any `content-visibility` cell it fires is pinned (§0.6 item 7, item 11); fires `#11-transformed-block-abspos-double-layout` and
`#11-transform-family-3d-and-containing-block`. Input lines: G10 the double-layout test's expected value and what
happens if it reproduces; `elidex-plugin/src/computed_style/tests.rs` (947 lines) placement.

**IBP-observer** (appendix F): §3.3.1 empty content rect for an inline box; isActive compares each observation's
**observed** box; the observed box's writer transition — `observe()` on an already-observed target replaces its
observation (Resize Observer `observe()` steps 1–4: `unobserve()`, then a new `ResizeObservation` with the new box
and fresh last-reported sizes), where `ResizeObserverRegistry::observe` today keeps the old one
(`elidex-api-observers/src/resize.rs:133-164`), with a re-observe cell (`content-box` then `border-box`); host closure
only marshals (C-3d owns its migration). Input line: G9 the device-pixel arm has no device-pixel input
(`SizeProvider`, `resize.rs:19`) — that branch is `#11-resize-observer-device-pixel-box`'s (§3): IBP-observer's observed-box match compares the CSS-pixel content-box size for it, as every box does today (`gather_observations`, `resize.rs:235-296`, reads no observed box), and records that fallback in the slot (§6).

**IBP-css-machinery** (appendix G's machinery paragraph): derive the population by its property — CSS machinery a
program repair consumes that elidex lacks — with a measuring method (every selector and media query in the
repairs its consumers' plans author, parsed and matched against elidex's `elidex-css`) and what it cannot see
(repairs whose machinery §0.6 item 7 routes elsewhere); land after IBP-sandbox (E17) and before IBP-ua-display
(E18), IBP-layout (E19) and the `content-visibility` program (E22); pins its `content-visibility` cell (the author
`i` rule, §0.6 item 11). Its consumers' placements come from their plans' cell tests (§0.6 item 7), so the `i` and `scripting`
consumers named here are candidates: an edge whose consumer's plan authors no rule needing it is dropped there (E22 is not: item 11 fixes the
`[hidden=until-found i]` rule), and
if the population comes out empty the sub-PR is not opened.

**IBP-ua-display** (appendix G): derive, by §0.6 item 7's cell test, the exposures of PR-1a, PR-1b, PR-1c and PR-1d,
with a measuring command and a statement of what it cannot see; repair the A93 ones — the UA rules in the core and compat
sheets (compat-sheet placement for shadow trees decided with the `tt` precedent, `ua.rs:68-73`), consuming
`IBP-css-machinery`'s machinery (E18), and the non-exposing `audio`'s computed-value force
(from F16); build the `[hidden]:not([hidden=until-found i]):not(embed)` rule `IBP-layout` also exposes (one
builder per repair) and land before `IBP-layout` (E25) and before the first program PR that fires one of its cells
(E12, PR-1a as derived in §0.6 item 7); it fires no `content-visibility` cell (its rules only hide), and
umbrella exposures whose repair is that program are not its population (§0.6 item 7); place by §0.6 item 7's
cell test — A96 remainder to `#11-ua-sheet-html-rendering-display-fidelity`, any A102 divergence recorded
cell-pinned. Rev 10's dispositions (Appendix G) are inputs it re-derives.

### §4.1 Touch sets against pending umbrella prereqs and live lanes

Each prereq's touch set is its own plan's; these are the files this program edits that their §8 blocks cite
(`awk 'NR>=5524 && NR<5900' <umbrella> | grep -oE '(inline/collect\.rs|intrinsic/mod\.rs)[:0-9-]*'` →
`collect.rs:218-220`, `collect.rs:260-272`, `intrinsic/mod.rs:134`, `154bac3f` coordinates):

| Pending / live | Shared file | Their cited hunk (`e2d62b9e`) | This program's hunk | Verdict |
|---|---|---|---|---|
| reconciler / item-boundary / pseudo prereqs | `inline/collect.rs` | `:223-225`, `:265-277` | `:11-19`, `:248` (IBP-predicate) | disjoint |
| min-content prereq | `elidex-layout/src/intrinsic/mod.rs` | `:134` | `:98` (IBP-layout) | disjoint |
| end-of-line white-space prereq | `pack/mod.rs`, `inline/measure.rs`, `whitespace.rs` | — | not touched | disjoint |
| pseudo prereq | `elidex-style/src/pseudo.rs`, `walk.rs` | generation | `walk.rs:510-513` only if IBP-ua-display's placement decision adds the compat sheet there | IBP-ua-display's plan checks |
| L3 lane `domform-submittable-category` | `elidex-ecs/src/components.rs`; `elidex-form-core/src/lib.rs`; `elidex-form-core/src/input.rs`; `elidex-shell/src/content/form_input.rs` | `:598-610`; `:173-180`, `:420-432`; `:712-719`; its hunk | a declaration beside `IsModalDialog` (`:377`); `:232-279`, `input.rs:814-837` (IBP-classify, F7); IBP-sandbox's gate re-point | disjoint, disjoint, disjoint, IBP-sandbox's to check |
| F7's other sites (touch-set evidence, not the population — the population is Appendix B's, re-measured at IBP-classify's cut; no live branch touches these today) | `elidex-css/src/selector/matching.rs`; `elidex-form/src/label.rs`; `elidex-form/src/reconciler.rs`; `elidex-js/src/vm/host/html_input_proto.rs` | — | `:401`; `:47`; `:130`; `:783-820` (IBP-classify) | IBP-classify's plan re-checks at its cut |
| IBP-css-machinery × IBP-classify (no edge between them) | `elidex-css/src/selector/matching.rs` | — | `:322-341` (IBP-css-machinery, `match_attr`) vs `:401` (IBP-classify, F7) | disjoint; whichever lands second rebases |
| focus lane (`#11-canonical-focus-update-steps`; next slice A2b not started — no local or remote branch touches the file) | `elidex-dom-api/src/focus/predicate.rs` | — | `:87-89`, `:280-287` (IBP-classify, F7) | disjoint today; IBP-classify's plan re-checks at its cut |

## §5. Out of scope, with disposition

- Replaced-inline layout — `#11-replaced-inline-no-atomic-layout` (fired by IBP-predicate and IBP-layout; stays
  open: no program PR makes a replaced inline newly reachable, cell 6c).
- Alt-text rendering — `#11-img-alt-text-rendering` (NEW; §0.6 item 7 shows neither A93 half fires).
- Runtime image loading, string-built documents, `srcset`/`picture`, and HTML's progressive request states — one
  image-request pipeline (design §18.4–§18.5): `#11-img-dynamic-image-request` (NEW).
- `<input type=image>`'s loading, rendering and submit coordinates — `#11-input-image-state` (existing).
- `object` — `#11-object-element-replacedness` (NEW). `appearance`, incl. the `select` base-appearance rules —
  `#11-appearance-replacedness` (NEW).
- `content-visibility` — its own program (not an umbrella prereq: not ordered ahead, §0.6 item 12),
  `#11-content-visibility-skip-contents` (§0.6 item 11);
  container-side containments `#11-css-containment`, `auto` `#11-content-visibility-auto`.
- UA-rule defects no program PR's cell fires (A96, §0.6 item 7) — `#11-ua-sheet-html-rendering-display-fidelity`,
  registered by IBP-ua-display for those its plan finds; `ruby`/`rt`, `br`/`wbr` — the umbrella's dispositions.
- `clientTop`/`clientLeft` step 2 reads the computed border width; elidex returns the used one, which differs for a
  collapsed-border table cell (`elidex-layout-table/src/algo.rs:321` `resolve_collapsed_borders`) —
  `#11-client-border-computed-value` (NEW).
- The embedder→embeddee flag union (§7.1.5) — B1 req 2's.
- MathML (no `display: math`), `offsetTop`/`offsetLeft` (umbrella's `#11-inline-box-decoration-splits`),
  `is_block_level` (§0.6 item 2) — no slot.

## §6. Slots and ledger (program level)

**Landing vehicle of this memo**: a docs-only approval PR carrying **this file and the umbrella's ledger rows for
it** (the A114 idiom; the #515 precedent) — items 5–6, item 7's rule, item 11, item 12 and item 13, the table's
approval-PR rows, drafted in this worktree as the umbrella's ledger rows **A152–A156** (appended after A151; they cite the umbrella by
section and ledger id, not by line). This memo's umbrella line numbers are at `e2d62b9e`; the five inserted rows move
every later umbrella line by +5. Its landing makes §0.6 item 5 (the ordering, with its two relaxations), item 6's existence, item 7's rule and
item 12's reading true. Memory lives outside the repo, so the memory re-points below happen **at the approval
PR's landing**, not in it. Every other record ships with the sub-PR whose landing makes it true.

| Action | Ships with |
|---|---|
| Umbrella record (A114): §0.6 items 5, 6 (ordering incl. relaxations 5(a)/5(b); new prereq exists), re-pointing the umbrella's in-repo trigger sites that name "the predicate prereq" (`:2327-2328`, `:6582`, `:6929`, `:6933`) | the approval PR (umbrella ledger row) |
| Umbrella record: §0.6 item 7's rule (the cell test, the ownership rule, one builder per repair, the pinned-divergence clause) | the approval PR (umbrella ledger row) |
| Umbrella record: §0.6 item 11 — the `content-visibility` skip-contents program, not ordered ahead of any PR (item 12), with the pinned cells PR-1b, PR-1c and PR-1d carry (§0.6 item 11) | the approval PR (umbrella ledger row) |
| Umbrella record: §0.6 item 13 — PR-1b 0 → 1, PR-1c 1 → 2, PR-1d 1 → 2 own deferrals, conditional on each pin firing at that PR's actual base | the approval PR (umbrella ledger row) |
| Umbrella ledger amendment: §0.6 item 12 — A102's separator read as the size of the repair (a whole program with its own umbrella and slot), A102 standing | the approval PR (umbrella ledger row) |
| Umbrella record: §0.6 items 1, 3 (req 5, req 3) | IBP-classify |
| Umbrella record: §0.6 item 2 (req 2) | IBP-predicate |
| Umbrella record: §0.6 item 14 (req 8) | IBP-transform |
| Umbrella record: `IBP-ua-display`'s population under §0.6 item 7, discharge of item 6 | IBP-ua-display |
| Record each A102 accepted, cell-pinned divergence the exposing PR owns while unlanded (§0.6 item 7's ownership rule): an umbrella ledger row — never `IBP-ua-display` for another PR's exposure; a cell found after its exposer has landed is recorded by its finder against the exposer's own-deferral count, with no second slot | each exposing PR, at its landing; the finder, for a post-landing finding |
| B1 plan-memo and SoT `#11-browsing-context-state-ecs-components`: §0.6 item 4 | IBP-sandbox |
| S5-4 plan record: §0.6 items 8, 9 | IBP-sandbox |
| Re-point every memory surface naming the singular "predicate prereq" as a trigger or ordering (`grep -rn -i 'predicate prereq' ~/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory/` → 4 files; the status/history lines in `project_line-box-decorated-inline-content.md` are the lane's own log, not triggers): `project_open-defer-slots.md:339` `#11-replaced-inline-no-atomic-layout` → IBP-predicate / IBP-layout; `:345` `#11-pseudo-generation-on-replaced-originating-element` → IBP-layout (E6, I1: F4's first consumer lands after the presence switch); `:350` `#11-transform-family-3d-and-containing-block` and `:351` `#11-transformed-block-abspos-double-layout` → IBP-transform; `project_inline-fragmented-fn-seams-1-2.md:170` ("the seventh, the predicate prereq, is NOT exempted") → every IBP sub-PR is non-exempt; its third disjunct (`:174-176`, the next change touching `elidex-layout-multicol/src/fill.rs`) **fires at IBP-layout**, which edits `find_tallest_monolithic` (`fill.rs:407-418`, D7) — recorded by IBP-layout with the landing 4-question audit (row below); `project_browsing-context-state-ecs-components.md:11` names the old "P1c" → IBP-sandbox | the approval PR's landing (memory) |
| `#11-pseudo-generation-on-replaced-originating-element`: trigger fires; input F4 (first consumed after the presence switch, E6) | IBP-layout |
| `#11-inline-fragmented-fn-seams-1-2`: its third disjunct fires (IBP-layout edits `fill.rs:407-418`); the landing 4-question audit decides fold or keep | IBP-layout |
| `#11-replaced-inline-no-atomic-layout`: fires at both; stays open with a **new trigger** — the next change to the IFC's atomic dispatch (`InlineItem::Atomic` routing) after IBP-predicate — and *Re-eval* 2026-11-01 | IBP-predicate, IBP-layout |
| `#11-transformed-block-abspos-double-layout`, `#11-transform-family-3d-and-containing-block`: triggers fire; recorded by IBP-transform's memo | IBP-transform |
| Enrich `#11-resize-observer-device-pixel-box` (M4-12 roadmap §H-8 backlog): IBP-observer's `device-pixel-content-box` arm falls back to the CSS-pixel content-box size until the slot lands; list it in the open-slot registry with that fallback | IBP-observer |
| Enrich `#11-form-control-ua-rendering-fidelity`: facet (1) (`input[type=hidden]`) is **discharged** by `IBP-layout` if its plan places the Hidden-state `display: none !important` rule A93 under §0.6 item 7's cell test (otherwise it stays A96 in the facet); the `i` attribute flag its facet (3) names is recorded by `IBP-css-machinery` (row below). *Why deferred* (facets (2), (3)): UA-declaration fidelity — which UA rule a button-type or mixed-case `type` input matches — author-visible today (A96); a UA rule `IBP-layout`'s plan places is recorded against the facet it discharges. Trigger and re-eval stay the SoT's (quoted in the next row); the trigger **fires at `IBP-layout`**, whose plan re-audits facets (2)/(3) by the landing 4-question audit (fold or keep) | IBP-layout |
| Enrich `#11-form-control-ua-rendering-fidelity` with a new facet: the consequences of the presence switch that `IBP-layout`'s plan places A96 (author-visible today and left exactly as found), each named with its spec section. The slot's SoT (`project_open-defer-slots.md`) keeps its own "**Trigger**: a form-control rendering-fidelity pass, or `BrowserCore` becoming selectable [these now affect core]. **Re-eval date**: with the form-control / `BrowserCore` work." *Why deferred* (the facet): author-visible today independent of the program (A96). *Trigger* (the facet's own, which `IBP-layout`'s derivation cannot fire at creation): an HTML forms-rendering WPT subset (`html/rendering/widgets`) declared supported. *Re-eval*: 2026-11-01 — shipped with `IBP-layout` | IBP-layout |
| Record in `#11-form-control-ua-rendering-fidelity` facet (3): the `i` attribute flag landed (facet (3)'s UA rules stay the slot's) | IBP-css-machinery |
| Enrich `#11-media-extended-features`: `IBP-css-machinery` lands the `scripting` feature (from F10); record it. *Why deferred* (the rest of the set): the remaining MQ5 features (`hover`, `pointer`, `update`, `overflow-*`, colour depth) need producers of device facts elidex's media environment does not carry (`elidex-css/src/media/types.rs:229-231`), none of which this program supplies. *Trigger*: a mediaqueries-5 WPT subset other than `scripting` declared supported, or a device-fact producer added to the media environment. *Re-eval*: 2026-11-01 | IBP-css-machinery |
| Open `#11-content-visibility-skip-contents` (NEW, pre-existing class: elidex has no `content-visibility`) as the home of the `content-visibility` program (§0.6 item 11): applies-to (consuming F1), the `[hidden=until-found i]` rule, the skipped-contents flag and its readers, `details`' UA shadow tree, fragment-navigation reveal; own umbrella memo and per-PR plan-review; **owner: this Layout lane** (default; the user may assign another). *Why a program slot*: the repair is program-scale (§0.6 item 12), so it is **not A93-ordered** and its exposures are pinned as A102-disposed accepted divergences, each its exposing PR's own deferral (§0.6 item 13). *Trigger*: the approval PR lands — authoring of its umbrella memo starts, and that memo's approval gates `IBP-sandbox` (E26); its implementation PRs follow `IBP-predicate` (E20) and `IBP-css-machinery` (E22). *Re-eval*: 2026-11-01 | the approval PR's landing (memory) |
| Register `#11-css-containment` (NEW, pre-existing class). *Gap*: the container-side containments (size, layout, style, paint) that `content-visibility: hidden` implies. *Why deferred*: A96 — `<div hidden="until-found">tall</div>` is tall today and the program changes no cell of it. *Trigger*: `contain` added to elidex's CSS property registry, or a css-contain-2 WPT subset declared supported by another lane. *Re-eval*: 2026-11-01 | the approval PR's landing (memory) |
| Register `#11-content-visibility-auto` (NEW). *Gap*: `content-visibility: auto`. *Why deferred*: `auto` needs relevance-to-the-user tracking (viewport proximity, focus, selection, top layer) beyond the `hidden` mechanism, and `content-visibility: auto` computes as visible today for on-screen content, the only case the program's cells reach. *Trigger*: a css-contain-2 `content-visibility: auto` WPT subset declared supported, or an off-screen-rendering-skip project — events outside the `content-visibility` program's declared subset. *Re-eval*: 2026-11-01 | the approval PR's landing (memory) |
| `#11-ua-sheet-html-rendering-display-fidelity` (pre-existing class): registered **by IBP-ua-display** for the A96 remainder its plan derives by §0.6 item 7's cell test, with that plan's Why / Trigger / Re-eval; rev 10's instance-level analysis is Appendix G (input only) | IBP-ua-display (if needed) |
| Enrich `#11-cross-document-adopt-on-insert`: its composed-tree-root proxy is promoted into F10 | IBP-sandbox |
| Enrich `#11-domparser-full-document-parse-fidelity`: its inert marker is F10's final representation of "browsing context is null"; its date (2026-08-31) has passed — refreshed to 2026-11-01 | IBP-sandbox |
| Register `#11-appearance-replacedness` (NEW). *Gap*: classification assumes `appearance: auto` (and the `select` base-appearance UA rules are unapplied). *Why deferred*: without an `appearance` property there is no author value to key on; adding the property is style-system work whose own design (the value grammar, the base appearance) no program PR needs. *Trigger*: `appearance` added to elidex's CSS property registry (the property's handler in `elidex-css`/`elidex-plugin`) — an event of that work, not of this slot — or a css-ui-4 `appearance` WPT subset declared supported. *Re-eval*: 2026-11-01 | IBP-classify |
| Register `#11-object-element-replacedness` (NEW). *Gap*: D8. *Why deferred*: no producer of `object` content exists, so the classification has no state to read. *Trigger*: any `object`/`embed` content loading. *Re-eval*: 2026-11-01 | IBP-classify |
| Register `#11-img-dynamic-image-request` (NEW). *Gap*: §5's image-pipeline bullet. *Why deferred*: runtime image loading is image-pipeline work (design §18.4–§18.5) that no program PR consumes, and a pending `<img>` classifies correctly (replaced, rule (2)) meanwhile. *Trigger*: elidex declaring a WPT subset that exercises a dynamically set `<img src>` supported — an event independent of this slot's own implementation, which cannot fire at creation. *Re-eval*: 2026-11-01 | IBP-classify |
| Register `#11-img-alt-text-rendering` (NEW). *Gap*: a rule-(3) `<img>` renders neither its alt text nor a broken-image icon. *Why deferred*: it is `<img>` fallback **rendering** (design §18.10), orthogonal to classification, and §0.6 item 7 shows neither A93 half fires. *Trigger*: any change to `<img>` painting in `elidex-render` (the `ImageData` paint arm, `builder/walk.rs:376`). *Re-eval*: 2026-11-01 | IBP-classify |
| Register `#11-client-border-computed-value` (NEW). *Gap*: §5. *Why deferred*: step 2's computed-value reading is a CSSOM-fidelity change to the four handlers' second step, unrelated to whether the box is inline, and it diverges only in collapsed-border tables. *Trigger*: a change to collapsed-border resolution (`resolve_collapsed_borders`, `elidex-layout-table/src/algo.rs:321`), or a CSSOM-View `client-props-*` WPT subset declared supported. *Re-eval*: 2026-11-01 | IBP-predicate |

Every new slot is pre-existing class — `#11-content-visibility-skip-contents` included, the home of a program
repairing a pre-existing class (§0.6 items 11, 12), not A93-ordered. **Own deferrals** (§0.6 item 13; cap policy
`feedback_defer_cap_policy.md:12`/`:16`): each `content-visibility` pin counts for its exposing PR, conditional on
the cell firing at that PR's actual base — `IBP-sandbox` 1, `IBP-css-machinery` 1, `IBP-layout` 1 (its A96 facets
and the seams firing are pre-existing, not own), and the umbrella's PR-1b 0 → 1, PR-1c 1 → 2, PR-1d 1 → 2; each ≤ 3.
No sub-PR opens any other own deferral. Withdrawn before opening:
`#11-resize-observer-activation-ignores-observed-box` (IBP-observer implements it),
`#11-parser-scripting-flag-from-sandbox` (IBP-sandbox), `#11-html-rendering-forced-display-none`
(`IBP-ua-display`, by A93 — an instance-level placement; if `IBP-ua-display`'s plan places that defect A96 under
§0.6 item 7's cell test, it registers it after all).

## §7. Open item against d5

`elidex-js/src/vm/host/event_handler_attrs.rs:602-609` evaluates the settings-level verdict when `HostData` is
installed but no DOM is bound ("VmObject-only dispatch — e.g. AbortSignal / FileReader handlers on a DOM-less VM");
under (iv) F9 lives in the `EcsDom`. The only exerciser found is a test (`vm/tests/tests_abort_statics.rs:14`).
IBP-sandbox's memo establishes production reachability; if none, the branch is deleted and fails closed; if some,
the dispatch binds the owning document's `EcsDom` first; only if that is impossible does d5 reopen for this site (a
per-VM copy would be the second store d5 excludes).

---

# Appendices — input to the sub-PR plan-memos, **not reviewed here**

Moved from rev 4 under R2-S (the A145 idiom). Each item is evidence and a starting point for its sub-PR's own
memo, which re-derives it; nothing here is a decision of this program memo. Items rev 4 had that round 2 showed
wrong are **dropped**, listed at the head of their appendix.

## Appendix A — input to IBP-sandbox's plan-memo

Dropped: the write point at `run_scripts_and_finalize` (`pipeline.rs:187`) — superseded by stamp-at-creation
(§0.5); the reading "absent = fully sandboxed" as the representation of a browsing-context-less document —
superseded by F9/F10's separation; stamping `build_paged_pipeline` — rev 6 deletes it (§4, dead code); `createHTMLDocument` as a writer — it does not exist at `e2d62b9e` (`git grep -n -i createHTMLDocument -- crates` → none).

Gate sites found at `e2d62b9e` (`git grep -n 'sandbox_flags\|forms_allowed()\|popups_allowed()\|modals_allowed()\|scripts_allowed()' -- crates`,
production), today's read → rev 4's proposed read:
- eval gate `elidex-js/src/engine.rs:174` (`scripts_allowed()` → HostData) → `ScriptContext.dom`
  (`elidex-script-session/src/engine.rs:32-34`).
- compile gate `vm/host/event_handler_attrs.rs:553`; invocation gate `:602` (bound arm) → bound DOM.
- invocation gate `:602-609` (unbound arm) → §7.
- `window_dialogs.rs:52` (`modals_allowed`), `:214` (`window.open`) → bound DOM.
- shell: `content/form_input.rs:129` (`forms_allowed`), `content/link_nav.rs:46` (`popups_allowed`),
  `link_nav.rs:57` (top navigation), `content/iframe/thread.rs:227` (OOP rebuild; see G12) → `pipeline.dom`.
- parse entries: `elidex-html-parser/src/lib.rs:48-49` (`parse_html`, html5ever `ParseOpts::default()`),
  `parse_strict` (`:66`), `parse_progressive_str` (`:195`), `parse_progressive` (`:208`, bytes, via
  `elidex-navigation`'s `load_document`); fragment entries `vm/host/dom_inner_html.rs:421`/`:476`;
  `inert_document.rs:143` already passes `scripting_disabled: true`.

Deletion list, with the grep that proves it (15 lines today, all to be deleted):

```
git grep -n 'fn set_sandbox_flags\|fn sandbox_flags\|fn forms_allowed\|fn popups_allowed\|fn modals_allowed\|fn scripts_allowed' -- crates/script crates/shell
#   elidex-js/src/engine.rs:137/:655/:661/:669/:677
#   elidex-js/src/vm/host_data/mod.rs:1095/:1106/:1115/:1122/:1130/:1139
#   elidex-script-session/src/engine.rs:515/:519/:526/:533
```

Document-creating paths: parser document roots (`elidex-html-parser/src/convert.rs:18`;
`elidex-html-parser-strict/src/tree_builder/mod.rs:102`, `result.rs:150`); `create_document_node`
(`elidex-ecs/src/dom/mod.rs:466-471`; DOMParser `elidex-form/src/inert_document.rs:119`; the fragment scratch
document `dom/tree/teardown.rs:326`); the WPT harness (`crates/tools/elidex-wpt/src/harness.rs:37-45`, parses
and styles, no layout); `build_paged_pipeline` (`pipeline.rs:399`). The node→document rule to promote:
`elidex-script-session/src/scripting.rs:96-111` (composed tree root if a Document, else `owner_document`;
unresolvable → fails open there — the promoted composition must state its own default).

Rev 4 cell ideas: a top-level document stamped before its first cascade (canvas replaced in pass 1); a
sandboxed iframe (no `allow-scripts`) — no script, canvas ordinary, `<noscript>` disabled-mode; a DOMParser
document — canvas ordinary; an unstamped `EcsDom::new()` document — gates refuse; a canvas in a declarative
shadow root resolves its document through the composed tree root; every gate row refuses in a sandboxed frame
as today.

F11's caller list (moved from the body at rev 11) — the derivation applies at **every production caller of a fragment parse**, measured by `git grep -n 'parse_fragment_progressive(\|parse_html_fragment(' -- crates` less tests and the parser's own fallback (`elidex-html-parser/src/lib.rs:253`): in `elidex-script-session/src/mutation/html_fragment.rs`, `apply_set_inner_html` (`:66`, parse at `:87`), `apply_set_outer_html` (`:137`, parse at `:168`) and `apply_insert_adjacent_html` (`:187`, parses at `:203`/`:212`/`:229`/`:239`; insertAdjacentHTML step 5 invokes the fragment parsing algorithm; production via `elidex-dom-api/src/element/tree.rs:616-663` → `mutation/mod.rs:226`). **What the command cannot see**: a fragment parse reached through another name (a wrapper that forwards to these entries is found through them; a new parser entry would not be).

## Appendix B — input to IBP-classify's plan-memo

Moved (rev 7): the `src`-mutation algorithm (steps 11/13/18: F5 removal, F6 transitions; cell ideas R7, R9) is now IBP-layout's input (§4, I1). Moved back in part (rev 33): F6's transitions, its clone row (and R7, R9 for F6) are IBP-classify's again — I1 holds back only F5, and F6 must land with its whole writer set; F5 removal stays IBP-layout's. Dropped: rev 4's `ImageRequestState` with a `CompletelyAvailable` variant (F5 is `ImageData`, R2-AVAIL); rev 4's
reset rule that kept an available image on **any** `src` mutation (steps 11/13 precede step 18).

Rev 4's row table (to re-derive per §15.5.x and the probe):
- `img`: null source ∧ non-empty `alt` → non-replaced (arm i); broken ∧ non-empty `alt` → non-replaced (arm ii);
  no `alt` → replaced (rule 2); `alt=""` with nothing to show → replaced, natural dimensions 0 (rule 4); pending
  or available → replaced.
- `input type=image` → replaced in every state. Other `input` types, `select`, `textarea`, `meter`, `progress`
  → replaced under `auto` (native), devolved → probe. Checkbox/radio, range → non-devolvable.
  `button` → probe.
- `canvas` → replaced iff scripting enabled (F10). `embed`, `iframe`, `video` → replaced. `audio` → replaced iff
  exposing a UI (`controls` or scripting disabled). `object` → ordinary. `output` and others → ordinary.
  Outermost SVG `svg` → replaced (elidex reading). Other SVG, MathML, `TagType`-less entities → not replaced.

Probe page (rev 4; to extend with the four conflicting widgets and outermost `svg`):

```html
<!doctype html>
<style>
  .t::before { content: ""; display: inline-block; width: 50px; height: 10px; }
  .n { appearance: none; }  .b { border: 2px solid; }  .g { background-color: #eee; }
</style>
<p><button class="t">b</button> <button>b</button> <button class="t n">b</button> <button class="n">b</button>
   <button class="t b">b</button> <button class="b">b</button></p>
<p><input class="t"> <input> <input class="t b"> <input class="b"> <input class="t g"> <input class="g">
   <select class="t"><option>o</select> <select><option>o</select>
   <textarea class="t"></textarea> <textarea></textarea> <textarea class="t b"></textarea> <textarea class="b"></textarea>
   <meter class="t"></meter> <meter></meter> <progress class="t"></progress> <progress></progress>
   <input type="checkbox" class="t b"> <input type="checkbox" class="b">
   <output class="t">o</output> <output>o</output></p>
```

Read per pair `offsetWidth(.t) − offsetWidth(twin)`: 50 ⇒ `::before` generated (not replaced for css-pseudo-4
§4.1), 0 ⇒ suppressed — one-way (rev 33): suppression does not by itself mean replaced (§4 IBP-classify's probe
bullet).

Loader facts: the image step is inline in `load_document` (`elidex-navigation/src/loader.rs:281-294`);
extraction `resource.rs:197-220` matches `img` in any namespace and skips an empty `src`;
`NetworkHandle::mock_with_responses` (`elidex-net/src/broker/handle.rs:400`) is behind `elidex-net`'s
`test-hooks` feature, and `elidex-navigation` has no `[dev-dependencies]`. The reset seam:
`EcsDom::reconcile_attribute_derived_components` (`elidex-ecs/src/dom/attribute_reconcile.rs:50`). Clone table:
`elidex-ecs/src/dom/tree_clone.rs:13-39`. Cascade origin: `elidex-style/src/cascade/mod.rs:99`/`:117`.

Rev 4 cell ideas: R1 pending `img` with `alt` → replaced; R3 broken + `alt` → non-replaced; R4 no `src`/no
`alt` → replaced (6c's fixture); R5 `alt=""` + broken → replaced; R6 no `alt` + broken → replaced; R7 `src`
mutation from broken → pending; R8 `alt` + no `src` → non-replaced (6k); R9 clone; R10 canvas with and without
scripting; R11 embedded elements and audio; R12 widgets; R13 `output`/`div`/`span`; R14 SVG-namespace `img`,
outermost `svg`; R15 pseudo entity; R16 loader outcomes with the mock; R17 devolution (F8, and F4's read of it if the probe keeps it).

F7's site list (moved from the body at rev 11), with its measuring command: measured over every attribute-read API — the `Attributes` methods `get`, `contains`, `iter` (`elidex-ecs/src/components.rs:15`, `:34`, `:39`) and the `EcsDom` methods `get_attribute`, `with_attribute`, `has_attribute` (`elidex-ecs/src/dom/attribute.rs:59`, `:83`, `:100`) — by listing every production `"type"` literal (`git grep -n '"type"' -- crates` less tests, then keeping the reads of an `input` element's attribute): seven mappings — `FormControlKind::from_type_str` via `from_tag_and_type_attr` (`elidex-form-core/src/lib.rs:232-279`; callers `lib.rs:845`, `:908`, `elidex-form/src/reconciler.rs:130`); `input_list_applies_to_type` (`elidex-form-core/src/input.rs:820-837`, whose doc `:814-819` says it exists because `from_type_str` collapses `image`); the `:checked`-candidate test (`elidex-css/src/selector/matching.rs:401`); the hidden-input test (`elidex-form/src/label.rs:47`); the focus predicate's `TagDefault::Input` arm (`elidex-dom-api/src/focus/predicate.rs:87-89`) and its `is_hidden_input` (`:280-287`, called at `:212`); and the VM's `KNOWN_INPUT_TYPES` (`elidex-js/src/vm/host/html_input_proto.rs:783-806`, read at `:820`). **What the command cannot see**: a read through a variable key (e.g. an interned `well_known.type_attr`, `elidex-js/src/vm/well_known.rs:1071`, whose uses at `vm/host/html_*_proto.rs` register the IDL property and read no attribute) or through `iter()` filtered by a computed name. **Outside the property, decided**: the strict parser's frameset-ok test (`elidex-html-parser-strict/src/tree_builder/modes/in_body.rs:180-188`) reads a start-tag **token**'s attributes, which HTML's tree-construction step specifies on the token itself, before any element state exists; the crate depends only on `elidex-ecs` and the step is not a kind mapping, so it stays a token test. They disagree today: `image` is listed by `KNOWN_INPUT_TYPES` and falls to `_ => Self::TextInput` in `from_type_str` (`lib.rs:277`); `from_type_str` maps `select-one` and `textarea` (`lib.rs:275-276`) to `Select`/`TextArea`, which `KNOWN_INPUT_TYPES` does not list

## Appendix C — input to IBP-predicate's plan-memo

Display facets: `Display::is_inline_flow` (NEW) = `Inline` (no other outer-inline flow value,
`computed_style/display.rs:7-29`); `Display::is_inline_level_independent_fc` (NEW) = `is_atomic_inline`'s body
moved (`collect.rs:14-19`, call `:248`). Composition sketch: no `ComputedStyle` → false; display not inline
flow → false; pseudo → the `content` arm; uses button layout (F7) → false (a `<button style="display:inline">` behaves as `inline-block`, §15.5.3; every button-layout element establishes a new formatting context); else ¬replaced ∧ ¬`content`-replaced. `ContentValue` has no image
item (`box_model.rs:35-56`); `parse_content` drops `url()` (`elidex-css/src/declaration/misc.rs:441`).
`client*` handlers `layout_query.rs:121`/`:128`/`:135`/`:145`; `get_padding_box` `:346`; `scroll*` `:159`/`:167`;
test `:497`; allowlist rows `pending-migration:C-3b` (`.claude/tools/layout-box-reader-allowlist.tsv:64`).
Cell ideas: inline span → all four 0; inline-block control; `img` (no `alt`) inline → border; `img alt=x` inline
→ 0; `input` inline (not devolved) → border; `output` inline → 0; `scroll*` unchanged; rewritten border test;
`none`/`contents`/blockified/style-less → false; pseudo inline → true; `ContentValue` totality; display facet
membership.

## Appendix D — input to IBP-layout's plan-memo

Known form-control rendering divergences (rev 17, corrected from rev 16's body; input for the scope rule of §2 I1 —
IBP-layout's plan decides which fall inside code it rewrites, the rest stay A96 slots):
(a) `input` Submit/Reset with `value=""`: `from_input_element` (`elidex-form-core/src/lib.rs:843-853`) substitutes
"Submit"/"Reset" whenever the raw value is empty, absent and present-but-empty alike, so `value=""` paints "Submit"
where §15.5.12 ("value attribute, if any") gives empty text; an absent `value` correctly gets the substituted label.
The substitution lives in form-core's `value` field, not in the `emit_form_control` arm. (b) `input` Button state
with no `value`: raw value empty, no substitution, and `emit_button` returns on the empty label
(`elidex-render/src/builder/form.rs:391-394`) — nothing painted; §15.5.12 makes the type-derived text
implementation-defined, so this is a quality gap rather than a clear conformance divergence. (c) `input` Color: the
`emit_button` arm (`form.rs:120-131`, `Color` at `:124`) paints the value's text, where §15.5.9 gives a child box
with a `background-color` hint. (d) The same arm paints Range, Date, DatetimeLocal, Time, Week, Month and File as
their value text, while §15.5.8 (range control), §15.5.7 (domain-specific widgets) and §15.5.11 (file upload
control) describe other renderings — these are outside the content-source obligation (not button-layout
elements) and stay A96 unless IBP-layout's rewrite of the arm covers them. Hint placement input: the cascade's hint
callback is either `get_presentational_hints` (`elidex-dom-compat/src/presentational.rs:47`) or an empty
`no_hints`, chosen by `presentational_compat()` (`elidex-shell/src/lib.rs:126-150`; `elidex-style`'s own
`resolve_styles` passes `no_hints`, `lib.rs:297-320`), so a §15.5.9 hint placed there is dropped in
`BrowserCore`/`App`, while it is a rendering-section hint, not a legacy presentational attribute.

Rev 17's body analysis of today's form-control code (moved at rev 18, with the paint facts corrected — input to
`IBP-layout`'s plan, which owns every consequence of its presence switch since rev 19 — not reviewed here):
(e) `button` subtrees. Layout: `FormControlState` presence makes a `button` replaced (`get_intrinsic_size`,
`elidex-layout-block/src/helpers.rs:405-418`, read at `block/mod.rs:224`), so `block/mod.rs:380` takes the replaced
branch and the children branch (`:401`) is not reached — the children are not laid out. Paint: they **are** painted
(rev 17 said not — wrong): the walk visits entities without a `LayoutBox` (`elidex-render/src/builder/walk.rs:104-111`),
`dispatch_children` runs after `emit_form_control` (`walk.rs:409-447`), and the legacy inline path paints a run's text
against the nearest ancestor `LayoutBox` (`builder/inline.rs:146-171`), reading `letter_spacing`/`word_spacing`
(`:62-63`). So at `IBP-layout` only layout is first-time; paint-visible differences are A96 candidates.
(f) HTML §15.3.10 per property: `input, button, textarea` reset `letter-spacing`/`word-spacing`/`line-height` to
`initial`, and `input, select, button, textarea` reset `text-transform`/`text-indent`/`text-shadow` (all six
inherited, `webref css`); elidex's sheets set none of the six (`/usr/bin/grep -c -F <property>` on
`elidex-style/src/ua.rs` and `elidex-dom-compat/src/legacy_ua.rs` → 0 each); `text-align: center` is present
(`ua.rs:152`). Because (e)'s paint reads `letter_spacing`/`word_spacing`, those two gaps are visible on a `button`'s
children today (A96 candidates); the others' visibility is the plans' to derive.
(g) Further §15.3.10 candidates: `align-content: center` on `button` and the Color/Reset/Button/Submit `input`
states (0 hits in elidex's sheets); `box-sizing: border-box` on `button` and several `input` states (elidex sets
`box-sizing` only on `table`, `ua.rs:23`).
(h) `<output>`: its `emit_form_control` arm is a no-op (`elidex-render/src/builder/form.rs:136-140`), so its
widget-paint answer does not change when F4 answers it ordinary; its children follow (e)'s paint path.
(i) Hidden-state `input`: laid out through the same replaced path as (e) (`FormControlState` of kind `Hidden`); how
a script-appended child is painted today follows (e)'s walk and is not measured here.

Rev 14's widget-rendering analysis (moved from §2 I1 at rev 15; its `button` premise rested on a quotation not in HTML §15.5.3, so the button-layout rows are probe-decided again and every claim below that depends on them is to be re-derived): The **presence-keyed population** is defined by the property: every production site, in any crate — layout, paint, hit testing, accessibility or other — that uses component presence (`ImageData`, `FormControlState`, `IframeData`, …) to stand for **replacedness** or for **widget rendering**, or to gate whether an element's children are laid out or painted. Pure paint-source reads are outside it unless they gate children or chrome: the `ImageData` pixel read (`elidex-render/src/builder/walk.rs:376`) only supplies pixels and stays as it is; the `IframeDisplayList` read (`:393-406`) also skips child painting ("skip child painting (break the loop)") and is inside. `IBP-layout` owns the population, measures it in its own plan-memo with a command and a statement of what the command cannot see, and switches every member **together**, each onto the predicate it stands for: a replacedness site (sizing path, whether children are laid out, fallback skipping, monolithic-ness) onto **F4**; a widget-rendering site (paint's `emit_form_control` dispatch, `walk.rs:409`, which draws chrome and control content) onto **F7 with native appearance** (F7 ∧ not devolved, F8). Both come from `IBP-classify`, so E4 orders them. **Where F4 and F7 can differ**: F4 answers every native-appearance widget replaced (css-ui-4 §7.2) **except the `button` element**, whose content is its children (rev 14 attributed this to HTML §15.5.3 with a quotation that is not in the spec — removed), so F4 answers it non-replaced — a "replaced" answer would leave its children unlaid and give it no defined rendering. [Withdrawn at rev 15, not input: rev 14 here claimed that "native widget, non-replaced" could not arise for any other widget and that the `input` button states and every other `input` are replaced; those rows are probe-decided (§1 F4).] For the `button` element the one defined rendering is §15.5.3's: widget chrome around its laid-out children; the control-label painting (`fcs.value()`, `elidex-render/src/builder/form.rs:391`, which for a `button` is its `value` attribute, `elidex-form-core/src/lib.rs` `from_button_element`) does not apply to it. A devolved widget (F8; its replacedness as the probe measures) has no native appearance: CSS lays it out, and paint draws its control content (e.g. a text input's value) without native chrome — its primitive appearance.

Seed — layout crates only (`git grep -n 'get_intrinsic_size\|&ImageData>' -- crates/layout`, less definition `helpers.rs:405`,
re-export `lib.rs:24`, import `elidex-layout-flex/src/fragment.rs:8`, body `helpers.rs:407`): [the population is IBP-layout's plan's, by I1's property, and includes paint (`elidex-render/src/builder/walk.rs:409` → `emit_form_control`) and any other crate] `block/mod.rs:224`
(also `:258`/`:273`/`:282`/`:566`), `positioned/layout.rs:116`, `elidex-layout/src/intrinsic/mod.rs:98`,
`block/children/stack.rs:319`, `elidex-layout-flex/src/fragment.rs:196`/`:353`,
`elidex-layout-multicol/src/fill.rs:418`. Natural-size contract ideas: available `img` → `ImageData` dims;
rule-(4) `img` → 0×0; rule-(2) `img` → probe; `canvas` → bitmap from `width`/`height` (300×150 default,
§4.12.5); `video` → default object size 300×150 (§4.8.8: "The default object size is a width of 300 CSS pixels
and a height of 150 CSS pixels"); `iframe` → `IframeData`; `embed`, `audio` with a UI, outermost `svg` → to
decide; widgets → `form_intrinsic_size`. Cell ideas: L1 `alt=""` img 0×0; L2 no-`alt`/pending by probe; L3/L4
canvas; L5 pending img monolithic, `output` splittable; L6 `output` non-replaced; L7 `img alt=x` non-replaced;
L8 multicol `iframe` and transformed `div` monolithic; L9 `video` 300×150; L10–L12 `embed`/`audio`/`svg`.

## Appendix E — input to IBP-transform's plan-memo

`creates_stacking_context` callers: `elidex-render/src/builder/walk.rs:231`, `paint_order.rs:78`/`:81`/`:151`/`:158`,
`elidex-layout/src/hit_test.rs:186`; family terms at `computed_style/mod.rs:688`/`:693`/`:694`;
`will_change_stacking` derivation `elidex-css-transform/src/lib.rs:157`; `WILL_CHANGE_STACKING_PROPS`
`transform_math/mod.rs:245`; geometry callers `hit_test.rs:136`, `builder/transform.rs:26`, `hit_test.rs:172`,
`walk.rs:315`; `has_transform` reads `hit_test.rs:116`, `positioned/mod.rs:215`. Rev 4 idea: give
`creates_stacking_context` a transformability parameter so every caller must answer it; split the
`will-change` list. Cell ideas: req 8's own pins (fixed descendant; perspective and `will-change` hit tests;
inline-block controls), `will-change: opacity` still stacks, transformed inline `img` transforms, render emits
no transform push for an inline span, the double-layout test.

## Appendix F — input to IBP-observer's plan-memo

`gather_observations` `elidex-api-observers/src/resize.rs:235`; detector `:259-262` (content size for every
`box` option); `SizeProvider` `:19` (content rect + border size only); host closure
`elidex-js/src/vm/host/resize_observer.rs:404-407`; `ResizeObservation.options` stored but unused (`resize.rs:93-94`).
Cell ideas: inline span → content rect (0,0,0,0), padding change → nothing; inline-block and inline `img` →
non-empty; border-box observation → border area, border change fires; a `div` border-box observation fires on a
padding-only change, content-box does not.

## Appendix G — input to IBP-ua-display's plan-memo

Rev 10's worked partition (moved from §0.6 item 7 at rev 11; its sort keys are superseded by item 7's property, which also reaches `content-visibility`):

   - **UA `display` rules.** Measured population: every HTML §15 rule setting `display` whose selector's elements
     compute a different `display` in elidex's production cascade (`elidex-style/src/ua.rs`,
     `elidex-dom-compat/src/legacy_ua.rs`, and inside shadow trees the sheet list at
     `elidex-style/src/walk.rs:510-513`, which omits the compat sheet). The command prints every spec rule with its
     full selector list and any `@media` wrapper:
     `.claude/tools/webref body html rendering | awk '/^[[:space:]]*@media/ {media=$0; next} /^[[:space:]]*$/ {buf=""; next} /\}/ && !/\{/ {buf=""; media=""; next} /\{/ && !/\}/ {sel=buf $0; buf=""; next} /\{.*display[ -:]/ {print NR": "media" "$0; next} /^[[:space:]]*display[ -:]/ {print NR": "media" "sel" -> "$0; next} {buf=buf $0 " "}'`
     (43 rules at `e2d62b9e`), each checked against `grep -n display crates/css/elidex-style/src/ua.rs
     crates/dom/elidex-dom-compat/src/legacy_ua.rs`. **What it cannot see**: `display` behaviour HTML states in
     prose rather than as a rule (§15.5.3's button coercion, §15.4.1's forced `none` for `audio` — both read by hand,
     and the latter is in the population); rules set in other specs' UA sheets; and elidex's `display` values set by
     code rather than by the UA sheets (presentational hints, `blockify`).
     **The one-way comparison** (elidex `display` rules with no spec `display` rule): `ua.rs:128` gives `textarea`
     (and `select` outside the drop-down case) `inline-block`, where HTML has no `display` rule but says in prose
     that the element renders "as an 'inline-block' box" (§15.5.17, §15.5.16) — harmless, the same computed value.
     - **In `IBP-ua-display` (A93 reaches; bounded UA rules, A102), ordered before PR-1a (E12).**
       - Spec `display: none`, and some selected element computes a non-`none` display **whose laid-out subtree can
         contain an inline box** (one such element places the whole rule) — the element itself when it computes
         `inline`, or its descendants when it is laid out as a block: `area, base, datalist, rp`; `[hidden]:not([hidden=until-found i]):not(embed)`; `dialog:not([open])`;
         `[popover]:not(:popover-open):not(dialog[open])`; `@media (scripting) { noscript }`; the non-exposing
         `audio`'s forced `none` (§15.4.1, F16); compat `basefont, noembed, noframes, param`; and the table `form`
         rule, authored as the selector list `table > form, thead > form, tbody > form, tfoot > form, tr > form` (no
         `:is()` needed). Witnesses: an empty decorated element (`<dialog style="padding:10px"></dialog>`) renders
         nothing today — right — and gets a marker pair at **PR-1a** — wrong on the item-stream channel [rev 23: PR-1a lays out no decoration; the
         geometry is PR-1b/PR-1c's, existence PR-1d's, §0.6 item 7]; and a
         script-appended `<form>` in a `<table>` holding `<span style="padding:10px;background:red"></span>` — elidex
         lays a block child of a table out as an anonymous cell (`elidex-layout-table/src/lib.rs:286-289`), so today
         the empty span renders nothing — right — and at PR-1a it gets markers — wrong on the item-stream channel.
       - Spec `block` / `inline-block`, element computes `inline`: `dialog, search, hgroup, menu, option, optgroup
         { display: block }`; compat `dir, listing, plaintext, xmp { display: block }`, `marquee { display:
         inline-block }`; and `center` **inside shadow trees**, where the compat sheet is absent. Witness:
         `<div>x<search style="line-height:100px"></search></div>` — right today (the empty inline `search` adds no
         height to `x`'s line; as a block it would take no part in that line either) and wrong at **PR-1d**, where M6
         gives the empty inline `search` a 100px contribution to `x`'s line. (Rev 8 kept these in the slot on PR-1a
         alone; PR-1d is the PR that reaches them.)
       - E12 is kept to **PR-1a** for the whole sub-PR — the earliest PR that reaches any member; one sub-PR, one
         edge.
     - **In the slot `#11-ua-sheet-html-rendering-display-fidelity` (A96).** Re-checked under both sort keys:
       `details > summary:first-of-type { display: list-item }` — spec value not `none`, and the element computes
       `block` today (`ua.rs:19`), not `inline`; the difference between `block` and `list-item` is the disclosure
       `::marker`, which no program PR changes (the inline boxes inside a `summary` get the same markers, boxes and
       line contributions whether it is `block` or `list-item`). Its repair needs `:first-of-type`, which elidex
       lacks (the supported pseudo-classes, `elidex-css/src/selector/matching.rs:185-240`). `input[type=hidden i] {
       display: none !important }` computes `inline-block` today (`ua.rs:128`) and is void, so its laid-out subtree
       holds no inline box; it keeps its existing home, `#11-form-control-ua-rendering-fidelity` facet (1). Coverage
       limit (A96's form): no cell of this program pins the slot member.
     - **Machinery (input to `IBP-css-machinery` since rev 21), by A102's separator.** UA-sheet authoring first: elidex has no popover, so
       no element's popover visibility state is ever "showing" (HTML §4.16.3 `:popover-open`) and the rule is
       **exactly** `[popover]:not(dialog[open]) { display: none }` while that holds; `dialog:popover-open { display:
       block }` matches nothing. That reduction is authored, with no slot — as for `<br>`/`<wbr>` (umbrella `:6280`),
       the only trigger would be the popover feature itself. Features still needed, each a **bounded** repair in a
       crate the sub-PR already edits (`elidex-css`): the `i` attribute-selector flag (selectors-4 §6.3) —
       `parse_attribute_matcher` (`elidex-css/src/selector/parse.rs:461-490`) consumes no flag, so inside the
       `parse_nested_block` at `:188-200` the selector fails and the `[hidden]` rule would be dropped (the same flag
       is facet (3) of `#11-form-control-ua-rendering-fidelity`, "`[type=… i]` or value normalization", whose UA
       rules stay that slot's); and the `scripting` media feature (mediaqueries-5 §9.1; not among the supported
       features, `elidex-css/src/media/parse.rs:713-726`), whose value is F10 — one feature of the extended set
       `#11-media-extended-features` books (`elidex-css/src/media.rs:20-21`), landed here because its input is this
       program's. `:is()` is not needed (the table rule is a selector list) and `:first-of-type` only by the slot member, so both
       stay out of the program.
       Compat-sheet elements inside shadow trees (`ua.rs:68-73` records the `tt` precedent of keeping an obsolete
       element's rule in the core sheet for that reason): placement is `IBP-ua-display`'s plan's decision, with that
       precedent as input.
     Outside the measured population (same computed `display` today, or not a `display` rule): `embed[hidden]` (its
     `display: inline` equals the initial value; `height`/`width: 0` are not `display` — `IBP-layout`'s cell by §3's
     §15.3.1 row, rev 34); `[hidden=until-found i]`
     (`content-visibility`, which elidex lacks); `slot`, `html`/`body`, the table rules, `li`, `fieldset`,
     `details`/`summary`, `input`/`button`/`select`; the `select` base-appearance rules (conditional on
     `appearance: base` — `#11-appearance-replacedness`); `:host summary` (`details`' UA shadow tree, which elidex
     does not build). `ruby`/`rt` and `br`/`wbr` keep the umbrella's dispositions: ruby is "unimplemented, out of
     scope" (umbrella `:899`), and `<br>`/`<wbr>` "carry no break behaviour engine-wide" with no slot (umbrella
     `:6280`).
