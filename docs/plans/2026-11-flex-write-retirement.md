# Retire both Flex item computed-style writes

PM 1087 assigns this standalone implementation PR to L1 between the [style producer](2026-11-flex-item-blockification.md) and [Flex continuation](2026-11-flex-source-cursor.md). It transfers the joint writer removal from C1b.2. Its parent is the landed 3b.2-pre head. The canonical assignment part of C1b.1 is implemented here with its first live producer on the existing size types. The wider C1b.1 reference migration stays after C1a.

## Dependency and cutover

1. At the prerequisite base, run the production style walk through two contents wrappers to an auto-width inline-flex child. Inspect its published Flex display, inherited color and first auto-width dispatch. Keep both existing Flex writes until this producer meets the restyle, shadow, assigned-node and pseudo controls in the style memo.
2. In the joint cut commit, carry optional assigned physical content width/height and independently optional physical used margins through the existing child invocation. Preserve definite zero and signed used values. The parent passes the unshifted margin-box origin. The common box receiver applies each assigned edge once and selects each established content size before the child sizes descendants. Basis/minimum probes carry neither assigned axis, cross measurement receives only its established main axis, definitive placement carries both. Preserve containing references, probe state, baseline, generation and the full child layout outcome.
3. In that same commit migrate every definitive Flex placement, including continuous and fragmented dispatch, to the common receiver. Remove collection's display reinsertion and placement's width, height and margin reinsertion. Inventory constructors and callbacks at the implementation head. Trace the receiving branch before descendant layout in Block, nested Flex, Grid, Table, Multicol, replaced and box-producing early returns. Verify none and contents at collection. Grid can receive a Flex assignment here; its own production of assignments remains in C1b.3.

## §2. Evidence at the prerequisite head

Run the numerical crossing fixtures in the [Flex input memo](2026-11-flex-item-inputs.md). Record the item input before publication of the outer box, its descendant geometry and its computed style before collection, after probes and after two definitive placements on the same tree. Count actual computed-style updates, including identical-value reinsertions. Require zero. Exercise the promoted inline-flex item through its first dispatch again at this head.

| Boundary | Observation |
|---|---|
| Width and immutable style | At 80px and 120px the 50% item receives 40px then 60px with unchanged percentage. The existing 200/300px auto-margin fixture also checks descendant widths 50/75px. |
| Margin-box origin and all edges | Check the 200px and 80px two-item rows, assigned zero, a fixed leading margin, both signs of cross-axis residual and the next sibling's origin against the explicit numbers in the input memo. |
| Specialized and early receivers | In the growing nested Grid observe 100px at Grid entry, track and child; inspect the corresponding nested Flex, Table cell and Multicol column geometry after changing the assigned size. For empty/early-return boxes assert the assigned content dimensions and used edges. |
| Replaced and stretch | Observe the 70px line, the independently assigned main axis in cross measurement, and 100×70 at the final image reader. The final solved pair must avoid another intrinsic solve. Preserve the existing unassigned and partial-axis sizing behavior. |

Record all-feature Flex, Block, Grid, Table, Multicol, Layout and Render results, the receiver inventory, and the zero-write evidence before requesting the independent implementation gate. This head supplies repeated sizing without a cursor. The real p/q interruption, saved continuation, count/replay and per-entry remaining-room proof belong to 3b.2f. Residual C1b.2 extends the phase-specific reference and axis work after C1b.1; it reruns the transferred geometry controls.

At the joint deletion head, migrate the three direct-ECS Flex layout fixtures in spec_compliance.rs (base lines 11, 45 and 79). Supply computed Block, Flex and Table respectively. Assert stable computed display and first-dispatch behavior. Rerun the producer's real stylesheet cases for authored Inline, InlineFlex and InlineTable and the promotion through contents.

### Reproduce the production inventory

Run these searches at both the parent of the atomic cut and its proposed landing head. Record both full SHAs and the classified results in the PR evidence.

```sh
git grep -n -w -e layout_child -e LayoutInput -e dispatch_layout_child -e ChildLayoutFn -- crates/layout
git grep -n -w -e ComputedStyle -e insert_one -e get_style -- crates/layout/elidex-layout-flex/src
```

On bc523010 these searches return 309 and 151 matching lines respectively (including declarations, comments and fixtures; verified 2026-11-25). On the current implementation base the first search finds real child invocations in Flex helpers.rs:165,265 and algo.rs:229,495 and the dispatcher in elidex-layout/src/layout/mod.rs. The second finds the conditional collection insertion in helpers.rs:72 and the definitive placement insertion in algo.rs:450. Verify those two writers at the pre-cut revision even if the style producer has made the first condition dormant. Trace aliases, callback arguments and helper calls to their terminal readers and mutation sites. Classify test fixtures, documentation and declarations separately from executable callers.

At the cut head report the following counts: 0 production Flex computed-style writes; N definitive placement callers identified and N using the common assignment receiver; the count of unclassified callers or box-producing exits, which must be zero. Report phase and formatter subtotals, explicitly covering replaced and early-return boxes. Give a surviving dispatcher and a definitive callback file/line as positive controls at the cut and show each assignment reaches the child before descendant sizing. Compare the classified base/cut sites and explain each removal, move or addition. Run the zero-update probe and two-placement fixture as an independent check.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-flexbox-1 §4 Flex Items | Consume style-published blockification; remove collection writer | Inline, InlineFlex, InlineTable and promotion through contents | Flex collection → first child dispatch | three migrated fixtures and unchanged Block control | yes |
| css-flexbox-1 §9.8 Definite and Indefinite Sizes | Apply established physical content axes without writing computed style | unassigned probe, main-only measurement, definitive pair | Flex sizing and placement → common child receiver → specialized formatter or box-producing early return | both physical axes | yes |
| css-flexbox-1 §9.4 Cross Size Determination | Carry established stretch size into descendant layout | stretched child, nested formatter, replaced final pair | line sizing → definitive receiver → descendant or intrinsic-size reader | assigned and unassigned cross axis | yes |
| css-flexbox-1 §9.5 Main-Axis Alignment | Publish parent-used margins once from the margin-box origin | positive share, assigned zero, fixed edge | Flex positioning → common box receiver → sibling advance | both main edges | yes |
| css-flexbox-1 §9.6 Cross-Axis Alignment | Preserve signed cross margins at the receiver | positive residual and negative overflow | Flex placement → published box edges and origin | both cross edges | yes |

The collection and placement writes share one removal commit with all receiving formatters so every definitive caller observes the same input contract at landing (PM 1087 B).
