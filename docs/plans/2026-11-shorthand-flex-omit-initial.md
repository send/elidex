# Flex shorthand serialization for declared values

PR 2 of the [handler-dispatch plan](2026-07-shorthand-serialization-handler-dispatch.md), under `#11-style-shorthand-expand`. Review baseline `ff3ec52b`. The deliverable is the declared-value readback of `flex-flow` and `flex` through both inline and rule style declarations. A complete set of representable physical longhands currently yields no reconstruction because FlexHandler inherits the default serializer.

## Landing order and prerequisite contracts

PM 928/929 approved two independent prerequisites. Each has its own plan, worktree, PR and full plan review. Both land before the Flex serializer. The [umbrella staging erratum](2026-07-shorthand-serialization-handler-dispatch.md#staging-umbrella--per-family) includes physical numeric triples in PR 2.

1. **Flex shorthand parser.** The declaration parser currently tries a length before a number, so bare `0` yields grow 1 instead of 0, changing the cascaded value and layout. The prerequisite owns the inverse grammar, both component-group orders, contiguous factors, rollback, unitless zero disambiguation, complete consumption, priority and invalid inputs. It must preserve supported calc expressions and reject duplicate bases, negative factors, split factor groups and surplus tokens. Unify the basis value parser as `elidex_plugin::css_resolve::parse_flex_basis` (NEW) for the declaration longhand, shorthand and FlexHandler. Use one `CssValue::Auto` representation, one nonnegative factor parser (reject negatives), and the existing calc-capable CSS value grammar at the shared dependency tier. The current handler-only length helper lacks calc support. The parser plan owns that migration, typed parse tests, computed/layout regression for bare zero, and the flex-valid, flex-invalid and flex-shorthand WPT subset. Replace the parser’s informative §7.1.1 citation with normative css-flexbox-1 §7.1.
2. **Declaration order.** The shared prerequisite for `#11-shorthand-direct-decl-cascade-order` owns ordered declarations, priority and pending-substitution provenance across both public getters. CSSOM describes an ordered declaration block and gathers longhands in canonical order for readback. CSS Values requires recovery of the original shorthand when all winning longhands carry the same original pending substitution. Elidex currently represents the variable-bearing shorthand under its own key. The prerequisite must preserve those semantics when that key coexists with expanded declarations. L1 owns the slot with re-evaluation on 2026-12-15. PM owns the ledger update. [CSSOM declaration block](https://drafts.csswg.org/cssom-1/#css-declaration-block), [CSS Values pending substitution](https://drafts.csswg.org/css-values-5/#pending-substitution-value).

## §1. Data flow and ownership

`elidex_css::shorthand_longhands` already gives direction/wrap and grow/shrink/basis in grammar order. The inline getter in `crates/dom/elidex-dom-api/src/style.rs` reads stored strings. The rule getter in `crates/dom/elidex-dom-api/src/cssom_sheet.rs` serializes declaration values. Both call `elidex_style::serialize_shorthand_value`. That coordinator gathers all longhands, checks priorities, resolves the owner by the first longhand, dispatches, and then applies the [value-kind override](2026-07-shorthand-value-kind-gate.md). FlexHandler owns both first longhands. Add both arms to its existing property handler. This transformation needs only the declaration values on readback, with no ECS component, cache, layout state, or VM-specific code.

For `flex-flow`, adapt the existing plugin helper `serialize_omit_initial` to take ordered longhand pairs plus an initial-value lookup. Use the same implementation for Multicol and Flex, migrating its literal tests and the sole existing production caller together. This resolves Q4 in the [Multicol plan](2026-07-shorthand-multicol-omit-initial.md) and avoids building two temporary vectors per caller. Each owner supplies its own `initial_value(name).to_css_string()`. When everything is initial, the grammar requires at least one component. For flow, both row and nowrap reparse to row/nowrap. CSSOM specifies canonical order and omission, and leaves the survivor unspecified. Elidex breaks the tie by retaining the earliest grammar component, row. This derives the shared helper rule independently for Flex. Recheck both existing Multicol all-initial cases by reparsing the retained first component, plus their mixed and empty controls. This resolves Multicol Q3. Replace the historical authority claim in the helper docstring, prune its T row from `.claude/tools/ecs-mission-allowlist.tsv`, run the ratchet and report this disposition for group (k) of `#11-core-browser-parity-deviations` at landing.

For `flex`, use a FlexHandler-local collapse of the ordered triple. Its omission defaults and positional factors require a distinct rule. Match the three positions, preserve the supplied spelling of retained components, and return a candidate for every complete triple, including nonphysical strings. Only the coordinator can override that candidate for CSS-wide or unresolved values. Unknown shorthands still return None.

## §2. Coupled invariants and corner matrix

1. **Exact declared-value round trip.** Reparse output and compare the ordered serialized longhands, including units and keywords. Equivalent layout in one container cannot establish this property.
2. **Grammar and canonical order.** Flow takes one or both disjoint keywords. Flex takes a basis, a group of one or two factors, or both groups in either order. Inside the group, shrink requires grow. A bare zero takes a factor slot until both slots are occupied. A zero percentage remains a percentage.
3. **Omission defaults and aliases.** Flow omissions restore row/nowrap. Flex omissions restore grow 1, shrink 1 and a zero-length basis, whereas the physical longhand initials are 0/1/auto. The keyword none expands to 0/0/auto. Use auto as a basis by itself for 1/1/auto. Explicit CSS-wide initial must remain distinguishable from a triple of physical values.
4. **Coverage and declaration order.** Both getters must select winning declarations before family collapse. A partial substitution returns the empty string. A complete original substitution returns its original text. These cases depend on the declaration-order prerequisite and the existing value-kind gate.
5. **Serializer × parser.** Every emitted physical spelling must be in the prerequisite parser’s accepted language and reproduce the ordered typed values. Test the Multicol helper migration and Flex flow against the same omission and nonempty-output invariants.

Apply CSSOM §6.7.2 step 2 to the specified omission defaults in css-flexbox-1 §7.1. No compatibility exception applies. Emit none for 0/0/auto. When the basis is the serialized omission default (0px), emit grow and include shrink only if it differs from 1. When the basis differs from that default, emit it alone for a 1/1 factor pair. Otherwise emit grow, optional nondefault shrink, and the basis. Retain explicit 0% and other declared units that would change on reparsing. Physical values never synthesize a CSS-wide keyword.

**One-token tie.** For 1/1/0px both `1` and `0px` parse to the same triple under §7.1. CSSOM step 2 permits omission and requires representability. Elidex chooses `1`: it retains the first grammar component and is shorter in characters. This rule also keeps the all-default serialization nonempty.

| Parsed physical longhands | Proposed readback | What reparsing must preserve |
|---|---|---|
| row / nowrap, column / nowrap, row / wrap | row, column, wrap respectively | A valid single keyword restores the omitted component. |
| column-reverse / wrap-reverse | column-reverse wrap-reverse | Author order may be reversed. Exercise all four directions × all three wraps. |
| 0 / 0 / auto, 1 / 1 / auto, 0 / 1 / auto | none, auto, 0 auto respectively | A bare 0 would change the third triple's basis. |
| 1 / 1 / 0px, 1 / 1 / 0% | 1, 0% respectively | A dimension and a percentage remain distinct even at zero. |
| 0 / 1 / 0px, 2 / 1 / 0px, 2 / 0 / 0px | 0, 2, 2 0 respectively | Omitted basis restores 0px. The second number is shrink. |
| 1 / 2 / 25%, 2 / 1 / 25% | 1 2 25%, 2 25% respectively | Grow anchors nondefault shrink. Omitting only shrink keeps the basis. |
| 1 / 1 / content, 0.5 / 0 / 10px | content, 0.5 0 10px respectively | Preserve basis-only keyword, fractional grow and explicit shrink zero. |

### Ordering regression gate

At the baseline both getters reconstruct first and fall back to the direct key on None. The following outputs follow from that code path. For each row test both inline and rule declarations, at equal priority.

| Declaration order | Baseline | Handler-only change | Required after both prerequisites and Flex |
|---|---|---|---|
| `flex: initial; flex: var(--x)` | var(--x) | initial | var(--x) |
| `flex-flow: initial; flex-flow: var(--x)` | var(--x) | initial | var(--x) |
| `flex: var(--x); flex: initial` | var(--x) | initial | initial |
| `flex-flow: var(--x); flex-flow: initial` | var(--x) | initial | initial |

The declaration-order prerequisite exercises both orders on every then-covered shorthand (margin, padding, border-radius, overflow, gap, border-spacing, column-rule and columns), with priority and partially superseded substitutions. PR 2 extends the same public tests to flex and flex-flow. A failure blocks landing.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSSOM §6.6.1 The CSSStyleDeclaration Interface | getPropertyValue steps 1.2.2.2, 1.2.3–1.2.4 | absent longhand or mixed priority, complete uniform list | serialize_shorthand_value | ✓ | yes |
| CSSOM §6.7.2 Serializing CSS Values | list steps 1.2–1.3 | representability and covered-family override | serialize_shorthand_value | ✓ | yes |
| CSSOM §6.7.2 Serializing CSS Values | step 2 | grammar equivalence, order, omission, compatibility | FlexHandler | ✓ | yes |
| css-flexbox-1 §5.1 Flex Flow Direction: the flex-direction property | initial | row | FlexHandler::initial_value | ✓ | yes |
| css-flexbox-1 §5.2 Flex Line Wrapping: the flex-wrap property | initial | nowrap | FlexHandler::initial_value | ✓ | yes |
| css-flexbox-1 §5.3 Flex Direction and Wrap: the flex-flow shorthand | grammar | direction, wrap, both | serialize_omit_initial | ✓ | yes |
| css-flexbox-1 §7.1 The flex Shorthand | grammar and defaults | none, factors, basis, both orders, unitless zero | FlexHandler (parser prerequisite verified separately) | ✓ | yes |
| css-flexbox-1 §7.2.1 The flex-grow property | initial | zero, nonnegative factors | FlexHandler | ✓ | yes |
| css-flexbox-1 §7.2.2 The flex-shrink property | initial | one, nonnegative factors | FlexHandler | ✓ | yes |
| css-flexbox-1 §7.2.3 The flex-basis property | initial and type | auto, content, supported width values | FlexHandler | supported subset | yes |

**Breadth** is two modules and ten rows. The table covers the serializer implementation. The declaration-order specification belongs to the separate prerequisite. Reproduce the citation checks from the repository root:

```sh
.claude/tools/webref heading cssom-1 6.6
.claude/tools/webref heading cssom-1 6.7
.claude/tools/webref heading css-flexbox-1 5
.claude/tools/webref heading css-flexbox-1 7
.claude/tools/webref body cssom-1 dom-cssstyledeclaration-getpropertyvalue
.claude/tools/webref body cssom-1 serialize-a-css-value
.claude/tools/webref body cssom-1 css-declaration-block
.claude/tools/webref body css-values-5 pending-substitution-value
for anchor in flex-direction-property flex-wrap-property flex-flow-property flex-property flex-grow-property flex-shrink-property flex-basis-property; do
  .claude/tools/webref body css-flexbox-1 "$anchor"
done
```

### §3.1 User-input touch audit

Author CSS reaches this change as declared longhand strings through the inline or rule getter. The handler compares whole components and joins them in grammar order. Presence, priority, pending substitution and mixed kinds are resolved by the upstream mechanisms. Parser input validation and range handling are owned by the Flex parser prerequisite above.

## §4. Verification and landing boundary

- Exercise every physical row above through the populated registry using the existing style test helper `serialize_parsed`. For each, parse the emitted shorthand into declarations, check the longhands, and serialize again to establish idempotence. Use explicit longhand declarations to construct inputs independently of shorthand parsing. Cover inline and rule `getPropertyValue` through their production handlers with representative flow, numeric and keyword cases.
- Cover both new families with identical CSS-wide keywords, mixed keywords, keyword plus physical, longhand var(), missing longhand, mixed priority and all important. Check the public result and distinguish None from Some(empty) in the coordinator. Keep an uncovered background control, migrate the existing Flex-is-uncovered assertions, and retain Multicol helper tests. Test whole-shorthand var() alone through the public fallback.
- Test `#11-shorthand-omit-initial-csswide-omission` as a carried slot (L1, 2026-12-15). Uniform explicit initial returns initial. Mixed explicit initial and physical longhands return empty under the value-kind gate. This PR covers omission of physical defaults. A broader explicit-keyword omission decision requires a separately justified round-trip rule.
- Put the Flex matrix in a child test module `crates/css/elidex-style/src/tests/shorthand/flex.rs` (NEW) so it can use the existing parent helpers. Keep inline integration in `crates/dom/elidex-dom-api/src/style/tests.rs` and place rule API cases in a sibling VM test module `crates/script/elidex-js/src/vm/tests/tests_cssom_flex.rs` (NEW), registered in `tests/mod.rs`. This leaves the existing 986-line rule test file untouched.
- The linked WPT accepts two rule cssText spellings for bare zero, `flex: 0px;` and `flex: 0 1 0px;`. Our getter emits `0` for 0/1/0px under the general omission rule. Record the difference as a cross-check, with separate coverage for rule block regrouping. [WPT source](https://github.com/web-platform-tests/wpt/blob/master/css/cssom/flex-serialization.html).
- Run targeted all-features tests for elidex-plugin, elidex-css-flex, elidex-css-multicol, elidex-style, elidex-dom-api and elidex-js. Before push run the repository gate and obtain PM approval. Post-push review is one Codex review requested with PM go. The PM reads and relays its verdict.
