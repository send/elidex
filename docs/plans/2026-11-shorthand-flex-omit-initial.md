# Flex shorthand serialization for declared values

F is the final implementation row (11) of the [Flex omit-initial program](2026-11-flex-omit-initial-program.md). Planning base 400cbdd8 includes the S plan (#760). Verify the actual landed G and S implementations and their transitive dependencies before activating either family. G supplies the inverse grammar, D2 the declared winners and provenance, H the typed handler and S the public formatter. C2 supplies the independent used-basis reader. Both FlexHandler arms land together; the existing coordinator selects the family on the winning typed inputs.

## §1. Change and acceptance

Implement FlexHandler’s two typed family serializers under the existing coordinator. G owns grammar, D owns winners, H supplies typed dispatch, S supplies component text. Support pending and CSS-wide reads via their existing coordinator branches. Exercise independent rule and inline getters, empty/mixed-priority and missing members, alias selection and repeated reads. Assert canonical grammar order and exact zero basis semantics against the parser and used layout. Cross-check pinned WPT expectations and document normative differences.

For 1/1/0px both `1` and `0px` parse to the same triple under §7.1. CSSOM step 2 permits omission and requires representability. Elidex chooses `1`: it retains the first grammar component and is shorter in characters. This rule also keeps the all-default serialization nonempty.

| Parsed physical longhands | Required declared readback | What reparsing must preserve |
|---|---|---|
| row / nowrap, column / nowrap, row / wrap | row, column, wrap respectively | A valid single keyword restores the omitted component. |
| column-reverse / wrap-reverse | column-reverse wrap-reverse | Author order may be reversed. Exercise all four directions × all three wraps. |
| 0 / 0 / auto, 1 / 1 / auto, 0 / 1 / auto | none, auto, 0 auto respectively | A bare 0 would change the third triple's basis. |
| 1 / 1 / 0px, 1 / 1 / 0% | 1, 0% respectively | A dimension and a percentage remain distinct even at zero. |
| 0 / 1 / 0px, 2 / 1 / 0px, 2 / 0 / 0px | 0, 2, 2 0 respectively | Omitted basis restores 0px. The second number is shrink. |
| 1 / 2 / 25%, 2 / 1 / 25% | 1 2 25%, 2 25% respectively | Grow anchors nondefault shrink. Omitting only shrink keeps the basis. |
| 1 / 1 / content, 0.5 / 0 / 10px | content, 0.5 0 10px respectively | Preserve basis-only keyword, fractional grow and explicit shrink zero. |
| 1 / 1 / calc(2px + 10%); 1 / 1 / 0em | `calc(10% + 2px)`; `0em` | From independent registry-parsed longhands, assert basis-only readback, reparsed expression or Em unit, and identical second serialization. |
| 0.1234567 / 1 / 0px | `0.123457` | Independently assert the literal on inline and rule getters. Reparse and read again as `0.123457`. Also check 1/1/0.1234567px and 1/1/0.1234567% as `0.123457px` and `0.123457%`. |
| 0.9999999 / 1 / auto; 1 / 1 / auto | `1 auto`; `auto` | In both public getters and typed serialize_parsed, the former retains grow before rounding. Repeated reads of the original remain `1 auto`. Reparsing that output creates exact grow 1 and subsequently returns `auto`. |

## §2. Crossing checks

| Crossing | Producer → consumer | Discriminating observation |
|---|---|---|
| Typed omission × public rounding | H, S → F | grow/shrink/basis 0.9999999/1/auto reads as “1 auto”, exact 1/1/auto as “auto”. Reading the original twice preserves the distinction. Reparsing rounded output can produce exact 1. Check both public getters. |
| Grammar × omission × used basis | G, C2 → F | 0/1/0px → “0”, 1/1/0% → “0%”, 0/1/auto → “0 auto”. Reparse the resulting triple and check its layout effect. Check direction/wrap permutations and CSS-wide or pending-group readback through the coordinator. |
| Earlier Flex verdicts | D → H → S → F | PM 928 F1/F2: omit 0px by grammar without a WPT-only exception. F6: for flex and flex-flow a later var after initial reads as var(--x); the reverse reads initial after D. F7: justify the all-initial survivor from CSSOM equivalence and canonical ordering before adopting it. |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSSOM §6.6.1 The CSSStyleDeclaration Interface | getPropertyValue, priority, mutation | complete, partial, mixed winner | D coordinator and CSSOM | inline and rule reads | yes |
| CSSOM §6.7.2 Serializing CSS Values | list 1.1–1.3, omission step 2 and component output | preferred shorthand, equivalence, numeric precision | H / S / F | supported families | yes |
| css-flexbox-1 §5.3 Flex Direction and Wrap: the flex-flow shorthand | component grammar | direction, wrapping or both | F selection | yes | yes |
| css-flexbox-1 §7.1 The flex Shorthand | parse groups and fill defaults | none, factors, basis, bare zero | G parsing / F selection | supported basis | yes |

## Binding R3 acceptance

**C-7.** For grow/shrink/basis 0/1/0px the declared getter emits `0`. The rule-cssText WPT accepts `flex: 0px;` or `flex: 0 1 0px;`; retain this as a spelling cross-check for the separate block regrouping slot. Preserve explicit CSS-wide keywords because omitting them can reparse as physical Flex defaults and lose inheritance/reset semantics. Report Flex as affected under #11-shorthand-omit-initial-csswide-omission until a meaning-preserving family rule covers the case. [WPT flex-serialization.html](https://github.com/web-platform-tests/wpt/blob/e00a7b14e8e3d9c04933e29ed247baf76c60706e/css/cssom/flex-serialization.html)


## F8 · Selection and production fixtures

Use the typed values from H before calling S on each selected component. For physical flex triples, first recognize the exact none alias. If basis is the omitted zero length (0px), select grow and the nondefault shrink when present. Otherwise use the basis alone for exact 1/1, or emit grow, nondefault shrink if present, and basis in canonical order. Keep grow as the anchor for a nondefault shrink, including when grow is 1. Flow defaults are row/nowrap: emit both nondefaults in direction/wrap order, a single nondefault alone, and row when neither survives. Exercise both input orders for two-keyword flow. All-default flow and the one-token flex tie must produce nonempty grammar-valid output. Reserve explicit CSS-wide declarations for D’s value-kind branch.

| Crossing | Fixture → observations |
|---|---|
| H typed dispatch → S rendering → both declared getters | Build the table above once from independently parsed longhands and once from shorthand expansions. Read actual element.style and an unmatched rule.style. For 0.9999999/1/auto assert `1 auto` twice and original grow below 1; exact 1/1/auto reads `auto`. Set the first output on a fresh owner: grow becomes exactly 1 and the next read is `auto`. An unrelated inline mutation preserves the original selected components. For a tiny positive 0.0000001px basis, 1/1 factors select the basis and render `0px`; starting with exact 0px selects `1`. |
| Parser and C2 layout → observable zero distinction | Put a non-growing item with zero automatic-minimum contribution (explicit min-main-size:0), no edges, and a 37px unbreakable child in an indefinite-main content measurement. Compare grow/shrink/basis 0/1/0px, 0/1/0% and 0/1/auto (preferred main auto). Expect declared `0`, `0 0%`, `0 auto` and measured bases 0/37/37px. Reparse each and repeat. In a definite 100px container confirm bases 0/0/37px. Record the measured basis separately from final distributed item size. |
| N math and S spelling → G reparse | From 1/1 factors exercise the three declared bases calc(1px / 0), calc(-1px / 0), calc(0px / 0); select the basis and require the corresponding calc(infinity * 1px), calc(-infinity * 1px), calc(NaN * 1px). Reparse and check type and sign or NaN classification; verify the same second declared read and N’s separate computed clamping. Keep percentage-dependence and non-px zero units in the declared projection. |
| D2 winner → coordinator → Flex handler | Test missing member and mixed priority (empty value), all-important physical members (canonical shorthand and important priority), complete pending group (original var) and partial pending group (empty). Test initial then whole var and reverse order through both declared owners, then attach/match to verify the computed longhands. A physical 0/1/auto remains `0 auto`; explicit initial stays `initial`. Replay all 12 direction/wrap combinations and control the index/priority on both owners. |
| Existing families after Flex activation | Repeat H’s Box rectangular/radius, Table axis-pair and Multicol all-default controls through the real getters. Verify S’s media precision, D2 reflection/source fidelity and C2 animation geometry using their accepted fixtures without adding another serializer. |

**M1.** Put family-selection cases alongside FlexHandler, inverse grammar cases with css declaration parsing, wrapper and priority/order fixtures with DOM API/JS CSSOM and actual geometry with Flex layout. Planning-base Flex lib.rs is 708 lines; recount all touched modules and apply the standalone split rule. Run all-features suites for elidex-plugin, elidex-css, elidex-css-flex, elidex-style, elidex-css-box, elidex-css-table, elidex-css-multicol, elidex-layout-flex, elidex-dom-api, elidex-js, elidex-shell and elidex-wpt plus affected readers. At the C-7 WPT pin compare css/cssom/flex-serialization.html alongside G’s flex-valid, flex-invalid, flex-shorthand, flex-shorthand-calc and flex-computed controls. Record exact file, selected assertion, runner, revision, expected/observed results and implementation SHA. Label adapted engine assertions and external WPT runs separately; preserve the bare-zero spelling comparison under C-7.

**M2 and program exit.** Trace from both declared getters through D2’s typed block, H’s coordinator and built-in dispatch to FlexHandler, S’s final numeric/calc output, G’s declaration reparse and C2’s actual layout measurement. Record the landed producer/reader file, symbol, reviewed head, main merge and fixture per F8 row. Baseline FlexHandler inherits the trait’s default shorthand method; verify both override arms and no Flex-specific bypass in either getter at activation. Record the same symbols for the no-regression controls. Use the [G′b.6 downstream exit record](2026-11-length-reader-closeout.md) and umbrella child order to verify *implementation* merges and results for T/G, all page/property readers, N, C1, the five G′b domains and wrapper deletion, C2, D1/D2, H, S and F on the integrated head. Include the C-13 rule/inline matrix, computed/used and live media checks, plus zero-basis, tie-break and pinned WPT evidence. Report missing evidence to its existing owner and record PM dispositions for open slots, including C-7 and the December 15 backstop. Apply umbrella §3 CI/review gates. Completion of this plan review is a planning milestone; report program exit after the final implementation evidence has been reconciled.
