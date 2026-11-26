# Preserve Grid item progress when page width changes track membership

3b.2g under PM 1010. The 3b.2f plan landed at c4b05a9c. L1 owns this child after 3b.2f and before the pagination plan. It consumes the PM 1012 consumed-interval and typed-token contract established in 3b.2.

## §1. Grid placement and content progress

### P2 prerequisite and retained 3b.2g scope (PM 1115)

P2 follows P1's shared layout outcome and precedes 3b.2f. At unchanged width, choose the admitted rows with the existing Grid break policy before publishing item positions. Keep sizing probes isolated from definitive output. For each admitted item fragment make one definitive child call with the fragmentainer space still available at its entry and the incoming continuation matched by stable identity. On first entry pass no continuation. Publish the child's actual fragment extent, source contribution and real outgoing continuation. On same-width resumption route that exact continuation back to its child. A completed sibling stays complete and independent interruptions retain separate identities. Remove `collect_spanning_item_breaks` and its synthetic track-height tokens in P2. One producer owns each interruption from its first dispatch through its final contribution.

At fixed width, force a split between a child's source boxes P and Q. On entry one observe P alone, Q untouched, the actual child extent and its nested token; on entry two observe that same token at dispatch, Q alone, no replay of P or completed siblings, and completion with an empty complement. Give P a different height from the crossed track so geometric synthesis fails the test. A child that spans tracks but completes on its first call yields no child continuation.

PM 1117 applies the shared Block used-size rule in page-source-cursor §1 to the Grid child. Keep the existing Grid style writer until C1b.3. With a 210px definite child, two 16px blocks P and Q, zero edges and a forced break before Q, supply 95px of room on entry and at least 115px at the same width on resume. Assert P alone, 16px of laid-out source content, a 95px published child content box, and size progress 95/210 on the actual child token. The crossed tracks sum to 80px before the trailing gap; neither that sum nor the 16px prefix determines the published height. On resumption the incoming child token supplies the size progress and the route independently: expect Q alone, a 115px box, and completion. Verify the definitive child box remains in ECS after Grid placement and compare both calls with the authored-height Block control. A completed parallel sibling acquires no new contribution. Grid uses each child's returned geometry and forwards its token intact; the Grid container tracks its own size progress. The row cursor remains independent of both.

Inventory at base 01fadb51: run `rg -n 'child_break_tokens|consumed_block_size' crates/layout/elidex-layout-grid/src/tests` and repeat at implementation head. In `fragmentation.rs`, preserve the empty-vector control in `spanning_item_fragmentation`; replace `spanning_item_produces_child_break_token`'s span-implies-interruption expectation with real completion and interruption controls; change `spanning_item_consumed_size_correct` from a sum of track heights to the observed child consumption; change `resume_with_child_break_tokens` from checking a retained entity/vector to observing the incoming child token and the emitted suffix. Include test names, comments and assertions when reconciling the search. Retain row-routing checks for the existing same-width policy.

3b.2g consumes P2. It owns re-placement on width change, selecting items from the complement after regrouping, mapping admitted-row origins, adopting the common chooser with the forced/avoid fixtures below, and count/paint replay. The §2 width-changing crossings remain 3b.2g's obligations.

At c4b05a9c elidex-layout-grid/src/lib.rs expands auto-repeat against the current inline size, then places items using that column count. compute_grid_fragmentation applies a previously saved row_index to the new row tracks. Increasing or decreasing the column count changes the items that precede that row. A width increase can also reduce the new row count below the saved index. Admission and track placement continue to use the existing Grid algorithms.

Commit only the chosen break’s consumed source intervals, keyed by formatting owner and item, to the run-owned token. Carry committed coverage through regrouping and select each item's remaining contribution from its complement. Include the terminal boundary when an item finishes, including an empty item. A partially consumed item retains its covered ranges alongside wholly consumed siblings. Recompute track placement from the full input at the new geometry; determine eligibility for fragment output from that item's coverage. Dense and explicit placement can produce disjoint coverage across items. Preserve it independently of recomputed row membership.

Use 3b.2’s explicit token alternatives at the child boundary. A clean boundary continues selection among remaining items. For each interrupted spanning child keep its stable entity and actual nested token; route it back to that child even if several children span the break. A local run continuation inside a child is interpreted by its own formatter. This routing conveys how to resume the child; the owner/item interval set remains authoritative for what has been consumed.

### Select the break before publishing item positions

At the pre-P2 base lib.rs calls position_items before compute_grid_fragmentation. P2 changes this ordering; 3b.2g extends the selector as follows. Keep track expansion, item-to-cell placement and sizing available to boundary selection. Adapt Grid row boundaries to the common chooser in elidex-layout-block (find_best_break and its shared forced-boundary handling from 3b.2f). Aggregate applicable values over all items starting or ending at the boundary and associate each candidate with its own proposed coverage. Propagate first/last row values to the container as required by Grid §13. Among unforced fitting boundaries prefer an allowed boundary to one with applicable avoid; relax in the shared chooser when progress requires it. A forced boundary takes precedence over avoid. A forced break *inside* a spanning item splits that item’s content independently of siblings.

| Boundary fixture | Expected winner and published intervals |
|---|---|
| Two rows AB and CD both fit; only D requests break-before:page | Break before the CD row, commit A/B only. |
| Single-item rows A/B/C, room for two, all auto | Choose B❘C and commit A/B. |
| Same rows, B❘C has avoid and A❘B is allowed | Choose A❘B. The current generation contains A only. |
| B❘C has both forced and avoid values | Choose B❘C and commit A/B. |

After P2 has selected the admitted rows, 3b.2g maps their positions after regrouping. Map the block-start of the first remaining admitted row to the page content block origin: subtract that row’s full-grid block-start from admitted row positions, preserving the distances between them. Keep the current column positions from full-input placement. Position only admitted item fragments. Reuse P2’s definitive dispatch and returned contributions after regrouping. Obtain the fragment baseline from admitted output. Finish every admitted parallel child before publishing. For each admitted child replace its incoming continuation with its actual outgoing state or mark the item complete. Carry each unadmitted child’s existing intervals and nested token unchanged by identity. Decide completion only after these outcomes are available, even on the last row: return a continuation whenever source coverage is incomplete or a nested continuation remains.

## §2. Crossing checks

| Crossing | Producer → consumer | Distinguishing observation |
|---|---|---|
| Auto-repeat × progress | Auto-placement at one width → next-width fragment | Use repeat(auto-fill, 40px), zero gap, row flow, four labeled equal-height items A–D and room for one row. At 80px AB fit in the first row; at 40px only A fits. Run both 80→40 and 40→80 through actual layout continuations. Capture emitted entities on every fragment and require A–D once each, with C and B respectively the first remaining item. Repeat with fixed explicit two-column placement, whose membership stays stable. |
| Spanning item × row reflow | Interrupted child → re-placed item | Start without an input token. Place an item over multiple rows with identifiable child text P and Q separated by a forced internal break; give its sibling unbroken content. Inspect the actual child token and partial coverage returned after P. Change the width, then assert that the suffix Q is emitted once at child-local inline offset zero, sibling contributions are preserved and no completed child restarts. Assert the interrupted-child alternative and its nested content coverage; compare with a clean boundary for a nonspanning item and repeat at constant width. |
| Placement order × replay | Dense/explicitly placed items → consumed intervals | With source order A/B/C, place B in row one and A/C in row two. After B completes, assert B’s interval and terminal boundary are covered while A/C are untouched. Resume with A/C only. Repeat with dense placement filling an earlier hole. At the boundary record which entities actually finished and which span it. Replaying at the new width must preserve disjoint progress even when row membership differs. Include a shrink in row count beyond the old row index and require termination. |

### Independent count and paint observations

In the parallel fixture, interrupt two spanning children X and Y at distinct interior breaks x1❘x2 and y1❘y2. Have a later admitted sibling Z finish after X interrupts. Require the first page to emit x1, y1 and all of Z. Count one emitted source line for each prefix and two for Z. With zero edges, check each interrupted box’s published block extent against the remaining room at its own entry under CSS Break 3 §5.3. Check the completed auto-sized Z against its two lines, its terminal boundary, and both nested tokens keyed to their entities. Change the width so that only X is admitted on page two. Require only x2 at child-local inline offset zero, completion of X, and the original Y token and covered prefix unchanged in the outgoing state. Admit Y on page three: require only y2 at child-local offset zero, empty complement and no outgoing token. Check the distinct interval sets, actual IDs, positions and sibling absence on each page in both passes.

Separately, fit the entire grid into one row with two nonspanning items. One forces a break between P and Q; the sibling finishes on page one. All tracks fit, but the actual child outcome leaves Q. Assert an outgoing token keyed to that item and exactly Q on page two, with the completed sibling absent and no further continuation.

For each fixture drive counting layout with real outgoing tokens, capture emissions before overwrite and save input tokens and widths. Replay the saved inputs in fresh paint-layout generations. Pass a distinct nonzero layout generation per page and pass into do_layout_grid_fragmented; query boxes visible to that generation for every presence and absence assertion. Compare count and paint independently with literal expectations. For the width fixtures use one row of block room with zero gaps, margins, borders and padding. Coordinates below are (inline, block) relative to the page content origin.

| Column setup and widths | Expected page-local output |
|---|---|
| Auto-fill, 80→40→40 | A(0,0) B(40,0) / C(0,0) / D(0,0) |
| Auto-fill, 40→80→80 | A(0,0) / B(40,0) / C(0,0) D(40,0) |
| Two fixed 40px columns, 80→40 and 40→80 | A(0,0) B(40,0) / C(0,0) D(40,0) in either run, including when the second column overflows the narrow page |

Check cumulative coverage, absence of consumed and rejected items, and an empty complement on the stated final page. The avoid-retreat fixture likewise requires B to have no visible box in the generation that admitted only A.

### §2.1 P2 addendum under PM 1138: Grid item sizing and placement

Review base `780ac30e`, PR #785. The original P2 dispatch contract and the width-changing 3b.2g plan above still apply. This addendum owns the item box from measurement in an admitted area through publication. The regression at each of the last three review rounds came from treating an intermediate result as final: a border-box height as content height, a stretch target as an unconstrained used size, then a container-width probe as intrinsic width and an auto-margin flag as final alignment.

**One layout sequence**

1. **Keep the authored intent until placement.** Resolve self/parent keyword selection at item collection. Retain the effective keyword, overflow-position (omitted, safe, unsafe), physical auto-margin sides and the container’s writing mode/direction. When the self value uses the container’s items value, take its associated safety as well. Sweep the Flex and Grid CSS property handlers, style resolution and computed serialization: today the computed style has safety fields for content distribution, which cannot carry item self-alignment. Exercise authored declarations through parse → cascade → layout; give self/items their own preserved safety wherever that chain loses it. Reject invalid combinations at parse time. Preserve the existing baseline-group calculation and its specified fallback.
2. **Measure the item in the grid area.** Keep track contributions distinct from used item measurements. Obtain intrinsic min-content and max-content contributions for the item’s formatting context; a normal auto-width layout at the grid container width cannot stand in for max-content. Use the item’s available axis in its actual area. For a definite available content space Q, fit-content is max(min-content, min(Q, max-content)). For instance min-content 40, max-content 140, Q=80 yields 80; Q=20 yields 40; Q=180 yields 140. Flow-layout auto block size comes from laying out content at the resolved inline size. Preserve intrinsic sizing of replaced boxes and aspect ratios when resolving normal versus explicit stretch. CSS Grid 2 §6.2 supplies the sizing mode. Stretch of an auto axis without auto margins targets the available margin-box space. An auto margin disables stretching on that axis while preserving the keyword for placement.
3. **The formatter resolves the used box.** Enter with the definite grid area as the containing block. Resolve percentages against the specified reference; subtract fixed margins, borders and padding once from the stretch budget. Apply box-sizing to specified dimensions and constraints, then clamp the content size, honoring min over a conflicting max. Invoke the same box-sizing path for Block, Flex, Grid, Table and Multicol items. For an ordinary block grid item, establish an independent formatting context in the probe and definitive layout; children’s margins stay inside. Preserve subgrid track participation. For each axis let F be area extent minus used border-box size and non-auto margins. Assign max(F,0) to one auto margin, or half to each of two. At F≤0 each auto margin is zero. The formatter returns the measured box including these used margins. Keep computed style immutable.
4. **Grid aligns that returned margin box once.** Use the remaining signed space after *used* margins. For start/end/center the logical offset is zero/all/half of the remainder. When positive space was absorbed, residual space is zero. Explicit safe overflow uses start; explicit unsafe preserves signed offsets. For omitted safety follow CSS Align 3 §4.4.1.3: honor alignment for area overflow, bias overflow away from an unreachable edge of the nearest scroll container when applying automatic scroll safety. For a 120px item, area x=40…140 and accessible page x=0…500, omitted end and unsafe end both give x=20; safe end gives x=40. With area x=0…100 against an unreachable left edge at zero, automatic safety gives x=0, explicit unsafe gives −20. Stretch on a definite or constrained size uses its positional fallback. Do not turn end/center into start during collection or floor the signed placement offset to zero.
5. **Convert logical placement exactly once.** Start from the admitted area’s physical low edge, the item’s used margin-box extent and the container axes. On a forward axis the physical low edge of the margin box is area-low + logical offset; on a reversed axis it is area-low + area-extent − margin-box-extent − logical offset. Finally add the physical low-side used margin and border/padding to obtain the content origin, once across Grid and the child formatter. Horizontal-tb block advances toward +y, vertical-lr toward +x, vertical-rl toward −x. LTR inline advances toward +x horizontally and +y vertically; RTL reverses only the inline direction. Map the row/column origin in the same basis, including nonzero track offsets. Reuse this transform for probes and publication. Keep the signed physical alignment displacement separate from fragment admission/consumed room.
6. **Publish only admitted output.** A non-publishing probe determines the whole item’s used metrics. Both continuous and fragmented placement consume the same metrics, intent, area and transform. Enter a child once definitively for the admitted piece, with its identity-matched token and remaining room. Apply the first-piece block alignment once; preserve inline placement on subsequent pieces and preserve the child-reported extent, coverage and outgoing token. Compare fresh DOMs; verify count/paint and rejected items through the existing P2 observations.

Implementation sites verified at 780ac30e. In Grid, remove early keyword rewriting in helpers.rs, separate track probing from item fitting, and use position.rs for post-fit placement. In Block, sizing.rs and the LayoutInput item context own constrained box and margin resolution; block/mod.rs owns margin collapse. In CSS, audit elidex-css-flex and elidex-css-grid handlers plus elidex-style resolve/flex.rs and resolve/grid.rs. Use existing intrinsic measurement and logical geometry helpers after checking whether they return actual intrinsic sizes and whether their argument is an origin or a displacement. No new ECS store or cached probe result is required.

**Coupled invariants.** The matrix below crosses margin mask (M), keyword/safety (K), free-space sign (F), writing mode (W), constraints (C) and pagination (P). Pair checks: M×K retain keyword; M×F distribute only surplus; M×W map each auto side; M×C distribute after clamp; M×P keep used margins page-stable. K×F permit signed offsets; K×W map logical end once; K×C align constrained size; K×P preserve first-piece alignment. F×W reverse signed overflow as well as positive offsets; F×C derive sign after clamp; F×P distinguish overflow displacement from available page room. W×C map physical min/max to the correct axis; W×P preserve column progression in reversed block flow; C×P keep the same constrained reference on first dispatch.

**Published-box matrix.** Each numeric cell below gives the physical low border edge, relative to the area's low edge, in CSS px. Run **every cell twice**, continuous and in a 500px fragmentainer; the two expected numbers are identical. Use a 100×100 area with physical low edge (40,40) in an accessible 500×500 page, LTR, zero padding/border/fixed margins. I = inline, B = block. Read x for horizontal I and vertical B, y otherwise. Auto mask is in logical order, so for vertical-rl B, start means right. The other axis has no auto margins and uses start. M-start/end are the expected used logical margins. U is the expected used size. Constraints N, min and max use a preferred length of U, U−20 and U+20 respectively; min/max clamp to U. These three independent sizing fixtures yield the same placement oracle. For plain stretch the definite-size fixture exercises fallback; the auto-size fixtures below exercise actual stretching. The fixtures use the same writing mode on item and container. For an orthogonal child keep these container-axis placement expectations and resolve its physical style constraints through its own axes.

In the positional columns each triple lists **omitted / safe / unsafe**. In the stretch column the triple lists **plain stretch / safe stretch / unsafe stretch**. A dash means the declaration is invalid and the cascade retains the preceding value; seed it with start and verify computed start plus the start-column geometry. Negative entries are expected signed coordinates.

| Mode | Axis | Auto sides | Constraint | F | U | M-start/end | Start | End | Center | Stretch |
|---|---|---|---|---:|---:|---|---|---|---|---|
| horizontal-tb | I | none | N | 40 | 60 | 0/0 | 0/0/0 | 40/40/40 | 20/20/20 | 0/—/— |
| horizontal-tb | I | none | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | I | none | N | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | I | none | min | 40 | 60 | 0/0 | 0/0/0 | 40/40/40 | 20/20/20 | 0/—/— |
| horizontal-tb | I | none | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | I | none | min | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | I | none | max | 40 | 60 | 0/0 | 0/0/0 | 40/40/40 | 20/20/20 | 0/—/— |
| horizontal-tb | I | none | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | I | none | max | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | I | start | N | 40 | 60 | 40/0 | 40/40/40 | 40/40/40 | 40/40/40 | 40/—/— |
| horizontal-tb | I | start | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | I | start | N | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | I | start | min | 40 | 60 | 40/0 | 40/40/40 | 40/40/40 | 40/40/40 | 40/—/— |
| horizontal-tb | I | start | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | I | start | min | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | I | start | max | 40 | 60 | 40/0 | 40/40/40 | 40/40/40 | 40/40/40 | 40/—/— |
| horizontal-tb | I | start | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | I | start | max | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | I | end | N | 40 | 60 | 0/40 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | I | end | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | I | end | N | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | I | end | min | 40 | 60 | 0/40 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | I | end | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | I | end | min | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | I | end | max | 40 | 60 | 0/40 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | I | end | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | I | end | max | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | I | both | N | 40 | 60 | 20/20 | 20/20/20 | 20/20/20 | 20/20/20 | 20/—/— |
| horizontal-tb | I | both | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | I | both | N | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | I | both | min | 40 | 60 | 20/20 | 20/20/20 | 20/20/20 | 20/20/20 | 20/—/— |
| horizontal-tb | I | both | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | I | both | min | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | I | both | max | 40 | 60 | 20/20 | 20/20/20 | 20/20/20 | 20/20/20 | 20/—/— |
| horizontal-tb | I | both | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | I | both | max | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | B | none | N | 40 | 60 | 0/0 | 0/0/0 | 40/40/40 | 20/20/20 | 0/—/— |
| horizontal-tb | B | none | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | B | none | N | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | B | none | min | 40 | 60 | 0/0 | 0/0/0 | 40/40/40 | 20/20/20 | 0/—/— |
| horizontal-tb | B | none | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | B | none | min | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | B | none | max | 40 | 60 | 0/0 | 0/0/0 | 40/40/40 | 20/20/20 | 0/—/— |
| horizontal-tb | B | none | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | B | none | max | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | B | start | N | 40 | 60 | 40/0 | 40/40/40 | 40/40/40 | 40/40/40 | 40/—/— |
| horizontal-tb | B | start | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | B | start | N | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | B | start | min | 40 | 60 | 40/0 | 40/40/40 | 40/40/40 | 40/40/40 | 40/—/— |
| horizontal-tb | B | start | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | B | start | min | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | B | start | max | 40 | 60 | 40/0 | 40/40/40 | 40/40/40 | 40/40/40 | 40/—/— |
| horizontal-tb | B | start | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | B | start | max | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | B | end | N | 40 | 60 | 0/40 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | B | end | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | B | end | N | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | B | end | min | 40 | 60 | 0/40 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | B | end | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | B | end | min | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | B | end | max | 40 | 60 | 0/40 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | B | end | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | B | end | max | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | B | both | N | 40 | 60 | 20/20 | 20/20/20 | 20/20/20 | 20/20/20 | 20/—/— |
| horizontal-tb | B | both | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | B | both | N | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | B | both | min | 40 | 60 | 20/20 | 20/20/20 | 20/20/20 | 20/20/20 | 20/—/— |
| horizontal-tb | B | both | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | B | both | min | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| horizontal-tb | B | both | max | 40 | 60 | 20/20 | 20/20/20 | 20/20/20 | 20/20/20 | 20/—/— |
| horizontal-tb | B | both | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| horizontal-tb | B | both | max | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | I | none | N | 40 | 60 | 0/0 | 0/0/0 | 40/40/40 | 20/20/20 | 0/—/— |
| vertical-lr | I | none | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | I | none | N | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | I | none | min | 40 | 60 | 0/0 | 0/0/0 | 40/40/40 | 20/20/20 | 0/—/— |
| vertical-lr | I | none | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | I | none | min | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | I | none | max | 40 | 60 | 0/0 | 0/0/0 | 40/40/40 | 20/20/20 | 0/—/— |
| vertical-lr | I | none | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | I | none | max | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | I | start | N | 40 | 60 | 40/0 | 40/40/40 | 40/40/40 | 40/40/40 | 40/—/— |
| vertical-lr | I | start | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | I | start | N | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | I | start | min | 40 | 60 | 40/0 | 40/40/40 | 40/40/40 | 40/40/40 | 40/—/— |
| vertical-lr | I | start | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | I | start | min | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | I | start | max | 40 | 60 | 40/0 | 40/40/40 | 40/40/40 | 40/40/40 | 40/—/— |
| vertical-lr | I | start | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | I | start | max | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | I | end | N | 40 | 60 | 0/40 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | I | end | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | I | end | N | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | I | end | min | 40 | 60 | 0/40 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | I | end | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | I | end | min | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | I | end | max | 40 | 60 | 0/40 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | I | end | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | I | end | max | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | I | both | N | 40 | 60 | 20/20 | 20/20/20 | 20/20/20 | 20/20/20 | 20/—/— |
| vertical-lr | I | both | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | I | both | N | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | I | both | min | 40 | 60 | 20/20 | 20/20/20 | 20/20/20 | 20/20/20 | 20/—/— |
| vertical-lr | I | both | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | I | both | min | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | I | both | max | 40 | 60 | 20/20 | 20/20/20 | 20/20/20 | 20/20/20 | 20/—/— |
| vertical-lr | I | both | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | I | both | max | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | B | none | N | 40 | 60 | 0/0 | 0/0/0 | 40/40/40 | 20/20/20 | 0/—/— |
| vertical-lr | B | none | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | B | none | N | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | B | none | min | 40 | 60 | 0/0 | 0/0/0 | 40/40/40 | 20/20/20 | 0/—/— |
| vertical-lr | B | none | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | B | none | min | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | B | none | max | 40 | 60 | 0/0 | 0/0/0 | 40/40/40 | 20/20/20 | 0/—/— |
| vertical-lr | B | none | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | B | none | max | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | B | start | N | 40 | 60 | 40/0 | 40/40/40 | 40/40/40 | 40/40/40 | 40/—/— |
| vertical-lr | B | start | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | B | start | N | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | B | start | min | 40 | 60 | 40/0 | 40/40/40 | 40/40/40 | 40/40/40 | 40/—/— |
| vertical-lr | B | start | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | B | start | min | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | B | start | max | 40 | 60 | 40/0 | 40/40/40 | 40/40/40 | 40/40/40 | 40/—/— |
| vertical-lr | B | start | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | B | start | max | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | B | end | N | 40 | 60 | 0/40 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | B | end | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | B | end | N | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | B | end | min | 40 | 60 | 0/40 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | B | end | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | B | end | min | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | B | end | max | 40 | 60 | 0/40 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | B | end | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | B | end | max | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | B | both | N | 40 | 60 | 20/20 | 20/20/20 | 20/20/20 | 20/20/20 | 20/—/— |
| vertical-lr | B | both | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | B | both | N | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | B | both | min | 40 | 60 | 20/20 | 20/20/20 | 20/20/20 | 20/20/20 | 20/—/— |
| vertical-lr | B | both | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | B | both | min | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-lr | B | both | max | 40 | 60 | 20/20 | 20/20/20 | 20/20/20 | 20/20/20 | 20/—/— |
| vertical-lr | B | both | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-lr | B | both | max | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-rl | I | none | N | 40 | 60 | 0/0 | 0/0/0 | 40/40/40 | 20/20/20 | 0/—/— |
| vertical-rl | I | none | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | I | none | N | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-rl | I | none | min | 40 | 60 | 0/0 | 0/0/0 | 40/40/40 | 20/20/20 | 0/—/— |
| vertical-rl | I | none | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | I | none | min | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-rl | I | none | max | 40 | 60 | 0/0 | 0/0/0 | 40/40/40 | 20/20/20 | 0/—/— |
| vertical-rl | I | none | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | I | none | max | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-rl | I | start | N | 40 | 60 | 40/0 | 40/40/40 | 40/40/40 | 40/40/40 | 40/—/— |
| vertical-rl | I | start | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | I | start | N | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-rl | I | start | min | 40 | 60 | 40/0 | 40/40/40 | 40/40/40 | 40/40/40 | 40/—/— |
| vertical-rl | I | start | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | I | start | min | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-rl | I | start | max | 40 | 60 | 40/0 | 40/40/40 | 40/40/40 | 40/40/40 | 40/—/— |
| vertical-rl | I | start | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | I | start | max | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-rl | I | end | N | 40 | 60 | 0/40 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | I | end | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | I | end | N | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-rl | I | end | min | 40 | 60 | 0/40 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | I | end | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | I | end | min | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-rl | I | end | max | 40 | 60 | 0/40 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | I | end | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | I | end | max | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-rl | I | both | N | 40 | 60 | 20/20 | 20/20/20 | 20/20/20 | 20/20/20 | 20/—/— |
| vertical-rl | I | both | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | I | both | N | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-rl | I | both | min | 40 | 60 | 20/20 | 20/20/20 | 20/20/20 | 20/20/20 | 20/—/— |
| vertical-rl | I | both | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | I | both | min | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-rl | I | both | max | 40 | 60 | 20/20 | 20/20/20 | 20/20/20 | 20/20/20 | 20/—/— |
| vertical-rl | I | both | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | I | both | max | -20 | 120 | 0/0 | 0/0/0 | -20/0/-20 | -10/0/-10 | 0/—/— |
| vertical-rl | B | none | N | 40 | 60 | 0/0 | 40/40/40 | 0/0/0 | 20/20/20 | 40/—/— |
| vertical-rl | B | none | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | B | none | N | -20 | 120 | 0/0 | -20/-20/-20 | 0/-20/0 | -10/-20/-10 | -20/—/— |
| vertical-rl | B | none | min | 40 | 60 | 0/0 | 40/40/40 | 0/0/0 | 20/20/20 | 40/—/— |
| vertical-rl | B | none | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | B | none | min | -20 | 120 | 0/0 | -20/-20/-20 | 0/-20/0 | -10/-20/-10 | -20/—/— |
| vertical-rl | B | none | max | 40 | 60 | 0/0 | 40/40/40 | 0/0/0 | 20/20/20 | 40/—/— |
| vertical-rl | B | none | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | B | none | max | -20 | 120 | 0/0 | -20/-20/-20 | 0/-20/0 | -10/-20/-10 | -20/—/— |
| vertical-rl | B | start | N | 40 | 60 | 40/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | B | start | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | B | start | N | -20 | 120 | 0/0 | -20/-20/-20 | 0/-20/0 | -10/-20/-10 | -20/—/— |
| vertical-rl | B | start | min | 40 | 60 | 40/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | B | start | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | B | start | min | -20 | 120 | 0/0 | -20/-20/-20 | 0/-20/0 | -10/-20/-10 | -20/—/— |
| vertical-rl | B | start | max | 40 | 60 | 40/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | B | start | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | B | start | max | -20 | 120 | 0/0 | -20/-20/-20 | 0/-20/0 | -10/-20/-10 | -20/—/— |
| vertical-rl | B | end | N | 40 | 60 | 0/40 | 40/40/40 | 40/40/40 | 40/40/40 | 40/—/— |
| vertical-rl | B | end | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | B | end | N | -20 | 120 | 0/0 | -20/-20/-20 | 0/-20/0 | -10/-20/-10 | -20/—/— |
| vertical-rl | B | end | min | 40 | 60 | 0/40 | 40/40/40 | 40/40/40 | 40/40/40 | 40/—/— |
| vertical-rl | B | end | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | B | end | min | -20 | 120 | 0/0 | -20/-20/-20 | 0/-20/0 | -10/-20/-10 | -20/—/— |
| vertical-rl | B | end | max | 40 | 60 | 0/40 | 40/40/40 | 40/40/40 | 40/40/40 | 40/—/— |
| vertical-rl | B | end | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | B | end | max | -20 | 120 | 0/0 | -20/-20/-20 | 0/-20/0 | -10/-20/-10 | -20/—/— |
| vertical-rl | B | both | N | 40 | 60 | 20/20 | 20/20/20 | 20/20/20 | 20/20/20 | 20/—/— |
| vertical-rl | B | both | N | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | B | both | N | -20 | 120 | 0/0 | -20/-20/-20 | 0/-20/0 | -10/-20/-10 | -20/—/— |
| vertical-rl | B | both | min | 40 | 60 | 20/20 | 20/20/20 | 20/20/20 | 20/20/20 | 20/—/— |
| vertical-rl | B | both | min | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | B | both | min | -20 | 120 | 0/0 | -20/-20/-20 | 0/-20/0 | -10/-20/-10 | -20/—/— |
| vertical-rl | B | both | max | 40 | 60 | 20/20 | 20/20/20 | 20/20/20 | 20/20/20 | 20/—/— |
| vertical-rl | B | both | max | 0 | 100 | 0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/—/— |
| vertical-rl | B | both | max | -20 | 120 | 0/0 | -20/-20/-20 | 0/-20/0 | -10/-20/-10 | -20/—/— |

**Auto sizing and named historical cells.** Each row runs in a fresh DOM in both modes; the same expected value applies to each. Rows specify physical properties unless otherwise marked. P2's earlier fragmentation inventory remains the oracle when the item spans multiple fragments.

| Fixture | Expected used geometry in continuous and fragmented layout |
|---|---|
| 300px-wide grid, fixed 100px area, auto-width item with a 100px-wide child, left auto / both inline auto, default stretch | Width 100, left/right margins 0/0, local x=0 in both cases. Explicit start, no auto margins, has the same width. |
| Area 100, preferred auto, true intrinsic range 40…80, both inline auto, end | Width 80, margins 10/10, x=10. Without auto margins, explicit stretch gives width 100, x=0. |
| Area x=40…140, width 120, left auto, end | Margin 0; absolute x=20 for omitted and unsafe, x=40 for safe. The end edges are 140 and 160 respectively. |
| Area x=0…100, width 40, block end/start, vertical-rl | End x=0, start x=60. Vertical-lr control gives end x=60, start x=0. |
| Auto stretch, max-width 40 in 100 | Width 40. Start justification and both inline margins auto gives width 40, used margins 30/30, x=30. |
| Auto stretch, min-width 120 in 100 | Width 120, zero auto margins. Vertical-lr inline stretch with max-height 70 in 100 gives height 70. |
| Auto block item, child height 10, child margin-bottom 12 | Item’s own bottom margin 0. At natural height 22 with both block margins auto, y=39 in 100. At explicit height 50, margins 25/25 and y=25, including end alignment. |
| 100px row, auto stretch height, 10px padding top and bottom; border-box / content-box | Content height 80, border-box height 100; computed height remains auto in all four sizing-mode/page combinations. |
| 50% × 50% item in 100×100 area | Width and height 50, independent of container width. |
| Two axes simultaneously, vertical-rl LTR, 40×60 item, both block margins auto, inline end | Physical (x,y)=(30,40); RTL changes y to 0. With orthogonal horizontal-tb child, the same physical dimensions give the same placement. |
| Two rows of width 100 in a vertical-rl grid of width 200, 40px item at block-end in the second row | Second row occupies x=0…100; item x=0. In vertical-lr it occupies x=100…200; item x=160. |

Use an expectation table independent of the production resolver. Assert used margin values as well as the published child and descendant rectangles so compensating negative margins cannot satisfy an offset test. Test invalid safety/keyword syntax at the property handler; exercise valid and omitted safety through CSS cascade and the ordinary Grid entry. Re-run the existing P1 comment/source cases and the P2 count, paint, resume and 80px fixtures. Implementation starts after PM disposition of the five-axis plan review; Stage 1–2 and an independent geometry re-check precede external PR review.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-grid-2 §7.2.3.2 Repeat-to-fill: auto-fill and auto-fit repetitions | determine track count | one and two columns, fixed explicit control | track expansion → placement | auto-fill regression | yes |
| css-break-3 §2.1 Parallel Fragmentation Flows | complete all admitted siblings and preserve each child continuation | two interrupted children, deferred continuation, finished sibling | definitive Grid publication and keyed child layout | parallel items | yes |
| css-break-3 §4.3 Forced Breaks | combine applicable values | force overrides avoid | Grid adapter and shared chooser | row fixtures | yes |
| css-break-3 §4.4 Unforced Breaks | rank admissible boundaries and relax for progress | ordinary overflow and avoid retreat | shared chooser and selected intervals | row fixtures | yes |
| css-grid-2 §13 Fragmenting Grid Layout | choose break and preserve unfinished child | row boundary, spanning child | Grid fragmentation producer and child layout reader | supported fragmentation | yes |
| css-break-3 §5.1 Breaking into Varying-size Fragmentainers | resume progress | narrower, wider and constant geometry | shared cursor at layout entry | supported page sizes | yes |
| css-grid-2 §6.1 Grid Item Display | establish item context | block item and subgrid | item context and inner formatter | independent/subgrid | yes |
| css-grid-2 §6.2 Grid Item Sizing | choose auto sizing | fit-content, stretch, normal/replaced | area probe and shared formatter | definite/auto | yes |
| css-sizing-3 §2 Terminology: fit-content size | bound available space by intrinsic sizes | below min, between, above max | intrinsic measurement and sizing | three available-space bands | yes |
| css-sizing-3 §3.3 Box Sizing | resolve preferred size and constraints | content-box, border-box, min/max | shared formatter | both sizing modes | yes |
| css-grid-2 §11.2 Aligning with auto margins | distribute positive free space | neither, one or both auto; zero or deficit | formatter margins then Grid placement | both axes | yes |
| css-align-3 §4.4.1.3 All Other Alignment | place overflowing subject | omitted, explicit safe or unsafe; reachable and unreachable edge | style resolution and signed Grid offset | matrix plus scroll-edge control | yes |
| css-writing-modes-4 §3.2 Block Flow Direction | map logical axis and origin | horizontal-tb, vertical-lr, vertical-rl | admitted area and physical placement | both inline directions | yes |

**Breadth and review boundary.** The cumulative map spans five specs and thirteen rows. Keep the addendum with P2 for PM review: sizing produces the used margin box that signed alignment consumes, and writing-mode conversion and page placement publish that same box. A split between these steps would leave the three measured cross-boundary regressions without an end-to-end acceptance test on the implementation PR. The separate 3b.2g width-change plan keeps its existing boundary. The PM decides whether to retain this per-PR scope at plan review.

Specification anchors for the additional rows: [Grid item display](https://www.w3.org/TR/css-grid-2/#grid-item-display), [Grid item sizing](https://www.w3.org/TR/css-grid-2/#grid-item-sizing), [Grid auto margins](https://www.w3.org/TR/css-grid-2/#auto-margins), [fit-content size](https://www.w3.org/TR/css-sizing-3/#fit-content-size), [box sizing](https://www.w3.org/TR/css-sizing-3/#box-sizing), [overflow alignment](https://drafts.csswg.org/css-align-3/#overflow-values), [block flow direction](https://www.w3.org/TR/css-writing-modes-4/#block-flow).

Extend elidex-layout-grid/src/tests/fragmentation.rs through the normal Grid entry. Build the width-changing checks on the same-width dispatch and emitted-suffix assertions assigned to P2 in the §1 test inventory. Run Grid and layout all-feature tests.
