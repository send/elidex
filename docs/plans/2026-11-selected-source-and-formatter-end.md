# P-A: Selected source and formatter completion

## §1. Producer and consumer contract

Under the [page-source program](2026-11-page-source-cursor.md), Block, inline collection, ordinary whole Flex items and Table rows, Multicol and whole-row Grid describe the definitive selection in a stable owner's source coordinates. Only that owner's selection commit writes its child terminal. Measure/probe, rejected line or avoided break, and positioned side effects cannot write it. A winning empty participating box can finish; excluded descendants contribute no source. Preserve exposed children of contents, distinguish collapsed whitespace from a selected line, and separate first/last in-flow box propagation from source participation. Keep stable IDs across flattened wrappers. Consumers are parent stack/inline owners, P-B's evidence validator and P-C's generation-filtered traversal.

## §2. Review inputs and fixtures

- R1 F2, PM 1180 H1: run none/one/two comments, empty in-flow sibling, hidden subtree, contents, float, positioned, mixed, whitespace and visible line. Assert participant ID, source range, print membership and separate static-position effect. Pass first/last eligible boundary to P-B.
- R1 F3, PM 1180 H2: the selected atomic reader takes a supplied formatter end. Complete commits the selected terminal; an escaping Whole Continue or supplied Incomplete yields Incomplete at the inline boundary. Earlier selected prefix remains, unfinished atom/suffix earns no terminal. Rejected line drops its tentative geometry and leaves source for the winner. Compare inline-block/flex/grid/table, directly and wrapped, with selected-prefix membership. Test a real ordinary completion and inject Continue and Incomplete at this checkpoint. P-B subsequently supplies real Multicol ends, including positive batch draining and zero-room stall; P-C checks prefix paint. Flex and Table receiver dependencies are explicit in the umbrella.
- R2-3: exercise Block depth limit and inline collection budget with a visible sentinel under the guard. Empty and fully excluded groups finish; unvisited participating input without a resumable route yields Incomplete, no enclosing terminal and no selected suffix. Test a committed prefix here; P-C checks the partial display. Unknown participation at cutoff cannot certify exhaustion.
- Reference to P-B’s PM 1177 F1 / PM 1178 and R1 F5: overlapping spans A/B/C distinguish per-piece source additions {A,B}, {}, {C} from P-B's outstanding track geometry. Preserve 30×10 dimensions and the 50/50 then 100 carry. P-A never awards a second terminal to pay geometric debt.
- Evidence for P-C’s R1 F6 / R2-6: retain base observations for ordinary 80×20 atom and blue draw, local-run owner, rejected candidate and visible-line membership. Measure outstanding base observables before implementing changed selection; archive producer identities/generation and feed identical selected input to P-C. Typed ends absent on base require assertions at this checkpoint.

Ordinary admission is a P-A producer checkpoint. Port the disjoint first/second-piece whole-item terminal assertions at frozen Flex fragmentation.rs lines 196/219 and 661/683 (row and column). Port block_preserves_table_row_contributions_at_a_forced_break and block_reports_only_rows_admitted_by_table_grid. Assert exactly the admitted rows through their Block parent. These cases must return known selections. Keep the nested Flex item and Table cell paths private until 3b.2f and #11-table-fragment-source-cursor respectively can propagate their real ends; the inline reader cannot infer a dropped descendant end.

## §3. Spec coverage map

| Spec section | Contract |
|---|---|
| css-display-3 §2.5 | Box generation and flattened contents inform participation. |
| css-break-3 §2.1, 3.1.1 | Parallel flows and in-flow break propagation have separate ownership. |
| css-grid-2 §13 | Between-row source admission follows PM 1178; internal item splits have a separate receiver. |
