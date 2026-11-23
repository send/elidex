# Parse the supported flex shorthand into its specified longhands

This prerequisite of the Flex increment in `#11-style-shorthand-expand` is child G of the approved umbrella (PM 977). It lands after T so acceptance covers receiving-property reparse after var() substitution. G′ then removes all lenient unit/length parser paths listed in the umbrella. Baseline `c3c78070`. A declaration such as `flex: 0` must produce grow 0, shrink 1 and a zero-length basis. Today the parser accepts the first token as a length, yielding grow 1 and making an otherwise inflexible empty item consume free space. Fix this at parse time for all consumers.

## §1. Entry points and shared grammar

Both the declaration block reader and stylesheet rule parser call `parse_property_value`. Its built-in arms for flex-grow/shrink/basis and flex run before registry fallback. Direct FlexHandler parsing also exists. The built-in accepts calc and rejects negative factors, while the handler clamps negative numbers and its length helper admits unknown units through a fallback. Shorthand parsing also implements basis keywords independently. Test registry-free parsing, registry-backed parsing and the directly invoked handler.

Keep declaration production, property naming and priority handling in elidex-css. Put the common typed factor and basis value parsers in `elidex_plugin::css_resolve`, available to both sibling crates. Expose `elidex_plugin::css_resolve::parse_flex_basis` (NEW). Return CssValue::Auto for auto in the parser and the handler initial, the content keyword, or a validated nonnegative length/percentage or supported math expression. Pass the resulting value directly into flex triples. Share one factor parser for grow and shrink on all declaration and handler paths. Accept a nonnegative Number or a number-typed calc tree. At computation use the existing arithmetic evaluator for finite ordinary factor math and clamp the result to the factor range in both resolver paths. Keep the declared tree for readback. N owns changes to shared arithmetic and numeric conversion. Remove the local grammar duplicates. Move imports at each existing strict-parser call site. Convert errors at the property boundary.

Move the existing strict length/percentage and typed calc core from elidex-css values.rs into an elidex-plugin css_resolve sibling module, preserving the strict entry points and existing recursion and node limits, and admitting the PM 977 unit subset. Put factor/basis entry points beside it. G′a begins the remaining handler migrations after G; G′b retires the last lenient wrapper after C1. N owns conversion of original numeric lexemes and IEEE evaluation for all consumers. N owns simplification, C1 the computed payload and C2 Flex layout/animation. G tests the declared typed basis tree and the public effects of ordinary factors.

## §2. Coupled invariants

1. **Group boundaries and zero.** Consume at most two adjacent factors, optionally before or after one basis. Try a number as a factor before treating a zero as a length. Only a zero preceded by both factors can be a bare basis. Preserve a zero percentage and dimension as a basis. A failed tentative group restores the cursor before the other alternative is considered.
2. **Defaults and completeness.** A missing grow or shrink restores 1. A missing basis restores Length(0, Px). An isolated none produces 0/0/auto. The CSS-wide and variable fast paths stay at the declaration boundary. Keep priority parsing in the caller and require complete consumption at the property boundary, including rule and setter paths. Invalid input must produce no partial triple.
3. **Typed agreement.** A shorthand component and the corresponding explicit longhand reach the same typed representation. Negative factors and literal negative bases fail on all paths. Supported calc length-percentage expressions retain their tree and existing range handling. Distinguish an absent component from a parse error followed by residual tokens.
4. **Cascade and layout.** Expansion produces exactly grow/shrink/basis in canonical order with uniform priority. Family readback and layout should both see the same values after cascade. No CSSOM serializer is required to establish the parser result.

| Input | Ordered grow/shrink/basis or expected failure |
|---|---|
| 0; 2; 2 0 | 0/1/0px; 2/1/0px; 2/0/0px |
| 0px; 0%; 0 0 0 | 1/1/0px; 1/1/0%; 0/0/0px |
| auto; none; content; 0 auto | 1/1/auto; 0/0/auto; 1/1/content; 0/1/auto |
| 25% 2; 2 25%; 25% 2 3; 2 3 25% | 2/1/25% for the first pair, 2/3/25% for the second |
| 2 calc(50% - 1px); calc(50% - 1px) 2 | Same typed basis and factors in either order |
| 2 10px 3; 2 3 4; auto content; none 1; -1; 2 -3; -1px; 2bogus | Invalid declaration, adjoining valid declarations still parse |
| 0 !important; inherit; var(--x) | Priority on all three; CSS-wide expansion; via T reparse, see controls below |

**Basis validation on all three paths.** Independently feed `-1px`, `-5%` and `2bogus` to the flex-basis declaration parser, flex shorthand (alone and after two factors), and direct FlexHandler longhand parse. Require rejection with no emitted declaration or partial triple. For each route use `0em` and `calc(10% + 2px)` as positive controls. The declaration parser currently accepts signed literal lengths/percentages; the handler helper maps unrecognized units to Px. Keep range checking of literal values separate from calc’s deferred range check.

**Substitution × admission.** On both inline and rule styles set --x:2 0 10px and flex:var(--x). Check grow 2, shrink 0, basis 10px. Compare explicit longhands. With --x:2 -3 10px or --b:-1px on flex-basis, check invalid-at-computed-value behavior. With --g:2, flex-grow:var(--g) computes to 2. These are single winning declarations. T owns cascade collisions and D owns public order/provenance.

**Expression type × position.** Through both built-in and handler paths, grow calc(1 + 1) computes to 2 and calc(-1) to 0. Via shorthand, test calc(1 + 1) 10px and 10px calc(1 + 1), both 2/1/10px. Reject an incompatible sum of a number and a length. Accept calc(-1px) as a typed basis; C1/C2 apply the range when the required inputs become available. Keep the property-range distinction in the extracted code's documentation, replacing the obsolete Level 3 §10.1 citation.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-flexbox-1 §7.1 The flex Shorthand | grammar | none or one/both groups, omission, bare zero | parse_flex_shorthand | supported basis grammar | yes |
| css-flexbox-1 §7.2.1 The flex-grow property | value | literal or number-typed calc factor | FlexHandler | supported arithmetic | yes |
| css-flexbox-1 §7.2.2 The flex-shrink property | value | literal or number-typed calc factor | FlexHandler | supported arithmetic | yes |
| css-flexbox-1 §7.2.3 The flex-basis property | value | supported keyword or width, percentage | FlexHandler | supported subset | yes |
| css-sizing-3 §3.2 Sizing Values | width-derived basis | auto, content and supported numeric width values; stretch, min-content, max-content and fit-content remain outside the supported width subset | shared basis parser | supported subset | yes |
| css-values-4 §10.12 Range Checking | math functions | preserve supported calculations and apply their range at the specified stage | shared basis parser | supported subset | yes |
| css-values-4 §5.4 Numbers with Units | recognize dimension syntax | number and unit identifier | shared basis parser | supported length units | yes |
| css-values-4 §6.1 Relative Lengths | resolve font and viewport dimensions | existing contexts and deferred units | shared lookup, calc leaves and computed readers | PM 977 subset | yes |
| css-values-4 §6.2 Absolute Lengths | convert a physical length | fixed unit ratios and canonical px | shared conversion and media range values | all seven units | yes |
| css-values-4 §9.1 Numeric Functions | range timing | invalid literal or computed/used clamp | factor and basis parse docs, factor resolution | supported arithmetic | yes |
| css-values-4 §10.9 Type Checking | result type | number factor or length-percentage basis | shared typed calc parser | supported arithmetic | yes |
| css-flexbox-1 §9.7 Resolving Flexible Lengths | regression observation | positive free space and grow zero | layout test | observation only | yes |

Use normative §7.1 in the rewritten implementation. §7.1.1 Basic Values of flex is informative. Reproduce headings with `.claude/tools/webref heading css-flexbox-1 7` and `.claude/tools/webref heading css-flexbox-1 9.7`. Use `.claude/tools/webref body css-flexbox-1 flex-property` for the group grammar and omission rules. [Flex shorthand and components](https://drafts.csswg.org/css-flexbox-1/#flex-property).

### §3.1 User-input touch audit

The token stream comes from inline markup, stylesheet rules or CSSOM setters. Exercise malformed functions, repeated groups, invalid dimensions and trailing tokens before the next declaration. Bound tentative parsing to the two grammar groups. Retain the existing parser depth/node budgets and test that failure leaves the next declaration readable.

## §4. Verification and landing

- Extend the existing elidex-css shorthand declaration tests with the typed matrix above. Cross-check setProperty, inline block and rule block entry points including a following declaration and !important. Test standalone handler values in elidex-css-flex and the extracted value parser in elidex-plugin.
- Through the style pipeline parse a fixed 100px horizontal container with two empty, unconstrained children, zero min-width, no margin, gap or border. Give the first child flex:0 and the second flex:1. Assert computed grow values 0 and 1, and used widths 0px and 100px. The bad parse would split the positive free space evenly. Include a control using explicit longhands. Construct the styles from declaration text to exercise the defect.
- Cross-check flex-valid.html, flex-invalid.html, flex-shorthand.html, flex-shorthand-calc.html and flex-computed.html against web-platform-tests/wpt at e00a7b14e8e3d9c04933e29ed247baf76c60706e. Record the filename, assertion, typed expansion and any separate serialization/computed dependency in the implementation PR description.
- Put the grammar matrix in a new flex_grammar sibling test module in elidex-css, handler controls in elidex-css-flex, expression controls with the new plugin module, cascade tests in elidex-style, setter/rule tests in elidex-dom-api and the sizing control in elidex-layout-flex. Run each crate's all-features tests. Audit existing strict caller regressions at the extraction commit.
- Reconcile the draft with the umbrella verdict, request its own full plan review and land after T. Before push run repository CI and obtain PM go. Single-pass post-push Codex review, then merge at PM's explicitly approved head.

## Shared grammar contract for G′a and G′b

Move the strict unit lookup, length and length-percentage entry points in values.rs with the calc grammar into elidex-plugin. CSS Values §5.4 defines dimension syntax; §6 defines the length units. Accept §6.2 absolute units and em/rem/vw/vh/vmin/vmax plus lvw/lvh/lvmin/lvmax. Reject other §6.1 units at parse time, including in calc, until the contexts tracked by the umbrella slot exist. Test CSS case-insensitivity, Q/mm/cm/in/pt/pc/px conversions, font and viewport units with real reference contexts, unitless zero, a percentage and an unknown dimension. Keep the numeric category and the reference dependence in the typed result. Re-run existing strict clients. G′a migrates page, text, table and multicol; G′b after C1 migrates reference-dependent consumers and deletes legacy wrappers. The input category comes from the property grammar; it never depends on which parser entry point was used.

### Reader inventory and representation

G preserves the specified unit in a typed declared length, including distinct vw and lvw spellings. The shared resolver maps both to the same large viewport measure. Resolve absolute units with the CSS §6.2 ratios; keep the original media unit and value for RangeValue::Converted. Verify declared longhand output, computed px and used geometry separately: 1in, 2.54cm, 25.4mm, 101.6Q, 72pt and 6pc all resolve to 96px. In a 200px by 100px viewport, 10lvw and 10lvmax resolve to 20px; 10lvh and 10lvmin resolve to 10px. Test upper/lower-case input at the strict lookup and retain the canonical suffix on declared output.

Run `git grep -l -w LengthUnit c3c78070 -- crates` (57 files including tests). For each production hit trace the value from parser or constructor to its switch, including wildcard arms and Px-only guards. Repeat the inventory on the implementation head. Classify each as a producer, declared serializer, computed/used conversion, or consumer of already normalized px. Regression tests must reach each changed reader from the strict declaration route.

| Consumer group | Check at G |
|---|---|
| Plugin values, css_resolve and ordinary style resolution | Keep declared magnitude and unit until conversion, including calc leaves. Apply fixed ratios or the supplied font/viewport context once. |
| Media parse, eval, types and serializer | Share unit recognition and conversion while preserving tolerant comparison for physical lengths and direct-px comparison. Check threshold equality and reparse at 2.54cm. |
| Animation detection, interpolation and apply | Trace declared keyframe and computed transition inputs separately. Normalize supported literal lengths before Px-only apply_dimension, apply_flex_basis, apply_px and apply_optional_px consume them. Verify physical length and large-viewport alias samples update the field. C2 owns publishing newly generated mixed calc samples. |
| Grid, Flex, Box, Float, Transform, Text, Multicol, Table, Background, paged style and render | Inspect reachable readers of newly admitted strict values. Test the original property at its actual reference size whenever G changes its value path. Record any unchanged producer-only hit with its caller. Each remaining alternative parser moves with its reader in G′a or G′b. |
| DOM presentational hints, UA defaults and tests | Check literal construction versus decoding. Keep constructors that supply canonical px; validate the downstream invariant with a direct strict-parsed control. |

## Binding R3 acceptance

**C-4.** Replace the parallel absolute conversion table in media/parse.rs::media_length_to_value with the shared unit classification and conversion. Preserve RangeValue::Converted (original unit/value and tolerant comparison) for converted physical units, and exact comparison for direct px. Exercise `(width: 2.54cm)` at 96px with the equality and range boundaries, and direct px on either side. Assert identical matching and serialization/reparse before and after extraction. Reject deferred units through both ordinary and media parsers.

**V-C2, V-C3.** Exercise each deferred family from umbrella §1 through direct declaration parsing, calc, substituted width and @supports. The direct declaration is invalid; a winning var(--x) containing 1ch becomes invalid at computed-value time instead of reviving an earlier declaration. Pair vw/vh/vmin/vmax with lvw/lvh/lvmin/lvmax and require equal used values in a non-square viewport. Enumerate every LengthUnit match, including Px-only apply_px, wildcard arms, aliases and the media conversion path from C-4. Test 1in = 96px through each changed reader, preserving converted-media comparison semantics.
