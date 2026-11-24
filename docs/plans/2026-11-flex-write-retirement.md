# Retire both Flex item computed-style writes

PM 1087 assigns this standalone implementation PR to L1 between the [style producer](2026-11-flex-item-blockification.md) and [Flex continuation](2026-11-flex-source-cursor.md). It transfers the joint writer removal from C1b.2. Its parent is the landed 3b.2-pre head. The canonical assignment part of C1b.1 is implemented here with its first live producer on the existing size types. The wider C1b.1 reference migration stays after C1a.

## Dependency and cutover

1. At the prerequisite base, run the production style walk through two contents wrappers to an auto-width inline-flex child. Inspect its published Flex display, inherited color and first auto-width dispatch. Keep both existing Flex writes until this producer meets the restyle, shadow, assigned-node and pseudo controls in the style memo.
2. In the joint cut commit, carry optional assigned physical content width/height and independently optional physical used margins through the existing child invocation. Preserve definite zero and signed used values. The parent passes the unshifted margin-box origin. The common box receiver applies each assigned edge once and selects each established content size before the child sizes descendants. Basis/minimum probes carry neither assigned axis, cross measurement receives only its established main axis, definitive placement carries both. Preserve containing references, probe state, baseline, generation and the full child layout outcome.
3. In that same commit migrate every definitive Flex placement, including continuous and fragmented dispatch, to the common receiver. Remove collection's display reinsertion and placement's width, height and margin reinsertion. Inventory constructors and callbacks at the implementation head. Trace the receiving branch before descendant layout in Block, nested Flex, Grid, Table, Multicol, replaced and box-producing early returns. Verify none and contents at collection. Grid can receive a Flex assignment here; its own production of assignments remains in C1b.3.

## Evidence at the prerequisite head

Run the numerical crossing fixtures in the [Flex input memo](2026-11-flex-item-inputs.md). Record the item input before publication of the outer box, its descendant geometry and its computed style before collection, after probes and after two definitive placements on the same tree. Count actual computed-style updates, including identical-value reinsertions. Require zero. Exercise the promoted inline-flex item through its first dispatch again at this head.

| Boundary | Observation |
|---|---|
| Width and immutable style | At 80px and 120px the 50% item receives 40px then 60px with unchanged percentage. The existing 200/300px auto-margin fixture also checks descendant widths 50/75px. |
| Margin-box origin and all edges | Check the 200px and 80px two-item rows, assigned zero, a fixed leading margin, both signs of cross-axis residual and the next sibling's origin against the explicit numbers in the input memo. |
| Specialized and early receivers | In the growing nested Grid observe 100px at Grid entry, track and child; inspect the corresponding nested Flex, Table cell and Multicol column geometry after changing the assigned size. For empty/early-return boxes assert the assigned content dimensions and used edges. |
| Replaced and stretch | Observe the 70px line, the independently assigned main axis in cross measurement, and 100×70 at the final image reader. The final solved pair must avoid another intrinsic solve. Preserve the existing unassigned and partial-axis sizing behavior. |

Record all-feature Flex, Block, Grid, Table, Multicol, Layout and Render results, the receiver inventory, and the zero-write evidence before requesting the independent implementation gate. This head supplies repeated sizing without a cursor. The real p/q interruption, saved continuation, count/replay and per-entry remaining-room proof belong to 3b.2f. Residual C1b.2 extends the phase-specific reference and axis work after C1b.1; it reruns the transferred geometry controls. The Flex input memo's spec coverage map applies to the transferred sizing, stretch and margin observations.
