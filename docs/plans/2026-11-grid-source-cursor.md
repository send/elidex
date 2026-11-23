# Preserve Grid item progress when page width changes track membership

3b.2g under PM 1010. The 3b.2f plan landed at c4b05a9c. L1 owns this child after 3b.2f and before the pagination plan. It consumes the PM 1012 consumed-interval and typed-token contract established in 3b.2.

## §1. Grid placement and content progress

At c4b05a9c elidex-layout-grid/src/lib.rs expands auto-repeat against the current inline size, then places items using that column count. compute_grid_fragmentation applies a previously saved row_index to the new row tracks. Increasing or decreasing the column count changes the items that precede that row. A width increase can also reduce the new row count below the saved index. Admission and track placement continue to use the existing Grid algorithms.

Commit only the chosen break’s consumed source intervals, keyed by formatting owner and item, to the run-owned token. Carry committed coverage through regrouping and select each item's remaining contribution from its complement. Include the terminal boundary when an item finishes, including an empty item. A partially consumed item retains its covered ranges alongside wholly consumed siblings. Recompute track placement from the full input at the new geometry; determine eligibility for fragment output from that item's coverage. Dense and explicit placement can produce disjoint coverage across items. Preserve it independently of recomputed row membership.

Use 3b.2’s explicit token alternatives at the child boundary. A clean boundary continues selection among remaining items. For each interrupted spanning child keep its stable entity and actual nested token; route it back to that child even if several children span the break. A local run continuation inside a child is interpreted by its own formatter. This routing conveys how to resume the child; the owner/item interval set remains authoritative for what has been consumed.

### Select the break before publishing item positions

At this head lib.rs calls position_items before compute_grid_fragmentation. Keep track expansion, item-to-cell placement and sizing available to boundary selection. Adapt Grid row boundaries to the common chooser in elidex-layout-block (find_best_break and its shared forced-boundary handling from 3b.2f). Aggregate applicable values over all items starting or ending at the boundary and associate each candidate with its own proposed coverage. Propagate first/last row values to the container as required by Grid §13. Among unforced fitting boundaries prefer an allowed boundary to one with applicable avoid; relax in the shared chooser when progress requires it. A forced boundary takes precedence over avoid. A forced break *inside* a spanning item splits that item’s content independently of siblings.

| Boundary fixture | Expected winner and published intervals |
|---|---|
| Two rows AB and CD both fit; only D requests break-before:page | Break before the CD row, commit A/B only. |
| Single-item rows A/B/C, room for two, all auto | Choose B❘C and commit A/B. |
| Same rows, B❘C has avoid and A❘B is allowed | Choose A❘B. The current generation contains A only. |
| B❘C has both forced and avoid values | Choose B❘C and commit A/B. |

Once the boundary is selected, position only the admitted items and child fragments. In position.rs the definitive child call currently receives neither a fragmentainer nor a token and retains only layout_box. Give this call the selected fragment’s remaining block room and the incoming token matched to the item (None on first entry). Keep the child’s actual fragment extent and outgoing token from this single definitive call. Commit its partial intervals with the interrupted-child alternative only when it contributes to this fragment. Replace the synthetic track-height tokens in collect_spanning_item_breaks with those child outcomes. Track extents still describe geometry. Preliminary positioning, intrinsic sizing and rejected candidates must publish neither progress nor generation-visible boxes. Obtain the fragment baseline from admitted output.

## §2. Crossing checks

| Crossing | Producer → consumer | Distinguishing observation |
|---|---|---|
| Auto-repeat × progress | Auto-placement at one width → next-width fragment | Use repeat(auto-fill, 40px), zero gap, row flow, four labeled equal-height items A–D and room for one row. At 80px AB fit in the first row; at 40px only A fits. Run both 80→40 and 40→80 through actual layout continuations. Capture emitted entities on every fragment and require A–D once each, with C and B respectively the first remaining item. Repeat with fixed explicit two-column placement, whose membership stays stable. |
| Spanning item × row reflow | Interrupted child → re-placed item | Start without an input token. Place an item over multiple rows with identifiable child text P and Q separated by a forced internal break; give its sibling unbroken content. Inspect the actual child token and partial coverage returned after P. Change the width, then assert that the suffix Q is emitted once at child-local inline offset zero, sibling contributions are preserved and no completed child restarts. Assert the interrupted-child alternative and its nested content coverage; compare with a clean boundary for a nonspanning item and repeat at constant width. |
| Placement order × replay | Dense/explicitly placed items → consumed intervals | With source order A/B/C, place B in row one and A/C in row two. After B completes, assert B’s interval and terminal boundary are covered while A/C are untouched. Resume with A/C only. Repeat with dense placement filling an earlier hole. At the boundary record which entities actually finished and which span it. Replaying at the new width must preserve disjoint progress even when row membership differs. Include a shrink in row count beyond the old row index and require termination. |

For the auto-repeat fixtures capture item IDs, terminal/partial intervals, and page-local positions before the next layout overwrites them. Run the count sequence using actual outgoing tokens; save the input token and geometry for every page. Replay these inputs in fresh paint-layout generations. Compare each pass independently against expected cell membership and coordinates derived from current track placement and the remaining intervals. In particular, at 40→80 the new two-column placement has A/B in row one and C/D in row two: A was consumed, so B retains its column offset of 40px. At 80→40 the first remaining item C is in the sole column at inline offset zero. Verify previously consumed and rejected items are absent from each page’s output, the final complement is empty and no extra page is emitted. Also verify the avoid-retreat case leaves no box for B in the rejected generation.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-grid-2 §7.2.3.2 Repeat-to-fill: auto-fill and auto-fit repetitions | determine track count | one and two columns, fixed explicit control | track expansion → placement | auto-fill regression | yes |
| css-break-3 §4.3 Forced Breaks | combine applicable values | force overrides avoid | Grid adapter and shared chooser | row fixtures | yes |
| css-break-3 §4.4 Unforced Breaks | rank admissible boundaries and relax for progress | ordinary overflow and avoid retreat | shared chooser and selected intervals | row fixtures | yes |
| css-grid-2 §13 Fragmenting Grid Layout | choose break and preserve unfinished child | row boundary, spanning child | Grid fragmentation producer and child layout reader | supported fragmentation | yes |
| css-break-3 §5.1 Breaking into Varying-size Fragmentainers | resume progress | narrower, wider and constant geometry | shared cursor at layout entry | supported page sizes | yes |

Extend elidex-layout-grid/src/tests/fragmentation.rs through the normal Grid entry. Its existing resume_with_child_break_tokens test only requires completion; read emitted content and the new progress as well. Run Grid and layout all-feature tests.
