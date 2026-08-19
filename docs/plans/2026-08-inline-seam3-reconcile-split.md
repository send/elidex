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

⚠ **That rule offers two routes and this memo takes the first; the second is weighed here rather
than passed over.** Route (i) is to route the contradiction to plan-review as a plan delta — taken.
Route (ii) is to **re-slice it out of the PR, when the defect pattern says the slice boundary is
wrong**. Rejected, on the pattern rather than on convenience: a re-slice would move **the boundary**,
and **no finding has landed on the moved range** — the byte-identical body, which is what a re-slice
would re-cut. It was ratified by the umbrella, is proven unchanged by §6's two harnesses, and has
not moved since the first commit.

⚠ **Findings have landed outside the memo** — the `reconcile_flows` docstring's citations and
`collect.rs`'s delegation comment — but both are text this PR **authored**, not the moved range.
The defects cluster at what this PR wrote, not at where it cut, which is a stronger ground for
keeping the boundary than a claim about the memo would be.

Recorded because "we took route (i)" reads as a settled rule when it is a choice between two, and
the reader cannot otherwise tell which.

**Coordinate frames.** Every `file:line` naming a *pre-split* location — `inline/mod.rs`,
`collect.rs`, the base range — is a **`658cc302`** coordinate, produced by a command named beside
it. References to `inline/reconcile.rs`, and to `inline/mod.rs`'s **residue** after the split, are
necessarily **post-split** coordinates: `reconcile.rs` does not exist at `658cc302` at all. Each is
labelled where it appears; an unlabelled coordinate is a base one.

⚠ **Review history lives in `project_seam3-pr508-review-history.md`, not here** — what each
revision got wrong, and why. The umbrella states the rule: *"a past-tense ledger restates the
normative decisions and then drifts from them"*. ⚠ It extends past narrative to **provenance**: a
claim is carried by the command that produces it, never by prose asserting it was checked. Where
this memo states a figure, a count, or a population, either the command is beside it or the claim
does not belong here.

* **`preflight.py` — run it, do not read a stored verdict from here.**

  ```
  python3 .claude/skills/elidex-plan-review/preflight.py \
      docs/plans/2026-08-inline-seam3-reconcile-split.md
  ```

  ⚠ **No counts are recorded in this bullet, for the same reason §5.5 records no `wc -l`**: the
  checker's output is a function of the memo, so *every edit to the memo can change it* — and edits
  did. ⚠ **One description, not two.** This bullet previously carried a causal explanation *and* a
  separate "stable shape" list saying different things about the same counters; they drifted apart
  and contradicted each other two bullets apart
  ([[feedback_duplicated-decision-surface-blocks-converge]]). What the run asserts:
  * **soft warnings only, no hard failures** — and the soft ones are **heterogeneous**, so read
    them, never infer a cause from the total. They currently include an `N entries` claim without a
    cached artifact **and** a `path … contains shell glob/brace syntax` warning emitted by a command
    §5.3.1 itself embeds. The total therefore tracks neither §3's rows nor the memo's `N entries`
    claims alone.
  * **unmapped-label rows** count **§3's table rows** whose label `SPEC_LABEL_REVERSE` lacks.
  * **the label warning is not noise, and the scope of that is exactly two labels.**
    `SPEC_LABEL_REVERSE` does not map **`CSS 2`** or **`css-writing-modes-4`** — this memo's two —
    so both §3 rows land in `unmapped-label rows` and the run reports `parsed citations: 0`, i.e.
    **the §3 citation gate is vacuous *for this memo***. Its warning count therefore rises with
    §3's row count and says nothing about §3's correctness. ⚠ **Not "no CSS-module label is
    mapped", and not a claim about other plan-memos**: `preflight.py:62` maps
    `"CSS Selectors L4": "selectors-4"`, so the map's CSS coverage is partial, not empty, and a
    memo citing only mapped labels would have a working gate.
  * What the vacuous gate costs *here* is stated rather than hidden: **both** of §3's citations were
    verified by hand, because the gate could not — `webref heading CSS2 10.8` and
    `webref heading css-writing-modes-4 6.4`. ⚠ "Both", not a count: §3's own K/M line is the count.
* **`plan-xcheck.py` — NOT RUNNABLE on this branch, and no verdict is recorded here.**
  `ls .claude/tools/plan-xcheck.py` → no such file: the checker lives on the umbrella's branch and
  travelled with the umbrella's tooling under the narrowing above (preamble), so a reader of *this*
  PR cannot re-derive any count from it. ⚠ A stored verdict was therefore removed — it was the
  same defect as a stored `wc -l`, one bullet away from the rule forbidding it, and it additionally
  predated three later commits to this memo.

  What survives is the *shape*, which the umbrella's branch can re-derive: the checker is written
  for a **multi-PR umbrella** (it harvests §6 as a per-PR cell matrix, cross-checks M-rows and a
  flip partition, and expects the umbrella's PR labels), so its `[PR]` / `[CELL]` / `[FLIP]`
  families are inapplicable to a single-PR memo by construction. Its **generic** checks are not,
  and were acted on while it was still reachable: the `K=`/`M=` breadth check and the per-PR
  own-deferral check (§3 and §5.3 answer both), and check 12, which correctly caught a superseded
  line range (`:411-637`) restated beside its corrected form in §5.2's quotation.
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

**Breadth**: K=2 specs, M=2 entries. **Split decision**: single PR, both below the
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

Spec-governed concerns it surfaces, none of them cited in the source. Every §-number↔title pair
below was resolved with `.claude/tools/webref` — **`heading` for the pairs, `dfn` where a
*property* or *term* had to be located from its name first**. Both emit the pair. ⚠ **Annotations are per *citation*, not per bullet, and three carry one.** `overflow`'s records a
real disambiguation (two hits); `text-align`'s and `column box`'s record a single-hit lookup and
disambiguate nothing. Every citation without an annotation — including the two inside an annotated
bullet — re-derives with `heading <module> <section>`:

* `:437-447` — IFC-local logical → absolute physical fold keyed on `is_vertical`
  → **css-writing-modes-4 §6.4** *Abstract-to-Physical Mappings*. ⚠ **This one leaves the list**:
  uncited *in the source*, but this PR cites it in the `reconcile_flows` docstring, so it is the
  table's authored row, not part of the out-of-scope complement below.
* `:470` — `text-align` already baked into `inline_start` → **css-text-3 §6.1** *Text Alignment:
  the `text-align` shorthand* (`webref dfn css-text-3 text-align` → **one** exact hit,
  `type=property`, `#propdef-text-align` — nothing to disambiguate here, unlike `overflow` below)
* `:557-569`, `:588-594` — relative/sticky offset preserved through reposition
  → **css-position-3 §3.3** *Relative Positioning* / **§3.4** *Sticky positioning*
* `:514` — the term "fragmentainer" → **css-break-4 §2** *Fragmentation Model and Terminology*
* `:483-492`, `:531-554` — a box continuing across **column boxes**
  → **css-multicol-1 §2** *The Multi-Column Model* (`webref dfn css-multicol-1 'column box'` → §2).
* `:419`, `:550-554`, `:506-510` — the abspos toggle; `overflow:hidden` clipping; the paged path
  and its page generation → **css-position-3 §2** *Choosing A Positioning Scheme: position
  property*; **css-overflow-3 §3.1** *Managing Overflow: the `overflow-x`, `overflow-y`, and
  `overflow` properties* (`webref dfn css-overflow-3 overflow` returns **two** hits — the
  `type=dfn` term at §2 and the `type=property` at §3.1; the clipping behaviour cited here is the
  property, so §3.1, not §2); **css-break-4 §2**.

**The rest are out of scope, by change class rather than by grep** (the first bullet excepted, per
its own ⚠): this PR authors no algorithm, so it neither creates nor deepens a *missing*-citation
defect. That is exactly the position #497 took when it declined to add a §9.4.2 module-doc
citation to `collect.rs`/`styled_run.rs` as over-claiming. ⚠ **It is not a defence against an
*incorrect* citation** — a different class, and the one that governs the two citations this PR
does author. §9 books the complement with an explicit disposition, not a pointer.

**So the table below is what the PR *carries*, not what the range's spec surface is** — and its
rows have two distinct provenances, which is the distinction the map exists to record:

⚠ **Two rows, three citation instances** — the row count and the instance count are different
numbers and conflating them misclassified a row:

* **CSS 2 §10.8** is **dual-provenance**. One instance *travels unchanged* — the comment inside
  the byte-identical body, moved and not authored. A **second is newly authored by this PR**, in
  the `reconcile_flows` docstring, and it is the one that spells out the full §number↔title pair:
  ```
  git grep -c "Line height calculations" 658cc302 -- crates    # → no output, rc=1 (zero hits)
  ```
  So "travels unchanged / not authored" describes only half of this row, and the authored half is
  where an over-claim can live — which is why the docstring states the gap
  **positively** — what §10.8/§10.8.1 machinery *is* implemented, and what is not — rather than
  scoping "unimplemented" to any one property.
* **css-writing-modes-4 §6.4** is *newly authored by this PR* — the fold it governs is likewise
  inside the untouched body, but the **citation** is text this PR writes, which is exactly why it
  has to appear here. ⚠ It is scoped in the docstring to §6.4's **axis assignment only**: the
  fold reads `is_vertical` and never `direction`, and applies no `vertical-rl` block-axis
  reversal, whereas §6.4's mapping is keyed on the used `writing-mode` *and* `direction` (its
  `block-start` row is `top`/`right`/`left`). An unscoped "follows §6.4" would assert conformance
  the body contradicts — the class #497 fixed in `154bac3f`.

⚠ **This paragraph states no row count** — the table below is the enumeration.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-writing-modes-4 §6.4 Abstract-to-Physical Mappings | the abstract→physical mapping | inline axis → physical x (horizontal) / y (vertical); block axis → the other | **authored by this PR** — the `reconcile_flows` docstring cites it for the IFC-local logical → absolute physical fold keyed on `is_vertical`. The fold itself is inside the byte-identical body and is untouched; the *citation* is new text, which is why it belongs in this map. Pair verified with `.claude/tools/webref heading css-writing-modes-4 6.4` | ✓ | yes |
| CSS 2 §10.8 Line height calculations: the `line-height` and `vertical-align` properties | `vertical-align` within the line box | not implemented — the atomic's block-axis reposition target is the line top (baseline-naive). ⚠ **Both** sinks, not the mid-break one: the citing comment sits under `if persist_flow {` (`:468`), and the target itself comes from `static_atomic_reposition_records`, whose docstring calls itself "the SINGLE derivation shared by both the `persist_flow` sink … and the `do_carrier` sink" (`:717-720`) and which returns `line.block_start` (`:734`) | ⚠ **dual**: the body comment at `:480-481` moves verbatim (no code touched), **and** this PR authors a second instance in the `reconcile_flows` docstring — the only one carrying the full §number↔title pair (`git grep -c "Line height calculations" 658cc302 -- crates` → zero hits). The authored instance states the gap **positively**: §10.8.1 leading/half-leading and the baseline derivation are implemented and cited elsewhere in the crate; `vertical-align` alignment, §10.8's strut, and its uppermost-to-lowermost line-box height are not. Title↔number pair verified with `.claude/tools/webref heading CSS2 10.8` | ✓ for citations carried; the uncited complement is §9's | yes |

## §4. Verified current state

⚠ **Each row's Result is a *reading* of the command's output** — a count, the line numbers that
matter, and a classification such as "call" or "comment". `git grep -n` emits commit-prefixed
whole source lines; the cells do not reproduce them. Run the command for the raw text — the
reading is a navigation aid, not a substitute for it.

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

> Its four helpers (…) already sit immediately below at `:648-783`, so this one has a
> **ready-made module boundary**.

(⚠ `:648-783` is that memo's own pre-#497 frame; on `658cc302` the helper span is `650-785`,
uniformly +2 — the same shift that takes its `:138`/`:139` to this base's `:140`/`:141`.)

The helpers are named as **adjacent evidence that the seam is clean** — the reason the extraction
will not tangle — not as members of it. The seam is sized "~227 lines" in that same entry, and
`637 − 411 + 1 = 227` is the range **alone** (range plus helpers is ~363), so the two documents
never disagreed about the seam's extent.

**The back-edge this leaves is the crate's established idiom, measured rather than asserted.**
`reconcile.rs` imports four helpers defined in its parent, which is the shape a reader is most
likely to challenge. The crate already runs it:

⚠ **The rows below are instances, not a census, and no instance count is stated.** The population
is whatever this returns, and a reader who wants it should run it rather than trust a list:

```
git grep -n 'use super::' 658cc302 -- 'crates/layout/elidex-layout-block/src/**/*.rs' | grep -v tests
```

| shape | instances on `658cc302` |
|---|---|
| child imports a fn defined in the parent | `positioned/layout.rs:20` (`use super::resolve_offset`, defined `positioned/mod.rs:46`); `block/children/{stack.rs:13, helpers.rs:14, shift.rs:8}` (`use super::super::is_block_level`, defined `block/mod.rs:46`); `block/children/stack.rs:16` (`use super::{make_block_break_token, …}`, defined `block/children/mod.rs:49`) |
| **bidirectional** parent↔child | `positioned/mod.rs:28` re-exports `layout::{…}` while `layout.rs` imports `super::resolve_offset`; `block/mod.rs:36` imports `children::shift_block_children` (the call is `:463`) while `children/*` import `super::super::is_block_level`; **`block/children/mod.rs:18`** (`pub use stack::stack_block_children`) against **`stack.rs:16`** — ⚠ the **closest analogue to this PR**, because its child→parent leg is a *direct* parent-defined-`fn` import, exactly like `reconcile.rs`'s `use super::{clear_inline_flows, …}`, whereas the other two route through a grandparent (`super::super::`) or a re-export |

So keeping all four beside each other is one uniform rule where any split would be a 2/2 — the
*opposite* of *one issue, one way* — and the arrangement instantiates an idiom the crate already
carries, not a novelty this PR introduces. 

⚠ **The correspondence §6 depends on**: since all four stay, the moved range is *exactly* what §6's
harness extracts — no `fn` sits outside both extracts. That is what makes the harness a proof of
the whole move rather than of part of it, and §8 carries it as a DoD clause.

⚠ Review history — including what was argued and withdrawn about moving two of the four — lives in
`project_seam3-pr508-review-history.md`. What survives here is the decision and its grounds. ⚠ §9
records that "all four stay" is **declined on scope**, not settled — a third configuration (all
four into a shared sibling imported by both) is neither weighed nor foreclosed here.

### §5.3 `#[allow(clippy::too_many_arguments)]` on the new function

**Own-deferral count: the seam-3 prereq opens ONE — `#11-inline-fragmented-fn-seams-1-2`.**
The successor slot is **created by this PR** (`grep -rl '#11-inline-fragmented-fn-seams-1-2'
<memory-dir>` returns only files this PR writes). Its contents split by **origin, not by count**.
**Pre-existing**: seams 1 and 2, named by the source slot on 2026-07-28.
**Created by this PR**: the moved body's probe universal losing its counterexamples to the residue
(§7.2 — the *text* is pre-existing, the *separation* is this PR's, and it cannot be repaired here
without breaking byte-identity); `reconcile_flows`' eleven-parameter signature and its
adjacent-`bool` window, which §9 itself calls new — **and the helper-home question**, which an
earlier classification filed as neither. ⚠ That was wrong: the question is "where should the four
helpers live once their principal caller is a **sibling module**", and at `658cc302` there is no
sibling module, so the question could not be asked. Its premise is brought into existence by this
PR; it is own. ⚠ **No own-concern count is stated here** — the bullet above *is* the enumeration,
and a count beside a list it does not contain is the restated-derived-value shape that drifted
elsewhere in this memo. **What the cap measures is slots, and this PR opens exactly one**:
`#11-inline-fragmented-fn-seams-1-2`, within the ≤3 per-PR own-deferral cap. Concern *count* is
not a cap input; concern *origin* is what §5.3 splits on, per its own "by origin, not by count".
§10 carries the slot's `(own)` row.

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
(`unoffset_origins`, `flow_lines`, `relpos_atomic_placements`) superficially resemble the shape,
which is the shape CLAUDE.md's *side-store→component 判定ルール* is written about. Not a claim they
should become components — they are intra-pass scratch, consumed before the pass ends, which is a
defensible reading of that rule's scope — but the ECS question must be *on* the slot, not absent
from it.

### §5.3.1 ECS-native check / OO → ECS mapping

`axes.md`'s Axis 2 `[plan]` detect entry asks a plan-memo to carry this subsection explicitly. The
check was run; only its *result* was missing, which is the thing a reader cannot reconstruct.

| question | answer |
|---|---|
| Does the extraction introduce an OO pattern (registry, observer, subscriber list, class-owned state)? | **No.** It adds one `pub(super) fn` and a `mod` declaration. No trait, no `Vec<Box<dyn …>>`, no `ObjectKind` variant, no new state container. |
| Does it move per-entity state into a side-store? | **No.** The three parameters (`unoffset_origins`, `flow_lines`, `relpos_atomic_placements`) are **pre-existing**, produced by `layout_atomic_items` and the packer; the split only makes them cross a function boundary. ⚠ Only two are entity-*keyed* (`HashMap<Entity, _>`); `relpos_atomic_placements: &[(Entity, f32, f32)]` is a flat slice, iterated in order and never looked up. The distinction is load-bearing because the rule's trigger text is written about `HashMap<entity, _>`. |
| Do they meet CLAUDE.md's *side-store→component* rule? | **Not applicable as a defect**, on two independent grounds. **Shape**: they are arguments threaded through one call chain, not an entity-keyed registry held beside the World, so the rule's subject is not what they are. **Lifetime**: all three are intra-pass scratch consumed before the pass ends. ⚠ The lifetime ground survives the `do_carrier` path, which is the one that looks like a counterexample — values from all three *are* copied into `ColumnFlowSlice`, but that carrier is itself drained inside the same pass, per its own authoritative docstring (`elidex-ecs/src/components/inline_flow.rs`): *"it lives only between the IFC layout (write) and the multicol fill (drain) **within one layout pass** (transport, not state)"*. The question is nonetheless **put on the successor slot** (§9) rather than answered silently, because a future reshaping should re-make the judgment rather than inherit it. |
| What ECS state does the moved code own? | Two components, and **this row is scoped to the split's two modules — it is NOT the workspace write-set.** Within them: **`InlineFlow`** — insert in `reconcile.rs`; removal via `remove_one::<InlineFlow>` inside `clear_inline_flows` (`mod.rs`), invoked from *both* modules (the residue's two early-return exits and the moved `!env.is_probe`-gated call). **`ColumnFlowSlice`** — insert-or-remove in `reconcile.rs`, plus two removals in the residue's early-return exits. Both write sets span the new module boundary, symmetrically. ⚠ **The workspace complement is non-empty and is not listed here** — run `git grep -n 'InlineFlow\|ColumnFlowSlice' -- 'crates/**/*.rs'` and classify the hits by hand. It reaches `elidex-layout-multicol` and `block/children/shift.rs` ([[feedback_universal-claims-need-the-complement-measured]]). ⚠ **Anchor the enumerator on the component NAME, not on the call syntax.** Call-shaped patterns (`insert_one(.*ColumnFlowSlice`, `remove_one::<ColumnFlowSlice>`, …) drop two real write sites here: `elidex-layout-multicol/src/lib.rs`'s path-qualified `remove_one::<elidex_ecs::ColumnFlowSlice>`, and `reconcile.rs`'s `insert_one` whose `ColumnFlowSlice { .. }` literal spans several lines. The name is invariant; the call syntax is not, so a syntax-anchored pattern is a filter that looks like an enumerator ([[feedback_writesite-audit-includes-struct-literal-ctors]], [[feedback_checks-must-not-be-defined-by-the-symptom-vocabulary]]). |
| Does anything outside the crate depend on this function's clear having run? | **Yes, and it is worth knowing before touching the persist/clear cycle.** `elidex-layout-multicol/src/lib.rs` re-inserts `InlineFlow` on the run-start after the IFC pass (`position_column_fragments`), and guards it with a `debug_assert!` that the run-start carries **no** `InlineFlow` at build time — *"cleared each column by `clear_inline_flows`"*. So the moved block's clear is a precondition of another crate's write. Nothing in this PR changes it (the code is byte-identical), but a future reshaping of the cycle that reads only the two modules above would not see the constraint. |
| Is `ColumnFlowSlice` itself a side-store→component candidate? | **No, and the question is category-confused.** `ColumnFlowSlice` **is already an ECS component**; there is no side-store to migrate *from*. Its docstring makes both halves explicit — *"so it **is** a component (per-entity, `Send + Sync`, not a per-VM identity handle — the side-store→component rule), **not** a side-store"* — and the carrier is drained within the pass, so "it outlives the pass" was false too. ⚠ A *different* and still-open question exists nearby — whether per-entity payloads about *other* entities belong on those entities rather than on the IFC parent — but that is an **ownership** question, not this rule, and asserting it under this rule's name would direct future work to dismantle an established ECS-native phase boundary. Not routed, because this PR has no ownership invariant to offer for it. |

### §5.4 `#[allow(clippy::too_many_lines)]` on the residue

§8 requires this re-evaluated, and the source slot says (`:37`) to drop it "if the residue no
longer needs it". **It still does.** Measured by deleting the attribute with the split applied:
`this function has too many lines (177/100)` (178 before §2.2's fold). It stays, and the measurement is recorded so the
next toucher re-runs it rather than re-litigating it.

### §5.5 Resulting sizes

```
wc -l crates/layout/elidex-layout-block/src/inline/{mod,reconcile}.rs
```

⚠ **`reconcile.rs`'s line count is structurally unstable and this memo states it nowhere.** It is
the one figure in this PR that its own later commits move, because every such commit adds prose to
that file; each stored copy has gone stale, and a stored copy is what §8's *"Figures are
referenced, not restated"* forbids. **The command above is the record.** Anything that needs the
value runs it; nothing stores it — not this section, not §5.6, not a commit message (§8), and not
the successor slot (§10, which re-measures at landing).

`mod.rs`'s **573** is stated because it is stable — it has held at every commit on this branch —
and the 1000-line argument turns on it. Both files sit below
[[feedback_touch-time-split-means-while-writing]]'s 700–800 band, and the residue is 212 lines
further from the 1000-line gate than it was — the source slot's *second* trigger disjunct, which
this PR moves away from firing rather than toward.

⚠ Which revisions moved the number, and why the memo restated it four times before stopping, is
review history: `project_seam3-pr508-review-history.md`, not here.

### §5.6 Provenance of §5.3–§5.5's figures

**Status: re-measured on the committed implementation.** Every figure below was first taken on a
throwaway extraction that was not in the branch, and §8 required each to be re-run against the
shipped tree before landing. Result of that re-run:

| figure | predicted | measured on the implementation |
|---|---|---|
| `too_many_arguments` load-bearing | yes | yes — `this function has too many arguments (11/7)` |
| `too_many_lines` still load-bearing on the residue | yes, `178/100` | yes, **`177/100`** (§2.2's fold) |
| `inline/mod.rs` | 573 | **573** |
| `reconcile.rs` | 254 | ⚠ **wrong, and structurally unstable — no value is recorded anywhere in this memo.** Run §5.5's `wc -l`; see §5.5 for why a stored copy is forbidden |
| §6 harness | 6 hunks, `226 == 226` | **6 hunks, `226 == 226`, PASS** |
| test baseline | 325 | **325 passed, 0 failed** |

The recipe the numbers come from, so a reader can re-derive rather than trust:

```
# 1. reconcile.rs = module doc + the `use` block + the AUTHORED `reconcile_flows`
#    docstring + `#[allow(clippy::too_many_arguments)]` + the §2 signature, then the
#    range's lines 413-419 and 421-639 with §2.3's four substitutions applied.
#    ⚠ The `use` block is what makes `wc -l` determinate, so it
#    is part of the recipe: std `HashMap`; `elidex_ecs::{ColumnFlowSlice, EcsDom, Entity,
#    InlineFlow}`; `elidex_plugin::Point`; and `super::{clear_inline_flows,
#    relpos_atomic_reposition_records, reposition_atomic_box,
#    static_atomic_reposition_records}`. `InlineFlowLine` needs none — the range already
#    writes it fully qualified (`:426`, `:458`).
#    ⚠ THE DOCSTRING IS THE LARGEST AUTHORED PART, and omitting it was this recipe's
#    defect for several revisions. NO RANK IS CLAIMED BEYOND THAT: it is larger than
#    the other authored parts (signature, `use` block, module doc), and *smaller* than
#    the moved range, which is 226 lines and not authored here. Its extent is a
#    measurement, not a stored figure -- it moves whenever a review finding edits it:
#      awk '/^\/\/\//{if(!s)s=NR;e=NR} END{print s"-"e}' \
#        crates/layout/elidex-layout-block/src/inline/reconcile.rs
#    ⚠ What is NOT derivable from the base is the docstring's *authored* content, and
#    that is narrower than "all of it". Per §3 for the citation and §7.2 for the probe universal, `CSS 2 §10.8` and that universal's
#    TEXT both already exist inside the moved range at 658cc302 (base `:480` and
#    `:521`); §3 records `CSS 2 §10.8` as a DUAL-PROVENANCE row for exactly this reason.
#    Authored here are: the number-title pair, the `css-writing-modes-4` citation, and
#    the SCOPING of the probe universal to this function -- not the citations wholesale.
#    Do not restate that split here; §3 is its site, and restating it is how this
#    sentence went wrong.
#    ⚠ This recipe reconstructs the file's ELEMENTS, and deliberately does not add up
#    to `wc -l`: the two blank separators and the function's closing `}` are structure,
#    not elements, and naming them would make this a line-accounting table instead of a
#    reconstruction. Measure the total; do not sum this list.
# 2. mod.rs = the same file with 413-639 replaced by the call alone, and
#    `mod reconcile;` added beside the sibling `mod` declarations. ⚠ `:420`
#    (`let first_baseline = packer.first_baseline;`) does NOT survive as a
#    binding: per §2.2 it is folded into `InlineLayoutResult { first_baseline:
#    packer.first_baseline, … }`. Retaining it verbatim reconstructs a residue
#    one line longer than the shipped one.
cargo clippy -p elidex-layout-block --all-features      # → clean, with both #[allow]s
cargo test   -p elidex-layout-block --all-features      # → 325 passed, 0 failed
wc -l crates/layout/elidex-layout-block/src/inline/{mod,reconcile}.rs
```

⚠ `reconcile.rs` has no final value: it moves with every commit that documents it, including
review-fix commits. The resolution is a **command**, not a corrected number — any number written
here is falsified by the act of writing it
([[feedback_document-landing-invalidates-its-own-measurements]]).

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

⚠ **The harness above covers the body, and the body is not the whole move** — a move is the
extracted text *plus the call that replaces it*, and the call is outside the compared region by
construction (the extract begins after the signature's `) {`). **The gap is not theoretical**: a transposition of the call's two adjacent `bool`
arguments was introduced into the working tree during review, and the body harness reported
`6 hunks, 226 == 226, PASS` while 52 tests failed and clippy and `cargo fmt --check` stayed clean.

### §6.1 The call-site half

Argument order must equal parameter order. Mechanically: parse `reconcile_flows`' parameter names
from its signature, parse the argument identifiers at `mod.rs`'s call, strip the `&` and `packer.`
the signature introduces, and compare pairwise.

**Pass condition**: the two name lists are equal. This is stronger than it looks, because the
extraction deliberately names every parameter after the binding it replaced — so the check reaches
all 11 positions, including the three `bool`s no type can distinguish.

⚠ **Mutation-verified, because a check that has never failed proves nothing.** Against the
preserved transposed tree it reports `MISMATCH at position 5: parameter persist_flow receives
do_carrier` and exits 1 — the exact defect the body harness passes.

⚠ **Neither harness ships in the repository, and that is the disposition, not an oversight.**
`git diff --name-only origin/main...HEAD` names four files (§8), none under `.claude/` or
`scripts/` — the preamble's narrowing keeps this PR to what its own change makes true, and a
one-shot verification procedure is not that. What makes it acceptable is that the recipes above
are **sufficient to reconstruct**: a reviewer independently rebuilt §6 from this prose and
reproduced `6 hunks, 226 == 226` exactly, without the original script. Standing them up is the
successor slot's call, not this PR's.

Together the two halves cover the move: §6 proves the extracted text is unchanged, §6.1 proves it
is invoked with the bindings it was extracted from. ⚠ Neither reaches a *semantic* change to the
residue around the call; that is what the test suite is for, and §8 requires it green.

⚠ The correspondence the harness needs — the moved range is exactly what the PR moves, no `fn`
outside both extracts — is established in §5.2 and carried as a DoD clause in §8. It is not an
assumption: if any later revision widens the move, the harness stops proving the whole of it.

⚠ **The harness's base is the pinned `658cc302`, NOT `origin/main`.** Step 1's line offsets are
`658cc302` coordinates; once `main` advances, `origin/main` addresses whatever then occupies
`413-419`/`421-639`, so the harness would either fail on unrelated later changes or — worse —
compare the moved function against the wrong source block and pass. The freshness check §0 runs
(`git diff --stat 154bac3f..658cc302 -- crates/layout/` → empty) is a *separate* obligation and
does not license floating the extraction base.

⚠ It also compares against the repository, **not against a diff file written earlier in the
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
  unspecified, and inside it the block both writes `InlineFlow` and pushes
  `carrier_groups` → `ColumnFlowSlice.flow_groups`, read back by
  `elidex-layout-multicol/src/fill.rs:235`. The ground that does hold: the **same map instance is
  moved** into the callee, not rebuilt from another source, so any given pass drains in exactly
  the order it would have. The concern is
  therefore **closed here**, not routed.
* **The `LayoutBox`/`BoxModel` reader allowlist** — no row owed (§4), because the moved range's
  five `LayoutBox` tokens are all in comments (and its `BoxModel` count is 0), and the wire strips
  comment lines *before* matching (`layout-box-reader-trip-wire.sh:125`). Asserted by running
  `mise run trip-wires`, not by this paragraph.

### §7.1 Comments the move makes less accurate — the whole class, measured

⚠ **Pinned to `658cc302`, not the working tree.** This is a *pre-change* inventory, and this PR
rewrites three of the five sites — run against the tree it returns **2** (the two deliberately
left), which cannot substantiate the table below. Same pinning rule as §6's harness.

```
python3 - <<'EOF'
import re, subprocess
files = subprocess.run(["git","ls-tree","-r","--name-only","658cc302","--",
                        "crates/layout/elidex-layout-block/src"],
                       capture_output=True, text=True, check=True).stdout.split()
for f in (x for x in files if x.endswith(".rs")):
    src = subprocess.run(["git","show",f"658cc302:{f}"],
                         capture_output=True, text=True, check=True).stdout
    flat = re.sub(r'\n\s*//[/!]?', '', src)
    for m in re.finditer(r'persist block|reconcile (comment )?in `layout_inline_context_fragmented`', flat):
        print(f, flat[m.start()-45:m.end()+35])
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

**The sites outside the range**, splitting on whether they name a *location* or the block as a
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

* `clear_inline_flows` takes no probe flag (`fn clear_inline_flows(dom, candidates, persisted)`),
  so "the **`is_probe`-gated** `clear_inline_flows`" cannot name the definition. It names the
  **call** guarded by `if !env.is_probe`, and that call travelled with the block, so it is in the
  same file and below the comment. Still true. ⚠ **Stated as an ordering, not as two line
  numbers**: `reconcile.rs`'s coordinates move with every commit that documents the file (§5.5),
  so a stored pair here goes stale exactly as the `wc -l` did. The relation is what the claim
  needs, and it is stable.
* "mirroring" asserts a shape analogy and makes no location claim at all.

### §7.2 One comment inside the range IS made less discoverable, and it cannot be fixed here

The moved body carries a universal about probe behaviour —

> a probe neither PUSHes (box store, #315/#318), SHIFTs (#318), CLEARs (R1), nor WRITEs persisted
> render state

— and the residue holds **two counterexamples to the CLEAR clause**: the early returns at
`inline/mod.rs:163` and `:203` (⚠ **post-split** coordinates — `:162`/`:201` on `658cc302`, the
frame §4 and §5.2 use) call `clear_inline_flows` **ungated by `is_probe`**, whereas the
moved call is `!env.is_probe`-gated.

**Behaviour is unchanged and not at risk**, which is why this is recorded rather than fixed: both
exits are gated on `items.is_empty()` / no-usable-font, inputs that do not depend on `is_probe`, so
a probe and the definitive pass reach them identically. What the split changes is **discoverability**
— the universal now sits in a file containing neither counterexample.

**The defect is repaired in this PR; only the comment's own text is not.** Byte-identity
constrains the text *below* the signature's `) {` — exactly what §6's harness extracts. The
module doc and the `reconcile_flows` docstring above it are authored by this PR and are free, so
the docstring now scopes the universal to this function and names the residue's two ungated
clears. What cannot happen here is editing the **body comment**, which would break the proof.

⚠ **What is routed, therefore, is narrower than "the repair"**: aligning the two residue clears with
the moved one (or rewording the body text) is the residual, and it goes to
`#11-inline-fragmented-fn-seams-1-2` — ⚠ with a **`reconcile.rs`-scoped trigger disjunct added to
that slot**, because both of its existing disjuncts are residue/`mod.rs`-scoped and neither reaches
a false universal sitting in the new file (and disjunct 1 is self-exempted for six of the umbrella's
seven PRs). A concern routed to a slot no trigger reaches is not booked, it is dropped
([[feedback_enumerated-exemptions-leave-the-next-class-authoritative]]).

Recorded because a byte-identity contract that costs *nothing* is a claim worth distrusting — but
the price turned out to be one comment's wording, not its discoverability, and the earlier framing
collected credit for honesty while overstating what the contract forbade.

## §8. Definition of done

* `crates/layout/elidex-layout-block/src/inline/reconcile.rs` (NEW) exists and holds the module
  doc, the `use` block §5.6 pins, the attribute, and `reconcile_flows` — and **no `fn` outside the
  one §6's harness extracts**, which is what makes the harness a proof of the whole move rather
  than of part of it. (The constraint is on `fn`s, not on "nothing else": the imports are
  mandatory, since the moved lines name `InlineFlow`, `ColumnFlowSlice`, `EcsDom`, `Entity`,
  `Point` and `HashMap` unqualified.)
* **Both halves of §6 pass** and their output is in the PR body — the body harness
  (`6 hunks, 226 == 226`) **and** the §6.1 call-site check. ⚠ The second is not optional:
  the first cannot see the call, and that gap was exercised for real during review.
* `cargo test -p elidex-layout-block --all-features` green with **no test touched** —
  `git diff --name-only origin/main...HEAD` names no file under `.../tests/` or `tests.rs`. ⚠ **The ground, not just the outcome**: the `#[cfg(test)] pub(crate) use` re-exports the test
  modules reach through are at `mod.rs:28-34`, entirely above the range, and the one `pub` item
  the block calls (`reposition_atomic_box`) stays — so no test path can change. Baseline: 325
  tests, per §5.6.
* ⚠ **`mise run ci` cannot pass on this branch, and this DoD does not claim it does.**
  `mise.toml:116` defines `ci` as depending on `deny`, and `deny` fails on an **upstream**
  advisory-db breakage (`duplicate advisory ID: RUSTSEC-2026-0244`) that is red on every branch
  including `main`, and that this change cannot affect. What is required instead, and what was run:
  `check` / `lint` / `test-all` / `doc` / `trip-wires` each individually green (`rc=0`, zero
  failure lines), which is every `ci` dependency except `deny` and the no-op `ci-sweep`. ⚠ On CI
  the `Licenses & Vulnerabilities` job is **skipped** by the path filter (this PR touches no
  `deny.toml` / `Cargo.toml` / `Cargo.lock` / `.github/workflows/**` — the filter's **whole**
  `config` predicate, `ci.yml:43-47`), so the **PR** gate is unaffected. ⚠ Not "local-only": a `push` to
  `main` bypasses the path filter unconditionally, so `deny` runs post-merge and is red there for
  the same upstream reason — pre-existing, not caused by this PR.
  Earlier rounds of this PR's review record reported "`mise run ci` green"; that was false and is
  corrected here — including `cargo doc` with `RUSTDOCFLAGS=-D warnings`
  (`relpos_atomic_reposition_records`'s intra-doc link to `[static_atomic_reposition_records]`
  (`mod.rs:745`) must still resolve — it does trivially, both stay).
* **Every figure §5.6 marks pending is re-measured on the committed tree**: the two `#[allow]`s'
  necessity (`too_many_arguments`; `too_many_lines` at the figure §5.4 records), both `wc -l`s,
  and §6's hunk count. ⚠ **Figures are referenced, not restated** — every duplicated measurement
  in this memo drifted at least once.
  * ⚠ **`reconcile.rs`'s `wc -l` is not stored in this memo, and must not be stored in the
    message that lands.** It is the one unstable figure (§5.5): every commit that documents that
    file moves it, *including the commits that fix review findings*, so any location that stores
    it is falsified by the next such commit. The obligation is to **run** §5.5's command, not to
    store its output ([[feedback_document-landing-invalidates-its-own-measurements]]).
  * ⚠ **The landing record is THREE artifacts, and this DoD governs all of them.** ⚠ The
    easiest to forget is the **PR description**, and it is the one most easily fixed. The three: **(1)** this memo; **(2)** the **PR description**, editable at any time
    with `gh pr edit --body-file` — check it with
    `gh pr view <n> --json body -q .body | grep -nE 'origin/main|[0-9]{3}'`; **(3)** the **squash
    commit message**. ⚠ **Per-commit bodies on this branch cannot be repaired** — amend is
    hook-denied — so they are historical, not authoritative, and GitHub's
    **default** squash message is their concatenation, so it can carry claims this memo has
    since **retracted**. ⚠ **The quantifier here is "at least one", and that is all the
    evidence supports.** The known instance: `24874f54`'s body asserts *"Every finding landed
    on the memo's bookkeeping"*, which `c3efc9b7` narrowed — the narrowing is recorded in the
    **preamble** (`:40-43`), not in §9. ⚠ **Do not upgrade this to "every retracted claim
    survives in the commit that made it".** That is false in both directions: a claim can be
    retracted in the *memo* without ever appearing in any commit body, and a body that quotes a
    retracted claim may be the **retracting** commit, which carries the correction with it.
    `git log --format='%h %B' 658cc302..HEAD` enumerates commit *bodies*, not retracted
    *claims*, so it cannot decide that universal — the population and the predicate do not match
    ([[feedback_universal-claims-need-the-complement-measured]]).
    ⚠ **This is deliberately not an enumeration.** A retraction is semantic, so no command can
    list which unamendable bodies now contradict the memo, and a hand-maintained list of them
    is a second decision surface that drifts — the one that stood here did, omitting
    `24874f54`'s claim ([[feedback_duplicated-decision-surface-blocks-converge]]). The rule is
    therefore categorical and needs no list, which is also why it survives the correction
    above: **accepting the default violates this DoD**, because the landing message is the
    composed text below and nothing else — regardless of what the default happens to contain.

    ⚠ **The message is written here, not promised.** "Authored fresh at merge" would be a promise
    about a future check, which §0's rule forbids — *a claim is carried by the command that produces
    it, never by prose asserting it was checked.* So merging is a paste, not a re-derivation:

    ```text
    refactor(layout): split the IFC flow reconcile out of inline/mod.rs (seam 3)

    Moves layout_inline_context_fragmented's 226-line flow-reconcile block from
    inline/mod.rs into a new inline/reconcile.rs as pub(super) fn reconcile_flows,
    byte-identical modulo the extracted signature.

    Proof (both halves required; see the memo's §6 / §6.1):
      * body: extract mod.rs 413-419 + 421-639 from the PINNED BASE 658cc302 --
        NOT origin/main, whose line numbers move -- and diff at n=0 against
        reconcile.rs's body. Pass = 6 single-line hunks, one per binding
        substitution, and 226 == 226.
      * call site: parse reconcile_flows' parameter names and the call's argument
        identifiers and compare pairwise. Pass = equal. Mutation-verified against a
        real transposition of TWO of the three adjacent bools, which the body
        harness passes.

    Stable figures: inline/mod.rs 785 -> 573; too_many_lines 177/100 and
    too_many_arguments 11/7 both still load-bearing; 325 tests pass.
    reconcile.rs's line count is deliberately NOT recorded -- it moves with every
    commit that documents the file. Measure it: wc -l.

    mise run ci cannot pass on any branch: it depends on deny, which fails to load
    the advisory DB (RUSTSEC-2026-0244, upstream, red on main too). check / lint /
    test-all / doc / trip-wires each run green individually. On CI the deny job is
    skipped by the path filter for this PR.
    ```
  * The other five figures are stable — `mod.rs` 573, `too_many_lines` 177/100,
    `too_many_arguments` 11/7, §6's `6 hunks / 226 == 226`, 325 tests — and have held at every
    commit on this branch, so recording them is safe and they go in the squash message at merge.
* **`git diff --name-only origin/main...HEAD` names four files** — this memo, `inline/mod.rs`,
  the new `inline/reconcile.rs`, and `inline/collect.rs` (one comment, §7). ⚠ **Nothing under
  `.claude/`, and no second `docs/plans/` file** — that is the mechanical statement of the
  narrowing in the preamble, and the cheapest way for a reviewer to confirm it.
* §10's ledger actions applied. ⚠ **Their targets are NOT in this repository**, so this line is
  not verifiable from the diff and must not be read as an in-repo obligation left undone — see
  §10's preamble.

## §9. Out of scope, with disposition

* **Seams 1 and 2** of `#11-inline-fragmented-fn-decomposition` — `mod.rs:266-303` (orphans/widows
  break computation) and `:388-411` (the packer-relative → layout coordinate fold), both
  re-measured on `658cc302`. The umbrella books them into the successor slot
  `#11-inline-fragmented-fn-seams-1-2` (§10).

    ⚠ **The trigger has FIRED for all three seams, today.** The source slot's trigger reads *"the
  next change that touches `layout_inline_context_fragmented`'s **body** (per CLAUDE.md
  touch-time discipline, **at which point the seam work is owed anyway**)"* — and this PR
  replaces 226 lines of that body with a call.

  **The ground is the ratified range, and nothing else.** ⚠ Two earlier grounds are withdrawn
  because neither yields the conclusion: CLAUDE.md's touch-time clause is scoped to **>1000-line
  files** (`inline/mod.rs` is **785** at `658cc302`, §0) and its *"split は単独 PR"* separates a
  split PR from a **feature** PR, not one seam from another — the banner split that created the
  source slot shipped **three** cohesion seams in one split PR
  ([[project_inline-mod-split-owed]]). And "folding three ranges forfeits the per-range proof" is
  false: §6's harness takes *(pinned base, line range, extracted fn)*, so three ranges are three
  runs against the same `658cc302`.

  What actually bounds this PR is that **the umbrella ratified `413-639` and only that**. Widening
  to seams 1 and 2 is an unratified scope change to a ratified surface
  ([[feedback_plan-ratified-surface-is-a-design-change]]), and the per-seam cohesion analysis the
  source slot calls for has not been done for them. ⚠ The slot is a *discharge route*, not an
  exemption — the trigger has fired and seams 1 and 2 are owed; only the vehicle is deferred.
* **The eleven-parameter signature.** Reducing it is a design change (§5.3) and belongs with the
  successor slot `#11-inline-fragmented-fn-seams-1-2`, whose subject is the residue's
  decomposition. ⚠ **Stated here rather than referenced**, because the slot itself lives in the
  user-level memory directory (§10) and a repository-only reader must still be able to tell when
  this work reopens. `#11-inline-fragmented-fn-seams-1-2`'s trigger, verbatim in substance:

  > **Either** the first change, after any of the decorated-inline umbrella's PRs, that touches
  > `layout_inline_context_fragmented`'s residue — self-exempted for **six** of the umbrella's
  > seven PRs on the grounds its §10 states, but ⚠ **not** for the seventh (the predicate prereq),
  > which the umbrella deliberately leaves un-exempted — **or** `inline/mod.rs` growing back
  > toward 1000 lines. **Re-eval 2026-11-01.**

  **The eleven-parameter question is reached by the first disjunct and needs no new one**: the
  eleven-argument *call site* lives in `layout_inline_context_fragmented`'s body (§5.1 — "calls it
  where the block was"), so every reshaping §5.3 names necessarily edits the residue. What §10 *does* carry is the slot's subject line naming §5.3's
  candidate shapes **including the side-store→component one**, so the question is not pre-answered
  as a grouping. ⚠ **Booked alongside it, because it is a different defect the count would hide**: the signature
  ends `is_vertical: bool, persist_flow: bool, do_carrier: bool` and the call site passes them
  positionally, so **any transposition of the three is type-correct and compiles silently**. That
  window is *new* — pre-split these were three named `let` bindings in scope (`:173`, `:322`,
  `:343`). Measured across all non-test `crates/layout` source, only four functions have two or
  more adjacent `bool` parameters, and `reconcile_flows` is the **only one in
  `elidex-layout-block`**; the crate's other wide signatures separate them, apparently
  deliberately — `layout_atomic_items` (`atomic.rs:26`) puts `layout_generation: u32` between
  `is_vertical` and `is_probe`. A slot told only "eleven is too many" may answer with a grouping
  that keeps the triple adjacent, so the adjacency is named on the slot, not just the count. The real ground is altitude — the fix for a three-`bool` positional window is a **type** (an enum
  or a flags struct, so a transposition fails to compile); shuffling an unrelated parameter between
  them to defeat ordering is a bandaid that leaves the hazard's shape intact.

  ⚠ **Why the type cannot be introduced here.** Byte-identity governs the body after `) {`; the
  signature is **authored**, so that contract does not govern it. The obstruction is one level
  in: the flags are **consumed by the arms** `if persist_flow { … } else if do_carrier { … }`,
  which *are* inside the compared body, so a `FlowSink` enum rewrites them and breaks the proof.
  ⚠ Half the hazard **is** closed here, so do
  not re-derive it at the slot: §6.1's call-site check compares argument names to parameter names
  pairwise and is mutation-verified against a real transposition of this very triple.
* **Where the four helpers should live once their principal caller is a sibling module.** §5.2
  keeps all four in `mod.rs`, and two of its four reasons are *design* reasons (`pub` API;
  residue callers) while two are *scope* reasons (outside the ratified range). ⚠ A third
  configuration exists that §5.2 does not weigh — all four into a shared sibling imported by both
  `mod.rs` and `reconcile.rs`, which satisfies the uniformity argument **and** removes the
  child→parent back-edge. Declining it here is right (it is outside the range), but leaving it
  unrouted would let the next reader take §5.2 as "settled" rather than "declined on scope".
  **Routed to `#11-inline-fragmented-fn-seams-1-2`**, whose entry carries it.
* **The uncited spec-governed concerns §3's complement command surfaces** — `text-align` baked
  into `inline_start`; relative/sticky offset preservation; fragmentainer terminology; column-box
  continuation; the abspos toggle, `overflow:hidden` clipping and the paged path. §3 states that
  §9 books this class, so here it is booked, with an explicit disposition rather than a pointer:
  **accepted as pre-existing and deliberately not cited by this PR.** The ground is change class —
  this PR authors no algorithm, so it neither creates nor deepens a missing-citation defect, which
  is the position #497 took when it declined to add a §9.4.2 module-doc citation to
  `collect.rs`/`styled_run.rs` as over-claiming. ⚠ **Not routed to a slot, and that is the
  disposition, not an omission**: the class is a property of the *residue's* algorithm, not of the
  move, so it reopens when the algorithm is next authored — not on a date. ⚠ It is also **not** a
  defence against an *incorrect* citation, which is a different class and is why the two citations
  this PR does author are scoped in the `reconcile_flows` docstring rather than asserted flat.
* **The CSS 2 §10.8 `vertical-align` deferral** that §3's CSS 2 row records — likewise
  pre-existing, and owned by the umbrella itself (its §5.3 books the line-box height/baseline work
  under `#11-inline-root-inline-box`). Recorded here so the row is dispositioned rather than
  merely observed.
* **Cold gate** ([[feedback_split-on-touch-prereq-workflow]]), re-run on `658cc302` at this PR's
  own touch set (`crates/layout/elidex-layout-block/` plus the four `.claude/`+`docs/` files of
  `f63eb623`):
  * Open PRs: `gh pr diff <n> --name-only` for each co-open PR — the set is whatever
    `gh pr list --state open` returns at the time of the run, **not a list stored here**, since a
    stored enumeration goes stale as PRs open and close (it did: it named #503, since closed, and
    predated #507). At the latest run: 381, 501, 502, 505, 506, 507 → none touches
    `crates/layout/elidex-layout-block/`, and `comm -12` against this branch's name set is empty
    for every one. #507 is a 38-crate dependabot bump touching `Cargo.toml`/`Cargo.lock` only;
    disposition unchanged. ⚠ **File-disjointness is not the whole gate for #501**: it
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
      deletions(-)` (verified 2026-08-16). Recorded because the class recurs
      ([[feedback_universal-claims-need-the-complement-measured]]): a "none besides X" claim is a
      claim about the complement, and only running the loop measures it.
  * ⚠ The gate is re-run **before merge**, not trusted from here — `main` moves, and PRs open and
    close. "At push time" is spent: this branch is pushed and #508 is open, so the next binding
    run is the pre-merge one.

## §10. Slot ledger actions at landing

⚠ **Where these land, and why the diff cannot show them.** Every target below is a file in the
user-level agent memory directory (`~/.claude/projects/<repo-key>/memory/`), **outside this
repository** — `git ls-files | grep -c memory/` → **0**. A reader checking §8's "§10's ledger
actions applied" against the diff finds nothing and could reasonably conclude the actions were
skipped. The diff cannot show them either way. The single table below is the record, and a row
the landing still owes says so.

⚠ **One table, no count stated** — a count beside a list is a restated derived value.
⚠ **A row the landing still owes opens its second cell with `still owed`**, so which rows are
outstanding is a grep, not an inference from verb mood.

⚠ **A file may appear on both sides; a *clause* may not.**

The split is **by clause, not by file**:
this PR writes only what its own landing makes true, and the umbrella's framing of the same file
leaves with the umbrella. So "named above" never means "outstanding", and "named below" never
means "wholly rewritten" — read each side for *which clause* it claims. Booking a whole file on
both sides ships it as neither; naming a whole file on one side contradicts the other.

⚠ **Loop-lifecycle rows are RETIRED, not rewritten**, and they are rows in the same table — the
record stays single. Their subject is this PR *while in flight*, so the landing removes them
rather than correcting them. ⚠ **Retire is not unconditional delete**: `#retire-after-migrate`
below marks a target that carries content no other file holds.

**Leaving** — all keyed to the umbrella program, none to this move: registering the umbrella's own
slot in `project_open-defer-slots.md` — ⚠ **and only its own**, which is the distinction that
keeps this row from swallowing the one below it: the *source* slot's registration stays here
(this move partially closes it), while the *umbrella's* slot leaves. Booked on both sides it
would have shipped as neither. The umbrella keeps
`#11-css2-spec-label-normalisation` (2026-10-31) and its own slot (2026-11-01);
correcting `#11-layoutbox-trip-wire-not-in-ci`, whose premise **#496 (`da958ace`) falsified**, not
this PR; rewriting the *umbrella's framing* in `project_line-box-decorated-inline-content.md`,
`MEMORY.md`'s Layout-lane entry and `active-lane-detail.md`; and shipping the plan-checker
standing note, which travels with the tooling. Each is real and none is dropped — they land with
the umbrella, and this narrowing is an input to its owed round 20.

⚠ **Leaving is not the same as delivered, and one row needs a mechanism rather than a sentence.**
The umbrella's next session reads `project_line-box-decorated-inline-content.md`, whose
"NEXT SESSION STARTS HERE" still instructed it to carry the memo and tooling on the seam-3 branch.
So this memo's "an input to its owed round 20" is inert unless written there. **§10 therefore
carries a row that writes the narrowing into the umbrella's own SSoT**, below.

⚠ **The plan-checker trigger is NOT consumed by this PR and stays ARMED.** It reads *"the first
PR that **ships** these two files"* — a **predicate over PRs**, not a calendar slot. This PR
ships neither (§8), so it is not the triggering event; the trigger waits for whichever PR does.
Re-keying it to an unspecified "state" would **disarm** a live obligation (the
`SPEC_LABEL_REVERSE` CSS-label extension).

**Applied at landing** — each row states what *this move* makes true:

| target (outside the repo) | what the landing writes, and what makes it this PR's |
|---|---|
| `project_inline-fragmented-fn-decomposition.md` | status → **PARTIALLY CLOSED**, seam 3 ✅ discharged — this PR discharges seam 3 and only seam 3. Plus the facts this PR falsifies — ⚠ **not claimed exhaustive**, since the sweep of an untracked file cannot be re-run from here; the ones found and fixed are: the front-matter ("508 lines … three concrete seams"); its own frame ("`mod.rs:139-646` = **508 lines**", `#[allow]` at `:138`, "After the split `mod.rs` is **783 lines**" — ⚠ those are *its* pre-#497 coordinates, which on `658cc302` read `:141-648` / `:140` / 785); the line listing seam 3 as open; the row reading "On landing, drop the `#[allow(clippy::too_many_lines)]` if the residue no longer needs it" (**re-evaluated and kept**; ⚠ take the figure from §5.4, not from that row); and the 783 band argument. ⚠ Sweeping only that last row would leave the memo asserting a size, a line range and an open seam this PR closes — the *statements* surface left standing while the *obligation* surface was fixed ([[feedback_sweep-obligations-not-only-statements]]) |
| `project_inline-fragmented-fn-seams-1-2.md` | **created** — seams 1 and 2 (pre-existing, §9), the eleven-parameter signature and the adjacent-`bool` window (both created by this PR), the helper-home question (§9), and **§7.2's probe-comment repair** (the moved body asserts a probe universal whose two counterexamples now live in the residue; the *discoverability* half is repaired in this PR via the `reconcile_flows` docstring; what this slot receives is the **body comment's wording**, which byte-identity forbids editing here). ⚠ **The slot's trigger is NOT verbatim from the umbrella's §10 row** — self-exemptions and the predicate-prereq un-exemption are, but a **third disjunct was added: the next change that touches `inline/reconcile.rs`**. Both inherited disjuncts are scoped to the residue / `inline/mod.rs`, so neither could ever reach a probe universal living in the new file. Re-eval **2026-11-01**. Its subject line names §5.3's candidate shapes **including the side-store→component one**, so the question is not pre-answered as a grouping — and §9 records that the slot's *existing* disjunct 1 already reaches the signature, because the call site is in the residue. Its size-disjunct baseline is the residue's `wc -l`, **re-measured at landing** rather than copied from §5.5 |
| `project_open-defer-slots.md` (the slot SoT) | the source slot registered as partially closed and the successor slot registered `(own)`. ⚠ The ground, anchored to the base rather than to now: `git grep -c 'inline-fragmented-fn' 658cc302` over the memory dir is not runnable (the dir is untracked), so the check is `grep -c` on the file **before this PR's own UPDATE block** — which returned 0. Running it after the block lands returns non-zero *because of this row*, so the post-landing value is not evidence ([[feedback_document-landing-invalidates-its-own-measurements]]). Dates are each slot's own — **2026-10-28** for the source, **2026-11-01** for the successor |
| the **stale `508` figure** | `still owed` — `layout_inline_context_fragmented` is **295** lines after the move (`mod.rs:142-436`), not 508. Sites: `grep -rnE -e '508[ -]lines?' -e '508 *行' <memory-dir> --include='*.md'` (two `-e` patterns, because a `|` inside this table cell must be escaped and `\|` is a *literal* pipe in ERE) → `active-lane-detail.md:149`, `project_inline-mod-split-owed.md:51`, `project_layoutbox-trip-wire-in-ci-next.md:71`, and `project_inline-fragmented-fn-decomposition.md` (`:11` title, `:28` under its own pre-seam-3 banner, so `:28` needs no edit). ⚠ **A grep keyed on this PR (`#508`, `layout-inline-seam3`) finds none of them** — they name the figure, not the PR. ⚠ **`project_layoutbox-trip-wire-in-ci-next.md:71` also says "trigger not yet fired"** — this PR **is** that trigger (`project_inline-fragmented-fn-decomposition.md`: "the next change that touches `layout_inline_context_fragmented`'s body"), so that clause is falsified independently of the figure |
| `project_inline-mod-split-owed.md` | `:82`'s "leaving `mod.rs` at **783**" corrected — a sibling site of the same class in a different file, reached by no row above. ⚠ `:51`'s `508` is a **second** site *inside this same file*, reached only by the `508` row above: swept per-file is still swept partially. A class swept per-file is a class swept partially ([[feedback_semantic-sibling-selfseed-and-regate-breadth]]) |
| `project_line-box-decorated-inline-content.md` (umbrella SSoT) | the narrowing written in. Its "NEXT SESSION STARTS HERE" told the next session that this branch carries the umbrella memo + tooling and that §10's rows ship here. Recorded: what left, why (the preamble's rule), and that umbrella §8 is contradicted and is round 20's input. ⚠ **The plan-checker note's trigger is left ARMED and unmodified** — it reads "the first PR that ships these two files", this PR ships neither, so this PR is not that event and nothing was consumed. Re-keying it to a vaguer "state" would disarm a live obligation. ⚠ **Additive is not sufficient** — a new block that records the narrowing while the original instruction stands leaves both live and the narrowing inert; the superseded passage must be struck, not merely followed |
| `MEMORY.md` | **That one clause only**: the Layout-lane entry no longer directs the next session to produce this PR — the one bookkeeping fact the *landing itself* makes true. ⚠ The entry's remaining content is the **umbrella's framing** of the lane and stays with the umbrella (see the "Leaving" paragraph, which scopes its half the same way). The entry is split by *clause*, so exactly one side owns each half and neither can read the other's as outstanding |
| `MEMORY.md` — the `🟡 IN FLIGHT: PR #508` bullet (**retired**) | `still owed` — a *separate* bullet from the Layout-lane entry. ⚠ **Retire the loop-state clauses only** — head sha, dry-streak, paused-round note. Clauses that outlive the loop stay, and this memo asserts two of them elsewhere: the harnesses are not in the repo (§6.1) and `mise run ci` cannot pass (§8). Same treatment for the Layout-lane entry's `merge 未` / `converge loop 継続中` clause |
| `project_pr508-converge-in-flight.md` (**`#retire-after-migrate`**) | `still owed` — 🔴 **Do NOT delete outright — it is the sole home of both harnesses.** Its own harness section carries their full text, and no executable copy exists in this repository — the plan memo quotes the harnesses' output strings in prose but ships no runnable file (§6.1: *"Neither harness ships in the repository, and that is the disposition"*, handing "standing them up" to the successor slot). **Migrate that section to `project_inline-fragmented-fn-seams-1-2.md` first**, then retire the loop state. ⚠ **What dies with it**, and moves in the same edit: its harness section (above), and its pointers to `project_seam3-pr508-review-history.md`. Retiring this file and `MEMORY.md`'s `IN FLIGHT` bullet together leaves that memo with **no inbound edge inside the memory dir** — it stays reachable from this plan (the preamble, §5.2 and §5.5), but a successor navigating memory alone would not find it. **Add the pointer to `project_inline-fragmented-fn-seams-1-2.md`**, which today has none |
