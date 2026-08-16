# Seam-3 prereq split: the IFC's flow reconcile leaves `inline/mod.rs`

Prereq PR of the `#11-line-box-decorated-inline-content` umbrella
(`docs/plans/2026-08-line-box-decorated-inline-content.md`, on branch `layout-decorated-inline`,
frozen at rev 24 and **not** approved for implementation). The umbrella's §8 fixes this PR's
contract and §9 its scope; this memo re-anchors both against **`658cc302`**, the base this PR
actually has.

It also discharges `#11-inline-fragmented-fn-decomposition`'s **seam 3**, whose trigger — "the
next change that touches `layout_inline_context_fragmented`'s body" — the umbrella's PR-1a
fires.

⚠ **This PR carries the code move and nothing else, and that is a narrowing of what the umbrella
§8/§10 assigned it** (user-ratified 2026-08-16). The umbrella routes three further payloads here:
its own plan-memo, its two plan-checker tools, and five slot-ledger rows. All three are *umbrella
program bookkeeping*, and this PR's whole reason to exist ahead of the umbrella's approval is that
**it is independent of that approval** — so a PR justified as approval-independent must not
register the program's slots or rewrite its memory files. The rule applied, which is the umbrella's
own citation rule with "citation" replaced by "ledger row":

> a PR carries the bookkeeping **its own change makes true**, and hands the program's bookkeeping
> to the program.

So §10 keeps only what this move makes true (the source slot's partial close, the `#[allow]`
re-evaluation, that slot memo's falsified figures, the successor's size baseline). The umbrella's
memo, tooling and remaining ledger rows travel with the umbrella. ⚠ This contradicts the
umbrella's §8 sentence "**This memo and the branch's tooling files … ship with the seam-3 prereq
PR** … No later PR re-ships them", which is a ratified surface — it is an input to the umbrella's
owed round 20, not a decision this memo may take silently
([[feedback_plan-ratified-surface-is-a-design-change]]).

**Every `file:line` below is a `658cc302` coordinate**, produced by a command named beside it.

⚠ **Gate applicability**, stated from each checker's actual output rather than from a reading of
it — ⚠ an earlier revision of this paragraph mis-stated both counts and dismissed a checker whose
generic checks do apply, which is the blanket-dismissal shape it exists to prevent.

* **`preflight.py` — 0 hard, 1 soft**, plus one warning outside the soft count. The soft one is
  this line's own `1 entries` enumeration; the other is
  `unrecognized labels: ['CSS 2']`, which is **not noise** — it is why the run also reports
  `parsed citations: 0`, i.e. the §3 citation gate is *vacuous* for this memo as it is for every
  CSS-module plan-memo. ⚠ **Not this PR's to fix, and not waved away either**: extending
  `preflight.py`'s `SPEC_LABEL_REVERSE` to CSS-module labels is the standing plan-checker
  maintenance note in `.claude/skills/elidex-plan-review/SKILL.md`, whose trigger is "the first PR
  that ships these two files" — which, after the narrowing above, is **not this PR**. It travels
  with the umbrella's tooling. What that costs *here* is stated rather than hidden: §3's one
  citation was verified by hand (`webref heading CSS2 10.8`), because the gate could not.
* **`plan-xcheck.py` — 5 findings, all artefacts of shape mismatch**, and the count is 5 only
  after this revision fixed the three that were *not*. It was written for a multi-PR umbrella: it
  harvests §6 as a per-PR cell matrix, cross-checks M-rows and a flip partition, and expects the
  umbrella's PR labels. What survives is `[PR] PR-1a referenced but not defined in §5.3` (this
  memo cites the umbrella's PR-1a without defining it), three `[CELL]`s reading §6's numbered
  harness steps as cells, and `[FLIP]`. ⚠ **Three others were genuine and are now fixed**: its
  `K=`/`M=` breadth check and its per-PR own-deferral check are generic (§3 and §5.3 now answer
  both), and its check 12 correctly caught a superseded line range (`:411-637`) restated beside
  its corrected form in §5.2's quotation.
* The point of recording this is that a reader can tell an inapplicable checker from a skipped
  one — which requires the applicable subset to be named, not the whole run waved off.

## §0. Re-anchoring against `658cc302` — and why nothing moved

The umbrella measured seam 3 at `inline/mod.rs:413-639` on `154bac3f`. Two commits landed since:

```
git diff --stat 154bac3f..658cc302 -- crates/layout/     # → empty
```

so every layout coordinate the umbrella carries is still live, `wc -l …/inline/mod.rs` is **785**
(the number §9 quotes), and the range needs no shift. Verified at both ends:

| line | content (`658cc302`) |
|---|---|
| 413 | `    // Reconcile InlineFlow across the IFC's render-run-groups every pass: persist` |
| 639 | `    }` — the close of `if !env.is_probe { clear_inline_flows(…) }` |
| 640 | blank; `InlineLayoutResult { … }` follows at `:641` |

So `413-639` is the whole reconcile and nothing else: it opens on its own leading comment and
closes on the last statement before the return value.

## §1. What the block is, and why it is a seam

`layout_inline_context_fragmented` (`inline/mod.rs:141`, 508 lines, carrying a pre-existing
`#[allow(clippy::too_many_lines)]` at `:140`) ends in a 227-line block that does one thing:
**reconcile the IFC's render-visible flow state** after packing.

* **`InlineFlow` persist** — fold each render-run-group's lines from IFC-local logical to
  absolute physical, then either write them on the group's run-start key (`:526-529`) or capture
  them into the multicol mid-break carrier (`:544-545`).
* **Atomic reposition** — the static-atomic (`:493-505`) and relpos/sticky (`:576-595`) arms,
  each with the same two-sink split (reposition now vs. carry to the multicol seam).
* **`ColumnFlowSlice` carrier reconcile** — insert-or-remove on `parent_entity` (`:602-616`).
* **`InlineFlow` staleness clear** — `clear_inline_flows` under the probe gate (`:637-639`).

It reads eleven bindings from the enclosing function and produces nothing the enclosing function
reads back: **every effect is a `dom` mutation.** That is what makes it extractable by a signature
alone. ⚠ It is **not** what the source slot meant by "a ready-made module boundary" — that phrase
attaches to the helpers' adjacency, a different property, and §5.2 shows why the distinction
matters.

## §2. The binding set, measured

§8 permits exactly one class of edit: enclosing-fn bindings become parameters. The set, and its
resulting signature (both **measured by compiling the extraction against `658cc302`**, not derived
by reading — the artifact is `reconcile.rs` as it compiled, and every type below is the one the
compiler demanded):

| binding | in `layout_inline_context_fragmented` | parameter |
|---|---|---|
| `dom` | fn parameter | `dom: &mut EcsDom` |
| `parent_entity` | fn parameter | `parent_entity: Entity` |
| `content_origin` | fn parameter | `content_origin: Point` |
| `env` | fn parameter | `env: &crate::LayoutEnv<'_>` |
| `is_vertical` | `:173` | `is_vertical: bool` |
| `persist_flow` | `:322` | `persist_flow: bool` |
| `do_carrier` | `:343` | `do_carrier: bool` |
| `candidate_keys` | `:153`, a `Vec<Entity>` | `candidate_keys: &[Entity]` |
| `unoffset_origins` | `:177`, a `HashMap<Entity, Point>` | `unoffset_origins: &HashMap<Entity, Point>` |
| `packer.flow_lines` | `:251`, moved out at `:454` | `flow_lines: HashMap<Entity, Vec<InlineFlowLine>>` |
| `packer.relpos_atomic_placements` | `:251`, borrowed at `:571` | `relpos_atomic_placements: &[(Entity, f32, f32)]` |

Return type: **unit**. Nothing the block computes outlives it — `persisted_keys`,
`carrier_groups` and `carrier_atomics` are all block-local.

### §2.1 `packer` cannot be passed whole, and that is a fact about the base, not a preference

`packer.static_positions` is moved out at `:389-391`, **before** the seam. A partially-moved
struct cannot be moved again, so the three fields the block reads are passed individually. This
also means the reader-facing consequence is confined to three call-site substitutions (§2.3),
rather than a struct-shaped parameter whose only purpose would be to keep a diff quiet.

### §2.2 `:420` stays behind — the one line of the range that does not travel

`let first_baseline = packer.first_baseline;` (`:420`) sits inside the range but **has no reader
inside it** (`awk 'NR>=421 && NR<=639' … | grep -c first_baseline` → 0). It is parked there
because `packer` is consumed below; its only consumer is `InlineLayoutResult` at `:644`.

Moving it would cost **two** edits that are not binding substitutions — an input parameter
(whose name would collide with the `let`'s own binding) *and* an appended return statement — to
relocate a line that does no reconcile work. It therefore does not travel. This is the only
departure from "the range moves", and it is stated here rather than absorbed silently because §8's
proof obligation is a line-for-line one.

⚠ **It does not survive in the residue either, and that changed at `/simplify`.** The plan as
reviewed kept the `let` verbatim above the call. But the hoist existed *because* 226 lines of
`packer`-consuming code followed it; with those lines gone it became a single-use binding sitting
18 lines above its only reader, directly above a `&mut dom` call — reading as a deliberate
pre-call snapshot when the ordering is incidental. It is now folded into its sole consumer:

```rust
    InlineLayoutResult { …, first_baseline: packer.first_baseline, … }
```

Legal after the call moves `packer.flow_lines`, because the fields are disjoint and
`Option<f32>` is `Copy`; `reconcile_flows` never receives `packer`, so no ordering is observable.
Measured consequence: the residue's clippy line count drops 178 → **177**, which §5.4 records.

The moved body is consequently **226 lines**: `413-419` (the block's own leading comment) plus
`421-639`.

### §2.3 The permitted substitutions — six sites, four bindings

Measured with the harness in §6:

| # | before (`658cc302`) | after | sites |
|---|---|---|---|
| 1 | `packer.flow_lines` | `flow_lines` | `:454` |
| 2 | `&packer.relpos_atomic_placements` | `relpos_atomic_placements` | `:571` |
| 3 | `&unoffset_origins` | `unoffset_origins` | `:494`, `:544`, `:574` |
| 4 | `&candidate_keys` | `candidate_keys` | `:638` |

Substitutions 3 and 4 drop a `&` because the parameter *is* the reference the enclosing binding
was not. ⚠ They are **not optional**: with `unoffset_origins: &HashMap<…>` and
`candidate_keys: &[Entity]`, keeping the `&` compiles (deref coercion) but fires
`clippy::needless_borrow`, which `-D warnings` rejects. The alternatives were measured and each
costs a lint suppression instead: `&Vec<Entity>` fires `ptr_arg`, and by-value `Vec<Entity>`
fires `needless_pass_by_value` (clippy's pedantic group is warn-level workspace-wide,
`Cargo.toml:254`). Dropping the `&` is the only form that needs no `#[allow]`.

`candidate_keys.contains(k)` at `:622` is **unchanged** — `<[T]>::contains` and
`Vec::<T>::contains` take the same argument.

## §3. Spec coverage map

**Breadth**: K=1 specs, M=1 entries (verified 2026-08-16 — `preflight.py` reports the same,
`unique specs (K): 1`, `total entries (M): 1`). **Split decision**: single PR, both below the
K≥4 / M≥20 recommend threshold, and `preflight.py` independently returns
`split decision: ok (single PR scope)`.

⚠ **The row set is a judgment about the change class, and the grep only bounds it from below.**
The moved range's existing citations are

```
git show 658cc302:crates/layout/elidex-layout-block/src/inline/mod.rs \
  | awk 'NR>=413 && NR<=639' | grep -nE '§|CSS |UAX|spec'      # → 1 line, offset 68 = :480
```

but that pattern can only match prose **that already carries a citation**, so its output is not a
measurement of the spec surface — presenting it as one would be a universal claim over an
unmeasured complement. **The complement is non-empty**, and this is the command that shows it:

```
git show 658cc302:crates/layout/elidex-layout-block/src/inline/mod.rs | awk 'NR>=413 && NR<=639' \
  | grep -inE 'logical|physical|text-align|sticky|fragmentainer|overflow|abspos|paged|margin-box'
```

Spec-governed concerns it surfaces, none of them cited in the source, each §-number↔title pair
resolved with `.claude/tools/webref heading`:

* `:437-447` — IFC-local logical → absolute physical fold keyed on `is_vertical`
  → **css-writing-modes-4 §6.4** *Abstract-to-Physical Mappings*
* `:470` — `text-align` already baked into `inline_start` → **css-text-3** `text-align`
* `:557-569`, `:588-594` — relative/sticky offset preserved through reposition
  → **css-position-3 §3.3** *Relative Positioning* / **§3.4** *Sticky positioning*
* `:514` — the term "fragmentainer" → **css-break-4 §2** *Fragmentation Model and Terminology*
* `:483-492`, `:531-554` — a box continuing across **column boxes**
  → **css-multicol-1 §2** *The Multi-Column Model* (`webref dfn css-multicol-1 'column box'` → §2).
  ⚠ An earlier drafting cited §7 *Filling Columns*, which is column **balancing** (`column-fill`) —
  the wrong section
* `:419`, `:550-554`, `:506-510` — the abspos toggle; `overflow:hidden` clipping; the paged path
  and its page generation → **css-position-3 §2** *Choosing A Positioning Scheme*;
  **css-overflow-3** `overflow`; **css-break-4 §2**

⚠ **This list is a lower bound too**, not a closed enumeration — an earlier drafting called it
"measured" while giving no command, and §9 then promoted it to a definite description. The claim
it supports needs only non-emptiness.

**They are nonetheless out of scope, by change class rather than by grep**: this PR authors no
algorithm, so it neither creates nor deepens a missing-citation defect. That is exactly the
position #497 took when it declined to add a §9.4.2 module-doc citation to
`collect.rs`/`styled_run.rs` as over-claiming. §9 books the class rather than dropping it. The one
row below is therefore what the PR *carries*, and the map's honest breadth claim is "one citation
travels unchanged", not "one citation is the surface".

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSS 2 §10.8 Line height calculations: the `line-height` and `vertical-align` properties | `vertical-align` within the line box | not implemented — the atomic's block-axis reposition target is the line top (baseline-naive). ⚠ **Both** sinks, not the mid-break one: the citing comment sits under `if persist_flow {` (`:468`), and the target itself comes from `static_atomic_reposition_records`, whose docstring calls itself "the SINGLE derivation shared by both the `persist_flow` sink … and the `do_carrier` sink" (`:717-720`) and which returns `line.block_start` (`:734`) | comment only, at `:480-481`; moves verbatim to the new module, no code touched. Title↔number pair verified with `.claude/tools/webref heading CSS2 10.8` | ✓ for citations carried; the uncited complement is §9's | yes |

## §4. Verified current state

Every row's Result is the command's **whole** output, not a reading of it.

| claim | command | result |
|---|---|---|
| the range is unchanged since the umbrella measured it | `git diff --stat 154bac3f..658cc302 -- crates/layout/` | empty |
| `inline/mod.rs` size before | `git show 658cc302:crates/layout/elidex-layout-block/src/inline/mod.rs \| wc -l` | 785 |
| the two record-producing helpers are called only from inside `413-639` | `git grep -n 'static_atomic_reposition_records' 658cc302 -- crates` | four lines: `:494`, `:544` (calls, both inside the range), `:721` (the **definition**), `:745` (an intra-doc link in the sibling helper's docstring) |
| — same, relpos | `git grep -n 'relpos_atomic_reposition_records' 658cc302 -- crates` | two lines: `:570` (call, inside), `:747` (definition) |
| `clear_inline_flows` has residue callers | `git grep -n 'clear_inline_flows' 658cc302 -- crates` | 13 lines. `inline/mod.rs`: calls `:162`, `:201` (the two early returns) and `:638` (inside the range); definition `:775`; **comments `:157`, `:549`, `:597`** — the last two inside the range (see §7). Elsewhere: `shift.rs:129`, `inline_flow.rs:219`, multicol `lib.rs:500`/`:625`/`:635`, `tests/mid_break_flow.rs:603`, all comments |
| `reposition_atomic_box` is public API, called only from inside the range | `git grep -n 'reposition_atomic_box' 658cc302 -- crates` | 13 lines. `inline/mod.rs`: definition `:679` (`pub`), **calls `:496`, `:578` — both inside the range**, comment `:176`. Cross-crate: `elidex-layout-multicol/src/lib.rs:11` (import), `:664` (call), `:646` (comment). Comments: `shift.rs:70`/`:155`, `atomic.rs:17`, `multicol/src/tests/mid_break_flow.rs:1042`/`:1055`/`:1210` |
| the moved range holds no `LayoutBox`/`BoxModel` **reader** | `git show 658cc302:…/inline/mod.rs \| awk 'NR>=413 && NR<=639' \| grep -cwE 'LayoutBox\|BoxModel'` | 5, and all five are `//` comment lines (`:469`, `:472`, `:524`, `:535`, `:557`) — ⚠ the wire greps **both** tokens, so the `BoxModel` half was measured too: 0 |
| …and comment lines cannot reach the wire | `awk 'NR>=124 && NR<=126' .claude/tools/layout-box-reader-trip-wire.sh` | `:124` is the `git grep -nwE 'LayoutBox\|BoxModel'`, **`:125` is `\| strip_comments`**, `:126` the test-path filter |
| …so the allowlist is unchanged | `grep -n 'inline/mod.rs' .claude/tools/layout-box-reader-allowlist.tsv` | one row (TSV `:73`), classified `type-def`, whose content line is `inline/mod.rs:29` — above the range, and unmoved |

## §5. Design

### §5.1 Home and name

New module `crates/layout/elidex-layout-block/src/inline/reconcile.rs` (NEW), holding
`pub(super) fn reconcile_flows(…)`. `inline/mod.rs` gains `mod reconcile;` beside its existing
siblings and calls it where the block was.

`reconcile` matches the sibling idiom — `atomic`, `measure`, `pack`, `collect`, `styled_run`,
`whitespace` (`ls crates/layout/elidex-layout-block/src/inline/`) are each named for one concern,
in the shortest form that names it. `pub(super)` is the narrowest visibility that works: the sole
caller is `inline` itself.

`reconcile_flows` rather than `reconcile_inline_flows` because the block reconciles **two**
components — `InlineFlow` and the `ColumnFlowSlice` carrier — and naming one of them in the
function would misdescribe the other. The docstring names both.

### §5.2 All four adjacent helpers stay in `inline/mod.rs`

The moved block calls four module-private / `pub` helpers that sit below it. **All four stay**;
`reconcile.rs` reaches them with one `use super::{…}`. The move is the range and nothing else.

| helper | definition on `658cc302` | why it stays |
|---|---|---|
| `reposition_atomic_box` | `:679` (`pub`) | public API — `elidex-layout-multicol/src/lib.rs:11`, `:664` |
| `static_atomic_reposition_records` | `:721` | outside `413-639`; the range is what the umbrella ratified |
| `relpos_atomic_reposition_records` | `:747` | same |
| `clear_inline_flows` | `:775` | two residue callers, `:162` and `:201` (the early returns) |

Coordinates from `git show 658cc302:crates/layout/elidex-layout-block/src/inline/mod.rs | grep -nE '^(pub )?fn '`.

⚠ **An earlier revision of this memo proposed moving two of the four, on a premise that
measurement refutes.** It read the source slot memo
(`project_inline-fragmented-fn-decomposition.md:29`) as defining seam 3 to include its helpers,
and therefore as disagreeing with the umbrella's `mod.rs:413-639`. Re-read, that entry opens by
naming the seam as a line range (in its own pre-#497 frame, which the umbrella's `:413-639`
supersedes) and then says of the helpers:

> Its four helpers (…) already sit immediately below at `:648-783`, so this one has a
> **ready-made module boundary**.

(⚠ `:648-783` is that memo's own pre-#497 frame; on `658cc302` the helper span is `650-785`,
uniformly +2 — the same shift that takes its `:138`/`:139` to this base's `:140`/`:141`.)

The helpers are named as **adjacent evidence that the seam is clean** — the reason the extraction
will not tangle — not as members of it. ⚠ **The decisive support is arithmetic, not grammar**: the
same entry sizes the seam "~227 lines", and `637 − 411 + 1 = 227` is the range **alone**; range
plus helpers is ~363. So the two
documents never disagreed, and there was nothing for this review to adjudicate. Recorded rather
than deleted because the withdrawn argument's *other* half was wrong too, in a way worth not
repeating: "leaving them makes the residue hold private functions no line of the residue calls"
treats a parent-module helper called by a child module as dead code. It is the ordinary Rust
arrangement, and the widening would have replaced one uniform rule (all four live beside each
other) with a 2/2 split — the *opposite* of *one issue, one way*. ⚠ An earlier drafting added
"and it is what the other two helpers do anyway", which is **false and measured so**: neither of
the other two is an instance of "private, with no residue caller" — `reposition_atomic_box` is
`pub` (`:679`), and `clear_inline_flows` has two residue callers (`:162`, `:201`), as §5.2's own
table says. The uniformity argument stands on its own; that sentence did not.

⚠ The withdrawal also repairs three claims that depended on it, each of which had drifted into a
different artifact: §5.2's four coordinates were **post-split residue** coordinates (uniformly
−212, exactly §5.5's residue delta) rather than the `658cc302` ones this memo's preamble promises;
§5.5's sizes described the un-widened configuration while §8's DoD specified the widened one; and
"the §6 harness covers them under the same criterion" was false, because §6 extracts one function
body and the helpers are outside it. All three were symptoms of one cause — **the configuration
that was compiled and harnessed was not the configuration the memo recommended**
([[feedback_verified-claims-go-stale-under-own-later-edits]]). With the widening withdrawn, the
measured artifact and the specified artifact are the same one again, and §5.5 / §6 / §8 describe it.

### §5.3 `#[allow(clippy::too_many_arguments)]` on the new function

**Own-deferral count: the seam-3 prereq opens ONE — `#11-inline-fragmented-fn-seams-1-2`.**
⚠ An earlier drafting said "none", on the premise that everything §9 defers is "routed to a slot
that already exists". Both halves were false, and measured so: the successor slot **is created by
this PR** (`grep -rl '#11-inline-fragmented-fn-seams-1-2' <memory-dir>` returns only files this PR
writes), and `#11-inline-spec-cite-misattribution` is not open either (§9 now claims no routing to
it at all). Of the four items on the new slot, **two are pre-existing** (seams 1 and 2, named by
the source slot on 2026-07-28) and **two are created here** (`reconcile_flows`' eleven-parameter
signature and its adjacent-`bool` window, which §9 itself calls new). One own slot is within the
≤3 per-PR cap, and §10 carries its `(own)` row.

Eleven parameters exceeds clippy's threshold of seven. **Measured, not assumed**: removing the
attribute and running `cargo clippy -p elidex-layout-block --all-features` reports
`too_many_arguments`, so the attribute is load-bearing.

It is the crate's established idiom for exactly this shape — `inline/collect.rs:193`,
`inline/atomic.rs:25`, `inline/pack/mod.rs:678`, `block/children/helpers.rs:36` and `:198` all
carry it. ⚠ **The parameter count is the honest measurement of this seam's coupling, and this PR
does not reduce it**: reshaping the signature is a design change, and a design change is precisely
what "byte-identical modulo the extracted signature" exists to keep out of a move. It is not
deferred silently either — §9 books it, **with the candidate shapes named**, because a booking
that names only the OO answer gets only the OO answer when the slot is picked up: a context struct
or a fold into `LayoutEnv` are groupings, and three of the eleven parameters
(`unoffset_origins`, `flow_lines`, `relpos_atomic_placements`) are **entity-keyed side-stores**,
which is the shape CLAUDE.md's *side-store→component 判定ルール* is written about. Not a claim they
should become components — they are intra-pass scratch, consumed before the pass ends, which is a
defensible reading of that rule's scope — but the ECS question must be *on* the slot, not absent
from it.

### §5.4 `#[allow(clippy::too_many_lines)]` on the residue

§8 requires this re-evaluated, and the source slot says (`:37`) to drop it "if the residue no
longer needs it". **It still does.** Measured by deleting the attribute with the split applied:
`this function has too many lines (177/100)` (178 before §2.2's fold). It stays, and the measurement is recorded so the
next toucher re-runs it rather than re-litigating it.

### §5.5 Resulting sizes

`inline/mod.rs` **785 → 573**; `reconcile.rs` **265**. ⚠ The plan predicted 254 from the
exploratory extraction; the shipped module is **11 lines longer**, carrying a real module doc and
a docstring on `reconcile_flows` that the throwaway did not. Recorded as a correction rather than
silently overwritten — §5.6 exists so that a predicted figure and a measured one stay
distinguishable. Both files sit below
[[feedback_touch-time-split-means-while-writing]]'s 700–800 band, and the residue is 212 lines
further from the 1000-line gate than it was — which is the source slot's *second* trigger
disjunct, and this PR moves it away from firing rather than toward it.

### §5.6 Provenance of §5.3–§5.5's figures

**Status: re-measured on the committed implementation.** Every figure below was first taken on a
throwaway extraction that was not in the branch, and §8 required each to be re-run against the
shipped tree before landing. Result of that re-run:

| figure | predicted | measured on the implementation |
|---|---|---|
| `too_many_arguments` load-bearing | yes | yes — `this function has too many arguments (11/7)` |
| `too_many_lines` still load-bearing on the residue | yes, `178/100` | yes, **`177/100`** (§2.2's fold) |
| `inline/mod.rs` | 573 | **573** |
| `reconcile.rs` | 254 | **265** (§5.5 — the docstrings) |
| §6 harness | 6 hunks, `226 == 226` | **6 hunks, `226 == 226`, PASS** |
| test baseline | 325 | **325 passed, 0 failed** |

The recipe the numbers come from, so a reader can re-derive rather than trust:

```
# 1. reconcile.rs = module doc + the `use` block + `#[allow(clippy::too_many_arguments)]`
#    + the §2 signature, then the range's lines 413-419 and 421-639 with §2.3's four
#    substitutions applied. ⚠ The `use` block is what makes `wc -l` determinate, so it
#    is part of the recipe: std `HashMap`; `elidex_ecs::{ColumnFlowSlice, EcsDom, Entity,
#    InlineFlow}`; `elidex_plugin::Point`; and `super::{clear_inline_flows,
#    relpos_atomic_reposition_records, reposition_atomic_box,
#    static_atomic_reposition_records}`. `InlineFlowLine` needs none — the range already
#    writes it fully qualified (`:426`, `:458`).
# 2. mod.rs = the same file with 413-639 replaced by `:420` verbatim + the call,
#    and `mod reconcile;` added beside the sibling `mod` declarations.
cargo clippy -p elidex-layout-block --all-features      # → clean, with both #[allow]s
cargo test   -p elidex-layout-block --all-features      # → 325 passed, 0 failed
wc -l crates/layout/elidex-layout-block/src/inline/{mod,reconcile}.rs
```

⚠ **One prediction was wrong, and that is the point of the table**: `reconcile.rs` came out at 265
rather than 254. A memo that had simply asserted 254 would now be carrying a false figure into
§10's successor-slot baseline; instead the discrepancy is visible and the baseline takes the
measured number ([[feedback_verified-claims-go-stale-under-own-later-edits]]).

## §6. Proof obligation

§8's criterion — "a diff of the moved body against the same block extracted from that base
differs only in those bindings" — is discharged by a harness, not by inspection:

1. Extract `413-419` + `421-639` from `git show 658cc302:…/inline/mod.rs`.
2. Extract the new function's body from `reconcile.rs` (everything between the signature's
   closing `) {` and the final `}`).
3. `difflib.unified_diff` with `n=0`.

**Pass condition**: every hunk is one of §2.3's four substitutions, and the two extracts have the
same line count (226). Run against the exploratory extraction on `658cc302`, the harness returns
**six single-line hunks**, one per site in §2.3, and `226 == 226` — a figure inheriting §5.6's
pending status. The same harness is re-run on the committed implementation and its output goes in
the PR body.

⚠ The harness covers exactly the moved range, which since §5.2's withdrawal is exactly what the
PR moves — no `fn` sits outside both extracts. That correspondence is a DoD clause (§8), not an
assumption: if any later revision widens the move, the harness stops proving the whole of it.

⚠ The harness compares against **`origin/main`, not against a diff file written earlier in the
session** — a pre-generated diff goes stale under the author's own later edits, which is the
failure [[feedback_verified-claims-go-stale-under-own-later-edits]] names and which the umbrella's
own rev-22 gate caught in this lane.

## §7. Downstream

**Nothing outside the crate can observe this PR.** The three surfaces that could, checked:

* **Public API** — unchanged. `reposition_atomic_box` stays in `inline/mod.rs` with its
  `pub` visibility and its `elidex-layout-multicol` consumers untouched; the new module is
  `pub(super)`.
* **Behaviour** — the block's only effects are `dom` mutations, and §6 proves the *statements*
  producing them are the same statements in the same textual order. ⚠ **The write order needs a
  second, different ground**, because the principal write loop drains a `HashMap` — `flow_lines`
  is `HashMap<Entity, Vec<InlineFlowLine>>` (`inline/pack/mod.rs:154`), whose iteration order is
  unspecified, and inside it the block both writes `InlineFlow` (`:528`) and pushes
  `carrier_groups` → `ColumnFlowSlice.flow_groups` (`:609-611`), read back by
  `elidex-layout-multicol/src/fill.rs:235`. The ground that does hold: the **same map instance is
  moved** into the callee, not rebuilt from another source, so any given pass drains in exactly
  the order it would have. Whether that consumer is order-sensitive at all is pre-existing and
  untouched; §9 routes it.
* **The `LayoutBox`/`BoxModel` reader allowlist** — no row owed (§4), because the moved range's
  five `LayoutBox` tokens are all in comments (and its `BoxModel` count is 0), and the wire strips
  comment lines *before* matching (`layout-box-reader-trip-wire.sh:125`). Asserted by running
  `mise run trip-wires`, not by this paragraph.

### §7.1 Comments the move makes less accurate — the whole class, measured

⚠ **An earlier revision said "**One** thing the move does make less accurate" and named only the
two sites *inside* the range.** That is a closed-set claim whose complement — comments **outside**
the range that address the block by its old location — was never measured, in a memo that invokes
[[feedback_universal-claims-need-the-complement-measured]] twice elsewhere. Measured now, and
⚠ **a line-based grep is not enough**: one site wraps the phrase across two `//` lines, so the
sweep flattens comment continuations first:

```
python3 - <<'EOF'
import re, pathlib
for p in pathlib.Path('crates/layout/elidex-layout-block/src').rglob('*.rs'):
    flat = re.sub(r'\n\s*//[/!]?', '', p.read_text())
    for m in re.finditer(r'persist block|reconcile (comment )?in `layout_inline_context_fragmented`', flat):
        print(p, flat[m.start()-45:m.end()+35])
EOF
```

⚠ **That sweep is rooted at `crates/layout/elidex-layout-block/src` and matches two phrasings, so
it is narrower than the class.** Re-run over the whole workspace with `Written by` added, it
returns a **sixth** site — `crates/core/elidex-ecs/src/components/inline_flow.rs:204`, the
`ColumnFlowSlice` component docstring: "Written by `layout_inline_context_fragmented` on the **IFC
container** entity". **Left deliberately, and this one is not the concept/location split**:
`reconcile_flows` is `pub(super)` inside a private `mod reconcile;`, so from `elidex-ecs` it is not
a nameable symbol at all — repointing the component's SSoT at something unreachable would be worse
than the mild imprecision, and `layout_inline_context_fragmented` remains the only entry point
through which the write happens.

**Six sites outside the range**, splitting on whether they name a *location* or the block as a
*concept*:

| site | names | disposition |
|---|---|---|
| `mod.rs:175-177` | "see the persist block's `reposition_atomic_box` calls" — an in-file pointer, and no such block is in this file now | **fixed here** |
| `mod.rs:562` | "the reconcile comment in `layout_inline_context_fragmented`" | **fixed here** |
| `collect.rs:130` | "see the reconcile in `layout_inline_context_fragmented`" | **fixed here** |
| `collect.rs:209` | "the projection axis the persist block uses for ALL groups" | **left** — names the block as a concept, still true |
| `atomic.rs:17` | "The persist block uses this as the reposition delta basis" | **left** — same |

The three fixed ones name a location that moved; the two left name a thing that still exists. That
is the line, and it is why `collect.rs` appears in this PR's diff (§8).

⚠ **Two sites inside the range were reported as unfixable collateral, and both were non-defects.**
An earlier drafting said `:549` ("by the `is_probe`-gated `clear_inline_flows` **below**") and
`:597` ("Carrier reconcile (insert-or-remove, **mirroring** `clear_inline_flows`)") had gone wrong
because `clear_inline_flows` now lives in the parent module. **Withdrawn — measured:**

* `clear_inline_flows` takes no probe flag (`fn clear_inline_flows(dom, candidates, persisted)`),
  so "the **`is_probe`-gated** `clear_inline_flows`" cannot name the definition. It names the
  **call** guarded by `if !env.is_probe`, and that call travelled with the block — it is at
  `reconcile.rs:263`, in the same file, **below** the comment at `:174`. Still true.
* "mirroring" asserts a shape analogy and makes no location claim at all.

The error was reasoning about where the *definition* went without checking what the comments
actually referred to. ⚠ It propagated: `c931dad5`'s commit message records the non-defect as
"known and deliberate" and cannot be amended (the hooks deny it), and the successor slot memo
carried it as work to do until this revision removed it. **Nothing inside the range needed
fixing**, so the byte-identity contract cost this PR nothing here — which is the honest version of
the claim, and a stronger one.

## §8. Definition of done

* `crates/layout/elidex-layout-block/src/inline/reconcile.rs` (NEW) exists and holds the module
  doc, the `use` block §5.6 pins, the attribute, and `reconcile_flows` — and **no `fn` outside the
  one §6's harness extracts**, which is what makes the harness a proof of the whole move rather
  than of part of it. (The constraint is on `fn`s, not on "nothing else": the imports are
  mandatory, since the moved lines name `InlineFlow`, `ColumnFlowSlice`, `EcsDom`, `Entity`,
  `Point` and `HashMap` unqualified.)
* §6's harness passes and its output is in the PR body.
* `cargo test -p elidex-layout-block --all-features` green with **no test touched** —
  `git diff --name-only origin/main...HEAD` names no file under `.../tests/` or `tests.rs`.
  ⚠ **The ground, not just the outcome**: the `#[cfg(test)] pub(crate) use` re-exports the test
  modules reach through are at `mod.rs:28-34`, entirely above the range, and the one `pub` item
  the block calls (`reposition_atomic_box`) stays — so no test path can change. Baseline: 325
  tests, per §5.6.
* `mise run ci` green — including `trip-wires` (§7) and `cargo doc` with `RUSTDOCFLAGS=-D warnings`
  (`relpos_atomic_reposition_records`'s intra-doc link to `[static_atomic_reposition_records]`
  (`mod.rs:745`) must still resolve — it does trivially, both stay).
* **Every figure §5.6 marks pending is re-measured on the committed tree** and recorded in the
  commit message: the two `#[allow]`s' necessity (`too_many_arguments`; `too_many_lines` at
  the figure §5.4 records), both `wc -l`s, and §6's hunk count. ⚠ **Figures are referenced, not
  restated** — every duplicated measurement in this memo drifted at least once.
* **`git diff --name-only origin/main...HEAD` names four files** — this memo, `inline/mod.rs`,
  the new `inline/reconcile.rs`, and `inline/collect.rs` (one comment, §7). ⚠ **Nothing under
  `.claude/`, and no second `docs/plans/` file** — that is the mechanical statement of the
  narrowing in the preamble, and the cheapest way for a reviewer to confirm it.
* §10's ledger actions applied.

## §9. Out of scope, with disposition

* **Seams 1 and 2** of `#11-inline-fragmented-fn-decomposition` — `mod.rs:266-303` (orphans/widows
  break computation) and `:388-411` (the packer-relative → layout coordinate fold), both
  re-measured on `658cc302`. The umbrella books them into the successor slot
  `#11-inline-fragmented-fn-seams-1-2` (§10). They are not folded in here because the umbrella's
  scope sentence is explicit and because neither is touched by PR-1a, so neither has a fired
  trigger.
* **The eleven-parameter signature.** Reducing it is a design change (§5.3) and belongs with the
  successor slot `#11-inline-fragmented-fn-seams-1-2`, whose subject is the residue's
  decomposition. **Its existing trigger already reaches it**, and no new disjunct is needed: the
  eleven-argument *call site* lives in `layout_inline_context_fragmented`'s body (§5.1 — "calls it
  where the block was"), so every reshaping §5.3 names necessarily edits the residue, which is
  disjunct 1. ⚠ **An intermediate revision argued the opposite** — that both disjuncts name
  `inline/mod.rs` while `reconcile_flows` is in the new file, so neither could ever fire — and was
  wrong, because it reasoned about the definition and forgot the call. It reached that conclusion
  by adopting a reviewer's finding without re-deriving it, which is the one thing this program's
  front matter exists to prevent. What §10 *does* carry is the slot's subject line naming §5.3's
  candidate shapes **including the side-store→component one**, so the question is not pre-answered
  as a grouping.
  ⚠ **Booked alongside it, because it is a different defect the count would hide**: the signature
  ends `is_vertical: bool, persist_flow: bool, do_carrier: bool` and the call site passes them
  positionally, so **any transposition of the three is type-correct and compiles silently**. That
  window is *new* — pre-split these were three named `let` bindings in scope (`:173`, `:322`,
  `:343`). Measured across all non-test `crates/layout` source, only four functions have two or
  more adjacent `bool` parameters, and `reconcile_flows` is the **only one in
  `elidex-layout-block`**; the crate's other wide signatures separate them, apparently
  deliberately — `layout_atomic_items` (`atomic.rs:26`) puts `layout_generation: u32` between
  `is_vertical` and `is_probe`. A slot told only "eleven is too many" may answer with a grouping
  that keeps the triple adjacent, so the adjacency is named on the slot, not just the count.
  ⚠ **The ground for deferring it is *not* byte-identity**, and an earlier drafting implied it was:
  §6 extracts only the text between the signature's `) {` and the final `}`, so the signature sits
  outside the compared region **by construction** and a parameter reorder would be harness-neutral.
  The real ground is altitude — the fix for a three-`bool` positional window is a **type** (an enum
  or a flags struct, so a transposition fails to compile); shuffling an unrelated parameter between
  them to defeat ordering is a bandaid that leaves the hazard's shape intact. That is design work,
  which is what this PR excludes.
* **Where the four helpers should live once their principal caller is a sibling module.** §5.2
  keeps all four in `mod.rs`, and two of its four reasons are *design* reasons (`pub` API;
  residue callers) while two are *scope* reasons (outside the ratified range). ⚠ A third
  configuration exists that §5.2 does not weigh — all four into a shared sibling imported by both
  `mod.rs` and `reconcile.rs`, which satisfies the uniformity argument **and** removes the
  child→parent back-edge. Declining it here is right (it is outside the range), but leaving it
  unrouted would let the next reader take §5.2 as "settled" rather than "declined on scope".
  Routed to `#11-inline-fragmented-fn-seams-1-2`.
* **The uncited spec-governed prose inside the moved range** — the list lives in **§3 and is not
  restated here**; an earlier drafting copied it and the copy drifted to `css-multicol-1 §7`, the
  section §3 itself records as wrong. **Pre-existing, untouched, and routed nowhere by this PR.**
  ⚠ An earlier drafting claimed this bullet "adds the seam-3 range's uncited-prose class" to
  `#11-inline-spec-cite-misattribution`. Over-reach on two measured counts: that slot is **not
  open** (`grep -rl '#11-inline-spec-cite-misattribution' <memory-dir>` → nothing; it exists only
  inside the unapproved umbrella, which books opening it to PR-1a), and its scope is the
  umbrella's **wrong-section** class — misattributed citations, found by three concept greps — not
  *missing* ones. A PR cannot enlarge another program's unopened slot by asserting it in prose. So
  this memo claims no routing: the prose was uncited before and after, this PR authors no
  algorithm, and adding citations would be an edit to the moved lines that §6 fails on.
* **The CSS 2 §10.8 `vertical-align` deferral** that §3's one row records — likewise
  pre-existing, and owned by the umbrella itself (its §5.3 books the line-box height/baseline work
  under `#11-inline-root-inline-box`). Recorded here so the row is dispositioned rather than
  merely observed.
* **Cold gate** ([[feedback_split-on-touch-prereq-workflow]]), re-run on `658cc302` at this PR's
  own touch set (`crates/layout/elidex-layout-block/` plus the four `.claude/`+`docs/` files of
  `f63eb623`):
  * Open PRs: `gh pr diff <n> --name-only` for each of the six open PRs (506, 505, 503, 502, 501,
    381) → none touches `crates/layout/elidex-layout-block/`, and `comm -12` against this branch's
    name set is empty for every one. ⚠ **File-disjointness is not the whole gate for #501**: it
    edits `preflight.py`, the checker whose verdict this memo's preamble quotes, so there is a
    *behavioural* dependency the name-set intersection cannot see. Dispositioned above (§9's
    plan-checker bullet) rather than left to the `comm -12`; the preamble's preflight figures are
    re-run at push time either way.
  * Unmerged refs: `git diff --name-only origin/main...<ref>` over **every** local and `origin/`
    ref (not a hand-picked subset — the loop's output is the set) → **nine** touch
    `crates/layout/elidex-layout-block/`, of which **five** touch `inline/mod.rs`:
    `origin/feat/inline-pipeline-slice3`, `…slice3p`, `…slice3pb`,
    `origin/feat/white-space-collapsing` and `origin/layout-css2-cite-sweep`. Dispositions,
    measured rather than assumed:
    * The four `feat/…` refs are **abandoned**: 207–216 commits behind `origin/main`, last commit
      2026-06-01…06-03. Any of them would conflict with far more than this seam if revived; that
      is a pre-existing condition of those branches, not a collision this PR creates.
    * `origin/layout-css2-cite-sweep` is **#497's unsquashed history, already in `main`** —
      `git diff --stat origin/main...origin/layout-css2-cite-sweep` and
      `git show --stat 154bac3f` both report `15 files changed, 76 insertions(+), 34 deletions(-)` (verified 2026-08-16).
    * **The remaining four of the nine touch the crate but not `inline/mod.rs`**, which is why
      they drop out at the file width: `feat/m4-1.5-2-plugin-arch-anim` (693 behind),
      `origin/chore/css-break-3-citation-sweep` and `origin/fix/destructuring-params` (both 197
      behind), and `layout-text-height-split` — which is additionally **#500's unsquashed history,
      already in `main`**: `git diff --stat origin/main...layout-text-height-split` and
      `git show --stat 4357cd4c` both report `11 files changed, 1393 insertions(+), 1345
      deletions(-)` (verified 2026-08-16). Named because disposing six of nine and leaving the
      rest unstated is the very complement gap the ⚠ below records.
    * ⚠ An earlier drafting of this bullet named only two refs and was **wrong** — it stated a
      conclusion the loop had not been run to produce. Recorded because the class recurs
      ([[feedback_universal-claims-need-the-complement-measured]]): a "none besides X" claim is a
      claim about the complement, and only running the loop measures it.
  * ⚠ The gate is re-run at push time, not trusted from here — `main` moves.

## §10. Slot ledger actions at landing

⚠ **The umbrella's §10 books five rows to "the seam-3 prereq PR"; four of them leave** with the
narrowing in the preamble. The test applied to each is the same: *does this PR's change make it
true?*

**Leaving** — all keyed to the umbrella program, none to this move: registering the umbrella's own
slot in `project_open-defer-slots.md` — ⚠ **and only its own**. An earlier drafting sent away
"the two other unregistered Layout-lane slots", one of which the umbrella names as
`#11-inline-fragmented-fn-decomposition`, i.e. the very slot the *Staying* table below registers
and closes. Booked on both sides, it would have shipped as neither. The umbrella keeps
`#11-css2-spec-label-normalisation` (2026-10-31) and its own slot (2026-11-01);
correcting `#11-layoutbox-trip-wire-not-in-ci`, whose premise **#496 (`da958ace`) falsified**, not
this PR; rewriting `project_line-box-decorated-inline-content.md`, `MEMORY.md`'s Layout-lane entry
and `active-lane-detail.md`, which carry the *umbrella's* framing; and shipping the plan-checker
standing note, which travels with the tooling. Each is real and none is dropped — they land with
the umbrella, and this narrowing is an input to its owed round 20.

⚠ **Leaving is not the same as delivered, and one row needs a mechanism rather than a sentence.**
The plan-checker standing note's trigger is an *event* — "the first PR that ships these two files"
— and this PR **is** that event and ships neither, so the trigger passes unfired rather than
transferring. The umbrella's next session reads `project_line-box-decorated-inline-content.md`,
whose "NEXT SESSION STARTS HERE" still instructs it to carry the memo and tooling on the seam-3
branch. So this memo's "an input to its owed round 20" is inert unless written there. **§10
therefore carries a row that writes the narrowing into the umbrella's own SSoT**, below.

**Staying** — the five this move makes true:

| Action | Note |
|---|---|
| Register **and** close `#11-inline-fragmented-fn-decomposition` in one row, as a **partial** close | this PR discharges seam 3 and only seam 3. Seams 1 and 2 (§9) stay open in the successor slot `#11-inline-fragmented-fn-seams-1-2` (pre-existing class; trigger and self-exemptions verbatim from the umbrella's §10 row; re-eval 2026-11-01). The slot's subject line names §5.3's candidate shapes for the eleven-parameter signature, **including the side-store→component one**, so the question is not pre-answered as a grouping — and §9 records that the slot's *existing* disjunct 1 already reaches it, because the call site is in the residue |
| Correct **every** fact this PR falsifies in `project_inline-fragmented-fn-decomposition.md` | the class, measured on that file: `:3` (front-matter, "508 lines … three concrete seams"), `:17` (its own frame: `mod.rs:139-646` = 508 lines, `#[allow]` at `:138`, "After the split `mod.rs` is **783 lines**" — ⚠ those are *its* pre-#497 coordinates, which on `658cc302` read `:141-648` / `:140` / 785, and after this PR the file is 573), `:29` (seam 3 listed open), `:37` ("On landing, drop the `#[allow(clippy::too_many_lines)]` if the residue no longer needs it" — **re-evaluated and kept** — ⚠ take the figure from §5.4 at landing rather than from this row; the number moved once already when `/simplify` folded the hoist, and a row that restates it is the duplication that drifted) and `:42` (the 783 band argument). ⚠ Sweeping only `:37` would leave the memo asserting a size, a line range and an open seam this PR closes — the *statements* surface left standing while the *obligation* surface was fixed ([[feedback_sweep-obligations-not-only-statements]]) |
| Correct `project_inline-mod-split-owed.md:82` — "leaving `mod.rs` at **783**" | a sibling site of the same class, in a different file, reached by neither row above. A class swept per-file is a class swept partially ([[feedback_semantic-sibling-selfseed-and-regate-breadth]]) |
| **Register `#11-inline-fragmented-fn-decomposition` (as partially closed) and `#11-inline-fragmented-fn-seams-1-2` (`(own)`) in `project_open-defer-slots.md`** | ⚠ the SoT per `MEMORY.md`, and `grep -c 'inline-fragmented-fn' project_open-defer-slots.md` → **0**: without this row a brand-new `#11-` slot lands with no SoT entry and no record of the omission. Dates are each slot's own — **2026-10-28** for the source (from its memo), **2026-11-01** for the successor. This is the row an earlier drafting sent away with the umbrella while simultaneously keeping the close, per the ⚠ above |
| **Write the narrowing into `project_line-box-decorated-inline-content.md`** — the umbrella's SSoT | its "NEXT SESSION STARTS HERE" tells the next session the seam-3 branch carries the umbrella memo + tooling and that §10's five rows ship there. After this PR, four rows and both files are still owed and the trigger that would have discharged one has been consumed. Record: what left, why (the preamble's rule), that umbrella §8 is contradicted and is round 20's input, and that the plan-checker note's trigger must be **re-keyed to a state, not an event**, since its event has passed |
| Record the residue's size in the successor slot as its size-disjunct baseline | ⚠ **re-measure with `wc -l` at landing** rather than copying §5.5's 573, per §5.6 — every figure in this memo predates the committed implementation |
