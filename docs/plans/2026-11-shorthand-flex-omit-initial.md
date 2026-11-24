# Flex shorthand serialization for declared values

Draft child F of the Flex omit-initial program. Submit for an independent full plan review after the umbrella verdict.

## §1. Change and acceptance

Implement FlexHandler’s two typed family serializers under the existing coordinator. G owns grammar, D owns winners, H supplies typed dispatch, S supplies component text. Support pending and CSS-wide reads via their existing coordinator branches. Exercise independent rule and inline getters, empty/mixed-priority and missing members, alias selection and repeated reads. Assert canonical grammar order and exact zero basis semantics against the parser and used layout. Cross-check pinned WPT expectations and document normative differences.

For 1/1/0px both `1` and `0px` parse to the same triple under §7.1. CSSOM step 2 permits omission and requires representability. Elidex chooses `1`: it retains the first grammar component and is shorter in characters. This rule also keeps the all-default serialization nonempty.

| Parsed physical longhands | Proposed readback | What reparsing must preserve |
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

**C-7.** For grow/shrink/basis 0/1/0px the declared getter emits `0`. The rule-cssText WPT accepts `flex: 0px;` or `flex: 0 1 0px;`; retain this as a spelling cross-check for the separate block regrouping slot. Preserve explicit CSS-wide keywords because omitting them can reparse as physical Flex defaults and lose inheritance/reset semantics. Report Flex as affected under #11-shorthand-omit-initial-csswide-omission until a meaning-preserving family rule covers the case. [WPT flex-serialization.html](https://github.com/web-platform-tests/wpt/blob/master/css/cssom/flex-serialization.html)
