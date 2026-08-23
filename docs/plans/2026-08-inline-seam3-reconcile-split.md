# Seam-3 prereq split: the IFC's flow reconcile leaves `inline/mod.rs`

Prereq PR of the `#11-line-box-decorated-inline-content` umbrella
(`docs/plans/2026-08-line-box-decorated-inline-content.md`, on branch `layout-decorated-inline`,
frozen at rev 24 and **not** approved for implementation). The umbrella's §8 fixes this PR's
contract and §9 its scope; this memo re-anchors both against **`658cc302`**, the base this PR
actually has.

It also discharges `#11-inline-fragmented-fn-decomposition`'s **seam 3**: that slot's trigger
fires on this PR, and the slot memo is canonical for its wording.

⚠ **This PR carries the code move and nothing else, and that is a narrowing of what the umbrella
§8/§10 assigned it** (user-ratified 2026-08-16). The umbrella routes three further payloads here:
its own plan-memo, its two plan-checker tools, and five slot-ledger rows. All three are *umbrella
program bookkeeping*, and this PR's whole reason to exist ahead of the umbrella's approval is that
**it is independent of that approval** — so a PR justified as approval-independent must not
register the program's slots or rewrite its memory files. The rule applied, which is the umbrella's
own citation rule with "citation" replaced by "ledger row":

> a PR carries the bookkeeping **its own change makes true**, and hands the program's bookkeeping
> to the program.

So the slot ledger keeps only what this move makes true (the source slot's partial close, the `#[allow]`
re-evaluation, that slot memo's falsified figures, the successor's size baseline). The umbrella's
memo, tooling and remaining ledger rows travel with the umbrella. ⚠ This contradicts the
umbrella's §8 sentence "**This memo and the branch's tooling files … ship with the seam-3 prereq
PR** … No later PR re-ships them" and its §5.3 "the seam-3 prereq opens none" (it opens one), both
ratified surfaces — they are inputs to the umbrella's owed round 20, not a decision this memo may take silently
([[feedback_plan-ratified-surface-is-a-design-change]]).

**Coordinate frames.** Every `file:line` naming a *pre-split* location — `inline/mod.rs`,
`collect.rs`, the base range — is a **`658cc302`** coordinate, produced by a command named beside
it. References to `inline/reconcile.rs`, and to `inline/mod.rs`'s **residue** after the split, are
necessarily **post-split** coordinates: `reconcile.rs` does not exist at `658cc302` at all. Each is
labelled where it appears; an unlabelled coordinate is a base one.

* **`preflight.py` — run it, do not read a stored verdict from here.**

  ```
  python3 .claude/skills/elidex-plan-review/preflight.py \
      docs/plans/2026-08-inline-seam3-reconcile-split.md
  ```

* **`plan-xcheck.py` — NOT RUNNABLE on this branch.**
  `ls .claude/tools/plan-xcheck.py` → no such file: the checker lives on the umbrella's branch and
  travelled with the umbrella's tooling under the narrowing above (preamble).
  The checker is written
  for a **multi-PR umbrella** (it harvests §6 as a per-PR cell matrix, cross-checks M-rows and a
  flip partition, and expects the umbrella's PR labels), so its `[PR]` / `[CELL]` / `[FLIP]`
  families are inapplicable to a single-PR memo by construction.

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

⚠ **It does not survive in the residue either.** The hoist existed *because* 226 lines of
`packer`-consuming code followed it; with those lines gone it became a single-use binding sitting
18 lines above its only reader, directly above a `&mut dom` call — reading as a deliberate
pre-call snapshot when the ordering is incidental. It is folded into its sole consumer:

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

**Breadth**: `preflight.py` computes K and M from the table's
rows, so run it. The `css-inline-3` rows below are authored by this PR
(`git grep -c css-inline-3 658cc302 -- crates` → no hits). Anchors such as `#propdef-line-height` are cited without a row by design. **Split decision**: single PR, both below the
K≥4 / M≥20 recommend threshold, and `preflight.py` independently returns
`split decision: ok (single PR scope)`.

⚠ **The row set is a judgment about the change class, and the grep only bounds it from below.**
The moved range's existing citations are

```
git show 658cc302:crates/layout/elidex-layout-block/src/inline/mod.rs \
  | awk 'NR>=413 && NR<=639' | grep -nE '§|CSS |UAX|spec'      # → 1 line, offset 68 = :480
```

but that pattern can only match prose **that already carries a citation**, so its output is not a
measurement of the spec surface. **The complement is non-empty**, and this is the command that shows it:

```
git show 658cc302:crates/layout/elidex-layout-block/src/inline/mod.rs | awk 'NR>=413 && NR<=639' \
  | grep -inE 'logical|physical|text-align|sticky|fragmentainer|overflow|abspos|paged|margin-box'
```

Spec-governed concerns in the range, none cited in the source (a judgment; the command above only bounds the set from below). Every §-number↔title pair
below was resolved with `.claude/tools/webref` — **`heading` for the pairs, `dfn` where a
*property* or *term* had to be located from its name first**. Both emit the pair. Every citation without an annotation re-derives with `heading <module> <section>`:

* `:437-447` — IFC-local logical → absolute physical fold keyed on `is_vertical`
  → **css-writing-modes-4 §6.4** *Abstract-to-Physical Mappings*. Uncited *in the source*, but this PR cites it in the `reconcile_flows` docstring, so it is the
  table's authored row, not part of the out-of-scope complement below.
* `:470` — `text-align` already baked into `inline_start` → **css-text-3 §6.1** *Text Alignment:
  the `text-align` shorthand* (`webref dfn css-text-3 text-align` → **one** exact hit,
  `type=property`, `#propdef-text-align`)
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

**The rest are out of scope, by change class rather than by grep** (the first bullet excepted): this PR authors no algorithm, so it neither creates nor deepens a *missing*-citation
defect. That is exactly the position #497 took when it **withdrew** the module-doc citations it had
added to `collect.rs` (CSS 2 §9.2 *Controlling box generation* — the **parent**, cited "rather
than any one child precisely because which child applies is decided per arm") and `styled_run.rs`
(CSS 2 §9.2.2 *Inline-level elements and inline boxes*) as over-claiming. ⚠ **Read from the landed commits**
(`45c72c0a`, and the files at `154bac3f`), **not from #497's PR body**, which records the
additions and not the withdrawal that superseded them. ⚠ **`45c72c0a` is not reachable from
`origin/main`** — its branch is gone from the remote, so `git show` fails in a fresh clone; the
durable route is `refs/pull/497/head`. ⚠ **It is not a defence against an
*incorrect* citation** — a different class, and the one that governs the citations this PR
does author. §9 dispositions the complement: it states why the class is out of scope for this PR and names the slot that owns it.

**So the table below is what the PR *carries*, not what the range's spec surface is** — and its
rows have two distinct provenances, which is the distinction the map exists to record:

* **CSS 2 §10.8** is **dual-provenance**. One instance *travels unchanged* — the comment inside
  the body that is byte-identical modulo the extracted signature, moved and not authored. A second is authored by this PR in the
  `reconcile_flows` docstring — ⚠ **as a bare §-number**: the docstring carries no CSS 2
  §number↔title pair, and the pair for CSS 2 lives in **this row**, not in the `css-inline-3` rows.
  Measured:
  ```
  git grep -ci "line height calculations" 658cc302 -- crates  # → one hit, and it carries the
                                                             # title's leading clause only, not
                                                             # the full pair.
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

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-writing-modes-4 §6.4 Abstract-to-Physical Mappings | the abstract→physical mapping | inline axis → physical x (horizontal) / y (vertical); block axis → the other | **authored by this PR** — the `reconcile_flows` docstring cites it for the IFC-local logical → absolute physical fold keyed on `is_vertical`. The fold itself is inside the byte-identical (modulo the signature's bindings) body and is untouched; the *citation* is new text, which is why it belongs in this map. Pair verified with `.claude/tools/webref heading css-writing-modes-4 6.4` | ✓ | yes |
| css-inline-3 §1.1 Module Interactions | the supersession ground | `css-inline-3` *"replaces and extends the CSS inline layout model and features defined in [CSS2] section 10.8"* | **authored by this PR** in the `reconcile_flows` docstring, as the citation that licenses anchoring on `css-inline-3` rather than CSS 2 §10.8. Pair verified with `.claude/tools/webref heading css-inline-3 1.1` | ✓ | no |
| css-inline-3 §2.2 Layout Within Line Boxes | the line-box sizing step list | step 2 *"Content Size Contribution Calculation"* = the per-box block contribution this crate does compute (`line-height` for horizontal text, the margin box for atomics); step 3 *"Line Box Sizing"* = the aggregation it substitutes with a max | **authored by this PR** in the `reconcile_flows` docstring, as the current statement of CSS 2 §10.8's step 1 / step 3. Pair verified with `.claude/tools/webref heading css-inline-3 2.2`; step text with `.claude/tools/webref body css-inline-3 line-layout` | ✓ | yes |
| css-inline-3 §4.2 Transverse Box Alignment: the vertical-align property | `vertical-align` within the line box | not implemented — the atomic's block-axis reposition target is the line top | **authored by this PR** in the `reconcile_flows` docstring, as the current anchor for the gap CSS 2 §10.8 states in superseded form (`css-inline-3` §1.1 *"replaces and extends … [CSS2] section 10.8"*). Pair verified with `.claude/tools/webref heading css-inline-3 4.2` | ✓ | yes |
| css-inline-3 §5.3 Calculating the Logical Height Contributions ("Layout Bounds") of Inline Boxes | the half-leading derivation | run-level from one resolved font; §5.3's *normal* branch (the default) wants every glyph's A and D | **authored by this PR** in the same docstring, as §10.8.1's current statement. Pair verified with `.claude/tools/webref heading css-inline-3 5.3` | ✓ | yes |
| CSS 2 §10.8 Line height calculations: the `line-height` and `vertical-align` properties | `vertical-align` within the line box | not implemented — the atomic's block-axis reposition target is the line top (baseline-naive). ⚠ **Both** sinks, not the mid-break one: the citing comment sits under `if persist_flow {` (`:468`), and the target itself comes from `static_atomic_reposition_records`, whose docstring calls itself "the SINGLE derivation shared by both the `persist_flow` sink … and the `do_carrier` sink" (`:717-720`) and which returns `line.block_start` (`:734`) | ⚠ **the body comment at `:480-481` moves verbatim (no code touched); the docstring's own §10.8 references are now bare numbers; the CSS 2 §number↔title pair lives in **this row**, not in the `css-inline-3` rows (those carry the `css-inline-3` pairs, which the docstring also spells out in full)** (⚠ **the enumerator must be case-insensitive** — `inline_flow.rs:100` spells it lowercase:
`git grep -ci "line height calculations" 658cc302 -- crates`
→ one hit, `elidex-ecs/src/components/inline_flow.rs`, carrying the title's leading clause beside the
number but not the full pair as spelled out here; the site is lowercase, so it is the Title-cased form that returns empty,
and a pattern spanning the number and the title would be a filter, not an enumerator). The authored instance states the gap **positively**, and the positive clause is scoped: §10.8.1 half-leading is *approximated* in the first-baseline derivation only, at **run level from a single resolved font** — `pack/mod.rs` takes `em_height` from `measure_text` and `line_height` from the computed style
(resolved at `inline/styled_run.rs:96`); `measure_text` resolves one `font_id` (`elidex-shaping/src/measurement.rs:54` resolves the single `font_id`; its doc line *"using the first matching font family"* is at `:40`), whereas §10.8.1 is stated **per glyph** (`webref body CSS2 line-height`: *"for each glyph, determine the A and D … glyphs in a single element may come from different fonts"*). Graded against §10.8.1 (per glyph) a mixed-font box is outside it; graded against
`css-inline-3 §5.3` the *not-normal* branch prescribes exactly this single-font derivation and the
*normal* branch — the initial, inherited value whose keyword reaches computed style — is where the
divergence lives. What stays leading-naive is the baseline *within* the line box on the horizontal path, recorded in two other crates — `elidex_ecs::InlineFlowLine`'s `block_size` field doc and `elidex-render`'s `builder/inline.rs`; `vertical-align` alignment (`css-inline-3` §4.2), `css-inline-3` §5.3's strut, and `css-inline-3` §2.2 step 3's line-box sizing (CSS 2 §10.8 step 3's uppermost-to-lowermost height) are not. Title↔number pair verified with `.claude/tools/webref heading CSS2 10.8` | ✓ for citations carried; the uncited complement is §9's | yes |

## §4. Verified current state

| claim | command | result |
|---|---|---|
| the range is unchanged since the umbrella measured it | `git diff --stat 154bac3f..658cc302 -- crates/layout/` | empty |
| `inline/mod.rs` size before | `git show 658cc302:crates/layout/elidex-layout-block/src/inline/mod.rs \| wc -l` | 785 |
| the two record-producing helpers are called only from inside `413-639` | `git grep -n 'static_atomic_reposition_records' 658cc302 -- crates` | four lines: `:494`, `:544` (calls, both inside the range), `:721` (the **definition**), `:745` (an intra-doc link in the sibling helper's docstring) |
| — same, relpos | `git grep -n 'relpos_atomic_reposition_records' 658cc302 -- crates` | two lines: `:570` (call, inside), `:747` (definition) |
| `clear_inline_flows` has residue callers | `git grep -n 'clear_inline_flows' 658cc302 -- crates` | 13 lines. `inline/mod.rs`: calls `:162`, `:201` (the two early returns) and `:638` (inside the range); definition `:775`; **comments `:157`, `:549`, `:597`** — the last two inside the range (see §7). Elsewhere: `shift.rs:129`, `inline_flow.rs:219`, multicol `lib.rs:500`/`:625`/`:635`, `tests/mid_break_flow.rs:603`, all comments |
| `reposition_atomic_box` is public API, called only from inside the range | `git grep -n 'reposition_atomic_box' 658cc302 -- crates` | 13 lines. `inline/mod.rs`: definition `:679` (`pub`), **calls `:496`, `:578` — both inside the range**, comment `:176`. Cross-crate: `elidex-layout-multicol/src/lib.rs:11` (import), `:664` (call), `:646` (comment). Comments: `shift.rs:70`/`:155`, `atomic.rs:17`, `multicol/src/tests/mid_break_flow.rs:1042`/`:1055`/`:1210` |
| the moved range holds no `LayoutBox`/`BoxModel` **reader** | `git show 658cc302:…/inline/mod.rs \| awk 'NR>=413 && NR<=639' \| grep -cwE -e LayoutBox -e BoxModel` | 5, and all five are `//` comment lines (`:469`, `:472`, `:524`, `:535`, `:557`) — ⚠ the wire greps **both** tokens, so the `BoxModel` half was measured too: 0 |
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
| `static_atomic_reposition_records` | `:721` | outside `413-639`; the range is what the umbrella ratified. ⚠ **Its only non-doc callers now live in `reconcile.rs`** (both inside `reconcile_flows`), so keeping it here is accepted cohesion debt, not a claim the home is right |
| `relpos_atomic_reposition_records` | `:747` | same — sole non-doc caller is in `reconcile.rs`'s `reconcile_flows` |
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

⚠ **The edge is bidirectional, and this PR accepts that rather than resolving it**: `mod.rs`
calls into `reconcile`, and `reconcile` imports four helpers back out of `mod.rs`. For two of
them the sole consumer is now the child, so a cohesion argument says they should travel. The
reason they do not is scope — moving a definition is not the ratified range — and the question
is booked as **"Where the four helpers should live"** in `#11-inline-fragmented-fn-seams-1-2`.

**The back-edge this leaves is the crate's established idiom, measured rather than asserted.**
`reconcile.rs` imports four helpers defined in its parent, which is the shape a reader is most
likely to challenge. The crate already runs it:

The population is whatever this returns:

```
git grep -n 'use super::' 658cc302 -- 'crates/layout/elidex-layout-block/src/**/*.rs' | grep -v tests
```

| shape | instances on `658cc302` |
|---|---|
| child imports a fn defined in the parent | `positioned/layout.rs:20` (`use super::resolve_offset`, defined `positioned/mod.rs:46`); `block/children/{stack.rs:13, helpers.rs:14, shift.rs:8}` (`use super::super::is_block_level`, defined `block/mod.rs:46`); `block/children/stack.rs:16` (`use super::{make_block_break_token, …}`, defined `block/children/mod.rs:49`) |
| **bidirectional** parent↔child | `positioned/mod.rs:28` re-exports `layout::{…}` while `layout.rs` imports `super::resolve_offset`; `block/mod.rs:36` imports `children::shift_block_children` (the call is `:463`) while `children/*` import `super::super::is_block_level`; **`block/children/mod.rs:18`** (`pub use stack::stack_block_children`) against **`stack.rs:16`** — ⚠ the **closest analogue to this PR**, because its child→parent leg is a *direct* parent-defined-`fn` import, exactly like `reconcile.rs`'s `use super::{clear_inline_flows, …}`, whereas the other two route through a grandparent (`super::super::`) or a re-export |

Keeping all four beside each other is at least one uniform rule, where splitting two would be a
2/2 — the *opposite* of *one issue, one way* — and the arrangement instantiates an idiom the crate
already carries, not a novelty this PR introduces. ⚠ **This is not a claim the home is right**:
§9 records a third configuration (a shared sibling module importing into both) that satisfies the
uniformity argument *and* removes the back-edge, and it is un-weighed here. The reason these two
stay is scope. 

⚠ **The correspondence §6 depends on**: since all four stay, the moved range is *exactly* what §6's
harness extracts — no `fn` sits outside both extracts. That is what makes the harness a proof of
the whole move rather than of part of it, and §8 carries it as a DoD clause.

### §5.3 `#[allow(clippy::too_many_arguments)]` on the new function

The successor slot `#11-inline-fragmented-fn-seams-1-2` is **created by this PR**. Its contents split by **origin, not by count**.
**Pre-existing**: seams 1 and 2, named by the source slot on 2026-07-28.
**Created by this PR**: the moved body's probe universal losing its counterexamples to the residue
(§7.2 — the *text* is pre-existing, the *separation* is this PR's, and it cannot be repaired here
without breaking byte-identity); `reconcile_flows`' eleven-parameter signature and its
adjacent-`bool` window, which §9 itself calls new — **and the helper-home question**: the question is "where should the four
helpers live once their principal caller is a **sibling module**", and at `658cc302` there is no
sibling module, so the question could not be asked. Its premise is brought into existence by this
PR; it is own.
The slot ledger carries the slot's `(own)` row (§10).

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
should become components — the *parameters* are intra-pass scratch, which is a defensible
reading of that rule's scope (⚠ their **values** do escape, via the carrier — see the row below) — but the ECS question must be *on* the slot, not absent
from it.

### §5.3.1 ECS-native check / OO → ECS mapping

`axes.md`'s Axis 2 `[plan]` detect entry asks a plan-memo to carry this subsection explicitly. The
check was run; only its *result* was missing, which is the thing a reader cannot reconstruct.

| question | answer |
|---|---|
| Does the extraction introduce an OO pattern (registry, observer, subscriber list, class-owned state)? | **No.** It adds one `pub(super) fn` and a `mod` declaration. No trait, no `Vec<Box<dyn …>>`, no `ObjectKind` variant, no new state container. |
| Does it move per-entity state into a side-store? | **No.** The three parameters (`unoffset_origins`, `flow_lines`, `relpos_atomic_placements`) are **pre-existing**, produced by `layout_atomic_items` and the packer; the split only makes them cross a function boundary. ⚠ Only two are entity-*keyed* (`HashMap<Entity, _>`); `relpos_atomic_placements: &[(Entity, f32, f32)]` is a flat slice, iterated in order and never looked up. The distinction is load-bearing because the rule's trigger text is written about `HashMap<entity, _>`. |
| Do they meet CLAUDE.md's *side-store→component* rule? | **Not applicable as a defect.** **Shape**: they are arguments threaded through one call chain, not held beside the World. `elidex-layout-multicol` drains a carrier into a `FragmentSnapshot` (`fill.rs:228`) and folds it into a render-visible `InlineFlow` (`lib.rs:640`), so a **stale** one is not inert. What actually bounds the hazard is **which entities the drain reaches** — `fill.rs:220` iterates `carry_midbreak` chained with `break_out_child`, selected out of `composed_children_flat(dom, entity)` (`lib.rs:173`), which is **neither "direct children" nor that flattened set entire**. Values from all three *are* copied into `ColumnFlowSlice`, and that carrier does **not** always die with the pass: one written on a *nested* IFC container is reached by neither terminal path, and outlives `layout_multicol`'s return. The residual — a carrier that outlives its pass and whose entity then changes role — is `#11-inline-fragmented-fn-seams-1-2`'s, and `reconcile.rs`'s docstring is the contract. ⚠ **This is documented in-tree**: `elidex-layout-multicol/src/fill.rs:48-53` states that a deeper IFC "writes its carrier on that inner container, which this drain (keyed on the direct child) never reaches: the carrier leaks (benign — render never reads it)". ⚠ **`elidex-ecs/src/components/inline_flow.rs:214-215` asserts the narrower lifetime universal**; what the same docstring carries that *does* hold is `:214` **"Never read by render"** and `:217` "a stray write that is never drained is benign". Enumerating the terminal-path set, and correcting the component's universal, are `#11-inline-fragmented-fn-seams-1-2`'s. The question is nonetheless **put on the successor slot** (§9) rather than answered silently, because a future reshaping should re-make the judgment rather than inherit it. |
| What ECS state does the moved code own? | Two components, and **this row is scoped to the split's two modules — it is NOT the workspace write-set.** Within them: **`InlineFlow`** — insert in `reconcile.rs`; removal via `remove_one::<InlineFlow>` inside `clear_inline_flows` (`mod.rs`), invoked from *both* modules (the residue's two early-return exits and the moved `!env.is_probe`-gated call). **`ColumnFlowSlice`** — insert-or-remove in `reconcile.rs`, plus two removals in the residue's early-return exits. Both write sets span the new module boundary, symmetrically. ⚠ **The workspace complement is non-empty and is not listed here** — run `git grep -n 'InlineFlow\|ColumnFlowSlice' -- 'crates/**/*.rs'` and classify the hits by hand. The two receivers most likely to be missed are `elidex-layout-multicol` and `block/children/shift.rs`. ⚠ `ColumnFlowSlice` is not read by `elidex-render`: `git grep -l ColumnFlowSlice -- 'crates/core/elidex-render/**'` → empty, and the live control `git grep -l InlineFlow -- 'crates/core/elidex-render/**'` → non-empty. ⚠ **Anchor the enumerator on the component NAME, not on the call syntax.** Call-shaped patterns (`insert_one(.*ColumnFlowSlice`, `remove_one::<ColumnFlowSlice>`, …) drop two real write sites here: `elidex-layout-multicol/src/lib.rs`'s path-qualified `remove_one::<elidex_ecs::ColumnFlowSlice>`, and `reconcile.rs`'s `insert_one` whose `ColumnFlowSlice { .. }` literal spans several lines. The name is invariant; the call syntax is not, so a syntax-anchored pattern is a filter that looks like an enumerator ([[feedback_writesite-audit-includes-struct-literal-ctors]], [[feedback_checks-must-not-be-defined-by-the-symptom-vocabulary]]). |
| Does anything outside the crate depend on this function's clear having run? | **Yes, and it is worth knowing before touching the persist/clear cycle.** `elidex-layout-multicol/src/lib.rs` re-inserts `InlineFlow` on the run-start after the IFC pass (`position_column_fragments`), and guards it with a `debug_assert!` that the run-start carries **no** `InlineFlow` at build time — *"cleared each column by `clear_inline_flows`"*. So the moved block's clear is a precondition of another crate's write. Nothing in this PR changes it (the code is byte-identical modulo the signature's bindings), but a future reshaping of the cycle that reads only the two modules above would not see the constraint. |
| Is `ColumnFlowSlice` itself a side-store→component candidate? | **No, and the question is category-confused.** `ColumnFlowSlice` **is already an ECS component**; there is no side-store to migrate *from*. Its docstring makes both halves explicit — *"so it **is** a component (per-entity, `Send + Sync`, not a per-VM identity handle — the side-store→component rule), **not** a side-store"*. ⚠ A *different* and still-open question exists nearby — whether per-entity payloads about *other* entities belong on those entities rather than on the IFC parent — but that is an **ownership** question, not this rule, and asserting it under this rule's name would direct future work to dismantle an established ECS-native phase boundary. This PR has no ownership invariant to offer for it, so it is **routed, not answered**: it is booked on `#11-inline-fragmented-fn-seams-1-2` as a bullet distinct from the side-store→component rule, and that slot's trigger reaches it. |

### §5.4 `#[allow(clippy::too_many_lines)]` on the residue

§8 requires this re-evaluated, and the source slot says (`:37`) to drop it "if the residue no
longer needs it". **It still does.** Measured by deleting the attribute with the split applied:
`this function has too many lines (177/100)` (178 before §2.2's fold). It stays, and the measurement is recorded so the
next toucher re-runs it rather than re-litigating it.

### §5.5 Resulting sizes

```
wc -l crates/layout/elidex-layout-block/src/inline/{mod,reconcile}.rs
```

`mod.rs` is **573**, and the 1000-line argument turns on it. Both files sit below
[[feedback_touch-time-split-means-while-writing]]'s 700–800 band, and the residue is 212 lines
further from the 1000-line gate than it was — the source slot's 1000-line trigger disjunct, which
this PR moves away from firing rather than toward.

### §5.6 Provenance of §5.3–§5.5's figures

**Status: re-measured on the committed implementation.** Every figure below was first taken on a
throwaway extraction that was not in the branch, and §8 required each to be re-run against the
shipped tree before landing. Result of that re-run:

| figure | predicted | measured on the implementation |
|---|---|---|
| `too_many_arguments` load-bearing | yes | yes — `this function has too many arguments (11/7)` |
| `too_many_lines` still load-bearing on the residue | yes, `178/100` | yes, **`177/100`** (§2.2's fold) |
| `inline/mod.rs` | 573 | **573** |
| `reconcile.rs` | 254 | run §5.5's `wc -l` |
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
#    The docstring's extent:
#      awk '/^\/\/\//{if(!s)s=NR;e=NR} END{print s"-"e}' \
#        crates/layout/elidex-layout-block/src/inline/reconcile.rs
#    ⚠ What is NOT derivable from the base is the docstring's *authored* content, and
#    that is narrower than "all of it". Per §3 for the citation and §7.2 for the probe universal, `CSS 2 §10.8` and that universal's
#    TEXT both already exist inside the moved range at 658cc302 (base `:480` and
#    `:521`); §3 records `CSS 2 §10.8` as a DUAL-PROVENANCE row for exactly this reason.
#    Authored here are: the `css-writing-modes-4` citation, the `css-inline-3`
#    citations, and the SCOPING of the probe universal to this function
#    -- not the citations wholesale.
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

## §6. Proof obligation

§8's criterion — "a diff of the moved body against the same block extracted from that base
differs only in those bindings" — is discharged by a harness, not by inspection:

1. Extract `413-419` + `421-639` from `git show 658cc302:…/inline/mod.rs`.
2. Extract the new function's body from `reconcile.rs` (everything between the signature's
   closing `) {` and the final `}`).
3. `difflib.unified_diff` with `n=0`.

**Pass condition**: every hunk is one of §2.3's four substitutions, and the two extracts have the
same line count (226). Run against the exploratory extraction on `658cc302`, the harness returns
**six single-line hunks**, one per site in §2.3, and `226 == 226`. The same harness is re-run on the committed implementation and its output goes in
the PR body.

⚠ **The harness above covers the body, and the body is not the whole move** — a move is the
extracted text *plus the call that replaces it*, and the call is outside the compared region by
construction (the extract begins after the signature's `) {`).

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
one-shot verification procedure is not that. Standing them up is the
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

### §7.1 Comments the move makes less accurate

⚠ **Pinned to `658cc302`, not the working tree.** This is a *pre-change* inventory, and this PR
rewrites three of the five sites — run against the tree it returns **2** (the two of the
seed's five that are deliberately left), which cannot substantiate the table below. Same pinning rule as §6's harness.

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

⚠ **That sweep is a *seed*, not an inventory.** It is keyed on the vocabulary these comments
happen to use ("persist block", "reconcile … in `layout_inline_context_fragmented`") rather than
on the property being tested, so it can neither be shown complete nor safely widened: adding
`Written by` and re-running over the whole workspace does surface a real further site, but it
also matches `elidex-ecs/src/components.rs:189` ("Written by the pre-layout generated-content
pass"), a different subject entirely. The authoritative predicate is *"a comment that names where
the moved block lives"*; the command only proposes candidates for it. The further site it proposes — `elidex-ecs/src/components/inline_flow.rs:204` — has its
disposition in the §7.1 table.

**Sites outside the range**, splitting on whether they name a *location* or the block as a
*concept*:

| site | names | disposition |
|---|---|---|
| `mod.rs:174-176` | "see the persist block's `reposition_atomic_box` calls" — an in-file pointer, and no such block is in this file now | **fixed here** |
| `mod.rs:774` | "the reconcile comment in `layout_inline_context_fragmented`" | **fixed here** |
| `collect.rs:130` | "see the reconcile in `layout_inline_context_fragmented`" | **fixed here** |
| `collect.rs:209` | "the projection axis the persist block uses for ALL groups" | **left** — names the block as a concept, still true |
| `atomic.rs:17` | "The persist block uses this as the reposition delta basis" | **left** — same |
| `elidex-ecs/src/components/inline_flow.rs:204` | "Written by `layout_inline_context_fragmented` on the **IFC container** entity" | **left**, and not the concept/location split: `reconcile_flows` is `pub(super)` inside a private `mod reconcile;`, so from `elidex-ecs` it is not a nameable symbol — repointing the component's SSoT at something unreachable would be worse than the mild imprecision, and `layout_inline_context_fragmented` remains the only entry point through which the write happens. Reached by widening the seed, not by it |

For the five sites the seed reaches, the line is: the fixed ones name a location that moved, the
left ones name a thing that still exists — which is why `collect.rs` appears in this PR's diff
(§8). ⚠ **The sixth row is an exception, dispositioned on its own ground**: it names a location
*and* is left, because the location it would be repointed at is not nameable from `elidex-ecs`.

* `clear_inline_flows` takes no probe flag (`fn clear_inline_flows(dom, candidates, persisted)`),
  so "the **`is_probe`-gated** `clear_inline_flows`" cannot name the definition. It names the
  **call** guarded by `if !env.is_probe`, and that call travelled with the block, so it is in the
  same file and below the comment. Still true.
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
`#11-inline-fragmented-fn-seams-1-2`.

## §8. Definition of done

* `crates/layout/elidex-layout-block/src/inline/reconcile.rs` (NEW) exists and holds the module
  doc, the `use` block §5.6 pins, the attribute, and `reconcile_flows` — and **no `fn` outside the
  one §6's harness extracts**, which is what makes the harness a proof of the whole move rather
  than of part of it. (The constraint is on `fn`s, not on "nothing else": the imports are
  mandatory, since the moved lines name `InlineFlow`, `ColumnFlowSlice`, `EcsDom`, `Entity`,
  `Point` and `HashMap` unqualified.)
* **Both halves of §6 pass** and their output is in the PR body — the body harness
  (`6 hunks, 226 == 226`) **and** the §6.1 call-site check. ⚠ The second is not optional:
  the first cannot see the call.
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
  `cargo doc` with `RUSTDOCFLAGS=-D warnings`:
  `relpos_atomic_reposition_records`'s intra-doc link to `[static_atomic_reposition_records]`
  (`mod.rs:745`) must still resolve — it does trivially, both stay.
* **Every figure §5.6 marks pending is re-measured on the committed tree**: the two `#[allow]`s'
  necessity (`too_many_arguments`; `too_many_lines` at the figure §5.4 records), both `wc -l`s,
  and §6's hunk count.
  * The **squash commit message** is the text below; accepting GitHub's default squash message violates this DoD.
  * The **PR description** is re-checked at merge: `gh pr view 508 --json body -q .body | tr '\n' ' ' | grep -oE '[^.]*reconcile\.rs[^.]*'` — read the sentences it prints; none may state a line count.

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
  * The five figures are stable — `mod.rs` 573, `too_many_lines` 177/100,
    `too_many_arguments` 11/7, §6's `6 hunks / 226 == 226`, 325 tests — and go in the squash message at merge.
* **`git diff --name-only origin/main...HEAD` names four files** — this memo, `inline/mod.rs`,
  the new `inline/reconcile.rs`, and `inline/collect.rs` (one comment, §7). ⚠ **Nothing under
  `.claude/`, and no second `docs/plans/` file** — that is the mechanical statement of the
  narrowing in the preamble, and the cheapest way for a reviewer to confirm it.

* **No row of the landing checklist is left `still owed`.** Evaluate it, do not assert it —
  this must print `0` at merge:

  ```sh
  L=$(ls "$HOME"/.claude/projects/*elidex*/memory/project_seam3-pr508-review-history.md)
  grep -c '| `still owed`' "$L"
  ```

  The targets are outside this repository, so the diff cannot show them.

## §9. Out of scope, with disposition

⚠ **One home per disposition, and for routed work this section is not it.** A concern here is
either **accepted** — terminal, no work follows, and this section is canonical for it — or
**routed**, in which case the *slot* is canonical for what the work is and when it reopens, and
this section carries only **why it is not in this PR**, plus the slot's name.

* **Seams 1 and 2** of `#11-inline-fragmented-fn-decomposition` — `mod.rs:266-303` (orphans/widows
  break computation) and `:388-411` (the packer-relative → layout coordinate fold), both
  re-measured on `658cc302`. The umbrella books them into the successor slot
  `#11-inline-fragmented-fn-seams-1-2`.

  **The ground is the ratified range, and nothing else.** What actually bounds this PR is that **the umbrella ratified `413-639` and only that**. Widening
  to seams 1 and 2 is an unratified scope change to a ratified surface
  ([[feedback_plan-ratified-surface-is-a-design-change]]), and the per-seam cohesion analysis the
  source slot calls for has not been done for them.
* **The eleven-parameter signature.** Reducing it is a design change (§5.3) and belongs with the
  successor slot `#11-inline-fragmented-fn-seams-1-2`, whose subject is the residue's
  decomposition. ⚠ **Booked alongside it, because it is a different defect the count would hide**: the signature
  carries `is_vertical: bool, persist_flow: bool, do_carrier: bool` **adjacent** (positions 5-7 of
  11 — adjacency is the hazard, not terminal position) and the call site passes them positionally, so **any transposition of the three is type-correct and compiles silently**. That
  window is *new* — pre-split these were three named `let` bindings in scope (`:173`, `:322`,
  `:343`). The fix for a three-`bool` positional window is a **type** (an enum
  or a flags struct, so a transposition fails to compile).

  ⚠ **Why the type cannot be introduced here.** Byte-identity governs the body after `) {`; the
  signature is **authored**, so that contract does not govern it. The obstruction is one level
  in: the flags are **consumed by the arms** `if persist_flow { … } else if do_carrier { … }`,
  which *are* inside the compared body, so a `FlowSink` enum rewrites them and breaks the proof.
* **Where the four helpers should live once their principal caller is a sibling module.** §5.2
  keeps all four in `mod.rs`, and two of its four reasons are *design* reasons (`pub` API;
  residue callers) while two are *scope* reasons (outside the ratified range). ⚠ A third
  configuration exists that §5.2 does not weigh — all four into a shared sibling imported by both
  `mod.rs` and `reconcile.rs`, which satisfies the uniformity argument **and** removes the
  child→parent back-edge. Declining it here is right (it is outside the range), but leaving it
  unrouted would let the next reader take §5.2 as "settled" rather than "declined on scope".
  **Routed to `#11-inline-fragmented-fn-seams-1-2`**, whose entry carries it.
* **The uncited spec-governed concerns in the moved body** (the slot's entry enumerates them).
  **Why not in this PR** — the ground is change class —
  this PR authors no algorithm, so it neither creates nor deepens a missing-citation defect, which
  is the position #497 took when it **withdrew** the module-doc citations it had added to
  `collect.rs` (CSS 2 §9.2 *Controlling box generation*, the parent section) and `styled_run.rs`
  (CSS 2 §9.2.2 *Inline-level elements and inline boxes*) as over-claiming. **Routed to
  `#11-inline-fragmented-fn-seams-1-2`**.

* **CSS 2 §10.8 is superseded, and the instance this PR authored is re-anchored.**
  `css-inline-3` §1.1 *Module Interactions* says the module *"replaces and extends the CSS inline
  layout model and features defined in [CSS2] section 10.8"*, so the `reconcile_flows` docstring
  now names `css-inline-3` §2.2 / §4.2 / §5.3 as its governing sections. **Why not in this PR**: the sites are pre-existing.
  **`inline/mod.rs:65` and `:116` are ordinary
  `CSS 2.1 §10.8` citations in the residue, outside the byte-identity range — this PR could have
  re-pointed them and chose not to.** Re-pointing the rest is a crate-wide sweep
  whose class (module supersession) is a different one from *wrong-section* misattribution, since
  CSS 2 §10.8 genuinely is the section it names. **Routed to
  `#11-css2-line-height-supersession-reanchor`**; deliberately **not** folded into
  `#11-css2-spec-label-normalisation`, whose memo states it owns *hygiene, not correctness*.
* **The `vertical-align` gap** that §3's CSS 2 row and the docstring record — pre-existing, and this
  PR authors no algorithm for it. **Routed to `#11-vertical-align-line-box`** (pre-existing class).
* **Cold gate** (not a concern — a gate record, re-run at merge per its last line; [[feedback_split-on-touch-prereq-workflow]]), re-run on `658cc302` at this PR's
  own touch set (`crates/layout/elidex-layout-block/` plus the four `.claude/`+`docs/` files of
  `f63eb623`):
  * Open PRs: `gh pr diff <n> --name-only` for each co-open PR — the set is whatever
    `gh pr list --state open` returns at the time of the run, **not a list stored here**.
    ⚠ **File-disjointness is not the whole gate for #501**: it
    edits `preflight.py`, so there is a
    *behavioural* dependency the name-set intersection cannot see.
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
      deletions(-)` (verified 2026-08-16).
  * ⚠ The gate is re-run **before merge**, not trusted from here — `main` moves, and PRs open and
    close.

## §10. Where the slot-ledger actions are recorded

Not carried here. The row-by-row checklist — which targets the landing writes, which it still
owes, and against what predicate — is `project_seam3-pr508-review-history.md`, in the agent
memory directory alongside every file those rows act on.
