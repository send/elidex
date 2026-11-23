# Resume wrapped Flex items across page widths

3b.2f under PM 1009. The 3b.2 plan landed at cee0efc6. Implement after 3b.2 establishes the PM 1012 consumed-interval and typed-token contract. Grid follows this child. The contract is observable at the Flex layout boundary before the later pagination plan replays whole pages.

## §1. Commit consumed source intervals at the chosen break

At cee0efc6 elidex-layout-flex/src/lib.rs reads line_index and item_index, collects the items in order-modified document order, and rebuilds line_ranges against the new container main size. fragment.rs::fragment_row_flex then starts at the saved line number. In fragment_column_flex the saved item is relative to a line. For each forced break or overflow/avoid candidate retain the covered source intervals keyed by formatting owner and item. Publish only the selected candidate’s intervals, coalesced with earlier committed coverage. Full item coverage includes its terminal boundary, even for an empty item. A partial item contributes only its committed intervals. On resume, use the complement for each item in order-modified order and reflow the remaining content at the new width. The count and paint passes start from the same immutable coverage. Finish when the complement is empty. Remove the old line/item-relative token fields from the producers and readers.

Use 3b.2’s explicit routing at child layout. A clean child boundary enters the next eligible item; an interrupted child carries its identity and nested token back to that same item. The child’s local run continuation remains in the owning block and is consumed by its inline formatter. Wire the currently unused resume-child argument to the corresponding child layout. Restore the prior text context there with zero inline advance for consumed content. At a between-item break inspect the clean-boundary alternative; at a mid-item break inspect the interrupted-child alternative and its nested local-run state. Both resume from the same interval representation.

### Select the boundary and publish its fragment

Use the shared block fragmentation chooser (block/fragmentation.rs::find_best_break) for Flex. The Flex adapter supplies candidate boundaries, block extents, applicable forced/avoid values and their source-interval snapshots. For a row, propagate the values from every item of the relevant lines under Flexbox §10. For a column, collect the values at each item boundary. Resolve forced boundaries before unforced ranking, sharing that handling in the common chooser path. The existing fitting-candidate ranking handles unforced preference and relaxation; preserve its tie and progress rules. A child's internal break-inside restriction applies within that child. Retire competing Flex-local selection logic.

| Executable fixture | Winning boundary and committed coverage |
|---|---|
| Two row lines AB and CD both fit; only D requests break-before:page | Break before CD. Commit A/B; C and D first appear on the next page. |
| Column A/B/C, room for two; all boundaries auto | At overflow choose B❘C and commit A/B. |
| Same column, avoid at B❘C, A❘B allowed | Choose A❘B, commit only A. B has no current-page output. |
| At B❘C both a forced page break and an applicable avoid | Choose B❘C and commit A/B. |
| Only B has break-inside:avoid | B❘C stays eligible. Commit A/B and resume C. |

At the definitive item-layout call in algo.rs, pass the remaining fragmentainer block space and the incoming token matched to that child, including None on its first fragment. Retain its returned content intervals, outgoing token and actual fragment extent. Cross-size measurement supplies sizing data only. Select the break before definitive positioning. Call definitive layout once for an admitted item fragment, keep its returned outcome for publication, and end the page at an interruption. Commit the typed interrupted-child alternative and partial coverage only for the selected child fragment. Preserve its actual extent when publishing the LayoutBox. Position and publish only selected item fragments in the current generation. A candidate discarded by the chooser leaves no generation-visible box or committed intervals.

## §2. Crossing checks

| Crossing | Producer → consumer | Distinguishing observation |
|---|---|---|
| Width × item membership | Row flex break → next fragment | Four labeled items A–D, flex:0 0 40px, wrapping enabled, zero gaps and margins, one line of block room per fragment. At widths 80→40→40 record AB, C, D. At widths 40→80→80 record A, BC, D. Read actual emitted item identities after each layout before their next layout. The flattened order is ABCD exactly once. |
| Ordering × cursor | Sorted flex items → break producer and consumer | For DOM ABCD and CSS order BDAC, the first 40px page covers only source B. At 80px the next pages emit DA then C. Keep an unchanged-width control. For a column use a two-word A (one word per line at 40px, both at 80px) followed by single-line B and C. With two lines of block room and no shrink, the narrow first page ends at A❘B. After widening, the next page emits B,C at block offsets zero and one line, even though A would now occupy only one line. |
| Nested progress × replay | Interrupted item → next child layout | Start with no token. Place a completed sibling before an item containing identifiable text runs p and q separated by a forced internal page break. Inspect the real outgoing interrupted-child token, child identity, fragment extent and partial coverage after p. Change the page width. Assert exactly q at child-local inline offset zero and absence of p and the completed sibling. Resume twice from the same token and geometry on equivalent input, comparing the fragment membership, remaining child content and committed interval sets. A wholly finished item cannot reappear; the partial item can contribute distinct fragments. |

For the two row width sequences, drive counting layout from its real outgoing tokens. Save each input token and width, and record current-generation emissions before overwrite. Replay those inputs in fresh paint-layout generations. Check each pass independently against its three expected page groups. On each page require the first item at (0,0) and the second, when present, at (40px,0) with zero padding and borders. Check exact source coverage after every page, absence of all rejected or previously completed items in that generation, and an empty complement with no outgoing token on page three. Apply the same generation-visible absence check to the avoid-retreat case.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-break-3 §5.1 Breaking into Varying-size Fragmentainers | preserve content progress at new size | width grows, shrinks or stays constant | Flex layout and shared token | supported widths | yes |
| css-break-3 §4.3 Forced Breaks | combine applicable forced values | force overrides avoid before overflow ranking | shared chooser and Flex candidate adapter | page cases | yes |
| css-break-3 §4.4 Unforced Breaks | rank fitting candidates and relax constraints for progress | allowed, applicable avoid, inapplicable internal avoid | shared chooser and candidate coverage | row and column | yes |
| css-flexbox-1 §10 Fragmenting Flex Layout | choose item or line break | row, column, interrupted item | row/column break producer and nested child reader | supported Flex fragmentation | yes |

Use all-feature Flex crate tests through its layout entry with real break tokens, followed by the layout integration suite. Preserve the existing direction and writing-mode support boundary.
