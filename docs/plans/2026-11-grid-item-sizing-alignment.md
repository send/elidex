# PR-GA — Grid item sizing and alignment

## §1. Landing and acceptance boundary

PM 1141 places this slice before the combined Block/Grid fragmentation PR #785. Implement on a fresh branch from origin/main. PR-GA owns authored self-alignment through the real child dispatch and the published rectangle in ordinary Grid layout. Run its geometry fixtures through continuous layout and wholly admitted 500px pages. P2 reruns that suite and owns actual interruption, source contribution and replay. Review this plan together with the page-source-cursor umbrella delta and the Grid source cursor’s P2 contract.

## §2. From authored style to a placed item

1. **Preserve intent.** Parse, cascade and serialize each element’s own alignment keyword and overflow modifier. The child’s computed justify-self and align-self preserve auto; the parent’s computed justify-items and align-items retain their own keyword and modifier. At Grid item collection, resolve auto to the corresponding parent items pair for layout only. An explicit child self value supplies its own pair, including omitted safety. Keep the logical auto-margin mask even when resolved raw lengths are zero. Preserve baseline-group handling and fallback. Audit CSS Flex/Grid property handlers and style resolve/flex.rs and resolve/grid.rs. Test invalid modifier/keyword syntax against a seeded valid declaration. Content-distribution safety has a separate consumer.
2. **Measure in the item’s actual area.** The intrinsic producer supplies **inner** min-content and max-content sizes on the fit axis. Child content, including its relevant outer edges, participates in those measurements. The measured item’s own margin, border and padding belong to its outer track contribution, a separate output of track sizing. Never substitute an auto-width layout at the container width for its max-content size. Resolve fixed margins and decorations against the grid area once. The common formatter owns the content-space budget Q = max(0, area − fixed margins − borders − padding); auto margins contribute zero here. For a definite available size use max(inner min-content, min(Q, inner max-content)) for fit-content. With inner range 40…140, budgets 20/80/180 give 40/80/140. Auto stretch without auto margins uses Q; an auto margin on that axis suppresses growth but retains the authored alignment. Measure a flow item’s natural block size at the resolved inline size. For an auto-width replaced item with natural dimensions, normal alignment uses natural sizing; explicit inline stretch fits the available width subject to constraints and can change the natural ratio when height is definite.
3. **Resolve the box once.** The formatter resolves preferred dimensions, percentages and min/max relative to their specified references, converting specified border-box values to content sizes once. Clamp the preferred size, with min winning over a conflicting max. Distribute positive remaining area space to one or both auto margins; at zero or a deficit each auto side is zero. Return used size and margins to Grid. An ordinary block item establishes an independent context for both measurement and publication. Block, Flex, Grid, Table and Multicol item dispatch must agree on the outer-box contract. Computed style stays immutable.
4. **Select subgrid overrides by track axis first.** In each subgridded axis, fit the subgrid to its spanned parent tracks, ignore the item’s preferred/min/max size constraints and self-alignment on that axis and use the parent lines. Its content-distribution alignment cannot move those inherited lines. Account for its edges under Grid’s subgrid rules. In a standalone axis of the very same item use the ordinary sizing, used-margin and alignment sequence. Determine column/row participation from the parent track mapping, including orthogonal writing modes. Exercise column-only, row-only, both and neither on both probe and definitive calls.
5. **Produce reachable scroll geometry during layout.** The layout_tree entry (or layout_fragmented_with_tokens for paged layout) seeds the initial scrollport from the current layout viewport or page input, in its own unscrolled physical coordinate system. Each ancestor formatter that establishes a scroll container for the axis supplies that scrollport’s reachable start-side boundary from its current padding-box geometry and scroll origin, together with its physical axis direction. Carry the nearest boundary per axis in the pass-local child input. Carry the reachable edge and the actual candidate area origin in the same physical root/page layout coordinates. Accumulate child-to-root translations on descent. A probe measuring at local zero uses the candidate area’s saved translation for placement tests; convert its final displacement back to local coordinates once. A non-scroller preserves the inherited constraint. Use layout coordinates before scrolling/paint transforms. Reversed progression protects the physical high edge. The boundary is the unreachable-side limit, not a clip at both viewport edges: overflow can extend the scrollable end. Derive the origin and relevant start edge from the current ancestor sizing pass before positioning children. Where a reversed or auto-sized ancestor needs measured extent, measure it in a non-publishing sizing pass, finalize its padding-box frame, then run placement with that frame; never infer reachability from the placed item’s own overflow. Intrinsic-only probes need no placement. Probes that return placement and definitive dispatch receive the same finalized frame for the same input; on a new page or resize derive a new frame. No previous-generation LayoutBox, paint bounds or global ECS cache supplies this input.
6. **Align after used margins.** In logical coordinates, start/end/center consume zero/all/half the residual signed space. A positive surplus absorbed by auto margins leaves no alignment displacement. A deficit leaves the full signed displacement for the requested keyword. Explicit safe overflow falls back to flow-start; explicit unsafe keeps the requested position. For omitted safety first compute the requested placement, then bias it only as required by the nearest unreachable edge per CSS Align §4.4.1.3. Test identical areas under distinct ancestor scrollports, including explicit safe/unsafe controls, on initial layout and after moving the nested scroller. Apply the stretch fallback to definite or constrained boxes.
7. **Map the logical margin-box offset to physical layout coordinates once.** Forward low edge = area-low + logical offset; reversed low edge = area-low + area-extent − margin-box extent − logical offset. Add the physical low margin and decoration once to obtain the content origin. Use the container axes for placement, including the area origin, and the item axes for sizing its contents. In horizontal-tb block advances down, vertical-lr right, vertical-rl left; RTL reverses inline progression. The same resolved area and used metrics drive non-publishing measurement and the one definitive dispatch. P2 consumes the resulting placement and the logical block-entry distance separately.

At review base 780ac30e, the integration prototype uses Grid helpers.rs to collect items, position.rs for area measurement and placement, and Block sizing.rs and LayoutInput for constrained sizing. Inventory the equivalent entry points on origin/main before implementation. Implement the ordinary path there; keep the P2 continuation protocol in #785. Real-dispatch checks assert the item’s used margins and its descendants’ origins as well as its published rectangle.

**Coupled invariants.** The matrix below crosses margin mask (M), keyword/safety (K), free-space sign (F), writing mode (W), constraints (C) and pagination (P). Pair checks: M×K retain keyword; M×F distribute only surplus; M×W map each auto side; M×C distribute after clamp; M×P keep used margins page-stable. K×F permit signed offsets; K×W map logical end once; K×C align constrained size; K×P preserve first-piece alignment. F×W reverse signed overflow as well as positive offsets; F×C derive sign after clamp; F×P distinguish overflow displacement from available page room. W×C map physical min/max to the correct axis; W×P preserve column progression in reversed block flow; C×P keep the same constrained reference on first dispatch.

**Published-box matrix.** Each numeric cell below gives the physical low border edge, relative to the area's low edge, in CSS px. Run **every cell twice**, continuous and in a 500px fragmentainer; the two expected numbers are identical. Use a 100×100 area with physical low edge (40,40) in an accessible 500×500 page, LTR, zero padding/border/fixed margins. I = inline, B = block. Read x for horizontal I and vertical B, y otherwise. Auto mask is in logical order, so for vertical-rl B, start means right. The other axis has no auto margins and uses start. M-start/end are the expected used logical margins. U is the expected used size. Constraints N, min and max use a preferred length of U, U−20 and U+20 respectively; min/max clamp to U. These three independent sizing fixtures yield the same placement oracle. For plain stretch the definite-size fixture exercises fallback; the auto-size fixtures below exercise actual stretching. The fixtures use the same writing mode on item and container. For an orthogonal child keep these container-axis placement expectations and resolve its physical style constraints through its own axes.

In the positional columns each triple lists **omitted / safe / unsafe**. In the stretch column the triple lists **plain stretch / safe stretch / unsafe stretch**. A dash means the declaration is invalid and the cascade retains the preceding value; seed it with start and verify computed start plus the start-column geometry. Negative entries are expected signed coordinates. These matrix cells exercise explicit child keywords; the auto controls below observe both computed serialization and used placement.

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
| Auto stretch, max-width 40 in 100, no auto margins | Width 40, x=0. |
| Start, both inline margins auto, max-width 40, fixed-width 80px child in 100 | Width 40, margins 30/30, x=30. |
| Same declarations, empty item or empty auto-width block child with only height and bottom margin | Inner intrinsic width 0, item width 0, margins 50/50, x=50. |
| Auto stretch, min-width 120 in 100 | Width 120, zero auto margins. Vertical-lr inline stretch with max-height 70 in 100 gives height 70. |
| Auto block item, child height 10, child margin-bottom 12 | Item’s own bottom margin 0. At natural height 22 with both block margins auto, y=39 in 100. At explicit height 50, margins 25/25 and y=25, including end alignment. |
| 100px row, auto stretch height, 10px padding top and bottom; border-box / content-box | Content height 80, border-box height 100; computed height remains auto in all four sizing-mode/page combinations. |
| 50% × 50% item in 100×100 area | Width and height 50, independent of container width. |
| Two axes simultaneously, vertical-rl LTR, 40×60 item, both block margins auto, inline end | Physical (x,y)=(30,40); RTL changes y to 0. With orthogonal horizontal-tb child, the same physical dimensions give the same placement. |
| Two rows of width 100 in a vertical-rl grid of width 200, 40px item at block-end in the second row | Second row occupies x=0…100; item x=0. In vertical-lr it occupies x=100…200; item x=160. |
| Start, area 100, auto width, fixed 40px child, 10px inline padding on each side | Inner min/max 40/40, budget 80, content width 40, border width 60. Adding fixed margins 5/15 gives budget 60, width 40, outer width 80. Stretch with these same margins and padding yields content 60, border width 80, outer width 100. |
| Omitted end, area x=40…140, item width 120, left margin auto, nearest reachable left edge 0 / 40 | Root case x=20, nested scroller case x=40; used left margin 0 in both. Explicit unsafe x=20 in both, explicit safe x=40 in both. |
| Horizontal-tb LTR, 100px column, zero edges/margins, parent justify-items: safe end, child justify-self: auto | Computed parent value is safe end and computed child value is auto at both widths. Real dispatch puts a 40px-wide child at local x=60 and a 120px-wide child at x=0. |
| Same controls on the block axis, 100px row, parent align-items: safe end, child align-self: auto | Computed parent value is safe end and computed child value is auto at both heights. Real dispatch puts a 40px-high child at local y=60 and a 120px-high child at y=0. |
| Override either self property above with unsafe end on the 120px child | Computed child value is unsafe end and computed parent value remains safe end. Local coordinate on the tested axis is −20. |
| Replaced image, natural 60×30, auto width, definite 30px height, zero margins/edges, 100px column | With justify-self: normal, content is 60×30; with explicit stretch, content is 100×30. |
| Column-only subgrid, 100×100 area, width 40 and max-width 50, height 40, both self keywords end, zero edges | Width 100 at x=0, height 40 at y=60. Row-only mirror with height 40 and max-height 50 gives height 100 at y=0 and width 40 at x=60. Both axes subgridded gives 100×100 at (0,0). Probe and published boxes agree. |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-grid-2 §6.1 Grid Item Display | establish item context | block item and subgrid | item context and inner formatter | independent/subgrid | yes |
| css-grid-2 §6.2 Grid Item Sizing | choose auto sizing | fit-content, replaced normal/natural versus explicit stretch | area probe and shared formatter | definite/auto | yes |
| css-sizing-3 §2 Terminology: fit-content size | bound available space by intrinsic sizes | below min, between, above max | intrinsic measurement and sizing | three available-space bands | yes |
| css-sizing-3 §3.3 Box Sizing | resolve preferred size and constraints | content-box, border-box, min/max | shared formatter | both sizing modes | yes |
| css-grid-2 §11.2 Aligning with auto margins | distribute positive free space | neither, one or both auto; zero or deficit | formatter margins then Grid placement | both axes | yes |
| css-align-3 §6.1 Inline-Axis Self-Alignment | compute justify-self then resolve used pair | auto or explicit keyword/modifier | CSSOM serialization → Grid collection | parent/child and x controls | yes |
| css-align-3 §6.2 Block-Axis Self-Alignment | compute align-self then resolve used pair | auto or explicit keyword/modifier | CSSOM serialization → Grid collection | parent/child and y controls | yes |
| css-align-3 §4.4.1.3 All Other Alignment | place overflowing subject | omitted, explicit safe or unsafe; reachable and unreachable edge | style resolution and signed Grid offset | matrix plus scroll-edge control | yes |
| css-writing-modes-4 §3.2 Block Flow Direction | map logical axis and origin | horizontal-tb, vertical-lr, vertical-rl | admitted area and physical placement | both inline directions | yes |
| css-grid-2 §9 Subgrids | select sizing and alignment override | column, row, both or neither | parent track mapping → subgrid item dispatch | per-axis fixtures | yes |
| css-box-4 §3 Margins | form content-space budget | fixed and auto margins, decorated item | inner measurement → used box | fit and stretch | yes |

PM 1142 accepts the five-spec breadth recommendation and keeps PR-GA as a terminal slice under the umbrella: its complete contract is observable at one real dispatch boundary. PR-GA supplies the acceptance suite; P2 supplies the interruption crossing after PR-GA lands. [Inline self-alignment](https://drafts.csswg.org/css-align-3/#justify-self-property), [block self-alignment](https://drafts.csswg.org/css-align-3/#align-self-property), [Grid item sizing](https://drafts.csswg.org/css-grid-2/#grid-item-sizing), [subgrid axes](https://drafts.csswg.org/css-grid-2/#subgrids), [auto margins](https://drafts.csswg.org/css-grid-2/#auto-margins), [intrinsic size contributions](https://drafts.csswg.org/css-sizing-3/#intrinsic-size-contribution), [margins](https://drafts.csswg.org/css-box-4/#margins), [overflow alignment](https://drafts.csswg.org/css-align-3/#overflow-values), [block flow](https://drafts.csswg.org/css-writing-modes-4/#block-flow).
