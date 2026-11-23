# Resume wrapped Flex items across page widths

3b.2f under PM 1009. The 3b.2 plan landed at cee0efc6. Implement after 3b.2 establishes the PM 1012 consumed-interval and typed-token contract. Grid follows this child. The contract is observable at the Flex layout boundary before the later pagination plan replays whole pages.

## §1. Commit consumed source intervals at the chosen break

At cee0efc6 elidex-layout-flex/src/lib.rs reads line_index and item_index, collects the items in order-modified document order, and rebuilds line_ranges against the new container main size. fragment.rs::fragment_row_flex then starts at the saved line number. In fragment_column_flex the saved item is relative to a line. For each forced break or overflow/avoid candidate retain the covered source intervals keyed by formatting owner and item. Publish only the selected candidate’s intervals, coalesced with earlier committed coverage. Full item coverage includes its terminal boundary, even for an empty item. A partial item contributes only its committed intervals. On resume, use the complement for each item in order-modified order and reflow the remaining content at the new width. The count and paint passes start from the same immutable coverage. Finish when the complement is empty. Remove the old line/item-relative token fields from the producers and readers.

Use 3b.2’s explicit routing at child layout. A clean child boundary enters the next eligible item; an interrupted child carries its identity and nested token back to that same item. The child’s local run continuation remains in the owning block and is consumed by its inline formatter. Wire the currently unused resume-child argument to the corresponding child layout. Restore the prior text context there with zero inline advance for consumed content. At a between-item break inspect the clean-boundary alternative; at a mid-item break inspect the interrupted-child alternative and its nested local-run state. Both resume from the same interval representation.

## §2. Crossing checks

| Crossing | Producer → consumer | Distinguishing observation |
|---|---|---|
| Width × item membership | Row flex break → next fragment | Four labeled items A–D, flex:0 0 40px, wrapping enabled, zero gaps and margins, one line of block room per fragment. At widths 80→40→40 record AB, C, D. At widths 40→80→80 record A, BC, D. Read actual emitted item identities after each layout before their next layout. The flattened order is ABCD exactly once. |
| Ordering × cursor | Sorted flex items → break producer and consumer | Give distinct order values that differ from DOM order. Assert exact coverage in order-modified order at both widths. Preserve unchanged-width behavior and include a column-flex break within a line. |
| Nested progress × replay | Interrupted item → next child layout | Give one item distinguishable child content on both sides of its break. Resume twice from the same token and geometry on equivalent input, comparing the fragment membership, remaining child content and committed interval sets. A wholly finished item cannot reappear; the partial item can contribute distinct fragments. |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-break-3 §5.1 Breaking into Varying-size Fragmentainers | preserve content progress at new size | width grows, shrinks or stays constant | Flex layout and shared token | supported widths | yes |
| css-flexbox-1 §10 Fragmenting Flex Layout | choose item or line break | row, column, interrupted item | row/column break producer and nested child reader | supported Flex fragmentation | yes |

Use all-feature Flex crate tests through its layout entry with real break tokens, followed by the layout integration suite. Preserve the existing direction and writing-mode support boundary.
