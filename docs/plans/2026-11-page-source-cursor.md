# Resume inline content across pages of different widths

3b.2 owns the shared source cursor for binding F3. The adopted umbrella at d1bff8b6 assigns source progress to L1. 3b.1 supplies selected geometry. 3b.3 will replay page identities and break events. PM 1009 assigns Flex to 3b.2f. PM 1010 assigns Grid to L1 in 3b.2g and closes the Block, Flex, Grid and Multicol audit.

## §1. Continuation state

BreakTokenData::Block currently carries a child index and an optional count of already emitted lines. Both block/mod.rs and the mid-run and trailing-run paths in block/children/stack.rs pass that count to the inline formatter. inline/mod.rs packs the entire run at the new width, then slices by the old line count. A page change can place different words before that count.

Record the first unconsumed logical source position in the owned break token. Define one shared source cursor type and advancement rule in the existing layout token layer. A boundary before an item identifies its stable entity in its owning formatting context; the inline text variant additionally carries the logical text position. 3b.2f and 3b.2g use this same type and rule for Flex and Grid items. Keep the block child index for stable child boundaries. Inside an inline run identify the text entity and a valid source boundary, or the boundary before/after an atomic or generated inline item, with enough owner and item identity to distinguish generated content from an element’s own children. Keep the cursor local to a layout run with immutable input. A changed DOM or style snapshot starts a fresh run.

Carry provenance from inline collection through whitespace processing, text transformation, shaping and line commitment. Preserve the source interval for collapsed whitespace spanning nodes and expansions from one source character. Break only at supported cluster boundaries; where a source span produces multiple breakable output units, retain a position within that span’s deterministic transformed sequence. Resume by restoring context for whitespace, shaping and bidi order across the boundary, suppressing earlier content from the committed fragment. Select the cursor at the last line actually committed after widow/orphan adjustment. Source progress stays independent of line number, physical offset and visual glyph order. Empty runs and nonpainting items must still advance. Preserve the existing routing for whole versus fragmented multicol inline flows using the presence of a continuation.

The inline formatter returns both the fragment’s output and its end cursor. Thread that cursor through the nested child token in both block paths. Use the input cursor with the selected content extent for both counting and painting. Keep state on the token chain owned by the pagination run. The test reads the produced content at the seam before the next layout pass overwrites per-entity layout data.

### Sibling audit at d1bff8b6

| Consumer | Observation | Disposition |
|---|---|---|
| Flex | elidex-layout-flex/src/lib.rs rebuilds line_ranges using container_main and wrap. fragment.rs::fragment_row_flex takes resume_line from the previous fragment and skips to that index in the new ranges. Four distinct 40px items A–D fit two per row at 80px and one per row at 40px. After consuming A/B, line index 1 at 40px starts at B. Reversing widths, after consuming A, line index 1 at 80px starts at C. Column flex also addresses an item relative to its line. | 3b.2f follows 3b.2 and precedes 3b.3. Use the shared source cursor and run both four-item width transitions as executable regressions. Check ordered items and nested child continuations. |
| Grid | elidex-layout-grid/src/lib.rs reads resume_row against recomputed row tracks. Fixed cell placement can keep the same item identities even when track sizes change. For repeat(auto-fill, 40px) with zero gap, lib.rs expands tracks against container_inline_size and passes explicit_cols to place_items. With four auto-placed items A–D the second row starts at C at 80px and at B at 40px. A token for row 1 denotes different remaining items. | 3b.2g follows 3b.2f and precedes 3b.3. Use the shared cursor. Run A–D at 80→40px and 40→80px, assert exactly-once item coverage, compare explicit placement, and check the remaining content of a spanning child. |
| Multicol | fill.rs::extract_child_index reads the stable outer block child ordinal. At d1bff8b6, elidex-layout-multicol/src/fill.rs:247 retains the entire output token; fill.rs:154 passes it into the next column and fill.rs:485 places it in LayoutInput. elidex-layout-block/src/block/children/stack.rs:91 extracts the nested token and stack.rs:341 forwards it to the resumed child. Column fill uses the outer ordinal for fragment membership. | The inspected forwarding path needs no distinct resume mechanism. Exercise both outer child transitions and a partial inline run across columns. Verify the nested source cursor survives fill and balancing, as well as existing whole-column persistence. |

## §2. Crossing checks

| Crossing | Producer → consumer | Discriminating observation |
|---|---|---|
| Source boundary × geometry | Committed line → next-width formatter | For wide→narrow and narrow→wide, use distinct words, deterministic metrics and room for one committed line. Reassemble source intervals and emitted text over all fragments; each word appears exactly once and in order. Keep a constant-width control. |
| Source boundary × run kind | Collector and packer → direct and mixed block consumers | Repeat with multiple text nodes, a nested inline, an atomic item, a transformed character, collapsed whitespace and an empty decorated inline. Verify source coverage and atomic identity across an inline run followed by a block and a trailing inline run. Use L2’s existing semantics as inputs. |
| Geometry × replay | Count pass → per-page layout pass | With the same start cursor and width, both passes return the same end cursor and selected source intervals. Check termination and total pages against emitted fragments. An updated snapshot must begin at the initial cursor. |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-break-3 §5.1 Breaking into Varying-size Fragmentainers | Continue content progress at the new size | Wider, narrower, same width | Inline collection, line commit, Block token producers and both resume paths | Supported inline paths | yes |

Review this memo together with the 3b.2f and 3b.2g umbrella rows. The 3b.1 plan landed at be0c5d24; implementation follows its style-to-layout handoff. Test elidex-layout-block and elidex-layout through their normal entry points. The later page-plan child supplies the integrated page sequence for public replay.
