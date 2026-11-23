# Parse the supported flex shorthand into its specified longhands

This prerequisite of the Flex increment in `#11-style-shorthand-expand` is draft G under PM 961. It lands after T so acceptance covers receiving-property reparse after var() substitution. The next PR migrates grid, float and transform grammar consumers. Baseline `c3c78070`. A declaration such as `flex: 0` must produce grow 0, shrink 1 and a zero-length basis. Today the parser accepts the first token as a length, yielding grow 1 and making an otherwise inflexible empty item consume free space. Fix this at parse time for all consumers.

## §1. Entry points and shared grammar

Both the declaration block reader and stylesheet rule parser call `parse_property_value`. Its built-in arms for flex-grow/shrink/basis and flex run before registry fallback. Direct FlexHandler parsing also exists. The built-in accepts calc and rejects negative factors, while the handler clamps negative numbers and its length helper admits unknown units through a fallback. Shorthand parsing also implements basis keywords independently. Test registry-free parsing, registry-backed parsing and the directly invoked handler.

Keep declaration production, property naming and priority handling in elidex-css. Put the common typed factor and basis value parsers in `elidex_plugin::css_resolve`, available to both sibling crates. Expose `elidex_plugin::css_resolve::parse_flex_basis` (NEW). Return CssValue::Auto for auto in the parser and the handler initial, the content keyword, or a validated nonnegative length/percentage or supported math expression. Pass the resulting value directly into flex triples. Share one factor parser for grow and shrink on all declaration and handler paths. Accept a nonnegative Number or a number-typed calc tree. At computation use the existing arithmetic evaluator for finite ordinary factor math and clamp the result to the factor range in both resolver paths. Keep the declared tree for readback. N owns changes to shared arithmetic and numeric conversion. Remove the local grammar duplicates. Move imports at each existing strict-parser call site. Convert errors at the property boundary.

Move the existing strict length/percentage and typed calc core from elidex-css values.rs into an elidex-plugin css_resolve sibling module, preserving the existing strict callers and their unit set, recursion and node limits. Put factor/basis entry points beside it. PM 961 assigns the remaining lax handler grammar to a dedicated migration immediately after G. N owns conversion of original numeric lexemes and IEEE evaluation for all consumers. C owns the computed mixed basis and layout/animation. G tests the declared typed basis tree and the public effects of ordinary factors.

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

**Substitution × admission.** On both inline and rule styles set --x:2 0 10px and flex:var(--x). Check grow 2, shrink 0, basis 10px. Compare explicit longhands. With --x:2 -3 10px or --b:-1px on flex-basis, check invalid-at-computed-value behavior. With --g:2, flex-grow:var(--g) computes to 2. These are single winning declarations. D owns competing longhands.

**Expression type × position.** Through both built-in and handler paths, grow calc(1 + 1) computes to 2 and calc(-1) to 0. Via shorthand, test calc(1 + 1) 10px and 10px calc(1 + 1), both 2/1/10px. Reject an incompatible sum of a number and a length. Accept calc(-1px) as a typed basis; C owns top-level basis clamping. Keep the property-range distinction in the extracted code's documentation, replacing the obsolete Level 3 §10.1 citation.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-flexbox-1 §7.1 The flex Shorthand | grammar | none or one/both groups, omission, bare zero | parse_flex_shorthand | ✓ | yes |
| css-flexbox-1 §7.2.1 The flex-grow property | value | literal or number-typed calc factor | FlexHandler | supported arithmetic | yes |
| css-flexbox-1 §7.2.2 The flex-shrink property | value | literal or number-typed calc factor | FlexHandler | supported arithmetic | yes |
| css-flexbox-1 §7.2.3 The flex-basis property | value | supported keyword or width, percentage | FlexHandler | supported subset | yes |
| css-sizing-3 §3.2 Sizing Values | width-derived basis | auto, content and supported numeric width values; stretch, min-content, max-content and fit-content remain outside the supported width subset | shared basis parser | supported subset | yes |
| css-values-4 §10.12 Range Checking | math functions | preserve supported calculations and apply their range at the specified stage | shared basis parser | supported subset | yes |
| css-values-4 §5.4 Numbers with Units | validate dimension token | known length unit or rejection | shared basis parser | supported length units | yes |
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
