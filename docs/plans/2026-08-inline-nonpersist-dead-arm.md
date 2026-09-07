# Dead-arm prereq split: delete the IFC packer's unreachable non-persist commit arm

Prereq PR of the `#11-line-box-decorated-inline-content` umbrella
(`docs/plans/2026-08-line-box-decorated-inline-content.md`, on branch `layout-decorated-inline`,
frozen at rev 24 and **not** approved for implementation). The umbrella's §5.1 M4 establishes the
arm is dead, its §5.2 surface table enumerates this PR's sites, and its §10 books exactly two
ledger rows to it; §8 fixes the ordering (seam-3 first — landed as PR #508, `7e256029`, so this PR
pays the asymmetric-cost argument nothing). This PR exists ahead of the umbrella's approval
because it is **independent of that approval**: the arm's unreachability is a fact of
`origin/main`'s types, provable and deletable whatever the umbrella decides.

Per the narrowing rule ratified on the seam-3 PR (2026-08-16, recorded in
`docs/plans/2026-08-inline-seam3-reconcile-split.md`'s preamble):

> a PR carries the bookkeeping **its own change makes true**, and hands the program's bookkeeping
> to the program.

So this PR ships this memo and the deletion, and at landing performs the two §10 ledger actions
this deletion makes true (§5 below) — nothing else of the umbrella's.

**Coordinate frames.** The umbrella's declared frame is **`154bac3f`** (its front matter: "Every
`file:line` in this memo is a `154bac3f` coordinate"); in that frame its dead-arm sites are
`inline/mod.rs:239`/`:240-251`/`:322` and `pack/mod.rs:393-421`. This PR's base is **`7e256029`**
(seam-3 landed), and every coordinate below is a `7e256029` coordinate, re-measured. The shift
against the umbrella's frame: seam-3 made two one-line insertions in `inline/mod.rs` — the
`mod reconcile;` declaration near `:17` and one doc-comment line in the `:172-181` region — so a
site above `:17` shifts 0, a site in `(17, ~172]` shifts +1, and the dead-arm sites (all ≥ `:215`)
shift **+2**;
`pack/mod.rs` is untouched since the umbrella's frame:

```
git diff --stat 154bac3f..7e256029 -- crates/layout/elidex-layout-block/src/inline/pack/
# → empty
git diff 658cc302..7e256029 -- crates/layout/elidex-layout-block/src/inline/mod.rs | grep -c '^@@'
# → 4 (two +1 insertions above the dead-arm sites, the seam-3 extraction below them, one in-place reword)
git show 7e256029:crates/layout/elidex-layout-block/src/inline/mod.rs | grep -n 'persist_candidate ='
# → 241 (umbrella's :239 + 2; base-pinned — this PR deletes the symbol from the working tree)
```

## §1. The arm, and the proof it is unreachable

`LinePacker::flush_line`'s rendered-content branch forks on `self.flow_align`
(`crates/layout/elidex-layout-block/src/inline/pack/mod.rs:232`): the `Some` arm runs the per-line
natural→painted bake, the `else` arm (`:393-421`) commits `current_line_entity_rects` unaligned,
per break segment, unmerged. The `else` arm is dead, by three facts that compose:

1. **`FragmentationType` is a two-variant exhaustive enum** — `Page` and `Column`, nothing else
   (`crates/layout/elidex-layout-block/src/lib.rs:36-42`).
2. **`InlineFragConstraint.fragmentation_type` is non-optional** — `pub fragmentation_type:
   crate::FragmentationType` (`crates/layout/elidex-layout-block/src/inline/mod.rs:95`).
3. Therefore `persist_candidate` (`inline/mod.rs:241`):

   ```rust
   let persist_candidate = frag_constraint.is_none() || frag_is_paged || frag_is_column;
   ```

   is **identically true**: `frag_constraint` is either `None` (first disjunct) or `Some(c)` with
   `c.fragmentation_type ∈ {Page, Column}` (second or third disjunct, `:237-240`). So
   `flow_align = persist_candidate.then_some(…)` (`:242`) is always `Some`, and
   `pack::LinePacker::new` has exactly one caller passing exactly that value:

   ```
   grep -rn 'LinePacker::new' crates/
   # → crates/layout/elidex-layout-block/src/inline/mod.rs:253 (sole caller)
   ```

Both `flow_align` read sites — the `flush_line` fork (`pack/mod.rs:232`) and `place_item`'s
recording gate (`pack/mod.rs:736`) — therefore take their `Some`/`true` branch on every
execution. The `else` arm at `:393-421` and the `is_some()` guard at `:736` are unreachable and
identically-true respectively.

The arm's own comment (`:394-400`) already knows this fate — it books its deferred work to
`#11-inline-align-clientrects-nonpersist-path` "(re-evaluate at Z-slice landing, which deletes
the non-persist paths)". This PR is that deletion arriving earlier, with a proof instead of a
Z-slice: CLAUDE.md's design discipline ("dead code は接続するか削除") does not let a provably
unreachable arm wait on an unrelated program.

**What is NOT dead, and stays.** The *post-pack* persistence narrowing is live:
`persist_flow = persist_candidate && (!frag_is_column || column_is_whole)` (`inline/mod.rs:324`)
gates whether the optimistically-recorded `flow_lines` are persisted, and its second conjunct is
genuinely false for a multicol mid-break run (`do_carrier`, `:345`). Only the **first** conjunct
is redundant. The multicol-mid-break carrier route, the paged slice/rebase (`:364`), and the
suppressed-line discard arm (`pack/mod.rs:423-431`) are all reachable and untouched.

## §2. The change, site by site

All sites at `7e256029` coordinates. The shape of every edit is *deletion of an
identically-decided branch plus the type simplification that makes the deletion permanent* —
`Option<FlowAlign>` → `FlowAlign`, so the dead fork cannot grow back type-silently.

`crates/layout/elidex-layout-block/src/inline/mod.rs`:

* `:241` — delete `persist_candidate`.
* `:242-248` — `flow_align` becomes a plain `pack::FlowAlign { … }` construction (no
  `.then_some`).
* `:324` — `persist_flow = !frag_is_column || column_is_whole` (drop the identically-true
  conjunct).
* Comments: the persistence-gate block (`:215-236`) loses its `persist_candidate`-OPTIMISTIC
  paragraph — the optimism it describes (record now, discard if the run does not persist) is
  real and stays, but it is `persist_flow`'s to describe, so the surviving text moves the
  explanation there; the `persist_flow` block (`:311-322`) is reworded to define the gate
  directly instead of as a narrowing of a deleted pre-gate; `:322`'s "see `persist_candidate`"
  pointer goes with it.

`crates/layout/elidex-layout-block/src/inline/pack/mod.rs`:

* `:126-128` — field `flow_align: FlowAlign`; doc comment rewritten (it currently documents the
  `None` state).
* `:175`/`:188` — constructor takes `FlowAlign`.
* `:232` — `if let Some(fa) = self.flow_align {` → `let fa = self.flow_align;` and the arm's body
  dedents; **the `else` arm `:393-421` is deleted whole**, including its comment naming the slot.
* `:736` — the `if self.flow_align.is_some()` gate is removed; the recording `match` dedents.
* Comments: the commit-seam comment (`:214-223`) loses its "the non-persisting fragmentation
  path keeps the natural per-segment commit (path 2 — deferred, the `else` arm)" clause;
  `place_item`'s recording comment (`:717`) loses "when persisting (`flow_align.is_some()`)" —
  recording is unconditional, discard is the caller's `persist_flow` decision; `FlowAlign`'s
  struct doc (`:34-35`) loses its "Present (`Some`) … `None` skips all flow recording"
  sentence, and its `is_vertical` doc (`:70-72`) loses "gated on a `Some` `FlowAlign`".

`crates/layout/elidex-layout-block/src/inline/pack/items.rs`:

* `:37-38` — `FlowMember`'s doc "(only when persisting — `flow_align.is_some()`)" → recorded
  unconditionally; where a recorded run goes is the caller's `persist_flow` routing —
  persisted, carried per column, or (probe only) discarded.

`crates/layout/elidex-layout-block/src/inline/tests/inline_flow/fragment.rs`:

* `:45` — "(D-mc2 — the optimistic `flow_align` for `Column` …)" → the optimism is now the
  packer's unconditional recording, not a `flow_align` variant; reworded, claim unchanged.

`crates/layout/elidex-layout-block/src/inline/reconcile.rs` (found by the pre-push `/simplify`
pass — the initial sweep grepped only the deleted tokens, i.e. was defined by the symptom
vocabulary; the property "describes `persist_flow`'s formula" reaches one more site):

* `:178-179` — the mutual-exclusivity proof said `do_carrier` "falsifies `persist_flow`'s second
  conjunct"; with the conjunct gone, `do_carrier` is the exact negation of `persist_flow` (De
  Morgan), and the doc now says so. A `grep -rn 'persist_flow\|do_carrier' crates/ --include='*.rs'`
  sweep confirms no other site describes the formula's *shape* (the rest reference the flags or
  the implication chain `do_carrier ⟹ persist_flow == false`, both shape-independent).

Two further hygiene edits from the same `/simplify` pass, both inside the dedented `flush_line`
body: the duplicate `let block_size = self.current_line_height;` binding (stranded in one scope
with `line_height` by the dedent) is dropped — the `InlineFlowLine` push reads `block_size:
line_height`; and the persistence-gate narration that lived at both the `FlowAlign` construction
site and the `persist_flow` gate is consolidated at the gate (one decision, one narration site) —
the construction site keeps only the recording-is-unconditional paragraph, which is about the
input built there.

### Pre-push `/code-review` findings, applied — including the class the deletion made evident

The 8-angle bug/cleanup pass (medium) confirmed the deletion's mechanics independently (token
identity of the dedent, constructor sweep incl. `#[cfg(test)]`, truth-table equivalence) and
surfaced two real find groups, both applied:

1. **A false claim my own consolidation had replicated**: "a non-persisting run's recorded lines
   are discarded" is wrong for the carrier route — `reconcile_flows` captures them into
   `ColumnFlowSlice` as the paint source for terminal-Z C-1/C-2; only a **probe's** are
   discarded. Reworded at the construction-site paragraph, the `persist_flow` gate comment,
   `place_item`'s recording comment, `FlowMember`'s doc, and this memo (the items.rs bullet
   above); also the stranded "When persisting:" in `pack`'s Atomic-dispatch comment and
   `FlowAlign`'s "context for persisting" heading (recording is unconditional).
2. **The two-bool `reconcile_flows` interface re-encoded the deleted third state.** With the
   conjunct gone, `do_carrier` is definitionally `!persist_flow`, so the `persist_flow ||
   do_carrier` guard (`reconcile.rs:219`, base frame) was a tautology whose skip path was
   unreachable by exactly §1's sole-caller argument — the one place §4's "type simplification
   makes the deletion permanent" did not reach. Collapsed to **one bit**: `inline/mod.rs` derives
   `do_carrier = !persist_flow` (the `frag_is_paged || do_carrier` consumer keeps the name),
   `reconcile_flows` takes `persist_flow` alone, the tautological guard is dropped (its block
   dedents), the `else if do_carrier && …` arms lose their redundant conjunct, and the carrier
   reconcile keys on `!persist_flow`. The exclusivity the arms rely on now holds by construction;
   the doc's "if both were ever true" hedge is deleted with the state it defended. Also moved the
   `frag_is_paged`/`frag_is_column` bindings down beside `persist_flow` (their placement was
   residue of the deleted pre-gate).

⚠ Item 2 exceeds the surface the umbrella's §5.2 dead-arm row enumerates (that row lists
`pack/mod.rs`'s arm and the `inline/mod.rs` pre-gate half only; the interface collapse reaches
`reconcile.rs`'s signature). It is the same class — control flow shaped by the vacuous pre-gate —
so it lands here under *one issue, one way*, and the delta is recorded as an input for the
umbrella's round 20 rather than taken silently.

## §3. Spec coverage map

The PR adds no algorithm and changes no observable behaviour (§4), so every row is a
**relocation or a no-change attestation**, not new coverage. The css-text-3 rows are pre-existing
citations in `flush_line`'s bake that this PR's dedent moves (re-verified via webref, not trusted
as moved-OK); the cssom-view-1 row is the deleted arm's would-be observable.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSS TEXT 3 §4.1.2 Phase II: Trimming and Positioning (#white-space-phase-2) | trailing collapsible spaces hang, count toward neither alignment nor justification | relocated by dedent, semantics untouched | `pack/mod.rs` `flush_line` trimmed-width / hang exclusion | ✗ | no |
| CSS TEXT 3 §6.1 Text Alignment: the text-align shorthand (#text-align-property) | last line before a forced break is start-aligned under `justify` | relocated by dedent, semantics untouched | `pack/mod.rs` `flush_line` `justify_eligible` | ✗ | no |
| CSS TEXT 3 §6.3 Last Line Alignment: the text-align-last property (#text-align-last-property) | `auto` → start for a suppressed justify line | relocated by dedent, semantics untouched | `pack/mod.rs` `flush_line` `offset_align` fallback | ✗ | no |
| CSS TEXT 3 §6.4 Justification Method: the text-justify property (#text-justify-property) | justify distribution over word-separators | relocated by dedent, semantics untouched | `pack/mod.rs` `flush_line` `justify_dist` / `bake_justify` | ✗ | no |
| CSS TEXT 3 §6.4.3 Unexpandable Text (#justify-limits) | zero-opportunity justify line falls back to `text-align-last` = start | relocated by dedent, semantics untouched | `pack/mod.rs` `flush_line` `distribute` gate | ✗ | no |
| CSSOM VIEW 1 §6 Extensions to the Element Interface (#extension-to-the-element-interface) | `getClientRects` box-fragment geometry | **no-change attestation**: the deleted arm committed unmerged, unaligned per-segment rects — the divergence slot `#11-inline-align-clientrects-nonpersist-path` tracked — but was unreachable (§1), so no observable geometry changes and the slot closes (§5) | deleted `else` arm, `pack/mod.rs:393-421` (base frame) | ✗ | no |

## §4. Behaviour: none, and how that is verified

Every deleted branch is unreachable and every de-gated site had an identically-true guard, so no
execution changes. Verification is the crate's own suite plus the workspace gate:

```
cargo test -p elidex-layout-block --all-features
mise run ci        # before push (⚠ cargo-deny is upstream-broken per the standing blocker;
                   # judge by `grep 'ERROR task failed'`, not by sibling SIGTERM fallout)
```

No test is added: a test for the absence of dead code is the compiler's job (`Option` removal
makes reintroduction a type error), and the suite's existing multicol/paged/persist tests
(`tests/inline_flow/{fragment,persist}.rs`) pin the live paths this PR must not disturb.

## §5. Ledger actions at landing (the two §10 rows this deletion makes true)

Both edit the defer-slot SoT `project_open-defer-slots.md` (memory), whose line 313 currently
reads:

> `#11-inline-align-clientrects-nonpersist-path` + `#11-inline-relayout-box-staleness` (fold into
> terminal-Z C-3/C-4)

1. **Close `#11-inline-align-clientrects-nonpersist-path`** — the arm it books work against is
   deleted; there is no non-persist commit left to align. Closed-by, not won't-fix: the defect
   class it named cannot occur.
2. **Split the joint "fold into terminal-Z C-3/C-4" parenthetical** — the fold note survives for
   `#11-inline-relayout-box-staleness` alone; leaving the pairing would strand the SoT asserting
   a fold against a closed slot.

**Disclosed non-edits.** `docs/plans/2026-07-terminal-z-c3a-seam-and-audit-plan.md:573` and
`:661-663` *quote* the SoT's pairing (gate item 6 for C-3b). Those are point-in-time reports
inside a merged decision doc with its own coordinate frame; the SoT is canonical and a C-3b
session re-reads it (the doc's own citation points there). This close half-discharges that gate
item — `#11-inline-align-clientrects-nonpersist-path` is "resolved" — which the corrected SoT
states. The umbrella's own copy of the slot's story (§5.1 M4, §5.2) travels with the umbrella per
the narrowing rule.
