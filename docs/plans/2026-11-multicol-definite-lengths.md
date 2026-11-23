# Resolve column widths and paint snapped column rules

3c.3 follows the Text and Table plans at decff274 and 3a03bce0. It completes the definite-reference reader migration before N replaces shared arithmetic. Column measure and column-rule thickness have different used-value rules.

## §1. Route declarations to the two consumers

Migrate the width arms of columns and column-rule in declaration/misc.rs and the corresponding MulticolHandler longhands to G’s nonnegative length category. Retain the existing keyword and count parsers. The declaration layer supplies a bounded value and attaches priority after complete expansion, including for inline declarations and T’s receiving-property reparse. Require exhaustion within the bounded input. Use the ordinary style writer and registered handler with the same G evaluation and 3c.1 font context.

For column-width keep auto distinct from a specified zero. Preserve the computed length, including 0 and 0.5 CSS px. In compute_column_geometry apply the 1 CSS px minimum to the specified width used to calculate count. Use the available inline extent of that layout invocation, including on resumption. Keep the algorithm’s count cap, gap handling and redistribution of available room.

For column-rule-width compute the nonnegative absolute length then snap as a line width at the target device scale. Use one computed width operation for style/resolve/box_model and the registered writer. Supply device pixels per CSS pixel via the style resolution context. For screen, propagate the existing shell viewport snapshot’s device pixel ratio through ordinary resolution and handler dispatch. On scale change rerun style with the new snapshot. For print, pass an explicit output scale from the print target through build_paged_pipeline; preserve 1 dppx for the current CSS-pixel output when no raster density is specified. Test the headless entry using explicit scales. Compute once per style resolution. The rule paint consumer in builder/paint uses the stored snapped width for visible styles and suppresses none and hidden at use. Preserve medium’s existing 3px control and its declared serialization fidelity disposition (C-16).

## §2. Crossing checks

| Crossing | Producer → consumer | Required observation |
|---|---|---|
| Shorthand × registered width | Built-in and T reparse → ordinary style and registered longhands | columns:calc(12pt + 1px) 2 gives 17px and count 2. column-rule:solid calc(12pt + 1px) gives solid and 17px at 1 dppx. Repeat in reverse component order, via var holding the full value and via equivalent longhands. Inspect all expanded fields, including default currentcolor. Exercise unitless zero, 12pt, deferred units, 10bogus and a percentage hidden inside calc. |
| Cascade × component reset | Bounded declaration lists → winning longhands | Start with 3px/count 4 or solid/3px/red. An invalid direct shorthand with a third columns item or duplicate rule width keeps all earlier fields. A winning invalid var shorthand resets columns to auto/auto or rule to medium/none/currentcolor. Compare computed color with a distinct element color. Verify a following color declaration survives and !important reaches all components. |
| Computed width × geometry | Computed column-width → actual count and placement | In 12 CSS px, zero gap, auto count, both calc(1px - 1px) and 0.5px compute to their exact values and select 12 columns. A literal negative after 3px retains 3px and four columns; calc(1px - 2px) wins, computes zero and selects 12. Use short labeled fragments and a definite column height to observe adjacent column offsets. With both width and count auto there is no multicol container; with width auto and count 2 expect two columns. |
| New fragment × available width | Paged layout → resumed column geometry | Resume the same 3px-width, auto-count group from a 12px area into a 9px area, zero gap. Inspect counts 4 then 3 and the next labeled source contribution using the existing pagination contract. |
| Computed rule × paint | Output scale and style → computed readback and rule drawing | At 1 dppx, calc(.25px + .25px) computes to 1 CSS px for solid, none and hidden. At 2 dppx it computes to 0.5 CSS px for all three. Set two populated columns and an 8px gap. Inspect both computed routes and actual rule commands: solid spans one device pixel at each scale, none and hidden emit no stroke. A zero length stays zero and emits no stroke. Rerun the same document with only the output scale changed and inspect new computed fields and draw commands. |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-multicol-1 §3.1 Column width | compute width | auto, zero, positive | style and registered writer | both arms | yes |
| css-multicol-1 §3.3 Columns shorthand | expand width and count | either order, invalid winner | declaration parser and T | both components | yes |
| css-multicol-1 §3.4 Pseudo-algorithm | calculate used count | specified width, explicit count, available room | layout-multicol | supported branches | yes |
| css-multicol-1 §4.3 Column rule style | suppress rule at use | solid, none, hidden | rule paint | suppression branches | yes |
| css-multicol-1 §4.4 Column rule width | compute snapped absolute length | numeric and keyword | style and registered writer | width arms | yes |
| css-multicol-1 §4.5 Column rule shorthand | expand constituent declarations | omitted, explicit, invalid | shorthand and longhands | all components | yes |
| css-values-4 §6 Distance Units (snap as a line width) | snap as a line width | zero, subpixel, scale change | shared computed-width operation | line-width snapping | yes |
| css-values-4 §9.1 Numeric Functions | enforce nonnegative range | literal invalid and calculated clamp | G to computed widths | numeric arms | yes |

Check WPT css/css-multicol/zero-column-width-computed-style.html, zero-column-width-layout.html and parsing/column-width-computed.html at e00a7b14e8e3d9c04933e29ed247baf76c60706e. Record each applicable assertion, result, runner and revision in the implementation PR. Run the all-features parser, plugin, style, Multicol handler, layout-multicol, render and shell tests; remeasure touched module sizes before editing.

### Cross-reader closeout

Use the source-ID and closing-child table in the umbrella. For each row assigned to 3c.3 attach the Text, Table and Multicol records it calls for. Check actual grammar calls, accepted categories, direct/substituted results, placement of tests, scope and exact-head landing gates. Text contributes root/rem, generated-child and numeric-category controls; Table contributes axis inheritance and separated/collapsed cell gaps. Multicol contributes both shorthand paths, rule phases, width-to-count cases and pinned WPT results. Record plan evidence separately from execution evidence, which becomes available when the implementations land. Hand the three finite fixture sets to N for arithmetic replay.
