# Preserve Grid item progress when page width changes track membership

3b.2g under PM 1010. The 3b.2f plan landed at c4b05a9c. L1 owns this child after 3b.2f and before the pagination plan. It consumes the PM 1012 consumed-interval and typed-token contract established in 3b.2.

## §1. Grid placement and content progress

### P2 prerequisite and retained 3b.2g scope (PM 1115)

P2 follows P1's shared layout outcome and precedes 3b.2f. At unchanged width, choose the admitted rows with the existing Grid break policy before publishing item positions. Keep sizing probes isolated from definitive output. For each admitted item fragment make one definitive child call with the fragmentainer space still available at its entry and the incoming continuation matched by stable identity. On first entry pass no continuation. Publish the child's actual fragment extent, source contribution and real outgoing continuation. On same-width resumption route that exact continuation back to its child. A completed sibling stays complete and independent interruptions retain separate identities. Remove `collect_spanning_item_breaks` and its synthetic track-height tokens in P2. One producer owns each interruption from its first dispatch through its final contribution.

At fixed width, force a split between a child's source boxes P and Q. On entry one observe P alone, Q untouched, the actual child extent and its nested token; on entry two observe that same token at dispatch, Q alone, no replay of P or completed siblings, and completion with an empty complement. Give P a different height from the crossed track so geometric synthesis fails the test. A child that spans tracks but completes on its first call yields no child continuation.

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

In the parallel fixture, interrupt two spanning children X and Y at distinct interior breaks x1❘x2 and y1❘y2. Have a later admitted sibling Z finish after X interrupts. Require the first page to emit x1, y1 and all of Z. Inspect the actual extent of each fragment against its fixture content (one line for each prefix, two for Z), Z’s terminal boundary, and both nested tokens keyed to their entities. Change the width so that only X is admitted on page two. Require only x2 at child-local inline offset zero, completion of X, and the original Y token and covered prefix unchanged in the outgoing state. Admit Y on page three: require only y2 at child-local offset zero, empty complement and no outgoing token. Check the distinct interval sets, actual IDs, positions and sibling absence on each page in both passes.

Separately, fit the entire grid into one row with two nonspanning items. One forces a break between P and Q; the sibling finishes on page one. All tracks fit, but the actual child outcome leaves Q. Assert an outgoing token keyed to that item and exactly Q on page two, with the completed sibling absent and no further continuation.

For each fixture drive counting layout with real outgoing tokens, capture emissions before overwrite and save input tokens and widths. Replay the saved inputs in fresh paint-layout generations. Pass a distinct nonzero layout generation per page and pass into do_layout_grid_fragmented; query boxes visible to that generation for every presence and absence assertion. Compare count and paint independently with literal expectations. For the width fixtures use one row of block room with zero gaps, margins, borders and padding. Coordinates below are (inline, block) relative to the page content origin.

| Column setup and widths | Expected page-local output |
|---|---|
| Auto-fill, 80→40→40 | A(0,0) B(40,0) / C(0,0) / D(0,0) |
| Auto-fill, 40→80→80 | A(0,0) / B(40,0) / C(0,0) D(40,0) |
| Two fixed 40px columns, 80→40 and 40→80 | A(0,0) B(40,0) / C(0,0) D(40,0) in either run, including when the second column overflows the narrow page |

Check cumulative coverage, absence of consumed and rejected items, and an empty complement on the stated final page. The avoid-retreat fixture likewise requires B to have no visible box in the generation that admitted only A.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-grid-2 §7.2.3.2 Repeat-to-fill: auto-fill and auto-fit repetitions | determine track count | one and two columns, fixed explicit control | track expansion → placement | auto-fill regression | yes |
| css-break-3 §2.1 Parallel Fragmentation Flows | complete all admitted siblings and preserve each child continuation | two interrupted children, deferred continuation, finished sibling | definitive Grid publication and keyed child layout | parallel items | yes |
| css-break-3 §4.3 Forced Breaks | combine applicable values | force overrides avoid | Grid adapter and shared chooser | row fixtures | yes |
| css-break-3 §4.4 Unforced Breaks | rank admissible boundaries and relax for progress | ordinary overflow and avoid retreat | shared chooser and selected intervals | row fixtures | yes |
| css-grid-2 §13 Fragmenting Grid Layout | choose break and preserve unfinished child | row boundary, spanning child | Grid fragmentation producer and child layout reader | supported fragmentation | yes |
| css-break-3 §5.1 Breaking into Varying-size Fragmentainers | resume progress | narrower, wider and constant geometry | shared cursor at layout entry | supported page sizes | yes |

Extend elidex-layout-grid/src/tests/fragmentation.rs through the normal Grid entry. Build the width-changing checks on the same-width dispatch and emitted-suffix assertions assigned to P2 in the §1 test inventory. Run Grid and layout all-feature tests.
