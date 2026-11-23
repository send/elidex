# Carry containing references into block, table and replaced layout

C1b.1 follows C1a at f7900f62 and implements the first part of PM 1027 on existing Dimension variants. It changes the shared dispatch input and the production block-family callers together. C1b.2 and C1b.3 then migrate Flex and Grid item placement, respectively.

## §1. Resolve before choosing algorithmic available space

At the parent’s construction of each LayoutInput, take the physical containing content width and height with their actual definiteness, and the containing block’s writing mode. Derive the percentage inline basis from that mode: physical width in horizontal writing, physical height in vertical writing. Preserve an unavailable physical axis as absent. The child’s writing mode determines its own logical-to-physical mapping. Carry this information through the existing child dispatch entry. Keep any finite available-space fallback needed to run an algorithm separate from the percentage bases.

Replace compute_inline_containing’s missing-height fallback and its existing test. Migrate producers and consumers of the plain containing_inline_size field in the same change so every formatter can build and receive the invocation. In particular retain the optional basis across Flex’s vertical fallback and Multicol’s max-height calculation. Preserve their existing available-space behavior where space is needed for iteration. Flex item assignment and Grid contribution/placement are the subsequent children’s changes.

At the owning parent invocation resolve each preferred physical size, min/max constraint and edge once against its proper reference. Preserve definite zero, auto and a still-unresolved percentage until the relevant constraint or intrinsic algorithm selects its fallback. Hand block and table helpers the resolved padding/borders and constraints. Resolve explicit border-box height by subtracting those edges once. Remove the downstream synthetic zero basis for percentage padding and the positive-only test for percentage max-height. Keep computed style immutable across measurements and final layout.

For replaced elements, combine the resolved preferred physical sizes with intrinsic dimensions and aspect ratio in one shared sizing operation. Use it in normal flow and before positioned layout splits into inline and block constraint equations. Map the resulting physical pair to the positioned axes, then solve insets and margins. Remove the parallel positioned percentage/auto size decision for replaced elements. Preserve ordinary positioned non-replaced sizing.

## §2. Crossing checks

| Crossing | Production input → consumer | Required observation |
|---|---|---|
| Missing versus zero | Parent dispatch → max-height in block and Multicol | In normal flow give a child 30px content, max-height:50%, no edges. A definite zero containing height yields cap 0 and height 0. An auto-height parent with indefinite basis yields an unresolved cap and 30px content. For Multicol sample the constraint passed to its algorithm in both cases. |
| Orthogonal axes × available space | Containing vertical-lr parent → horizontal-tb child | At physical 90×200, 10% top padding on a 40px border-box child uses 200px and leaves 20px content height. With physical height missing versus explicitly zero, inspect the parent resolver and Flex/Multicol child invocation before their available-space fallback: the inline bases are absent and 0 respectively. Physical width remains 90 in both. |
| Padding × shared height | Parent edge resolution → block and table consumers | In horizontal-tb, height 40px border-box with 10% top padding and zero other edges yields padding/content 10/30 at width 100, then 20/20 at width 200. Assert actual block boxes and the unchanged computed percentage. At table layout’s shared padding reader use a separated table with zero spacing and borders, and inspect its padding and first cell origin at both widths. |
| Ratio × positioned axes | Resolved height → normal-flow and absolutely positioned replaced layout | Intrinsic image 60×30, width:auto, height:50%, no edges. In a 100px-high containing block both routes yield 100×50. In absolute layout fix top/left at zero and leave opposite insets auto. In ordinary flow keep an indefinite-height control at intrinsic 60×30 and a definite zero-height control at 0×0. Inspect the final image boxes. |
| Per-entry bases × replay | Pagination plan → continued owner’s actual child dispatch | In one paged run select 100px and 200px containing inline extents on successive entries. Inspect the same continued 50%-width owner at 50px then 100px independently in count and paint, before the next layout overwrites the boxes. Record entry identities and the unchanged percentage component. |

Use plain lengths and percentages at this stage. Preserve current Flex and Grid placement until their respective owner migrations. The C1b.2 column-Flex fixture uses a 40px border box with 10px padding to produce a 30px content extent and a 15px item basis at 50%. C1c repeats the relevant cases with mixed values.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-sizing-3 §4.1 Percentage Sizing | propagate physical percentage bases | absent, zero, positive | parent → block constraints | supported numeric cases | yes |
| css-sizing-3 §3.3 Box Edges for Sizing | subtract resolved edges | border/content box | block height and table edge readers | both boxes | yes |
| css-box-4 §4.1 Page-relative Padding Properties | resolve against containing inline basis | horizontal, vertical, orthogonal | dispatch input to resolved padding | physical edges | yes |
| css-sizing-4 §4.2 Effects of Preferred Aspect Ratio on Automatic Sizes | derive auto physical size | definite and indefinite preferred height | shared replaced operation | two flow entries | yes |
| css-position-3 §4.1 Automatic Sizes of Absolutely-Positioned Boxes | supply replaced size to axis constraints | positioned replaced vs ordinary positioned | positioned layout and constraints | replaced branch | yes |

The four modules cover a single shared reference boundary. Normal and positioned replaced callers must agree on one physical pair before solving axis equations; the table, Flex and Multicol callers of the input API must preserve zero versus absence at its first landing. Review breadth against that independently usable boundary.

Reinventory LayoutInput constructors, inline basis conversions, block child helpers, table padding readers and normal/positioned replaced paths at the implementation head. Add focused fixtures in the existing writing-mode, replaced, positioned, table and paged suites. Run all-features tests for elidex-layout-block, elidex-layout-table, elidex-layout-flex, elidex-layout-grid, elidex-layout-multicol, elidex-layout and elidex-render. Record crossed inputs, both public layout outcomes and landing gates for C1b.3’s program closeout. Follow umbrella §3.
