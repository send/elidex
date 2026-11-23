# Compute Text lengths with the applicable font reference

Row 3c.1 follows 3b.4, T and G. PM 1019 assigns this plan the Text part of the G′a migration. Its implementation supplies the font context used by the subsequent Table and Multicol children. N later replaces the shared numeric arithmetic while retaining these reader fixtures.

## §1. Declaration and computation

Route built-in font-size and line-height longhands and their independent size and slash-line-height arms in declaration/font.rs through G’s property-selected grammar. Apply the same category and range in TextHandler. Font-size takes a nonnegative length-percentage or its supported keywords. Line-height additionally takes normal and a nonnegative number. Accept a well-typed out-of-range calculation and clamp when the reference is available. Dispatch a number-valued calculation to G’s scalar reader and retain the inherited number until each element derives its used line height. Letter-spacing and word-spacing retain their supported signed length and normal arms. Remove the Text parser copy. G′b owns retirement of the remaining legacy plugin wrapper. C-5 stays with Intersection Observer's separate “parse a margin” algorithm.

Compute font-size first in the ordinary style walk. Carry root identity into style/resolve/mod.rs: for the root's own font-size resolve rem against the initial font size. Publish that result to the same entity's remaining computations as both its current font and its root reference. Pass the root reference unchanged to descendants while updating their em reference to each element's computed font. Font-size percentages use the parent font; line-height percentages use the newly computed element font. Share the plugin evaluation calls between style/resolve/font.rs and TextHandler’s font and spacing readers. Exercise the registered handler with the same phase-appropriate context, then inspect the published ComputedStyle separately through the ordinary document walk.

## §2. Crossing checks

Use stylesheet and inline declarations, both direct and with the complete value supplied through var(--v). Test built-in longhands, the font shorthand and its slash arm, and registered longhands where supported. Supply a real font-family in shorthand fixtures. Require complete token consumption. Compare published fields and actual used line height at the child.

| Crossing | Producer → consumer | Distinguishing observations |
|---|---|---|
| Root identity × font references | Walk → root style and descendant style | With initial font 16px, root font-size:1.5rem yields 24px. Its own line-height:calc(1rem + 1px) is 25px. A child with font-size:10px and the same line-height also computes 25px; changing rem to em on the child yields 11px. Rerun with root font-size:1rem and expect 16px and root line-height 17px. |
| Percentage basis × stage | Parent font → child size → child line height | With parent 20px, child font-size:calc(50% + 2px) gives 12px. Child line-height:calc(100% + 3px) gives 15px. Exercise the font shorthand and both independent longhands as well as the registered handlers. |
| Category × inheritance | Scalar arithmetic → inherited number → used length | Parent font 12px and line-height:calc(1 + .5) give a computed multiplier of 1.5 and used 18px. A child of size 20px inherits 1.5 and uses 30px. A length-valued 18px on the parent stays 18px on that child. Check both shorthand and longhand routes. |
| Range × cascade | Parser or substituted winner → computed value | For font-size and line-height, reject direct negative length and percentage literals. Reject negative number literals for line-height. Keep an earlier valid 13px in each case. With parent size 20px and line-height 22px, a winning var containing those invalid tokens instead inherits 20px or 22px. A winning calc(1px - 2px) computes to 0px. Line-height:calc(1 - 2) retains a zero multiplier. Repeat appropriate size/slash cases via font shorthand; assert inherited fields after invalid substitution. |
| Unit grammar × spacing | G parser and evaluator → both Text readers | Verify signed −2px and calc(1px - 3px) for each spacing property, plus normal. Check 12pt, 1pc and 1in as 16px, 16px and 96px; exercise Q/cm/mm with precision tolerance. Reject deferred units, 10bogus and trailing tokens with direct and substituted controls. |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-values-4 §6.1.1 Font-relative lengths | supply font metrics | root self, root remaining properties, descendants | style walk and Text reader | supported relative units | yes |
| css-values-4 §6.2 Absolute lengths | canonicalize units | physical lengths and invalid dimensions | G parser to Text numeric reader | supported units | yes |
| css-values-4 §9.1 Numeric Functions | apply nonnegative range | literal rejected, math clamped | direct and substituted receiving parser to computed style | numeric arms | yes |
| css-fonts-4 §2.5 Font size | resolve numeric size | length, percentage or mixed math | font-size longhand and TextHandler | numeric arm | yes |
| css-fonts-4 §2.7 Shorthand font property | admit size and optional slash line-height | valid pair, invalid size or line-height | declaration/font.rs | size and slash arms | yes |
| css-inline-3 §5.1 Line Spacing | compute and inherit line-height | normal, number, percentage and length | style and registered line-height reader | supported categories | yes |
| css-text-3 §7.1 Word Spacing | compute word spacing | signed length or normal | style and registered spacing reader | supported arms | yes |
| css-text-3 §7.2 Tracking | compute letter spacing | signed length or normal | style and registered spacing reader | supported arms | yes |

Place declaration fixtures beside font shorthand parsing, context fixtures in the style walk tests, and registered-reader cases in elidex-css-text. Run all-features tests for elidex-plugin, elidex-css, elidex-style and elidex-css-text. Preserve the root, category and range controls for N. Recheck touched module sizes before implementation. Follow umbrella §3 for review and exact-head landing clearance.
