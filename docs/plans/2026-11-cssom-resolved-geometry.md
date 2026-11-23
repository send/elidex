# Resolve CSSOM geometry from the queried element

C1a follows N’s plan at da2ba4ad. It runs on the existing Dimension and FlexBasis types. It makes layout-dependent CSSOM reads observable before C1b changes layout inputs and C1c activates mixed computed fields.

## §1. Public query and geometry ownership

GetComputedStyle::invoke in elidex-dom-api currently reads ComputedStyle and calls the style-only serializer. The WPT harness uses the same projection after resolving style. Route DOM property access and geometry-dependent harness cases through one engine-independent resolved-value query. Its inputs identify the property, the target’s computed style, applicability and its current layout geometry. Use the existing entity and published screen geometry view. The owning pipeline must bring applicable geometry up to date before querying it. Never read a leftover LayoutBox from a prior screen generation or paginated pass. Keep geometry in its existing layout store.

For width on a rendered element to which width applies, select the used CSS width of the appropriate content or border box according to box-sizing. Return computed width for display:none, display:contents and inapplicable widths such as a non-replaced inline. For transform-origin on a rendered transformable element, resolve X against reference-box width and Y against its height using the same untransformed local reference geometry as paint. Preserve the existing supported axis and serialization semantics. Apply the height-like resolved-value rule from Transforms §4 and CSSOM §9. Flex-basis returns its computed keyword or length-percentage. Keep lower-level registry projections computed for transitions and underlying values. Preserve existing color handling, unvisited-color privacy and missing-style error behavior on the DOM path.

## §2. Crossing checks on current types

| Crossing | Production entry → consumer | Fixture and expected result |
|---|---|---|
| Layout × CSSOM width | Relayout → DOM getter and harness | Give a rendered block width:50%, zero margins, horizontal padding 5px on both sides and borders 2px on each side. With a 200px containing width, content-box exposes 100px (border box 114px). Border-box exposes 100px (content box 86px). Resize the container to 300px and expect 150px from the same percentage style. Query through both entries. |
| Applicability × geometry lifecycle | Style/display change → resolved-value query | After reading a visible width, switch to none and then contents. Read the computed 50% each time. A non-replaced inline with width:50% also returns 50%. Redisplay the block at a different containing width and assert its new used width. Interpose a paged layout before a subsequent screen read and check against current screen geometry. |
| Origin × reference geometry | Layout → DOM/harness and paint | Set origin to 25% 50% and transform to scale(2), with box origin (0,0) and no other transform. At reference-box sizes 100×40 and 200×80, getter offsets are (25,20) and (50,40)px. Final local matrix translations are (−25,−20) and (−50,−40). Keep the computed axes unchanged across layouts. For display:none check the computed percentage pair. |
| Public result × computed projection | Query, registry and transition input | For the rendered block whose width reads 100px, the lower-level width projection retains 50%. On a flex item with flex-basis:25%, the public getter and computed projection both retain 25% as the container changes size. Preserve auto and content controls. |

The harness must lay out a document before testing the rendered branches and must call the same query as the DOM entry. A style-only fixture can exercise computed fallback through an explicitly inapplicable case. Test the actual DOM bridge property read and the geometry-dependent harness path independently. Preserve unrelated public property serialization through existing regression cases.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| cssom-1 §9 Resolved Values | select the resolved width | applicable rendered width versus computed | DOM query and WPT harness | relevant display and applicability branches | yes |
| css-transforms-1 §4 The transform-origin Property | resolve two axes against local geometry | rendered transformable box and computed fallback | query and transform helper | supported axes | yes |
| css-flexbox-1 §7.2.3 The flex-basis property | preserve the computed projection | percentage and keywords | registry and public getter | supported current types | yes |

Run all-features tests for elidex-dom-api, elidex-style, elidex-wpt, elidex-css-transform, elidex-css-flex and elidex-render. Recheck touched module sizes and production callers on the implementation head. Record DOM and harness results, layout freshness, and computed-projection controls for C1c. Follow umbrella §3 for landing gates.
