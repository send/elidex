# Carry Flex item size and margin decisions into child layout

C1b.2 follows the C1b.1 plan at 765698e7. It supplies the Flex evidence for C1b binding F1 under PM 1027. Grid closes F1 in C1b.3. Work on the existing Dimension and FlexBasis variants, using C1b.1’s shared child invocation with independent physical reference and algorithmic dimensions.

## §1. Use one dispatch path for measurement and placement

At the Flex container resolve its padding and borders against the selected containing rectangle. Derive the definite inner main extent, when available, from its content box. In a column use that same optional content height for the percentage flex basis and vertical constraints. Keep absent, definite zero and algorithmic available space distinct through helpers.rs item collection and the callbacks in algo.rs. Use the containing writing mode to select percentage edge references. The item’s own writing mode maps its logical axes.

At relayout_item_at_position the current formatter writes physical lengths and zero margins into the item’s ComputedStyle. Replace that write with invocation-local assignments on the ordinary child dispatch. After free-space distribution and alignment, Flex owns the physical content size and its auto-margin decisions. The assigned content size supersedes the item’s preferred size for that invocation. final_main is already a content size; subtract cross-axis padding and border from the assigned outer cross extent exactly once. Preserve the selected percentage reference for the item's own edges separately. Pass its assigned inner size and applicable definiteness on to descendant layout. Preserve stretch definiteness for descendant percentages (CSS Flexbox §9.8).

Resolve auto margins in Flex and record which share was consumed by the position. When the Flex cursor already includes the leading auto share, child layout adds zero further displacement for that margin. Leave other edges and the actual computed Auto intact. Probes carry only assignments established in their own phase. Thread the invocation through block, nested Flex and replaced item dispatch using C1b.1’s precedence for an already solved physical content pair. Remove the style insertion and any dependency on restyling to undo placement. Keep offsets, baselines, break continuations, layout generation and probe status on the production path.

## §2. Crossing checks

| Crossing | Producer → consumer | Required observation |
|---|---|---|
| Percentage × auto margin × rerun | Row Flex → item → nested block | Use a single non-growing/non-shrinking item with flex-basis:auto, width:50%, margin-left:auto, zero other edges and a descendant width:50%. At container widths 200 then 300px, with no restyle between layouts, item widths are 100 then 150px and left offsets 100 then 150px. Descendant widths are 50 then 75px. Observe the item style before measurement, after probe and after placement: width stays 50% and margin stays Auto. Capture the child callback’s origin and size before outer LayoutBox publication. |
| Box sizing × definite column main axis | Parent resolver → basis and max-height → child dispatch | In horizontal-tb use a column Flex container with height 40px border-box, 10px top padding and no other edges. For a nonempty item with 50% basis, no grow or shrink, min-height:0 and max-height:100%, inspect the 30px basis for both percentage operations, 15px flex base and 15px final content height. Contrast a definite zero basis and an auto-height container: the former resolves to zero, the latter leaves the percentage indefinite for the existing content sizing path. |
| Stretch × descendant percentage | Flex line cross-size → final child layout | In a single-line row 60px high, stretch an auto-height border-box item with 5px top and bottom padding. The item receives 50px of content height and its 50%-height child uses 25px. Inspect the nested child box and immutable auto height; double subtraction would give 20px. |
| Replaced and nested dispatch × assigned size | Flex → existing child formatter | Repeat placement with an intrinsic 2:1 image and with a nested flex container. Inspect the assigned physical pair at the callback and inside the receiving formatter, and its effect on descendant placement where applicable. Check the same entity in a second pass with a changed containing extent and verify its preferred fields and margins survive. |
| Selected page × live item style | Pagination count/replay → row Flex | Reuse the landed 3b.2f width-changing fixture with a percentage-sized item. In each selected entry capture item identity, computed percentage, used width and descendant bounds before overwrite. Compare count and replay independently against the entry’s own width and preserve the source intervals and continuation just as in 3b.2f. |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-flexbox-1 §7.2.3 The flex-basis property | resolve against definite inner main size | percentage, auto and indefinite | item collection and flex basis | current numeric and keyword routes | yes |
| css-flexbox-1 §8.1 Aligning with auto margins | assign positive free space | auto share versus ordinary margin | item placement → child offset | existing main-axis placement | yes |
| css-flexbox-1 §9.8 Definite and Indefinite Sizes | establish descendant reference | stretched cross size, definite zero and absent basis | final child dispatch | relevant definiteness branches | yes |
| css-sizing-3 §3.3 Box Edges for Sizing | select content extent | padding/border already consumed | container resolution and physical item assignments | both box models | yes |

Trace item collection, measurement, flexible-length resolution, positioning and final dispatch in elidex-layout-flex. Reinventory direct ComputedStyle insertions and child input constructors at the implementation head. Put numerical cases in the Flex layout suites and run all-features tests for elidex-layout-flex, elidex-layout-block, elidex-layout, elidex-layout-grid and elidex-render. Pass C1b.1’s shared-constructor controls and record before/after style and callback observations for C1b.3. C1c owns mixed-field activation; C2 owns the later Flex probing and sampling algorithm. Follow umbrella §3 for landing gates.
