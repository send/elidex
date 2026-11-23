# Carry border spacing from declarations to separated table cells

Row 3c.2 follows the Text plan landed at decff274. Its implementation consumes T’s receiving declaration context, G’s nonnegative length grammar and the computed element font supplied by 3c.1. The next integration step is 3c.3 Multicol. N subsequently replays all three readers’ finite arithmetic fixtures.

## §1. Compute and consume the horizontal and vertical pair

The authored border-spacing property takes one or two nonnegative lengths. In declaration/misc.rs, select G’s length grammar for each item. The current parser accepts a length-percentage and filters a top-level percentage. A percentage inside a calculation must also fail the length category. Accept a single item when the input is exhausted after it. Otherwise require a valid second item and exhaustion before expanding either axis. Duplicate a single value or emit the horizontal then vertical value. Apply the same category and range at both registered TableHandler axis entries. Preserve the existing internal longhand names and pair serialization.

Use G’s shared numeric conversion in the ordinary style box_model writer and the registered handler’s resolve path. Supply the element’s completed font context for em and the document root reference for rem. Reject a negative literal at parse. Accept a length calculation whose definite result is negative and clamp it to zero at computation. Store the two computed pixel lengths on ComputedStyle. Inheritance copies those lengths. Table layout takes the pair from the table style in separate mode and uses zero spacing on both axes in collapse mode. Keep the computed fields available in either mode.

## §2. Crossing checks

Drive stylesheet and inline declarations through ordinary parsing, T substitution and the style walk. Separately exercise horizontal and vertical registered parse, resolve and computed readback. Feed each valid pair through the public table layout path. Compare horizontal/vertical gaps between adjacent cell border boxes in a two-row, two-column fixed-layout table with no spans, sufficient width, zero borders and padding. Use coordinates from the same layout generation.

| Crossing | Producer → consumer | Expected observation |
|---|---|---|
| Cardinality and units × axis order | G parser → declaration expansion → style and registered readers | 12pt expands to 16/16px. With calc(1in - 24px) 12pt, read 72/16px and measure horizontal/vertical gaps of 72/16px in separate mode. Repeat with the whole pair supplied by var; give the registered axes their corresponding individual values. Swap items and check both gaps swap. |
| Category × cascade | Direct input or T receiving-property reparse → both computed fields | Parent 11/17px, earlier table declaration 3/7px. A later direct 9px calc(1px + 10%) fails as a whole and leaves 3/7. Via a winning var containing that pair, both inherited fields become 11/17. Repeat with invalid first item, a negative literal, an unrecognized unit, and a third or trailing token. Neither axis may take the 9px prefix. |
| Range × actual use | Shared computation → table grid | calc(1px - 2px) 5px computes to 0/5px and gives gaps 0/5. Swap the items to cover the vertical clamp. Exercise direct, substituted and registered axis routes. |
| Font context × inheritance | Parent computed fields → child table style → layout | Set the parent font to 20px and spacing to 1em 0.5em. A 10px table with border-spacing:inherit keeps 20/10px including the measured gaps; specifying the same em pair on the table yields 10/5px. Explicit inheritance overrides the table UA declaration. |
| Computed pair × border model | Table style → separated/collapsed grid | On the same 72/16px fixture change only border-collapse from separate to collapse. Assert computed 72/16 in both, cell gaps 72/16 then 0/0. |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-tables-3 §3.5.2.1 The Border-Spacing property | expand, inherit and consume the computed pair | one/two items, separate/collapse | declaration parser, TableHandler, style and layout-table | both axes and models | yes |
| css-values-4 §6 Distance Units | select length category and convert | absolute, relative, unitless zero, invalid percentage | G grammar → both axis readers | supported lengths | yes |
| css-values-4 §9.1 Numeric Functions | apply numeric range | negative literal rejected, calculated negative clamped | receiving parser → computed pair | nonnegative lengths | yes |

Include unitless zero, mixed arithmetic, unknown and deferred units in both axis parser tests. Keep existing computed readback and shorthand serialization coverage. Put declaration tests alongside grid_table.rs, substitution and inheritance cases in elidex-style, axis tests in elidex-css-table and measured-gap controls in elidex-layout-table. Run all-features tests for elidex-plugin, elidex-css, elidex-style, elidex-css-table and elidex-layout-table. At decff274 the declaration/misc.rs, TableHandler and layout-table/lib.rs modules have 840, 393 and 825 lines respectively. Recount touched modules at implementation. Record the fixture results for 3c.3’s cross-reader checks and N’s arithmetic replay.
