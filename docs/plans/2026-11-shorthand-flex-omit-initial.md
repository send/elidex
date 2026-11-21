# Flex shorthand serialization for declared values

PR 2 of the [handler-dispatch plan](2026-07-shorthand-serialization-handler-dispatch.md), under `#11-style-shorthand-expand`. Review baseline `ff3ec52b`. The deliverable is the declared-value readback of `flex-flow` and `flex` through both inline and rule style declarations. A complete set of representable physical longhands currently yields no reconstruction because FlexHandler inherits the default serializer.

## §1. Data flow and ownership

`elidex_css::shorthand_longhands` already gives direction/wrap and grow/shrink/basis in grammar order. The inline getter in `elidex-dom-api/src/style.rs` reads stored strings. The rule getter in `cssom_sheet.rs` serializes declaration values. Both call `elidex_style::serialize_shorthand_value`. That coordinator gathers all longhands, checks priorities, resolves the owner by the first longhand, dispatches, and then applies the [value-kind override](2026-07-shorthand-value-kind-gate.md). FlexHandler owns both first longhands. Add both arms to its existing property handler. This transformation needs only the declaration values on readback, with no ECS component, cache, layout state, or VM-specific code.

For `flex-flow`, adapt the existing plugin helper `serialize_omit_initial` to take ordered longhand pairs plus an initial-value lookup. Use the same implementation for Multicol and Flex, migrating its literal tests and the sole existing production caller together. This resolves Q4 in the [Multicol plan](2026-07-shorthand-multicol-omit-initial.md) and avoids building two temporary vectors per caller. Each owner supplies its own `initial_value(name).to_css_string()`. Keep the first canonical component when everything is initial.

For `flex`, use a FlexHandler-local collapse of the ordered triple. Its omission defaults and positional factors require a distinct rule. Match the three positions, preserve the supplied spelling of retained components, and return a candidate for every complete triple, including nonphysical strings. Only the coordinator can override that candidate for CSS-wide or unresolved values. Unknown shorthands still return None.

## §2. Coupled invariants and corner matrix

1. **Exact declared-value round trip.** Reparse output and compare the ordered serialized longhands, including units and keywords. Equivalent layout in one container cannot establish this property.
2. **Grammar and canonical order.** Flow takes one or both disjoint keywords. Flex takes a basis, a group of one or two factors, or both groups in either order. Inside the group, shrink requires grow. A bare zero takes a factor slot until both slots are occupied. A zero percentage remains a percentage.
3. **Omission defaults and aliases.** Flow omissions restore row/nowrap. Flex omissions restore grow 1, shrink 1 and a zero-length basis, whereas the physical longhand initials are 0/1/auto. The keyword none expands to 0/0/auto. Use auto as a basis by itself for 1/1/auto. Explicit CSS-wide initial must remain distinguishable from a triple of physical values.
4. **Coordinator authority and compatibility.** Presence, priority, and value kinds interact with coverage. The getter may fall back to a direct shorthand declaration on None. Enabling coverage also exposes the existing later-direct-variable ordering gap for Flex (see §4). CSSOM qualifies omission when a shorter spelling is less backwards-compatible. Preserve explicit zero units where omission can change the basis in a reader using a percentage default.

Proposed Flex algorithm for review: spell 0/0/auto as none. For a 1/1 pair emit the basis alone (including 0px, 0%, auto, content). Otherwise emit grow, emit shrink when it differs from 1, then emit basis. Keep 0px explicit under the compatibility clause. For the specific 0/1/0px triple use the full three-token form recognized by the CSSOM-linked WPT, which also makes the intended role of every zero explicit. These are serialization choices under the CSSOM principles. Flexbox specifies the expansion, not a unique spelling of each triple.

| Parsed physical longhands | Proposed readback | What reparsing must preserve |
|---|---|---|
| row / nowrap, column / nowrap, row / wrap | row, column, wrap respectively | A valid single keyword restores the omitted component. |
| column-reverse / wrap-reverse | column-reverse wrap-reverse | Author order may be reversed. Exercise all four directions × all three wraps. |
| 0 / 0 / auto, 1 / 1 / auto, 0 / 1 / auto | none, auto, 0 auto respectively | A bare 0 would change the third triple's basis. |
| 1 / 1 / 0px, 1 / 1 / 0% | 0px, 0% respectively | A dimension and a percentage remain distinct even at zero. |
| 0 / 1 / 0px, 2 / 1 / 0px, 2 / 0 / 0px | 0 1 0px, 2 0px, 2 0 0px respectively | No implicit zero percentage, no shrink in grow position. |
| 1 / 2 / 25%, 2 / 1 / 25% | 1 2 25%, 2 25% respectively | Grow anchors nondefault shrink. Omitting only shrink keeps the basis. |
| 1 / 1 / content, 0.5 / 0 / 10px | content, 0.5 0 10px respectively | Preserve basis-only keyword, fractional grow and explicit shrink zero. |

### Parser work required by readback

In `elidex-css/src/declaration/flex.rs`, `parse_flex_shorthand` tries a length before numeric factors. The length helper accepts unitless zero, so bare 0 takes the basis branch with grow 1. A leading auto or length returns early, and the numeric branch consumes a basis only after a successful second factor. The outer declaration parser rejects remaining tokens. Consequently valid spellings such as 0 auto, 2 25%, content alone and basis-first forms cannot all survive a getter/setter round trip today.

Implement the grammar at the existing shorthand expansion entry point: recognize none as a complete alternative, parse the contiguous factor group and basis in either order with rollback, prefer numbers as factors (including zero), and allow a trailing unitless zero basis after two factors. Reuse the basis value parsing logic for supported auto, content and length/percentage expressions. Share it with the longhand parsing path to the extent their value representations and crate boundaries permit. Keep the declaration parser responsible for priority and end-of-input. Reject duplicate basis, a third nonzero number, negative factors, a basis between two factors, and trailing garbage. Keep existing valid triples, including calc expressions already accepted by the shorthand. Test basis-first plus one/two factors, standalone 0, 0px, 0%, 0 0 0, and 0 with !important. Compare serialized longhands to accommodate equivalent internal keyword representations.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSSOM §6.6.1 The CSSStyleDeclaration Interface | getPropertyValue | absent longhand or mixed priority, complete uniform list | serialize_shorthand_value | ✓ | yes |
| CSSOM §6.7.2 Serializing CSS Values | list steps 1.2–1.3 | representability and covered-family override | serialize_shorthand_value | ✓ | yes |
| CSSOM §6.7.2 Serializing CSS Values | step 2 | grammar equivalence, order, omission, compatibility | FlexHandler | ✓ | yes |
| css-flexbox-1 §5.1 Flex Flow Direction: the flex-direction property | initial | row | FlexHandler::initial_value | ✓ | yes |
| css-flexbox-1 §5.2 Flex Line Wrapping: the flex-wrap property | initial | nowrap | FlexHandler::initial_value | ✓ | yes |
| css-flexbox-1 §5.3 Flex Direction and Wrap: the flex-flow shorthand | grammar | direction, wrap, both | serialize_omit_initial | ✓ | yes |
| css-flexbox-1 §7.1 The flex Shorthand | grammar and defaults | none, factors, basis, both orders, unitless zero | parse_flex_shorthand and FlexHandler | ✓ | yes |
| css-flexbox-1 §7.2.1 The flex-grow property | initial | zero, nonnegative factors | FlexHandler | ✓ | yes |
| css-flexbox-1 §7.2.2 The flex-shrink property | initial | one, nonnegative factors | FlexHandler | ✓ | yes |
| css-flexbox-1 §7.2.3 The flex-basis property | initial and type | auto, content, supported width values | parse_flex_shorthand | supported subset | yes |

**Breadth** is two modules and ten rows. One per-family PR keeps the inverse grammar and the readback contract reviewable together. Section numbers and titles were checked with webref heading queries for cssom 6.6/6.7 and css-flexbox-1 5/7. Definitions and initial rows were checked using webref body for the linked sections and individual longhands. [CSSOM getter](https://drafts.csswg.org/cssom-1/#dom-cssstyledeclaration-getpropertyvalue), [CSSOM serialization](https://drafts.csswg.org/cssom-1/#serialize-a-css-value), [Flex flow](https://drafts.csswg.org/css-flexbox-1/#flex-flow-property), [Flex shorthand](https://drafts.csswg.org/css-flexbox-1/#flex-property), [Flex components](https://drafts.csswg.org/css-flexbox-1/#flex-components).

### §3.1 User-input touch audit

Author CSS reaches the shorthand parser as tokens. Bound consumption to two factors and one basis, preserve function token boundaries and fail the declaration as a unit on extra input. The getter receives serialized component strings. It compares whole components and joins them without parsing or executing strings. Unknown property names cannot select a Flex arm through the registry. Longhand numeric range and basis grammar extensions beyond the currently supported value model belong to their defining parsers.

## §4. Verification and landing boundary

- Exercise every physical row above through the populated registry using the existing style test helper `serialize_parsed`. For each, parse the emitted shorthand into declarations, check the longhands, and serialize again to establish idempotence. Use explicit longhand declarations to construct inputs independently of shorthand parsing. Cover inline and rule `getPropertyValue` through their production handlers with representative flow, numeric and keyword cases.
- Cover both new families with identical CSS-wide keywords, mixed keywords, keyword plus physical, longhand var(), missing longhand, mixed priority and all important. Check the public result and distinguish None from Some(empty) in the coordinator. Keep an uncovered background control, migrate the existing Flex-is-uncovered assertions, and retain Multicol helper tests. Test whole-shorthand var() alone through the public fallback.
- The known covered-family stale-longhand versus later direct var() behavior is tracked by `#11-shorthand-direct-decl-cascade-order`. Its fix requires declaration order available outside the handler. Record the impact for both newly covered shorthands. The later Text, Box and layered/grid increments remain under the parent slot.
- The CSSOM-linked WPT exercises rule cssText, including the 0/1/0px case and lists of mixed CSS-wide declarations. Use it as a compatibility cross-check. Rule block regrouping has a different entry point, so passing the getter tests cannot establish a pass for the whole WPT. [WPT source](https://github.com/web-platform-tests/wpt/blob/master/css/cssom/flex-serialization.html).
- Run targeted all-features tests for elidex-css, elidex-plugin, elidex-css-flex, elidex-css-multicol, elidex-style and elidex-dom-api, followed by the repository pre-push gate. Request plan review before implementing. The decision for review is the explicit zero-length basis compatibility policy in §2.
